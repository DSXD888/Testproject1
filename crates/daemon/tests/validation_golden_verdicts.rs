#![forbid(unsafe_code)]

mod support;

#[path = "support/skills.rs"]
mod skills_support;

mod jobs {
    pub mod runner {
        use anyhow::Result;
        use chrono::{DateTime, Utc};
        use serde::{Deserialize, Serialize};
        use serde_json::Value;

        use crate::support::db::StateDb;

        #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
        pub struct MutationAck {
            pub accepted: bool,
            pub visible_after_seq: i64,
            pub continuation_job_id: Option<String>,
        }

        #[derive(Debug, Clone, PartialEq, Eq)]
        pub struct EnqueueJobRequest {
            pub job_id: String,
            pub kind: String,
            pub process_id: Option<String>,
            pub notice_id: Option<String>,
            pub scope_kind: Option<String>,
            pub scope_id: Option<String>,
            pub parent_job_id: Option<String>,
            pub continuation_chain_id: Option<String>,
            pub validation_required: bool,
            pub scheduled_for: Option<DateTime<Utc>>,
            pub input_payload: Value,
            pub idempotency_key: String,
        }

        #[derive(Debug, Clone)]
        pub struct JobRunner;

        impl JobRunner {
            #[allow(
                clippy::missing_const_for_fn,
                clippy::unnecessary_wraps,
                clippy::unused_self
            )]
            pub fn enqueue_job(
                &self,
                _state_db: &mut StateDb,
                _request: &EnqueueJobRequest,
            ) -> Result<MutationAck> {
                Ok(MutationAck {
                    accepted: true,
                    visible_after_seq: 1,
                    continuation_job_id: None,
                })
            }
        }
    }
}

#[path = "../src/validation/executor.rs"]
#[allow(dead_code)]
mod executor_impl;

mod tests {
    use std::collections::BTreeSet;
    use std::fs;

    use anyhow::{Result, ensure};
    use serde::Deserialize;
    use serde_json::{Value, json};

    use crate::executor_impl::{
        ProcessValidationState, ValidationHandlingMode, ValidationTrustState,
        ValidatorBasisSufficiency, ValidatorIssue, ValidatorResultContract, ValidatorSkillBundle,
        ValidatorVerdict, decide_validator_verdict, parse_validator_result_contract,
    };
    use crate::skills_support::{lint_skill_directory, repo_root, repo_skill_dir};

    #[derive(Debug, Deserialize)]
    struct CanaryInputCase {
        case_id: String,
        corpus: String,
        manifest_path: String,
        expected_verdict: String,
        expected_basis_sufficiency: String,
        provenance: String,
        notes: String,
    }

    #[derive(Debug, Deserialize)]
    struct GoldenAssertions {
        cases: Vec<GoldenAssertionCase>,
        calibration_targets: CalibrationTargets,
        cross_model_audit: CrossModelAudit,
    }

    #[derive(Debug, Deserialize)]
    struct GoldenAssertionCase {
        case_id: String,
        expected_verdict: String,
        expected_basis_sufficiency: String,
        required_issue_categories: Vec<String>,
        provenance: String,
    }

    #[allow(clippy::struct_field_names)]
    #[derive(Debug, Deserialize)]
    struct CalibrationTargets {
        mutated_bad_catch_rate_min_bps: u32,
        clean_real_approval_min_bps: u32,
        false_positive_rate_max_bps: u32,
        wilson_confidence_bps: u32,
    }

    #[derive(Debug, Deserialize)]
    struct CrossModelAudit {
        required: bool,
        comparison_model_family: String,
    }

    fn load_bundle() -> Result<ValidatorSkillBundle> {
        ValidatorSkillBundle::load(&repo_root()?)
    }

