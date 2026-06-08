#![forbid(unsafe_code)]
#![allow(clippy::duplicate_mod)]

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
#[path = "../src/jobs/runner.rs"]
#[allow(dead_code)]
mod runner_impl;
#[path = "../src/sources/registry.rs"]
#[allow(dead_code)]
mod sources_impl;
#[path = "../src/scheduler/tick.rs"]
#[allow(dead_code)]
mod tick_impl;

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

mod scheduler {
    pub mod cron {
        pub use crate::cron_impl::*;
    }
}

mod sources {
    pub mod registry {
        pub use crate::sources_impl::*;
    }
}

mod support {
    pub mod db {
        pub use crate::db_support::*;
    }
}

mod tests {
    use anyhow::{Result, anyhow, ensure};
    use chrono::{DateTime, TimeZone, Utc};
    use rusqlite::Connection;
    use rusqlite::params;
    use serde_json::{Value, json};
    use std::fs;
    use std::path::Path;
    use std::process::Command;
    use std::thread;
    use std::time::{Duration, Instant};

    use crate::cron_impl::CronScheduler;
    use crate::db_support::{StateDb, TestStateDb};
    use crate::runner_impl::{EnqueueJobRequest, JobRunner};
    use crate::sources_impl::SourceRegistry;
    use crate::tick_impl::run_scheduler_tick;

    fn ts(
        year: i32,
        month: u32,
        day: u32,
        hour: u32,
        minute: u32,
        second: u32,
    ) -> Result<DateTime<Utc>> {
        Utc.with_ymd_and_hms(year, month, day, hour, minute, second)
            .single()
            .ok_or_else(|| anyhow!("invalid timestamp"))
    }

    fn count_jobs(state_db: &TestStateDb) -> Result<i64> {
        Ok(state_db
            .connection()
            .query_row("SELECT COUNT(*) FROM workflow_jobs", [], |row| row.get(0))?)
    }

    fn count_fts_jobs_at(conn: &Connection, scheduled_for: &str) -> Result<i64> {
        Ok(conn.query_row(
            "\
            SELECT COUNT(*)
            FROM workflow_jobs
            WHERE kind = 'sync_source'
              AND scope_id = 'fts'
              AND scheduled_for = ?1
            ",
            params![scheduled_for],
            |row| row.get(0),
        )?)
    }

    fn count_fts_jobs_at_status(
        conn: &Connection,
        scheduled_for: &str,
        status: &str,
    ) -> Result<i64> {
        Ok(conn.query_row(
            "\
            SELECT COUNT(*)
            FROM workflow_jobs
            WHERE kind = 'sync_source'
              AND scope_id = 'fts'
              AND scheduled_for = ?1
              AND status = ?2
            ",
            params![scheduled_for, status],
            |row| row.get(0),
        )?)
    }

    fn fts_schedule_state(scheduler: &CronScheduler) -> Result<(Option<String>, Option<String>)> {
        let definition = scheduler
            .definitions()
            .get("fts::procurement-ingestion/fts")
            .ok_or_else(|| anyhow!("missing FTS cron definition"))?;
        Ok((
            definition.last_run_at.as_ref().map(DateTime::to_rfc3339),
            definition.next_run_at.as_ref().map(DateTime::to_rfc3339),
        ))
    }

    fn query_single_fts_job(
        conn: &Connection,
        scheduled_for: &str,
    ) -> Result<(String, String, String, String)> {
        Ok(conn.query_row(
            "\
            SELECT kind, scope_id, scheduled_for, input_payload
            FROM workflow_jobs
            WHERE kind = 'sync_source'
              AND scope_id = 'fts'
              AND scheduled_for = ?1
            ",
            params![scheduled_for],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )?)
    }

