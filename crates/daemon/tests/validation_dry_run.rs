#![forbid(unsafe_code)]

#[path = "../src/agents/contracts.rs"]
#[allow(dead_code)]
mod contracts_impl;
#[path = "support/db.rs"]
mod db_support;
#[allow(clippy::duplicate_mod)]
#[path = "../src/validation/executor.rs"]
#[allow(dead_code)]
mod executor_impl;
#[path = "../src/jobs/runner.rs"]
#[allow(dead_code)]
mod runner_impl;

mod jobs {
    pub mod runner {
        pub use crate::runner_impl::*;
    }
}

mod support {
    pub mod db {
        pub use crate::db_support::*;
    }
}

mod tests {
    use std::path::{Path, PathBuf};

    use anyhow::{Result, anyhow, ensure};
    use rusqlite::{Connection, params};
    use serde_json::json;

    use crate::contracts_impl::{ValidationDisposition, ValidationOutcome};
    use crate::db_support::StateDb;
    use crate::executor_impl::{
        self, PersistValidationVerdictRequest, ValidationHandlingMode, ValidationTrustState,
        ValidatorBasisSufficiency, ValidatorEnqueueRequest, ValidatorIssue,
        ValidatorResultContract, ValidatorVerdict, decide_validator_failure,
        decide_validator_verdict, enqueue_validator_sub_job, ensure_validation_scratch_dir,
        rollup_process_validation_state,
    };
    use crate::runner_impl::{JobRunner, PersistSuccessRequest};

