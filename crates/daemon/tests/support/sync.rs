#![forbid(unsafe_code)]
#![allow(dead_code)]

#[path = "../../src/db/sync.rs"]
mod sync_impl;

use anyhow::{Context, Result, ensure};
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use serde_json::{Value, json};

use super::db::{DEFAULT_SCHEMA_EPOCH, L4TestStateDb};

#[allow(unused_imports)]
pub use sync_impl::SupabaseRpcClient;
#[allow(unused_imports)]
pub use sync_impl::{
    BatchEnvelope, BatchEnvelopeAssembler, OutboxOp, OutboxWriter, WriterBindingConfig,
    WriterBindingMode, WriterLease, WriterLeaseAcquireResult, canonical_content_hash,
};

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
    pub row_version: i64,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PushOwnedFixture {
    pub process_id: String,
    pub row_version: i64,
    pub stage_run_id: String,
    pub commit_group_id: String,
    pub schema_epoch: i64,
}

impl PushOwnedFixture {
    pub fn new(process_id: impl Into<String>, row_version: i64) -> Self {
        Self {
            process_id: process_id.into(),
            row_version,
            stage_run_id: DEFAULT_STAGE_RUN_ID.to_owned(),
            commit_group_id: format!("{DEFAULT_COMMIT_GROUP_ID}-{row_version}"),
            schema_epoch: DEFAULT_SCHEMA_EPOCH,
        }
    }

    pub fn stage_run_id(mut self, stage_run_id: impl Into<String>) -> Self {
        self.stage_run_id = stage_run_id.into();
        self
    }

    pub fn commit_group_id(mut self, commit_group_id: impl Into<String>) -> Self {
        self.commit_group_id = commit_group_id.into();
        self
    }

