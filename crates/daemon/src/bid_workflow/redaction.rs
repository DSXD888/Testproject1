#![allow(clippy::module_name_repetitions)]

use anyhow::{Context, Result};
use serde::Serialize;
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

pub const REDACTED: &str = "[REDACTED]";

pub fn redact_company_profile(profile: &impl Serialize) -> Result<Value> {
    let value =
        serde_json::to_value(profile).context("serializing company profile for redaction")?;
    Ok(redact_named_value(Some("companyProfile"), &value))
}

pub fn redact_bid_scope_payload(scope: &impl Serialize) -> Result<Value> {
    let value =
        serde_json::to_value(scope).context("serializing bid scope payload for redaction")?;
    Ok(redact_named_value(Some("bidScopePayload"), &value))
}

#[must_use]
pub fn redact_bid_skill_input(input: &Value) -> Value {
    redact_named_value(None, input)
}

#[must_use]
pub fn redact_bid_error_context(context: &Value) -> Value {
    redact_error_context_value(context, false)
}

#[must_use]
pub fn redact_structured_trace_value(value: &Value) -> Value {
    redact_named_value(None, value)
}

#[must_use]
pub fn redact_bid_workflow_state_payload(payload: &Value) -> Value {
    let mut redacted = payload.clone();
    if let Value::Object(map) = &mut redacted
        && let Some(Value::String(raw_scope)) = map.get("bid_scope_json")
    {
        let replacement = serde_json::from_str::<Value>(raw_scope).map_or_else(
            |_| Value::String(REDACTED.to_owned()),
            |scope| redact_named_value(Some("bidScopePayload"), &scope),
        );
        map.insert("bid_scope_json".to_owned(), replacement.to_string().into());
    }
    redact_named_value(None, &redacted)
}

#[must_use]
pub fn redact_sensitive_text(text: &str) -> String {
    text.split_whitespace()
        .map(redact_sensitive_token)
        .collect::<Vec<_>>()
        .join(" ")
}

#[must_use]
pub fn stable_redaction_hash(label: &str, value: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(label.as_bytes());
    hasher.update(b"\0");
    hasher.update(value.as_bytes());
    let digest = hasher.finalize();
    format!("{label}:sha256:{digest:x}")
}

fn redact_named_value(key: Option<&str>, value: &Value) -> Value {
    let key_kind = key.map_or(KeyKind::Regular, classify_key);
    match (key_kind, value) {
        (KeyKind::StableHash, _) => value.clone(),
        (KeyKind::SensitiveHash, Value::String(raw)) => {
            Value::String(stable_redaction_hash(key.unwrap_or("value"), raw))
        }
        (KeyKind::SensitiveHash, Value::Array(items)) => {
            let hash_label = key.unwrap_or("value");
            Value::Array(
                items
                    .iter()
                    .map(|item| match item {
                        Value::String(raw) => Value::String(stable_redaction_hash(hash_label, raw)),
                        other => redact_named_value(None, other),
                    })
                    .collect(),
            )
        }
        (KeyKind::SensitiveHash, Value::Number(number)) => Value::String(stable_redaction_hash(
            key.unwrap_or("number"),
            &number.to_string(),
        )),
        (KeyKind::SensitiveRedact, Value::Object(object)) => Value::Object(
            object
                .iter()
                .map(|(child_key, child_value)| {
                    (
                        child_key.clone(),
                        redact_named_value(Some(child_key), child_value),
                    )
                })
                .collect(),
        ),
        (KeyKind::SensitiveRedact, Value::Array(items)) => Value::Array(
            items
                .iter()
                .map(|item| redact_named_value(None, item))
                .collect(),
        ),
        (KeyKind::SensitiveRedact, Value::Null) => Value::Null,
        (KeyKind::SensitiveRedact, _) => Value::String(REDACTED.to_owned()),
        (_, Value::Object(object)) => redact_object(object),
        (_, Value::Array(items)) => Value::Array(
            items
                .iter()
                .map(|item| redact_named_value(None, item))
                .collect(),
        ),
        (_, Value::String(text)) => Value::String(redact_sensitive_text(text)),
        _ => value.clone(),
    }
}

