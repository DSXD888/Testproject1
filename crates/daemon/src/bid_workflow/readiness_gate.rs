use std::collections::BTreeSet;
use std::io;

use anyhow::{Context, Result};
use chrono::{DateTime, NaiveDate, Utc};
use rusqlite::{Connection, OptionalExtension, Row, params};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;

use super::decision_basis_hash;
use super::types::{
    BidScopePayload, CompanyAddress, CompanyCertification, CompanyContact,
    CompanyFinancialStatements, CompanyInsurancePolicy, CompanyProfile, SmeStatus,
};

pub const QUOTE_ROLE_QUALITY_OVERRIDE_REASON_CODE: &str = "bid_quote_role_quality_override";
pub const TRUST_DEBT_ACCEPTANCE_REASON_CODE: &str = "bid_trust_debt_acceptance";
pub const NON_OVERRIDEABLE_GATE_OVERRIDE_REASON_CODE: &str =
    "bid_non_overrideable_gate_override_attempt";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BidReadinessBlocker {
    Phase3Pending,
    OperatorNotApproved,
    DecisionBasisStale,
    ClassificationAmbiguous,
    TariffBranchDivergence,
    SupplementaryUnitMissing,
    BrokerRequired,
    ClassificationFreshnessExpired,
    CompanyProfileOutOfDate,
    QuoteRoleFloorOnly,
    UpstreamTrustUnvalidated,
    PortalOpportunityRefMissing,
    L5ReadinessProjectionMissing,
    RejectedNonOverrideableOverride,
}

impl BidReadinessBlocker {
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Phase3Pending => "PHASE3_PENDING",
            Self::OperatorNotApproved => "OPERATOR_NOT_APPROVED",
            Self::DecisionBasisStale => "DECISION_BASIS_STALE",
            Self::ClassificationAmbiguous => "CLASSIFICATION_AMBIGUOUS",
            Self::TariffBranchDivergence => "TARIFF_BRANCH_DIVERGENCE",
            Self::SupplementaryUnitMissing => "SUPPLEMENTARY_UNIT_MISSING",
            Self::BrokerRequired => "BROKER_REQUIRED",
            Self::ClassificationFreshnessExpired => "CLASSIFICATION_FRESH_UNTIL_EXPIRED",
            Self::CompanyProfileOutOfDate => "COMPANY_PROFILE_OUT_OF_DATE",
            Self::QuoteRoleFloorOnly => "QUOTE_ROLE_FLOOR_ONLY",
            Self::UpstreamTrustUnvalidated => "UPSTREAM_TRUST_UNVALIDATED",
            Self::PortalOpportunityRefMissing => "PORTAL_OPPORTUNITY_REF_MISSING",
            Self::L5ReadinessProjectionMissing => "L5_READINESS_PROJECTION_MISSING",
            Self::RejectedNonOverrideableOverride => "NON_OVERRIDEABLE_GATE_OVERRIDE_REJECTED",
        }
    }

    #[must_use]
    pub const fn source(self) -> &'static str {
        match self {
            Self::Phase3Pending => "bid_workflow_readiness_gate",
            Self::OperatorNotApproved => "operator_decision_heads",
            Self::DecisionBasisStale => "operator_decision_heads.reviewed_basis_hash",
            Self::ClassificationAmbiguous
            | Self::TariffBranchDivergence
            | Self::SupplementaryUnitMissing
            | Self::BrokerRequired
            | Self::ClassificationFreshnessExpired
            | Self::L5ReadinessProjectionMissing => "landed_cost_estimates",
            Self::CompanyProfileOutOfDate => "shared_company_profile",
            Self::QuoteRoleFloorOnly => "supplier_quotes.price_signal_role",
            Self::UpstreamTrustUnvalidated => "pipeline_assessments.validation_state",
            Self::PortalOpportunityRefMissing => "BidScopePayload.portalOpportunityRef",
            Self::RejectedNonOverrideableOverride => "operator_input_overrides",
        }
    }

    #[must_use]
    pub const fn overrideable(self) -> bool {
        matches!(self, Self::QuoteRoleFloorOnly)
    }

    #[must_use]
    pub const fn diagnostic(self) -> &'static str {
        match self {
            Self::Phase3Pending => "Phase 3 readiness evaluator has not run",
            Self::OperatorNotApproved => {
                "operator_decision_heads.disposition must be approved and not stale"
            }
            Self::DecisionBasisStale => {
                "reviewed_basis_hash must equal the current L6 decision basis hash"
            }
            Self::ClassificationAmbiguous => {
                "selected landed-cost row has CLASSIFICATION_AMBIGUOUS"
            }
            Self::TariffBranchDivergence => "selected landed-cost row has TARIFF_BRANCH_DIVERGENCE",
            Self::SupplementaryUnitMissing => {
                "selected landed-cost row has SUPPLEMENTARY_UNIT_MISSING"
            }
            Self::BrokerRequired => "selected landed-cost row has decision_grade broker_required",
            Self::ClassificationFreshnessExpired => {
                "selected landed-cost row has missing or expired classification_fresh_until"
            }
            Self::CompanyProfileOutOfDate => {
                "company profile is missing, version-mismatched, or has expired required records"
            }
            Self::QuoteRoleFloorOnly => {
                "selected quotes are floor-only evidence and need structured operator override"
            }
            Self::UpstreamTrustUnvalidated => {
                "pipeline_assessments.validation_state is not validated for this exact scope"
            }
            Self::PortalOpportunityRefMissing => {
                "portalOpportunityRef must be non-empty before portal side effects"
            }
            Self::L5ReadinessProjectionMissing => {
                "selected candidate lacks a landed_cost_estimates row for a selected quote"
            }
            Self::RejectedNonOverrideableOverride => {
                "operator attempted to override a non-overrideable bid readiness gate"
            }
        }
    }
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolved_company_profile: Option<CompanyProfile>,
}

