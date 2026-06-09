#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use anyhow::{Context, Result, ensure};
use chrono::{DateTime, TimeDelta, Utc};
use rusqlite::{
    Connection, OptionalExtension, Transaction, TransactionBehavior, params, params_from_iter,
};
use serde_json::Value;

use crate::jobs::runner::ClaimedJob;

const DEFAULT_WORKFLOW_JOB_LEASE_SECONDS: i64 = 300;
const DEFAULT_WORKFLOW_JOB_HEARTBEAT_SECONDS: u64 = 60;

pub struct JobLeaseHeartbeat {
    stop_tx: Option<mpsc::Sender<()>>,
    join_handle: Option<JoinHandle<()>>,
}

impl JobLeaseHeartbeat {
    #[must_use]
    pub fn start(state_db_path: PathBuf, job_id: String, lease_owner: String) -> Self {
        let (stop_tx, stop_rx) = mpsc::channel();
        let join_handle = thread::spawn(move || {
            loop {
                match stop_rx
                    .recv_timeout(Duration::from_secs(DEFAULT_WORKFLOW_JOB_HEARTBEAT_SECONDS))
                {
                    Ok(()) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
                    Err(mpsc::RecvTimeoutError::Timeout) => {
                        let _ = renew_job_lease(&state_db_path, &job_id, &lease_owner, Utc::now());
                    }
                }
            }
        });

        Self {
            stop_tx: Some(stop_tx),
            join_handle: Some(join_handle),
        }
    }

    fn stop(&mut self) {
        if let Some(stop_tx) = self.stop_tx.take() {
            let _ = stop_tx.send(());
        }
        if let Some(join_handle) = self.join_handle.take() {
            let _ = join_handle.join();
        }
    }
}

