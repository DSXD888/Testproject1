use anyhow::{Result, bail};

use crate::jobs::runner::{ClaimedJob, JobRunner};
use crate::{LiveLoopReport, RuntimeBootstrap};

#[allow(clippy::future_not_send)]
pub async fn run_bid_workflow_job(
    _runtime: &mut RuntimeBootstrap,
    _runner: &JobRunner,
    _claimed: ClaimedJob,
) -> Result<LiveLoopReport> {
    bail!("Phase 3 implements bid workflow job execution")
}