    fn load_canary_cases() -> Result<Vec<CanaryInputCase>> {
        let path = repo_skill_dir("validation/base")?.join("canary.inputs.jsonl");
        let contents = fs::read_to_string(&path)?;
        contents
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| Ok(serde_json::from_str::<CanaryInputCase>(line)?))
            .collect()
    }

    fn load_golden_assertions() -> Result<GoldenAssertions> {
        let path = repo_skill_dir("validation/base")?.join("golden.assertions.yaml");
        Ok(serde_yaml::from_str(&fs::read_to_string(path)?)?)
    }

    fn issue(severity: &str, category: &str, description: &str, evidence_refs: &[&str]) -> Value {
        json!({
            "severity": severity,
            "category": category,
            "description": description,
            "evidence_refs": evidence_refs,
        })
    }

    fn payload(
        verdict: ValidatorVerdict,
        basis_sufficiency: ValidatorBasisSufficiency,
        enforcement_eligible: bool,
        confidence_bps: u16,
        issues: &[Value],
        summary: &str,
    ) -> Value {
        json!({
            "verdict": verdict.as_str(),
            "basis_sufficiency": basis_sufficiency.as_str(),
            "enforcement_eligible": enforcement_eligible,
            "confidence_bps": confidence_bps,
            "issues": issues.to_vec(),
            "summary": summary,
        })
    }

    fn parse_payload(payload: &Value) -> Result<ValidatorResultContract> {
        let bundle = load_bundle()?;
        parse_validator_result_contract(&bundle.result_schema, payload)
    }

    fn rate_bps(numerator: u32, denominator: u32) -> u32 {
        assert!(denominator > 0, "denominator must be non-zero");
        let scaled = (u64::from(numerator) * 10_000_u64 + u64::from(denominator) / 2)
            / u64::from(denominator);
        let Ok(scaled) = u32::try_from(scaled) else {
            return u32::MAX;
        };
        scaled
    }

    fn wilson_lower_bound_bps(successes: u32, total: u32, z_score: f64) -> f64 {
        assert!(total > 0, "total must be non-zero");
        let n = f64::from(total);
        let phat = f64::from(successes) / n;
        let z2 = z_score * z_score;
        let variance_term = phat.mul_add(1.0 - phat, z2 / (4.0 * n));
        let numerator = z_score.mul_add(-((variance_term / n).sqrt()), phat + z2 / (2.0 * n));
        let denominator = 1.0 + z2 / n;
        (numerator / denominator) * 10_000.0
    }

    fn issue_categories(contract: &ValidatorResultContract) -> BTreeSet<&str> {
        contract
            .issues
            .iter()
            .map(|issue| issue.category.as_str())
            .collect()
    }

    fn issue_severities(contract: &ValidatorResultContract) -> BTreeSet<&str> {
        contract
            .issues
            .iter()
            .map(|issue| issue.severity.as_str())
            .collect()
    }

    #[test]
    fn l3_t_validation_golden_verdicts_001_validator_skill_directory_contains_all_required_assets()
    -> Result<()> {
        let skill_dir = repo_skill_dir("validation/base")?;
        let linted = lint_skill_directory(&skill_dir)?;
        let bundle = load_bundle()?;
        let canary_cases = load_canary_cases()?;
        let golden_assertions = load_golden_assertions()?;

        for relative in [
            "SKILL.md",
            "result.schema.json",
            "checkpoint.schema.json",
            "canary.inputs.jsonl",
            "golden.assertions.yaml",
            "validator-rubric.yaml",
            "references/manifest-corpus-index.md",
        ] {
            ensure!(
                skill_dir.join(relative).is_file(),
                "L3-T-validation_golden_verdicts-001 missing `{relative}`"
            );
        }

        ensure!(linted.metadata.skill_id == "validation/base");
        ensure!(bundle.skill_path == "skills/validation/base/SKILL.md");
        ensure!(
            bundle.validator_rubric_hash.len() == 64,
            "expected SHA-256 validator rubric hash"
        );
        ensure!(
            canary_cases.len() == 3,
            "expected exactly three validator canary fixtures"
        );
        ensure!(
            canary_cases
                .iter()
                .map(|case| case.corpus.as_str())
                .collect::<BTreeSet<_>>()
                == BTreeSet::from(["clean-real", "mutated-bad", "weird-but-valid"]),
            "expected explicit clean-real / weird-but-valid / mutated-bad corpora"
        );
        ensure!(
            golden_assertions.cases.len() == 3,
            "expected golden assertions for all three corpora"
        );
        Ok(())
    }

    #[test]
    fn l3_t_validation_golden_verdicts_002_clean_real_manifest_is_approved_with_non_low_basis()
    -> Result<()> {
        let contract = parse_payload(&payload(
            ValidatorVerdict::Approved,
            ValidatorBasisSufficiency::High,
            true,
            9_800,
            &[],
            "Complete manifest agrees with daemon appendix evidence.",
        ))?;

        let decision = decide_validator_verdict(
            &contract,
            ValidationHandlingMode {
                dry_run: true,
                enforcement_active: false,
            },
        );

        ensure!(contract.verdict == ValidatorVerdict::Approved);
        ensure!(contract.basis_sufficiency != ValidatorBasisSufficiency::Low);
        ensure!(decision.trust_state == ValidationTrustState::Verified);
        ensure!(decision.process_validation_state == ProcessValidationState::Validated);
        Ok(())
    }

    #[test]
    fn l3_t_validation_golden_verdicts_003_mutated_bad_manifest_is_rejected_with_explicit_evidence_refs()
    -> Result<()> {
        let issues = vec![
            issue(
                "critical",
                "consistency",
                "Agent narrative contradicts daemon appendix counts.",
                &[
                    "run-manifest.md#Summary",
                    "Daemon Evidence Appendix > representative_item_sample",
                ],
            ),
            issue(
                "major",
                "plausibility",
                "Claimed sample pricing is implausible for the observed market slice.",
                &[
                    "run-manifest.md#Evidence",
                    "artifacts/corpus/mutated-bad/manifest-001.md",
                ],
            ),
        ];
        let contract = parse_payload(&payload(
            ValidatorVerdict::Rejected,
            ValidatorBasisSufficiency::High,
            true,
            9_600,
            &issues,
            "Mutated manifest is internally inconsistent and not fit for approval.",
        ))?;

        ensure!(contract.verdict == ValidatorVerdict::Rejected);
        ensure!(
            issue_categories(&contract) == BTreeSet::from(["consistency", "plausibility"]),
            "mutated-bad verdict must surface explicit consistency and plausibility categories"
        );
        ensure!(
            contract
                .issues
                .iter()
                .all(|issue: &ValidatorIssue| !issue.evidence_refs.is_empty()),
            "every mutated-bad issue must carry evidence refs"
        );
        ensure!(
            contract
                .issues
                .iter()
                .flat_map(|issue| issue.evidence_refs.iter())
                .all(|reference| {
                    reference.contains("run-manifest")
                        || reference.contains("Daemon Evidence Appendix")
                        || reference.contains("artifacts/")
                }),
            "mutated-bad evidence refs must point at manifest or appendix anchors"
        );
        Ok(())
    }

    #[test]
    fn l3_t_validation_golden_verdicts_004_weird_but_valid_cases_are_flagged_or_low_basis_not_rejected()
    -> Result<()> {
        let issues = vec![issue(
            "observation",
            "completeness",
            "Manifest is unusual and missing some corroborating appendix evidence.",
            &["run-manifest.md#Warnings"],
        )];
        let contract = parse_payload(&payload(
            ValidatorVerdict::Flagged,
            ValidatorBasisSufficiency::Low,
            false,
            6_200,
            &issues,
            "Manifest is unusual but not clearly wrong.",
        ))?;

        ensure!(contract.verdict != ValidatorVerdict::Rejected);
        ensure!(
            contract.verdict == ValidatorVerdict::Flagged
                || contract.basis_sufficiency == ValidatorBasisSufficiency::Low
        );
        ensure!(
            issue_severities(&contract).contains("observation"),
            "weird-but-valid cases should surface observation-level concern, not hard rejection"
        );
        Ok(())
    }

    #[test]
    fn l3_t_validation_golden_verdicts_005_thin_appendix_evidence_yields_low_basis_without_guessing_domain_semantics()
    -> Result<()> {
        let issues = vec![issue(
            "observation",
            "completeness",
            "Appendix is too thin to support a confident semantic judgment.",
            &["Daemon Evidence Appendix > artifact_refs"],
        )];
        let contract = parse_payload(&payload(
            ValidatorVerdict::Flagged,
            ValidatorBasisSufficiency::Low,
            false,
            5_500,
            &issues,
            "Insufficient appendix evidence for confident semantic validation.",
        ))?;

        ensure!(contract.basis_sufficiency == ValidatorBasisSufficiency::Low);
        ensure!(
            !issue_categories(&contract).contains("plausibility")
                && !issue_categories(&contract).contains("consistency"),
            "thin evidence should not force guessed domain-semantics issues"
        );
        ensure!(
            issue_severities(&contract) == BTreeSet::from(["observation"]),
            "thin evidence should degrade basis sufficiency rather than escalate severity"
        );
        Ok(())
    }

    #[test]
    fn l3_t_validation_golden_verdicts_006_cross_model_audit_records_divergence_without_overwriting_primary_judgment()
    -> Result<()> {
        let assertions = load_golden_assertions()?;
        let primary = parse_payload(&payload(
            ValidatorVerdict::Approved,
            ValidatorBasisSufficiency::High,
            true,
            9_700,
            &[],
            "Primary validator approves the clean-real manifest.",
        ))?;

        let audit_report = json!({
            "case_id": "clean-real-001",
            "primary_judgment": {
                "verdict": primary.verdict.as_str(),
                "basis_sufficiency": primary.basis_sufficiency.as_str(),
                "confidence_bps": primary.confidence_bps,
            },
            "cross_model_audit": {
                "comparison_model_family": assertions.cross_model_audit.comparison_model_family,
                "comparison_verdict": "flagged",
                "comparison_basis_sufficiency": "medium",
                "divergence_reason": "alternate family marked the appendix as borderline thin",
            }
        });

        ensure!(assertions.cross_model_audit.required);
        ensure!(
            audit_report["primary_judgment"]["verdict"] == "approved",
            "primary calibration judgment should remain authoritative"
        );
        ensure!(
            audit_report["cross_model_audit"]["comparison_model_family"]
                == assertions.cross_model_audit.comparison_model_family,
            "cross-model audit must record the alternate validator family explicitly"
        );
        ensure!(
            audit_report["cross_model_audit"]["comparison_verdict"]
                != audit_report["primary_judgment"]["verdict"],
            "cross-model divergence should be recorded as a comparison outcome, not overwrite the primary judgment"
        );
        Ok(())
    }

    #[test]
    fn l3_t_validation_golden_verdicts_007_calibration_reports_assert_numeric_targets_and_wilson_precision_gate()
    -> Result<()> {
        let assertions = load_golden_assertions()?;
        let mutated_bad_catch_rate_bps = rate_bps(45, 50);
        let clean_real_approval_bps = rate_bps(57, 60);
        let false_positive_rate_bps = rate_bps(2, 60);
        let wilson_precision_bps = wilson_lower_bound_bps(100, 100, 1.959_963_984_540_054);

        ensure!(
            mutated_bad_catch_rate_bps
                >= assertions
                    .calibration_targets
                    .mutated_bad_catch_rate_min_bps,
            "mutated-bad catch rate fell below the Layer 3 floor"
        );
        ensure!(
            clean_real_approval_bps >= assertions.calibration_targets.clean_real_approval_min_bps,
            "clean-real approval rate fell below the Layer 3 floor"
        );
        ensure!(
            false_positive_rate_bps < assertions.calibration_targets.false_positive_rate_max_bps,
            "false-positive rate exceeded the Layer 3 ceiling"
        );
        ensure!(
            assertions.calibration_targets.wilson_confidence_bps == 9_500,
            "expected 95%% Wilson confidence gate"
        );
        ensure!(
            wilson_precision_bps >= f64::from(assertions.calibration_targets.wilson_confidence_bps),
            "Wilson-bound precision gate not met"
        );
        Ok(())
    }

    #[test]
    fn validation_golden_verdicts_metadata_aligns_canary_and_golden_case_definitions() -> Result<()>
    {
        let canary_cases = load_canary_cases()?;
        let assertions = load_golden_assertions()?;
        let canary_case_ids = canary_cases
            .iter()
            .map(|case| case.case_id.as_str())
            .collect::<BTreeSet<_>>();
        let golden_case_ids = assertions
            .cases
            .iter()
            .map(|case| case.case_id.as_str())
            .collect::<BTreeSet<_>>();

        ensure!(
            canary_case_ids == golden_case_ids,
            "canary inputs and golden assertions must cover the same case ids"
        );
        ensure!(
            canary_cases.iter().all(|case| {
                case.manifest_path.starts_with("artifacts/corpus/")
                    && !case.expected_verdict.is_empty()
                    && !case.expected_basis_sufficiency.is_empty()
                    && !case.provenance.is_empty()
                    && !case.notes.is_empty()
            }),
            "every canary case must preserve manifest path, expected verdict/basis, provenance, and notes"
        );
        ensure!(
            assertions
                .cases
                .iter()
                .all(|case| !case.expected_verdict.is_empty()
                    && !case.expected_basis_sufficiency.is_empty()
                    && !case.provenance.is_empty()),
            "every golden assertion must preserve expected verdict/basis and provenance"
        );
        ensure!(
            assertions
                .cases
                .iter()
                .filter(|case| case.expected_verdict != "approved")
                .all(|case| !case.required_issue_categories.is_empty()),
            "non-clean golden assertions should retain explicit issue-category expectations"
        );
        Ok(())
    }
}
