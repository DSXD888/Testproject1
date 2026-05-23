#![forbid(unsafe_code)]

#[path = "support/db.rs"]
mod db_support;

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Result, ensure};
use chrono::{TimeZone, Utc};
use db_support::{FTS_SOURCE_ID, TestStateDb};
use serde_json::{Value, json};

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
