#![forbid(unsafe_code)]

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, ensure};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use uuid::Uuid;

#[cfg(not(test))]
use crate::db::sqlite::StateDb;
#[cfg(test)]
use crate::support::db::StateDb;

use crate::jobs::runner::{EnqueueJobRequest, JobRunner, MutationAck};

pub const VALIDATOR_JOB_KIND: &str = "semantic_validate";
pub const VALIDATOR_SKILL_PATH: &str = "skills/validation/base/SKILL.md";
pub const VALIDATOR_RESULT_SCHEMA_PATH: &str = "skills/validation/base/result.schema.json";
pub const VALIDATOR_RUBRIC_PATH: &str = "skills/validation/base/validator-rubric.yaml";
pub const VALIDATION_FLAGGED: &str = "VALIDATION_FLAGGED";
pub const VALIDATION_SKIPPED_VALIDATOR_ERROR: &str = "VALIDATION_SKIPPED_VALIDATOR_ERROR";
const EXPECTED_VALIDATOR_SKILL_ID: &str = "validation/base";
const EXPECTED_INTERACTION_PROFILE: &str = "headless_contract";
const EXPECTED_MODEL_PROFILE: &str = "structured_extract_low_cost";
const DEFAULT_AGENT_CLI: &str = "claude";

#[derive(Debug, Clone, Deserialize)]
struct ValidatorSkillFrontmatter {
    skill_id: String,
    skill_version: String,
    interaction_profile: String,
    result_schema_version: String,
    model_profile: String,
    #[serde(default)]
    agent_cli: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatorSkillBundle {
    pub skill_path: String,
    pub skill_version: String,
    pub interaction_profile: String,
    pub model_profile: String,
    pub agent_cli: String,
    pub result_schema_path: String,
    pub result_schema: Value,
    pub validator_rubric_path: String,
    pub validator_rubric_hash: String,
}

impl ValidatorSkillBundle {
    pub fn load(repo_root: &Path) -> Result<Self> {
        let skill_markdown_path = repo_root.join(VALIDATOR_SKILL_PATH);
        let skill_markdown = fs::read_to_string(&skill_markdown_path)
            .with_context(|| format!("reading {}", skill_markdown_path.display()))?;
        let (frontmatter, _) = split_frontmatter(&skill_markdown)?;

        ensure!(
            frontmatter.skill_id == EXPECTED_VALIDATOR_SKILL_ID,
            "validator skill_id `{}` must be `{EXPECTED_VALIDATOR_SKILL_ID}`",
            frontmatter.skill_id,
        );
        ensure!(
            frontmatter.interaction_profile == EXPECTED_INTERACTION_PROFILE,
            "validator interaction_profile `{}` must be `{EXPECTED_INTERACTION_PROFILE}`",
            frontmatter.interaction_profile,
        );
        ensure!(
            frontmatter.model_profile == EXPECTED_MODEL_PROFILE,
            "validator model_profile `{}` must be `{EXPECTED_MODEL_PROFILE}`",
            frontmatter.model_profile,
        );
        ensure!(
            !frontmatter.result_schema_version.is_empty(),
            "validator result_schema_version must not be empty",
        );

        let result_schema_path = repo_root.join(VALIDATOR_RESULT_SCHEMA_PATH);
        let result_schema = serde_json::from_str(
            &fs::read_to_string(&result_schema_path)
                .with_context(|| format!("reading {}", result_schema_path.display()))?,
        )
        .with_context(|| format!("parsing {}", result_schema_path.display()))?;

        let validator_rubric_path = repo_root.join(VALIDATOR_RUBRIC_PATH);
        let validator_rubric = fs::read(&validator_rubric_path)
            .with_context(|| format!("reading {}", validator_rubric_path.display()))?;

        Ok(Self {
            skill_path: VALIDATOR_SKILL_PATH.to_owned(),
            skill_version: frontmatter.skill_version,
            interaction_profile: frontmatter.interaction_profile,
            model_profile: frontmatter.model_profile,
            agent_cli: frontmatter
                .agent_cli
                .unwrap_or_else(|| DEFAULT_AGENT_CLI.to_owned()),
            result_schema_path: VALIDATOR_RESULT_SCHEMA_PATH.to_owned(),
            result_schema,
            validator_rubric_path: VALIDATOR_RUBRIC_PATH.to_owned(),
            validator_rubric_hash: sha256_hex(&validator_rubric),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidatorKickoffPacket {
    pub worker_job_id: String,
    pub worker_session_id: Option<String>,
    pub process_id: Option<String>,
    pub scope_kind: Option<String>,
    pub scope_id: Option<String>,
    pub run_manifest_path: String,
    pub run_manifest_hash: String,
    pub validator_rubric_hash: String,
    pub drift_delta_json: Value,
    pub oqg_anomaly_summary: Vec<String>,
    pub representative_item_sample: Value,
    pub corpus_provenance_tag: Option<String>,
    pub purpose: ValidatorJobPurpose,
    pub canary_case: Option<ValidationCanaryCase>,
    pub retroactive_backlog: Option<RetroactiveBacklogMetadata>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatorEnqueueRequest {
    pub validator_job_id: Option<String>,
    pub idempotency_key: Option<String>,
    pub worker_job_id: String,
    pub worker_session_id: Option<String>,
    pub process_id: Option<String>,
    pub scope_kind: Option<String>,
    pub scope_id: Option<String>,
    pub continuation_chain_id: Option<String>,
    pub run_manifest_path: PathBuf,
    pub run_manifest_hash: String,
    pub drift_delta_json: Value,
    pub oqg_anomaly_summary: Vec<String>,
    pub representative_item_sample: Value,
    pub corpus_provenance_tag: Option<String>,
    pub purpose: ValidatorJobPurpose,
    pub canary_case: Option<ValidationCanaryCase>,
    pub retroactive_backlog: Option<RetroactiveBacklogMetadata>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ValidatorJobPurpose {
    #[default]
    LiveValidation,
    SyntheticCanary,
    RetroactiveBacklog,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnqueuedValidatorJob {
    pub job_id: String,
    pub enqueue_request: EnqueueJobRequest,
    pub kickoff_packet: ValidatorKickoffPacket,
    pub skill: ValidatorSkillBundle,
    pub ack: MutationAck,
}

pub fn build_validator_kickoff_packet(
    skill: &ValidatorSkillBundle,
    request: &ValidatorEnqueueRequest,
) -> ValidatorKickoffPacket {
    ValidatorKickoffPacket {
        worker_job_id: request.worker_job_id.clone(),
        worker_session_id: request.worker_session_id.clone(),
        process_id: request.process_id.clone(),
        scope_kind: request.scope_kind.clone(),
        scope_id: request.scope_id.clone(),
        run_manifest_path: request.run_manifest_path.to_string_lossy().into_owned(),
        run_manifest_hash: request.run_manifest_hash.clone(),
        validator_rubric_hash: skill.validator_rubric_hash.clone(),
        drift_delta_json: request.drift_delta_json.clone(),
        oqg_anomaly_summary: request.oqg_anomaly_summary.clone(),
        representative_item_sample: request.representative_item_sample.clone(),
        corpus_provenance_tag: request.corpus_provenance_tag.clone(),
        purpose: request.purpose,
        canary_case: request.canary_case.clone(),
        retroactive_backlog: request.retroactive_backlog.clone(),
    }
}

pub fn enqueue_validator_sub_job(
    repo_root: &Path,
    state_db: &mut StateDb,
    runner: &JobRunner,
    request: &ValidatorEnqueueRequest,
) -> Result<EnqueuedValidatorJob> {
    let skill = ValidatorSkillBundle::load(repo_root)?;
    let kickoff_packet = build_validator_kickoff_packet(&skill, request);
    let job_id = request
        .validator_job_id
        .clone()
        .unwrap_or_else(|| format!("job-{}", Uuid::new_v4()));
    let idempotency_key = request.idempotency_key.clone().unwrap_or_else(|| {
        format!(
            "validator:{}:{}:{}",
            request.worker_job_id, request.run_manifest_hash, skill.validator_rubric_hash
        )
    });

    let enqueue_request = EnqueueJobRequest {
        job_id: job_id.clone(),
        kind: VALIDATOR_JOB_KIND.to_owned(),
        process_id: request.process_id.clone(),
        notice_id: None,
        scope_kind: request.scope_kind.clone(),
        scope_id: request.scope_id.clone(),
        parent_job_id: Some(request.worker_job_id.clone()),
        continuation_chain_id: request.continuation_chain_id.clone(),
        validation_required: false,
        scheduled_for: None,
        input_payload: json!({
            "skill_binding": {
                "path": skill.skill_path,
                "version": skill.skill_version,
            },
            "agent_cli": skill.agent_cli,
            "execution_profile": {
                "interaction_profile": skill.interaction_profile,
                "model_profile": skill.model_profile,
            },
            "validator_packet": kickoff_packet,
        }),
        idempotency_key,
    };

    let ack = runner.enqueue_job(state_db, &enqueue_request)?;
    Ok(EnqueuedValidatorJob {
        job_id,
        enqueue_request,
        kickoff_packet,
        skill,
        ack,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidatorVerdict {
    Approved,
    Rejected,
    Flagged,
}

impl ValidatorVerdict {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Approved => "approved",
            Self::Rejected => "rejected",
            Self::Flagged => "flagged",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidatorBasisSufficiency {
    High,
    Medium,
    Low,
}

impl ValidatorBasisSufficiency {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::High => "high",
            Self::Medium => "medium",
            Self::Low => "low",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidatorIssue {
    pub severity: String,
    pub category: String,
    pub description: String,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationCanaryCase {
    pub case_id: String,
    pub expected_verdict: Option<ValidatorVerdict>,
    pub expected_basis_sufficiency: ValidatorBasisSufficiency,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidatorResultContract {
    pub verdict: ValidatorVerdict,
    pub basis_sufficiency: ValidatorBasisSufficiency,
    pub enforcement_eligible: bool,
    pub confidence_bps: u16,
    pub issues: Vec<ValidatorIssue>,
    pub summary: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ValidationHealthState {
    #[default]
    Healthy,
    Degraded,
    Blind,
}

impl ValidationHealthState {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Healthy => "healthy",
            Self::Degraded => "degraded",
            Self::Blind => "blind",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidationTrustState {
    Verified,
    Flagged,
    Unverified,
    Suspect,
}

impl ValidationTrustState {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Verified => "verified",
            Self::Flagged => "flagged",
            Self::Unverified => "unverified",
            Self::Suspect => "suspect",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetroactiveBacklogMetadata {
    pub high_value_opportunity: bool,
    pub operator_reviewed: bool,
    pub enforcement_ready: bool,
    pub trust_state: ValidationTrustState,
    pub observed_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetroactiveBacklogItem {
    pub backlog_job_id: String,
    pub worker_job_id: String,
    pub worker_session_id: Option<String>,
    pub process_id: Option<String>,
    pub scope_kind: Option<String>,
    pub scope_id: Option<String>,
    pub trust_state: ValidationTrustState,
    pub high_value_opportunity: bool,
    pub operator_reviewed: bool,
    pub enforcement_ready: bool,
    pub observed_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProcessValidationState {
    Validated,
    Flagged,
    Unverified,
}

impl ProcessValidationState {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Validated => "validated",
            Self::Flagged => "flagged",
            Self::Unverified => "unverified",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ValidationHandlingMode {
    pub dry_run: bool,
    pub enforcement_active: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationHandlingDecision {
    pub trust_state: ValidationTrustState,
    pub process_validation_state: ProcessValidationState,
    pub advance_worker: bool,
    pub quarantine_worker: bool,
    pub queue_retroactive_backlog: bool,
    pub validation_marker: Option<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ValidationTelemetryWindow {
    pub validation_attempt_count: u32,
    pub approved_count: u32,
    pub rejected_count: u32,
    pub flagged_count: u32,
    pub total_cost_pence: u32,
    pub operator_override_count: u32,
    pub execution_failure_count: u32,
    pub canary_run_count: u32,
    pub canary_success_count: u32,
    pub timeout_spike: bool,
    pub basis_insufficiency_spike: bool,
    pub provider_unavailable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationTelemetrySnapshot {
    pub validator_approval_rate_bps: u16,
    pub validator_rejection_rate_bps: u16,
    pub validator_flag_rate_bps: u16,
    pub validator_cost_per_run_pence: u32,
    pub operator_override_rate_bps: u16,
    pub validator_execution_failure_rate_bps: u16,
    pub validator_canary_pass_rate_bps: u16,
    pub validator_health_state: ValidationHealthState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationBlackoutIncident {
    pub health_state: ValidationHealthState,
    pub affected_window_start: String,
    pub affected_window_end: String,
    pub reason: String,
    pub trust_debt_count: u32,
}

#[must_use]
pub fn decide_validator_verdict(
    contract: &ValidatorResultContract,
    mode: ValidationHandlingMode,
) -> ValidationHandlingDecision {
    if contract.basis_sufficiency == ValidatorBasisSufficiency::Low {
        return ValidationHandlingDecision {
            trust_state: ValidationTrustState::Unverified,
            process_validation_state: ProcessValidationState::Unverified,
            advance_worker: true,
            quarantine_worker: false,
            queue_retroactive_backlog: false,
            validation_marker: Some(VALIDATION_FLAGGED),
        };
    }

    match contract.verdict {
        ValidatorVerdict::Approved => ValidationHandlingDecision {
            trust_state: ValidationTrustState::Verified,
            process_validation_state: ProcessValidationState::Validated,
            advance_worker: true,
            quarantine_worker: false,
            queue_retroactive_backlog: false,
            validation_marker: None,
        },
        ValidatorVerdict::Rejected if mode.enforcement_active && contract.enforcement_eligible => {
            ValidationHandlingDecision {
                trust_state: ValidationTrustState::Suspect,
                process_validation_state: ProcessValidationState::Flagged,
                advance_worker: false,
                quarantine_worker: true,
                queue_retroactive_backlog: false,
                validation_marker: None,
            }
        }
        ValidatorVerdict::Rejected if mode.dry_run => ValidationHandlingDecision {
            trust_state: ValidationTrustState::Suspect,
            process_validation_state: ProcessValidationState::Flagged,
            advance_worker: true,
            quarantine_worker: false,
            queue_retroactive_backlog: false,
            validation_marker: Some(VALIDATION_FLAGGED),
        },
        ValidatorVerdict::Flagged | ValidatorVerdict::Rejected => ValidationHandlingDecision {
            trust_state: ValidationTrustState::Flagged,
            process_validation_state: ProcessValidationState::Flagged,
            advance_worker: true,
            quarantine_worker: false,
            queue_retroactive_backlog: false,
            validation_marker: Some(VALIDATION_FLAGGED),
        },
    }
}

#[must_use]
pub const fn decide_validator_failure() -> ValidationHandlingDecision {
    ValidationHandlingDecision {
        trust_state: ValidationTrustState::Unverified,
        process_validation_state: ProcessValidationState::Unverified,
        advance_worker: true,
        quarantine_worker: false,
        queue_retroactive_backlog: true,
        validation_marker: Some(VALIDATION_SKIPPED_VALIDATOR_ERROR),
    }
}

#[must_use]
pub fn compute_validation_telemetry(
    window: &ValidationTelemetryWindow,
    canary_override: Option<ValidationHealthState>,
) -> ValidationTelemetrySnapshot {
    let validated_count = window
        .approved_count
        .saturating_add(window.rejected_count)
        .saturating_add(window.flagged_count);
    let health_state = derive_validation_health_state(window, canary_override);

    ValidationTelemetrySnapshot {
        validator_approval_rate_bps: basis_points(window.approved_count, validated_count),
        validator_rejection_rate_bps: basis_points(window.rejected_count, validated_count),
        validator_flag_rate_bps: basis_points(window.flagged_count, validated_count),
        validator_cost_per_run_pence: window
            .total_cost_pence
            .checked_div(validated_count)
            .unwrap_or(0),
        operator_override_rate_bps: basis_points(
            window.operator_override_count,
            window.rejected_count,
        ),
        validator_execution_failure_rate_bps: basis_points(
            window.execution_failure_count,
            window.validation_attempt_count,
        ),
        validator_canary_pass_rate_bps: basis_points(
            window.canary_success_count,
            window.canary_run_count,
        ),
        validator_health_state: health_state,
    }
}

#[must_use]
pub fn derive_validation_health_state(
    window: &ValidationTelemetryWindow,
    canary_override: Option<ValidationHealthState>,
) -> ValidationHealthState {
    if matches!(canary_override, Some(ValidationHealthState::Blind)) || window.provider_unavailable
    {
        return ValidationHealthState::Blind;
    }

    let execution_failure_rate_bps = basis_points(
        window.execution_failure_count,
        window.validation_attempt_count,
    );
    if execution_failure_rate_bps > 5_000 {
        return ValidationHealthState::Blind;
    }

    if matches!(canary_override, Some(ValidationHealthState::Degraded))
        || execution_failure_rate_bps > 2_000
        || window.timeout_spike
        || window.basis_insufficiency_spike
    {
        return ValidationHealthState::Degraded;
    }

    ValidationHealthState::Healthy
}

#[must_use]
pub fn canary_health_override(
    canary_case: &ValidationCanaryCase,
    contract: &ValidatorResultContract,
) -> Option<ValidationHealthState> {
    if contract.basis_sufficiency != canary_case.expected_basis_sufficiency {
        return Some(ValidationHealthState::Degraded);
    }

    match canary_case.expected_verdict {
        Some(expected) if contract.verdict != expected => {
            if expected == ValidatorVerdict::Rejected
                && contract.verdict == ValidatorVerdict::Approved
            {
                Some(ValidationHealthState::Blind)
            } else {
                Some(ValidationHealthState::Degraded)
            }
        }
        None if contract.basis_sufficiency != ValidatorBasisSufficiency::Low => {
            Some(ValidationHealthState::Degraded)
        }
        _ => None,
    }
}

#[must_use]
pub fn blackout_incident_for_transition(
    previous: ValidationHealthState,
    next: ValidationHealthState,
    affected_window_start: DateTime<Utc>,
    affected_window_end: DateTime<Utc>,
    reason: impl Into<String>,
    trust_debt_count: u32,
) -> Option<ValidationBlackoutIncident> {
    if previous == ValidationHealthState::Blind || next != ValidationHealthState::Blind {
        return None;
    }

    Some(ValidationBlackoutIncident {
        health_state: next,
        affected_window_start: affected_window_start.to_rfc3339(),
        affected_window_end: affected_window_end.to_rfc3339(),
        reason: reason.into(),
        trust_debt_count,
    })
}

#[must_use]
pub fn rollup_process_validation_state(
    trust_states: impl IntoIterator<Item = ValidationTrustState>,
) -> ProcessValidationState {
    let mut saw_unverified = false;
    for trust_state in trust_states {
        match trust_state {
            ValidationTrustState::Flagged | ValidationTrustState::Suspect => {
                return ProcessValidationState::Flagged;
            }
            ValidationTrustState::Unverified => {
                saw_unverified = true;
            }
            ValidationTrustState::Verified => {}
        }
    }

    if saw_unverified {
        ProcessValidationState::Unverified
    } else {
        ProcessValidationState::Validated
    }
}

pub fn enqueue_retroactive_backlog_job(
    repo_root: &Path,
    state_db: &mut StateDb,
    runner: &JobRunner,
    mut request: ValidatorEnqueueRequest,
    backlog: RetroactiveBacklogMetadata,
) -> Result<EnqueuedValidatorJob> {
    request.purpose = ValidatorJobPurpose::RetroactiveBacklog;
    request.retroactive_backlog = Some(backlog);
    enqueue_validator_sub_job(repo_root, state_db, runner, &request)
}

pub fn enqueue_validation_canary_job(
    repo_root: &Path,
    state_db: &mut StateDb,
    runner: &JobRunner,
    mut request: ValidatorEnqueueRequest,
    canary_case: ValidationCanaryCase,
) -> Result<EnqueuedValidatorJob> {
    request.purpose = ValidatorJobPurpose::SyntheticCanary;
    request.canary_case = Some(canary_case);
    enqueue_validator_sub_job(repo_root, state_db, runner, &request)
}

pub fn load_pending_retroactive_backlog(
    conn: &rusqlite::Connection,
) -> Result<Vec<RetroactiveBacklogItem>> {
    let mut statement = conn.prepare(
        "\
        SELECT id, input_payload
        FROM workflow_jobs
        WHERE kind = ?1
          AND status = 'pending'
        ",
    )?;
    let rows = statement.query_map(rusqlite::params![VALIDATOR_JOB_KIND], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;

    let mut items = Vec::new();
    for row in rows {
        let (job_id, raw_payload) = row?;
        let payload: Value =
            serde_json::from_str(&raw_payload).context("parsing workflow_jobs.input_payload")?;
        let request = payload
            .get("request")
            .ok_or_else(|| anyhow!("workflow job missing request wrapper"))?;
        let packet = request
            .get("validator_packet")
            .ok_or_else(|| anyhow!("workflow job missing validator_packet"))?;
        let purpose: ValidatorJobPurpose = serde_json::from_value(
            packet
                .get("purpose")
                .cloned()
                .unwrap_or_else(|| json!("live_validation")),
        )
        .context("parsing validator purpose")?;
        if purpose != ValidatorJobPurpose::RetroactiveBacklog {
            continue;
        }
        let backlog: RetroactiveBacklogMetadata = serde_json::from_value(
            packet
                .get("retroactive_backlog")
                .cloned()
                .ok_or_else(|| anyhow!("retroactive backlog job missing backlog metadata"))?,
        )
        .context("parsing retroactive backlog metadata")?;
        items.push(RetroactiveBacklogItem {
            backlog_job_id: job_id,
            worker_job_id: packet
                .get("worker_job_id")
                .and_then(Value::as_str)
                .ok_or_else(|| anyhow!("retroactive backlog missing worker_job_id"))?
                .to_owned(),
            worker_session_id: packet
                .get("worker_session_id")
                .and_then(Value::as_str)
                .map(str::to_owned),
            process_id: packet
                .get("process_id")
                .and_then(Value::as_str)
                .map(str::to_owned),
            scope_kind: packet
                .get("scope_kind")
                .and_then(Value::as_str)
                .map(str::to_owned),
            scope_id: packet
                .get("scope_id")
                .and_then(Value::as_str)
                .map(str::to_owned),
            trust_state: backlog.trust_state,
            high_value_opportunity: backlog.high_value_opportunity,
            operator_reviewed: backlog.operator_reviewed,
            enforcement_ready: backlog.enforcement_ready,
            observed_at: backlog.observed_at,
        });
    }

    sort_retroactive_backlog(&mut items);
    Ok(items)
}

pub fn sort_retroactive_backlog(items: &mut [RetroactiveBacklogItem]) {
    items.sort_by(|left, right| {
        retroactive_backlog_rank(left)
            .cmp(&retroactive_backlog_rank(right))
            .then_with(|| right.observed_at.cmp(&left.observed_at))
            .then_with(|| left.backlog_job_id.cmp(&right.backlog_job_id))
    });
}

pub fn apply_retroactive_validation_consequences(
    state_db: &mut StateDb,
    worker_job_id: &str,
    process_id: &str,
    decision: &ValidationHandlingDecision,
    verdict_id: &str,
) -> Result<()> {
    let trust_state = decision.trust_state.as_str();
    let process_state = decision.process_validation_state.as_str();
    let retro_basis_hash = format!("retro-validation:{verdict_id}");
    let review_reason = decision
        .validation_marker
        .unwrap_or(VALIDATION_FLAGGED)
        .to_owned();

    let ((), _visible_after_seq) = state_db.with_write_transaction(|tx| {
        tx.execute(
            "UPDATE listing_evidence SET trust_state = ?1 WHERE source_job_id = ?2",
            rusqlite::params![trust_state, worker_job_id],
        )?;
        tx.execute(
            "\
            UPDATE pipeline_assessments
            SET validation_state = ?1,
                review_required = 1,
                review_stage = 'manifest',
                review_reason = ?2,
                decision_basis_hash = ?3,
                updated_at = CURRENT_TIMESTAMP
            WHERE process_id = ?4
            ",
            rusqlite::params![process_state, review_reason, retro_basis_hash, process_id],
        )?;
        Ok(())
    })?;
    Ok(())
}

pub fn drain_retroactive_backlog<F>(
    state_db: &mut StateDb,
    mut items: Vec<RetroactiveBacklogItem>,
    mut handle_item: F,
) -> Result<Vec<String>>
where
    F: FnMut(&RetroactiveBacklogItem) -> Result<Value>,
{
    sort_retroactive_backlog(&mut items);
    let mut drained_ids = Vec::with_capacity(items.len());
    for item in items {
        let payload = handle_item(&item)?;
        mark_retroactive_backlog_job_completed(state_db, &item.backlog_job_id, &payload)?;
        drained_ids.push(item.backlog_job_id);
    }
    Ok(drained_ids)
}

#[must_use]
pub fn retroactive_recovery_summary(processed_count: u32, issue_count: u32) -> String {
    format!("validated retroactively: {processed_count} runs processed, {issue_count} issues found")
}

fn mark_retroactive_backlog_job_completed(
    state_db: &mut StateDb,
    backlog_job_id: &str,
    output_payload: &Value,
) -> Result<()> {
    let (updated, _visible_after_seq) = state_db.with_write_transaction(|tx| {
        let updated = tx.execute(
            "\
            UPDATE workflow_jobs
            SET status = 'completed',
                completed_at = ?1,
                output_payload = ?2
            WHERE id = ?3
            ",
            rusqlite::params![
                Utc::now().to_rfc3339(),
                output_payload.to_string(),
                backlog_job_id,
            ],
        )?;
        Ok(updated)
    })?;
    ensure!(
        updated == 1,
        "retroactive backlog job missing for completion: {backlog_job_id}"
    );
    Ok(())
}

fn basis_points(numerator: u32, denominator: u32) -> u16 {
    if denominator == 0 {
        return 0;
    }

    let scaled = numerator.saturating_mul(10_000) / denominator;
    u16::try_from(scaled.min(10_000)).expect("basis points are capped to fit in u16")
}

const fn retroactive_backlog_rank(item: &RetroactiveBacklogItem) -> u8 {
    if item.high_value_opportunity {
        0
    } else if item.operator_reviewed {
        1
    } else if item.enforcement_ready {
        2
    } else {
        3
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistValidationVerdictRequest {
    pub verdict_id: String,
    pub worker_job_id: String,
    pub worker_session_id: Option<String>,
    pub validator_job_id: String,
    pub validator_session_id: Option<String>,
    pub run_manifest_hash: String,
    pub validator_skill: ValidatorSkillBundle,
    pub payload: Value,
    pub oqg_signals_summary: Option<Value>,
    pub incident_signature: Option<String>,
    pub quality_assessment_json: Option<Value>,
    pub skill_improvement_suggestions_json: Option<Value>,
    pub override_verdict: Option<String>,
    pub override_reason: Option<String>,
    pub override_by: Option<String>,
    pub override_at: Option<String>,
    pub verdict_rendered_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistedValidationVerdict {
    pub verdict_id: String,
    pub contract: ValidatorResultContract,
    pub visible_after_seq: i64,
}

pub fn persist_validation_verdict(
    state_db: &mut StateDb,
    request: &PersistValidationVerdictRequest,
) -> Result<PersistedValidationVerdict> {
    let contract =
        parse_validator_result_contract(&request.validator_skill.result_schema, &request.payload)?;
    let verdict_rendered_at = request
        .verdict_rendered_at
        .clone()
        .unwrap_or_else(|| Utc::now().to_rfc3339());
    let anchor_conflict_count = contract
        .issues
        .iter()
        .filter(|issue| issue.category == "identity")
        .count();

    let ((), visible_after_seq) = state_db.with_write_transaction(|tx| {
        tx.execute(
            "\
            INSERT INTO validation_verdicts (
              id,
              worker_job_id,
              worker_session_id,
              validator_job_id,
              validator_session_id,
              validator_skill_path,
              validator_skill_version,
              verdict,
              confidence_bps,
              summary,
              basis_sufficiency,
              anchor_conflict_count,
              incident_signature,
              enforcement_eligible,
              issues_json,
              quality_assessment_json,
              skill_improvement_suggestions_json,
              run_manifest_hash,
              oqg_signals_summary,
              override_verdict,
              override_reason,
              override_by,
              override_at,
              verdict_rendered_at
            ) VALUES (
              ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24
            )
            ",
            rusqlite::params![
                request.verdict_id,
                request.worker_job_id,
                request.worker_session_id,
                request.validator_job_id,
                request.validator_session_id,
                request.validator_skill.skill_path,
                request.validator_skill.skill_version,
                contract.verdict.as_str(),
                i64::from(contract.confidence_bps),
                contract.summary,
                contract.basis_sufficiency.as_str(),
                i64::try_from(anchor_conflict_count).context("anchor conflict count overflow")?,
                request.incident_signature,
                i64::from(contract.enforcement_eligible),
                serde_json::to_string(&contract.issues).context("serializing validator issues")?,
                request.quality_assessment_json.as_ref().map(Value::to_string),
                request
                    .skill_improvement_suggestions_json
                    .as_ref()
                    .map(Value::to_string),
                request.run_manifest_hash,
                request.oqg_signals_summary.as_ref().map(Value::to_string),
                request.override_verdict,
                request.override_reason,
                request.override_by,
                request.override_at,
                verdict_rendered_at,
            ],
        )
        .context("inserting validation_verdicts row")?;

        let updated = tx.execute(
            "UPDATE workflow_jobs SET validation_verdict_id = ?1 WHERE id = ?2",
            rusqlite::params![request.verdict_id, request.worker_job_id],
        )?;
        ensure!(
            updated == 1,
            "worker job missing for validation verdict linkage: {}",
            request.worker_job_id,
        );

        Ok(())
    })?;

    Ok(PersistedValidationVerdict {
        verdict_id: request.verdict_id.clone(),
        contract,
        visible_after_seq,
    })
}

#[must_use]
pub fn validation_scratch_dir(artifact_root: &Path, run_id: &str) -> PathBuf {
    artifact_root.join("scratch").join(run_id)
}

pub fn ensure_validation_scratch_dir(artifact_root: &Path, run_id: &str) -> Result<PathBuf> {
    let dir = validation_scratch_dir(artifact_root, run_id);
    fs::create_dir_all(&dir)
        .with_context(|| format!("creating validation scratch dir {}", dir.display()))?;
    Ok(dir)
}

pub fn parse_validator_result_contract(
    schema: &Value,
    payload: &Value,
) -> Result<ValidatorResultContract> {
    validate_json_payload(schema, payload)?;
    serde_json::from_value(payload.clone()).context("parsing validator result contract")
}

fn split_frontmatter(markdown: &str) -> Result<(ValidatorSkillFrontmatter, &str)> {
    let mut sections = markdown.splitn(3, "---");
    let leading = sections
        .next()
        .ok_or_else(|| anyhow!("validator SKILL.md missing YAML frontmatter"))?;
    ensure!(
        leading.trim().is_empty(),
        "validator SKILL.md must start with YAML frontmatter",
    );
    let yaml = sections
        .next()
        .ok_or_else(|| anyhow!("validator SKILL.md missing YAML frontmatter body"))?;
    let body = sections
        .next()
        .ok_or_else(|| anyhow!("validator SKILL.md missing markdown body"))?;
    let frontmatter: ValidatorSkillFrontmatter =
        serde_yaml::from_str(yaml).context("parsing validator skill frontmatter")?;
    Ok((frontmatter, body))
}

fn validate_json_payload(schema: &Value, payload: &Value) -> Result<()> {
    let payload_object = payload
        .as_object()
        .ok_or_else(|| anyhow!("result payload must be a JSON object"))?;

    if let Some(required_fields) = schema.get("required").and_then(Value::as_array) {
        for field in required_fields {
            let field_name = field
                .as_str()
                .ok_or_else(|| anyhow!("required field names must be strings"))?;
            ensure!(
                payload_object.contains_key(field_name),
                "result payload missing required field `{field_name}`",
            );
        }
    }

    if let Some(properties) = schema.get("properties").and_then(Value::as_object) {
        for (name, property_schema) in properties {
            let Some(value) = payload_object.get(name) else {
                continue;
            };
            if let Some(expected_type) = property_schema.get("type").and_then(Value::as_str) {
                ensure!(
                    value_matches_type(value, expected_type),
                    "result payload field `{name}` violated type `{expected_type}`",
                );
            }
        }
    }

    Ok(())
}

fn value_matches_type(value: &Value, expected_type: &str) -> bool {
    match expected_type {
        "string" => value.is_string(),
        "integer" => value.as_i64().is_some() || value.as_u64().is_some(),
        "number" => value.is_number(),
        "boolean" => value.is_boolean(),
        "array" => value.is_array(),
        "object" => value.is_object(),
        "null" => value.is_null(),
        _ => false,
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    format!("{digest:x}")
}
