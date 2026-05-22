use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::str::FromStr;

use super::types::{BidStateValidationError, BidWorkflowState};

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
        _transition: BidTransition,
    ) -> Result<BidWorkflowState, BidStateTransitionError> {
        Err(BidStateTransitionError::phase3_pending())
    }
}

#[cfg(test)]
mod tests {
    use super::{BidStateMachine, BidWorkflowState};

    #[test]
    fn l6_t_bid_workflow_states_001_state_machine_entry_validates_lifecycle_strings() {
        let machine =
            BidStateMachine::from_state_str("ready_to_bid").expect("valid lifecycle state");

        assert_eq!(machine.state(), BidWorkflowState::ReadyToBid);
        assert!(BidStateMachine::from_state_str("READY_TO_BID").is_err());
        assert!(BidStateMachine::from_state_str("").is_err());
    }
}
