#![forbid(unsafe_code)]

use anyhow::{Context, Result, bail, ensure};
use reqwest::Client;
use reqwest::header::{ACCEPT, CONTENT_TYPE};
use rusqlite::{Connection, OptionalExtension, Transaction, TransactionBehavior, params};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

const DEFAULT_EXECUTOR_ID: &str = "local-daemon";
const WRITER_LEASE_HEARTBEAT_INTERVAL_SECS: i64 = 30;
const WRITER_LEASE_MIN_TTL_SECS: i64 = 120;
const WRITER_LEASE_NEXT_SAFE_ACTION: &str =
    "switch this install to viewer-only or perform explicit rehome";

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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SyncBatchRow {
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BatchEnvelope {
    pub batch_id: String,
    pub executor_id: String,
    pub stream_id: String,
    pub status: String,
    pub first_local_seq: i64,
    pub last_local_seq: i64,
    pub schema_epoch: i64,
    pub stage_run_id: String,
    pub commit_group_ids: Vec<String>,
    pub writer_epoch: i64,
    pub payload_json: String,
    pub payload_hash: String,
    pub retry_count: i64,
    pub rows: Vec<SyncBatchRow>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BatchEnvelopeAssembler {
    executor_id: String,
    stream_id: String,
    writer_epoch: i64,
    max_rows: usize,
}

#[derive(Clone, Debug)]
pub struct SupabaseRpcClient {
    api_url: String,
    service_role_key: String,
    client: Client,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PushBatchApplyResponse {
    pub accepted: bool,
    pub status: String,
    pub raw: Value,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WriterLease {
    org_id: String,
    executor_id: String,
    heartbeat_interval_secs: i64,
    lease_ttl_secs: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WriterLeaseSnapshot {
    pub org_id: String,
    pub holder_executor_id: Option<String>,
    pub lease_expires_at_epoch_secs: i64,
    pub writer_epoch: i64,
    pub heartbeat_interval_secs: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WriterLeaseConflict {
    pub org_id: String,
    pub current_holder_executor_id: String,
    pub lease_expires_at_epoch_secs: i64,
    pub writer_epoch: i64,
    pub next_safe_action: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WriterLeaseAcquireResult {
    Acquired(WriterLeaseSnapshot),
    Conflict(WriterLeaseConflict),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WriterBindingMode {
    Writer,
    ViewerOnly,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WriterBindingConfig {
    org_id: String,
    executor_id: String,
    requested_mode: WriterBindingMode,
    approved_writer: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WriterBindingDecision {
    pub org_id: String,
    pub executor_id: String,
    pub accepted: bool,
    pub effective_mode: WriterBindingMode,
    pub next_safe_actions: Vec<String>,
}

#[derive(Debug)]
struct WriterLeaseDbRow {
    holder_executor_id: Option<String>,
    lease_expires_at_epoch_secs: Option<i64>,
    writer_epoch: i64,
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

impl WriterLease {
    pub fn new(org_id: impl Into<String>, executor_id: impl Into<String>) -> Result<Self> {
        let org_id = org_id.into();
        let executor_id = executor_id.into();
        ensure_non_empty("org_id", &org_id)?;
        ensure_non_empty("executor_id", &executor_id)?;
        Ok(Self {
            org_id,
            executor_id,
            heartbeat_interval_secs: WRITER_LEASE_HEARTBEAT_INTERVAL_SECS,
            lease_ttl_secs: WRITER_LEASE_MIN_TTL_SECS,
        })
    }

    pub fn acquire(
        &self,
        conn: &mut Connection,
        now_epoch_secs: i64,
    ) -> Result<WriterLeaseAcquireResult> {
        ensure!(now_epoch_secs >= 0, "now_epoch_secs must be non-negative");
        ensure_active_writer_lease_table(conn)?;
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .context("starting writer lease acquisition transaction")?;
        let row = load_writer_lease_row(&tx, &self.org_id)?;
        let new_expiry = now_epoch_secs + self.lease_ttl_secs;
        let (next_epoch, status) = match row {
            None => (1, "acquired"),
            Some(ref current)
                if current.holder_executor_id.as_deref() == Some(&self.executor_id)
                    && current
                        .lease_expires_at_epoch_secs
                        .is_some_and(|expiry| expiry >= now_epoch_secs) =>
            {
                (current.writer_epoch, "renewed")
            }
            Some(ref current)
                if current.holder_executor_id.as_deref() == Some(&self.executor_id)
                    || current.holder_executor_id.is_none()
                    || current
                        .lease_expires_at_epoch_secs
                        .is_none_or(|expiry| expiry < now_epoch_secs) =>
            {
                (current.writer_epoch + 1, "acquired")
            }
            Some(current) => {
                let conflict = self.conflict_from_row(&current);
                tx.commit()
                    .context("committing unchanged writer lease conflict transaction")?;
                return Ok(WriterLeaseAcquireResult::Conflict(conflict));
            }
        };

        upsert_writer_lease_row(&tx, &self.org_id, &self.executor_id, new_expiry, next_epoch)
            .with_context(|| format!("{status} active writer lease"))?;
        tx.commit()
            .context("committing writer lease acquisition transaction")?;
        Ok(WriterLeaseAcquireResult::Acquired(self.snapshot(
            Some(self.executor_id.clone()),
            new_expiry,
            next_epoch,
        )))
    }

    pub fn heartbeat(
        &self,
        conn: &mut Connection,
        now_epoch_secs: i64,
    ) -> Result<WriterLeaseAcquireResult> {
        ensure!(now_epoch_secs >= 0, "now_epoch_secs must be non-negative");
        ensure_active_writer_lease_table(conn)?;
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .context("starting writer lease heartbeat transaction")?;
        let current = load_writer_lease_row(&tx, &self.org_id)?
            .context("writer lease heartbeat requires an existing lease row")?;
        if current.holder_executor_id.as_deref() != Some(&self.executor_id) {
            let conflict = self.conflict_from_row(&current);
            tx.commit()
                .context("committing unchanged writer lease heartbeat conflict")?;
            return Ok(WriterLeaseAcquireResult::Conflict(conflict));
        }
        ensure!(
            current
                .lease_expires_at_epoch_secs
                .is_some_and(|expiry| expiry >= now_epoch_secs),
            "writer lease heartbeat requires an unexpired lease"
        );

        let new_expiry = now_epoch_secs + self.lease_ttl_secs;
        upsert_writer_lease_row(
            &tx,
            &self.org_id,
            &self.executor_id,
            new_expiry,
            current.writer_epoch,
        )
        .context("heartbeating active writer lease")?;
        tx.commit()
            .context("committing writer lease heartbeat transaction")?;
        Ok(WriterLeaseAcquireResult::Acquired(self.snapshot(
            Some(self.executor_id.clone()),
            new_expiry,
            current.writer_epoch,
        )))
    }

    pub fn release(&self, conn: &Connection) -> Result<()> {
        ensure_active_writer_lease_table(conn)?;
        conn.execute(
            "\
            UPDATE active_writer_lease
            SET holder_executor_id = NULL,
                lease_expires_at_epoch_secs = NULL,
                updated_at_epoch_secs = strftime('%s', 'now')
            WHERE org_id = ?1
              AND holder_executor_id = ?2
            ",
            params![self.org_id, self.executor_id],
        )
        .context("releasing active writer lease")?;
        Ok(())
    }

    pub fn current_epoch(&self, conn: &Connection) -> Result<Option<i64>> {
        ensure_active_writer_lease_table(conn)?;
        conn.query_row(
            "SELECT writer_epoch FROM active_writer_lease WHERE org_id = ?1",
            params![self.org_id],
            |row| row.get(0),
        )
        .optional()
        .context("reading active writer lease epoch")
    }

    fn snapshot(
        &self,
        holder_executor_id: Option<String>,
        lease_expires_at_epoch_secs: i64,
        writer_epoch: i64,
    ) -> WriterLeaseSnapshot {
        WriterLeaseSnapshot {
            org_id: self.org_id.clone(),
            holder_executor_id,
            lease_expires_at_epoch_secs,
            writer_epoch,
            heartbeat_interval_secs: self.heartbeat_interval_secs,
        }
    }

    fn conflict_from_row(&self, row: &WriterLeaseDbRow) -> WriterLeaseConflict {
        WriterLeaseConflict {
            org_id: self.org_id.clone(),
            current_holder_executor_id: row.holder_executor_id.clone().unwrap_or_default(),
            lease_expires_at_epoch_secs: row.lease_expires_at_epoch_secs.unwrap_or_default(),
            writer_epoch: row.writer_epoch,
            next_safe_action: WRITER_LEASE_NEXT_SAFE_ACTION.to_owned(),
        }
    }
}

impl WriterBindingConfig {
    pub fn new(
        org_id: impl Into<String>,
        executor_id: impl Into<String>,
        requested_mode: WriterBindingMode,
        approved_writer: bool,
    ) -> Result<Self> {
        let org_id = org_id.into();
        let executor_id = executor_id.into();
        ensure_non_empty("org_id", &org_id)?;
        ensure_non_empty("executor_id", &executor_id)?;
        Ok(Self {
            org_id,
            executor_id,
            requested_mode,
            approved_writer,
        })
    }

    #[must_use]
    pub fn validate(&self) -> WriterBindingDecision {
        if self.requested_mode == WriterBindingMode::Writer && !self.approved_writer {
            return WriterBindingDecision {
                org_id: self.org_id.clone(),
                executor_id: self.executor_id.clone(),
                accepted: false,
                effective_mode: WriterBindingMode::ViewerOnly,
                next_safe_actions: vec![
                    "switch this install to viewer-only".to_owned(),
                    "perform explicit rehome before enabling writer mode".to_owned(),
                ],
            };
        }
        WriterBindingDecision {
            org_id: self.org_id.clone(),
            executor_id: self.executor_id.clone(),
            accepted: true,
            effective_mode: self.requested_mode,
            next_safe_actions: Vec::new(),
        }
    }
}

impl SupabaseRpcClient {
    pub fn new(api_url: impl Into<String>, service_role_key: impl Into<String>) -> Result<Self> {
        let api_url = api_url.into().trim_end_matches('/').to_owned();
        let service_role_key = service_role_key.into();
        ensure_non_empty("api_url", &api_url)?;
        ensure_non_empty("service_role_key", &service_role_key)?;
        Ok(Self {
            api_url,
            service_role_key,
            client: Client::new(),
        })
    }

    pub async fn apply_push_batch(&self, batch: &BatchEnvelope) -> Result<PushBatchApplyResponse> {
        let batch_json = parse_payload(&batch.payload_json)?;
        self.apply_push_batch_json(batch_json, batch.writer_epoch)
            .await
    }

    pub async fn apply_push_batch_json(
        &self,
        batch_json: Value,
        writer_epoch: i64,
    ) -> Result<PushBatchApplyResponse> {
        ensure!(writer_epoch >= 0, "writer_epoch must be non-negative");
        let url = format!("{}/rest/v1/rpc/apply_push_batch", self.api_url);
        let body = json!({
            "batch_json": batch_json,
            "writer_epoch": writer_epoch,
        });
        let response = self
            .client
            .post(url)
            .bearer_auth(&self.service_role_key)
            .header("apikey", &self.service_role_key)
            .header(ACCEPT, "application/json")
            .header(CONTENT_TYPE, "application/json")
            .json(&body)
            .send()
            .await
            .context("calling Supabase apply_push_batch RPC")?;
        let status_code = response.status();
        let value = response
            .json::<Value>()
            .await
            .context("parsing apply_push_batch RPC response")?;
        ensure!(
            status_code.is_success(),
            "apply_push_batch RPC failed with HTTP {status_code}: {value}"
        );
        PushBatchApplyResponse::from_rpc_value(value)
    }
}

impl PushBatchApplyResponse {
    pub fn from_rpc_value(value: Value) -> Result<Self> {
        let accepted = value
            .get("accepted")
            .and_then(Value::as_bool)
            .context("apply_push_batch response missing boolean accepted")?;
        let status = value
            .get("status")
            .and_then(Value::as_str)
            .context("apply_push_batch response missing string status")?
            .to_owned();
        Ok(Self {
            accepted,
            status,
            raw: value,
        })
    }

    #[must_use]
    pub fn error_code(&self) -> Option<&str> {
        self.raw.get("error").and_then(Value::as_str)
    }

    #[must_use]
    pub fn is_stale_writer_rejection(&self) -> bool {
        !self.accepted && self.status == "rejected" && self.error_code() == Some("stale_writer")
    }
}

impl BatchEnvelopeAssembler {
    pub fn new(
        executor_id: impl Into<String>,
        stream_id: impl Into<String>,
        writer_epoch: i64,
        max_rows: usize,
    ) -> Result<Self> {
        let executor_id = executor_id.into();
        let stream_id = stream_id.into();
        ensure_non_empty("executor_id", &executor_id)?;
        ensure_non_empty("stream_id", &stream_id)?;
        ensure!(writer_epoch >= 0, "writer_epoch must be non-negative");
        ensure!(max_rows > 0, "max_rows must be positive");
        Ok(Self {
            executor_id,
            stream_id,
            writer_epoch,
            max_rows,
        })
    }

    pub fn materialize_next_batch(&self, conn: &mut Connection) -> Result<Option<BatchEnvelope>> {
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .context("starting sync batch transaction")?;
        if self.load_active_batch_in(&tx)?.is_some() {
            bail!(
                "stream `{}` already has an in-flight or retry sync batch",
                self.stream_id
            );
        }

        let candidates = load_dispatchable_rows(&tx, &self.executor_id)?;
        if candidates.is_empty() {
            tx.commit()
                .context("committing empty sync batch transaction")?;
            return Ok(None);
        }
        validate_epoch_and_stage_boundaries(&candidates)?;
        let selected = select_batch_rows(&candidates, self.max_rows);
        let batch = self.insert_batch_for_rows(&tx, &selected, "in_flight", 0)?;
        tx.commit().context("committing sync batch envelope")?;
        Ok(Some(batch))
    }

    pub fn load_active_batch(&self, conn: &Connection) -> Result<Option<BatchEnvelope>> {
        self.load_active_batch_in(conn)
    }

    pub fn load_batch(&self, conn: &Connection, batch_id: &str) -> Result<Option<BatchEnvelope>> {
        let batch = conn
            .query_row(
                "\
                SELECT batch_id, executor_id, stream_id, status, first_local_seq, last_local_seq,
                       schema_epoch, stage_run_id, commit_group_ids_json, writer_epoch,
                       payload_json, payload_hash, retry_count
                FROM sync_batches
                WHERE batch_id = ?1
                  AND executor_id = ?2
                  AND stream_id = ?3
                ",
                params![batch_id, self.executor_id, self.stream_id],
                hydrate_batch,
            )
            .optional()
            .context("loading sync batch")?;
        Ok(batch)
    }

    pub fn mark_retry(&self, conn: &Connection, batch_id: &str) -> Result<BatchEnvelope> {
        let before = self
            .load_batch(conn, batch_id)?
            .with_context(|| format!("missing sync batch `{batch_id}`"))?;
        ensure!(
            matches!(before.status.as_str(), "in_flight" | "retry"),
            "only active batches can enter retry state"
        );
        conn.execute(
            "\
            UPDATE sync_batches
            SET status = 'retry',
                retry_count = retry_count + 1,
                last_attempt_at = CURRENT_TIMESTAMP
            WHERE batch_id = ?1
            ",
            params![batch_id],
        )
        .context("marking sync batch retry")?;
        self.load_batch(conn, batch_id)?
            .with_context(|| format!("missing sync batch `{batch_id}` after retry update"))
    }

    pub fn mark_acknowledged(
        &self,
        conn: &Connection,
        batch_id: &str,
        response_json: &str,
    ) -> Result<()> {
        ensure_valid_json_object(response_json)?;
        let updated = conn
            .execute(
                "\
                UPDATE sync_batches
                SET status = 'acknowledged',
                    acknowledged_at = CURRENT_TIMESTAMP,
                    response_json = ?2
                WHERE batch_id = ?1
                  AND executor_id = ?3
                  AND stream_id = ?4
                  AND status IN ('in_flight', 'retry')
                ",
                params![batch_id, response_json, self.executor_id, self.stream_id],
            )
            .context("marking sync batch acknowledged")?;
        ensure!(
            updated == 1,
            "no active sync batch `{batch_id}` to acknowledge"
        );
        Ok(())
    }

    pub fn mark_rejected(
        &self,
        conn: &Connection,
        batch_id: &str,
        response_json: &str,
    ) -> Result<()> {
        ensure_valid_json_object(response_json)?;
        let updated = conn
            .execute(
                "\
                UPDATE sync_batches
                SET status = 'rejected',
                    rejected_at = CURRENT_TIMESTAMP,
                    response_json = ?2
                WHERE batch_id = ?1
                  AND executor_id = ?3
                  AND stream_id = ?4
                  AND status IN ('in_flight', 'retry')
                ",
                params![batch_id, response_json, self.executor_id, self.stream_id],
            )
            .context("marking sync batch rejected")?;
        ensure!(updated == 1, "no active sync batch `{batch_id}` to reject");
        Ok(())
    }

    pub fn record_apply_response(
        &self,
        conn: &mut Connection,
        batch_id: &str,
        response: &PushBatchApplyResponse,
    ) -> Result<Option<BatchEnvelope>> {
        let response_json =
            serde_json::to_string(&response.raw).context("serializing apply response")?;
        if response.accepted {
            self.mark_acknowledged(conn, batch_id, &response_json)?;
            return Ok(None);
        }

        if response.is_stale_writer_rejection() {
            return self
                .re_envelope_after_stale_writer_with_response(conn, batch_id, &response_json)
                .map(Some);
        }

        self.mark_rejected(conn, batch_id, &response_json)?;
        Ok(None)
    }

    pub fn re_envelope_after_stale_writer(
        &self,
        conn: &mut Connection,
        batch_id: &str,
    ) -> Result<BatchEnvelope> {
        self.re_envelope_after_stale_writer_with_response(
            conn,
            batch_id,
            &json!({"error": "stale_writer"}).to_string(),
        )
    }

    fn re_envelope_after_stale_writer_with_response(
        &self,
        conn: &mut Connection,
        batch_id: &str,
        response_json: &str,
    ) -> Result<BatchEnvelope> {
        ensure_valid_json_object(response_json)?;
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .context("starting stale-writer re-envelope transaction")?;
        let stale_batch = self
            .load_batch(&tx, batch_id)?
            .with_context(|| format!("missing stale sync batch `{batch_id}`"))?;
        ensure!(
            matches!(stale_batch.status.as_str(), "in_flight" | "retry"),
            "only active batches can be re-enveloped after stale writer rejection"
        );
        tx.execute(
            "\
            UPDATE sync_batches
            SET status = 'rejected',
                rejected_at = CURRENT_TIMESTAMP,
                response_json = ?2
            WHERE batch_id = ?1
            ",
            params![batch_id, response_json],
        )
        .context("rejecting stale writer batch")?;

        let mut rows = Vec::with_capacity(stale_batch.rows.len());
        for stale_row in &stale_batch.rows {
            rows.push(load_outbox_row_by_seq(&tx, stale_row.local_seq)?);
        }
        let batch = self.insert_batch_for_rows(&tx, &rows, "in_flight", 0)?;
        tx.commit()
            .context("committing stale-writer re-envelope transaction")?;
        Ok(batch)
    }

    fn load_active_batch_in(&self, conn: &Connection) -> Result<Option<BatchEnvelope>> {
        let batch_id = conn
            .query_row(
                "\
                SELECT batch_id
                FROM sync_batches
                WHERE executor_id = ?1
                  AND stream_id = ?2
                  AND status IN ('in_flight', 'retry')
                ORDER BY created_at, first_local_seq
                LIMIT 1
                ",
                params![self.executor_id, self.stream_id],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .context("loading active sync batch id")?;
        batch_id
            .map(|id| self.load_batch(conn, &id))
            .transpose()
            .map(Option::flatten)
    }

    fn insert_batch_for_rows(
        &self,
        conn: &Connection,
        rows: &[SyncBatchRow],
        status: &str,
        retry_count: i64,
    ) -> Result<BatchEnvelope> {
        ensure!(!rows.is_empty(), "cannot create empty sync batch");
        validate_epoch_and_stage_boundaries(rows)?;
        let commit_group_ids = commit_group_ids_for_rows(rows);
        let first_local_seq = rows
            .iter()
            .map(|row| row.local_seq)
            .min()
            .context("batch rows must not be empty")?;
        let last_local_seq = rows
            .iter()
            .map(|row| row.local_seq)
            .max()
            .context("batch rows must not be empty")?;
        let schema_epoch = rows[0].schema_epoch;
        let stage_run_id = rows[0].stage_run_id.clone();
        let envelope_rows = rows
            .iter()
            .map(|row| row_json(row, self.writer_epoch))
            .collect::<Result<Vec<_>>>()?;
        let envelope = json!({
            "executor_id": self.executor_id,
            "stream_id": self.stream_id,
            "schema_epoch": schema_epoch,
            "stage_run_id": stage_run_id,
            "writer_epoch": self.writer_epoch,
            "commit_group_ids": commit_group_ids,
            "rows": envelope_rows,
        });
        let payload_json =
            serde_json::to_string(&envelope).context("serializing sync batch envelope")?;
        let payload_hash = sha256_hex(payload_json.as_bytes());
        let batch_id = format!(
            "{}:{}:{}:{}:{}",
            self.stream_id,
            first_local_seq,
            last_local_seq,
            self.writer_epoch,
            &payload_hash[..16]
        );
        let commit_group_ids_json = serde_json::to_string(&commit_group_ids)
            .context("serializing sync batch commit_group_ids")?;

        conn.execute(
            "\
            INSERT INTO sync_batches (
              batch_id, executor_id, stream_id, status, first_local_seq, last_local_seq,
              schema_epoch, stage_run_id, commit_group_ids_json, writer_epoch, payload_json,
              payload_hash, retry_count, last_attempt_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, CURRENT_TIMESTAMP)
            ",
            params![
                batch_id,
                self.executor_id,
                self.stream_id,
                status,
                first_local_seq,
                last_local_seq,
                schema_epoch,
                stage_run_id,
                commit_group_ids_json,
                self.writer_epoch,
                payload_json,
                payload_hash,
                retry_count,
            ],
        )
        .context("inserting sync_batches envelope")?;

        for row in rows {
            conn.execute(
                "UPDATE local_outbox SET writer_epoch = ?1 WHERE local_seq = ?2",
                params![self.writer_epoch, row.local_seq],
            )
            .with_context(|| format!("assigning writer_epoch for local_seq {}", row.local_seq))?;
        }

        self.load_batch(conn, &batch_id)?
            .with_context(|| format!("missing newly inserted batch `{batch_id}`"))
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

fn ensure_active_writer_lease_table(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "\
        CREATE TABLE IF NOT EXISTS active_writer_lease (
          org_id TEXT PRIMARY KEY,
          holder_executor_id TEXT,
          lease_expires_at_epoch_secs INTEGER,
          writer_epoch INTEGER NOT NULL DEFAULT 0 CHECK (writer_epoch >= 0),
          updated_at_epoch_secs INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
          CHECK (length(trim(org_id)) > 0),
          CHECK (
            holder_executor_id IS NULL
            OR length(trim(holder_executor_id)) > 0
          ),
          CHECK (
            lease_expires_at_epoch_secs IS NULL
            OR lease_expires_at_epoch_secs >= 0
          )
        );
        ",
    )
    .context("ensuring active_writer_lease support table")
}

fn load_writer_lease_row(conn: &Connection, org_id: &str) -> Result<Option<WriterLeaseDbRow>> {
    conn.query_row(
        "\
        SELECT holder_executor_id, lease_expires_at_epoch_secs, writer_epoch
        FROM active_writer_lease
        WHERE org_id = ?1
        ",
        params![org_id],
        |row| {
            Ok(WriterLeaseDbRow {
                holder_executor_id: row.get(0)?,
                lease_expires_at_epoch_secs: row.get(1)?,
                writer_epoch: row.get(2)?,
            })
        },
    )
    .optional()
    .context("loading active writer lease row")
}

fn upsert_writer_lease_row(
    conn: &Connection,
    org_id: &str,
    holder_executor_id: &str,
    lease_expires_at_epoch_secs: i64,
    writer_epoch: i64,
) -> Result<()> {
    conn.execute(
        "\
        INSERT INTO active_writer_lease (
          org_id, holder_executor_id, lease_expires_at_epoch_secs, writer_epoch,
          updated_at_epoch_secs
        ) VALUES (?1, ?2, ?3, ?4, strftime('%s', 'now'))
        ON CONFLICT(org_id) DO UPDATE SET
          holder_executor_id = excluded.holder_executor_id,
          lease_expires_at_epoch_secs = excluded.lease_expires_at_epoch_secs,
          writer_epoch = excluded.writer_epoch,
          updated_at_epoch_secs = excluded.updated_at_epoch_secs
        ",
        params![
            org_id,
            holder_executor_id,
            lease_expires_at_epoch_secs,
            writer_epoch
        ],
    )
    .context("upserting active writer lease row")?;
    Ok(())
}

fn load_dispatchable_rows(conn: &Connection, executor_id: &str) -> Result<Vec<SyncBatchRow>> {
    let mut stmt = conn.prepare(
        "\
        SELECT local_seq, executor_id, entity_kind, entity_id, op, row_version, stage_run_id,
               schema_epoch, commit_group_id, writer_epoch, payload_json, content_hash
        FROM local_outbox
        WHERE executor_id = ?1
          AND writer_epoch IS NULL
        ORDER BY row_version, entity_kind, entity_id, local_seq
        ",
    )?;
    let rows = stmt
        .query_map(params![executor_id], hydrate_outbox_row)?
        .collect::<std::result::Result<Vec<_>, _>>()
        .context("loading dispatchable local_outbox rows")?;
    Ok(rows)
}

fn load_outbox_row_by_seq(conn: &Connection, local_seq: i64) -> Result<SyncBatchRow> {
    conn.query_row(
        "\
        SELECT local_seq, executor_id, entity_kind, entity_id, op, row_version, stage_run_id,
               schema_epoch, commit_group_id, writer_epoch, payload_json, content_hash
        FROM local_outbox
        WHERE local_seq = ?1
        ",
        params![local_seq],
        hydrate_outbox_row,
    )
    .with_context(|| format!("loading local_outbox row {local_seq}"))
}

fn hydrate_outbox_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<SyncBatchRow> {
    Ok(SyncBatchRow {
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
}

fn hydrate_batch(row: &rusqlite::Row<'_>) -> rusqlite::Result<BatchEnvelope> {
    let commit_group_ids_json: String = row.get(8)?;
    let payload_json: String = row.get(10)?;
    let rows = parse_batch_rows(&payload_json).map_err(to_sql_error)?;
    let commit_group_ids =
        serde_json::from_str::<Vec<String>>(&commit_group_ids_json).map_err(to_sql_error)?;
    Ok(BatchEnvelope {
        batch_id: row.get(0)?,
        executor_id: row.get(1)?,
        stream_id: row.get(2)?,
        status: row.get(3)?,
        first_local_seq: row.get(4)?,
        last_local_seq: row.get(5)?,
        schema_epoch: row.get(6)?,
        stage_run_id: row.get(7)?,
        commit_group_ids,
        writer_epoch: row.get(9)?,
        payload_json,
        payload_hash: row.get(11)?,
        retry_count: row.get(12)?,
        rows,
    })
}

fn parse_batch_rows(payload_json: &str) -> Result<Vec<SyncBatchRow>> {
    let payload = parse_payload(payload_json)?;
    let rows = payload
        .get("rows")
        .and_then(Value::as_array)
        .context("sync batch payload_json must contain rows array")?;
    rows.iter().map(parse_batch_row).collect()
}

fn parse_batch_row(value: &Value) -> Result<SyncBatchRow> {
    Ok(SyncBatchRow {
        local_seq: get_i64(value, "local_seq")?,
        executor_id: get_string(value, "executor_id")?,
        entity_kind: get_string(value, "entity_kind")?,
        entity_id: get_string(value, "entity_id")?,
        op: get_string(value, "op")?,
        row_version: get_i64(value, "row_version")?,
        stage_run_id: get_string(value, "stage_run_id")?,
        schema_epoch: get_i64(value, "schema_epoch")?,
        commit_group_id: get_string(value, "commit_group_id")?,
        writer_epoch: Some(get_i64(value, "writer_epoch")?),
        payload_json: serde_json::to_string(
            value
                .get("payload_json")
                .context("batch row missing payload_json")?,
        )
        .context("serializing batch row payload_json")?,
        content_hash: get_string(value, "content_hash")?,
    })
}

fn row_json(row: &SyncBatchRow, writer_epoch: i64) -> Result<Value> {
    Ok(json!({
        "local_seq": row.local_seq,
        "executor_id": row.executor_id,
        "entity_kind": row.entity_kind,
        "entity_id": row.entity_id,
        "op": row.op,
        "row_version": row.row_version,
        "stage_run_id": row.stage_run_id,
        "schema_epoch": row.schema_epoch,
        "commit_group_id": row.commit_group_id,
        "writer_epoch": writer_epoch,
        "payload_json": parse_payload(&row.payload_json)?,
        "content_hash": row.content_hash,
    }))
}

fn validate_epoch_and_stage_boundaries(rows: &[SyncBatchRow]) -> Result<()> {
    let first = rows.first().context("batch must contain rows")?;
    ensure!(first.schema_epoch > 0, "schema_epoch must be positive");
    ensure_non_empty("stage_run_id", &first.stage_run_id)?;
    for row in rows {
        ensure!(
            row.schema_epoch == first.schema_epoch,
            "batch cannot mix schema_epoch values"
        );
        ensure!(
            row.stage_run_id == first.stage_run_id,
            "batch cannot mix stage_run_id values"
        );
    }
    Ok(())
}

fn select_batch_rows(candidates: &[SyncBatchRow], max_rows: usize) -> Vec<SyncBatchRow> {
    let mut commit_group_ids = Vec::<String>::new();
    for row in candidates {
        if !commit_group_ids.contains(&row.commit_group_id) {
            commit_group_ids.push(row.commit_group_id.clone());
        }
    }

    let mut selected = Vec::new();
    for commit_group_id in commit_group_ids {
        let group_rows = candidates
            .iter()
            .filter(|row| row.commit_group_id == commit_group_id)
            .cloned()
            .collect::<Vec<_>>();
        if !selected.is_empty() && selected.len() + group_rows.len() > max_rows {
            break;
        }
        selected.extend(group_rows);
        if selected.len() >= max_rows {
            break;
        }
    }
    selected
}

fn commit_group_ids_for_rows(rows: &[SyncBatchRow]) -> Vec<String> {
    let mut commit_group_ids = Vec::<String>::new();
    for row in rows {
        if !commit_group_ids.contains(&row.commit_group_id) {
            commit_group_ids.push(row.commit_group_id.clone());
        }
    }
    commit_group_ids
}

fn ensure_valid_json_object(response_json: &str) -> Result<()> {
    let value = parse_payload(response_json)?;
    ensure!(value.is_object(), "response_json must be a JSON object");
    Ok(())
}

fn get_i64(value: &Value, field: &str) -> Result<i64> {
    value
        .get(field)
        .and_then(Value::as_i64)
        .with_context(|| format!("batch row missing integer {field}"))
}

fn get_string(value: &Value, field: &str) -> Result<String> {
    let text = value
        .get(field)
        .and_then(Value::as_str)
        .with_context(|| format!("batch row missing string {field}"))?;
    Ok(text.to_owned())
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn to_sql_error(error: impl std::fmt::Display) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(
        0,
        rusqlite::types::Type::Text,
        Box::new(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            error.to_string(),
        )),
    )
}

fn ensure_non_empty(field: &str, value: &str) -> Result<()> {
    ensure!(!value.trim().is_empty(), "{field} must not be empty");
    Ok(())
}
