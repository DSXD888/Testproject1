#![forbid(unsafe_code)]
#![allow(clippy::duplicate_mod)]

mod support;

#[path = "../src/bid_workflow/types.rs"]
#[allow(dead_code)]
mod types;

use anyhow::{Context, Result};
use rusqlite::{Connection, params};
use serde_json::Value;

use crate::support::db::{FTS_SOURCE_ID, TestStateDb, seed_process};
use crate::types::{
    BidScopePayload, BidScopeValidationError, BidWorkflowState, SideEffectClass,
    SqliteBidScopeReferenceResolver, bid_workflow_viewer_projection_columns,
    default_bid_scope_fields, validate_bid_scope, validate_bid_scope_shape,
};

const BASIS_HASH: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

fn valid_scope(process_id: &str) -> BidScopePayload {
    BidScopePayload {
        process_id: process_id.to_owned(),
        lot_ids: vec!["lot-1".to_owned()],
        candidate_ids: vec!["candidate-1".to_owned()],
        selected_quote_ids: vec!["quote-1".to_owned()],
        company_profile_version: "company-profile-v1".to_owned(),
        bid_portal_type: "procontract".to_owned(),
        portal_opportunity_ref: "DN123456".to_owned(),
        review_required: true,
        decision_basis_hash: BASIS_HASH.to_owned(),
        side_effect_cap: SideEffectClass::RemoteDraft,
    }
}

fn seed_valid_bid_scope_graph(conn: &Connection) -> Result<()> {
    seed_bid_scope_notice_graph(conn)?;
    seed_bid_scope_quote_graph(conn)?;
    seed_bid_scope_remote_tables(conn)?;
    Ok(())
}

fn seed_bid_scope_notice_graph(conn: &Connection) -> Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO sources (id, kind, enabled, config) VALUES (?1, 'procurement', 1, '{}')",
        params![FTS_SOURCE_ID],
    )?;
    seed_process(conn, "process-1")?;
    conn.execute(
        "\
        INSERT INTO contract_notices (
          id, process_id, source_id, source_notice_id, raw_document_id, title,
          procurement_stage, published_at, source_updated_at, normalized_payload, owner_id
        ) VALUES (
          'notice-1', 'process-1', ?1, 'source-notice-1', 'raw-document-1',
          'Scope validation notice', 'tender', '2026-05-22T00:00:00Z',
          '2026-05-22T00:00:00Z', '{}', 'local-owner'
        )
        ",
        params![FTS_SOURCE_ID],
    )?;
    conn.execute(
        "\
        INSERT INTO notice_lots (
          id, notice_id, lot_number, title, owner_id
        ) VALUES (
          'lot-1', 'notice-1', 1, 'Scope validation lot', 'local-owner'
        )
        ",
        [],
    )?;
    Ok(())
}

fn seed_bid_scope_quote_graph(conn: &Connection) -> Result<()> {
    conn.execute(
        "\
        INSERT INTO product_candidates (
          id, process_id, notice_id, lot_id, name, candidate_type,
          match_archetype, head_noun, owner_id
        ) VALUES (
          'candidate-1', 'process-1', 'notice-1', 'lot-1', 'Test product',
          'candidate', 'direct', 'product', 'local-owner'
        )
        ",
        [],
    )?;
    conn.execute(
        "\
        INSERT INTO supplier_quotes (
          id, process_id, notice_id, product_candidate_id, product_name, region,
          quote_class, source_name, unit_price_original_minor, currency,
          unit_price_gbp_pence, price_signal_role, evidence, owner_id
        ) VALUES (
          'quote-1', 'process-1', 'notice-1', 'candidate-1', 'Test product',
          'UK', 'marketplace', 'fixture', 1000, 'GBP', 1000,
          'normal_market_quote', '{}', 'local-owner'
        )
        ",
        [],
    )?;
    conn.execute(
        "\
        INSERT INTO pipeline_assessments (
          process_id, pipeline_state, decision_basis_hash, owner_id
        ) VALUES (
          'process-1', 'net_margin_passed', ?1, 'local-owner'
        )
        ",
        params![BASIS_HASH],
    )?;
    Ok(())
}

