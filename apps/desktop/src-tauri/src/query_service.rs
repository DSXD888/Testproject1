#![forbid(unsafe_code)]

use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use rusqlite::{
    Connection, OpenFlags, OptionalExtension, Transaction, TransactionBehavior, params,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

const DEFAULT_SCHEMA_EPOCH: i64 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryEnvelope<T> {
    pub data: T,
    pub snapshot_seq: i64,
    pub snapshot_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeysetPage<T> {
    pub items: Vec<T>,
    pub next_cursor: Option<String>,
    pub has_more: bool,
}

#[derive(Debug)]
pub struct QueryService {
    conn: Connection,
    capabilities: QueryCapabilities,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct QueryCapabilities {
    process_aliases: bool,
    pending_operator_intents: bool,
}

type CoverageRow = (
    Option<i64>,
    Option<i64>,
    Option<i64>,
    Option<i64>,
    Option<i64>,
    Option<i64>,
    i64,
    Option<String>,
);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QueryServiceErrorCode {
    InvalidCursor,
    InvalidFilter,
    NotFound,
    Busy,
    StorageUnreadable,
    SchemaMismatch,
    ArtifactUnavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryServiceError {
    pub code: QueryServiceErrorCode,
    pub message: String,
    pub retryable: bool,
}

impl std::fmt::Display for QueryServiceError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{:?}: {}", self.code, self.message)
    }
}

impl std::error::Error for QueryServiceError {}

impl QueryServiceError {
    fn invalid_cursor(message: impl Into<String>) -> Self {
        Self {
            code: QueryServiceErrorCode::InvalidCursor,
            message: message.into(),
            retryable: false,
        }
    }

    fn invalid_filter(message: impl Into<String>) -> Self {
        Self {
            code: QueryServiceErrorCode::InvalidFilter,
            message: message.into(),
            retryable: false,
        }
    }

    fn not_found(message: impl Into<String>) -> Self {
        Self {
            code: QueryServiceErrorCode::NotFound,
            message: message.into(),
            retryable: false,
        }
    }

    fn busy(message: impl Into<String>) -> Self {
        Self {
            code: QueryServiceErrorCode::Busy,
            message: message.into(),
            retryable: true,
        }
    }

    fn storage_unreadable(message: impl Into<String>) -> Self {
        Self {
            code: QueryServiceErrorCode::StorageUnreadable,
            message: message.into(),
            retryable: true,
        }
    }

    fn schema_mismatch(message: impl Into<String>) -> Self {
        Self {
            code: QueryServiceErrorCode::SchemaMismatch,
            message: message.into(),
            retryable: false,
        }
    }

    fn artifact_unavailable(message: impl Into<String>) -> Self {
        Self {
            code: QueryServiceErrorCode::ArtifactUnavailable,
            message: message.into(),
            retryable: false,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessListFilters {
    pub source_ids: Vec<String>,
    pub pipeline_states: Vec<String>,
    pub decision_dispositions: Vec<String>,
    pub stale_only: bool,
    pub search: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessDecision {
    pub disposition: Option<String>,
    pub is_stale: bool,
    pub decided_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessListItem {
    pub process_id: String,
    pub canonical_process_id: String,
    pub title: String,
    pub buyer_name: String,
    pub published_at: String,
    pub deadline_at: Option<String>,
    pub pipeline_state: String,
    pub priced_coverage_bps: Option<i64>,
    pub net_qualified_coverage_bps: Option<i64>,
    pub net_qualified_value_coverage_bps: Option<i64>,
    pub gross_margin_bps: Option<i64>,
    pub net_margin_base_bps: Option<i64>,
    pub candidate_count: i64,
    pub quoted_candidate_count: i64,
    pub decision: ProcessDecision,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractNoticeVm {
    pub notice_id: String,
    pub source_id: String,
    pub source_notice_id: String,
    pub title: String,
    pub buyer_name: Option<String>,
    pub procurement_stage: String,
    pub access_route: Option<String>,
    pub deadline_at: Option<String>,
    pub published_at: String,
    pub estimated_value_gbp_ex_vat_pence: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NoticeLotVm {
    pub lot_id: String,
    pub notice_id: String,
    pub lot_number: Option<i64>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub estimated_value_pence: Option<i64>,
    pub deadline_at: Option<String>,
    pub is_bidable_separately: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductCandidateVm {
    pub candidate_id: String,
    pub lot_id: String,
    pub name: String,
    pub candidate_type: String,
    pub match_archetype: String,
    pub head_noun: String,
    pub quoteability_score_bps: Option<i64>,
    pub candidate_value_share_bps: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SupplierQuoteVm {
    pub quote_id: String,
    pub candidate_id: Option<String>,
    pub product_name: String,
    pub source_name: String,
    pub source_url: Option<String>,
    pub unit_price_gbp_pence: i64,
    pub minimum_order_quantity: Option<String>,
    pub lead_time_days: Option<i64>,
    pub price_signal_role: String,
    pub match_confidence_bps: Option<i64>,
    pub quote_usability_confidence_bps: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LandedCostVm {
    pub estimate_id: String,
    pub candidate_id: String,
    pub supplier_quote_id: String,
    pub net_margin_low_bps: i64,
    pub net_margin_base_bps: i64,
    pub net_margin_high_bps: i64,
    pub decision_grade: String,
    pub confidence_bps: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestVm {
    pub markdown: String,
    pub basis_hash: String,
    pub generated_at: String,
    pub sections: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessDetailVm {
    pub process_id: String,
    pub canonical_process_id: String,
    pub notices: Vec<ContractNoticeVm>,
    pub lots: Vec<NoticeLotVm>,
    pub candidates: Vec<ProductCandidateVm>,
    pub quotes: Vec<SupplierQuoteVm>,
    pub landed_costs: Vec<LandedCostVm>,
    pub manifest: Option<ManifestVm>,
    pub coverage: CoverageVm,
    pub decision: ProcessDecision,
    pub errors: Vec<ErrorListItem>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionListFilters {
    pub phases: Vec<String>,
    pub execution_modes: Vec<String>,
    pub active_only: bool,
    pub search: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionListItem {
    pub session_id: String,
    pub daemon_instance_id: String,
    pub skill_path: String,
    pub skill_version: Option<String>,
    pub execution_mode: String,
    pub phase: String,
    pub started_at: Option<String>,
    pub ended_at: Option<String>,
    pub duration_ms: Option<i64>,
    pub cleanup_verdict: Option<String>,
    pub error_type: Option<String>,
    pub exit_code: Option<i64>,
    pub checkpoint_count: i64,
    pub byte_count: i64,
    pub reattachable: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorListFilters {
    pub severities: Vec<String>,
    pub resolved: Option<bool>,
    pub pipeline_stages: Vec<String>,
    pub search: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorListItem {
    pub error_manifest_id: String,
    pub process_id: Option<String>,
    pub pipeline_stage: Option<String>,
    pub error_type: String,
    pub error_message: String,
    pub cleanup_verdict: Option<String>,
    pub severity: String,
    pub skill_path: Option<String>,
    pub created_at: String,
    pub resolved: bool,
    pub agent_session_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionHistoryRequest {
    pub session_id: String,
    pub from_seq: u64,
    pub to_seq: u64,
    pub max_bytes: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionHistoryAvailability {
    Complete,
    Partial,
    Missing,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum SessionFrame {
    Lifecycle {
        seq: u64,
        state: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        cleanup_verdict: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
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
        event: Value,
        ts: String,
    },
    Gap {
        seq: u64,
        from_seq: u64,
        to_seq: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionHistoryChunk {
    pub session_id: String,
    pub first_seq: u64,
    pub last_seq: u64,
    pub frames: Vec<SessionFrame>,
    pub availability: SessionHistoryAvailability,
}

#[derive(Debug, Deserialize)]
struct StreamIndexEntry {
    chunk: String,
    first_seq: u64,
    last_seq: u64,
    wall_ts_start: String,
}

impl QueryService {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, QueryServiceError> {
        let path = path.as_ref();
        let conn = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .map_err(|error| {
            QueryServiceError::storage_unreadable(format!(
                "opening read-only SQLite database {} failed: {error}",
                path.display()
            ))
        })?;

        conn.execute_batch(
            "\
            PRAGMA query_only = ON;
            PRAGMA busy_timeout = 1000;
            ",
        )
        .map_err(|error| QueryServiceError::busy(error.to_string()))?;

        let quick_check: String = conn
            .query_row("PRAGMA quick_check", [], |row| row.get(0))
            .map_err(|error| QueryServiceError::storage_unreadable(error.to_string()))?;
        if quick_check != "ok" {
            return Err(QueryServiceError::storage_unreadable(
                "PRAGMA quick_check did not return ok",
            ));
        }

        let schema_epoch: i64 = conn
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .map_err(|error| QueryServiceError::busy(error.to_string()))?;
        if schema_epoch != DEFAULT_SCHEMA_EPOCH {
            return Err(QueryServiceError::schema_mismatch(format!(
                "expected schema epoch {DEFAULT_SCHEMA_EPOCH}, found {schema_epoch}"
            )));
        }

        let capabilities = QueryCapabilities {
            process_aliases: table_exists(&conn, "process_aliases")?,
            pending_operator_intents: table_exists(&conn, "pending_operator_intents")?,
        };

        Ok(Self { conn, capabilities })
    }

    #[cfg(test)]
    pub const fn connection(&self) -> &Connection {
        &self.conn
    }

    pub fn list_processes(
        &mut self,
        cursor: Option<&str>,
        limit: usize,
        filters: &ProcessListFilters,
    ) -> Result<QueryEnvelope<KeysetPage<ProcessListItem>>, QueryServiceError> {
        validate_process_filters(filters)?;
        let cursor = cursor
            .map(decode_process_cursor)
            .transpose()
            .map_err(QueryServiceError::invalid_cursor)?;

        self.with_snapshot(|tx, capabilities| {
            let mut stmt = tx
                .prepare(
                    "\
                SELECT p.id,
                       COALESCE(NULLIF(p.canonical_process_id, ''), p.id),
                       p.title,
                       COALESCE(p.buyer_name, ''),
                       p.latest_published_at,
                       p.latest_deadline_at,
                       COALESCE(pa.pipeline_state, p.current_stage),
                       pa.priced_coverage_bps,
                       pa.net_qualified_coverage_bps,
                       pa.net_qualified_value_coverage_bps,
                       pa.gross_margin_bps,
                       pa.net_margin_bps,
                       pa.decision_basis_hash,
                       (SELECT COUNT(*) FROM product_candidates pc WHERE pc.process_id = p.id),
                       (
                         SELECT COUNT(DISTINCT sq.product_candidate_id)
                         FROM supplier_quotes sq
                         WHERE sq.process_id = p.id AND sq.product_candidate_id IS NOT NULL
                       ),
                       (
                         SELECT source_id
                         FROM contract_notices cn
                         WHERE cn.process_id = p.id
                         ORDER BY cn.published_at DESC, cn.id DESC
                         LIMIT 1
                       )
                FROM procurement_processes p
                LEFT JOIN pipeline_assessments pa ON pa.process_id = p.id
                ORDER BY p.latest_published_at DESC, p.id DESC
                ",
                )
                .map_err(|error| QueryServiceError::busy(error.to_string()))?;

            let rows = stmt
                .query_map([], |row| {
                    Ok(ProcessRow {
                        process_id: row.get(0)?,
                        canonical_process_id: row.get(1)?,
                        title: row.get(2)?,
                        buyer_name: row.get(3)?,
                        published_at: row.get(4)?,
                        deadline_at: row.get(5)?,
                        pipeline_state: row.get(6)?,
                        priced_coverage_bps: row.get(7)?,
                        net_qualified_coverage_bps: row.get(8)?,
                        net_qualified_value_coverage_bps: row.get(9)?,
                        gross_margin_bps: row.get(10)?,
                        net_margin_base_bps: row.get(11)?,
                        decision_basis_hash: row.get(12)?,
                        candidate_count: row.get(13)?,
                        quoted_candidate_count: row.get(14)?,
                        primary_source_id: row.get(15)?,
                    })
                })
                .map_err(|error| QueryServiceError::busy(error.to_string()))?;

            let mut items = Vec::new();
            for row in rows {
                let row = row.map_err(|error| QueryServiceError::busy(error.to_string()))?;
                let decision = latest_process_decision(
                    tx,
                    capabilities.pending_operator_intents,
                    &row.process_id,
                    row.decision_basis_hash.as_deref(),
                )?;
                if !matches_process_filters(&row, &decision, filters) {
                    continue;
                }
                if let Some(cursor) = cursor.as_ref()
                    && !is_after_process_cursor(&row, cursor)
                {
                    continue;
                }
                items.push(ProcessListItem {
                    process_id: row.process_id,
                    canonical_process_id: row.canonical_process_id,
                    title: row.title,
                    buyer_name: row.buyer_name,
                    published_at: row.published_at,
                    deadline_at: row.deadline_at,
                    pipeline_state: row.pipeline_state,
                    priced_coverage_bps: row.priced_coverage_bps,
                    net_qualified_coverage_bps: row.net_qualified_coverage_bps,
                    net_qualified_value_coverage_bps: row.net_qualified_value_coverage_bps,
                    gross_margin_bps: row.gross_margin_bps,
                    net_margin_base_bps: row.net_margin_base_bps,
                    candidate_count: row.candidate_count,
                    quoted_candidate_count: row.quoted_candidate_count,
                    decision,
                });
            }

            Ok(page_process_items(items, limit))
        })
    }

    #[allow(clippy::too_many_lines)]
    pub fn get_process_detail(
        &mut self,
        process_id: &str,
    ) -> Result<QueryEnvelope<ProcessDetailVm>, QueryServiceError> {
        self.with_snapshot(|tx, capabilities| {
            let canonical_process_id =
                resolve_canonical_process_id(tx, capabilities.process_aliases, process_id)?
                    .ok_or_else(|| {
                        QueryServiceError::not_found(format!("unknown process `{process_id}`"))
                    })?;

            let process = tx
                .query_row(
                    "\
                SELECT id,
                       COALESCE(NULLIF(canonical_process_id, ''), id),
                       title,
                       COALESCE(buyer_name, ''),
                       latest_published_at,
                       latest_deadline_at
                FROM procurement_processes
                WHERE id = ?1
                ",
                    params![canonical_process_id],
                    |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, String>(2)?,
                            row.get::<_, String>(3)?,
                            row.get::<_, String>(4)?,
                            row.get::<_, Option<String>>(5)?,
                        ))
                    },
                )
                .optional()
                .map_err(|error| QueryServiceError::busy(error.to_string()))?
                .ok_or_else(|| {
                    QueryServiceError::not_found(format!(
                        "canonical process `{canonical_process_id}` is missing"
                    ))
                })?;

            let notices = load_notices(tx, &process.0)?;
            let lots = load_lots(tx, &process.0)?;
            let candidates = load_candidates(tx, &process.0)?;
            let quotes = load_quotes(tx, &process.0)?;
            let landed_costs = load_landed_costs(tx, &process.0)?;
            let coverage = load_coverage(tx, &process.0)?;
            let decision = latest_process_decision(
                tx,
                capabilities.pending_operator_intents,
                &process.0,
                coverage.basis_hash.as_deref(),
            )?;
            let manifest = load_manifest(tx, &process.0, coverage.basis_hash.as_deref())?;
            let errors = load_errors_for_process(tx, &process.0)?;

            Ok(ProcessDetailVm {
                process_id: process.0,
                canonical_process_id: process.1,
                notices,
                lots,
                candidates,
                quotes,
                landed_costs,
                manifest,
                coverage: coverage.metrics,
                decision,
                errors,
            })
        })
    }

    pub fn list_sessions(
        &mut self,
        cursor: Option<&str>,
        limit: usize,
        filters: &SessionListFilters,
    ) -> Result<QueryEnvelope<KeysetPage<SessionListItem>>, QueryServiceError> {
        validate_session_filters(filters)?;
        let cursor = cursor
            .map(decode_session_cursor)
            .transpose()
            .map_err(QueryServiceError::invalid_cursor)?;

        self.with_snapshot(|tx, _| {
            let mut stmt = tx
                .prepare(
                    "\
                SELECT id,
                       skill_path,
                       skill_version,
                       execution_mode,
                       status,
                       started_at,
                       completed_at,
                       created_at,
                       terminal_output_bytes,
                       structured_trace,
                       error_type,
                       exit_code,
                       cleanup_verdict,
                       last_checkpoint_seq
                FROM agent_session_logs
                ORDER BY COALESCE(started_at, created_at) DESC, id DESC
                ",
                )
                .map_err(|error| QueryServiceError::busy(error.to_string()))?;

            let rows = stmt
                .query_map([], |row| {
                    Ok(SessionRow {
                        session_id: row.get(0)?,
                        skill_path: row.get(1)?,
                        skill_version: row.get(2)?,
                        execution_mode: row.get(3)?,
                        status: row.get(4)?,
                        started_at: row.get(5)?,
                        completed_at: row.get(6)?,
                        created_at: row.get(7)?,
                        terminal_output_bytes: row.get(8)?,
                        structured_trace: row.get(9)?,
                        error_type: row.get(10)?,
                        exit_code: row.get(11)?,
                        cleanup_verdict: row.get(12)?,
                        last_checkpoint_seq: row.get(13)?,
                    })
                })
                .map_err(|error| QueryServiceError::busy(error.to_string()))?;

            let mut items = Vec::new();
            for row in rows {
                let row = row.map_err(|error| QueryServiceError::busy(error.to_string()))?;
                let phase = session_phase(&row.status, row.structured_trace.as_deref());
                if !matches_session_filters(&row, &phase, filters) {
                    continue;
                }
                if let Some(cursor) = cursor.as_ref()
                    && !is_after_session_cursor(&row, cursor)
                {
                    continue;
                }
                items.push(SessionListItem {
                    session_id: row.session_id,
                    daemon_instance_id: extract_daemon_instance_id(row.structured_trace.as_deref())
                        .unwrap_or_else(|| "unknown".to_owned()),
                    skill_path: row.skill_path,
                    skill_version: row.skill_version,
                    execution_mode: row.execution_mode.clone(),
                    phase,
                    started_at: row.started_at.clone(),
                    ended_at: row.completed_at.clone(),
                    duration_ms: duration_ms(
                        row.started_at.as_deref(),
                        row.completed_at.as_deref(),
                    ),
                    cleanup_verdict: row.cleanup_verdict.map(|value| value.to_lowercase()),
                    error_type: row.error_type,
                    exit_code: row.exit_code,
                    checkpoint_count: row.last_checkpoint_seq.map_or(0, i64::from),
                    byte_count: row.terminal_output_bytes.unwrap_or(0),
                    reattachable: row.execution_mode == "pty"
                        && matches!(
                            row.status.as_str(),
                            "created" | "spawned" | "active" | "terminating"
                        ),
                });
            }

            Ok(page_session_items(items, limit))
        })
    }

    pub fn list_errors(
        &mut self,
        cursor: Option<&str>,
        limit: usize,
        filters: &ErrorListFilters,
    ) -> Result<QueryEnvelope<KeysetPage<ErrorListItem>>, QueryServiceError> {
        validate_error_filters(filters)?;
        let cursor = cursor
            .map(decode_error_cursor)
            .transpose()
            .map_err(QueryServiceError::invalid_cursor)?;

        self.with_snapshot(|tx, _| {
            let mut stmt = tx
                .prepare(
                    "\
                SELECT e.id,
                       e.process_id,
                       e.pipeline_stage,
                       e.error_type,
                       e.error_message,
                       s.cleanup_verdict,
                       e.skill_path,
                       e.created_at,
                       e.resolved,
                       e.agent_session_id
                FROM error_manifests e
                LEFT JOIN agent_session_logs s ON s.id = e.agent_session_id
                ORDER BY e.created_at DESC, e.id DESC
                ",
                )
                .map_err(|error| QueryServiceError::busy(error.to_string()))?;

            let rows = stmt
                .query_map([], |row| {
                    Ok(ErrorListItem {
                        error_manifest_id: row.get(0)?,
                        process_id: row.get(1)?,
                        pipeline_stage: row.get(2)?,
                        error_type: row.get(3)?,
                        error_message: row.get(4)?,
                        cleanup_verdict: row
                            .get::<_, Option<String>>(5)?
                            .map(|value| value.to_lowercase()),
                        severity: String::new(),
                        skill_path: row.get(6)?,
                        created_at: row.get(7)?,
                        resolved: row.get::<_, i64>(8)? != 0,
                        agent_session_id: row.get(9)?,
                    })
                })
                .map_err(|error| QueryServiceError::busy(error.to_string()))?;

            let mut items = Vec::new();
            for row in rows {
                let mut item = row.map_err(|error| QueryServiceError::busy(error.to_string()))?;
                item.severity =
                    derive_error_severity(&item.error_type, item.cleanup_verdict.as_deref());
                if !matches_error_filters(&item, filters) {
                    continue;
                }
                if let Some(cursor) = cursor.as_ref()
                    && !is_after_error_cursor(&item, cursor)
                {
                    continue;
                }
                items.push(item);
            }

            Ok(page_error_items(items, limit))
        })
    }

    pub fn get_session_history(
        &mut self,
        request: &SessionHistoryRequest,
    ) -> Result<QueryEnvelope<SessionHistoryChunk>, QueryServiceError> {
        if request.from_seq == 0 || request.from_seq > request.to_seq {
            return Err(QueryServiceError::invalid_filter(
                "session replay range must be a non-empty inclusive range",
            ));
        }

        self.with_snapshot(|tx, _| {
            let row = tx
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
                .map_err(|error| QueryServiceError::busy(error.to_string()))?
                .ok_or_else(|| {
                    QueryServiceError::not_found(format!(
                        "unknown session `{}`",
                        request.session_id
                    ))
                })?;

            let terminal_output_path = row.1.ok_or_else(|| {
                QueryServiceError::artifact_unavailable(format!(
                    "session `{}` has no historical artifact path",
                    request.session_id
                ))
            })?;

            if row.0 == "pty" {
                load_pty_history(&request.session_id, &terminal_output_path, request)
            } else {
                load_contract_history(&request.session_id, &terminal_output_path, request)
            }
        })
    }

    fn with_snapshot<T, F>(&mut self, f: F) -> Result<QueryEnvelope<T>, QueryServiceError>
    where
        F: FnOnce(&Transaction<'_>, QueryCapabilities) -> Result<T, QueryServiceError>,
    {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Deferred)
            .map_err(|error| QueryServiceError::busy(error.to_string()))?;
        let data = f(&tx, self.capabilities)?;
        let snapshot_seq = tx
            .query_row(
                "SELECT commit_seq FROM commit_clock WHERE id = 1",
                [],
                |row| row.get(0),
            )
            .map_err(|error| QueryServiceError::busy(error.to_string()))?;
        let snapshot_at = tx
            .query_row("SELECT STRFTIME('%Y-%m-%dT%H:%M:%fZ', 'now')", [], |row| {
                row.get(0)
            })
            .map_err(|error| QueryServiceError::busy(error.to_string()))?;
        drop(tx);

        Ok(QueryEnvelope {
            data,
            snapshot_seq,
            snapshot_at,
        })
    }
}

#[derive(Debug)]
struct ProcessRow {
    process_id: String,
    canonical_process_id: String,
    title: String,
    buyer_name: String,
    published_at: String,
    deadline_at: Option<String>,
    pipeline_state: String,
    priced_coverage_bps: Option<i64>,
    net_qualified_coverage_bps: Option<i64>,
    net_qualified_value_coverage_bps: Option<i64>,
    gross_margin_bps: Option<i64>,
    net_margin_base_bps: Option<i64>,
    decision_basis_hash: Option<String>,
    candidate_count: i64,
    quoted_candidate_count: i64,
    primary_source_id: Option<String>,
}

#[derive(Debug)]
struct SessionRow {
    session_id: String,
    skill_path: String,
    skill_version: Option<String>,
    execution_mode: String,
    status: String,
    started_at: Option<String>,
    completed_at: Option<String>,
    created_at: String,
    terminal_output_bytes: Option<i64>,
    structured_trace: Option<String>,
    error_type: Option<String>,
    exit_code: Option<i64>,
    cleanup_verdict: Option<String>,
    last_checkpoint_seq: Option<u32>,
}

#[derive(Debug)]
struct CoverageLoad {
    metrics: CoverageVm,
    basis_hash: Option<String>,
}

fn table_exists(conn: &Connection, table: &str) -> Result<bool, QueryServiceError> {
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
            params![table],
            |row| row.get(0),
        )
        .map_err(|error| QueryServiceError::busy(error.to_string()))?;
    Ok(count == 1)
}

fn validate_process_filters(filters: &ProcessListFilters) -> Result<(), QueryServiceError> {
    if filters
        .source_ids
        .iter()
        .any(|value| value.trim().is_empty())
    {
        return Err(QueryServiceError::invalid_filter(
            "sourceIds must not contain blank values",
        ));
    }
    Ok(())
}

fn validate_session_filters(filters: &SessionListFilters) -> Result<(), QueryServiceError> {
    if filters
        .execution_modes
        .iter()
        .any(|value| value.trim().is_empty())
    {
        return Err(QueryServiceError::invalid_filter(
            "executionModes must not contain blank values",
        ));
    }
    Ok(())
}

fn validate_error_filters(filters: &ErrorListFilters) -> Result<(), QueryServiceError> {
    if filters
        .severities
        .iter()
        .any(|value| value.trim().is_empty())
    {
        return Err(QueryServiceError::invalid_filter(
            "severities must not contain blank values",
        ));
    }
    Ok(())
}

fn resolve_canonical_process_id(
    tx: &Transaction<'_>,
    aliases_enabled: bool,
    process_id: &str,
) -> Result<Option<String>, QueryServiceError> {
    if aliases_enabled
        && let Some(canonical_process_id) = tx
            .query_row(
                "SELECT canonical_process_id FROM process_aliases WHERE alias_process_id = ?1",
                params![process_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| QueryServiceError::busy(error.to_string()))?
    {
        return Ok(Some(canonical_process_id));
    }

    tx.query_row(
        "SELECT COALESCE(NULLIF(canonical_process_id, ''), id) FROM procurement_processes WHERE id = ?1",
        params![process_id],
        |row| row.get(0),
    )
    .optional()
    .map_err(|error| QueryServiceError::busy(error.to_string()))
}

fn latest_process_decision(
    tx: &Transaction<'_>,
    pending_enabled: bool,
    process_id: &str,
    current_basis_hash: Option<&str>,
) -> Result<ProcessDecision, QueryServiceError> {
    if !pending_enabled {
        return Ok(ProcessDecision {
            disposition: None,
            is_stale: false,
            decided_at: None,
        });
    }

    let row: Option<(String, String, String)> = tx
        .query_row(
            "\
            SELECT desired_disposition,
                   reviewed_basis_hash,
                   COALESCE(reviewed_at, queued_at)
            FROM pending_operator_intents
            WHERE process_id = ?1
              AND superseded_by_intent_id IS NULL
            ORDER BY queued_at DESC, id DESC
            LIMIT 1
            ",
            params![process_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()
        .map_err(|error| QueryServiceError::busy(error.to_string()))?;

    let Some((disposition, reviewed_basis_hash, decided_at)) = row else {
        return Ok(ProcessDecision {
            disposition: None,
            is_stale: false,
            decided_at: None,
        });
    };

    Ok(ProcessDecision {
        disposition: Some(disposition),
        is_stale: current_basis_hash.is_some_and(|current| current != reviewed_basis_hash),
        decided_at: Some(decided_at),
    })
}

fn matches_process_filters(
    row: &ProcessRow,
    decision: &ProcessDecision,
    filters: &ProcessListFilters,
) -> bool {
    if !filters.source_ids.is_empty()
        && row
            .primary_source_id
            .as_deref()
            .is_none_or(|source_id| !filters.source_ids.iter().any(|value| value == source_id))
    {
        return false;
    }
    if !filters.pipeline_states.is_empty()
        && !filters
            .pipeline_states
            .iter()
            .any(|value| value == &row.pipeline_state)
    {
        return false;
    }
    if !filters.decision_dispositions.is_empty()
        && decision.disposition.as_deref().is_none_or(|value| {
            !filters
                .decision_dispositions
                .iter()
                .any(|item| item == value)
        })
    {
        return false;
    }
    if filters.stale_only && !decision.is_stale {
        return false;
    }
    if let Some(search) = filters.search.as_deref() {
        let search = search.to_lowercase();
        let title = row.title.to_lowercase();
        let buyer_name = row.buyer_name.to_lowercase();
        if !title.contains(&search) && !buyer_name.contains(&search) {
            return false;
        }
    }
    true
}

fn page_process_items(items: Vec<ProcessListItem>, limit: usize) -> KeysetPage<ProcessListItem> {
    let has_more = items.len() > limit;
    let mut page_items = items;
    page_items.truncate(limit);
    let next_cursor = if has_more {
        page_items
            .last()
            .map(|item| encode_process_cursor(&item.published_at, &item.process_id))
    } else {
        None
    };
    KeysetPage {
        items: page_items,
        next_cursor,
        has_more,
    }
}

fn page_session_items(items: Vec<SessionListItem>, limit: usize) -> KeysetPage<SessionListItem> {
    let has_more = items.len() > limit;
    let mut page_items = items;
    page_items.truncate(limit);
    let next_cursor = if has_more {
        page_items.last().map(|item| {
            encode_session_cursor(item.started_at.as_deref().unwrap_or(""), &item.session_id)
        })
    } else {
        None
    };
    KeysetPage {
        items: page_items,
        next_cursor,
        has_more,
    }
}

fn page_error_items(items: Vec<ErrorListItem>, limit: usize) -> KeysetPage<ErrorListItem> {
    let has_more = items.len() > limit;
    let mut page_items = items;
    page_items.truncate(limit);
    let next_cursor = if has_more {
        page_items
            .last()
            .map(|item| encode_error_cursor(&item.created_at, &item.error_manifest_id))
    } else {
        None
    };
    KeysetPage {
        items: page_items,
        next_cursor,
        has_more,
    }
}

fn encode_process_cursor(published_at: &str, process_id: &str) -> String {
    format!("{published_at}\t{process_id}")
}

fn decode_process_cursor(cursor: &str) -> Result<(String, String), String> {
    let Some((published_at, process_id)) = cursor.split_once('\t') else {
        return Err("process cursor must contain publishedAt and processId".to_owned());
    };
    Ok((published_at.to_owned(), process_id.to_owned()))
}

fn is_after_process_cursor(row: &ProcessRow, cursor: &(String, String)) -> bool {
    row.published_at < cursor.0 || (row.published_at == cursor.0 && row.process_id < cursor.1)
}

fn encode_session_cursor(started_at: &str, session_id: &str) -> String {
    format!("{started_at}\t{session_id}")
}

fn decode_session_cursor(cursor: &str) -> Result<(String, String), String> {
    let Some((started_at, session_id)) = cursor.split_once('\t') else {
        return Err("session cursor must contain startedAt and sessionId".to_owned());
    };
    Ok((started_at.to_owned(), session_id.to_owned()))
}

fn is_after_session_cursor(row: &SessionRow, cursor: &(String, String)) -> bool {
    let started_at = row.started_at.as_deref().unwrap_or(row.created_at.as_str());
    started_at < cursor.0.as_str() || (started_at == cursor.0.as_str() && row.session_id < cursor.1)
}

fn encode_error_cursor(created_at: &str, error_manifest_id: &str) -> String {
    format!("{created_at}\t{error_manifest_id}")
}

fn decode_error_cursor(cursor: &str) -> Result<(String, String), String> {
    let Some((created_at, error_manifest_id)) = cursor.split_once('\t') else {
        return Err("error cursor must contain createdAt and errorManifestId".to_owned());
    };
    Ok((created_at.to_owned(), error_manifest_id.to_owned()))
}

fn is_after_error_cursor(item: &ErrorListItem, cursor: &(String, String)) -> bool {
    item.created_at < cursor.0 || (item.created_at == cursor.0 && item.error_manifest_id < cursor.1)
}

fn load_notices(
    tx: &Transaction<'_>,
    process_id: &str,
) -> Result<Vec<ContractNoticeVm>, QueryServiceError> {
    let mut stmt = tx
        .prepare(
            "\
            SELECT id, source_id, source_notice_id, title, buyer_name, procurement_stage,
                   access_route, deadline_at, published_at, estimated_value_gbp_ex_vat_pence
            FROM contract_notices
            WHERE process_id = ?1
            ORDER BY published_at DESC, id DESC
            ",
        )
        .map_err(|error| QueryServiceError::busy(error.to_string()))?;
    let rows = stmt
        .query_map(params![process_id], |row| {
            Ok(ContractNoticeVm {
                notice_id: row.get(0)?,
                source_id: row.get(1)?,
                source_notice_id: row.get(2)?,
                title: row.get(3)?,
                buyer_name: row.get(4)?,
                procurement_stage: row.get(5)?,
                access_route: row.get(6)?,
                deadline_at: row.get(7)?,
                published_at: row.get(8)?,
                estimated_value_gbp_ex_vat_pence: row.get(9)?,
            })
        })
        .map_err(|error| QueryServiceError::busy(error.to_string()))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| QueryServiceError::busy(error.to_string()))
}

fn load_lots(
    tx: &Transaction<'_>,
    process_id: &str,
) -> Result<Vec<NoticeLotVm>, QueryServiceError> {
    let mut stmt = tx
        .prepare(
            "\
            SELECT id, notice_id, lot_number, title, description, estimated_value_pence,
                   deadline_at, is_bidable_separately
            FROM notice_lots
            WHERE notice_id IN (SELECT id FROM contract_notices WHERE process_id = ?1)
            ORDER BY COALESCE(lot_number, 9_223_372_036_854_775_807), id
            ",
        )
        .map_err(|error| QueryServiceError::busy(error.to_string()))?;
    let rows = stmt
        .query_map(params![process_id], |row| {
            Ok(NoticeLotVm {
                lot_id: row.get(0)?,
                notice_id: row.get(1)?,
                lot_number: row.get(2)?,
                title: row.get(3)?,
                description: row.get(4)?,
                estimated_value_pence: row.get(5)?,
                deadline_at: row.get(6)?,
                is_bidable_separately: row.get::<_, i64>(7)? != 0,
            })
        })
        .map_err(|error| QueryServiceError::busy(error.to_string()))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| QueryServiceError::busy(error.to_string()))
}

fn load_candidates(
    tx: &Transaction<'_>,
    process_id: &str,
) -> Result<Vec<ProductCandidateVm>, QueryServiceError> {
    let mut stmt = tx
        .prepare(
            "\
            SELECT id, lot_id, name, candidate_type, match_archetype, head_noun,
                   quoteability_score_bps, candidate_value_share_bps
            FROM product_candidates
            WHERE process_id = ?1
            ORDER BY COALESCE(candidate_value_share_bps, 0) DESC, id
            ",
        )
        .map_err(|error| QueryServiceError::busy(error.to_string()))?;
    let rows = stmt
        .query_map(params![process_id], |row| {
            Ok(ProductCandidateVm {
                candidate_id: row.get(0)?,
                lot_id: row.get(1)?,
                name: row.get(2)?,
                candidate_type: row.get(3)?,
                match_archetype: row.get(4)?,
                head_noun: row.get(5)?,
                quoteability_score_bps: row.get(6)?,
                candidate_value_share_bps: row.get(7)?,
            })
        })
        .map_err(|error| QueryServiceError::busy(error.to_string()))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| QueryServiceError::busy(error.to_string()))
}

fn load_quotes(
    tx: &Transaction<'_>,
    process_id: &str,
) -> Result<Vec<SupplierQuoteVm>, QueryServiceError> {
    let mut stmt = tx
        .prepare(
            "\
            SELECT id, product_candidate_id, product_name, source_name, source_url,
                   unit_price_gbp_pence, minimum_order_quantity, lead_time_days,
                   price_signal_role, match_confidence_bps, quote_usability_confidence_bps
            FROM supplier_quotes
            WHERE process_id = ?1
            ORDER BY COALESCE(match_confidence_bps, 0) DESC, id
            ",
        )
        .map_err(|error| QueryServiceError::busy(error.to_string()))?;
    let rows = stmt
        .query_map(params![process_id], |row| {
            Ok(SupplierQuoteVm {
                quote_id: row.get(0)?,
                candidate_id: row.get(1)?,
                product_name: row.get(2)?,
                source_name: row.get(3)?,
                source_url: row.get(4)?,
                unit_price_gbp_pence: row.get(5)?,
                minimum_order_quantity: row.get::<_, Option<String>>(6)?,
                lead_time_days: row.get(7)?,
                price_signal_role: row.get(8)?,
                match_confidence_bps: row.get(9)?,
                quote_usability_confidence_bps: row.get(10)?,
            })
        })
        .map_err(|error| QueryServiceError::busy(error.to_string()))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| QueryServiceError::busy(error.to_string()))
}

fn load_landed_costs(
    tx: &Transaction<'_>,
    process_id: &str,
) -> Result<Vec<LandedCostVm>, QueryServiceError> {
    let mut stmt = tx
        .prepare(
            "\
            SELECT id, product_candidate_id, supplier_quote_id, net_margin_low_bps,
                   net_margin_base_bps, net_margin_high_bps, decision_grade, confidence_bps
            FROM landed_cost_estimates
            WHERE process_id = ?1
            ORDER BY confidence_bps DESC, id
            ",
        )
        .map_err(|error| QueryServiceError::busy(error.to_string()))?;
    let rows = stmt
        .query_map(params![process_id], |row| {
            Ok(LandedCostVm {
                estimate_id: row.get(0)?,
                candidate_id: row.get(1)?,
                supplier_quote_id: row.get(2)?,
                net_margin_low_bps: row.get(3)?,
                net_margin_base_bps: row.get(4)?,
                net_margin_high_bps: row.get(5)?,
                decision_grade: row.get(6)?,
                confidence_bps: row.get(7)?,
            })
        })
        .map_err(|error| QueryServiceError::busy(error.to_string()))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| QueryServiceError::busy(error.to_string()))
}

fn load_manifest(
    tx: &Transaction<'_>,
    process_id: &str,
    basis_hash: Option<&str>,
) -> Result<Option<ManifestVm>, QueryServiceError> {
    tx.query_row(
        "\
        SELECT markdown, created_at
        FROM manifests
        WHERE process_id = ?1
        ORDER BY created_at DESC, id DESC
        LIMIT 1
        ",
        params![process_id],
        |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
    )
    .optional()
    .map_err(|error| QueryServiceError::busy(error.to_string()))?
    .map(|(markdown, generated_at)| ManifestVm {
        sections: manifest_sections(&markdown),
        markdown,
        basis_hash: basis_hash.unwrap_or_default().to_owned(),
        generated_at,
    })
    .pipe(Ok)
}

fn load_coverage(
    tx: &Transaction<'_>,
    process_id: &str,
) -> Result<CoverageLoad, QueryServiceError> {
    let row: Option<CoverageRow> = tx
        .query_row(
            "\
            SELECT priced_coverage_bps,
                   landed_cost_coverage_bps,
                   net_qualified_coverage_bps,
                   priced_value_coverage_bps,
                   landed_cost_value_coverage_bps,
                   net_qualified_value_coverage_bps,
                   unresolved_mandatory_lots,
                   decision_basis_hash
            FROM pipeline_assessments
            WHERE process_id = ?1
            ",
            params![process_id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                    row.get(7)?,
                ))
            },
        )
        .optional()
        .map_err(|error| QueryServiceError::busy(error.to_string()))?;

    let Some(row) = row else {
        return Ok(CoverageLoad {
            metrics: CoverageVm {
                priced_coverage_bps: 0,
                landed_cost_coverage_bps: 0,
                net_qualified_coverage_bps: 0,
                priced_value_coverage_bps: 0,
                landed_cost_value_coverage_bps: 0,
                net_qualified_value_coverage_bps: 0,
                unresolved_mandatory_lots: 0,
            },
            basis_hash: None,
        });
    };

    Ok(CoverageLoad {
        metrics: CoverageVm {
            priced_coverage_bps: row.0.unwrap_or(0),
            landed_cost_coverage_bps: row.1.unwrap_or(0),
            net_qualified_coverage_bps: row.2.unwrap_or(0),
            priced_value_coverage_bps: row.3.unwrap_or(0),
            landed_cost_value_coverage_bps: row.4.unwrap_or(0),
            net_qualified_value_coverage_bps: row.5.unwrap_or(0),
            unresolved_mandatory_lots: row.6,
        },
        basis_hash: row.7,
    })
}

fn load_errors_for_process(
    tx: &Transaction<'_>,
    process_id: &str,
) -> Result<Vec<ErrorListItem>, QueryServiceError> {
    let mut stmt = tx
        .prepare(
            "\
            SELECT e.id,
                   e.process_id,
                   e.pipeline_stage,
                   e.error_type,
                   e.error_message,
                   s.cleanup_verdict,
                   e.skill_path,
                   e.created_at,
                   e.resolved,
                   e.agent_session_id
            FROM error_manifests e
            LEFT JOIN agent_session_logs s ON s.id = e.agent_session_id
            WHERE e.process_id = ?1 AND e.resolved = 0
            ORDER BY e.created_at DESC, e.id DESC
            ",
        )
        .map_err(|error| QueryServiceError::busy(error.to_string()))?;
    let rows = stmt
        .query_map(params![process_id], |row| {
            let error_type: String = row.get(3)?;
            let cleanup_verdict = row
                .get::<_, Option<String>>(5)?
                .map(|value| value.to_lowercase());
            Ok(ErrorListItem {
                error_manifest_id: row.get(0)?,
                process_id: row.get(1)?,
                pipeline_stage: row.get(2)?,
                error_message: row.get(4)?,
                severity: derive_error_severity(&error_type, cleanup_verdict.as_deref()),
                skill_path: row.get(6)?,
                created_at: row.get(7)?,
                resolved: row.get::<_, i64>(8)? != 0,
                agent_session_id: row.get(9)?,
                error_type,
                cleanup_verdict,
            })
        })
        .map_err(|error| QueryServiceError::busy(error.to_string()))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| QueryServiceError::busy(error.to_string()))
}

fn matches_session_filters(row: &SessionRow, phase: &str, filters: &SessionListFilters) -> bool {
    if filters.active_only && !matches!(phase, "created" | "spawned" | "active" | "terminating") {
        return false;
    }
    if !filters.phases.is_empty() && !filters.phases.iter().any(|value| value == phase) {
        return false;
    }
    if !filters.execution_modes.is_empty()
        && !filters
            .execution_modes
            .iter()
            .any(|value| value == &row.execution_mode)
    {
        return false;
    }
    if let Some(search) = filters.search.as_deref() {
        let search = search.to_lowercase();
        if !row.skill_path.to_lowercase().contains(&search)
            && !row.session_id.to_lowercase().contains(&search)
        {
            return false;
        }
    }
    true
}

fn matches_error_filters(item: &ErrorListItem, filters: &ErrorListFilters) -> bool {
    let resolved = filters.resolved.unwrap_or(false);
    if item.resolved != resolved {
        return false;
    }
    if !filters.severities.is_empty()
        && !filters
            .severities
            .iter()
            .any(|value| value == &item.severity)
    {
        return false;
    }
    if !filters.pipeline_stages.is_empty()
        && item
            .pipeline_stage
            .as_deref()
            .is_none_or(|value| !filters.pipeline_stages.iter().any(|stage| stage == value))
    {
        return false;
    }
    if let Some(search) = filters.search.as_deref() {
        let search = search.to_lowercase();
        if !item.error_type.to_lowercase().contains(&search)
            && !item.error_message.to_lowercase().contains(&search)
        {
            return false;
        }
    }
    true
}

fn session_phase(status: &str, structured_trace: Option<&str>) -> String {
    parse_phase_history(structured_trace)
        .and_then(|mut phases| phases.pop())
        .unwrap_or_else(|| match status {
            "created" => "created".to_owned(),
            "spawned" => "spawned".to_owned(),
            "active" => "active".to_owned(),
            "terminating" => "terminating".to_owned(),
            "completed" | "failed" | "cancelled" | "obsolete" => "archived".to_owned(),
            _ => "exited".to_owned(),
        })
}

fn parse_phase_history(structured_trace: Option<&str>) -> Option<Vec<String>> {
    structured_trace
        .and_then(|raw| serde_json::from_str::<Value>(raw).ok())
        .and_then(|value| {
            value
                .get("phase_history")
                .and_then(Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(Value::as_str)
                        .map(str::to_owned)
                        .collect::<Vec<_>>()
                })
        })
}

fn extract_daemon_instance_id(structured_trace: Option<&str>) -> Option<String> {
    structured_trace
        .and_then(|raw| serde_json::from_str::<Value>(raw).ok())
        .and_then(|value| {
            value
                .get("daemon_instance_id")
                .and_then(Value::as_str)
                .map(str::to_owned)
                .or_else(|| {
                    value
                        .get("runtime_context")
                        .and_then(|inner| inner.get("daemon_instance_id"))
                        .and_then(Value::as_str)
                        .map(str::to_owned)
                })
        })
}

fn duration_ms(started_at: Option<&str>, ended_at: Option<&str>) -> Option<i64> {
    let started_at = started_at?;
    let started_at = DateTime::parse_from_rfc3339(started_at).ok()?;
    let ended_at = ended_at
        .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
        .unwrap_or_else(|| Utc::now().fixed_offset());
    Some((ended_at - started_at).num_milliseconds())
}

fn derive_error_severity(error_type: &str, cleanup_verdict: Option<&str>) -> String {
    if cleanup_verdict == Some("leaked")
        || matches!(
            error_type,
            "AGENT_CLEANUP_INCOMPLETE"
                | "AGENT_DAEMON_CRASH_DURING_SESSION"
                | "HOST_RESOURCE_EXHAUSTED"
                | "DAEMON_STARTUP_FAILED"
        )
    {
        return "critical".to_owned();
    }

    if matches!(
        error_type,
        "AGENT_STALLED"
            | "AGENT_AUTH_CHALLENGE"
            | "AGENT_SKILL_FAILED"
            | "AGENT_OUTPUT_CONTRACT_INVALID"
            | "AGENT_SUSPECT_OUTPUT"
            | "AGENT_PARTIAL_FAILURE"
            | "AGENT_INTERNAL_ERROR"
            | "AGENT_RUN_FAILED"
            | "AGENT_PROTOCOL_MISMATCH"
            | "SOURCE_SOFT_BLOCKED"
            | "SOURCE_CAPTCHA_BLOCKED"
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
            | "HOST_SLEEP_INTERRUPTED"
    ) {
        return "warning".to_owned();
    }

    "info".to_owned()
}

fn manifest_sections(markdown: &str) -> Vec<String> {
    markdown
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim_start();
            trimmed
                .strip_prefix('#')
                .map(str::trim_start)
                .filter(|value| !value.is_empty())
                .map(str::to_owned)
        })
        .collect()
}

fn load_pty_history(
    session_id: &str,
    index_path: &str,
    request: &SessionHistoryRequest,
) -> Result<SessionHistoryChunk, QueryServiceError> {
    let index_path = PathBuf::from(index_path);
    let entries = read_stream_index(&index_path)?;
    if entries.is_empty() {
        return Err(QueryServiceError::artifact_unavailable(format!(
            "session `{session_id}` has no durable stream index"
        )));
    }

    let max_bytes = request.max_bytes.unwrap_or(256 * 1024);
    let mut frames = Vec::new();
    let mut bytes_read = 0_usize;
    let mut coverage_end = request.from_seq.saturating_sub(1);

    for entry in entries {
        if entry.last_seq < request.from_seq {
            continue;
        }
        if entry.first_seq > request.to_seq {
            break;
        }
        if entry.first_seq > coverage_end.saturating_add(1) {
            break;
        }

        let chunk_path = index_path.parent().map_or_else(
            || PathBuf::from(&entry.chunk),
            |parent| parent.join(&entry.chunk),
        );
        let Some(bytes) = fs::read(&chunk_path)
            .ok()
            .and_then(|encoded| zstd::stream::decode_all(encoded.as_slice()).ok())
        else {
            break;
        };
        if bytes_read.saturating_add(bytes.len()) > max_bytes {
            break;
        }
        bytes_read += bytes.len();
        coverage_end = coverage_end.max(entry.last_seq.min(request.to_seq));
        frames.push(SessionFrame::Pty {
            seq: entry.first_seq,
            chunk: bytes,
            ts: entry.wall_ts_start,
        });
    }

    let availability = if coverage_end >= request.to_seq {
        SessionHistoryAvailability::Complete
    } else if frames.is_empty() {
        SessionHistoryAvailability::Missing
    } else {
        SessionHistoryAvailability::Partial
    };

    Ok(SessionHistoryChunk {
        session_id: session_id.to_owned(),
        first_seq: request.from_seq,
        last_seq: coverage_end.max(request.from_seq),
        frames,
        availability,
    })
}

fn load_contract_history(
    session_id: &str,
    transcript_path: &str,
    request: &SessionHistoryRequest,
) -> Result<SessionHistoryChunk, QueryServiceError> {
    let transcript = fs::read_to_string(transcript_path).map_err(|error| {
        QueryServiceError::artifact_unavailable(format!(
            "reading transcript {transcript_path} failed: {error}"
        ))
    })?;

    if request.from_seq > 1 {
        return Ok(SessionHistoryChunk {
            session_id: session_id.to_owned(),
            first_seq: request.from_seq,
            last_seq: request.from_seq,
            frames: Vec::new(),
            availability: SessionHistoryAvailability::Missing,
        });
    }

    let max_bytes = request.max_bytes.unwrap_or(256 * 1024);
    let bytes = transcript.into_bytes();
    let allowed = bytes.len().min(max_bytes);
    let text = String::from_utf8_lossy(&bytes[..allowed]).into_owned();
    let availability = if allowed == bytes.len() {
        SessionHistoryAvailability::Complete
    } else {
        SessionHistoryAvailability::Partial
    };

    Ok(SessionHistoryChunk {
        session_id: session_id.to_owned(),
        first_seq: 1,
        last_seq: 1,
        frames: vec![SessionFrame::Stdout {
            seq: 1,
            chunk: text,
            ts: current_iso_timestamp(),
        }],
        availability,
    })
}

fn read_stream_index(path: &Path) -> Result<Vec<StreamIndexEntry>, QueryServiceError> {
    let content = fs::read_to_string(path).map_err(|error| {
        QueryServiceError::artifact_unavailable(format!(
            "reading stream index {} failed: {error}",
            path.display()
        ))
    })?;
    content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            serde_json::from_str::<StreamIndexEntry>(line).map_err(|error| {
                QueryServiceError::artifact_unavailable(format!(
                    "parsing stream index {} failed: {error}",
                    path.display()
                ))
            })
        })
        .collect()
}

fn current_iso_timestamp() -> String {
    Utc::now().to_rfc3339()
}

trait Pipe: Sized {
    fn pipe<T>(self, f: impl FnOnce(Self) -> T) -> T {
        f(self)
    }
}

impl<T> Pipe for T {}