impl Drop for JobLeaseHeartbeat {
    fn drop(&mut self) {
        self.stop();
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ClaimableJobRow {
    id: String,
    kind: String,
    process_id: Option<String>,
    scope_kind: Option<String>,
    scope_id: Option<String>,
    input_payload: String,
}

pub(super) fn claim_next_job(conn: &Connection, executor_id: &str) -> Result<Option<ClaimedJob>> {
    let now = Utc::now();
    let tx = Transaction::new_unchecked(conn, TransactionBehavior::Immediate)
        .context("starting immediate transaction for workflow job claim")?;

    recycle_expired_leases(&tx, now)?;

    let Some(row) = select_next_claimable_job(&tx)? else {
        tx.commit()
            .context("committing empty workflow job claim transaction")?;
        return Ok(None);
    };

    let claimed_job = build_claimed_job(row)?;
    apply_claim(&tx, &claimed_job.id, executor_id, now)?;

    tx.commit().context("committing workflow job claim")?;

    Ok(Some(claimed_job))
}

pub fn claim_next_job_for_kinds(
    conn: &Connection,
    executor_id: &str,
    allowed_kinds: &[&str],
) -> Result<Option<ClaimedJob>> {
    ensure!(
        !allowed_kinds.is_empty(),
        "workflow job lane claim requires at least one allowed kind"
    );

    let now = Utc::now();
    let tx = Transaction::new_unchecked(conn, TransactionBehavior::Immediate)
        .context("starting immediate transaction for lane-filtered workflow job claim")?;

    recycle_expired_leases(&tx, now)?;

    let Some(row) = select_next_claimable_job_for_kinds(&tx, allowed_kinds)? else {
        tx.commit()
            .context("committing empty lane-filtered workflow job claim transaction")?;
        return Ok(None);
    };

    let claimed_job = build_claimed_job(row)?;
    apply_claim(&tx, &claimed_job.id, executor_id, now)?;

    tx.commit()
        .context("committing lane-filtered workflow job claim")?;

    Ok(Some(claimed_job))
}

/// Claim a specific pending/retry job by id, bypassing FIFO ordering.
/// Used by `--ingest` flows so they can reliably claim the job they just
/// enqueued even when older unrelated jobs (extraction, validation, etc.)
/// are also pending.
pub(super) fn claim_job_by_id(
    conn: &Connection,
    executor_id: &str,
    job_id: &str,
) -> Result<Option<ClaimedJob>> {
    let now = Utc::now();
    let tx = Transaction::new_unchecked(conn, TransactionBehavior::Immediate)
        .context("starting immediate transaction for workflow job claim-by-id")?;

    recycle_expired_leases(&tx, now)?;

    let Some(row) = select_claimable_job_by_id(&tx, job_id)? else {
        tx.commit()
            .context("committing empty workflow job claim-by-id transaction")?;
        return Ok(None);
    };

    let claimed_job = build_claimed_job(row)?;
    apply_claim(&tx, &claimed_job.id, executor_id, now)?;

    tx.commit().context("committing workflow job claim-by-id")?;

    Ok(Some(claimed_job))
}

fn select_claimable_job_by_id(
    tx: &Transaction<'_>,
    job_id: &str,
) -> Result<Option<ClaimableJobRow>> {
    let mut statement = tx.prepare(
        "\
        SELECT id, kind, process_id, scope_kind, scope_id, input_payload
        FROM workflow_jobs
        WHERE id = ?1
          AND status IN ('pending', 'retry')
        LIMIT 1
        ",
    )?;

    statement
        .query_row(params![job_id], |row| {
            Ok(ClaimableJobRow {
                id: row.get(0)?,
                kind: row.get(1)?,
                process_id: row.get(2)?,
                scope_kind: row.get(3)?,
                scope_id: row.get(4)?,
                input_payload: row.get(5)?,
            })
        })
        .optional()
        .context("selecting claimable workflow job by id")
}

fn recycle_expired_leases(tx: &Transaction<'_>, now: DateTime<Utc>) -> Result<()> {
    tx.execute(
        "\
        UPDATE workflow_jobs
        SET status = 'retry',
            lease_owner = NULL,
            lease_expires_at = NULL,
            started_at = NULL
        WHERE status = 'running'
          AND completed_at IS NULL
          AND lease_expires_at IS NOT NULL
          AND datetime(lease_expires_at) <= datetime(?1)
        ",
        params![now.to_rfc3339()],
    )
    .context("recycling expired workflow job leases")?;
    Ok(())
}

fn select_next_claimable_job(tx: &Transaction<'_>) -> Result<Option<ClaimableJobRow>> {
    let mut statement = tx.prepare(
        "\
        SELECT id, kind, process_id, scope_kind, scope_id, input_payload
        FROM workflow_jobs
        WHERE status IN ('pending', 'retry')
        -- FR-G1.6: scheduled/overdue work must sort ahead of newer unscheduled manual work.
        ORDER BY COALESCE(scheduled_for, created_at), created_at
        LIMIT 1
        ",
    )?;

    statement
        .query_row([], |row| {
            Ok(ClaimableJobRow {
                id: row.get(0)?,
                kind: row.get(1)?,
                process_id: row.get(2)?,
                scope_kind: row.get(3)?,
                scope_id: row.get(4)?,
                input_payload: row.get(5)?,
            })
        })
        .optional()
        .context("selecting next claimable workflow job")
}

fn select_next_claimable_job_for_kinds(
    tx: &Transaction<'_>,
    allowed_kinds: &[&str],
) -> Result<Option<ClaimableJobRow>> {
    let placeholders = std::iter::repeat_n("?", allowed_kinds.len())
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!(
        "\
        SELECT id, kind, process_id, scope_kind, scope_id, input_payload
        FROM workflow_jobs
        WHERE status IN ('pending', 'retry')
          AND kind IN ({placeholders})
        -- FR-G2.1: each autonomous worker lane claims only its job class.
        -- FR-G1.6: scheduled/overdue work must sort ahead of newer unscheduled manual work.
        ORDER BY COALESCE(scheduled_for, created_at), created_at
        LIMIT 1
        "
    );
    let mut statement = tx.prepare(&sql)?;

    statement
        .query_row(params_from_iter(allowed_kinds.iter().copied()), |row| {
            Ok(ClaimableJobRow {
                id: row.get(0)?,
                kind: row.get(1)?,
                process_id: row.get(2)?,
                scope_kind: row.get(3)?,
                scope_id: row.get(4)?,
                input_payload: row.get(5)?,
            })
        })
        .optional()
        .context("selecting next lane-filtered claimable workflow job")
}

fn build_claimed_job(row: ClaimableJobRow) -> Result<ClaimedJob> {
    let input_payload = serde_json::from_str::<Value>(&row.input_payload).map_err(|error| {
        anyhow::anyhow!("parsing workflow job input payload for {}: {error}", row.id)
    })?;

    Ok(ClaimedJob {
        id: row.id,
        kind: row.kind,
        process_id: row.process_id,
        scope_kind: row.scope_kind,
        scope_id: row.scope_id,
        input_payload,
    })
}

fn apply_claim(
    tx: &Transaction<'_>,
    job_id: &str,
    executor_id: &str,
    now: DateTime<Utc>,
) -> Result<()> {
    let updated = tx
        .execute(
            "\
            UPDATE workflow_jobs
            SET status = 'running',
                lease_owner = ?1,
                lease_expires_at = ?2,
                started_at = ?3,
                attempt_count = attempt_count + 1
            WHERE id = ?4
              AND status IN ('pending', 'retry')
            ",
            params![
                executor_id,
                lease_deadline(now).to_rfc3339(),
                now.to_rfc3339(),
                job_id
            ],
        )
        .context("marking workflow job running")?;

    ensure!(
        updated == 1,
        "workflow job {job_id} was not claimed atomically"
    );
    Ok(())
}

fn renew_job_lease(
    state_db_path: &Path,
    job_id: &str,
    lease_owner: &str,
    now: DateTime<Utc>,
) -> Result<()> {
    let conn = Connection::open(state_db_path)
        .with_context(|| format!("opening SQLite database {}", state_db_path.display()))?;
    conn.busy_timeout(Duration::from_secs(5)).with_context(|| {
        format!(
            "configuring SQLite busy timeout for {}",
            state_db_path.display()
        )
    })?;
    conn.execute(
        "\
        UPDATE workflow_jobs
        SET lease_expires_at = ?1
        WHERE id = ?2
          AND status = 'running'
          AND completed_at IS NULL
          AND lease_owner = ?3
          AND lease_expires_at IS NOT NULL
        ",
        params![lease_deadline(now).to_rfc3339(), job_id, lease_owner],
    )
    .with_context(|| format!("renewing workflow job lease for {job_id}"))?;
    Ok(())
}

fn lease_deadline(now: DateTime<Utc>) -> DateTime<Utc> {
    now + TimeDelta::seconds(DEFAULT_WORKFLOW_JOB_LEASE_SECONDS)
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use chrono::{DateTime, Utc};
    use rusqlite::{Connection, params};
    use serde_json::json;

    use super::{claim_job_by_id, claim_next_job_for_kinds};

    fn workflow_jobs_conn() -> Result<Connection> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(
            "\
            CREATE TABLE workflow_jobs (
              id TEXT PRIMARY KEY,
              kind TEXT NOT NULL,
              process_id TEXT,
              notice_id TEXT,
              scope_kind TEXT,
              scope_id TEXT,
              parent_job_id TEXT,
              continuation_chain_id TEXT,
              chain_closed INTEGER NOT NULL DEFAULT 0,
              validation_required INTEGER NOT NULL DEFAULT 0,
              validation_verdict_id TEXT,
              status TEXT NOT NULL,
              scheduled_for TEXT,
              lease_owner TEXT,
              lease_expires_at TEXT,
              attempt_count INTEGER NOT NULL DEFAULT 0,
              input_payload TEXT NOT NULL,
              output_payload TEXT,
              last_error TEXT,
              started_at TEXT,
              completed_at TEXT,
              created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );
            ",
        )?;
        Ok(conn)
    }

