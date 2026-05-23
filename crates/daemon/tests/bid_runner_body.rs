#![forbid(unsafe_code)]
#![allow(clippy::duplicate_mod)]

mod support;

#[path = "../src/bid_workflow/types.rs"]
#[allow(dead_code)]
mod types;

#[path = "../src/bid_workflow/decision_basis_hash.rs"]
#[allow(dead_code)]
mod decision_basis_hash;

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, ensure};
use assert_cmd::Command;
use rusqlite::{Connection, params};
use serde_json::{Value, json};
use tempfile::{TempDir, tempdir};

use crate::support::db::{DEFAULT_SCHEMA_EPOCH, FTS_SOURCE_ID, StateDb, seed_process};
use crate::types::{BidScopePayload, SideEffectClass};

const BASIS_HASH_PLACEHOLDER: &str =
    "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const PROCESS_ID: &str = "process-bid-runner";
const NOTICE_ID: &str = "notice-bid-runner";
const LOT_ID: &str = "lot-bid-runner";
const CANDIDATE_ID: &str = "candidate-bid-runner";
const QUOTE_ID: &str = "quote-bid-runner";
const PROFILE_ID: &str = "company-profile-bid-runner";
const PORTAL_REF: &str = "DN-BID-RUNNER-001";

struct BidRunnerFixture {
    _temp: TempDir,
    state_root: PathBuf,
    runtime_root: PathBuf,
    artifact_root: PathBuf,
}

impl BidRunnerFixture {
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

    fn seed_ready_scope(&self) -> Result<String> {
        let conn = self.conn()?;
        seed_bid_readiness_graph(&conn)?;
        seed_remote_projection_tables(&conn)?;
        let mut scope = bid_scope(BASIS_HASH_PLACEHOLDER);
        let current_hash = decision_basis_hash::compute(&conn, &scope)?;
        scope.decision_basis_hash.clone_from(&current_hash);
        seed_operator_decision_head(&conn, &current_hash)?;
        Ok(current_hash)
    }

