#![forbid(unsafe_code)]
#![allow(clippy::duplicate_mod)]
#![allow(dead_code, unused_imports, clippy::missing_const_for_fn)]

use std::collections::BTreeSet;
use std::str::FromStr;

use anyhow::{Result, anyhow, ensure};
use chrono::{DateTime, Duration, Utc};
use cron::Schedule;
use rusqlite::{Connection, params};

use super::db::TestStateDb;

#[path = "../../src/sources/registry.rs"]
mod sources_impl;

pub use sources_impl::{CanaryResult, CircuitState};
use sources_impl::{SourceDefinition, SourceJobClass, SourceRegistry, SourceRuntimeState};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnqueuedJobSummary {
    pub definition_key: String,
    pub source_id: String,
    pub skill_id: String,
    pub skill_version: String,
    pub scheduled_for: DateTime<Utc>,
}

pub struct CronSchedulerHarness {
    now: DateTime<Utc>,
    state_db: TestStateDb,
    seeded_definition_keys: BTreeSet<String>,
}

impl CronSchedulerHarness {
    pub fn new(now: DateTime<Utc>) -> Result<Self> {
        Ok(Self {
            now,
            state_db: TestStateDb::new()?,
            seeded_definition_keys: BTreeSet::new(),
        })
    }

    pub fn seed_definition(
        &mut self,
        source_id: &str,
        skill_id: &str,
        skill_version: &str,
        cron_expression: &str,
        enabled: bool,
    ) -> Result<()> {
        seed_source(
            self.state_db.connection(),
            source_id,
            source_kind(source_id),
            enabled,
        )?;
        let registry = SourceRegistry::load(self.state_db.connection())?;
        let registered = registry
            .get(source_id)
            .ok_or_else(|| anyhow!("missing source `{source_id}`"))?;
        let binding = binding_for(&registered.definition, skill_id)
            .ok_or_else(|| anyhow!("missing binding `{skill_id}` for `{source_id}`"))?;

        ensure!(binding.skill_version == skill_version);
        ensure!(binding.cron_expression.as_deref() == Some(cron_expression));
        ensure!(registered.definition.enabled == enabled);

        self.seeded_definition_keys
            .insert(format!("{source_id}::{skill_id}"));
        Ok(())
    }

    pub fn seed_source_runtime_state(
        &self,
        source_id: &str,
        circuit_state: CircuitState,
        cooldown_until: Option<DateTime<Utc>>,
        last_canary_result: Option<CanaryResult>,
    ) -> Result<()> {
        let registry = SourceRegistry::load(self.state_db.connection())?;
        let registered = registry
            .get(source_id)
            .ok_or_else(|| anyhow!("missing source `{source_id}`"))?;
        let observed_at = self.now - Duration::minutes(5);
        let runtime_state = SourceRuntimeState {
            source_id: source_id.to_owned(),
            last_health_state: registered.runtime_state.last_health_state,
            circuit_state,
            challenge_event_count: registered.runtime_state.challenge_event_count,
            challenge_window_started_at: registered.runtime_state.challenge_window_started_at,
            circuit_opened_at: registered
                .runtime_state
                .circuit_opened_at
                .or(Some(observed_at)),
            cooldown_until,
            last_canary_at: registered.runtime_state.last_canary_at,
            last_canary_result,
            last_error_code: registered.runtime_state.last_error_code.clone(),
            updated_at: Some(observed_at),
            created_at: registered.runtime_state.created_at.or(Some(observed_at)),
        };
        sources_impl::persist_runtime_state(self.state_db.connection(), &runtime_state)?;
        Ok(())
    }

    pub fn run_due_sweep(&self) -> Result<Vec<EnqueuedJobSummary>> {
        let registry = SourceRegistry::load(self.state_db.connection())?;
        let mut jobs = Vec::new();
        for definition_key in &self.seeded_definition_keys {
            let (source_id, skill_id) = definition_key
                .split_once("::")
                .ok_or_else(|| anyhow!("invalid definition key `{definition_key}`"))?;
            let registered = registry
                .get(source_id)
                .ok_or_else(|| anyhow!("missing source `{source_id}`"))?;
            let binding = binding_for(&registered.definition, skill_id)
                .ok_or_else(|| anyhow!("missing binding `{skill_id}`"))?;
            if !registered.definition.enabled || !binding.enabled {
                continue;
            }
            if !registered.allows_job_class(job_class(skill_id), self.now) {
                continue;
            }

            let cron_expression = binding
                .cron_expression
                .as_deref()
                .ok_or_else(|| anyhow!("binding `{skill_id}` missing cron expression"))?;
            let schedule = Schedule::from_str(cron_expression)?;
            let anchor = self.now - Duration::hours(1);
            if let Some(scheduled_for) = schedule.after(&anchor).next()
                && scheduled_for <= self.now
            {
                jobs.push(EnqueuedJobSummary {
                    definition_key: definition_key.clone(),
                    source_id: source_id.to_owned(),
                    skill_id: skill_id.to_owned(),
                    skill_version: binding.skill_version.clone(),
                    scheduled_for,
                });
            }
        }
        jobs.sort_by(|left, right| left.definition_key.cmp(&right.definition_key));
        Ok(jobs)
    }

    pub fn advance_to(&mut self, instant: DateTime<Utc>) {
        self.now = instant;
    }
}

fn seed_source(conn: &Connection, source_id: &str, kind: &str, enabled: bool) -> Result<()> {
    conn.execute(
        "\
        INSERT INTO sources (id, kind, enabled, config)
        VALUES (?1, ?2, ?3, '{}')
        ON CONFLICT(id) DO UPDATE SET kind = excluded.kind, enabled = excluded.enabled, config = excluded.config
        ",
        params![source_id, kind, i64::from(enabled)],
    )?;
    Ok(())
}

fn source_kind(source_id: &str) -> &'static str {
    match source_id {
        "fts" | "contracts-finder" => "procurement",
        _ => "supplier",
    }
}

fn binding_for<'a>(
    definition: &'a SourceDefinition,
    skill_id: &str,
) -> Option<&'a sources_impl::SkillBinding> {
    [
        definition.production_binding.as_ref(),
        definition.canary_binding.as_ref(),
        definition.auth_refresh_binding.as_ref(),
    ]
    .into_iter()
    .flatten()
    .find(|binding| binding.skill_id == skill_id)
}

fn job_class(skill_id: &str) -> SourceJobClass {
    if skill_id.starts_with("source-health/") {
        SourceJobClass::SourceHealth
    } else if skill_id.starts_with("auth-refresh/") {
        SourceJobClass::AuthRefresh
    } else {
        SourceJobClass::Production
    }
}
