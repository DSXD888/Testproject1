#![forbid(unsafe_code)]
#![allow(dead_code)]

use std::collections::BTreeMap;

#[path = "../../src/db/sync.rs"]
mod sync_impl;

use anyhow::{Context, Result, bail, ensure};
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use serde_json::{Value, json};

use super::db::{DEFAULT_SCHEMA_EPOCH, L4TestStateDb};

#[allow(unused_imports)]
pub use sync_impl::SupabaseRpcClient;
#[allow(unused_imports)]
pub use sync_impl::{
    BatchEnvelope, BatchEnvelopeAssembler, FencedPullOutcome, FencedPullPageBoundary, OutboxOp,
    OutboxWriter, PendingIntentReconciler, PendingIntentReconciliationPlan, PullCursor,
    PullCursorDomain, PullCursorStore, PushBatchApplyResponse, RemoteOwnedChange, SyncCycle,
    SyncCycleOutcome, SyncCyclePhase, SyncCycleTrigger, SyncHealth, ViewerSyncStatus,
    ViewerSyncStatusWriter, WriterBindingConfig, WriterBindingMode, WriterLease,
    WriterLeaseAcquireResult, canonical_content_hash, decision_conflict_event_payload,
};
#[allow(unused_imports)]
pub use sync_impl::{OperatorDecisionApplyResponse, OperatorDecisionCommand};

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

    pub fn record_successful_apply_response(&mut self, batch_id: &str) -> Result<()> {
        let response = PushBatchApplyResponse::from_rpc_value(json!({
            "accepted": true,
            "status": "applied"
        }))?;
        let assembler = BatchEnvelopeAssembler::new(DEFAULT_EXECUTOR_ID, "push-owned", 1, 100)?;
        assembler.record_apply_response(&mut self.conn, batch_id, &response)?;
        Ok(())
    }

    pub fn viewer_sync_status(&self) -> Result<ViewerSyncStatus> {
        ViewerSyncStatusWriter::for_default_org(DEFAULT_SCHEMA_EPOCH)?
            .load(&self.conn)?
            .context("missing viewer_sync_status row")
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

    pub fn record_stale_writer_rejection_after_reacquire(
        &mut self,
        batch_id: &str,
        writer_epoch: i64,
    ) -> Result<BatchEnvelope> {
        let stale_writer_epoch = self.batch_by_id(batch_id)?.writer_epoch;
        let response = PushBatchApplyResponse::from_rpc_value(json!({
            "accepted": false,
            "status": "rejected",
            "error": "stale_writer",
            "remote_writer_epoch": writer_epoch,
            "writer_epoch": stale_writer_epoch
        }))?;
        let assembler =
            BatchEnvelopeAssembler::new(DEFAULT_EXECUTOR_ID, "push-owned", writer_epoch, 100)?;
        assembler
            .record_apply_response(&mut self.conn, batch_id, &response)?
            .context("stale writer response did not create replacement batch")
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CasDecisionHead {
    pub process_id: String,
    pub disposition: String,
    pub remote_revision: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperatorDecisionIntentStatus {
    pub status: String,
    pub applied_revision: Option<i64>,
    pub conflict_json: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingOperatorIntentStatus {
    pub id: String,
    pub status: String,
    pub superseded_by_intent_id: Option<String>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CasCommandLedgerRow {
    pub command_id: String,
    pub process_id: String,
    pub base_revision: i64,
    pub applied_revision: Option<i64>,
    pub disposition: String,
    pub result: String,
    pub response_payload_json: String,
    pub created_at_epoch_secs: i64,
}

pub struct OperatorDecisionCasHarness {
    conn: Connection,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct QueuedOperatorDecisionIntent {
    id: i64,
    process_id: String,
    disposition: String,
    base_remote_revision: i64,
    command_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FixtureDecisionHead {
    disposition: String,
    remote_revision: i64,
}

impl OperatorDecisionCasHarness {
    pub fn new(state_db: &L4TestStateDb) -> Result<Self> {
        let conn = Connection::open(state_db.db_path())
            .with_context(|| format!("opening L4 state DB {}", state_db.db_path().display()))?;
        conn.execute_batch("PRAGMA foreign_keys = ON")?;
        ensure_operator_decision_cas_fixture_tables(&conn)?;
        Ok(Self { conn })
    }

    pub fn seed_head(
        &self,
        process_id: &str,
        disposition: &str,
        remote_revision: i64,
    ) -> Result<()> {
        ensure!(
            matches!(
                disposition,
                "pending" | "watching" | "approved" | "rejected"
            ),
            "unsupported fixture disposition `{disposition}`"
        );
        ensure!(
            remote_revision > 0,
            "remote_revision must be positive in CAS fixtures"
        );
        self.conn
            .execute(
                "\
                INSERT INTO l4_operator_decision_heads_fixture (
                  process_id, disposition, remote_revision
                ) VALUES (?1, ?2, ?3)
                ON CONFLICT(process_id) DO UPDATE SET
                  disposition = excluded.disposition,
                  remote_revision = excluded.remote_revision
                ",
                params![process_id, disposition, remote_revision],
            )
            .context("seeding operator decision CAS fixture head")?;
        Ok(())
    }

    pub fn queue_intent(
        &self,
        process_id: &str,
        disposition: &str,
        base_remote_revision: i64,
        command_id: &str,
    ) -> Result<()> {
        ensure!(
            matches!(
                disposition,
                "pending" | "watching" | "approved" | "rejected"
            ),
            "unsupported fixture disposition `{disposition}`"
        );
        ensure!(
            base_remote_revision > 0,
            "base_remote_revision must be positive in CAS fixtures"
        );
        self.conn
            .execute(
                "\
                INSERT INTO l4_operator_decision_intents_fixture (
                  process_id, disposition, base_remote_revision, command_id
                ) VALUES (?1, ?2, ?3, ?4)
                ",
                params![process_id, disposition, base_remote_revision, command_id],
            )
            .context("queueing operator decision CAS intent")?;
        Ok(())
    }

    pub fn queue_pending_operator_intent(
        &self,
        process_id: &str,
        desired_disposition: &str,
        base_remote_revision: i64,
        intent_id: &str,
        idempotency_key: &str,
        queued_at: &str,
    ) -> Result<()> {
        ensure!(
            matches!(desired_disposition, "approve" | "reject" | "watch"),
            "unsupported pending operator intent disposition `{desired_disposition}`"
        );
        ensure!(
            base_remote_revision >= 0,
            "base_remote_revision must be non-negative for pending intents"
        );
        self.conn
            .execute(
                "\
                INSERT INTO procurement_processes (
                  id, current_stage, title, latest_published_at, latest_updated_at, owner_id
                ) VALUES (?1, 'qualification', 'L4 pending intent fixture',
                          '2026-05-08T00:00:00Z', '2026-05-08T00:00:00Z', 'local-owner')
                ON CONFLICT(id) DO NOTHING
                ",
                params![process_id],
            )
            .context("seeding process for pending operator intent")?;
        self.conn
            .execute(
                "\
                INSERT INTO pending_operator_intents (
                  id, process_id, desired_disposition, reviewed_basis_hash, reviewed_manifest_hash,
                  reviewed_at_stage_run_id, reviewed_at, base_remote_revision, idempotency_key,
                  status, superseded_by_intent_id, queued_at, decision_note, last_attempt_at,
                  last_error
                ) VALUES (
                  ?1, ?2, ?3, 'basis-hash', 'manifest-hash', 'stage-run', ?6, ?4, ?5,
                  'queued', NULL, ?6, NULL, NULL, NULL
                )
                ",
                params![
                    intent_id,
                    process_id,
                    desired_disposition,
                    base_remote_revision,
                    idempotency_key,
                    queued_at
                ],
            )
            .context("queueing real pending operator intent")?;
        Ok(())
    }

    pub fn prepare_pending_intents(&mut self) -> Result<PendingIntentReconciliationPlan> {
        let heads = self.fixture_head_revisions()?;
        PendingIntentReconciler::prepare_commands(&mut self.conn, |process_id| {
            heads
                .get(process_id)
                .copied()
                .with_context(|| format!("missing fixture remote head `{process_id}`"))
        })
    }

    pub fn record_pending_cas_response(
        &self,
        response: &OperatorDecisionApplyResponse,
    ) -> Result<()> {
        PendingIntentReconciler::record_cas_response(&self.conn, response)
    }

    pub fn apply_pending_command_to_fixture(
        &self,
        command: &OperatorDecisionCommand,
    ) -> Result<OperatorDecisionApplyResponse> {
        let tx = self
            .conn
            .unchecked_transaction()
            .context("starting pending command fixture transaction")?;
        let current = load_fixture_decision_head(&tx, &command.process_id)?;
        let response = if current.remote_revision == command.base_revision {
            tx.execute(
                "\
                UPDATE l4_operator_decision_heads_fixture
                SET disposition = ?2,
                    remote_revision = ?3
                WHERE process_id = ?1
                ",
                params![
                    command.process_id,
                    command.disposition,
                    command.base_revision + 1
                ],
            )
            .context("applying pending command to fixture head")?;
            json!({
                "accepted": true,
                "status": "applied",
                "process_id": command.process_id,
                "command_id": command.command_id,
                "applied_revision": command.base_revision + 1
            })
        } else {
            json!({
                "accepted": false,
                "status": "conflict",
                "process_id": command.process_id,
                "command_id": command.command_id,
                "current_revision": current.remote_revision,
                "current_disposition": current.disposition,
                "rejected_payload": {
                    "process_id": command.process_id,
                    "disposition": command.disposition,
                    "base_revision": command.base_revision,
                    "command_id": command.command_id
                }
            })
        };
        tx.commit()
            .context("committing pending command fixture transaction")?;
        OperatorDecisionApplyResponse::from_rpc_value(response)
    }

    pub fn flush_next_intent(&mut self) -> Result<OperatorDecisionApplyResponse> {
        let tx = self
            .conn
            .transaction()
            .context("starting operator decision CAS fixture transaction")?;
        let intent = load_next_queued_operator_decision_intent(&tx)?;
        let response_json =
            if let Some(replay) = load_fixture_command_ledger_response(&tx, &intent.command_id)? {
                mark_replayed_intent_from_ledger(&tx, &intent, &replay)?;
                replay
            } else {
                let current = load_fixture_decision_head(&tx, &intent.process_id)?;
                if current.remote_revision == intent.base_remote_revision {
                    apply_fixture_decision_intent(&tx, &intent, current.remote_revision + 1)?
                } else {
                    conflict_fixture_decision_intent(&tx, &intent, &current)?
                }
            };
        tx.commit()
            .context("committing operator decision CAS fixture transaction")?;
        OperatorDecisionApplyResponse::from_rpc_value(response_json)
    }

    pub fn head(&self, process_id: &str) -> Result<CasDecisionHead> {
        self.conn
            .query_row(
                "\
                SELECT process_id, disposition, remote_revision
                FROM l4_operator_decision_heads_fixture
                WHERE process_id = ?1
                ",
                params![process_id],
                |row| {
                    Ok(CasDecisionHead {
                        process_id: row.get(0)?,
                        disposition: row.get(1)?,
                        remote_revision: row.get(2)?,
                    })
                },
            )
            .optional()
            .context("loading operator decision CAS fixture head")?
            .with_context(|| format!("missing operator decision CAS fixture head `{process_id}`"))
    }

    pub fn intent_status(&self, command_id: &str) -> Result<OperatorDecisionIntentStatus> {
        self.conn
            .query_row(
                "\
                SELECT status, applied_revision, conflict_json
                FROM l4_operator_decision_intents_fixture
                WHERE command_id = ?1
                ",
                params![command_id],
                |row| {
                    Ok(OperatorDecisionIntentStatus {
                        status: row.get(0)?,
                        applied_revision: row.get(1)?,
                        conflict_json: row.get(2)?,
                    })
                },
            )
            .optional()
            .context("loading operator decision CAS fixture intent status")?
            .with_context(|| format!("missing operator decision CAS intent `{command_id}`"))
    }

    pub fn pending_intent_status(&self, intent_id: &str) -> Result<PendingOperatorIntentStatus> {
        self.conn
            .query_row(
                "\
                SELECT id, status, superseded_by_intent_id, last_error
                FROM pending_operator_intents
                WHERE id = ?1
                ",
                params![intent_id],
                |row| {
                    Ok(PendingOperatorIntentStatus {
                        id: row.get(0)?,
                        status: row.get(1)?,
                        superseded_by_intent_id: row.get(2)?,
                        last_error: row.get(3)?,
                    })
                },
            )
            .optional()
            .context("loading real pending operator intent status")?
            .with_context(|| format!("missing pending operator intent `{intent_id}`"))
    }

    pub fn ledger_row(&self, command_id: &str) -> Result<CasCommandLedgerRow> {
        self.conn
            .query_row(
                "\
                SELECT command_id, process_id, base_revision, applied_revision, disposition,
                       result, response_payload_json, created_at_epoch_secs
                FROM l4_operator_decision_commands_fixture
                WHERE command_id = ?1
                ",
                params![command_id],
                |row| {
                    Ok(CasCommandLedgerRow {
                        command_id: row.get(0)?,
                        process_id: row.get(1)?,
                        base_revision: row.get(2)?,
                        applied_revision: row.get(3)?,
                        disposition: row.get(4)?,
                        result: row.get(5)?,
                        response_payload_json: row.get(6)?,
                        created_at_epoch_secs: row.get(7)?,
                    })
                },
            )
            .optional()
            .context("loading operator decision command ledger row")?
            .with_context(|| format!("missing command ledger row `{command_id}`"))
    }

    pub fn command_ledger_row_count(&self, command_id: &str) -> Result<i64> {
        self.conn
            .query_row(
                "\
                SELECT COUNT(*)
                FROM l4_operator_decision_commands_fixture
                WHERE command_id = ?1
                ",
                params![command_id],
                |row| row.get(0),
            )
            .context("counting command ledger rows")
    }

    pub fn set_ledger_created_at(&self, command_id: &str, epoch_secs: i64) -> Result<()> {
        let updated = self
            .conn
            .execute(
                "\
                UPDATE l4_operator_decision_commands_fixture
                SET created_at_epoch_secs = ?2
                WHERE command_id = ?1
                ",
                params![command_id, epoch_secs],
            )
            .context("backdating command ledger row")?;
        ensure!(
            updated == 1,
            "expected one command ledger row to update for `{command_id}`, got {updated}"
        );
        Ok(())
    }

    pub fn gc_command_ledger(&self, retention_days: i64, now_epoch_secs: i64) -> Result<usize> {
        ensure!(retention_days > 0, "retention_days must be positive");
        let cutoff = now_epoch_secs - retention_days * 86_400;
        self.conn
            .execute(
                "\
                DELETE FROM l4_operator_decision_commands_fixture
                WHERE created_at_epoch_secs < ?1
                ",
                params![cutoff],
            )
            .context("garbage-collecting command ledger fixture")
    }

    fn fixture_head_revisions(&self) -> Result<BTreeMap<String, i64>> {
        let mut stmt = self.conn.prepare(
            "\
            SELECT process_id, remote_revision
            FROM l4_operator_decision_heads_fixture
            ",
        )?;
        let rows = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()
            .context("loading fixture decision head revisions")?;
        Ok(rows.into_iter().collect())
    }
}

fn load_next_queued_operator_decision_intent(
    tx: &Transaction<'_>,
) -> Result<QueuedOperatorDecisionIntent> {
    tx.query_row(
        "\
        SELECT id, process_id, disposition, base_remote_revision, command_id
        FROM l4_operator_decision_intents_fixture
        WHERE status = 'queued'
        ORDER BY id ASC
        LIMIT 1
        ",
        [],
        |row| {
            Ok(QueuedOperatorDecisionIntent {
                id: row.get(0)?,
                process_id: row.get(1)?,
                disposition: row.get(2)?,
                base_remote_revision: row.get(3)?,
                command_id: row.get(4)?,
            })
        },
    )
    .optional()
    .context("loading next queued operator decision intent")?
    .context("no queued operator decision intent to flush")
}

fn load_fixture_decision_head(
    tx: &Transaction<'_>,
    process_id: &str,
) -> Result<FixtureDecisionHead> {
    tx.query_row(
        "\
        SELECT disposition, remote_revision
        FROM l4_operator_decision_heads_fixture
        WHERE process_id = ?1
        ",
        params![process_id],
        |row| {
            Ok(FixtureDecisionHead {
                disposition: row.get(0)?,
                remote_revision: row.get(1)?,
            })
        },
    )
    .optional()
    .context("loading current operator decision head")?
    .with_context(|| format!("missing remote decision head `{process_id}`"))
}

fn load_fixture_command_ledger_response(
    tx: &Transaction<'_>,
    command_id: &str,
) -> Result<Option<Value>> {
    tx.query_row(
        "\
        SELECT response_payload_json
        FROM l4_operator_decision_commands_fixture
        WHERE command_id = ?1
        ",
        params![command_id],
        |row| row.get::<_, String>(0),
    )
    .optional()
    .context("loading command ledger replay payload")?
    .map(|payload| serde_json::from_str(&payload).context("parsing command ledger replay payload"))
    .transpose()
}

fn mark_replayed_intent_from_ledger(
    tx: &Transaction<'_>,
    intent: &QueuedOperatorDecisionIntent,
    ledger_response: &Value,
) -> Result<()> {
    let status = ledger_response
        .get("status")
        .and_then(Value::as_str)
        .context("ledger response missing status")?;
    if status == "applied" {
        let applied_revision = ledger_response
            .get("applied_revision")
            .and_then(Value::as_i64)
            .context("ledger applied response missing applied_revision")?;
        tx.execute(
            "\
            UPDATE l4_operator_decision_intents_fixture
            SET status = 'applied',
                applied_revision = ?2
            WHERE id = ?1
            ",
            params![intent.id, applied_revision],
        )
        .context("marking replayed operator decision intent applied")?;
    } else {
        tx.execute(
            "\
            UPDATE l4_operator_decision_intents_fixture
            SET status = 'conflicted',
                conflict_json = ?2
            WHERE id = ?1
            ",
            params![intent.id, ledger_response.to_string()],
        )
        .context("marking replayed operator decision intent conflicted")?;
    }
    Ok(())
}

fn insert_fixture_command_ledger(
    tx: &Transaction<'_>,
    intent: &QueuedOperatorDecisionIntent,
    applied_revision: Option<i64>,
    result: &str,
    response_payload: &Value,
) -> Result<()> {
    tx.execute(
        "\
        INSERT INTO l4_operator_decision_commands_fixture (
          command_id, process_id, base_revision, applied_revision, disposition, result,
          response_payload_json
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
        ",
        params![
            intent.command_id,
            intent.process_id,
            intent.base_remote_revision,
            applied_revision,
            intent.disposition,
            result,
            response_payload.to_string()
        ],
    )
    .context("inserting operator decision command ledger row")?;
    Ok(())
}

fn apply_fixture_decision_intent(
    tx: &Transaction<'_>,
    intent: &QueuedOperatorDecisionIntent,
    applied_revision: i64,
) -> Result<Value> {
    tx.execute(
        "\
        UPDATE l4_operator_decision_heads_fixture
        SET disposition = ?2,
            remote_revision = ?3
        WHERE process_id = ?1
        ",
        params![intent.process_id, intent.disposition, applied_revision],
    )
    .context("applying operator decision CAS fixture command")?;
    tx.execute(
        "\
        UPDATE l4_operator_decision_intents_fixture
        SET status = 'applied',
            applied_revision = ?2
        WHERE id = ?1
        ",
        params![intent.id, applied_revision],
    )
    .context("marking operator decision CAS intent applied")?;
    let response = json!({
        "accepted": true,
        "status": "applied",
        "process_id": intent.process_id,
        "command_id": intent.command_id,
        "applied_revision": applied_revision
    });
    insert_fixture_command_ledger(tx, intent, Some(applied_revision), "applied", &response)?;
    Ok(response)
}

fn conflict_fixture_decision_intent(
    tx: &Transaction<'_>,
    intent: &QueuedOperatorDecisionIntent,
    current: &FixtureDecisionHead,
) -> Result<Value> {
    let rejected_payload = json!({
        "process_id": intent.process_id,
        "disposition": intent.disposition,
        "base_revision": intent.base_remote_revision,
        "command_id": intent.command_id
    });
    let conflict = json!({
        "accepted": false,
        "status": "conflict",
        "process_id": intent.process_id,
        "command_id": intent.command_id,
        "current_revision": current.remote_revision,
        "current_disposition": current.disposition,
        "rejected_payload": rejected_payload
    });
    tx.execute(
        "\
        UPDATE l4_operator_decision_intents_fixture
        SET status = 'conflicted',
            conflict_json = ?2
        WHERE id = ?1
        ",
        params![intent.id, conflict.to_string()],
    )
    .context("marking operator decision CAS intent conflicted")?;
    insert_fixture_command_ledger(tx, intent, None, "conflict", &conflict)?;
    Ok(conflict)
}

fn ensure_operator_decision_cas_fixture_tables(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "\
        CREATE TABLE IF NOT EXISTS l4_operator_decision_heads_fixture (
          process_id TEXT PRIMARY KEY,
          disposition TEXT NOT NULL CHECK (disposition IN ('pending', 'watching', 'approved', 'rejected')),
          remote_revision INTEGER NOT NULL CHECK (remote_revision > 0)
        );

        CREATE TABLE IF NOT EXISTS l4_operator_decision_intents_fixture (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          process_id TEXT NOT NULL,
          disposition TEXT NOT NULL CHECK (disposition IN ('pending', 'watching', 'approved', 'rejected')),
          base_remote_revision INTEGER NOT NULL CHECK (base_remote_revision > 0),
          command_id TEXT NOT NULL,
          status TEXT NOT NULL DEFAULT 'queued' CHECK (status IN ('queued', 'applied', 'conflicted')),
          applied_revision INTEGER,
          conflict_json TEXT,
          queued_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        );

        CREATE TABLE IF NOT EXISTS l4_operator_decision_commands_fixture (
          command_id TEXT PRIMARY KEY,
          process_id TEXT NOT NULL,
          base_revision INTEGER NOT NULL,
          applied_revision INTEGER,
          disposition TEXT NOT NULL CHECK (disposition IN ('pending', 'watching', 'approved', 'rejected')),
          result TEXT NOT NULL CHECK (result IN ('applied', 'conflict', 'superseded')),
          response_payload_json TEXT NOT NULL,
          created_at_epoch_secs INTEGER NOT NULL DEFAULT 1800000000
        );
        ",
    )
    .context("ensuring operator decision CAS fixture tables")
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

    pub fn insert_pending_outbox_marker(&self, entity_id: &str) -> Result<i64> {
        ensure!(!entity_id.trim().is_empty(), "entity_id must not be empty");
        let payload = json!({
            "id": entity_id,
            "current_stage": "sync-status-fixture",
            "title": format!("L4 sync status fixture {entity_id}"),
            "latest_published_at": "2026-05-08T00:00:00Z",
            "latest_updated_at": "2026-05-08T00:00:00Z",
            "owner_id": "local-owner",
            "row_version": 1,
            "stage_run_id": "l4-sync-status-stage"
        });
        let payload_json = payload.to_string();
        let content_hash = canonical_content_hash(&payload_json)?;
        self.conn
            .execute(
                "\
                INSERT INTO local_outbox (
                  executor_id, entity_kind, entity_id, op, row_version, stage_run_id,
                  schema_epoch, commit_group_id, payload_json, content_hash
                ) VALUES (?1, 'procurement_processes', ?2, 'insert', 1,
                          'l4-sync-status-stage', ?3, ?4, ?5, ?6)
                ",
                params![
                    DEFAULT_EXECUTOR_ID,
                    entity_id,
                    DEFAULT_SCHEMA_EPOCH,
                    format!("l4-sync-status-{entity_id}"),
                    payload_json,
                    content_hash
                ],
            )
            .context("inserting pending outbox marker")?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn record_viewer_sync_apply_success(
        &self,
        now_epoch_secs: i64,
        last_acked_local_seq: i64,
        projection_generation: i64,
        daemon_version: &str,
    ) -> Result<ViewerSyncStatus> {
        ViewerSyncStatusWriter::new("default", DEFAULT_SCHEMA_EPOCH, daemon_version)?
            .record_apply_success(
                &self.conn,
                now_epoch_secs,
                last_acked_local_seq,
                projection_generation,
            )
    }

    pub fn record_viewer_sync_heartbeat(
        &self,
        now_epoch_secs: i64,
        projection_generation: i64,
        daemon_version: &str,
    ) -> Result<ViewerSyncStatus> {
        ViewerSyncStatusWriter::new("default", DEFAULT_SCHEMA_EPOCH, daemon_version)?
            .record_heartbeat_status(&self.conn, now_epoch_secs, projection_generation)
    }

    pub fn release(&self, executor_id: &str) -> Result<()> {
        WriterLease::new("default", executor_id)?.release(&self.conn)?;
        Ok(())
    }

    pub fn current_epoch(&self) -> Result<Option<i64>> {
        WriterLease::new("default", DEFAULT_EXECUTOR_ID)?.current_epoch(&self.conn)
    }
}

fn ensure_remote_owned_fixture_tables(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "\
        CREATE TABLE IF NOT EXISTS l4_remote_owned_fixture (
          domain TEXT NOT NULL,
          entity_id TEXT NOT NULL,
          remote_change_seq INTEGER NOT NULL CHECK (remote_change_seq > 0),
          payload_json TEXT NOT NULL,
          deleted_at TEXT,
          op TEXT NOT NULL CHECK (op IN ('insert', 'update', 'tombstone')),
          PRIMARY KEY (domain, remote_change_seq)
        );

        CREATE TABLE IF NOT EXISTS l4_remote_owned_applied (
          domain TEXT NOT NULL,
          remote_change_seq INTEGER NOT NULL CHECK (remote_change_seq > 0),
          entity_id TEXT NOT NULL,
          payload_json TEXT NOT NULL,
          deleted_at TEXT,
          PRIMARY KEY (domain, remote_change_seq)
        );
        ",
    )
    .context("ensuring remote-owned pull fixture tables")
}

