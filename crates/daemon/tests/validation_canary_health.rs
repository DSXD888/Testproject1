#![forbid(unsafe_code)]
#![allow(clippy::duplicate_mod)]

#[path = "../src/agents/contracts.rs"]
#[allow(dead_code)]
mod contracts_impl;
#[path = "support/db.rs"]
mod db_support;
#[allow(clippy::duplicate_mod)]
#[path = "../src/validation/executor.rs"]
#[allow(dead_code)]
mod executor_impl;
#[allow(clippy::duplicate_mod)]
#[path = "../src/validation/policy.rs"]
#[allow(dead_code)]
mod policy_impl;
#[path = "../src/jobs/runner.rs"]
#[allow(dead_code)]
mod runner_impl;
mod support {
    pub mod db {
        pub use crate::db_support::*;
    }

    #[path = "validation.rs"]
    pub mod validation;
}

mod jobs {
    pub mod runner {
        pub use crate::runner_impl::*;
    }
}

mod tests {
    use std::collections::BTreeSet;
    use std::path::{Path, PathBuf};

    use anyhow::{Result, anyhow, ensure};
    use chrono::{TimeZone, Utc};
    use rusqlite::{Connection, params};
    use serde_json::json;

    use crate::executor_impl::VALIDATOR_JOB_KIND;
    use crate::executor_impl::{
        ValidationBlackoutIncident, ValidationCanaryCase, ValidationHealthState,
        ValidationTelemetryWindow, ValidatorBasisSufficiency, ValidatorEnqueueRequest,
        ValidatorJobPurpose, ValidatorResultContract, ValidatorVerdict,
        blackout_incident_for_transition, canary_health_override, compute_validation_telemetry,
        derive_validation_health_state, enqueue_validation_canary_job,
    };
    use crate::policy_impl::{
        DemotionTrigger, DryRunLadderPhase, ValidationPolicyConfig, ValidationPolicyContext,
        ValidationPolicyMode, evaluate_validation_policy,
        evaluate_validation_policy_with_blind_demotion,
    };
    use crate::runner_impl::JobRunner;
    use crate::support::db::StateDb;
    use crate::support::validation::SyntheticCanaryFixture;

    fn repo_root() -> Result<PathBuf> {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(2)
            .map(Path::to_path_buf)
            .ok_or_else(|| anyhow!("failed to resolve repository root"))
    }

    fn open_state_db() -> Result<(tempfile::TempDir, StateDb)> {
        let temp = tempfile::tempdir()?;
        let db_path = temp.path().join("state.db");
        let state_db = StateDb::open(&db_path)?;
        Ok((temp, state_db))
    }

    fn seed_job(conn: &Connection, job_id: &str) -> Result<()> {
        conn.execute(
            "\
            INSERT INTO workflow_jobs (id, kind, status, input_payload)
            VALUES (?1, 'stage-job', 'pending', ?2)
            ",
            params![job_id, json!({"job_id": job_id}).to_string()],
        )?;
        Ok(())
    }

