#![forbid(unsafe_code)]

//! Stage 14 Phase 2 — IPC handlers for operator clarification + decision
//! withdrawal. Mirrors the CLI helpers in `main.rs` (`capture_operator_*`,
//! `withdraw_operator_*`) but with typed-param entry points so the desktop
//! Tauri shell can invoke them via the Unix-socket bridge instead of
//! spawning the daemon as a subprocess.

use anyhow::Context;
use rusqlite::params;
use serde::Deserialize;
use serde_json::Value;
use thiserror::Error;

#[cfg(not(test))]
use crate::db::sqlite::StateDb;
use crate::jobs::runner::{MutationAck, allocate_stage_run_id};
#[cfg(test)]
use crate::support::db::StateDb;
use crate::{
    DEFAULT_LOCAL_OPERATOR_ID, OPERATOR_DECISION_STAGE_NAME, OperatorOverrideReason,
    lookup_intent_by_idempotency_key, persist_stage, read_latest_manifest_basis_hash, sha256_hex,
    validate_operator_override_payload,
};

pub struct ClarificationsHandlerContext<'a> {
    pub state_db: &'a mut StateDb,
}

#[derive(Debug, Clone, Error)]
#[error("{code}: {message}")]
pub struct ClarificationsHandlerError {
    pub code: String,
    pub message: String,
    pub retryable: bool,
}

impl ClarificationsHandlerError {
    fn invalid_params(message: impl Into<String>) -> Self {
        Self {
            code: "invalid_params".to_owned(),
            message: message.into(),
            retryable: false,
        }
    }

    fn not_found(message: impl Into<String>) -> Self {
        Self {
            code: "not_found".to_owned(),
            message: message.into(),
            retryable: false,
        }
    }

