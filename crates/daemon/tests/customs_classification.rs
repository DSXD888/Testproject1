#![forbid(unsafe_code)]

#[allow(dead_code, clippy::missing_const_for_fn)]
#[path = "../src/errors/manifest.rs"]
mod errors_manifest_impl;
#[allow(dead_code, clippy::missing_const_for_fn)]
#[path = "../src/errors/markdown.rs"]
mod errors_markdown_impl;
mod support;

use std::collections::BTreeSet;
use std::path::PathBuf;

use anyhow::{Context, Result, ensure};
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::{Value, json};

use crate::errors_manifest_impl::{ErrorCode, parse_error_code};
use support::db::{DEFAULT_SCHEMA_EPOCH, FTS_SOURCE_ID, L4TestStateDb, StateDb};
use support::sync::{OutboxOp, OutboxWriter, canonical_content_hash, write_push_owned_transaction};

// L5 ownership:
// - L5-T-customs_classification-004,006,007,008,022,023,024,036
// - L5-T-measure_interaction-019,020
//
// Downstream L5 beads extend this target with tariff graph cache,
// product_commodity_codes, complete landed_cost_estimates population,
// ErrorCode integration, measure interaction warnings/blockers,
// VAT/valuation conflict persistence, redaction sentinels, and L5->L6
// readiness blocker coverage.
const L5_PRODUCT_COMMODITY_CODES_MIGRATION: &str = "0013_l5_product_commodity_codes";

fn assert_l5_product_commodity_codes_schema_applied() -> Result<()> {
    let db = L4TestStateDb::new()?;
    let conn = db.connection();

    let user_version: i64 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;
    ensure!(
        user_version == DEFAULT_SCHEMA_EPOCH,
        "L5 customs classification smoke expected schema epoch {DEFAULT_SCHEMA_EPOCH}, got {user_version}"
    );

    let migration_id: Option<String> = conn
        .query_row(
            "SELECT id FROM schema_migrations WHERE id = ?1",
            [L5_PRODUCT_COMMODITY_CODES_MIGRATION],
            |row| row.get(0),
        )
        .optional()?;
    ensure!(
        migration_id.as_deref() == Some(L5_PRODUCT_COMMODITY_CODES_MIGRATION),
        "L5 customs classification smoke did not observe migration {L5_PRODUCT_COMMODITY_CODES_MIGRATION}"
    );

    let table_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'product_commodity_codes'",
        [],
        |row| row.get(0),
    )?;
    ensure!(
        table_count == 1,
        "L5 customs classification smoke did not observe product_commodity_codes"
    );

    Ok(())
}

fn assert_l5_landed_cost_freshness_projection_applied() -> Result<()> {
    let db = L4TestStateDb::new()?;
    let conn = db.connection();
    let mut statement = conn.prepare("PRAGMA table_info(landed_cost_estimates)")?;
    let mut rows = statement.query([])?;
    let mut columns = BTreeSet::new();
    while let Some(row) = rows.next()? {
        let name: String = row.get(1)?;
        columns.insert(name);
    }

    for required in [
        "tariff_checked_at",
        "measures_fresh_until",
        "classification_fresh_until",
        "trade_remedy_watch_state",
        "blockers",
    ] {
        ensure!(
            columns.contains(required),
            "L5 customs freshness expected landed_cost_estimates column `{required}`"
        );
    }

    Ok(())
}

fn assert_trade_remedy_watch_stale_error_code_registered() -> Result<()> {
    let parsed = parse_error_code("TRADE_REMEDY_WATCH_STALE")?;
    ensure!(
        parsed == ErrorCode::TradeRemedyWatchStale,
        "L5 customs freshness expected TRADE_REMEDY_WATCH_STALE parser mapping"
    );
    ensure!(
        ErrorCode::TradeRemedyWatchStale.as_str() == "TRADE_REMEDY_WATCH_STALE",
        "L5 customs freshness expected TRADE_REMEDY_WATCH_STALE string mapping"
    );
    Ok(())
}

#[test]
fn l5_customs_classification_smoke_l5_t_customs_classification_004_006_007_008_022_023_024_036()
-> Result<()> {
    assert_l5_product_commodity_codes_schema_applied()?;
    assert_l5_landed_cost_freshness_projection_applied()?;
    assert_trade_remedy_watch_stale_error_code_registered()
}

