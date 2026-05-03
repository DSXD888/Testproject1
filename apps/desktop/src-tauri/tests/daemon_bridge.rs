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
    DaemonInfoSnapshot, DecisionDisposition, GetDaemonInfoParams, MutationAck,
    NativeSubscriptionEvent, ReconnectOutcome, SessionFrame, TriggerIngestionParams,
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
                "schemaEpoch": 7,
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
        "schemaEpoch": 7,
        "commitSeq": 77,
        "healthState": { "status": "ready" },
        "mutationsAllowed": true,
        "activeSessions": {
            "count": 1,
            "ids": ["session-1"],
            "states": ["active"]
        },
        "reconnectOutcome": "freshConnection",
        "streamResumePermitted": false
    }))
    .expect("welcome json should serialize")
}

#[test]
fn bug_reconnect_outcome_regression_accepts_camel_case_daemon_welcome() -> TestResult {
    let runtime = TempRuntime::new()?;
    runtime.write_descriptor()?;
    let listener = UnixListener::bind(runtime.socket_path())?;
    let server = thread::spawn(
        move || -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            let (mut stream, _) = listener.accept()?;
            let _hello = read_client_value(&stream)?;
            write_server_message(&mut stream, &welcome_message())?;
            Ok(())
        },
    );

    let bridge = bridge(&runtime);
    let connection = bridge.connect()?;

    assert_eq!(
        connection.welcome().reconnect_outcome,
        ReconnectOutcome::FreshConnection
    );
    server
        .join()
        .map_err(|_| "camel-case welcome server panicked")??;
    Ok(())
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
                        "schemaEpoch": 7,
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
            decision_note: None,
        },
        "idem-2",
    )?;
    assert_eq!(ack.visible_after_seq, 19);
    server.join().expect("server should join")?;
    Ok(())
}

#[test]
fn rs_dbr_006b_topic_events_do_not_poison_command_responses() -> TestResult {
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
                    "messageType": "topicEvent",
                    "topic": "daemon.health",
                    "payload": {
                        "status": "degraded"
                    }
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
                        "visible_after_seq": 21,
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
            decision_note: None,
        },
        "idem-topic",
    )?;
    assert_eq!(ack.visible_after_seq, 21);
    server.join().expect("server should join")?;
    Ok(())
}

#[test]
fn bug_buffered_lifecycle_response_regression_preserves_command_response() -> TestResult {
    let runtime = TempRuntime::new()?;
    runtime.write_descriptor()?;
    let listener = UnixListener::bind(runtime.socket_path())?;
    let server = thread::spawn(
        move || -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            let (mut stream, _) = listener.accept()?;
            let _hello = read_client_value(&stream)?;
            write_server_message(&mut stream, &welcome_message())?;
            let command = read_client_value(&stream)?;
            let mut payload = String::new();
            for seq in 1..=8 {
                payload.push_str(
                    &json!({
                        "messageType": "lifecycle",
                        "seq": seq,
                        "sessionId": "session-buffered",
                        "phase": "active"
                    })
                    .to_string(),
                );
                payload.push('\n');
            }
            payload.push_str(
                &json!({
                    "messageType": "commandResponse",
                    "requestId": command["requestId"],
                    "payload": {
                        "accepted": true,
                        "visible_after_seq": 29,
                        "continuation_job_id": null
                    }
                })
                .to_string(),
            );
            payload.push('\n');
            stream.write_all(payload.as_bytes())?;
            stream.flush()?;
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
            decision_note: None,
        },
        "idem-buffered-response",
    )?;
    assert_eq!(ack.visible_after_seq, 29);
    server.join().expect("server should join")?;
    Ok(())
}

