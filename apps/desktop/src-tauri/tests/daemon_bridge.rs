#![forbid(unsafe_code)]

#[path = "../src/daemon_bridge.rs"]
#[allow(dead_code)]
mod daemon_bridge_impl;

use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

use daemon_bridge_impl::{
    BridgeError, CaptureDecisionParams, DaemonBridge, DaemonBridgeConfig, DaemonCommandMethod,
    DaemonInfoSnapshot, DecisionDisposition, GetDaemonInfoParams, LifecyclePhase, MutationAck,
    StreamOutputKind, StructuredSessionEvent, TriggerIngestionParams, daemon_health_topic,
    gap_frame, project_lifecycle_frame, project_output_frame, project_structured_event,
    session_topic,
};
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

static NEXT_RUNTIME_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone)]
struct TempRuntime {
    dir: PathBuf,
}

impl TempRuntime {
    fn new() -> std::io::Result<Self> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |duration| duration.as_nanos());
        let unique = NEXT_RUNTIME_ID.fetch_add(1, Ordering::Relaxed);
        let dir =
            PathBuf::from("/tmp").join(format!("pdb-{}-{timestamp}-{unique}", std::process::id()));
        fs::create_dir_all(&dir)?;
        Ok(Self { dir })
    }

    fn socket_path(&self) -> PathBuf {
        self.dir.join("daemon.sock")
    }

    fn descriptor_path(&self) -> PathBuf {
        self.dir.join("daemon.info.json")
    }

    fn write_descriptor(&self) -> std::io::Result<()> {
        fs::write(
            self.descriptor_path(),
            serde_json::to_vec(&json!({
                "instanceId": "daemon-1",
                "pid": 42,
                "socketPath": self.socket_path().display().to_string(),
                "protocolVersion": 1,
                "schemaEpoch": 1,
                "stateRoot": self.dir.display().to_string(),
                "startedAt": "2026-03-28T18:00:00Z",
                "ready": true
            }))?,
        )
    }
}

impl Drop for TempRuntime {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

fn bridge(runtime: &TempRuntime) -> DaemonBridge {
    DaemonBridge::with_config(DaemonBridgeConfig::new(runtime.dir.clone()))
}

fn write_server_message(stream: &mut UnixStream, message: &Value) -> Result<(), BridgeError> {
    let payload = serde_json::to_string(message).map_err(|error| BridgeError {
        code: "transport_unavailable".to_owned(),
        message: error.to_string(),
        retryable: true,
        details: None,
    })?;
    stream
        .write_all(payload.as_bytes())
        .map_err(|error| BridgeError {
            code: "transport_unavailable".to_owned(),
            message: error.to_string(),
            retryable: true,
            details: None,
        })?;
    stream.write_all(b"\n").map_err(|error| BridgeError {
        code: "transport_unavailable".to_owned(),
        message: error.to_string(),
        retryable: true,
        details: None,
    })?;
    stream.flush().map_err(|error| BridgeError {
        code: "transport_unavailable".to_owned(),
        message: error.to_string(),
        retryable: true,
        details: None,
    })
}

fn read_client_value(stream: &UnixStream) -> serde_json::Result<Value> {
    let mut reader = BufReader::new(stream);
    let mut buffer = String::new();
    let _ = reader.read_line(&mut buffer);
    serde_json::from_str(buffer.trim_end())
}

fn welcome_message() -> Value {
    serde_json::to_value(json!({
        "messageType": "welcome",
        "selectedProtocolVersion": 1,
        "daemonVersion": "1.2.3",
        "instanceId": "daemon-1",
        "schemaEpoch": 1,
        "commitSeq": 77,
        "healthState": { "status": "ready" },
        "mutationsAllowed": true,
        "activeSessions": {
            "count": 1,
            "ids": ["session-1"],
            "states": ["active"]
        },
        "reconnectOutcome": "fresh_connection",
        "streamResumePermitted": false
    }))
    .expect("welcome json should serialize")
}

#[test]
fn rs_dbr_001_connects_with_the_current_hello_welcome_contract() -> TestResult {
    let runtime = TempRuntime::new()?;
    runtime.write_descriptor()?;
    let listener = UnixListener::bind(runtime.socket_path())?;
    let server = thread::spawn(
        move || -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
            let (mut stream, _) = listener.accept()?;
            let hello = read_client_value(&stream)?;
            write_server_message(&mut stream, &welcome_message())?;
            Ok(hello)
        },
    );

