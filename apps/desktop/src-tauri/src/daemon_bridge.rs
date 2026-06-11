#![forbid(unsafe_code)]

use std::fmt;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

pub const DEFAULT_PROTOCOL_VERSION: u32 = 1;
pub const DEFAULT_SCHEMA_EPOCH: i64 = 11;
const DESCRIPTOR_FILENAME: &str = "daemon.info.json";
const DESKTOP_VERSION: &str = "procintel-desktop";
static NEXT_REQUEST_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProtocolRange {
    pub min: u32,
    pub max: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HealthState {
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ActiveSessionSummary {
    pub count: usize,
    pub ids: Vec<String>,
    pub states: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ReconnectOutcome {
    FreshConnection,
    SameDaemonReconnect,
    DaemonRestartInvalidated,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DaemonWireWelcome {
    pub selected_protocol_version: u32,
    pub daemon_version: String,
    pub instance_id: String,
    pub schema_epoch: i64,
    pub commit_seq: i64,
    pub health_state: HealthState,
    pub mutations_allowed: bool,
    pub active_sessions: ActiveSessionSummary,
    pub reconnect_outcome: ReconnectOutcome,
    pub stream_resume_permitted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BridgeError {
    pub code: String,
    pub message: String,
    pub retryable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Value>,
}

impl BridgeError {
    fn transport_unavailable(message: impl Into<String>) -> Self {
        Self {
            code: "transport_unavailable".to_owned(),
            message: message.into(),
            retryable: true,
            details: None,
        }
    }

    fn transport_timeout(message: impl Into<String>) -> Self {
        Self {
            code: "transport_timeout".to_owned(),
            message: message.into(),
            retryable: true,
            details: None,
        }
    }

    fn invalid_params(message: impl Into<String>) -> Self {
        Self {
            code: "invalid_params".to_owned(),
            message: message.into(),
            retryable: false,
            details: None,
        }
    }
}

impl fmt::Display for BridgeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for BridgeError {}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MutationDurability {
    Committed,
    Queued,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RemoteState {
    Committed,
    Queued,
    Conflicted,
    NotApplicable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MutationAck {
    pub accepted: bool,
    pub idempotency_key: String,
    pub durability: MutationDurability,
    pub visible_after_seq: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub job_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    pub remote_state: RemoteState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conflict_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DaemonInfoSnapshot {
    pub instance_id: String,
    pub daemon_version: String,
    pub protocol_version: u32,
    pub schema_epoch: i64,
    pub socket_path: String,
    pub descriptor_path: String,
    pub state_root: String,
    pub state_db_path: String,
    pub artifact_root: String,
    pub wal_bytes: Option<u64>,
    pub started_at: String,
    pub startup_recovery: StartupRecovery,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StartupRecovery {
    pub recovered_interrupted_sessions: usize,
    pub last_recovery_at: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CronDefinitionStatus {
    pub definition_key: String,
    pub source_id: String,
    pub skill_id: String,
    pub skill_version: String,
    pub cron_expression: String,
    pub enabled: bool,
    pub last_run_at: Option<String>,
    pub next_run_at: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CronWorkerLaneStatus {
    pub lane: String,
    pub running: bool,
    pub claimable_kinds: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CronAwaitingAccessStatus {
    pub active_count: i64,
    pub expired_count: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CronStatusSnapshot {
    pub definitions: Vec<CronDefinitionStatus>,
    pub scheduler_running: bool,
    pub last_tick_at: Option<String>,
    pub next_run_at: Option<String>,
    pub worker_lanes: Vec<CronWorkerLaneStatus>,
    pub awaiting_access: CronAwaitingAccessStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum StructuredSessionEvent {
    Started {
        pid: u32,
        pgid: u32,
        sid: u32,
        mode: String,
    },
    Checkpoint {
        checkpoint_seq: u32,
        path: String,
    },
    Progress {
        completed: u32,
        total: Option<u32>,
    },
    AuthChallenge {
        kind: String,
        detail: String,
        intervention: Option<Value>,
    },
    Warning {
        code: String,
        detail: String,
    },
    FinalResultReady {
        path: String,
    },
    Exited {
        exit_code: Option<i32>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum SessionFrame {
    Lifecycle {
        seq: u64,
        state: String,
        cleanup_verdict: Option<String>,
        archive_seq: Option<i64>,
        ts: String,
    },
    Pty {
        seq: u64,
        chunk: Vec<u8>,
        ts: String,
    },
    Stdout {
        seq: u64,
        chunk: String,
        ts: String,
    },
    Stderr {
        seq: u64,
        chunk: String,
        ts: String,
    },
    Event {
        seq: u64,
        event: StructuredSessionEvent,
        ts: String,
    },
    Gap {
        from_seq: u64,
        to_seq: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeSubscriptionEvent {
    Topic {
        topic: String,
        payload: Value,
    },
    Lifecycle {
        seq: u64,
        session_id: String,
        state: String,
        cleanup_verdict: Option<String>,
        archive_seq: Option<i64>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GetDaemonInfoParams {}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GetCronStatusParams {}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TriggerIngestionParams {
    pub source_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TriggerEnrichmentParams {
    pub process_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DecisionDisposition {
    Approve,
    Reject,
    Watch,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaptureDecisionParams {
    pub process_id: String,
    pub disposition: DecisionDisposition,
    pub reviewed_basis_hash: String,
    pub reviewed_manifest_hash: String,
    pub reviewed_at_stage_run_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decision_note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaptureClarificationParams {
    pub process_id: String,
    pub candidate_id: String,
    pub reason_code: String,
    pub payload: Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WithdrawClarificationParams {
    pub process_id: String,
    pub candidate_id: String,
    pub reason_code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WithdrawDecisionParams {
    pub idempotency_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResolveErrorParams {
    pub error_manifest_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StopSessionParams {
    pub session_id: String,
    pub mode: StopMode,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum StopMode {
    Graceful,
    Force,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SendSessionInputParams {
    pub session_id: String,
    pub input_utf8: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResizeSessionParams {
    pub session_id: String,
    pub cols: u16,
    pub rows: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RetryJobParams {
    pub job_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RestartDaemonParams {}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RerunFromErrorParams {
    pub error_manifest_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DaemonCommandMethod {
    GetDaemonInfo,
    GetCronStatus,
    TriggerIngestion,
    TriggerEnrichment,
    CaptureDecision,
    CaptureClarification,
    WithdrawClarification,
    WithdrawDecision,
    ResolveError,
    StopSession,
    SendSessionInput,
    ResizeSession,
    RetryJob,
    RestartDaemon,
    RerunFromError,
}

impl DaemonCommandMethod {
    pub const ALL: [Self; 15] = [
        Self::GetDaemonInfo,
        Self::GetCronStatus,
        Self::TriggerIngestion,
        Self::TriggerEnrichment,
        Self::CaptureDecision,
        Self::CaptureClarification,
        Self::WithdrawClarification,
        Self::WithdrawDecision,
        Self::ResolveError,
        Self::StopSession,
        Self::SendSessionInput,
        Self::ResizeSession,
        Self::RetryJob,
        Self::RestartDaemon,
        Self::RerunFromError,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::GetDaemonInfo => "get_daemon_info",
            Self::GetCronStatus => "get_cron_status",
            Self::TriggerIngestion => "trigger_ingestion",
            Self::TriggerEnrichment => "trigger_enrichment",
            Self::CaptureDecision => "capture_decision",
            Self::CaptureClarification => "capture_clarification",
            Self::WithdrawClarification => "withdraw_clarification",
            Self::WithdrawDecision => "withdraw_decision",
            Self::ResolveError => "resolve_error",
            Self::StopSession => "stop_session",
            Self::SendSessionInput => "send_session_input",
            Self::ResizeSession => "resize_session",
            Self::RetryJob => "retry_job",
            Self::RestartDaemon => "restart_daemon",
            Self::RerunFromError => "rerun_from_error",
        }
    }
}

#[derive(Debug, Clone)]
pub struct DaemonBridgeConfig {
    pub runtime_dir: PathBuf,
    pub desktop_version: String,
    pub supported_protocol_range: ProtocolRange,
    pub expected_schema_epoch: i64,
    pub last_seen_instance_id: Option<String>,
}

impl DaemonBridgeConfig {
    #[must_use]
    pub fn new(runtime_dir: impl Into<PathBuf>) -> Self {
        Self {
            runtime_dir: runtime_dir.into(),
            desktop_version: DESKTOP_VERSION.to_owned(),
            supported_protocol_range: ProtocolRange {
                min: DEFAULT_PROTOCOL_VERSION,
                max: DEFAULT_PROTOCOL_VERSION,
            },
            expected_schema_epoch: DEFAULT_SCHEMA_EPOCH,
            last_seen_instance_id: None,
        }
    }

    fn descriptor_path(&self) -> PathBuf {
        self.runtime_dir.join(DESCRIPTOR_FILENAME)
    }
}

pub struct DaemonBridge {
    config: DaemonBridgeConfig,
}

impl DaemonBridge {
    #[must_use]
    pub fn new(runtime_dir: impl Into<PathBuf>) -> Self {
        Self {
            config: DaemonBridgeConfig::new(runtime_dir),
        }
    }

    #[must_use]
    pub const fn with_config(config: DaemonBridgeConfig) -> Self {
        Self { config }
    }

    pub fn session_frame_from_value(value: Value) -> Result<SessionFrame, BridgeError> {
        serde_json::from_value(value).map_err(|error| {
            BridgeError::transport_unavailable(format!(
                "parsing projected session frame failed: {error}"
            ))
        })
    }

    pub fn connect(&self) -> Result<DaemonBridgeConnection, BridgeError> {
        let descriptor = read_descriptor(&self.config.descriptor_path())?;
        let mut writer = UnixStream::connect(&descriptor.socket_path).map_err(|error| {
            BridgeError::transport_unavailable(format!(
                "opening daemon socket {} failed: {error}",
                descriptor.socket_path
            ))
        })?;
        let reader_stream = writer.try_clone().map_err(|error| {
            BridgeError::transport_unavailable(format!(
                "cloning daemon socket {} failed: {error}",
                descriptor.socket_path
            ))
        })?;
        let mut reader = BufReader::new(reader_stream);

        write_frame(
            &mut writer,
            &DaemonWireRequest::Hello {
                desktop_version: self.config.desktop_version.clone(),
                supported_protocol_range: self.config.supported_protocol_range.clone(),
                expected_schema_epoch: self.config.expected_schema_epoch,
                last_seen_instance_id: self.config.last_seen_instance_id.clone(),
            },
        )?;

        match read_frame(&mut reader)? {
            DaemonWireResponse::Welcome(welcome) => Ok(DaemonBridgeConnection {
                reader,
                writer,
                welcome,
            }),
            DaemonWireResponse::Rejection(rejection) => Err(rejection.into_bridge_error()),
            frame => Err(BridgeError::transport_unavailable(format!(
                "expected welcome or rejection during handshake, received {:?}",
                frame.message_type()
            ))),
        }
    }
}

#[derive(Debug)]
pub struct DaemonBridgeConnection {
    reader: BufReader<UnixStream>,
    writer: UnixStream,
    welcome: DaemonWireWelcome,
}

impl DaemonBridgeConnection {
    #[must_use]
    pub const fn welcome(&self) -> &DaemonWireWelcome {
        &self.welcome
    }

    pub fn set_read_timeout(&self, timeout: Option<Duration>) -> Result<(), BridgeError> {
        self.reader
            .get_ref()
            .set_read_timeout(timeout)
            .map_err(|error| {
                BridgeError::transport_unavailable(format!(
                    "configuring daemon bridge read timeout failed: {error}"
                ))
            })
    }

    pub fn read_subscription_event(
        &mut self,
    ) -> Result<Option<NativeSubscriptionEvent>, BridgeError> {
        match read_frame(&mut self.reader) {
            Ok(DaemonWireResponse::TopicEvent { topic, payload }) => {
                Ok(Some(NativeSubscriptionEvent::Topic { topic, payload }))
            }
            Ok(DaemonWireResponse::Lifecycle {
                seq,
                session_id,
                phase,
                cleanup_verdict,
                archive_seq,
            }) => Ok(Some(NativeSubscriptionEvent::Lifecycle {
                seq,
                session_id,
                state: phase,
                cleanup_verdict,
                archive_seq,
            })),
            Ok(DaemonWireResponse::Error {
                request_id: None,
                code,
                message,
                retryable,
                details,
            }) => Err(BridgeError {
                code,
                message,
                retryable,
                details,
            }),
            Ok(frame) => Err(BridgeError::transport_unavailable(format!(
                "received unexpected frame `{}` while awaiting subscription event",
                frame.message_type()
            ))),
            Err(error) if error.code == "transport_timeout" => Ok(None),
            Err(error) => Err(error),
        }
    }

    pub fn get_daemon_info(&mut self) -> Result<DaemonInfoSnapshot, BridgeError> {
        self.call_readonly(DaemonCommandMethod::GetDaemonInfo, &GetDaemonInfoParams {})
    }

    pub fn get_cron_status(&mut self) -> Result<CronStatusSnapshot, BridgeError> {
        self.call_readonly(DaemonCommandMethod::GetCronStatus, &GetCronStatusParams {})
    }

    pub fn trigger_ingestion(
        &mut self,
        params: &TriggerIngestionParams,
        idempotency_key: &str,
    ) -> Result<MutationAck, BridgeError> {
        self.call_mutation(
            DaemonCommandMethod::TriggerIngestion,
            params,
            idempotency_key,
            None,
        )
    }

    pub fn trigger_enrichment(
        &mut self,
        params: &TriggerEnrichmentParams,
        idempotency_key: &str,
    ) -> Result<MutationAck, BridgeError> {
        self.call_mutation(
            DaemonCommandMethod::TriggerEnrichment,
            params,
            idempotency_key,
            None,
        )
    }

    pub fn capture_decision(
        &mut self,
        params: &CaptureDecisionParams,
        idempotency_key: &str,
    ) -> Result<MutationAck, BridgeError> {
        self.call_mutation(
            DaemonCommandMethod::CaptureDecision,
            params,
            idempotency_key,
            None,
        )
    }

    pub fn capture_clarification(
        &mut self,
        params: &CaptureClarificationParams,
        idempotency_key: &str,
    ) -> Result<MutationAck, BridgeError> {
        self.call_mutation(
            DaemonCommandMethod::CaptureClarification,
            params,
            idempotency_key,
            None,
        )
    }

    pub fn withdraw_clarification(
        &mut self,
        params: &WithdrawClarificationParams,
        idempotency_key: &str,
    ) -> Result<MutationAck, BridgeError> {
        self.call_mutation(
            DaemonCommandMethod::WithdrawClarification,
            params,
            idempotency_key,
            None,
        )
    }

    pub fn withdraw_decision(
        &mut self,
        params: &WithdrawDecisionParams,
        idempotency_key: &str,
    ) -> Result<MutationAck, BridgeError> {
        self.call_mutation(
            DaemonCommandMethod::WithdrawDecision,
            params,
            idempotency_key,
            None,
        )
    }

    pub fn resolve_error(
        &mut self,
        params: &ResolveErrorParams,
        idempotency_key: &str,
    ) -> Result<MutationAck, BridgeError> {
        self.call_mutation(
            DaemonCommandMethod::ResolveError,
            params,
            idempotency_key,
            None,
        )
    }

    pub fn stop_session(
        &mut self,
        params: &StopSessionParams,
        idempotency_key: &str,
    ) -> Result<MutationAck, BridgeError> {
        self.call_mutation(
            DaemonCommandMethod::StopSession,
            params,
            idempotency_key,
            Some(params.session_id.clone()),
        )
    }

    pub fn send_session_input(
        &mut self,
        params: &SendSessionInputParams,
        idempotency_key: &str,
    ) -> Result<MutationAck, BridgeError> {
        self.call_mutation(
            DaemonCommandMethod::SendSessionInput,
            params,
            idempotency_key,
            Some(params.session_id.clone()),
        )
    }

    pub fn resize_session(
        &mut self,
        params: &ResizeSessionParams,
        idempotency_key: &str,
    ) -> Result<MutationAck, BridgeError> {
        self.call_mutation(
            DaemonCommandMethod::ResizeSession,
            params,
            idempotency_key,
            Some(params.session_id.clone()),
        )
    }

    pub fn retry_job(
        &mut self,
        params: &RetryJobParams,
        idempotency_key: &str,
    ) -> Result<MutationAck, BridgeError> {
        self.call_mutation(DaemonCommandMethod::RetryJob, params, idempotency_key, None)
    }

    pub fn restart_daemon(
        &mut self,
        params: &RestartDaemonParams,
        idempotency_key: &str,
    ) -> Result<MutationAck, BridgeError> {
        self.call_mutation(
            DaemonCommandMethod::RestartDaemon,
            params,
            idempotency_key,
            None,
        )
    }

    pub fn rerun_from_error(
        &mut self,
        params: &RerunFromErrorParams,
        idempotency_key: &str,
    ) -> Result<MutationAck, BridgeError> {
        self.call_mutation(
            DaemonCommandMethod::RerunFromError,
            params,
            idempotency_key,
            None,
        )
    }

    pub fn subscribe(&mut self, topic: &str, from_seq: Option<u64>) -> Result<(), BridgeError> {
        let request_id = next_request_id();
        write_frame(
            &mut self.writer,
            &DaemonWireRequest::Subscribe {
                request_id: request_id.clone(),
                topic: topic.to_owned(),
                from_seq,
            },
        )?;
        self.await_empty_response(&request_id)
    }

    pub fn unsubscribe(&mut self, topic: &str) -> Result<(), BridgeError> {
        let request_id = next_request_id();
        write_frame(
            &mut self.writer,
            &DaemonWireRequest::Unsubscribe {
                request_id: request_id.clone(),
                topic: topic.to_owned(),
            },
        )?;
        self.await_empty_response(&request_id)
    }

    fn call_readonly<P, R>(
        &mut self,
        method: DaemonCommandMethod,
        params: &P,
    ) -> Result<R, BridgeError>
    where
        P: Serialize,
        R: DeserializeOwned,
    {
        let request_id = next_request_id();
        write_frame(
            &mut self.writer,
            &DaemonWireRequest::Command {
                request_id: request_id.clone(),
                method: method.as_str().to_owned(),
                params: to_value(params)?,
                idempotency_key: None,
            },
        )?;
        self.await_response(&request_id)
    }

    fn call_mutation<P>(
        &mut self,
        method: DaemonCommandMethod,
        params: &P,
        idempotency_key: &str,
        session_id: Option<String>,
    ) -> Result<MutationAck, BridgeError>
    where
        P: Serialize,
    {
        let request_id = next_request_id();
        write_frame(
            &mut self.writer,
            &DaemonWireRequest::Command {
                request_id: request_id.clone(),
                method: method.as_str().to_owned(),
                params: to_value(params)?,
                idempotency_key: Some(idempotency_key.to_owned()),
            },
        )?;
        let raw: DaemonMutationAck = self.await_response(&request_id)?;
        Ok(MutationAck {
            accepted: raw.accepted,
            idempotency_key: idempotency_key.to_owned(),
            durability: if raw.accepted {
                MutationDurability::Committed
            } else {
                MutationDurability::Queued
            },
            visible_after_seq: raw.visible_after_seq,
            job_id: raw.continuation_job_id,
            session_id,
            remote_state: RemoteState::NotApplicable,
            conflict_reason: None,
        })
    }

    fn await_empty_response(&mut self, request_id: &str) -> Result<(), BridgeError> {
        let payload: Value = self.await_response(request_id)?;
        if payload != Value::Null {
            return Err(BridgeError::transport_unavailable(
                "expected empty response payload for subscription command",
            ));
        }
        Ok(())
    }

    fn await_response<T>(&mut self, request_id: &str) -> Result<T, BridgeError>
    where
        T: DeserializeOwned,
    {
        loop {
            match read_frame(&mut self.reader)? {
                DaemonWireResponse::CommandResponse {
                    request_id: response_id,
                    payload,
                } if response_id == request_id => {
                    return serde_json::from_value(payload).map_err(|error| {
                        BridgeError::transport_unavailable(format!(
                            "parsing command response for {request_id} failed: {error}"
                        ))
                    });
                }
                DaemonWireResponse::Error {
                    request_id: response_id,
                    code,
                    message,
                    retryable,
                    details,
                } if response_id.as_deref() == Some(request_id) => {
                    return Err(BridgeError {
                        code,
                        message,
                        retryable,
                        details,
                    });
                }
                DaemonWireResponse::Lifecycle { .. } | DaemonWireResponse::TopicEvent { .. } => {}
                DaemonWireResponse::CommandResponse {
                    request_id: response_id,
                    ..
                } => {
                    return Err(BridgeError::transport_unavailable(format!(
                        "received response for unexpected request id `{response_id}` while awaiting `{request_id}`"
                    )));
                }
                DaemonWireResponse::Error {
                    request_id: response_id,
                    ..
                } => {
                    return Err(BridgeError::transport_unavailable(format!(
                        "received bridge error for unexpected request id `{response_id:?}` while awaiting `{request_id}`"
                    )));
                }
                frame => {
                    return Err(BridgeError::transport_unavailable(format!(
                        "received unexpected frame `{}` while awaiting `{request_id}`",
                        frame.message_type()
                    )));
                }
            }
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DaemonDescriptor {
    socket_path: String,
}

#[derive(Debug, Deserialize)]
struct DaemonMutationAck {
    accepted: bool,
    #[serde(alias = "visible_after_seq")]
    visible_after_seq: i64,
    #[serde(alias = "continuation_job_id")]
    continuation_job_id: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(
    tag = "messageType",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
enum DaemonWireRequest {
    Hello {
        desktop_version: String,
        supported_protocol_range: ProtocolRange,
        expected_schema_epoch: i64,
        last_seen_instance_id: Option<String>,
    },
    Command {
        request_id: String,
        method: String,
        params: Value,
        idempotency_key: Option<String>,
    },
    Subscribe {
        request_id: String,
        topic: String,
        from_seq: Option<u64>,
    },
    Unsubscribe {
        request_id: String,
        topic: String,
    },
}

#[derive(Debug, Deserialize)]
#[serde(
    tag = "messageType",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
enum DaemonWireResponse {
    Welcome(DaemonWireWelcome),
    Rejection(HandshakeRejection),
    Lifecycle {
        seq: u64,
        session_id: String,
        phase: String,
        cleanup_verdict: Option<String>,
        archive_seq: Option<i64>,
    },
    CommandResponse {
        request_id: String,
        payload: Value,
    },
    Error {
        request_id: Option<String>,
        code: String,
        message: String,
        retryable: bool,
        details: Option<Value>,
    },
    TopicEvent {
        topic: String,
        payload: Value,
    },
}

impl DaemonWireResponse {
    const fn message_type(&self) -> &'static str {
        match self {
            Self::Welcome(_) => "welcome",
            Self::Rejection(_) => "rejection",
            Self::Lifecycle { .. } => "lifecycle",
            Self::CommandResponse { .. } => "commandResponse",
            Self::Error { .. } => "error",
            Self::TopicEvent { .. } => "topicEvent",
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HandshakeRejection {
    code: String,
    message: String,
    supported_protocol_range: Option<ProtocolRange>,
    expected_schema_epoch: Option<i64>,
    actual_schema_epoch: Option<i64>,
}

impl HandshakeRejection {
    fn into_bridge_error(self) -> BridgeError {
        let details = if self.supported_protocol_range.is_some()
            || self.expected_schema_epoch.is_some()
            || self.actual_schema_epoch.is_some()
        {
            Some(json!({
                "supportedProtocolRange": self.supported_protocol_range,
                "expectedSchemaEpoch": self.expected_schema_epoch,
                "actualSchemaEpoch": self.actual_schema_epoch,
            }))
        } else {
            None
        };

        BridgeError {
            code: self.code.clone(),
            retryable: false,
            message: self.message,
            details,
        }
    }
}

fn read_descriptor(path: &Path) -> Result<DaemonDescriptor, BridgeError> {
    let raw = fs::read_to_string(path).map_err(|error| {
        BridgeError::transport_unavailable(format!(
            "reading daemon descriptor {} failed: {error}",
            path.display()
        ))
    })?;
    serde_json::from_str(&raw).map_err(|error| {
        BridgeError::transport_unavailable(format!(
            "parsing daemon descriptor {} failed: {error}",
            path.display()
        ))
    })
}

fn write_frame<T: Serialize>(stream: &mut UnixStream, message: &T) -> Result<(), BridgeError> {
    let payload = serde_json::to_string(message).map_err(|error| {
        BridgeError::transport_unavailable(format!("serializing daemon wire frame failed: {error}"))
    })?;
    stream
        .write_all(payload.as_bytes())
        .map_err(|error| BridgeError::transport_unavailable(error.to_string()))?;
    stream
        .write_all(b"\n")
        .map_err(|error| BridgeError::transport_unavailable(error.to_string()))?;
    stream
        .flush()
        .map_err(|error| BridgeError::transport_unavailable(error.to_string()))
}

fn read_frame(reader: &mut impl BufRead) -> Result<DaemonWireResponse, BridgeError> {
    let mut buffer = String::new();
    let bytes = reader.read_line(&mut buffer).map_err(|error| {
        if matches!(
            error.kind(),
            std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
        ) {
            return BridgeError::transport_timeout("reading daemon wire frame timed out");
        }

        BridgeError::transport_unavailable(format!("reading daemon wire frame failed: {error}"))
    })?;
    if bytes == 0 {
        return Err(BridgeError::transport_unavailable(
            "daemon closed the bridge socket unexpectedly",
        ));
    }
    serde_json::from_str(buffer.trim_end()).map_err(|error| {
        BridgeError::transport_unavailable(format!("parsing daemon wire frame failed: {error}"))
    })
}

fn to_value<T: Serialize>(params: &T) -> Result<Value, BridgeError> {
    serde_json::to_value(params)
        .map_err(|error| BridgeError::invalid_params(format!("serializing params failed: {error}")))
}

fn next_request_id() -> String {
    format!("bridge-{}", NEXT_REQUEST_ID.fetch_add(1, Ordering::Relaxed))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;
    use std::os::unix::net::UnixListener;
    use std::thread;

    type TestResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

    fn sample_cron_status_payload() -> Value {
        json!({
            "definitions": [{
                "definitionKey": "fts::procurement-ingestion/fts",
                "sourceId": "fts",
                "skillId": "procurement-ingestion/fts",
                "skillVersion": "1.0.0",
                "cronExpression": "15 4 * * *",
                "enabled": true,
                "lastRunAt": "2026-03-30T04:15:00Z",
                "nextRunAt": "2026-03-31T04:15:00Z"
            }],
            "schedulerRunning": true,
            "lastTickAt": "2026-03-30T04:15:10Z",
            "nextRunAt": "2026-03-31T04:15:00Z",
            "workerLanes": [{
                "lane": "pty_browser",
                "running": true,
                "claimableKinds": ["sync_source", "source_health", "auth_refresh"]
            }, {
                "lane": "coverage_deterministic",
                "running": true,
                "claimableKinds": [
                    "commercial_sweep",
                    "validation_cross_model_audit",
                    "validation_calibration_readiness_sweep"
                ]
            }],
            "awaitingAccess": {
                "activeCount": 2,
                "expiredCount": 1
            }
        })
    }

    fn io_error(message: impl Into<String>) -> io::Error {
        io::Error::other(message.into())
    }

    #[test]
    fn desktop_status_001_bridge_get_cron_status_is_read_only_and_typed() -> TestResult {
        let temp = tempfile::tempdir()?;
        let socket_path = temp.path().join("daemon.sock");
        let listener = UnixListener::bind(socket_path.as_path())?;
        fs::write(
            temp.path().join(DESCRIPTOR_FILENAME),
            serde_json::to_vec(&json!({
                "socketPath": socket_path.display().to_string()
            }))?,
        )?;

        let server = thread::spawn(move || -> TestResult {
            let (mut stream, _) = listener.accept()?;
            let mut reader = BufReader::new(stream.try_clone()?);

            let mut hello = String::new();
            reader.read_line(&mut hello)?;
            let hello_frame: Value = serde_json::from_str(hello.trim_end())?;
            if hello_frame.get("messageType").and_then(Value::as_str) != Some("hello") {
                return Err(io_error("daemon bridge did not send hello first").into());
            }

            writeln!(
                stream,
                "{}",
                json!({
                    "messageType": "welcome",
                    "selectedProtocolVersion": DEFAULT_PROTOCOL_VERSION,
                    "daemonVersion": "test",
                    "instanceId": "daemon-test",
                    "schemaEpoch": DEFAULT_SCHEMA_EPOCH,
                    "commitSeq": 7,
                    "healthState": { "status": "ready" },
                    "mutationsAllowed": true,
                    "activeSessions": { "count": 0, "ids": [], "states": [] },
                    "reconnectOutcome": "freshConnection",
                    "streamResumePermitted": true
                })
            )?;

            let mut command = String::new();
            reader.read_line(&mut command)?;
            let command_frame: Value = serde_json::from_str(command.trim_end())?;
            if command_frame.get("messageType").and_then(Value::as_str) != Some("command") {
                return Err(io_error("daemon bridge did not send command frame").into());
            }
            if command_frame.get("method").and_then(Value::as_str) != Some("get_cron_status") {
                return Err(io_error("daemon bridge called the wrong cron-status method").into());
            }
            if command_frame.get("idempotencyKey") != Some(&Value::Null) {
                return Err(
                    io_error("cron-status command must not carry an idempotency key").into(),
                );
            }
            if command_frame.get("params") != Some(&json!({})) {
                return Err(io_error("cron-status command params must be empty").into());
            }

            let request_id = command_frame
                .get("requestId")
                .and_then(Value::as_str)
                .ok_or_else(|| io_error("cron-status command request id missing"))?;
            writeln!(
                stream,
                "{}",
                json!({
                    "messageType": "commandResponse",
                    "requestId": request_id,
                    "payload": sample_cron_status_payload()
                })
            )?;
            Ok(())
        });

        let bridge = DaemonBridge::new(temp.path());
        let mut connection = bridge.connect()?;
        let snapshot = connection.get_cron_status()?;

        assert_eq!(snapshot.definitions.len(), 1);
        assert_eq!(
            snapshot
                .definitions
                .first()
                .ok_or_else(|| io_error("cron definition missing"))?
                .definition_key,
            "fts::procurement-ingestion/fts"
        );
        assert!(snapshot.scheduler_running);
        assert_eq!(
            snapshot.last_tick_at.as_deref(),
            Some("2026-03-30T04:15:10Z")
        );
        assert_eq!(
            snapshot.next_run_at.as_deref(),
            Some("2026-03-31T04:15:00Z")
        );
        assert_eq!(snapshot.worker_lanes.len(), 2);
        assert_eq!(snapshot.awaiting_access.active_count, 2);
        assert_eq!(snapshot.awaiting_access.expired_count, 1);

        server
            .join()
            .map_err(|_| io_error("fake daemon server panicked"))??;
        Ok(())
    }
}
