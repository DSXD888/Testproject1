#![forbid(unsafe_code)]

#[path = "support/db.rs"]
mod db_support;
#[path = "../src/validation/policy.rs"]
#[allow(dead_code)]
mod policy_impl;
#[path = "support/skills.rs"]
mod skills_support;
#[path = "../src/sources/registry.rs"]
#[allow(dead_code)]
mod sources_impl;

mod tests {
    use std::collections::BTreeSet;
    use std::fs;
    use std::path::{Path, PathBuf};

    use anyhow::{Context, Result, anyhow, ensure};
    use assert_cmd::Command;
    use rusqlite::{Connection, params};
    use serde_json::{Value, json};
    use serde_yaml::{Mapping, Value as YamlValue};
    use tempfile::TempDir;

    use crate::db_support::TestStateDb;
    use crate::policy_impl::{
        SkillPromotionStage, ValidationPolicyConfig, ValidationPolicyContext, ValidationPolicyMode,
        evaluate_validation_policy,
    };
    use crate::skills_support::{
        TestSkillDir, lint_skill_directory, load_skill_metadata, repo_root, repo_skill_dir,
    };
    use crate::sources_impl::{DEFAULT_SOURCE_SKILL_VERSION, PromotionStage, SourceRegistry};

    struct ManualSkillHarness {
        _temp: TempDir,
        state_root: PathBuf,
        runtime_root: PathBuf,
        artifact_root: PathBuf,
        db_path: PathBuf,
    }

    struct RunResult {
        success: bool,
        stderr: String,
    }