#[test]
fn l5_t_customs_classification_004_006_product_commodity_codes_replace_under_one_stage_run()
-> Result<()> {
    let (_temp, mut state_db) = new_state_db()?;
    let conn = state_db.connection_mut();
    seed_l5_fk_graph(conn)?;

    conn.execute(
        "\
        INSERT INTO product_commodity_codes (
          id, product_candidate_id, commodity_code, digits, source, score_bps,
          rank_ordinal, description, evidence_json, owner_id, row_version, stage_run_id
        ) VALUES (
          'l5-old-code', 'l5-product-candidate', '3923100000', 10,
          'hmrc_fpo_or_search_suggestion', 9100, 1, 'old plastic branch',
          '{\"agreement_state\":\"old\"}', 'local-owner', 1, 'stage-old'
        )
        ",
        [],
    )?;

    let writer = outbox_writer()?;
    write_push_owned_transaction(conn, "l5-pcc-replacement", |tx| {
        tx.execute(
            "DELETE FROM product_commodity_codes WHERE product_candidate_id = ?1",
            ["l5-product-candidate"],
        )?;
        insert_replacement_product_commodity_code_rows(tx, &writer)
    })?;

    ensure!(
        query_count(
            conn,
            "SELECT COUNT(*) FROM product_commodity_codes WHERE stage_run_id = 'stage-old'",
        )? == 0,
        "stale commodity-code rows survived candidate-level replacement"
    );
    ensure!(
        query_count(
            conn,
            "SELECT COUNT(*) FROM product_commodity_codes
             WHERE product_candidate_id = 'l5-product-candidate'
               AND stage_run_id = 'stage-new'
               AND last_local_seq IS NOT NULL",
        )? == 2,
        "replacement rows were not inserted under one stage_run_id with outbox seqs"
    );
    ensure!(
        query_count(
            conn,
            "SELECT COUNT(*) FROM local_outbox
             WHERE entity_kind = 'product_commodity_codes'
               AND commit_group_id = 'l5-pcc-replacement'",
        )? == 2,
        "product_commodity_codes replacement did not emit local_outbox rows"
    );
    Ok(())
}

#[test]
fn l5_t_customs_classification_007_036_and_measure_interaction_019_complete_landed_cost_row_uses_push_owned_outbox()
-> Result<()> {
    let (_temp, mut state_db) = new_state_db()?;
    let conn = state_db.connection_mut();
    seed_l5_fk_graph(conn)?;
    let local_seq = insert_complete_landed_cost_row(conn)?;

    let columns = table_columns(conn, "landed_cost_estimates")?;
    let expected_columns: BTreeSet<String> = LANDED_COST_COLUMN_CONTRACT
        .iter()
        .copied()
        .map(str::to_owned)
        .collect();
    ensure!(
        columns == expected_columns,
        "landed_cost_estimates column contract drifted"
    );

    let null_columns = landed_cost_null_columns(conn, "l5-landed-cost-complete")?;
    ensure!(
        null_columns
            == BTreeSet::from([
                "cbam_estimate_gbp_pence".to_owned(),
                "permit_responsibility".to_owned(),
            ]),
        "unexpected explicit-null landed-cost columns: {null_columns:?}"
    );

    let outbox: (String, String, String, String, String) = conn.query_row(
        "\
        SELECT entity_kind, entity_id, op, stage_run_id, commit_group_id
        FROM local_outbox
        WHERE local_seq = ?1
        ",
        [local_seq],
        |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
            ))
        },
    )?;
    ensure!(
        outbox
            == (
                "landed_cost_estimates".to_owned(),
                "l5-landed-cost-complete".to_owned(),
                "insert".to_owned(),
                "l5-stage-gate".to_owned(),
                "l5-landed-cost-complete".to_owned(),
            ),
        "landed_cost_estimates outbox row did not preserve L4 push-owned semantics"
    );
    Ok(())
}