pub fn evaluate_bid_readiness(
    conn: &Connection,
    scope: &BidScopePayload,
) -> Result<BidReadinessGateResult> {
    let mut blockers = Vec::new();

    if scope.portal_opportunity_ref.trim().is_empty() {
        push_unique(
            &mut blockers,
            BidReadinessBlocker::PortalOpportunityRefMissing,
        );
    }

    evaluate_operator_decision(conn, scope, &mut blockers)?;
    evaluate_l5_projection(conn, scope, &mut blockers)?;
    let resolved_company_profile = evaluate_company_profile(conn, scope, &mut blockers)?;
    evaluate_quote_role_quality(conn, scope, &mut blockers)?;
    evaluate_upstream_trust(conn, scope, &mut blockers)?;
    evaluate_rejected_non_overrideable_attempts(conn, scope, &mut blockers)?;

    let override_required = if blockers
        .iter()
        .any(|blocker| matches!(blocker, BidReadinessBlocker::QuoteRoleFloorOnly))
    {
        Some(BidReadinessOverrideKind::QuoteRoleQuality)
    } else {
        None
    };

    Ok(BidReadinessGateResult {
        approved: blockers.is_empty(),
        blockers,
        override_required,
        resolved_company_profile,
    })
}

#[must_use]
pub fn phase3_pending_readiness() -> BidReadinessGateResult {
    BidReadinessGateResult {
        approved: false,
        blockers: vec![BidReadinessBlocker::Phase3Pending],
        override_required: None,
        resolved_company_profile: None,
    }
}

fn evaluate_operator_decision(
    conn: &Connection,
    scope: &BidScopePayload,
    blockers: &mut Vec<BidReadinessBlocker>,
) -> Result<()> {
    let head = load_operator_decision_head(conn, &scope.process_id)?;
    let Some(head) = head else {
        push_unique(blockers, BidReadinessBlocker::OperatorNotApproved);
        push_unique(blockers, BidReadinessBlocker::DecisionBasisStale);
        return Ok(());
    };

    if head.disposition != "approved" || head.decision_stale {
        push_unique(blockers, BidReadinessBlocker::OperatorNotApproved);
    }

    match decision_basis_hash::compute(conn, scope) {
        Ok(current_hash) => {
            if head.reviewed_basis_hash.as_deref() != Some(current_hash.as_str())
                || scope.decision_basis_hash != current_hash
            {
                push_unique(blockers, BidReadinessBlocker::DecisionBasisStale);
            }
        }
        Err(_) => push_unique(blockers, BidReadinessBlocker::DecisionBasisStale),
    }

    Ok(())
}

