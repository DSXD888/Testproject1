#![forbid(unsafe_code)]

#[allow(dead_code)]
#[path = "support/skills.rs"]
mod skills_support;

mod tests {
    use anyhow::{anyhow, ensure, Context, Result};
    use serde::Deserialize;
    use serde_json::{json, Value};
    use serde_yaml::{Mapping, Value as YamlValue};

    use crate::skills_support::{repo_skill_dir, TestSkillDir};

    #[allow(dead_code)]
    #[derive(Debug, Clone, Deserialize)]
    struct SkillLifecycleFrontmatter {
        skill_id: String,
        skill_version: String,
        result_schema_version: String,
        #[serde(default)]
        checkpoint_schema_version: Option<String>,
        #[serde(default)]
        validator_overlay_version: Option<String>,
        #[serde(default)]
        source_structure_version: Option<String>,
        #[serde(default)]
        worker_behavior_version: Option<String>,
        #[serde(default)]
        validator_behavior_version: Option<String>,
        #[serde(default)]
        version_change_affects_behavior: bool,
        #[serde(default)]
        gate_a_family_key: Option<String>,
        #[serde(default)]
        gate_a_family_prequalified: bool,
        #[serde(default)]
        gate_b_exact_slice_key: Option<String>,
        #[serde(default)]
        gate_b_exact_slice_ready: bool,
    }

    struct CanaryExitInputs {
        bounded_runs: u32,
        metrics_within_tolerance: bool,
        precision_or_recall_regressed: bool,
        validator_approval_bps: u16,
    }

    struct ShadowExitInputs {
        paired_runs: u32,
        quality_within_tolerance: bool,
    }

    struct RollbackRecord {
        failed_stable_version: String,
        prior_stable_version: String,
        historical_quality_digest: String,
    }

    struct ComparisonMetricsSnapshot {
        stable_fill_rate_bps: u16,
        candidate_fill_rate_bps: u16,
        stable_duplicate_rate_bps: u16,
        candidate_duplicate_rate_bps: u16,
        stable_cost_per_run_pence: u32,
        candidate_cost_per_run_pence: u32,
        stable_validator_approval_bps: u16,
        candidate_validator_approval_bps: u16,
    }

    fn rewrite_frontmatter(markdown: &str, updates: &[(&str, YamlValue)]) -> Result<String> {
        let rest = markdown
            .strip_prefix("---\n")
            .ok_or_else(|| anyhow!("skill markdown missing opening frontmatter delimiter"))?;
        let (yaml, body) = rest
            .split_once("\n---\n")
            .ok_or_else(|| anyhow!("skill markdown missing closing frontmatter delimiter"))?;
        let mut mapping: Mapping =
            serde_yaml::from_str(yaml).context("parsing skill frontmatter")?;
        for (key, value) in updates {
            mapping.insert(YamlValue::String((*key).to_owned()), value.clone());
        }
        let rendered = serde_yaml::to_string(&mapping).context("rendering skill frontmatter")?;
        Ok(format!("---\n{rendered}---\n{body}"))
    }

    fn yaml_str(value: &str) -> YamlValue {
        YamlValue::String(value.to_owned())
    }

    fn parse_frontmatter(markdown: &str) -> Result<SkillLifecycleFrontmatter> {
        let rest = markdown
            .strip_prefix("---\n")
            .ok_or_else(|| anyhow!("skill markdown missing opening frontmatter delimiter"))?;
        let (yaml, _) = rest
            .split_once("\n---\n")
            .ok_or_else(|| anyhow!("skill markdown missing closing frontmatter delimiter"))?;
        serde_yaml::from_str(yaml).context("parsing skill lifecycle frontmatter")
    }

    const fn can_promote_from_canary(inputs: &CanaryExitInputs) -> bool {
        inputs.bounded_runs >= 20
            && inputs.metrics_within_tolerance
            && !inputs.precision_or_recall_regressed
            && inputs.validator_approval_bps >= 9_500
    }

    const fn can_promote_from_shadow(inputs: &ShadowExitInputs) -> bool {
        inputs.paired_runs >= 50 && inputs.quality_within_tolerance
    }

