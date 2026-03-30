#![forbid(unsafe_code)]

use anyhow::{Context, Result, ensure};
use chrono::{DateTime, TimeDelta, Utc};
use rusqlite::{Connection, OptionalExtension, Transaction, TransactionBehavior, params};
use serde_json::Value;

use super::ClaimedJob;

const DEFAULT_WORKFLOW_JOB_LEASE_SECONDS: i64 = 300;

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

fn lease_deadline(now: DateTime<Utc>) -> DateTime<Utc> {
    now + TimeDelta::seconds(DEFAULT_WORKFLOW_JOB_LEASE_SECONDS)
}
