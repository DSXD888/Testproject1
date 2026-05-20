#![forbid(unsafe_code)]

mod support;

use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, ensure};
use rusqlite::OptionalExtension;
use serde_json::Value;

use support::db::{DEFAULT_SCHEMA_EPOCH, L4TestStateDb};

// L5 ownership:
// - L5-T-customs_classification-027,028,029,030,031,032,035
//
// Downstream L5 beads extend this target with gold/silver/bronze corpus
// scoring, abstention-positive-class metrics, and mutation-family sweeps.
const L5_PRODUCT_COMMODITY_CODES_MIGRATION: &str = "0013_l5_product_commodity_codes";
const FASTENERS_FIXTURE_ROOT: &str = "tests/fixtures/calibration/customs-classification/fasteners";
const REQUIRED_FASTENERS_MUTATION_FAMILIES: [&str; 13] = [
    "clean",
    "weird",
    "mutated",
    "pairwise",
    "source_identity",
    "page_kind",
    "currency_locale",
    "unit_basis",
    "product_identity",
    "warning_omission",
    "count_mismatch",
    "fabricated_evidence",
    "continuation_integrity",
];
const REQUIRED_FASTENERS_MUTATION_DIMENSIONS: [&str; 10] = [
    "hs6_perturbation",
    "exporter_swap",
    "country_of_origin_swap",
    "currency_swap",
    "packaging_weight_perturbation",
    "supplementary_unit_removal",
    "incoterm_swap",
    "additional_code_evidence_removal",
    "vat_relief_evidence_removal",
    "taric_hmrc_disagreement_injection",
];

fn assert_l5_product_commodity_codes_schema_applied() -> Result<()> {
    let db = L4TestStateDb::new()?;
    let conn = db.connection();

    let user_version: i64 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;
    ensure!(
        user_version == DEFAULT_SCHEMA_EPOCH,
        "L5 customs classification accuracy smoke expected schema epoch {DEFAULT_SCHEMA_EPOCH}, got {user_version}"
    );

    let migration_id: Option<String> = conn
        .query_row(
            "SELECT id FROM schema_migrations WHERE id = ?1",
            [L5_PRODUCT_COMMODITY_CODES_MIGRATION],
            |row| row.get(0),
        )
        .optional()?;
    ensure!(
        migration_id.as_deref() == Some(L5_PRODUCT_COMMODITY_CODES_MIGRATION),
        "L5 customs classification accuracy smoke did not observe migration {L5_PRODUCT_COMMODITY_CODES_MIGRATION}"
    );

    let table_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'product_commodity_codes'",
        [],
        |row| row.get(0),
    )?;
    ensure!(
        table_count == 1,
        "L5 customs classification accuracy smoke did not observe product_commodity_codes"
    );

    Ok(())
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn fasteners_fixture_root() -> PathBuf {
    repo_root().join(FASTENERS_FIXTURE_ROOT)
}

fn read_json(path: &Path) -> Result<Value> {
    let body =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    serde_json::from_str(&body).with_context(|| format!("failed to parse {}", path.display()))
}

fn read_jsonl(path: &Path) -> Result<Vec<Value>> {
    let body =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    body.lines()
        .enumerate()
        .map(|(index, line)| {
            serde_json::from_str(line)
                .with_context(|| format!("failed to parse {} line {}", path.display(), index + 1))
        })
        .collect()
}

fn string_field<'a>(value: &'a Value, field: &str) -> Result<&'a str> {
    value
        .get(field)
        .and_then(Value::as_str)
        .with_context(|| format!("missing string field {field}"))
}

fn nested_string_field<'a>(value: &'a Value, object: &str, field: &str) -> Result<&'a str> {
    value
        .get(object)
        .and_then(|nested| nested.get(field))
        .and_then(Value::as_str)
        .with_context(|| format!("missing string field {object}.{field}"))
}

fn nested_bool_field(value: &Value, object: &str, field: &str) -> Result<bool> {
    value
        .get(object)
        .and_then(|nested| nested.get(field))
        .and_then(Value::as_bool)
        .with_context(|| format!("missing bool field {object}.{field}"))
}

