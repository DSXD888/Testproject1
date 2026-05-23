#![forbid(unsafe_code)]

#[path = "../src/agents/artifacts.rs"]
#[allow(dead_code)]
mod artifacts_impl;
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
    use chrono::Utc;
    use rusqlite::{Connection, params};
    use serde_json::json;

    use crate::artifacts_impl::{
        ArtifactStore, DaemonEvidenceAppendix, OutcomeClass, RunManifestDraft,
        RunManifestFrontmatter, RunNarrative,
    };
    use crate::db_support::StateDb;
    use crate::executor_impl::{
        self, AGENT_SUSPECT_OUTPUT, PersistValidationVerdictRequest, ValidationExecutorEffect,
        ValidationHandlingMode, ValidationTrustState, ValidatorBasisSufficiency,
        ValidatorEnqueueRequest, ValidatorIssue, ValidatorResultContract, ValidatorVerdict,
        decide_validator_failure, decide_validator_verdict, enqueue_validator_sub_job,
        parse_validator_result_contract, validation_executor_effect,
    };
    use crate::runner_impl::JobRunner;

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

    fn sample_request() -> ValidatorEnqueueRequest {
        ValidatorEnqueueRequest {
            validator_job_id: Some("validator-job-001".to_owned()),
            idempotency_key: Some("validator-idempotency-001".to_owned()),
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
            corpus_provenance_tag: Some("clean_real".to_owned()),
            purpose: executor_impl::ValidatorJobPurpose::LiveValidation,
            canary_case: None,
            retroactive_backlog: None,
        }
    }

    fn approved_contract() -> ValidatorResultContract {
        ValidatorResultContract {
            verdict: ValidatorVerdict::Approved,
            basis_sufficiency: ValidatorBasisSufficiency::High,
            enforcement_eligible: true,
            confidence_bps: 8_500,
            issues: Vec::new(),
            summary: "approved".to_owned(),
        }
    }

    fn flagged_contract() -> ValidatorResultContract {
        ValidatorResultContract {
            verdict: ValidatorVerdict::Flagged,
            basis_sufficiency: ValidatorBasisSufficiency::Medium,
            enforcement_eligible: false,
            confidence_bps: 6_400,
            issues: vec![ValidatorIssue {
                severity: "minor".to_owned(),
                category: "consistency".to_owned(),
                description: "minor disagreement".to_owned(),
                evidence_refs: vec!["daemon_appendix.observed_url_chain".to_owned()],
            }],
            summary: "flagged".to_owned(),
        }
    }

    fn rejected_contract() -> ValidatorResultContract {
        ValidatorResultContract {
            verdict: ValidatorVerdict::Rejected,
            basis_sufficiency: ValidatorBasisSufficiency::High,
            enforcement_eligible: true,
            confidence_bps: 9_000,
            issues: vec![ValidatorIssue {
                severity: "critical".to_owned(),
                category: "identity".to_owned(),
                description: "critical mismatch".to_owned(),
                evidence_refs: vec!["result_contract.item_42".to_owned()],
            }],
            summary: "rejected".to_owned(),
        }
    }

    fn sample_manifest() -> RunManifestDraft {
        RunManifestDraft {
            frontmatter: RunManifestFrontmatter {
                manifest_version: 1,
                skill_id: "procurement-ingestion/fts".to_owned(),
                skill_version: "1.0.0".to_owned(),
                job_id: "job-001".to_owned(),
                session_id: "session-001".to_owned(),
                execution_mode: "contract".to_owned(),
                started_at: "2026-03-30T16:00:00Z".to_owned(),
                completed_at: "2026-03-30T16:01:00Z".to_owned(),
                outcome_class: OutcomeClass::Completed,
                items_processed: 1,
                items_skipped: 0,
                items_failed: 0,
                warnings_count: 0,
                agent_claimed_pages_visited: 1,
                agent_claimed_source_url: "https://www.find-tender.service.gov.uk/notice/1"
                    .to_owned(),
                agent_claimed_fingerprint_state: "matched".to_owned(),
                agent_claimed_locale_state: "en-GB".to_owned(),
                agent_claimed_currency_state: "GBP".to_owned(),
                agent_narrative_missing: false,
            },
            narrative: Some(RunNarrative {
                summary: "summary".to_owned(),
                steps_taken: "steps".to_owned(),
                decisions_made: "decisions".to_owned(),
                warnings: "warnings".to_owned(),
                evidence: "evidence".to_owned(),
            }),
            appendix: DaemonEvidenceAppendix {
                observed_url_chain: vec![
                    "https://www.find-tender.service.gov.uk".to_owned(),
                    "https://www.find-tender.service.gov.uk/notice/1".to_owned(),
                ],
                observed_page_kind_sequence: vec!["listing".to_owned(), "notice".to_owned()],
                daemon_observed_fingerprint_state: "matched".to_owned(),
                daemon_observed_locale: "en-GB".to_owned(),
                daemon_observed_currency: "GBP".to_owned(),
                auth_challenge_events: Vec::new(),
                locator_fallback_depth: 0,
                basis_sufficiency: "high".to_owned(),
                page_fingerprint_hash: Some("hash-001".to_owned()),
                auth_challenge_detected: false,
                extraction_fill_rate_bps: Some(9_900),
                checkpoint_count: 1,
                byte_count_compressed: None,
                validator_rubric_hash: Some("rubric-sha256".to_owned()),
                drift_delta_json: json!({"fingerprint_delta_bps": 125}),
                result_item_count: 1,
                oqg_anomaly_summary: vec!["duplicate_rate:low".to_owned()],
                representative_item_sample: json!({"item":"sample"}),
                artifact_refs: vec!["results/result.json".to_owned()],
                bid_scope_summary: None,
                side_effect_progression: Vec::new(),
                readiness_evidence: None,
                validator_verdict: None,
                override_metadata: None,
                final_bid_state: None,
            },
            disagreements: Vec::new(),
        }
    }

    #[test]
    fn l3_t_validation_executor_001_validator_sub_jobs_enqueue_with_parent_and_chain() -> Result<()>
    {
        let (_temp, mut state_db) = open_state_db()?;
        seed_process(state_db.connection(), "process-001")?;
        seed_job(state_db.connection(), "worker-job-001")?;
        let runner = JobRunner::new("executor-001");
        let request = sample_request();

        let enqueued = enqueue_validator_sub_job(&repo_root()?, &mut state_db, &runner, &request)?;
        let (parent_job_id, continuation_chain_id): (Option<String>, Option<String>) =
            state_db.connection().query_row(
                "SELECT parent_job_id, continuation_chain_id FROM workflow_jobs WHERE id = ?1",
                params![enqueued.job_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )?;
        ensure!(parent_job_id.as_deref() == Some("worker-job-001"));
        ensure!(continuation_chain_id.as_deref() == Some("chain-001"));
        Ok(())
    }

    #[test]
    fn l3_t_validation_executor_002_validator_jobs_use_contract_low_cost_base_skill_only()
    -> Result<()> {
        let (_temp, mut state_db) = open_state_db()?;
        seed_process(state_db.connection(), "process-001")?;
        seed_job(state_db.connection(), "worker-job-001")?;
        let runner = JobRunner::new("executor-002");
        let request = sample_request();

        let enqueued = enqueue_validator_sub_job(&repo_root()?, &mut state_db, &runner, &request)?;
        ensure!(enqueued.skill.interaction_profile == "headless_contract");
        ensure!(enqueued.skill.model_profile == "structured_extract_low_cost");
        ensure!(enqueued.skill.skill_path == "skills/validation/base/SKILL.md");
        ensure!(enqueued.enqueue_request.kind == "semantic_validate");
        ensure!(enqueued.enqueue_request.notice_id.is_none());
        Ok(())
    }

    #[test]
    fn l3_t_validation_executor_003_kickoff_packet_contains_bounded_structured_evidence()
    -> Result<()> {
        let (_temp, mut state_db) = open_state_db()?;
        seed_process(state_db.connection(), "process-001")?;
        seed_job(state_db.connection(), "worker-job-001")?;
        let runner = JobRunner::new("executor-003");
        let request = sample_request();
        let enqueued = enqueue_validator_sub_job(&repo_root()?, &mut state_db, &runner, &request)?;
        let payload = &enqueued.enqueue_request.input_payload;

        ensure!(
            payload["validator_packet"]["run_manifest_path"]
                == "artifacts/manifests/worker-session-001.md"
        );
        ensure!(payload["validator_packet"]["run_manifest_hash"] == "manifest-hash-001");
        ensure!(
            payload["validator_packet"]["validator_rubric_hash"]
                == enqueued.skill.validator_rubric_hash
        );
        ensure!(payload["validator_packet"]["drift_delta_json"]["fingerprint_delta_bps"] == 125);
        ensure!(payload["validator_packet"]["oqg_anomaly_summary"][0] == "duplicate_rate:low");
        ensure!(payload["validator_packet"]["representative_item_sample"]["item"] == "sample");
        ensure!(payload.get("raw_forensic_bytes").is_none());
        ensure!(payload.get("internal_reasoning").is_none());
        ensure!(payload.get("result_json").is_none());
        Ok(())
    }

    #[test]
    fn l3_t_validation_executor_004_validator_result_contract_requires_all_fields() -> Result<()> {
        let repo_root = repo_root()?;
        let skill = executor_impl::ValidatorSkillBundle::load(&repo_root)?;
        let valid = json!({
            "verdict":"approved",
            "basis_sufficiency":"high",
            "enforcement_eligible":true,
            "confidence_bps":8500,
            "issues":[],
            "summary":"ok"
        });
        parse_validator_result_contract(&skill.result_schema, &valid)?;

        let invalid = json!({
            "verdict":"approved",
            "basis_sufficiency":"high",
            "enforcement_eligible":true,
            "confidence_bps":8500,
            "issues":[]
        });
        ensure!(
            parse_validator_result_contract(&skill.result_schema, &invalid).is_err(),
            "missing summary should be schema-invalid",
        );
        Ok(())
    }

    #[test]
    fn l3_t_validation_executor_005_approved_verdict_persists_and_links_atomically() -> Result<()> {
        let (_temp, mut state_db) = open_state_db()?;
        seed_process(state_db.connection(), "process-001")?;
        seed_job(state_db.connection(), "worker-job-001")?;
        seed_job(state_db.connection(), "validator-job-001")?;
        let skill = executor_impl::ValidatorSkillBundle::load(&repo_root()?)?;
        let persisted = executor_impl::persist_validation_verdict(
            &mut state_db,
            &PersistValidationVerdictRequest {
                verdict_id: "verdict-001".to_owned(),
                worker_job_id: "worker-job-001".to_owned(),
                worker_session_id: Some("worker-session-001".to_owned()),
                validator_job_id: "validator-job-001".to_owned(),
                validator_session_id: Some("validator-session-001".to_owned()),
                run_manifest_hash: "manifest-hash-001".to_owned(),
                validator_skill: skill,
                payload: serde_json::to_value(approved_contract())?,
                oqg_signals_summary: Some(json!({"duplicate_rate":"low"})),
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

        let count: i64 = state_db.connection().query_row(
            "SELECT COUNT(*) FROM validation_verdicts WHERE id = ?1",
            params![persisted.verdict_id],
            |row| row.get(0),
        )?;
        let linked: Option<String> = state_db.connection().query_row(
            "SELECT validation_verdict_id FROM workflow_jobs WHERE id = ?1",
            params!["worker-job-001"],
            |row| row.get(0),
        )?;
        ensure!(count == 1);
        ensure!(linked.as_deref() == Some("verdict-001"));
        Ok(())
    }

    #[test]
    fn l3_t_validation_executor_006_flagged_and_low_basis_keep_worker_accepted() -> Result<()> {
        let flagged = decide_validator_verdict(
            &flagged_contract(),
            ValidationHandlingMode {
                dry_run: false,
                enforcement_active: false,
            },
        );
        ensure!(flagged.advance_worker);
        ensure!(!flagged.quarantine_worker);
        ensure!(flagged.trust_state == ValidationTrustState::Flagged);

        let low_basis = decide_validator_verdict(
            &ValidatorResultContract {
                basis_sufficiency: ValidatorBasisSufficiency::Low,
                ..approved_contract()
            },
            ValidationHandlingMode {
                dry_run: false,
                enforcement_active: false,
            },
        );
        ensure!(low_basis.advance_worker);
        ensure!(!low_basis.quarantine_worker);
        ensure!(low_basis.trust_state == ValidationTrustState::Unverified);
        Ok(())
    }

    #[test]
    fn l3_t_validation_executor_007_dry_run_rejected_enforcement_eligible_still_advances()
    -> Result<()> {
        let decision = decide_validator_verdict(
            &rejected_contract(),
            ValidationHandlingMode {
                dry_run: true,
                enforcement_active: false,
            },
        );
        ensure!(decision.advance_worker);
        ensure!(!decision.quarantine_worker);
        ensure!(decision.trust_state == ValidationTrustState::Suspect);
        Ok(())
    }

    #[test]
    fn l4_t_validation_executor_001_enforcement_rejection_maps_to_quarantine_effect() -> Result<()>
    {
        let mode = ValidationHandlingMode {
            dry_run: false,
            enforcement_active: true,
        };
        let decision = decide_validator_verdict(&rejected_contract(), mode);

        ensure!(!decision.advance_worker);
        ensure!(decision.quarantine_worker);
        ensure!(decision.operator_alert_error_code == Some(AGENT_SUSPECT_OUTPUT));
        ensure!(
            validation_executor_effect(mode, &decision)
                == ValidationExecutorEffect::QuarantineProvisional
        );

        let approval = decide_validator_verdict(&approved_contract(), mode);
        ensure!(
            validation_executor_effect(mode, &approval)
                == ValidationExecutorEffect::PromoteProvisional
        );

        let dry_run_mode = ValidationHandlingMode {
            dry_run: true,
            enforcement_active: false,
        };
        let dry_run_rejection = decide_validator_verdict(&rejected_contract(), dry_run_mode);
        ensure!(
            validation_executor_effect(dry_run_mode, &dry_run_rejection)
                == ValidationExecutorEffect::DirectWrite
        );
        Ok(())
    }

    #[test]
    fn l3_t_validation_executor_008_unknown_verdict_triggers_fail_open() -> Result<()> {
        let repo_root = repo_root()?;
        let skill = executor_impl::ValidatorSkillBundle::load(&repo_root)?;
        let payload = json!({
            "verdict":"mystery",
            "basis_sufficiency":"high",
            "enforcement_eligible":true,
            "confidence_bps":8500,
            "issues":[],
            "summary":"oops"
        });
        ensure!(parse_validator_result_contract(&skill.result_schema, &payload).is_err());
        let fallback = decide_validator_failure();
        ensure!(fallback.advance_worker);
        ensure!(fallback.queue_retroactive_backlog);
        Ok(())
    }

    #[test]
    fn l3_t_validation_executor_009_schema_invalid_payload_triggers_fail_open() -> Result<()> {
        let repo_root = repo_root()?;
        let skill = executor_impl::ValidatorSkillBundle::load(&repo_root)?;
        let payload = json!({
            "verdict":"approved",
            "basis_sufficiency":"high",
            "enforcement_eligible":true,
            "confidence_bps":"wrong",
            "issues":[],
            "summary":"oops"
        });
        ensure!(parse_validator_result_contract(&skill.result_schema, &payload).is_err());
        let fallback = decide_validator_failure();
        ensure!(fallback.advance_worker);
        ensure!(fallback.queue_retroactive_backlog);
        Ok(())
    }

    #[test]
    fn l3_t_validation_executor_010_unpersisted_verdict_leaves_worker_row_unlinked() -> Result<()> {
        let (_temp, state_db) = open_state_db()?;
        seed_job(state_db.connection(), "worker-job-001")?;
        let linked: Option<String> = state_db.connection().query_row(
            "SELECT validation_verdict_id FROM workflow_jobs WHERE id = ?1",
            params!["worker-job-001"],
            |row| row.get(0),
        )?;
        let fallback = decide_validator_failure();
        ensure!(linked.is_none());
        ensure!(fallback.trust_state == ValidationTrustState::Unverified);
        Ok(())
    }

    #[test]
    fn l3_t_validation_executor_011_continuation_segments_preserve_chain_identity() -> Result<()> {
        let (_temp, mut state_db) = open_state_db()?;
        seed_process(state_db.connection(), "process-001")?;
        seed_job(state_db.connection(), "worker-job-001")?;
        seed_job(state_db.connection(), "worker-job-002")?;
        let runner = JobRunner::new("executor-011");
        let first = sample_request();
        let second = ValidatorEnqueueRequest {
            worker_job_id: "worker-job-002".to_owned(),
            validator_job_id: Some("validator-job-002".to_owned()),
            ..sample_request()
        };

        let first_job = enqueue_validator_sub_job(&repo_root()?, &mut state_db, &runner, &first)?;
        let second_job = enqueue_validator_sub_job(&repo_root()?, &mut state_db, &runner, &second)?;
        let chains: Vec<String> = {
            let mut stmt = state_db.connection().prepare(
                "SELECT continuation_chain_id FROM workflow_jobs WHERE id IN (?1, ?2) ORDER BY id",
            )?;
            stmt.query_map(params![first_job.job_id, second_job.job_id], |row| {
                row.get(0)
            })?
            .collect::<std::result::Result<Vec<String>, _>>()?
        };
        ensure!(chains == vec!["chain-001".to_owned(), "chain-001".to_owned()]);
        Ok(())
    }

    #[test]
    fn l3_t_validation_executor_012_run_manifest_fallback_still_preserves_l3_appendix_additions()
    -> Result<()> {
        let temp = tempfile::tempdir()?;
        let store = ArtifactStore::new(temp.path())?;
        let started_at = Utc::now();
        let stored = store.write_run_manifest(&sample_manifest(), started_at)?;
        let markdown = std::fs::read_to_string(&stored.path)?;
        ensure!(markdown.contains("validator_rubric_hash: rubric-sha256"));
        ensure!(markdown.contains("drift_delta_json"));
        ensure!(markdown.contains("representative_item_sample"));
        ensure!(!stored.warning_review_required);
        Ok(())
    }
}
