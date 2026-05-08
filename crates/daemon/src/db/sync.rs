#![forbid(unsafe_code)]

use anyhow::{Context, Result, ensure};
use rusqlite::{Connection, Transaction, TransactionBehavior, params};
use serde_json::Value;
use sha2::{Digest, Sha256};

const DEFAULT_EXECUTOR_ID: &str = "local-daemon";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OutboxOp {
    Insert,
    Update,
}

impl OutboxOp {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Insert => "insert",
            Self::Update => "update",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OutboxWriter {
    executor_id: String,
    schema_epoch: i32,
}

impl OutboxWriter {
    pub fn new(executor_id: impl Into<String>, schema_epoch: i32) -> Result<Self> {
        let executor_id = executor_id.into();
        ensure_non_empty("executor_id", &executor_id)?;
        ensure!(schema_epoch > 0, "schema_epoch must be positive");
        Ok(Self {
            executor_id,
            schema_epoch,
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn write_push_owned_with_outbox(
        &self,
        tx: &Transaction<'_>,
        entity_kind: &str,
        entity_id: &str,
        op: OutboxOp,
        payload_json: &str,
        content_hash: &str,
        stage_run_id: &str,
        commit_group_id: &str,
    ) -> Result<i64> {
        ensure_non_empty("entity_kind", entity_kind)?;
        ensure_non_empty("entity_id", entity_id)?;
        ensure_non_empty("stage_run_id", stage_run_id)?;
        ensure_non_empty("commit_group_id", commit_group_id)?;
        ensure_non_empty("content_hash", content_hash)?;

        let payload = parse_payload(payload_json)?;
        let row_version = payload_row_version(&payload)?;
        let canonical_hash = canonical_content_hash(payload_json)?;
        ensure!(
            canonical_hash == content_hash,
            "content_hash does not match canonical payload JSON"
        );

        tx.execute(
            "\
            INSERT INTO local_outbox (
              executor_id, entity_kind, entity_id, op, row_version, stage_run_id,
              schema_epoch, commit_group_id, payload_json, content_hash
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
            ",
            params![
                self.executor_id,
                entity_kind,
                entity_id,
                op.as_str(),
                row_version,
                stage_run_id,
                self.schema_epoch,
                commit_group_id,
                payload_json,
                content_hash
            ],
        )
        .context("writing local_outbox entry")?;

        Ok(tx.last_insert_rowid())
    }
}

#[allow(clippy::too_many_arguments)]
pub fn write_push_owned_with_outbox(
    tx: &Transaction<'_>,
    entity_kind: &str,
    entity_id: &str,
    op: OutboxOp,
    payload_json: &str,
    content_hash: &str,
    schema_epoch: i32,
    stage_run_id: &str,
    commit_group_id: &str,
) -> Result<i64> {
    let writer = OutboxWriter::new(DEFAULT_EXECUTOR_ID, schema_epoch)?;
    writer.write_push_owned_with_outbox(
        tx,
        entity_kind,
        entity_id,
        op,
        payload_json,
        content_hash,
        stage_run_id,
        commit_group_id,
    )
}

pub fn write_push_owned_transaction<T, F>(
    conn: &mut Connection,
    commit_group_id: &str,
    f: F,
) -> Result<T>
where
    F: FnOnce(&Transaction<'_>) -> Result<T>,
{
    ensure_non_empty("commit_group_id", commit_group_id)?;
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .context("starting push-owned SQLite transaction")?;
    let before = outbox_count_for_commit_group(&tx, commit_group_id)?;
    let value = f(&tx)?;
    let after = outbox_count_for_commit_group(&tx, commit_group_id)?;
    ensure!(
        after > before,
        "push-owned transaction omitted local_outbox entry for commit_group_id `{commit_group_id}`"
    );
    tx.commit()
        .context("committing push-owned SQLite transaction")?;
    Ok(value)
}

pub fn canonical_content_hash(payload_json: &str) -> Result<String> {
    let payload = parse_payload(payload_json)?;
    let canonical =
        serde_json::to_string(&payload).context("serializing canonical payload JSON")?;
    let mut hasher = Sha256::new();
    hasher.update(canonical.as_bytes());
    Ok(format!("{:x}", hasher.finalize()))
}

fn parse_payload(payload_json: &str) -> Result<Value> {
    serde_json::from_str(payload_json).context("parsing payload_json")
}

fn payload_row_version(payload: &Value) -> Result<i64> {
    let row_version = payload
        .get("row_version")
        .and_then(Value::as_i64)
        .context("payload_json must contain integer row_version")?;
    ensure!(row_version > 0, "payload_json row_version must be positive");
    Ok(row_version)
}

fn outbox_count_for_commit_group(tx: &Transaction<'_>, commit_group_id: &str) -> Result<i64> {
    tx.query_row(
        "SELECT COUNT(*) FROM local_outbox WHERE commit_group_id = ?1",
        params![commit_group_id],
        |row| row.get(0),
    )
    .context("counting local_outbox rows for commit_group_id")
}

fn ensure_non_empty(field: &str, value: &str) -> Result<()> {
    ensure!(!value.trim().is_empty(), "{field} must not be empty");
    Ok(())
}
