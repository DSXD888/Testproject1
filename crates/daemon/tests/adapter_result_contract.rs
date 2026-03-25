#![forbid(unsafe_code)]

mod support;

#[path = "../src/agents/artifacts.rs"]
mod artifacts_impl;
#[path = "support/assertions.rs"]
mod assertions_support;

#[cfg(unix)]
mod tests {
    use std::fs;
    use std::path::Path;

    use anyhow::{Result, ensure};
    use chrono::{TimeZone, Utc};
    use rusqlite::params;
    use serde_json::json;

    use crate::artifacts_impl;
    use crate::artifacts_impl::{
        ArtifactStore, DaemonEvidenceAppendix, OutcomeClass, RunManifestDraft,
        RunManifestFrontmatter, RunNarrative, persist_run_manifest_metadata,
        read_contiguous_jsonl_prefix, validate_result_payload,
    };
    use crate::assertions_support::ManifestAssert;
    use crate::support::TestContext;
    use crate::support::db::TestStateDb;

    fn fixed_started_at() -> Result<chrono::DateTime<Utc>> {
        Utc.with_ymd_and_hms(2026, 3, 25, 3, 0, 0)
            .single()
            .ok_or_else(|| anyhow::anyhow!("failed to construct fixed test timestamp"))
    }

    fn sample_frontmatter(outcome_class: OutcomeClass) -> RunManifestFrontmatter {
        RunManifestFrontmatter {
            manifest_version: 1,
            skill_id: "skill/fts-ingestion".to_owned(),
            skill_version: "2026.03.25".to_owned(),
            job_id: "job-123".to_owned(),
            session_id: "session-123".to_owned(),
            execution_mode: "contract".to_owned(),
            started_at: "2026-03-25T03:00:00Z".to_owned(),
            completed_at: "2026-03-25T03:01:00Z".to_owned(),
            outcome_class,
            items_processed: 10,
            items_skipped: 1,
            items_failed: 0,
            warnings_count: 1,
            agent_claimed_pages_visited: 4,
            agent_claimed_source_url: "https://www.find-tender.service.gov.uk".to_owned(),
            agent_claimed_fingerprint_state: "matched".to_owned(),
            agent_claimed_locale_state: "en-GB".to_owned(),
            agent_claimed_currency_state: "GBP".to_owned(),
            agent_narrative_missing: false,
        }
    }

    fn sample_narrative() -> RunNarrative {
        RunNarrative {
            summary: "Processed the source successfully.".to_owned(),
            steps_taken: "Fetched source, parsed lots, wrote artifacts.".to_owned(),
            decisions_made: "Retained only durable evidence for the final manifest.".to_owned(),
            warnings: "One low-priority warning remained for review.".to_owned(),
            evidence: "See checkpoint and result artifact refs in the appendix.".to_owned(),
        }
    }

    fn sample_appendix() -> DaemonEvidenceAppendix {
        DaemonEvidenceAppendix {
            observed_url_chain: vec![
                "https://www.find-tender.service.gov.uk".to_owned(),
                "https://www.find-tender.service.gov.uk/notice/123".to_owned(),
            ],
            observed_page_kind_sequence: vec!["listing".to_owned(), "notice".to_owned()],
            daemon_observed_fingerprint_state: "matched".to_owned(),
            daemon_observed_locale: "en-GB".to_owned(),
            daemon_observed_currency: "GBP".to_owned(),
            auth_challenge_events: vec!["none".to_owned()],
            locator_fallback_depth: 2,
            checkpoint_count: 3,
            result_item_count: 12,
            oqg_anomaly_summary: vec!["duplicate_rate:low".to_owned()],
            representative_item_sample: json!({
                "notice_id": "123",
                "title": "PPE supply"
            }),
            artifact_refs: vec![
                "checkpoints/checkpoint-1.json".to_owned(),
                "results/result.json".to_owned(),
            ],
        }
    }

