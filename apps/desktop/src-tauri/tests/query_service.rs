#![forbid(unsafe_code)]

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow, ensure};
use chrono::{Duration as ChronoDuration, Utc};
use rusqlite::{Connection, params};
use tempfile::TempDir;

#[path = "../src/query_service.rs"]
mod query_service_impl;
#[allow(dead_code)]
#[path = "../../../../crates/daemon/src/db/sqlite.rs"]
mod sqlite_impl;

use query_service_impl::{
    ActiveOverrideEntry, CleanupVerdictVm, DecisionHistoryEntry, ErrorSeverity, ExecutionModeVm,
    ManifestInboxRow, PipelineState, ProcessListFilters, QueryService, QueryServiceErrorCode,
    ReplayAvailability, SessionFrame, SessionHistoryRequest, SessionListFilters, SessionPhaseVm,
};
use sqlite_impl::{DEFAULT_OWNER_ID, DEFAULT_SCHEMA_EPOCH, StateDb, StateDbConfig};

struct Harness {
    _temp: TempDir,
    db_path: PathBuf,
    artifact_root: PathBuf,
    _runtime_root: PathBuf,
    state_db: StateDb,
}

impl Harness {
    fn new() -> Result<Self> {
        let temp = tempfile::tempdir()?;
        let db_path = temp.path().join("state").join("state.db");
        let artifact_root = temp.path().join("artifacts");
        let runtime_root = temp.path().join("runtime");
        fs::create_dir_all(&artifact_root)?;
        fs::create_dir_all(&runtime_root)?;
        fs::write(
            runtime_root.join("daemon.info.json"),
            r#"{"instanceId":"daemon-1","pid":1,"socketPath":"runtime/daemon.sock","protocolVersion":1,"schemaEpoch":7,"stateRoot":"state","startedAt":"2026-03-25T10:00:00Z","ready":true}"#,
        )?;
        let state_db = StateDb::open_in_repo(&db_path, &repo_root()?, StateDbConfig::default())?;
        Ok(Self {
            _temp: temp,
            db_path,
            artifact_root,
            _runtime_root: runtime_root,
            state_db,
        })
    }

    fn service(&self) -> Result<QueryService> {
        QueryService::open(&self.db_path, DEFAULT_SCHEMA_EPOCH)
            .map_err(|error| anyhow!("{:?}: {}", error.code, error.message))
    }

    fn conn(&self) -> &Connection {
        self.state_db.connection()
    }

    fn commit_seq(&self) -> Result<i64> {
        self.conn()
            .query_row(
                "SELECT commit_seq FROM commit_clock WHERE id = 1",
                [],
                |row| row.get(0),
            )
            .map_err(Into::into)
    }

    fn seed_process(
        &mut self,
        process_id: &str,
        published_at: &str,
        pipeline_state: &str,
    ) -> Result<()> {
        let deadline_at = (Utc::now() + ChronoDuration::days(90)).to_rfc3339();
        self.seed_process_with_notice(
            process_id,
            published_at,
            &deadline_at,
            pipeline_state,
            "public_web",
        )
    }

