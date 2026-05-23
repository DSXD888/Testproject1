#![forbid(unsafe_code)]
#![allow(clippy::duplicate_mod)]

#[path = "../src/agents/artifacts.rs"]
#[allow(dead_code)]
mod artifacts_impl;
#[path = "../src/agents/contracts.rs"]
#[allow(dead_code)]
mod contracts_impl;
#[path = "support/db.rs"]
mod db_support;
#[path = "../src/validation/executor.rs"]
#[allow(dead_code)]
mod executor_impl;
#[path = "../src/validation/manifest_writer.rs"]
#[allow(dead_code)]
mod manifest_writer_impl;
#[path = "../src/jobs/runner.rs"]
#[allow(dead_code)]
mod runner_impl;
#[path = "../src/bid_workflow/state_machine.rs"]
#[allow(dead_code)]
mod state_machine_impl;
#[path = "../src/bid_workflow/types.rs"]
#[allow(dead_code)]
mod types;

mod agents {
    pub mod artifacts {
        pub use crate::artifacts_impl::*;
    }
}

mod jobs {
    pub mod runner {
        pub use crate::runner_impl::*;
    }
}

mod support {
    pub mod db {
        pub use crate::db_support::*;
    }
}

mod tests {
    use std::path::{Path, PathBuf};

    use anyhow::{Context, Result, ensure};
    use chrono::{TimeZone, Utc};
    use serde_json::{Value, json};
    use tempfile::tempdir;

    use crate::artifacts_impl::{
        BidScopeSummary, DaemonEvidenceAppendix, GateResult, OperatorOverrideRecord, OutcomeClass,
        ReadinessEvidenceSnapshot, RunManifestFrontmatter, RunNarrative, SideEffectCheckpoint,
        ValidatorVerdictSummary,
    };
    use crate::executor_impl::ValidatorSkillBundle;
    use crate::manifest_writer_impl::{ManifestWriteRequest, write_hybrid_run_manifest};
    use crate::state_machine_impl::{
        BidStateMachine, BidStateTransitionRejection,
        OperatorOverrideRecord as StateMachineOverrideRecord, TransitionContext, ValidatorGate,
    };
    use crate::types::{BidWorkflowState, SideEffectClass};

    fn repo_root() -> Result<PathBuf> {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(2)
            .map(Path::to_path_buf)
            .context("resolving repository root")
    }

    fn assert_rejected(
        from: BidWorkflowState,
        to: BidWorkflowState,
        context: &TransitionContext,
        reason: BidStateTransitionRejection,
    ) {
        let mut machine = BidStateMachine::new(from);
        let error = machine
            .transition_to(to, context)
            .expect_err("transition should fail closed");

        assert_eq!(error.from(), from);
        assert_eq!(error.to(), to);
        assert_eq!(error.reason(), reason);
        assert_eq!(machine.state(), from);
    }

    fn structured_override() -> StateMachineOverrideRecord {
        StateMachineOverrideRecord {
            kind: "validation_gate".to_owned(),
            reason: "Operator reviewed the validator outage against captured portal evidence."
                .to_owned(),
            recorded_by: "operator-alinaaitova".to_owned(),
            recorded_at: "2026-05-23T12:00:00Z".to_owned(),
        }
    }

    fn artifact_override() -> OperatorOverrideRecord {
        OperatorOverrideRecord {
            kind: "validation_gate".to_owned(),
            reason: "Operator reviewed the validator outage against captured portal evidence."
                .to_owned(),
            recorded_by: "operator-alinaaitova".to_owned(),
            recorded_at: "2026-05-23T12:00:00Z".to_owned(),
        }
    }

    fn final_submit_context(
        from: BidWorkflowState,
        validator_gate: ValidatorGate,
    ) -> TransitionContext {
        let mut context = TransitionContext::satisfied_for(from, BidWorkflowState::BidSubmitted);
        context.validator_gate = validator_gate;
        context
    }

