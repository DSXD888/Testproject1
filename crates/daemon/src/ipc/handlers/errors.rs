#![forbid(unsafe_code)]

use chrono::Utc;
use serde::Deserialize;
use thiserror::Error;

use crate::db::sqlite::StateDb;
#[cfg(not(test))]
use crate::errors::manifest::ErrorManifestStore;
#[cfg(test)]
use crate::errors_manifest_impl::ErrorManifestStore;
use crate::jobs::runner::{EnqueueJobRequest, JobRunner, MutationAck};

const EXECUTOR_ID: &str = "local-daemon";
const RESOLVED_BY: &str = "desktop-operator";

pub struct ErrorsHandlerContext<'a> {
    pub state_db: &'a mut StateDb,
}

#[derive(Debug, Clone, Error)]
#[error("{code}: {message}")]
pub struct ErrorsHandlerError {
    pub code: String,
    pub message: String,
    pub retryable: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResolveErrorParams {
    pub error_manifest_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RerunFromErrorParams {
    pub error_manifest_id: String,
}

pub fn resolve_error(
    context: &mut ErrorsHandlerContext<'_>,
    params: &ResolveErrorParams,
    _idempotency_key: &str,
) -> std::result::Result<MutationAck, ErrorsHandlerError> {
    let record =
        ErrorManifestStore::load_record(context.state_db.connection(), &params.error_manifest_id)
            .map_err(classify_load_error)?;

    if record.resolved {
        return Ok(MutationAck {
            accepted: true,
            visible_after_seq: context
                .state_db
                .current_commit_seq()
                .map_err(map_internal_error)?,
            continuation_job_id: None,
        });
    }

    let ((), visible_after_seq) = context
        .state_db
        .with_write_transaction(|tx| {
            ErrorManifestStore::resolve(tx, &params.error_manifest_id, RESOLVED_BY, Utc::now())?;
            Ok(())
        })
        .map_err(map_internal_error)?;

    Ok(MutationAck {
        accepted: true,
        visible_after_seq,
        continuation_job_id: None,
    })
}

pub fn rerun_from_error(
    context: &mut ErrorsHandlerContext<'_>,
    params: &RerunFromErrorParams,
    idempotency_key: &str,
) -> std::result::Result<MutationAck, ErrorsHandlerError> {
    let record =
        ErrorManifestStore::load_record(context.state_db.connection(), &params.error_manifest_id)
            .map_err(classify_load_error)?;

    if record.resolved {
        return Err(ErrorsHandlerError {
            code: "conflict".to_owned(),
            message: format!(
                "error manifest `{}` is already resolved",
                params.error_manifest_id
            ),
            retryable: false,
        });
    }

    let process_id = record.process_id.ok_or_else(|| ErrorsHandlerError {
        code: "conflict".to_owned(),
        message: format!(
            "error manifest `{}` is not rerunnable because it is not tied to a process",
            params.error_manifest_id
        ),
        retryable: false,
    })?;

    let job_id = format!("rerun-error-{}", uuid::Uuid::new_v4());
    let ack = JobRunner::new(EXECUTOR_ID)
        .enqueue_job(
            context.state_db,
            &EnqueueJobRequest {
                job_id: job_id.clone(),
                kind: "rerun_from_error".to_owned(),
                process_id: Some(process_id.clone()),
                notice_id: None,
                scope_kind: Some("process".to_owned()),
                scope_id: Some(process_id),
                parent_job_id: record.job_id.clone(),
                continuation_chain_id: None,
                validation_required: false,
                input_payload: serde_json::json!({
                    "error_manifest_id": params.error_manifest_id,
                    "pipeline_stage": record.pipeline_stage,
                }),
                idempotency_key: idempotency_key.to_owned(),
            },
        )
        .map_err(map_internal_error)?;

    Ok(MutationAck {
        continuation_job_id: Some(job_id),
        ..ack
    })
}

fn classify_load_error(error: anyhow::Error) -> ErrorsHandlerError {
    let message = error.to_string();
    if message.contains("error manifest missing for") {
        return ErrorsHandlerError {
            code: "not_found".to_owned(),
            message,
            retryable: false,
        };
    }
    map_internal_error(error)
}

#[allow(clippy::needless_pass_by_value)]
fn map_internal_error(error: anyhow::Error) -> ErrorsHandlerError {
    ErrorsHandlerError {
        code: "busy".to_owned(),
        message: error.to_string(),
        retryable: true,
    }
}
