#![forbid(unsafe_code)]

#[path = "../src/agents/adapter.rs"]
mod adapter_impl;
#[path = "../src/agents/codex.rs"]
mod codex_impl;

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
    use crate::codex_impl::CodexAdapter;

    const KNOWN_VERSION: &str = "1.2.3";

    struct FakeCodexCli {
        _tempdir: TempDir,
        program: PathBuf,
        state_dir: PathBuf,
    }

    impl FakeCodexCli {
        fn new(version: &str, probe_json: &str) -> Result<Self> {
            let tempdir = TempDir::new()?;
            let state_dir = tempdir.path().join("state");
            fs::create_dir_all(&state_dir)?;
            fs::write(state_dir.join("version.txt"), format!("codex {version}\n"))?;
            fs::write(state_dir.join("probe.json"), probe_json)?;
            fs::write(state_dir.join("probe-mode.txt"), "ok\n")?;

            let program = tempdir.path().join("codex");
            fs::write(
                &program,
                format!(
                    "#!/bin/sh\nset -eu\nstate_dir=\"{}\"\ncase \"${{1-}}\" in\n  --version)\n    cat \"$state_dir/version.txt\"\n    ;;\n  probe)\n    probe_mode=$(cat \"$state_dir/probe-mode.txt\")\n    if [ \"$probe_mode\" = fail ]; then\n      echo 'probe failed' >&2\n      exit 7\n    fi\n    if [ \"$probe_mode\" = unsupported ]; then\n      echo \"error: unexpected argument '--json' found\" >&2\n      exit 2\n    fi\n    cat \"$state_dir/probe.json\"\n    ;;\n  exec)\n    exit 0\n    ;;\n  *)\n    echo 'unknown command' >&2\n    exit 64\n    ;;\nesac\n",
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
                format!("codex {version}\n"),
            )?;
            Ok(())
        }

        fn set_probe_mode(&self, mode: &str) -> Result<()> {
            fs::write(self.state_dir.join("probe-mode.txt"), format!("{mode}\n"))?;
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
    async fn ut_cdx_001_probe_returns_all_normative_capability_fields() -> Result<()> {
        let cli = FakeCodexCli::new(
            KNOWN_VERSION,
            r#"{"auth_healthy":true,"structured_output":true,"pty_viable":true,"browser_tools":false}"#,
        )?;
        let adapter = CodexAdapter::new(cli.program.clone(), known_versions());

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
    async fn ut_cdx_002_unknown_cli_versions_enter_canary_mode() -> Result<()> {
        let cli = FakeCodexCli::new(
            "9.9.9",
            r#"{"auth_healthy":true,"structured_output":true,"pty_viable":true,"browser_tools":true}"#,
        )?;
        let adapter = CodexAdapter::new(cli.program.clone(), known_versions());

        let report = adapter
            .probe(&probe_context(program_dir(&cli.program)))
            .await?;
        ensure!(
            report.is_canary,
            "unknown Codex version was not marked canary"
        );
        Ok(())
    }

    #[tokio::test]
    async fn ut_cdx_003_missing_cli_or_probe_failure_yields_explicit_failure() -> Result<()> {
        let adapter = CodexAdapter::new(
            PathBuf::from("/definitely/not/installed/codex"),
            known_versions(),
        );
        let Err(error) = adapter.probe(&probe_context(Path::new("."))).await else {
            bail!("missing Codex binary should fail");
        };
        ensure!(
            parse_protocol_error(&error).contains("AGENT_INTERNAL_ERROR"),
            "probe failure did not classify as AGENT_INTERNAL_ERROR"
        );
        Ok(())
    }

    #[tokio::test]
    async fn ut_cdx_011_probe_falls_back_when_probe_json_is_unsupported() -> Result<()> {
        let cli = FakeCodexCli::new(
            KNOWN_VERSION,
            r#"{"auth_healthy":false,"structured_output":false,"pty_viable":false,"browser_tools":false}"#,
        )?;
        cli.set_probe_mode("unsupported")?;
        let adapter = CodexAdapter::new(cli.program.clone(), known_versions());

        let report = adapter
            .probe(&probe_context(program_dir(&cli.program)))
            .await?;

        ensure!(
            report.auth_healthy,
            "probe fallback should assume auth is healthy"
        );
        ensure!(
            report.structured_output,
            "probe fallback should assume structured output is available"
        );
        ensure!(
            report.pty_viable,
            "probe fallback should assume PTY is viable"
        );
        ensure!(
            report.browser_tools,
            "probe fallback should assume browser tools are available"
        );
        Ok(())
    }

    #[tokio::test]
    async fn ut_cdx_004_resolve_mode_honors_pty_required_deterministically() -> Result<()> {
        let cli = FakeCodexCli::new(
            KNOWN_VERSION,
            r#"{"auth_healthy":true,"structured_output":true,"pty_viable":true,"browser_tools":true}"#,
        )?;
        let adapter = CodexAdapter::new(cli.program.clone(), known_versions());
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
    async fn ut_cdx_005_resolve_mode_honors_contract_required_deterministically() -> Result<()> {
        let cli = FakeCodexCli::new(
            KNOWN_VERSION,
            r#"{"auth_healthy":true,"structured_output":true,"pty_viable":false,"browser_tools":true}"#,
        )?;
        let adapter = CodexAdapter::new(cli.program.clone(), known_versions());
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
    async fn ut_cdx_006_resolve_mode_runs_the_full_fallback_matrix() -> Result<()> {
        let cli = FakeCodexCli::new(
            KNOWN_VERSION,
            r#"{"auth_healthy":true,"structured_output":true,"pty_viable":true,"browser_tools":true}"#,
        )?;
        let adapter = CodexAdapter::new(cli.program, known_versions());

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
    async fn ut_cdx_007_build_launch_spec_returns_spawn_inputs_only() -> Result<()> {
        let cli = FakeCodexCli::new(
            KNOWN_VERSION,
            r#"{"auth_healthy":true,"structured_output":true,"pty_viable":true,"browser_tools":true}"#,
        )?;
        let adapter = CodexAdapter::new(cli.program.clone(), known_versions());
        let session_root = TempDir::new()?;
        let request = SpawnRequest {
            program: PathBuf::from("/ignored/by/adapter"),
            args: vec![OsString::from("skill-run")],
            env: BTreeMap::from([(OsString::from("ORIGINAL"), OsString::from("1"))]),
            cwd: session_root.path().to_path_buf(),
        };

        let spec = adapter.build_launch_spec(&request, ExecutionMode::Contract)?;

        ensure!(
            spec.program == cli.program,
            "launch spec should use Codex executable"
        );
        ensure!(
            spec.args
                == vec![
                    OsString::from("exec"),
                    OsString::from("--json"),
                    OsString::from("--skip-git-repo-check"),
                    OsString::from("skill-run"),
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
            !spec.env.contains_key(&OsString::from("CODEX_HOME")),
            "session-local CODEX_HOME should stay unset without seeded auth"
        );
        ensure!(
            spec.env.get(&OsString::from("CODEX_BROWSER_PROFILE_DIR"))
                == Some(
                    &session_root
                        .path()
                        .join(".codex-browser-profile")
                        .into_os_string()
                ),
            "isolated browser profile dir missing"
        );
        ensure!(
            spec.env
                .contains_key(&OsString::from("CODEX_ADAPTER_VERSION")),
            "adapter version marker missing"
        );

        let seeded_home = session_root.path().join(".codex-home");
        fs::create_dir_all(&seeded_home)?;
        fs::write(seeded_home.join("auth.json"), "{}")?;
        let seeded_contract = adapter.build_launch_spec(&request, ExecutionMode::Contract)?;
        ensure!(
            seeded_contract.env.get(&OsString::from("CODEX_HOME"))
                == Some(&seeded_home.into_os_string()),
            "session-local CODEX_HOME should be enabled when auth is pre-seeded"
        );

        let pty_spec = adapter.build_launch_spec(&request, ExecutionMode::Pty)?;
        ensure!(
            pty_spec.args == seeded_contract.args,
            "PTY runs should use the same unattended exec invocation as contract runs"
        );
        Ok(())
    }

    #[tokio::test]
    async fn ut_cdx_008_jsonl_parser_translates_vendor_output() -> Result<()> {
        let cli = FakeCodexCli::new(
            KNOWN_VERSION,
            r#"{"auth_healthy":true,"structured_output":true,"pty_viable":true,"browser_tools":true}"#,
        )?;
        let adapter = CodexAdapter::new(cli.program, known_versions());
        let mut parser = adapter.parser(ExecutionMode::Contract);

        let frames = vec![
            OutputFrame::Stdout(Bytes::from_static(
                br#"{"event":"started","pid":11,"pgid":12,"sid":13}
{"event":"checkpoint","seq":2,"path":"checkpoint-2.json"}
"#,
            )),
            OutputFrame::Stderr(Bytes::from_static(
                br#"{"event":"warning","code":"stderr","detail":"noise"}
"#,
            )),
            OutputFrame::Stdout(Bytes::from_static(
                br#"{"event":"progress","completed":1,"total":2}
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
            "PTY bytes should fail in Codex contract parser"
        );
        Ok(())
    }

    #[tokio::test]
    async fn ut_cdx_009_parser_finish_runs_once_and_classifies_abrupt_end() -> Result<()> {
        let cli = FakeCodexCli::new(
            KNOWN_VERSION,
            r#"{"auth_healthy":true,"structured_output":true,"pty_viable":true,"browser_tools":true}"#,
        )?;
        let adapter = CodexAdapter::new(cli.program, known_versions());
        let mut parser = adapter.parser(ExecutionMode::Contract);

        let buffered = parser
            .on_output(&OutputFrame::Stdout(Bytes::from_static(
                br#"{"event":"exited","code":0}"#,
            )))
            .await?;
        ensure!(
            buffered.is_empty(),
            "complete no-newline tail should wait for finish"
        );

        let finished = parser.finish().await?;
        ensure!(finished.len() == 1, "expected exactly one finished event");
        match &finished[0] {
            ControlEvent::Exited { status } => {
                ensure!(status.success(), "expected successful exit status");
            }
            _ => bail!("expected exited event from finish"),
        }

        let second = parser.finish().await;
        ensure!(second.is_err(), "second finish call should fail");
        Ok(())
    }

    #[tokio::test]
    async fn ut_cdx_010_malformed_jsonl_and_truncated_tail_fail_explicitly() -> Result<()> {
        let cli = FakeCodexCli::new(
            KNOWN_VERSION,
            r#"{"auth_healthy":true,"structured_output":true,"pty_viable":true,"browser_tools":true}"#,
        )?;
        let adapter = CodexAdapter::new(cli.program, known_versions());

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
            bail!("malformed JSONL should fail");
        };
        ensure!(
            parse_protocol_error(&malformed_error).contains("AGENT_PROTOCOL_MISMATCH"),
            "malformed JSONL did not classify as protocol mismatch"
        );

        let mut truncated_parser = adapter.parser(ExecutionMode::Contract);
        let partial = truncated_parser
            .on_output(&OutputFrame::Stdout(Bytes::from_static(
                br#"{"event":"progress","completed":1"#,
            )))
            .await?;
        ensure!(partial.is_empty(), "partial JSONL tail should buffer first");
        let Err(truncated_error) = truncated_parser.finish().await else {
            bail!("truncated tail should fail");
        };
        ensure!(
            parse_protocol_error(&truncated_error).contains("AGENT_PROTOCOL_MISMATCH"),
            "truncated tail did not classify as protocol mismatch"
        );

        Ok(())
    }

    #[tokio::test]
    async fn it_cdx_010_cli_version_changes_force_reprobe() -> Result<()> {
        let cli = FakeCodexCli::new(
            KNOWN_VERSION,
            r#"{"auth_healthy":true,"structured_output":true,"pty_viable":true,"browser_tools":true}"#,
        )?;
        let adapter = CodexAdapter::new(cli.program.clone(), known_versions());
        let context = probe_context(program_dir(&cli.program));

        let first = adapter.probe(&context).await?;
        ensure!(
            first.cli_version == KNOWN_VERSION,
            "unexpected first probe version"
        );
        ensure!(!first.is_canary, "known version unexpectedly marked canary");
        ensure!(
            adapter.id() == "codex",
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
            adapter.id() == "codex",
            "adapter identity changed across probes"
        );
        Ok(())
    }

    #[tokio::test]
    async fn parser_maps_auth_challenge_variants() -> Result<()> {
        let cli = FakeCodexCli::new(
            KNOWN_VERSION,
            r#"{"auth_healthy":true,"structured_output":true,"pty_viable":true,"browser_tools":true}"#,
        )?;
        let adapter = CodexAdapter::new(cli.program, known_versions());
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

    #[tokio::test]
    async fn ut_cdx_012_parser_accepts_current_exec_json_streams_and_mixed_pty_text() -> Result<()>
    {
        let cli = FakeCodexCli::new(
            KNOWN_VERSION,
            r#"{"auth_healthy":true,"structured_output":true,"pty_viable":true,"browser_tools":true}"#,
        )?;
        let adapter = CodexAdapter::new(cli.program, known_versions());

        let mut contract_parser = adapter.parser(ExecutionMode::Contract);
        let contract_events = contract_parser
            .on_output(&OutputFrame::Stdout(Bytes::from_static(
                br#"{"type":"thread.started","thread_id":"thread-1"}
{"type":"turn.started"}
{"type":"item.started","item":{"id":"item_0","type":"command_execution","command":"/bin/zsh -lc pwd","aggregated_output":"","exit_code":null,"status":"in_progress"}}
{"type":"item.completed","item":{"id":"item_0","type":"command_execution","command":"/bin/zsh -lc pwd","aggregated_output":"/tmp/workdir\n","exit_code":0,"status":"completed"}}
{"type":"item.completed","item":{"id":"item_1","type":"agent_message","text":"done"}}
{"type":"turn.completed","usage":{"input_tokens":1,"cached_input_tokens":0,"output_tokens":1}}
"#,
            )))
            .await?;
        ensure!(
            contract_events.is_empty(),
            "current codex exec JSONL should be tolerated without synthetic control events"
        );

        let mut pty_parser = adapter.parser(ExecutionMode::Pty);
        let pty_events = pty_parser
            .on_output(&OutputFrame::Pty(Bytes::from_static(
                br#"Reading additional input from stdin...
{"type":"thread.started","thread_id":"thread-1"}
{"type":"item.completed","item":{"id":"item_1","type":"agent_message","text":"done"}}
"#,
            )))
            .await?;
        ensure!(
            pty_events.is_empty(),
            "mixed PTY text plus exec JSONL should not trip the parser"
        );
        Ok(())
    }
}