    fn sample_draft(outcome_class: OutcomeClass) -> RunManifestDraft {
        RunManifestDraft {
            frontmatter: sample_frontmatter(outcome_class),
            narrative: Some(sample_narrative()),
            appendix: sample_appendix(),
            disagreements: Vec::new(),
        }
    }

    #[test]
    fn ut_rsl_001_valid_result_schema_payloads_pass_structural_validation() -> Result<()> {
        let ctx = TestContext::new()?;
        ensure!(ctx.temp.path().exists(), "tempdir missing");
        ensure!(
            ctx.state_root.ends_with("state"),
            "state_root shape changed"
        );
        ensure!(
            ctx.runtime_root.ends_with("runtime"),
            "runtime_root shape changed"
        );
        ensure!(
            ctx.db_path.ends_with("state/state.db"),
            "db_path shape changed"
        );

        let schema = json!({
            "type": "object",
            "required": ["notice_id", "score_bps", "items"],
            "properties": {
                "notice_id": { "type": "string" },
                "score_bps": { "type": "integer" },
                "items": { "type": "array" }
            }
        });
        let payload = json!({
            "notice_id": "ocds-123",
            "score_bps": 8200,
            "items": []
        });

        validate_result_payload(&schema, &payload)?;
        Ok(())
    }

    #[test]
    fn ut_rsl_002_required_frontmatter_fields_round_trip_through_yaml() -> Result<()> {
        let ctx = TestContext::new()?;
        let store = ArtifactStore::new(&ctx.artifact_root)?;
        ensure!(
            store.root() == ctx.artifact_root.as_path(),
            "artifact root changed unexpectedly"
        );
        let started_at = fixed_started_at()?;
        let stored =
            store.write_run_manifest(&sample_draft(OutcomeClass::Completed), started_at)?;
        let manifest = ManifestAssert::load(&stored.path)?;
        let frontmatter: RunManifestFrontmatter = manifest.frontmatter()?;

        ensure!(
            frontmatter.skill_id == "skill/fts-ingestion",
            "unexpected skill_id"
        );
        ensure!(frontmatter.job_id == "job-123", "unexpected job_id");
        ensure!(
            frontmatter.session_id == "session-123",
            "unexpected session_id"
        );
        ensure!(
            frontmatter.agent_claimed_source_url.starts_with("https://"),
            "source URL missing"
        );
        Ok(())
    }

    #[test]
    fn ut_rsl_003_required_narrative_sections_must_be_present_and_non_empty() -> Result<()> {
        let ctx = TestContext::new()?;
        let store = ArtifactStore::new(&ctx.artifact_root)?;
        let started_at = fixed_started_at()?;
        let stored =
            store.write_run_manifest(&sample_draft(OutcomeClass::Completed), started_at)?;
        let manifest = ManifestAssert::load(&stored.path)?;

        manifest.require_non_empty_sections(&[
            "Summary",
            "Steps Taken",
            "Decisions Made",
            "Warnings",
            "Evidence",
        ])?;
        Ok(())
    }

    #[test]
    fn ut_rsl_004_agent_claimed_and_daemon_observed_naming_never_collide() -> Result<()> {
        let ctx = TestContext::new()?;
        let store = ArtifactStore::new(&ctx.artifact_root)?;
        let started_at = fixed_started_at()?;
        let stored =
            store.write_run_manifest(&sample_draft(OutcomeClass::Completed), started_at)?;
        let manifest = ManifestAssert::load(&stored.path)?;
        let frontmatter: serde_yaml::Value = manifest.frontmatter()?;
        let appendix: serde_yaml::Value = manifest.appendix_yaml()?;

        let frontmatter_keys = frontmatter
            .as_mapping()
            .into_iter()
            .flat_map(|mapping| mapping.keys())
            .filter_map(serde_yaml::Value::as_str)
            .collect::<Vec<_>>();
        let appendix_keys = appendix
            .as_mapping()
            .into_iter()
            .flat_map(|mapping| mapping.keys())
            .filter_map(serde_yaml::Value::as_str)
            .collect::<Vec<_>>();

        ensure!(
            frontmatter_keys
                .iter()
                .all(|key| !key.starts_with("daemon_observed_")),
            "frontmatter contained daemon_observed_* keys"
        );
        ensure!(
            appendix_keys
                .iter()
                .all(|key| !key.starts_with("agent_claimed_")),
            "appendix contained agent_claimed_* keys"
        );
        Ok(())
    }

