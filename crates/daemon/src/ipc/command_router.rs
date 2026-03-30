#![forbid(unsafe_code)]

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::ipc::handlers::control::{
    ControlHandlerContext, ControlHandlerError, get_daemon_info, restart_daemon,
};
use crate::ipc::handlers::decisions::{
    CaptureDecisionParams, DecisionsHandlerContext, DecisionsHandlerError, capture_decision,
};
use crate::ipc::handlers::errors::{
    ErrorsHandlerContext, ErrorsHandlerError, RerunFromErrorParams, ResolveErrorParams,
    rerun_from_error, resolve_error,
};
use crate::ipc::handlers::jobs::{
    JobsHandlerContext, JobsHandlerError, RetryJobParams, TriggerEnrichmentParams,
    TriggerIngestionParams, retry_job, trigger_enrichment, trigger_ingestion,
};
use crate::ipc::handlers::sessions::{
    ResizeSessionParams, SendSessionInputParams, SessionHandlerError, SessionsHandlerContext,
    StopSessionParams, resize_session, send_session_input, stop_session,
};
use crate::ipc::handshake::OperationalStatusSnapshot;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BridgeError {
    pub code: String,
    pub message: String,
    pub retryable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Value>,
}

impl BridgeError {
    pub fn invalid_params(message: impl Into<String>) -> Self {
        Self {
            code: "invalid_params".to_owned(),
            message: message.into(),
            retryable: false,
            details: None,
        }
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self {
            code: "not_found".to_owned(),
            message: message.into(),
            retryable: false,
            details: None,
        }
    }

    pub fn busy(message: impl Into<String>) -> Self {
        Self {
            code: "busy".to_owned(),
            message: message.into(),
            retryable: true,
            details: None,
        }
    }
}

pub enum DispatchOutcome {
    Response(Value),
    DisconnectAfterResponse(Value),
}

pub struct CommandRouter<'a> {
    control: ControlHandlerContext<'a>,
    jobs: JobsHandlerContext<'a>,
    operational_status: OperationalStatusSnapshot,
}

impl<'a> CommandRouter<'a> {
    pub const fn new(
        control: ControlHandlerContext<'a>,
        jobs: JobsHandlerContext<'a>,
        operational_status: OperationalStatusSnapshot,
    ) -> Self {
        Self {
            control,
            jobs,
            operational_status,
        }
    }

