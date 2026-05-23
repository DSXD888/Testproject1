#![forbid(unsafe_code)]
#![allow(clippy::duplicate_mod)]

mod support;

#[path = "../src/bid_workflow/types.rs"]
#[allow(dead_code)]
mod types;

#[path = "../src/bid_workflow/decision_basis_hash.rs"]
#[allow(dead_code)]
mod decision_basis_hash;

#[path = "../src/bid_workflow/readiness_gate.rs"]
#[allow(dead_code)]
mod readiness_gate;

use anyhow::Result;
use rusqlite::{Connection, params};
use serde_json::json;

use crate::readiness_gate::{
    BidReadinessBlocker, NON_OVERRIDEABLE_GATE_OVERRIDE_REASON_CODE,
    QUOTE_ROLE_QUALITY_OVERRIDE_REASON_CODE, TRUST_DEBT_ACCEPTANCE_REASON_CODE,
    evaluate_bid_readiness,
};
use crate::support::db::{FTS_SOURCE_ID, TestStateDb, seed_process};
use crate::types::{BidScopePayload, SideEffectClass};

const PROCESS_ID: &str = "process-readiness";
const NOTICE_ID: &str = "notice-readiness";
const LOT_ID: &str = "lot-readiness";
const CANDIDATE_ID: &str = "candidate-readiness";
const QUOTE_ID: &str = "quote-readiness";
const LCE_ID: &str = "lce-readiness";
const PROFILE_ID: &str = "company-profile-readiness";

fn base_scope() -> BidScopePayload {
    BidScopePayload {
        process_id: PROCESS_ID.to_owned(),
        lot_ids: vec![LOT_ID.to_owned()],
        candidate_ids: vec![CANDIDATE_ID.to_owned()],
        selected_quote_ids: vec![QUOTE_ID.to_owned()],
        company_profile_version: PROFILE_ID.to_owned(),
        bid_portal_type: "procontract".to_owned(),
        portal_opportunity_ref: "DN-READINESS-001".to_owned(),
        review_required: true,
        decision_basis_hash: "0".repeat(64),
        side_effect_cap: SideEffectClass::RemoteDraft,
    }
}

fn seeded_ready_scope() -> Result<(TestStateDb, BidScopePayload)> {
    let db = TestStateDb::new()?;
    let conn = db.connection();
    seed_readiness_graph(conn)?;
    seed_remote_projection_tables(conn)?;
    let mut scope = base_scope();
    let current_hash = decision_basis_hash::compute(conn, &scope)?;
    scope.decision_basis_hash.clone_from(&current_hash);
    seed_operator_decision_head(conn, "approved", Some(&current_hash), false)?;
    Ok((db, scope))
}

fn seed_readiness_graph(conn: &Connection) -> Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO sources (id, kind, enabled, config) VALUES (?1, 'procurement', 1, '{}')",
        params![FTS_SOURCE_ID],
    )?;
    seed_process(conn, PROCESS_ID)?;
    conn.execute(
        "\
        INSERT INTO contract_notices (
          id, process_id, source_id, source_notice_id, raw_document_id, title,
          procurement_stage, published_at, source_updated_at, normalized_payload, owner_id
        ) VALUES (
          ?1, ?2, ?3, 'source-notice-readiness', 'raw-document-readiness',
          'Readiness notice', 'tender', '2026-05-22T00:00:00Z',
          '2026-05-22T00:00:00Z', '{}', 'local-owner'
        )
        ",
        params![NOTICE_ID, PROCESS_ID, FTS_SOURCE_ID],
    )?;
    conn.execute(
        "\
        INSERT INTO notice_lots (
          id, notice_id, lot_number, title, owner_id
        ) VALUES (
          ?1, ?2, 1, 'Readiness lot', 'local-owner'
        )
        ",
        params![LOT_ID, NOTICE_ID],
    )?;
    conn.execute(
        "\
        INSERT INTO pipeline_assessments (
          process_id, pipeline_state, validation_state, decision_basis_hash, owner_id
        ) VALUES (
          ?1, 'net_margin_passed', 'validated', 'pipeline-basis-readiness-v1', 'local-owner'
        )
        ",
        params![PROCESS_ID],
    )?;
    seed_candidate_quote_and_landed_cost(conn)?;
    Ok(())
}