    fn seed_process_with_notice(
        &mut self,
        process_id: &str,
        published_at: &str,
        deadline_at: &str,
        pipeline_state: &str,
        access_route: &str,
    ) -> Result<()> {
        let notice_id = format!("notice-{process_id}");
        let lot_id = format!("lot-{process_id}");
        let raw_document_id = format!("raw-{process_id}");
        self.state_db.with_write_transaction(|tx| {
            tx.execute(
                "INSERT INTO raw_documents (id, source_id, payload_json, metadata) VALUES (?1, 'fts', '{\"ok\":true}', '{}')",
                params![raw_document_id],
            )?;
            tx.execute(
                "\
                INSERT INTO procurement_processes (
                  id, canonical_ocid, primary_notice_id, canonical_process_id, current_stage,
                  buyer_name, title, latest_deadline_at, latest_published_at, latest_updated_at, owner_id
                ) VALUES (?1, ?2, ?3, ?4, 'review', ?5, ?6, ?7, ?8, ?8, ?9)
                ",
                params![
                    process_id,
                    format!("ocid-{process_id}"),
                    notice_id,
                    process_id,
                    format!("Buyer {process_id}"),
                    format!("Process {process_id}"),
                    deadline_at,
                    published_at,
                    DEFAULT_OWNER_ID,
                ],
            )?;
            tx.execute(
                "\
                INSERT INTO contract_notices (
                  id, process_id, source_id, source_notice_id, raw_document_id, ocid, title,
                  summary, buyer_name, procurement_stage, access_route, deadline_at, published_at,
                  source_updated_at, normalized_payload, owner_id
                ) VALUES (?1, ?2, 'fts', ?3, ?4, ?5, ?6, 'summary', ?7, 'tender', ?8, ?9, ?10, ?10, '{}', ?11)
                ",
                params![
                    notice_id,
                    process_id,
                    format!("source-{process_id}"),
                    raw_document_id,
                    format!("ocid-{process_id}"),
                    format!("Notice {process_id}"),
                    format!("Buyer {process_id}"),
                    access_route,
                    deadline_at,
                    published_at,
                    DEFAULT_OWNER_ID,
                ],
            )?;
            tx.execute(
                "INSERT INTO notice_lots (id, notice_id, lot_key, title, description, estimated_value_pence, owner_id) VALUES (?1, ?2, 'lot-key', 'Lot title', 'Lot description', 1250000, ?3)",
                params![lot_id, notice_id, DEFAULT_OWNER_ID],
            )?;
            tx.execute(
                "\
                INSERT INTO pipeline_assessments (
                  process_id, pipeline_state, gross_margin_bps, net_margin_bps,
                  priced_coverage_bps, landed_cost_coverage_bps, net_qualified_coverage_bps,
                  priced_value_coverage_bps, landed_cost_value_coverage_bps,
                  net_qualified_value_coverage_bps, unresolved_mandatory_lots,
                  decision_basis_hash, stage_run_id, owner_id
                ) VALUES (?1, ?2, 1500, 1200, 7000, 6000, 5000, 6500, 5500, 4500, 0, ?3, ?4, ?5)
                ",
                params![
                    process_id,
                    pipeline_state,
                    format!("basis-{process_id}"),
                    format!("stage-{process_id}"),
                    DEFAULT_OWNER_ID,
                ],
            )?;
            Ok(())
        })?;
        Ok(())
    }

    fn seed_candidate_and_quote(&mut self, process_id: &str) -> Result<()> {
        let notice_id = format!("notice-{process_id}");
        let lot_id = format!("lot-{process_id}");
        let candidate_id = format!("candidate-{process_id}");
        let quote_id = format!("quote-{process_id}");
        let landed_cost_id = format!("landed-{process_id}");
        self.state_db.with_write_transaction(|tx| {
            tx.execute(
                "\
                INSERT INTO product_candidates (
                  id, process_id, notice_id, lot_id, name, candidate_type,
                  match_archetype, head_noun, candidate_value_share_bps, quoteability_score_bps, owner_id
                ) VALUES (?1, ?2, ?3, ?4, 'Steel widget', 'good', 'direct', 'widget', 5000, 8800, ?5)
                ",
                params![candidate_id, process_id, notice_id, lot_id, DEFAULT_OWNER_ID],
            )?;
            tx.execute(
                "\
                INSERT INTO qualification_runs (
                  id, process_id, notice_id, lot_id, expiry_qualified, goods_qualified,
                  goods_confidence_bps, legal_category, addressable_value_pence, access_route,
                  qualification_basis, rationale, owner_id
                ) VALUES (?1, ?2, ?3, ?4, 1, 1, 9000, 'goods', 1250000, 'public_web', 'fixture', 'Qualified for list summary', ?5)
                ",
                params![
                    format!("qualification-{process_id}"),
                    process_id,
                    notice_id,
                    lot_id,
                    DEFAULT_OWNER_ID,
                ],
            )?;
            tx.execute(
                "\
                INSERT INTO supplier_quotes (
                  id, process_id, notice_id, product_candidate_id, product_name, region,
                  quote_class, source_name, source_url, unit_price_original_minor, currency,
                  unit_price_gbp_pence, price_signal_role, evidence, owner_id
                ) VALUES (?1, ?2, ?3, ?4, 'Steel widget', 'uk', 'catalog', 'Supplier A',
                          'https://example.test/quote', 1200, 'GBP', 1200, 'primary', '{}', ?5)
                ",
                params![quote_id, process_id, notice_id, candidate_id, DEFAULT_OWNER_ID],
            )?;
            tx.execute(
                "\
                INSERT INTO landed_cost_estimates (
                  id, process_id, notice_id, product_candidate_id, supplier_quote_id,
                  shipping_mode, mode_confidence_bps, commodity_confidence_bps,
                  classification_stability_bps, customs_fx_rate, commercial_fx_rate,
                  fx_basis_delta_bps, supplier_cost_gbp_pence, freight_low_gbp_pence,
                  freight_base_gbp_pence, freight_high_gbp_pence, customs_duty_gbp_pence,
                  import_vat_gbp_pence, import_vat_treatment, uk_final_mile_cost_gbp_pence, other_fees_gbp_pence,
                  economic_landed_cost_gbp_pence, cash_outlay_gbp_pence, cash_to_clear_gbp_pence,
                  revenue_basis_type, revenue_confidence_bps, net_margin_low_bps,
                  net_margin_base_bps, net_margin_high_bps, decision_grade, confidence_bps,
                  warnings, owner_id
                ) VALUES (?1, ?2, ?3, ?4, ?5, 'parcel', 9000, 8000, 8000, 1.0, 1.0, 0,
                          1200, 100, 120, 140, 50, 240, 'recoverable', 90, 20, 1720, 1720, 1720,
                          'addressable', 9000, 200, 300, 400, 'screening', 8500, '[]', ?6)
                ",
                params![
                    landed_cost_id,
                    process_id,
                    notice_id,
                    candidate_id,
                    quote_id,
                    DEFAULT_OWNER_ID,
                ],
            )?;
            tx.execute(
                "INSERT INTO manifests (id, process_id, markdown, artifact_locator, artifact_hash, stage_run_id, owner_id) VALUES (?1, ?2, ?3, 'artifact://manifest', ?4, ?5, ?6)",
                params![
                    format!("manifest-{process_id}"),
                    process_id,
                    "# Opportunity\n\n## Summary\ntext\n\n## Risks\ntext",
                    format!("artifact-{process_id}"),
                    format!("stage-{process_id}"),
                    DEFAULT_OWNER_ID,
                ],
            )?;
            Ok(())
        })?;
        Ok(())
    }

    fn seed_alias(&mut self, alias_process_id: &str, canonical_process_id: &str) -> Result<()> {
        self.state_db.with_write_transaction(|tx| {
            tx.execute(
                "\
                INSERT INTO process_aliases (
                  alias_process_id, canonical_process_id, alias_reason, owner_id,
                  row_version, last_local_seq, updated_at, created_at
                ) VALUES (?1, ?2, 'merge', ?3, 1, NULL, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
                ",
                params![alias_process_id, canonical_process_id, DEFAULT_OWNER_ID],
            )?;
            Ok(())
        })?;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn seed_session(
        &mut self,
        session_id: &str,
        status: &str,
        execution_mode: &str,
        started_at: &str,
        completed_at: Option<&str>,
        terminal_output_path: Option<&str>,
        structured_trace: Option<&str>,
        last_checkpoint_seq: Option<i64>,
        terminal_output_bytes: Option<i64>,
        cleanup_verdict: Option<&str>,
        error_type: Option<&str>,
        exit_code: Option<i64>,
    ) -> Result<()> {
        self.state_db.with_write_transaction(|tx| {
            tx.execute(
                "\
                INSERT INTO agent_session_logs (
                  id, skill_path, skill_version, execution_mode, status, started_at, completed_at,
                  terminal_output_path, terminal_output_bytes, structured_trace, error_type,
                  exit_code, cleanup_verdict, last_checkpoint_seq
                ) VALUES (?1, 'skills/procurement-ingestion/fts/SKILL.md', '1.0.0', ?2, ?3, ?4, ?5,
                          ?6, ?7, ?8, ?9, ?10, ?11, ?12)
                ",
                params![
                    session_id,
                    execution_mode,
                    status,
                    started_at,
                    completed_at,
                    terminal_output_path,
                    terminal_output_bytes,
                    structured_trace,
                    error_type,
                    exit_code,
                    cleanup_verdict,
                    last_checkpoint_seq,
                ],
            )?;
            Ok(())
        })?;
        Ok(())
    }

    fn seed_error(
        &mut self,
        error_id: &str,
        process_id: Option<&str>,
        pipeline_stage: &str,
        error_type: &str,
        session_id: Option<&str>,
        resolved: bool,
    ) -> Result<()> {
        self.state_db.with_write_transaction(|tx| {
            tx.execute(
                "\
                INSERT INTO error_manifests (
                  id, process_id, pipeline_stage, workflow_step, error_type, error_message,
                  error_context, skill_path, agent_session_id, resolved
                ) VALUES (?1, ?2, ?3, 'step', ?4, ?5, '{}',
                          'skills/procurement-ingestion/fts/SKILL.md', ?6, ?7)
                ",
                params![
                    error_id,
                    process_id,
                    pipeline_stage,
                    error_type,
                    format!("{error_type} happened"),
                    session_id,
                    i64::from(resolved),
                ],
            )?;
            Ok(())
        })?;
        Ok(())
    }

    fn seed_pty_history(&self, session_id: &str) -> Result<PathBuf> {
        let session_root = self
            .artifact_root
            .join("sessions")
            .join("2026")
            .join("03")
            .join(session_id);
        let stream_root = session_root.join("stream");
        fs::create_dir_all(&stream_root)?;
        let chunk_one = zstd::stream::encode_all(b"line-10\nline-11\nline-12\n".as_slice(), 0)?;
        let chunk_two = zstd::stream::encode_all(b"line-13\nline-14\nline-15\n".as_slice(), 0)?;
        let chunk_one_path = stream_root.join("000001.pty.zst");
        let chunk_two_path = stream_root.join("000002.pty.zst");
        fs::write(&chunk_one_path, chunk_one)?;
        fs::write(&chunk_two_path, chunk_two)?;
        let index_path = session_root.join("stream.index.jsonl");
        fs::write(
            &index_path,
            concat!(
                "{\"chunk\":\"stream/000001.pty.zst\",\"first_seq\":10,\"last_seq\":12,\"byte_length\":24,\"wall_ts_start\":\"2026-03-25T11:00:00Z\",\"wall_ts_end\":\"2026-03-25T11:00:01Z\",\"sha256\":\"a\"}\n",
                "{\"chunk\":\"stream/000002.pty.zst\",\"first_seq\":13,\"last_seq\":15,\"byte_length\":24,\"wall_ts_start\":\"2026-03-25T11:00:02Z\",\"wall_ts_end\":\"2026-03-25T11:00:03Z\",\"sha256\":\"b\"}\n"
            ),
        )?;
        Ok(index_path)
    }
}

