use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::str::FromStr;

use rusqlite::{Connection, OptionalExtension};
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

impl BidScopePayload {
    pub const DEFAULT_REVIEW_REQUIRED: bool = true;
    pub const DEFAULT_SIDE_EFFECT_CAP: SideEffectClass = SideEffectClass::RemoteDraft;
    pub const SUPPORTED_PORTAL_TYPE: &'static str = "procontract";

    #[must_use]
    pub const fn default_review_required() -> bool {
        Self::DEFAULT_REVIEW_REQUIRED
    }

    #[must_use]
    pub const fn default_side_effect_cap() -> SideEffectClass {
        Self::DEFAULT_SIDE_EFFECT_CAP
    }
}

#[must_use]
pub const fn default_bid_scope_fields() -> (bool, SideEffectClass) {
    (
        BidScopePayload::DEFAULT_REVIEW_REQUIRED,
        BidScopePayload::DEFAULT_SIDE_EFFECT_CAP,
    )
}

const DECISION_BASIS_HASH_LENGTH: usize = 64;
const VIEWER_SAFE_BID_WORKFLOW_COLUMNS: &[&str] = &[
    "process_id",
    "bid_state",
    "review_required",
    "bid_job_id",
    "started_at",
    "completed_at",
    "row_version",
    "stage_run_id",
    "updated_at",
    "created_at",
];

#[must_use]
pub const fn bid_workflow_viewer_projection_columns() -> &'static [&'static str] {
    VIEWER_SAFE_BID_WORKFLOW_COLUMNS
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BidScopeValidationError {
    EmptyField(&'static str),
    EmptyList(&'static str),
    UnsupportedPortalType(String),
    InvalidDecisionBasisHash,
    ProcessNotFound(String),
    OperatorDecisionNotApproved(String),
    PipelineRollupMissing(String),
    LotNotOwnedByProcess { lot_id: String, process_id: String },
    CandidateNotOwnedBySelectedLots { candidate_id: String },
    QuoteNotOwnedBySelectedCandidates { quote_id: String },
    CompanyProfileVersionNotFound(String),
    ReferenceLookupFailed(String),
}

impl BidScopeValidationError {
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::EmptyField(_) => "BID_SCOPE_EMPTY_FIELD",
            Self::EmptyList(_) => "BID_SCOPE_EMPTY_LIST",
            Self::UnsupportedPortalType(_) => "BID_SCOPE_UNSUPPORTED_PORTAL",
            Self::InvalidDecisionBasisHash => "BID_SCOPE_INVALID_DECISION_BASIS_HASH",
            Self::ProcessNotFound(_) => "BID_SCOPE_PROCESS_NOT_FOUND",
            Self::OperatorDecisionNotApproved(_) => "BID_SCOPE_OPERATOR_DECISION_NOT_APPROVED",
            Self::PipelineRollupMissing(_) => "BID_SCOPE_PIPELINE_ROLLUP_MISSING",
            Self::LotNotOwnedByProcess { .. } => "BID_SCOPE_LOT_NOT_OWNED_BY_PROCESS",
            Self::CandidateNotOwnedBySelectedLots { .. } => "BID_SCOPE_CANDIDATE_NOT_OWNED_BY_LOTS",
            Self::QuoteNotOwnedBySelectedCandidates { .. } => {
                "BID_SCOPE_QUOTE_NOT_OWNED_BY_CANDIDATES"
            }
            Self::CompanyProfileVersionNotFound(_) => "BID_SCOPE_COMPANY_PROFILE_NOT_FOUND",
            Self::ReferenceLookupFailed(_) => "BID_SCOPE_REFERENCE_LOOKUP_FAILED",
        }
    }
}

impl Display for BidScopeValidationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyField(field) => write!(f, "{field} must not be empty"),
            Self::EmptyList(field) => write!(f, "{field} must contain at least one non-empty id"),
            Self::UnsupportedPortalType(portal_type) => write!(
                f,
                "unsupported bidPortalType `{portal_type}`; expected `{}`",
                BidScopePayload::SUPPORTED_PORTAL_TYPE
            ),
            Self::InvalidDecisionBasisHash => write!(
                f,
                "decisionBasisHash must be a 64-character hexadecimal SHA-256 string"
            ),
            Self::ProcessNotFound(process_id) => {
                write!(f, "process `{process_id}` does not exist")
            }
            Self::OperatorDecisionNotApproved(process_id) => write!(
                f,
                "process `{process_id}` does not have a current approved operator decision"
            ),
            Self::PipelineRollupMissing(process_id) => write!(
                f,
                "process `{process_id}` does not have a current pipeline rollup"
            ),
            Self::LotNotOwnedByProcess { lot_id, process_id } => {
                write!(
                    f,
                    "lot `{lot_id}` does not belong to process `{process_id}`"
                )
            }
            Self::CandidateNotOwnedBySelectedLots { candidate_id } => write!(
                f,
                "candidate `{candidate_id}` does not belong to any selected lot"
            ),
            Self::QuoteNotOwnedBySelectedCandidates { quote_id } => write!(
                f,
                "quote `{quote_id}` does not belong to any selected candidate"
            ),
            Self::CompanyProfileVersionNotFound(version) => {
                write!(f, "company profile version `{version}` does not exist")
            }
            Self::ReferenceLookupFailed(message) => {
                write!(f, "bid scope reference lookup failed: {message}")
            }
        }
    }
}

