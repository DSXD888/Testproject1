#![forbid(unsafe_code)]
#![allow(clippy::duplicate_mod)]

mod support;

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result, anyhow, ensure};
use assert_cmd::Command;
use rusqlite::{Connection, params};
use serde_json::{Value, json};
use tempfile::TempDir;

use crate::support::db::StateDb;
use crate::support::pdf::{sample_docling_output, sample_pdf_bytes};

struct CascadeHarness {
    _temp: TempDir,
    state_root: PathBuf,
    runtime_root: PathBuf,
    artifact_root: PathBuf,
    db_path: PathBuf,
    pdf_path: PathBuf,
    wrapper_path: PathBuf,
    pythonpath_root: PathBuf,
    model_artifacts_path: PathBuf,
}

struct RunResult {
    stdout: String,
    stderr: String,
    success: bool,
}

struct ExtractJobSpec<'a> {
    job_id: &'a str,
    process_id: &'a str,
    raw_document_id: &'a str,
    notice_id: Option<&'a str>,
    document_set_id: Option<&'a str>,
    document_role: Option<&'a str>,
    expected_overlays: &'a [&'a str],
}

impl CascadeHarness {
    fn new() -> Result<Self> {
        let temp = tempfile::tempdir()?;
        let root = temp.path().to_path_buf();
        let state_root = root.join("state");
        let runtime_root = root.join("runtime");
        let artifact_root = root.join("artifacts");
        let input_root = root.join("inputs");
        let pythonpath_root = root.join("python");
        let model_artifacts_path = root.join("docling-models");
        fs::create_dir_all(&state_root)?;
        fs::create_dir_all(&runtime_root)?;
        fs::create_dir_all(&artifact_root)?;
        fs::create_dir_all(&input_root)?;
        fs::create_dir_all(&pythonpath_root)?;
        fs::create_dir_all(&model_artifacts_path)?;

        let pdf_path = input_root.join("sample.pdf");
        fs::write(&pdf_path, sample_pdf_bytes())
            .with_context(|| format!("writing {}", pdf_path.display()))?;

        let docling_module = pythonpath_root.join("docling.py");
        fs::write(&docling_module, "__version__ = '2.85.0'\n")
            .with_context(|| format!("writing {}", docling_module.display()))?;

        let wrapper_path = root.join("docling-wrapper.py");
        let mut payload = sample_docling_output();
        payload
            .get_mut("metadata")
            .and_then(Value::as_object_mut)
            .ok_or_else(|| anyhow!("sample Docling output missing metadata object"))?
            .insert(
                "docling_version".to_owned(),
                Value::String("2.85.0".to_owned()),
            );
        let payload = serde_json::to_string(&payload)?;
        fs::write(
            &wrapper_path,
            format!(
                r#"#!/usr/bin/env python3
import argparse
import json

PAYLOAD = json.loads(r'''{payload}''')

parser = argparse.ArgumentParser()
parser.add_argument("--input")
parser.add_argument("--output")
parser.add_argument("--strategy")
parser.add_argument("--timeout")
parser.add_argument("--version", action="store_true")
args = parser.parse_args()

if args.version:
    print("2.85.0")
    raise SystemExit(0)

with open(args.output, "w", encoding="utf-8") as handle:
    json.dump(PAYLOAD, handle, indent=2)
"#
            ),
        )
        .with_context(|| format!("writing {}", wrapper_path.display()))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&wrapper_path)?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&wrapper_path, perms)?;
        }

        let db_path = state_root.join("state.db");
        let _ = StateDb::open(&db_path)?;

        Ok(Self {
            _temp: temp,
            state_root,
            runtime_root,
            artifact_root,
            db_path,
            pdf_path,
            wrapper_path,
            pythonpath_root,
            model_artifacts_path,
        })
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

    fn seed_document_set(
        &self,
        process_id: &str,
        notice_id: &str,
        document_set_id: &str,
    ) -> Result<()> {
        let conn = self.connection()?;
        conn.execute(
            "\
            INSERT INTO tender_document_sets (
              id, process_id, notice_id, document_set_hash, completeness_state, document_roles_json,
              documents_json, acquired_at, owner_id
            ) VALUES (?1, ?2, ?3, ?4, 'complete', ?5, ?6, '2026-04-12T00:00:00Z', 'local-owner')
            ",
            params![
                document_set_id,
                process_id,
                notice_id,
                format!("hash-{document_set_id}"),
                json!([]).to_string(),
                json!([]).to_string(),
            ],
        )?;
        Ok(())
    }

    fn seed_raw_document(&self, raw_document_id: &str) -> Result<()> {
        let conn = self.connection()?;
        conn.execute(
            "\
            INSERT INTO raw_documents (
              id, source_id, content_type, storage_path, metadata
            ) VALUES (?1, 'fts', 'application/pdf', ?2, ?3)
            ",
            params![
                raw_document_id,
                self.pdf_path.to_string_lossy(),
                json!({"source_url": "https://example.test/sample.pdf"}).to_string(),
            ],
        )?;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn seed_extract_job(&self, spec: &ExtractJobSpec<'_>) -> Result<()> {
        let conn = self.connection()?;
        conn.execute(
            "\
            INSERT INTO workflow_jobs (
              id, kind, process_id, notice_id, scope_kind, scope_id, status, scheduled_for, input_payload
            ) VALUES (?1, 'extract_document_structure', ?2, ?3, 'notice', ?4, 'pending', ?5, ?6)
            ",
            params![
                spec.job_id,
                spec.process_id,
                spec.notice_id,
                spec.notice_id.unwrap_or(spec.raw_document_id),
                "1970-01-01T00:00:00Z",
                json!({
                    "request": {
                        "raw_document_id": spec.raw_document_id,
                        "process_id": spec.process_id,
                        "notice_id": spec.notice_id,
                        "document_set_id": spec.document_set_id,
                        "docling_wrapper_path": self.wrapper_path,
                        "model_artifacts_path": self.model_artifacts_path,
                        "document_timeout_secs": 30,
                        "wall_clock_timeout_secs": 30,
                        "processing_strategy": "standard",
                        "document_role": spec.document_role,
                        "extraction_policy_version": "2026.04.12",
                        "expected_overlays": spec.expected_overlays,
                    }
                })
                .to_string(),
            ],
        )?;
        Ok(())
    }

    fn run_next_job(&self) -> Result<RunResult> {
        let output = Command::cargo_bin("daemon")?
            .arg("--run-next-job")
            .arg("--state-root")
            .arg(&self.state_root)
            .arg("--runtime-dir")
            .arg(&self.runtime_root)
            .arg("--artifact-root")
            .arg(&self.artifact_root)
            .env("PYTHONPATH", &self.pythonpath_root)
            .output()
            .context("running daemon --run-next-job")?;

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
}