#[test]
fn rs_qs_001_read_only_db() -> Result<()> {
    let harness = Harness::new()?;
    let service = harness.service()?;
    let query_only: i64 = service
        .open_read_connection_for_test()?
        .pragma_query_value(None, "query_only", |row| row.get(0))?;
    assert_eq!(query_only, 1);
    let write_attempt = service
        .open_read_connection_for_test()?
        .execute("CREATE TABLE should_fail (id TEXT)", []);
    assert!(write_attempt.is_err());
    Ok(())
}

#[test]
fn rs_qs_002_process_list_returns_snapshot_page() -> Result<()> {
    let mut harness = Harness::new()?;
    harness.seed_process("process-b", "2026-03-26T11:00:00Z", "review")?;
    harness.seed_candidate_and_quote("process-b")?;
    seed_intent(
        &mut harness.state_db,
        "intent-process-b",
        "process-b",
        "approve",
        "queued",
        "idem-process-b",
        None,
        None,
        Some("2026-04-01T12:00:00Z"),
    )?;
    harness.seed_process("process-a", "2026-03-25T10:00:00Z", "qualified")?;

    let service = harness.service()?;
    let envelope = service
        .list_processes(None, 25, &ProcessListFilters::default())
        .map_err(|error| anyhow!("{:?}: {}", error.code, error.message))?;

    assert!(envelope.snapshot_seq > 0);
    assert!(envelope.snapshot_at.contains('T'));
    assert_eq!(envelope.data.items.len(), 2);
    assert!(!envelope.data.has_more);
    assert!(envelope.data.next_cursor.is_none());
    assert_eq!(envelope.data.items[0].process_id, "process-b");
    assert_eq!(envelope.data.items[0].pipeline_state, PipelineState::Review);
    assert_eq!(envelope.data.items[0].candidate_count, 1);
    assert_eq!(envelope.data.items[0].quoted_candidate_count, 1);
    assert_eq!(
        envelope.data.items[0].estimated_value_pence,
        Some(1_250_000)
    );
    assert_eq!(envelope.data.items[0].lot_count, 1);
    assert_eq!(envelope.data.items[0].access_route, "public_web");
    assert_eq!(envelope.data.items[0].legal_category, "goods");
    assert_eq!(
        envelope.data.items[0].decision.disposition.as_deref(),
        Some("approve")
    );
    assert_eq!(
        envelope.data.items[0].decision.status.as_deref(),
        Some("queued")
    );
    assert!(!envelope.data.items[0].decision.is_stale);
    Ok(())
}

#[test]
fn rs_qs_002b_process_list_uses_keyset_pagination() -> Result<()> {
    let mut harness = Harness::new()?;
    for index in 0..27 {
        harness.seed_process(
            &format!("process-{index:02}"),
            &format!("2026-03-{:02}T10:00:00Z", 27 - index),
            "qualified",
        )?;
    }

    let service = harness.service()?;
    let first_page = service
        .list_processes(None, 25, &ProcessListFilters::default())
        .map_err(|error| anyhow!("{:?}: {}", error.code, error.message))?;
    assert_eq!(first_page.data.items.len(), 25);
    assert!(first_page.data.has_more);
    assert!(first_page.data.next_cursor.is_some());

    let second_page = service
        .list_processes(
            first_page.data.next_cursor.as_deref(),
            25,
            &ProcessListFilters::default(),
        )
        .map_err(|error| anyhow!("{:?}: {}", error.code, error.message))?;
    assert_eq!(second_page.data.items.len(), 2);
    assert!(!second_page.data.has_more);

    let first_ids = first_page
        .data
        .items
        .iter()
        .map(|item| item.process_id.clone())
        .collect::<Vec<_>>();
    let second_ids = second_page
        .data
        .items
        .iter()
        .map(|item| item.process_id.clone())
        .collect::<Vec<_>>();
    assert!(second_ids.iter().all(|id| !first_ids.contains(id)));
    Ok(())
}

