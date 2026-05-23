#![forbid(unsafe_code)]

mod support;

#[path = "../src/bid_workflow/types.rs"]
#[allow(dead_code)]
mod types;

#[path = "../src/bid_workflow/decision_basis_hash.rs"]
#[allow(dead_code)]
mod decision_basis_hash;

use anyhow::Result;
use rusqlite::{Connection, params};

use crate::support::db::{FTS_SOURCE_ID, TestStateDb, seed_process};
use crate::types::{BidScopePayload, SideEffectClass};

const PROCESS_ID: &str = "process-decision-basis";
const NOTICE_ID: &str = "notice-decision-basis";
const LOT_ID: &str = "lot-decision-basis";
const PIPELINE_BASIS_HASH: &str = "pipeline-basis-v1";
const COMPANY_PROFILE_ID: &str = "company-profile-v1";

fn valid_scope() -> BidScopePayload {
    BidScopePayload {
        process_id: PROCESS_ID.to_owned(),
        lot_ids: vec![LOT_ID.to_owned()],
        candidate_ids: vec!["candidate-2".to_owned(), "candidate-1".to_owned()],
        selected_quote_ids: vec!["quote-2".to_owned(), "quote-1".to_owned()],
        company_profile_version: COMPANY_PROFILE_ID.to_owned(),
        bid_portal_type: "procontract".to_owned(),
        portal_opportunity_ref: "DN-BASIS-001".to_owned(),
        review_required: true,
        decision_basis_hash: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
            .to_owned(),
        side_effect_cap: SideEffectClass::RemoteDraft,
    }
}

fn seed_decision_basis_fixture(conn: &Connection) -> Result<()> {
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
          ?1, ?2, ?3, 'source-notice-basis', 'raw-document-basis',
          'Decision basis notice', 'tender', '2026-05-22T00:00:00Z',
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
          ?1, ?2, 1, 'Decision basis lot', 'local-owner'
        )
        ",
        params![LOT_ID, NOTICE_ID],
    )?;
    conn.execute(
        "\
        INSERT INTO pipeline_assessments (
          process_id, pipeline_state, decision_basis_hash, owner_id
        ) VALUES (
          ?1, 'net_margin_passed', ?2, 'local-owner'
        )
        ",
        params![PROCESS_ID, PIPELINE_BASIS_HASH],
    )?;
    seed_candidate_quote_and_landed_cost(conn, "candidate-1", "quote-1", "lce-1")?;
    seed_candidate_quote_and_landed_cost(conn, "candidate-2", "quote-2", "lce-2")?;
    seed_company_profile_projection(conn)?;
    Ok(())
}

fn seed_candidate_quote_and_landed_cost(
    conn: &Connection,
    candidate_id: &str,
    quote_id: &str,
    estimate_id: &str,
) -> Result<()> {
    conn.execute(
        "\
        INSERT INTO product_candidates (
          id, process_id, notice_id, lot_id, name, candidate_type,
          match_archetype, head_noun, owner_id
        ) VALUES (
          ?1, ?2, ?3, ?4, ?5, 'candidate', 'direct', 'widget', 'local-owner'
        )
        ",
        params![
            candidate_id,
            PROCESS_ID,
            NOTICE_ID,
            LOT_ID,
            format!("Decision basis product {candidate_id}")
        ],
    )?;
    conn.execute(
        "\
        INSERT INTO supplier_quotes (
          id, process_id, notice_id, product_candidate_id, product_name, region,
          quote_class, source_name, unit_price_original_minor, currency,
          unit_price_gbp_pence, price_signal_role, evidence, owner_id
        ) VALUES (
          ?1, ?2, ?3, ?4, 'Decision basis product', 'UK', 'marketplace',
          'fixture', 1000, 'GBP', 1000, 'normal_market_quote', '{}', 'local-owner'
        )
        ",
        params![quote_id, PROCESS_ID, NOTICE_ID, candidate_id],
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
        params![estimate_id, PROCESS_ID, NOTICE_ID, candidate_id, quote_id],
    )?;
    Ok(())
}

fn seed_company_profile_projection(conn: &Connection) -> Result<()> {
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
          ?1, 1, NULL
        )
        ",
        params![COMPANY_PROFILE_ID],
    )?;
    Ok(())
}

