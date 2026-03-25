#![forbid(unsafe_code)]

mod support;

#[path = "../src/agents/adapter.rs"]
mod adapter_impl;
#[path = "../src/agents/artifacts.rs"]
mod artifacts_impl;
#[path = "../src/agents/contracts.rs"]
mod contracts_impl;
#[path = "../src/errors/manifest.rs"]
mod errors_manifest_impl;
#[path = "../src/errors/markdown.rs"]
mod errors_markdown_impl;
#[path = "../src/prompt/renderer.rs"]
mod prompt_renderer_impl;
#[path = "../src/jobs/runner.rs"]
mod runner_impl;
#[path = "support/skills.rs"]
mod skills_support;

#[cfg(unix)]
mod tests {
    use std::collections::BTreeMap;
    use std::ffi::OsString;
    use std::fs;
    use std::path::Path;
    use std::path::PathBuf;

    use anyhow::{Result, bail, ensure};
    use async_trait::async_trait;
    use bytes::Bytes;
    use chrono::TimeZone;
    use portable_pty::ExitStatus;
    use rusqlite::params;
    use serde_json::json;
    use sha2::{Digest, Sha256};

    use crate::adapter_impl::{
        AdapterParser, AgentAdapter, AuthChallengeKind, CapabilityReport, ControlEvent,
        ExecutionMode, InteractionProfile, LaunchSpec, OutputFrame, ProbeCtx, SkillManifest,
        SpawnRequest, resolve_execution_mode,
    };
    use crate::artifacts_impl::{
        ArtifactStore, DaemonEvidenceAppendix, OutcomeClass, RunManifestDraft,
        RunManifestFrontmatter, RunNarrative, StoredManifest, persist_run_manifest_metadata,
        read_contiguous_jsonl_prefix, validate_result_payload,
    };
    use crate::contracts_impl::{
        ERROR_OUTPUT_CONTRACT_INVALID, ERROR_PARTIAL_FAILURE, ERROR_RUN_FAILED,
        ERROR_SUSPECT_OUTPUT, OqgCheckInput, StageValidationWrite, ValidationDisposition,
        ValidationOutcome, classify_terminal_failure, persist_stage_validation,
        persist_validated_checkpoint, validate_final_result,
    };
    use crate::errors_manifest_impl::{
        ErrorCode, ErrorManifestDraft, ErrorManifestStore, ErrorRuntimeContext,
    };
    use crate::prompt_renderer_impl::{
        PromptRenderer, PromptRendererInput, RUN_MANIFEST_INSTRUCTION_BLOCK,
    };
    use crate::runner_impl::{
        FreshnessState, JOB_OBSOLETE, JobRunner, ObsoleteJobRequest, PersistContinuationRequest,
        PersistSuccessRequest, SQLITE_WRITE_FAILED, StageCommitSemantics, allocate_stage_run_id,
        basis_hash_mismatch, classify_sqlite_write_failed, persist_failed_set_replacement_attempt,
        set_stage_head_freshness, stage_scope_rule,
    };
    use crate::skills_support::{TestSkillDir, lint_skill_directory};
    use crate::support::TestContext;
    use crate::support::db::{StateDb, TestStateDb};

    struct DummyParser {
        mode: ExecutionMode,
    }

    #[async_trait]
    impl AdapterParser for DummyParser {
        async fn on_output(&mut self, frame: &OutputFrame) -> Result<Vec<ControlEvent>> {
            let events = match (self.mode, frame) {
                (ExecutionMode::Contract, OutputFrame::Stdout(payload))
                    if payload.starts_with(b"checkpoint:") =>
                {
                    vec![ControlEvent::Checkpoint {
                        seq: 1,
                        path: PathBuf::from("checkpoint-1.json"),
                    }]
                }
                _ => Vec::new(),
            };
            Ok(events)
        }

        async fn finish(&mut self) -> Result<Vec<ControlEvent>> {
            Ok(vec![ControlEvent::Exited {
                status: ExitStatus::with_exit_code(0),
            }])
        }
    }

    struct DummyAdapter;

    #[async_trait]
    impl AgentAdapter for DummyAdapter {
        fn id(&self) -> &'static str {
            "dummy"
        }

        async fn probe(&self, ctx: &ProbeCtx) -> Result<CapabilityReport> {
            let _ = &ctx.working_directory;
            Ok(CapabilityReport {
                cli_version: "1.0.0".to_owned(),
                auth_healthy: true,
                structured_output: true,
                pty_viable: true,
                browser_tools: true,
                is_canary: false,
            })
        }

        fn resolve_mode(
            &self,
            skill: &SkillManifest,
            caps: &CapabilityReport,
        ) -> Result<ExecutionMode> {
            resolve_execution_mode(skill, caps)
        }

        fn build_launch_spec(
            &self,
            req: &SpawnRequest,
            _mode: ExecutionMode,
        ) -> Result<LaunchSpec> {
            Ok(LaunchSpec {
                program: req.program.clone(),
                args: req.args.clone(),
                env: req.env.clone(),
                cwd: req.cwd.clone(),
            })
        }