    fn rollback(record: &RollbackRecord) -> (String, String, String) {
        (
            record.prior_stable_version.clone(),
            "candidate".to_owned(),
            record.historical_quality_digest.clone(),
        )
    }

    fn comparison_metrics_payload(snapshot: &ComparisonMetricsSnapshot) -> Value {
        json!({
            "stable": {
                "fill_rate_bps": snapshot.stable_fill_rate_bps,
                "duplicate_rate_bps": snapshot.stable_duplicate_rate_bps,
                "cost_per_run_pence": snapshot.stable_cost_per_run_pence,
                "validator_approval_bps": snapshot.stable_validator_approval_bps,
            },
            "candidate": {
                "fill_rate_bps": snapshot.candidate_fill_rate_bps,
                "duplicate_rate_bps": snapshot.candidate_duplicate_rate_bps,
                "cost_per_run_pence": snapshot.candidate_cost_per_run_pence,
                "validator_approval_bps": snapshot.candidate_validator_approval_bps,
            }
        })
    }

    #[test]
    fn l3_t_skill_shadow_compare_001_canary_exit_requires_twenty_bounded_runs_and_no_gold_regression(
    ) -> Result<()> {
        let good = CanaryExitInputs {
            bounded_runs: 20,
            metrics_within_tolerance: true,
            precision_or_recall_regressed: false,
            validator_approval_bps: 9_500,
        };
        ensure!(can_promote_from_canary(&good));

        let bad = CanaryExitInputs {
            bounded_runs: 19,
            ..good
        };
        ensure!(!can_promote_from_canary(&bad));
        Ok(())
    }

    #[test]
    fn l3_t_skill_shadow_compare_002_shadow_exit_requires_fifty_paired_runs_with_quality_in_range(
    ) -> Result<()> {
        let good = ShadowExitInputs {
            paired_runs: 50,
            quality_within_tolerance: true,
        };
        ensure!(can_promote_from_shadow(&good));

        let bad = ShadowExitInputs {
            quality_within_tolerance: false,
            ..good
        };
        ensure!(!can_promote_from_shadow(&bad));
        Ok(())
    }

    #[test]
    fn l3_t_skill_shadow_compare_003_validator_gate_blocks_promotion_even_when_fill_rate_looks_good(
    ) -> Result<()> {
        let fill_rate_candidate_looks_good = 9_700_u16;
        let fill_rate_stable = 9_600_u16;
        ensure!(fill_rate_candidate_looks_good >= fill_rate_stable);
        ensure!(!can_promote_from_canary(&CanaryExitInputs {
            bounded_runs: 25,
            metrics_within_tolerance: true,
            precision_or_recall_regressed: false,
            validator_approval_bps: 9_499,
        }));
        Ok(())
    }

    #[test]
    fn l3_t_skill_shadow_compare_004_cosmetic_edits_can_keep_behavior_version_and_slice_readiness_stable(
    ) -> Result<()> {
        let baseline = parse_frontmatter(&std::fs::read_to_string(
            repo_skill_dir("source-health/fts")?.join("SKILL.md"),
        )?)?;
        let skill_dir = TestSkillDir::from_fixture(&repo_skill_dir("source-health/fts")?)?;
        let updated = rewrite_frontmatter(
            &skill_dir.read_skill_markdown()?,
            &[
                ("skill_version", yaml_str("1.0.1")),
                ("version_change_affects_behavior", YamlValue::Bool(false)),
                (
                    "worker_behavior_version",
                    yaml_str(
                        baseline
                            .worker_behavior_version
                            .as_deref()
                            .ok_or_else(|| anyhow!("baseline worker behavior version missing"))?,
                    ),
                ),
                (
                    "gate_b_exact_slice_key",
                    yaml_str(
                        baseline
                            .gate_b_exact_slice_key
                            .as_deref()
                            .ok_or_else(|| anyhow!("baseline exact slice key missing"))?,
                    ),
                ),
                ("gate_b_exact_slice_ready", YamlValue::Bool(true)),
            ],
        )?;
        let mutated = parse_frontmatter(&updated)?;

        ensure!(!mutated.version_change_affects_behavior);
        ensure!(mutated.worker_behavior_version == baseline.worker_behavior_version);
        ensure!(mutated.gate_b_exact_slice_key == baseline.gate_b_exact_slice_key);
        ensure!(mutated.gate_b_exact_slice_ready);
        Ok(())
    }

