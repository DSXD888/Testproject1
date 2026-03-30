#![forbid(unsafe_code)]

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use nix::errno::Errno;
use nix::sys::signal::{Signal, kill};
use nix::sys::wait::{WaitPidFlag, WaitStatus, waitpid};
use nix::unistd::Pid;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

#[cfg(not(test))]
use crate::errors::manifest::{
    ErrorCode, ErrorManifestDraft, ErrorManifestStore, ErrorRuntimeContext,
};
#[cfg(test)]
use crate::errors_manifest_impl::{
    ErrorCode, ErrorManifestDraft, ErrorManifestStore, ErrorRuntimeContext,
};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CleanupVerdict {
    Clean,
    Forced,
    Leaked,
}

impl CleanupVerdict {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Clean => "clean",
            Self::Forced => "forced",
            Self::Leaked => "leaked",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ReaperConfig {
    pub terminating_grace_timeout: Duration,
    pub drain_timeout: Duration,
    pub reap_completion_budget: Duration,
    pub archive_finalization_budget: Duration,
}

impl Default for ReaperConfig {
    fn default() -> Self {
        Self {
            terminating_grace_timeout: Duration::from_secs(10),
            drain_timeout: Duration::from_secs(5),
            reap_completion_budget: Duration::from_secs(25),
            archive_finalization_budget: Duration::from_secs(10),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReapEvent {
    pub pid: u32,
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CleanupOutcome {
    pub cleanup_verdict: CleanupVerdict,
    pub action_log: Vec<String>,
    pub leaked_pids: Vec<u32>,
    pub report_path: PathBuf,
    pub error_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CrashRecoveryResult {
    pub error_code: String,
    pub manifest_path: PathBuf,
    pub cleanup_outcome: CleanupOutcome,
}

#[derive(Debug, Clone)]
pub struct CleanupRequest {
    pub session_id: String,
    pub started_at: DateTime<Utc>,
    pub process_group_id: i32,
    pub root_pid: Option<u32>,
    pub observed_descendant_pids: Vec<u32>,
    pub send_pty_interrupt: bool,
}

#[derive(Debug, Clone)]
pub struct CrashRecoveryContext<'a> {
    pub conn: &'a Connection,
    pub started_at: DateTime<Utc>,
    pub job_id: Option<&'a str>,
    pub process_id: Option<&'a str>,
    pub skill_path: Option<&'a Path>,
    pub runtime_context: ErrorRuntimeContext,
}

pub struct SessionReaper {
    artifact_root: PathBuf,
    pid_map: HashMap<u32, String>,
    config: ReaperConfig,
}

#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompletionState {
    pub child_exit_observed: bool,
    pub pty_drained: bool,
    pub descendants_clean: bool,
    pub result_validated: bool,
    pub verdict_recorded: bool,
}

impl CompletionState {
    pub const fn next_unmet_requirement(self) -> Option<&'static str> {
        if !self.child_exit_observed {
            Some("child_exit_observed")
        } else if !self.pty_drained {
            Some("pty_drained")
        } else if !self.descendants_clean {
            Some("descendants_clean")
        } else if !self.result_validated {
            Some("result_validated")
        } else if !self.verdict_recorded {
            Some("verdict_recorded")
        } else {
            None
        }
    }
}

impl SessionReaper {
    #[allow(dead_code)]
    pub fn new(artifact_root: &Path) -> Result<Self> {
        Self::with_config(artifact_root, ReaperConfig::default())
    }

    #[allow(dead_code)]
    pub fn for_test(_state_root: &Path, artifact_root: &Path) -> Result<Self> {
        Self::with_config(
            artifact_root,
            ReaperConfig {
                terminating_grace_timeout: Duration::from_millis(250),
                drain_timeout: Duration::from_millis(150),
                reap_completion_budget: Duration::from_millis(400),
                archive_finalization_budget: Duration::from_millis(150),
            },
        )
    }

    fn with_config(artifact_root: &Path, config: ReaperConfig) -> Result<Self> {
        fs::create_dir_all(artifact_root)
            .with_context(|| format!("creating {}", artifact_root.display()))?;
        Ok(Self {
            artifact_root: artifact_root.to_path_buf(),
            pid_map: HashMap::new(),
            config,
        })
    }

    pub fn register_pid(&mut self, session_id: &str, pid: u32) {
        self.pid_map.insert(pid, session_id.to_owned());
    }

    pub fn drain_waitpid(&mut self) -> Result<Vec<ReapEvent>> {
        let mut events = Vec::new();
        loop {
            match waitpid(None, Some(WaitPidFlag::WNOHANG)) {
                Ok(
                    WaitStatus::Exited(pid, _)
                    | WaitStatus::Signaled(pid, _, _)
                    | WaitStatus::Stopped(pid, _)
                    | WaitStatus::Continued(pid),
                ) => {
                    let raw_pid = pid.as_raw().cast_unsigned();
                    let session_id = self.pid_map.remove(&raw_pid);
                    events.push(ReapEvent {
                        pid: raw_pid,
                        session_id,
                    });
                }
                Ok(WaitStatus::StillAlive) | Err(Errno::ECHILD) => break,
                Err(error) => return Err(error).context("draining waitpid"),
            }
        }
        Ok(events)
    }

    pub fn cleanup_process_group(&self, request: &CleanupRequest) -> Result<CleanupOutcome> {
        let mut action_log = Vec::new();
        if request.send_pty_interrupt {
            action_log.push("pty-ctrl-c".to_owned());
        }
        let observed_descendants = Self::cleanup_targets(request)?;

        if Self::scan_survivors(request.process_group_id, &observed_descendants)?.is_empty() {
            return self.write_cleanup_outcome(
                &request.session_id,
                request.started_at,
                CleanupVerdict::Clean,
                &action_log,
                &[],
                None,
            );
        }

        action_log.push("sigint".to_owned());
        let _ = kill(Pid::from_raw(-request.process_group_id), Signal::SIGINT);
        if Self::poll_for_survivors(
            request.process_group_id,
            &observed_descendants,
            self.config.terminating_grace_timeout,
        )?
        .is_empty()
        {
            return self.write_cleanup_outcome(
                &request.session_id,
                request.started_at,
                CleanupVerdict::Clean,
                &action_log,
                &[],
                None,
            );
        }

        action_log.push("sigterm".to_owned());
        let _ = kill(Pid::from_raw(-request.process_group_id), Signal::SIGTERM);
        if Self::poll_for_survivors(
            request.process_group_id,
            &observed_descendants,
            self.config.drain_timeout,
        )?
        .is_empty()
        {
            return self.write_cleanup_outcome(
                &request.session_id,
                request.started_at,
                CleanupVerdict::Forced,
                &action_log,
                &[],
                None,
            );
        }

        action_log.push("sigkill".to_owned());
        let _ = kill(Pid::from_raw(-request.process_group_id), Signal::SIGKILL);
        let leaked_pids = Self::poll_for_survivors(
            request.process_group_id,
            &observed_descendants,
            self.config.reap_completion_budget,
        )?;
        let verdict = if leaked_pids.is_empty() {
            CleanupVerdict::Forced
        } else {
            CleanupVerdict::Leaked
        };
        let error_code = if verdict == CleanupVerdict::Leaked {
            Some("AGENT_CLEANUP_INCOMPLETE".to_owned())
        } else {
            None
        };
        self.write_cleanup_outcome(
            &request.session_id,
            request.started_at,
            verdict,
            &action_log,
            &leaked_pids,
            error_code,
        )
    }

    pub fn reconcile_after_crash(
        &self,
        session_id: &str,
        process_group_id: i32,
        root_pid: Option<u32>,
        context: &CrashRecoveryContext<'_>,
    ) -> Result<CrashRecoveryResult> {
        let cleanup_outcome = self.cleanup_process_group(&CleanupRequest {
            session_id: session_id.to_owned(),
            started_at: context.started_at,
            process_group_id,
            root_pid,
            observed_descendant_pids: Vec::new(),
            send_pty_interrupt: false,
        })?;
        let mut runtime_context = context.runtime_context.clone();
        runtime_context.cleanup_verdict = Some(cleanup_outcome.cleanup_verdict.as_str().to_owned());
        runtime_context.pgid = Some(process_group_id.unsigned_abs());
        let session_artifact_path = self.session_dir(session_id, context.started_at)?;
        let stored = ErrorManifestStore::new(&self.artifact_root)?.create(
            context.conn,
            &ErrorManifestDraft {
                job_id: context.job_id.map(str::to_owned),
                process_id: context.process_id.map(str::to_owned),
                pipeline_stage: "daemon_startup".to_owned(),
                workflow_step: "crash_recovery".to_owned(),
                error_type: ErrorCode::AgentDaemonCrashDuringSession,
                error_message:
                    "daemon restarted while the session was still active; recovery terminated the stale process group"
                        .to_owned(),
                error_context: serde_json::json!({
                    "process_group_id": process_group_id,
                    "cleanup_report_path": cleanup_outcome.report_path,
                    "leaked_pids": cleanup_outcome.leaked_pids,
                }),
                skill_path: context
                    .skill_path
                    .map(|path| path.to_string_lossy().into_owned()),
                agent_session_id: Some(session_id.to_owned()),
                runtime_context,
                session_artifact_path: Some(session_artifact_path.to_string_lossy().into_owned()),
                next_debugging_action:
                    "Inspect the recovery manifest, cleanup report, and persisted session trace before retrying the run."
                        .to_owned(),
            },
            Utc::now(),
        )?;
        Ok(CrashRecoveryResult {
            error_code: "AGENT_DAEMON_CRASH_DURING_SESSION".to_owned(),
            manifest_path: stored.markdown_path,
            cleanup_outcome,
        })
    }

    pub fn process_group_members(process_group_id: i32) -> Result<Vec<u32>> {
        let output = std::process::Command::new("ps")
            .args(["-axo", "pid=,pgid=,stat="])
            .output()
            .context("running ps for process-group scan")?;
        let stdout = String::from_utf8(output.stdout).context("decoding process-group scan")?;
        let mut members = Vec::new();
        for line in stdout.lines() {
            let columns: Vec<_> = line.split_whitespace().collect();
            if columns.len() != 3 {
                continue;
            }
            let Ok(pid) = columns[0].parse::<u32>() else {
                continue;
            };
            let Ok(parsed_process_group_id) = columns[1].parse::<i32>() else {
                continue;
            };
            let state = columns[2];
            if parsed_process_group_id == process_group_id && !state.starts_with('Z') {
                members.push(pid);
            }
        }
        Ok(members)
    }

    pub fn descendant_pids(root_pid: u32) -> Result<Vec<u32>> {
        let output = std::process::Command::new("ps")
            .args(["-axo", "pid=,ppid="])
            .output()
            .context("running ps for descendant scan")?;
        let stdout = String::from_utf8(output.stdout).context("decoding descendant scan")?;

        let mut parent_map: HashMap<u32, Vec<u32>> = HashMap::new();
        for line in stdout.lines() {
            let columns: Vec<_> = line.split_whitespace().collect();
            if columns.len() != 2 {
                continue;
            }
            let Ok(pid) = columns[0].parse::<u32>() else {
                continue;
            };
            let Ok(parent_pid) = columns[1].parse::<u32>() else {
                continue;
            };
            parent_map.entry(parent_pid).or_default().push(pid);
        }

        let mut found = Vec::new();
        let mut queue = VecDeque::from([root_pid]);
        while let Some(parent_pid) = queue.pop_front() {
            if let Some(children) = parent_map.get(&parent_pid) {
                for child_pid in children {
                    found.push(*child_pid);
                    queue.push_back(*child_pid);
                }
            }
        }
        Ok(found)
    }

    pub const fn completion_gate(state: CompletionState) -> bool {
        state.next_unmet_requirement().is_none()
    }

    fn cleanup_targets(request: &CleanupRequest) -> Result<Vec<u32>> {
        let mut observed_descendants = request.observed_descendant_pids.clone();
        observed_descendants.extend(
            request
                .root_pid
                .map(Self::descendant_pids)
                .transpose()?
                .unwrap_or_default(),
        );
        observed_descendants.sort_unstable();
        observed_descendants.dedup();
        Ok(observed_descendants)
    }

    fn poll_for_survivors(
        process_group_id: i32,
        observed_descendants: &[u32],
        timeout: Duration,
    ) -> Result<Vec<u32>> {
        let started = std::time::Instant::now();
        loop {
            let survivors = Self::scan_survivors(process_group_id, observed_descendants)?;
            if survivors.is_empty() || started.elapsed() >= timeout {
                return Ok(survivors);
            }
            std::thread::park_timeout(Duration::from_millis(25));
        }
    }

    fn scan_survivors(process_group_id: i32, observed_descendants: &[u32]) -> Result<Vec<u32>> {
        let mut survivors = Self::process_group_members(process_group_id)?;
        survivors.extend(Self::live_pids(observed_descendants)?);
        survivors.sort_unstable();
        survivors.dedup();
        Ok(survivors)
    }

    fn live_pids(candidate_pids: &[u32]) -> Result<Vec<u32>> {
        if candidate_pids.is_empty() {
            return Ok(Vec::new());
        }
        let output = std::process::Command::new("ps")
            .args(["-axo", "pid=,stat="])
            .output()
            .context("running ps for live-pid scan")?;
        let stdout = String::from_utf8(output.stdout).context("decoding live-pid scan")?;
        let mut live = Vec::new();
        for line in stdout.lines() {
            let columns: Vec<_> = line.split_whitespace().collect();
            if columns.len() != 2 {
                continue;
            }
            let Ok(pid) = columns[0].parse::<u32>() else {
                continue;
            };
            if !candidate_pids.contains(&pid) || columns[1].starts_with('Z') {
                continue;
            }
            live.push(pid);
        }
        Ok(live)
    }

    fn session_dir(&self, session_id: &str, started_at: DateTime<Utc>) -> Result<PathBuf> {
        let session_dir = self
            .artifact_root
            .join("sessions")
            .join(started_at.format("%Y").to_string())
            .join(started_at.format("%m").to_string())
            .join(session_id);
        fs::create_dir_all(&session_dir)
            .with_context(|| format!("creating {}", session_dir.display()))?;
        Ok(session_dir)
    }

    fn write_cleanup_outcome(
        &self,
        session_id: &str,
        started_at: DateTime<Utc>,
        cleanup_verdict: CleanupVerdict,
        action_log: &[String],
        leaked_pids: &[u32],
        error_code: Option<String>,
    ) -> Result<CleanupOutcome> {
        let session_dir = self.session_dir(session_id, started_at)?;
        let report_path = session_dir.join("cleanup_report.json");
        let payload = serde_json::json!({
            "cleanupVerdict": cleanup_verdict.as_str(),
            "actions": action_log,
            "leakedPids": leaked_pids,
            "errorCode": error_code,
            "budgets": {
                "terminatingGraceTimeoutMs": self.config.terminating_grace_timeout.as_millis(),
                "drainTimeoutMs": self.config.drain_timeout.as_millis(),
                "reapCompletionBudgetMs": self.config.reap_completion_budget.as_millis(),
                "archiveFinalizationBudgetMs": self.config.archive_finalization_budget.as_millis(),
            }
        });
        fs::write(
            &report_path,
            serde_json::to_vec_pretty(&payload).context("serializing cleanup report")?,
        )
        .with_context(|| format!("writing {}", report_path.display()))?;
        Ok(CleanupOutcome {
            cleanup_verdict,
            action_log: action_log.to_vec(),
            leaked_pids: leaked_pids.to_vec(),
            report_path,
            error_code,
        })
    }
}