    fn frontmatter() -> RunManifestFrontmatter {
        RunManifestFrontmatter {
            manifest_version: 1,
            skill_id: "bid-portal/procontract".to_owned(),
            skill_version: "bid-portal/procontract@0.1.0+1.0".to_owned(),
            job_id: "bid-job-validator-overlay".to_owned(),
            session_id: "session-bid-validator-overlay".to_owned(),
            execution_mode: "pty".to_owned(),
            started_at: "2026-05-23T12:00:00Z".to_owned(),
            completed_at: "2026-05-23T12:02:00Z".to_owned(),
            outcome_class: OutcomeClass::Completed,
            items_processed: 1,
            items_skipped: 0,
            items_failed: 0,
            warnings_count: 0,
            agent_claimed_pages_visited: 11,
            agent_claimed_source_url: "https://procontract.due-north.com".to_owned(),
            agent_claimed_fingerprint_state: "procontract_bid_portal".to_owned(),
            agent_claimed_locale_state: "en-GB".to_owned(),
            agent_claimed_currency_state: "GBP".to_owned(),
            agent_narrative_missing: false,
        }
    }

    fn appendix(override_metadata: Option<OperatorOverrideRecord>) -> DaemonEvidenceAppendix {
        DaemonEvidenceAppendix {
            observed_url_chain: vec!["https://procontract.due-north.com/opportunity".to_owned()],
            observed_page_kind_sequence: vec![
                "procontract_opportunity".to_owned(),
                "bid_submitted".to_owned(),
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
            checkpoint_count: 11,
            byte_count_compressed: Some(2_048),
            validator_rubric_hash: Some("rubric-hash".to_owned()),
            drift_delta_json: Value::Null,
            result_item_count: 1,
            oqg_anomaly_summary: vec!["validation_override_logged=true".to_owned()],
            representative_item_sample: json!({"bid_state": "bid_submitted"}),
            artifact_refs: vec!["sessions/test/step-11-confirmation.webp".to_owned()],
            bid_scope_summary: Some(BidScopeSummary {
                process_id_hash: "process-hash".to_owned(),
                lot_id_hashes: vec!["lot-hash".to_owned()],
                candidate_id_hashes: vec!["candidate-hash".to_owned()],
                selected_quote_id_hashes: vec!["quote-hash".to_owned()],
                portal_opportunity_ref_hash: "portal-ref-hash".to_owned(),
                decision_basis_hash: "decision-basis-hash".to_owned(),
                side_effect_cap: "FormalSubmission".to_owned(),
                review_required: true,
            }),
            side_effect_progression: vec![SideEffectCheckpoint {
                step: "11".to_owned(),
                side_effect_class: "FormalSubmission".to_owned(),
                screenshot_path: "step-11-confirmation.webp".to_owned(),
                timestamp: "2026-05-23T12:02:00Z".to_owned(),
            }],
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
                verdict: "rejected".to_owned(),
                rubric_version: "validation/overlay-bid-portal-procontract@0.1.0".to_owned(),
                reasons: vec!["validator_outage_or_rejection_override=true".to_owned()],
            }),
            override_metadata,
            final_bid_state: Some("bid_submitted".to_owned()),
        }
    }

    #[test]
    #[doc = "L6-T-validator_overlay-008"]
    fn bid_portal_overlay_routes_through_existing_validator_bundle_and_rejection_blocks_submit()
    -> Result<()> {
        let bundle = ValidatorSkillBundle::load_bid_portal_procontract_overlay(&repo_root()?)?;

        ensure!(bundle.skill_path == "skills/validation/overlay-bid-portal-procontract/SKILL.md");
        ensure!(bundle.result_schema_path == "skills/validation/base/result.schema.json");
        ensure!(
            bundle.validator_rubric_path == "skills/bid-portal-procontract/validator-rubric.yaml"
        );
        ensure!(bundle.model_profile == "structured_extract_low_cost");
        ensure!(bundle.worker_behavior_version == "bid-portal/procontract@0.1.0+1.0");
        ensure!(
            bundle.validator_behavior_version
                == "validation/overlay-bid-portal-procontract@0.1.0+1.0"
        );

        let context =
            final_submit_context(BidWorkflowState::BidInProgress, ValidatorGate::Rejected);
        assert_rejected(
            BidWorkflowState::BidInProgress,
            BidWorkflowState::BidSubmitted,
            &context,
            BidStateTransitionRejection::ValidationGateBlocked,
        );
        Ok(())
    }

    #[test]
    #[doc = "L6-T-validator_overlay-009"]
    fn validator_outage_blocks_final_submit_but_draft_save_remains_reachable() {
        let outage_context =
            final_submit_context(BidWorkflowState::DraftSaved, ValidatorGate::Outage);
        assert_rejected(
            BidWorkflowState::DraftSaved,
            BidWorkflowState::BidSubmitted,
            &outage_context,
            BidStateTransitionRejection::ValidationGateBlocked,
        );

        let mut draft_context = TransitionContext::satisfied_for(
            BidWorkflowState::BidInProgress,
            BidWorkflowState::DraftSaved,
        );
        draft_context.validator_gate = ValidatorGate::Outage;
        draft_context.side_effect_reached = Some(SideEffectClass::RemoteDraft);
        let mut machine = BidStateMachine::new(BidWorkflowState::BidInProgress);
        let state = machine
            .transition_to(BidWorkflowState::DraftSaved, &draft_context)
            .expect("draft_saved must not require validator clearance");
        assert_eq!(state, BidWorkflowState::DraftSaved);
    }

    #[test]
    #[doc = "L6-T-validator_overlay-010"]
    fn structured_override_allows_final_submit_and_renders_manifest_audit() -> Result<()> {
        let mut boolean_only =
            final_submit_context(BidWorkflowState::DraftSaved, ValidatorGate::Rejected);
        boolean_only.validation_override_logged = true;
        assert_rejected(
            BidWorkflowState::DraftSaved,
            BidWorkflowState::BidSubmitted,
            &boolean_only,
            BidStateTransitionRejection::ValidationGateBlocked,
        );

        let mut structured =
            final_submit_context(BidWorkflowState::DraftSaved, ValidatorGate::Rejected);
        structured.validation_override_logged = true;
        structured.validation_override = Some(structured_override());
        let mut machine = BidStateMachine::new(BidWorkflowState::DraftSaved);
        let state = machine
            .transition_to(BidWorkflowState::BidSubmitted, &structured)
            .expect("structured validation override should allow final submit");
        assert_eq!(state, BidWorkflowState::BidSubmitted);

        let temp = tempdir()?;
        let stored = write_hybrid_run_manifest(ManifestWriteRequest {
            artifact_root: temp.path(),
            started_at: Utc
                .with_ymd_and_hms(2026, 5, 23, 12, 0, 0)
                .single()
                .context("building fixed timestamp")?,
            frontmatter: frontmatter(),
            narrative: Some(RunNarrative {
                summary: "Bid workflow reached bid_submitted with validation override.".to_owned(),
                steps_taken: "Daemon recorded screenshots and validator output.".to_owned(),
                decisions_made: "Operator override was attached to the daemon appendix.".to_owned(),
                warnings: "Validator gate override present.".to_owned(),
                evidence: "See daemon evidence appendix override_metadata.".to_owned(),
            }),
            appendix: appendix(Some(artifact_override())),
            disagreements: Vec::new(),
            missing_narrative_reason: "not used",
        })?;
        let manifest = std::fs::read_to_string(stored.path)?;
        ensure!(manifest.contains("override_metadata:"));
        ensure!(manifest.contains("kind: validation_gate"));
        ensure!(manifest.contains("recorded_by: operator-alinaaitova"));
        Ok(())
    }
}
