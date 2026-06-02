#![forbid(unsafe_code)]

mod support;

use std::collections::BTreeSet;

use anyhow::{Context, Result, ensure};
use rusqlite::{Connection, params};

use support::db::L4TestStateDb;

const INSTALLATION_LABOUR_MIGRATION_ID: &str = "0014_installation_labour_fields";
const REQUIRED_LANDED_COST_COLUMNS: &[&str] = &[
    "goods_landed_cost_gbp_pence",
    "installation_labour_cost_gbp_pence",
    "installation_labour_low_gbp_pence",
    "installation_labour_high_gbp_pence",
    "installation_labour_confidence_bps",
    "permit_traffic_mgmt_cost_gbp_pence",
    "permit_traffic_mgmt_low_gbp_pence",
    "permit_traffic_mgmt_high_gbp_pence",
    "permit_responsibility",
];

#[derive(Debug, Clone, PartialEq, Eq)]
struct LandedCostSeed<'a> {
    id: &'a str,
    process_id: &'a str,
    notice_id: &'a str,
    product_candidate_id: &'a str,
    supplier_quote_id: &'a str,
    goods_landed_cost_gbp_pence: i64,
    installation_labour_cost_gbp_pence: i64,
    installation_labour_low_gbp_pence: i64,
    installation_labour_high_gbp_pence: i64,
    installation_labour_confidence_bps: i64,
    permit_traffic_mgmt_cost_gbp_pence: i64,
    permit_traffic_mgmt_low_gbp_pence: i64,
    permit_traffic_mgmt_high_gbp_pence: i64,
    permit_responsibility: &'a str,
    economic_landed_cost_gbp_pence: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PersistedLandedCostSeed {
    goods_landed_cost_gbp_pence: i64,
    installation_labour_cost_gbp_pence: i64,
    installation_labour_low_gbp_pence: i64,
    installation_labour_high_gbp_pence: i64,
    installation_labour_confidence_bps: i64,
    permit_traffic_mgmt_cost_gbp_pence: i64,
    permit_traffic_mgmt_low_gbp_pence: i64,
    permit_traffic_mgmt_high_gbp_pence: i64,
    permit_responsibility: String,
    economic_landed_cost_gbp_pence: i64,
}

#[test]
#[doc = "RS-SEED-001"]
fn rs_seed_001_landed_cost_insert_sql_helper_direct_seeds_labour_columns() -> Result<()> {
    let db = L4TestStateDb::new()?;
    let conn = db.connection();
    apply_installation_labour_migration_if_needed(conn)?;
    let ids = db.seed_push_owned_rows()?;
    seed_candidate_and_quote(conn, &ids.process_id, &ids.notice_id, &ids.lot_id)?;

    let seed = LandedCostSeed {
        id: "il-seed-landed-cost",
        process_id: &ids.process_id,
        notice_id: &ids.notice_id,
        product_candidate_id: "il-seed-candidate",
        supplier_quote_id: "il-seed-quote",
        goods_landed_cost_gbp_pence: 700_000,
        installation_labour_cost_gbp_pence: 120_000,
        installation_labour_low_gbp_pence: 96_000,
        installation_labour_high_gbp_pence: 144_000,
        installation_labour_confidence_bps: 7_500,
        permit_traffic_mgmt_cost_gbp_pence: 30_000,
        permit_traffic_mgmt_low_gbp_pence: 20_000,
        permit_traffic_mgmt_high_gbp_pence: 45_000,
        permit_responsibility: "contractor_provided",
        economic_landed_cost_gbp_pence: 850_000,
    };

    insert_landed_cost_seed(conn, &seed)?;
    let persisted = persisted_landed_cost_seed(conn, seed.id)?;

    ensure!(
        persisted
            == PersistedLandedCostSeed {
                goods_landed_cost_gbp_pence: seed.goods_landed_cost_gbp_pence,
                installation_labour_cost_gbp_pence: seed.installation_labour_cost_gbp_pence,
                installation_labour_low_gbp_pence: seed.installation_labour_low_gbp_pence,
                installation_labour_high_gbp_pence: seed.installation_labour_high_gbp_pence,
                installation_labour_confidence_bps: seed.installation_labour_confidence_bps,
                permit_traffic_mgmt_cost_gbp_pence: seed.permit_traffic_mgmt_cost_gbp_pence,
                permit_traffic_mgmt_low_gbp_pence: seed.permit_traffic_mgmt_low_gbp_pence,
                permit_traffic_mgmt_high_gbp_pence: seed.permit_traffic_mgmt_high_gbp_pence,
                permit_responsibility: seed.permit_responsibility.to_owned(),
                economic_landed_cost_gbp_pence: seed.economic_landed_cost_gbp_pence,
            },
        "landed_cost_insert_sql() did not persist the labour/permit/goods-only seed columns"
    );
    ensure!(
        goods_value_share_bps(
            persisted.goods_landed_cost_gbp_pence,
            persisted.installation_labour_cost_gbp_pence,
            persisted.permit_traffic_mgmt_cost_gbp_pence,
        ) == 8_235,
        "goods-value share must be reconstructable from one persisted landed_cost_estimates row"
    );
    Ok(())
}

