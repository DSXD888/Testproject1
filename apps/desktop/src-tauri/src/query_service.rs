#![forbid(unsafe_code)]
use std::collections::{BTreeSet, HashMap, HashSet};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use rusqlite::{Connection, OpenFlags, OptionalExtension, params};
use serde::{Deserialize, Serialize};

const DEFAULT_PAGE_LIMIT: usize = 25;
const MAX_PAGE_LIMIT: usize = 100;
const SQLITE_BUSY_TIMEOUT_MS: u64 = 1_000;
const EXPECTED_SCHEMA_EPOCH: i64 = 6;

#[derive(Debug)]
pub struct QueryService {
    db_path: PathBuf,
    runtime_dir: PathBuf,
    expected_schema_epoch: i64,
    conn: Connection,
}

impl QueryService {
    pub fn open(
        db_path: impl Into<PathBuf>,
        expected_schema_epoch: i64,
    ) -> Result<Self, QueryServiceError> {
        let db_path = db_path.into();
        let runtime_dir = runtime_dir_for_db(&db_path);
        let conn = Self::open_read_connection(&db_path, expected_schema_epoch)?;
        Ok(Self {
            db_path,
            runtime_dir,
            expected_schema_epoch,
            conn,
        })
    }

    #[allow(dead_code)]
    pub const fn connection(&self) -> &Connection {
        &self.conn
    }

    pub fn list_processes(
        &self,
        cursor: Option<&str>,
        limit: usize,
        filters: &ProcessListFilters,
    ) -> Result<QueryEnvelope<KeysetPage<ProcessListItem>>, QueryServiceError> {
        validate_process_filters(filters)?;
        let cursor = decode_cursor(cursor)?;
        let limit = validate_limit(limit)?;

        self.with_snapshot(|conn| {
            let items = load_process_list_rows(conn, cursor.as_ref(), limit, filters)?;
            Ok(build_page(items, limit, |item| {
                CursorToken::new(&item.published_at, &item.process_id)
            }))
        })
    }

    pub fn get_process_detail(
        &self,
        process_id: &str,
    ) -> Result<QueryEnvelope<ProcessDetailVm>, QueryServiceError> {
        self.with_snapshot(|conn| {
            let canonical_process_id = resolve_process_id(conn, process_id)?;
            let process_row = conn
                .query_row(
                    "\
                    SELECT id, COALESCE(canonical_process_id, id)
                    FROM procurement_processes
                    WHERE id = ?1
                    ",
                    params![canonical_process_id],
                    |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
                )
                .optional()
                .map_err(storage_unreadable)?
                .ok_or_else(|| QueryServiceError::not_found("process not found"))?;
            let decisions = latest_decisions(conn)?;

            let notices = load_notice_rows(conn, &canonical_process_id)?;
            let lots = load_lot_rows(conn, &canonical_process_id)?;
            let candidates = load_candidate_rows(conn, &canonical_process_id)?;
            let quotes = load_quote_rows(conn, &canonical_process_id)?;
            let landed_costs = load_landed_cost_rows(conn, &canonical_process_id)?;
            let errors = load_error_rows(
                conn,
                Some(&canonical_process_id),
                None,
                MAX_PAGE_LIMIT,
                &ErrorListFilters::default(),
            )?;

            let (coverage, pipeline_basis_hash, pipeline_stage_run_id, pipeline_state) =
                load_coverage(conn, &canonical_process_id)?;
            let (manifest, manifest_error) = load_manifest(
                conn,
                &canonical_process_id,
                pipeline_basis_hash.as_deref(),
                pipeline_stage_run_id.as_deref(),
            )?;
            let decision = decision_status_for_process(
                decisions.get(&canonical_process_id),
                pipeline_basis_hash.as_deref(),
            );
            let review_metadata = ReviewMetadataVm {
                basis_hash: pipeline_basis_hash,
                manifest_hash: manifest
                    .as_ref()
                    .and_then(|manifest| manifest.artifact_hash.clone()),
                stage_run_id: manifest
                    .as_ref()
                    .and_then(|manifest| manifest.stage_run_id.clone())
                    .or(pipeline_stage_run_id),
            };

            Ok(ProcessDetailVm {
                process_id: process_row.0,
                canonical_process_id: process_row.1,
                pipeline_state,
                notices,
                lots,
                candidates,
                quotes,
                landed_costs,
                manifest,
                review_metadata,
                manifest_error,
                coverage,
                decision,
                errors,
            })
        })
    }

    pub fn list_sessions(
        &self,
        cursor: Option<&str>,
        limit: usize,
        filters: &SessionListFilters,
    ) -> Result<QueryEnvelope<KeysetPage<SessionListItem>>, QueryServiceError> {
        validate_session_filters(filters)?;
        let cursor = decode_cursor(cursor)?;
        let limit = validate_limit(limit)?;

        self.with_snapshot(|conn| {
            let items =
                load_session_rows(conn, &self.runtime_dir, cursor.as_ref(), limit, filters)?;
            Ok(build_page(items, limit, |item| {
                CursorToken::new(item.started_at.as_deref().unwrap_or(""), &item.session_id)
            }))
        })
    }

    pub fn list_errors(
        &self,
        cursor: Option<&str>,
        limit: usize,
        filters: &ErrorListFilters,
    ) -> Result<QueryEnvelope<KeysetPage<ErrorListItem>>, QueryServiceError> {
        validate_error_filters(filters)?;
        let cursor = decode_cursor(cursor)?;
        let limit = validate_limit(limit)?;

        self.with_snapshot(|conn| {
            let items = load_error_rows(conn, None, cursor.as_ref(), limit, filters)?;
            Ok(build_page(items, limit, |item| {
                CursorToken::new(&item.created_at, &item.error_manifest_id)
            }))
        })
    }

