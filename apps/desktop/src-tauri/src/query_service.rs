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
const EXPECTED_SCHEMA_EPOCH: i64 = 1;

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
        let runtime_dir = db_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("runtime");
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
            let decisions = latest_decisions(conn)?;
            let process_sources = process_sources(conn)?;
            let mut items = load_process_list_rows(conn, &decisions)?;

            items.retain(|item| {
                process_matches_filters(item, filters, process_sources.get(&item.process_id))
            });
            items = apply_cursor(items, cursor.as_ref(), |item| {
                CursorToken::new(&item.published_at, &item.process_id)
            });

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
            let mut items = load_session_rows(conn, &self.runtime_dir)?;
            items.retain(|item| session_matches_filters(item, filters));
            items = apply_cursor(items, cursor.as_ref(), |item| {
                CursorToken::new(item.started_at.as_deref().unwrap_or(""), &item.session_id)
            });

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
            let mut items = load_error_rows(conn, None, filters)?;
            items.retain(|item| error_matches_filters(item, filters));
            items = apply_cursor(items, cursor.as_ref(), |item| {
                CursorToken::new(&item.created_at, &item.error_manifest_id)
            });

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

    const fn as_filter_value(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Qualifying => "qualifying",
            Self::Enriching => "enriching",
            Self::Qualified => "qualified",
            Self::Review => "review",
            Self::Rejected => "rejected",
            Self::Error => "error",
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

    const fn as_filter_value(self) -> &'static str {
        match self {
            Self::Pty => "pty",
            Self::Contract => "contract",
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

    const fn as_filter_value(self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Spawned => "spawned",
            Self::Active => "active",
            Self::Terminating => "terminating",
            Self::Exited => "exited",
            Self::Reaped => "reaped",
            Self::Archived => "archived",
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

impl ErrorSeverity {
    const fn as_filter_value(self) -> &'static str {
        match self {
            Self::Critical => "critical",
            Self::Warning => "warning",
            Self::Info => "info",
        }
    }
}

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
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum SessionFrame {
    Pty { seq: u64, text: String },
    Stdout { seq: u64, text: String },
    Stderr { seq: u64, text: String },
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
    decisions: &HashMap<String, DecisionRecord>,
) -> Result<Vec<ProcessListItem>, QueryServiceError> {
    let mut stmt = conn
        .prepare(
            "\
            SELECT p.id,
                   COALESCE(p.canonical_process_id, p.id),
                   p.title,
                   COALESCE(p.buyer_name, ''),
                   p.latest_published_at,
                   p.latest_deadline_at,
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
                   ),
                   COALESCE(
                     (
                       SELECT COUNT(*)
                       FROM notice_lots nl
                       INNER JOIN contract_notices cn ON cn.id = nl.notice_id
                       WHERE cn.process_id = p.id
                     ),
                     0
                   ),
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
                   ),
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
                   ),
                   COALESCE(pa.pipeline_state, 'pending'),
                   pa.priced_coverage_bps,
                   pa.net_qualified_coverage_bps,
                   pa.net_qualified_value_coverage_bps,
                   pa.gross_margin_bps,
                   pa.net_margin_bps,
                   COALESCE((SELECT COUNT(*) FROM product_candidates pc WHERE pc.process_id = p.id), 0),
                   COALESCE((SELECT COUNT(*) FROM supplier_quotes sq WHERE sq.process_id = p.id), 0),
                   pa.decision_basis_hash
            FROM procurement_processes p
            LEFT JOIN pipeline_assessments pa ON pa.process_id = p.id
            ORDER BY p.latest_published_at DESC, p.id DESC
            ",
        )
        .map_err(storage_unreadable)?;

    let rows = stmt
        .query_map([], |row| {
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
            ))
        })
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
            decision_basis_hash,
        ) = row.map_err(storage_unreadable)?;

        Ok(ProcessListItem {
            process_id: process_id.clone(),
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
            decision: decision_status_for_process(
                decisions.get(&process_id),
                decision_basis_hash.as_deref(),
            ),
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
            SELECT markdown, artifact_locator, artifact_hash, stage_run_id, created_at
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
                ))
            },
        )
        .optional()
        .map_err(storage_unreadable)?;

    let Some((markdown, artifact_locator, artifact_hash, stage_run_id, generated_at)) =
        manifest_row
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

