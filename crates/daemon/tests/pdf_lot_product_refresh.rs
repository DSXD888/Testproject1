#![forbid(unsafe_code)]

mod support;

#[path = "support/skills.rs"]
#[allow(dead_code)]
mod skills_support;

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result, ensure};
use assert_cmd::Command;
use rusqlite::{Connection, params};
use serde_json::{Value, json};
use tempfile::TempDir;

use crate::support::db::{DocumentExtractionSeed, StateDb, seed_document_extraction};
use crate::support::pdf::sample_semantic_result;
use skills_support::repo_skill_dir;

fn tender_spec_schema() -> Result<Value> {
    let path = repo_skill_dir("pdf-extraction/tender-specification")?.join("result.schema.json");
    let raw = fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    serde_json::from_str(&raw).with_context(|| format!("parsing {}", path.display()))
}

#[test]
fn test_tender_spec_schema_exposes_lot_refresh_fields() -> Result<()> {
    let schema = tender_spec_schema()?;
    let lot_required = schema["properties"]["lots"]["items"]["required"]
        .as_array()
        .context("lots item required array missing")?;

    for field in [
        "lot_number",
        "title",
        "description",
        "requirements",
        "evaluation_criteria",
        "cpv_codes",
        "product_requirements",
        "technical_standards",
        "evidence_anchor",
    ] {
        ensure!(
            lot_required.contains(&Value::String(field.to_owned())),
            "lot refresh contract missing field {field}"
        );
    }
    Ok(())
}

#[test]
fn test_tender_spec_schema_exposes_product_seed_refresh_fields() -> Result<()> {
    let schema = tender_spec_schema()?;
    let seed_required = schema["properties"]["product_requirement_seeds"]["items"]["required"]
        .as_array()
        .context("product_requirement_seeds required array missing")?;

    for field in [
        "seed_key",
        "lot_number",
        "candidate_name",
        "summary",
        "requirements",
        "evidence_anchor",
    ] {
        ensure!(
            seed_required.contains(&Value::String(field.to_owned())),
            "product seed refresh contract missing field {field}"
        );
    }
    Ok(())
}

struct LotRefreshHarness {
    _temp: TempDir,
    state_root: PathBuf,
    runtime_root: PathBuf,
    artifact_root: PathBuf,
    db_path: PathBuf,
}

struct RunResult {
    stderr: String,
    success: bool,
}

struct FakeClaudeCli {
    program: PathBuf,
}

impl LotRefreshHarness {
    fn new() -> Result<Self> {
        let temp = tempfile::tempdir()?;
        let root = temp.path().to_path_buf();
        let state_root = root.join("state");
        let runtime_root = root.join("runtime");
        let artifact_root = root.join("artifacts");
        fs::create_dir_all(&state_root)?;
        fs::create_dir_all(&runtime_root)?;
        fs::create_dir_all(&artifact_root)?;
        let db_path = state_root.join("state.db");
        let _ = StateDb::open(&db_path)?;
        Ok(Self {
            _temp: temp,
            state_root,
            runtime_root,
            artifact_root,
            db_path,
        })
    }

    fn connection(&self) -> Result<Connection> {
        Connection::open(&self.db_path)
            .with_context(|| format!("opening {}", self.db_path.display()))
    }

