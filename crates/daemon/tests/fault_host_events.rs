#![forbid(unsafe_code)]

#[path = "../src/agents/arbiter.rs"]
#[allow(dead_code)]
mod arbiter_impl;
#[path = "../src/agents/contracts.rs"]
#[allow(dead_code)]
mod contracts_impl;
#[path = "../src/scheduler/cron.rs"]
#[allow(dead_code)]
mod cron_impl;
#[path = "support/db.rs"]
mod db_support;
#[path = "../src/pty/health.rs"]
#[allow(dead_code)]
mod health_impl;
#[path = "../src/sources/registry.rs"]
#[allow(dead_code)]
mod registry_impl;
#[path = "../src/jobs/runner.rs"]
#[allow(dead_code)]
mod runner_impl;

mod agents {
    pub mod arbiter {
        pub use crate::arbiter_impl::*;
    }
}

mod jobs {
    pub mod runner {
        pub use crate::runner_impl::*;
    }
}

mod sources {
    pub mod registry {
        pub use crate::registry_impl::*;
    }
}

mod support {
    pub mod db {
        pub use crate::db_support::*;
    }
}

mod tests {
    use anyhow::{Result, ensure};
    use chrono::{TimeZone, Utc};
    use rusqlite::params;

    use crate::arbiter_impl::{
        AdmissionPriority, AdmissionRequest, ArbiterDecision, DenialReason, ResourceArbiter,
        ResourceBudgets, SourceKind, SourceLaneRequest,
    };
    use crate::cron_impl::CronScheduler;
    use crate::db_support::StateDb;
    use crate::health_impl::{
        HealthIssue, HostResources, StallTracker, detect_auth_challenge, detect_host_sleep,
        detect_resource_pressure,
    };
    use crate::registry_impl::SourceRegistry;
    use crate::runner_impl::evaluate_host_health;
    use std::time::Duration;

    fn open_state_db() -> Result<(tempfile::TempDir, StateDb)> {
        let temp = tempfile::tempdir()?;
        let db_path = temp.path().join("state.db");
        let state_db = StateDb::open(&db_path)?;
        Ok((temp, state_db))
    }

    fn seed_sources(conn: &rusqlite::Connection) -> Result<()> {
        for (id, kind) in [
            ("fts", "procurement"),
            ("alibaba", "supplier"),
            ("1688", "supplier"),
        ] {
            conn.execute(
                "\
                INSERT INTO sources (id, kind, enabled, config)
                VALUES (?1, ?2, 1, '{}')
                ON CONFLICT(id) DO UPDATE SET
                  kind = excluded.kind,
                  enabled = excluded.enabled,
                  config = excluded.config
                ",
                params![id, kind],
            )?;
        }
        Ok(())
    }

    fn mark_source_runtime(
        conn: &rusqlite::Connection,
        source_id: &str,
        circuit_state: &str,
        cooldown_until: Option<&str>,
        last_canary_result: Option<&str>,
    ) -> Result<()> {
        conn.execute(
            "\
            UPDATE source_runtime_state
            SET circuit_state = ?2,
                cooldown_until = ?3,
                last_canary_result = ?4,
                updated_at = CURRENT_TIMESTAMP
            WHERE source_id = ?1
            ",
            params![source_id, circuit_state, cooldown_until, last_canary_result],
        )?;
        Ok(())
    }

    #[test]
    fn l3_t_fault_host_events_001_monotonic_suspend_gap_taints_browser_pty_sessions() {
        let observation = evaluate_host_health(
            Duration::from_secs(10),
            Duration::from_secs(5),
            false,
            false,
            true,
        );

        assert_eq!(observation.issue.as_deref(), Some("HOST_SLEEP_INTERRUPTED"));
        assert!(observation.tainted_active_browser_session);
        assert!(!observation.blocks_new_admission);
    }

    #[test]
    fn l3_t_fault_host_events_002_wall_clock_change_without_monotonic_gap_does_not_emit_sleep() {
        let observation =
            evaluate_host_health(Duration::ZERO, Duration::from_secs(5), false, false, true);

        assert_eq!(observation.issue, None);
        assert_eq!(
            detect_host_sleep(Duration::ZERO, Duration::from_secs(5)),
            None
        );
    }

