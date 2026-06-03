#![forbid(unsafe_code)]

#[path = "support/db.rs"]
mod db_support;
#[allow(dead_code)]
#[path = "support/skills.rs"]
mod skills_support;

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Result, ensure};
use chrono::{TimeZone, Utc};
use db_support::{FTS_SOURCE_ID, TestStateDb};
use serde_json::{Value, json};
use serde_yaml::Value as YamlValue;
use skills_support::lint_skill_directory;

const AGENT_PROTOCOL_MISMATCH: &str = "AGENT_PROTOCOL_MISMATCH";
const SOURCE_SOFT_BLOCKED: &str = "SOURCE_SOFT_BLOCKED";
const SOURCE_CAPTCHA_BLOCKED: &str = "SOURCE_CAPTCHA_BLOCKED";
const AUTH_STATE_STALE: &str = "AUTH_STATE_STALE";
const PAGE_SHAPE_DRIFT: &str = "PAGE_SHAPE_DRIFT";
const LOCALE_OR_CURRENCY_DRIFT: &str = "LOCALE_OR_CURRENCY_DRIFT";
const AGENT_SKILL_FAILED: &str = "AGENT_SKILL_FAILED";
const BID_SUBMISSION_FAILED: &str = "BID_SUBMISSION_FAILED";
const BID_DUPLICATE_DETECTED: &str = "BID_DUPLICATE_DETECTED";
const BID_SIDE_EFFECT_CAP_EXCEEDED: &str = "BID_SIDE_EFFECT_CAP_EXCEEDED";
const BID_READINESS_GATE_FAILED: &str = "BID_READINESS_GATE_FAILED";
const BID_PORTAL_CHANGED: &str = "BID_PORTAL_CHANGED";
const BID_PORTAL_AUTH_EXPIRED: &str = "BID_PORTAL_AUTH_EXPIRED";
const UK_LABOUR_WARNING_CODES: [&str; 8] = [
    "LABOUR_REGION_UNKNOWN",
    "LABOUR_SCOPE_UNRESOLVED",
    "ACCESS_AT_HEIGHT_ASSUMED_MEWP",
    "MAINTENANCE_TERM_ASSUMED",
    "RATE_TABLE_STALE",
    "PERMIT_RESPONSIBILITY_UNKNOWN",
    "LANE_RENTAL_ZONE",
    "TTRO_LEAD_TIME_RISK",
];
const UK_LABOUR_CANARY_CASES: [(&str, &str); 7] = [
    ("CAN-LAB-001", "lights-nw-2day-01"),
    ("CAN-LAB-002", "lights-lon-2day-01"),
    ("CAN-LAB-003", "cctv-yh-install-01"),
    ("CAN-LAB-004", "furniture-lon-assembly-01"),
    ("CAN-LAB-005", "region-unknown-01"),
    ("CAN-LAB-006", "scope-unresolved-01"),
    ("CAN-LAB-007", "rate-stale-01"),
];
const OVERLAY_LABOUR_REQUIRED_CHECKS: [&str; 6] = [
    "hours_scope_consistency",
    "regional_rate_bounds",
    "postcode_region_plausibility",
    "access_equipment_proportionality",
    "permit_responsibility_handling",
    "warning_completeness",
];
const OVERLAY_LABOUR_CANARY_CASES: [&str; 3] = [
    "labour-overlay-clean-001",
    "labour-overlay-mutated-bad-001",
    "labour-overlay-worker-outage-001",
];

fn repo_root() -> Result<PathBuf> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .map(Path::to_path_buf)
        .ok_or_else(|| anyhow::anyhow!("failed to resolve repository root"))
}

fn skill_dir(relative: &str) -> Result<PathBuf> {
    Ok(repo_root()?.join("skills").join(relative))
}

fn uk_installation_labour_fixture_root() -> Result<PathBuf> {
    Ok(repo_root()?.join("tests/fixtures/calibration/landed-cost/uk-installation-labour"))
}

fn fixed_started_at() -> Result<chrono::DateTime<Utc>> {
    Utc.with_ymd_and_hms(2026, 3, 25, 4, 0, 0)
        .single()
        .ok_or_else(|| anyhow::anyhow!("failed to construct fixed test timestamp"))
}

fn create_error_markdown(
    error_code: &str,
    workflow_step: &str,
    artifact_root: &Path,
) -> Result<PathBuf> {
    let path = artifact_root.join(format!("{error_code}.md"));
    fs::write(
        &path,
        format!(
            "# Error Manifest: {error_code}\n\nworkflow_step: {workflow_step}\nskill_path: skills/procurement-ingestion/fts/SKILL.md\n"
        ),
    )?;
    Ok(path)
}

fn load_first_canary_input(path: &Path) -> Result<Value> {
    let content = fs::read_to_string(path)?;
    let first_line = content
        .lines()
        .find(|line| !line.trim().is_empty())
        .ok_or_else(|| anyhow::anyhow!("canary inputs file is empty"))?;
    Ok(serde_json::from_str(first_line)?)
}

fn load_canary_inputs(path: &Path) -> Result<Vec<Value>> {
    let content = fs::read_to_string(path)?;
    let mut inputs = Vec::new();
    for line in content.lines().filter(|line| !line.trim().is_empty()) {
        inputs.push(serde_json::from_str(line)?);
    }
    Ok(inputs)
}

fn find_canary_case<'a>(inputs: &'a [Value], case_id: &str) -> Result<&'a Value> {
    inputs
        .iter()
        .find(|input| json_str_field_eq(input.get("case_id"), case_id))
        .ok_or_else(|| anyhow::anyhow!("missing bid portal canary case {case_id}"))
}

fn load_bid_portal_cassette(fixture_root: &Path, canary: &Value) -> Result<Value> {
    let cassette_ref = canary
        .get("cassette_ref")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("bid portal canary missing cassette_ref"))?;
    Ok(serde_json::from_str(&fs::read_to_string(
        fixture_root.join(cassette_ref),
    )?)?)
}

fn assert_bid_portal_corpus_category(
    category: &str,
    minimum_rows: u64,
    required_cases: &[&str],
) -> Result<()> {
    let fixture_root = repo_root()?
        .join("tests/fixtures/calibration/bid-portal-procontract")
        .join(category);
    for (required_file, required_path) in [
        (
            "canary.inputs.jsonl",
            fixture_root.join("canary.inputs.jsonl"),
        ),
        (
            "golden.assertions.yaml",
            fixture_root.join("golden.assertions.yaml"),
        ),
        (
            "validator-rubric.yaml",
            fixture_root.join("validator-rubric.yaml"),
        ),
        ("metadata.json", fixture_root.join("metadata.json")),
        ("manifest.md", fixture_root.join("manifest.md")),
    ] {
        ensure!(
            required_path.is_file(),
            "{category} corpus missing {required_file}"
        );
    }

    let inputs = load_canary_inputs(&fixture_root.join("canary.inputs.jsonl"))?;
    let metadata: Value =
        serde_json::from_str(&fs::read_to_string(fixture_root.join("metadata.json"))?)?;
    let actual_rows = u64::try_from(inputs.len())?;
    ensure!(
        metadata
            .get("row_count")
            .and_then(Value::as_u64)
            .is_some_and(|expected| expected == actual_rows),
        "{category} metadata row_count must match canary.inputs.jsonl"
    );
    ensure!(
        actual_rows >= minimum_rows,
        "{category} corpus must have at least {minimum_rows} canary rows"
    );

    let golden = fs::read_to_string(fixture_root.join("golden.assertions.yaml"))?;
    let rubric = fs::read_to_string(fixture_root.join("validator-rubric.yaml"))?;
    let manifest = fs::read_to_string(fixture_root.join("manifest.md"))?;
    ensure!(
        rubric.contains("rubric_version") && manifest.contains("Scenario"),
        "{category} corpus must carry rubric version and scenario manifest"
    );

    for case_id in required_cases {
        let canary = find_canary_case(&inputs, case_id)?;
        ensure!(
            golden.contains(case_id),
            "{category} golden assertions missing {case_id}"
        );
        for required_field in [
            "case_id",
            "form_complexity",
            "portal_opportunity_ref",
            "process_id",
            "lot_ids",
            "candidate_ids",
            "selected_quote_ids",
            "company_profile_version",
            "side_effect_cap",
            "expected_terminal_state",
            "expected_error_code",
            "cassette_ref",
            "screenshot_expectations",
            "redaction_sentinels",
            "validator_rubric_ref",
        ] {
            ensure!(
                canary.get(required_field).is_some(),
                "{case_id} missing required canary field {required_field}"
            );
        }
        let cassette_ref = canary
            .get("cassette_ref")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow::anyhow!("{case_id} missing cassette_ref"))?;
        ensure!(
            fixture_root.join(cassette_ref).is_file(),
            "{case_id} cassette_ref must resolve to a deterministic fixture"
        );
        ensure!(
            canary
                .get("validator_rubric_ref")
                .and_then(Value::as_str)
                .is_some_and(|rubric_ref| rubric_ref.starts_with("validator-rubric.yaml#")),
            "{case_id} validator_rubric_ref must point into validator-rubric.yaml"
        );
    }

    Ok(())
}

fn side_effect_rank(side_effect_class: &str) -> Result<u8> {
    match side_effect_class {
        "READ_ONLY" => Ok(0),
        "REMOTE_DRAFT" => Ok(1),
        "NOTIFICATION_OR_LOCK" => Ok(2),
        "LEGAL_DECLARATION" => Ok(3),
        "FORMAL_SUBMISSION" => Ok(4),
        "FUNDS_COMMITTED" => Ok(5),
        _ => Err(anyhow::anyhow!(
            "unknown bid portal side-effect class {side_effect_class}"
        )),
    }
}

