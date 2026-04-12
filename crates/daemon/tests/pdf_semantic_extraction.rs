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
fn test_tender_spec_contract_maps_to_single_lots_target_domain() -> Result<()> {
    let schema = tender_spec_schema()?;
    let extraction_metadata = &schema["properties"]["extraction_metadata"];
    let target_domain = &extraction_metadata["properties"]["target_domain"];

    ensure!(
        target_domain["const"] == Value::String("lots".to_owned()),
        "tender-spec semantic rows must persist with target_domain=lots"
    );

    let required = extraction_metadata["required"]
        .as_array()
        .context("extraction_metadata.required missing")?;
    ensure!(
        required.contains(&Value::String("extraction_scope".to_owned())),
        "tender-spec extraction metadata must expose extraction_scope"
    );
    ensure!(
        required.contains(&Value::String("policy_version".to_owned())),
        "tender-spec extraction metadata must expose policy_version"
    );
    Ok(())
}

#[test]
fn test_tender_spec_contract_carries_semantic_persistence_fields() -> Result<()> {
    let schema = tender_spec_schema()?;
    let required = schema["required"]
        .as_array()
        .context("schema required array missing")?;

    for field in [
        "field_confidences",
        "extraction_confidence_bps",
        "warnings",
        "evidence_anchors",
        "extraction_metadata",
    ] {
        ensure!(
            required.contains(&Value::String(field.to_owned())),
            "tender-spec schema missing required persistence field {field}"
        );
    }
    Ok(())
}
