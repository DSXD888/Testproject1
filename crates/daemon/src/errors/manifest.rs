#![forbid(unsafe_code)]

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, ensure};
use chrono::{DateTime, Utc};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[cfg(not(test))]
use crate::errors::markdown;
#[cfg(test)]
use crate::errors_markdown_impl as markdown;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorCode {
    SourceChanged,
    NoticeParseFailed,
    GoodsClassificationLowConfidence,
    SupplierMatchLowConfidence,
    CommodityCodeLowConfidence,
    RevenueBasisUncertain,
    FpoAccessUnavailable,
    AgentStalled,
    AgentAuthChallenge,
    AgentSkillFailed,
    AgentOutputContractInvalid,
    AgentSuspectOutput,
    AgentPartialFailure,
    AgentInternalError,
    AgentRunFailed,
    AgentCleanupIncomplete,
    AgentDaemonCrashDuringSession,
    AgentProtocolMismatch,
    SourceSoftBlocked,
    SourceCaptchaBlocked,
    AuthStateStale,
    PageShapeDrift,
    LocaleOrCurrencyDrift,
    HostSleepInterrupted,
    HostResourceExhausted,
    SkillDirectoryInvalid,
    SqliteWriteFailed,
    DaemonStartupFailed,
}

impl ErrorCode {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SourceChanged => "SOURCE_CHANGED",
            Self::NoticeParseFailed => "NOTICE_PARSE_FAILED",
            Self::GoodsClassificationLowConfidence => "GOODS_CLASSIFICATION_LOW_CONFIDENCE",
            Self::SupplierMatchLowConfidence => "SUPPLIER_MATCH_LOW_CONFIDENCE",
            Self::CommodityCodeLowConfidence => "COMMODITY_CODE_LOW_CONFIDENCE",
            Self::RevenueBasisUncertain => "REVENUE_BASIS_UNCERTAIN",
            Self::FpoAccessUnavailable => "FPO_ACCESS_UNAVAILABLE",
            Self::AgentStalled => "AGENT_STALLED",
            Self::AgentAuthChallenge => "AGENT_AUTH_CHALLENGE",
            Self::AgentSkillFailed => "AGENT_SKILL_FAILED",
            Self::AgentOutputContractInvalid => "AGENT_OUTPUT_CONTRACT_INVALID",
            Self::AgentSuspectOutput => "AGENT_SUSPECT_OUTPUT",
            Self::AgentPartialFailure => "AGENT_PARTIAL_FAILURE",
            Self::AgentInternalError => "AGENT_INTERNAL_ERROR",
            Self::AgentRunFailed => "AGENT_RUN_FAILED",
            Self::AgentCleanupIncomplete => "AGENT_CLEANUP_INCOMPLETE",
            Self::AgentDaemonCrashDuringSession => "AGENT_DAEMON_CRASH_DURING_SESSION",
            Self::AgentProtocolMismatch => "AGENT_PROTOCOL_MISMATCH",
            Self::SourceSoftBlocked => "SOURCE_SOFT_BLOCKED",
            Self::SourceCaptchaBlocked => "SOURCE_CAPTCHA_BLOCKED",
            Self::AuthStateStale => "AUTH_STATE_STALE",
            Self::PageShapeDrift => "PAGE_SHAPE_DRIFT",
            Self::LocaleOrCurrencyDrift => "LOCALE_OR_CURRENCY_DRIFT",
            Self::HostSleepInterrupted => "HOST_SLEEP_INTERRUPTED",
            Self::HostResourceExhausted => "HOST_RESOURCE_EXHAUSTED",
            Self::SkillDirectoryInvalid => "SKILL_DIRECTORY_INVALID",
            Self::SqliteWriteFailed => "SQLITE_WRITE_FAILED",
            Self::DaemonStartupFailed => "DAEMON_STARTUP_FAILED",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorRuntimeContext {
    pub cli_vendor: Option<String>,
    pub cli_version: Option<String>,
    pub adapter_version: Option<String>,
    pub model_id: Option<String>,
    pub config_hash: Option<String>,
    pub skill_version: Option<String>,
    pub pid: Option<u32>,
    pub pgid: Option<u32>,
    pub cleanup_verdict: Option<String>,
    pub last_valid_checkpoint_sequence: Option<u32>,
    pub daemon_instance_id: Option<String>,
    pub commit_seq: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorManifestDraft {
    pub job_id: Option<String>,
    pub process_id: Option<String>,
    pub pipeline_stage: String,
    pub workflow_step: String,
    pub error_type: ErrorCode,
    pub error_message: String,
    pub error_context: Value,
    pub skill_path: Option<String>,
    pub agent_session_id: Option<String>,
    pub runtime_context: ErrorRuntimeContext,
    pub session_artifact_path: Option<String>,
    pub next_debugging_action: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorManifestRecord {
    pub id: String,
    pub job_id: Option<String>,
    pub process_id: Option<String>,
    pub pipeline_stage: String,
    pub workflow_step: String,
    pub error_type: ErrorCode,
    pub error_message: String,
    pub error_context: Value,
    pub skill_path: Option<String>,
    pub agent_session_id: Option<String>,
    pub markdown_path: PathBuf,
    pub session_artifact_path: Option<String>,
    pub next_debugging_action: String,
    pub resolved: bool,
    pub resolved_at: Option<String>,
    pub resolved_by: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredErrorManifest {
    pub id: String,
    pub markdown_path: PathBuf,
}

pub struct ErrorManifestStore {
    root: PathBuf,
}

impl ErrorManifestStore {
    pub fn new(root: impl AsRef<Path>) -> Result<Self> {
        let root = root.as_ref().to_path_buf();
        fs::create_dir_all(&root)?;
        Ok(Self { root })
    }

    pub fn create(
        &self,
        conn: &Connection,
        draft: &ErrorManifestDraft,
        created_at: DateTime<Utc>,
    ) -> Result<StoredErrorManifest> {
        validate_draft(draft)?;
        let id = format!("error-{}", Uuid::new_v4());
        let markdown_path = self.markdown_path(&id, created_at)?;
        let record = ErrorManifestRecord {
            id: id.clone(),
            job_id: draft.job_id.clone(),
            process_id: draft.process_id.clone(),
            pipeline_stage: draft.pipeline_stage.clone(),
            workflow_step: draft.workflow_step.clone(),
            error_type: draft.error_type,
            error_message: draft.error_message.clone(),
            error_context: draft.error_context.clone(),
            skill_path: draft.skill_path.clone(),
            agent_session_id: draft.agent_session_id.clone(),
            markdown_path: markdown_path.clone(),
            session_artifact_path: draft.session_artifact_path.clone(),
            next_debugging_action: draft.next_debugging_action.clone(),
            resolved: false,
            resolved_at: None,
            resolved_by: None,
            created_at: created_at.to_rfc3339(),
        };

        let markdown = markdown::render_error_manifest_markdown(&record, &draft.runtime_context)?;
        write_markdown(&markdown_path, &markdown)?;

        conn.execute(
            "\
            INSERT INTO error_manifests (
              id, job_id, process_id, pipeline_stage, workflow_step, error_type,
              error_message, error_context, skill_path, agent_session_id, markdown_path,
              resolved, resolved_at, resolved_by, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, 0, NULL, NULL, ?12)
            ",
            params![
                id,
                draft.job_id,
                draft.process_id,
                draft.pipeline_stage,
                draft.workflow_step,
                draft.error_type.as_str(),
                draft.error_message,
                draft.error_context.to_string(),
                draft.skill_path,
                draft.agent_session_id,
                markdown_path.to_string_lossy(),
                created_at.to_rfc3339(),
            ],
        )
        .context("inserting error_manifests row")?;

        Ok(StoredErrorManifest { id, markdown_path })
    }

    pub fn resolve(
        conn: &Connection,
        manifest_id: &str,
        resolved_by: &str,
        resolved_at: DateTime<Utc>,
    ) -> Result<()> {
        let updated = conn.execute(
            "\
            UPDATE error_manifests
            SET resolved = 1,
                resolved_at = ?1,
                resolved_by = ?2
            WHERE id = ?3
            ",
            params![resolved_at.to_rfc3339(), resolved_by, manifest_id],
        )?;
        ensure!(updated == 1, "error manifest missing for {manifest_id}");
        Ok(())
    }

    pub fn load_record(conn: &Connection, manifest_id: &str) -> Result<ErrorManifestRecord> {
        conn.query_row(
            "\
            SELECT id, job_id, process_id, pipeline_stage, workflow_step, error_type,
                   error_message, error_context, skill_path, agent_session_id, markdown_path,
                   resolved, resolved_at, resolved_by, created_at
            FROM error_manifests WHERE id = ?1
            ",
            params![manifest_id],
            |row| {
                let error_type = parse_error_code(&row.get::<_, String>(5)?).map_err(|error| {
                    rusqlite::Error::FromSqlConversionFailure(
                        5,
                        rusqlite::types::Type::Text,
                        Box::new(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            error.to_string(),
                        )),
                    )
                })?;
                let error_context_text: String = row.get(7)?;
                let error_context =
                    serde_json::from_str::<Value>(&error_context_text).map_err(|error| {
                        rusqlite::Error::FromSqlConversionFailure(
                            error_context_text.len(),
                            rusqlite::types::Type::Text,
                            Box::new(error),
                        )
                    })?;
                Ok(ErrorManifestRecord {
                    id: row.get(0)?,
                    job_id: row.get(1)?,
                    process_id: row.get(2)?,
                    pipeline_stage: row.get(3)?,
                    workflow_step: row.get(4)?,
                    error_type,
                    error_message: row.get(6)?,
                    error_context,
                    skill_path: row.get(8)?,
                    agent_session_id: row.get(9)?,
                    markdown_path: PathBuf::from(row.get::<_, String>(10)?),
                    session_artifact_path: None,
                    next_debugging_action:
                        "See linked session artifacts and rerun after resolving the root cause."
                            .to_owned(),
                    resolved: row.get::<_, i64>(11)? != 0,
                    resolved_at: row.get(12)?,
                    resolved_by: row.get(13)?,
                    created_at: row.get(14)?,
                })
            },
        )
        .optional()?
        .ok_or_else(|| anyhow!("error manifest missing for {manifest_id}"))
    }

    fn markdown_path(&self, manifest_id: &str, created_at: DateTime<Utc>) -> Result<PathBuf> {
        let dir = self
            .root
            .join("error-manifests")
            .join(created_at.format("%Y").to_string())
            .join(created_at.format("%m").to_string());
        fs::create_dir_all(&dir)?;
        Ok(dir.join(format!("{manifest_id}.md")))
    }
}

fn validate_draft(draft: &ErrorManifestDraft) -> Result<()> {
    ensure!(
        !draft.pipeline_stage.trim().is_empty(),
        "pipeline_stage must not be empty"
    );
    ensure!(
        !draft.workflow_step.trim().is_empty(),
        "workflow_step must not be empty"
    );
    ensure!(
        !draft.error_message.trim().is_empty(),
        "error_message must not be empty"
    );
    ensure!(
        !draft.next_debugging_action.trim().is_empty(),
        "next_debugging_action must not be empty"
    );
    Ok(())
}

fn write_markdown(path: &Path, markdown: &str) -> Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .with_context(|| format!("creating error manifest markdown {}", path.display()))?;
    file.write_all(markdown.as_bytes())?;
    file.flush()?;
    Ok(())
}

fn parse_error_code(raw: &str) -> Result<ErrorCode> {
    let code = match raw {
        "SOURCE_CHANGED" => ErrorCode::SourceChanged,
        "NOTICE_PARSE_FAILED" => ErrorCode::NoticeParseFailed,
        "GOODS_CLASSIFICATION_LOW_CONFIDENCE" => ErrorCode::GoodsClassificationLowConfidence,
        "SUPPLIER_MATCH_LOW_CONFIDENCE" => ErrorCode::SupplierMatchLowConfidence,
        "COMMODITY_CODE_LOW_CONFIDENCE" => ErrorCode::CommodityCodeLowConfidence,
        "REVENUE_BASIS_UNCERTAIN" => ErrorCode::RevenueBasisUncertain,
        "FPO_ACCESS_UNAVAILABLE" => ErrorCode::FpoAccessUnavailable,
        "AGENT_STALLED" => ErrorCode::AgentStalled,
        "AGENT_AUTH_CHALLENGE" => ErrorCode::AgentAuthChallenge,
        "AGENT_SKILL_FAILED" => ErrorCode::AgentSkillFailed,
        "AGENT_OUTPUT_CONTRACT_INVALID" => ErrorCode::AgentOutputContractInvalid,
        "AGENT_SUSPECT_OUTPUT" => ErrorCode::AgentSuspectOutput,
        "AGENT_PARTIAL_FAILURE" => ErrorCode::AgentPartialFailure,
        "AGENT_INTERNAL_ERROR" => ErrorCode::AgentInternalError,
        "AGENT_RUN_FAILED" => ErrorCode::AgentRunFailed,
        "AGENT_CLEANUP_INCOMPLETE" => ErrorCode::AgentCleanupIncomplete,
        "AGENT_DAEMON_CRASH_DURING_SESSION" => ErrorCode::AgentDaemonCrashDuringSession,
        "AGENT_PROTOCOL_MISMATCH" => ErrorCode::AgentProtocolMismatch,
        "SOURCE_SOFT_BLOCKED" => ErrorCode::SourceSoftBlocked,
        "SOURCE_CAPTCHA_BLOCKED" => ErrorCode::SourceCaptchaBlocked,
        "AUTH_STATE_STALE" => ErrorCode::AuthStateStale,
        "PAGE_SHAPE_DRIFT" => ErrorCode::PageShapeDrift,
        "LOCALE_OR_CURRENCY_DRIFT" => ErrorCode::LocaleOrCurrencyDrift,
        "HOST_SLEEP_INTERRUPTED" => ErrorCode::HostSleepInterrupted,
        "HOST_RESOURCE_EXHAUSTED" => ErrorCode::HostResourceExhausted,
        "SKILL_DIRECTORY_INVALID" => ErrorCode::SkillDirectoryInvalid,
        "SQLITE_WRITE_FAILED" => ErrorCode::SqliteWriteFailed,
        "DAEMON_STARTUP_FAILED" => ErrorCode::DaemonStartupFailed,
        other => return Err(anyhow!("unknown error code `{other}`")),
    };
    Ok(code)
}
