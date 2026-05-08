#![forbid(unsafe_code)]
#![allow(clippy::duplicate_mod)]

mod support;

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result, anyhow, ensure};
use assert_cmd::Command;
use rusqlite::{Connection, params};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tempfile::TempDir;

use crate::support::db::{DocumentExtractionSeed, StateDb, seed_document_extraction};
use crate::support::pdf::{sample_docling_output, sample_pdf_bytes, sample_semantic_result};

struct CascadeHarness {
    _temp: TempDir,
    state_root: PathBuf,
    runtime_root: PathBuf,
    artifact_root: PathBuf,
    db_path: PathBuf,
    pdf_path: PathBuf,
    wrapper_path: PathBuf,
    python_bin_path: PathBuf,
    pythonpath_root: PathBuf,
    docling_version_path: PathBuf,
    model_artifacts_path: PathBuf,
}

struct RunResult {
    stdout: String,
    stderr: String,
    success: bool,
}

struct FakeClaudeCli {
    program: PathBuf,
}

struct ExtractJobSpec<'a> {
    job_id: &'a str,
    process_id: &'a str,
    raw_document_id: &'a str,
    notice_id: Option<&'a str>,
    document_set_id: Option<&'a str>,
    document_role: Option<&'a str>,
    expected_overlays: &'a [&'a str],
    chunk_plan: Option<Value>,
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
        let python_bin_path = root.join("fake-python");
        let docling_version_path = root.join("docling-version.txt");
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
        fs::write(&docling_version_path, "2.85.0\n")
            .with_context(|| format!("writing {}", docling_version_path.display()))?;
        fs::write(
            &python_bin_path,
            format!(
                "#!/bin/sh\nset -eu\nif [ \"${{1-}}\" = \"-c\" ]; then\n  cat \"{}\"\n  exit 0\nfi\nexec python3 \"$@\"\n",
                docling_version_path.display()
            ),
        )
        .with_context(|| format!("writing {}", python_bin_path.display()))?;

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
            let mut perms = fs::metadata(&python_bin_path)?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&python_bin_path, perms)?;

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
            python_bin_path,
            pythonpath_root,
            docling_version_path,
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
        self.seed_document_set_with_documents(process_id, notice_id, document_set_id, &[])
    }

    fn seed_document_set_with_documents(
        &self,
        process_id: &str,
        notice_id: &str,
        document_set_id: &str,
        document_ids: &[&str],
    ) -> Result<()> {
        self.seed_document_set_with_state_and_documents(
            process_id,
            notice_id,
            document_set_id,
            "complete",
            document_ids,
        )
    }

    fn seed_document_set_with_state_and_documents(
        &self,
        process_id: &str,
        notice_id: &str,
        document_set_id: &str,
        completeness_state: &str,
        document_ids: &[&str],
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
                json!(
                    document_ids
                        .iter()
                        .map(|raw_document_id| json!({"raw_document_id": raw_document_id}))
                        .collect::<Vec<_>>()
                )
                .to_string(),
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

    fn overwrite_pdf_bytes(&self, bytes: &[u8]) -> Result<()> {
        fs::write(&self.pdf_path, bytes)
            .with_context(|| format!("writing {}", self.pdf_path.display()))
    }

    fn configure_docling_runtime(&self, version: &str) -> Result<()> {
        let docling_module = self.pythonpath_root.join("docling.py");
        fs::write(&docling_module, format!("__version__ = '{version}'\n"))
            .with_context(|| format!("writing {}", docling_module.display()))?;
        fs::write(&self.docling_version_path, format!("{version}\n"))
            .with_context(|| format!("writing {}", self.docling_version_path.display()))?;

        let mut payload = sample_docling_output();
        payload
            .get_mut("metadata")
            .and_then(Value::as_object_mut)
            .ok_or_else(|| anyhow!("sample Docling output missing metadata object"))?
            .insert(
                "docling_version".to_owned(),
                Value::String(version.to_owned()),
            );
        let payload = serde_json::to_string(&payload)?;
        fs::write(
            &self.wrapper_path,
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
    print("{version}")
    raise SystemExit(0)

with open(args.output, "w", encoding="utf-8") as handle:
    json.dump(PAYLOAD, handle, indent=2)
"#
            ),
        )
        .with_context(|| format!("writing {}", self.wrapper_path.display()))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&self.wrapper_path)?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&self.wrapper_path, perms)?;
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn seed_existing_document_extraction(
        &self,
        extraction_id: &str,
        raw_document_id: &str,
        process_id: &str,
        notice_id: &str,
        document_set_id: &str,
        source_content_hash: &str,
        engine_version: &str,
        semantic_extraction_state: &str,
    ) -> Result<()> {
        let conn = self.connection()?;
        seed_document_extraction(
            &conn,
            &DocumentExtractionSeed {
                id: extraction_id,
                raw_document_id,
                process_id,
                notice_id: Some(notice_id),
                document_set_id: Some(document_set_id),
                source_content_hash,
                document_role: Some("technical_specification"),
                semantic_extraction_state,
            },
        )?;
        conn.execute(
            "\
            UPDATE document_extractions
            SET extraction_engine_version = ?2,
                expected_overlays_json = ?3,
                extraction_policy_version = '2026.04.12'
            WHERE id = ?1
            ",
            params![
                extraction_id,
                engine_version,
                json!(["pdf-extraction/tender-specification"]).to_string(),
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
                        "chunk_plan": spec.chunk_plan.clone(),
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
            .env("DOCLING_PYTHON_BIN", &self.python_bin_path)
            .output()
            .context("running daemon --run-next-job")?;

        Ok(RunResult {
            stdout: String::from_utf8(output.stdout).context("decoding stdout")?,
            stderr: String::from_utf8(output.stderr).context("decoding stderr")?,
            success: output.status.success(),
        })
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

    fn run_next_job_with_claude_result(&self, result_payload: &Value) -> Result<RunResult> {
        let fake_claude = self.prepare_fake_claude(result_payload)?;
        let output = Command::cargo_bin("daemon")?
            .arg("--run-next-job")
            .arg("--state-root")
            .arg(&self.state_root)
            .arg("--runtime-dir")
            .arg(&self.runtime_root)
            .arg("--artifact-root")
            .arg(&self.artifact_root)
            .env("PYTHONPATH", &self.pythonpath_root)
            .env("DAEMON_CLAUDE_PROGRAM", &fake_claude.program)
            .output()
            .context("running daemon --run-next-job with fake Claude")?;

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

fn latest_semantic_job_id(conn: &Connection, parent_job_id: &str) -> Result<String> {
    conn.query_row(
        "\
        SELECT id
        FROM workflow_jobs
        WHERE parent_job_id = ?1
          AND kind = 'extract_document_semantics'
        ORDER BY created_at DESC, id DESC
        LIMIT 1
        ",
        params![parent_job_id],
        |row| row.get(0),
    )
    .context("loading latest semantic child job id")
}

fn latest_semantic_row(
    conn: &Connection,
    document_extraction_id: &str,
) -> Result<(String, String, String)> {
    conn.query_row(
        "\
        SELECT skill_id, target_domain, extraction_scope
        FROM document_semantic_extractions
        WHERE document_extraction_id = ?1
        ORDER BY rowid DESC
        LIMIT 1
        ",
        params![document_extraction_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    )
    .context("loading latest document_semantic_extractions row")
}

fn child_extraction_scopes(conn: &Connection, parent_job_id: &str) -> Result<Vec<String>> {
    let mut statement = conn.prepare(
        "\
        SELECT json_extract(input_payload, '$.request.request.extraction_scope')
        FROM workflow_jobs
        WHERE parent_job_id = ?1
          AND kind = 'extract_document_semantics'
        ORDER BY created_at, id
        ",
    )?;
    let rows = statement.query_map(params![parent_job_id], |row| row.get::<_, String>(0))?;
    rows.collect::<std::result::Result<Vec<_>, _>>()
        .context("loading semantic child extraction scopes")
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

fn document_set_extraction_state(conn: &Connection, document_set_id: &str) -> Result<String> {
    conn.query_row(
        "SELECT extraction_state FROM tender_document_sets WHERE id = ?1",
        params![document_set_id],
        |row| row.get(0),
    )
    .context("loading tender_document_sets.extraction_state")
}

fn job_count(conn: &Connection, kind: &str) -> Result<i64> {
    conn.query_row(
        "SELECT COUNT(*) FROM workflow_jobs WHERE kind = ?1",
        params![kind],
        |row| row.get(0),
    )
    .context("counting workflow_jobs by kind")
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
        chunk_plan: None,
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
        chunk_plan: None,
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
        chunk_plan: None,
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
        chunk_plan: None,
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
    let continuation_link_count: i64 = conn.query_row(
        "\
        SELECT COUNT(*)
        FROM workflow_jobs
        WHERE parent_job_id = ?1
          AND kind = 'extract_document_semantics'
          AND continuation_chain_id IS NOT NULL
        ",
        params!["job-struct-3"],
        |row| row.get(0),
    )?;
    ensure!(
        continuation_link_count == 0,
        "cross-skill cascade must not use continuation_chain_id"
    );
    Ok(())
}

#[test]
fn test_cascade_semantic_persistence_happy_path() -> Result<()> {
    let harness = CascadeHarness::new()?;
    harness.seed_process("proc-semantic-run")?;
    harness.seed_raw_document("raw-semantic-run")?;
    harness.seed_notice(
        "proc-semantic-run",
        "notice-semantic-run",
        "raw-semantic-run",
    )?;
    harness.seed_document_set(
        "proc-semantic-run",
        "notice-semantic-run",
        "docset-semantic-run",
    )?;
    harness.seed_extract_job(&ExtractJobSpec {
        job_id: "job-struct-semantic",
        process_id: "proc-semantic-run",
        raw_document_id: "raw-semantic-run",
        notice_id: Some("notice-semantic-run"),
        document_set_id: Some("docset-semantic-run"),
        document_role: Some("technical_specification"),
        expected_overlays: &["pdf-extraction/tender-specification"],
        chunk_plan: None,
    })?;

    let structural = harness.run_next_job()?;
    ensure!(structural.success, "stderr: {}", structural.stderr);

    let semantic = harness.run_next_job_with_claude_result(&sample_semantic_result(
        "pdf-extraction/tender-specification",
    ))?;
    ensure!(semantic.success, "stderr: {}", semantic.stderr);

    let conn = harness.connection()?;
    let semantic_job_id = latest_semantic_job_id(&conn, "job-struct-semantic")?;
    ensure!(job_status(&conn, &semantic_job_id)? == "completed");

    let output = job_output(&conn, "job-struct-semantic")?;
    let document_extraction_id = output["result"]["document_extraction_id"]
        .as_str()
        .ok_or_else(|| anyhow!("missing document_extraction_id"))?;
    let (skill_id, target_domain, extraction_scope) =
        latest_semantic_row(&conn, document_extraction_id)?;
    ensure!(skill_id == "pdf-extraction/tender-specification");
    ensure!(target_domain == "lots");
    ensure!(extraction_scope == "full_document");

    let semantic_state: String = conn.query_row(
        "SELECT semantic_extraction_state FROM document_extractions WHERE id = ?1",
        params![document_extraction_id],
        |row| row.get(0),
    )?;
    ensure!(semantic_state == "complete");
    Ok(())
}

#[test]
fn test_cascade_chunk_plan_enqueues_merge_job_when_budget_exceeded() -> Result<()> {
    let harness = CascadeHarness::new()?;
    harness.seed_process("proc-chunk")?;
    harness.seed_raw_document("raw-chunk")?;
    harness.seed_notice("proc-chunk", "notice-chunk", "raw-chunk")?;
    harness.seed_document_set("proc-chunk", "notice-chunk", "docset-chunk")?;
    harness.seed_extract_job(&ExtractJobSpec {
        job_id: "job-struct-chunk",
        process_id: "proc-chunk",
        raw_document_id: "raw-chunk",
        notice_id: Some("notice-chunk"),
        document_set_id: Some("docset-chunk"),
        document_role: Some("technical_specification"),
        expected_overlays: &["pdf-extraction/tender-specification"],
        chunk_plan: Some(json!({
            "plan_hash": "plan-123",
            "chunks": [
                {
                    "chunk_index": 0,
                    "page_start": 1,
                    "page_end": 2,
                    "section_refs": ["lot-1"],
                    "markdown": "# Chunk 0"
                },
                {
                    "chunk_index": 1,
                    "page_start": 3,
                    "page_end": 4,
                    "section_refs": ["lot-2"],
                    "markdown": "# Chunk 1"
                }
            ]
        })),
    })?;

    let run = harness.run_next_job()?;
    ensure!(run.success, "stderr: {}", run.stderr);

    let conn = harness.connection()?;
    let scopes = child_extraction_scopes(&conn, "job-struct-chunk")?;
    ensure!(scopes.len() == 3);
    ensure!(scopes.iter().any(|scope| scope == "chunk:plan-123:0"));
    ensure!(scopes.iter().any(|scope| scope == "chunk:plan-123:1"));
    ensure!(scopes.iter().any(|scope| scope == "chunk_merged:plan-123"));
    Ok(())
}

#[test]
fn test_cascade_structural_failure() -> Result<()> {
    let harness = CascadeHarness::new()?;
    harness.seed_process("proc-struct-fail")?;
    harness.seed_raw_document("raw-struct-fail")?;
    harness.seed_notice("proc-struct-fail", "notice-struct-fail", "raw-struct-fail")?;
    harness.seed_document_set(
        "proc-struct-fail",
        "notice-struct-fail",
        "docset-struct-fail",
    )?;
    harness.overwrite_pdf_bytes(b"")?;
    harness.seed_extract_job(&ExtractJobSpec {
        job_id: "job-struct-fail",
        process_id: "proc-struct-fail",
        raw_document_id: "raw-struct-fail",
        notice_id: Some("notice-struct-fail"),
        document_set_id: Some("docset-struct-fail"),
        document_role: Some("technical_specification"),
        expected_overlays: &["pdf-extraction/tender-specification"],
        chunk_plan: None,
    })?;

    let run = harness.run_next_job()?;
    ensure!(!run.success, "structural failure should fail the worker");
    ensure!(
        run.stderr.contains("EXTRACTION_EMPTY_PDF"),
        "stderr: {}",
        run.stderr
    );

    let conn = harness.connection()?;
    ensure!(job_status(&conn, "job-struct-fail")? == "failed");
    ensure!(child_job_count(&conn, "job-struct-fail")? == 0);
    let extraction_rows: i64 = conn.query_row(
        "SELECT COUNT(*) FROM document_extractions WHERE process_id = ?1",
        params!["proc-struct-fail"],
        |row| row.get(0),
    )?;
    ensure!(extraction_rows == 0);
    Ok(())
}

#[test]
fn test_no_pdf_graceful_skip() -> Result<()> {
    let harness = CascadeHarness::new()?;
    harness.seed_process("proc-no-pdf")?;
    harness.seed_notice("proc-no-pdf", "notice-no-pdf", "raw-no-pdf")?;
    harness.seed_document_set_with_state_and_documents(
        "proc-no-pdf",
        "notice-no-pdf",
        "docset-no-pdf",
        "none",
        &[],
    )?;

    let conn = harness.connection()?;
    ensure!(document_set_extraction_state(&conn, "docset-no-pdf")? == "none");
    ensure!(job_count(&conn, "extract_document_structure")? == 0);
    ensure!(
        conn.query_row::<i64, _, _>(
            "SELECT COUNT(*) FROM document_extractions WHERE document_set_id = ?1",
            params!["docset-no-pdf"],
            |row| row.get(0),
        )? == 0
    );
    Ok(())
}

#[test]
fn test_soft_stale_content_hash_change() -> Result<()> {
    let harness = CascadeHarness::new()?;
    harness.seed_process("proc-stale-hash")?;
    harness.seed_raw_document("raw-stale-hash")?;
    harness.seed_notice("proc-stale-hash", "notice-stale-hash", "raw-stale-hash")?;
    harness.seed_document_set("proc-stale-hash", "notice-stale-hash", "docset-stale-hash")?;

    let original_bytes = sample_pdf_bytes();
    let original_hash = format!("{:x}", Sha256::digest(&original_bytes));
    harness.seed_existing_document_extraction(
        "extract-old-hash",
        "raw-stale-hash",
        "proc-stale-hash",
        "notice-stale-hash",
        "docset-stale-hash",
        &original_hash,
        "2.85.0",
        "complete",
    )?;

    let mut updated_bytes = original_bytes;
    updated_bytes.extend_from_slice(b"%updated-content\n");
    harness.overwrite_pdf_bytes(&updated_bytes)?;

    harness.seed_extract_job(&ExtractJobSpec {
        job_id: "job-struct-stale-hash",
        process_id: "proc-stale-hash",
        raw_document_id: "raw-stale-hash",
        notice_id: Some("notice-stale-hash"),
        document_set_id: Some("docset-stale-hash"),
        document_role: Some("technical_specification"),
        expected_overlays: &["pdf-extraction/tender-specification"],
        chunk_plan: None,
    })?;

    let run = harness.run_next_job()?;
    ensure!(run.success, "stderr: {}", run.stderr);

    let conn = harness.connection()?;
    let output = job_output(&conn, "job-struct-stale-hash")?;
    ensure!(output["result"]["freshness_state"] == Value::String("soft_stale".to_owned()));
    ensure!(
        output["result"]["freshness_reason"] == Value::String("content_hash_change".to_owned())
    );
    let hard_invalidated = output["result"]["hard_invalidated_extraction_ids"]
        .as_array()
        .ok_or_else(|| anyhow!("missing hard_invalidated_extraction_ids"))?;
    ensure!(
        hard_invalidated.contains(&Value::String("extract-old-hash".to_owned())),
        "superseded extraction id missing from hard-invalid list"
    );
    Ok(())
}

#[test]
fn test_soft_stale_engine_upgrade() -> Result<()> {
    let harness = CascadeHarness::new()?;
    harness.seed_process("proc-stale-engine")?;
    harness.seed_raw_document("raw-stale-engine")?;
    harness.seed_notice(
        "proc-stale-engine",
        "notice-stale-engine",
        "raw-stale-engine",
    )?;
    harness.seed_document_set(
        "proc-stale-engine",
        "notice-stale-engine",
        "docset-stale-engine",
    )?;

    let source_hash = format!("{:x}", Sha256::digest(sample_pdf_bytes()));
    harness.seed_existing_document_extraction(
        "extract-old-engine",
        "raw-stale-engine",
        "proc-stale-engine",
        "notice-stale-engine",
        "docset-stale-engine",
        &source_hash,
        "2.84.0",
        "complete",
    )?;
    harness.configure_docling_runtime("2.86.0")?;

    harness.seed_extract_job(&ExtractJobSpec {
        job_id: "job-struct-stale-engine",
        process_id: "proc-stale-engine",
        raw_document_id: "raw-stale-engine",
        notice_id: Some("notice-stale-engine"),
        document_set_id: Some("docset-stale-engine"),
        document_role: Some("technical_specification"),
        expected_overlays: &["pdf-extraction/tender-specification"],
        chunk_plan: None,
    })?;

    let run = harness.run_next_job()?;
    ensure!(run.success, "stderr: {}", run.stderr);

    let conn = harness.connection()?;
    let output = job_output(&conn, "job-struct-stale-engine")?;
    ensure!(output["result"]["freshness_state"] == Value::String("soft_stale".to_owned()));
    ensure!(output["result"]["freshness_reason"] == Value::String("engine_upgrade".to_owned()));
    Ok(())
}

#[test]
fn test_hard_invalid_superseded() -> Result<()> {
    let harness = CascadeHarness::new()?;
    harness.seed_process("proc-hard-invalid")?;
    harness.seed_raw_document("raw-hard-invalid")?;
    harness.seed_notice(
        "proc-hard-invalid",
        "notice-hard-invalid",
        "raw-hard-invalid",
    )?;
    harness.seed_document_set(
        "proc-hard-invalid",
        "notice-hard-invalid",
        "docset-hard-invalid",
    )?;

    let original_bytes = sample_pdf_bytes();
    let original_hash = format!("{:x}", Sha256::digest(&original_bytes));
    harness.seed_existing_document_extraction(
        "extract-hard-invalid-old",
        "raw-hard-invalid",
        "proc-hard-invalid",
        "notice-hard-invalid",
        "docset-hard-invalid",
        &original_hash,
        "2.85.0",
        "complete",
    )?;

    let mut updated_bytes = original_bytes;
    updated_bytes.extend_from_slice(b"%superseded\n");
    harness.overwrite_pdf_bytes(&updated_bytes)?;

    harness.seed_extract_job(&ExtractJobSpec {
        job_id: "job-struct-hard-invalid",
        process_id: "proc-hard-invalid",
        raw_document_id: "raw-hard-invalid",
        notice_id: Some("notice-hard-invalid"),
        document_set_id: Some("docset-hard-invalid"),
        document_role: Some("technical_specification"),
        expected_overlays: &["pdf-extraction/tender-specification"],
        chunk_plan: None,
    })?;

    let run = harness.run_next_job()?;
    ensure!(run.success, "stderr: {}", run.stderr);

    let conn = harness.connection()?;
    let output = job_output(&conn, "job-struct-hard-invalid")?;
    let new_extraction_id = output["result"]["document_extraction_id"]
        .as_str()
        .ok_or_else(|| anyhow!("missing document_extraction_id"))?;
    let (is_current, superseded_by): (i64, Option<String>) = conn.query_row(
        "SELECT is_current, superseded_by_extraction_id FROM document_extractions WHERE id = ?1",
        params!["extract-hard-invalid-old"],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    ensure!(is_current == 0);
    ensure!(superseded_by.as_deref() == Some(new_extraction_id));
    Ok(())
}

#[test]
fn test_extraction_state_rollup() -> Result<()> {
    let harness = CascadeHarness::new()?;
    harness.seed_process("proc-rollup")?;
    harness.seed_raw_document("raw-rollup-1")?;
    harness.seed_raw_document("raw-rollup-2")?;
    harness.seed_notice("proc-rollup", "notice-rollup", "raw-rollup-1")?;
    harness.seed_document_set_with_documents(
        "proc-rollup",
        "notice-rollup",
        "docset-rollup",
        &["raw-rollup-1", "raw-rollup-2"],
    )?;

    harness.seed_extract_job(&ExtractJobSpec {
        job_id: "job-struct-rollup-1",
        process_id: "proc-rollup",
        raw_document_id: "raw-rollup-1",
        notice_id: Some("notice-rollup"),
        document_set_id: Some("docset-rollup"),
        document_role: Some("technical_specification"),
        expected_overlays: &["pdf-extraction/tender-specification"],
        chunk_plan: None,
    })?;

    let structural_first = harness.run_next_job()?;
    ensure!(
        structural_first.success,
        "stderr: {}",
        structural_first.stderr
    );

    let conn = harness.connection()?;
    ensure!(document_set_extraction_state(&conn, "docset-rollup")? == "partial");

    let semantic = harness.run_next_job_with_claude_result(&sample_semantic_result(
        "pdf-extraction/tender-specification",
    ))?;
    ensure!(semantic.success, "stderr: {}", semantic.stderr);
    ensure!(document_set_extraction_state(&conn, "docset-rollup")? == "partial");

    harness.seed_extract_job(&ExtractJobSpec {
        job_id: "job-struct-rollup-2",
        process_id: "proc-rollup",
        raw_document_id: "raw-rollup-2",
        notice_id: Some("notice-rollup"),
        document_set_id: Some("docset-rollup"),
        document_role: Some("administrative"),
        expected_overlays: &[],
        chunk_plan: None,
    })?;

    let structural_second = harness.run_next_job()?;
    ensure!(
        structural_second.success,
        "stderr: {}",
        structural_second.stderr
    );
    ensure!(document_set_extraction_state(&conn, "docset-rollup")? == "complete");
    Ok(())
}

#[test]
fn test_partial_document_set_only_enqueues_available_raw_documents() -> Result<()> {
    let harness = CascadeHarness::new()?;
    harness.seed_process("proc-partial-set")?;
    harness.seed_raw_document("raw-partial-available")?;
    harness.seed_notice(
        "proc-partial-set",
        "notice-partial-set",
        "raw-partial-available",
    )?;
    harness.seed_document_set_with_state_and_documents(
        "proc-partial-set",
        "notice-partial-set",
        "docset-partial-set",
        "partial",
        &["raw-partial-available", "raw-partial-missing"],
    )?;
    harness.seed_extract_job(&ExtractJobSpec {
        job_id: "job-struct-partial-set",
        process_id: "proc-partial-set",
        raw_document_id: "raw-partial-available",
        notice_id: Some("notice-partial-set"),
        document_set_id: Some("docset-partial-set"),
        document_role: Some("technical_specification"),
        expected_overlays: &["pdf-extraction/tender-specification"],
        chunk_plan: None,
    })?;

    let structural = harness.run_next_job()?;
    ensure!(structural.success, "stderr: {}", structural.stderr);

    let conn = harness.connection()?;
    let output = job_output(&conn, "job-struct-partial-set")?;
    ensure!(
        output["result"]["document_set_extraction_state"] == Value::String("partial".to_owned())
    );

    let semantic = harness.run_next_job_with_claude_result(&sample_semantic_result(
        "pdf-extraction/tender-specification",
    ))?;
    ensure!(semantic.success, "stderr: {}", semantic.stderr);
    ensure!(document_set_extraction_state(&conn, "docset-partial-set")? == "complete");
    Ok(())
}

#[test]
fn test_procurement_ingestion_regression() -> Result<()> {
    let main_rs_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/main.rs");
    let main_rs = fs::read_to_string(&main_rs_path)
        .with_context(|| format!("reading {}", main_rs_path.display()))?;
    ensure!(
        main_rs.contains(".starts_with(\"procurement-ingestion/\")"),
        "manual skill runner lost procurement-ingestion persistence branch"
    );
    ensure!(
        main_rs.contains(".starts_with(\"pdf-extraction/\")"),
        "manual skill runner lost pdf-extraction persistence branch"
    );
    Ok(())
}
