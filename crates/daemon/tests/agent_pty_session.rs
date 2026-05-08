#![forbid(unsafe_code)]
#![allow(clippy::duplicate_mod)]

mod support;

#[path = "support/artifacts.rs"]
mod artifacts_support;
#[path = "../src/pty/health.rs"]
mod health_impl;
#[path = "../src/pty/session.rs"]
mod session_impl;

#[cfg(unix)]
mod tests {
    use anyhow::Result;
    use chrono::Utc;
    use nix::unistd::{getpgid, getsid};
    use serde::Deserialize;
    use std::fs;
    use std::io::{BufRead, BufReader, Write};
    use std::time::Duration;

    use crate::artifacts_support::{
        ArtifactSandbox, CaptureConfig, PtyCapture, decode_chunk, ensure_contiguous,
    };
    use crate::health_impl::{
        HealthIssue, HostResources, StallTracker, detect_auth_challenge, detect_host_sleep,
        detect_resource_pressure, pre_spawn_failure_phases,
    };
    use crate::session_impl::{
        BrowserViewport, PtyCommand, PtyGeometry, PtyPairHandle, PtySession, SessionPhase,
        SpawnOptions, SpawnPath, spawn_policy_steps,
    };
    use crate::support;
    use crate::support::process::{ProcessIsolationGuard, ProcessTreeHelper};

    #[derive(Debug, Deserialize)]
    struct PtyReport {
        isatty_stdin: bool,
        isatty_stdout: bool,
        isatty_stderr: bool,
        tty: String,
        pid: u32,
        pgid: u32,
        sid: u32,
    }

    #[test]
    fn ut_pty_001_pair_is_created_before_spawn() -> Result<()> {
        let ctx = support::TestContext::new()?;
        assert!(ctx.temp.path().exists());
        assert!(ctx.state_root.ends_with("state"));
        assert!(ctx.runtime_root.ends_with("runtime"));
        assert!(ctx.artifact_root.ends_with("artifacts"));
        assert!(ctx.db_path.ends_with("state/state.db"));

        let state_db = support::db::TestStateDb::new()?;
        assert!(state_db.db_path().exists());
        assert_eq!(
            state_db.connection().query_row(
                "SELECT commit_seq FROM commit_clock WHERE id = 1",
                [],
                |row| { row.get::<_, i64>(0) }
            )?,
            0
        );

        let pair = PtyPairHandle::open(PtyGeometry::default())?;
        assert_eq!(pair.geometry(), PtyGeometry::new(120, 40));
        assert!(pair.master_fd().is_some());
        Ok(())
    }

    #[test]
    fn ut_pty_002_spawn_policy_orders_pre_exec_steps() {
        assert_eq!(
            spawn_policy_steps(),
            [
                "openpty",
                "setsid",
                "controlling_terminal",
                "stdio_redirect",
                "exec",
            ]
        );
    }

    #[test]
    fn ut_pty_003_runtime_ids_exist_before_spawned_session_is_returned() -> Result<()> {
        let mut session = PtySession::spawn(report_command()?, SpawnOptions::default())?;
        assert!(session.runtime_ids.sid > 0);
        assert!(session.runtime_ids.pid > 0);
        assert!(session.runtime_ids.pgid > 0);
        assert_eq!(session.browser_viewport, BrowserViewport::default());
        assert_eq!(session.spawn_path, SpawnPath::PortablePty);
        assert!(session.master_fd.is_some());
        assert_eq!(
            session
                .phase_history
                .iter()
                .map(|phase| phase.as_str())
                .collect::<Vec<_>>(),
            vec!["created", "spawned", "active"]
        );
        let _ = session.read_line()?;
        let status = session.finish()?;
        assert!(status.success());
        Ok(())
    }

    #[test]
    fn ut_pty_004_master_fd_is_marked_cloexec() -> Result<()> {
        let pair = PtyPairHandle::open(PtyGeometry::default())?;
        assert!(pair.master_has_cloexec()?);
        Ok(())
    }