    fn enqueue_fts_history(db_path: &Path, scheduled_for: DateTime<Utc>) -> Result<()> {
        let mut state_db = StateDb::open(db_path)?;
        JobRunner::new("scheduler-tick-test").enqueue_job(
            &mut state_db,
            &EnqueueJobRequest {
                job_id: "job-scheduler-history".to_owned(),
                kind: "sync_source".to_owned(),
                process_id: None,
                notice_id: None,
                scope_kind: Some("source".to_owned()),
                scope_id: Some("fts".to_owned()),
                parent_job_id: None,
                continuation_chain_id: None,
                validation_required: false,
                scheduled_for: Some(scheduled_for),
                input_payload: json!({
                    "source_id": "fts",
                    "skill_id": "procurement-ingestion/fts",
                    "skill_version": "1.0.0",
                    "cron_expression": "0 15 4 * * *",
                    "scheduled_for": scheduled_for.to_rfc3339(),
                    "schedule_key": "fts::procurement-ingestion/fts",
                }),
                idempotency_key: format!(
                    "cron:fts:procurement-ingestion/fts:{}",
                    scheduled_for.to_rfc3339()
                ),
            },
        )?;
        Ok(())
    }

    #[test]
    fn tick_001_due_slot_enqueues_exactly_one_row() -> Result<()> {
        let state_db = TestStateDb::new()?;
        let registry = SourceRegistry::load(state_db.connection())?;
        let mut scheduler = CronScheduler::from_registry(&registry);
        scheduler.refresh_next_run_at(ts(2026, 6, 5, 4, 10, 0)?)?;

        let mut tick_db = StateDb::open(state_db.db_path())?;
        let enqueued = run_scheduler_tick(
            &mut scheduler,
            &registry,
            &mut tick_db,
            ts(2026, 6, 5, 4, 15, 0)?,
        )?;
        ensure!(enqueued == 1, "tick_001 expected exactly one enqueue");

        let (kind, scope_id, scheduled_for, input_payload) =
            query_single_fts_job(state_db.connection(), "2026-06-05T04:15:00+00:00")?;
        ensure!(matches!(kind.as_str(), "sync_source"));
        ensure!(matches!(scope_id.as_str(), "fts"));
        ensure!(matches!(
            scheduled_for.as_str(),
            "2026-06-05T04:15:00+00:00"
        ));

        let payload: Value = serde_json::from_str(&input_payload)?;
        ensure!(matches!(
            payload
                .pointer("/request/source_id")
                .and_then(Value::as_str),
            Some("fts")
        ));
        ensure!(matches!(
            payload.pointer("/request/skill_id").and_then(Value::as_str),
            Some("procurement-ingestion/fts")
        ));
        ensure!(matches!(
            payload
                .pointer("/request/schedule_key")
                .and_then(Value::as_str),
            Some("fts::procurement-ingestion/fts")
        ));
        ensure!(matches!(
            payload
                .pointer("/runner_meta/idempotency_key")
                .and_then(Value::as_str),
            Some("cron:fts:procurement-ingestion/fts:2026-06-05T04:15:00+00:00")
        ));
        Ok(())
    }

