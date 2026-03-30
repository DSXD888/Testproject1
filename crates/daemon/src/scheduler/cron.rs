#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::str::FromStr;

use anyhow::{Context, Result, anyhow};
use chrono::{DateTime, Utc};
use cron::Schedule;
use uuid::Uuid;

use crate::agents::arbiter::AdmissionPriority;
use crate::jobs::runner::EnqueueJobRequest;
use crate::sources::registry::{RegisteredSource, SourceJobClass, SourceRegistry};

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
        let definitions = registry
            .sources()
            .values()
            .flat_map(|source| definitions_for_source(source).into_iter())
            .map(|definition| (definition_key(&definition), definition))
            .collect();
        Self { definitions }
    }

    #[must_use]
    pub const fn definitions(&self) -> &BTreeMap<String, CronJobDefinition> {
        &self.definitions
    }

    pub fn refresh_next_run_at(&mut self, now: DateTime<Utc>) -> Result<()> {
        for definition in self.definitions.values_mut() {
            let schedule = parse_schedule(definition)?;
            let anchor = definition.last_run_at.unwrap_or(now);
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

            let registered_source = registry.get(&definition.source_id).ok_or_else(|| {
                anyhow!("unknown source registry entry `{}`", definition.source_id)
            })?;
            if Self::suppression_reason(registered_source, definition, now).is_some() {
                continue;
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
    };
    let schedule_key = definition_key(definition);
    EnqueueJobRequest {
        job_id: format!("job-{}", Uuid::new_v4()),
        kind: kind.to_owned(),
        process_id: None,
        notice_id: None,
        scope_kind: Some("source".to_owned()),
        scope_id: Some(definition.source_id.clone()),
        parent_job_id: None,
        continuation_chain_id: None,
        validation_required: false,
        scheduled_for: Some(scheduled_for),
        input_payload: serde_json::json!({
            "source_id": definition.source_id,
            "skill_id": definition.skill_id,
            "skill_version": definition.skill_version,
            "cron_expression": definition.cron_expression,
            "scheduled_for": scheduled_for.to_rfc3339(),
            "schedule_key": schedule_key,
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
    if skill_id.contains("auth-refresh") || skill_id.contains("source-health") {
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
    match source_job_class_for_skill(skill_id) {
        SourceJobClass::Production => CronJobClass::Production,
        SourceJobClass::SourceHealth => CronJobClass::SourceHealth,
        SourceJobClass::AuthRefresh => CronJobClass::AuthRefresh,
    }
}
