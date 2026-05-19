#![forbid(unsafe_code)]

#[allow(dead_code, clippy::missing_const_for_fn)]
#[path = "../src/errors/manifest.rs"]
mod errors_manifest_impl;
#[allow(dead_code, clippy::missing_const_for_fn)]
#[path = "../src/errors/markdown.rs"]
mod errors_markdown_impl;
mod support;

use std::collections::BTreeSet;

use anyhow::{Result, ensure};
use rusqlite::OptionalExtension;

use crate::errors_manifest_impl::{ErrorCode, parse_error_code};
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