#[test]
fn l5_t_customs_classification_022_023_024_034_readiness_blockers_persist_from_l5_evidence()
-> Result<()> {
    let (_temp, mut state_db) = new_state_db()?;
    let conn = state_db.connection_mut();
    seed_l5_fk_graph(conn)?;
    insert_complete_landed_cost_row(conn)?;

    let (decision_grade, blockers_json, classification_fresh_until, watch_state): (
        String,
        String,
        String,
        String,
    ) = conn.query_row(
        "\
        SELECT decision_grade, blockers, classification_fresh_until, trade_remedy_watch_state
        FROM landed_cost_estimates
        WHERE id = 'l5-landed-cost-complete'
        ",
        [],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
    )?;
    let blocker_codes = issue_codes(&blockers_json)?;

    ensure!(decision_grade == "broker_required");
    for required in [
        "CLASSIFICATION_AMBIGUOUS",
        "TARIFF_BRANCH_DIVERGENCE",
        "SUPPLEMENTARY_UNIT_MISSING",
    ] {
        ensure!(
            blocker_codes.contains(required),
            "readiness blocker `{required}` was not persisted"
        );
    }
    ensure!(
        classification_fresh_until.as_str() < "2026-05-20T00:00:00Z",
        "expired classification_fresh_until must remain visible to L6 readiness"
    );
    ensure!(
        watch_state == "investigation_open",
        "TRA watcher state must persist independently of tariff freshness"
    );
    Ok(())
}

#[test]
fn l5_t_measure_interaction_020_redaction_sentinel_design_is_phase_6_ready() -> Result<()> {
    let root = repo_root()?;
    let design = std::fs::read_to_string(
        root.join("docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-redaction-sentinel-design.md"),
    )?;
    let fixture = std::fs::read_to_string(root.join("tests/fixtures/l5/redaction/sentinels.yaml"))?;

    for field in [
        "exporter_rate_basis",
        "exporter_identity_confidence_bps",
        "origin_evidence_state",
        "trade_remedy_cases_json",
        "duty_breakdown_json",
        "measure_interaction_json",
        "confidence_breakdown.evidence_leverage",
    ] {
        ensure!(design.contains(field), "redaction design missing `{field}`");
        ensure!(
            fixture.contains(field),
            "redaction fixture missing `{field}`"
        );
    }
    ensure!(
        design.contains("table-level RLS as sufficient evidence"),
        "Phase 6 sentinel design must reject table-level RLS alone"
    );
    Ok(())
}

const LANDED_COST_COLUMN_CONTRACT: &[&str] = &[
    "id",
    "process_id",
    "notice_id",
    "product_candidate_id",
    "supplier_quote_id",
    "shipping_mode",
    "mode_confidence_bps",
    "packing_scenario",
    "declarable_commodity_code_10",
    "candidate_commodity_codes_json",
    "additional_codes",
    "additional_code_requirements_json",
    "supplementary_units_required_json",
    "commodity_confidence_bps",
    "classification_stability_bps",
    "customs_valuation_method",
    "valuation_mode",
    "customs_fx_rate",
    "commercial_fx_rate",
    "fx_basis_delta_bps",
    "fx_rate_checked_at",
    "supplier_cost_gbp_pence",
    "freight_low_gbp_pence",
    "freight_base_gbp_pence",
    "freight_high_gbp_pence",
    "customs_duty_gbp_pence",
    "trade_remedy_duty_gbp_pence",
    "duty_outcome_min_gbp_pence",
    "duty_outcome_max_gbp_pence",
    "customs_cost_spread_bps",
    "trade_remedy_state",
    "trade_remedy_cases_json",
    "duty_breakdown_json",
    "measure_interaction_json",
    "exporter_rate_basis",
    "exporter_identity_confidence_bps",
    "origin_evidence_state",
    "import_vat_gbp_pence",
    "vat_rate_state",
    "import_vat_treatment",
    "vat_recovery_rate_bps",
    "uk_final_mile_cost_gbp_pence",
    "other_fees_gbp_pence",
    "goods_landed_cost_gbp_pence",
    "installation_labour_cost_gbp_pence",
    "installation_labour_low_gbp_pence",
    "installation_labour_high_gbp_pence",
    "installation_labour_confidence_bps",
    "permit_traffic_mgmt_cost_gbp_pence",
    "permit_traffic_mgmt_low_gbp_pence",
    "permit_traffic_mgmt_high_gbp_pence",
    "permit_responsibility",
    "economic_landed_cost_gbp_pence",
    "cash_outlay_gbp_pence",
    "cash_to_clear_gbp_pence",
    "working_capital_cost_gbp_pence",
    "revenue_basis_type",
    "revenue_confidence_bps",
    "customs_resolution_confidence_bps",
    "commercial_estimate_confidence_bps",
    "net_margin_low_bps",
    "net_margin_base_bps",
    "net_margin_high_bps",
    "decision_grade",
    "confidence_bps",
    "confidence_breakdown",
    "warnings",
    "blockers",
    "tariff_checked_at",
    "measures_fresh_until",
    "classification_fresh_until",
    "trade_remedy_watch_state",
    "cbam_exposure_flag",
    "cbam_estimate_gbp_pence",
    "owner_id",
    "row_version",
    "last_local_seq",
    "stage_run_id",
    "updated_at",
    "created_at",
];