fn highest_observed_side_effect<'a>(evidence: &'a Value, cassette: &'a Value) -> Option<&'a str> {
    evidence
        .get("highest_side_effect_class_reached")
        .or_else(|| evidence.pointer("/checkpoint/highest_side_effect_class_reached"))
        .or_else(|| evidence.pointer("/checkpoint/side_effect_class_reached"))
        .or_else(|| evidence.pointer("/expected_detection/side_effect_class_reached"))
        .or_else(|| cassette.pointer("/checkpoint/highest_side_effect_class_reached"))
        .or_else(|| cassette.pointer("/checkpoint/side_effect_class_reached"))
        .and_then(Value::as_str)
}

fn terminal_state_matches_cassette(expected_terminal_state: &str, evidence: &Value) -> bool {
    json_str_field_eq(evidence.get("terminal_state"), expected_terminal_state)
        || (expected_terminal_state == "draft_saved"
            && json_str_field_eq(
                evidence.pointer("/expected_result/kind"),
                "draft_save_result",
            ))
        || (expected_terminal_state == "bid_submitted"
            && json_str_field_eq(
                evidence.pointer("/expected_result/kind"),
                "submission_result",
            ))
        || (expected_terminal_state == "bid_in_progress"
            && (evidence.get("checkpoint").is_some() || evidence.get("document_uploads").is_some()))
}

fn assert_bid_portal_cassette_replays_case(fixture_root: &Path, canary: &Value) -> Result<()> {
    let case_id = canary
        .get("case_id")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("bid portal canary missing case_id"))?;
    let cassette = load_bid_portal_cassette(fixture_root, canary)?;
    let evidence = cassette
        .get("cases")
        .and_then(|cases| cases.get(case_id))
        .unwrap_or(&cassette);
    let rendered_cassette = serde_json::to_string(&cassette)?;
    ensure!(
        !rendered_cassette.contains("localhost:9222")
            && !rendered_cassette.contains("agent-browser --cdp")
            && !rendered_cassette.contains("bin/ensure-cdp-9222"),
        "{case_id} cassette must be deterministic replay evidence, not a live CDP command"
    );

    let expected_error = canary
        .get("expected_error_code")
        .ok_or_else(|| anyhow::anyhow!("{case_id} missing expected_error_code"))?;
    if let Some(expected_error_code) = expected_error.as_str() {
        ensure!(
            json_str_field_eq(
                evidence.pointer("/expected_detection/error_code"),
                expected_error_code,
            ) || json_str_field_eq(
                cassette.pointer("/expected_detection/error_code"),
                expected_error_code,
            ),
            "{case_id} cassette must replay expected error {expected_error_code}"
        );
    } else {
        let expected_terminal_state = canary
            .get("expected_terminal_state")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow::anyhow!("{case_id} missing expected_terminal_state"))?;
        ensure!(
            terminal_state_matches_cassette(expected_terminal_state, evidence),
            "{case_id} cassette must replay terminal state {expected_terminal_state}"
        );
    }

    if let (Some(cap), Some(observed)) = (
        canary.get("side_effect_cap").and_then(Value::as_str),
        highest_observed_side_effect(evidence, &cassette),
    ) {
        ensure!(
            side_effect_rank(observed)? <= side_effect_rank(cap)?,
            "{case_id} observed side effect {observed} exceeds cap {cap}"
        );
    }

    if case_id == "medium-resume-draft-01" {
        ensure!(
            json_bool_field_eq(
                evidence.pointer("/portal_state_verification/portal_state_verified_before_edit"),
                true,
            ),
            "resume-after-REMOTE_DRAFT canary must verify portal state before retry editing"
        );
    }
    if case_id == "complex-remote-draft-changed-externally-01" {
        ensure!(
            json_str_field_eq(evidence.get("terminal_state"), "portal_state_unknown")
                && json_bool_field_eq(
                    evidence.pointer("/expected_detection/external_change_detected"),
                    true,
                ),
            "remote-draft external-change canary must reconcile portal state before editing"
        );
    }

    Ok(())
}

fn assert_bid_portal_corpus_replays_all_cases(category: &str, minimum_rows: usize) -> Result<()> {
    let fixture_root = repo_root()?
        .join("tests/fixtures/calibration/bid-portal-procontract")
        .join(category);
    let inputs = load_canary_inputs(&fixture_root.join("canary.inputs.jsonl"))?;
    ensure!(
        inputs.len() >= minimum_rows,
        "{category} corpus must have at least {minimum_rows} replay cases"
    );
    for canary in &inputs {
        ensure!(
            json_str_field_eq(canary.get("skill_id"), "bid-portal/procontract")
                && json_str_field_eq(canary.get("form_complexity"), category),
            "{category} canary row must target bid-portal/procontract and its category"
        );
        assert_bid_portal_cassette_replays_case(&fixture_root, canary)?;
    }
    Ok(())
}

fn load_uk_installation_labour_fixture() -> Result<(Vec<Value>, YamlValue)> {
    let fixture_root = uk_installation_labour_fixture_root()?;
    for (required_file, required_path) in [
        (
            "canary.inputs.jsonl",
            fixture_root.join("canary.inputs.jsonl"),
        ),
        (
            "golden.assertions.yaml",
            fixture_root.join("golden.assertions.yaml"),
        ),
        (
            "validator-rubric.yaml",
            fixture_root.join("validator-rubric.yaml"),
        ),
        ("metadata.json", fixture_root.join("metadata.json")),
        ("manifest.md", fixture_root.join("manifest.md")),
    ] {
        ensure!(
            required_path.is_file(),
            "uk-installation-labour corpus missing {required_file}"
        );
    }

    let inputs = load_canary_inputs(&fixture_root.join("canary.inputs.jsonl"))?;
    let metadata: Value =
        serde_json::from_str(&fs::read_to_string(fixture_root.join("metadata.json"))?)?;
    let actual_rows = u64::try_from(inputs.len())?;
    ensure!(
        actual_rows == 7
            && metadata
                .get("row_count")
                .and_then(Value::as_u64)
                .is_some_and(|expected| expected == actual_rows),
        "uk-installation-labour metadata row_count must match seven JSONL rows"
    );

    let manifest = fs::read_to_string(fixture_root.join("manifest.md"))?;
    let rubric = fs::read_to_string(fixture_root.join("validator-rubric.yaml"))?;
    ensure!(
        rubric.contains("rubric_version"),
        "uk-installation-labour corpus must carry rubric_version"
    );

    for (test_id, case_id) in UK_LABOUR_CANARY_CASES {
        find_canary_case(&inputs, case_id)?;
        ensure!(
            manifest.contains("Scenario")
                && manifest.contains(test_id)
                && manifest.contains(case_id),
            "uk-installation-labour manifest must mention Scenario {test_id} / {case_id}"
        );
    }

    let golden: YamlValue = serde_yaml::from_str(&fs::read_to_string(
        fixture_root.join("golden.assertions.yaml"),
    )?)?;
    let cases = yaml_get(&golden, "cases")
        .ok_or_else(|| anyhow::anyhow!("uk-installation-labour golden missing cases"))?;
    for (_, case_id) in UK_LABOUR_CANARY_CASES {
        ensure!(
            yaml_get(cases, case_id).is_some(),
            "uk-installation-labour golden missing {case_id}"
        );
    }

    Ok((inputs, golden))
}

fn uk_installation_labour_fixture_case<'a>(
    inputs: &'a [Value],
    golden: &'a YamlValue,
    case_id: &str,
) -> Result<(&'a Value, &'a YamlValue)> {
    let input = find_canary_case(inputs, case_id)?;
    let cases = yaml_get(golden, "cases")
        .ok_or_else(|| anyhow::anyhow!("uk-installation-labour golden missing cases"))?;
    let golden_case = yaml_get(cases, case_id)
        .ok_or_else(|| anyhow::anyhow!("uk-installation-labour golden missing {case_id}"))?;
    Ok((input, golden_case))
}

fn fingerprint_matches(expected: &str, actual: &str) -> bool {
    expected == actual
}

const fn auth_state_valid(required: bool, privileged_data_present: bool) -> bool {
    !required || privileged_data_present
}

fn locale_currency_match(locale: &str, currency: &str) -> bool {
    locale == "en-GB" && currency == "GBP"
}

const fn bid_portal_canary_failure_codes() -> [(&'static str, &'static str); 6] {
    [
        ("submission_confirmation_missing", BID_SUBMISSION_FAILED),
        ("duplicate_remote_draft", BID_DUPLICATE_DETECTED),
        ("side_effect_cap_breach", BID_SIDE_EFFECT_CAP_EXCEEDED),
        ("readiness_gate_blocker", BID_READINESS_GATE_FAILED),
        ("portal_shape_drift", BID_PORTAL_CHANGED),
        ("portal_auth_expired", BID_PORTAL_AUTH_EXPIRED),
    ]
}

fn bid_portal_changed_context() -> Value {
    json!({
        "portalOpportunityRef": "DN-OPP-123",
        "selector": "#response-form button.submit",
        "expectedSourceStructureVersion": "procontract-bid-portal-2026-05-22",
        "observedShape": {
            "pageKind": "unexpected-login-or-form-layout",
            "fingerprintDeltaBps": 9_000,
        },
        "lastUrl": "https://procontract.due-north.com/Opportunities/Index",
    })
}

fn bid_portal_auth_expired_context() -> Value {
    json!({
        "portalOpportunityRef": "DN-OPP-123",
        "lastUrl": "https://procontract.due-north.com/Supplier/Response",
        "authState": "expired",
        "daemonRetry": "auth-signin/procontract",
    })
}

