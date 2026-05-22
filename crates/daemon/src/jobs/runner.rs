#![forbid(unsafe_code)]

#[cfg(not(test))]
use crate::validation::executor as validation_executor;
#[cfg(test)]
#[allow(clippy::duplicate_mod)]
#[path = "../validation/executor.rs"]
mod validation_executor;

#[allow(clippy::duplicate_mod)]
#[cfg(test)]
#[path = "claim.rs"]
mod claim;
#[cfg(not(test))]
use crate::jobs::claim;
#[cfg(not(test))]
use crate::pty::health::{
    HostResources, detect_host_sleep, detect_resource_pressure, resolve_health_issue,
};

use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result, anyhow, ensure};
use chrono::{DateTime, Utc};
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use uuid::Uuid;

#[cfg(not(test))]
use crate::agents::contracts;
#[cfg(test)]
use crate::contracts_impl as contracts;
#[cfg(not(test))]
use crate::db::sqlite::StateDb;
#[cfg(test)]
use crate::support::db::StateDb;
#[cfg(test)]
#[allow(clippy::duplicate_mod)]
#[path = "../pty/health.rs"]
mod health;
#[cfg(test)]
use health::{HostResources, detect_host_sleep, detect_resource_pressure, resolve_health_issue};

use contracts::{
    StageValidationWrite, ValidationDisposition, ValidationOutcome, persist_stage_validation,
};

pub const SQLITE_WRITE_FAILED: &str = "SQLITE_WRITE_FAILED";
pub const JOB_OBSOLETE: &str = "JOB_OBSOLETE";
const DEFAULT_EXECUTOR_OWNER_ID: &str = "local-owner";

