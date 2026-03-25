#![forbid(unsafe_code)]

mod support;

use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::thread;

use anyhow::{Context, Result, anyhow};
use nix::unistd::Uid;
use serde_yaml::Value;

#[path = "../src/ipc/descriptor.rs"]
mod descriptor_impl;
#[path = "../src/ipc/handshake.rs"]
mod handshake_impl;

use crate::descriptor_impl::{
    DaemonDescriptor, LockGuard, RuntimePaths, bind_listener, write_descriptor_atomic,
};
use crate::handshake_impl::{
    ActiveSessions, ClientMessage, HandshakeContext, HealthState, ProtocolRange, ServerMessage,
};
use crate::support::db::{DEFAULT_SCHEMA_EPOCH, FTS_SOURCE_ID, TestStateDb};

#[test]
fn ci_core_004_runtime_inventory_matches_manifest() -> Result<()> {
    let manifest_path = repo_root()?.join("docs/pipeline/l1-daemon-core/manifest.yaml");
    let manifest_text = fs::read_to_string(&manifest_path)
        .with_context(|| format!("reading {}", manifest_path.display()))?;
    let manifest: Value = serde_yaml::from_str(&manifest_text)
        .with_context(|| format!("parsing {}", manifest_path.display()))?;

    let runtime_entries = manifest
        .get("key_files_to_create")
        .and_then(Value::as_sequence)
        .ok_or_else(|| anyhow!("manifest.yaml missing key_files_to_create"))?;

    let runtime_paths: Vec<&str> = runtime_entries
        .iter()
        .filter_map(|entry| entry.get("path").and_then(Value::as_str))
        .collect();

    let daemon_source_paths = runtime_paths
        .iter()
        .filter(|path| path.starts_with("crates/daemon/src/"))
        .count();
    let skill_paths = runtime_paths
        .iter()
        .filter(|path| path.starts_with("skills/"))
        .count();

    assert_eq!(daemon_source_paths, 17);
    assert_eq!(skill_paths, 2);
    assert!(runtime_paths.contains(&"migrations/0003_sqlite_runtime.sql"));

    Ok(())
}

#[test]
fn it_ipc_013_state_db_bootstrap_uses_one_database() -> Result<()> {
    let ctx = support::TestContext::new()?;
    assert!(ctx.db_path.ends_with("state/state.db"));
    assert!(ctx.state_root.ends_with("state"));
    assert!(ctx.runtime_root.ends_with("runtime"));
    assert!(ctx.artifact_root.ends_with("artifacts"));
    assert!(ctx.temp.path().exists());

    let state_db = TestStateDb::new()?;
    let conn = state_db.connection();

    let journal_mode: String = conn.query_row("PRAGMA journal_mode", [], |row| row.get(0))?;
    assert_eq!(journal_mode.to_lowercase(), "wal");

    let quick_check: String = conn.query_row("PRAGMA quick_check", [], |row| row.get(0))?;
    assert_eq!(quick_check, "ok");

    let user_version: i64 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;
    assert_eq!(user_version, DEFAULT_SCHEMA_EPOCH);

    let commit_seq: i64 = conn.query_row(
        "SELECT commit_seq FROM commit_clock WHERE id = 1",
        [],
        |row| row.get(0),
    )?;
    assert_eq!(commit_seq, 0);

    let (kind, enabled, config): (String, i64, String) = conn.query_row(
        "SELECT kind, enabled, config FROM sources WHERE id = ?1",
        [FTS_SOURCE_ID],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    )?;
    assert_eq!(kind, "procurement");
    assert_eq!(enabled, 1);
    assert!(config.contains("find-tender.service.gov.uk"));

    let expected_tables = [
        "sources",
        "workflow_jobs",
        "process_stage_status",
        "process_stage_heads",
        "agent_session_logs",
        "raw_documents",
        "error_manifests",
        "executors",
        "source_cursors",
        "commit_clock",
    ];

    for table_name in expected_tables {
        let exists: i64 = conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
            [table_name],
            |row| row.get(0),
        )?;
        assert_eq!(exists, 1, "expected table {table_name} to exist");
    }

    let sibling_entries: Vec<String> =
        fs::read_dir(state_db.db_path().parent().ok_or_else(|| {
            anyhow!(
                "state db path has no parent: {}",
                state_db.db_path().display()
            )
        })?)?
        .map(|entry| entry.map(|dir_entry| dir_entry.file_name().to_string_lossy().into_owned()))
        .collect::<std::result::Result<_, _>>()?;

    assert!(!sibling_entries.iter().any(|name| name == "artifacts.db"));

    Ok(())
}