impl Error for BidScopeValidationError {}

pub trait BidScopeReferenceResolver {
    fn process_exists(&self, process_id: &str) -> Result<bool, BidScopeValidationError>;

    fn process_has_approved_operator_decision(
        &self,
        process_id: &str,
    ) -> Result<bool, BidScopeValidationError>;

    fn process_has_current_pipeline_rollup(
        &self,
        process_id: &str,
    ) -> Result<bool, BidScopeValidationError>;

    fn lot_belongs_to_process(
        &self,
        lot_id: &str,
        process_id: &str,
    ) -> Result<bool, BidScopeValidationError>;

    fn candidate_belongs_to_lots(
        &self,
        candidate_id: &str,
        lot_ids: &[String],
    ) -> Result<bool, BidScopeValidationError>;

    fn quote_belongs_to_candidates(
        &self,
        quote_id: &str,
        candidate_ids: &[String],
    ) -> Result<bool, BidScopeValidationError>;

    fn company_profile_version_exists(
        &self,
        company_profile_version: &str,
    ) -> Result<bool, BidScopeValidationError>;
}

pub struct SqliteBidScopeReferenceResolver<'conn> {
    conn: &'conn Connection,
}

impl<'conn> SqliteBidScopeReferenceResolver<'conn> {
    #[must_use]
    pub const fn new(conn: &'conn Connection) -> Self {
        Self { conn }
    }

    fn exists(
        &self,
        sql: &str,
        params: &[&dyn rusqlite::ToSql],
    ) -> Result<bool, BidScopeValidationError> {
        self.conn
            .query_row(sql, params, |row| row.get::<_, i64>(0))
            .optional()
            .map(|count| count.unwrap_or(0) > 0)
            .map_err(|error| BidScopeValidationError::ReferenceLookupFailed(error.to_string()))
    }
}

impl BidScopeReferenceResolver for SqliteBidScopeReferenceResolver<'_> {
    fn process_exists(&self, process_id: &str) -> Result<bool, BidScopeValidationError> {
        self.exists(
            "SELECT COUNT(*) FROM procurement_processes WHERE id = ?1",
            &[&process_id],
        )
    }

    fn process_has_approved_operator_decision(
        &self,
        process_id: &str,
    ) -> Result<bool, BidScopeValidationError> {
        self.exists(
            "\
            SELECT COUNT(*)
            FROM operator_decision_heads
            WHERE process_id = ?1
              AND disposition = 'approved'
              AND COALESCE(decision_stale, 0) = 0
              AND deleted_at IS NULL
            ",
            &[&process_id],
        )
    }

    fn process_has_current_pipeline_rollup(
        &self,
        process_id: &str,
    ) -> Result<bool, BidScopeValidationError> {
        self.exists(
            "\
            SELECT COUNT(*)
            FROM pipeline_assessments
            WHERE process_id = ?1
              AND decision_basis_hash IS NOT NULL
              AND length(trim(decision_basis_hash)) > 0
            ",
            &[&process_id],
        )
    }

    fn lot_belongs_to_process(
        &self,
        lot_id: &str,
        process_id: &str,
    ) -> Result<bool, BidScopeValidationError> {
        self.exists(
            "\
            SELECT COUNT(*)
            FROM notice_lots nl
            JOIN contract_notices cn ON cn.id = nl.notice_id
            WHERE nl.id = ?1
              AND cn.process_id = ?2
            ",
            &[&lot_id, &process_id],
        )
    }

    fn candidate_belongs_to_lots(
        &self,
        candidate_id: &str,
        lot_ids: &[String],
    ) -> Result<bool, BidScopeValidationError> {
        if lot_ids.is_empty() {
            return Ok(false);
        }
        let placeholders = std::iter::repeat_n("?", lot_ids.len())
            .collect::<Vec<_>>()
            .join(", ");
        let sql = format!(
            "\
            SELECT COUNT(*)
            FROM product_candidates
            WHERE id = ?
              AND lot_id IN ({placeholders})
            "
        );
        let params = std::iter::once(&candidate_id as &dyn rusqlite::ToSql)
            .chain(lot_ids.iter().map(|lot_id| lot_id as &dyn rusqlite::ToSql))
            .collect::<Vec<_>>();
        self.exists(&sql, &params)
    }

    fn quote_belongs_to_candidates(
        &self,
        quote_id: &str,
        candidate_ids: &[String],
    ) -> Result<bool, BidScopeValidationError> {
        if candidate_ids.is_empty() {
            return Ok(false);
        }
        let placeholders = std::iter::repeat_n("?", candidate_ids.len())
            .collect::<Vec<_>>()
            .join(", ");
        let sql = format!(
            "\
            SELECT COUNT(*)
            FROM supplier_quotes
            WHERE id = ?
              AND product_candidate_id IN ({placeholders})
            "
        );
        let params = std::iter::once(&quote_id as &dyn rusqlite::ToSql)
            .chain(
                candidate_ids
                    .iter()
                    .map(|candidate_id| candidate_id as &dyn rusqlite::ToSql),
            )
            .collect::<Vec<_>>();
        self.exists(&sql, &params)
    }

    fn company_profile_version_exists(
        &self,
        company_profile_version: &str,
    ) -> Result<bool, BidScopeValidationError> {
        self.exists(
            "\
            SELECT COUNT(*)
            FROM shared_company_profile
            WHERE (id = ?1 OR CAST(remote_revision AS TEXT) = ?1)
              AND deleted_at IS NULL
            ",
            &[&company_profile_version],
        )
    }
}