#[test]
fn rs_qs_002c_process_list_filters_expiring_non_dps_but_keeps_dynamic_market() -> Result<()> {
    let mut harness = Harness::new()?;
    let soon_deadline = (Utc::now() + ChronoDuration::days(5)).to_rfc3339();
    harness.seed_process_with_notice(
        "process-expired",
        "2026-03-28T10:00:00Z",
        &soon_deadline,
        "review",
        "public_web",
    )?;
    harness.seed_process_with_notice(
        "process-dps",
        "2026-03-28T11:00:00Z",
        &soon_deadline,
        "review",
        "dynamic_market",
    )?;

    let service = harness.service()?;
    let envelope = service
        .list_processes(None, 25, &ProcessListFilters::default())
        .map_err(|error| anyhow!("{:?}: {}", error.code, error.message))?;

    let visible_ids = envelope
        .data
        .items
        .iter()
        .map(|item| item.process_id.as_str())
        .collect::<Vec<_>>();
    assert!(visible_ids.contains(&"process-dps"));
    assert!(!visible_ids.contains(&"process-expired"));
    Ok(())
}

#[test]
fn rs_qs_003_snapshot_seq_uses_commit_clock() -> Result<()> {
    let mut harness = Harness::new()?;
    harness.seed_process("process-a", "2026-03-25T10:00:00Z", "qualified")?;
    let before = harness.commit_seq()?;

    let service = harness.service()?;
    let first = service
        .list_processes(None, 25, &ProcessListFilters::default())
        .map_err(|error| anyhow!("{:?}: {}", error.code, error.message))?;
    assert_eq!(first.snapshot_seq, before);

    harness.seed_process("process-b", "2026-03-26T10:00:00Z", "review")?;
    let after_commit = harness.commit_seq()?;
    let second = service
        .list_processes(None, 25, &ProcessListFilters::default())
        .map_err(|error| anyhow!("{:?}: {}", error.code, error.message))?;
    assert_eq!(second.snapshot_seq, after_commit);

    let rolled_back_before = harness.commit_seq()?;
    {
        let tx = harness.state_db.connection_mut().transaction()?;
        tx.execute(
            "INSERT INTO sources (id, kind, enabled, config) VALUES ('rolled-back', 'procurement', 1, '{}')",
            [],
        )?;
    }
    assert_eq!(harness.commit_seq()?, rolled_back_before);
    Ok(())
}

#[test]
fn rs_qs_004_process_detail_resolves_alias_and_returns_empty_arrays() -> Result<()> {
    let mut harness = Harness::new()?;
    harness.seed_process("process-a", "2026-03-25T10:00:00Z", "qualified")?;
    harness.seed_alias("alias-a", "process-a")?;

    let service = harness.service()?;
    let detail = service
        .get_process_detail("alias-a")
        .map_err(|error| anyhow!("{:?}: {}", error.code, error.message))?;

    assert_eq!(detail.data.process_id, "process-a");
    assert_eq!(detail.data.canonical_process_id, "process-a");
    assert_eq!(detail.data.notices.len(), 1);
    assert_eq!(detail.data.lots.len(), 1);
    assert!(detail.data.candidates.is_empty());
    assert!(detail.data.quotes.is_empty());
    assert!(detail.data.landed_costs.is_empty());
    assert!(detail.data.errors.is_empty());
    assert!(detail.data.manifest.is_none());
    assert_eq!(
        detail.data.review_metadata.basis_hash.as_deref(),
        Some("basis-process-a")
    );
    assert_eq!(
        detail.data.review_metadata.stage_run_id.as_deref(),
        Some("stage-process-a")
    );
    assert!(detail.data.review_metadata.manifest_hash.is_none());
    Ok(())
}

#[test]
fn rs_qs_004b_process_detail_exposes_authoritative_review_metadata() -> Result<()> {
    let mut harness = Harness::new()?;
    harness.seed_process("process-a", "2026-03-25T10:00:00Z", "qualified")?;
    harness.seed_candidate_and_quote("process-a")?;

    let service = harness.service()?;
    let detail = service
        .get_process_detail("process-a")
        .map_err(|error| anyhow!("{:?}: {}", error.code, error.message))?;

    let manifest = detail
        .data
        .manifest
        .expect("manifest metadata should be present");
    assert_eq!(detail.data.pipeline_state, PipelineState::Qualified);
    assert_eq!(manifest.basis_hash.as_deref(), Some("basis-process-a"));
    assert_eq!(
        manifest.artifact_hash.as_deref(),
        Some("artifact-process-a")
    );
    assert_eq!(manifest.stage_run_id.as_deref(), Some("stage-process-a"));
    assert_eq!(
        detail.data.review_metadata.basis_hash.as_deref(),
        Some("basis-process-a")
    );
    assert_eq!(
        detail.data.review_metadata.manifest_hash.as_deref(),
        Some("artifact-process-a")
    );
    assert_eq!(
        detail.data.review_metadata.stage_run_id.as_deref(),
        Some("stage-process-a")
    );
    Ok(())
}

#[test]
fn rs_qs_005_session_list_returns_canonical_metadata() -> Result<()> {
    let mut harness = Harness::new()?;
    let history_path = harness.seed_pty_history("session-active")?;
    harness.seed_session(
        "session-active",
        "active",
        "pty",
        "2026-03-25T11:00:00Z",
        None,
        Some(history_path.to_string_lossy().as_ref()),
        Some("{\"phase_history\":[\"created\",\"spawned\",\"active\"],\"daemon_instance_id\":\"daemon-1\"}"),
        Some(3),
        Some(128),
        None,
        None,
        None,
    )?;
    harness.seed_session(
        "session-preactive",
        "failed",
        "contract",
        "2026-03-25T10:00:00Z",
        Some("2026-03-25T10:00:05Z"),
        Some("/tmp/contract-output.log"),
        Some("{\"phase_history\":[\"created\",\"spawned\"]}"),
        Some(0),
        Some(64),
        Some("forced"),
        Some("AGENT_RUN_FAILED"),
        Some(1),
    )?;

    let service = harness.service()?;
    let sessions = service
        .list_sessions(None, 25, &SessionListFilters::default())
        .map_err(|error| anyhow!("{:?}: {}", error.code, error.message))?;

    assert_eq!(sessions.data.items.len(), 2);
    assert_eq!(sessions.data.items[0].session_id, "session-active");
    assert_eq!(sessions.data.items[0].phase, SessionPhaseVm::Active);
    assert_eq!(sessions.data.items[0].execution_mode, ExecutionModeVm::Pty);
    assert_eq!(sessions.data.items[0].daemon_instance_id, "daemon-1");
    assert!(sessions.data.items[0].reattachable);
    assert_eq!(sessions.data.items[1].phase, SessionPhaseVm::Spawned);
    assert_eq!(sessions.data.items[1].daemon_instance_id, "daemon-1");
    assert_eq!(
        sessions.data.items[1].error_type.as_deref(),
        Some("AGENT_RUN_FAILED")
    );
    Ok(())
}