fn job_status(conn: &Connection, job_id: &str) -> Result<String> {
    conn.query_row(
        "SELECT status FROM workflow_jobs WHERE id = ?1",
        params![job_id],
        |row| row.get(0),
    )
    .context("reading workflow_jobs.status")
}

fn job_output(conn: &Connection, job_id: &str) -> Result<Value> {
    let payload: String = conn.query_row(
        "SELECT output_payload FROM workflow_jobs WHERE id = ?1",
        params![job_id],
        |row| row.get(0),
    )?;
    serde_json::from_str(&payload).context("parsing workflow_jobs.output_payload")
}

fn child_job_count(conn: &Connection, parent_job_id: &str) -> Result<i64> {
    conn.query_row(
        "SELECT COUNT(*) FROM workflow_jobs WHERE parent_job_id = ?1 AND kind = 'extract_document_semantics'",
        params![parent_job_id],
        |row| row.get(0),
    )
    .context("counting semantic child jobs")
}

fn latest_document_extraction(
    conn: &Connection,
) -> Result<(String, Option<String>, Option<String>, String)> {
    conn.query_row(
        "\
        SELECT id, notice_id, document_set_id, semantic_extraction_state
        FROM document_extractions
        ORDER BY rowid DESC
        LIMIT 1
        ",
        [],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
    )
    .context("loading latest document_extractions row")
}

#[test]
fn test_cascade_happy_path() -> Result<()> {
    let harness = CascadeHarness::new()?;
    harness.seed_process("proc-1")?;
    harness.seed_raw_document("raw-pdf-1")?;
    harness.seed_notice("proc-1", "notice-1", "raw-pdf-1")?;
    harness.seed_document_set("proc-1", "notice-1", "docset-1")?;
    harness.seed_extract_job(&ExtractJobSpec {
        job_id: "job-struct-1",
        process_id: "proc-1",
        raw_document_id: "raw-pdf-1",
        notice_id: Some("notice-1"),
        document_set_id: Some("docset-1"),
        document_role: Some("technical_specification"),
        expected_overlays: &["pdf-extraction/tender-specification"],
    })?;

    let run = harness.run_next_job()?;
    ensure!(run.success, "stderr: {}", run.stderr);
    ensure!(
        run.stdout.contains("Structural extraction completed"),
        "stdout: {}",
        run.stdout
    );

    let conn = harness.connection()?;
    ensure!(job_status(&conn, "job-struct-1")? == "completed");
    ensure!(child_job_count(&conn, "job-struct-1")? == 1);
    let (_, notice_id, document_set_id, semantic_state) = latest_document_extraction(&conn)?;
    ensure!(notice_id.as_deref() == Some("notice-1"));
    ensure!(document_set_id.as_deref() == Some("docset-1"));
    ensure!(semantic_state == "pending");

    let output = job_output(&conn, "job-struct-1")?;
    ensure!(output["result"]["cache_hit"] == Value::Bool(false));
    ensure!(
        output["result"]["document_extraction_id"]
            .as_str()
            .is_some_and(|value| !value.is_empty())
    );
    Ok(())
}

