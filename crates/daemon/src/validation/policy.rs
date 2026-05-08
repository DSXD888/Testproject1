#![forbid(unsafe_code)]

use std::collections::BTreeSet;

use rusqlite::{Connection, OptionalExtension, types::Type};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

pub const FULL_VALIDATION_BPS: u16 = 10_000;
pub const BOOTSTRAP_RUN_TARGET: u32 = 20;
pub const CALIBRATING_VALIDATION_BPS: u16 = 5_000;
pub const MIN_ADJUDICATED_CASES_FOR_ENFORCEMENT: i64 = 30;
pub const MIN_AUDITED_APPROVALS_FOR_ENFORCEMENT: i64 = 60;
pub const MIN_WILSON_LOWER_BOUND_BPS: i64 = 7_000;
pub const MAX_FALSE_PASS_UPPER_BOUND_BPS: i64 = 500;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ValidationPolicyMode {
    Always,
    Sample,
    Trigger,
    #[default]
    Skip,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillPromotionStage {
    Draft,
    Candidate,
    Canary,
    Shadow,
    Stable,
    Rollback,
}

impl SkillPromotionStage {
    #[must_use]
    pub const fn is_pre_stable(self) -> bool {
        matches!(
            self,
            Self::Draft | Self::Candidate | Self::Canary | Self::Shadow
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidationTrigger {
    FillRateDrop,
    FirstRunAfterVersionChange,
    SuspectOutputInLastFive,
    OperatorRequested,
    SourceHealthAmberOrRed,
    PageFingerprintUnknown,
    ContinuationChainLengthGtOne,
    HighValueOpportunity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceHealthState {
    Green,
    Amber,
    Red,
}

impl SourceHealthState {
    #[must_use]
    pub const fn is_amber_or_red(self) -> bool {
        matches!(self, Self::Amber | Self::Red)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BasisSufficiency {
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationPolicyConfig {
    pub worker_skill_id: String,
    pub source_id: String,
    pub validation_policy: ValidationPolicyMode,
    pub validation_sample_rate: u16,
    #[serde(default)]
    pub validation_triggers: BTreeSet<ValidationTrigger>,
    pub worker_behavior_version: String,
    pub validator_behavior_version: String,
    #[serde(default)]
    pub validator_overlay_compatible_versions: BTreeSet<String>,
}

impl ValidationPolicyConfig {
    #[must_use]
    pub const fn sample_rate_bps(&self) -> u16 {
        if self.validation_sample_rate > FULL_VALIDATION_BPS {
            FULL_VALIDATION_BPS
        } else {
            self.validation_sample_rate
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationSliceKey {
    pub worker_skill_id: String,
    pub source_id: String,
    pub worker_behavior_version: String,
    pub validator_behavior_version: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EnforcementReadinessState {
    Eligible,
    DryRun,
    Demoted,
}

impl EnforcementReadinessState {
    fn parse(raw: &str, column_index: usize) -> rusqlite::Result<Self> {
        match raw {
            "eligible" => Ok(Self::Eligible),
            "dry_run" => Ok(Self::DryRun),
            "demoted" => Ok(Self::Demoted),
            other => Err(conversion_error(
                column_index,
                format!("invalid enforcement_readiness.enforcement_state `{other}`"),
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnforcementReadinessRecord {
    pub slice_id: String,
    pub worker_skill_id: String,
    pub source_id: String,
    pub worker_behavior_version: String,
    pub validator_overlay_version: String,
    pub enforcement_state: EnforcementReadinessState,
    pub wilson_lower_bound_bps: i64,
    pub false_pass_upper_bound_bps: i64,
    pub adjudicated_count: i64,
    pub audited_approvals_count: i64,
    pub dominant_failure_families: Vec<String>,
    pub missing_failure_families: Vec<String>,
    pub demotion_reason: Option<String>,
    pub active_demotion_trigger_json: Option<Value>,
}

#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SliceEligibility {
    pub slice_id: String,
    pub persisted_state: EnforcementReadinessState,
    pub overlay_compatible: bool,
    pub adjudicated_threshold_met: bool,
    pub audited_approvals_threshold_met: bool,
    pub wilson_lower_bound_met: bool,
    pub false_pass_upper_bound_met: bool,
    pub dominant_failure_family_coverage_met: bool,
    pub no_active_demotion_trigger: bool,
    pub enforcement_eligible: bool,
}

#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationPolicyContext {
    pub schema_valid: bool,
    pub promotion_stage: SkillPromotionStage,
    pub source_health: SourceHealthState,
    pub page_fingerprint_unknown: bool,
    pub first_run_after_worker_version_change: bool,
    pub validator_error_spike_same_slice: bool,
    pub continuation_chain_length: u32,
    pub locator_fallback_depth: u32,
    pub locator_fallback_depth_threshold: u32,
    pub recent_basis_sufficiency: Option<BasisSufficiency>,
    pub high_value_opportunity: bool,
    pub suspect_output_count_last_five: u8,
    pub operator_requested: bool,
    pub fill_rate_drop_bps: Option<u16>,
    pub fill_rate_drop_threshold_bps: u16,
    pub sample_roll_bps: Option<u16>,
    pub base_risk_multiplier_bps: u16,
    pub calibration_thresholds_met: bool,
    pub operator_override_spike: bool,
    pub challenge_bank_regressed: bool,
    pub anomaly_conditioned_error_spike: bool,
    pub signature_limited_ready: bool,
    pub full_slice_enforcement_ready: bool,
    pub bootstrap_completed_runs: u32,
}

impl Default for ValidationPolicyContext {
    fn default() -> Self {
        Self {
            schema_valid: true,
            promotion_stage: SkillPromotionStage::Stable,
            source_health: SourceHealthState::Green,
            page_fingerprint_unknown: false,
            first_run_after_worker_version_change: false,
            validator_error_spike_same_slice: false,
            continuation_chain_length: 1,
            locator_fallback_depth: 0,
            locator_fallback_depth_threshold: 1,
            recent_basis_sufficiency: Some(BasisSufficiency::High),
            high_value_opportunity: false,
            suspect_output_count_last_five: 0,
            operator_requested: false,
            fill_rate_drop_bps: None,
            fill_rate_drop_threshold_bps: 0,
            sample_roll_bps: None,
            base_risk_multiplier_bps: 0,
            calibration_thresholds_met: false,
            operator_override_spike: false,
            challenge_bank_regressed: false,
            anomaly_conditioned_error_spike: false,
            signature_limited_ready: false,
            full_slice_enforcement_ready: false,
            bootstrap_completed_runs: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationReason {
    pub key: String,
    pub detail: Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DemotionTrigger {
    OperatorOverrideSpike,
    ChallengeBankRegression,
    AnomalyConditionedErrorSpike,
    OverlayIncompatibleWorkerVersion,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnforcementEligibility {
    pub slice_key: ValidationSliceKey,
    pub overlay_compatible: bool,
    pub calibration_thresholds_met: bool,
    pub demotion_triggers: Vec<DemotionTrigger>,
    pub enforcement_eligible: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DryRunLadderPhase {
    Bootstrap,
    Calibrating,
    SignatureLimitedEnforcement,
    FullSliceEnforcement,
}

impl DryRunLadderPhase {
    #[must_use]
    pub const fn validation_rate_bps(self, high_risk: bool) -> u16 {
        match self {
            Self::Calibrating => {
                if high_risk {
                    FULL_VALIDATION_BPS
                } else {
                    CALIBRATING_VALIDATION_BPS
                }
            }
            Self::Bootstrap | Self::SignatureLimitedEnforcement | Self::FullSliceEnforcement => {
                FULL_VALIDATION_BPS
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationDecision {
    pub validation_required: bool,
    pub oqg_required: bool,
    pub policy_mode: ValidationPolicyMode,
    pub effective_validation_probability_bps: u16,
    pub risk_multiplier_bps: u16,
    pub reasons: Vec<ValidationReason>,
    pub trigger_hits: BTreeSet<ValidationTrigger>,
    pub slice_key: ValidationSliceKey,
    pub enforcement: EnforcementEligibility,
    pub dry_run_phase: DryRunLadderPhase,
}

#[must_use]
pub fn evaluate_validation_policy(
    config: &ValidationPolicyConfig,
    context: &ValidationPolicyContext,
) -> ValidationDecision {
    let slice_key = slice_key(config);
    let enforcement = evaluate_enforcement_eligibility(config, context);
    let dry_run_phase = derive_dry_run_phase(context);

    let mut reasons = Vec::new();
    if !context.schema_valid {
        push_reason(
            &mut reasons,
            "schema_invalid_output_contract",
            json!({"error_code": "AGENT_OUTPUT_CONTRACT_INVALID"}),
        );
        return ValidationDecision {
            validation_required: false,
            oqg_required: false,
            policy_mode: config.validation_policy,
            effective_validation_probability_bps: 0,
            risk_multiplier_bps: 0,
            reasons,
            trigger_hits: BTreeSet::new(),
            slice_key,
            enforcement,
            dry_run_phase,
        };
    }

    let trigger_hits = evaluate_trigger_hits(config, context, &mut reasons);
    let risk_multiplier_bps = effective_risk_multiplier(context, &mut reasons);

    let mut effective_validation_probability_bps = 0;
    let mut validation_required = false;

    match config.validation_policy {
        ValidationPolicyMode::Always => {
            effective_validation_probability_bps = FULL_VALIDATION_BPS;
            validation_required = true;
            push_reason(&mut reasons, "validation_policy_always", json!({}));
        }
        ValidationPolicyMode::Sample => {
            effective_validation_probability_bps =
                config.sample_rate_bps().max(risk_multiplier_bps);
            validation_required = sample_decision(
                effective_validation_probability_bps,
                context.sample_roll_bps,
            );
            push_reason(
                &mut reasons,
                "validation_policy_sample",
                json!({
                    "validation_sample_rate_bps": config.sample_rate_bps(),
                    "risk_multiplier_bps": risk_multiplier_bps,
                    "effective_validation_probability_bps": effective_validation_probability_bps,
                    "sample_roll_bps": context.sample_roll_bps,
                }),
            );
        }
        ValidationPolicyMode::Trigger => {
            validation_required = !trigger_hits.is_empty();
            push_reason(
                &mut reasons,
                "validation_policy_trigger",
                json!({"trigger_count": trigger_hits.len()}),
            );
        }
        ValidationPolicyMode::Skip => {
            push_reason(&mut reasons, "validation_policy_skip", json!({}));
        }
    }

    if context.promotion_stage.is_pre_stable() {
        validation_required = true;
        effective_validation_probability_bps = FULL_VALIDATION_BPS;
        push_reason(
            &mut reasons,
            "pre_stable_promotion_stage",
            json!({"stage": context.promotion_stage}),
        );
    }

    if !trigger_hits.is_empty() {
        validation_required = true;
        effective_validation_probability_bps =
            effective_validation_probability_bps.max(FULL_VALIDATION_BPS);
    }

    ValidationDecision {
        validation_required,
        oqg_required: true,
        policy_mode: config.validation_policy,
        effective_validation_probability_bps,
        risk_multiplier_bps,
        reasons,
        trigger_hits,
        slice_key,
        enforcement,
        dry_run_phase,
    }
}

#[must_use]
pub fn evaluate_validation_policy_with_readiness(
    config: &ValidationPolicyConfig,
    context: &ValidationPolicyContext,
    readiness: Option<&EnforcementReadinessRecord>,
) -> ValidationDecision {
    let mut adjusted = context.clone();
    if let Some(readiness) = readiness {
        let eligibility = evaluate_slice_eligibility(config, readiness);
        adjusted.calibration_thresholds_met = eligibility.enforcement_eligible;
        adjusted.full_slice_enforcement_ready = eligibility.enforcement_eligible;
        if !eligibility.no_active_demotion_trigger {
            adjusted.anomaly_conditioned_error_spike = true;
        }
    }
    evaluate_validation_policy(config, &adjusted)
}

#[must_use]
pub fn apply_validator_blind_demotion(
    context: &ValidationPolicyContext,
) -> ValidationPolicyContext {
    let mut adjusted = context.clone();
    adjusted.signature_limited_ready = false;
    adjusted.full_slice_enforcement_ready = false;
    adjusted.anomaly_conditioned_error_spike = true;
    adjusted
}

#[must_use]
pub fn evaluate_validation_policy_with_blind_demotion(
    config: &ValidationPolicyConfig,
    context: &ValidationPolicyContext,
    validator_is_blind: bool,
) -> ValidationDecision {
    if validator_is_blind {
        evaluate_validation_policy(config, &apply_validator_blind_demotion(context))
    } else {
        evaluate_validation_policy(config, context)
    }
}

#[must_use]
pub fn evaluate_enforcement_eligibility(
    config: &ValidationPolicyConfig,
    context: &ValidationPolicyContext,
) -> EnforcementEligibility {
    let slice_key = slice_key(config);
    let overlay_compatible = config.validator_overlay_compatible_versions.is_empty()
        || config
            .validator_overlay_compatible_versions
            .contains(&config.worker_behavior_version);
    let mut demotion_triggers = Vec::new();

    if context.operator_override_spike {
        demotion_triggers.push(DemotionTrigger::OperatorOverrideSpike);
    }
    if context.challenge_bank_regressed {
        demotion_triggers.push(DemotionTrigger::ChallengeBankRegression);
    }
    if context.anomaly_conditioned_error_spike {
        demotion_triggers.push(DemotionTrigger::AnomalyConditionedErrorSpike);
    }
    if !overlay_compatible {
        demotion_triggers.push(DemotionTrigger::OverlayIncompatibleWorkerVersion);
    }

    EnforcementEligibility {
        slice_key,
        overlay_compatible,
        calibration_thresholds_met: context.calibration_thresholds_met,
        enforcement_eligible: overlay_compatible
            && context.calibration_thresholds_met
            && demotion_triggers.is_empty(),
        demotion_triggers,
    }
}

#[must_use]
pub fn derive_slice_id(slice_key: &ValidationSliceKey) -> String {
    format!(
        "{}::{}::{}::{}",
        slice_key.worker_skill_id,
        slice_key.source_id,
        slice_key.worker_behavior_version,
        slice_key.validator_behavior_version
    )
}

pub fn read_enforcement_state(
    conn: &Connection,
    slice_id: &str,
) -> rusqlite::Result<Option<EnforcementReadinessRecord>> {
    conn.query_row(
        "\
        SELECT slice_id, worker_skill_id, source_id, worker_behavior_version,
               validator_overlay_version, enforcement_state, wilson_lower_bound_bps,
               false_pass_upper_bound_bps, adjudicated_count, audited_approvals_count,
               dominant_failure_families_json, missing_failure_families_json,
               demotion_reason, active_demotion_trigger_json
        FROM enforcement_readiness
        WHERE slice_id = ?1
        ",
        [slice_id],
        |row| {
            let state_raw: String = row.get(5)?;
            let dominant_raw: String = row.get(10)?;
            let missing_raw: String = row.get(11)?;
            let active_demotion_raw: Option<String> = row.get(13)?;
            Ok(EnforcementReadinessRecord {
                slice_id: row.get(0)?,
                worker_skill_id: row.get(1)?,
                source_id: row.get(2)?,
                worker_behavior_version: row.get(3)?,
                validator_overlay_version: row.get(4)?,
                enforcement_state: EnforcementReadinessState::parse(&state_raw, 5)?,
                wilson_lower_bound_bps: row.get(6)?,
                false_pass_upper_bound_bps: row.get(7)?,
                adjudicated_count: row.get(8)?,
                audited_approvals_count: row.get(9)?,
                dominant_failure_families: parse_json_string_vec(&dominant_raw, 10)?,
                missing_failure_families: parse_json_string_vec(&missing_raw, 11)?,
                demotion_reason: row.get(12)?,
                active_demotion_trigger_json: parse_optional_json_value(
                    active_demotion_raw.as_deref(),
                    13,
                )?,
            })
        },
    )
    .optional()
}

#[must_use]
pub fn evaluate_slice_eligibility(
    config: &ValidationPolicyConfig,
    readiness: &EnforcementReadinessRecord,
) -> SliceEligibility {
    let expected_slice_id = derive_slice_id(&ValidationSliceKey {
        worker_skill_id: config.worker_skill_id.clone(),
        source_id: config.source_id.clone(),
        worker_behavior_version: config.worker_behavior_version.clone(),
        validator_behavior_version: config.validator_behavior_version.clone(),
    });
    let tuple_matches = readiness.slice_id == expected_slice_id
        && readiness.worker_skill_id == config.worker_skill_id
        && readiness.source_id == config.source_id
        && readiness.worker_behavior_version == config.worker_behavior_version
        && readiness.validator_overlay_version == config.validator_behavior_version;
    let compatible_range_matches = config.validator_overlay_compatible_versions.is_empty()
        || config
            .validator_overlay_compatible_versions
            .contains(&config.worker_behavior_version);
    let overlay_compatible = tuple_matches && compatible_range_matches;
    let adjudicated_threshold_met =
        readiness.adjudicated_count >= MIN_ADJUDICATED_CASES_FOR_ENFORCEMENT;
    let audited_approvals_threshold_met =
        readiness.audited_approvals_count >= MIN_AUDITED_APPROVALS_FOR_ENFORCEMENT;
    let wilson_lower_bound_met = readiness.wilson_lower_bound_bps >= MIN_WILSON_LOWER_BOUND_BPS;
    let false_pass_upper_bound_met =
        readiness.false_pass_upper_bound_bps <= MAX_FALSE_PASS_UPPER_BOUND_BPS;
    let dominant_failure_family_coverage_met = !readiness.dominant_failure_families.is_empty()
        && readiness.missing_failure_families.is_empty();
    let no_active_demotion_trigger = readiness.enforcement_state
        != EnforcementReadinessState::Demoted
        && !has_active_demotion_trigger(readiness.active_demotion_trigger_json.as_ref());
    let enforcement_eligible = readiness.enforcement_state == EnforcementReadinessState::Eligible
        && overlay_compatible
        && adjudicated_threshold_met
        && audited_approvals_threshold_met
        && wilson_lower_bound_met
        && false_pass_upper_bound_met
        && dominant_failure_family_coverage_met
        && no_active_demotion_trigger;

    SliceEligibility {
        slice_id: readiness.slice_id.clone(),
        persisted_state: readiness.enforcement_state,
        overlay_compatible,
        adjudicated_threshold_met,
        audited_approvals_threshold_met,
        wilson_lower_bound_met,
        false_pass_upper_bound_met,
        dominant_failure_family_coverage_met,
        no_active_demotion_trigger,
        enforcement_eligible,
    }
}

#[must_use]
pub const fn derive_dry_run_phase(context: &ValidationPolicyContext) -> DryRunLadderPhase {
    if context.full_slice_enforcement_ready {
        DryRunLadderPhase::FullSliceEnforcement
    } else if context.signature_limited_ready {
        DryRunLadderPhase::SignatureLimitedEnforcement
    } else if context.bootstrap_completed_runs < BOOTSTRAP_RUN_TARGET {
        DryRunLadderPhase::Bootstrap
    } else {
        DryRunLadderPhase::Calibrating
    }
}

fn slice_key(config: &ValidationPolicyConfig) -> ValidationSliceKey {
    ValidationSliceKey {
        worker_skill_id: config.worker_skill_id.clone(),
        source_id: config.source_id.clone(),
        worker_behavior_version: config.worker_behavior_version.clone(),
        validator_behavior_version: config.validator_behavior_version.clone(),
    }
}

fn parse_json_string_vec(raw: &str, column_index: usize) -> rusqlite::Result<Vec<String>> {
    serde_json::from_str(raw).map_err(|err| {
        rusqlite::Error::FromSqlConversionFailure(column_index, Type::Text, Box::new(err))
    })
}

fn parse_optional_json_value(
    raw: Option<&str>,
    column_index: usize,
) -> rusqlite::Result<Option<Value>> {
    raw.map(|value| {
        serde_json::from_str(value).map_err(|err| {
            rusqlite::Error::FromSqlConversionFailure(column_index, Type::Text, Box::new(err))
        })
    })
    .transpose()
}

fn has_active_demotion_trigger(active_trigger: Option<&Value>) -> bool {
    match active_trigger {
        None | Some(Value::Null) => false,
        Some(Value::String(value)) => !value.trim().is_empty(),
        Some(Value::Array(values)) => !values.is_empty(),
        Some(Value::Object(values)) => !values.is_empty(),
        Some(Value::Bool(_) | Value::Number(_)) => true,
    }
}

fn conversion_error(column_index: usize, message: String) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(
        column_index,
        Type::Text,
        Box::new(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            message,
        )),
    )
}

fn evaluate_trigger_hits(
    config: &ValidationPolicyConfig,
    context: &ValidationPolicyContext,
    reasons: &mut Vec<ValidationReason>,
) -> BTreeSet<ValidationTrigger> {
    let mut hits = BTreeSet::new();
    for trigger in &config.validation_triggers {
        if trigger_is_active(*trigger, context) {
            hits.insert(*trigger);
            push_reason(
                reasons,
                trigger_reason_key(*trigger),
                trigger_detail(*trigger, context),
            );
        }
    }
    hits
}

fn effective_risk_multiplier(
    context: &ValidationPolicyContext,
    reasons: &mut Vec<ValidationReason>,
) -> u16 {
    let mut multiplier = context.base_risk_multiplier_bps.min(FULL_VALIDATION_BPS);

    if context.source_health.is_amber_or_red() {
        multiplier = FULL_VALIDATION_BPS;
        push_reason(
            reasons,
            "source_health_amber_or_red",
            json!({"state": context.source_health}),
        );
    }
    if context.page_fingerprint_unknown {
        multiplier = FULL_VALIDATION_BPS;
        push_reason(reasons, "page_fingerprint_unknown", json!({}));
    }
    if context.first_run_after_worker_version_change {
        multiplier = FULL_VALIDATION_BPS;
        push_reason(reasons, "first_run_after_version_change", json!({}));
    }
    if context.validator_error_spike_same_slice {
        multiplier = FULL_VALIDATION_BPS;
        push_reason(reasons, "validator_error_spike_same_slice", json!({}));
    }
    if context.continuation_chain_length > 1 {
        multiplier = FULL_VALIDATION_BPS;
        push_reason(
            reasons,
            "continuation_chain_length_gt_1",
            json!({"continuation_chain_length": context.continuation_chain_length}),
        );
    }
    if context.locator_fallback_depth > context.locator_fallback_depth_threshold {
        multiplier = FULL_VALIDATION_BPS;
        push_reason(
            reasons,
            "high_locator_fallback_depth",
            json!({
                "locator_fallback_depth": context.locator_fallback_depth,
                "locator_fallback_depth_threshold": context.locator_fallback_depth_threshold,
            }),
        );
    }
    if context.recent_basis_sufficiency == Some(BasisSufficiency::Low) {
        multiplier = FULL_VALIDATION_BPS;
        push_reason(reasons, "recent_basis_sufficiency_low", json!({}));
    }
    if context.high_value_opportunity {
        multiplier = FULL_VALIDATION_BPS;
        push_reason(reasons, "high_value_opportunity", json!({}));
    }
    if context.suspect_output_count_last_five > 0 {
        multiplier = FULL_VALIDATION_BPS;
        push_reason(
            reasons,
            "suspect_output_in_last_5",
            json!({"count": context.suspect_output_count_last_five}),
        );
    }

    multiplier
}

const fn trigger_reason_key(trigger: ValidationTrigger) -> &'static str {
    match trigger {
        ValidationTrigger::FillRateDrop => "fill_rate_drop_gt_threshold",
        ValidationTrigger::FirstRunAfterVersionChange => "first_run_after_version_change",
        ValidationTrigger::SuspectOutputInLastFive => "suspect_output_in_last_5",
        ValidationTrigger::OperatorRequested => "operator_requested",
        ValidationTrigger::SourceHealthAmberOrRed => "source_health_amber_or_red",
        ValidationTrigger::PageFingerprintUnknown => "page_fingerprint_unknown",
        ValidationTrigger::ContinuationChainLengthGtOne => "continuation_chain_length_gt_1",
        ValidationTrigger::HighValueOpportunity => "high_value_opportunity",
    }
}

fn trigger_detail(trigger: ValidationTrigger, context: &ValidationPolicyContext) -> Value {
    match trigger {
        ValidationTrigger::FillRateDrop => json!({
            "fill_rate_drop_bps": context.fill_rate_drop_bps,
            "fill_rate_drop_threshold_bps": context.fill_rate_drop_threshold_bps,
        }),
        ValidationTrigger::FirstRunAfterVersionChange
        | ValidationTrigger::PageFingerprintUnknown
        | ValidationTrigger::HighValueOpportunity => json!({}),
        ValidationTrigger::SuspectOutputInLastFive => {
            json!({"count": context.suspect_output_count_last_five})
        }
        ValidationTrigger::OperatorRequested => json!({"source": "operator_requested"}),
        ValidationTrigger::SourceHealthAmberOrRed => json!({"state": context.source_health}),
        ValidationTrigger::ContinuationChainLengthGtOne => {
            json!({"continuation_chain_length": context.continuation_chain_length})
        }
    }
}

fn trigger_is_active(trigger: ValidationTrigger, context: &ValidationPolicyContext) -> bool {
    match trigger {
        ValidationTrigger::FillRateDrop => context
            .fill_rate_drop_bps
            .is_some_and(|drop| drop > context.fill_rate_drop_threshold_bps),
        ValidationTrigger::FirstRunAfterVersionChange => {
            context.first_run_after_worker_version_change
        }
        ValidationTrigger::SuspectOutputInLastFive => context.suspect_output_count_last_five > 0,
        ValidationTrigger::OperatorRequested => context.operator_requested,
        ValidationTrigger::SourceHealthAmberOrRed => context.source_health.is_amber_or_red(),
        ValidationTrigger::PageFingerprintUnknown => context.page_fingerprint_unknown,
        ValidationTrigger::ContinuationChainLengthGtOne => context.continuation_chain_length > 1,
        ValidationTrigger::HighValueOpportunity => context.high_value_opportunity,
    }
}

fn sample_decision(probability_bps: u16, roll_bps: Option<u16>) -> bool {
    if probability_bps >= FULL_VALIDATION_BPS {
        return true;
    }
    roll_bps.is_some_and(|roll| roll < probability_bps)
}

fn push_reason(reasons: &mut Vec<ValidationReason>, key: &str, detail: Value) {
    if reasons.iter().any(|reason| reason.key == key) {
        return;
    }
    reasons.push(ValidationReason {
        key: key.to_owned(),
        detail,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(mode: ValidationPolicyMode) -> ValidationPolicyConfig {
        ValidationPolicyConfig {
            worker_skill_id: "procurement-ingestion/fts".to_owned(),
            source_id: "fts".to_owned(),
            validation_policy: mode,
            validation_sample_rate: 2_500,
            validation_triggers: BTreeSet::from([
                ValidationTrigger::FillRateDrop,
                ValidationTrigger::FirstRunAfterVersionChange,
                ValidationTrigger::SuspectOutputInLastFive,
                ValidationTrigger::OperatorRequested,
                ValidationTrigger::SourceHealthAmberOrRed,
                ValidationTrigger::PageFingerprintUnknown,
                ValidationTrigger::ContinuationChainLengthGtOne,
                ValidationTrigger::HighValueOpportunity,
            ]),
            worker_behavior_version: "worker.v1".to_owned(),
            validator_behavior_version: "validator.v1".to_owned(),
            validator_overlay_compatible_versions: BTreeSet::from(["worker.v1".to_owned()]),
        }
    }

    fn context() -> ValidationPolicyContext {
        ValidationPolicyContext {
            sample_roll_bps: Some(9_000),
            locator_fallback_depth_threshold: 1,
            fill_rate_drop_threshold_bps: 500,
            recent_basis_sufficiency: Some(BasisSufficiency::High),
            ..ValidationPolicyContext::default()
        }
    }

    #[test]
    fn ut_validation_policy_001_schema_invalid_outputs_never_spawn_semantic_validation() {
        let mut ctx = context();
        ctx.schema_valid = false;

        let decision = evaluate_validation_policy(&config(ValidationPolicyMode::Always), &ctx);

        assert!(!decision.validation_required);
        assert!(!decision.oqg_required);
        assert!(
            decision
                .reasons
                .iter()
                .any(|reason| reason.key == "schema_invalid_output_contract")
        );
    }

    #[test]
    fn ut_validation_policy_002_pre_stable_stages_force_validation() {
        for stage in [
            SkillPromotionStage::Draft,
            SkillPromotionStage::Candidate,
            SkillPromotionStage::Canary,
            SkillPromotionStage::Shadow,
        ] {
            let mut ctx = context();
            ctx.promotion_stage = stage;

            let decision = evaluate_validation_policy(&config(ValidationPolicyMode::Skip), &ctx);

            assert!(decision.validation_required);
            assert!(
                decision
                    .reasons
                    .iter()
                    .any(|reason| reason.key == "pre_stable_promotion_stage")
            );
        }
    }

    #[test]
    fn ut_validation_policy_003_policy_always_validates_every_eligible_run() {
        let decision =
            evaluate_validation_policy(&config(ValidationPolicyMode::Always), &context());

        assert!(decision.validation_required);
        assert_eq!(
            decision.effective_validation_probability_bps,
            FULL_VALIDATION_BPS
        );
        assert!(
            decision
                .reasons
                .iter()
                .any(|reason| reason.key == "validation_policy_always")
        );
    }

    #[test]
    fn ut_validation_policy_004_sample_uses_max_rate_and_records_reason() {
        let mut ctx = context();
        ctx.base_risk_multiplier_bps = 4_000;
        ctx.sample_roll_bps = Some(3_000);

        let decision = evaluate_validation_policy(&config(ValidationPolicyMode::Sample), &ctx);

        assert!(decision.validation_required);
        assert_eq!(decision.effective_validation_probability_bps, 4_000);
        assert!(
            decision
                .reasons
                .iter()
                .any(|reason| reason.key == "validation_policy_sample")
        );
    }

    #[test]
    fn ut_validation_policy_005_source_health_amber_or_red_forces_validation() {
        for state in [SourceHealthState::Amber, SourceHealthState::Red] {
            let mut ctx = context();
            ctx.source_health = state;

            let decision = evaluate_validation_policy(&config(ValidationPolicyMode::Sample), &ctx);

            assert!(decision.validation_required);
            assert_eq!(decision.risk_multiplier_bps, FULL_VALIDATION_BPS);
            assert!(
                decision
                    .trigger_hits
                    .contains(&ValidationTrigger::SourceHealthAmberOrRed)
            );
        }
    }

    #[test]
    fn ut_validation_policy_006_unknown_fingerprint_forces_validation() {
        let mut ctx = context();
        ctx.page_fingerprint_unknown = true;

        let decision = evaluate_validation_policy(&config(ValidationPolicyMode::Sample), &ctx);

        assert!(decision.validation_required);
        assert_eq!(decision.risk_multiplier_bps, FULL_VALIDATION_BPS);
        assert!(
            decision
                .trigger_hits
                .contains(&ValidationTrigger::PageFingerprintUnknown)
        );
    }

    #[test]
    fn ut_validation_policy_007_first_run_after_behavior_change_forces_validation() {
        let mut ctx = context();
        ctx.first_run_after_worker_version_change = true;

        let decision = evaluate_validation_policy(&config(ValidationPolicyMode::Sample), &ctx);

        assert!(decision.validation_required);
        assert!(
            decision
                .trigger_hits
                .contains(&ValidationTrigger::FirstRunAfterVersionChange)
        );
    }

    #[test]
    fn ut_validation_policy_008_continuation_and_locator_fallback_force_validation() {
        let mut chain_ctx = context();
        chain_ctx.continuation_chain_length = 2;
        let chain_decision =
            evaluate_validation_policy(&config(ValidationPolicyMode::Sample), &chain_ctx);
        assert!(chain_decision.validation_required);
        assert!(
            chain_decision
                .reasons
                .iter()
                .any(|reason| reason.key == "continuation_chain_length_gt_1")
        );

        let mut fallback_ctx = context();
        fallback_ctx.locator_fallback_depth = 2;
        let fallback_decision =
            evaluate_validation_policy(&config(ValidationPolicyMode::Sample), &fallback_ctx);
        assert!(fallback_decision.validation_required);
        assert!(
            fallback_decision
                .reasons
                .iter()
                .any(|reason| reason.key == "high_locator_fallback_depth")
        );
    }

    #[test]
    fn ut_validation_policy_009_low_basis_and_validator_error_spike_force_validation() {
        let mut low_basis_ctx = context();
        low_basis_ctx.recent_basis_sufficiency = Some(BasisSufficiency::Low);
        let low_basis_decision =
            evaluate_validation_policy(&config(ValidationPolicyMode::Sample), &low_basis_ctx);
        assert!(low_basis_decision.validation_required);
        assert!(
            low_basis_decision
                .reasons
                .iter()
                .any(|reason| reason.key == "recent_basis_sufficiency_low")
        );

        let mut spike_ctx = context();
        spike_ctx.validator_error_spike_same_slice = true;
        let spike_decision =
            evaluate_validation_policy(&config(ValidationPolicyMode::Sample), &spike_ctx);
        assert!(spike_decision.validation_required);
        assert!(
            spike_decision
                .reasons
                .iter()
                .any(|reason| reason.key == "validator_error_spike_same_slice")
        );
    }

    #[test]
    fn ut_validation_policy_010_high_value_and_suspect_output_force_validation() {
        let mut high_value_ctx = context();
        high_value_ctx.high_value_opportunity = true;
        let high_value_decision =
            evaluate_validation_policy(&config(ValidationPolicyMode::Sample), &high_value_ctx);
        assert!(high_value_decision.validation_required);
        assert!(
            high_value_decision
                .reasons
                .iter()
                .any(|reason| reason.key == "high_value_opportunity")
        );

        let mut suspect_ctx = context();
        suspect_ctx.suspect_output_count_last_five = 2;
        let suspect_decision =
            evaluate_validation_policy(&config(ValidationPolicyMode::Sample), &suspect_ctx);
        assert!(suspect_decision.validation_required);
        let reason = suspect_decision
            .reasons
            .iter()
            .find(|reason| reason.key == "suspect_output_in_last_5")
            .expect("reason");
        assert_eq!(reason.detail["count"], 2);
    }

    #[test]
    fn ut_validation_policy_011_trigger_mode_validates_only_on_declared_triggers() {
        let no_trigger =
            evaluate_validation_policy(&config(ValidationPolicyMode::Trigger), &context());
        assert!(!no_trigger.validation_required);

        let mut triggered_ctx = context();
        triggered_ctx.fill_rate_drop_bps = Some(600);
        let triggered =
            evaluate_validation_policy(&config(ValidationPolicyMode::Trigger), &triggered_ctx);
        assert!(triggered.validation_required);
        assert!(
            triggered
                .trigger_hits
                .contains(&ValidationTrigger::FillRateDrop)
        );
    }

    #[test]
    fn ut_validation_policy_012_skip_mode_keeps_oqg_and_allows_overrides() {
        let skipped = evaluate_validation_policy(&config(ValidationPolicyMode::Skip), &context());
        assert!(!skipped.validation_required);
        assert!(skipped.oqg_required);

        let mut override_ctx = context();
        override_ctx.high_value_opportunity = true;
        let overridden =
            evaluate_validation_policy(&config(ValidationPolicyMode::Skip), &override_ctx);
        assert!(overridden.validation_required);
        assert!(overridden.oqg_required);
    }

    #[test]
    fn ut_validation_policy_013_trigger_reasons_are_structured() {
        let mut ctx = context();
        ctx.fill_rate_drop_bps = Some(700);

        let decision = evaluate_validation_policy(&config(ValidationPolicyMode::Trigger), &ctx);
        let reason = decision
            .reasons
            .iter()
            .find(|reason| reason.key == "fill_rate_drop_gt_threshold")
            .expect("fill-rate trigger reason");

        assert_eq!(reason.detail["fill_rate_drop_bps"], 700);
        assert_eq!(reason.detail["fill_rate_drop_threshold_bps"], 500);
    }

    #[test]
    fn ut_validation_policy_014_slice_key_is_exact_and_cosmetic_edits_do_not_reset() {
        let cfg = config(ValidationPolicyMode::Sample);
        let decision = evaluate_validation_policy(&cfg, &context());

        assert_eq!(
            decision.slice_key,
            ValidationSliceKey {
                worker_skill_id: "procurement-ingestion/fts".to_owned(),
                source_id: "fts".to_owned(),
                worker_behavior_version: "worker.v1".to_owned(),
                validator_behavior_version: "validator.v1".to_owned(),
            }
        );
        assert!(decision.enforcement.overlay_compatible);
    }

    #[test]
    fn ut_validation_policy_015_demotion_is_immediate() {
        let mut ctx = context();
        ctx.calibration_thresholds_met = true;
        ctx.operator_override_spike = true;

        let decision = evaluate_validation_policy(&config(ValidationPolicyMode::Always), &ctx);

        assert!(!decision.enforcement.enforcement_eligible);
        assert!(
            decision
                .enforcement
                .demotion_triggers
                .contains(&DemotionTrigger::OperatorOverrideSpike)
        );
    }

    #[test]
    fn ut_validation_policy_016_dry_run_ladder_uses_exact_rates() {
        let bootstrap = derive_dry_run_phase(&context());
        assert_eq!(bootstrap, DryRunLadderPhase::Bootstrap);
        assert_eq!(bootstrap.validation_rate_bps(false), FULL_VALIDATION_BPS);

        let mut calibrating_ctx = context();
        calibrating_ctx.bootstrap_completed_runs = BOOTSTRAP_RUN_TARGET;
        let calibrating = derive_dry_run_phase(&calibrating_ctx);
        assert_eq!(calibrating, DryRunLadderPhase::Calibrating);
        assert_eq!(
            calibrating.validation_rate_bps(false),
            CALIBRATING_VALIDATION_BPS
        );
        assert_eq!(calibrating.validation_rate_bps(true), FULL_VALIDATION_BPS);

        let mut signature_limited_ctx = calibrating_ctx;
        signature_limited_ctx.signature_limited_ready = true;
        assert_eq!(
            derive_dry_run_phase(&signature_limited_ctx),
            DryRunLadderPhase::SignatureLimitedEnforcement
        );

        let mut full_ctx = signature_limited_ctx;
        full_ctx.full_slice_enforcement_ready = true;
        assert_eq!(
            derive_dry_run_phase(&full_ctx),
            DryRunLadderPhase::FullSliceEnforcement
        );
    }

    #[test]
    fn ut_validation_policy_017_fill_rate_drop_trigger_records_delta() {
        let mut ctx = context();
        ctx.fill_rate_drop_bps = Some(900);

        let decision = evaluate_validation_policy(&config(ValidationPolicyMode::Trigger), &ctx);
        let reason = decision
            .reasons
            .iter()
            .find(|reason| reason.key == "fill_rate_drop_gt_threshold")
            .expect("fill rate reason");

        assert!(decision.validation_required);
        assert_eq!(reason.detail["fill_rate_drop_bps"], 900);
    }

    #[test]
    fn ut_validation_policy_018_operator_requested_forces_validation_even_in_skip_mode() {
        let mut ctx = context();
        ctx.operator_requested = true;

        let decision = evaluate_validation_policy(&config(ValidationPolicyMode::Skip), &ctx);
        let reason = decision
            .reasons
            .iter()
            .find(|reason| reason.key == "operator_requested")
            .expect("operator override reason");

        assert!(decision.validation_required);
        assert_eq!(reason.detail["source"], "operator_requested");
    }
}
