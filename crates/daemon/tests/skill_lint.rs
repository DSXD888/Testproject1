#![forbid(unsafe_code)]

#[path = "support/skills.rs"]
mod skills_support;

use std::collections::BTreeSet;

use anyhow::{Result, ensure};

use skills_support::{TestSkillDir, lint_skill_directory, repo_root, repo_skill_dir};

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

    ensure!(
        procurement_skill_dirs
            == BTreeSet::from(["contracts-finder".to_owned(), "fts".to_owned(),]),
        "FTS and Contracts Finder must be the active procurement source skills in Layer 2"
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
    let source_health = lint_skill_directory(&repo_skill_dir("source-health/fts")?)?;
    ensure!(source_health.metadata.promotion_stage.as_deref() == Some("stable"));
    ensure!(source_health.metadata.source_structure_version.as_deref() == Some("fts-2026-03-30"));
    ensure!(
        source_health.metadata.worker_behavior_version.as_deref()
            == Some("source-health/fts@1.0.0+1.0.0")
    );
    ensure!(source_health.metadata.validator_overlay_version.as_deref() == Some("2026.03.30-1"));
    ensure!(source_health.metadata.gate_a_family_key.as_deref() == Some("source-health"));
    ensure!(
        source_health.metadata.gate_b_exact_slice_key.as_deref()
            == Some("source-health/fts::fts::source-health/fts@1.0.0+1.0.0::2026.03.30-1")
    );
    ensure!(!source_health.metadata.gate_b_exact_slice_ready);

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
            "worker_behavior_version: source-health/fts@1.0.0+1.0.0\n",
            "",
        )
        .replace(
            "gate_b_exact_slice_key: source-health/fts::fts::source-health/fts@1.0.0+1.0.0::2026.03.30-1\n",
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