    fn seed_bid_job(&self, job_id: &str, basis_hash: &str) -> Result<()> {
        let conn = self.conn()?;
        conn.execute(
            "\
            INSERT INTO workflow_jobs (
              id, kind, process_id, scope_kind, scope_id, status, input_payload
            ) VALUES (?1, 'run_bid_workflow', ?2, 'process', ?2, 'pending', ?3)
            ",
            params![
                job_id,
                PROCESS_ID,
                bid_job_payload(job_id, basis_hash).to_string()
            ],
        )
        .with_context(|| format!("seeding bid workflow job {job_id}"))?;
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

fn bid_scope(basis_hash: &str) -> BidScopePayload {
    BidScopePayload {
        process_id: PROCESS_ID.to_owned(),
        lot_ids: vec![LOT_ID.to_owned()],
        candidate_ids: vec![CANDIDATE_ID.to_owned()],
        selected_quote_ids: vec![QUOTE_ID.to_owned()],
        company_profile_version: PROFILE_ID.to_owned(),
        bid_portal_type: "procontract".to_owned(),
        portal_opportunity_ref: PORTAL_REF.to_owned(),
        review_required: true,
        decision_basis_hash: basis_hash.to_owned(),
        side_effect_cap: SideEffectClass::RemoteDraft,
    }
}

fn bid_job_payload(job_id: &str, basis_hash: &str) -> Value {
    json!({
        "request": {
            "bid_scope_payload": bid_scope_payload_json(basis_hash),
            "company_profile": company_profile_json(),
            "bid_documents": [bid_document_json()],
            "operator_id": "operator-alinaaitova",
            "stage_run_id": format!("stage-run-{job_id}"),
            "simulate_bid_skill_result": simulated_draft_result()
        },
        "runner_meta": {
            "idempotency_key": format!("bid-runner-body::{job_id}::{basis_hash}"),
            "visible_after_seq": 1,
            "executor_id": "test"
        }
    })
}

fn bid_scope_payload_json(basis_hash: &str) -> Value {
    json!({
        "processId": PROCESS_ID,
        "lotIds": [LOT_ID],
        "candidateIds": [CANDIDATE_ID],
        "selectedQuoteIds": [QUOTE_ID],
        "companyProfileVersion": PROFILE_ID,
        "bidPortalType": "procontract",
        "portalOpportunityRef": PORTAL_REF,
        "reviewRequired": true,
        "decisionBasisHash": basis_hash,
        "sideEffectCap": "REMOTE_DRAFT"
    })
}

fn company_profile_json() -> Value {
    json!({
        "companyName": "Sorelon Bid Runner Ltd",
        "registrationNumber": "12345678",
        "vatNumber": "GB123456789",
        "address": {
            "line1": "1 Runner Street",
            "line2": null,
            "city": "London",
            "county": null,
            "postcode": "SW1A 1AA",
            "country": "GB"
        },
        "primaryContact": {
            "name": "Procurement Lead",
            "email": "procurement@example.invalid",
            "phone": "+442070000000",
            "role": "Bid manager"
        },
        "certifications": [{
            "type": "iso9001",
            "number": "ISO-BID-RUNNER",
            "expiryDate": "2099-01-01",
            "issuingBody": "UKAS"
        }],
        "insurancePolicies": [{
            "type": "public_liability",
            "provider": "Runner Mutual",
            "policyNumber": "PL-BID-RUNNER",
            "coverAmountPence": 500_000_000,
            "expiryDate": "2099-01-01"
        }],
        "financialStatements": {
            "lastFiledYear": 2025,
            "companiesHouseRef": "CH-BID-RUNNER"
        },
        "smeStatus": "small",
        "equalityStatement": "Current",
        "environmentalPolicy": "Current"
    })
}

fn bid_document_json() -> Value {
    json!({
        "documentType": "pricing_schedule",
        "fileName": "pricing.pdf",
        "filePath": "artifacts/pricing.pdf",
        "contentHash": "sha256:pricing",
        "expiryDate": null,
        "issuingBody": null
    })
}

fn simulated_draft_result() -> Value {
    json!({
        "finalSideEffectClass": "REMOTE_DRAFT",
        "portalStateVerified": true,
        "validationApproved": true,
        "draftSaveResult": {
            "draftSaved": true,
            "draftId": "draft-bid-runner-001",
            "sideEffectClass": "REMOTE_DRAFT"
        }
    })
}

fn seed_bid_readiness_graph(conn: &Connection) -> Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO sources (id, kind, enabled, config) VALUES (?1, 'procurement', 1, '{}')",
        params![FTS_SOURCE_ID],
    )?;
    seed_process(conn, PROCESS_ID)?;
    conn.execute(
        "\
        INSERT INTO contract_notices (
          id, process_id, source_id, source_notice_id, raw_document_id, title,
          procurement_stage, published_at, source_updated_at, normalized_payload, owner_id
        ) VALUES (
          ?1, ?2, ?3, 'source-notice-bid-runner', 'raw-document-bid-runner',
          'Bid runner notice', 'tender', '2026-05-22T00:00:00Z',
          '2026-05-22T00:00:00Z', '{}', 'local-owner'
        )
        ",
        params![NOTICE_ID, PROCESS_ID, FTS_SOURCE_ID],
    )?;
    conn.execute(
        "\
        INSERT INTO notice_lots (
          id, notice_id, lot_number, title, owner_id
        ) VALUES (
          ?1, ?2, 1, 'Bid runner lot', 'local-owner'
        )
        ",
        params![LOT_ID, NOTICE_ID],
    )?;
    conn.execute(
        "\
        INSERT INTO pipeline_assessments (
          process_id, pipeline_state, validation_state, decision_basis_hash, owner_id
        ) VALUES (
          ?1, 'net_margin_passed', 'validated', 'pipeline-basis-bid-runner-v1', 'local-owner'
        )
        ",
        params![PROCESS_ID],
    )?;
    seed_candidate_quote_and_landed_cost(conn)?;
    Ok(())
}

fn seed_candidate_quote_and_landed_cost(conn: &Connection) -> Result<()> {
    conn.execute(
        "\
        INSERT INTO product_candidates (
          id, process_id, notice_id, lot_id, name, candidate_type,
          match_archetype, head_noun, owner_id
        ) VALUES (
          ?1, ?2, ?3, ?4, 'Bid runner product',
          'candidate', 'direct', 'widget', 'local-owner'
        )
        ",
        params![CANDIDATE_ID, PROCESS_ID, NOTICE_ID, LOT_ID],
    )?;
    conn.execute(
        "\
        INSERT INTO supplier_quotes (
          id, process_id, notice_id, product_candidate_id, product_name, region,
          quote_class, source_name, unit_price_original_minor, currency,
          unit_price_gbp_pence, price_signal_role, evidence, owner_id
        ) VALUES (
          ?1, ?2, ?3, ?4, 'Bid runner product', 'UK', 'marketplace',
          'fixture', 1000, 'GBP', 1000, 'normal_market_quote', '{}', 'local-owner'
        )
        ",
        params![QUOTE_ID, PROCESS_ID, NOTICE_ID, CANDIDATE_ID],
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
          'lce-bid-runner', ?1, ?2, ?3, ?4,
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
        params![PROCESS_ID, NOTICE_ID, CANDIDATE_ID, QUOTE_ID],
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
          ?1, 'Sorelon Bid Runner Ltd', '12345678', 'GB123456789', ?2, ?3, ?4, 1, NULL
        )
        ",
        params![
            PROFILE_ID,
            json!({
                "line1": "1 Runner Street",
                "line2": null,
                "city": "London",
                "county": null,
                "postcode": "SW1A 1AA",
                "country": "GB"
            })
            .to_string(),
            json!({
                "name": "Procurement Lead",
                "email": "procurement@example.invalid",
                "phone": "+442070000000",
                "role": "Bid manager"
            })
            .to_string(),
            json!({
                "certifications": [{
                    "type": "iso9001",
                    "number": "ISO-BID-RUNNER",
                    "expiryDate": "2099-01-01",
                    "issuingBody": "UKAS"
                }],
                "insurancePolicies": [{
                    "type": "public_liability",
                    "provider": "Runner Mutual",
                    "policyNumber": "PL-BID-RUNNER",
                    "coverAmountPence": 500_000_000,
                    "expiryDate": "2099-01-01"
                }],
                "financialStatements": {
                    "lastFiledYear": 2025,
                    "companiesHouseRef": "CH-BID-RUNNER"
                },
                "smeStatus": "small",
                "equalityStatement": "Current",
                "environmentalPolicy": "Current"
            })
            .to_string(),
        ],
    )?;
    Ok(())
}

