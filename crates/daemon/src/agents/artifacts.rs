#![forbid(unsafe_code)]

use std::fmt::Write as _;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail, ensure};
use chrono::{DateTime, Utc};
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

const RUN_MANIFEST_FILE: &str = "run-manifest.md";
const MAX_RUN_MANIFEST_BYTES: usize = 50 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OutcomeClass {
    Completed,
    ContinuationRequired,
    DegradedCompleted,
    BlockedHumanAction,
    SuspectOutput,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RunManifestFrontmatter {
    pub manifest_version: u32,
    pub skill_id: String,
    pub skill_version: String,
    pub job_id: String,
    pub session_id: String,
    pub execution_mode: String,
    pub started_at: String,
    pub completed_at: String,
    pub outcome_class: OutcomeClass,
    pub items_processed: u32,
    pub items_skipped: u32,
    pub items_failed: u32,
    pub warnings_count: u32,
    pub agent_claimed_pages_visited: u32,
    pub agent_claimed_source_url: String,
    pub agent_claimed_fingerprint_state: String,
    pub agent_claimed_locale_state: String,
    pub agent_claimed_currency_state: String,
    pub agent_narrative_missing: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RunNarrative {
    pub summary: String,
    pub steps_taken: String,
    pub decisions_made: String,
    pub warnings: String,
    pub evidence: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DaemonEvidenceAppendix {
    pub observed_url_chain: Vec<String>,
    pub observed_page_kind_sequence: Vec<String>,
    pub daemon_observed_fingerprint_state: String,
    pub daemon_observed_locale: String,
    pub daemon_observed_currency: String,
    pub auth_challenge_events: Vec<String>,
    pub locator_fallback_depth: u32,
    pub basis_sufficiency: String,
    pub page_fingerprint_hash: Option<String>,
    pub auth_challenge_detected: bool,
    pub extraction_fill_rate_bps: Option<u32>,
    pub checkpoint_count: u32,
    pub byte_count_compressed: Option<u64>,
    pub validator_rubric_hash: Option<String>,
    pub drift_delta_json: Value,
    pub result_item_count: u32,
    pub oqg_anomaly_summary: Vec<String>,
    pub representative_item_sample: Value,
    pub artifact_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClaimObservationDisagreement {
    pub field: String,
    pub agent_claimed: String,
    pub daemon_observed: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunManifestDraft {
    pub frontmatter: RunManifestFrontmatter,
    pub narrative: Option<RunNarrative>,
    pub appendix: DaemonEvidenceAppendix,
    pub disagreements: Vec<ClaimObservationDisagreement>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredManifest {
    pub path: PathBuf,
    pub sha256: String,
    pub size_bytes: usize,
    pub warning_review_required: bool,
}

#[derive(Debug, Clone)]
pub struct ArtifactStore {
    root: PathBuf,
}

impl ArtifactStore {
    pub fn new(root: impl AsRef<Path>) -> Result<Self> {
        let root = root.as_ref().to_path_buf();
        fs::create_dir_all(&root)
            .with_context(|| format!("creating artifact root {}", root.display()))?;
        Ok(Self { root })
    }

    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn session_dir(&self, session_id: &str, started_at: DateTime<Utc>) -> Result<PathBuf> {
        ensure!(!session_id.is_empty(), "session_id must not be empty");
        let session_dir = self
            .root
            .join("sessions")
            .join(started_at.format("%Y").to_string())
            .join(started_at.format("%m").to_string())
            .join(session_id);
        fs::create_dir_all(&session_dir)
            .with_context(|| format!("creating session artifact dir {}", session_dir.display()))?;
        Ok(session_dir)
    }

    pub fn run_manifest_path(
        &self,
        session_id: &str,
        started_at: DateTime<Utc>,
    ) -> Result<PathBuf> {
        Ok(self
            .session_dir(session_id, started_at)?
            .join(RUN_MANIFEST_FILE))
    }

    pub fn write_run_manifest(
        &self,
        draft: &RunManifestDraft,
        started_at: DateTime<Utc>,
    ) -> Result<StoredManifest> {
        validate_frontmatter(&draft.frontmatter)?;
        validate_appendix(&draft.appendix)?;
        let narrative = normalized_narrative(
            draft.narrative.as_ref(),
            draft.frontmatter.agent_narrative_missing,
        )?;
        validate_narrative(&narrative)?;

        let markdown = render_run_manifest(
            &draft.frontmatter,
            &narrative,
            &draft.appendix,
            &draft.disagreements,
        )?;
        ensure!(
            markdown.len() <= MAX_RUN_MANIFEST_BYTES,
            "run manifest exceeds size budget"
        );

        let path = self.run_manifest_path(&draft.frontmatter.session_id, started_at)?;
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .with_context(|| format!("creating run manifest {}", path.display()))?;
        file.write_all(markdown.as_bytes())
            .with_context(|| format!("writing run manifest {}", path.display()))?;
        file.flush()
            .with_context(|| format!("flushing run manifest {}", path.display()))?;

        let sha256 = sha256_hex(markdown.as_bytes());
        Ok(StoredManifest {
            path,
            sha256,
            size_bytes: markdown.len(),
            warning_review_required: !draft.disagreements.is_empty(),
        })
    }
}

impl RunManifestDraft {
    #[must_use]
    pub fn fallback(
        mut frontmatter: RunManifestFrontmatter,
        appendix: DaemonEvidenceAppendix,
        reason: &str,
    ) -> Self {
        frontmatter.agent_narrative_missing = true;
        Self {
            frontmatter,
            narrative: Some(RunNarrative {
                summary: format!(
                    "Daemon synthesized this fallback manifest because agent narrative was unavailable: {reason}"
                ),
                steps_taken: "Daemon persisted the machine-observed execution evidence only."
                    .to_owned(),
                decisions_made: "Narrative sections were synthesized to preserve reviewability."
                    .to_owned(),
                warnings: "Agent-authored narrative was unavailable for this run.".to_owned(),
                evidence: "Use the daemon evidence appendix and artifact refs for review."
                    .to_owned(),
            }),
            appendix,
            disagreements: Vec::new(),
        }
    }
}

pub fn validate_result_payload(schema: &Value, payload: &Value) -> Result<()> {
    let payload_object = payload
        .as_object()
        .ok_or_else(|| anyhow!("result payload must be a JSON object"))?;

    if let Some(required_fields) = schema.get("required").and_then(Value::as_array) {
        for field in required_fields {
            let field_name = field
                .as_str()
                .ok_or_else(|| anyhow!("required field names must be strings"))?;
            ensure!(
                payload_object.contains_key(field_name),
                "result payload missing required field `{field_name}`"
            );
        }
    }

    if let Some(properties) = schema.get("properties").and_then(Value::as_object) {
        for (name, property_schema) in properties {
            let Some(value) = payload_object.get(name) else {
                continue;
            };
            if let Some(expected_type) = property_schema.get("type").and_then(Value::as_str) {
                ensure!(
                    value_matches_type(value, expected_type),
                    "result payload field `{name}` violated type `{expected_type}`"
                );
            }
        }
    }

    Ok(())
}

pub fn persist_run_manifest_metadata(
    conn: &Connection,
    session_id: &str,
    stored: &StoredManifest,
) -> Result<()> {
    let updated = conn
        .execute(
            "\
            UPDATE agent_session_logs
            SET run_manifest_path = ?1,
                run_manifest_hash = ?2
            WHERE id = ?3
            ",
            params![stored.path.to_string_lossy(), stored.sha256, session_id,],
        )
        .context("persisting run manifest metadata")?;
    ensure!(
        updated == 1,
        "agent_session_logs row missing for {session_id}"
    );
    Ok(())
}

pub fn read_contiguous_jsonl_prefix(path: &Path) -> Result<Vec<Value>> {
    let bytes = fs::read(path).with_context(|| format!("reading index {}", path.display()))?;
    let mut values = Vec::new();

    for chunk in bytes.split_inclusive(|byte| *byte == b'\n') {
        if !chunk.ends_with(b"\n") {
            break;
        }

        let line = trim_line(chunk);
        if line.is_empty() {
            continue;
        }

        let Ok(value) = serde_json::from_slice::<Value>(line) else {
            break;
        };
        values.push(value);
    }

    Ok(values)
}

fn validate_frontmatter(frontmatter: &RunManifestFrontmatter) -> Result<()> {
    ensure!(
        frontmatter.manifest_version > 0,
        "manifest_version must be positive"
    );
    ensure!(
        !frontmatter.skill_id.is_empty(),
        "skill_id must not be empty"
    );
    ensure!(
        !frontmatter.skill_version.is_empty(),
        "skill_version must not be empty"
    );
    ensure!(!frontmatter.job_id.is_empty(), "job_id must not be empty");
    ensure!(
        !frontmatter.session_id.is_empty(),
        "session_id must not be empty"
    );
    ensure!(
        !frontmatter.execution_mode.is_empty(),
        "execution_mode must not be empty"
    );
    ensure!(
        !frontmatter.started_at.is_empty(),
        "started_at must not be empty"
    );
    ensure!(
        !frontmatter.completed_at.is_empty(),
        "completed_at must not be empty"
    );
    ensure!(
        frontmatter.agent_claimed_source_url.starts_with("http"),
        "agent_claimed_source_url must look like a URL"
    );
    ensure!(
        !frontmatter.agent_claimed_fingerprint_state.is_empty(),
        "agent_claimed_fingerprint_state must not be empty"
    );
    ensure!(
        !frontmatter.agent_claimed_locale_state.is_empty(),
        "agent_claimed_locale_state must not be empty"
    );
    ensure!(
        !frontmatter.agent_claimed_currency_state.is_empty(),
        "agent_claimed_currency_state must not be empty"
    );
    Ok(())
}

fn validate_narrative(narrative: &RunNarrative) -> Result<()> {
    ensure!(
        !narrative.summary.trim().is_empty(),
        "Summary section must not be empty"
    );
    ensure!(
        !narrative.steps_taken.trim().is_empty(),
        "Steps Taken section must not be empty"
    );
    ensure!(
        !narrative.decisions_made.trim().is_empty(),
        "Decisions Made section must not be empty"
    );
    ensure!(
        !narrative.warnings.trim().is_empty(),
        "Warnings section must not be empty"
    );
    ensure!(
        !narrative.evidence.trim().is_empty(),
        "Evidence section must not be empty"
    );
    Ok(())
}

fn validate_appendix(appendix: &DaemonEvidenceAppendix) -> Result<()> {
    ensure!(
        !appendix.daemon_observed_fingerprint_state.is_empty(),
        "daemon_observed_fingerprint_state must not be empty"
    );
    ensure!(
        !appendix.daemon_observed_locale.is_empty(),
        "daemon_observed_locale must not be empty"
    );
    ensure!(
        !appendix.daemon_observed_currency.is_empty(),
        "daemon_observed_currency must not be empty"
    );
    ensure!(
        !appendix.basis_sufficiency.trim().is_empty(),
        "basis_sufficiency must not be empty"
    );
    Ok(())
}

fn normalized_narrative(
    narrative: Option<&RunNarrative>,
    agent_narrative_missing: bool,
) -> Result<RunNarrative> {
    match narrative {
        Some(narrative) => Ok(narrative.clone()),
        None if agent_narrative_missing => Ok(RunNarrative {
            summary:
                "Daemon synthesized this fallback manifest because agent narrative was unavailable."
                    .to_owned(),
            steps_taken: "Daemon retained the durable artifact references and observed evidence."
                .to_owned(),
            decisions_made: "Narrative sections were synthesized to keep the manifest reviewable."
                .to_owned(),
            warnings: "Agent narrative missing; use appendix evidence for operator review."
                .to_owned(),
            evidence: "Fallback manifest contains only machine-authored evidence anchors."
                .to_owned(),
        }),
        None => bail!("agent narrative missing without fallback flag"),
    }
}

fn render_run_manifest(
    frontmatter: &RunManifestFrontmatter,
    narrative: &RunNarrative,
    appendix: &DaemonEvidenceAppendix,
    disagreements: &[ClaimObservationDisagreement],
) -> Result<String> {
    let frontmatter_yaml =
        serde_yaml::to_string(frontmatter).context("serializing run manifest frontmatter")?;
    let appendix_yaml =
        serde_yaml::to_string(appendix).context("serializing daemon evidence appendix")?;

    let disagreement_block = if disagreements.is_empty() {
        "None.\n".to_owned()
    } else {
        let mut rendered = String::new();
        for entry in disagreements {
            writeln!(
                rendered,
                "- {}: agent=`{}` daemon=`{}`",
                entry.field, entry.agent_claimed, entry.daemon_observed
            )
            .context("rendering disagreement summary")?;
        }
        rendered
    };

    Ok(format!(
        "---\n{frontmatter_yaml}---\n\
         ## Summary\n{summary}\n\n\
         ## Steps Taken\n{steps_taken}\n\n\
         ## Decisions Made\n{decisions_made}\n\n\
         ## Warnings\n{warnings}\n\n\
         ## Evidence\n{evidence}\n\n\
         ## Daemon Evidence Appendix\n```yaml\n{appendix_yaml}```\n\n\
         ## Claim/Observation Disagreements\n{disagreement_block}",
        summary = narrative.summary.trim(),
        steps_taken = narrative.steps_taken.trim(),
        decisions_made = narrative.decisions_made.trim(),
        warnings = narrative.warnings.trim(),
        evidence = narrative.evidence.trim(),
    ))
}

fn trim_line(bytes: &[u8]) -> &[u8] {
    let mut start = 0;
    let mut end = bytes.len();

    while start < end && bytes[start].is_ascii_whitespace() {
        start += 1;
    }
    while end > start && bytes[end - 1].is_ascii_whitespace() {
        end -= 1;
    }

    &bytes[start..end]
}

fn sha256_hex(bytes: &[u8]) -> String {
    let hash = Sha256::digest(bytes);
    format!("{hash:x}")
}

fn value_matches_type(value: &Value, expected_type: &str) -> bool {
    match expected_type {
        "string" => value.is_string(),
        "integer" => value.as_i64().is_some() || value.as_u64().is_some(),
        "number" => value.is_number(),
        "boolean" => value.is_boolean(),
        "array" => value.is_array(),
        "object" => value.is_object(),
        "null" => value.is_null(),
        _ => false,
    }
}
