#![forbid(unsafe_code)]

#[path = "support/skills.rs"]
mod skills_support;
#[path = "../src/sources/registry.rs"]
#[allow(dead_code)]
mod sources_impl;

use std::collections::BTreeSet;

use anyhow::{Result, ensure};
use serde_json::Value as JsonValue;

use skills_support::{
    LintedSkill, TestSkillDir, lint_skill_directory, load_skill_metadata, repo_root, repo_skill_dir,
};
use sources_impl::DEFAULT_SOURCE_SKILL_VERSION;

const RUN_MANIFEST_REQUIRED_TOKENS: &[&str] = &[
    "manifest_version",
    "skill_id",
    "skill_version",
    "job_id",
    "session_id",
    "execution_mode",
    "Summary",
    "Steps Taken",
    "Decisions Made",
    "Warnings",
    "Evidence",
];

const SIGNED_IN_BROWSER_SKILLS: &[&str] = &[
    "bid-portal-procontract",
    "procurement-ingestion/procontract-discover",
    "procurement-ingestion/procontract-enrich-tender",
    "procurement-ingestion/procontract-register-interest",
    "marketplace-search/alibaba",
    "landed-cost/shipment-sizing",
    "landed-cost/searates-freight-estimate",
    "source-health/alibaba",
    "auth-refresh/alibaba",
    "auth-refresh/1688",
];

const CDP_BROWSER_SKILLS: &[&str] = &[
    "bid-portal-procontract",
    "procurement-ingestion/fts",
    "procurement-ingestion/contracts-finder",
    "procurement-ingestion/procontract-discover",
    "procurement-ingestion/procontract-enrich-tender",
    "procurement-ingestion/procontract-register-interest",
    "marketplace-search/alibaba",
    "marketplace-search/amazon-uk",
    "marketplace-search/ebay-uk",
    "marketplace-search/gem-imports",
    "marketplace-search/mx-wholesale",
    "landed-cost/shipment-sizing",
    "landed-cost/searates-freight-estimate",
    "source-health/alibaba",
    "source-health/fts",
    "auth-refresh/alibaba",
    "auth-refresh/1688",
];

const AGENT_BROWSER_SUBCOMMANDS: &[&str] = &[
    "open", "click", "fill", "snapshot", "wait", "scroll", "get", "download", "eval", "press",
];

