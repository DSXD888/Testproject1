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
    JobsHandlerContext, SideEffectClass, TriggerBidWorkflowParams, trigger_bid_workflow,
};
use crate::support::db::{StateDb, TestStateDb, seed_process};

const BASIS_HASH: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

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