fn seed_bid_scope_remote_tables(conn: &Connection) -> Result<()> {
    conn.execute(
        "\
        CREATE TEMP TABLE operator_decision_heads (
          process_id TEXT PRIMARY KEY,
          disposition TEXT NOT NULL,
          decision_stale INTEGER NOT NULL DEFAULT 0,
          deleted_at TEXT
        )
        ",
        [],
    )?;
    conn.execute(
        "\
        INSERT INTO operator_decision_heads (
          process_id, disposition, decision_stale, deleted_at
        ) VALUES (
          'process-1', 'approved', 0, NULL
        )
        ",
        [],
    )?;
    conn.execute(
        "\
        CREATE TEMP TABLE shared_company_profile (
          id TEXT PRIMARY KEY,
          remote_revision INTEGER NOT NULL,
          deleted_at TEXT
        )
        ",
        [],
    )?;
    conn.execute(
        "\
        INSERT INTO shared_company_profile (
          id, remote_revision, deleted_at
        ) VALUES (
          'company-profile-v1', 1, NULL
        )
        ",
        [],
    )?;
    Ok(())
}

fn validate_with_sqlite(
    conn: &Connection,
    scope: &BidScopePayload,
) -> std::result::Result<(), BidScopeValidationError> {
    let references = SqliteBidScopeReferenceResolver::new(conn);
    validate_bid_scope(scope, &references)
}

#[test]
#[doc = "L6-T-bid_workflow_states-030"]
fn l6_t_bid_workflow_states_030_accepts_valid_scope_references() -> Result<()> {
    let db = TestStateDb::new()?;
    let conn = db.connection();
    seed_valid_bid_scope_graph(conn)?;

    validate_with_sqlite(conn, &valid_scope("process-1"))
        .context("valid bid scope should pass shape and ownership validation")?;
    Ok(())
}

#[test]
#[doc = "L6-T-bid_workflow_states-030"]
fn l6_t_bid_workflow_states_030_rejects_missing_required_shape_fields() {
    let mut empty_process = valid_scope("");
    empty_process.process_id.clear();
    let mut empty_lots = valid_scope("process-1");
    empty_lots.lot_ids.clear();
    let mut empty_candidate = valid_scope("process-1");
    empty_candidate.candidate_ids = vec![" ".to_owned()];
    let mut empty_quotes = valid_scope("process-1");
    empty_quotes.selected_quote_ids.clear();
    let mut empty_profile = valid_scope("process-1");
    empty_profile.company_profile_version.clear();
    let mut unsupported_portal = valid_scope("process-1");
    unsupported_portal.bid_portal_type = "proactis".to_owned();
    let mut empty_ref = valid_scope("process-1");
    empty_ref.portal_opportunity_ref.clear();
    let mut invalid_hash = valid_scope("process-1");
    invalid_hash.decision_basis_hash = "not-a-sha256".to_owned();

    let cases = [
        (empty_process, "BID_SCOPE_EMPTY_FIELD"),
        (empty_lots, "BID_SCOPE_EMPTY_LIST"),
        (empty_candidate, "BID_SCOPE_EMPTY_LIST"),
        (empty_quotes, "BID_SCOPE_EMPTY_LIST"),
        (empty_profile, "BID_SCOPE_EMPTY_FIELD"),
        (unsupported_portal, "BID_SCOPE_UNSUPPORTED_PORTAL"),
        (empty_ref, "BID_SCOPE_EMPTY_FIELD"),
        (invalid_hash, "BID_SCOPE_INVALID_DECISION_BASIS_HASH"),
    ];

    for (scope, expected_code) in cases {
        let error = validate_bid_scope_shape(&scope).expect_err("invalid shape rejected");

        assert_eq!(error.code(), expected_code);
    }
}

