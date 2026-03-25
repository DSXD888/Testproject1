#![forbid(unsafe_code)]

mod support;

#[path = "../src/agents/adapter.rs"]
mod adapter_impl;
#[path = "../src/agents/contracts.rs"]
mod contracts_impl;

#[cfg(unix)]
mod tests {
    use std::collections::BTreeMap;
    use std::ffi::OsString;
    use std::path::PathBuf;

    use anyhow::{Result, bail, ensure};
    use async_trait::async_trait;
    use bytes::Bytes;
    use portable_pty::ExitStatus;
    use rusqlite::params;
    use serde_json::json;

    use crate::adapter_impl::{
        AdapterParser, AgentAdapter, AuthChallengeKind, CapabilityReport, ControlEvent,
        ExecutionMode, InteractionProfile, LaunchSpec, OutputFrame, ProbeCtx, SkillManifest,
        SpawnRequest, resolve_execution_mode,
    };
    use crate::contracts_impl::{
        ERROR_OUTPUT_CONTRACT_INVALID, ERROR_PARTIAL_FAILURE, ERROR_RUN_FAILED,
        ERROR_SUSPECT_OUTPUT, OqgCheckInput, StageValidationWrite, ValidationDisposition,
        classify_terminal_failure, persist_stage_validation, persist_validated_checkpoint,
        validate_final_result,
    };
    use crate::support::TestContext;
    use crate::support::db::TestStateDb;

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
    async fn adapter_contract_surface_is_exercised_end_to_end() -> Result<()> {
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
}
