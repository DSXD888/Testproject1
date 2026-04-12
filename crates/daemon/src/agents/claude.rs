#![forbid(unsafe_code)]

use std::collections::BTreeSet;
use std::ffi::OsString;
use std::path::PathBuf;
use std::process::Command;

use anyhow::{Context, Result, anyhow, bail};
use async_trait::async_trait;
use portable_pty::ExitStatus;
use serde::Deserialize;
use serde_json::Value;

#[cfg(test)]
use crate::adapter_impl as adapter;
#[cfg(not(test))]
use crate::agents::adapter;

use adapter::{
    AdapterParser, AgentAdapter, AuthChallengeKind, CapabilityReport, ControlEvent, ExecutionMode,
    LaunchSpec, OutputFrame, ProbeCtx, SkillManifest, SpawnRequest, resolve_execution_mode,
};

const DEFAULT_PROGRAM: &str = "claude";
const KNOWN_GOOD_VERSIONS: &[&str] = &["1.2.3", "1.2.4"];
const INTERNAL_ERROR_CODE: &str = "AGENT_INTERNAL_ERROR";
const PROTOCOL_MISMATCH_CODE: &str = "AGENT_PROTOCOL_MISMATCH";

#[derive(Debug, Clone)]
pub struct ClaudeAdapter {
    program: PathBuf,
    known_versions: BTreeSet<String>,
}

#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Deserialize)]
struct ProbeEnvelope {
    auth_healthy: bool,
    structured_output: bool,
    pty_viable: bool,
    browser_tools: bool,
}

#[derive(Debug)]
struct ClaudeParser {
    mode: ExecutionMode,
    stdout_buffer: Vec<u8>,
    stderr_buffer: Vec<u8>,
    finished: bool,
}

impl Default for ClaudeAdapter {
    fn default() -> Self {
        Self::new(
            PathBuf::from(DEFAULT_PROGRAM),
            KNOWN_GOOD_VERSIONS
                .iter()
                .map(|version| (*version).to_owned())
                .collect(),
        )
    }
}

impl ClaudeAdapter {
    #[allow(clippy::missing_const_for_fn)]
    #[must_use]
    pub fn new(program: PathBuf, known_versions: BTreeSet<String>) -> Self {
        Self {
            program,
            known_versions,
        }
    }

    fn run_probe_command(&self, ctx: &ProbeCtx, args: &[&str]) -> Result<Vec<u8>> {
        let output = Command::new(&self.program)
            .args(args)
            .current_dir(&ctx.working_directory)
            .output()
            .with_context(|| {
                format!(
                    "{}: failed to spawn `{}` with args {:?}",
                    INTERNAL_ERROR_CODE,
                    self.program.display(),
                    args
                )
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!(
                "{}: `{}` {:?} exited unsuccessfully: {}",
                INTERNAL_ERROR_CODE,
                self.program.display(),
                args,
                stderr.trim()
            );
        }

        Ok(output.stdout)
    }

    fn parse_version(stdout: &[u8]) -> Result<String> {
        let text = String::from_utf8(stdout.to_vec())
            .with_context(|| format!("{INTERNAL_ERROR_CODE}: invalid UTF-8 in version output"))?;
        text.split_whitespace()
            .find(|token| {
                token.contains('.') && token.chars().next().is_some_and(|ch| ch.is_ascii_digit())
            })
            .map(str::to_owned)
            .ok_or_else(|| anyhow!("{INTERNAL_ERROR_CODE}: missing Claude CLI version token"))
    }
}

impl ClaudeParser {
    #[allow(clippy::missing_const_for_fn)]
    fn new(mode: ExecutionMode) -> Self {
        Self {
            mode,
            stdout_buffer: Vec::new(),
            stderr_buffer: Vec::new(),
            finished: false,
        }
    }

    fn parse_frame_value(&self, value: &Value) -> Result<Vec<ControlEvent>> {
        let kind = value
            .get("event")
            .or_else(|| value.get("type"))
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("{PROTOCOL_MISMATCH_CODE}: missing event/type field"))?;

