#![forbid(unsafe_code)]
#![allow(clippy::duplicate_mod)]

mod support;

#[path = "../src/bid_workflow/types.rs"]
#[allow(dead_code)]
mod types;

#[path = "../src/bid_workflow/decision_basis_hash.rs"]
#[allow(dead_code)]
mod decision_basis_hash;

use anyhow::{Context, Result, ensure};
use assert_cmd::Command;
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::{Value, json};
use tempfile::{TempDir, tempdir};

use crate::support::db::{DEFAULT_SCHEMA_EPOCH, FTS_SOURCE_ID, StateDb, seed_process};
use crate::types::{BidScopePayload, SideEffectClass};

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

    fn seed_bid_job(&self, job_id: &str, process_id: &str) -> Result<()> {
        self.seed_bid_job_with_basis_hash(job_id, process_id, BASIS_HASH)
    }

    fn seed_bid_job_with_basis_hash(
        &self,
        job_id: &str,
        process_id: &str,
        basis_hash: &str,
    ) -> Result<()> {
        let conn = self.conn()?;
        conn.execute(
            "\
            INSERT INTO workflow_jobs (
              id, kind, process_id, scope_kind, scope_id, status, input_payload
            ) VALUES (?1, 'run_bid_workflow', NULL, 'process', ?2, 'failed', ?3)
            ",
            params![
                job_id,
                process_id,
                bid_job_payload(process_id, basis_hash).to_string()
            ],
        )
        .context("seeding run_bid_workflow job")?;
        Ok(())
    }

    fn seed_ready_process_graph(&self, process_id: &str) -> Result<String> {
        let conn = self.conn()?;
        seed_bid_readiness_graph(&conn, process_id)?;
        let mut scope = bid_scope(process_id, BASIS_HASH);
        let current_hash = decision_basis_hash::compute(&conn, &scope)?;
        scope.decision_basis_hash.clone_from(&current_hash);
        seed_operator_decision_head(&conn, process_id, &current_hash)?;
        Ok(current_hash)
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

fn bid_scope(process_id: &str, basis_hash: &str) -> BidScopePayload {
    BidScopePayload {
        process_id: process_id.to_owned(),
        lot_ids: vec!["lot-1".to_owned()],
        candidate_ids: vec!["candidate-1".to_owned()],
        selected_quote_ids: vec!["quote-1".to_owned()],
        company_profile_version: "company-profile-v1".to_owned(),
        bid_portal_type: "procontract".to_owned(),
        portal_opportunity_ref: "DN123456".to_owned(),
        review_required: true,
        decision_basis_hash: basis_hash.to_owned(),
        side_effect_cap: SideEffectClass::RemoteDraft,
    }
}

fn bid_job_payload(process_id: &str, basis_hash: &str) -> Value {
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
                "decisionBasisHash": basis_hash,
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

fn seed_bid_readiness_graph(conn: &Connection, process_id: &str) -> Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO sources (id, kind, enabled, config) VALUES (?1, 'procurement', 1, '{}')",
        params![FTS_SOURCE_ID],
    )?;
    seed_process(conn, process_id)?;
    conn.execute(
        "\
        INSERT INTO contract_notices (
          id, process_id, source_id, source_notice_id, raw_document_id, title,
          procurement_stage, published_at, source_updated_at, normalized_payload, owner_id
        ) VALUES (
          'notice-1', ?1, ?2, 'source-notice-1', 'raw-document-1',
          'Bid workflow readiness notice', 'tender', '2026-05-22T00:00:00Z',
          '2026-05-22T00:00:00Z', '{}', 'local-owner'
        )
        ",
        params![process_id, FTS_SOURCE_ID],
    )?;
    conn.execute(
        "\
        INSERT INTO notice_lots (
          id, notice_id, lot_number, title, owner_id
        ) VALUES (
          'lot-1', 'notice-1', 1, 'Bid workflow readiness lot', 'local-owner'
        )
        ",
        [],
    )?;
    conn.execute(
        "\
        INSERT INTO pipeline_assessments (
          process_id, pipeline_state, validation_state, decision_basis_hash, owner_id
        ) VALUES (
          ?1, 'net_margin_passed', 'validated', 'pipeline-basis-cli-v1', 'local-owner'
        )
        ",
        params![process_id],
    )?;
    seed_candidate_quote_landed_cost(conn, process_id)?;
    seed_remote_projection_tables(conn)?;
    Ok(())
}