    #[test]
    fn ut_pty_005_helper_path_is_selected_when_pre_exec_safety_is_unavailable() -> Result<()> {
        assert_eq!(SpawnPath::resolve(false), SpawnPath::HelperBinary);
        assert_eq!(SpawnPath::resolve(true), SpawnPath::PortablePty);

        let temp = tempfile::tempdir()?;
        let session_root = temp.path().join("isolated-session");
        let browser_profile_dir = session_root.join(".codex-browser-profile");
        fs::create_dir_all(&browser_profile_dir)?;
        let expected_session_root = fs::canonicalize(&session_root)?;
        let expected_browser_profile_dir = fs::canonicalize(&browser_profile_dir)?;

        let mut session = PtySession::spawn(
            PtyCommand::new("/bin/sh")
                .arg("-lc")
                .arg("printf '%s|%s\\n' \"$PWD\" \"$CODEX_BROWSER_PROFILE_DIR\"")
                .cwd(&session_root)
                .env(
                    "CODEX_BROWSER_PROFILE_DIR",
                    expected_browser_profile_dir.as_os_str().to_owned(),
                ),
            SpawnOptions {
                spawn_path: SpawnPath::HelperBinary,
                ..SpawnOptions::default()
            },
        )?;
        let line = session.read_line()?;
        let status = session.finish()?;

        assert!(status.success());
        assert_eq!(session.spawn_path, SpawnPath::HelperBinary);
        assert!(session.runtime_ids.sid > 0);
        assert!(session.runtime_ids.pid > 0);
        assert!(session.runtime_ids.pgid > 0);
        assert_eq!(
            line.trim_end(),
            format!(
                "{}|{}",
                expected_session_root.display(),
                expected_browser_profile_dir.display()
            )
        );
        Ok(())
    }

    #[test]
    fn ut_pty_006_default_geometry_is_separate_from_browser_viewport() {
        let options = SpawnOptions::default();
        assert_eq!(options.geometry, PtyGeometry::new(120, 40));
        assert_eq!(
            options.browser_viewport,
            BrowserViewport {
                width: 1280,
                height: 720,
            }
        );
        assert_ne!(options.geometry.cols, options.browser_viewport.width);
    }

    #[test]
    fn ut_pty_007_resize_mutates_geometry_only_when_invoked() -> Result<()> {
        let mut session = PtySession::spawn(echo_command(), SpawnOptions::default())?;
        assert_eq!(session.geometry, PtyGeometry::new(120, 40));
        session.resize(140, 50)?;
        assert_eq!(session.geometry, PtyGeometry::new(140, 50));
        session.write_input(b"resize-check\n")?;
        let echoed = session.read_line()?;
        assert_eq!(echoed.trim_end(), "resize-check");
        let status = session.finish()?;
        assert!(status.success());
        Ok(())
    }

    #[test]
    fn it_pty_019_exec_failure_is_classified_without_runtime_ids() -> Result<()> {
        let result = PtySession::spawn(
            PtyCommand::new("definitely-not-a-real-command-for-pty-bead"),
            SpawnOptions::default(),
        );
        let Err(error) = result else {
            return Err(anyhow::anyhow!("spawn should fail"));
        };
        assert_eq!(error.classification(), "AGENT_RUN_FAILED");
        assert!(error.runtime_ids().is_none());
        Ok(())
    }

    #[test]
    fn it_pty_022_real_child_confirms_terminal_semantics() -> Result<()> {
        let mut session = PtySession::spawn(report_command()?, SpawnOptions::default())?;
        let line = session.read_line()?;
        let report: PtyReport = serde_json::from_str(line.trim_end())?;

        assert!(report.isatty_stdin);
        assert!(report.isatty_stdout);
        assert!(report.isatty_stderr);
        assert!(report.tty.starts_with("/dev/"));
        assert_eq!(report.pid, session.runtime_ids.pid);
        assert_eq!(report.pgid, session.runtime_ids.pgid);
        assert_eq!(report.sid, session.runtime_ids.sid);

        let parent_sid = getsid(None)?.as_raw().cast_unsigned();
        let parent_pgid = getpgid(None)?.as_raw().cast_unsigned();
        assert_ne!(report.sid, parent_sid);
        assert_ne!(report.pgid, parent_pgid);

        let status = session.finish()?;
        assert!(status.success());
        assert_eq!(session.phase_history.last(), Some(&SessionPhase::Archived));
        Ok(())
    }

    #[test]
    fn it_pty_008_output_is_sequenced_before_capture_consumption() -> Result<()> {
        let sandbox = ArtifactSandbox::new()?;
        let mut capture = PtyCapture::new(
            sandbox.root(),
            "ordered-capture",
            Utc::now(),
            CaptureConfig::default(),
        )?;

        capture.append(1, b"hello ", Utc::now())?;
        capture.append(2, b"world\n", Utc::now())?;
        let _ = capture.finalize()?;
        let index = sandbox.read_index("ordered-capture")?;
        ensure_contiguous(&index)?;
        let chunk_path = sandbox.chunk_paths("ordered-capture")?.remove(0);
        let payload = decode_chunk(&chunk_path)?;
        assert_eq!(payload, b"hello world\n");
        Ok(())
    }

