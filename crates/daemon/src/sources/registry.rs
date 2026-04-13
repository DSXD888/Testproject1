#![forbid(unsafe_code)]

use std::collections::BTreeMap;

use anyhow::{Context, Result, bail};
use chrono::{DateTime, Duration, NaiveDateTime, Utc};
use rusqlite::{Connection, OptionalExtension, Transaction, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

pub const DEFAULT_SOURCE_HEALTH_CADENCE_MINUTES: i64 = 30;
pub const DEFAULT_BREAKER_CHALLENGE_THRESHOLD: u32 = 3;
pub const DEFAULT_BREAKER_CHALLENGE_WINDOW_MINUTES: i64 = 60;
pub const DEFAULT_BREAKER_COOLDOWN_MINUTES: i64 = 120;
pub const DEFAULT_SOURCE_SKILL_VERSION: &str = "2026.03.30-1";
pub const DEFAULT_VALIDATOR_OVERLAY_VERSION: &str = "2026.03.30-1";
pub const DEFAULT_BINDING_RESULT_SCHEMA_VERSION: &str = "1.0.0";
pub const DEFAULT_BINDING_CHECKPOINT_SCHEMA_VERSION: &str = "1.0.0";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    Procurement,
    Supplier,
}

impl SourceKind {
    pub fn parse(raw: &str) -> Result<Self> {
        match raw {
            "procurement" => Ok(Self::Procurement),
            "supplier" => Ok(Self::Supplier),
            _ => bail!("unknown source kind `{raw}`"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthState {
    Unknown,
    Green,
    Amber,
    Red,
}

impl HealthState {
    fn parse(raw: &str) -> Result<Self> {
        match raw {
            "unknown" => Ok(Self::Unknown),
            "green" => Ok(Self::Green),
            "amber" => Ok(Self::Amber),
            "red" => Ok(Self::Red),
            _ => bail!("unknown source health state `{raw}`"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CircuitState {
    Closed,
    HalfOpen,
    Open,
}

impl CircuitState {
    fn parse(raw: &str) -> Result<Self> {
        match raw {
            "closed" => Ok(Self::Closed),
            "half_open" => Ok(Self::HalfOpen),
            "open" => Ok(Self::Open),
            _ => bail!("unknown source circuit state `{raw}`"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CanaryResult {
    Passed,
    Failed,
    TimedOut,
}

impl CanaryResult {
    fn parse(raw: &str) -> Result<Self> {
        match raw {
            "passed" => Ok(Self::Passed),
            "failed" => Ok(Self::Failed),
            "timed_out" => Ok(Self::TimedOut),
            _ => bail!("unknown canary result `{raw}`"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceJobClass {
    Production,
    SourceHealth,
    AuthRefresh,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PromotionStage {
    Draft,
    Candidate,
    Canary,
    Shadow,
    #[default]
    Stable,
    Rollback,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillVersionDimensions {
    pub skill: String,
    pub result_schema: String,
    pub checkpoint_schema: String,
    pub validator_overlay: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillBehaviorVersions {
    pub worker_behavior_version: String,
    pub validator_behavior_version: String,
    pub version_change_affects_behavior: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalibrationReadiness {
    pub family_key: String,
    pub family_prequalified: bool,
    pub exact_slice_key: String,
    pub exact_slice_ready: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillBinding {
    pub skill_id: String,
    pub skill_version: String,
    pub cron_expression: Option<String>,
    pub enabled: bool,
    pub version_dimensions: SkillVersionDimensions,
    pub promotion_stage: PromotionStage,
    pub behavior_versions: SkillBehaviorVersions,
    pub calibration: CalibrationReadiness,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourcePolicy {
    pub canary_cadence_minutes: i64,
    pub challenge_threshold: u32,
    pub challenge_window_minutes: i64,
    pub cooldown_minutes: i64,
}

impl Default for SourcePolicy {
    fn default() -> Self {
        Self {
            canary_cadence_minutes: DEFAULT_SOURCE_HEALTH_CADENCE_MINUTES,
            challenge_threshold: DEFAULT_BREAKER_CHALLENGE_THRESHOLD,
            challenge_window_minutes: DEFAULT_BREAKER_CHALLENGE_WINDOW_MINUTES,
            cooldown_minutes: DEFAULT_BREAKER_COOLDOWN_MINUTES,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceDefinition {
    pub source_id: String,
    pub kind: SourceKind,
    pub enabled: bool,
    pub source_structure_version: String,
    pub config: Value,
    pub production_binding: Option<SkillBinding>,
    pub canary_binding: Option<SkillBinding>,
    pub auth_refresh_binding: Option<SkillBinding>,
    pub policy: SourcePolicy,
    pub default_health_state: HealthState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceRuntimeState {
    pub source_id: String,
    pub last_health_state: HealthState,
    pub circuit_state: CircuitState,
    pub challenge_event_count: u32,
    pub challenge_window_started_at: Option<DateTime<Utc>>,
    pub circuit_opened_at: Option<DateTime<Utc>>,
    pub cooldown_until: Option<DateTime<Utc>>,
    pub last_canary_at: Option<DateTime<Utc>>,
    pub last_canary_result: Option<CanaryResult>,
    pub last_error_code: Option<String>,
    pub updated_at: Option<DateTime<Utc>>,
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegisteredSource {
    pub definition: SourceDefinition,
    pub runtime_state: SourceRuntimeState,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SourceRegistry {
    sources: BTreeMap<String, RegisteredSource>,
}

impl SourceRegistry {
    pub fn load(conn: &Connection) -> Result<Self> {
        let mut statement = conn.prepare(
            "\
            SELECT id, kind, enabled, config
            FROM sources
            ORDER BY id
            ",
        )?;
        let rows = statement.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, String>(3)?,
            ))
        })?;

        let mut sources = BTreeMap::new();
        for row in rows {
            let (source_id, raw_kind, enabled, config_json) = row?;
            let kind = SourceKind::parse(&raw_kind)?;
            let config = serde_json::from_str::<Value>(&config_json)
                .with_context(|| format!("parsing sources.config for {source_id}"))?;
            let definition = definition_for_row(&source_id, kind, enabled != 0, config);
            let runtime_state = load_runtime_state(conn, &source_id, &definition)?;
            sources.insert(
                source_id.clone(),
                RegisteredSource {
                    definition,
                    runtime_state,
                },
            );
        }

        Ok(Self { sources })
    }

    #[must_use]
    pub const fn sources(&self) -> &BTreeMap<String, RegisteredSource> {
        &self.sources
    }

    #[must_use]
    pub fn get(&self, source_id: &str) -> Option<&RegisteredSource> {
        self.sources.get(source_id)
    }

    #[must_use]
    pub fn builtin_definition(source_id: &str) -> Option<SourceDefinition> {
        builtin_definitions().remove(source_id)
    }
}

impl RegisteredSource {
    #[must_use]
    pub fn effective_circuit_state(&self, now: DateTime<Utc>) -> CircuitState {
        if self.runtime_state.circuit_state == CircuitState::Open
            && self
                .runtime_state
                .cooldown_until
                .is_some_and(|cooldown_until| cooldown_until <= now)
        {
            CircuitState::HalfOpen
        } else {
            self.runtime_state.circuit_state
        }
    }

    #[must_use]
    pub fn allows_job_class(&self, class: SourceJobClass, now: DateTime<Utc>) -> bool {
        match self.effective_circuit_state(now) {
            CircuitState::Closed => true,
            CircuitState::HalfOpen => {
                matches!(
                    class,
                    SourceJobClass::SourceHealth | SourceJobClass::AuthRefresh
                )
            }
            CircuitState::Open => false,
        }
    }
}

impl SourceRuntimeState {
    #[must_use]
    pub fn with_canary_outcome(
        &self,
        definition: &SourceDefinition,
        observed_at: DateTime<Utc>,
        result: CanaryResult,
        health_state: HealthState,
        last_error_code: Option<String>,
    ) -> Option<Self> {
        if self
            .freshness_timestamp()
            .is_some_and(|freshness| observed_at < freshness)
        {
            return None;
        }

        let mut next = self.clone();
        next.last_health_state = health_state;
        next.last_canary_at = Some(observed_at);
        next.last_canary_result = Some(result);
        next.last_error_code = last_error_code;
        next.updated_at = Some(observed_at);

        let is_recovery_canary = self.circuit_state == CircuitState::HalfOpen
            || (self.circuit_state == CircuitState::Open
                && self
                    .cooldown_until
                    .is_some_and(|cooldown_until| cooldown_until <= observed_at));

        match result {
            CanaryResult::Passed => {
                next.circuit_state = CircuitState::Closed;
                next.challenge_event_count = 0;
                next.challenge_window_started_at = None;
                next.circuit_opened_at = None;
                next.cooldown_until = None;
            }
            CanaryResult::Failed | CanaryResult::TimedOut => {
                if is_recovery_canary {
                    next.circuit_state = CircuitState::Open;
                    next.circuit_opened_at = Some(observed_at);
                    next.cooldown_until =
                        Some(observed_at + Duration::minutes(definition.policy.cooldown_minutes));
                } else {
                    next.circuit_state = CircuitState::HalfOpen;
                }
            }
        }

        Some(next)
    }

    #[must_use]
    pub fn with_challenge_failure(
        &self,
        definition: &SourceDefinition,
        observed_at: DateTime<Utc>,
        last_error_code: impl Into<String>,
    ) -> Option<Self> {
        if self
            .freshness_timestamp()
            .is_some_and(|freshness| observed_at < freshness)
        {
            return None;
        }

        let mut next = self.clone();
        let challenge_window = Duration::minutes(definition.policy.challenge_window_minutes);
        let reset_window = next
            .challenge_window_started_at
            .is_none_or(|started_at| observed_at - started_at > challenge_window);

        if reset_window {
            next.challenge_window_started_at = Some(observed_at);
            next.challenge_event_count = 1;
        } else {
            next.challenge_event_count = next.challenge_event_count.saturating_add(1);
        }

        next.last_error_code = Some(last_error_code.into());
        next.updated_at = Some(observed_at);

        if next.challenge_event_count >= definition.policy.challenge_threshold {
            next.circuit_state = CircuitState::Open;
            next.last_health_state = HealthState::Red;
            next.circuit_opened_at = Some(observed_at);
            next.cooldown_until =
                Some(observed_at + Duration::minutes(definition.policy.cooldown_minutes));
        }

        Some(next)
    }

    #[must_use]
    pub fn with_manual_reset(&self, observed_at: DateTime<Utc>) -> Option<Self> {
        if self
            .freshness_timestamp()
            .is_some_and(|freshness| observed_at < freshness)
        {
            return None;
        }

        let mut next = self.clone();
        next.circuit_state = CircuitState::HalfOpen;
        next.cooldown_until = None;
        next.updated_at = Some(observed_at);
        Some(next)
    }

    fn freshness_timestamp(&self) -> Option<DateTime<Utc>> {
        self.last_canary_at
            .or(self.updated_at)
            .or(self.circuit_opened_at)
            .or(self.created_at)
    }
}

pub fn persist_runtime_state(
    conn: &Connection,
    runtime_state: &SourceRuntimeState,
) -> Result<bool> {
    let tx = Transaction::new_unchecked(conn, TransactionBehavior::Immediate)
        .context("starting source registry runtime-state transaction")?;
    let current = load_runtime_state_row(&tx, &runtime_state.source_id)?;
    if current
        .as_ref()
        .and_then(SourceRuntimeState::freshness_timestamp)
        .zip(runtime_state.freshness_timestamp())
        .is_some_and(|(current, next)| next < current)
    {
        tx.commit()
            .context("committing no-op source registry runtime-state transaction")?;
        return Ok(false);
    }

    tx.execute(
        "\
        INSERT INTO source_runtime_state (
          source_id, last_health_state, circuit_state, challenge_event_count,
          challenge_window_started_at, circuit_opened_at, cooldown_until,
          last_canary_at, last_canary_result, last_error_code, updated_at, created_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, COALESCE(?11, CURRENT_TIMESTAMP), COALESCE(?12, CURRENT_TIMESTAMP))
        ON CONFLICT(source_id) DO UPDATE SET
          last_health_state = excluded.last_health_state,
          circuit_state = excluded.circuit_state,
          challenge_event_count = excluded.challenge_event_count,
          challenge_window_started_at = excluded.challenge_window_started_at,
          circuit_opened_at = excluded.circuit_opened_at,
          cooldown_until = excluded.cooldown_until,
          last_canary_at = excluded.last_canary_at,
          last_canary_result = excluded.last_canary_result,
          last_error_code = excluded.last_error_code,
          updated_at = excluded.updated_at,
          created_at = COALESCE(source_runtime_state.created_at, excluded.created_at)
        ",
        params![
            runtime_state.source_id,
            runtime_state.last_health_state.as_str(),
            runtime_state.circuit_state.as_str(),
            i64::from(runtime_state.challenge_event_count),
            runtime_state.challenge_window_started_at.map(|value| value.to_rfc3339()),
            runtime_state.circuit_opened_at.map(|value| value.to_rfc3339()),
            runtime_state.cooldown_until.map(|value| value.to_rfc3339()),
            runtime_state.last_canary_at.map(|value| value.to_rfc3339()),
            runtime_state.last_canary_result.map(CanaryResult::as_str),
            runtime_state.last_error_code,
            runtime_state.updated_at.map(|value| value.to_rfc3339()),
            runtime_state.created_at.map(|value| value.to_rfc3339()),
        ],
    )
    .with_context(|| format!("persisting source runtime state for {}", runtime_state.source_id))?;

    tx.commit()
        .context("committing source registry runtime-state transaction")?;
    Ok(true)
}

fn load_runtime_state(
    conn: &Connection,
    source_id: &str,
    definition: &SourceDefinition,
) -> Result<SourceRuntimeState> {
    Ok(load_runtime_state_row(conn, source_id)?
        .unwrap_or_else(|| default_runtime_state(source_id, definition.default_health_state)))
}

fn load_runtime_state_row(
    conn: &Connection,
    source_id: &str,
) -> Result<Option<SourceRuntimeState>> {
    conn.query_row(
        "\
        SELECT source_id, last_health_state, circuit_state, challenge_event_count,
               challenge_window_started_at, circuit_opened_at, cooldown_until,
               last_canary_at, last_canary_result, last_error_code, updated_at, created_at
        FROM source_runtime_state
        WHERE source_id = ?1
        ",
        [source_id],
        |row| {
            Ok(SourceRuntimeState {
                source_id: row.get::<_, String>(0)?,
                last_health_state: HealthState::parse(&row.get::<_, String>(1)?)
                    .map_err(to_sql_error)?,
                circuit_state: CircuitState::parse(&row.get::<_, String>(2)?)
                    .map_err(to_sql_error)?,
                challenge_event_count: u32::try_from(row.get::<_, i64>(3)?)
                    .map_err(to_sql_error)?,
                challenge_window_started_at: parse_optional_timestamp(row.get(4)?)
                    .map_err(to_sql_error)?,
                circuit_opened_at: parse_optional_timestamp(row.get(5)?).map_err(to_sql_error)?,
                cooldown_until: parse_optional_timestamp(row.get(6)?).map_err(to_sql_error)?,
                last_canary_at: parse_optional_timestamp(row.get(7)?).map_err(to_sql_error)?,
                last_canary_result: row
                    .get::<_, Option<String>>(8)?
                    .map(|value| CanaryResult::parse(&value))
                    .transpose()
                    .map_err(to_sql_error)?,
                last_error_code: row.get(9)?,
                updated_at: parse_optional_timestamp(row.get(10)?).map_err(to_sql_error)?,
                created_at: parse_optional_timestamp(row.get(11)?).map_err(to_sql_error)?,
            })
        },
    )
    .optional()
    .context("loading source runtime state")
}

fn default_runtime_state(source_id: &str, default_health_state: HealthState) -> SourceRuntimeState {
    SourceRuntimeState {
        source_id: source_id.to_owned(),
        last_health_state: default_health_state,
        circuit_state: CircuitState::Closed,
        challenge_event_count: 0,
        challenge_window_started_at: None,
        circuit_opened_at: None,
        cooldown_until: None,
        last_canary_at: None,
        last_canary_result: None,
        last_error_code: None,
        updated_at: None,
        created_at: None,
    }
}

fn parse_optional_timestamp(raw: Option<String>) -> Result<Option<DateTime<Utc>>> {
    raw.map(|value| parse_timestamp(&value)).transpose()
}

fn parse_timestamp(raw: &str) -> Result<DateTime<Utc>> {
    if let Ok(timestamp) = DateTime::parse_from_rfc3339(raw) {
        return Ok(timestamp.with_timezone(&Utc));
    }

    let naive = NaiveDateTime::parse_from_str(raw, "%Y-%m-%d %H:%M:%S")
        .with_context(|| format!("parsing timestamp `{raw}`"))?;
    Ok(DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc))
}

fn to_sql_error(error: impl std::fmt::Display) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(
        0,
        rusqlite::types::Type::Text,
        Box::new(std::io::Error::other(error.to_string())),
    )
}

#[allow(clippy::too_many_lines)]
fn builtin_definitions() -> BTreeMap<String, SourceDefinition> {
    let mut definitions = BTreeMap::new();
    definitions.insert(
        "fts".to_owned(),
        SourceDefinition {
            source_id: "fts".to_owned(),
            kind: SourceKind::Procurement,
            enabled: true,
            source_structure_version: "fts-2026-03-30".to_owned(),
            config: json!({}),
            production_binding: Some(stable_binding(
                "procurement-ingestion/fts",
                DEFAULT_SOURCE_SKILL_VERSION,
                Some("0 15 4 * * *"),
                "procurement-ingestion",
                "fts",
            )),
            canary_binding: Some(stable_binding(
                "source-health/fts",
                DEFAULT_SOURCE_SKILL_VERSION,
                Some("0 */30 * * * *"),
                "source-health",
                "fts",
            )),
            auth_refresh_binding: None,
            policy: SourcePolicy::default(),
            default_health_state: HealthState::Green,
        },
    );
    definitions.insert(
        "contracts-finder".to_owned(),
        SourceDefinition {
            source_id: "contracts-finder".to_owned(),
            kind: SourceKind::Procurement,
            enabled: true,
            source_structure_version: "contracts-finder-2026-03-30".to_owned(),
            config: json!({}),
            production_binding: Some(stable_binding(
                "procurement-ingestion/contracts-finder",
                DEFAULT_SOURCE_SKILL_VERSION,
                Some("0 45 4 * * *"),
                "procurement-ingestion",
                "contracts-finder",
            )),
            canary_binding: None,
            auth_refresh_binding: None,
            policy: SourcePolicy::default(),
            default_health_state: HealthState::Green,
        },
    );
    definitions.insert(
        "procontract".to_owned(),
        SourceDefinition {
            source_id: "procontract".to_owned(),
            kind: SourceKind::Procurement,
            enabled: true,
            source_structure_version: "procontract-2026-04-13".to_owned(),
            config: json!({}),
            production_binding: Some(stable_binding(
                "procurement-ingestion/procontract-discover",
                DEFAULT_SOURCE_SKILL_VERSION,
                Some("0 0 5 * * *"),
                "procurement-ingestion",
                "procontract",
            )),
            canary_binding: None,
            auth_refresh_binding: None,
            policy: SourcePolicy::default(),
            default_health_state: HealthState::Green,
        },
    );
    definitions.insert(
        "procontract-interest".to_owned(),
        SourceDefinition {
            source_id: "procontract-interest".to_owned(),
            kind: SourceKind::Procurement,
            enabled: true,
            source_structure_version: "procontract-interest-2026-04-13".to_owned(),
            config: json!({}),
            production_binding: Some(stable_binding(
                "procurement-ingestion/procontract-register-interest",
                DEFAULT_SOURCE_SKILL_VERSION,
                Some("0 30 5 * * *"),
                "procurement-ingestion",
                "procontract-interest",
            )),
            canary_binding: None,
            auth_refresh_binding: None,
            policy: SourcePolicy::default(),
            default_health_state: HealthState::Green,
        },
    );
    definitions.insert(
        "procontract-enrich".to_owned(),
        SourceDefinition {
            source_id: "procontract-enrich".to_owned(),
            kind: SourceKind::Procurement,
            enabled: true,
            source_structure_version: "procontract-enrich-2026-04-13".to_owned(),
            config: json!({}),
            production_binding: Some(stable_binding(
                "procurement-ingestion/procontract-enrich-tender",
                DEFAULT_SOURCE_SKILL_VERSION,
                Some("0 0 6 * * *"),
                "procurement-ingestion",
                "procontract-enrich",
            )),
            canary_binding: None,
            auth_refresh_binding: None,
            policy: SourcePolicy::default(),
            default_health_state: HealthState::Green,
        },
    );
    definitions.insert(
        "alibaba".to_owned(),
        SourceDefinition {
            source_id: "alibaba".to_owned(),
            kind: SourceKind::Supplier,
            enabled: true,
            source_structure_version: "alibaba-2026-03-30".to_owned(),
            config: json!({}),
            production_binding: None,
            canary_binding: Some(stable_binding(
                "source-health/alibaba",
                DEFAULT_SOURCE_SKILL_VERSION,
                Some("0 */30 * * * *"),
                "source-health",
                "alibaba",
            )),
            auth_refresh_binding: Some(stable_binding(
                "auth-refresh/alibaba",
                DEFAULT_SOURCE_SKILL_VERSION,
                None,
                "auth-refresh",
                "alibaba",
            )),
            policy: SourcePolicy::default(),
            default_health_state: HealthState::Unknown,
        },
    );
    definitions.insert(
        "1688".to_owned(),
        SourceDefinition {
            source_id: "1688".to_owned(),
            kind: SourceKind::Supplier,
            enabled: true,
            source_structure_version: "1688-2026-03-30".to_owned(),
            config: json!({}),
            production_binding: None,
            canary_binding: None,
            auth_refresh_binding: Some(stable_binding(
                "auth-refresh/1688",
                DEFAULT_SOURCE_SKILL_VERSION,
                None,
                "auth-refresh",
                "1688",
            )),
            policy: SourcePolicy::default(),
            default_health_state: HealthState::Unknown,
        },
    );
    definitions
}

fn definition_for_row(
    source_id: &str,
    kind: SourceKind,
    enabled: bool,
    config: Value,
) -> SourceDefinition {
    let mut definition =
        SourceRegistry::builtin_definition(source_id).unwrap_or_else(|| SourceDefinition {
            source_id: source_id.to_owned(),
            kind,
            enabled,
            source_structure_version: format!("{source_id}-untracked"),
            config: json!({}),
            production_binding: None,
            canary_binding: None,
            auth_refresh_binding: None,
            policy: SourcePolicy::default(),
            default_health_state: HealthState::Unknown,
        });
    definition.kind = kind;
    definition.enabled = enabled;
    definition.config = config;
    definition
}

fn stable_binding(
    skill_id: &str,
    skill_version: &str,
    cron_expression: Option<&str>,
    gate_a_family_key: &str,
    source_scope: &str,
) -> SkillBinding {
    let worker_behavior_version =
        format!("{skill_id}@{skill_version}+{DEFAULT_BINDING_RESULT_SCHEMA_VERSION}");
    let validator_behavior_version = DEFAULT_VALIDATOR_OVERLAY_VERSION.to_owned();
    SkillBinding {
        skill_id: skill_id.to_owned(),
        skill_version: skill_version.to_owned(),
        cron_expression: cron_expression.map(str::to_owned),
        enabled: true,
        version_dimensions: SkillVersionDimensions {
            skill: skill_version.to_owned(),
            result_schema: DEFAULT_BINDING_RESULT_SCHEMA_VERSION.to_owned(),
            checkpoint_schema: DEFAULT_BINDING_CHECKPOINT_SCHEMA_VERSION.to_owned(),
            validator_overlay: DEFAULT_VALIDATOR_OVERLAY_VERSION.to_owned(),
        },
        promotion_stage: PromotionStage::Stable,
        behavior_versions: SkillBehaviorVersions {
            worker_behavior_version: worker_behavior_version.clone(),
            validator_behavior_version: validator_behavior_version.clone(),
            version_change_affects_behavior: true,
        },
        calibration: CalibrationReadiness {
            family_key: gate_a_family_key.to_owned(),
            family_prequalified: false,
            exact_slice_key: format!(
                "{skill_id}::{source_scope}::{worker_behavior_version}::{validator_behavior_version}"
            ),
            exact_slice_ready: false,
        },
    }
}

impl HealthState {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::Green => "green",
            Self::Amber => "amber",
            Self::Red => "red",
        }
    }
}

impl CircuitState {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Closed => "closed",
            Self::HalfOpen => "half_open",
            Self::Open => "open",
        }
    }
}

impl CanaryResult {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Passed => "passed",
            Self::Failed => "failed",
            Self::TimedOut => "timed_out",
        }
    }
}