    #[test]
    fn ut_rsl_005_claim_observation_disagreements_append_explicit_summary() -> Result<()> {
        let ctx = TestContext::new()?;
        let store = ArtifactStore::new(&ctx.artifact_root)?;
        let started_at = fixed_started_at()?;
        let mut draft = sample_draft(OutcomeClass::Completed);
        draft
            .disagreements
            .push(artifacts_impl::ClaimObservationDisagreement {
                field: "locale_state".to_owned(),
                agent_claimed: "en-GB".to_owned(),
                daemon_observed: "fr-FR".to_owned(),
            });

        let stored = store.write_run_manifest(&draft, started_at)?;
        ensure!(
            stored.warning_review_required,
            "disagreement should mark manifest warning-review-required"
        );
        let manifest = ManifestAssert::load(&stored.path)?;
        let disagreement_lines = manifest.disagreement_lines()?;
        ensure!(
            disagreement_lines
                .iter()
                .any(|line| line.contains("locale_state") && line.contains("fr-FR")),
            "disagreement summary missing expected field"
        );
        Ok(())
    }

    #[test]
    fn ut_rsl_006_continuation_required_is_distinct_from_partial_failure() -> Result<()> {
        let frontmatter = sample_frontmatter(OutcomeClass::ContinuationRequired);
        let yaml = serde_yaml::to_string(&frontmatter)?;
        ensure!(
            yaml.contains("outcome_class: continuation_required"),
            "continuation_required outcome was not serialized distinctly"
        );
        ensure!(
            !yaml.contains("AGENT_PARTIAL_FAILURE"),
            "continuation_required should not collapse into AGENT_PARTIAL_FAILURE"
        );
        Ok(())
    }

    #[test]
    fn ut_rsl_007_degraded_completed_is_distinct_from_failure_outcomes() -> Result<()> {
        let frontmatter = sample_frontmatter(OutcomeClass::DegradedCompleted);
        let yaml = serde_yaml::to_string(&frontmatter)?;
        ensure!(
            yaml.contains("outcome_class: degraded_completed"),
            "degraded_completed outcome was not serialized distinctly"
        );
        ensure!(
            !yaml.contains("outcome_class: failed"),
            "degraded_completed should not serialize as failure"
        );
        Ok(())
    }

    #[test]
    fn it_rsl_008_daemon_synthesizes_fallback_manifest_when_narrative_missing() -> Result<()> {
        let ctx = TestContext::new()?;
        let store = ArtifactStore::new(&ctx.artifact_root)?;
        let started_at = fixed_started_at()?;
        let draft = RunManifestDraft::fallback(
            sample_frontmatter(OutcomeClass::BlockedHumanAction),
            sample_appendix(),
            "agent lost connectivity",
        );

        let stored = store.write_run_manifest(&draft, started_at)?;
        let manifest = ManifestAssert::load(&stored.path)?;
        let frontmatter: RunManifestFrontmatter = manifest.frontmatter()?;

        ensure!(
            frontmatter.agent_narrative_missing,
            "fallback manifest missing fallback flag"
        );
        manifest.require_non_empty_sections(&[
            "Summary",
            "Steps Taken",
            "Decisions Made",
            "Warnings",
            "Evidence",
        ])?;
        let appendix: DaemonEvidenceAppendix = manifest.appendix_yaml()?;
        ensure!(
            appendix.checkpoint_count == 3,
            "appendix missing machine evidence"
        );
        Ok(())
    }

