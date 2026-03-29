#![forbid(unsafe_code)]

use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::thread::{self, JoinHandle};

use anyhow::{Context, Result, anyhow};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use tauri::ipc::InvokeBody;
use tauri::test::{MockRuntime, get_ipc_response, mock_builder, mock_context, noop_assets};
use tauri::webview::InvokeRequest;
use tauri::{WebviewUrl, WebviewWindow, WebviewWindowBuilder};

#[path = "../../src/main.rs"]
#[allow(dead_code)]
pub mod desktop_main;

pub struct NativeAppHarness {
    #[allow(dead_code)]
    pub app: tauri::App<MockRuntime>,
    pub webview: WebviewWindow<MockRuntime>,
}

impl NativeAppHarness {
    pub fn build(runtime: &desktop_main::TestRuntimeHarness) -> Result<Self> {
        let builder = desktop_main::configure_builder(mock_builder().manage(runtime.clone()));
        let app = builder
            .build(mock_context(noop_assets()))
            .context("building mock tauri app")?;
        let webview = WebviewWindowBuilder::new(&app, "main", WebviewUrl::default())
            .build()
            .context("building mock tauri webview")?;
        Ok(Self { app, webview })
    }

    pub fn invoke_value(&self, cmd: &str, payload: Value) -> Result<Value> {
        let request = InvokeRequest {
            cmd: cmd.to_owned(),
            callback: tauri::ipc::CallbackFn(0),
            error: tauri::ipc::CallbackFn(1),
            url: "http://tauri.localhost"
                .parse()
                .context("parsing tauri localhost url")?,
            body: if payload.is_null() {
                InvokeBody::default()
            } else {
                InvokeBody::Json(payload)
            },
            headers: tauri::http::HeaderMap::default(),
            invoke_key: tauri::test::INVOKE_KEY.to_owned(),
        };
        get_ipc_response(&self.webview, request)
            .map_err(|error| anyhow!("tauri invoke `{cmd}` failed: {error}"))?
            .deserialize::<Value>()
            .context("deserializing tauri invoke payload")
    }

    pub fn invoke_typed<T: DeserializeOwned>(&self, cmd: &str, payload: Value) -> Result<T> {
        serde_json::from_value(self.invoke_value(cmd, payload)?)
            .with_context(|| format!("deserializing typed response for `{cmd}`"))
    }
}

pub struct DaemonServer {
    join: Option<JoinHandle<Result<()>>>,
}

impl DaemonServer {
    pub fn start(runtime: &desktop_main::TestRuntimeHarness, welcome: Value) -> Result<Self> {
        fs::create_dir_all(runtime.runtime_dir())?;
        let listener = UnixListener::bind(runtime.socket_path())
            .with_context(|| format!("binding {}", runtime.socket_path().display()))?;
        write_descriptor(runtime, true)?;

        let join = thread::spawn(move || -> Result<()> {
            let (mut stream, _) = listener
                .accept()
                .context("accepting daemon bridge client")?;
            let _hello = read_client_value(&stream)?;
            write_server_message(&mut stream, &welcome)?;
            Ok(())
        });

        Ok(Self { join: Some(join) })
    }
}

impl Drop for DaemonServer {
    fn drop(&mut self) {
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}
pub fn write_descriptor(runtime: &desktop_main::TestRuntimeHarness, ready: bool) -> Result<()> {
    fs::create_dir_all(runtime.runtime_dir())?;
    fs::write(
        runtime.descriptor_path(),
        serde_json::to_vec(&json!({
            "instanceId": "daemon-native",
            "pid": 4242,
            "socketPath": runtime.socket_path().display().to_string(),
            "protocolVersion": 1,
            "schemaEpoch": 1,
            "stateRoot": runtime.state_root().display().to_string(),
            "startedAt": "2026-03-29T12:00:00Z",
            "ready": ready
        }))?,
    )?;
    Ok(())
}

pub fn welcome_message(
    instance_id: &str,
    health_status: &str,
    active_sessions: usize,
    stream_resume_permitted: bool,
) -> Value {
    json!({
        "messageType": "welcome",
        "selectedProtocolVersion": 1,
        "daemonVersion": "1.2.3",
        "instanceId": instance_id,
        "schemaEpoch": 1,
        "commitSeq": 77,
        "healthState": { "status": health_status },
        "mutationsAllowed": true,
        "activeSessions": {
            "count": active_sessions,
            "ids": (0..active_sessions).map(|index| format!("session-{index}")).collect::<Vec<_>>(),
            "states": (0..active_sessions).map(|_| "active").collect::<Vec<_>>()
        },
        "reconnectOutcome": "freshConnection",
        "streamResumePermitted": stream_resume_permitted
    })
}

fn write_server_message(stream: &mut UnixStream, message: &Value) -> Result<()> {
    let payload = serde_json::to_string(message)?;
    stream.write_all(payload.as_bytes())?;
    stream.write_all(b"\n")?;
    stream.flush()?;
    Ok(())
}

fn read_client_value(stream: &UnixStream) -> Result<Value> {
    let mut reader = BufReader::new(stream);
    let mut buffer = String::new();
    let _ = reader.read_line(&mut buffer)?;
    serde_json::from_str(buffer.trim_end()).context("parsing client frame")
}
