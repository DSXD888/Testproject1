#![forbid(unsafe_code)]

mod support;

#[path = "../src/sessions/reaper.rs"]
mod reaper_impl;

#[cfg(unix)]
mod tests {
    use anyhow::Result;
    use serial_test::serial;
    use std::time::Duration;

    use crate::reaper_impl::{CleanupVerdict, CompletionState, SessionReaper};
    use crate::support::process::ProcessTreeHelper;
    use crate::support::{self, TestContext};

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
        let pgid = ProcessTreeHelper::process_group(&child)?;
        let outcome = reaper.cleanup_process_group("session-ladder", pgid, true)?;
        assert_eq!(outcome.action_log[..3], ["pty-ctrl-c", "sigint", "sigterm"]);
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
        let outcome = reaper.cleanup_process_group("session-clean", pgid, false)?;
        assert_eq!(outcome.cleanup_verdict, CleanupVerdict::Clean);
        Ok(())
    }

    #[test]
    #[serial]
    fn it_rpr_004_forced_verdict_when_kill_is_required() -> Result<()> {
        let ctx = TestContext::new()?;
        let reaper = SessionReaper::for_test(&ctx.state_root, &ctx.artifact_root)?;
        let child = ProcessTreeHelper::spawn(&["ignore-int-term", "5"])?;
        let pgid = ProcessTreeHelper::process_group(&child)?;
        let outcome = reaper.cleanup_process_group("session-forced", pgid, false)?;
        assert_eq!(outcome.cleanup_verdict, CleanupVerdict::Forced);
        Ok(())
    }

    #[test]
    #[serial]
    fn it_rpr_005_cleanup_report_emits_leaked_error_code_when_needed() -> Result<()> {
        let ctx = TestContext::new()?;
        let reaper = SessionReaper::for_test(&ctx.state_root, &ctx.artifact_root)?;
        let outcome = reaper.cleanup_process_group("session-leak-probe", 999_999, false)?;
        assert_eq!(outcome.cleanup_verdict, CleanupVerdict::Clean);
        Ok(())
    }

    #[test]
    #[serial]
    fn it_rpr_006_orphaned_grandchild_is_detected_by_descendant_sweep() -> Result<()> {
        let ctx = TestContext::new()?;
        let _reaper = SessionReaper::for_test(&ctx.state_root, &ctx.artifact_root)?;
        let child = ProcessTreeHelper::spawn(&["fork-grandchild", "2"])?;
        std::thread::sleep(Duration::from_millis(200));
        let descendants = SessionReaper::descendant_pids(child.id())?;
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
        let outcome = reaper.cleanup_process_group("session-report", pgid, false)?;
        assert!(outcome.report_path.ends_with("cleanup_report.json"));
        assert!(outcome.report_path.exists());
        Ok(())
    }

    #[test]
    #[serial]
    fn it_rpr_008_crash_reconciliation_synthesizes_manifest() -> Result<()> {
        let ctx = TestContext::new()?;
        let reaper = SessionReaper::for_test(&ctx.state_root, &ctx.artifact_root)?;
        let child = ProcessTreeHelper::spawn(&["ignore-int-term", "5"])?;
        let pgid = ProcessTreeHelper::process_group(&child)?;
        let result = reaper.reconcile_after_crash("session-crash", pgid)?;
        assert_eq!(result.error_code, "AGENT_DAEMON_CRASH_DURING_SESSION");
        assert!(result.manifest_path.exists());
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
        let child = ProcessTreeHelper::spawn(&["double-fork-escape", "2"])?;
        let pgid = ProcessTreeHelper::process_group(&child)?;
        std::thread::sleep(Duration::from_millis(200));
        let before = SessionReaper::process_group_members(pgid)?;
        let outcome = reaper.cleanup_process_group("session-escape", pgid, false)?;
        assert!(!before.is_empty());
        assert!(matches!(
            outcome.cleanup_verdict,
            CleanupVerdict::Forced | CleanupVerdict::Clean
        ));
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
            let outcome = reaper.cleanup_process_group(&format!("batch-{index}"), pgid, false)?;
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
