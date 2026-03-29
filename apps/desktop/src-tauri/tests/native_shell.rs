#![forbid(unsafe_code)]

mod support;

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
fn tau_int_004_005_006_007_cover_native_bootstrap_modes() -> Result<()> {
    let _guard = TEST_MUTEX
        .get_or_init(|| Mutex::new(()))
        .lock()
        .map_err(|_| anyhow!("native-shell test lock poisoned"))?;
    let _reset = TestStateReset::new()?;

    blocked_launch()?;
    let (_browse_temp, browse_runtime) = build_harness()?;
    let browse = NativeAppHarness::build(&browse_runtime)?
        .invoke_value("get_bootstrap_snapshot", Value::Null)?;
    anyhow::ensure!(json_mode(&browse) == Some("browse_only"));

    let (_mismatch_temp, mismatch_runtime) = build_harness()?;
    fs::create_dir_all(mismatch_runtime.state_root())?;
    let mismatch_db = mismatch_runtime.state_root().join("state.db");
    let mismatch_conn = Connection::open(&mismatch_db)?;
    mismatch_conn.pragma_update(None, "user_version", 99_i64)?;
    let mismatch = NativeAppHarness::build(&mismatch_runtime)?
        .invoke_value("get_bootstrap_snapshot", Value::Null)?;
    anyhow::ensure!(json_mode(&mismatch) == Some("incompatible"));

    let (_recovery_temp, recovery_runtime) = build_harness()?;
    fs::write(recovery_runtime.state_root(), "not-a-directory")?;
    let recovery = NativeAppHarness::build(&recovery_runtime)?
        .invoke_value("get_bootstrap_snapshot", Value::Null)?;
    anyhow::ensure!(json_mode(&recovery) == Some("recovery_required"));

    let (_limited_temp, limited_runtime) = build_harness()?;
    let _server = DaemonServer::start(
        &limited_runtime,
        welcome_message("daemon-limited", "degraded", 1, false),
    )?;
    let limited = NativeAppHarness::build(&limited_runtime)?
        .invoke_value("get_bootstrap_snapshot", Value::Null)?;
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