pub use claim::JobLeaseHeartbeat;
#[allow(unused_imports)]
pub use validation_executor::{
    EnqueuedValidatorJob, PersistValidationVerdictRequest, PersistedValidationVerdict,
    ProcessValidationState, RetroactiveBacklogItem, RetroactiveBacklogMetadata,
    VALIDATION_SKIPPED_VALIDATOR_ERROR, VALIDATOR_JOB_KIND, ValidationBlackoutIncident,
    ValidationCanaryCase, ValidationHandlingMode, ValidationHealthState,
    ValidationTelemetrySnapshot, ValidationTelemetryWindow, ValidationTrustState,
    ValidatorBasisSufficiency, ValidatorEnqueueRequest, ValidatorJobPurpose, ValidatorSkillBundle,
    ValidatorVerdict, decide_validator_failure, decide_validator_verdict,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SkillTelemetryWindow {
    pub successful_runs: u32,
    pub total_runs: u32,
    pub durations_ms: Vec<u64>,
    pub cost_per_run_pence: Option<u32>,
    pub token_count: Option<u32>,
    pub checkpoint_count: Option<u32>,
    pub retry_count: Option<u32>,
    pub extraction_fill_rate_bps: Option<u32>,
    pub page_fingerprint_delta_bps: Option<u32>,
    pub auth_challenge_rate_bps: Option<u32>,
    pub listing_match_rate_bps: Option<u32>,
    pub validation_rejection_rate_bps: Option<u32>,
    pub drift_score_bps: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkillTelemetrySnapshot {
    pub success_rate: Option<f64>,
    pub median_duration_ms: Option<u64>,
    pub p95_duration_ms: Option<u64>,
    pub cost_per_run_pence: Option<u32>,
    pub token_count: Option<u32>,
    pub checkpoint_count: Option<u32>,
    pub retry_count: Option<u32>,
    pub extraction_fill_rate_bps: Option<u32>,
    pub page_fingerprint_delta_bps: Option<u32>,
    pub auth_challenge_rate_bps: Option<u32>,
    pub listing_match_rate_bps: Option<u32>,
    pub validation_rejection_rate_bps: Option<u32>,
    pub drift_score_bps: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkillTelemetryThresholds {
    pub success_rate_min: f64,
    pub median_duration_baseline_multiplier: u32,
    pub p95_duration_baseline_multiplier: u32,
    pub cost_per_run_baseline_multiplier: u32,
    pub retry_count_average_max: u32,
    pub extraction_fill_rate_min_bps: u32,
    pub page_fingerprint_delta_max_bps: u32,
    pub auth_challenge_rate_max_bps: u32,
    pub listing_match_rate_min_bps: u32,
    pub validation_rejection_rate_max_bps: u32,
    pub drift_score_max_bps: u32,
}

#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostHealthObservation {
    pub issue: Option<String>,
    pub tainted_active_browser_session: bool,
    pub blocks_new_admission: bool,
    pub monotonic_gap_ms: u64,
    pub sleep_threshold_ms: u64,
    pub cpu_pressure: bool,
    pub memory_pressure: bool,
}

pub const CANONICAL_SCOPE_KINDS: [&str; 6] = [
    "process",
    "notice",
    "lot",
    "candidate",
    "candidate_source_pair",
    "candidate_quote_scenario",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StageCommitSemantics {
    Precursor,
    SetReplacement,
    Additive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FreshnessState {
    Fresh,
    SoftStale,
    HardInvalid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StageScopeRule {
    pub scope_kind: Option<&'static str>,
    pub commit_semantics: StageCommitSemantics,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimedJob {
    pub id: String,
    pub kind: String,
    pub process_id: Option<String>,
    pub scope_kind: Option<String>,
    pub scope_id: Option<String>,
    pub input_payload: Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MutationAck {
    pub accepted: bool,
    pub visible_after_seq: i64,
    pub continuation_job_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistSuccessRequest {
    pub job_id: String,
    pub idempotency_key: String,
    pub session_id: String,
    pub output_payload: Value,
    pub stage: String,
    pub scope_kind: String,
    pub scope_id: String,
    pub stage_run_id: String,
    pub process_id: String,
    pub owner_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistEnforcementProvisionalRequest {
    pub job_id: String,
    pub idempotency_key: String,
    pub output_payload: Value,
    pub result_contract_blob_path: Option<PathBuf>,
    pub stage: String,
    pub scope_kind: String,
    pub scope_id: String,
    pub stage_run_id: String,
    pub process_id: String,
    pub owner_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromoteProvisionalRequest {
    pub worker_job_id: String,
    pub idempotency_key: String,
    pub process_id: String,
    pub stage: String,
    pub scope_kind: String,
    pub scope_id: String,
    pub stage_run_id: String,
    pub owner_id: String,
    pub override_verdict: Option<String>,
    pub override_reason: Option<String>,
    pub override_by: Option<String>,
    pub override_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnqueueJobRequest {
    pub job_id: String,
    pub kind: String,
    pub process_id: Option<String>,
    pub notice_id: Option<String>,
    pub scope_kind: Option<String>,
    pub scope_id: Option<String>,
    pub parent_job_id: Option<String>,
    pub continuation_chain_id: Option<String>,
    pub validation_required: bool,
    pub scheduled_for: Option<DateTime<Utc>>,
    pub input_payload: Value,
    pub idempotency_key: String,
}

pub fn enqueue_bid_workflow_request<S, C, D>(
    bid_scope: &S,
    operator_id: &str,
    company_profile: &C,
    bid_documents: &[D],
    stage_run_id: &str,
) -> Result<EnqueueJobRequest>
where
    S: Serialize,
    C: Serialize,
    D: Serialize,
{
    ensure!(
        !operator_id.trim().is_empty(),
        "operator_id must not be empty"
    );
    ensure!(
        !stage_run_id.trim().is_empty(),
        "stage_run_id must not be empty"
    );

    let bid_scope_payload =
        serde_json::to_value(bid_scope).context("serializing bid scope payload")?;
    let process_id = required_payload_string(&bid_scope_payload, "processId", "process_id")?;
    let decision_basis_hash = required_payload_string(
        &bid_scope_payload,
        "decisionBasisHash",
        "decision_basis_hash",
    )?;
    let company_profile =
        serde_json::to_value(company_profile).context("serializing company profile")?;
    let bid_documents = serde_json::to_value(bid_documents).context("serializing bid documents")?;

    Ok(EnqueueJobRequest {
        job_id: format!("job-{}", Uuid::new_v4()),
        kind: "run_bid_workflow".to_owned(),
        process_id: Some(process_id.clone()),
        notice_id: None,
        scope_kind: Some("process".to_owned()),
        scope_id: Some(process_id),
        parent_job_id: None,
        continuation_chain_id: None,
        validation_required: true,
        scheduled_for: None,
        input_payload: json!({
            "bid_scope_payload": bid_scope_payload,
            "company_profile": company_profile,
            "bid_documents": bid_documents,
            "operator_id": operator_id,
            "stage_run_id": stage_run_id,
            "trigger": "bid_workflow_request",
            "admission_priority": "bid_automation",
            "continuation": {
                "parent_job_id": Value::Null,
                "continuation_chain_id": Value::Null,
            },
        }),
        idempotency_key: decision_basis_hash,
    })
}

fn required_payload_string(payload: &Value, camel_key: &str, snake_key: &str) -> Result<String> {
    let value = payload
        .get(camel_key)
        .or_else(|| payload.get(snake_key))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow!("bid scope payload missing `{camel_key}`"))?;
    Ok(value.to_owned())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistContinuationRequest {
    pub job_id: String,
    pub idempotency_key: String,
    pub continuation_job_id: String,
    pub continuation_chain_id: String,
    pub next_job_kind: String,
    pub process_id: Option<String>,
    pub scope_kind: Option<String>,
    pub scope_id: Option<String>,
    pub resume_token: Value,
    pub owner_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObsoleteJobRequest {
    pub job_id: String,
    pub idempotency_key: String,
    pub rerun_job_id: String,
    pub next_job_kind: String,
    pub process_id: Option<String>,
    pub scope_kind: Option<String>,
    pub scope_id: Option<String>,
    pub rerun_from_stage: String,
    pub expected_input_basis_hash: String,
    pub actual_input_basis_hash: String,
}

#[derive(Debug, Clone)]
pub struct JobRunner {
    executor_id: String,
}

impl JobRunner {
    #[must_use]
    pub fn new(executor_id: impl Into<String>) -> Self {
        Self {
            executor_id: executor_id.into(),
        }
    }

    pub fn claim_next_job(&self, conn: &Connection) -> Result<Option<ClaimedJob>> {
        touch_executor(conn, &self.executor_id)?;
        claim::claim_next_job(conn, &self.executor_id).context("claiming next workflow job")
    }

    /// Claim a specific job by id, bypassing FIFO ordering. Intended for
    /// `--ingest` callers that have just enqueued a job and need to run it
    /// immediately regardless of what else is pending in the queue.
    pub fn claim_job_by_id(&self, conn: &Connection, job_id: &str) -> Result<Option<ClaimedJob>> {
        touch_executor(conn, &self.executor_id)?;
        claim::claim_job_by_id(conn, &self.executor_id, job_id)
            .context("claiming workflow job by id")
    }

    pub fn persist_success(
        &self,
        state_db: &mut StateDb,
        request: &PersistSuccessRequest,
        outcome: &ValidationOutcome,
    ) -> Result<MutationAck> {
        if let Some(existing) = find_existing_ack(
            state_db.connection(),
            &request.job_id,
            &request.idempotency_key,
        )? {
            return Ok(existing);
        }

        ensure!(
            outcome.disposition == ValidationDisposition::Accepted,
            "persist_success requires accepted outcome"
        );
        ensure!(
            !request.stage_run_id.is_empty(),
            "stage_run_id must not be empty"
        );
        let stage_rule = validate_stage_scope(&request.stage, &request.scope_kind)?;
        ensure!(
            stage_rule.commit_semantics != StageCommitSemantics::Precursor,
            "ingest precursor stage does not use process_stage_status"
        );

        let (predicted_visible_after_seq, visible_after_seq) =
            state_db.with_write_transaction(|tx| {
                let predicted_visible_after_seq = current_commit_seq(tx)? + 1;
                let stage_write = StageValidationWrite {
                    process_id: &request.process_id,
                    stage: &request.stage,
                    scope_kind: &request.scope_kind,
                    scope_id: &request.scope_id,
                    stage_run_id: &request.stage_run_id,
                    owner_id: &request.owner_id,
                };
                persist_stage_validation(tx, &stage_write, outcome)?;

                let stored_payload = json!({
                    "result": request.output_payload,
                    "runner_meta": {
                        "idempotency_key": request.idempotency_key,
                        "visible_after_seq": predicted_visible_after_seq,
                        "executor_id": self.executor_id,
                        "continuation_job_id": Value::Null,
                    }
                })
                .to_string();

                let updated = tx.execute(
                    "\
                    UPDATE workflow_jobs
                    SET status = 'completed',
                        completed_at = ?1,
                        output_payload = ?2
                    WHERE id = ?3
                    ",
                    params![Utc::now().to_rfc3339(), stored_payload, request.job_id],
                )?;
                ensure!(updated == 1, "workflow job missing for {}", request.job_id);
                Ok(predicted_visible_after_seq)
            })?;

        ensure!(
            predicted_visible_after_seq == visible_after_seq,
            "predicted commit fence diverged from committed fence"
        );

        Ok(MutationAck {
            accepted: true,
            visible_after_seq,
            continuation_job_id: None,
        })
    }

    pub fn persist_enforcement_provisional_and_enqueue_validator(
        &self,
        repo_root: &Path,
        state_db: &mut StateDb,
        provisional_request: &PersistEnforcementProvisionalRequest,
        validator_request: &ValidatorEnqueueRequest,
    ) -> Result<EnqueuedValidatorJob> {
        ensure!(
            provisional_request.job_id == validator_request.worker_job_id,
            "validator request worker_job_id must match provisional worker job"
        );
        self.persist_enforcement_provisional_output(state_db, provisional_request)?;
        self.enqueue_validation_sub_job(repo_root, state_db, validator_request)
    }

    fn persist_enforcement_provisional_output(
        &self,
        state_db: &mut StateDb,
        request: &PersistEnforcementProvisionalRequest,
    ) -> Result<MutationAck> {
        if let Some(existing) = find_existing_ack(
            state_db.connection(),
            &request.job_id,
            &request.idempotency_key,
        )? {
            return Ok(existing);
        }

        ensure!(
            !request.stage_run_id.is_empty(),
            "stage_run_id must not be empty"
        );
        let stage_rule = validate_stage_scope(&request.stage, &request.scope_kind)?;
        ensure!(
            stage_rule.commit_semantics != StageCommitSemantics::Precursor,
            "ingest precursor stage does not use provisional promotion"
        );
        let provisional_id = format!("provisional-{}", request.job_id);
        let payload_json = serde_json::to_string(&request.output_payload)
            .context("serializing provisional worker output payload")?;
        let result_contract_blob_path = request
            .result_contract_blob_path
            .as_ref()
            .map(|path| path.to_string_lossy().into_owned());

        let (predicted_visible_after_seq, visible_after_seq) =
            state_db.with_write_transaction(|tx| {
                let predicted_visible_after_seq = current_commit_seq(tx)? + 1;
                ensure_no_existing_provisional(tx, &request.job_id)?;
                tx.execute(
                    "\
                    INSERT INTO provisional_worker_outputs (
                      id, worker_job_id, payload_json, result_contract_blob_path,
                      validation_status
                    ) VALUES (?1, ?2, ?3, ?4, 'pending')
                    ",
                    params![
                        provisional_id,
                        request.job_id,
                        payload_json,
                        result_contract_blob_path
                    ],
                )
                .context("inserting enforcement provisional worker output")?;

                let stored_payload = json!({
                    "result": request.output_payload,
                    "runner_meta": {
                        "idempotency_key": request.idempotency_key,
                        "visible_after_seq": predicted_visible_after_seq,
                        "executor_id": self.executor_id,
                        "continuation_job_id": Value::Null,
                        "provisional_worker_output_id": provisional_id,
                        "promotion_pending": true,
                    }
                })
                .to_string();

                let updated = tx.execute(
                    "\
                    UPDATE workflow_jobs
                    SET status = 'completed',
                        validation_required = 1,
                        completed_at = ?1,
                        output_payload = ?2
                    WHERE id = ?3
                    ",
                    params![Utc::now().to_rfc3339(), stored_payload, request.job_id],
                )?;
                ensure!(updated == 1, "workflow job missing for {}", request.job_id);
                Ok(predicted_visible_after_seq)
            })?;

        ensure!(
            predicted_visible_after_seq == visible_after_seq,
            "predicted commit fence diverged from committed fence"
        );

        Ok(MutationAck {
            accepted: true,
            visible_after_seq,
            continuation_job_id: None,
        })
    }

    pub fn promote_enforcement_provisional_output(
        &self,
        state_db: &mut StateDb,
        request: &PromoteProvisionalRequest,
    ) -> Result<MutationAck> {
        if let Some(existing) = find_existing_ack(
            state_db.connection(),
            &request.worker_job_id,
            &request.idempotency_key,
        )? {
            return Ok(existing);
        }

        ensure!(
            !request.stage_run_id.is_empty(),
            "stage_run_id must not be empty"
        );
        let stage_rule = validate_stage_scope(&request.stage, &request.scope_kind)?;
        ensure!(
            stage_rule.commit_semantics != StageCommitSemantics::Precursor,
            "ingest precursor stage does not use provisional promotion"
        );
        let schema_epoch =
            i32::try_from(state_db.schema_epoch()).context("schema_epoch exceeds i32")?;

        let (ack, visible_after_seq) = state_db.with_write_transaction(|tx| {
            promote_enforcement_provisional_output_tx(tx, &self.executor_id, schema_epoch, request)
        })?;

        ensure!(
            ack.visible_after_seq == visible_after_seq,
            "predicted commit fence diverged from committed fence"
        );
        Ok(ack)
    }

    pub fn enqueue_job(
        &self,
        state_db: &mut StateDb,
        request: &EnqueueJobRequest,
    ) -> Result<MutationAck> {
        if let Some(existing) = find_existing_enqueue_ack(state_db.connection(), request)? {
            return Ok(existing);
        }

        let (predicted_visible_after_seq, visible_after_seq) =
            state_db.with_write_transaction(|tx| {
                let predicted_visible_after_seq = current_commit_seq(tx)? + 1;
                let stored_input_payload = json!({
                    "request": request.input_payload,
                    "runner_meta": {
                        "idempotency_key": request.idempotency_key,
                        "visible_after_seq": predicted_visible_after_seq,
                        "executor_id": self.executor_id,
                    }
                })
                .to_string();

                tx.execute(
                    "\
                    INSERT INTO workflow_jobs (
                      id, kind, process_id, notice_id, scope_kind, scope_id, parent_job_id,
                      continuation_chain_id, validation_required, status, scheduled_for,
                      input_payload
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'pending', ?10, ?11)
                    ",
                    params![
                        request.job_id,
                        request.kind,
                        request.process_id,
                        request.notice_id,
                        request.scope_kind,
                        request.scope_id,
                        request.parent_job_id,
                        request.continuation_chain_id,
                        i64::from(request.validation_required),
                        request
                            .scheduled_for
                            .map(|scheduled_for| scheduled_for.to_rfc3339()),
                        stored_input_payload,
                    ],
                )?;

                Ok(predicted_visible_after_seq)
            })?;

        ensure!(
            predicted_visible_after_seq == visible_after_seq,
            "predicted commit fence diverged from committed fence"
        );

        Ok(MutationAck {
            accepted: true,
            visible_after_seq,
            continuation_job_id: None,
        })
    }

    pub fn enqueue_validation_sub_job(
        &self,
        repo_root: &Path,
        state_db: &mut StateDb,
        request: &ValidatorEnqueueRequest,
    ) -> Result<EnqueuedValidatorJob> {
        validation_executor::enqueue_validator_sub_job(repo_root, state_db, self, request)
    }

    pub fn persist_validation_verdict(
        &self,
        state_db: &mut StateDb,
        request: &PersistValidationVerdictRequest,
    ) -> Result<PersistedValidationVerdict> {
        let _ = &self.executor_id;
        validation_executor::persist_validation_verdict(state_db, request)
    }

    pub fn enqueue_retroactive_backlog_job(
        &self,
        repo_root: &Path,
        state_db: &mut StateDb,
        request: ValidatorEnqueueRequest,
        backlog: RetroactiveBacklogMetadata,
    ) -> Result<EnqueuedValidatorJob> {
        validation_executor::enqueue_retroactive_backlog_job(
            repo_root, state_db, self, request, backlog,
        )
    }

    pub fn enqueue_validation_canary_job(
        &self,
        repo_root: &Path,
        state_db: &mut StateDb,
        request: ValidatorEnqueueRequest,
        canary_case: ValidationCanaryCase,
    ) -> Result<EnqueuedValidatorJob> {
        validation_executor::enqueue_validation_canary_job(
            repo_root,
            state_db,
            self,
            request,
            canary_case,
        )
    }

    pub fn persist_continuation(
        &self,
        state_db: &mut StateDb,
        request: &PersistContinuationRequest,
    ) -> Result<MutationAck> {
        if let Some(existing) = find_existing_ack(
            state_db.connection(),
            &request.job_id,
            &request.idempotency_key,
        )? {
            return Ok(existing);
        }

        let (predicted_visible_after_seq, visible_after_seq) =
            state_db.with_write_transaction(|tx| {
                let predicted_visible_after_seq = current_commit_seq(tx)? + 1;

                tx.execute(
                    "\
                    INSERT INTO workflow_jobs (
                      id, kind, process_id, scope_kind, scope_id, parent_job_id,
                      continuation_chain_id, status, input_payload
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'pending', ?8)
                    ",
                    params![
                        request.continuation_job_id,
                        request.next_job_kind,
                        request.process_id,
                        request.scope_kind,
                        request.scope_id,
                        request.job_id,
                        request.continuation_chain_id,
                        json!({"resume_token": request.resume_token}).to_string(),
                    ],
                )?;

                let stored_payload = json!({
                    "runner_meta": {
                        "idempotency_key": request.idempotency_key,
                        "visible_after_seq": predicted_visible_after_seq,
                        "executor_id": self.executor_id,
                        "continuation_job_id": request.continuation_job_id,
                    }
                })
                .to_string();

                let updated = tx.execute(
                    "\
                    UPDATE workflow_jobs
                    SET status = 'completed',
                        chain_closed = 0,
                        completed_at = ?1,
                        output_payload = ?2
                    WHERE id = ?3
                    ",
                    params![Utc::now().to_rfc3339(), stored_payload, request.job_id],
                )?;
                ensure!(updated == 1, "workflow job missing for {}", request.job_id);

                Ok(predicted_visible_after_seq)
            })?;

        ensure!(
            predicted_visible_after_seq == visible_after_seq,
            "predicted commit fence diverged from committed fence"
        );

        Ok(MutationAck {
            accepted: true,
            visible_after_seq,
            continuation_job_id: Some(request.continuation_job_id.clone()),
        })
    }

    pub fn mark_obsolete_and_enqueue_rerun(
        &self,
        state_db: &mut StateDb,
        request: &ObsoleteJobRequest,
    ) -> Result<MutationAck> {
        if let Some(existing) = find_existing_ack(
            state_db.connection(),
            &request.job_id,
            &request.idempotency_key,
        )? {
            return Ok(existing);
        }

        ensure!(
            request.expected_input_basis_hash != request.actual_input_basis_hash,
            "basis hash mismatch handler requires differing hashes"
        );

        let (predicted_visible_after_seq, visible_after_seq) =
            state_db.with_write_transaction(|tx| {
                let predicted_visible_after_seq = current_commit_seq(tx)? + 1;
                tx.execute(
                    "\
                    INSERT INTO workflow_jobs (
                      id, kind, process_id, scope_kind, scope_id, parent_job_id,
                      status, input_payload
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'pending', ?7)
                    ",
                    params![
                        request.rerun_job_id,
                        request.next_job_kind,
                        request.process_id,
                        request.scope_kind,
                        request.scope_id,
                        request.job_id,
                        json!({
                            "rerun_from_stage": request.rerun_from_stage,
                            "expected_input_basis_hash": request.expected_input_basis_hash,
                        })
                        .to_string(),
                    ],
                )?;

                let updated = tx.execute(
                    "\
                    UPDATE workflow_jobs
                    SET status = 'obsolete',
                        last_error = ?1,
                        completed_at = ?2,
                        output_payload = ?3
                    WHERE id = ?4
                    ",
                    params![
                        format!(
                            "{}: expected {} got {}",
                            JOB_OBSOLETE,
                            request.expected_input_basis_hash,
                            request.actual_input_basis_hash
                        ),
                        Utc::now().to_rfc3339(),
                        json!({
                            "runner_meta": {
                                "idempotency_key": request.idempotency_key,
                                "visible_after_seq": predicted_visible_after_seq,
                                "executor_id": self.executor_id,
                                "continuation_job_id": request.rerun_job_id,
                            }
                        })
                        .to_string(),
                        request.job_id,
                    ],
                )?;
                ensure!(updated == 1, "workflow job missing for {}", request.job_id);
                Ok(predicted_visible_after_seq)
            })?;

        ensure!(
            predicted_visible_after_seq == visible_after_seq,
            "predicted commit fence diverged from committed fence"
        );

        Ok(MutationAck {
            accepted: true,
            visible_after_seq,
            continuation_job_id: Some(request.rerun_job_id.clone()),
        })
    }
}

#[must_use]
pub fn start_job_lease_heartbeat(
    state_db_path: PathBuf,
    job_id: impl Into<String>,
    lease_owner: impl Into<String>,
) -> JobLeaseHeartbeat {
    JobLeaseHeartbeat::start(state_db_path, job_id.into(), lease_owner.into())
}

fn touch_executor(conn: &Connection, executor_id: &str) -> Result<()> {
    conn.execute(
        "\
        INSERT INTO executors (
          id, device_name, status, last_heartbeat_at, owner_id, updated_at
        ) VALUES (?1, ?2, 'ready', CURRENT_TIMESTAMP, ?3, CURRENT_TIMESTAMP)
        ON CONFLICT(id) DO UPDATE SET
          device_name = excluded.device_name,
          status = excluded.status,
          owner_id = excluded.owner_id,
          last_heartbeat_at = CURRENT_TIMESTAMP,
          updated_at = CURRENT_TIMESTAMP
        ",
        params![executor_id, executor_id, DEFAULT_EXECUTOR_OWNER_ID],
    )
    .context("touching executor identity")?;
    Ok(())
}

pub fn classify_sqlite_write_failed(error: &rusqlite::Error) -> anyhow::Error {
    anyhow!("{SQLITE_WRITE_FAILED}: {error}")
}

#[must_use]
pub fn compute_skill_telemetry(window: &SkillTelemetryWindow) -> SkillTelemetrySnapshot {
    SkillTelemetrySnapshot {
        success_rate: if window.total_runs == 0 {
            None
        } else {
            Some(f64::from(window.successful_runs) / f64::from(window.total_runs))
        },
        median_duration_ms: percentile_duration_ms(&window.durations_ms, 1, 2),
        p95_duration_ms: percentile_duration_ms(&window.durations_ms, 95, 100),
        cost_per_run_pence: window.cost_per_run_pence,
        token_count: window.token_count,
        checkpoint_count: window.checkpoint_count,
        retry_count: window.retry_count,
        extraction_fill_rate_bps: window.extraction_fill_rate_bps,
        page_fingerprint_delta_bps: window.page_fingerprint_delta_bps,
        auth_challenge_rate_bps: window.auth_challenge_rate_bps,
        listing_match_rate_bps: window.listing_match_rate_bps,
        validation_rejection_rate_bps: window.validation_rejection_rate_bps,
        drift_score_bps: window.drift_score_bps,
    }
}

#[must_use]
pub const fn skill_telemetry_thresholds() -> SkillTelemetryThresholds {
    SkillTelemetryThresholds {
        success_rate_min: 0.90,
        median_duration_baseline_multiplier: 2,
        p95_duration_baseline_multiplier: 3,
        cost_per_run_baseline_multiplier: 2,
        retry_count_average_max: 2,
        extraction_fill_rate_min_bps: 7_000,
        page_fingerprint_delta_max_bps: 2_000,
        auth_challenge_rate_max_bps: 1_000,
        listing_match_rate_min_bps: 5_000,
        validation_rejection_rate_max_bps: 2_000,
        drift_score_max_bps: 3_000,
    }
}

#[must_use]
pub fn evaluate_host_health(
    monotonic_gap: Duration,
    sleep_threshold: Duration,
    cpu_pressure: bool,
    memory_pressure: bool,
    browser_driven_pty_session: bool,
) -> HostHealthObservation {
    let resources = HostResources {
        cpu_pressure,
        memory_pressure,
    };
    let sleep_issue = detect_host_sleep(monotonic_gap, sleep_threshold);
    let resource_issue = detect_resource_pressure(resources);
    let issue = resolve_health_issue(None, None, sleep_issue, resource_issue);

    HostHealthObservation {
        issue: issue.map(|value| value.code().to_owned()),
        tainted_active_browser_session: sleep_issue.is_some() && browser_driven_pty_session,
        blocks_new_admission: resource_issue.is_some(),
        monotonic_gap_ms: u64::try_from(monotonic_gap.as_millis()).unwrap_or(u64::MAX),
        sleep_threshold_ms: u64::try_from(sleep_threshold.as_millis()).unwrap_or(u64::MAX),
        cpu_pressure: resources.cpu_pressure,
        memory_pressure: resources.memory_pressure,
    }
}

fn percentile_duration_ms(
    durations_ms: &[u64],
    numerator: usize,
    denominator: usize,
) -> Option<u64> {
    if durations_ms.is_empty() {
        return None;
    }

    let mut sorted = durations_ms.to_vec();
    sorted.sort_unstable();
    let len = sorted.len();
    let max_index = len.saturating_sub(1);
    let rank = max_index
        .saturating_mul(numerator)
        .saturating_add(denominator.saturating_sub(1))
        / denominator;
    let index = rank.min(max_index);
    sorted.get(index).copied()
}

#[must_use]
pub fn allocate_stage_run_id() -> String {
    format!("stage-run-{}", Uuid::new_v4())
}

pub fn stage_scope_rule(stage: &str) -> Result<StageScopeRule> {
    let normalized = stage.to_ascii_lowercase();
    let rule = match normalized.as_str() {
        "ingest" => StageScopeRule {
            scope_kind: None,
            commit_semantics: StageCommitSemantics::Precursor,
        },
        "normalize" | "prequalify notice" | "acquire docs" | "extract lots" => StageScopeRule {
            scope_kind: Some("notice"),
            commit_semantics: StageCommitSemantics::SetReplacement,
        },
        "relate" | "manifest" | "operator decision" | "bid workflow" => StageScopeRule {
            scope_kind: Some("process"),
            commit_semantics: StageCommitSemantics::SetReplacement,
        },
        "qualify lot" => StageScopeRule {
            scope_kind: Some("lot"),
            commit_semantics: StageCommitSemantics::SetReplacement,
        },
        "extract products" => StageScopeRule {
            scope_kind: Some("lot"),
            commit_semantics: StageCommitSemantics::Additive,
        },
        "score quoteability" | "gross margin gate" | "net margin gate" => StageScopeRule {
            scope_kind: Some("candidate"),
            commit_semantics: StageCommitSemantics::Additive,
        },
        "price-signal match" => StageScopeRule {
            scope_kind: Some("candidate_source_pair"),
            commit_semantics: StageCommitSemantics::Additive,
        },
        "landed cost" => StageScopeRule {
            scope_kind: Some("candidate_quote_scenario"),
            commit_semantics: StageCommitSemantics::Additive,
        },
        _ => return Err(anyhow!("unknown stage `{stage}`")),
    };
    Ok(rule)
}

pub fn validate_scope_kind(scope_kind: &str) -> Result<()> {
    ensure!(
        CANONICAL_SCOPE_KINDS.contains(&scope_kind),
        "scope_kind `{scope_kind}` is not canonical"
    );
    Ok(())
}

pub fn validate_stage_scope(stage: &str, scope_kind: &str) -> Result<StageScopeRule> {
    validate_scope_kind(scope_kind)?;
    let rule = stage_scope_rule(stage)?;
    let expected_scope_kind = rule
        .scope_kind
        .ok_or_else(|| anyhow!("stage `{stage}` is a precursor and has no scope_kind"))?;
    ensure!(
        expected_scope_kind == scope_kind,
        "stage `{stage}` must use scope_kind `{expected_scope_kind}`"
    );
    Ok(rule)
}

pub fn set_stage_head_freshness(
    conn: &Connection,
    process_id: &str,
    stage: &str,
    scope_kind: &str,
    scope_id: &str,
    freshness_state: FreshnessState,
) -> Result<()> {
    let updated = conn.execute(
        "\
        UPDATE process_stage_heads
        SET freshness_state = ?1,
            updated_at = CURRENT_TIMESTAMP
        WHERE process_id = ?2 AND stage = ?3 AND scope_kind = ?4 AND scope_id = ?5
        ",
        params![
            freshness_state.as_str(),
            process_id,
            stage,
            scope_kind,
            scope_id,
        ],
    )?;
    ensure!(updated == 1, "stage head missing for freshness update");
    Ok(())
}

pub fn persist_failed_set_replacement_attempt(
    conn: &Connection,
    write: &StageValidationWrite<'_>,
    error_manifest_id: Option<&str>,
) -> Result<()> {
    let rule = validate_stage_scope(write.stage, write.scope_kind)?;
    ensure!(
        rule.commit_semantics == StageCommitSemantics::SetReplacement,
        "partial-failure helper is only valid for set-replacement stages"
    );

    conn.execute(
        "\
        INSERT INTO process_stage_status (
          id, process_id, stage, scope_kind, scope_id, stage_run_id, state,
          freshness_state, started_at, completed_at, error_manifest_id,
          input_revision, output_revision, owner_id
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'failed', 'fresh', ?7, ?8, ?9, 0, 0, ?10)
        ",
        params![
            format!("{}-failed", write.stage_run_id),
            write.process_id,
            write.stage,
            write.scope_kind,
            write.scope_id,
            write.stage_run_id,
            Utc::now().to_rfc3339(),
            Utc::now().to_rfc3339(),
            error_manifest_id,
            write.owner_id,
        ],
    )?;
    Ok(())
}

#[must_use]
pub fn basis_hash_mismatch(expected: &str, actual: &str) -> bool {
    expected != actual
}

impl FreshnessState {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Fresh => "fresh",
            Self::SoftStale => "soft_stale",
            Self::HardInvalid => "hard_invalid",
        }
    }
}

fn current_commit_seq(conn: &Connection) -> Result<i64> {
    conn.query_row(
        "SELECT commit_seq FROM commit_clock WHERE id = 1",
        [],
        |row| row.get(0),
    )
    .context("reading current commit_seq")
}

pub fn promote_enforcement_provisional_output_tx(
    tx: &Transaction<'_>,
    executor_id: &str,
    schema_epoch: i32,
    request: &PromoteProvisionalRequest,
) -> Result<MutationAck> {
    let predicted_visible_after_seq = current_commit_seq(tx)? + 1;
    let provisional = load_provisional_for_promotion(tx, &request.worker_job_id)?;
    let verdict = load_linked_validation_verdict(tx, &request.worker_job_id)?;
    let promotion_source = if request.override_verdict.is_some() {
        apply_operator_override(tx, request, &verdict.id)?;
        "operator_override"
    } else {
        ensure!(
            verdict.verdict == "approved",
            "provisional promotion requires approved verdict, got `{}`",
            verdict.verdict
        );
        "validator_approved"
    };

    approve_provisional_for_promotion(tx, &provisional, request.override_verdict.is_some())?;
    let stage_write = StagePromotionWrite {
        process_id: &request.process_id,
        stage: &request.stage,
        scope_kind: &request.scope_kind,
        scope_id: &request.scope_id,
        stage_run_id: &request.stage_run_id,
        owner_id: &request.owner_id,
        executor_id,
        schema_epoch,
        provisional_id: &provisional.id,
        validation_verdict_id: &verdict.id,
        promotion_source,
        promoted_payload_json: &provisional.payload_json,
    };
    promote_stage_head_with_outbox(tx, &stage_write)?;

    let promoted_payload: Value = serde_json::from_str(&provisional.payload_json)
        .context("parsing promoted provisional payload JSON")?;
    let stored_payload = json!({
        "result": promoted_payload,
        "runner_meta": {
            "idempotency_key": request.idempotency_key,
            "visible_after_seq": predicted_visible_after_seq,
            "executor_id": executor_id,
            "continuation_job_id": Value::Null,
            "provisional_worker_output_id": provisional.id,
            "validation_verdict_id": verdict.id,
            "promotion_source": promotion_source,
        }
    })
    .to_string();
    let updated = tx.execute(
        "\
        UPDATE workflow_jobs
        SET status = 'completed',
            validation_required = 1,
            validation_verdict_id = ?1,
            completed_at = ?2,
            output_payload = ?3
        WHERE id = ?4
        ",
        params![
            verdict.id,
            Utc::now().to_rfc3339(),
            stored_payload,
            request.worker_job_id
        ],
    )?;
    ensure!(
        updated == 1,
        "workflow job missing for {}",
        request.worker_job_id
    );

    Ok(MutationAck {
        accepted: true,
        visible_after_seq: predicted_visible_after_seq,
        continuation_job_id: None,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ProvisionalForPromotion {
    id: String,
    payload_json: String,
    validation_status: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ValidationVerdictForPromotion {
    id: String,
    verdict: String,
}

#[derive(Debug, Clone, Copy)]
struct StagePromotionWrite<'a> {
    process_id: &'a str,
    stage: &'a str,
    scope_kind: &'a str,
    scope_id: &'a str,
    stage_run_id: &'a str,
    owner_id: &'a str,
    executor_id: &'a str,
    schema_epoch: i32,
    provisional_id: &'a str,
    validation_verdict_id: &'a str,
    promotion_source: &'a str,
    promoted_payload_json: &'a str,
}

fn ensure_no_existing_provisional(tx: &Transaction<'_>, worker_job_id: &str) -> Result<()> {
    let existing_count: i64 = tx
        .query_row(
            "\
            SELECT COUNT(*)
            FROM provisional_worker_outputs
            WHERE worker_job_id = ?1
            ",
            params![worker_job_id],
            |row| row.get(0),
        )
        .context("counting existing provisional worker outputs")?;
    ensure!(
        existing_count == 0,
        "provisional output already exists for worker_job_id `{worker_job_id}`"
    );
    Ok(())
}

fn load_provisional_for_promotion(
    tx: &Transaction<'_>,
    worker_job_id: &str,
) -> Result<ProvisionalForPromotion> {
    tx.query_row(
        "\
        SELECT id, payload_json, validation_status
        FROM provisional_worker_outputs
        WHERE worker_job_id = ?1
        ORDER BY created_at DESC, id DESC
        ",
        params![worker_job_id],
        |row| {
            Ok(ProvisionalForPromotion {
                id: row.get(0)?,
                payload_json: row.get(1)?,
                validation_status: row.get(2)?,
            })
        },
    )
    .optional()?
    .ok_or_else(|| anyhow!("provisional output missing for worker_job_id `{worker_job_id}`"))
}

fn load_linked_validation_verdict(
    tx: &Transaction<'_>,
    worker_job_id: &str,
) -> Result<ValidationVerdictForPromotion> {
    tx.query_row(
        "\
        SELECT vv.id, vv.verdict
        FROM workflow_jobs w
        JOIN validation_verdicts vv ON vv.id = w.validation_verdict_id
        WHERE w.id = ?1
        ",
        params![worker_job_id],
        |row| {
            Ok(ValidationVerdictForPromotion {
                id: row.get(0)?,
                verdict: row.get(1)?,
            })
        },
    )
    .optional()?
    .ok_or_else(|| anyhow!("validation verdict missing for worker job `{worker_job_id}`"))
}

fn apply_operator_override(
    tx: &Transaction<'_>,
    request: &PromoteProvisionalRequest,
    validation_verdict_id: &str,
) -> Result<()> {
    ensure!(
        request.override_verdict.as_deref() == Some("approved"),
        "operator override promotion requires override_verdict `approved`"
    );
    ensure!(
        request
            .override_reason
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty()),
        "operator override promotion requires override_reason"
    );
    ensure!(
        request
            .override_by
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty()),
        "operator override promotion requires override_by"
    );
    ensure!(
        request
            .override_at
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty()),
        "operator override promotion requires override_at"
    );
    let updated = tx.execute(
        "\
        UPDATE validation_verdicts
        SET override_verdict = ?1,
            override_reason = ?2,
            override_by = ?3,
            override_at = ?4
        WHERE id = ?5
        ",
        params![
            request.override_verdict.as_deref(),
            request.override_reason.as_deref(),
            request.override_by.as_deref(),
            request.override_at.as_deref(),
            validation_verdict_id
        ],
    )?;
    ensure!(
        updated == 1,
        "validation verdict missing for override `{validation_verdict_id}`"
    );
    Ok(())
}

fn approve_provisional_for_promotion(
    tx: &Transaction<'_>,
    provisional: &ProvisionalForPromotion,
    allow_rejected_override: bool,
) -> Result<()> {
    match provisional.validation_status.as_str() {
        "approved" => return Ok(()),
        "pending" => {}
        "rejected" if allow_rejected_override => {}
        status => {
            return Err(anyhow!(
                "provisional output `{}` cannot be promoted from status `{status}`",
                provisional.id
            ));
        }
    }

    let updated = tx.execute(
        "\
        UPDATE provisional_worker_outputs
        SET validation_status = 'approved',
            updated_at = CURRENT_TIMESTAMP
        WHERE id = ?1
        ",
        params![provisional.id],
    )?;
    ensure!(
        updated == 1,
        "provisional output missing for approval `{}`",
        provisional.id
    );
    Ok(())
}

fn promote_stage_head_with_outbox(
    tx: &Transaction<'_>,
    write: &StagePromotionWrite<'_>,
) -> Result<i64> {
    upsert_promoted_stage_status(tx, write)?;
    let outbox_op = upsert_promoted_stage_head(tx, write)?;
    let row_version = promoted_stage_head_row_version(tx, write)?;
    let local_seq = write_promoted_stage_head_outbox(tx, write, outbox_op, row_version)?;
    tx.execute(
        "\
        UPDATE process_stage_heads
        SET last_local_seq = ?1
        WHERE process_id = ?2 AND stage = ?3 AND scope_kind = ?4 AND scope_id = ?5
        ",
        params![
            local_seq,
            write.process_id,
            write.stage,
            write.scope_kind,
            write.scope_id
        ],
    )?;
    Ok(local_seq)
}

fn upsert_promoted_stage_status(
    tx: &Transaction<'_>,
    write: &StagePromotionWrite<'_>,
) -> Result<()> {
    tx.execute(
        "\
        INSERT INTO process_stage_status (
          id, process_id, stage, scope_kind, scope_id, stage_run_id, state,
          freshness_state, started_at, completed_at, owner_id
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'succeeded', 'fresh', ?7, ?7, ?8)
        ON CONFLICT(process_id, stage, scope_kind, scope_id, stage_run_id)
        DO UPDATE SET
          state = excluded.state,
          completed_at = excluded.completed_at,
          owner_id = excluded.owner_id
        ",
        params![
            format!("{}-{}-succeeded", write.stage_run_id, write.process_id),
            write.process_id,
            write.stage,
            write.scope_kind,
            write.scope_id,
            write.stage_run_id,
            Utc::now().to_rfc3339(),
            write.owner_id
        ],
    )?;
    Ok(())
}

fn upsert_promoted_stage_head(
    tx: &Transaction<'_>,
    write: &StagePromotionWrite<'_>,
) -> Result<&'static str> {
    let existing_count: i64 = tx.query_row(
        "\
        SELECT COUNT(*)
        FROM process_stage_heads
        WHERE process_id = ?1 AND stage = ?2 AND scope_kind = ?3 AND scope_id = ?4
        ",
        params![
            write.process_id,
            write.stage,
            write.scope_kind,
            write.scope_id
        ],
        |row| row.get(0),
    )?;
    let outbox_op = if existing_count == 0 {
        "insert"
    } else {
        "update"
    };

    tx.execute(
        "\
        INSERT INTO process_stage_heads (
          process_id, stage, scope_kind, scope_id, current_stage_run_id,
          freshness_state, owner_id, row_version, stage_run_id
        ) VALUES (?1, ?2, ?3, ?4, ?5, 'fresh', ?6, 1, ?5)
        ON CONFLICT(process_id, stage, scope_kind, scope_id) DO UPDATE SET
          current_stage_run_id = excluded.current_stage_run_id,
          freshness_state = excluded.freshness_state,
          owner_id = excluded.owner_id,
          stage_run_id = excluded.stage_run_id,
          row_version = process_stage_heads.row_version + 1,
          updated_at = CURRENT_TIMESTAMP
        ",
        params![
            write.process_id,
            write.stage,
            write.scope_kind,
            write.scope_id,
            write.stage_run_id,
            write.owner_id
        ],
    )?;
    Ok(outbox_op)
}

fn promoted_stage_head_row_version(
    tx: &Transaction<'_>,
    write: &StagePromotionWrite<'_>,
) -> Result<i64> {
    tx.query_row(
        "\
        SELECT row_version
        FROM process_stage_heads
        WHERE process_id = ?1 AND stage = ?2 AND scope_kind = ?3 AND scope_id = ?4
        ",
        params![
            write.process_id,
            write.stage,
            write.scope_kind,
            write.scope_id
        ],
        |row| row.get(0),
    )
    .context("loading promoted stage head row_version")
}

fn write_promoted_stage_head_outbox(
    tx: &Transaction<'_>,
    write: &StagePromotionWrite<'_>,
    outbox_op: &str,
    row_version: i64,
) -> Result<i64> {
    let promoted_payload: Value = serde_json::from_str(write.promoted_payload_json)
        .context("parsing promoted provisional payload JSON for outbox")?;
    let payload_json = json!({
        "process_id": write.process_id,
        "stage": write.stage,
        "scope_kind": write.scope_kind,
        "scope_id": write.scope_id,
        "current_stage_run_id": write.stage_run_id,
        "freshness_state": "fresh",
        "owner_id": write.owner_id,
        "row_version": row_version,
        "stage_run_id": write.stage_run_id,
        "validation_result": write.promotion_source,
        "provisional_worker_output_id": write.provisional_id,
        "validation_verdict_id": write.validation_verdict_id,
        "promoted_payload": promoted_payload,
    })
    .to_string();
    let content_hash = canonical_content_hash(&payload_json)?;
    let entity_id = format!(
        "{}:{}:{}:{}",
        write.process_id, write.stage, write.scope_kind, write.scope_id
    );
    let commit_group_id = format!(
        "provisional-promotion:{}:{}",
        write.provisional_id, write.stage_run_id
    );
    tx.execute(
        "\
        INSERT INTO local_outbox (
          executor_id, entity_kind, entity_id, op, row_version, stage_run_id,
          schema_epoch, commit_group_id, payload_json, content_hash
        ) VALUES (?1, 'process_stage_heads', ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
        ",
        params![
            write.executor_id,
            entity_id,
            outbox_op,
            row_version,
            write.stage_run_id,
            write.schema_epoch,
            commit_group_id,
            payload_json,
            content_hash
        ],
    )
    .context("writing promoted stage head outbox entry")?;
    Ok(tx.last_insert_rowid())
}

fn canonical_content_hash(payload_json: &str) -> Result<String> {
    let payload: Value = serde_json::from_str(payload_json).context("parsing payload_json")?;
    let canonical =
        serde_json::to_string(&payload).context("serializing canonical payload JSON")?;
    let mut hasher = Sha256::new();
    hasher.update(canonical.as_bytes());
    Ok(format!("{:x}", hasher.finalize()))
}

pub fn find_existing_ack(
    conn: &Connection,
    job_id: &str,
    idempotency_key: &str,
) -> Result<Option<MutationAck>> {
    let output_payload: Option<String> = conn
        .query_row(
            "SELECT output_payload FROM workflow_jobs WHERE id = ?1 AND status = 'completed'",
            params![job_id],
            |row| row.get(0),
        )
        .optional()
        .context("loading workflow job output payload")?;

    let Some(output_payload) = output_payload else {
        return Ok(None);
    };

    let parsed: Value =
        serde_json::from_str(&output_payload).context("parsing workflow output payload")?;
    let meta = parsed
        .get("runner_meta")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("completed workflow job missing runner_meta payload"))?;

    if meta.get("idempotency_key").and_then(Value::as_str) != Some(idempotency_key) {
        return Ok(None);
    }

    let visible_after_seq = meta
        .get("visible_after_seq")
        .and_then(Value::as_i64)
        .ok_or_else(|| anyhow!("runner_meta missing visible_after_seq"))?;
    let continuation_job_id = meta
        .get("continuation_job_id")
        .and_then(Value::as_str)
        .map(str::to_owned);

    Ok(Some(MutationAck {
        accepted: true,
        visible_after_seq,
        continuation_job_id,
    }))
}