    #[test]
    fn it_pty_009_chunk_boundary_rotates_at_threshold() -> Result<()> {
        let sandbox = ArtifactSandbox::new()?;
        let mut capture = PtyCapture::new(
            sandbox.root(),
            "boundary",
            Utc::now(),
            CaptureConfig {
                compressed_chunk_target_bytes: 32 * 1024,
            },
        )?;

        let first_payload = (32_680..32_880)
            .find_map(|candidate_len| {
                let mut state = 0x1234_5678_u32;
                let payload: Vec<u8> = (0..candidate_len)
                    .map(|_| {
                        state ^= state << 13;
                        state ^= state >> 17;
                        state ^= state << 5;
                        u8::try_from(state & 0xff).expect("low byte always fits in u8")
                    })
                    .collect();
                let compressed = zstd::stream::encode_all(payload.as_slice(), 0).ok()?;
                (compressed.len() == 32 * 1024).then_some(payload)
            })
            .ok_or_else(|| anyhow::anyhow!("failed to synthesize exact 32KB compressed payload"))?;
        capture.append(1, &first_payload, Utc::now())?;
        capture.append(2, b"overflow-byte", Utc::now())?;
        let index = capture.finalize()?;

        assert!(index.len() >= 2);
        ensure_contiguous(&index)?;
        Ok(())
    }

    #[test]
    fn it_pty_010_zero_byte_session_finalizes_without_partial_chunks() -> Result<()> {
        let sandbox = ArtifactSandbox::new()?;
        let mut capture = PtyCapture::new(
            sandbox.root(),
            "empty",
            Utc::now(),
            CaptureConfig::default(),
        )?;

        let index = capture.finalize()?;
        assert!(index.is_empty());
        assert!(sandbox.chunk_paths("empty")?.is_empty());
        Ok(())
    }

    #[test]
    fn it_pty_011_recovery_truncates_torn_index_line() -> Result<()> {
        let sandbox = ArtifactSandbox::new()?;
        let mut capture = PtyCapture::new(
            sandbox.root(),
            "recovery",
            Utc::now(),
            CaptureConfig::default(),
        )?;
        capture.append(1, b"capture\n", Utc::now())?;
        let paths = capture.paths().clone();
        let _ = capture.finalize()?;

        std::fs::OpenOptions::new()
            .append(true)
            .open(&paths.index_path)?
            .write_all(b"{\"chunk\":\"broken")?;

        let recovered = crate::artifacts_support::PtyCapture::recover(&paths)?;
        ensure_contiguous(&recovered)?;
        assert_eq!(recovered.len(), 1);
        Ok(())
    }

    #[test]
    fn it_pty_020_hash_matches_durable_chunk_payload() -> Result<()> {
        let sandbox = ArtifactSandbox::new()?;
        let mut capture = PtyCapture::new(
            sandbox.root(),
            "hashes",
            Utc::now(),
            CaptureConfig::default(),
        )?;
        capture.append(1, b"hash-me", Utc::now())?;
        let index = capture.finalize()?;
        let chunk_path = sandbox.chunk_paths("hashes")?.remove(0);
        let hash = crate::artifacts_support::compressed_payload_sha256(&chunk_path)?;
        assert_eq!(index[0].sha256, hash);
        Ok(())
    }

    #[test]
    fn it_pty_021_temp_chunk_never_survives_recovery() -> Result<()> {
        let sandbox = ArtifactSandbox::new()?;
        let paths = sandbox.session_paths("temp-cleanup")?;
        std::fs::write(paths.stream_dir.join("000001.pty.zst.tmp"), b"temp")?;
        let recovered = crate::artifacts_support::PtyCapture::recover(&paths)?;
        assert!(recovered.is_empty());
        assert!(!paths.stream_dir.join("000001.pty.zst.tmp").exists());
        Ok(())
    }