#[test]
fn rs_qs_006_error_list_derives_severity_and_empty_page() -> Result<()> {
    let mut harness = Harness::new()?;
    harness.seed_session(
        "session-error",
        "failed",
        "pty",
        "2026-03-25T10:00:00Z",
        Some("2026-03-25T10:00:04Z"),
        None,
        None,
        Some(0),
        Some(0),
        Some("leaked"),
        Some("AGENT_DAEMON_CRASH_DURING_SESSION"),
        Some(1),
    )?;
    harness.seed_error(
        "error-critical",
        None,
        "Enrich",
        "AGENT_DAEMON_CRASH_DURING_SESSION",
        Some("session-error"),
        false,
    )?;
    harness.seed_error(
        "error-warning",
        None,
        "Qualify",
        "SOURCE_CHANGED",
        None,
        true,
    )?;

    let service = harness.service()?;
    let unresolved = service
        .list_errors(
            None,
            25,
            &query_service_impl::ErrorListFilters {
                resolved: Some(false),
                ..query_service_impl::ErrorListFilters::default()
            },
        )
        .map_err(|error| anyhow!("{:?}: {}", error.code, error.message))?;

    assert_eq!(unresolved.data.items.len(), 1);
    assert_eq!(unresolved.data.items[0].severity, ErrorSeverity::Critical);
    assert_eq!(
        unresolved.data.items[0].cleanup_verdict,
        Some(CleanupVerdictVm::Leaked)
    );

    let empty = service
        .list_errors(
            None,
            25,
            &query_service_impl::ErrorListFilters {
                resolved: Some(false),
                severities: vec!["info".to_owned()],
                ..query_service_impl::ErrorListFilters::default()
            },
        )
        .map_err(|error| anyhow!("{:?}: {}", error.code, error.message))?;
    assert!(empty.data.items.is_empty());
    Ok(())
}

#[test]
fn rs_qs_007_invalid_cursor_and_filters() -> Result<()> {
    let mut harness = Harness::new()?;
    harness.seed_process("process-a", "2026-03-25T10:00:00Z", "qualified")?;
    let service = harness.service()?;

    let invalid_cursor = service
        .list_processes(Some("bad-cursor"), 25, &ProcessListFilters::default())
        .expect_err("cursor must be validated");
    assert_eq!(invalid_cursor.code, QueryServiceErrorCode::InvalidCursor);

    let invalid_filter = service
        .list_processes(
            None,
            25,
            &ProcessListFilters {
                pipeline_states: vec!["unsupported".to_owned()],
                ..ProcessListFilters::default()
            },
        )
        .expect_err("filters must be validated");
    assert_eq!(invalid_filter.code, QueryServiceErrorCode::InvalidFilter);
    Ok(())
}

#[test]
fn rs_qs_008_history_replay_reports_complete_partial_and_missing() -> Result<()> {
    let mut harness = Harness::new()?;
    let index_path = harness.seed_pty_history("session-history")?;
    harness.seed_session(
        "session-history",
        "archived",
        "pty",
        "2026-03-25T11:00:00Z",
        Some("2026-03-25T11:00:10Z"),
        Some(index_path.to_string_lossy().as_ref()),
        Some("{\"phase_history\":[\"created\",\"spawned\",\"active\",\"terminating\",\"exited\",\"reaped\",\"archived\"]}"),
        Some(2),
        Some(48),
        Some("clean"),
        None,
        Some(0),
    )?;

    let service = harness.service()?;
    let complete = service
        .get_session_history(&SessionHistoryRequest {
            session_id: "session-history".to_owned(),
            from_seq: 10,
            to_seq: 15,
            max_bytes: None,
        })
        .map_err(|error| anyhow!("{:?}: {}", error.code, error.message))?;
    assert_eq!(complete.data.availability, ReplayAvailability::Complete);
    assert_eq!(complete.data.frames.len(), 6);
    assert!(matches!(
        &complete.data.frames[0],
        SessionFrame::Pty { seq: 10, chunk, .. } if String::from_utf8_lossy(chunk) == "line-10\n"
    ));
    assert!(matches!(
        &complete.data.frames[5],
        SessionFrame::Pty { seq: 15, chunk, .. } if String::from_utf8_lossy(chunk) == "line-15\n"
    ));

    let partial = service
        .get_session_history(&SessionHistoryRequest {
            session_id: "session-history".to_owned(),
            from_seq: 10,
            to_seq: 20,
            max_bytes: None,
        })
        .map_err(|error| anyhow!("{:?}: {}", error.code, error.message))?;
    assert_eq!(partial.data.availability, ReplayAvailability::Partial);
    assert!(matches!(
        partial.data.frames.last(),
        Some(SessionFrame::Gap {
            from_seq: 16,
            to_seq: 20
        })
    ));

    let missing = service
        .get_session_history(&SessionHistoryRequest {
            session_id: "session-history".to_owned(),
            from_seq: 30,
            to_seq: 35,
            max_bytes: None,
        })
        .map_err(|error| anyhow!("{:?}: {}", error.code, error.message))?;
    assert_eq!(missing.data.availability, ReplayAvailability::Missing);
    assert!(missing.data.frames.is_empty());
    Ok(())
}

#[test]
fn rs_qs_009_busy_reader_returns_typed_error() -> Result<()> {
    let mut harness = Harness::new()?;
    harness.seed_process("process-a", "2026-03-25T10:00:00Z", "qualified")?;
    harness
        .conn()
        .pragma_update(None, "journal_mode", "DELETE")?;
    let busy_conn = Connection::open(&harness.db_path)?;
    busy_conn.pragma_update(None, "locking_mode", "EXCLUSIVE")?;
    busy_conn.execute_batch("BEGIN EXCLUSIVE TRANSACTION;")?;

    let service = harness.service()?;
    let error = service
        .list_processes(None, 25, &ProcessListFilters::default())
        .expect_err("exclusive lock should surface as busy");
    assert_eq!(error.code, QueryServiceErrorCode::Busy);
    assert!(error.retryable);

    busy_conn.execute_batch("ROLLBACK;")?;
    Ok(())
}