    let connection = bridge(&runtime).connect()?;
    assert_eq!(connection.welcome().daemon_version, "1.2.3");
    assert_eq!(connection.welcome().commit_seq, 77);
    assert_eq!(connection.welcome().active_sessions.count, 1);

    let hello = server.join().expect("server should join")?;
    assert_eq!(hello["messageType"], "hello");
    assert_eq!(hello["desktopVersion"], "procintel-desktop");
    assert_eq!(hello["supportedProtocolRange"]["min"], 1);
    Ok(())
}

#[test]
fn rs_dbr_002_get_daemon_info_is_a_read_command_without_idempotency() -> TestResult {
    let runtime = TempRuntime::new()?;
    runtime.write_descriptor()?;
    let listener = UnixListener::bind(runtime.socket_path())?;
    let server_runtime = runtime.clone();
    let server = thread::spawn(
        move || -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
            let (mut stream, _) = listener.accept()?;
            let _hello = read_client_value(&stream)?;
            write_server_message(&mut stream, &welcome_message())?;
            let command = read_client_value(&stream)?;
            write_server_message(
                &mut stream,
                &json!({
                    "messageType": "commandResponse",
                    "requestId": command["requestId"],
                    "payload": {
                        "instanceId": "daemon-1",
                        "daemonVersion": "1.2.3",
                        "protocolVersion": 1,
                        "schemaEpoch": 1,
                        "socketPath": server_runtime.socket_path().display().to_string(),
                        "descriptorPath": server_runtime.descriptor_path().display().to_string(),
                        "stateRoot": server_runtime.dir.display().to_string(),
                        "stateDbPath": server_runtime.dir.join("state.db").display().to_string(),
                        "artifactRoot": server_runtime.dir.join("artifacts").display().to_string(),
                        "walBytes": 512,
                        "startedAt": "2026-03-28T18:00:00Z",
                        "startupRecovery": {
                            "recoveredInterruptedSessions": 0,
                            "lastRecoveryAt": null
                        }
                    }
                }),
            )?;
            Ok(command)
        },
    );

    let mut connection = bridge(&runtime).connect()?;
    let info: DaemonInfoSnapshot = connection.get_daemon_info()?;
    assert_eq!(info.protocol_version, 1);
    assert_eq!(info.wal_bytes, Some(512));

    let command = server.join().expect("server should join")?;
    assert_eq!(command["messageType"], "command");
    assert_eq!(command["method"], "get_daemon_info");
    assert_eq!(
        command["params"],
        serde_json::to_value(GetDaemonInfoParams {})?
    );
    assert_eq!(command["idempotencyKey"], Value::Null);
    Ok(())
}

#[test]
fn rs_dbr_003_mutation_commands_preserve_method_names_and_adapt_daemon_acks() -> TestResult {
    let runtime = TempRuntime::new()?;
    runtime.write_descriptor()?;
    let listener = UnixListener::bind(runtime.socket_path())?;
    let server = thread::spawn(
        move || -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
            let (mut stream, _) = listener.accept()?;
            let _hello = read_client_value(&stream)?;
            write_server_message(&mut stream, &welcome_message())?;
            let command = read_client_value(&stream)?;
            write_server_message(
                &mut stream,
                &json!({
                    "messageType": "commandResponse",
                    "requestId": command["requestId"],
                    "payload": {
                        "accepted": true,
                        "visible_after_seq": 88,
                        "continuation_job_id": "job-123"
                    }
                }),
            )?;
            Ok(command)
        },
    );

    let mut connection = bridge(&runtime).connect()?;
    let ack: MutationAck = connection.trigger_ingestion(
        &TriggerIngestionParams {
            source_id: "fts".to_owned(),
        },
        "idem-1",
    )?;

    assert!(ack.accepted);
    assert_eq!(ack.idempotency_key, "idem-1");
    assert_eq!(ack.visible_after_seq, 88);
    assert_eq!(ack.job_id.as_deref(), Some("job-123"));

    let command = server.join().expect("server should join")?;
    assert_eq!(command["method"], "trigger_ingestion");
    assert_eq!(command["params"]["sourceId"], "fts");
    assert_eq!(command["idempotencyKey"], "idem-1");
    Ok(())
}