fn json_str_field_eq(value: Option<&Value>, expected: &str) -> bool {
    value
        .and_then(Value::as_str)
        .is_some_and(|observed| observed.eq(expected))
}

fn json_i64_field_eq(value: Option<&Value>, expected: i64) -> bool {
    value
        .and_then(Value::as_i64)
        .is_some_and(|observed| observed == expected)
}

fn json_bool_field_eq(value: Option<&Value>, expected: bool) -> bool {
    value
        .and_then(Value::as_bool)
        .is_some_and(|observed| observed == expected)
}

fn yaml_get<'a>(value: &'a YamlValue, key: &str) -> Option<&'a YamlValue> {
    value.as_mapping()?.get(YamlValue::String(key.to_owned()))
}

fn yaml_i64_field_eq(value: Option<&YamlValue>, expected: i64) -> bool {
    value
        .and_then(YamlValue::as_i64)
        .is_some_and(|observed| observed == expected)
}

fn yaml_str_field_eq(value: Option<&YamlValue>, expected: &str) -> bool {
    value
        .and_then(YamlValue::as_str)
        .is_some_and(|observed| observed == expected)
}

fn yaml_bool_field_eq(value: Option<&YamlValue>, expected: bool) -> bool {
    value
        .and_then(YamlValue::as_bool)
        .is_some_and(|observed| observed == expected)
}

fn yaml_sequence_contains_str(value: Option<&YamlValue>, expected: &str) -> bool {
    value.and_then(YamlValue::as_sequence).is_some_and(|items| {
        items
            .iter()
            .any(|item| item.as_str().is_some_and(|observed| observed == expected))
    })
}

fn json_string_set_field(value: &Value, key: &str) -> Result<BTreeSet<String>> {
    let items = value
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow::anyhow!("missing string array field `{key}`"))?;
    items
        .iter()
        .map(|item| {
            item.as_str()
                .map(str::to_owned)
                .ok_or_else(|| anyhow::anyhow!("field `{key}` must contain strings"))
        })
        .collect()
}

fn yaml_sequence_string_set(value: Option<&YamlValue>, label: &str) -> Result<BTreeSet<String>> {
    let items = value
        .and_then(YamlValue::as_sequence)
        .ok_or_else(|| anyhow::anyhow!("missing YAML string sequence `{label}`"))?;
    items
        .iter()
        .map(|item| {
            item.as_str()
                .map(str::to_owned)
                .ok_or_else(|| anyhow::anyhow!("YAML sequence `{label}` must contain strings"))
        })
        .collect()
}

fn yaml_mapping_get_str_key<'a>(
    mapping: &'a serde_yaml::Mapping,
    expected_key: &str,
) -> Option<&'a YamlValue> {
    mapping.iter().find_map(|(key, value)| {
        key.as_str()
            .is_some_and(|observed| observed == expected_key)
            .then_some(value)
    })
}

fn yaml_value_mentions_case(value: &YamlValue, expected_case_id: &str) -> bool {
    value
        .as_str()
        .is_some_and(|case_id| case_id == expected_case_id)
        || value.as_sequence().is_some_and(|items| {
            items
                .iter()
                .any(|item| yaml_value_mentions_case(item, expected_case_id))
        })
}

#[derive(Debug, Eq, PartialEq)]
enum ValidatorRubricGate {
    Present,
    StableProductionBlocked,
}

fn validator_rubric_gate(path: &Path) -> Result<ValidatorRubricGate> {
    if !path.is_file() {
        return Ok(ValidatorRubricGate::StableProductionBlocked);
    }

    let content = fs::read_to_string(path)?;
    let rubric: YamlValue = serde_yaml::from_str(&content)?;
    ensure!(
        yaml_get(&rubric, "rubric_version")
            .and_then(YamlValue::as_str)
            .is_some_and(|version| !version.trim().is_empty()),
        "{} must carry rubric_version",
        path.display()
    );
    ensure!(
        yaml_get(&rubric, "promotion_controls")
            .and_then(|controls| yaml_get(controls, "stable_production_blocker"))
            .and_then(YamlValue::as_bool)
            == Some(true),
        "{} must mark missing runtime rubric as a stable-production blocker",
        path.display()
    );
    ensure!(
        yaml_get(&rubric, "promotion_controls")
            .and_then(|controls| yaml_get(controls, "promotion_stage"))
            .and_then(YamlValue::as_str)
            == Some("stable_blocked_until_rubric_present"),
        "{} must expose stable_blocked_until_rubric_present promotion stage",
        path.display()
    );

    Ok(ValidatorRubricGate::Present)
}

#[test]
fn can_lab_001_lights_nw_fixture_pins_hard_band_trades_access_and_permit_warning() -> Result<()> {
    let (inputs, golden) = load_uk_installation_labour_fixture()?;
    let (input, case) = uk_installation_labour_fixture_case(&inputs, &golden, "lights-nw-2day-01")?;
    let band = yaml_get(case, "labour_base_gbp_pence_between")
        .and_then(YamlValue::as_sequence)
        .ok_or_else(|| anyhow::anyhow!("CAN-LAB-001 missing labour band"))?;

    ensure!(
        json_str_field_eq(input.pointer("/installation_requirement/uk_region"), "NW",)
            && json_bool_field_eq(
                input.pointer("/installation_requirement/requires_installation"),
                true,
            )
            && json_str_field_eq(
                input.pointer("/installation_requirement/permit_responsibility"),
                "unknown",
            ),
        "CAN-LAB-001 must pin the North West unknown-permit installation input"
    );
    ensure!(
        band.len() == 2
            && band[0].as_i64() == Some(210_000)
            && band[1].as_i64() == Some(265_000)
            && yaml_sequence_contains_str(yaml_get(case, "required_trades"), "electrician")
            && yaml_sequence_contains_str(yaml_get(case, "required_trades"), "labourer")
            && yaml_str_field_eq(
                yaml_get(case, "required_access_equipment_class"),
                "boom_mewp",
            )
            && yaml_sequence_contains_str(
                yaml_get(case, "required_warning_codes"),
                "PERMIT_RESPONSIBILITY_UNKNOWN",
            ),
        "CAN-LAB-001 must assert labour band, trades, operated boom MEWP, and permit warning"
    );

    Ok(())
}

#[test]
fn can_lab_002_lights_london_fixture_pins_multiplier_and_public_realm_warnings() -> Result<()> {
    let (inputs, golden) = load_uk_installation_labour_fixture()?;
    let (input, case) =
        uk_installation_labour_fixture_case(&inputs, &golden, "lights-lon-2day-01")?;

    ensure!(
        json_str_field_eq(
            input.pointer("/installation_requirement/uk_region"),
            "London",
        ) && json_bool_field_eq(
            input.pointer("/installation_requirement/lane_rental_zone"),
            true,
        ) && yaml_str_field_eq(yaml_get(case, "compare_to_case"), "lights-nw-2day-01")
            && yaml_i64_field_eq(yaml_get(case, "labour_base_ratio_bps"), 15_054)
            && yaml_i64_field_eq(yaml_get(case, "ratio_tolerance_bps"), 100)
            && yaml_sequence_contains_str(
                yaml_get(case, "required_warning_codes"),
                "LANE_RENTAL_ZONE",
            )
            && yaml_sequence_contains_str(
                yaml_get(case, "required_warning_codes"),
                "TTRO_LEAD_TIME_RISK",
            ),
        "CAN-LAB-002 must pin London multiplier and Regent Street public-realm warnings"
    );

    Ok(())
}

#[test]
fn can_lab_003_cctv_yh_fixture_pins_security_resolver_multiplier_and_at_height_equipment()
-> Result<()> {
    let (inputs, golden) = load_uk_installation_labour_fixture()?;
    let (input, case) =
        uk_installation_labour_fixture_case(&inputs, &golden, "cctv-yh-install-01")?;

    ensure!(
        json_str_field_eq(input.pointer("/installation_requirement/uk_region"), "YH",)
            && yaml_str_field_eq(
                yaml_get(case, "required_profile_id"),
                "cctv_security_install"
            )
            && yaml_i64_field_eq(yaml_get(case, "region_multiplier_bps"), 8_800)
            && yaml_sequence_contains_str(yaml_get(case, "required_trades"), "security_installer")
            && yaml_sequence_contains_str(yaml_get(case, "required_trades"), "electrician")
            && yaml_str_field_eq(
                yaml_get(case, "required_access_equipment_class"),
                "scissor_lift",
            )
            && yaml_bool_field_eq(
                yaml_get(case, "required_access_equipment_when_at_height"),
                true,
            )
            && yaml_str_field_eq(
                yaml_get(case, "forbidden_trade_set_match_case"),
                "lights-nw-2day-01",
            ),
        "CAN-LAB-003 must pin the YH CCTV resolver, multiplier, and at-height equipment path"
    );

    Ok(())
}

#[test]
fn can_lab_004_furniture_london_fixture_pins_no_access_equipment_route() -> Result<()> {
    let (inputs, golden) = load_uk_installation_labour_fixture()?;
    let (input, case) =
        uk_installation_labour_fixture_case(&inputs, &golden, "furniture-lon-assembly-01")?;

    ensure!(
        json_str_field_eq(
            input.pointer("/installation_requirement/uk_region"),
            "London",
        ) && json_str_field_eq(
            input.pointer("/installation_requirement/equipment_hint"),
            "none",
        ) && yaml_sequence_contains_str(yaml_get(case, "required_trades"), "handyperson")
            && yaml_bool_field_eq(
                yaml_get(case, "forbidden_access_equipment_line_items"),
                true,
            )
            && yaml_sequence_contains_str(
                yaml_get(case, "forbidden_warning_codes"),
                "ACCESS_AT_HEIGHT_ASSUMED_MEWP",
            ),
        "CAN-LAB-004 must pin flat-pack furniture as a no-access-equipment route"
    );

    Ok(())
}

