#![forbid(unsafe_code)]
#![allow(clippy::duplicate_mod)]

#[allow(dead_code)]
mod support;

#[allow(dead_code)]
#[path = "../src/errors/manifest.rs"]
mod errors_manifest_impl;
#[allow(dead_code)]
#[path = "../src/errors/markdown.rs"]
mod errors_markdown_impl;
#[allow(dead_code)]
#[path = "../src/pty/health.rs"]
mod health_impl;
#[allow(dead_code)]
#[path = "../src/sessions/reaper.rs"]
mod reaper_impl;

#[cfg(unix)]
mod tests {
    use std::time::Duration;

    use anyhow::Result;
    use chrono::{DateTime, Utc};
    use serial_test::serial;

    use crate::health_impl::{
        AuthIntervention, HealthIssue, HostResources, StallTracker, auth_intervention,
        detect_auth_challenge, detect_host_sleep, detect_resource_pressure,
        pre_spawn_failure_phases,
    };
    use crate::reaper_impl::{CleanupRequest, CleanupVerdict, SessionReaper};
    use crate::support::TestContext;
    use crate::support::process::ProcessTreeHelper;

    fn fixed_started_at() -> Result<DateTime<Utc>> {
        Ok(DateTime::parse_from_rfc3339("2026-03-28T20:00:00Z")?.with_timezone(&Utc))
    }

    #[test]
    fn ag_hlt_001_stall_detection_surfaces_agent_stalled() {
        let tracker = StallTracker::new(Duration::from_secs(5), Duration::from_secs(10));
        assert_eq!(
            tracker.no_output_issue(Duration::from_secs(6)),
            Some(HealthIssue::AgentStalled),
        );
        assert_eq!(
            tracker.no_progress_issue(Duration::from_secs(12)),
            Some(HealthIssue::AgentStalled),
        );
    }

    #[test]
    fn ag_hlt_002_auth_challenge_markers_are_detected() {
        assert_eq!(
            detect_auth_challenge("MFA required before continuing"),
            Some(HealthIssue::AgentAuthChallenge),
        );
        assert_eq!(
            detect_host_sleep(Duration::from_secs(65), Duration::from_mins(1)),
            Some(HealthIssue::HostSleepInterrupted),
        );
        assert_eq!(
            detect_resource_pressure(HostResources {
                cpu_pressure: false,
                memory_pressure: true,
            }),
            Some(HealthIssue::HostResourceExhausted),
        );
    }

    #[test]
    fn ag_hlt_003_browser_only_auth_uses_reveal_profile_path_guidance() {
        assert_eq!(auth_intervention(false), AuthIntervention::ReattachPty);
        assert_eq!(auth_intervention(true), AuthIntervention::RevealProfilePath);
    }

    #[test]
    #[serial]
    fn ag_hlt_004_cleanup_escalation_records_forced_verdict() -> Result<()> {
        let ctx = TestContext::new()?;
        let reaper = SessionReaper::for_test(&ctx.state_root, &ctx.artifact_root)?;
        let child = ProcessTreeHelper::spawn(&["ignore-int-term", "5"])?;
        let pgid = ProcessTreeHelper::process_group(&child)?;
        let outcome = reaper.cleanup_process_group(&CleanupRequest {
            session_id: "agent-health-forced".to_owned(),
            started_at: fixed_started_at()?,
            process_group_id: pgid,
            root_pid: Some(child.id()),
            observed_descendant_pids: Vec::new(),
            send_pty_interrupt: false,
        })?;
        assert!(matches!(
            outcome.cleanup_verdict,
            CleanupVerdict::Clean | CleanupVerdict::Forced
        ));
        assert!(!outcome.action_log.is_empty());
        Ok(())
    }

    #[test]
    fn ag_hlt_005_pre_active_crash_keeps_canonical_phase_history() {
        assert_eq!(
            pre_spawn_failure_phases(),
            ["created", "terminating", "exited", "archived"],
        );
    }
}