    #[test]
    fn l3_t_fault_host_events_003_resource_pressure_blocks_new_work_but_existing_work_remains()
    -> Result<()> {
        let mut arbiter = ResourceArbiter::new(ResourceBudgets::default());
        let running = AdmissionRequest {
            job_id: "job-running".to_owned(),
            session_id: Some("session-running".to_owned()),
            priority: AdmissionPriority::ProcurementIngestion,
            browser_session: true,
            adapter_session: true,
            source_lane: Some(SourceLaneRequest {
                source_id: "fts".to_owned(),
                source_kind: SourceKind::Procurement,
            }),
            validator_sub_job: false,
            concurrency_key: None,
            max_concurrency: None,
        };
        let existing_lease = match arbiter.admit(&running) {
            ArbiterDecision::Granted(lease) => lease,
            ArbiterDecision::Denied(reason) => anyhow::bail!("unexpected denial: {reason:?}"),
        };

        let observation =
            evaluate_host_health(Duration::ZERO, Duration::from_secs(5), true, false, false);
        assert_eq!(
            observation.issue.as_deref(),
            Some("HOST_RESOURCE_EXHAUSTED")
        );
        assert!(observation.blocks_new_admission);
        assert_eq!(arbiter.snapshot().browser_slots, 1);
        assert_eq!(existing_lease.browser_slots, 1);
        Ok(())
    }

    #[test]
    fn l3_t_fault_host_events_004_source_lane_denials_remain_source_specific_under_pressure()
    -> Result<()> {
        let mut arbiter = ResourceArbiter::new(ResourceBudgets::default());
        let first = AdmissionRequest {
            job_id: "job-fts-1".to_owned(),
            session_id: Some("session-fts-1".to_owned()),
            priority: AdmissionPriority::ProcurementIngestion,
            browser_session: true,
            adapter_session: true,
            source_lane: Some(SourceLaneRequest {
                source_id: "fts".to_owned(),
                source_kind: SourceKind::Procurement,
            }),
            validator_sub_job: false,
            concurrency_key: None,
            max_concurrency: None,
        };
        let second_same_source = AdmissionRequest {
            job_id: "job-fts-2".to_owned(),
            session_id: Some("session-fts-2".to_owned()),
            priority: AdmissionPriority::ProcurementIngestion,
            browser_session: false,
            adapter_session: true,
            source_lane: Some(SourceLaneRequest {
                source_id: "fts".to_owned(),
                source_kind: SourceKind::Procurement,
            }),
            validator_sub_job: false,
            concurrency_key: None,
            max_concurrency: None,
        };
        let other_source = AdmissionRequest {
            job_id: "job-cf-1".to_owned(),
            session_id: Some("session-cf-1".to_owned()),
            priority: AdmissionPriority::ProcurementIngestion,
            browser_session: false,
            adapter_session: true,
            source_lane: Some(SourceLaneRequest {
                source_id: "contracts-finder".to_owned(),
                source_kind: SourceKind::Procurement,
            }),
            validator_sub_job: false,
            concurrency_key: None,
            max_concurrency: None,
        };

        assert!(matches!(arbiter.admit(&first), ArbiterDecision::Granted(_)));
        match arbiter.admit(&second_same_source) {
            ArbiterDecision::Denied(DenialReason::SourceLaneExhausted { source_id, .. }) => {
                assert_eq!(source_id, "fts");
            }
            other => anyhow::bail!("unexpected decision: {other:?}"),
        }
        assert!(matches!(
            arbiter.admit(&other_source),
            ArbiterDecision::Granted(_)
        ));
        Ok(())
    }

    #[test]
    #[allow(clippy::needless_collect)]
    fn l3_t_fault_host_events_005_startup_catchup_orders_overdue_work_by_priority() -> Result<()> {
        let (_temp, state_db) = open_state_db()?;
        seed_sources(state_db.connection())?;
        let registry = SourceRegistry::load(state_db.connection())?;
        let mut scheduler = CronScheduler::from_registry(&registry);
        let now = Utc
            .with_ymd_and_hms(2026, 3, 30, 12, 0, 0)
            .single()
            .unwrap();
        scheduler.refresh_next_run_at(now)?;

        for definition_key in scheduler.definitions().keys().cloned().collect::<Vec<_>>() {
            scheduler.record_enqueued(&definition_key, now - chrono::Duration::days(2))?;
        }

        let ready = scheduler.dispatch_due_jobs(&registry, now)?;
        ensure!(!ready.is_empty());
        let priorities = ready
            .iter()
            .map(|dispatch| dispatch.priority)
            .collect::<Vec<_>>();
        ensure!(priorities.windows(2).all(|pair| pair[0] <= pair[1]));
        Ok(())
    }

