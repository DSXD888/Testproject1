#![forbid(unsafe_code)]
#![allow(clippy::duplicate_mod)]

mod support;

mod jobs {
    pub mod runner {
        use anyhow::Result;
        use chrono::{DateTime, Utc};
        use serde::{Deserialize, Serialize};
        use serde_json::Value;

        use crate::support::db::StateDb;

        #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
        pub struct MutationAck {
            pub accepted: bool,
            pub visible_after_seq: i64,
            pub continuation_job_id: Option<String>,
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

        #[derive(Debug, Clone)]
        pub struct JobRunner;

        impl JobRunner {
            #[allow(
                clippy::missing_const_for_fn,
                clippy::unnecessary_wraps,
                clippy::unused_self
            )]
            pub fn enqueue_job(
                &self,
                _state_db: &mut StateDb,
                _request: &EnqueueJobRequest,
            ) -> Result<MutationAck> {
                Ok(MutationAck {
                    accepted: true,
                    visible_after_seq: 1,
                    continuation_job_id: None,
                })
            }
        }
    }
}

#[allow(clippy::duplicate_mod)]
#[path = "../src/db/checkpoints.rs"]
#[allow(dead_code)]
mod checkpoints_impl;
#[path = "../src/agents/contracts.rs"]
#[allow(dead_code)]
mod contracts_impl;
#[path = "../src/validation/executor.rs"]
#[allow(dead_code)]
mod executor_impl;

mod tests {
    use anyhow::{Result, ensure};
    use chrono::{TimeZone, Utc};
    use rusqlite::params;
    use serde_json::{Value, json};

    use crate::checkpoints_impl::persist_checkpoint_artifact;
    use crate::contracts_impl::{
        ERROR_OUTPUT_CONTRACT_INVALID, ERROR_PARTIAL_FAILURE, ERROR_RUN_FAILED, OqgCheckInput,
        StageValidationWrite, ValidationDisposition, ValidationOutcome, classify_terminal_failure,
        persist_stage_validation, persist_validated_checkpoint, validate_final_result,
    };
    use crate::executor_impl::{
        ProcessValidationState, VALIDATION_SKIPPED_VALIDATOR_ERROR, ValidationTrustState,
        decide_validator_failure,
    };
    use crate::support::TestContext;
    use crate::support::db::TestStateDb;

    fn fixed_started_at() -> Result<chrono::DateTime<Utc>> {
        Utc.with_ymd_and_hms(2026, 3, 30, 16, 0, 0)
            .single()
            .ok_or_else(|| anyhow::anyhow!("failed to construct fixed timestamp"))
    }

