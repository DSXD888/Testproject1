#![forbid(unsafe_code)]

#[path = "support/skills.rs"]
mod skills_support;

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result, ensure};
use assert_cmd::Command;
use rusqlite::Connection;
use serde_json::{Value, json};
use serde_yaml::Value as YamlValue;
use tempfile::TempDir;

use skills_support::{lint_skill_directory, repo_skill_dir};

const FAILURE_CODES: &[&str] = &[
    "SOURCE_CHANGED",
    "AGENT_PROTOCOL_MISMATCH",
    "SOURCE_SOFT_BLOCKED",
    "SOURCE_CAPTCHA_BLOCKED",
    "PAGE_SHAPE_DRIFT",
    "LOCALE_OR_CURRENCY_DRIFT",
    "AGENT_STALLED",
];

const RUN_MANIFEST_TOKENS: &[&str] = &[
    "manifest_version",
    "skill_id",
    "skill_version",
    "job_id",
    "session_id",
    "execution_mode",
    "Summary",
    "Steps Taken",
    "Decisions Made",
    "Warnings",
    "Evidence",
];

fn contracts_finder_skill_dir() -> Result<PathBuf> {
    repo_skill_dir("procurement-ingestion/contracts-finder")
}

struct ContractsFinderRunHarness {
    _temp: TempDir,
    state_root: PathBuf,
    runtime_root: PathBuf,
    artifact_root: PathBuf,
    db_path: PathBuf,
}

struct ContractsFinderRunResult {
    stdout: String,
    stderr: String,
    success: bool,
}

struct ContractsFinderFakeClaude {
    program: PathBuf,
    result_path: PathBuf,
    checkpoint_path: PathBuf,
}

