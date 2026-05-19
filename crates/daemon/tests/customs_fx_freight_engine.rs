#![forbid(unsafe_code)]

mod support;

use anyhow::{Result, ensure};
use rusqlite::OptionalExtension;

use support::db::{DEFAULT_SCHEMA_EPOCH, L4TestStateDb};

// L5 ownership:
// - L5-T-fx_engine-005,006,009,010
// - L5-T-freight_engine-012,015,016
//
// Downstream L5 beads extend this target with HMRC FX rollover cron,
// historical correction enqueue, fx_basis_delta_bps storage,
// Freightos/SeaRates/TARIC prerequisite checks, marketplace source-health
// cron, and provider discrepancy warning persistence.
const L5_PRODUCT_COMMODITY_CODES_MIGRATION: &str = "0013_l5_product_commodity_codes";

fn assert_l5_product_commodity_codes_schema_applied() -> Result<()> {
    let db = L4TestStateDb::new()?;
    let conn = db.connection();

    let user_version: i64 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;
    ensure!(
        user_version == DEFAULT_SCHEMA_EPOCH,
        "L5 customs FX/freight smoke expected schema epoch {DEFAULT_SCHEMA_EPOCH}, got {user_version}"
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
        "L5 customs FX/freight smoke did not observe migration {L5_PRODUCT_COMMODITY_CODES_MIGRATION}"
    );

    let table_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'product_commodity_codes'",
        [],
        |row| row.get(0),
    )?;
    ensure!(
        table_count == 1,
        "L5 customs FX/freight smoke did not observe product_commodity_codes"
    );

    Ok(())
}

#[test]
fn l5_customs_fx_freight_engine_smoke_l5_t_fx_engine_005_006_009_010_freight_engine_012_015_016()
-> Result<()> {
    assert_l5_product_commodity_codes_schema_applied()
}