        fn parser(&self, mode: ExecutionMode) -> Box<dyn AdapterParser> {
            Box::new(DummyParser { mode })
        }
    }

    fn assert_checkpoint_event(event: &ControlEvent, seq: u32, path: &str) -> Result<()> {
        match event {
            ControlEvent::Checkpoint {
                seq: seen,
                path: seen_path,
            } => {
                ensure!(*seen == seq, "unexpected checkpoint sequence");
                ensure!(
                    seen_path.as_path() == std::path::Path::new(path),
                    "unexpected checkpoint path"
                );
                Ok(())
            }
            _ => bail!("unexpected checkpoint event"),
        }
    }

    fn assert_exit_status_success(event: &ControlEvent) -> Result<()> {
        match event {
            ControlEvent::Exited { status } => {
                ensure!(status.success(), "expected successful exit status");
                Ok(())
            }
            _ => bail!("unexpected exit event"),
        }
    }

    fn assert_auth_challenge(
        event: ControlEvent,
        expected_kind: &AuthChallengeKind,
        expected_detail: &str,
    ) -> Result<()> {
        match event {
            ControlEvent::AuthChallenge { kind, detail } => {
                ensure!(kind == *expected_kind, "unexpected auth challenge kind");
                ensure!(
                    detail == expected_detail,
                    "unexpected auth challenge detail"
                );
                Ok(())
            }
            _ => bail!("unexpected auth challenge event"),
        }
    }

    fn assert_warning_event(
        event: ControlEvent,
        expected_code: &str,
        expected_detail: &str,
    ) -> Result<()> {
        match event {
            ControlEvent::Warning { code, detail } => {
                ensure!(code == expected_code, "unexpected warning code");
                ensure!(detail == expected_detail, "unexpected warning detail");
                Ok(())
            }
            _ => bail!("unexpected warning event"),
        }
    }

    fn assert_started_event(event: &ControlEvent, expected_ids: (u32, u32, u32)) -> Result<()> {
        match event {
            ControlEvent::Started {
                pid: process_id,
                pgid: process_group_id,
                sid: session_id,
                mode,
            } => {
                ensure!(
                    (*process_id, *process_group_id, *session_id) == expected_ids,
                    "unexpected runtime identifiers"
                );
                ensure!(*mode == ExecutionMode::Pty, "unexpected execution mode");
                Ok(())
            }
            _ => bail!("unexpected started event"),
        }
    }

    fn assert_progress_event(
        event: &ControlEvent,
        completed: u32,
        total: Option<u32>,
    ) -> Result<()> {
        match event {
            ControlEvent::Progress {
                completed: actual_completed,
                total: actual_total,
            } => {
                ensure!(
                    *actual_completed == completed,
                    "unexpected progress completed count"
                );
                ensure!(*actual_total == total, "unexpected progress total");
                Ok(())
            }
            _ => bail!("unexpected progress event"),
        }
    }

    fn seed_process(conn: &rusqlite::Connection, process_id: &str) -> Result<()> {
        conn.execute(
            "\
            INSERT INTO procurement_processes (
              id, current_stage, title, latest_published_at, latest_updated_at, owner_id
            ) VALUES (?1, 'qualification', 'Process title', '2026-03-25T00:00:00Z',
                      '2026-03-25T00:00:00Z', 'local-owner')
            ",
            params![process_id],
        )?;
        Ok(())
    }

    fn seed_session(conn: &rusqlite::Connection, session_id: &str) -> Result<()> {
        conn.execute(
            "\
            INSERT INTO agent_session_logs (id, skill_path, execution_mode, status)
            VALUES (?1, 'skills/example/SKILL.md', 'contract', 'running')
            ",
            params![session_id],
        )?;
        Ok(())
    }

    fn sample_schema() -> serde_json::Value {
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

    fn sample_payload() -> serde_json::Value {
        json!({
            "notice_id": "ocds-123",
            "score_bps": 8200,
            "items": []
        })
    }

    fn healthy_oqg() -> OqgCheckInput {
        OqgCheckInput {
            expected_fill_bps: 9000,
            observed_fill_bps: 9000,
            duplicate_rate_bps: 100,
            max_duplicate_rate_bps: 500,
            observed_values: vec![100, 200, 300],
            allowed_min_value: 50,
            allowed_max_value: 500,
        }
    }

    fn sample_runtime_context() -> ErrorRuntimeContext {
        ErrorRuntimeContext {
            cli_vendor: Some("codex".to_owned()),
            cli_version: Some("1.2.3".to_owned()),
            adapter_version: Some("0.1.0".to_owned()),
            model_id: Some("gpt-5".to_owned()),
            config_hash: Some("cfg-123".to_owned()),
            skill_version: Some("2026.03.25".to_owned()),
            pid: Some(100),
            pgid: Some(200),
            cleanup_verdict: Some("Clean".to_owned()),
            last_valid_checkpoint_sequence: Some(4),
            daemon_instance_id: Some("daemon-1".to_owned()),
            commit_seq: Some(12),
        }
    }

    fn sample_error_draft(
        error_type: ErrorCode,
        error_context: serde_json::Value,
    ) -> ErrorManifestDraft {
        ErrorManifestDraft {
            job_id: Some("job-123".to_owned()),
            process_id: Some("process-123".to_owned()),
            pipeline_stage: "qualification".to_owned(),
            workflow_step: "schema-validate".to_owned(),
            error_type,
            error_message: "contract path failed".to_owned(),
            error_context,
            skill_path: Some("skills/fts/SKILL.md".to_owned()),
            agent_session_id: Some("session-123".to_owned()),
            runtime_context: sample_runtime_context(),
            session_artifact_path: Some("artifacts/sessions/2026/03/session-123".to_owned()),
            next_debugging_action:
                "Inspect the session artifacts and rerun after fixing the root cause.".to_owned(),
        }
    }

    fn sample_manifest_appendix() -> DaemonEvidenceAppendix {
        DaemonEvidenceAppendix {
            observed_url_chain: vec!["file://local-process".to_owned()],
            observed_page_kind_sequence: vec!["local-data".to_owned()],
            daemon_observed_fingerprint_state: "matched".to_owned(),
            daemon_observed_locale: "en-GB".to_owned(),
            daemon_observed_currency: "GBP".to_owned(),
            auth_challenge_events: vec!["none".to_owned()],
            locator_fallback_depth: 0,
            checkpoint_count: 0,
            result_item_count: 1,
            oqg_anomaly_summary: Vec::new(),
            representative_item_sample: json!({"process_id":"process-123"}),
            artifact_refs: vec!["manifests/process-123.json".to_owned()],
        }
    }

    fn fixed_started_at() -> Result<chrono::DateTime<chrono::Utc>> {
        chrono::Utc
            .with_ymd_and_hms(2026, 3, 25, 3, 0, 0)
            .single()
            .ok_or_else(|| anyhow::anyhow!("failed to construct fixed test timestamp"))
    }

    fn seed_job(
        conn: &rusqlite::Connection,
        job_id: &str,
        process_id: Option<&str>,
        status: &str,
        input_payload: &serde_json::Value,
    ) -> Result<()> {
        conn.execute(
            "\
            INSERT INTO workflow_jobs (
              id, kind, process_id, scope_kind, scope_id, status, input_payload
            ) VALUES (?1, 'stage-job', ?2, 'notice', 'notice-1', ?3, ?4)
            ",
            params![job_id, process_id, status, input_payload.to_string()],
        )?;
        Ok(())
    }

    fn seed_stage_head(
        conn: &rusqlite::Connection,
        process_id: &str,
        stage: &str,
        scope_kind: &str,
        scope_id: &str,
        current_stage_run_id: &str,
    ) -> Result<()> {
        conn.execute(
            "\
            INSERT INTO process_stage_heads (
              process_id, stage, scope_kind, scope_id, current_stage_run_id, owner_id
            ) VALUES (?1, ?2, ?3, ?4, ?5, 'local-owner')
            ",
            params![
                process_id,
                stage,
                scope_kind,
                scope_id,
                current_stage_run_id
            ],
        )?;
        Ok(())
    }

    fn seed_error_manifest_refs(conn: &rusqlite::Connection) -> Result<()> {
        seed_process(conn, "process-123")?;
        conn.execute(
            "\
            INSERT INTO workflow_jobs (
              id, kind, process_id, scope_kind, scope_id, status, input_payload
            ) VALUES (?1, 'stage-job', ?2, 'notice', 'notice-1', 'failed', '{}')
            ",
            params!["job-123", "process-123"],
        )?;
        conn.execute(
            "\
            INSERT INTO agent_session_logs (id, job_id, skill_path, execution_mode, status)
            VALUES (?1, ?2, 'skills/fts/SKILL.md', 'contract', 'failed')
            ",
            params!["session-123", "job-123"],
        )?;
        Ok(())
    }

    #[tokio::test]
    async fn ut_con_001_contract_mode_preserves_stdout_and_stderr_separately() -> Result<()> {
        let stdout = OutputFrame::Stdout(Bytes::from_static(b"stdout-frame"));
        let stderr = OutputFrame::Stderr(Bytes::from_static(b"stderr-frame"));
        ensure!(
            stdout == OutputFrame::Stdout(Bytes::from_static(b"stdout-frame")),
            "stdout frame changed type or content"
        );
        ensure!(
            stderr == OutputFrame::Stderr(Bytes::from_static(b"stderr-frame")),
            "stderr frame changed type or content"
        );
        Ok(())
    }

    #[tokio::test]
    async fn ut_con_002_parser_events_are_emitted_after_capture_sequence() -> Result<()> {
        let mut parser = DummyParser {
            mode: ExecutionMode::Contract,
        };
        let mut captured_frames = Vec::new();
        let frame = OutputFrame::Stdout(Bytes::from_static(b"checkpoint:1"));
        captured_frames.push(frame.clone());
        let events = parser.on_output(&frame).await?;
        ensure!(
            captured_frames.len() == 1,
            "capture sequence did not record exactly one frame"
        );
        assert_checkpoint_event(&events[0], 1, "checkpoint-1.json")?;
        Ok(())
    }

    #[tokio::test]
    async fn ut_con_003_final_result_ready_exposes_path_only() -> Result<()> {
        let event = ControlEvent::FinalResultReady {
            path: PathBuf::from("result.json"),
        };
        match event {
            ControlEvent::FinalResultReady { path } => {
                ensure!(
                    path.as_path() == std::path::Path::new("result.json"),
                    "unexpected final result path"
                );
                Ok(())
            }
            _ => bail!("unexpected control event kind"),
        }
    }

    #[tokio::test]
    async fn ut_con_004_contract_required_never_degrades_to_pty() -> Result<()> {
        let skill = SkillManifest {
            interaction_profile: InteractionProfile::ContractRequired,
        };
        let caps = CapabilityReport {
            cli_version: "1.0.0".to_owned(),
            auth_healthy: true,
            structured_output: false,
            pty_viable: true,
            browser_tools: false,
            is_canary: false,
        };
        ensure!(
            resolve_execution_mode(&skill, &caps).is_err(),
            "contract-required mode unexpectedly degraded to PTY"
        );
        Ok(())
    }

    #[tokio::test]
    async fn ut_con_005_contract_preferred_fallback_obeys_flag() -> Result<()> {
        let fallback_allowed = SkillManifest {
            interaction_profile: InteractionProfile::ContractPreferred {
                allow_pty_fallback: true,
            },
        };
        let fallback_blocked = SkillManifest {
            interaction_profile: InteractionProfile::ContractPreferred {
                allow_pty_fallback: false,
            },
        };
        let caps = CapabilityReport {
            cli_version: "1.0.0".to_owned(),
            auth_healthy: true,
            structured_output: false,
            pty_viable: true,
            browser_tools: false,
            is_canary: false,
        };
        let fallback_mode = resolve_execution_mode(&fallback_allowed, &caps)?;
        ensure!(
            fallback_mode == ExecutionMode::Pty,
            "contract-preferred fallback did not select PTY"
        );
        ensure!(
            resolve_execution_mode(&fallback_blocked, &caps).is_err(),
            "contract-preferred mode ignored allow_pty_fallback=false"
        );
        Ok(())
    }

    #[tokio::test]
    async fn it_con_019_adapter_contract_surface_is_exercised_end_to_end() -> Result<()> {
        let adapter = DummyAdapter;
        ensure!(adapter.id() == "dummy", "unexpected adapter identity");

        let probe_context = ProbeCtx {
            working_directory: PathBuf::from("."),
        };
        let caps = adapter.probe(&probe_context).await?;
        ensure!(caps.auth_healthy, "probe should mark auth healthy");
        ensure!(
            caps.browser_tools,
            "probe should mark browser tools available"
        );
        ensure!(!caps.is_canary, "probe should not mark canary mode");

        let skill = SkillManifest {
            interaction_profile: InteractionProfile::PtyRequired,
        };
        ensure!(
            adapter.resolve_mode(&skill, &caps)? == ExecutionMode::Pty,
            "pty-required skill did not resolve to PTY"
        );

        let request = SpawnRequest {
            program: PathBuf::from("/bin/echo"),
            args: vec![OsString::from("hello")],
            env: BTreeMap::from([(OsString::from("A"), OsString::from("1"))]),
            cwd: PathBuf::from("/tmp"),
        };
        let spec = adapter.build_launch_spec(&request, ExecutionMode::Pty)?;
        ensure!(
            spec.program.as_path() == std::path::Path::new("/bin/echo"),
            "unexpected launch program"
        );
        ensure!(
            spec.args == vec![OsString::from("hello")],
            "unexpected launch args"
        );
        ensure!(
            spec.cwd.as_path() == std::path::Path::new("/tmp"),
            "unexpected launch cwd"
        );
        ensure!(
            spec.env.get(&OsString::from("A")) == Some(&OsString::from("1")),
            "unexpected launch env"
        );

        let mut parser = adapter.parser(ExecutionMode::Contract);
        let finished = parser.finish().await?;
        assert_exit_status_success(&finished[0])?;

        let pty_frame = OutputFrame::Pty(Bytes::from_static(b"pty"));
        ensure!(
            pty_frame == OutputFrame::Pty(Bytes::from_static(b"pty")),
            "unexpected PTY frame"
        );

        assert_auth_challenge(
            ControlEvent::AuthChallenge {
                kind: AuthChallengeKind::Mfa,
                detail: "mfa".to_owned(),
            },
            &AuthChallengeKind::Mfa,
            "mfa",
        )?;

        assert_warning_event(
            ControlEvent::Warning {
                code: "warn".to_owned(),
                detail: "detail".to_owned(),
            },
            "warn",
            "detail",
        )?;

        let started_event = ControlEvent::Started {
            pid: 1,
            pgid: 2,
            sid: 3,
            mode: ExecutionMode::Pty,
        };
        assert_started_event(&started_event, (1, 2, 3))?;

        let progress_event = ControlEvent::Progress {
            completed: 1,
            total: Some(2),
        };
        assert_progress_event(&progress_event, 1, Some(2))?;

        for kind in [
            AuthChallengeKind::ReLogin,
            AuthChallengeKind::Sso,
            AuthChallengeKind::PermissionPrompt,
            AuthChallengeKind::Unknown,
        ] {
            let detail = format!("{kind:?}");
            assert_auth_challenge(
                ControlEvent::AuthChallenge {
                    kind: kind.clone(),
                    detail: detail.clone(),
                },
                &kind,
                &detail,
            )?;
        }

        Ok(())
    }

    #[test]
    fn it_con_005_claim_next_job_marks_running_and_parses_payload() -> Result<()> {
        let state_db = TestStateDb::new()?;
        seed_process(state_db.connection(), "process-claim")?;
        seed_job(
            state_db.connection(),
            "job-claim",
            Some("process-claim"),
            "pending",
            &json!({"skill":"dummy","scope":"notice-1"}),
        )?;
        let runner = JobRunner::new("executor-claim");

        let claimed = runner
            .claim_next_job(state_db.connection())?
            .ok_or_else(|| anyhow::anyhow!("expected a claimable job"))?;
        let (status, lease_owner): (String, Option<String>) = state_db.connection().query_row(
            "SELECT status, lease_owner FROM workflow_jobs WHERE id = ?1",
            params!["job-claim"],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;

        ensure!(claimed.id == "job-claim", "claimed wrong job");
        ensure!(
            claimed.input_payload["scope"] == "notice-1",
            "claim did not parse input payload"
        );
        ensure!(status == "running", "claimed job not marked running");
        ensure!(
            lease_owner.as_deref() == Some("executor-claim"),
            "lease_owner not written from executor id"
        );
        let executor_count: i64 = state_db.connection().query_row(
            "SELECT COUNT(*) FROM executors WHERE id = ?1 AND status = 'ready'",
            params!["executor-claim"],
            |row| row.get(0),
        )?;
        ensure!(
            executor_count == 1,
            "executor row was not registered on claim"
        );
        Ok(())
    }

    #[test]
    fn it_con_006_validated_checkpoints_update_last_checkpoint_seq_and_persist_in_order()
    -> Result<()> {
        let ctx = TestContext::new()?;
        ensure!(ctx.temp.path().exists(), "test tempdir missing");
        ensure!(
            ctx.db_path.ends_with("state/state.db"),
            "test db path changed"
        );
        ensure!(
            ctx.state_root.ends_with("state"),
            "state_root shape changed"
        );
        ensure!(
            ctx.runtime_root.ends_with("runtime"),
            "runtime_root shape changed"
        );
        ensure!(
            ctx.artifact_root.ends_with("artifacts"),
            "artifact_root shape changed"
        );
        let state_db = TestStateDb::new()?;
        ensure!(
            state_db.db_path().ends_with("state.db"),
            "sqlite test db path changed"
        );
        seed_session(state_db.connection(), "session-checkpoint")?;

        let first = persist_validated_checkpoint(state_db.connection(), "session-checkpoint", 1)?;
        let second = persist_validated_checkpoint(state_db.connection(), "session-checkpoint", 2)?;
        let stored: Option<u32> = state_db.connection().query_row(
            "SELECT last_checkpoint_seq FROM agent_session_logs WHERE id = ?1",
            params!["session-checkpoint"],
            |row| row.get(0),
        )?;

        ensure!(first == 1, "first checkpoint did not persist");
        ensure!(second == 2, "second checkpoint did not persist");
        ensure!(stored == Some(2), "last_checkpoint_seq did not update");
        Ok(())
    }

    #[test]
    fn it_con_007_out_of_order_checkpoint_sequencing_is_rejected() -> Result<()> {
        let state_db = TestStateDb::new()?;
        seed_session(state_db.connection(), "session-out-of-order")?;
        let _ = persist_validated_checkpoint(state_db.connection(), "session-out-of-order", 1)?;

        let error = persist_validated_checkpoint(state_db.connection(), "session-out-of-order", 3)
            .expect_err("out-of-order checkpoint should fail");
        ensure!(
            error.to_string().contains(ERROR_PARTIAL_FAILURE),
            "out-of-order checkpoint did not classify as AGENT_PARTIAL_FAILURE"
        );
        Ok(())
    }

    #[test]
    fn it_con_008_schema_valid_final_result_advances_success_path_only_after_validator_success()
    -> Result<()> {
        let state_db = TestStateDb::new()?;
        seed_process(state_db.connection(), "process-success")?;

        let outcome = validate_final_result(
            &sample_schema(),
            &sample_payload(),
            &healthy_oqg(),
            std::path::Path::new("results/result.json"),
        );
        ensure!(
            outcome.disposition == ValidationDisposition::Accepted,
            "valid result should be accepted"
        );

        let write = StageValidationWrite {
            process_id: "process-success",
            stage: "pricing",
            scope_kind: "notice",
            scope_id: "notice-1",
            stage_run_id: "run-success",
            owner_id: "local-owner",
        };
        persist_stage_validation(state_db.connection(), &write, &outcome)?;

        let current_stage_run_id: String = state_db.connection().query_row(
            "SELECT current_stage_run_id FROM process_stage_heads WHERE process_id = ?1 AND stage = ?2 AND scope_kind = ?3 AND scope_id = ?4",
            params!["process-success", "pricing", "notice", "notice-1"],
            |row| row.get(0),
        )?;
        ensure!(
            current_stage_run_id == "run-success",
            "stage head did not advance"
        );
        Ok(())
    }

    #[test]
    fn it_con_009_schema_invalid_final_result_fails_job_and_writes_error_context() -> Result<()> {
        let invalid_payload = json!({
            "notice_id": "ocds-123",
            "items": []
        });
        let outcome = validate_final_result(
            &sample_schema(),
            &invalid_payload,
            &healthy_oqg(),
            std::path::Path::new("results/result.json"),
        );
        ensure!(
            outcome.disposition == ValidationDisposition::Failed,
            "schema-invalid payload should fail"
        );
        ensure!(
            outcome.error_code == Some(ERROR_OUTPUT_CONTRACT_INVALID),
            "schema-invalid payload classified incorrectly"
        );
        let error_context = outcome
            .error_context_json
            .ok_or_else(|| anyhow::anyhow!("missing error context"))?;
        ensure!(
            error_context.contains("score_bps"),
            "error context missing failing field details"
        );

        let state_db = TestStateDb::new()?;
        let ctx = TestContext::new()?;
        let store = ErrorManifestStore::new(&ctx.artifact_root)?;
        seed_error_manifest_refs(state_db.connection())?;
        let stored = store.create(
            state_db.connection(),
            &sample_error_draft(
                ErrorCode::AgentOutputContractInvalid,
                serde_json::from_str(&error_context)?,
            ),
            fixed_started_at()?,
        )?;
        let record = ErrorManifestStore::load_record(state_db.connection(), &stored.id)?;
        ensure!(
            record.error_type == ErrorCode::AgentOutputContractInvalid,
            "schema-invalid error manifest classified incorrectly"
        );
        Ok(())
    }

    #[test]
    fn it_con_010_oqg_failure_quarantines_outputs_and_blocks_pipeline_head_advancement()
    -> Result<()> {
        let state_db = TestStateDb::new()?;
        seed_process(state_db.connection(), "process-quarantine")?;

        let oqg = OqgCheckInput {
            expected_fill_bps: 9000,
            observed_fill_bps: 7000,
            duplicate_rate_bps: 900,
            max_duplicate_rate_bps: 500,
            observed_values: vec![100, 5_000],
            allowed_min_value: 50,
            allowed_max_value: 500,
        };
        let outcome = validate_final_result(
            &sample_schema(),
            &sample_payload(),
            &oqg,
            std::path::Path::new("results/result.json"),
        );
        ensure!(
            outcome.disposition == ValidationDisposition::Quarantined,
            "OQG failure should quarantine"
        );
        ensure!(
            outcome.error_code == Some(ERROR_SUSPECT_OUTPUT),
            "OQG failure classified incorrectly"
        );

        let state_db_for_manifest = TestStateDb::new()?;
        let ctx = TestContext::new()?;
        let store = ErrorManifestStore::new(&ctx.artifact_root)?;
        seed_error_manifest_refs(state_db_for_manifest.connection())?;
        let error_context = outcome
            .error_context_json
            .clone()
            .ok_or_else(|| anyhow::anyhow!("missing OQG error context"))?;
        let stored_manifest = store.create(
            state_db_for_manifest.connection(),
            &sample_error_draft(
                ErrorCode::AgentSuspectOutput,
                serde_json::from_str(&error_context)?,
            ),
            fixed_started_at()?,
        )?;
        let record = ErrorManifestStore::load_record(
            state_db_for_manifest.connection(),
            &stored_manifest.id,
        )?;
        ensure!(
            record.error_type == ErrorCode::AgentSuspectOutput,
            "OQG error manifest classified incorrectly"
        );

        let write = StageValidationWrite {
            process_id: "process-quarantine",
            stage: "pricing",
            scope_kind: "notice",
            scope_id: "notice-1",
            stage_run_id: "run-quarantine",
            owner_id: "local-owner",
        };
        persist_stage_validation(state_db.connection(), &write, &outcome)?;

        let head_count: i64 = state_db.connection().query_row(
            "SELECT COUNT(*) FROM process_stage_heads WHERE process_id = ?1 AND stage = ?2",
            params!["process-quarantine", "pricing"],
            |row| row.get(0),
        )?;
        ensure!(
            head_count == 0,
            "quarantined output should not advance stage head"
        );
        Ok(())
    }

    #[test]
    fn it_con_011_partial_failure_with_valid_checkpoint_never_upgrades_to_success_without_final_contract_and_clean_reap()
    -> Result<()> {
        let classification = classify_terminal_failure(true, false, "Clean", 0, None::<&str>);
        ensure!(
            classification == Some(ERROR_PARTIAL_FAILURE),
            "checkpoint-only run should classify as AGENT_PARTIAL_FAILURE"
        );

        let state_db = TestStateDb::new()?;
        let ctx = TestContext::new()?;
        let store = ErrorManifestStore::new(&ctx.artifact_root)?;
        seed_error_manifest_refs(state_db.connection())?;
        let stored = store.create(
            state_db.connection(),
            &sample_error_draft(
                ErrorCode::AgentPartialFailure,
                json!({"checkpoint_seq": 1, "cleanup_verdict": "Clean"}),
            ),
            fixed_started_at()?,
        )?;
        let record = ErrorManifestStore::load_record(state_db.connection(), &stored.id)?;
        ensure!(
            record.error_type == ErrorCode::AgentPartialFailure,
            "partial-failure error manifest classified incorrectly"
        );
        ErrorManifestStore::resolve(
            state_db.connection(),
            &stored.id,
            "operator@example.com",
            fixed_started_at()?,
        )?;
        Ok(())
    }

    #[tokio::test]
    async fn it_con_016_stderr_only_contract_sessions_preserved_on_output_lane_without_semantic_loss()
    -> Result<()> {
        let stderr = OutputFrame::Stderr(Bytes::from_static(b"stderr-only"));
        ensure!(
            stderr == OutputFrame::Stderr(Bytes::from_static(b"stderr-only")),
            "stderr-only contract frame was not preserved"
        );
        Ok(())
    }

    #[test]
    fn e2e_con_014_non_zero_exit_without_specific_pattern_is_classified_agent_run_failed()
    -> Result<()> {
        let classification = classify_terminal_failure(false, false, "Forced", 17, None::<&str>);
        ensure!(
            classification == Some(ERROR_RUN_FAILED),
            "non-zero exit should classify as AGENT_RUN_FAILED"
        );
        Ok(())
    }

    fn load_manifest_generation_skill() -> Result<(crate::skills_support::LintedSkill, PathBuf)> {
        let skill_dir = crate::skills_support::repo_skill_dir("manifest-generation")?;
        let copied_skill = TestSkillDir::from_fixture(&skill_dir)?;
        let copied_markdown = copied_skill.read_skill_markdown()?;
        let rebuilt_skill = TestSkillDir::from_fixture_filtered(&skill_dir, Some("SKILL.md"))?;
        rebuilt_skill.overwrite("SKILL.md", &copied_markdown)?;
        let linted = lint_skill_directory(copied_skill.path())?;
        Ok((linted, skill_dir))
    }

    fn build_manifest_generation_result_payload(skill_dir: &Path) -> Result<serde_json::Value> {
        let result_schema: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(skill_dir.join("result.schema.json"))?)?;
        let basis_input = json!({
            "process_id": "process-123",
            "candidate_count": 2,
            "margin_band": "high"
        })
        .to_string();
        let decision_basis_hash = format!("{:x}", Sha256::digest(basis_input.as_bytes()));
        let result_payload = json!({
            "process_id": "process-123",
            "decisionBasisHash": decision_basis_hash,
            "manifestMarkdown": "# Opportunity Manifest\n\nStrong opportunity.",
            "candidate_count": 2
        });
        validate_result_payload(&result_schema, &result_payload)?;
        Ok(result_payload)
    }

    fn write_manifest_generation_result_artifact(
        ctx: &TestContext,
        result_payload: &serde_json::Value,
    ) -> Result<(ArtifactStore, PathBuf)> {
        let artifact_store = ArtifactStore::new(&ctx.artifact_root)?;
        ensure!(
            artifact_store.root() == ctx.artifact_root.as_path(),
            "artifact store root mismatch"
        );
        let result_artifact_path = ctx.artifact_root.join("manifests").join("process-123.json");
        fs::create_dir_all(
            result_artifact_path
                .parent()
                .ok_or_else(|| anyhow::anyhow!("missing result artifact parent"))?,
        )?;
        fs::write(
            &result_artifact_path,
            serde_json::to_string_pretty(result_payload)?,
        )?;
        Ok((artifact_store, result_artifact_path))
    }

    fn verify_manifest_index_reader(ctx: &TestContext) -> Result<()> {
        let index_path = ctx.artifact_root.join("manifest.index.jsonl");
        fs::write(
            &index_path,
            concat!(
                "{\"seq\":1,\"path\":\"manifests/process-123.json\"}\n",
                "{\"seq\":2,\"path\":\"manifests/process-124.json\""
            ),
        )?;
        let contiguous_prefix = read_contiguous_jsonl_prefix(&index_path)?;
        ensure!(
            contiguous_prefix.len() == 1,
            "contiguous prefix reader did not stop at torn line"
        );
        Ok(())
    }

    fn create_manifest_generation_run_manifest(
        artifact_store: &ArtifactStore,
        linted: &crate::skills_support::LintedSkill,
    ) -> Result<StoredManifest> {
        let stored_manifest = artifact_store.write_run_manifest(
            &RunManifestDraft {
                frontmatter: RunManifestFrontmatter {
                    manifest_version: 1,
                    skill_id: linted.metadata.skill_id.clone(),
                    skill_version: linted.metadata.skill_version.clone(),
                    job_id: "job-manifest".to_owned(),
                    session_id: "session-manifest".to_owned(),
                    execution_mode: "contract".to_owned(),
                    started_at: fixed_started_at()?.to_rfc3339(),
                    completed_at: fixed_started_at()?.to_rfc3339(),
                    outcome_class: OutcomeClass::Completed,
                    items_processed: 1,
                    items_skipped: 0,
                    items_failed: 0,
                    warnings_count: 0,
                    agent_claimed_pages_visited: 0,
                    agent_claimed_source_url: "https://local.example/process-123".to_owned(),
                    agent_claimed_fingerprint_state: "matched".to_owned(),
                    agent_claimed_locale_state: "en-GB".to_owned(),
                    agent_claimed_currency_state: "GBP".to_owned(),
                    agent_narrative_missing: false,
                },
                narrative: Some(RunNarrative {
                    summary: "Generated a deterministic process-scoped opportunity manifest."
                        .to_owned(),
                    steps_taken:
                        "Read local process data, synthesized markdown, and wrote artifacts."
                            .to_owned(),
                    decisions_made:
                        "Used only local pipeline data and emitted a single manifest payload."
                            .to_owned(),
                    warnings: "No warnings.".to_owned(),
                    evidence: "See the result payload and manifest artifact refs.".to_owned(),
                }),
                appendix: sample_manifest_appendix(),
                disagreements: Vec::new(),
            },
            fixed_started_at()?,
        )?;

        let fallback = RunManifestDraft::fallback(
            RunManifestFrontmatter {
                manifest_version: 1,
                skill_id: "manifest-generation".to_owned(),
                skill_version: "1.0.0".to_owned(),
                job_id: "job-manifest-fallback".to_owned(),
                session_id: "session-manifest-fallback".to_owned(),
                execution_mode: "contract".to_owned(),
                started_at: fixed_started_at()?.to_rfc3339(),
                completed_at: fixed_started_at()?.to_rfc3339(),
                outcome_class: OutcomeClass::BlockedHumanAction,
                items_processed: 0,
                items_skipped: 0,
                items_failed: 1,
                warnings_count: 1,
                agent_claimed_pages_visited: 0,
                agent_claimed_source_url: "https://local.example/process-123".to_owned(),
                agent_claimed_fingerprint_state: "unknown".to_owned(),
                agent_claimed_locale_state: "unknown".to_owned(),
                agent_claimed_currency_state: "unknown".to_owned(),
                agent_narrative_missing: false,
            },
            sample_manifest_appendix(),
            "agent narrative missing",
        );
        ensure!(
            fallback.frontmatter.agent_narrative_missing,
            "fallback manifest did not flip agent_narrative_missing"
        );
        Ok(stored_manifest)
    }

    fn load_manifest_generation_manifest_metadata(
        linted: &crate::skills_support::LintedSkill,
        stored_manifest: &StoredManifest,
    ) -> Result<(PathBuf, String)> {
        let state_db = TestStateDb::new()?;
        state_db.connection().execute(
            "\
            INSERT INTO agent_session_logs (id, skill_path, skill_version, execution_mode, status)
            VALUES (?1, ?2, ?3, 'contract', 'completed')
            ",
            params![
                "session-manifest",
                "skills/manifest-generation/SKILL.md",
                &linted.metadata.skill_version,
            ],
        )?;
        persist_run_manifest_metadata(state_db.connection(), "session-manifest", stored_manifest)?;

        let (manifest_path, manifest_hash): (String, String) = state_db.connection().query_row(
            "SELECT run_manifest_path, run_manifest_hash FROM agent_session_logs WHERE id = ?1",
            params!["session-manifest"],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        Ok((PathBuf::from(manifest_path), manifest_hash))
    }

    #[test]
    fn e2e_con_013_manifest_generation_skill_completes_in_contract_mode_and_persists_result_artifact_and_run_manifest()
    -> Result<()> {
        let (linted, skill_dir) = load_manifest_generation_skill()?;
        ensure!(
            linted.metadata.interaction_profile == "headless_contract",
            "manifest-generation skill must be headless_contract"
        );

        let rendered_prompt = PromptRenderer::render(&PromptRendererInput {
            skill_id: linted.metadata.skill_id.clone(),
            skill_version: linted.metadata.skill_version.clone(),
            skill_markdown: linted.skill_markdown.clone(),
            job_id: "job-manifest".to_owned(),
            session_id: "session-manifest".to_owned(),
            execution_mode: "contract".to_owned(),
            job_parameters: json!({"process_id":"process-123"}),
            resume_token: None,
            company_profile: None,
        })?;
        ensure!(
            rendered_prompt.contains(RUN_MANIFEST_INSTRUCTION_BLOCK),
            "rendered prompt missing run-manifest instruction block"
        );

        let ctx = TestContext::new()?;
        let result_payload = build_manifest_generation_result_payload(&skill_dir)?;
        let (artifact_store, result_artifact_path) =
            write_manifest_generation_result_artifact(&ctx, &result_payload)?;
        verify_manifest_index_reader(&ctx)?;
        let stored_manifest = create_manifest_generation_run_manifest(&artifact_store, &linted)?;
        let (manifest_path, manifest_hash) =
            load_manifest_generation_manifest_metadata(&linted, &stored_manifest)?;

        ensure!(manifest_path.exists(), "run manifest was not persisted");
        ensure!(
            !manifest_hash.is_empty(),
            "run manifest hash was not persisted"
        );
        ensure!(
            result_artifact_path.exists(),
            "manifest-generation result artifact was not persisted"
        );
        Ok(())
    }

    #[test]
    fn it_con_012_sqlite_write_failure_emits_no_new_visible_after_seq_and_advances_no_stage_head()
    -> Result<()> {
        let state_db = TestStateDb::new()?;
        seed_process(state_db.connection(), "process-write-failure")?;
        seed_job(
            state_db.connection(),
            "job-write-failure",
            Some("process-write-failure"),
            "pending",
            &json!({"skill":"dummy"}),
        )?;
        let mut owned_db = StateDb::open(state_db.db_path())?;
        let runner = JobRunner::new("executor-1");
        let before_commit_seq = owned_db.current_commit_seq()?;

        let request = PersistSuccessRequest {
            job_id: "job-write-failure".to_owned(),
            idempotency_key: "ack-write-failure".to_owned(),
            session_id: "session-write-failure".to_owned(),
            output_payload: json!({"result": true}),
            stage: "Manifest".to_owned(),
            scope_kind: "process".to_owned(),
            scope_id: "missing-process".to_owned(),
            stage_run_id: "run-write-failure".to_owned(),
            process_id: "missing-process".to_owned(),
            owner_id: "local-owner".to_owned(),
        };
        let accepted = ValidationOutcome {
            disposition: ValidationDisposition::Accepted,
            error_code: None,
            error_context_json: None,
            anomalies: Vec::new(),
        };

        let error = runner
            .persist_success(&mut owned_db, &request, &accepted)
            .expect_err("foreign-key failure should abort write");
        let after_commit_seq = owned_db.current_commit_seq()?;
        let head_count: i64 = owned_db.connection().query_row(
            "SELECT COUNT(*) FROM process_stage_heads WHERE current_stage_run_id = ?1",
            params!["run-write-failure"],
            |row| row.get(0),
        )?;

        ensure!(
            before_commit_seq == after_commit_seq,
            "failed write should not advance commit fence"
        );
        ensure!(
            head_count == 0,
            "failed write should not advance stage head"
        );
        let sqlite_error = error
            .chain()
            .find_map(|cause| cause.downcast_ref::<rusqlite::Error>());
        ensure!(
            classify_sqlite_write_failed(
                sqlite_error.ok_or_else(|| anyhow::anyhow!("expected rusqlite error"))?
            )
            .to_string()
            .contains(SQLITE_WRITE_FAILED),
            "write failure was not classified as SQLITE_WRITE_FAILED"
        );
        Ok(())
    }

    #[test]
    fn it_con_015_continuation_required_preserves_committed_work_and_enqueues_follow_on_with_resume_token()
    -> Result<()> {
        let state_db = TestStateDb::new()?;
        seed_process(state_db.connection(), "process-continuation")?;
        seed_job(
            state_db.connection(),
            "job-continuation",
            Some("process-continuation"),
            "pending",
            &json!({"skill":"dummy"}),
        )?;
        let mut owned_db = StateDb::open(state_db.db_path())?;
        let runner = JobRunner::new("executor-1");

        let ack = runner.persist_continuation(
            &mut owned_db,
            &PersistContinuationRequest {
                job_id: "job-continuation".to_owned(),
                idempotency_key: "ack-continuation".to_owned(),
                continuation_job_id: "job-continuation-2".to_owned(),
                continuation_chain_id: "chain-1".to_owned(),
                next_job_kind: "stage-job".to_owned(),
                process_id: Some("process-continuation".to_owned()),
                scope_kind: Some("notice".to_owned()),
                scope_id: Some("notice-1".to_owned()),
                resume_token: json!({"cursor":"next-page"}),
                owner_id: "local-owner".to_owned(),
            },
        )?;

        let status: String = owned_db.connection().query_row(
            "SELECT status FROM workflow_jobs WHERE id = ?1",
            params!["job-continuation"],
            |row| row.get(0),
        )?;
        let next_input_payload: String = owned_db.connection().query_row(
            "SELECT input_payload FROM workflow_jobs WHERE id = ?1",
            params!["job-continuation-2"],
            |row| row.get(0),
        )?;

        ensure!(ack.accepted, "continuation ack should be accepted");
        ensure!(
            ack.continuation_job_id.as_deref() == Some("job-continuation-2"),
            "continuation ack missing follow-on job id"
        );
        ensure!(
            ack.visible_after_seq > 0,
            "continuation ack missing visible_after_seq"
        );
        ensure!(
            status == "completed",
            "originating job did not persist committed work"
        );
        ensure!(
            next_input_payload.contains("next-page"),
            "follow-on job missing resume token"
        );
        Ok(())
    }

    #[test]
    fn it_con_017_retrying_lost_mutation_ack_with_same_idempotency_context_returns_original_visible_after_seq_no_duplicates()
    -> Result<()> {
        let state_db = TestStateDb::new()?;
        seed_process(state_db.connection(), "process-idempotent")?;
        seed_job(
            state_db.connection(),
            "job-idempotent",
            Some("process-idempotent"),
            "pending",
            &json!({"skill":"dummy"}),
        )?;
        let mut owned_db = StateDb::open(state_db.db_path())?;
        let runner = JobRunner::new("executor-1");
        let accepted = ValidationOutcome {
            disposition: ValidationDisposition::Accepted,
            error_code: None,
            error_context_json: None,
            anomalies: Vec::new(),
        };
        let request = PersistSuccessRequest {
            job_id: "job-idempotent".to_owned(),
            idempotency_key: "ack-idempotent".to_owned(),
            session_id: "session-idempotent".to_owned(),
            output_payload: json!({"result": true}),
            stage: "Manifest".to_owned(),
            scope_kind: "process".to_owned(),
            scope_id: "process-idempotent".to_owned(),
            stage_run_id: "run-idempotent".to_owned(),
            process_id: "process-idempotent".to_owned(),
            owner_id: "local-owner".to_owned(),
        };

        let first = runner.persist_success(&mut owned_db, &request, &accepted)?;
        let second = runner.persist_success(&mut owned_db, &request, &accepted)?;
        let status_row_count: i64 = owned_db.connection().query_row(
            "SELECT COUNT(*) FROM process_stage_status WHERE process_id = ?1 AND stage_run_id = ?2",
            params!["process-idempotent", "run-idempotent"],
            |row| row.get(0),
        )?;

        ensure!(
            first.visible_after_seq == second.visible_after_seq,
            "idempotent retry returned a different visibleAfterSeq"
        );
        ensure!(
            status_row_count == 1,
            "idempotent retry duplicated stage writes"
        );
        Ok(())
    }

    #[test]
    fn it_con_018_concurrent_wal_readers_hold_stable_snapshot_and_competing_writer_surfaces_sqlite_write_failed()
    -> Result<()> {
        let state_db = TestStateDb::new()?;
        seed_process(state_db.connection(), "process-wal")?;
        state_db.connection().execute(
            "\
            INSERT INTO process_stage_heads (
              process_id, stage, scope_kind, scope_id, current_stage_run_id, owner_id
            ) VALUES (?1, 'pricing', 'notice', 'notice-1', 'run-initial', 'local-owner')
            ",
            params!["process-wal"],
        )?;

        let db_path = state_db.db_path().to_path_buf();
        let writer = rusqlite::Connection::open(&db_path)?;
        writer.pragma_update(None, "busy_timeout", 1_000_i64)?;
        let reader = rusqlite::Connection::open(&db_path)?;
        let competing_writer = rusqlite::Connection::open(&db_path)?;
        competing_writer.pragma_update(None, "busy_timeout", 1_i64)?;

        let read_tx = reader.unchecked_transaction()?;
        let snapshot_before: String = read_tx.query_row(
            "SELECT current_stage_run_id FROM process_stage_heads WHERE process_id = ?1",
            params!["process-wal"],
            |row| row.get(0),
        )?;

        let write_tx = writer.unchecked_transaction()?;
        write_tx.execute(
            "UPDATE process_stage_heads SET current_stage_run_id = 'run-updated' WHERE process_id = ?1",
            params!["process-wal"],
        )?;

        let competing_error = competing_writer
            .execute(
                "UPDATE process_stage_heads SET current_stage_run_id = 'run-competing' WHERE process_id = ?1",
                params!["process-wal"],
            )
            .expect_err("competing writer should exhaust busy timeout");
        let classified = classify_sqlite_write_failed(&competing_error).to_string();

        let snapshot_during: String = read_tx.query_row(
            "SELECT current_stage_run_id FROM process_stage_heads WHERE process_id = ?1",
            params!["process-wal"],
            |row| row.get(0),
        )?;
        write_tx.commit()?;
        read_tx.commit()?;
        let snapshot_after: String = reader.query_row(
            "SELECT current_stage_run_id FROM process_stage_heads WHERE process_id = ?1",
            params!["process-wal"],
            |row| row.get(0),
        )?;

        ensure!(
            snapshot_before == "run-initial" && snapshot_during == "run-initial",
            "reader snapshot was not stable during concurrent write"
        );
        ensure!(
            snapshot_after == "run-updated",
            "post-commit reader did not observe writer commit"
        );
        ensure!(
            classified.contains(SQLITE_WRITE_FAILED),
            "competing writer did not classify as SQLITE_WRITE_FAILED"
        );
        Ok(())
    }

    #[test]
    fn it_con_020_stage_to_scope_mapping_matches_the_six_canonical_scope_kinds() -> Result<()> {
        let cases = [
            (
                "Normalize",
                Some("notice"),
                StageCommitSemantics::SetReplacement,
            ),
            (
                "Relate",
                Some("process"),
                StageCommitSemantics::SetReplacement,
            ),
            (
                "Qualify lot",
                Some("lot"),
                StageCommitSemantics::SetReplacement,
            ),
            (
                "Extract products",
                Some("lot"),
                StageCommitSemantics::Additive,
            ),
            (
                "Price-signal match",
                Some("candidate_source_pair"),
                StageCommitSemantics::Additive,
            ),
            (
                "Landed cost",
                Some("candidate_quote_scenario"),
                StageCommitSemantics::Additive,
            ),
        ];

        for (stage, expected_scope_kind, expected_semantics) in cases {
            let rule = stage_scope_rule(stage)?;
            ensure!(
                rule.scope_kind == expected_scope_kind,
                "unexpected scope_kind for stage {stage}"
            );
            ensure!(
                rule.commit_semantics == expected_semantics,
                "unexpected commit semantics for stage {stage}"
            );
        }
        Ok(())
    }

    #[test]
    fn it_con_021_set_replacement_stages_never_leak_partial_writes_or_head_movement() -> Result<()>
    {
        let state_db = TestStateDb::new()?;
        seed_process(state_db.connection(), "process-set-replacement")?;
        seed_stage_head(
            state_db.connection(),
            "process-set-replacement",
            "Normalize",
            "notice",
            "notice-1",
            "run-old",
        )?;
        let failed_run_id = allocate_stage_run_id();
        let write = StageValidationWrite {
            process_id: "process-set-replacement",
            stage: "Normalize",
            scope_kind: "notice",
            scope_id: "notice-1",
            stage_run_id: &failed_run_id,
            owner_id: "local-owner",
        };

        persist_failed_set_replacement_attempt(state_db.connection(), &write, Some("error-1"))?;

        let current_stage_run_id: String = state_db.connection().query_row(
            "SELECT current_stage_run_id FROM process_stage_heads WHERE process_id = ?1 AND stage = ?2 AND scope_kind = ?3 AND scope_id = ?4",
            params!["process-set-replacement", "Normalize", "notice", "notice-1"],
            |row| row.get(0),
        )?;
        let failed_status_count: i64 = state_db.connection().query_row(
            "SELECT COUNT(*) FROM process_stage_status WHERE process_id = ?1 AND stage_run_id = ?2 AND state = 'failed'",
            params!["process-set-replacement", failed_run_id],
            |row| row.get(0),
        )?;

        ensure!(
            current_stage_run_id == "run-old",
            "failed set-replacement attempt advanced stage head"
        );
        ensure!(failed_status_count == 1, "failed attempt not recorded");
        Ok(())
    }

    #[test]
    fn it_con_022_additive_stages_commit_multiple_scoped_units_from_one_session() -> Result<()> {
        let state_db = TestStateDb::new()?;
        seed_process(state_db.connection(), "process-additive")?;
        seed_job(
            state_db.connection(),
            "job-additive-a",
            Some("process-additive"),
            "pending",
            &json!({"skill":"dummy"}),
        )?;
        seed_job(
            state_db.connection(),
            "job-additive-b",
            Some("process-additive"),
            "pending",
            &json!({"skill":"dummy"}),
        )?;
        let mut owned_db = StateDb::open(state_db.db_path())?;
        let runner = JobRunner::new("executor-additive");
        let accepted = ValidationOutcome {
            disposition: ValidationDisposition::Accepted,
            error_code: None,
            error_context_json: None,
            anomalies: Vec::new(),
        };

        let first_run_id = allocate_stage_run_id();
        let second_run_id = allocate_stage_run_id();
        ensure!(
            first_run_id != second_run_id,
            "stage_run_id must be fresh per unit"
        );

        let _ = runner.persist_success(
            &mut owned_db,
            &PersistSuccessRequest {
                job_id: "job-additive-a".to_owned(),
                idempotency_key: "ack-additive-a".to_owned(),
                session_id: "session-additive".to_owned(),
                output_payload: json!({"result": "a"}),
                stage: "Gross margin gate".to_owned(),
                scope_kind: "candidate".to_owned(),
                scope_id: "candidate-a".to_owned(),
                stage_run_id: first_run_id,
                process_id: "process-additive".to_owned(),
                owner_id: "local-owner".to_owned(),
            },
            &accepted,
        )?;
        let _ = runner.persist_success(
            &mut owned_db,
            &PersistSuccessRequest {
                job_id: "job-additive-b".to_owned(),
                idempotency_key: "ack-additive-b".to_owned(),
                session_id: "session-additive".to_owned(),
                output_payload: json!({"result": "b"}),
                stage: "Gross margin gate".to_owned(),
                scope_kind: "candidate".to_owned(),
                scope_id: "candidate-b".to_owned(),
                stage_run_id: second_run_id,
                process_id: "process-additive".to_owned(),
                owner_id: "local-owner".to_owned(),
            },
            &accepted,
        )?;

        let committed_scope_count: i64 = owned_db.connection().query_row(
            "SELECT COUNT(*) FROM process_stage_heads WHERE process_id = ?1 AND stage = ?2",
            params!["process-additive", "Gross margin gate"],
            |row| row.get(0),
        )?;
        ensure!(
            committed_scope_count == 2,
            "additive stage did not preserve both committed scope units"
        );
        Ok(())
    }

    #[test]
    fn it_con_023_expected_input_basis_hash_mismatch_marks_job_obsolete_and_enqueues_rerun()
    -> Result<()> {
        let state_db = TestStateDb::new()?;
        seed_process(state_db.connection(), "process-basis")?;
        seed_job(
            state_db.connection(),
            "job-basis",
            Some("process-basis"),
            "pending",
            &json!({"skill":"dummy"}),
        )?;
        let mut owned_db = StateDb::open(state_db.db_path())?;
        let runner = JobRunner::new("executor-basis");

        ensure!(
            basis_hash_mismatch("expected-hash", "actual-hash"),
            "basis hash mismatch helper failed"
        );
        let ack = runner.mark_obsolete_and_enqueue_rerun(
            &mut owned_db,
            &ObsoleteJobRequest {
                job_id: "job-basis".to_owned(),
                idempotency_key: "ack-basis".to_owned(),
                rerun_job_id: "job-basis-rerun".to_owned(),
                next_job_kind: "stage-job".to_owned(),
                process_id: Some("process-basis".to_owned()),
                scope_kind: Some("notice".to_owned()),
                scope_id: Some("notice-1".to_owned()),
                rerun_from_stage: "Normalize".to_owned(),
                expected_input_basis_hash: "expected-hash".to_owned(),
                actual_input_basis_hash: "actual-hash".to_owned(),
            },
        )?;

        let (status, last_error): (String, Option<String>) = owned_db.connection().query_row(
            "SELECT status, last_error FROM workflow_jobs WHERE id = ?1",
            params!["job-basis"],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        let rerun_count: i64 = owned_db.connection().query_row(
            "SELECT COUNT(*) FROM workflow_jobs WHERE id = ?1 AND parent_job_id = ?2",
            params!["job-basis-rerun", "job-basis"],
            |row| row.get(0),
        )?;

        ensure!(ack.accepted, "obsolete ack should be accepted");
        ensure!(
            ack.continuation_job_id.as_deref() == Some("job-basis-rerun"),
            "rerun job id missing from obsolete ack"
        );
        ensure!(
            status == "obsolete",
            "basis-hash mismatch did not mark job obsolete"
        );
        ensure!(
            last_error
                .as_deref()
                .unwrap_or_default()
                .contains(JOB_OBSOLETE),
            "obsolete job missing mismatch error context"
        );
        ensure!(
            rerun_count == 1,
            "basis-hash mismatch did not enqueue rerun"
        );
        Ok(())
    }

    #[test]
    fn it_con_024_additive_stage_preserves_successful_units_when_sibling_fails() -> Result<()> {
        let state_db = TestStateDb::new()?;
        seed_process(state_db.connection(), "process-additive-mixed")?;
        seed_job(
            state_db.connection(),
            "job-additive-success",
            Some("process-additive-mixed"),
            "pending",
            &json!({"skill":"dummy"}),
        )?;
        let mut owned_db = StateDb::open(state_db.db_path())?;
        let runner = JobRunner::new("executor-additive-mixed");
        let accepted = ValidationOutcome {
            disposition: ValidationDisposition::Accepted,
            error_code: None,
            error_context_json: None,
            anomalies: Vec::new(),
        };

        let _ = runner.persist_success(
            &mut owned_db,
            &PersistSuccessRequest {
                job_id: "job-additive-success".to_owned(),
                idempotency_key: "ack-additive-success".to_owned(),
                session_id: "session-additive-mixed".to_owned(),
                output_payload: json!({"result": "success-a"}),
                stage: "Gross margin gate".to_owned(),
                scope_kind: "candidate".to_owned(),
                scope_id: "candidate-a".to_owned(),
                stage_run_id: "stage-run-additive-a".to_owned(),
                process_id: "process-additive-mixed".to_owned(),
                owner_id: "local-owner".to_owned(),
            },
            &accepted,
        )?;

        owned_db.connection().execute(
            "\
            INSERT INTO process_stage_status (
              id, process_id, stage, scope_kind, scope_id, stage_run_id, state,
              freshness_state, started_at, completed_at, error_manifest_id,
              input_revision, output_revision, owner_id
            ) VALUES (
              'stage-run-additive-b-failed', 'process-additive-mixed', 'Gross margin gate',
              'candidate', 'candidate-b', 'stage-run-additive-b', 'failed', 'fresh',
              '2026-03-25T00:00:00Z', '2026-03-25T00:00:01Z', NULL, 0, 0, 'local-owner'
            )
            ",
            [],
        )?;

        let success_heads: i64 = owned_db.connection().query_row(
            "SELECT COUNT(*) FROM process_stage_heads WHERE process_id = ?1 AND stage = ?2",
            params!["process-additive-mixed", "Gross margin gate"],
            |row| row.get(0),
        )?;
        let failed_status_rows: i64 = owned_db.connection().query_row(
            "SELECT COUNT(*) FROM process_stage_status WHERE process_id = ?1 AND stage = ?2 AND state = 'failed'",
            params!["process-additive-mixed", "Gross margin gate"],
            |row| row.get(0),
        )?;

        ensure!(
            success_heads == 1,
            "successful additive scope head was not preserved"
        );
        ensure!(
            failed_status_rows == 1,
            "failed additive sibling status row was not recorded"
        );
        Ok(())
    }

    #[test]
    fn it_con_025_soft_stale_vs_hard_invalid_follow_staleness_cascade_rules() -> Result<()> {
        let state_db = TestStateDb::new()?;
        seed_process(state_db.connection(), "process-staleness")?;
        seed_stage_head(
            state_db.connection(),
            "process-staleness",
            "Manifest",
            "process",
            "process-staleness",
            "run-stage",
        )?;
        ensure!(
            FreshnessState::Fresh.as_str() == "fresh",
            "fresh freshness state changed"
        );

        set_stage_head_freshness(
            state_db.connection(),
            "process-staleness",
            "Manifest",
            "process",
            "process-staleness",
            FreshnessState::SoftStale,
        )?;
        let soft_state: String = state_db.connection().query_row(
            "SELECT freshness_state FROM process_stage_heads WHERE process_id = ?1",
            params!["process-staleness"],
            |row| row.get(0),
        )?;
        set_stage_head_freshness(
            state_db.connection(),
            "process-staleness",
            "Manifest",
            "process",
            "process-staleness",
            FreshnessState::HardInvalid,
        )?;
        let hard_state: String = state_db.connection().query_row(
            "SELECT freshness_state FROM process_stage_heads WHERE process_id = ?1",
            params!["process-staleness"],
            |row| row.get(0),
        )?;

        ensure!(soft_state == "soft_stale", "soft-stale state not persisted");
        ensure!(
            hard_state == "hard_invalid",
            "hard-invalid state not persisted"
        );
        Ok(())
    }

    #[test]
    fn it_con_026_one_session_exercising_set_replacement_and_additive_proves_stage_run_id_scoping()
    -> Result<()> {
        let set_replacement = stage_scope_rule("Normalize")?;
        let additive = stage_scope_rule("Gross margin gate")?;
        let set_replacement_run_id = allocate_stage_run_id();
        let additive_run_id_a = allocate_stage_run_id();
        let additive_run_id_b = allocate_stage_run_id();

        ensure!(
            set_replacement.commit_semantics == StageCommitSemantics::SetReplacement,
            "set-replacement stage rule changed"
        );
        ensure!(
            additive.commit_semantics == StageCommitSemantics::Additive,
            "additive stage rule changed"
        );
        ensure!(
            set_replacement_run_id != additive_run_id_a
                && additive_run_id_a != additive_run_id_b
                && set_replacement_run_id != additive_run_id_b,
            "stage_run_id allocation reused cohort ids"
        );
        Ok(())
    }
}
