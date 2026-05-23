#![forbid(unsafe_code)]
#![allow(clippy::duplicate_mod)]

#[path = "../src/agents/artifacts.rs"]
#[allow(dead_code)]
mod artifacts_impl;
#[path = "../src/validation/manifest_writer.rs"]
#[allow(dead_code)]
mod manifest_writer_impl;

mod agents {
    pub mod artifacts {
        pub use crate::artifacts_impl::*;
    }
}

mod tests {
    use std::fs;

    use anyhow::{Context, Result, ensure};
    use chrono::{TimeZone, Utc};
    use serde_json::{Value, json};
    use tempfile::tempdir;

    use crate::artifacts_impl::{
        BidScopeSummary, DaemonEvidenceAppendix, GateResult, OperatorOverrideRecord, OutcomeClass,
        ReadinessEvidenceSnapshot, RunManifestFrontmatter, RunNarrative, SideEffectCheckpoint,
        ValidatorVerdictSummary,
    };
    use crate::manifest_writer_impl::{ManifestWriteRequest, write_hybrid_run_manifest};

    fn fixed_started_at() -> Result<chrono::DateTime<Utc>> {
        Utc.with_ymd_and_hms(2026, 5, 23, 9, 30, 0)
            .single()
            .context("constructing fixed manifest timestamp")
    }

    fn frontmatter() -> RunManifestFrontmatter {
        RunManifestFrontmatter {
            manifest_version: 1,
            skill_id: "bid-portal/procontract".to_owned(),
            skill_version: "bid-portal/procontract@0.1.0+1.0".to_owned(),
            job_id: "bid-job-001".to_owned(),
            session_id: "session-bid-manifest-001".to_owned(),
            execution_mode: "pty".to_owned(),
            started_at: "2026-05-23T09:30:00Z".to_owned(),
            completed_at: "2026-05-23T09:31:00Z".to_owned(),
            outcome_class: OutcomeClass::Completed,
            items_processed: 1,
            items_skipped: 0,
            items_failed: 0,
            warnings_count: 0,
            agent_claimed_pages_visited: 3,
            agent_claimed_source_url: "https://procontract.due-north.com".to_owned(),
            agent_claimed_fingerprint_state: "procontract_bid_portal".to_owned(),
            agent_claimed_locale_state: "en-GB".to_owned(),
            agent_claimed_currency_state: "GBP".to_owned(),
            agent_narrative_missing: false,
        }
    }

    fn narrative() -> RunNarrative {
        RunNarrative {
            summary: "Bid workflow reached draft_saved for the scoped Procontract run.".to_owned(),
            steps_taken: "Daemon recorded checkpoints and portal evidence.".to_owned(),
            decisions_made: "Scoped identifiers are retained in the daemon appendix.".to_owned(),
            warnings: "None.".to_owned(),
            evidence: "Use daemon appendix artifact refs for review.".to_owned(),
        }
    }

