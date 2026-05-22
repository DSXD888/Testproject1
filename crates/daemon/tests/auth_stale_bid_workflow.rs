#![forbid(unsafe_code)]

mod support;

use anyhow::{Context, Result, ensure};
use assert_cmd::Command;
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::{Value, json};
use tempfile::{TempDir, tempdir};

use crate::support::db::{DEFAULT_SCHEMA_EPOCH, StateDb, seed_process};

const BASIS_HASH: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

struct AuthStaleBidWorkflowFixture {
    _temp: TempDir,
    state_root: std::path::PathBuf,
    runtime_root: std::path::PathBuf,
    artifact_root: std::path::PathBuf,
}

impl AuthStaleBidWorkflowFixture {
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

    fn seed_auth_stale_bid_job(
        &self,
        job_id: &str,
        process_id: &str,
        retry_attempt: Option<u64>,
    ) -> Result<()> {
        let conn = self.conn()?;
        conn.execute(
            "\
            INSERT INTO workflow_jobs (
              id, kind, process_id, scope_kind, scope_id, status, input_payload
            ) VALUES (?1, 'run_bid_workflow', ?2, 'process', ?2, 'pending', ?3)
            ",
            params![
                job_id,
                process_id,
                auth_stale_bid_job_payload(process_id, retry_attempt).to_string()
            ],
        )
        .context("seeding auth-stale run_bid_workflow job")?;
        Ok(())
    }

    fn run_next_job(&self) -> Result<std::process::Output> {
        Command::cargo_bin("daemon")?
            .arg("--run-next-job")
            .arg("--state-root")
            .arg(&self.state_root)
            .arg("--runtime-dir")
            .arg(&self.runtime_root)
            .arg("--artifact-root")
            .arg(&self.artifact_root)
            .output()
            .context("running daemon --run-next-job")
    }
}

fn auth_stale_bid_job_payload(process_id: &str, retry_attempt: Option<u64>) -> Value {
    let mut request = json!({
        "bid_scope_payload": valid_scope(process_id),
        "company_profile": valid_company_profile(),
        "bid_documents": [valid_document()],
        "operator_id": "operator-alinaaitova",
        "stage_run_id": "stage-run-bid-001",
        "simulate_auth_state_stale": true
    });
    if let Some(attempt) = retry_attempt {
        request["auth_retry"] = json!({
            "attempt": attempt,
            "auth_skill_id": "auth-signin/procontract",
            "auth_refresh_job_id": "job-auth-refresh-existing"
        });
    }

    json!({
        "request": request,
        "runner_meta": {
            "idempotency_key": "seed-auth-stale-bid-job",
            "visible_after_seq": 1,
            "executor_id": "test"
        }
    })
}

fn valid_scope(process_id: &str) -> Value {
    json!({
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
    })
}

fn valid_company_profile() -> Value {
    json!({
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
    })
}

fn valid_document() -> Value {
    json!({
        "documentType": "pricing_schedule",
        "fileName": "pricing.pdf",
        "filePath": "artifacts/pricing.pdf",
        "contentHash": "sha256:pricing",
        "expiryDate": null,
        "issuingBody": null
    })
}

fn job_status(conn: &Connection, job_id: &str) -> Result<String> {
    conn.query_row(
        "SELECT status FROM workflow_jobs WHERE id = ?1",
        params![job_id],
        |row| row.get(0),
    )
    .with_context(|| format!("reading status for {job_id}"))
}

fn job_payload(conn: &Connection, job_id: &str, column: &str) -> Result<Value> {
    let raw: String = conn.query_row(
        &format!("SELECT {column} FROM workflow_jobs WHERE id = ?1"),
        params![job_id],
        |row| row.get(0),
    )?;
    serde_json::from_str(&raw).with_context(|| format!("parsing {column} for {job_id}"))
}

