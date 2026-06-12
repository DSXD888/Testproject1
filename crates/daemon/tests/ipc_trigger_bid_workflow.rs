#![forbid(unsafe_code)]
#![allow(clippy::duplicate_mod)]

mod support;

#[path = "../src/agents/contracts.rs"]
#[allow(dead_code)]
mod contracts_impl;
#[path = "../src/ipc/handlers/jobs.rs"]
#[allow(dead_code)]
mod jobs_handler_impl;
#[path = "../src/jobs/runner.rs"]
#[allow(dead_code)]
mod runner_impl;

mod db {
    pub mod sqlite {
        pub use crate::support::db::StateDb;
    }
}

mod jobs {
    pub mod runner {
        pub use crate::runner_impl::*;
    }
}

use anyhow::{Context, Result, ensure};
use rusqlite::OptionalExtension;
use serde_json::Value;

use crate::jobs_handler_impl::{
    BidDocument, BidScopePayload, CompanyAddress, CompanyContact, CompanyProfile,
    JobsHandlerContext, SideEffectClass, TriggerBidWorkflowParams, TriggerEnrichmentParams,
    TriggerIngestionParams, trigger_bid_workflow, trigger_enrichment, trigger_ingestion,
};
use crate::support::db::{FTS_SOURCE_ID, StateDb, TestStateDb, seed_process};

const BASIS_HASH: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const AUTONOMOUS_SCHEDULING_STATUS: &str = include_str!(
    "../../../docs/pipeline/autonomous-daemon-scheduling/artifacts/autonomous-daemon-scheduling-status.yaml"
);

fn valid_scope(process_id: &str) -> BidScopePayload {
    BidScopePayload {
        process_id: process_id.to_owned(),
        lot_ids: vec!["lot-1".to_owned()],
        candidate_ids: vec!["candidate-1".to_owned()],
        selected_quote_ids: vec!["quote-1".to_owned()],
        company_profile_version: "company-profile-v1".to_owned(),
        bid_portal_type: "procontract".to_owned(),
        portal_opportunity_ref: "DN123456".to_owned(),
        review_required: true,
        decision_basis_hash: BASIS_HASH.to_owned(),
        side_effect_cap: SideEffectClass::RemoteDraft,
    }
}

fn valid_company_profile() -> CompanyProfile {
    CompanyProfile {
        company_name: "Sorelon".to_owned(),
        registration_number: "12345678".to_owned(),
        vat_number: None,
        address: CompanyAddress {
            line1: "1 Test Street".to_owned(),
            line2: None,
            city: "London".to_owned(),
            county: None,
            postcode: "SW1A 1AA".to_owned(),
            country: "GB".to_owned(),
        },
        primary_contact: CompanyContact {
            name: "Operator".to_owned(),
            email: "operator@example.invalid".to_owned(),
            phone: "0000000000".to_owned(),
            role: "Procurement".to_owned(),
        },
        certifications: Vec::new(),
        insurance_policies: Vec::new(),
        financial_statements: None,
        sme_status: None,
        equality_statement: None,
        environmental_policy: None,
    }
}

fn valid_document() -> BidDocument {
    BidDocument {
        document_type: "pricing_schedule".to_owned(),
        file_name: "pricing.pdf".to_owned(),
        file_path: "artifacts/pricing.pdf".to_owned(),
        content_hash: "sha256:pricing".to_owned(),
        expiry_date: None,
        issuing_body: None,
    }
}

fn valid_params(process_id: &str) -> TriggerBidWorkflowParams {
    TriggerBidWorkflowParams {
        process_id: process_id.to_owned(),
        bid_scope: valid_scope(process_id),
        operator_id: "operator-alinaaitova".to_owned(),
        company_profile: valid_company_profile(),
        bid_documents: vec![valid_document()],
        stage_run_id: "stage-run-bid-001".to_owned(),
    }
}

fn readiness_blocked_params(process_id: &str) -> TriggerBidWorkflowParams {
    let mut params = valid_params(process_id);
    "READINESS-BLOCKED".clone_into(&mut params.bid_scope.portal_opportunity_ref);
    params
}

