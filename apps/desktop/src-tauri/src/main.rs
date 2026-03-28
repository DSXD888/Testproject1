#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![forbid(unsafe_code)]

use std::env;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

use anyhow::{Context, Result, ensure};
use rusqlite::{Connection, OpenFlags};
use serde::{Deserialize, Serialize};

const DEFAULT_SCHEMA_EPOCH: i64 = 1;
const DEFAULT_PROTOCOL_VERSION: u32 = 1;
const DESCRIPTOR_FILENAME: &str = "daemon.info.json";
const RETRY_DELAY: Duration = Duration::from_millis(200);

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum RuntimeMode {
    FullControl,
    ControlLimited,
    BrowseOnly,
    Incompatible,
    RecoveryRequired,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum StorageState {
    Ready,
    Repairing,
    Unreadable,
    SchemaMismatch,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum DaemonState {
    #[allow(dead_code)]
    Starting,
    Ready,
    Degraded,
    Unavailable,
    Incompatible,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum ControlAccess {
    Full,
    Limited,
    ReadOnly,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum LoadState {
    Normal,
    Busy,
    Saturated,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct ControlPlaneState {
    daemon_instance_id: Option<String>,
    daemon_version: String,
    protocol_version: u32,
    schema_epoch: i64,
    storage: StorageState,
    daemon: DaemonState,
    control: ControlAccess,
    load: LoadState,
    reasons: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct BootstrapSnapshot {
    mode: RuntimeMode,
    control_plane: ControlPlaneState,
    launched_daemon: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct StorageProbe {
    state: StorageState,
    reasons: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct BootstrapPaths {
    repo_root: PathBuf,
    state_root: PathBuf,
    runtime_dir: PathBuf,
    artifact_root: PathBuf,
}

impl BootstrapPaths {
    fn from_process_env() -> Self {
        let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../..")
            .canonicalize()
            .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.."));
        let state_root = env::var_os("PROCINTEL_STATE_ROOT")
            .map_or_else(|| repo_root.join("state"), PathBuf::from);
        let runtime_dir = env::var_os("PROCINTEL_RUNTIME_DIR")
            .map_or_else(|| repo_root.join("runtime"), PathBuf::from);
        let artifact_root = env::var_os("PROCINTEL_ARTIFACT_ROOT")
            .map_or_else(|| repo_root.join("artifacts"), PathBuf::from);

        Self {
            repo_root,
            state_root,
            runtime_dir,
            artifact_root,
        }
    }

    fn state_db_path(&self) -> PathBuf {
        self.state_root.join("state.db")
    }

    fn descriptor_path(&self) -> PathBuf {
        self.runtime_dir.join(DESCRIPTOR_FILENAME)
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct DaemonDescriptor {
    instance_id: String,
    socket_path: String,
    protocol_version: u32,
    schema_epoch: i64,
    state_root: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct HealthState {
    status: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct ActiveSessionSummary {
    count: usize,
    ids: Vec<String>,
    states: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct ProtocolRange {
    min: u32,
    max: u32,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
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
        active_sessions: ActiveSessionSummary,
        reconnect_outcome: String,
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

#[derive(Clone, Debug, Serialize)]
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

#[derive(Clone, Debug, PartialEq, Eq)]
enum HandshakeAttempt {
    Welcome {
        daemon_version: String,
        instance_id: String,
        protocol_version: u32,
        schema_epoch: i64,
        daemon_health: String,
        mutations_allowed: bool,
    },
    Unavailable(String),
    Incompatible(String),
}

trait BootstrapRuntime {
    fn check_storage(&mut self, paths: &BootstrapPaths) -> Result<StorageProbe>;
    fn read_descriptor(&mut self, paths: &BootstrapPaths) -> Result<Option<DaemonDescriptor>>;
    fn handshake(&mut self, descriptor: &DaemonDescriptor) -> Result<HandshakeAttempt>;
    fn launch_daemon(&mut self, paths: &BootstrapPaths) -> Result<()>;
    fn wait_before_retry(&mut self);
}

struct ProcessBootstrapRuntime;

impl BootstrapRuntime for ProcessBootstrapRuntime {
    fn check_storage(&mut self, paths: &BootstrapPaths) -> Result<StorageProbe> {
        let state_db_path = paths.state_db_path();
        if !state_db_path.exists() {
            return Ok(StorageProbe {
                state: StorageState::Ready,
                reasons: vec!["State database will be initialized by daemon launch.".to_owned()],
            });
        }

        let conn =
            match Connection::open_with_flags(&state_db_path, OpenFlags::SQLITE_OPEN_READ_ONLY) {
                Ok(conn) => conn,
                Err(error) => {
                    return Ok(StorageProbe {
                        state: StorageState::Unreadable,
                        reasons: vec![format!(
                            "Unable to open state database {}: {error}",
                            state_db_path.display()
                        )],
                    });
                }
            };
        let quick_check: String = conn.query_row("PRAGMA quick_check", [], |row| row.get(0))?;
        if quick_check != "ok" {
            return Ok(StorageProbe {
                state: StorageState::Repairing,
                reasons: vec![format!("SQLite quick_check returned `{quick_check}`.")],
            });
        }

        let user_version: i64 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;
        if user_version != DEFAULT_SCHEMA_EPOCH {
            return Ok(StorageProbe {
                state: StorageState::SchemaMismatch,
                reasons: vec![format!(
                    "Schema epoch mismatch: desktop expects {DEFAULT_SCHEMA_EPOCH}, DB has {user_version}.",
                )],
            });
        }

        Ok(StorageProbe {
            state: StorageState::Ready,
            reasons: vec!["State database passed quick_check and schema validation.".to_owned()],
        })
    }

    fn read_descriptor(&mut self, paths: &BootstrapPaths) -> Result<Option<DaemonDescriptor>> {
        match fs::read_to_string(paths.descriptor_path()) {
            Ok(contents) => {
                let descriptor = serde_json::from_str::<DaemonDescriptor>(&contents)
                    .context("parsing daemon descriptor")?;
                Ok(Some(descriptor))
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error).context("reading daemon descriptor"),
        }
    }

    fn handshake(&mut self, descriptor: &DaemonDescriptor) -> Result<HandshakeAttempt> {
        #[cfg(unix)]
        {
            let stream = match std::os::unix::net::UnixStream::connect(&descriptor.socket_path) {
                Ok(stream) => stream,
                Err(error) => {
                    return Ok(HandshakeAttempt::Unavailable(format!(
                        "daemon socket unavailable: {error}"
                    )));
                }
            };

            let hello = ClientMessage::Hello {
                desktop_version: env!("CARGO_PKG_VERSION").to_owned(),
                supported_protocol_range: ProtocolRange {
                    min: DEFAULT_PROTOCOL_VERSION,
                    max: DEFAULT_PROTOCOL_VERSION,
                },
                expected_schema_epoch: DEFAULT_SCHEMA_EPOCH,
                last_seen_instance_id: None,
            };

            let mut writer = stream.try_clone().context("cloning daemon socket writer")?;
            serde_json::to_writer(&mut writer, &hello).context("serializing hello frame")?;
            writer.write_all(b"\n").context("terminating hello frame")?;
            writer.flush().context("flushing hello frame")?;

            let mut line = String::new();
            let mut reader = BufReader::new(stream);
            reader
                .read_line(&mut line)
                .context("reading daemon welcome frame")?;
            let response =
                serde_json::from_str::<ServerMessage>(&line).context("parsing daemon response")?;

            Ok(match response {
                ServerMessage::Welcome {
                    daemon_version,
                    instance_id,
                    selected_protocol_version,
                    schema_epoch,
                    health_state,
                    mutations_allowed,
                    ..
                } => HandshakeAttempt::Welcome {
                    daemon_version,
                    instance_id,
                    protocol_version: selected_protocol_version,
                    schema_epoch,
                    daemon_health: health_state.status,
                    mutations_allowed,
                },
                ServerMessage::Rejection { code, message, .. } => {
                    if code == "protocol_range_mismatch" || code == "schema_epoch_mismatch" {
                        HandshakeAttempt::Incompatible(message)
                    } else {
                        HandshakeAttempt::Unavailable(message)
                    }
                }
            })
        }

        #[cfg(not(unix))]
        {
            let _ = descriptor;
            Ok(HandshakeAttempt::Unavailable(
                "named-pipe bootstrap is not implemented on this platform yet".to_owned(),
            ))
        }
    }

    fn launch_daemon(&mut self, paths: &BootstrapPaths) -> Result<()> {
        let mut command = env::var_os("PROCINTEL_DAEMON_BIN").map_or_else(
            || {
                let mut command = Command::new("cargo");
                command
                    .current_dir(&paths.repo_root)
                    .arg("run")
                    .arg("-p")
                    .arg("daemon")
                    .arg("--")
                    .arg("--state-root")
                    .arg(&paths.state_root)
                    .arg("--runtime-dir")
                    .arg(&paths.runtime_dir)
                    .arg("--artifact-root")
                    .arg(&paths.artifact_root)
                    .arg("--serve");
                command
            },
            |binary| {
                let mut command = Command::new(binary);
                command
                    .arg("--state-root")
                    .arg(&paths.state_root)
                    .arg("--runtime-dir")
                    .arg(&paths.runtime_dir)
                    .arg("--artifact-root")
                    .arg(&paths.artifact_root)
                    .arg("--serve");
                command
            },
        );

        command
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .context("launching detached daemon process")?;
        Ok(())
    }

    fn wait_before_retry(&mut self) {
        thread::park_timeout(RETRY_DELAY);
    }
}

const fn derive_mode(state: &ControlPlaneState) -> RuntimeMode {
    if matches!(
        state.storage,
        StorageState::Unreadable | StorageState::Repairing
    ) {
        return RuntimeMode::RecoveryRequired;
    }
    if matches!(state.storage, StorageState::SchemaMismatch)
        || matches!(state.daemon, DaemonState::Incompatible)
    {
        return RuntimeMode::Incompatible;
    }
    if matches!(state.daemon, DaemonState::Unavailable) {
        return RuntimeMode::BrowseOnly;
    }
    if matches!(state.daemon, DaemonState::Starting | DaemonState::Degraded)
        || matches!(
            state.control,
            ControlAccess::Limited | ControlAccess::ReadOnly
        )
        || matches!(state.load, LoadState::Busy | LoadState::Saturated)
    {
        return RuntimeMode::ControlLimited;
    }
    RuntimeMode::FullControl
}

fn bootstrap(
    runtime: &mut impl BootstrapRuntime,
    paths: &BootstrapPaths,
) -> Result<BootstrapSnapshot> {
    let mut launched_daemon = false;
    let mut storage = runtime.check_storage(paths)?;
    let mut descriptor = runtime.read_descriptor(paths)?;
    let mut handshake = descriptor
        .as_ref()
        .map(|descriptor| runtime.handshake(descriptor))
        .transpose()?
        .unwrap_or_else(|| {
            HandshakeAttempt::Unavailable(
                "daemon descriptor missing before first handshake".to_owned(),
            )
        });

    if matches!(handshake, HandshakeAttempt::Unavailable(_)) {
        runtime.launch_daemon(paths)?;
        launched_daemon = true;
        runtime.wait_before_retry();
        storage = runtime.check_storage(paths)?;
        descriptor = runtime.read_descriptor(paths)?;
        handshake = descriptor
            .as_ref()
            .map(|descriptor| runtime.handshake(descriptor))
            .transpose()?
            .unwrap_or_else(|| {
                HandshakeAttempt::Unavailable(
                    "daemon descriptor missing after detached launch attempt".to_owned(),
                )
            });
    }

    Ok(BootstrapSnapshot {
        mode: derive_mode(&control_plane_from(storage.clone(), handshake.clone())),
        control_plane: control_plane_from(storage, handshake),
        launched_daemon,
    })
}

fn control_plane_from(storage: StorageProbe, handshake: HandshakeAttempt) -> ControlPlaneState {
    let mut reasons = storage.reasons;
    let (daemon_instance_id, daemon_version, protocol_version, schema_epoch, daemon, control, load) =
        match handshake {
            HandshakeAttempt::Welcome {
                daemon_version,
                instance_id,
                protocol_version,
                schema_epoch,
                daemon_health,
                mutations_allowed,
            } => {
                let daemon = if daemon_health == "ready" {
                    DaemonState::Ready
                } else {
                    reasons.push(format!("Daemon reported `{daemon_health}` health."));
                    DaemonState::Degraded
                };
                let control = if daemon_health == "ready" && mutations_allowed {
                    ControlAccess::Full
                } else if mutations_allowed {
                    ControlAccess::Limited
                } else {
                    reasons.push("Daemon handshake reported mutations disabled.".to_owned());
                    ControlAccess::ReadOnly
                };
                let load = if daemon_health == "saturated" {
                    LoadState::Saturated
                } else if daemon_health == "busy" {
                    LoadState::Busy
                } else {
                    LoadState::Normal
                };
                (
                    Some(instance_id),
                    daemon_version,
                    protocol_version,
                    schema_epoch,
                    daemon,
                    control,
                    load,
                )
            }
            HandshakeAttempt::Unavailable(reason) => {
                reasons.push(reason);
                (
                    None,
                    "unavailable".to_owned(),
                    DEFAULT_PROTOCOL_VERSION,
                    DEFAULT_SCHEMA_EPOCH,
                    DaemonState::Unavailable,
                    ControlAccess::ReadOnly,
                    LoadState::Normal,
                )
            }
            HandshakeAttempt::Incompatible(reason) => {
                reasons.push(reason);
                (
                    None,
                    "incompatible".to_owned(),
                    DEFAULT_PROTOCOL_VERSION,
                    DEFAULT_SCHEMA_EPOCH,
                    DaemonState::Incompatible,
                    ControlAccess::ReadOnly,
                    LoadState::Normal,
                )
            }
        };

    ControlPlaneState {
        daemon_instance_id,
        daemon_version,
        protocol_version,
        schema_epoch,
        storage: storage.state,
        daemon,
        control,
        load,
        reasons,
    }
}

fn fallback_snapshot(error: &anyhow::Error) -> BootstrapSnapshot {
    let control_plane = ControlPlaneState {
        daemon_instance_id: None,
        daemon_version: "bootstrap-error".to_owned(),
        protocol_version: DEFAULT_PROTOCOL_VERSION,
        schema_epoch: DEFAULT_SCHEMA_EPOCH,
        storage: StorageState::Repairing,
        daemon: DaemonState::Unavailable,
        control: ControlAccess::ReadOnly,
        load: LoadState::Normal,
        reasons: vec![format!("Desktop bootstrap failed: {error}")],
    };
    BootstrapSnapshot {
        mode: RuntimeMode::RecoveryRequired,
        control_plane,
        launched_daemon: false,
    }
}

fn main() {
    let snapshot = bootstrap(
        &mut ProcessBootstrapRuntime,
        &BootstrapPaths::from_process_env(),
    )
    .unwrap_or_else(|error| fallback_snapshot(&error));

    if let Err(error) = tauri::Builder::default()
        .manage(snapshot)
        .run(tauri::generate_context!())
    {
        eprintln!("error while running desktop shell: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct FakeRuntime {
        steps: Vec<&'static str>,
        storage_checks: Vec<StorageProbe>,
        descriptors: Vec<Option<DaemonDescriptor>>,
        handshakes: Vec<HandshakeAttempt>,
        launches: usize,
    }

    impl BootstrapRuntime for FakeRuntime {
        fn check_storage(&mut self, _paths: &BootstrapPaths) -> Result<StorageProbe> {
            self.steps.push("storage");
            Ok(self.storage_checks.remove(0))
        }

        fn read_descriptor(&mut self, _paths: &BootstrapPaths) -> Result<Option<DaemonDescriptor>> {
            self.steps.push("descriptor");
            Ok(self.descriptors.remove(0))
        }

        fn handshake(&mut self, _descriptor: &DaemonDescriptor) -> Result<HandshakeAttempt> {
            self.steps.push("handshake");
            Ok(self.handshakes.remove(0))
        }

        fn launch_daemon(&mut self, _paths: &BootstrapPaths) -> Result<()> {
            self.steps.push("launch");
            self.launches += 1;
            Ok(())
        }

        fn wait_before_retry(&mut self) {
            self.steps.push("wait");
        }
    }

    fn ready_storage() -> StorageProbe {
        StorageProbe {
            state: StorageState::Ready,
            reasons: vec!["ready".to_owned()],
        }
    }

    fn descriptor() -> DaemonDescriptor {
        DaemonDescriptor {
            instance_id: "daemon-1".to_owned(),
            socket_path: "/tmp/daemon.sock".to_owned(),
            protocol_version: DEFAULT_PROTOCOL_VERSION,
            schema_epoch: DEFAULT_SCHEMA_EPOCH,
            state_root: "/tmp/state".to_owned(),
        }
    }

    #[test]
    fn runtime_mode_precedence_is_total() -> Result<()> {
        let base = ControlPlaneState {
            daemon_instance_id: Some("daemon-1".to_owned()),
            daemon_version: "1.0.0".to_owned(),
            protocol_version: 1,
            schema_epoch: 1,
            storage: StorageState::Ready,
            daemon: DaemonState::Ready,
            control: ControlAccess::Full,
            load: LoadState::Normal,
            reasons: vec![],
        };

        let mut recovering = base.clone();
        recovering.storage = StorageState::Repairing;
        ensure!(derive_mode(&recovering) == RuntimeMode::RecoveryRequired);

        let mut incompatible = base.clone();
        incompatible.daemon = DaemonState::Incompatible;
        ensure!(derive_mode(&incompatible) == RuntimeMode::Incompatible);

        let mut browse_only = base.clone();
        browse_only.daemon = DaemonState::Unavailable;
        ensure!(derive_mode(&browse_only) == RuntimeMode::BrowseOnly);

        let mut limited = base.clone();
        limited.load = LoadState::Busy;
        ensure!(derive_mode(&limited) == RuntimeMode::ControlLimited);

        ensure!(derive_mode(&base) == RuntimeMode::FullControl);
        Ok(())
    }

    #[test]
    fn descriptor_read_precedes_single_flight_launch() -> Result<()> {
        let mut runtime = FakeRuntime {
            storage_checks: vec![ready_storage(), ready_storage()],
            descriptors: vec![Some(descriptor()), Some(descriptor())],
            handshakes: vec![
                HandshakeAttempt::Unavailable("no daemon".to_owned()),
                HandshakeAttempt::Welcome {
                    daemon_version: "1.0.0".to_owned(),
                    instance_id: "daemon-2".to_owned(),
                    protocol_version: 1,
                    schema_epoch: 1,
                    daemon_health: "ready".to_owned(),
                    mutations_allowed: true,
                },
            ],
            ..FakeRuntime::default()
        };

        let snapshot = bootstrap(&mut runtime, &BootstrapPaths::from_process_env())?;

        ensure!(snapshot.launched_daemon);
        ensure!(runtime.launches == 1);
        ensure!(
            runtime.steps
                == vec![
                    "storage",
                    "descriptor",
                    "handshake",
                    "launch",
                    "wait",
                    "storage",
                    "descriptor",
                    "handshake"
                ]
        );
        ensure!(snapshot.mode == RuntimeMode::FullControl);
        Ok(())
    }

    #[test]
    fn failed_retry_falls_back_to_browse_only() -> Result<()> {
        let mut runtime = FakeRuntime {
            storage_checks: vec![ready_storage(), ready_storage()],
            descriptors: vec![Some(descriptor()), None],
            handshakes: vec![HandshakeAttempt::Unavailable("no daemon".to_owned())],
            ..FakeRuntime::default()
        };

        let snapshot = bootstrap(&mut runtime, &BootstrapPaths::from_process_env())?;

        ensure!(snapshot.launched_daemon);
        ensure!(snapshot.mode == RuntimeMode::BrowseOnly);
        Ok(())
    }
}
