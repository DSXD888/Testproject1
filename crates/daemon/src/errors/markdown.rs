#![forbid(unsafe_code)]

use anyhow::Result;

#[cfg(not(test))]
use crate::errors::manifest;
#[cfg(test)]
use crate::errors_manifest_impl as manifest;

use manifest::{ErrorCode, ErrorManifestRecord, ErrorRuntimeContext};

pub fn render_error_manifest_markdown(
    record: &ErrorManifestRecord,
    runtime_context: &ErrorRuntimeContext,
) -> Result<String> {
    let session_artifact_path = record
        .session_artifact_path
        .as_deref()
        .unwrap_or("not available");
    let resolved_at = record.resolved_at.as_deref().unwrap_or("unresolved");
    let resolved_by = record.resolved_by.as_deref().unwrap_or("unassigned");
    let operator_alert = operator_alert_section(record);

    Ok(format!(
        "# Error Manifest: {error_type}\n\n\
         ## Pipeline Summary\n\
         - pipeline_stage: {pipeline_stage}\n\
         - workflow_step: {workflow_step}\n\
         - error_message: {error_message}\n\
         - created_at: {created_at}\n\n\
         ## Runtime Context\n\
         | Field | Value |\n\
         | --- | --- |\n\
         | cli_vendor | {cli_vendor} |\n\
         | cli_version | {cli_version} |\n\
         | adapter_version | {adapter_version} |\n\
         | model_id | {model_id} |\n\
         | config_hash | {config_hash} |\n\
         | skill_version | {skill_version} |\n\
         | pid | {pid} |\n\
         | pgid | {pgid} |\n\
         | cleanup_verdict | {cleanup_verdict} |\n\
         | last_valid_checkpoint_sequence | {last_valid_checkpoint_sequence} |\n\
         | daemon_instance_id | {daemon_instance_id} |\n\
         | commit_seq | {commit_seq} |\n\n\
         ## Error Context\n\
         ```json\n{error_context}\n```\n\n\
         ## Linked Artifacts\n\
         - markdown_path: {markdown_path}\n\
         - session_artifact_path: {session_artifact_path}\n\n\
         {operator_alert}\
         ## Next Debugging Action\n\
         {next_debugging_action}\n\n\
         ## Resolution\n\
         - resolved: {resolved}\n\
         - resolved_at: {resolved_at}\n\
         - resolved_by: {resolved_by}\n",
        error_type = record.error_type.as_str(),
        pipeline_stage = record.pipeline_stage,
        workflow_step = record.workflow_step,
        error_message = record.error_message,
        created_at = record.created_at,
        cli_vendor = runtime_context.cli_vendor.as_deref().unwrap_or("unknown"),
        cli_version = runtime_context.cli_version.as_deref().unwrap_or("unknown"),
        adapter_version = runtime_context
            .adapter_version
            .as_deref()
            .unwrap_or("unknown"),
        model_id = runtime_context.model_id.as_deref().unwrap_or("unknown"),
        config_hash = runtime_context.config_hash.as_deref().unwrap_or("unknown"),
        skill_version = runtime_context
            .skill_version
            .as_deref()
            .unwrap_or("unknown"),
        pid = runtime_context
            .pid
            .map_or_else(|| "unknown".to_owned(), |value| value.to_string()),
        pgid = runtime_context
            .pgid
            .map_or_else(|| "unknown".to_owned(), |value| value.to_string()),
        cleanup_verdict = runtime_context
            .cleanup_verdict
            .as_deref()
            .unwrap_or("unknown"),
        last_valid_checkpoint_sequence = runtime_context
            .last_valid_checkpoint_sequence
            .map_or_else(|| "unknown".to_owned(), |value| value.to_string()),
        daemon_instance_id = runtime_context
            .daemon_instance_id
            .as_deref()
            .unwrap_or("unknown"),
        commit_seq = runtime_context
            .commit_seq
            .map_or_else(|| "unknown".to_owned(), |value| value.to_string()),
        error_context = serde_json::to_string_pretty(&record.error_context)?,
        markdown_path = record.markdown_path.display(),
        session_artifact_path = session_artifact_path,
        operator_alert = operator_alert,
        next_debugging_action = record.next_debugging_action,
        resolved = record.resolved,
        resolved_at = resolved_at,
        resolved_by = resolved_by,
    ))
}

