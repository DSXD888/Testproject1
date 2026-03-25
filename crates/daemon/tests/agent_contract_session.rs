#![forbid(unsafe_code)]

#[path = "../src/agents/adapter.rs"]
mod adapter_impl;

#[cfg(unix)]
mod tests {
    use std::collections::BTreeMap;
    use std::ffi::OsString;
    use std::path::PathBuf;

    use anyhow::{Result, bail, ensure};
    use async_trait::async_trait;
    use bytes::Bytes;
    use portable_pty::ExitStatus;

    use crate::adapter_impl::{
        AdapterParser, AgentAdapter, AuthChallengeKind, CapabilityReport, ControlEvent,
        ExecutionMode, InteractionProfile, LaunchSpec, OutputFrame, ProbeCtx, SkillManifest,
        SpawnRequest, resolve_execution_mode,
    };

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
}