fn new_state_db() -> Result<(tempfile::TempDir, StateDb)> {
    let temp = tempfile::tempdir()?;
    let db_path = temp.path().join("state.db");
    let state_db = StateDb::open(&db_path)?;
    Ok((temp, state_db))
}

fn outbox_writer() -> Result<OutboxWriter> {
    let schema_epoch =
        i32::try_from(DEFAULT_SCHEMA_EPOCH).context("DEFAULT_SCHEMA_EPOCH exceeds i32")?;
    OutboxWriter::new("l5-customs-gate", schema_epoch)
}

fn insert_replacement_product_commodity_code_rows(
    tx: &rusqlite::Transaction<'_>,
    writer: &OutboxWriter,
) -> Result<()> {
    for (id, code, source, rank) in [
        (
            "l5-new-code-1",
            "4819100000",
            "hmrc_tariff_keyword_search",
            1_i64,
        ),
        ("l5-new-code-2", "4823908580", "atr_codebook_reuse", 2_i64),
    ] {
        let payload = json!({
            "id": id,
            "product_candidate_id": "l5-product-candidate",
            "commodity_code": code,
            "digits": 10,
            "source": source,
            "score_bps": 8800,
            "rank_ordinal": rank,
            "description": "shortlisted branch",
            "evidence_json": {"candidateRecallAgreement": [{"candidateCode": "4819"}]},
            "owner_id": "local-owner",
            "row_version": 2,
            "stage_run_id": "stage-new"
        });
        let payload_json = payload.to_string();
        let content_hash = canonical_content_hash(&payload_json)?;
        tx.execute(
            "\
            INSERT INTO product_commodity_codes (
              id, product_candidate_id, commodity_code, digits, source, score_bps,
              rank_ordinal, description, evidence_json, owner_id, row_version, stage_run_id
            ) VALUES (
              ?1, 'l5-product-candidate', ?2, 10, ?3, 8800, ?4,
              'shortlisted branch', ?5, 'local-owner', 2, 'stage-new'
            )
            ",
            params![id, code, source, rank, payload["evidence_json"].to_string()],
        )?;
        let local_seq = writer.write_push_owned_with_outbox(
            tx,
            "product_commodity_codes",
            id,
            OutboxOp::Insert,
            &payload_json,
            &content_hash,
            "stage-new",
            "l5-pcc-replacement",
        )?;
        tx.execute(
            "UPDATE product_commodity_codes SET last_local_seq = ?1 WHERE id = ?2",
            params![local_seq, id],
        )?;
    }
    Ok(())
}

fn seed_l5_fk_graph(conn: &Connection) -> Result<()> {
    conn.execute(
        "\
        INSERT INTO procurement_processes (
          id, current_stage, title, latest_published_at, latest_updated_at, owner_id
        ) VALUES (
          'l5-process', 'classification', 'L5 customs gate process',
          '2026-05-19T00:00:00Z', '2026-05-19T00:00:00Z', 'local-owner'
        )
        ",
        [],
    )?;
    conn.execute(
        "\
        INSERT INTO contract_notices (
          id, process_id, source_id, source_notice_id, raw_document_id, title,
          procurement_stage, published_at, source_updated_at, normalized_payload, owner_id
        ) VALUES (
          'l5-notice', 'l5-process', ?1, 'l5-source-notice', 'l5-raw-doc',
          'L5 customs gate notice', 'tender', '2026-05-19T00:00:00Z',
          '2026-05-19T00:00:00Z', '{}', 'local-owner'
        )
        ",
        [FTS_SOURCE_ID],
    )?;
    conn.execute(
        "\
        INSERT INTO notice_lots (
          id, notice_id, lot_number, title, estimated_value_pence, owner_id
        ) VALUES ('l5-lot', 'l5-notice', 1, 'L5 customs lot', 250000, 'local-owner')
        ",
        [],
    )?;
    conn.execute(
        "\
        INSERT INTO product_candidates (
          id, process_id, notice_id, lot_id, name, candidate_type, match_archetype,
          head_noun, owner_id
        ) VALUES (
          'l5-product-candidate', 'l5-process', 'l5-notice', 'l5-lot',
          'Paperboard cartons', 'physical_good', 'direct_supply',
          'carton', 'local-owner'
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
          'l5-supplier-quote', 'l5-process', 'l5-notice', 'l5-product-candidate',
          'Paperboard cartons', 'CN', 'supplier_listing', 'Alibaba',
          1200, 'USD', 940, 'optimistic_floor', '{}', 'local-owner'
        )
        ",
        [],
    )?;
    Ok(())
}

