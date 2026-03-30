#![forbid(unsafe_code)]

mod support;

use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::net::Shutdown;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::thread;

use anyhow::{Context, Result, anyhow, bail};
use nix::unistd::Uid;
use serde_json::json;
use serde_yaml::Value;

#[path = "../src/ipc/command_router.rs"]
#[allow(dead_code)]
mod command_router_impl;
#[path = "../src/agents/contracts.rs"]
#[allow(dead_code)]
mod contracts_impl;
#[path = "../src/ipc/handlers/control.rs"]
#[allow(dead_code)]
mod control_handler_impl;
#[path = "../src/ipc/handlers/decisions.rs"]
#[allow(dead_code)]
mod decisions_handler_impl;
#[path = "../src/ipc/descriptor.rs"]
mod descriptor_impl;
#[path = "../src/ipc/handlers/errors.rs"]
#[allow(dead_code)]
mod errors_handler_impl;
#[path = "../src/errors/manifest.rs"]
#[allow(dead_code)]
mod errors_manifest_impl;
#[path = "../src/errors/markdown.rs"]
#[allow(dead_code)]
mod errors_markdown_impl;
#[path = "../src/ipc/handshake.rs"]
mod handshake_impl;
#[path = "../src/ipc/handlers/jobs.rs"]
#[allow(dead_code)]
mod jobs_handler_impl;
#[path = "../src/pty/session.rs"]
#[allow(dead_code)]
mod pty_session_impl;
#[path = "../src/jobs/runner.rs"]
#[allow(dead_code)]
mod runner_impl;
#[path = "../src/ipc/handlers/sessions.rs"]
#[allow(dead_code)]
mod session_handler_impl;

mod db {
    pub mod sqlite {
        pub use crate::support::db::StateDb;
    }
}

mod jobs {
    pub mod runner {
        pub use crate::runner_impl::*;
    }
}

mod pty {
    pub mod session {
        pub use crate::pty_session_impl::*;
    }
}

mod ipc {
    pub mod command_router {
        pub use crate::command_router_impl::*;
    }

    pub mod descriptor {
        pub use crate::descriptor_impl::*;
    }

    pub mod handlers {
        pub mod control {
            pub use crate::control_handler_impl::*;
        }
        pub mod decisions {
            pub use crate::decisions_handler_impl::*;
        }
        pub mod errors {
            pub use crate::errors_handler_impl::*;
        }
        pub mod jobs {
            pub use crate::jobs_handler_impl::*;
        }
        pub mod sessions {
            pub use crate::session_handler_impl::*;
        }
    }
}

use crate::descriptor_impl::{
    DaemonDescriptor, LockGuard, RuntimePaths, bind_listener, read_descriptor,
    write_descriptor_atomic,
};
use crate::handshake_impl::{
    ActiveSessions, ClientMessage, HandshakeContext, HealthState, LifecycleFrame, LifecyclePhase,
    ProtocolRange, ReconnectOutcome, ServerMessage,
};
use crate::support::db::{DEFAULT_SCHEMA_EPOCH, FTS_SOURCE_ID, StateDb, TestStateDb};

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
        "source_runtime_state",
        "workflow_jobs",
        "validation_verdicts",
        "listing_evidence",
        "process_relations",
        "process_aliases",
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

    let mut source_runtime_state_columns =
        conn.prepare("PRAGMA table_info(source_runtime_state)")?;
    let source_runtime_state_columns = source_runtime_state_columns
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    assert!(
        source_runtime_state_columns
            .iter()
            .any(|column| column == "circuit_state")
    );
    assert!(
        source_runtime_state_columns
            .iter()
            .any(|column| column == "last_canary_result")
    );

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
            reconnect_outcome,
            stream_resume_permitted,
            ..
        } => {
            assert_eq!(selected_protocol_version, 1);
            assert_eq!(daemon_version, env!("CARGO_PKG_VERSION"));
            assert_eq!(schema_epoch, DEFAULT_SCHEMA_EPOCH);
            assert_eq!(commit_seq, 0);
            assert!(!mutations_allowed);
            assert_eq!(active_sessions.count, 0);
            assert_eq!(reconnect_outcome, ReconnectOutcome::FreshConnection);
            assert!(!stream_resume_permitted);
        }
        other => bail!("unexpected response: {other:?}"),
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
        other => bail!("unexpected response: {other:?}"),
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
        other => bail!("unexpected response: {other:?}"),
    }

    Ok(())
}