#[test]
fn l6_t_daemon_runner_009_auth_stale_enqueues_auth_refresh_and_one_bid_retry() -> Result<()> {
    let fixture = AuthStaleBidWorkflowFixture::new()?;
    fixture.seed_process("process-auth-stale-001")?;
    fixture.seed_auth_stale_bid_job("bid-auth-stale-original", "process-auth-stale-001", None)?;

    let output = fixture.run_next_job()?;
    let stderr = String::from_utf8(output.stderr).context("decoding daemon stderr")?;
    ensure!(
        output.status.success(),
        "first AUTH_STATE_STALE should enqueue bounded retry, stderr: {stderr}"
    );

    let conn = fixture.conn()?;
    ensure!(
        job_status(&conn, "bid-auth-stale-original")? == "completed",
        "original bid job should complete by delegating auth refresh"
    );
    let output_payload = job_payload(&conn, "bid-auth-stale-original", "output_payload")?;
    let auth_job_id = output_payload["runner_meta"]["auth_refresh_job_id"]
        .as_str()
        .context("auth_refresh_job_id missing")?;
    let retry_job_id = output_payload["runner_meta"]["continuation_job_id"]
        .as_str()
        .context("continuation_job_id missing")?;

    let auth_row: (String, Option<String>, String) = conn
        .query_row(
            "\
            SELECT kind, parent_job_id, status
            FROM workflow_jobs
            WHERE id = ?1
            ",
            params![auth_job_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()?
        .context("auth refresh sub-job missing")?;
    ensure!(
        auth_row.0 == "run_auth_signin_procontract",
        "auth sub-job should target auth-signin/procontract"
    );
    ensure!(
        auth_row.1.as_deref() == Some("bid-auth-stale-original"),
        "auth sub-job must cite the stale bid job as parent"
    );
    ensure!(auth_row.2 == "pending", "auth sub-job should be pending");

    let retry_row: (String, Option<String>, String, String) = conn
        .query_row(
            "\
            SELECT kind, parent_job_id, status, input_payload
            FROM workflow_jobs
            WHERE id = ?1
            ",
            params![retry_job_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .optional()?
        .context("bid retry job missing")?;
    ensure!(
        retry_row.0 == "run_bid_workflow",
        "retry job should rerun bid workflow"
    );
    ensure!(
        retry_row.1.as_deref() == Some(auth_job_id),
        "bid retry must be chained after auth refresh"
    );
    ensure!(retry_row.2 == "pending", "bid retry should be pending");

    let retry_payload: Value = serde_json::from_str(&retry_row.3)?;
    ensure!(
        retry_payload["request"]["auth_retry"]["attempt"] == 1,
        "bid retry input should record bounded retry attempt"
    );
    ensure!(
        retry_payload["request"]
            .get("simulate_auth_state_stale")
            .is_none(),
        "bid retry should not carry the synthetic stale-auth signal forward"
    );
    Ok(())
}

#[test]
fn l6_t_daemon_runner_009_second_auth_stale_fails_closed_without_third_job() -> Result<()> {
    let fixture = AuthStaleBidWorkflowFixture::new()?;
    fixture.seed_process("process-auth-stale-002")?;
    fixture.seed_auth_stale_bid_job("bid-auth-stale-retry", "process-auth-stale-002", Some(1))?;

    let output = fixture.run_next_job()?;
    ensure!(
        !output.status.success(),
        "second AUTH_STATE_STALE should exhaust bounded retry"
    );
    let stderr = String::from_utf8(output.stderr).context("decoding daemon stderr")?;
    ensure!(
        stderr.contains("BID_PORTAL_AUTH_EXPIRED"),
        "stderr should report bid portal auth expiry: {stderr}"
    );

    let conn = fixture.conn()?;
    ensure!(
        job_status(&conn, "bid-auth-stale-retry")? == "failed",
        "second stale-auth job should fail closed"
    );
    let (last_error, output_payload): (String, String) = conn.query_row(
        "\
        SELECT last_error, output_payload
        FROM workflow_jobs
        WHERE id = 'bid-auth-stale-retry'
        ",
        [],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    ensure!(
        last_error.contains("BID_PORTAL_AUTH_EXPIRED"),
        "last_error should cite BID_PORTAL_AUTH_EXPIRED"
    );
    let parsed_output: Value = serde_json::from_str(&output_payload)?;
    ensure!(
        parsed_output["auth_retry"]["bounded_retry_exhausted"] == true,
        "failure payload should record bounded retry exhaustion"
    );
    let auth_jobs: i64 = conn.query_row(
        "SELECT COUNT(*) FROM workflow_jobs WHERE kind = 'run_auth_signin_procontract'",
        [],
        |row| row.get(0),
    )?;
    ensure!(
        auth_jobs == 0,
        "second stale-auth must not enqueue another auth job"
    );
    Ok(())
}