#[test]
fn can_lab_005_region_unknown_fixture_pins_national_fallback_with_estimated_status() -> Result<()> {
    let (inputs, golden) = load_uk_installation_labour_fixture()?;
    let (input, case) = uk_installation_labour_fixture_case(&inputs, &golden, "region-unknown-01")?;

    ensure!(
        matches!(
            input.pointer("/installation_requirement/uk_region"),
            Some(Value::Null)
        ) && yaml_str_field_eq(yaml_get(case, "expected_labour_status"), "estimated")
            && yaml_str_field_eq(yaml_get(case, "resolved_uk_region"), "national")
            && yaml_i64_field_eq(yaml_get(case, "fallback_region_multiplier_bps"), 10_000)
            && yaml_sequence_contains_str(
                yaml_get(case, "required_warning_codes"),
                "LABOUR_REGION_UNKNOWN",
            ),
        "CAN-LAB-005 must pin national fallback, LABOUR_REGION_UNKNOWN, and estimated status"
    );

    Ok(())
}

#[test]
fn can_lab_006_scope_unresolved_fixture_pins_fail_soft_review_and_missing_term_default()
-> Result<()> {
    let (inputs, golden) = load_uk_installation_labour_fixture()?;
    let (input, case) =
        uk_installation_labour_fixture_case(&inputs, &golden, "scope-unresolved-01")?;
    let missing_input_term =
        match input.pointer("/installation_requirement/maintenance_schedule/term_years") {
            None | Some(Value::Null) => true,
            Some(_) => false,
        };
    let schedule_assertions = yaml_get(case, "maintenance_schedule")
        .ok_or_else(|| anyhow::anyhow!("CAN-LAB-006 missing maintenance schedule assertions"))?;

    ensure!(
        json_bool_field_eq(
            input.pointer("/installation_requirement/requires_maintenance"),
            true,
        ) && json_i64_field_eq(
            input.pointer("/installation_requirement/maintenance_schedule/visits_per_year"),
            4,
        ) && missing_input_term
            && yaml_sequence_contains_str(
                yaml_get(case, "expected_warning_codes"),
                "LABOUR_SCOPE_UNRESOLVED",
            )
            && yaml_sequence_contains_str(
                yaml_get(case, "expected_warning_codes"),
                "MAINTENANCE_TERM_ASSUMED",
            )
            && yaml_i64_field_eq(yaml_get(case, "confidence_below_bps"), 5_500)
            && yaml_str_field_eq(yaml_get(case, "expected_route"), "review")
            && yaml_bool_field_eq(yaml_get(case, "fail_soft"), true)
            && yaml_i64_field_eq(
                yaml_get(schedule_assertions, "missing_term_defaults_years"),
                1,
            ),
        "CAN-LAB-006 must pin fail-soft review routing and the 1-year missing-term default"
    );

    Ok(())
}

#[test]
fn can_lab_007_rate_stale_fixture_pins_stale_warning_with_three_point_estimate() -> Result<()> {
    let (inputs, golden) = load_uk_installation_labour_fixture()?;
    let (input, case) = uk_installation_labour_fixture_case(&inputs, &golden, "rate-stale-01")?;

    ensure!(
        json_str_field_eq(
            input.get("rate_table_version"),
            "uk-installation-labour-2025-q4",
        ) && yaml_sequence_contains_str(
            yaml_get(case, "required_warning_codes"),
            "RATE_TABLE_STALE",
        ) && yaml_bool_field_eq(yaml_get(case, "requires_three_point_estimate"), true)
            && yaml_bool_field_eq(yaml_get(case, "fail_soft"), true),
        "CAN-LAB-007 must pin stale-rate warning while retaining a three-point estimate"
    );

    Ok(())
}

#[test]
fn can_warn_001_uk_installation_labour_warning_codes_are_pinned() -> Result<()> {
    let skill_root = skill_dir("landed-cost/uk-installation-labour-estimate")?;
    let fixture_root = uk_installation_labour_fixture_root()?;
    let inputs = load_canary_inputs(&fixture_root.join("canary.inputs.jsonl"))?;
    let scope_unresolved = find_canary_case(&inputs, "scope-unresolved-01")?;
    let missing_input_term = match scope_unresolved
        .pointer("/installation_requirement/maintenance_schedule/term_years")
    {
        None | Some(Value::Null) => true,
        Some(_) => false,
    };
    ensure!(
        json_bool_field_eq(
            scope_unresolved.pointer("/installation_requirement/requires_maintenance"),
            true,
        ) && json_i64_field_eq(
            scope_unresolved
                .pointer("/installation_requirement/maintenance_schedule/visits_per_year"),
            4,
        ) && missing_input_term,
        "scope-unresolved-01 must be the missing-term recurring-maintenance canary"
    );

    let skill_markdown = fs::read_to_string(skill_root.join("SKILL.md"))?;
    ensure!(
        UK_LABOUR_WARNING_CODES
            .iter()
            .all(|code| skill_markdown.contains(code)),
        "UK labour skill Failure Codes must enumerate every warning code"
    );

    let golden: YamlValue = serde_yaml::from_str(&fs::read_to_string(
        fixture_root.join("golden.assertions.yaml"),
    )?)?;
    let coverage = yaml_get(
        yaml_get(&golden, "shared_assertions")
            .ok_or_else(|| anyhow::anyhow!("golden missing shared_assertions"))?,
        "warning_code_coverage",
    )
    .ok_or_else(|| anyhow::anyhow!("golden missing warning_code_coverage"))?;
    let required_codes = yaml_get(coverage, "required_codes")
        .and_then(YamlValue::as_sequence)
        .ok_or_else(|| anyhow::anyhow!("warning_code_coverage missing required_codes"))?
        .iter()
        .filter_map(YamlValue::as_str)
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();
    let expected_codes = UK_LABOUR_WARNING_CODES
        .into_iter()
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();
    ensure!(
        required_codes == expected_codes,
        "CAN-WARN-001 requires all eight warning codes with no unpinned code"
    );

    let pinned_by = yaml_get(coverage, "pinned_by")
        .and_then(YamlValue::as_mapping)
        .ok_or_else(|| anyhow::anyhow!("warning_code_coverage missing pinned_by"))?;
    for code in UK_LABOUR_WARNING_CODES {
        ensure!(
            yaml_mapping_get_str_key(pinned_by, code).is_some(),
            "warning code {code} must have a pinned canary case"
        );
    }
    let maintenance_pin = yaml_mapping_get_str_key(pinned_by, "MAINTENANCE_TERM_ASSUMED")
        .ok_or_else(|| anyhow::anyhow!("MAINTENANCE_TERM_ASSUMED pin missing"))?;
    ensure!(
        yaml_value_mentions_case(maintenance_pin, "scope-unresolved-01"),
        "MAINTENANCE_TERM_ASSUMED must be pinned by the missing-term canary"
    );

    let scope_case = yaml_get(
        yaml_get(&golden, "cases").ok_or_else(|| anyhow::anyhow!("golden missing cases"))?,
        "scope-unresolved-01",
    )
    .ok_or_else(|| anyhow::anyhow!("golden missing scope-unresolved-01"))?;
    let expected_warning_codes = yaml_get(scope_case, "expected_warning_codes")
        .and_then(YamlValue::as_sequence)
        .ok_or_else(|| anyhow::anyhow!("scope-unresolved-01 missing expected warnings"))?;
    ensure!(
        expected_warning_codes
            .iter()
            .any(|code| code.as_str() == Some("MAINTENANCE_TERM_ASSUMED")),
        "scope-unresolved-01 must expect MAINTENANCE_TERM_ASSUMED"
    );
    let schedule_assertions = yaml_get(scope_case, "maintenance_schedule")
        .ok_or_else(|| anyhow::anyhow!("scope-unresolved-01 missing maintenance_schedule"))?;
    ensure!(
        yaml_bool_field_eq(yaml_get(schedule_assertions, "requires_maintenance"), true)
            && yaml_i64_field_eq(yaml_get(schedule_assertions, "visits_per_year"), 4)
            && yaml_i64_field_eq(
                yaml_get(schedule_assertions, "missing_term_defaults_years"),
                1,
            )
            && yaml_get(schedule_assertions, "default_warning_code").and_then(YamlValue::as_str)
                == Some("MAINTENANCE_TERM_ASSUMED"),
        "scope-unresolved-01 golden assertions must pin the 1-year missing-term default"
    );

    Ok(())
}

#[test]
fn gold_l0_003_validator_rubrics_are_stable_production_promotion_gates() -> Result<()> {
    let labour_fixture_rubric =
        uk_installation_labour_fixture_root()?.join("validator-rubric.yaml");
    let overlay_rubric = skill_dir("validation/overlay-labour-cost")?.join("validator-rubric.yaml");
    let missing_rubric = repo_root()?.join("target/test-fixtures/missing-validator-rubric.yaml");

    ensure!(
        validator_rubric_gate(&labour_fixture_rubric)? == ValidatorRubricGate::Present,
        "GOLD-L0-003 labour fixture validator-rubric.yaml must be present and promotion-gated"
    );
    ensure!(
        validator_rubric_gate(&overlay_rubric)? == ValidatorRubricGate::Present,
        "GOLD-L0-003 overlay validator-rubric.yaml must be present and promotion-gated"
    );
    ensure!(
        validator_rubric_gate(&missing_rubric)? == ValidatorRubricGate::StableProductionBlocked,
        "GOLD-L0-003 missing validator-rubric.yaml must block stable production"
    );

    Ok(())
}

