#![forbid(unsafe_code)]

#[path = "../src/agents/contracts.rs"]
#[allow(dead_code)]
mod contracts_impl;
#[path = "support/db.rs"]
mod db_support;
#[path = "../src/jobs/runner.rs"]
#[allow(dead_code)]
mod runner_impl;

mod support {
    pub mod db {
        pub use crate::db_support::*;
    }
}

mod tests {
    use std::path::Path;
    use std::sync::{Arc, Barrier};
    use std::thread;

    use anyhow::{Context, Result, anyhow, ensure};
    use chrono::{DateTime, Utc};
    use rusqlite::params;
    use serde_json::json;

    use crate::db_support::{StateDb, TestStateDb};
    use crate::runner_impl::JobRunner;

    fn seed_job(
        conn: &rusqlite::Connection,
        job_id: &str,
        status: &str,
        scheduled_for: Option<&str>,
        created_at: &str,
    ) -> Result<()> {
        conn.execute(
            "\
            INSERT INTO workflow_jobs (
              id, kind, status, scheduled_for, input_payload, created_at
            ) VALUES (?1, 'stage-job', ?2, ?3, ?4, ?5)
            ",
            params![
                job_id,
                status,
                scheduled_for,
                json!({ "job_id": job_id }).to_string(),
                created_at,
            ],
        )?;
        Ok(())
    }

    fn seed_running_job(
        conn: &rusqlite::Connection,
        job_id: &str,
        lease_owner: &str,
        lease_expires_at: &str,
        attempt_count: i64,
        created_at: &str,
    ) -> Result<()> {
        conn.execute(
            "\
            INSERT INTO workflow_jobs (
              id, kind, status, lease_owner, lease_expires_at, attempt_count, input_payload,
              started_at, created_at
            ) VALUES (?1, 'stage-job', 'running', ?2, ?3, ?4, ?5, ?6, ?7)
            ",
            params![
                job_id,
                lease_owner,
                lease_expires_at,
                attempt_count,
                json!({ "job_id": job_id }).to_string(),
                "2026-03-24T00:00:00Z",
                created_at,
            ],
        )?;
        Ok(())
    }

    fn parse_rfc3339(timestamp: &str) -> Result<DateTime<Utc>> {
        DateTime::parse_from_rfc3339(timestamp)
            .map(|value| value.with_timezone(&Utc))
            .with_context(|| format!("parsing timestamp {timestamp}"))
    }

    #[test]
    fn l3_t_job_claim_001_concurrent_claimers_cannot_double_claim_same_runnable_job() -> Result<()>
    {
        let state_db = TestStateDb::new()?;
        seed_job(
            state_db.connection(),
            "job-claim-001",
            "pending",
            Some("2026-03-25T01:00:00Z"),
            "2026-03-25T00:00:00Z",
        )?;

        let db_path = state_db.db_path().to_path_buf();
        let barrier = Arc::new(Barrier::new(4));
        let mut handles = Vec::new();
        for index in 0..4 {
            let db_path = db_path.clone();
            let barrier = Arc::clone(&barrier);
            handles.push(thread::spawn(move || -> Result<Option<String>> {
                let state_db = StateDb::open(&db_path)?;
                barrier.wait();
                Ok(JobRunner::new(format!("claimer-{index}"))
                    .claim_next_job(state_db.connection())?
                    .map(|job| job.id))
            }));
        }

        let mut claimed_ids = Vec::new();
        for handle in handles {
            let result = handle
                .join()
                .map_err(|_| anyhow!("L3-T-job_claim-001 worker thread panicked"))??;
            if let Some(job_id) = result {
                claimed_ids.push(job_id);
            }
        }

        ensure!(
            claimed_ids.len() == 1,
            "L3-T-job_claim-001 expected exactly one claimed job, got {}",
            claimed_ids.len()
        );
        ensure!(
            claimed_ids[0] == "job-claim-001",
            "L3-T-job_claim-001 claimed unexpected job {}",
            claimed_ids[0]
        );

        let running_count: i64 = state_db.connection().query_row(
            "SELECT COUNT(*) FROM workflow_jobs WHERE id = ?1 AND status = 'running'",
            params!["job-claim-001"],
            |row| row.get(0),
        )?;
        ensure!(
            running_count == 1,
            "L3-T-job_claim-001 expected one running row, found {running_count}"
        );
        Ok(())
    }

    #[test]
    fn l3_t_job_claim_002_claim_eligibility_and_ordering_match_runner_contract() -> Result<()> {
        let state_db = TestStateDb::new()?;
        seed_job(
            state_db.connection(),
            "job-retry-first",
            "retry",
            Some("2026-03-25T01:00:00Z"),
            "2026-03-25T00:00:00Z",
        )?;
        seed_job(
            state_db.connection(),
            "job-pending-unscheduled",
            "pending",
            None,
            "2026-03-25T02:00:00Z",
        )?;
        seed_job(
            state_db.connection(),
            "job-pending-later",
            "pending",
            Some("2026-03-25T03:00:00Z"),
            "2026-03-25T00:30:00Z",
        )?;
        seed_job(
            state_db.connection(),
            "job-running-ignored",
            "running",
            Some("2026-03-24T00:00:00Z"),
            "2026-03-24T00:00:00Z",
        )?;
        state_db.connection().execute(
            "UPDATE workflow_jobs SET lease_owner = 'busy', started_at = '2026-03-24T00:00:00Z' WHERE id = ?1",
            params!["job-running-ignored"],
        )?;
        seed_job(
            state_db.connection(),
            "job-completed-ignored",
            "completed",
            Some("2026-03-24T00:30:00Z"),
            "2026-03-24T00:30:00Z",
        )?;

        let first = JobRunner::new("order-1")
            .claim_next_job(state_db.connection())?
            .ok_or_else(|| anyhow!("L3-T-job_claim-002 expected first claim"))?;
        let second = JobRunner::new("order-2")
            .claim_next_job(state_db.connection())?
            .ok_or_else(|| anyhow!("L3-T-job_claim-002 expected second claim"))?;
        let third = JobRunner::new("order-3")
            .claim_next_job(state_db.connection())?
            .ok_or_else(|| anyhow!("L3-T-job_claim-002 expected third claim"))?;
        let fourth = JobRunner::new("order-4").claim_next_job(state_db.connection())?;

        ensure!(
            first.id == "job-retry-first",
            "L3-T-job_claim-002 expected retry job first, got {}",
            first.id
        );
        ensure!(
            second.id == "job-pending-unscheduled",
            "L3-T-job_claim-002 expected unscheduled pending job second, got {}",
            second.id
        );
        ensure!(
            third.id == "job-pending-later",
            "L3-T-job_claim-002 expected later scheduled job third, got {}",
            third.id
        );
        ensure!(
            fourth.is_none(),
            "L3-T-job_claim-002 expected running/completed jobs to remain unclaimable"
        );
        Ok(())
    }

    #[test]
    fn l3_t_job_claim_003_successful_claim_sets_attempt_and_runtime_metadata_atomically()
    -> Result<()> {
        let state_db = TestStateDb::new()?;
        seed_job(
            state_db.connection(),
            "job-claim-003",
            "pending",
            None,
            "2026-03-25T00:00:00Z",
        )?;

        let claimed = JobRunner::new("executor-003")
            .claim_next_job(state_db.connection())?
            .ok_or_else(|| anyhow!("L3-T-job_claim-003 expected a claimed job"))?;
        ensure!(
            claimed.id == "job-claim-003",
            "L3-T-job_claim-003 claimed wrong job {}",
            claimed.id
        );

        let (status, lease_owner, started_at, lease_expires_at, attempt_count): (
            String,
            Option<String>,
            Option<String>,
            Option<String>,
            i64,
        ) = state_db.connection().query_row(
            "\
            SELECT status, lease_owner, started_at, lease_expires_at, attempt_count
            FROM workflow_jobs
            WHERE id = ?1
            ",
            params!["job-claim-003"],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            },
        )?;

        ensure!(
            status == "running",
            "L3-T-job_claim-003 expected running status, got {status}"
        );
        ensure!(
            lease_owner.as_deref() == Some("executor-003"),
            "L3-T-job_claim-003 expected lease_owner to be executor-003"
        );
        let started_at = started_at
            .ok_or_else(|| anyhow!("L3-T-job_claim-003 expected started_at to be written"))?;
        let lease_expires_at = lease_expires_at
            .ok_or_else(|| anyhow!("L3-T-job_claim-003 expected lease_expires_at to be written"))?;
        ensure!(
            attempt_count == 1,
            "L3-T-job_claim-003 expected attempt_count to increment to 1, got {attempt_count}"
        );
        ensure!(
            parse_rfc3339(&lease_expires_at)? > parse_rfc3339(&started_at)?,
            "L3-T-job_claim-003 expected lease_expires_at after started_at"
        );
        Ok(())
    }

    #[test]
    fn l3_t_job_claim_004_expired_lease_returns_work_to_runnable_pool_exactly_once() -> Result<()> {
        let state_db = TestStateDb::new()?;
        seed_running_job(
            state_db.connection(),
            "job-expired-lease",
            "stuck-owner",
            "2026-03-24T00:00:00Z",
            1,
            "2026-03-24T00:00:00Z",
        )?;

        let reclaimed = JobRunner::new("executor-reclaim-1")
            .claim_next_job(state_db.connection())?
            .ok_or_else(|| anyhow!("L3-T-job_claim-004 expected reclaimed job"))?;
        let second = JobRunner::new("executor-reclaim-2").claim_next_job(state_db.connection())?;

        ensure!(
            reclaimed.id == "job-expired-lease",
            "L3-T-job_claim-004 reclaimed unexpected job {}",
            reclaimed.id
        );
        ensure!(
            second.is_none(),
            "L3-T-job_claim-004 expected the expired lease to return exactly once"
        );

        let (status, lease_owner, attempt_count, row_count): (String, Option<String>, i64, i64) =
            state_db.connection().query_row(
                "\
                SELECT status, lease_owner, attempt_count,
                       (SELECT COUNT(*) FROM workflow_jobs WHERE id = ?1)
                FROM workflow_jobs
                WHERE id = ?1
                ",
                params!["job-expired-lease"],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )?;

        ensure!(
            status == "running",
            "L3-T-job_claim-004 expected reclaimed job to be running, got {status}"
        );
        ensure!(
            lease_owner.as_deref() == Some("executor-reclaim-1"),
            "L3-T-job_claim-004 expected new lease owner after reclaim"
        );
        ensure!(
            attempt_count == 2,
            "L3-T-job_claim-004 expected reclaim to increment attempt_count to 2, got {attempt_count}"
        );
        ensure!(
            row_count == 1,
            "L3-T-job_claim-004 expected one authoritative row, found {row_count}"
        );
        Ok(())
    }

    #[test]
    fn l3_t_job_claim_005_runner_uses_claim_module_as_single_sql_claim_surface() -> Result<()> {
        let runner_source = include_str!("../src/jobs/runner.rs");
        let claim_source = include_str!("../src/jobs/claim.rs");
        let test_surface = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/job_claim.rs");

        ensure!(
            test_surface.exists(),
            "L3-T-job_claim-005 expected dedicated job_claim.rs test surface"
        );
        ensure!(
            runner_source.contains("claim::claim_next_job(conn, &self.executor_id)"),
            "L3-T-job_claim-005 expected runner to delegate claim work to jobs/claim.rs"
        );
        ensure!(
            !runner_source
                .contains("SELECT id, kind, process_id, scope_kind, scope_id, input_payload"),
            "L3-T-job_claim-005 found competing inline claim selection SQL in runner.rs"
        );
        ensure!(
            !runner_source.contains("ORDER BY COALESCE(scheduled_for, created_at), created_at"),
            "L3-T-job_claim-005 found competing inline claim ordering SQL in runner.rs"
        );
        ensure!(
            claim_source.contains("WHERE status IN ('pending', 'retry')"),
            "L3-T-job_claim-005 expected claim.rs to remain the SQL eligibility surface"
        );
        ensure!(
            claim_source.contains("attempt_count = attempt_count + 1"),
            "L3-T-job_claim-005 expected claim.rs to own attempt-count mutation"
        );
        Ok(())
    }
}
