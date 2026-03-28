#![forbid(unsafe_code)]

#[allow(clippy::missing_const_for_fn)]
#[path = "../src/db/sqlite.rs"]
mod sqlite_impl;

use anyhow::Result;
use rusqlite::{Connection, params};

use crate::sqlite_impl::{DEFAULT_SCHEMA_EPOCH, StateDb};

const DEFAULT_OWNER_ID: &str = "local-owner";

fn open_temp_db() -> Result<(tempfile::TempDir, StateDb)> {
    let temp = tempfile::tempdir()?;
    let db_path = temp.path().join("state.db");
    let state_db = sqlite_impl::open_state_db(&db_path, DEFAULT_SCHEMA_EPOCH)?;
    Ok((temp, state_db))
}

fn seed_process(conn: &Connection, process_id: &str) -> Result<()> {
    conn.execute(
        "\
        INSERT INTO procurement_processes (
          id, canonical_process_id, current_stage, title, latest_published_at,
          latest_updated_at, owner_id
        ) VALUES (?1, ?1, 'review', 'Seed process', '2026-03-28T09:00:00Z',
                  '2026-03-28T09:00:00Z', ?2)
        ",
        params![process_id, DEFAULT_OWNER_ID],
    )?;
    conn.execute(
        "\
        INSERT INTO contract_notices (
          id, process_id, source_id, source_notice_id, raw_document_id, title,
          procurement_stage, published_at, source_updated_at, normalized_payload, owner_id
        ) VALUES ('notice-1', ?1, 'fts', 'source-1', 'raw-1', 'Notice title',
                  'tender', '2026-03-28T09:00:00Z', '2026-03-28T09:00:00Z', '{}', ?2)
        ",
        params![process_id, DEFAULT_OWNER_ID],
    )?;
    conn.execute(
        "\
        INSERT INTO notice_lots (
          id, notice_id, title, owner_id
        ) VALUES ('lot-1', 'notice-1', 'Lot 1', ?1)
        ",
        params![DEFAULT_OWNER_ID],
    )?;
    conn.execute(
        "\
        INSERT INTO product_candidates (
          id, process_id, notice_id, lot_id, name, candidate_type, match_archetype,
          head_noun, owner_id
        ) VALUES ('candidate-1', ?1, 'notice-1', 'lot-1', 'Widget', 'good', 'direct',
                  'widget', ?2)
        ",
        params![process_id, DEFAULT_OWNER_ID],
    )?;
    Ok(())
}

#[test]
fn ag_enr_001_detail_surface_can_read_enrichment_rows() -> Result<()> {
    let (_temp, state_db) = open_temp_db()?;
    let conn = state_db.connection();
    seed_process(conn, "process-1")?;

    conn.execute(
        "\
        INSERT INTO supplier_quotes (
          id, process_id, notice_id, product_candidate_id, product_name, region,
          quote_class, source_name, unit_price_original_minor, currency, unit_price_gbp_pence,
          price_signal_role, evidence, owner_id
        ) VALUES ('quote-1', 'process-1', 'notice-1', 'candidate-1', 'Widget', 'uk',
                  'catalog', 'Supplier A', 1200, 'GBP', 1200, 'primary', '{}', ?1)
        ",
        params![DEFAULT_OWNER_ID],
    )?;
    conn.execute(
        "\
        INSERT INTO landed_cost_estimates (
          id, process_id, notice_id, product_candidate_id, supplier_quote_id, shipping_mode,
          mode_confidence_bps, commodity_confidence_bps, classification_stability_bps,
          customs_fx_rate, commercial_fx_rate, fx_basis_delta_bps, supplier_cost_gbp_pence,
          freight_low_gbp_pence, freight_base_gbp_pence, freight_high_gbp_pence,
          customs_duty_gbp_pence, import_vat_gbp_pence, import_vat_treatment,
          uk_final_mile_cost_gbp_pence, other_fees_gbp_pence, economic_landed_cost_gbp_pence,
          cash_outlay_gbp_pence, cash_to_clear_gbp_pence, revenue_basis_type,
          revenue_confidence_bps, net_margin_low_bps, net_margin_base_bps, net_margin_high_bps,
          decision_grade, confidence_bps, warnings, owner_id
        ) VALUES (
          'landed-1', 'process-1', 'notice-1', 'candidate-1', 'quote-1', 'parcel',
          9000, 8000, 8000, 1.0, 1.0, 0, 1200, 100, 120, 140, 50, 240, 'recoverable',
          90, 20, 1720, 1720, 1720, 'addressable', 9000, 200, 300, 400, 'screening',
          8500, '[]', ?1
        )
        ",
        params![DEFAULT_OWNER_ID],
    )?;

    let quote_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM supplier_quotes WHERE process_id = 'process-1'",
        [],
        |row| row.get(0),
    )?;
    let landed_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM landed_cost_estimates WHERE process_id = 'process-1'",
        [],
        |row| row.get(0),
    )?;

    assert_eq!(quote_count, 1);
    assert_eq!(landed_count, 1);
    Ok(())
}

