#![forbid(unsafe_code)]

#[path = "../src/bid_workflow/state_machine.rs"]
#[allow(dead_code)]
mod state_machine;
#[path = "../src/bid_workflow/types.rs"]
#[allow(dead_code)]
mod types;

use state_machine::{
    BidStateMachine, BidStateTransitionRejection, TransitionContext, TransitionTrigger,
    ValidatorGate, allowed_transitions,
};
use types::{BidWorkflowState, SideEffectClass};

#[allow(clippy::needless_pass_by_value)]
fn assert_transition(from: BidWorkflowState, to: BidWorkflowState, context: TransitionContext) {
    let mut machine = BidStateMachine::new(from);

    let state = machine
        .transition_to(to, &context)
        .expect("transition should pass");

    assert_eq!(state, to);
    assert_eq!(machine.state(), to);
}

#[allow(clippy::needless_pass_by_value)]
fn assert_rejected(
    from: BidWorkflowState,
    to: BidWorkflowState,
    context: TransitionContext,
    reason: BidStateTransitionRejection,
) {
    let mut machine = BidStateMachine::new(from);

    let error = machine
        .transition_to(to, &context)
        .expect_err("transition should fail");

    assert_eq!(error.from(), from);
    assert_eq!(error.to(), to);
    assert_eq!(error.reason(), reason);
    assert_eq!(error.code(), reason.as_str());
    assert_eq!(machine.state(), from);
}

fn satisfied(from: BidWorkflowState, to: BidWorkflowState) -> TransitionContext {
    TransitionContext::satisfied_for(from, to)
}

#[test]
fn l6_t_bid_workflow_states_002_not_started_to_ready_requires_green_readiness() {
    assert_eq!(
        allowed_transitions(BidWorkflowState::NotStarted),
        &[BidWorkflowState::ReadyToBid]
    );
    assert_transition(
        BidWorkflowState::NotStarted,
        BidWorkflowState::ReadyToBid,
        satisfied(BidWorkflowState::NotStarted, BidWorkflowState::ReadyToBid),
    );
}

#[test]
fn l6_t_bid_workflow_states_003_not_started_to_ready_rejects_readiness_blocker() {
    let mut context = satisfied(BidWorkflowState::NotStarted, BidWorkflowState::ReadyToBid);
    context.readiness_passed = false;

    assert_rejected(
        BidWorkflowState::NotStarted,
        BidWorkflowState::ReadyToBid,
        context,
        BidStateTransitionRejection::ReadinessGateFailed,
    );
}

#[test]
fn l6_t_bid_workflow_states_004_ready_to_in_progress_requires_scope_and_session() {
    assert_transition(
        BidWorkflowState::ReadyToBid,
        BidWorkflowState::BidInProgress,
        satisfied(
            BidWorkflowState::ReadyToBid,
            BidWorkflowState::BidInProgress,
        ),
    );
}

#[test]
fn l6_t_bid_workflow_states_005_ready_to_in_progress_rejects_missing_scope() {
    let mut context = satisfied(
        BidWorkflowState::ReadyToBid,
        BidWorkflowState::BidInProgress,
    );
    context.bid_scope_defined = false;

    assert_rejected(
        BidWorkflowState::ReadyToBid,
        BidWorkflowState::BidInProgress,
        context,
        BidStateTransitionRejection::BidScopeMissing,
    );
}

#[test]
fn l6_t_bid_workflow_states_006_ready_to_not_started_allows_prerequisite_regression() {
    assert_transition(
        BidWorkflowState::ReadyToBid,
        BidWorkflowState::NotStarted,
        satisfied(BidWorkflowState::ReadyToBid, BidWorkflowState::NotStarted),
    );
}

#[test]
fn l6_t_bid_workflow_states_007_ready_to_not_started_rejects_active_remote_work() {
    let mut context = satisfied(BidWorkflowState::ReadyToBid, BidWorkflowState::NotStarted);
    context.active_bid_job = true;

    assert_rejected(
        BidWorkflowState::ReadyToBid,
        BidWorkflowState::NotStarted,
        context,
        BidStateTransitionRejection::ActiveBidJobOrRemoteSideEffect,
    );
}

#[test]
fn l6_t_bid_workflow_states_008_in_progress_to_draft_saved_requires_draft_evidence() {
    assert_transition(
        BidWorkflowState::BidInProgress,
        BidWorkflowState::DraftSaved,
        satisfied(
            BidWorkflowState::BidInProgress,
            BidWorkflowState::DraftSaved,
        ),
    );
}

#[test]
fn l6_t_bid_workflow_states_009_in_progress_to_draft_saved_rejects_missing_evidence() {
    let mut context = satisfied(
        BidWorkflowState::BidInProgress,
        BidWorkflowState::DraftSaved,
    );
    context.draft_evidence_recorded = false;

    assert_rejected(
        BidWorkflowState::BidInProgress,
        BidWorkflowState::DraftSaved,
        context,
        BidStateTransitionRejection::DraftEvidenceMissing,
    );
}