fn insert_complete_landed_cost_row(conn: &mut Connection) -> Result<i64> {
    let writer = outbox_writer()?;
    let payload = complete_landed_cost_payload();
    let payload_json = payload.to_string();
    let content_hash = canonical_content_hash(&payload_json)?;
    let warnings = json!([{
        "code": "additional_code_unsatisfied:DE 6/17",
        "severity": "warning"
    }])
    .to_string();
    let blockers = json!([
        {"code": "CLASSIFICATION_AMBIGUOUS", "severity": "blocked"},
        {"code": "TARIFF_BRANCH_DIVERGENCE", "severity": "blocked"},
        {"code": "SUPPLEMENTARY_UNIT_MISSING", "severity": "blocked"},
        {"code": "classification_fresh_until_expired", "severity": "blocked"}
    ])
    .to_string();
    let confidence_breakdown = json!({
        "evidence_leverage": [{
            "parameter": "supplementary_units",
            "marginDeltaBps": 350,
            "unlockable": true
        }]
    })
    .to_string();
    let local_seq = write_push_owned_transaction(conn, "l5-landed-cost-complete", |tx| {
        tx.execute(
            complete_landed_cost_insert_sql(),
            params![
                json!(["4819100000", "3923100000", "4823908580"]).to_string(),
                json!(["DE 6/17"]).to_string(),
                json!([{
                    "code": "DE 6/17",
                    "satisfiability": "unknown",
                    "evidenceRequired": ["vat_relief_document"]
                }])
                .to_string(),
                json!(["kg_net"]).to_string(),
                json!([{
                    "caseReference": "TRA/2026/ACME-ALUMINIUM",
                    "state": "investigation_open"
                }])
                .to_string(),
                json!({
                    "components": [{
                        "measureId": "specific-aluminium-kg",
                        "amountPence": null
                    }]
                })
                .to_string(),
                json!({
                    "family": "conditional",
                    "selectedBranch": "out_of_quota"
                })
                .to_string(),
                confidence_breakdown,
                warnings,
                blockers,
            ],
        )?;
        let local_seq = writer.write_push_owned_with_outbox(
            tx,
            "landed_cost_estimates",
            "l5-landed-cost-complete",
            OutboxOp::Insert,
            &payload_json,
            &content_hash,
            "l5-stage-gate",
            "l5-landed-cost-complete",
        )?;
        tx.execute(
            "\
            UPDATE landed_cost_estimates
            SET last_local_seq = ?1
            WHERE id = 'l5-landed-cost-complete'
            ",
            [local_seq],
        )?;
        Ok(local_seq)
    })?;
    Ok(local_seq)
}

