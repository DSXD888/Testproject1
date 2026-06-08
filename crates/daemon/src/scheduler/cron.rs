#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::str::FromStr;

use anyhow::{Context, Result, anyhow};
use chrono::{DateTime, Duration, Utc};
use cron::Schedule;
use uuid::Uuid;

use crate::agents::arbiter::AdmissionPriority;
use crate::jobs::runner::EnqueueJobRequest;
use crate::sources::registry::{RegisteredSource, SourceJobClass, SourceRegistry};

pub const VALIDATION_CALIBRATION_SOURCE_ID: &str = "validation-calibration";
pub const VALIDATION_CALIBRATION_READINESS_SWEEP_SKILL_ID: &str =
    "validation/calibration-readiness-sweep";
pub const VALIDATION_CROSS_MODEL_AUDIT_SKILL_ID: &str = "validation/cross-model-audit-sample";
pub const VALIDATION_CALIBRATION_SKILL_VERSION: &str = "2026.05.06-1";
pub const VALIDATION_CALIBRATION_READINESS_SWEEP_CRON: &str = "0 20 2 * * *";
pub const VALIDATION_CROSS_MODEL_AUDIT_CRON: &str = "0 30 3 * * *";
pub const VALIDATION_CROSS_MODEL_ALTERNATE_MODEL_FAMILY: &str = "alternate-validator";
pub const VALIDATION_CROSS_MODEL_ACTIVE_TRAFFIC_WINDOW_DAYS: i64 = 30;
pub const COMMERCIAL_SWEEP_SOURCE_ID: &str = "commercial-sweep";
pub const COMMERCIAL_SWEEP_SKILL_ID: &str = "pipeline/commercial-sweep";
pub const COMMERCIAL_SWEEP_SKILL_VERSION: &str = "2026.06.07-1";
pub const COMMERCIAL_SWEEP_CRON: &str = "0 0 7 * * *";
pub const L5_COMMERCIAL_INTELLIGENCE_SOURCE_ID: &str = "l5-commercial-intelligence";
pub const L5_COMMERCIAL_INTELLIGENCE_SKILL_VERSION: &str = "2026.05.19-1";
pub const HMRC_TARIFF_NEWS_WEEKLY_SKILL_ID: &str = "landed-cost/hmrc-tariff-news-watch";
pub const TRA_CASE_REGISTER_DAILY_SKILL_ID: &str = "landed-cost/tra-case-register-watch";
pub const HMRC_FX_MONTHLY_ROLLOVER_SKILL_ID: &str = "landed-cost/hmrc-fx-monthly-rollover";
pub const MARKETPLACE_SOURCE_HEALTH_CANARY_SKILL_ID: &str =
    "source-health/l5-marketplace-commercial-intelligence";
