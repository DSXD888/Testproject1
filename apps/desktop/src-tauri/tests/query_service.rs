#![forbid(unsafe_code)]

#[path = "../src/query_service.rs"]
#[allow(dead_code)]
mod query_service_impl;
#[path = "../../../../crates/daemon/src/db/sqlite.rs"]
#[allow(dead_code)]
mod sqlite_impl;

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use query_service_impl::{
    ErrorListFilters, ProcessListFilters, QueryService, QueryServiceErrorCode,
    SessionHistoryAvailability, SessionHistoryRequest, SessionListFilters,
};
use rusqlite::params;
use sqlite_impl::{DEFAULT_SCHEMA_EPOCH, StateDb, StateDbConfig};

type TestResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;
static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(1);

struct TestDb {
    root: PathBuf,
    db_path: PathBuf,
}

impl TestDb {
    fn new() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |duration| duration.as_nanos());
        let suffix = NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed);
        let root =
            PathBuf::from("/tmp").join(format!("qs-{}-{unique}-{suffix}", std::process::id()));
        fs::create_dir_all(&root)?;
        let db_path = root.join("state.db");
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../..")
            .canonicalize()?;
        let state_db = StateDb::open_in_repo(
            &db_path,
            &repo_root,
            StateDbConfig {
                schema_epoch: DEFAULT_SCHEMA_EPOCH,
                ..StateDbConfig::default()
            },
        )?;
        seed_fixture(state_db.connection(), &root)?;
        drop(state_db);
        Ok(Self { root, db_path })
    }
}