    pub fn get_session_history(
        &self,
        request: &SessionHistoryRequest,
    ) -> Result<QueryEnvelope<SessionHistoryChunk>, QueryServiceError> {
        if request.to_seq < request.from_seq || request.from_seq == 0 {
            return Err(QueryServiceError::invalid_filter(
                "history range must be a non-empty positive sequence window",
            ));
        }

        self.with_snapshot(|conn| {
            let (execution_mode, terminal_output_path) = conn
                .query_row(
                    "\
                    SELECT execution_mode, terminal_output_path
                    FROM agent_session_logs
                    WHERE id = ?1
                    ",
                    params![request.session_id],
                    |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?)),
                )
                .optional()
                .map_err(storage_unreadable)?
                .ok_or_else(|| QueryServiceError::not_found("session not found"))?;

            let Some(terminal_output_path) = terminal_output_path else {
                return Ok(SessionHistoryChunk {
                    session_id: request.session_id.clone(),
                    first_seq: request.from_seq,
                    last_seq: request.to_seq,
                    frames: Vec::new(),
                    availability: ReplayAvailability::Missing,
                });
            };

            if terminal_output_path.ends_with("stream.index.jsonl") {
                return read_pty_history(Path::new(&terminal_output_path), request);
            }

            read_contract_history(Path::new(&terminal_output_path), &execution_mode, request)
        })
    }

    pub fn list_manifests(
        &self,
    ) -> Result<QueryEnvelope<Vec<ManifestInboxRow>>, QueryServiceError> {
        self.with_snapshot(load_manifest_inbox_rows)
    }

    pub fn get_decision_history(
        &self,
        process_id: &str,
    ) -> Result<QueryEnvelope<Vec<DecisionHistoryEntry>>, QueryServiceError> {
        self.with_snapshot(|conn| {
            let canonical = resolve_process_id(conn, process_id)?;
            load_decision_history_rows(conn, &canonical)
        })
    }

    pub fn get_active_overrides(
        &self,
        process_id: &str,
    ) -> Result<QueryEnvelope<Vec<ActiveOverrideEntry>>, QueryServiceError> {
        self.with_snapshot(|conn| {
            let canonical = resolve_process_id(conn, process_id)?;
            load_active_override_rows(conn, &canonical)
        })
    }

    #[cfg(test)]
    pub fn open_read_connection_for_test(&self) -> Result<Connection, QueryServiceError> {
        Self::open_read_connection(&self.db_path, EXPECTED_SCHEMA_EPOCH)
    }

    fn with_snapshot<T, F>(&self, reader: F) -> Result<QueryEnvelope<T>, QueryServiceError>
    where
        F: FnOnce(&Connection) -> Result<T, QueryServiceError>,
    {
        let data = reader(&self.conn)?;
        validate_storage(&self.conn, self.expected_schema_epoch)?;
        let snapshot_seq = read_commit_seq(&self.conn)?;
        let snapshot_at = snapshot_at(&self.conn)?;
        Ok(QueryEnvelope {
            data,
            snapshot_seq,
            snapshot_at,
        })
    }

    fn open_read_connection(
        db_path: &Path,
        expected_schema_epoch: i64,
    ) -> Result<Connection, QueryServiceError> {
        let conn = Connection::open_with_flags(db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(storage_unreadable)?;
        conn.busy_timeout(Duration::from_millis(SQLITE_BUSY_TIMEOUT_MS))
            .map_err(storage_unreadable)?;
        conn.execute_batch("PRAGMA query_only = ON;")
            .map_err(storage_unreadable)?;
        let _ = expected_schema_epoch;
        Ok(conn)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct QueryEnvelope<T> {
    pub data: T,
    pub snapshot_seq: i64,
    pub snapshot_at: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct KeysetPage<T> {
    pub items: Vec<T>,
    pub next_cursor: Option<String>,
    pub has_more: bool,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PipelineState {
    Pending,
    Qualifying,
    Enriching,
    Qualified,
    Review,
    Rejected,
    Error,
}

impl PipelineState {
    fn from_db(value: &str) -> Self {
        match value {
            "pending" => Self::Pending,
            "qualifying" => Self::Qualifying,
            "enriching" => Self::Enriching,
            "qualified" => Self::Qualified,
            "review" => Self::Review,
            "rejected" => Self::Rejected,
            _ => Self::Error,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionModeVm {
    Pty,
    Contract,
}

impl ExecutionModeVm {
    fn from_db(value: &str) -> Self {
        match value {
            "pty" => Self::Pty,
            _ => Self::Contract,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SessionPhaseVm {
    Created,
    Spawned,
    Active,
    Terminating,
    Exited,
    Reaped,
    Archived,
}

impl SessionPhaseVm {
    fn from_db(value: &str) -> Self {
        match value {
            "created" => Self::Created,
            "spawned" => Self::Spawned,
            "active" => Self::Active,
            "terminating" => Self::Terminating,
            "reaped" => Self::Reaped,
            "archived" => Self::Archived,
            _ => Self::Exited,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CleanupVerdictVm {
    Clean,
    Forced,
    Leaked,
}

impl CleanupVerdictVm {
    fn from_db(value: &str) -> Option<Self> {
        match value {
            "clean" => Some(Self::Clean),
            "forced" => Some(Self::Forced),
            "leaked" => Some(Self::Leaked),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ErrorSeverity {
    Critical,
    Warning,
    Info,
}

impl ErrorSeverity {}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum QueryServiceErrorCode {
    NotFound,
    InvalidCursor,
    InvalidFilter,
    Busy,
    StorageUnreadable,
    SchemaMismatch,
    ArtifactUnavailable,
}

impl fmt::Display for QueryServiceErrorCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::NotFound => "not_found",
            Self::InvalidCursor => "invalid_cursor",
            Self::InvalidFilter => "invalid_filter",
            Self::Busy => "busy",
            Self::StorageUnreadable => "storage_unreadable",
            Self::SchemaMismatch => "schema_mismatch",
            Self::ArtifactUnavailable => "artifact_unavailable",
        })
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProcessListFilters {
    pub source_ids: Vec<String>,
    pub pipeline_states: Vec<String>,
    pub decision_dispositions: Vec<String>,
    pub stale_only: bool,
    pub search: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProcessListItem {
    pub process_id: String,
    pub canonical_process_id: String,
    pub title: String,
    pub buyer_name: String,
    pub published_at: String,
    pub deadline_at: Option<String>,
    pub estimated_value_pence: Option<i64>,
    pub lot_count: i64,
    pub access_route: String,
    pub legal_category: String,
    pub pipeline_state: PipelineState,
    pub priced_coverage_bps: Option<i64>,
    pub net_qualified_coverage_bps: Option<i64>,
    pub net_qualified_value_coverage_bps: Option<i64>,
    pub gross_margin_bps: Option<i64>,
    pub net_margin_base_bps: Option<i64>,
    pub candidate_count: i64,
    pub quoted_candidate_count: i64,
    pub decision: DecisionStatus,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DecisionStatus {
    pub disposition: Option<String>,
    pub status: Option<String>,
    pub is_stale: bool,
    pub decided_at: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProcessDetailVm {
    pub process_id: String,
    pub canonical_process_id: String,
    pub pipeline_state: PipelineState,
    pub notices: Vec<ContractNoticeVm>,
    pub lots: Vec<NoticeLotVm>,
    pub candidates: Vec<ProductCandidateVm>,
    pub quotes: Vec<SupplierQuoteVm>,
    pub landed_costs: Vec<LandedCostVm>,
    pub manifest: Option<ManifestVm>,
    pub review_metadata: ReviewMetadataVm,
    pub manifest_error: Option<QueryServiceError>,
    pub coverage: CoverageVm,
    pub decision: DecisionStatus,
    pub errors: Vec<ErrorListItem>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ContractNoticeVm {
    pub notice_id: String,
    pub source_id: String,
    pub source_notice_id: String,
    pub title: String,
    pub buyer_name: Option<String>,
    pub procurement_stage: String,
    pub summary: Option<String>,
    pub published_at: String,
    pub deadline_at: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NoticeLotVm {
    pub lot_id: String,
    pub notice_id: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub estimated_value_pence: Option<i64>,
    pub deadline_at: Option<String>,
    pub is_bidable_separately: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProductCandidateVm {
    pub candidate_id: String,
    pub lot_id: String,
    pub name: String,
    pub candidate_type: String,
    pub specification_text: Option<String>,
    pub candidate_value_share_bps: Option<i64>,
    pub quoteability_score_bps: Option<i64>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SupplierQuoteVm {
    pub quote_id: String,
    pub product_candidate_id: Option<String>,
    pub product_name: String,
    pub source_name: String,
    pub source_url: Option<String>,
    pub currency: String,
    pub unit_price_gbp_pence: i64,
    pub price_signal_role: String,
    pub ship_from_country: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LandedCostVm {
    pub estimate_id: String,
    pub product_candidate_id: String,
    pub supplier_quote_id: String,
    pub economic_landed_cost_gbp_pence: i64,
    pub net_margin_base_bps: i64,
    pub decision_grade: String,
    pub confidence_bps: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ManifestVm {
    pub markdown: String,
    pub basis_hash: Option<String>,
    pub generated_at: String,
    pub sections: Vec<String>,
    pub artifact_hash: Option<String>,
    pub stage_run_id: Option<String>,
    /// Stage 14 Phase 2 — raw `clarification_prompts_json` from the
    /// `manifests` table. Parsed on the React side into per-prompt input
    /// controls in the clarification form.
    #[serde(default)]
    pub clarification_prompts_json: Option<String>,
    /// Stage 14 Phase 2 — `manifest_basis_hash` directly from the manifest
    /// row, used by the desktop UI's stale banner to detect basis drift.
    #[serde(default)]
    pub manifest_basis_hash: Option<String>,
    /// Stage 14 Phase 2 — `stoplight` directly from the manifest row.
    #[serde(default)]
    pub stoplight: Option<String>,
    /// Stage 14 Phase 2 — `suggested_action` directly from the manifest row.
    #[serde(default)]
    pub suggested_action: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CoverageVm {
    pub priced_coverage_bps: i64,
    pub landed_cost_coverage_bps: i64,
    pub net_qualified_coverage_bps: i64,
    pub priced_value_coverage_bps: i64,
    pub landed_cost_value_coverage_bps: i64,
    pub net_qualified_value_coverage_bps: i64,
    pub unresolved_mandatory_lots: i64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ReviewMetadataVm {
    pub basis_hash: Option<String>,
    pub manifest_hash: Option<String>,
    pub stage_run_id: Option<String>,
}

/// Stage 14 Phase 2 — one row in the Manifest Inbox.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ManifestInboxRow {
    pub process_id: String,
    pub canonical_process_id: String,
    pub title: Option<String>,
    pub buyer_name: Option<String>,
    pub estimated_value_pence: Option<i64>,
    pub stoplight: Option<String>,
    pub suggested_action: Option<String>,
    pub clarification_prompt_count: i64,
    pub manifest_basis_hash: Option<String>,
    pub generated_at: String,
    pub decision: DecisionStatus,
}

/// Stage 14 Phase 2 — one entry in the decision-history audit chain.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DecisionHistoryEntry {
    pub intent_id: String,
    pub idempotency_key: String,
    pub disposition: String,
    pub status: String,
    pub reviewed_basis_hash: String,
    pub reviewed_manifest_hash: String,
    pub reviewed_at: String,
    pub queued_at: String,
    pub superseded_by_intent_id: Option<String>,
    pub decision_note: Option<String>,
}

/// Stage 14 Phase 2 — one active operator override.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ActiveOverrideEntry {
    pub override_id: String,
    pub product_candidate_id: String,
    pub reason_code: String,
    pub payload_json: String,
    pub notes: Option<String>,
    pub basis_hash_when_provided: Option<String>,
    pub provided_at: String,
    pub provided_by: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SessionListFilters {
    pub phases: Vec<String>,
    pub execution_modes: Vec<String>,
    pub active_only: bool,
    pub search: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SessionListItem {
    pub session_id: String,
    pub daemon_instance_id: String,
    pub skill_path: String,
    pub skill_version: Option<String>,
    pub execution_mode: ExecutionModeVm,
    pub phase: SessionPhaseVm,
    pub started_at: Option<String>,
    pub ended_at: Option<String>,
    pub duration_ms: Option<i64>,
    pub cleanup_verdict: Option<CleanupVerdictVm>,
    pub error_type: Option<String>,
    pub exit_code: Option<i64>,
    pub checkpoint_count: i64,
    pub byte_count: i64,
    pub reattachable: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ErrorListFilters {
    pub severities: Vec<String>,
    pub resolved: Option<bool>,
    pub pipeline_stages: Vec<String>,
    pub search: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ErrorListItem {
    pub error_manifest_id: String,
    pub process_id: Option<String>,
    pub pipeline_stage: Option<String>,
    pub error_type: String,
    pub error_message: String,
    pub cleanup_verdict: Option<CleanupVerdictVm>,
    pub severity: ErrorSeverity,
    pub skill_path: Option<String>,
    pub created_at: String,
    pub resolved: bool,
    pub agent_session_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SessionHistoryRequest {
    pub session_id: String,
    pub from_seq: u64,
    pub to_seq: u64,
    pub max_bytes: Option<usize>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SessionHistoryChunk {
    pub session_id: String,
    pub first_seq: u64,
    pub last_seq: u64,
    pub frames: Vec<SessionFrame>,
    pub availability: ReplayAvailability,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReplayAvailability {
    Complete,
    Partial,
    Missing,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum StructuredSessionEvent {
    Started {
        pid: u32,
        pgid: u32,
        sid: u32,
        mode: String,
    },
    Checkpoint {
        checkpoint_seq: u32,
        path: String,
    },
    Progress {
        completed: u32,
        total: Option<u32>,
    },
    AuthChallenge {
        kind: String,
        detail: String,
        intervention: Option<serde_json::Value>,
    },
    Warning {
        code: String,
        detail: String,
    },
    FinalResultReady {
        path: String,
    },
    Exited {
        exit_code: Option<i32>,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum SessionFrame {
    Lifecycle {
        seq: u64,
        state: String,
        cleanup_verdict: Option<String>,
        archive_seq: Option<i64>,
        ts: String,
    },
    Pty {
        seq: u64,
        chunk: Vec<u8>,
        ts: String,
    },
    Stdout {
        seq: u64,
        chunk: String,
        ts: String,
    },
    Stderr {
        seq: u64,
        chunk: String,
        ts: String,
    },
    Event {
        seq: u64,
        event: StructuredSessionEvent,
        ts: String,
    },
    Gap {
        from_seq: u64,
        to_seq: u64,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct QueryServiceError {
    pub code: QueryServiceErrorCode,
    pub message: String,
    pub retryable: bool,
}

impl QueryServiceError {
    fn new(code: QueryServiceErrorCode, message: impl Into<String>, retryable: bool) -> Self {
        Self {
            code,
            message: message.into(),
            retryable,
        }
    }

    fn not_found(message: impl Into<String>) -> Self {
        Self::new(QueryServiceErrorCode::NotFound, message, false)
    }

    fn invalid_cursor(message: impl Into<String>) -> Self {
        Self::new(QueryServiceErrorCode::InvalidCursor, message, false)
    }

    fn invalid_filter(message: impl Into<String>) -> Self {
        Self::new(QueryServiceErrorCode::InvalidFilter, message, false)
    }

    fn artifact_unavailable(message: impl Into<String>) -> Self {
        Self::new(QueryServiceErrorCode::ArtifactUnavailable, message, false)
    }
}

impl fmt::Display for QueryServiceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:?}: {}", self.code, self.message)
    }
}

impl std::error::Error for QueryServiceError {}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct CursorToken {
    sort_at: String,
    stable_id: String,
}

impl CursorToken {
    fn new(sort_at: &str, stable_id: &str) -> Self {
        Self {
            sort_at: sort_at.to_owned(),
            stable_id: stable_id.to_owned(),
        }
    }
}

#[derive(Clone, Debug)]
struct DecisionRecord {
    disposition: String,
    status: String,
    reviewed_basis_hash: Option<String>,
    decided_at: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
struct StreamIndexEntry {
    chunk: String,
    first_seq: u64,
    last_seq: u64,
}

#[allow(clippy::too_many_lines)]
fn load_process_list_rows(
    conn: &Connection,
    cursor: Option<&CursorToken>,
    limit: usize,
    filters: &ProcessListFilters,
) -> Result<Vec<ProcessListItem>, QueryServiceError> {
    let source_ids = json_filter_values(&filters.source_ids)?;
    let pipeline_states = json_filter_values(&filters.pipeline_states)?;
    let decision_dispositions = json_filter_values(&filters.decision_dispositions)?;
    let fetch_limit = i64::try_from(limit + 1)
        .map_err(|_| QueryServiceError::invalid_filter("limit exceeds supported range"))?;
    let mut stmt = conn
        .prepare(
            "\
            WITH process_rows AS (
              SELECT p.id AS process_id,
                     COALESCE(p.canonical_process_id, p.id) AS canonical_process_id,
                     p.title,
                     COALESCE(p.buyer_name, '') AS buyer_name,
                     p.latest_published_at AS published_at,
                     p.latest_deadline_at AS deadline_at,
                     COALESCE(
                       (
                         SELECT SUM(nl.estimated_value_pence)
                         FROM notice_lots nl
                         INNER JOIN contract_notices cn ON cn.id = nl.notice_id
                         WHERE cn.process_id = p.id
                       ),
                       (
                         SELECT MAX(cn.estimated_value_gbp_ex_vat_pence)
                         FROM contract_notices cn
                         WHERE cn.process_id = p.id
                       )
                     ) AS estimated_value_pence,
                     COALESCE(
                       (
                         SELECT COUNT(*)
                         FROM notice_lots nl
                         INNER JOIN contract_notices cn ON cn.id = nl.notice_id
                         WHERE cn.process_id = p.id
                       ),
                       0
                     ) AS lot_count,
                     COALESCE(
                       (
                         SELECT qr.access_route
                         FROM qualification_runs qr
                         WHERE qr.process_id = p.id AND qr.access_route IS NOT NULL
                         ORDER BY qr.qualified_at DESC, qr.id DESC
                         LIMIT 1
                       ),
                       (
                         SELECT cn.access_route
                         FROM contract_notices cn
                         WHERE cn.process_id = p.id AND cn.access_route IS NOT NULL
                         ORDER BY cn.published_at DESC, cn.id DESC
                         LIMIT 1
                       ),
                       'unknown'
                     ) AS access_route,
                     COALESCE(
                       (
                         SELECT CASE
                           WHEN COUNT(DISTINCT qr.legal_category) > 1 THEN 'mixed'
                           ELSE MAX(qr.legal_category)
                         END
                         FROM qualification_runs qr
                         WHERE qr.process_id = p.id AND qr.legal_category IS NOT NULL
                       ),
                       'unknown'
                     ) AS legal_category,
                     COALESCE(pa.pipeline_state, 'pending') AS pipeline_state,
                     pa.priced_coverage_bps,
                     pa.net_qualified_coverage_bps,
                     pa.net_qualified_value_coverage_bps,
                     pa.gross_margin_bps,
                     pa.net_margin_bps,
                     COALESCE((SELECT COUNT(*) FROM product_candidates pc WHERE pc.process_id = p.id), 0) AS candidate_count,
                     COALESCE((SELECT COUNT(*) FROM supplier_quotes sq WHERE sq.process_id = p.id), 0) AS quoted_candidate_count,
                     pa.decision_basis_hash,
                     (
                       SELECT poi.desired_disposition
                       FROM pending_operator_intents poi
                       WHERE poi.process_id = p.id AND poi.superseded_by_intent_id IS NULL
                       ORDER BY poi.queued_at DESC, poi.id DESC
                       LIMIT 1
                     ) AS decision_disposition,
                     (
                       SELECT poi.status
                       FROM pending_operator_intents poi
                       WHERE poi.process_id = p.id AND poi.superseded_by_intent_id IS NULL
                       ORDER BY poi.queued_at DESC, poi.id DESC
                       LIMIT 1
                     ) AS decision_status,
                     (
                       SELECT poi.reviewed_basis_hash
                       FROM pending_operator_intents poi
                       WHERE poi.process_id = p.id AND poi.superseded_by_intent_id IS NULL
                       ORDER BY poi.queued_at DESC, poi.id DESC
                       LIMIT 1
                     ) AS decision_reviewed_basis_hash,
                     (
                       SELECT poi.reviewed_at
                       FROM pending_operator_intents poi
                       WHERE poi.process_id = p.id AND poi.superseded_by_intent_id IS NULL
                       ORDER BY poi.queued_at DESC, poi.id DESC
                       LIMIT 1
                     ) AS decision_decided_at
              FROM procurement_processes p
              LEFT JOIN pipeline_assessments pa ON pa.process_id = p.id
            )
            SELECT process_id,
                   canonical_process_id,
                   title,
                   buyer_name,
                   published_at,
                   deadline_at,
                   estimated_value_pence,
                   lot_count,
                   access_route,
                   legal_category,
                   pipeline_state,
                   priced_coverage_bps,
                   net_qualified_coverage_bps,
                   net_qualified_value_coverage_bps,
                   gross_margin_bps,
                   net_margin_bps,
                   candidate_count,
                   quoted_candidate_count,
                   decision_disposition,
                   decision_status,
                   decision_reviewed_basis_hash,
                   decision_decided_at,
                   decision_basis_hash
            FROM process_rows
            WHERE (
                    json_array_length(?1) = 0
                    OR EXISTS (
                      SELECT 1
                      FROM contract_notices cn
                      WHERE cn.process_id = process_rows.process_id
                        AND cn.source_id IN (SELECT value FROM json_each(?1))
                    )
                  )
              AND (
                    json_array_length(?2) = 0
                    OR pipeline_state IN (SELECT value FROM json_each(?2))
                  )
              AND (
                    json_array_length(?3) = 0
                    OR decision_disposition IN (SELECT value FROM json_each(?3))
                  )
              AND (
                    ?4 = 0
                    OR decision_status = 'stale'
                    OR (
                         decision_status = 'queued'
                         AND decision_reviewed_basis_hash IS NOT NULL
                         AND decision_basis_hash IS NOT NULL
                         AND decision_reviewed_basis_hash <> decision_basis_hash
                       )
                  )
              AND (
                    ?5 IS NULL
                    OR lower(title) LIKE '%' || lower(?5) || '%'
                    OR lower(buyer_name) LIKE '%' || lower(?5) || '%'
                  )
              AND (
                    access_route = 'dynamic_market'
                    OR deadline_at IS NULL
                    OR datetime(deadline_at) >= datetime('now', '+30 days')
                  )
              AND (
                    ?6 IS NULL
                    OR published_at < ?6
                    OR (published_at = ?6 AND process_id < ?7)
                  )
            ORDER BY published_at DESC, process_id DESC
            LIMIT ?8
            ",
        )
        .map_err(storage_unreadable)?;

    let rows = stmt
        .query_map(
            params![
                source_ids,
                pipeline_states,
                decision_dispositions,
                i64::from(filters.stale_only),
                filters.search,
                cursor.map(|value| value.sort_at.as_str()),
                cursor.map(|value| value.stable_id.as_str()),
                fetch_limit,
            ],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, Option<i64>>(6)?,
                    row.get::<_, i64>(7)?,
                    row.get::<_, String>(8)?,
                    row.get::<_, String>(9)?,
                    row.get::<_, String>(10)?,
                    row.get::<_, Option<i64>>(11)?,
                    row.get::<_, Option<i64>>(12)?,
                    row.get::<_, Option<i64>>(13)?,
                    row.get::<_, Option<i64>>(14)?,
                    row.get::<_, Option<i64>>(15)?,
                    row.get::<_, i64>(16)?,
                    row.get::<_, i64>(17)?,
                    row.get::<_, Option<String>>(18)?,
                    row.get::<_, Option<String>>(19)?,
                    row.get::<_, Option<String>>(20)?,
                    row.get::<_, Option<String>>(21)?,
                    row.get::<_, Option<String>>(22)?,
                ))
            },
        )
        .map_err(storage_unreadable)?;

    rows.map(|row| {
        let (
            process_id,
            canonical_process_id,
            title,
            buyer_name,
            published_at,
            deadline_at,
            estimated_value_pence,
            lot_count,
            access_route,
            legal_category,
            pipeline_state,
            priced_coverage_bps,
            net_qualified_coverage_bps,
            net_qualified_value_coverage_bps,
            gross_margin_bps,
            net_margin_base_bps,
            candidate_count,
            quoted_candidate_count,
            decision_disposition,
            decision_status,
            decision_reviewed_basis_hash,
            decision_decided_at,
            decision_basis_hash,
        ) = row.map_err(storage_unreadable)?;
        let decision_basis_drifted = matches!(
            (&decision_reviewed_basis_hash, &decision_basis_hash),
            (Some(reviewed), Some(current)) if reviewed != current
        );
        let decision_is_stale = matches!(decision_status.as_deref(), Some("stale"))
            || (matches!(decision_status.as_deref(), Some("queued")) && decision_basis_drifted);

        Ok(ProcessListItem {
            process_id,
            canonical_process_id,
            title,
            buyer_name,
            published_at,
            deadline_at,
            estimated_value_pence,
            lot_count,
            access_route,
            legal_category,
            pipeline_state: PipelineState::from_db(&pipeline_state),
            priced_coverage_bps,
            net_qualified_coverage_bps,
            net_qualified_value_coverage_bps,
            gross_margin_bps,
            net_margin_base_bps,
            candidate_count,
            quoted_candidate_count,
            decision: DecisionStatus {
                disposition: decision_disposition,
                status: decision_status,
                is_stale: decision_is_stale,
                decided_at: decision_decided_at,
            },
        })
    })
    .collect()
}

fn load_notice_rows(
    conn: &Connection,
    process_id: &str,
) -> Result<Vec<ContractNoticeVm>, QueryServiceError> {
    let mut stmt = conn
        .prepare(
            "\
            SELECT id, source_id, source_notice_id, title, buyer_name, procurement_stage, summary,
                   published_at, deadline_at
            FROM contract_notices
            WHERE process_id = ?1
            ORDER BY published_at DESC, id DESC
            ",
        )
        .map_err(storage_unreadable)?;
    let rows = stmt
        .query_map(params![process_id], |row| {
            Ok(ContractNoticeVm {
                notice_id: row.get(0)?,
                source_id: row.get(1)?,
                source_notice_id: row.get(2)?,
                title: row.get(3)?,
                buyer_name: row.get(4)?,
                procurement_stage: row.get(5)?,
                summary: row.get(6)?,
                published_at: row.get(7)?,
                deadline_at: row.get(8)?,
            })
        })
        .map_err(storage_unreadable)?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(storage_unreadable)
}

fn load_lot_rows(
    conn: &Connection,
    process_id: &str,
) -> Result<Vec<NoticeLotVm>, QueryServiceError> {
    let mut stmt = conn
        .prepare(
            "\
            SELECT nl.id, nl.notice_id, nl.title, nl.description, nl.estimated_value_pence,
                   nl.deadline_at, nl.is_bidable_separately
            FROM notice_lots nl
            INNER JOIN contract_notices cn ON cn.id = nl.notice_id
            WHERE cn.process_id = ?1
            ORDER BY nl.created_at, nl.id
            ",
        )
        .map_err(storage_unreadable)?;
    let rows = stmt
        .query_map(params![process_id], |row| {
            Ok(NoticeLotVm {
                lot_id: row.get(0)?,
                notice_id: row.get(1)?,
                title: row.get(2)?,
                description: row.get(3)?,
                estimated_value_pence: row.get(4)?,
                deadline_at: row.get(5)?,
                is_bidable_separately: row.get::<_, i64>(6)? == 1,
            })
        })
        .map_err(storage_unreadable)?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(storage_unreadable)
}

fn load_candidate_rows(
    conn: &Connection,
    process_id: &str,
) -> Result<Vec<ProductCandidateVm>, QueryServiceError> {
    let mut stmt = conn
        .prepare(
            "\
            SELECT id, lot_id, name, candidate_type, specification_text,
                   candidate_value_share_bps, quoteability_score_bps
            FROM product_candidates
            WHERE process_id = ?1
            ORDER BY created_at, id
            ",
        )
        .map_err(storage_unreadable)?;
    let rows = stmt
        .query_map(params![process_id], |row| {
            Ok(ProductCandidateVm {
                candidate_id: row.get(0)?,
                lot_id: row.get(1)?,
                name: row.get(2)?,
                candidate_type: row.get(3)?,
                specification_text: row.get(4)?,
                candidate_value_share_bps: row.get(5)?,
                quoteability_score_bps: row.get(6)?,
            })
        })
        .map_err(storage_unreadable)?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(storage_unreadable)
}

fn load_quote_rows(
    conn: &Connection,
    process_id: &str,
) -> Result<Vec<SupplierQuoteVm>, QueryServiceError> {
    let mut stmt = conn
        .prepare(
            "\
            SELECT id, product_candidate_id, product_name, source_name, source_url, currency,
                   unit_price_gbp_pence, price_signal_role, ship_from_country
            FROM supplier_quotes
            WHERE process_id = ?1
            ORDER BY created_at, id
            ",
        )
        .map_err(storage_unreadable)?;
    let rows = stmt
        .query_map(params![process_id], |row| {
            Ok(SupplierQuoteVm {
                quote_id: row.get(0)?,
                product_candidate_id: row.get(1)?,
                product_name: row.get(2)?,
                source_name: row.get(3)?,
                source_url: row.get(4)?,
                currency: row.get(5)?,
                unit_price_gbp_pence: row.get(6)?,
                price_signal_role: row.get(7)?,
                ship_from_country: row.get(8)?,
            })
        })
        .map_err(storage_unreadable)?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(storage_unreadable)
}

fn load_landed_cost_rows(
    conn: &Connection,
    process_id: &str,
) -> Result<Vec<LandedCostVm>, QueryServiceError> {
    let mut stmt = conn
        .prepare(
            "\
            SELECT id, product_candidate_id, supplier_quote_id, economic_landed_cost_gbp_pence,
                   net_margin_base_bps, decision_grade, confidence_bps
            FROM landed_cost_estimates
            WHERE process_id = ?1
            ORDER BY created_at, id
            ",
        )
        .map_err(storage_unreadable)?;
    let rows = stmt
        .query_map(params![process_id], |row| {
            Ok(LandedCostVm {
                estimate_id: row.get(0)?,
                product_candidate_id: row.get(1)?,
                supplier_quote_id: row.get(2)?,
                economic_landed_cost_gbp_pence: row.get(3)?,
                net_margin_base_bps: row.get(4)?,
                decision_grade: row.get(5)?,
                confidence_bps: row.get(6)?,
            })
        })
        .map_err(storage_unreadable)?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(storage_unreadable)
}

fn load_manifest(
    conn: &Connection,
    process_id: &str,
    basis_hash: Option<&str>,
    pipeline_stage_run_id: Option<&str>,
) -> Result<(Option<ManifestVm>, Option<QueryServiceError>), QueryServiceError> {
    let manifest_row = conn
        .query_row(
            "\
            SELECT markdown, artifact_locator, artifact_hash, stage_run_id, created_at,
                   clarification_prompts_json, manifest_basis_hash, stoplight, suggested_action
            FROM manifests
            WHERE process_id = ?1
            ORDER BY created_at DESC, id DESC
            LIMIT 1
            ",
            params![process_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, Option<String>>(6)?,
                    row.get::<_, Option<String>>(7)?,
                    row.get::<_, Option<String>>(8)?,
                ))
            },
        )
        .optional()
        .map_err(storage_unreadable)?;

    let Some((
        markdown,
        artifact_locator,
        artifact_hash,
        stage_run_id,
        generated_at,
        clarification_prompts_json,
        manifest_basis_hash,
        stoplight,
        suggested_action,
    )) = manifest_row
    else {
        return Ok((None, None));
    };

    if markdown.trim().is_empty() && artifact_locator.is_some() {
        return Ok((
            None,
            Some(QueryServiceError::artifact_unavailable(
                "manifest metadata exists but markdown is unavailable",
            )),
        ));
    }

    Ok((
        Some(ManifestVm {
            markdown: markdown.clone(),
            basis_hash: basis_hash.map(ToOwned::to_owned),
            generated_at,
            sections: markdown
                .lines()
                .filter_map(|line| line.strip_prefix("## ").map(ToOwned::to_owned))
                .collect(),
            artifact_hash,
            stage_run_id: stage_run_id.or_else(|| pipeline_stage_run_id.map(ToOwned::to_owned)),
            clarification_prompts_json,
            manifest_basis_hash,
            stoplight,
            suggested_action,
        }),
        None,
    ))
}

fn load_coverage(
    conn: &Connection,
    process_id: &str,
) -> Result<(CoverageVm, Option<String>, Option<String>, PipelineState), QueryServiceError> {
    let row = conn
        .query_row(
            "\
            SELECT priced_coverage_bps, landed_cost_coverage_bps, net_qualified_coverage_bps,
                   priced_value_coverage_bps, landed_cost_value_coverage_bps,
                   net_qualified_value_coverage_bps, unresolved_mandatory_lots,
                   decision_basis_hash, stage_run_id, COALESCE(pipeline_state, 'pending')
            FROM pipeline_assessments
            WHERE process_id = ?1
            ",
            params![process_id],
            |row| {
                Ok((
                    row.get::<_, Option<i64>>(0)?,
                    row.get::<_, Option<i64>>(1)?,
                    row.get::<_, Option<i64>>(2)?,
                    row.get::<_, Option<i64>>(3)?,
                    row.get::<_, Option<i64>>(4)?,
                    row.get::<_, Option<i64>>(5)?,
                    row.get::<_, i64>(6)?,
                    row.get::<_, Option<String>>(7)?,
                    row.get::<_, Option<String>>(8)?,
                    row.get::<_, String>(9)?,
                ))
            },
        )
        .optional()
        .map_err(storage_unreadable)?;

    let Some(row) = row else {
        return Ok((
            CoverageVm {
                priced_coverage_bps: 0,
                landed_cost_coverage_bps: 0,
                net_qualified_coverage_bps: 0,
                priced_value_coverage_bps: 0,
                landed_cost_value_coverage_bps: 0,
                net_qualified_value_coverage_bps: 0,
                unresolved_mandatory_lots: 0,
            },
            None,
            None,
            PipelineState::Pending,
        ));
    };

    Ok((
        CoverageVm {
            priced_coverage_bps: row.0.unwrap_or_default(),
            landed_cost_coverage_bps: row.1.unwrap_or_default(),
            net_qualified_coverage_bps: row.2.unwrap_or_default(),
            priced_value_coverage_bps: row.3.unwrap_or_default(),
            landed_cost_value_coverage_bps: row.4.unwrap_or_default(),
            net_qualified_value_coverage_bps: row.5.unwrap_or_default(),
            unresolved_mandatory_lots: row.6,
        },
        row.7,
        row.8,
        PipelineState::from_db(&row.9),
    ))
}

#[allow(clippy::too_many_lines)]
fn load_session_rows(
    conn: &Connection,
    runtime_dir: &Path,
    cursor: Option<&CursorToken>,
    limit: usize,
    filters: &SessionListFilters,
) -> Result<Vec<SessionListItem>, QueryServiceError> {
    let phases = json_filter_values(&filters.phases)?;
    let execution_modes = json_filter_values(&filters.execution_modes)?;
    let fetch_limit = i64::try_from(limit + 1)
        .map_err(|_| QueryServiceError::invalid_filter("limit exceeds supported range"))?;
    let descriptor_instance_id = current_descriptor_instance_id(runtime_dir);
    let mut stmt = conn
        .prepare(
            "\
            WITH session_rows AS (
              SELECT id,
                     skill_path,
                     skill_version,
                     execution_mode,
                     status,
                     started_at,
                     completed_at,
                     CASE
                       WHEN started_at IS NOT NULL AND completed_at IS NOT NULL
                       THEN CAST((julianday(completed_at) - julianday(started_at)) * 86400000 AS INTEGER)
                       ELSE NULL
                     END AS duration_ms,
                     cleanup_verdict,
                     error_type,
                     exit_code,
                     COALESCE(last_checkpoint_seq, 0) AS checkpoint_count,
                     COALESCE(terminal_output_bytes, 0) AS byte_count,
                     structured_trace,
                     COALESCE(json_extract(structured_trace, '$.daemon_instance_id'), '') AS raw_daemon_instance_id,
                     COALESCE(
                       json_extract(structured_trace, '$.phase_history[#-1]'),
                       CASE status
                         WHEN 'created' THEN 'created'
                         WHEN 'spawned' THEN 'spawned'
                         WHEN 'active' THEN 'active'
                         WHEN 'terminating' THEN 'terminating'
                         WHEN 'exited' THEN 'exited'
                         WHEN 'reaped' THEN 'reaped'
                         ELSE 'archived'
                       END
                     ) AS session_phase
              FROM agent_session_logs
            )
            SELECT id, skill_path, skill_version, execution_mode, started_at, completed_at,
                   duration_ms, cleanup_verdict, error_type, exit_code, checkpoint_count,
                   byte_count, raw_daemon_instance_id, session_phase
            FROM session_rows
            WHERE (
                    json_array_length(?1) = 0
                    OR session_phase IN (SELECT value FROM json_each(?1))
                  )
              AND (
                    json_array_length(?2) = 0
                    OR execution_mode IN (SELECT value FROM json_each(?2))
                  )
              AND (
                    ?3 = 0
                    OR session_phase IN ('created', 'spawned', 'active', 'terminating')
                  )
              AND (
                    ?4 IS NULL
                    OR lower(id) LIKE '%' || lower(?4) || '%'
                    OR lower(skill_path) LIKE '%' || lower(?4) || '%'
                  )
              AND (
                    ?5 IS NULL
                    OR COALESCE(started_at, '') < ?5
                    OR (COALESCE(started_at, '') = ?5 AND id < ?6)
                  )
            ORDER BY COALESCE(started_at, '') DESC, id DESC
            LIMIT ?7
            ",
        )
        .map_err(storage_unreadable)?;
    let rows = stmt
        .query_map(
            params![
                phases,
                execution_modes,
                i64::from(filters.active_only),
                filters.search,
                cursor.map(|value| value.sort_at.as_str()),
                cursor.map(|value| value.stable_id.as_str()),
                fetch_limit,
            ],
            |row| {
                let execution_mode = ExecutionModeVm::from_db(&row.get::<_, String>(3)?);
                let session_phase = SessionPhaseVm::from_db(&row.get::<_, String>(13)?);
                let raw_daemon_instance_id: String = row.get(12)?;
                Ok(SessionListItem {
                    session_id: row.get(0)?,
                    daemon_instance_id: if raw_daemon_instance_id.is_empty() {
                        descriptor_instance_id.clone().unwrap_or_default()
                    } else {
                        raw_daemon_instance_id
                    },
                    skill_path: row.get(1)?,
                    skill_version: row.get(2)?,
                    execution_mode,
                    phase: session_phase,
                    started_at: row.get(4)?,
                    ended_at: row.get(5)?,
                    duration_ms: row.get(6)?,
                    cleanup_verdict: row
                        .get::<_, Option<String>>(7)?
                        .and_then(|value| CleanupVerdictVm::from_db(&value)),
                    error_type: row.get(8)?,
                    exit_code: row.get(9)?,
                    checkpoint_count: row.get(10)?,
                    byte_count: row.get(11)?,
                    reattachable: matches!(
                        session_phase,
                        SessionPhaseVm::Created
                            | SessionPhaseVm::Spawned
                            | SessionPhaseVm::Active
                            | SessionPhaseVm::Terminating
                    ) && execution_mode == ExecutionModeVm::Pty,
                })
            },
        )
        .map_err(storage_unreadable)?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(storage_unreadable)
}

fn load_error_rows(
    conn: &Connection,
    process_id: Option<&str>,
    cursor: Option<&CursorToken>,
    limit: usize,
    filters: &ErrorListFilters,
) -> Result<Vec<ErrorListItem>, QueryServiceError> {
    let severities = json_filter_values(&filters.severities)?;
    let pipeline_stages = json_filter_values(&filters.pipeline_stages)?;
    let fetch_limit = i64::try_from(limit + 1)
        .map_err(|_| QueryServiceError::invalid_filter("limit exceeds supported range"))?;
    let mut stmt = conn
        .prepare(
            "\
            WITH error_rows AS (
              SELECT em.id,
                     em.process_id,
                     em.pipeline_stage,
                     em.error_type,
                     em.error_message,
                     asl.cleanup_verdict,
                     em.skill_path,
                     em.created_at,
                     em.resolved,
                     em.agent_session_id,
                     CASE
                       WHEN asl.cleanup_verdict = 'leaked'
                         OR em.error_type IN (
                           'AGENT_CLEANUP_INCOMPLETE',
                           'AGENT_DAEMON_CRASH_DURING_SESSION',
                           'HOST_RESOURCE_EXHAUSTED',
                           'DAEMON_STARTUP_FAILED'
                         )
                       THEN 'critical'
                       WHEN em.error_type LIKE 'AGENT_%'
                         OR em.error_type LIKE 'SOURCE_%'
                         OR em.error_type IN (
                           'HOST_SLEEP_INTERRUPTED',
                           'AUTH_STATE_STALE',
                           'PAGE_SHAPE_DRIFT',
                           'LOCALE_OR_CURRENCY_DRIFT',
                           'NOTICE_PARSE_FAILED',
                           'GOODS_CLASSIFICATION_LOW_CONFIDENCE',
                           'SUPPLIER_MATCH_LOW_CONFIDENCE',
                           'COMMODITY_CODE_LOW_CONFIDENCE',
                           'REVENUE_BASIS_UNCERTAIN',
                           'FPO_ACCESS_UNAVAILABLE',
                           'SQLITE_WRITE_FAILED'
                         )
                       THEN 'warning'
                       ELSE 'info'
                     END AS severity
              FROM error_manifests em
              LEFT JOIN agent_session_logs asl ON asl.id = em.agent_session_id
            )
            SELECT id, process_id, pipeline_stage, error_type, error_message, cleanup_verdict,
                   skill_path, created_at, resolved, agent_session_id, severity
            FROM error_rows
            WHERE (?1 IS NULL OR process_id = ?1)
              AND (?2 IS NULL OR resolved = ?2)
              AND (
                    json_array_length(?3) = 0
                    OR severity IN (SELECT value FROM json_each(?3))
                  )
              AND (
                    json_array_length(?4) = 0
                    OR pipeline_stage IN (SELECT value FROM json_each(?4))
                  )
              AND (
                    ?5 IS NULL
                    OR lower(error_type) LIKE '%' || lower(?5) || '%'
                    OR lower(error_message) LIKE '%' || lower(?5) || '%'
                  )
              AND (
                    ?6 IS NULL
                    OR created_at < ?6
                    OR (created_at = ?6 AND id < ?7)
                  )
            ORDER BY created_at DESC, id DESC
            LIMIT ?8
            ",
        )
        .map_err(storage_unreadable)?;
    let rows = stmt
        .query_map(
            params![
                process_id,
                filters.resolved.map(i64::from),
                severities,
                pipeline_stages,
                filters.search,
                cursor.map(|value| value.sort_at.as_str()),
                cursor.map(|value| value.stable_id.as_str()),
                fetch_limit,
            ],
            map_error_row,
        )
        .map_err(storage_unreadable)?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(storage_unreadable)
}

fn map_error_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ErrorListItem> {
    let cleanup_verdict: Option<String> = row.get(5)?;
    let severity: String = row.get(10)?;
    Ok(ErrorListItem {
        error_manifest_id: row.get(0)?,
        process_id: row.get(1)?,
        pipeline_stage: row.get(2)?,
        error_type: row.get(3)?,
        error_message: row.get(4)?,
        cleanup_verdict: cleanup_verdict
            .as_deref()
            .and_then(CleanupVerdictVm::from_db),
        severity: match severity.as_str() {
            "critical" => ErrorSeverity::Critical,
            "warning" => ErrorSeverity::Warning,
            _ => ErrorSeverity::Info,
        },
        skill_path: row.get(6)?,
        created_at: row.get(7)?,
        resolved: row.get::<_, i64>(8)? == 1,
        agent_session_id: row.get(9)?,
    })
}

fn latest_decisions(
    conn: &Connection,
) -> Result<HashMap<String, DecisionRecord>, QueryServiceError> {
    if !table_exists(conn, "pending_operator_intents")? {
        return Ok(HashMap::new());
    }

    let mut stmt = conn
        .prepare(
            "\
            SELECT process_id, desired_disposition, status, reviewed_basis_hash, reviewed_at
            FROM pending_operator_intents
            WHERE superseded_by_intent_id IS NULL
            ORDER BY queued_at DESC, id DESC
            ",
        )
        .map_err(storage_unreadable)?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                DecisionRecord {
                    disposition: row.get(1)?,
                    status: row.get(2)?,
                    reviewed_basis_hash: row.get(3)?,
                    decided_at: row.get(4)?,
                },
            ))
        })
        .map_err(storage_unreadable)?;

    let mut decisions = HashMap::new();
    for row in rows {
        let (process_id, record) = row.map_err(storage_unreadable)?;
        decisions.entry(process_id).or_insert(record);
    }
    Ok(decisions)
}

fn load_manifest_inbox_rows(conn: &Connection) -> Result<Vec<ManifestInboxRow>, QueryServiceError> {
    if !table_exists(conn, "manifests")? {
        return Ok(Vec::new());
    }
    let decisions = latest_decisions(conn)?;
    let mut stmt = conn
        .prepare(
            "\
            SELECT m.process_id,
                   COALESCE(p.canonical_process_id, m.process_id) AS canonical_process_id,
                   m.stoplight,
                   m.suggested_action,
                   m.clarification_prompt_count,
                   m.manifest_basis_hash,
                   COALESCE(m.updated_at, m.created_at) AS generated_at,
                   (
                     SELECT COALESCE(SUM(nl.estimated_value_pence), 0)
                     FROM notice_lots nl
                     INNER JOIN contract_notices cn ON cn.id = nl.notice_id
                     WHERE cn.process_id = m.process_id
                   ) AS estimated_value_pence,
                   (
                     SELECT n.title
                     FROM contract_notices n
                     WHERE n.process_id = m.process_id
                     ORDER BY n.published_at DESC, n.id ASC
                     LIMIT 1
                   ) AS title,
                   (
                     SELECT n.buyer_name
                     FROM contract_notices n
                     WHERE n.process_id = m.process_id
                     ORDER BY n.published_at DESC, n.id ASC
                     LIMIT 1
                   ) AS buyer_name,
                   pa.decision_basis_hash AS decision_basis_hash
            FROM manifests m
            LEFT JOIN procurement_processes p ON p.id = m.process_id
            LEFT JOIN pipeline_assessments pa ON pa.process_id = m.process_id
            ORDER BY m.updated_at DESC, m.process_id ASC
            ",
        )
        .map_err(storage_unreadable)?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                ManifestInboxRow {
                    process_id: row.get::<_, String>(0)?,
                    canonical_process_id: row.get::<_, String>(1)?,
                    title: row.get::<_, Option<String>>(8)?,
                    buyer_name: row.get::<_, Option<String>>(9)?,
                    estimated_value_pence: row.get::<_, Option<i64>>(7)?,
                    stoplight: row.get::<_, Option<String>>(2)?,
                    suggested_action: row.get::<_, Option<String>>(3)?,
                    clarification_prompt_count: row.get::<_, i64>(4)?,
                    manifest_basis_hash: row.get::<_, Option<String>>(5)?,
                    generated_at: row.get::<_, String>(6)?,
                    decision: DecisionStatus {
                        disposition: None,
                        status: None,
                        is_stale: false,
                        decided_at: None,
                    },
                },
                row.get::<_, Option<String>>(10)?,
            ))
        })
        .map_err(storage_unreadable)?;
    let mut out: Vec<ManifestInboxRow> = Vec::new();
    for row in rows {
        let (mut row, current_decision_basis_hash) = row.map_err(storage_unreadable)?;
        row.decision = decision_status_for_process(
            decisions.get(&row.canonical_process_id),
            current_decision_basis_hash.as_deref(),
        );
        out.push(row);
    }
    Ok(out)
}

fn load_decision_history_rows(
    conn: &Connection,
    process_id: &str,
) -> Result<Vec<DecisionHistoryEntry>, QueryServiceError> {
    if !table_exists(conn, "pending_operator_intents")? {
        return Ok(Vec::new());
    }
    let mut stmt = conn
        .prepare(
            "\
            SELECT id, idempotency_key, desired_disposition, status,
                   reviewed_basis_hash, reviewed_manifest_hash, reviewed_at,
                   queued_at, superseded_by_intent_id, decision_note
            FROM pending_operator_intents
            WHERE process_id = ?1
            ORDER BY queued_at DESC, id DESC
            ",
        )
        .map_err(storage_unreadable)?;
    let rows = stmt
        .query_map(params![process_id], |row| {
            Ok(DecisionHistoryEntry {
                intent_id: row.get::<_, String>(0)?,
                idempotency_key: row.get::<_, String>(1)?,
                disposition: row.get::<_, String>(2)?,
                status: row.get::<_, String>(3)?,
                reviewed_basis_hash: row.get::<_, String>(4)?,
                reviewed_manifest_hash: row.get::<_, String>(5)?,
                reviewed_at: row.get::<_, String>(6)?,
                queued_at: row.get::<_, String>(7)?,
                superseded_by_intent_id: row.get::<_, Option<String>>(8)?,
                decision_note: row.get::<_, Option<String>>(9)?,
            })
        })
        .map_err(storage_unreadable)?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(storage_unreadable)
}

fn load_active_override_rows(
    conn: &Connection,
    process_id: &str,
) -> Result<Vec<ActiveOverrideEntry>, QueryServiceError> {
    if !table_exists(conn, "operator_input_overrides")? {
        return Ok(Vec::new());
    }
    let mut stmt = conn
        .prepare(
            "\
            SELECT id, product_candidate_id, reason_code, payload_json, notes,
                   basis_hash_when_provided, provided_at, provided_by
            FROM operator_input_overrides
            WHERE process_id = ?1
              AND withdrawn_at IS NULL
              AND superseded_by_override_id IS NULL
            ORDER BY provided_at DESC, id ASC
            ",
        )
        .map_err(storage_unreadable)?;
    let rows = stmt
        .query_map(params![process_id], |row| {
            Ok(ActiveOverrideEntry {
                override_id: row.get::<_, String>(0)?,
                product_candidate_id: row.get::<_, String>(1)?,
                reason_code: row.get::<_, String>(2)?,
                payload_json: row.get::<_, String>(3)?,
                notes: row.get::<_, Option<String>>(4)?,
                basis_hash_when_provided: row.get::<_, Option<String>>(5)?,
                provided_at: row.get::<_, String>(6)?,
                provided_by: row.get::<_, String>(7)?,
            })
        })
        .map_err(storage_unreadable)?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(storage_unreadable)
}

fn resolve_process_id(
    conn: &Connection,
    requested_process_id: &str,
) -> Result<String, QueryServiceError> {
    if table_exists(conn, "process_aliases")?
        && let Some(canonical_process_id) = conn
            .query_row(
                "SELECT canonical_process_id FROM process_aliases WHERE alias_process_id = ?1",
                params![requested_process_id],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(storage_unreadable)?
    {
        return Ok(canonical_process_id);
    }

    conn.query_row(
        "\
        SELECT id
        FROM procurement_processes
        WHERE id = ?1 OR canonical_process_id = ?1
        ORDER BY CASE WHEN id = ?1 THEN 0 ELSE 1 END
        LIMIT 1
        ",
        params![requested_process_id],
        |row| row.get::<_, String>(0),
    )
    .optional()
    .map_err(storage_unreadable)?
    .ok_or_else(|| QueryServiceError::not_found("process not found"))
}

fn read_pty_history(
    index_path: &Path,
    request: &SessionHistoryRequest,
) -> Result<SessionHistoryChunk, QueryServiceError> {
    if !index_path.exists() {
        return Err(QueryServiceError::artifact_unavailable(
            "session history metadata path is missing",
        ));
    }

    let index_contents = fs::read_to_string(index_path).map_err(|error| {
        QueryServiceError::artifact_unavailable(format!("reading stream index failed: {error}"))
    })?;
    let entries: Vec<StreamIndexEntry> = index_contents
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            serde_json::from_str::<StreamIndexEntry>(line).map_err(|error| {
                QueryServiceError::artifact_unavailable(format!(
                    "parsing stream index failed: {error}"
                ))
            })
        })
        .collect::<Result<_, _>>()?;

    let mut frames = Vec::new();
    let mut covered = BTreeSet::new();
    for entry in entries
        .iter()
        .filter(|entry| entry.last_seq >= request.from_seq && entry.first_seq <= request.to_seq)
    {
        let chunk_path = index_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(&entry.chunk);
        let encoded = fs::read(&chunk_path).map_err(|error| {
            QueryServiceError::artifact_unavailable(format!(
                "reading archived PTY chunk {} failed: {error}",
                chunk_path.display()
            ))
        })?;
        let decoded = zstd::stream::decode_all(encoded.as_slice()).map_err(|error| {
            QueryServiceError::artifact_unavailable(format!(
                "decoding archived PTY chunk {} failed: {error}",
                chunk_path.display()
            ))
        })?;
        let chunk_frames = decode_pty_chunk_frames(&decoded, entry.first_seq, entry.last_seq);
        for frame in chunk_frames {
            let SessionFrame::Pty { seq, .. } = &frame else {
                continue;
            };
            if *seq < request.from_seq || *seq > request.to_seq {
                continue;
            }
            covered.insert(*seq);
            frames.push(frame);
        }
    }

    frames.extend(gap_frames_for_range(
        &covered,
        request.from_seq,
        request.to_seq,
    ));
    limit_history_payload(&mut frames, request.max_bytes);

    Ok(SessionHistoryChunk {
        session_id: request.session_id.clone(),
        first_seq: request.from_seq,
        last_seq: request.to_seq,
        availability: availability_for_range(&covered, request.from_seq, request.to_seq),
        frames,
    })
}

fn read_contract_history(
    output_path: &Path,
    execution_mode: &str,
    request: &SessionHistoryRequest,
) -> Result<SessionHistoryChunk, QueryServiceError> {
    let content = fs::read_to_string(output_path).map_err(|error| {
        QueryServiceError::artifact_unavailable(format!(
            "reading contract output {} failed: {error}",
            output_path.display()
        ))
    })?;
    let mut frames = Vec::new();
    let mut covered = BTreeSet::new();
    if execution_mode == "contract" {
        let (stdout_text, stderr_text) = split_contract_output(&content);
        if request.from_seq <= 1 && request.to_seq >= 1 && !stdout_text.is_empty() {
            frames.push(SessionFrame::Stdout {
                seq: 1,
                chunk: stdout_text.to_owned(),
                ts: String::new(),
            });
            covered.insert(1);
        }
        if request.from_seq <= 2 && request.to_seq >= 2 && !stderr_text.is_empty() {
            frames.push(SessionFrame::Stderr {
                seq: 2,
                chunk: stderr_text.to_owned(),
                ts: String::new(),
            });
            covered.insert(2);
        }
    }

    frames.extend(gap_frames_for_range(
        &covered,
        request.from_seq,
        request.to_seq,
    ));
    limit_history_payload(&mut frames, request.max_bytes);

    Ok(SessionHistoryChunk {
        session_id: request.session_id.clone(),
        first_seq: request.from_seq,
        last_seq: request.to_seq,
        availability: availability_for_range(&covered, request.from_seq, request.to_seq),
        frames,
    })
}

fn split_contract_output(content: &str) -> (&str, &str) {
    let stripped = content.strip_prefix("--- stdout ---\n").unwrap_or(content);
    let mut parts = stripped.splitn(2, "\n--- stderr ---\n");
    let stdout_text = parts.next().unwrap_or_default();
    let stderr_text = parts.next().unwrap_or_default();
    (stdout_text, stderr_text)
}

fn limit_history_payload(frames: &mut Vec<SessionFrame>, max_bytes: Option<usize>) {
    let Some(max_bytes) = max_bytes else {
        return;
    };
    let mut used = 0usize;
    frames.retain(|frame| {
        let next = match frame {
            SessionFrame::Pty { chunk, .. } => chunk.len(),
            SessionFrame::Stdout { chunk, .. } | SessionFrame::Stderr { chunk, .. } => chunk.len(),
            SessionFrame::Lifecycle { .. }
            | SessionFrame::Event { .. }
            | SessionFrame::Gap { .. } => 0,
        };
        if used + next > max_bytes {
            return false;
        }
        used += next;
        true
    });
}

fn decode_pty_chunk_frames(decoded: &[u8], first_seq: u64, last_seq: u64) -> Vec<SessionFrame> {
    split_pty_frames(decoded)
        .into_iter()
        .take(usize::try_from(last_seq.saturating_sub(first_seq) + 1).unwrap_or(usize::MAX))
        .enumerate()
        .map(|(offset, chunk)| SessionFrame::Pty {
            seq: first_seq + u64::try_from(offset).unwrap_or(0),
            chunk,
            ts: String::new(),
        })
        .collect()
}

fn split_pty_frames(decoded: &[u8]) -> Vec<Vec<u8>> {
    let mut frames = Vec::new();
    let mut start = 0usize;
    for (index, byte) in decoded.iter().enumerate() {
        if *byte != b'\n' {
            continue;
        }
        if start <= index {
            frames.push(decoded[start..=index].to_vec());
        }
        start = index + 1;
    }
    if start < decoded.len() {
        frames.push(decoded[start..].to_vec());
    }
    frames
        .into_iter()
        .filter(|frame| !frame.is_empty())
        .collect()
}

fn gap_frames_for_range(covered: &BTreeSet<u64>, from_seq: u64, to_seq: u64) -> Vec<SessionFrame> {
    let mut gaps = Vec::new();
    let mut gap_start: Option<u64> = None;
    for seq in from_seq..=to_seq {
        if covered.contains(&seq) {
            if let Some(start) = gap_start.take() {
                gaps.push(SessionFrame::Gap {
                    from_seq: start,
                    to_seq: seq.saturating_sub(1),
                });
            }
            continue;
        }
        gap_start.get_or_insert(seq);
    }
    if let Some(start) = gap_start {
        gaps.push(SessionFrame::Gap {
            from_seq: start,
            to_seq,
        });
    }
    if covered.is_empty() {
        return Vec::new();
    }
    gaps
}

fn availability_for_range(
    covered: &BTreeSet<u64>,
    from_seq: u64,
    to_seq: u64,
) -> ReplayAvailability {
    let requested: BTreeSet<u64> = (from_seq..=to_seq).collect();
    if covered.is_empty() {
        return ReplayAvailability::Missing;
    }
    if covered == &requested {
        return ReplayAvailability::Complete;
    }
    ReplayAvailability::Partial
}

fn validate_process_filters(filters: &ProcessListFilters) -> Result<(), QueryServiceError> {
    validate_filter_values(
        &filters.pipeline_states,
        &[
            "pending",
            "qualifying",
            "enriching",
            "qualified",
            "review",
            "rejected",
            "error",
        ],
        "unsupported pipeline state filter",
    )?;
    validate_filter_values(
        &filters.decision_dispositions,
        &["approve", "reject", "watch"],
        "unsupported decision disposition filter",
    )?;
    Ok(())
}

fn validate_session_filters(filters: &SessionListFilters) -> Result<(), QueryServiceError> {
    validate_filter_values(
        &filters.phases,
        &[
            "created",
            "spawned",
            "active",
            "terminating",
            "exited",
            "reaped",
            "archived",
        ],
        "unsupported session phase filter",
    )?;
    validate_filter_values(
        &filters.execution_modes,
        &["pty", "contract"],
        "unsupported execution mode filter",
    )?;
    Ok(())
}

fn validate_error_filters(filters: &ErrorListFilters) -> Result<(), QueryServiceError> {
    validate_filter_values(
        &filters.severities,
        &["critical", "warning", "info"],
        "unsupported severity filter",
    )?;
    Ok(())
}

fn validate_filter_values(
    values: &[String],
    allowed: &[&str],
    message: &str,
) -> Result<(), QueryServiceError> {
    if values.is_empty() {
        return Ok(());
    }
    let allowed: HashSet<&str> = allowed.iter().copied().collect();
    if values.iter().all(|value| allowed.contains(value.as_str())) {
        return Ok(());
    }
    Err(QueryServiceError::invalid_filter(message))
}

fn validate_limit(limit: usize) -> Result<usize, QueryServiceError> {
    if limit == 0 || limit > MAX_PAGE_LIMIT {
        return Err(QueryServiceError::invalid_filter(format!(
            "limit must be between 1 and {MAX_PAGE_LIMIT}"
        )));
    }
    Ok(limit)
}

fn decode_cursor(cursor: Option<&str>) -> Result<Option<CursorToken>, QueryServiceError> {
    cursor
        .map(|raw| {
            serde_json::from_str::<CursorToken>(raw).map_err(|error| {
                QueryServiceError::invalid_cursor(format!("cursor could not be parsed: {error}"))
            })
        })
        .transpose()
}

fn build_page<T, F>(mut items: Vec<T>, limit: usize, key: F) -> KeysetPage<T>
where
    F: Fn(&T) -> CursorToken,
{
    let effective_limit = if limit == 0 {
        DEFAULT_PAGE_LIMIT
    } else {
        limit
    };
    let has_more = items.len() > effective_limit;
    if has_more {
        items.truncate(effective_limit);
    }
    let next_cursor = items
        .last()
        .map(|item| serde_json::to_string(&key(item)).unwrap_or_else(|_| String::new()));
    KeysetPage {
        next_cursor: if has_more { next_cursor } else { None },
        has_more,
        items,
    }
}

fn decision_status_for_process(
    decision: Option<&DecisionRecord>,
    current_basis_hash: Option<&str>,
) -> DecisionStatus {
    let Some(decision) = decision else {
        return DecisionStatus {
            disposition: None,
            status: None,
            is_stale: false,
            decided_at: None,
        };
    };
    let basis_drifted = matches!(
        (&decision.reviewed_basis_hash, current_basis_hash),
        (Some(reviewed), Some(current)) if reviewed != current
    );
    let is_stale = decision.status == "stale" || (decision.status == "queued" && basis_drifted);
    DecisionStatus {
        disposition: Some(decision.disposition.clone()),
        status: Some(decision.status.clone()),
        is_stale,
        decided_at: decision.decided_at.clone(),
    }
}

fn runtime_dir_for_db(db_path: &Path) -> PathBuf {
    let parent = db_path.parent().unwrap_or_else(|| Path::new("."));
    let db_name = db_path.file_name().and_then(|value| value.to_str());
    let parent_name = parent.file_name().and_then(|value| value.to_str());
    if db_name == Some("state.db") && parent_name == Some("state") {
        return parent.parent().unwrap_or(parent).join("runtime");
    }
    parent.join("runtime")
}

fn current_descriptor_instance_id(runtime_dir: &Path) -> Option<String> {
    let descriptor_path = runtime_dir.join("daemon.info.json");
    let contents = fs::read_to_string(descriptor_path).ok()?;
    let value = serde_json::from_str::<serde_json::Value>(&contents).ok()?;
    value
        .get("instanceId")
        .and_then(serde_json::Value::as_str)
        .map(ToOwned::to_owned)
}

fn json_filter_values(values: &[String]) -> Result<String, QueryServiceError> {
    serde_json::to_string(values).map_err(|error| {
        QueryServiceError::invalid_filter(format!("filter encoding failed: {error}"))
    })
}

fn validate_storage(
    conn: &Connection,
    expected_schema_epoch: i64,
) -> Result<(), QueryServiceError> {
    let schema_epoch: i64 = conn
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .map_err(storage_unreadable)?;
    if schema_epoch != 0 && schema_epoch != expected_schema_epoch {
        return Err(QueryServiceError::new(
            QueryServiceErrorCode::SchemaMismatch,
            format!(
                "schema epoch mismatch: database has {schema_epoch}, desktop expects {expected_schema_epoch}"
            ),
            false,
        ));
    }

    let quick_check: String = conn
        .query_row("PRAGMA quick_check", [], |row| row.get(0))
        .map_err(storage_unreadable)?;
    if quick_check != "ok" {
        return Err(QueryServiceError::new(
            QueryServiceErrorCode::StorageUnreadable,
            format!("PRAGMA quick_check failed: {quick_check}"),
            true,
        ));
    }

    Ok(())
}

fn read_commit_seq(conn: &Connection) -> Result<i64, QueryServiceError> {
    conn.query_row(
        "SELECT commit_seq FROM commit_clock WHERE id = 1",
        [],
        |row| row.get(0),
    )
    .map_err(storage_unreadable)
}

fn snapshot_at(conn: &Connection) -> Result<String, QueryServiceError> {
    conn.query_row("SELECT strftime('%Y-%m-%dT%H:%M:%fZ', 'now')", [], |row| {
        row.get(0)
    })
    .map_err(storage_unreadable)
}

fn table_exists(conn: &Connection, table: &str) -> Result<bool, QueryServiceError> {
    conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
        params![table],
        |row| row.get::<_, i64>(0),
    )
    .map(|count| count == 1)
    .map_err(storage_unreadable)
}

#[allow(clippy::needless_pass_by_value)]
fn storage_unreadable(error: rusqlite::Error) -> QueryServiceError {
    let is_busy = matches!(
        error.sqlite_error_code(),
        Some(rusqlite::ErrorCode::DatabaseBusy | rusqlite::ErrorCode::DatabaseLocked)
    );
    QueryServiceError::new(
        if is_busy {
            QueryServiceErrorCode::Busy
        } else {
            QueryServiceErrorCode::StorageUnreadable
        },
        format!("query-service storage read failed: {error}"),
        true,
    )
}
