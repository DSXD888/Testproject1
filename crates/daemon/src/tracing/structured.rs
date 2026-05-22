#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::agents::adapter::{AuthChallengeKind, CapabilityReport, ControlEvent, ExecutionMode};
#[cfg(not(test))]
use crate::errors::manifest::ErrorCode;

pub const STRUCTURED_TRACE_SCHEMA_VERSION: u32 = 1;
pub const CONTROL_PROTOCOL_RETENTION_DAYS: u32 = 30;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StructuredControlEventKind {
    Spawn,
    Signal,
    Exit,
    Reap,
    CleanupVerdict,
    ResourceGranted,
    ResourceDenied,
    ResourceReleased,
    CircuitTransition,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StructuredProtocolEventKind {
    AdapterStarted,
    Checkpoint,
    Progress,
    AuthChallenge,
    Warning,
    FinalResultReady,
    ResultContractReady,
    ParserError,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraceIdentity {
    pub job_id: String,
    pub session_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub process_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_manifest_hash: Option<String>,
}

impl TraceIdentity {
    #[must_use]
    pub fn new(job_id: impl Into<String>, session_id: impl Into<String>) -> Self {
        Self {
            job_id: job_id.into(),
            session_id: session_id.into(),
            process_id: None,
            run_manifest_hash: None,
        }
    }

    #[must_use]
    pub fn with_process_id(mut self, process_id: impl Into<String>) -> Self {
        self.process_id = Some(process_id.into());
        self
    }

    #[must_use]
    pub fn with_run_manifest_hash(mut self, run_manifest_hash: impl Into<String>) -> Self {
        self.run_manifest_hash = Some(run_manifest_hash.into());
        self
    }
}

#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilitySnapshot {
    pub cli_version: String,
    pub auth_healthy: bool,
    pub structured_output: bool,
    pub pty_viable: bool,
    pub browser_tools: bool,
    pub is_canary: bool,
}

impl From<&CapabilityReport> for CapabilitySnapshot {
    fn from(value: &CapabilityReport) -> Self {
        Self {
            cli_version: value.cli_version.clone(),
            auth_healthy: value.auth_healthy,
            structured_output: value.structured_output,
            pty_viable: value.pty_viable,
            browser_tools: value.browser_tools,
            is_canary: value.is_canary,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetentionPolicy {
    pub control_protocol_days: u32,
}

impl Default for RetentionPolicy {
    fn default() -> Self {
        Self {
            control_protocol_days: CONTROL_PROTOCOL_RETENTION_DAYS,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StructuredControlEventRecord {
    pub seq: u64,
    pub event: StructuredControlEventKind,
    pub identity: TraceIdentity,
    pub execution_mode: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub at: Option<String>,
    pub details: Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StructuredProtocolEventRecord {
    pub seq: u64,
    pub event: StructuredProtocolEventKind,
    pub identity: TraceIdentity,
    pub execution_mode: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub at: Option<String>,
    pub details: Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StructuredTrace {
    pub schema_version: u32,
    pub execution_mode: String,
    pub phase_history: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capability_report: Option<CapabilitySnapshot>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub control_events: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub control_event_records: Vec<StructuredControlEventRecord>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub protocol_events: Vec<StructuredProtocolEventRecord>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub crash_recovered: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retention_policy: Option<RetentionPolicy>,
    #[serde(flatten, default, skip_serializing_if = "BTreeMap::is_empty")]
    pub additional_fields: BTreeMap<String, Value>,
}

impl StructuredTrace {
    #[must_use]
    pub fn new(
        execution_mode: ExecutionMode,
        phase_history: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            schema_version: STRUCTURED_TRACE_SCHEMA_VERSION,
            execution_mode: execution_mode_label(execution_mode).to_owned(),
            phase_history: phase_history.into_iter().map(Into::into).collect(),
            capability_report: None,
            control_events: Vec::new(),
            control_event_records: Vec::new(),
            protocol_events: Vec::new(),
            crash_recovered: false,
            retention_policy: Some(RetentionPolicy::default()),
            additional_fields: BTreeMap::new(),
        }
    }

    #[must_use]
    pub fn with_capability_report(mut self, capability_report: &CapabilityReport) -> Self {
        self.capability_report = Some(CapabilitySnapshot::from(capability_report));
        self
    }

    #[must_use]
    pub const fn mark_crash_recovered(mut self) -> Self {
        self.crash_recovered = true;
        self
    }

    pub fn insert_metadata(&mut self, key: impl Into<String>, value: Value) {
        self.additional_fields.insert(key.into(), value);
    }

    pub fn insert_error_code_metadata(
        &mut self,
        error_code: &(impl StructuredErrorCodeMetadata + ?Sized),
    ) -> Result<()> {
        let code = error_code.error_code();
        let class = error_code
            .error_class()
            .with_context(|| format!("classifying structured error code `{code}`"))?;
        self.insert_metadata("error_code", Value::String(code.to_owned()));
        self.insert_metadata("error_class", Value::String(class.to_owned()));
        Ok(())
    }

    #[cfg(not(test))]
    pub fn insert_manifest_error_code_metadata(&mut self, error_code: ErrorCode) {
        self.insert_metadata("error_code", Value::String(error_code.as_str().to_owned()));
        self.insert_metadata(
            "error_class",
            Value::String(error_code_class(error_code).to_owned()),
        );
    }

    pub fn push_control_record(&mut self, record: StructuredControlEventRecord) {
        self.control_events
            .push(legacy_control_event_name(&record.event).to_owned());
        self.control_event_records.push(record);
    }

    pub fn push_protocol_record(&mut self, record: StructuredProtocolEventRecord) {
        self.protocol_events.push(record);
    }

    #[allow(clippy::too_many_lines)]
    pub fn extend_from_adapter_events(
        &mut self,
        identity: &TraceIdentity,
        events: &[ControlEvent],
    ) {
        let execution_mode = self.execution_mode.clone();
        let mut control_seq = u64::try_from(self.control_event_records.len()).unwrap_or(0);
        let mut protocol_seq = u64::try_from(self.protocol_events.len()).unwrap_or(0);
        for event in events {
            match event {
                ControlEvent::Started {
                    pid,
                    pgid,
                    sid,
                    mode,
                } => {
                    let mode_label = execution_mode_label(*mode).to_owned();
                    control_seq += 1;
                    self.push_control_record(StructuredControlEventRecord {
                        seq: control_seq,
                        event: StructuredControlEventKind::Spawn,
                        identity: identity.clone(),
                        execution_mode: mode_label,
                        at: None,
                        details: json!({
                            "pid": pid,
                            "pgid": pgid,
                            "sid": sid,
                        }),
                    });
                }
                ControlEvent::Exited { status } => {
                    control_seq += 1;
                    self.push_control_record(StructuredControlEventRecord {
                        seq: control_seq,
                        event: StructuredControlEventKind::Exit,
                        identity: identity.clone(),
                        execution_mode: execution_mode.clone(),
                        at: None,
                        details: json!({
                            "success": status.success(),
                            "exit_code": status.exit_code(),
                        }),
                    });
                }
                ControlEvent::Checkpoint { seq, path } => {
                    protocol_seq += 1;
                    self.push_protocol_record(StructuredProtocolEventRecord {
                        seq: protocol_seq,
                        event: StructuredProtocolEventKind::Checkpoint,
                        identity: identity.clone(),
                        execution_mode: execution_mode.clone(),
                        at: None,
                        details: json!({
                            "seq": seq,
                            "path": path,
                        }),
                    });
                }
                ControlEvent::Progress { completed, total } => {
                    protocol_seq += 1;
                    self.push_protocol_record(StructuredProtocolEventRecord {
                        seq: protocol_seq,
                        event: StructuredProtocolEventKind::Progress,
                        identity: identity.clone(),
                        execution_mode: execution_mode.clone(),
                        at: None,
                        details: json!({
                            "completed": completed,
                            "total": total,
                        }),
                    });
                }
                ControlEvent::AuthChallenge { kind, detail } => {
                    protocol_seq += 1;
                    self.push_protocol_record(StructuredProtocolEventRecord {
                        seq: protocol_seq,
                        event: StructuredProtocolEventKind::AuthChallenge,
                        identity: identity.clone(),
                        execution_mode: execution_mode.clone(),
                        at: None,
                        details: json!({
                            "kind": auth_challenge_kind_label(kind),
                            "detail": detail,
                        }),
                    });
                }
                ControlEvent::Warning { code, detail } => {
                    protocol_seq += 1;
                    self.push_protocol_record(StructuredProtocolEventRecord {
                        seq: protocol_seq,
                        event: StructuredProtocolEventKind::Warning,
                        identity: identity.clone(),
                        execution_mode: execution_mode.clone(),
                        at: None,
                        details: json!({
                            "code": code,
                            "detail": detail,
                        }),
                    });
                }
                ControlEvent::FinalResultReady { path } => {
                    protocol_seq += 1;
                    self.push_protocol_record(StructuredProtocolEventRecord {
                        seq: protocol_seq,
                        event: StructuredProtocolEventKind::FinalResultReady,
                        identity: identity.clone(),
                        execution_mode: execution_mode.clone(),
                        at: None,
                        details: json!({
                            "path": path,
                        }),
                    });
                }
            }
        }
    }

    pub fn to_json_string(&self) -> Result<String> {
        serde_json::to_string(self).context("serializing structured trace")
    }

    pub fn to_json_value(&self) -> Result<Value> {
        serde_json::to_value(self).context("serializing structured trace value")
    }

    pub fn parse(raw: &str) -> Result<Self> {
        serde_json::from_str(raw).context("parsing structured trace")
    }

    pub fn crash_recovered(raw: Option<&str>) -> Result<String> {
        let mut trace = raw
            .map(Self::parse)
            .transpose()?
            .unwrap_or_else(|| Self::new(ExecutionMode::Contract, ["created"]));
        trace.crash_recovered = true;
        trace.to_json_string()
    }
}

pub fn parse_phase_history(raw: Option<&str>, fallback_status: &str) -> Vec<String> {
    StructuredTrace::parse(raw.unwrap_or_default())
        .ok()
        .map(|trace| trace.phase_history)
        .filter(|history| !history.is_empty())
        .unwrap_or_else(|| default_phase_history_for_status(fallback_status))
}

pub fn default_phase_history_for_status(status: &str) -> Vec<String> {
    match status {
        "spawned" => vec!["created".to_owned(), "spawned".to_owned()],
        "active" => vec![
            "created".to_owned(),
            "spawned".to_owned(),
            "active".to_owned(),
        ],
        "terminating" => vec![
            "created".to_owned(),
            "spawned".to_owned(),
            "active".to_owned(),
            "terminating".to_owned(),
        ],
        "exited" => vec![
            "created".to_owned(),
            "spawned".to_owned(),
            "active".to_owned(),
            "terminating".to_owned(),
            "exited".to_owned(),
        ],
        "reaped" => vec![
            "created".to_owned(),
            "spawned".to_owned(),
            "active".to_owned(),
            "terminating".to_owned(),
            "exited".to_owned(),
            "reaped".to_owned(),
        ],
        "archived" | "completed" | "failed" => vec![
            "created".to_owned(),
            "spawned".to_owned(),
            "active".to_owned(),
            "terminating".to_owned(),
            "exited".to_owned(),
            "reaped".to_owned(),
            "archived".to_owned(),
        ],
        _ => vec!["created".to_owned()],
    }
}

pub fn add_root_string_field(raw: &str, key: &str, value: &str) -> Result<String> {
    let mut trace = StructuredTrace::parse(raw)?;
    trace.insert_metadata(key, Value::String(value.to_owned()));
    trace.to_json_string()
}

pub trait StructuredErrorCodeMetadata {
    fn error_code(&self) -> &str;

    fn error_class(&self) -> Option<&'static str> {
        error_code_class_for_str(self.error_code())
    }
}

impl StructuredErrorCodeMetadata for str {
    fn error_code(&self) -> &str {
        self
    }
}

impl StructuredErrorCodeMetadata for &str {
    fn error_code(&self) -> &str {
        self
    }
}

#[cfg(not(test))]
impl StructuredErrorCodeMetadata for ErrorCode {
    fn error_code(&self) -> &str {
        self.as_str()
    }

    fn error_class(&self) -> Option<&'static str> {
        Some(error_code_class(*self))
    }
}

#[must_use]
pub fn error_code_class_for_str(error_code: &str) -> Option<&'static str> {
    let class = match error_code {
        "SOURCE_CHANGED" | "NOTICE_PARSE_FAILED" => "source_change",
        "EXTRACTION_ENCRYPTED_PDF"
        | "EXTRACTION_EMPTY_PDF"
        | "EXTRACTION_CORRUPT_PDF"
        | "EXTRACTION_ENGINE_UNAVAILABLE"
        | "EXTRACTION_TIMEOUT"
        | "EXTRACTION_OOM" => "extraction_failure",
        "GOODS_CLASSIFICATION_LOW_CONFIDENCE"
        | "SUPPLIER_MATCH_LOW_CONFIDENCE"
        | "COMMODITY_CODE_LOW_CONFIDENCE"
        | "REVENUE_BASIS_UNCERTAIN" => "low_confidence",
        "CLASSIFICATION_AMBIGUOUS"
        | "TARIFF_BRANCH_DIVERGENCE"
        | "MEASURE_INTERACTION_UNKNOWN"
        | "TRADE_REMEDY_WATCH_STALE"
        | "FX_RATE_MISSING" => "decision_grade_blocker",
        "EXPORTER_UNRESOLVED" | "SUPPLEMENTARY_UNIT_MISSING" | "FREIGHT_ESTIMATE_FAILED" => {
            "evidence_gap"
        }
        "VALUATION_MODE_CONFLICT" => "valuation_conflict",
        "FPO_ACCESS_UNAVAILABLE" => "recall_source_unavailable",
        "BID_SUBMISSION_FAILED"
        | "BID_DUPLICATE_DETECTED"
        | "BID_SIDE_EFFECT_CAP_EXCEEDED"
        | "BID_READINESS_GATE_FAILED"
        | "BID_PORTAL_CHANGED"
        | "BID_PORTAL_AUTH_EXPIRED" => "portal_workflow",
        "AGENT_STALLED"
        | "AGENT_AUTH_CHALLENGE"
        | "AGENT_SKILL_FAILED"
        | "AGENT_OUTPUT_CONTRACT_INVALID"
        | "AGENT_SUSPECT_OUTPUT"
        | "AGENT_PARTIAL_FAILURE"
        | "AGENT_INTERNAL_ERROR"
        | "AGENT_RUN_FAILED"
        | "AGENT_CLEANUP_INCOMPLETE"
        | "AGENT_DAEMON_CRASH_DURING_SESSION"
        | "AGENT_PROTOCOL_MISMATCH" => "agent_execution_failure",
        "SOURCE_SOFT_BLOCKED"
        | "SOURCE_CAPTCHA_BLOCKED"
        | "AUTH_STATE_STALE"
        | "PAGE_SHAPE_DRIFT"
        | "LOCALE_OR_CURRENCY_DRIFT" => "source_health",
        "HOST_SLEEP_INTERRUPTED" | "HOST_RESOURCE_EXHAUSTED" => "host_environment",
        "SKILL_DIRECTORY_INVALID" | "SQLITE_WRITE_FAILED" | "DAEMON_STARTUP_FAILED" => {
            "daemon_internal"
        }
        _ => return None,
    };
    Some(class)
}

#[cfg(not(test))]
#[must_use]
pub const fn error_code_class(error_code: ErrorCode) -> &'static str {
    match error_code {
        ErrorCode::SourceChanged | ErrorCode::NoticeParseFailed => "source_change",
        ErrorCode::ExtractionEncryptedPdf
        | ErrorCode::ExtractionEmptyPdf
        | ErrorCode::ExtractionCorruptPdf
        | ErrorCode::ExtractionEngineUnavailable
        | ErrorCode::ExtractionTimeout
        | ErrorCode::ExtractionOom => "extraction_failure",
        ErrorCode::GoodsClassificationLowConfidence
        | ErrorCode::SupplierMatchLowConfidence
        | ErrorCode::CommodityCodeLowConfidence
        | ErrorCode::RevenueBasisUncertain => "low_confidence",
        ErrorCode::ClassificationAmbiguous
        | ErrorCode::TariffBranchDivergence
        | ErrorCode::MeasureInteractionUnknown
        | ErrorCode::TradeRemedyWatchStale
        | ErrorCode::FxRateMissing => "decision_grade_blocker",
        ErrorCode::ExporterUnresolved
        | ErrorCode::SupplementaryUnitMissing
        | ErrorCode::FreightEstimateFailed => "evidence_gap",
        ErrorCode::ValuationModeConflict => "valuation_conflict",
        ErrorCode::FpoAccessUnavailable => "recall_source_unavailable",
        ErrorCode::BidSubmissionFailed
        | ErrorCode::BidDuplicateDetected
        | ErrorCode::BidSideEffectCapExceeded
        | ErrorCode::BidReadinessGateFailed
        | ErrorCode::BidPortalChanged
        | ErrorCode::BidPortalAuthExpired => "portal_workflow",
        ErrorCode::AgentStalled
        | ErrorCode::AgentAuthChallenge
        | ErrorCode::AgentSkillFailed
        | ErrorCode::AgentOutputContractInvalid
        | ErrorCode::AgentSuspectOutput
        | ErrorCode::AgentPartialFailure
        | ErrorCode::AgentInternalError
        | ErrorCode::AgentRunFailed
        | ErrorCode::AgentCleanupIncomplete
        | ErrorCode::AgentDaemonCrashDuringSession
        | ErrorCode::AgentProtocolMismatch => "agent_execution_failure",
        ErrorCode::SourceSoftBlocked
        | ErrorCode::SourceCaptchaBlocked
        | ErrorCode::AuthStateStale
        | ErrorCode::PageShapeDrift
        | ErrorCode::LocaleOrCurrencyDrift => "source_health",
        ErrorCode::HostSleepInterrupted | ErrorCode::HostResourceExhausted => "host_environment",
        ErrorCode::SkillDirectoryInvalid
        | ErrorCode::SqliteWriteFailed
        | ErrorCode::DaemonStartupFailed => "daemon_internal",
    }
}

pub fn trace_contains_artifact_ref(raw: &str, artifact_path: &Path) -> Result<bool> {
    let parsed = StructuredTrace::parse(raw)?;
    let candidate = artifact_path.to_string_lossy();
    Ok(parsed
        .additional_fields
        .values()
        .any(|value| contains_string(value, &candidate)))
}

fn contains_string(value: &Value, candidate: &str) -> bool {
    match value {
        Value::String(value) => value == candidate,
        Value::Array(values) => values.iter().any(|entry| contains_string(entry, candidate)),
        Value::Object(object) => object
            .values()
            .any(|entry| contains_string(entry, candidate)),
        _ => false,
    }
}

const fn execution_mode_label(mode: ExecutionMode) -> &'static str {
    match mode {
        ExecutionMode::Pty => "pty",
        ExecutionMode::Contract => "contract",
    }
}

const fn legacy_control_event_name(event: &StructuredControlEventKind) -> &'static str {
    match event {
        StructuredControlEventKind::Spawn => "started",
        StructuredControlEventKind::Signal => "signal",
        StructuredControlEventKind::Exit => "exited",
        StructuredControlEventKind::Reap => "reaped",
        StructuredControlEventKind::CleanupVerdict => "cleanup_verdict",
        StructuredControlEventKind::ResourceGranted => "resource_granted",
        StructuredControlEventKind::ResourceDenied => "resource_denied",
        StructuredControlEventKind::ResourceReleased => "resource_released",
        StructuredControlEventKind::CircuitTransition => "circuit_transition",
    }
}

const fn auth_challenge_kind_label(kind: &AuthChallengeKind) -> &'static str {
    match kind {
        AuthChallengeKind::ReLogin => "re_login",
        AuthChallengeKind::Mfa => "mfa",
        AuthChallengeKind::Sso => "sso",
        AuthChallengeKind::PermissionPrompt => "permission_prompt",
        AuthChallengeKind::Unknown => "unknown",
    }
}

#[allow(clippy::trivially_copy_pass_by_ref)]
const fn is_false(value: &bool) -> bool {
    !*value
}