    #[test]
    fn tick_002_repeated_ticks_same_slot_idempotent() -> Result<()> {
        let state_db = TestStateDb::new()?;
        let registry = SourceRegistry::load(state_db.connection())?;
        let due_slot = "2026-06-05T04:15:00+00:00";
        let now = ts(2026, 6, 5, 4, 15, 0)?;

        let mut first_scheduler = CronScheduler::from_registry(&registry);
        first_scheduler.refresh_next_run_at(ts(2026, 6, 5, 4, 10, 0)?)?;
        let mut tick_db = StateDb::open(state_db.db_path())?;
        let first_count = run_scheduler_tick(&mut first_scheduler, &registry, &mut tick_db, now)?;
        ensure!(first_count == 1, "tick_002 expected first tick to enqueue");
        ensure!(
            count_fts_jobs_at(state_db.connection(), due_slot)? == 1,
            "tick_002 expected one pending FTS row after first tick"
        );

        let mut pending_scheduler = CronScheduler::from_registry(&registry);
        pending_scheduler.refresh_next_run_at(ts(2026, 6, 5, 4, 10, 0)?)?;
        let pending_count =
            run_scheduler_tick(&mut pending_scheduler, &registry, &mut tick_db, now)?;
        ensure!(
            pending_count == 1,
            "tick_002 expected pending duplicate to use the same dispatch tuple"
        );
        ensure!(
            count_fts_jobs_at(state_db.connection(), due_slot)? == 1,
            "tick_002 expected pending idempotency to keep one FTS row"
        );

        state_db.connection().execute(
            "\
            UPDATE workflow_jobs
            SET status = 'completed'
            WHERE kind = 'sync_source'
              AND scope_id = 'fts'
              AND scheduled_for = ?1
            ",
            params![due_slot],
        )?;

        let mut completed_scheduler = CronScheduler::from_registry(&registry);
        completed_scheduler.refresh_next_run_at(ts(2026, 6, 5, 4, 10, 0)?)?;
        let completed_count =
            run_scheduler_tick(&mut completed_scheduler, &registry, &mut tick_db, now)?;
        ensure!(
            completed_count == 1,
            "tick_002 expected completed duplicate to use the same dispatch tuple"
        );
        ensure!(
            count_fts_jobs_at(state_db.connection(), due_slot)? == 1,
            "tick_002 expected completed idempotency to keep one FTS row"
        );

        let main_source = include_str!("../src/main.rs");
        ensure!(
            !main_source.contains("enqueue_startup_cron_jobs"),
            "tick_002 expected startup cron enqueue shim to be removed"
        );
        ensure!(
            !main_source.contains("fn enqueue_due_cron_jobs("),
            "tick_002 expected no second cron dispatch implementation in main.rs"
        );
        let cron_source = include_str!("../src/scheduler/cron.rs");
        ensure!(
            !main_source.contains("cron_dedup") && !cron_source.contains("cron_dedup"),
            "tick_002 expected no auxiliary cron dedup table or flag"
        );
        Ok(())
    }

    #[test]
    fn tick_003_long_gap_catchup() -> Result<()> {
        let state_db = TestStateDb::new()?;
        let registry = SourceRegistry::load(state_db.connection())?;
        let mut scheduler = CronScheduler::from_registry(&registry);
        scheduler.refresh_next_run_at(ts(2026, 6, 5, 4, 10, 0)?)?;
        let mut tick_db = StateDb::open(state_db.db_path())?;

        let first_slot = ts(2026, 6, 5, 4, 15, 0)?;
        let first_enqueued =
            run_scheduler_tick(&mut scheduler, &registry, &mut tick_db, first_slot)?;
        ensure!(
            first_enqueued == 1,
            "tick_003 expected first normal FTS slot enqueue"
        );
        ensure!(
            count_fts_jobs_at(state_db.connection(), "2026-06-05T04:15:00+00:00")? == 1,
            "tick_003 expected first FTS row at the normal slot"
        );
        let (last_after_first, next_after_first) = fts_schedule_state(&scheduler)?;
        ensure!(
            last_after_first.as_deref() == Some("2026-06-05T04:15:00+00:00"),
            "tick_003 expected record_enqueued to store first last_run_at"
        );
        ensure!(
            next_after_first.as_deref() == Some("2026-06-06T04:15:00+00:00"),
            "tick_003 expected next FTS run after first slot"
        );

        let gap_now = ts(2026, 6, 8, 6, 0, 0)?;
        let due_dispatches = scheduler.dispatch_due_jobs(&registry, gap_now)?;
        let due_keys = due_dispatches
            .iter()
            .map(|dispatch| dispatch.definition_key.as_str())
            .collect::<Vec<_>>();
        let fts_due = due_dispatches
            .iter()
            .find(|dispatch| dispatch.definition_key == "fts::procurement-ingestion/fts")
            .ok_or_else(|| anyhow!("tick_003 expected FTS catch-up dispatch in {due_keys:?}"))?;
        ensure!(
            fts_due.enqueue_request.scheduled_for == Some(ts(2026, 6, 6, 4, 15, 0)?),
            "tick_003 expected FTS catch-up slot to follow schedule.after(last_run_at); due keys: {due_keys:?}"
        );

        let gap_enqueued = run_scheduler_tick(&mut scheduler, &registry, &mut tick_db, gap_now)?;
        ensure!(
            gap_enqueued >= 1,
            "tick_003 expected long-gap tick to enqueue overdue work"
        );
        ensure!(
            count_fts_jobs_at(state_db.connection(), "2026-06-06T04:15:00+00:00")? == 1,
            "tick_003 expected FTS catch-up row at the next missed slot"
        );
        ensure!(
            count_fts_jobs_at(state_db.connection(), "2026-06-08T04:15:00+00:00")? == 0,
            "tick_003 expected catch-up not to jump directly to the current-day slot"
        );
        let (last_after_gap, next_after_gap) = fts_schedule_state(&scheduler)?;
        ensure!(
            last_after_gap.as_deref() == Some("2026-06-06T04:15:00+00:00"),
            "tick_003 expected record_enqueued to store catch-up last_run_at"
        );
        ensure!(
            next_after_gap.as_deref() == Some("2026-06-07T04:15:00+00:00"),
            "tick_003 expected record_enqueued to advance next_run_at in memory"
        );

        let main_source = include_str!("../src/main.rs");
        let thread_tick_start = main_source
            .find("fn run_scheduler_thread_tick(")
            .ok_or_else(|| anyhow!("run_scheduler_thread_tick function missing"))?;
        let thread_tick_source = main_source
            .get(thread_tick_start..)
            .ok_or_else(|| anyhow!("scheduler thread tick source slice out of bounds"))?;
        let thread_tick_len = thread_tick_source
            .find("fn seed_cron_scheduler_from_history(")
            .ok_or_else(|| anyhow!("scheduler thread tick source slice out of bounds"))?;
        let thread_tick_end = thread_tick_start
            .checked_add(thread_tick_len)
            .ok_or_else(|| anyhow!("scheduler thread tick source slice overflow"))?;
        let thread_tick_body = main_source
            .get(thread_tick_start..thread_tick_end)
            .ok_or_else(|| anyhow!("scheduler thread tick source slice out of bounds"))?;
        ensure!(
            !thread_tick_body.contains("detect_host_sleep"),
            "tick_003 expected host-sleep detection not to suppress scheduler ticks"
        );
        Ok(())
    }

