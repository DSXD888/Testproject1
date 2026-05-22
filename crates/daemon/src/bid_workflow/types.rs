use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::str::FromStr;

use serde::{Deserialize, Serialize};

const VALID_BID_WORKFLOW_STATES: &str = "not_started, ready_to_bid, bid_in_progress, draft_saved, bid_submitted, bid_failed, portal_state_unknown";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BidScopePayload {
    pub process_id: String,
    pub lot_ids: Vec<String>,
    pub candidate_ids: Vec<String>,
    pub selected_quote_ids: Vec<String>,
    pub company_profile_version: String,
    pub bid_portal_type: String,
    pub portal_opportunity_ref: String,
    pub review_required: bool,
    pub decision_basis_hash: String,
    pub side_effect_cap: SideEffectClass,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SideEffectClass {
    ReadOnly,
    RemoteDraft,
    NotificationOrLock,
    LegalDeclaration,
    FormalSubmission,
    FundsCommitted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BidDocument {
    pub document_type: String,
    pub file_name: String,
    pub file_path: String,
    pub content_hash: String,
    pub expiry_date: Option<String>,
    pub issuing_body: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BidWorkflowState {
    NotStarted,
    ReadyToBid,
    BidInProgress,
    DraftSaved,
    BidSubmitted,
    BidFailed,
    PortalStateUnknown,
}

impl BidWorkflowState {
    pub const ALL: [Self; 7] = [
        Self::NotStarted,
        Self::ReadyToBid,
        Self::BidInProgress,
        Self::DraftSaved,
        Self::BidSubmitted,
        Self::BidFailed,
        Self::PortalStateUnknown,
    ];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NotStarted => "not_started",
            Self::ReadyToBid => "ready_to_bid",
            Self::BidInProgress => "bid_in_progress",
            Self::DraftSaved => "draft_saved",
            Self::BidSubmitted => "bid_submitted",
            Self::BidFailed => "bid_failed",
            Self::PortalStateUnknown => "portal_state_unknown",
        }
    }
}

impl Display for BidWorkflowState {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for BidWorkflowState {
    type Err = BidStateValidationError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "not_started" => Ok(Self::NotStarted),
            "ready_to_bid" => Ok(Self::ReadyToBid),
            "bid_in_progress" => Ok(Self::BidInProgress),
            "draft_saved" => Ok(Self::DraftSaved),
            "bid_submitted" => Ok(Self::BidSubmitted),
            "bid_failed" => Ok(Self::BidFailed),
            "portal_state_unknown" => Ok(Self::PortalStateUnknown),
            invalid_state => Err(BidStateValidationError::unknown_state(invalid_state)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BidStateValidationError {
    invalid_state: String,
}

impl BidStateValidationError {
    #[must_use]
    pub fn unknown_state(invalid_state: impl Into<String>) -> Self {
        Self {
            invalid_state: invalid_state.into(),
        }
    }

    #[must_use]
    pub fn invalid_state(&self) -> &str {
        &self.invalid_state
    }
}

impl Display for BidStateValidationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "unknown bid workflow state `{}`; expected one of: {}",
            self.invalid_state, VALID_BID_WORKFLOW_STATES
        )
    }
}

impl Error for BidStateValidationError {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PortalStateVerification {
    pub opportunity_found: bool,
    pub existing_draft_detected: bool,
    pub existing_submission_detected: bool,
    pub portal_status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmissionResult {
    pub submitted: bool,
    pub confirmation_ref: Option<String>,
    pub final_side_effect_class: SideEffectClass,
    pub screenshot_paths: Vec<String>,
    pub portal_timestamp: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftSaveResult {
    pub draft_saved: bool,
    pub portal_draft_ref: Option<String>,
    pub side_effect_class: SideEffectClass,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompanyProfile {
    pub company_name: String,
    pub registration_number: String,
    pub vat_number: Option<String>,
    pub address: CompanyAddress,
    pub primary_contact: CompanyContact,
    pub certifications: Vec<CompanyCertification>,
    pub insurance_policies: Vec<CompanyInsurancePolicy>,
    pub financial_statements: Option<CompanyFinancialStatements>,
    pub sme_status: Option<SmeStatus>,
    pub equality_statement: Option<String>,
    pub environmental_policy: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompanyAddress {
    pub line1: String,
    pub line2: Option<String>,
    pub city: String,
    pub county: Option<String>,
    pub postcode: String,
    pub country: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompanyContact {
    pub name: String,
    pub email: String,
    pub phone: String,
    pub role: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompanyCertification {
    #[serde(rename = "type")]
    pub certification_type: String,
    pub number: String,
    pub expiry_date: String,
    pub issuing_body: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompanyInsurancePolicy {
    #[serde(rename = "type")]
    pub policy_type: String,
    pub provider: String,
    pub policy_number: String,
    pub cover_amount_pence: i64,
    pub expiry_date: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompanyFinancialStatements {
    pub last_filed_year: i32,
    pub companies_house_ref: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SmeStatus {
    Micro,
    Small,
    Medium,
    Large,
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::BidWorkflowState;

    #[test]
    fn l6_t_bid_workflow_states_001_lifecycle_enum_accepts_exactly_seven() {
        let valid_states = [
            "not_started",
            "ready_to_bid",
            "bid_in_progress",
            "draft_saved",
            "bid_submitted",
            "bid_failed",
            "portal_state_unknown",
        ];

        assert_eq!(BidWorkflowState::ALL.len(), valid_states.len());

        for (state, expected) in BidWorkflowState::ALL.into_iter().zip(valid_states) {
            let parsed = BidWorkflowState::from_str(expected).expect("valid state parses");

            assert_eq!(parsed, state);
            assert_eq!(state.as_str(), expected);
            assert_eq!(state.to_string(), expected);
        }
    }

    #[test]
    fn l6_t_bid_workflow_states_001_rejects_unknown_lifecycle_strings() {
        for invalid in ["invalid", "READY_TO_BID", "", "ready-to-bid"] {
            let error = BidWorkflowState::from_str(invalid).expect_err("invalid state rejected");

            assert_eq!(error.invalid_state(), invalid);
            assert!(
                error.to_string().contains("unknown bid workflow state"),
                "error should explain unknown state, got {error}"
            );
        }
    }

    #[test]
    fn l6_t_bid_workflow_states_001_lifecycle_enum_serde_round_trips() {
        for state in BidWorkflowState::ALL {
            let json = serde_json::to_string(&state).expect("state serializes");
            let parsed: BidWorkflowState = serde_json::from_str(&json).expect("state deserializes");

            assert_eq!(json, format!("\"{state}\""));
            assert_eq!(parsed, state);
        }

        assert!(serde_json::from_str::<BidWorkflowState>("\"invalid\"").is_err());
    }
}
