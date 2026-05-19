#![forbid(unsafe_code)]
#![allow(clippy::duplicate_mod)]

mod support;

#[allow(dead_code)]
#[path = "../src/errors/manifest.rs"]
mod errors_manifest_impl;
#[allow(dead_code)]
#[path = "../src/errors/markdown.rs"]
mod errors_markdown_impl;
#[path = "../src/sessions/reaper.rs"]
mod reaper_impl;

#[cfg(unix)]
mod tests {
    use anyhow::Result;
    use chrono::{DateTime, Utc};
    use serial_test::serial;
    use std::path::Path;
    use std::time::Duration;

    use crate::errors_manifest_impl::ErrorRuntimeContext;
    use crate::reaper_impl::{
        CleanupRequest, CleanupVerdict, CompletionState, CrashRecoveryContext, SessionReaper,
    };
    use crate::support::process::{ProcessIsolationGuard, ProcessTreeHelper};
    use crate::support::{self, TestContext};

    fn fixed_started_at() -> Result<DateTime<Utc>> {
        Ok(DateTime::parse_from_rfc3339("2026-03-25T11:00:00Z")?.with_timezone(&Utc))
    }

    fn wait_for_signal_ignoring_helper() {
        std::thread::sleep(Duration::from_millis(750));
    }

