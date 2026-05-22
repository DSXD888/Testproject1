use serde::{Deserialize, Serialize};

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