#[test]
fn it_ipc_008_socket_accepts_handshake_only() -> Result<()> {
    let setup = HandshakeHarness::new()?;
    let responses = [
        setup.request(&ClientMessage::Mutation {
            command: "startJob".to_owned(),
        })?,
        setup.request(&ClientMessage::Command {
            request_id: "req-1".to_owned(),
            method: "get_daemon_info".to_owned(),
            params: json!({}),
            idempotency_key: None,
        })?,
        setup.request(&ClientMessage::Subscribe {
            request_id: "req-2".to_owned(),
            topic: "daemon.health".to_owned(),
            from_seq: Some(42),
        })?,
        setup.request(&ClientMessage::Unsubscribe {
            request_id: "req-3".to_owned(),
            topic: "daemon.health".to_owned(),
        })?,
    ];

    for response in responses {
        match response {
            ServerMessage::Rejection { code, .. } => {
                assert_eq!(code, "mutations_unavailable");
            }
            other => bail!("unexpected response: {other:?}"),
        }
    }

    Ok(())
}

#[test]
fn it_ipc_014_router_active_accepts_subscribe_and_emits_initial_health_payload() -> Result<()> {
    let setup = HandshakeHarness::new()?;
    let frames = setup.request_frames_with_router(&[
        ClientMessage::Hello {
            desktop_version: "desktop-1.0.0".to_owned(),
            supported_protocol_range: ProtocolRange { min: 1, max: 1 },
            expected_schema_epoch: DEFAULT_SCHEMA_EPOCH,
            last_seen_instance_id: None,
        },
        ClientMessage::Subscribe {
            request_id: "req-health".to_owned(),
            topic: "daemon.health".to_owned(),
            from_seq: None,
        },
        ClientMessage::Unsubscribe {
            request_id: "req-unsub".to_owned(),
            topic: "daemon.health".to_owned(),
        },
    ])?;

    assert!(matches!(
        frames.first(),
        Some(ServerMessage::Welcome { .. })
    ));
    assert!(matches!(
        frames.get(1),
        Some(ServerMessage::CommandResponse { request_id, payload })
            if request_id == "req-health" && payload == &serde_json::Value::Null
    ));
    assert!(matches!(
        frames.get(2),
        Some(ServerMessage::TopicEvent { topic, payload })
            if topic == "daemon.health"
                && payload["daemon"] == "ready"
                && payload["control"] == "full"
    ));
    assert!(matches!(
        frames.get(3),
        Some(ServerMessage::CommandResponse { request_id, payload })
            if request_id == "req-unsub" && payload == &serde_json::Value::Null
    ));

    Ok(())
}

#[test]
fn it_ipc_014_router_accepts_supported_subscriptions() -> Result<()> {
    let mut setup = HandshakeHarness::new()?;
    setup.context.mutations_allowed = true;
    setup.context.lifecycle_frames = vec![LifecycleFrame {
        seq: 5,
        session_id: "session-1".to_owned(),
        phase: LifecyclePhase::Active,
        cleanup_verdict: None,
        archive_seq: None,
    }];

    let responses = setup.request_frames_with_router(&[
        ClientMessage::Hello {
            desktop_version: "desktop-1.0.0".to_owned(),
            supported_protocol_range: ProtocolRange { min: 1, max: 1 },
            expected_schema_epoch: DEFAULT_SCHEMA_EPOCH,
            last_seen_instance_id: None,
        },
        ClientMessage::Subscribe {
            request_id: "req-health".to_owned(),
            topic: "daemon.health".to_owned(),
            from_seq: None,
        },
        ClientMessage::Subscribe {
            request_id: "req-session".to_owned(),
            topic: "session.stream.session-1".to_owned(),
            from_seq: Some(4),
        },
        ClientMessage::Unsubscribe {
            request_id: "req-unsub".to_owned(),
            topic: "session.stream.session-1".to_owned(),
        },
    ])?;

    assert!(matches!(
        responses.first(),
        Some(ServerMessage::Welcome { .. })
    ));
    assert!(responses.iter().any(|response| {
        matches!(
            response,
            ServerMessage::CommandResponse { request_id, payload }
                if request_id == "req-health" && *payload == serde_json::Value::Null
        )
    }));
    assert!(responses.iter().any(|response| {
        matches!(
            response,
            ServerMessage::CommandResponse { request_id, payload }
                if request_id == "req-session" && *payload == serde_json::Value::Null
        )
    }));
    assert!(responses.iter().any(|response| {
        matches!(
            response,
            ServerMessage::CommandResponse { request_id, payload }
                if request_id == "req-unsub" && *payload == serde_json::Value::Null
        )
    }));
    Ok(())
}

