#![forbid(unsafe_code)]

use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocumentRole {
    TechnicalSpecification,
    InvitationToTender,
    StatementOfRequirements,
    PricingSchedule,
    Boq,
    MixedTenderPack,
    CombinedPack,
    Administrative,
    CoverLetter,
    TermsAndConditions,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedExtractionPolicy {
    pub document_role: DocumentRole,
    pub expected_overlays: Vec<String>,
    pub extraction_policy_version: String,
}

impl DocumentRole {
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::TechnicalSpecification => "technical_specification",
            Self::InvitationToTender => "invitation_to_tender",
            Self::StatementOfRequirements => "statement_of_requirements",
            Self::PricingSchedule => "pricing_schedule",
            Self::Boq => "boq",
            Self::MixedTenderPack => "mixed_tender_pack",
            Self::CombinedPack => "combined_pack",
            Self::Administrative => "administrative",
            Self::CoverLetter => "cover_letter",
            Self::TermsAndConditions => "terms_and_conditions",
            Self::Unknown => "unknown",
        }
    }
}

pub fn resolve_document_role(
    ocds_document_type: Option<&str>,
    filename: Option<&str>,
    extracted_markdown: &str,
    extracted_tables_json: &Value,
) -> DocumentRole {
    if let Some(role) = ocds_document_type.and_then(role_from_ocds_document_type) {
        return role;
    }
    if let Some(role) = filename.and_then(role_from_filename) {
        return role;
    }
    role_from_structural_content(extracted_markdown, extracted_tables_json)
}

pub fn resolve_extraction_policy(
    role: &DocumentRole,
    extracted_markdown: &str,
    extracted_tables_json: &Value,
) -> ResolvedExtractionPolicy {
    let expected_overlays = match role {
        DocumentRole::TechnicalSpecification
        | DocumentRole::InvitationToTender
        | DocumentRole::StatementOfRequirements => {
            vec!["pdf-extraction/tender-specification".to_owned()]
        }
        DocumentRole::PricingSchedule => vec!["pdf-extraction/pricing-schedule".to_owned()],
        DocumentRole::Boq => vec!["pdf-extraction/bill-of-quantities".to_owned()],
        DocumentRole::MixedTenderPack | DocumentRole::CombinedPack => {
            let mut overlays = vec!["pdf-extraction/tender-specification".to_owned()];
            if looks_like_pricing_schedule(extracted_markdown, extracted_tables_json) {
                overlays.push("pdf-extraction/pricing-schedule".to_owned());
            }
            if looks_like_boq(extracted_markdown, extracted_tables_json) {
                overlays.push("pdf-extraction/bill-of-quantities".to_owned());
            }
            overlays
        }
        DocumentRole::Administrative
        | DocumentRole::CoverLetter
        | DocumentRole::TermsAndConditions
        | DocumentRole::Unknown => Vec::new(),
    };

    ResolvedExtractionPolicy {
        document_role: role.clone(),
        expected_overlays,
        extraction_policy_version: "2026.04.12".to_owned(),
    }
}

fn role_from_ocds_document_type(raw: &str) -> Option<DocumentRole> {
    let normalized = normalize(raw);
    match normalized.as_str() {
        "technicalspecifications" | "technicalspecification" | "technical_specification" => {
            Some(DocumentRole::TechnicalSpecification)
        }
        "invitationtotender" | "invitation_to_tender" | "biddingdocuments" => {
            Some(DocumentRole::InvitationToTender)
        }
        "statementofrequirements" | "statement_of_requirements" => {
            Some(DocumentRole::StatementOfRequirements)
        }
        "pricingschedule" | "pricing_schedule" | "scheduleofrates" => {
            Some(DocumentRole::PricingSchedule)
        }
        "billofquantities" | "bill_of_quantities" | "boq" => Some(DocumentRole::Boq),
        "coverletter" | "cover_letter" => Some(DocumentRole::CoverLetter),
        "termsandconditions" | "terms_and_conditions" | "tandc" => {
            Some(DocumentRole::TermsAndConditions)
        }
        "administrative" => Some(DocumentRole::Administrative),
        _ => None,
    }
}

fn role_from_filename(raw: &str) -> Option<DocumentRole> {
    let normalized = normalize(raw);
    if normalized.contains("pricing") && normalized.contains("schedule") {
        return Some(DocumentRole::PricingSchedule);
    }
    if normalized.contains("scheduleofrates") || normalized.contains("ratecard") {
        return Some(DocumentRole::PricingSchedule);
    }
    if normalized.contains("billofquantities") || normalized.contains("boq") {
        return Some(DocumentRole::Boq);
    }
    if normalized.contains("statementofrequirements") {
        return Some(DocumentRole::StatementOfRequirements);
    }
    if normalized.contains("invitationtotender") || normalized.contains("-itt") {
        return Some(DocumentRole::InvitationToTender);
    }
    if normalized.contains("specification") {
        return Some(DocumentRole::TechnicalSpecification);
    }
    if normalized.contains("coverletter") {
        return Some(DocumentRole::CoverLetter);
    }
    if normalized.contains("termsandconditions") || normalized.contains("termsconditions") {
        return Some(DocumentRole::TermsAndConditions);
    }
    if normalized.contains("combinedpack") {
        return Some(DocumentRole::CombinedPack);
    }
    None
}

