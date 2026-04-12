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

type CandidateProjectionRow = (String, Option<String>, Option<String>, Option<String>);

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

    fn seed_notice_payload_raw_document(&self, notice_id: &str) -> Result<()> {
        let conn = self.connection()?;
        conn.execute(
            "\
            INSERT INTO raw_documents (
              id, source_id, content_type, storage_path, payload_json, metadata
            ) VALUES (?1, 'fts', 'application/json', NULL, ?2, ?3)
            ",
            params![
                format!("raw-{notice_id}"),
                json!({
                    "tender": {
                        "classification": {
                            "id": "39150000"
                        }
                    }
                })
                .to_string(),
                json!({"fixture": "notice-only"}).to_string(),
            ],
        )?;
        Ok(())
    }

    fn seed_document_set_state(
        &self,
        process_id: &str,
        notice_id: &str,
        document_set_id: &str,
        completeness_state: &str,
    ) -> Result<()> {
        let conn = self.connection()?;
        conn.execute(
            "\
            INSERT INTO tender_document_sets (
              id, process_id, notice_id, document_set_hash, completeness_state, document_roles_json,
              documents_json, acquired_at, owner_id, extraction_state
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, '2026-04-12T00:00:00Z', 'local-owner', 'none')
            ",
            params![
                document_set_id,
                process_id,
                notice_id,
                format!("hash-{document_set_id}"),
                completeness_state,
                json!([]).to_string(),
                json!([]).to_string(),
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
        notice_id: &str,
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
                    {"table": "notice_lots", "id": stable_lot_id(notice_id, "1")},
                    {
                        "table": "product_candidates",
                        "id": stable_candidate_id(&stable_lot_id(notice_id, "1"), "cabinet-std-01")
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
        self.seed_semantic_job_for_skill(
            job_id,
            process_id,
            document_extraction_id,
            notice_id,
            "pdf-extraction/tender-specification",
        )
    }

    fn seed_semantic_job_for_skill(
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
                json!({
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

    fn run_pipeline(&self, source_id: &str) -> Result<RunResult> {
        let output = Command::cargo_bin("daemon")?
            .arg("--pipeline")
            .arg(source_id)
            .arg("--state-root")
            .arg(&self.state_root)
            .arg("--runtime-dir")
            .arg(&self.runtime_root)
            .arg("--artifact-root")
            .arg(&self.artifact_root)
            .output()
            .context("running daemon --pipeline for notice-only fallback")?;
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

fn qualification_basis(conn: &Connection, qualification_id: &str) -> Result<String> {
    conn.query_row(
        "SELECT qualification_basis FROM qualification_runs WHERE id = ?1",
        params![qualification_id],
        |row| row.get(0),
    )
    .context("reading qualification_runs.qualification_basis")
}

fn document_set_extraction_state(conn: &Connection, document_set_id: &str) -> Result<String> {
    conn.query_row(
        "SELECT extraction_state FROM tender_document_sets WHERE id = ?1",
        params![document_set_id],
        |row| row.get(0),
    )
    .context("reading tender_document_sets.extraction_state")
}

fn lot_row(conn: &Connection, lot_id: &str) -> Result<(String, Option<String>, Option<String>)> {
    conn.query_row(
        "SELECT title, description, documents FROM notice_lots WHERE id = ?1",
        params![lot_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    )
    .context("reading notice_lots row")
}

fn seed_notice_only_lot(
    conn: &Connection,
    lot_id: &str,
    notice_id: &str,
    title: &str,
    estimated_value_pence: i64,
) -> Result<()> {
    conn.execute(
        "\
        INSERT INTO notice_lots (
          id, notice_id, source_lot_id, lot_key, lot_number, title, estimated_value_pence,
          owner_id, row_version
        ) VALUES (?1, ?2, 'implicit', 'implicit-0', 1, ?3, ?4, 'local-owner', 1)
        ",
        params![lot_id, notice_id, title, estimated_value_pence],
    )?;
    Ok(())
}

fn candidate_projection(conn: &Connection, candidate_id: &str) -> Result<CandidateProjectionRow> {
    conn.query_row(
        "SELECT name, specification_text, numeric_specs_json, source_evidence_json FROM product_candidates WHERE id = ?1",
        params![candidate_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
    )
    .context("reading product_candidates row")
}

fn stable_lot_id(notice_id: &str, lot_number: &str) -> String {
    format!("{notice_id}-lot-{lot_number}")
}

fn stable_candidate_id(lot_id: &str, candidate_key: &str) -> String {
    format!("{lot_id}-candidate-{candidate_key}")
}

#[test]
fn test_lot_extraction_uses_document_evidence() -> Result<()> {
    let harness = LotRefreshHarness::new()?;
    harness.seed_process("proc-lot-doc")?;
    harness.seed_notice("proc-lot-doc", "notice-lot-doc", "raw-lot-doc")?;
    harness.seed_document_extraction(
        "extract-lot-doc",
        "raw-lot-doc",
        "proc-lot-doc",
        "notice-lot-doc",
    )?;
    let conn = harness.connection()?;
    seed_notice_only_lot(
        &conn,
        "notice-lot-doc-lot-implicit",
        "notice-lot-doc",
        "Notice-only title",
        250_000,
    )?;
    harness.seed_semantic_job(
        "job-lot-doc",
        "proc-lot-doc",
        "extract-lot-doc",
        "notice-lot-doc",
    )?;

    let run = harness.run_next_job(&sample_semantic_result(
        "pdf-extraction/tender-specification",
    ))?;
    ensure!(run.success, "stderr: {}", run.stderr);

    let (title, description, documents) = lot_row(&conn, "notice-lot-doc-lot-implicit")?;
    ensure!(title == "Storage cabinets");
    ensure!(
        description.as_deref() == Some("Supply and delivery of stainless steel storage cabinets.")
    );
    ensure!(
        documents
            .as_deref()
            .is_some_and(|value| value.contains("document_derived"))
    );
    Ok(())
}

#[test]
fn test_product_extraction_uses_tender_spec_requirement_seeds() -> Result<()> {
    let harness = LotRefreshHarness::new()?;
    harness.seed_process("proc-seeds")?;
    harness.seed_notice("proc-seeds", "notice-seeds", "raw-seeds")?;
    harness.seed_document_extraction("extract-seeds", "raw-seeds", "proc-seeds", "notice-seeds")?;
    harness.seed_semantic_job("job-seeds", "proc-seeds", "extract-seeds", "notice-seeds")?;

    let run = harness.run_next_job(&sample_semantic_result(
        "pdf-extraction/tender-specification",
    ))?;
    ensure!(run.success, "stderr: {}", run.stderr);

    let lot_id = stable_lot_id("notice-seeds", "1");
    let candidate_id = stable_candidate_id(&lot_id, "cabinet-std-01");
    let (name, specification_text, _numeric_specs_json, source_evidence_json) =
        candidate_projection(&harness.connection()?, &candidate_id)?;
    ensure!(name == "Stainless steel storage cabinet");
    ensure!(
        specification_text
            .as_deref()
            .is_some_and(|value| value.contains("BS EN 16121"))
    );
    ensure!(
        source_evidence_json
            .as_deref()
            .is_some_and(|value| value.contains("pdf-extraction/tender-specification"))
    );
    Ok(())
}

#[test]
fn test_product_extraction_uses_pricing_and_quantity_overlays_when_present() -> Result<()> {
    let harness = LotRefreshHarness::new()?;
    harness.seed_process("proc-overlay-enrich")?;
    harness.seed_notice(
        "proc-overlay-enrich",
        "notice-overlay-enrich",
        "raw-overlay-enrich",
    )?;
    harness.seed_document_extraction(
        "extract-overlay-enrich",
        "raw-overlay-enrich",
        "proc-overlay-enrich",
        "notice-overlay-enrich",
    )?;
    harness.seed_semantic_job(
        "job-overlay-tender",
        "proc-overlay-enrich",
        "extract-overlay-enrich",
        "notice-overlay-enrich",
    )?;
    let tender = harness.run_next_job(&sample_semantic_result(
        "pdf-extraction/tender-specification",
    ))?;
    ensure!(tender.success, "stderr: {}", tender.stderr);

    harness.seed_semantic_job_for_skill(
        "job-overlay-pricing",
        "proc-overlay-enrich",
        "extract-overlay-enrich",
        "notice-overlay-enrich",
        "pdf-extraction/pricing-schedule",
    )?;
    let pricing =
        harness.run_next_job(&sample_semantic_result("pdf-extraction/pricing-schedule"))?;
    ensure!(pricing.success, "stderr: {}", pricing.stderr);

    harness.seed_semantic_job_for_skill(
        "job-overlay-boq",
        "proc-overlay-enrich",
        "extract-overlay-enrich",
        "notice-overlay-enrich",
        "pdf-extraction/bill-of-quantities",
    )?;
    let boq = harness.run_next_job(&sample_semantic_result("pdf-extraction/bill-of-quantities"))?;
    ensure!(boq.success, "stderr: {}", boq.stderr);

    let lot_id = stable_lot_id("notice-overlay-enrich", "1");
    let candidate_id = stable_candidate_id(&lot_id, "cabinet-std-01");
    let (_name, _specification_text, numeric_specs_json, source_evidence_json) =
        candidate_projection(&harness.connection()?, &candidate_id)?;
    ensure!(
        numeric_specs_json
            .as_deref()
            .is_some_and(|value| value.contains("pricing_overlay"))
    );
    ensure!(
        numeric_specs_json
            .as_deref()
            .is_some_and(|value| value.contains("quantity_overlay"))
    );
    ensure!(
        source_evidence_json
            .as_deref()
            .is_some_and(|value| value.contains("pdf-extraction/pricing-schedule"))
    );
    ensure!(
        source_evidence_json
            .as_deref()
            .is_some_and(|value| value.contains("pdf-extraction/bill-of-quantities"))
    );
    Ok(())
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
        &stable_lot_id("notice-refresh-change", "1"),
    )?;

    let mut previous_result = sample_semantic_result("pdf-extraction/tender-specification");
    previous_result["lots"][0]["title"] = Value::String("Legacy storage cabinets".to_owned());
    harness.seed_current_semantic_result(
        "semantic-refresh-old",
        "extract-refresh-change",
        "proc-refresh-change",
        "notice-refresh-change",
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
            &stable_lot_id("notice-refresh-change", "1"),
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
        &stable_lot_id("notice-refresh-same", "1"),
    )?;

    let identical_result = sample_semantic_result("pdf-extraction/tender-specification");
    harness.seed_current_semantic_result(
        "semantic-refresh-same-old",
        "extract-refresh-same",
        "proc-refresh-same",
        "notice-refresh-same",
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
            &stable_lot_id("notice-refresh-same", "1"),
        )? == "fresh"
    );
    Ok(())
}

#[test]
fn test_fallback_to_notice_text() -> Result<()> {
    let harness = LotRefreshHarness::new()?;
    harness.seed_process("proc-notice-only")?;
    harness.seed_notice("proc-notice-only", "notice-notice-only", "raw-no-pdf-doc")?;
    harness.seed_notice_payload_raw_document("notice-notice-only")?;
    harness.seed_document_set_state(
        "proc-notice-only",
        "notice-notice-only",
        "docset-notice-only",
        "none",
    )?;
    harness.seed_stage_head(
        "proc-notice-only",
        "Normalize",
        "notice",
        "notice-notice-only",
    )?;

    let run = harness.run_pipeline("fts")?;
    ensure!(run.success, "stderr: {}", run.stderr);

    let conn = harness.connection()?;
    ensure!(qualification_basis(&conn, "qual-notice-notice-only")? == "notice_only");
    ensure!(document_set_extraction_state(&conn, "docset-notice-only")? == "none");
    ensure!(
        conn.query_row::<i64, _, _>("SELECT COUNT(*) FROM document_extractions", [], |row| row
            .get(0),)?
            == 0
    );
    Ok(())
}

#[test]
fn test_fallback_to_notice_only() -> Result<()> {
    test_fallback_to_notice_text()
}
