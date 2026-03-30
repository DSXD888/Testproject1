#![forbid(unsafe_code)]

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, ensure};
use chrono::{DateTime, Utc};
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredCheckpointArtifact {
    pub seq: u32,
    pub path: PathBuf,
    pub sha256: String,
    pub size_bytes: usize,
}

pub fn load_last_checkpoint_seq(conn: &Connection, session_id: &str) -> Result<Option<u32>> {
    let last_checkpoint_seq: Option<u32> = conn
        .query_row(
            "SELECT last_checkpoint_seq FROM agent_session_logs WHERE id = ?1",
            params![session_id],
            |row| row.get(0),
        )
        .optional()
        .context("loading prior checkpoint sequence")?
        .flatten();
    ensure!(
        last_checkpoint_seq.is_some() || session_exists(conn, session_id)?,
        "agent_session_logs row missing for {session_id}"
    );
    Ok(last_checkpoint_seq)
}

pub fn persist_checkpoint_sequence(
    conn: &Connection,
    session_id: &str,
    next_seq: u32,
) -> Result<u32> {
    let updated = conn
        .execute(
            "UPDATE agent_session_logs SET last_checkpoint_seq = ?1 WHERE id = ?2",
            params![next_seq, session_id],
        )
        .context("persisting checkpoint sequence")?;
    ensure!(
        updated == 1,
        "agent_session_logs row missing for {session_id}"
    );
    Ok(next_seq)
}

#[allow(dead_code)]
pub fn persist_checkpoint_artifact(
    conn: &Connection,
    artifact_root: &Path,
    session_id: &str,
    started_at: DateTime<Utc>,
    schema: &Value,
    payload: &Value,
) -> Result<StoredCheckpointArtifact> {
    validate_checkpoint_payload(schema, payload)?;

    let seq = payload
        .get("seq")
        .and_then(Value::as_u64)
        .ok_or_else(|| anyhow!("checkpoint payload missing integer `seq`"))?;
    let seq = u32::try_from(seq).context("converting checkpoint `seq` to u32")?;

    let session_dir = checkpoint_session_dir(artifact_root, session_id, started_at)?;
    let path = session_dir.join(format!("checkpoint-{seq}.json"));
    let rendered = serde_json::to_vec_pretty(payload).context("serializing checkpoint payload")?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .with_context(|| format!("creating checkpoint artifact {}", path.display()))?;
    file.write_all(&rendered)
        .with_context(|| format!("writing checkpoint artifact {}", path.display()))?;
    file.flush()
        .with_context(|| format!("flushing checkpoint artifact {}", path.display()))?;

    let sha256 = sha256_hex(&rendered);
    persist_checkpoint_reference(conn, session_id, seq, &path, &sha256)?;

    Ok(StoredCheckpointArtifact {
        seq,
        path,
        sha256,
        size_bytes: rendered.len(),
    })
}

#[allow(dead_code)]
fn validate_checkpoint_payload(schema: &Value, payload: &Value) -> Result<()> {
    let payload_object = payload
        .as_object()
        .ok_or_else(|| anyhow!("checkpoint payload must be a JSON object"))?;

    if let Some(required_fields) = schema.get("required").and_then(Value::as_array) {
        for field in required_fields {
            let field_name = field
                .as_str()
                .ok_or_else(|| anyhow!("required field names must be strings"))?;
            ensure!(
                payload_object.contains_key(field_name),
                "checkpoint payload missing required field `{field_name}`"
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
                    "checkpoint payload field `{name}` violated type `{expected_type}`"
                );
            }
        }
    }

    Ok(())
}

#[allow(dead_code)]
fn persist_checkpoint_reference(
    conn: &Connection,
    session_id: &str,
    seq: u32,
    path: &Path,
    sha256: &str,
) -> Result<()> {
    let job_id: String = conn
        .query_row(
            "SELECT job_id FROM agent_session_logs WHERE id = ?1",
            params![session_id],
            |row| row.get(0),
        )
        .context("loading checkpoint session job_id")?;

    let existing: Option<String> = conn
        .query_row(
            "SELECT output_payload FROM workflow_jobs WHERE id = ?1",
            params![job_id],
            |row| row.get(0),
        )
        .optional()
        .context("loading workflow job payload for checkpoint ref")?;

    let mut payload = existing
        .as_deref()
        .and_then(|raw| serde_json::from_str::<Value>(raw).ok())
        .filter(Value::is_object)
        .unwrap_or_else(|| Value::Object(Map::default()));

    let object = payload
        .as_object_mut()
        .ok_or_else(|| anyhow!("workflow_jobs.output_payload must be a JSON object"))?;
    object.insert(
        "checkpoint_ref".to_owned(),
        serde_json::json!({
            "seq": seq,
            "path": path,
            "sha256": sha256,
        }),
    );

    conn.execute(
        "UPDATE workflow_jobs SET output_payload = ?1 WHERE id = ?2",
        params![payload.to_string(), job_id],
    )
    .context("persisting checkpoint reference to workflow job")?;
    Ok(())
}

#[allow(dead_code)]
fn checkpoint_session_dir(
    artifact_root: &Path,
    session_id: &str,
    started_at: DateTime<Utc>,
) -> Result<PathBuf> {
    ensure!(!session_id.is_empty(), "session_id must not be empty");
    let session_dir = artifact_root
        .join("sessions")
        .join(started_at.format("%Y").to_string())
        .join(started_at.format("%m").to_string())
        .join(session_id)
        .join("checkpoints");
    fs::create_dir_all(&session_dir)
        .with_context(|| format!("creating checkpoint dir {}", session_dir.display()))?;
    Ok(session_dir)
}

fn session_exists(conn: &Connection, session_id: &str) -> Result<bool> {
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM agent_session_logs WHERE id = ?1",
            params![session_id],
            |row| row.get(0),
        )
        .context("checking agent_session_logs row existence")?;
    Ok(count == 1)
}

#[allow(dead_code)]
fn value_matches_type(value: &Value, expected_type: &str) -> bool {
    match expected_type {
        "string" => value.is_string(),
        "integer" => value.as_i64().is_some() || value.as_u64().is_some(),
        "number" => value.is_number(),
        "boolean" => value.is_boolean(),
        "array" => value.is_array(),
        "object" => value.is_object(),
        "null" => value.is_null(),
        _ => true,
    }
}

#[allow(dead_code)]
fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    format!("{digest:x}")
}