#[test]
fn l6_t_bid_workflow_states_010_in_progress_to_submitted_requires_validation_and_cap() {
    assert_transition(
        BidWorkflowState::BidInProgress,
        BidWorkflowState::BidSubmitted,
        satisfied(
            BidWorkflowState::BidInProgress,
            BidWorkflowState::BidSubmitted,
        ),
    );
}

#[test]
fn l6_t_bid_workflow_states_011_in_progress_to_submitted_rejects_final_gate_failures() {
    let from = BidWorkflowState::BidInProgress;
    let to = BidWorkflowState::BidSubmitted;
    let mut stale = satisfied(from, to);
    stale.decision_basis_current = false;
    assert_rejected(
        from,
        to,
        stale,
        BidStateTransitionRejection::DecisionBasisStale,
    );

    let mut validator_rejected = satisfied(from, to);
    validator_rejected.validator_gate = ValidatorGate::Rejected;
    assert_rejected(
        from,
        to,
        validator_rejected,
        BidStateTransitionRejection::ValidationGateBlocked,
    );

    let mut cap_breach = satisfied(from, to);
    cap_breach.side_effect_cap = SideEffectClass::RemoteDraft;
    assert_rejected(
        from,
        to,
        cap_breach,
        BidStateTransitionRejection::SideEffectCapExceeded,
    );
}

#[test]
fn l6_t_bid_workflow_states_012_in_progress_to_failed_records_pre_mutation_failure() {
    assert_transition(
        BidWorkflowState::BidInProgress,
        BidWorkflowState::BidFailed,
        satisfied(BidWorkflowState::BidInProgress, BidWorkflowState::BidFailed),
    );
}

#[test]
fn l6_t_bid_workflow_states_013_in_progress_to_failed_rejects_remote_draft_side_effect() {
    let mut context = satisfied(BidWorkflowState::BidInProgress, BidWorkflowState::BidFailed);
    context.side_effect_reached = Some(SideEffectClass::RemoteDraft);

    assert_rejected(
        BidWorkflowState::BidInProgress,
        BidWorkflowState::BidFailed,
        context,
        BidStateTransitionRejection::RemoteDraftSideEffectRequiresPortalStateUnknown,
    );
}

#[test]
fn l6_t_bid_workflow_states_014_in_progress_to_unknown_requires_remote_mutation_risk() {
    assert_transition(
        BidWorkflowState::BidInProgress,
        BidWorkflowState::PortalStateUnknown,
        satisfied(
            BidWorkflowState::BidInProgress,
            BidWorkflowState::PortalStateUnknown,
        ),
    );
}

#[test]
fn l6_t_bid_workflow_states_015_in_progress_to_unknown_rejects_no_remote_risk() {
    let mut context = satisfied(
        BidWorkflowState::BidInProgress,
        BidWorkflowState::PortalStateUnknown,
    );
    context.side_effect_reached = None;
    context.portal_evidence_ambiguous = false;

    assert_rejected(
        BidWorkflowState::BidInProgress,
        BidWorkflowState::PortalStateUnknown,
        context,
        BidStateTransitionRejection::RemoteMutationMissing,
    );
}

#[test]
fn l6_t_bid_workflow_states_016_draft_saved_to_submitted_rechecks_readiness_and_validation() {
    assert_transition(
        BidWorkflowState::DraftSaved,
        BidWorkflowState::BidSubmitted,
        satisfied(BidWorkflowState::DraftSaved, BidWorkflowState::BidSubmitted),
    );
}

#[test]
fn l6_t_bid_workflow_states_017_draft_saved_to_submitted_rejects_final_gate_failures() {
    let from = BidWorkflowState::DraftSaved;
    let to = BidWorkflowState::BidSubmitted;
    let mut stale = satisfied(from, to);
    stale.decision_basis_current = false;
    assert_rejected(
        from,
        to,
        stale,
        BidStateTransitionRejection::DecisionBasisStale,
    );

    let mut validator_outage = satisfied(from, to);
    validator_outage.validator_gate = ValidatorGate::Outage;
    assert_rejected(
        from,
        to,
        validator_outage,
        BidStateTransitionRejection::ValidationGateBlocked,
    );

    let mut override_logged = satisfied(from, to);
    override_logged.validator_gate = ValidatorGate::Outage;
    override_logged.validation_override_logged = true;
    assert_transition(from, to, override_logged);
}

#[test]
fn l6_t_bid_workflow_states_018_draft_saved_to_in_progress_requires_verified_draft() {
    assert_transition(
        BidWorkflowState::DraftSaved,
        BidWorkflowState::BidInProgress,
        satisfied(
            BidWorkflowState::DraftSaved,
            BidWorkflowState::BidInProgress,
        ),
    );
}