fn seed_candidate_quote_and_landed_cost(conn: &Connection) -> Result<()> {
    conn.execute(
        "\
        INSERT INTO product_candidates (
          id, process_id, notice_id, lot_id, name, candidate_type,
          match_archetype, head_noun, owner_id
        ) VALUES (
          ?1, ?2, ?3, ?4, 'Readiness product',
          'candidate', 'direct', 'widget', 'local-owner'
        )
        ",
        params![CANDIDATE_ID, PROCESS_ID, NOTICE_ID, LOT_ID],
    )?;
    conn.execute(
        "\
        INSERT INTO supplier_quotes (
          id, process_id, notice_id, product_candidate_id, product_name, region,
          quote_class, source_name, unit_price_original_minor, currency,
          unit_price_gbp_pence, price_signal_role, evidence, owner_id
        ) VALUES (
          ?1, ?2, ?3, ?4, 'Readiness product', 'UK', 'marketplace',
          'fixture', 1000, 'GBP', 1000, 'normal_market_quote', '{}', 'local-owner'
        )
        ",
        params![QUOTE_ID, PROCESS_ID, NOTICE_ID, CANDIDATE_ID],
    )?;
    conn.execute(
        "\
        INSERT INTO landed_cost_estimates (
          id, process_id, notice_id, product_candidate_id, supplier_quote_id,
          shipping_mode, mode_confidence_bps, declarable_commodity_code_10,
          commodity_confidence_bps, classification_stability_bps,
          customs_fx_rate, commercial_fx_rate, fx_basis_delta_bps, fx_rate_checked_at,
          supplier_cost_gbp_pence, freight_low_gbp_pence, freight_base_gbp_pence,
          freight_high_gbp_pence, customs_duty_gbp_pence, import_vat_gbp_pence,
          import_vat_treatment, uk_final_mile_cost_gbp_pence, other_fees_gbp_pence,
          economic_landed_cost_gbp_pence, cash_outlay_gbp_pence, cash_to_clear_gbp_pence,
          revenue_basis_type, revenue_confidence_bps, net_margin_low_bps,
          net_margin_base_bps, net_margin_high_bps, decision_grade, confidence_bps,
          warnings, blockers, tariff_checked_at, measures_fresh_until,
          classification_fresh_until, trade_remedy_watch_state, trade_remedy_cases_json,
          duty_breakdown_json, measure_interaction_json, owner_id
        ) VALUES (
          ?1, ?2, ?3, ?4, ?5,
          'parcel', 9000, '1234567890',
          9000, 9000,
          1.0, 1.0, 0, '2026-05-22T00:00:00Z',
          1000, 100, 125,
          150, 50, 235,
          'recoverable', 25, 10,
          1210, 1210, 1210,
          'notice_lot_value', 9000, 2000,
          3000, 4000, 'ready_for_bid', 9000,
          '[]', '[]', '2026-05-22T00:00:00Z', '2099-01-01T00:00:00Z',
          '2099-01-01T00:00:00Z', 'none', '[]',
          '[]', '[]', 'local-owner'
        )
        ",
        params![LCE_ID, PROCESS_ID, NOTICE_ID, CANDIDATE_ID, QUOTE_ID],
    )?;
    Ok(())
}

fn seed_remote_projection_tables(conn: &Connection) -> Result<()> {
    conn.execute(
        "\
        CREATE TEMP TABLE operator_decision_heads (
          id TEXT PRIMARY KEY,
          process_id TEXT NOT NULL UNIQUE,
          disposition TEXT NOT NULL,
          reviewed_basis_hash TEXT,
          decision_stale INTEGER NOT NULL DEFAULT 0,
          deleted_at TEXT
        )
        ",
        [],
    )?;
    conn.execute(
        "\
        CREATE TEMP TABLE shared_company_profile (
          id TEXT PRIMARY KEY,
          company_name TEXT,
          registration_number TEXT,
          vat_number TEXT,
          address_json TEXT,
          contact_json TEXT,
          bid_profile_json TEXT,
          remote_revision INTEGER NOT NULL,
          deleted_at TEXT
        )
        ",
        [],
    )?;
    seed_company_profile(conn, "2099-01-01", "2099-01-01")?;
    Ok(())
}

