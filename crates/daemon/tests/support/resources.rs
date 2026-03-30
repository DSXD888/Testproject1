#![forbid(unsafe_code)]
#![allow(clippy::duplicate_mod)]
#![allow(dead_code, unused_imports, clippy::match_same_arms)]

use std::collections::BTreeMap;

use anyhow::{Result, anyhow};

#[path = "../../src/agents/arbiter.rs"]
mod arbiter_impl;

pub use arbiter_impl::{AdmissionPriority, ArbiterDecision, DenialReason, ReleaseCause};
use arbiter_impl::{
    AdmissionRequest, ArbiterEventKind, ResourceArbiter, ResourceBudgets, ResourceLease,
    SourceKind, SourceLaneRequest,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrantDenyReleaseEvent {
    pub kind: String,
    pub job_id: String,
    pub denial_reason: Option<String>,
    pub release_cause: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdmissionOutcome {
    Granted(String),
    Denied(String),
}

#[derive(Debug, Clone)]
pub struct ResourceArbiterFixture {
    arbiter: ResourceArbiter,
    source_lane_overrides: BTreeMap<String, u32>,
    source_lane_in_use: BTreeMap<String, u32>,
    leases: BTreeMap<String, ResourceLease>,
    captured_events: Vec<GrantDenyReleaseEvent>,
    next_job_seq: u32,
}

impl ResourceArbiterFixture {
    #[must_use]
    pub fn with_defaults(browser_slots: u32, agent_slots: u32) -> Self {
        let budgets = ResourceBudgets {
            browser_slots,
            agent_slots,
            ..ResourceBudgets::default()
        };
        Self {
            arbiter: ResourceArbiter::new(budgets),
            source_lane_overrides: BTreeMap::new(),
            source_lane_in_use: BTreeMap::new(),
            leases: BTreeMap::new(),
            captured_events: Vec::new(),
            next_job_seq: 1,
        }
    }

    pub fn seed_source_lane(&mut self, source_id: &str, lanes: u32) {
        self.source_lane_overrides
            .insert(source_id.to_owned(), lanes);
    }

    pub fn admit(
        &mut self,
        job_kind: &str,
        source_id: &str,
        uses_browser: bool,
        is_validator: bool,
    ) -> AdmissionOutcome {
        let job_id = format!("fixture-job-{}", self.next_job_seq);
        self.next_job_seq += 1;

        if let Some(limit) = self.source_lane_overrides.get(source_id).copied() {
            let in_use = self.source_lane_in_use.get(source_id).copied().unwrap_or(0);
            if in_use >= limit {
                self.captured_events.push(GrantDenyReleaseEvent {
                    kind: "deny".to_owned(),
                    job_id,
                    denial_reason: Some("source_lane_override_exhausted".to_owned()),
                    release_cause: None,
                });
                return AdmissionOutcome::Denied("source_lane_override_exhausted".to_owned());
            }
        }

        let request = AdmissionRequest {
            job_id: job_id.clone(),
            session_id: Some(format!("session-{job_id}")),
            priority: admission_priority(job_kind),
            browser_session: uses_browser,
            adapter_session: true,
            source_lane: Some(SourceLaneRequest {
                source_id: source_id.to_owned(),
                source_kind: source_kind(source_id),
            }),
            validator_sub_job: is_validator,
            concurrency_key: Some(format!("fixture/{job_kind}/{source_id}")),
            max_concurrency: None,
        };

        match self.arbiter.admit(&request) {
            ArbiterDecision::Granted(lease) => {
                *self
                    .source_lane_in_use
                    .entry(source_id.to_owned())
                    .or_default() += 1;
                self.leases.insert(job_id.clone(), lease);
                self.capture_latest_event();
                AdmissionOutcome::Granted(job_id)
            }
            ArbiterDecision::Denied(reason) => {
                self.capture_latest_event();
                AdmissionOutcome::Denied(format!("{reason:?}"))
            }
        }
    }

    pub fn release(&mut self, job_id: &str) -> Result<()> {
        let lease = self
            .leases
            .remove(job_id)
            .ok_or_else(|| anyhow!("missing lease for `{job_id}`"))?;
        if let Some(source_id) = &lease.source_lane_key
            && let Some(in_use) = self.source_lane_in_use.get_mut(source_id)
        {
            *in_use = in_use.saturating_sub(1);
            if *in_use == 0 {
                self.source_lane_in_use.remove(source_id);
            }
        }
        self.arbiter.release(&lease, ReleaseCause::Archived);
        self.capture_latest_event();
        Ok(())
    }

    #[must_use]
    pub fn captured_events(&self) -> Vec<GrantDenyReleaseEvent> {
        self.captured_events.clone()
    }

    fn capture_latest_event(&mut self) {
        if let Some(event) = self.arbiter.events().last() {
            let denial_reason = event
                .denial_reason
                .as_ref()
                .map(|reason| format!("{reason:?}"));
            let release_cause = event
                .release_cause
                .as_ref()
                .map(|cause| format!("{cause:?}"));
            let kind = match event.kind {
                ArbiterEventKind::Grant => "grant",
                ArbiterEventKind::Deny => "deny",
                ArbiterEventKind::Release => "release",
            };
            self.captured_events.push(GrantDenyReleaseEvent {
                kind: kind.to_owned(),
                job_id: event.job_id.clone(),
                denial_reason,
                release_cause,
            });
        }
    }
}

fn admission_priority(job_kind: &str) -> AdmissionPriority {
    match job_kind {
        "procurement_ingestion" => AdmissionPriority::ProcurementIngestion,
        "auth_refresh" | "source_health" => AdmissionPriority::AuthRefreshOrHealthProbe,
        "qualification" => AdmissionPriority::Qualification,
        "supplier_enrichment" => AdmissionPriority::SupplierEnrichment,
        "manifest_generation" => AdmissionPriority::ManifestGeneration,
        "bid_automation" => AdmissionPriority::BidAutomation,
        _ => AdmissionPriority::ManifestGeneration,
    }
}

fn source_kind(source_id: &str) -> SourceKind {
    match source_id {
        "fts" | "contracts-finder" => SourceKind::Procurement,
        _ => SourceKind::Supplier,
    }
}
