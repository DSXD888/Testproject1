#![forbid(unsafe_code)]

use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthIssue {
    AgentStalled,
    AgentAuthChallenge,
    HostSleepInterrupted,
    HostResourceExhausted,
}

impl HealthIssue {
    pub const fn code(self) -> &'static str {
        match self {
            Self::AgentStalled => "AGENT_STALLED",
            Self::AgentAuthChallenge => "AGENT_AUTH_CHALLENGE",
            Self::HostSleepInterrupted => "HOST_SLEEP_INTERRUPTED",
            Self::HostResourceExhausted => "HOST_RESOURCE_EXHAUSTED",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StallTracker {
    no_output_timeout: Duration,
    no_progress_timeout: Duration,
    last_output_elapsed: Duration,
    last_progress_elapsed: Duration,
}

impl StallTracker {
    pub const fn new(no_output_timeout: Duration, no_progress_timeout: Duration) -> Self {
        Self {
            no_output_timeout,
            no_progress_timeout,
            last_output_elapsed: Duration::ZERO,
            last_progress_elapsed: Duration::ZERO,
        }
    }

    pub const fn record_output(&mut self, elapsed: Duration) {
        self.last_output_elapsed = elapsed;
    }

    pub const fn record_progress(&mut self, elapsed: Duration) {
        self.last_progress_elapsed = elapsed;
    }

    pub fn no_output_issue(self, now_elapsed: Duration) -> Option<HealthIssue> {
        if now_elapsed.saturating_sub(self.last_output_elapsed) >= self.no_output_timeout {
            Some(HealthIssue::AgentStalled)
        } else {
            None
        }
    }

    pub fn no_progress_issue(self, now_elapsed: Duration) -> Option<HealthIssue> {
        if now_elapsed.saturating_sub(self.last_progress_elapsed) >= self.no_progress_timeout {
            Some(HealthIssue::AgentStalled)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HostResources {
    pub cpu_pressure: bool,
    pub memory_pressure: bool,
}

pub fn detect_auth_challenge(output: &str) -> Option<HealthIssue> {
    let lowered = output.to_ascii_lowercase();
    let markers = [
        "auth challenge",
        "re-login",
        "mfa",
        "single sign-on",
        "sso",
        "permission prompt",
    ];
    if markers.iter().any(|marker| lowered.contains(marker)) {
        Some(HealthIssue::AgentAuthChallenge)
    } else {
        None
    }
}

pub fn detect_host_sleep(monotonic_gap: Duration, threshold: Duration) -> Option<HealthIssue> {
    if monotonic_gap >= threshold {
        Some(HealthIssue::HostSleepInterrupted)
    } else {
        None
    }
}

pub const fn detect_resource_pressure(resources: HostResources) -> Option<HealthIssue> {
    if resources.cpu_pressure || resources.memory_pressure {
        Some(HealthIssue::HostResourceExhausted)
    } else {
        None
    }
}

pub const fn pre_spawn_failure_phases() -> [&'static str; 4] {
    ["created", "terminating", "exited", "archived"]
}