    fn seed_process(&self, process_id: &str) -> Result<()> {
        let conn = self.connection()?;
        conn.execute(
            "\
            INSERT INTO procurement_processes (
              id, current_stage, title, latest_published_at, latest_updated_at, owner_id
            ) VALUES (?1, 'acquire docs', 'Process title', '2026-04-12T00:00:00Z',
                      '2026-04-12T00:00:00Z', 'local-owner')
            ",
            params![process_id],
        )?;
        Ok(())
    }

    fn seed_notice(&self, process_id: &str, notice_id: &str, raw_document_id: &str) -> Result<()> {
        let conn = self.connection()?;
        conn.execute(
            "\
            INSERT INTO contract_notices (
              id, process_id, source_id, source_notice_id, raw_document_id, title,
              procurement_stage, published_at, source_updated_at, normalized_payload, owner_id
            ) VALUES (?1, ?2, 'fts', ?3, ?4, 'Notice title', 'tender',
                      '2026-04-12T00:00:00Z', '2026-04-12T00:00:00Z', '{}', 'local-owner')
            ",
            params![
                notice_id,
                process_id,
                format!("source-{notice_id}"),
                raw_document_id
            ],
        )?;
        Ok(())
    }

    fn seed_document_extraction(
        &self,
        extraction_id: &str,
        raw_document_id: &str,
        process_id: &str,
        notice_id: &str,
    ) -> Result<()> {
        let conn = self.connection()?;
        seed_document_extraction(
            &conn,
            &DocumentExtractionSeed {
                id: extraction_id,
                raw_document_id,
                process_id,
                notice_id: Some(notice_id),
                document_set_id: None,
                source_content_hash: "refresh-hash",
                document_role: Some("technical_specification"),
                semantic_extraction_state: "pending",
            },
        )?;
        conn.execute(
            "UPDATE document_extractions SET expected_overlays_json = ?1 WHERE id = ?2",
            params![
                serde_json::to_string(&["pdf-extraction/tender-specification"])?,
                extraction_id
            ],
        )?;
        Ok(())
    }

    fn seed_stage_head(
        &self,
        process_id: &str,
        stage: &str,
        scope_kind: &str,
        scope_id: &str,
    ) -> Result<()> {
        let conn = self.connection()?;
        conn.execute(
            "\
            INSERT INTO process_stage_heads (
              process_id, stage, scope_kind, scope_id, current_stage_run_id, owner_id
            ) VALUES (?1, ?2, ?3, ?4, ?5, 'local-owner')
            ",
            params![
                process_id,
                stage,
                scope_kind,
                scope_id,
                format!("run-{stage}-{scope_id}")
            ],
        )?;
        Ok(())
    }

    fn seed_current_semantic_result(
        &self,
        semantic_id: &str,
        document_extraction_id: &str,
        process_id: &str,
        result_json: &Value,
    ) -> Result<()> {
        let conn = self.connection()?;
        conn.execute(
            "\
            INSERT INTO document_semantic_extractions (
              id, document_extraction_id, process_id, skill_id, skill_version, job_id, session_id,
              extraction_scope, target_domain, is_current, extracted_data_json, field_confidences_json,
              extraction_confidence_bps, warnings_json, evidence_anchors_json, written_entity_refs_json,
              owner_id
            ) VALUES (?1, ?2, ?3, 'pdf-extraction/tender-specification', '2026.04.12', ?4, ?5,
                      'full_document', 'lots', 1, ?6, ?7, ?8, ?9, ?10, ?11, 'local-owner')
            ",
            params![
                semantic_id,
                document_extraction_id,
                process_id,
                format!("job-{semantic_id}"),
                format!("session-{semantic_id}"),
                result_json.to_string(),
                json!({"lots[0].title": 9100}).to_string(),
                9100,
                json!([]).to_string(),
                json!([]).to_string(),
                json!([
                    {"table": "notice_lots", "id": lot_id_for_extraction(document_extraction_id)},
                    {
                        "table": "product_candidates",
                        "id": candidate_id_for_extraction(document_extraction_id)
                    }
                ])
                .to_string(),
            ],
        )?;
        conn.execute(
            "UPDATE document_extractions SET semantic_extraction_state = 'complete' WHERE id = ?1",
            params![document_extraction_id],
        )?;
        Ok(())
    }

    fn seed_semantic_job(
        &self,
        job_id: &str,
        process_id: &str,
        document_extraction_id: &str,
        notice_id: &str,
    ) -> Result<()> {
        let conn = self.connection()?;
        conn.execute(
            "\
            INSERT INTO workflow_jobs (
              id, kind, process_id, notice_id, scope_kind, scope_id, status, scheduled_for, input_payload
            ) VALUES (?1, 'extract_document_semantics', ?2, ?3, 'notice', ?4, 'pending', ?5, ?6)
            ",
            params![
                job_id,
                process_id,
                notice_id,
                notice_id,
                "1970-01-01T00:00:00Z",
                json!({
                    "request": {
                        "document_extraction_id": document_extraction_id,
                        "process_id": process_id,
                        "notice_id": notice_id,
                        "skill_id": "pdf-extraction/tender-specification",
                        "extraction_scope": "full_document",
                        "target_domain": "lots",
                        "stage_run_id": format!("stage-run-{job_id}")
                    }
                })
                .to_string(),
            ],
        )?;
        Ok(())
    }

    fn prepare_fake_claude(&self, result_payload: &Value) -> Result<FakeClaudeCli> {
        let state_dir = self.state_root.join("fake-claude");
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
        fs::write(&result_source, serde_json::to_vec_pretty(result_payload)?)?;
        let program = state_dir.join("claude");
        fs::write(
            &program,
            format!(
                "#!/bin/sh\nset -eu\nstate_dir=\"{}\"\nif [ \"${{1-}}\" = \"--version\" ] || [ \"${{1-}}\" = \"-v\" ]; then\n  printf 'claude 1.2.3\\n'\n  exit 0\nfi\nif [ \"${{1-}}\" = \"probe\" ]; then\n  cat \"$state_dir/probe.json\"\n  exit 0\nfi\nmode=pty\nfor arg in \"$@\"; do\n  if [ \"$arg\" = \"--output-format\" ]; then\n    mode=contract\n    break\n  fi\ndone\nif [ \"$mode\" = contract ]; then\n  cp \"$state_dir/result-source.json\" \"$PWD/result.json\"\n  printf '{{\"event\":\"result\",\"path\":\"%s/result.json\"}}\\n' \"$PWD\"\nelse\n  printf 'unexpected pty mode\\n' >&2\n  exit 1\nfi\n",
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
        Ok(FakeClaudeCli { program })
    }

    fn run_next_job(&self, result_payload: &Value) -> Result<RunResult> {
        let fake_claude = self.prepare_fake_claude(result_payload)?;
        let output = Command::cargo_bin("daemon")?
            .arg("--run-next-job")
            .arg("--state-root")
            .arg(&self.state_root)
            .arg("--runtime-dir")
            .arg(&self.runtime_root)
            .arg("--artifact-root")
            .arg(&self.artifact_root)
            .env("DAEMON_CLAUDE_PROGRAM", &fake_claude.program)
            .output()
            .context("running daemon --run-next-job for lot refresh")?;
        Ok(RunResult {
            stderr: String::from_utf8(output.stderr).context("decoding stderr")?,
            success: output.status.success(),
        })
    }
}

fn job_output(conn: &Connection, job_id: &str) -> Result<Value> {
    let payload: String = conn.query_row(
        "SELECT output_payload FROM workflow_jobs WHERE id = ?1",
        params![job_id],
        |row| row.get(0),
    )?;
    serde_json::from_str(&payload).context("parsing workflow_jobs.output_payload")
}

fn stage_freshness(
    conn: &Connection,
    process_id: &str,
    stage: &str,
    scope_kind: &str,
    scope_id: &str,
) -> Result<String> {
    conn.query_row(
        "\
        SELECT freshness_state
        FROM process_stage_heads
        WHERE process_id = ?1 AND stage = ?2 AND scope_kind = ?3 AND scope_id = ?4
        ",
        params![process_id, stage, scope_kind, scope_id],
        |row| row.get(0),
    )
    .context("reading process_stage_heads.freshness_state")
}

fn lot_id_for_extraction(document_extraction_id: &str) -> String {
    format!("{document_extraction_id}-lot-1")
}

fn candidate_id_for_extraction(document_extraction_id: &str) -> String {
    format!("{document_extraction_id}-candidate-cabinet-std-01")
}

#[test]
fn test_downstream_trigger_on_change() -> Result<()> {
    let harness = LotRefreshHarness::new()?;
    harness.seed_process("proc-refresh-change")?;
    harness.seed_notice(
        "proc-refresh-change",
        "notice-refresh-change",
        "raw-refresh-change",
    )?;
    harness.seed_document_extraction(
        "extract-refresh-change",
        "raw-refresh-change",
        "proc-refresh-change",
        "notice-refresh-change",
    )?;
    harness.seed_stage_head(
        "proc-refresh-change",
        "Extract lots",
        "notice",
        "notice-refresh-change",
    )?;
    harness.seed_stage_head(
        "proc-refresh-change",
        "Extract products",
        "lot",
        &lot_id_for_extraction("extract-refresh-change"),
    )?;

    let mut previous_result = sample_semantic_result("pdf-extraction/tender-specification");
    previous_result["lots"][0]["title"] = Value::String("Legacy storage cabinets".to_owned());
    harness.seed_current_semantic_result(
        "semantic-refresh-old",
        "extract-refresh-change",
        "proc-refresh-change",
        &previous_result,
    )?;
    harness.seed_semantic_job(
        "job-refresh-change",
        "proc-refresh-change",
        "extract-refresh-change",
        "notice-refresh-change",
    )?;

    let run = harness.run_next_job(&sample_semantic_result(
        "pdf-extraction/tender-specification",
    ))?;
    ensure!(run.success, "stderr: {}", run.stderr);

    let conn = harness.connection()?;
    let output = job_output(&conn, "job-refresh-change")?;
    ensure!(
        output["result"]["downstream_refresh_required"] == Value::Bool(true),
        "changed authoritative output must trigger downstream refresh"
    );
    ensure!(
        stage_freshness(
            &conn,
            "proc-refresh-change",
            "Extract lots",
            "notice",
            "notice-refresh-change",
        )? == "soft_stale"
    );
    ensure!(
        stage_freshness(
            &conn,
            "proc-refresh-change",
            "Extract products",
            "lot",
            &lot_id_for_extraction("extract-refresh-change"),
        )? == "soft_stale"
    );
    Ok(())
}

#[test]
fn test_no_trigger_on_identical() -> Result<()> {
    let harness = LotRefreshHarness::new()?;
    harness.seed_process("proc-refresh-same")?;
    harness.seed_notice(
        "proc-refresh-same",
        "notice-refresh-same",
        "raw-refresh-same",
    )?;
    harness.seed_document_extraction(
        "extract-refresh-same",
        "raw-refresh-same",
        "proc-refresh-same",
        "notice-refresh-same",
    )?;
    harness.seed_stage_head(
        "proc-refresh-same",
        "Extract lots",
        "notice",
        "notice-refresh-same",
    )?;
    harness.seed_stage_head(
        "proc-refresh-same",
        "Extract products",
        "lot",
        &lot_id_for_extraction("extract-refresh-same"),
    )?;

    let identical_result = sample_semantic_result("pdf-extraction/tender-specification");
    harness.seed_current_semantic_result(
        "semantic-refresh-same-old",
        "extract-refresh-same",
        "proc-refresh-same",
        &identical_result,
    )?;
    harness.seed_semantic_job(
        "job-refresh-same",
        "proc-refresh-same",
        "extract-refresh-same",
        "notice-refresh-same",
    )?;

    let run = harness.run_next_job(&identical_result)?;
    ensure!(run.success, "stderr: {}", run.stderr);

    let conn = harness.connection()?;
    let output = job_output(&conn, "job-refresh-same")?;
    ensure!(
        output["result"]["downstream_refresh_required"] == Value::Bool(false),
        "identical authoritative output must not trigger downstream refresh"
    );
    ensure!(
        stage_freshness(
            &conn,
            "proc-refresh-same",
            "Extract lots",
            "notice",
            "notice-refresh-same",
        )? == "fresh"
    );
    ensure!(
        stage_freshness(
            &conn,
            "proc-refresh-same",
            "Extract products",
            "lot",
            &lot_id_for_extraction("extract-refresh-same"),
        )? == "fresh"
    );
    Ok(())
}