pub struct PullCursorHarness {
    conn: Connection,
}

impl PullCursorHarness {
    pub fn new(state_db: &L4TestStateDb) -> Result<Self> {
        let conn = Connection::open(state_db.db_path())
            .with_context(|| format!("opening L4 state DB {}", state_db.db_path().display()))?;
        conn.execute_batch("PRAGMA foreign_keys = ON")?;
        PullCursorStore::initialize(&conn)?;
        ensure_remote_owned_fixture_tables(&conn)?;
        Ok(Self { conn })
    }

    pub fn all(&self) -> Result<Vec<PullCursor>> {
        PullCursorStore::all(&self.conn)
    }

    pub fn load(&self, domain: PullCursorDomain) -> Result<PullCursor> {
        PullCursorStore::load(&self.conn, domain)
    }

    pub fn record_success(
        &self,
        domain: PullCursorDomain,
        remote_change_seq: i64,
    ) -> Result<PullCursor> {
        PullCursorStore::record_success(&self.conn, domain, remote_change_seq)
    }

    pub fn record_failure(&self, domain: PullCursorDomain) -> Result<PullCursor> {
        PullCursorStore::record_failure(&self.conn, domain)
    }

    pub fn seed_remote_owned_change(
        &self,
        domain: PullCursorDomain,
        entity_id: &str,
        payload: &Value,
    ) -> Result<i64> {
        self.append_remote_owned_change(domain, entity_id, payload, None, "insert")
    }