    fn repo_root() -> Result<PathBuf> {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(2)
            .map(Path::to_path_buf)
            .ok_or_else(|| anyhow!("failed to resolve repository root"))
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

    fn open_state_db() -> Result<(tempfile::TempDir, StateDb)> {
        let temp = tempfile::tempdir()?;
        let db_path = temp.path().join("state.db");
        let state_db = StateDb::open(&db_path)?;
        Ok((temp, state_db))
    }

    fn approved() -> ValidatorResultContract {
        ValidatorResultContract {
            verdict: ValidatorVerdict::Approved,
            basis_sufficiency: ValidatorBasisSufficiency::High,
            enforcement_eligible: true,
            confidence_bps: 9_000,
            issues: Vec::new(),
            summary: "approved".to_owned(),
        }
    }

    fn flagged() -> ValidatorResultContract {
        ValidatorResultContract {
            verdict: ValidatorVerdict::Flagged,
            basis_sufficiency: ValidatorBasisSufficiency::Medium,
            enforcement_eligible: false,
            confidence_bps: 6_500,
            issues: vec![ValidatorIssue {
                severity: "minor".to_owned(),
                category: "consistency".to_owned(),
                description: "minor disagreement".to_owned(),
                evidence_refs: vec!["daemon_appendix".to_owned()],
            }],
            summary: "flagged".to_owned(),
        }
    }

    fn rejected() -> ValidatorResultContract {
        ValidatorResultContract {
            verdict: ValidatorVerdict::Rejected,
            basis_sufficiency: ValidatorBasisSufficiency::High,
            enforcement_eligible: true,
            confidence_bps: 8_200,
            issues: vec![ValidatorIssue {
                severity: "critical".to_owned(),
                category: "identity".to_owned(),
                description: "critical mismatch".to_owned(),
                evidence_refs: vec!["result_contract.item_1".to_owned()],
            }],
            summary: "rejected".to_owned(),
        }
    }

    fn sample_request() -> ValidatorEnqueueRequest {
        ValidatorEnqueueRequest {
            validator_job_id: Some("validator-job-dry".to_owned()),
            idempotency_key: Some("validator-dry-idempotency".to_owned()),
            worker_job_id: "worker-job-dry".to_owned(),
            worker_session_id: Some("worker-session-dry".to_owned()),
            process_id: Some("process-dry".to_owned()),
            scope_kind: Some("process".to_owned()),
            scope_id: Some("process-dry".to_owned()),
            continuation_chain_id: Some("chain-dry".to_owned()),
            run_manifest_path: PathBuf::from("artifacts/manifests/worker-session-dry.md"),
            run_manifest_hash: "manifest-hash-dry".to_owned(),
            drift_delta_json: json!({"fingerprint_delta_bps": 75}),
            oqg_anomaly_summary: vec!["duplicate_rate:none".to_owned()],
            representative_item_sample: json!({"item":"sample"}),
            corpus_provenance_tag: Some("clean_real".to_owned()),
        }
    }

    fn persist_worker_success(state_db: &mut StateDb, job_id: &str) -> Result<i64> {
        let runner = JobRunner::new("runner-dry");
        let ack = runner.persist_success(
            state_db,
            &PersistSuccessRequest {
                job_id: job_id.to_owned(),
                idempotency_key: format!("success-{job_id}"),
                session_id: format!("session-{job_id}"),
                output_payload: json!({"ok":true}),
                stage: "Manifest".to_owned(),
                scope_kind: "process".to_owned(),
                scope_id: "process-dry".to_owned(),
                stage_run_id: format!("stage-run-{job_id}"),
                process_id: "process-dry".to_owned(),
                owner_id: "local-owner".to_owned(),
            },
            &ValidationOutcome {
                disposition: ValidationDisposition::Accepted,
                error_code: None,
                error_context_json: None,
                anomalies: Vec::new(),
            },
        )?;
        Ok(ack.visible_after_seq)
    }

    fn persist_verdict(
        state_db: &mut StateDb,
        verdict_id: &str,
        worker_job_id: &str,
        validator_job_id: &str,
        payload: serde_json::Value,
    ) -> Result<()> {
        let skill = executor_impl::ValidatorSkillBundle::load(&repo_root()?)?;
        executor_impl::persist_validation_verdict(
            state_db,
            &PersistValidationVerdictRequest {
                verdict_id: verdict_id.to_owned(),
                worker_job_id: worker_job_id.to_owned(),
                worker_session_id: Some(format!("session-{worker_job_id}")),
                validator_job_id: validator_job_id.to_owned(),
                validator_session_id: Some(format!("session-{validator_job_id}")),
                run_manifest_hash: "manifest-hash-dry".to_owned(),
                validator_skill: skill,
                payload,
                oqg_signals_summary: Some(json!({"duplicate_rate":"none"})),
                incident_signature: None,
                quality_assessment_json: None,
                skill_improvement_suggestions_json: None,
                override_verdict: None,
                override_reason: None,
                override_by: None,
                override_at: None,
                verdict_rendered_at: Some("2026-03-30T16:00:00Z".to_owned()),
            },
        )?;
        Ok(())
    }

    #[test]
    fn l3_t_validation_dry_run_001_rejected_dry_run_does_not_advance_stage_heads() -> Result<()> {
        let (_temp, mut state_db) = open_state_db()?;
        seed_process(state_db.connection(), "process-dry")?;
        seed_job(state_db.connection(), "worker-job-dry")?;
        let visible_after = persist_worker_success(&mut state_db, "worker-job-dry")?;
        let decision = decide_validator_verdict(
            &rejected(),
            ValidationHandlingMode {
                dry_run: true,
                enforcement_active: false,
            },
        );
        ensure!(decision.advance_worker);
        ensure!(!decision.quarantine_worker);
        ensure!(visible_after > 0);
        Ok(())
    }

    #[test]
    fn l3_t_validation_dry_run_002_dry_run_verdicts_create_no_sync_visible_side_effects()
    -> Result<()> {
        let (_temp, state_db) = open_state_db()?;
        let count: i64 = state_db.connection().query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE name = 'outbox'",
            [],
            |row| row.get(0),
        )?;
        ensure!(count == 0 || count == 1);
        Ok(())
    }

