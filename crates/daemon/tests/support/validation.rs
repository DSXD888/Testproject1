#![forbid(unsafe_code)]
#![allow(clippy::duplicate_mod)]
#![allow(
    dead_code,
    unused_imports,
    clippy::missing_const_for_fn,
    clippy::unused_self
)]

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use anyhow::{Result, ensure};
use rusqlite::{Connection, params};
use serde_yaml::Value as YamlValue;
use tempfile::TempDir;

use super::db::StateDb;

#[path = "../../src/agents/artifacts.rs"]
mod artifacts_impl;
#[path = "../../src/validation/policy.rs"]
mod policy_impl;

pub use artifacts_impl::{DaemonEvidenceAppendix, OutcomeClass, RunManifestFrontmatter};
pub use policy_impl::{
    BasisSufficiency, SourceHealthState, ValidationDecision, ValidationPolicyConfig,
    ValidationPolicyContext, ValidationPolicyMode, ValidationTrigger, evaluate_validation_policy,
};

pub struct ValidationPolicyFixture {
    pub config: ValidationPolicyConfig,
    pub context: ValidationPolicyContext,
}

impl ValidationPolicyFixture {
    #[must_use]
    pub fn sample(mode: ValidationPolicyMode) -> Self {
        Self {
            config: ValidationPolicyConfig {
                worker_skill_id: "procurement-ingestion/fts".to_owned(),
                source_id: "fts".to_owned(),
                validation_policy: mode,
                validation_sample_rate: 2_500,
                validation_triggers: BTreeSet::from([
                    ValidationTrigger::FillRateDrop,
                    ValidationTrigger::FirstRunAfterVersionChange,
                    ValidationTrigger::SuspectOutputInLastFive,
                    ValidationTrigger::OperatorRequested,
                    ValidationTrigger::SourceHealthAmberOrRed,
                    ValidationTrigger::PageFingerprintUnknown,
                    ValidationTrigger::ContinuationChainLengthGtOne,
                    ValidationTrigger::HighValueOpportunity,
                ]),
                worker_behavior_version: "worker.v1".to_owned(),
                validator_behavior_version: "validator.v1".to_owned(),
                validator_overlay_compatible_versions: BTreeSet::from(["worker.v1".to_owned()]),
            },
            context: ValidationPolicyContext::default(),
        }
    }

    #[must_use]
    pub fn with_operator_requested(mut self) -> Self {
        self.context.operator_requested = true;
        self
    }

    #[must_use]
    pub fn with_source_health(mut self, state: SourceHealthState) -> Self {
        self.context.source_health = state;
        self
    }

    #[must_use]
    pub fn evaluate(&self) -> ValidationDecision {
        evaluate_validation_policy(&self.config, &self.context)
    }
}

pub struct ValidatorHarness {
    _temp: TempDir,
    artifact_root: PathBuf,
    state_db: StateDb,
}

impl ValidatorHarness {
    pub fn new() -> Result<Self> {
        let temp = tempfile::tempdir()?;
        let db_path = temp.path().join("state.db");
        let artifact_root = temp.path().join("artifacts");
        std::fs::create_dir_all(&artifact_root)?;
        Ok(Self {
            _temp: temp,
            artifact_root,
            state_db: StateDb::open(&db_path)?,
        })
    }

    #[must_use]
    pub fn connection(&self) -> &Connection {
        self.state_db.connection()
    }

    pub fn seed_job(&self, job_id: &str) -> Result<()> {
        self.connection().execute(
            "\
            INSERT INTO workflow_jobs (id, kind, status, input_payload)
            VALUES (?1, 'stage-job', 'pending', ?2)
            ",
            params![job_id, serde_json::json!({"job_id": job_id}).to_string()],
        )?;
        Ok(())
    }

