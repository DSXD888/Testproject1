#![forbid(unsafe_code)]

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, ensure};
use assert_cmd::Command;
use rusqlite::{Connection, OptionalExtension};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tempfile::TempDir;

struct RunHarness {
    _temp: TempDir,
    state_root: PathBuf,
    runtime_root: PathBuf,
    artifact_root: PathBuf,
    fixture_path: PathBuf,
    db_path: PathBuf,
}

struct RunResult {
    stdout: String,
    stderr: String,
    success: bool,
}

struct FakeClaudeCli {
    program: PathBuf,
    result_path: PathBuf,
    checkpoint_path: PathBuf,
}

type RuntimeIdentityRow = (
    Option<i64>,
    Option<i64>,
    Option<i64>,
    Option<String>,
    Option<i64>,
);

impl RunHarness {
    fn new() -> Result<Self> {
        let temp = tempfile::tempdir()?;
        let root = temp.path().to_path_buf();
        let state_root = root.join("state");
        let runtime_root = root.join("runtime");
        let artifact_root = root.join("artifacts");
        fs::create_dir_all(&state_root)?;
        fs::create_dir_all(&runtime_root)?;
        fs::create_dir_all(&artifact_root)?;
        Ok(Self {
            _temp: temp,
            db_path: state_root.join("state.db"),
            fixture_path: root.join("fixture.json"),
            state_root,
            runtime_root,
            artifact_root,
        })
    }

    fn run(&self, fixture: &Value) -> Result<RunResult> {
        fs::write(&self.fixture_path, serde_json::to_vec_pretty(fixture)?)
            .with_context(|| format!("writing {}", self.fixture_path.display()))?;
        let fake_claude = self.prepare_fake_claude(fixture)?;

        let output = Command::cargo_bin("daemon")?
            .arg("--ingest")
            .arg("fts")
            .arg("--agent-cli")
            .arg("claude")
            .arg("--agent-cli-program")
            .arg(&fake_claude.program)
            .arg("--state-root")
            .arg(&self.state_root)
            .arg("--runtime-dir")
            .arg(&self.runtime_root)
            .arg("--artifact-root")
            .arg(&self.artifact_root)
            .arg("--fixture-path")
            .arg(&self.fixture_path)
            .env("DAEMON_TEST_CLAUDE_RESULT_PATH", &fake_claude.result_path)
            .env(
                "DAEMON_TEST_CLAUDE_CHECKPOINT_PATH",
                &fake_claude.checkpoint_path,
            )
            .output()
            .context("running daemon binary")?;

        Ok(RunResult {
            stdout: String::from_utf8(output.stdout).context("decoding stdout")?,
            stderr: String::from_utf8(output.stderr).context("decoding stderr")?,
            success: output.status.success(),
        })
    }

    fn run_procontract_interest_replay(&self, result_contract: &Value) -> Result<RunResult> {
        let replay_path = self
            .state_root
            .parent()
            .ok_or_else(|| anyhow::anyhow!("state root missing parent"))?
            .join("procontract-interest-result.json");
        fs::write(&replay_path, serde_json::to_vec_pretty(result_contract)?)
            .with_context(|| format!("writing {}", replay_path.display()))?;

        let output = Command::cargo_bin("daemon")?
            .arg("--ingest")
            .arg("procontract-interest")
            .arg("--state-root")
            .arg(&self.state_root)
            .arg("--runtime-dir")
            .arg(&self.runtime_root)
            .arg("--artifact-root")
            .arg(&self.artifact_root)
            .env("PROCONTRACT_INTEREST_REPLAY_RESULT_JSON", &replay_path)
            .output()
            .context("running daemon binary")?;

        Ok(RunResult {
            stdout: String::from_utf8(output.stdout).context("decoding stdout")?,
            stderr: String::from_utf8(output.stderr).context("decoding stderr")?,
            success: output.status.success(),
        })
    }

    fn connection(&self) -> Result<Connection> {
        Connection::open(&self.db_path)
            .with_context(|| format!("opening {}", self.db_path.display()))
    }

