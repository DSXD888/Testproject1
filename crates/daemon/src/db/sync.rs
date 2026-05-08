#![forbid(unsafe_code)]

use anyhow::{Context, Result, bail, ensure};
use reqwest::Client;
use reqwest::header::{ACCEPT, CONTENT_TYPE};
use rusqlite::{Connection, OptionalExtension, Transaction, TransactionBehavior, params};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const DEFAULT_EXECUTOR_ID: &str = "local-daemon";
const WRITER_LEASE_HEARTBEAT_INTERVAL_SECS: i64 = 30;
const WRITER_LEASE_MIN_TTL_SECS: i64 = 120;
const WRITER_LEASE_NEXT_SAFE_ACTION: &str =
    "switch this install to viewer-only or perform explicit rehome";
const SQLITE_SNAPSHOT_DEFAULT_CADENCE_SECS: i64 = 6 * 60 * 60;
pub const REMOTE_CONFIG_OPERATIONAL_WARN_AFTER_SECS: i64 = 60 * 60;
pub const REMOTE_CONFIG_SAFETY_CRITICAL_TTL_SECS: i64 = 4 * 60 * 60;

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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PullCursorDomain {
    SharedSourceConfigs,
    SharedCompanyProfile,
    OperatorDecisionHeads,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RemoteConfigStalenessSubject {
    SharedCompanyProfile,
    SharedSourceConfigsDisplay,
    SharedSourceConfigsSchedule,
    SharedSourceConfigsEnabled,
    OperatorDecisionHeads,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RemoteConfigStalenessClass {
    Informational,
    Operational,
    SafetyCritical,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RemoteConfigStalenessAction {
    UseCached,
    WarnUseCached,
    DegradeSource,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RemoteConfigStaleness {
    pub subject: RemoteConfigStalenessSubject,
    pub class: RemoteConfigStalenessClass,
    pub action: RemoteConfigStalenessAction,
    pub age_secs: i64,
    pub warn_after_secs: i64,
    pub safety_critical_ttl_secs: i64,
    pub cas_still_required: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PullCursor {
    pub domain: PullCursorDomain,
    pub last_remote_change_seq: i64,
    pub last_pulled_at: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PullCursorStore;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RemoteOwnedChange {
    pub remote_change_seq: i64,
    pub entity_id: String,
    pub payload_json: String,
    pub deleted_at: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FencedPullPageBoundary {
    pub lower_exclusive: i64,
    pub hiwater_inclusive: i64,
    pub limit: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FencedPullOutcome {
    pub domain: PullCursorDomain,
    pub start_cursor: i64,
    pub hiwater: i64,
    pub final_cursor: i64,
    pub page_boundaries: Vec<FencedPullPageBoundary>,
    pub applied_remote_change_seqs: Vec<i64>,
    pub applied_entity_ids: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SyncCycleTrigger {
    Startup,
    Scheduled,
    Manual,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SyncHealth {
    Healthy,
    Degraded,
    Offline,
}

impl SyncHealth {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Healthy => "healthy",
            Self::Degraded => "degraded",
            Self::Offline => "offline",
        }
    }

    #[must_use]
    pub const fn from_last_success(
        now_epoch_secs: i64,
        last_success_epoch_secs: Option<i64>,
        daemon_online: bool,
    ) -> Self {
        if !daemon_online {
            return Self::Offline;
        }
        let Some(last_success_epoch_secs) = last_success_epoch_secs else {
            return Self::Offline;
        };
        let age_secs = now_epoch_secs.saturating_sub(last_success_epoch_secs);
        if age_secs < 90 {
            Self::Healthy
        } else if age_secs <= 300 {
            Self::Degraded
        } else {
            Self::Offline
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ViewerSyncStatus {
    pub org_id: String,
    pub sync_health: SyncHealth,
    pub last_push_success_at_epoch_secs: Option<i64>,
    pub last_push_attempt_at_epoch_secs: Option<i64>,
    pub last_acked_local_seq: Option<i64>,
    pub pending_outbox_count: i64,
    pub schema_epoch: i64,
    pub projection_generation: i64,
    pub daemon_version: String,
    pub updated_at_epoch_secs: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ViewerSyncStatusWriter {
    org_id: String,
    schema_epoch: i64,
    daemon_version: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SchemaEpochStatus {
    pub org_id: String,
    pub paused: bool,
    pub reason: Option<String>,
    pub local_schema_epoch: i64,
    pub remote_schema_epoch: Option<i64>,
    pub dispatch_policy: SyncDispatchPolicy,
    pub local_operation_available: bool,
    pub operator_message: String,
    pub updated_at_epoch_secs: i64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SyncDispatchPolicy {
    PushAndPullAllowed,
    PushAndPullPaused,
}

impl SchemaEpochStatus {
    #[must_use]
    pub const fn push_dispatch_allowed(&self) -> bool {
        matches!(self.dispatch_policy, SyncDispatchPolicy::PushAndPullAllowed)
    }

    #[must_use]
    pub const fn pull_dispatch_allowed(&self) -> bool {
        matches!(self.dispatch_policy, SyncDispatchPolicy::PushAndPullAllowed)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SchemaEpochGuard;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SqliteSnapshotScheduler {
    cadence_secs: i64,
    destination_dir: String,
    encryption_key_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SqliteSnapshotRequest {
    pub source_path: String,
    pub destination_path: String,
    pub encryption_key_id: String,
    pub scheduled_for_epoch_secs: i64,
    pub wal_checkpointed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SqliteSnapshotArtifact {
    pub destination_path: String,
    pub encrypted: bool,
    pub bytes_written: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SqliteSnapshotRecord {
    pub snapshot_id: String,
    pub status: String,
    pub source_path: String,
    pub destination_path: String,
    pub encryption_key_id: String,
    pub scheduled_for_epoch_secs: i64,
    pub started_at_epoch_secs: i64,
    pub completed_at_epoch_secs: i64,
    pub wal_checkpointed: bool,
    pub encrypted: bool,
    pub bytes_written: i64,
    pub warning_code: Option<String>,
    pub warning_message: Option<String>,
    pub local_operation_available: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SyncCyclePhase {
    RemotePullCompleted(PullCursorDomain),
    RemotePullFailed(PullCursorDomain),
    PendingIntentReconciled,
    CasFlushed,
    PushDispatched,
    CrossAuthorityDispatchPaused,
    LocalOperationContinues,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SyncCycleOutcome {
    pub trigger: SyncCycleTrigger,
    pub sync_health: SyncHealth,
    pub phases: Vec<SyncCyclePhase>,
    pub cas_flush_allowed: bool,
    pub push_dispatch_allowed: bool,
    pub local_operation_available: bool,
    pub pull_failure: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SyncCycle {
    remote_domains: Vec<PullCursorDomain>,
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
pub struct OperatorDecisionCommand {
    pub process_id: String,
    pub disposition: String,
    pub base_revision: i64,
    pub command_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperatorDecisionApplyResponse {
    pub accepted: bool,
    pub status: String,
    pub process_id: String,
    pub command_id: String,
    pub applied_revision: Option<i64>,
    pub current_revision: Option<i64>,
    pub current_disposition: Option<String>,
    pub rejected_payload: Option<Value>,
    pub raw: Value,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PendingIntentReconciliationPlan {
    pub superseded_intent_ids: Vec<String>,
    pub conflicted_intent_ids: Vec<String>,
    pub commands_to_flush: Vec<OperatorDecisionCommand>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PendingIntentReconciler;

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

#[derive(Clone, Debug, Eq, PartialEq)]
struct PendingOperatorIntent {
    id: String,
    process_id: String,
    desired_disposition: String,
    base_remote_revision: i64,
    idempotency_key: String,
}

struct SnapshotRunInput {
    status: &'static str,
    source_path: String,
    destination_path: String,
    scheduled_for_epoch_secs: i64,
    started_at_epoch_secs: i64,
    completed_at_epoch_secs: i64,
    wal_checkpointed: bool,
    encrypted: bool,
    bytes_written: i64,
    warning_code: Option<String>,
    warning_message: Option<String>,
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

impl PullCursorDomain {
    #[must_use]
    pub const fn all() -> [Self; 3] {
        [
            Self::SharedSourceConfigs,
            Self::SharedCompanyProfile,
            Self::OperatorDecisionHeads,
        ]
    }

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SharedSourceConfigs => "shared_source_configs",
            Self::SharedCompanyProfile => "shared_company_profile",
            Self::OperatorDecisionHeads => "operator_decision_heads",
        }
    }

    fn from_str(value: &str) -> Result<Self> {
        match value {
            "shared_source_configs" => Ok(Self::SharedSourceConfigs),
            "shared_company_profile" => Ok(Self::SharedCompanyProfile),
            "operator_decision_heads" => Ok(Self::OperatorDecisionHeads),
            _ => bail!("unsupported pull cursor domain `{value}`"),
        }
    }
}

impl RemoteConfigStalenessSubject {
    #[must_use]
    pub const fn staleness_class(self) -> RemoteConfigStalenessClass {
        match self {
            Self::SharedCompanyProfile => RemoteConfigStalenessClass::Informational,
            Self::SharedSourceConfigsDisplay
            | Self::SharedSourceConfigsSchedule
            | Self::OperatorDecisionHeads => RemoteConfigStalenessClass::Operational,
            Self::SharedSourceConfigsEnabled => RemoteConfigStalenessClass::SafetyCritical,
        }
    }

    #[must_use]
    pub const fn cas_still_required(self) -> bool {
        matches!(self, Self::OperatorDecisionHeads)
    }
}

impl RemoteConfigStaleness {
    pub fn evaluate_class(
        subject: RemoteConfigStalenessSubject,
        now_epoch_secs: i64,
        last_pulled_at_epoch_secs: i64,
    ) -> Result<Self> {
        ensure!(
            now_epoch_secs >= 0,
            "remote config staleness now_epoch_secs must be non-negative"
        );
        ensure!(
            last_pulled_at_epoch_secs >= 0,
            "remote config staleness last_pulled_at_epoch_secs must be non-negative"
        );
        let age_secs = now_epoch_secs.saturating_sub(last_pulled_at_epoch_secs);
        Ok(Self::evaluate_age(subject, age_secs))
    }

    #[must_use]
    pub const fn evaluate_age(subject: RemoteConfigStalenessSubject, age_secs: i64) -> Self {
        let class = subject.staleness_class();
        let safe_age_secs = if age_secs < 0 { 0 } else { age_secs };
        let action = match class {
            RemoteConfigStalenessClass::Informational => RemoteConfigStalenessAction::UseCached,
            RemoteConfigStalenessClass::Operational => {
                if safe_age_secs >= REMOTE_CONFIG_OPERATIONAL_WARN_AFTER_SECS {
                    RemoteConfigStalenessAction::WarnUseCached
                } else {
                    RemoteConfigStalenessAction::UseCached
                }
            }
            RemoteConfigStalenessClass::SafetyCritical => {
                if safe_age_secs >= REMOTE_CONFIG_SAFETY_CRITICAL_TTL_SECS {
                    RemoteConfigStalenessAction::DegradeSource
                } else if safe_age_secs >= REMOTE_CONFIG_OPERATIONAL_WARN_AFTER_SECS {
                    RemoteConfigStalenessAction::WarnUseCached
                } else {
                    RemoteConfigStalenessAction::UseCached
                }
            }
        };
        Self {
            subject,
            class,
            action,
            age_secs: safe_age_secs,
            warn_after_secs: REMOTE_CONFIG_OPERATIONAL_WARN_AFTER_SECS,
            safety_critical_ttl_secs: REMOTE_CONFIG_SAFETY_CRITICAL_TTL_SECS,
            cas_still_required: subject.cas_still_required(),
        }
    }

    #[must_use]
    pub const fn warns(&self) -> bool {
        matches!(self.action, RemoteConfigStalenessAction::WarnUseCached)
    }

    #[must_use]
    pub const fn degrades_source(&self) -> bool {
        matches!(self.action, RemoteConfigStalenessAction::DegradeSource)
    }
}

impl PullCursorStore {
    pub fn initialize(conn: &Connection) -> Result<()> {
        ensure_pull_cursors_table(conn)?;
        for domain in PullCursorDomain::all() {
            conn.execute(
                "INSERT OR IGNORE INTO pull_cursors (domain) VALUES (?1)",
                params![domain.as_str()],
            )
            .with_context(|| format!("initializing pull cursor {}", domain.as_str()))?;
        }
        Ok(())
    }

    pub fn all(conn: &Connection) -> Result<Vec<PullCursor>> {
        Self::initialize(conn)?;
        let mut stmt = conn
            .prepare(
                "\
                SELECT domain, last_remote_change_seq, last_pulled_at
                FROM pull_cursors
                ORDER BY domain COLLATE BINARY
                ",
            )
            .context("preparing pull cursor listing")?;
        stmt.query_map([], hydrate_pull_cursor)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .context("listing pull cursors")
    }

    pub fn load(conn: &Connection, domain: PullCursorDomain) -> Result<PullCursor> {
        Self::initialize(conn)?;
        conn.query_row(
            "\
            SELECT domain, last_remote_change_seq, last_pulled_at
            FROM pull_cursors
            WHERE domain = ?1
            ",
            params![domain.as_str()],
            hydrate_pull_cursor,
        )
        .with_context(|| format!("loading pull cursor {}", domain.as_str()))
    }

    pub fn record_success(
        conn: &Connection,
        domain: PullCursorDomain,
        remote_change_seq: i64,
    ) -> Result<PullCursor> {
        ensure!(
            remote_change_seq >= 0,
            "remote_change_seq must be non-negative"
        );
        let current = Self::load(conn, domain)?;
        ensure!(
            remote_change_seq >= current.last_remote_change_seq,
            "pull cursor {} cannot move backward from {} to {}",
            domain.as_str(),
            current.last_remote_change_seq,
            remote_change_seq
        );
        conn.execute(
            "\
            UPDATE pull_cursors
            SET last_remote_change_seq = ?2,
                last_pulled_at = CURRENT_TIMESTAMP,
                updated_at = CURRENT_TIMESTAMP
            WHERE domain = ?1
            ",
            params![domain.as_str(), remote_change_seq],
        )
        .with_context(|| format!("advancing pull cursor {}", domain.as_str()))?;
        Self::load(conn, domain)
    }

    pub fn record_failure(conn: &Connection, domain: PullCursorDomain) -> Result<PullCursor> {
        Self::load(conn, domain)
    }

    pub fn apply_fenced_pull<ReadHiwater, FetchPage, ApplyRow>(
        conn: &Connection,
        domain: PullCursorDomain,
        page_size: usize,
        mut read_hiwater: ReadHiwater,
        mut fetch_page: FetchPage,
        mut apply_row: ApplyRow,
    ) -> Result<FencedPullOutcome>
    where
        ReadHiwater: FnMut(PullCursorDomain) -> Result<i64>,
        FetchPage: FnMut(PullCursorDomain, i64, i64, usize) -> Result<Vec<RemoteOwnedChange>>,
        ApplyRow: FnMut(PullCursorDomain, &RemoteOwnedChange) -> Result<()>,
    {
        ensure!(page_size > 0, "fenced pull page_size must be positive");
        let start_cursor = Self::load(conn, domain)?.last_remote_change_seq;
        let hiwater = read_hiwater(domain)
            .with_context(|| format!("reading pull hiwater for {}", domain.as_str()))?;
        ensure!(
            hiwater >= start_cursor,
            "pull hiwater {} for {} is behind cursor {}",
            hiwater,
            domain.as_str(),
            start_cursor
        );

        let mut lower_exclusive = start_cursor;
        let mut page_boundaries = Vec::new();
        let mut applied_remote_change_seqs = Vec::new();
        let mut applied_entity_ids = Vec::new();

        while lower_exclusive < hiwater {
            page_boundaries.push(FencedPullPageBoundary {
                lower_exclusive,
                hiwater_inclusive: hiwater,
                limit: page_size,
            });
            let page =
                fetch_page(domain, lower_exclusive, hiwater, page_size).with_context(|| {
                    format!(
                        "fetching pull page for {} in ({}, {}]",
                        domain.as_str(),
                        lower_exclusive,
                        hiwater
                    )
                })?;
            ensure!(
                page.len() <= page_size,
                "pull page for {} returned {} rows over limit {}",
                domain.as_str(),
                page.len(),
                page_size
            );
            if page.is_empty() {
                break;
            }

            for change in page {
                ensure!(
                    change.remote_change_seq > lower_exclusive
                        && change.remote_change_seq <= hiwater,
                    "remote change {} for {} escaped fenced bounds ({}, {}]",
                    change.remote_change_seq,
                    domain.as_str(),
                    lower_exclusive,
                    hiwater
                );
                apply_row(domain, &change).with_context(|| {
                    format!(
                        "applying remote-owned change {} for {}",
                        change.remote_change_seq,
                        domain.as_str()
                    )
                })?;
                lower_exclusive = change.remote_change_seq;
                applied_remote_change_seqs.push(change.remote_change_seq);
                applied_entity_ids.push(change.entity_id);
            }
        }

        let final_cursor = Self::record_success(conn, domain, hiwater)?.last_remote_change_seq;
        Ok(FencedPullOutcome {
            domain,
            start_cursor,
            hiwater,
            final_cursor,
            page_boundaries,
            applied_remote_change_seqs,
            applied_entity_ids,
        })
    }
}

impl Default for SyncCycle {
    fn default() -> Self {
        Self::new()
    }
}

impl SyncCycle {
    #[must_use]
    pub fn new() -> Self {
        Self {
            remote_domains: PullCursorDomain::all().to_vec(),
        }
    }

    pub fn with_remote_domains(remote_domains: Vec<PullCursorDomain>) -> Result<Self> {
        ensure!(
            !remote_domains.is_empty(),
            "sync cycle requires at least one remote-owned pull domain"
        );
        Ok(Self { remote_domains })
    }

    pub fn run<PullRemote, ReconcilePendingIntents, FlushCas, DispatchPush>(
        &self,
        trigger: SyncCycleTrigger,
        mut pull_remote: PullRemote,
        mut reconcile_pending_intents: ReconcilePendingIntents,
        mut flush_cas: FlushCas,
        mut dispatch_push: DispatchPush,
    ) -> Result<SyncCycleOutcome>
    where
        PullRemote: FnMut(PullCursorDomain) -> Result<()>,
        ReconcilePendingIntents: FnMut() -> Result<()>,
        FlushCas: FnMut() -> Result<()>,
        DispatchPush: FnMut() -> Result<()>,
    {
        let mut phases = Vec::with_capacity(self.remote_domains.len() + 3);
        for &domain in &self.remote_domains {
            match pull_remote(domain) {
                Ok(()) => phases.push(SyncCyclePhase::RemotePullCompleted(domain)),
                Err(error) => {
                    phases.push(SyncCyclePhase::RemotePullFailed(domain));
                    phases.push(SyncCyclePhase::CrossAuthorityDispatchPaused);
                    phases.push(SyncCyclePhase::LocalOperationContinues);
                    return Ok(SyncCycleOutcome {
                        trigger,
                        sync_health: SyncHealth::Degraded,
                        phases,
                        cas_flush_allowed: false,
                        push_dispatch_allowed: false,
                        local_operation_available: true,
                        pull_failure: Some(error.to_string()),
                    });
                }
            }
        }

        reconcile_pending_intents()
            .context("reconciling pending operator intents after remote-owned pulls")?;
        phases.push(SyncCyclePhase::PendingIntentReconciled);
        flush_cas().context("flushing CAS commands after remote-owned pulls")?;
        phases.push(SyncCyclePhase::CasFlushed);
        dispatch_push().context("dispatching push batches after remote-owned pulls")?;
        phases.push(SyncCyclePhase::PushDispatched);

        Ok(SyncCycleOutcome {
            trigger,
            sync_health: SyncHealth::Healthy,
            phases,
            cas_flush_allowed: true,
            push_dispatch_allowed: true,
            local_operation_available: true,
            pull_failure: None,
        })
    }
}

impl ViewerSyncStatusWriter {
    pub fn new(
        org_id: impl Into<String>,
        schema_epoch: i64,
        daemon_version: impl Into<String>,
    ) -> Result<Self> {
        let org_id = org_id.into();
        let daemon_version = daemon_version.into();
        ensure_non_empty("org_id", &org_id)?;
        ensure!(schema_epoch > 0, "schema_epoch must be positive");
        ensure_non_empty("daemon_version", &daemon_version)?;
        Ok(Self {
            org_id,
            schema_epoch,
            daemon_version,
        })
    }

    pub fn for_default_org(schema_epoch: i64) -> Result<Self> {
        Self::new("default", schema_epoch, env!("CARGO_PKG_VERSION"))
    }

    pub fn record_apply_success(
        &self,
        conn: &Connection,
        now_epoch_secs: i64,
        last_acked_local_seq: i64,
        projection_generation: i64,
    ) -> Result<ViewerSyncStatus> {
        ensure!(now_epoch_secs >= 0, "now_epoch_secs must be non-negative");
        ensure!(
            last_acked_local_seq > 0,
            "last_acked_local_seq must be positive"
        );
        ensure!(
            projection_generation >= 0,
            "projection_generation must be non-negative"
        );
        let pending_outbox_count = pending_outbox_count(conn)?;
        upsert_viewer_sync_status(
            conn,
            &ViewerSyncStatus {
                org_id: self.org_id.clone(),
                sync_health: SyncHealth::Healthy,
                last_push_success_at_epoch_secs: Some(now_epoch_secs),
                last_push_attempt_at_epoch_secs: Some(now_epoch_secs),
                last_acked_local_seq: Some(last_acked_local_seq),
                pending_outbox_count,
                schema_epoch: self.schema_epoch,
                projection_generation,
                daemon_version: self.daemon_version.clone(),
                updated_at_epoch_secs: now_epoch_secs,
            },
        )?;
        self.load(conn)?
            .with_context(|| format!("missing viewer_sync_status row for org `{}`", self.org_id))
    }

    pub fn record_heartbeat_status(
        &self,
        conn: &Connection,
        now_epoch_secs: i64,
        projection_generation: i64,
    ) -> Result<ViewerSyncStatus> {
        ensure!(now_epoch_secs >= 0, "now_epoch_secs must be non-negative");
        ensure!(
            projection_generation >= 0,
            "projection_generation must be non-negative"
        );
        ensure_viewer_sync_status_table(conn)?;
        let current = self.load(conn)?;
        let last_push_success_at_epoch_secs = current
            .as_ref()
            .and_then(|status| status.last_push_success_at_epoch_secs);
        let last_acked_local_seq = current
            .as_ref()
            .and_then(|status| status.last_acked_local_seq);
        let sync_health =
            SyncHealth::from_last_success(now_epoch_secs, last_push_success_at_epoch_secs, true);
        let pending_outbox_count = pending_outbox_count(conn)?;
        upsert_viewer_sync_status(
            conn,
            &ViewerSyncStatus {
                org_id: self.org_id.clone(),
                sync_health,
                last_push_success_at_epoch_secs,
                last_push_attempt_at_epoch_secs: Some(now_epoch_secs),
                last_acked_local_seq,
                pending_outbox_count,
                schema_epoch: self.schema_epoch,
                projection_generation,
                daemon_version: self.daemon_version.clone(),
                updated_at_epoch_secs: now_epoch_secs,
            },
        )?;
        self.load(conn)?
            .with_context(|| format!("missing viewer_sync_status row for org `{}`", self.org_id))
    }

    pub fn record_sync_paused(
        &self,
        conn: &Connection,
        now_epoch_secs: i64,
        projection_generation: i64,
    ) -> Result<ViewerSyncStatus> {
        ensure!(now_epoch_secs >= 0, "now_epoch_secs must be non-negative");
        ensure!(
            projection_generation >= 0,
            "projection_generation must be non-negative"
        );
        ensure_viewer_sync_status_table(conn)?;
        let current = self.load(conn)?;
        let pending_outbox_count = pending_outbox_count(conn)?;
        upsert_viewer_sync_status(
            conn,
            &ViewerSyncStatus {
                org_id: self.org_id.clone(),
                sync_health: SyncHealth::Offline,
                last_push_success_at_epoch_secs: current
                    .as_ref()
                    .and_then(|status| status.last_push_success_at_epoch_secs),
                last_push_attempt_at_epoch_secs: Some(now_epoch_secs),
                last_acked_local_seq: current
                    .as_ref()
                    .and_then(|status| status.last_acked_local_seq),
                pending_outbox_count,
                schema_epoch: self.schema_epoch,
                projection_generation,
                daemon_version: self.daemon_version.clone(),
                updated_at_epoch_secs: now_epoch_secs,
            },
        )?;
        self.load(conn)?
            .with_context(|| format!("missing viewer_sync_status row for org `{}`", self.org_id))
    }

    pub fn load(&self, conn: &Connection) -> Result<Option<ViewerSyncStatus>> {
        ensure_viewer_sync_status_table(conn)?;
        conn.query_row(
            "\
            SELECT org_id, sync_health, last_push_success_at_epoch_secs,
                   last_push_attempt_at_epoch_secs, last_acked_local_seq, pending_outbox_count,
                   schema_epoch, projection_generation, daemon_version, updated_at_epoch_secs
            FROM viewer_sync_status
            WHERE org_id = ?1
            ",
            params![self.org_id],
            hydrate_viewer_sync_status,
        )
        .optional()
        .context("loading viewer_sync_status row")
    }
}

impl SchemaEpochGuard {
    pub fn local_schema_epoch(conn: &Connection) -> Result<i64> {
        conn.pragma_query_value(None, "user_version", |row| row.get(0))
            .context("reading local SQLite PRAGMA user_version")
    }

    pub fn record_remote_epoch_check(
        conn: &Connection,
        org_id: &str,
        remote_schema_epoch: Option<i64>,
        now_epoch_secs: i64,
        projection_generation: i64,
        daemon_version: &str,
    ) -> Result<SchemaEpochStatus> {
        ensure_non_empty("org_id", org_id)?;
        ensure!(now_epoch_secs >= 0, "now_epoch_secs must be non-negative");
        let local_schema_epoch = Self::local_schema_epoch(conn)?;
        ensure!(
            local_schema_epoch > 0,
            "local schema_epoch must be positive"
        );
        if let Some(remote_schema_epoch) = remote_schema_epoch {
            ensure!(
                remote_schema_epoch > 0,
                "remote schema_epoch must be positive when present"
            );
        }

        if remote_schema_epoch == Some(local_schema_epoch) {
            if let Some(paused) = Self::load(conn, org_id)?
                && paused.paused
            {
                return Ok(paused);
            }
            return Ok(SchemaEpochStatus {
                org_id: org_id.to_owned(),
                paused: false,
                reason: None,
                local_schema_epoch,
                remote_schema_epoch,
                dispatch_policy: SyncDispatchPolicy::PushAndPullAllowed,
                local_operation_available: true,
                operator_message: "schema epochs match; sync may proceed".to_owned(),
                updated_at_epoch_secs: now_epoch_secs,
            });
        }

        Self::pause_for_mismatch(
            conn,
            org_id,
            local_schema_epoch,
            remote_schema_epoch,
            now_epoch_secs,
            projection_generation,
            daemon_version,
        )
    }

    pub fn pause_for_mismatch(
        conn: &Connection,
        org_id: &str,
        local_schema_epoch: i64,
        remote_schema_epoch: Option<i64>,
        now_epoch_secs: i64,
        projection_generation: i64,
        daemon_version: &str,
    ) -> Result<SchemaEpochStatus> {
        ensure_non_empty("org_id", org_id)?;
        ensure!(
            local_schema_epoch > 0,
            "local schema_epoch must be positive"
        );
        ensure!(now_epoch_secs >= 0, "now_epoch_secs must be non-negative");
        if let Some(remote_schema_epoch) = remote_schema_epoch {
            ensure!(
                remote_schema_epoch > 0,
                "remote schema_epoch must be positive when present"
            );
        }

        ensure_sync_pause_state_table(conn)?;
        let operator_message = remote_schema_epoch.map_or_else(
            || {
                format!(
                    "sync paused: remote schema_epoch is unavailable while local schema_epoch is {local_schema_epoch}; run epoch repair and reproject before resuming remote writes"
                )
            },
            |remote| {
                format!(
                "sync paused: local schema_epoch {local_schema_epoch} does not match remote schema_epoch {remote}; run epoch repair and reproject before resuming remote writes"
                )
            },
        );
        conn.execute(
            "\
            INSERT INTO sync_pause_state (
              org_id, paused, reason, local_schema_epoch, remote_schema_epoch,
              operator_message, updated_at_epoch_secs
            ) VALUES (?1, 1, 'schema_epoch_mismatch', ?2, ?3, ?4, ?5)
            ON CONFLICT(org_id) DO UPDATE SET
              paused = 1,
              reason = 'schema_epoch_mismatch',
              local_schema_epoch = excluded.local_schema_epoch,
              remote_schema_epoch = excluded.remote_schema_epoch,
              operator_message = excluded.operator_message,
              updated_at_epoch_secs = excluded.updated_at_epoch_secs
            ",
            params![
                org_id,
                local_schema_epoch,
                remote_schema_epoch,
                operator_message,
                now_epoch_secs
            ],
        )
        .context("recording schema epoch sync pause")?;
        ViewerSyncStatusWriter::new(org_id, local_schema_epoch, daemon_version)?
            .record_sync_paused(conn, now_epoch_secs, projection_generation)?;
        Self::load(conn, org_id)?.context("missing schema epoch pause state after update")
    }

    pub fn load(conn: &Connection, org_id: &str) -> Result<Option<SchemaEpochStatus>> {
        ensure_non_empty("org_id", org_id)?;
        ensure_sync_pause_state_table(conn)?;
        conn.query_row(
            "\
            SELECT org_id, paused, reason, local_schema_epoch, remote_schema_epoch,
                   operator_message, updated_at_epoch_secs
            FROM sync_pause_state
            WHERE org_id = ?1
            ",
            params![org_id],
            hydrate_schema_epoch_status,
        )
        .optional()
        .context("loading schema epoch sync pause state")
    }

    pub fn is_sync_paused(conn: &Connection, org_id: &str) -> Result<bool> {
        Ok(Self::load(conn, org_id)?.is_some_and(|status| status.paused))
    }
}

impl SqliteSnapshotScheduler {
    pub fn new(
        cadence_secs: i64,
        destination_dir: impl Into<String>,
        encryption_key_id: impl Into<String>,
    ) -> Result<Self> {
        let destination_dir = destination_dir.into();
        let encryption_key_id = encryption_key_id.into();
        ensure!(cadence_secs > 0, "snapshot cadence must be positive");
        ensure_non_empty("destination_dir", &destination_dir)?;
        ensure_non_empty("encryption_key_id", &encryption_key_id)?;
        Ok(Self {
            cadence_secs,
            destination_dir,
            encryption_key_id,
        })
    }

    pub fn default_cadence(
        destination_dir: impl Into<String>,
        encryption_key_id: impl Into<String>,
    ) -> Result<Self> {
        Self::new(
            SQLITE_SNAPSHOT_DEFAULT_CADENCE_SECS,
            destination_dir,
            encryption_key_id,
        )
    }

    pub fn run_due<F>(
        &self,
        conn: &Connection,
        source_path: &Path,
        now_epoch_secs: i64,
        mut run_snapshot: F,
    ) -> Result<SqliteSnapshotRecord>
    where
        F: FnMut(&SqliteSnapshotRequest) -> Result<SqliteSnapshotArtifact>,
    {
        ensure!(now_epoch_secs >= 0, "now_epoch_secs must be non-negative");
        ensure_sqlite_snapshot_runs_table(conn)?;
        let source_path = source_path.display().to_string();
        if let Some(last_attempt) = latest_sqlite_snapshot_attempt(conn)?
            && now_epoch_secs < last_attempt + self.cadence_secs
        {
            return self.record_snapshot_run(
                conn,
                &SnapshotRunInput {
                    status: "skipped",
                    source_path,
                    destination_path: String::new(),
                    scheduled_for_epoch_secs: now_epoch_secs,
                    started_at_epoch_secs: now_epoch_secs,
                    completed_at_epoch_secs: now_epoch_secs,
                    wal_checkpointed: false,
                    encrypted: false,
                    bytes_written: 0,
                    warning_code: None,
                    warning_message: None,
                },
            );
        }

        let destination_path = self.destination_path(&source_path, now_epoch_secs);
        let wal_checkpointed = checkpoint_wal_passive(conn)?;
        let request = SqliteSnapshotRequest {
            source_path: source_path.clone(),
            destination_path: destination_path.clone(),
            encryption_key_id: self.encryption_key_id.clone(),
            scheduled_for_epoch_secs: now_epoch_secs,
            wal_checkpointed,
        };
        let result = run_snapshot(&request);
        match result {
            Ok(artifact) if artifact.encrypted => self.record_snapshot_run(
                conn,
                &SnapshotRunInput {
                    status: "succeeded",
                    source_path,
                    destination_path: artifact.destination_path,
                    scheduled_for_epoch_secs: now_epoch_secs,
                    started_at_epoch_secs: now_epoch_secs,
                    completed_at_epoch_secs: now_epoch_secs,
                    wal_checkpointed,
                    encrypted: true,
                    bytes_written: artifact.bytes_written,
                    warning_code: None,
                    warning_message: None,
                },
            ),
            Ok(artifact) => self.record_snapshot_run(
                conn,
                &SnapshotRunInput {
                    status: "failed",
                    source_path,
                    destination_path: artifact.destination_path,
                    scheduled_for_epoch_secs: now_epoch_secs,
                    started_at_epoch_secs: now_epoch_secs,
                    completed_at_epoch_secs: now_epoch_secs,
                    wal_checkpointed,
                    encrypted: false,
                    bytes_written: artifact.bytes_written,
                    warning_code: Some("SQLITE_SNAPSHOT_UNENCRYPTED".to_owned()),
                    warning_message: Some(
                        "snapshot runner returned an unencrypted artifact".to_owned(),
                    ),
                },
            ),
            Err(error) => self.record_snapshot_run(
                conn,
                &SnapshotRunInput {
                    status: "failed",
                    source_path,
                    destination_path,
                    scheduled_for_epoch_secs: now_epoch_secs,
                    started_at_epoch_secs: now_epoch_secs,
                    completed_at_epoch_secs: now_epoch_secs,
                    wal_checkpointed,
                    encrypted: false,
                    bytes_written: 0,
                    warning_code: Some("SQLITE_SNAPSHOT_FAILED".to_owned()),
                    warning_message: Some(error.to_string()),
                },
            ),
        }
    }

    pub fn records(conn: &Connection) -> Result<Vec<SqliteSnapshotRecord>> {
        ensure_sqlite_snapshot_runs_table(conn)?;
        let mut stmt = conn
            .prepare(
                "\
                SELECT snapshot_id, status, source_path, destination_path, encryption_key_id,
                       scheduled_for_epoch_secs, started_at_epoch_secs, completed_at_epoch_secs,
                       wal_checkpointed, encrypted, bytes_written, warning_code, warning_message,
                       local_operation_available
                FROM local_sqlite_snapshot_runs
                ORDER BY scheduled_for_epoch_secs, snapshot_id
                ",
            )
            .context("preparing SQLite snapshot run listing")?;
        stmt.query_map([], hydrate_sqlite_snapshot_record)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .context("listing SQLite snapshot runs")
    }

    fn destination_path(&self, source_path: &str, now_epoch_secs: i64) -> String {
        let source_name = Path::new(source_path)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("state.sqlite");
        PathBuf::from(&self.destination_dir)
            .join(format!("{now_epoch_secs}-{source_name}.sqlite.enc"))
            .display()
            .to_string()
    }

    fn record_snapshot_run(
        &self,
        conn: &Connection,
        input: &SnapshotRunInput,
    ) -> Result<SqliteSnapshotRecord> {
        ensure!(
            matches!(input.status, "succeeded" | "failed" | "skipped"),
            "unsupported snapshot status `{}`",
            input.status
        );
        let snapshot_id = format!(
            "{}:{}:{}",
            input.scheduled_for_epoch_secs,
            input.status,
            sha256_hex(input.destination_path.as_bytes())
                .chars()
                .take(12)
                .collect::<String>()
        );
        conn.execute(
            "\
            INSERT INTO local_sqlite_snapshot_runs (
              snapshot_id, status, source_path, destination_path, encryption_key_id,
              scheduled_for_epoch_secs, started_at_epoch_secs, completed_at_epoch_secs,
              wal_checkpointed, encrypted, bytes_written, warning_code, warning_message,
              local_operation_available
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, 1)
            ",
            params![
                snapshot_id,
                input.status,
                input.source_path,
                input.destination_path,
                self.encryption_key_id,
                input.scheduled_for_epoch_secs,
                input.started_at_epoch_secs,
                input.completed_at_epoch_secs,
                bool_to_i64(input.wal_checkpointed),
                bool_to_i64(input.encrypted),
                input.bytes_written,
                input.warning_code.as_deref(),
                input.warning_message.as_deref(),
            ],
        )
        .context("recording SQLite snapshot run")?;
        Self::records(conn)?
            .into_iter()
            .find(|record| record.snapshot_id == snapshot_id)
            .with_context(|| format!("missing recorded SQLite snapshot run `{snapshot_id}`"))
    }
}

impl SqliteSnapshotArtifact {
    #[must_use]
    pub fn encrypted(destination_path: impl Into<String>, bytes_written: i64) -> Self {
        Self {
            destination_path: destination_path.into(),
            encrypted: true,
            bytes_written,
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

    pub async fn update_operator_decision(
        &self,
        command: &OperatorDecisionCommand,
    ) -> Result<OperatorDecisionApplyResponse> {
        command.validate()?;
        let url = format!("{}/rest/v1/rpc/update_operator_decision", self.api_url);
        let body = json!({
            "process_id": command.process_id,
            "disposition": command.disposition,
            "base_revision": command.base_revision,
            "command_id": command.command_id,
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
            .context("calling Supabase update_operator_decision RPC")?;
        let status_code = response.status();
        let value = response
            .json::<Value>()
            .await
            .context("parsing update_operator_decision RPC response")?;
        ensure!(
            status_code.is_success(),
            "update_operator_decision RPC failed with HTTP {status_code}: {value}"
        );
        OperatorDecisionApplyResponse::from_rpc_value(value)
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

    #[must_use]
    pub fn is_schema_epoch_mismatch(&self) -> bool {
        !self.accepted
            && self.status == "rejected"
            && self.error_code() == Some("schema_epoch_mismatch")
    }

    #[must_use]
    pub fn remote_schema_epoch(&self) -> Option<i64> {
        self.raw.get("remote_schema_epoch").and_then(Value::as_i64)
    }
}

impl OperatorDecisionCommand {
    pub fn new(
        process_id: impl Into<String>,
        disposition: impl Into<String>,
        base_revision: i64,
        command_id: impl Into<String>,
    ) -> Result<Self> {
        let command = Self {
            process_id: process_id.into(),
            disposition: disposition.into(),
            base_revision,
            command_id: command_id.into(),
        };
        command.validate()?;
        Ok(command)
    }

    pub fn validate(&self) -> Result<()> {
        ensure_non_empty("process_id", &self.process_id)?;
        ensure_non_empty("disposition", &self.disposition)?;
        ensure_non_empty("command_id", &self.command_id)?;
        ensure!(
            matches!(
                self.disposition.as_str(),
                "pending" | "watching" | "approved" | "rejected"
            ),
            "invalid operator decision disposition `{}`",
            self.disposition
        );
        ensure!(
            self.base_revision >= 0,
            "base_revision must be a non-negative remote revision"
        );
        Ok(())
    }
}

impl OperatorDecisionApplyResponse {
    pub fn from_rpc_value(value: Value) -> Result<Self> {
        let accepted = value
            .get("accepted")
            .and_then(Value::as_bool)
            .context("update_operator_decision response missing boolean accepted")?;
        let status = value
            .get("status")
            .and_then(Value::as_str)
            .context("update_operator_decision response missing string status")?
            .to_owned();
        let process_id = value
            .get("process_id")
            .and_then(Value::as_str)
            .context("update_operator_decision response missing string process_id")?
            .to_owned();
        let command_id = value
            .get("command_id")
            .and_then(Value::as_str)
            .context("update_operator_decision response missing string command_id")?
            .to_owned();
        let applied_revision = value.get("applied_revision").and_then(Value::as_i64);
        let current_revision = value.get("current_revision").and_then(Value::as_i64);
        let current_disposition = value
            .get("current_disposition")
            .and_then(Value::as_str)
            .map(str::to_owned);
        let rejected_payload = value.get("rejected_payload").cloned();

        Ok(Self {
            accepted,
            status,
            process_id,
            command_id,
            applied_revision,
            current_revision,
            current_disposition,
            rejected_payload,
            raw: value,
        })
    }

    #[must_use]
    pub fn is_conflict(&self) -> bool {
        !self.accepted && self.status == "conflict"
    }
}

impl PendingIntentReconciler {
    pub fn prepare_commands<RemoteRevision>(
        conn: &mut Connection,
        mut remote_revision_for_process: RemoteRevision,
    ) -> Result<PendingIntentReconciliationPlan>
    where
        RemoteRevision: FnMut(&str) -> Result<i64>,
    {
        let (superseded_intent_ids, candidates) = coalesce_pending_operator_intents(conn)?;
        let mut plan = PendingIntentReconciliationPlan {
            superseded_intent_ids,
            conflicted_intent_ids: Vec::new(),
            commands_to_flush: Vec::new(),
        };

        for intent in candidates {
            let current_remote_revision = remote_revision_for_process(&intent.process_id)
                .with_context(|| format!("loading remote decision head `{}`", intent.process_id))?;
            if current_remote_revision != intent.base_remote_revision {
                let conflict_payload = json!({
                    "kind": "remote_revision_conflict",
                    "process_id": &intent.process_id,
                    "intent_id": &intent.id,
                    "command_id": &intent.idempotency_key,
                    "base_remote_revision": intent.base_remote_revision,
                    "current_remote_revision": current_remote_revision,
                });
                mark_pending_intent_conflicted(conn, &intent, &conflict_payload)?;
                plan.conflicted_intent_ids.push(intent.id);
                continue;
            }

            plan.commands_to_flush.push(OperatorDecisionCommand::new(
                intent.process_id,
                remote_disposition_for_desired(&intent.desired_disposition)?,
                intent.base_remote_revision,
                intent.idempotency_key,
            )?);
        }

        Ok(plan)
    }

    pub fn record_cas_response(
        conn: &Connection,
        response: &OperatorDecisionApplyResponse,
    ) -> Result<()> {
        let (status, last_error): (&str, Option<String>) = if response.accepted {
            ("applied", None)
        } else if response.is_conflict() {
            ("conflicted", Some(response.raw.to_string()))
        } else {
            ("failed", Some(response.raw.to_string()))
        };
        let updated = conn
            .execute(
                "\
                UPDATE pending_operator_intents
                SET status = ?2,
                    last_attempt_at = CURRENT_TIMESTAMP,
                    last_error = ?3
                WHERE idempotency_key = ?1
                  AND superseded_by_intent_id IS NULL
                ",
                params![response.command_id, status, last_error],
            )
            .context("recording pending operator intent CAS response")?;
        ensure!(
            updated == 1,
            "expected one pending operator intent for command `{}`, updated {updated}",
            response.command_id
        );
        Ok(())
    }
}

fn coalesce_pending_operator_intents(
    conn: &mut Connection,
) -> Result<(Vec<String>, Vec<PendingOperatorIntent>)> {
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .context("starting pending operator intent reconciliation transaction")?;
    let intents = load_queued_unsuperseded_operator_intents(&tx)?;
    let mut superseded_intent_ids = Vec::new();
    let mut candidates = Vec::new();
    let mut current_process_id: Option<String> = None;
    let mut current_winner_id: Option<String> = None;

    for intent in intents {
        if current_process_id.as_deref() == Some(intent.process_id.as_str()) {
            let winner_id = current_winner_id
                .as_deref()
                .context("pending intent coalescing missing current winner")?;
            mark_pending_intent_superseded(&tx, &intent.id, winner_id)?;
            superseded_intent_ids.push(intent.id);
        } else {
            current_process_id = Some(intent.process_id.clone());
            current_winner_id = Some(intent.id.clone());
            candidates.push(intent);
        }
    }

    tx.commit()
        .context("committing pending operator intent coalescing")?;
    Ok((superseded_intent_ids, candidates))
}

fn load_queued_unsuperseded_operator_intents(
    conn: &Connection,
) -> Result<Vec<PendingOperatorIntent>> {
    let mut stmt = conn.prepare(
        "\
        SELECT id, process_id, desired_disposition, base_remote_revision, idempotency_key
        FROM pending_operator_intents
        WHERE status = 'queued'
          AND superseded_by_intent_id IS NULL
        ORDER BY process_id ASC, queued_at DESC, id DESC
        ",
    )?;
    stmt.query_map([], |row| {
        Ok(PendingOperatorIntent {
            id: row.get(0)?,
            process_id: row.get(1)?,
            desired_disposition: row.get(2)?,
            base_remote_revision: row.get(3)?,
            idempotency_key: row.get(4)?,
        })
    })?
    .collect::<std::result::Result<Vec<_>, _>>()
    .context("loading queued pending operator intents")
}

fn mark_pending_intent_superseded(
    tx: &Transaction<'_>,
    intent_id: &str,
    winner_id: &str,
) -> Result<()> {
    let updated = tx
        .execute(
            "\
            UPDATE pending_operator_intents
            SET status = 'superseded',
                superseded_by_intent_id = ?2,
                last_attempt_at = CURRENT_TIMESTAMP
            WHERE id = ?1
              AND status = 'queued'
              AND superseded_by_intent_id IS NULL
            ",
            params![intent_id, winner_id],
        )
        .context("marking older pending operator intent superseded")?;
    ensure!(
        updated == 1,
        "expected to supersede one pending operator intent `{intent_id}`, updated {updated}"
    );
    Ok(())
}

fn mark_pending_intent_conflicted(
    conn: &Connection,
    intent: &PendingOperatorIntent,
    conflict_payload: &Value,
) -> Result<()> {
    let updated = conn
        .execute(
            "\
            UPDATE pending_operator_intents
            SET status = 'conflicted',
                last_attempt_at = CURRENT_TIMESTAMP,
                last_error = ?2
            WHERE id = ?1
              AND status = 'queued'
              AND superseded_by_intent_id IS NULL
            ",
            params![intent.id, conflict_payload.to_string()],
        )
        .context("marking pending operator intent conflicted")?;
    ensure!(
        updated == 1,
        "expected to conflict one pending operator intent `{}`, updated {updated}",
        intent.id
    );
    Ok(())
}

fn remote_disposition_for_desired(desired_disposition: &str) -> Result<&'static str> {
    match desired_disposition {
        "approve" => Ok("approved"),
        "reject" => Ok("rejected"),
        "watch" => Ok("watching"),
        _ => bail!("unsupported pending operator intent disposition `{desired_disposition}`"),
    }
}

pub fn decision_conflict_event_payload(response: &OperatorDecisionApplyResponse) -> Result<Value> {
    ensure!(
        response.is_conflict(),
        "decision.conflict payload requires a CAS conflict response"
    );
    let current_remote_revision = response
        .current_revision
        .context("CAS conflict response missing current_revision")?;
    let current_disposition = response
        .current_disposition
        .as_deref()
        .context("CAS conflict response missing current_disposition")?;
    let rejected_payload = response
        .rejected_payload
        .clone()
        .context("CAS conflict response missing rejected_payload")?;

    #[cfg(not(test))]
    {
        let event = crate::ipc::handlers::decisions::decision_conflict_ipc_event(
            &crate::ipc::handlers::decisions::DecisionConflictEventParams {
                process_id: &response.process_id,
                command_id: &response.command_id,
                current_remote_revision,
                current_disposition,
                rejected_payload: &rejected_payload,
            },
        )?;
        return Ok(event.payload);
    }

    #[cfg(test)]
    Ok(json!({
        "event": "decision.conflict",
        "process_id": response.process_id,
        "command_id": response.command_id,
        "current_remote_revision": current_remote_revision,
        "current_disposition": current_disposition,
        "rejected_payload": rejected_payload,
    }))
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
        if SchemaEpochGuard::is_sync_paused(conn, "default")? {
            return Ok(None);
        }
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
            let batch = self
                .load_batch(conn, batch_id)?
                .with_context(|| format!("missing acknowledged sync batch `{batch_id}`"))?;
            self.mark_acknowledged(conn, batch_id, &response_json)?;
            ViewerSyncStatusWriter::for_default_org(batch.schema_epoch)?.record_apply_success(
                conn,
                current_epoch_secs()?,
                batch.last_local_seq,
                current_projection_generation(conn)?,
            )?;
            return Ok(None);
        }

        if response.is_schema_epoch_mismatch() {
            let batch = self
                .load_batch(conn, batch_id)?
                .with_context(|| format!("missing schema-mismatched sync batch `{batch_id}`"))?;
            self.mark_rejected(conn, batch_id, &response_json)?;
            SchemaEpochGuard::pause_for_mismatch(
                conn,
                "default",
                batch.schema_epoch,
                response.remote_schema_epoch(),
                current_epoch_secs()?,
                current_projection_generation(conn)?,
                env!("CARGO_PKG_VERSION"),
            )?;
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

fn ensure_viewer_sync_status_table(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "\
        CREATE TABLE IF NOT EXISTS viewer_sync_status (
          org_id TEXT PRIMARY KEY,
          sync_health TEXT NOT NULL DEFAULT 'offline'
            CHECK (sync_health IN ('healthy', 'degraded', 'offline')),
          last_push_success_at_epoch_secs INTEGER,
          last_push_attempt_at_epoch_secs INTEGER,
          last_acked_local_seq INTEGER,
          pending_outbox_count INTEGER NOT NULL DEFAULT 0 CHECK (pending_outbox_count >= 0),
          schema_epoch INTEGER NOT NULL CHECK (schema_epoch > 0),
          projection_generation INTEGER NOT NULL DEFAULT 0 CHECK (projection_generation >= 0),
          daemon_version TEXT NOT NULL,
          updated_at_epoch_secs INTEGER NOT NULL,
          CHECK (length(trim(org_id)) > 0),
          CHECK (
            last_push_success_at_epoch_secs IS NULL
            OR last_push_success_at_epoch_secs >= 0
          ),
          CHECK (
            last_push_attempt_at_epoch_secs IS NULL
            OR last_push_attempt_at_epoch_secs >= 0
          ),
          CHECK (last_acked_local_seq IS NULL OR last_acked_local_seq > 0),
          CHECK (length(trim(daemon_version)) > 0),
          CHECK (updated_at_epoch_secs >= 0)
        );
        ",
    )
    .context("ensuring viewer_sync_status support table")
}

fn ensure_sync_pause_state_table(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "\
        CREATE TABLE IF NOT EXISTS sync_pause_state (
          org_id TEXT PRIMARY KEY,
          paused INTEGER NOT NULL CHECK (paused IN (0, 1)),
          reason TEXT,
          local_schema_epoch INTEGER NOT NULL CHECK (local_schema_epoch > 0),
          remote_schema_epoch INTEGER,
          operator_message TEXT NOT NULL,
          updated_at_epoch_secs INTEGER NOT NULL CHECK (updated_at_epoch_secs >= 0),
          CHECK (length(trim(org_id)) > 0),
          CHECK (reason IS NULL OR length(trim(reason)) > 0),
          CHECK (remote_schema_epoch IS NULL OR remote_schema_epoch > 0),
          CHECK (length(trim(operator_message)) > 0)
        );
        ",
    )
    .context("ensuring sync_pause_state support table")
}

fn ensure_sqlite_snapshot_runs_table(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "\
        CREATE TABLE IF NOT EXISTS local_sqlite_snapshot_runs (
          snapshot_id TEXT PRIMARY KEY,
          status TEXT NOT NULL CHECK (status IN ('succeeded', 'failed', 'skipped')),
          source_path TEXT NOT NULL,
          destination_path TEXT NOT NULL,
          encryption_key_id TEXT NOT NULL,
          scheduled_for_epoch_secs INTEGER NOT NULL CHECK (scheduled_for_epoch_secs >= 0),
          started_at_epoch_secs INTEGER NOT NULL CHECK (started_at_epoch_secs >= 0),
          completed_at_epoch_secs INTEGER NOT NULL CHECK (completed_at_epoch_secs >= 0),
          wal_checkpointed INTEGER NOT NULL CHECK (wal_checkpointed IN (0, 1)),
          encrypted INTEGER NOT NULL CHECK (encrypted IN (0, 1)),
          bytes_written INTEGER NOT NULL CHECK (bytes_written >= 0),
          warning_code TEXT,
          warning_message TEXT,
          local_operation_available INTEGER NOT NULL CHECK (local_operation_available IN (0, 1)),
          created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
          CHECK (length(trim(snapshot_id)) > 0),
          CHECK (length(trim(source_path)) > 0),
          CHECK (length(trim(encryption_key_id)) > 0),
          CHECK (completed_at_epoch_secs >= started_at_epoch_secs),
          CHECK (warning_code IS NULL OR length(trim(warning_code)) > 0),
          CHECK (warning_message IS NULL OR length(trim(warning_message)) > 0)
        );
        ",
    )
    .context("ensuring local_sqlite_snapshot_runs support table")
}

fn upsert_viewer_sync_status(conn: &Connection, status: &ViewerSyncStatus) -> Result<()> {
    ensure_viewer_sync_status_table(conn)?;
    conn.execute(
        "\
        INSERT INTO viewer_sync_status (
          org_id, sync_health, last_push_success_at_epoch_secs,
          last_push_attempt_at_epoch_secs, last_acked_local_seq, pending_outbox_count,
          schema_epoch, projection_generation, daemon_version, updated_at_epoch_secs
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
        ON CONFLICT(org_id) DO UPDATE SET
          sync_health = excluded.sync_health,
          last_push_success_at_epoch_secs = excluded.last_push_success_at_epoch_secs,
          last_push_attempt_at_epoch_secs = excluded.last_push_attempt_at_epoch_secs,
          last_acked_local_seq = excluded.last_acked_local_seq,
          pending_outbox_count = excluded.pending_outbox_count,
          schema_epoch = excluded.schema_epoch,
          projection_generation = excluded.projection_generation,
          daemon_version = excluded.daemon_version,
          updated_at_epoch_secs = excluded.updated_at_epoch_secs
        ",
        params![
            status.org_id,
            status.sync_health.as_str(),
            status.last_push_success_at_epoch_secs,
            status.last_push_attempt_at_epoch_secs,
            status.last_acked_local_seq,
            status.pending_outbox_count,
            status.schema_epoch,
            status.projection_generation,
            status.daemon_version,
            status.updated_at_epoch_secs,
        ],
    )
    .context("upserting viewer_sync_status row")?;
    Ok(())
}

fn hydrate_viewer_sync_status(row: &rusqlite::Row<'_>) -> rusqlite::Result<ViewerSyncStatus> {
    let health_name: String = row.get(1)?;
    let sync_health = match health_name.as_str() {
        "healthy" => SyncHealth::Healthy,
        "degraded" => SyncHealth::Degraded,
        "offline" => SyncHealth::Offline,
        other => return Err(to_sql_error(format!("unsupported sync_health `{other}`"))),
    };
    Ok(ViewerSyncStatus {
        org_id: row.get(0)?,
        sync_health,
        last_push_success_at_epoch_secs: row.get(2)?,
        last_push_attempt_at_epoch_secs: row.get(3)?,
        last_acked_local_seq: row.get(4)?,
        pending_outbox_count: row.get(5)?,
        schema_epoch: row.get(6)?,
        projection_generation: row.get(7)?,
        daemon_version: row.get(8)?,
        updated_at_epoch_secs: row.get(9)?,
    })
}

fn hydrate_schema_epoch_status(row: &rusqlite::Row<'_>) -> rusqlite::Result<SchemaEpochStatus> {
    let paused = row.get::<_, i64>(1)? == 1;
    Ok(SchemaEpochStatus {
        org_id: row.get(0)?,
        paused,
        reason: row.get(2)?,
        local_schema_epoch: row.get(3)?,
        remote_schema_epoch: row.get(4)?,
        dispatch_policy: if paused {
            SyncDispatchPolicy::PushAndPullPaused
        } else {
            SyncDispatchPolicy::PushAndPullAllowed
        },
        local_operation_available: true,
        operator_message: row.get(5)?,
        updated_at_epoch_secs: row.get(6)?,
    })
}

fn hydrate_sqlite_snapshot_record(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<SqliteSnapshotRecord> {
    Ok(SqliteSnapshotRecord {
        snapshot_id: row.get(0)?,
        status: row.get(1)?,
        source_path: row.get(2)?,
        destination_path: row.get(3)?,
        encryption_key_id: row.get(4)?,
        scheduled_for_epoch_secs: row.get(5)?,
        started_at_epoch_secs: row.get(6)?,
        completed_at_epoch_secs: row.get(7)?,
        wal_checkpointed: row.get::<_, i64>(8)? == 1,
        encrypted: row.get::<_, i64>(9)? == 1,
        bytes_written: row.get(10)?,
        warning_code: row.get(11)?,
        warning_message: row.get(12)?,
        local_operation_available: row.get::<_, i64>(13)? == 1,
    })
}

fn latest_sqlite_snapshot_attempt(conn: &Connection) -> Result<Option<i64>> {
    conn.query_row(
        "SELECT MAX(scheduled_for_epoch_secs) FROM local_sqlite_snapshot_runs WHERE status <> 'skipped'",
        [],
        |row| row.get(0),
    )
    .context("loading latest SQLite snapshot attempt")
}

fn checkpoint_wal_passive(conn: &Connection) -> Result<bool> {
    let checkpoint = conn.query_row("PRAGMA wal_checkpoint(PASSIVE)", [], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, i64>(1)?,
            row.get::<_, i64>(2)?,
        ))
    });
    match checkpoint {
        Ok((_busy, _log_frames, _checkpointed_frames)) => Ok(true),
        Err(error) => Err(error).context("running passive WAL checkpoint before SQLite snapshot"),
    }
}

const fn bool_to_i64(value: bool) -> i64 {
    if value { 1 } else { 0 }
}

fn pending_outbox_count(conn: &Connection) -> Result<i64> {
    conn.query_row(
        "SELECT COUNT(*) FROM local_outbox WHERE writer_epoch IS NULL",
        [],
        |row| row.get(0),
    )
    .context("counting pending local_outbox rows")
}

fn current_projection_generation(conn: &Connection) -> Result<i64> {
    conn.query_row(
        "\
        SELECT COALESCE(MAX(generation_id), 0)
        FROM projection_generations
        WHERE status = 'published'
        ",
        [],
        |row| row.get(0),
    )
    .context("loading current projection generation")
}

fn current_epoch_secs() -> Result<i64> {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system clock is before UNIX_EPOCH")?;
    i64::try_from(duration.as_secs()).context("current UNIX timestamp exceeds i64")
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

fn ensure_pull_cursors_table(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "\
        CREATE TABLE IF NOT EXISTS pull_cursors (
          domain TEXT PRIMARY KEY CHECK (
            domain IN (
              'shared_source_configs',
              'shared_company_profile',
              'operator_decision_heads'
            )
          ),
          last_remote_change_seq INTEGER NOT NULL DEFAULT 0 CHECK (last_remote_change_seq >= 0),
          last_pulled_at TEXT,
          updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
        );
        ",
    )
    .context("ensuring pull_cursors support table")
}

fn hydrate_pull_cursor(row: &rusqlite::Row<'_>) -> rusqlite::Result<PullCursor> {
    let domain_name: String = row.get(0)?;
    let domain = PullCursorDomain::from_str(&domain_name).map_err(to_sql_error)?;
    Ok(PullCursor {
        domain,
        last_remote_change_seq: row.get(1)?,
        last_pulled_at: row.get(2)?,
    })
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