fn redact_object(object: &Map<String, Value>) -> Value {
    if object.contains_key("bidScopePayload") || object.contains_key("bid_scope_payload") {
        let mut redacted = Map::new();
        for (key, value) in object {
            redacted.insert(key.clone(), redact_named_value(Some(key), value));
        }
        return Value::Object(redacted);
    }

    Value::Object(
        object
            .iter()
            .map(|(key, value)| (key.clone(), redact_named_value(Some(key), value)))
            .collect(),
    )
}

fn redact_error_context_value(value: &Value, inside_bid_scope: bool) -> Value {
    match value {
        Value::Array(items) => Value::Array(
            items
                .iter()
                .map(|item| redact_error_context_value(item, inside_bid_scope))
                .collect(),
        ),
        Value::Object(object) => Value::Object(
            object
                .iter()
                .map(|(key, value)| {
                    let normalized = key.replace(['_', '-'], "").to_ascii_lowercase();
                    let child_inside_bid_scope =
                        inside_bid_scope || normalized.as_str() == "bidscopepayload";
                    let redacted = if is_sensitive_error_context_key(&normalized, inside_bid_scope)
                    {
                        match value {
                            Value::String(raw) => Value::String(stable_redaction_hash(key, raw)),
                            Value::Array(items) => Value::Array(
                                items
                                    .iter()
                                    .map(|item| match item {
                                        Value::String(raw) => {
                                            Value::String(stable_redaction_hash(key, raw))
                                        }
                                        other => redact_error_context_value(
                                            other,
                                            child_inside_bid_scope,
                                        ),
                                    })
                                    .collect(),
                            ),
                            Value::Number(number) => {
                                Value::String(stable_redaction_hash(key, &number.to_string()))
                            }
                            Value::Null => Value::Null,
                            _ => Value::String(REDACTED.to_owned()),
                        }
                    } else {
                        redact_error_context_value(value, child_inside_bid_scope)
                    };
                    (key.clone(), redacted)
                })
                .collect(),
        ),
        Value::String(text) => Value::String(redact_sensitive_text(text)),
        _ => value.clone(),
    }
}