    fn seed_process(conn: &Connection, process_id: &str) -> Result<()> {
        conn.execute(
            "\
            INSERT INTO procurement_processes (
              id, current_stage, title, latest_published_at, latest_updated_at, owner_id
            ) VALUES (?1, 'manifest', 'Process title', '2026-03-30T00:00:00Z',
                      '2026-03-30T00:00:00Z', 'local-owner')
            ",
            params![process_id],
        )?;
        Ok(())
    }

    fn sample_request() -> ValidatorEnqueueRequest {
        ValidatorEnqueueRequest {
            validator_job_id: Some("validator-canary-001".to_owned()),
            idempotency_key: Some("validator-canary-idempotency-001".to_owned()),
            worker_job_id: "worker-job-001".to_owned(),
            worker_session_id: Some("worker-session-001".to_owned()),
            process_id: Some("process-001".to_owned()),
            scope_kind: Some("process".to_owned()),
            scope_id: Some("process-001".to_owned()),
            continuation_chain_id: Some("chain-001".to_owned()),
            run_manifest_path: PathBuf::from("artifacts/manifests/worker-session-001.md"),
            run_manifest_hash: "manifest-hash-001".to_owned(),
            drift_delta_json: json!({"fingerprint_delta_bps": 125}),
            oqg_anomaly_summary: vec!["duplicate_rate:low".to_owned()],
            representative_item_sample: json!({"process_id":"process-001","item":"sample"}),
            corpus_provenance_tag: Some(SyntheticCanaryFixture::corpus_tag().to_owned()),
            purpose: ValidatorJobPurpose::LiveValidation,
            canary_case: None,
            retroactive_backlog: None,
        }
    }

    fn known_good_case() -> ValidationCanaryCase {
        ValidationCanaryCase {
            case_id: SyntheticCanaryFixture::known_good_case_id().to_owned(),
            expected_verdict: Some(ValidatorVerdict::Approved),
            expected_basis_sufficiency: ValidatorBasisSufficiency::High,
        }
    }

    fn known_bad_case() -> ValidationCanaryCase {
        ValidationCanaryCase {
            case_id: SyntheticCanaryFixture::known_bad_case_id().to_owned(),
            expected_verdict: Some(ValidatorVerdict::Rejected),
            expected_basis_sufficiency: ValidatorBasisSufficiency::High,
        }
    }

    fn insufficient_basis_case() -> ValidationCanaryCase {
        ValidationCanaryCase {
            case_id: SyntheticCanaryFixture::insufficient_basis_case_id().to_owned(),
            expected_verdict: None,
            expected_basis_sufficiency: ValidatorBasisSufficiency::Low,
        }
    }

    fn approved_contract() -> ValidatorResultContract {
        ValidatorResultContract {
            verdict: ValidatorVerdict::Approved,
            basis_sufficiency: ValidatorBasisSufficiency::High,
            enforcement_eligible: true,
            confidence_bps: 9_000,
            issues: Vec::new(),
            summary: "approved".to_owned(),
        }
    }

    fn flagged_contract() -> ValidatorResultContract {
        ValidatorResultContract {
            verdict: ValidatorVerdict::Flagged,
            basis_sufficiency: ValidatorBasisSufficiency::Medium,
            enforcement_eligible: false,
            confidence_bps: 7_000,
            issues: Vec::new(),
            summary: "flagged".to_owned(),
        }
    }

    fn low_basis_contract() -> ValidatorResultContract {
        ValidatorResultContract {
            verdict: ValidatorVerdict::Flagged,
            basis_sufficiency: ValidatorBasisSufficiency::Low,
            enforcement_eligible: false,
            confidence_bps: 5_500,
            issues: Vec::new(),
            summary: "low basis".to_owned(),
        }
    }

    fn policy_config() -> ValidationPolicyConfig {
        ValidationPolicyConfig {
            worker_skill_id: "procurement-ingestion/fts".to_owned(),
            source_id: "fts".to_owned(),
            validation_policy: ValidationPolicyMode::Sample,
            validation_sample_rate: 5_000,
            validation_triggers: BTreeSet::default(),
            worker_behavior_version: "worker.v1".to_owned(),
            validator_behavior_version: "validator.v1".to_owned(),
            validator_overlay_compatible_versions: BTreeSet::default(),
        }
    }

    fn policy_context() -> ValidationPolicyContext {
        ValidationPolicyContext {
            calibration_thresholds_met: true,
            signature_limited_ready: true,
            full_slice_enforcement_ready: true,
            bootstrap_completed_runs: 50,
            ..ValidationPolicyContext::default()
        }
    }

    #[test]
    fn l3_t_validation_canary_health_001_synthetic_canaries_use_the_production_validator_path()
    -> Result<()> {
        let (_temp, mut state_db) = open_state_db()?;
        seed_process(state_db.connection(), "process-001")?;
        seed_job(state_db.connection(), "worker-job-001")?;
        let runner = JobRunner::new("validator-canary-001");

        let enqueued = enqueue_validation_canary_job(
            &repo_root()?,
            &mut state_db,
            &runner,
            sample_request(),
            known_good_case(),
        )?;

        ensure!(enqueued.skill.skill_path == "skills/validation/base/SKILL.md");
        ensure!(enqueued.skill.result_schema_path == "skills/validation/base/result.schema.json");
        ensure!(enqueued.skill.model_profile == "structured_extract_low_cost");
        ensure!(enqueued.skill.interaction_profile == "headless_contract");
        ensure!(enqueued.enqueue_request.kind == VALIDATOR_JOB_KIND);
        ensure!(
            enqueued.enqueue_request.input_payload["validator_packet"]["purpose"]
                == "synthetic_canary"
        );
        ensure!(
            enqueued.enqueue_request.input_payload["validator_packet"]["canary_case"]["case_id"]
                == "known-good"
        );
        Ok(())
    }

    #[test]
    fn l3_t_validation_canary_health_002_missing_canary_persistence_counts_as_execution_failure() {
        let telemetry = compute_validation_telemetry(
            &ValidationTelemetryWindow {
                validation_attempt_count: 4,
                execution_failure_count: 1,
                canary_run_count: 3,
                canary_success_count: 2,
                ..ValidationTelemetryWindow::default()
            },
            None,
        );

        assert_eq!(telemetry.validator_canary_pass_rate_bps, 6_666);
        assert_eq!(telemetry.validator_execution_failure_rate_bps, 2_500);
    }

    #[test]
    fn l3_t_validation_canary_health_003_failure_rate_above_2000_bps_degrades_health() {
        let state = derive_validation_health_state(
            &ValidationTelemetryWindow {
                validation_attempt_count: 11,
                execution_failure_count: 3,
                ..ValidationTelemetryWindow::default()
            },
            None,
        );

        assert_eq!(state, ValidationHealthState::Degraded);
    }

    #[test]
    fn l3_t_validation_canary_health_004_failure_rate_above_5000_bps_blinds_health() {
        let state = derive_validation_health_state(
            &ValidationTelemetryWindow {
                validation_attempt_count: 10,
                execution_failure_count: 6,
                ..ValidationTelemetryWindow::default()
            },
            None,
        );

        assert_eq!(state, ValidationHealthState::Blind);
    }

    #[test]
    fn l3_t_validation_canary_health_005_canary_misclassification_forces_health_drop_and_blackout_incident()
    -> Result<()> {
        let blind_override = canary_health_override(&known_bad_case(), &approved_contract());
        ensure!(blind_override == Some(ValidationHealthState::Blind));

        let degraded_override = canary_health_override(&known_good_case(), &flagged_contract());
        ensure!(degraded_override == Some(ValidationHealthState::Degraded));

        let low_basis_override =
            canary_health_override(&insufficient_basis_case(), &low_basis_contract());
        ensure!(low_basis_override.is_none());

        let incident: ValidationBlackoutIncident = blackout_incident_for_transition(
            ValidationHealthState::Healthy,
            ValidationHealthState::Blind,
            Utc.with_ymd_and_hms(2026, 3, 30, 1, 0, 0)
                .single()
                .ok_or_else(|| anyhow!("ts"))?,
            Utc.with_ymd_and_hms(2026, 3, 30, 1, 15, 0)
                .single()
                .ok_or_else(|| anyhow!("ts"))?,
            "known-bad approved unexpectedly",
            200,
        )
        .ok_or_else(|| anyhow!("expected blackout incident"))?;

        ensure!(incident.trust_debt_count == 200);
        ensure!(incident.reason.contains("known-bad"));
        Ok(())
    }

    #[test]
    fn l3_t_validation_canary_health_006_blind_state_auto_demotes_enforcement_ready_slices() {
        let baseline = evaluate_validation_policy(&policy_config(), &policy_context());
        assert!(baseline.enforcement.enforcement_eligible);
        assert_eq!(
            baseline.dry_run_phase,
            DryRunLadderPhase::FullSliceEnforcement
        );

        let blind = evaluate_validation_policy_with_blind_demotion(
            &policy_config(),
            &policy_context(),
            true,
        );
        assert!(!blind.enforcement.enforcement_eligible);
        assert!(
            blind
                .enforcement
                .demotion_triggers
                .contains(&DemotionTrigger::AnomalyConditionedErrorSpike)
        );
        assert_eq!(blind.dry_run_phase, DryRunLadderPhase::Calibrating);
    }

    #[test]
    fn l3_t_validation_canary_health_007_validation_telemetry_emits_all_eight_metrics() {
        let telemetry = compute_validation_telemetry(
            &ValidationTelemetryWindow {
                validation_attempt_count: 10,
                approved_count: 6,
                rejected_count: 2,
                flagged_count: 2,
                total_cost_pence: 250,
                operator_override_count: 1,
                execution_failure_count: 1,
                canary_run_count: 3,
                canary_success_count: 2,
                ..ValidationTelemetryWindow::default()
            },
            Some(ValidationHealthState::Degraded),
        );

        assert_eq!(telemetry.validator_approval_rate_bps, 6_000);
        assert_eq!(telemetry.validator_rejection_rate_bps, 2_000);
        assert_eq!(telemetry.validator_flag_rate_bps, 2_000);
        assert_eq!(telemetry.validator_cost_per_run_pence, 25);
        assert_eq!(telemetry.operator_override_rate_bps, 5_000);
        assert_eq!(telemetry.validator_execution_failure_rate_bps, 1_000);
        assert_eq!(telemetry.validator_canary_pass_rate_bps, 6_666);
        assert_eq!(telemetry.validator_health_state.as_str(), "degraded");
    }

    #[test]
    fn l3_t_validation_canary_health_008_morning_queue_surfaces_one_blackout_incident_with_trust_debt()
    -> Result<()> {
        let incident = blackout_incident_for_transition(
            ValidationHealthState::Degraded,
            ValidationHealthState::Blind,
            Utc.with_ymd_and_hms(2026, 3, 30, 2, 0, 0)
                .single()
                .ok_or_else(|| anyhow!("ts"))?,
            Utc.with_ymd_and_hms(2026, 3, 30, 9, 0, 0)
                .single()
                .ok_or_else(|| anyhow!("ts"))?,
            "overnight validator blackout",
            200,
        )
        .ok_or_else(|| anyhow!("expected incident"))?;

        ensure!(incident.trust_debt_count == 200);
        ensure!(incident.health_state == ValidationHealthState::Blind);
        Ok(())
    }
}