    #[test]
    fn tick_006_failed_slot_reenqueueable() -> Result<()> {
        let state_db = TestStateDb::new()?;
        let registry = SourceRegistry::load(state_db.connection())?;
        let scheduled_for = ts(2026, 6, 5, 4, 15, 0)?;
        let due_slot = scheduled_for.to_rfc3339();
        enqueue_fts_history(state_db.db_path(), scheduled_for)?;
        state_db.connection().execute(
            "\
            UPDATE workflow_jobs
            SET status = 'failed'
            WHERE kind = 'sync_source'
              AND scope_id = 'fts'
              AND scheduled_for = ?1
            ",
            params![due_slot],
        )?;
        ensure!(
            count_fts_jobs_at_status(state_db.connection(), &due_slot, "failed")? == 1,
            "tick_006 expected one failed seed row"
        );

        let mut scheduler = CronScheduler::from_registry(&registry);
        scheduler.refresh_next_run_at(ts(2026, 6, 5, 4, 10, 0)?)?;
        let mut tick_db = StateDb::open(state_db.db_path())?;
        let enqueued = run_scheduler_tick(&mut scheduler, &registry, &mut tick_db, scheduled_for)?;
        ensure!(enqueued == 1, "tick_006 expected failed slot to re-enqueue");
        ensure!(
            count_fts_jobs_at(state_db.connection(), &due_slot)? == 2,
            "tick_006 expected failed row plus new pending row for same slot"
        );
        ensure!(
            count_fts_jobs_at_status(state_db.connection(), &due_slot, "pending")? == 1,
            "tick_006 expected one new pending retry row"
        );
        ensure!(
            count_fts_jobs_at_status(state_db.connection(), &due_slot, "failed")? == 1,
            "tick_006 expected original failed row to remain failed"
        );
        Ok(())
    }