impl Drop for TestDb {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn seed_fixture(conn: &rusqlite::Connection, root: &std::path::Path) -> TestResult {
    conn.execute(
        "INSERT INTO procurement_processes (id, canonical_process_id, current_stage, buyer_name, title, latest_deadline_at, latest_published_at, latest_updated_at, owner_id) VALUES (?1, NULL, 'review', 'Buyer 1', 'Medical gloves', '2026-04-01T10:00:00Z', '2026-03-28T09:00:00Z', '2026-03-28T09:00:00Z', 'local-owner')",
        params!["proc-1"],
    )?;
    conn.execute(
        "INSERT INTO procurement_processes (id, canonical_process_id, current_stage, buyer_name, title, latest_deadline_at, latest_published_at, latest_updated_at, owner_id) VALUES (?1, NULL, 'pending', 'Buyer 2', 'Office chairs', '2026-04-02T10:00:00Z', '2026-03-27T09:00:00Z', '2026-03-27T09:00:00Z', 'local-owner')",
        params!["proc-2"],
    )?;
    conn.execute(
        "INSERT INTO process_aliases (alias_process_id, canonical_process_id, owner_id) VALUES (?1, ?2, 'local-owner')",
        params!["proc-alias", "proc-1"],
    )?;
    conn.execute(
        "INSERT INTO contract_notices (id, process_id, source_id, source_notice_id, raw_document_id, ocid, title, summary, buyer_name, procurement_stage, access_route, deadline_at, published_at, source_updated_at, normalized_payload, owner_id) VALUES (?1, ?2, 'fts', 'notice-1', 'raw-1', 'ocid-1', 'Medical gloves', 'Summary', 'Buyer 1', 'tender', 'https://example.test/notice/1', '2026-04-01T10:00:00Z', '2026-03-28T09:00:00Z', '2026-03-28T09:00:00Z', '{}', 'local-owner')",
        params!["notice-1", "proc-1"],
    )?;
    conn.execute(
        "INSERT INTO notice_lots (id, notice_id, lot_key, lot_number, title, description, estimated_value_pence, deadline_at, is_bidable_separately, documents, owner_id) VALUES (?1, ?2, 'lot-key-1', 1, 'Lot 1', 'Core PPE', 250000, '2026-04-01T10:00:00Z', 1, '[]', 'local-owner')",
        params!["lot-1", "notice-1"],
    )?;
    conn.execute(
        "INSERT INTO product_candidates (id, process_id, notice_id, lot_id, name, candidate_type, match_archetype, head_noun, quoteability_score_bps, candidate_value_share_bps, owner_id) VALUES (?1, ?2, ?3, ?4, 'Disposable gloves', 'product', 'standard_class_led', 'gloves', 8100, 9000, 'local-owner')",
        params!["cand-1", "proc-1", "notice-1", "lot-1"],
    )?;
    conn.execute(
        "INSERT INTO supplier_quotes (id, process_id, notice_id, product_candidate_id, product_name, region, quote_class, source_name, source_url, unit_price_original_minor, currency, unit_price_gbp_pence, price_signal_role, evidence, owner_id) VALUES (?1, ?2, ?3, ?4, 'Disposable gloves', 'UK', 'list', 'Supplier A', 'https://example.test/quote/1', 1200, 'GBP', 1200, 'primary', '{}', 'local-owner')",
        params!["quote-1", "proc-1", "notice-1", "cand-1"],
    )?;
    conn.execute(
        "INSERT INTO landed_cost_estimates (id, process_id, notice_id, product_candidate_id, supplier_quote_id, shipping_mode, mode_confidence_bps, commodity_confidence_bps, classification_stability_bps, customs_valuation_method, valuation_mode, customs_fx_rate, commercial_fx_rate, fx_basis_delta_bps, supplier_cost_gbp_pence, freight_low_gbp_pence, freight_base_gbp_pence, freight_high_gbp_pence, customs_duty_gbp_pence, import_vat_gbp_pence, vat_rate_state, import_vat_treatment, uk_final_mile_cost_gbp_pence, other_fees_gbp_pence, economic_landed_cost_gbp_pence, cash_outlay_gbp_pence, cash_to_clear_gbp_pence, revenue_basis_type, revenue_confidence_bps, net_margin_low_bps, net_margin_base_bps, net_margin_high_bps, decision_grade, confidence_bps, warnings, owner_id) VALUES (?1, ?2, ?3, ?4, ?5, 'air', 7000, 7200, 7100, 'cif', 'estimate', 1.0, 1.0, 0, 1200, 100, 120, 150, 50, 60, 'standard', 'payable', 30, 20, 1480, 1510, 200, 'list_price', 8000, 1800, 2000, 2200, 'screening', 7600, '[]', 'local-owner')",
        params!["landed-1", "proc-1", "notice-1", "cand-1", "quote-1"],
    )?;
    conn.execute(
        "INSERT INTO manifests (id, process_id, markdown, artifact_hash, artifact_locator, owner_id) VALUES (?1, ?2, '# Summary\\n## Risks\\nText', 'artifact-hash-1', '/artifacts/manifest.md', 'local-owner')",
        params!["manifest-1", "proc-1"],
    )?;
    conn.execute(
        "INSERT INTO pipeline_assessments (process_id, pipeline_state, gross_margin_bps, net_margin_bps, priced_coverage_bps, landed_cost_coverage_bps, net_qualified_coverage_bps, priced_value_coverage_bps, landed_cost_value_coverage_bps, net_qualified_value_coverage_bps, unresolved_mandatory_lots, decision_basis_hash, owner_id) VALUES (?1, 'review', 2500, 2000, 8000, 7500, 7000, 8200, 7600, 7100, 0, 'basis-1', 'local-owner')",
        params!["proc-1"],
    )?;
    conn.execute(
        "INSERT INTO pending_operator_intents (id, process_id, desired_disposition, reviewed_basis_hash, reviewed_manifest_hash, reviewed_at_stage_run_id, reviewed_at, idempotency_key, queued_at) VALUES (?1, ?2, 'watch', 'basis-0', 'manifest-hash-1', 'stage-1', '2026-03-28T09:30:00Z', 'idem-1', '2026-03-28T09:30:00Z')",
        params!["intent-1", "proc-1"],
    )?;
    conn.execute(
        "INSERT INTO agent_session_logs (id, job_id, skill_path, skill_version, execution_mode, started_at, completed_at, status, terminal_output_path, terminal_output_bytes, structured_trace, error_type, exit_code, cleanup_verdict, last_checkpoint_seq) VALUES (?1, NULL, 'skills/procurement-ingestion/fts/SKILL.md', '1.0.0', 'pty', '2026-03-28T09:00:00Z', '2026-03-28T09:05:00Z', 'completed', ?2, 18, ?3, NULL, 0, 'clean', 2)",
        params![
            "session-1",
            root.join("session-1").join("stream.index.jsonl").display().to_string(),
            serde_json::json!({
                "phase_history": ["created", "spawned", "active", "terminating", "exited", "reaped", "archived"],
                "daemon_instance_id": "daemon-1"
            }).to_string()
        ],
    )?;
    conn.execute(
        "INSERT INTO error_manifests (id, job_id, process_id, pipeline_stage, workflow_step, error_type, error_message, error_context, skill_path, agent_session_id, markdown_path, resolved, created_at) VALUES (?1, NULL, ?2, 'qualify', 'lot_review', 'AGENT_CLEANUP_INCOMPLETE', 'cleanup issue', '{}', 'skills/procurement-ingestion/fts/SKILL.md', 'session-1', '/artifacts/error.md', 0, '2026-03-28T09:10:00Z')",
        params!["error-1", "proc-1"],
    )?;

