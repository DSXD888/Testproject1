#![forbid(unsafe_code)]

#[allow(dead_code)]
mod daemon_bridge;
#[allow(dead_code)]
mod query_service;

use std::fs;
use std::io::{BufRead, BufReader, Write};
#[cfg(unix)]
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::Duration;

use crate::daemon_bridge::{DaemonBridge, RestartDaemonParams};
use anyhow::{Context, Result, anyhow};
use chrono::Utc;
use rusqlite::{Connection, OptionalExtension, Transaction, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use tauri::Manager;

const DESCRIPTOR_FILENAME: &str = "daemon.info.json";
const SOCKET_FILENAME: &str = "daemon.sock";
const STATE_DIRNAME: &str = "state";
const RUNTIME_DIRNAME: &str = "runtime";
const ARTIFACT_DIRNAME: &str = "artifacts";
const STATE_DB_FILENAME: &str = "state.db";
const L2_DECISION_MIGRATION_PATH: &str = "migrations/0004_l2_pending_operator_intents.sql";
const EXPECTED_PROTOCOL_VERSION: u32 = 1;
const EXPECTED_SCHEMA_EPOCH: i64 = 1;
const DEFAULT_DAEMON_PROGRAM: &str = "daemon";
const HANDSHAKE_RETRY_BACKOFF_MS: [u64; 6] = [150, 300, 600, 1_000, 1_500, 2_000];

static LAUNCH_GUARD: OnceLock<Mutex<bool>> = OnceLock::new();

fn main() -> Result<()> {
    tauri::Builder::default()
        .manage(BootstrapStore::default())
        .invoke_handler(tauri::generate_handler![
            get_bootstrap_snapshot,
            refresh_bootstrap_snapshot,
            list_errors,
            get_daemon_info,
            restart_daemon,
            quit_ui,
            stop_background_daemon_and_quit,
            open_logs,
            open_artifacts,
            capture_decision,
            resolve_error,
            rerun_from_error
        ])
        .setup(|app| {
            let snapshot = bootstrap_application(app.handle());
            app.state::<BootstrapStore>().replace(snapshot)?;
            Ok(())
        })
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
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct BootstrapSnapshot {
    mode: BootstrapMode,
    control_plane: ControlPlaneState,
    descriptor: Option<DaemonDescriptorSnapshot>,
    launched_daemon: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DaemonInfoSnapshot {
    daemon_instance_id: String,
    daemon_version: String,
    protocol_version: u32,
    schema_epoch: i64,
    socket_path: String,
    descriptor_path: String,
    db_path: String,
    artifact_root: String,
    wal_bytes: u64,
    startup_recovery_summary: String,
    live_available: bool,
    descriptor_backed: bool,
    storage: String,
    daemon: String,
    control: String,
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

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum DecisionDisposition {
    Approve,
    Reject,
    Watch,
}

impl DecisionDisposition {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Approve => "approve",
            Self::Reject => "reject",
            Self::Watch => "watch",
        }
    }
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct DecisionMutationAck {
    accepted: bool,
    visible_after_seq: i64,
    idempotency_key: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct DecisionCaptureContext {
    process_id: String,
    reviewed_basis_hash: String,
    reviewed_manifest_hash: String,
    reviewed_at_stage_run_id: String,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
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
        let base_dir = app
            .path()
            .app_local_data_dir()
            .context("resolving app-local data directory")?;
        Ok(Self {
            state_root: base_dir.join(STATE_DIRNAME),
            runtime_dir: base_dir.join(RUNTIME_DIRNAME),
            artifact_root: base_dir.join(ARTIFACT_DIRNAME),
        })
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

fn bootstrap_application<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> BootstrapSnapshot {
    match DesktopPaths::from_handle(app) {
        Ok(paths) => run_startup_sequence(&paths),
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

fn run_startup_sequence(paths: &DesktopPaths) -> BootstrapSnapshot {
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

    match try_handshake(paths, descriptor.as_ref()) {
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

    finish_handshake_retries(paths, storage.state, descriptor, reasons, launched_daemon)
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
        let frame = read_handshake_frame(socket_path.as_path())?;
        parse_handshake_frame(frame)
    }

    #[cfg(not(unix))]
    {
        let _ = paths;
        let _ = descriptor;
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

fn spawn_program(mut command: Command, paths: &DesktopPaths, source: &str) -> Result<()> {
    command
        .arg("--serve")
        .arg("--state-root")
        .arg(&paths.state_root)
        .arg("--runtime-dir")
        .arg(&paths.runtime_dir)
        .arg("--artifact-root")
        .arg(&paths.artifact_root)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

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
        last_seen_instance_id: None,
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
) -> BootstrapSnapshot {
    for backoff_ms in HANDSHAKE_RETRY_BACKOFF_MS {
        thread::park_timeout(Duration::from_millis(backoff_ms));
        let (fresh_descriptor, descriptor_reasons) =
            read_descriptor(paths.descriptor_path().as_path());
        reasons.extend(descriptor_reasons);
        let retry_descriptor = fresh_descriptor.as_ref().or(descriptor.as_ref());
        match try_handshake(paths, retry_descriptor) {
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
fn get_bootstrap_snapshot(
    app: tauri::AppHandle,
    store: tauri::State<'_, BootstrapStore>,
) -> std::result::Result<BootstrapSnapshot, String> {
    if let Some(snapshot) = store
        .inner()
        .snapshot()
        .map_err(|error| error.to_string())?
    {
        return Ok(snapshot);
    }

    let snapshot = bootstrap_application(&app);
    store
        .inner()
        .replace(snapshot.clone())
        .map_err(|error| error.to_string())?;
    Ok(snapshot)
}

#[tauri::command]
#[allow(dead_code)]
#[allow(clippy::needless_pass_by_value)]
fn refresh_bootstrap_snapshot(
    app: tauri::AppHandle,
    store: tauri::State<'_, BootstrapStore>,
) -> std::result::Result<BootstrapSnapshot, String> {
    let snapshot = bootstrap_application(&app);
    store
        .inner()
        .replace(snapshot.clone())
        .map_err(|error| error.to_string())?;
    Ok(snapshot)
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
fn list_errors(
    app: tauri::AppHandle,
    cursor: Option<String>,
    limit: Option<usize>,
    filters: Option<query_service::ErrorListFilters>,
) -> std::result::Result<
    query_service::QueryEnvelope<query_service::KeysetPage<query_service::ErrorListItem>>,
    String,
> {
    let paths = DesktopPaths::from_handle(&app).map_err(|error| error.to_string())?;
    let service = query_service::QueryService::open(paths.state_db_path(), EXPECTED_SCHEMA_EPOCH)
        .map_err(|error| error.to_string())?;
    service
        .list_errors(
            cursor.as_deref(),
            limit.unwrap_or(25),
            &filters.unwrap_or_default(),
        )
        .map_err(|error| error.to_string())
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
fn get_daemon_info(
    app: tauri::AppHandle,
    store: tauri::State<'_, BootstrapStore>,
) -> std::result::Result<DaemonInfoSnapshot, String> {
    let snapshot = if let Some(snapshot) = store
        .inner()
        .snapshot()
        .map_err(|error| error.to_string())?
    {
        snapshot
    } else {
        let snapshot = bootstrap_application(&app);
        store
            .inner()
            .replace(snapshot.clone())
            .map_err(|error| error.to_string())?;
        snapshot
    };
    let paths = DesktopPaths::from_handle(&app).map_err(|error| error.to_string())?;
    let wal_path = PathBuf::from(format!("{}-wal", paths.state_db_path().display()));
    let wal_bytes = fs::metadata(&wal_path).map_or(0_u64, |metadata| metadata.len());

    Ok(DaemonInfoSnapshot {
        daemon_instance_id: snapshot.control_plane.daemon_instance_id.clone(),
        daemon_version: snapshot.control_plane.daemon_version.clone(),
        protocol_version: snapshot.control_plane.protocol_version,
        schema_epoch: snapshot.control_plane.schema_epoch,
        socket_path: paths.socket_path().display().to_string(),
        descriptor_path: paths.descriptor_path().display().to_string(),
        db_path: paths.state_db_path().display().to_string(),
        artifact_root: paths.artifact_root.display().to_string(),
        wal_bytes,
        startup_recovery_summary: snapshot.control_plane.reasons.join("\n"),
        live_available: snapshot.control_plane.daemon != DaemonState::Unavailable,
        descriptor_backed: snapshot.control_plane.daemon == DaemonState::Unavailable,
        storage: storage_label(snapshot.control_plane.storage).to_owned(),
        daemon: daemon_label(snapshot.control_plane.daemon).to_owned(),
        control: control_label(snapshot.control_plane.control).to_owned(),
    })
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
fn restart_daemon(
    app: tauri::AppHandle,
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
    let refreshed = bootstrap_application(&app);
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
fn open_logs(app: tauri::AppHandle) -> std::result::Result<String, String> {
    let paths = DesktopPaths::from_handle(&app).map_err(|error| error.to_string())?;
    open_path_in_desktop(paths.runtime_dir.as_path()).map_err(|error| error.to_string())?;
    Ok(paths.runtime_dir.display().to_string())
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
fn open_artifacts(app: tauri::AppHandle) -> std::result::Result<String, String> {
    let paths = DesktopPaths::from_handle(&app).map_err(|error| error.to_string())?;
    open_path_in_desktop(paths.artifact_root.as_path()).map_err(|error| error.to_string())?;
    Ok(paths.artifact_root.display().to_string())
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
fn quit_ui(app: tauri::AppHandle) -> String {
    app.exit(0);
    "Quit UI requested.".to_owned()
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
fn stop_background_daemon_and_quit(app: tauri::AppHandle) -> std::result::Result<String, String> {
    let paths = DesktopPaths::from_handle(&app).map_err(|error| error.to_string())?;
    let (descriptor, _) = read_descriptor(paths.descriptor_path().as_path());
    if let Some(descriptor) = descriptor {
        signal_daemon_stop(descriptor.pid).map_err(|error| error.to_string())?;
    }
    app.exit(0);
    Ok("Stop background daemon and quit requested.".to_owned())
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
fn capture_decision(
    app: tauri::AppHandle,
    params: CaptureDecisionParams,
) -> std::result::Result<DecisionMutationAck, String> {
    let paths = DesktopPaths::from_handle(&app).map_err(|error| error.to_string())?;
    capture_decision_impl(paths.state_db_path().as_path(), params)
        .map_err(|error| error.to_string())
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
fn resolve_error(
    app: tauri::AppHandle,
    error_manifest_id: String,
) -> std::result::Result<OperatorMutationAck, String> {
    let paths = DesktopPaths::from_handle(&app).map_err(|error| error.to_string())?;
    resolve_error_impl(paths.state_db_path().as_path(), error_manifest_id.as_str())
        .map_err(|error| error.to_string())
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
fn rerun_from_error(
    app: tauri::AppHandle,
    error_manifest_id: String,
) -> std::result::Result<OperatorMutationAck, String> {
    let paths = DesktopPaths::from_handle(&app).map_err(|error| error.to_string())?;
    rerun_from_error_impl(paths.state_db_path().as_path(), error_manifest_id.as_str())
        .map_err(|error| error.to_string())
}

const fn storage_label(value: StorageState) -> &'static str {
    match value {
        StorageState::Ready => "ready",
        StorageState::Repairing => "repairing",
        StorageState::Unreadable => "unreadable",
        StorageState::SchemaMismatch => "schema_mismatch",
    }
}

const fn daemon_label(value: DaemonState) -> &'static str {
    match value {
        DaemonState::Starting => "starting",
        DaemonState::Ready => "ready",
        DaemonState::Degraded => "degraded",
        DaemonState::Unavailable => "unavailable",
        DaemonState::Incompatible => "incompatible",
    }
}

const fn control_label(value: ControlState) -> &'static str {
    match value {
        ControlState::Full => "full",
        ControlState::Limited => "limited",
        ControlState::ReadOnly => "read_only",
    }
}

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

fn ensure_decision_schema(conn: &Connection) -> Result<()> {
    let path = repo_root()?.join(L2_DECISION_MIGRATION_PATH);
    let ddl = fs::read_to_string(&path)
        .with_context(|| format!("reading migration {}", path.display()))?;
    conn.execute_batch(&ddl)
        .with_context(|| format!("applying migration {}", path.display()))?;
    Ok(())
}

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

fn table_exists(conn: &Connection, table_name: &str) -> Result<bool> {
    conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
        params![table_name],
        |row| row.get::<_, i64>(0),
    )
    .map(|count| count == 1)
    .context("checking sqlite table existence")
}

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
    use tempfile::tempdir;

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
}