fn seed_candidate_quote_landed_cost(conn: &Connection, process_id: &str) -> Result<()> {
    conn.execute(
        "\
        INSERT INTO product_candidates (
          id, process_id, notice_id, lot_id, name, candidate_type,
          match_archetype, head_noun, owner_id
        ) VALUES (
          'candidate-1', ?1, 'notice-1', 'lot-1', 'Bid workflow product',
          'candidate', 'direct', 'widget', 'local-owner'
        )
        ",
        params![process_id],
    )?;
    conn.execute(
        "\
        INSERT INTO supplier_quotes (
          id, process_id, notice_id, product_candidate_id, product_name, region,
          quote_class, source_name, unit_price_original_minor, currency,
          unit_price_gbp_pence, price_signal_role, evidence, owner_id
        ) VALUES (
          'quote-1', ?1, 'notice-1', 'candidate-1', 'Bid workflow product',
          'UK', 'marketplace', 'fixture', 1000, 'GBP', 1000,
          'normal_market_quote', '{}', 'local-owner'
        )
        ",
        params![process_id],
    )?;
    conn.execute(
        "\
        INSERT INTO landed_cost_estimates (
          id, process_id, notice_id, product_candidate_id, supplier_quote_id,
          shipping_mode, mode_confidence_bps, declarable_commodity_code_10,
          commodity_confidence_bps, classification_stability_bps,
          customs_fx_rate, commercial_fx_rate, fx_basis_delta_bps, fx_rate_checked_at,
          supplier_cost_gbp_pence, freight_low_gbp_pence, freight_base_gbp_pence,
          freight_high_gbp_pence, customs_duty_gbp_pence, import_vat_gbp_pence,
          import_vat_treatment, uk_final_mile_cost_gbp_pence, other_fees_gbp_pence,
          economic_landed_cost_gbp_pence, cash_outlay_gbp_pence, cash_to_clear_gbp_pence,
          revenue_basis_type, revenue_confidence_bps, net_margin_low_bps,
          net_margin_base_bps, net_margin_high_bps, decision_grade, confidence_bps,
          warnings, blockers, tariff_checked_at, measures_fresh_until,
          classification_fresh_until, trade_remedy_watch_state, trade_remedy_cases_json,
          duty_breakdown_json, measure_interaction_json, owner_id
        ) VALUES (
          'lce-1', ?1, 'notice-1', 'candidate-1', 'quote-1',
          'parcel', 9000, '1234567890',
          9000, 9000,
          1.0, 1.0, 0, '2026-05-22T00:00:00Z',
          1000, 100, 125,
          150, 50, 235,
          'recoverable', 25, 10,
          1210, 1210, 1210,
          'notice_lot_value', 9000, 2000,
          3000, 4000, 'ready_for_bid', 9000,
          '[]', '[]', '2026-05-22T00:00:00Z', '2099-01-01T00:00:00Z',
          '2099-01-01T00:00:00Z', 'none', '[]',
          '[]', '[]', 'local-owner'
        )
        ",
        params![process_id],
    )?;
    Ok(())
}

fn seed_remote_projection_tables(conn: &Connection) -> Result<()> {
    conn.execute(
        "\
        CREATE TABLE IF NOT EXISTS operator_decision_heads (
          id TEXT PRIMARY KEY,
          process_id TEXT NOT NULL UNIQUE,
          disposition TEXT NOT NULL,
          reviewed_basis_hash TEXT,
          decision_stale INTEGER NOT NULL DEFAULT 0,
          deleted_at TEXT
        )
        ",
        [],
    )?;
    conn.execute(
        "\
        CREATE TABLE IF NOT EXISTS shared_company_profile (
          id TEXT PRIMARY KEY,
          company_name TEXT,
          registration_number TEXT,
          vat_number TEXT,
          address_json TEXT,
          contact_json TEXT,
          bid_profile_json TEXT,
          remote_revision INTEGER NOT NULL,
          deleted_at TEXT
        )
        ",
        [],
    )?;
    conn.execute(
        "\
        INSERT INTO shared_company_profile (
          id, company_name, registration_number, vat_number, address_json,
          contact_json, bid_profile_json, remote_revision, deleted_at
        ) VALUES (
          'company-profile-v1', 'Sorelon', '12345678', NULL, ?1, ?2, ?3, 1, NULL
        )
        ",
        params![
            json!({
                "line1": "1 Test Street",
                "line2": null,
                "city": "London",
                "county": null,
                "postcode": "SW1A 1AA",
                "country": "GB"
            })
            .to_string(),
            json!({
                "name": "Operator",
                "email": "operator@example.invalid",
                "phone": "0000000000",
                "role": "Procurement"
            })
            .to_string(),
            json!({
                "certifications": [{
                    "type": "iso9001",
                    "number": "ISO-CLI",
                    "expiryDate": "2099-01-01",
                    "issuingBody": "UKAS"
                }],
                "insurancePolicies": [{
                    "type": "public_liability",
                    "provider": "Fixture Insurance",
                    "policyNumber": "PL-CLI",
                    "coverAmountPence": 500_000_000_i64,
                    "expiryDate": "2099-01-01"
                }]
            })
            .to_string(),
        ],
    )?;
    Ok(())
}

fn seed_operator_decision_head(
    conn: &Connection,
    process_id: &str,
    basis_hash: &str,
) -> Result<()> {
    conn.execute(
        "\
        INSERT INTO operator_decision_heads (
          id, process_id, disposition, reviewed_basis_hash, decision_stale, deleted_at
        ) VALUES (
          'decision-cli', ?1, 'approved', ?2, 0, NULL
        )
        ",
        params![process_id, basis_hash],
    )?;
    Ok(())
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
    let json_slice = stdout
        .get(start..=end)
        .with_context(|| format!("daemon stdout JSON bounds were invalid: {stdout}"))?;
    serde_json::from_str(json_slice)
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
    let basis_hash = fixture.seed_ready_process_graph("process-bid-001")?;
    fixture.seed_bid_job_with_basis_hash("bid-job-ready", "process-bid-001", &basis_hash)?;

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
            == format!("manual-bid-workflow::bid-job-ready::{basis_hash}"),
        "child idempotency key must be scoped to manual bid recovery"
    );
    Ok(())
}
