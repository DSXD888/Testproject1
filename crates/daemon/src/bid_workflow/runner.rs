use anyhow::{Context, Result, bail, ensure};
use chrono::Utc;
use rusqlite::{OptionalExtension, params};
use serde_json::{Value, json};
use uuid::Uuid;

use super::state_machine::{BidStateMachine, BidTransition};
use super::types::{BidScopePayload, BidWorkflowState};
use crate::errors::manifest::ErrorCode;
use crate::jobs::runner::{ClaimedJob, JobRunner};
use crate::sources::registry::SourceRegistry;
use crate::{LiveLoopReport, RuntimeBootstrap};

const BID_PORTAL_SKILL_ID: &str = "bid-portal/procontract";
const BID_PORTAL_INTERACTION_PROFILE: &str = "pty_interactive";
const AUTH_SIGNIN_PROCONTRACT_JOB_KIND: &str = "run_auth_signin_procontract";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BidPortalSkillProfile {
    pub skill_id: String,
    pub skill_version: String,
    pub interaction_profile: String,
}

#[allow(clippy::future_not_send)]
pub async fn run_bid_workflow_job(
    runtime: &mut RuntimeBootstrap,
    _runner: &JobRunner,
    claimed: ClaimedJob,
) -> Result<LiveLoopReport> {
    let _bid_scope = parse_bid_scope_payload(&claimed.input_payload)?;
    if bid_auth_state_stale_requested(&claimed.input_payload) {
        return orchestrate_bid_auth_stale_retry(runtime, &claimed);
    }

    let skill_profile = load_bid_portal_skill_profile()?;
    ensure_bid_portal_pty_profile(&skill_profile)?;
    ensure_initial_bid_in_progress_transition_is_wired()?;

    bail!("Phase 3 implements bid workflow job execution")
}

pub fn parse_bid_scope_payload(input_payload: &Value) -> Result<BidScopePayload> {
    let request_payload = input_payload.get("request").unwrap_or(input_payload);
    let bid_scope_payload = request_payload
        .get("bidScopePayload")
        .or_else(|| request_payload.get("bid_scope_payload"))
        .unwrap_or(request_payload);

    serde_json::from_value(bid_scope_payload.clone())
        .context("invalid BidScopePayload in workflow_jobs.input_payload")
}

fn bid_auth_state_stale_requested(input_payload: &Value) -> bool {
    fn value_signals_stale(value: &Value) -> bool {
        value.as_str().is_some_and(|text| {
            matches!(
                text,
                "AUTH_STATE_STALE"
                    | "auth_state_stale"
                    | "auth-state-stale"
                    | "stale"
                    | "expired"
                    | "session_expired"
            )
        })
    }

    fn object_signals_stale(value: &Value) -> bool {
        value.as_object().is_some_and(|object| {
            object
                .get("simulate_auth_state_stale")
                .or_else(|| object.get("simulateAuthStateStale"))
                .and_then(Value::as_bool)
                == Some(true)
                || object
                    .get("error_code")
                    .or_else(|| object.get("errorCode"))
                    .is_some_and(value_signals_stale)
                || object
                    .get("auth_state")
                    .or_else(|| object.get("authState"))
                    .is_some_and(value_signals_stale)
                || object.get("result").is_some_and(object_signals_stale)
                || object
                    .get("skill_result")
                    .or_else(|| object.get("skillResult"))
                    .is_some_and(object_signals_stale)
        })
    }

    object_signals_stale(input_payload)
        || input_payload
            .get("request")
            .is_some_and(object_signals_stale)
}

fn bid_auth_retry_attempt(input_payload: &Value) -> u64 {
    input_payload
        .get("request")
        .unwrap_or(input_payload)
        .get("auth_retry")
        .or_else(|| {
            input_payload
                .get("request")
                .unwrap_or(input_payload)
                .get("authRetry")
        })
        .and_then(|value| value.get("attempt"))
        .and_then(Value::as_u64)
        .unwrap_or(0)
}