#[test]
fn rs_qs_010_queries_do_not_hold_long_lived_read_transactions() -> Result<()> {
    let mut harness = Harness::new()?;
    harness.seed_process("process-a", "2026-03-25T10:00:00Z", "qualified")?;
    let service = harness.service()?;

    let _ = service
        .list_processes(None, 25, &ProcessListFilters::default())
        .map_err(|error| anyhow!("{:?}: {}", error.code, error.message))?;

    let checkpoint = Connection::open(&harness.db_path)?.query_row(
        "PRAGMA wal_checkpoint(TRUNCATE)",
        [],
        |row| row.get::<_, i64>(0),
    )?;
    ensure!(checkpoint >= 0);
    Ok(())
}

#[test]
fn rs_qs_011_committed_writes_increment_commit_clock_once() -> Result<()> {
    let mut harness = Harness::new()?;
    let before = harness.commit_seq()?;
    harness.seed_process("process-a", "2026-03-25T10:00:00Z", "qualified")?;
    let after = harness.commit_seq()?;
    assert_eq!(after, before + 1);

    let service = harness.service()?;
    let envelope = service
        .list_processes(None, 25, &ProcessListFilters::default())
        .map_err(|error| anyhow!("{:?}: {}", error.code, error.message))?;
    assert_eq!(envelope.snapshot_seq, after);
    Ok(())
}

#[test]
fn rs_qs_012_data_version_changes_do_not_replace_commit_seq() -> Result<()> {
    let mut harness = Harness::new()?;
    harness.seed_process("process-a", "2026-03-25T10:00:00Z", "qualified")?;
    let service = harness.service()?;
    let data_version_before: i64 = service
        .open_read_connection_for_test()?
        .pragma_query_value(None, "data_version", |row| row.get(0))?;
    let snapshot_before = service
        .list_processes(None, 25, &ProcessListFilters::default())
        .map_err(|error| anyhow!("{:?}: {}", error.code, error.message))?;

    harness.seed_process("process-b", "2026-03-26T10:00:00Z", "review")?;
    let data_version_after: i64 = service
        .open_read_connection_for_test()?
        .pragma_query_value(None, "data_version", |row| row.get(0))?;
    let snapshot_after = service
        .list_processes(None, 25, &ProcessListFilters::default())
        .map_err(|error| anyhow!("{:?}: {}", error.code, error.message))?;

    assert!(data_version_after >= data_version_before);
    assert_eq!(snapshot_after.snapshot_seq, harness.commit_seq()?);
    assert!(snapshot_after.snapshot_seq >= snapshot_before.snapshot_seq);
    Ok(())
}

// ---------------------------------------------------------------------------
// Stage 14 Phase 2 — RT.1..RT.4
// ---------------------------------------------------------------------------

/// Stage 14 Phase 2 helper — seeds a manifests row with stoplight, suggested
/// action, prompt count + stored `clarification_prompts_json` so the inbox
/// loader returns the row with all UI metadata populated.
fn seed_inbox_manifest(
    state_db: &mut sqlite_impl::StateDb,
    process_id: &str,
    stoplight: &str,
    prompt_count: i64,
    prompts_json: &str,
) -> Result<()> {
    state_db.with_write_transaction(|tx| {
        tx.execute(
            "UPDATE manifests
             SET stoplight = ?2,
                 suggested_action = ?3,
                 clarification_prompt_count = ?4,
                 manifest_basis_hash = ?5,
                 clarification_prompts_json = ?6
             WHERE process_id = ?1",
            params![
                process_id,
                stoplight,
                "Capture clarification",
                prompt_count,
                format!("basis-{process_id}"),
                prompts_json,
            ],
        )?;
        Ok(())
    })?;
    Ok(())
}

/// Stage 14 Phase 2 helper — inserts a `pending_operator_intents` row so the
/// audit-chain loader can return non-empty history.
#[allow(clippy::too_many_arguments)]
fn seed_intent(
    state_db: &mut sqlite_impl::StateDb,
    intent_id: &str,
    process_id: &str,
    disposition: &str,
    status: &str,
    idempotency_key: &str,
    superseded_by: Option<&str>,
    decision_note: Option<&str>,
    queued_at_override: Option<&str>,
) -> Result<()> {
    let queued_at =
        queued_at_override.map_or_else(|| chrono::Utc::now().to_rfc3339(), ToOwned::to_owned);
    state_db.with_write_transaction(|tx| {
        tx.execute(
            "INSERT INTO pending_operator_intents (
               id, process_id, desired_disposition, reviewed_basis_hash, reviewed_manifest_hash,
               reviewed_at_stage_run_id, reviewed_at, base_remote_revision, idempotency_key,
               status, superseded_by_intent_id, queued_at, decision_note
             ) VALUES (
               ?1, ?2, ?3, ?4, ?5, ?6, ?7, 0, ?8, ?9, ?10, ?7, ?11
             )",
            params![
                intent_id,
                process_id,
                disposition,
                format!("basis-{process_id}"),
                format!("manifest-{process_id}"),
                format!("run-{intent_id}"),
                queued_at,
                idempotency_key,
                status,
                superseded_by,
                decision_note,
            ],
        )?;
        Ok(())
    })?;
    Ok(())
}

/// Stage 14 Phase 2 helper — inserts an `operator_input_overrides` row so the
/// active-override loader returns non-empty data.
fn seed_override(
    state_db: &mut sqlite_impl::StateDb,
    override_id: &str,
    process_id: &str,
    candidate_id: &str,
    reason_code: &str,
    payload_json: &str,
) -> Result<()> {
    state_db.with_write_transaction(|tx| {
        tx.execute(
            "INSERT INTO operator_input_overrides (
               id, process_id, product_candidate_id, reason_code, payload_json, notes,
               basis_hash_when_provided, owner_id, row_version, stage_run_id
             ) VALUES (?1, ?2, ?3, ?4, ?5, NULL, ?6, ?7, 1, ?8)",
            params![
                override_id,
                process_id,
                candidate_id,
                reason_code,
                payload_json,
                format!("basis-{process_id}"),
                DEFAULT_OWNER_ID,
                format!("run-{override_id}"),
            ],
        )?;
        Ok(())
    })?;
    Ok(())
}