const fn operator_alert_section(record: &ErrorManifestRecord) -> &'static str {
    match record.error_type {
        ErrorCode::ClassificationAmbiguous => {
            "## Operator Alert\n\
             `CLASSIFICATION_AMBIGUOUS`: Top HS6 candidates diverged across recall sources; block auto-qualification and escalate classification to a broker.\n\n"
        }
        ErrorCode::TariffBranchDivergence => {
            "## Operator Alert\n\
             `TARIFF_BRANCH_DIVERGENCE`: UK10 branches split on remedy/no-remedy or duty type; treat this as a hard blocker, not a scenario band.\n\n"
        }
        ErrorCode::MeasureInteractionUnknown => {
            "## Operator Alert\n\
             `MEASURE_INTERACTION_UNKNOWN`: Active measure interaction rules are unclear; require manual customs review before any auto-decision.\n\n"
        }
        ErrorCode::ExporterUnresolved => {
            "## Operator Alert\n\
             `EXPORTER_UNRESOLVED`: A trade-remedy exporter or manufacturer is unresolved; apply the residual rate and alert the operator.\n\n"
        }
        ErrorCode::SupplementaryUnitMissing => {
            "## Operator Alert\n\
             `SUPPLEMENTARY_UNIT_MISSING`: Required supplementary-unit evidence is unavailable; abstain on that duty component until evidence exists.\n\n"
        }
        ErrorCode::ValuationModeConflict => {
            "## Operator Alert\n\
             `VALUATION_MODE_CONFLICT`: Material proxy and declaration valuation diverge; preserve the uncertainty range for operator review.\n\n"
        }
        ErrorCode::FpoAccessUnavailable => {
            "## Operator Alert\n\
             `FPO_ACCESS_UNAVAILABLE`: FPO or search-suggestion access is unavailable; continue with remaining recall sources and apply the confidence penalty.\n\n"
        }
        ErrorCode::TradeRemedyWatchStale => {
            "## Operator Alert\n\
             `TRADE_REMEDY_WATCH_STALE`: Trade-remedy watch freshness expired for the cached classification; reclassify before relying on landed cost.\n\n"
        }
        ErrorCode::FxRateMissing => {
            "## Operator Alert\n\
             `FX_RATE_MISSING`: No valid HMRC customs FX snapshot is available; block landed-cost computation until the customs rate is present.\n\n"
        }
        ErrorCode::FreightEstimateFailed => {
            "## Operator Alert\n\
             `FREIGHT_ESTIMATE_FAILED`: Neither freight provider produced a quote; require a manual freight estimate before decision-grade margin use.\n\n"
        }
        ErrorCode::AgentSuspectOutput => {
            "## Operator Alert\n\
         Rejected enforcement-active worker output is quarantined as `AGENT_SUSPECT_OUTPUT`. \
         Do not promote or outbox this worker output; inspect validator evidence and rerun from \
         the last trusted checkpoint.\n\n"
        }
        ErrorCode::SourceChanged
        | ErrorCode::NoticeParseFailed
        | ErrorCode::ExtractionEncryptedPdf
        | ErrorCode::ExtractionEmptyPdf
        | ErrorCode::ExtractionCorruptPdf
        | ErrorCode::ExtractionEngineUnavailable
        | ErrorCode::ExtractionTimeout
        | ErrorCode::ExtractionOom
        | ErrorCode::GoodsClassificationLowConfidence
        | ErrorCode::SupplierMatchLowConfidence
        | ErrorCode::CommodityCodeLowConfidence
        | ErrorCode::RevenueBasisUncertain
        | ErrorCode::AgentStalled
        | ErrorCode::AgentAuthChallenge
        | ErrorCode::AgentSkillFailed
        | ErrorCode::AgentOutputContractInvalid
        | ErrorCode::AgentPartialFailure
        | ErrorCode::AgentInternalError
        | ErrorCode::AgentRunFailed
        | ErrorCode::AgentCleanupIncomplete
        | ErrorCode::AgentDaemonCrashDuringSession
        | ErrorCode::AgentProtocolMismatch
        | ErrorCode::SourceSoftBlocked
        | ErrorCode::SourceCaptchaBlocked
        | ErrorCode::AuthStateStale
        | ErrorCode::PageShapeDrift
        | ErrorCode::LocaleOrCurrencyDrift
        | ErrorCode::HostSleepInterrupted
        | ErrorCode::HostResourceExhausted
        | ErrorCode::SkillDirectoryInvalid
        | ErrorCode::SqliteWriteFailed
        | ErrorCode::DaemonStartupFailed => "",
    }
}
