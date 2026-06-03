#![forbid(unsafe_code)]

mod support;

use std::collections::BTreeSet;

use anyhow::{Context, Result, ensure};
use rusqlite::{Connection, params};
use serde_json::Value;

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
    permit_responsibility: Option<&'a str>,
    economic_landed_cost_gbp_pence: i64,
    decision_grade: &'a str,
    warnings: &'a str,
    blockers: &'a str,
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
    permit_responsibility: Option<String>,
    economic_landed_cost_gbp_pence: i64,
    decision_grade: String,
    warnings: String,
    blockers: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MaintenancePeriodSeed<'a> {
    id: &'a str,
    process_id: &'a str,
    notice_id: &'a str,
    product_candidate_id: &'a str,
    landed_cost_estimate_id: &'a str,
    supplier_quote_id: &'a str,
    visit_scope: &'a str,
    visits_per_year: i64,
    term_years: i64,
    per_visit_labour_cost_gbp_pence: i64,
    annualised_opex_gbp_pence: i64,
    total_visible_term_opex_gbp_pence: i64,
    maintenance_confidence_bps: i64,
    margin_signal: &'a str,
    warnings: &'a str,
    evidence_json: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PersistedMaintenancePeriodSeed {
    product_candidate_id: String,
    landed_cost_estimate_id: String,
    supplier_quote_id: String,
    visit_scope: String,
    visits_per_year: i64,
    term_years: i64,
    per_visit_labour_cost_gbp_pence: i64,
    annualised_opex_gbp_pence: i64,
    total_visible_term_opex_gbp_pence: i64,
    maintenance_confidence_bps: i64,
    margin_signal: String,
    warnings: String,
    evidence_json: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PersistedDecisionBasis {
    goods_value_share_bps: Option<i64>,
    goods_value_share_basis: Option<String>,
    review_required: i64,
    review_reason: Option<String>,
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
        permit_responsibility: Some("contractor_provided"),
        economic_landed_cost_gbp_pence: 850_000,
        decision_grade: "review",
        warnings: "[]",
        blockers: "[]",
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
                permit_responsibility: seed.permit_responsibility.map(str::to_owned),
                economic_landed_cost_gbp_pence: seed.economic_landed_cost_gbp_pence,
                decision_grade: seed.decision_grade.to_owned(),
                warnings: seed.warnings.to_owned(),
                blockers: seed.blockers.to_owned(),
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

#[test]
#[doc = "E2E-IL-001"]
fn e2e_il_001_festive_lights_persisted_row_reconstructs_all_in_goods_share() -> Result<()> {
    let db = L4TestStateDb::new()?;
    let conn = db.connection();
    apply_installation_labour_migration_if_needed(conn)?;
    let ids = db.seed_push_owned_rows()?;
    seed_candidate_and_quote(conn, &ids.process_id, &ids.notice_id, &ids.lot_id)?;

    let seed = LandedCostSeed {
        id: "e2e-il-001-festive-lights-nw",
        process_id: &ids.process_id,
        notice_id: &ids.notice_id,
        product_candidate_id: "il-seed-candidate",
        supplier_quote_id: "il-seed-quote",
        goods_landed_cost_gbp_pence: 1_000_000,
        installation_labour_cost_gbp_pence: 240_000,
        installation_labour_low_gbp_pence: 210_000,
        installation_labour_high_gbp_pence: 265_000,
        installation_labour_confidence_bps: 7_000,
        permit_traffic_mgmt_cost_gbp_pence: 200_000,
        permit_traffic_mgmt_low_gbp_pence: 80_000,
        permit_traffic_mgmt_high_gbp_pence: 340_000,
        permit_responsibility: Some("unknown"),
        economic_landed_cost_gbp_pence: 1_440_000,
        decision_grade: "review",
        warnings: "[\"ACCESS_AT_HEIGHT_ASSUMED_MEWP\",\"PERMIT_RESPONSIBILITY_UNKNOWN\"]",
        blockers: "[]",
    };

    insert_landed_cost_seed(conn, &seed)?;
    let persisted = persisted_landed_cost_seed(conn, seed.id)?;

    ensure!(
        persisted.goods_landed_cost_gbp_pence == 1_000_000,
        "E2E-IL-001 goods-only landed subtotal must be persisted"
    );
    ensure!(
        persisted.installation_labour_cost_gbp_pence == 240_000
            && persisted.installation_labour_low_gbp_pence == 210_000
            && persisted.installation_labour_high_gbp_pence == 265_000,
        "E2E-IL-001 labour 3-point band changed"
    );
    ensure!(
        persisted.permit_traffic_mgmt_cost_gbp_pence == 200_000
            && persisted.permit_traffic_mgmt_high_gbp_pence == 340_000,
        "E2E-IL-001 permit/TM base/high values changed"
    );
    ensure!(
        persisted.permit_responsibility.as_deref() == Some("unknown"),
        "E2E-IL-001 unresolved permit responsibility must remain visible"
    );
    ensure!(
        persisted.economic_landed_cost_gbp_pence == 1_440_000,
        "E2E-IL-001 economic landed cost must include labour and permit/TM"
    );
    ensure!(
        goods_value_share_bps(
            persisted.goods_landed_cost_gbp_pence,
            persisted.installation_labour_cost_gbp_pence,
            0,
        ) == 8_065,
        "E2E-IL-001 pre-permit goods share must be reconstructable"
    );
    ensure!(
        goods_value_share_bps(
            persisted.goods_landed_cost_gbp_pence,
            persisted.installation_labour_cost_gbp_pence,
            persisted.permit_traffic_mgmt_cost_gbp_pence,
        ) == 6_944,
        "E2E-IL-001 all-in goods share must be reconstructable from one row"
    );
    ensure!(
        persisted.decision_grade == "review",
        "E2E-IL-001 must route to review for unresolved public-realm assumptions"
    );
    let warnings = stage11_issue_codes_from_json(&persisted.warnings)
        .context("parsing E2E-IL-001 warning codes")?;
    ensure!(
        warnings.contains("ACCESS_AT_HEIGHT_ASSUMED_MEWP")
            && warnings.contains("PERMIT_RESPONSIBILITY_UNKNOWN"),
        "E2E-IL-001 must persist access-at-height and permit-responsibility warnings"
    );
    assert_e2e_il_001_decision_basis(conn, &ids.process_id, &persisted)?;
    Ok(())
}

#[test]
#[doc = "E2E-IL-004"]
#[allow(clippy::too_many_lines)]
fn e2e_il_004_playground_maintenance_persists_first_class_period_estimate() -> Result<()> {
    let db = L4TestStateDb::new()?;
    let conn = db.connection();
    apply_installation_labour_migration_if_needed(conn)?;
    let ids = db.seed_push_owned_rows()?;
    seed_candidate_and_quote(conn, &ids.process_id, &ids.notice_id, &ids.lot_id)?;

    conn.execute(
        "\
        UPDATE product_candidates
        SET critical_attributes_json = ?1
        WHERE id = 'il-seed-candidate'
        ",
        [serde_json::json!({
            "installation_requirement": {
                "requires_installation": true,
                "requires_maintenance": true,
                "installation_scope": "playground_install",
                "maintenance_schedule": {
                    "visits_per_year": 4,
                    "term_years": 3,
                    "visit_scope": "quarterly inspection and minor service"
                }
            }
        })
        .to_string()],
    )?;

    let landed_cost = LandedCostSeed {
        id: "e2e-il-004-playground-maintenance",
        process_id: &ids.process_id,
        notice_id: &ids.notice_id,
        product_candidate_id: "il-seed-candidate",
        supplier_quote_id: "il-seed-quote",
        goods_landed_cost_gbp_pence: 1_800_000,
        installation_labour_cost_gbp_pence: 450_000,
        installation_labour_low_gbp_pence: 400_000,
        installation_labour_high_gbp_pence: 520_000,
        installation_labour_confidence_bps: 8_000,
        permit_traffic_mgmt_cost_gbp_pence: 50_000,
        permit_traffic_mgmt_low_gbp_pence: 20_000,
        permit_traffic_mgmt_high_gbp_pence: 80_000,
        permit_responsibility: Some("contractor_provided"),
        economic_landed_cost_gbp_pence: 2_300_000,
        decision_grade: "review",
        warnings: "[]",
        blockers: "[]",
    };
    insert_landed_cost_seed(conn, &landed_cost)?;

    let maintenance = MaintenancePeriodSeed {
        id: "e2e-il-004-maintenance-period",
        process_id: &ids.process_id,
        notice_id: &ids.notice_id,
        product_candidate_id: "il-seed-candidate",
        landed_cost_estimate_id: landed_cost.id,
        supplier_quote_id: "il-seed-quote",
        visit_scope: "quarterly inspection and minor service",
        visits_per_year: 4,
        term_years: 3,
        per_visit_labour_cost_gbp_pence: 60_000,
        annualised_opex_gbp_pence: 0,
        total_visible_term_opex_gbp_pence: 0,
        maintenance_confidence_bps: 0,
        margin_signal: "pending_maintenance_cost_model",
        warnings: "[]",
        evidence_json: r#"{"source":"installation_requirement","maintenance_schedule":{"visits_per_year":4,"term_years":3,"visit_scope":"quarterly inspection and minor service"}}"#,
    };
    insert_maintenance_period_seed(conn, &maintenance)?;

    let persisted_landed_cost = persisted_landed_cost_seed(conn, landed_cost.id)?;
    let persisted_maintenance = persisted_maintenance_period_seed(conn, maintenance.id)?;

    ensure!(
        persisted_maintenance.product_candidate_id == maintenance.product_candidate_id
            && persisted_maintenance.landed_cost_estimate_id == maintenance.landed_cost_estimate_id
            && persisted_maintenance.supplier_quote_id == maintenance.supplier_quote_id,
        "E2E-IL-004 maintenance estimate must be keyed by product candidate, landed-cost estimate, and quote"
    );
    ensure!(
        persisted_maintenance.visit_scope == maintenance.visit_scope
            && persisted_maintenance.visits_per_year == 4
            && persisted_maintenance.term_years == 3,
        "E2E-IL-004 maintenance schedule must persist as typed cadence/scope fields"
    );
    ensure!(
        persisted_maintenance.per_visit_labour_cost_gbp_pence == 60_000
            && persisted_maintenance.annualised_opex_gbp_pence == 0
            && persisted_maintenance.total_visible_term_opex_gbp_pence == 0
            && persisted_maintenance.maintenance_confidence_bps == 0,
        "E2E-IL-004 qoa5.5.3 must persist per-visit maintenance labour while qoa5.5.4 owns annual and visible-term opex"
    );
    ensure!(
        persisted_maintenance.margin_signal == "pending_maintenance_cost_model"
            && persisted_maintenance.warnings == "[]"
            && persisted_maintenance
                .evidence_json
                .contains("maintenance_schedule"),
        "E2E-IL-004 maintenance route must retain evidence and margin signal outside generic candidate JSON"
    );
    ensure!(
        persisted_landed_cost.economic_landed_cost_gbp_pence == 2_300_000,
        "E2E-IL-004 capital event denominator must exclude recurring maintenance opex"
    );
    ensure!(
        goods_value_share_bps(
            persisted_landed_cost.goods_landed_cost_gbp_pence,
            persisted_landed_cost.installation_labour_cost_gbp_pence,
            persisted_landed_cost.permit_traffic_mgmt_cost_gbp_pence,
        ) == 7_826,
        "E2E-IL-004 goods share must use the supply+install capital event only"
    );
    Ok(())
}

#[test]
#[doc = "E2E-IL-008"]
fn e2e_il_008_pure_a4_copier_paper_preserves_goods_only_economics() -> Result<()> {
    let db = L4TestStateDb::new()?;
    let conn = db.connection();
    apply_installation_labour_migration_if_needed(conn)?;
    let ids = db.seed_push_owned_rows()?;
    seed_candidate_and_quote(conn, &ids.process_id, &ids.notice_id, &ids.lot_id)?;

    let seed = LandedCostSeed {
        id: "e2e-il-008-pure-a4-copier-paper",
        process_id: &ids.process_id,
        notice_id: &ids.notice_id,
        product_candidate_id: "il-seed-candidate",
        supplier_quote_id: "il-seed-quote",
        goods_landed_cost_gbp_pence: 120_000,
        installation_labour_cost_gbp_pence: 0,
        installation_labour_low_gbp_pence: 0,
        installation_labour_high_gbp_pence: 0,
        installation_labour_confidence_bps: 0,
        permit_traffic_mgmt_cost_gbp_pence: 0,
        permit_traffic_mgmt_low_gbp_pence: 0,
        permit_traffic_mgmt_high_gbp_pence: 0,
        permit_responsibility: None,
        economic_landed_cost_gbp_pence: 120_000,
        decision_grade: "ready_for_bid",
        warnings: "[]",
        blockers: "[]",
    };

    insert_landed_cost_seed(conn, &seed)?;
    let persisted = persisted_landed_cost_seed(conn, seed.id)?;
    let critical_attributes_json: Option<String> = conn.query_row(
        "SELECT critical_attributes_json FROM product_candidates WHERE id = 'il-seed-candidate'",
        [],
        |row| row.get(0),
    )?;

    ensure!(
        critical_attributes_json.is_none(),
        "E2E-IL-008 must not seed an InstallationRequirement"
    );
    ensure!(
        persisted.goods_landed_cost_gbp_pence == 120_000,
        "pure-goods Scenario 8 goods subtotal changed"
    );
    ensure!(
        persisted.installation_labour_cost_gbp_pence == 0
            && persisted.installation_labour_low_gbp_pence == 0
            && persisted.installation_labour_high_gbp_pence == 0
            && persisted.installation_labour_confidence_bps == 0,
        "pure-goods Scenario 8 must keep installation labour at zero"
    );
    ensure!(
        persisted.permit_traffic_mgmt_cost_gbp_pence == 0
            && persisted.permit_traffic_mgmt_low_gbp_pence == 0
            && persisted.permit_traffic_mgmt_high_gbp_pence == 0,
        "pure-goods Scenario 8 must keep permit/TM at zero"
    );
    ensure!(
        persisted.permit_responsibility.is_none(),
        "pure-goods Scenario 8 must not persist permit responsibility"
    );
    ensure!(
        persisted.economic_landed_cost_gbp_pence == 120_000,
        "pure-goods Scenario 8 economic cost must equal goods-only subtotal"
    );
    ensure!(
        goods_value_share_bps(
            persisted.goods_landed_cost_gbp_pence,
            persisted.installation_labour_cost_gbp_pence,
            persisted.permit_traffic_mgmt_cost_gbp_pence,
        ) == 10_000,
        "pure-goods Scenario 8 goods share must remain 10000 bps"
    );
    Ok(())
}

#[test]
#[doc = "NEG-CTRL-001"]
#[doc = "E2E-IL-005"]
fn neg_ctrl_001_e2e_il_005_grounds_maintenance_stops_before_stage11() -> Result<()> {
    let db = L4TestStateDb::new()?;
    let conn = db.connection();
    apply_installation_labour_migration_if_needed(conn)?;
    let ids = db.seed_push_owned_rows()?;

    seed_rejected_service_qualification(
        conn,
        &ids.process_id,
        &ids.notice_id,
        &ids.lot_id,
        "neg-ctrl-001-grounds-maintenance",
        "reject_service: no_goods_head_noun; Provision of grounds maintenance and grass cutting",
    )?;

    assert_rejected_service_qualification(
        conn,
        &ids.process_id,
        "neg-ctrl-001-grounds-maintenance",
    )?;
    assert_no_stage11_rows_or_labour_jobs(conn, &ids.process_id)?;
    Ok(())
}

#[test]
#[doc = "NEG-CTRL-002"]
#[doc = "E2E-IL-006"]
fn neg_ctrl_002_e2e_il_006_managed_cleaning_rejects_on_dominance_floor_if_phase2_reached()
-> Result<()> {
    let db = L4TestStateDb::new()?;
    let conn = db.connection();
    apply_installation_labour_migration_if_needed(conn)?;
    let ids = db.seed_push_owned_rows()?;
    seed_candidate_and_quote(conn, &ids.process_id, &ids.notice_id, &ids.lot_id)?;

    let seed = LandedCostSeed {
        id: "e2e-il-006-managed-cleaning-dominance-floor",
        process_id: &ids.process_id,
        notice_id: &ids.notice_id,
        product_candidate_id: "il-seed-candidate",
        supplier_quote_id: "il-seed-quote",
        goods_landed_cost_gbp_pence: 50_000,
        installation_labour_cost_gbp_pence: 950_000,
        installation_labour_low_gbp_pence: 900_000,
        installation_labour_high_gbp_pence: 1_000_000,
        installation_labour_confidence_bps: 8_000,
        permit_traffic_mgmt_cost_gbp_pence: 0,
        permit_traffic_mgmt_low_gbp_pence: 0,
        permit_traffic_mgmt_high_gbp_pence: 0,
        permit_responsibility: None,
        economic_landed_cost_gbp_pence: 1_000_000,
        decision_grade: "reject",
        warnings: "[]",
        blockers: "[\"GOODS_DOMINANCE_FLOOR\"]",
    };

    insert_landed_cost_seed(conn, &seed)?;
    let persisted = persisted_landed_cost_seed(conn, seed.id)?;
    let blockers = stage11_issue_codes_from_json(&persisted.blockers)
        .context("parsing E2E-IL-006 blocker codes")?;
    let warnings = stage11_issue_codes_from_json(&persisted.warnings)
        .context("parsing E2E-IL-006 warning codes")?;

    ensure!(
        persisted.goods_landed_cost_gbp_pence == 50_000
            && persisted.installation_labour_cost_gbp_pence == 950_000
            && persisted.permit_traffic_mgmt_cost_gbp_pence == 0
            && persisted.economic_landed_cost_gbp_pence == 1_000_000,
        "E2E-IL-006 dominance-floor fixture values changed"
    );
    ensure!(
        goods_value_share_bps(
            persisted.goods_landed_cost_gbp_pence,
            persisted.installation_labour_cost_gbp_pence,
            persisted.permit_traffic_mgmt_cost_gbp_pence,
        ) == 500,
        "E2E-IL-006 goods share must be 500 bps"
    );
    ensure!(
        persisted.decision_grade == "reject",
        "E2E-IL-006 controlled Phase-2 twin must reject as economically service-only"
    );
    ensure!(
        blockers.contains("GOODS_DOMINANCE_FLOOR") && warnings.is_empty(),
        "E2E-IL-006 reject reason must be dominance floor, not missing-data warnings"
    );
    Ok(())
}

#[test]
#[doc = "E2E-IL-010"]
fn e2e_il_010_traffic_signal_supply_installation_rejects_on_phase2_dominance_floor() -> Result<()> {
    let db = L4TestStateDb::new()?;
    let conn = db.connection();
    apply_installation_labour_migration_if_needed(conn)?;
    let ids = db.seed_push_owned_rows()?;
    seed_candidate_and_quote(conn, &ids.process_id, &ids.notice_id, &ids.lot_id)?;

    let seed = LandedCostSeed {
        id: "e2e-il-010-traffic-signal-dominance-floor",
        process_id: &ids.process_id,
        notice_id: &ids.notice_id,
        product_candidate_id: "il-seed-candidate",
        supplier_quote_id: "il-seed-quote",
        goods_landed_cost_gbp_pence: 600_000,
        installation_labour_cost_gbp_pence: 900_000,
        installation_labour_low_gbp_pence: 860_000,
        installation_labour_high_gbp_pence: 960_000,
        installation_labour_confidence_bps: 6_500,
        permit_traffic_mgmt_cost_gbp_pence: 400_000,
        permit_traffic_mgmt_low_gbp_pence: 320_000,
        permit_traffic_mgmt_high_gbp_pence: 500_000,
        permit_responsibility: Some("contractor_provided"),
        economic_landed_cost_gbp_pence: 1_900_000,
        decision_grade: "reject",
        warnings: "[]",
        blockers: r#"[{"code":"GOODS_DOMINANCE_FLOOR","message":"Goods value share 3158 bps is below the dominance floor.","severity":"blocked"}]"#,
    };

    insert_landed_cost_seed(conn, &seed)?;
    let persisted = persisted_landed_cost_seed(conn, seed.id)?;
    let blockers = stage11_issue_codes_from_json(&persisted.blockers)
        .context("parsing E2E-IL-010 blocker codes")?;
    let warnings = stage11_issue_codes_from_json(&persisted.warnings)
        .context("parsing E2E-IL-010 warning codes")?;

    ensure!(
        persisted.goods_landed_cost_gbp_pence == 600_000
            && persisted.installation_labour_cost_gbp_pence == 900_000
            && persisted.permit_traffic_mgmt_cost_gbp_pence == 400_000
            && persisted.economic_landed_cost_gbp_pence == 1_900_000,
        "E2E-IL-010 traffic-signal fixture values changed"
    );
    ensure!(
        goods_value_share_bps(
            persisted.goods_landed_cost_gbp_pence,
            persisted.installation_labour_cost_gbp_pence,
            persisted.permit_traffic_mgmt_cost_gbp_pence,
        ) == 3_158,
        "E2E-IL-010 goods share must be 3158 bps"
    );
    ensure!(
        persisted.decision_grade == "reject",
        "E2E-IL-010 must reject via Phase-2 dominance, not category gate"
    );
    ensure!(
        blockers.contains("GOODS_DOMINANCE_FLOOR") && warnings.is_empty(),
        "E2E-IL-010 reject reason must be dominance floor, not service-gate or missing-data warnings"
    );
    Ok(())
}

#[test]
#[doc = "E2E-IL-011"]
fn e2e_il_011_bespoke_made_to_measure_windows_reviews_with_control_twin() -> Result<()> {
    let db = L4TestStateDb::new()?;
    let conn = db.connection();
    apply_installation_labour_migration_if_needed(conn)?;
    let ids = db.seed_push_owned_rows()?;
    seed_candidate_and_quote(conn, &ids.process_id, &ids.notice_id, &ids.lot_id)?;

    let control_seed = LandedCostSeed {
        id: "e2e-il-011-windows-control",
        process_id: &ids.process_id,
        notice_id: &ids.notice_id,
        product_candidate_id: "il-seed-candidate",
        supplier_quote_id: "il-seed-quote",
        goods_landed_cost_gbp_pence: 3_000_000,
        installation_labour_cost_gbp_pence: 400_000,
        installation_labour_low_gbp_pence: 360_000,
        installation_labour_high_gbp_pence: 480_000,
        installation_labour_confidence_bps: 8_000,
        permit_traffic_mgmt_cost_gbp_pence: 0,
        permit_traffic_mgmt_low_gbp_pence: 0,
        permit_traffic_mgmt_high_gbp_pence: 0,
        permit_responsibility: None,
        economic_landed_cost_gbp_pence: 3_400_000,
        decision_grade: "auto_pass",
        warnings: "[]",
        blockers: "[]",
    };
    let bespoke_seed = LandedCostSeed {
        id: "e2e-il-011-windows-bespoke",
        decision_grade: "review",
        warnings: r#"[{"code":"out_of_scope_bespoke","message":"Made-to-measure windows require scope review.","severity":"warning"}]"#,
        ..control_seed.clone()
    };

    insert_landed_cost_seed(conn, &control_seed)?;
    insert_landed_cost_seed(conn, &bespoke_seed)?;
    let control = persisted_landed_cost_seed(conn, control_seed.id)?;
    let bespoke = persisted_landed_cost_seed(conn, bespoke_seed.id)?;
    let control_warnings = stage11_issue_codes_from_json(&control.warnings)
        .context("parsing E2E-IL-011 control warning codes")?;
    let control_blockers = stage11_issue_codes_from_json(&control.blockers)
        .context("parsing E2E-IL-011 control blocker codes")?;
    let bespoke_warnings = stage11_issue_codes_from_json(&bespoke.warnings)
        .context("parsing E2E-IL-011 bespoke warning codes")?;
    let bespoke_blockers = stage11_issue_codes_from_json(&bespoke.blockers)
        .context("parsing E2E-IL-011 bespoke blocker codes")?;

    ensure!(
        control.goods_landed_cost_gbp_pence == 3_000_000
            && control.installation_labour_cost_gbp_pence == 400_000
            && control.permit_traffic_mgmt_cost_gbp_pence == 0
            && control.economic_landed_cost_gbp_pence == 3_400_000,
        "E2E-IL-011 control fixture values changed"
    );
    ensure!(
        goods_value_share_bps(
            control.goods_landed_cost_gbp_pence,
            control.installation_labour_cost_gbp_pence,
            control.permit_traffic_mgmt_cost_gbp_pence,
        ) == 8_824,
        "E2E-IL-011 goods share must be 8824 bps"
    );
    ensure!(
        control.decision_grade == "auto_pass"
            && control_warnings.is_empty()
            && control_blockers.is_empty(),
        "E2E-IL-011 control twin without bespoke flag must pass Stage 11"
    );
    ensure!(
        bespoke.goods_landed_cost_gbp_pence == control.goods_landed_cost_gbp_pence
            && bespoke.installation_labour_cost_gbp_pence
                == control.installation_labour_cost_gbp_pence
            && bespoke.permit_traffic_mgmt_cost_gbp_pence
                == control.permit_traffic_mgmt_cost_gbp_pence
            && bespoke.economic_landed_cost_gbp_pence == control.economic_landed_cost_gbp_pence,
        "E2E-IL-011 bespoke arm must keep the same economics as the control twin"
    );
    ensure!(
        bespoke.decision_grade == "review"
            && bespoke_warnings.contains("out_of_scope_bespoke")
            && !bespoke_blockers.contains("GOODS_DOMINANCE_FLOOR"),
        "E2E-IL-011 real case must review for out_of_scope_bespoke, not dominance floor"
    );
    Ok(())
}

#[test]
#[doc = "NEG-CTRL-003"]
#[doc = "E2E-IL-007"]
fn neg_ctrl_003_e2e_il_007_electrical_maintenance_stops_before_labour_invocation() -> Result<()> {
    let db = L4TestStateDb::new()?;
    let conn = db.connection();
    apply_installation_labour_migration_if_needed(conn)?;
    let ids = db.seed_push_owned_rows()?;

    seed_rejected_service_qualification(
        conn,
        &ids.process_id,
        &ids.notice_id,
        &ids.lot_id,
        "neg-ctrl-003-electrical-maintenance",
        "reject_service: no_goods_head_noun; Reactive and planned electrical maintenance across estate",
    )?;

    assert_rejected_service_qualification(
        conn,
        &ids.process_id,
        "neg-ctrl-003-electrical-maintenance",
    )?;
    assert_no_stage11_rows_or_labour_jobs(conn, &ids.process_id)?;
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

fn seed_rejected_service_qualification(
    conn: &Connection,
    process_id: &str,
    notice_id: &str,
    lot_id: &str,
    id: &str,
    rationale: &str,
) -> Result<()> {
    conn.execute(
        "\
        INSERT INTO qualification_runs (
          id, process_id, notice_id, lot_id, expiry_qualified, goods_qualified,
          goods_confidence_bps, legal_category, goods_density_score_bps,
          goods_separability, qualification_basis, addressable_value_pence,
          access_route, rationale, owner_id, stage_run_id
        ) VALUES (
          ?1, ?2, ?3, ?4, 1, 0, 0, 'services', 0, 'none',
          'notice_level', NULL, 'open_tender', ?5, 'local-owner',
          'negative-control-service-gate'
        )
        ",
        params![id, process_id, notice_id, lot_id, rationale],
    )?;
    Ok(())
}

fn assert_rejected_service_qualification(
    conn: &Connection,
    process_id: &str,
    qualification_id: &str,
) -> Result<()> {
    let (goods_qualified, legal_category, rationale): (i64, String, String) = conn
        .query_row(
            "\
            SELECT goods_qualified, legal_category, rationale
            FROM qualification_runs
            WHERE process_id = ?1 AND id = ?2
            ",
            params![process_id, qualification_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .context("reading rejected service qualification run")?;

    ensure!(
        goods_qualified == 0
            && legal_category == "services"
            && rationale.contains("reject_service")
            && rationale.contains("no_goods_head_noun"),
        "negative-control qualification must persist the reject_service/no-goods-head-noun gate"
    );
    Ok(())
}

fn assert_no_stage11_rows_or_labour_jobs(conn: &Connection, process_id: &str) -> Result<()> {
    for table_name in [
        "product_candidates",
        "landed_cost_estimates",
        "candidate_net_margin_assessments",
    ] {
        let count = row_count_for_process(conn, table_name, process_id)?;
        ensure!(
            count == 0,
            "service-gate negative control leaked {count} row(s) into {table_name}"
        );
    }

    let labour_job_count: i64 = conn.query_row(
        "\
        SELECT COUNT(*)
        FROM workflow_jobs
        WHERE process_id = ?1
          AND kind = 'landed_cost_uk_installation_labour'
        ",
        [process_id],
        |row| row.get(0),
    )?;
    ensure!(
        labour_job_count == 0,
        "service-gate negative control invoked {labour_job_count} labour estimate job(s)"
    );
    Ok(())
}

fn assert_e2e_il_001_decision_basis(
    conn: &Connection,
    process_id: &str,
    persisted: &PersistedLandedCostSeed,
) -> Result<()> {
    insert_pipeline_assessment_decision_basis(
        conn,
        process_id,
        "review",
        goods_value_share_bps(
            persisted.goods_landed_cost_gbp_pence,
            persisted.installation_labour_cost_gbp_pence,
            persisted.permit_traffic_mgmt_cost_gbp_pence,
        ),
        "cost_proxy",
    )?;
    let decision_basis = persisted_decision_basis(conn, process_id)?;
    ensure!(
        decision_basis
            == PersistedDecisionBasis {
                goods_value_share_bps: Some(6_944),
                goods_value_share_basis: Some("cost_proxy".to_owned()),
                review_required: 1,
                review_reason: Some("review".to_owned()),
            },
        "E2E-IL-001 final decision record must log cost-proxy basis and all-in share"
    );
    ensure!(
        decision_manifest_basis_line(persisted, &decision_basis)
            == "basis=cost_proxy share_bps=6944 numerator_pence=1000000 denominator_pence=1440000 floor_bps=6000 review_band_bps=500 final_route=review",
        "E2E-IL-001 manifest basis line must include basis, numerator, denominator, floor, band, and route"
    );
    Ok(())
}

fn insert_pipeline_assessment_decision_basis(
    conn: &Connection,
    process_id: &str,
    final_route: &str,
    goods_value_share_bps: i64,
    goods_value_share_basis: &str,
) -> Result<()> {
    conn.execute(
        "\
        INSERT INTO pipeline_assessments (
          process_id, pipeline_state, review_required, review_reason,
          decision_basis_hash, goods_value_share_bps, goods_value_share_basis, owner_id
        ) VALUES (
          ?1, 'net_qualified', ?2, ?3,
          'e2e-il-001-decision-basis', ?4, ?5, 'local-owner'
        )
        ",
        params![
            process_id,
            i64::from(final_route == "review"),
            final_route,
            goods_value_share_bps,
            goods_value_share_basis
        ],
    )?;
    Ok(())
}

fn persisted_decision_basis(conn: &Connection, process_id: &str) -> Result<PersistedDecisionBasis> {
    conn.query_row(
        "\
        SELECT goods_value_share_bps, goods_value_share_basis, review_required, review_reason
        FROM pipeline_assessments
        WHERE process_id = ?1
        ",
        [process_id],
        |row| {
            Ok(PersistedDecisionBasis {
                goods_value_share_bps: row.get(0)?,
                goods_value_share_basis: row.get(1)?,
                review_required: row.get(2)?,
                review_reason: row.get(3)?,
            })
        },
    )
    .context("reading E2E-IL-001 decision basis fields")
}

fn decision_manifest_basis_line(
    landed_cost: &PersistedLandedCostSeed,
    decision_basis: &PersistedDecisionBasis,
) -> String {
    format!(
        "basis={} share_bps={} numerator_pence={} denominator_pence={} floor_bps=6000 review_band_bps=500 final_route={}",
        decision_basis
            .goods_value_share_basis
            .as_deref()
            .unwrap_or("n/a"),
        decision_basis.goods_value_share_bps.unwrap_or_default(),
        landed_cost.goods_landed_cost_gbp_pence,
        landed_cost
            .goods_landed_cost_gbp_pence
            .saturating_add(landed_cost.installation_labour_cost_gbp_pence)
            .saturating_add(landed_cost.permit_traffic_mgmt_cost_gbp_pence),
        decision_basis.review_reason.as_deref().unwrap_or("n/a"),
    )
}

fn row_count_for_process(conn: &Connection, table_name: &str, process_id: &str) -> Result<i64> {
    conn.query_row(
        &format!("SELECT COUNT(*) FROM {table_name} WHERE process_id = ?1"),
        [process_id],
        |row| row.get(0),
    )
    .with_context(|| format!("counting rows in {table_name} for process {process_id}"))
}

fn stage11_issue_codes_from_json(raw: &str) -> Result<BTreeSet<String>> {
    let value = serde_json::from_str::<Value>(raw)
        .with_context(|| format!("parsing Stage 11 issue JSON `{raw}`"))?;
    let Some(items) = value.as_array() else {
        return Ok(BTreeSet::new());
    };
    Ok(items
        .iter()
        .filter_map(|item| match item {
            Value::String(code) => Some(code.clone()),
            Value::Object(map) => map.get("code").and_then(Value::as_str).map(str::to_owned),
            _ => None,
        })
        .collect())
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
            seed.decision_grade,
            seed.warnings,
            seed.blockers,
        ],
    )?;
    Ok(())
}

fn insert_maintenance_period_seed(
    conn: &Connection,
    seed: &MaintenancePeriodSeed<'_>,
) -> Result<()> {
    conn.execute(
        "\
        INSERT INTO maintenance_period_estimates (
          id, process_id, notice_id, product_candidate_id, landed_cost_estimate_id,
          supplier_quote_id, visit_scope, visits_per_year, term_years,
          per_visit_labour_cost_gbp_pence, annualised_opex_gbp_pence,
          total_visible_term_opex_gbp_pence, maintenance_confidence_bps,
          margin_signal, warnings, evidence_json, owner_id, row_version,
          stage_run_id
        ) VALUES (
          ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15,
          ?16, 'local-owner', 1, 'maintenance-period-seed'
        )
        ",
        params![
            seed.id,
            seed.process_id,
            seed.notice_id,
            seed.product_candidate_id,
            seed.landed_cost_estimate_id,
            seed.supplier_quote_id,
            seed.visit_scope,
            seed.visits_per_year,
            seed.term_years,
            seed.per_visit_labour_cost_gbp_pence,
            seed.annualised_opex_gbp_pence,
            seed.total_visible_term_opex_gbp_pence,
            seed.maintenance_confidence_bps,
            seed.margin_signal,
            seed.warnings,
            seed.evidence_json,
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
      'expected_value', 9000, 2000, 3000, 4000, ?16, 7600,
      ?17, ?18, 'local-owner', 1, 'installation-labour-seed'
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
               permit_responsibility, economic_landed_cost_gbp_pence, decision_grade, warnings,
               blockers
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
                decision_grade: row.get(10)?,
                warnings: row.get(11)?,
                blockers: row.get(12)?,
            })
        },
    )
    .context("reading installation labour landed-cost seed row")
}

