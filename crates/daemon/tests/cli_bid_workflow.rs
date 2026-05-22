#![forbid(unsafe_code)]

mod support;

use anyhow::{Context, Result, ensure};
use assert_cmd::Command;
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::{Value, json};
use tempfile::{TempDir, tempdir};

use crate::support::db::{DEFAULT_SCHEMA_EPOCH, StateDb, seed_process};

const BASIS_HASH: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

struct CliBidWorkflowFixture {
    _temp: TempDir,
    state_root: std::path::PathBuf,
    runtime_root: std::path::PathBuf,
    artifact_root: std::path::PathBuf,
}

impl CliBidWorkflowFixture {
    fn new() -> Result<Self> {
        let temp = tempdir()?;
        let state_root = temp.path().join("state");
        let runtime_root = temp.path().join("runtime");
        let artifact_root = temp.path().join("artifacts");
        std::fs::create_dir_all(&state_root)?;
        std::fs::create_dir_all(&runtime_root)?;
        std::fs::create_dir_all(&artifact_root)?;
        let state_db = StateDb::open(state_root.join("state.db"))?;
        ensure!(
            state_db.schema_epoch() == DEFAULT_SCHEMA_EPOCH,
            "test state db opened at unexpected schema epoch"
        );
        drop(state_db);
        Ok(Self {
            _temp: temp,
            state_root,
            runtime_root,
            artifact_root,
        })
    }

    fn conn(&self) -> Result<Connection> {
        Connection::open(self.state_root.join("state.db")).context("opening fixture state db")
    }

    fn seed_process(&self, process_id: &str) -> Result<()> {
        let conn = self.conn()?;
        seed_process(&conn, process_id)
    }

    fn seed_bid_job(&self, job_id: &str, process_id: &str) -> Result<()> {
        let conn = self.conn()?;
        conn.execute(
            "\
            INSERT INTO workflow_jobs (
              id, kind, process_id, scope_kind, scope_id, status, input_payload
            ) VALUES (?1, 'run_bid_workflow', NULL, 'process', ?2, 'failed', ?3)
            ",
            params![job_id, process_id, bid_job_payload(process_id).to_string()],
        )
        .context("seeding run_bid_workflow job")?;
        Ok(())
    }

    fn run_bid_workflow(&self, job_id: &str) -> Result<std::process::Output> {
        Command::cargo_bin("daemon")?
            .arg("--run-bid-workflow")
            .arg(job_id)
            .arg("--state-root")
            .arg(&self.state_root)
            .arg("--runtime-dir")
            .arg(&self.runtime_root)
            .arg("--artifact-root")
            .arg(&self.artifact_root)
            .output()
            .context("running daemon --run-bid-workflow")
    }
}

fn bid_job_payload(process_id: &str) -> Value {
    json!({
        "request": {
            "bid_scope_payload": {
                "processId": process_id,
                "lotIds": ["lot-1"],
                "candidateIds": ["candidate-1"],
                "selectedQuoteIds": ["quote-1"],
                "companyProfileVersion": "company-profile-v1",
                "bidPortalType": "procontract",
                "portalOpportunityRef": "DN123456",
                "reviewRequired": true,
                "decisionBasisHash": BASIS_HASH,
                "sideEffectCap": "REMOTE_DRAFT"
            },
            "company_profile": {
                "companyName": "Sorelon",
                "registrationNumber": "12345678",
                "vatNumber": null,
                "address": {
                    "line1": "1 Test Street",
                    "line2": null,
                    "city": "London",
                    "county": null,
                    "postcode": "SW1A 1AA",
                    "country": "GB"
                },
                "primaryContact": {
                    "name": "Operator",
                    "email": "operator@example.invalid",
                    "phone": "0000000000",
                    "role": "Procurement"
                },
                "certifications": [],
                "insurancePolicies": [],
                "financialStatements": null,
                "smeStatus": null,
                "equalityStatement": null,
                "environmentalPolicy": null
            },
            "bid_documents": [{
                "documentType": "pricing_schedule",
                "fileName": "pricing.pdf",
                "filePath": "artifacts/pricing.pdf",
                "contentHash": "sha256:pricing",
                "expiryDate": null,
                "issuingBody": null
            }],
            "operator_id": "operator-alinaaitova",
            "stage_run_id": "stage-run-bid-001"
        },
        "runner_meta": {
            "idempotency_key": "seed-bid-job",
            "visible_after_seq": 1,
            "executor_id": "test"
        }
    })
}