#[test]
fn it_ipc_001_descriptor_is_written_only_after_runtime_is_ready() -> Result<()> {
    let context = support::TestContext::new()?;
    let runtime_paths = RuntimePaths::new(context.state_root.clone(), context.runtime_root);

    let _lock = LockGuard::acquire(&runtime_paths)?;
    assert!(!runtime_paths.descriptor_path().exists());

    let state_db = TestStateDb::new()?;
    assert_eq!(
        state_db
            .connection()
            .query_row("SELECT 1", [], |row| row.get::<_, i64>(0))?,
        1
    );
    assert!(!runtime_paths.descriptor_path().exists());

    let _listener = bind_listener(&runtime_paths)?;
    assert!(!runtime_paths.descriptor_path().exists());

    let descriptor = DaemonDescriptor::new(
        runtime_paths.socket_path(),
        runtime_paths.state_root(),
        DEFAULT_SCHEMA_EPOCH,
    );
    write_descriptor_atomic(runtime_paths.descriptor_path(), &descriptor)?;

    let written = fs::read_to_string(runtime_paths.descriptor_path())?;
    let parsed: DaemonDescriptor = serde_json::from_str(&written)?;
    assert!(parsed.ready);
    assert_eq!(parsed.schema_epoch, DEFAULT_SCHEMA_EPOCH);
    assert_eq!(
        parsed.socket_path,
        runtime_paths.socket_path().display().to_string()
    );

    Ok(())
}

#[test]
fn it_ipc_002_lock_file_blocks_second_writer() -> Result<()> {
    let context = support::TestContext::new()?;
    let runtime_paths = RuntimePaths::new(context.state_root.clone(), context.runtime_root);

    let _first = LockGuard::acquire(&runtime_paths)?;
    let second = LockGuard::acquire(&runtime_paths);
    assert!(second.is_err());

    Ok(())
}

#[test]
fn it_ipc_003_successful_handshake_returns_runtime_compatibility() -> Result<()> {
    let setup = HandshakeHarness::new()?;
    let response = setup.request(&ClientMessage::Hello {
        desktop_version: "desktop-1.0.0".to_owned(),
        supported_protocol_range: ProtocolRange { min: 1, max: 1 },
        expected_schema_epoch: DEFAULT_SCHEMA_EPOCH,
        last_seen_instance_id: None,
    })?;

    match response {
        ServerMessage::Welcome {
            selected_protocol_version,
            daemon_version,
            schema_epoch,
            commit_seq,
            mutations_allowed,
            active_sessions,
            ..
        } => {
            assert_eq!(selected_protocol_version, 1);
            assert_eq!(daemon_version, env!("CARGO_PKG_VERSION"));
            assert_eq!(schema_epoch, DEFAULT_SCHEMA_EPOCH);
            assert_eq!(commit_seq, 0);
            assert!(!mutations_allowed);
            assert_eq!(active_sessions.count, 0);
        }
        other @ ServerMessage::Rejection { .. } => panic!("unexpected response: {other:?}"),
    }

    Ok(())
}

#[test]
fn it_ipc_004_protocol_range_mismatch_is_rejected() -> Result<()> {
    let setup = HandshakeHarness::new()?;
    let response = setup.request(&ClientMessage::Hello {
        desktop_version: "desktop-1.0.0".to_owned(),
        supported_protocol_range: ProtocolRange { min: 2, max: 3 },
        expected_schema_epoch: DEFAULT_SCHEMA_EPOCH,
        last_seen_instance_id: None,
    })?;

    match response {
        ServerMessage::Rejection {
            code,
            supported_protocol_range,
            ..
        } => {
            assert_eq!(code, "protocol_range_mismatch");
            assert_eq!(
                supported_protocol_range,
                Some(ProtocolRange { min: 1, max: 1 })
            );
        }
        other @ ServerMessage::Welcome { .. } => panic!("unexpected response: {other:?}"),
    }

    Ok(())
}

#[test]
fn it_ipc_005_schema_epoch_mismatch_reports_expected_and_actual() -> Result<()> {
    let setup = HandshakeHarness::new()?;
    let response = setup.request(&ClientMessage::Hello {
        desktop_version: "desktop-1.0.0".to_owned(),
        supported_protocol_range: ProtocolRange { min: 1, max: 1 },
        expected_schema_epoch: DEFAULT_SCHEMA_EPOCH + 1,
        last_seen_instance_id: None,
    })?;

    match response {
        ServerMessage::Rejection {
            code,
            expected_schema_epoch,
            actual_schema_epoch,
            ..
        } => {
            assert_eq!(code, "schema_epoch_mismatch");
            assert_eq!(expected_schema_epoch, Some(DEFAULT_SCHEMA_EPOCH + 1));
            assert_eq!(actual_schema_epoch, Some(DEFAULT_SCHEMA_EPOCH));
        }
        other @ ServerMessage::Welcome { .. } => panic!("unexpected response: {other:?}"),
    }

    Ok(())
}