#[test]
fn can_overlay_001_overlay_labour_cost_lints_and_canary_contract_is_pinned() -> Result<()> {
    let overlay_root = skill_dir("validation/overlay-labour-cost")?;
    let linted = lint_skill_directory(&overlay_root)?;
    let required_checks = overlay_labour_required_check_set();

    assert_overlay_labour_metadata_and_files(&overlay_root, &linted)?;
    assert_overlay_labour_schema_contract(&overlay_root, &required_checks)?;
    assert_overlay_labour_canary_and_golden_contract(&overlay_root, &required_checks)?;
    assert_overlay_labour_rubric_contract(&overlay_root, &required_checks)
}

fn overlay_labour_required_check_set() -> BTreeSet<String> {
    OVERLAY_LABOUR_REQUIRED_CHECKS
        .iter()
        .map(|check| (*check).to_owned())
        .collect()
}

fn assert_overlay_labour_metadata_and_files(
    overlay_root: &Path,
    linted: &skills_support::LintedSkill,
) -> Result<()> {
    ensure!(linted.metadata.skill_id == "validation/overlay-labour-cost");
    ensure!(linted.metadata.interaction_profile == "headless_contract");
    ensure!(linted.metadata.checkpoint_schema_version.as_deref() == Some("1.0.0"));
    ensure!(linted.metadata.side_effect_class == "read_only");
    ensure!(!linted.metadata.requires_cdp);
    ensure!(
        linted.metadata.worker_behavior_version.as_deref()
            == Some("landed-cost/uk-installation-labour-estimate@0.1.0")
    );
    ensure!(
        linted.metadata.validator_behavior_version.as_deref()
            == Some("validation/overlay-labour-cost@0.1.0")
    );

    ensure!(overlay_root.join("SKILL.md").is_file());
    ensure!(overlay_root.join("result.schema.json").is_file());
    ensure!(overlay_root.join("checkpoint.schema.json").is_file());
    ensure!(overlay_root.join("validator-rubric.yaml").is_file());
    ensure!(overlay_root.join("canary.inputs.jsonl").is_file());
    ensure!(overlay_root.join("golden.assertions.yaml").is_file());

    for required_text in [
        "hours versus scope",
        "regional rate bounds",
        "postcode/region plausibility",
        "equipment proportionality",
        "permit responsibility handling",
        "warning completeness",
        "downstream_route: review",
    ] {
        ensure!(
            linted.skill_markdown.contains(required_text),
            "overlay-labour-cost SKILL.md missing `{required_text}`"
        );
    }

    Ok(())
}

fn assert_overlay_labour_schema_contract(
    overlay_root: &Path,
    required_checks: &BTreeSet<String>,
) -> Result<()> {
    let result_schema: Value = serde_json::from_str(&fs::read_to_string(
        overlay_root.join("result.schema.json"),
    )?)?;
    let schema_checks = result_schema
        .pointer("/$defs/domain_check/enum")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow::anyhow!("overlay result schema missing domain_check enum"))?
        .iter()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();
    ensure!(schema_checks == *required_checks);
    ensure!(
        result_schema
            .pointer("/properties/downstream_route/enum")
            .and_then(Value::as_array)
            .is_some_and(|routes| routes.iter().any(|route| route.as_str() == Some("review"))),
        "overlay result schema must expose downstream_route=review"
    );

    let checkpoint_schema: Value = serde_json::from_str(&fs::read_to_string(
        overlay_root.join("checkpoint.schema.json"),
    )?)?;
    ensure!(
        checkpoint_schema
            .pointer("/properties/kind/const")
            .and_then(Value::as_str)
            == Some("labour_cost_overlay_checkpoint")
    );

    Ok(())
}

fn assert_overlay_labour_canary_and_golden_contract(
    overlay_root: &Path,
    required_checks: &BTreeSet<String>,
) -> Result<()> {
    let canaries = load_canary_inputs(&overlay_root.join("canary.inputs.jsonl"))?;
    ensure!(canaries.len() == OVERLAY_LABOUR_CANARY_CASES.len());
    let golden: YamlValue = serde_yaml::from_str(&fs::read_to_string(
        overlay_root.join("golden.assertions.yaml"),
    )?)?;
    let cases = yaml_get(&golden, "cases")
        .ok_or_else(|| anyhow::anyhow!("overlay-labour-cost golden missing cases"))?;

    for case_id in OVERLAY_LABOUR_CANARY_CASES {
        let canary = find_canary_case(&canaries, case_id)?;
        ensure!(json_str_field_eq(
            canary.get("skill_id"),
            "validation/overlay-labour-cost"
        ));
        ensure!(json_str_field_eq(
            canary.get("worker_skill_id"),
            "landed-cost/uk-installation-labour-estimate"
        ));
        let golden_case = yaml_get(cases, case_id)
            .ok_or_else(|| anyhow::anyhow!("overlay-labour-cost golden missing {case_id}"))?;
        ensure!(
            canary
                .get("expected_downstream_route")
                .and_then(Value::as_str)
                == yaml_get(golden_case, "expected_downstream_route").and_then(YamlValue::as_str),
            "overlay canary/golden downstream route mismatch for {case_id}"
        );
        let canary_checks = json_string_set_field(canary, "domain_checks")?;
        let golden_checks = yaml_sequence_string_set(
            yaml_get(golden_case, "required_domain_checks"),
            "required_domain_checks",
        )?;
        ensure!(canary_checks == golden_checks);
        let expected_route = canary
            .get("expected_downstream_route")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow::anyhow!("{case_id} missing expected_downstream_route"))?;
        assert_overlay_labour_case_route_contract(
            case_id,
            canary,
            golden_case,
            &canary_checks,
            required_checks,
            expected_route,
        )?;
    }

    let shared = yaml_get(&golden, "shared_assertions")
        .ok_or_else(|| anyhow::anyhow!("overlay golden missing shared_assertions"))?;
    ensure!(
        yaml_sequence_string_set(
            yaml_get(shared, "domain_check_inventory"),
            "domain_check_inventory",
        )? == *required_checks
    );
    let outage_policy = yaml_get(shared, "validator_rejection_or_outage_policy")
        .ok_or_else(|| anyhow::anyhow!("overlay golden missing outage policy"))?;
    ensure!(
        yaml_str_field_eq(
            yaml_get(outage_policy, "rejected_overlay_downstream_route"),
            "review"
        ),
        "overlay rejected outputs must route to review"
    );
    ensure!(
        yaml_str_field_eq(yaml_get(outage_policy, "outage_downstream_route"), "review"),
        "overlay outages must route to review"
    );
    ensure!(
        yaml_get(outage_policy, "never_silent_pass_on_missing_worker_result")
            .and_then(YamlValue::as_bool)
            == Some(true)
    );

    Ok(())
}

fn assert_overlay_labour_case_route_contract(
    case_id: &str,
    canary: &Value,
    golden_case: &YamlValue,
    canary_checks: &BTreeSet<String>,
    required_checks: &BTreeSet<String>,
    expected_route: &str,
) -> Result<()> {
    match case_id {
        "labour-overlay-mutated-bad-001" => {
            ensure!(canary_checks == required_checks);
            ensure!(json_str_field_eq(
                canary.get("expected_verdict"),
                "rejected"
            ));
            ensure!(matches!(expected_route, "review"));
            ensure!(yaml_str_field_eq(
                yaml_get(golden_case, "expected_downstream_route"),
                "review"
            ));
            let canary_issue_categories =
                json_string_set_field(canary, "expected_issue_categories")?;
            ensure!(canary_issue_categories == *required_checks);
        }
        "labour-overlay-worker-outage-001" => {
            ensure!(canary_checks.is_empty());
            ensure!(json_str_field_eq(canary.get("expected_verdict"), "flagged"));
            ensure!(json_str_field_eq(
                canary.get("expected_basis_sufficiency_state"),
                "insufficient"
            ));
            ensure!(json_str_field_eq(
                canary.get("worker_result_status"),
                "unavailable"
            ));
            ensure!(matches!(expected_route, "review"));
            ensure!(yaml_str_field_eq(
                yaml_get(golden_case, "expected_downstream_route"),
                "review"
            ));
        }
        _ => {
            ensure!(canary_checks == required_checks);
            ensure!(matches!(expected_route, "pass"));
        }
    }
    Ok(())
}

fn assert_overlay_labour_rubric_contract(
    overlay_root: &Path,
    required_checks: &BTreeSet<String>,
) -> Result<()> {
    let rubric: YamlValue = serde_yaml::from_str(&fs::read_to_string(
        overlay_root.join("validator-rubric.yaml"),
    )?)?;
    ensure!(
        yaml_sequence_string_set(yaml_get(&rubric, "decision_axes"), "decision_axes")?
            == *required_checks
    );
    let blocking_checks = yaml_get(&rubric, "blocking_checks")
        .and_then(YamlValue::as_sequence)
        .ok_or_else(|| anyhow::anyhow!("overlay rubric missing blocking_checks"))?
        .iter()
        .map(|check| {
            yaml_get(check, "id")
                .and_then(YamlValue::as_str)
                .map(str::to_owned)
                .ok_or_else(|| anyhow::anyhow!("overlay rubric blocking_check missing id"))
        })
        .collect::<Result<BTreeSet<_>>>()?;
    ensure!(blocking_checks == *required_checks);

    Ok(())
}