const fn complete_landed_cost_insert_sql() -> &'static str {
    "\
    INSERT INTO landed_cost_estimates (
      id, process_id, notice_id, product_candidate_id, supplier_quote_id,
      shipping_mode, mode_confidence_bps, packing_scenario,
      declarable_commodity_code_10, candidate_commodity_codes_json, additional_codes,
      additional_code_requirements_json, supplementary_units_required_json,
      commodity_confidence_bps, classification_stability_bps, customs_valuation_method,
      valuation_mode, customs_fx_rate, commercial_fx_rate, fx_basis_delta_bps,
      fx_rate_checked_at, supplier_cost_gbp_pence, freight_low_gbp_pence,
      freight_base_gbp_pence, freight_high_gbp_pence, customs_duty_gbp_pence,
      trade_remedy_duty_gbp_pence, duty_outcome_min_gbp_pence,
      duty_outcome_max_gbp_pence, customs_cost_spread_bps, trade_remedy_state,
      trade_remedy_cases_json, duty_breakdown_json, measure_interaction_json,
      exporter_rate_basis, exporter_identity_confidence_bps, origin_evidence_state,
      import_vat_gbp_pence, vat_rate_state, import_vat_treatment, vat_recovery_rate_bps,
      uk_final_mile_cost_gbp_pence, other_fees_gbp_pence, economic_landed_cost_gbp_pence,
      cash_outlay_gbp_pence, cash_to_clear_gbp_pence, working_capital_cost_gbp_pence,
      revenue_basis_type, revenue_confidence_bps, customs_resolution_confidence_bps,
      commercial_estimate_confidence_bps, net_margin_low_bps, net_margin_base_bps,
      net_margin_high_bps, decision_grade, confidence_bps, confidence_breakdown,
      warnings, blockers, tariff_checked_at, measures_fresh_until,
      classification_fresh_until, trade_remedy_watch_state, cbam_exposure_flag,
      cbam_estimate_gbp_pence, owner_id, row_version, stage_run_id
    ) VALUES (
      'l5-landed-cost-complete', 'l5-process', 'l5-notice',
      'l5-product-candidate', 'l5-supplier-quote', 'sea_lcl', 9100,
      'palletized', '4819100000', ?1, ?2, ?3, ?4, 8700, 8800,
      '1_actual', 'declaration', 0.7834, 0.8070, 301,
      '2026-05-19T12:00:00Z', 94000, 18000, 22000, 30000, 4700,
      35100, 4700, 39800, 2100, 'investigation_open', ?5, ?6, ?7,
      'residual_default', 1500, 'mfn_default_missing_origin_proof',
      25000, 'standard', 'standard_rate_default', 0, 9000, 2000,
      202700, 202700, 202700, 1100, 'expected_value', 9000, 7600,
      8200, 700, 1000, 1300, 'broker_required', 6200, ?8, ?9, ?10,
      '2026-05-19T12:00:00Z', '2026-05-19T00:00:00Z',
      '2026-05-19T00:00:00Z', 'investigation_open', 1, NULL,
      'local-owner', 1, 'l5-stage-gate'
    )
    "
}

fn complete_landed_cost_payload() -> Value {
    json!({
        "id": "l5-landed-cost-complete",
        "process_id": "l5-process",
        "notice_id": "l5-notice",
        "product_candidate_id": "l5-product-candidate",
        "supplier_quote_id": "l5-supplier-quote",
        "decision_grade": "broker_required",
        "classification_fresh_until": "2026-05-19T00:00:00Z",
        "row_version": 1,
        "stage_run_id": "l5-stage-gate"
    })
}

fn table_columns(conn: &Connection, table: &str) -> Result<BTreeSet<String>> {
    let mut statement = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let mut rows = statement.query([])?;
    let mut columns = BTreeSet::new();
    while let Some(row) = rows.next()? {
        columns.insert(row.get::<_, String>(1)?);
    }
    Ok(columns)
}

fn landed_cost_null_columns(conn: &Connection, id: &str) -> Result<BTreeSet<String>> {
    let mut statement = conn.prepare(
        "\
        SELECT *
        FROM landed_cost_estimates
        WHERE id = ?1
        ",
    )?;
    let column_names: Vec<String> = statement
        .column_names()
        .into_iter()
        .map(str::to_owned)
        .collect();
    let mut rows = statement.query([id])?;
    let row = rows
        .next()?
        .ok_or_else(|| anyhow::anyhow!("missing landed_cost_estimates row `{id}`"))?;
    let mut null_columns = BTreeSet::new();
    for (index, column) in column_names.iter().enumerate() {
        if matches!(row.get_ref(index)?, rusqlite::types::ValueRef::Null) {
            null_columns.insert(column.clone());
        }
    }
    Ok(null_columns)
}

fn query_count(conn: &Connection, sql: &str) -> Result<i64> {
    Ok(conn.query_row(sql, [], |row| row.get(0))?)
}

fn issue_codes(issue_json: &str) -> Result<BTreeSet<String>> {
    let issues: Vec<Value> = serde_json::from_str(issue_json)?;
    Ok(issues
        .iter()
        .filter_map(|issue| issue.get("code").and_then(Value::as_str))
        .map(str::to_owned)
        .collect())
}

fn repo_root() -> Result<PathBuf> {
    Ok(PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .ok_or_else(|| anyhow::anyhow!("daemon crate should be two levels below repo root"))?
        .to_path_buf())
}