fn metadata_count(metadata: &Value, section: &str, field: &str) -> Result<u64> {
    metadata
        .get(section)
        .and_then(|nested| nested.get(field))
        .and_then(Value::as_u64)
        .with_context(|| format!("missing numeric metadata field {section}.{field}"))
}

fn assert_fasteners_root_files(root: &Path) -> Result<()> {
    for file_name in [
        "canary.inputs.jsonl",
        "golden.assertions.yaml",
        "metadata.json",
        "manifest.md",
    ] {
        ensure!(
            root.join(file_name).is_file(),
            "fasteners fixture missing root file {file_name}"
        );
    }

    for variant in ["clean", "weird", "mutated", "pairwise"] {
        let variant_root = root.join(variant);
        ensure!(
            variant_root.is_dir(),
            "fasteners fixture missing variant directory {variant}"
        );
        ensure!(
            variant_root.join("metadata.json").is_file(),
            "fasteners fixture missing {variant}/metadata.json"
        );
        ensure!(
            variant_root.join("manifest.md").is_file(),
            "fasteners fixture missing {variant}/manifest.md"
        );
    }

    Ok(())
}

fn assert_fasteners_metadata(root: &Path) -> Result<()> {
    let metadata = read_json(&root.join("metadata.json"))?;
    ensure!(
        metadata.get("fixture_surface").and_then(Value::as_str) == Some("calibration-corpus"),
        "fasteners metadata must identify the calibration corpus surface"
    );
    ensure!(
        metadata.get("category").and_then(Value::as_str) == Some("fasteners"),
        "fasteners metadata category mismatch"
    );
    ensure!(
        metadata.get("trade_remedy_active").and_then(Value::as_bool) == Some(true),
        "fasteners metadata must mark trade remedy active"
    );

    ensure!(
        metadata_count(&metadata, "minimum_corpus", "gold")? == 10
            && metadata_count(&metadata, "minimum_corpus", "silver")? == 15
            && metadata_count(&metadata, "minimum_corpus", "bronze")? == 10
            && metadata_count(&metadata, "minimum_corpus", "pairwise_pairs")? == 12,
        "fasteners metadata minimum corpus counts changed"
    );
    ensure!(
        metadata_count(&metadata, "actual_corpus", "gold")? == 47
            && metadata_count(&metadata, "actual_corpus", "silver")? == 63
            && metadata_count(&metadata, "actual_corpus", "bronze")? == 10
            && metadata_count(&metadata, "actual_corpus", "pairwise_pairs")? == 24
            && metadata_count(&metadata, "actual_corpus", "total_cases")? == 120,
        "fasteners metadata actual corpus counts changed"
    );

    Ok(())
}

fn assert_fasteners_manifest_and_golden(root: &Path) -> Result<()> {
    let manifest_path = root.join("manifest.md");
    let manifest = fs::read_to_string(&manifest_path)
        .with_context(|| format!("failed to read {}", manifest_path.display()))?;
    for expected in [
        "Fixture surface: calibration-corpus",
        "gold: 47 cases",
        "silver: 63 cases",
        "bronze: 10 cases",
        "pairwise: 24 pairs",
        "HS Chapter 73",
        "Chapter 76",
        "Steel AD/CVD active",
    ] {
        ensure!(
            manifest.contains(expected),
            "fasteners manifest missing expected text {expected}"
        );
    }

    let golden_path = root.join("golden.assertions.yaml");
    let golden = fs::read_to_string(&golden_path)
        .with_context(|| format!("failed to read {}", golden_path.display()))?;
    for expected in [
        "L5-T-customs_classification-027",
        "production_gate: eligible",
        "correct_abstention_count: 37",
        "pairwise_pairs: 24",
    ] {
        ensure!(
            golden.contains(expected),
            "fasteners golden assertions missing expected text {expected}"
        );
    }

    Ok(())
}

fn assert_silver_reviewers(accuracy_case: &Value, case_id: &str) -> Result<()> {
    let reviewers = accuracy_case
        .get("reviewers")
        .and_then(Value::as_array)
        .with_context(|| format!("silver case {case_id} missing reviewers"))?;
    let distinct_reasoned_reviewers = reviewers
        .iter()
        .filter_map(|reviewer| {
            let reviewer_id = reviewer.get("reviewerId").and_then(Value::as_str)?;
            let reasoning = reviewer.get("reasoning").and_then(Value::as_str)?;
            (!reasoning.trim().is_empty()).then_some(reviewer_id)
        })
        .collect::<HashSet<_>>();
    ensure!(
        distinct_reasoned_reviewers.len() >= 2,
        "silver case {case_id} needs two independent reviewers with reasoning"
    );
    Ok(())
}