    fn prepare_fake_claude(&self, fixture: &Value) -> Result<FakeClaudeCli> {
        let root = self
            .state_root
            .parent()
            .ok_or_else(|| anyhow::anyhow!("state root missing parent"))?;
        let state_dir = root.join("fake-claude");
        fs::create_dir_all(&state_dir)?;
        let probe_json = state_dir.join("probe.json");
        fs::write(
            &probe_json,
            serde_json::to_vec_pretty(&json!({
                "auth_healthy": true,
                "structured_output": true,
                "pty_viable": true,
                "browser_tools": true
            }))?,
        )?;
        let result_source = state_dir.join("result-source.json");
        fs::write(
            &result_source,
            serde_json::to_vec_pretty(&manifest_result_payload(fixture)?)?,
        )?;
        let program = state_dir.join("claude");
        fs::write(
            &program,
            format!(
                "#!/bin/sh\nset -eu\nstate_dir=\"{}\"\nif [ \"${{1-}}\" = \"--version\" ] || [ \"${{1-}}\" = \"-v\" ]; then\n  printf 'claude 1.2.3\\n'\n  exit 0\nfi\nif [ \"${{1-}}\" = \"probe\" ]; then\n  cat \"$state_dir/probe.json\"\n  exit 0\nfi\nmode=pty\nfor arg in \"$@\"; do\n  if [ \"$arg\" = \"--output-format\" ]; then\n    mode=contract\n    break\n  fi\ndone\nif [ \"$mode\" = contract ]; then\n  cp \"$state_dir/result-source.json\" \"$DAEMON_TEST_CLAUDE_RESULT_PATH\"\n  printf '{{\"event\":\"result\",\"path\":\"%s\"}}\\n' \"$DAEMON_TEST_CLAUDE_RESULT_PATH\"\nelse\n  : > \"$DAEMON_TEST_CLAUDE_CHECKPOINT_PATH\"\n  printf '{{\"event\":\"checkpoint\",\"seq\":1,\"path\":\"%s\"}}\\n' \"$DAEMON_TEST_CLAUDE_CHECKPOINT_PATH\"\nfi\n",
                state_dir.display()
            ),
        )?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&program)?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&program, perms)?;
        }
        Ok(FakeClaudeCli {
            program,
            result_path: self.artifact_root.join("fake-claude-result.json"),
            checkpoint_path: self.artifact_root.join("fake-claude-checkpoint.json"),
        })
    }
}

fn manifest_result_payload(fixture: &Value) -> Result<Value> {
    if fixture["notices"].as_array().is_some_and(Vec::is_empty) {
        let markdown = "# Opportunity Manifest\n\n## Header\n\n- Process: process-empty\n- Title: Empty batch\n\n## Decision Summary\n\n## Per-Candidate Detail\n\n## Operator Input Requests\n\n## Evidence Trail\n\n## Coverage Metrics\n\n- Candidate count: 0\n";
        return Ok(json!({
            "process_id": "process-empty",
            "decisionBasisHash": "empty",
            "manifestMarkdown": format!(
                "---\nmanifest_version: v1.0-2026-05-01\nprocess_id: process-empty\nbasis_hash: empty\nstoplight: review\nsuggested_action: Operator review required\nclarification_prompts: []\n---\n{markdown}"
            ),
            "stoplight": "review",
            "suggested_action": "Operator review required",
            "suggested_action_reasoning": "process-empty has no Stage 12 candidates, so the manifest remains in review.",
            "clarification_prompts": [],
            "candidate_count": 0,
            "priced_candidate_count": 0,
            "clarification_prompt_count": 0,
        }));
    }
    let process_id = fixture["notices"][0]["processId"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("fixture missing processId"))?;
    let title = fixture["notices"][0]["title"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("fixture missing title"))?;
    let candidate_count = fixture["notices"][0]["lots"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("fixture missing lots"))?
        .iter()
        .filter_map(|lot| lot["candidateCount"].as_u64())
        .sum::<u64>();
    let body = format!(
        "# Opportunity Manifest\n\n## Header\n\n- Process: {process_id}\n- Title: {title}\n\n## Decision Summary\n\n## Per-Candidate Detail\n\n## Operator Input Requests\n\n## Evidence Trail\n\n## Coverage Metrics\n\n- Candidate count: {candidate_count}\n"
    );
    let decision_basis_hash = format!("{:x}", Sha256::digest(body.as_bytes()));
    let markdown = format!(
        "---\nmanifest_version: v1.0-2026-05-01\nprocess_id: {process_id}\nbasis_hash: {decision_basis_hash}\nstoplight: review\nsuggested_action: Operator review required\nclarification_prompts: []\n---\n{body}"
    );
    let mut payload = json!({
        "process_id": process_id,
        "decisionBasisHash": decision_basis_hash,
        "manifestMarkdown": markdown,
        "stoplight": "review",
        "suggested_action": "Operator review required",
        "suggested_action_reasoning": format!("{title} requires operator review before the manifest can drive a decision."),
        "clarification_prompts": [],
        "candidate_count": candidate_count,
        "priced_candidate_count": 0,
        "clarification_prompt_count": 0,
    });
    if fixture["manifestSchemaInvalid"] == json!(true) {
        payload
            .as_object_mut()
            .ok_or_else(|| anyhow::anyhow!("result payload must be object"))?
            .remove("process_id");
    }
    Ok(payload)
}

fn default_fixture() -> Value {
    json!({
      "cursorBefore": "cursor-000",
      "cursorAfter": "cursor-001",
      "continuationRequired": false,
      "fingerprintState": "matched",
      "localeState": "en-GB",
      "currencyState": "GBP",
      "ptyTranscriptLines": [
        "fingerprint matched",
        "transcript says 99 candidates",
        "ingestion complete"
      ],
      "notices": [
        {
          "processId": "process-fts-001",
          "noticeId": "notice-fts-001",
          "sourceNoticeId": "ocds-001",
          "sourceUrl": "https://www.find-tender.service.gov.uk/notice/001",
          "title": "Supply of industrial widgets",
          "buyerName": "Widgetshire Council",
          "publishedAt": "2026-03-25T00:00:00Z",
          "sourceUpdatedAt": "2026-03-25T00:30:00Z",
          "ocid": "ocds-1234",
          "procurementStage": "tender",
          "lots": [
            {
              "lotId": "lot-001",
              "sourceLotId": "source-lot-001",
              "title": "Widgets lot",
              "candidateCount": 2,
              "estimatedValuePence": 120_000
            },
            {
              "lotId": "lot-002",
              "sourceLotId": "source-lot-002",
              "title": "Replacement widgets",
              "candidateCount": 1,
              "estimatedValuePence": 120_000
            }
          ]
        }
      ]
    })
}

