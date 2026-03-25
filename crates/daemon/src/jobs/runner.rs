#![forbid(unsafe_code)]

use anyhow::{Context, Result, anyhow, ensure};
use chrono::Utc;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[cfg(not(test))]
use crate::agents::contracts;
#[cfg(test)]
use crate::contracts_impl as contracts;
#[cfg(not(test))]
use crate::db::sqlite::StateDb;
#[cfg(test)]
use crate::support::db::StateDb;

use contracts::{
    StageValidationWrite, ValidationDisposition, ValidationOutcome, persist_stage_validation,
};

pub const SQLITE_WRITE_FAILED: &str = "SQLITE_WRITE_FAILED";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimedJob {
    pub id: String,
    pub kind: String,
    pub process_id: Option<String>,
    pub scope_kind: Option<String>,
    pub scope_id: Option<String>,
    pub input_payload: Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MutationAck {
    pub accepted: bool,
    pub visible_after_seq: i64,
    pub continuation_job_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistSuccessRequest {
    pub job_id: String,
    pub idempotency_key: String,
    pub session_id: String,
    pub output_payload: Value,
    pub stage: String,
    pub scope_kind: String,
    pub scope_id: String,
    pub stage_run_id: String,
    pub process_id: String,
    pub owner_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistContinuationRequest {
    pub job_id: String,
    pub idempotency_key: String,
    pub continuation_job_id: String,
    pub continuation_chain_id: String,
    pub next_job_kind: String,
    pub process_id: Option<String>,
    pub scope_kind: Option<String>,
    pub scope_id: Option<String>,
    pub resume_token: Value,
    pub owner_id: String,
}

#[derive(Debug, Clone)]
pub struct JobRunner {
    executor_id: String,
}

impl JobRunner {
    #[must_use]
    pub fn new(executor_id: impl Into<String>) -> Self {
        Self {
            executor_id: executor_id.into(),
        }
    }

    pub fn claim_next_job(&self, conn: &Connection) -> Result<Option<ClaimedJob>> {
        let mut statement = conn.prepare(
            "\
            SELECT id, kind, process_id, scope_kind, scope_id, input_payload
            FROM workflow_jobs
            WHERE status IN ('pending', 'retry')
            ORDER BY COALESCE(scheduled_for, created_at), created_at
            LIMIT 1
            ",
        )?;

        let job = statement
            .query_row([], |row| {
                let payload: String = row.get(5)?;
                let parsed = serde_json::from_str::<Value>(&payload).map_err(|error| {
                    rusqlite::Error::FromSqlConversionFailure(
                        payload.len(),
                        rusqlite::types::Type::Text,
                        Box::new(error),
                    )
                })?;
                Ok(ClaimedJob {
                    id: row.get(0)?,
                    kind: row.get(1)?,
                    process_id: row.get(2)?,
                    scope_kind: row.get(3)?,
                    scope_id: row.get(4)?,
                    input_payload: parsed,
                })
            })
            .optional()
            .context("claiming next workflow job")?;

        if let Some(job) = &job {
            conn.execute(
                "\
                UPDATE workflow_jobs
                SET status = 'running',
                    lease_owner = ?1,
                    started_at = ?2,
                    attempt_count = attempt_count + 1
                WHERE id = ?3
                ",
                params![self.executor_id, Utc::now().to_rfc3339(), job.id],
            )
            .context("marking workflow job running")?;
        }

        Ok(job)
    }

    pub fn persist_success(
        &self,
        state_db: &mut StateDb,
        request: &PersistSuccessRequest,
        outcome: &ValidationOutcome,
    ) -> Result<MutationAck> {
        if let Some(existing) = find_existing_ack(
            state_db.connection(),
            &request.job_id,
            &request.idempotency_key,
        )? {
            return Ok(existing);
        }

        ensure!(
            outcome.disposition == ValidationDisposition::Accepted,
            "persist_success requires accepted outcome"
        );

        let (predicted_visible_after_seq, visible_after_seq) =
            state_db.with_write_transaction(|tx| {
                let predicted_visible_after_seq = current_commit_seq(tx)? + 1;
                let stage_write = StageValidationWrite {
                    process_id: &request.process_id,
                    stage: &request.stage,
                    scope_kind: &request.scope_kind,
                    scope_id: &request.scope_id,
                    stage_run_id: &request.stage_run_id,
                    owner_id: &request.owner_id,
                };
                persist_stage_validation(tx, &stage_write, outcome)?;

                let stored_payload = json!({
                    "result": request.output_payload,
                    "runner_meta": {
                        "idempotency_key": request.idempotency_key,
                        "visible_after_seq": predicted_visible_after_seq,
                        "executor_id": self.executor_id,
                        "continuation_job_id": Value::Null,
                    }
                })
                .to_string();

                let updated = tx.execute(
                    "\
                    UPDATE workflow_jobs
                    SET status = 'completed',
                        completed_at = ?1,
                        output_payload = ?2
                    WHERE id = ?3
                    ",
                    params![Utc::now().to_rfc3339(), stored_payload, request.job_id],
                )?;
                ensure!(updated == 1, "workflow job missing for {}", request.job_id);
                Ok(predicted_visible_after_seq)
            })?;

        ensure!(
            predicted_visible_after_seq == visible_after_seq,
            "predicted commit fence diverged from committed fence"
        );

        Ok(MutationAck {
            accepted: true,
            visible_after_seq,
            continuation_job_id: None,
        })
    }

    pub fn persist_continuation(
        &self,
        state_db: &mut StateDb,
        request: &PersistContinuationRequest,
    ) -> Result<MutationAck> {
        if let Some(existing) = find_existing_ack(
            state_db.connection(),
            &request.job_id,
            &request.idempotency_key,
        )? {
            return Ok(existing);
        }

        let (predicted_visible_after_seq, visible_after_seq) =
            state_db.with_write_transaction(|tx| {
                let predicted_visible_after_seq = current_commit_seq(tx)? + 1;

                tx.execute(
                    "\
                    INSERT INTO workflow_jobs (
                      id, kind, process_id, scope_kind, scope_id, parent_job_id,
                      continuation_chain_id, status, input_payload
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'pending', ?8)
                    ",
                    params![
                        request.continuation_job_id,
                        request.next_job_kind,
                        request.process_id,
                        request.scope_kind,
                        request.scope_id,
                        request.job_id,
                        request.continuation_chain_id,
                        json!({"resume_token": request.resume_token}).to_string(),
                    ],
                )?;

                let stored_payload = json!({
                    "runner_meta": {
                        "idempotency_key": request.idempotency_key,
                        "visible_after_seq": predicted_visible_after_seq,
                        "executor_id": self.executor_id,
                        "continuation_job_id": request.continuation_job_id,
                    }
                })
                .to_string();

                let updated = tx.execute(
                    "\
                    UPDATE workflow_jobs
                    SET status = 'completed',
                        chain_closed = 0,
                        completed_at = ?1,
                        output_payload = ?2
                    WHERE id = ?3
                    ",
                    params![Utc::now().to_rfc3339(), stored_payload, request.job_id],
                )?;
                ensure!(updated == 1, "workflow job missing for {}", request.job_id);

                Ok(predicted_visible_after_seq)
            })?;

        ensure!(
            predicted_visible_after_seq == visible_after_seq,
            "predicted commit fence diverged from committed fence"
        );

        Ok(MutationAck {
            accepted: true,
            visible_after_seq,
            continuation_job_id: Some(request.continuation_job_id.clone()),
        })
    }
}

pub fn classify_sqlite_write_failed(error: &rusqlite::Error) -> anyhow::Error {
    anyhow!("{SQLITE_WRITE_FAILED}: {error}")
}

fn current_commit_seq(conn: &Connection) -> Result<i64> {
    conn.query_row(
        "SELECT commit_seq FROM commit_clock WHERE id = 1",
        [],
        |row| row.get(0),
    )
    .context("reading current commit_seq")
}

fn find_existing_ack(
    conn: &Connection,
    job_id: &str,
    idempotency_key: &str,
) -> Result<Option<MutationAck>> {
    let output_payload: Option<String> = conn
        .query_row(
            "SELECT output_payload FROM workflow_jobs WHERE id = ?1 AND status = 'completed'",
            params![job_id],
            |row| row.get(0),
        )
        .optional()
        .context("loading workflow job output payload")?;

    let Some(output_payload) = output_payload else {
        return Ok(None);
    };

    let parsed: Value =
        serde_json::from_str(&output_payload).context("parsing workflow output payload")?;
    let meta = parsed
        .get("runner_meta")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("completed workflow job missing runner_meta payload"))?;

    if meta.get("idempotency_key").and_then(Value::as_str) != Some(idempotency_key) {
        return Ok(None);
    }

    let visible_after_seq = meta
        .get("visible_after_seq")
        .and_then(Value::as_i64)
        .ok_or_else(|| anyhow!("runner_meta missing visible_after_seq"))?;
    let continuation_job_id = meta
        .get("continuation_job_id")
        .and_then(Value::as_str)
        .map(str::to_owned);

    Ok(Some(MutationAck {
        accepted: true,
        visible_after_seq,
        continuation_job_id,
    }))
}