fn is_sensitive_error_context_key(normalized: &str, inside_bid_scope: bool) -> bool {
    if normalized.ends_with("hash") {
        return false;
    }
    if normalized == "portalopportunityref" {
        return inside_bid_scope;
    }
    matches!(
        normalized,
        "processid"
            | "lotid"
            | "lotids"
            | "candidateid"
            | "candidateids"
            | "quoteid"
            | "quoteids"
            | "selectedquoteid"
            | "selectedquoteids"
            | "operatoremail"
            | "companyname"
            | "registrationnumber"
            | "vatnumber"
            | "number"
            | "certificationnumber"
            | "policynumber"
            | "coveramountpence"
            | "companieshouseref"
            | "address"
            | "line1"
            | "line2"
            | "city"
            | "county"
            | "postcode"
            | "country"
            | "primarycontact"
            | "contact"
            | "name"
            | "email"
            | "phone"
            | "role"
            | "filepath"
            | "localdocumentpath"
            | "documentpath"
            | "portalformvalues"
            | "formvalues"
            | "companyprofile"
            | "financialstatements"
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KeyKind {
    Regular,
    StableHash,
    SensitiveHash,
    SensitiveRedact,
}

fn classify_key(key: &str) -> KeyKind {
    let normalized = key.replace(['_', '-'], "").to_ascii_lowercase();
    if normalized.ends_with("hash")
        || matches!(
            normalized.as_str(),
            "decisionbasishash"
                | "contenthash"
                | "artifacthash"
                | "runmanifesthash"
                | "pagefingerprinthash"
                | "validatorrubrichash"
        )
    {
        return KeyKind::StableHash;
    }
    if matches!(
        normalized.as_str(),
        "processid"
            | "lotid"
            | "lotids"
            | "candidateid"
            | "candidateids"
            | "quoteid"
            | "quoteids"
            | "selectedquoteid"
            | "selectedquoteids"
            | "portalopportunityref"
            | "companyname"
            | "registrationnumber"
            | "vatnumber"
            | "number"
            | "certificationnumber"
            | "policynumber"
            | "coveramountpence"
            | "companieshouseref"
    ) {
        return KeyKind::SensitiveHash;
    }
    if matches!(
        normalized.as_str(),
        "address"
            | "line1"
            | "line2"
            | "city"
            | "county"
            | "postcode"
            | "country"
            | "primarycontact"
            | "contact"
            | "name"
            | "email"
            | "phone"
            | "role"
            | "filepath"
            | "localdocumentpath"
            | "documentpath"
            | "portalformvalues"
            | "formvalues"
            | "companyprofile"
            | "financialstatements"
    ) {
        return KeyKind::SensitiveRedact;
    }
    KeyKind::Regular
}

fn redact_sensitive_token(token: &str) -> String {
    let trimmed = token.trim_matches(|ch: char| {
        matches!(
            ch,
            ',' | ';' | ':' | ')' | '(' | '[' | ']' | '{' | '}' | '`' | '"' | '\''
        )
    });
    if trimmed.contains("Sentinel-L6-2026")
        || trimmed.contains("@sorelon.test")
        || looks_like_email(trimmed)
    {
        token.replace(trimmed, REDACTED)
    } else {
        token.to_owned()
    }
}

fn looks_like_email(value: &str) -> bool {
    let Some((local, domain)) = value.split_once('@') else {
        return false;
    };
    !local.is_empty()
        && domain.contains('.')
        && !std::path::Path::new(domain)
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("webp"))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{
        REDACTED, redact_bid_error_context, redact_bid_workflow_state_payload,
        redact_sensitive_text,
    };

    #[test]
    fn redacts_nested_bid_payload_values_but_preserves_hashes() {
        let value = json!({
            "bidScopePayload": {
                "processId": "process-raw",
                "lotIds": ["lot-raw"],
                "decisionBasisHash": "abcdef",
                "sideEffectCap": "REMOTE_DRAFT"
            },
            "error_code": "BID_SUBMISSION_FAILED",
            "bid_state": "bid_failed",
            "side_effect_class": "REMOTE_DRAFT"
        });

        let redacted = redact_bid_error_context(&value).to_string();

        assert!(!redacted.contains("process-raw"));
        assert!(!redacted.contains("lot-raw"));
        assert!(redacted.contains("abcdef"));
        assert!(redacted.contains("BID_SUBMISSION_FAILED"));
        assert!(redacted.contains("REMOTE_DRAFT"));
    }

    #[test]
    fn redacts_bid_scope_json_for_sync_payloads() {
        let value = json!({
            "id": "process-raw",
            "bid_scope_json": json!({
                "processId": "process-raw",
                "portalOpportunityRef": "DN-SENTINEL",
                "decisionBasisHash": "hash-ok",
                "sideEffectCap": "REMOTE_DRAFT"
            }).to_string(),
            "row_version": 1
        });

        let redacted = redact_bid_workflow_state_payload(&value).to_string();

        assert!(!redacted.contains("DN-SENTINEL"));
        assert!(redacted.contains("hash-ok"));
        assert!(redacted.contains("REMOTE_DRAFT"));
    }

    #[test]
    fn redacts_sentinel_text_tokens() {
        let redacted = redact_sensitive_text("contact sentinel@sorelon.test-L6-2026 now");

        assert!(redacted.contains(REDACTED));
        assert!(!redacted.contains("sentinel@sorelon"));
    }
}
