#![forbid(unsafe_code)]

use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Serialize;

use crate::db::sqlite::StateDb;
use crate::ipc::descriptor::{DESCRIPTOR_FILENAME, DaemonDescriptor};
use crate::jobs::runner::MutationAck;

pub struct ControlHandlerContext<'a> {
    pub descriptor: &'a DaemonDescriptor,
    pub state_db: &'a StateDb,
    pub artifact_root: &'a Path,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DaemonInfoSnapshot {
    pub instance_id: String,
    pub daemon_version: String,
    pub protocol_version: u32,
    pub schema_epoch: i64,
    pub socket_path: String,
    pub descriptor_path: String,
    pub state_root: String,
    pub state_db_path: String,
    pub artifact_root: String,
    pub wal_bytes: Option<u64>,
    pub started_at: String,
    pub startup_recovery: StartupRecovery,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartupRecovery {
    pub recovered_interrupted_sessions: usize,
    pub last_recovery_at: Option<String>,
}

pub fn get_daemon_info(context: &ControlHandlerContext<'_>) -> Result<DaemonInfoSnapshot> {
    let descriptor_path = descriptor_path_from_socket(&context.descriptor.socket_path);
    let state_db_path = state_db_path_from_root(&context.descriptor.state_root);
    let wal_bytes = wal_bytes_for(&state_db_path)?;
    let startup_recovery = load_startup_recovery(context.state_db)?;

    Ok(DaemonInfoSnapshot {
        instance_id: context.descriptor.instance_id.clone(),
        daemon_version: env!("CARGO_PKG_VERSION").to_owned(),
        protocol_version: context.descriptor.protocol_version,
        schema_epoch: context.descriptor.schema_epoch,
        socket_path: context.descriptor.socket_path.clone(),
        descriptor_path: descriptor_path.display().to_string(),
        state_root: context.descriptor.state_root.clone(),
        state_db_path: state_db_path.display().to_string(),
        artifact_root: context.artifact_root.display().to_string(),
        wal_bytes,
        started_at: context.descriptor.started_at.clone(),
        startup_recovery,
    })
}

pub fn restart_daemon(context: &ControlHandlerContext<'_>) -> Result<MutationAck> {
    Ok(MutationAck {
        accepted: true,
        visible_after_seq: context.state_db.current_commit_seq()?,
        continuation_job_id: None,
    })
}

fn descriptor_path_from_socket(socket_path: &str) -> PathBuf {
    let socket = Path::new(socket_path);
    socket
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(DESCRIPTOR_FILENAME)
}

fn state_db_path_from_root(state_root: &str) -> PathBuf {
    Path::new(state_root).join("state.db")
}

fn wal_bytes_for(state_db_path: &Path) -> Result<Option<u64>> {
    let wal_path = PathBuf::from(format!("{}-wal", state_db_path.display()));
    match fs::metadata(&wal_path) {
        Ok(metadata) => Ok(Some(metadata.len())),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error).with_context(|| format!("reading {}", wal_path.display())),
    }
}

fn load_startup_recovery(state_db: &StateDb) -> Result<StartupRecovery> {
    let mut stmt = state_db.connection().prepare(
        "\
        SELECT structured_trace, completed_at
        FROM agent_session_logs
        WHERE structured_trace IS NOT NULL
        ",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, Option<String>>(0)?,
            row.get::<_, Option<String>>(1)?,
        ))
    })?;

    let mut recovered_interrupted_sessions = 0_usize;
    let mut last_recovery_at: Option<String> = None;

    for row in rows {
        let (structured_trace, completed_at): (Option<String>, Option<String>) = row?;
        let Some(trace) = structured_trace else {
            continue;
        };
        let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&trace) else {
            continue;
        };
        let recovered = parsed
            .get("crash_recovered")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        if !recovered {
            continue;
        }

        recovered_interrupted_sessions += 1;
        if completed_at.as_ref() > last_recovery_at.as_ref() {
            last_recovery_at = completed_at;
        }
    }

    Ok(StartupRecovery {
        recovered_interrupted_sessions,
        last_recovery_at,
    })
}