#[test]
fn l6_t_bid_workflow_states_019_draft_saved_to_in_progress_rejects_unverified_draft() {
    let mut context = satisfied(
        BidWorkflowState::DraftSaved,
        BidWorkflowState::BidInProgress,
    );
    context.portal_state_verified = false;

    assert_rejected(
        BidWorkflowState::DraftSaved,
        BidWorkflowState::BidInProgress,
        context,
        BidStateTransitionRejection::PortalVerificationMissing,
    );
}

#[test]
fn l6_t_bid_workflow_states_020_draft_saved_to_failed_records_operator_abandonment() {
    assert_transition(
        BidWorkflowState::DraftSaved,
        BidWorkflowState::BidFailed,
        satisfied(BidWorkflowState::DraftSaved, BidWorkflowState::BidFailed),
    );
}

#[test]
fn l6_t_bid_workflow_states_021_draft_saved_to_failed_rejects_unsafe_retry_failure() {
    let mut context = satisfied(BidWorkflowState::DraftSaved, BidWorkflowState::BidFailed);
    context.trigger = TransitionTrigger::Failure;
    context.remote_state_harmless = false;

    assert_rejected(
        BidWorkflowState::DraftSaved,
        BidWorkflowState::BidFailed,
        context,
        BidStateTransitionRejection::RemoteMutationUnresolved,
    );
}

#[test]
fn l6_t_bid_workflow_states_022_unknown_to_in_progress_requires_reconciliation() {
    assert_transition(
        BidWorkflowState::PortalStateUnknown,
        BidWorkflowState::BidInProgress,
        satisfied(
            BidWorkflowState::PortalStateUnknown,
            BidWorkflowState::BidInProgress,
        ),
    );
}

#[test]
fn l6_t_bid_workflow_states_023_unknown_to_in_progress_rejects_missing_reconciliation() {
    let mut context = satisfied(
        BidWorkflowState::PortalStateUnknown,
        BidWorkflowState::BidInProgress,
    );
    context.operator_reconciled = false;

    assert_rejected(
        BidWorkflowState::PortalStateUnknown,
        BidWorkflowState::BidInProgress,
        context,
        BidStateTransitionRejection::ReconciliationMissing,
    );
}

#[test]
fn l6_t_bid_workflow_states_024_unknown_to_submitted_requires_manual_confirmation() {
    assert_transition(
        BidWorkflowState::PortalStateUnknown,
        BidWorkflowState::BidSubmitted,
        satisfied(
            BidWorkflowState::PortalStateUnknown,
            BidWorkflowState::BidSubmitted,
        ),
    );
}

#[test]
fn l6_t_bid_workflow_states_025_unknown_to_submitted_rejects_missing_confirmation_ref() {
    let mut context = satisfied(
        BidWorkflowState::PortalStateUnknown,
        BidWorkflowState::BidSubmitted,
    );
    context.confirmation_ref = None;

    assert_rejected(
        BidWorkflowState::PortalStateUnknown,
        BidWorkflowState::BidSubmitted,
        context,
        BidStateTransitionRejection::SubmissionConfirmationMissing,
    );
}

#[test]
fn l6_t_bid_workflow_states_026_unknown_to_failed_requires_unrecoverable_evidence() {
    assert_transition(
        BidWorkflowState::PortalStateUnknown,
        BidWorkflowState::BidFailed,
        satisfied(
            BidWorkflowState::PortalStateUnknown,
            BidWorkflowState::BidFailed,
        ),
    );
}

#[test]
fn l6_t_bid_workflow_states_027_unknown_to_failed_rejects_ambiguous_portal_evidence() {
    let mut context = satisfied(
        BidWorkflowState::PortalStateUnknown,
        BidWorkflowState::BidFailed,
    );
    context.portal_evidence_ambiguous = true;

    assert_rejected(
        BidWorkflowState::PortalStateUnknown,
        BidWorkflowState::BidFailed,
        context,
        BidStateTransitionRejection::PortalEvidenceAmbiguous,
    );
}

#[test]
fn l6_t_bid_workflow_states_028_failed_to_ready_requires_retry_review() {
    assert_transition(
        BidWorkflowState::BidFailed,
        BidWorkflowState::ReadyToBid,
        satisfied(BidWorkflowState::BidFailed, BidWorkflowState::ReadyToBid),
    );
}

#[test]
fn l6_t_bid_workflow_states_029_failed_to_ready_rejects_unreviewed_failure() {
    let mut context = satisfied(BidWorkflowState::BidFailed, BidWorkflowState::ReadyToBid);
    context.previous_failure_reviewed = false;

    assert_rejected(
        BidWorkflowState::BidFailed,
        BidWorkflowState::ReadyToBid,
        context,
        BidStateTransitionRejection::PreviousFailureNotReviewed,
    );
}