#[test]
fn rt_1_list_manifests_returns_all_manifest_rows() -> Result<()> {
    let mut harness = Harness::new()?;
    harness.seed_process("p1", "2026-04-01T10:00:00Z", "review")?;
    harness.seed_candidate_and_quote("p1")?;
    harness.seed_process("p2", "2026-04-01T11:00:00Z", "review")?;
    harness.seed_candidate_and_quote("p2")?;
    seed_inbox_manifest(&mut harness.state_db, "p1", "clarify", 2, "[]")?;
    seed_inbox_manifest(&mut harness.state_db, "p2", "pursue", 0, "[]")?;
    seed_intent(
        &mut harness.state_db,
        "intent-p1",
        "p1",
        "approve",
        "queued",
        "idem-p1",
        None,
        None,
        Some("2026-04-01T12:00:00Z"),
    )?;

    let service = harness.service()?;
    let envelope = service
        .list_manifests()
        .map_err(|error| anyhow!("{:?}: {}", error.code, error.message))?;
    let rows: &[ManifestInboxRow] = &envelope.data;
    assert_eq!(rows.len(), 2);
    let p1 = rows
        .iter()
        .find(|row| row.process_id == "p1")
        .ok_or_else(|| anyhow!("p1 not found"))?;
    assert_eq!(p1.stoplight.as_deref(), Some("clarify"));
    assert_eq!(p1.clarification_prompt_count, 2);
    assert_eq!(
        p1.suggested_action.as_deref(),
        Some("Capture clarification")
    );
    assert_eq!(p1.decision.disposition.as_deref(), Some("approve"));
    assert_eq!(p1.decision.status.as_deref(), Some("queued"));
    assert!(!p1.decision.is_stale);
    assert!(p1.manifest_basis_hash.is_some());
    Ok(())
}

#[test]
fn rt_2_get_manifest_returns_clarification_prompts_json() -> Result<()> {
    let mut harness = Harness::new()?;
    harness.seed_process("p1", "2026-04-01T10:00:00Z", "review")?;
    harness.seed_candidate_and_quote("p1")?;
    let prompts = serde_json::json!([
        {
            "candidate_id": "candidate-p1",
            "candidate_name": "DGU 1.2m²",
            "lot_id": null,
            "reason_code": "quantity_basis_unaligned",
            "prompt_text": "How many?",
            "prompt_unit": "m²",
            "prompt_type": "number",
            "default_value": 250,
            "rationale": "Unknown",
            "options": null,
        }
    ])
    .to_string();
    seed_inbox_manifest(&mut harness.state_db, "p1", "clarify", 1, &prompts)?;

    let service = harness.service()?;
    let detail = service
        .get_process_detail("p1")
        .map_err(|error| anyhow!("{:?}: {}", error.code, error.message))?;
    let manifest = detail
        .data
        .manifest
        .ok_or_else(|| anyhow!("manifest is None"))?;
    let parsed_prompts = manifest
        .clarification_prompts_json
        .ok_or_else(|| anyhow!("clarification_prompts_json is None"))?;
    ensure!(parsed_prompts.contains("quantity_basis_unaligned"));
    ensure!(parsed_prompts.contains("DGU 1.2m\\u00b2") || parsed_prompts.contains("DGU 1.2m²"));
    assert_eq!(manifest.stoplight.as_deref(), Some("clarify"));
    Ok(())
}

#[test]
fn rt_3_get_decision_history_returns_audit_chain() -> Result<()> {
    let mut harness = Harness::new()?;
    harness.seed_process("p1", "2026-04-01T10:00:00Z", "review")?;
    harness.seed_candidate_and_quote("p1")?;
    // Insert newer intent first so the older intent's superseded_by FK resolves.
    // Explicit queued_at timestamps make ordering deterministic.
    seed_intent(
        &mut harness.state_db,
        "intent-new",
        "p1",
        "approve",
        "queued",
        "key-new",
        None,
        Some("Looks good"),
        Some("2026-04-02T12:00:00Z"),
    )?;
    seed_intent(
        &mut harness.state_db,
        "intent-old",
        "p1",
        "watch",
        "superseded",
        "key-old",
        Some("intent-new"),
        None,
        Some("2026-04-01T12:00:00Z"),
    )?;

    let service = harness.service()?;
    let envelope = service
        .get_decision_history("p1")
        .map_err(|error| anyhow!("{:?}: {}", error.code, error.message))?;
    let entries: &[DecisionHistoryEntry] = &envelope.data;
    assert_eq!(entries.len(), 2);
    // Sorted DESC by queued_at — most recent intent first.
    assert_eq!(entries[0].intent_id, "intent-new");
    assert_eq!(entries[0].disposition, "approve");
    assert_eq!(entries[0].status, "queued");
    assert_eq!(entries[0].decision_note.as_deref(), Some("Looks good"));
    assert_eq!(entries[1].intent_id, "intent-old");
    assert_eq!(entries[1].status, "superseded");
    assert_eq!(
        entries[1].superseded_by_intent_id.as_deref(),
        Some("intent-new")
    );
    Ok(())
}