#[test]
fn test_cascade_no_overlays() -> Result<()> {
    let harness = CascadeHarness::new()?;
    harness.seed_process("proc-2")?;
    harness.seed_raw_document("raw-pdf-2")?;
    harness.seed_notice("proc-2", "notice-2", "raw-pdf-2")?;
    harness.seed_document_set("proc-2", "notice-2", "docset-2")?;
    harness.seed_extract_job(&ExtractJobSpec {
        job_id: "job-struct-2",
        process_id: "proc-2",
        raw_document_id: "raw-pdf-2",
        notice_id: Some("notice-2"),
        document_set_id: Some("docset-2"),
        document_role: Some("administrative"),
        expected_overlays: &[],
    })?;

    let run = harness.run_next_job()?;
    ensure!(run.success, "stderr: {}", run.stderr);

    let conn = harness.connection()?;
    ensure!(job_status(&conn, "job-struct-2")? == "completed");
    ensure!(child_job_count(&conn, "job-struct-2")? == 0);
    let (_, _, _, semantic_state) = latest_document_extraction(&conn)?;
    ensure!(semantic_state == "skipped");
    Ok(())
}

#[test]
fn test_cascade_standalone_entry() -> Result<()> {
    let harness = CascadeHarness::new()?;
    harness.seed_process("proc-standalone")?;
    harness.seed_raw_document("raw-pdf-standalone")?;
    harness.seed_extract_job(&ExtractJobSpec {
        job_id: "job-struct-standalone",
        process_id: "proc-standalone",
        raw_document_id: "raw-pdf-standalone",
        notice_id: None,
        document_set_id: None,
        document_role: Some("administrative"),
        expected_overlays: &[],
    })?;

    let run = harness.run_next_job()?;
    ensure!(run.success, "stderr: {}", run.stderr);

    let conn = harness.connection()?;
    let (_, notice_id, document_set_id, _) = latest_document_extraction(&conn)?;
    ensure!(notice_id.is_none());
    ensure!(document_set_id.is_none());
    ensure!(job_status(&conn, "job-struct-standalone")? == "completed");
    Ok(())
}

#[test]
fn test_cascade_parent_child_linkage() -> Result<()> {
    let harness = CascadeHarness::new()?;
    harness.seed_process("proc-3")?;
    harness.seed_raw_document("raw-pdf-3")?;
    harness.seed_notice("proc-3", "notice-3", "raw-pdf-3")?;
    harness.seed_document_set("proc-3", "notice-3", "docset-3")?;
    harness.seed_extract_job(&ExtractJobSpec {
        job_id: "job-struct-3",
        process_id: "proc-3",
        raw_document_id: "raw-pdf-3",
        notice_id: Some("notice-3"),
        document_set_id: Some("docset-3"),
        document_role: Some("technical_specification"),
        expected_overlays: &[
            "pdf-extraction/tender-specification",
            "pdf-extraction/pricing-schedule",
        ],
    })?;

    let run = harness.run_next_job()?;
    ensure!(run.success, "stderr: {}", run.stderr);

    let conn = harness.connection()?;
    ensure!(child_job_count(&conn, "job-struct-3")? == 2);
    let output = job_output(&conn, "job-struct-3")?;
    let child_ids = output["result"]["semantic_child_job_ids"]
        .as_array()
        .ok_or_else(|| anyhow!("missing semantic_child_job_ids array"))?;
    ensure!(child_ids.len() == 2);

    let document_extraction_id = output["result"]["document_extraction_id"]
        .as_str()
        .ok_or_else(|| anyhow!("missing document_extraction_id"))?;
    let linked_count: i64 = conn.query_row(
        "\
        SELECT COUNT(*)
        FROM workflow_jobs
        WHERE parent_job_id = ?1
          AND kind = 'extract_document_semantics'
          AND json_extract(input_payload, '$.request.request.document_extraction_id') = ?2
        ",
        params!["job-struct-3", document_extraction_id],
        |row| row.get(0),
    )?;
    ensure!(linked_count == 2);
    Ok(())
}