    #[test]
    fn tick_004_clock_seam() -> Result<()> {
        let state_db = TestStateDb::new()?;
        let registry = SourceRegistry::load(state_db.connection())?;
        let mut scheduler = CronScheduler::from_registry(&registry);
        scheduler.refresh_next_run_at(ts(2026, 6, 5, 3, 0, 0)?)?;

        let mut tick_db = StateDb::open(state_db.db_path())?;
        let before_count = run_scheduler_tick(
            &mut scheduler,
            &registry,
            &mut tick_db,
            ts(2026, 6, 5, 3, 0, 0)?,
        )?;
        ensure!(
            before_count == 0,
            "tick_004 expected no before-slot enqueues"
        );
        ensure!(
            count_jobs(&state_db)? == 0,
            "tick_004 expected no workflow jobs before FTS slot"
        );

        let due_count = run_scheduler_tick(
            &mut scheduler,
            &registry,
            &mut tick_db,
            ts(2026, 6, 5, 4, 15, 0)?,
        )?;
        ensure!(due_count > 0, "tick_004 expected due-slot enqueues");

        let fts_count: i64 = state_db.connection().query_row(
            "\
            SELECT COUNT(*)
            FROM workflow_jobs
            WHERE kind = 'sync_source'
              AND scope_id = 'fts'
              AND scheduled_for = ?1
            ",
            params!["2026-06-05T04:15:00+00:00"],
            |row| row.get(0),
        )?;
        ensure!(
            fts_count == 1,
            "tick_004 expected exactly one due FTS sync_source row, got {fts_count}"
        );

        let source = include_str!("../src/scheduler/tick.rs");
        let tick_body_start = source
            .find("pub fn run_scheduler_tick(")
            .ok_or_else(|| anyhow!("run_scheduler_tick function missing"))?;
        let tick_body = source
            .get(tick_body_start..)
            .ok_or_else(|| anyhow!("run_scheduler_tick source slice out of bounds"))?;
        ensure!(
            !tick_body.contains("Utc::now()"),
            "tick_004 run_scheduler_tick must use injected time, not Utc::now()"
        );
        Ok(())
    }

    #[test]
    fn tick_007_serve_scheduler_thread_spawn_poll_terminate() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let state_root = temp.path().join("state");
        let runtime_root = temp.path().join("runtime");
        let artifact_root = temp.path().join("artifacts");
        fs::create_dir_all(&state_root)?;
        fs::create_dir_all(&runtime_root)?;
        fs::create_dir_all(&artifact_root)?;
        let db_path = state_root.join("state.db");
        enqueue_fts_history(&db_path, ts(2026, 6, 5, 4, 15, 0)?)?;

        let mut child = Command::new(env!("CARGO_BIN_EXE_daemon"))
            .arg("--serve")
            .arg("--state-root")
            .arg(&state_root)
            .arg("--runtime-dir")
            .arg(&runtime_root)
            .arg("--artifact-root")
            .arg(&artifact_root)
            .env("DAEMON_SCHEDULER_TICK_INTERVAL_SECS", "1")
            .env_remove("SUPABASE_URL")
            .env_remove("NEXT_PUBLIC_SUPABASE_URL")
            .env_remove("SUPABASE_SERVICE_ROLE_KEY")
            .env_remove("SUPABASE_SERVICE_KEY")
            .spawn()
            .map_err(|error| anyhow!("spawning daemon --serve: {error}"))?;

        let deadline = Instant::now() + Duration::from_secs(10);
        let mut observed_thread_tick = false;
        while Instant::now() < deadline {
            if let Some(status) = child.try_wait()? {
                anyhow::bail!("daemon --serve exited before scheduler tick: {status}");
            }
            if let Ok(conn) = Connection::open(&db_path)
                && let Ok(count) = count_fts_jobs_at(&conn, "2026-06-06T04:15:00+00:00")
                && count == 1
            {
                observed_thread_tick = true;
                break;
            }
            thread::sleep(Duration::from_millis(100));
        }

        if child.try_wait()?.is_none() {
            child.kill()?;
        }
        let _ = child.wait()?;
        ensure!(
            observed_thread_tick,
            "tick_007 expected serve scheduler thread to enqueue cold-start catch-up"
        );

        let source = include_str!("../src/main.rs");
        ensure!(
            source.contains("struct SchedulerTickThread"),
            "tick_007 expected a dedicated scheduler thread owner"
        );
        ensure!(
            source.contains("recv_timeout(interval)"),
            "tick_007 expected scheduler thread to use recv_timeout loop"
        );
        ensure!(
            source.contains("SchedulerTickThread::spawn("),
            "tick_007 expected serve path to spawn the scheduler thread"
        );
        Ok(())
    }
}