fn evaluate_l5_projection(
    conn: &Connection,
    scope: &BidScopePayload,
    blockers: &mut Vec<BidReadinessBlocker>,
) -> Result<()> {
    let rows = load_l5_readiness_rows(conn, scope)?;
    if rows.len() < scope.candidate_ids.len() {
        push_unique(blockers, BidReadinessBlocker::L5ReadinessProjectionMissing);
    }

    for row in rows {
        if row.decision_grade == "broker_required" {
            push_unique(blockers, BidReadinessBlocker::BrokerRequired);
        }
        if classification_freshness_expired(row.classification_fresh_until.as_deref()) {
            push_unique(
                blockers,
                BidReadinessBlocker::ClassificationFreshnessExpired,
            );
        }
        for code in row.blockers {
            match code.as_str() {
                "CLASSIFICATION_AMBIGUOUS" => {
                    push_unique(blockers, BidReadinessBlocker::ClassificationAmbiguous);
                }
                "TARIFF_BRANCH_DIVERGENCE" => {
                    push_unique(blockers, BidReadinessBlocker::TariffBranchDivergence);
                }
                "SUPPLEMENTARY_UNIT_MISSING" => {
                    push_unique(blockers, BidReadinessBlocker::SupplementaryUnitMissing);
                }
                _ => {}
            }
        }
    }

    Ok(())
}

fn evaluate_company_profile(
    conn: &Connection,
    scope: &BidScopePayload,
    blockers: &mut Vec<BidReadinessBlocker>,
) -> Result<Option<CompanyProfile>> {
    let profile = load_company_profile(conn, &scope.company_profile_version)?;
    let Some(profile) = profile else {
        push_unique(blockers, BidReadinessBlocker::CompanyProfileOutOfDate);
        return Ok(None);
    };

    if !company_profile_current(&profile) {
        push_unique(blockers, BidReadinessBlocker::CompanyProfileOutOfDate);
    }

    Ok(Some(profile))
}

fn evaluate_quote_role_quality(
    conn: &Connection,
    scope: &BidScopePayload,
    blockers: &mut Vec<BidReadinessBlocker>,
) -> Result<()> {
    let quote_roles = load_selected_quote_roles(conn, &scope.selected_quote_ids)?;
    if quote_roles.is_empty() {
        push_unique(blockers, BidReadinessBlocker::QuoteRoleFloorOnly);
        return Ok(());
    }

    let floor_only = quote_roles
        .iter()
        .all(|role| role == "domestic_wholesale_floor" || role == "optimistic_floor");
    if floor_only && !has_quote_role_quality_override(conn, scope)? {
        push_unique(blockers, BidReadinessBlocker::QuoteRoleFloorOnly);
    }

    Ok(())
}