pub fn validate_bid_scope_shape(scope: &BidScopePayload) -> Result<(), BidScopeValidationError> {
    require_non_empty("processId", &scope.process_id)?;
    require_non_empty_list("lotIds", &scope.lot_ids)?;
    require_non_empty_list("candidateIds", &scope.candidate_ids)?;
    require_non_empty_list("selectedQuoteIds", &scope.selected_quote_ids)?;
    require_non_empty("companyProfileVersion", &scope.company_profile_version)?;
    require_non_empty("bidPortalType", &scope.bid_portal_type)?;
    if scope.bid_portal_type.trim() != BidScopePayload::SUPPORTED_PORTAL_TYPE {
        return Err(BidScopeValidationError::UnsupportedPortalType(
            scope.bid_portal_type.clone(),
        ));
    }
    require_non_empty("portalOpportunityRef", &scope.portal_opportunity_ref)?;
    require_decision_basis_hash(&scope.decision_basis_hash)?;
    Ok(())
}

pub fn validate_bid_scope<R>(
    scope: &BidScopePayload,
    references: &R,
) -> Result<(), BidScopeValidationError>
where
    R: BidScopeReferenceResolver + ?Sized,
{
    validate_bid_scope_shape(scope)?;

    if !references.process_exists(scope.process_id.trim())? {
        return Err(BidScopeValidationError::ProcessNotFound(
            scope.process_id.clone(),
        ));
    }
    if !references.process_has_approved_operator_decision(scope.process_id.trim())? {
        return Err(BidScopeValidationError::OperatorDecisionNotApproved(
            scope.process_id.clone(),
        ));
    }
    if !references.process_has_current_pipeline_rollup(scope.process_id.trim())? {
        return Err(BidScopeValidationError::PipelineRollupMissing(
            scope.process_id.clone(),
        ));
    }

    for lot_id in &scope.lot_ids {
        if !references.lot_belongs_to_process(lot_id.trim(), scope.process_id.trim())? {
            return Err(BidScopeValidationError::LotNotOwnedByProcess {
                lot_id: lot_id.clone(),
                process_id: scope.process_id.clone(),
            });
        }
    }
    for candidate_id in &scope.candidate_ids {
        if !references.candidate_belongs_to_lots(candidate_id.trim(), &scope.lot_ids)? {
            return Err(BidScopeValidationError::CandidateNotOwnedBySelectedLots {
                candidate_id: candidate_id.clone(),
            });
        }
    }
    for quote_id in &scope.selected_quote_ids {
        if !references.quote_belongs_to_candidates(quote_id.trim(), &scope.candidate_ids)? {
            return Err(BidScopeValidationError::QuoteNotOwnedBySelectedCandidates {
                quote_id: quote_id.clone(),
            });
        }
    }
    if !references.company_profile_version_exists(scope.company_profile_version.trim())? {
        return Err(BidScopeValidationError::CompanyProfileVersionNotFound(
            scope.company_profile_version.clone(),
        ));
    }

    Ok(())
}

fn require_non_empty(field: &'static str, value: &str) -> Result<(), BidScopeValidationError> {
    if value.trim().is_empty() {
        return Err(BidScopeValidationError::EmptyField(field));
    }
    Ok(())
}

fn require_non_empty_list(
    field: &'static str,
    values: &[String],
) -> Result<(), BidScopeValidationError> {
    if values.is_empty() || values.iter().any(|value| value.trim().is_empty()) {
        return Err(BidScopeValidationError::EmptyList(field));
    }
    Ok(())
}

fn require_decision_basis_hash(value: &str) -> Result<(), BidScopeValidationError> {
    if value.len() != DECISION_BASIS_HASH_LENGTH
        || !value.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        return Err(BidScopeValidationError::InvalidDecisionBasisHash);
    }
    Ok(())
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
