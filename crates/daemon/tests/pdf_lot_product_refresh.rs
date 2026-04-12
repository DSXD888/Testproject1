#![forbid(unsafe_code)]

#[path = "support/skills.rs"]
#[allow(dead_code)]
mod skills_support;

use std::fs;

use anyhow::{ensure, Context, Result};
use serde_json::Value;

use skills_support::repo_skill_dir;

fn tender_spec_schema() -> Result<Value> {
    let path = repo_skill_dir("pdf-extraction/tender-specification")?.join("result.schema.json");
    let raw = fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    serde_json::from_str(&raw).with_context(|| format!("parsing {}", path.display()))
}

#[test]
fn test_tender_spec_schema_exposes_lot_refresh_fields() -> Result<()> {
    let schema = tender_spec_schema()?;
    let lot_required = schema["properties"]["lots"]["items"]["required"]
        .as_array()
        .context("lots item required array missing")?;

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
            "lot refresh contract missing field {field}"
        );
    }
    Ok(())
}

#[test]
fn test_tender_spec_schema_exposes_product_seed_refresh_fields() -> Result<()> {
    let schema = tender_spec_schema()?;
    let seed_required = schema["properties"]["product_requirement_seeds"]["items"]["required"]
        .as_array()
        .context("product_requirement_seeds required array missing")?;

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
            "product seed refresh contract missing field {field}"
        );
    }
    Ok(())
}