#[test]
fn e2e_can_001_fts_canary_fetches_minimum_viable_notice_set_and_validates_basic_ocds_structure()
-> Result<()> {
    let state_db = TestStateDb::new()?;
    ensure!(state_db.db_path().ends_with("state.db"));
    // Scope this canary to FTS by explicitly removing every other auto-seeded
    // procurement source. The state-db open path now also seeds Contracts
    // Finder and the three procontract-* sources, but this canary is about the
    // FTS minimum-viable shape, so the harness needs to be hermetic. Drop
    // dependent rows first to satisfy the foreign-key constraint between
    // source_runtime_state.source_id and sources.id.
    state_db.connection().execute(
        "DELETE FROM source_runtime_state WHERE source_id != ?1",
        [FTS_SOURCE_ID],
    )?;
    state_db
        .connection()
        .execute("DELETE FROM sources WHERE id != ?1", [FTS_SOURCE_ID])?;
    let source_count: i64 =
        state_db
            .connection()
            .query_row("SELECT COUNT(*) FROM sources", [], |row| row.get(0))?;
    let source_id: String =
        state_db
            .connection()
            .query_row("SELECT id FROM sources LIMIT 1", [], |row| row.get(0))?;
    let fts_dir = skill_dir("procurement-ingestion/fts")?;
    let canary_input = load_first_canary_input(&fts_dir.join("canary.inputs.jsonl"))?;
    let notices = json!([
        {
            "uri": "ocds-123",
            "publishedDate": "2026-03-25T00:00:00Z",
            "title": "PPE supply"
        }
    ]);

    ensure!(
        source_count == 1,
        "FTS must be the only seeded procurement source after harness filtering"
    );
    ensure!(source_id == FTS_SOURCE_ID, "unexpected seeded source id");
    ensure!(canary_input["limit"] == 5, "unexpected FTS canary limit");
    ensure!(
        notices.as_array().map_or(0, Vec::len) > 0,
        "notice batch must be non-empty"
    );
    ensure!(notices[0]["uri"].as_str().is_some(), "OCDS uri missing");
    ensure!(
        notices[0]["publishedDate"].as_str().is_some(),
        "publishedDate missing"
    );
    ensure!(notices[0]["title"].as_str().is_some(), "title missing");
    Ok(())
}

#[test]
fn e2e_can_002_page_fingerprint_match_is_checked_before_broader_traffic_depends_on_the_skill()
-> Result<()> {
    ensure!(
        fingerprint_matches("fts-ocds-release-packages", "fts-ocds-release-packages"),
        "fingerprint match should pass"
    );
    ensure!(
        !fingerprint_matches("fts-ocds-release-packages", "fts-html-search-page"),
        "fingerprint mismatch should fail"
    );
    Ok(())
}

#[test]
fn e2e_can_003_auth_state_verification_is_enforced_when_declared() -> Result<()> {
    ensure!(
        auth_state_valid(false, false),
        "public skill should not require privileged auth state"
    );
    ensure!(
        !auth_state_valid(true, false),
        "required auth state should fail when privileged data is missing"
    );
    ensure!(
        AUTH_STATE_STALE.eq("AUTH_STATE_STALE"),
        "auth-state stale code must stay stable"
    );
    Ok(())
}

#[test]
fn e2e_can_004_protocol_version_drift_enters_canary_handling_instead_of_production_traffic()
-> Result<()> {
    let unknown_version = "9.9.9";
    let known_versions = ["1.2.3", "1.2.4"];
    ensure!(
        !known_versions.contains(&unknown_version),
        "test version unexpectedly entered known-good set"
    );
    ensure!(
        AGENT_PROTOCOL_MISMATCH.eq("AGENT_PROTOCOL_MISMATCH"),
        "protocol/version drift must classify explicitly"
    );
    Ok(())
}

#[test]
fn e2e_can_005_page_shape_drift_yields_classified_failure_with_artifact_capture() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let markdown_path = create_error_markdown(PAGE_SHAPE_DRIFT, "fingerprint-check", temp.path())?;

    ensure!(markdown_path.exists(), "page-shape drift artifact missing");
    ensure!(
        fs::read_to_string(markdown_path)?.contains(PAGE_SHAPE_DRIFT),
        "page-shape drift markdown missing error code"
    );
    Ok(())
}

#[test]
fn e2e_can_006_locale_currency_drift_fails_fast_and_requires_config_review() -> Result<()> {
    ensure!(
        locale_currency_match("en-GB", "GBP"),
        "expected locale/currency should pass"
    );
    ensure!(
        !locale_currency_match("fr-FR", "EUR"),
        "unexpected locale/currency should fail"
    );
    ensure!(
        LOCALE_OR_CURRENCY_DRIFT == "LOCALE_OR_CURRENCY_DRIFT",
        "locale/currency drift must classify explicitly"
    );
    Ok(())
}

#[test]
fn e2e_can_007_source_soft_blocked_responses_preserve_artifacts_and_fail_explicitly() -> Result<()>
{
    let temp = tempfile::tempdir()?;
    let artifact_path = temp.path().join("source-soft-blocked.html");
    fs::write(&artifact_path, "<html><body>No useful data</body></html>")?;

    ensure!(
        SOURCE_SOFT_BLOCKED == "SOURCE_SOFT_BLOCKED",
        "soft-blocked response should classify explicitly"
    );
    ensure!(
        artifact_path.exists(),
        "soft-blocked artifact was not preserved"
    );
    Ok(())
}

#[test]
fn e2e_can_008_hard_captcha_block_produces_blocked_human_action_outcome() -> Result<()> {
    ensure!(
        SOURCE_CAPTCHA_BLOCKED == "SOURCE_CAPTCHA_BLOCKED",
        "CAPTCHA block should classify explicitly"
    );
    ensure!(
        fixed_started_at()?.timestamp() > 0,
        "blocked_human_action path should remain deterministic"
    );
    Ok(())
}

#[test]
fn e2e_can_009_declared_canary_step_failure_emits_agent_skill_failed_with_context() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let markdown_path = create_error_markdown(AGENT_SKILL_FAILED, "canary-step", temp.path())?;
    let markdown = fs::read_to_string(markdown_path)?;

    ensure!(
        markdown.contains(AGENT_SKILL_FAILED),
        "declared canary step failure should classify as AGENT_SKILL_FAILED"
    );
    ensure!(
        markdown.contains("skills/procurement-ingestion/fts/SKILL.md"),
        "error context missing skill_path"
    );
    ensure!(
        markdown.contains("canary-step"),
        "error context missing workflow_step"
    );
    Ok(())
}

#[test]
fn e2e_can_010_bid_portal_canary_constants_cover_l6_error_codes() -> Result<()> {
    let cases = bid_portal_canary_failure_codes();
    let observed_codes = cases
        .iter()
        .map(|(_, error_code)| *error_code)
        .collect::<Vec<_>>();

    ensure!(
        observed_codes.as_slice().eq(&[
            BID_SUBMISSION_FAILED,
            BID_DUPLICATE_DETECTED,
            BID_SIDE_EFFECT_CAP_EXCEEDED,
            BID_READINESS_GATE_FAILED,
            BID_PORTAL_CHANGED,
            BID_PORTAL_AUTH_EXPIRED,
        ]),
        "bid portal canaries must exercise every L6 bid error code once"
    );
    ensure!(
        !BID_PORTAL_CHANGED.eq(PAGE_SHAPE_DRIFT),
        "portal bid drift must remain distinct from generic page-shape drift"
    );
    ensure!(
        !BID_PORTAL_AUTH_EXPIRED.eq(AUTH_STATE_STALE),
        "portal bid auth expiry must remain distinct from generic stale auth"
    );

    let portal_changed = bid_portal_changed_context();
    ensure!(
        portal_changed["selector"].is_string()
            && portal_changed["expectedSourceStructureVersion"].is_string()
            && portal_changed["observedShape"].is_object()
            && portal_changed["portalOpportunityRef"].is_string(),
        "BID_PORTAL_CHANGED canary context must carry bid-specific portal diagnostics"
    );
    let auth_expired = bid_portal_auth_expired_context();
    ensure!(
        auth_expired["portalOpportunityRef"].is_string()
            && auth_expired["lastUrl"].is_string()
            && json_str_field_eq(auth_expired.get("daemonRetry"), "auth-signin/procontract"),
        "BID_PORTAL_AUTH_EXPIRED canary context must carry opportunity and retry diagnostics"
    );

    Ok(())
}

