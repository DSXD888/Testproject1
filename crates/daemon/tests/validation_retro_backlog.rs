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

    use crate::db_support::StateDb;
    use crate::executor_impl::{
        PersistValidationVerdictRequest, RetroactiveBacklogMetadata, ValidationHandlingMode,
        ValidationTrustState, ValidatorBasisSufficiency, ValidatorEnqueueRequest, ValidatorIssue,
        ValidatorJobPurpose, ValidatorResultContract, ValidatorSkillBundle, ValidatorVerdict,
        apply_retroactive_validation_consequences, decide_validator_verdict,
        drain_retroactive_backlog, enqueue_retroactive_backlog_job,
        load_pending_retroactive_backlog, persist_validation_verdict, retroactive_recovery_summary,
    };
    use crate::runner_impl::JobRunner;

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

    fn sample_request(worker_job_id: &str, validator_job_id: &str) -> ValidatorEnqueueRequest {
        ValidatorEnqueueRequest {
            validator_job_id: Some(validator_job_id.to_owned()),
            idempotency_key: Some(format!("retro-{validator_job_id}")),
            worker_job_id: worker_job_id.to_owned(),
            worker_session_id: Some(format!("session-{worker_job_id}")),
            process_id: Some("process-001".to_owned()),
            scope_kind: Some("process".to_owned()),
            scope_id: Some("process-001".to_owned()),
            continuation_chain_id: Some("chain-001".to_owned()),
            run_manifest_path: PathBuf::from(format!("artifacts/manifests/{worker_job_id}.md")),
            run_manifest_hash: format!("manifest-hash-{worker_job_id}"),
            drift_delta_json: json!({"fingerprint_delta_bps": 50}),
            oqg_anomaly_summary: vec!["duplicate_rate:none".to_owned()],
            representative_item_sample: json!({"worker_job_id": worker_job_id}),
            corpus_provenance_tag: Some("clean_real".to_owned()),
            purpose: ValidatorJobPurpose::LiveValidation,
            canary_case: None,
            retroactive_backlog: None,
        }
    }

    fn backlog_metadata(
        observed_at: &str,
        high_value: bool,
        operator_reviewed: bool,
        enforcement_ready: bool,
    ) -> RetroactiveBacklogMetadata {
        RetroactiveBacklogMetadata {
            high_value_opportunity: high_value,
            operator_reviewed,
            enforcement_ready,
            trust_state: ValidationTrustState::Unverified,
            observed_at: observed_at.to_owned(),
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

    fn rejected_contract() -> ValidatorResultContract {
        ValidatorResultContract {
            verdict: ValidatorVerdict::Rejected,
            basis_sufficiency: ValidatorBasisSufficiency::High,
            enforcement_eligible: true,
            confidence_bps: 8_800,
            issues: vec![ValidatorIssue {
                severity: "critical".to_owned(),
                category: "identity".to_owned(),
                description: "critical mismatch".to_owned(),
                evidence_refs: vec!["result_contract.item_1".to_owned()],
            }],
            summary: "rejected".to_owned(),
        }
    }

    fn persist_approved_verdict(
        state_db: &mut StateDb,
        worker_job_id: &str,
        validator_job_id: &str,
        verdict_id: &str,
    ) -> Result<()> {
        let skill = ValidatorSkillBundle::load(&repo_root()?)?;
        persist_validation_verdict(
            state_db,
            &PersistValidationVerdictRequest {
                verdict_id: verdict_id.to_owned(),
                worker_job_id: worker_job_id.to_owned(),
                worker_session_id: Some(format!("session-{worker_job_id}")),
                validator_job_id: validator_job_id.to_owned(),
                validator_session_id: Some(format!("session-{validator_job_id}")),
                run_manifest_hash: format!("manifest-hash-{worker_job_id}"),
                validator_skill: skill,
                payload: serde_json::to_value(approved_contract())?,
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

    fn seed_pipeline_assessment(conn: &Connection) -> Result<()> {
        conn.execute(
            "\
            INSERT INTO pipeline_assessments (
              process_id, pipeline_state, decision_basis_hash, validation_state, owner_id
            ) VALUES ('process-001', 'review', 'basis-a', 'validated', 'local-owner')
            ",
            [],
        )?;
        Ok(())
    }

    fn seed_pending_intent(conn: &Connection) -> Result<()> {
        conn.execute(
            "\
            INSERT INTO pending_operator_intents (
              id, process_id, desired_disposition, reviewed_basis_hash, reviewed_manifest_hash,
              reviewed_at_stage_run_id, reviewed_at, idempotency_key, queued_at
            ) VALUES (
              'intent-001', 'process-001', 'watch', 'basis-a', 'manifest-a', 'stage-a',
              '2026-03-28T09:00:00Z', 'decision-key', '2026-03-28T09:00:00Z'
            )
            ",
            [],
        )?;
        Ok(())
    }

    fn seed_listing_evidence(conn: &Connection, worker_job_id: &str) -> Result<()> {
        conn.execute(
            "\
            INSERT INTO listing_evidence (
              id, source, source_listing_id, normalized_title, compliance_evidence_tier,
              fetched_at, source_job_id
            ) VALUES (
              'listing-001', 'alibaba', 'listing-a', 'Widget', 'structured_attribute',
              '2026-03-30T16:00:00Z', ?1
            )
            ",
            params![worker_job_id],
        )?;
        Ok(())
    }

    #[test]
    fn l3_t_validation_retro_backlog_001_fail_open_creates_one_backlog_item_with_identity_and_priority_inputs()
    -> Result<()> {
        let (_temp, mut state_db) = open_state_db()?;
        seed_process(state_db.connection(), "process-001")?;
        seed_job(state_db.connection(), "worker-job-001")?;
        let runner = JobRunner::new("retro-001");

        enqueue_retroactive_backlog_job(
            &repo_root()?,
            &mut state_db,
            &runner,
            sample_request("worker-job-001", "validator-retro-001"),
            backlog_metadata("2026-03-30T10:00:00Z", true, false, false),
        )?;

        let items = load_pending_retroactive_backlog(state_db.connection())?;
        ensure!(items.len() == 1);
        ensure!(items[0].worker_job_id == "worker-job-001");
        ensure!(items[0].trust_state == ValidationTrustState::Unverified);
        ensure!(items[0].high_value_opportunity);
        Ok(())
    }

    #[test]
    fn l3_t_validation_retro_backlog_002_backlog_priority_is_high_value_then_operator_reviewed_then_enforcement_then_recent()
    -> Result<()> {
        let (_temp, mut state_db) = open_state_db()?;
        seed_process(state_db.connection(), "process-001")?;
        for worker_job_id in ["worker-hv", "worker-op", "worker-enf", "worker-recent"] {
            seed_job(state_db.connection(), worker_job_id)?;
        }
        let runner = JobRunner::new("retro-002");

        enqueue_retroactive_backlog_job(
            &repo_root()?,
            &mut state_db,
            &runner,
            sample_request("worker-recent", "validator-retro-004"),
            backlog_metadata("2026-03-30T12:00:00Z", false, false, false),
        )?;
        enqueue_retroactive_backlog_job(
            &repo_root()?,
            &mut state_db,
            &runner,
            sample_request("worker-enf", "validator-retro-003"),
            backlog_metadata("2026-03-30T09:00:00Z", false, false, true),
        )?;
        enqueue_retroactive_backlog_job(
            &repo_root()?,
            &mut state_db,
            &runner,
            sample_request("worker-op", "validator-retro-002"),
            backlog_metadata("2026-03-30T08:00:00Z", false, true, false),
        )?;
        enqueue_retroactive_backlog_job(
            &repo_root()?,
            &mut state_db,
            &runner,
            sample_request("worker-hv", "validator-retro-001"),
            backlog_metadata("2026-03-30T07:00:00Z", true, false, false),
        )?;

        let ordered = load_pending_retroactive_backlog(state_db.connection())?;
        let ids = ordered
            .iter()
            .map(|item| item.worker_job_id.clone())
            .collect::<Vec<_>>();
        ensure!(
            ids == vec![
                "worker-hv".to_owned(),
                "worker-op".to_owned(),
                "worker-enf".to_owned(),
                "worker-recent".to_owned(),
            ]
        );
        Ok(())
    }

    #[test]
    fn l3_t_validation_retro_backlog_003_recovery_drains_backlog_in_priority_order_and_persists_verdicts()
    -> Result<()> {
        let (_temp, mut state_db) = open_state_db()?;
        seed_process(state_db.connection(), "process-001")?;
        seed_job(state_db.connection(), "worker-hv")?;
        seed_job(state_db.connection(), "worker-recent")?;
        let runner = JobRunner::new("retro-003");

        enqueue_retroactive_backlog_job(
            &repo_root()?,
            &mut state_db,
            &runner,
            sample_request("worker-recent", "validator-retro-102"),
            backlog_metadata("2026-03-30T12:00:00Z", false, false, false),
        )?;
        enqueue_retroactive_backlog_job(
            &repo_root()?,
            &mut state_db,
            &runner,
            sample_request("worker-hv", "validator-retro-101"),
            backlog_metadata("2026-03-30T07:00:00Z", true, false, false),
        )?;

        let ordered = load_pending_retroactive_backlog(state_db.connection())?;
        for (index, item) in ordered.iter().enumerate() {
            persist_approved_verdict(
                &mut state_db,
                &item.worker_job_id,
                &item.backlog_job_id,
                &format!("retro-verdict-{}", index + 1),
            )?;
        }
        let drained = drain_retroactive_backlog(&mut state_db, ordered, |item| {
            Ok(json!({"summary": format!("completed {}", item.worker_job_id)}))
        })?;

        ensure!(drained.len() == 2);
        let verdict_count: i64 = state_db.connection().query_row(
            "SELECT COUNT(*) FROM validation_verdicts",
            [],
            |row| row.get(0),
        )?;
        ensure!(verdict_count == 2);

        let first_completed_worker: String = state_db.connection().query_row(
            "\
            SELECT worker_job_id
            FROM validation_verdicts
            WHERE id = 'retro-verdict-1'
            ",
            [],
            |row| row.get(0),
        )?;
        ensure!(first_completed_worker == "worker-hv");
        Ok(())
    }

    #[test]
    fn l3_t_validation_retro_backlog_004_retroactive_rejection_marks_outputs_suspect_and_decisions_stale()
    -> Result<()> {
        let (_temp, mut state_db) = open_state_db()?;
        seed_process(state_db.connection(), "process-001")?;
        seed_job(state_db.connection(), "worker-job-001")?;
        seed_pipeline_assessment(state_db.connection())?;
        seed_pending_intent(state_db.connection())?;
        seed_listing_evidence(state_db.connection(), "worker-job-001")?;

        let decision = decide_validator_verdict(
            &rejected_contract(),
            ValidationHandlingMode {
                dry_run: true,
                enforcement_active: false,
            },
        );
        apply_retroactive_validation_consequences(
            &mut state_db,
            "worker-job-001",
            "process-001",
            &decision,
            "retro-verdict-rejected",
        )?;

        let trust_state: String = state_db.connection().query_row(
            "SELECT trust_state FROM listing_evidence WHERE id = 'listing-001'",
            [],
            |row| row.get(0),
        )?;
        let validation_state: String = state_db.connection().query_row(
            "SELECT validation_state FROM pipeline_assessments WHERE process_id = 'process-001'",
            [],
            |row| row.get(0),
        )?;
        let stale: i64 = state_db.connection().query_row(
            "\
            SELECT CASE WHEN poi.reviewed_basis_hash <> pa.decision_basis_hash THEN 1 ELSE 0 END
            FROM pending_operator_intents poi
            INNER JOIN pipeline_assessments pa ON pa.process_id = poi.process_id
            WHERE poi.id = 'intent-001'
            ",
            [],
            |row| row.get(0),
        )?;

        ensure!(trust_state == "suspect");
        ensure!(validation_state == "flagged");
        ensure!(stale == 1);
        Ok(())
    }

    #[test]
    fn l3_t_validation_retro_backlog_005_backlog_items_do_not_silently_expire() -> Result<()> {
        let (_temp, mut state_db) = open_state_db()?;
        seed_process(state_db.connection(), "process-001")?;
        seed_job(state_db.connection(), "worker-hv")?;
        seed_job(state_db.connection(), "worker-op")?;
        let runner = JobRunner::new("retro-005");

        enqueue_retroactive_backlog_job(
            &repo_root()?,
            &mut state_db,
            &runner,
            sample_request("worker-hv", "validator-retro-201"),
            backlog_metadata("2026-03-30T07:00:00Z", true, false, false),
        )?;
        enqueue_retroactive_backlog_job(
            &repo_root()?,
            &mut state_db,
            &runner,
            sample_request("worker-op", "validator-retro-202"),
            backlog_metadata("2026-03-30T08:00:00Z", false, true, false),
        )?;

        let pending_count: i64 = state_db.connection().query_row(
            "SELECT COUNT(*) FROM workflow_jobs WHERE kind = 'semantic_validate' AND status = 'pending'",
            [],
            |row| row.get(0),
        )?;
        let items = load_pending_retroactive_backlog(state_db.connection())?;

        ensure!(pending_count == 2);
        ensure!(items.len() == 2);
        ensure!(items.iter().any(|item| item.high_value_opportunity));
        ensure!(items.iter().any(|item| item.operator_reviewed));
        Ok(())
    }

    #[test]
    fn l3_t_validation_retro_backlog_006_recovery_surfaces_one_summary_event_and_status_updates()
    -> Result<()> {
        let (_temp, mut state_db) = open_state_db()?;
        seed_process(state_db.connection(), "process-001")?;
        seed_job(state_db.connection(), "worker-job-001")?;
        seed_pipeline_assessment(state_db.connection())?;
        seed_pending_intent(state_db.connection())?;
        seed_listing_evidence(state_db.connection(), "worker-job-001")?;

        let decision = decide_validator_verdict(
            &rejected_contract(),
            ValidationHandlingMode {
                dry_run: true,
                enforcement_active: false,
            },
        );
        apply_retroactive_validation_consequences(
            &mut state_db,
            "worker-job-001",
            "process-001",
            &decision,
            "retro-verdict-summary",
        )?;

        let summary = retroactive_recovery_summary(3, 1);
        let validation_state: String = state_db.connection().query_row(
            "SELECT validation_state FROM pipeline_assessments WHERE process_id = 'process-001'",
            [],
            |row| row.get(0),
        )?;

        ensure!(summary.contains("validated retroactively: 3 runs processed, 1 issues found"));
        ensure!(validation_state == "flagged");
        Ok(())
    }
}