    #[test]
    fn e2e_pty_017_real_child_capture_creates_artifacts() -> Result<()> {
        let sandbox = ArtifactSandbox::new()?;
        let mut session = PtySession::spawn(report_command()?, SpawnOptions::default())?;
        let line = session.read_line()?;
        let mut capture = PtyCapture::new(
            sandbox.root(),
            "real-child",
            Utc::now(),
            CaptureConfig::default(),
        )?;
        capture.append(1, line.as_bytes(), Utc::now())?;
        let index = capture.finalize()?;
        let status = session.finish()?;

        assert!(status.success());
        assert_eq!(index.len(), 1);
        assert!(!sandbox.chunk_paths("real-child")?.is_empty());
        Ok(())
    }

    #[test]
    fn e2e_pty_018_transcript_is_reproducible_without_resize() -> Result<()> {
        let sandbox_a = ArtifactSandbox::new()?;
        let sandbox_b = ArtifactSandbox::new()?;

        let mut session_a = PtySession::spawn(report_command()?, SpawnOptions::default())?;
        let line_a = session_a.read_line()?;
        let report_a: PtyReport = serde_json::from_str(line_a.trim_end())?;
        let mut capture_a = PtyCapture::new(
            sandbox_a.root(),
            "repro-a",
            Utc::now(),
            CaptureConfig::default(),
        )?;
        capture_a.append(1, line_a.as_bytes(), Utc::now())?;
        let _ = capture_a.finalize()?;
        let _ = session_a.finish()?;

        let mut session_b = PtySession::spawn(report_command()?, SpawnOptions::default())?;
        let line_b = session_b.read_line()?;
        let report_b: PtyReport = serde_json::from_str(line_b.trim_end())?;
        let mut capture_b = PtyCapture::new(
            sandbox_b.root(),
            "repro-b",
            Utc::now(),
            CaptureConfig::default(),
        )?;
        capture_b.append(1, line_b.as_bytes(), Utc::now())?;
        let _ = capture_b.finalize()?;
        let _ = session_b.finish()?;

        assert_eq!(report_a.isatty_stdin, report_b.isatty_stdin);
        assert_eq!(report_a.isatty_stdout, report_b.isatty_stdout);
        assert_eq!(report_a.isatty_stderr, report_b.isatty_stderr);
        Ok(())
    }

    #[test]
    fn helper_report_session_semantics_mode_runs() -> Result<()> {
        let output = ProcessTreeHelper::output(&["report-session-semantics"])?;
        assert!(output.status.success());
        let report: PtyReport = serde_json::from_slice(&output.stdout)?;
        assert!(report.pid > 0);
        assert!(report.pgid > 0);
        assert!(report.sid > 0);
        Ok(())
    }

    #[test]
    fn helper_stdout_stderr_mode_runs() -> Result<()> {
        let output = ProcessTreeHelper::output(&["stdout-stderr"])?;
        assert!(output.status.success());
        assert_eq!(String::from_utf8(output.stdout)?.trim_end(), "stdout-frame");
        assert_eq!(String::from_utf8(output.stderr)?.trim_end(), "stderr-frame");
        Ok(())
    }

    #[test]
    fn helper_ignore_int_term_survives_term_until_killed() -> Result<()> {
        let mut guard = ProcessIsolationGuard::new("ignore-int-term");
        let mut child = ProcessTreeHelper::spawn(&["ignore-int-term", "5"])?;
        guard.register_pid(child.id());

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow::anyhow!("helper stdout was not piped"))?;
        let mut stdout = BufReader::new(stdout);
        let mut readiness = String::new();
        stdout.read_line(&mut readiness)?;
        assert_eq!(readiness.trim_end(), "ready");

        nix::sys::signal::kill(
            nix::unistd::Pid::from_raw(child.id().cast_signed()),
            nix::sys::signal::Signal::SIGTERM,
        )?;
        std::thread::sleep(Duration::from_millis(100));
        assert!(child.try_wait()?.is_none());

