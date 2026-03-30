#![forbid(unsafe_code)]

use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResourceBudgets {
    pub browser_slots: u32,
    pub agent_slots: u32,
    pub procurement_source_lanes: u32,
    pub supplier_source_lanes: u32,
}

impl Default for ResourceBudgets {
    fn default() -> Self {
        Self {
            browser_slots: 2,
            agent_slots: 4,
            procurement_source_lanes: 1,
            supplier_source_lanes: 2,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceKind {
    Procurement,
    Supplier,
}

impl SourceKind {
    #[must_use]
    pub const fn lane_limit(self, budgets: ResourceBudgets) -> u32 {
        match self {
            Self::Procurement => budgets.procurement_source_lanes,
            Self::Supplier => budgets.supplier_source_lanes,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AdmissionPriority {
    ProcurementIngestion,
    AuthRefreshOrHealthProbe,
    Qualification,
    SupplierEnrichment,
    ManifestGeneration,
    BidAutomation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceLaneRequest {
    pub source_id: String,
    pub source_kind: SourceKind,
}

#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmissionRequest {
    pub job_id: String,
    pub session_id: Option<String>,
    pub priority: AdmissionPriority,
    pub browser_session: bool,
    pub adapter_session: bool,
    pub source_lane: Option<SourceLaneRequest>,
    pub validator_sub_job: bool,
    pub concurrency_key: Option<String>,
    pub max_concurrency: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceLease {
    pub job_id: String,
    pub session_id: Option<String>,
    pub priority: AdmissionPriority,
    pub browser_slots: u32,
    pub agent_slots: u32,
    pub source_lane_key: Option<String>,
    pub concurrency_key: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DenialReason {
    BrowserSlotsExhausted {
        limit: u32,
        in_use: u32,
    },
    AgentSlotsExhausted {
        limit: u32,
        in_use: u32,
    },
    SourceLaneExhausted {
        source_id: String,
        limit: u32,
        in_use: u32,
    },
    SkillConcurrencyCapReached {
        key: String,
        limit: u32,
        in_use: u32,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArbiterDecision {
    Granted(ResourceLease),
    Denied(DenialReason),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReleaseCause {
    PreSpawnFailure,
    Reaped,
    Archived,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArbiterEventKind {
    Grant,
    Deny,
    Release,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArbiterEvent {
    pub kind: ArbiterEventKind,
    pub job_id: String,
    pub session_id: Option<String>,
    pub priority: AdmissionPriority,
    pub denial_reason: Option<DenialReason>,
    pub release_cause: Option<ReleaseCause>,
    pub browser_slots_in_use: u32,
    pub agent_slots_in_use: u32,
    pub source_lane_key: Option<String>,
    pub source_lane_in_use: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArbiterSnapshot {
    pub browser_slots: u32,
    pub agent_slots: u32,
    pub source_lanes: BTreeMap<String, u32>,
    pub concurrency: BTreeMap<String, u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct EffectiveDemand {
    browser_slots: u32,
    agent_slots: u32,
}

#[derive(Debug, Clone)]
pub struct ResourceArbiter {
    budgets: ResourceBudgets,
    browser_slots_in_use: u32,
    agent_slots_in_use: u32,
    source_lanes_in_use: BTreeMap<String, u32>,
    concurrency_in_use: BTreeMap<String, u32>,
    events: Vec<ArbiterEvent>,
}

impl ResourceArbiter {
    #[must_use]
    pub const fn new(budgets: ResourceBudgets) -> Self {
        Self {
            budgets,
            browser_slots_in_use: 0,
            agent_slots_in_use: 0,
            source_lanes_in_use: BTreeMap::new(),
            concurrency_in_use: BTreeMap::new(),
            events: Vec::new(),
        }
    }

    #[must_use]
    pub const fn budgets(&self) -> ResourceBudgets {
        self.budgets
    }

    #[must_use]
    pub fn snapshot(&self) -> ArbiterSnapshot {
        ArbiterSnapshot {
            browser_slots: self.browser_slots_in_use,
            agent_slots: self.agent_slots_in_use,
            source_lanes: self.source_lanes_in_use.clone(),
            concurrency: self.concurrency_in_use.clone(),
        }
    }

    #[must_use]
    pub fn events(&self) -> &[ArbiterEvent] {
        &self.events
    }

    pub fn admit(&mut self, request: &AdmissionRequest) -> ArbiterDecision {
        if let Some(reason) = self.deny_reason(request) {
            self.push_event(request, ArbiterEventKind::Deny, Some(reason.clone()), None);
            return ArbiterDecision::Denied(reason);
        }

        let demand = request.effective_demand();
        self.browser_slots_in_use += demand.browser_slots;
        self.agent_slots_in_use += demand.agent_slots;

        if let Some(source_lane) = request.effective_source_lane() {
            *self
                .source_lanes_in_use
                .entry(source_lane.source_id.clone())
                .or_default() += 1;
        }

        if let (Some(key), Some(_)) = (&request.concurrency_key, request.max_concurrency) {
            *self.concurrency_in_use.entry(key.clone()).or_default() += 1;
        }

        let lease = ResourceLease {
            job_id: request.job_id.clone(),
            session_id: request.session_id.clone(),
            priority: request.priority,
            browser_slots: demand.browser_slots,
            agent_slots: demand.agent_slots,
            source_lane_key: request
                .effective_source_lane()
                .map(|source_lane| source_lane.source_id.clone()),
            concurrency_key: request.concurrency_key.clone(),
        };

        self.push_event(request, ArbiterEventKind::Grant, None, None);
        ArbiterDecision::Granted(lease)
    }

    pub fn release(&mut self, lease: &ResourceLease, cause: ReleaseCause) {
        self.browser_slots_in_use = self
            .browser_slots_in_use
            .saturating_sub(lease.browser_slots);
        self.agent_slots_in_use = self.agent_slots_in_use.saturating_sub(lease.agent_slots);

        if let Some(source_lane_key) = &lease.source_lane_key
            && let Some(in_use) = self.source_lanes_in_use.get_mut(source_lane_key)
        {
            *in_use = in_use.saturating_sub(1);
            if *in_use == 0 {
                self.source_lanes_in_use.remove(source_lane_key);
            }
        }

        if let Some(concurrency_key) = &lease.concurrency_key
            && let Some(in_use) = self.concurrency_in_use.get_mut(concurrency_key)
        {
            *in_use = in_use.saturating_sub(1);
            if *in_use == 0 {
                self.concurrency_in_use.remove(concurrency_key);
            }
        }

        self.events.push(ArbiterEvent {
            kind: ArbiterEventKind::Release,
            job_id: lease.job_id.clone(),
            session_id: lease.session_id.clone(),
            priority: lease.priority,
            denial_reason: None,
            release_cause: Some(cause),
            browser_slots_in_use: self.browser_slots_in_use,
            agent_slots_in_use: self.agent_slots_in_use,
            source_lane_key: lease.source_lane_key.clone(),
            source_lane_in_use: lease
                .source_lane_key
                .as_ref()
                .and_then(|key| self.source_lanes_in_use.get(key).copied()),
        });
    }

    fn deny_reason(&self, request: &AdmissionRequest) -> Option<DenialReason> {
        if let (Some(key), Some(limit)) = (&request.concurrency_key, request.max_concurrency) {
            let in_use = self
                .concurrency_in_use
                .get(key)
                .copied()
                .unwrap_or_default();
            if in_use >= limit {
                return Some(DenialReason::SkillConcurrencyCapReached {
                    key: key.clone(),
                    limit,
                    in_use,
                });
            }
        }

        let demand = request.effective_demand();
        if demand.browser_slots > 0
            && self.browser_slots_in_use + demand.browser_slots > self.budgets.browser_slots
        {
            return Some(DenialReason::BrowserSlotsExhausted {
                limit: self.budgets.browser_slots,
                in_use: self.browser_slots_in_use,
            });
        }

        if demand.agent_slots > 0
            && self.agent_slots_in_use + demand.agent_slots > self.budgets.agent_slots
        {
            return Some(DenialReason::AgentSlotsExhausted {
                limit: self.budgets.agent_slots,
                in_use: self.agent_slots_in_use,
            });
        }

        if let Some(source_lane) = request.effective_source_lane() {
            let in_use = self
                .source_lanes_in_use
                .get(&source_lane.source_id)
                .copied()
                .unwrap_or_default();
            let limit = source_lane.source_kind.lane_limit(self.budgets);
            if in_use >= limit {
                return Some(DenialReason::SourceLaneExhausted {
                    source_id: source_lane.source_id.clone(),
                    limit,
                    in_use,
                });
            }
        }

        None
    }

    fn push_event(
        &mut self,
        request: &AdmissionRequest,
        kind: ArbiterEventKind,
        denial_reason: Option<DenialReason>,
        release_cause: Option<ReleaseCause>,
    ) {
        let source_lane_key = request
            .effective_source_lane()
            .map(|source_lane| source_lane.source_id.clone());
        let source_lane_in_use = source_lane_key
            .as_ref()
            .and_then(|key| self.source_lanes_in_use.get(key).copied());

        self.events.push(ArbiterEvent {
            kind,
            job_id: request.job_id.clone(),
            session_id: request.session_id.clone(),
            priority: request.priority,
            denial_reason,
            release_cause,
            browser_slots_in_use: self.browser_slots_in_use,
            agent_slots_in_use: self.agent_slots_in_use,
            source_lane_key,
            source_lane_in_use,
        });
    }
}

impl AdmissionRequest {
    #[must_use]
    pub const fn effective_source_lane(&self) -> Option<&SourceLaneRequest> {
        if self.validator_sub_job {
            None
        } else {
            self.source_lane.as_ref()
        }
    }

    #[must_use]
    const fn effective_demand(&self) -> EffectiveDemand {
        if self.validator_sub_job {
            EffectiveDemand {
                browser_slots: 0,
                agent_slots: self.adapter_session as u32,
            }
        } else {
            EffectiveDemand {
                browser_slots: self.browser_session as u32,
                agent_slots: self.adapter_session as u32,
            }
        }
    }
}
