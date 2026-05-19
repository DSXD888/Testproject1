#![forbid(unsafe_code)]

mod support;

use anyhow::{Result, ensure};
use rusqlite::OptionalExtension;

use support::db::{DEFAULT_SCHEMA_EPOCH, L4TestStateDb};

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

#[test]
fn l5_customs_classification_smoke_l5_t_customs_classification_004_006_007_008_022_023_024_036()
-> Result<()> {
    assert_l5_product_commodity_codes_schema_applied()
}