#[test]
#[doc = "L6-T-bid_workflow_states-030"]
fn l6_t_bid_workflow_states_030_rejects_invalid_reference_chain() -> Result<()> {
    let db = TestStateDb::new()?;
    let conn = db.connection();
    seed_valid_bid_scope_graph(conn)?;

    let missing_process = valid_scope("missing-process");
    assert_eq!(
        validate_with_sqlite(conn, &missing_process)
            .expect_err("missing process rejected")
            .code(),
        "BID_SCOPE_PROCESS_NOT_FOUND"
    );

    conn.execute(
        "UPDATE operator_decision_heads SET disposition = 'pending' WHERE process_id = 'process-1'",
        [],
    )?;
    assert_eq!(
        validate_with_sqlite(conn, &valid_scope("process-1"))
            .expect_err("unapproved decision rejected")
            .code(),
        "BID_SCOPE_OPERATOR_DECISION_NOT_APPROVED"
    );
    conn.execute(
        "UPDATE operator_decision_heads SET disposition = 'approved' WHERE process_id = 'process-1'",
        [],
    )?;

    conn.execute(
        "DELETE FROM pipeline_assessments WHERE process_id = 'process-1'",
        [],
    )?;
    assert_eq!(
        validate_with_sqlite(conn, &valid_scope("process-1"))
            .expect_err("missing rollup rejected")
            .code(),
        "BID_SCOPE_PIPELINE_ROLLUP_MISSING"
    );
    conn.execute(
        "\
        INSERT INTO pipeline_assessments (
          process_id, pipeline_state, decision_basis_hash, owner_id
        ) VALUES ('process-1', 'net_margin_passed', ?1, 'local-owner')
        ",
        params![BASIS_HASH],
    )?;

    let mut wrong_lot = valid_scope("process-1");
    wrong_lot.lot_ids = vec!["missing-lot".to_owned()];
    assert_eq!(
        validate_with_sqlite(conn, &wrong_lot)
            .expect_err("lot ownership rejected")
            .code(),
        "BID_SCOPE_LOT_NOT_OWNED_BY_PROCESS"
    );

    let mut wrong_candidate = valid_scope("process-1");
    wrong_candidate.candidate_ids = vec!["missing-candidate".to_owned()];
    assert_eq!(
        validate_with_sqlite(conn, &wrong_candidate)
            .expect_err("candidate ownership rejected")
            .code(),
        "BID_SCOPE_CANDIDATE_NOT_OWNED_BY_LOTS"
    );

    let mut wrong_quote = valid_scope("process-1");
    wrong_quote.selected_quote_ids = vec!["missing-quote".to_owned()];
    assert_eq!(
        validate_with_sqlite(conn, &wrong_quote)
            .expect_err("quote ownership rejected")
            .code(),
        "BID_SCOPE_QUOTE_NOT_OWNED_BY_CANDIDATES"
    );

    let mut wrong_profile = valid_scope("process-1");
    wrong_profile.company_profile_version = "missing-profile".to_owned();
    assert_eq!(
        validate_with_sqlite(conn, &wrong_profile)
            .expect_err("company profile rejected")
            .code(),
        "BID_SCOPE_COMPANY_PROFILE_NOT_FOUND"
    );
    Ok(())
}

#[test]
#[doc = "L6-T-bid_workflow_states-032"]
fn l6_t_bid_workflow_states_032_defaults_to_operator_review_and_remote_draft_cap() {
    let (review_required, side_effect_cap) = default_bid_scope_fields();

    assert!(review_required);
    assert_eq!(side_effect_cap, SideEffectClass::RemoteDraft);
    assert_eq!(BidScopePayload::default_review_required(), review_required);
    assert_eq!(BidScopePayload::default_side_effect_cap(), side_effect_cap);
}

#[test]
#[doc = "L6-T-bid_workflow_states-033"]
fn l6_t_bid_workflow_states_033_persists_scope_json_without_viewer_projection() -> Result<()> {
    let db = TestStateDb::new()?;
    let conn = db.connection();
    let persisted_states = [
        BidWorkflowState::BidInProgress,
        BidWorkflowState::DraftSaved,
        BidWorkflowState::BidSubmitted,
        BidWorkflowState::BidFailed,
        BidWorkflowState::PortalStateUnknown,
    ];

    for state in persisted_states {
        let process_id = format!("scope-json-{}", state.as_str());
        seed_process(conn, &process_id)?;
        let mut scope = valid_scope(&process_id);
        scope.portal_opportunity_ref = format!("DN-{}", state.as_str());
        let scope_json = serde_json::to_string(&scope)?;
        conn.execute(
            "\
            INSERT INTO bid_workflow_states (
              process_id, bid_state, bid_scope_json, review_required, owner_id
            ) VALUES (
              ?1, ?2, ?3, 1, 'local-owner'
            )
            ",
            params![process_id, state.as_str(), scope_json],
        )?;

        let stored_json: String = conn.query_row(
            "SELECT bid_scope_json FROM bid_workflow_states WHERE process_id = ?1",
            params![process_id],
            |row| row.get(0),
        )?;
        let stored: Value = serde_json::from_str(&stored_json)?;
        assert_eq!(
            stored["portalOpportunityRef"],
            format!("DN-{}", state.as_str())
        );
        assert_eq!(stored["sideEffectCap"], "REMOTE_DRAFT");
    }

    assert!(
        !bid_workflow_viewer_projection_columns().contains(&"bid_scope_json"),
        "viewer projection columns must not expose raw BidScopePayload"
    );
    assert!(
        bid_workflow_viewer_projection_columns()
            .iter()
            .all(|column| !column.contains("scope") && !column.contains("manifest")),
        "viewer projection columns must stay summary-only"
    );
    Ok(())
}