#[test]
fn bug_subscription_replay_regression_handles_one_shot_topic_replay() -> TestResult {
    let runtime = TempRuntime::new()?;
    runtime.write_descriptor()?;
    let listener = UnixListener::bind(runtime.socket_path())?;
    let server = thread::spawn(
        move || -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
            let (mut stream, _) = listener.accept()?;
            let _hello = read_client_value(&stream)?;
            write_server_message(&mut stream, &welcome_message())?;
            let subscribe = read_client_value(&stream)?;
            write_server_message(
                &mut stream,
                &json!({
                    "messageType": "commandResponse",
                    "requestId": subscribe["requestId"],
                    "payload": null
                }),
            )?;
            write_server_message(
                &mut stream,
                &json!({
                    "messageType": "topicEvent",
                    "topic": "sessions",
                    "payload": {
                        "sessionId": "session-1",
                        "frame": {
                            "kind": "lifecycle",
                            "seq": 12,
                            "state": "active",
                            "cleanupVerdict": null,
                            "archiveSeq": null,
                            "ts": null
                        }
                    }
                }),
            )?;
            Ok(subscribe)
        },
    );

    let mut connection = bridge(&runtime).connect()?;
    connection.subscribe("sessions", Some(7))?;
    match connection.read_subscription_event()? {
        Some(NativeSubscriptionEvent::Topic { topic, payload }) => {
            assert_eq!(topic, "sessions");
            assert_eq!(payload["sessionId"], "session-1");
            assert_eq!(payload["frame"]["seq"], 12);
        }
        event => {
            return Err(std::io::Error::other(format!(
                "expected replayed sessions topic event, got {event:?}"
            ))
            .into());
        }
    }
    let closed = connection
        .read_subscription_event()
        .expect_err("one-shot replay server should close the subscription socket");
    assert_eq!(closed.code, "transport_unavailable");

    let subscribe = server.join().expect("server should join")?;
    assert_eq!(subscribe["messageType"], "subscribe");
    assert_eq!(subscribe["topic"], "sessions");
    assert_eq!(subscribe["fromSeq"], 7);
    Ok(())
}

#[test]
fn rs_dbr_007_closed_method_set_stays_stable() {
    let methods = DaemonCommandMethod::ALL
        .iter()
        .map(|method| method.as_str())
        .collect::<Vec<_>>();
    // Stage 14 Phase 2 added 3 new operator-decision IPC methods
    // (capture_clarification, withdraw_clarification, withdraw_decision).
    assert_eq!(methods.len(), 14);
    assert_eq!(methods[0], "get_daemon_info");
    assert!(methods.contains(&"capture_decision"));
    assert!(methods.contains(&"capture_clarification"));
    assert!(methods.contains(&"withdraw_clarification"));
    assert!(methods.contains(&"withdraw_decision"));
    assert!(methods.contains(&"resolve_error"));
    assert!(methods.contains(&"rerun_from_error"));
}

#[test]
fn rs_dbr_008_projects_lifecycle_frames_with_archive_metadata() -> TestResult {
    let frame = DaemonBridge::session_frame_from_value(json!({
        "kind": "lifecycle",
        "seq": 11,
        "state": "archived",
        "cleanupVerdict": "clean",
        "archiveSeq": 42,
        "ts": "2026-03-28T10:00:00Z"
    }))?;

    assert_eq!(
        frame,
        SessionFrame::Lifecycle {
            seq: 11,
            state: "archived".to_owned(),
            cleanup_verdict: Some("clean".to_owned()),
            archive_seq: Some(42),
            ts: "2026-03-28T10:00:00Z".to_owned(),
        },
    );
    Ok(())
}

#[test]
fn rs_dbr_009_projects_auth_challenge_events_without_dropping_intervention() -> TestResult {
    let frame = DaemonBridge::session_frame_from_value(json!({
        "kind": "event",
        "seq": 7,
        "event": {
            "type": "authChallenge",
            "kind": "mfa",
            "detail": "Browser sign-in required.",
            "intervention": {
                "type": "reveal_profile_path",
                "profilePath": "/tmp/procintel/browser-profile",
                "reattachable": true
            }
        },
        "ts": "2026-03-28T10:00:07Z"
    }))?;

    match frame {
        SessionFrame::Event { event, .. } => {
            let serialized = serde_json::to_value(event)?;
            assert_eq!(serialized["type"], "authChallenge");
            assert_eq!(
                serialized["intervention"]["profilePath"],
                "/tmp/procintel/browser-profile",
            );
        }
        other => {
            return Err(format!("expected event frame, got {other:?}").into());
        }
    }
    Ok(())
}

#[test]
fn rs_dbr_010_projects_gap_frames_with_exact_ranges() -> TestResult {
    let frame = DaemonBridge::session_frame_from_value(json!({
        "kind": "gap",
        "fromSeq": 100,
        "toSeq": 150
    }))?;

    assert_eq!(
        frame,
        SessionFrame::Gap {
            from_seq: 100,
            to_seq: 150,
        },
    );
    Ok(())
}
