#![forbid(unsafe_code)]
#![allow(clippy::duplicate_mod)]
#![allow(dead_code, unused_imports, clippy::needless_pass_by_ref_mut)]

use anyhow::{Result, ensure};
use serde_json::{Value, json};

use super::telemetry::OperationalStatusSnapshot;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BridgeError {
    pub code: String,
    pub message: String,
    pub retryable: bool,
    pub details: Option<Value>,
}

impl BridgeError {
    #[must_use]
    pub fn invalid_params(message: impl Into<String>) -> Self {
        Self {
            code: "invalid_params".to_owned(),
            message: message.into(),
            retryable: false,
            details: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct OperationalStatusHarness {
    commit_seq: i64,
    operational_status: OperationalStatusSnapshot,
}

impl OperationalStatusHarness {
    #[must_use]
    pub const fn new(commit_seq: i64, operational_status: OperationalStatusSnapshot) -> Self {
        Self {
            commit_seq,
            operational_status,
        }
    }

    pub fn dispatch(
        &mut self,
        method: &str,
        params: Value,
    ) -> std::result::Result<Value, BridgeError> {
        match method {
            "get_cron_status" => {
                require_empty_params(params, method)?;
                serde_json::to_value(self.operational_status.cron.clone())
                    .map_err(|error| BridgeError::invalid_params(error.to_string()))
            }
            "get_source_health_status" => {
                let source_id = parse_source_health_params(params)?;
                if let Some(source_id) = source_id {
                    let sources = self
                        .operational_status
                        .source_health
                        .sources
                        .iter()
                        .filter(|entry| entry.source_id == source_id)
                        .cloned()
                        .collect::<Vec<_>>();
                    Ok(json!({ "sources": sources }))
                } else {
                    serde_json::to_value(self.operational_status.source_health.clone())
                        .map_err(|error| BridgeError::invalid_params(error.to_string()))
                }
            }
            "get_validation_status" => {
                require_empty_params(params, method)?;
                serde_json::to_value(self.operational_status.validation.clone())
                    .map_err(|error| BridgeError::invalid_params(error.to_string()))
            }
            "get_resource_arbiter_status" => {
                require_empty_params(params, method)?;
                serde_json::to_value(self.operational_status.resource_arbiter.clone())
                    .map_err(|error| BridgeError::invalid_params(error.to_string()))
            }
            _ => Err(BridgeError::invalid_params(format!(
                "unsupported status method `{method}`"
            ))),
        }
    }

    pub fn assert_read_only_status_call(&mut self, method: &str) -> Result<()> {
        let commit_seq = self.commit_seq;
        let _ = self
            .dispatch(method, json!({}))
            .map_err(|error| anyhow::anyhow!(error.message))?;
        ensure!(
            self.commit_seq == commit_seq,
            "status method `{method}` must not mutate commit_seq"
        );
        Ok(())
    }

    pub fn assert_invalid_params(&mut self, method: &str, params: Value) -> Result<()> {
        let error = self
            .dispatch(method, params)
            .expect_err("invalid params should fail");
        ensure!(error.code == "invalid_params");
        Ok(())
    }
}

fn require_empty_params(params: Value, method: &str) -> std::result::Result<(), BridgeError> {
    match params {
        Value::Object(map) if map.is_empty() => Ok(()),
        _ => Err(BridgeError::invalid_params(format!(
            "{method} does not accept params"
        ))),
    }
}

fn parse_source_health_params(params: Value) -> std::result::Result<Option<String>, BridgeError> {
    match params {
        Value::Object(map) if map.is_empty() => Ok(None),
        Value::Object(mut map) if map.len() == 1 && map.contains_key("source_id") => map
            .remove("source_id")
            .and_then(|value| value.as_str().map(str::to_owned))
            .ok_or_else(|| BridgeError::invalid_params("source_id must be a string"))
            .map(Some),
        _ => Err(BridgeError::invalid_params(
            "get_source_health_status accepts only an optional source_id filter",
        )),
    }
}