fn open_state_db(test_db: &TestStateDb) -> Result<StateDb> {
    StateDb::open(test_db.db_path()).context("opening isolated state db")
}

fn workflow_job_count(state_db: &StateDb) -> Result<i64> {
    state_db
        .connection()
        .query_row("SELECT COUNT(*) FROM workflow_jobs", [], |row| row.get(0))
        .context("counting workflow jobs")
}

fn assert_status_artifact_contains(snippet: &str) {
    assert!(
        AUTONOMOUS_SCHEDULING_STATUS.contains(snippet),
        "autonomous scheduling status artifact missing decision snippet:\n{snippet}"
    );
}

#[test]
fn decision_001_operator_decisions_recorded() {
    assert_eq!(
        AUTONOMOUS_SCHEDULING_STATUS
            .matches("status: RECORDED")
            .count(),
        8
    );

    assert_status_artifact_contains(
        r"  Q1_worker_exec_context:
    status: RECORDED
    chosen_branch: own_state_db_connection_per_lane",
    );
    assert_status_artifact_contains(
        r#"    operator_answer: "Q1 = own `StateDb` connection per lane.""#,
    );
    assert_status_artifact_contains(
        r"  Q4_awaiting_access_model:
    status: RECORDED
    chosen_branch: dedicated_table_epoch_11",
    );
    assert_status_artifact_contains(
        r#"    operator_answer: "Q4 = dedicated table (epoch 10->11).""#,
    );
    assert_status_artifact_contains(
        r"  Q5_awaiting_access_expiry:
    status: RECORDED
    chosen_branch: notice_deadline_else_max_age",
    );
    assert_status_artifact_contains(
        r#"    operator_answer: "Q5 = expire on notice `deadline_at` if known, else max-age since interest (NEVER the RFx response deadline - unknown for awaiting-access items).""#,
    );
    assert_status_artifact_contains(
        r"  Q6_shadow_flag:
    status: RECORDED
    chosen_branch: shadow_flag_first",
    );
    assert_status_artifact_contains(
        r#"    operator_answer: "Q6 = shadow flag first for `source_health`/`auth_refresh`.""#,
    );
    assert_status_artifact_contains(
        r"  Q7_desktop_status_ux:
    status: RECORDED
    chosen_branch: minimal_read_only_desktop_status_fields",
    );
    assert_status_artifact_contains(
        r#"    operator_answer: "Q7 = minimal read-only desktop status fields.""#,
    );
    assert_status_artifact_contains(
        r"  Q8_residual_chained_nudge:
    status: RECORDED
    chosen_branch: scheduled_sweep_only",
    );
    assert_status_artifact_contains(
        r#"    operator_answer: "Q8-residual = scheduled sweep only.""#,
    );
    assert_status_artifact_contains(
        r"  Q9_registry_reload_trigger:
    status: RECORDED
    chosen_branch: registry_generation_counter",
    );
    assert_status_artifact_contains(r#"    operator_answer: "Q9 = registry generation counter.""#);
    assert_status_artifact_contains(
        r"  E1_trigger_enrichment_routing:
    status: RECORDED
    chosen_branch: named_extension_not_cron_core",
    );
    assert_status_artifact_contains(
        r#"    operator_answer: "E1 = route `enrich_process` only if operator wants the button fixed now.""#,
    );
}

#[test]
fn m4_trigger_ingestion_enqueues_routable_sync_source_job() -> Result<()> {
    let test_db = TestStateDb::new()?;
    let mut state_db = open_state_db(&test_db)?;
    let params = TriggerIngestionParams {
        source_id: FTS_SOURCE_ID.to_owned(),
    };
    let mut context = JobsHandlerContext {
        state_db: &mut state_db,
    };

    let ack = trigger_ingestion(&mut context, &params, "ingestion-trigger-key")
        .context("desktop ingestion trigger should enqueue")?;

    assert!(ack.accepted);
    let continuation_job_id = ack
        .continuation_job_id
        .as_deref()
        .context("continuation job id missing")?;
    ensure!(
        continuation_job_id.starts_with("job-"),
        "continuation job id should use job prefix"
    );
    assert_eq!(workflow_job_count(context.state_db)?, 1);

    let row: (String, String, Option<String>, Option<String>, String) = context
        .state_db
        .connection()
        .query_row(
            "\
            SELECT kind, status, scope_kind, scope_id, input_payload
            FROM workflow_jobs
            WHERE id = ?1
            ",
            [continuation_job_id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            },
        )
        .optional()?
        .context("enqueued ingestion job missing")?;
    assert_eq!(row.0, "sync_source");
    assert_eq!(row.1, "pending");
    assert_eq!(row.2.as_deref(), Some("source"));
    assert_eq!(row.3.as_deref(), Some(FTS_SOURCE_ID));

    let payload: Value = serde_json::from_str(&row.4)?;
    assert_eq!(payload["request"]["source_id"], FTS_SOURCE_ID);
    assert_eq!(payload["request"]["skill_id"], "procurement-ingestion/fts");
    assert_eq!(payload["request"]["trigger"], "desktop_command");
    assert_eq!(
        payload["runner_meta"]["idempotency_key"],
        "ingestion-trigger-key"
    );
    Ok(())
}

#[test]
fn trigger_enrichment_001_preserves_named_extension_branch() -> Result<()> {
    assert!(AUTONOMOUS_SCHEDULING_STATUS.contains("E1_trigger_enrichment_routing:"));
    assert!(AUTONOMOUS_SCHEDULING_STATUS.contains("chosen_branch: named_extension_not_cron_core"));

    let test_db = TestStateDb::new()?;
    let mut state_db = open_state_db(&test_db)?;
    seed_process(state_db.connection(), "process-enrich-001")?;
    let params = TriggerEnrichmentParams {
        process_id: "process-enrich-001".to_owned(),
    };
    let mut context = JobsHandlerContext {
        state_db: &mut state_db,
    };

    let ack = trigger_enrichment(&mut context, &params, "enrichment-trigger-key")
        .context("desktop enrichment trigger should remain a named extension")?;

    assert!(ack.accepted);
    let continuation_job_id = ack
        .continuation_job_id
        .as_deref()
        .context("continuation job id missing")?;
    assert_eq!(workflow_job_count(context.state_db)?, 1);

    let row: (
        String,
        String,
        Option<String>,
        Option<String>,
        Option<String>,
        String,
    ) = context
        .state_db
        .connection()
        .query_row(
            "\
            SELECT kind, status, scope_kind, scope_id, process_id, input_payload
            FROM workflow_jobs
            WHERE id = ?1
            ",
            [continuation_job_id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                ))
            },
        )
        .optional()?
        .context("enqueued enrichment extension job missing")?;
    assert_eq!(row.0, "enrich_process");
    assert_eq!(row.1, "pending");
    assert_eq!(row.2.as_deref(), Some("process"));
    assert_eq!(row.3.as_deref(), Some("process-enrich-001"));
    assert_eq!(row.4.as_deref(), Some("process-enrich-001"));

    let payload: Value = serde_json::from_str(&row.5)?;
    assert_eq!(payload["request"]["process_id"], "process-enrich-001");
    assert_eq!(payload["request"]["trigger"], "desktop_command");
    assert_eq!(
        payload["runner_meta"]["idempotency_key"],
        "enrichment-trigger-key"
    );
    Ok(())
}