fn seed_operator_decision_head(conn: &Connection, basis_hash: &str) -> Result<()> {
    conn.execute(
        "\
        INSERT INTO operator_decision_heads (
          id, process_id, disposition, reviewed_basis_hash, decision_stale, deleted_at
        ) VALUES (
          'decision-bid-runner', ?1, 'approved', ?2, 0, NULL
        )
        ",
        params![PROCESS_ID, basis_hash],
    )?;
    Ok(())
}

fn run_successful_bid_job(fixture: &BidRunnerFixture, job_id: &str) -> Result<Value> {
    let basis_hash = fixture.seed_ready_scope()?;
    fixture.seed_bid_job(job_id, &basis_hash)?;

    let output = fixture.run_next_job()?;
    let stderr = String::from_utf8(output.stderr).context("decoding daemon stderr")?;
    ensure!(
        output.status.success(),
        "simulated bid workflow should complete, stderr: {stderr}"
    );
    let stdout = String::from_utf8(output.stdout).context("decoding daemon stdout")?;
    ensure!(
        !stdout.contains("Phase 3 implements bid workflow job execution"),
        "runner body still contains Phase 3 sentinel: {stdout}"
    );

    let conn = fixture.conn()?;
    job_payload(&conn, job_id, "output_payload")
}

fn job_payload(conn: &Connection, job_id: &str, column: &str) -> Result<Value> {
    let raw: String = conn.query_row(
        &format!("SELECT {column} FROM workflow_jobs WHERE id = ?1"),
        params![job_id],
        |row| row.get(0),
    )?;
    serde_json::from_str(&raw).with_context(|| format!("parsing {column} for {job_id}"))
}

fn bid_workflow_state_row(conn: &Connection) -> Result<(String, String, i64, String)> {
    conn.query_row(
        "\
        SELECT bid_state, bid_job_id, row_version, stage_run_id
        FROM bid_workflow_states
        WHERE process_id = ?1
        ",
        params![PROCESS_ID],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
    )
    .context("reading bid_workflow_states row")
}

fn session_log_row(conn: &Connection, session_id: &str) -> Result<(String, String, String)> {
    conn.query_row(
        "\
        SELECT run_manifest_path, terminal_output_path, structured_trace
        FROM agent_session_logs
        WHERE id = ?1
        ",
        params![session_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    )
    .context("reading agent_session_logs row")
}

fn artifact_refs(output_payload: &Value) -> Result<Vec<String>> {
    output_payload
        .pointer("/artifact_refs")
        .context("artifact_refs missing")?
        .as_array()
        .context("artifact_refs must be an array")?
        .iter()
        .map(|value| {
            value
                .as_str()
                .map(str::to_owned)
                .context("artifact ref must be a string")
        })
        .collect()
}

fn json_str_at<'a>(value: &'a Value, pointer: &str) -> Result<&'a str> {
    value
        .pointer(pointer)
        .with_context(|| format!("JSON pointer missing: {pointer}"))?
        .as_str()
        .with_context(|| format!("JSON pointer must be a string: {pointer}"))
}

fn path_exists(path: &str) -> bool {
    Path::new(path).exists()
}