    fn insert_job(conn: &Connection, job_id: &str, status: &str, created_at: &str) -> Result<()> {
        conn.execute(
            "\
            INSERT INTO workflow_jobs (
              id, kind, process_id, scope_kind, scope_id, status, input_payload, created_at
            ) VALUES (?1, 'landed_cost_test', 'process-test', 'candidate_quote', ?1, ?2, ?3, ?4)
            ",
            params![
                job_id,
                status,
                json!({"job_id": job_id}).to_string(),
                created_at
            ],
        )?;
        Ok(())
    }

    fn insert_job_with_kind(
        conn: &Connection,
        job_id: &str,
        kind: &str,
        status: &str,
        created_at: &str,
        scheduled_for: Option<&str>,
    ) -> Result<()> {
        conn.execute(
            "\
            INSERT INTO workflow_jobs (
              id, kind, process_id, scope_kind, scope_id, status,
              scheduled_for, input_payload, created_at
            ) VALUES (?1, ?2, 'process-test', 'pipeline', ?1, ?3, ?4, ?5, ?6)
            ",
            params![
                job_id,
                kind,
                status,
                scheduled_for,
                json!({"job_id": job_id, "kind": kind}).to_string(),
                created_at
            ],
        )?;
        Ok(())
    }

    fn insert_running_job_with_kind(
        conn: &Connection,
        job_id: &str,
        kind: &str,
        lease_owner: &str,
        lease_expires_at: &str,
        attempt_count: i64,
        created_at: &str,
    ) -> Result<()> {
        conn.execute(
            "\
            INSERT INTO workflow_jobs (
              id, kind, process_id, scope_kind, scope_id, status, lease_owner,
              lease_expires_at, attempt_count, input_payload, started_at, created_at
            ) VALUES (
              ?1, ?2, 'process-test', 'pipeline', ?1, 'running', ?3, ?4, ?5, ?6,
              '2026-04-01T00:00:00Z', ?7
            )
            ",
            params![
                job_id,
                kind,
                lease_owner,
                lease_expires_at,
                attempt_count,
                json!({"job_id": job_id, "kind": kind}).to_string(),
                created_at
            ],
        )?;
        Ok(())
    }

    #[test]
    fn lease_deadline_uses_existing_three_hundred_second_model() -> Result<()> {
        let now = DateTime::parse_from_rfc3339("2026-04-01T12:00:00Z")?.with_timezone(&Utc);
        let deadline = super::lease_deadline(now);

        assert_eq!((deadline - now).num_seconds(), 300);
        Ok(())
    }

    #[test]
    fn heartbeat_interval_uses_existing_sixty_second_model() {
        assert_eq!(super::DEFAULT_WORKFLOW_JOB_HEARTBEAT_SECONDS, 60);
    }

    #[test]
    fn claim_job_by_id_returns_specific_job_when_older_jobs_exist() -> Result<()> {
        let conn = workflow_jobs_conn()?;
        insert_job(&conn, "older-job", "pending", "2026-04-01T00:00:00Z")?;
        insert_job(&conn, "newer-job", "pending", "2026-04-02T00:00:00Z")?;

        let claimed = claim_job_by_id(&conn, "executor-test", "newer-job")?
            .ok_or_else(|| anyhow::anyhow!("newer job should be claimable"))?;

        assert_eq!(claimed.id, "newer-job");
        let (status, lease_owner, attempt_count): (String, String, i64) = conn.query_row(
            "\
            SELECT status, lease_owner, attempt_count
            FROM workflow_jobs
            WHERE id = 'newer-job'
            ",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )?;
        assert_eq!(status, "running");
        assert_eq!(lease_owner, "executor-test");
        assert_eq!(attempt_count, 1);

        let older_status: String = conn.query_row(
            "SELECT status FROM workflow_jobs WHERE id = 'older-job'",
            [],
            |row| row.get(0),
        )?;
        assert_eq!(older_status, "pending");
        Ok(())
    }

    #[test]
    fn claim_job_by_id_returns_none_for_nonexistent_or_completed_jobs() -> Result<()> {
        let conn = workflow_jobs_conn()?;
        insert_job(&conn, "completed-job", "completed", "2026-04-01T00:00:00Z")?;

        assert!(claim_job_by_id(&conn, "executor-test", "missing-job")?.is_none());
        assert!(claim_job_by_id(&conn, "executor-test", "completed-job")?.is_none());

        let (status, attempt_count): (String, i64) = conn.query_row(
            "SELECT status, attempt_count FROM workflow_jobs WHERE id = 'completed-job'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        assert_eq!(status, "completed");
        assert_eq!(attempt_count, 0);
        Ok(())
    }

    #[test]
    fn claim_next_job_for_kinds_ignores_other_lanes_and_preserves_order() -> Result<()> {
        let conn = workflow_jobs_conn()?;
        insert_job_with_kind(
            &conn,
            "pty-old",
            "sync_source",
            "pending",
            "2026-04-01T00:00:00Z",
            None,
        )?;
        insert_job_with_kind(
            &conn,
            "coverage-new",
            "commercial_sweep",
            "pending",
            "2026-04-03T00:00:00Z",
            None,
        )?;
        insert_job_with_kind(
            &conn,
            "coverage-overdue",
            "validation_cross_model_audit",
            "pending",
            "2026-04-04T00:00:00Z",
            Some("2026-03-31T00:00:00Z"),
        )?;

        let coverage_kinds = [
            "commercial_sweep",
            "validation_cross_model_audit",
            "validation_calibration_readiness_sweep",
        ];
        let claimed = claim_next_job_for_kinds(&conn, "coverage-executor", &coverage_kinds)?
            .ok_or_else(|| anyhow::anyhow!("coverage job should be claimable"))?;
        assert_eq!(claimed.id, "coverage-overdue");
        assert_eq!(claimed.kind, "validation_cross_model_audit");

        let claimed = claim_next_job_for_kinds(&conn, "coverage-executor", &coverage_kinds)?
            .ok_or_else(|| anyhow::anyhow!("second coverage job should be claimable"))?;
        assert_eq!(claimed.id, "coverage-new");
        assert_eq!(claimed.kind, "commercial_sweep");

        let pty_status: String = conn.query_row(
            "SELECT status FROM workflow_jobs WHERE id = 'pty-old'",
            [],
            |row| row.get(0),
        )?;
        assert_eq!(pty_status, "pending");

        let pty_kinds = ["sync_source", "source_health", "auth_refresh"];
        let claimed = claim_next_job_for_kinds(&conn, "pty-executor", &pty_kinds)?
            .ok_or_else(|| anyhow::anyhow!("pty job should be claimable"))?;
        assert_eq!(claimed.id, "pty-old");
        assert_eq!(claimed.kind, "sync_source");
        Ok(())
    }

    #[test]
    fn claim_next_job_for_kinds_returns_none_when_lane_has_no_matching_jobs() -> Result<()> {
        let conn = workflow_jobs_conn()?;
        insert_job_with_kind(
            &conn,
            "coverage-only",
            "commercial_sweep",
            "pending",
            "2026-04-01T00:00:00Z",
            None,
        )?;

        let pty_kinds = ["sync_source", "source_health", "auth_refresh"];
        assert!(claim_next_job_for_kinds(&conn, "pty-executor", &pty_kinds)?.is_none());

        let status: String = conn.query_row(
            "SELECT status FROM workflow_jobs WHERE id = 'coverage-only'",
            [],
            |row| row.get(0),
        )?;
        assert_eq!(status, "pending");
        Ok(())
    }

    #[test]
    fn claim_next_job_for_kinds_recycles_expired_lease_before_lane_claim() -> Result<()> {
        let conn = workflow_jobs_conn()?;
        insert_running_job_with_kind(
            &conn,
            "expired-sync-source",
            "sync_source",
            "stale-executor",
            "2026-04-01T00:00:00Z",
            1,
            "2026-04-01T00:00:00Z",
        )?;
        insert_job_with_kind(
            &conn,
            "newer-sync-source",
            "sync_source",
            "pending",
            "2026-04-03T00:00:00Z",
            None,
        )?;

        let pty_kinds = ["sync_source", "source_health", "auth_refresh"];
        let reclaimed = claim_next_job_for_kinds(&conn, "pty-executor", &pty_kinds)?
            .ok_or_else(|| anyhow::anyhow!("expired lane job should be claimable"))?;

        assert_eq!(reclaimed.id, "expired-sync-source");
        let (status, lease_owner, attempt_count): (String, Option<String>, i64) = conn.query_row(
            "\
                SELECT status, lease_owner, attempt_count
                FROM workflow_jobs
                WHERE id = 'expired-sync-source'
                ",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )?;
        assert_eq!(status, "running");
        assert_eq!(lease_owner.as_deref(), Some("pty-executor"));
        assert_eq!(attempt_count, 2);
        Ok(())
    }
}
