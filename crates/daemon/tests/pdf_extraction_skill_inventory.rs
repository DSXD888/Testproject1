#![forbid(unsafe_code)]

#[path = "support/skills.rs"]
mod skills_support;

use anyhow::{anyhow, ensure, Result};
use serde_json::Value;

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

const EXPECTED_TENDER_SPEC_SKILL_FRONTMATTER: &str = "\
---\n\
skill_id: pdf-extraction/tender-specification\n\
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

fn lint_tender_spec_skill() -> Result<skills_support::LintedSkill> {
    lint_skill_directory(&repo_skill_dir("pdf-extraction/tender-specification")?)
}

fn tender_spec_schema() -> Result<Value> {
    let schema_path =
        repo_skill_dir("pdf-extraction/tender-specification")?.join("result.schema.json");
    let schema = std::fs::read_to_string(&schema_path)?;
    Ok(serde_json::from_str(&schema)?)
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

#[test]
fn test_tender_spec_skill_lint() -> Result<()> {
    let linted = lint_tender_spec_skill()?;
    ensure!(
        linted
            .skill_markdown
            .starts_with(EXPECTED_TENDER_SPEC_SKILL_FRONTMATTER),
        "tender-specification frontmatter drifted from the bead contract"
    );
    ensure!(linted.metadata.skill_id == "pdf-extraction/tender-specification");
    ensure!(linted.metadata.interaction_profile == "headless_contract");
    ensure!(linted.metadata.model_profile == "structured_extract_low_cost");
    ensure!(linted.metadata.side_effect_class == "none");
    Ok(())
}

#[test]
fn test_tender_spec_schema_contains_lot_and_product_requirement_fields() -> Result<()> {
    let schema = tender_spec_schema()?;
    let lot_required = schema["properties"]["lots"]["items"]["required"]
        .as_array()
        .ok_or_else(|| anyhow!("lots required array missing"))?;
    let seed_required = schema["properties"]["product_requirement_seeds"]["items"]["required"]
        .as_array()
        .ok_or_else(|| anyhow!("product_requirement_seeds required array missing"))?;

    for field in [
        "lot_number",
        "title",
        "description",
        "requirements",
        "evaluation_criteria",
        "cpv_codes",
        "product_requirements",
        "technical_standards",
        "evidence_anchor",
    ] {
        ensure!(
            lot_required.contains(&Value::String(field.to_owned())),
            "tender-spec schema missing lot field {field}"
        );
    }

    for field in [
        "seed_key",
        "lot_number",
        "candidate_name",
        "summary",
        "requirements",
        "evidence_anchor",
    ] {
        ensure!(
            seed_required.contains(&Value::String(field.to_owned())),
            "tender-spec schema missing product seed field {field}"
        );
    }

    ensure!(
        schema["properties"]["extraction_metadata"]["properties"]["target_domain"]["const"]
            == Value::String("lots".to_owned()),
        "tender-spec extraction metadata must pin target_domain to lots"
    );
    Ok(())
}