#[test]
fn it_ipc_006_desktop_restart_while_daemon_stays_alive_allows_resume() -> Result<()> {
    let setup = HandshakeHarness::new()?;
    let response = setup.request(&ClientMessage::Hello {
        desktop_version: "desktop-1.0.0".to_owned(),
        supported_protocol_range: ProtocolRange { min: 1, max: 1 },
        expected_schema_epoch: DEFAULT_SCHEMA_EPOCH,
        last_seen_instance_id: Some(setup.context.instance_id.clone()),
    })?;

    match response {
        ServerMessage::Welcome {
            instance_id,
            reconnect_outcome,
            stream_resume_permitted,
            ..
        } => {
            assert_eq!(instance_id, setup.context.instance_id);
            assert_eq!(reconnect_outcome, ReconnectOutcome::SameDaemonReconnect);
            assert!(stream_resume_permitted);
        }
        other => bail!("unexpected response: {other:?}"),
    }

    Ok(())
}

#[test]
fn it_ipc_007_daemon_restart_changes_instance_id_and_invalidates_resume() -> Result<()> {
    let mut setup = HandshakeHarness::new()?;
    let first_instance = setup.context.instance_id.clone();
    let first_descriptor = read_descriptor(setup.runtime_paths.descriptor_path())?;
    assert_eq!(first_descriptor.instance_id, first_instance);

    setup.restart_daemon()?;
    let restarted_instance = setup.context.instance_id.clone();
    assert_ne!(restarted_instance, first_instance);

    let restarted_descriptor = read_descriptor(setup.runtime_paths.descriptor_path())?;
    assert_eq!(restarted_descriptor.instance_id, restarted_instance);

    let response = setup.request(&ClientMessage::Hello {
        desktop_version: "desktop-1.0.0".to_owned(),
        supported_protocol_range: ProtocolRange { min: 1, max: 1 },
        expected_schema_epoch: DEFAULT_SCHEMA_EPOCH,
        last_seen_instance_id: Some(first_instance),
    })?;

    match response {
        ServerMessage::Welcome {
            instance_id,
            reconnect_outcome,
            stream_resume_permitted,
            ..
        } => {
            assert_eq!(instance_id, restarted_instance);
            assert_eq!(
                reconnect_outcome,
                ReconnectOutcome::DaemonRestartInvalidated
            );
            assert!(!stream_resume_permitted);
        }
        other => bail!("unexpected response: {other:?}"),
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
        other => bail!("unexpected response: {other:?}"),
    }

    Ok(())
}

#[test]
fn it_ipc_011_archived_and_reaped_lifecycle_frames_carry_terminal_metadata() -> Result<()> {
    let mut setup = HandshakeHarness::new()?;
    setup.context.lifecycle_frames = vec![
        LifecycleFrame {
            seq: 7,
            session_id: "session-1".to_owned(),
            phase: LifecyclePhase::Created,
            cleanup_verdict: None,
            archive_seq: None,
        },
        LifecycleFrame {
            seq: 8,
            session_id: "session-1".to_owned(),
            phase: LifecyclePhase::Reaped,
            cleanup_verdict: Some("clean".to_owned()),
            archive_seq: None,
        },
        LifecycleFrame {
            seq: 9,
            session_id: "session-1".to_owned(),
            phase: LifecyclePhase::Archived,
            cleanup_verdict: None,
            archive_seq: Some(42),
        },
    ];

    let frames = setup.request_frames(&ClientMessage::Hello {
        desktop_version: "desktop-1.0.0".to_owned(),
        supported_protocol_range: ProtocolRange { min: 1, max: 1 },
        expected_schema_epoch: DEFAULT_SCHEMA_EPOCH,
        last_seen_instance_id: None,
    })?;
    assert_eq!(frames.len(), 4);

    match &frames[1] {
        ServerMessage::Lifecycle {
            seq,
            phase,
            cleanup_verdict,
            archive_seq,
            ..
        } => {
            assert_eq!(*seq, 7);
            assert_eq!(*phase, LifecyclePhase::Created);
            assert_eq!(cleanup_verdict, &None);
            assert_eq!(archive_seq, &None);
        }
        other => bail!("expected lifecycle frame, got {other:?}"),
    }

    match &frames[2] {
        ServerMessage::Lifecycle {
            seq,
            session_id,
            phase,
            cleanup_verdict,
            archive_seq,
        } => {
            assert_eq!(*seq, 8);
            assert_eq!(session_id, "session-1");
            assert_eq!(*phase, LifecyclePhase::Reaped);
            assert_eq!(cleanup_verdict.as_deref(), Some("clean"));
            assert_eq!(*archive_seq, None);
        }
        other => bail!("expected lifecycle frame, got {other:?}"),
    }

    match &frames[3] {
        ServerMessage::Lifecycle {
            seq,
            session_id,
            phase,
            cleanup_verdict,
            archive_seq,
        } => {
            assert_eq!(*seq, 9);
            assert_eq!(session_id, "session-1");
            assert_eq!(*phase, LifecyclePhase::Archived);
            assert_eq!(*cleanup_verdict, None);
            assert_eq!(*archive_seq, Some(42));
        }
        other => bail!("expected lifecycle frame, got {other:?}"),
    }

    let archived_json = serde_json::to_value(&frames[3])?;
    assert_eq!(archived_json["messageType"], "lifecycle");
    assert_eq!(archived_json["phase"], "archived");
    assert_eq!(archived_json["archiveSeq"], 42);

    let reaped_json = serde_json::to_value(&frames[2])?;
    assert_eq!(reaped_json["messageType"], "lifecycle");
    assert_eq!(reaped_json["phase"], "reaped");
    assert_eq!(reaped_json["cleanupVerdict"], "clean");

    Ok(())
}

