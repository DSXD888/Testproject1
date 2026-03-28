#![forbid(unsafe_code)]

use std::ffi::OsString;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Mutex;

use anyhow::{Context, Result};
use serde::Serialize;
use thiserror::Error;

use crate::db::sqlite::StateDb;
use crate::ipc::descriptor::{DESCRIPTOR_FILENAME, DaemonDescriptor};
use crate::jobs::runner::MutationAck;

pub struct ControlHandlerContext<'a> {
    pub descriptor: &'a DaemonDescriptor,
    pub artifact_root: &'a Path,
    pub restart_controller: &'a RestartController,
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

#[derive(Debug, Clone)]
pub struct RestartPlan {
    program: PathBuf,
    args: Vec<OsString>,
    delay_ms: u64,
}

impl RestartPlan {
    #[must_use]
    pub const fn new(program: PathBuf, args: Vec<OsString>, delay_ms: u64) -> Self {
        Self {
            program,
            args,
            delay_ms,
        }
    }

    fn spawn_replacement(&self) -> Result<()> {
        let mut command = std::process::Command::new(&self.program);
        command
            .args(&self.args)
            .env("DAEMON_RESTART_DELAY_MS", self.delay_ms.to_string())
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        command
            .spawn()
            .with_context(|| format!("spawning daemon restart from {}", self.program.display()))?;
        Ok(())
    }
}

#[derive(Debug)]
struct RestartIntent {
    idempotency_key: String,
    ack: MutationAck,
}

pub struct RestartController {
    plan: RestartPlan,
    pending: Mutex<Option<RestartIntent>>,
}

impl RestartController {
    #[must_use]
    pub const fn new(plan: RestartPlan) -> Self {
        Self {
            plan,
            pending: Mutex::new(None),
        }
    }

    pub fn request_restart(
        &self,
        state_db: &mut StateDb,
        idempotency_key: &str,
    ) -> std::result::Result<MutationAck, ControlHandlerError> {
        let mut pending = self.pending.lock().map_err(|_| ControlHandlerError {
            code: "busy".to_owned(),
            message: "restart controller lock poisoned".to_owned(),
            retryable: true,
        })?;

        if let Some(existing) = pending.as_ref() {
            if existing.idempotency_key == idempotency_key {
                return Ok(existing.ack.clone());
            }
            return Err(ControlHandlerError {
                code: "conflict".to_owned(),
                message: "daemon restart is already pending".to_owned(),
                retryable: false,
            });
        }

        let ((), visible_after_seq) = state_db
            .with_write_transaction(|_| Ok(()))
            .map_err(map_internal_error)?;

        self.plan.spawn_replacement().map_err(map_internal_error)?;

        let ack = MutationAck {
            accepted: true,
            visible_after_seq,
            continuation_job_id: None,
        };
        *pending = Some(RestartIntent {
            idempotency_key: idempotency_key.to_owned(),
            ack: ack.clone(),
        });
        drop(pending);
        Ok(ack)
    }

    #[must_use]
    pub fn restart_requested(&self) -> bool {
        self.pending.lock().is_ok_and(|pending| pending.is_some())
    }
}

#[derive(Debug, Clone, Error)]
#[error("{code}: {message}")]
pub struct ControlHandlerError {
    pub code: String,
    pub message: String,
    pub retryable: bool,
}

pub fn get_daemon_info(
    context: &ControlHandlerContext<'_>,
    state_db: &StateDb,
) -> Result<DaemonInfoSnapshot> {
    let descriptor_path = descriptor_path_from_socket(&context.descriptor.socket_path);
    let state_db_path = state_db_path_from_root(&context.descriptor.state_root);
    let wal_bytes = wal_bytes_for(&state_db_path)?;
    let startup_recovery = load_startup_recovery(state_db)?;

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

pub fn restart_daemon(
    context: &ControlHandlerContext<'_>,
    state_db: &mut StateDb,
    idempotency_key: &str,
) -> std::result::Result<MutationAck, ControlHandlerError> {
    context
        .restart_controller
        .request_restart(state_db, idempotency_key)
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

#[allow(clippy::needless_pass_by_value)]
fn map_internal_error(error: anyhow::Error) -> ControlHandlerError {
    ControlHandlerError {
        code: "busy".to_owned(),
        message: error.to_string(),
        retryable: true,
    }
}