    let session_dir = root.join("session-1").join("stream");
    fs::create_dir_all(&session_dir)?;
    let chunk_path = session_dir.join("000001.pty.zst");
    fs::write(&chunk_path, zstd::stream::encode_all(&b"hello\n"[..], 0)?)?;
    fs::write(
        root.join("session-1").join("stream.index.jsonl"),
        format!(
            "{}\n",
            serde_json::json!({
                "chunk": "stream/000001.pty.zst",
                "first_seq": 1,
                "last_seq": 1,
                "byte_length": 10,
                "wall_ts_start": "2026-03-28T09:00:01Z",
                "wall_ts_end": "2026-03-28T09:00:01Z",
                "sha256": "abc"
            })
        ),
    )?;
    Ok(())
}

#[test]
fn rs_qs_001_query_service_is_read_only_and_reports_query_only() -> TestResult {
    let db = TestDb::new()?;
    let service = QueryService::open(&db.db_path)?;
    let query_only: i64 = service
        .connection()
        .query_row("PRAGMA query_only", [], |row| row.get(0))?;
    assert_eq!(query_only, 1);
    Ok(())
}

#[test]
fn rs_qs_002_snapshot_seq_comes_from_commit_clock() -> TestResult {
    let db = TestDb::new()?;
    let mut service = QueryService::open(&db.db_path)?;
    let envelope = service.list_processes(None, 25, &ProcessListFilters::default())?;
    assert_eq!(envelope.snapshot_seq, 0);
    Ok(())
}

#[test]
fn rs_qs_003_list_processes_returns_a_stable_keyset_page() -> TestResult {
    let db = TestDb::new()?;
    let mut service = QueryService::open(&db.db_path)?;
    let envelope = service.list_processes(None, 1, &ProcessListFilters::default())?;
    assert_eq!(envelope.data.items.len(), 1);
    assert!(envelope.data.has_more);
    assert!(envelope.data.next_cursor.is_some());
    assert_eq!(envelope.data.items[0].process_id, "proc-1");
    Ok(())
}

#[test]
fn rs_qs_004_get_process_detail_resolves_aliases_and_returns_complete_vm() -> TestResult {
    let db = TestDb::new()?;
    let mut service = QueryService::open(&db.db_path)?;
    let envelope = service.get_process_detail("proc-alias")?;
    assert_eq!(envelope.data.process_id, "proc-1");
    assert_eq!(envelope.data.canonical_process_id, "proc-1");
    assert_eq!(envelope.data.notices.len(), 1);
    assert_eq!(envelope.data.lots.len(), 1);
    assert_eq!(envelope.data.candidates.len(), 1);
    assert_eq!(envelope.data.quotes.len(), 1);
    assert_eq!(envelope.data.landed_costs.len(), 1);
    assert!(envelope.data.manifest.is_some());
    assert!(
        envelope
            .data
            .manifest
            .as_ref()
            .is_some_and(|manifest| !manifest.sections.is_empty())
    );
    Ok(())
}

#[test]
fn rs_qs_005_list_sessions_returns_canonical_metadata() -> TestResult {
    let db = TestDb::new()?;
    let mut service = QueryService::open(&db.db_path)?;
    let envelope = service.list_sessions(None, 25, &SessionListFilters::default())?;
    assert_eq!(envelope.data.items.len(), 1);
    let item = &envelope.data.items[0];
    assert_eq!(item.session_id, "session-1");
    assert_eq!(item.daemon_instance_id, "daemon-1");
    assert_eq!(item.phase, "archived");
    assert_eq!(item.checkpoint_count, 2);
    assert_eq!(item.byte_count, 18);
    assert!(!item.reattachable);
    Ok(())
}

#[test]
fn rs_qs_006_list_errors_derives_severity_and_defaults_to_unresolved() -> TestResult {
    let db = TestDb::new()?;
    let mut service = QueryService::open(&db.db_path)?;
    let envelope = service.list_errors(None, 25, &ErrorListFilters::default())?;
    assert_eq!(envelope.data.items.len(), 1);
    assert_eq!(envelope.data.items[0].severity, "critical");
    assert!(!envelope.data.items[0].resolved);
    Ok(())
}

#[test]
fn rs_qs_007_invalid_cursor_surfaces_typed_error() -> TestResult {
    let db = TestDb::new()?;
    let mut service = QueryService::open(&db.db_path)?;
    let error = service
        .list_processes(Some("not-a-cursor"), 25, &ProcessListFilters::default())
        .expect_err("invalid cursor must fail");
    assert_eq!(error.code, QueryServiceErrorCode::InvalidCursor);
    Ok(())
}

#[test]
fn rs_qs_008_invalid_filter_surfaces_typed_error_without_partial_results() -> TestResult {
    let db = TestDb::new()?;
    let mut service = QueryService::open(&db.db_path)?;
    let error = service
        .list_processes(
            None,
            25,
            &ProcessListFilters {
                source_ids: vec![String::new()],
                ..ProcessListFilters::default()
            },
        )
        .expect_err("blank source id must fail");
    assert_eq!(error.code, QueryServiceErrorCode::InvalidFilter);
    Ok(())
}

#[test]
fn rs_qs_009_get_process_detail_reports_not_found() -> TestResult {
    let db = TestDb::new()?;
    let mut service = QueryService::open(&db.db_path)?;
    let error = service
        .get_process_detail("missing")
        .expect_err("missing process should fail");
    assert_eq!(error.code, QueryServiceErrorCode::NotFound);
    Ok(())
}

#[test]
fn rs_qs_010_session_history_reports_complete_ranges() -> TestResult {
    let db = TestDb::new()?;
    let mut service = QueryService::open(&db.db_path)?;
    let history = service.get_session_history(&SessionHistoryRequest {
        session_id: "session-1".to_owned(),
        from_seq: 1,
        to_seq: 1,
        max_bytes: None,
    })?;
    assert_eq!(
        history.data.availability,
        SessionHistoryAvailability::Complete
    );
    assert_eq!(history.data.frames.len(), 1);
    Ok(())
}

#[test]
fn rs_qs_011_session_history_reports_missing_ranges_without_faking_frames() -> TestResult {
    let db = TestDb::new()?;
    let mut service = QueryService::open(&db.db_path)?;
    let history = service.get_session_history(&SessionHistoryRequest {
        session_id: "session-1".to_owned(),
        from_seq: 2,
        to_seq: 3,
        max_bytes: None,
    })?;
    assert_eq!(
        history.data.availability,
        SessionHistoryAvailability::Missing
    );
    assert!(history.data.frames.is_empty());
    Ok(())
}

#[test]
fn rs_qs_012_session_history_missing_artifact_is_typed_artifact_unavailable() -> TestResult {
    let db = TestDb::new()?;
    let broken_path = db.root.join("missing.index.jsonl").display().to_string();
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()?;
    let conn = StateDb::open_in_repo(
        &db.db_path,
        &repo_root,
        StateDbConfig {
            schema_epoch: DEFAULT_SCHEMA_EPOCH,
            ..StateDbConfig::default()
        },
    )?;
    conn.connection().execute(
        "INSERT INTO agent_session_logs (id, skill_path, execution_mode, status, terminal_output_path) VALUES ('session-missing', 'skills/example/SKILL.md', 'pty', 'completed', ?1)",
        params![broken_path],
    )?;
    drop(conn);

    let mut service = QueryService::open(&db.db_path)?;
    let error = service
        .get_session_history(&SessionHistoryRequest {
            session_id: "session-missing".to_owned(),
            from_seq: 1,
            to_seq: 1,
            max_bytes: None,
        })
        .expect_err("missing artifact should fail");
    assert_eq!(error.code, QueryServiceErrorCode::ArtifactUnavailable);
    Ok(())
}