fn set_field(fixture: &mut Value, key: &str, value: Value) -> Result<()> {
    fixture
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("fixture must be an object"))?
        .insert(key.to_owned(), value);
    Ok(())
}

fn job_input_payload(conn: &Connection) -> Result<Value> {
    let payload: String = conn.query_row(
        "SELECT input_payload FROM workflow_jobs WHERE parent_job_id IS NULL ORDER BY created_at DESC LIMIT 1",
        [],
        |row| row.get(0),
    )?;
    serde_json::from_str(&payload).context("parsing workflow_jobs.input_payload")
}

fn job_output_payload(conn: &Connection) -> Result<Value> {
    let payload: String = conn.query_row(
        "SELECT output_payload FROM workflow_jobs WHERE parent_job_id IS NULL ORDER BY created_at DESC LIMIT 1",
        [],
        |row| row.get(0),
    )?;
    serde_json::from_str(&payload).context("parsing workflow_jobs.output_payload")
}

fn latest_job_status(conn: &Connection) -> Result<String> {
    conn.query_row(
        "SELECT status FROM workflow_jobs WHERE parent_job_id IS NULL ORDER BY created_at DESC LIMIT 1",
        [],
        |row| row.get(0),
    )
    .context("loading latest workflow status")
}

fn count_rows(conn: &Connection, table: &str) -> Result<i64> {
    conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
        row.get(0)
    })
    .with_context(|| format!("counting rows in {table}"))
}