    #[test]
    fn it_rsl_009_manifest_path_hash_and_size_budget_are_persisted() -> Result<()> {
        let ctx = TestContext::new()?;
        let store = ArtifactStore::new(&ctx.artifact_root)?;
        let started_at = fixed_started_at()?;
        let stored =
            store.write_run_manifest(&sample_draft(OutcomeClass::Completed), started_at)?;
        let state_db = TestStateDb::new()?;
        ensure!(
            state_db.db_path().ends_with("state.db"),
            "test db path changed"
        );

        state_db.connection().execute(
            "\
            INSERT INTO agent_session_logs (
              id, skill_path, execution_mode, status
            ) VALUES (?1, ?2, ?3, ?4)
            ",
            params![
                "session-123",
                "skills/fts/SKILL.md",
                "contract",
                "completed"
            ],
        )?;

        persist_run_manifest_metadata(state_db.connection(), "session-123", &stored)?;
        let (path, hash): (String, String) = state_db.connection().query_row(
            "SELECT run_manifest_path, run_manifest_hash FROM agent_session_logs WHERE id = ?1",
            params!["session-123"],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;

        let bytes = fs::read(&stored.path)?;
        let computed = {
            use sha2::{Digest, Sha256};
            let digest = Sha256::digest(&bytes);
            format!("{digest:x}")
        };

        ensure!(
            Path::new(&path) == stored.path.as_path(),
            "persisted manifest path mismatch"
        );
        ensure!(hash == computed, "persisted manifest hash mismatch");
        ensure!(
            stored.size_bytes < 50 * 1024,
            "manifest exceeded size budget"
        );
        ensure!(
            ManifestAssert::load(&stored.path)?.file_size()? == stored.size_bytes,
            "stored size mismatch"
        );
        Ok(())
    }

    #[test]
    fn it_rsl_011_daemon_appendix_contains_machine_observed_fields() -> Result<()> {
        let ctx = TestContext::new()?;
        let store = ArtifactStore::new(&ctx.artifact_root)?;
        let started_at = fixed_started_at()?;
        let stored =
            store.write_run_manifest(&sample_draft(OutcomeClass::Completed), started_at)?;
        let manifest = ManifestAssert::load(&stored.path)?;
        let appendix: DaemonEvidenceAppendix = manifest.appendix_yaml()?;

        ensure!(appendix.checkpoint_count == 3, "checkpoint count missing");
        ensure!(appendix.result_item_count == 12, "result count missing");
        ensure!(
            appendix.oqg_anomaly_summary == vec!["duplicate_rate:low".to_owned()],
            "OQG summary missing"
        );
        ensure!(
            appendix.artifact_refs
                == vec![
                    "checkpoints/checkpoint-1.json".to_owned(),
                    "results/result.json".to_owned(),
                ],
            "artifact refs missing"
        );
        Ok(())
    }

    #[test]
    fn it_rsl_012_manifest_index_readers_expose_only_contiguous_prefix_after_recovery() -> Result<()>
    {
        let ctx = TestContext::new()?;
        let index_path = ctx.artifact_root.join("manifest.index.jsonl");
        fs::create_dir_all(&ctx.artifact_root)?;
        fs::write(
            &index_path,
            concat!(
                "{\"seq\":1,\"path\":\"run-manifest-1.md\"}\n",
                "{\"seq\":2,\"path\":\"run-manifest-2.md\"}\n",
                "{\"seq\":3,\"path\":\"run-manifest-3.md\""
            ),
        )?;

        let prefix = read_contiguous_jsonl_prefix(&index_path)?;
        ensure!(
            prefix.len() == 2,
            "reader did not stop at contiguous prefix"
        );
        ensure!(prefix[0]["seq"] == 1, "first committed row missing");
        ensure!(prefix[1]["seq"] == 2, "second committed row missing");
        Ok(())
    }
}
