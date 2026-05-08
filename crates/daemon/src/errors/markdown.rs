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

fn operator_alert_section(record: &ErrorManifestRecord) -> &'static str {
    if record.error_type == ErrorCode::AgentSuspectOutput {
        "## Operator Alert\n\
         Rejected enforcement-active worker output is quarantined as `AGENT_SUSPECT_OUTPUT`. \
         Do not promote or outbox this worker output; inspect validator evidence and rerun from \
         the last trusted checkpoint.\n\n"
    } else {
        ""
    }
}