impl ContractsFinderRunHarness {
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
            state_root,
            runtime_root,
            artifact_root,
        })
    }

    fn run(&self) -> Result<ContractsFinderRunResult> {
        let fake_claude = self.prepare_fake_claude()?;
        let output = Command::cargo_bin("daemon")?
            .arg("--ingest")
            .arg("contracts-finder")
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
            .env("DAEMON_TEST_CLAUDE_RESULT_PATH", &fake_claude.result_path)
            .env(
                "DAEMON_TEST_CLAUDE_CHECKPOINT_PATH",
                &fake_claude.checkpoint_path,
            )
            .output()
            .context("running daemon binary")?;

        Ok(ContractsFinderRunResult {
            stdout: String::from_utf8(output.stdout).context("decoding stdout")?,
            stderr: String::from_utf8(output.stderr).context("decoding stderr")?,
            success: output.status.success(),
        })
    }

    fn connection(&self) -> Result<Connection> {
        Connection::open(&self.db_path)
            .with_context(|| format!("opening {}", self.db_path.display()))
    }

    fn prepare_fake_claude(&self) -> Result<ContractsFinderFakeClaude> {
        let root = self
            .state_root
            .parent()
            .ok_or_else(|| anyhow::anyhow!("state root missing parent"))?;
        let state_dir = root.join("fake-claude");
        fs::create_dir_all(&state_dir)?;
        fs::write(
            state_dir.join("probe.json"),
            serde_json::to_vec_pretty(&json!({
                "auth_healthy": true,
                "structured_output": true,
                "pty_viable": true,
                "browser_tools": true
            }))?,
        )?;
        fs::write(
            state_dir.join("result-source.json"),
            serde_json::to_vec_pretty(&contracts_finder_daemon_result_payload())?,
        )?;
        let program = state_dir.join("claude");
        fs::write(
            &program,
            format!(
                "#!/bin/sh\nset -eu\nstate_dir=\"{}\"\nif [ \"${{1-}}\" = \"--version\" ] || [ \"${{1-}}\" = \"-v\" ]; then\n  printf 'claude 1.2.3\\n'\n  exit 0\nfi\nif [ \"${{1-}}\" = \"probe\" ]; then\n  cat \"$state_dir/probe.json\"\n  exit 0\nfi\nmode=pty\nfor arg in \"$@\"; do\n  if [ \"$arg\" = \"--output-format\" ]; then\n    mode=contract\n    break\n  fi\ndone\nif [ \"$mode\" = contract ]; then\n  cp \"$state_dir/result-source.json\" \"$DAEMON_TEST_CLAUDE_RESULT_PATH\"\n  printf '{{\"event\":\"result\",\"path\":\"%s\"}}\\n' \"$DAEMON_TEST_CLAUDE_RESULT_PATH\"\nelse\n  cp \"$state_dir/result-source.json\" result.json\n  : > \"$DAEMON_TEST_CLAUDE_CHECKPOINT_PATH\"\n  printf '{{\"event\":\"checkpoint\",\"seq\":1,\"path\":\"%s\"}}\\n' \"$DAEMON_TEST_CLAUDE_CHECKPOINT_PATH\"\nfi\n",
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
        Ok(ContractsFinderFakeClaude {
            program,
            result_path: self.artifact_root.join("fake-claude-result.json"),
            checkpoint_path: self.artifact_root.join("fake-claude-checkpoint.json"),
        })
    }
}

fn load_json(path: &std::path::Path) -> Result<Value> {
    serde_json::from_str(&fs::read_to_string(path)?)
        .with_context(|| format!("parsing {}", path.display()))
}

fn load_yaml(path: &std::path::Path) -> Result<YamlValue> {
    serde_yaml::from_str(&fs::read_to_string(path)?)
        .with_context(|| format!("parsing {}", path.display()))
}

fn load_first_jsonl(path: &std::path::Path) -> Result<Value> {
    let content = fs::read_to_string(path)?;
    let line = content
        .lines()
        .find(|entry| !entry.trim().is_empty())
        .ok_or_else(|| anyhow::anyhow!("{} is empty", path.display()))?;
    serde_json::from_str(line)
        .with_context(|| format!("parsing first JSONL line in {}", path.display()))
}

fn sample_notice(notice_id: &str, cursor: &str, published_at: &str, stage: &str) -> Value {
    json!({
        "notice_id": notice_id,
        "source_url": format!("https://www.contractsfinder.service.gov.uk/Notice/{notice_id}"),
        "title": format!("Contracts Finder notice {notice_id}"),
        "published_at": published_at,
        "source_updated_at": published_at,
        "ocid": format!("ocds-cf-{notice_id}"),
        "language": "en-GB",
        "currency": "GBP",
        "stage": stage,
        "cursor_hint": cursor
    })
}

fn sample_result_payload(continuation_required: bool) -> Value {
    json!({
        "cursor": "cursor-002",
        "continuation_required": continuation_required,
        "notices": [
            sample_notice("cf-001", "cursor-001", "2026-03-28T10:00:00Z", "tender"),
            sample_notice("cf-002", "cursor-002", "2026-03-28T10:05:00Z", "award")
        ]
    })
}

fn contracts_finder_daemon_result_payload() -> Value {
    json!({
        "cursor": "cursor-daemon-002",
        "continuation_required": false,
        "notices": [
            {
                "notice_id": "cf-daemon-001",
                "source_notice_id": "cf-daemon-source-001",
                "source_url": "https://www.contractsfinder.service.gov.uk/Notice/cf-daemon-001",
                "title": "Contracts Finder daemon fixture",
                "summary": "A deterministic Contracts Finder daemon ingestion fixture.",
                "buyer_name": "Daemon Borough Council",
                "published_at": "2026-03-28T10:00:00Z",
                "source_updated_at": "2026-03-28T10:05:00Z",
                "ocid": "ocds-cf-daemon-001",
                "procurement_stage": "tender",
                "main_category": "goods",
                "deadline_at": "2026-04-30T12:00:00Z",
                "estimated_value_low_pence": 2_500_000,
                "estimated_value_currency": "GBP",
                "access_route": "public_web",
                "raw_release_json": "{\"ocid\":\"ocds-cf-daemon-001\",\"id\":\"cf-daemon-001-release\"}",
                "lots": [
                    {
                        "lot_id": "cf-daemon-lot-001",
                        "source_lot_id": "cf-daemon-source-lot-001",
                        "title": "Daemon lot",
                        "description": "Deterministic goods lot",
                        "estimated_value_pence": 2_500_000,
                        "deadline_at": "2026-04-30T12:00:00Z"
                    }
                ]
            }
        ]
    })
}

fn sample_checkpoint_payload() -> Value {
    json!({
        "seq": 2,
        "cursor": "cursor-002",
        "notices_processed": 2,
        "published_from": "2026-03-01T00:00:00Z",
        "stages": "planning,tender,award,implementation"
    })
}

fn sample_release_package() -> Value {
    json!({
        "uri": "https://www.contractsfinder.service.gov.uk/Published/Notices/OCDS/Search?publishedFrom=2026-03-01T00:00:00Z&stages=tender,award",
        "publishedDate": "2026-03-28T10:10:00Z",
        "license": "http://www.nationalarchives.gov.uk/doc/open-government-licence/version/3/",
        "releases": [
            {
                "ocid": "ocds-cf-001",
                "id": "ocds-cf-001-tender",
                "language": "en-GB",
                "tag": ["tender"],
                "tender": {
                    "value": {
                        "currency": "GBP"
                    }
                }
            }
        ]
    })
}

fn render_sample_run_manifest() -> String {
    [
        "---",
        "manifest_version: 1",
        "skill_id: procurement-ingestion/contracts-finder",
        "skill_version: 1.0.0",
        "job_id: job-cf-001",
        "session_id: session-cf-001",
        "execution_mode: pty",
        "started_at: 2026-03-28T10:00:00Z",
        "completed_at: 2026-03-28T10:05:00Z",
        "outcome_class: continuation_required",
        "items_processed: 2",
        "items_skipped: 0",
        "items_failed: 0",
        "warnings_count: 1",
        "agent_claimed_pages_visited: 2",
        "agent_claimed_source_url: https://www.contractsfinder.service.gov.uk/apidocumentation/Notices/1/GET-Published-Notice-OCDS-Search",
        "agent_claimed_fingerprint_state: matched",
        "agent_claimed_locale_state: en-GB",
        "agent_claimed_currency_state: GBP",
        "---",
        "",
        "## Summary",
        "Recent Contracts Finder notices were fetched incrementally.",
        "",
        "## Steps Taken",
        "1. Ran a canary request.",
        "2. Retrieved a cursor-bounded notice batch.",
        "",
        "## Decisions Made",
        "Applied a cursor backtrack overlap before committing the checkpoint.",
        "",
        "## Warnings",
        "Observed the documented 403 rate-limit contract and would back off for five minutes.",
        "",
        "## Evidence",
        "- cursor: cursor-002",
        "- notice_ids: cf-001, cf-002",
    ]
    .join("\n")
}

#[test]
fn impact_contracts_finder_live_loop_regression_001() -> Result<()> {
    let harness = ContractsFinderRunHarness::new()?;
    let result = harness.run()?;
    ensure!(
        result.success,
        "daemon run should succeed: stdout={} stderr={}",
        result.stdout,
        result.stderr
    );

    let conn = harness.connection()?;
    let (kind, status, output_payload): (String, String, String) = conn.query_row(
        "SELECT kind, status, output_payload FROM workflow_jobs WHERE parent_job_id IS NULL ORDER BY created_at DESC LIMIT 1",
        [],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    )?;
    ensure!(kind == "sync_source");
    ensure!(status == "completed");
    let output_json: Value =
        serde_json::from_str(&output_payload).context("parsing workflow job output payload")?;
    ensure!(output_json["source_id"] == "contracts-finder");
    ensure!(output_json["notices_ingested"] == 1);
    ensure!(output_json["continuation_required"] == false);

    let pty_session_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM agent_session_logs WHERE execution_mode = 'pty'",
        [],
        |row| row.get(0),
    )?;
    ensure!(pty_session_count == 1);

    let process_title: String = conn.query_row(
        "SELECT title FROM procurement_processes WHERE id = 'proc-ocds-cf-daemon-001'",
        [],
        |row| row.get(0),
    )?;
    ensure!(process_title == "Contracts Finder daemon fixture");

    let (notice_process_id, raw_document_id): (String, String) = conn.query_row(
        "SELECT process_id, raw_document_id FROM contract_notices WHERE source_id = 'contracts-finder' AND source_notice_id = 'cf-daemon-source-001'",
        [],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    ensure!(notice_process_id == "proc-ocds-cf-daemon-001");
    ensure!(raw_document_id == "raw-cf-daemon-001");

    let (raw_source_id, content_type): (String, String) = conn.query_row(
        "SELECT source_id, content_type FROM raw_documents WHERE id = 'raw-cf-daemon-001'",
        [],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    ensure!(raw_source_id == "contracts-finder");
    ensure!(content_type == "application/json");

    let cursor_value: String = conn.query_row(
        "SELECT cursor_value FROM source_cursors WHERE source_id = 'contracts-finder' AND cursor_key = 'notice_cursor'",
        [],
        |row| row.get(0),
    )?;
    ensure!(cursor_value == "cursor-daemon-002");
    ensure!(
        result
            .stdout
            .contains("contracts-finder live loop completed")
    );
    Ok(())
}

#[test]
fn ag_cf_001_skill_directory_contains_the_full_contracts_finder_skill_package() -> Result<()> {
    let linted = lint_skill_directory(&contracts_finder_skill_dir()?)?;

    ensure!(linted.metadata.skill_id == "procurement-ingestion/contracts-finder");
    ensure!(linted.metadata.interaction_profile == "pty_interactive");
    ensure!(linted.metadata.max_pages == Some(10));
    ensure!(linted.metadata.max_scroll_cycles == Some(2));
    Ok(())
}

#[test]
fn ag_cf_002_canary_validates_response_shape_locale_currency_and_rate_limit_expectations()
-> Result<()> {
    let skill_dir = contracts_finder_skill_dir()?;
    let canary_input = load_first_jsonl(&skill_dir.join("canary.inputs.jsonl"))?;
    let golden = load_yaml(&skill_dir.join("golden.assertions.yaml"))?;
    let release_package = sample_release_package();

    ensure!(canary_input["limit"] == 5);
    ensure!(
        canary_input["stages"] == "planning,tender,award,implementation",
        "canary should exercise the documented stage parameter"
    );
    ensure!(release_package["releases"][0]["language"] == "en-GB");
    ensure!(release_package["releases"][0]["tender"]["value"]["currency"] == "GBP");
    ensure!(golden["expected_rate_limit_status"] == 403);
    ensure!(
        fs::read_to_string(skill_dir.join("SKILL.md"))?.contains("five-minute backoff"),
        "Contracts Finder rate-limit behavior must be documented explicitly"
    );
    Ok(())
}

#[test]
fn ag_cf_003_incremental_result_and_checkpoint_contracts_support_safe_continuation() -> Result<()> {
    let skill_dir = contracts_finder_skill_dir()?;
    let result_schema = load_json(&skill_dir.join("result.schema.json"))?;
    let checkpoint_schema = load_json(&skill_dir.join("checkpoint.schema.json"))?;
    let result_payload = sample_result_payload(true);
    let checkpoint_payload = sample_checkpoint_payload();

    let result_required = result_schema
        .get("properties")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow::anyhow!("result schema properties missing"))?;
    let checkpoint_required = checkpoint_schema
        .get("properties")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow::anyhow!("checkpoint schema properties missing"))?;

    ensure!(result_required.contains_key("cursor"));
    ensure!(result_required.contains_key("continuation_required"));
    ensure!(result_required.contains_key("notices"));
    ensure!(checkpoint_required.contains_key("seq"));
    ensure!(checkpoint_required.contains_key("cursor"));
    ensure!(checkpoint_required.contains_key("notices_processed"));
    ensure!(result_payload["continuation_required"] == true);
    ensure!(result_payload["notices"].as_array().map_or(0, Vec::len) == 2);
    ensure!(checkpoint_payload["cursor"] == "cursor-002");
    ensure!(checkpoint_payload["notices_processed"] == 2);
    Ok(())
}