        match kind {
            "started" => Ok(vec![ControlEvent::Started {
                pid: required_u32(value, "pid")?,
                pgid: required_u32(value, "pgid")?,
                sid: required_u32(value, "sid")?,
                mode: self.mode,
            }]),
            "checkpoint" => Ok(vec![ControlEvent::Checkpoint {
                seq: required_u32(value, "seq")?,
                path: PathBuf::from(required_str(value, "path")?),
            }]),
            "progress" => Ok(vec![ControlEvent::Progress {
                completed: required_u32(value, "completed")?,
                total: optional_u32(value, "total")?,
            }]),
            "result" | "final_result_ready" => Ok(vec![ControlEvent::FinalResultReady {
                path: PathBuf::from(required_str(value, "path")?),
            }]),
            "warning" => Ok(vec![ControlEvent::Warning {
                code: required_str(value, "code")?.to_owned(),
                detail: required_str(value, "detail")?.to_owned(),
            }]),
            "auth" | "auth_challenge" => Ok(vec![ControlEvent::AuthChallenge {
                kind: parse_auth_kind(required_str(value, "kind")?),
                detail: required_str(value, "detail")?.to_owned(),
            }]),
            "exited" => Ok(vec![ControlEvent::Exited {
                status: ExitStatus::with_exit_code(required_u32(value, "code")?),
            }]),
            other => bail!("{PROTOCOL_MISMATCH_CODE}: unsupported event `{other}`"),
        }
    }

    fn process_buffer(&self, buffer: &mut Vec<u8>, flush_tail: bool) -> Result<Vec<ControlEvent>> {
        let mut events = Vec::new();

        while let Some(newline_index) = buffer.iter().position(|byte| *byte == b'\n') {
            let line: Vec<u8> = buffer.drain(..=newline_index).collect();
            let trimmed = trim_line(&line);
            if trimmed.is_empty() {
                continue;
            }
            if self.mode == ExecutionMode::Pty {
                // In PTY mode, lines are mixed text and optional JSON events.
                // Skip non-JSON lines silently — only parse lines that look like JSON.
                if let Ok(value) = serde_json::from_slice::<Value>(trimmed)
                    && let Ok(parsed) = self.parse_frame_value(&value)
                {
                    events.extend(parsed);
                }
            } else {
                let value: Value = serde_json::from_slice(trimmed)
                    .with_context(|| format!("{PROTOCOL_MISMATCH_CODE}: invalid JSON frame"))?;
                events.extend(self.parse_frame_value(&value)?);
            }
        }

        if flush_tail && !trim_line(buffer).is_empty() {
            let tail = std::mem::take(buffer);
            if self.mode == ExecutionMode::Pty {
                if let Ok(value) = serde_json::from_slice::<Value>(trim_line(&tail))
                    && let Ok(parsed) = self.parse_frame_value(&value)
                {
                    events.extend(parsed);
                }
            } else {
                let value: Value = serde_json::from_slice(trim_line(&tail)).with_context(|| {
                    format!("{PROTOCOL_MISMATCH_CODE}: truncated stream tail or invalid JSON")
                })?;
                events.extend(self.parse_frame_value(&value)?);
            }
        }

        Ok(events)
    }
}

#[async_trait]
impl AgentAdapter for ClaudeAdapter {
    fn id(&self) -> &'static str {
        "claude"
    }

    async fn probe(&self, ctx: &ProbeCtx) -> Result<CapabilityReport> {
        let version_stdout = self.run_probe_command(ctx, &["--version"])?;
        let cli_version = Self::parse_version(&version_stdout)?;

        let envelope = match self.run_probe_command(ctx, &["probe", "--json"]) {
            Ok(probe_stdout) => serde_json::from_slice::<ProbeEnvelope>(&probe_stdout)
                .with_context(|| format!("{INTERNAL_ERROR_CODE}: invalid Claude probe JSON"))?,
            Err(_) => {
                // Claude CLI v2+ does not ship `probe --json`; assume full capabilities.
                ProbeEnvelope {
                    auth_healthy: true,
                    structured_output: true,
                    pty_viable: true,
                    browser_tools: true,
                }
            }
        };

        Ok(CapabilityReport {
            cli_version: cli_version.clone(),
            auth_healthy: envelope.auth_healthy,
            structured_output: envelope.structured_output,
            pty_viable: envelope.pty_viable,
            browser_tools: envelope.browser_tools,
            is_canary: !self.known_versions.contains(&cli_version),
        })
    }

    fn resolve_mode(
        &self,
        skill: &SkillManifest,
        caps: &CapabilityReport,
    ) -> Result<ExecutionMode> {
        resolve_execution_mode(skill, caps)
    }

    fn build_launch_spec(&self, req: &SpawnRequest, mode: ExecutionMode) -> Result<LaunchSpec> {
        let mut args = Vec::new();
        if mode == ExecutionMode::Contract {
            args.push(OsString::from("--output-format"));
            args.push(OsString::from("stream-json"));
        }
        args.extend(req.args.clone());

        let mut env = req.env.clone();
        // Only set CLAUDE_CONFIG_DIR if the session-local config directory
        // already exists (pre-seeded with auth).  Otherwise leave the env
        // variable unset so the CLI falls through to the user's default config.
        let session_config = req.cwd.join(".claude").join("config");
        if session_config.join("auth.json").exists() {
            env.insert(
                OsString::from("CLAUDE_CONFIG_DIR"),
                session_config.into_os_string(),
            );
        }
        env.insert(
            OsString::from("CLAUDE_BROWSER_PROFILE_DIR"),
            req.cwd
                .join(".claude")
                .join("browser-profile")
                .into_os_string(),
        );
        env.insert(
            OsString::from("CLAUDE_ADAPTER_VERSION"),
            OsString::from(env!("CARGO_PKG_VERSION")),
        );
        if let Ok(system_path) = std::env::var("PATH") {
            env.insert(OsString::from("PATH"), OsString::from(system_path));
        }

        Ok(LaunchSpec {
            program: self.program.clone(),
            args,
            env,
            cwd: req.cwd.clone(),
        })
    }

    fn parser(&self, mode: ExecutionMode) -> Box<dyn AdapterParser> {
        Box::new(ClaudeParser::new(mode))
    }
}