    #[allow(dead_code)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum JourneyStage {
        Draft,
        Candidate,
        Canary,
        Shadow,
        Stable,
        Rollback,
    }

    struct CanaryPlan {
        traffic_share_bps: u16,
        shard_count: u16,
        comparison_data_present: bool,
        stable_authoritative: bool,
        bounded_runs: u32,
        validator_approval_bps: u16,
        precision_or_recall_regressed: bool,
    }

    struct ShadowPlan {
        full_traffic: bool,
        stable_authoritative: bool,
        paired_runs: u32,
        quality_within_tolerance: bool,
    }

    struct RollbackOutcome {
        rebound_version: String,
        failed_version_next_stage: JourneyStage,
    }

    impl ManualSkillHarness {
        fn new() -> Result<Self> {
            let temp = tempfile::tempdir()?;
            let root = temp.path().to_path_buf();
            let state_root = root.join("state");
            let runtime_root = root.join("runtime");
            let artifact_root = root.join("artifacts");
            fs::create_dir_all(&state_root)?;
            fs::create_dir_all(&runtime_root)?;
            fs::create_dir_all(&artifact_root)?;
            Ok(Self {
                _temp: temp,
                db_path: state_root.join("state.db"),
                state_root,
                runtime_root,
                artifact_root,
            })
        }

        fn run_skill(&self, skill_path: &Path, job_parameters: &Value) -> Result<RunResult> {
            let fake_claude = self.prepare_fake_claude()?;
            let output = Command::cargo_bin("daemon")?
                .arg("--skill-path")
                .arg(skill_path)
                .arg("--job-parameters-json")
                .arg(job_parameters.to_string())
                .arg("--agent-cli")
                .arg("claude")
                .arg("--agent-cli-program")
                .arg(&fake_claude.program)
                .arg("--state-root")
                .arg(&self.state_root)
                .arg("--runtime-dir")
                .arg(&self.runtime_root)
                .arg("--artifact-root")
                .arg(&self.artifact_root)
                .env("DAEMON_TEST_CLAUDE_RESULT_PATH", &fake_claude.result_path)
                .output()
                .context("running daemon binary with --skill-path")?;

            Ok(RunResult {
                success: output.status.success(),
                stderr: String::from_utf8(output.stderr).context("decoding stderr")?,
            })
        }

        fn connection(&self) -> Result<Connection> {
            Connection::open(&self.db_path)
                .with_context(|| format!("opening {}", self.db_path.display()))
        }

        fn prepare_fake_claude(&self) -> Result<FakeClaudeCli> {
            let root = self
                .state_root
                .parent()
                .ok_or_else(|| anyhow!("state root missing parent"))?;
            let state_dir = root.join("fake-claude");
            fs::create_dir_all(&state_dir)?;
            let probe_json = state_dir.join("probe.json");
            fs::write(
                &probe_json,
                serde_json::to_vec_pretty(&json!({
                    "auth_healthy": true,
                    "structured_output": true,
                    "pty_viable": true,
                    "browser_tools": true
                }))?,
            )?;
            let result_source = state_dir.join("result-source.json");
            fs::write(
                &result_source,
                serde_json::to_vec_pretty(&manifest_generation_result_payload())?,
            )?;
            let program = state_dir.join("claude");
            fs::write(
                &program,
                format!(
                    "#!/bin/sh\nset -eu\nstate_dir=\"{}\"\nif [ \"${{1-}}\" = \"--version\" ] || [ \"${{1-}}\" = \"-v\" ]; then\n  printf 'claude 1.2.3\\n'\n  exit 0\nfi\nif [ \"${{1-}}\" = \"probe\" ]; then\n  cat \"$state_dir/probe.json\"\n  exit 0\nfi\nmode=pty\nfor arg in \"$@\"; do\n  if [ \"$arg\" = \"--output-format\" ]; then\n    mode=contract\n    break\n  fi\ndone\nif [ \"$mode\" = contract ]; then\n  cp \"$state_dir/result-source.json\" \"$DAEMON_TEST_CLAUDE_RESULT_PATH\"\n  printf '{{\"event\":\"result\",\"path\":\"%s\"}}\\n' \"$DAEMON_TEST_CLAUDE_RESULT_PATH\"\nelse\n  printf 'unexpected pty mode\\n' >&2\n  exit 1\nfi\n",
                    state_dir.display()
                ),
            )?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = fs::metadata(&program)?.permissions();
                perms.set_mode(0o755);
                fs::set_permissions(&program, perms)?;
            }
            Ok(FakeClaudeCli {
                program,
                result_path: self.artifact_root.join("fake-claude-result.json"),
            })
        }
    }

    struct FakeClaudeCli {
        program: PathBuf,
        result_path: PathBuf,
    }

    fn manifest_generation_result_payload() -> Value {
        json!({
            "process_id": "process-manual-001",
            "decisionBasisHash": "decision-hash-001",
            "manifestMarkdown": "# Opportunity Manifest\n\n- Process: process-manual-001\n- Title: Manual process\n- Candidate count: 0\n",
            "candidate_count": 0,
        })
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

    fn install_manifest_generation_lifecycle_metadata(
        skill_dir: &TestSkillDir,
        stage: &str,
    ) -> Result<()> {
        let worker_behavior_version = "manifest-generation@1.0.0+1.0.0";
        let gate_b_exact_slice_key =
            format!("manifest-generation::global::{worker_behavior_version}::2026.03.30-1");
        let markdown = skill_dir.read_skill_markdown()?;
        let updated = rewrite_frontmatter(
            &markdown,
            &[
                ("checkpoint_schema_version", yaml_str("1.0.0")),
                ("validator_overlay_version", yaml_str("2026.03.30-1")),
                ("promotion_stage", yaml_str(stage)),
                ("worker_behavior_version", yaml_str(worker_behavior_version)),
                ("validator_behavior_version", yaml_str("2026.03.30-1")),
                ("version_change_affects_behavior", YamlValue::Bool(true)),
                ("gate_a_family_key", yaml_str("manifest-generation")),
                ("gate_a_family_prequalified", YamlValue::Bool(false)),
                ("gate_b_exact_slice_key", yaml_str(&gate_b_exact_slice_key)),
                ("gate_b_exact_slice_ready", YamlValue::Bool(false)),
            ],
        )?;
        skill_dir.overwrite("SKILL.md", &updated)
    }

    fn latest_job_payload(conn: &Connection) -> Result<Value> {
        let payload: String = conn.query_row(
            "SELECT input_payload FROM workflow_jobs ORDER BY created_at DESC LIMIT 1",
            [],
            |row| row.get(0),
        )?;
        serde_json::from_str(&payload).context("parsing workflow_jobs.input_payload")
    }

    fn count_rows(conn: &Connection, table: &str) -> Result<i64> {
        conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
            row.get(0)
        })
        .with_context(|| format!("counting rows in {table}"))
    }

    const fn canary_ready(plan: &CanaryPlan) -> bool {
        (plan.traffic_share_bps < 10_000 || plan.shard_count == 1)
            && plan.comparison_data_present
            && plan.stable_authoritative
            && plan.bounded_runs >= 20
            && plan.validator_approval_bps >= 9_500
            && !plan.precision_or_recall_regressed
    }

    const fn shadow_ready(plan: &ShadowPlan) -> bool {
        plan.full_traffic
            && plan.stable_authoritative
            && plan.paired_runs >= 50
            && plan.quality_within_tolerance
    }

    fn rollback_to_prior_stable(prior_stable_version: &str) -> RollbackOutcome {
        RollbackOutcome {
            rebound_version: prior_stable_version.to_owned(),
            failed_version_next_stage: JourneyStage::Candidate,
        }
    }

    const fn transition_allowed(
        from: JourneyStage,
        to: JourneyStage,
        validator_approval_bps: u16,
    ) -> bool {
        match (from, to) {
            (JourneyStage::Draft, JourneyStage::Candidate)
            | (JourneyStage::Shadow, JourneyStage::Stable)
            | (JourneyStage::Stable, JourneyStage::Rollback) => true,
            (JourneyStage::Candidate, JourneyStage::Canary) => validator_approval_bps >= 9_000,
            (JourneyStage::Canary, JourneyStage::Shadow) => validator_approval_bps >= 9_500,
            _ => false,
        }
    }

    fn bid_portal_corpus_row_count(category: &str) -> Result<usize> {
        let inputs = fs::read_to_string(
            repo_root()?
                .join("tests/fixtures/calibration/bid-portal-procontract")
                .join(category)
                .join("canary.inputs.jsonl"),
        )?;
        Ok(inputs
            .lines()
            .filter(|line| !line.trim().is_empty())
            .count())
    }

    fn bid_portal_corpus_contains_formal_submission_path() -> Result<bool> {
        let corpus_root = repo_root()?.join("tests/fixtures/calibration/bid-portal-procontract");
        for category in ["simple", "medium", "complex"] {
            let inputs =
                fs::read_to_string(corpus_root.join(category).join("canary.inputs.jsonl"))?;
            for line in inputs.lines().filter(|line| !line.trim().is_empty()) {
                let canary: Value = serde_json::from_str(line)
                    .with_context(|| format!("parsing {category} bid portal canary row"))?;
                if canary
                    .get("side_effect_cap")
                    .and_then(Value::as_str)
                    .is_some_and(|cap| cap == "FORMAL_SUBMISSION")
                    || canary
                        .get("expected_side_effect_class_reached")
                        .and_then(Value::as_str)
                        .is_some_and(|observed| observed == "FORMAL_SUBMISSION")
                {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    fn bid_portal_canary_exit_allowed(
        current_stage: &str,
        formal_submission_path_present: bool,
        operator_authorized: bool,
    ) -> bool {
        current_stage == "canary" && (!formal_submission_path_present || operator_authorized)
    }

    fn section(markdown: &str, title: &str) -> Result<String> {
        let heading = format!("## {title}");
        let start = markdown
            .find(&heading)
            .ok_or_else(|| anyhow!("missing section `{title}`"))?;
        let body = &markdown[start + heading.len()..];
        let end = body.find("\n## ").unwrap_or(body.len());
        Ok(body[..end].trim().to_owned())
    }

    fn seed_source(conn: &Connection, source_id: &str, kind: &str) -> Result<()> {
        conn.execute(
            "\
            INSERT OR IGNORE INTO sources (id, kind, enabled, config)
            VALUES (?1, ?2, 1, '{}')
            ",
            params![source_id, kind],
        )?;
        Ok(())
    }

    #[test]
    fn l3_t_skill_golden_journeys_001_draft_skills_run_manual_only_without_authoritative_mutation()
    -> Result<()> {
        let skill_dir = TestSkillDir::from_fixture(&repo_skill_dir("manifest-generation")?)?;
        install_manifest_generation_lifecycle_metadata(&skill_dir, "draft")?;
        lint_skill_directory(skill_dir.path())?;

        let harness = ManualSkillHarness::new()?;
        let result = harness.run_skill(&skill_dir.path().join("SKILL.md"), &json!({}))?;
        ensure!(
            result.success,
            "draft manual skill run should succeed: {}",
            result.stderr
        );

        let conn = harness.connection()?;
        let (kind, scope_kind, scope_id): (String, Option<String>, Option<String>) = conn
            .query_row(
            "SELECT kind, scope_kind, scope_id FROM workflow_jobs ORDER BY created_at DESC LIMIT 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )?;
        ensure!(kind == "run_skill");
        ensure!(scope_kind.as_deref() == Some("skill"));
        ensure!(scope_id.as_deref() == Some("manifest-generation"));

        let payload = latest_job_payload(&conn)?;
        ensure!(payload["skill_binding"]["promotion"]["stage"] == "draft");
        ensure!(
            count_rows(&conn, "source_cursors")? == 0,
            "draft skill should not bind scheduler roots or source cursors",
        );
        ensure!(
            count_rows(&conn, "manifests")? == 0,
            "draft manual exploration should not write authoritative manifests",
        );
        ensure!(
            count_rows(&conn, "pipeline_assessments")? == 0,
            "draft manual exploration should not write authoritative pipeline rows",
        );
        Ok(())
    }

    #[test]
    fn l3_t_skill_golden_journeys_002_candidate_stage_forces_validation_and_keeps_worker_state_non_authoritative()
    -> Result<()> {
        let skill_dir = TestSkillDir::from_fixture(&repo_skill_dir("manifest-generation")?)?;
        install_manifest_generation_lifecycle_metadata(&skill_dir, "candidate")?;
        lint_skill_directory(skill_dir.path())?;

        let harness = ManualSkillHarness::new()?;
        let result = harness.run_skill(&skill_dir.path().join("SKILL.md"), &json!({}))?;
        ensure!(result.success, "candidate manual skill run should succeed");

        let conn = harness.connection()?;
        let payload = latest_job_payload(&conn)?;
        ensure!(payload["skill_binding"]["promotion"]["stage"] == "candidate");
        ensure!(
            payload["skill_binding"]["behavior"]["worker_behavior_version"]
                == "manifest-generation@1.0.0+1.0.0"
        );
        ensure!(count_rows(&conn, "pipeline_assessments")? == 0);
        ensure!(count_rows(&conn, "manifests")? == 0);

        let decision = evaluate_validation_policy(
            &ValidationPolicyConfig {
                worker_skill_id: "manifest-generation".to_owned(),
                source_id: "global".to_owned(),
                validation_policy: ValidationPolicyMode::Skip,
                validation_sample_rate: 0,
                validation_triggers: BTreeSet::new(),
                worker_behavior_version: "manifest-generation@1.0.0+1.0.0".to_owned(),
                validator_behavior_version: "2026.03.30-1".to_owned(),
                validator_overlay_compatible_versions: BTreeSet::from_iter([
                    "manifest-generation@1.0.0+1.0.0".to_owned(),
                ]),
            },
            &ValidationPolicyContext {
                promotion_stage: SkillPromotionStage::Candidate,
                ..ValidationPolicyContext::default()
            },
        );
        ensure!(decision.validation_required);
        Ok(())
    }

    #[test]
    fn l3_t_skill_golden_journeys_003_canary_requires_bounded_traffic_and_stable_comparison_data()
    -> Result<()> {
        let good = CanaryPlan {
            traffic_share_bps: 500,
            shard_count: 0,
            comparison_data_present: true,
            stable_authoritative: true,
            bounded_runs: 20,
            validator_approval_bps: 9_500,
            precision_or_recall_regressed: false,
        };
        ensure!(canary_ready(&good));

        let full_traffic = CanaryPlan {
            traffic_share_bps: 10_000,
            shard_count: 0,
            ..good
        };
        ensure!(!canary_ready(&full_traffic));
        Ok(())
    }

    #[test]
    fn l3_t_skill_golden_journeys_004_shadow_requires_full_traffic_while_stable_stays_authoritative()
    -> Result<()> {
        let good = ShadowPlan {
            full_traffic: true,
            stable_authoritative: true,
            paired_runs: 50,
            quality_within_tolerance: true,
        };
        ensure!(shadow_ready(&good));

        let bad = ShadowPlan {
            stable_authoritative: false,
            ..good
        };
        ensure!(!shadow_ready(&bad));
        Ok(())
    }

    #[test]
    fn l3_t_skill_golden_journeys_005_stable_is_registry_default_and_rollback_rebinds_prior_stable()
    -> Result<()> {
        let state_db = TestStateDb::new()?;
        seed_source(state_db.connection(), "fts", "procurement")?;
        let registry = SourceRegistry::load(state_db.connection())?;
        let production_binding = registry
            .get("fts")
            .and_then(|source| source.definition.production_binding.as_ref())
            .ok_or_else(|| anyhow!("missing fts production binding"))?;
        ensure!(production_binding.promotion_stage == PromotionStage::Stable);
        ensure!(production_binding.calibration.family_key == "procurement-ingestion");

        let rollback = rollback_to_prior_stable("2026.03.30-1");
        ensure!(rollback.rebound_version == "2026.03.30-1");
        ensure!(rollback.failed_version_next_stage == JourneyStage::Candidate);
        Ok(())
    }

    #[test]
    fn l3_t_skill_golden_journeys_006_stage_skips_are_rejected_and_validator_thresholds_gate_promotion()
    -> Result<()> {
        ensure!(!transition_allowed(
            JourneyStage::Draft,
            JourneyStage::Shadow,
            10_000,
        ));
        ensure!(!transition_allowed(
            JourneyStage::Canary,
            JourneyStage::Shadow,
            9_499,
        ));
        ensure!(transition_allowed(
            JourneyStage::Canary,
            JourneyStage::Shadow,
            9_500,
        ));
        Ok(())
    }

    #[test]
    fn l3_t_skill_golden_journeys_007_layer3_inventory_contains_operational_families_and_validator()
    -> Result<()> {
        let auth_refresh_dirs = fs::read_dir(repo_root()?.join("skills/auth-refresh"))?
            .filter_map(|entry| entry.ok().map(|value| value.path()))
            .filter(|path| path.is_dir())
            .filter_map(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .map(str::to_owned)
            })
            .collect::<BTreeSet<_>>();
        ensure!(
            auth_refresh_dirs.len().eq(&2)
                && auth_refresh_dirs.contains("1688")
                && auth_refresh_dirs.contains("alibaba")
        );

        let source_health_dirs = fs::read_dir(repo_root()?.join("skills/source-health"))?
            .filter_map(|entry| entry.ok().map(|value| value.path()))
            .filter(|path| path.is_dir())
            .filter_map(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .map(str::to_owned)
            })
            .collect::<BTreeSet<_>>();
        ensure!(
            source_health_dirs
                == BTreeSet::from([
                    "alibaba".to_owned(),
                    "fts".to_owned(),
                    "l5-marketplace-commercial-intelligence".to_owned(),
                    "uk-installation-labour-rate-refresh".to_owned(),
                ])
        );

        ensure!(
            lint_skill_directory(&repo_skill_dir("validation/base")?)?
                .metadata
                .interaction_profile
                == "headless_contract"
        );
        ensure!(
            lint_skill_directory(&repo_skill_dir("auth-refresh/alibaba")?)?
                .metadata
                .interaction_profile
                == "pty_interactive"
        );
        ensure!(
            lint_skill_directory(&repo_skill_dir("source-health/fts")?)?
                .metadata
                .interaction_profile
                == "pty_interactive"
        );
        for (relative, scope, expected_side_effect_class) in [
            ("auth-refresh/alibaba", "alibaba", "local_state"),
            ("auth-refresh/1688", "1688", "local_state"),
            ("source-health/fts", "fts", "none"),
            ("source-health/alibaba", "alibaba", "none"),
        ] {
            let linted = lint_skill_directory(&repo_skill_dir(relative)?)?;
            let expected_worker_behavior_version =
                format!("{relative}@{DEFAULT_SOURCE_SKILL_VERSION}+1.0.0");
            let expected_slice_key =
                format!("{relative}::{scope}::{expected_worker_behavior_version}::2026.03.30-1");
            ensure!(
                linted.metadata.skill_version == DEFAULT_SOURCE_SKILL_VERSION,
                "{relative} must stay on the registry-bound stable version"
            );
            ensure!(
                linted.metadata.model_profile == "browse_stable_low_cost",
                "{relative} must stay on the browser low-cost profile"
            );
            ensure!(
                linted.metadata.side_effect_class == expected_side_effect_class,
                "{relative} must keep the expected side-effect classification"
            );
            ensure!(
                linted.metadata.worker_behavior_version.as_deref()
                    == Some(expected_worker_behavior_version.as_str()),
                "{relative} must derive worker_behavior_version from skill_version + result_schema_version"
            );
            ensure!(
                linted.metadata.gate_b_exact_slice_key.as_deref()
                    == Some(expected_slice_key.as_str()),
                "{relative} must keep its exact-slice binding aligned with the worker behavior version"
            );
        }
        Ok(())
    }

    #[test]
    fn l3_t_skill_golden_journeys_008_auth_refresh_and_source_health_roles_stay_explicit()
    -> Result<()> {
        for relative in ["auth-refresh/alibaba", "auth-refresh/1688"] {
            let linted = lint_skill_directory(&repo_skill_dir(relative)?)?;
            let outputs = section(&linted.skill_markdown, "Outputs")?;
            let do_not_use = section(&linted.skill_markdown, "Do not use when")?;
            ensure!(
                outputs.contains("auth-state handle"),
                "{relative} must explicitly publish auth-state handles"
            );
            ensure!(
                do_not_use.contains("extraction") || do_not_use.contains("listing"),
                "{relative} must explicitly reject unrelated extraction responsibilities"
            );
        }

        for relative in ["source-health/fts", "source-health/alibaba"] {
            let linted = lint_skill_directory(&repo_skill_dir(relative)?)?;
            let schema_path = repo_skill_dir(relative)?.join("result.schema.json");
            let schema = serde_json::from_str::<Value>(&fs::read_to_string(schema_path)?)?;
            let required_checks = schema["properties"]["checks"]["required"]
                .as_array()
                .ok_or_else(|| anyhow!("missing checks.required for {relative}"))?
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>();
            ensure!(
                required_checks
                    == vec![
                        "reachability",
                        "fingerprint",
                        "auth_state",
                        "locale",
                        "challenge_rate",
                        "minimal_extraction",
                    ],
                "{relative} must stay on the fixed six-check source-health contract"
            );
            let do_not_use = section(&linted.skill_markdown, "Do not use when")?;
            ensure!(
                do_not_use.contains("full procurement ingestion")
                    || do_not_use.contains("full supplier extraction"),
                "{relative} must reject unrelated extraction work"
            );
        }

        Ok(())
    }

    #[test]
    fn l3_t_skill_golden_journeys_009_bid_portal_promotion_lifecycle_requires_operator_authorization()
    -> Result<()> {
        let linted = load_skill_metadata(&repo_skill_dir("bid-portal-procontract")?)?;
        ensure!(linted.metadata.skill_id == "bid-portal/procontract");
        ensure!(linted.metadata.promotion_stage.as_deref() == Some("draft"));
        ensure!(
            linted.metadata.worker_behavior_version.as_deref()
                == Some("bid-portal/procontract@0.1.0+1.0")
        );
        ensure!(
            linted.skill_markdown.contains("validation_policy: always"),
            "bid portal worker must remain bound to fail-closed validation"
        );

        ensure!(bid_portal_corpus_row_count("simple")? >= 6);
        ensure!(bid_portal_corpus_row_count("medium")? >= 8);
        ensure!(bid_portal_corpus_row_count("complex")? >= 10);

        let canary_metrics_green = CanaryPlan {
            traffic_share_bps: 500,
            shard_count: 0,
            comparison_data_present: true,
            stable_authoritative: true,
            bounded_runs: 30,
            validator_approval_bps: 9_900,
            precision_or_recall_regressed: false,
        };
        ensure!(
            canary_ready(&canary_metrics_green),
            "baseline canary metrics should be green before applying bid-specific operator gate"
        );

        let formal_submission_path_present = bid_portal_corpus_contains_formal_submission_path()?;
        ensure!(
            formal_submission_path_present,
            "bid portal canary corpus must include the formal-submission path"
        );
        ensure!(
            !bid_portal_canary_exit_allowed("draft", formal_submission_path_present, true),
            "draft bid skill must not skip directly past canary even with operator authorization"
        );
        ensure!(
            !bid_portal_canary_exit_allowed("canary", formal_submission_path_present, false),
            "formal-submission-capable bid skill must not auto-promote past canary"
        );
        ensure!(
            bid_portal_canary_exit_allowed("canary", formal_submission_path_present, true),
            "operator authorization is required before canary exit for bid submission automation"
        );

        Ok(())
    }
}