        guard.assert_no_survivors();
        let _ = ProcessTreeHelper::wait_for_exit(&mut child, Duration::from_secs(1));
        Ok(())
    }

    #[test]
    fn helper_process_group_lookup_runs() -> Result<()> {
        let mut child = ProcessTreeHelper::spawn(&["sleep", "1"])?;
        let process_group_id = ProcessTreeHelper::process_group(&child)?;
        assert!(process_group_id > 0);
        let _ = ProcessTreeHelper::wait_for_exit(&mut child, Duration::from_secs(2))?;
        Ok(())
    }

    #[test]
    fn it_pty_012_nominal_session_reaches_canonical_public_phases() -> Result<()> {
        let mut session = PtySession::spawn(report_command()?, SpawnOptions::default())?;
        let _ = session.read_line()?;
        let status = session.finish()?;
        assert!(status.success());
        assert_eq!(
            session
                .phase_history
                .iter()
                .map(|phase| phase.as_str())
                .collect::<Vec<_>>(),
            vec![
                "created",
                "spawned",
                "active",
                "terminating",
                "exited",
                "reaped",
                "archived"
            ]
        );
        Ok(())
    }

    #[test]
    fn it_pty_013_pre_spawn_failure_normalizes_without_runtime_ids() -> Result<()> {
        let result = PtySession::spawn(
            PtyCommand::new("definitely-not-a-real-command-for-pre-spawn-failure"),
            SpawnOptions::default(),
        );
        let Err(error) = result else {
            return Err(anyhow::anyhow!("spawn should fail"));
        };
        assert!(error.runtime_ids().is_none());
        assert_eq!(
            pre_spawn_failure_phases(),
            ["created", "terminating", "exited", "archived"]
        );
        Ok(())
    }

    #[test]
    fn it_pty_014_immediate_exit_still_records_active_before_finish() -> Result<()> {
        let mut session = PtySession::spawn(
            PtyCommand::new(ProcessTreeHelper::binary_path()?.display().to_string())
                .arg("exit-code")
                .arg("0"),
            SpawnOptions::default(),
        )?;
        let status = session.finish()?;
        assert!(status.success());
        assert!(session.phase_history.contains(&SessionPhase::Active));
        assert_eq!(session.phase_history[2], SessionPhase::Active);
        Ok(())
    }

    #[test]
    fn it_pty_015_no_output_timeout_classifies_stall() -> Result<()> {
        let mut child = ProcessTreeHelper::spawn(&["silent-stall", "2"])?;
        let mut tracker = StallTracker::new(Duration::from_millis(100), Duration::from_secs(5));
        tracker.record_progress(Duration::ZERO);
        std::thread::sleep(Duration::from_millis(150));
        assert_eq!(
            tracker.no_output_issue(Duration::from_millis(150)),
            Some(HealthIssue::AgentStalled)
        );
        let _ = nix::sys::signal::kill(
            nix::unistd::Pid::from_raw(child.id().cast_signed()),
            nix::sys::signal::Signal::SIGKILL,
        );
        let _ = ProcessTreeHelper::wait_for_exit(&mut child, Duration::from_secs(1));
        Ok(())
    }

    #[test]
    fn it_pty_016_auth_sleep_and_resource_signals_classify_expected_health_issues() {
        let mut tracker = StallTracker::new(Duration::from_secs(5), Duration::from_millis(100));
        tracker.record_output(Duration::ZERO);
        assert_eq!(
            tracker.no_progress_issue(Duration::from_millis(150)),
            Some(HealthIssue::AgentStalled)
        );
        assert_eq!(HealthIssue::AgentStalled.code(), "AGENT_STALLED");
        assert_eq!(
            detect_auth_challenge("AUTH CHALLENGE: re-login required"),
            Some(HealthIssue::AgentAuthChallenge)
        );
        assert_eq!(
            HealthIssue::AgentAuthChallenge.code(),
            "AGENT_AUTH_CHALLENGE"
        );
        assert_eq!(
            detect_host_sleep(Duration::from_secs(10), Duration::from_secs(5)),
            Some(HealthIssue::HostSleepInterrupted)
        );
        assert_eq!(
            HealthIssue::HostSleepInterrupted.code(),
            "HOST_SLEEP_INTERRUPTED"
        );
        assert_eq!(
            detect_resource_pressure(HostResources {
                cpu_pressure: true,
                memory_pressure: false,
            }),
            Some(HealthIssue::HostResourceExhausted)
        );
        assert_eq!(
            HealthIssue::HostResourceExhausted.code(),
            "HOST_RESOURCE_EXHAUSTED"
        );
    }

    fn report_command() -> Result<PtyCommand> {
        Ok(PtyCommand::new(ProcessTreeHelper::binary_path()?).arg("report-session-semantics"))
    }

    fn echo_command() -> PtyCommand {
        let script = r#"
import sys
import time

line = sys.stdin.readline().rstrip("\n")
print(line, flush=True)
time.sleep(0.2)
"#;
        PtyCommand::new("python3").arg("-c").arg(script)
    }
}