#[test]
fn e2e_ing_001_manual_cli_trigger_creates_sync_source_job_under_daemon_control() -> Result<()> {
    let harness = RunHarness::new()?;
    let result = harness.run(&default_fixture())?;
    ensure!(
        result.success,
        "daemon run should succeed: {}",
        result.stderr
    );

    let conn = harness.connection()?;
    let (kind, status): (String, String) = conn.query_row(
        "SELECT kind, status FROM workflow_jobs WHERE parent_job_id IS NULL ORDER BY created_at DESC LIMIT 1",
        [],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    ensure!(kind == "sync_source");
    ensure!(status == "completed");
    Ok(())
}

#[test]
fn e2e_ing_002_job_creation_persists_explicit_skill_binding_and_resolved_version() -> Result<()> {
    let harness = RunHarness::new()?;
    let result = harness.run(&default_fixture())?;
    ensure!(result.success);

    let conn = harness.connection()?;
    let payload = job_input_payload(&conn)?;
    ensure!(payload["skill_binding"]["path"] == "skills/procurement-ingestion/fts/SKILL.md");
    ensure!(payload["skill_binding"]["version"] == "1.0.0");
    ensure!(payload["agent_cli"] == "claude");
    Ok(())
}

#[test]
fn e2e_ing_003_fts_skill_resolves_to_pty_execution() -> Result<()> {
    let harness = RunHarness::new()?;
    let result = harness.run(&default_fixture())?;
    ensure!(result.success);

    let conn = harness.connection()?;
    let execution_mode: String = conn.query_row(
        "SELECT execution_mode FROM agent_session_logs WHERE execution_mode = 'pty' LIMIT 1",
        [],
        |row| row.get(0),
    )?;
    ensure!(execution_mode == "pty");
    Ok(())
}

#[test]
fn e2e_ing_004_live_loop_spawn_records_runtime_identifiers() -> Result<()> {
    let harness = RunHarness::new()?;
    let result = harness.run(&default_fixture())?;
    ensure!(result.success);

    let conn = harness.connection()?;
    let (session_id, process_id, process_group_id, adapter_id, exit_code): RuntimeIdentityRow =
        conn.query_row(
        "SELECT sid, pid, pgid, adapter_id, exit_code FROM agent_session_logs WHERE execution_mode = 'pty' LIMIT 1",
        [],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
    )?;
    ensure!(session_id.unwrap_or_default() > 0);
    ensure!(process_id.unwrap_or_default() > 0);
    ensure!(process_group_id.unwrap_or_default() > 0);
    ensure!(adapter_id.as_deref() == Some("claude"));
    ensure!(exit_code == Some(0));
    Ok(())
}

#[test]
fn e2e_ing_005_page_fingerprint_proven_before_extraction() -> Result<()> {
    let harness = RunHarness::new()?;
    let result = harness.run(&default_fixture())?;
    ensure!(result.success);

    let conn = harness.connection()?;
    let trace: String = conn.query_row(
        "SELECT structured_trace FROM agent_session_logs WHERE execution_mode = 'pty' LIMIT 1",
        [],
        |row| row.get(0),
    )?;
    let parsed: Value = serde_json::from_str(&trace)?;
    ensure!(parsed["fingerprint_state"] == "matched");
    Ok(())
}

#[test]
fn e2e_ing_006_cursor_backtrack_and_batch_boundary_advancement_enforced() -> Result<()> {
    let harness = RunHarness::new()?;
    let result = harness.run(&default_fixture())?;
    ensure!(result.success);

    let conn = harness.connection()?;
    let cursor_value: String = conn.query_row(
        "SELECT cursor_value FROM source_cursors WHERE source_id = 'fts' AND cursor_key = 'notice_cursor'",
        [],
        |row| row.get(0),
    )?;
    let cursor_json: Value = serde_json::from_str(&cursor_value)?;
    ensure!(cursor_json["cursor_before"] == "cursor-000");
    ensure!(cursor_json["cursor_after"] == "cursor-001");
    ensure!(cursor_json["backtrack_window_seconds"] == 60);
    Ok(())
}

#[test]
fn e2e_ing_007_empty_fts_batch_completes_without_advancing_cursor() -> Result<()> {
    let harness = RunHarness::new()?;
    let mut fixture = default_fixture();
    set_field(&mut fixture, "notices", json!([]))?;
    set_field(&mut fixture, "cursorBefore", json!("cursor-pre"))?;
    let result = harness.run(&fixture)?;
    ensure!(result.success, "empty batch should still succeed");

    let conn = harness.connection()?;
    ensure!(latest_job_status(&conn)? == "completed");
    let cursor_value: String = conn.query_row(
        "SELECT cursor_value FROM source_cursors WHERE source_id = 'fts' AND cursor_key = 'notice_cursor'",
        [],
        |row| row.get(0),
    )?;
    let cursor_json: Value = serde_json::from_str(&cursor_value)?;
    ensure!(cursor_json["cursor_before"] == "cursor-pre");
    ensure!(cursor_json.get("cursor_after").is_none() || cursor_json["cursor_after"].is_null());
    Ok(())
}

#[test]
fn e2e_ing_008_capture_write_failure_becomes_job_failure() -> Result<()> {
    let harness = RunHarness::new()?;
    let mut fixture = default_fixture();
    set_field(&mut fixture, "captureWriteFailure", json!(true))?;
    let result = harness.run(&fixture)?;
    ensure!(!result.success, "capture failure should fail the run");

    let conn = harness.connection()?;
    ensure!(latest_job_status(&conn)? == "failed");
    ensure!(count_rows(&conn, "manifests")? == 0);
    Ok(())
}

#[test]
fn e2e_ing_009_workflow_progression_uses_validated_contracts_not_transcripts() -> Result<()> {
    let harness = RunHarness::new()?;
    let result = harness.run(&default_fixture())?;
    ensure!(result.success);

    let conn = harness.connection()?;
    let output = job_output_payload(&conn)?;
    ensure!(output["candidate_count"] == 3);
    ensure!(!result.stdout.contains("99 candidates"));
    Ok(())
}

#[test]
fn e2e_ing_010_cli_summary_and_manifest_artifacts_emitted() -> Result<()> {
    let harness = RunHarness::new()?;
    let result = harness.run(&default_fixture())?;
    ensure!(result.success);
    ensure!(result.stdout.contains("FTS live loop completed"));

    let conn = harness.connection()?;
    let manifest_locator: String = conn.query_row(
        "SELECT artifact_locator FROM manifests LIMIT 1",
        [],
        |row| row.get(0),
    )?;
    ensure!(Path::new(&manifest_locator).exists());
    Ok(())
}

#[test]
fn e2e_ing_011_schema_and_oqg_jointly_gate_pipeline_advancement() -> Result<()> {
    let invalid_harness = RunHarness::new()?;
    let mut invalid_fixture = default_fixture();
    set_field(&mut invalid_fixture, "manifestSchemaInvalid", json!(true))?;
    let invalid = invalid_harness.run(&invalid_fixture)?;
    ensure!(!invalid.success);
    let invalid_conn = invalid_harness.connection()?;
    ensure!(latest_job_status(&invalid_conn)? == "failed");
    ensure!(count_rows(&invalid_conn, "manifests")? == 0);
    let invalid_error: String = invalid_conn.query_row(
        "SELECT error_type FROM error_manifests ORDER BY created_at DESC LIMIT 1",
        [],
        |row| row.get(0),
    )?;
    ensure!(invalid_error == "AGENT_OUTPUT_CONTRACT_INVALID");

    let suspect_harness = RunHarness::new()?;
    let mut suspect_fixture = default_fixture();
    set_field(&mut suspect_fixture, "manifestOqgSuspect", json!(true))?;
    let suspect = suspect_harness.run(&suspect_fixture)?;
    ensure!(!suspect.success);
    let suspect_conn = suspect_harness.connection()?;
    ensure!(count_rows(&suspect_conn, "manifests")? == 0);
    let suspect_error: String = suspect_conn.query_row(
        "SELECT error_type FROM error_manifests ORDER BY created_at DESC LIMIT 1",
        [],
        |row| row.get(0),
    )?;
    ensure!(suspect_error == "AGENT_SUSPECT_OUTPUT");
    Ok(())
}

#[test]
fn e2e_ing_012_live_loop_executes_normalization_through_manifest() -> Result<()> {
    let harness = RunHarness::new()?;
    let result = harness.run(&default_fixture())?;
    ensure!(result.success);

    let conn = harness.connection()?;
    let mut stmt =
        conn.prepare("SELECT DISTINCT stage FROM process_stage_status ORDER BY stage")?;
    let rows: Vec<String> = stmt
        .query_map([], |row| row.get(0))?
        .collect::<std::result::Result<_, _>>()?;
    for stage in [
        "Normalize",
        "Prequalify notice",
        "Acquire docs",
        "Extract lots",
        "Qualify lot",
        "Extract products",
        "Score quoteability",
        "Price-signal match",
        "Gross margin gate",
        "Landed cost",
        "Net margin gate",
        "Relate",
        "Manifest",
    ] {
        ensure!(rows.iter().any(|row| row == stage), "missing stage {stage}");
    }
    Ok(())
}

#[test]
fn e2e_ing_013_stage_outputs_commit_atomically() -> Result<()> {
    let harness = RunHarness::new()?;
    let mut fixture = default_fixture();
    set_field(&mut fixture, "sqliteWriteFailure", json!(true))?;
    let result = harness.run(&fixture)?;
    ensure!(!result.success);

    let conn = harness.connection()?;
    ensure!(count_rows(&conn, "procurement_processes")? == 0);
    ensure!(count_rows(&conn, "process_stage_status")? == 0);
    ensure!(count_rows(&conn, "raw_documents")? == 0);
    Ok(())
}

#[test]
fn e2e_ing_014_stage_to_scope_mapping_matches_six_canonical_scope_kinds() -> Result<()> {
    let harness = RunHarness::new()?;
    let result = harness.run(&default_fixture())?;
    ensure!(result.success);

    let conn = harness.connection()?;
    let mut stmt =
        conn.prepare("SELECT DISTINCT scope_kind FROM process_stage_status ORDER BY scope_kind")?;
    let scopes: Vec<String> = stmt
        .query_map([], |row| row.get(0))?
        .collect::<std::result::Result<_, _>>()?;
    ensure!(
        scopes
            == vec![
                "candidate".to_owned(),
                "candidate_quote_scenario".to_owned(),
                "candidate_source_pair".to_owned(),
                "lot".to_owned(),
                "notice".to_owned(),
                "process".to_owned(),
            ]
    );
    Ok(())
}

#[test]
fn e2e_ing_015_set_replacement_never_leaks_partial_writes() -> Result<()> {
    let harness = RunHarness::new()?;
    let success = harness.run(&default_fixture())?;
    ensure!(success.success);

    let conn = harness.connection()?;
    let original_head: String = conn.query_row(
        "SELECT current_stage_run_id FROM process_stage_heads WHERE process_id = 'process-fts-001' AND stage = 'Normalize' AND scope_kind = 'notice' AND scope_id = 'notice-fts-001'",
        [],
        |row| row.get(0),
    )?;

    let mut failure_fixture = default_fixture();
    set_field(
        &mut failure_fixture,
        "setReplacementFailureStage",
        json!("Normalize"),
    )?;
    let failure = harness.run(&failure_fixture)?;
    ensure!(!failure.success);

    let next_head: String = conn.query_row(
        "SELECT current_stage_run_id FROM process_stage_heads WHERE process_id = 'process-fts-001' AND stage = 'Normalize' AND scope_kind = 'notice' AND scope_id = 'notice-fts-001'",
        [],
        |row| row.get(0),
    )?;
    let failed_scope_head: Option<String> = conn
        .query_row(
            "SELECT current_stage_run_id FROM process_stage_heads WHERE process_id = 'process-fts-001' AND stage = 'Normalize' AND scope_kind = 'notice' AND scope_id = 'failed-scope'",
            [],
            |row| row.get(0),
        )
        .optional()?;
    ensure!(next_head == original_head);
    ensure!(failed_scope_head.is_none());
    Ok(())
}

#[test]
fn e2e_ing_016_additive_stages_commit_multiple_scoped_units() -> Result<()> {
    let harness = RunHarness::new()?;
    let result = harness.run(&default_fixture())?;
    ensure!(result.success);

    let conn = harness.connection()?;
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM process_stage_status WHERE stage = 'Score quoteability'",
        [],
        |row| row.get(0),
    )?;
    ensure!(count == 3);
    Ok(())
}