fn load_session_rows(
    conn: &Connection,
    runtime_dir: &Path,
) -> Result<Vec<SessionListItem>, QueryServiceError> {
    let mut stmt = conn
        .prepare(
            "\
            SELECT id, skill_path, skill_version, execution_mode, status, started_at, completed_at,
                   CASE
                     WHEN started_at IS NOT NULL AND completed_at IS NOT NULL
                     THEN CAST((julianday(completed_at) - julianday(started_at)) * 86400000 AS INTEGER)
                     ELSE NULL
                   END AS duration_ms,
                   cleanup_verdict, error_type, exit_code,
                   COALESCE(last_checkpoint_seq, 0), COALESCE(terminal_output_bytes, 0),
                   structured_trace
            FROM agent_session_logs
            ORDER BY started_at DESC, id DESC
            ",
        )
        .map_err(storage_unreadable)?;
    let rows = stmt
        .query_map([], |row| {
            let phase = row.get::<_, String>(4)?;
            let structured_trace = row.get::<_, Option<String>>(13)?;
            let execution_mode = ExecutionModeVm::from_db(&row.get::<_, String>(3)?);
            let session_phase = canonical_phase(structured_trace.as_deref(), &phase);
            Ok(SessionListItem {
                session_id: row.get(0)?,
                daemon_instance_id: structured_trace
                    .as_deref()
                    .and_then(trace_daemon_instance_id)
                    .or_else(|| current_descriptor_instance_id(runtime_dir))
                    .unwrap_or_default(),
                skill_path: row.get(1)?,
                skill_version: row.get(2)?,
                execution_mode,
                phase: session_phase,
                started_at: row.get(5)?,
                ended_at: row.get(6)?,
                duration_ms: row.get(7)?,
                cleanup_verdict: row
                    .get::<_, Option<String>>(8)?
                    .and_then(|value| CleanupVerdictVm::from_db(&value)),
                error_type: row.get(9)?,
                exit_code: row.get(10)?,
                checkpoint_count: row.get(11)?,
                byte_count: row.get(12)?,
                reattachable: matches!(
                    session_phase,
                    SessionPhaseVm::Created
                        | SessionPhaseVm::Spawned
                        | SessionPhaseVm::Active
                        | SessionPhaseVm::Terminating
                ) && execution_mode == ExecutionModeVm::Pty,
            })
        })
        .map_err(storage_unreadable)?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(storage_unreadable)
}

fn load_error_rows(
    conn: &Connection,
    process_id: Option<&str>,
    filters: &ErrorListFilters,
) -> Result<Vec<ErrorListItem>, QueryServiceError> {
    let items = if let Some(process_id) = process_id {
        let mut stmt = conn
            .prepare(
                "\
                SELECT em.id, em.process_id, em.pipeline_stage, em.error_type, em.error_message,
                       asl.cleanup_verdict, em.skill_path, em.created_at, em.resolved, em.agent_session_id
                FROM error_manifests em
                LEFT JOIN agent_session_logs asl ON asl.id = em.agent_session_id
                WHERE em.process_id = ?1
                ORDER BY em.created_at DESC, em.id DESC
                ",
            )
            .map_err(storage_unreadable)?;
        let rows = stmt
            .query_map(params![process_id], map_error_row)
            .map_err(storage_unreadable)?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(storage_unreadable)?
    } else {
        let mut stmt = conn
            .prepare(
                "\
                SELECT em.id, em.process_id, em.pipeline_stage, em.error_type, em.error_message,
                       asl.cleanup_verdict, em.skill_path, em.created_at, em.resolved, em.agent_session_id
                FROM error_manifests em
                LEFT JOIN agent_session_logs asl ON asl.id = em.agent_session_id
                ORDER BY em.created_at DESC, em.id DESC
                ",
            )
            .map_err(storage_unreadable)?;
        let rows = stmt
            .query_map([], map_error_row)
            .map_err(storage_unreadable)?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(storage_unreadable)?
    };
    Ok(items
        .into_iter()
        .filter(|item| error_matches_filters(item, filters))
        .collect())
}

fn map_error_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ErrorListItem> {
    let error_type: String = row.get(3)?;
    let cleanup_verdict: Option<String> = row.get(5)?;
    Ok(ErrorListItem {
        error_manifest_id: row.get(0)?,
        process_id: row.get(1)?,
        pipeline_stage: row.get(2)?,
        error_type: error_type.clone(),
        error_message: row.get(4)?,
        cleanup_verdict: cleanup_verdict
            .as_deref()
            .and_then(CleanupVerdictVm::from_db),
        severity: derive_severity(&error_type, cleanup_verdict.as_deref()),
        skill_path: row.get(6)?,
        created_at: row.get(7)?,
        resolved: row.get::<_, i64>(8)? == 1,
        agent_session_id: row.get(9)?,
    })
}

