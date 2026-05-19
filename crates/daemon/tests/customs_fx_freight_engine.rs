#![forbid(unsafe_code)]

#[path = "../src/agents/arbiter.rs"]
#[allow(dead_code)]
mod arbiter_impl;
#[path = "../src/agents/contracts.rs"]
#[allow(dead_code)]
mod contracts_impl;
#[path = "../src/scheduler/cron.rs"]
#[allow(dead_code)]
mod cron_impl;
#[path = "../src/jobs/runner.rs"]
#[allow(dead_code)]
mod runner_impl;
#[path = "../src/sources/registry.rs"]
#[allow(dead_code)]
mod sources_impl;
mod support;

use std::process::Command;

use anyhow::{Context, Result, anyhow, ensure};
use chrono::{DateTime, TimeZone, Utc};
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::{Value, json};

use support::db::{DEFAULT_SCHEMA_EPOCH, L4TestStateDb};
use support::sync::{OutboxOp, OutboxWriter, canonical_content_hash, write_push_owned_transaction};

mod agents {
    pub mod arbiter {
        pub use crate::arbiter_impl::*;
    }
}

mod jobs {
    pub mod runner {
        pub use crate::runner_impl::*;
    }
}

mod sources {
    pub mod registry {
        pub use crate::sources_impl::*;
    }
}

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

#[test]
fn l5_t_fx_engine_009_hmrc_fx_monthly_rollover_cron_dispatches() -> Result<()> {
    if std::env::var_os("L5_FX_CRON_CHILD").is_some() {
        return assert_hmrc_fx_rollover_dispatch();
    }

    let output = Command::new(std::env::current_exe().context("locating current test binary")?)
        .arg("--exact")
        .arg("l5_t_fx_engine_009_hmrc_fx_monthly_rollover_cron_dispatches")
        .arg("--nocapture")
        .env("L5_FX_CRON_CHILD", "1")
        .env(cron_impl::L5_ENABLE_HMRC_FX_ROLLOVER_CRON_ENV, "1")
        .output()
        .context("running HMRC FX rollover cron child assertion")?;

    ensure!(
        output.status.success(),
        "L5-T-fx_engine-009 child assertion failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn l5_t_fx_engine_010_landed_cost_dual_fx_fields_persist_with_l4_outbox() -> Result<()> {
    let db = L4TestStateDb::new()?;
    let ids = db.seed_push_owned_rows()?;
    let mut conn = Connection::open(db.db_path())?;
    conn.execute_batch("PRAGMA foreign_keys = ON")?;
    seed_landed_cost_inputs(&conn, &ids)?;

    let payload = fx_landed_cost_payload();
    let payload_json = serde_json::to_string(&payload)?;
    let content_hash = canonical_content_hash(&payload_json)?;
    let writer = OutboxWriter::new("l5-fx-gate", i32::try_from(DEFAULT_SCHEMA_EPOCH)?)?;

    let local_seq = write_push_owned_transaction(&mut conn, "l5-fx-gate-commit", |tx| {
        tx.execute(
            landed_cost_insert_sql(),
            params![
                payload["id"].as_str(),
                ids.process_id,
                ids.notice_id,
                "l5-fx-candidate",
                "l5-fx-quote",
                payload["customs_fx_rate"].as_f64(),
                payload["commercial_fx_rate"].as_f64(),
                payload["fx_basis_delta_bps"].as_i64(),
                payload["fx_rate_checked_at"].as_str(),
                payload["row_version"].as_i64(),
                payload["stage_run_id"].as_str(),
            ],
        )?;
        let local_seq = writer.write_push_owned_with_outbox(
            tx,
            "landed_cost_estimates",
            payload["id"]
                .as_str()
                .context("payload id must be a string")?,
            OutboxOp::Insert,
            &payload_json,
            &content_hash,
            payload["stage_run_id"]
                .as_str()
                .context("payload stage_run_id must be a string")?,
            "l5-fx-gate-commit",
        )?;
        tx.execute(
            "UPDATE landed_cost_estimates SET last_local_seq = ?1 WHERE id = ?2",
            params![local_seq, payload["id"].as_str()],
        )?;
        Ok(local_seq)
    })?;

    assert_landed_cost_fx_row(&conn, local_seq)?;
    assert_landed_cost_outbox_row(&conn, local_seq, &content_hash)
}

fn assert_hmrc_fx_rollover_dispatch() -> Result<()> {
    let db = L4TestStateDb::new()?;
    let registry = sources_impl::SourceRegistry::load(db.connection())?;
    let mut scheduler = cron_impl::CronScheduler::from_registry(&registry);
    scheduler.refresh_next_run_at(ts(2026, 5, 18, 4, 9, 59)?)?;

    let due_at = ts(2026, 5, 18, 4, 10, 1)?;
    let dispatches = scheduler.dispatch_due_jobs(&registry, due_at)?;
    let dispatch = dispatches
        .iter()
        .find(|dispatch| {
            dispatch.definition.skill_id == cron_impl::HMRC_FX_MONTHLY_ROLLOVER_SKILL_ID
        })
        .ok_or_else(|| anyhow!("missing HMRC FX monthly rollover dispatch"))?;
    let payload = dispatch
        .enqueue_request
        .input_payload
        .as_object()
        .context("cron input payload must be an object")?;

    ensure!(dispatch.definition.enabled);
    ensure!(dispatch.definition.source_id == cron_impl::L5_COMMERCIAL_INTELLIGENCE_SOURCE_ID);
    ensure!(dispatch.definition.cron_expression == cron_impl::HMRC_FX_MONTHLY_ROLLOVER_CRON);
    ensure!(dispatch.enqueue_request.kind == "sync_source");
    ensure!(dispatch.enqueue_request.scope_kind.as_deref() == Some("source"));
    ensure!(
        dispatch.enqueue_request.scope_id.as_deref()
            == Some(cron_impl::L5_COMMERCIAL_INTELLIGENCE_SOURCE_ID)
    );
    ensure!(
        payload.get("skill_id").and_then(Value::as_str)
            == Some(cron_impl::HMRC_FX_MONTHLY_ROLLOVER_SKILL_ID)
    );
    ensure!(
        dispatch
            .enqueue_request
            .idempotency_key
            .contains(cron_impl::HMRC_FX_MONTHLY_ROLLOVER_SKILL_ID)
    );
    Ok(())
}

fn seed_landed_cost_inputs(conn: &Connection, ids: &support::db::PushOwnedSeedIds) -> Result<()> {
    conn.execute(
        "\
        INSERT INTO product_candidates (
          id, process_id, notice_id, lot_id, name, candidate_type, match_archetype,
          head_noun, owner_id
        ) VALUES ('l5-fx-candidate', ?1, ?2, ?3, 'Stainless hex bolts',
                  'standard_commodity', 'exact', 'hex bolt', 'local-owner')
        ",
        params![ids.process_id, ids.notice_id, ids.lot_id],
    )?;
    conn.execute(
        "\
        INSERT INTO supplier_quotes (
          id, process_id, notice_id, product_candidate_id, product_name, region,
          quote_class, source_name, unit_price_original_minor, currency,
          unit_price_gbp_pence, price_signal_role, evidence, owner_id
        ) VALUES ('l5-fx-quote', ?1, ?2, 'l5-fx-candidate', 'Stainless hex bolts',
                  'CN', 'supplier_listing', 'Alibaba', 1200, 'USD', 940,
                  'optimistic_floor', '{}', 'local-owner')
        ",
        params![ids.process_id, ids.notice_id],
    )?;
    Ok(())
}

