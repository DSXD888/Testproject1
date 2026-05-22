use crate::bid_workflow::types::SideEffectClass;
use crate::errors::manifest::ErrorCode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapDecision {
    Allowed,
    SaveDraftAndStop { reason: ErrorCode },
    RouteToPortalStateUnknown { reason: ErrorCode },
}

impl CapDecision {
    #[must_use]
    pub const fn is_allowed(self) -> bool {
        matches!(self, Self::Allowed)
    }

    #[must_use]
    pub const fn reason(self) -> Option<ErrorCode> {
        match self {
            Self::Allowed => None,
            Self::SaveDraftAndStop { reason } | Self::RouteToPortalStateUnknown { reason } => {
                Some(reason)
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BidPortalStep {
    NavigateToLogin,
    VerifyAuthenticatedState,
    SearchOpportunity,
    VerifyOpportunityState,
    OpenBidResponseForm,
    FillCompanyDetails,
    FillLotSelectionAndPricing,
    UploadRequiredDocuments,
    ReviewSubmissionSummary,
    SaveDraft,
    SubmitExpressionOfInterest,
    CaptureConfirmationPage,
}

impl BidPortalStep {
    pub const PROCONTRACT_READ_ONLY_PREFIX: [Self; 5] = [
        Self::NavigateToLogin,
        Self::VerifyAuthenticatedState,
        Self::SearchOpportunity,
        Self::VerifyOpportunityState,
        Self::OpenBidResponseForm,
    ];

    pub const PROCONTRACT_REMOTE_DRAFT_BODY: [Self; 3] = [
        Self::FillCompanyDetails,
        Self::FillLotSelectionAndPricing,
        Self::UploadRequiredDocuments,
    ];

    #[must_use]
    pub const fn step_id(self) -> &'static str {
        match self {
            Self::NavigateToLogin => "L6-FR-1.7.7",
            Self::VerifyAuthenticatedState => "L6-FR-1.7.8",
            Self::SearchOpportunity => "L6-FR-1.7.9",
            Self::VerifyOpportunityState => "L6-FR-1.7.10",
            Self::OpenBidResponseForm => "L6-FR-1.7.11",
            Self::FillCompanyDetails => "L6-FR-1.7.12",
            Self::FillLotSelectionAndPricing => "L6-FR-1.7.13",
            Self::UploadRequiredDocuments => "L6-FR-1.7.14",
            Self::ReviewSubmissionSummary => "L6-FR-1.7.15",
            Self::SaveDraft => "L6-FR-1.7.16",
            Self::SubmitExpressionOfInterest => "L6-FR-1.7.17",
            Self::CaptureConfirmationPage => "L6-FR-1.7.18",
        }
    }

    #[must_use]
    pub const fn side_effect_class(self) -> SideEffectClass {
        match self {
            Self::NavigateToLogin
            | Self::VerifyAuthenticatedState
            | Self::SearchOpportunity
            | Self::VerifyOpportunityState
            | Self::OpenBidResponseForm
            | Self::ReviewSubmissionSummary
            | Self::CaptureConfirmationPage => SideEffectClass::ReadOnly,
            Self::FillCompanyDetails
            | Self::FillLotSelectionAndPricing
            | Self::UploadRequiredDocuments
            | Self::SaveDraft => SideEffectClass::RemoteDraft,
            Self::SubmitExpressionOfInterest => SideEffectClass::FormalSubmission,
        }
    }
}

#[must_use]
pub fn enforce_cap(next: SideEffectClass, cap: SideEffectClass) -> CapDecision {
    if next <= cap {
        CapDecision::Allowed
    } else if requires_human_reconciliation(next) {
        CapDecision::RouteToPortalStateUnknown {
            reason: ErrorCode::BidSideEffectCapExceeded,
        }
    } else {
        CapDecision::SaveDraftAndStop {
            reason: ErrorCode::BidSideEffectCapExceeded,
        }
    }
}

#[must_use]
pub fn enforce_procontract_step_cap(step: BidPortalStep, cap: SideEffectClass) -> CapDecision {
    enforce_cap(step.side_effect_class(), cap)
}

#[must_use]
pub fn enforce_procontract_observed_side_effect(
    observed: SideEffectClass,
    cap: SideEffectClass,
) -> CapDecision {
    if requires_human_reconciliation(observed) {
        CapDecision::RouteToPortalStateUnknown {
            reason: ErrorCode::BidSideEffectCapExceeded,
        }
    } else {
        enforce_cap(observed, cap)
    }
}

#[must_use]
pub fn planned_procontract_steps(
    review_required: bool,
    cap: SideEffectClass,
) -> Vec<BidPortalStep> {
    let mut steps = Vec::with_capacity(11);
    steps.extend(BidPortalStep::PROCONTRACT_READ_ONLY_PREFIX);
    steps.extend(BidPortalStep::PROCONTRACT_REMOTE_DRAFT_BODY);
    steps.push(BidPortalStep::ReviewSubmissionSummary);

    if review_required || cap < SideEffectClass::FormalSubmission {
        steps.push(BidPortalStep::SaveDraft);
    } else {
        steps.push(BidPortalStep::SubmitExpressionOfInterest);
        steps.push(BidPortalStep::CaptureConfirmationPage);
    }

    steps
}

#[must_use]
pub const fn requires_human_reconciliation(side_effect: SideEffectClass) -> bool {
    matches!(
        side_effect,
        SideEffectClass::NotificationOrLock
            | SideEffectClass::LegalDeclaration
            | SideEffectClass::FundsCommitted
    )
}
