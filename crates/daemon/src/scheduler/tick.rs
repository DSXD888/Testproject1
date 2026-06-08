#![forbid(unsafe_code)]

use anyhow::{Context, Result, anyhow};
use chrono::{DateTime, Utc};

#[cfg(not(test))]
use crate::db::sqlite::StateDb;
use crate::jobs::runner::JobRunner;
use crate::scheduler::cron::CronScheduler;
use crate::sources::registry::SourceRegistry;
#[cfg(test)]
use crate::support::db::StateDb;

pub const SCHEDULER_TICK_EXECUTOR_ID: &str = "local-daemon";

pub fn run_scheduler_tick(
    scheduler: &mut CronScheduler,
    registry: &SourceRegistry,
    state_db: &mut StateDb,
    now: DateTime<Utc>,
) -> Result<usize> {
    let dispatches = scheduler
        .dispatch_due_jobs(registry, now)
        .context("collecting due cron jobs during scheduler tick")?;
    if dispatches.is_empty() {
        return Ok(0);
    }

    let runner = JobRunner::new(SCHEDULER_TICK_EXECUTOR_ID);
    let mut enqueued = 0_usize;
    for dispatch in dispatches {
        let scheduled_for = dispatch.enqueue_request.scheduled_for.ok_or_else(|| {
            anyhow!(
                "cron dispatch {} missing scheduled_for",
                dispatch.definition_key
            )
        })?;
        runner
            .enqueue_job(state_db, &dispatch.enqueue_request)
            .with_context(|| format!("enqueuing cron root {}", dispatch.definition_key))?;
        scheduler
            .record_enqueued(&dispatch.definition_key, scheduled_for)
            .with_context(|| format!("recording cron enqueue for {}", dispatch.definition_key))?;
        enqueued += 1;
    }

    Ok(enqueued)
}
