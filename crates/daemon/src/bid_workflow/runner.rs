use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use anyhow::{Context, Result, anyhow, bail, ensure};
use chrono::Utc;
use rusqlite::{OptionalExtension, params};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use super::readiness_gate::{BidReadinessGateResult, evaluate_bid_readiness};
use super::side_effect_guard::{
    CapDecision, enforce_procontract_observed_side_effect, enforce_procontract_step_cap,
    planned_procontract_steps,
};
use super::state_machine::{
    BidStateMachine, BidTransition, TransitionContext, TransitionTrigger, ValidatorGate,
};
use super::types::{
    BidDocument, BidScopePayload, BidWorkflowState, CompanyProfile, SideEffectClass,
};
use crate::agents::arbiter::{
    AdmissionPriority, AdmissionRequest, ArbiterDecision, ReleaseCause, ResourceArbiter,
    ResourceLease,
};
use crate::agents::artifacts::{
    BidScopeSummary, DaemonEvidenceAppendix, GateResult, OutcomeClass, ReadinessEvidenceSnapshot,
    RunManifestFrontmatter, RunNarrative, SideEffectCheckpoint, ValidatorVerdictSummary,
    persist_run_manifest_metadata,
};
use crate::db::sqlite::DEFAULT_OWNER_ID;
use crate::errors::manifest::ErrorCode;
use crate::jobs::runner::{ClaimedJob, JobRunner};
use crate::pty::capture::{CaptureConfig, PtyCapture};
use crate::sources::registry::SourceRegistry;
use crate::validation::manifest_writer::{ManifestWriteRequest, write_hybrid_run_manifest};
use crate::{LiveLoopReport, RuntimeBootstrap};

const BID_PORTAL_SKILL_ID: &str = "bid-portal/procontract";
const BID_PORTAL_INTERACTION_PROFILE: &str = "pty_interactive";
const AUTH_SIGNIN_PROCONTRACT_JOB_KIND: &str = "run_auth_signin_procontract";
const BID_PORTAL_SKILL_PATH: &str = "skills/bid-portal-procontract";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BidPortalSkillProfile {
    pub skill_id: String,
    pub skill_version: String,
    pub interaction_profile: String,
}

