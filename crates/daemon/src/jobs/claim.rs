#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use anyhow::{Context, Result, ensure};
use chrono::{DateTime, TimeDelta, Utc};
use rusqlite::{Connection, OptionalExtension, Transaction, TransactionBehavior, params};
use serde_json::Value;

use super::ClaimedJob;

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
