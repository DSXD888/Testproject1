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
const L5_SUPPLIER_PRICE_ENRICHMENT_SKILL_VERSION: &str = "2026.04.21-1";
const L5_PROCUREMENT_INGESTION_SKILL_VERSION: &str = "2026.05.19-1";
const L5_SOURCE_HEALTH_SKILL_VERSION: &str = "2026.05.19-1";
const L5_MARKETPLACE_VALIDATOR_OVERLAY_VERSION: &str =
    "validation/overlay-customs-classification@1.0.0";
const L5_MARKETPLACE_VALIDATOR_BEHAVIOR_VERSION: &str =
    "validation/overlay-customs-classification@1.0.0+1.0";
const L5_PCS_WORKER_BEHAVIOR_VERSION: &str = "procurement-ingestion/pcs@1.0.0";

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
        "pcs".to_owned(),
        SourceDefinition {
            source_id: "pcs".to_owned(),
            kind: SourceKind::Procurement,
            enabled: true,
            source_structure_version: "pcs-2026-05-19".to_owned(),
            config: json!({
                "jurisdiction": "scotland",
                "api_family": "pcs_ocds_web_api",
                "scope_decision": "in_scope_2026-05-08",
                "account_state": "public_no_account_required",
                "notice_list_endpoint": "https://api.publiccontractsscotland.gov.uk/v1/Notices",
                "notice_family_endpoint": "https://api.publiccontractsscotland.gov.uk/v1/Notice?id={ocid}",
                "validator_overlay": "validation/overlay-customs-classification",
                "worker_behavior_version": L5_PCS_WORKER_BEHAVIOR_VERSION,
                "validator_behavior_version": L5_MARKETPLACE_VALIDATOR_BEHAVIOR_VERSION,
            }),
            production_binding: Some(l5_pcs_candidate_binding()),
            canary_binding: None,
            auth_refresh_binding: None,
            policy: SourcePolicy::default(),
            default_health_state: HealthState::Unknown,
        },
    );
    definitions.insert(
        "alibaba".to_owned(),
        SourceDefinition {
            source_id: "alibaba".to_owned(),
            kind: SourceKind::Supplier,
            enabled: true,
            source_structure_version: "alibaba-2026-03-30".to_owned(),
            config: json!({
                "price_signal_role": "optimistic_floor",
                "region": "CN",
                "quote_class": "supplier_price_enrichment",
            }),
            production_binding: Some(candidate_binding(
                "supplier-price-enrichment/alibaba",
                L5_SUPPLIER_PRICE_ENRICHMENT_SKILL_VERSION,
                None,
                "supplier-price-enrichment",
                "alibaba",
            )),
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
            config: json!({
                "price_signal_role": "domestic_wholesale_floor",
                "region": "CN",
                "quote_class": "supplier_price_enrichment",
            }),
            production_binding: Some(candidate_binding(
                "supplier-price-enrichment/1688",
                L5_SUPPLIER_PRICE_ENRICHMENT_SKILL_VERSION,
                None,
                "supplier-price-enrichment",
                "1688",
            )),
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
    definitions.insert(
        "amazon".to_owned(),
        SourceDefinition {
            source_id: "amazon".to_owned(),
            kind: SourceKind::Supplier,
            enabled: true,
            source_structure_version: "amazon-2026-05-19".to_owned(),
            config: json!({
                "price_signal_role": "benchmark_ceiling",
                "region": "GB",
                "quote_class": "supplier_price_enrichment",
                "search_strategy": "browser_first",
                "pa_api_sunset": "2026-04-30",
                "pa_api_enabled": false,
                "validator_overlay": "validation/overlay-customs-classification",
                "worker_behavior_version": "supplier-price-enrichment/amazon@2.0.0",
                "validator_behavior_version": L5_MARKETPLACE_VALIDATOR_BEHAVIOR_VERSION,
            }),
            production_binding: Some(l5_marketplace_candidate_binding(
                "supplier-price-enrichment/amazon",
                L5_SUPPLIER_PRICE_ENRICHMENT_SKILL_VERSION,
                "amazon",
                "supplier-price-enrichment/amazon@2.0.0",
            )),
            canary_binding: None,
            auth_refresh_binding: None,
            policy: SourcePolicy::default(),
            default_health_state: HealthState::Unknown,
        },
    );
    definitions.insert(
        "l5-marketplace-commercial-intelligence".to_owned(),
        SourceDefinition {
            source_id: "l5-marketplace-commercial-intelligence".to_owned(),
            kind: SourceKind::Supplier,
            enabled: true,
            source_structure_version: "l5-marketplace-commercial-intelligence-2026-05-19"
                .to_owned(),
            config: json!({
                "covered_sources": [
                    "alibaba",
                    "1688",
                    "amazon",
                    "pcs",
                    "searates",
                    "freightos",
                    "taric"
                ],
                "purpose": "source_health_canary",
            }),
            production_binding: None,
            canary_binding: Some(candidate_binding(
                "source-health/l5-marketplace-commercial-intelligence",
                L5_SOURCE_HEALTH_SKILL_VERSION,
                None,
                "source-health",
                "l5-marketplace-commercial-intelligence",
            )),
            auth_refresh_binding: None,
            policy: SourcePolicy::default(),
            default_health_state: HealthState::Unknown,
        },
    );
    // Stage 9 — marketplace-search sources. Candidate-driven, not cron-driven;
    // the manual live-loop trigger assembles a batch of product_candidates
    // from search_profiles and passes them to the skill as job_parameters.
    definitions.insert(
        "gem-imports".to_owned(),
        SourceDefinition {
            source_id: "gem-imports".to_owned(),
            kind: SourceKind::Supplier,
            enabled: true,
            source_structure_version: "gem-imports-2026-04-19".to_owned(),
            config: json!({
                "price_signal_role": "uk_wholesale",
                "region": "GB",
                "quote_class": "wholesale",
            }),
            production_binding: Some(stable_binding(
                "marketplace-search/gem-imports",
                DEFAULT_SOURCE_SKILL_VERSION,
                None,
                "marketplace-search",
                "gem-imports",
            )),
            canary_binding: None,
            auth_refresh_binding: None,
            policy: SourcePolicy::default(),
            default_health_state: HealthState::Unknown,
        },
    );
    definitions.insert(
        "mx-wholesale".to_owned(),
        SourceDefinition {
            source_id: "mx-wholesale".to_owned(),
            kind: SourceKind::Supplier,
            enabled: true,
            source_structure_version: "mx-wholesale-2026-04-19".to_owned(),
            config: json!({
                "price_signal_role": "uk_wholesale",
                "region": "GB",
                "quote_class": "wholesale",
            }),
            production_binding: Some(stable_binding(
                "marketplace-search/mx-wholesale",
                DEFAULT_SOURCE_SKILL_VERSION,
                None,
                "marketplace-search",
                "mx-wholesale",
            )),
            canary_binding: None,
            auth_refresh_binding: None,
            policy: SourcePolicy::default(),
            default_health_state: HealthState::Unknown,
        },
    );
    definitions.insert(
        "amazon-uk".to_owned(),
        SourceDefinition {
            source_id: "amazon-uk".to_owned(),
            kind: SourceKind::Supplier,
            enabled: true,
            source_structure_version: "amazon-uk-2026-04-19".to_owned(),
            config: json!({
                "price_signal_role": "uk_retail_benchmark",
                "region": "GB",
                "quote_class": "spot",
            }),
            production_binding: Some(stable_binding(
                "marketplace-search/amazon-uk",
                DEFAULT_SOURCE_SKILL_VERSION,
                None,
                "marketplace-search",
                "amazon-uk",
            )),
            canary_binding: None,
            auth_refresh_binding: None,
            policy: SourcePolicy::default(),
            default_health_state: HealthState::Unknown,
        },
    );
    definitions.insert(
        "ebay-uk".to_owned(),
        SourceDefinition {
            source_id: "ebay-uk".to_owned(),
            kind: SourceKind::Supplier,
            enabled: true,
            source_structure_version: "ebay-uk-2026-04-19".to_owned(),
            config: json!({
                "price_signal_role": "uk_retail_benchmark",
                "region": "GB",
                "quote_class": "spot",
            }),
            production_binding: Some(stable_binding(
                "marketplace-search/ebay-uk",
                DEFAULT_SOURCE_SKILL_VERSION,
                None,
                "marketplace-search",
                "ebay-uk",
            )),
            canary_binding: None,
            auth_refresh_binding: None,
            policy: SourcePolicy::default(),
            default_health_state: HealthState::Unknown,
        },
    );
    definitions.insert(
        "alibaba-search".to_owned(),
        SourceDefinition {
            source_id: "alibaba-search".to_owned(),
            kind: SourceKind::Supplier,
            enabled: true,
            source_structure_version: "alibaba-search-2026-04-19".to_owned(),
            config: json!({
                "price_signal_role": "cn_supplier_floor",
                "region": "CN",
                "quote_class": "trade",
            }),
            production_binding: Some(stable_binding(
                "marketplace-search/alibaba",
                DEFAULT_SOURCE_SKILL_VERSION,
                None,
                "marketplace-search",
                "alibaba-search",
            )),
            canary_binding: None,
            auth_refresh_binding: None,
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
    binding_with_stage(
        skill_id,
        skill_version,
        cron_expression,
        gate_a_family_key,
        source_scope,
        PromotionStage::Stable,
    )
}

fn candidate_binding(
    skill_id: &str,
    skill_version: &str,
    cron_expression: Option<&str>,
    gate_a_family_key: &str,
    source_scope: &str,
) -> SkillBinding {
    binding_with_stage(
        skill_id,
        skill_version,
        cron_expression,
        gate_a_family_key,
        source_scope,
        PromotionStage::Candidate,
    )
}

fn l5_marketplace_candidate_binding(
    skill_id: &str,
    skill_version: &str,
    source_scope: &str,
    worker_behavior_version: &str,
) -> SkillBinding {
    let mut binding = candidate_binding(
        skill_id,
        skill_version,
        None,
        "supplier-price-enrichment",
        source_scope,
    );
    L5_MARKETPLACE_VALIDATOR_OVERLAY_VERSION
        .clone_into(&mut binding.version_dimensions.validator_overlay);
    worker_behavior_version.clone_into(&mut binding.behavior_versions.worker_behavior_version);
    L5_MARKETPLACE_VALIDATOR_BEHAVIOR_VERSION
        .clone_into(&mut binding.behavior_versions.validator_behavior_version);
    binding.calibration.exact_slice_key = format!(
        "{skill_id}::{source_scope}::{worker_behavior_version}::{L5_MARKETPLACE_VALIDATOR_BEHAVIOR_VERSION}"
    );
    binding
}

fn l5_pcs_candidate_binding() -> SkillBinding {
    let mut binding = candidate_binding(
        "procurement-ingestion/pcs",
        L5_PROCUREMENT_INGESTION_SKILL_VERSION,
        None,
        "procurement-ingestion",
        "pcs",
    );
    L5_MARKETPLACE_VALIDATOR_OVERLAY_VERSION
        .clone_into(&mut binding.version_dimensions.validator_overlay);
    L5_PCS_WORKER_BEHAVIOR_VERSION
        .clone_into(&mut binding.behavior_versions.worker_behavior_version);
    L5_MARKETPLACE_VALIDATOR_BEHAVIOR_VERSION
        .clone_into(&mut binding.behavior_versions.validator_behavior_version);
    binding.calibration.exact_slice_key = format!(
        "procurement-ingestion/pcs::pcs::{L5_PCS_WORKER_BEHAVIOR_VERSION}::{L5_MARKETPLACE_VALIDATOR_BEHAVIOR_VERSION}"
    );
    binding
}

fn binding_with_stage(
    skill_id: &str,
    skill_version: &str,
    cron_expression: Option<&str>,
    gate_a_family_key: &str,
    source_scope: &str,
    promotion_stage: PromotionStage,
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
        promotion_stage,
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

#[cfg(test)]
mod tests {
    use super::{PromotionStage, SkillBinding, SourceDefinition, SourceKind, SourceRegistry};
    use std::{fs, path::Path};

    #[test]
    fn l5_t_freight_engine_011_012_014_016_registry_exposes_l5_source_shapes() {
        let alibaba =
            SourceRegistry::builtin_definition("alibaba").expect("Alibaba source should exist");
        assert_alibaba_source_shape(&alibaba);

        let domestic_wholesale =
            SourceRegistry::builtin_definition("1688").expect("1688 source should exist");
        assert_1688_source_shape(&domestic_wholesale);

        let amazon = SourceRegistry::builtin_definition("amazon")
            .expect("Amazon L5 supplier source should exist");
        assert_amazon_source_shape(&amazon);

        let pcs =
            SourceRegistry::builtin_definition("pcs").expect("PCS source should be registered");
        assert_pcs_source_shape(&pcs);

        let aggregate =
            SourceRegistry::builtin_definition("l5-marketplace-commercial-intelligence")
                .expect("L5 marketplace source-health aggregate should exist");
        assert_l5_aggregate_source_shape(&aggregate);
    }

    fn production_binding(source: &SourceDefinition) -> &SkillBinding {
        source
            .production_binding
            .as_ref()
            .expect("production binding should exist for L5 maturation")
    }

    fn assert_alibaba_source_shape(alibaba: &SourceDefinition) {
        let alibaba_binding = production_binding(alibaba);
        assert_eq!(alibaba.kind, SourceKind::Supplier);
        assert_eq!(
            alibaba_binding.skill_id,
            "supplier-price-enrichment/alibaba"
        );
        assert_eq!(alibaba_binding.promotion_stage, PromotionStage::Candidate);
        assert_eq!(
            alibaba.config["price_signal_role"], "optimistic_floor",
            "L5-T-freight_engine-012 requires explicit Alibaba quote-role metadata"
        );
    }

    fn assert_1688_source_shape(domestic_wholesale: &SourceDefinition) {
        let domestic_binding = production_binding(domestic_wholesale);
        assert_eq!(domestic_binding.skill_id, "supplier-price-enrichment/1688");
        assert_eq!(domestic_binding.promotion_stage, PromotionStage::Candidate);
        assert_eq!(
            domestic_wholesale.config["price_signal_role"],
            "domestic_wholesale_floor"
        );
    }

    fn assert_amazon_source_shape(amazon: &SourceDefinition) {
        let amazon_binding = production_binding(amazon);
        assert_eq!(amazon.kind, SourceKind::Supplier);
        assert_eq!(amazon_binding.skill_id, "supplier-price-enrichment/amazon");
        assert_eq!(amazon_binding.promotion_stage, PromotionStage::Candidate);
        assert_eq!(amazon.config["price_signal_role"], "benchmark_ceiling");
        assert_eq!(amazon.config["search_strategy"], "browser_first");
        assert_eq!(amazon.config["pa_api_sunset"], "2026-04-30");
        assert_eq!(amazon.config["pa_api_enabled"].as_bool(), Some(false));
        assert_eq!(
            amazon.config["validator_overlay"],
            "validation/overlay-customs-classification"
        );
        assert_eq!(
            amazon_binding.behavior_versions.worker_behavior_version,
            "supplier-price-enrichment/amazon@2.0.0"
        );
        assert_eq!(
            amazon_binding.behavior_versions.validator_behavior_version,
            "validation/overlay-customs-classification@1.0.0+1.0"
        );
        assert!(
            amazon_binding
                .calibration
                .exact_slice_key
                .contains("supplier-price-enrichment/amazon@2.0.0"),
            "L5-T-freight_engine-011 requires the Amazon L5 behavior identity in registry"
        );
    }

    fn assert_pcs_source_shape(pcs: &SourceDefinition) {
        let pcs_binding = production_binding(pcs);
        assert_eq!(pcs.kind, SourceKind::Procurement);
        assert_eq!(pcs_binding.skill_id, "procurement-ingestion/pcs");
        assert_eq!(pcs_binding.promotion_stage, PromotionStage::Candidate);
        assert_eq!(pcs.config["scope_decision"], "in_scope_2026-05-08");
        assert_eq!(
            pcs.config["account_state"], "public_no_account_required",
            "L5-T-freight_engine-014 requires PCS to remain a public no-key OCDS source"
        );
        assert_eq!(
            pcs.config["notice_list_endpoint"],
            "https://api.publiccontractsscotland.gov.uk/v1/Notices"
        );
        assert_eq!(
            pcs_binding.behavior_versions.worker_behavior_version,
            "procurement-ingestion/pcs@1.0.0"
        );
        assert_eq!(
            pcs_binding.behavior_versions.validator_behavior_version,
            "validation/overlay-customs-classification@1.0.0+1.0"
        );
        assert!(
            pcs_binding
                .calibration
                .exact_slice_key
                .contains("procurement-ingestion/pcs@1.0.0"),
            "L5-T-freight_engine-014 requires PCS behavior identity in registry"
        );
    }

    fn assert_l5_aggregate_source_shape(aggregate: &SourceDefinition) {
        let aggregate_binding = aggregate
            .canary_binding
            .as_ref()
            .expect("L5 source-health aggregate should expose a canary binding");
        assert_eq!(
            aggregate_binding.skill_id,
            "source-health/l5-marketplace-commercial-intelligence"
        );
        assert_eq!(aggregate_binding.promotion_stage, PromotionStage::Candidate);
        assert!(
            aggregate.config["covered_sources"]
                .as_array()
                .expect("covered_sources should be an array")
                .iter()
                .any(|source| source == "pcs"),
            "L5-T-freight_engine-014 and 016 require PCS coverage in the aggregate source-health shape"
        );
    }

    #[test]
    fn l5_t_freight_engine_014_pcs_skill_artifacts_cover_scotland_ocds_canaries() {
        let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("daemon crate should be two levels below repo root");
        for relative_path in [
            "skills/procurement-ingestion/pcs/SKILL.md",
            "skills/procurement-ingestion/pcs/result.schema.json",
            "skills/procurement-ingestion/pcs/checkpoint.schema.json",
            "skills/procurement-ingestion/pcs/canary.inputs.jsonl",
            "skills/procurement-ingestion/pcs/golden.assertions.yaml",
            "skills/procurement-ingestion/pcs/validator-rubric.yaml",
            "tests/fixtures/calibration/marketplace-search/pcs/canary.inputs.jsonl",
            "tests/fixtures/calibration/marketplace-search/pcs/golden.assertions.yaml",
        ] {
            assert!(
                repo_root.join(relative_path).is_file(),
                "L5-T-freight_engine-014 requires PCS artifact `{relative_path}`"
            );
        }

        let skill_md =
            fs::read_to_string(repo_root.join("skills/procurement-ingestion/pcs/SKILL.md"))
                .expect("PCS SKILL.md should be readable");
        assert!(skill_md.contains("promotion_stage: candidate"));
        assert!(skill_md.contains("auth_requirements: none"));
        assert!(skill_md.contains("https://api.publiccontractsscotland.gov.uk/v1/Notices"));
        assert!(
            skill_md.contains("https://api.publiccontractsscotland.gov.uk/v1/Notice?id={ocid}")
        );
        assert!(!skill_md.contains("replace with live-verified"));

        let result_schema = fs::read_to_string(
            repo_root.join("skills/procurement-ingestion/pcs/result.schema.json"),
        )
        .expect("PCS result schema should be readable");
        assert!(result_schema.contains("\"const\": \"GB-SCT\""));
        assert!(result_schema.contains("public_no_account_required"));

        let canary = fs::read_to_string(
            repo_root.join("skills/procurement-ingestion/pcs/canary.inputs.jsonl"),
        )
        .expect("PCS canary inputs should be readable");
        assert!(canary.contains("l5-pcs-notice-detail-fasteners"));
        assert!(canary.contains("l5-pcs-notice-detail-cartons"));
        assert!(canary.contains("l5-pcs-notice-detail-gloves"));
        assert!(canary.contains("ocds-r6ebe6-0000750353"));
        assert!(canary.contains("ocds-r6ebe6-0000773465"));
        assert!(canary.contains("ocds-r6ebe6-0000766503"));
        assert!(canary.contains("l5-pcs-notice-list-zero-result"));
        assert!(canary.contains("l5-pcs-notice-list-stale-cassette"));
    }
}
