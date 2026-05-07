#![forbid(unsafe_code)]
#![allow(dead_code)]

use anyhow::{Result, ensure};
use serde_json::{Value, json};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedactionSentinelFixture {
    pub org_id: String,
    pub process_id: String,
    pub job_id: String,
    pub verdict_id: String,
    pub command_id: String,
    pub operator_identity: String,
    pub company_profile_secret: String,
    pub supplier_raw_secret: String,
    pub tender_metadata_secret: String,
    pub validator_reasoning_secret: String,
    pub full_job_context_secret: String,
    pub command_history_secret: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::struct_field_names)]
pub struct RedactedSummaryPayloads {
    pub workflow_job_summary: Value,
    pub error_manifest_summary: Value,
    pub validation_verdict_summary: Value,
    pub operator_decision_summary: Value,
}

impl Default for RedactionSentinelFixture {
    fn default() -> Self {
        Self {
            org_id: "l4-redaction-org".to_owned(),
            process_id: "l4-redaction-process".to_owned(),
            job_id: "l4-redaction-job".to_owned(),
            verdict_id: "l4-redaction-verdict".to_owned(),
            command_id: "l4-redaction-command".to_owned(),
            operator_identity: "operator-secret:l4@example.test".to_owned(),
            company_profile_secret: "company-profile-secret:vault-ref-42".to_owned(),
            supplier_raw_secret: "supplier-raw-secret:quoted-page-html".to_owned(),
            tender_metadata_secret: "tender-metadata-secret:document-page-map".to_owned(),
            validator_reasoning_secret: "validator-reasoning-secret:chain-of-thought".to_owned(),
            full_job_context_secret: "full-job-context-secret:input-output-payload".to_owned(),
            command_history_secret: "command-history-secret:cas-ledger-response".to_owned(),
        }
    }
}

impl RedactionSentinelFixture {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn sensitive_markers(&self) -> Vec<&str> {
        vec![
            &self.operator_identity,
            &self.company_profile_secret,
            &self.supplier_raw_secret,
            &self.tender_metadata_secret,
            &self.validator_reasoning_secret,
            &self.full_job_context_secret,
            &self.command_history_secret,
        ]
    }

    pub fn source_payloads(&self) -> Value {
        json!({
            "operator_decision_head": {
                "process_id": self.process_id,
                "operator_id": self.operator_identity,
                "command_id": self.command_id,
                "command_history": self.command_history_secret
            },
            "company_profile": {
                "org_id": self.org_id,
                "profile": self.company_profile_secret
            },
            "supplier_quote": {
                "raw_supplier_data": self.supplier_raw_secret
            },
            "tender_document": {
                "raw_metadata": self.tender_metadata_secret
            },
            "validation_verdict": {
                "id": self.verdict_id,
                "reasoning": self.validator_reasoning_secret
            },
            "workflow_job": {
                "id": self.job_id,
                "full_context": self.full_job_context_secret
            }
        })
    }

    pub fn redacted_summary_payloads(&self) -> RedactedSummaryPayloads {
        RedactedSummaryPayloads {
            workflow_job_summary: json!({
                "job_id": self.job_id,
                "org_id": self.org_id,
                "job_kind": "validation",
                "status": "completed",
                "skill_path": "validation/base"
            }),
            error_manifest_summary: json!({
                "manifest_id": "l4-redaction-manifest",
                "org_id": self.org_id,
                "job_id": self.job_id,
                "error_type": "validation",
                "error_code": "AGENT_SUSPECT_OUTPUT",
                "incident_signature": "l4-redaction-incident"
            }),
            validation_verdict_summary: json!({
                "verdict_id": self.verdict_id,
                "org_id": self.org_id,
                "worker_job_id": self.job_id,
                "verdict": "rejected",
                "basis_sufficiency": "high",
                "enforcement_eligible": false
            }),
            operator_decision_summary: json!({
                "process_id": self.process_id,
                "org_id": self.org_id,
                "disposition": "pending",
                "decision_stale": false
            }),
        }
    }

    pub fn assert_no_sentinels(&self, rendered_payload: &str) -> Result<()> {
        for marker in self.sensitive_markers() {
            ensure!(
                !rendered_payload.contains(marker),
                "redacted payload leaked sentinel marker `{marker}`"
            );
        }
        Ok(())
    }
}
