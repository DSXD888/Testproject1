#![forbid(unsafe_code)]

use anyhow::{Context, Result};
use nix::sys::signal::{Signal, kill};
use nix::unistd::Pid;
use rusqlite::{OptionalExtension, params};
use serde::Deserialize;
use thiserror::Error;

#[cfg(not(test))]
use crate::db::sqlite::StateDb;
use crate::jobs::runner::MutationAck;
#[cfg(test)]
use crate::support::db::StateDb;

pub struct SessionsHandlerContext<'a> {
    pub state_db: &'a mut StateDb,
}

#[derive(Debug, Clone, Error)]
#[error("{code}: {message}")]
pub struct SessionHandlerError {
    pub code: String,
    pub message: String,
    pub retryable: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StopSessionParams {
    pub session_id: String,
    pub mode: StopMode,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum StopMode {
    Graceful,
    Force,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SendSessionInputParams {
    pub session_id: String,
    pub input_utf8: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResizeSessionParams {
    pub session_id: String,
    pub cols: u16,
    pub rows: u16,
}

#[derive(Debug)]
struct SessionControlRow {
    status: String,
    execution_mode: String,
    pty_session_id: Option<String>,
    pgid: Option<i32>,
}

pub fn stop_session(
    context: &mut SessionsHandlerContext<'_>,
    params: &StopSessionParams,
) -> std::result::Result<MutationAck, SessionHandlerError> {
    let session = load_session(context.state_db, &params.session_id)
        .map_err(map_internal_error)?
        .ok_or_else(|| SessionHandlerError {
            code: "not_found".to_owned(),
            message: format!("unknown session `{}`", params.session_id),
            retryable: false,
        })?;

    if is_terminal_status(&session.status) {
        return Err(SessionHandlerError {
            code: "already_terminal".to_owned(),
            message: format!("session `{}` is already terminal", params.session_id),
            retryable: false,
        });
    }

    let process_group_id = session.pgid.ok_or_else(|| SessionHandlerError {
        code: "conflict".to_owned(),
        message: format!(
            "session `{}` does not expose a process group",
            params.session_id
        ),
        retryable: false,
    })?;

    send_stop_signal(process_group_id, params.mode).map_err(map_internal_error)?;

    let ((), visible_after_seq) = context
        .state_db
        .with_write_transaction(|tx| {
            tx.execute(
                "\
                UPDATE agent_session_logs
                SET status = 'terminating'
                WHERE id = ?1
                ",
                params![params.session_id],
            )?;
            Ok(())
        })
        .map_err(map_internal_error)?;

    Ok(MutationAck {
        accepted: true,
        visible_after_seq,
        continuation_job_id: None,
    })
}

pub fn send_session_input(
    context: &SessionsHandlerContext<'_>,
    params: &SendSessionInputParams,
) -> std::result::Result<MutationAck, SessionHandlerError> {
    let session = require_interactive_pty_session(context.state_db, &params.session_id)?;
    let _ = session;
    let _ = params.input_utf8.as_str();

    Err(SessionHandlerError {
        code: "not_interactive".to_owned(),
        message: "live PTY attachment is not available through the command surface yet".to_owned(),
        retryable: false,
    })
}

pub fn resize_session(
    context: &SessionsHandlerContext<'_>,
    params: &ResizeSessionParams,
) -> std::result::Result<MutationAck, SessionHandlerError> {
    if params.cols == 0 || params.rows == 0 {
        return Err(SessionHandlerError {
            code: "invalid_params".to_owned(),
            message: "invalid terminal geometry".to_owned(),
            retryable: false,
        });
    }
    let session = require_interactive_pty_session(context.state_db, &params.session_id)?;
    let _ = session;

    Err(SessionHandlerError {
        code: "not_interactive".to_owned(),
        message: "live PTY attachment is not available through the command surface yet".to_owned(),
        retryable: false,
    })
}

fn require_interactive_pty_session(
    state_db: &StateDb,
    session_id: &str,
) -> std::result::Result<SessionControlRow, SessionHandlerError> {
    let session = load_session(state_db, session_id)
        .map_err(map_internal_error)?
        .ok_or_else(|| SessionHandlerError {
            code: "not_found".to_owned(),
            message: format!("unknown session `{session_id}`"),
            retryable: false,
        })?;

    if is_terminal_status(&session.status) {
        return Err(SessionHandlerError {
            code: "already_terminal".to_owned(),
            message: format!("session `{session_id}` is already terminal"),
            retryable: false,
        });
    }

    if session.execution_mode != "pty" || session.pty_session_id.is_none() {
        return Err(SessionHandlerError {
            code: "not_interactive".to_owned(),
            message: format!("session `{session_id}` is not an attached PTY session"),
            retryable: false,
        });
    }

    Ok(session)
}

fn load_session(state_db: &StateDb, session_id: &str) -> Result<Option<SessionControlRow>> {
    state_db
        .connection()
        .query_row(
            "\
            SELECT status, execution_mode, pty_session_id, pgid
            FROM agent_session_logs
            WHERE id = ?1
            ",
            params![session_id],
            |row| {
                Ok(SessionControlRow {
                    status: row.get(0)?,
                    execution_mode: row.get(1)?,
                    pty_session_id: row.get(2)?,
                    pgid: row.get(3)?,
                })
            },
        )
        .optional()
        .context("loading session control row")
}

fn send_stop_signal(process_group_id: i32, mode: StopMode) -> Result<()> {
    let signal = match mode {
        StopMode::Graceful => Signal::SIGINT,
        StopMode::Force => Signal::SIGKILL,
    };
    kill(Pid::from_raw(-process_group_id), signal)
        .with_context(|| format!("sending {signal:?} to process group {process_group_id}"))?;
    Ok(())
}

fn is_terminal_status(status: &str) -> bool {
    matches!(status, "completed" | "failed" | "cancelled" | "obsolete")
}

#[allow(clippy::needless_pass_by_value)]
fn map_internal_error(error: anyhow::Error) -> SessionHandlerError {
    SessionHandlerError {
        code: "busy".to_owned(),
        message: error.to_string(),
        retryable: true,
    }
}
