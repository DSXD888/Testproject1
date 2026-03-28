#![forbid(unsafe_code)]

use anyhow::{Context, Result, ensure};
use rusqlite::{OptionalExtension, params};
use serde::Deserialize;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::db::sqlite::StateDb;
use crate::jobs::runner::{EnqueueJobRequest, JobRunner, MutationAck};

const EXECUTOR_ID: &str = "local-daemon";

pub struct JobsHandlerContext<'a> {
    pub state_db: &'a mut StateDb,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TriggerIngestionParams {
    pub source_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TriggerEnrichmentParams {
    pub process_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RetryJobParams {
    pub job_id: String,
}

pub fn trigger_ingestion(
    context: &mut JobsHandlerContext<'_>,
    params: &TriggerIngestionParams,
    idempotency_key: &str,
) -> Result<MutationAck> {
    let source_kind = source_kind(context.state_db, &params.source_id)?
        .ok_or_else(|| anyhow::anyhow!("unknown source `{}`", params.source_id))?;
    ensure!(
        source_kind == "procurement",
        "source `{}` is not ingestible",
        params.source_id
    );

    JobRunner::new(EXECUTOR_ID).enqueue_job(
        context.state_db,
        &EnqueueJobRequest {
            job_id: job_id("job"),
            kind: "sync_source".to_owned(),
            process_id: None,
            notice_id: None,
            scope_kind: Some("source".to_owned()),
            scope_id: Some(params.source_id.clone()),
            parent_job_id: None,
            continuation_chain_id: None,
            validation_required: false,
            input_payload: json!({
                "source_id": params.source_id,
                "trigger": "desktop_command",
            }),
            idempotency_key: idempotency_key.to_owned(),
        },
    )
}

pub fn trigger_enrichment(
    context: &mut JobsHandlerContext<'_>,
    params: &TriggerEnrichmentParams,
    idempotency_key: &str,
) -> Result<MutationAck> {
    ensure!(
        process_exists(context.state_db, &params.process_id)?,
        "unknown process `{}`",
        params.process_id
    );

    JobRunner::new(EXECUTOR_ID).enqueue_job(
        context.state_db,
        &EnqueueJobRequest {
            job_id: job_id("job"),
            kind: "enrich_process".to_owned(),
            process_id: Some(params.process_id.clone()),
            notice_id: None,
            scope_kind: Some("process".to_owned()),
            scope_id: Some(params.process_id.clone()),
            parent_job_id: None,
            continuation_chain_id: None,
            validation_required: false,
            input_payload: json!({
                "process_id": params.process_id,
                "trigger": "desktop_command",
            }),
            idempotency_key: idempotency_key.to_owned(),
        },
    )
}

pub fn retry_job(
    context: &mut JobsHandlerContext<'_>,
    params: &RetryJobParams,
    idempotency_key: &str,
) -> Result<MutationAck> {
    let row = load_retryable_job(context.state_db, &params.job_id)?
        .ok_or_else(|| anyhow::anyhow!("unknown or non-retryable job `{}`", params.job_id))?;

    JobRunner::new(EXECUTOR_ID).enqueue_job(
        context.state_db,
        &EnqueueJobRequest {
            job_id: job_id("job"),
            kind: row.kind,
            process_id: row.process_id,
            notice_id: row.notice_id,
            scope_kind: row.scope_kind,
            scope_id: row.scope_id,
            parent_job_id: Some(params.job_id.clone()),
            continuation_chain_id: row.continuation_chain_id,
            validation_required: row.validation_required,
            input_payload: row.input_payload,
            idempotency_key: idempotency_key.to_owned(),
        },
    )
}

fn source_kind(state_db: &StateDb, source_id: &str) -> Result<Option<String>> {
    state_db
        .connection()
        .query_row(
            "SELECT kind FROM sources WHERE id = ?1 AND enabled = 1",
            params![source_id],
            |row| row.get(0),
        )
        .optional()
        .context("loading source kind")
}

fn process_exists(state_db: &StateDb, process_id: &str) -> Result<bool> {
    let count: i64 = state_db.connection().query_row(
        "SELECT COUNT(*) FROM procurement_processes WHERE id = ?1",
        params![process_id],
        |row| row.get(0),
    )?;
    Ok(count == 1)
}

struct RetryableJobRow {
    kind: String,
    process_id: Option<String>,
    notice_id: Option<String>,
    scope_kind: Option<String>,
    scope_id: Option<String>,
    continuation_chain_id: Option<String>,
    validation_required: bool,
    input_payload: Value,
}

fn load_retryable_job(state_db: &StateDb, job_id: &str) -> Result<Option<RetryableJobRow>> {
    state_db
        .connection()
        .query_row(
            "\
            SELECT kind, process_id, notice_id, scope_kind, scope_id, continuation_chain_id,
                   validation_required, input_payload
            FROM workflow_jobs
            WHERE id = ?1 AND status IN ('failed', 'obsolete', 'retry')
            ",
            params![job_id],
            |row| {
                let raw_payload: String = row.get(7)?;
                let input_payload =
                    serde_json::from_str::<Value>(&raw_payload).map_err(|error| {
                        rusqlite::Error::FromSqlConversionFailure(
                            raw_payload.len(),
                            rusqlite::types::Type::Text,
                            Box::new(error),
                        )
                    })?;
                Ok(RetryableJobRow {
                    kind: row.get(0)?,
                    process_id: row.get(1)?,
                    notice_id: row.get(2)?,
                    scope_kind: row.get(3)?,
                    scope_id: row.get(4)?,
                    continuation_chain_id: row.get(5)?,
                    validation_required: row.get::<_, i64>(6)? != 0,
                    input_payload,
                })
            },
        )
        .optional()
        .context("loading retryable job")
}

fn job_id(prefix: &str) -> String {
    format!("{prefix}-{}", Uuid::new_v4())
}