    fn seed_process(conn: &rusqlite::Connection, process_id: &str) -> Result<()> {
        conn.execute(
            "\
            INSERT INTO procurement_processes (
              id, current_stage, title, latest_published_at, latest_updated_at, owner_id
            ) VALUES (?1, 'extract products', 'Process title', '2026-03-30T00:00:00Z',
                      '2026-03-30T00:00:00Z', 'local-owner')
            ",
            params![process_id],
        )?;
        Ok(())
    }

    fn seed_job(conn: &rusqlite::Connection, job_id: &str) -> Result<()> {
        conn.execute(
            "\
            INSERT INTO workflow_jobs (id, kind, status, input_payload, output_payload)
            VALUES (?1, 'stage-job', 'pending', ?2, '{}')
            ",
            params![job_id, json!({ "job_id": job_id }).to_string()],
        )?;
        Ok(())
    }

    fn seed_session(conn: &rusqlite::Connection, session_id: &str, job_id: &str) -> Result<()> {
        conn.execute(
            "\
            INSERT INTO agent_session_logs (id, job_id, skill_path, execution_mode, status)
            VALUES (?1, ?2, 'skills/example/SKILL.md', 'contract', 'running')
            ",
            params![session_id, job_id],
        )?;
        Ok(())
    }

    fn checkpoint_schema() -> Value {
        json!({
            "type": "object",
            "required": ["seq", "cursor", "notices_processed"],
            "properties": {
                "seq": {"type": "integer"},
                "cursor": {"type": "string"},
                "notices_processed": {"type": "integer"}
            }
        })
    }

    fn valid_checkpoint_payload(seq: u32, cursor: &str) -> Value {
        json!({
            "seq": seq,
            "cursor": cursor,
            "notices_processed": seq,
        })
    }

    fn sample_schema() -> Value {
        json!({
            "type": "object",
            "required": ["notice_id", "score_bps", "items"],
            "properties": {
                "notice_id": {"type": "string"},
                "score_bps": {"type": "integer"},
                "items": {"type": "array"}
            }
        })
    }

    fn healthy_oqg() -> OqgCheckInput {
        OqgCheckInput {
            expected_fill_bps: 9_000,
            observed_fill_bps: 9_000,
            duplicate_rate_bps: 100,
            max_duplicate_rate_bps: 500,
            observed_values: vec![100, 200, 300],
            allowed_min_value: 50,
            allowed_max_value: 500,
        }
    }

    #[test]
    fn l3_t_fault_partial_failure_001_valid_checkpoint_followed_by_crash_preserves_last_checkpoint_seq_and_blocks_success()
    -> Result<()> {
        let state_db = TestStateDb::new()?;
        seed_job(state_db.connection(), "worker-job-001")?;
        seed_session(state_db.connection(), "session-001", "worker-job-001")?;

        let stored_seq = persist_validated_checkpoint(state_db.connection(), "session-001", 1)?;
        let classification = classify_terminal_failure(true, false, "Clean", 0, None::<&str>);
        let status: String = state_db.connection().query_row(
            "SELECT status FROM workflow_jobs WHERE id = ?1",
            params!["worker-job-001"],
            |row| row.get(0),
        )?;
        let last_checkpoint_seq: Option<u32> = state_db.connection().query_row(
            "SELECT last_checkpoint_seq FROM agent_session_logs WHERE id = ?1",
            params!["session-001"],
            |row| row.get(0),
        )?;

        ensure!(stored_seq == 1);
        ensure!(classification == Some(ERROR_PARTIAL_FAILURE));
        ensure!(
            status == "pending",
            "checkpoint-only crash should not upgrade job to success"
        );
        ensure!(last_checkpoint_seq == Some(1));
        Ok(())
    }

    #[test]
    fn l3_t_fault_partial_failure_002_out_of_order_checkpoint_replay_is_rejected_without_advancing_checkpoint_reference()
    -> Result<()> {
        let ctx = TestContext::new()?;
        let state_db = TestStateDb::new()?;
        seed_job(state_db.connection(), "worker-job-002")?;
        seed_session(state_db.connection(), "session-002", "worker-job-002")?;

        let first_artifact = persist_checkpoint_artifact(
            state_db.connection(),
            &ctx.artifact_root,
            "session-002",
            fixed_started_at()?,
            &checkpoint_schema(),
            &valid_checkpoint_payload(1, "cursor-001"),
        )?;
        let _ = persist_validated_checkpoint(state_db.connection(), "session-002", 1)?;

        let error = persist_validated_checkpoint(state_db.connection(), "session-002", 3)
            .expect_err("out-of-order checkpoint should fail");
        let last_checkpoint_seq: Option<u32> = state_db.connection().query_row(
            "SELECT last_checkpoint_seq FROM agent_session_logs WHERE id = ?1",
            params!["session-002"],
            |row| row.get(0),
        )?;
        let output_payload: String = state_db.connection().query_row(
            "SELECT output_payload FROM workflow_jobs WHERE id = ?1",
            params!["worker-job-002"],
            |row| row.get(0),
        )?;
        let parsed: Value = serde_json::from_str(&output_payload)?;

        ensure!(error.to_string().contains(ERROR_PARTIAL_FAILURE));
        ensure!(last_checkpoint_seq == Some(1));
        ensure!(
            parsed["checkpoint_ref"]["seq"] == 1,
            "last durable checkpoint reference should remain at the validated seq"
        );
        ensure!(
            parsed["checkpoint_ref"]["path"] == first_artifact.path.to_string_lossy().to_string(),
            "checkpoint reference path should stay pinned to the last valid artifact"
        );
        Ok(())
    }

    #[test]
    fn l3_t_fault_partial_failure_003_continuation_crash_leaves_chain_open_and_prevents_head_closure()
    -> Result<()> {
        let state_db = TestStateDb::new()?;
        seed_process(state_db.connection(), "process-003")?;
        state_db.connection().execute(
            "\
            INSERT INTO workflow_jobs (
              id, kind, process_id, scope_kind, scope_id, continuation_chain_id, chain_closed,
              status, input_payload
            ) VALUES (?1, 'stage-job', ?2, 'notice', 'notice-003', ?3, 0, 'completed', '{}')
            ",
            params!["worker-job-003", "process-003", "chain-003"],
        )?;
        state_db.connection().execute(
            "\
            INSERT INTO workflow_jobs (
              id, kind, process_id, scope_kind, scope_id, parent_job_id,
              continuation_chain_id, status, input_payload
            ) VALUES (?1, 'stage-job', ?2, 'notice', 'notice-003', ?3, ?4, 'pending', ?5)
            ",
            params![
                "worker-job-003-segment-2",
                "process-003",
                "worker-job-003",
                "chain-003",
                json!({"resume_token":"cursor-002"}).to_string(),
            ],
        )?;

        let classification = classify_terminal_failure(true, false, "Forced", 1, None::<&str>);
        let chain_closed: i64 = state_db.connection().query_row(
            "SELECT chain_closed FROM workflow_jobs WHERE id = ?1",
            params!["worker-job-003"],
            |row| row.get(0),
        )?;
        let child_count: i64 = state_db.connection().query_row(
            "SELECT COUNT(*) FROM workflow_jobs WHERE continuation_chain_id = ?1",
            params!["chain-003"],
            |row| row.get(0),
        )?;
        let head_count: i64 = state_db.connection().query_row(
            "SELECT COUNT(*) FROM process_stage_heads WHERE process_id = ?1",
            params!["process-003"],
            |row| row.get(0),
        )?;

        ensure!(classification == Some(ERROR_PARTIAL_FAILURE));
        ensure!(
            chain_closed == 0,
            "continuation chain should remain open after crash"
        );
        ensure!(
            child_count == 2,
            "continuation child should remain queued for recovery"
        );
        ensure!(
            head_count == 0,
            "continuation crash must not finalize stage-head closure"
        );
        Ok(())
    }

    #[test]
    fn l3_t_fault_partial_failure_004_additive_partial_commits_preserve_successful_siblings_without_advancing_failed_sibling_heads()
    -> Result<()> {
        let state_db = TestStateDb::new()?;
        seed_process(state_db.connection(), "process-004")?;

        let accepted = ValidationOutcome {
            disposition: ValidationDisposition::Accepted,
            error_code: None,
            error_context_json: None,
            anomalies: Vec::new(),
        };
        let failed = ValidationOutcome {
            disposition: ValidationDisposition::Failed,
            error_code: Some(ERROR_RUN_FAILED),
            error_context_json: Some(json!({"reason":"validator crash"}).to_string()),
            anomalies: Vec::new(),
        };

        persist_stage_validation(
            state_db.connection(),
            &StageValidationWrite {
                process_id: "process-004",
                stage: "extract products",
                scope_kind: "lot",
                scope_id: "lot-a",
                stage_run_id: "run-004-a",
                owner_id: "local-owner",
            },
            &accepted,
        )?;
        persist_stage_validation(
            state_db.connection(),
            &StageValidationWrite {
                process_id: "process-004",
                stage: "extract products",
                scope_kind: "lot",
                scope_id: "lot-b",
                stage_run_id: "run-004-b",
                owner_id: "local-owner",
            },
            &failed,
        )?;

        let successful_head_count: i64 = state_db.connection().query_row(
            "SELECT COUNT(*) FROM process_stage_heads WHERE process_id = ?1 AND stage = ?2 AND scope_id = ?3",
            params!["process-004", "extract products", "lot-a"],
            |row| row.get(0),
        )?;
        let failed_head_count: i64 = state_db.connection().query_row(
            "SELECT COUNT(*) FROM process_stage_heads WHERE process_id = ?1 AND stage = ?2 AND scope_id = ?3",
            params!["process-004", "extract products", "lot-b"],
            |row| row.get(0),
        )?;
        let failed_status: i64 = state_db.connection().query_row(
            "SELECT COUNT(*) FROM process_stage_status WHERE process_id = ?1 AND stage = ?2 AND scope_id = ?3 AND state = 'failed'",
            params!["process-004", "extract products", "lot-b"],
            |row| row.get(0),
        )?;

        ensure!(
            successful_head_count == 1,
            "successful sibling scope should remain durable"
        );
        ensure!(
            failed_head_count == 0,
            "failed sibling must not advance a stage head"
        );
        ensure!(
            failed_status == 1,
            "failed sibling should remain explicit in stage status"
        );
        Ok(())
    }

    #[test]
    fn l3_t_fault_partial_failure_005_schema_invalid_final_result_fails_worker_path_without_validator_sub_job()
    -> Result<()> {
        let state_db = TestStateDb::new()?;
        seed_job(state_db.connection(), "worker-job-005")?;

        let outcome = validate_final_result(
            &sample_schema(),
            &json!({
                "notice_id": "ocds-005",
                "items": [],
            }),
            &healthy_oqg(),
            std::path::Path::new("results/result.json"),
        );
        let validator_jobs: i64 = state_db.connection().query_row(
            "SELECT COUNT(*) FROM workflow_jobs WHERE kind = 'semantic_validate'",
            [],
            |row| row.get(0),
        )?;

        ensure!(outcome.disposition == ValidationDisposition::Failed);
        ensure!(outcome.error_code == Some(ERROR_OUTPUT_CONTRACT_INVALID));
        ensure!(
            validator_jobs == 0,
            "schema-invalid worker result must not enqueue validator jobs"
        );
        Ok(())
    }

    #[test]
    fn l3_t_fault_partial_failure_006_unpersisted_validator_verdict_fails_open_without_linking_validation_verdict()
    -> Result<()> {
        let state_db = TestStateDb::new()?;
        seed_job(state_db.connection(), "worker-job-006")?;

        let decision = decide_validator_failure();
        let linked_verdict_id: Option<String> = state_db.connection().query_row(
            "SELECT validation_verdict_id FROM workflow_jobs WHERE id = ?1",
            params!["worker-job-006"],
            |row| row.get(0),
        )?;
        let verdict_count: i64 = state_db.connection().query_row(
            "SELECT COUNT(*) FROM validation_verdicts",
            [],
            |row| row.get(0),
        )?;

        ensure!(
            decision.advance_worker,
            "validator failure should remain fail-open"
        );
        ensure!(
            decision.trust_state == ValidationTrustState::Unverified
                && decision.process_validation_state == ProcessValidationState::Unverified
        );
        ensure!(decision.queue_retroactive_backlog);
        ensure!(decision.validation_marker == Some(VALIDATION_SKIPPED_VALIDATOR_ERROR));
        ensure!(
            linked_verdict_id.is_none(),
            "worker job should not receive a verdict link"
        );
        ensure!(
            verdict_count == 0,
            "no semantic verdict row should be persisted"
        );
        Ok(())
    }

    #[test]
    fn l3_t_fault_partial_failure_007_checkpoint_schema_is_validated_before_artifact_persistence()
    -> Result<()> {
        let ctx = TestContext::new()?;
        let state_db = TestStateDb::new()?;
        seed_job(state_db.connection(), "worker-job-007")?;
        seed_session(state_db.connection(), "session-007", "worker-job-007")?;

        let error = persist_checkpoint_artifact(
            state_db.connection(),
            &ctx.artifact_root,
            "session-007",
            fixed_started_at()?,
            &checkpoint_schema(),
            &json!({
                "seq": 1,
                "notices_processed": 1,
            }),
        )
        .expect_err("malformed checkpoint payload should fail schema validation");

        let output_payload: String = state_db.connection().query_row(
            "SELECT output_payload FROM workflow_jobs WHERE id = ?1",
            params!["worker-job-007"],
            |row| row.get(0),
        )?;
        let parsed: Value = serde_json::from_str(&output_payload)?;

        ensure!(
            error
                .to_string()
                .contains("checkpoint payload missing required field `cursor`"),
            "checkpoint schema validation should surface the missing required field"
        );
        ensure!(
            parsed.get("checkpoint_ref").is_none(),
            "malformed checkpoint payload must not write a broken checkpoint_ref"
        );
        Ok(())
    }
}
