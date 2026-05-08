#![forbid(unsafe_code)]
#![allow(dead_code)]

#[path = "../../src/db/sync.rs"]
mod sync_impl;

use anyhow::{Context, Result, ensure};
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use serde_json::{Value, json};

use super::db::{DEFAULT_SCHEMA_EPOCH, L4TestStateDb};

pub use sync_impl::{OutboxOp, OutboxWriter, canonical_content_hash};

const DEFAULT_EXECUTOR_ID: &str = "l4-sync-test-executor";
const DEFAULT_STAGE_RUN_ID: &str = "l4-sync-stage-run";
const DEFAULT_COMMIT_GROUP_ID: &str = "l4-sync-commit-group";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MutatedPushOwnedRow {
    pub entity_kind: String,
    pub entity_id: String,
    pub commit_group_id: String,
    pub stage_run_id: String,
    pub local_seq: i64,
    pub content_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutboxRow {
    pub local_seq: i64,
    pub executor_id: String,
    pub entity_kind: String,
    pub entity_id: String,
    pub op: String,
    pub row_version: i64,
    pub stage_run_id: String,
    pub schema_epoch: i64,
    pub commit_group_id: String,
    pub writer_epoch: Option<i64>,
    pub payload_json: String,
    pub content_hash: String,
}

pub struct OutboxHarness {
    conn: Connection,
    writer: OutboxWriter,
}

impl OutboxHarness {
    pub fn new(state_db: &L4TestStateDb) -> Result<Self> {
        let conn = Connection::open(state_db.db_path())
            .with_context(|| format!("opening L4 state DB {}", state_db.db_path().display()))?;
        conn.execute_batch("PRAGMA foreign_keys = ON")?;
        let schema_epoch =
            i32::try_from(DEFAULT_SCHEMA_EPOCH).context("DEFAULT_SCHEMA_EPOCH exceeds i32")?;
        let writer = OutboxWriter::new(DEFAULT_EXECUTOR_ID, schema_epoch)?;
        Ok(Self { conn, writer })
    }

    #[allow(clippy::missing_const_for_fn)]
    pub fn connection(&self) -> &Connection {
        &self.conn
    }

    pub fn mutate_push_owned_row(&mut self) -> Result<MutatedPushOwnedRow> {
        let commit_group_id = DEFAULT_COMMIT_GROUP_ID;
        let stage_run_id = DEFAULT_STAGE_RUN_ID;
        let process_id = "l4-sync-process";
        let payload = json!({
            "id": process_id,
            "current_stage": "qualification",
            "title": "L4 sync process",
            "latest_published_at": "2026-05-06T00:00:00Z",
            "latest_updated_at": "2026-05-06T00:00:00Z",
            "owner_id": "local-owner",
            "row_version": 1,
            "stage_run_id": stage_run_id
        });
        let payload_json = payload.to_string();
        let content_hash = canonical_content_hash(&payload_json)?;
        let writer = self.writer.clone();

        let local_seq =
            sync_impl::write_push_owned_transaction(&mut self.conn, commit_group_id, |tx| {
                tx.execute(
                    "\
                INSERT INTO procurement_processes (
                  id, current_stage, title, latest_published_at, latest_updated_at, owner_id,
                  row_version, stage_run_id
                ) VALUES (?1, 'qualification', 'L4 sync process', '2026-05-06T00:00:00Z',
                          '2026-05-06T00:00:00Z', 'local-owner', 1, ?2)
                ",
                    params![process_id, stage_run_id],
                )?;
                let local_seq = writer.write_push_owned_with_outbox(
                    tx,
                    "procurement_processes",
                    process_id,
                    OutboxOp::Insert,
                    &payload_json,
                    &content_hash,
                    stage_run_id,
                    commit_group_id,
                )?;
                tx.execute(
                    "UPDATE procurement_processes SET last_local_seq = ?1 WHERE id = ?2",
                    params![local_seq, process_id],
                )?;
                Ok(local_seq)
            })?;

        Ok(MutatedPushOwnedRow {
            entity_kind: "procurement_processes".to_owned(),
            entity_id: process_id.to_owned(),
            commit_group_id: commit_group_id.to_owned(),
            stage_run_id: stage_run_id.to_owned(),
            local_seq,
            content_hash,
        })
    }

    pub fn mutate_without_outbox(&mut self, process_id: &str) -> Result<()> {
        sync_impl::write_push_owned_transaction(&mut self.conn, "l4-missing-outbox", |tx| {
            tx.execute(
                "\
                INSERT INTO procurement_processes (
                  id, current_stage, title, latest_published_at, latest_updated_at, owner_id,
                  row_version, stage_run_id
                ) VALUES (?1, 'qualification', 'L4 missing outbox', '2026-05-06T00:00:00Z',
                          '2026-05-06T00:00:00Z', 'local-owner', 1, 'l4-missing-outbox-stage')
                ",
                params![process_id],
            )?;
            Ok(())
        })
    }

    pub fn outbox_rows_for_entity(
        &self,
        entity_kind: &str,
        entity_id: &str,
    ) -> Result<Vec<OutboxRow>> {
        let mut stmt = self.conn.prepare(
            "\
            SELECT local_seq, executor_id, entity_kind, entity_id, op, row_version, stage_run_id,
                   schema_epoch, commit_group_id, writer_epoch, payload_json, content_hash
            FROM local_outbox
            WHERE entity_kind = ?1 AND entity_id = ?2
            ORDER BY local_seq
            ",
        )?;
        let rows = stmt
            .query_map(params![entity_kind, entity_id], |row| {
                Ok(OutboxRow {
                    local_seq: row.get(0)?,
                    executor_id: row.get(1)?,
                    entity_kind: row.get(2)?,
                    entity_id: row.get(3)?,
                    op: row.get(4)?,
                    row_version: row.get(5)?,
                    stage_run_id: row.get(6)?,
                    schema_epoch: row.get(7)?,
                    commit_group_id: row.get(8)?,
                    writer_epoch: row.get(9)?,
                    payload_json: row.get(10)?,
                    content_hash: row.get(11)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn process_last_local_seq(&self, process_id: &str) -> Result<Option<i64>> {
        self.conn
            .query_row(
                "SELECT last_local_seq FROM procurement_processes WHERE id = ?1",
                params![process_id],
                |row| row.get(0),
            )
            .optional()
            .context("reading process last_local_seq")
    }

    pub fn count_rows(&self, table: &str) -> Result<i64> {
        ensure!(
            matches!(
                table,
                "local_outbox" | "procurement_processes" | "provisional_worker_outputs"
            ),
            "unsupported test row count table `{table}`"
        );
        let sql = format!("SELECT COUNT(*) FROM {table}");
        self.conn
            .query_row(&sql, [], |row| row.get(0))
            .with_context(|| format!("counting rows in {table}"))
    }
}

pub struct EnforcementHarness {
    conn: Connection,
}

impl EnforcementHarness {
    pub fn new(state_db: &L4TestStateDb) -> Result<Self> {
        let conn = Connection::open(state_db.db_path())
            .with_context(|| format!("opening L4 state DB {}", state_db.db_path().display()))?;
        conn.execute_batch("PRAGMA foreign_keys = ON")?;
        Ok(Self { conn })
    }

    pub fn reject_provisional_output(&mut self, worker_job_id: &str) -> Result<()> {
        let tx = self.conn.transaction()?;
        seed_workflow_job(&tx, worker_job_id)?;
        tx.execute(
            "\
            INSERT INTO provisional_worker_outputs (
              id, worker_job_id, payload_json, result_contract_blob_path, validation_status
            ) VALUES (?1, ?2, ?3, ?4, 'rejected')
            ",
            params![
                format!("provisional-{worker_job_id}"),
                worker_job_id,
                json!({"worker_job_id": worker_job_id, "result": "suspect"}).to_string(),
                format!("artifacts/provisional/{worker_job_id}.json")
            ],
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn count_rows(&self, table: &str) -> Result<i64> {
        ensure!(
            matches!(
                table,
                "local_outbox" | "procurement_processes" | "provisional_worker_outputs"
            ),
            "unsupported test row count table `{table}`"
        );
        let sql = format!("SELECT COUNT(*) FROM {table}");
        self.conn
            .query_row(&sql, [], |row| row.get(0))
            .with_context(|| format!("counting rows in {table}"))
    }
}

fn seed_workflow_job(tx: &Transaction<'_>, worker_job_id: &str) -> Result<()> {
    tx.execute(
        "\
        INSERT INTO workflow_jobs (
          id, kind, status, input_payload
        ) VALUES (?1, 'validator', 'completed', ?2)
        ON CONFLICT(id) DO NOTHING
        ",
        params![
            worker_job_id,
            json!({"worker_job_id": worker_job_id}).to_string()
        ],
    )?;
    Ok(())
}

pub fn parse_payload(payload_json: &str) -> Result<Value> {
    serde_json::from_str(payload_json).context("parsing payload JSON")
}