#[test]
fn it_ipc_008_socket_accepts_handshake_only() -> Result<()> {
    let setup = HandshakeHarness::new()?;
    let response = setup.request(&ClientMessage::Mutation {
        command: "startJob".to_owned(),
    })?;

    match response {
        ServerMessage::Rejection { code, .. } => {
            assert_eq!(code, "mutations_unavailable");
        }
        other @ ServerMessage::Welcome { .. } => panic!("unexpected response: {other:?}"),
    }

    Ok(())
}

#[test]
fn it_ipc_009_peer_uid_mismatch_is_rejected() -> Result<()> {
    let mut setup = HandshakeHarness::new()?;
    setup.context.expected_peer_uid = Some(Uid::current().as_raw().saturating_add(1));
    let response = setup.request(&ClientMessage::Hello {
        desktop_version: "desktop-1.0.0".to_owned(),
        supported_protocol_range: ProtocolRange { min: 1, max: 1 },
        expected_schema_epoch: DEFAULT_SCHEMA_EPOCH,
        last_seen_instance_id: None,
    })?;

    match response {
        ServerMessage::Rejection { code, .. } => {
            assert_eq!(code, "peer_uid_mismatch");
        }
        other @ ServerMessage::Welcome { .. } => panic!("unexpected response: {other:?}"),
    }

    Ok(())
}

#[test]
fn it_ipc_010_startup_failure_never_writes_ready_descriptor() -> Result<()> {
    let context = support::TestContext::new()?;
    let runtime_paths = RuntimePaths::new(context.state_root.clone(), context.runtime_root);
    let _lock = LockGuard::acquire(&runtime_paths)?;
    let state_db = TestStateDb::new()?;

    state_db
        .connection()
        .pragma_update(None, "user_version", 99_i64)?;
    let reopen = crate::support::db::StateDb::open(state_db.db_path());

    assert!(reopen.is_err());
    assert!(!runtime_paths.descriptor_path().exists());

    Ok(())
}

fn repo_root() -> Result<PathBuf> {
    let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    crate_dir
        .ancestors()
        .nth(2)
        .map(Path::to_path_buf)
        .ok_or_else(|| {
            anyhow!(
                "failed to resolve repository root from {}",
                crate_dir.display()
            )
        })
}

struct HandshakeHarness {
    _context: support::TestContext,
    context: HandshakeContext,
    listener: UnixListener,
    socket_path: PathBuf,
}

impl HandshakeHarness {
    fn new() -> Result<Self> {
        let ctx = support::TestContext::new()?;
        let runtime_paths = RuntimePaths::new(ctx.state_root.clone(), ctx.runtime_root.clone());
        fs::create_dir_all(runtime_paths.runtime_dir())?;
        fs::set_permissions(
            runtime_paths.runtime_dir(),
            fs::Permissions::from_mode(0o700),
        )?;
        let listener = bind_listener(&runtime_paths)?;
        let context = HandshakeContext {
            protocol_version: 1,
            daemon_version: env!("CARGO_PKG_VERSION").to_owned(),
            instance_id: "instance-1".to_owned(),
            schema_epoch: DEFAULT_SCHEMA_EPOCH,
            commit_seq: 0,
            health_state: HealthState {
                status: "ready".to_owned(),
            },
            active_sessions: ActiveSessions::default(),
            mutations_allowed: false,
            expected_peer_uid: None,
        };

        Ok(Self {
            _context: ctx,
            context,
            listener,
            socket_path: runtime_paths.socket_path().to_path_buf(),
        })
    }

    fn request(&self, message: &ClientMessage) -> Result<ServerMessage> {
        let listener = self.listener.try_clone()?;
        let context = self.context.clone();
        let handle = thread::spawn(move || handshake_impl::serve_one(&listener, &context));

        let mut client = UnixStream::connect(&self.socket_path)
            .with_context(|| format!("connecting to {}", self.socket_path.display()))?;
        let payload = serde_json::to_string(message)?;
        client.write_all(payload.as_bytes())?;
        client.write_all(b"\n")?;
        client.flush()?;

        let mut reader = BufReader::new(client);
        let mut response = String::new();
        reader.read_line(&mut response)?;

        handle
            .join()
            .map_err(|_| anyhow!("handshake thread panicked"))??;
        serde_json::from_str(response.trim_end()).context("parsing handshake response")
    }
}