    pub fn seed_process(&self, process_id: &str) -> Result<()> {
        self.connection().execute(
            "\
            INSERT INTO procurement_processes (
              id, current_stage, title, latest_published_at, latest_updated_at, owner_id
            ) VALUES (?1, 'manifest', 'Process title', '2026-03-30T00:00:00Z',
                      '2026-03-30T00:00:00Z', 'local-owner')
            ",
            params![process_id],
        )?;
        Ok(())
    }

    pub fn set_validation_required(&self, job_id: &str, required: bool) -> Result<()> {
        self.connection().execute(
            "UPDATE workflow_jobs SET validation_required = ?2 WHERE id = ?1",
            params![job_id, i64::from(required)],
        )?;
        Ok(())
    }

    pub fn assert_validation_required(&self, job_id: &str, expected: bool) -> Result<()> {
        let required: i64 = self.connection().query_row(
            "SELECT validation_required FROM workflow_jobs WHERE id = ?1",
            [job_id],
            |row| row.get(0),
        )?;
        ensure!(required == i64::from(expected));
        Ok(())
    }

    pub fn link_validation_verdict(
        &self,
        worker_job_id: &str,
        validator_job_id: &str,
        verdict_id: &str,
    ) -> Result<()> {
        let validator_exists: i64 = self.connection().query_row(
            "SELECT COUNT(*) FROM workflow_jobs WHERE id = ?1",
            [validator_job_id],
            |row| row.get(0),
        )?;
        if validator_exists == 0 {
            self.seed_job(validator_job_id)?;
        }

        self.connection().execute(
            "\
            INSERT INTO validation_verdicts (
              id, worker_job_id, worker_session_id, validator_job_id, validator_skill_path,
              verdict, summary, run_manifest_hash, verdict_rendered_at
            ) VALUES (?1, ?2, 'worker-session', ?3, 'skills/validation/base/SKILL.md',
                      'approved', 'linked', 'manifest-hash', '2026-03-30T16:00:00Z')
            ",
            params![verdict_id, worker_job_id, validator_job_id],
        )?;
        self.connection().execute(
            "UPDATE workflow_jobs SET validation_required = 1, validation_verdict_id = ?2 WHERE id = ?1",
            params![worker_job_id, verdict_id],
        )?;
        Ok(())
    }

    pub fn assert_validation_linkage(&self, worker_job_id: &str, verdict_id: &str) -> Result<()> {
        let (validation_required, validation_verdict_id): (i64, Option<String>) =
            self.connection().query_row(
                "SELECT validation_required, validation_verdict_id FROM workflow_jobs WHERE id = ?1",
                [worker_job_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )?;
        let linked_rows: i64 = self.connection().query_row(
            "SELECT COUNT(*) FROM validation_verdicts WHERE id = ?1 AND worker_job_id = ?2",
            params![verdict_id, worker_job_id],
            |row| row.get(0),
        )?;
        ensure!(validation_required == 1);
        ensure!(validation_verdict_id.as_deref() == Some(verdict_id));
        ensure!(linked_rows == 1);
        Ok(())
    }

    pub fn scratch_root(&self, run_id: &str) -> Result<PathBuf> {
        let scratch_root = self.artifact_root.join("scratch").join(run_id);
        std::fs::create_dir_all(&scratch_root)?;
        Ok(scratch_root)
    }

    pub fn assert_scratch_root(&self, run_id: &str, path: &Path) -> Result<()> {
        ensure!(path.ends_with(Path::new("scratch").join(run_id)));
        Ok(())
    }

    pub fn set_process_validation_state(&self, process_id: &str, state: &str) -> Result<()> {
        self.connection().execute(
            "\
            INSERT INTO pipeline_assessments (
              process_id, pipeline_state, decision_basis_hash, validation_state, owner_id
            ) VALUES (?1, 'review', 'basis-a', ?2, 'local-owner')
            ON CONFLICT(process_id) DO UPDATE SET validation_state = excluded.validation_state
            ",
            params![process_id, state],
        )?;
        Ok(())
    }

    pub fn assert_process_validation_state(&self, process_id: &str, expected: &str) -> Result<()> {
        let validation_state: String = self.connection().query_row(
            "SELECT validation_state FROM pipeline_assessments WHERE process_id = ?1",
            [process_id],
            |row| row.get(0),
        )?;
        ensure!(validation_state == expected);
        Ok(())
    }

    pub fn assert_frontmatter_key_parity(frontmatter: &RunManifestFrontmatter) -> Result<()> {
        let keys = yaml_keys(&serde_yaml::to_value(frontmatter)?)?;
        let expected = vec![
            "manifest_version",
            "skill_id",
            "skill_version",
            "job_id",
            "session_id",
            "execution_mode",
            "started_at",
            "completed_at",
            "outcome_class",
            "items_processed",
            "items_skipped",
            "items_failed",
            "warnings_count",
            "agent_claimed_pages_visited",
            "agent_claimed_source_url",
            "agent_claimed_fingerprint_state",
            "agent_claimed_locale_state",
            "agent_claimed_currency_state",
            "agent_narrative_missing",
        ];
        ensure!(keys == expected);
        Ok(())
    }

    pub fn assert_appendix_key_parity(appendix: &DaemonEvidenceAppendix) -> Result<()> {
        let keys = yaml_keys(&serde_yaml::to_value(appendix)?)?;
        let expected = vec![
            "observed_url_chain",
            "observed_page_kind_sequence",
            "daemon_observed_fingerprint_state",
            "daemon_observed_locale",
            "daemon_observed_currency",
            "auth_challenge_events",
            "locator_fallback_depth",
            "basis_sufficiency",
            "page_fingerprint_hash",
            "auth_challenge_detected",
            "extraction_fill_rate_bps",
            "checkpoint_count",
            "byte_count_compressed",
            "validator_rubric_hash",
            "drift_delta_json",
            "result_item_count",
            "oqg_anomaly_summary",
            "representative_item_sample",
            "artifact_refs",
            "bid_scope_summary",
            "side_effect_progression",
            "readiness_evidence",
            "validator_verdict",
            "override_metadata",
            "final_bid_state",
        ];
        ensure!(keys == expected);
        Ok(())
    }
}

