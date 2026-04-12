#![forbid(unsafe_code)]

#[path = "support/skills.rs"]
mod skills_support;

use anyhow::{Result, ensure};

use skills_support::{lint_skill_directory, repo_skill_dir};

const EXPECTED_BASE_SKILL_FRONTMATTER: &str = "\
---\n\
skill_id: pdf-extraction/base\n\
skill_version: 1.0.0\n\
interaction_profile: headless_contract\n\
result_schema_version: 1.0.0\n\
model_profile: structured_extract_low_cost\n\
agent_cli: claude\n\
side_effect_class: none\n\
---\n";

fn lint_base_skill() -> Result<skills_support::LintedSkill> {
    lint_skill_directory(&repo_skill_dir("pdf-extraction/base")?)
}

#[test]
fn test_base_skill_lint() -> Result<()> {
    let linted = lint_base_skill()?;
    ensure!(linted.metadata.skill_id == "pdf-extraction/base");
    ensure!(linted.metadata.skill_version == "1.0.0");
    ensure!(linted.metadata.result_schema_version == "1.0.0");
    ensure!(
        linted
            .skill_markdown
            .starts_with(EXPECTED_BASE_SKILL_FRONTMATTER),
        "base pdf-extraction skill frontmatter drifted from the bead contract"
    );
    Ok(())
}

#[test]
fn test_interaction_profile_headless_contract() -> Result<()> {
    let linted = lint_base_skill()?;
    ensure!(linted.metadata.interaction_profile == "headless_contract");
    Ok(())
}

#[test]
fn test_model_profile_low_cost() -> Result<()> {
    let linted = lint_base_skill()?;
    ensure!(linted.metadata.model_profile == "structured_extract_low_cost");
    Ok(())
}

#[test]
fn test_side_effect_class_none() -> Result<()> {
    let linted = lint_base_skill()?;
    ensure!(linted.metadata.side_effect_class == "none");
    Ok(())
}