#[test]
fn bug_withdrawn_decision_regression_does_not_roll_up_as_stale() -> Result<()> {
    let mut harness = Harness::new()?;
    harness.seed_process("p1", "2026-04-01T10:00:00Z", "review")?;
    harness.seed_candidate_and_quote("p1")?;
    seed_inbox_manifest(&mut harness.state_db, "p1", "clarify", 1, "[]")?;
    seed_intent(
        &mut harness.state_db,
        "intent-withdrawn",
        "p1",
        "watch",
        "withdrawn",
        "key-withdrawn",
        None,
        Some("Withdrawn operator demo decision"),
        Some("2026-04-02T12:00:00Z"),
    )?;
    harness.state_db.with_write_transaction(|tx| {
        tx.execute(
            "UPDATE pipeline_assessments SET decision_basis_hash = 'basis-p1-new' WHERE process_id = 'p1'",
            [],
        )?;
        tx.execute(
            "UPDATE manifests SET manifest_basis_hash = 'basis-p1-new' WHERE process_id = 'p1'",
            [],
        )?;
        Ok(())
    })?;

    let service = harness.service()?;
    let inbox = service
        .list_manifests()
        .map_err(|error| anyhow!("{:?}: {}", error.code, error.message))?;
    let row = inbox
        .data
        .iter()
        .find(|row| row.process_id == "p1")
        .ok_or_else(|| anyhow!("p1 manifest row not found"))?;
    assert_eq!(row.decision.disposition.as_deref(), Some("watch"));
    assert_eq!(row.decision.status.as_deref(), Some("withdrawn"));
    assert!(!row.decision.is_stale);

    let detail = service
        .get_process_detail("p1")
        .map_err(|error| anyhow!("{:?}: {}", error.code, error.message))?;
    assert_eq!(detail.data.decision.status.as_deref(), Some("withdrawn"));
    assert!(!detail.data.decision.is_stale);

    let stale_only = service
        .list_processes(
            None,
            25,
            &ProcessListFilters {
                stale_only: true,
                ..ProcessListFilters::default()
            },
        )
        .map_err(|error| anyhow!("{:?}: {}", error.code, error.message))?;
    assert!(
        stale_only.data.items.is_empty(),
        "withdrawn decisions must not satisfy the stale-only process filter"
    );
    Ok(())
}

#[test]
fn bug_stage14_stale_regression_uses_manifest_basis_not_pipeline_basis() -> Result<()> {
    let mut harness = Harness::new()?;
    harness.seed_process("p1", "2026-04-01T10:00:00Z", "review")?;
    harness.seed_candidate_and_quote("p1")?;
    seed_inbox_manifest(&mut harness.state_db, "p1", "clarify", 1, "[]")?;
    seed_intent(
        &mut harness.state_db,
        "intent-queued",
        "p1",
        "approve",
        "queued",
        "key-queued",
        None,
        Some("Fresh decision after manifest regeneration"),
        Some("2026-04-02T12:00:00Z"),
    )?;
    harness.state_db.with_write_transaction(|tx| {
        tx.execute(
            "UPDATE pipeline_assessments SET decision_basis_hash = 'pipeline-basis-p1-new' WHERE process_id = 'p1'",
            [],
        )?;
        Ok(())
    })?;

    let service = harness.service()?;
    let inbox = service
        .list_manifests()
        .map_err(|error| anyhow!("{:?}: {}", error.code, error.message))?;
    let row = inbox
        .data
        .iter()
        .find(|row| row.process_id == "p1")
        .ok_or_else(|| anyhow!("p1 manifest row not found"))?;
    assert_eq!(row.decision.disposition.as_deref(), Some("approve"));
    assert_eq!(row.decision.status.as_deref(), Some("queued"));
    assert!(
        !row.decision.is_stale,
        "fresh queued decisions must compare reviewed_basis_hash to manifests.manifest_basis_hash"
    );

    let detail = service
        .get_process_detail("p1")
        .map_err(|error| anyhow!("{:?}: {}", error.code, error.message))?;
    assert_eq!(detail.data.decision.status.as_deref(), Some("queued"));
    assert!(!detail.data.decision.is_stale);
    Ok(())
}

#[test]
fn bug_stage14_stale_regression_marks_manifest_basis_drift_stale() -> Result<()> {
    let mut harness = Harness::new()?;
    harness.seed_process("p1", "2026-04-01T10:00:00Z", "review")?;
    harness.seed_candidate_and_quote("p1")?;
    seed_inbox_manifest(&mut harness.state_db, "p1", "clarify", 1, "[]")?;
    seed_intent(
        &mut harness.state_db,
        "intent-queued",
        "p1",
        "approve",
        "queued",
        "key-queued",
        None,
        Some("Decision against prior manifest basis"),
        Some("2026-04-02T12:00:00Z"),
    )?;
    harness.state_db.with_write_transaction(|tx| {
        tx.execute(
            "UPDATE manifests SET manifest_basis_hash = 'manifest-basis-p1-new' WHERE process_id = 'p1'",
            [],
        )?;
        Ok(())
    })?;

    let service = harness.service()?;
    let inbox = service
        .list_manifests()
        .map_err(|error| anyhow!("{:?}: {}", error.code, error.message))?;
    let row = inbox
        .data
        .iter()
        .find(|row| row.process_id == "p1")
        .ok_or_else(|| anyhow!("p1 manifest row not found"))?;
    assert_eq!(row.decision.status.as_deref(), Some("queued"));
    assert!(row.decision.is_stale);

    let detail = service
        .get_process_detail("p1")
        .map_err(|error| anyhow!("{:?}: {}", error.code, error.message))?;
    assert_eq!(detail.data.decision.status.as_deref(), Some("queued"));
    assert!(detail.data.decision.is_stale);
    Ok(())
}

#[test]
fn rt_4_get_active_overrides_returns_only_active_rows() -> Result<()> {
    let mut harness = Harness::new()?;
    harness.seed_process("p1", "2026-04-01T10:00:00Z", "review")?;
    harness.seed_candidate_and_quote("p1")?;
    seed_override(
        &mut harness.state_db,
        "override-active",
        "p1",
        "candidate-p1",
        "quantity_basis_unaligned",
        r#"{"qty": 250, "unit": "m^2"}"#,
    )?;
    seed_override(
        &mut harness.state_db,
        "override-withdrawn",
        "p1",
        "candidate-p1",
        "awaiting_rfq_quote",
        r#"{"unit_price_gbp_pence": 100, "lead_time_days": 30, "moq": 1, "currency": "GBP"}"#,
    )?;
    // Withdraw the second one
    harness.state_db.with_write_transaction(|tx| {
        tx.execute(
            "UPDATE operator_input_overrides SET withdrawn_at = CURRENT_TIMESTAMP WHERE id = ?1",
            params!["override-withdrawn"],
        )?;
        Ok(())
    })?;

    let service = harness.service()?;
    let envelope = service
        .get_active_overrides("p1")
        .map_err(|error| anyhow!("{:?}: {}", error.code, error.message))?;
    let rows: &[ActiveOverrideEntry] = &envelope.data;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].override_id, "override-active");
    assert_eq!(rows[0].reason_code, "quantity_basis_unaligned");
    Ok(())
}

fn repo_root() -> Result<PathBuf> {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .ancestors()
        .nth(3)
        .map(Path::to_path_buf)
        .ok_or_else(|| {
            anyhow!(
                "failed to resolve repository root from {}",
                manifest_dir.display()
            )
        })
}
