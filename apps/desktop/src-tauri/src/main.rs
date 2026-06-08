#![forbid(unsafe_code)]

#[allow(dead_code)]
mod daemon_bridge;
#[allow(dead_code)]
mod query_service;

use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
#[cfg(unix)]
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::Duration;

use self::daemon_bridge::{
    BridgeError as DaemonBridgeError,
    CaptureClarificationParams as BridgeCaptureClarificationParams,
    CaptureDecisionParams as BridgeCaptureDecisionParams, DaemonBridge,
    DecisionDisposition as BridgeDecisionDisposition, MutationAck as BridgeMutationAck,
    NativeSubscriptionEvent, RerunFromErrorParams as BridgeRerunFromErrorParams,
    ResizeSessionParams as BridgeResizeSessionParams,
    ResolveErrorParams as BridgeResolveErrorParams, RestartDaemonParams,
    RetryJobParams as BridgeRetryJobParams, SendSessionInputParams as BridgeSendSessionInputParams,
    StopSessionParams as BridgeStopSessionParams,
    TriggerEnrichmentParams as BridgeTriggerEnrichmentParams,
    TriggerIngestionParams as BridgeTriggerIngestionParams,
    WithdrawClarificationParams as BridgeWithdrawClarificationParams,
    WithdrawDecisionParams as BridgeWithdrawDecisionParams,
};
use anyhow::{Context, Result, anyhow};
use chrono::Utc;
use rusqlite::Connection;
#[cfg(test)]
use rusqlite::{OptionalExtension, Transaction, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tauri::{Emitter, Manager};

const DESCRIPTOR_FILENAME: &str = "daemon.info.json";
const SOCKET_FILENAME: &str = "daemon.sock";
const STATE_DIRNAME: &str = "state";
const RUNTIME_DIRNAME: &str = "runtime";
const ARTIFACT_DIRNAME: &str = "artifacts";
const STATE_DB_FILENAME: &str = "state.db";
const EXPECTED_PROTOCOL_VERSION: u32 = 1;
const EXPECTED_SCHEMA_EPOCH: i64 = 11;
const DEFAULT_DAEMON_PROGRAM: &str = "daemon";
const DESKTOP_BASE_DIR_ENV: &str = "PROCINTEL_DESKTOP_BASE_DIR";
const DESKTOP_STATE_ROOT_ENV: &str = "PROCINTEL_DESKTOP_STATE_ROOT";
const DESKTOP_RUNTIME_DIR_ENV: &str = "PROCINTEL_DESKTOP_RUNTIME_DIR";
const DESKTOP_ARTIFACT_ROOT_ENV: &str = "PROCINTEL_DESKTOP_ARTIFACT_ROOT";
const HANDSHAKE_RETRY_BACKOFF_MS: [u64; 6] = [150, 300, 600, 1_000, 1_500, 2_000];
const NATIVE_SUBSCRIPTION_POLL_MS: u64 = 250;

static LAUNCH_GUARD: OnceLock<Mutex<bool>> = OnceLock::new();
#[cfg(test)]
type TestDaemonLaunchHook = Arc<dyn Fn(&Path, &Path, &Path) -> Result<()> + Send + Sync>;
#[cfg(test)]
static TEST_DAEMON_LAUNCH_HOOK: OnceLock<Mutex<Option<TestDaemonLaunchHook>>> = OnceLock::new();
#[cfg(test)]
type TestPipelineRerunHook = Arc<dyn Fn(&str) -> Result<()> + Send + Sync>;
#[cfg(test)]
static TEST_PIPELINE_RERUN_HOOK: OnceLock<Mutex<Option<TestPipelineRerunHook>>> = OnceLock::new();
#[cfg(test)]
type TestDaemonStopHook = Arc<dyn Fn(u32) -> Result<()> + Send + Sync>;
#[cfg(test)]
static TEST_DAEMON_STOP_HOOK: OnceLock<Mutex<Option<TestDaemonStopHook>>> = OnceLock::new();
#[cfg(test)]
type TestProcessRunningHook = Arc<dyn Fn(u32) -> Result<bool> + Send + Sync>;
#[cfg(test)]
static TEST_PROCESS_RUNNING_HOOK: OnceLock<Mutex<Option<TestProcessRunningHook>>> = OnceLock::new();

const PIPELINE_RERUN_INGEST_STAGES: [&str; 3] = ["landed-cost", "net-gate", "manifest"];

pub fn configure_builder<R: tauri::Runtime>(builder: tauri::Builder<R>) -> tauri::Builder<R> {
    builder
        .manage(BootstrapStore::default())
        .manage(NativeSubscriptionStore::default())
        .invoke_handler(tauri::generate_handler![
            get_bootstrap_snapshot,
            refresh_bootstrap_snapshot,
            list_processes,
            get_process_detail,
            list_manifests,
            get_decision_history,
            get_active_overrides,
            list_sessions,
            get_session_history,
            list_errors,
            get_daemon_info,
            trigger_ingestion,
            trigger_enrichment,
            restart_daemon,
            retry_job,
            stop_session,
            send_session_input,
            resize_session,
            subscribe_topic,
            unsubscribe_topic,
            quit_ui,
            stop_background_daemon_and_quit,
            open_logs,
            open_artifacts,
            capture_decision,
            capture_clarification,
            withdraw_clarification,
            withdraw_decision,
            trigger_pipeline_rerun,
            resolve_error,
            rerun_from_error
        ])
        .setup(|app| {
            let snapshot = bootstrap_application(app.handle(), None);
            app.state::<BootstrapStore>().replace(snapshot)?;
            Ok(())
        })
}

fn main() -> Result<()> {
    configure_builder(tauri::Builder::default())
        .run(tauri::generate_context!())
        .context("running desktop shell")?;

    Ok(())
}

#[derive(Default)]
struct BootstrapStore(Mutex<Option<BootstrapSnapshot>>);

impl BootstrapStore {
    fn replace(&self, snapshot: BootstrapSnapshot) -> Result<()> {
        let mut guard = self
            .0
            .lock()
            .map_err(|_| anyhow!("bootstrap store lock poisoned"))?;
        *guard = Some(snapshot);
        drop(guard);
        Ok(())
    }

    #[allow(dead_code)]
    fn snapshot(&self) -> Result<Option<BootstrapSnapshot>> {
        self.0
            .lock()
            .map(|guard| guard.clone())
            .map_err(|_| anyhow!("bootstrap store lock poisoned"))
    }

    fn last_seen_instance_id(&self) -> Result<Option<String>> {
        self.snapshot().map(|snapshot| {
            snapshot.and_then(|snapshot| {
                if snapshot.control_plane.daemon_instance_id.is_empty() {
                    None
                } else {
                    Some(snapshot.control_plane.daemon_instance_id)
                }
            })
        })
    }
}

#[derive(Default)]
struct NativeSubscriptionStore(Mutex<HashMap<String, NativeSubscriptionHandle>>);

struct NativeSubscriptionHandle {
    stop: Arc<AtomicBool>,
}

impl NativeSubscriptionStore {
    fn replace(
        &self,
        topic: String,
        handle: NativeSubscriptionHandle,
    ) -> Result<Option<NativeSubscriptionHandle>> {
        let mut guard = self
            .0
            .lock()
            .map_err(|_| anyhow!("native subscription store lock poisoned"))?;
        Ok(guard.insert(topic, handle))
    }

    fn remove(&self, topic: &str) -> Result<Option<NativeSubscriptionHandle>> {
        let mut guard = self
            .0
            .lock()
            .map_err(|_| anyhow!("native subscription store lock poisoned"))?;
        Ok(guard.remove(topic))
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct TopicSubscriptionParams {
    topic: String,
    from_seq: Option<u64>,
}

#[cfg(test)]
#[derive(Clone, Debug)]
pub struct TestRuntimeHarness {
    base_dir: PathBuf,
    exit_codes: Arc<Mutex<Vec<i32>>>,
    stopped_pids: Arc<Mutex<Vec<u32>>>,
}

#[cfg(test)]
#[allow(clippy::must_use_candidate, clippy::missing_errors_doc)]
impl TestRuntimeHarness {
    pub fn new(base_dir: PathBuf) -> Self {
        Self {
            base_dir,
            exit_codes: Arc::new(Mutex::new(Vec::new())),
            stopped_pids: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }

    pub fn state_root(&self) -> PathBuf {
        self.base_dir.join(STATE_DIRNAME)
    }

    pub fn runtime_dir(&self) -> PathBuf {
        self.base_dir.join(RUNTIME_DIRNAME)
    }

    pub fn artifact_root(&self) -> PathBuf {
        self.base_dir.join(ARTIFACT_DIRNAME)
    }

    pub fn descriptor_path(&self) -> PathBuf {
        self.runtime_dir().join(DESCRIPTOR_FILENAME)
    }

    pub fn socket_path(&self) -> PathBuf {
        self.runtime_dir().join(SOCKET_FILENAME)
    }

    pub fn exit_codes(&self) -> Result<Vec<i32>> {
        self.exit_codes
            .lock()
            .map(|codes| codes.clone())
            .map_err(|_| anyhow!("test runtime exit-code lock poisoned"))
    }

    pub fn stopped_pids(&self) -> Result<Vec<u32>> {
        self.stopped_pids
            .lock()
            .map(|pids| pids.clone())
            .map_err(|_| anyhow!("test runtime stop-pid lock poisoned"))
    }

    fn record_exit(&self, code: i32) -> Result<()> {
        self.exit_codes
            .lock()
            .map_err(|_| anyhow!("test runtime exit-code lock poisoned"))?
            .push(code);
        Ok(())
    }

    fn record_stopped_pid(&self, pid: u32) -> Result<()> {
        self.stopped_pids
            .lock()
            .map_err(|_| anyhow!("test runtime stop-pid lock poisoned"))?
            .push(pid);
        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::missing_errors_doc)]
pub fn reset_test_runtime_state() -> Result<()> {
    if let Some(guard) = LAUNCH_GUARD.get() {
        *guard
            .lock()
            .map_err(|_| anyhow!("desktop daemon launch guard is poisoned"))? = false;
    }

    let hook = TEST_DAEMON_LAUNCH_HOOK.get_or_init(|| Mutex::new(None));
    *hook
        .lock()
        .map_err(|_| anyhow!("test daemon launch hook lock poisoned"))? = None;

    let rerun_hook = TEST_PIPELINE_RERUN_HOOK.get_or_init(|| Mutex::new(None));
    *rerun_hook
        .lock()
        .map_err(|_| anyhow!("test pipeline rerun hook lock poisoned"))? = None;

    let stop_hook = TEST_DAEMON_STOP_HOOK.get_or_init(|| Mutex::new(None));
    *stop_hook
        .lock()
        .map_err(|_| anyhow!("test daemon stop hook lock poisoned"))? = None;

    let running_hook = TEST_PROCESS_RUNNING_HOOK.get_or_init(|| Mutex::new(None));
    *running_hook
        .lock()
        .map_err(|_| anyhow!("test process-running hook lock poisoned"))? = None;

    Ok(())
}

#[cfg(test)]
#[allow(clippy::missing_errors_doc)]
pub fn set_test_daemon_launch_hook<F>(hook: F) -> Result<()>
where
    F: Fn(&Path, &Path, &Path) -> Result<()> + Send + Sync + 'static,
{
    let slot = TEST_DAEMON_LAUNCH_HOOK.get_or_init(|| Mutex::new(None));
    *slot
        .lock()
        .map_err(|_| anyhow!("test daemon launch hook lock poisoned"))? = Some(Arc::new(hook));
    Ok(())
}

#[cfg(test)]
fn set_test_pipeline_rerun_hook<F>(hook: F) -> Result<()>
where
    F: Fn(&str) -> Result<()> + Send + Sync + 'static,
{
    let slot = TEST_PIPELINE_RERUN_HOOK.get_or_init(|| Mutex::new(None));
    *slot
        .lock()
        .map_err(|_| anyhow!("test pipeline rerun hook lock poisoned"))? = Some(Arc::new(hook));
    Ok(())
}

#[cfg(test)]
fn set_test_daemon_stop_hook<F>(hook: F) -> Result<()>
where
    F: Fn(u32) -> Result<()> + Send + Sync + 'static,
{
    let slot = TEST_DAEMON_STOP_HOOK.get_or_init(|| Mutex::new(None));
    *slot
        .lock()
        .map_err(|_| anyhow!("test daemon stop hook lock poisoned"))? = Some(Arc::new(hook));
    Ok(())
}

#[cfg(test)]
fn set_test_process_running_hook<F>(hook: F) -> Result<()>
where
    F: Fn(u32) -> Result<bool> + Send + Sync + 'static,
{
    let slot = TEST_PROCESS_RUNNING_HOOK.get_or_init(|| Mutex::new(None));
    *slot
        .lock()
        .map_err(|_| anyhow!("test process-running hook lock poisoned"))? = Some(Arc::new(hook));
    Ok(())
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct BootstrapSnapshot {
    mode: BootstrapMode,
    control_plane: ControlPlaneState,
    descriptor: Option<DaemonDescriptorSnapshot>,
    launched_daemon: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CaptureDecisionParams {
    process_id: String,
    disposition: DecisionDisposition,
    reviewed_basis_hash: Option<String>,
    reviewed_manifest_hash: Option<String>,
    reviewed_at_stage_run_id: Option<String>,
    decision_note: Option<String>,
    idempotency_key: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CaptureClarificationFrontendParams {
    process_id: String,
    candidate_id: String,
    reason_code: String,
    payload: Value,
    note: Option<String>,
    idempotency_key: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WithdrawClarificationFrontendParams {
    process_id: String,
    candidate_id: String,
    reason_code: String,
    idempotency_key: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WithdrawDecisionFrontendParams {
    idempotency_key: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TriggerPipelineRerunParams {
    process_id: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GetDecisionHistoryParams {
    process_id: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GetActiveOverridesParams {
    process_id: String,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum DecisionDisposition {
    Approve,
    Reject,
    Watch,
}

#[cfg(test)]
impl DecisionDisposition {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Approve => "approve",
            Self::Reject => "reject",
            Self::Watch => "watch",
        }
    }
}

#[cfg(test)]
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct DecisionMutationAck {
    accepted: bool,
    visible_after_seq: i64,
    idempotency_key: String,
}

#[cfg(test)]
#[derive(Clone, Debug, PartialEq, Eq)]
struct DecisionCaptureContext {
    process_id: String,
    reviewed_basis_hash: String,
    reviewed_manifest_hash: String,
    reviewed_at_stage_run_id: String,
}

#[cfg(test)]
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
struct OperatorMutationAck {
    accepted: bool,
    visible_after_seq: i64,
    job_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct RestartMutationAck {
    accepted: bool,
    visible_after_seq: i64,
    idempotency_key: String,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct PipelineRerunAck {
    accepted: bool,
    visible_after_seq: i64,
    idempotency_key: String,
    stages: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct GetProcessDetailParams {
    process_id: String,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
enum BootstrapMode {
    FullControl,
    ControlLimited,
    BrowseOnly,
    Incompatible,
    RecoveryRequired,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ControlPlaneState {
    daemon_instance_id: String,
    daemon_version: String,
    protocol_version: u32,
    schema_epoch: i64,
    storage: StorageState,
    daemon: DaemonState,
    control: ControlState,
    load: LoadState,
    reasons: Vec<String>,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum StorageState {
    Ready,
    Repairing,
    Unreadable,
    SchemaMismatch,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum DaemonState {
    Starting,
    Ready,
    Degraded,
    Unavailable,
    Incompatible,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum ControlState {
    Full,
    Limited,
    ReadOnly,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum LoadState {
    Normal,
    Busy,
    Saturated,
}

#[derive(Clone, Debug)]
struct StorageInspection {
    state: StorageState,
    reasons: Vec<String>,
}

#[derive(Clone, Debug)]
struct DesktopPaths {
    state_root: PathBuf,
    runtime_dir: PathBuf,
    artifact_root: PathBuf,
}

impl DesktopPaths {
    fn from_handle<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> Result<Self> {
        #[cfg(test)]
        if let Some(harness) = app.try_state::<TestRuntimeHarness>() {
            return Ok(Self::from_base_dir(harness.base_dir()));
        }

        let base_dir = std::env::var_os(DESKTOP_BASE_DIR_ENV)
            .map(PathBuf::from)
            .map_or_else(
                || {
                    app.path()
                        .app_local_data_dir()
                        .context("resolving app-local data directory")
                },
                Ok,
            )?;
        Ok(Self::from_base_dir_with_overrides(
            base_dir,
            env_path(DESKTOP_STATE_ROOT_ENV),
            env_path(DESKTOP_RUNTIME_DIR_ENV),
            env_path(DESKTOP_ARTIFACT_ROOT_ENV),
        ))
    }

    #[cfg(test)]
    fn from_base_dir(base_dir: impl Into<PathBuf>) -> Self {
        Self::from_base_dir_with_overrides(base_dir, None, None, None)
    }

    fn from_base_dir_with_overrides(
        base_dir: impl Into<PathBuf>,
        state_root: Option<PathBuf>,
        runtime_dir: Option<PathBuf>,
        artifact_root: Option<PathBuf>,
    ) -> Self {
        let base_dir = base_dir.into();
        Self {
            state_root: state_root.unwrap_or_else(|| base_dir.join(STATE_DIRNAME)),
            runtime_dir: runtime_dir.unwrap_or_else(|| base_dir.join(RUNTIME_DIRNAME)),
            artifact_root: artifact_root.unwrap_or_else(|| base_dir.join(ARTIFACT_DIRNAME)),
        }
    }

    fn state_db_path(&self) -> PathBuf {
        self.state_root.join(STATE_DB_FILENAME)
    }

    fn descriptor_path(&self) -> PathBuf {
        self.runtime_dir.join(DESCRIPTOR_FILENAME)
    }

    fn socket_path(&self) -> PathBuf {
        self.runtime_dir.join(SOCKET_FILENAME)
    }
}

fn env_path(name: &str) -> Option<PathBuf> {
    std::env::var_os(name)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct DaemonDescriptorSnapshot {
    instance_id: String,
    pid: u32,
    socket_path: String,
    protocol_version: u32,
    schema_epoch: i64,
    state_root: String,
    started_at: String,
    ready: bool,
}

#[derive(Debug, Serialize)]
#[serde(
    tag = "messageType",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
enum ClientMessage {
    Hello {
        desktop_version: String,
        supported_protocol_range: ProtocolRange,
        expected_schema_epoch: i64,
        last_seen_instance_id: Option<String>,
    },
}

#[derive(Debug, Deserialize)]
#[serde(
    tag = "messageType",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
enum ServerMessage {
    Welcome {
        selected_protocol_version: u32,
        daemon_version: String,
        instance_id: String,
        schema_epoch: i64,
        commit_seq: i64,
        health_state: HealthState,
        mutations_allowed: bool,
        active_sessions: ActiveSessions,
        reconnect_outcome: ReconnectOutcome,
        stream_resume_permitted: bool,
    },
    Rejection {
        code: String,
        message: String,
        supported_protocol_range: Option<ProtocolRange>,
        expected_schema_epoch: Option<i64>,
        actual_schema_epoch: Option<i64>,
    },
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProtocolRange {
    min: u32,
    max: u32,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HealthState {
    status: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ActiveSessions {
    count: usize,
    ids: Vec<String>,
    states: Vec<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
enum ReconnectOutcome {
    FreshConnection,
    SameDaemonReconnect,
    DaemonRestartInvalidated,
}

enum HandshakeFailure {
    Unavailable(String),
    Incompatible(String),
}

fn bootstrap_application<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    last_seen_instance_id: Option<&str>,
) -> BootstrapSnapshot {
    match DesktopPaths::from_handle(app) {
        Ok(paths) => run_startup_sequence(&paths, last_seen_instance_id),
        Err(error) => BootstrapSnapshot {
            mode: BootstrapMode::RecoveryRequired,
            control_plane: ControlPlaneState {
                daemon_instance_id: String::new(),
                daemon_version: String::new(),
                protocol_version: EXPECTED_PROTOCOL_VERSION,
                schema_epoch: EXPECTED_SCHEMA_EPOCH,
                storage: StorageState::Unreadable,
                daemon: DaemonState::Unavailable,
                control: ControlState::ReadOnly,
                load: LoadState::Normal,
                reasons: vec![format!(
                    "Unable to resolve desktop runtime paths: {error:#}"
                )],
            },
            descriptor: None,
            launched_daemon: false,
        },
    }
}

fn run_startup_sequence(
    paths: &DesktopPaths,
    last_seen_instance_id: Option<&str>,
) -> BootstrapSnapshot {
    let storage = inspect_storage(paths);
    let (descriptor, descriptor_reasons) = read_descriptor(paths.descriptor_path().as_path());
    let mut reasons = storage.reasons.clone();
    reasons.extend(descriptor_reasons);

    if matches!(
        storage.state,
        StorageState::Unreadable | StorageState::Repairing | StorageState::SchemaMismatch
    ) {
        return snapshot_without_handshake(
            storage.state,
            descriptor,
            if storage.state == StorageState::SchemaMismatch {
                DaemonState::Incompatible
            } else {
                DaemonState::Unavailable
            },
            reasons,
            false,
        );
    }

    match try_handshake(paths, descriptor.as_ref(), last_seen_instance_id) {
        Ok(welcome) => {
            return snapshot_from_welcome(storage.state, descriptor, welcome, reasons, false);
        }
        Err(HandshakeFailure::Incompatible(reason)) => {
            reasons.push(reason);
            return snapshot_without_handshake(
                storage.state,
                descriptor,
                DaemonState::Incompatible,
                reasons,
                false,
            );
        }
        Err(HandshakeFailure::Unavailable(reason)) => reasons.push(reason),
    }

    let launched_daemon = match launch_detached_daemon(paths) {
        Ok(was_launched) => {
            reasons.push(if was_launched {
                "Daemon bootstrap launched a detached background daemon.".to_owned()
            } else {
                "Skipped daemon launch because this startup sequence already attempted it."
                    .to_owned()
            });
            was_launched
        }
        Err(error) => {
            reasons.push(format!("Daemon auto-launch failed: {error:#}"));
            return snapshot_without_handshake(
                storage.state,
                descriptor,
                DaemonState::Unavailable,
                reasons,
                false,
            );
        }
    };

    finish_handshake_retries(
        paths,
        storage.state,
        descriptor,
        reasons,
        launched_daemon,
        last_seen_instance_id,
    )
}

fn inspect_storage(paths: &DesktopPaths) -> StorageInspection {
    if let Err(error) = fs::create_dir_all(&paths.state_root) {
        return StorageInspection {
            state: StorageState::Unreadable,
            reasons: vec![format!(
                "State directory {} is not writable: {error}",
                paths.state_root.display()
            )],
        };
    }

    let db_path = paths.state_db_path();
    let db_preexisted = db_path.exists();
    let connection = match Connection::open(&db_path) {
        Ok(connection) => connection,
        Err(error) => {
            return StorageInspection {
                state: StorageState::Unreadable,
                reasons: vec![format!(
                    "State database {} could not be opened: {error}",
                    db_path.display()
                )],
            };
        }
    };

    let quick_check: String = match connection.query_row("PRAGMA quick_check", [], |row| row.get(0))
    {
        Ok(value) => value,
        Err(error) => {
            return StorageInspection {
                state: StorageState::Unreadable,
                reasons: vec![format!(
                    "PRAGMA quick_check failed for {}: {error}",
                    db_path.display()
                )],
            };
        }
    };

    if quick_check != "ok" {
        return StorageInspection {
            state: StorageState::Repairing,
            reasons: vec![format!(
                "SQLite quick_check reported {quick_check} for {}.",
                db_path.display()
            )],
        };
    }

    let schema_epoch: i64 =
        match connection.pragma_query_value(None, "user_version", |row| row.get(0)) {
            Ok(value) => value,
            Err(error) => {
                return StorageInspection {
                    state: StorageState::Unreadable,
                    reasons: vec![format!(
                        "PRAGMA user_version failed for {}: {error}",
                        db_path.display()
                    )],
                };
            }
        };

    if schema_epoch != 0 && schema_epoch != EXPECTED_SCHEMA_EPOCH {
        return StorageInspection {
            state: StorageState::SchemaMismatch,
            reasons: vec![format!(
                "Desktop expects schema epoch {EXPECTED_SCHEMA_EPOCH}, but {} is at {schema_epoch}.",
                db_path.display()
            )],
        };
    }

    let mut reasons = Vec::new();
    if db_preexisted {
        reasons.push(format!(
            "SQLite quick_check passed for {} with schema epoch {}.",
            db_path.display(),
            if schema_epoch == 0 {
                EXPECTED_SCHEMA_EPOCH
            } else {
                schema_epoch
            }
        ));
    } else {
        reasons.push(format!(
            "State database {} did not exist yet; the daemon will initialize it on first serve.",
            db_path.display()
        ));
    }

    StorageInspection {
        state: StorageState::Ready,
        reasons,
    }
}

fn read_descriptor(path: &Path) -> (Option<DaemonDescriptorSnapshot>, Vec<String>) {
    match fs::read_to_string(path) {
        Ok(contents) => match serde_json::from_str::<DaemonDescriptorSnapshot>(&contents) {
            Ok(descriptor) => {
                let reason = format!(
                    "Read daemon descriptor {} for instance {}.",
                    path.display(),
                    descriptor.instance_id
                );
                (Some(descriptor), vec![reason])
            }
            Err(error) => (
                None,
                vec![format!(
                    "Ignoring unreadable daemon descriptor {}: {error}",
                    path.display()
                )],
            ),
        },
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => (None, Vec::new()),
        Err(error) => (
            None,
            vec![format!(
                "Descriptor read failed for {}: {error}",
                path.display()
            )],
        ),
    }
}

fn try_handshake(
    paths: &DesktopPaths,
    descriptor: Option<&DaemonDescriptorSnapshot>,
    last_seen_instance_id: Option<&str>,
) -> std::result::Result<ServerWelcome, HandshakeFailure> {
    let socket_path = descriptor
        .map(|value| PathBuf::from(&value.socket_path))
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| paths.socket_path());

    if let Some(descriptor) = descriptor
        && !descriptor.ready
    {
        return Err(HandshakeFailure::Unavailable(format!(
            "Descriptor for instance {} is not marked ready yet.",
            descriptor.instance_id
        )));
    }

    #[cfg(unix)]
    {
        let frame = read_handshake_frame(socket_path.as_path(), last_seen_instance_id)?;
        parse_handshake_frame(frame)
    }

    #[cfg(not(unix))]
    {
        let _ = paths;
        let _ = descriptor;
        let _ = last_seen_instance_id;
        Err(HandshakeFailure::Unavailable(
            "Layer 2 desktop bootstrap currently expects Unix-domain daemon transport; named-pipe support is not implemented yet."
                .to_owned(),
        ))
    }
}

fn launch_detached_daemon(paths: &DesktopPaths) -> Result<bool> {
    let guard = LAUNCH_GUARD.get_or_init(|| Mutex::new(false));
    let mut launched = guard
        .lock()
        .map_err(|_| anyhow!("desktop daemon launch guard is poisoned"))?;
    if *launched {
        return Ok(false);
    }

    fs::create_dir_all(&paths.runtime_dir)
        .with_context(|| format!("creating {}", paths.runtime_dir.display()))?;
    fs::create_dir_all(&paths.artifact_root)
        .with_context(|| format!("creating {}", paths.artifact_root.display()))?;

    #[cfg(test)]
    {
        let hook = TEST_DAEMON_LAUNCH_HOOK.get_or_init(|| Mutex::new(None));
        let hook = hook
            .lock()
            .map_err(|_| anyhow!("test daemon launch hook lock poisoned"))?
            .clone();
        if let Some(hook) = hook {
            hook(
                paths.state_root.as_path(),
                paths.runtime_dir.as_path(),
                paths.artifact_root.as_path(),
            )?;
            *launched = true;
            drop(launched);
            return Ok(true);
        }
    }

    spawn_daemon_process(paths).context("spawning detached daemon process")?;
    *launched = true;
    drop(launched);
    Ok(true)
}

fn spawn_daemon_process(paths: &DesktopPaths) -> Result<()> {
    if let Some(explicit_program) = std::env::var_os("PROCINTEL_DAEMON_PROGRAM") {
        return spawn_program(
            Command::new(explicit_program),
            paths,
            "PROCINTEL_DAEMON_PROGRAM override",
        );
    }

    if let Some(command) = sibling_daemon_command()? {
        match spawn_program(command, paths, "daemon binary next to desktop executable") {
            Ok(()) => return Ok(()),
            Err(error) if is_not_found_error(&error) => {}
            Err(error) => return Err(error),
        }
    }

    match spawn_program(
        Command::new(DEFAULT_DAEMON_PROGRAM),
        paths,
        "daemon binary on PATH",
    ) {
        Ok(()) => Ok(()),
        Err(error) if is_not_found_error(&error) => spawn_program(
            cargo_daemon_command()?,
            paths,
            "cargo workspace daemon fallback",
        ),
        Err(error) => Err(error),
    }
}

fn sibling_daemon_command() -> Result<Option<Command>> {
    let current_exe = std::env::current_exe().context("resolving desktop executable path")?;
    let daemon_name = if cfg!(windows) {
        "daemon.exe"
    } else {
        "daemon"
    };
    let daemon_path = current_exe.with_file_name(daemon_name);
    if daemon_path.is_file() {
        Ok(Some(Command::new(daemon_path)))
    } else {
        Ok(None)
    }
}

fn spawn_program(mut command: Command, paths: &DesktopPaths, source: &str) -> Result<()> {
    let launch_log_path = paths.runtime_dir.join("daemon-launch.log");
    let launch_log = OpenOptions::new()
        .create(true)
        .append(true)
        .open(launch_log_path.as_path())
        .with_context(|| format!("opening daemon launch log {}", launch_log_path.display()))?;
    let launch_error_log = launch_log
        .try_clone()
        .with_context(|| format!("cloning daemon launch log {}", launch_log_path.display()))?;

    command
        .arg("--serve")
        .arg("--state-root")
        .arg(&paths.state_root)
        .arg("--runtime-dir")
        .arg(&paths.runtime_dir)
        .arg("--artifact-root")
        .arg(&paths.artifact_root)
        .stdin(Stdio::null())
        .stdout(Stdio::from(launch_log))
        .stderr(Stdio::from(launch_error_log));

    command
        .spawn()
        .with_context(|| format!("launching daemon via {source}"))?;
    Ok(())
}

fn cargo_daemon_command() -> Result<Command> {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let repo_root = manifest_dir
        .ancestors()
        .nth(2)
        .ok_or_else(|| anyhow!("resolving repository root from {}", manifest_dir.display()))?;

    let mut command = Command::new("cargo");
    command
        .current_dir(repo_root)
        .args(["run", "-p", "daemon", "--"]);
    Ok(command)
}

fn run_daemon_ingest_stage(paths: &DesktopPaths, stage: &str) -> Result<()> {
    #[cfg(test)]
    {
        let hook = TEST_PIPELINE_RERUN_HOOK
            .get_or_init(|| Mutex::new(None))
            .lock()
            .map_err(|_| anyhow!("test pipeline rerun hook lock poisoned"))?
            .clone();
        if let Some(hook) = hook {
            return hook(stage);
        }
    }

    let mut command = cargo_daemon_command()?;
    command
        .arg("--state-root")
        .arg(&paths.state_root)
        .arg("--runtime-dir")
        .arg(&paths.runtime_dir)
        .arg("--artifact-root")
        .arg(&paths.artifact_root)
        .arg("--ingest")
        .arg(stage);

    let output = command
        .output()
        .with_context(|| format!("running daemon ingest stage `{stage}`"))?;
    if !output.status.success() {
        let status = output.status;
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(anyhow!(
            "daemon ingest stage `{stage}` failed with status {status}; stderr: {stderr}; stdout: {stdout}"
        ));
    }

    Ok(())
}

fn reset_daemon_launch_guard() -> Result<()> {
    if let Some(guard) = LAUNCH_GUARD.get() {
        *guard
            .lock()
            .map_err(|_| anyhow!("desktop daemon launch guard is poisoned"))? = false;
    }
    Ok(())
}

fn process_is_running(pid: u32) -> Result<bool> {
    #[cfg(test)]
    {
        let hook = TEST_PROCESS_RUNNING_HOOK
            .get_or_init(|| Mutex::new(None))
            .lock()
            .map_err(|_| anyhow!("test process-running hook lock poisoned"))?
            .clone();
        if let Some(hook) = hook {
            return hook(pid);
        }
    }

    let status = Command::new("kill")
        .arg("-0")
        .arg(pid.to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .with_context(|| format!("checking daemon pid {pid}"))?;
    if !status.success() {
        return Ok(false);
    }
    Ok(!process_is_zombie(pid)?)
}

fn process_is_zombie(pid: u32) -> Result<bool> {
    let output = Command::new("ps")
        .arg("-p")
        .arg(pid.to_string())
        .arg("-o")
        .arg("stat=")
        .stdin(Stdio::null())
        .output()
        .with_context(|| format!("checking daemon pid {pid} state"))?;
    if !output.status.success() {
        return Ok(false);
    }
    let state = String::from_utf8_lossy(&output.stdout);
    Ok(state.trim().starts_with('Z'))
}

fn wait_for_daemon_stop(pid: u32) -> Result<()> {
    for _ in 0..60 {
        if !process_is_running(pid)? {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(100));
    }
    Err(anyhow!("daemon pid {pid} did not stop within 6 seconds"))
}

fn stop_daemon_for_exclusive_pipeline_rerun(paths: &DesktopPaths) -> Result<bool> {
    let (descriptor, _) = read_descriptor(paths.descriptor_path().as_path());
    let Some(descriptor) = descriptor else {
        return Ok(false);
    };

    if !process_is_running(descriptor.pid)? {
        reset_daemon_launch_guard()?;
        return Ok(false);
    }

    signal_daemon_stop(descriptor.pid).with_context(|| {
        format!(
            "stopping daemon pid {} before pipeline rerun",
            descriptor.pid
        )
    })?;
    wait_for_daemon_stop(descriptor.pid).with_context(|| {
        format!(
            "waiting for daemon pid {} to release runtime lock",
            descriptor.pid
        )
    })?;
    reset_daemon_launch_guard()?;
    Ok(true)
}

fn wait_for_daemon_ready(paths: &DesktopPaths) -> Result<()> {
    for backoff_ms in HANDSHAKE_RETRY_BACKOFF_MS {
        thread::sleep(Duration::from_millis(backoff_ms));
        let (descriptor, _) = read_descriptor(paths.descriptor_path().as_path());
        if try_handshake(paths, descriptor.as_ref(), None).is_ok() {
            return Ok(());
        }
    }
    Err(anyhow!(
        "daemon did not become ready after pipeline rerun restart"
    ))
}

fn ensure_daemon_ready_after_pipeline_rerun(paths: &DesktopPaths) -> Result<()> {
    let (descriptor, _) = read_descriptor(paths.descriptor_path().as_path());
    if try_handshake(paths, descriptor.as_ref(), None).is_ok() {
        return Ok(());
    }

    reset_daemon_launch_guard()?;
    launch_detached_daemon(paths).context("restarting daemon after pipeline rerun")?;
    wait_for_daemon_ready(paths)
}

fn read_commit_seq(db_path: &Path) -> Result<i64> {
    let conn = Connection::open(db_path)
        .with_context(|| format!("opening state database {}", db_path.display()))?;
    conn.query_row(
        "SELECT commit_seq FROM commit_clock WHERE id = 1",
        [],
        |row| row.get::<_, i64>(0),
    )
    .context("reading commit_clock after pipeline rerun")
}

fn snapshot_from_welcome(
    storage: StorageState,
    descriptor: Option<DaemonDescriptorSnapshot>,
    welcome: ServerWelcome,
    mut reasons: Vec<String>,
    launched_daemon: bool,
) -> BootstrapSnapshot {
    reasons.push(format!(
        "Daemon handshake established with instance {} at commit sequence {}.",
        welcome.instance_id, welcome.commit_seq
    ));
    if welcome.reconnect_outcome == ReconnectOutcome::DaemonRestartInvalidated {
        reasons.push(
            "Daemon restart invalidated stream resume; PTY sessions from the previous instance are historical only."
                .to_owned(),
        );
    }
    if !welcome.stream_resume_permitted {
        reasons.push("Stream resume is not currently permitted for this handshake.".to_owned());
    }

    let daemon = match welcome.health_status.as_str() {
        "ready" => DaemonState::Ready,
        "starting" => DaemonState::Starting,
        "incompatible" => DaemonState::Incompatible,
        _ => DaemonState::Degraded,
    };
    let load = match welcome.active_session_count {
        0..=4 => LoadState::Normal,
        5..=12 => LoadState::Busy,
        _ => LoadState::Saturated,
    };
    let control = if !welcome.mutations_allowed {
        ControlState::ReadOnly
    } else if matches!(daemon, DaemonState::Starting | DaemonState::Degraded)
        || matches!(load, LoadState::Busy | LoadState::Saturated)
    {
        ControlState::Limited
    } else {
        ControlState::Full
    };
    let control_plane = ControlPlaneState {
        daemon_instance_id: welcome.instance_id,
        daemon_version: welcome.daemon_version,
        protocol_version: welcome.selected_protocol_version,
        schema_epoch: welcome.schema_epoch,
        storage,
        daemon,
        control,
        load,
        reasons,
    };

    BootstrapSnapshot {
        mode: derive_mode(&control_plane),
        control_plane,
        descriptor,
        launched_daemon,
    }
}

fn snapshot_without_handshake(
    storage: StorageState,
    descriptor: Option<DaemonDescriptorSnapshot>,
    daemon: DaemonState,
    reasons: Vec<String>,
    launched_daemon: bool,
) -> BootstrapSnapshot {
    let control_plane = ControlPlaneState {
        daemon_instance_id: descriptor
            .as_ref()
            .map(|value| value.instance_id.clone())
            .unwrap_or_default(),
        daemon_version: String::new(),
        protocol_version: EXPECTED_PROTOCOL_VERSION,
        schema_epoch: EXPECTED_SCHEMA_EPOCH,
        storage,
        daemon,
        control: ControlState::ReadOnly,
        load: LoadState::Normal,
        reasons,
    };

    BootstrapSnapshot {
        mode: derive_mode(&control_plane),
        control_plane,
        descriptor,
        launched_daemon,
    }
}

fn derive_mode(control_plane: &ControlPlaneState) -> BootstrapMode {
    if matches!(
        control_plane.storage,
        StorageState::Unreadable | StorageState::Repairing
    ) {
        return BootstrapMode::RecoveryRequired;
    }

    if control_plane.storage == StorageState::SchemaMismatch
        || control_plane.daemon == DaemonState::Incompatible
    {
        return BootstrapMode::Incompatible;
    }

    if control_plane.daemon == DaemonState::Unavailable {
        return BootstrapMode::BrowseOnly;
    }

    if matches!(
        control_plane.daemon,
        DaemonState::Starting | DaemonState::Degraded
    ) || matches!(
        control_plane.control,
        ControlState::Limited | ControlState::ReadOnly
    ) || matches!(control_plane.load, LoadState::Busy | LoadState::Saturated)
    {
        return BootstrapMode::ControlLimited;
    }

    BootstrapMode::FullControl
}

struct ServerWelcome {
    selected_protocol_version: u32,
    daemon_version: String,
    instance_id: String,
    schema_epoch: i64,
    commit_seq: i64,
    health_status: String,
    mutations_allowed: bool,
    active_session_count: usize,
    reconnect_outcome: ReconnectOutcome,
    stream_resume_permitted: bool,
}

#[cfg(unix)]
fn read_handshake_frame(
    socket_path: &Path,
    last_seen_instance_id: Option<&str>,
) -> std::result::Result<ServerMessage, HandshakeFailure> {
    let mut stream = UnixStream::connect(socket_path).map_err(|error| {
        HandshakeFailure::Unavailable(format!(
            "Daemon socket {} is unreachable: {error}",
            socket_path.display()
        ))
    })?;
    let payload = serde_json::to_string(&ClientMessage::Hello {
        desktop_version: env!("CARGO_PKG_VERSION").to_owned(),
        supported_protocol_range: ProtocolRange {
            min: EXPECTED_PROTOCOL_VERSION,
            max: EXPECTED_PROTOCOL_VERSION,
        },
        expected_schema_epoch: EXPECTED_SCHEMA_EPOCH,
        last_seen_instance_id: last_seen_instance_id.map(str::to_owned),
    })
    .map_err(|error| {
        HandshakeFailure::Unavailable(format!("Serializing desktop hello failed: {error}"))
    })?;

    stream
        .write_all(payload.as_bytes())
        .and_then(|()| stream.write_all(b"\n"))
        .and_then(|()| stream.flush())
        .map_err(|error| {
            HandshakeFailure::Unavailable(format!(
                "Writing desktop hello to {} failed: {error}",
                socket_path.display()
            ))
        })?;

    let mut response = String::new();
    BufReader::new(stream)
        .read_line(&mut response)
        .map_err(|error| {
            HandshakeFailure::Unavailable(format!(
                "Reading daemon welcome from {} failed: {error}",
                socket_path.display()
            ))
        })?;

    if response.trim().is_empty() {
        return Err(HandshakeFailure::Unavailable(format!(
            "Daemon {} closed the handshake stream without a welcome frame.",
            socket_path.display()
        )));
    }

    serde_json::from_str::<ServerMessage>(response.trim_end()).map_err(|error| {
        HandshakeFailure::Unavailable(format!(
            "Parsing daemon handshake frame from {} failed: {error}",
            socket_path.display()
        ))
    })
}

fn parse_handshake_frame(
    frame: ServerMessage,
) -> std::result::Result<ServerWelcome, HandshakeFailure> {
    match frame {
        ServerMessage::Welcome {
            selected_protocol_version,
            daemon_version,
            instance_id,
            schema_epoch,
            commit_seq,
            health_state,
            mutations_allowed,
            active_sessions,
            reconnect_outcome,
            stream_resume_permitted,
        } => {
            if selected_protocol_version != EXPECTED_PROTOCOL_VERSION {
                return Err(HandshakeFailure::Incompatible(format!(
                    "Daemon selected protocol version {selected_protocol_version}, expected {EXPECTED_PROTOCOL_VERSION}."
                )));
            }
            if schema_epoch != EXPECTED_SCHEMA_EPOCH {
                return Err(HandshakeFailure::Incompatible(format!(
                    "Daemon reported schema epoch {schema_epoch}, expected {EXPECTED_SCHEMA_EPOCH}."
                )));
            }
            if active_sessions.count != active_sessions.ids.len()
                || active_sessions.count != active_sessions.states.len()
            {
                return Err(HandshakeFailure::Unavailable(format!(
                    "Daemon welcome for instance {instance_id} carried inconsistent active session counts."
                )));
            }

            Ok(ServerWelcome {
                selected_protocol_version,
                daemon_version,
                instance_id,
                schema_epoch,
                commit_seq,
                health_status: health_state.status,
                mutations_allowed,
                active_session_count: active_sessions.count,
                reconnect_outcome,
                stream_resume_permitted,
            })
        }
        ServerMessage::Rejection {
            code,
            message,
            supported_protocol_range,
            expected_schema_epoch,
            actual_schema_epoch,
        } => {
            let detail = format!(
                "Daemon rejected the desktop hello with code `{code}`: {message} (supported={supported_protocol_range:?}, expectedSchema={expected_schema_epoch:?}, actualSchema={actual_schema_epoch:?})."
            );
            if matches!(
                code.as_str(),
                "protocol_range_mismatch" | "schema_epoch_mismatch"
            ) {
                Err(HandshakeFailure::Incompatible(detail))
            } else {
                Err(HandshakeFailure::Unavailable(detail))
            }
        }
    }
}

fn is_not_found_error(error: &anyhow::Error) -> bool {
    error
        .chain()
        .filter_map(|cause| cause.downcast_ref::<std::io::Error>())
        .any(|io_error| io_error.kind() == std::io::ErrorKind::NotFound)
}

fn finish_handshake_retries(
    paths: &DesktopPaths,
    storage_state: StorageState,
    mut descriptor: Option<DaemonDescriptorSnapshot>,
    mut reasons: Vec<String>,
    launched_daemon: bool,
    last_seen_instance_id: Option<&str>,
) -> BootstrapSnapshot {
    for backoff_ms in HANDSHAKE_RETRY_BACKOFF_MS {
        thread::park_timeout(Duration::from_millis(backoff_ms));
        let (fresh_descriptor, descriptor_reasons) =
            read_descriptor(paths.descriptor_path().as_path());
        reasons.extend(descriptor_reasons);
        let retry_descriptor = fresh_descriptor.as_ref().or(descriptor.as_ref());
        match try_handshake(paths, retry_descriptor, last_seen_instance_id) {
            Ok(welcome) => {
                reasons.push(format!(
                    "Daemon handshake succeeded after auto-launch retry ({backoff_ms} ms backoff)."
                ));
                return snapshot_from_welcome(
                    storage_state,
                    fresh_descriptor.or(descriptor),
                    welcome,
                    reasons,
                    launched_daemon,
                );
            }
            Err(HandshakeFailure::Incompatible(reason)) => {
                reasons.push(reason);
                return snapshot_without_handshake(
                    storage_state,
                    fresh_descriptor.or(descriptor),
                    DaemonState::Incompatible,
                    reasons,
                    launched_daemon,
                );
            }
            Err(HandshakeFailure::Unavailable(_)) => {
                descriptor = fresh_descriptor.or(descriptor);
            }
        }
    }

    reasons.push(
        "Daemon remained unavailable after the bounded post-launch handshake retries.".to_owned(),
    );
    snapshot_without_handshake(
        storage_state,
        descriptor,
        DaemonState::Unavailable,
        reasons,
        launched_daemon,
    )
}

#[tauri::command]
#[allow(dead_code)]
#[allow(clippy::needless_pass_by_value)]
fn get_bootstrap_snapshot<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    store: tauri::State<'_, BootstrapStore>,
) -> std::result::Result<BootstrapSnapshot, String> {
    if let Some(snapshot) = store
        .inner()
        .snapshot()
        .map_err(|error| error.to_string())?
    {
        return Ok(snapshot);
    }

    let last_seen_instance_id = store
        .inner()
        .last_seen_instance_id()
        .map_err(|error| error.to_string())?;
    let snapshot = bootstrap_application(&app, last_seen_instance_id.as_deref());
    store
        .inner()
        .replace(snapshot.clone())
        .map_err(|error| error.to_string())?;
    Ok(snapshot)
}

#[tauri::command]
#[allow(dead_code)]
#[allow(clippy::needless_pass_by_value)]
fn refresh_bootstrap_snapshot<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    store: tauri::State<'_, BootstrapStore>,
) -> std::result::Result<BootstrapSnapshot, String> {
    let last_seen_instance_id = store
        .inner()
        .last_seen_instance_id()
        .map_err(|error| error.to_string())?;
    let snapshot = bootstrap_application(&app, last_seen_instance_id.as_deref());
    store
        .inner()
        .replace(snapshot.clone())
        .map_err(|error| error.to_string())?;
    Ok(snapshot)
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
fn list_processes<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    cursor: Option<String>,
    limit: Option<usize>,
    filters: Option<query_service::ProcessListFilters>,
) -> std::result::Result<Value, String> {
    let paths = DesktopPaths::from_handle(&app).map_err(|error| error.to_string())?;
    serialize_query_result(
        query_service::QueryService::open(paths.state_db_path(), EXPECTED_SCHEMA_EPOCH).and_then(
            |service| {
                service.list_processes(
                    cursor.as_deref(),
                    limit.unwrap_or(25),
                    &filters.unwrap_or_default(),
                )
            },
        ),
    )
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
fn get_process_detail<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    params: GetProcessDetailParams,
) -> std::result::Result<Value, String> {
    let paths = DesktopPaths::from_handle(&app).map_err(|error| error.to_string())?;
    serialize_query_result(
        query_service::QueryService::open(paths.state_db_path(), EXPECTED_SCHEMA_EPOCH)
            .and_then(|service| service.get_process_detail(params.process_id.as_str())),
    )
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
fn list_sessions<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    cursor: Option<String>,
    limit: Option<usize>,
    filters: Option<query_service::SessionListFilters>,
) -> std::result::Result<Value, String> {
    let paths = DesktopPaths::from_handle(&app).map_err(|error| error.to_string())?;
    serialize_query_result(
        query_service::QueryService::open(paths.state_db_path(), EXPECTED_SCHEMA_EPOCH).and_then(
            |service| {
                service.list_sessions(
                    cursor.as_deref(),
                    limit.unwrap_or(25),
                    &filters.unwrap_or_default(),
                )
            },
        ),
    )
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
fn get_session_history<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    params: query_service::SessionHistoryRequest,
) -> std::result::Result<Value, String> {
    let paths = DesktopPaths::from_handle(&app).map_err(|error| error.to_string())?;
    serialize_query_result(
        query_service::QueryService::open(paths.state_db_path(), EXPECTED_SCHEMA_EPOCH)
            .and_then(|service| service.get_session_history(&params)),
    )
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
fn list_errors<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    cursor: Option<String>,
    limit: Option<usize>,
    filters: Option<query_service::ErrorListFilters>,
) -> std::result::Result<Value, String> {
    let paths = DesktopPaths::from_handle(&app).map_err(|error| error.to_string())?;
    serialize_query_result(
        query_service::QueryService::open(paths.state_db_path(), EXPECTED_SCHEMA_EPOCH).and_then(
            |service| {
                service.list_errors(
                    cursor.as_deref(),
                    limit.unwrap_or(25),
                    &filters.unwrap_or_default(),
                )
            },
        ),
    )
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
fn get_daemon_info<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
) -> std::result::Result<Value, String> {
    let paths = DesktopPaths::from_handle(&app).map_err(|error| error.to_string())?;
    serialize_bridge_result(with_daemon_connection(&paths, |connection| {
        connection.get_daemon_info()
    }))
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
fn trigger_ingestion<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    params: BridgeTriggerIngestionParams,
) -> std::result::Result<Value, String> {
    let paths = DesktopPaths::from_handle(&app).map_err(|error| error.to_string())?;
    let idempotency_key = next_idempotency_key("desktop-ingestion");
    serialize_bridge_result(with_daemon_connection(&paths, |connection| {
        connection.trigger_ingestion(&params, idempotency_key.as_str())
    }))
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
fn trigger_enrichment<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    params: BridgeTriggerEnrichmentParams,
) -> std::result::Result<Value, String> {
    let paths = DesktopPaths::from_handle(&app).map_err(|error| error.to_string())?;
    let idempotency_key = next_idempotency_key("desktop-enrichment");
    serialize_bridge_result(with_daemon_connection(&paths, |connection| {
        connection.trigger_enrichment(&params, idempotency_key.as_str())
    }))
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
fn restart_daemon<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    store: tauri::State<'_, BootstrapStore>,
) -> std::result::Result<RestartMutationAck, String> {
    let paths = DesktopPaths::from_handle(&app).map_err(|error| error.to_string())?;
    let bridge = DaemonBridge::new(&paths.runtime_dir);
    let mut connection = bridge.connect().map_err(|error| error.to_string())?;
    let idempotency_key = format!(
        "desktop-restart-{}",
        Utc::now().timestamp_nanos_opt().unwrap_or_default()
    );
    let ack = connection
        .restart_daemon(&RestartDaemonParams {}, idempotency_key.as_str())
        .map_err(|error| error.to_string())?;
    let last_seen_instance_id = store
        .inner()
        .last_seen_instance_id()
        .map_err(|error| error.to_string())?;
    let refreshed = bootstrap_application(&app, last_seen_instance_id.as_deref());
    store
        .inner()
        .replace(refreshed)
        .map_err(|error| error.to_string())?;
    Ok(RestartMutationAck {
        accepted: ack.accepted,
        visible_after_seq: ack.visible_after_seq,
        idempotency_key: ack.idempotency_key,
    })
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
fn open_logs<R: tauri::Runtime>(app: tauri::AppHandle<R>) -> std::result::Result<String, String> {
    let paths = DesktopPaths::from_handle(&app).map_err(|error| error.to_string())?;
    open_path_in_desktop(paths.runtime_dir.as_path()).map_err(|error| error.to_string())?;
    Ok(paths.runtime_dir.display().to_string())
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
fn open_artifacts<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
) -> std::result::Result<String, String> {
    let paths = DesktopPaths::from_handle(&app).map_err(|error| error.to_string())?;
    open_path_in_desktop(paths.artifact_root.as_path()).map_err(|error| error.to_string())?;
    Ok(paths.artifact_root.display().to_string())
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
fn subscribe_topic<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    params: TopicSubscriptionParams,
    store: tauri::State<'_, NativeSubscriptionStore>,
) -> std::result::Result<(), String> {
    let paths = DesktopPaths::from_handle(&app).map_err(|error| error.to_string())?;
    let topic = params.topic.clone();
    let stop = Arc::new(AtomicBool::new(false));

    if let Some(existing) = store
        .inner()
        .replace(
            topic.clone(),
            NativeSubscriptionHandle {
                stop: Arc::clone(&stop),
            },
        )
        .map_err(|error| error.to_string())?
    {
        existing.stop.store(true, Ordering::Relaxed);
    }

    thread::spawn(move || {
        run_native_subscription_loop(app, paths, topic, params.from_seq, stop);
    });

    Ok(())
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
fn unsubscribe_topic(
    params: TopicSubscriptionParams,
    store: tauri::State<'_, NativeSubscriptionStore>,
) -> std::result::Result<(), String> {
    if let Some(existing) = store
        .inner()
        .remove(params.topic.as_str())
        .map_err(|error| error.to_string())?
    {
        existing.stop.store(true, Ordering::Relaxed);
    }

    Ok(())
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
fn quit_ui<R: tauri::Runtime>(app: tauri::AppHandle<R>) -> String {
    #[cfg(test)]
    if let Some(harness) = app.try_state::<TestRuntimeHarness>() {
        let _ = harness.record_exit(0);
        return "Quit UI requested.".to_owned();
    }

    app.exit(0);
    "Quit UI requested.".to_owned()
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
fn stop_background_daemon_and_quit<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
) -> std::result::Result<String, String> {
    let paths = DesktopPaths::from_handle(&app).map_err(|error| error.to_string())?;
    let (descriptor, _) = read_descriptor(paths.descriptor_path().as_path());
    if let Some(descriptor) = descriptor {
        #[cfg(test)]
        if let Some(harness) = app.try_state::<TestRuntimeHarness>() {
            harness
                .record_stopped_pid(descriptor.pid)
                .map_err(|error| error.to_string())?;
        } else {
            signal_daemon_stop(descriptor.pid).map_err(|error| error.to_string())?;
        }

        #[cfg(not(test))]
        signal_daemon_stop(descriptor.pid).map_err(|error| error.to_string())?;
    }

    #[cfg(test)]
    if let Some(harness) = app.try_state::<TestRuntimeHarness>() {
        harness.record_exit(0).map_err(|error| error.to_string())?;
        return Ok("Stop background daemon and quit requested.".to_owned());
    }

    app.exit(0);
    Ok("Stop background daemon and quit requested.".to_owned())
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
fn capture_decision<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    params: CaptureDecisionParams,
) -> std::result::Result<Value, String> {
    let paths = DesktopPaths::from_handle(&app).map_err(|error| error.to_string())?;
    let idempotency_key = params
        .idempotency_key
        .clone()
        .unwrap_or_else(|| next_idempotency_key("desktop-decision"));
    let bridge_params = match bridge_capture_decision_params(&params) {
        Ok(value) => value,
        Err(error) => return serialize_bridge_result::<BridgeMutationAck>(Err(error)),
    };
    serialize_bridge_result(with_daemon_connection(&paths, |connection| {
        connection.capture_decision(&bridge_params, idempotency_key.as_str())
    }))
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
fn capture_clarification<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    params: CaptureClarificationFrontendParams,
) -> std::result::Result<Value, String> {
    let paths = DesktopPaths::from_handle(&app).map_err(|error| error.to_string())?;
    let idempotency_key = params
        .idempotency_key
        .clone()
        .unwrap_or_else(|| next_idempotency_key("desktop-clarification"));
    let bridge_params = BridgeCaptureClarificationParams {
        process_id: params.process_id,
        candidate_id: params.candidate_id,
        reason_code: params.reason_code,
        payload: params.payload,
        note: params.note,
    };
    serialize_bridge_result(with_daemon_connection(&paths, |connection| {
        connection.capture_clarification(&bridge_params, idempotency_key.as_str())
    }))
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
fn withdraw_clarification<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    params: WithdrawClarificationFrontendParams,
) -> std::result::Result<Value, String> {
    let paths = DesktopPaths::from_handle(&app).map_err(|error| error.to_string())?;
    let idempotency_key = params
        .idempotency_key
        .clone()
        .unwrap_or_else(|| next_idempotency_key("desktop-withdraw-clarification"));
    let bridge_params = BridgeWithdrawClarificationParams {
        process_id: params.process_id,
        candidate_id: params.candidate_id,
        reason_code: params.reason_code,
    };
    serialize_bridge_result(with_daemon_connection(&paths, |connection| {
        connection.withdraw_clarification(&bridge_params, idempotency_key.as_str())
    }))
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
fn withdraw_decision<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    params: WithdrawDecisionFrontendParams,
) -> std::result::Result<Value, String> {
    let paths = DesktopPaths::from_handle(&app).map_err(|error| error.to_string())?;
    let bridge_params = BridgeWithdrawDecisionParams {
        idempotency_key: params.idempotency_key,
    };
    let bridge_idempotency_key = next_idempotency_key("desktop-withdraw-decision");
    serialize_bridge_result(with_daemon_connection(&paths, |connection| {
        connection.withdraw_decision(&bridge_params, bridge_idempotency_key.as_str())
    }))
}

/// Stage 14 Phase 2 — re-runs the live Stage 11→12→13 chain so manifest rows
/// are regenerated with any newly captured operator overrides applied.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
fn trigger_pipeline_rerun<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    params: TriggerPipelineRerunParams,
) -> std::result::Result<Value, String> {
    let paths = DesktopPaths::from_handle(&app).map_err(|error| error.to_string())?;
    stop_daemon_for_exclusive_pipeline_rerun(&paths).map_err(|error| error.to_string())?;
    let rerun_result = trigger_pipeline_rerun_impl(&paths, &params);
    let restart_result = ensure_daemon_ready_after_pipeline_rerun(&paths);

    rerun_result
        .and_then(|ack| {
            restart_result?;
            Ok(ack)
        })
        .and_then(|ack| serde_json::to_value(ack).context("serializing pipeline rerun ack"))
        .map_err(|error| error.to_string())
}

fn trigger_pipeline_rerun_impl(
    paths: &DesktopPaths,
    params: &TriggerPipelineRerunParams,
) -> Result<PipelineRerunAck> {
    anyhow::ensure!(
        !params.process_id.trim().is_empty(),
        "process_id is required for pipeline rerun"
    );

    let idempotency_key = next_idempotency_key("desktop-pipeline-rerun");
    let mut completed_stages = Vec::with_capacity(PIPELINE_RERUN_INGEST_STAGES.len());
    for stage in PIPELINE_RERUN_INGEST_STAGES {
        run_daemon_ingest_stage(paths, stage)
            .with_context(|| format!("running pipeline rerun stage `{stage}`"))?;
        completed_stages.push(stage.to_owned());
    }

    Ok(PipelineRerunAck {
        accepted: true,
        visible_after_seq: read_commit_seq(paths.state_db_path().as_path())?,
        idempotency_key,
        stages: completed_stages,
    })
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
fn list_manifests<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
) -> std::result::Result<Value, String> {
    let paths = DesktopPaths::from_handle(&app).map_err(|error| error.to_string())?;
    serialize_query_result(
        query_service::QueryService::open(paths.state_db_path(), EXPECTED_SCHEMA_EPOCH)
            .and_then(|service| service.list_manifests()),
    )
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
fn get_decision_history<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    params: GetDecisionHistoryParams,
) -> std::result::Result<Value, String> {
    let paths = DesktopPaths::from_handle(&app).map_err(|error| error.to_string())?;
    serialize_query_result(
        query_service::QueryService::open(paths.state_db_path(), EXPECTED_SCHEMA_EPOCH)
            .and_then(|service| service.get_decision_history(params.process_id.as_str())),
    )
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
fn get_active_overrides<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    params: GetActiveOverridesParams,
) -> std::result::Result<Value, String> {
    let paths = DesktopPaths::from_handle(&app).map_err(|error| error.to_string())?;
    serialize_query_result(
        query_service::QueryService::open(paths.state_db_path(), EXPECTED_SCHEMA_EPOCH)
            .and_then(|service| service.get_active_overrides(params.process_id.as_str())),
    )
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
fn resolve_error<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    params: BridgeResolveErrorParams,
) -> std::result::Result<Value, String> {
    let paths = DesktopPaths::from_handle(&app).map_err(|error| error.to_string())?;
    let idempotency_key = next_idempotency_key("desktop-resolve-error");
    serialize_bridge_result(with_daemon_connection(&paths, |connection| {
        connection.resolve_error(&params, idempotency_key.as_str())
    }))
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
fn rerun_from_error<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    params: BridgeRerunFromErrorParams,
) -> std::result::Result<Value, String> {
    let paths = DesktopPaths::from_handle(&app).map_err(|error| error.to_string())?;
    let idempotency_key = next_idempotency_key("desktop-rerun-error");
    serialize_bridge_result(with_daemon_connection(&paths, |connection| {
        connection.rerun_from_error(&params, idempotency_key.as_str())
    }))
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
fn retry_job<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    params: BridgeRetryJobParams,
) -> std::result::Result<Value, String> {
    let paths = DesktopPaths::from_handle(&app).map_err(|error| error.to_string())?;
    let idempotency_key = next_idempotency_key("desktop-retry-job");
    serialize_bridge_result(with_daemon_connection(&paths, |connection| {
        connection.retry_job(&params, idempotency_key.as_str())
    }))
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
fn stop_session<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    params: BridgeStopSessionParams,
) -> std::result::Result<Value, String> {
    let paths = DesktopPaths::from_handle(&app).map_err(|error| error.to_string())?;
    let idempotency_key = next_idempotency_key("desktop-stop-session");
    serialize_bridge_result(with_daemon_connection(&paths, |connection| {
        connection.stop_session(&params, idempotency_key.as_str())
    }))
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
fn send_session_input<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    params: BridgeSendSessionInputParams,
) -> std::result::Result<Value, String> {
    let paths = DesktopPaths::from_handle(&app).map_err(|error| error.to_string())?;
    let idempotency_key = next_idempotency_key("desktop-session-input");
    serialize_bridge_result(with_daemon_connection(&paths, |connection| {
        connection.send_session_input(&params, idempotency_key.as_str())
    }))
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
fn resize_session<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    params: BridgeResizeSessionParams,
) -> std::result::Result<Value, String> {
    let paths = DesktopPaths::from_handle(&app).map_err(|error| error.to_string())?;
    let idempotency_key = next_idempotency_key("desktop-resize-session");
    serialize_bridge_result(with_daemon_connection(&paths, |connection| {
        connection.resize_session(&params, idempotency_key.as_str())
    }))
}

fn serialize_query_result<T: Serialize>(
    result: std::result::Result<T, query_service::QueryServiceError>,
) -> std::result::Result<Value, String> {
    serialize_command_payload(result)
}

fn serialize_bridge_result<T: Serialize>(
    result: std::result::Result<T, DaemonBridgeError>,
) -> std::result::Result<Value, String> {
    serialize_command_payload(result)
}

fn serialize_command_payload<T: Serialize, E: Serialize>(
    result: std::result::Result<T, E>,
) -> std::result::Result<Value, String> {
    match result {
        Ok(value) => serde_json::to_value(value).map_err(|error| error.to_string()),
        Err(error) => serde_json::to_value(error).map_err(|error| error.to_string()),
    }
}

#[allow(clippy::needless_pass_by_value)]
fn run_native_subscription_loop<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    paths: DesktopPaths,
    topic: String,
    from_seq: Option<u64>,
    stop: Arc<AtomicBool>,
) {
    let bridge = DaemonBridge::new(&paths.runtime_dir);
    let Ok(mut connection) = bridge.connect() else {
        return;
    };

    if connection
        .set_read_timeout(Some(Duration::from_millis(NATIVE_SUBSCRIPTION_POLL_MS)))
        .is_err()
    {
        return;
    }

    if connection.subscribe(topic.as_str(), from_seq).is_err() {
        return;
    }

    while !stop.load(Ordering::Relaxed) {
        match connection.read_subscription_event() {
            Ok(Some(NativeSubscriptionEvent::Topic {
                topic: emitted_topic,
                payload,
            })) => {
                let _ = app.emit(
                    native_event_name_for_topic(emitted_topic.as_str()).as_str(),
                    payload,
                );
            }
            Ok(Some(NativeSubscriptionEvent::Lifecycle {
                seq,
                session_id,
                state,
                cleanup_verdict,
                archive_seq,
            })) => {
                let _ = app.emit(
                    native_event_name_for_topic(topic.as_str()).as_str(),
                    json!({
                        "sessionId": session_id,
                        "frame": {
                            "kind": "lifecycle",
                            "seq": seq,
                            "state": state,
                            "cleanupVerdict": cleanup_verdict,
                            "archiveSeq": archive_seq,
                            "ts": Utc::now().to_rfc3339(),
                        },
                    }),
                );
            }
            Ok(None) => {}
            Err(_) => break,
        }
    }

    let _ = connection.unsubscribe(topic.as_str());
}

fn native_event_name_for_topic(topic: &str) -> String {
    let mut event_name = String::from("procintel/topic/");
    for value in topic.chars() {
        if value.is_ascii_alphanumeric() || matches!(value, '-' | '_' | '/' | ':') {
            event_name.push(value);
        } else {
            event_name.push('_');
        }
    }
    event_name
}

fn bridge_transport_unavailable(message: impl Into<String>) -> DaemonBridgeError {
    DaemonBridgeError {
        code: "transport_unavailable".to_owned(),
        message: message.into(),
        retryable: true,
        details: None,
    }
}

fn connect_daemon_bridge(
    paths: &DesktopPaths,
) -> std::result::Result<daemon_bridge::DaemonBridgeConnection, DaemonBridgeError> {
    DaemonBridge::new(&paths.runtime_dir).connect()
}

fn with_daemon_connection<T, F>(
    paths: &DesktopPaths,
    mut operation: F,
) -> std::result::Result<T, DaemonBridgeError>
where
    F: FnMut(
        &mut daemon_bridge::DaemonBridgeConnection,
    ) -> std::result::Result<T, DaemonBridgeError>,
{
    let mut connection = match connect_daemon_bridge(paths) {
        Ok(connection) => connection,
        Err(error) if error.code == "transport_unavailable" => {
            reset_daemon_launch_guard().map_err(|restart_error| {
                bridge_transport_unavailable(format!(
                    "resetting daemon launch guard after bridge failure failed: {restart_error}"
                ))
            })?;
            launch_detached_daemon(paths).map_err(|restart_error| {
                bridge_transport_unavailable(format!(
                    "restarting daemon after bridge connection failure failed: {restart_error}"
                ))
            })?;
            wait_for_daemon_ready(paths).map_err(|restart_error| {
                bridge_transport_unavailable(format!(
                    "daemon did not become ready after bridge restart: {restart_error}"
                ))
            })?;
            connect_daemon_bridge(paths)?
        }
        Err(error) => return Err(error),
    };
    operation(&mut connection)
}

fn next_idempotency_key(prefix: &str) -> String {
    format!(
        "{prefix}-{}",
        Utc::now().timestamp_nanos_opt().unwrap_or_default()
    )
}

fn bridge_capture_decision_params(
    params: &CaptureDecisionParams,
) -> std::result::Result<BridgeCaptureDecisionParams, DaemonBridgeError> {
    Ok(BridgeCaptureDecisionParams {
        process_id: params.process_id.clone(),
        disposition: match params.disposition {
            DecisionDisposition::Approve => BridgeDecisionDisposition::Approve,
            DecisionDisposition::Reject => BridgeDecisionDisposition::Reject,
            DecisionDisposition::Watch => BridgeDecisionDisposition::Watch,
        },
        reviewed_basis_hash: required_capture_metadata(
            params.reviewed_basis_hash.clone(),
            "reviewedBasisHash",
        )?,
        reviewed_manifest_hash: required_capture_metadata(
            params.reviewed_manifest_hash.clone(),
            "reviewedManifestHash",
        )?,
        reviewed_at_stage_run_id: required_capture_metadata(
            params.reviewed_at_stage_run_id.clone(),
            "reviewedAtStageRunId",
        )?,
        decision_note: params.decision_note.clone(),
    })
}

fn required_capture_metadata(
    value: Option<String>,
    field_name: &str,
) -> std::result::Result<String, DaemonBridgeError> {
    value.ok_or_else(|| DaemonBridgeError {
        code: "invalid_params".to_owned(),
        message: format!("capture_decision requires {field_name}"),
        retryable: false,
        details: None,
    })
}

#[cfg(test)]
#[allow(clippy::needless_pass_by_value)]
fn capture_decision_impl(
    db_path: &Path,
    params: CaptureDecisionParams,
) -> Result<DecisionMutationAck> {
    let mut conn = Connection::open(db_path)
        .with_context(|| format!("opening SQLite database {}", db_path.display()))?;
    conn.execute_batch(
        "\
        PRAGMA foreign_keys = ON;\
        PRAGMA busy_timeout = 5000;\
        ",
    )
    .context("configuring decision-write SQLite pragmas")?;
    ensure_decision_schema(&conn)?;

    let idempotency_key = params.idempotency_key.clone().unwrap_or_else(|| {
        format!(
            "decision-{}-{}",
            params.process_id,
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        )
    });

    if let Some(visible_after_seq) = existing_decision_receipt(&conn, idempotency_key.as_str())
        .context("reading decision receipt")?
    {
        return Ok(DecisionMutationAck {
            accepted: true,
            visible_after_seq,
            idempotency_key,
        });
    }

    let context = decision_capture_context(&conn, &params)
        .with_context(|| format!("loading decision context for {}", params.process_id))?;
    let reviewed_at = Utc::now().to_rfc3339();
    let intent_id = format!("intent-{idempotency_key}");

    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .context("starting capture_decision transaction")?;
    let visible_after_seq = increment_commit_clock_tx(&tx)?;
    let previous_intent_id = latest_unsuperseded_intent_id(&tx, context.process_id.as_str())
        .context("loading latest unsuperseded decision")?;

    tx.execute(
        "\
        INSERT INTO pending_operator_intents (
          id, process_id, desired_disposition, reviewed_basis_hash, reviewed_manifest_hash,
          reviewed_at_stage_run_id, reviewed_at, base_remote_revision, idempotency_key, status,
          superseded_by_intent_id, queued_at, decision_note, last_attempt_at, last_error
        ) VALUES (
          ?1, ?2, ?3, ?4, ?5,
          ?6, ?7, 0, ?8, 'queued',
          NULL, ?7, ?9, NULL, NULL
        )
        ",
        params![
            intent_id,
            context.process_id,
            params.disposition.as_str(),
            context.reviewed_basis_hash,
            context.reviewed_manifest_hash,
            context.reviewed_at_stage_run_id,
            reviewed_at,
            idempotency_key,
            params.decision_note,
        ],
    )
    .context("inserting pending_operator_intents row")?;

    if let Some(previous_intent_id) = previous_intent_id {
        tx.execute(
            "\
            UPDATE pending_operator_intents
            SET superseded_by_intent_id = ?1
            WHERE id = ?2 AND superseded_by_intent_id IS NULL
            ",
            params![intent_id, previous_intent_id],
        )
        .context("superseding previous decision intent")?;
    }

    tx.execute(
        "\
        INSERT INTO pending_operator_intent_receipts (
          idempotency_key, intent_id, visible_after_seq
        ) VALUES (?1, ?2, ?3)
        ",
        params![idempotency_key, intent_id, visible_after_seq],
    )
    .context("inserting pending_operator_intent_receipts row")?;

    tx.commit()
        .context("committing capture_decision transaction")?;

    Ok(DecisionMutationAck {
        accepted: true,
        visible_after_seq,
        idempotency_key,
    })
}

#[cfg(test)]
#[allow(dead_code)]
fn resolve_error_impl(db_path: &Path, error_manifest_id: &str) -> Result<OperatorMutationAck> {
    let mut conn = Connection::open(db_path)
        .with_context(|| format!("opening SQLite database {}", db_path.display()))?;
    conn.execute_batch(
        "\
        PRAGMA foreign_keys = ON;\
        PRAGMA busy_timeout = 5000;\
        ",
    )
    .context("configuring resolve_error SQLite pragmas")?;

    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .context("starting resolve_error transaction")?;
    let visible_after_seq = increment_commit_clock_tx(&tx)?;
    let updated = tx.execute(
        "\
        UPDATE error_manifests
        SET resolved = 1,
            resolved_at = ?1,
            resolved_by = 'desktop-operator'
        WHERE id = ?2
        ",
        params![Utc::now().to_rfc3339(), error_manifest_id],
    )?;
    anyhow::ensure!(
        updated == 1,
        "error manifest missing for {error_manifest_id}"
    );
    tx.commit()
        .context("committing resolve_error transaction")?;

    Ok(OperatorMutationAck {
        accepted: true,
        visible_after_seq,
        job_id: None,
    })
}

#[cfg(test)]
#[allow(dead_code)]
fn rerun_from_error_impl(db_path: &Path, error_manifest_id: &str) -> Result<OperatorMutationAck> {
    let mut conn = Connection::open(db_path)
        .with_context(|| format!("opening SQLite database {}", db_path.display()))?;
    conn.execute_batch(
        "\
        PRAGMA foreign_keys = ON;\
        PRAGMA busy_timeout = 5000;\
        ",
    )
    .context("configuring rerun_from_error SQLite pragmas")?;

    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .context("starting rerun_from_error transaction")?;
    let visible_after_seq = increment_commit_clock_tx(&tx)?;
    let (process_id, pipeline_stage) = tx
        .query_row(
            "\
            SELECT process_id, pipeline_stage
            FROM error_manifests
            WHERE id = ?1
            ",
            params![error_manifest_id],
            |row| Ok((row.get::<_, Option<String>>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()?
        .ok_or_else(|| anyhow!("error manifest missing for {error_manifest_id}"))?;

    let job_id = format!(
        "rerun-error-{}",
        Utc::now().timestamp_nanos_opt().unwrap_or_default()
    );
    tx.execute(
        "\
        INSERT INTO workflow_jobs (
          id, kind, process_id, scope_kind, scope_id, status, input_payload, created_at
        ) VALUES (?1, 'rerun_from_error', ?2, 'process', COALESCE(?2, ?1), 'pending', ?3, ?4)
        ",
        params![
            job_id,
            process_id,
            serde_json::json!({
                "error_manifest_id": error_manifest_id,
                "pipeline_stage": pipeline_stage,
            })
            .to_string(),
            Utc::now().to_rfc3339(),
        ],
    )?;
    tx.commit()
        .context("committing rerun_from_error transaction")?;

    Ok(OperatorMutationAck {
        accepted: true,
        visible_after_seq,
        job_id: Some(job_id),
    })
}

fn open_path_in_desktop(path: &Path) -> Result<()> {
    let opener = if cfg!(target_os = "macos") {
        "open"
    } else if cfg!(target_os = "windows") {
        "explorer"
    } else {
        "xdg-open"
    };

    Command::new(opener)
        .arg(path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .with_context(|| format!("opening {} with {}", path.display(), opener))?;
    Ok(())
}

fn signal_daemon_stop(pid: u32) -> Result<()> {
    #[cfg(test)]
    {
        let hook = TEST_DAEMON_STOP_HOOK
            .get_or_init(|| Mutex::new(None))
            .lock()
            .map_err(|_| anyhow!("test daemon stop hook lock poisoned"))?
            .clone();
        if let Some(hook) = hook {
            return hook(pid);
        }
    }

    Command::new("kill")
        .arg("-TERM")
        .arg(pid.to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .with_context(|| format!("sending TERM to daemon pid {pid}"))?;
    Ok(())
}

#[cfg(test)]
fn ensure_decision_schema(conn: &Connection) -> Result<()> {
    const L2_DECISION_MIGRATION_PATH: &str = "migrations/0004_l2_pending_operator_intents.sql";
    let path = repo_root()?.join(L2_DECISION_MIGRATION_PATH);
    let ddl = fs::read_to_string(&path)
        .with_context(|| format!("reading migration {}", path.display()))?;
    conn.execute_batch(&ddl)
        .with_context(|| format!("applying migration {}", path.display()))?;
    Ok(())
}

#[cfg(test)]
fn existing_decision_receipt(conn: &Connection, idempotency_key: &str) -> Result<Option<i64>> {
    if !table_exists(conn, "pending_operator_intent_receipts")? {
        return Ok(None);
    }

    conn.query_row(
        "SELECT visible_after_seq FROM pending_operator_intent_receipts WHERE idempotency_key = ?1",
        params![idempotency_key],
        |row| row.get::<_, i64>(0),
    )
    .optional()
    .context("querying pending_operator_intent_receipts")
}

#[cfg(test)]
fn decision_capture_context(
    conn: &Connection,
    params: &CaptureDecisionParams,
) -> Result<DecisionCaptureContext> {
    let process_id = resolve_process_id_for_decision(conn, params.process_id.as_str())?;

    let assessment = conn
        .query_row(
            "\
            SELECT decision_basis_hash, stage_run_id
            FROM pipeline_assessments
            WHERE process_id = ?1
            ",
            params![process_id],
            |row| {
                Ok((
                    row.get::<_, Option<String>>(0)?,
                    row.get::<_, Option<String>>(1)?,
                ))
            },
        )
        .optional()
        .context("querying pipeline_assessments for decision context")?;

    let manifest = conn
        .query_row(
            "\
            SELECT artifact_hash, stage_run_id
            FROM manifests
            WHERE process_id = ?1
            ORDER BY created_at DESC, id DESC
            LIMIT 1
            ",
            params![process_id],
            |row| {
                Ok((
                    row.get::<_, Option<String>>(0)?,
                    row.get::<_, Option<String>>(1)?,
                ))
            },
        )
        .optional()
        .context("querying manifests for decision context")?;

    let reviewed_basis_hash = params
        .reviewed_basis_hash
        .clone()
        .or_else(|| {
            assessment
                .as_ref()
                .and_then(|(basis_hash, _)| basis_hash.clone())
        })
        .ok_or_else(|| anyhow!("decision basis hash is unavailable for {process_id}"))?;
    let reviewed_manifest_hash = params
        .reviewed_manifest_hash
        .clone()
        .or_else(|| {
            manifest
                .as_ref()
                .and_then(|(artifact_hash, _)| artifact_hash.clone())
        })
        .ok_or_else(|| anyhow!("reviewed manifest hash is unavailable for {process_id}"))?;
    let reviewed_at_stage_run_id = params
        .reviewed_at_stage_run_id
        .clone()
        .or_else(|| {
            manifest
                .as_ref()
                .and_then(|(_, stage_run_id)| stage_run_id.clone())
        })
        .or_else(|| {
            assessment
                .as_ref()
                .and_then(|(_, stage_run_id)| stage_run_id.clone())
        })
        .ok_or_else(|| anyhow!("reviewed stage run id is unavailable for {process_id}"))?;

    Ok(DecisionCaptureContext {
        process_id,
        reviewed_basis_hash,
        reviewed_manifest_hash,
        reviewed_at_stage_run_id,
    })
}

#[cfg(test)]
fn resolve_process_id_for_decision(
    conn: &Connection,
    requested_process_id: &str,
) -> Result<String> {
    if table_exists(conn, "process_aliases")?
        && let Some(canonical_process_id) = conn
            .query_row(
                "SELECT canonical_process_id FROM process_aliases WHERE alias_process_id = ?1",
                params![requested_process_id],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .context("querying process_aliases")?
    {
        return Ok(canonical_process_id);
    }

    conn.query_row(
        "\
        SELECT id
        FROM procurement_processes
        WHERE id = ?1 OR canonical_process_id = ?1
        ORDER BY CASE WHEN id = ?1 THEN 0 ELSE 1 END
        LIMIT 1
        ",
        params![requested_process_id],
        |row| row.get::<_, String>(0),
    )
    .optional()
    .context("querying procurement_processes for decision target")?
    .ok_or_else(|| anyhow!("unknown process `{requested_process_id}`"))
}

#[cfg(test)]
fn latest_unsuperseded_intent_id(tx: &Transaction<'_>, process_id: &str) -> Result<Option<String>> {
    tx.query_row(
        "\
        SELECT id
        FROM pending_operator_intents
        WHERE process_id = ?1 AND superseded_by_intent_id IS NULL
        ORDER BY queued_at DESC, id DESC
        LIMIT 1
        ",
        params![process_id],
        |row| row.get::<_, String>(0),
    )
    .optional()
    .context("querying latest unsuperseded decision intent")
}

#[cfg(test)]
fn increment_commit_clock_tx(tx: &Transaction<'_>) -> Result<i64> {
    let updated = tx
        .execute(
            "UPDATE commit_clock SET commit_seq = commit_seq + 1 WHERE id = 1",
            [],
        )
        .context("incrementing commit_clock for decision mutation")?;
    anyhow::ensure!(updated == 1, "commit_clock singleton row missing");
    tx.query_row(
        "SELECT commit_seq FROM commit_clock WHERE id = 1",
        [],
        |row| row.get::<_, i64>(0),
    )
    .context("reading committed decision visible_after_seq")
}

#[cfg(test)]
fn table_exists(conn: &Connection, table_name: &str) -> Result<bool> {
    conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
        params![table_name],
        |row| row.get::<_, i64>(0),
    )
    .map(|count| count == 1)
    .context("checking sqlite table existence")
}

#[cfg(test)]
fn repo_root() -> Result<PathBuf> {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .ancestors()
        .nth(3)
        .map(Path::to_path_buf)
        .ok_or_else(|| {
            anyhow!(
                "failed to resolve repository root from {}",
                manifest_dir.display()
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(unix)]
    use std::os::unix::net::UnixListener;
    use tempfile::tempdir;

    static TEST_RUNTIME_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    fn lock_test_runtime() -> Result<std::sync::MutexGuard<'static, ()>> {
        TEST_RUNTIME_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .map_err(|_| anyhow!("test runtime lock poisoned"))
    }

    fn seeded_db() -> Result<(tempfile::TempDir, PathBuf)> {
        let temp = tempdir()?;
        let db_path = temp.path().join("state.db");
        let conn = Connection::open(&db_path)?;
        conn.execute_batch(
            "\
            PRAGMA foreign_keys = ON;
            CREATE TABLE procurement_processes (
              id TEXT PRIMARY KEY,
              canonical_process_id TEXT
            );
            CREATE TABLE pipeline_assessments (
              process_id TEXT PRIMARY KEY REFERENCES procurement_processes(id),
              decision_basis_hash TEXT,
              stage_run_id TEXT
            );
            CREATE TABLE manifests (
              id TEXT PRIMARY KEY,
              process_id TEXT NOT NULL REFERENCES procurement_processes(id),
              artifact_hash TEXT,
              stage_run_id TEXT,
              created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );
            CREATE TABLE commit_clock (
              id INTEGER PRIMARY KEY CHECK (id = 1),
              commit_seq INTEGER NOT NULL DEFAULT 0
            );
            INSERT INTO commit_clock (id, commit_seq) VALUES (1, 0);
            INSERT INTO procurement_processes (id, canonical_process_id) VALUES ('process-1', 'process-1');
            INSERT INTO pipeline_assessments (process_id, decision_basis_hash, stage_run_id)
            VALUES ('process-1', 'basis-a', 'stage-a');
            INSERT INTO manifests (id, process_id, artifact_hash, stage_run_id)
            VALUES ('manifest-1', 'process-1', 'manifest-a', 'stage-a');
            ",
        )?;
        Ok((temp, db_path))
    }

    #[test]
    fn bug_runtime_dir_override_regression_desktop_paths_can_isolate_daemon_runtime() -> Result<()>
    {
        let temp = tempdir()?;
        let base_dir = temp.path().join("base");
        let state_root = temp.path().join("state-root");
        let runtime_dir = temp.path().join("isolated-runtime");
        let artifact_root = temp.path().join("artifact-root");

        let paths = DesktopPaths::from_base_dir_with_overrides(
            base_dir.clone(),
            Some(state_root.clone()),
            Some(runtime_dir.clone()),
            Some(artifact_root.clone()),
        );

        anyhow::ensure!(paths.state_root == state_root);
        anyhow::ensure!(paths.runtime_dir == runtime_dir);
        anyhow::ensure!(paths.artifact_root == artifact_root);
        anyhow::ensure!(paths.state_db_path() == state_root.join(STATE_DB_FILENAME));
        anyhow::ensure!(paths.descriptor_path() == runtime_dir.join(DESCRIPTOR_FILENAME));
        anyhow::ensure!(paths.socket_path() == runtime_dir.join(SOCKET_FILENAME));

        let default_paths = DesktopPaths::from_base_dir(base_dir.clone());
        anyhow::ensure!(default_paths.state_root == base_dir.join(STATE_DIRNAME));
        anyhow::ensure!(default_paths.runtime_dir == base_dir.join(RUNTIME_DIRNAME));
        anyhow::ensure!(default_paths.artifact_root == base_dir.join(ARTIFACT_DIRNAME));
        Ok(())
    }

    #[test]
    fn bug_pipeline_rerun_regression_runs_live_ingest_chain_instead_of_unsupported_queue_job()
    -> Result<()> {
        let _guard = lock_test_runtime()?;
        reset_test_runtime_state()?;
        let temp = tempdir()?;
        let paths = DesktopPaths::from_base_dir(temp.path());
        fs::create_dir_all(paths.state_root.as_path())?;
        let conn = Connection::open(paths.state_db_path())?;
        conn.execute_batch(
            "\
            CREATE TABLE commit_clock (
              id INTEGER PRIMARY KEY CHECK (id = 1),
              commit_seq INTEGER NOT NULL
            );
            INSERT INTO commit_clock (id, commit_seq) VALUES (1, 42);
            ",
        )?;

        let seen = Arc::new(Mutex::new(Vec::<String>::new()));
        let seen_for_hook = Arc::clone(&seen);
        set_test_pipeline_rerun_hook(move |stage| {
            seen_for_hook
                .lock()
                .map_err(|_| anyhow!("test stage list lock poisoned"))?
                .push(stage.to_owned());
            Ok(())
        })?;

        let ack = trigger_pipeline_rerun_impl(
            &paths,
            &TriggerPipelineRerunParams {
                process_id: "process-1".to_owned(),
            },
        )?;

        anyhow::ensure!(ack.accepted);
        anyhow::ensure!(ack.visible_after_seq == 42);
        anyhow::ensure!(
            ack.stages
                == vec![
                    "landed-cost".to_owned(),
                    "net-gate".to_owned(),
                    "manifest".to_owned()
                ]
        );
        anyhow::ensure!(
            *seen
                .lock()
                .map_err(|_| anyhow!("test stage list lock poisoned"))?
                == ack.stages
        );
        reset_test_runtime_state()?;
        Ok(())
    }

    #[test]
    fn bug_pipeline_rerun_regression_releases_desktop_daemon_lock_before_ingest() -> Result<()> {
        let _guard = lock_test_runtime()?;
        reset_test_runtime_state()?;
        let temp = tempdir()?;
        let paths = DesktopPaths::from_base_dir(temp.path());
        fs::create_dir_all(paths.runtime_dir.as_path())?;
        fs::create_dir_all(paths.state_root.as_path())?;
        fs::write(
            paths.descriptor_path(),
            serde_json::to_vec(&DaemonDescriptorSnapshot {
                instance_id: "daemon-for-rerun-test".to_owned(),
                pid: 4242,
                socket_path: paths.socket_path().display().to_string(),
                protocol_version: EXPECTED_PROTOCOL_VERSION,
                schema_epoch: EXPECTED_SCHEMA_EPOCH,
                state_root: paths.state_root.display().to_string(),
                started_at: "2026-05-03T00:00:00Z".to_owned(),
                ready: true,
            })?,
        )?;

        let running = Arc::new(AtomicBool::new(true));
        let running_for_check = Arc::clone(&running);
        set_test_process_running_hook(move |pid| {
            anyhow::ensure!(pid == 4242);
            Ok(running_for_check.load(Ordering::Relaxed))
        })?;

        let stopped_pids = Arc::new(Mutex::new(Vec::<u32>::new()));
        let stopped_pids_for_hook = Arc::clone(&stopped_pids);
        let running_for_stop = Arc::clone(&running);
        set_test_daemon_stop_hook(move |pid| {
            stopped_pids_for_hook
                .lock()
                .map_err(|_| anyhow!("test stopped-pids lock poisoned"))?
                .push(pid);
            running_for_stop.store(false, Ordering::Relaxed);
            Ok(())
        })?;

        anyhow::ensure!(stop_daemon_for_exclusive_pipeline_rerun(&paths)?);
        anyhow::ensure!(
            *stopped_pids
                .lock()
                .map_err(|_| anyhow!("test stopped-pids lock poisoned"))?
                == vec![4242]
        );
        anyhow::ensure!(!running.load(Ordering::Relaxed));

        reset_test_runtime_state()?;
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn bug_pipeline_rerun_regression_defunct_daemon_pid_is_not_running() -> Result<()> {
        let _guard = lock_test_runtime()?;
        reset_test_runtime_state()?;
        let mut child = Command::new("sh")
            .arg("-c")
            .arg("exit 0")
            .spawn()
            .context("spawning short-lived child process")?;
        thread::sleep(Duration::from_millis(100));

        let running = process_is_running(child.id())?;
        let _status = child.wait().context("reaping short-lived child process")?;

        anyhow::ensure!(!running, "defunct child should not count as running");
        reset_test_runtime_state()?;
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn bug_pipeline_rerun_regression_restarts_after_stale_descriptor() -> Result<()> {
        let _guard = lock_test_runtime()?;
        reset_test_runtime_state()?;
        let temp = tempdir()?;
        let paths = DesktopPaths::from_base_dir(temp.path());
        fs::create_dir_all(paths.runtime_dir.as_path())?;
        fs::create_dir_all(paths.state_root.as_path())?;
        fs::write(
            paths.descriptor_path(),
            serde_json::to_vec(&DaemonDescriptorSnapshot {
                instance_id: "stale-daemon-for-rerun-test".to_owned(),
                pid: 4242,
                socket_path: paths.socket_path().display().to_string(),
                protocol_version: EXPECTED_PROTOCOL_VERSION,
                schema_epoch: EXPECTED_SCHEMA_EPOCH,
                state_root: paths.state_root.display().to_string(),
                started_at: "2026-05-03T00:00:00Z".to_owned(),
                ready: true,
            })?,
        )?;
        let (stale_descriptor, _) = read_descriptor(paths.descriptor_path().as_path());
        anyhow::ensure!(
            try_handshake(&paths, stale_descriptor.as_ref(), None).is_err(),
            "stale descriptor unexpectedly accepted a daemon handshake"
        );

        let launches = Arc::new(Mutex::new(0usize));
        let launches_for_hook = Arc::clone(&launches);
        set_test_daemon_launch_hook(move |state_root, runtime_dir, _artifact_root| {
            *launches_for_hook
                .lock()
                .map_err(|_| anyhow!("test launch count lock poisoned"))? += 1;

            let socket_path = runtime_dir.join(SOCKET_FILENAME);
            let listener = UnixListener::bind(socket_path.as_path())
                .with_context(|| format!("binding fake daemon socket {}", socket_path.display()))?;
            let descriptor = DaemonDescriptorSnapshot {
                instance_id: "fresh-daemon-for-rerun-test".to_owned(),
                pid: 5252,
                socket_path: socket_path.display().to_string(),
                protocol_version: EXPECTED_PROTOCOL_VERSION,
                schema_epoch: EXPECTED_SCHEMA_EPOCH,
                state_root: state_root.display().to_string(),
                started_at: "2026-05-03T00:00:01Z".to_owned(),
                ready: true,
            };
            fs::write(
                runtime_dir.join(DESCRIPTOR_FILENAME),
                serde_json::to_vec(&descriptor)?,
            )?;

            thread::spawn(move || {
                let Ok((mut stream, _addr)) = listener.accept() else {
                    return;
                };
                let mut request = String::new();
                {
                    let mut reader = BufReader::new(&mut stream);
                    if reader.read_line(&mut request).is_err() {
                        return;
                    }
                }
                let response = json!({
                    "messageType": "welcome",
                    "selectedProtocolVersion": EXPECTED_PROTOCOL_VERSION,
                    "daemonVersion": "test",
                    "instanceId": "fresh-daemon-for-rerun-test",
                    "schemaEpoch": EXPECTED_SCHEMA_EPOCH,
                    "commitSeq": 1,
                    "healthState": { "status": "ready" },
                    "mutationsAllowed": true,
                    "activeSessions": { "count": 0, "ids": [], "states": [] },
                    "reconnectOutcome": "freshConnection",
                    "streamResumePermitted": true
                });
                let _ = writeln!(stream, "{response}");
            });
            Ok(())
        })?;

        ensure_daemon_ready_after_pipeline_rerun(&paths)?;
        anyhow::ensure!(
            *launches
                .lock()
                .map_err(|_| anyhow!("test launch count lock poisoned"))?
                == 1
        );

        reset_test_runtime_state()?;
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn bug_bridge_restart_regression_recovers_from_stale_daemon_socket() -> Result<()> {
        let _guard = lock_test_runtime()?;
        reset_test_runtime_state()?;
        let temp = tempdir()?;
        let paths = DesktopPaths::from_base_dir(temp.path());
        fs::create_dir_all(paths.runtime_dir.as_path())?;
        fs::create_dir_all(paths.state_root.as_path())?;
        fs::write(
            paths.descriptor_path(),
            serde_json::to_vec(&DaemonDescriptorSnapshot {
                instance_id: "stale-daemon-for-bridge-test".to_owned(),
                pid: 6262,
                socket_path: paths.socket_path().display().to_string(),
                protocol_version: EXPECTED_PROTOCOL_VERSION,
                schema_epoch: EXPECTED_SCHEMA_EPOCH,
                state_root: paths.state_root.display().to_string(),
                started_at: "2026-05-03T00:00:00Z".to_owned(),
                ready: true,
            })?,
        )?;

        let launches = Arc::new(Mutex::new(0usize));
        let launches_for_hook = Arc::clone(&launches);
        set_test_daemon_launch_hook(move |state_root, runtime_dir, _artifact_root| {
            *launches_for_hook
                .lock()
                .map_err(|_| anyhow!("test launch count lock poisoned"))? += 1;

            let socket_path = runtime_dir.join(SOCKET_FILENAME);
            let listener = UnixListener::bind(socket_path.as_path())
                .with_context(|| format!("binding fake daemon socket {}", socket_path.display()))?;
            let descriptor = DaemonDescriptorSnapshot {
                instance_id: "fresh-daemon-for-bridge-test".to_owned(),
                pid: 6363,
                socket_path: socket_path.display().to_string(),
                protocol_version: EXPECTED_PROTOCOL_VERSION,
                schema_epoch: EXPECTED_SCHEMA_EPOCH,
                state_root: state_root.display().to_string(),
                started_at: "2026-05-03T00:00:01Z".to_owned(),
                ready: true,
            };
            fs::write(
                runtime_dir.join(DESCRIPTOR_FILENAME),
                serde_json::to_vec(&descriptor)?,
            )?;

            thread::spawn(move || {
                for _ in 0..2 {
                    let Ok((mut stream, _addr)) = listener.accept() else {
                        return;
                    };
                    let mut request = String::new();
                    {
                        let mut reader = BufReader::new(&mut stream);
                        if reader.read_line(&mut request).is_err() {
                            return;
                        }
                    }
                    let response = json!({
                        "messageType": "welcome",
                        "selectedProtocolVersion": EXPECTED_PROTOCOL_VERSION,
                        "daemonVersion": "test",
                        "instanceId": "fresh-daemon-for-bridge-test",
                        "schemaEpoch": EXPECTED_SCHEMA_EPOCH,
                        "commitSeq": 7,
                        "healthState": { "status": "ready" },
                        "mutationsAllowed": true,
                        "activeSessions": { "count": 0, "ids": [], "states": [] },
                        "reconnectOutcome": "freshConnection",
                        "streamResumePermitted": true
                    });
                    let _ = writeln!(stream, "{response}");
                }
            });
            Ok(())
        })?;

        let instance_id = with_daemon_connection(&paths, |connection| {
            Ok(connection.welcome().instance_id.clone())
        })?;

        anyhow::ensure!(instance_id == "fresh-daemon-for-bridge-test");
        anyhow::ensure!(
            *launches
                .lock()
                .map_err(|_| anyhow!("test launch count lock poisoned"))?
                == 1
        );

        reset_test_runtime_state()?;
        Ok(())
    }

    #[test]
    fn capture_decision_persists_required_review_metadata() -> Result<()> {
        let (_temp, db_path) = seeded_db()?;
        let ack = capture_decision_impl(
            db_path.as_path(),
            CaptureDecisionParams {
                process_id: "process-1".to_owned(),
                disposition: DecisionDisposition::Approve,
                reviewed_basis_hash: None,
                reviewed_manifest_hash: None,
                reviewed_at_stage_run_id: None,
                decision_note: None,
                idempotency_key: Some("decision-key-1".to_owned()),
            },
        )?;

        anyhow::ensure!(ack.accepted);
        anyhow::ensure!(ack.visible_after_seq == 1);

        let conn = Connection::open(&db_path)?;
        let row: (String, String, String, String, String, String, String) = conn.query_row(
            "\
            SELECT process_id, desired_disposition, reviewed_basis_hash, reviewed_manifest_hash,
                   reviewed_at_stage_run_id, status, idempotency_key
            FROM pending_operator_intents
            WHERE idempotency_key = 'decision-key-1'
            ",
            [],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                ))
            },
        )?;
        anyhow::ensure!(row.0 == "process-1");
        anyhow::ensure!(row.1 == "approve");
        anyhow::ensure!(row.2 == "basis-a");
        anyhow::ensure!(row.3 == "manifest-a");
        anyhow::ensure!(row.4 == "stage-a");
        anyhow::ensure!(row.5 == "queued");
        anyhow::ensure!(row.6 == "decision-key-1");
        Ok(())
    }

    #[test]
    fn capture_decision_is_idempotent_for_same_key() -> Result<()> {
        let (_temp, db_path) = seeded_db()?;
        let first = capture_decision_impl(
            db_path.as_path(),
            CaptureDecisionParams {
                process_id: "process-1".to_owned(),
                disposition: DecisionDisposition::Watch,
                reviewed_basis_hash: None,
                reviewed_manifest_hash: None,
                reviewed_at_stage_run_id: None,
                decision_note: Some("track".to_owned()),
                idempotency_key: Some("decision-key-2".to_owned()),
            },
        )?;
        let second = capture_decision_impl(
            db_path.as_path(),
            CaptureDecisionParams {
                process_id: "process-1".to_owned(),
                disposition: DecisionDisposition::Watch,
                reviewed_basis_hash: None,
                reviewed_manifest_hash: None,
                reviewed_at_stage_run_id: None,
                decision_note: Some("track".to_owned()),
                idempotency_key: Some("decision-key-2".to_owned()),
            },
        )?;

        anyhow::ensure!(first.visible_after_seq == second.visible_after_seq);
        let conn = Connection::open(&db_path)?;
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM pending_operator_intents WHERE idempotency_key = 'decision-key-2'",
            [],
            |row| row.get(0),
        )?;
        anyhow::ensure!(count == 1);
        Ok(())
    }

    #[test]
    fn capture_decision_supersedes_previous_unsuperseded_intent() -> Result<()> {
        let (_temp, db_path) = seeded_db()?;
        let first = capture_decision_impl(
            db_path.as_path(),
            CaptureDecisionParams {
                process_id: "process-1".to_owned(),
                disposition: DecisionDisposition::Approve,
                reviewed_basis_hash: Some("basis-a".to_owned()),
                reviewed_manifest_hash: Some("manifest-a".to_owned()),
                reviewed_at_stage_run_id: Some("stage-a".to_owned()),
                decision_note: None,
                idempotency_key: Some("decision-key-3".to_owned()),
            },
        )?;
        let second = capture_decision_impl(
            db_path.as_path(),
            CaptureDecisionParams {
                process_id: "process-1".to_owned(),
                disposition: DecisionDisposition::Reject,
                reviewed_basis_hash: Some("basis-b".to_owned()),
                reviewed_manifest_hash: Some("manifest-b".to_owned()),
                reviewed_at_stage_run_id: Some("stage-b".to_owned()),
                decision_note: None,
                idempotency_key: Some("decision-key-4".to_owned()),
            },
        )?;

        anyhow::ensure!(second.visible_after_seq > first.visible_after_seq);
        let conn = Connection::open(&db_path)?;
        let superseded: Option<String> = conn.query_row(
            "SELECT superseded_by_intent_id FROM pending_operator_intents WHERE idempotency_key = 'decision-key-3'",
            [],
            |row| row.get(0),
        )?;
        let latest_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM pending_operator_intents WHERE process_id = 'process-1' AND superseded_by_intent_id IS NULL",
            [],
            |row| row.get(0),
        )?;
        anyhow::ensure!(superseded.is_some());
        anyhow::ensure!(latest_count == 1);
        Ok(())
    }

    #[test]
    fn bug_native_subscription_event_name_regression_sanitizes_topic_dots() {
        assert_eq!(
            native_event_name_for_topic("daemon.health"),
            "procintel/topic/daemon_health"
        );
        assert_eq!(
            native_event_name_for_topic("sessions"),
            "procintel/topic/sessions"
        );
    }
}