fn fx_landed_cost_payload() -> Value {
    json!({
        "id": "l5-fx-landed",
        "customs_fx_rate": 0.7834,
        "commercial_fx_rate": 0.8070,
        "fx_basis_delta_bps": 301,
        "fx_rate_checked_at": "2026-05-19T12:00:00Z",
        "row_version": 1,
        "stage_run_id": "l5-fx-gate-stage"
    })
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
      economic_landed_cost_gbp_pence, cash_outlay_gbp_pence, cash_to_clear_gbp_pence,
      revenue_basis_type, revenue_confidence_bps, net_margin_low_bps, net_margin_base_bps,
      net_margin_high_bps, decision_grade, confidence_bps, warnings, blockers, owner_id,
      row_version, stage_run_id
    ) VALUES (
      ?1, ?2, ?3, ?4, ?5, 'sea', 9100, 8700, 8800, ?6, ?7, ?8, ?9,
      940, 180, 220, 300, 47, 250, 'standard', 90, 20, 2027, 2027, 2027,
      'expected_value', 9000, 1500, 1800, 2100, 'review_required', 8400,
      '[]', '[]', 'local-owner', ?10, ?11
    )
    "
}

fn assert_landed_cost_fx_row(conn: &Connection, local_seq: i64) -> Result<()> {
    let row = conn.query_row(
        "\
        SELECT customs_fx_rate, commercial_fx_rate, fx_basis_delta_bps,
               fx_rate_checked_at, last_local_seq
        FROM landed_cost_estimates
        WHERE id = 'l5-fx-landed'
        ",
        [],
        |row| {
            Ok((
                row.get::<_, f64>(0)?,
                row.get::<_, f64>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, i64>(4)?,
            ))
        },
    )?;
    ensure!(
        row == (
            0.7834,
            0.8070,
            301,
            "2026-05-19T12:00:00Z".to_owned(),
            local_seq
        )
    );
    Ok(())
}

fn assert_landed_cost_outbox_row(
    conn: &Connection,
    local_seq: i64,
    content_hash: &str,
) -> Result<()> {
    let row = conn.query_row(
        "\
        SELECT entity_kind, entity_id, op, row_version, stage_run_id, content_hash
        FROM local_outbox
        WHERE local_seq = ?1
        ",
        [local_seq],
        |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
            ))
        },
    )?;
    ensure!(
        row == (
            "landed_cost_estimates".to_owned(),
            "l5-fx-landed".to_owned(),
            "insert".to_owned(),
            1,
            "l5-fx-gate-stage".to_owned(),
            content_hash.to_owned(),
        )
    );
    Ok(())
}

fn ts(
    year: i32,
    month: u32,
    day: u32,
    hour: u32,
    minute: u32,
    second: u32,
) -> Result<DateTime<Utc>> {
    Utc.with_ymd_and_hms(year, month, day, hour, minute, second)
        .single()
        .ok_or_else(|| anyhow!("invalid timestamp"))
}
