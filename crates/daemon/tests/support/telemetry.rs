#![forbid(unsafe_code)]
#![allow(clippy::duplicate_mod)]
#![allow(dead_code, unused_imports, clippy::cast_possible_truncation)]

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CronDefinitionStatus {
    pub definition_key: String,
    pub source_id: String,
    pub skill_id: String,
    pub skill_version: String,
    pub cron_expression: String,
    pub enabled: bool,
    pub last_run_at: Option<String>,
    pub next_run_at: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CronWorkerLaneStatus {
    pub lane: String,
    pub running: bool,
    pub claimable_kinds: Vec<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CronAwaitingAccessStatus {
    pub active_count: i64,
    pub expired_count: i64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CronStatusSnapshot {
    pub definitions: Vec<CronDefinitionStatus>,
    pub scheduler_running: bool,
    pub last_tick_at: Option<String>,
    pub next_run_at: Option<String>,
    pub worker_lanes: Vec<CronWorkerLaneStatus>,
    pub awaiting_access: CronAwaitingAccessStatus,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SourceHealthStatusEntry {
    pub source_id: String,
    pub health_state: String,
    pub circuit_state: String,
    pub production_allowed: bool,
    pub cooldown_until: Option<String>,
    pub last_canary_at: Option<String>,
    pub last_canary_result: Option<String>,
    pub last_error_code: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SourceHealthStatusSnapshot {
    pub sources: Vec<SourceHealthStatusEntry>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ValidationStatusSnapshot {
    pub validator_approval_rate_bps: u16,
    pub validator_rejection_rate_bps: u16,
    pub validator_flag_rate_bps: u16,
    pub validator_cost_per_run_pence: u32,
    pub operator_override_rate_bps: u16,
    pub validator_execution_failure_rate_bps: u16,
    pub validator_canary_pass_rate_bps: u16,
    pub validator_health_state: String,
    pub pending_backlog_count: u32,
    pub pending_canary_count: u32,
    pub validator_job_count: u32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ResourceArbiterEventSnapshot {
    pub kind: String,
    pub job_id: String,
    pub session_id: Option<String>,
    pub priority: String,
    pub denial_reason: Option<String>,
    pub release_cause: Option<String>,
    pub browser_slots_in_use: u32,
    pub agent_slots_in_use: u32,
    pub source_lane_key: Option<String>,
    pub source_lane_in_use: Option<u32>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ResourceArbiterStatusSnapshot {
    pub browser_slots_limit: u32,
    pub browser_slots_in_use: u32,
    pub agent_slots_limit: u32,
    pub agent_slots_in_use: u32,
    pub procurement_source_lane_limit: u32,
    pub supplier_source_lane_limit: u32,
    pub source_lanes_in_use: BTreeMap<String, u32>,
    pub concurrency_in_use: BTreeMap<String, u32>,
    pub recent_events: Vec<ResourceArbiterEventSnapshot>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OperationalStatusSnapshot {
    pub cron: CronStatusSnapshot,
    pub source_health: SourceHealthStatusSnapshot,
    pub validation: ValidationStatusSnapshot,
    pub resource_arbiter: ResourceArbiterStatusSnapshot,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TelemetryProbe {
    snapshot: OperationalStatusSnapshot,
    blackout_incidents: Vec<String>,
}

impl Default for TelemetryProbe {
    fn default() -> Self {
        Self {
            snapshot: sample_operational_status(),
            blackout_incidents: Vec::new(),
        }
    }
}

impl TelemetryProbe {
    pub fn record_skill_metric(&mut self, metric: &str, value: u32) {
        match metric {
            "browser_slots_in_use" => self.snapshot.resource_arbiter.browser_slots_in_use = value,
            "agent_slots_in_use" => self.snapshot.resource_arbiter.agent_slots_in_use = value,
            "pending_backlog_count" => self.snapshot.validation.pending_backlog_count = value,
            "pending_canary_count" => self.snapshot.validation.pending_canary_count = value,
            "validator_job_count" => self.snapshot.validation.validator_job_count = value,
            _ => {}
        }
    }

    pub fn record_validation_metric(&mut self, metric: &str, value: u32) {
        match metric {
            "validator_approval_rate_bps" => {
                self.snapshot.validation.validator_approval_rate_bps = value as u16;
            }
            "validator_rejection_rate_bps" => {
                self.snapshot.validation.validator_rejection_rate_bps = value as u16;
            }
            "validator_flag_rate_bps" => {
                self.snapshot.validation.validator_flag_rate_bps = value as u16;
            }
            "operator_override_rate_bps" => {
                self.snapshot.validation.operator_override_rate_bps = value as u16;
            }
            "validator_execution_failure_rate_bps" => {
                self.snapshot
                    .validation
                    .validator_execution_failure_rate_bps = value as u16;
            }
            "validator_canary_pass_rate_bps" => {
                self.snapshot.validation.validator_canary_pass_rate_bps = value as u16;
            }
            "validator_cost_per_run_pence" => {
                self.snapshot.validation.validator_cost_per_run_pence = value;
            }
            _ => {}
        }
    }

    pub fn transition_health_state(&mut self, next_state: impl Into<String>) {
        let next_state = next_state.into();
        if self.snapshot.validation.validator_health_state != next_state && next_state == "blackout"
        {
            self.blackout_incidents
                .push("validator_blackout".to_owned());
        }
        self.snapshot.validation.validator_health_state = next_state;
    }

    #[must_use]
    pub fn blackout_incidents(&self) -> &[String] {
        &self.blackout_incidents
    }

    #[must_use]
    pub fn operational_status_snapshot(&self) -> OperationalStatusSnapshot {
        self.snapshot.clone()
    }
}

#[must_use]
pub fn sample_cron_status() -> CronStatusSnapshot {
    CronStatusSnapshot {
        definitions: vec![CronDefinitionStatus {
            definition_key: "fts::procurement-ingestion/fts".to_owned(),
            source_id: "fts".to_owned(),
            skill_id: "procurement-ingestion/fts".to_owned(),
            skill_version: "2026.03.30-1".to_owned(),
            cron_expression: "0 15 4 * * *".to_owned(),
            enabled: true,
            last_run_at: Some("2026-03-30T04:15:00Z".to_owned()),
            next_run_at: Some("2026-03-31T04:15:00Z".to_owned()),
        }],
        scheduler_running: true,
        last_tick_at: Some("2026-03-30T04:15:10Z".to_owned()),
        next_run_at: Some("2026-03-31T04:15:00Z".to_owned()),
        worker_lanes: vec![
            CronWorkerLaneStatus {
                lane: "pty_browser".to_owned(),
                running: true,
                claimable_kinds: vec![
                    "sync_source".to_owned(),
                    "source_health".to_owned(),
                    "auth_refresh".to_owned(),
                ],
            },
            CronWorkerLaneStatus {
                lane: "coverage_deterministic".to_owned(),
                running: true,
                claimable_kinds: vec![
                    "commercial_sweep".to_owned(),
                    "validation_cross_model_audit".to_owned(),
                    "validation_calibration_readiness_sweep".to_owned(),
                ],
            },
        ],
        awaiting_access: CronAwaitingAccessStatus {
            active_count: 2,
            expired_count: 1,
        },
    }
}

#[must_use]
pub fn sample_source_health_status() -> SourceHealthStatusSnapshot {
    SourceHealthStatusSnapshot {
        sources: vec![SourceHealthStatusEntry {
            source_id: "fts".to_owned(),
            health_state: "green".to_owned(),
            circuit_state: "closed".to_owned(),
            production_allowed: true,
            cooldown_until: None,
            last_canary_at: Some("2026-03-30T04:00:00Z".to_owned()),
            last_canary_result: Some("passed".to_owned()),
            last_error_code: None,
        }],
    }
}

#[must_use]
pub fn sample_validation_status() -> ValidationStatusSnapshot {
    ValidationStatusSnapshot {
        validator_approval_rate_bps: 10_000,
        validator_rejection_rate_bps: 0,
        validator_flag_rate_bps: 0,
        validator_cost_per_run_pence: 12,
        operator_override_rate_bps: 0,
        validator_execution_failure_rate_bps: 0,
        validator_canary_pass_rate_bps: 10_000,
        validator_health_state: "healthy".to_owned(),
        pending_backlog_count: 0,
        pending_canary_count: 0,
        validator_job_count: 3,
    }
}

#[must_use]
pub fn sample_resource_arbiter_status() -> ResourceArbiterStatusSnapshot {
    ResourceArbiterStatusSnapshot {
        browser_slots_limit: 2,
        browser_slots_in_use: 1,
        agent_slots_limit: 4,
        agent_slots_in_use: 2,
        procurement_source_lane_limit: 1,
        supplier_source_lane_limit: 2,
        source_lanes_in_use: BTreeMap::from([("fts".to_owned(), 1)]),
        concurrency_in_use: BTreeMap::from([("procurement-ingestion/fts".to_owned(), 1)]),
        recent_events: vec![ResourceArbiterEventSnapshot {
            kind: "grant".to_owned(),
            job_id: "job-123".to_owned(),
            session_id: Some("session-123".to_owned()),
            priority: "procurement_ingestion".to_owned(),
            denial_reason: None,
            release_cause: None,
            browser_slots_in_use: 1,
            agent_slots_in_use: 2,
            source_lane_key: Some("fts".to_owned()),
            source_lane_in_use: Some(1),
        }],
    }
}

#[must_use]
pub fn sample_operational_status() -> OperationalStatusSnapshot {
    OperationalStatusSnapshot {
        cron: sample_cron_status(),
        source_health: sample_source_health_status(),
        validation: sample_validation_status(),
        resource_arbiter: sample_resource_arbiter_status(),
    }
}