    pub fn pull_fenced(
        &self,
        domain: PullCursorDomain,
        page_size: usize,
    ) -> Result<FencedPullOutcome> {
        PullCursorStore::apply_fenced_pull(
            &self.conn,
            domain,
            page_size,
            |pull_domain| self.remote_hiwater(pull_domain),
            |pull_domain, lower_exclusive, hiwater, limit| {
                self.remote_page(pull_domain, lower_exclusive, hiwater, limit)
            },
            |pull_domain, change| self.apply_remote_change(pull_domain, change),
        )
    }

    pub fn pull_fenced_with_concurrent_change(
        &self,
        domain: PullCursorDomain,
        page_size: usize,
        concurrent_entity_id: &str,
        concurrent_payload: &Value,
    ) -> Result<FencedPullOutcome> {
        let mut injected = false;
        PullCursorStore::apply_fenced_pull(
            &self.conn,
            domain,
            page_size,
            |pull_domain| {
                let hiwater = self.remote_hiwater(pull_domain)?;
                if !injected {
                    self.seed_remote_owned_change(
                        pull_domain,
                        concurrent_entity_id,
                        concurrent_payload,
                    )?;
                    injected = true;
                }
                Ok(hiwater)
            },
            |pull_domain, lower_exclusive, hiwater, limit| {
                self.remote_page(pull_domain, lower_exclusive, hiwater, limit)
            },
            |pull_domain, change| self.apply_remote_change(pull_domain, change),
        )
    }