fn process_sources(
    conn: &Connection,
) -> Result<HashMap<String, HashSet<String>>, QueryServiceError> {
    let mut stmt = conn
        .prepare("SELECT process_id, source_id FROM contract_notices")
        .map_err(storage_unreadable)?;
    let rows = stmt
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(storage_unreadable)?;
    let mut map = HashMap::<String, HashSet<String>>::new();
    for row in rows {
        let (process_id, source_id) = row.map_err(storage_unreadable)?;
        map.entry(process_id).or_default().insert(source_id);
    }
    Ok(map)
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
            SELECT process_id, desired_disposition, reviewed_basis_hash, reviewed_at
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
                    reviewed_basis_hash: row.get(2)?,
                    decided_at: row.get(3)?,
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
        for seq in entry.first_seq.max(request.from_seq)..=entry.last_seq.min(request.to_seq) {
            covered.insert(seq);
        }
        frames.push(SessionFrame::Pty {
            seq: entry.first_seq,
            text: String::from_utf8_lossy(&decoded).into_owned(),
        });
    }

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
                text: stdout_text.to_owned(),
            });
            covered.insert(1);
        }
        if request.from_seq <= 2 && request.to_seq >= 2 && !stderr_text.is_empty() {
            frames.push(SessionFrame::Stderr {
                seq: 2,
                text: stderr_text.to_owned(),
            });
            covered.insert(2);
        }
    }

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
            SessionFrame::Pty { text, .. }
            | SessionFrame::Stdout { text, .. }
            | SessionFrame::Stderr { text, .. } => text.len(),
        };
        if used + next > max_bytes {
            return false;
        }
        used += next;
        true
    });
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

fn process_matches_filters(
    item: &ProcessListItem,
    filters: &ProcessListFilters,
    source_ids: Option<&HashSet<String>>,
) -> bool {
    if !filters.source_ids.is_empty() {
        let Some(source_ids) = source_ids else {
            return false;
        };
        if !filters
            .source_ids
            .iter()
            .any(|source_id| source_ids.contains(source_id))
        {
            return false;
        }
    }

    if !filters.pipeline_states.is_empty()
        && !filters
            .pipeline_states
            .iter()
            .any(|state| *state == item.pipeline_state.as_filter_value())
    {
        return false;
    }

    if !filters.decision_dispositions.is_empty() {
        let Some(disposition) = &item.decision.disposition else {
            return false;
        };
        if !filters
            .decision_dispositions
            .iter()
            .any(|candidate| candidate == disposition)
        {
            return false;
        }
    }

    if filters.stale_only && !item.decision.is_stale {
        return false;
    }

    if let Some(search) = &filters.search {
        let needle = search.to_lowercase();
        if !item.title.to_lowercase().contains(&needle)
            && !item.buyer_name.to_lowercase().contains(&needle)
        {
            return false;
        }
    }

    true
}

fn session_matches_filters(item: &SessionListItem, filters: &SessionListFilters) -> bool {
    if !filters.phases.is_empty()
        && !filters
            .phases
            .iter()
            .any(|phase| *phase == item.phase.as_filter_value())
    {
        return false;
    }
    if !filters.execution_modes.is_empty()
        && !filters
            .execution_modes
            .iter()
            .any(|mode| *mode == item.execution_mode.as_filter_value())
    {
        return false;
    }
    if filters.active_only
        && !matches!(
            item.phase,
            SessionPhaseVm::Created
                | SessionPhaseVm::Spawned
                | SessionPhaseVm::Active
                | SessionPhaseVm::Terminating
        )
    {
        return false;
    }
    if let Some(search) = &filters.search {
        let needle = search.to_lowercase();
        if !item.session_id.to_lowercase().contains(&needle)
            && !item.skill_path.to_lowercase().contains(&needle)
        {
            return false;
        }
    }
    true
}

