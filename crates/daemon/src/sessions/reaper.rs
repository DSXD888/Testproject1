#![forbid(unsafe_code)]

use anyhow::{Context, Result};
use nix::errno::Errno;
use nix::sys::signal::{Signal, kill};
use nix::sys::wait::{WaitPidFlag, WaitStatus, waitpid};
use nix::unistd::Pid;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

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

impl SessionReaper {
    pub fn for_test(_state_root: &Path, artifact_root: &Path) -> Result<Self> {
        fs::create_dir_all(artifact_root)
            .with_context(|| format!("creating {}", artifact_root.display()))?;
        Ok(Self {
            artifact_root: artifact_root.to_path_buf(),
            pid_map: HashMap::new(),
            config: ReaperConfig::default(),
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

    pub fn cleanup_process_group(
        &self,
        session_id: &str,
        process_group_id: i32,
        send_pty_interrupt: bool,
    ) -> Result<CleanupOutcome> {
        let mut action_log = Vec::new();
        if send_pty_interrupt {
            action_log.push("pty-ctrl-c".to_owned());
        }

        if Self::process_group_members(process_group_id)?.is_empty() {
            return self.write_cleanup_outcome(
                session_id,
                CleanupVerdict::Clean,
                &action_log,
                &[],
                None,
            );
        }

        action_log.push("sigint".to_owned());
        let _ = kill(Pid::from_raw(-process_group_id), Signal::SIGINT);
        thread::sleep(Duration::from_millis(100));
        if Self::process_group_members(process_group_id)?.is_empty() {
            return self.write_cleanup_outcome(
                session_id,
                CleanupVerdict::Clean,
                &action_log,
                &[],
                None,
            );
        }

        action_log.push("sigterm".to_owned());
        let _ = kill(Pid::from_raw(-process_group_id), Signal::SIGTERM);
        thread::sleep(Duration::from_millis(100));
        if Self::process_group_members(process_group_id)?.is_empty() {
            return self.write_cleanup_outcome(
                session_id,
                CleanupVerdict::Forced,
                &action_log,
                &[],
                None,
            );
        }

        action_log.push("sigkill".to_owned());
        let _ = kill(Pid::from_raw(-process_group_id), Signal::SIGKILL);
        thread::sleep(Duration::from_millis(100));
        let leaked_pids = Self::process_group_members(process_group_id)?;
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
        self.write_cleanup_outcome(session_id, verdict, &action_log, &leaked_pids, error_code)
    }

    pub fn reconcile_after_crash(
        &self,
        session_id: &str,
        process_group_id: i32,
    ) -> Result<CrashRecoveryResult> {
        let _ = kill(Pid::from_raw(-process_group_id), Signal::SIGKILL);
        let manifest_dir = self.artifact_root.join("error-manifests");
        fs::create_dir_all(&manifest_dir)
            .with_context(|| format!("creating {}", manifest_dir.display()))?;
        let manifest_path = manifest_dir.join(format!("{session_id}.md"));
        fs::write(
            &manifest_path,
            "# AGENT_DAEMON_CRASH_DURING_SESSION\n\nRecovered after crash.\n",
        )
        .with_context(|| format!("writing {}", manifest_path.display()))?;
        Ok(CrashRecoveryResult {
            error_code: "AGENT_DAEMON_CRASH_DURING_SESSION".to_owned(),
            manifest_path,
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
        state.child_exit_observed
            && state.pty_drained
            && state.descendants_clean
            && state.result_validated
            && state.verdict_recorded
    }

    fn write_cleanup_outcome(
        &self,
        session_id: &str,
        cleanup_verdict: CleanupVerdict,
        action_log: &[String],
        leaked_pids: &[u32],
        error_code: Option<String>,
    ) -> Result<CleanupOutcome> {
        let session_dir = self.artifact_root.join("sessions").join(session_id);
        fs::create_dir_all(&session_dir)
            .with_context(|| format!("creating {}", session_dir.display()))?;
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
            leaked_pids: payload["leakedPids"]
                .as_array()
                .expect("leaked pids array")
                .iter()
                .filter_map(serde_json::Value::as_u64)
                .filter_map(|pid| u32::try_from(pid).ok())
                .collect(),
            report_path,
            error_code,
        })
    }
}