fn assert_fasteners_case_contract(root: &Path) -> Result<()> {
    let cases = read_jsonl(&root.join("canary.inputs.jsonl"))?;
    ensure!(
        cases.len() == 120,
        "fasteners JSONL expected 120 cases, got {}",
        cases.len()
    );

    let mut gold_count = 0_usize;
    let mut silver_count = 0_usize;
    let mut bronze_count = 0_usize;
    let mut correct_abstention_count = 0_usize;
    let mut covered_families = HashSet::new();
    let mut covered_dimensions = HashSet::new();
    let mut pair_counts = HashMap::<String, usize>::new();

    for accuracy_case in &cases {
        let case_id = string_field(accuracy_case, "caseId")?;
        ensure!(
            string_field(accuracy_case, "category")? == "fasteners",
            "case {case_id} is not in the fasteners category"
        );

        match string_field(accuracy_case, "labelTier")? {
            "gold" => gold_count += 1,
            "silver" => {
                silver_count += 1;
                assert_silver_reviewers(accuracy_case, case_id)?;
            }
            "bronze" => bronze_count += 1,
            tier => anyhow::bail!("case {case_id} has unknown label tier {tier}"),
        }

        let mutation_family = string_field(accuracy_case, "mutationFamily")?;
        covered_families.insert(mutation_family.to_owned());
        if let Some(dimension) = accuracy_case
            .get("mutationDimension")
            .and_then(Value::as_str)
        {
            covered_dimensions.insert(dimension.to_owned());
        }
        if mutation_family == "pairwise" {
            let pair_id = string_field(accuracy_case, "pairId")?;
            *pair_counts.entry(pair_id.to_owned()).or_insert(0) += 1;
        }

        let expected_decision =
            nested_string_field(accuracy_case, "groundTruth", "expectedDecision")?;
        let abstained = nested_bool_field(accuracy_case, "systemOutput", "abstained")?;
        if abstained && expected_decision != "auto_pass" {
            correct_abstention_count += 1;
        }
    }

    ensure!(
        gold_count == 47 && silver_count == 63 && bronze_count == 10,
        "fasteners label counts were gold={gold_count}, silver={silver_count}, bronze={bronze_count}"
    );
    ensure!(
        correct_abstention_count == 37,
        "fasteners expected 37 correct abstentions, got {correct_abstention_count}"
    );
    ensure!(
        pair_counts.len() == 24,
        "fasteners expected 24 pairwise pairs, got {}",
        pair_counts.len()
    );
    ensure!(
        pair_counts.values().all(|count| *count == 2),
        "every fasteners pairwise pair must have exactly two records"
    );

    for family in REQUIRED_FASTENERS_MUTATION_FAMILIES {
        ensure!(
            covered_families.contains(family),
            "fasteners corpus missing mutation family {family}"
        );
    }
    for dimension in REQUIRED_FASTENERS_MUTATION_DIMENSIONS {
        ensure!(
            covered_dimensions.contains(dimension),
            "fasteners corpus missing customs mutation dimension {dimension}"
        );
    }

    let rule_of_three_upper_bound_bps = 30_000_usize.div_ceil(cases.len());
    ensure!(
        rule_of_three_upper_bound_bps <= 300,
        "fasteners corpus too small for zero-event production gate: {rule_of_three_upper_bound_bps} bps"
    );

    Ok(())
}

#[test]
fn l5_customs_classification_accuracy_smoke_l5_t_customs_classification_027_028_029_030_031_032_035()
-> Result<()> {
    assert_l5_product_commodity_codes_schema_applied()
}

#[test]
fn l5_fasteners_goldset_corpus_l5_t_customs_classification_027() -> Result<()> {
    let root = fasteners_fixture_root();
    assert_fasteners_root_files(&root)?;
    assert_fasteners_metadata(&root)?;
    assert_fasteners_manifest_and_golden(&root)?;
    assert_fasteners_case_contract(&root)
}
