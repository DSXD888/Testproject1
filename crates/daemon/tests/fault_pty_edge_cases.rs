#![forbid(unsafe_code)]
#![allow(clippy::duplicate_mod)]
#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(clippy::all)]
#![allow(clippy::pedantic, clippy::nursery)]

#[allow(dead_code, unused_imports)]
mod support;

#[allow(dead_code, unused_imports)]
#[path = "support/artifacts.rs"]
mod artifacts_support;
#[allow(dead_code)]
#[path = "../src/pty/health.rs"]
mod health_impl;
#[allow(dead_code)]
#[path = "../src/pty/session.rs"]
mod session_impl;

#[cfg(unix)]
mod tests {
    use anyhow::Result;
    use serde::Deserialize;

    use crate::session_impl::{
        BrowserViewport, PtyCommand, PtyGeometry, PtyPairHandle, PtySession, SpawnOptions,
        SpawnPath, spawn_policy_steps,
    };

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
    fn l3_t_fault_pty_edge_cases_001_pair_is_created_before_spawn() -> Result<()> {
        let pair = PtyPairHandle::open(PtyGeometry::default())?;
        assert_eq!(pair.geometry(), PtyGeometry::new(120, 40));
        assert!(pair.master_fd().is_some());
        Ok(())
    }

    #[test]
    fn l3_t_fault_pty_edge_cases_002_spawn_policy_orders_pre_exec_steps() {
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
    fn l3_t_fault_pty_edge_cases_003_runtime_ids_exist_before_spawned_session_is_returned()
    -> Result<()> {
        let mut session = PtySession::spawn(report_command()?, SpawnOptions::default())?;
        assert!(session.runtime_ids.sid > 0);
        assert!(session.runtime_ids.pid > 0);
        assert!(session.runtime_ids.pgid > 0);
        let _ = session.read_line()?;
        let status = session.finish()?;
        assert!(status.success());
        Ok(())
    }

    #[test]
    fn l3_t_fault_pty_edge_cases_004_master_fd_is_marked_cloexec() -> Result<()> {
        let pair = PtyPairHandle::open(PtyGeometry::default())?;
        assert!(pair.master_has_cloexec()?);
        Ok(())
    }

    #[test]
    fn l3_t_fault_pty_edge_cases_005_helper_path_is_selected_when_pre_exec_safety_is_unavailable()
    -> Result<()> {
        let mut session = PtySession::spawn(
            PtyCommand::new("/bin/sh")
                .arg("-lc")
                .arg("printf 'helper-path\\n'"),
            SpawnOptions {
                spawn_path: SpawnPath::HelperBinary,
                ..SpawnOptions::default()
            },
        )?;
        let line = session.read_line()?;
        let status = session.finish()?;
        assert!(status.success());
        assert_eq!(line.trim_end(), "helper-path");
        assert_eq!(session.spawn_path, SpawnPath::HelperBinary);
        Ok(())
    }

    #[test]
    fn l3_t_fault_pty_edge_cases_006_default_geometry_is_separate_from_browser_viewport() {
        let options = SpawnOptions::default();
        assert_eq!(options.geometry, PtyGeometry::new(120, 40));
        assert_eq!(
            options.browser_viewport,
            BrowserViewport {
                width: 1280,
                height: 720,
            }
        );
    }

    #[test]
    fn l3_t_fault_pty_edge_cases_007_resize_mutates_geometry_only_when_invoked() -> Result<()> {
        let mut session = PtySession::spawn(report_command()?, SpawnOptions::default())?;
        assert_eq!(session.geometry, PtyGeometry::new(120, 40));
        session.resize(140, 50)?;
        assert_eq!(session.geometry, PtyGeometry::new(140, 50));
        let _ = session.read_line()?;
        let status = session.finish()?;
        assert!(status.success());
        Ok(())
    }

    #[test]
    fn l3_t_fault_pty_edge_cases_008_exec_failure_is_classified_without_runtime_ids() -> Result<()>
    {
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

    fn report_command() -> Result<PtyCommand> {
        Ok(
            PtyCommand::new(crate::support::process::ProcessTreeHelper::binary_path()?)
                .arg("report-session-semantics"),
        )
    }
}
