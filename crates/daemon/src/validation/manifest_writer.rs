#![forbid(unsafe_code)]

use std::path::Path;

use anyhow::Result;
use chrono::{DateTime, Utc};

use crate::agents::artifacts::{
    ArtifactStore, ClaimObservationDisagreement, DaemonEvidenceAppendix, RunManifestDraft,
    RunManifestFrontmatter, RunNarrative, StoredManifest,
};

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

    if let Some(narrative) = request.narrative {
        let draft = RunManifestDraft {
            frontmatter: request.frontmatter,
            narrative: Some(narrative),
            appendix: request.appendix.clone(),
            disagreements: request.disagreements.clone(),
        };

        return store
            .write_run_manifest(&draft, request.started_at)
            .or_else(|error| {
                let mut fallback = RunManifestDraft::fallback(
                    draft.frontmatter,
                    request.appendix,
                    &format!("agent narrative invalid: {error}"),
                );
                fallback.disagreements = request.disagreements;
                store.write_run_manifest(&fallback, request.started_at)
            });
    }

    let mut fallback = RunManifestDraft::fallback(
        request.frontmatter,
        request.appendix,
        request.missing_narrative_reason,
    );
    fallback.disagreements = request.disagreements;
    store.write_run_manifest(&fallback, request.started_at)
}