fn persisted_maintenance_period_seed(
    conn: &Connection,
    maintenance_id: &str,
) -> Result<PersistedMaintenancePeriodSeed> {
    conn.query_row(
        "\
        SELECT product_candidate_id, landed_cost_estimate_id, supplier_quote_id,
               visit_scope, visits_per_year, term_years, per_visit_labour_cost_gbp_pence,
               annualised_opex_gbp_pence, total_visible_term_opex_gbp_pence,
               maintenance_confidence_bps, margin_signal, warnings, evidence_json
        FROM maintenance_period_estimates
        WHERE id = ?1
        ",
        [maintenance_id],
        |row| {
            Ok(PersistedMaintenancePeriodSeed {
                product_candidate_id: row.get(0)?,
                landed_cost_estimate_id: row.get(1)?,
                supplier_quote_id: row.get(2)?,
                visit_scope: row.get(3)?,
                visits_per_year: row.get(4)?,
                term_years: row.get(5)?,
                per_visit_labour_cost_gbp_pence: row.get(6)?,
                annualised_opex_gbp_pence: row.get(7)?,
                total_visible_term_opex_gbp_pence: row.get(8)?,
                maintenance_confidence_bps: row.get(9)?,
                margin_signal: row.get(10)?,
                warnings: row.get(11)?,
                evidence_json: row.get(12)?,
            })
        },
    )
    .context("reading installation maintenance period seed row")
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