    fn appendix() -> DaemonEvidenceAppendix {
        DaemonEvidenceAppendix {
            observed_url_chain: vec!["https://procontract.due-north.com/opportunity".to_owned()],
            observed_page_kind_sequence: vec![
                "procontract_opportunity".to_owned(),
                "draft_saved".to_owned(),
            ],
            daemon_observed_fingerprint_state: "procontract_bid_portal".to_owned(),
            daemon_observed_locale: "en-GB".to_owned(),
            daemon_observed_currency: "GBP".to_owned(),
            auth_challenge_events: Vec::new(),
            locator_fallback_depth: 0,
            basis_sufficiency: "high".to_owned(),
            page_fingerprint_hash: Some("portal-fingerprint-hash".to_owned()),
            auth_challenge_detected: false,
            extraction_fill_rate_bps: Some(10_000),
            checkpoint_count: 3,
            byte_count_compressed: Some(2_048),
            validator_rubric_hash: Some("rubric-hash".to_owned()),
            drift_delta_json: Value::Null,
            result_item_count: 1,
            oqg_anomaly_summary: vec![
                "side_effect_reached=RemoteDraft; side_effect_cap=RemoteDraft".to_owned(),
            ],
            representative_item_sample: json!({
                "process_id_hash": "process-hash",
                "bid_state": "draft_saved"
            }),
            artifact_refs: vec![
                "sessions/2026/05/session-bid-manifest-001/step-01.webp".to_owned(),
                "sessions/2026/05/session-bid-manifest-001/terminal-output.txt".to_owned(),
            ],
            bid_scope_summary: Some(BidScopeSummary {
                process_id_hash: "process-hash".to_owned(),
                lot_id_hashes: vec!["lot-hash".to_owned()],
                candidate_id_hashes: vec!["candidate-hash".to_owned()],
                selected_quote_id_hashes: vec!["quote-hash".to_owned()],
                portal_opportunity_ref_hash: "portal-ref-hash".to_owned(),
                decision_basis_hash: "decision-basis-hash".to_owned(),
                side_effect_cap: "RemoteDraft".to_owned(),
                review_required: true,
            }),
            side_effect_progression: vec![
                SideEffectCheckpoint {
                    step: "01".to_owned(),
                    side_effect_class: "ReadOnly".to_owned(),
                    screenshot_path: "step-01.webp".to_owned(),
                    timestamp: "2026-05-23T09:30:00Z".to_owned(),
                },
                SideEffectCheckpoint {
                    step: "02".to_owned(),
                    side_effect_class: "RemoteDraft".to_owned(),
                    screenshot_path: "step-02.webp".to_owned(),
                    timestamp: "2026-05-23T09:30:30Z".to_owned(),
                },
            ],
            readiness_evidence: Some(ReadinessEvidenceSnapshot {
                gates_evaluated_at_start: vec![GateResult {
                    code: "ALL_READINESS_GATES_PASSED".to_owned(),
                    source: "bid_workflow_readiness_gate".to_owned(),
                    passed: true,
                    overrideable: false,
                }],
                gates_re_evaluated_at_final_submit: vec![GateResult {
                    code: "ALL_READINESS_GATES_PASSED".to_owned(),
                    source: "bid_workflow_readiness_gate".to_owned(),
                    passed: true,
                    overrideable: false,
                }],
                decision_basis_hash_at_start: "decision-basis-hash".to_owned(),
                decision_basis_hash_at_final_submit: Some("decision-basis-hash".to_owned()),
            }),
            validator_verdict: Some(ValidatorVerdictSummary {
                verdict: "approved".to_owned(),
                rubric_version: "validation/overlay-bid-portal-procontract@pending".to_owned(),
                reasons: vec!["portal_state_verified=true".to_owned()],
            }),
            override_metadata: Some(OperatorOverrideRecord {
                kind: "validation_gate".to_owned(),
                reason: "operator accepted validator outage for final submit".to_owned(),
                recorded_by: "operator@example.test".to_owned(),
                recorded_at: "2026-05-23T09:30:45Z".to_owned(),
            }),
            final_bid_state: Some("draft_saved".to_owned()),
        }
    }

    #[test]
    #[doc = "L6-T-daemon_runner-008"]
    fn bid_manifest_renders_l6_daemon_appendix_fields() -> Result<()> {
        let temp = tempdir()?;
        let stored = write_hybrid_run_manifest(ManifestWriteRequest {
            artifact_root: temp.path(),
            started_at: fixed_started_at()?,
            frontmatter: frontmatter(),
            narrative: Some(narrative()),
            appendix: appendix(),
            disagreements: Vec::new(),
            missing_narrative_reason: "not used",
        })?;
        let manifest = fs::read_to_string(&stored.path)?;
        let appendix_block = manifest
            .split("## Daemon Evidence Appendix")
            .nth(1)
            .context("daemon evidence appendix section missing")?;

        for expected in [
            "bid_scope_summary:",
            "side_effect_progression:",
            "readiness_evidence:",
            "validator_verdict:",
            "override_metadata:",
            "final_bid_state: draft_saved",
            "side_effect_class: RemoteDraft",
            "screenshot_path: step-02.webp",
            "gates_re_evaluated_at_final_submit:",
        ] {
            ensure!(
                appendix_block.contains(expected),
                "appendix missing expected L6 field `{expected}`"
            );
        }
        Ok(())
    }

    #[test]
    #[doc = "L6-T-daemon_runner-008"]
    fn bid_manifest_keeps_bid_identifiers_out_of_agent_narrative() -> Result<()> {
        let temp = tempdir()?;
        let stored = write_hybrid_run_manifest(ManifestWriteRequest {
            artifact_root: temp.path(),
            started_at: fixed_started_at()?,
            frontmatter: frontmatter(),
            narrative: Some(narrative()),
            appendix: appendix(),
            disagreements: Vec::new(),
            missing_narrative_reason: "not used",
        })?;
        let manifest = fs::read_to_string(&stored.path)?;
        let narrative_block = manifest
            .split("## Daemon Evidence Appendix")
            .next()
            .context("manifest narrative prefix missing")?;

        for raw_identifier in ["process-raw", "lot-raw", "candidate-raw", "quote-raw"] {
            ensure!(
                !narrative_block.contains(raw_identifier),
                "raw bid identifier leaked into agent narrative: {raw_identifier}"
            );
        }
        ensure!(
            manifest.contains("process_id_hash: process-hash"),
            "hashed bid scope evidence should remain in daemon appendix"
        );
        Ok(())
    }
}
