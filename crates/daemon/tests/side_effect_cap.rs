#![forbid(unsafe_code)]

#[allow(dead_code, clippy::missing_const_for_fn)]
#[path = "../src/errors/manifest.rs"]
mod errors_manifest_impl;
#[allow(dead_code, clippy::missing_const_for_fn)]
#[path = "../src/errors/markdown.rs"]
mod errors_markdown_impl;

mod errors {
    pub mod manifest {
        pub use crate::errors_manifest_impl::*;
    }
}

#[allow(dead_code)]
#[path = "../src/bid_workflow/types.rs"]
mod types_impl;

mod bid_workflow {
    pub mod types {
        pub use crate::types_impl::*;
    }
}

#[allow(dead_code)]
#[path = "../src/bid_workflow/side_effect_guard.rs"]
mod side_effect_guard;

use bid_workflow::types::SideEffectClass;
use errors_manifest_impl::ErrorCode;
use side_effect_guard::{
    BidPortalStep, CapDecision, enforce_cap, enforce_procontract_observed_side_effect,
    enforce_procontract_step_cap, planned_procontract_steps,
};

fn assert_stop_with_cap_error(decision: CapDecision) {
    assert_eq!(decision.reason(), Some(ErrorCode::BidSideEffectCapExceeded));
    assert!(!decision.is_allowed());
}

#[test]
fn l6_t_portal_side_effects_001_side_effect_class_order_is_stable() {
    assert!(SideEffectClass::ReadOnly < SideEffectClass::RemoteDraft);
    assert!(SideEffectClass::RemoteDraft < SideEffectClass::NotificationOrLock);
    assert!(SideEffectClass::NotificationOrLock < SideEffectClass::LegalDeclaration);
    assert!(SideEffectClass::LegalDeclaration < SideEffectClass::FormalSubmission);
    assert!(SideEffectClass::FormalSubmission < SideEffectClass::FundsCommitted);
}

#[test]
fn l6_t_portal_side_effects_004_read_only_cap_blocks_remote_form_mutation() {
    let decision =
        enforce_procontract_step_cap(BidPortalStep::FillCompanyDetails, SideEffectClass::ReadOnly);

    assert!(matches!(decision, CapDecision::SaveDraftAndStop { .. }));
    assert_stop_with_cap_error(decision);
}

#[test]
fn l6_t_portal_side_effects_005_remote_draft_cap_allows_draft_but_blocks_submit() {
    assert_eq!(
        enforce_procontract_step_cap(BidPortalStep::SaveDraft, SideEffectClass::RemoteDraft),
        CapDecision::Allowed
    );

    let decision = enforce_procontract_step_cap(
        BidPortalStep::SubmitExpressionOfInterest,
        SideEffectClass::RemoteDraft,
    );

    assert!(matches!(decision, CapDecision::SaveDraftAndStop { .. }));
    assert_stop_with_cap_error(decision);
}

#[test]
fn l6_t_portal_side_effects_006_formal_submission_cap_blocks_funds_committed() {
    assert_eq!(
        enforce_cap(
            SideEffectClass::FormalSubmission,
            SideEffectClass::FormalSubmission,
        ),
        CapDecision::Allowed
    );

    let decision = enforce_procontract_observed_side_effect(
        SideEffectClass::FundsCommitted,
        SideEffectClass::FormalSubmission,
    );

    assert!(matches!(
        decision,
        CapDecision::RouteToPortalStateUnknown { .. }
    ));
    assert_stop_with_cap_error(decision);
}

#[test]
fn l6_t_portal_side_effects_007_unexpected_reconciliation_classes_route_unknown() {
    for observed in [
        SideEffectClass::NotificationOrLock,
        SideEffectClass::LegalDeclaration,
        SideEffectClass::FundsCommitted,
    ] {
        let decision =
            enforce_procontract_observed_side_effect(observed, SideEffectClass::FundsCommitted);

        assert!(matches!(
            decision,
            CapDecision::RouteToPortalStateUnknown { .. }
        ));
        assert_stop_with_cap_error(decision);
    }
}

#[test]
fn l6_t_portal_side_effects_005_default_remote_draft_plan_never_reaches_submit() {
    let steps = planned_procontract_steps(true, SideEffectClass::RemoteDraft);

    assert!(steps.contains(&BidPortalStep::SaveDraft));
    assert!(!steps.contains(&BidPortalStep::SubmitExpressionOfInterest));
    assert!(steps.into_iter().all(|step| {
        enforce_procontract_step_cap(step, SideEffectClass::RemoteDraft).is_allowed()
    }));
}

#[test]
fn l6_t_portal_side_effects_006_authorized_formal_submission_plan_reaches_confirmation() {
    let steps = planned_procontract_steps(false, SideEffectClass::FormalSubmission);

    assert!(steps.contains(&BidPortalStep::SubmitExpressionOfInterest));
    assert!(steps.contains(&BidPortalStep::CaptureConfirmationPage));
    assert!(steps.into_iter().all(|step| {
        enforce_procontract_step_cap(step, SideEffectClass::FormalSubmission).is_allowed()
    }));
}