    pub const fn schema_epoch(mut self, schema_epoch: i64) -> Self {
        self.schema_epoch = schema_epoch;
        self
    }
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
        self.mutate_push_owned_fixture(
            PushOwnedFixture::new("l4-sync-process", 1).commit_group_id(DEFAULT_COMMIT_GROUP_ID),
        )
    }

    pub fn mutate_push_owned_fixture(
        &mut self,
        fixture: PushOwnedFixture,
    ) -> Result<MutatedPushOwnedRow> {
        let payload = json!({
            "id": fixture.process_id,
            "current_stage": "qualification",
            "title": format!("L4 sync process {}", fixture.process_id),
            "latest_published_at": "2026-05-06T00:00:00Z",
            "latest_updated_at": "2026-05-06T00:00:00Z",
            "owner_id": "local-owner",
            "row_version": fixture.row_version,
            "stage_run_id": fixture.stage_run_id
        });
        let payload_json = payload.to_string();
        let content_hash = canonical_content_hash(&payload_json)?;
        let schema_epoch =
            i32::try_from(fixture.schema_epoch).context("fixture schema_epoch exceeds i32")?;
        let writer = OutboxWriter::new(DEFAULT_EXECUTOR_ID, schema_epoch)?;

        let local_seq = sync_impl::write_push_owned_transaction(
            &mut self.conn,
            &fixture.commit_group_id,
            |tx| {
                tx.execute(
                    "\
                INSERT INTO procurement_processes (
                  id, current_stage, title, latest_published_at, latest_updated_at, owner_id,
                  row_version, stage_run_id
                ) VALUES (?1, 'qualification', ?2, '2026-05-06T00:00:00Z',
                          '2026-05-06T00:00:00Z', 'local-owner', ?3, ?4)
                ",
                    params![
                        fixture.process_id,
                        format!("L4 sync process {}", fixture.process_id),
                        fixture.row_version,
                        fixture.stage_run_id
                    ],
                )?;
                let local_seq = writer.write_push_owned_with_outbox(
                    tx,
                    "procurement_processes",
                    &fixture.process_id,
                    OutboxOp::Insert,
                    &payload_json,
                    &content_hash,
                    &fixture.stage_run_id,
                    &fixture.commit_group_id,
                )?;
                tx.execute(
                    "UPDATE procurement_processes SET last_local_seq = ?1 WHERE id = ?2",
                    params![local_seq, fixture.process_id],
                )?;
                Ok(local_seq)
            },
        )?;

        Ok(MutatedPushOwnedRow {
            entity_kind: "procurement_processes".to_owned(),
            entity_id: fixture.process_id,
            commit_group_id: fixture.commit_group_id,
            stage_run_id: fixture.stage_run_id,
            local_seq,
            row_version: fixture.row_version,
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

    pub fn all_outbox_rows(&self) -> Result<Vec<OutboxRow>> {
        let mut stmt = self.conn.prepare(
            "\
            SELECT local_seq, executor_id, entity_kind, entity_id, op, row_version, stage_run_id,
                   schema_epoch, commit_group_id, writer_epoch, payload_json, content_hash
            FROM local_outbox
            ORDER BY local_seq
            ",
        )?;
        let rows = stmt
            .query_map([], |row| {
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

    pub fn outbox_row_by_seq(&self, local_seq: i64) -> Result<OutboxRow> {
        self.all_outbox_rows()?
            .into_iter()
            .find(|row| row.local_seq == local_seq)
            .with_context(|| format!("missing outbox row {local_seq}"))
    }

    pub fn assemble_batch(
        &mut self,
        max_rows: usize,
        writer_epoch: i64,
    ) -> Result<Option<BatchEnvelope>> {
        let assembler =
            BatchEnvelopeAssembler::new(DEFAULT_EXECUTOR_ID, "push-owned", writer_epoch, max_rows)?;
        assembler.materialize_next_batch(&mut self.conn)
    }

    pub fn load_active_batch(&self) -> Result<Option<BatchEnvelope>> {
        let assembler = BatchEnvelopeAssembler::new(DEFAULT_EXECUTOR_ID, "push-owned", 1, 100)?;
        assembler.load_active_batch(&self.conn)
    }

    pub fn mark_batch_retry(&self, batch_id: &str) -> Result<BatchEnvelope> {
        let assembler = BatchEnvelopeAssembler::new(DEFAULT_EXECUTOR_ID, "push-owned", 1, 100)?;
        assembler.mark_retry(&self.conn, batch_id)
    }

    pub fn mark_batch_acknowledged(&self, batch_id: &str) -> Result<()> {
        let assembler = BatchEnvelopeAssembler::new(DEFAULT_EXECUTOR_ID, "push-owned", 1, 100)?;
        assembler.mark_acknowledged(&self.conn, batch_id, "{}")
    }

    pub fn re_envelope_after_stale_writer(
        &mut self,
        batch_id: &str,
        writer_epoch: i64,
    ) -> Result<BatchEnvelope> {
        let assembler =
            BatchEnvelopeAssembler::new(DEFAULT_EXECUTOR_ID, "push-owned", writer_epoch, 100)?;
        assembler.re_envelope_after_stale_writer(&mut self.conn, batch_id)
    }

    pub fn batch_by_id(&self, batch_id: &str) -> Result<BatchEnvelope> {
        let assembler = BatchEnvelopeAssembler::new(DEFAULT_EXECUTOR_ID, "push-owned", 1, 100)?;
        assembler
            .load_batch(&self.conn, batch_id)
            .with_context(|| format!("missing batch {batch_id}"))?
            .with_context(|| format!("missing batch {batch_id}"))
    }

    pub fn sync_batch_count(&self) -> Result<i64> {
        self.conn
            .query_row("SELECT COUNT(*) FROM sync_batches", [], |row| row.get(0))
            .context("counting sync_batches")
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

pub struct WriterLeaseHarness {
    conn: Connection,
}

impl WriterLeaseHarness {
    pub fn new(state_db: &L4TestStateDb) -> Result<Self> {
        let conn = Connection::open(state_db.db_path())
            .with_context(|| format!("opening L4 state DB {}", state_db.db_path().display()))?;
        conn.execute_batch("PRAGMA foreign_keys = ON")?;
        Ok(Self { conn })
    }

    pub fn acquire(
        &mut self,
        executor_id: &str,
        now_epoch_secs: i64,
    ) -> Result<WriterLeaseAcquireResult> {
        WriterLease::new("default", executor_id)?.acquire(&mut self.conn, now_epoch_secs)
    }

    pub fn heartbeat(
        &mut self,
        executor_id: &str,
        now_epoch_secs: i64,
    ) -> Result<WriterLeaseAcquireResult> {
        WriterLease::new("default", executor_id)?.heartbeat(&mut self.conn, now_epoch_secs)
    }

    pub fn release(&self, executor_id: &str) -> Result<()> {
        WriterLease::new("default", executor_id)?.release(&self.conn)?;
        Ok(())
    }

    pub fn current_epoch(&self) -> Result<Option<i64>> {
        WriterLease::new("default", DEFAULT_EXECUTOR_ID)?.current_epoch(&self.conn)
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