#[test]
fn e2e_ing_017_basis_hash_mismatch_marks_job_obsolete() -> Result<()> {
    let harness = RunHarness::new()?;
    let mut fixture = default_fixture();
    set_field(&mut fixture, "basisHashMismatch", json!(true))?;
    let result = harness.run(&fixture)?;
    ensure!(!result.success);

    let conn = harness.connection()?;
    let original_status: String = conn.query_row(
        "SELECT status FROM workflow_jobs WHERE parent_job_id IS NULL ORDER BY created_at DESC LIMIT 1",
        [],
        |row| row.get(0),
    )?;
    ensure!(original_status == "obsolete");
    let rerun_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM workflow_jobs WHERE kind = 'sync_source' AND status = 'pending'",
        [],
        |row| row.get(0),
    )?;
    ensure!(rerun_count >= 1);
    Ok(())
}

#[test]
fn e2e_ing_018_soft_stale_vs_hard_invalid_follow_cascade_rules() -> Result<()> {
    let harness = RunHarness::new()?;
    let mut fixture = default_fixture();
    set_field(
        &mut fixture,
        "softStaleTargets",
        json!([{ "stage": "Score quoteability", "scopeKind": "candidate", "scopeId": "lot-001-candidate-1" }]),
    )?;
    set_field(
        &mut fixture,
        "hardInvalidTargets",
        json!([{ "stage": "Landed cost", "scopeKind": "candidate_quote_scenario", "scopeId": "scenario-lot-001-candidate-1" }]),
    )?;
    let result = harness.run(&fixture)?;
    ensure!(result.success);

    let conn = harness.connection()?;
    let soft: String = conn.query_row(
        "SELECT freshness_state FROM process_stage_heads WHERE stage = 'Score quoteability' AND scope_id = 'lot-001-candidate-1'",
        [],
        |row| row.get(0),
    )?;
    let hard: String = conn.query_row(
        "SELECT freshness_state FROM process_stage_heads WHERE stage = 'Landed cost' AND scope_id = 'scenario-lot-001-candidate-1'",
        [],
        |row| row.get(0),
    )?;
    ensure!(soft == "soft_stale");
    ensure!(hard == "hard_invalid");
    Ok(())
}