fn apply_installation_labour_migration_if_needed(conn: &Connection) -> Result<()> {
    let columns = table_columns(conn, "landed_cost_estimates")?;
    let missing_columns = REQUIRED_LANDED_COST_COLUMNS
        .iter()
        .filter(|column| !columns.contains(**column))
        .copied()
        .collect::<Vec<_>>();
    let maintenance_table_exists = table_exists(conn, "maintenance_period_estimates")?;

    if missing_columns.is_empty() && maintenance_table_exists {
        record_installation_labour_migration(conn)?;
        return Ok(());
    }

    ensure!(
        missing_columns.len() == REQUIRED_LANDED_COST_COLUMNS.len() && !maintenance_table_exists,
        "partially applied installation labour migration; missing columns: {missing_columns:?}, maintenance table exists: {maintenance_table_exists}"
    );

    conn.execute_batch(include_str!(
        "../../../migrations/0014_installation_labour_fields.sql"
    ))
    .context("applying installation labour migration in test harness")?;
    record_installation_labour_migration(conn)
}

fn record_installation_labour_migration(conn: &Connection) -> Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO schema_migrations (id) VALUES (?1)",
        [INSTALLATION_LABOUR_MIGRATION_ID],
    )?;
    Ok(())
}

fn table_columns(conn: &Connection, table_name: &str) -> Result<BTreeSet<String>> {
    let mut statement = conn.prepare(&format!("PRAGMA table_info({table_name})"))?;
    let mut rows = statement.query([])?;
    let mut columns = BTreeSet::new();
    while let Some(row) = rows.next()? {
        columns.insert(row.get(1)?);
    }
    Ok(columns)
}

fn table_exists(conn: &Connection, table_name: &str) -> Result<bool> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
        [table_name],
        |row| row.get(0),
    )?;
    Ok(count == 1)
}

fn seed_candidate_and_quote(
    conn: &Connection,
    process_id: &str,
    notice_id: &str,
    lot_id: &str,
) -> Result<()> {
    conn.execute(
        "\
        INSERT INTO product_candidates (
          id, process_id, notice_id, lot_id, name, candidate_type,
          match_archetype, head_noun, owner_id
        ) VALUES (
          'il-seed-candidate', ?1, ?2, ?3, 'Installation labour seed product',
          'candidate', 'direct', 'shelving', 'local-owner'
        )
        ",
        params![process_id, notice_id, lot_id],
    )?;
    conn.execute(
        "\
        INSERT INTO supplier_quotes (
          id, process_id, notice_id, product_candidate_id, product_name, region,
          quote_class, source_name, unit_price_original_minor, currency,
          unit_price_gbp_pence, price_signal_role, evidence, owner_id
        ) VALUES (
          'il-seed-quote', ?1, ?2, 'il-seed-candidate', 'Installation labour seed product',
          'UK', 'marketplace', 'fixture', 700000, 'GBP', 700000,
          'normal_market_quote', '{}', 'local-owner'
        )
        ",
        params![process_id, notice_id],
    )?;
    Ok(())
}

