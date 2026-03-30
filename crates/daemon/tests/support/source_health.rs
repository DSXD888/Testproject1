#![forbid(unsafe_code)]
#![allow(clippy::duplicate_mod)]
#![allow(dead_code, unused_imports, clippy::needless_pass_by_value)]

use anyhow::{Result, anyhow, ensure};
use chrono::{DateTime, Duration, Utc};
use rusqlite::{Connection, params};

use super::db::TestStateDb;

#[path = "../../src/sources/registry.rs"]
mod sources_impl;

pub use sources_impl::{
    CanaryResult, CircuitState, DEFAULT_BREAKER_CHALLENGE_THRESHOLD,
    DEFAULT_BREAKER_COOLDOWN_MINUTES, DEFAULT_SOURCE_HEALTH_CADENCE_MINUTES, HealthState,
    SourceRegistry, SourceRuntimeState, persist_runtime_state,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CanaryOutcomeKind {
    Passed,
    Failed(String),
    TimedOut(String),
}

impl CanaryOutcomeKind {
    const fn result(&self) -> CanaryResult {
        match self {
            Self::Passed => CanaryResult::Passed,
            Self::Failed(_) => CanaryResult::Failed,
            Self::TimedOut(_) => CanaryResult::TimedOut,
        }
    }

    const fn health_state(&self) -> HealthState {
        match self {
            Self::Passed => HealthState::Green,
            Self::Failed(_) | Self::TimedOut(_) => HealthState::Red,
        }
    }

    fn last_error_code(&self) -> Option<String> {
        match self {
            Self::Passed => None,
            Self::Failed(code) | Self::TimedOut(code) => Some(code.clone()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CircuitEvent {
    ChallengeFailure(String),
    RecoveryCanary(CanaryOutcomeKind),
    ManualReset,
}

pub struct SourceHealthFixture {
    state_db: TestStateDb,
}

impl SourceHealthFixture {
    pub fn from_fixture(name: &str) -> Result<Self> {
        let state_db = TestStateDb::new()?;
        seed_source(state_db.connection(), name, source_kind(name))?;
        Ok(Self { state_db })
    }

    pub fn seed_runtime_state(&self, source_id: &str, state: SourceRuntimeState) -> Result<()> {
        ensure!(state.source_id == source_id);
        persist_runtime_state(self.state_db.connection(), &state)?;
        Ok(())
    }

    pub fn run_canary(
        &self,
        source_id: &str,
        outcome_kind: CanaryOutcomeKind,
    ) -> Result<SourceRuntimeState> {
        let registry = SourceRegistry::load(self.state_db.connection())?;
        let registered = registry
            .get(source_id)
            .ok_or_else(|| anyhow!("missing source `{source_id}`"))?;
        let observed_at = fresh_timestamp(&registered.runtime_state);
        let next = registered
            .runtime_state
            .with_canary_outcome(
                &registered.definition,
                observed_at,
                outcome_kind.result(),
                outcome_kind.health_state(),
                outcome_kind.last_error_code(),
            )
            .ok_or_else(|| anyhow!("failed to compute canary outcome for `{source_id}`"))?;
        persist_runtime_state(self.state_db.connection(), &next)?;
        Ok(next)
    }

    #[must_use]
    pub fn connection(&self) -> &Connection {
        self.state_db.connection()
    }
}

pub struct CircuitBreakerFixture {
    state_db: TestStateDb,
}

impl CircuitBreakerFixture {
    pub fn from_fixture(name: &str) -> Result<Self> {
        let state_db = TestStateDb::new()?;
        seed_source(state_db.connection(), name, source_kind(name))?;
        Ok(Self { state_db })
    }

    pub fn transition(
        &self,
        source_id: &str,
        event: CircuitEvent,
        observed_at: DateTime<Utc>,
    ) -> Result<SourceRuntimeState> {
        let registry = SourceRegistry::load(self.state_db.connection())?;
        let registered = registry
            .get(source_id)
            .ok_or_else(|| anyhow!("missing source `{source_id}`"))?;
        let next = match event {
            CircuitEvent::ChallengeFailure(code) => registered
                .runtime_state
                .with_challenge_failure(&registered.definition, observed_at, code),
            CircuitEvent::RecoveryCanary(outcome_kind) => {
                registered.runtime_state.with_canary_outcome(
                    &registered.definition,
                    observed_at,
                    outcome_kind.result(),
                    outcome_kind.health_state(),
                    outcome_kind.last_error_code(),
                )
            }
            CircuitEvent::ManualReset => registered.runtime_state.with_manual_reset(observed_at),
        }
        .ok_or_else(|| anyhow!("failed to transition breaker state for `{source_id}`"))?;
        persist_runtime_state(self.state_db.connection(), &next)?;
        Ok(next)
    }

    pub fn assert_state(
        &self,
        source_id: &str,
        expected_state: CircuitState,
        expected_last_error_code: Option<&str>,
    ) -> Result<()> {
        let (circuit_state, last_error_code): (String, Option<String>) =
            self.state_db.connection().query_row(
                "SELECT circuit_state, last_error_code FROM source_runtime_state WHERE source_id = ?1",
                [source_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )?;
        let expected_state = match expected_state {
            CircuitState::Closed => "closed",
            CircuitState::HalfOpen => "half_open",
            CircuitState::Open => "open",
        };

        ensure!(
            circuit_state == expected_state,
            "unexpected circuit state for `{source_id}`"
        );
        ensure!(last_error_code.as_deref() == expected_last_error_code);
        Ok(())
    }
}

fn seed_source(conn: &Connection, source_id: &str, kind: &str) -> Result<()> {
    conn.execute(
        "\
        INSERT OR IGNORE INTO sources (id, kind, enabled, config)
        VALUES (?1, ?2, 1, '{}')
        ",
        params![source_id, kind],
    )?;
    Ok(())
}

fn source_kind(source_id: &str) -> &'static str {
    match source_id {
        "fts" | "contracts-finder" => "procurement",
        _ => "supplier",
    }
}

fn fresh_timestamp(runtime_state: &SourceRuntimeState) -> DateTime<Utc> {
    [
        runtime_state.last_canary_at,
        runtime_state.updated_at,
        runtime_state.circuit_opened_at,
        runtime_state.created_at,
    ]
    .into_iter()
    .flatten()
    .max()
    .unwrap_or_else(Utc::now)
        + Duration::minutes(5)
}