fn seed_company_profile(
    conn: &Connection,
    certification_expiry: &str,
    insurance_expiry: &str,
) -> Result<()> {
    let address = json!({
        "line1": "1 Readiness Street",
        "line2": null,
        "city": "London",
        "county": null,
        "postcode": "SW1A 1AA",
        "country": "GB"
    });
    let contact = json!({
        "name": "Procurement Lead",
        "email": "procurement@example.com",
        "phone": "+442070000000",
        "role": "Bid manager"
    });
    let bid_profile = json!({
        "certifications": [{
            "type": "iso9001",
            "number": "ISO-9001-READINESS",
            "expiryDate": certification_expiry,
            "issuingBody": "UKAS"
        }],
        "insurancePolicies": [{
            "type": "public_liability",
            "provider": "Readiness Mutual",
            "policyNumber": "PL-READINESS",
            "coverAmountPence": 500_000_000_i64,
            "expiryDate": insurance_expiry
        }],
        "financialStatements": {
            "lastFiledYear": 2025,
            "companiesHouseRef": "CH-READINESS"
        },
        "smeStatus": "small",
        "equalityStatement": "Current",
        "environmentalPolicy": "Current"
    });
    conn.execute(
        "\
        INSERT INTO shared_company_profile (
          id, company_name, registration_number, vat_number, address_json,
          contact_json, bid_profile_json, remote_revision, deleted_at
        ) VALUES (
          ?1, 'Sorelon Readiness Ltd', '12345678', 'GB123456789', ?2, ?3, ?4, 1, NULL
        )
        ON CONFLICT(id) DO UPDATE SET
          bid_profile_json = excluded.bid_profile_json,
          deleted_at = excluded.deleted_at
        ",
        params![
            PROFILE_ID,
            address.to_string(),
            contact.to_string(),
            bid_profile.to_string()
        ],
    )?;
    Ok(())
}

fn seed_operator_decision_head(
    conn: &Connection,
    disposition: &str,
    reviewed_basis_hash: Option<&str>,
    decision_stale: bool,
) -> Result<()> {
    conn.execute(
        "\
        INSERT INTO operator_decision_heads (
          id, process_id, disposition, reviewed_basis_hash, decision_stale, deleted_at
        ) VALUES (
          'decision-readiness', ?1, ?2, ?3, ?4, NULL
        )
        ON CONFLICT(process_id) DO UPDATE SET
          disposition = excluded.disposition,
          reviewed_basis_hash = excluded.reviewed_basis_hash,
          decision_stale = excluded.decision_stale,
          deleted_at = excluded.deleted_at
        ",
        params![
            PROCESS_ID,
            disposition,
            reviewed_basis_hash,
            i64::from(decision_stale)
        ],
    )?;
    Ok(())
}

fn insert_pending_operator_intent(conn: &Connection, basis_hash: &str) -> Result<()> {
    conn.execute(
        "\
        INSERT INTO pending_operator_intents (
          id, process_id, desired_disposition, reviewed_basis_hash,
          reviewed_manifest_hash, reviewed_at_stage_run_id, reviewed_at,
          idempotency_key, queued_at
        ) VALUES (
          'pending-readiness-intent', ?1, 'approve', ?2, 'manifest-hash',
          'stage-run-readiness', '2026-05-23T00:00:00Z',
          'pending-readiness-intent-key', '2026-05-23T00:00:00Z'
        )
        ",
        params![PROCESS_ID, basis_hash],
    )?;
    Ok(())
}

fn insert_override(
    conn: &Connection,
    reason_code: &str,
    basis_hash: &str,
    payload: &serde_json::Value,
) -> Result<()> {
    let existing_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM operator_input_overrides WHERE reason_code = ?1",
        params![reason_code],
        |row| row.get(0),
    )?;
    conn.execute(
        "\
        INSERT INTO operator_input_overrides (
          id, process_id, product_candidate_id, reason_code, payload_json,
          basis_hash_when_provided, owner_id
        ) VALUES (
          ?1, ?2, ?3, ?4, ?5, ?6, 'local-owner'
        )
        ",
        params![
            format!("override-{reason_code}-{existing_count}"),
            PROCESS_ID,
            CANDIDATE_ID,
            reason_code,
            payload.to_string(),
            basis_hash
        ],
    )?;
    Ok(())
}