#[allow(clippy::future_not_send, clippy::too_many_lines)]
pub async fn run_bid_workflow_job(
    runtime: &mut RuntimeBootstrap,
    _runner: &JobRunner,
    claimed: ClaimedJob,
) -> Result<LiveLoopReport> {
    let mut bid_resource_lease: Option<BidResourceLease> = None;
    let attempt: Result<LiveLoopReport> = async {
        let request = BidWorkflowRunRequest::from_input_payload(&claimed.input_payload)?;
        let bid_scope = request.bid_scope.clone();
        if bid_auth_state_stale_requested(&claimed.input_payload) {
            return orchestrate_bid_auth_stale_retry(runtime, &claimed);
        }

        let readiness = evaluate_bid_readiness(runtime.state_db.connection(), &bid_scope)
            .context("evaluating bid workflow readiness")?;
        if !readiness.approved {
            return fail_bid_workflow_job(
                runtime,
                &claimed,
                ErrorCode::BidReadinessGateFailed,
                "bid workflow readiness gate failed",
                json!({
                    "readiness": {
                        "approved": false,
                        "blockers": readiness.blockers.iter().map(|blocker| {
                            json!({
                                "code": blocker.code(),
                                "source": blocker.source(),
                                "overrideable": blocker.overrideable(),
                                "diagnostic": blocker.diagnostic(),
                            })
                        }).collect::<Vec<_>>(),
                    }
                }),
            );
        }

        let skill_profile = load_bid_portal_skill_profile()?;
        ensure_bid_portal_pty_profile(&skill_profile)?;
        ensure_planned_bid_portal_side_effects_within_cap(&bid_scope)?;
        ensure_initial_bid_in_progress_transition_is_wired()?;

        let company_profile = request
            .company_profile
            .or_else(|| readiness.resolved_company_profile.clone())
            .ok_or_else(|| anyhow!("bid workflow request missing resolved CompanyProfile"))?;
        let quote_summaries =
            load_selected_quote_summaries(runtime.state_db.connection(), &bid_scope)?;
        let stage_run_id = request
            .stage_run_id
            .unwrap_or_else(|| format!("stage-run-bid-{}", Uuid::new_v4().simple()));
        let session_id = format!("session-bid-{}", sha256_hex(claimed.id.as_bytes()));
        let started_at = Utc::now();
        let session_dir =
            crate::agents::artifacts::ArtifactStore::new(&runtime.artifact_root)?
                .session_dir(&session_id, started_at)?;
        let skill_input = build_bid_skill_input(
            &bid_scope,
            &company_profile,
            &request.bid_documents,
            &quote_summaries,
            &skill_profile,
            &claimed,
            &stage_run_id,
        );
        write_bid_skill_input(&session_dir, &skill_input)?;

        bid_resource_lease = Some(acquire_bid_resource_lease(
            &runtime.resource_arbiter,
            &claimed.id,
            &session_id,
            &bid_scope.portal_opportunity_ref,
        )?);

        persist_bid_state_transition(
            runtime,
            &claimed,
            &bid_scope,
            BidWorkflowState::ReadyToBid,
            &stage_run_id,
            None,
        )?;
        transition_bid_in_progress(&bid_scope)?;
        persist_bid_state_transition(
            runtime,
            &claimed,
            &bid_scope,
            BidWorkflowState::BidInProgress,
            &stage_run_id,
            None,
        )?;

        let execution = execute_bid_portal_skill(
            runtime,
            &skill_profile,
            &claimed,
            &session_id,
            started_at,
            &session_dir,
            &skill_input,
            bid_scope.side_effect_cap,
            request.simulated_result,
        )
        .await?;
        let final_state =
            final_state_for_skill_result(&execution.result_json, bid_scope.side_effect_cap)?;
        let final_readiness = evaluate_bid_readiness(runtime.state_db.connection(), &bid_scope)
            .context("re-evaluating bid workflow readiness before final manifest")?;
        let manifest = write_bid_run_manifest(
            runtime,
            &skill_profile,
            &claimed,
            &session_id,
            started_at,
            &bid_scope,
            &stage_run_id,
            final_state,
            &execution,
            &readiness,
            &final_readiness,
        )?;
        persist_run_manifest_metadata(runtime.state_db.connection(), &session_id, &manifest)?;

        let final_context = final_transition_context(
            BidWorkflowState::BidInProgress,
            final_state,
            bid_scope.side_effect_cap,
            &execution,
        );
        let mut machine = BidStateMachine::new(BidWorkflowState::BidInProgress);
        machine
            .transition_to(final_state, &final_context)
            .map_err(|error| anyhow!(error))?;

        let output_payload = json!({
            "result": execution.result_json,
            "runner_meta": {
                "idempotency_key": request.idempotency_key.unwrap_or_else(|| bid_scope.decision_basis_hash.clone()),
                "visible_after_seq": Value::Null,
                "executor_id": "local-daemon",
                "session_id": session_id,
                "run_manifest_path": manifest.path,
                "run_manifest_hash": manifest.sha256,
                "bid_state": final_state.as_str(),
            },
            "skill_input": skill_input,
            "artifact_refs": execution.artifact_refs,
        });
        persist_bid_state_transition(
            runtime,
            &claimed,
            &bid_scope,
            final_state,
            &stage_run_id,
            Some(&output_payload),
        )?;

        if let Some(lease) = bid_resource_lease.as_mut() {
            lease.set_release_cause(ReleaseCause::Archived);
        }
        Ok(LiveLoopReport {
            summary_line: format!(
                "Bid workflow completed: job_id={} process_id={} bid_state={} session_id={}",
                claimed.id,
                bid_scope.process_id,
                final_state.as_str(),
                session_id
            ),
            status: "completed".to_owned(),
        })
    }
    .await;

    let release_result = release_bid_resource_lease(&runtime.resource_arbiter, bid_resource_lease);

    match (attempt, release_result) {
        (Ok(report), Ok(())) => Ok(report),
        (Ok(_), Err(error)) | (Err(error), Ok(())) => fail_bid_workflow_job(
            runtime,
            &claimed,
            ErrorCode::BidSubmissionFailed,
            &error.to_string(),
            json!({
                "error_code": ErrorCode::BidSubmissionFailed.as_str(),
                "job_kind": "run_bid_workflow",
            }),
        ),
        (Err(error), Err(release_error)) => fail_bid_workflow_job(
            runtime,
            &claimed,
            ErrorCode::BidSubmissionFailed,
            &format!("{error}; additionally failed to release bid resource lease: {release_error}"),
            json!({
                "error_code": ErrorCode::BidSubmissionFailed.as_str(),
                "job_kind": "run_bid_workflow",
            }),
        ),
    }
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

#[derive(Debug, Clone)]
struct BidWorkflowRunRequest {
    bid_scope: BidScopePayload,
    company_profile: Option<CompanyProfile>,
    bid_documents: Vec<BidDocument>,
    stage_run_id: Option<String>,
    idempotency_key: Option<String>,
    simulated_result: Option<Value>,
}

impl BidWorkflowRunRequest {
    fn from_input_payload(input_payload: &Value) -> Result<Self> {
        let request_payload = input_payload.get("request").unwrap_or(input_payload);
        let bid_scope = parse_bid_scope_payload(input_payload)?;
        let company_profile = request_payload
            .get("companyProfile")
            .or_else(|| request_payload.get("company_profile"))
            .map(|value| serde_json::from_value(value.clone()))
            .transpose()
            .context("parsing CompanyProfile from bid workflow payload")?;
        let bid_documents = request_payload
            .get("bidDocuments")
            .or_else(|| request_payload.get("bid_documents"))
            .map(|value| serde_json::from_value(value.clone()))
            .transpose()
            .context("parsing BidDocument list from bid workflow payload")?
            .unwrap_or_default();
        let stage_run_id = request_payload
            .get("stageRunId")
            .or_else(|| request_payload.get("stage_run_id"))
            .and_then(Value::as_str)
            .map(str::to_owned);
        let idempotency_key = input_payload
            .get("runner_meta")
            .and_then(|meta| meta.get("idempotency_key"))
            .and_then(Value::as_str)
            .map(str::to_owned);
        let simulated_result = request_payload
            .get("simulateBidSkillResult")
            .or_else(|| request_payload.get("simulate_bid_skill_result"))
            .or_else(|| request_payload.get("simulatedBidSkillResult"))
            .or_else(|| request_payload.get("simulated_bid_skill_result"))
            .cloned();

        Ok(Self {
            bid_scope,
            company_profile,
            bid_documents,
            stage_run_id,
            idempotency_key,
            simulated_result,
        })
    }
}

#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone)]
struct BidSkillExecution {
    result_json: Value,
    result_path: PathBuf,
    screenshot_paths: Vec<String>,
    artifact_refs: Vec<String>,
    terminal_output_path: Option<String>,
    terminal_output_bytes: Option<i64>,
    terminal_output_hash: Option<String>,
    terminal_output_tail: Option<String>,
    side_effect_reached: SideEffectClass,
    portal_state_verified: bool,
    validation_gate: ValidatorGate,
    validation_override_logged: bool,
    submission_confirmed: bool,
    confirmation_ref: Option<String>,
    draft_evidence_recorded: bool,
}

struct BidResourceLease {
    lease: ResourceLease,
    release_cause: ReleaseCause,
}

impl BidResourceLease {
    const fn new(lease: ResourceLease) -> Self {
        Self {
            lease,
            release_cause: ReleaseCause::PreSpawnFailure,
        }
    }

    const fn set_release_cause(&mut self, release_cause: ReleaseCause) {
        self.release_cause = release_cause;
    }
}

fn acquire_bid_resource_lease(
    arbiter: &Mutex<ResourceArbiter>,
    job_id: &str,
    session_id: &str,
    portal_opportunity_ref: &str,
) -> Result<BidResourceLease> {
    let request = AdmissionRequest {
        job_id: job_id.to_owned(),
        session_id: Some(session_id.to_owned()),
        priority: AdmissionPriority::BidAutomation,
        browser_session: true,
        adapter_session: true,
        source_lane: None,
        validator_sub_job: false,
        concurrency_key: Some(format!("bid-portal/procontract:{portal_opportunity_ref}")),
        max_concurrency: Some(1),
    };
    let lease = {
        let mut arbiter = arbiter
            .lock()
            .map_err(|_| anyhow!("resource arbiter mutex poisoned"))?;
        match arbiter.admit(&request) {
            ArbiterDecision::Granted(lease) => lease,
            ArbiterDecision::Denied(reason) => {
                bail!(
                    "resource arbiter denied bid workflow admission for job {job_id}: {reason:?}"
                );
            }
        }
    };
    Ok(BidResourceLease::new(lease))
}

fn release_bid_resource_lease(
    arbiter: &Mutex<ResourceArbiter>,
    bid_resource_lease: Option<BidResourceLease>,
) -> Result<()> {
    let Some(bid_resource_lease) = bid_resource_lease else {
        return Ok(());
    };
    arbiter
        .lock()
        .map_err(|_| anyhow!("resource arbiter mutex poisoned"))?
        .release(&bid_resource_lease.lease, bid_resource_lease.release_cause);
    Ok(())
}

fn transition_bid_in_progress(scope: &BidScopePayload) -> Result<()> {
    let mut machine = BidStateMachine::new(BidWorkflowState::ReadyToBid);
    let mut context = TransitionContext::satisfied_for(
        BidWorkflowState::ReadyToBid,
        BidWorkflowState::BidInProgress,
    );
    context.trigger = TransitionTrigger::StartBid;
    context.side_effect_cap = scope.side_effect_cap;
    machine
        .transition_to(BidWorkflowState::BidInProgress, &context)
        .map(|_| ())
        .map_err(|error| anyhow!(error))
}

fn write_bid_skill_input(session_dir: &Path, skill_input: &Value) -> Result<()> {
    let workdir = session_dir.join("workdir");
    fs::create_dir_all(&workdir).with_context(|| format!("creating {}", workdir.display()))?;
    let path = workdir.join("bid-skill-input.json");
    fs::write(&path, serde_json::to_vec_pretty(skill_input)?)
        .with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

fn build_bid_skill_input(
    bid_scope: &BidScopePayload,
    company_profile: &CompanyProfile,
    bid_documents: &[BidDocument],
    quote_summaries: &[Value],
    skill_profile: &BidPortalSkillProfile,
    claimed: &ClaimedJob,
    stage_run_id: &str,
) -> Value {
    json!({
        "bidScopePayload": bid_scope,
        "companyProfile": company_profile,
        "bidDocuments": bid_documents,
        "selectedQuoteSummaries": quote_summaries,
        "portalOpportunityRef": bid_scope.portal_opportunity_ref,
        "skillBinding": {
            "skillId": skill_profile.skill_id,
            "skillVersion": skill_profile.skill_version,
            "interactionProfile": skill_profile.interaction_profile,
        },
        "runnerContext": {
            "jobId": claimed.id,
            "processId": bid_scope.process_id,
            "stageRunId": stage_run_id,
            "sideEffectCap": bid_scope.side_effect_cap,
            "reviewRequired": bid_scope.review_required,
        }
    })
}

fn load_selected_quote_summaries(
    conn: &rusqlite::Connection,
    bid_scope: &BidScopePayload,
) -> Result<Vec<Value>> {
    if bid_scope.selected_quote_ids.is_empty() {
        return Ok(Vec::new());
    }
    let placeholders = std::iter::repeat_n("?", bid_scope.selected_quote_ids.len())
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!(
        "\
        SELECT id, product_candidate_id, source_name, quote_class, currency,
               unit_price_gbp_pence, price_signal_role
        FROM supplier_quotes
        WHERE id IN ({placeholders})
        ORDER BY id
        "
    );
    let params = bid_scope
        .selected_quote_ids
        .iter()
        .map(|quote_id| quote_id as &dyn rusqlite::ToSql)
        .collect::<Vec<_>>();
    let mut stmt = conn
        .prepare(&sql)
        .context("preparing selected quote summary query")?;
    let rows = stmt.query_map(params.as_slice(), |row| {
        Ok(json!({
            "quoteId": row.get::<_, String>(0)?,
            "candidateId": row.get::<_, String>(1)?,
            "sourceName": row.get::<_, Option<String>>(2)?,
            "quoteClass": row.get::<_, Option<String>>(3)?,
            "currency": row.get::<_, Option<String>>(4)?,
            "unitPriceGbpPence": row.get::<_, Option<i64>>(5)?,
            "priceSignalRole": row.get::<_, Option<String>>(6)?,
        }))
    })?;
    rows.collect::<std::result::Result<Vec<_>, _>>()
        .context("loading selected quote summaries")
}

#[allow(clippy::future_not_send, clippy::too_many_arguments)]
async fn execute_bid_portal_skill(
    runtime: &RuntimeBootstrap,
    skill_profile: &BidPortalSkillProfile,
    claimed: &ClaimedJob,
    session_id: &str,
    started_at: chrono::DateTime<Utc>,
    session_dir: &Path,
    skill_input: &Value,
    side_effect_cap: SideEffectClass,
    simulated_result: Option<Value>,
) -> Result<BidSkillExecution> {
    if let Some(result_json) = simulated_result {
        return materialize_simulated_bid_execution(
            runtime,
            skill_profile,
            claimed,
            session_id,
            started_at,
            session_dir,
            skill_input,
            side_effect_cap,
            result_json,
        );
    }

    let skill_path = crate::repo_root()?.join(BID_PORTAL_SKILL_PATH);
    let skill = crate::load_skill_from_path(&skill_path)?;
    let cli = default_bid_runner_cli(runtime);
    let executed =
        crate::execute_skill_session(runtime, &skill, &claimed.id, session_id, skill_input, &cli)
            .await?;
    crate::insert_executed_session_log(
        runtime.state_db.connection(),
        &skill,
        &claimed.id,
        &executed,
    )?;
    let result_path = executed
        .result_path
        .as_ref()
        .ok_or_else(|| anyhow!("bid portal skill did not write a result contract"))?;
    let result_json = crate::read_result_contract(result_path)?;
    let screenshot_paths = extract_screenshot_paths(&result_json);
    let side_effect_reached = result_side_effect_class(&result_json)?;
    ensure_observed_side_effect_allowed(side_effect_reached, side_effect_cap)?;
    let confirmation_ref = confirmation_ref_from_result(&result_json);
    let submission_confirmed =
        submission_confirmed_from_result(&result_json, confirmation_ref.as_deref());
    let portal_state_verified = portal_state_verified_from_result(&result_json);
    let validation_gate = validation_gate_from_result(&result_json);
    let validation_override_logged = validation_override_logged_from_result(&result_json);
    let draft_evidence_recorded =
        draft_evidence_recorded_from_result(&result_json, side_effect_reached);
    Ok(BidSkillExecution {
        result_json,
        result_path: result_path.clone(),
        screenshot_paths,
        artifact_refs: executed.artifact_refs,
        terminal_output_path: executed.terminal_output_path,
        terminal_output_bytes: executed.terminal_output_bytes,
        terminal_output_hash: executed.terminal_output_hash,
        terminal_output_tail: executed.terminal_output_tail,
        side_effect_reached,
        portal_state_verified,
        validation_gate,
        validation_override_logged,
        submission_confirmed,
        confirmation_ref,
        draft_evidence_recorded,
    })
}

fn default_bid_runner_cli(runtime: &RuntimeBootstrap) -> crate::Cli {
    let runtime_dir = Path::new(&runtime.descriptor.socket_path)
        .parent()
        .map_or_else(|| Path::new("runtime").to_path_buf(), Path::to_path_buf);
    crate::Cli {
        dry_run: false,
        state_root: Path::new(&runtime.descriptor.state_root).to_path_buf(),
        runtime_dir,
        artifact_root: runtime.artifact_root.clone(),
        serve: false,
        serve_once: false,
        ingest: None,
        skill_path: None,
        job_parameters_json: None,
        adapter_choice: None,
        agent_cli_program: None,
        fixture_path: None,
        pipeline: None,
        run_next_job: false,
        run_bid_workflow: None,
        capture_decision: None,
        disposition: None,
        idempotency_key: None,
        reasoning: None,
        capture_clarification: None,
        candidate_id: None,
        reason_code: None,
        payload: None,
        note: None,
        withdraw_clarification: None,
        withdraw_decision: None,
        list_decisions: false,
        process_id: None,
        list_status_filter: None,
    }
}

#[allow(clippy::too_many_arguments)]
fn materialize_simulated_bid_execution(
    runtime: &RuntimeBootstrap,
    skill_profile: &BidPortalSkillProfile,
    claimed: &ClaimedJob,
    session_id: &str,
    started_at: chrono::DateTime<Utc>,
    session_dir: &Path,
    skill_input: &Value,
    side_effect_cap: SideEffectClass,
    mut result_json: Value,
) -> Result<BidSkillExecution> {
    let screenshots = materialize_screenshots(session_dir, &mut result_json)?;
    let result_path = session_dir.join("result.json");
    fs::write(&result_path, serde_json::to_vec_pretty(&result_json)?)
        .with_context(|| format!("writing {}", result_path.display()))?;

    let mut capture = PtyCapture::new(
        &runtime.artifact_root,
        session_id,
        started_at,
        CaptureConfig::default(),
    )?;
    let terminal = serde_json::to_vec(&json!({
        "event": "simulated_bid_portal_run",
        "jobId": claimed.id,
        "skillInputHash": sha256_hex(serde_json::to_string(skill_input)?.as_bytes()),
        "result": result_json,
    }))?;
    capture.append(1, &terminal, Utc::now())?;
    let stream_index = capture.finalize()?;
    let terminal_output_path = session_dir.join("terminal-output.txt");
    fs::write(&terminal_output_path, &terminal)
        .with_context(|| format!("writing {}", terminal_output_path.display()))?;
    let terminal_output_hash = sha256_hex(&terminal);
    let terminal_output_tail = String::from_utf8_lossy(&terminal).into_owned();
    let terminal_output_bytes =
        i64::try_from(terminal.len()).context("converting terminal output size")?;

    insert_synthetic_bid_session_log(
        runtime,
        skill_profile,
        claimed,
        session_id,
        started_at,
        session_dir,
        &terminal_output_path,
        &terminal_output_hash,
        terminal_output_bytes,
        &terminal_output_tail,
        stream_index.len(),
    )?;

    let side_effect_reached = result_side_effect_class(&result_json)?;
    ensure_observed_side_effect_allowed(side_effect_reached, side_effect_cap)?;
    let confirmation_ref = confirmation_ref_from_result(&result_json);
    let submission_confirmed =
        submission_confirmed_from_result(&result_json, confirmation_ref.as_deref());
    let portal_state_verified = portal_state_verified_from_result(&result_json);
    let validation_gate = validation_gate_from_result(&result_json);
    let validation_override_logged = validation_override_logged_from_result(&result_json);
    let draft_evidence_recorded =
        draft_evidence_recorded_from_result(&result_json, side_effect_reached);
    let stream_artifact_refs = stream_index
        .into_iter()
        .map(|entry| stream_chunk_artifact_ref(session_dir, &entry.chunk))
        .collect::<Result<Vec<_>>>()?;

    Ok(BidSkillExecution {
        result_json,
        result_path,
        screenshot_paths: screenshots.clone(),
        artifact_refs: screenshots
            .into_iter()
            .chain(std::iter::once(
                terminal_output_path.to_string_lossy().into_owned(),
            ))
            .chain(stream_artifact_refs)
            .collect(),
        terminal_output_path: Some(terminal_output_path.to_string_lossy().into_owned()),
        terminal_output_bytes: Some(terminal_output_bytes),
        terminal_output_hash: Some(terminal_output_hash),
        terminal_output_tail: Some(terminal_output_tail),
        side_effect_reached,
        portal_state_verified,
        validation_gate,
        validation_override_logged,
        submission_confirmed,
        confirmation_ref,
        draft_evidence_recorded,
    })
}

fn stream_chunk_artifact_ref(session_dir: &Path, chunk: &str) -> Result<String> {
    let chunk_path = Path::new(chunk);
    ensure!(
        !chunk_path.as_os_str().is_empty(),
        "PTY stream chunk path must stay relative to the session directory: {chunk}"
    );
    let mut resolved = session_dir.to_path_buf();
    for component in chunk_path.components() {
        let std::path::Component::Normal(segment) = component else {
            bail!("PTY stream chunk path must stay relative to the session directory: {chunk}");
        };
        resolved.push(segment);
    }
    Ok(resolved.to_string_lossy().into_owned())
}

fn ensure_observed_side_effect_allowed(
    observed: SideEffectClass,
    cap: SideEffectClass,
) -> Result<()> {
    match enforce_procontract_observed_side_effect(observed, cap) {
        CapDecision::Allowed => Ok(()),
        CapDecision::SaveDraftAndStop { reason }
        | CapDecision::RouteToPortalStateUnknown { reason } => {
            bail!(
                "{}: observed bid portal side-effect {:?} exceeded cap {:?}",
                reason.as_str(),
                observed,
                cap
            )
        }
    }
}

fn confirmation_ref_from_result(result_json: &Value) -> Option<String> {
    result_json
        .pointer("/submissionResult/confirmationRef")
        .or_else(|| result_json.pointer("/submission_result/confirmation_ref"))
        .or_else(|| result_json.get("confirmationRef"))
        .or_else(|| result_json.get("confirmation_ref"))
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(str::to_owned)
}

fn submission_confirmed_from_result(result_json: &Value, confirmation_ref: Option<&str>) -> bool {
    result_json
        .pointer("/submissionResult/submitted")
        .or_else(|| result_json.pointer("/submission_result/submitted"))
        .or_else(|| result_json.get("submitted"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
        && confirmation_ref.is_some_and(|value| !value.trim().is_empty())
}

fn portal_state_verified_from_result(result_json: &Value) -> bool {
    result_json
        .get("portalStateVerified")
        .or_else(|| result_json.get("portal_state_verified"))
        .and_then(Value::as_bool)
        .unwrap_or(true)
}

fn validation_gate_from_result(result_json: &Value) -> ValidatorGate {
    if result_json
        .get("validationApproved")
        .or_else(|| result_json.get("validation_approved"))
        .and_then(Value::as_bool)
        .unwrap_or(true)
    {
        ValidatorGate::Approved
    } else {
        ValidatorGate::Rejected
    }
}

fn validation_override_logged_from_result(result_json: &Value) -> bool {
    result_json
        .get("validationOverrideLogged")
        .or_else(|| result_json.get("validation_override_logged"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn draft_evidence_recorded_from_result(
    result_json: &Value,
    side_effect_reached: SideEffectClass,
) -> bool {
    result_json
        .pointer("/draftSaveResult/draftSaved")
        .or_else(|| result_json.pointer("/draft_save_result/draft_saved"))
        .or_else(|| result_json.get("draftSaved"))
        .or_else(|| result_json.get("draft_saved"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
        || side_effect_reached >= SideEffectClass::RemoteDraft
}

fn materialize_screenshots(session_dir: &Path, result_json: &mut Value) -> Result<Vec<String>> {
    let mut screenshots = extract_screenshot_paths(result_json);
    if screenshots.is_empty() {
        screenshots = vec![
            session_dir
                .join("step-01.webp")
                .to_string_lossy()
                .into_owned(),
            session_dir
                .join("step-02.webp")
                .to_string_lossy()
                .into_owned(),
            session_dir
                .join("step-03.webp")
                .to_string_lossy()
                .into_owned(),
        ];
    }
    for screenshot in &screenshots {
        let path = Path::new(screenshot);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
        }
        if !path.exists() {
            fs::write(path, b"RIFF\x10\x00\x00\x00WEBPVP8 \x00\x00\x00\x00")
                .with_context(|| format!("writing {}", path.display()))?;
        }
    }
    result_json
        .as_object_mut()
        .context("bid skill result must be a JSON object")?
        .insert("screenshotPaths".to_owned(), json!(screenshots));
    Ok(screenshots)
}

fn extract_screenshot_paths(result_json: &Value) -> Vec<String> {
    result_json
        .get("screenshotPaths")
        .or_else(|| result_json.get("screenshot_paths"))
        .or_else(|| result_json.pointer("/submissionResult/screenshotPaths"))
        .or_else(|| result_json.pointer("/submission_result/screenshot_paths"))
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

fn final_state_for_skill_result(
    result_json: &Value,
    side_effect_cap: SideEffectClass,
) -> Result<BidWorkflowState> {
    let side_effect_reached = result_side_effect_class(result_json)?;
    match enforce_procontract_observed_side_effect(side_effect_reached, side_effect_cap) {
        CapDecision::Allowed => {}
        CapDecision::SaveDraftAndStop { .. } => return Ok(BidWorkflowState::DraftSaved),
        CapDecision::RouteToPortalStateUnknown { .. } => {
            return Ok(BidWorkflowState::PortalStateUnknown);
        }
    }
    if result_json
        .pointer("/submissionResult/submitted")
        .or_else(|| result_json.get("submitted"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        Ok(BidWorkflowState::BidSubmitted)
    } else if result_json
        .pointer("/draftSaveResult/draftSaved")
        .or_else(|| result_json.get("draftSaved"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        Ok(BidWorkflowState::DraftSaved)
    } else if result_json
        .get("portalStateUnknown")
        .or_else(|| result_json.get("portal_state_unknown"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        Ok(BidWorkflowState::PortalStateUnknown)
    } else {
        Ok(BidWorkflowState::BidFailed)
    }
}

fn result_side_effect_class(result_json: &Value) -> Result<SideEffectClass> {
    result_json
        .get("finalSideEffectClass")
        .or_else(|| result_json.get("final_side_effect_class"))
        .or_else(|| result_json.pointer("/submissionResult/finalSideEffectClass"))
        .or_else(|| result_json.pointer("/draftSaveResult/sideEffectClass"))
        .and_then(Value::as_str)
        .and_then(parse_side_effect_class)
        .ok_or_else(|| anyhow!("bid skill result missing final side-effect class"))
}

fn parse_side_effect_class(value: &str) -> Option<SideEffectClass> {
    match value {
        "READ_ONLY" | "read_only" => Some(SideEffectClass::ReadOnly),
        "REMOTE_DRAFT" | "remote_draft" => Some(SideEffectClass::RemoteDraft),
        "NOTIFICATION_OR_LOCK" | "notification_or_lock" => {
            Some(SideEffectClass::NotificationOrLock)
        }
        "LEGAL_DECLARATION" | "legal_declaration" => Some(SideEffectClass::LegalDeclaration),
        "FORMAL_SUBMISSION" | "formal_submission" => Some(SideEffectClass::FormalSubmission),
        "FUNDS_COMMITTED" | "funds_committed" => Some(SideEffectClass::FundsCommitted),
        _ => None,
    }
}

#[allow(clippy::too_many_arguments)]
fn insert_synthetic_bid_session_log(
    runtime: &RuntimeBootstrap,
    skill_profile: &BidPortalSkillProfile,
    claimed: &ClaimedJob,
    session_id: &str,
    started_at: chrono::DateTime<Utc>,
    session_dir: &Path,
    terminal_output_path: &Path,
    terminal_output_hash: &str,
    terminal_output_bytes: i64,
    terminal_output_tail: &str,
    stream_chunk_count: usize,
) -> Result<()> {
    runtime.state_db.connection().execute(
        "\
        INSERT INTO agent_session_logs (
          id, job_id, skill_path, skill_version, execution_mode, cli_vendor, cli_version,
          adapter_id, adapter_version, config_hash, pty_session_id, sid, pid, pgid, started_at,
          completed_at, status, terminal_output_path, terminal_output_hash,
          terminal_output_bytes, terminal_output_tail, structured_trace, error_type,
          error_message, exit_code, cleanup_verdict, last_checkpoint_seq
        ) VALUES (?1, ?2, ?3, ?4, 'pty', 'daemon-bid-runner', NULL,
                  'synthetic-pty', ?5, ?6, ?1, NULL, NULL, NULL, ?7,
                  ?8, 'completed', ?9, ?10, ?11, ?12, ?13, NULL,
                  NULL, 0, 'clean', ?14)
        ON CONFLICT(id) DO UPDATE SET
          completed_at = excluded.completed_at,
          status = excluded.status,
          terminal_output_path = excluded.terminal_output_path,
          terminal_output_hash = excluded.terminal_output_hash,
          terminal_output_bytes = excluded.terminal_output_bytes,
          terminal_output_tail = excluded.terminal_output_tail,
          structured_trace = excluded.structured_trace,
          exit_code = excluded.exit_code,
          cleanup_verdict = excluded.cleanup_verdict,
          last_checkpoint_seq = excluded.last_checkpoint_seq
        ",
        params![
            session_id,
            claimed.id,
            format!("{BID_PORTAL_SKILL_PATH}/SKILL.md"),
            skill_profile.skill_version,
            env!("CARGO_PKG_VERSION"),
            sha256_hex(session_dir.to_string_lossy().as_bytes()),
            started_at.to_rfc3339(),
            Utc::now().to_rfc3339(),
            terminal_output_path.to_string_lossy(),
            terminal_output_hash,
            terminal_output_bytes,
            terminal_output_tail,
            json!({
                "execution_mode": "pty",
                "synthetic_bid_portal_run": true,
                "stream_chunk_count": stream_chunk_count,
                "resource_priority": "bid_automation",
            })
            .to_string(),
            u32::try_from(stream_chunk_count).context("stream chunk count exceeds u32")?,
        ],
    )?;
    Ok(())
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn write_bid_run_manifest(
    runtime: &RuntimeBootstrap,
    skill_profile: &BidPortalSkillProfile,
    claimed: &ClaimedJob,
    session_id: &str,
    started_at: chrono::DateTime<Utc>,
    bid_scope: &BidScopePayload,
    stage_run_id: &str,
    final_state: BidWorkflowState,
    execution: &BidSkillExecution,
    start_readiness: &BidReadinessGateResult,
    final_readiness: &BidReadinessGateResult,
) -> Result<crate::agents::artifacts::StoredManifest> {
    let outcome_class = match final_state {
        BidWorkflowState::DraftSaved
        | BidWorkflowState::BidSubmitted
        | BidWorkflowState::PortalStateUnknown => OutcomeClass::Completed,
        BidWorkflowState::BidFailed
        | BidWorkflowState::NotStarted
        | BidWorkflowState::ReadyToBid
        | BidWorkflowState::BidInProgress => OutcomeClass::Failed,
    };
    let narrative = RunNarrative {
        summary: format!(
            "Bid workflow reached {} for the scoped Procontract run.",
            final_state.as_str()
        ),
        steps_taken:
            "Daemon recorded portal automation checkpoints, screenshots, and forensic PTY output."
                .to_owned(),
        decisions_made: format!(
            "Side-effect cap, review requirement, and scoped identifiers are recorded in the daemon evidence appendix for stageRunId={stage_run_id}."
        ),
        warnings: if matches!(final_state, BidWorkflowState::PortalStateUnknown) {
            "Portal state requires operator reconciliation before retry.".to_owned()
        } else {
            "None.".to_owned()
        },
        evidence:
            "Use daemon evidence appendix artifact refs for screenshots, result contract, and PTY capture."
                .to_owned(),
    };
    let (items_failed, warnings_count) = bid_manifest_counts(final_state);
    let side_effect_progression = side_effect_progression(execution, started_at);
    let readiness_evidence = ReadinessEvidenceSnapshot {
        gates_evaluated_at_start: readiness_gate_results(start_readiness),
        gates_re_evaluated_at_final_submit: readiness_gate_results(final_readiness),
        decision_basis_hash_at_start: bid_scope.decision_basis_hash.clone(),
        decision_basis_hash_at_final_submit: Some(bid_scope.decision_basis_hash.clone()),
    };
    write_hybrid_run_manifest(ManifestWriteRequest {
        artifact_root: &runtime.artifact_root,
        started_at,
        frontmatter: RunManifestFrontmatter {
            manifest_version: 1,
            skill_id: skill_profile.skill_id.clone(),
            skill_version: skill_profile.skill_version.clone(),
            job_id: claimed.id.clone(),
            session_id: session_id.to_owned(),
            execution_mode: "pty".to_owned(),
            started_at: started_at.to_rfc3339(),
            completed_at: Utc::now().to_rfc3339(),
            outcome_class,
            items_processed: 1,
            items_skipped: 0,
            items_failed,
            warnings_count,
            agent_claimed_pages_visited: u32::try_from(execution.screenshot_paths.len())
                .unwrap_or(u32::MAX),
            agent_claimed_source_url: format!(
                "https://procontract.due-north.com/Opportunities/Index?ref={}",
                bid_scope.portal_opportunity_ref
            ),
            agent_claimed_fingerprint_state: "procontract_bid_portal".to_owned(),
            agent_claimed_locale_state: "en-GB".to_owned(),
            agent_claimed_currency_state: "GBP".to_owned(),
            agent_narrative_missing: false,
        },
        narrative: Some(narrative),
        appendix: DaemonEvidenceAppendix {
            observed_url_chain: vec![format!(
                "https://procontract.due-north.com/Opportunities/Index?ref={}",
                bid_scope.portal_opportunity_ref
            )],
            observed_page_kind_sequence: vec![
                "procontract_opportunity".to_owned(),
                final_state.as_str().to_owned(),
            ],
            daemon_observed_fingerprint_state: "procontract_bid_portal".to_owned(),
            daemon_observed_locale: "en-GB".to_owned(),
            daemon_observed_currency: "GBP".to_owned(),
            auth_challenge_events: Vec::new(),
            locator_fallback_depth: 0,
            basis_sufficiency: "high".to_owned(),
            page_fingerprint_hash: Some(sha256_hex(bid_scope.portal_opportunity_ref.as_bytes())),
            auth_challenge_detected: false,
            extraction_fill_rate_bps: Some(10_000),
            checkpoint_count: u32::try_from(execution.screenshot_paths.len()).unwrap_or(u32::MAX),
            byte_count_compressed: execution
                .terminal_output_bytes
                .and_then(|bytes| u64::try_from(bytes).ok()),
            validator_rubric_hash: None,
            drift_delta_json: Value::Null,
            result_item_count: 1,
            oqg_anomaly_summary: vec![format!(
                "side_effect_reached={:?}; side_effect_cap={:?}",
                execution.side_effect_reached, bid_scope.side_effect_cap
            )],
            representative_item_sample: json!({
                "process_id_hash": stable_id_hash("process", &bid_scope.process_id),
                "portal_opportunity_ref_hash": stable_id_hash("portal_ref", &bid_scope.portal_opportunity_ref),
                "bid_state": final_state.as_str(),
            }),
            artifact_refs: execution.artifact_refs.clone(),
            bid_scope_summary: Some(BidScopeSummary {
                process_id_hash: stable_id_hash("process", &bid_scope.process_id),
                lot_id_hashes: hash_id_list("lot", &bid_scope.lot_ids),
                candidate_id_hashes: hash_id_list("candidate", &bid_scope.candidate_ids),
                selected_quote_id_hashes: hash_id_list("quote", &bid_scope.selected_quote_ids),
                portal_opportunity_ref_hash: stable_id_hash(
                    "portal_ref",
                    &bid_scope.portal_opportunity_ref,
                ),
                decision_basis_hash: bid_scope.decision_basis_hash.clone(),
                side_effect_cap: format!("{:?}", bid_scope.side_effect_cap),
                review_required: bid_scope.review_required,
            }),
            side_effect_progression,
            readiness_evidence: Some(readiness_evidence),
            validator_verdict: Some(ValidatorVerdictSummary {
                verdict: validation_verdict_label(execution).to_owned(),
                rubric_version: "validation/overlay-bid-portal-procontract@pending".to_owned(),
                reasons: validation_verdict_reasons(execution),
            }),
            override_metadata: None,
            final_bid_state: Some(final_state.as_str().to_owned()),
        },
        disagreements: Vec::new(),
        missing_narrative_reason: "bid workflow narrative was provided",
    })
}

const fn bid_manifest_counts(final_state: BidWorkflowState) -> (u32, u32) {
    match final_state {
        BidWorkflowState::BidFailed => (1, 0),
        BidWorkflowState::PortalStateUnknown => (0, 1),
        BidWorkflowState::NotStarted
        | BidWorkflowState::ReadyToBid
        | BidWorkflowState::BidInProgress
        | BidWorkflowState::DraftSaved
        | BidWorkflowState::BidSubmitted => (0, 0),
    }
}

fn side_effect_progression(
    execution: &BidSkillExecution,
    started_at: chrono::DateTime<Utc>,
) -> Vec<SideEffectCheckpoint> {
    let checkpoint_count = execution.screenshot_paths.len();
    execution
        .screenshot_paths
        .iter()
        .enumerate()
        .map(|(index, screenshot_path)| {
            let side_effect_class = if index + 1 == checkpoint_count {
                format!("{:?}", execution.side_effect_reached)
            } else {
                "ReadOnly".to_owned()
            };
            SideEffectCheckpoint {
                step: format!("{:02}", index + 1),
                side_effect_class,
                screenshot_path: screenshot_path.clone(),
                timestamp: started_at.to_rfc3339(),
            }
        })
        .collect()
}

fn readiness_gate_results(readiness: &BidReadinessGateResult) -> Vec<GateResult> {
    if readiness.approved {
        return vec![GateResult {
            code: "ALL_READINESS_GATES_PASSED".to_owned(),
            source: "bid_workflow_readiness_gate".to_owned(),
            passed: true,
            overrideable: false,
        }];
    }
    readiness
        .blockers
        .iter()
        .map(|blocker| GateResult {
            code: blocker.code().to_owned(),
            source: blocker.source().to_owned(),
            passed: false,
            overrideable: blocker.overrideable(),
        })
        .collect()
}

fn hash_id_list(label: &str, values: &[String]) -> Vec<String> {
    values
        .iter()
        .map(|value| stable_id_hash(label, value))
        .collect()
}

fn stable_id_hash(label: &str, value: &str) -> String {
    sha256_hex(format!("{label}:{value}").as_bytes())
}

const fn validation_verdict_label(execution: &BidSkillExecution) -> &'static str {
    match execution.validation_gate {
        ValidatorGate::Approved => "approved",
        ValidatorGate::Rejected => "rejected",
        ValidatorGate::Outage => "outage",
        ValidatorGate::Missing => "missing",
        ValidatorGate::NotRequired => "not_required",
    }
}

fn validation_verdict_reasons(execution: &BidSkillExecution) -> Vec<String> {
    let mut reasons = Vec::new();
    reasons.push(format!(
        "portal_state_verified={}",
        execution.portal_state_verified
    ));
    reasons.push(format!(
        "side_effect_reached={:?}",
        execution.side_effect_reached
    ));
    if execution.validation_override_logged {
        reasons.push("validation_override_logged=true".to_owned());
    }
    reasons
}

fn final_transition_context(
    from: BidWorkflowState,
    to: BidWorkflowState,
    side_effect_cap: SideEffectClass,
    execution: &BidSkillExecution,
) -> TransitionContext {
    let mut context = TransitionContext::satisfied_for(from, to);
    context.side_effect_reached = Some(execution.side_effect_reached);
    context.side_effect_cap = side_effect_cap;
    context.draft_evidence_recorded = execution.draft_evidence_recorded;
    context.screenshots_recorded = !execution.screenshot_paths.is_empty();
    context.run_manifest_linked = true;
    context.portal_state_verified = execution.portal_state_verified;
    context.validator_gate = execution.validation_gate;
    context.validation_override_logged = execution.validation_override_logged;
    context.submission_confirmed = execution.submission_confirmed;
    context
        .confirmation_ref
        .clone_from(&execution.confirmation_ref);
    context.error_manifest_recorded = true;
    context
}

#[allow(clippy::too_many_lines)]
fn persist_bid_state_transition(
    runtime: &mut RuntimeBootstrap,
    claimed: &ClaimedJob,
    bid_scope: &BidScopePayload,
    target: BidWorkflowState,
    stage_run_id: &str,
    output_payload: Option<&Value>,
) -> Result<()> {
    let schema_epoch =
        i32::try_from(runtime.state_db.schema_epoch()).context("schema_epoch exceeds i32")?;
    let bid_scope_json = serde_json::to_string(bid_scope).context("serializing BidScopePayload")?;
    let now = Utc::now().to_rfc3339();
    let final_state = matches!(
        target,
        BidWorkflowState::DraftSaved
            | BidWorkflowState::BidSubmitted
            | BidWorkflowState::BidFailed
            | BidWorkflowState::PortalStateUnknown
    );
    let output_payload_text = output_payload.map(Value::to_string);
    runtime.state_db.with_write_transaction(|tx| {
        let existing: Option<i64> = tx
            .query_row(
                "SELECT row_version FROM bid_workflow_states WHERE process_id = ?1",
                params![bid_scope.process_id],
                |row| row.get(0),
            )
            .optional()?;
        let op = if existing.is_some() {
            "update"
        } else {
            "insert"
        };
        tx.execute(
            "\
            INSERT INTO bid_workflow_states (
              process_id, bid_state, bid_scope_json, review_required, bid_job_id,
              started_at, completed_at, owner_id, row_version, stage_run_id, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 1, ?9, ?10)
            ON CONFLICT(process_id) DO UPDATE SET
              bid_state = excluded.bid_state,
              bid_scope_json = excluded.bid_scope_json,
              review_required = excluded.review_required,
              bid_job_id = excluded.bid_job_id,
              started_at = COALESCE(bid_workflow_states.started_at, excluded.started_at),
              completed_at = excluded.completed_at,
              row_version = bid_workflow_states.row_version + 1,
              stage_run_id = excluded.stage_run_id,
              updated_at = excluded.updated_at
            ",
            params![
                bid_scope.process_id,
                target.as_str(),
                bid_scope_json,
                i64::from(bid_scope.review_required),
                claimed.id,
                now,
                final_state.then_some(now.as_str()),
                DEFAULT_OWNER_ID,
                stage_run_id,
                now,
            ],
        )?;
        let row_version: i64 = tx.query_row(
            "SELECT row_version FROM bid_workflow_states WHERE process_id = ?1",
            params![bid_scope.process_id],
            |row| row.get(0),
        )?;
        let payload_json = json!({
            "process_id": bid_scope.process_id,
            "bid_state": target.as_str(),
            "bid_scope_json": bid_scope_json,
            "review_required": bid_scope.review_required,
            "bid_job_id": claimed.id,
            "owner_id": DEFAULT_OWNER_ID,
            "row_version": row_version,
            "stage_run_id": stage_run_id,
        });
        let content_hash = canonical_content_hash(&payload_json)?;
        tx.execute(
            "\
            INSERT INTO local_outbox (
              executor_id, entity_kind, entity_id, op, row_version, stage_run_id,
              schema_epoch, commit_group_id, payload_json, content_hash
            ) VALUES (?1, 'bid_workflow_states', ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            ",
            params![
                "local-daemon",
                bid_scope.process_id,
                op,
                row_version,
                stage_run_id,
                schema_epoch,
                format!("bid-workflow:{}:{}", claimed.id, target.as_str()),
                payload_json.to_string(),
                content_hash,
            ],
        )?;
        let local_seq = tx.last_insert_rowid();
        tx.execute(
            "UPDATE bid_workflow_states SET last_local_seq = ?1 WHERE process_id = ?2",
            params![local_seq, bid_scope.process_id],
        )?;
        if final_state {
            let payload = output_payload_text.unwrap_or_else(|| {
                json!({
                    "error_code": ErrorCode::BidSubmissionFailed.as_str(),
                    "bid_state": target.as_str(),
                })
                .to_string()
            });
            tx.execute(
                "\
                UPDATE workflow_jobs
                SET status = ?1,
                    completed_at = ?2,
                    output_payload = ?3
                WHERE id = ?4
                ",
                params![
                    if target == BidWorkflowState::BidFailed {
                        "failed"
                    } else {
                        "completed"
                    },
                    Utc::now().to_rfc3339(),
                    payload,
                    claimed.id,
                ],
            )?;
        }
        Ok(())
    })?;
    Ok(())
}

fn fail_bid_workflow_job(
    runtime: &mut RuntimeBootstrap,
    claimed: &ClaimedJob,
    error_code: ErrorCode,
    message: &str,
    mut payload: Value,
) -> Result<LiveLoopReport> {
    payload
        .as_object_mut()
        .context("bid workflow failure payload must be a JSON object")?
        .insert("error_code".to_owned(), json!(error_code.as_str()));
    runtime.state_db.with_write_transaction(|tx| {
        tx.execute(
            "\
            UPDATE workflow_jobs
            SET status = 'failed',
                last_error = ?1,
                completed_at = ?2,
                output_payload = ?3
            WHERE id = ?4
            ",
            params![
                format!("{}: {message}", error_code.as_str()),
                Utc::now().to_rfc3339(),
                payload.to_string(),
                claimed.id,
            ],
        )?;
        Ok(())
    })?;
    Ok(LiveLoopReport {
        summary_line: format!(
            "Bid workflow failed: job_id={} reason={}: {message}",
            claimed.id,
            error_code.as_str()
        ),
        status: "failed".to_owned(),
    })
}

fn canonical_content_hash(value: &Value) -> Result<String> {
    let canonical = serde_json::to_string(value).context("serializing canonical outbox payload")?;
    Ok(sha256_hex(canonical.as_bytes()))
}

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
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

pub fn ensure_planned_bid_portal_side_effects_within_cap(
    bid_scope: &BidScopePayload,
) -> Result<()> {
    for step in planned_procontract_steps(bid_scope.review_required, bid_scope.side_effect_cap) {
        match enforce_procontract_step_cap(step, bid_scope.side_effect_cap) {
            CapDecision::Allowed => {}
            CapDecision::SaveDraftAndStop { reason }
            | CapDecision::RouteToPortalStateUnknown { reason } => {
                bail!(
                    "{}: bid portal step {} declares {:?}, exceeding sideEffectCap {:?}",
                    reason.as_str(),
                    step.step_id(),
                    step.side_effect_class(),
                    bid_scope.side_effect_cap
                );
            }
        }
    }

    Ok(())
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
        ensure_one_workflow_job_updated(updated, &claimed.id)?;
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
        ensure_one_workflow_job_updated(updated, &claimed.id)?;
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

fn ensure_one_workflow_job_updated(updated: usize, job_id: &str) -> Result<()> {
    match updated {
        1 => Ok(()),
        0 => bail!("workflow job missing for {job_id}"),
        row_count => bail!("workflow job update affected {row_count} rows for {job_id}"),
    }
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
        ensure_bid_portal_pty_profile, ensure_planned_bid_portal_side_effects_within_cap,
        initial_bid_in_progress_transition, load_bid_portal_skill_profile, parse_bid_scope_payload,
    };
    use crate::bid_workflow::state_machine::BidTransition;
    use crate::bid_workflow::types::{BidWorkflowState, SideEffectClass};

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

    #[test]
    fn l6_t_daemon_runner_004_wires_planned_step_cap_guard() {
        let mut scope = parse_bid_scope_payload(&valid_bid_scope_payload())
            .expect("valid scope payload parses");
        ensure_planned_bid_portal_side_effects_within_cap(&scope)
            .expect("default remote draft path is within cap");

        scope.side_effect_cap = SideEffectClass::ReadOnly;
        let error = ensure_planned_bid_portal_side_effects_within_cap(&scope)
            .expect_err("read-only cap blocks remote draft steps");
        assert!(error.to_string().contains("BID_SIDE_EFFECT_CAP_EXCEEDED"));
        assert!(error.to_string().contains("L6-FR-1.7.12"));
    }
}