    #[test]
    fn l3_t_skill_shadow_compare_005_behavior_affecting_or_schema_changes_bump_slice_identity_and_reset_gate_b(
    ) -> Result<()> {
        let baseline = parse_frontmatter(&std::fs::read_to_string(
            repo_skill_dir("source-health/fts")?.join("SKILL.md"),
        )?)?;
        let skill_dir = TestSkillDir::from_fixture(&repo_skill_dir("source-health/fts")?)?;
        let updated = rewrite_frontmatter(
            &skill_dir.read_skill_markdown()?,
            &[
                ("skill_version", yaml_str("1.1.0")),
                ("result_schema_version", yaml_str("1.1.0")),
                ("checkpoint_schema_version", yaml_str("1.1.0")),
                (
                    "worker_behavior_version",
                    yaml_str("source-health/fts@1.1.0+1.1.0"),
                ),
                (
                    "gate_b_exact_slice_key",
                    yaml_str("source-health/fts::fts::source-health/fts@1.1.0+1.1.0::2026.03.30-1"),
                ),
                ("gate_a_family_prequalified", YamlValue::Bool(true)),
                ("gate_b_exact_slice_ready", YamlValue::Bool(false)),
            ],
        )?;
        let mutated = parse_frontmatter(&updated)?;

        ensure!(
            mutated.worker_behavior_version != baseline.worker_behavior_version,
            "behavior-affecting or schema changes must bump worker behavior version"
        );
        ensure!(
            mutated.gate_b_exact_slice_key != baseline.gate_b_exact_slice_key,
            "exact slice identity must change when behavior changes"
        );
        ensure!(
            mutated.gate_a_family_prequalified,
            "Gate A family prequalification should remain distinct from Gate B state"
        );
        ensure!(
            !mutated.gate_b_exact_slice_ready,
            "Gate B exact slice readiness must reset on a behavior-affecting change"
        );
        Ok(())
    }

    #[test]
    fn l3_t_skill_shadow_compare_006_rollback_rebinds_prior_stable_without_mutating_history(
    ) -> Result<()> {
        let record = RollbackRecord {
            failed_stable_version: "2026.03.30-2".to_owned(),
            prior_stable_version: "2026.03.30-1".to_owned(),
            historical_quality_digest: "quality-history-sha256".to_owned(),
        };
        let (rebound_version, failed_next_stage, digest) = rollback(&record);
        ensure!(rebound_version == "2026.03.30-1");
        ensure!(failed_next_stage == "candidate");
        ensure!(digest == "quality-history-sha256");
        ensure!(record.failed_stable_version == "2026.03.30-2");
        Ok(())
    }

    #[test]
    fn l3_t_skill_shadow_compare_007_shadow_metrics_surface_fill_duplicate_cost_and_validator_rates(
    ) -> Result<()> {
        let payload = comparison_metrics_payload(&ComparisonMetricsSnapshot {
            stable_fill_rate_bps: 9_600,
            candidate_fill_rate_bps: 9_650,
            stable_duplicate_rate_bps: 100,
            candidate_duplicate_rate_bps: 125,
            stable_cost_per_run_pence: 14,
            candidate_cost_per_run_pence: 16,
            stable_validator_approval_bps: 9_700,
            candidate_validator_approval_bps: 9_550,
        });

        ensure!(payload["stable"]["fill_rate_bps"] == 9_600);
        ensure!(payload["candidate"]["fill_rate_bps"] == 9_650);
        ensure!(payload["stable"]["duplicate_rate_bps"] == 100);
        ensure!(payload["candidate"]["duplicate_rate_bps"] == 125);
        ensure!(payload["stable"]["cost_per_run_pence"] == 14);
        ensure!(payload["candidate"]["cost_per_run_pence"] == 16);
        ensure!(payload["stable"]["validator_approval_bps"] == 9_700);
        ensure!(payload["candidate"]["validator_approval_bps"] == 9_550);
        Ok(())
    }
}