#[test]
fn l6_t_ipc_trigger_bid_workflow_001_rejects_invalid_bid_scope_before_enqueue() -> Result<()> {
    let test_db = TestStateDb::new()?;
    let mut state_db = open_state_db(&test_db)?;
    seed_process(state_db.connection(), "process-bid-001")?;
    let mut params = valid_params("process-bid-001");
    params.bid_scope.lot_ids.clear();
    let mut context = JobsHandlerContext {
        state_db: &mut state_db,
    };

    let error = trigger_bid_workflow(&mut context, &params, "bid-trigger-key")
        .expect_err("empty lot_ids must be rejected");

    assert_eq!(error.code, "invalid_params");
    assert!(error.message.contains("lotIds"));
    assert!(!error.retryable);
    assert_eq!(workflow_job_count(context.state_db)?, 0);
    Ok(())
}

#[test]
fn l6_t_ipc_trigger_bid_workflow_002_returns_readiness_blockers_before_enqueue() -> Result<()> {
    let test_db = TestStateDb::new()?;
    let mut state_db = open_state_db(&test_db)?;
    let params = valid_params("missing-process");
    let mut context = JobsHandlerContext {
        state_db: &mut state_db,
    };

    let error = trigger_bid_workflow(&mut context, &params, "bid-trigger-key")
        .expect_err("missing process must block before enqueue");

    assert_eq!(error.code, "conflict");
    assert_eq!(error.message, "bid workflow readiness failed");
    assert!(!error.retryable);
    let details = error.details.context("readiness details missing")?;
    let blockers = details
        .get("blockers")
        .and_then(Value::as_array)
        .context("readiness blockers missing")?;
    assert_eq!(blockers[0]["code"], "PROCESS_NOT_FOUND");
    assert_eq!(blockers[0]["source"], "procurement_processes");
    assert_eq!(blockers[0]["overrideable"], false);
    assert_eq!(workflow_job_count(context.state_db)?, 0);
    Ok(())
}

