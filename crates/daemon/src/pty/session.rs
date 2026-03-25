#![forbid(unsafe_code)]

use anyhow::Result;
use nix::fcntl::{FcntlArg, FdFlag, fcntl};
use nix::unistd::{Pid, getpgid, getsid};
use portable_pty::{
    Child, CommandBuilder, MasterPty, NativePtySystem, PtySize, PtySystem, SlavePty,
};
use std::io::{BufRead, BufReader, Read, Write};
use std::os::fd::RawFd;
use thiserror::Error;

pub const DEFAULT_PTY_COLS: u16 = 120;
pub const DEFAULT_PTY_ROWS: u16 = 40;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionPhase {
    Created,
    Spawned,
    Active,
    Terminating,
    Exited,
    Reaped,
    Archived,
}

impl SessionPhase {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Spawned => "spawned",
            Self::Active => "active",
            Self::Terminating => "terminating",
            Self::Exited => "exited",
            Self::Reaped => "reaped",
            Self::Archived => "archived",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpawnPath {
    PortablePty,
    HelperBinary,
}

impl SpawnPath {
    pub const fn resolve(portable_pty_pre_exec_safe: bool) -> Self {
        if portable_pty_pre_exec_safe {
            Self::PortablePty
        } else {
            Self::HelperBinary
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PtyGeometry {
    pub cols: u16,
    pub rows: u16,
}

impl PtyGeometry {
    pub const fn new(cols: u16, rows: u16) -> Self {
        Self { cols, rows }
    }

    pub const fn to_pty_size(self) -> PtySize {
        PtySize {
            cols: self.cols,
            rows: self.rows,
            pixel_width: 0,
            pixel_height: 0,
        }
    }
}

impl Default for PtyGeometry {
    fn default() -> Self {
        Self::new(DEFAULT_PTY_COLS, DEFAULT_PTY_ROWS)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BrowserViewport {
    pub width: u16,
    pub height: u16,
}

impl Default for BrowserViewport {
    fn default() -> Self {
        Self {
            width: 1280,
            height: 720,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PtyCommand {
    program: String,
    args: Vec<String>,
}

impl PtyCommand {
    pub fn new(program: impl Into<String>) -> Self {
        Self {
            program: program.into(),
            args: Vec::new(),
        }
    }

    pub fn arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }

    fn into_builder(self) -> CommandBuilder {
        let mut builder = CommandBuilder::new(self.program);
        for arg in self.args {
            builder.arg(arg);
        }
        builder.set_controlling_tty(true);
        builder
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpawnOptions {
    pub geometry: PtyGeometry,
    pub browser_viewport: BrowserViewport,
    pub spawn_path: SpawnPath,
}

impl Default for SpawnOptions {
    fn default() -> Self {
        Self {
            geometry: PtyGeometry::default(),
            browser_viewport: BrowserViewport::default(),
            spawn_path: SpawnPath::resolve(true),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeIds {
    pub sid: u32,
    pub pid: u32,
    pub pgid: u32,
}

#[derive(Debug, Error)]
pub enum PtySessionError {
    #[error("AGENT_RUN_FAILED: {detail}")]
    AgentRunFailed {
        detail: String,
        runtime_ids: Option<RuntimeIds>,
    },
    #[error("AGENT_RUN_FAILED: helper-binary path required for safe PTY spawn")]
    HelperBinaryRequired,
}

impl PtySessionError {
    pub const fn classification(&self) -> &'static str {
        match self {
            Self::AgentRunFailed { .. } | Self::HelperBinaryRequired => "AGENT_RUN_FAILED",
        }
    }

    pub const fn runtime_ids(&self) -> Option<RuntimeIds> {
        match self {
            Self::AgentRunFailed { runtime_ids, .. } => *runtime_ids,
            Self::HelperBinaryRequired => None,
        }
    }
}

pub struct PtyPairHandle {
    master: Box<dyn MasterPty + Send>,
    slave: Box<dyn SlavePty + Send>,
    geometry: PtyGeometry,
    master_fd: Option<RawFd>,
}

impl PtyPairHandle {
    pub fn open(geometry: PtyGeometry) -> Result<Self, PtySessionError> {
        let pty_system = NativePtySystem::default();
        let pair = pty_system
            .openpty(geometry.to_pty_size())
            .map_err(|error| PtySessionError::AgentRunFailed {
                detail: format!("failed to create PTY pair: {error:#}"),
                runtime_ids: None,
            })?;
        let portable_pty::PtyPair { master, slave } = pair;
        let master_fd = master.as_raw_fd();
        Ok(Self {
            master,
            slave,
            geometry,
            master_fd,
        })
    }

    pub const fn geometry(&self) -> PtyGeometry {
        self.geometry
    }

    pub const fn master_fd(&self) -> Option<RawFd> {
        self.master_fd
    }

    pub fn master_has_cloexec(&self) -> Result<bool, PtySessionError> {
        let Some(master_fd) = self.master_fd else {
            return Ok(false);
        };
        let flags = fcntl(master_fd, FcntlArg::F_GETFD).map_err(|error| {
            PtySessionError::AgentRunFailed {
                detail: format!("failed to inspect master fd flags: {error}"),
                runtime_ids: None,
            }
        })?;
        let fd_flags = FdFlag::from_bits_truncate(flags);
        Ok(fd_flags.contains(FdFlag::FD_CLOEXEC))
    }

    fn into_parts(
        self,
    ) -> (
        Box<dyn MasterPty + Send>,
        Box<dyn SlavePty + Send>,
        Option<RawFd>,
        PtyGeometry,
    ) {
        (self.master, self.slave, self.master_fd, self.geometry)
    }
}

pub struct PtySession {
    child: Box<dyn Child + Send + Sync>,
    reader: BufReader<Box<dyn Read + Send>>,
    writer: Box<dyn Write + Send>,
    master: Box<dyn MasterPty + Send>,
    pub runtime_ids: RuntimeIds,
    pub geometry: PtyGeometry,
    pub browser_viewport: BrowserViewport,
    pub spawn_path: SpawnPath,
    pub phase_history: Vec<SessionPhase>,
    pub master_fd: Option<RawFd>,
}

impl PtySession {
    pub fn spawn(command: PtyCommand, options: SpawnOptions) -> Result<Self, PtySessionError> {
        if options.spawn_path == SpawnPath::HelperBinary {
            return Err(PtySessionError::HelperBinaryRequired);
        }

        let phase_history = vec![SessionPhase::Created];
        let pair = PtyPairHandle::open(options.geometry)?;
        let (master, slave, master_fd, geometry) = pair.into_parts();
        let reader =
            master
                .try_clone_reader()
                .map_err(|error| PtySessionError::AgentRunFailed {
                    detail: format!("failed to clone PTY reader: {error:#}"),
                    runtime_ids: None,
                })?;
        let writer = master
            .take_writer()
            .map_err(|error| PtySessionError::AgentRunFailed {
                detail: format!("failed to take PTY writer: {error:#}"),
                runtime_ids: None,
            })?;

        let child = slave
            .spawn_command(command.into_builder())
            .map_err(|error| PtySessionError::AgentRunFailed {
                detail: format!("failed to spawn PTY child: {error:#}"),
                runtime_ids: None,
            })?;
        let pid = child
            .process_id()
            .ok_or_else(|| PtySessionError::AgentRunFailed {
                detail: "PTY child did not expose a process id".to_owned(),
                runtime_ids: None,
            })?;
        let runtime_ids = record_runtime_ids(pid)?;

        Ok(Self {
            child,
            reader: BufReader::new(reader),
            writer,
            master,
            runtime_ids,
            geometry,
            browser_viewport: options.browser_viewport,
            spawn_path: options.spawn_path,
            phase_history: {
                let mut phases = phase_history;
                phases.push(SessionPhase::Spawned);
                phases.push(SessionPhase::Active);
                phases
            },
            master_fd,
        })
    }

    pub fn write_input(&mut self, input: impl AsRef<[u8]>) -> Result<(), PtySessionError> {
        self.writer
            .write_all(input.as_ref())
            .map_err(|error| PtySessionError::AgentRunFailed {
                detail: format!("failed writing to PTY: {error}"),
                runtime_ids: Some(self.runtime_ids),
            })?;
        self.writer
            .flush()
            .map_err(|error| PtySessionError::AgentRunFailed {
                detail: format!("failed flushing PTY writer: {error}"),
                runtime_ids: Some(self.runtime_ids),
            })?;
        Ok(())
    }

    pub fn read_line(&mut self) -> Result<String, PtySessionError> {
        let mut buffer = String::new();
        self.reader
            .read_line(&mut buffer)
            .map_err(|error| PtySessionError::AgentRunFailed {
                detail: format!("failed reading PTY output: {error}"),
                runtime_ids: Some(self.runtime_ids),
            })?;
        Ok(buffer)
    }

    pub fn resize(&mut self, cols: u16, rows: u16) -> Result<(), PtySessionError> {
        let next_geometry = PtyGeometry::new(cols, rows);
        self.master
            .resize(next_geometry.to_pty_size())
            .map_err(|error| PtySessionError::AgentRunFailed {
                detail: format!("failed resizing PTY: {error:#}"),
                runtime_ids: Some(self.runtime_ids),
            })?;
        self.geometry = next_geometry;
        Ok(())
    }

    pub fn finish(&mut self) -> Result<portable_pty::ExitStatus, PtySessionError> {
        self.phase_history.push(SessionPhase::Terminating);
        let status = self
            .child
            .wait()
            .map_err(|error| PtySessionError::AgentRunFailed {
                detail: format!("failed waiting for PTY child: {error}"),
                runtime_ids: Some(self.runtime_ids),
            })?;
        self.phase_history.push(SessionPhase::Exited);
        self.phase_history.push(SessionPhase::Reaped);
        self.phase_history.push(SessionPhase::Archived);
        Ok(status)
    }
}

pub const fn spawn_policy_steps() -> [&'static str; 5] {
    [
        "openpty",
        "setsid",
        "controlling_terminal",
        "stdio_redirect",
        "exec",
    ]
}

fn record_runtime_ids(pid: u32) -> Result<RuntimeIds, PtySessionError> {
    let pid = Pid::from_raw(pid.cast_signed());
    let sid = getsid(Some(pid)).map_err(|error| PtySessionError::AgentRunFailed {
        detail: format!("failed to query session id: {error}"),
        runtime_ids: None,
    })?;
    let process_group_id = getpgid(Some(pid)).map_err(|error| PtySessionError::AgentRunFailed {
        detail: format!("failed to query process group id: {error}"),
        runtime_ids: None,
    })?;

    Ok(RuntimeIds {
        sid: sid.as_raw().cast_unsigned(),
        pid: pid.as_raw().cast_unsigned(),
        pgid: process_group_id.as_raw().cast_unsigned(),
    })
}
