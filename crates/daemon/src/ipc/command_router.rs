#![forbid(unsafe_code)]

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::ipc::handlers::control::{ControlHandlerContext, get_daemon_info, restart_daemon};

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
}

impl<'a> CommandRouter<'a> {
    pub const fn new(control: ControlHandlerContext<'a>) -> Self {
        Self { control }
    }

    pub fn dispatch(
        &self,
        method: &str,
        params: Value,
        idempotency_key: Option<&str>,
    ) -> std::result::Result<DispatchOutcome, BridgeError> {
        match method {
            "get_daemon_info" => {
                let _: EmptyParams = parse_params(params)?;
                let snapshot = get_daemon_info(&self.control)
                    .map_err(|error| BridgeError::busy(error.to_string()))?;
                Ok(DispatchOutcome::Response(to_value(snapshot)?))
            }
            "restart_daemon" => {
                if idempotency_key.is_none() {
                    return Err(BridgeError::invalid_params(
                        "restart_daemon requires idempotencyKey",
                    ));
                }

                let _: EmptyParams = parse_params(params)?;
                let ack = restart_daemon(&self.control)
                    .map_err(|error| BridgeError::busy(error.to_string()))?;
                Ok(DispatchOutcome::DisconnectAfterResponse(to_value(ack)?))
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

fn parse_params<T: DeserializeOwned>(params: Value) -> std::result::Result<T, BridgeError> {
    serde_json::from_value(params).map_err(|error| BridgeError::invalid_params(error.to_string()))
}

fn to_value<T: Serialize>(value: T) -> std::result::Result<Value, BridgeError> {
    serde_json::to_value(value).map_err(|error| BridgeError::busy(error.to_string()))
}