#[async_trait]
impl AdapterParser for ClaudeParser {
    async fn on_output(&mut self, frame: &OutputFrame) -> Result<Vec<ControlEvent>> {
        match frame {
            OutputFrame::Pty(_) => {
                if self.mode != ExecutionMode::Pty {
                    bail!("{PROTOCOL_MISMATCH_CODE}: Claude contract parser received PTY bytes");
                }
                if let OutputFrame::Pty(bytes) = frame {
                    self.stdout_buffer.extend_from_slice(bytes);
                }
                let mut buffer = std::mem::take(&mut self.stdout_buffer);
                let events = self.process_buffer(&mut buffer, false)?;
                self.stdout_buffer = buffer;
                Ok(events)
            }
            OutputFrame::Stdout(bytes) => {
                self.stdout_buffer.extend_from_slice(bytes);
                let mut buffer = std::mem::take(&mut self.stdout_buffer);
                let events = self.process_buffer(&mut buffer, false)?;
                self.stdout_buffer = buffer;
                Ok(events)
            }
            OutputFrame::Stderr(bytes) => {
                self.stderr_buffer.extend_from_slice(bytes);
                let mut buffer = std::mem::take(&mut self.stderr_buffer);
                let events = self.process_buffer(&mut buffer, false)?;
                self.stderr_buffer = buffer;
                Ok(events)
            }
        }
    }

    async fn finish(&mut self) -> Result<Vec<ControlEvent>> {
        if self.finished {
            bail!("{INTERNAL_ERROR_CODE}: Claude parser finish called more than once");
        }
        self.finished = true;

        let mut stdout_buffer = std::mem::take(&mut self.stdout_buffer);
        let mut stderr_buffer = std::mem::take(&mut self.stderr_buffer);
        let mut events = self.process_buffer(&mut stdout_buffer, true)?;
        events.extend(self.process_buffer(&mut stderr_buffer, true)?);
        Ok(events)
    }
}

fn trim_line(bytes: &[u8]) -> &[u8] {
    let mut start = 0;
    let mut end = bytes.len();

    while start < end && bytes[start].is_ascii_whitespace() {
        start += 1;
    }
    while end > start && bytes[end - 1].is_ascii_whitespace() {
        end -= 1;
    }

    &bytes[start..end]
}

fn required_str<'a>(value: &'a Value, field: &str) -> Result<&'a str> {
    value
        .get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("{PROTOCOL_MISMATCH_CODE}: missing string field `{field}`"))
}

fn optional_u32(value: &Value, field: &str) -> Result<Option<u32>> {
    value
        .get(field)
        .map(|entry| {
            entry
                .as_u64()
                .ok_or_else(|| {
                    anyhow!("{PROTOCOL_MISMATCH_CODE}: invalid integer field `{field}`")
                })?
                .try_into()
                .map_err(|_| {
                    anyhow!("{PROTOCOL_MISMATCH_CODE}: out-of-range integer field `{field}`")
                })
        })
        .transpose()
}

fn required_u32(value: &Value, field: &str) -> Result<u32> {
    optional_u32(value, field)?
        .ok_or_else(|| anyhow!("{PROTOCOL_MISMATCH_CODE}: missing integer field `{field}`"))
}

fn parse_auth_kind(raw: &str) -> AuthChallengeKind {
    match raw {
        "relogin" => AuthChallengeKind::ReLogin,
        "mfa" => AuthChallengeKind::Mfa,
        "sso" => AuthChallengeKind::Sso,
        "permission_prompt" => AuthChallengeKind::PermissionPrompt,
        _ => AuthChallengeKind::Unknown,
    }
}