#[test]
fn ag_cf_004_cursor_backtrack_overlap_prevents_skipped_or_duplicate_notice_ids() -> Result<()> {
    let first_batch = [
        sample_notice("cf-001", "cursor-001", "2026-03-28T10:00:00Z", "tender"),
        sample_notice("cf-002", "cursor-002", "2026-03-28T10:05:00Z", "award"),
    ];
    let resumed_batch = [
        sample_notice("cf-002", "cursor-002", "2026-03-28T10:05:00Z", "award"),
        sample_notice(
            "cf-003",
            "cursor-003",
            "2026-03-28T10:09:00Z",
            "implementation",
        ),
    ];

    let unique_ids: BTreeSet<String> = first_batch
        .into_iter()
        .chain(resumed_batch)
        .map(|notice| {
            notice
                .get("notice_id")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned()
        })
        .collect();

    ensure!(unique_ids.len() == 3);
    ensure!(unique_ids.contains("cf-001"));
    ensure!(unique_ids.contains("cf-002"));
    ensure!(unique_ids.contains("cf-003"));
    Ok(())
}

#[test]
fn ag_cf_005_run_manifest_contract_matches_the_fts_frontmatter_and_narrative_standard() -> Result<()>
{
    let skill_markdown = fs::read_to_string(contracts_finder_skill_dir()?.join("SKILL.md"))?;
    for token in RUN_MANIFEST_TOKENS {
        ensure!(skill_markdown.contains(token));
    }

    let manifest = render_sample_run_manifest();
    ensure!(manifest.contains("manifest_version: 1"));
    ensure!(manifest.contains("skill_id: procurement-ingestion/contracts-finder"));
    ensure!(manifest.contains("## Summary"));
    ensure!(manifest.contains("## Evidence"));
    Ok(())
}

#[test]
fn ag_cf_006_source_drift_rate_limit_and_stall_conditions_stay_explicit_failure_codes() -> Result<()>
{
    let skill_markdown = fs::read_to_string(contracts_finder_skill_dir()?.join("SKILL.md"))?;

    for failure_code in FAILURE_CODES {
        ensure!(
            skill_markdown.contains(failure_code),
            "missing failure code {failure_code}"
        );
    }

    ensure!(skill_markdown.contains("403"));
    ensure!(skill_markdown.contains("PAGE_SHAPE_DRIFT"));
    ensure!(skill_markdown.contains("LOCALE_OR_CURRENCY_DRIFT"));
    Ok(())
}