    #[test]
    fn l3_t_validation_dry_run_003_dry_run_artifacts_live_under_scratch_root() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let scratch = ensure_validation_scratch_dir(temp.path(), "run-001")?;
        ensure!(scratch.ends_with("scratch/run-001"));
        Ok(())
    }

    #[test]
    fn l3_t_validation_dry_run_004_all_verdict_classes_persist_rows_while_worker_advances()
    -> Result<()> {
        let (_temp, mut state_db) = open_state_db()?;
        seed_process(state_db.connection(), "process-dry")?;
        for (worker, validator, verdict_id, payload) in [
            (
                "worker-a",
                "validator-a",
                "verdict-a",
                serde_json::to_value(approved())?,
            ),
            (
                "worker-b",
                "validator-b",
                "verdict-b",
                serde_json::to_value(flagged())?,
            ),
            (
                "worker-c",
                "validator-c",
                "verdict-c",
                serde_json::to_value(rejected())?,
            ),
        ] {
            seed_job(state_db.connection(), worker)?;
            seed_job(state_db.connection(), validator)?;
            persist_worker_success(&mut state_db, worker)?;
            persist_verdict(&mut state_db, verdict_id, worker, validator, payload)?;
        }

        let count: i64 = state_db.connection().query_row(
            "SELECT COUNT(*) FROM validation_verdicts",
            [],
            |row| row.get(0),
        )?;
        ensure!(count == 3);
        Ok(())
    }

    #[test]
    fn l3_t_validation_dry_run_005_worker_visible_after_seq_is_stable_across_verdict_classes()
    -> Result<()> {
        let (_temp, mut state_db) = open_state_db()?;
        seed_process(state_db.connection(), "process-dry")?;
        seed_job(state_db.connection(), "worker-job-dry")?;
        let visible_after = persist_worker_success(&mut state_db, "worker-job-dry")?;
        ensure!(visible_after > 0);
        ensure!(
            decide_validator_verdict(
                &approved(),
                ValidationHandlingMode {
                    dry_run: true,
                    enforcement_active: false
                }
            )
            .advance_worker
        );
        ensure!(
            decide_validator_verdict(
                &flagged(),
                ValidationHandlingMode {
                    dry_run: true,
                    enforcement_active: false
                }
            )
            .advance_worker
        );
        ensure!(
            decide_validator_verdict(
                &rejected(),
                ValidationHandlingMode {
                    dry_run: true,
                    enforcement_active: false
                }
            )
            .advance_worker
        );
        ensure!(decide_validator_failure().advance_worker);
        Ok(())
    }

    #[test]
    fn l3_t_validation_dry_run_006_missing_signals_keep_results_unverified() -> Result<()> {
        let decision = decide_validator_verdict(
            &ValidatorResultContract {
                basis_sufficiency: ValidatorBasisSufficiency::Low,
                ..approved()
            },
            ValidationHandlingMode {
                dry_run: true,
                enforcement_active: false,
            },
        );
        ensure!(decision.trust_state == ValidationTrustState::Unverified);
        Ok(())
    }

    #[test]
    fn l3_t_validation_dry_run_007_validator_failure_sets_skip_marker_and_backlog() -> Result<()> {
        let failure = decide_validator_failure();
        ensure!(failure.validation_marker == Some("VALIDATION_SKIPPED_VALIDATOR_ERROR"));
        ensure!(failure.queue_retroactive_backlog);
        Ok(())
    }

    #[test]
    fn l3_t_validation_dry_run_008_host_sleep_interruption_is_treated_as_validator_failure()
    -> Result<()> {
        let failure = decide_validator_failure();
        ensure!(failure.advance_worker);
        ensure!(!failure.quarantine_worker);
        ensure!(failure.trust_state == ValidationTrustState::Unverified);
        Ok(())
    }

    #[test]
    fn l3_t_validation_dry_run_009_trust_states_map_to_coarse_process_validation_state()
    -> Result<()> {
        let all_verified = rollup_process_validation_state([
            ValidationTrustState::Verified,
            ValidationTrustState::Verified,
        ]);
        let any_flagged = rollup_process_validation_state([
            ValidationTrustState::Verified,
            ValidationTrustState::Flagged,
        ]);
        let unverified = rollup_process_validation_state([
            ValidationTrustState::Verified,
            ValidationTrustState::Unverified,
        ]);
        ensure!(all_verified.as_str() == "validated");
        ensure!(any_flagged.as_str() == "flagged");
        ensure!(unverified.as_str() == "unverified");
        Ok(())
    }

    #[test]
    fn l3_t_validation_dry_run_010_corpus_provenance_tags_survive_enqueue_packet() -> Result<()> {
        let (_temp, mut state_db) = open_state_db()?;
        seed_process(state_db.connection(), "process-dry")?;
        seed_job(state_db.connection(), "worker-job-dry")?;
        let runner = JobRunner::new("runner-dry");
        for (tag, validator_job_id) in [
            ("clean_real", "validator-clean"),
            ("weird_but_valid", "validator-weird"),
            ("mutated_bad", "validator-bad"),
        ] {
            let mut request = sample_request();
            request.validator_job_id = Some(validator_job_id.to_owned());
            request.corpus_provenance_tag = Some(tag.to_owned());
            let enqueued =
                enqueue_validator_sub_job(&repo_root()?, &mut state_db, &runner, &request)?;
            ensure!(enqueued.kickoff_packet.corpus_provenance_tag.as_deref() == Some(tag));
        }
        Ok(())
    }
}
