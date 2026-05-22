use std::error::Error;
use std::fmt::{self, Display, Formatter};

use super::types::BidWorkflowState;

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BidStateTransitionError {
    message: String,
}

impl BidStateTransitionError {
    #[must_use]
    pub fn phase3_pending() -> Self {
        Self {
            message: "Phase 3 implements bid workflow state transitions".to_string(),
        }
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

    #[must_use]
    pub const fn state(&self) -> BidWorkflowState {
        self.state
    }

    #[allow(clippy::needless_pass_by_ref_mut, clippy::unused_self)]
    pub fn transition(
        &mut self,
        _transition: BidTransition,
    ) -> Result<BidWorkflowState, BidStateTransitionError> {
        Err(BidStateTransitionError::phase3_pending())
    }
}