    fn wait_for_descendants(root_pid: u32) -> Result<Vec<u32>> {
        for _ in 0..40 {
            let descendants = SessionReaper::descendant_pids(root_pid)?;
            if !descendants.is_empty() {
                return Ok(descendants);
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        SessionReaper::descendant_pids(root_pid)
    }

    #[test]
    #[serial]
    fn it_rpr_001_waitpid_drain_associates_pid_with_session() -> Result<()> {
        let ctx = TestContext::new()?;
        assert!(ctx.temp.path().exists());
        assert!(ctx.runtime_root.ends_with("runtime"));
        assert!(ctx.db_path.ends_with("state/state.db"));
        let state_db = support::db::TestStateDb::new()?;
        assert!(state_db.db_path().exists());
        assert_eq!(
            state_db.connection().query_row(
                "SELECT commit_seq FROM commit_clock WHERE id = 1",
                [],
                |row| row.get::<_, i64>(0)
            )?,
            0
        );
        let mut reaper = SessionReaper::for_test(&ctx.state_root, &ctx.artifact_root)?;
        let child = ProcessTreeHelper::spawn(&["sleep", "5"])?;
        reaper.register_pid("session-1", child.id());
        nix::sys::signal::kill(
            nix::unistd::Pid::from_raw(child.id().cast_signed()),
            nix::sys::signal::Signal::SIGKILL,
        )?;
        let started = std::time::Instant::now();
        let mut all_events = Vec::new();
        loop {
            let events = reaper.drain_waitpid()?;
            all_events.extend(events);
            if all_events
                .iter()
                .any(|event| event.session_id.as_deref() == Some("session-1"))
            {
                break;
            }
            assert!(
                started.elapsed() < Duration::from_secs(1),
                "timed out waiting for reap event"
            );
            std::thread::sleep(Duration::from_millis(25));
        }
        assert!(
            all_events
                .iter()
                .any(|event| event.session_id.as_deref() == Some("session-1"))
        );
        Ok(())
    }

    #[test]
    #[serial]
    fn it_rpr_002_cleanup_ladder_records_signal_order() -> Result<()> {
        let ctx = TestContext::new()?;
        let reaper = SessionReaper::for_test(&ctx.state_root, &ctx.artifact_root)?;
        let child = ProcessTreeHelper::spawn(&["ignore-int-term", "5"])?;
        wait_for_signal_ignoring_helper();
        let pgid = ProcessTreeHelper::process_group(&child)?;
        let outcome = reaper.cleanup_process_group(&CleanupRequest {
            session_id: "session-ladder".to_owned(),
            started_at: fixed_started_at()?,
            process_group_id: pgid,
            root_pid: Some(child.id()),
            observed_descendant_pids: Vec::new(),
            send_pty_interrupt: true,
        })?;
        assert!(
            outcome.action_log.len() >= 2,
            "expected at least pty interrupt + sigint actions, got {:?}",
            outcome.action_log
        );
        assert_eq!(outcome.action_log[0], "pty-ctrl-c");
        assert_eq!(outcome.action_log[1], "sigint");
        let expected_ladder = ["pty-ctrl-c", "sigint", "sigterm", "sigkill"];
        for (index, action) in outcome.action_log.iter().enumerate() {
            assert_eq!(
                action,
                expected_ladder.get(index).unwrap_or(&""),
                "unexpected cleanup action order: {:?}",
                outcome.action_log
            );
        }
        Ok(())
    }

    #[test]
    #[serial]
    fn it_rpr_003_clean_verdict_when_descendants_exit_before_deadline() -> Result<()> {
        let ctx = TestContext::new()?;
        let reaper = SessionReaper::for_test(&ctx.state_root, &ctx.artifact_root)?;
        let mut child = ProcessTreeHelper::spawn(&["sleep", "0"])?;
        let pgid = ProcessTreeHelper::process_group(&child)?;
        let _ = ProcessTreeHelper::wait_for_exit(&mut child, Duration::from_secs(1))?;
        let outcome = reaper.cleanup_process_group(&CleanupRequest {
            session_id: "session-clean".to_owned(),
            started_at: fixed_started_at()?,
            process_group_id: pgid,
            root_pid: Some(child.id()),
            observed_descendant_pids: Vec::new(),
            send_pty_interrupt: false,
        })?;
        assert_eq!(outcome.cleanup_verdict, CleanupVerdict::Clean);
        Ok(())
    }

    #[test]
    #[serial]
    fn it_rpr_004_forced_verdict_when_kill_is_required() -> Result<()> {
        let ctx = TestContext::new()?;
        let reaper = SessionReaper::for_test(&ctx.state_root, &ctx.artifact_root)?;
        let child = ProcessTreeHelper::spawn(&["ignore-int-term", "5"])?;
        wait_for_signal_ignoring_helper();
        let pgid = ProcessTreeHelper::process_group(&child)?;
        let outcome = reaper.cleanup_process_group(&CleanupRequest {
            session_id: "session-forced".to_owned(),
            started_at: fixed_started_at()?,
            process_group_id: pgid,
            root_pid: Some(child.id()),
            observed_descendant_pids: Vec::new(),
            send_pty_interrupt: false,
        })?;
        assert_eq!(outcome.cleanup_verdict, CleanupVerdict::Forced);
        Ok(())
    }

    #[test]
    #[serial]
    fn it_rpr_005_cleanup_report_emits_leaked_error_code_when_needed() -> Result<()> {
        let ctx = TestContext::new()?;
        let reaper = SessionReaper::for_test(&ctx.state_root, &ctx.artifact_root)?;
        let mut guard = ProcessIsolationGuard::new("it_rpr_005");
        let child = ProcessTreeHelper::spawn(&["sleep", "5"])?;
        let survivor = ProcessTreeHelper::spawn(&["sleep", "5"])?;
        let pgid = ProcessTreeHelper::process_group(&child)?;
        guard.register_pgid(pgid);
        guard.register_pid(survivor.id());
        let outcome = reaper.cleanup_process_group(&CleanupRequest {
            session_id: "session-leak-probe".to_owned(),
            started_at: fixed_started_at()?,
            process_group_id: pgid,
            root_pid: Some(child.id()),
            observed_descendant_pids: vec![survivor.id()],
            send_pty_interrupt: false,
        })?;
        for pid in &outcome.leaked_pids {
            guard.register_pid(*pid);
        }
        assert_eq!(outcome.cleanup_verdict, CleanupVerdict::Leaked);
        assert_eq!(
            outcome.error_code.as_deref(),
            Some("AGENT_CLEANUP_INCOMPLETE")
        );
        assert!(!outcome.leaked_pids.is_empty());
        Ok(())
    }

    #[test]
    #[serial]
    fn it_rpr_006_orphaned_grandchild_is_detected_by_descendant_sweep() -> Result<()> {
        let ctx = TestContext::new()?;
        let _reaper = SessionReaper::for_test(&ctx.state_root, &ctx.artifact_root)?;
        let child = ProcessTreeHelper::spawn(&["fork-grandchild", "2"])?;
        let descendants = wait_for_descendants(child.id())?;
        assert!(!descendants.is_empty());
        Ok(())
    }

    #[test]
    #[serial]
    fn it_rpr_007_cleanup_report_is_emitted() -> Result<()> {
        let ctx = TestContext::new()?;
        let reaper = SessionReaper::for_test(&ctx.state_root, &ctx.artifact_root)?;
        let mut child = ProcessTreeHelper::spawn(&["sleep", "0"])?;
        let pgid = ProcessTreeHelper::process_group(&child)?;
        let _ = ProcessTreeHelper::wait_for_exit(&mut child, Duration::from_secs(1))?;
        let outcome = reaper.cleanup_process_group(&CleanupRequest {
            session_id: "session-report".to_owned(),
            started_at: fixed_started_at()?,
            process_group_id: pgid,
            root_pid: Some(child.id()),
            observed_descendant_pids: Vec::new(),
            send_pty_interrupt: false,
        })?;
        assert!(
            outcome
                .report_path
                .ends_with("sessions/2026/03/session-report/cleanup_report.json")
        );
        assert!(outcome.report_path.exists());
        Ok(())
    }

    #[test]
    #[serial]
    fn it_rpr_008_crash_reconciliation_synthesizes_manifest() -> Result<()> {
        let ctx = TestContext::new()?;
        let state_db = support::db::TestStateDb::new()?;
        let reaper = SessionReaper::for_test(&ctx.state_root, &ctx.artifact_root)?;
        let child = ProcessTreeHelper::spawn(&["ignore-int-term", "5"])?;
        wait_for_signal_ignoring_helper();
        let pgid = ProcessTreeHelper::process_group(&child)?;
        let started_at = fixed_started_at()?;
        state_db.connection().execute(
            "\
            INSERT INTO agent_session_logs (
              id, job_id, skill_path, skill_version, execution_mode, cli_vendor, cli_version,
              adapter_version, config_hash, pty_session_id, pid, pgid, started_at, completed_at,
              status, terminal_output_path, terminal_output_hash, terminal_output_bytes,
              terminal_output_tail, structured_trace, error_type, error_message, cleanup_verdict,
              last_checkpoint_seq
            ) VALUES (?1, NULL, ?2, ?3, ?4, NULL, NULL, NULL, NULL, NULL, ?5, ?6, ?7, NULL, ?8,
                      NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, ?9)
            ",
            rusqlite::params![
                "session-crash",
                "skills/procurement-ingestion/fts/SKILL.md",
                "1.0.0",
                "pty",
                child.id(),
                pgid,
                started_at.to_rfc3339(),
                "active",
                3_u32,
            ],
        )?;
        let result = reaper.reconcile_after_crash(
            "session-crash",
            pgid,
            Some(child.id()),
            &CrashRecoveryContext {
                conn: state_db.connection(),
                started_at,
                job_id: None,
                process_id: None,
                skill_path: Some(Path::new("skills/procurement-ingestion/fts/SKILL.md")),
                runtime_context: ErrorRuntimeContext {
                    cli_vendor: Some("fixture-cli".to_owned()),
                    cli_version: Some("1.0.0".to_owned()),
                    adapter_version: Some("daemon-test".to_owned()),
                    model_id: None,
                    config_hash: None,
                    skill_version: Some("1.0.0".to_owned()),
                    pid: Some(child.id()),
                    pgid: Some(pgid.unsigned_abs()),
                    cleanup_verdict: None,
                    last_valid_checkpoint_sequence: Some(3),
                    daemon_instance_id: Some("daemon-test-instance".to_owned()),
                    commit_seq: Some(0),
                },
            },
        )?;
        assert_eq!(result.error_code, "AGENT_DAEMON_CRASH_DURING_SESSION");
        assert!(result.manifest_path.exists());
        // The renderer derives the year/month bucket from the current UTC clock,
        // so build the expected fragment dynamically rather than hardcoding a
        // month — otherwise this test fails the moment the calendar rolls over.
        let now = Utc::now();
        let expected_bucket = now.format("/error-manifests/%Y/%m/").to_string();
        let actual = result.manifest_path.to_string_lossy().into_owned();
        assert!(
            actual.contains(&expected_bucket),
            "manifest path {actual} should contain {expected_bucket}",
        );
        let error_type: String = state_db.connection().query_row(
            "SELECT error_type FROM error_manifests ORDER BY created_at DESC LIMIT 1",
            [],
            |row| row.get(0),
        )?;
        assert_eq!(error_type, "AGENT_DAEMON_CRASH_DURING_SESSION");
        Ok(())
    }

    #[test]
    #[serial]
    fn it_rpr_009_completion_gate_waits_for_drain_before_reaped() {
        assert!(!SessionReaper::completion_gate(CompletionState {
            child_exit_observed: true,
            pty_drained: false,
            descendants_clean: true,
            result_validated: true,
            verdict_recorded: true,
        }));
        assert!(SessionReaper::completion_gate(CompletionState {
            child_exit_observed: true,
            pty_drained: true,
            descendants_clean: true,
            result_validated: true,
            verdict_recorded: true,
        }));
    }

    #[test]
    #[serial]
    fn it_rpr_010_browser_escape_case_records_non_group_descendants() -> Result<()> {
        let ctx = TestContext::new()?;
        let reaper = SessionReaper::for_test(&ctx.state_root, &ctx.artifact_root)?;
        let mut guard = ProcessIsolationGuard::new("it_rpr_010");
        let child = ProcessTreeHelper::spawn(&["sleep", "2"])?;
        let survivor = ProcessTreeHelper::spawn(&["sleep", "2"])?;
        let pgid = ProcessTreeHelper::process_group(&child)?;
        guard.register_pgid(pgid);
        guard.register_pid(survivor.id());
        let before = SessionReaper::process_group_members(pgid)?;
        let outcome = reaper.cleanup_process_group(&CleanupRequest {
            session_id: "session-escape".to_owned(),
            started_at: fixed_started_at()?,
            process_group_id: pgid,
            root_pid: Some(child.id()),
            observed_descendant_pids: vec![survivor.id()],
            send_pty_interrupt: false,
        })?;
        for pid in &outcome.leaked_pids {
            guard.register_pid(*pid);
        }
        assert!(!before.is_empty());
        assert_eq!(outcome.cleanup_verdict, CleanupVerdict::Leaked);
        assert!(
            outcome
                .leaked_pids
                .iter()
                .any(|pid| !before.iter().any(|member| member == pid))
        );
        Ok(())
    }

    #[test]
    #[serial]
    fn it_rpr_011_health_related_errors_are_visible_through_reaper_completion_rules() -> Result<()>
    {
        let _ctx = TestContext::new()?;
        assert!(!SessionReaper::completion_gate(CompletionState {
            child_exit_observed: false,
            pty_drained: true,
            descendants_clean: true,
            result_validated: true,
            verdict_recorded: true,
        }));
        Ok(())
    }

    #[test]
    #[serial]
    fn e2e_rpr_012_reliability_batch_leaves_no_known_orphans() -> Result<()> {
        let ctx = TestContext::new()?;
        let reaper = SessionReaper::for_test(&ctx.state_root, &ctx.artifact_root)?;
        for index in 0..10 {
            let mut child = ProcessTreeHelper::spawn(&["sleep", "0"])?;
            let pgid = ProcessTreeHelper::process_group(&child)?;
            let _ = ProcessTreeHelper::wait_for_exit(&mut child, Duration::from_secs(1))?;
            let outcome = reaper.cleanup_process_group(&CleanupRequest {
                session_id: format!("batch-{index}"),
                started_at: fixed_started_at()?,
                process_group_id: pgid,
                root_pid: Some(child.id()),
                observed_descendant_pids: Vec::new(),
                send_pty_interrupt: false,
            })?;
            assert_eq!(outcome.cleanup_verdict, CleanupVerdict::Clean);
            assert!(outcome.leaked_pids.is_empty());
        }
        Ok(())
    }

    #[test]
    #[serial]
    fn it_rpr_013_unknown_pid_reap_is_safe() -> Result<()> {
        let ctx = TestContext::new()?;
        let mut reaper = SessionReaper::for_test(&ctx.state_root, &ctx.artifact_root)?;
        let mut child = ProcessTreeHelper::spawn(&["exit-code", "0"])?;
        let _ = ProcessTreeHelper::wait_for_exit(&mut child, Duration::from_secs(1))?;
        let events = reaper.drain_waitpid()?;
        assert!(events.iter().all(|event| event.session_id.is_none()));
        Ok(())
    }
}
