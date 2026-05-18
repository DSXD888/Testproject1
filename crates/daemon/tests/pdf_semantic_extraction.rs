#![forbid(unsafe_code)]
#![allow(clippy::duplicate_mod)]

mod support;

#[path = "support/skills.rs"]
#[allow(dead_code)]
mod skills_support;

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result, ensure};
use assert_cmd::Command;
use rusqlite::{Connection, params};
use serde_json::Value;
use tempfile::TempDir;

use crate::support::db::{DocumentExtractionSeed, StateDb, seed_document_extraction};
use crate::support::pdf::sample_semantic_result;
use skills_support::{lint_skill_directory, repo_skill_dir};

struct SemanticHarness {
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

impl SemanticHarness {
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
        expected_overlays: &[&str],
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
                source_content_hash: "semantic-hash",
                document_role: Some("technical_specification"),
                semantic_extraction_state: "pending",
            },
        )?;
        conn.execute(
            "UPDATE document_extractions SET expected_overlays_json = ?1 WHERE id = ?2",
            params![serde_json::to_string(expected_overlays)?, extraction_id],
        )?;
        Ok(())
    }

    fn seed_semantic_job(
        &self,
        job_id: &str,
        process_id: &str,
        document_extraction_id: &str,
        notice_id: &str,
        skill_id: &str,
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
                serde_json::json!({
                    "request": {
                        "document_extraction_id": document_extraction_id,
                        "process_id": process_id,
                        "notice_id": notice_id,
                        "skill_id": skill_id,
                        "extraction_scope": "full_document",
                        "target_domain": match skill_id {
                            "pdf-extraction/tender-specification" => "lots",
                            "pdf-extraction/pricing-schedule" => "pricing",
                            "pdf-extraction/bill-of-quantities" => "quantities",
                            _ => "generic",
                        },
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
            serde_json::to_vec_pretty(&serde_json::json!({
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
            .context("running daemon --run-next-job for semantic extraction")?;
        Ok(RunResult {
            stderr: String::from_utf8(output.stderr).context("decoding stderr")?,
            success: output.status.success(),
        })
    }
}

fn tender_spec_schema() -> Result<Value> {
    let path = repo_skill_dir("pdf-extraction/tender-specification")?.join("result.schema.json");
    let raw = fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    serde_json::from_str(&raw).with_context(|| format!("parsing {}", path.display()))
}

fn boq_schema() -> Result<Value> {
    let path = repo_skill_dir("pdf-extraction/bill-of-quantities")?.join("result.schema.json");
    let raw = fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    serde_json::from_str(&raw).with_context(|| format!("parsing {}", path.display()))
}

fn latest_semantic_row(conn: &Connection) -> Result<(String, String, String, String)> {
    conn.query_row(
        "\
        SELECT id, skill_id, target_domain, extraction_scope
        FROM document_semantic_extractions
        ORDER BY rowid DESC
        LIMIT 1
        ",
        [],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
    )
    .context("loading latest document_semantic_extractions row")
}

fn semantic_state(conn: &Connection, document_extraction_id: &str) -> Result<String> {
    conn.query_row(
        "SELECT semantic_extraction_state FROM document_extractions WHERE id = ?1",
        params![document_extraction_id],
        |row| row.get(0),
    )
    .context("reading semantic_extraction_state")
}

fn workflow_job_status(conn: &Connection, job_id: &str) -> Result<String> {
    conn.query_row(
        "SELECT status FROM workflow_jobs WHERE id = ?1",
        params![job_id],
        |row| row.get(0),
    )
    .context("reading workflow_jobs.status")
}

#[test]
fn test_tender_spec_contract_maps_to_single_lots_target_domain() -> Result<()> {
    let schema = tender_spec_schema()?;
    let extraction_metadata = &schema["properties"]["extraction_metadata"];
    let target_domain = &extraction_metadata["properties"]["target_domain"];

    ensure!(
        target_domain["const"] == Value::String("lots".to_owned()),
        "tender-spec semantic rows must persist with target_domain=lots"
    );

    let required = extraction_metadata["required"]
        .as_array()
        .context("extraction_metadata.required missing")?;
    ensure!(
        required.contains(&Value::String("extraction_scope".to_owned())),
        "tender-spec extraction metadata must expose extraction_scope"
    );
    ensure!(
        required.contains(&Value::String("policy_version".to_owned())),
        "tender-spec extraction metadata must expose policy_version"
    );
    Ok(())
}

#[test]
fn test_tender_spec_contract_carries_semantic_persistence_fields() -> Result<()> {
    let schema = tender_spec_schema()?;
    let required = schema["required"]
        .as_array()
        .context("schema required array missing")?;

    for field in [
        "field_confidences",
        "extraction_confidence_bps",
        "warnings",
        "evidence_anchors",
        "extraction_metadata",
    ] {
        ensure!(
            required.contains(&Value::String(field.to_owned())),
            "tender-spec schema missing required persistence field {field}"
        );
    }
    Ok(())
}

#[test]
fn test_boq_skill_package_lints() -> Result<()> {
    let linted = lint_skill_directory(&repo_skill_dir("pdf-extraction/bill-of-quantities")?)?;
    ensure!(
        linted.metadata.skill_id == "pdf-extraction/bill-of-quantities",
        "unexpected BoQ skill id"
    );
    ensure!(
        linted.metadata.interaction_profile == "headless_contract",
        "BoQ overlay must stay in contract mode"
    );
    ensure!(
        linted.metadata.model_profile == "structured_extract_low_cost",
        "BoQ overlay must stay on the low-cost structured model tier"
    );
    Ok(())
}

#[test]
fn test_boq_contract_maps_to_quantities_target_domain() -> Result<()> {
    let schema = boq_schema()?;
    let extraction_metadata = &schema["properties"]["extraction_metadata"];
    let target_domain = &extraction_metadata["properties"]["target_domain"];

    ensure!(
        target_domain["const"] == Value::String("quantities".to_owned()),
        "BoQ semantic rows must persist with target_domain=quantities"
    );

    let required = extraction_metadata["required"]
        .as_array()
        .context("extraction_metadata.required missing")?;
    ensure!(
        required.contains(&Value::String("extraction_scope".to_owned())),
        "BoQ extraction metadata must expose extraction_scope"
    );
    Ok(())
}

#[test]
fn test_boq_contract_requires_merge_safe_identity_fields() -> Result<()> {
    let schema = boq_schema()?;
    let item_required = schema["properties"]["items"]["items"]["required"]
        .as_array()
        .context("items required array missing")?;
    let summary_required = schema["properties"]["summary"]["required"]
        .as_array()
        .context("summary required array missing")?;

    for field in [
        "item_key",
        "item_ref",
        "section_ref",
        "continuation_of_item_key",
        "evidence_anchor",
    ] {
        ensure!(
            item_required.contains(&Value::String(field.to_owned())),
            "BoQ schema missing merge-safe item field {field}"
        );
    }

    ensure!(
        summary_required.contains(&Value::String("item_count".to_owned())),
        "BoQ summary must expose item_count"
    );
    ensure!(
        summary_required.contains(&Value::String("total_value_pence".to_owned())),
        "BoQ summary must expose total_value_pence"
    );
    Ok(())
}

#[test]
fn test_semantic_result_persistence() -> Result<()> {
    let harness = SemanticHarness::new()?;
    harness.seed_process("proc-semantic")?;
    harness.seed_notice("proc-semantic", "notice-semantic", "raw-semantic")?;
    harness.seed_document_extraction(
        "extract-semantic",
        "raw-semantic",
        "proc-semantic",
        "notice-semantic",
        &["pdf-extraction/tender-specification"],
    )?;
    harness.seed_semantic_job(
        "job-semantic-1",
        "proc-semantic",
        "extract-semantic",
        "notice-semantic",
        "pdf-extraction/tender-specification",
    )?;

    let run = harness.run_next_job(&sample_semantic_result(
        "pdf-extraction/tender-specification",
    ))?;
    ensure!(run.success, "stderr: {}", run.stderr);

    let conn = harness.connection()?;
    let (_, skill_id, target_domain, extraction_scope) = latest_semantic_row(&conn)?;
    ensure!(skill_id == "pdf-extraction/tender-specification");
    ensure!(target_domain == "lots");
    ensure!(extraction_scope == "full_document");
    Ok(())
}

#[test]
fn test_semantic_extraction_happy_path() -> Result<()> {
    test_semantic_result_persistence()
}

#[test]
fn test_semantic_aggregate_state_update() -> Result<()> {
    let harness = SemanticHarness::new()?;
    harness.seed_process("proc-aggregate")?;
    harness.seed_notice("proc-aggregate", "notice-aggregate", "raw-aggregate")?;
    harness.seed_document_extraction(
        "extract-aggregate",
        "raw-aggregate",
        "proc-aggregate",
        "notice-aggregate",
        &[
            "pdf-extraction/tender-specification",
            "pdf-extraction/pricing-schedule",
        ],
    )?;

    harness.seed_semantic_job(
        "job-aggregate-1",
        "proc-aggregate",
        "extract-aggregate",
        "notice-aggregate",
        "pdf-extraction/tender-specification",
    )?;
    let first = harness.run_next_job(&sample_semantic_result(
        "pdf-extraction/tender-specification",
    ))?;
    ensure!(first.success, "stderr: {}", first.stderr);
    let conn = harness.connection()?;
    ensure!(semantic_state(&conn, "extract-aggregate")? == "partial");

    harness.seed_semantic_job(
        "job-aggregate-2",
        "proc-aggregate",
        "extract-aggregate",
        "notice-aggregate",
        "pdf-extraction/pricing-schedule",
    )?;
    let second =
        harness.run_next_job(&sample_semantic_result("pdf-extraction/pricing-schedule"))?;
    ensure!(second.success, "stderr: {}", second.stderr);
    ensure!(semantic_state(&conn, "extract-aggregate")? == "complete");
    Ok(())
}

#[test]
fn test_semantic_supersession() -> Result<()> {
    let harness = SemanticHarness::new()?;
    harness.seed_process("proc-super")?;
    harness.seed_notice("proc-super", "notice-super", "raw-super")?;
    harness.seed_document_extraction(
        "extract-super",
        "raw-super",
        "proc-super",
        "notice-super",
        &["pdf-extraction/pricing-schedule"],
    )?;

    harness.seed_semantic_job(
        "job-super-1",
        "proc-super",
        "extract-super",
        "notice-super",
        "pdf-extraction/pricing-schedule",
    )?;
    let first = harness.run_next_job(&sample_semantic_result("pdf-extraction/pricing-schedule"))?;
    ensure!(first.success, "stderr: {}", first.stderr);
    let conn = harness.connection()?;
    let (first_id, _, _, _) = latest_semantic_row(&conn)?;

    harness.seed_semantic_job(
        "job-super-2",
        "proc-super",
        "extract-super",
        "notice-super",
        "pdf-extraction/pricing-schedule",
    )?;
    let second =
        harness.run_next_job(&sample_semantic_result("pdf-extraction/pricing-schedule"))?;
    ensure!(second.success, "stderr: {}", second.stderr);

    let (is_current, superseded_by): (i64, Option<String>) = conn.query_row(
        "SELECT is_current, superseded_by_semantic_extraction_id FROM document_semantic_extractions WHERE id = ?1",
        params![first_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    ensure!(is_current == 0);
    ensure!(superseded_by.is_some());
    Ok(())
}

#[test]
fn test_semantic_written_entity_refs_track_domain_writes() -> Result<()> {
    let harness = SemanticHarness::new()?;
    harness.seed_process("proc-refs")?;
    harness.seed_notice("proc-refs", "notice-refs", "raw-refs")?;
    harness.seed_document_extraction(
        "extract-refs",
        "raw-refs",
        "proc-refs",
        "notice-refs",
        &["pdf-extraction/tender-specification"],
    )?;
    harness.seed_semantic_job(
        "job-refs",
        "proc-refs",
        "extract-refs",
        "notice-refs",
        "pdf-extraction/tender-specification",
    )?;

    let run = harness.run_next_job(&sample_semantic_result(
        "pdf-extraction/tender-specification",
    ))?;
    ensure!(run.success, "stderr: {}", run.stderr);

    let conn = harness.connection()?;
    let written_refs_json: String = conn.query_row(
        "SELECT written_entity_refs_json FROM document_semantic_extractions WHERE document_extraction_id = ?1",
        params!["extract-refs"],
        |row| row.get(0),
    )?;
    let written_refs: Vec<Value> =
        serde_json::from_str(&written_refs_json).context("parsing written_entity_refs_json")?;
    ensure!(
        written_refs
            .iter()
            .any(|entry| entry["table"] == Value::String("notice_lots".to_owned()))
    );
    ensure!(
        written_refs
            .iter()
            .any(|entry| entry["table"] == Value::String("product_candidates".to_owned()))
    );

    let lot_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM notice_lots WHERE notice_id = ?1",
        params!["notice-refs"],
        |row| row.get(0),
    )?;
    let candidate_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM product_candidates WHERE process_id = ?1",
        params!["proc-refs"],
        |row| row.get(0),
    )?;
    ensure!(lot_count >= 1);
    ensure!(candidate_count >= 1);
    Ok(())
}

#[test]
fn test_contract_validation_runs_before_persistence() -> Result<()> {
    let harness = SemanticHarness::new()?;
    harness.seed_process("proc-invalid")?;
    harness.seed_notice("proc-invalid", "notice-invalid", "raw-invalid")?;
    harness.seed_document_extraction(
        "extract-invalid",
        "raw-invalid",
        "proc-invalid",
        "notice-invalid",
        &["pdf-extraction/tender-specification"],
    )?;
    harness.seed_semantic_job(
        "job-invalid",
        "proc-invalid",
        "extract-invalid",
        "notice-invalid",
        "pdf-extraction/tender-specification",
    )?;

    let invalid_payload = serde_json::json!({
        "lots": [],
        "warnings": [],
        "evidence_anchors": [],
        "extraction_metadata": {
            "extraction_scope": "full_document"
        }
    });
    let run = harness.run_next_job(&invalid_payload)?;
    ensure!(!run.success, "invalid semantic payload should fail");

    let conn = harness.connection()?;
    ensure!(workflow_job_status(&conn, "job-invalid")? == "failed");
    let semantic_rows: i64 = conn.query_row(
        "SELECT COUNT(*) FROM document_semantic_extractions WHERE document_extraction_id = ?1",
        params!["extract-invalid"],
        |row| row.get(0),
    )?;
    let lot_rows: i64 = conn.query_row(
        "SELECT COUNT(*) FROM notice_lots WHERE notice_id = ?1",
        params!["notice-invalid"],
        |row| row.get(0),
    )?;
    let candidate_rows: i64 = conn.query_row(
        "SELECT COUNT(*) FROM product_candidates WHERE process_id = ?1",
        params!["proc-invalid"],
        |row| row.get(0),
    )?;
    ensure!(semantic_rows == 0);
    ensure!(lot_rows == 0);
    ensure!(candidate_rows == 0);
    Ok(())
}

#[test]
fn test_written_entity_refs_track_notice_lot_and_product_candidate_writes() -> Result<()> {
    test_semantic_written_entity_refs_track_domain_writes()
}

#[test]
fn test_tender_spec_payload_persists_as_target_domain_lots_but_still_writes_candidate_seeds()
-> Result<()> {
    let harness = SemanticHarness::new()?;
    harness.seed_process("proc-target-domain")?;
    harness.seed_notice(
        "proc-target-domain",
        "notice-target-domain",
        "raw-target-domain",
    )?;
    harness.seed_document_extraction(
        "extract-target-domain",
        "raw-target-domain",
        "proc-target-domain",
        "notice-target-domain",
        &["pdf-extraction/tender-specification"],
    )?;
    harness.seed_semantic_job(
        "job-target-domain",
        "proc-target-domain",
        "extract-target-domain",
        "notice-target-domain",
        "pdf-extraction/tender-specification",
    )?;

    let run = harness.run_next_job(&sample_semantic_result(
        "pdf-extraction/tender-specification",
    ))?;
    ensure!(run.success, "stderr: {}", run.stderr);

    let conn = harness.connection()?;
    let (_, skill_id, target_domain, _) = latest_semantic_row(&conn)?;
    ensure!(skill_id == "pdf-extraction/tender-specification");
    ensure!(target_domain == "lots");

    let lot_rows: i64 = conn.query_row(
        "SELECT COUNT(*) FROM notice_lots WHERE notice_id = ?1",
        params!["notice-target-domain"],
        |row| row.get(0),
    )?;
    let candidate_rows: i64 = conn.query_row(
        "SELECT COUNT(*) FROM product_candidates WHERE process_id = ?1",
        params!["proc-target-domain"],
        |row| row.get(0),
    )?;
    ensure!(lot_rows >= 1);
    ensure!(candidate_rows >= 1);
    Ok(())
}