pub const HMRC_TARIFF_NEWS_WEEKLY_CRON: &str = "0 15 3 * * 1";
pub const TRA_CASE_REGISTER_DAILY_CRON: &str = "0 45 2 * * *";
pub const HMRC_FX_MONTHLY_ROLLOVER_CRON: &str = "0 10 4 18-25 * *";
pub const MARKETPLACE_SOURCE_HEALTH_CANARY_CRON: &str = "0 0 5 * * *";
pub const L5_ENABLE_HMRC_TARIFF_NEWS_CRON_ENV: &str = "L5_ENABLE_HMRC_TARIFF_NEWS_CRON";
pub const L5_ENABLE_TRA_CASE_REGISTER_CRON_ENV: &str = "L5_ENABLE_TRA_CASE_REGISTER_CRON";
pub const L5_ENABLE_HMRC_FX_ROLLOVER_CRON_ENV: &str = "L5_ENABLE_HMRC_FX_ROLLOVER_CRON";
pub const L5_ENABLE_MARKETPLACE_SOURCE_HEALTH_CRON_ENV: &str =
    "L5_ENABLE_MARKETPLACE_SOURCE_HEALTH_CRON";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CronJobDefinition {
    pub source_id: String,
    pub skill_id: String,
    pub skill_version: String,
    pub cron_expression: String,
    pub enabled: bool,
    pub last_run_at: Option<DateTime<Utc>>,
    pub next_run_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CronJobClass {
    Production,
    SourceHealth,
    AuthRefresh,
    ValidationCalibration,
    CommercialSweep,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CronDispatch {
    pub definition_key: String,
    pub definition: CronJobDefinition,
    pub job_class: CronJobClass,
    pub priority: AdmissionPriority,
    pub enqueue_request: EnqueueJobRequest,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CronSuppressionReason {
    SourceCircuitHalfOpen {
        source_id: String,
    },
    SourceCircuitOpen {
        source_id: String,
    },
    SourceDefinitionDisabled {
        source_id: String,
    },
    InvalidCronExpression {
        source_id: String,
        cron_expression: String,
    },
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CronScheduler {
    definitions: BTreeMap<String, CronJobDefinition>,
}

impl CronScheduler {
    #[must_use]
    pub fn from_registry(registry: &SourceRegistry) -> Self {
        let mut definitions: BTreeMap<String, CronJobDefinition> = registry
            .sources()
            .values()
            .flat_map(|source| definitions_for_source(source).into_iter())
            .map(|definition| (definition_key(&definition), definition))
            .collect();
        for definition in scheduler_owned_definitions() {
            definitions.insert(definition_key(&definition), definition);
        }
        Self { definitions }
    }

    #[must_use]
    pub const fn definitions(&self) -> &BTreeMap<String, CronJobDefinition> {
        &self.definitions
    }

    pub fn refresh_next_run_at(&mut self, now: DateTime<Utc>) -> Result<()> {
        let cold_start_anchor = now.checked_sub_signed(Duration::seconds(1)).unwrap_or(now);
        for definition in self.definitions.values_mut() {
            let schedule = parse_schedule(definition)?;
            let anchor = definition.last_run_at.unwrap_or(cold_start_anchor);
            definition.next_run_at = schedule.after(&anchor).next();
        }
        Ok(())
    }

    pub fn dispatch_due_jobs(
        &self,
        registry: &SourceRegistry,
        now: DateTime<Utc>,
    ) -> Result<Vec<CronDispatch>> {
        let mut ready = Vec::new();
        for (definition_key, definition) in &self.definitions {
            if !definition.enabled {
                continue;
            }

            let Some(next_run_at) = definition.next_run_at else {
                continue;
            };
            if next_run_at > now {
                continue;
            }

            if requires_source_registry_lookup(&definition.source_id) {
                let registered_source = registry.get(&definition.source_id).ok_or_else(|| {
                    anyhow!("unknown source registry entry `{}`", definition.source_id)
                })?;
                if Self::suppression_reason(registered_source, definition, now).is_some() {
                    continue;
                }
            }

            let priority = admission_priority_for_skill(&definition.skill_id);
            let job_class = job_class_for_skill(&definition.skill_id);
            ready.push(CronDispatch {
                definition_key: definition_key.clone(),
                definition: definition.clone(),
                job_class,
                priority,
                enqueue_request: build_enqueue_request(definition, next_run_at, job_class),
            });
        }

        ready.sort_by(|left, right| {
            left.priority
                .cmp(&right.priority)
                .then_with(|| {
                    left.definition
                        .next_run_at
                        .cmp(&right.definition.next_run_at)
                })
                .then_with(|| left.definition.source_id.cmp(&right.definition.source_id))
        });

        Ok(ready)
    }

    pub fn suppression_reason(
        source: &RegisteredSource,
        definition: &CronJobDefinition,
        now: DateTime<Utc>,
    ) -> Option<CronSuppressionReason> {
        if !source.definition.enabled || !definition.enabled {
            return Some(CronSuppressionReason::SourceDefinitionDisabled {
                source_id: definition.source_id.clone(),
            });
        }

        let job_class = source_job_class_for_skill(&definition.skill_id);
        let allowed = source.allows_job_class(job_class, now);
        if allowed {
            return None;
        }

        match source.effective_circuit_state(now) {
            crate::sources::registry::CircuitState::HalfOpen => {
                Some(CronSuppressionReason::SourceCircuitHalfOpen {
                    source_id: definition.source_id.clone(),
                })
            }
            crate::sources::registry::CircuitState::Open => {
                Some(CronSuppressionReason::SourceCircuitOpen {
                    source_id: definition.source_id.clone(),
                })
            }
            crate::sources::registry::CircuitState::Closed => None,
        }
    }

    pub fn record_enqueued(
        &mut self,
        definition_key: &str,
        enqueued_at: DateTime<Utc>,
    ) -> Result<()> {
        let definition = self
            .definitions
            .get_mut(definition_key)
            .ok_or_else(|| anyhow!("unknown cron definition key `{definition_key}`"))?;
        let schedule = parse_schedule(definition)?;
        definition.last_run_at = Some(enqueued_at);
        definition.next_run_at = schedule.after(&enqueued_at).next();
        Ok(())
    }
}

fn scheduler_owned_definitions() -> Vec<CronJobDefinition> {
    validation_calibration_definitions()
        .into_iter()
        .chain(commercial_sweep_definitions())
        .chain(l5_commercial_intelligence_definitions())
        .collect()
}

fn definitions_for_source(source: &RegisteredSource) -> Vec<CronJobDefinition> {
    [
        source.definition.production_binding.as_ref(),
        source.definition.canary_binding.as_ref(),
        source.definition.auth_refresh_binding.as_ref(),
    ]
    .into_iter()
    .flatten()
    .filter_map(|binding| {
        binding
            .cron_expression
            .as_ref()
            .map(|cron_expression| CronJobDefinition {
                source_id: source.definition.source_id.clone(),
                skill_id: binding.skill_id.clone(),
                skill_version: binding.skill_version.clone(),
                cron_expression: cron_expression.clone(),
                enabled: source.definition.enabled && binding.enabled,
                last_run_at: None,
                next_run_at: None,
            })
    })
    .collect()
}

fn l5_commercial_intelligence_definitions() -> Vec<CronJobDefinition> {
    [
        (
            HMRC_TARIFF_NEWS_WEEKLY_SKILL_ID,
            HMRC_TARIFF_NEWS_WEEKLY_CRON,
            L5_ENABLE_HMRC_TARIFF_NEWS_CRON_ENV,
        ),
        (
            TRA_CASE_REGISTER_DAILY_SKILL_ID,
            TRA_CASE_REGISTER_DAILY_CRON,
            L5_ENABLE_TRA_CASE_REGISTER_CRON_ENV,
        ),
        (
            HMRC_FX_MONTHLY_ROLLOVER_SKILL_ID,
            HMRC_FX_MONTHLY_ROLLOVER_CRON,
            L5_ENABLE_HMRC_FX_ROLLOVER_CRON_ENV,
        ),
        (
            MARKETPLACE_SOURCE_HEALTH_CANARY_SKILL_ID,
            MARKETPLACE_SOURCE_HEALTH_CANARY_CRON,
            L5_ENABLE_MARKETPLACE_SOURCE_HEALTH_CRON_ENV,
        ),
    ]
    .into_iter()
    .map(
        |(skill_id, cron_expression, enabled_env)| CronJobDefinition {
            source_id: L5_COMMERCIAL_INTELLIGENCE_SOURCE_ID.to_owned(),
            skill_id: skill_id.to_owned(),
            skill_version: L5_COMMERCIAL_INTELLIGENCE_SKILL_VERSION.to_owned(),
            cron_expression: cron_expression.to_owned(),
            enabled: env_flag_enabled(enabled_env),
            last_run_at: None,
            next_run_at: None,
        },
    )
    .collect()
}

fn validation_calibration_definitions() -> Vec<CronJobDefinition> {
    [
        (
            VALIDATION_CALIBRATION_READINESS_SWEEP_SKILL_ID,
            VALIDATION_CALIBRATION_READINESS_SWEEP_CRON,
        ),
        (
            VALIDATION_CROSS_MODEL_AUDIT_SKILL_ID,
            VALIDATION_CROSS_MODEL_AUDIT_CRON,
        ),
    ]
    .into_iter()
    .map(|(skill_id, cron_expression)| CronJobDefinition {
        source_id: VALIDATION_CALIBRATION_SOURCE_ID.to_owned(),
        skill_id: skill_id.to_owned(),
        skill_version: VALIDATION_CALIBRATION_SKILL_VERSION.to_owned(),
        cron_expression: cron_expression.to_owned(),
        enabled: true,
        last_run_at: None,
        next_run_at: None,
    })
    .collect()
}

fn commercial_sweep_definitions() -> Vec<CronJobDefinition> {
    vec![CronJobDefinition {
        source_id: COMMERCIAL_SWEEP_SOURCE_ID.to_owned(),
        skill_id: COMMERCIAL_SWEEP_SKILL_ID.to_owned(),
        skill_version: COMMERCIAL_SWEEP_SKILL_VERSION.to_owned(),
        cron_expression: COMMERCIAL_SWEEP_CRON.to_owned(),
        enabled: true,
        last_run_at: None,
        next_run_at: None,
    }]
}

fn requires_source_registry_lookup(source_id: &str) -> bool {
    source_id != VALIDATION_CALIBRATION_SOURCE_ID
        && source_id != COMMERCIAL_SWEEP_SOURCE_ID
        && source_id != L5_COMMERCIAL_INTELLIGENCE_SOURCE_ID
}

fn env_flag_enabled(var_name: &str) -> bool {
    std::env::var(var_name).is_ok_and(|value| {
        matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        )
    })
}

fn definition_key(definition: &CronJobDefinition) -> String {
    format!("{}::{}", definition.source_id, definition.skill_id)
}

fn parse_schedule(definition: &CronJobDefinition) -> Result<Schedule> {
    Schedule::from_str(&definition.cron_expression).with_context(|| {
        format!(
            "parsing cron expression `{}` for source `{}`",
            definition.cron_expression, definition.source_id
        )
    })
}

fn build_enqueue_request(
    definition: &CronJobDefinition,
    scheduled_for: DateTime<Utc>,
    job_class: CronJobClass,
) -> EnqueueJobRequest {
    let kind = match job_class {
        CronJobClass::Production => "sync_source",
        CronJobClass::SourceHealth => "source_health",
        CronJobClass::AuthRefresh => "auth_refresh",
        CronJobClass::ValidationCalibration
            if definition.skill_id == VALIDATION_CROSS_MODEL_AUDIT_SKILL_ID =>
        {
            "validation_cross_model_audit"
        }
        CronJobClass::ValidationCalibration => "validation_calibration_readiness_sweep",
        CronJobClass::CommercialSweep => "commercial_sweep",
    };
    let (scope_kind, scope_id) = match job_class {
        CronJobClass::ValidationCalibration => (
            "validation_calibration",
            VALIDATION_CALIBRATION_SOURCE_ID.to_owned(),
        ),
        CronJobClass::CommercialSweep => ("pipeline", "commercial".to_owned()),
        CronJobClass::Production | CronJobClass::SourceHealth | CronJobClass::AuthRefresh => {
            ("source", definition.source_id.clone())
        }
    };
    let schedule_key = definition_key(definition);
    EnqueueJobRequest {
        job_id: format!("job-{}", Uuid::new_v4()),
        kind: kind.to_owned(),
        process_id: None,
        notice_id: None,
        scope_kind: Some(scope_kind.to_owned()),
        scope_id: Some(scope_id),
        parent_job_id: None,
        continuation_chain_id: None,
        validation_required: definition.skill_id == VALIDATION_CROSS_MODEL_AUDIT_SKILL_ID,
        scheduled_for: Some(scheduled_for),
        input_payload: serde_json::json!({
            "source_id": definition.source_id,
            "skill_id": definition.skill_id,
            "skill_version": definition.skill_version,
            "cron_expression": definition.cron_expression,
            "scheduled_for": scheduled_for.to_rfc3339(),
            "schedule_key": schedule_key,
            "alternate_model_family": if definition.skill_id == VALIDATION_CROSS_MODEL_AUDIT_SKILL_ID {
                Some(VALIDATION_CROSS_MODEL_ALTERNATE_MODEL_FAMILY)
            } else {
                None
            },
            "active_traffic_window_days": if definition.skill_id == VALIDATION_CROSS_MODEL_AUDIT_SKILL_ID {
                Some(VALIDATION_CROSS_MODEL_ACTIVE_TRAFFIC_WINDOW_DAYS)
            } else {
                None
            },
        }),
        idempotency_key: format!(
            "cron:{}:{}:{}",
            definition.source_id,
            definition.skill_id,
            scheduled_for.to_rfc3339()
        ),
    }
}

fn admission_priority_for_skill(skill_id: &str) -> AdmissionPriority {
    if skill_id == COMMERCIAL_SWEEP_SKILL_ID {
        AdmissionPriority::ManifestGeneration
    } else if skill_id.starts_with("validation/") {
        AdmissionPriority::Qualification
    } else if skill_id.contains("auth-refresh") || skill_id.contains("source-health") {
        AdmissionPriority::AuthRefreshOrHealthProbe
    } else if skill_id.contains("supplier") || skill_id.contains("enrichment") {
        AdmissionPriority::SupplierEnrichment
    } else if skill_id.contains("manifest") {
        AdmissionPriority::ManifestGeneration
    } else if skill_id.contains("bid") {
        AdmissionPriority::BidAutomation
    } else if skill_id.contains("qualification") {
        AdmissionPriority::Qualification
    } else {
        AdmissionPriority::ProcurementIngestion
    }
}

fn source_job_class_for_skill(skill_id: &str) -> SourceJobClass {
    if skill_id.contains("source-health") {
        SourceJobClass::SourceHealth
    } else if skill_id.contains("auth-refresh") {
        SourceJobClass::AuthRefresh
    } else {
        SourceJobClass::Production
    }
}

fn job_class_for_skill(skill_id: &str) -> CronJobClass {
    if skill_id == COMMERCIAL_SWEEP_SKILL_ID {
        return CronJobClass::CommercialSweep;
    }
    if skill_id.starts_with("validation/") {
        return CronJobClass::ValidationCalibration;
    }
    match source_job_class_for_skill(skill_id) {
        SourceJobClass::Production => CronJobClass::Production,
        SourceJobClass::SourceHealth => CronJobClass::SourceHealth,
        SourceJobClass::AuthRefresh => CronJobClass::AuthRefresh,
    }
}
