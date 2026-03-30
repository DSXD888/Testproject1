#![forbid(unsafe_code)]
#![allow(clippy::duplicate_mod)]

#[path = "../src/validation/policy.rs"]
#[allow(dead_code)]
mod policy_impl;
mod support;

mod tests {
    use std::collections::BTreeSet;
    use std::fs;
    use std::path::PathBuf;

    use anyhow::{Result, anyhow, ensure};

    use crate::policy_impl::{
        BOOTSTRAP_RUN_TARGET, BasisSufficiency, CALIBRATING_VALIDATION_BPS, DemotionTrigger,
        DryRunLadderPhase, FULL_VALIDATION_BPS, SkillPromotionStage, SourceHealthState,
        ValidationPolicyConfig, ValidationPolicyContext, ValidationPolicyMode, ValidationSliceKey,
        ValidationTrigger, derive_dry_run_phase, evaluate_validation_policy,
    };
    use crate::support::validation::{
        SourceHealthState as FixtureSourceHealthState, ValidationPolicyFixture,
        ValidationPolicyMode as FixtureValidationPolicyMode,
        ValidationTrigger as FixtureValidationTrigger,
    };

    fn manifest_root() -> Result<PathBuf> {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .canonicalize()
            .map_err(Into::into)
    }

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

    fn reason_detail<'a>(
        decision: &'a crate::policy_impl::ValidationDecision,
        key: &str,
    ) -> Result<&'a serde_json::Value> {
        decision
            .reasons
            .iter()
            .find(|reason| reason.key == key)
            .map(|reason| &reason.detail)
            .ok_or_else(|| anyhow!("missing reason `{key}`"))
    }

    #[test]
    fn l3_t_validation_policy_001_schema_invalid_outputs_never_spawn_semantic_validation()
    -> Result<()> {
        let mut ctx = context();
        ctx.schema_valid = false;

        let decision = evaluate_validation_policy(&config(ValidationPolicyMode::Always), &ctx);

        ensure!(!decision.validation_required);
        ensure!(!decision.oqg_required);
        ensure!(reason_detail(&decision, "schema_invalid_output_contract").is_ok());
        Ok(())
    }

    #[test]
    fn l3_t_validation_policy_002_pre_stable_stages_force_validation() -> Result<()> {
        for stage in [
            SkillPromotionStage::Draft,
            SkillPromotionStage::Candidate,
            SkillPromotionStage::Canary,
            SkillPromotionStage::Shadow,
        ] {
            let mut ctx = context();
            ctx.promotion_stage = stage;

            let decision = evaluate_validation_policy(&config(ValidationPolicyMode::Skip), &ctx);

            ensure!(decision.validation_required);
            ensure!(reason_detail(&decision, "pre_stable_promotion_stage").is_ok());
        }
        Ok(())
    }

    #[test]
    fn l3_t_validation_policy_003_policy_always_validates_every_eligible_run() -> Result<()> {
        let decision =
            evaluate_validation_policy(&config(ValidationPolicyMode::Always), &context());

        ensure!(decision.validation_required);
        ensure!(decision.effective_validation_probability_bps == FULL_VALIDATION_BPS);
        ensure!(reason_detail(&decision, "validation_policy_always").is_ok());
        Ok(())
    }

    #[test]
    fn l3_t_validation_policy_004_sample_uses_max_rate_and_records_reason() -> Result<()> {
        let mut ctx = context();
        ctx.base_risk_multiplier_bps = 4_000;
        ctx.sample_roll_bps = Some(3_000);

        let decision = evaluate_validation_policy(&config(ValidationPolicyMode::Sample), &ctx);
        let detail = reason_detail(&decision, "validation_policy_sample")?;

        ensure!(decision.validation_required);
        ensure!(decision.effective_validation_probability_bps == 4_000);
        ensure!(detail["validation_sample_rate_bps"] == 2_500);
        ensure!(detail["risk_multiplier_bps"] == 4_000);
        Ok(())
    }

    #[test]
    fn l3_t_validation_policy_005_source_health_amber_or_red_forces_validation() -> Result<()> {
        for state in [SourceHealthState::Amber, SourceHealthState::Red] {
            let mut ctx = context();
            ctx.source_health = state;

            let decision = evaluate_validation_policy(&config(ValidationPolicyMode::Sample), &ctx);

            ensure!(decision.validation_required);
            ensure!(decision.risk_multiplier_bps == FULL_VALIDATION_BPS);
            ensure!(
                decision
                    .trigger_hits
                    .contains(&ValidationTrigger::SourceHealthAmberOrRed)
            );
        }
        Ok(())
    }

    #[test]
    fn l3_t_validation_policy_006_unknown_fingerprint_forces_validation() -> Result<()> {
        let mut ctx = context();
        ctx.page_fingerprint_unknown = true;

        let decision = evaluate_validation_policy(&config(ValidationPolicyMode::Sample), &ctx);

        ensure!(decision.validation_required);
        ensure!(decision.risk_multiplier_bps == FULL_VALIDATION_BPS);
        ensure!(
            decision
                .trigger_hits
                .contains(&ValidationTrigger::PageFingerprintUnknown)
        );
        Ok(())
    }

    #[test]
    fn l3_t_validation_policy_007_first_run_after_behavior_change_forces_validation() -> Result<()>
    {
        let mut ctx = context();
        ctx.first_run_after_worker_version_change = true;

        let decision = evaluate_validation_policy(&config(ValidationPolicyMode::Sample), &ctx);

        ensure!(decision.validation_required);
        ensure!(
            decision
                .trigger_hits
                .contains(&ValidationTrigger::FirstRunAfterVersionChange)
        );
        Ok(())
    }

    #[test]
    fn l3_t_validation_policy_008_continuation_and_locator_fallback_force_validation() -> Result<()>
    {
        let mut chain_ctx = context();
        chain_ctx.continuation_chain_length = 2;
        let chain_decision =
            evaluate_validation_policy(&config(ValidationPolicyMode::Sample), &chain_ctx);
        ensure!(chain_decision.validation_required);
        ensure!(reason_detail(&chain_decision, "continuation_chain_length_gt_1").is_ok());

        let mut fallback_ctx = context();
        fallback_ctx.locator_fallback_depth = 2;
        let fallback_decision =
            evaluate_validation_policy(&config(ValidationPolicyMode::Sample), &fallback_ctx);
        ensure!(fallback_decision.validation_required);
        ensure!(reason_detail(&fallback_decision, "high_locator_fallback_depth").is_ok());
        Ok(())
    }

    #[test]
    fn l3_t_validation_policy_009_low_basis_and_validator_error_spike_force_validation()
    -> Result<()> {
        let mut low_basis_ctx = context();
        low_basis_ctx.recent_basis_sufficiency = Some(BasisSufficiency::Low);
        let low_basis_decision =
            evaluate_validation_policy(&config(ValidationPolicyMode::Sample), &low_basis_ctx);
        ensure!(low_basis_decision.validation_required);
        ensure!(reason_detail(&low_basis_decision, "recent_basis_sufficiency_low").is_ok());

        let mut spike_ctx = context();
        spike_ctx.validator_error_spike_same_slice = true;
        let spike_decision =
            evaluate_validation_policy(&config(ValidationPolicyMode::Sample), &spike_ctx);
        ensure!(spike_decision.validation_required);
        ensure!(reason_detail(&spike_decision, "validator_error_spike_same_slice").is_ok());
        Ok(())
    }

    #[test]
    fn l3_t_validation_policy_010_high_value_and_suspect_output_force_validation() -> Result<()> {
        let mut high_value_ctx = context();
        high_value_ctx.high_value_opportunity = true;
        let high_value_decision =
            evaluate_validation_policy(&config(ValidationPolicyMode::Sample), &high_value_ctx);
        ensure!(high_value_decision.validation_required);
        ensure!(reason_detail(&high_value_decision, "high_value_opportunity").is_ok());

        let mut suspect_ctx = context();
        suspect_ctx.suspect_output_count_last_five = 2;
        let suspect_decision =
            evaluate_validation_policy(&config(ValidationPolicyMode::Sample), &suspect_ctx);
        let detail = reason_detail(&suspect_decision, "suspect_output_in_last_5")?;
        ensure!(suspect_decision.validation_required);
        ensure!(detail["count"] == 2);
        Ok(())
    }

    #[test]
    fn l3_t_validation_policy_011_trigger_mode_validates_only_on_declared_triggers() -> Result<()> {
        let no_trigger =
            evaluate_validation_policy(&config(ValidationPolicyMode::Trigger), &context());
        ensure!(!no_trigger.validation_required);

        let mut triggered_ctx = context();
        triggered_ctx.fill_rate_drop_bps = Some(600);
        let triggered =
            evaluate_validation_policy(&config(ValidationPolicyMode::Trigger), &triggered_ctx);
        ensure!(triggered.validation_required);
        ensure!(
            triggered
                .trigger_hits
                .contains(&ValidationTrigger::FillRateDrop)
        );
        Ok(())
    }

    #[test]
    fn l3_t_validation_policy_012_skip_mode_keeps_oqg_and_allows_overrides() -> Result<()> {
        let skipped = evaluate_validation_policy(&config(ValidationPolicyMode::Skip), &context());
        ensure!(!skipped.validation_required);
        ensure!(skipped.oqg_required);

        let mut override_ctx = context();
        override_ctx.high_value_opportunity = true;
        let overridden =
            evaluate_validation_policy(&config(ValidationPolicyMode::Skip), &override_ctx);
        ensure!(overridden.validation_required);
        ensure!(overridden.oqg_required);
        Ok(())
    }

    #[test]
    fn l3_t_validation_policy_013_trigger_reasons_are_structured() -> Result<()> {
        let mut ctx = context();
        ctx.fill_rate_drop_bps = Some(700);

        let decision = evaluate_validation_policy(&config(ValidationPolicyMode::Trigger), &ctx);
        let detail = reason_detail(&decision, "fill_rate_drop_gt_threshold")?;

        ensure!(detail["fill_rate_drop_bps"] == 700);
        ensure!(detail["fill_rate_drop_threshold_bps"] == 500);
        Ok(())
    }

    #[test]
    fn l3_t_validation_policy_014_slice_key_is_exact_and_cosmetic_edits_do_not_reset() -> Result<()>
    {
        let cfg = config(ValidationPolicyMode::Sample);
        let decision = evaluate_validation_policy(&cfg, &context());

        ensure!(
            decision.slice_key
                == ValidationSliceKey {
                    worker_skill_id: "procurement-ingestion/fts".to_owned(),
                    source_id: "fts".to_owned(),
                    worker_behavior_version: "worker.v1".to_owned(),
                    validator_behavior_version: "validator.v1".to_owned(),
                }
        );
        ensure!(decision.enforcement.overlay_compatible);
        Ok(())
    }

    #[test]
    fn l3_t_validation_policy_015_demotion_is_immediate() -> Result<()> {
        let mut ctx = context();
        ctx.calibration_thresholds_met = true;
        ctx.operator_override_spike = true;

        let decision = evaluate_validation_policy(&config(ValidationPolicyMode::Always), &ctx);

        ensure!(!decision.enforcement.enforcement_eligible);
        ensure!(
            decision
                .enforcement
                .demotion_triggers
                .contains(&DemotionTrigger::OperatorOverrideSpike)
        );
        Ok(())
    }

    #[test]
    fn l3_t_validation_policy_016_dry_run_ladder_uses_exact_rates() -> Result<()> {
        let bootstrap = derive_dry_run_phase(&context());
        ensure!(bootstrap == DryRunLadderPhase::Bootstrap);
        ensure!(bootstrap.validation_rate_bps(false) == FULL_VALIDATION_BPS);

        let mut calibrating_ctx = context();
        calibrating_ctx.bootstrap_completed_runs = BOOTSTRAP_RUN_TARGET;
        let calibrating = derive_dry_run_phase(&calibrating_ctx);
        ensure!(calibrating == DryRunLadderPhase::Calibrating);
        ensure!(calibrating.validation_rate_bps(false) == CALIBRATING_VALIDATION_BPS);
        ensure!(calibrating.validation_rate_bps(true) == FULL_VALIDATION_BPS);

        let mut signature_limited_ctx = calibrating_ctx;
        signature_limited_ctx.signature_limited_ready = true;
        ensure!(
            derive_dry_run_phase(&signature_limited_ctx)
                == DryRunLadderPhase::SignatureLimitedEnforcement
        );

        let mut full_ctx = signature_limited_ctx;
        full_ctx.full_slice_enforcement_ready = true;
        ensure!(derive_dry_run_phase(&full_ctx) == DryRunLadderPhase::FullSliceEnforcement);
        Ok(())
    }

    #[test]
    fn l3_t_validation_policy_017_fill_rate_drop_trigger_records_delta() -> Result<()> {
        let mut ctx = context();
        ctx.fill_rate_drop_bps = Some(900);

        let decision = evaluate_validation_policy(&config(ValidationPolicyMode::Trigger), &ctx);
        let detail = reason_detail(&decision, "fill_rate_drop_gt_threshold")?;

        ensure!(decision.validation_required);
        ensure!(detail["fill_rate_drop_bps"] == 900);
        Ok(())
    }

    #[test]
    fn l3_t_validation_policy_018_operator_requested_forces_validation_even_in_skip_mode()
    -> Result<()> {
        let mut ctx = context();
        ctx.operator_requested = true;

        let decision = evaluate_validation_policy(&config(ValidationPolicyMode::Skip), &ctx);
        let detail = reason_detail(&decision, "operator_requested")?;

        ensure!(decision.validation_required);
        ensure!(detail["source"] == "operator_requested");
        Ok(())
    }

    #[test]
    fn validation_policy_support_fixture_applies_runtime_like_inputs() -> Result<()> {
        let fixture = ValidationPolicyFixture::sample(FixtureValidationPolicyMode::Skip)
            .with_operator_requested()
            .with_source_health(FixtureSourceHealthState::Amber);
        let decision = fixture.evaluate();

        ensure!(decision.validation_required);
        ensure!(
            decision
                .trigger_hits
                .contains(&FixtureValidationTrigger::OperatorRequested)
        );
        Ok(())
    }

    #[test]
    fn ci_l3_008_validation_policy_requirement_ids_have_one_authoritative_home() -> Result<()> {
        let crate_root = manifest_root()?;
        let unit_source = fs::read_to_string(crate_root.join("src/validation/policy.rs"))?;
        let integration_source = fs::read_to_string(crate_root.join("tests/validation_policy.rs"))?;

        for id in 1..=18 {
            let public_name = format!("fn l3_t_validation_policy_{id:03}_");
            let internal_name = format!("fn ut_validation_policy_{id:03}_");
            ensure!(
                unit_source.matches(&public_name).count() == 0,
                "CI-L3-008 duplicate public id still present in src/validation/policy.rs: {public_name}"
            );
            ensure!(
                unit_source.matches(&internal_name).count() == 1,
                "CI-L3-008 expected one internal unit-test home in src/validation/policy.rs: {internal_name}"
            );
            ensure!(
                integration_source.matches(&public_name).count() == 1,
                "CI-L3-008 expected one authoritative public test home in tests/validation_policy.rs: {public_name}"
            );
        }

        Ok(())
    }
}