    pub fn applied_remote_entity_ids(&self, domain: PullCursorDomain) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare(
            "\
            SELECT entity_id
            FROM l4_remote_owned_applied
            WHERE domain = ?1
            ORDER BY remote_change_seq
            ",
        )?;
        stmt.query_map(params![domain.as_str()], |row| row.get(0))?
            .collect::<std::result::Result<Vec<_>, _>>()
            .context("listing applied remote entity ids")
    }

    pub fn applied_remote_change_seqs(&self, domain: PullCursorDomain) -> Result<Vec<i64>> {
        let mut stmt = self.conn.prepare(
            "\
            SELECT remote_change_seq
            FROM l4_remote_owned_applied
            WHERE domain = ?1
            ORDER BY remote_change_seq
            ",
        )?;
        stmt.query_map(params![domain.as_str()], |row| row.get(0))?
            .collect::<std::result::Result<Vec<_>, _>>()
            .context("listing applied remote change sequences")
    }

    pub fn exercise_remote_change_sequence(
        &self,
        domain: PullCursorDomain,
        entity_id: &str,
    ) -> Result<Vec<i64>> {
        Ok(vec![
            self.append_remote_owned_change(
                domain,
                entity_id,
                &json!({"entity_id": entity_id, "op": "insert"}),
                None,
                "insert",
            )?,
            self.append_remote_owned_change(
                domain,
                entity_id,
                &json!({"entity_id": entity_id, "op": "update"}),
                None,
                "update",
            )?,
            self.append_remote_owned_change(
                domain,
                entity_id,
                &json!({"entity_id": entity_id, "op": "tombstone"}),
                Some("2026-05-08T00:00:00Z"),
                "tombstone",
            )?,
        ])
    }

    pub fn remote_change_history(&self, domain: PullCursorDomain) -> Result<Vec<i64>> {
        let mut stmt = self.conn.prepare(
            "\
            SELECT remote_change_seq
            FROM l4_remote_owned_fixture
            WHERE domain = ?1
            ORDER BY remote_change_seq
            ",
        )?;
        stmt.query_map(params![domain.as_str()], |row| row.get(0))?
            .collect::<std::result::Result<Vec<_>, _>>()
            .context("listing remote-owned fixture change history")
    }

    pub fn run_sync_cycle(&self, trigger: SyncCycleTrigger) -> Result<SyncCycleOutcome> {
        SyncCycle::new().run(
            trigger,
            |domain| {
                self.pull_fenced(domain, 100)?;
                Ok(())
            },
            || Ok(()),
            || Ok(()),
            || Ok(()),
        )
    }

    pub fn run_sync_cycle_with_pull_failure(
        &self,
        trigger: SyncCycleTrigger,
        failed_domain: PullCursorDomain,
    ) -> Result<SyncCycleOutcome> {
        SyncCycle::new().run(
            trigger,
            |domain| {
                if domain == failed_domain {
                    bail!("simulated remote pull failure for {}", domain.as_str());
                }
                self.pull_fenced(domain, 100)?;
                Ok(())
            },
            || Ok(()),
            || Ok(()),
            || Ok(()),
        )
    }

    pub fn write_local_process_marker(&self, process_id: &str) -> Result<()> {
        self.conn
            .execute(
                "\
                INSERT INTO procurement_processes (
                  id, current_stage, title, latest_published_at, latest_updated_at, owner_id,
                  row_version, stage_run_id
                ) VALUES (?1, 'qualification', 'L4 local degraded marker',
                          '2026-05-08T00:00:00Z', '2026-05-08T00:00:00Z',
                          'local-owner', 1, 'l4-degraded-local-op')
                ",
                params![process_id],
            )
            .context("writing local process marker")?;
        Ok(())
    }

    pub fn local_process_exists(&self, process_id: &str) -> Result<bool> {
        let exists = self
            .conn
            .query_row(
                "SELECT 1 FROM procurement_processes WHERE id = ?1",
                params![process_id],
                |_| Ok(()),
            )
            .optional()
            .context("checking local process marker")?
            .is_some();
        Ok(exists)
    }

    fn append_remote_owned_change(
        &self,
        domain: PullCursorDomain,
        entity_id: &str,
        payload: &Value,
        deleted_at: Option<&str>,
        op: &str,
    ) -> Result<i64> {
        let next_seq = self.remote_hiwater(domain)? + 1;
        self.conn.execute(
            "\
            INSERT INTO l4_remote_owned_fixture (
              domain, entity_id, remote_change_seq, payload_json, deleted_at, op
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            ",
            params![
                domain.as_str(),
                entity_id,
                next_seq,
                payload.to_string(),
                deleted_at,
                op
            ],
        )?;
        Ok(next_seq)
    }

    fn remote_hiwater(&self, domain: PullCursorDomain) -> Result<i64> {
        self.conn
            .query_row(
                "\
                SELECT COALESCE(MAX(remote_change_seq), 0)
                FROM l4_remote_owned_fixture
                WHERE domain = ?1
                ",
                params![domain.as_str()],
                |row| row.get(0),
            )
            .context("reading remote-owned fixture hiwater")
    }

    fn remote_page(
        &self,
        domain: PullCursorDomain,
        lower_exclusive: i64,
        hiwater: i64,
        limit: usize,
    ) -> Result<Vec<RemoteOwnedChange>> {
        let limit = i64::try_from(limit).context("pull page limit exceeds i64")?;
        let mut stmt = self.conn.prepare(
            "\
            SELECT remote_change_seq, entity_id, payload_json, deleted_at
            FROM l4_remote_owned_fixture
            WHERE domain = ?1
              AND remote_change_seq > ?2
              AND remote_change_seq <= ?3
            ORDER BY remote_change_seq
            LIMIT ?4
            ",
        )?;
        stmt.query_map(
            params![domain.as_str(), lower_exclusive, hiwater, limit],
            |row| {
                Ok(RemoteOwnedChange {
                    remote_change_seq: row.get(0)?,
                    entity_id: row.get(1)?,
                    payload_json: row.get(2)?,
                    deleted_at: row.get(3)?,
                })
            },
        )?
        .collect::<std::result::Result<Vec<_>, _>>()
        .context("fetching remote-owned fixture page")
    }

    fn apply_remote_change(
        &self,
        domain: PullCursorDomain,
        change: &RemoteOwnedChange,
    ) -> Result<()> {
        self.conn.execute(
            "\
            INSERT INTO l4_remote_owned_applied (
              domain, remote_change_seq, entity_id, payload_json, deleted_at
            ) VALUES (?1, ?2, ?3, ?4, ?5)
            ",
            params![
                domain.as_str(),
                change.remote_change_seq,
                change.entity_id,
                change.payload_json,
                change.deleted_at
            ],
        )?;
        Ok(())
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
