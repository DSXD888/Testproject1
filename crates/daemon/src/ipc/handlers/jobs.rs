#![forbid(unsafe_code)]

use anyhow::{Context, Result};
use rusqlite::{OptionalExtension, params};
use serde::Deserialize;
use serde_json::{Value, json};
use thiserror::Error;
use uuid::Uuid;

#[cfg(not(test))]
use crate::bid_workflow::{
    BidDocument, BidReadinessBlocker, BidScopePayload, CompanyProfile, evaluate_bid_readiness,
};
use crate::db::sqlite::StateDb;
use crate::jobs::runner::{
    EnqueueJobRequest, JobRunner, MutationAck, enqueue_bid_workflow_request,
};
#[cfg(test)]
#[allow(unused_imports)]
pub use test_bid_workflow::{
    BidDocument, BidReadinessBlocker, BidScopePayload, CompanyAddress, CompanyContact,
    CompanyProfile, SideEffectClass, evaluate_bid_readiness,
};

const EXECUTOR_ID: &str = "local-daemon";

#[cfg(test)]
mod test_bid_workflow {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct BidScopePayload {
        pub process_id: String,
        pub lot_ids: Vec<String>,
        pub candidate_ids: Vec<String>,
        pub selected_quote_ids: Vec<String>,
        pub company_profile_version: String,
        pub bid_portal_type: String,
        pub portal_opportunity_ref: String,
        pub review_required: bool,
        pub decision_basis_hash: String,
        pub side_effect_cap: SideEffectClass,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    pub enum SideEffectClass {
        ReadOnly,
        RemoteDraft,
        NotificationOrLock,
        LegalDeclaration,
        FormalSubmission,
        FundsCommitted,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct BidDocument {
        pub document_type: String,
        pub file_name: String,
        pub file_path: String,
        pub content_hash: String,
        pub expiry_date: Option<String>,
        pub issuing_body: Option<String>,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct CompanyProfile {
        pub company_name: String,
        pub registration_number: String,
        pub vat_number: Option<String>,
        pub address: CompanyAddress,
        pub primary_contact: CompanyContact,
        pub certifications: Vec<CompanyCertification>,
        pub insurance_policies: Vec<CompanyInsurancePolicy>,
        pub financial_statements: Option<CompanyFinancialStatements>,
        pub sme_status: Option<SmeStatus>,
        pub equality_statement: Option<String>,
        pub environmental_policy: Option<String>,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct CompanyAddress {
        pub line1: String,
        pub line2: Option<String>,
        pub city: String,
        pub county: Option<String>,
        pub postcode: String,
        pub country: String,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct CompanyContact {
        pub name: String,
        pub email: String,
        pub phone: String,
        pub role: String,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct CompanyCertification {
        #[serde(rename = "type")]
        pub certification_type: String,
        pub number: String,
        pub expiry_date: String,
        pub issuing_body: String,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct CompanyInsurancePolicy {
        #[serde(rename = "type")]
        pub policy_type: String,
        pub provider: String,
        pub policy_number: String,
        pub cover_amount_pence: i64,
        pub expiry_date: String,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct CompanyFinancialStatements {
        pub last_filed_year: i32,
        pub companies_house_ref: String,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(rename_all = "snake_case")]
    pub enum SmeStatus {
        Micro,
        Small,
        Medium,
        Large,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(rename_all = "snake_case")]
    pub enum BidReadinessBlocker {
        Phase3Pending,
        TestBlocked,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct BidReadinessGateResult {
        pub approved: bool,
        pub blockers: Vec<BidReadinessBlocker>,
        pub override_required: Option<String>,
    }

    pub fn evaluate_bid_readiness(scope: &BidScopePayload) -> BidReadinessGateResult {
        if scope.portal_opportunity_ref == "READINESS-BLOCKED" {
            return BidReadinessGateResult {
                approved: false,
                blockers: vec![BidReadinessBlocker::TestBlocked],
                override_required: None,
            };
        }

        BidReadinessGateResult {
            approved: true,
            blockers: Vec::new(),
            override_required: None,
        }
    }
}

pub struct JobsHandlerContext<'a> {
    pub state_db: &'a mut StateDb,
}

#[derive(Debug, Clone, Error)]
#[error("{code}: {message}")]
pub struct JobsHandlerError {
    pub code: String,
    pub message: String,
    pub retryable: bool,
    pub details: Option<Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TriggerIngestionParams {
    pub source_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TriggerEnrichmentParams {
    pub process_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TriggerBidWorkflowParams {
    pub process_id: String,
    pub bid_scope: BidScopePayload,
    pub operator_id: String,
    pub company_profile: CompanyProfile,
    pub bid_documents: Vec<BidDocument>,
    pub stage_run_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RetryJobParams {
    pub job_id: String,
}

pub fn trigger_ingestion(
    context: &mut JobsHandlerContext<'_>,
    params: &TriggerIngestionParams,
    idempotency_key: &str,
) -> std::result::Result<MutationAck, JobsHandlerError> {
    let job_id = job_id("job");
    let source_kind =
        source_kind(context.state_db, &params.source_id).map_err(map_internal_error)?;
    let source_kind = source_kind.ok_or_else(|| JobsHandlerError {
        code: "not_found".to_owned(),
        message: format!("unknown source `{}`", params.source_id),
        retryable: false,
        details: None,
    })?;
    if source_kind != "procurement" {
        return Err(JobsHandlerError {
            code: "conflict".to_owned(),
            message: format!("source `{}` is not ingestible", params.source_id),
            retryable: false,
            details: None,
        });
    }

    let ack = JobRunner::new(EXECUTOR_ID)
        .enqueue_job(
            context.state_db,
            &EnqueueJobRequest {
                job_id: job_id.clone(),
                kind: "sync_source".to_owned(),
                process_id: None,
                notice_id: None,
                scope_kind: Some("source".to_owned()),
                scope_id: Some(params.source_id.clone()),
                parent_job_id: None,
                continuation_chain_id: None,
                validation_required: false,
                scheduled_for: None,
                input_payload: json!({
                    "source_id": params.source_id,
                    "trigger": "desktop_command",
                }),
                idempotency_key: idempotency_key.to_owned(),
            },
        )
        .map_err(map_internal_error)?;

    Ok(MutationAck {
        continuation_job_id: Some(job_id),
        ..ack
    })
}

pub fn trigger_enrichment(
    context: &mut JobsHandlerContext<'_>,
    params: &TriggerEnrichmentParams,
    idempotency_key: &str,
) -> std::result::Result<MutationAck, JobsHandlerError> {
    let job_id = job_id("job");
    if !process_exists(context.state_db, &params.process_id).map_err(map_internal_error)? {
        return Err(JobsHandlerError {
            code: "not_found".to_owned(),
            message: format!("unknown process `{}`", params.process_id),
            retryable: false,
            details: None,
        });
    }

    let ack = JobRunner::new(EXECUTOR_ID)
        .enqueue_job(
            context.state_db,
            &EnqueueJobRequest {
                job_id: job_id.clone(),
                kind: "enrich_process".to_owned(),
                process_id: Some(params.process_id.clone()),
                notice_id: None,
                scope_kind: Some("process".to_owned()),
                scope_id: Some(params.process_id.clone()),
                parent_job_id: None,
                continuation_chain_id: None,
                validation_required: false,
                scheduled_for: None,
                input_payload: json!({
                    "process_id": params.process_id,
                    "trigger": "desktop_command",
                }),
                idempotency_key: idempotency_key.to_owned(),
            },
        )
        .map_err(map_internal_error)?;

    Ok(MutationAck {
        continuation_job_id: Some(job_id),
        ..ack
    })
}

pub fn trigger_bid_workflow(
    context: &mut JobsHandlerContext<'_>,
    params: &TriggerBidWorkflowParams,
    idempotency_key: &str,
) -> std::result::Result<MutationAck, JobsHandlerError> {
    validate_bid_workflow_request(params)?;

    let blockers = bid_workflow_readiness_blockers(context.state_db, params)?;
    if !blockers.is_empty() {
        return Err(bid_readiness_failed(&blockers));
    }

    let mut enqueue_request = enqueue_bid_workflow_request(
        &params.bid_scope,
        &params.operator_id,
        &params.company_profile,
        &params.bid_documents,
        &params.stage_run_id,
    )
    .map_err(map_internal_error)?;
    idempotency_key.clone_into(&mut enqueue_request.idempotency_key);
    let job_id = enqueue_request.job_id.clone();

    let ack = JobRunner::new(EXECUTOR_ID)
        .enqueue_job(context.state_db, &enqueue_request)
        .map_err(map_internal_error)?;

    Ok(MutationAck {
        continuation_job_id: Some(job_id),
        ..ack
    })
}

pub fn retry_job(
    context: &mut JobsHandlerContext<'_>,
    params: &RetryJobParams,
    idempotency_key: &str,
) -> std::result::Result<MutationAck, JobsHandlerError> {
    let new_job_id = job_id("job");
    let row =
        match load_retryable_job(context.state_db, &params.job_id).map_err(map_internal_error)? {
            RetryableJobLookup::Retryable(row) => row,
            RetryableJobLookup::Missing => {
                return Err(JobsHandlerError {
                    code: "not_found".to_owned(),
                    message: format!("unknown job `{}`", params.job_id),
                    retryable: false,
                    details: None,
                });
            }
            RetryableJobLookup::NotRetryable => {
                return Err(JobsHandlerError {
                    code: "conflict".to_owned(),
                    message: format!("job `{}` is not retryable", params.job_id),
                    retryable: false,
                    details: None,
                });
            }
        };

    let ack = JobRunner::new(EXECUTOR_ID)
        .enqueue_job(
            context.state_db,
            &EnqueueJobRequest {
                job_id: new_job_id.clone(),
                kind: row.kind,
                process_id: row.process_id,
                notice_id: row.notice_id,
                scope_kind: row.scope_kind,
                scope_id: row.scope_id,
                parent_job_id: Some(params.job_id.clone()),
                continuation_chain_id: row.continuation_chain_id,
                validation_required: row.validation_required,
                scheduled_for: None,
                input_payload: row.input_payload,
                idempotency_key: idempotency_key.to_owned(),
            },
        )
        .map_err(map_internal_error)?;

    Ok(MutationAck {
        continuation_job_id: Some(new_job_id),
        ..ack
    })
}

fn validate_bid_workflow_request(
    params: &TriggerBidWorkflowParams,
) -> std::result::Result<(), JobsHandlerError> {
    require_non_empty("processId", &params.process_id)?;
    require_non_empty("bidScope.processId", &params.bid_scope.process_id)?;
    if params.process_id.trim() != params.bid_scope.process_id.trim() {
        return Err(invalid_request(
            "bidScope.processId must match top-level processId",
        ));
    }

    require_non_empty_vec("lotIds", &params.bid_scope.lot_ids)?;
    require_non_empty_vec("candidateIds", &params.bid_scope.candidate_ids)?;
    require_non_empty_vec("selectedQuoteIds", &params.bid_scope.selected_quote_ids)?;
    require_non_empty(
        "companyProfileVersion",
        &params.bid_scope.company_profile_version,
    )?;
    require_non_empty("bidPortalType", &params.bid_scope.bid_portal_type)?;
    require_non_empty(
        "portalOpportunityRef",
        &params.bid_scope.portal_opportunity_ref,
    )?;
    require_decision_basis_hash(&params.bid_scope.decision_basis_hash)?;
    require_non_empty("operatorId", &params.operator_id)?;
    require_non_empty("stageRunId", &params.stage_run_id)?;
    Ok(())
}

fn require_non_empty(field: &str, value: &str) -> std::result::Result<(), JobsHandlerError> {
    if value.trim().is_empty() {
        return Err(invalid_request(format!("{field} must not be empty")));
    }
    Ok(())
}

fn require_non_empty_vec(
    field: &str,
    values: &[String],
) -> std::result::Result<(), JobsHandlerError> {
    if values.is_empty() || values.iter().any(|value| value.trim().is_empty()) {
        return Err(invalid_request(format!("{field} must not be empty")));
    }
    Ok(())
}

fn require_decision_basis_hash(value: &str) -> std::result::Result<(), JobsHandlerError> {
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(invalid_request(
            "decisionBasisHash must be a 64-character hex string",
        ));
    }
    Ok(())
}

fn invalid_request(message: impl Into<String>) -> JobsHandlerError {
    JobsHandlerError {
        code: "invalid_params".to_owned(),
        message: message.into(),
        retryable: false,
        details: None,
    }
}

fn bid_workflow_readiness_blockers(
    state_db: &StateDb,
    params: &TriggerBidWorkflowParams,
) -> std::result::Result<Vec<Value>, JobsHandlerError> {
    let mut blockers = Vec::new();

    if !process_exists(state_db, &params.process_id).map_err(map_internal_error)? {
        blockers.push(json!({
            "code": "PROCESS_NOT_FOUND",
            "source": "procurement_processes",
            "overrideable": false,
        }));
    }

    let readiness = evaluate_bid_readiness(&params.bid_scope);
    for blocker in &readiness.blockers {
        if let Some(blocker) = bid_readiness_blocker_payload(blocker) {
            blockers.push(blocker);
        }
    }

    Ok(blockers)
}

#[cfg(not(test))]
const fn bid_readiness_blocker_payload(blocker: &BidReadinessBlocker) -> Option<Value> {
    match blocker {
        // Phase 3 replaces this scaffold blocker with the real L5/operator gate
        // evidence. Phase 2 still needs an enqueueable smoke path.
        BidReadinessBlocker::Phase3Pending => None,
    }
}

#[cfg(test)]
fn bid_readiness_blocker_payload(blocker: &BidReadinessBlocker) -> Option<Value> {
    match blocker {
        BidReadinessBlocker::Phase3Pending => None,
        BidReadinessBlocker::TestBlocked => Some(json!({
            "code": "TEST_READINESS_BLOCKED",
            "source": "bid_workflow_readiness_gate",
            "overrideable": false,
        })),
    }
}

fn bid_readiness_failed(blockers: &[Value]) -> JobsHandlerError {
    JobsHandlerError {
        code: "conflict".to_owned(),
        message: "bid workflow readiness failed".to_owned(),
        retryable: false,
        details: Some(json!({ "blockers": blockers })),
    }
}

fn source_kind(state_db: &StateDb, source_id: &str) -> Result<Option<String>> {
    state_db
        .connection()
        .query_row(
            "SELECT kind FROM sources WHERE id = ?1 AND enabled = 1",
            params![source_id],
            |row| row.get(0),
        )
        .optional()
        .context("loading source kind")
}

fn process_exists(state_db: &StateDb, process_id: &str) -> Result<bool> {
    let count: i64 = state_db.connection().query_row(
        "SELECT COUNT(*) FROM procurement_processes WHERE id = ?1",
        params![process_id],
        |row| row.get(0),
    )?;
    Ok(count == 1)
}

struct RetryableJobRow {
    kind: String,
    process_id: Option<String>,
    notice_id: Option<String>,
    scope_kind: Option<String>,
    scope_id: Option<String>,
    continuation_chain_id: Option<String>,
    validation_required: bool,
    input_payload: Value,
}

enum RetryableJobLookup {
    Missing,
    NotRetryable,
    Retryable(RetryableJobRow),
}

fn load_retryable_job(state_db: &StateDb, job_id: &str) -> Result<RetryableJobLookup> {
    let row = state_db
        .connection()
        .query_row(
            "\
            SELECT kind, process_id, notice_id, scope_kind, scope_id, continuation_chain_id,
                   validation_required, input_payload, status
            FROM workflow_jobs
            WHERE id = ?1
            ",
            params![job_id],
            |row| {
                let raw_payload: String = row.get(7)?;
                let input_payload =
                    serde_json::from_str::<Value>(&raw_payload).map_err(|error| {
                        rusqlite::Error::FromSqlConversionFailure(
                            raw_payload.len(),
                            rusqlite::types::Type::Text,
                            Box::new(error),
                        )
                    })?;
                let job_row = RetryableJobRow {
                    kind: row.get(0)?,
                    process_id: row.get(1)?,
                    notice_id: row.get(2)?,
                    scope_kind: row.get(3)?,
                    scope_id: row.get(4)?,
                    continuation_chain_id: row.get(5)?,
                    validation_required: row.get::<_, i64>(6)? != 0,
                    input_payload,
                };
                let status: String = row.get(8)?;
                Ok((job_row, status))
            },
        )
        .optional()
        .context("loading retryable job")?;

    Ok(match row {
        None => RetryableJobLookup::Missing,
        Some((row, status)) if matches!(status.as_str(), "failed" | "obsolete" | "retry") => {
            RetryableJobLookup::Retryable(row)
        }
        Some(_) => RetryableJobLookup::NotRetryable,
    })
}

fn job_id(prefix: &str) -> String {
    format!("{prefix}-{}", Uuid::new_v4())
}

#[allow(clippy::needless_pass_by_value)]
fn map_internal_error(error: anyhow::Error) -> JobsHandlerError {
    JobsHandlerError {
        code: "busy".to_owned(),
        message: error.to_string(),
        retryable: true,
        details: None,
    }
}
