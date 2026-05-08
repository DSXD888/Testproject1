#![forbid(unsafe_code)]

use std::collections::BTreeSet;

use rusqlite::{Connection, params, types::Type};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

const WILSON_Z_95: f64 = 1.959_963_984_540_054;
const BPS_SCALE: f64 = 10_000.0;
const RULE_OF_THREE_NUMERATOR_BPS: u32 = 30_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EnforcementReadinessState {
    Eligible,
    DryRun,
    Demoted,
}

impl EnforcementReadinessState {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Eligible => "eligible",
            Self::DryRun => "dry_run",
            Self::Demoted => "demoted",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DemotionThresholds {
    pub override_reviewed_rejections_min: u32,
    pub override_rate_bps_min: u32,
    pub override_rate_multiplier_bps: u32,
    pub challenge_pass_rate_min_bps: u32,
    pub anomaly_samples_min: u32,
    pub anomaly_error_rate_bps_min: u32,
    pub anomaly_error_rate_multiplier_bps: u32,
}

pub const DEFAULT_DEMOTION_THRESHOLDS: DemotionThresholds = DemotionThresholds {
    override_reviewed_rejections_min: 10,
    override_rate_bps_min: 1_500,
    override_rate_multiplier_bps: 20_000,
    challenge_pass_rate_min_bps: 9_500,
    anomaly_samples_min: 20,
    anomaly_error_rate_bps_min: 500,
    anomaly_error_rate_multiplier_bps: 20_000,
};

#[allow(clippy::struct_field_names)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalibrationSliceConfig {
    pub slice_id: String,
    pub worker_skill_id: String,
    pub source_id: String,
    pub worker_behavior_version: String,
    pub validator_overlay_version: String,
    pub validator_overlay_compatible_versions: BTreeSet<String>,
    pub dominant_failure_families: BTreeSet<String>,
    pub demotion_thresholds: DemotionThresholds,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DemotionTriggerKind {
    OverrideSpike,
    ChallengeRegression,
    AnomalyValidatorError,
    CompatibleRangeExit,
}

impl DemotionTriggerKind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::OverrideSpike => "override_spike",
            Self::ChallengeRegression => "challenge_regression",
            Self::AnomalyValidatorError => "anomaly_validator_error",
            Self::CompatibleRangeExit => "compatible_range_exit",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DemotionTrigger {
    pub kind: DemotionTriggerKind,
    pub reason: String,
    pub details: Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReadinessAggregates {
    pub adjudicated_count: u32,
    pub correct_rejection_count: u32,
    pub audited_approvals_count: u32,
    pub false_pass_count: u32,
    pub covered_failure_families: BTreeSet<String>,
    pub override_reviewed_rejections: u32,
    pub operator_overrides: u32,
    pub challenge_total: u32,
    pub challenge_passed: u32,
    pub critical_mutated_bad_approved: bool,
    pub anomaly_samples: u32,
    pub anomaly_errors: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnforcementReadinessSnapshot {
    pub slice_id: String,
    pub enforcement_state: EnforcementReadinessState,
    pub wilson_lower_bound_bps: i64,
    pub false_pass_upper_bound_bps: i64,
    pub adjudicated_count: i64,
    pub audited_approvals_count: i64,
    pub dominant_failure_families: Vec<String>,
    pub missing_failure_families: Vec<String>,
    pub active_demotion_trigger_json: Option<Value>,
}

#[must_use]
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub fn wilson_lower_bound_bps(correct: u32, total: u32) -> i64 {
    if total == 0 || correct == 0 {
        return 0;
    }
    let successes = f64::from(correct.min(total));
    let samples = f64::from(total);
    let phat = successes / samples;
    let z2 = WILSON_Z_95 * WILSON_Z_95;
    let denominator = 1.0 + z2 / samples;
    let center = phat + z2 / (2.0 * samples);
    let margin = WILSON_Z_95 * (phat.mul_add(1.0 - phat, z2 / (4.0 * samples)) / samples).sqrt();
    ((center - margin) / denominator * BPS_SCALE)
        .floor()
        .clamp(0.0, BPS_SCALE) as i64
}

#[must_use]
pub fn rule_of_three_false_pass_upper_bound_bps(misses: u32, audited: u32) -> i64 {
    if audited == 0 {
        return 10_000;
    }
    if misses == 0 {
        return i64::from(ceil_div(RULE_OF_THREE_NUMERATOR_BPS, audited).min(10_000));
    }
    wilson_upper_bound_bps(misses.min(audited), audited)
}

pub fn recompute_enforcement_readiness(
    conn: &Connection,
    config: &CalibrationSliceConfig,
    evaluated_at: &str,
) -> rusqlite::Result<EnforcementReadinessSnapshot> {
    let aggregates = load_readiness_aggregates(conn, &config.slice_id)?;
    let wilson_lower_bound_bps = wilson_lower_bound_bps(
        aggregates.correct_rejection_count,
        aggregates.adjudicated_count,
    );
    let false_pass_upper_bound_bps = rule_of_three_false_pass_upper_bound_bps(
        aggregates.false_pass_count,
        aggregates.audited_approvals_count,
    );
    let missing_failure_families = missing_failure_families(config, &aggregates);
    let active_triggers = evaluate_demotion_triggers(config, &aggregates);
    let overlay_compatible = overlay_compatible(config);
    let calibration_thresholds_met = overlay_compatible
        && aggregates.adjudicated_count >= 30
        && aggregates.audited_approvals_count >= 60
        && wilson_lower_bound_bps >= 7_000
        && false_pass_upper_bound_bps <= 500
        && missing_failure_families.is_empty();
    let enforcement_state = if active_triggers.is_empty() {
        if calibration_thresholds_met {
            EnforcementReadinessState::Eligible
        } else {
            EnforcementReadinessState::DryRun
        }
    } else {
        EnforcementReadinessState::Demoted
    };
    let active_demotion_trigger_json = if active_triggers.is_empty() {
        None
    } else {
        Some(json!(active_triggers))
    };
    let demotion_reason = active_triggers
        .first()
        .map(|trigger| trigger.kind.as_str().to_owned());
    let dominant_failure_families = config
        .dominant_failure_families
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    let missing_failure_families_vec = missing_failure_families.into_iter().collect::<Vec<_>>();

    persist_readiness(
        conn,
        config,
        evaluated_at,
        enforcement_state,
        wilson_lower_bound_bps,
        false_pass_upper_bound_bps,
        i64::from(aggregates.adjudicated_count),
        i64::from(aggregates.audited_approvals_count),
        &dominant_failure_families,
        &missing_failure_families_vec,
        demotion_reason.as_deref(),
        active_demotion_trigger_json.as_ref(),
    )?;

    Ok(EnforcementReadinessSnapshot {
        slice_id: config.slice_id.clone(),
        enforcement_state,
        wilson_lower_bound_bps,
        false_pass_upper_bound_bps,
        adjudicated_count: i64::from(aggregates.adjudicated_count),
        audited_approvals_count: i64::from(aggregates.audited_approvals_count),
        dominant_failure_families,
        missing_failure_families: missing_failure_families_vec,
        active_demotion_trigger_json,
    })
}

fn load_readiness_aggregates(
    conn: &Connection,
    slice_id: &str,
) -> rusqlite::Result<ReadinessAggregates> {
    let mut aggregates = ReadinessAggregates {
        adjudicated_count: 0,
        correct_rejection_count: 0,
        audited_approvals_count: 0,
        false_pass_count: 0,
        covered_failure_families: BTreeSet::new(),
        override_reviewed_rejections: 0,
        operator_overrides: 0,
        challenge_total: 0,
        challenge_passed: 0,
        critical_mutated_bad_approved: false,
        anomaly_samples: 0,
        anomaly_errors: 0,
    };
    let mut stmt = conn.prepare(
        "\
        SELECT corpus_kind, expected_verdict, actual_verdict, mutation_family, metadata_json
        FROM validation_calibration_cases
        WHERE slice_id = ?1 AND adjudicated_at IS NOT NULL
        ",
    )?;
    let mut rows = stmt.query([slice_id])?;
    while let Some(row) = rows.next()? {
        let corpus_kind: String = row.get(0)?;
        let expected_verdict: Option<String> = row.get(1)?;
        let actual_verdict: Option<String> = row.get(2)?;
        let mutation_family: Option<String> = row.get(3)?;
        let metadata_raw: String = row.get(4)?;
        let metadata = parse_metadata(&metadata_raw, 4)?;
        apply_case(
            &mut aggregates,
            &corpus_kind,
            expected_verdict.as_deref(),
            actual_verdict.as_deref(),
            mutation_family.as_deref(),
            &metadata,
        );
    }
    Ok(aggregates)
}

fn apply_case(
    aggregates: &mut ReadinessAggregates,
    corpus_kind: &str,
    expected_verdict: Option<&str>,
    actual_verdict: Option<&str>,
    mutation_family: Option<&str>,
    metadata: &Value,
) {
    if expected_verdict == Some("rejected") {
        aggregates.adjudicated_count += 1;
        if actual_verdict == Some("rejected") {
            aggregates.correct_rejection_count += 1;
        }
        if actual_verdict == Some("approved") {
            aggregates.false_pass_count += 1;
        }
        if let Some(family) = mutation_family.filter(|family| !family.trim().is_empty()) {
            aggregates
                .covered_failure_families
                .insert(family.to_owned());
        }
    }
    if expected_verdict == Some("approved") {
        aggregates.audited_approvals_count += 1;
    }
    if matches!(corpus_kind, "mutated" | "pairwise") {
        aggregates.challenge_total += 1;
        if expected_verdict == actual_verdict {
            aggregates.challenge_passed += 1;
        }
        if expected_verdict == Some("rejected")
            && actual_verdict == Some("approved")
            && metadata_bool(metadata, "critical")
        {
            aggregates.critical_mutated_bad_approved = true;
        }
    }
    if corpus_kind == "override" {
        if metadata_bool(metadata, "operator_reviewed_rejection")
            || expected_verdict == Some("rejected")
        {
            aggregates.override_reviewed_rejections += 1;
        }
        if metadata_bool(metadata, "operator_overrode_rejection")
            || actual_verdict == Some("approved")
        {
            aggregates.operator_overrides += 1;
        }
    }
    if metadata_bool(metadata, "anomaly_conditioned") {
        aggregates.anomaly_samples += 1;
        if metadata_bool(metadata, "validator_error") {
            aggregates.anomaly_errors += 1;
        }
    }
}

fn evaluate_demotion_triggers(
    config: &CalibrationSliceConfig,
    aggregates: &ReadinessAggregates,
) -> Vec<DemotionTrigger> {
    let mut triggers = Vec::new();
    let thresholds = config.demotion_thresholds;
    if aggregates.override_reviewed_rejections >= thresholds.override_reviewed_rejections_min {
        let override_rate_bps = rate_bps(
            aggregates.operator_overrides,
            aggregates.override_reviewed_rejections,
        );
        if override_rate_bps >= thresholds.override_rate_bps_min {
            triggers.push(DemotionTrigger {
                kind: DemotionTriggerKind::OverrideSpike,
                reason: "operator override rate spike".to_owned(),
                details: json!({
                    "operator_reviewed_rejections": aggregates.override_reviewed_rejections,
                    "operator_overrides": aggregates.operator_overrides,
                    "operator_override_rate_bps": override_rate_bps,
                    "threshold_bps": thresholds.override_rate_bps_min
                }),
            });
        }
    }
    let challenge_pass_rate_bps = rate_bps(aggregates.challenge_passed, aggregates.challenge_total);
    if aggregates.critical_mutated_bad_approved
        || (aggregates.challenge_total > 0
            && challenge_pass_rate_bps < thresholds.challenge_pass_rate_min_bps)
    {
        triggers.push(DemotionTrigger {
            kind: DemotionTriggerKind::ChallengeRegression,
            reason: "challenge bank regression".to_owned(),
            details: json!({
                "critical_mutated_bad_approved": aggregates.critical_mutated_bad_approved,
                "challenge_pass_rate_bps": challenge_pass_rate_bps,
                "threshold_bps": thresholds.challenge_pass_rate_min_bps
            }),
        });
    }
    if aggregates.anomaly_samples >= thresholds.anomaly_samples_min {
        let anomaly_error_rate_bps =
            rate_bps(aggregates.anomaly_errors, aggregates.anomaly_samples);
        if anomaly_error_rate_bps >= thresholds.anomaly_error_rate_bps_min {
            triggers.push(DemotionTrigger {
                kind: DemotionTriggerKind::AnomalyValidatorError,
                reason: "anomaly-conditioned validator error rate".to_owned(),
                details: json!({
                    "anomaly_samples": aggregates.anomaly_samples,
                    "anomaly_errors": aggregates.anomaly_errors,
                    "anomaly_error_rate_bps": anomaly_error_rate_bps,
                    "threshold_bps": thresholds.anomaly_error_rate_bps_min
                }),
            });
        }
    }
    if !overlay_compatible(config) {
        triggers.push(DemotionTrigger {
            kind: DemotionTriggerKind::CompatibleRangeExit,
            reason: "worker behavior version outside validator overlay compatible range".to_owned(),
            details: json!({
                "worker_behavior_version": config.worker_behavior_version,
                "validator_overlay_compatible_versions": config.validator_overlay_compatible_versions
            }),
        });
    }
    triggers
}

fn missing_failure_families(
    config: &CalibrationSliceConfig,
    aggregates: &ReadinessAggregates,
) -> BTreeSet<String> {
    config
        .dominant_failure_families
        .difference(&aggregates.covered_failure_families)
        .cloned()
        .collect()
}

fn overlay_compatible(config: &CalibrationSliceConfig) -> bool {
    config.validator_overlay_compatible_versions.is_empty()
        || config
            .validator_overlay_compatible_versions
            .contains(&config.worker_behavior_version)
}

#[allow(clippy::too_many_arguments)]
fn persist_readiness(
    conn: &Connection,
    config: &CalibrationSliceConfig,
    evaluated_at: &str,
    enforcement_state: EnforcementReadinessState,
    wilson_lower_bound_bps: i64,
    false_pass_upper_bound_bps: i64,
    adjudicated_count: i64,
    audited_approvals_count: i64,
    dominant_failure_families: &[String],
    missing_failure_families: &[String],
    demotion_reason: Option<&str>,
    active_demotion_trigger_json: Option<&Value>,
) -> rusqlite::Result<()> {
    let dominant_failure_families_json =
        to_json_string(dominant_failure_families, "dominant_failure_families")?;
    let missing_failure_families_json =
        to_json_string(missing_failure_families, "missing_failure_families")?;
    let thresholds_json = to_json_string(&config.demotion_thresholds, "demotion_thresholds")?;
    let active_trigger_json = active_demotion_trigger_json.map(Value::to_string);
    conn.execute(
        "\
        INSERT INTO enforcement_readiness (
          slice_id, worker_skill_id, source_id, worker_behavior_version,
          validator_overlay_version, enforcement_state, last_evaluated_at,
          wilson_lower_bound_bps, false_pass_upper_bound_bps,
          adjudicated_count, audited_approvals_count,
          dominant_failure_families_json, missing_failure_families_json,
          demotion_reason, demotion_thresholds_json, active_demotion_trigger_json,
          updated_at
        ) VALUES (
          ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?7
        )
        ON CONFLICT(slice_id) DO UPDATE SET
          worker_skill_id = excluded.worker_skill_id,
          source_id = excluded.source_id,
          worker_behavior_version = excluded.worker_behavior_version,
          validator_overlay_version = excluded.validator_overlay_version,
          enforcement_state = excluded.enforcement_state,
          last_evaluated_at = excluded.last_evaluated_at,
          wilson_lower_bound_bps = excluded.wilson_lower_bound_bps,
          false_pass_upper_bound_bps = excluded.false_pass_upper_bound_bps,
          adjudicated_count = excluded.adjudicated_count,
          audited_approvals_count = excluded.audited_approvals_count,
          dominant_failure_families_json = excluded.dominant_failure_families_json,
          missing_failure_families_json = excluded.missing_failure_families_json,
          demotion_reason = excluded.demotion_reason,
          demotion_thresholds_json = excluded.demotion_thresholds_json,
          active_demotion_trigger_json = excluded.active_demotion_trigger_json,
          updated_at = excluded.updated_at
        ",
        params![
            config.slice_id,
            config.worker_skill_id,
            config.source_id,
            config.worker_behavior_version,
            config.validator_overlay_version,
            enforcement_state.as_str(),
            evaluated_at,
            wilson_lower_bound_bps,
            false_pass_upper_bound_bps,
            adjudicated_count,
            audited_approvals_count,
            dominant_failure_families_json,
            missing_failure_families_json,
            demotion_reason,
            thresholds_json,
            active_trigger_json,
        ],
    )?;
    Ok(())
}

#[must_use]
const fn rate_bps(numerator: u32, denominator: u32) -> u32 {
    if denominator == 0 {
        return 0;
    }
    numerator.saturating_mul(10_000) / denominator
}

#[must_use]
const fn ceil_div(numerator: u32, denominator: u32) -> u32 {
    numerator.div_ceil(denominator)
}

#[must_use]
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn wilson_upper_bound_bps(events: u32, total: u32) -> i64 {
    if total == 0 {
        return 10_000;
    }
    let event_count = f64::from(events.min(total));
    let samples = f64::from(total);
    let phat = event_count / samples;
    let z2 = WILSON_Z_95 * WILSON_Z_95;
    let denominator = 1.0 + z2 / samples;
    let center = phat + z2 / (2.0 * samples);
    let margin = WILSON_Z_95 * (phat.mul_add(1.0 - phat, z2 / (4.0 * samples)) / samples).sqrt();
    ((center + margin) / denominator * BPS_SCALE)
        .ceil()
        .clamp(0.0, BPS_SCALE) as i64
}

fn metadata_bool(metadata: &Value, key: &str) -> bool {
    metadata.get(key).and_then(Value::as_bool).unwrap_or(false)
}

fn parse_metadata(raw: &str, column_index: usize) -> rusqlite::Result<Value> {
    serde_json::from_str(raw).map_err(|err| {
        rusqlite::Error::FromSqlConversionFailure(column_index, Type::Text, Box::new(err))
    })
}

fn to_json_string<T: Serialize + ?Sized>(value: &T, label: &str) -> rusqlite::Result<String> {
    serde_json::to_string(value).map_err(|err| {
        rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("failed to serialize {label}: {err}"),
        )))
    })
}
