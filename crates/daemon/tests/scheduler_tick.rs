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
    use rusqlite::params;

    use crate::cron_impl::CronScheduler;
    use crate::db_support::{StateDb, TestStateDb};
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
}
