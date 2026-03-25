#![forbid(unsafe_code)]

mod support;

#[path = "../src/pty/session.rs"]
mod session_impl;

#[cfg(unix)]
mod tests {
    use anyhow::Result;
    use nix::unistd::{getpgid, getsid};
    use serde::Deserialize;

    use crate::session_impl::{
        BrowserViewport, PtyCommand, PtyGeometry, PtyPairHandle, PtySession, SessionPhase,
        SpawnOptions, SpawnPath, spawn_policy_steps,
    };
    use crate::support;

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
        let mut session = PtySession::spawn(report_command(), SpawnOptions::default())?;
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
    fn ut_pty_005_helper_path_is_selected_when_pre_exec_safety_is_unavailable() {
        assert_eq!(SpawnPath::resolve(false), SpawnPath::HelperBinary);
        assert_eq!(SpawnPath::resolve(true), SpawnPath::PortablePty);
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
    fn it_pty_019_exec_failure_is_classified_without_runtime_ids() {
        let result = PtySession::spawn(
            PtyCommand::new("definitely-not-a-real-command-for-pty-bead"),
            SpawnOptions::default(),
        );
        let Err(error) = result else {
            panic!("spawn should fail");
        };
        assert_eq!(error.classification(), "AGENT_RUN_FAILED");
        assert!(error.runtime_ids().is_none());
    }

    #[test]
    fn it_pty_022_real_child_confirms_terminal_semantics() -> Result<()> {
        let mut session = PtySession::spawn(report_command(), SpawnOptions::default())?;
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

    fn report_command() -> PtyCommand {
        let script = r#"
import json
import os
import sys
import time

print(json.dumps({
  "isatty_stdin": os.isatty(0),
  "isatty_stdout": os.isatty(1),
  "isatty_stderr": os.isatty(2),
  "tty": os.ttyname(0),
  "pid": os.getpid(),
  "pgid": os.getpgid(0),
  "sid": os.getsid(0),
}), flush=True)
time.sleep(0.2)
"#;
        PtyCommand::new("python3").arg("-c").arg(script)
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