fn evaluate_upstream_trust(
    conn: &Connection,
    scope: &BidScopePayload,
    blockers: &mut Vec<BidReadinessBlocker>,
) -> Result<()> {
    let validation_state = conn
        .query_row(
            "\
            SELECT validation_state
            FROM pipeline_assessments
            WHERE process_id = ?1
            ",
            params![scope.process_id],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .context("loading pipeline_assessments.validation_state")?;

    if validation_state.as_deref() != Some("validated")
        && !has_scope_matched_trust_debt_acceptance(conn, scope)?
    {
        push_unique(blockers, BidReadinessBlocker::UpstreamTrustUnvalidated);
    }

    Ok(())
}

fn evaluate_rejected_non_overrideable_attempts(
    conn: &Connection,
    scope: &BidScopePayload,
    blockers: &mut Vec<BidReadinessBlocker>,
) -> Result<()> {
    if !table_exists(conn, "operator_input_overrides")? {
        return Ok(());
    }

    let count: i64 = conn
        .query_row(
            "\
            SELECT COUNT(*)
            FROM operator_input_overrides
            WHERE process_id = ?1
              AND reason_code = ?2
              AND withdrawn_at IS NULL
              AND superseded_by_override_id IS NULL
            ",
            params![scope.process_id, NON_OVERRIDEABLE_GATE_OVERRIDE_REASON_CODE],
            |row| row.get(0),
        )
        .context("counting rejected non-overrideable bid gate attempts")?;
    if count > 0 {
        push_unique(
            blockers,
            BidReadinessBlocker::RejectedNonOverrideableOverride,
        );
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct OperatorDecisionHead {
    disposition: String,
    reviewed_basis_hash: Option<String>,
    decision_stale: bool,
}

fn load_operator_decision_head(
    conn: &Connection,
    process_id: &str,
) -> Result<Option<OperatorDecisionHead>> {
    if !table_exists(conn, "operator_decision_heads")? {
        return Ok(None);
    }

    conn.query_row(
        "\
        SELECT disposition, reviewed_basis_hash, decision_stale
        FROM operator_decision_heads
        WHERE process_id = ?1
          AND deleted_at IS NULL
        ",
        params![process_id],
        |row| {
            Ok(OperatorDecisionHead {
                disposition: row.get(0)?,
                reviewed_basis_hash: row.get(1)?,
                decision_stale: row.get::<_, bool>(2)?,
            })
        },
    )
    .optional()
    .context("loading operator_decision_heads")
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct L5ReadinessRow {
    decision_grade: String,
    classification_fresh_until: Option<String>,
    blockers: Vec<String>,
}

fn load_l5_readiness_rows(
    conn: &Connection,
    scope: &BidScopePayload,
) -> Result<Vec<L5ReadinessRow>> {
    let mut rows = Vec::with_capacity(scope.candidate_ids.len());
    for candidate_id in &scope.candidate_ids {
        let row = load_l5_readiness_row(conn, candidate_id, &scope.selected_quote_ids)?;
        if let Some(row) = row {
            rows.push(row);
        }
    }
    Ok(rows)
}

fn load_l5_readiness_row(
    conn: &Connection,
    candidate_id: &str,
    selected_quote_ids: &[String],
) -> Result<Option<L5ReadinessRow>> {
    if selected_quote_ids.is_empty() {
        return Ok(None);
    }
    let quote_placeholders = std::iter::repeat_n("?", selected_quote_ids.len())
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!(
        "\
        SELECT decision_grade, classification_fresh_until, blockers
        FROM landed_cost_estimates
        WHERE product_candidate_id = ?
          AND supplier_quote_id IN ({quote_placeholders})
        ORDER BY updated_at DESC, id
        LIMIT 1
        "
    );
    let params = std::iter::once(&candidate_id as &dyn rusqlite::ToSql)
        .chain(
            selected_quote_ids
                .iter()
                .map(|quote_id| quote_id as &dyn rusqlite::ToSql),
        )
        .collect::<Vec<_>>();

    conn.query_row(&sql, params.as_slice(), |row| {
        let blockers_json: String = row.get(2)?;
        Ok(L5ReadinessRow {
            decision_grade: row.get(0)?,
            classification_fresh_until: row.get(1)?,
            blockers: parse_blocker_codes(&blockers_json).map_err(to_sql_error)?,
        })
    })
    .optional()
    .with_context(|| format!("loading landed_cost_estimates readiness row for `{candidate_id}`"))
}

fn load_selected_quote_roles(conn: &Connection, quote_ids: &[String]) -> Result<Vec<String>> {
    let mut roles = Vec::with_capacity(quote_ids.len());
    for quote_id in quote_ids {
        let role = conn
            .query_row(
                "\
                SELECT price_signal_role
                FROM supplier_quotes
                WHERE id = ?1
                ",
                params![quote_id],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .with_context(|| format!("loading selected quote role `{quote_id}`"))?;
        if let Some(role) = role {
            roles.push(role);
        }
    }
    Ok(roles)
}

fn has_quote_role_quality_override(conn: &Connection, scope: &BidScopePayload) -> Result<bool> {
    let overrides = load_active_override_payloads(
        conn,
        &scope.process_id,
        QUOTE_ROLE_QUALITY_OVERRIDE_REASON_CODE,
    )?;
    Ok(overrides.iter().any(|row| {
        row.basis_hash_when_provided.as_deref() == Some(scope.decision_basis_hash.as_str())
            && override_payload_has_acknowledgement(&row.payload)
    }))
}

fn has_scope_matched_trust_debt_acceptance(
    conn: &Connection,
    scope: &BidScopePayload,
) -> Result<bool> {
    let overrides =
        load_active_override_payloads(conn, &scope.process_id, TRUST_DEBT_ACCEPTANCE_REASON_CODE)?;
    Ok(overrides.iter().any(|row| {
        row.basis_hash_when_provided.as_deref() == Some(scope.decision_basis_hash.as_str())
            && trust_debt_payload_matches_scope(&row.payload, scope)
    }))
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct OverridePayloadRow {
    basis_hash_when_provided: Option<String>,
    payload: Value,
}

fn load_active_override_payloads(
    conn: &Connection,
    process_id: &str,
    reason_code: &str,
) -> Result<Vec<OverridePayloadRow>> {
    if !table_exists(conn, "operator_input_overrides")? {
        return Ok(Vec::new());
    }

    let mut stmt = conn
        .prepare(
            "\
            SELECT basis_hash_when_provided, payload_json
            FROM operator_input_overrides
            WHERE process_id = ?1
              AND reason_code = ?2
              AND withdrawn_at IS NULL
              AND superseded_by_override_id IS NULL
            ",
        )
        .context("preparing operator_input_overrides lookup")?;
    let rows = stmt
        .query_map(params![process_id, reason_code], |row| {
            let payload_json: String = row.get(1)?;
            Ok(OverridePayloadRow {
                basis_hash_when_provided: row.get(0)?,
                payload: serde_json::from_str(&payload_json).map_err(to_sql_error)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()
        .context("loading operator_input_overrides payloads")?;
    Ok(rows)
}

fn override_payload_has_acknowledgement(payload: &Value) -> bool {
    let acknowledged = payload
        .get("acknowledged")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let reason_present = payload
        .get("overrideReason")
        .or_else(|| payload.get("reason"))
        .and_then(Value::as_str)
        .is_some_and(|value| !value.trim().is_empty());
    let audit_present = payload
        .get("auditTrailEntryId")
        .or_else(|| payload.get("runManifestCitation"))
        .and_then(Value::as_str)
        .is_some_and(|value| !value.trim().is_empty());

    acknowledged && reason_present && audit_present
}

fn trust_debt_payload_matches_scope(payload: &Value, scope: &BidScopePayload) -> bool {
    payload
        .get("accepted")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        && payload
            .get("decisionBasisHash")
            .and_then(Value::as_str)
            .is_some_and(|value| value == scope.decision_basis_hash)
        && json_string_array_matches(payload.get("lotIds"), &scope.lot_ids)
        && json_string_array_matches(payload.get("candidateIds"), &scope.candidate_ids)
}

fn json_string_array_matches(value: Option<&Value>, expected: &[String]) -> bool {
    let Some(values) = value.and_then(Value::as_array) else {
        return false;
    };
    let actual = values
        .iter()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();
    let expected = expected.iter().cloned().collect::<BTreeSet<_>>();
    actual == expected
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BidProfileJson {
    #[serde(default)]
    certifications: Vec<CompanyCertification>,
    #[serde(default)]
    insurance_policies: Vec<CompanyInsurancePolicy>,
    financial_statements: Option<CompanyFinancialStatements>,
    sme_status: Option<SmeStatus>,
    equality_statement: Option<String>,
    environmental_policy: Option<String>,
}

fn load_company_profile(
    conn: &Connection,
    company_profile_version: &str,
) -> Result<Option<CompanyProfile>> {
    if !table_exists(conn, "shared_company_profile")? {
        return Ok(None);
    }
    let columns = table_columns(conn, "shared_company_profile")?;
    let select = [
        "id".to_owned(),
        select_column_or_null(&columns, "company_name"),
        select_column_or_null(&columns, "registration_number"),
        select_column_or_null(&columns, "vat_number"),
        select_column_or_null(&columns, "address_json"),
        select_column_or_null(&columns, "contact_json"),
        select_column_or_null(&columns, "bid_profile_json"),
    ]
    .join(", ");
    let sql = format!(
        "\
        SELECT {select}
        FROM shared_company_profile
        WHERE (id = ?1 OR CAST(remote_revision AS TEXT) = ?1)
          AND deleted_at IS NULL
        ORDER BY id
        LIMIT 1
        "
    );

    conn.query_row(
        &sql,
        params![company_profile_version],
        company_profile_from_row,
    )
    .optional()
    .context("loading shared_company_profile")
}

fn company_profile_from_row(row: &Row<'_>) -> rusqlite::Result<CompanyProfile> {
    let company_name = required_trimmed(row.get::<_, Option<String>>(1)?);
    let registration_number = required_trimmed(row.get::<_, Option<String>>(2)?);
    let address =
        required_json::<CompanyAddress>(row.get::<_, Option<String>>(4)?).map_err(to_sql_error)?;
    let primary_contact =
        required_json::<CompanyContact>(row.get::<_, Option<String>>(5)?).map_err(to_sql_error)?;
    let bid_profile =
        required_json::<BidProfileJson>(row.get::<_, Option<String>>(6)?).map_err(to_sql_error)?;

    Ok(CompanyProfile {
        company_name,
        registration_number,
        vat_number: row.get(3)?,
        address,
        primary_contact,
        certifications: bid_profile.certifications,
        insurance_policies: bid_profile.insurance_policies,
        financial_statements: bid_profile.financial_statements,
        sme_status: bid_profile.sme_status,
        equality_statement: bid_profile.equality_statement,
        environmental_policy: bid_profile.environmental_policy,
    })
}

fn company_profile_current(profile: &CompanyProfile) -> bool {
    !profile.company_name.trim().is_empty()
        && !profile.registration_number.trim().is_empty()
        && !profile.primary_contact.email.trim().is_empty()
        && !profile.primary_contact.phone.trim().is_empty()
        && !profile.certifications.is_empty()
        && !profile.insurance_policies.is_empty()
        && profile
            .certifications
            .iter()
            .all(|certification| expiry_is_current(&certification.expiry_date))
        && profile
            .insurance_policies
            .iter()
            .all(|policy| expiry_is_current(&policy.expiry_date))
}

fn required_trimmed(value: Option<String>) -> String {
    value.map_or_else(String::new, |value| value.trim().to_owned())
}

fn required_json<T>(value: Option<String>) -> Result<T>
where
    T: DeserializeOwned,
{
    let json = value
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
        .context("required company profile JSON field missing")?;
    serde_json::from_str(&json).context("parsing company profile JSON field")
}

fn select_column_or_null(columns: &[String], column: &str) -> String {
    if columns.iter().any(|candidate| candidate == column) {
        column.to_owned()
    } else {
        format!("NULL AS {column}")
    }
}

fn classification_freshness_expired(value: Option<&str>) -> bool {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_none_or(|value| !expiry_is_current(value))
}

fn expiry_is_current(value: &str) -> bool {
    if let Ok(timestamp) = DateTime::parse_from_rfc3339(value) {
        return timestamp.with_timezone(&Utc) > Utc::now();
    }
    NaiveDate::parse_from_str(value, "%Y-%m-%d").is_ok_and(|date| date >= Utc::now().date_naive())
}

fn parse_blocker_codes(raw_json: &str) -> Result<Vec<String>> {
    let value: Value = serde_json::from_str(raw_json).context("parsing L5 blocker JSON")?;
    let Some(values) = value.as_array() else {
        return Ok(Vec::new());
    };
    let mut blockers = values
        .iter()
        .filter_map(|value| {
            value.as_str().or_else(|| {
                value
                    .as_object()
                    .and_then(|object| object.get("code"))
                    .and_then(Value::as_str)
            })
        })
        .map(str::trim)
        .filter(|code| !code.is_empty())
        .map(str::to_owned)
        .collect::<Vec<_>>();
    blockers.sort();
    blockers.dedup();
    Ok(blockers)
}

fn table_exists(conn: &Connection, table: &str) -> Result<bool> {
    conn.query_row(
        "\
        SELECT EXISTS(
          SELECT 1 FROM sqlite_master
          WHERE type IN ('table', 'view') AND name = ?1
        ) OR EXISTS(
          SELECT 1 FROM sqlite_temp_master
          WHERE type IN ('table', 'view') AND name = ?1
        )
        ",
        params![table],
        |row| row.get::<_, bool>(0),
    )
    .with_context(|| format!("checking table existence for `{table}`"))
}

fn table_columns(conn: &Connection, table: &str) -> Result<Vec<String>> {
    let mut stmt = conn
        .prepare(&format!("PRAGMA table_info({table})"))
        .with_context(|| format!("preparing table_info query for `{table}`"))?;
    stmt.query_map([], |row| row.get::<_, String>(1))?
        .collect::<std::result::Result<Vec<_>, _>>()
        .with_context(|| format!("loading columns for `{table}`"))
}

fn push_unique(blockers: &mut Vec<BidReadinessBlocker>, blocker: BidReadinessBlocker) {
    if !blockers.contains(&blocker) {
        blockers.push(blocker);
    }
}

fn to_sql_error(error: impl std::fmt::Display) -> rusqlite::Error {
    rusqlite::Error::ToSqlConversionFailure(Box::new(io::Error::other(error.to_string())))
}