#[test]
fn l6_t_ipc_trigger_bid_workflow_002_blocks_readiness_before_enqueue() -> Result<()> {
    let test_db = TestStateDb::new()?;
    let mut state_db = open_state_db(&test_db)?;
    seed_process(state_db.connection(), "process-bid-001")?;
    let params = readiness_blocked_params("process-bid-001");
    let mut context = JobsHandlerContext {
        state_db: &mut state_db,
    };

    let error = trigger_bid_workflow(&mut context, &params, "bid-trigger-key")
        .expect_err("readiness blocker must stop enqueue");

    assert_eq!(error.code, "conflict");
    assert_eq!(error.message, "bid workflow readiness failed");
    assert!(!error.retryable);
    let details = error.details.context("readiness details missing")?;
    let blockers = details
        .get("blockers")
        .and_then(Value::as_array)
        .context("readiness blockers missing")?;
    assert_eq!(blockers.len(), 1);
    assert_eq!(blockers[0]["code"], "TEST_READINESS_BLOCKED");
    assert_eq!(blockers[0]["source"], "bid_workflow_readiness_gate");
    assert_eq!(blockers[0]["overrideable"], false);
    assert_eq!(workflow_job_count(context.state_db)?, 0);
    Ok(())
}

#[test]
fn l6_t_ipc_trigger_bid_workflow_003_enqueues_run_bid_workflow_once_when_eligible() -> Result<()> {
    let test_db = TestStateDb::new()?;
    let mut state_db = open_state_db(&test_db)?;
    seed_process(state_db.connection(), "process-bid-001")?;
    let params = valid_params("process-bid-001");
    let mut context = JobsHandlerContext {
        state_db: &mut state_db,
    };

    let ack = trigger_bid_workflow(&mut context, &params, "bid-trigger-key")
        .context("eligible bid workflow trigger should enqueue")?;

    assert!(ack.accepted);
    let continuation_job_id = ack
        .continuation_job_id
        .as_deref()
        .context("continuation job id missing")?;
    ensure!(
        continuation_job_id.starts_with("job-"),
        "continuation job id should use job prefix"
    );
    assert_eq!(workflow_job_count(context.state_db)?, 1);

    let row: (String, Option<String>, String) = context
        .state_db
        .connection()
        .query_row(
            "\
            SELECT kind, process_id, input_payload
            FROM workflow_jobs
            WHERE id = ?1
            ",
            [continuation_job_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()?
        .context("enqueued bid workflow job missing")?;
    assert_eq!(row.0, "run_bid_workflow");
    assert_eq!(row.1.as_deref(), Some("process-bid-001"));

    let payload: Value = serde_json::from_str(&row.2)?;
    assert_eq!(
        payload["request"]["bid_scope_payload"]["portalOpportunityRef"],
        "DN123456"
    );
    assert_eq!(
        payload["request"]["company_profile"]["registrationNumber"],
        "12345678"
    );
    assert_eq!(
        payload["request"]["bid_documents"][0]["documentType"],
        "pricing_schedule"
    );
    assert_eq!(payload["runner_meta"]["idempotency_key"], "bid-trigger-key");
    Ok(())
}
