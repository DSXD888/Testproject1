#![forbid(unsafe_code)]

#[path = "../src/prompt/renderer.rs"]
mod prompt_renderer_impl;
#[path = "support/skills.rs"]
mod skills_support;

use anyhow::{Result, ensure};

use skills_support::{TestSkillDir, lint_skill_directory, repo_root, repo_skill_dir};

#[test]
fn ut_skl_001_required_files_exist_in_every_production_skill_directory() -> Result<()> {
    let fts = lint_skill_directory(&repo_skill_dir("procurement-ingestion/fts")?)?;
    let manifest = lint_skill_directory(&repo_skill_dir("manifest-generation")?)?;
    let procurement_skill_dirs: Vec<_> =
        std::fs::read_dir(repo_root()?.join("skills/procurement-ingestion"))?
            .filter_map(|entry| entry.ok().map(|value| value.path()))
            .collect();

    ensure!(
        procurement_skill_dirs.len() == 1
            && procurement_skill_dirs[0].ends_with("procurement-ingestion/fts"),
        "FTS must be the only active procurement source in Layer 1"
    );
    ensure!(fts.metadata.interaction_profile == "pty_interactive");
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
    ensure!(fts_markdown.contains(prompt_renderer_impl::RUN_MANIFEST_INSTRUCTION_BLOCK));

    let dir = TestSkillDir::from_fixture(&repo_skill_dir("manifest-generation")?)?;
    let invalid_markdown = dir.read_skill_markdown()?.replace(
        prompt_renderer_impl::RUN_MANIFEST_INSTRUCTION_BLOCK,
        "Write a brief summary.",
    );
    dir.overwrite("SKILL.md", &invalid_markdown)?;

    let error = lint_skill_directory(dir.path()).expect_err("missing manifest block should fail");
    ensure!(error.to_string().contains("run-manifest instructions"));
    Ok(())
}