#[test]
fn e2e_ing_019_live_validation_marks_worker_row_and_links_verdict() -> Result<()> {
    let harness = RunHarness::new()?;
    let result = harness.run(&default_fixture())?;
    ensure!(result.success);

    let conn = harness.connection()?;
    let (job_id, parent_job_id, validation_required, validation_verdict_id): (
        String,
        Option<String>,
        i64,
        Option<String>,
    ) = conn.query_row(
        "SELECT id, parent_job_id, validation_required, validation_verdict_id FROM workflow_jobs WHERE parent_job_id IS NULL ORDER BY created_at DESC LIMIT 1",
        [],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
    )?;
    let validation_state: String = conn.query_row(
        "SELECT validation_state FROM pipeline_assessments WHERE process_id = 'process-fts-001'",
        [],
        |row| row.get(0),
    )?;
    let validator_job_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM workflow_jobs WHERE kind = 'semantic_validate' AND parent_job_id = ?1",
        [job_id],
        |row| row.get(0),
    )?;
    ensure!(parent_job_id.is_none());
    ensure!(validation_required == 1);
    ensure!(validation_verdict_id.is_some());
    ensure!(validation_state == "validated");
    ensure!(validator_job_count >= 1);
    Ok(())
}

#[test]
fn e2e_ing_020_experiment_4_hits_success_rate_and_zero_orphans() -> Result<()> {
    let mut successes = 0_u32;
    let mut leaked = 0_i64;

    for index in 0..10 {
        let harness = RunHarness::new()?;
        let mut fixture = default_fixture();
        if index == 9 {
            set_field(&mut fixture, "sourceChanged", json!(true))?;
        }
        let result = harness.run(&fixture)?;
        if result.success {
            successes += 1;
        }
        let conn = harness.connection()?;
        leaked += conn.query_row(
            "SELECT COUNT(*) FROM agent_session_logs WHERE cleanup_verdict = 'leaked'",
            [],
            |row| row.get::<_, i64>(0),
        )?;
    }

    ensure!(successes >= 9);
    ensure!(leaked == 0);
    Ok(())
}

#[test]
fn e2e_ing_021_source_changed_and_notice_parse_failed_preserve_evidence() -> Result<()> {
    let source_harness = RunHarness::new()?;
    let mut source_fixture = default_fixture();
    set_field(&mut source_fixture, "sourceChanged", json!(true))?;
    let source_result = source_harness.run(&source_fixture)?;
    ensure!(!source_result.success);
    let source_conn = source_harness.connection()?;
    let source_error: String = source_conn.query_row(
        "SELECT error_type FROM error_manifests ORDER BY created_at DESC LIMIT 1",
        [],
        |row| row.get(0),
    )?;
    let capture_path: String = source_conn.query_row(
        "SELECT terminal_output_path FROM agent_session_logs WHERE execution_mode = 'pty' LIMIT 1",
        [],
        |row| row.get(0),
    )?;
    ensure!(source_error == "SOURCE_CHANGED");
    ensure!(Path::new(&capture_path).exists());

    let parse_harness = RunHarness::new()?;
    let mut parse_fixture = default_fixture();
    set_field(&mut parse_fixture, "noticeParseFailed", json!(true))?;
    let parse_result = parse_harness.run(&parse_fixture)?;
    ensure!(!parse_result.success);
    let parse_conn = parse_harness.connection()?;
    let parse_error: String = parse_conn.query_row(
        "SELECT error_type FROM error_manifests ORDER BY created_at DESC LIMIT 1",
        [],
        |row| row.get(0),
    )?;
    ensure!(parse_error == "NOTICE_PARSE_FAILED");
    ensure!(count_rows(&parse_conn, "raw_documents")? == 1);
    Ok(())
}

