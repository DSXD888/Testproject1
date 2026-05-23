#![forbid(unsafe_code)]

#[allow(dead_code)]
#[path = "support/skills.rs"]
mod skills_support;

use anyhow::{Result, ensure};

use skills_support::{load_skill_metadata, repo_root};

#[test]
fn l6_t_validator_overlay_001_bid_portal_procontract_frontmatter_declares_overlay_contract()
-> Result<()> {
    let overlay = load_skill_metadata(
        &repo_root()?.join("skills/validation/overlay-bid-portal-procontract"),
    )?;

    ensure!(overlay.metadata.skill_id == "validation/overlay-bid-portal-procontract");
    ensure!(overlay.metadata.skill_version == "0.1.0");
    ensure!(overlay.metadata.interaction_profile == "headless_contract");
    ensure!(overlay.metadata.result_schema_version == "1.0");
    ensure!(overlay.metadata.checkpoint_schema_version.as_deref() == Some("1.0"));
    ensure!(overlay.metadata.validator_overlay_version.as_deref() == Some("0.1.0"));
    ensure!(overlay.metadata.model_profile == "structured_extract_low_cost");
    ensure!(overlay.metadata.side_effect_class == "read_only");
    ensure!(overlay.metadata.promotion_stage.as_deref() == Some("draft"));
    ensure!(
        overlay.metadata.worker_behavior_version.as_deref()
            == Some("bid-portal/procontract@0.1.0+1.0")
    );
    ensure!(
        overlay.metadata.validator_behavior_version.as_deref()
            == Some("validation/overlay-bid-portal-procontract@0.1.0+1.0")
    );
    ensure!(overlay.metadata.version_change_affects_behavior);
    ensure!(overlay.metadata.gate_a_family_key.as_deref() == Some("bid-portal-procontract"));
    ensure!(
        overlay
            .metadata
            .gate_b_exact_slice_key
            .as_deref()
            .is_some_and(|slice| slice.contains("bid-portal/procontract@0.1.0+1.0"))
    );

    for required in [
        "validation_policy: always",
        "rubric_version: bid-portal-procontract-rubric@1.0",
        "worker_skill_id: bid-portal/procontract",
        "run_manifest",
        "daemon_evidence_appendix",
        "portal_screenshots",
        "bid_scope_payload",
        "Assertion 1 - Opportunity reference match",
        "Assertion 2 - Process, lot, candidate, quote, and profile scope match",
        "Assertion 3 - Screenshot completeness",
        "Assertion 4 - Side-effect progression and cap",
        "Assertion 5 - Confirmation or draft evidence shape",
        "Assertion 6 - Citation completeness",
        "Assertion 7 - PII redaction",
        "Assertion 8 - Override anomaly flagging",
        "Assertion 9 - Clean-run approval",
        "BidScopePayload.sideEffectCap",
        "SubmissionResult.confirmation_ref",
        "DraftSaveResult.portal_draft_ref",
        "flagged",
        "outage",
    ] {
        ensure!(
            overlay.skill_markdown.contains(required),
            "bid portal validator overlay missing `{required}`"
        );
    }

    Ok(())
}