#[test]
fn rs_dbr_004_transport_failures_stay_distinct_from_daemon_refusals() -> TestResult {
    let runtime = TempRuntime::new()?;
    let error = bridge(&runtime)
        .connect()
        .expect_err("missing descriptor must fail");
    assert_eq!(error.code, "transport_unavailable");
    assert!(error.retryable);
    Ok(())
}

#[test]
fn rs_dbr_005_protocol_rejections_surface_explicit_bridge_codes() -> TestResult {
    let runtime = TempRuntime::new()?;
    runtime.write_descriptor()?;
    let listener = UnixListener::bind(runtime.socket_path())?;
    let server = thread::spawn(
        move || -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            let (mut stream, _) = listener.accept()?;
            let _hello = read_client_value(&stream)?;
            write_server_message(
                &mut stream,
                &json!({
                    "messageType": "rejection",
                    "code": "protocol_range_mismatch",
                    "message": "ranges do not overlap",
                    "supportedProtocolRange": { "min": 1, "max": 1 },
                    "expectedSchemaEpoch": null,
                    "actualSchemaEpoch": null
                }),
            )?;
            Ok(())
        },
    );

    let error = bridge(&runtime)
        .connect()
        .expect_err("rejection should bubble");
    assert_eq!(error.code, "protocol_range_mismatch");
    assert!(!error.retryable);
    assert!(error.details.is_some());
    server.join().expect("server should join")?;
    Ok(())
}

#[test]
fn rs_dbr_006_lifecycle_frames_do_not_poison_command_responses() -> TestResult {
    let runtime = TempRuntime::new()?;
    runtime.write_descriptor()?;
    let listener = UnixListener::bind(runtime.socket_path())?;
    let server = thread::spawn(
        move || -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            let (mut stream, _) = listener.accept()?;
            let _hello = read_client_value(&stream)?;
            write_server_message(&mut stream, &welcome_message())?;
            write_server_message(
                &mut stream,
                &json!({
                    "messageType": "lifecycle",
                    "seq": 1,
                    "sessionId": "session-1",
                    "phase": "active"
                }),
            )?;
            let command = read_client_value(&stream)?;
            write_server_message(
                &mut stream,
                &json!({
                    "messageType": "commandResponse",
                    "requestId": command["requestId"],
                    "payload": {
                        "accepted": true,
                        "visible_after_seq": 19,
                        "continuation_job_id": null
                    }
                }),
            )?;
            Ok(())
        },
    );

    let mut connection = bridge(&runtime).connect()?;
    let ack = connection.capture_decision(
        &CaptureDecisionParams {
            process_id: "proc-1".to_owned(),
            disposition: DecisionDisposition::Watch,
            reviewed_basis_hash: "basis-1".to_owned(),
            reviewed_manifest_hash: "manifest-1".to_owned(),
            reviewed_at_stage_run_id: "stage-1".to_owned(),
        },
        "idem-2",
    )?;
    assert_eq!(ack.visible_after_seq, 19);
    server.join().expect("server should join")?;
    Ok(())
}

#[test]
fn rs_dbr_007_closed_method_set_stays_stable() {
    let methods = DaemonCommandMethod::ALL
        .iter()
        .map(|method| method.as_str())
        .collect::<Vec<_>>();
    assert_eq!(methods.len(), 11);
    assert_eq!(methods[0], "get_daemon_info");
    assert!(methods.contains(&"capture_decision"));
    assert!(methods.contains(&"resolve_error"));
    assert!(methods.contains(&"rerun_from_error"));
}

