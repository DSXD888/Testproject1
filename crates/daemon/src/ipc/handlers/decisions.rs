#![forbid(unsafe_code)]

use anyhow::{Context, Result};
use rusqlite::{OptionalExtension, Transaction, params};
use serde::Deserialize;
use thiserror::Error;

#[cfg(not(test))]
use crate::db::sqlite::StateDb;
use crate::jobs::runner::MutationAck;
#[cfg(test)]
use crate::support::db::StateDb;

pub struct DecisionsHandlerContext<'a> {
    pub state_db: &'a mut StateDb,
}

#[derive(Debug, Clone, Error)]
#[error("{code}: {message}")]
pub struct DecisionsHandlerError {
    pub code: String,
    pub message: String,
    pub retryable: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionDisposition {
    Approve,
    Reject,
    Watch,
}

impl DecisionDisposition {
    const fn as_str(&self) -> &'static str {
        match self {
            Self::Approve => "approve",
            Self::Reject => "reject",
            Self::Watch => "watch",
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaptureDecisionParams {
    pub process_id: String,
    pub disposition: DecisionDisposition,
    pub reviewed_basis_hash: String,
    pub reviewed_manifest_hash: String,
    pub reviewed_at_stage_run_id: String,
    #[serde(default)]
    pub decision_note: Option<String>,
}

pub fn capture_decision(
    context: &mut DecisionsHandlerContext<'_>,
    params: &CaptureDecisionParams,
    idempotency_key: &str,
) -> std::result::Result<MutationAck, DecisionsHandlerError> {
    if let Some(visible_after_seq) = existing_receipt(context.state_db, idempotency_key)? {
        return Ok(MutationAck {
            accepted: true,
            visible_after_seq,
            continuation_job_id: None,
        });
    }

    let process_id = resolve_process_id(context.state_db, &params.process_id)?;

    let ((), visible_after_seq) = context
        .state_db
        .with_write_transaction(|tx| {
            let intent_id = format!("intent-{idempotency_key}");
            let previous_intent_id = latest_unsuperseded_intent_id(tx, &process_id)?;
            let reviewed_at = chrono::Utc::now().to_rfc3339();
            let predicted_visible_after_seq = current_commit_seq(tx)? + 1;

            tx.execute(
                "\
                INSERT INTO pending_operator_intents (
                  id, process_id, desired_disposition, reviewed_basis_hash, reviewed_manifest_hash,
                  reviewed_at_stage_run_id, reviewed_at, base_remote_revision, idempotency_key, status,
                  superseded_by_intent_id, queued_at, decision_note, last_attempt_at, last_error
                ) VALUES (
                  ?1, ?2, ?3, ?4, ?5,
                  ?6, ?7, 0, ?8, 'queued',
                  NULL, ?7, ?9, NULL, NULL
                )
                ",
                params![
                    intent_id,
                    process_id,
                    params.disposition.as_str(),
                    params.reviewed_basis_hash,
                    params.reviewed_manifest_hash,
                    params.reviewed_at_stage_run_id,
                    reviewed_at,
                    idempotency_key,
                    params.decision_note,
                ],
            )?;

            if let Some(previous_intent_id) = previous_intent_id {
                tx.execute(
                    "\
                    UPDATE pending_operator_intents
                    SET superseded_by_intent_id = ?1
                    WHERE id = ?2 AND superseded_by_intent_id IS NULL
                    ",
                    params![intent_id, previous_intent_id],
                )?;
            }

            tx.execute(
                "\
                INSERT INTO pending_operator_intent_receipts (
                  idempotency_key, intent_id, visible_after_seq
                ) VALUES (?1, ?2, ?3)
                ",
                params![idempotency_key, intent_id, predicted_visible_after_seq],
            )?;

            Ok(())
        })
        .map_err(map_internal_error)?;

    Ok(MutationAck {
        accepted: true,
        visible_after_seq,
        continuation_job_id: None,
    })
}

fn existing_receipt(
    state_db: &StateDb,
    idempotency_key: &str,
) -> std::result::Result<Option<i64>, DecisionsHandlerError> {
    state_db
        .connection()
        .query_row(
            "\
            SELECT visible_after_seq
            FROM pending_operator_intent_receipts
            WHERE idempotency_key = ?1
            ",
            params![idempotency_key],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .context("loading decision receipt")
        .map_err(map_internal_error)
}

fn resolve_process_id(
    state_db: &StateDb,
    requested_process_id: &str,
) -> std::result::Result<String, DecisionsHandlerError> {
    if let Some(canonical_process_id) = state_db
        .connection()
        .query_row(
            "SELECT canonical_process_id FROM process_aliases WHERE alias_process_id = ?1",
            params![requested_process_id],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .context("loading process alias")
        .map_err(map_internal_error)?
    {
        return Ok(canonical_process_id);
    }

    let exists = state_db
        .connection()
        .query_row(
            "SELECT COUNT(*) FROM procurement_processes WHERE id = ?1",
            params![requested_process_id],
            |row| row.get::<_, i64>(0),
        )
        .context("checking process existence")
        .map_err(map_internal_error)?;

    if exists == 1 {
        return Ok(requested_process_id.to_owned());
    }

    Err(DecisionsHandlerError {
        code: "not_found".to_owned(),
        message: format!("unknown process `{requested_process_id}`"),
        retryable: false,
    })
}

fn latest_unsuperseded_intent_id(
    conn: &Transaction<'_>,
    process_id: &str,
) -> Result<Option<String>> {
    conn.query_row(
        "\
        SELECT id
        FROM pending_operator_intents
        WHERE process_id = ?1 AND superseded_by_intent_id IS NULL
        ORDER BY queued_at DESC, id DESC
        LIMIT 1
        ",
        params![process_id],
        |row| row.get::<_, String>(0),
    )
    .optional()
    .context("loading latest unsuperseded decision intent")
}

fn current_commit_seq(tx: &Transaction<'_>) -> Result<i64> {
    tx.query_row(
        "SELECT commit_seq FROM commit_clock WHERE id = 1",
        [],
        |row| row.get(0),
    )
    .context("reading current commit_seq for decision write")
}

#[allow(clippy::needless_pass_by_value)]
fn map_internal_error(error: anyhow::Error) -> DecisionsHandlerError {
    DecisionsHandlerError {
        code: "busy".to_owned(),
        message: error.to_string(),
        retryable: true,
    }
}
