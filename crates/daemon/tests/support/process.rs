#![allow(dead_code)]
#![forbid(unsafe_code)]

use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, anyhow};
use nix::sys::signal::{Signal, kill};
use nix::unistd::{Pid, getpgid};

pub struct ProcessIsolationGuard {
    tracked_children: Vec<u32>,
    tracked_groups: Vec<i32>,
}

impl ProcessIsolationGuard {
    pub const fn new(_test_id: &str) -> Self {
        Self {
            tracked_children: Vec::new(),
            tracked_groups: Vec::new(),
        }
    }

    pub fn register_pid(&mut self, pid: u32) {
        self.tracked_children.push(pid);
    }

    pub fn register_pgid(&mut self, pgid: i32) {
        self.tracked_groups.push(pgid);
    }

    pub fn assert_no_survivors(&mut self) {
        for pgid in self.tracked_groups.drain(..) {
            let _ = kill(Pid::from_raw(-pgid), Signal::SIGKILL);
        }
        for pid in self.tracked_children.drain(..) {
            let _ = kill(Pid::from_raw(pid.cast_signed()), Signal::SIGKILL);
        }
    }
}

impl Drop for ProcessIsolationGuard {
    fn drop(&mut self) {
        self.assert_no_survivors();
    }
}

pub struct ProcessTreeHelper;

impl ProcessTreeHelper {
    pub fn binary_path() -> Result<PathBuf> {
        std::env::var_os("CARGO_BIN_EXE_test_agent")
            .map(PathBuf::from)
            .ok_or_else(|| anyhow!("CARGO_BIN_EXE_test_agent not set"))
    }

    pub fn command(args: &[&str]) -> Result<Command> {
        let mut command = Command::new(Self::binary_path()?);
        command.args(args);
        Ok(command)
    }

    pub fn spawn(args: &[&str]) -> Result<Child> {
        Self::command(args)?
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .with_context(|| format!("spawning test_agent with args {args:?}"))
    }

    pub fn output(args: &[&str]) -> Result<std::process::Output> {
        Self::command(args)?
            .output()
            .with_context(|| format!("running test_agent with args {args:?}"))
    }

    pub fn wait_for_exit(child: &mut Child, timeout: Duration) -> Result<std::process::ExitStatus> {
        let started = Instant::now();
        loop {
            if let Some(status) = child.try_wait().context("polling helper child")? {
                return Ok(status);
            }
            if started.elapsed() >= timeout {
                return Err(anyhow!("timed out waiting for helper child to exit"));
            }
            thread::sleep(Duration::from_millis(25));
        }
    }

    pub fn process_group(child: &Child) -> Result<i32> {
        let pid = Pid::from_raw(child.id().cast_signed());
        Ok(getpgid(Some(pid))
            .context("reading helper process group")?
            .as_raw())
    }
}