fn find_existing_enqueue_ack(
    conn: &Connection,
    request: &EnqueueJobRequest,
) -> Result<Option<MutationAck>> {
    let mut statement = conn.prepare(
        "\
        SELECT input_payload
        FROM workflow_jobs
        WHERE kind = ?1
          AND process_id IS ?2
          AND notice_id IS ?3
          AND scope_kind IS ?4
          AND scope_id IS ?5
          AND parent_job_id IS ?6
          AND continuation_chain_id IS ?7
          AND status IN ('pending', 'running', 'retry', 'completed')
        ORDER BY created_at DESC
        ",
    )?;
    let rows = statement.query_map(
        params![
            request.kind,
            request.process_id,
            request.notice_id,
            request.scope_kind,
            request.scope_id,
            request.parent_job_id,
            request.continuation_chain_id,
        ],
        |row| row.get::<_, String>(0),
    )?;

    for row in rows {
        let raw_payload = row?;
        let parsed: Value =
            serde_json::from_str(&raw_payload).context("parsing workflow input payload")?;
        let Some(meta) = parsed.get("runner_meta").and_then(Value::as_object) else {
            continue;
        };
        if meta.get("idempotency_key").and_then(Value::as_str)
            != Some(request.idempotency_key.as_str())
        {
            continue;
        }
        let visible_after_seq = meta
            .get("visible_after_seq")
            .and_then(Value::as_i64)
            .ok_or_else(|| anyhow!("runner_meta missing visible_after_seq"))?;
        return Ok(Some(MutationAck {
            accepted: true,
            visible_after_seq,
            continuation_job_id: None,
        }));
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use serde::Serialize;
    use serde_json::json;

    use super::{
        SkillTelemetryWindow, compute_skill_telemetry, enqueue_bid_workflow_request,
        skill_telemetry_thresholds,
    };

    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct TestBidScopePayload {
        process_id: String,
        lot_ids: Vec<String>,
        candidate_ids: Vec<String>,
        selected_quote_ids: Vec<String>,
        company_profile_version: String,
        bid_portal_type: String,
        portal_opportunity_ref: String,
        review_required: bool,
        decision_basis_hash: String,
        side_effect_cap: String,
    }

    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct TestCompanyProfile {
        company_name: String,
        registration_number: String,
    }

    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct TestBidDocument {
        document_type: String,
        file_name: String,
        file_path: String,
        content_hash: String,
    }

    fn test_scope() -> TestBidScopePayload {
        TestBidScopePayload {
            process_id: "process-bid-001".to_owned(),
            lot_ids: vec!["lot-001".to_owned()],
            candidate_ids: vec!["candidate-001".to_owned()],
            selected_quote_ids: vec!["quote-001".to_owned()],
            company_profile_version: "company-profile-v1".to_owned(),
            bid_portal_type: "procontract".to_owned(),
            portal_opportunity_ref: "DN123456".to_owned(),
            review_required: true,
            decision_basis_hash: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
                .to_owned(),
            side_effect_cap: "REMOTE_DRAFT".to_owned(),
        }
    }

    fn test_company_profile() -> TestCompanyProfile {
        TestCompanyProfile {
            company_name: "Redacted Supplier".to_owned(),
            registration_number: "12345678".to_owned(),
        }
    }

    fn test_bid_documents() -> Vec<TestBidDocument> {
        vec![TestBidDocument {
            document_type: "pricing_schedule".to_owned(),
            file_name: "pricing.pdf".to_owned(),
            file_path: "artifacts/pricing.pdf".to_owned(),
            content_hash: "doc-hash-001".to_owned(),
        }]
    }

    #[test]
    fn enqueue_bid_workflow_request_shapes_run_bid_workflow_job() {
        let scope = test_scope();
        let company_profile = test_company_profile();
        let bid_documents = test_bid_documents();

        let request = enqueue_bid_workflow_request(
            &scope,
            "operator-alinaaitova",
            &company_profile,
            &bid_documents,
            "stage-run-bid-001",
        )
        .expect("bid workflow enqueue request should build");

        assert!(request.job_id.starts_with("job-"));
        assert_eq!(request.kind, "run_bid_workflow");
        assert_eq!(request.process_id.as_deref(), Some("process-bid-001"));
        assert_eq!(request.notice_id, None);
        assert_eq!(request.scope_kind.as_deref(), Some("process"));
        assert_eq!(request.scope_id.as_deref(), Some("process-bid-001"));
        assert_eq!(request.parent_job_id, None);
        assert_eq!(request.continuation_chain_id, None);
        assert!(request.validation_required);
        assert_eq!(request.scheduled_for, None);
        assert_eq!(
            request.idempotency_key,
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
        );
        assert_eq!(
            request.input_payload["bid_scope_payload"]["portalOpportunityRef"],
            "DN123456"
        );
        assert_eq!(
            request.input_payload["company_profile"]["registrationNumber"],
            "12345678"
        );
        assert_eq!(
            request.input_payload["bid_documents"][0]["documentType"],
            "pricing_schedule"
        );
        assert_eq!(request.input_payload["operator_id"], "operator-alinaaitova");
        assert_eq!(request.input_payload["stage_run_id"], "stage-run-bid-001");
        assert_eq!(
            request.input_payload["admission_priority"],
            json!("bid_automation")
        );
    }

    #[test]
    fn enqueue_bid_workflow_request_rejects_missing_scope_identity() {
        let company_profile = test_company_profile();
        let bid_documents = test_bid_documents();
        let missing_process = json!({
            "decisionBasisHash": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
        });

        let err = enqueue_bid_workflow_request(
            &missing_process,
            "operator-alinaaitova",
            &company_profile,
            &bid_documents,
            "stage-run-bid-001",
        )
        .expect_err("scope without processId must be rejected");

        assert!(err.to_string().contains("processId"));
    }

    #[test]
    fn skill_telemetry_snapshot_preserves_non_applicable_metrics_as_none() {
        let snapshot = compute_skill_telemetry(&SkillTelemetryWindow {
            successful_runs: 1,
            total_runs: 1,
            durations_ms: vec![1_200],
            cost_per_run_pence: Some(12),
            token_count: Some(1_024),
            checkpoint_count: Some(1),
            retry_count: Some(0),
            extraction_fill_rate_bps: Some(10_000),
            page_fingerprint_delta_bps: Some(0),
            auth_challenge_rate_bps: Some(0),
            listing_match_rate_bps: None,
            validation_rejection_rate_bps: None,
            drift_score_bps: Some(0),
        });

        assert_eq!(snapshot.success_rate, Some(1.0));
        assert_eq!(snapshot.median_duration_ms, Some(1_200));
        assert_eq!(snapshot.p95_duration_ms, Some(1_200));
        assert_eq!(snapshot.listing_match_rate_bps, None);
        assert_eq!(snapshot.validation_rejection_rate_bps, None);
    }

    #[test]
    fn skill_telemetry_thresholds_match_layer3_spec_table() {
        let thresholds = skill_telemetry_thresholds();
        assert!((thresholds.success_rate_min - 0.90).abs() < f64::EPSILON);
        assert_eq!(thresholds.median_duration_baseline_multiplier, 2);
        assert_eq!(thresholds.p95_duration_baseline_multiplier, 3);
        assert_eq!(thresholds.cost_per_run_baseline_multiplier, 2);
        assert_eq!(thresholds.retry_count_average_max, 2);
        assert_eq!(thresholds.extraction_fill_rate_min_bps, 7_000);
        assert_eq!(thresholds.page_fingerprint_delta_max_bps, 2_000);
        assert_eq!(thresholds.auth_challenge_rate_max_bps, 1_000);
        assert_eq!(thresholds.listing_match_rate_min_bps, 5_000);
        assert_eq!(thresholds.validation_rejection_rate_max_bps, 2_000);
        assert_eq!(thresholds.drift_score_max_bps, 3_000);
    }
}
