use anyhow::{Context, Result, bail, ensure};
use serde_json::Value;

use super::state_machine::{BidStateMachine, BidTransition};
use super::types::{BidScopePayload, BidWorkflowState};
use crate::jobs::runner::{ClaimedJob, JobRunner};
use crate::sources::registry::SourceRegistry;
use crate::{LiveLoopReport, RuntimeBootstrap};

const BID_PORTAL_SKILL_ID: &str = "bid-portal/procontract";
const BID_PORTAL_INTERACTION_PROFILE: &str = "pty_interactive";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BidPortalSkillProfile {
    pub skill_id: String,
    pub skill_version: String,
    pub interaction_profile: String,
}

#[allow(clippy::future_not_send)]
pub async fn run_bid_workflow_job(
    _runtime: &mut RuntimeBootstrap,
    _runner: &JobRunner,
    claimed: ClaimedJob,
) -> Result<LiveLoopReport> {
    let _bid_scope = parse_bid_scope_payload(&claimed.input_payload)?;
    let skill_profile = load_bid_portal_skill_profile()?;
    ensure_bid_portal_pty_profile(&skill_profile)?;
    ensure_initial_bid_in_progress_transition_is_wired()?;

    bail!("Phase 3 implements bid workflow job execution")
}

pub fn parse_bid_scope_payload(input_payload: &Value) -> Result<BidScopePayload> {
    let bid_scope_payload = input_payload
        .get("bidScopePayload")
        .or_else(|| input_payload.get("bid_scope_payload"))
        .unwrap_or(input_payload);

    serde_json::from_value(bid_scope_payload.clone())
        .context("invalid BidScopePayload in workflow_jobs.input_payload")
}

pub fn load_bid_portal_skill_profile() -> Result<BidPortalSkillProfile> {
    let definition = SourceRegistry::builtin_definition(BID_PORTAL_SKILL_ID)
        .ok_or_else(|| anyhow::anyhow!("skill not registered: {BID_PORTAL_SKILL_ID}"))?;
    let binding = definition
        .production_binding
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("{BID_PORTAL_SKILL_ID} missing production binding"))?;
    ensure!(
        binding.skill_id == BID_PORTAL_SKILL_ID,
        "{BID_PORTAL_SKILL_ID} binding points at unexpected skill `{}`",
        binding.skill_id
    );
    let interaction_profile = definition
        .config
        .get("interaction_profile")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            anyhow::anyhow!("{BID_PORTAL_SKILL_ID} missing interaction_profile config")
        })?;

    Ok(BidPortalSkillProfile {
        skill_id: binding.skill_id.clone(),
        skill_version: binding.skill_version.clone(),
        interaction_profile: interaction_profile.to_owned(),
    })
}

pub fn ensure_bid_portal_pty_profile(profile: &BidPortalSkillProfile) -> Result<()> {
    ensure!(
        profile.skill_id == BID_PORTAL_SKILL_ID,
        "bid workflow runner loaded unexpected skill `{}`",
        profile.skill_id
    );
    ensure!(
        profile.interaction_profile == BID_PORTAL_INTERACTION_PROFILE,
        "{BID_PORTAL_SKILL_ID} must declare interaction_profile={BID_PORTAL_INTERACTION_PROFILE} (got `{}`)",
        profile.interaction_profile
    );
    Ok(())
}

#[must_use]
pub const fn initial_bid_in_progress_transition()
-> (BidWorkflowState, BidTransition, BidWorkflowState) {
    (
        BidWorkflowState::ReadyToBid,
        BidTransition::BeginPortalWork,
        BidWorkflowState::BidInProgress,
    )
}

fn ensure_initial_bid_in_progress_transition_is_wired() -> Result<()> {
    let (from_state, transition, expected_state) = initial_bid_in_progress_transition();
    let mut machine = BidStateMachine::new(from_state);
    let actual_state = machine.transition(transition).map_err(|error| {
        anyhow::anyhow!(
            "Phase 3 implements initial bid workflow state transition to {expected_state:?}: {error}"
        )
    })?;
    ensure!(
        actual_state == expected_state,
        "bid workflow initial transition reached {actual_state:?}, expected {expected_state:?}"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{
        BID_PORTAL_INTERACTION_PROFILE, BID_PORTAL_SKILL_ID, BidPortalSkillProfile,
        ensure_bid_portal_pty_profile, initial_bid_in_progress_transition,
        load_bid_portal_skill_profile, parse_bid_scope_payload,
    };
    use crate::bid_workflow::state_machine::BidTransition;
    use crate::bid_workflow::types::BidWorkflowState;

    fn valid_bid_scope_payload() -> serde_json::Value {
        json!({
            "processId": "process-1",
            "lotIds": ["lot-1"],
            "candidateIds": ["candidate-1"],
            "selectedQuoteIds": ["quote-1"],
            "companyProfileVersion": "company-profile-v1",
            "bidPortalType": "procontract",
            "portalOpportunityRef": "DN-123",
            "reviewRequired": true,
            "decisionBasisHash": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
            "sideEffectCap": "REMOTE_DRAFT"
        })
    }

    #[test]
    fn l6_t_daemon_runner_004_parses_direct_and_nested_bid_scope_payload() {
        let direct = parse_bid_scope_payload(&valid_bid_scope_payload())
            .expect("direct BidScopePayload parses");
        let nested = parse_bid_scope_payload(&json!({
            "bidScopePayload": valid_bid_scope_payload(),
            "stageRunId": "stage-run-1"
        }))
        .expect("nested BidScopePayload parses");

        assert_eq!(direct.process_id, "process-1");
        assert_eq!(nested.portal_opportunity_ref, "DN-123");
        assert!(parse_bid_scope_payload(&json!({"processId": "missing required fields"})).is_err());
    }

    #[test]
    fn l6_t_daemon_runner_004_loads_bid_portal_pty_profile_from_registry() {
        let profile =
            load_bid_portal_skill_profile().expect("bid-portal/procontract registry profile");

        assert_eq!(profile.skill_id, BID_PORTAL_SKILL_ID);
        assert_eq!(profile.interaction_profile, BID_PORTAL_INTERACTION_PROFILE);
        ensure_bid_portal_pty_profile(&profile).expect("pty profile accepted");
    }

    #[test]
    fn l6_t_daemon_runner_004_rejects_contract_only_bid_portal_profile() {
        let profile = BidPortalSkillProfile {
            skill_id: BID_PORTAL_SKILL_ID.to_owned(),
            skill_version: "0.1.0".to_owned(),
            interaction_profile: "headless_contract".to_owned(),
        };

        let error = ensure_bid_portal_pty_profile(&profile).expect_err("contract profile rejected");
        assert!(
            error
                .to_string()
                .contains("interaction_profile=pty_interactive")
        );
    }

    #[test]
    fn l6_t_daemon_runner_004_wires_initial_bid_in_progress_transition() {
        assert_eq!(
            initial_bid_in_progress_transition(),
            (
                BidWorkflowState::ReadyToBid,
                BidTransition::BeginPortalWork,
                BidWorkflowState::BidInProgress,
            )
        );
    }
}
