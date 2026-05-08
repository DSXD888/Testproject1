#![forbid(unsafe_code)]
#![allow(clippy::duplicate_mod)]

#[path = "../src/validation/calibration.rs"]
#[allow(dead_code)]
mod calibration_impl;
#[path = "../src/validation/policy.rs"]
#[allow(dead_code)]
mod policy_impl;
mod support;

mod tests {
    use std::collections::BTreeSet;
    use std::fs;
    use std::path::PathBuf;

    use anyhow::{Result, anyhow, ensure};
    use serde_json::json;

    use crate::calibration_impl::{
        CalibrationSliceConfig, DEFAULT_DEMOTION_THRESHOLDS,
        EnforcementReadinessState as CalibrationReadinessState, recompute_enforcement_readiness,
        rule_of_three_false_pass_upper_bound_bps, wilson_lower_bound_bps,
    };
    use crate::policy_impl::{
        BOOTSTRAP_RUN_TARGET, BasisSufficiency, CALIBRATING_VALIDATION_BPS, DemotionTrigger,
        DryRunLadderPhase, EnforcementReadinessState, FULL_VALIDATION_BPS, SkillPromotionStage,
        SourceHealthState, ValidationPolicyConfig, ValidationPolicyContext, ValidationPolicyMode,
        ValidationSliceKey, ValidationTrigger, derive_dry_run_phase, derive_slice_id,
        evaluate_slice_eligibility, evaluate_validation_policy,
        evaluate_validation_policy_with_readiness, read_enforcement_state,
    };
    use crate::support::db::{DEFAULT_SCHEMA_EPOCH, StateDb};
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