#[test]
fn it_ipc_012_handshake_distinguishes_same_daemon_reconnect_from_restart() -> Result<()> {
    let mut setup = HandshakeHarness::new()?;
    let original_instance = setup.context.instance_id.clone();

    let resumed = setup.request(&ClientMessage::Hello {
        desktop_version: "desktop-1.0.0".to_owned(),
        supported_protocol_range: ProtocolRange { min: 1, max: 1 },
        expected_schema_epoch: DEFAULT_SCHEMA_EPOCH,
        last_seen_instance_id: Some(original_instance.clone()),
    })?;

    setup.restart_daemon()?;
    let restarted_instance = setup.context.instance_id.clone();
    assert_ne!(restarted_instance, original_instance);

    let invalidated = setup.request(&ClientMessage::Hello {
        desktop_version: "desktop-1.0.0".to_owned(),
        supported_protocol_range: ProtocolRange { min: 1, max: 1 },
        expected_schema_epoch: DEFAULT_SCHEMA_EPOCH,
        last_seen_instance_id: Some(original_instance),
    })?;

    match resumed {
        ServerMessage::Welcome {
            reconnect_outcome,
            stream_resume_permitted,
            ..
        } => {
            assert_eq!(reconnect_outcome, ReconnectOutcome::SameDaemonReconnect);
            assert!(stream_resume_permitted);
        }
        other => bail!("expected welcome frame, got {other:?}"),
    }

    match invalidated {
        ServerMessage::Welcome {
            reconnect_outcome,
            stream_resume_permitted,
            ..
        } => {
            assert_eq!(
                reconnect_outcome,
                ReconnectOutcome::DaemonRestartInvalidated
            );
            assert!(!stream_resume_permitted);
        }
        other => bail!("expected welcome frame, got {other:?}"),
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
    test_context: support::TestContext,
    context: HandshakeContext,
    listener: Option<UnixListener>,
    runtime_paths: RuntimePaths,
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
        let descriptor = DaemonDescriptor::new(
            runtime_paths.socket_path(),
            runtime_paths.state_root(),
            DEFAULT_SCHEMA_EPOCH,
        );
        write_descriptor_atomic(runtime_paths.descriptor_path(), &descriptor)?;
        let context = HandshakeContext {
            protocol_version: 1,
            daemon_version: env!("CARGO_PKG_VERSION").to_owned(),
            instance_id: descriptor.instance_id,
            schema_epoch: DEFAULT_SCHEMA_EPOCH,
            commit_seq: 0,
            health_state: HealthState {
                status: "ready".to_owned(),
            },
            active_sessions: ActiveSessions::default(),
            mutations_allowed: false,
            expected_peer_uid: None,
            lifecycle_frames: Vec::new(),
        };

        let socket_path = runtime_paths.socket_path().to_path_buf();

        Ok(Self {
            test_context: ctx,
            context,
            listener: Some(listener),
            runtime_paths,
            socket_path,
        })
    }

    fn request(&self, message: &ClientMessage) -> Result<ServerMessage> {
        self.request_frames(message)?
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("daemon closed handshake stream without a response"))
    }

    fn request_frames(&self, message: &ClientMessage) -> Result<Vec<ServerMessage>> {
        let listener = self
            .listener
            .as_ref()
            .ok_or_else(|| anyhow!("handshake listener is not active"))?
            .try_clone()?;
        let context = self.context.clone();
        let handle = thread::spawn(move || handshake_impl::serve_one(&listener, &context, None));

        let mut client = UnixStream::connect(&self.socket_path)
            .with_context(|| format!("connecting to {}", self.socket_path.display()))?;
        let payload = serde_json::to_string(message)?;
        client.write_all(payload.as_bytes())?;
        client.write_all(b"\n")?;
        client.flush()?;

        let reader = BufReader::new(client);
        let mut frames = Vec::new();
        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            frames.push(serde_json::from_str(&line).context("parsing handshake response")?);
        }

        handle
            .join()
            .map_err(|_| anyhow!("handshake thread panicked"))??;
        Ok(frames)
    }

    fn request_frames_with_router(&self, messages: &[ClientMessage]) -> Result<Vec<ServerMessage>> {
        let listener = self
            .listener
            .as_ref()
            .ok_or_else(|| anyhow!("handshake listener is not active"))?
            .try_clone()?;
        let context = HandshakeContext {
            mutations_allowed: true,
            ..self.context.clone()
        };
        let socket_path = self.socket_path.clone();
        let descriptor = DaemonDescriptor::new(
            self.runtime_paths.socket_path(),
            self.runtime_paths.state_root(),
            DEFAULT_SCHEMA_EPOCH,
        );
        let artifact_root = self.test_context.artifact_root.clone();
        let handle = thread::spawn(move || -> Result<()> {
            let test_db = TestStateDb::new()?;
            let mut state_db = StateDb::open(test_db.db_path())?;
            let restart_controller = control_handler_impl::RestartController::new(
                control_handler_impl::RestartPlan::new(PathBuf::from("/bin/true"), Vec::new(), 0),
            );
            let mut router = command_router_impl::CommandRouter::new(
                control_handler_impl::ControlHandlerContext {
                    descriptor: &descriptor,
                    artifact_root: &artifact_root,
                    restart_controller: &restart_controller,
                },
                jobs_handler_impl::JobsHandlerContext {
                    state_db: &mut state_db,
                },
            );
            handshake_impl::serve_one(&listener, &context, Some(&mut router))
        });

        let mut client = UnixStream::connect(&socket_path)
            .with_context(|| format!("connecting to {}", socket_path.display()))?;
        let reader_stream = client.try_clone()?;
        reader_stream.set_read_timeout(Some(std::time::Duration::from_millis(25)))?;
        let mut reader = BufReader::new(reader_stream);
        let mut frames = Vec::new();
        for message in messages {
            let payload = serde_json::to_string(message)?;
            client.write_all(payload.as_bytes())?;
            client.write_all(b"\n")?;
            client.flush()?;
            drain_available_frames(&mut reader, &mut frames)?;
        }
        client.shutdown(Shutdown::Write)?;
        drain_remaining_frames(reader, &mut frames)?;

        handle
            .join()
            .map_err(|_| anyhow!("handshake thread panicked"))??;
        Ok(frames)
    }

    fn restart_daemon(&mut self) -> Result<()> {
        self.listener.take();
        let listener = bind_listener(&self.runtime_paths)?;
        let descriptor = DaemonDescriptor::new(
            self.runtime_paths.socket_path(),
            self.runtime_paths.state_root(),
            DEFAULT_SCHEMA_EPOCH,
        );
        write_descriptor_atomic(self.runtime_paths.descriptor_path(), &descriptor)?;
        self.context.instance_id = descriptor.instance_id;
        self.listener = Some(listener);
        Ok(())
    }
}

fn drain_available_frames(
    reader: &mut BufReader<UnixStream>,
    frames: &mut Vec<ServerMessage>,
) -> Result<()> {
    loop {
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => {
                if line.trim().is_empty() {
                    continue;
                }
                frames.push(serde_json::from_str(&line).context("parsing handshake response")?);
            }
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) =>
            {
                break;
            }
            Err(error) => return Err(error).context("reading handshake response"),
        }
    }
    Ok(())
}

fn drain_remaining_frames(
    reader: BufReader<UnixStream>,
    frames: &mut Vec<ServerMessage>,
) -> Result<()> {
    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        frames.push(serde_json::from_str(&line).context("parsing handshake response")?);
    }
    Ok(())
}
