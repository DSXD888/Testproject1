#![allow(clippy::module_name_repetitions)]

use std::collections::BTreeSet;

const PIPELINE_RUN_ENTITY_KINDS: [&str; 3] =
    ["qualification_runs", "manifests", "pipeline_assessments"];

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub enum ConsistencyInvariant {
    PipelineRunRemoteRevisionProvenance,
}

impl ConsistencyInvariant {
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::PipelineRunRemoteRevisionProvenance => "pipeline_run_remote_revision_provenance",
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum AutoRepairDisposition {
    DisabledNoSafeRule,
    SafeRuleRegistered,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ConsistencySeverity {
    Error,
}

#[derive(Debug, Clone, Eq, PartialEq)]
#[allow(clippy::struct_field_names)]
pub struct RemoteRevisionProvenance {
    source_config_revision: Option<i64>,
    company_profile_revision: Option<i64>,
    operator_decision_revision: Option<i64>,
}

impl RemoteRevisionProvenance {
    #[must_use]
    pub const fn new(
        source_config_revision: Option<i64>,
        company_profile_revision: Option<i64>,
        operator_decision_revision: Option<i64>,
    ) -> Self {
        Self {
            source_config_revision,
            company_profile_revision,
            operator_decision_revision,
        }
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.source_config_revision.is_none()
            && self.company_profile_revision.is_none()
            && self.operator_decision_revision.is_none()
    }

    #[must_use]
    pub fn non_positive_revision(&self) -> Option<(&'static str, i64)> {
        [
            ("source_config_revision", self.source_config_revision),
            ("company_profile_revision", self.company_profile_revision),
            (
                "operator_decision_revision",
                self.operator_decision_revision,
            ),
        ]
        .into_iter()
        .find_map(|(field, revision)| {
            revision
                .filter(|value| *value <= 0)
                .map(|value| (field, value))
        })
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PipelineRunProvenanceObservation {
    entity_kind: String,
    entity_id: String,
    persisted: RemoteRevisionProvenance,
    outbox: RemoteRevisionProvenance,
}

impl PipelineRunProvenanceObservation {
    #[must_use]
    pub fn new(
        entity_kind: impl Into<String>,
        entity_id: impl Into<String>,
        persisted: RemoteRevisionProvenance,
        outbox: RemoteRevisionProvenance,
    ) -> Self {
        Self {
            entity_kind: entity_kind.into(),
            entity_id: entity_id.into(),
            persisted,
            outbox,
        }
    }
}

#[derive(Debug, Default, Clone, Eq, PartialEq)]
pub struct ConsistencyAuditConfig {
    safe_repair_rules: BTreeSet<ConsistencyInvariant>,
}

impl ConsistencyAuditConfig {
    pub fn register_safe_repair_rule(&mut self, invariant: ConsistencyInvariant) {
        self.safe_repair_rules.insert(invariant);
    }

    #[must_use]
    pub fn allows_safe_repair(&self, invariant: ConsistencyInvariant) -> bool {
        self.safe_repair_rules.contains(&invariant)
    }

    fn auto_repair_disposition(&self, invariant: ConsistencyInvariant) -> AutoRepairDisposition {
        if self.allows_safe_repair(invariant) {
            AutoRepairDisposition::SafeRuleRegistered
        } else {
            AutoRepairDisposition::DisabledNoSafeRule
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ConsistencyAuditFinding {
    invariant: ConsistencyInvariant,
    severity: ConsistencySeverity,
    entity_kind: String,
    entity_id: String,
    message: String,
    auto_repair: AutoRepairDisposition,
}

impl ConsistencyAuditFinding {
    #[must_use]
    pub const fn invariant(&self) -> ConsistencyInvariant {
        self.invariant
    }

    #[must_use]
    pub const fn severity(&self) -> ConsistencySeverity {
        self.severity
    }

    #[must_use]
    pub fn entity_kind(&self) -> &str {
        &self.entity_kind
    }

    #[must_use]
    pub fn entity_id(&self) -> &str {
        &self.entity_id
    }

    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    #[must_use]
    pub const fn auto_repair(&self) -> AutoRepairDisposition {
        self.auto_repair
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ConsistencyAuditReport {
    invariant: ConsistencyInvariant,
    findings: Vec<ConsistencyAuditFinding>,
}

impl ConsistencyAuditReport {
    #[must_use]
    pub const fn invariant(&self) -> ConsistencyInvariant {
        self.invariant
    }

    #[must_use]
    pub fn findings(&self) -> &[ConsistencyAuditFinding] {
        &self.findings
    }

    #[must_use]
    pub const fn is_pass(&self) -> bool {
        self.findings.is_empty()
    }
}

#[must_use]
pub fn check_pipeline_run_provenance(
    observations: &[PipelineRunProvenanceObservation],
    config: &ConsistencyAuditConfig,
) -> ConsistencyAuditReport {
    let invariant = ConsistencyInvariant::PipelineRunRemoteRevisionProvenance;
    let mut findings = Vec::new();

    for observation in observations {
        if !is_pipeline_run_entity_kind(&observation.entity_kind) {
            findings.push(finding(
                invariant,
                observation,
                format!(
                    "entity kind `{}` is not a pipeline run provenance table",
                    observation.entity_kind
                ),
                config,
            ));
        }
        if observation.persisted.is_empty() {
            findings.push(finding(
                invariant,
                observation,
                "persisted pipeline run does not record any remote revision provenance",
                config,
            ));
        }
        if observation.outbox.is_empty() {
            findings.push(finding(
                invariant,
                observation,
                "outbox payload does not record any remote revision provenance",
                config,
            ));
        }
        if let Some((field, value)) = observation.persisted.non_positive_revision() {
            findings.push(finding(
                invariant,
                observation,
                format!("persisted `{field}` must be positive, got {value}"),
                config,
            ));
        }
        if let Some((field, value)) = observation.outbox.non_positive_revision() {
            findings.push(finding(
                invariant,
                observation,
                format!("outbox `{field}` must be positive, got {value}"),
                config,
            ));
        }
        if observation.persisted != observation.outbox {
            findings.push(finding(
                invariant,
                observation,
                "persisted remote revision provenance does not match outbox payload provenance",
                config,
            ));
        }
    }

    ConsistencyAuditReport {
        invariant,
        findings,
    }
}

#[must_use]
pub fn is_pipeline_run_entity_kind(entity_kind: &str) -> bool {
    PIPELINE_RUN_ENTITY_KINDS.contains(&entity_kind)
}

fn finding(
    invariant: ConsistencyInvariant,
    observation: &PipelineRunProvenanceObservation,
    message: impl Into<String>,
    config: &ConsistencyAuditConfig,
) -> ConsistencyAuditFinding {
    ConsistencyAuditFinding {
        invariant,
        severity: ConsistencySeverity::Error,
        entity_kind: observation.entity_kind.clone(),
        entity_id: observation.entity_id.clone(),
        message: message.into(),
        auto_repair: config.auto_repair_disposition(invariant),
    }
}
