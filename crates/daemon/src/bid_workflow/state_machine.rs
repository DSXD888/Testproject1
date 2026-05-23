use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::str::FromStr;

use super::types::{BidStateValidationError, BidWorkflowState, SideEffectClass};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BidTransition {
    Start,
    MarkReady,
    BeginPortalWork,
    SaveDraft,
    Submit,
    Fail,
    MarkPortalStateUnknown,
    ReconcilePortalState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransitionTrigger {
    Approval,
    StartBid,
    DaemonContinuation,
    PrerequisiteRegression,
    Withdraw,
    DraftSaved,
    SubmissionCompleted,
    Failure,
    Cancel,
    PortalAmbiguity,
    FinalSubmit,
    Resume,
    Abandon,
    Retry,
    ManualConfirmation,
    MarkUnrecoverable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidatorGate {
    NotRequired,
    Approved,
    Rejected,
    Outage,
    Missing,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperatorOverrideRecord {
    pub kind: String,
    pub reason: String,
    pub recorded_by: String,
    pub recorded_at: String,
}

impl OperatorOverrideRecord {
    #[must_use]
    pub fn validation_gate(
        reason: impl Into<String>,
        recorded_by: impl Into<String>,
        recorded_at: impl Into<String>,
    ) -> Self {
        Self {
            kind: "validation_gate".to_owned(),
            reason: reason.into(),
            recorded_by: recorded_by.into(),
            recorded_at: recorded_at.into(),
        }
    }

    #[must_use]
    pub fn is_structured_validation_gate(&self) -> bool {
        self.kind == "validation_gate"
            && !self.reason.trim().is_empty()
            && !self.recorded_by.trim().is_empty()
            && !self.recorded_at.trim().is_empty()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BidStateTransitionRejection {
    IllegalTransition,
    WrongTrigger,
    ReadinessGateFailed,
    DecisionBasisStale,
    BidScopeMissing,
    AdapterSessionMissing,
    ActiveBidJobOrRemoteSideEffect,
    DraftEvidenceMissing,
    PortalVerificationMissing,
    ScreenshotTrailMissing,
    ManifestMissing,
    SideEffectCapExceeded,
    SubmissionConfirmationMissing,
    ValidationGateBlocked,
    RemoteDraftSideEffectRequiresPortalStateUnknown,
    RemoteMutationMissing,
    FailureEvidenceMissing,
    ReconciliationMissing,
    RetryNotAuthorized,
    PreviousFailureNotReviewed,
    RemoteMutationUnresolved,
    PortalEvidenceAmbiguous,
    UnrecoverableEvidenceMissing,
}

impl BidStateTransitionRejection {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::IllegalTransition => "BID_ILLEGAL_TRANSITION",
            Self::WrongTrigger => "BID_WRONG_TRANSITION_TRIGGER",
            Self::ReadinessGateFailed => "BID_READINESS_GATE_FAILED",
            Self::DecisionBasisStale => "BID_DECISION_BASIS_STALE",
            Self::BidScopeMissing => "BID_SCOPE_MISSING",
            Self::AdapterSessionMissing => "BID_ADAPTER_SESSION_MISSING",
            Self::ActiveBidJobOrRemoteSideEffect => "BID_ACTIVE_JOB_OR_REMOTE_SIDE_EFFECT",
            Self::DraftEvidenceMissing => "BID_DRAFT_EVIDENCE_MISSING",
            Self::PortalVerificationMissing => "BID_PORTAL_VERIFICATION_MISSING",
            Self::ScreenshotTrailMissing => "BID_SCREENSHOT_TRAIL_MISSING",
            Self::ManifestMissing => "BID_RUN_MANIFEST_MISSING",
            Self::SideEffectCapExceeded => "BID_SIDE_EFFECT_CAP_EXCEEDED",
            Self::SubmissionConfirmationMissing => "BID_SUBMISSION_CONFIRMATION_MISSING",
            Self::ValidationGateBlocked => "BID_VALIDATION_GATE_BLOCKED",
            Self::RemoteDraftSideEffectRequiresPortalStateUnknown => {
                "BID_REMOTE_DRAFT_REQUIRES_PORTAL_STATE_UNKNOWN"
            }
            Self::RemoteMutationMissing => "BID_REMOTE_MUTATION_MISSING",
            Self::FailureEvidenceMissing => "BID_FAILURE_EVIDENCE_MISSING",
            Self::ReconciliationMissing => "BID_RECONCILIATION_MISSING",
            Self::RetryNotAuthorized => "BID_RETRY_NOT_AUTHORIZED",
            Self::PreviousFailureNotReviewed => "BID_PREVIOUS_FAILURE_NOT_REVIEWED",
            Self::RemoteMutationUnresolved => "BID_REMOTE_MUTATION_UNRESOLVED",
            Self::PortalEvidenceAmbiguous => "BID_PORTAL_EVIDENCE_AMBIGUOUS",
            Self::UnrecoverableEvidenceMissing => "BID_UNRECOVERABLE_EVIDENCE_MISSING",
        }
    }
}

#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransitionContext {
    pub trigger: TransitionTrigger,
    pub readiness_passed: bool,
    pub decision_basis_current: bool,
    pub bid_scope_defined: bool,
    pub adapter_session_created: bool,
    pub active_bid_job: bool,
    pub side_effect_reached: Option<SideEffectClass>,
    pub side_effect_cap: SideEffectClass,
    pub draft_evidence_recorded: bool,
    pub screenshots_recorded: bool,
    pub run_manifest_linked: bool,
    pub portal_state_verified: bool,
    pub submission_confirmed: bool,
    pub confirmation_ref: Option<String>,
    pub validator_gate: ValidatorGate,
    pub validation_override_logged: bool,
    pub validation_override: Option<OperatorOverrideRecord>,
    pub error_manifest_recorded: bool,
    pub operator_reconciled: bool,
    pub retry_authorized: bool,
    pub previous_failure_reviewed: bool,
    pub remote_state_harmless: bool,
    pub portal_evidence_ambiguous: bool,
    pub unrecoverable_confirmed: bool,
}

impl TransitionContext {
    #[must_use]
    pub const fn empty(trigger: TransitionTrigger) -> Self {
        Self {
            trigger,
            readiness_passed: false,
            decision_basis_current: false,
            bid_scope_defined: false,
            adapter_session_created: false,
            active_bid_job: false,
            side_effect_reached: None,
            side_effect_cap: SideEffectClass::ReadOnly,
            draft_evidence_recorded: false,
            screenshots_recorded: false,
            run_manifest_linked: false,
            portal_state_verified: false,
            submission_confirmed: false,
            confirmation_ref: None,
            validator_gate: ValidatorGate::Missing,
            validation_override_logged: false,
            validation_override: None,
            error_manifest_recorded: false,
            operator_reconciled: false,
            retry_authorized: false,
            previous_failure_reviewed: false,
            remote_state_harmless: false,
            portal_evidence_ambiguous: false,
            unrecoverable_confirmed: false,
        }
    }

    #[must_use]
    pub fn satisfied_for(from: BidWorkflowState, to: BidWorkflowState) -> Self {
        let mut context = Self::empty(default_trigger_for(from, to));
        context.readiness_passed = true;
        context.decision_basis_current = true;
        context.bid_scope_defined = true;
        context.adapter_session_created = true;
        context.side_effect_cap = SideEffectClass::FormalSubmission;
        context.draft_evidence_recorded = true;
        context.screenshots_recorded = true;
        context.run_manifest_linked = true;
        context.portal_state_verified = true;
        context.submission_confirmed = true;
        context.confirmation_ref = Some("CONF-123".to_string());
        context.validator_gate = ValidatorGate::Approved;
        context.error_manifest_recorded = true;
        context.operator_reconciled = true;
        context.retry_authorized = true;
        context.previous_failure_reviewed = true;
        context.remote_state_harmless = true;
        context.unrecoverable_confirmed = true;
        context.side_effect_reached = satisfied_side_effect_for(from, to);
        context
    }

    #[must_use]
    pub fn validation_allows_submission(&self) -> bool {
        matches!(self.validator_gate, ValidatorGate::Approved)
            || (self.validation_override_logged
                && self
                    .validation_override
                    .as_ref()
                    .is_some_and(OperatorOverrideRecord::is_structured_validation_gate))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BidStateTransitionError {
    from: BidWorkflowState,
    to: BidWorkflowState,
    reason: BidStateTransitionRejection,
    message: String,
}

impl BidStateTransitionError {
    #[must_use]
    pub fn phase3_pending() -> Self {
        Self {
            from: BidWorkflowState::NotStarted,
            to: BidWorkflowState::NotStarted,
            reason: BidStateTransitionRejection::IllegalTransition,
            message: "Phase 3 implements bid workflow state transitions".to_string(),
        }
    }

    #[must_use]
    pub fn rejected(
        from: BidWorkflowState,
        to: BidWorkflowState,
        reason: BidStateTransitionRejection,
    ) -> Self {
        Self {
            from,
            to,
            reason,
            message: format!(
                "bid workflow transition {from} -> {to} rejected: {}",
                reason.as_str()
            ),
        }
    }

    #[must_use]
    pub const fn from(&self) -> BidWorkflowState {
        self.from
    }

    #[must_use]
    pub const fn to(&self) -> BidWorkflowState {
        self.to
    }

    #[must_use]
    pub const fn reason(&self) -> BidStateTransitionRejection {
        self.reason
    }

    #[must_use]
    pub const fn code(&self) -> &'static str {
        self.reason.as_str()
    }
}

impl Display for BidStateTransitionError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl Error for BidStateTransitionError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BidStateMachine {
    state: BidWorkflowState,
}

impl BidStateMachine {
    #[must_use]
    pub const fn new(state: BidWorkflowState) -> Self {
        Self { state }
    }

    pub fn from_state_str(state: &str) -> Result<Self, BidStateValidationError> {
        BidWorkflowState::from_str(state).map(Self::new)
    }

    #[must_use]
    pub const fn state(&self) -> BidWorkflowState {
        self.state
    }

    #[allow(clippy::needless_pass_by_ref_mut, clippy::unused_self)]
    pub fn transition(
        &mut self,
        transition: BidTransition,
    ) -> Result<BidWorkflowState, BidStateTransitionError> {
        let target = transition_target(self.state, transition)?;
        let context = TransitionContext::satisfied_for(self.state, target);
        self.transition_to(target, &context)
    }

    pub fn transition_to(
        &mut self,
        target: BidWorkflowState,
        context: &TransitionContext,
    ) -> Result<BidWorkflowState, BidStateTransitionError> {
        validate_transition(self.state, target, context)?;
        self.state = target;
        Ok(self.state)
    }
}

pub fn transition_to(
    machine: &mut BidStateMachine,
    target: BidWorkflowState,
    context: &TransitionContext,
) -> Result<BidWorkflowState, BidStateTransitionError> {
    machine.transition_to(target, context)
}

#[must_use]
pub const fn allowed_transitions(from: BidWorkflowState) -> &'static [BidWorkflowState] {
    match from {
        BidWorkflowState::NotStarted | BidWorkflowState::BidFailed => {
            &[BidWorkflowState::ReadyToBid]
        }
        BidWorkflowState::ReadyToBid => &[
            BidWorkflowState::BidInProgress,
            BidWorkflowState::NotStarted,
        ],
        BidWorkflowState::BidInProgress => &[
            BidWorkflowState::DraftSaved,
            BidWorkflowState::BidSubmitted,
            BidWorkflowState::BidFailed,
            BidWorkflowState::PortalStateUnknown,
        ],
        BidWorkflowState::DraftSaved => &[
            BidWorkflowState::BidSubmitted,
            BidWorkflowState::BidInProgress,
            BidWorkflowState::BidFailed,
        ],
        BidWorkflowState::PortalStateUnknown => &[
            BidWorkflowState::BidInProgress,
            BidWorkflowState::BidSubmitted,
            BidWorkflowState::BidFailed,
        ],
        BidWorkflowState::BidSubmitted => &[],
    }
}

#[must_use]
pub fn is_allowed_transition(from: BidWorkflowState, to: BidWorkflowState) -> bool {
    allowed_transitions(from).contains(&to)
}

pub fn validate_transition(
    from: BidWorkflowState,
    to: BidWorkflowState,
    context: &TransitionContext,
) -> Result<(), BidStateTransitionError> {
    if !is_allowed_transition(from, to) {
        return reject(from, to, BidStateTransitionRejection::IllegalTransition);
    }

    match (from, to) {
        (BidWorkflowState::NotStarted, BidWorkflowState::ReadyToBid) => {
            validate_not_started_to_ready(from, to, context)
        }
        (BidWorkflowState::ReadyToBid, BidWorkflowState::BidInProgress) => {
            validate_ready_to_in_progress(from, to, context)
        }
        (BidWorkflowState::ReadyToBid, BidWorkflowState::NotStarted) => {
            validate_ready_to_not_started(from, to, context)
        }
        (BidWorkflowState::BidInProgress, BidWorkflowState::DraftSaved) => {
            validate_in_progress_to_draft_saved(from, to, context)
        }
        (
            BidWorkflowState::BidInProgress | BidWorkflowState::DraftSaved,
            BidWorkflowState::BidSubmitted,
        ) => validate_submission(from, to, context),
        (BidWorkflowState::BidInProgress, BidWorkflowState::BidFailed) => {
            validate_in_progress_to_failed(from, to, context)
        }
        (BidWorkflowState::BidInProgress, BidWorkflowState::PortalStateUnknown) => {
            validate_in_progress_to_unknown(from, to, context)
        }
        (BidWorkflowState::DraftSaved, BidWorkflowState::BidInProgress) => {
            validate_draft_saved_to_in_progress(from, to, context)
        }
        (BidWorkflowState::DraftSaved, BidWorkflowState::BidFailed) => {
            validate_draft_saved_to_failed(from, to, context)
        }
        (BidWorkflowState::PortalStateUnknown, BidWorkflowState::BidInProgress) => {
            validate_unknown_to_in_progress(from, to, context)
        }
        (BidWorkflowState::PortalStateUnknown, BidWorkflowState::BidSubmitted) => {
            validate_unknown_to_submitted(from, to, context)
        }
        (BidWorkflowState::PortalStateUnknown, BidWorkflowState::BidFailed) => {
            validate_unknown_to_failed(from, to, context)
        }
        (BidWorkflowState::BidFailed, BidWorkflowState::ReadyToBid) => {
            validate_failed_to_ready(from, to, context)
        }
        _ => Ok(()),
    }
}

fn validate_not_started_to_ready(
    from: BidWorkflowState,
    to: BidWorkflowState,
    context: &TransitionContext,
) -> Result<(), BidStateTransitionError> {
    require_trigger(from, to, context.trigger == TransitionTrigger::Approval)?;
    require_readiness_and_fresh_basis(from, to, context)
}

fn validate_ready_to_in_progress(
    from: BidWorkflowState,
    to: BidWorkflowState,
    context: &TransitionContext,
) -> Result<(), BidStateTransitionError> {
    require_trigger(
        from,
        to,
        matches!(
            context.trigger,
            TransitionTrigger::StartBid | TransitionTrigger::DaemonContinuation
        ),
    )?;
    require_readiness_and_fresh_basis(from, to, context)?;
    require_bid_scope_and_session(from, to, context)
}

fn validate_ready_to_not_started(
    from: BidWorkflowState,
    to: BidWorkflowState,
    context: &TransitionContext,
) -> Result<(), BidStateTransitionError> {
    require_trigger(
        from,
        to,
        matches!(
            context.trigger,
            TransitionTrigger::PrerequisiteRegression | TransitionTrigger::Withdraw
        ),
    )?;
    if context.active_bid_job || reached_remote_draft(context) {
        return reject(
            from,
            to,
            BidStateTransitionRejection::ActiveBidJobOrRemoteSideEffect,
        );
    }
    Ok(())
}

fn validate_in_progress_to_draft_saved(
    from: BidWorkflowState,
    to: BidWorkflowState,
    context: &TransitionContext,
) -> Result<(), BidStateTransitionError> {
    require_trigger(from, to, context.trigger == TransitionTrigger::DraftSaved)?;
    require_side_effect_cap(from, to, context, SideEffectClass::RemoteDraft)?;
    if !reached_remote_draft(context) {
        return reject(from, to, BidStateTransitionRejection::RemoteMutationMissing);
    }
    require_draft_audit_evidence(from, to, context)
}

fn validate_submission(
    from: BidWorkflowState,
    to: BidWorkflowState,
    context: &TransitionContext,
) -> Result<(), BidStateTransitionError> {
    require_trigger(
        from,
        to,
        matches!(
            context.trigger,
            TransitionTrigger::SubmissionCompleted | TransitionTrigger::FinalSubmit
        ),
    )?;
    require_readiness_and_fresh_basis(from, to, context)?;
    require_side_effect_cap(from, to, context, SideEffectClass::FormalSubmission)?;
    if !context.submission_confirmed
        || context
            .confirmation_ref
            .as_deref()
            .is_none_or(str::is_empty)
    {
        return reject(
            from,
            to,
            BidStateTransitionRejection::SubmissionConfirmationMissing,
        );
    }
    if !context.validation_allows_submission() {
        return reject(from, to, BidStateTransitionRejection::ValidationGateBlocked);
    }
    require_screenshots_and_manifest(from, to, context)
}

fn validate_in_progress_to_failed(
    from: BidWorkflowState,
    to: BidWorkflowState,
    context: &TransitionContext,
) -> Result<(), BidStateTransitionError> {
    require_trigger(
        from,
        to,
        matches!(
            context.trigger,
            TransitionTrigger::Failure | TransitionTrigger::Cancel
        ),
    )?;
    if reached_remote_draft(context) {
        return reject(
            from,
            to,
            BidStateTransitionRejection::RemoteDraftSideEffectRequiresPortalStateUnknown,
        );
    }
    require_failure_evidence(from, to, context)
}

fn validate_in_progress_to_unknown(
    from: BidWorkflowState,
    to: BidWorkflowState,
    context: &TransitionContext,
) -> Result<(), BidStateTransitionError> {
    require_trigger(
        from,
        to,
        matches!(
            context.trigger,
            TransitionTrigger::Failure
                | TransitionTrigger::Cancel
                | TransitionTrigger::PortalAmbiguity
        ),
    )?;
    if !reached_remote_draft(context) && !context.portal_evidence_ambiguous {
        return reject(from, to, BidStateTransitionRejection::RemoteMutationMissing);
    }
    Ok(())
}

fn validate_draft_saved_to_in_progress(
    from: BidWorkflowState,
    to: BidWorkflowState,
    context: &TransitionContext,
) -> Result<(), BidStateTransitionError> {
    require_trigger(from, to, context.trigger == TransitionTrigger::Resume)?;
    require_readiness_and_fresh_basis(from, to, context)?;
    require_bid_scope_and_session(from, to, context)?;
    if !context.portal_state_verified {
        return reject(
            from,
            to,
            BidStateTransitionRejection::PortalVerificationMissing,
        );
    }
    Ok(())
}

fn validate_draft_saved_to_failed(
    from: BidWorkflowState,
    to: BidWorkflowState,
    context: &TransitionContext,
) -> Result<(), BidStateTransitionError> {
    require_trigger(
        from,
        to,
        matches!(
            context.trigger,
            TransitionTrigger::Abandon | TransitionTrigger::Failure
        ),
    )?;
    if context.trigger == TransitionTrigger::Failure && !context.remote_state_harmless {
        return reject(
            from,
            to,
            BidStateTransitionRejection::RemoteMutationUnresolved,
        );
    }
    require_failure_evidence(from, to, context)
}

fn validate_unknown_to_in_progress(
    from: BidWorkflowState,
    to: BidWorkflowState,
    context: &TransitionContext,
) -> Result<(), BidStateTransitionError> {
    require_trigger(from, to, context.trigger == TransitionTrigger::Retry)?;
    require_operator_reconciliation(from, to, context)?;
    require_readiness_and_fresh_basis(from, to, context)?;
    require_bid_scope_and_session(from, to, context)?;
    require_retry_authorized(from, to, context)
}

fn validate_unknown_to_submitted(
    from: BidWorkflowState,
    to: BidWorkflowState,
    context: &TransitionContext,
) -> Result<(), BidStateTransitionError> {
    require_trigger(
        from,
        to,
        context.trigger == TransitionTrigger::ManualConfirmation,
    )?;
    require_operator_reconciliation(from, to, context)?;
    if !context.submission_confirmed
        || context
            .confirmation_ref
            .as_deref()
            .is_none_or(str::is_empty)
    {
        return reject(
            from,
            to,
            BidStateTransitionRejection::SubmissionConfirmationMissing,
        );
    }
    Ok(())
}

fn validate_unknown_to_failed(
    from: BidWorkflowState,
    to: BidWorkflowState,
    context: &TransitionContext,
) -> Result<(), BidStateTransitionError> {
    require_trigger(
        from,
        to,
        context.trigger == TransitionTrigger::MarkUnrecoverable,
    )?;
    require_operator_reconciliation(from, to, context)?;
    if context.portal_evidence_ambiguous {
        return reject(
            from,
            to,
            BidStateTransitionRejection::PortalEvidenceAmbiguous,
        );
    }
    if !context.unrecoverable_confirmed {
        return reject(
            from,
            to,
            BidStateTransitionRejection::UnrecoverableEvidenceMissing,
        );
    }
    require_failure_evidence(from, to, context)
}

fn validate_failed_to_ready(
    from: BidWorkflowState,
    to: BidWorkflowState,
    context: &TransitionContext,
) -> Result<(), BidStateTransitionError> {
    require_trigger(from, to, context.trigger == TransitionTrigger::Retry)?;
    require_readiness_and_fresh_basis(from, to, context)?;
    require_retry_authorized(from, to, context)?;
    if !context.previous_failure_reviewed {
        return reject(
            from,
            to,
            BidStateTransitionRejection::PreviousFailureNotReviewed,
        );
    }
    if reached_remote_draft(context) && !context.remote_state_harmless {
        return reject(
            from,
            to,
            BidStateTransitionRejection::RemoteMutationUnresolved,
        );
    }
    Ok(())
}

fn require_readiness_and_fresh_basis(
    from: BidWorkflowState,
    to: BidWorkflowState,
    context: &TransitionContext,
) -> Result<(), BidStateTransitionError> {
    if !context.readiness_passed {
        return reject(from, to, BidStateTransitionRejection::ReadinessGateFailed);
    }
    if !context.decision_basis_current {
        return reject(from, to, BidStateTransitionRejection::DecisionBasisStale);
    }
    Ok(())
}

fn require_bid_scope_and_session(
    from: BidWorkflowState,
    to: BidWorkflowState,
    context: &TransitionContext,
) -> Result<(), BidStateTransitionError> {
    if !context.bid_scope_defined {
        return reject(from, to, BidStateTransitionRejection::BidScopeMissing);
    }
    if !context.adapter_session_created {
        return reject(from, to, BidStateTransitionRejection::AdapterSessionMissing);
    }
    Ok(())
}

fn require_draft_audit_evidence(
    from: BidWorkflowState,
    to: BidWorkflowState,
    context: &TransitionContext,
) -> Result<(), BidStateTransitionError> {
    if !context.draft_evidence_recorded {
        return reject(from, to, BidStateTransitionRejection::DraftEvidenceMissing);
    }
    if !context.portal_state_verified {
        return reject(
            from,
            to,
            BidStateTransitionRejection::PortalVerificationMissing,
        );
    }
    require_screenshots_and_manifest(from, to, context)
}

fn require_screenshots_and_manifest(
    from: BidWorkflowState,
    to: BidWorkflowState,
    context: &TransitionContext,
) -> Result<(), BidStateTransitionError> {
    if !context.screenshots_recorded {
        return reject(
            from,
            to,
            BidStateTransitionRejection::ScreenshotTrailMissing,
        );
    }
    if !context.run_manifest_linked {
        return reject(from, to, BidStateTransitionRejection::ManifestMissing);
    }
    Ok(())
}

fn require_failure_evidence(
    from: BidWorkflowState,
    to: BidWorkflowState,
    context: &TransitionContext,
) -> Result<(), BidStateTransitionError> {
    if !context.error_manifest_recorded {
        return reject(
            from,
            to,
            BidStateTransitionRejection::FailureEvidenceMissing,
        );
    }
    require_screenshots_and_manifest(from, to, context)
}

fn require_operator_reconciliation(
    from: BidWorkflowState,
    to: BidWorkflowState,
    context: &TransitionContext,
) -> Result<(), BidStateTransitionError> {
    if !context.operator_reconciled || !context.portal_state_verified {
        return reject(from, to, BidStateTransitionRejection::ReconciliationMissing);
    }
    Ok(())
}

fn require_retry_authorized(
    from: BidWorkflowState,
    to: BidWorkflowState,
    context: &TransitionContext,
) -> Result<(), BidStateTransitionError> {
    if !context.retry_authorized {
        return reject(from, to, BidStateTransitionRejection::RetryNotAuthorized);
    }
    Ok(())
}

fn require_side_effect_cap(
    from: BidWorkflowState,
    to: BidWorkflowState,
    context: &TransitionContext,
    required: SideEffectClass,
) -> Result<(), BidStateTransitionError> {
    if context.side_effect_cap < required {
        return reject(from, to, BidStateTransitionRejection::SideEffectCapExceeded);
    }
    Ok(())
}

fn require_trigger(
    from: BidWorkflowState,
    to: BidWorkflowState,
    condition: bool,
) -> Result<(), BidStateTransitionError> {
    if condition {
        Ok(())
    } else {
        reject(from, to, BidStateTransitionRejection::WrongTrigger)
    }
}

fn reached_remote_draft(context: &TransitionContext) -> bool {
    context
        .side_effect_reached
        .is_some_and(|side_effect| side_effect >= SideEffectClass::RemoteDraft)
}

fn reject<T>(
    from: BidWorkflowState,
    to: BidWorkflowState,
    reason: BidStateTransitionRejection,
) -> Result<T, BidStateTransitionError> {
    Err(BidStateTransitionError::rejected(from, to, reason))
}

fn transition_target(
    from: BidWorkflowState,
    transition: BidTransition,
) -> Result<BidWorkflowState, BidStateTransitionError> {
    match (from, transition) {
        (BidWorkflowState::NotStarted, BidTransition::Start | BidTransition::MarkReady)
        | (BidWorkflowState::BidFailed, BidTransition::Start) => Ok(BidWorkflowState::ReadyToBid),
        (
            BidWorkflowState::ReadyToBid | BidWorkflowState::DraftSaved,
            BidTransition::Start
            | BidTransition::BeginPortalWork
            | BidTransition::ReconcilePortalState,
        )
        | (BidWorkflowState::PortalStateUnknown, BidTransition::ReconcilePortalState) => {
            Ok(BidWorkflowState::BidInProgress)
        }
        (BidWorkflowState::ReadyToBid, BidTransition::Fail) => Ok(BidWorkflowState::NotStarted),
        (BidWorkflowState::BidInProgress, BidTransition::SaveDraft) => {
            Ok(BidWorkflowState::DraftSaved)
        }
        (
            BidWorkflowState::BidInProgress
            | BidWorkflowState::DraftSaved
            | BidWorkflowState::PortalStateUnknown,
            BidTransition::Submit,
        ) => Ok(BidWorkflowState::BidSubmitted),
        (
            BidWorkflowState::BidInProgress
            | BidWorkflowState::DraftSaved
            | BidWorkflowState::PortalStateUnknown,
            BidTransition::Fail,
        ) => Ok(BidWorkflowState::BidFailed),
        (BidWorkflowState::BidInProgress, BidTransition::MarkPortalStateUnknown) => {
            Ok(BidWorkflowState::PortalStateUnknown)
        }
        _ => reject(from, from, BidStateTransitionRejection::IllegalTransition),
    }
}

const fn default_trigger_for(from: BidWorkflowState, to: BidWorkflowState) -> TransitionTrigger {
    match (from, to) {
        (BidWorkflowState::NotStarted, BidWorkflowState::ReadyToBid) => TransitionTrigger::Approval,
        (BidWorkflowState::ReadyToBid, BidWorkflowState::BidInProgress) => {
            TransitionTrigger::StartBid
        }
        (BidWorkflowState::ReadyToBid, BidWorkflowState::NotStarted) => {
            TransitionTrigger::PrerequisiteRegression
        }
        (BidWorkflowState::BidInProgress, BidWorkflowState::DraftSaved) => {
            TransitionTrigger::DraftSaved
        }
        (BidWorkflowState::BidInProgress, BidWorkflowState::BidSubmitted) => {
            TransitionTrigger::SubmissionCompleted
        }
        (BidWorkflowState::BidInProgress, BidWorkflowState::BidFailed) => {
            TransitionTrigger::Failure
        }
        (BidWorkflowState::BidInProgress, BidWorkflowState::PortalStateUnknown) => {
            TransitionTrigger::PortalAmbiguity
        }
        (BidWorkflowState::DraftSaved, BidWorkflowState::BidSubmitted) => {
            TransitionTrigger::FinalSubmit
        }
        (BidWorkflowState::DraftSaved, BidWorkflowState::BidInProgress) => {
            TransitionTrigger::Resume
        }
        (BidWorkflowState::DraftSaved, BidWorkflowState::BidFailed) => TransitionTrigger::Abandon,
        (BidWorkflowState::PortalStateUnknown, BidWorkflowState::BidInProgress)
        | (BidWorkflowState::BidFailed, BidWorkflowState::ReadyToBid) => TransitionTrigger::Retry,
        (BidWorkflowState::PortalStateUnknown, BidWorkflowState::BidSubmitted) => {
            TransitionTrigger::ManualConfirmation
        }
        (BidWorkflowState::PortalStateUnknown, BidWorkflowState::BidFailed) => {
            TransitionTrigger::MarkUnrecoverable
        }
        _ => TransitionTrigger::Failure,
    }
}

const fn satisfied_side_effect_for(
    from: BidWorkflowState,
    to: BidWorkflowState,
) -> Option<SideEffectClass> {
    match (from, to) {
        (
            BidWorkflowState::BidInProgress,
            BidWorkflowState::DraftSaved | BidWorkflowState::PortalStateUnknown,
        )
        | (BidWorkflowState::DraftSaved, _) => Some(SideEffectClass::RemoteDraft),
        (
            BidWorkflowState::BidInProgress | BidWorkflowState::PortalStateUnknown,
            BidWorkflowState::BidSubmitted,
        ) => Some(SideEffectClass::FormalSubmission),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        BidStateMachine, BidStateTransitionRejection, BidTransition, BidWorkflowState,
        TransitionContext, allowed_transitions, is_allowed_transition,
    };

    #[test]
    fn l6_t_bid_workflow_states_001_state_machine_entry_validates_lifecycle_strings() {
        let machine =
            BidStateMachine::from_state_str("ready_to_bid").expect("valid lifecycle state");

        assert_eq!(machine.state(), BidWorkflowState::ReadyToBid);
        assert!(BidStateMachine::from_state_str("READY_TO_BID").is_err());
        assert!(BidStateMachine::from_state_str("").is_err());
    }

    #[test]
    fn l6_t_bid_workflow_states_002_allowed_transition_table_contains_spec_pairs_only() {
        let transitions: usize = BidWorkflowState::ALL
            .into_iter()
            .map(|state| allowed_transitions(state).len())
            .sum();

        assert_eq!(transitions, 14);
        assert!(is_allowed_transition(
            BidWorkflowState::NotStarted,
            BidWorkflowState::ReadyToBid
        ));
        assert!(!is_allowed_transition(
            BidWorkflowState::BidSubmitted,
            BidWorkflowState::ReadyToBid
        ));
    }

    #[test]
    fn l6_t_bid_workflow_states_004_legacy_transition_api_reaches_bid_in_progress() {
        let mut machine = BidStateMachine::new(BidWorkflowState::ReadyToBid);

        let state = machine
            .transition(BidTransition::BeginPortalWork)
            .expect("legacy transition maps to context-checked transition");

        assert_eq!(state, BidWorkflowState::BidInProgress);
    }

    #[test]
    fn rejected_transition_does_not_mutate_state() {
        let mut machine = BidStateMachine::new(BidWorkflowState::BidInProgress);
        let mut context = TransitionContext::satisfied_for(
            BidWorkflowState::BidInProgress,
            BidWorkflowState::BidFailed,
        );
        context.side_effect_reached = Some(super::SideEffectClass::RemoteDraft);

        let error = machine
            .transition_to(BidWorkflowState::BidFailed, &context)
            .expect_err("remote draft failure must route through portal_state_unknown");

        assert_eq!(
            error.reason(),
            BidStateTransitionRejection::RemoteDraftSideEffectRequiresPortalStateUnknown
        );
        assert_eq!(machine.state(), BidWorkflowState::BidInProgress);
    }
}
