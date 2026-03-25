#![forbid(unsafe_code)]

#[path = "../src/agents/adapter.rs"]
mod adapter_impl;
#[path = "../src/agents/claude.rs"]
mod claude_impl;

#[cfg(unix)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};
    use std::ffi::OsString;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::{Path, PathBuf};

    use anyhow::{Result, bail, ensure};
    use bytes::Bytes;
    use tempfile::TempDir;

    use crate::adapter_impl::{
        AgentAdapter, AuthChallengeKind, ControlEvent, ExecutionMode, OutputFrame, ProbeCtx,
        SkillManifest, SpawnRequest,
    };
    use crate::claude_impl::ClaudeAdapter;

    const KNOWN_VERSION: &str = "1.2.3";

    struct FakeClaudeCli {
        _tempdir: TempDir,
        program: PathBuf,
        state_dir: PathBuf,
    }

    impl FakeClaudeCli {
        fn new(version: &str, probe_json: &str) -> Result<Self> {
            let tempdir = TempDir::new()?;
            let state_dir = tempdir.path().join("state");
            fs::create_dir_all(&state_dir)?;
            fs::write(state_dir.join("version.txt"), format!("claude {version}\n"))?;
            fs::write(state_dir.join("probe.json"), probe_json)?;

            let program = tempdir.path().join("claude");
            fs::write(
                &program,
                format!(
                    "#!/bin/sh\nset -eu\nstate_dir=\"{}\"\ncase \"${{1-}}\" in\n  --version)\n    cat \"$state_dir/version.txt\"\n    ;;\n  probe)\n    if [ \"${{CLAUDE_TEST_FAIL_PROBE:-0}}\" = \"1\" ]; then\n      echo 'probe failed' >&2\n      exit 7\n    fi\n    cat \"$state_dir/probe.json\"\n    ;;\n  *)\n    echo 'unknown command' >&2\n    exit 64\n    ;;\nesac\n",
                    state_dir.display()
                ),
            )?;
            let mut permissions = fs::metadata(&program)?.permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(&program, permissions)?;

            Ok(Self {
                _tempdir: tempdir,
                program,
                state_dir,
            })
        }

        fn set_version(&self, version: &str) -> Result<()> {
            fs::write(
                self.state_dir.join("version.txt"),
                format!("claude {version}\n"),
            )?;
            Ok(())
        }
    }

    fn known_versions() -> BTreeSet<String> {
        std::iter::once(KNOWN_VERSION.to_owned()).collect()
    }

    fn probe_context(path: &Path) -> ProbeCtx {
        ProbeCtx {
            working_directory: path.to_path_buf(),
        }
    }

    fn program_dir(program: &Path) -> &Path {
        program.parent().unwrap_or_else(|| Path::new("."))
    }

    fn parse_protocol_error(error: &anyhow::Error) -> String {
        error.to_string()
    }

    fn assert_started(event: &ControlEvent, expected_ids: (u32, u32, u32)) -> Result<()> {
        match event {
            ControlEvent::Started {
                pid,
                pgid,
                sid,
                mode,
            } => {
                ensure!(
                    (*pid, *pgid, *sid) == expected_ids,
                    "unexpected started identifiers"
                );
                ensure!(*mode == ExecutionMode::Contract, "unexpected started mode");
                Ok(())
            }
            _ => bail!("expected started event"),
        }
    }

    fn assert_progress(event: &ControlEvent, completed: u32, total: Option<u32>) -> Result<()> {
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
            _ => bail!("expected progress event"),
        }
    }

    fn assert_checkpoint(event: &ControlEvent, seq: u32, path: &str) -> Result<()> {
        match event {
            ControlEvent::Checkpoint {
                seq: actual_seq,
                path: actual_path,
            } => {
                ensure!(*actual_seq == seq, "unexpected checkpoint sequence");
                ensure!(
                    actual_path.as_path() == Path::new(path),
                    "unexpected checkpoint path"
                );
                Ok(())
            }
            _ => bail!("expected checkpoint event"),
        }
    }

    fn assert_warning(event: &ControlEvent, code: &str, detail: &str) -> Result<()> {
        match event {
            ControlEvent::Warning {
                code: actual_code,
                detail: actual_detail,
            } => {
                ensure!(actual_code == code, "unexpected warning code");
                ensure!(actual_detail == detail, "unexpected warning detail");
                Ok(())
            }
            _ => bail!("expected warning event"),
        }
    }

    #[tokio::test]
    async fn ut_cld_001_probe_reports_all_normative_capability_fields() -> Result<()> {
        let cli = FakeClaudeCli::new(
            KNOWN_VERSION,
            r#"{"auth_healthy":true,"structured_output":true,"pty_viable":true,"browser_tools":false}"#,
        )?;
        let adapter = ClaudeAdapter::new(cli.program.clone(), known_versions());

        let report = adapter
            .probe(&probe_context(program_dir(&cli.program)))
            .await?;

        ensure!(
            report.cli_version == KNOWN_VERSION,
            "unexpected CLI version"
        );
        ensure!(report.auth_healthy, "expected auth_healthy");
        ensure!(report.structured_output, "expected structured_output");
        ensure!(report.pty_viable, "expected pty_viable");
        ensure!(!report.browser_tools, "expected browser_tools=false");
        ensure!(
            !report.is_canary,
            "known version unexpectedly marked canary"
        );
        Ok(())
    }

    #[tokio::test]
    async fn ut_cld_002_unknown_claude_versions_enter_canary_mode() -> Result<()> {
        let cli = FakeClaudeCli::new(
            "9.9.9",
            r#"{"auth_healthy":true,"structured_output":true,"pty_viable":true,"browser_tools":true}"#,
        )?;
        let adapter = ClaudeAdapter::new(cli.program.clone(), known_versions());

        let report = adapter
            .probe(&probe_context(program_dir(&cli.program)))
            .await?;
        ensure!(
            report.is_canary,
            "unknown Claude version was not marked canary"
        );
        Ok(())
    }

    #[tokio::test]
    async fn ut_cld_003_probe_failure_classifies_explicit_adapter_internal_error() -> Result<()> {
        let adapter = ClaudeAdapter::new(
            PathBuf::from("/definitely/not/installed/claude"),
            known_versions(),
        );
        let Err(error) = adapter.probe(&probe_context(Path::new("."))).await else {
            bail!("missing Claude binary should fail");
        };
        ensure!(
            parse_protocol_error(&error).contains("AGENT_INTERNAL_ERROR"),
            "probe failure did not classify as AGENT_INTERNAL_ERROR"
        );
        Ok(())
    }

    #[tokio::test]
    async fn ut_cld_004_resolve_mode_honors_pty_required_deterministically() -> Result<()> {
        let cli = FakeClaudeCli::new(
            KNOWN_VERSION,
            r#"{"auth_healthy":true,"structured_output":true,"pty_viable":true,"browser_tools":true}"#,
        )?;
        let adapter = ClaudeAdapter::new(cli.program.clone(), known_versions());
        let skill = SkillManifest {
            interaction_profile: crate::adapter_impl::InteractionProfile::PtyRequired,
        };
        let caps = adapter
            .probe(&probe_context(program_dir(&cli.program)))
            .await?;
        ensure!(
            adapter.resolve_mode(&skill, &caps)? == ExecutionMode::Pty,
            "pty-required skill did not resolve to PTY"
        );
        Ok(())
    }

    #[tokio::test]
    async fn ut_cld_005_resolve_mode_honors_contract_required_deterministically() -> Result<()> {
        let cli = FakeClaudeCli::new(
            KNOWN_VERSION,
            r#"{"auth_healthy":true,"structured_output":true,"pty_viable":false,"browser_tools":true}"#,
        )?;
        let adapter = ClaudeAdapter::new(cli.program.clone(), known_versions());
        let skill = SkillManifest {
            interaction_profile: crate::adapter_impl::InteractionProfile::ContractRequired,
        };
        let caps = adapter
            .probe(&probe_context(program_dir(&cli.program)))
            .await?;
        ensure!(
            adapter.resolve_mode(&skill, &caps)? == ExecutionMode::Contract,
            "contract-required skill did not resolve to contract mode"
        );
        Ok(())
    }

    #[tokio::test]
    async fn ut_cld_006_resolve_mode_runs_the_full_fallback_matrix() -> Result<()> {
        let cli = FakeClaudeCli::new(
            KNOWN_VERSION,
            r#"{"auth_healthy":true,"structured_output":true,"pty_viable":true,"browser_tools":true}"#,
        )?;
        let adapter = ClaudeAdapter::new(cli.program, known_versions());

        let cases = [
            (
                crate::adapter_impl::InteractionProfile::PtyRequired,
                true,
                true,
                Some(ExecutionMode::Pty),
            ),
            (
                crate::adapter_impl::InteractionProfile::ContractRequired,
                true,
                false,
                Some(ExecutionMode::Contract),
            ),
            (
                crate::adapter_impl::InteractionProfile::ContractPreferred {
                    allow_pty_fallback: true,
                },
                true,
                false,
                Some(ExecutionMode::Contract),
            ),
            (
                crate::adapter_impl::InteractionProfile::ContractPreferred {
                    allow_pty_fallback: true,
                },
                false,
                true,
                Some(ExecutionMode::Pty),
            ),
            (
                crate::adapter_impl::InteractionProfile::ContractPreferred {
                    allow_pty_fallback: false,
                },
                false,
                true,
                None,
            ),
            (
                crate::adapter_impl::InteractionProfile::ContractPreferred {
                    allow_pty_fallback: true,
                },
                false,
                false,
                None,
            ),
        ];

        for (interaction_profile, structured_output, pty_viable, expected_mode) in cases {
            let skill = SkillManifest {
                interaction_profile,
            };
            let caps = crate::adapter_impl::CapabilityReport {
                cli_version: KNOWN_VERSION.to_owned(),
                auth_healthy: true,
                structured_output,
                pty_viable,
                browser_tools: true,
                is_canary: false,
            };
            let actual = adapter.resolve_mode(&skill, &caps).ok();
            ensure!(
                actual == expected_mode,
                "fallback matrix row resolved unexpectedly"
            );
        }

        Ok(())
    }

    #[tokio::test]
    async fn ut_cld_007_build_launch_spec_returns_isolated_launch_inputs() -> Result<()> {
        let cli = FakeClaudeCli::new(
            KNOWN_VERSION,
            r#"{"auth_healthy":true,"structured_output":true,"pty_viable":true,"browser_tools":true}"#,
        )?;
        let adapter = ClaudeAdapter::new(cli.program.clone(), known_versions());
        let session_root = TempDir::new()?;
        let request = SpawnRequest {
            program: PathBuf::from("/ignored/by/adapter"),
            args: vec![OsString::from("--headless")],
            env: BTreeMap::from([(OsString::from("ORIGINAL"), OsString::from("1"))]),
            cwd: session_root.path().to_path_buf(),
        };

        let spec = adapter.build_launch_spec(&request, ExecutionMode::Contract)?;

        ensure!(
            spec.program == cli.program,
            "launch spec should use Claude executable"
        );
        ensure!(
            spec.args
                == vec![
                    OsString::from("--output-format"),
                    OsString::from("stream-json"),
                    OsString::from("--headless"),
                ],
            "unexpected contract launch arguments"
        );
        ensure!(
            spec.cwd == request.cwd,
            "launch spec cwd changed unexpectedly"
        );
        ensure!(
            spec.env.get(&OsString::from("ORIGINAL")) == Some(&OsString::from("1")),
            "original environment was not preserved"
        );
        ensure!(
            spec.env.get(&OsString::from("CLAUDE_CONFIG_DIR"))
                == Some(
                    &session_root
                        .path()
                        .join(".claude")
                        .join("config")
                        .into_os_string()
                ),
            "isolated config dir missing"
        );
        ensure!(
            spec.env.get(&OsString::from("CLAUDE_BROWSER_PROFILE_DIR"))
                == Some(
                    &session_root
                        .path()
                        .join(".claude")
                        .join("browser-profile")
                        .into_os_string()
                ),
            "isolated browser profile dir missing"
        );
        ensure!(
            spec.env
                .contains_key(&OsString::from("CLAUDE_ADAPTER_VERSION")),
            "adapter version marker missing"
        );
        Ok(())
    }

    #[tokio::test]
    async fn ut_cld_008_parser_emits_canonical_events_while_output_bytes_stay_lossless()
    -> Result<()> {
        let cli = FakeClaudeCli::new(
            KNOWN_VERSION,
            r#"{"auth_healthy":true,"structured_output":true,"pty_viable":true,"browser_tools":true}"#,
        )?;
        let adapter = ClaudeAdapter::new(cli.program.clone(), known_versions());
        let mut parser = adapter.parser(ExecutionMode::Contract);

        let frames = vec![
            OutputFrame::Stdout(Bytes::from_static(
                br#"{"event":"started","pid":11,"pgid":12,"sid":13}
{"event":"checkpoint","seq":2,"path":"checkpoint-2.json"}
"#,
            )),
            OutputFrame::Stdout(Bytes::from_static(
                br#"
{"type":"progress","completed":1,"#,
            )),
            OutputFrame::Stderr(Bytes::from_static(
                br#"{"event":"warning","code":"stderr","detail":"noise"}
"#,
            )),
            OutputFrame::Stdout(Bytes::from_static(
                br#""total":2}
{"event":"result","path":"result.json"}
{"event":"exited","code":0}
"#,
            )),
        ];

        let captured = frames.clone();
        let mut events = Vec::new();
        for frame in &frames {
            events.extend(parser.on_output(frame).await?);
        }

        ensure!(captured == frames, "raw output frames were mutated");
        ensure!(events.len() == 6, "unexpected number of canonical events");
        assert_started(&events[0], (11, 12, 13))?;
        assert_checkpoint(&events[1], 2, "checkpoint-2.json")?;
        assert_warning(&events[2], "stderr", "noise")?;
        assert_progress(&events[3], 1, Some(2))?;
        match &events[4] {
            ControlEvent::FinalResultReady { path } => {
                ensure!(
                    path.as_path() == Path::new("result.json"),
                    "unexpected result path"
                );
            }
            _ => bail!("expected final result event"),
        }
        match &events[5] {
            ControlEvent::Exited { status } => {
                ensure!(status.success(), "expected successful exit status");
            }
            _ => bail!("expected exited event"),
        }

        let mut pty_parser = adapter.parser(ExecutionMode::Contract);
        let pty_error = pty_parser
            .on_output(&OutputFrame::Pty(Bytes::from_static(b"pty")))
            .await;
        ensure!(
            pty_error.is_err(),
            "PTY bytes should fail in Claude contract parser"
        );
        Ok(())
    }

    #[tokio::test]
    async fn ut_cld_009_parser_finish_runs_exactly_once() -> Result<()> {
        let cli = FakeClaudeCli::new(
            KNOWN_VERSION,
            r#"{"auth_healthy":true,"structured_output":true,"pty_viable":true,"browser_tools":true}"#,
        )?;
        let adapter = ClaudeAdapter::new(cli.program.clone(), known_versions());
        let mut parser = adapter.parser(ExecutionMode::Contract);

        let first = parser.finish().await?;
        ensure!(
            first.is_empty(),
            "clean parser finish should not fabricate events"
        );

        let second = parser.finish().await;
        ensure!(second.is_err(), "second finish call should fail");
        Ok(())
    }

    #[tokio::test]
    async fn ut_cld_010_malformed_and_truncated_streams_fail_explicitly() -> Result<()> {
        let cli = FakeClaudeCli::new(
            KNOWN_VERSION,
            r#"{"auth_healthy":true,"structured_output":true,"pty_viable":true,"browser_tools":true}"#,
        )?;
        let adapter = ClaudeAdapter::new(cli.program.clone(), known_versions());

        let mut malformed_parser = adapter.parser(ExecutionMode::Contract);
        let malformed = malformed_parser
            .on_output(&OutputFrame::Stdout(Bytes::from_static(
                br#"{"event":"started","pid":1,"pgid":2,"sid":3"#,
            )))
            .await?;
        ensure!(
            malformed.is_empty(),
            "partial malformed frame should buffer first"
        );
        let Err(malformed_error) = malformed_parser
            .on_output(&OutputFrame::Stdout(Bytes::from_static(b",\"oops\":]}\n")))
            .await
        else {
            bail!("malformed JSON should fail");
        };
        ensure!(
            parse_protocol_error(&malformed_error).contains("AGENT_PROTOCOL_MISMATCH"),
            "malformed JSON did not classify as protocol mismatch"
        );

        let mut truncated_parser = adapter.parser(ExecutionMode::Contract);
        let partial = truncated_parser
            .on_output(&OutputFrame::Stdout(Bytes::from_static(
                br#"{"type":"progress","completed":1"#,
            )))
            .await?;
        ensure!(
            partial.is_empty(),
            "partial stream tail should buffer first"
        );
        let Err(truncated_error) = truncated_parser.finish().await else {
            bail!("truncated tail should fail");
        };
        ensure!(
            parse_protocol_error(&truncated_error).contains("AGENT_PROTOCOL_MISMATCH"),
            "truncated stream tail did not classify as protocol mismatch"
        );

        let mut complete_tail_parser = adapter.parser(ExecutionMode::Contract);
        let buffered = complete_tail_parser
            .on_output(&OutputFrame::Stdout(Bytes::from_static(
                br#"{"type":"progress","completed":3,"#,
            )))
            .await?;
        ensure!(
            buffered.is_empty(),
            "complete tail case should buffer first chunk"
        );
        let final_chunk = complete_tail_parser
            .on_output(&OutputFrame::Stdout(Bytes::from_static(br#""total":4}"#)))
            .await?;
        ensure!(
            final_chunk.is_empty(),
            "parser should wait for finish to flush complete tail"
        );
        let flushed = complete_tail_parser.finish().await?;
        ensure!(
            flushed.len() == 1,
            "expected exactly one flushed progress event"
        );
        assert_progress(&flushed[0], 3, Some(4))?;

        Ok(())
    }

    #[tokio::test]
    async fn it_cld_010_cli_version_changes_invalidate_prior_probe_results() -> Result<()> {
        let cli = FakeClaudeCli::new(
            KNOWN_VERSION,
            r#"{"auth_healthy":true,"structured_output":true,"pty_viable":true,"browser_tools":true}"#,
        )?;
        let adapter = ClaudeAdapter::new(cli.program.clone(), known_versions());
        let context = probe_context(program_dir(&cli.program));

        let first = adapter.probe(&context).await?;
        ensure!(
            first.cli_version == KNOWN_VERSION,
            "unexpected first probe version"
        );
        ensure!(!first.is_canary, "known version unexpectedly marked canary");
        ensure!(
            adapter.id() == "claude",
            "adapter identity changed unexpectedly"
        );

        cli.set_version("9.9.9")?;
        let second = adapter.probe(&context).await?;
        ensure!(
            second.cli_version == "9.9.9",
            "probe did not refresh CLI version"
        );
        ensure!(
            second.is_canary,
            "new unknown version should enter canary mode"
        );
        ensure!(
            adapter.id() == "claude",
            "adapter identity changed across probes"
        );
        Ok(())
    }

    #[tokio::test]
    async fn parser_maps_auth_challenge_variants() -> Result<()> {
        let cli = FakeClaudeCli::new(
            KNOWN_VERSION,
            r#"{"auth_healthy":true,"structured_output":true,"pty_viable":true,"browser_tools":true}"#,
        )?;
        let adapter = ClaudeAdapter::new(cli.program, known_versions());
        let mut parser = adapter.parser(ExecutionMode::Contract);

        let events = parser
            .on_output(&OutputFrame::Stdout(Bytes::from_static(
                br#"{"event":"auth_challenge","kind":"mfa","detail":"login required"}
"#,
            )))
            .await?;
        ensure!(events.len() == 1, "expected one auth challenge event");
        match &events[0] {
            ControlEvent::AuthChallenge { kind, detail } => {
                ensure!(
                    *kind == AuthChallengeKind::Mfa,
                    "unexpected auth challenge kind"
                );
                ensure!(
                    detail == "login required",
                    "unexpected auth challenge detail"
                );
            }
            _ => bail!("expected auth challenge event"),
        }
        Ok(())
    }
}