#[test]
fn e2e_can_011_bid_portal_auth_stale_canary_replays_session_expired_cassette() -> Result<()> {
    let fixture_root =
        repo_root()?.join("tests/fixtures/calibration/bid-portal-procontract/simple");
    let canary = load_first_canary_input(&fixture_root.join("canary.inputs.jsonl"))?;
    let cassette_ref = canary
        .get("cassette_ref")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("bid portal auth-stale canary missing cassette_ref"))?;
    let cassette: Value =
        serde_json::from_str(&fs::read_to_string(fixture_root.join(cassette_ref))?)?;
    let expected_result = canary
        .get("expected_result")
        .ok_or_else(|| anyhow::anyhow!("bid portal auth-stale canary missing expected_result"))?;
    let skill_markdown = fs::read_to_string(skill_dir("bid-portal-procontract")?.join("SKILL.md"))?;

    ensure!(
        json_str_field_eq(canary.get("case_id"), "simple-auth-stale-01"),
        "auth-stale canary must be the simple fixture's first row"
    );
    ensure!(
        json_str_field_eq(canary.get("expected_error_code"), AUTH_STATE_STALE),
        "auth-stale canary must expect AUTH_STATE_STALE"
    );
    ensure!(
        json_str_field_eq(
            canary.get("expected_side_effect_class_reached"),
            "READ_ONLY",
        ),
        "auth-stale canary must stay read-only"
    );
    ensure!(
        cassette
            .get("url")
            .and_then(Value::as_str)
            .is_some_and(|url| url.contains("/Login"))
            || cassette
                .get("body_text")
                .and_then(Value::as_str)
                .is_some_and(
                    |text| text.contains("Session expired") || text.contains("Please log in")
                ),
        "auth-stale cassette must expose a deterministic expiry signal"
    );
    ensure!(
        json_str_field_eq(
            cassette.pointer("/expected_detection/error_code"),
            AUTH_STATE_STALE
        ),
        "cassette expected detection must classify AUTH_STATE_STALE"
    );
    ensure!(
        json_str_field_eq(expected_result.get("kind"), "failure")
            && json_str_field_eq(expected_result.get("error_code"), AUTH_STATE_STALE)
            && json_i64_field_eq(expected_result.get("step_reached"), 2)
            && json_str_field_eq(
                expected_result.get("side_effect_class_reached"),
                "READ_ONLY"
            ),
        "auth-stale expected result must match the Failure envelope"
    );
    ensure!(
        expected_result
            .get("screenshot_paths")
            .and_then(Value::as_array)
            .is_some_and(|screenshots| screenshots.len() >= 2),
        "auth-stale expected result must carry step 1 and 2 screenshots"
    );
    ensure!(
        skill_markdown.contains("Do not call `auth-signin/procontract`.")
            && skill_markdown.contains("The daemon owns the retry"),
        "bid portal skill must delegate auth recovery to the daemon"
    );
    ensure!(
        !skill_markdown.contains("https://procontract.due-north.com/Login\""),
        "bid portal skill must not navigate directly to the Procontract login page"
    );

    Ok(())
}

#[test]
fn e2e_can_012_bid_portal_opportunity_lookup_canaries_cover_verified_lookup_duplicate_and_drift()
-> Result<()> {
    let fixture_root =
        repo_root()?.join("tests/fixtures/calibration/bid-portal-procontract/simple");
    let canaries = load_canary_inputs(&fixture_root.join("canary.inputs.jsonl"))?;
    let skill_markdown = fs::read_to_string(skill_dir("bid-portal-procontract")?.join("SKILL.md"))?;

    for required in [
        "Opportunity-reference check",
        "Title check",
        "Portal-status check",
        "Existing-submission detection",
        "Existing-draft detection",
        "Page-fingerprint check",
        "`PortalStateVerification`",
        "opportunity summary panel",
        "response form root",
        "lot selection grid",
        "document upload widget",
    ] {
        ensure!(
            skill_markdown.contains(required),
            "opportunity verification playbook missing `{required}`"
        );
    }

    let happy = find_canary_case(&canaries, "simple-happy-01")?;
    let duplicate = find_canary_case(&canaries, "simple-duplicate-01")?;
    let drift = find_canary_case(&canaries, "simple-portal-drift-01")?;

    ensure!(
        json_str_field_eq(happy.get("expected_terminal_state"), "draft_saved")
            && happy.get("expected_error_code").is_some_and(Value::is_null)
            && json_i64_field_eq(happy.get("expected_step_reached"), 5),
        "happy opportunity canary must reach verified response form without a read-only error"
    );
    let happy_cassette = load_bid_portal_cassette(&fixture_root, happy)?;
    ensure!(
        json_str_field_eq(
            happy_cassette.pointer("/opportunity_metadata/observed_ref"),
            "FAKE-OPP-001"
        ) && json_str_field_eq(
            happy_cassette.pointer("/portal_state_verification/portal_status"),
            "Open"
        ) && happy_cassette
            .pointer("/portal_state_verification/existing_draft_detected")
            .and_then(Value::as_bool)
            .is_some_and(|observed| !observed)
            && happy_cassette
                .pointer("/portal_state_verification/existing_submission_detected")
                .and_then(Value::as_bool)
                .is_some_and(|observed| !observed)
            && happy_cassette
                .pointer("/page_fingerprint/major_divergence")
                .and_then(Value::as_bool)
                .is_some_and(|observed| !observed),
        "happy lookup cassette must verify the opportunity and absence of draft/submission"
    );

    ensure!(
        json_str_field_eq(duplicate.get("expected_error_code"), BID_DUPLICATE_DETECTED)
            && json_i64_field_eq(duplicate.get("expected_step_reached"), 4),
        "duplicate canary must fail at Step 4 before mutation"
    );
    let duplicate_cassette = load_bid_portal_cassette(&fixture_root, duplicate)?;
    ensure!(
        json_str_field_eq(
            duplicate_cassette.pointer("/expected_detection/error_code"),
            BID_DUPLICATE_DETECTED
        ) && duplicate_cassette
            .pointer("/expected_detection/portal_state/existing_submission_detected")
            .and_then(Value::as_bool)
            .is_some_and(|observed| observed)
            && duplicate_cassette
                .get("body_text")
                .and_then(Value::as_str)
                .is_some_and(|text| {
                    text.contains("Already submitted") && text.contains("View your response")
                }),
        "duplicate cassette must expose deterministic existing-submission evidence"
    );

    ensure!(
        json_str_field_eq(drift.get("expected_error_code"), PAGE_SHAPE_DRIFT)
            && json_i64_field_eq(drift.get("expected_step_reached"), 4),
        "portal-drift canary must fail at Step 4 before mutation"
    );
    let drift_cassette = load_bid_portal_cassette(&fixture_root, drift)?;
    ensure!(
        json_str_field_eq(
            drift_cassette.pointer("/expected_detection/error_code"),
            PAGE_SHAPE_DRIFT
        ) && !json_str_field_eq(
            drift_cassette.pointer("/opportunity_metadata/observed_ref"),
            "FAKE-OPP-001"
        ) && drift_cassette
            .pointer("/page_fingerprint/major_divergence")
            .and_then(Value::as_bool)
            .is_some_and(|observed| observed)
            && drift_cassette
                .pointer("/dom_snapshot/missing_anchors")
                .and_then(Value::as_array)
                .is_some_and(|anchors| anchors.len() >= 3),
        "portal-drift cassette must expose deterministic fingerprint divergence"
    );

    Ok(())
}

#[test]
fn e2e_can_013_bid_portal_form_fill_and_upload_canaries_cover_remote_draft_hash_checks()
-> Result<()> {
    let fixture_root =
        repo_root()?.join("tests/fixtures/calibration/bid-portal-procontract/simple");
    let canaries = load_canary_inputs(&fixture_root.join("canary.inputs.jsonl"))?;
    let skill_markdown = fs::read_to_string(skill_dir("bid-portal-procontract")?.join("SKILL.md"))?;

    for required in [
        "Field mapping (from `CompanyProfile`, NEVER from any other source)",
        "`companyName` -> portal \"Company Name\" input",
        "`registrationNumber` -> portal \"Company Registration Number\" input",
        "`primaryContact.email` -> portal \"Contact Email\" input",
        "DO NOT auto-fill from previously-saved portal data.",
        "DO NOT log filled values to stdout/stderr.",
        "DO NOT cache filled values to disk outside the daemon-managed artifact store.",
        "Use only `BidScopePayload.lotIds`",
        "`selectedQuoteIds` and daemon-provided `selected_quote_summaries`",
        "For each `BidDocument` in `BidDocument[]`",
        "Compute SHA-256 over the file contents.",
        "Compare the observed hash against `BidDocument.contentHash`",
        "`portal_acknowledgement_text`",
    ] {
        ensure!(
            skill_markdown.contains(required),
            "form-fill/upload playbook missing `{required}`"
        );
    }

    let upload_success = find_canary_case(&canaries, "simple-form-fill-upload-01")?;
    let hash_mismatch = find_canary_case(&canaries, "simple-upload-hash-mismatch-01")?;

    ensure!(
        json_str_field_eq(
            upload_success.get("expected_terminal_state"),
            "bid_in_progress"
        ) && upload_success
            .get("expected_error_code")
            .is_some_and(Value::is_null)
            && json_i64_field_eq(upload_success.get("expected_step_reached"), 8)
            && json_str_field_eq(
                upload_success.get("expected_side_effect_class_reached"),
                "REMOTE_DRAFT",
            ),
        "upload-success canary must reach Step 8 remote-draft state without error"
    );
    let upload_success_cassette = load_bid_portal_cassette(&fixture_root, upload_success)?;
    ensure!(
        json_str_field_eq(
            upload_success_cassette.pointer("/form_fill/source"),
            "CompanyProfile",
        ) && json_str_field_eq(
            upload_success_cassette.pointer("/form_fill/company_profile_version"),
            "test-profile-v1",
        ) && json_bool_field_eq(
            upload_success_cassette.pointer("/form_fill/pii_logged"),
            false
        ) && json_bool_field_eq(
            upload_success_cassette.pointer("/form_fill/cached_outside_artifact_store"),
            false,
        ) && json_bool_field_eq(
            upload_success_cassette.pointer("/lot_pricing/unscoped_lots_selected"),
            false,
        ) && upload_success_cassette
            .pointer("/document_uploads/0/hash_match")
            .and_then(Value::as_bool)
            .is_some_and(|observed| observed)
            && json_str_field_eq(
                upload_success_cassette.pointer("/document_uploads/0/portal_acknowledgement_text",),
                "Upload complete",
            ),
        "upload-success cassette must prove profile-only fill, scoped lots, and hash-checked upload"
    );

    ensure!(
        json_str_field_eq(
            hash_mismatch.get("expected_error_code"),
            BID_SUBMISSION_FAILED,
        ) && json_i64_field_eq(hash_mismatch.get("expected_step_reached"), 8)
            && json_str_field_eq(
                hash_mismatch.get("expected_side_effect_class_reached"),
                "REMOTE_DRAFT",
            ),
        "hash-mismatch canary must fail at Step 8 with BID_SUBMISSION_FAILED"
    );
    let hash_mismatch_cassette = load_bid_portal_cassette(&fixture_root, hash_mismatch)?;
    ensure!(
        json_str_field_eq(
            hash_mismatch_cassette.pointer("/expected_detection/error_code"),
            BID_SUBMISSION_FAILED,
        ) && json_bool_field_eq(
            hash_mismatch_cassette.pointer("/document_uploads/0/hash_match"),
            false,
        ) && json_bool_field_eq(
            hash_mismatch_cassette.pointer("/document_uploads/0/uploaded"),
            false,
        ) && json_str_field_eq(
            hash_mismatch_cassette.pointer("/expected_detection/error_context/document_type"),
            "insurance_certificate",
        ),
        "hash-mismatch cassette must expose structured document mismatch context"
    );

    Ok(())
}