#[test]
fn rs_dbr_008_stream_projection_stays_aligned_with_public_frame_contract() -> TestResult {
    assert_eq!(daemon_health_topic(), "daemon.health");
    assert_eq!(session_topic("session-1"), "sessions.session-1");

    let pty = project_output_frame(StreamOutputKind::Pty, 7, b"abc", "2026-03-28T20:00:00Z");
    let stdout = project_output_frame(
        StreamOutputKind::Stdout,
        8,
        b"stdout-frame",
        "2026-03-28T20:00:01Z",
    );
    let stderr = project_output_frame(
        StreamOutputKind::Stderr,
        9,
        b"stderr-frame",
        "2026-03-28T20:00:02Z",
    );
    let auth = project_structured_event(
        10,
        StructuredSessionEvent::AuthChallenge {
            kind: "browser_only".to_owned(),
            detail: "reveal_profile_path".to_owned(),
        },
        "2026-03-28T20:00:03Z",
    );
    let gap = gap_frame(11, 14);

    assert_eq!(
        serde_json::to_value(pty)?["kind"],
        Value::String("pty".to_owned())
    );
    assert_eq!(
        serde_json::to_value(stdout)?["kind"],
        Value::String("stdout".to_owned())
    );
    assert_eq!(
        serde_json::to_value(stderr)?["kind"],
        Value::String("stderr".to_owned())
    );
    assert_eq!(
        serde_json::to_value(auth)?["event"]["type"],
        Value::String("authChallenge".to_owned())
    );
    assert_eq!(
        serde_json::to_value(gap)?["kind"],
        Value::String("gap".to_owned())
    );

    Ok(())
}

#[test]
fn rs_dbr_009_commands_only_flow_after_successful_handshake() -> TestResult {
    let runtime = TempRuntime::new()?;
    runtime.write_descriptor()?;
    let listener = UnixListener::bind(runtime.socket_path())?;
    let server = thread::spawn(
        move || -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
            let (mut stream, _) = listener.accept()?;
            let hello = read_client_value(&stream)?;
            write_server_message(&mut stream, &welcome_message())?;
            let command = read_client_value(&stream)?;
            write_server_message(
                &mut stream,
                &json!({
                    "messageType": "error",
                    "requestId": command["requestId"],
                    "code": "mutations_unavailable",
                    "message": "router disabled",
                    "retryable": false,
                    "details": null
                }),
            )?;
            Ok(hello)
        },
    );

    let mut connection = bridge(&runtime).connect()?;
    let error = connection
        .trigger_ingestion(
            &TriggerIngestionParams {
                source_id: "fts".to_owned(),
            },
            "idem-dr22",
        )
        .expect_err("mutation should be refused by handshake-only server");
    assert_eq!(error.code, "mutations_unavailable");

    let hello = server.join().expect("server should join")?;
    assert_eq!(hello["messageType"], "hello");
    Ok(())
}

#[test]
fn rs_dbr_010_lifecycle_frames_keep_terminal_metadata_rules() -> TestResult {
    let ordered = vec![
        project_lifecycle_frame(1, "session-1", LifecyclePhase::Created, None, None)?,
        project_lifecycle_frame(2, "session-1", LifecyclePhase::Spawned, None, None)?,
        project_lifecycle_frame(3, "session-1", LifecyclePhase::Active, None, None)?,
        project_lifecycle_frame(4, "session-1", LifecyclePhase::Terminating, None, None)?,
        project_lifecycle_frame(5, "session-1", LifecyclePhase::Exited, None, None)?,
        project_lifecycle_frame(
            6,
            "session-1",
            LifecyclePhase::Reaped,
            Some("forced".to_owned()),
            None,
        )?,
        project_lifecycle_frame(7, "session-1", LifecyclePhase::Archived, None, Some(27))?,
    ];

    let phases: Vec<Value> = ordered
        .into_iter()
        .map(serde_json::to_value)
        .collect::<Result<_, _>>()?;
    assert_eq!(phases[0]["phase"], "created");
    assert_eq!(phases[5]["cleanupVerdict"], "forced");
    assert_eq!(phases[6]["archiveSeq"], 27);

    assert!(project_lifecycle_frame(8, "session-1", LifecyclePhase::Reaped, None, None).is_err());
    assert!(
        project_lifecycle_frame(
            9,
            "session-1",
            LifecyclePhase::Active,
            Some("clean".to_owned()),
            None,
        )
        .is_err()
    );

    Ok(())
}
