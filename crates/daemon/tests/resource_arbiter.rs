#![forbid(unsafe_code)]

#[path = "../src/agents/arbiter.rs"]
mod arbiter_impl;

mod tests {
    use anyhow::{Result, anyhow, ensure};

    use crate::arbiter_impl::{
        AdmissionPriority, AdmissionRequest, ArbiterDecision, ArbiterEventKind, DenialReason,
        ReleaseCause, ResourceArbiter, ResourceBudgets, SourceKind, SourceLaneRequest,
    };

    fn browser_request(job_id: &str, source_id: &str) -> AdmissionRequest {
        AdmissionRequest {
            job_id: job_id.to_owned(),
            session_id: Some(format!("session-{job_id}")),
            priority: AdmissionPriority::ProcurementIngestion,
            browser_session: true,
            adapter_session: true,
            source_lane: Some(SourceLaneRequest {
                source_id: source_id.to_owned(),
                source_kind: SourceKind::Procurement,
            }),
            validator_sub_job: false,
            concurrency_key: Some("skill/fts".to_owned()),
            max_concurrency: None,
        }
    }

    #[test]
    fn l3_t_job_claim_006_browser_slots_is_a_hard_cap() -> Result<()> {
        let mut arbiter = ResourceArbiter::new(ResourceBudgets::default());
        let first = arbiter.admit(&browser_request("job-browser-1", "fts-a"));
        let second = arbiter.admit(&browser_request("job-browser-2", "fts-b"));
        let third = arbiter.admit(&browser_request("job-browser-3", "fts-c"));

        ensure!(matches!(first, ArbiterDecision::Granted(_)));
        ensure!(matches!(second, ArbiterDecision::Granted(_)));
        ensure!(
            matches!(
                third,
                ArbiterDecision::Denied(DenialReason::BrowserSlotsExhausted {
                    limit: 2,
                    in_use: 2,
                })
            ),
            "L3-T-job_claim-006 expected third browser-backed session to be denied"
        );
        Ok(())
    }

    #[test]
    fn l3_t_job_claim_007_validator_sub_jobs_only_consume_agent_slots() -> Result<()> {
        let mut arbiter = ResourceArbiter::new(ResourceBudgets::default());

        let validator = AdmissionRequest {
            job_id: "validator-job".to_owned(),
            session_id: Some("validator-session".to_owned()),
            priority: AdmissionPriority::ManifestGeneration,
            browser_session: true,
            adapter_session: true,
            source_lane: Some(SourceLaneRequest {
                source_id: "fts".to_owned(),
                source_kind: SourceKind::Procurement,
            }),
            validator_sub_job: true,
            concurrency_key: Some("skill/validator-base".to_owned()),
            max_concurrency: None,
        };

        let lease = match arbiter.admit(&validator) {
            ArbiterDecision::Granted(lease) => lease,
            ArbiterDecision::Denied(reason) => {
                return Err(anyhow!(
                    "L3-T-job_claim-007 unexpectedly denied validator sub-job: {reason:?}"
                ));
            }
        };

        let snapshot = arbiter.snapshot();
        ensure!(lease.browser_slots == 0);
        ensure!(lease.agent_slots == 1);
        ensure!(lease.source_lane_key.is_none());
        ensure!(snapshot.browser_slots == 0);
        ensure!(snapshot.agent_slots == 1);
        ensure!(
            snapshot.source_lanes.is_empty(),
            "L3-T-job_claim-007 validator sub-jobs must not consume source lanes"
        );

        arbiter.release(&lease, ReleaseCause::Reaped);
        ensure!(arbiter.snapshot().agent_slots == 0);
        Ok(())
    }

