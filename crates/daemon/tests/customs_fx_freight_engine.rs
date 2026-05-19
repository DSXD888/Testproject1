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

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
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

#[test]
fn l5_t_freight_engine_015_operator_prerequisites_gate_claims_and_stable_promotion() -> Result<()> {
    let freightos = assert_operator_built_prerequisite(
        "landed-cost/freightos-freight-estimate",
        "freightos-freight-estimate",
    )?;
    let taric =
        assert_operator_built_prerequisite("landed-cost/taric-cross-check", "taric-cross-check")?;

    ensure!(freightos.claim_gate_ready);
    ensure!(taric.claim_gate_ready);
    ensure!(
        freightos.stable_production_blockers == ["validator-rubric.yaml"],
        "Freightos stable-production blockers should preserve missing rubric state"
    );
    ensure!(
        taric.stable_production_blockers == ["validator-rubric.yaml"],
        "TARIC stable-production blockers should preserve missing rubric state"
    );
    assert_stable_production_blocker_contract()
}

#[test]
fn l5_t_freight_engine_016_marketplace_source_health_cron_dispatches() -> Result<()> {
    if std::env::var_os("L5_MARKETPLACE_SOURCE_HEALTH_CRON_CHILD").is_some() {
        return assert_marketplace_source_health_dispatch();
    }

    let output = Command::new(std::env::current_exe().context("locating current test binary")?)
        .arg("--exact")
        .arg("l5_t_freight_engine_016_marketplace_source_health_cron_dispatches")
        .arg("--nocapture")
        .env("L5_MARKETPLACE_SOURCE_HEALTH_CRON_CHILD", "1")
        .env(cron_impl::L5_ENABLE_MARKETPLACE_SOURCE_HEALTH_CRON_ENV, "1")
        .output()
        .context("running marketplace source-health cron child assertion")?;

    ensure!(
        output.status.success(),
        "L5-T-freight_engine-016 child assertion failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

#[test]
fn l5_t_freight_engine_004_005_provider_warning_and_failure_persist_to_landed_cost_estimates()
-> Result<()> {
    let db = L4TestStateDb::new()?;
    let ids = db.seed_push_owned_rows()?;
    let conn = Connection::open(db.db_path())?;
    conn.execute_batch("PRAGMA foreign_keys = ON")?;
    seed_landed_cost_inputs(&conn, &ids)?;

    insert_freight_landed_cost_case(
        &conn,
        &ids,
        &FreightPersistenceCase {
            id: "l5-freight-disagreement",
            freight_low_gbp_pence: 175_000,
            freight_base_gbp_pence: 185_000,
            freight_high_gbp_pence: 195_000,
            warnings: json!([{
                "code": "FREIGHT_MODE_LOW_CONFIDENCE",
                "message": "Freightos and SeaRates exceeded the disagreement threshold",
                "severity": "warning"
            }]),
            blockers: json!([]),
            decision_grade: "review_required",
        },
    )?;
    insert_freight_landed_cost_case(
        &conn,
        &ids,
        &FreightPersistenceCase {
            id: "l5-freight-failed",
            freight_low_gbp_pence: 0,
            freight_base_gbp_pence: 0,
            freight_high_gbp_pence: 0,
            warnings: json!([]),
            blockers: json!([{
                "code": "FREIGHT_ESTIMATE_FAILED",
                "message": "Neither Freightos nor SeaRates returned a usable quote",
                "severity": "blocked"
            }]),
            decision_grade: "broker_required",
        },
    )?;

    assert_freight_landed_cost_issue(
        &conn,
        "l5-freight-disagreement",
        "FREIGHT_MODE_LOW_CONFIDENCE",
        IssueColumn::Warnings,
        185_000,
    )?;
    assert_freight_landed_cost_issue(
        &conn,
        "l5-freight-failed",
        "FREIGHT_ESTIMATE_FAILED",
        IssueColumn::Blockers,
        0,
    )
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

fn assert_marketplace_source_health_dispatch() -> Result<()> {
    let db = L4TestStateDb::new()?;
    let registry = sources_impl::SourceRegistry::load(db.connection())?;
    let mut scheduler = cron_impl::CronScheduler::from_registry(&registry);
    scheduler.refresh_next_run_at(ts(2026, 5, 19, 4, 59, 59)?)?;

    let due_at = ts(2026, 5, 19, 5, 0, 1)?;
    let dispatches = scheduler.dispatch_due_jobs(&registry, due_at)?;
    let dispatch = dispatches
        .iter()
        .find(|dispatch| {
            dispatch.definition.skill_id == cron_impl::MARKETPLACE_SOURCE_HEALTH_CANARY_SKILL_ID
        })
        .ok_or_else(|| anyhow!("missing marketplace source-health dispatch"))?;
    let payload = dispatch
        .enqueue_request
        .input_payload
        .as_object()
        .context("cron input payload must be an object")?;

    ensure!(dispatch.definition.enabled);
    ensure!(dispatch.definition.source_id == cron_impl::L5_COMMERCIAL_INTELLIGENCE_SOURCE_ID);
    ensure!(
        dispatch.definition.cron_expression == cron_impl::MARKETPLACE_SOURCE_HEALTH_CANARY_CRON
    );
    ensure!(dispatch.job_class == cron_impl::CronJobClass::SourceHealth);
    ensure!(dispatch.priority == arbiter_impl::AdmissionPriority::AuthRefreshOrHealthProbe);
    ensure!(dispatch.enqueue_request.kind == "source_health");
    ensure!(dispatch.enqueue_request.scope_kind.as_deref() == Some("source"));
    ensure!(
        dispatch.enqueue_request.scope_id.as_deref()
            == Some(cron_impl::L5_COMMERCIAL_INTELLIGENCE_SOURCE_ID)
    );
    ensure!(
        payload.get("skill_id").and_then(Value::as_str)
            == Some(cron_impl::MARKETPLACE_SOURCE_HEALTH_CANARY_SKILL_ID)
    );
    ensure!(
        dispatch
            .enqueue_request
            .idempotency_key
            .contains(cron_impl::MARKETPLACE_SOURCE_HEALTH_CANARY_SKILL_ID)
    );
    assert_l5_source_health_skill_artifacts()
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct OperatorPrerequisiteAudit {
    claim_gate_ready: bool,
    stable_production_blockers: Vec<String>,
}

fn assert_operator_built_prerequisite(
    skill_id: &str,
    evidence_dir_name: &str,
) -> Result<OperatorPrerequisiteAudit> {
    let root = repo_root()?;
    let skill_dir = root.join("skills").join(skill_id);
    let evidence_dir = root
        .join(
            "docs/pipeline/l5-expanded-commercial-intelligence/operator-built-skills/test-evidence",
        )
        .join(evidence_dir_name);
    let skill_markdown = read_existing_file(&skill_dir.join("SKILL.md"))?;
    let _: Value =
        serde_json::from_str(&read_existing_file(&skill_dir.join("result.schema.json"))?)?;
    let report = read_existing_file(&evidence_dir.join("TEST_REPORT.md"))?;
    let canary_inputs = read_existing_file(&evidence_dir.join("canary.inputs.jsonl"))?;
    let golden = read_existing_file(&evidence_dir.join("golden.assertions.yaml"))?;

    ensure!(
        skill_markdown.contains(&format!("skill_id: {skill_id}")),
        "operator prerequisite skill metadata does not name `{skill_id}`"
    );
    ensure!(
        skill_markdown.contains("requires_cdp: true"),
        "operator prerequisite `{skill_id}` must remain a CDP/browser skill"
    );
    ensure!(
        report.contains("STATUS: READY FOR L5 STEP 09 BEAD AUDIT"),
        "operator evidence report for `{skill_id}` must be READY before Phase 3/4 claims"
    );
    ensure_canary_inputs_parse(&canary_inputs)?;
    let _: serde_yaml::Value = serde_yaml::from_str(&golden)?;

    let present_stable_artifacts = stable_artifact_presence(&evidence_dir);
    Ok(OperatorPrerequisiteAudit {
        claim_gate_ready: true,
        stable_production_blockers: stable_production_blockers(&present_stable_artifacts),
    })
}

fn assert_stable_production_blocker_contract() -> Result<()> {
    let present = ["canary.inputs.jsonl", "golden.assertions.yaml"]
        .into_iter()
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();
    ensure!(
        stable_production_blockers(&present) == ["validator-rubric.yaml"],
        "missing rubric must remain a stable-production blocker"
    );
    Ok(())
}

fn assert_l5_source_health_skill_artifacts() -> Result<()> {
    let skill_dir =
        repo_root()?.join("skills/source-health/l5-marketplace-commercial-intelligence");
    let skill_markdown = read_existing_file(&skill_dir.join("SKILL.md"))?;
    let result_schema: Value =
        serde_json::from_str(&read_existing_file(&skill_dir.join("result.schema.json"))?)?;
    let canary_sources =
        source_health_canary_sources(&read_existing_file(&skill_dir.join("canary.inputs.jsonl"))?)?;
    let required_sources = result_schema
        .pointer("/properties/per_source_health/required")
        .and_then(Value::as_array)
        .context("per_source_health.required must be present")?
        .iter()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();

    ensure!(skill_markdown.contains("paid_api_auth_check"));
    ensure!(canary_sources == l5_source_set());
    ensure!(required_sources == l5_source_set());
    Ok(())
}

fn source_health_canary_sources(canary_text: &str) -> Result<BTreeSet<String>> {
    let mut sources = BTreeSet::new();
    for line in canary_text.lines().filter(|line| !line.trim().is_empty()) {
        let row: Value = serde_json::from_str(line)?;
        ensure!(
            row.get("paid_api_auth_check").and_then(Value::as_bool) == Some(false),
            "source-health canary must not perform paid API auth checks"
        );
        let source_id = row
            .get("source_id")
            .and_then(Value::as_str)
            .context("source-health canary row missing source_id")?;
        sources.insert(source_id.to_owned());
    }
    Ok(sources)
}

fn l5_source_set() -> BTreeSet<String> {
    [
        "1688",
        "alibaba",
        "amazon",
        "freightos",
        "pcs",
        "searates",
        "taric",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect()
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

#[derive(Debug, Clone)]
struct FreightPersistenceCase {
    id: &'static str,
    freight_low_gbp_pence: i64,
    freight_base_gbp_pence: i64,
    freight_high_gbp_pence: i64,
    warnings: Value,
    blockers: Value,
    decision_grade: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IssueColumn {
    Warnings,
    Blockers,
}

fn insert_freight_landed_cost_case(
    conn: &Connection,
    ids: &support::db::PushOwnedSeedIds,
    case: &FreightPersistenceCase,
) -> Result<()> {
    conn.execute(
        freight_landed_cost_insert_sql(),
        params![
            case.id,
            ids.process_id,
            ids.notice_id,
            "l5-fx-candidate",
            "l5-fx-quote",
            case.freight_low_gbp_pence,
            case.freight_base_gbp_pence,
            case.freight_high_gbp_pence,
            case.warnings.to_string(),
            case.blockers.to_string(),
            case.decision_grade,
        ],
    )?;
    Ok(())
}

fn assert_freight_landed_cost_issue(
    conn: &Connection,
    landed_cost_id: &str,
    expected_code: &str,
    expected_column: IssueColumn,
    expected_freight_base_gbp_pence: i64,
) -> Result<()> {
    let (freight_base_gbp_pence, decision_grade, warnings, blockers): (
        i64,
        String,
        String,
        String,
    ) = conn.query_row(
        "\
        SELECT freight_base_gbp_pence, decision_grade, warnings, blockers
        FROM landed_cost_estimates
        WHERE id = ?1
        ",
        [landed_cost_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
    )?;
    let warning_codes = issue_codes(&warnings)?;
    let blocker_codes = issue_codes(&blockers)?;

    ensure!(freight_base_gbp_pence == expected_freight_base_gbp_pence);
    match expected_column {
        IssueColumn::Warnings => {
            ensure!(warning_codes.contains(expected_code));
            ensure!(blocker_codes.is_empty());
            ensure!(decision_grade == "review_required");
        }
        IssueColumn::Blockers => {
            ensure!(blocker_codes.contains(expected_code));
            ensure!(warning_codes.is_empty());
            ensure!(decision_grade == "broker_required");
        }
    }
    Ok(())
}

fn issue_codes(issue_json: &str) -> Result<BTreeSet<String>> {
    let issues: Vec<Value> = serde_json::from_str(issue_json)?;
    Ok(issues
        .iter()
        .filter_map(|issue| issue.get("code").and_then(Value::as_str))
        .map(str::to_owned)
        .collect())
}

const fn freight_landed_cost_insert_sql() -> &'static str {
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
      ?1, ?2, ?3, ?4, ?5, 'sea_lcl', 7600, 8700, 8800, 0.7834, 0.8070,
      301, '2026-05-19T12:00:00Z', 940, ?6, ?7, ?8, 47, 250, 'standard',
      90, 20, 2027, 2027, 2027, 'expected_value', 9000, 1500, 1800, 2100,
      ?11, 7600, ?9, ?10, 'local-owner', 1, 'l5-freight-gate-stage'
    )
    "
}

fn repo_root() -> Result<PathBuf> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .context("daemon crate should be two levels below repo root")
}

fn read_existing_file(path: &Path) -> Result<String> {
    std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))
}

fn ensure_canary_inputs_parse(canary_text: &str) -> Result<()> {
    let mut row_count = 0usize;
    for line in canary_text.lines().filter(|line| !line.trim().is_empty()) {
        let _: Value = serde_json::from_str(line)?;
        row_count += 1;
    }
    ensure!(
        row_count > 0,
        "canary.inputs.jsonl must contain at least one row"
    );
    Ok(())
}

fn stable_artifact_presence(evidence_dir: &Path) -> BTreeSet<String> {
    stable_production_artifacts()
        .into_iter()
        .filter(|artifact| evidence_dir.join(artifact).exists())
        .map(str::to_owned)
        .collect()
}

fn stable_production_blockers(present_artifacts: &BTreeSet<String>) -> Vec<String> {
    stable_production_artifacts()
        .into_iter()
        .filter(|artifact| !present_artifacts.contains(*artifact))
        .map(str::to_owned)
        .collect()
}

const fn stable_production_artifacts() -> [&'static str; 3] {
    [
        "canary.inputs.jsonl",
        "golden.assertions.yaml",
        "validator-rubric.yaml",
    ]
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