#[test]
#[doc = "L6-T-daemon_runner-003 L6-T-daemon_runner-005 L6-T-daemon_runner-007 L6-T-daemon_runner-008"]
fn bid_runner_body_runs_simulated_draft_with_full_audit_trail() -> Result<()> {
    let fixture = BidRunnerFixture::new()?;
    let output_payload = run_successful_bid_job(&fixture, "bid-runner-body-001")?;

    ensure!(
        json_str_at(&output_payload, "/runner_meta/bid_state")? == "draft_saved",
        "runner output should report draft_saved"
    );
    ensure!(
        json_str_at(
            &output_payload,
            "/skill_input/bidScopePayload/portalOpportunityRef"
        )? == PORTAL_REF,
        "skill input should carry portal opportunity ref"
    );
    ensure!(
        json_str_at(&output_payload, "/skill_input/companyProfile/companyName")?
            == "Sorelon Bid Runner Ltd",
        "skill input should inject CompanyProfile"
    );
    ensure!(
        json_str_at(&output_payload, "/skill_input/bidDocuments/0/fileName")? == "pricing.pdf",
        "skill input should inject BidDocuments"
    );
    ensure!(
        json_str_at(
            &output_payload,
            "/skill_input/selectedQuoteSummaries/0/quoteId"
        )? == QUOTE_ID,
        "skill input should inject selected quote summaries"
    );

    let conn = fixture.conn()?;
    let state = bid_workflow_state_row(&conn)?;
    ensure!(
        state.0 == "draft_saved",
        "bid state should persist draft_saved"
    );
    ensure!(
        state.1 == "bid-runner-body-001",
        "bid state should cite the executed job"
    );

    let session_id = json_str_at(&output_payload, "/runner_meta/session_id")?;
    let (manifest_path, terminal_output_path, _structured_trace) =
        session_log_row(&conn, session_id)?;
    ensure!(
        path_exists(&manifest_path),
        "run manifest file should exist"
    );
    ensure!(
        path_exists(&terminal_output_path),
        "compressed/raw PTY output file should exist"
    );
    let manifest = std::fs::read_to_string(&manifest_path)
        .with_context(|| format!("reading run manifest {manifest_path}"))?;
    ensure!(
        manifest.contains("side_effect_reached=RemoteDraft")
            && manifest.contains(PORTAL_REF)
            && manifest.contains("side_effect_progression:")
            && manifest.contains("screenshot_path:"),
        "run manifest should cite side-effect progression, portal ref, and screenshot appendix rows"
    );
    for artifact_ref in artifact_refs(&output_payload)? {
        ensure!(
            path_exists(&artifact_ref),
            "artifact ref should exist on disk: {artifact_ref}"
        );
    }
    Ok(())
}

#[test]
#[doc = "L6-T-daemon_runner-006"]
fn bid_runner_body_idempotent_retry_updates_one_process_state_row() -> Result<()> {
    let fixture = BidRunnerFixture::new()?;
    let basis_hash = fixture.seed_ready_scope()?;
    fixture.seed_bid_job("bid-runner-body-002a", &basis_hash)?;
    fixture.seed_bid_job("bid-runner-body-002b", &basis_hash)?;

    let first = fixture.run_next_job()?;
    ensure!(
        first.status.success(),
        "first simulated run should complete: {}",
        String::from_utf8_lossy(&first.stderr)
    );
    let second = fixture.run_next_job()?;
    ensure!(
        second.status.success(),
        "retry simulated run should complete: {}",
        String::from_utf8_lossy(&second.stderr)
    );

    let conn = fixture.conn()?;
    let rows: i64 = conn.query_row(
        "SELECT COUNT(*) FROM bid_workflow_states WHERE process_id = ?1",
        params![PROCESS_ID],
        |row| row.get(0),
    )?;
    let state = bid_workflow_state_row(&conn)?;
    ensure!(rows == 1, "idempotent retry should keep one process row");
    ensure!(
        state.1 == "bid-runner-body-002b",
        "retry should update the existing process row to the latest job"
    );
    ensure!(
        state.2 >= 6,
        "two complete runs should update row_version instead of inserting a duplicate"
    );
    Ok(())
}

#[test]
#[doc = "L6-T-daemon_runner-011"]
fn bid_runner_body_records_bid_automation_priority() -> Result<()> {
    let fixture = BidRunnerFixture::new()?;
    let output_payload = run_successful_bid_job(&fixture, "bid-runner-body-003")?;
    let session_id = json_str_at(&output_payload, "/runner_meta/session_id")?;

    let conn = fixture.conn()?;
    let (_manifest_path, _terminal_output_path, structured_trace) =
        session_log_row(&conn, session_id)?;
    let trace: Value = serde_json::from_str(&structured_trace)?;
    ensure!(
        json_str_at(&trace, "/resource_priority")? == "bid_automation",
        "synthetic PTY trace should record BidAutomation admission priority"
    );
    Ok(())
}

#[test]
#[doc = "L6-T-daemon_runner-012"]
fn bid_runner_body_uses_support_state_db_fixture() -> Result<()> {
    let fixture = BidRunnerFixture::new()?;
    let state_db = StateDb::open(fixture.state_root.join("state.db"))?;
    ensure!(
        state_db.schema_epoch() == DEFAULT_SCHEMA_EPOCH,
        "runner test fixture must use support::db StateDb with the current schema"
    );
    Ok(())
}