fn readiness(conn: &Connection, scope: &BidScopePayload) -> Vec<BidReadinessBlocker> {
    evaluate_bid_readiness(conn, scope)
        .expect("readiness gate evaluates")
        .blockers
}

fn assert_approved(conn: &Connection, scope: &BidScopePayload) {
    let result = evaluate_bid_readiness(conn, scope).expect("readiness gate evaluates");

    assert!(result.approved, "expected approved readiness: {result:?}");
    assert!(result.blockers.is_empty());
    assert!(
        result.resolved_company_profile.is_some(),
        "green readiness should pre-fetch company profile"
    );
}

fn assert_blocks(conn: &Connection, scope: &BidScopePayload, expected: BidReadinessBlocker) {
    let blockers = readiness(conn, scope);

    assert!(
        blockers.contains(&expected),
        "expected blocker {expected:?}, got {blockers:?}"
    );
}

#[test]
#[doc = "L6-T-bid_readiness_gate-001"]
fn l6_t_bid_readiness_gate_001_pending_intents_do_not_approve_bid() -> Result<()> {
    let (db, scope) = seeded_ready_scope()?;
    let conn = db.connection();
    conn.execute("DELETE FROM operator_decision_heads", [])?;
    insert_pending_operator_intent(conn, &scope.decision_basis_hash)?;

    assert_blocks(conn, &scope, BidReadinessBlocker::OperatorNotApproved);
    Ok(())
}

#[test]
#[doc = "L6-T-bid_readiness_gate-002"]
fn l6_t_bid_readiness_gate_002_stale_reviewed_basis_blocks() -> Result<()> {
    let (db, scope) = seeded_ready_scope()?;
    seed_operator_decision_head(db.connection(), "approved", Some(&"a".repeat(64)), false)?;

    assert_blocks(
        db.connection(),
        &scope,
        BidReadinessBlocker::DecisionBasisStale,
    );
    Ok(())
}

#[test]
#[doc = "L6-T-bid_readiness_gate-003"]
fn l6_t_bid_readiness_gate_003_classification_ambiguous_blocks() -> Result<()> {
    let (db, scope) = seeded_ready_scope()?;
    db.connection().execute(
        "UPDATE landed_cost_estimates SET blockers = '[\"CLASSIFICATION_AMBIGUOUS\"]' WHERE id = ?1",
        params![LCE_ID],
    )?;

    assert_blocks(
        db.connection(),
        &scope,
        BidReadinessBlocker::ClassificationAmbiguous,
    );
    Ok(())
}

#[test]
#[doc = "L6-T-bid_readiness_gate-004"]
fn l6_t_bid_readiness_gate_004_tariff_branch_divergence_blocks() -> Result<()> {
    let (db, scope) = seeded_ready_scope()?;
    db.connection().execute(
        "UPDATE landed_cost_estimates SET blockers = '[\"TARIFF_BRANCH_DIVERGENCE\"]' WHERE id = ?1",
        params![LCE_ID],
    )?;

    assert_blocks(
        db.connection(),
        &scope,
        BidReadinessBlocker::TariffBranchDivergence,
    );
    Ok(())
}

#[test]
#[doc = "L6-T-bid_readiness_gate-005"]
fn l6_t_bid_readiness_gate_005_supplementary_unit_missing_blocks() -> Result<()> {
    let (db, scope) = seeded_ready_scope()?;
    db.connection().execute(
        "UPDATE landed_cost_estimates SET blockers = '[\"SUPPLEMENTARY_UNIT_MISSING\"]' WHERE id = ?1",
        params![LCE_ID],
    )?;

    assert_blocks(
        db.connection(),
        &scope,
        BidReadinessBlocker::SupplementaryUnitMissing,
    );
    Ok(())
}