#[test]
fn ag_enr_002_quote_confidence_fields_round_trip() -> Result<()> {
    let (_temp, state_db) = open_temp_db()?;
    let conn = state_db.connection();
    seed_process(conn, "process-1")?;

    conn.execute(
        "\
        INSERT INTO supplier_quotes (
          id, process_id, notice_id, product_candidate_id, product_name, region,
          quote_class, source_name, unit_price_original_minor, currency, unit_price_gbp_pence,
          price_signal_role, quote_usability_confidence_bps, shipping_basis_confidence_bps,
          quote_realism_confidence_bps, match_confidence_bps, evidence, owner_id
        ) VALUES ('quote-1', 'process-1', 'notice-1', 'candidate-1', 'Widget', 'uk',
                  'catalog', 'Supplier A', 1200, 'GBP', 1200, 'primary', 7800, 7600, 7400,
                  8000, '{}', ?1)
        ",
        params![DEFAULT_OWNER_ID],
    )?;

    let confidences: (i64, i64, i64, i64) = conn.query_row(
        "\
        SELECT quote_usability_confidence_bps, shipping_basis_confidence_bps,
               quote_realism_confidence_bps, match_confidence_bps
        FROM supplier_quotes
        WHERE id = 'quote-1'
        ",
        [],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
    )?;

    assert_eq!(confidences, (7800, 7600, 7400, 8000));
    Ok(())
}

#[test]
fn ag_enr_003_basis_hash_change_marks_latest_decision_stale() -> Result<()> {
    let (_temp, state_db) = open_temp_db()?;
    let conn = state_db.connection();
    seed_process(conn, "process-1")?;

    conn.execute(
        "\
        INSERT INTO pipeline_assessments (
          process_id, pipeline_state, decision_basis_hash, owner_id
        ) VALUES ('process-1', 'review', 'basis-a', ?1)
        ",
        params![DEFAULT_OWNER_ID],
    )?;
    conn.execute(
        "\
        INSERT INTO pending_operator_intents (
          id, process_id, desired_disposition, reviewed_basis_hash, reviewed_manifest_hash,
          reviewed_at_stage_run_id, reviewed_at, idempotency_key, queued_at
        ) VALUES (
          'intent-1', 'process-1', 'watch', 'basis-a', 'manifest-a', 'stage-a',
          '2026-03-28T09:00:00Z', 'decision-key', '2026-03-28T09:00:00Z'
        )
        ",
        [],
    )?;

    let stale_before: i64 = conn.query_row(
        "\
        SELECT CASE WHEN poi.reviewed_basis_hash <> pa.decision_basis_hash THEN 1 ELSE 0 END
        FROM pending_operator_intents poi
        INNER JOIN pipeline_assessments pa ON pa.process_id = poi.process_id
        WHERE poi.id = 'intent-1'
        ",
        [],
        |row| row.get(0),
    )?;
    assert_eq!(stale_before, 0);

    conn.execute(
        "UPDATE pipeline_assessments SET decision_basis_hash = 'basis-b' WHERE process_id = 'process-1'",
        [],
    )?;

    let stale_after: i64 = conn.query_row(
        "\
        SELECT CASE WHEN poi.reviewed_basis_hash <> pa.decision_basis_hash THEN 1 ELSE 0 END
        FROM pending_operator_intents poi
        INNER JOIN pipeline_assessments pa ON pa.process_id = poi.process_id
        WHERE poi.id = 'intent-1'
        ",
        [],
        |row| row.get(0),
    )?;
    assert_eq!(stale_after, 1);
    Ok(())
}

#[test]
fn ag_enr_004_partial_quote_data_does_not_block_process_read_models() -> Result<()> {
    let (_temp, state_db) = open_temp_db()?;
    let conn = state_db.connection();
    seed_process(conn, "process-1")?;

    conn.execute(
        "\
        INSERT INTO pipeline_assessments (
          process_id, pipeline_state, priced_coverage_bps, landed_cost_coverage_bps,
          net_qualified_coverage_bps, owner_id
        ) VALUES ('process-1', 'review', 0, 0, 0, ?1)
        ",
        params![DEFAULT_OWNER_ID],
    )?;

    let coverage: (i64, i64, i64) = conn.query_row(
        "\
        SELECT priced_coverage_bps, landed_cost_coverage_bps, net_qualified_coverage_bps
        FROM pipeline_assessments
        WHERE process_id = 'process-1'
        ",
        [],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    )?;
    let quote_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM supplier_quotes WHERE process_id = 'process-1'",
        [],
        |row| row.get(0),
    )?;

    assert_eq!(coverage, (0, 0, 0));
    assert_eq!(quote_count, 0);
    Ok(())
}
