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
    use chrono::{DateTime, Duration as ChronoDuration, TimeZone, Utc};
    use rusqlite::{Connection, params};

    use crate::cron_impl::CronScheduler;
    use crate::db_support::{StateDb, TestStateDb};
    use crate::sources_impl::{SourceRegistry, set_source_enabled, source_registry_generation};
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

    fn set_fts_source_enabled(conn: &Connection, enabled: bool) -> Result<bool> {
        set_source_enabled(conn, "fts", enabled)
    }

    fn count_jobs(conn: &Connection) -> Result<i64> {
        Ok(conn.query_row("SELECT COUNT(*) FROM workflow_jobs", [], |row| row.get(0))?)
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

    #[test]
    fn tick_005_registry_reload_on_generation_bump() -> Result<()> {
        let state_db = TestStateDb::new()?;
        let mut tick_db = StateDb::open(state_db.db_path())?;
        let _ = set_fts_source_enabled(state_db.connection(), false)?;

        let disabled_registry = SourceRegistry::load(state_db.connection())?;
        let mut disabled_scheduler = CronScheduler::from_registry(&disabled_registry);
        disabled_scheduler.refresh_next_run_at(ts(2026, 6, 5, 4, 10, 0)?)?;
        let disabled_definition = disabled_scheduler
            .definitions()
            .get("fts::procurement-ingestion/fts")
            .ok_or_else(|| anyhow!("missing disabled FTS cron definition"))?;
        ensure!(
            !disabled_definition.enabled,
            "tick_005 expected FTS cron definition to start disabled"
        );

        let due_now = ts(2026, 6, 5, 4, 15, 0)?;
        let disabled_count = run_scheduler_tick(
            &mut disabled_scheduler,
            &disabled_registry,
            &mut tick_db,
            due_now,
        )?;
        ensure!(
            disabled_count == 0,
            "tick_005 expected disabled source to enqueue nothing"
        );
        ensure!(
            count_jobs(state_db.connection())? == 0,
            "tick_005 expected no workflow rows before source enablement"
        );

        let observed_generation = source_registry_generation();
        let enabled_source_changed = set_fts_source_enabled(state_db.connection(), true)?;
        ensure!(
            enabled_source_changed,
            "tick_005 expected enabling FTS to mutate the source registry"
        );
        let bumped_generation = source_registry_generation();
        ensure!(
            bumped_generation > observed_generation,
            "tick_005 expected source mutation path to bump Q9 generation"
        );

        let reloaded_registry = SourceRegistry::load(state_db.connection())?;
        let mut reloaded_scheduler = CronScheduler::from_registry(&reloaded_registry);
        let reload_anchor = due_now
            .checked_sub_signed(ChronoDuration::seconds(1))
            .ok_or_else(|| anyhow!("tick_005 reload anchor underflow"))?;
        reloaded_scheduler.refresh_next_run_at(reload_anchor)?;
        let enabled_definition = reloaded_scheduler
            .definitions()
            .get("fts::procurement-ingestion/fts")
            .ok_or_else(|| anyhow!("missing reloaded FTS cron definition"))?;
        ensure!(
            enabled_definition.enabled,
            "tick_005 expected reloaded FTS cron definition to be enabled"
        );

        let reloaded_count = run_scheduler_tick(
            &mut reloaded_scheduler,
            &reloaded_registry,
            &mut tick_db,
            due_now,
        )?;
        ensure!(
            reloaded_count == 1,
            "tick_005 expected reloaded source to enqueue exactly the due slot"
        );
        ensure!(
            count_fts_jobs_at(state_db.connection(), "2026-06-05T04:15:00+00:00")? == 1,
            "tick_005 expected one FTS job at the current due slot"
        );
        ensure!(
            count_fts_jobs_at(state_db.connection(), "2026-06-04T04:15:00+00:00")? == 0,
            "tick_005 expected reload reseed to avoid older catch-up rows"
        );

        let (last_run_at, next_run_at) = fts_schedule_state(&reloaded_scheduler)?;
        ensure!(
            last_run_at.as_deref() == Some("2026-06-05T04:15:00+00:00"),
            "tick_005 expected record_enqueued to store the reloaded due slot"
        );
        ensure!(
            next_run_at.as_deref() == Some("2026-06-06T04:15:00+00:00"),
            "tick_005 expected next run to advance from the reloaded due slot"
        );

        Ok(())
    }
}