#[test]
fn ut_skl_001_required_files_exist_in_every_production_skill_directory() -> Result<()> {
    let fts = lint_skill_directory(&repo_skill_dir("procurement-ingestion/fts")?)?;
    let contracts_finder =
        lint_skill_directory(&repo_skill_dir("procurement-ingestion/contracts-finder")?)?;
    let pcs = lint_skill_directory(&repo_skill_dir("procurement-ingestion/pcs")?)?;
    let manifest = lint_skill_directory(&repo_skill_dir("manifest-generation")?)?;
    let procurement_skill_dirs: BTreeSet<_> =
        std::fs::read_dir(repo_root()?.join("skills/procurement-ingestion"))?
            .filter_map(|entry| entry.ok().map(|value| value.path()))
            .filter(|path| path.is_dir())
            .filter_map(|path| {
                path.file_name()
                    .and_then(|value| value.to_str())
                    .map(str::to_owned)
            })
            .collect();

    // FTS, Contracts Finder, PCS, and the three procontract-* skills are the
    // active procurement source skills. The procontract entries were authored
    // more recently and are not yet held to the strict lint shape (Non-goals,
    // Resume Strategy, Pagination Bounds), so this test only asserts that the
    // directory listing matches expectations and that the canonical FTS /
    // Contracts Finder / PCS / manifest skills lint cleanly.
    let expected: BTreeSet<String> = [
        "contracts-finder",
        "fts",
        "pcs",
        "procontract-discover",
        "procontract-enrich-tender",
        "procontract-register-interest",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
    ensure!(
        procurement_skill_dirs == expected,
        "procurement-ingestion directory contents drifted: expected {expected:?}, got {procurement_skill_dirs:?}"
    );
    ensure!(fts.metadata.interaction_profile == "pty_interactive");
    ensure!(contracts_finder.metadata.interaction_profile == "pty_interactive");
    ensure!(pcs.metadata.interaction_profile == "http_public");
    ensure!(manifest.metadata.interaction_profile == "headless_contract");
    Ok(())
}

#[test]
fn ut_skl_002_missing_skill_md_fails_lint_immediately() -> Result<()> {
    let dir = TestSkillDir::from_fixture_filtered(
        &repo_skill_dir("manifest-generation")?,
        Some("SKILL.md"),
    )?;
    let error = lint_skill_directory(dir.path()).expect_err("missing SKILL.md should fail");
    ensure!(error.to_string().contains("SKILL.md"));
    Ok(())
}

#[test]
fn ut_skl_003_required_metadata_fields_are_mandatory() -> Result<()> {
    let dir = TestSkillDir::from_fixture(&repo_skill_dir("manifest-generation")?)?;
    let invalid_markdown = dir
        .read_skill_markdown()?
        .replace("interaction_profile: headless_contract\n", "");
    dir.overwrite("SKILL.md", &invalid_markdown)?;

    let error = lint_skill_directory(dir.path()).expect_err("missing metadata should fail");
    ensure!(error.to_string().contains("interaction_profile"));
    Ok(())
}

#[test]
fn ut_skl_004_trigger_boundary_sections_are_mandatory() -> Result<()> {
    let dir = TestSkillDir::from_fixture(&repo_skill_dir("procurement-ingestion/fts")?)?;
    let invalid_markdown = dir
        .read_skill_markdown()?
        .replace("## Non-goals\n", "## Removed\n");
    dir.overwrite("SKILL.md", &invalid_markdown)?;

    let error = lint_skill_directory(dir.path()).expect_err("missing Non-goals should fail");
    ensure!(error.to_string().contains("Non-goals"));
    Ok(())
}

#[test]
fn ut_skl_005_schema_canary_and_golden_files_must_parse_and_be_non_empty() -> Result<()> {
    let result_schema = TestSkillDir::from_fixture(&repo_skill_dir("manifest-generation")?)?;
    result_schema.overwrite("result.schema.json", "{")?;
    ensure!(
        lint_skill_directory(result_schema.path())
            .expect_err("broken result schema should fail")
            .to_string()
            .contains("result.schema.json")
    );

    let checkpoint_schema = TestSkillDir::from_fixture(&repo_skill_dir("manifest-generation")?)?;
    checkpoint_schema.overwrite("checkpoint.schema.json", "{")?;
    ensure!(
        lint_skill_directory(checkpoint_schema.path())
            .expect_err("broken checkpoint schema should fail")
            .to_string()
            .contains("checkpoint.schema.json")
    );

    let canary = TestSkillDir::from_fixture(&repo_skill_dir("manifest-generation")?)?;
    canary.overwrite("canary.inputs.jsonl", "")?;
    ensure!(
        lint_skill_directory(canary.path())
            .expect_err("empty canary inputs should fail")
            .to_string()
            .contains("canary.inputs.jsonl")
    );

    let golden = TestSkillDir::from_fixture(&repo_skill_dir("manifest-generation")?)?;
    golden.overwrite("golden.assertions.yaml", "null\n")?;
    ensure!(
        lint_skill_directory(golden.path())
            .expect_err("empty golden assertions should fail")
            .to_string()
            .contains("golden.assertions.yaml")
    );
    Ok(())
}

#[test]
fn ut_skl_006_failure_code_enumeration_and_resume_strategy_mandatory() -> Result<()> {
    let failure_codes = TestSkillDir::from_fixture(&repo_skill_dir("manifest-generation")?)?;
    let missing_failure_codes = failure_codes
        .read_skill_markdown()?
        .replace("## Failure Codes\n", "## Removed Failure Codes\n");
    failure_codes.overwrite("SKILL.md", &missing_failure_codes)?;
    ensure!(
        lint_skill_directory(failure_codes.path())
            .expect_err("missing failure codes should fail")
            .to_string()
            .contains("Failure Codes")
    );

    let resume = TestSkillDir::from_fixture(&repo_skill_dir("manifest-generation")?)?;
    let missing_resume = resume
        .read_skill_markdown()?
        .replace("## Resume Strategy\n", "## Removed Resume Strategy\n");
    resume.overwrite("SKILL.md", &missing_resume)?;
    ensure!(
        lint_skill_directory(resume.path())
            .expect_err("missing resume strategy should fail")
            .to_string()
            .contains("Resume Strategy")
    );
    Ok(())
}

#[test]
fn ut_skl_007_bounded_pagination_and_side_effect_declarations_required_where_applicable()
-> Result<()> {
    let dir = TestSkillDir::from_fixture(&repo_skill_dir("procurement-ingestion/fts")?)?;
    let invalid_markdown = dir
        .read_skill_markdown()?
        .replace("maxPages: 5\n", "")
        .replace("side_effect_class: none\n", "");
    dir.overwrite("SKILL.md", &invalid_markdown)?;

    let error = lint_skill_directory(dir.path()).expect_err("missing bounds should fail");
    let error_text = error.to_string();
    ensure!(error_text.contains("maxPages") || error_text.contains("side_effect_class"));
    Ok(())
}

#[test]
fn ut_skl_008_run_manifest_instruction_block_required() -> Result<()> {
    let fts_markdown =
        std::fs::read_to_string(repo_skill_dir("procurement-ingestion/fts")?.join("SKILL.md"))?;
    for token in RUN_MANIFEST_REQUIRED_TOKENS {
        ensure!(fts_markdown.contains(token));
    }

    let dir = TestSkillDir::from_fixture(&repo_skill_dir("manifest-generation")?)?;
    let invalid_markdown = dir
        .read_skill_markdown()?
        .replace("manifest_version", "missing_frontmatter_version_token");
    dir.overwrite("SKILL.md", &invalid_markdown)?;

    let error = lint_skill_directory(dir.path()).expect_err("missing manifest block should fail");
    ensure!(error.to_string().contains("run-manifest instructions"));
    Ok(())
}

#[test]
fn qoa5_skl_lint_001_uk_installation_labour_frontmatter_is_http_public() -> Result<()> {
    let skill = load_skill_metadata(&repo_skill_dir(
        "landed-cost/uk-installation-labour-estimate",
    )?)?;

    ensure!(skill.metadata.skill_id == "landed-cost/uk-installation-labour-estimate");
    ensure!(skill.metadata.skill_version == "0.1.0");
    ensure!(skill.metadata.interaction_profile == "http_public");
    ensure!(skill.metadata.result_schema_version == "1.0.0");
    ensure!(skill.metadata.checkpoint_schema_version.is_none());
    ensure!(skill.metadata.model_profile == "browse_stable_low_cost");
    ensure!(!skill.metadata.requires_cdp);
    ensure!(skill.metadata.cdp_default_port.is_none());
    ensure!(skill.metadata.side_effect_class == "none");
    ensure!(skill.skill_markdown.contains("agent_cli: claude"));
    Ok(())
}

#[test]
fn qoa5_skl_lint_002_uk_installation_labour_has_no_browser_membership() -> Result<()> {
    let relative = "landed-cost/uk-installation-labour-estimate";
    ensure!(!CDP_BROWSER_SKILLS.contains(&relative));
    ensure!(!SIGNED_IN_BROWSER_SKILLS.contains(&relative));

    let skill = load_skill_metadata(&repo_skill_dir(relative)?)?;
    for forbidden in [
        "bin/ensure-cdp-9222",
        "agent-browser --cdp",
        "cdp_default_port",
        "CDP_BROWSER_SKILLS",
        "SIGNED_IN_BROWSER_SKILLS",
    ] {
        ensure!(
            !skill.skill_markdown.contains(forbidden),
            "{relative} must not document browser membership marker `{forbidden}`"
        );
    }
    Ok(())
}

#[test]
fn qoa5_skl_lint_003_uk_installation_labour_line_item_contract_matches_spec() -> Result<()> {
    let skill_dir = repo_skill_dir("landed-cost/uk-installation-labour-estimate")?;
    let result_schema: JsonValue = serde_json::from_str(&std::fs::read_to_string(
        skill_dir.join("result.schema.json"),
    )?)?;
    let golden: JsonValue = serde_yaml::from_str(&std::fs::read_to_string(
        skill_dir.join("golden.assertions.yaml"),
    )?)?;
    let expected_fields = [
        "labour_type",
        "trade",
        "quantity_hours",
        "rate_gbp_per_hour_pence",
        "subtotal_gbp_pence",
        "confidence_bps",
        "rate_source",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect::<Vec<_>>();

    let schema_required_fields =
        json_string_vec(&result_schema, "/$defs/labour_line_item/required")?;
    let golden_required_fields = json_string_vec(
        &golden,
        "/cases/lights-nw-2day-01/labour_line_item_required_fields",
    )?;
    ensure!(
        schema_required_fields == expected_fields,
        "labour_line_item schema required fields drifted from FR-L.6-c"
    );
    ensure!(
        golden_required_fields == expected_fields,
        "golden labour_line_item required fields drifted from FR-L.6-c"
    );
    ensure!(
        schema_required_fields == golden_required_fields,
        "schema and golden labour_line_item required fields disagree"
    );

    let schema_property_fields = result_schema
        .pointer("/$defs/labour_line_item/properties")
        .and_then(JsonValue::as_object)
        .ok_or_else(|| anyhow::anyhow!("labour_line_item properties should be present"))?
        .keys()
        .map(std::string::ToString::to_string)
        .collect::<BTreeSet<_>>();
    let expected_field_set = expected_fields.into_iter().collect::<BTreeSet<_>>();
    ensure!(
        schema_property_fields == expected_field_set,
        "labour_line_item schema properties must exactly match the FR-L.6-c fields"
    );
    ensure!(
        result_schema
            .pointer("/$defs/labour_line_item/additionalProperties")
            .and_then(JsonValue::as_bool)
            == Some(false),
        "labour_line_item schema must reject non-contract fields"
    );
    ensure!(
        result_schema
            .pointer("/$defs/labour_line_item/properties/rate_gbp_per_hour_pence/$ref")
            .and_then(JsonValue::as_str)
            == Some("#/$defs/money")
    );
    ensure!(
        result_schema
            .pointer("/$defs/labour_line_item/properties/subtotal_gbp_pence/$ref")
            .and_then(JsonValue::as_str)
            == Some("#/$defs/money")
    );
    ensure!(
        result_schema
            .pointer("/$defs/labour_line_item/properties/confidence_bps/$ref")
            .and_then(JsonValue::as_str)
            == Some("#/$defs/bps")
    );
    Ok(())
}

const QOA5_RATE_REFRESH_REQUIRED_FILES: [&str; 5] = [
    "SKILL.md",
    "result.schema.json",
    "canary.inputs.jsonl",
    "golden.assertions.yaml",
    "validator-rubric.yaml",
];

#[test]
fn qoa5_skl_rate_refresh_001_source_health_skill_lints_with_required_files() -> Result<()> {
    let relative = "source-health/uk-installation-labour-rate-refresh";
    let skill_dir = repo_skill_dir(relative)?;
    let skill = load_skill_metadata(&skill_dir)?;

    ensure!(skill.metadata.skill_id == relative);
    ensure!(skill.metadata.skill_version == "2026.06.03-1");
    ensure!(skill.metadata.interaction_profile == "http_public");
    ensure!(skill.metadata.result_schema_version == "1.0.0");
    ensure!(skill.metadata.checkpoint_schema_version.is_none());
    ensure!(skill.metadata.model_profile == "browse_stable_low_cost");
    ensure!(skill.metadata.side_effect_class == "none");
    ensure!(!skill.metadata.requires_cdp);
    ensure!(skill.metadata.cdp_default_port.is_none());
    ensure!(
        skill.metadata.source_structure_version.as_deref()
            == Some("uk-installation-labour-rate-refresh-2026-q2")
    );
    ensure!(skill.metadata.promotion_stage.as_deref() == Some("candidate"));
    ensure!(
        skill.metadata.worker_behavior_version.as_deref()
            == Some("source-health/uk-installation-labour-rate-refresh@2026.06.03-1+1.0.0")
    );
    ensure!(skill.metadata.gate_a_family_key.as_deref() == Some("source-health"));
    ensure!(!CDP_BROWSER_SKILLS.contains(&relative));
    ensure!(!SIGNED_IN_BROWSER_SKILLS.contains(&relative));

    for required_file in QOA5_RATE_REFRESH_REQUIRED_FILES {
        ensure!(
            skill_dir.join(required_file).is_file(),
            "rate-refresh skill missing required file `{required_file}`"
        );
    }

    let _: JsonValue = serde_json::from_str(&std::fs::read_to_string(
        skill_dir.join("result.schema.json"),
    )?)?;
    let _: serde_yaml::Value = serde_yaml::from_str(&std::fs::read_to_string(
        skill_dir.join("golden.assertions.yaml"),
    )?)?;
    let _: serde_yaml::Value = serde_yaml::from_str(&std::fs::read_to_string(
        skill_dir.join("validator-rubric.yaml"),
    )?)?;

    for required in [
        "quarterly source-health refresh",
        "not a live landed-cost estimate",
        "candidate JSON files",
        "Run Manifest Requirements",
    ] {
        ensure!(
            skill.skill_markdown.contains(required),
            "rate-refresh SKILL.md missing `{required}`"
        );
    }
    Ok(())
}

#[test]
fn qoa5_skl_rate_refresh_002_contract_enumerates_candidate_output_families() -> Result<()> {
    let skill_dir = repo_skill_dir("source-health/uk-installation-labour-rate-refresh")?;
    let expected = qoa5_rate_refresh_family_set();
    let result_schema: JsonValue = serde_json::from_str(&std::fs::read_to_string(
        skill_dir.join("result.schema.json"),
    )?)?;
    let required = result_schema
        .pointer("/properties/candidate_updates/required")
        .and_then(JsonValue::as_array)
        .ok_or_else(|| anyhow::anyhow!("candidate_updates.required should be present"))?;
    let schema_required_families = required
        .iter()
        .filter_map(JsonValue::as_str)
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();
    ensure!(schema_required_families == expected);

    let schema_property_families = result_schema
        .pointer("/properties/candidate_updates/properties")
        .and_then(JsonValue::as_object)
        .ok_or_else(|| anyhow::anyhow!("candidate_updates.properties should be present"))?
        .keys()
        .map(std::string::ToString::to_string)
        .collect::<BTreeSet<_>>();
    ensure!(schema_property_families == expected);

    let mut canary_count = 0_u32;
    for line in std::fs::read_to_string(skill_dir.join("canary.inputs.jsonl"))?
        .lines()
        .filter(|line| !line.trim().is_empty())
    {
        let row: JsonValue = serde_json::from_str(line)?;
        let canary_families = json_string_set(&row, "/families")?;
        ensure!(canary_families == expected);
        let expected_families =
            json_string_set(&row, "/expected_result/requires_candidate_updates_for")?;
        ensure!(expected_families == expected);
        let public_sources = row
            .pointer("/public_sources")
            .and_then(JsonValue::as_array)
            .ok_or_else(|| anyhow::anyhow!("public_sources should be present"))?;
        let mut public_source_families = BTreeSet::new();
        for source in public_sources {
            let family = source
                .get("family")
                .and_then(JsonValue::as_str)
                .ok_or_else(|| anyhow::anyhow!("public source family should be present"))?;
            public_source_families.insert(family.to_owned());
        }
        ensure!(public_source_families == expected);
        canary_count = canary_count.saturating_add(1);
    }
    ensure!(canary_count > 0);

    let golden = std::fs::read_to_string(skill_dir.join("golden.assertions.yaml"))?;
    let rubric = std::fs::read_to_string(skill_dir.join("validator-rubric.yaml"))?;
    for family in expected {
        ensure!(
            golden.contains(&family) && rubric.contains(&family),
            "golden/rubric missing candidate family `{family}`"
        );
    }
    Ok(())
}

#[test]
fn qoa5_skl_rate_refresh_003_declares_no_production_reference_mutation() -> Result<()> {
    let relative = "source-health/uk-installation-labour-rate-refresh";
    let skill_dir = repo_skill_dir(relative)?;
    let result_schema: JsonValue = serde_json::from_str(&std::fs::read_to_string(
        skill_dir.join("result.schema.json"),
    )?)?;
    ensure!(
        result_schema
            .pointer("/properties/production_reference_mutation_performed/const")
            .and_then(JsonValue::as_bool)
            == Some(false)
    );
    ensure!(
        result_schema
            .pointer("/$defs/candidate_family_result/properties/production_reference_mutated/const")
            .and_then(JsonValue::as_bool)
            == Some(false)
    );

    let skill = load_skill_metadata(&skill_dir)?;
    for required in [
        "No in-place mutation",
        "Never resume by editing production reference files in place",
        "production_reference_mutation_performed: false",
    ] {
        ensure!(
            skill.skill_markdown.contains(required),
            "rate-refresh SKILL.md missing no-mutation token `{required}`"
        );
    }

    for line in std::fs::read_to_string(skill_dir.join("canary.inputs.jsonl"))?
        .lines()
        .filter(|line| !line.trim().is_empty())
    {
        let row: JsonValue = serde_json::from_str(line)?;
        ensure!(
            row.get("allow_production_mutation")
                .and_then(JsonValue::as_bool)
                == Some(false),
            "rate-refresh canary must set allow_production_mutation=false"
        );
        let production_reference_root = row
            .get("production_reference_root")
            .and_then(JsonValue::as_str)
            .ok_or_else(|| anyhow::anyhow!("production_reference_root should be present"))?;
        let candidate_output_root = row
            .get("candidate_output_root")
            .and_then(JsonValue::as_str)
            .ok_or_else(|| anyhow::anyhow!("candidate_output_root should be present"))?;
        ensure!(
            !candidate_output_root.starts_with(production_reference_root),
            "candidate output root must not be inside production reference root"
        );
    }

    let golden = std::fs::read_to_string(skill_dir.join("golden.assertions.yaml"))?;
    let rubric = std::fs::read_to_string(skill_dir.join("validator-rubric.yaml"))?;
    for required in [
        "no_in_place_production_reference_mutation",
        "skills/landed-cost/uk-installation-labour-estimate/reference",
        "candidate_json_only_until_explicit_review",
    ] {
        ensure!(
            golden.contains(required) || rubric.contains(required),
            "rate-refresh golden/rubric missing no-mutation token `{required}`"
        );
    }
    Ok(())
}

fn qoa5_rate_refresh_family_set() -> BTreeSet<String> {
    [
        "labour_rates",
        "access_equipment_bands",
        "installation_profiles",
        "permit_tm_bands",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect()
}

fn json_string_vec(value: &JsonValue, pointer: &str) -> Result<Vec<String>> {
    let array = value
        .pointer(pointer)
        .and_then(JsonValue::as_array)
        .ok_or_else(|| anyhow::anyhow!("missing string array at `{pointer}`"))?;
    array
        .iter()
        .map(|item| {
            item.as_str()
                .map(str::to_owned)
                .ok_or_else(|| anyhow::anyhow!("non-string item at `{pointer}`"))
        })
        .collect::<Result<Vec<_>>>()
}

fn json_string_set(value: &JsonValue, pointer: &str) -> Result<BTreeSet<String>> {
    Ok(json_string_vec(value, pointer)?.into_iter().collect())
}

#[test]
fn l6_t_bid_portal_procontract_skill_001_frontmatter_declares_l6_contract() -> Result<()> {
    let skill = load_skill_metadata(&repo_skill_dir("bid-portal-procontract")?)?;
    ensure!(skill.metadata.skill_id == "bid-portal/procontract");
    ensure!(skill.metadata.skill_version == "0.1.0");
    ensure!(skill.metadata.interaction_profile == "pty_interactive");
    ensure!(skill.metadata.result_schema_version == "1.0.0");
    ensure!(skill.metadata.checkpoint_schema_version.as_deref() == Some("1.0.0"));
    ensure!(skill.metadata.model_profile == "portal_form_high_reliability");
    ensure!(skill.metadata.side_effect_class == "external_mutation");
    ensure!(skill.metadata.requires_cdp);
    ensure!(skill.metadata.cdp_default_port == Some(9222));
    ensure!(
        skill.metadata.source_structure_version.as_deref()
            == Some("procontract-bid-portal-2026-05-22")
    );
    ensure!(skill.metadata.promotion_stage.as_deref() == Some("draft"));
    ensure!(
        skill.metadata.worker_behavior_version.as_deref()
            == Some("bid-portal/procontract@0.1.0+1.0")
    );
    ensure!(skill.metadata.version_change_affects_behavior);
    ensure!(skill.skill_markdown.contains("agent_cli: claude"));
    ensure!(skill.skill_markdown.contains("validation_policy: always"));
    Ok(())
}

#[test]
fn l6_t_bid_portal_procontract_skill_004_cdp_discipline_and_playbook_contract() -> Result<()> {
    let skill = load_skill_metadata(&repo_skill_dir("bid-portal-procontract")?)?;
    let markdown = skill.skill_markdown.as_str();

    for required in [
        "bin/ensure-cdp-9222",
        "http://localhost:${CDP_PORT}/json/version",
        "agent-browser --cdp ${CDP_PORT}",
        "/Users/alinaaitova/.chrome-automation",
        "return `AUTH_STATE_STALE` immediately",
        "Do not call `auth-signin/procontract`.",
        "The daemon owns the retry",
        "return `BID_DUPLICATE_DETECTED`",
        "return `PAGE_SHAPE_DRIFT` or `BID_PORTAL_CHANGED`",
        "return `BID_SIDE_EFFECT_CAP_EXCEEDED`",
    ] {
        ensure!(
            markdown.contains(required),
            "bid portal skill missing `{required}`"
        );
    }

    for step_heading in [
        "### Step 1 - Navigate to Procontract portal login page",
        "### Step 2 - Verify authenticated state",
        "### Step 3 - Search or navigate to the target opportunity",
        "### Step 4 - Verify opportunity and duplicate state",
        "### Step 5 - Open bid response form",
        "### Step 6 - Fill company details section",
        "### Step 7 - Fill lot selection and pricing sections",
        "### Step 8 - Upload required documents",
        "### Step 9 - Review submission summary",
        "### Step 10a - Save draft",
        "### Step 10b - Submit expression of interest",
        "### Step 11 - Capture confirmation page/reference",
    ] {
        ensure!(
            markdown.contains(step_heading),
            "bid portal skill missing `{step_heading}`"
        );
    }

    ensure!(
        markdown.matches("Screenshot: required.").count() >= 12,
        "bid portal skill must require screenshots for every playbook branch"
    );

    for side_effect_contract in [
        "Side-effect class: `READ_ONLY`.",
        "Side-effect class: `REMOTE_DRAFT`.",
        "Side-effect class: `REMOTE_DRAFT -> state transitions to draft_saved`.",
        "Side-effect class: `FORMAL_SUBMISSION -> state transitions to bid_submitted`.",
    ] {
        ensure!(
            markdown.contains(side_effect_contract),
            "bid portal skill missing `{side_effect_contract}`"
        );
    }

    for failure_mode in [
        "`READ_ONLY`: Safe to retry",
        "`REMOTE_DRAFT`: Call `verifyPortalState()` before retry",
        "`NOTIFICATION_OR_LOCK`: Stop at `portal_state_unknown`",
        "`LEGAL_DECLARATION`: Stop at `portal_state_unknown`",
        "`FORMAL_SUBMISSION`: Never blind retry",
        "`FUNDS_COMMITTED`: Procontract bid workflow must not intentionally reach this class",
    ] {
        ensure!(
            markdown.contains(failure_mode),
            "bid portal skill missing failure mode `{failure_mode}`"
        );
    }

    for prohibition in [
        "DO NOT launch a fresh browser profile",
        "DO NOT close the operator's browser",
        "DO NOT sign out",
        "DO NOT solve captchas",
        "DO NOT type credentials directly",
        "DO NOT invoke auth-signin/procontract as a sub-skill",
        "DO NOT escalate side-effect class past BidScopePayload.sideEffectCap",
        "DO NOT submit any form OTHER than the bid response form",
        "DO NOT cache or log filled values to disk",
        "DO NOT proceed past Step 4 if verifyPortalState detects an existing submission",
        "DO NOT continue if portal state diverged from the expected opportunity",
    ] {
        ensure!(
            markdown.contains(prohibition),
            "bid portal skill missing prohibition `{prohibition}`"
        );
    }

    Ok(())
}

#[test]
fn ut_skl_009_l3_promotion_metadata_is_visible_in_linted_operational_skills() -> Result<()> {
    let auth_refresh_alibaba = lint_skill_directory(&repo_skill_dir("auth-refresh/alibaba")?)?;
    ensure!(auth_refresh_alibaba.metadata.skill_version == DEFAULT_SOURCE_SKILL_VERSION);
    ensure!(auth_refresh_alibaba.metadata.interaction_profile == "pty_interactive");
    ensure!(auth_refresh_alibaba.metadata.model_profile == "browse_stable_low_cost");
    ensure!(auth_refresh_alibaba.metadata.side_effect_class == "local_state");
    ensure!(auth_refresh_alibaba.metadata.promotion_stage.as_deref() == Some("stable"));
    ensure!(
        auth_refresh_alibaba
            .metadata
            .worker_behavior_version
            .as_deref()
            == Some("auth-refresh/alibaba@2026.03.30-1+1.0.0")
    );
    ensure!(
        auth_refresh_alibaba
            .metadata
            .gate_b_exact_slice_key
            .as_deref()
            == Some(
                "auth-refresh/alibaba::alibaba::auth-refresh/alibaba@2026.03.30-1+1.0.0::2026.03.30-1"
            )
    );

    let auth_refresh_1688 = lint_skill_directory(&repo_skill_dir("auth-refresh/1688")?)?;
    ensure!(auth_refresh_1688.metadata.skill_version == DEFAULT_SOURCE_SKILL_VERSION);
    ensure!(auth_refresh_1688.metadata.interaction_profile == "pty_interactive");
    ensure!(auth_refresh_1688.metadata.model_profile == "browse_stable_low_cost");
    ensure!(auth_refresh_1688.metadata.side_effect_class == "local_state");
    ensure!(auth_refresh_1688.metadata.promotion_stage.as_deref() == Some("stable"));
    ensure!(
        auth_refresh_1688
            .metadata
            .worker_behavior_version
            .as_deref()
            == Some("auth-refresh/1688@2026.03.30-1+1.0.0")
    );
    ensure!(
        auth_refresh_1688.metadata.gate_b_exact_slice_key.as_deref()
            == Some("auth-refresh/1688::1688::auth-refresh/1688@2026.03.30-1+1.0.0::2026.03.30-1")
    );

    let source_health = lint_skill_directory(&repo_skill_dir("source-health/fts")?)?;
    ensure!(source_health.metadata.skill_version == DEFAULT_SOURCE_SKILL_VERSION);
    ensure!(source_health.metadata.promotion_stage.as_deref() == Some("stable"));
    ensure!(source_health.metadata.model_profile == "browse_stable_low_cost");
    ensure!(source_health.metadata.side_effect_class == "none");
    ensure!(source_health.metadata.source_structure_version.as_deref() == Some("fts-2026-03-30"));
    ensure!(
        source_health.metadata.worker_behavior_version.as_deref()
            == Some("source-health/fts@2026.03.30-1+1.0.0")
    );
    ensure!(source_health.metadata.validator_overlay_version.as_deref() == Some("2026.03.30-1"));
    ensure!(source_health.metadata.gate_a_family_key.as_deref() == Some("source-health"));
    ensure!(
        source_health.metadata.gate_b_exact_slice_key.as_deref()
            == Some("source-health/fts::fts::source-health/fts@2026.03.30-1+1.0.0::2026.03.30-1")
    );
    ensure!(!source_health.metadata.gate_b_exact_slice_ready);

    let source_health_alibaba = lint_skill_directory(&repo_skill_dir("source-health/alibaba")?)?;
    ensure!(source_health_alibaba.metadata.skill_version == DEFAULT_SOURCE_SKILL_VERSION);
    ensure!(source_health_alibaba.metadata.interaction_profile == "pty_interactive");
    ensure!(source_health_alibaba.metadata.model_profile == "browse_stable_low_cost");
    ensure!(source_health_alibaba.metadata.side_effect_class == "none");
    ensure!(source_health_alibaba.metadata.promotion_stage.as_deref() == Some("stable"));
    ensure!(
        source_health_alibaba
            .metadata
            .worker_behavior_version
            .as_deref()
            == Some("source-health/alibaba@2026.03.30-1+1.0.0")
    );
    ensure!(
        source_health_alibaba
            .metadata
            .gate_b_exact_slice_key
            .as_deref()
            == Some(
                "source-health/alibaba::alibaba::source-health/alibaba@2026.03.30-1+1.0.0::2026.03.30-1"
            )
    );

    let validator = lint_skill_directory(&repo_skill_dir("validation/base")?)?;
    ensure!(validator.metadata.promotion_stage.as_deref() == Some("stable"));
    ensure!(validator.metadata.validator_overlay_version.as_deref() == Some("2026.03.30-1"));
    ensure!(
        validator.metadata.worker_behavior_version.as_deref()
            == Some("validation/base@2026.03.30-1+1.0")
    );
    ensure!(validator.metadata.gate_a_family_key.as_deref() == Some("validation"));
    Ok(())
}

#[test]
fn ut_skl_010_non_behavior_affecting_changes_require_explicit_behavior_identity() -> Result<()> {
    let dir = TestSkillDir::from_fixture(&repo_skill_dir("source-health/fts")?)?;
    let invalid_markdown = dir
        .read_skill_markdown()?
        .replace(
            "worker_behavior_version: source-health/fts@2026.03.30-1+1.0.0\n",
            "",
        )
        .replace(
            "gate_b_exact_slice_key: source-health/fts::fts::source-health/fts@2026.03.30-1+1.0.0::2026.03.30-1\n",
            "",
        )
        .replace("version_change_affects_behavior: true\n", "version_change_affects_behavior: false\n");
    dir.overwrite("SKILL.md", &invalid_markdown)?;

    let error = lint_skill_directory(dir.path())
        .expect_err("non-behavior-affecting metadata without pinned behavior identity should fail");
    let error_text = error.to_string();
    ensure!(
        error_text.contains("worker_behavior_version")
            || error_text.contains("gate_b_exact_slice_key")
    );
    Ok(())
}

#[test]
fn l5_t_freight_engine_010_1688_maturation_contract() -> Result<()> {
    let skill_dir = repo_skill_dir("supplier-price-enrichment/1688")?;
    let linted = lint_skill_directory(&skill_dir)?;
    assert_1688_metadata_and_skill_text(&linted)?;
    assert_1688_result_schema(&skill_dir)?;
    let canary_text = assert_1688_canaries(&skill_dir)?;
    let golden = assert_1688_golden_and_rubric(&skill_dir)?;
    assert_1688_calibration_mirror(&canary_text, &golden)?;
    Ok(())
}

fn assert_1688_metadata_and_skill_text(linted: &LintedSkill) -> Result<()> {
    ensure!(linted.metadata.promotion_stage.as_deref() == Some("stable"));
    ensure!(
        linted.metadata.worker_behavior_version.as_deref()
            == Some("supplier-price-enrichment/1688@2.0.0")
    );
    ensure!(
        linted.metadata.validator_behavior_version.as_deref()
            == Some("validation/overlay-customs-classification@1.0.0+1.0")
    );
    ensure!(
        linted
            .metadata
            .gate_b_exact_slice_key
            .as_deref()
            .is_some_and(|key| key.contains("supplier-price-enrichment/1688@2.0.0")
                && key.contains("validation/overlay-customs-classification@1.0.0+1.0"))
    );
    ensure!(linted.metadata.gate_a_family_prequalified);
    ensure!(linted.metadata.gate_b_exact_slice_ready);

    for required in [
        "validator_overlay: validation/overlay-customs-classification",
        "price_signal_role: \"domestic_wholesale_floor\"",
        "Chinese-only short clusters of 2-4 terms",
        "auth-refresh/1688",
        "1688_AUTH_REFRESH_REQUIRED",
        "1688_REGULATED_GOODS_EVIDENCE_MISSING",
        "1688_VOCABULARY_MISMATCH_NO_VALID_ZH_ALIAS",
        "lexicon expansion",
    ] {
        ensure!(
            linted.skill_markdown.contains(required),
            "1688 SKILL.md missing `{required}`"
        );
    }
    Ok(())
}

fn assert_1688_result_schema(skill_dir: &std::path::Path) -> Result<()> {
    let result_schema: JsonValue = serde_json::from_str(&std::fs::read_to_string(
        skill_dir.join("result.schema.json"),
    )?)?;
    ensure!(
        result_schema
            .pointer("/properties/supplier_quotes/items/properties/price_signal_role/const")
            .and_then(JsonValue::as_str)
            == Some("domestic_wholesale_floor")
    );
    let attempt_strategies = result_schema
        .pointer("/properties/search_attempts/items/properties/strategy/enum")
        .and_then(JsonValue::as_array)
        .expect("search_attempt strategy enum");
    for strategy in [
        "short_cluster",
        "auth_refresh_preflight",
        "lexicon_expansion",
    ] {
        ensure!(
            attempt_strategies
                .iter()
                .any(|value| value.as_str() == Some(strategy)),
            "1688 result schema missing strategy `{strategy}`"
        );
    }
    let failure_codes = result_schema
        .pointer("/properties/extraction_metadata/properties/failure_code/enum")
        .and_then(JsonValue::as_array)
        .expect("failure_code enum");
    for code in [
        "1688_AUTH_REFRESH_REQUIRED",
        "1688_REGULATED_GOODS_EVIDENCE_MISSING",
        "1688_VOCABULARY_MISMATCH_NO_VALID_ZH_ALIAS",
        "VOCABULARY_MISMATCH_EN_ONLY",
    ] {
        ensure!(
            failure_codes
                .iter()
                .any(|value| value.as_str() == Some(code)),
            "1688 result schema missing failure code `{code}`"
        );
    }
    Ok(())
}

fn assert_1688_canaries(skill_dir: &std::path::Path) -> Result<String> {
    let canary_text = std::fs::read_to_string(skill_dir.join("canary.inputs.jsonl"))?;
    let mut case_count = 0;
    let mut categories = BTreeSet::new();
    let mut intents = BTreeSet::new();
    let mut successful_short_clusters = 0;
    for line in canary_text.lines().filter(|line| !line.trim().is_empty()) {
        let row: JsonValue = serde_json::from_str(line)?;
        case_count += 1;
        ensure!(row.get("source_id").and_then(JsonValue::as_str) == Some("1688"));
        ensure!(
            row.get("expected_price_signal_role")
                .and_then(JsonValue::as_str)
                == Some("domestic_wholesale_floor")
        );
        categories.insert(
            row.get("category")
                .and_then(JsonValue::as_str)
                .expect("canary category")
                .to_owned(),
        );
        intents.insert(
            row.get("case_intent")
                .and_then(JsonValue::as_str)
                .expect("canary case_intent")
                .to_owned(),
        );
        let profile = row.get("search_profile").expect("search_profile");
        ensure!(
            profile
                .get("noun_aliases_zh")
                .and_then(JsonValue::as_array)
                .is_some(),
            "1688 canary missing noun_aliases_zh"
        );
        for wave in profile
            .get("query_waves_json")
            .and_then(JsonValue::as_array)
            .expect("query_waves_json")
        {
            ensure!(
                wave.get("query_en").is_none(),
                "1688 canaries must not send English query text"
            );
            if wave.get("strategy").and_then(JsonValue::as_str) == Some("short_cluster") {
                let term_count = wave
                    .get("term_count")
                    .and_then(JsonValue::as_i64)
                    .expect("short cluster term_count");
                ensure!(
                    (2..=4).contains(&term_count),
                    "short_cluster term_count must be 2-4"
                );
                ensure!(
                    wave.get("query_zh")
                        .and_then(JsonValue::as_str)
                        .is_some_and(|query| !query.trim().is_empty()),
                    "short_cluster query_zh must be populated"
                );
                successful_short_clusters += 1;
            }
        }
    }
    ensure!(
        case_count >= 5,
        "1688 canary set must contain at least five cases"
    );
    ensure!(
        categories
            == BTreeSet::from([
                "examination-gloves".to_owned(),
                "fasteners".to_owned(),
                "packaging-cartons".to_owned(),
            ]),
        "1688 canaries must cover all three L5 goods categories"
    );
    for intent in [
        "domestic_wholesale_floor_success",
        "short_cluster_pruning_to_two_terms",
        "regulated_goods_quote_suppressed_without_export_compliance",
        "auth_refresh_preflight_blocks_production_search",
        "vocabulary_mismatch_routes_to_lexicon_expansion",
    ] {
        ensure!(
            intents.contains(intent),
            "1688 canaries missing intent `{intent}`"
        );
    }
    ensure!(successful_short_clusters >= 4);
    Ok(canary_text)
}

fn assert_1688_golden_and_rubric(skill_dir: &std::path::Path) -> Result<String> {
    let golden = std::fs::read_to_string(skill_dir.join("golden.assertions.yaml"))?;
    let rubric = std::fs::read_to_string(skill_dir.join("validator-rubric.yaml"))?;
    let _: serde_yaml::Value = serde_yaml::from_str(&golden)?;
    let _: serde_yaml::Value = serde_yaml::from_str(&rubric)?;
    for required in [
        "L5-T-freight_engine-010",
        "L5-T-freight_engine-013",
        "L5-T-freight_engine-018",
        "domestic_wholesale_floor",
        "auth_refresh_1688_runs_before_production_search",
        "regulated_goods_without_export_compliance_suppresses_supplier_quote",
        "incompatible_versions_block_promotion: true",
    ] {
        ensure!(
            golden.contains(required) || rubric.contains(required),
            "1688 golden/rubric missing `{required}`"
        );
    }
    Ok(golden)
}

fn assert_1688_calibration_mirror(canary_text: &str, golden: &str) -> Result<()> {
    let mirror_dir = repo_root()?.join("tests/fixtures/calibration/marketplace-search/1688");
    ensure!(
        std::fs::read_to_string(mirror_dir.join("canary.inputs.jsonl"))? == canary_text,
        "1688 calibration mirror canaries must match the skill canaries"
    );
    ensure!(
        std::fs::read_to_string(mirror_dir.join("golden.assertions.yaml"))? == golden,
        "1688 calibration mirror golden assertions must match the skill assertions"
    );
    Ok(())
}

#[test]
fn l5_t_freight_engine_013_018_marketplace_fixture_promotion_gate() -> Result<()> {
    let repo_root = repo_root()?;
    for expected in marketplace_promotion_expectations() {
        assert_marketplace_promotion_artifact(&repo_root, expected)?;
    }

    ensure!(marketplace_behavior_pair_compatible(
        "supplier-price-enrichment/amazon@2.0.0",
        "validation/overlay-customs-classification@1.0.0+1.0"
    ));
    ensure!(!marketplace_behavior_pair_compatible(
        "supplier-price-enrichment/amazon@1.0.0",
        "validation/overlay-customs-classification@1.0.0+1.0"
    ));
    ensure!(!marketplace_promotion_authoritative(
        "candidate",
        true,
        false,
        true
    ));
    ensure!(marketplace_promotion_authoritative(
        "candidate",
        true,
        true,
        true
    ));
    ensure!(marketplace_promotion_authoritative(
        "stable", false, false, true
    ));
    ensure!(
        rollback_failed_marketplace_stable("supplier-price-enrichment/1688@2.0.0")
            == (
                "supplier-price-enrichment/1688@2.0.0".to_owned(),
                "candidate"
            )
    );

    Ok(())
}

fn assert_marketplace_promotion_artifact(
    repo_root: &std::path::Path,
    expected: MarketplacePromotionExpectation<'_>,
) -> Result<()> {
    let skill_dir = repo_root.join("skills").join(expected.skill_relative);
    let linted = load_skill_metadata(&skill_dir)?;
    ensure!(
        linted.metadata.promotion_stage.as_deref() == Some(expected.promotion_stage),
        "{} promotion stage drifted",
        expected.source_id
    );
    ensure!(
        linted.metadata.worker_behavior_version.as_deref()
            == Some(expected.worker_behavior_version),
        "{} worker behavior version drifted",
        expected.source_id
    );
    ensure!(
        linted.metadata.validator_behavior_version.as_deref()
            == Some(expected.validator_behavior_version),
        "{} validator behavior version drifted",
        expected.source_id
    );
    assert_marketplace_standard_files(&skill_dir, expected.source_id)?;
    let golden = assert_marketplace_golden_and_canary(&skill_dir, expected)?;
    assert_marketplace_calibration_mirror(repo_root, &skill_dir, expected, &golden)?;
    if expected.promotion_stage == "stable" {
        ensure!(
            linted.metadata.gate_a_family_prequalified,
            "{} stable promotion requires family prequalification",
            expected.source_id
        );
        ensure!(
            linted.metadata.gate_b_exact_slice_ready,
            "{} stable promotion requires exact-slice readiness",
            expected.source_id
        );
    }
    Ok(())
}

fn assert_marketplace_standard_files(skill_dir: &std::path::Path, source_id: &str) -> Result<()> {
    for file in MARKETPLACE_PROMOTION_REQUIRED_FILES {
        ensure!(
            skill_dir.join(file).exists(),
            "{source_id} missing standard production file {file}"
        );
    }
    Ok(())
}

fn assert_marketplace_golden_and_canary(
    skill_dir: &std::path::Path,
    expected: MarketplacePromotionExpectation<'_>,
) -> Result<String> {
    let golden = std::fs::read_to_string(skill_dir.join("golden.assertions.yaml"))?;
    let _: serde_yaml::Value = serde_yaml::from_str(&golden)?;
    for line in std::fs::read_to_string(skill_dir.join("canary.inputs.jsonl"))?
        .lines()
        .filter(|line| !line.trim().is_empty())
    {
        let _: JsonValue = serde_json::from_str(line)?;
    }
    for required in [
        "L5-T-freight_engine-013",
        "L5-T-freight_engine-018",
        expected.worker_behavior_version,
        expected.validator_behavior_version,
    ] {
        ensure!(
            golden.contains(required),
            "{} golden assertions missing `{required}`",
            expected.source_id
        );
    }
    if let Some(price_signal_role) = expected.price_signal_role {
        ensure!(
            golden.contains(price_signal_role),
            "{} golden assertions missing price role `{price_signal_role}`",
            expected.source_id
        );
    }
    Ok(golden)
}

fn assert_marketplace_calibration_mirror(
    repo_root: &std::path::Path,
    skill_dir: &std::path::Path,
    expected: MarketplacePromotionExpectation<'_>,
    golden: &str,
) -> Result<()> {
    let mirror_dir = repo_root
        .join("tests/fixtures/calibration/marketplace-search")
        .join(expected.mirror_id);
    ensure!(
        std::fs::read_to_string(mirror_dir.join("canary.inputs.jsonl"))?
            == std::fs::read_to_string(skill_dir.join("canary.inputs.jsonl"))?,
        "{} calibration mirror canaries must match the skill canaries",
        expected.source_id
    );
    ensure!(
        std::fs::read_to_string(mirror_dir.join("golden.assertions.yaml"))? == golden,
        "{} calibration mirror golden assertions must match the skill assertions",
        expected.source_id
    );
    Ok(())
}

#[derive(Clone, Copy)]
struct MarketplacePromotionExpectation<'a> {
    source_id: &'a str,
    mirror_id: &'a str,
    skill_relative: &'a str,
    promotion_stage: &'a str,
    worker_behavior_version: &'a str,
    validator_behavior_version: &'a str,
    price_signal_role: Option<&'a str>,
}

const MARKETPLACE_PROMOTION_REQUIRED_FILES: [&str; 6] = [
    "SKILL.md",
    "result.schema.json",
    "checkpoint.schema.json",
    "canary.inputs.jsonl",
    "golden.assertions.yaml",
    "validator-rubric.yaml",
];

const fn marketplace_promotion_expectations() -> [MarketplacePromotionExpectation<'static>; 5] {
    [
        MarketplacePromotionExpectation {
            source_id: "ebay",
            mirror_id: "ebay",
            skill_relative: "supplier-price-enrichment/ebay",
            promotion_stage: "stable",
            worker_behavior_version: "skills/supplier-price-enrichment/ebay@1.0.0",
            validator_behavior_version: "validation/overlay-customs-classification@1.0.0+1.0",
            price_signal_role: Some("benchmark_ceiling"),
        },
        MarketplacePromotionExpectation {
            source_id: "alibaba",
            mirror_id: "alibaba",
            skill_relative: "supplier-price-enrichment/alibaba",
            promotion_stage: "stable",
            worker_behavior_version: "skills/supplier-price-enrichment/alibaba@2.0.0",
            validator_behavior_version: "validation/overlay-customs-classification@1.0.0+1.0",
            price_signal_role: Some("optimistic_floor"),
        },
        MarketplacePromotionExpectation {
            source_id: "1688",
            mirror_id: "1688",
            skill_relative: "supplier-price-enrichment/1688",
            promotion_stage: "stable",
            worker_behavior_version: "supplier-price-enrichment/1688@2.0.0",
            validator_behavior_version: "validation/overlay-customs-classification@1.0.0+1.0",
            price_signal_role: Some("domestic_wholesale_floor"),
        },
        MarketplacePromotionExpectation {
            source_id: "amazon",
            mirror_id: "amazon",
            skill_relative: "supplier-price-enrichment/amazon",
            promotion_stage: "stable",
            worker_behavior_version: "supplier-price-enrichment/amazon@2.0.0",
            validator_behavior_version: "validation/overlay-customs-classification@1.0.0+1.0",
            price_signal_role: Some("benchmark_ceiling"),
        },
        MarketplacePromotionExpectation {
            source_id: "pcs",
            mirror_id: "pcs",
            skill_relative: "procurement-ingestion/pcs",
            promotion_stage: "candidate",
            worker_behavior_version: "procurement-ingestion/pcs@1.0.0",
            validator_behavior_version: "validation/overlay-customs-classification@1.0.0+1.0",
            price_signal_role: None,
        },
    ]
}

fn marketplace_behavior_pair_compatible(worker: &str, validator: &str) -> bool {
    marketplace_promotion_expectations().iter().any(|expected| {
        expected.worker_behavior_version == worker
            && expected.validator_behavior_version == validator
    })
}

fn marketplace_promotion_authoritative(
    stage: &str,
    canary_passed: bool,
    shadow_passed: bool,
    compatible_pair: bool,
) -> bool {
    compatible_pair && (stage == "stable" || (canary_passed && shadow_passed))
}

fn rollback_failed_marketplace_stable(failed_version: &str) -> (String, &'static str) {
    (failed_version.to_owned(), "candidate")
}

#[test]
fn l5_t_freight_engine_016_source_health_aggregate_skill_artifacts() -> Result<()> {
    let skill_dir = repo_skill_dir("source-health/l5-marketplace-commercial-intelligence")?;
    let linted = lint_skill_directory(&skill_dir)?;
    assert_l5_source_health_metadata(&linted)?;
    assert_l5_source_health_result_schema(&skill_dir)?;
    assert_l5_source_health_canaries(&skill_dir)?;
    assert_l5_source_health_golden_and_rubric(&skill_dir)
}

fn assert_l5_source_health_metadata(linted: &LintedSkill) -> Result<()> {
    ensure!(linted.metadata.skill_version == "2026.05.19-1");
    ensure!(linted.metadata.interaction_profile == "headless_contract");
    ensure!(linted.metadata.promotion_stage.as_deref() == Some("candidate"));
    ensure!(linted.metadata.model_profile == "tier_1_lightweight");
    ensure!(linted.metadata.side_effect_class == "none");
    ensure!(
        linted.metadata.source_structure_version.as_deref()
            == Some("l5-marketplace-commercial-intelligence-2026-05-19")
    );
    ensure!(
        linted.metadata.worker_behavior_version.as_deref()
            == Some("source-health/l5-marketplace-commercial-intelligence@2026.05.19-1+1.0.0")
    );
    ensure!(linted.metadata.validator_behavior_version.as_deref() == Some("2026.03.30-1"));
    ensure!(linted.metadata.gate_a_family_key.as_deref() == Some("source-health"));
    ensure!(
        linted.metadata.gate_b_exact_slice_key.as_deref()
            == Some(
                "source-health/l5-marketplace-commercial-intelligence::l5-marketplace-commercial-intelligence::source-health/l5-marketplace-commercial-intelligence@2026.05.19-1+1.0.0::2026.03.30-1"
            )
    );
    for required in [
        "Alibaba, 1688, Amazon, PCS Scotland, SeaRates, Freightos, and TARIC",
        "auth-refresh/1688",
        "paid_api_auth_check",
        "Do not omit sources",
    ] {
        ensure!(
            linted.skill_markdown.contains(required),
            "L5 aggregate source-health SKILL.md missing `{required}`"
        );
    }
    Ok(())
}

fn assert_l5_source_health_result_schema(skill_dir: &std::path::Path) -> Result<()> {
    let result_schema: JsonValue = serde_json::from_str(&std::fs::read_to_string(
        skill_dir.join("result.schema.json"),
    )?)?;
    let required = result_schema
        .pointer("/properties/per_source_health/required")
        .and_then(JsonValue::as_array)
        .expect("per_source_health.required should be present");
    let required_sources = required
        .iter()
        .filter_map(JsonValue::as_str)
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();
    ensure!(required_sources == l5_source_set());
    let paid_auth = result_schema
        .pointer(
            "/properties/per_source_health/patternProperties/^(alibaba|1688|amazon|pcs|searates|freightos|taric)$/properties/paid_api_auth_check/const",
        )
        .and_then(JsonValue::as_bool);
    ensure!(paid_auth == Some(false));
    Ok(())
}

fn assert_l5_source_health_canaries(skill_dir: &std::path::Path) -> Result<()> {
    let canary_text = std::fs::read_to_string(skill_dir.join("canary.inputs.jsonl"))?;
    let mut seen = BTreeSet::new();
    for line in canary_text.lines().filter(|line| !line.trim().is_empty()) {
        let row: JsonValue = serde_json::from_str(line)?;
        let source_id = row
            .get("source_id")
            .and_then(JsonValue::as_str)
            .expect("source_id should be present");
        seen.insert(source_id.to_owned());
        ensure!(
            row.get("paid_api_auth_check").and_then(JsonValue::as_bool) == Some(false),
            "{source_id} must not request paid API auth checks"
        );
        ensure!(
            row.get("canary_fixture")
                .and_then(JsonValue::as_str)
                .is_some_and(|path| !path.trim().is_empty()),
            "{source_id} must name a canary fixture"
        );
    }
    ensure!(seen == l5_source_set());
    Ok(())
}

fn assert_l5_source_health_golden_and_rubric(skill_dir: &std::path::Path) -> Result<()> {
    let golden = std::fs::read_to_string(skill_dir.join("golden.assertions.yaml"))?;
    let rubric = std::fs::read_to_string(skill_dir.join("validator-rubric.yaml"))?;
    let _: serde_yaml::Value = serde_yaml::from_str(&golden)?;
    let _: serde_yaml::Value = serde_yaml::from_str(&rubric)?;
    for required in [
        "L5-T-freight_engine-016",
        "paid_api_auth_check_is_false_for_every_source",
        "auth_refresh_1688_coordination_path_exists",
        "seven_source_coverage",
        "no_paid_api_auth",
        "stable_promotion_owner: l5-p5-marketplace-fixture-promotion-gate",
    ] {
        ensure!(
            golden.contains(required) || rubric.contains(required),
            "L5 aggregate source-health golden/rubric missing `{required}`"
        );
    }
    Ok(())
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

#[test]
fn ut_skl_011_cdp_browser_skills_have_preflight_and_no_bare_agent_browser_commands() -> Result<()> {
    for relative in CDP_BROWSER_SKILLS {
        let skill = load_skill_metadata(&repo_skill_dir(relative)?)?;
        ensure!(
            skill.metadata.requires_cdp,
            "{relative} must declare requires_cdp=true"
        );
        ensure!(
            skill.metadata.cdp_default_port == Some(9222),
            "{relative} must declare cdp_default_port: 9222"
        );
        ensure!(
            skill.skill_markdown.contains("agent-browser --cdp"),
            "{relative} must show agent-browser --cdp usage"
        );
        ensure!(
            skill
                .skill_markdown
                .contains("http://localhost:${CDP_PORT}/json/version"),
            "{relative} must contain the localhost CDP preflight"
        );
        ensure!(
            !skill.skill_markdown.contains("agent-browser close"),
            "{relative} must not close the operator browser"
        );
        ensure!(
            !skill.skill_markdown.contains("agent-browser install")
                && !skill.skill_markdown.contains("agent-browser --help"),
            "{relative} must not run browser setup/help commands"
        );

        for subcommand in AGENT_BROWSER_SUBCOMMANDS {
            let bare_command = format!("agent-browser {subcommand}");
            ensure!(
                !skill.skill_markdown.contains(&bare_command),
                "{relative} contains bare `{bare_command}` instead of `agent-browser --cdp`"
            );
        }
    }
    Ok(())
}

#[test]
fn ut_skl_012_signed_in_browser_skills_require_operator_cdp_session() -> Result<()> {
    for relative in SIGNED_IN_BROWSER_SKILLS {
        let skill = load_skill_metadata(&repo_skill_dir(relative)?)?;
        ensure!(
            skill.metadata.requires_cdp,
            "{relative} depends on signed-in browser state and must require CDP"
        );
        ensure!(
            skill.metadata.cdp_default_port == Some(9222),
            "{relative} must default to operator CDP port 9222"
        );
        ensure!(
            !skill.skill_markdown.contains("Password:")
                && !skill.skill_markdown.contains("Username:"),
            "{relative} must not embed credential-driven login instructions"
        );
        ensure!(
            !skill.skill_markdown.contains("log in with")
                && !skill.skill_markdown.contains("authenticate with"),
            "{relative} must block for operator auth instead of logging in"
        );
    }
    Ok(())
}

#[test]
fn ut_skl_302_cdp_skills_invoke_ensure_helper() -> Result<()> {
    // Every CDP-requiring skill must invoke bin/ensure-cdp-9222 in its body so
    // a Chrome that has crashed mid-execution can be relaunched without the
    // operator having to intervene manually. Self-policing — newly-added
    // CDP-requiring skills will fail this lint until they wire the helper in.
    for relative in CDP_BROWSER_SKILLS {
        let skill = load_skill_metadata(&repo_skill_dir(relative)?)?;
        ensure!(
            skill.skill_markdown.contains("bin/ensure-cdp-9222"),
            "{relative} declares requires_cdp:true but does not invoke bin/ensure-cdp-9222 in its body"
        );
    }
    Ok(())
}
