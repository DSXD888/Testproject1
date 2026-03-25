#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::PathBuf;

use anyhow::{Result, bail};
use async_trait::async_trait;
use bytes::Bytes;
use portable_pty::ExitStatus;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutputFrame {
    Pty(Bytes),
    Stdout(Bytes),
    Stderr(Bytes),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthChallengeKind {
    ReLogin,
    Mfa,
    Sso,
    PermissionPrompt,
    Unknown,
}

#[derive(Debug, Clone)]
pub enum ControlEvent {
    Started {
        pid: u32,
        pgid: u32,
        sid: u32,
        mode: ExecutionMode,
    },
    Checkpoint {
        seq: u32,
        path: PathBuf,
    },
    Progress {
        completed: u32,
        total: Option<u32>,
    },
    AuthChallenge {
        kind: AuthChallengeKind,
        detail: String,
    },
    Warning {
        code: String,
        detail: String,
    },
    FinalResultReady {
        path: PathBuf,
    },
    Exited {
        status: ExitStatus,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionMode {
    Pty,
    Contract,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchSpec {
    pub program: PathBuf,
    pub args: Vec<OsString>,
    pub env: BTreeMap<OsString, OsString>,
    pub cwd: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeCtx {
    pub working_directory: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillManifest {
    pub interaction_profile: InteractionProfile,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpawnRequest {
    pub program: PathBuf,
    pub args: Vec<OsString>,
    pub env: BTreeMap<OsString, OsString>,
    pub cwd: PathBuf,
}

#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityReport {
    pub cli_version: String,
    pub auth_healthy: bool,
    pub structured_output: bool,
    pub pty_viable: bool,
    pub browser_tools: bool,
    pub is_canary: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InteractionProfile {
    PtyRequired,
    ContractRequired,
    ContractPreferred { allow_pty_fallback: bool },
}

pub fn resolve_execution_mode(
    skill: &SkillManifest,
    caps: &CapabilityReport,
) -> Result<ExecutionMode> {
    match &skill.interaction_profile {
        InteractionProfile::PtyRequired => {
            if caps.pty_viable {
                Ok(ExecutionMode::Pty)
            } else {
                bail!("PTY execution is required but not viable")
            }
        }
        InteractionProfile::ContractRequired => {
            if caps.structured_output {
                Ok(ExecutionMode::Contract)
            } else {
                bail!("contract execution is required but structured output is unavailable")
            }
        }
        InteractionProfile::ContractPreferred { allow_pty_fallback } => {
            if caps.structured_output {
                Ok(ExecutionMode::Contract)
            } else if *allow_pty_fallback && caps.pty_viable {
                Ok(ExecutionMode::Pty)
            } else {
                bail!("skill cannot be executed under its declared interaction policy")
            }
        }
    }
}

#[async_trait]
pub trait AgentAdapter: Send + Sync {
    fn id(&self) -> &'static str;

    async fn probe(&self, ctx: &ProbeCtx) -> Result<CapabilityReport>;

    fn resolve_mode(&self, skill: &SkillManifest, caps: &CapabilityReport)
    -> Result<ExecutionMode>;

    fn build_launch_spec(&self, req: &SpawnRequest, mode: ExecutionMode) -> Result<LaunchSpec>;

    fn parser(&self, mode: ExecutionMode) -> Box<dyn AdapterParser>;
}

#[async_trait]
pub trait AdapterParser: Send {
    async fn on_output(&mut self, frame: &OutputFrame) -> Result<Vec<ControlEvent>>;

    async fn finish(&mut self) -> Result<Vec<ControlEvent>>;
}