fn insert_landed_cost_seed(conn: &Connection, seed: &LandedCostSeed<'_>) -> Result<()> {
    // MVP does not force-backfill historical landed-cost rows; nullable/default-0
    // migration columns keep old explicit INSERTs valid until assessments rerun.
    conn.execute(
        landed_cost_insert_sql(),
        params![
            seed.id,
            seed.process_id,
            seed.notice_id,
            seed.product_candidate_id,
            seed.supplier_quote_id,
            seed.goods_landed_cost_gbp_pence,
            seed.installation_labour_cost_gbp_pence,
            seed.installation_labour_low_gbp_pence,
            seed.installation_labour_high_gbp_pence,
            seed.installation_labour_confidence_bps,
            seed.permit_traffic_mgmt_cost_gbp_pence,
            seed.permit_traffic_mgmt_low_gbp_pence,
            seed.permit_traffic_mgmt_high_gbp_pence,
            seed.permit_responsibility,
            seed.economic_landed_cost_gbp_pence,
        ],
    )?;
    Ok(())
}

const fn landed_cost_insert_sql() -> &'static str {
    "\
    INSERT INTO landed_cost_estimates (
      id, process_id, notice_id, product_candidate_id, supplier_quote_id, shipping_mode,
      mode_confidence_bps, commodity_confidence_bps, classification_stability_bps,
      customs_fx_rate, commercial_fx_rate, fx_basis_delta_bps, fx_rate_checked_at,
      supplier_cost_gbp_pence, freight_low_gbp_pence, freight_base_gbp_pence,
      freight_high_gbp_pence, customs_duty_gbp_pence, import_vat_gbp_pence,
      import_vat_treatment, uk_final_mile_cost_gbp_pence, other_fees_gbp_pence,
      goods_landed_cost_gbp_pence, installation_labour_cost_gbp_pence,
      installation_labour_low_gbp_pence, installation_labour_high_gbp_pence,
      installation_labour_confidence_bps, permit_traffic_mgmt_cost_gbp_pence,
      permit_traffic_mgmt_low_gbp_pence, permit_traffic_mgmt_high_gbp_pence,
      permit_responsibility, economic_landed_cost_gbp_pence, cash_outlay_gbp_pence,
      cash_to_clear_gbp_pence, revenue_basis_type, revenue_confidence_bps,
      net_margin_low_bps, net_margin_base_bps, net_margin_high_bps, decision_grade,
      confidence_bps, warnings, blockers, owner_id, row_version, stage_run_id
    ) VALUES (
      ?1, ?2, ?3, ?4, ?5, 'domestic_install', 9000, 9000, 9000, 1.0, 1.0,
      0, '2026-06-02T00:00:00Z', 650000, 0, 0, 0, 0, 0, 'not_applicable',
      10000, 0, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?15, ?15,
      'expected_value', 9000, 2000, 3000, 4000, 'review_required', 7600,
      '[]', '[]', 'local-owner', 1, 'installation-labour-seed'
    )
    "
}

fn persisted_landed_cost_seed(
    conn: &Connection,
    landed_cost_id: &str,
) -> Result<PersistedLandedCostSeed> {
    conn.query_row(
        "\
        SELECT goods_landed_cost_gbp_pence, installation_labour_cost_gbp_pence,
               installation_labour_low_gbp_pence, installation_labour_high_gbp_pence,
               installation_labour_confidence_bps, permit_traffic_mgmt_cost_gbp_pence,
               permit_traffic_mgmt_low_gbp_pence, permit_traffic_mgmt_high_gbp_pence,
               permit_responsibility, economic_landed_cost_gbp_pence
        FROM landed_cost_estimates
        WHERE id = ?1
        ",
        [landed_cost_id],
        |row| {
            Ok(PersistedLandedCostSeed {
                goods_landed_cost_gbp_pence: row.get(0)?,
                installation_labour_cost_gbp_pence: row.get(1)?,
                installation_labour_low_gbp_pence: row.get(2)?,
                installation_labour_high_gbp_pence: row.get(3)?,
                installation_labour_confidence_bps: row.get(4)?,
                permit_traffic_mgmt_cost_gbp_pence: row.get(5)?,
                permit_traffic_mgmt_low_gbp_pence: row.get(6)?,
                permit_traffic_mgmt_high_gbp_pence: row.get(7)?,
                permit_responsibility: row.get(8)?,
                economic_landed_cost_gbp_pence: row.get(9)?,
            })
        },
    )
    .context("reading installation labour landed-cost seed row")
}

const fn goods_value_share_bps(goods: i64, labour: i64, permit_traffic_mgmt: i64) -> i64 {
    let denominator = goods
        .saturating_add(labour)
        .saturating_add(permit_traffic_mgmt);
    if denominator == 0 {
        return 0;
    }
    goods.saturating_mul(10_000).saturating_add(denominator / 2) / denominator
}