#[test]
#[doc = "L6-T-bid_readiness_gate-006"]
fn l6_t_bid_readiness_gate_006_broker_required_blocks() -> Result<()> {
    let (db, scope) = seeded_ready_scope()?;
    db.connection().execute(
        "UPDATE landed_cost_estimates SET decision_grade = 'broker_required' WHERE id = ?1",
        params![LCE_ID],
    )?;

    assert_blocks(db.connection(), &scope, BidReadinessBlocker::BrokerRequired);
    Ok(())
}

#[test]
#[doc = "L6-T-bid_readiness_gate-007"]
fn l6_t_bid_readiness_gate_007_expired_classification_freshness_blocks() -> Result<()> {
    let (db, scope) = seeded_ready_scope()?;
    db.connection().execute(
        "UPDATE landed_cost_estimates SET classification_fresh_until = '2000-01-01T00:00:00Z' WHERE id = ?1",
        params![LCE_ID],
    )?;

    assert_blocks(
        db.connection(),
        &scope,
        BidReadinessBlocker::ClassificationFreshnessExpired,
    );
    Ok(())
}

#[test]
#[doc = "L6-T-bid_readiness_gate-008"]
fn l6_t_bid_readiness_gate_008_expired_company_profile_blocks() -> Result<()> {
    let (db, scope) = seeded_ready_scope()?;
    seed_company_profile(db.connection(), "2000-01-01", "2099-01-01")?;

    assert_blocks(
        db.connection(),
        &scope,
        BidReadinessBlocker::CompanyProfileOutOfDate,
    );
    Ok(())
}

#[test]
#[doc = "L6-T-bid_readiness_gate-009"]
fn l6_t_bid_readiness_gate_009_floor_only_quote_role_blocks_without_override() -> Result<()> {
    let (db, scope) = seeded_ready_scope()?;
    db.connection().execute(
        "UPDATE supplier_quotes SET price_signal_role = 'domestic_wholesale_floor' WHERE id = ?1",
        params![QUOTE_ID],
    )?;

    let result = evaluate_bid_readiness(db.connection(), &scope)?;

    assert!(!result.approved);
    assert!(
        result
            .blockers
            .contains(&BidReadinessBlocker::QuoteRoleFloorOnly)
    );
    assert_eq!(
        result.override_required,
        Some(readiness_gate::BidReadinessOverrideKind::QuoteRoleQuality)
    );
    Ok(())
}

#[test]
#[doc = "L6-T-bid_readiness_gate-010"]
fn l6_t_bid_readiness_gate_010_quote_role_override_allows_progress() -> Result<()> {
    let (db, scope) = seeded_ready_scope()?;
    db.connection().execute(
        "UPDATE supplier_quotes SET price_signal_role = 'optimistic_floor' WHERE id = ?1",
        params![QUOTE_ID],
    )?;
    insert_override(
        db.connection(),
        QUOTE_ROLE_QUALITY_OVERRIDE_REASON_CODE,
        &scope.decision_basis_hash,
        &json!({
            "acknowledged": true,
            "overrideReason": "operator accepts floor-only quote evidence for this scope",
            "auditTrailEntryId": "run-manifest#quote-role-override"
        }),
    )?;

    assert_approved(db.connection(), &scope);
    Ok(())
}

#[test]
#[doc = "L6-T-bid_readiness_gate-011"]
fn l6_t_bid_readiness_gate_011_trust_debt_requires_exact_scope_acceptance() -> Result<()> {
    let (db, scope) = seeded_ready_scope()?;
    db.connection().execute(
        "UPDATE pipeline_assessments SET validation_state = 'flagged' WHERE process_id = ?1",
        params![PROCESS_ID],
    )?;
    insert_override(
        db.connection(),
        TRUST_DEBT_ACCEPTANCE_REASON_CODE,
        &scope.decision_basis_hash,
        &json!({
            "accepted": true,
            "decisionBasisHash": scope.decision_basis_hash,
            "lotIds": ["different-lot"],
            "candidateIds": [CANDIDATE_ID]
        }),
    )?;
    assert_blocks(
        db.connection(),
        &scope,
        BidReadinessBlocker::UpstreamTrustUnvalidated,
    );

    db.connection().execute(
        "UPDATE operator_input_overrides SET withdrawn_at = '2026-05-23T00:00:00Z'",
        [],
    )?;
    insert_override(
        db.connection(),
        TRUST_DEBT_ACCEPTANCE_REASON_CODE,
        &scope.decision_basis_hash,
        &json!({
            "accepted": true,
            "decisionBasisHash": scope.decision_basis_hash,
            "lotIds": [LOT_ID],
            "candidateIds": [CANDIDATE_ID]
        }),
    )?;

    assert_approved(db.connection(), &scope);
    Ok(())
}