fn seeded_hash() -> Result<(TestStateDb, BidScopePayload, String)> {
    let db = TestStateDb::new()?;
    let conn = db.connection();
    seed_decision_basis_fixture(conn)?;
    let scope = valid_scope();
    let hash = decision_basis_hash::compute(conn, &scope)?;
    Ok((db, scope, hash))
}

fn assert_single_vector_mutation_changes_hash<F>(mutate: F) -> Result<()>
where
    F: FnOnce(&Connection) -> Result<()>,
{
    let (db, scope, baseline) = seeded_hash()?;
    mutate(db.connection())?;
    let changed = decision_basis_hash::compute(db.connection(), &scope)?;
    assert_ne!(baseline, changed);
    Ok(())
}

#[test]
#[doc = "L6-T-bid_workflow_states-031"]
fn l6_t_bid_workflow_states_031_hash_is_lowercase_sha256_and_order_stable() -> Result<()> {
    let (db, scope, baseline) = seeded_hash()?;

    assert_eq!(baseline.len(), 64);
    assert!(
        baseline
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)),
        "decision basis hash must be lowercase hexadecimal: {baseline}"
    );

    let mut reordered_scope = scope;
    reordered_scope.candidate_ids.reverse();
    reordered_scope.selected_quote_ids.reverse();
    let reordered = decision_basis_hash::compute(db.connection(), &reordered_scope)?;
    assert_eq!(
        baseline, reordered,
        "selected candidate and quote ID order must not affect the decision basis hash"
    );
    Ok(())
}

#[test]
#[doc = "L6-T-bid_workflow_states-031"]
fn l6_t_bid_workflow_states_031_hash_changes_when_pipeline_assessment_changes() -> Result<()> {
    assert_single_vector_mutation_changes_hash(|conn| {
        conn.execute(
            "UPDATE pipeline_assessments SET decision_basis_hash = 'pipeline-basis-v2' WHERE process_id = ?1",
            params![PROCESS_ID],
        )?;
        Ok(())
    })
}

#[test]
#[doc = "L6-T-bid_workflow_states-031"]
fn l6_t_bid_workflow_states_031_hash_changes_when_quote_version_changes() -> Result<()> {
    assert_single_vector_mutation_changes_hash(|conn| {
        conn.execute(
            "UPDATE supplier_quotes SET row_version = row_version + 1 WHERE id = 'quote-1'",
            [],
        )?;
        Ok(())
    })
}

#[test]
#[doc = "L6-T-bid_workflow_states-031"]
fn l6_t_bid_workflow_states_031_hash_changes_when_company_profile_version_changes() -> Result<()> {
    assert_single_vector_mutation_changes_hash(|conn| {
        conn.execute(
            "UPDATE shared_company_profile SET remote_revision = 2 WHERE id = ?1",
            params![COMPANY_PROFILE_ID],
        )?;
        Ok(())
    })
}

#[test]
#[doc = "L6-T-bid_workflow_states-031"]
fn l6_t_bid_workflow_states_031_hash_changes_when_classification_freshness_changes() -> Result<()> {
    assert_single_vector_mutation_changes_hash(|conn| {
        conn.execute(
            "\
            UPDATE landed_cost_estimates
            SET classification_fresh_until = '2000-01-01T00:00:00Z'
            WHERE product_candidate_id = 'candidate-1'
            ",
            [],
        )?;
        Ok(())
    })
}

#[test]
#[doc = "L6-T-bid_workflow_states-031"]
fn l6_t_bid_workflow_states_031_hash_changes_when_l5_freshness_vector_changes() -> Result<()> {
    assert_single_vector_mutation_changes_hash(|conn| {
        conn.execute(
            "\
            UPDATE landed_cost_estimates
            SET tariff_checked_at = '2026-05-23T00:00:00Z'
            WHERE product_candidate_id = 'candidate-1'
            ",
            [],
        )?;
        Ok(())
    })
}

#[test]
#[doc = "L6-T-bid_workflow_states-031"]
fn l6_t_bid_workflow_states_031_hash_changes_when_l5_blocker_state_changes() -> Result<()> {
    assert_single_vector_mutation_changes_hash(|conn| {
        conn.execute(
            "\
            UPDATE landed_cost_estimates
            SET blockers = '[\"CLASSIFICATION_AMBIGUOUS\"]'
            WHERE product_candidate_id = 'candidate-1'
            ",
            [],
        )?;
        Ok(())
    })
}