    #[allow(clippy::too_many_lines)]
    #[allow(clippy::too_many_lines)]
    pub fn dispatch(
        &mut self,
        method: &str,
        params: Value,
        idempotency_key: Option<&str>,
    ) -> std::result::Result<DispatchOutcome, BridgeError> {
        match method {
            "get_daemon_info" => {
                let _: EmptyParams = parse_params(params)?;
                let snapshot = get_daemon_info(&self.control, self.jobs.state_db)
                    .map_err(|error| BridgeError::busy(error.to_string()))?;
                Ok(DispatchOutcome::Response(to_value(snapshot)?))
            }
            "get_cron_status" => {
                let _: EmptyParams = parse_params(params)?;
                Ok(DispatchOutcome::Response(to_value(
                    self.operational_status.cron.clone(),
                )?))
            }
            "get_source_health_status" => {
                let params: SourceHealthStatusParams = parse_params(params)?;
                let payload = if let Some(source_id) = params.source_id {
                    let sources = self
                        .operational_status
                        .source_health
                        .sources
                        .iter()
                        .filter(|entry| entry.source_id == source_id)
                        .cloned()
                        .collect::<Vec<_>>();
                    serde_json::json!({ "sources": sources })
                } else {
                    to_value(self.operational_status.source_health.clone())?
                };
                Ok(DispatchOutcome::Response(payload))
            }
            "get_validation_status" => {
                let _: EmptyParams = parse_params(params)?;
                Ok(DispatchOutcome::Response(to_value(
                    self.operational_status.validation.clone(),
                )?))
            }
            "get_resource_arbiter_status" => {
                let _: EmptyParams = parse_params(params)?;
                Ok(DispatchOutcome::Response(to_value(
                    self.operational_status.resource_arbiter.clone(),
                )?))
            }
            "restart_daemon" => {
                let idempotency_key = required_idempotency_key(idempotency_key, method)?;
                let _: EmptyParams = parse_params(params)?;
                let ack = restart_daemon(&self.control, self.jobs.state_db, idempotency_key)
                    .map_err(map_control_error)?;
                Ok(DispatchOutcome::DisconnectAfterResponse(to_value(ack)?))
            }
            "trigger_ingestion" => {
                let idempotency_key = required_idempotency_key(idempotency_key, method)?;
                let params: TriggerIngestionParams = parse_params(params)?;
                let ack = trigger_ingestion(&mut self.jobs, &params, idempotency_key)
                    .map_err(map_jobs_error)?;
                Ok(DispatchOutcome::Response(to_value(ack)?))
            }
            "trigger_enrichment" => {
                let idempotency_key = required_idempotency_key(idempotency_key, method)?;
                let params: TriggerEnrichmentParams = parse_params(params)?;
                let ack = trigger_enrichment(&mut self.jobs, &params, idempotency_key)
                    .map_err(map_jobs_error)?;
                Ok(DispatchOutcome::Response(to_value(ack)?))
            }
            "capture_decision" => {
                let idempotency_key = required_idempotency_key(idempotency_key, method)?;
                let params: CaptureDecisionParams = parse_params(params)?;
                let mut decisions = DecisionsHandlerContext {
                    state_db: self.jobs.state_db,
                };
                let ack = capture_decision(&mut decisions, &params, idempotency_key)
                    .map_err(map_decisions_error)?;
                Ok(DispatchOutcome::Response(to_value(ack)?))
            }
            "resolve_error" => {
                let idempotency_key = required_idempotency_key(idempotency_key, method)?;
                let params: ResolveErrorParams = parse_params(params)?;
                let mut errors = ErrorsHandlerContext {
                    state_db: self.jobs.state_db,
                };
                let ack = resolve_error(&mut errors, &params, idempotency_key)
                    .map_err(map_errors_error)?;
                Ok(DispatchOutcome::Response(to_value(ack)?))
            }
            "retry_job" => {
                let idempotency_key = required_idempotency_key(idempotency_key, method)?;
                let params: RetryJobParams = parse_params(params)?;
                let ack =
                    retry_job(&mut self.jobs, &params, idempotency_key).map_err(map_jobs_error)?;
                Ok(DispatchOutcome::Response(to_value(ack)?))
            }
            "stop_session" => {
                let _: &str = required_idempotency_key(idempotency_key, method)?;
                let params: StopSessionParams = parse_params(params)?;
                let mut sessions = SessionsHandlerContext {
                    state_db: self.jobs.state_db,
                };
                let ack = stop_session(&mut sessions, &params).map_err(map_session_error)?;
                Ok(DispatchOutcome::Response(to_value(ack)?))
            }
            "send_session_input" => {
                let _: &str = required_idempotency_key(idempotency_key, method)?;
                let params: SendSessionInputParams = parse_params(params)?;
                let sessions = SessionsHandlerContext {
                    state_db: self.jobs.state_db,
                };
                let ack = send_session_input(&sessions, &params).map_err(map_session_error)?;
                Ok(DispatchOutcome::Response(to_value(ack)?))
            }
            "resize_session" => {
                let _: &str = required_idempotency_key(idempotency_key, method)?;
                let params: ResizeSessionParams = parse_params(params)?;
                let sessions = SessionsHandlerContext {
                    state_db: self.jobs.state_db,
                };
                let ack = resize_session(&sessions, &params).map_err(map_session_error)?;
                Ok(DispatchOutcome::Response(to_value(ack)?))
            }
            "rerun_from_error" => {
                let idempotency_key = required_idempotency_key(idempotency_key, method)?;
                let params: RerunFromErrorParams = parse_params(params)?;
                let mut errors = ErrorsHandlerContext {
                    state_db: self.jobs.state_db,
                };
                let ack = rerun_from_error(&mut errors, &params, idempotency_key)
                    .map_err(map_errors_error)?;
                Ok(DispatchOutcome::Response(to_value(ack)?))
            }
            _ => Err(BridgeError::not_found(format!(
                "unsupported command method `{method}`"
            ))),
        }
    }
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct EmptyParams {}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceHealthStatusParams {
    source_id: Option<String>,
}

fn parse_params<T: DeserializeOwned>(params: Value) -> std::result::Result<T, BridgeError> {
    serde_json::from_value(params).map_err(|error| BridgeError::invalid_params(error.to_string()))
}

fn to_value<T: Serialize>(value: T) -> std::result::Result<Value, BridgeError> {
    serde_json::to_value(value).map_err(|error| BridgeError::busy(error.to_string()))
}

fn required_idempotency_key<'a>(
    idempotency_key: Option<&'a str>,
    method: &str,
) -> std::result::Result<&'a str, BridgeError> {
    idempotency_key
        .ok_or_else(|| BridgeError::invalid_params(format!("{method} requires idempotencyKey")))
}

fn map_session_error(error: SessionHandlerError) -> BridgeError {
    BridgeError {
        code: error.code,
        message: error.message,
        retryable: error.retryable,
        details: None,
    }
}

fn map_jobs_error(error: JobsHandlerError) -> BridgeError {
    BridgeError {
        code: error.code,
        message: error.message,
        retryable: error.retryable,
        details: None,
    }
}

fn map_decisions_error(error: DecisionsHandlerError) -> BridgeError {
    BridgeError {
        code: error.code,
        message: error.message,
        retryable: error.retryable,
        details: None,
    }
}

fn map_errors_error(error: ErrorsHandlerError) -> BridgeError {
    BridgeError {
        code: error.code,
        message: error.message,
        retryable: error.retryable,
        details: None,
    }
}

fn map_control_error(error: ControlHandlerError) -> BridgeError {
    BridgeError {
        code: error.code,
        message: error.message,
        retryable: error.retryable,
        details: None,
    }
}
