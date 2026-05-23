#![forbid(unsafe_code)]

use std::path::Path;

use anyhow::Context;
use anyhow::Result;
use chrono::{DateTime, Utc};
#[cfg(test)]
use serde_json::Value;

use crate::agents::artifacts::{
    ArtifactStore, ClaimObservationDisagreement, DaemonEvidenceAppendix, RunManifestDraft,
    RunManifestFrontmatter, RunNarrative, StoredManifest,
};
#[cfg(not(test))]
use crate::bid_workflow::redaction::{redact_bid_skill_input, redact_sensitive_text};

pub struct ManifestWriteRequest<'a> {
    pub artifact_root: &'a Path,
    pub started_at: DateTime<Utc>,
    pub frontmatter: RunManifestFrontmatter,
    pub narrative: Option<RunNarrative>,
    pub appendix: DaemonEvidenceAppendix,
    pub disagreements: Vec<ClaimObservationDisagreement>,
    pub missing_narrative_reason: &'a str,
}

pub fn write_hybrid_run_manifest(request: ManifestWriteRequest<'_>) -> Result<StoredManifest> {
    let store = ArtifactStore::new(request.artifact_root)?;
    let appendix = redact_appendix(&request.appendix)?;

    if let Some(narrative) = request.narrative {
        let narrative = redact_narrative(&narrative);
        let draft = RunManifestDraft {
            frontmatter: request.frontmatter,
            narrative: Some(narrative),
            appendix: appendix.clone(),
            disagreements: request.disagreements.clone(),
        };

        return store
            .write_run_manifest(&draft, request.started_at)
            .or_else(|error| {
                let mut fallback = RunManifestDraft::fallback(
                    draft.frontmatter,
                    appendix,
                    &format!("agent narrative invalid: {error}"),
                );
                fallback.disagreements = request.disagreements;
                store.write_run_manifest(&fallback, request.started_at)
            });
    }

    let mut fallback = RunManifestDraft::fallback(
        request.frontmatter,
        appendix,
        request.missing_narrative_reason,
    );
    fallback.disagreements = request.disagreements;
    store.write_run_manifest(&fallback, request.started_at)
}

fn redact_narrative(narrative: &RunNarrative) -> RunNarrative {
    RunNarrative {
        summary: redact_sensitive_text(&narrative.summary),
        steps_taken: redact_sensitive_text(&narrative.steps_taken),
        decisions_made: redact_sensitive_text(&narrative.decisions_made),
        warnings: redact_sensitive_text(&narrative.warnings),
        evidence: redact_sensitive_text(&narrative.evidence),
    }
}

fn redact_appendix(appendix: &DaemonEvidenceAppendix) -> Result<DaemonEvidenceAppendix> {
    let value =
        serde_json::to_value(appendix).context("serializing daemon appendix for redaction")?;
    let redacted = redact_bid_skill_input(&value);
    serde_json::from_value::<DaemonEvidenceAppendix>(redacted)
        .context("deserializing redacted daemon appendix")
}

#[cfg(test)]
fn redact_bid_skill_input(value: &Value) -> Value {
    match value {
        Value::Array(items) => Value::Array(items.iter().map(redact_bid_skill_input).collect()),
        Value::Object(object) => Value::Object(
            object
                .iter()
                .map(|(key, value)| {
                    let normalized = key.replace(['_', '-'], "").to_ascii_lowercase();
                    if normalized.ends_with("hash") {
                        return (key.clone(), value.clone());
                    }
                    let redacted = if matches!(
                        normalized.as_str(),
                        "companyname"
                            | "registrationnumber"
                            | "vatnumber"
                            | "number"
                            | "address"
                            | "line1"
                            | "line2"
                            | "city"
                            | "county"
                            | "postcode"
                            | "country"
                            | "primarycontact"
                            | "email"
                            | "phone"
                            | "role"
                            | "policynumber"
                            | "coveramountpence"
                            | "companieshouseref"
                            | "portalopportunityref"
                            | "filepath"
                            | "documentpath"
                    ) {
                        Value::String("[REDACTED]".to_owned())
                    } else {
                        redact_bid_skill_input(value)
                    };
                    (key.clone(), redacted)
                })
                .collect(),
        ),
        Value::String(text) => Value::String(redact_sensitive_text(text)),
        _ => value.clone(),
    }
}

#[cfg(test)]
fn redact_sensitive_text(text: &str) -> String {
    text.split_whitespace()
        .map(|token| {
            if token.contains("Sentinel-L6-2026") || token.contains("@sorelon.test") {
                "[REDACTED]"
            } else {
                token
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