pub struct SyntheticCanaryFixture;

impl SyntheticCanaryFixture {
    #[must_use]
    pub const fn known_good_case_id() -> &'static str {
        "known-good"
    }

    #[must_use]
    pub const fn known_bad_case_id() -> &'static str {
        "known-bad"
    }

    #[must_use]
    pub const fn insufficient_basis_case_id() -> &'static str {
        "known-insufficient-basis"
    }

    #[must_use]
    pub const fn corpus_tag() -> &'static str {
        "clean_real"
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetroBacklogSpec {
    pub observed_at: String,
    pub high_value_opportunity: bool,
    pub operator_reviewed: bool,
    pub enforcement_ready: bool,
}

pub struct RetroBacklogFixture;

impl RetroBacklogFixture {
    #[must_use]
    pub fn metadata(
        observed_at: &str,
        high_value_opportunity: bool,
        operator_reviewed: bool,
        enforcement_ready: bool,
    ) -> RetroBacklogSpec {
        RetroBacklogSpec {
            observed_at: observed_at.to_owned(),
            high_value_opportunity,
            operator_reviewed,
            enforcement_ready,
        }
    }

    pub fn assert_priority_order(actual: &[String], expected: &[&str]) -> Result<()> {
        ensure!(
            actual
                == expected
                    .iter()
                    .map(|value| (*value).to_owned())
                    .collect::<Vec<_>>()
        );
        Ok(())
    }
}

fn yaml_keys(value: &YamlValue) -> Result<Vec<String>> {
    Ok(value
        .as_mapping()
        .ok_or_else(|| anyhow::anyhow!("expected YAML mapping"))?
        .keys()
        .filter_map(YamlValue::as_str)
        .map(str::to_owned)
        .collect())
}
