#![forbid(unsafe_code)]

use std::io::{self, BufRead, BufReader, Read};
use std::os::fd::RawFd;
use std::sync::{Arc, Mutex};

use nix::errno::Errno;
use nix::fcntl::{FcntlArg, FdFlag, fcntl};
use nix::sys::wait::{WaitPidFlag, WaitStatus, waitpid};
use portable_pty::MasterPty;

use super::{DEFAULT_PTY_COLS, DEFAULT_PTY_ROWS, PtyGeometry, PtySessionError, RuntimeIds};

pub(super) const fn default_headless_geometry() -> PtyGeometry {
    PtyGeometry::new(DEFAULT_PTY_COLS, DEFAULT_PTY_ROWS)
}

pub(super) const fn normalize_headless_geometry(geometry: PtyGeometry) -> PtyGeometry {
    if geometry.cols == 0 || geometry.rows == 0 {
        default_headless_geometry()
    } else {
        geometry
    }
}

pub(super) fn master_has_cloexec(
    master_fd: Option<RawFd>,
    runtime_ids: Option<RuntimeIds>,
) -> Result<bool, PtySessionError> {
    let Some(master_fd) = master_fd else {
        return Ok(false);
    };
    let flags =
        fcntl(master_fd, FcntlArg::F_GETFD).map_err(|error| PtySessionError::AgentRunFailed {
            detail: format!("failed to inspect master fd flags: {error}"),
            runtime_ids,
        })?;
    let fd_flags = FdFlag::from_bits_truncate(flags);
    Ok(fd_flags.contains(FdFlag::FD_CLOEXEC))
}

pub(super) fn apply_spawn_geometry(
    master: &Arc<Mutex<Box<dyn MasterPty + Send>>>,
    geometry: PtyGeometry,
) -> Result<(), PtySessionError> {
    resize_master(master, geometry, None)
}

pub(super) fn resize_for_operator(
    master: &Arc<Mutex<Box<dyn MasterPty + Send>>>,
    cols: u16,
    rows: u16,
    runtime_ids: Option<RuntimeIds>,
) -> Result<PtyGeometry, PtySessionError> {
    let geometry = normalize_headless_geometry(PtyGeometry::new(cols, rows));
    resize_master(master, geometry, runtime_ids)?;
    Ok(geometry)
}

pub(super) fn read_line(
    reader: &mut BufReader<Box<dyn Read + Send>>,
    runtime_ids: RuntimeIds,
) -> Result<String, PtySessionError> {
    let mut buffer = String::new();
    match reader.read_line(&mut buffer) {
        Ok(_) => Ok(buffer),
        Err(error) if is_expected_pty_hangup(&error) => Ok(String::new()),
        Err(error) => Err(PtySessionError::AgentRunFailed {
            detail: format!("failed reading PTY output: {error}"),
            runtime_ids: Some(runtime_ids),
        }),
    }
}

#[allow(dead_code)]
pub(super) fn drain_waitpid_wakeups() -> Result<Vec<u32>, PtySessionError> {
    let mut reaped = Vec::new();
    loop {
        match waitpid(None, Some(WaitPidFlag::WNOHANG)) {
            Ok(
                WaitStatus::Exited(pid, _)
                | WaitStatus::Signaled(pid, _, _)
                | WaitStatus::Stopped(pid, _)
                | WaitStatus::Continued(pid),
            ) => reaped.push(pid.as_raw().unsigned_abs()),
            Ok(WaitStatus::StillAlive) | Err(Errno::ECHILD) => break,
            Err(error) => {
                return Err(PtySessionError::AgentRunFailed {
                    detail: format!("failed draining waitpid wakeups: {error}"),
                    runtime_ids: None,
                });
            }
        }
    }
    Ok(reaped)
}

fn resize_master(
    master: &Arc<Mutex<Box<dyn MasterPty + Send>>>,
    geometry: PtyGeometry,
    runtime_ids: Option<RuntimeIds>,
) -> Result<(), PtySessionError> {
    master
        .lock()
        .map_err(|_| PtySessionError::AgentRunFailed {
            detail: "PTY master lock poisoned".to_owned(),
            runtime_ids,
        })?
        .resize(geometry.to_pty_size())
        .map_err(|error| PtySessionError::AgentRunFailed {
            detail: format!("failed resizing PTY: {error:#}"),
            runtime_ids,
        })?;
    Ok(())
}

fn is_expected_pty_hangup(error: &io::Error) -> bool {
    error.raw_os_error() == Some(Errno::EIO as i32)
}
