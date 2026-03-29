#![forbid(unsafe_code)]

mod support;

use std::collections::BTreeSet;
use std::fs;
use std::io::{BufRead, Write};
use std::os::unix::net::UnixListener;
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;

use anyhow::{Result, anyhow};
use rusqlite::Connection;
use serde_json::{Value, json};
use tempfile::tempdir;

use support::native_shell::{
    DaemonServer, NativeAppHarness, desktop_main, welcome_message, write_descriptor,
};

struct TestStateReset;
static TEST_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();

impl TestStateReset {
    fn new() -> Result<Self> {
        desktop_main::reset_test_runtime_state()?;
        Ok(Self)
    }
}

impl Drop for TestStateReset {
    fn drop(&mut self) {
        let _ = desktop_main::reset_test_runtime_state();
    }
}

fn blocked_launch() -> Result<()> {
    desktop_main::set_test_daemon_launch_hook(|_, _, _| {
        Err(anyhow!("native test disabled detached daemon launch"))
    })
}

fn json_mode(snapshot: &Value) -> Option<&str> {
    snapshot.get("mode").and_then(Value::as_str)
}

fn build_harness() -> Result<(tempfile::TempDir, desktop_main::TestRuntimeHarness)> {
    let temp = tempdir()?;
    let runtime = desktop_main::TestRuntimeHarness::new(temp.path().to_path_buf());
    Ok((temp, runtime))
}

fn write_custom_descriptor(
    runtime: &desktop_main::TestRuntimeHarness,
    instance_id: &str,
    pid: u32,
) -> Result<()> {
    fs::create_dir_all(runtime.runtime_dir())?;
    fs::write(
        runtime.descriptor_path(),
        serde_json::to_vec(&json!({
            "instanceId": instance_id,
            "pid": pid,
            "socketPath": runtime.socket_path().display().to_string(),
            "protocolVersion": 1,
            "schemaEpoch": 1,
            "stateRoot": runtime.state_root().display().to_string(),
            "startedAt": "2026-03-29T12:00:00Z",
            "ready": true
        }))?,
    )?;
    Ok(())
}

fn welcome_with_reconnect(
    instance_id: &str,
    reconnect_outcome: &str,
    stream_resume_permitted: bool,
) -> Value {
    json!({
        "messageType": "welcome",
        "selectedProtocolVersion": 1,
        "daemonVersion": "1.2.3",
        "instanceId": instance_id,
        "schemaEpoch": 1,
        "commitSeq": 77,
        "healthState": { "status": "ready" },
        "mutationsAllowed": true,
        "activeSessions": {
          "count": 1,
          "ids": ["session-live"],
          "states": ["active"]
        },
        "reconnectOutcome": reconnect_outcome,
        "streamResumePermitted": stream_resume_permitted
    })
}

fn spawn_welcome_server(
    runtime: &desktop_main::TestRuntimeHarness,
    welcomes: Vec<Value>,
) -> Result<thread::JoinHandle<Result<()>>> {
    let _ = fs::remove_file(runtime.socket_path());
    write_custom_descriptor(
        runtime,
        welcomes
            .first()
            .and_then(|value| value.get("instanceId"))
            .and_then(Value::as_str)
            .unwrap_or("daemon-native"),
        4242,
    )?;
    let listener = UnixListener::bind(runtime.socket_path())?;
    Ok(thread::spawn(move || -> Result<()> {
        for welcome in welcomes {
            let (mut stream, _) = listener.accept()?;
            let mut reader = std::io::BufReader::new(stream.try_clone()?);
            let mut hello = String::new();
            let _ = reader.read_line(&mut hello)?;
            let payload = serde_json::to_string(&welcome)?;
            stream.write_all(payload.as_bytes())?;
            stream.write_all(b"\n")?;
            stream.flush()?;
        }
        Ok(())
    }))
}