fn role_from_structural_content(markdown: &str, tables_json: &Value) -> DocumentRole {
    let normalized = normalize(markdown);
    let pricing = looks_like_pricing_schedule(markdown, tables_json);
    let boq = looks_like_boq(markdown, tables_json);
    let tender = contains_lot_heading(markdown)
        || normalized.contains("technicalspecification")
        || normalized.contains("statementofrequirements")
        || normalized.contains("invitationtotender")
        || normalized.contains("standard")
        || normalized.contains("bsen");

    if normalized.contains("combinedpack") {
        return DocumentRole::CombinedPack;
    }
    if tender && (pricing || boq) {
        return DocumentRole::MixedTenderPack;
    }
    if boq {
        return DocumentRole::Boq;
    }
    if pricing {
        return DocumentRole::PricingSchedule;
    }
    if normalized.contains("statementofrequirements") {
        return DocumentRole::StatementOfRequirements;
    }
    if normalized.contains("invitationtotender") {
        return DocumentRole::InvitationToTender;
    }
    if tender {
        return DocumentRole::TechnicalSpecification;
    }
    if normalized.contains("coverletter") {
        return DocumentRole::CoverLetter;
    }
    if normalized.contains("termsandconditions") {
        return DocumentRole::TermsAndConditions;
    }
    DocumentRole::Unknown
}

fn looks_like_pricing_schedule(markdown: &str, tables_json: &Value) -> bool {
    let normalized = normalize(markdown);
    if normalized.contains("pricingschedule")
        || normalized.contains("scheduleofrates")
        || normalized.contains("ratecard")
    {
        return true;
    }
    table_headers(tables_json)
        .iter()
        .any(|header| header.contains("qty") || header.contains("rate") || header.contains("price"))
}

fn looks_like_boq(markdown: &str, tables_json: &Value) -> bool {
    let normalized = normalize(markdown);
    if normalized.contains("billofquantities")
        || normalized.contains("boq")
        || normalized.contains("scheduleofquantities")
    {
        return true;
    }
    table_headers(tables_json)
        .iter()
        .any(|header| header.contains("itemref") || header.contains("continuation"))
}

fn contains_lot_heading(markdown: &str) -> bool {
    markdown.lines().any(|line| {
        let line = normalize(line);
        line.as_bytes()
            .strip_prefix(b"lot")
            .is_some_and(|suffix| suffix.first().is_some_and(u8::is_ascii_digit))
    })
}

fn table_headers(tables_json: &Value) -> Vec<String> {
    tables_json
        .as_array()
        .into_iter()
        .flatten()
        .flat_map(|table| {
            table
                .get("headers")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
        })
        .filter_map(Value::as_str)
        .map(normalize)
        .collect()
}

fn normalize(raw: &str) -> String {
    raw.chars()
        .filter(char::is_ascii_alphanumeric)
        .flat_map(char::to_lowercase)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_role_from_ocds_document_type() {
        let role = resolve_document_role(Some("technicalSpecifications"), None, "", &json!([]));
        assert_eq!(role, DocumentRole::TechnicalSpecification);
    }

    #[test]
    fn test_role_from_filename() {
        let role = resolve_document_role(None, Some("Pricing Schedule.xlsx"), "", &json!([]));
        assert_eq!(role, DocumentRole::PricingSchedule);
    }

    #[test]
    fn test_role_from_structural_content() {
        let role = resolve_document_role(
            None,
            None,
            "# Tender Pack\n\n## Lot 1\n## Pricing Schedule\n| Item | Qty | Rate |",
            &json!([
                {"headers": ["Item", "Qty", "Rate"]}
            ]),
        );
        assert_eq!(role, DocumentRole::MixedTenderPack);
    }

    #[test]
    fn test_policy_mapping_for_single_overlay_roles() {
        let policy = resolve_extraction_policy(&DocumentRole::PricingSchedule, "", &json!([]));
        assert_eq!(
            policy.expected_overlays,
            vec!["pdf-extraction/pricing-schedule"]
        );
    }

    #[test]
    fn test_policy_mapping_for_administrative_skip() {
        let policy = resolve_extraction_policy(&DocumentRole::Administrative, "", &json!([]));
        assert!(policy.expected_overlays.is_empty());
    }

    #[test]
    fn test_policy_mapping_for_mixed_pack_can_emit_multiple_overlays() {
        let policy = resolve_extraction_policy(
            &DocumentRole::MixedTenderPack,
            "# Tender Pack\nPricing schedule\nBill of Quantities",
            &json!([
                {"headers": ["Qty", "Rate"]},
                {"headers": ["Item Ref", "Continuation"]}
            ]),
        );
        assert_eq!(
            policy.expected_overlays,
            vec![
                "pdf-extraction/tender-specification",
                "pdf-extraction/pricing-schedule",
                "pdf-extraction/bill-of-quantities",
            ]
        );
    }
}