fn child_bid_job_count(conn: &Connection, parent_job_id: &str) -> Result<i64> {
    conn.query_row(
        "\
        SELECT COUNT(*)
        FROM workflow_jobs
        WHERE kind = 'run_bid_workflow'
          AND parent_job_id = ?1
        ",
        params![parent_job_id],
        |row| row.get(0),
    )
    .context("counting child bid workflow jobs")
}

fn parse_stdout_json(stdout: &str) -> Result<Value> {
    let start = stdout
        .find('{')
        .with_context(|| format!("daemon stdout did not contain JSON object: {stdout}"))?;
    let end = stdout
        .rfind('}')
        .with_context(|| format!("daemon stdout did not contain JSON object: {stdout}"))?;
    serde_json::from_str(&stdout[start..=end])
        .with_context(|| format!("parsing daemon stdout JSON from: {stdout}"))
}

#[test]
fn l6_t_daemon_runner_010_cli_run_bid_workflow_blocks_before_enqueue() -> Result<()> {
    let fixture = CliBidWorkflowFixture::new()?;
    fixture.seed_bid_job("bid-job-blocked", "missing-process")?;

    let output = fixture.run_bid_workflow("bid-job-blocked")?;

    ensure!(
        !output.status.success(),
        "blocked readiness path must exit non-zero"
    );
    let stderr = String::from_utf8(output.stderr).context("decoding daemon stderr")?;
    ensure!(
        stderr.contains("bid readiness blocked"),
        "stderr must include readiness blocker context: {stderr}"
    );
    ensure!(
        stderr.contains("PROCESS_NOT_FOUND"),
        "stderr must include blocker code: {stderr}"
    );
    let conn = fixture.conn()?;
    ensure!(
        child_bid_job_count(&conn, "bid-job-blocked")? == 0,
        "blocked CLI trigger must not enqueue a child bid job"
    );
    Ok(())
}

#[test]
fn l6_t_daemon_runner_010_cli_run_bid_workflow_enqueues_after_readiness() -> Result<()> {
    let fixture = CliBidWorkflowFixture::new()?;
    fixture.seed_process("process-bid-001")?;
    fixture.seed_bid_job("bid-job-ready", "process-bid-001")?;

    let output = fixture.run_bid_workflow("bid-job-ready")?;

    let stderr = String::from_utf8(output.stderr).context("decoding daemon stderr")?;
    ensure!(
        output.status.success(),
        "eligible readiness path should succeed, stderr: {stderr}"
    );
    let stdout = String::from_utf8(output.stdout).context("decoding daemon stdout")?;
    let report = parse_stdout_json(&stdout)?;
    let continuation_job_id = report
        .get("continuation_job_id")
        .and_then(Value::as_str)
        .context("continuation_job_id missing")?;
    ensure!(
        continuation_job_id.starts_with("job-"),
        "continuation_job_id should use job prefix"
    );

    let conn = fixture.conn()?;
    ensure!(
        child_bid_job_count(&conn, "bid-job-ready")? == 1,
        "eligible CLI trigger must enqueue exactly one child bid job"
    );
    let child_row: (String, Option<String>, String) = conn
        .query_row(
            "\
            SELECT kind, parent_job_id, input_payload
            FROM workflow_jobs
            WHERE id = ?1
            ",
            params![continuation_job_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()?
        .context("child bid workflow job missing")?;
    ensure!(
        child_row.0 == "run_bid_workflow",
        "unexpected child job kind"
    );
    ensure!(
        child_row.1.as_deref() == Some("bid-job-ready"),
        "child job must cite source bid job as parent"
    );
    let payload: Value = serde_json::from_str(&child_row.2)?;
    ensure!(
        payload["request"]["bid_scope_payload"]["processId"] == "process-bid-001",
        "child input payload must preserve bid scope"
    );
    ensure!(
        payload["runner_meta"]["idempotency_key"]
            == format!("manual-bid-workflow::bid-job-ready::{BASIS_HASH}"),
        "child idempotency key must be scoped to manual bid recovery"
    );
    Ok(())
}