#[test]
#[doc = "L6-T-bid_readiness_gate-012"]
fn l6_t_bid_readiness_gate_012_green_candidate_is_eligible() -> Result<()> {
    let (db, scope) = seeded_ready_scope()?;

    assert_approved(db.connection(), &scope);
    Ok(())
}

#[test]
#[doc = "L6-T-bid_readiness_gate-013"]
fn l6_t_bid_readiness_gate_013_recomputes_current_basis_every_call() -> Result<()> {
    let (db, scope) = seeded_ready_scope()?;
    assert_approved(db.connection(), &scope);
    db.connection().execute(
        "UPDATE pipeline_assessments SET decision_basis_hash = 'pipeline-basis-readiness-v2' WHERE process_id = ?1",
        params![PROCESS_ID],
    )?;

    assert_blocks(
        db.connection(),
        &scope,
        BidReadinessBlocker::DecisionBasisStale,
    );
    Ok(())
}

#[test]
#[doc = "L6-T-bid_readiness_gate-014"]
fn l6_t_bid_readiness_gate_014_missing_portal_ref_blocks() -> Result<()> {
    let (db, mut scope) = seeded_ready_scope()?;
    scope.portal_opportunity_ref.clear();

    assert_blocks(
        db.connection(),
        &scope,
        BidReadinessBlocker::PortalOpportunityRefMissing,
    );
    Ok(())
}

#[test]
#[doc = "L6-T-bid_readiness_gate-015"]
fn l6_t_bid_readiness_gate_015_reports_multiple_blockers_together() -> Result<()> {
    let (db, mut scope) = seeded_ready_scope()?;
    let conn = db.connection();
    scope.portal_opportunity_ref.clear();
    seed_operator_decision_head(conn, "pending", Some(&scope.decision_basis_hash), false)?;
    conn.execute(
        "\
        UPDATE landed_cost_estimates
        SET blockers = '[\"CLASSIFICATION_AMBIGUOUS\", \"TARIFF_BRANCH_DIVERGENCE\"]',
            decision_grade = 'broker_required'
        WHERE id = ?1
        ",
        params![LCE_ID],
    )?;
    conn.execute(
        "UPDATE pipeline_assessments SET validation_state = 'unverified' WHERE process_id = ?1",
        params![PROCESS_ID],
    )?;

    let blockers = readiness(conn, &scope);

    for expected in [
        BidReadinessBlocker::PortalOpportunityRefMissing,
        BidReadinessBlocker::OperatorNotApproved,
        BidReadinessBlocker::ClassificationAmbiguous,
        BidReadinessBlocker::TariffBranchDivergence,
        BidReadinessBlocker::BrokerRequired,
        BidReadinessBlocker::UpstreamTrustUnvalidated,
    ] {
        assert!(
            blockers.contains(&expected),
            "expected blocker {expected:?}, got {blockers:?}"
        );
    }
    Ok(())
}

#[test]
#[doc = "L6-T-bid_readiness_gate-016"]
fn l6_t_bid_readiness_gate_016_non_overrideable_attempt_is_rejected() -> Result<()> {
    let (db, scope) = seeded_ready_scope()?;
    let conn = db.connection();
    seed_operator_decision_head(conn, "pending", Some(&scope.decision_basis_hash), false)?;
    insert_override(
        conn,
        NON_OVERRIDEABLE_GATE_OVERRIDE_REASON_CODE,
        &scope.decision_basis_hash,
        &json!({
            "acknowledged": true,
            "overrideReason": "operator attempted to bypass approval gate",
            "auditTrailEntryId": "run-manifest#rejected-override"
        }),
    )?;

    let blockers = readiness(conn, &scope);

    assert!(blockers.contains(&BidReadinessBlocker::OperatorNotApproved));
    assert!(blockers.contains(&BidReadinessBlocker::RejectedNonOverrideableOverride));
    Ok(())
}