#[test]
fn e2e_ing_022_low_confidence_outcomes_route_to_review() -> Result<()> {
    let harness = RunHarness::new()?;
    let mut fixture = default_fixture();
    set_field(
        &mut fixture,
        "goodsClassificationLowConfidence",
        json!(true),
    )?;
    set_field(&mut fixture, "supplierMatchLowConfidence", json!(true))?;
    set_field(&mut fixture, "commodityCodeLowConfidence", json!(true))?;
    let result = harness.run(&fixture)?;
    ensure!(result.success);

    let conn = harness.connection()?;
    let (review_required, review_reason): (i64, Option<String>) = conn.query_row(
        "SELECT review_required, review_reason FROM pipeline_assessments WHERE process_id = 'process-fts-001'",
        [],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    ensure!(review_required == 1);
    let review_reason = review_reason.unwrap_or_default();
    ensure!(review_reason.contains("GOODS_CLASSIFICATION_LOW_CONFIDENCE"));
    ensure!(review_reason.contains("SUPPLIER_MATCH_LOW_CONFIDENCE"));
    ensure!(review_reason.contains("COMMODITY_CODE_LOW_CONFIDENCE"));
    ensure!(count_rows(&conn, "error_manifests")? >= 3);
    Ok(())
}

#[test]
fn e2e_ing_023_uncertain_revenue_and_fpo_unavailability_reduce_confidence() -> Result<()> {
    let harness = RunHarness::new()?;
    let mut fixture = default_fixture();
    set_field(&mut fixture, "revenueBasisUncertain", json!(true))?;
    set_field(&mut fixture, "fpoAccessUnavailable", json!(true))?;
    let result = harness.run(&fixture)?;
    ensure!(result.success);

    let conn = harness.connection()?;
    let revenue_confidence: i64 = conn.query_row(
        "SELECT revenue_confidence_bps FROM candidate_pricing_assessments LIMIT 1",
        [],
        |row| row.get(0),
    )?;
    let customs_confidence: i64 = conn.query_row(
        "SELECT customs_resolution_confidence_bps FROM landed_cost_estimates LIMIT 1",
        [],
        |row| row.get(0),
    )?;
    ensure!(revenue_confidence < 8400);
    ensure!(customs_confidence < 8300);
    ensure!(count_rows(&conn, "error_manifests")? >= 2);
    Ok(())
}

#[test]
fn e2e_ing_024_live_loop_steps_write_only_declared_durable_surfaces() -> Result<()> {
    let harness = RunHarness::new()?;
    let result = harness.run(&default_fixture())?;
    ensure!(result.success);

    let conn = harness.connection()?;
    let payload = job_output_payload(&conn)?;
    let steps = payload["durable_step_surfaces"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("durable_step_surfaces missing"))?;
    ensure!(steps.len() == 14);
    ensure!(steps[1]["step"] == "job_creation");
    ensure!(steps[1]["sqliteWrites"][0] == "workflow_jobs");
    ensure!(steps[13]["step"] == "verifiable_output");
    ensure!(steps[13]["artifactWrites"][0] == "opportunity manifest markdown");
    Ok(())
}

#[test]
fn e2e_ing_025_manifests_row_references_local_markdown_artifact() -> Result<()> {
    let harness = RunHarness::new()?;
    let result = harness.run(&default_fixture())?;
    ensure!(result.success);

    let conn = harness.connection()?;
    let (markdown, locator): (String, String) = conn.query_row(
        "SELECT markdown, artifact_locator FROM manifests LIMIT 1",
        [],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    let stored = fs::read_to_string(&locator)?;
    ensure!(stored == markdown);
    Ok(())
}

#[test]
fn e2e_ing_026_one_session_proves_set_replacement_and_additive_scoping() -> Result<()> {
    let harness = RunHarness::new()?;
    let mut fixture = default_fixture();
    set_field(
        &mut fixture,
        "notices",
        json!([
          {
            "processId": "process-fts-001",
            "noticeId": "notice-fts-001",
            "sourceNoticeId": "ocds-001",
            "sourceUrl": "https://www.find-tender.service.gov.uk/notice/001",
            "title": "Supply of industrial widgets",
            "buyerName": "Widgetshire Council",
            "publishedAt": "2026-03-25T00:00:00Z",
            "sourceUpdatedAt": "2026-03-25T00:30:00Z",
            "ocid": "ocds-1234",
            "procurementStage": "tender",
            "lots": [{ "lotId": "lot-001", "title": "Widgets lot", "candidateCount": 2, "estimatedValuePence": 120_000 }]
          },
          {
            "processId": "process-fts-001",
            "noticeId": "notice-fts-002",
            "sourceNoticeId": "ocds-002",
            "sourceUrl": "https://www.find-tender.service.gov.uk/notice/002",
            "title": "Supply of emergency widgets",
            "buyerName": "Widgetshire Council",
            "publishedAt": "2026-03-25T01:00:00Z",
            "sourceUpdatedAt": "2026-03-25T01:30:00Z",
            "ocid": "ocds-5678",
            "procurementStage": "tender",
            "lots": [{ "lotId": "lot-003", "title": "Emergency widgets", "candidateCount": 1, "estimatedValuePence": 120_000 }]
          }
        ]),
    )?;
    let result = harness.run(&fixture)?;
    ensure!(result.success);

    let conn = harness.connection()?;
    let normalize_rows: i64 = conn.query_row(
        "SELECT COUNT(*) FROM process_stage_status WHERE stage = 'Normalize'",
        [],
        |row| row.get(0),
    )?;
    let normalize_stage_runs: i64 = conn.query_row(
        "SELECT COUNT(DISTINCT stage_run_id) FROM process_stage_status WHERE stage = 'Normalize'",
        [],
        |row| row.get(0),
    )?;
    let additive_rows: i64 = conn.query_row(
        "SELECT COUNT(*) FROM process_stage_status WHERE stage = 'Score quoteability'",
        [],
        |row| row.get(0),
    )?;
    ensure!(normalize_rows == 2);
    ensure!(normalize_stage_runs == 2);
    ensure!(additive_rows == 3);
    Ok(())
}

#[test]
fn e2e_ing_027_validator_fail_open_keeps_worker_completed_and_queues_backlog() -> Result<()> {
    let harness = RunHarness::new()?;
    let mut fixture = default_fixture();
    set_field(&mut fixture, "validatorFailOpen", json!(true))?;
    let result = harness.run(&fixture)?;
    ensure!(result.success);

    let conn = harness.connection()?;
    let (job_id, status, validation_required, validation_verdict_id): (
        String,
        String,
        i64,
        Option<String>,
    ) = conn.query_row(
        "SELECT id, status, validation_required, validation_verdict_id FROM workflow_jobs WHERE parent_job_id IS NULL ORDER BY created_at DESC LIMIT 1",
        [],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
    )?;
    let validation_state: String = conn.query_row(
        "SELECT validation_state FROM pipeline_assessments WHERE process_id = 'process-fts-001'",
        [],
        |row| row.get(0),
    )?;
    let output = job_output_payload(&conn)?;
    let failed_validator_jobs: i64 = conn.query_row(
        "SELECT COUNT(*) FROM workflow_jobs WHERE kind = 'semantic_validate' AND status = 'failed' AND parent_job_id = ?1",
        [&job_id],
        |row| row.get(0),
    )?;
    let pending_backlog_jobs: i64 = conn.query_row(
        "SELECT COUNT(*) FROM workflow_jobs WHERE kind = 'semantic_validate' AND status = 'pending' AND parent_job_id = ?1",
        [&job_id],
        |row| row.get(0),
    )?;

    ensure!(status == "completed");
    ensure!(validation_required == 1);
    ensure!(validation_verdict_id.is_none());
    ensure!(validation_state == "unverified");
    ensure!(output["validation_marker"] == "VALIDATION_SKIPPED_VALIDATOR_ERROR");
    ensure!(failed_validator_jobs >= 1);
    ensure!(pending_backlog_jobs >= 1);
    Ok(())
}

#[test]
fn e2e_ing_028_procontract_interest_replay_persistence_failure_marks_job_failed() -> Result<()> {
    let harness = RunHarness::new()?;
    let result_contract = json!({
        "cursor": "{}",
        "continuation_required": false,
        "registrations": [
            {
                "notice_id": "pc-invalid-action",
                "process_id": "proc-pc-invalid-action",
                "source_url": "https://example.invalid/pc-invalid-action",
                "action": "bogus_action"
            }
        ],
        "summary": {
            "total_evaluated": 1,
            "registered": 0,
            "already_registered": 0,
            "pending_registration": 0,
            "skipped_service": 0,
            "skipped_expiry": 0,
            "failed": 0
        }
    });

    let result = harness.run_procontract_interest_replay(&result_contract)?;
    ensure!(
        !result.success,
        "invalid replay should fail the CLI, stdout={}, stderr={}",
        result.stdout,
        result.stderr
    );

    let conn = harness.connection()?;
    let (status, last_error, output_payload): (String, Option<String>, Option<String>) = conn
        .query_row(
            "\
            SELECT status, last_error, output_payload
            FROM workflow_jobs
            WHERE scope_id = 'procontract-interest'
            ORDER BY created_at DESC
            LIMIT 1
            ",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )?;
    ensure!(status == "failed", "latest job status was {status}");
    let last_error = last_error.unwrap_or_default();
    ensure!(
        last_error.contains("unsupported ProContract registration action"),
        "unexpected last_error: {last_error}"
    );
    let payload: Value = serde_json::from_str(
        output_payload
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("failed job missing output_payload"))?,
    )?;
    ensure!(payload["error_code"] == "PROCONTRACT_INTEREST_PERSISTENCE_FAILED");
    ensure!(payload["source_id"] == "procontract-interest");
    ensure!(payload["replay_result_path"].as_str().is_some());
    Ok(())
}
