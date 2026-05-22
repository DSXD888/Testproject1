use serde::{Deserialize, Serialize};

use super::types::BidScopePayload;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BidReadinessBlocker {
    Phase3Pending,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BidReadinessOverrideKind {
    QuoteRoleQuality,
    ValidationGate,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BidReadinessGateResult {
    pub approved: bool,
    pub blockers: Vec<BidReadinessBlocker>,
    pub override_required: Option<BidReadinessOverrideKind>,
}

#[must_use]
pub fn evaluate_bid_readiness(_scope: &BidScopePayload) -> BidReadinessGateResult {
    BidReadinessGateResult {
        approved: false,
        blockers: vec![BidReadinessBlocker::Phase3Pending],
        override_required: None,
    }
}