    fn seed_eligible_readiness(
        conn: &rusqlite::Connection,
        config: &ValidationPolicyConfig,
    ) -> Result<String> {
        let slice_id = derive_slice_id(&ValidationSliceKey {
            worker_skill_id: config.worker_skill_id.clone(),
            source_id: config.source_id.clone(),
            worker_behavior_version: config.worker_behavior_version.clone(),
            validator_behavior_version: config.validator_behavior_version.clone(),
        });
        conn.execute(
            "\
            INSERT INTO enforcement_readiness (
              slice_id, worker_skill_id, source_id, worker_behavior_version,
              validator_overlay_version, enforcement_state, last_evaluated_at,
              wilson_lower_bound_bps, false_pass_upper_bound_bps,
              adjudicated_count, audited_approvals_count,
              dominant_failure_families_json, missing_failure_families_json,
              demotion_thresholds_json
            ) VALUES (
              ?1, ?2, ?3, ?4, ?5, 'eligible', '2026-05-08T00:00:00Z',
              7100, 500, 30, 60,
              '[\"price_mismatch\",\"missing_lot\"]', '[]',
              '{\"wilson_lower_bound_bps\":7000,\"false_pass_upper_bound_bps\":500}'
            )
            ",
            rusqlite::params![
                &slice_id,
                &config.worker_skill_id,
                &config.source_id,
                &config.worker_behavior_version,
                &config.validator_behavior_version,
            ],
        )?;
        Ok(slice_id)
    }

    fn calibration_config(
        compatible_versions: impl IntoIterator<Item = &'static str>,
        dominant_families: impl IntoIterator<Item = &'static str>,
    ) -> CalibrationSliceConfig {
        let cfg = config(ValidationPolicyMode::Sample);
        let slice_id = derive_slice_id(&ValidationSliceKey {
            worker_skill_id: cfg.worker_skill_id.clone(),
            source_id: cfg.source_id.clone(),
            worker_behavior_version: cfg.worker_behavior_version.clone(),
            validator_behavior_version: cfg.validator_behavior_version.clone(),
        });
        CalibrationSliceConfig {
            slice_id,
            worker_skill_id: cfg.worker_skill_id,
            source_id: cfg.source_id,
            worker_behavior_version: cfg.worker_behavior_version,
            validator_overlay_version: cfg.validator_behavior_version,
            validator_overlay_compatible_versions: compatible_versions
                .into_iter()
                .map(str::to_owned)
                .collect(),
            dominant_failure_families: dominant_families.into_iter().map(str::to_owned).collect(),
            demotion_thresholds: DEFAULT_DEMOTION_THRESHOLDS,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn seed_calibration_case(
        conn: &rusqlite::Connection,
        case_id: &str,
        slice_id: &str,
        corpus_kind: &str,
        expected_verdict: &str,
        actual_verdict: &str,
        mutation_family: Option<&str>,
        metadata: &serde_json::Value,
    ) -> Result<()> {
        conn.execute(
            "\
            INSERT INTO validation_calibration_cases (
              case_id, slice_id, corpus_kind, source_artifact_path,
              expected_verdict, actual_verdict, ground_truth_label,
              mutation_family, metadata_json, adjudicated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'known', ?7, ?8, '2026-05-08T00:00:00Z')
            ",
            rusqlite::params![
                case_id,
                slice_id,
                corpus_kind,
                format!("tests/fixtures/calibration/{case_id}.json"),
                expected_verdict,
                actual_verdict,
                mutation_family,
                metadata.to_string(),
            ],
        )?;
        Ok(())
    }

    fn seed_eligible_calibration_evidence(
        conn: &rusqlite::Connection,
        cfg: &CalibrationSliceConfig,
    ) -> Result<()> {
        for idx in 0..26 {
            seed_calibration_case(
                conn,
                &format!("reject-correct-{idx}"),
                &cfg.slice_id,
                "mutated",
                "rejected",
                "rejected",
                Some(if idx % 2 == 0 {
                    "price_mismatch"
                } else {
                    "missing_lot"
                }),
                &json!({}),
            )?;
        }
        for idx in 26..30 {
            seed_calibration_case(
                conn,
                &format!("reject-review-{idx}"),
                &cfg.slice_id,
                "mutated",
                "rejected",
                "needs_review",
                Some(if idx % 2 == 0 {
                    "price_mismatch"
                } else {
                    "missing_lot"
                }),
                &json!({}),
            )?;
        }
        for idx in 0..60 {
            seed_calibration_case(
                conn,
                &format!("approved-clean-{idx}"),
                &cfg.slice_id,
                "clean",
                "approved",
                "approved",
                None,
                &json!({}),
            )?;
        }
        Ok(())
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
    #[doc = "L4-T-sqlite_supabase_sync-050"]
    fn l4_t_sqlite_supabase_sync_050_policy_reads_eligible_slice_and_selects_enforcement()
    -> Result<()> {
        let temp = tempfile::tempdir()?;
        let db_path = temp.path().join("state.db");
        let state_db = StateDb::open(&db_path)?;
        ensure!(state_db.schema_epoch() == DEFAULT_SCHEMA_EPOCH);
        let cfg = config(ValidationPolicyMode::Skip);
        let slice_id = seed_eligible_readiness(state_db.connection(), &cfg)?;

        let readiness = read_enforcement_state(state_db.connection(), &slice_id)?
            .ok_or_else(|| anyhow!("missing seeded enforcement readiness row"))?;
        let eligibility = evaluate_slice_eligibility(&cfg, &readiness);
        let decision =
            evaluate_validation_policy_with_readiness(&cfg, &context(), Some(&readiness));

        ensure!(readiness.enforcement_state == EnforcementReadinessState::Eligible);
        ensure!(eligibility.enforcement_eligible);
        ensure!(eligibility.overlay_compatible);
        ensure!(eligibility.adjudicated_threshold_met);
        ensure!(eligibility.audited_approvals_threshold_met);
        ensure!(eligibility.wilson_lower_bound_met);
        ensure!(eligibility.false_pass_upper_bound_met);
        ensure!(eligibility.dominant_failure_family_coverage_met);
        ensure!(eligibility.no_active_demotion_trigger);
        ensure!(decision.enforcement.enforcement_eligible);
        ensure!(decision.dry_run_phase == DryRunLadderPhase::FullSliceEnforcement);
        ensure!(slice_id == "procurement-ingestion/fts::fts::worker.v1::validator.v1");
        Ok(())
    }

    #[test]
    #[doc = "L4-T-sqlite_supabase_sync-057"]
    fn l4_t_sqlite_supabase_sync_057_policy_readiness_survives_reopen() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let db_path = temp.path().join("state.db");
        let cfg = config(ValidationPolicyMode::Sample);
        let slice_id = {
            let state_db = StateDb::open(&db_path)?;
            seed_eligible_readiness(state_db.connection(), &cfg)?
        };

        let reopened = StateDb::open(&db_path)?;
        let readiness = read_enforcement_state(reopened.connection(), &slice_id)?
            .ok_or_else(|| anyhow!("missing persisted enforcement readiness after reopen"))?;
        let decision =
            evaluate_validation_policy_with_readiness(&cfg, &context(), Some(&readiness));

        ensure!(readiness.slice_id == slice_id);
        ensure!(readiness.enforcement_state == EnforcementReadinessState::Eligible);
        ensure!(decision.enforcement.enforcement_eligible);
        ensure!(decision.dry_run_phase == DryRunLadderPhase::FullSliceEnforcement);
        Ok(())
    }

    #[test]
    #[doc = "L4-T-sqlite_supabase_sync-075"]
    fn l4_t_sqlite_supabase_sync_075_calibration_math_boundary_cases() -> Result<()> {
        let lower_26_of_30 = wilson_lower_bound_bps(26, 30);
        ensure!(
            (7030..=7040).contains(&lower_26_of_30),
            "26/30 Wilson lower bound should be near 7030-7040 bps, got {lower_26_of_30}"
        );
        ensure!(wilson_lower_bound_bps(25, 30) < 7000);
        ensure!(rule_of_three_false_pass_upper_bound_bps(0, 60) == 500);
        ensure!(rule_of_three_false_pass_upper_bound_bps(0, 59) > 500);
        Ok(())
    }

    #[test]
    #[doc = "L4-T-sqlite_supabase_sync-051"]
    fn l4_t_sqlite_supabase_sync_051_missing_family_coverage_keeps_slice_dry_run() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let db_path = temp.path().join("state.db");
        let state_db = StateDb::open(&db_path)?;
        let cfg = calibration_config(["worker.v1"], ["price_mismatch", "missing_lot"]);
        for idx in 0..30 {
            seed_calibration_case(
                state_db.connection(),
                &format!("reject-price-only-{idx}"),
                &cfg.slice_id,
                "mutated",
                "rejected",
                "rejected",
                Some("price_mismatch"),
                &json!({}),
            )?;
        }
        for idx in 0..60 {
            seed_calibration_case(
                state_db.connection(),
                &format!("approved-missing-coverage-{idx}"),
                &cfg.slice_id,
                "clean",
                "approved",
                "approved",
                None,
                &json!({}),
            )?;
        }

        let snapshot =
            recompute_enforcement_readiness(state_db.connection(), &cfg, "2026-05-08T00:00:00Z")?;

        ensure!(snapshot.enforcement_state == CalibrationReadinessState::DryRun);
        ensure!(snapshot.missing_failure_families == vec!["missing_lot"]);
        Ok(())
    }

    #[test]
    #[doc = "L4-T-sqlite_supabase_sync-052"]
    fn l4_t_sqlite_supabase_sync_052_wilson_below_threshold_blocks_enforcement() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let db_path = temp.path().join("state.db");
        let state_db = StateDb::open(&db_path)?;
        let cfg = calibration_config(["worker.v1"], ["price_mismatch", "missing_lot"]);
        for idx in 0..25 {
            seed_calibration_case(
                state_db.connection(),
                &format!("reject-correct-low-wilson-{idx}"),
                &cfg.slice_id,
                "live",
                "rejected",
                "rejected",
                Some(if idx % 2 == 0 {
                    "price_mismatch"
                } else {
                    "missing_lot"
                }),
                &json!({}),
            )?;
        }
        for idx in 25..30 {
            seed_calibration_case(
                state_db.connection(),
                &format!("reject-review-low-wilson-{idx}"),
                &cfg.slice_id,
                "live",
                "rejected",
                "needs_review",
                Some(if idx % 2 == 0 {
                    "price_mismatch"
                } else {
                    "missing_lot"
                }),
                &json!({}),
            )?;
        }
        for idx in 0..60 {
            seed_calibration_case(
                state_db.connection(),
                &format!("approved-low-wilson-{idx}"),
                &cfg.slice_id,
                "clean",
                "approved",
                "approved",
                None,
                &json!({}),
            )?;
        }

        let snapshot =
            recompute_enforcement_readiness(state_db.connection(), &cfg, "2026-05-08T00:00:00Z")?;

        ensure!(snapshot.wilson_lower_bound_bps < 7000);
        ensure!(snapshot.enforcement_state == CalibrationReadinessState::DryRun);
        Ok(())
    }

    #[test]
    #[doc = "L4-T-sqlite_supabase_sync-053"]
    fn l4_t_sqlite_supabase_sync_053_override_spike_demotes_same_transaction() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let db_path = temp.path().join("state.db");
        let mut state_db = StateDb::open(&db_path)?;
        let cfg = calibration_config(["worker.v1"], ["price_mismatch", "missing_lot"]);
        seed_eligible_calibration_evidence(state_db.connection(), &cfg)?;

        state_db.with_write_transaction(|tx| {
            for idx in 0..10 {
                seed_calibration_case(
                    tx,
                    &format!("override-spike-{idx}"),
                    &cfg.slice_id,
                    "override",
                    "rejected",
                    "approved",
                    Some("price_mismatch"),
                    &json!({
                        "operator_reviewed_rejection": true,
                        "operator_overrode_rejection": true
                    }),
                )?;
            }
            let snapshot = recompute_enforcement_readiness(tx, &cfg, "2026-05-08T00:00:00Z")?;
            let persisted: String = tx.query_row(
                "SELECT enforcement_state FROM enforcement_readiness WHERE slice_id = ?1",
                [&cfg.slice_id],
                |row| row.get(0),
            )?;
            ensure!(snapshot.enforcement_state == CalibrationReadinessState::Demoted);
            ensure!(persisted == "demoted");
            ensure!(snapshot.active_demotion_trigger_json.is_some());
            Ok(())
        })?;
        Ok(())
    }

    #[test]
    #[doc = "L4-T-sqlite_supabase_sync-054"]
    fn l4_t_sqlite_supabase_sync_054_critical_mutated_bad_approval_demotes() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let db_path = temp.path().join("state.db");
        let state_db = StateDb::open(&db_path)?;
        let cfg = calibration_config(["worker.v1"], ["price_mismatch", "missing_lot"]);
        seed_eligible_calibration_evidence(state_db.connection(), &cfg)?;
        seed_calibration_case(
            state_db.connection(),
            "critical-mutated-approved",
            &cfg.slice_id,
            "mutated",
            "rejected",
            "approved",
            Some("price_mismatch"),
            &json!({ "critical": true }),
        )?;

        let snapshot =
            recompute_enforcement_readiness(state_db.connection(), &cfg, "2026-05-08T00:00:00Z")?;

        ensure!(snapshot.enforcement_state == CalibrationReadinessState::Demoted);
        ensure!(
            snapshot
                .active_demotion_trigger_json
                .as_ref()
                .is_some_and(|value| value.to_string().contains("challenge_regression"))
        );
        Ok(())
    }

    #[test]
    #[doc = "L4-T-sqlite_supabase_sync-055"]
    fn l4_t_sqlite_supabase_sync_055_anomaly_conditioned_validator_error_demotes() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let db_path = temp.path().join("state.db");
        let state_db = StateDb::open(&db_path)?;
        let cfg = calibration_config(["worker.v1"], ["price_mismatch", "missing_lot"]);
        seed_eligible_calibration_evidence(state_db.connection(), &cfg)?;
        for idx in 0..20 {
            seed_calibration_case(
                state_db.connection(),
                &format!("anomaly-error-{idx}"),
                &cfg.slice_id,
                "live",
                "needs_review",
                "needs_review",
                None,
                &json!({
                    "anomaly_conditioned": true,
                    "validator_error": true
                }),
            )?;
        }

        let snapshot =
            recompute_enforcement_readiness(state_db.connection(), &cfg, "2026-05-08T00:00:00Z")?;

        ensure!(snapshot.enforcement_state == CalibrationReadinessState::Demoted);
        ensure!(
            snapshot
                .active_demotion_trigger_json
                .as_ref()
                .is_some_and(|value| value.to_string().contains("anomaly_validator_error"))
        );
        Ok(())
    }

    #[test]
    #[doc = "L4-T-sqlite_supabase_sync-056"]
    fn l4_t_sqlite_supabase_sync_056_compatible_range_exit_demotes() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let db_path = temp.path().join("state.db");
        let state_db = StateDb::open(&db_path)?;
        let cfg = calibration_config(["worker.v2"], ["price_mismatch", "missing_lot"]);
        seed_eligible_calibration_evidence(state_db.connection(), &cfg)?;

        let snapshot =
            recompute_enforcement_readiness(state_db.connection(), &cfg, "2026-05-08T00:00:00Z")?;

        ensure!(snapshot.enforcement_state == CalibrationReadinessState::Demoted);
        ensure!(
            snapshot
                .active_demotion_trigger_json
                .as_ref()
                .is_some_and(|value| value.to_string().contains("compatible_range_exit"))
        );
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