    fn busy(message: impl Into<String>) -> Self {
        Self {
            code: "busy".to_owned(),
            message: message.into(),
            retryable: true,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaptureClarificationParams {
    pub process_id: String,
    pub candidate_id: String,
    pub reason_code: String,
    pub payload: Value,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WithdrawClarificationParams {
    pub process_id: String,
    pub candidate_id: String,
    pub reason_code: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WithdrawDecisionParams {
    pub idempotency_key: String,
}

#[allow(clippy::too_many_lines)]
pub fn capture_clarification(
    context: &mut ClarificationsHandlerContext<'_>,
    params: &CaptureClarificationParams,
    _idempotency_key: &str,
) -> std::result::Result<MutationAck, ClarificationsHandlerError> {
    let reason = OperatorOverrideReason::parse(&params.reason_code)
        .map_err(|error| ClarificationsHandlerError::invalid_params(error.to_string()))?;
    validate_operator_override_payload(reason, &params.payload)
        .map_err(|error| ClarificationsHandlerError::invalid_params(error.to_string()))?;
    let payload_canonical = serde_json::to_string(&params.payload).map_err(|error| {
        ClarificationsHandlerError::invalid_params(format!("serializing payload: {error}"))
    })?;

    let process_exists: i64 = context
        .state_db
        .connection()
        .query_row(
            "SELECT COUNT(*) FROM procurement_processes WHERE id = ?1",
            params![params.process_id],
            |row| row.get(0),
        )
        .map_err(|error| ClarificationsHandlerError::busy(error.to_string()))?;
    if process_exists != 1 {
        return Err(ClarificationsHandlerError::not_found(format!(
            "no procurement_processes row for process_id `{}`",
            params.process_id
        )));
    }
    let candidate_exists: i64 = context
        .state_db
        .connection()
        .query_row(
            "SELECT COUNT(*) FROM product_candidates WHERE id = ?1 AND process_id = ?2",
            params![params.candidate_id, params.process_id],
            |row| row.get(0),
        )
        .map_err(|error| ClarificationsHandlerError::busy(error.to_string()))?;
    if candidate_exists != 1 {
        return Err(ClarificationsHandlerError::not_found(format!(
            "no product_candidates row with id `{}` belonging to process `{}`",
            params.candidate_id, params.process_id
        )));
    }

    let basis_hash =
        read_latest_manifest_basis_hash(context.state_db.connection(), &params.process_id)
            .map_err(|error| ClarificationsHandlerError::busy(error.to_string()))?;
    let stage_run_id = allocate_stage_run_id();
    let override_id = format!(
        "override-{}-{}-{}-{}",
        sha256_hex(params.process_id.as_bytes())
            .chars()
            .take(8)
            .collect::<String>(),
        sha256_hex(params.candidate_id.as_bytes())
            .chars()
            .take(8)
            .collect::<String>(),
        reason.as_str(),
        sha256_hex(stage_run_id.as_bytes())
            .chars()
            .take(12)
            .collect::<String>()
    );

    let process_id_owned = params.process_id.clone();
    let candidate_id_owned = params.candidate_id.clone();
    let reason_code_str = reason.as_str().to_owned();
    let note = params.note.clone();
    let stage_run_id_for_query = stage_run_id;
    let basis_hash_for_query = basis_hash;
    let payload_for_query = payload_canonical;

    let ((), visible_after_seq) = context
        .state_db
        .with_write_transaction(move |tx| {
            tx.execute(
                "INSERT INTO operator_input_overrides (
                   id, process_id, product_candidate_id, reason_code, payload_json, notes,
                   basis_hash_when_provided, owner_id, row_version, stage_run_id
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 1, ?9)",
                params![
                    override_id,
                    process_id_owned,
                    candidate_id_owned,
                    reason_code_str,
                    payload_for_query,
                    note,
                    basis_hash_for_query,
                    DEFAULT_LOCAL_OPERATOR_ID,
                    stage_run_id_for_query,
                ],
            )
            .context("inserting operator_input_overrides row")?;
            tx.execute(
                "UPDATE operator_input_overrides
                 SET superseded_by_override_id = ?1,
                     row_version = row_version + 1,
                     stage_run_id = ?2,
                     updated_at = CURRENT_TIMESTAMP
                 WHERE process_id = ?3
                   AND product_candidate_id = ?4
                   AND reason_code = ?5
                   AND id <> ?1
                   AND withdrawn_at IS NULL
                   AND superseded_by_override_id IS NULL",
                params![
                    override_id,
                    stage_run_id_for_query,
                    process_id_owned,
                    candidate_id_owned,
                    reason_code_str,
                ],
            )
            .context("superseding existing operator_input_overrides rows")?;

            persist_stage(
                tx,
                &process_id_owned,
                OPERATOR_DECISION_STAGE_NAME,
                "process",
                &process_id_owned,
                &stage_run_id_for_query,
            )?;
            Ok(())
        })
        .map_err(|error| ClarificationsHandlerError::busy(error.to_string()))?;

    Ok(MutationAck {
        accepted: true,
        visible_after_seq,
        continuation_job_id: None,
    })
}

pub fn withdraw_clarification(
    context: &mut ClarificationsHandlerContext<'_>,
    params: &WithdrawClarificationParams,
    _idempotency_key: &str,
) -> std::result::Result<MutationAck, ClarificationsHandlerError> {
    let reason = OperatorOverrideReason::parse(&params.reason_code)
        .map_err(|error| ClarificationsHandlerError::invalid_params(error.to_string()))?;
    let stage_run_id = allocate_stage_run_id();

    let process_id_owned = params.process_id.clone();
    let candidate_id_owned = params.candidate_id.clone();
    let reason_code_str = reason.as_str().to_owned();
    let stage_run_id_for_query = stage_run_id;

    let (rows_affected, visible_after_seq) = context
        .state_db
        .with_write_transaction(move |tx| {
            let updated = tx
                .execute(
                    "UPDATE operator_input_overrides
                     SET withdrawn_at = CURRENT_TIMESTAMP,
                         updated_at = CURRENT_TIMESTAMP,
                         stage_run_id = ?1
                     WHERE process_id = ?2
                       AND product_candidate_id = ?3
                       AND reason_code = ?4
                       AND withdrawn_at IS NULL",
                    params![
                        stage_run_id_for_query,
                        process_id_owned,
                        candidate_id_owned,
                        reason_code_str,
                    ],
                )
                .context("withdrawing operator_input_overrides row")?;
            if updated > 0 {
                persist_stage(
                    tx,
                    &process_id_owned,
                    OPERATOR_DECISION_STAGE_NAME,
                    "process",
                    &process_id_owned,
                    &stage_run_id_for_query,
                )?;
            }
            Ok(updated)
        })
        .map_err(|error| ClarificationsHandlerError::busy(error.to_string()))?;

    if rows_affected == 0 {
        return Err(ClarificationsHandlerError::not_found(format!(
            "no active operator_input_overrides row for process `{}`, candidate `{}`, reason `{}`",
            params.process_id,
            params.candidate_id,
            reason.as_str()
        )));
    }

    Ok(MutationAck {
        accepted: true,
        visible_after_seq,
        continuation_job_id: None,
    })
}

pub fn withdraw_decision(
    context: &mut ClarificationsHandlerContext<'_>,
    params: &WithdrawDecisionParams,
    _idempotency_key: &str,
) -> std::result::Result<MutationAck, ClarificationsHandlerError> {
    let existing =
        lookup_intent_by_idempotency_key(context.state_db.connection(), &params.idempotency_key)
            .map_err(|error| ClarificationsHandlerError::busy(error.to_string()))?
            .ok_or_else(|| {
                ClarificationsHandlerError::not_found(format!(
                    "no pending_operator_intents row with idempotency_key `{}`",
                    params.idempotency_key
                ))
            })?;

    let stage_run_id = allocate_stage_run_id();
    let intent_id = existing.id.clone();
    let process_id = existing.process_id;
    let stage_run_id_for_query = stage_run_id;

    let ((), visible_after_seq) = context
        .state_db
        .with_write_transaction(move |tx| {
            tx.execute(
                "UPDATE pending_operator_intents
                 SET status = 'withdrawn',
                     last_attempt_at = CURRENT_TIMESTAMP,
                     last_error = NULL
                 WHERE id = ?1",
                params![intent_id],
            )
            .context("withdrawing pending_operator_intents row")?;
            persist_stage(
                tx,
                &process_id,
                OPERATOR_DECISION_STAGE_NAME,
                "process",
                &process_id,
                &stage_run_id_for_query,
            )?;
            Ok(())
        })
        .map_err(|error| ClarificationsHandlerError::busy(error.to_string()))?;

    Ok(MutationAck {
        accepted: true,
        visible_after_seq,
        continuation_job_id: None,
    })
}