    #[test]
    #[allow(clippy::needless_collect)]
    fn l3_t_fault_host_events_006_open_or_cooldown_sources_only_admit_health_and_auth_jobs()
    -> Result<()> {
        let (_temp, state_db) = open_state_db()?;
        seed_sources(state_db.connection())?;
        let now = Utc
            .with_ymd_and_hms(2026, 3, 30, 12, 0, 0)
            .single()
            .unwrap();
        mark_source_runtime(
            state_db.connection(),
            "fts",
            "open",
            Some("2026-03-30T13:00:00Z"),
            Some("failed"),
        )?;
        mark_source_runtime(
            state_db.connection(),
            "alibaba",
            "half_open",
            None,
            Some("failed"),
        )?;

        let registry = SourceRegistry::load(state_db.connection())?;
        let mut scheduler = CronScheduler::from_registry(&registry);
        scheduler.refresh_next_run_at(now)?;
        for definition_key in scheduler.definitions().keys().cloned().collect::<Vec<_>>() {
            scheduler.record_enqueued(&definition_key, now - chrono::Duration::days(2))?;
        }

        let ready = scheduler.dispatch_due_jobs(&registry, now)?;
        ensure!(ready.iter().all(|dispatch| {
            !dispatch.definition.source_id.eq("fts")
                || dispatch.definition.skill_id.contains("source-health")
                || dispatch.definition.skill_id.contains("auth-refresh")
        }));
        ensure!(ready.iter().any(|dispatch| {
            dispatch.definition.source_id == "alibaba"
                && (dispatch.definition.skill_id.contains("source-health")
                    || dispatch.definition.skill_id.contains("auth-refresh"))
        }));
        Ok(())
    }

    #[test]
    fn l3_t_fault_host_events_007_sleep_and_pressure_reuse_existing_health_issue_surface() {
        let sleep = evaluate_host_health(
            Duration::from_secs(10),
            Duration::from_secs(5),
            false,
            false,
            false,
        );
        let pressure =
            evaluate_host_health(Duration::ZERO, Duration::from_secs(5), true, false, false);

        assert_eq!(sleep.issue.as_deref(), Some("HOST_SLEEP_INTERRUPTED"));
        assert_eq!(pressure.issue.as_deref(), Some("HOST_RESOURCE_EXHAUSTED"));
    }

    #[test]
    fn l3_t_fault_host_events_008_stall_tracker_does_not_shadow_sleep_or_pressure() {
        let mut tracker = StallTracker::new(Duration::from_secs(5), Duration::from_millis(100));
        tracker.record_output(Duration::ZERO);
        let stall = tracker.no_progress_issue(Duration::from_millis(150));
        let sleep = evaluate_host_health(
            Duration::from_secs(10),
            Duration::from_secs(5),
            false,
            false,
            true,
        );
        let pressure =
            evaluate_host_health(Duration::ZERO, Duration::from_secs(5), true, false, false);

        assert_eq!(stall, Some(HealthIssue::AgentStalled));
        assert_eq!(sleep.issue.as_deref(), Some("HOST_SLEEP_INTERRUPTED"));
        assert_eq!(pressure.issue.as_deref(), Some("HOST_RESOURCE_EXHAUSTED"));
    }

    #[test]
    fn l3_t_fault_host_events_009_health_issue_inventory_stays_exact() {
        assert_eq!(
            HealthIssue::inventory()
                .into_iter()
                .map(HealthIssue::code)
                .collect::<Vec<_>>(),
            vec![
                "AGENT_STALLED",
                "AGENT_AUTH_CHALLENGE",
                "HOST_SLEEP_INTERRUPTED",
                "HOST_RESOURCE_EXHAUSTED",
            ]
        );
    }

    #[test]
    fn l3_t_fault_host_events_010_auth_sleep_and_pressure_emit_distinct_health_issues() {
        assert_eq!(
            detect_auth_challenge("AUTH CHALLENGE: permission prompt"),
            Some(HealthIssue::AgentAuthChallenge)
        );
        assert_eq!(
            detect_host_sleep(Duration::from_secs(8), Duration::from_secs(5)),
            Some(HealthIssue::HostSleepInterrupted)
        );
        assert_eq!(
            detect_resource_pressure(HostResources {
                cpu_pressure: false,
                memory_pressure: true,
            }),
            Some(HealthIssue::HostResourceExhausted)
        );
    }
}