    #[test]
    fn l3_t_job_claim_008_source_lanes_are_source_specific() -> Result<()> {
        let mut arbiter = ResourceArbiter::new(ResourceBudgets::default());
        let first = arbiter.admit(&browser_request("job-source-1", "fts"));
        let second = arbiter.admit(&browser_request("job-source-2", "fts"));
        let third = arbiter.admit(&browser_request("job-source-3", "cf"));

        ensure!(matches!(first, ArbiterDecision::Granted(_)));
        ensure!(
            matches!(
                second,
                ArbiterDecision::Denied(DenialReason::SourceLaneExhausted {
                    source_id,
                    limit: 1,
                    in_use: 1,
                }) if source_id == "fts"
            ),
            "L3-T-job_claim-008 expected same-source request to be denied on lane saturation"
        );
        ensure!(
            matches!(third, ArbiterDecision::Granted(_)),
            "L3-T-job_claim-008 expected unrelated source to proceed"
        );
        Ok(())
    }

    #[test]
    fn l3_t_job_claim_009_skill_cap_lowers_effective_ceiling_and_events_are_keyed() -> Result<()> {
        let mut arbiter = ResourceArbiter::new(ResourceBudgets::default());
        ensure!(arbiter.budgets().browser_slots == 2);
        ensure!(AdmissionPriority::AuthRefreshOrHealthProbe < AdmissionPriority::Qualification);
        ensure!(AdmissionPriority::BidAutomation > AdmissionPriority::ManifestGeneration);
        let first = AdmissionRequest {
            job_id: "job-cap-1".to_owned(),
            session_id: Some("session-cap-1".to_owned()),
            priority: AdmissionPriority::SupplierEnrichment,
            browser_session: true,
            adapter_session: true,
            source_lane: Some(SourceLaneRequest {
                source_id: "alibaba".to_owned(),
                source_kind: SourceKind::Supplier,
            }),
            validator_sub_job: false,
            concurrency_key: Some("skill/alibaba-enrichment".to_owned()),
            max_concurrency: Some(1),
        };
        let second = AdmissionRequest {
            job_id: "job-cap-2".to_owned(),
            session_id: Some("session-cap-2".to_owned()),
            ..first.clone()
        };

        let lease = match arbiter.admit(&first) {
            ArbiterDecision::Granted(lease) => lease,
            ArbiterDecision::Denied(reason) => {
                return Err(anyhow!(
                    "L3-T-job_claim-009 unexpectedly denied first request: {reason:?}"
                ));
            }
        };

        ensure!(
            matches!(
                arbiter.admit(&second),
                ArbiterDecision::Denied(DenialReason::SkillConcurrencyCapReached {
                    key,
                    limit: 1,
                    in_use: 1,
                }) if key == "skill/alibaba-enrichment"
            ),
            "L3-T-job_claim-009 expected skill max_concurrency to lower the effective ceiling"
        );

        let events = arbiter.events();
        ensure!(events.len() == 2);
        ensure!(events[0].kind == ArbiterEventKind::Grant);
        ensure!(events[0].job_id == "job-cap-1");
        ensure!(events[0].session_id.as_deref() == Some("session-cap-1"));
        ensure!(events[1].kind == ArbiterEventKind::Deny);
        ensure!(events[1].job_id == "job-cap-2");
        ensure!(events[1].session_id.as_deref() == Some("session-cap-2"));

        arbiter.release(&lease, ReleaseCause::PreSpawnFailure);
        let release_event = arbiter
            .events()
            .last()
            .ok_or_else(|| anyhow!("L3-T-job_claim-009 expected release event"))?;
        ensure!(release_event.kind == ArbiterEventKind::Release);
        ensure!(release_event.job_id == "job-cap-1");

        let second_lease = match arbiter.admit(&first) {
            ArbiterDecision::Granted(lease) => lease,
            ArbiterDecision::Denied(reason) => {
                return Err(anyhow!(
                    "L3-T-job_claim-009 unexpectedly denied second grant after release: {reason:?}"
                ));
            }
        };
        arbiter.release(&second_lease, ReleaseCause::Archived);
        let last_event = arbiter
            .events()
            .last()
            .ok_or_else(|| anyhow!("L3-T-job_claim-009 expected release event"))?;
        ensure!(last_event.kind == ArbiterEventKind::Release);
        ensure!(last_event.job_id == "job-cap-1");
        Ok(())
    }
}