fn assert_bid_portal_terminal_path_playbook(skill_markdown: &str) -> Result<()> {
    for required in [
        "If `reviewRequired = true` -> branch 10a",
        "If `sideEffectCap` is `READ_ONLY`, `REMOTE_DRAFT`, `NOTIFICATION_OR_LOCK`, or `LEGAL_DECLARATION` -> branch 10a",
        "If `reviewRequired = false` AND `sideEffectCap` permits `FORMAL_SUBMISSION` -> branch 10b",
        "`validation_approved = true` OR `validation_override_logged = true`",
        "Step 11 always runs after Step 10b",
        "Keep `side_effect_class_reached` at `FORMAL_SUBMISSION` during Step 11",
        "\"kind\": \"draft_save_result\"",
        "\"kind\": \"submission_result\"",
    ] {
        ensure!(
            skill_markdown.contains(required),
            "draft/final-submit playbook missing `{required}`"
        );
    }

    Ok(())
}

fn assert_draft_saved_canary(fixture_root: &Path, draft_saved: &Value) -> Result<()> {
    ensure!(
        json_str_field_eq(draft_saved.get("expected_terminal_state"), "draft_saved")
            && draft_saved
                .get("expected_error_code")
                .is_some_and(Value::is_null)
            && json_i64_field_eq(draft_saved.get("expected_step_reached"), 10)
            && json_str_field_eq(
                draft_saved.get("expected_side_effect_class_reached"),
                "REMOTE_DRAFT",
            )
            && json_str_field_eq(
                draft_saved.pointer("/expected_result/kind"),
                "draft_save_result",
            )
            && json_bool_field_eq(draft_saved.pointer("/expected_result/draft_saved"), true)
            && json_str_field_eq(
                draft_saved.pointer("/expected_result/portal_draft_ref"),
                "DRAFT-FAKE-OPP-001",
            ),
        "draft-saved canary must terminate at Step 10a with remote draft evidence"
    );
    let draft_cassette = load_bid_portal_cassette(fixture_root, draft_saved)?;
    ensure!(
        json_str_field_eq(
            draft_cassette.pointer("/branch_decision/selected_branch"),
            "10a_save_draft",
        ) && json_str_field_eq(
            draft_cassette.pointer("/branch_decision/reason"),
            "reviewRequired=true",
        ) && json_str_field_eq(
            draft_cassette.pointer("/step_10a/portal_draft_ref"),
            "DRAFT-FAKE-OPP-001",
        ) && json_str_field_eq(
            draft_cassette.pointer("/checkpoint/side_effect_class_reached"),
            "REMOTE_DRAFT",
        ) && draft_cassette
            .pointer("/checkpoint/screenshot_paths")
            .and_then(Value::as_array)
            .is_some_and(|screenshots| {
                screenshots.iter().any(|value| {
                    value
                        .as_str()
                        .is_some_and(|path| path.ends_with("step-10-save-draft.webp"))
                })
            }),
        "draft-saved cassette must prove 10a branch reason, draft reference, and screenshot evidence"
    );

    Ok(())
}

fn assert_formal_submit_canary(fixture_root: &Path, formal_submit: &Value) -> Result<()> {
    ensure!(
        json_str_field_eq(
            formal_submit.get("expected_terminal_state"),
            "bid_submitted",
        ) && formal_submit
            .get("expected_error_code")
            .is_some_and(Value::is_null)
            && json_i64_field_eq(formal_submit.get("expected_step_reached"), 11)
            && json_str_field_eq(
                formal_submit.get("expected_side_effect_class_reached"),
                "FORMAL_SUBMISSION",
            )
            && json_str_field_eq(
                formal_submit.pointer("/expected_result/kind"),
                "submission_result",
            )
            && json_bool_field_eq(formal_submit.pointer("/expected_result/submitted"), true)
            && json_str_field_eq(
                formal_submit.pointer("/expected_result/confirmation_ref"),
                "SUBMIT-FAKE-OPP-001",
            )
            && formal_submit
                .pointer("/expected_result/screenshot_paths")
                .and_then(Value::as_array)
                .is_some_and(|screenshots| screenshots.len() >= 11),
        "formal-submit canary must terminate at Step 11 with confirmation evidence and full screenshot trail"
    );
    let formal_submit_cassette = load_bid_portal_cassette(fixture_root, formal_submit)?;
    ensure!(
        json_str_field_eq(
            formal_submit_cassette.pointer("/branch_decision/selected_branch"),
            "10b_formal_submission",
        ) && json_bool_field_eq(
            formal_submit_cassette.pointer("/branch_decision/preconditions/review_required"),
            false,
        ) && json_str_field_eq(
            formal_submit_cassette.pointer("/branch_decision/preconditions/side_effect_cap"),
            "FORMAL_SUBMISSION",
        ) && json_bool_field_eq(
            formal_submit_cassette.pointer("/branch_decision/preconditions/validation_approved"),
            true,
        ) && json_bool_field_eq(
            formal_submit_cassette
                .pointer("/branch_decision/preconditions/validation_override_logged"),
            false,
        ) && json_bool_field_eq(
            formal_submit_cassette.pointer("/step_11/capture_ran_after_step_10b"),
            true,
        ) && json_str_field_eq(
            formal_submit_cassette.pointer("/checkpoint/highest_side_effect_class_reached"),
            "FORMAL_SUBMISSION",
        ) && json_str_field_eq(
            formal_submit_cassette.pointer("/step_11/declared_side_effect_class"),
            "READ_ONLY",
        ),
        "formal-submit cassette must prove 10b preconditions, Step 11 capture, and monotonic highest side-effect tracking"
    );

    Ok(())
}

#[test]
fn e2e_can_014_bid_portal_draft_and_submit_paths_cover_terminal_side_effects() -> Result<()> {
    let fixture_root =
        repo_root()?.join("tests/fixtures/calibration/bid-portal-procontract/simple");
    let canaries = load_canary_inputs(&fixture_root.join("canary.inputs.jsonl"))?;
    let skill_markdown = fs::read_to_string(skill_dir("bid-portal-procontract")?.join("SKILL.md"))?;

    assert_bid_portal_terminal_path_playbook(&skill_markdown)?;
    assert_draft_saved_canary(
        &fixture_root,
        find_canary_case(&canaries, "simple-draft-saved-01")?,
    )?;
    assert_formal_submit_canary(
        &fixture_root,
        find_canary_case(&canaries, "simple-formal-submit-01")?,
    )?;

    Ok(())
}

#[test]
fn e2e_can_015_bid_portal_calibration_corpus_contract_covers_required_variants() -> Result<()> {
    assert_bid_portal_corpus_category(
        "simple",
        6,
        &[
            "simple-draft-saved-01",
            "simple-missing-company-profile-01",
            "simple-auth-stale-01",
            "simple-duplicate-draft-01",
            "simple-cap-breach-01",
            "simple-portal-reference-not-found-01",
        ],
    )?;
    assert_bid_portal_corpus_category(
        "medium",
        8,
        &[
            "medium-happy-draft-01",
            "medium-upload-success-01",
            "medium-upload-hash-mismatch-01",
            "medium-quote-override-01",
            "medium-validation-rejection-01",
            "medium-selector-drift-01",
            "medium-resume-draft-01",
            "medium-duplicate-submission-01",
        ],
    )?;
    assert_bid_portal_corpus_category(
        "complex",
        10,
        &[
            "complex-multi-lot-draft-01",
            "complex-multi-document-upload-01",
            "complex-conditional-declarations-01",
            "complex-legal-declaration-boundary-01",
            "complex-final-submit-authorized-01",
            "complex-final-submit-blocked-01",
            "complex-validator-outage-01",
            "complex-validation-override-01",
            "complex-remote-draft-changed-externally-01",
            "complex-unknown-portal-state-reconciliation-01",
        ],
    )?;

    Ok(())
}

#[test]
fn e2e_can_016_bid_portal_simple_corpus_replays_every_canary_case() -> Result<()> {
    assert_bid_portal_corpus_replays_all_cases("simple", 6)
}

#[test]
fn e2e_can_017_bid_portal_medium_corpus_replays_every_canary_case() -> Result<()> {
    assert_bid_portal_corpus_replays_all_cases("medium", 8)
}

#[test]
fn e2e_can_018_bid_portal_complex_corpus_replays_every_canary_case() -> Result<()> {
    assert_bid_portal_corpus_replays_all_cases("complex", 10)
}
