#![forbid(unsafe_code)]

#[path = "../db/checkpoints.rs"]
mod checkpoint_store;

use std::path::Path;

use anyhow::{Context, Result, anyhow, ensure};
use chrono::Utc;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

pub const ERROR_OUTPUT_CONTRACT_INVALID: &str = "AGENT_OUTPUT_CONTRACT_INVALID";
pub const ERROR_SUSPECT_OUTPUT: &str = "AGENT_SUSPECT_OUTPUT";
pub const ERROR_PARTIAL_FAILURE: &str = "AGENT_PARTIAL_FAILURE";
pub const ERROR_RUN_FAILED: &str = "AGENT_RUN_FAILED";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationDisposition {
    Accepted,
    Quarantined,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationOutcome {
    pub disposition: ValidationDisposition,
    pub error_code: Option<&'static str>,
    pub error_context_json: Option<String>,
    pub anomalies: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OqgCheckInput {
    pub expected_fill_bps: u32,
    pub observed_fill_bps: u32,
    pub duplicate_rate_bps: u32,
    pub max_duplicate_rate_bps: u32,
    pub observed_values: Vec<i64>,
    pub allowed_min_value: i64,
    pub allowed_max_value: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageValidationWrite<'a> {
    pub process_id: &'a str,
    pub stage: &'a str,
    pub scope_kind: &'a str,
    pub scope_id: &'a str,
    pub stage_run_id: &'a str,
    pub owner_id: &'a str,
}

pub fn validate_result_schema(schema: &Value, payload: &Value) -> Result<()> {
    let payload_object = payload
        .as_object()
        .ok_or_else(|| anyhow!("result payload must be a JSON object"))?;

    if let Some(required_fields) = schema.get("required").and_then(Value::as_array) {
        for field in required_fields {
            let field_name = field
                .as_str()
                .ok_or_else(|| anyhow!("required field names must be strings"))?;
            ensure!(
                payload_object.contains_key(field_name),
                "result payload missing required field `{field_name}`"
            );
        }
    }

    if let Some(properties) = schema.get("properties").and_then(Value::as_object) {
        for (name, property_schema) in properties {
            let Some(value) = payload_object.get(name) else {
                continue;
            };
            if let Some(expected_type) = property_schema.get("type").and_then(Value::as_str) {
                ensure!(
                    value_matches_type(value, expected_type),
                    "result payload field `{name}` violated type `{expected_type}`"
                );
            }
        }
    }

    Ok(())
}

pub fn validate_checkpoint_sequence(last_checkpoint_seq: Option<u32>, next_seq: u32) -> Result<()> {
    let expected = last_checkpoint_seq.map_or(1, |previous| previous.saturating_add(1));
    ensure!(
        next_seq == expected,
        "{ERROR_PARTIAL_FAILURE}: expected checkpoint {expected}, got {next_seq}"
    );
    Ok(())
}

pub fn persist_validated_checkpoint(
    conn: &Connection,
    session_id: &str,
    next_seq: u32,
) -> Result<u32> {
    let last_checkpoint_seq = checkpoint_store::load_last_checkpoint_seq(conn, session_id)
        .with_context(|| format!("loading prior checkpoint sequence for session `{session_id}`"))?;

    validate_checkpoint_sequence(last_checkpoint_seq, next_seq)?;
    checkpoint_store::persist_checkpoint_sequence(conn, session_id, next_seq)
        .with_context(|| format!("persisting checkpoint sequence for session `{session_id}`"))
}

pub fn evaluate_oqg(input: &OqgCheckInput) -> Vec<String> {
    let mut anomalies = Vec::new();

    if input.observed_fill_bps.saturating_add(500) < input.expected_fill_bps {
        anomalies.push(format!(
            "fill_rate_bps:{}<{}",
            input.observed_fill_bps, input.expected_fill_bps
        ));
    }

    if input.duplicate_rate_bps > input.max_duplicate_rate_bps {
        anomalies.push(format!(
            "duplicate_rate_bps:{}>{}",
            input.duplicate_rate_bps, input.max_duplicate_rate_bps
        ));
    }

    if input
        .observed_values
        .iter()
        .any(|value| *value < input.allowed_min_value || *value > input.allowed_max_value)
    {
        anomalies.push(format!(
            "value_range_outlier:{}..{}",
            input.allowed_min_value, input.allowed_max_value
        ));
    }

    anomalies
}

pub fn validate_final_result(
    schema: &Value,
    payload: &Value,
    oqg_input: &OqgCheckInput,
    artifact_path: &Path,
) -> ValidationOutcome {
    if let Err(error) = validate_result_schema(schema, payload) {
        return ValidationOutcome {
            disposition: ValidationDisposition::Failed,
            error_code: Some(ERROR_OUTPUT_CONTRACT_INVALID),
            error_context_json: Some(
                json!({
                    "artifact_path": artifact_path,
                    "reason": error.to_string(),
                })
                .to_string(),
            ),
            anomalies: Vec::new(),
        };
    }

    let anomalies = evaluate_oqg(oqg_input);
    if !anomalies.is_empty() {
        return ValidationOutcome {
            disposition: ValidationDisposition::Quarantined,
            error_code: Some(ERROR_SUSPECT_OUTPUT),
            error_context_json: Some(
                json!({
                    "artifact_path": artifact_path,
                    "anomalies": anomalies,
                })
                .to_string(),
            ),
            anomalies,
        };
    }

    ValidationOutcome {
        disposition: ValidationDisposition::Accepted,
        error_code: None,
        error_context_json: None,
        anomalies,
    }
}

pub fn persist_stage_validation(
    conn: &Connection,
    write: &StageValidationWrite<'_>,
    outcome: &ValidationOutcome,
) -> Result<()> {
    let now = Utc::now().to_rfc3339();
    let state = match outcome.disposition {
        ValidationDisposition::Accepted => "succeeded",
        ValidationDisposition::Quarantined => "quarantined",
        ValidationDisposition::Failed => "failed",
    };

    conn.execute(
        "\
        INSERT INTO process_stage_status (
          id, process_id, stage, scope_kind, scope_id, stage_run_id, state,
          freshness_state, started_at, completed_at, error_manifest_id,
          input_revision, output_revision, owner_id
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'fresh', ?8, ?9, NULL, 0, 0, ?10)
        ",
        params![
            format!("{}-{}", write.stage_run_id, state),
            write.process_id,
            write.stage,
            write.scope_kind,
            write.scope_id,
            write.stage_run_id,
            state,
            now,
            now,
            write.owner_id,
        ],
    )
    .context("inserting process_stage_status row")?;

    if outcome.disposition == ValidationDisposition::Accepted {
        conn.execute(
            "\
            INSERT INTO process_stage_heads (
              process_id, stage, scope_kind, scope_id, current_stage_run_id,
              freshness_state, owner_id
            ) VALUES (?1, ?2, ?3, ?4, ?5, 'fresh', ?6)
            ON CONFLICT(process_id, stage, scope_kind, scope_id) DO UPDATE SET
              current_stage_run_id = excluded.current_stage_run_id,
              freshness_state = excluded.freshness_state,
              owner_id = excluded.owner_id,
              updated_at = CURRENT_TIMESTAMP
            ",
            params![
                write.process_id,
                write.stage,
                write.scope_kind,
                write.scope_id,
                write.stage_run_id,
                write.owner_id,
            ],
        )
        .context("upserting process_stage_heads row")?;
    }

    Ok(())
}

pub fn classify_terminal_failure(
    has_valid_checkpoint: bool,
    has_final_contract: bool,
    cleanup_verdict: &str,
    exit_code: i32,
    specific_error_code: Option<&str>,
) -> Option<&'static str> {
    if has_valid_checkpoint && (!has_final_contract || cleanup_verdict != "Clean") {
        return Some(ERROR_PARTIAL_FAILURE);
    }

    if exit_code != 0 && specific_error_code.is_none() {
        return Some(ERROR_RUN_FAILED);
    }

    None
}

fn value_matches_type(value: &Value, expected_type: &str) -> bool {
    match expected_type {
        "string" => value.is_string(),
        "integer" => value.as_i64().is_some() || value.as_u64().is_some(),
        "number" => value.is_number(),
        "boolean" => value.is_boolean(),
        "array" => value.is_array(),
        "object" => value.is_object(),
        "null" => value.is_null(),
        _ => false,
    }
}