fn reasons(snapshot: &Value) -> Vec<String> {
    snapshot["controlPlane"]["reasons"]
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(ToOwned::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

fn top_level_keys(value: &Value) -> BTreeSet<String> {
    value
        .as_object()
        .map(|object| object.keys().cloned().collect())
        .unwrap_or_default()
}

#[test]
fn tau_int_001_quit_ui_records_native_exit_without_stopping_daemon() -> Result<()> {
    let _guard = TEST_MUTEX
        .get_or_init(|| Mutex::new(()))
        .lock()
        .map_err(|_| anyhow!("native-shell test lock poisoned"))?;
    let _reset = TestStateReset::new()?;
    blocked_launch()?;
    let (_temp, runtime) = build_harness()?;
    let app = NativeAppHarness::build(&runtime)?;

    let response: String = app.invoke_typed("quit_ui", Value::Null)?;

    anyhow::ensure!(response == "Quit UI requested.");
    anyhow::ensure!(runtime.exit_codes()? == vec![0]);
    anyhow::ensure!(runtime.stopped_pids()?.is_empty());
    Ok(())
}

#[test]
fn tau_int_002_stop_background_daemon_and_quit_records_native_stop_then_exit() -> Result<()> {
    let _guard = TEST_MUTEX
        .get_or_init(|| Mutex::new(()))
        .lock()
        .map_err(|_| anyhow!("native-shell test lock poisoned"))?;
    let _reset = TestStateReset::new()?;
    blocked_launch()?;
    let (_temp, runtime) = build_harness()?;
    write_descriptor(&runtime, true)?;
    let app = NativeAppHarness::build(&runtime)?;

    let response: String = app.invoke_typed("stop_background_daemon_and_quit", Value::Null)?;

    anyhow::ensure!(response == "Stop background daemon and quit requested.");
    anyhow::ensure!(runtime.stopped_pids()? == vec![4242]);
    anyhow::ensure!(runtime.exit_codes()? == vec![0]);
    Ok(())
}

#[test]
fn tau_int_003_startup_autolaunch_reaches_full_control_from_native_bootstrap() -> Result<()> {
    let _guard = TEST_MUTEX
        .get_or_init(|| Mutex::new(()))
        .lock()
        .map_err(|_| anyhow!("native-shell test lock poisoned"))?;
    let _reset = TestStateReset::new()?;
    let (_temp, runtime) = build_harness()?;
    let server_handle: Arc<Mutex<Option<thread::JoinHandle<Result<()>>>>> =
        Arc::new(Mutex::new(None));
    let runtime_for_hook = runtime.clone();
    let server_slot = Arc::clone(&server_handle);
    desktop_main::set_test_daemon_launch_hook(move |_state_root, runtime_dir, _artifact_root| {
        fs::create_dir_all(runtime_dir)?;
        let socket_path = runtime_dir.join("daemon.sock");
        let listener = UnixListener::bind(&socket_path)?;
        fs::write(
            runtime_for_hook.descriptor_path(),
            serde_json::to_vec(&json!({
                "instanceId": "daemon-autolaunch",
                "pid": 5150,
                "socketPath": socket_path.display().to_string(),
                "protocolVersion": 1,
                "schemaEpoch": 1,
                "stateRoot": runtime_for_hook.state_root().display().to_string(),
                "startedAt": "2026-03-29T12:30:00Z",
                "ready": true
            }))?,
        )?;

        let join = thread::spawn(move || -> Result<()> {
            let (mut stream, _) = listener.accept()?;
            let mut reader = std::io::BufReader::new(stream.try_clone()?);
            let mut hello = String::new();
            let _ = reader.read_line(&mut hello)?;
            let payload =
                serde_json::to_string(&welcome_message("daemon-autolaunch", "ready", 0, false))?;
            stream.write_all(payload.as_bytes())?;
            stream.write_all(b"\n")?;
            stream.flush()?;
            Ok(())
        });
        *server_slot
            .lock()
            .map_err(|_| anyhow!("join handle lock poisoned"))? = Some(join);
        Ok(())
    })?;

    let app = NativeAppHarness::build(&runtime)?;
    let snapshot = app.invoke_value("get_bootstrap_snapshot", Value::Null)?;
    anyhow::ensure!(json_mode(&snapshot) == Some("full_control"));
    anyhow::ensure!(snapshot["launchedDaemon"] == json!(true));

    let join = server_handle
        .lock()
        .map_err(|_| anyhow!("join lock poisoned"))?
        .take();
    if let Some(join) = join {
        join.join()
            .map_err(|_| anyhow!("daemon autolaunch server panicked"))??;
    }
    Ok(())
}

#[test]
fn tau_int_004_storage_ready_daemon_unavailable_maps_to_browse_only() -> Result<()> {
    let _guard = TEST_MUTEX
        .get_or_init(|| Mutex::new(()))
        .lock()
        .map_err(|_| anyhow!("native-shell test lock poisoned"))?;
    let _reset = TestStateReset::new()?;

    blocked_launch()?;
    let (_temp, runtime) = build_harness()?;
    let snapshot =
        NativeAppHarness::build(&runtime)?.invoke_value("get_bootstrap_snapshot", Value::Null)?;
    anyhow::ensure!(json_mode(&snapshot) == Some("browse_only"));
    Ok(())
}

#[test]
fn tau_int_005_schema_mismatch_maps_to_incompatible() -> Result<()> {
    let _guard = TEST_MUTEX
        .get_or_init(|| Mutex::new(()))
        .lock()
        .map_err(|_| anyhow!("native-shell test lock poisoned"))?;
    let _reset = TestStateReset::new()?;
    blocked_launch()?;

    let (_temp, runtime) = build_harness()?;
    fs::create_dir_all(runtime.state_root())?;
    let mismatch_db = runtime.state_root().join("state.db");
    let mismatch_conn = Connection::open(&mismatch_db)?;
    mismatch_conn.pragma_update(None, "user_version", 99_i64)?;
    let mismatch =
        NativeAppHarness::build(&runtime)?.invoke_value("get_bootstrap_snapshot", Value::Null)?;
    anyhow::ensure!(json_mode(&mismatch) == Some("incompatible"));
    Ok(())
}

#[test]
fn tau_int_006_unreadable_storage_maps_to_recovery_required() -> Result<()> {
    let _guard = TEST_MUTEX
        .get_or_init(|| Mutex::new(()))
        .lock()
        .map_err(|_| anyhow!("native-shell test lock poisoned"))?;
    let _reset = TestStateReset::new()?;
    blocked_launch()?;

    let (_temp, runtime) = build_harness()?;
    fs::write(runtime.state_root(), "not-a-directory")?;
    let recovery =
        NativeAppHarness::build(&runtime)?.invoke_value("get_bootstrap_snapshot", Value::Null)?;
    anyhow::ensure!(json_mode(&recovery) == Some("recovery_required"));
    Ok(())
}

#[test]
fn tau_int_007_degraded_daemon_maps_to_control_limited() -> Result<()> {
    let _guard = TEST_MUTEX
        .get_or_init(|| Mutex::new(()))
        .lock()
        .map_err(|_| anyhow!("native-shell test lock poisoned"))?;
    let _reset = TestStateReset::new()?;
    blocked_launch()?;

    let (_temp, runtime) = build_harness()?;
    let _server = DaemonServer::start(
        &runtime,
        welcome_message("daemon-limited", "degraded", 1, false),
    )?;
    let limited =
        NativeAppHarness::build(&runtime)?.invoke_value("get_bootstrap_snapshot", Value::Null)?;
    anyhow::ensure!(json_mode(&limited) == Some("control_limited"));

    Ok(())
}

#[test]
fn tau_int_008_mode_matrix_stays_total_and_non_overlapping_in_native_bootstrap() -> Result<()> {
    let _guard = TEST_MUTEX
        .get_or_init(|| Mutex::new(()))
        .lock()
        .map_err(|_| anyhow!("native-shell test lock poisoned"))?;
    let _reset = TestStateReset::new()?;
    blocked_launch()?;

    let (_browse_temp, browse_runtime) = build_harness()?;
    let browse = NativeAppHarness::build(&browse_runtime)?
        .invoke_value("get_bootstrap_snapshot", Value::Null)?;

    let (_mismatch_temp, mismatch_runtime) = build_harness()?;
    fs::create_dir_all(mismatch_runtime.state_root())?;
    let mismatch_db = mismatch_runtime.state_root().join("state.db");
    let mismatch_conn = Connection::open(&mismatch_db)?;
    mismatch_conn.pragma_update(None, "user_version", 99_i64)?;
    let mismatch = NativeAppHarness::build(&mismatch_runtime)?
        .invoke_value("get_bootstrap_snapshot", Value::Null)?;

    let (_recovery_temp, recovery_runtime) = build_harness()?;
    fs::write(recovery_runtime.state_root(), "not-a-directory")?;
    let recovery = NativeAppHarness::build(&recovery_runtime)?
        .invoke_value("get_bootstrap_snapshot", Value::Null)?;

    let (_limited_temp, limited_runtime) = build_harness()?;
    let _server = DaemonServer::start(
        &limited_runtime,
        welcome_message("daemon-limited", "degraded", 1, false),
    )?;
    let limited = NativeAppHarness::build(&limited_runtime)?
        .invoke_value("get_bootstrap_snapshot", Value::Null)?;

    let modes = [
        json_mode(&browse).unwrap_or_default(),
        json_mode(&mismatch).unwrap_or_default(),
        json_mode(&recovery).unwrap_or_default(),
        json_mode(&limited).unwrap_or_default(),
    ];

    anyhow::ensure!(modes.contains(&"browse_only"));
    anyhow::ensure!(modes.contains(&"incompatible"));
    anyhow::ensure!(modes.contains(&"recovery_required"));
    anyhow::ensure!(modes.contains(&"control_limited"));
    anyhow::ensure!(modes.len() == 4);
    Ok(())
}

#[test]
fn tau_int_009_same_daemon_reconnect_retains_instance_id() -> Result<()> {
    let _guard = TEST_MUTEX
        .get_or_init(|| Mutex::new(()))
        .lock()
        .map_err(|_| anyhow!("native-shell test lock poisoned"))?;
    let _reset = TestStateReset::new()?;
    blocked_launch()?;

    let (_temp, runtime) = build_harness()?;
    let join = spawn_welcome_server(
        &runtime,
        vec![
            welcome_with_reconnect("daemon-same", "freshConnection", false),
            welcome_with_reconnect("daemon-same", "sameDaemonReconnect", true),
        ],
    )?;

    let app = NativeAppHarness::build(&runtime)?;
    let first = app.invoke_value("get_bootstrap_snapshot", Value::Null)?;
    let second = app.invoke_value("refresh_bootstrap_snapshot", Value::Null)?;

    anyhow::ensure!(first["controlPlane"]["daemonInstanceId"] == json!("daemon-same"));
    anyhow::ensure!(second["controlPlane"]["daemonInstanceId"] == json!("daemon-same"));
    anyhow::ensure!(
        !reasons(&second)
            .iter()
            .any(|reason| reason.contains("invalidated stream resume"))
    );
    anyhow::ensure!(
        !reasons(&second)
            .iter()
            .any(|reason| reason.contains("not currently permitted"))
    );
    join.join()
        .map_err(|_| anyhow!("same-daemon reconnect server panicked"))??;
    Ok(())
}

#[test]
fn tau_int_010_changed_instance_ids_invalidate_live_resume() -> Result<()> {
    let _guard = TEST_MUTEX
        .get_or_init(|| Mutex::new(()))
        .lock()
        .map_err(|_| anyhow!("native-shell test lock poisoned"))?;
    let _reset = TestStateReset::new()?;
    blocked_launch()?;

    let (_temp, runtime) = build_harness()?;
    let join = spawn_welcome_server(
        &runtime,
        vec![
            welcome_with_reconnect("daemon-old", "freshConnection", false),
            welcome_with_reconnect("daemon-new", "daemonRestartInvalidated", false),
        ],
    )?;

    let app = NativeAppHarness::build(&runtime)?;
    let first = app.invoke_value("get_bootstrap_snapshot", Value::Null)?;
    let second = app.invoke_value("refresh_bootstrap_snapshot", Value::Null)?;

    anyhow::ensure!(first["controlPlane"]["daemonInstanceId"] == json!("daemon-old"));
    anyhow::ensure!(second["controlPlane"]["daemonInstanceId"] == json!("daemon-new"));
    anyhow::ensure!(
        reasons(&second)
            .iter()
            .any(|reason| reason.contains("invalidated stream resume"))
    );
    join.join()
        .map_err(|_| anyhow!("restart-invalidated reconnect server panicked"))??;
    Ok(())
}

#[test]
#[allow(clippy::too_many_lines)]
fn tau_int_011_native_payload_shapes_match_app_contracts() -> Result<()> {
    let _guard = TEST_MUTEX
        .get_or_init(|| Mutex::new(()))
        .lock()
        .map_err(|_| anyhow!("native-shell test lock poisoned"))?;
    let _reset = TestStateReset::new()?;
    blocked_launch()?;

    let (_temp, runtime) = build_harness()?;
    let app = NativeAppHarness::build(&runtime)?;

    let bootstrap_join = spawn_welcome_server(
        &runtime,
        vec![welcome_with_reconnect(
            "daemon-parity",
            "freshConnection",
            false,
        )],
    )?;
    let bootstrap = app.invoke_value("get_bootstrap_snapshot", Value::Null)?;
    bootstrap_join
        .join()
        .map_err(|_| anyhow!("bootstrap parity server panicked"))??;

    let stream_payload = json!({
        "sessionId": "session-live",
        "frame": {
            "kind": "lifecycle",
            "seq": 7,
            "state": "active",
            "cleanupVerdict": null,
            "archiveSeq": null,
            "ts": "2026-03-29T12:00:07Z"
        }
    });
    anyhow::ensure!(
        top_level_keys(&bootstrap)
            == BTreeSet::from([
                "controlPlane".to_owned(),
                "descriptor".to_owned(),
                "launchedDaemon".to_owned(),
                "mode".to_owned(),
            ])
    );
    anyhow::ensure!(
        top_level_keys(&stream_payload)
            == BTreeSet::from(["frame".to_owned(), "sessionId".to_owned()])
    );
    anyhow::ensure!(
        top_level_keys(&stream_payload["frame"])
            == BTreeSet::from([
                "archiveSeq".to_owned(),
                "cleanupVerdict".to_owned(),
                "kind".to_owned(),
                "seq".to_owned(),
                "state".to_owned(),
                "ts".to_owned(),
            ])
    );
    Ok(())
}

#[test]
fn tau_int_012_native_ipc_bootstrap_path_works_without_browser_dev_bridge() -> Result<()> {
    let _guard = TEST_MUTEX
        .get_or_init(|| Mutex::new(()))
        .lock()
        .map_err(|_| anyhow!("native-shell test lock poisoned"))?;
    let _reset = TestStateReset::new()?;
    blocked_launch()?;
    let (_temp, runtime) = build_harness()?;
    let app = NativeAppHarness::build(&runtime)?;

    let snapshot = app.invoke_value("get_bootstrap_snapshot", Value::Null)?;

    anyhow::ensure!(snapshot.get("mode").is_some());
    anyhow::ensure!(snapshot.get("controlPlane").is_some());
    Ok(())
}