fn error_matches_filters(item: &ErrorListItem, filters: &ErrorListFilters) -> bool {
    if let Some(resolved) = filters.resolved
        && item.resolved != resolved
    {
        return false;
    }
    if !filters.severities.is_empty()
        && !filters
            .severities
            .iter()
            .any(|severity| *severity == item.severity.as_filter_value())
    {
        return false;
    }
    if !filters.pipeline_stages.is_empty() {
        let Some(stage) = &item.pipeline_stage else {
            return false;
        };
        if !filters
            .pipeline_stages
            .iter()
            .any(|candidate| candidate == stage)
        {
            return false;
        }
    }
    if let Some(search) = &filters.search {
        let needle = search.to_lowercase();
        if !item.error_type.to_lowercase().contains(&needle)
            && !item.error_message.to_lowercase().contains(&needle)
        {
            return false;
        }
    }
    true
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

fn apply_cursor<T, F>(items: Vec<T>, cursor: Option<&CursorToken>, key: F) -> Vec<T>
where
    F: Fn(&T) -> CursorToken,
{
    let Some(cursor) = cursor else {
        return items;
    };
    items
        .into_iter()
        .filter(|item| {
            let cursor_key = key(item);
            cursor_key.sort_at < cursor.sort_at
                || (cursor_key.sort_at == cursor.sort_at && cursor_key.stable_id < cursor.stable_id)
        })
        .collect()
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
            is_stale: false,
            decided_at: None,
        };
    };
    let is_stale = matches!(
        (&decision.reviewed_basis_hash, current_basis_hash),
        (Some(reviewed), Some(current)) if reviewed != current
    );
    DecisionStatus {
        disposition: Some(decision.disposition.clone()),
        is_stale,
        decided_at: decision.decided_at.clone(),
    }
}

fn canonical_phase(structured_trace: Option<&str>, status: &str) -> SessionPhaseVm {
    phase_history(structured_trace, status)
        .last()
        .map_or(SessionPhaseVm::Archived, |value| {
            SessionPhaseVm::from_db(value)
        })
}

fn phase_history(structured_trace: Option<&str>, status: &str) -> Vec<String> {
    structured_trace
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(raw).ok())
        .and_then(|value| {
            value
                .get("phase_history")
                .and_then(serde_json::Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(serde_json::Value::as_str)
                        .map(ToOwned::to_owned)
                        .collect::<Vec<_>>()
                })
        })
        .filter(|history| !history.is_empty())
        .unwrap_or_else(|| default_phase_history(status))
}

fn default_phase_history(status: &str) -> Vec<String> {
    match status {
        "created" => vec!["created".to_owned()],
        "spawned" => vec!["created".to_owned(), "spawned".to_owned()],
        "active" => vec![
            "created".to_owned(),
            "spawned".to_owned(),
            "active".to_owned(),
        ],
        "terminating" => vec![
            "created".to_owned(),
            "spawned".to_owned(),
            "active".to_owned(),
            "terminating".to_owned(),
        ],
        "exited" => vec![
            "created".to_owned(),
            "spawned".to_owned(),
            "active".to_owned(),
            "terminating".to_owned(),
            "exited".to_owned(),
        ],
        "reaped" => vec![
            "created".to_owned(),
            "spawned".to_owned(),
            "active".to_owned(),
            "terminating".to_owned(),
            "exited".to_owned(),
            "reaped".to_owned(),
        ],
        _ => vec![
            "created".to_owned(),
            "spawned".to_owned(),
            "active".to_owned(),
            "terminating".to_owned(),
            "exited".to_owned(),
            "reaped".to_owned(),
            "archived".to_owned(),
        ],
    }
}

fn derive_severity(error_type: &str, cleanup_verdict: Option<&str>) -> ErrorSeverity {
    if cleanup_verdict == Some("leaked")
        || matches!(
            error_type,
            "AGENT_CLEANUP_INCOMPLETE"
                | "AGENT_DAEMON_CRASH_DURING_SESSION"
                | "HOST_RESOURCE_EXHAUSTED"
                | "DAEMON_STARTUP_FAILED"
        )
    {
        return ErrorSeverity::Critical;
    }

    if error_type.starts_with("AGENT_")
        || matches!(
            error_type,
            "HOST_SLEEP_INTERRUPTED"
                | "AUTH_STATE_STALE"
                | "PAGE_SHAPE_DRIFT"
                | "LOCALE_OR_CURRENCY_DRIFT"
                | "NOTICE_PARSE_FAILED"
                | "GOODS_CLASSIFICATION_LOW_CONFIDENCE"
                | "SUPPLIER_MATCH_LOW_CONFIDENCE"
                | "COMMODITY_CODE_LOW_CONFIDENCE"
                | "REVENUE_BASIS_UNCERTAIN"
                | "FPO_ACCESS_UNAVAILABLE"
                | "SQLITE_WRITE_FAILED"
        )
        || error_type.starts_with("SOURCE_")
    {
        return ErrorSeverity::Warning;
    }

    ErrorSeverity::Info
}

fn trace_daemon_instance_id(structured_trace: &str) -> Option<String> {
    serde_json::from_str::<serde_json::Value>(structured_trace)
        .ok()
        .and_then(|value| {
            value
                .get("daemon_instance_id")
                .and_then(serde_json::Value::as_str)
                .map(ToOwned::to_owned)
        })
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
