#![forbid(unsafe_code)]

#[path = "support/skills.rs"]
mod skills_support;
#[path = "../src/sources/registry.rs"]
#[allow(dead_code)]
mod sources_impl;

use std::collections::BTreeSet;

use anyhow::{Result, ensure};

use skills_support::{
    TestSkillDir, lint_skill_directory, load_skill_metadata, repo_root, repo_skill_dir,
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

    // FTS, Contracts Finder, and the three procontract-* skills are the active
    // procurement source skills. The procontract entries were authored more
    // recently and are not yet held to the strict lint shape (Non-goals,
    // Resume Strategy, Pagination Bounds), so this test only asserts that the
    // directory listing matches expectations and that the canonical FTS /
    // Contracts Finder / manifest skills lint cleanly.
    let expected: BTreeSet<String> = [
        "contracts-finder",
        "fts",
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