fn orchestrate_bid_auth_stale_retry(
    runtime: &mut RuntimeBootstrap,
    claimed: &ClaimedJob,
) -> Result<LiveLoopReport> {
    let retry_attempt = bid_auth_retry_attempt(&claimed.input_payload);
    if retry_attempt > 0 {
        return fail_bid_portal_auth_expired(runtime, claimed, retry_attempt);
    }

    if let Some(existing) =
        existing_bid_auth_retry(runtime, &claimed.id).context("checking existing bid auth retry")?
    {
        return Ok(LiveLoopReport {
            summary_line: format!(
                "Bid workflow auth refresh already enqueued: job_id={} auth_job_id={} retry_job_id={}",
                claimed.id, existing.auth_job_id, existing.retry_job_id
            ),
            status: "completed".to_owned(),
        });
    }

    let auth_job_id = format!("job-{}", Uuid::new_v4());
    let retry_job_id = format!("job-{}", Uuid::new_v4());
    let continuation_chain_id = continuation_chain_id(&claimed.input_payload, &claimed.id);
    let retry_input_payload = retry_bid_input_payload(&claimed.input_payload, &auth_job_id)?;
    let auth_input_payload = auth_refresh_input_payload(claimed, &continuation_chain_id);
    let completed_payload = json!({
        "runner_meta": {
            "idempotency_key": format!("bid-auth-state-stale::{}", claimed.id),
            "visible_after_seq": Value::Null,
            "executor_id": "local-daemon",
            "continuation_job_id": retry_job_id,
            "auth_refresh_job_id": auth_job_id,
        },
        "auth_retry": {
            "error_code": ErrorCode::AuthStateStale.as_str(),
            "auth_refresh_job_id": auth_job_id,
            "retry_job_id": retry_job_id,
            "auth_skill_id": "auth-signin/procontract",
            "attempt": 1,
        }
    });

    runtime.state_db.with_write_transaction(|tx| {
        tx.execute(
            "\
            INSERT INTO workflow_jobs (
              id, kind, process_id, scope_kind, scope_id, parent_job_id,
              continuation_chain_id, validation_required, status, input_payload
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0, 'pending', ?8)
            ",
            params![
                auth_job_id,
                AUTH_SIGNIN_PROCONTRACT_JOB_KIND,
                claimed.process_id,
                claimed.scope_kind,
                claimed.scope_id,
                claimed.id,
                continuation_chain_id,
                auth_input_payload.to_string(),
            ],
        )?;
        tx.execute(
            "\
            INSERT INTO workflow_jobs (
              id, kind, process_id, scope_kind, scope_id, parent_job_id,
              continuation_chain_id, validation_required, status, input_payload
            ) VALUES (?1, 'run_bid_workflow', ?2, ?3, ?4, ?5, ?6, 1, 'pending', ?7)
            ",
            params![
                retry_job_id,
                claimed.process_id,
                claimed.scope_kind,
                claimed.scope_id,
                auth_job_id,
                continuation_chain_id,
                retry_input_payload.to_string(),
            ],
        )?;
        let updated = tx.execute(
            "\
            UPDATE workflow_jobs
            SET status = 'completed',
                chain_closed = 0,
                completed_at = ?1,
                output_payload = ?2
            WHERE id = ?3
            ",
            params![
                Utc::now().to_rfc3339(),
                completed_payload.to_string(),
                claimed.id
            ],
        )?;
        ensure!(updated == 1, "workflow job missing for {}", claimed.id);
        Ok(())
    })?;

    Ok(LiveLoopReport {
        summary_line: format!(
            "Bid workflow auth refresh enqueued: job_id={} auth_job_id={} retry_job_id={}",
            claimed.id, auth_job_id, retry_job_id
        ),
        status: "completed".to_owned(),
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ExistingBidAuthRetry {
    auth_job_id: String,
    retry_job_id: String,
}

fn existing_bid_auth_retry(
    runtime: &RuntimeBootstrap,
    parent_job_id: &str,
) -> Result<Option<ExistingBidAuthRetry>> {
    let auth_job_id: Option<String> = runtime
        .state_db
        .connection()
        .query_row(
            "\
            SELECT id
            FROM workflow_jobs
            WHERE parent_job_id = ?1
              AND kind = ?2
            ORDER BY created_at DESC
            LIMIT 1
            ",
            params![parent_job_id, AUTH_SIGNIN_PROCONTRACT_JOB_KIND],
            |row| row.get(0),
        )
        .optional()?;
    let Some(auth_job_id) = auth_job_id else {
        return Ok(None);
    };
    let retry_job_id: Option<String> = runtime
        .state_db
        .connection()
        .query_row(
            "\
            SELECT id
            FROM workflow_jobs
            WHERE parent_job_id = ?1
              AND kind = 'run_bid_workflow'
            ORDER BY created_at DESC
            LIMIT 1
            ",
            params![auth_job_id],
            |row| row.get(0),
        )
        .optional()?;
    Ok(retry_job_id.map(|retry_job_id| ExistingBidAuthRetry {
        auth_job_id,
        retry_job_id,
    }))
}

fn fail_bid_portal_auth_expired(
    runtime: &mut RuntimeBootstrap,
    claimed: &ClaimedJob,
    retry_attempt: u64,
) -> Result<LiveLoopReport> {
    let error_code = ErrorCode::BidPortalAuthExpired;
    let message = format!(
        "{}: Procontract session remained expired after daemon-orchestrated auth retry",
        error_code.as_str()
    );
    let output_payload = json!({
        "error_code": error_code.as_str(),
        "auth_retry": {
            "attempt": retry_attempt,
            "auth_skill_id": "auth-signin/procontract",
            "bounded_retry_exhausted": true,
        }
    });
    runtime.state_db.with_write_transaction(|tx| {
        let updated = tx.execute(
            "\
            UPDATE workflow_jobs
            SET status = 'failed',
                last_error = ?1,
                completed_at = ?2,
                output_payload = ?3
            WHERE id = ?4
            ",
            params![
                message,
                Utc::now().to_rfc3339(),
                output_payload.to_string(),
                claimed.id
            ],
        )?;
        ensure!(updated == 1, "workflow job missing for {}", claimed.id);
        Ok(())
    })?;

    Ok(LiveLoopReport {
        summary_line: format!(
            "Bid workflow failed: job_id={} reason={message}",
            claimed.id
        ),
        status: "failed".to_owned(),
    })
}

fn continuation_chain_id(input_payload: &Value, job_id: &str) -> String {
    input_payload
        .get("runner_meta")
        .and_then(|meta| meta.get("continuation_chain_id"))
        .and_then(Value::as_str)
        .or_else(|| {
            input_payload
                .get("request")
                .and_then(|request| request.get("continuation_chain_id"))
                .and_then(Value::as_str)
        })
        .map_or_else(|| job_id.to_owned(), str::to_owned)
}

fn retry_bid_input_payload(input_payload: &Value, auth_job_id: &str) -> Result<Value> {
    let mut retry_payload = input_payload.clone();
    // Clippy's `map_or` suggestion borrows `retry_payload` twice here.
    #[allow(clippy::option_if_let_else)]
    let request = match retry_payload.get_mut("request") {
        Some(request) => request,
        None => &mut retry_payload,
    };
    let request_object = request
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("bid workflow request payload must be a JSON object"))?;

    for key in [
        "simulate_auth_state_stale",
        "simulateAuthStateStale",
        "error_code",
        "errorCode",
        "auth_state",
        "authState",
        "result",
        "skill_result",
        "skillResult",
    ] {
        request_object.remove(key);
    }
    request_object.insert(
        "auth_retry".to_owned(),
        json!({
            "attempt": 1,
            "auth_skill_id": "auth-signin/procontract",
            "auth_refresh_job_id": auth_job_id,
        }),
    );

    if let Some(meta) = retry_payload
        .as_object_mut()
        .and_then(|object| object.get_mut("runner_meta"))
        .and_then(Value::as_object_mut)
    {
        meta.insert(
            "idempotency_key".to_owned(),
            json!(format!("bid-auth-retry::{auth_job_id}::1")),
        );
        meta.insert("continuation_job_id".to_owned(), Value::Null);
    }

    Ok(retry_payload)
}

fn auth_refresh_input_payload(claimed: &ClaimedJob, continuation_chain_id: &str) -> Value {
    json!({
        "request": {
            "skill_id": "auth-signin/procontract",
            "reason": ErrorCode::AuthStateStale.as_str(),
            "source_job_id": claimed.id,
            "source_job_kind": claimed.kind,
        },
        "runner_meta": {
            "idempotency_key": format!("auth-signin/procontract::{continuation_chain_id}::1"),
            "continuation_chain_id": continuation_chain_id,
            "executor_id": "local-daemon",
        }
    })
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
        let queued = parse_bid_scope_payload(&json!({
            "request": {
                "bid_scope_payload": valid_bid_scope_payload(),
                "stage_run_id": "stage-run-1"
            },
            "runner_meta": {
                "idempotency_key": "queued-bid"
            }
        }))
        .expect("queued BidScopePayload parses");

        assert_eq!(direct.process_id, "process-1");
        assert_eq!(nested.portal_opportunity_ref, "DN-123");
        assert_eq!(queued.selected_quote_ids, vec!["quote-1"]);
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
