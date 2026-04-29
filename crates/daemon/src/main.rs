#![forbid(unsafe_code)]

mod db {
    pub mod sqlite;
}

mod ipc {
    pub mod command_router;
    pub mod descriptor;
    pub mod handlers {
        pub mod control;
        pub mod decisions;
        pub mod errors;
        pub mod jobs;
        pub mod sessions;
    }
    pub mod handshake;
}

#[allow(dead_code)]
mod agents {
    pub mod adapter;
    pub mod arbiter;
    pub mod artifacts;
    pub mod claude;
    pub mod codex;
    pub mod contracts;
}

#[allow(dead_code)]
mod errors {
    pub mod manifest;
    pub mod markdown;
}

#[allow(dead_code)]
mod jobs {
    pub mod claim;
    pub mod runner;
}

#[allow(dead_code)]
mod pty {
    pub mod capture;
    pub mod health;
    pub mod session;
}

#[allow(dead_code)]
mod prompt {
    pub mod renderer;
}

#[allow(dead_code)]
mod pdf {
    pub mod chunking;
    pub mod engine;
    pub mod policy;
}

#[allow(dead_code)]
mod scheduler {
    pub mod cron;
}

#[allow(dead_code)]
mod sessions {
    pub mod reaper;
}

#[allow(dead_code)]
mod tracing {
    pub mod forensic;
    pub mod structured;
}

#[allow(dead_code)]
mod sources {
    pub mod registry;
}

#[allow(dead_code)]
mod validation {
    pub mod executor;
    pub mod manifest_writer;
    pub mod policy;
}

#[cfg(test)]
use crate::agents::adapter as adapter_impl;
#[cfg(test)]
use crate::agents::contracts as contracts_impl;
#[cfg(test)]
use crate::errors::manifest as errors_manifest_impl;
#[cfg(test)]
use crate::errors::markdown as errors_markdown_impl;

#[cfg(test)]
mod support {
    pub mod db {
        pub use crate::db::sqlite::StateDb;
    }
}

use std::collections::BTreeMap;
use std::convert::TryFrom;
use std::ffi::OsString;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process;
use std::process::Stdio;
use std::sync::Mutex;
use std::time::Duration;

use agents::adapter::{
    AgentAdapter, CapabilityReport, ControlEvent, ExecutionMode, InteractionProfile, LaunchSpec,
    OutputFrame, ProbeCtx, SkillManifest, SpawnRequest,
};
use agents::arbiter::{
    AdmissionPriority, AdmissionRequest, ArbiterDecision, ArbiterEvent, ArbiterEventKind,
    DenialReason, ReleaseCause, ResourceArbiter, ResourceBudgets, ResourceLease, SourceKind,
    SourceLaneRequest,
};
use agents::artifacts::{
    ArtifactStore, DaemonEvidenceAppendix, OutcomeClass, RunManifestFrontmatter, RunNarrative,
    persist_run_manifest_metadata,
};
use agents::claude::ClaudeAdapter;
use std::collections::BTreeSet;

use agents::codex::CodexAdapter;
use agents::contracts::{
    OqgCheckInput, StageValidationWrite, ValidationDisposition, ValidationOutcome,
    persist_stage_validation, validate_final_result,
};
use anyhow::{Context, Result, anyhow, bail, ensure};
use bytes::Bytes;
use chrono::{DateTime, Utc};
use clap::Parser;
use db::sqlite::{DEFAULT_OWNER_ID, DEFAULT_SCHEMA_EPOCH, StateDb, open_state_db};
use errors::manifest::{ErrorCode, ErrorManifestDraft, ErrorManifestStore, ErrorRuntimeContext};
use ipc::descriptor::{
    DaemonDescriptor, LockGuard, RuntimePaths, bind_listener, write_descriptor_atomic,
};
use ipc::handlers::control::{ControlHandlerContext, RestartController, RestartPlan};
use ipc::handlers::jobs::JobsHandlerContext;
use ipc::handshake::{
    ActiveSessions, CronDefinitionStatus, CronStatusSnapshot, HandshakeContext, HealthState,
    LifecycleFrame, LifecyclePhase, OperationalStatusSnapshot, ResourceArbiterEventSnapshot,
    ResourceArbiterStatusSnapshot, SourceHealthStatusEntry, SourceHealthStatusSnapshot,
    ValidationStatusSnapshot,
};
use jobs::runner::{
    ClaimedJob, EnqueueJobRequest, EnqueuedValidatorJob, FreshnessState, JobLeaseHeartbeat,
    JobRunner, ObsoleteJobRequest, PersistSuccessRequest, PersistValidationVerdictRequest,
    ProcessValidationState, RetroactiveBacklogMetadata, SkillTelemetrySnapshot,
    SkillTelemetryThresholds, SkillTelemetryWindow, VALIDATION_SKIPPED_VALIDATOR_ERROR,
    VALIDATOR_JOB_KIND, ValidationHandlingMode, ValidatorBasisSufficiency, ValidatorEnqueueRequest,
    ValidatorVerdict, allocate_stage_run_id, compute_skill_telemetry, decide_validator_failure,
    decide_validator_verdict, set_stage_head_freshness, skill_telemetry_thresholds,
    start_job_lease_heartbeat,
};
use nix::unistd::{Pid, getpgid, getsid};
use pdf::engine::{
    EngineConfig as PdfEngineConfig, PdfExtractionEngine, PdfInput, PersistStructuralExtractionRow,
    ProcessingStrategy, clone_reused_extraction, compute_source_hash, lookup_reusable_extraction,
    persist_structural_extraction,
};
use pdf::policy::{resolve_document_role, resolve_extraction_policy};
use prompt::renderer::{PromptRenderer, PromptRendererInput};
use pty::capture::{CaptureConfig, PtyCapture, compressed_payload_sha256};
use pty::health::{
    HealthIssue, HostResources, detect_host_sleep, detect_resource_pressure, resolve_health_issue,
};
use pty::session::{PtyCommand, PtySession, RuntimeIds as PtyRuntimeIds, SpawnOptions, SpawnPath};
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use scheduler::cron::CronScheduler;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use sessions::reaper::{CrashRecoveryContext, SessionReaper};
use sha2::{Digest, Sha256};
use sources::registry::{HealthState as RuntimeHealthState, SourceRegistry};
use tracing::forensic::{
    ForensicCaptureSummary, ForensicIdentity, ForensicPtyCapture, capture_contract_output,
    summarize_contract_capture,
};
use tracing::structured::{
    StructuredControlEventKind, StructuredControlEventRecord, StructuredTrace, TraceIdentity,
    parse_phase_history,
};
use tracing_subscriber::EnvFilter;
use uuid::Uuid;
use validation::manifest_writer::{ManifestWriteRequest, write_hybrid_run_manifest};
use validation::policy::{
    BasisSufficiency as PolicyBasisSufficiency, SkillPromotionStage as PolicySkillPromotionStage,
    SourceHealthState as PolicyHealthState, ValidationPolicyConfig, ValidationPolicyContext,
    ValidationPolicyMode, ValidationTrigger, evaluate_validation_policy,
};

const BACKTRACK_WINDOW_SECONDS: u32 = 60;
const CLI_VENDOR: &str = "fixture-cli";
const CLI_VERSION: &str = "1.0.0";
const DEFAULT_AGENT_CLI: &str = "claude";
const LOCAL_EXECUTOR_ID: &str = "local-daemon";
const DEFAULT_RESTART_DELAY_MS: u64 = 250;
const RESTART_DELAY_ENV: &str = "DAEMON_RESTART_DELAY_MS";
#[cfg(test)]
const ACTIVE_STAGES: [&str; 12] = [
    "Normalize",
    "Prequalify notice",
    "Acquire docs",
    "Extract lots",
    "Qualify lot",
    "Extract products",
    "Score quoteability",
    "Price-signal match",
    "Gross margin gate",
    "Landed cost",
    "Net margin gate",
    "Manifest",
];

const LIVE_LOOP_STEP_SURFACES: [StepSurface; 14] = [
    StepSurface {
        step: "trigger",
        sqlite_writes: &[],
        artifact_writes: &[],
    },
    StepSurface {
        step: "job_creation",
        sqlite_writes: &["workflow_jobs"],
        artifact_writes: &[],
    },
    StepSurface {
        step: "adapter_selection",
        sqlite_writes: &[],
        artifact_writes: &[],
    },
    StepSurface {
        step: "session_spawn",
        sqlite_writes: &["agent_session_logs"],
        artifact_writes: &[],
    },
    StepSurface {
        step: "skill_execution",
        sqlite_writes: &[],
        artifact_writes: &[],
    },
    StepSurface {
        step: "incremental_retrieval",
        sqlite_writes: &["raw_documents", "source_cursors"],
        artifact_writes: &[],
    },
    StepSurface {
        step: "capture",
        sqlite_writes: &["agent_session_logs"],
        artifact_writes: &["session stream chunks", "stream.index.jsonl"],
    },
    StepSurface {
        step: "contracts",
        sqlite_writes: &["agent_session_logs"],
        artifact_writes: &["result.json"],
    },
    StepSurface {
        step: "run_manifest",
        sqlite_writes: &["agent_session_logs"],
        artifact_writes: &["run-manifest.md"],
    },
    StepSurface {
        step: "result_validation",
        sqlite_writes: &[],
        artifact_writes: &[],
    },
    StepSurface {
        step: "pipeline_processing",
        sqlite_writes: &[
            "procurement_processes",
            "contract_notices",
            "notice_lots",
            "qualification_runs",
            "product_candidates",
            "supplier_quotes",
            "candidate_pricing_assessments",
            "landed_cost_estimates",
        ],
        artifact_writes: &[],
    },
    StepSurface {
        step: "stage_tracking",
        sqlite_writes: &[
            "process_stage_status",
            "process_stage_heads",
            "commit_clock",
        ],
        artifact_writes: &[],
    },
    StepSurface {
        step: "session_cleanup",
        sqlite_writes: &["agent_session_logs"],
        artifact_writes: &[],
    },
    StepSurface {
        step: "verifiable_output",
        sqlite_writes: &["pipeline_assessments", "manifests", "workflow_jobs"],
        artifact_writes: &["opportunity manifest markdown"],
    },
];

#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Parser)]
#[command(
    name = "daemon",
    version,
    about = "Layer 1 thin local runner bootstrap"
)]
struct Cli {
    /// Start the daemon, write readiness artifacts, and exit cleanly.
    #[arg(long, default_value_t = false)]
    dry_run: bool,

    /// Runtime state root containing the local `SQLite` database.
    #[arg(long, default_value = "state")]
    state_root: PathBuf,

    /// Runtime directory containing the lock, socket, and descriptor.
    #[arg(long, default_value = "runtime")]
    runtime_dir: PathBuf,

    /// Artifact root for captured bytes, run manifests, and opportunity manifests.
    #[arg(long, default_value = "artifacts")]
    artifact_root: PathBuf,

    /// When enabled, keep serving handshake connections until the daemon exits.
    #[arg(
        long,
        default_value_t = false,
        conflicts_with_all = ["serve_once", "ingest", "skill_path", "dry_run", "pipeline"]
    )]
    serve: bool,

    /// When enabled, accept exactly one handshake before exiting.
    #[arg(long, default_value_t = false, conflicts_with = "serve")]
    serve_once: bool,

    /// Run the manual live-loop trigger for the declared source id.
    #[arg(long)]
    ingest: Option<String>,

    /// Run a bound skill under daemon control.
    #[arg(long)]
    skill_path: Option<PathBuf>,

    /// Optional JSON object containing job parameters for `--skill-path`.
    #[arg(long)]
    job_parameters_json: Option<String>,

    /// Select the agent CLI adapter to use for this run (`claude` or `codex`).
    #[arg(long = "agent-cli")]
    adapter_choice: Option<String>,

    /// Override the selected agent CLI program path.
    #[arg(long)]
    agent_cli_program: Option<PathBuf>,

    /// Fixture file used by tests to drive deterministic live-loop execution.
    #[arg(long, hide = true)]
    fixture_path: Option<PathBuf>,

    /// Run deterministic pipeline stages on already-ingested data for the given source.
    #[arg(long)]
    pipeline: Option<String>,

    /// Claim and execute exactly one pending workflow job. Hidden because it is
    /// currently only used by deterministic integration tests and PDF queue work.
    #[arg(
        long,
        hide = true,
        default_value_t = false,
        conflicts_with_all = [
            "serve",
            "serve_once",
            "ingest",
            "skill_path",
            "dry_run",
            "pipeline"
        ]
    )]
    run_next_job: bool,

    /// Dispatch the supplier-price-enrichment skill for auto-search-eligible
    /// search_profiles whose source is `<source_id>` (e.g. `alibaba`). Batches
    /// profiles into a single skill invocation so the agent receives the full
    /// SearchProfile JSON, not just head_noun + name.
    #[arg(long)]
    enrich_pricing: Option<String>,

    /// Maximum number of search_profiles to batch into a single enrichment
    /// skill invocation. Defaults to 4. Increase carefully: agent context
    /// cost scales with batch size.
    #[arg(long, default_value_t = 4)]
    enrich_pricing_limit: usize,

    /// Dry-run mode for `--enrich-pricing`. Resolves the target skill path,
    /// builds the job_parameters JSON, and prints both to stdout — does NOT
    /// spawn the agent session. Use to validate per-source routing + inspect
    /// what the agent would see in its prompt.
    #[arg(long, default_value_t = false)]
    enrich_pricing_dry_run: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AgentCliKind {
    Claude,
    Codex,
}

impl AgentCliKind {
    fn parse(raw: &str) -> Result<Self> {
        match raw {
            "claude" => Ok(Self::Claude),
            "codex" => Ok(Self::Codex),
            other => bail!("unsupported agent_cli `{other}`"),
        }
    }

    const fn as_str(self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Codex => "codex",
        }
    }

    const fn env_key(self) -> &'static str {
        match self {
            Self::Claude => "DAEMON_CLAUDE_PROGRAM",
            Self::Codex => "DAEMON_CODEX_PROGRAM",
        }
    }
}

struct RuntimeBootstrap {
    _lock: LockGuard,
    listener: std::os::unix::net::UnixListener,
    descriptor: DaemonDescriptor,
    state_db: StateDb,
    artifact_root: PathBuf,
    resource_arbiter: Mutex<ResourceArbiter>,
    restart_controller: RestartController,
}

#[derive(Debug, Clone)]
struct PersistedSessionRow {
    id: String,
    job_id: Option<String>,
    skill_path: String,
    pid: Option<u32>,
    pgid: Option<i32>,
    started_at: DateTime<Utc>,
    status: String,
    structured_trace: Option<String>,
    last_checkpoint_seq: Option<u32>,
}

#[derive(Debug, Clone)]
struct AdapterRuntimeConfig {
    kind: AgentCliKind,
    program: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct StepSurface {
    step: &'static str,
    sqlite_writes: &'static [&'static str],
    artifact_writes: &'static [&'static str],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FixtureNotice {
    process_id: String,
    notice_id: String,
    source_notice_id: String,
    source_url: String,
    title: String,
    buyer_name: String,
    published_at: String,
    source_updated_at: String,
    #[serde(default)]
    ocid: Option<String>,
    #[serde(default = "default_procurement_stage")]
    procurement_stage: String,
    #[serde(default = "default_fixture_lots")]
    lots: Vec<FixtureLot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FixtureLot {
    lot_id: String,
    #[serde(default)]
    source_lot_id: Option<String>,
    title: String,
    #[serde(default = "default_candidate_count")]
    candidate_count: u32,
    #[serde(default = "default_lot_value")]
    estimated_value_pence: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct StageTarget {
    stage: String,
    scope_kind: String,
    scope_id: String,
}

#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LiveLoopFixture {
    #[serde(default)]
    cursor_before: Option<String>,
    #[serde(default = "default_cursor_after")]
    cursor_after: String,
    #[serde(default)]
    continuation_required: bool,
    #[serde(default = "default_fingerprint_state")]
    fingerprint_state: String,
    #[serde(default = "default_locale_state")]
    locale_state: String,
    #[serde(default = "default_currency_state")]
    currency_state: String,
    #[serde(default = "default_transcript_lines")]
    pty_transcript_lines: Vec<String>,
    #[serde(default)]
    source_changed: bool,
    #[serde(default)]
    notice_parse_failed: bool,
    #[serde(default)]
    capture_write_failure: bool,
    #[serde(default)]
    sqlite_write_failure: bool,
    #[serde(default)]
    manifest_schema_invalid: bool,
    #[serde(default)]
    manifest_oqg_suspect: bool,
    #[serde(default)]
    validator_fail_open: bool,
    #[serde(default)]
    validator_flagged: bool,
    #[serde(default)]
    validator_rejected: bool,
    #[serde(default)]
    validator_low_basis: bool,
    #[serde(default = "default_extraction_fill_rate_bps")]
    extraction_fill_rate_bps: u16,
    #[serde(default)]
    operator_requested: bool,
    #[serde(default)]
    basis_hash_mismatch: bool,
    #[serde(default)]
    goods_classification_low_confidence: bool,
    #[serde(default)]
    supplier_match_low_confidence: bool,
    #[serde(default)]
    commodity_code_low_confidence: bool,
    #[serde(default)]
    revenue_basis_uncertain: bool,
    #[serde(default)]
    fpo_access_unavailable: bool,
    #[serde(default)]
    host_sleep_monotonic_gap_ms: u64,
    #[serde(default = "default_host_sleep_threshold_ms")]
    host_sleep_threshold_ms: u64,
    #[serde(default)]
    host_cpu_pressure: bool,
    #[serde(default)]
    host_memory_pressure: bool,
    #[serde(default)]
    set_replacement_failure_stage: Option<String>,
    #[serde(default)]
    soft_stale_targets: Vec<StageTarget>,
    #[serde(default)]
    hard_invalid_targets: Vec<StageTarget>,
    #[serde(default = "default_notices")]
    notices: Vec<FixtureNotice>,
}

#[derive(Debug, Clone, Deserialize)]
struct SkillFrontmatter {
    skill_id: String,
    skill_version: String,
    interaction_profile: String,
    result_schema_version: String,
    #[serde(default)]
    checkpoint_schema_version: Option<String>,
    #[serde(default)]
    validator_overlay_version: Option<String>,
    model_profile: String,
    #[serde(default)]
    agent_cli: Option<String>,
    #[serde(default)]
    source_structure_version: Option<String>,
    #[serde(default)]
    worker_behavior_version: Option<String>,
    #[serde(default)]
    validator_behavior_version: Option<String>,
    #[serde(default)]
    validation_policy: Option<ValidationPolicyMode>,
    #[serde(default)]
    validation_sample_rate: Option<u16>,
    #[serde(default)]
    validation_triggers: Option<BTreeSet<ValidationTrigger>>,
    #[serde(default = "default_skill_promotion_stage")]
    promotion_stage: SkillPromotionStageMetadata,
    #[serde(default = "default_version_change_affects_behavior")]
    version_change_affects_behavior: bool,
    #[serde(default)]
    gate_a_family_key: Option<String>,
    #[serde(default)]
    gate_a_family_prequalified: bool,
    #[serde(default)]
    gate_b_exact_slice_key: Option<String>,
    #[serde(default)]
    gate_b_exact_slice_ready: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
enum SkillPromotionStageMetadata {
    Draft,
    Candidate,
    Canary,
    Shadow,
    #[default]
    Stable,
    Rollback,
}

#[derive(Debug, Clone)]
struct LoadedSkill {
    skill_path: PathBuf,
    skill_markdown: String,
    metadata: SkillFrontmatter,
    result_schema: Value,
}

#[derive(Debug, Clone)]
struct SessionDraft {
    session_id: String,
    job_id: String,
    skill_path: String,
    skill_version: String,
    execution_mode: String,
    status: String,
    cli_vendor: Option<String>,
    cli_version: Option<String>,
    adapter_id: Option<String>,
    adapter_version: Option<String>,
    config_hash: Option<String>,
    pty_session_id: Option<String>,
    sid: Option<u32>,
    pid: Option<u32>,
    pgid: Option<u32>,
    started_at: String,
    completed_at: String,
    terminal_output_path: Option<String>,
    terminal_output_hash: Option<String>,
    terminal_output_bytes: Option<i64>,
    terminal_output_tail: Option<String>,
    structured_trace: Option<String>,
    error_type: Option<String>,
    error_message: Option<String>,
    exit_code: Option<i32>,
    cleanup_verdict: Option<String>,
    last_checkpoint_seq: Option<u32>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct CaptureSummary {
    index_path: PathBuf,
    first_chunk_hash: String,
    total_bytes: i64,
    tail: String,
    artifact_refs: Vec<String>,
}

#[derive(Debug, Clone)]
struct PipelinePersistence {
    process_id: String,
    review_required: bool,
    review_reason: Option<String>,
    gross_margin_bps: i64,
    net_margin_bps: i64,
    manifest_stage_run_id: String,
}

#[derive(Debug, Clone)]
struct LiveValidationExecution {
    validation_required: bool,
    validator_job_id: Option<String>,
    validation_verdict_id: Option<String>,
    trust_state: jobs::runner::ValidationTrustState,
    run_scoped_validation_rejection_rate_bps: Option<u32>,
    process_validation_state: ProcessValidationState,
    validation_marker: Option<&'static str>,
    validation_telemetry: ValidationStatusSnapshot,
}

#[derive(Debug, Clone)]
struct ManifestComputation {
    markdown: String,
    decision_basis_hash: String,
    candidate_count: u32,
}

#[derive(Debug, Clone)]
struct LiveLoopReport {
    summary_line: String,
    status: String,
}

#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct HostHealthSnapshot {
    issue: Option<String>,
    tainted_active_browser_session: bool,
    blocks_new_admission: bool,
    monotonic_gap_ms: u64,
    sleep_threshold_ms: u64,
    cpu_pressure: bool,
    memory_pressure: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct SkillRunTelemetry {
    skill_id: String,
    skill_version: String,
    worker_behavior_version: String,
    validator_behavior_version: String,
    telemetry: SkillTelemetrySnapshot,
    thresholds: SkillTelemetryThresholds,
    host_health: HostHealthSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProcontractRegistrationContract {
    #[serde(default)]
    cursor: String,
    continuation_required: bool,
    #[serde(default)]
    registrations: Vec<ProcontractRegistrationRecord>,
    summary: ProcontractRegistrationSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProcontractRegistrationRecord {
    notice_id: String,
    #[serde(default)]
    process_id: Option<String>,
    #[serde(default)]
    source_url: Option<String>,
    action: String,
    #[serde(default)]
    skip_reason: Option<String>,
    #[serde(default)]
    failure_reason: Option<String>,
    #[serde(default)]
    registered_at: Option<String>,
    #[serde(default)]
    pre_filter_result: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProcontractRegistrationSummary {
    total_evaluated: u32,
    registered: u32,
    already_registered: u32,
    skipped_service: u32,
    skipped_expiry: u32,
    failed: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProcontractEnrichmentContract {
    #[serde(default)]
    cursor: String,
    continuation_required: bool,
    #[serde(default)]
    enriched_tenders: Vec<ProcontractEnrichedTender>,
    summary: ProcontractEnrichmentSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProcontractEnrichedTender {
    notice_id: String,
    #[serde(default)]
    process_id: Option<String>,
    rfx_id: String,
    rfx_url: String,
    updated_title: String,
    updated_description: String,
    #[serde(default)]
    updated_deadline_at: Option<String>,
    buyer_name: String,
    #[serde(default)]
    buyer_contact: Option<Value>,
    #[serde(default)]
    documents_downloaded: Vec<ProcontractDownloadedDocument>,
    documents_count: u32,
    enriched_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProcontractDownloadedDocument {
    filename: String,
    content_type: String,
    #[serde(default)]
    size_bytes: Option<i64>,
    local_path: String,
    #[serde(default)]
    download_url: Option<String>,
    #[serde(default)]
    document_role_hint: Option<String>,
    #[serde(default)]
    extracted_from_archive: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProcontractEnrichmentSummary {
    total_checked: u32,
    enriched: u32,
    not_yet_responded: u32,
    failed: u32,
}

#[derive(Debug, Clone)]
struct ProcontractInterestCandidate {
    notice_id: String,
    process_id: String,
    source_url: String,
    title: String,
    summary: String,
    main_category: String,
    deadline_at: Option<String>,
}

#[derive(Debug, Clone)]
struct ProcontractInterestEvaluation {
    notice_id: String,
    process_id: String,
    source_url: String,
    title: String,
    action: &'static str,
    skip_reason: Option<&'static str>,
    pre_filter_result: Value,
}

#[derive(Debug, Clone)]
struct ExecutedSession {
    session_id: String,
    started_at: DateTime<Utc>,
    execution_mode: ExecutionMode,
    adapter_id: String,
    cli_vendor: String,
    config_hash: String,
    pty_session_id: Option<String>,
    runtime_ids: Option<PtyRuntimeIds>,
    completed_at: String,
    terminal_output_path: Option<String>,
    terminal_output_hash: Option<String>,
    terminal_output_bytes: Option<i64>,
    terminal_output_tail: Option<String>,
    structured_trace: String,
    exit_code: i32,
    cleanup_verdict: String,
    last_checkpoint_seq: Option<u32>,
    result_path: Option<PathBuf>,
    artifact_refs: Vec<String>,
}

impl SkillFrontmatter {
    fn validation_policy_mode(&self) -> ValidationPolicyMode {
        self.validation_policy
            .unwrap_or(ValidationPolicyMode::Always)
    }

    fn validation_sample_rate_bps(&self) -> u16 {
        self.validation_sample_rate.unwrap_or(10_000).min(10_000)
    }

    fn validation_triggers(&self) -> BTreeSet<ValidationTrigger> {
        self.validation_triggers
            .clone()
            .unwrap_or_else(default_live_validation_triggers)
    }

    fn checkpoint_schema_version(&self) -> String {
        self.checkpoint_schema_version
            .clone()
            .unwrap_or_else(|| self.result_schema_version.clone())
    }

    fn validator_overlay_version(&self) -> String {
        self.validator_overlay_version
            .clone()
            .unwrap_or_else(|| "2026.03.30-1".to_owned())
    }

    fn worker_behavior_version(&self) -> String {
        self.worker_behavior_version.clone().unwrap_or_else(|| {
            if self.version_change_affects_behavior {
                format!(
                    "{}@{}+{}",
                    self.skill_id, self.skill_version, self.result_schema_version
                )
            } else {
                format!("{}@schema+{}", self.skill_id, self.result_schema_version)
            }
        })
    }

    fn validator_behavior_version(&self) -> String {
        self.validator_behavior_version
            .clone()
            .unwrap_or_else(|| self.validator_overlay_version())
    }

    fn gate_a_family_key(&self) -> String {
        self.gate_a_family_key
            .clone()
            .unwrap_or_else(|| skill_family_key(&self.skill_id))
    }

    fn gate_b_exact_slice_key(&self) -> String {
        self.gate_b_exact_slice_key.clone().unwrap_or_else(|| {
            format!(
                "{}::{}::{}::{}",
                self.skill_id,
                skill_source_scope(&self.skill_id),
                self.worker_behavior_version(),
                self.validator_behavior_version()
            )
        })
    }

    fn source_structure_version(&self) -> Option<String> {
        registry_source_id_for_skill(&self.skill_id)
            .and_then(|source_id| {
                SourceRegistry::builtin_definition(source_id)
                    .map(|definition| definition.source_structure_version)
            })
            .or_else(|| self.source_structure_version.clone())
    }
}

const fn default_skill_promotion_stage() -> SkillPromotionStageMetadata {
    SkillPromotionStageMetadata::Stable
}

fn default_live_validation_triggers() -> BTreeSet<ValidationTrigger> {
    BTreeSet::from([
        ValidationTrigger::FillRateDrop,
        ValidationTrigger::FirstRunAfterVersionChange,
        ValidationTrigger::SuspectOutputInLastFive,
        ValidationTrigger::OperatorRequested,
        ValidationTrigger::SourceHealthAmberOrRed,
        ValidationTrigger::PageFingerprintUnknown,
        ValidationTrigger::ContinuationChainLengthGtOne,
        ValidationTrigger::HighValueOpportunity,
    ])
}

const fn default_version_change_affects_behavior() -> bool {
    true
}

fn skill_family_key(skill_id: &str) -> String {
    skill_id
        .split_once('/')
        .map_or_else(|| skill_id.to_owned(), |(family, _)| family.to_owned())
}

fn skill_source_scope(skill_id: &str) -> String {
    skill_id
        .rsplit_once('/')
        .map_or_else(|| "global".to_owned(), |(_, scope)| scope.to_owned())
}

fn registry_source_id_for_skill(skill_id: &str) -> Option<&str> {
    let (family, scope) = skill_id.split_once('/')?;
    match family {
        "procurement-ingestion" | "source-health" | "auth-refresh" => Some(scope),
        _ => None,
    }
}

fn skill_binding_payload(skill: &LoadedSkill) -> Value {
    json!({
        "path": skill.skill_path.to_string_lossy(),
        "version": skill.metadata.skill_version,
        "version_dimensions": {
            "skill_version": skill.metadata.skill_version,
            "result_schema_version": skill.metadata.result_schema_version,
            "checkpoint_schema_version": skill.metadata.checkpoint_schema_version(),
            "validator_overlay_version": skill.metadata.validator_overlay_version(),
            "source_structure_version": skill.metadata.source_structure_version(),
        },
        "promotion": {
            "stage": skill.metadata.promotion_stage,
        },
        "behavior": {
            "worker_behavior_version": skill.metadata.worker_behavior_version(),
            "validator_behavior_version": skill.metadata.validator_behavior_version(),
            "version_change_affects_behavior": skill.metadata.version_change_affects_behavior,
        },
        "calibration": {
            "gate_a_family_key": skill.metadata.gate_a_family_key(),
            "gate_a_family_prequalified": skill.metadata.gate_a_family_prequalified,
            "gate_b_exact_slice_key": skill.metadata.gate_b_exact_slice_key(),
            "gate_b_exact_slice_ready": skill.metadata.gate_b_exact_slice_ready,
            "gate_b_preserved_by_non_behavior_affecting_bump": !skill.metadata.version_change_affects_behavior,
        },
    })
}

struct LivePtyRegistration<'a> {
    session_id: &'a str,
}

impl Drop for LivePtyRegistration<'_> {
    fn drop(&mut self) {
        crate::ipc::handlers::sessions::unregister_live_pty_session(self.session_id);
    }
}

struct ResourceLeaseGuard<'a> {
    arbiter: &'a Mutex<ResourceArbiter>,
    lease: Option<ResourceLease>,
    release_cause: ReleaseCause,
}

impl<'a> ResourceLeaseGuard<'a> {
    const fn new(
        arbiter: &'a Mutex<ResourceArbiter>,
        lease: ResourceLease,
        release_cause: ReleaseCause,
    ) -> Self {
        Self {
            arbiter,
            lease: Some(lease),
            release_cause,
        }
    }

    const fn set_release_cause(&mut self, release_cause: ReleaseCause) {
        self.release_cause = release_cause;
    }
}

impl Drop for ResourceLeaseGuard<'_> {
    fn drop(&mut self) {
        let Some(lease) = self.lease.take() else {
            return;
        };
        if let Ok(mut arbiter) = self.arbiter.lock() {
            arbiter.release(&lease, self.release_cause);
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_target(false)
        .compact()
        .init();

    maybe_delay_for_restart();

    let mut runtime = bootstrap_runtime(&cli)?;

    if cli.serve {
        serve_handshake_service(&mut runtime)?;
        return Ok(());
    }

    if cli.serve_once {
        serve_handshake_once(&runtime)?;
        return Ok(());
    }

    if let Some(skill_path) = &cli.skill_path {
        let report = run_bound_skill(
            &mut runtime,
            skill_path,
            parse_job_parameters(cli.job_parameters_json.as_deref())?,
            &cli,
        )
        .await?;
        if report.status == "completed" {
            println!("{}", report.summary_line);
            return Ok(());
        }
        eprintln!("{}", report.summary_line);
        bail!(report.summary_line);
    }

    if let Some(source_id) = &cli.ingest {
        let report = match source_id.as_str() {
            "fts" => {
                let fixture = load_fixture(cli.fixture_path.as_deref())?;
                run_fts_live_loop(&mut runtime, fixture, &cli).await?
            }
            "contracts-finder" => run_contracts_finder_live_loop(&mut runtime, &cli).await?,
            "procontract" => run_procontract_discover_live_loop(&mut runtime, &cli).await?,
            "procontract-interest" => {
                run_procontract_register_interest_live_loop(&mut runtime, &cli).await?
            }
            "procontract-enrich" => {
                run_procontract_enrich_tender_live_loop(&mut runtime, &cli).await?
            }
            other => bail!("unsupported source: {other}"),
        };
        if report.status == "completed" {
            println!("{}", report.summary_line);
            return Ok(());
        }
        eprintln!("{}", report.summary_line);
        bail!(report.summary_line);
    }

    if let Some(source_id) = &cli.pipeline {
        let report = run_deterministic_pipeline(&mut runtime, source_id)?;
        println!("{}", report.summary_line);
        return Ok(());
    }

    if let Some(source_id) = &cli.enrich_pricing {
        let report =
            run_enrich_pricing(&mut runtime, source_id, cli.enrich_pricing_limit, &cli).await?;
        if report.status == "completed" {
            println!("{}", report.summary_line);
            return Ok(());
        }
        eprintln!("{}", report.summary_line);
        bail!(report.summary_line);
    }

    if cli.run_next_job {
        let report = run_next_workflow_job(&mut runtime, &cli).await?;
        if report.status == "completed" {
            println!("{}", report.summary_line);
            return Ok(());
        }
        eprintln!("{}", report.summary_line);
        bail!(report.summary_line);
    }

    if cli.dry_run {
        ::tracing::info!("dry-run bootstrap completed");
    }

    Ok(())
}

#[derive(Debug, Clone, Deserialize)]
struct StructuralExtractionJobRequest {
    raw_document_id: String,
    process_id: String,
    #[serde(default)]
    notice_id: Option<String>,
    #[serde(default)]
    document_set_id: Option<String>,
    #[serde(default)]
    stage_run_id: Option<String>,
    #[serde(default)]
    extraction_policy_version: Option<String>,
    #[serde(default)]
    document_role: Option<String>,
    #[serde(default)]
    expected_overlays: Vec<String>,
    #[serde(default)]
    docling_wrapper_path: Option<PathBuf>,
    #[serde(default)]
    model_artifacts_path: Option<PathBuf>,
    #[serde(default)]
    document_timeout_secs: Option<u64>,
    #[serde(default)]
    wall_clock_timeout_secs: Option<u64>,
    #[serde(default)]
    processing_strategy: Option<String>,
    #[serde(default)]
    chunk_plan: Option<SemanticChunkPlanRequest>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
struct SemanticExtractionJobRequest {
    document_extraction_id: String,
    process_id: String,
    #[serde(default)]
    notice_id: Option<String>,
    #[serde(default)]
    document_set_id: Option<String>,
    #[serde(default)]
    raw_document_id: Option<String>,
    skill_id: String,
    extraction_scope: String,
    #[serde(default)]
    target_domain: Option<String>,
    #[serde(default)]
    stage_run_id: Option<String>,
    #[serde(default)]
    chunk_plan_hash: Option<String>,
    #[serde(default)]
    chunk_index: Option<usize>,
    #[serde(default)]
    merge_job: bool,
    #[serde(default)]
    merge_chunk_indices: Vec<usize>,
}

#[derive(Debug, Clone)]
struct StructuralExtractionDispatchOutcome {
    document_extraction_id: String,
    stage_run_id: String,
    semantic_extraction_state: String,
    expected_overlays: Vec<String>,
    enqueued_semantic_job_ids: Vec<String>,
    cache_hit: bool,
    freshness_state: String,
    freshness_reason: Option<String>,
    hard_invalidated_extraction_ids: Vec<String>,
    document_set_extraction_state: Option<String>,
}

#[derive(Debug, Clone)]
struct StructuralFreshnessAssessment {
    freshness_state: String,
    freshness_reason: Option<String>,
    hard_invalidated_extraction_ids: Vec<String>,
}

#[derive(Debug, Clone)]
struct SemanticPersistenceOutcome {
    semantic_extraction_state: String,
    downstream_refresh_required: bool,
    document_set_extraction_state: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SemanticChunkRequest {
    chunk_index: usize,
    page_start: u32,
    page_end: u32,
    section_refs: Vec<String>,
    markdown: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SemanticChunkPlanRequest {
    plan_hash: String,
    chunks: Vec<SemanticChunkRequest>,
}

#[allow(clippy::future_not_send)]
async fn run_next_workflow_job(
    runtime: &mut RuntimeBootstrap,
    cli: &Cli,
) -> Result<LiveLoopReport> {
    let runner = JobRunner::new(LOCAL_EXECUTOR_ID);
    let claimed = runner
        .claim_next_job(runtime.state_db.connection())?
        .ok_or_else(|| anyhow!("expected a pending workflow job"))?;

    match claimed.kind.as_str() {
        "extract_document_structure" => {
            run_extract_document_structure_job(runtime, &runner, claimed).await
        }
        "extract_document_semantics" => {
            run_extract_document_semantics_job(runtime, &runner, claimed, cli).await
        }
        other => fail_job(
            runtime,
            &claimed.id,
            &format!("unsupported queued job kind `{other}`"),
            json!({"error_code": "AGENT_RUN_FAILED", "job_kind": other}),
        ),
    }
}

#[allow(clippy::future_not_send, clippy::too_many_lines)]
async fn run_extract_document_structure_job(
    runtime: &mut RuntimeBootstrap,
    runner: &JobRunner,
    claimed: ClaimedJob,
) -> Result<LiveLoopReport> {
    let claimed_id = claimed.id.clone();
    let attempt: Result<LiveLoopReport> = async {
        let request = parse_structural_extraction_request(&claimed.input_payload)?;
        let source_path =
            load_raw_document_source_path(runtime.state_db.connection(), &request.raw_document_id)?;
        let processing_strategy =
            parse_processing_strategy(request.processing_strategy.as_deref())?;
        let engine_config = PdfEngineConfig {
            docling_wrapper_path: request
                .docling_wrapper_path
                .clone()
                .unwrap_or(default_docling_wrapper_path()?),
            model_artifacts_path: request
                .model_artifacts_path
                .clone()
                .unwrap_or(default_docling_artifacts_path()?),
            document_timeout_secs: request.document_timeout_secs.unwrap_or(300),
            wall_clock_timeout_secs: request.wall_clock_timeout_secs.unwrap_or(330),
            python_bin: default_docling_python_bin(),
        };
        let engine = PdfExtractionEngine::new(engine_config.clone());
        if !engine.is_available()? {
            bail!(
                "Docling runtime unavailable for wrapper {}",
                engine_config.docling_wrapper_path.display()
            );
        }

        let source_bytes =
            fs::read(&source_path).with_context(|| format!("reading {}", source_path.display()))?;
        let source_content_hash = compute_source_hash(&source_bytes);
        let engine_version = probe_docling_runtime_version()?;
        let source_filename = source_path
            .file_name()
            .and_then(|name| name.to_str())
            .map(str::to_owned);
        let stage_run_id = request
            .stage_run_id
            .clone()
            .unwrap_or_else(allocate_stage_run_id);

        let outcome = if let Some(existing) = lookup_reusable_extraction(
            runtime.state_db.connection(),
            &source_content_hash,
            &engine_version,
            processing_strategy,
        )? {
            let tables_json: Value = serde_json::from_str(&existing.extracted_tables_json)
                .context("parsing cached structural tables_json")?;
            let resolved_role = resolve_document_role(
                load_notice_document_type(
                    runtime.state_db.connection(),
                    request.notice_id.as_deref(),
                )?
                .as_deref()
                .or(request.document_role.as_deref()),
                source_filename.as_deref(),
                &existing.extracted_markdown,
                &tables_json,
            );
            let mut resolved_policy =
                resolve_extraction_policy(&resolved_role, &existing.extracted_markdown, &tables_json);
            if !request.expected_overlays.is_empty() {
                resolved_policy
                    .expected_overlays
                    .clone_from(&request.expected_overlays);
            }
            let expected_overlays = resolved_policy.expected_overlays;
            let semantic_extraction_state = if expected_overlays.is_empty() {
                "skipped".to_owned()
            } else {
                "pending".to_owned()
            };
            let mut row = clone_reused_extraction(
                &existing,
                &request.raw_document_id,
                &request.process_id,
                request.notice_id.as_deref(),
                request.document_set_id.as_deref(),
                &stage_run_id,
            );
            row.document_role = Some(resolved_role.as_str().to_owned());
            row.extraction_policy_version = request
                .extraction_policy_version
                .clone()
                .unwrap_or(resolved_policy.extraction_policy_version);
            row.expected_overlays_json = serde_json::to_string(&expected_overlays)
                .context("serializing expected overlays for cache hit")?;
            row.semantic_extraction_state
                .clone_from(&semantic_extraction_state);
            LOCAL_EXECUTOR_ID.clone_into(&mut row.owner_id);
            persist_structural_extraction(runtime.state_db.connection(), &row)?;
            let freshness = assess_structural_freshness(
                runtime.state_db.connection(),
                &request.raw_document_id,
                &row.extraction_id,
            )?;
            let document_set_extraction_state = update_document_set_extraction_state(
                runtime.state_db.connection(),
                request.document_set_id.as_deref(),
            )?;
            let enqueued_semantic_job_ids = enqueue_semantic_child_jobs(
                runtime,
                runner,
                &claimed,
                &request,
                &row.extraction_id,
                &stage_run_id,
                &expected_overlays,
            )?;
            StructuralExtractionDispatchOutcome {
                document_extraction_id: row.extraction_id,
                stage_run_id,
                semantic_extraction_state,
                expected_overlays,
                enqueued_semantic_job_ids,
                cache_hit: true,
                freshness_state: freshness.freshness_state,
                freshness_reason: freshness.freshness_reason,
                hard_invalidated_extraction_ids: freshness.hard_invalidated_extraction_ids,
                document_set_extraction_state,
            }
        } else {
            let extraction = engine
                .extract(PdfInput {
                    raw_document_id: Some(request.raw_document_id.clone()),
                    source_path,
                    page_range: None,
                    processing_strategy,
                })
                .await?;
            let mut resolved_policy = resolve_extraction_policy(
                &resolve_document_role(
                    load_notice_document_type(
                        runtime.state_db.connection(),
                        request.notice_id.as_deref(),
                    )?
                    .as_deref()
                    .or(request.document_role.as_deref()),
                    source_filename.as_deref(),
                    &extraction.extracted_markdown,
                    &extraction.tables_json,
                ),
                &extraction.extracted_markdown,
                &extraction.tables_json,
            );
            if !request.expected_overlays.is_empty() {
                resolved_policy
                    .expected_overlays
                    .clone_from(&request.expected_overlays);
            }
            let resolved_document_role = Some(resolved_policy.document_role.as_str().to_owned());
            let expected_overlays = resolved_policy.expected_overlays;
            let semantic_extraction_state = if expected_overlays.is_empty() {
                "skipped".to_owned()
            } else {
                "pending".to_owned()
            };
            let row = PersistStructuralExtractionRow {
                extraction_id: extraction.extraction_id.clone(),
                raw_document_id: request.raw_document_id.clone(),
                process_id: request.process_id.clone(),
                notice_id: request.notice_id.clone(),
                document_set_id: request.document_set_id.clone(),
                source_content_hash: extraction.source_hash.clone(),
                extraction_engine_version: extraction.engine_version.clone(),
                processing_strategy,
                page_count: extraction.page_count,
                document_title: extraction
                    .metadata_json
                    .get("document_title")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
                document_language: extraction
                    .metadata_json
                    .get("document_language")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
                document_role: resolved_document_role,
                extraction_policy_version: request
                    .extraction_policy_version
                    .clone()
                    .unwrap_or(resolved_policy.extraction_policy_version),
                expected_overlays_json: serde_json::to_string(&expected_overlays)
                    .context("serializing expected overlays for cache miss")?,
                extracted_markdown: extraction.extracted_markdown,
                extracted_sections_json: extraction.sections_json.to_string(),
                extracted_tables_json: extraction.tables_json.to_string(),
                extracted_metadata_json: extraction.metadata_json.to_string(),
                table_count: extraction.quality.table_count,
                section_count: extraction.quality.section_count,
                image_count: extraction.quality.image_count,
                has_ocr_content: extraction.quality.has_ocr_content,
                empty_page_count: extraction.quality.empty_page_count,
                quality_warnings_json: if extraction.quality.warnings.is_empty() {
                    None
                } else {
                    Some(json!(extraction.quality.warnings).to_string())
                },
                semantic_extraction_state: semantic_extraction_state.clone(),
                owner_id: LOCAL_EXECUTOR_ID.to_owned(),
                stage_run_id,
            };
            persist_structural_extraction(runtime.state_db.connection(), &row)?;
            let freshness = assess_structural_freshness(
                runtime.state_db.connection(),
                &request.raw_document_id,
                &row.extraction_id,
            )?;
            let document_set_extraction_state = update_document_set_extraction_state(
                runtime.state_db.connection(),
                request.document_set_id.as_deref(),
            )?;
            let enqueued_semantic_job_ids = enqueue_semantic_child_jobs(
                runtime,
                runner,
                &claimed,
                &request,
                &row.extraction_id,
                &row.stage_run_id,
                &expected_overlays,
            )?;
            StructuralExtractionDispatchOutcome {
                document_extraction_id: row.extraction_id,
                stage_run_id: row.stage_run_id,
                semantic_extraction_state,
                expected_overlays,
                enqueued_semantic_job_ids,
                cache_hit: false,
                freshness_state: freshness.freshness_state,
                freshness_reason: freshness.freshness_reason,
                hard_invalidated_extraction_ids: freshness.hard_invalidated_extraction_ids,
                document_set_extraction_state,
            }
        };

        let scope_id = request
            .notice_id
            .clone()
            .unwrap_or_else(|| request.raw_document_id.clone());
        runner.persist_success(
            &mut runtime.state_db,
            &PersistSuccessRequest {
                job_id: claimed.id.clone(),
                idempotency_key: format!("extract_document_structure::{}", claimed.id),
                session_id: format!("daemon-struct-{}", outcome.document_extraction_id),
                output_payload: json!({
                    "raw_document_id": request.raw_document_id,
                    "document_extraction_id": outcome.document_extraction_id,
                    "cache_hit": outcome.cache_hit,
                    "expected_overlays": outcome.expected_overlays,
                    "semantic_extraction_state": outcome.semantic_extraction_state,
                    "semantic_child_job_ids": outcome.enqueued_semantic_job_ids,
                    "stage_run_id": outcome.stage_run_id,
                    "freshness_state": outcome.freshness_state,
                    "freshness_reason": outcome.freshness_reason,
                    "hard_invalidated_extraction_ids": outcome.hard_invalidated_extraction_ids,
                    "document_set_extraction_state": outcome.document_set_extraction_state,
                }),
                stage: "acquire docs".to_owned(),
                scope_kind: "notice".to_owned(),
                scope_id,
                stage_run_id: outcome.stage_run_id.clone(),
                process_id: request.process_id.clone(),
                owner_id: LOCAL_EXECUTOR_ID.to_owned(),
            },
            &ValidationOutcome {
                disposition: ValidationDisposition::Accepted,
                error_code: None,
                error_context_json: None,
                anomalies: Vec::new(),
            },
        )?;

        Ok(LiveLoopReport {
            summary_line: format!(
                "Structural extraction completed: job_id={} extraction_id={} cache_hit={} semantic_children={}",
                claimed.id,
                outcome.document_extraction_id,
                outcome.cache_hit,
                outcome.enqueued_semantic_job_ids.len()
            ),
            status: "completed".to_owned(),
        })
    }
    .await;

    match attempt {
        Ok(report) => Ok(report),
        Err(error) => fail_job(
            runtime,
            &claimed_id,
            &error.to_string(),
            json!({
                "error_code": "AGENT_RUN_FAILED",
                "job_kind": "extract_document_structure",
            }),
        ),
    }
}

#[allow(clippy::future_not_send, clippy::too_many_lines)]
async fn run_extract_document_semantics_job(
    runtime: &mut RuntimeBootstrap,
    runner: &JobRunner,
    claimed: ClaimedJob,
    cli: &Cli,
) -> Result<LiveLoopReport> {
    let claimed_id = claimed.id.clone();
    let attempt: Result<LiveLoopReport> = async {
        let request = parse_semantic_extraction_request(&claimed.input_payload)?;
        let skill = load_pdf_extraction_skill(&request.skill_id)?;
        let session_id = format!("session-semantic-{}", sha256_hex(claimed.id.as_bytes()));
        let executed = execute_skill_session(
            runtime,
            &skill,
            &claimed.id,
            &session_id,
            &claimed.input_payload,
            cli,
        )
        .await?;
        insert_executed_session_log(runtime.state_db.connection(), &skill, &claimed.id, &executed)?;

        let result_path = executed
            .result_path
            .as_ref()
            .ok_or_else(|| anyhow!("semantic extraction session missing result contract path"))?;
        let result_text = fs::read_to_string(result_path)
            .with_context(|| format!("reading semantic extraction result {}", result_path.display()))?;
        let result_json: Value =
            serde_json::from_str(&result_text).context("parsing semantic extraction result JSON")?;

        let persistence = persist_extraction_from_agent_output(
            &mut runtime.state_db,
            &request.document_extraction_id,
            &request.process_id,
            &skill,
            &claimed.id,
            &session_id,
            &result_json,
        )?;

        let scope_id = request
            .notice_id
            .clone()
            .or_else(|| request.raw_document_id.clone())
            .unwrap_or_else(|| request.document_extraction_id.clone());
        let stage_run_id = format!(
            "{}-{}",
            request
                .stage_run_id
                .clone()
                .unwrap_or_else(allocate_stage_run_id),
            slug_token(&skill.metadata.skill_id),
        );
        let target_domain = target_domain_for_overlay(&skill.metadata.skill_id);
        runner.persist_success(
            &mut runtime.state_db,
            &PersistSuccessRequest {
                job_id: claimed.id.clone(),
                idempotency_key: format!("extract_document_semantics::{}", claimed.id),
                session_id: session_id.clone(),
                output_payload: json!({
                    "document_extraction_id": request.document_extraction_id,
                    "skill_id": skill.metadata.skill_id,
                    "target_domain": target_domain,
                    "extraction_scope": request.extraction_scope,
                    "semantic_extraction_state": persistence.semantic_extraction_state,
                    "downstream_refresh_required": persistence.downstream_refresh_required,
                    "document_set_extraction_state": persistence.document_set_extraction_state,
                }),
                stage: "acquire docs".to_owned(),
                scope_kind: "notice".to_owned(),
                scope_id,
                stage_run_id,
                process_id: request.process_id.clone(),
                owner_id: LOCAL_EXECUTOR_ID.to_owned(),
            },
            &ValidationOutcome {
                disposition: ValidationDisposition::Accepted,
                error_code: None,
                error_context_json: None,
                anomalies: Vec::new(),
            },
        )?;

        Ok(LiveLoopReport {
            summary_line: format!(
                "Semantic extraction completed: job_id={} document_extraction_id={} skill_id={} target_domain={}",
                claimed.id,
                request.document_extraction_id,
                skill.metadata.skill_id,
                target_domain,
            ),
            status: "completed".to_owned(),
        })
    }
    .await;

    match attempt {
        Ok(report) => Ok(report),
        Err(error) => fail_job(
            runtime,
            &claimed_id,
            &error.to_string(),
            json!({
                "error_code": "AGENT_RUN_FAILED",
                "job_kind": "extract_document_semantics",
            }),
        ),
    }
}

fn parse_structural_extraction_request(payload: &Value) -> Result<StructuralExtractionJobRequest> {
    let request = payload
        .get("request")
        .and_then(|value| value.get("request"))
        .or_else(|| payload.get("request"))
        .unwrap_or(payload);
    serde_json::from_value(request.clone()).context("parsing structural extraction job request")
}

fn parse_semantic_extraction_request(payload: &Value) -> Result<SemanticExtractionJobRequest> {
    let request = payload
        .get("request")
        .and_then(|value| value.get("request"))
        .or_else(|| payload.get("request"))
        .unwrap_or(payload);
    serde_json::from_value(request.clone()).context("parsing semantic extraction job request")
}

fn parse_processing_strategy(raw: Option<&str>) -> Result<ProcessingStrategy> {
    match raw.unwrap_or("standard") {
        "standard" => Ok(ProcessingStrategy::Standard),
        "force_full_page_ocr" => Ok(ProcessingStrategy::ForceFullPageOcr),
        "fast_table_structure" => Ok(ProcessingStrategy::FastTableStructure),
        other => bail!("unsupported processing_strategy `{other}`"),
    }
}

fn default_docling_wrapper_path() -> Result<PathBuf> {
    Ok(repo_root()?.join("scripts/docling-extract.py"))
}

fn default_docling_artifacts_path() -> Result<PathBuf> {
    if let Some(path) = std::env::var_os("DOCLING_ARTIFACTS_PATH") {
        return Ok(PathBuf::from(path));
    }
    let home = std::env::var_os("HOME").ok_or_else(|| {
        anyhow!("DOCLING_ARTIFACTS_PATH is unset and HOME is unavailable for default model path")
    })?;
    Ok(PathBuf::from(home).join(".cache/docling/models"))
}

fn default_docling_python_bin() -> PathBuf {
    // Prefer the docling venv if it exists, otherwise fall back to system python3.
    let venv_python = PathBuf::from(std::env::var_os("DOCLING_PYTHON_BIN").unwrap_or_else(|| {
        let home = std::env::var_os("HOME").unwrap_or_default();
        PathBuf::from(home)
            .join(".docling-venv/bin/python")
            .into_os_string()
    }));
    if venv_python.is_file() {
        return venv_python;
    }
    PathBuf::from("python3")
}

fn load_notice_document_type(conn: &Connection, notice_id: Option<&str>) -> Result<Option<String>> {
    let Some(notice_id) = notice_id else {
        return Ok(None);
    };
    let payload: Option<String> = conn
        .query_row(
            "SELECT normalized_payload FROM contract_notices WHERE id = ?1",
            params![notice_id],
            |row| row.get(0),
        )
        .optional()?;
    let Some(payload) = payload else {
        return Ok(None);
    };
    let value: Value = serde_json::from_str(&payload)
        .with_context(|| format!("parsing normalized_payload for {notice_id}"))?;
    Ok(value
        .get("documentType")
        .or_else(|| value.get("document_type"))
        .and_then(Value::as_str)
        .map(str::to_owned))
}

fn probe_docling_runtime_version() -> Result<String> {
    let python_bin = default_docling_python_bin();
    let output = process::Command::new(&python_bin)
        .arg("-c")
        .arg("from importlib.metadata import version; print(version('docling'))")
        .output()
        .context("probing Docling runtime version")?;
    ensure!(
        output.status.success(),
        "Docling runtime probe failed with status {:?}",
        output.status.code()
    );
    let version = String::from_utf8(output.stdout)
        .context("decoding Docling runtime version output")?
        .trim()
        .to_owned();
    ensure!(
        !version.is_empty(),
        "Docling runtime probe returned empty version output"
    );
    Ok(version)
}

fn load_raw_document_source_path(conn: &Connection, raw_document_id: &str) -> Result<PathBuf> {
    let storage_path: Option<String> = conn
        .query_row(
            "SELECT storage_path FROM raw_documents WHERE id = ?1",
            params![raw_document_id],
            |row| row.get(0),
        )
        .optional()?
        .flatten();
    let raw_path = storage_path
        .map(PathBuf::from)
        .ok_or_else(|| anyhow!("raw_document `{raw_document_id}` is missing a storage_path"))?;
    // Resolve relative paths against the repo root.
    let path = if raw_path.is_absolute() {
        raw_path
    } else {
        repo_root()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(&raw_path)
    };
    ensure!(
        path.is_file(),
        "raw_document `{raw_document_id}` storage_path does not exist: {}",
        path.display()
    );
    Ok(path)
}

fn target_domain_for_overlay(skill_id: &str) -> &'static str {
    match skill_id {
        "pdf-extraction/tender-specification" => "lots",
        "pdf-extraction/pricing-schedule" => "pricing",
        "pdf-extraction/bill-of-quantities" => "quantities",
        _ => "generic",
    }
}

fn load_pdf_extraction_skill(skill_id: &str) -> Result<LoadedSkill> {
    let skill_root = repo_root()?.join("skills").join(skill_id);
    load_skill(&skill_root, skill_id)
}

#[allow(clippy::too_many_lines)]
fn enqueue_semantic_child_jobs(
    runtime: &mut RuntimeBootstrap,
    runner: &JobRunner,
    claimed: &ClaimedJob,
    request: &StructuralExtractionJobRequest,
    document_extraction_id: &str,
    stage_run_id: &str,
    expected_overlays: &[String],
) -> Result<Vec<String>> {
    let mut job_ids = Vec::new();
    let scope_id = request
        .notice_id
        .clone()
        .unwrap_or_else(|| request.raw_document_id.clone());

    for skill_id in expected_overlays {
        let job_id = format!("job-{}", Uuid::new_v4());
        let target_domain = target_domain_for_overlay(skill_id);
        if let Some(plan) = request.chunk_plan.as_ref() {
            for chunk in &plan.chunks {
                let chunk_job_id = format!("job-{}", Uuid::new_v4());
                runner.enqueue_job(
                    &mut runtime.state_db,
                    &EnqueueJobRequest {
                        job_id: chunk_job_id.clone(),
                        kind: "extract_document_semantics".to_owned(),
                        process_id: Some(request.process_id.clone()),
                        notice_id: request.notice_id.clone(),
                        scope_kind: Some("notice".to_owned()),
                        scope_id: Some(scope_id.clone()),
                        parent_job_id: Some(claimed.id.clone()),
                        continuation_chain_id: None,
                        validation_required: false,
                        scheduled_for: None,
                        input_payload: json!({
                            "request": {
                                "document_extraction_id": document_extraction_id,
                                "process_id": request.process_id,
                                "notice_id": request.notice_id,
                                "document_set_id": request.document_set_id,
                                "raw_document_id": request.raw_document_id,
                                "skill_id": skill_id,
                                "extraction_scope": format!("chunk:{}:{}", plan.plan_hash, chunk.chunk_index),
                                "target_domain": target_domain,
                                "stage_run_id": stage_run_id,
                                "chunk_plan_hash": plan.plan_hash,
                                "chunk_index": chunk.chunk_index,
                            }
                        }),
                        idempotency_key: format!(
                            "extract_document_structure::{}::{}::chunk::{}",
                            claimed.id, skill_id, chunk.chunk_index
                        ),
                    },
                )?;
                job_ids.push(chunk_job_id);
            }

            let merge_job_id = format!("job-{}", Uuid::new_v4());
            let merge_chunk_indices = plan
                .chunks
                .iter()
                .map(|chunk| chunk.chunk_index)
                .collect::<Vec<_>>();
            runner.enqueue_job(
                &mut runtime.state_db,
                &EnqueueJobRequest {
                    job_id: merge_job_id.clone(),
                    kind: "extract_document_semantics".to_owned(),
                    process_id: Some(request.process_id.clone()),
                    notice_id: request.notice_id.clone(),
                    scope_kind: Some("notice".to_owned()),
                    scope_id: Some(scope_id.clone()),
                    parent_job_id: Some(claimed.id.clone()),
                    continuation_chain_id: None,
                    validation_required: false,
                    scheduled_for: None,
                    input_payload: json!({
                        "request": {
                            "document_extraction_id": document_extraction_id,
                            "process_id": request.process_id,
                            "notice_id": request.notice_id,
                            "document_set_id": request.document_set_id,
                            "raw_document_id": request.raw_document_id,
                            "skill_id": skill_id,
                            "extraction_scope": format!("chunk_merged:{}", plan.plan_hash),
                            "target_domain": target_domain,
                            "stage_run_id": stage_run_id,
                            "chunk_plan_hash": plan.plan_hash,
                            "merge_job": true,
                            "merge_chunk_indices": merge_chunk_indices,
                        }
                    }),
                    idempotency_key: format!(
                        "extract_document_structure::{}::{}::merge::{}",
                        claimed.id, skill_id, plan.plan_hash
                    ),
                },
            )?;
            job_ids.push(merge_job_id);
        } else {
            runner.enqueue_job(
                &mut runtime.state_db,
                &EnqueueJobRequest {
                    job_id: job_id.clone(),
                    kind: "extract_document_semantics".to_owned(),
                    process_id: Some(request.process_id.clone()),
                    notice_id: request.notice_id.clone(),
                    scope_kind: Some("notice".to_owned()),
                    scope_id: Some(scope_id.clone()),
                    parent_job_id: Some(claimed.id.clone()),
                    continuation_chain_id: None,
                    validation_required: false,
                    scheduled_for: None,
                    input_payload: json!({
                        "request": {
                            "document_extraction_id": document_extraction_id,
                            "process_id": request.process_id,
                            "notice_id": request.notice_id,
                            "document_set_id": request.document_set_id,
                            "raw_document_id": request.raw_document_id,
                            "skill_id": skill_id,
                            "extraction_scope": "full_document",
                            "target_domain": target_domain,
                            "stage_run_id": stage_run_id,
                        }
                    }),
                    idempotency_key: format!(
                        "extract_document_structure::{}::{}",
                        claimed.id, skill_id
                    ),
                },
            )?;
            job_ids.push(job_id);
        }
    }

    Ok(job_ids)
}

fn bootstrap_runtime(cli: &Cli) -> Result<RuntimeBootstrap> {
    let runtime_paths = RuntimePaths::new(cli.state_root.clone(), cli.runtime_dir.clone());
    let lock_guard = LockGuard::acquire(&runtime_paths)?;
    let mut state_db = open_state_db(runtime_paths.state_db_path(), DEFAULT_SCHEMA_EPOCH)?;
    let listener = bind_listener(&runtime_paths)?;
    let descriptor = DaemonDescriptor::new(
        runtime_paths.socket_path(),
        runtime_paths.state_root(),
        DEFAULT_SCHEMA_EPOCH,
    );
    fs::create_dir_all(&cli.artifact_root)
        .with_context(|| format!("creating {}", cli.artifact_root.display()))?;
    reconcile_interrupted_sessions(&state_db, &cli.artifact_root, &descriptor.instance_id)?;
    ensure_local_executor_registered(state_db.connection())?;
    enqueue_startup_cron_jobs(&mut state_db, Utc::now())?;
    write_descriptor_atomic(runtime_paths.descriptor_path(), &descriptor)?;

    ::tracing::info!(
        socket_path = %runtime_paths.socket_path().display(),
        state_db = %runtime_paths.state_db_path().display(),
        artifact_root = %cli.artifact_root.display(),
        "daemon bootstrap ready"
    );

    Ok(RuntimeBootstrap {
        _lock: lock_guard,
        listener,
        descriptor,
        state_db,
        artifact_root: cli.artifact_root.clone(),
        resource_arbiter: Mutex::new(ResourceArbiter::new(ResourceBudgets::default())),
        restart_controller: RestartController::new(build_restart_plan(cli)?),
    })
}

fn enqueue_startup_cron_jobs(state_db: &mut StateDb, now: DateTime<Utc>) -> Result<()> {
    let source_registry = SourceRegistry::load(state_db.connection())
        .context("loading source registry during daemon bootstrap")?;
    let mut cron_scheduler = CronScheduler::from_registry(&source_registry);
    cron_scheduler
        .refresh_next_run_at(now)
        .context("refreshing cron scheduler definitions during daemon bootstrap")?;
    seed_cron_scheduler_from_history(state_db.connection(), &mut cron_scheduler)
        .context("seeding cron scheduler from workflow history")?;

    let dispatches = cron_scheduler
        .dispatch_due_jobs(&source_registry, now)
        .context("collecting due cron jobs during daemon bootstrap")?;
    if dispatches.is_empty() {
        return Ok(());
    }

    let runner = JobRunner::new(LOCAL_EXECUTOR_ID);
    let mut enqueued = 0_usize;
    for dispatch in dispatches {
        let scheduled_for = dispatch.enqueue_request.scheduled_for.ok_or_else(|| {
            anyhow!(
                "cron dispatch {} missing scheduled_for",
                dispatch.definition_key
            )
        })?;
        runner
            .enqueue_job(state_db, &dispatch.enqueue_request)
            .with_context(|| format!("enqueuing cron root {}", dispatch.definition_key))?;
        cron_scheduler
            .record_enqueued(&dispatch.definition_key, scheduled_for)
            .with_context(|| format!("recording cron enqueue for {}", dispatch.definition_key))?;
        enqueued += 1;
    }

    ::tracing::info!(enqueued, "queued overdue cron jobs during daemon bootstrap");
    Ok(())
}

fn seed_cron_scheduler_from_history(
    conn: &Connection,
    cron_scheduler: &mut CronScheduler,
) -> Result<()> {
    let mut latest_by_definition = BTreeMap::new();
    let mut statement = conn.prepare(
        "\
        SELECT scheduled_for, input_payload
        FROM workflow_jobs
        WHERE scheduled_for IS NOT NULL
        ORDER BY datetime(scheduled_for)
        ",
    )?;
    let rows = statement.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;

    for row in rows {
        let (scheduled_for_raw, input_payload_raw) = row?;
        let scheduled_for = DateTime::parse_from_rfc3339(&scheduled_for_raw)
            .with_context(|| format!("parsing scheduled_for timestamp `{scheduled_for_raw}`"))?
            .with_timezone(&Utc);
        let input_payload = serde_json::from_str::<Value>(&input_payload_raw)
            .context("parsing workflow job input payload during cron bootstrap")?;
        let Some(definition_key) = cron_definition_key_from_payload(&input_payload) else {
            continue;
        };

        latest_by_definition
            .entry(definition_key)
            .and_modify(|seen: &mut DateTime<Utc>| {
                if scheduled_for > *seen {
                    *seen = scheduled_for;
                }
            })
            .or_insert(scheduled_for);
    }

    for (definition_key, scheduled_for) in latest_by_definition {
        if cron_scheduler.definitions().contains_key(&definition_key) {
            cron_scheduler
                .record_enqueued(&definition_key, scheduled_for)
                .with_context(|| format!("restoring cron definition {definition_key}"))?;
        }
    }

    Ok(())
}

fn cron_definition_key_from_payload(input_payload: &Value) -> Option<String> {
    let request = input_payload.get("request")?;
    if let Some(schedule_key) = request.get("schedule_key").and_then(Value::as_str) {
        return Some(schedule_key.to_owned());
    }

    let source_id = request.get("source_id").and_then(Value::as_str)?;
    let skill_id = request.get("skill_id").and_then(Value::as_str)?;
    Some(format!("{source_id}::{skill_id}"))
}

fn ensure_local_executor_registered(conn: &Connection) -> Result<()> {
    conn.execute(
        "\
        INSERT INTO executors (
          id, device_name, status, last_heartbeat_at, owner_id, updated_at
        ) VALUES (?1, ?2, 'ready', CURRENT_TIMESTAMP, ?3, CURRENT_TIMESTAMP)
        ON CONFLICT(id) DO UPDATE SET
          device_name = excluded.device_name,
          status = excluded.status,
          owner_id = excluded.owner_id,
          last_heartbeat_at = CURRENT_TIMESTAMP,
          updated_at = CURRENT_TIMESTAMP
        ",
        params![LOCAL_EXECUTOR_ID, LOCAL_EXECUTOR_ID, DEFAULT_OWNER_ID],
    )?;
    Ok(())
}

fn serve_handshake_once(runtime: &RuntimeBootstrap) -> Result<()> {
    let handshake_context = build_handshake_context(runtime, false)?;
    ipc::handshake::serve_one(&runtime.listener, &handshake_context, None)
}

fn serve_handshake_service(runtime: &mut RuntimeBootstrap) -> Result<()> {
    ::tracing::info!(
        socket_path = %runtime.descriptor.socket_path,
        "daemon service accepting repeated desktop connections"
    );

    loop {
        let (stream, _) = match runtime.listener.accept() {
            Ok(connection) => connection,
            Err(error) if error.kind() == ErrorKind::Interrupted => continue,
            Err(error) => return Err(error).context("accepting persistent handshake connection"),
        };

        let handshake_context = build_handshake_context(runtime, true)?;
        let mut command_router = ipc::command_router::CommandRouter::new(
            ControlHandlerContext {
                descriptor: &runtime.descriptor,
                artifact_root: &runtime.artifact_root,
                restart_controller: &runtime.restart_controller,
            },
            JobsHandlerContext {
                state_db: &mut runtime.state_db,
            },
            handshake_context.operational_status.clone(),
        );
        if let Err(error) =
            ipc::handshake::handle_connection(stream, &handshake_context, Some(&mut command_router))
        {
            ::tracing::warn!(error = %error, "persistent handshake connection failed");
        }

        if runtime.restart_controller.restart_requested() {
            return Ok(());
        }
    }
}

fn build_handshake_context(
    runtime: &RuntimeBootstrap,
    command_router_active: bool,
) -> Result<HandshakeContext> {
    let operational_status = build_operational_status_snapshot(runtime)?;
    Ok(HandshakeContext {
        protocol_version: runtime.descriptor.protocol_version,
        daemon_version: env!("CARGO_PKG_VERSION").to_owned(),
        instance_id: runtime.descriptor.instance_id.clone(),
        schema_epoch: DEFAULT_SCHEMA_EPOCH,
        commit_seq: runtime.state_db.current_commit_seq()?,
        health_state: HealthState {
            status: "ready".to_owned(),
        },
        active_sessions: load_active_sessions(runtime.state_db.connection())?,
        operational_status,
        mutations_allowed: command_router_active,
        expected_peer_uid: None,
        lifecycle_frames: persisted_lifecycle_frames(runtime.state_db.connection())?,
    })
}

fn build_operational_status_snapshot(
    runtime: &RuntimeBootstrap,
) -> Result<OperationalStatusSnapshot> {
    let now = Utc::now();
    Ok(OperationalStatusSnapshot {
        cron: build_cron_status_snapshot(runtime.state_db.connection(), now)?,
        source_health: build_source_health_status_snapshot(runtime.state_db.connection(), now)?,
        validation: build_validation_status_snapshot(runtime.state_db.connection())?,
        resource_arbiter: build_resource_arbiter_status_snapshot(&runtime.resource_arbiter)?,
    })
}

fn build_cron_status_snapshot(conn: &Connection, now: DateTime<Utc>) -> Result<CronStatusSnapshot> {
    let registry = SourceRegistry::load(conn).context("loading source registry for cron status")?;
    let mut scheduler = CronScheduler::from_registry(&registry);
    scheduler
        .refresh_next_run_at(now)
        .context("refreshing cron definitions for status snapshot")?;
    seed_cron_scheduler_from_history(conn, &mut scheduler)
        .context("seeding cron status snapshot from history")?;

    let definitions = scheduler
        .definitions()
        .iter()
        .map(|(definition_key, definition)| CronDefinitionStatus {
            definition_key: definition_key.clone(),
            source_id: definition.source_id.clone(),
            skill_id: definition.skill_id.clone(),
            skill_version: definition.skill_version.clone(),
            cron_expression: definition.cron_expression.clone(),
            enabled: definition.enabled,
            last_run_at: definition.last_run_at.map(|value| value.to_rfc3339()),
            next_run_at: definition.next_run_at.map(|value| value.to_rfc3339()),
        })
        .collect();

    Ok(CronStatusSnapshot { definitions })
}

fn build_source_health_status_snapshot(
    conn: &Connection,
    now: DateTime<Utc>,
) -> Result<SourceHealthStatusSnapshot> {
    let registry =
        SourceRegistry::load(conn).context("loading source registry for source health snapshot")?;
    let mut sources = Vec::new();
    for registered in registry.sources().values() {
        sources.push(SourceHealthStatusEntry {
            source_id: registered.definition.source_id.clone(),
            health_state: enum_label(registered.runtime_state.last_health_state)?,
            circuit_state: enum_label(registered.effective_circuit_state(now))?,
            production_allowed: registered
                .allows_job_class(sources::registry::SourceJobClass::Production, now),
            cooldown_until: registered
                .runtime_state
                .cooldown_until
                .map(|value| value.to_rfc3339()),
            last_canary_at: registered
                .runtime_state
                .last_canary_at
                .map(|value| value.to_rfc3339()),
            last_canary_result: registered
                .runtime_state
                .last_canary_result
                .map(enum_label)
                .transpose()?,
            last_error_code: registered.runtime_state.last_error_code.clone(),
        });
    }
    Ok(SourceHealthStatusSnapshot { sources })
}

fn build_validation_status_snapshot(conn: &Connection) -> Result<ValidationStatusSnapshot> {
    let mut window = validation::executor::ValidationTelemetryWindow::default();
    let mut validator_job_count = 0_u32;
    let mut pending_backlog_count = 0_u32;
    let mut pending_canary_count = 0_u32;

    let mut verdict_statement = conn.prepare(
        "\
        SELECT verdict, override_verdict
        FROM validation_verdicts
        ",
    )?;
    let verdict_rows = verdict_statement.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))
    })?;
    for row in verdict_rows {
        let (verdict, override_verdict) = row?;
        match verdict.as_str() {
            "approved" => window.approved_count += 1,
            "rejected" => window.rejected_count += 1,
            "flagged" => window.flagged_count += 1,
            _ => {}
        }
        if override_verdict.is_some() {
            window.operator_override_count += 1;
        }
    }

    let mut job_statement = conn.prepare(
        "\
        SELECT status, input_payload
        FROM workflow_jobs
        WHERE kind = ?1
        ",
    )?;
    let job_rows = job_statement.query_map(params![VALIDATOR_JOB_KIND], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    for row in job_rows {
        let (status, raw_payload) = row?;
        validator_job_count += 1;
        if matches!(status.as_str(), "completed" | "failed") {
            window.validation_attempt_count += 1;
        }
        if status == "failed" {
            window.execution_failure_count += 1;
        }

        let payload = serde_json::from_str::<Value>(&raw_payload)
            .context("parsing validator workflow input payload for status snapshot")?;
        let request = payload.get("request").unwrap_or(&payload);
        let packet = request.get("validator_packet").unwrap_or(request);
        let purpose = packet
            .get("purpose")
            .and_then(Value::as_str)
            .unwrap_or("live_validation");
        if purpose == "retroactive_backlog" && matches!(status.as_str(), "pending" | "retry") {
            pending_backlog_count += 1;
        }
        if purpose == "synthetic_canary" {
            if matches!(status.as_str(), "completed" | "failed") {
                window.canary_run_count += 1;
            }
            if matches!(status.as_str(), "pending" | "retry") {
                pending_canary_count += 1;
            }
        }
    }

    let telemetry = validation::executor::compute_validation_telemetry(&window, None);
    Ok(ValidationStatusSnapshot {
        validator_approval_rate_bps: telemetry.validator_approval_rate_bps,
        validator_rejection_rate_bps: telemetry.validator_rejection_rate_bps,
        validator_flag_rate_bps: telemetry.validator_flag_rate_bps,
        validator_cost_per_run_pence: telemetry.validator_cost_per_run_pence,
        operator_override_rate_bps: telemetry.operator_override_rate_bps,
        validator_execution_failure_rate_bps: telemetry.validator_execution_failure_rate_bps,
        validator_canary_pass_rate_bps: telemetry.validator_canary_pass_rate_bps,
        validator_health_state: telemetry.validator_health_state.as_str().to_owned(),
        pending_backlog_count,
        pending_canary_count,
        validator_job_count,
    })
}

fn build_resource_arbiter_status_snapshot(
    arbiter: &Mutex<ResourceArbiter>,
) -> Result<ResourceArbiterStatusSnapshot> {
    let (budgets, snapshot, recent_events) = {
        let arbiter = arbiter
            .lock()
            .map_err(|_| anyhow!("resource arbiter mutex poisoned"))?;
        let budgets = arbiter.budgets();
        let snapshot = arbiter.snapshot();
        let recent_events = arbiter
            .events()
            .iter()
            .rev()
            .take(10)
            .map(resource_arbiter_event_snapshot)
            .collect::<Vec<_>>();
        drop(arbiter);
        (budgets, snapshot, recent_events)
    };

    Ok(ResourceArbiterStatusSnapshot {
        browser_slots_limit: budgets.browser_slots,
        browser_slots_in_use: snapshot.browser_slots,
        agent_slots_limit: budgets.agent_slots,
        agent_slots_in_use: snapshot.agent_slots,
        procurement_source_lane_limit: budgets.procurement_source_lanes,
        supplier_source_lane_limit: budgets.supplier_source_lanes,
        source_lanes_in_use: snapshot.source_lanes,
        concurrency_in_use: snapshot.concurrency,
        recent_events,
    })
}

fn resource_arbiter_event_snapshot(event: &ArbiterEvent) -> ResourceArbiterEventSnapshot {
    ResourceArbiterEventSnapshot {
        kind: arbiter_event_kind_label(&event.kind).to_owned(),
        job_id: event.job_id.clone(),
        session_id: event.session_id.clone(),
        priority: admission_priority_label(event.priority).to_owned(),
        denial_reason: event.denial_reason.as_ref().map(denial_reason_label),
        release_cause: event
            .release_cause
            .map(release_cause_label)
            .map(str::to_owned),
        browser_slots_in_use: event.browser_slots_in_use,
        agent_slots_in_use: event.agent_slots_in_use,
        source_lane_key: event.source_lane_key.clone(),
        source_lane_in_use: event.source_lane_in_use,
    }
}

fn enum_label<T: Serialize>(value: T) -> Result<String> {
    serde_json::to_value(value)
        .context("serializing enum label")?
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| anyhow!("serialized enum label was not a string"))
}

const fn arbiter_event_kind_label(kind: &ArbiterEventKind) -> &'static str {
    match kind {
        ArbiterEventKind::Grant => "grant",
        ArbiterEventKind::Deny => "deny",
        ArbiterEventKind::Release => "release",
    }
}

const fn admission_priority_label(priority: AdmissionPriority) -> &'static str {
    match priority {
        AdmissionPriority::ProcurementIngestion => "procurement_ingestion",
        AdmissionPriority::AuthRefreshOrHealthProbe => "auth_refresh_or_health_probe",
        AdmissionPriority::Qualification => "qualification",
        AdmissionPriority::SupplierEnrichment => "supplier_enrichment",
        AdmissionPriority::ManifestGeneration => "manifest_generation",
        AdmissionPriority::BidAutomation => "bid_automation",
    }
}

fn denial_reason_label(reason: &DenialReason) -> String {
    match reason {
        DenialReason::BrowserSlotsExhausted { limit, in_use } => {
            format!("browser_slots_exhausted:{in_use}/{limit}")
        }
        DenialReason::AgentSlotsExhausted { limit, in_use } => {
            format!("agent_slots_exhausted:{in_use}/{limit}")
        }
        DenialReason::SourceLaneExhausted {
            source_id,
            limit,
            in_use,
        } => format!("source_lane_exhausted:{source_id}:{in_use}/{limit}"),
        DenialReason::SkillConcurrencyCapReached { key, limit, in_use } => {
            format!("skill_concurrency_cap_reached:{key}:{in_use}/{limit}")
        }
    }
}

const fn release_cause_label(cause: ReleaseCause) -> &'static str {
    match cause {
        ReleaseCause::PreSpawnFailure => "pre_spawn_failure",
        ReleaseCause::Reaped => "reaped",
        ReleaseCause::Archived => "archived",
    }
}

fn build_restart_plan(cli: &Cli) -> Result<RestartPlan> {
    let program = std::env::current_exe().context("locating current daemon executable")?;
    let args = vec![
        OsString::from("--serve"),
        OsString::from("--state-root"),
        cli.state_root.as_os_str().to_os_string(),
        OsString::from("--runtime-dir"),
        cli.runtime_dir.as_os_str().to_os_string(),
        OsString::from("--artifact-root"),
        cli.artifact_root.as_os_str().to_os_string(),
    ];
    Ok(RestartPlan::new(program, args, DEFAULT_RESTART_DELAY_MS))
}

fn maybe_delay_for_restart() {
    let Ok(raw_delay) = std::env::var(RESTART_DELAY_ENV) else {
        return;
    };
    let Ok(delay_ms) = raw_delay.parse::<u64>() else {
        return;
    };
    if delay_ms > 0 {
        std::thread::sleep(std::time::Duration::from_millis(delay_ms));
    }
}

fn adapter_runtime_config(cli: &Cli, skill: &LoadedSkill) -> Result<AdapterRuntimeConfig> {
    let selected = if let Some(raw) = cli.adapter_choice.as_deref() {
        AgentCliKind::parse(raw)?
    } else if let Some(raw) = skill.metadata.agent_cli.as_deref() {
        AgentCliKind::parse(raw)?
    } else if let Ok(raw) = std::env::var("DAEMON_AGENT_CLI") {
        AgentCliKind::parse(raw.trim())?
    } else {
        AgentCliKind::parse(DEFAULT_AGENT_CLI)?
    };

    let program = cli
        .agent_cli_program
        .clone()
        .or_else(|| std::env::var_os(selected.env_key()).map(PathBuf::from));

    Ok(AdapterRuntimeConfig {
        kind: selected,
        program,
    })
}

fn build_adapter(config: &AdapterRuntimeConfig) -> Box<dyn AgentAdapter> {
    match config.kind {
        AgentCliKind::Claude => Box::new(ClaudeAdapter::new(
            config
                .program
                .clone()
                .unwrap_or_else(|| PathBuf::from(AgentCliKind::Claude.as_str())),
            std::iter::IntoIterator::into_iter(["1.2.3".to_owned(), "1.2.4".to_owned()]).collect(),
        )),
        AgentCliKind::Codex => Box::new(CodexAdapter::new(
            config
                .program
                .clone()
                .unwrap_or_else(|| PathBuf::from(AgentCliKind::Codex.as_str())),
            std::iter::IntoIterator::into_iter(["1.2.3".to_owned(), "1.2.4".to_owned()]).collect(),
        )),
    }
}

fn skill_manifest(skill: &LoadedSkill) -> Result<SkillManifest> {
    let interaction_profile = match skill.metadata.interaction_profile.as_str() {
        "pty_interactive" => InteractionProfile::PtyRequired,
        "headless_contract" => InteractionProfile::ContractRequired,
        other => bail!("unsupported interaction_profile `{other}`"),
    };
    Ok(SkillManifest {
        interaction_profile,
    })
}

fn parse_job_parameters(raw: Option<&str>) -> Result<Value> {
    raw.map_or_else(
        || Ok(json!({})),
        |raw| serde_json::from_str(raw).context("parsing --job-parameters-json"),
    )
}

fn reconcile_interrupted_sessions(
    state_db: &StateDb,
    artifact_root: &Path,
    daemon_instance_id: &str,
) -> Result<()> {
    let reaper = SessionReaper::new(artifact_root)?;
    let interrupted = interrupted_sessions(state_db.connection())?;
    if interrupted.is_empty() {
        return Ok(());
    }

    for session in interrupted {
        let pgid = session
            .pgid
            .ok_or_else(|| anyhow!("interrupted session {} missing pgid", session.id))?;
        let commit_seq = state_db.current_commit_seq()?;
        let result = reaper.reconcile_after_crash(
            &session.id,
            pgid,
            session.pid,
            &CrashRecoveryContext {
                conn: state_db.connection(),
                started_at: session.started_at,
                job_id: session.job_id.as_deref(),
                process_id: None,
                skill_path: Some(Path::new(&session.skill_path)),
                runtime_context: ErrorRuntimeContext {
                    cli_vendor: Some(CLI_VENDOR.to_owned()),
                    cli_version: Some(CLI_VERSION.to_owned()),
                    adapter_version: Some(env!("CARGO_PKG_VERSION").to_owned()),
                    model_id: None,
                    config_hash: None,
                    skill_version: None,
                    pid: session.pid,
                    pgid: session.pgid.map(i32::unsigned_abs),
                    cleanup_verdict: None,
                    last_valid_checkpoint_sequence: session.last_checkpoint_seq,
                    daemon_instance_id: Some(daemon_instance_id.to_owned()),
                    commit_seq: Some(commit_seq),
                },
            },
        )?;
        mark_session_crash_recovered(
            state_db.connection(),
            &session,
            result.cleanup_outcome.cleanup_verdict.as_str(),
            &result.error_code,
        )?;
    }

    Ok(())
}

fn interrupted_sessions(conn: &Connection) -> Result<Vec<PersistedSessionRow>> {
    let mut stmt = conn.prepare(
        "\
        SELECT id, job_id, skill_path, pid, pgid, started_at, status, structured_trace,
               cleanup_verdict, last_checkpoint_seq
        FROM agent_session_logs
        WHERE status IN ('spawned', 'active', 'terminating')
        ORDER BY started_at, id
        ",
    )?;
    let rows = stmt.query_map([], |row| {
        let pid = row
            .get::<_, Option<i64>>(3)?
            .map(u32::try_from)
            .transpose()
            .map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(
                    3,
                    rusqlite::types::Type::Integer,
                    Box::new(error),
                )
            })?;
        let process_group_id = row
            .get::<_, Option<i64>>(4)?
            .map(i32::try_from)
            .transpose()
            .map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(
                    4,
                    rusqlite::types::Type::Integer,
                    Box::new(error),
                )
            })?;
        let started_at = parse_timestamp(&row.get::<_, String>(5)?).map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(
                5,
                rusqlite::types::Type::Text,
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    error.to_string(),
                )),
            )
        })?;
        Ok(PersistedSessionRow {
            id: row.get(0)?,
            job_id: row.get(1)?,
            skill_path: row.get(2)?,
            pid,
            pgid: process_group_id,
            started_at,
            status: row.get(6)?,
            structured_trace: row.get(7)?,
            last_checkpoint_seq: row.get(9)?,
        })
    })?;
    rows.collect::<std::result::Result<Vec<_>, _>>()
        .context("loading interrupted sessions")
}

fn load_active_sessions(conn: &Connection) -> Result<ActiveSessions> {
    let mut stmt = conn.prepare(
        "\
        SELECT id, status
        FROM agent_session_logs
        WHERE status IN ('created', 'spawned', 'active', 'terminating')
        ORDER BY started_at, id
        ",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    let entries = rows
        .collect::<std::result::Result<Vec<_>, _>>()
        .context("loading active sessions")?;
    Ok(ActiveSessions {
        count: entries.len(),
        ids: entries.iter().map(|(id, _)| id.clone()).collect(),
        states: entries.into_iter().map(|(_, state)| state).collect(),
    })
}

fn persisted_lifecycle_frames(conn: &Connection) -> Result<Vec<LifecycleFrame>> {
    let mut stmt = conn.prepare(
        "\
        SELECT id, status, structured_trace, cleanup_verdict
        FROM agent_session_logs
        ORDER BY started_at, id
        ",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, Option<String>>(2)?,
            row.get::<_, Option<String>>(3)?,
        ))
    })?;
    let mut seq = 1_u64;
    let mut frames = Vec::new();
    for row in rows {
        let (session_id, status, structured_trace, cleanup_verdict) = row?;
        for phase in phase_history_from_trace(structured_trace.as_deref(), &status) {
            let lifecycle_phase = parse_lifecycle_phase(&phase)?;
            if lifecycle_phase == LifecyclePhase::Reaped && cleanup_verdict.is_none() {
                continue;
            }
            let archive_seq = if lifecycle_phase == LifecyclePhase::Archived {
                Some(i64::try_from(seq).context("converting lifecycle seq to archive_seq")?)
            } else {
                None
            };
            frames.push(LifecycleFrame {
                seq,
                session_id: session_id.clone(),
                phase: lifecycle_phase,
                cleanup_verdict: if lifecycle_phase == LifecyclePhase::Reaped {
                    cleanup_verdict.clone()
                } else {
                    None
                },
                archive_seq,
            });
            seq += 1;
        }
    }
    Ok(frames)
}

fn mark_session_crash_recovered(
    conn: &Connection,
    session: &PersistedSessionRow,
    cleanup_verdict: &str,
    error_code: &str,
) -> Result<()> {
    let updated = conn.execute(
        "\
        UPDATE agent_session_logs
        SET status = 'failed',
            completed_at = ?1,
            error_type = ?2,
            error_message = ?3,
            cleanup_verdict = ?4,
            structured_trace = ?5
        WHERE id = ?6
        ",
        params![
            Utc::now().to_rfc3339(),
            error_code,
            "daemon restart invalidated the in-flight session; startup crash recovery closed it",
            cleanup_verdict,
            crash_recovered_trace(session),
            session.id,
        ],
    )?;
    ensure!(updated == 1, "missing session row {}", session.id);
    Ok(())
}

fn crash_recovered_trace(session: &PersistedSessionRow) -> String {
    let mut trace = session
        .structured_trace
        .as_deref()
        .and_then(|raw| serde_json::from_str::<Value>(raw).ok())
        .filter(Value::is_object)
        .unwrap_or_else(|| json!({}));
    let mut history =
        phase_history_from_trace(session.structured_trace.as_deref(), &session.status);
    for phase in ["terminating", "exited", "reaped", "archived"] {
        if !history.iter().any(|existing| existing == phase) {
            history.push(phase.to_owned());
        }
    }
    let object = trace
        .as_object_mut()
        .expect("trace object filtered above must remain an object");
    object.insert(
        "phase_history".to_owned(),
        Value::Array(history.into_iter().map(Value::String).collect()),
    );
    object.insert("crash_recovered".to_owned(), Value::Bool(true));
    trace.to_string()
}

fn phase_history_from_trace(structured_trace: Option<&str>, status: &str) -> Vec<String> {
    parse_phase_history(structured_trace, status)
}

fn parse_lifecycle_phase(raw: &str) -> Result<LifecyclePhase> {
    match raw {
        "created" => Ok(LifecyclePhase::Created),
        "spawned" => Ok(LifecyclePhase::Spawned),
        "active" => Ok(LifecyclePhase::Active),
        "terminating" => Ok(LifecyclePhase::Terminating),
        "exited" => Ok(LifecyclePhase::Exited),
        "reaped" => Ok(LifecyclePhase::Reaped),
        "archived" => Ok(LifecyclePhase::Archived),
        other => Err(anyhow!("unknown lifecycle phase `{other}`")),
    }
}

fn parse_timestamp(raw: &str) -> Result<DateTime<Utc>> {
    Ok(DateTime::parse_from_rfc3339(raw)
        .with_context(|| format!("parsing timestamp {raw}"))?
        .with_timezone(&Utc))
}

fn load_fixture(path: Option<&Path>) -> Result<LiveLoopFixture> {
    match path {
        Some(path) => {
            let text =
                fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
            serde_json::from_str(&text).with_context(|| format!("parsing {}", path.display()))
        }
        None => Ok(default_fixture()),
    }
}

fn procurement_manual_source_id(skill_id: &str, job_parameters: &Value) -> String {
    if let Some(source_id) = job_parameters.get("source_id").and_then(Value::as_str) {
        return source_id.to_owned();
    }

    match skill_id {
        "procurement-ingestion/procontract-discover" => "procontract".to_owned(),
        "procurement-ingestion/procontract-register-interest" => "procontract-interest".to_owned(),
        "procurement-ingestion/procontract-enrich-tender" => "procontract-enrich".to_owned(),
        _ => skill_id
            .strip_prefix("procurement-ingestion/")
            .unwrap_or("unknown")
            .to_owned(),
    }
}

fn create_source_sync_job_from_parameters(
    state_db: &mut StateDb,
    source_id: &str,
    skill: &LoadedSkill,
    job_parameters: &Value,
    agent_cli: AgentCliKind,
) -> Result<String> {
    let job_id = format!(
        "job-{}",
        sha256_hex(&UuidSeed::from_time(Utc::now()).as_bytes())
    );
    let ((), _) = state_db.with_write_transaction(|tx| {
        tx.execute(
            "\
            INSERT INTO workflow_jobs (
              id, kind, scope_kind, scope_id, status, input_payload, validation_required,
              parent_job_id, validation_verdict_id
            ) VALUES (?1, 'sync_source', 'source', ?2, 'pending', ?3, 0, NULL, NULL)
            ",
            params![
                job_id,
                source_id,
                json!({
                    "source_id": source_id,
                    "trigger": "manual_cli",
                    "skill_binding": skill_binding_payload(skill),
                    "agent_cli": agent_cli.as_str(),
                    "job_parameters": job_parameters,
                    "durable_step_surfaces": live_loop_surfaces_json(),
                })
                .to_string(),
            ],
        )?;
        Ok(())
    })?;
    Ok(job_id)
}

fn complete_source_job(
    runtime: &mut RuntimeBootstrap,
    job_id: &str,
    summary_payload: &Value,
    summary_line: String,
) -> Result<LiveLoopReport> {
    runtime.state_db.with_write_transaction(|tx| {
        tx.execute(
            "\
            UPDATE workflow_jobs
            SET status = 'completed',
                completed_at = ?1,
                output_payload = ?2
            WHERE id = ?3
            ",
            params![Utc::now().to_rfc3339(), summary_payload.to_string(), job_id],
        )?;
        Ok(())
    })?;

    Ok(LiveLoopReport {
        summary_line,
        status: "completed".to_owned(),
    })
}

fn read_result_contract(result_path: &Path) -> Result<Value> {
    let result_text = fs::read_to_string(result_path)
        .with_context(|| format!("reading result contract from {}", result_path.display()))?;
    serde_json::from_str(&result_text).context("parsing result contract JSON")
}

fn load_procontract_interest_candidates(
    conn: &Connection,
) -> Result<Vec<ProcontractInterestCandidate>> {
    let mut statement = conn.prepare(
        "\
        SELECT cn.id,
               cn.process_id,
               'https://procontract.due-north.com/Advert?advertId=' || cn.source_notice_id,
               cn.title,
               COALESCE(cn.summary, ''),
               COALESCE(cn.main_category, ''),
               cn.deadline_at
        FROM contract_notices cn
        WHERE cn.source_id = 'procontract'
          AND cn.procurement_stage IN ('tender', 'future_opportunity')
          AND NOT EXISTS (
            SELECT 1
            FROM process_stage_status pss
            WHERE pss.process_id = cn.process_id
              AND pss.stage = 'interest_registration'
          )
        ORDER BY COALESCE(cn.deadline_at, '9999-12-31T23:59:59Z'), cn.source_updated_at DESC, cn.id
        ",
    )?;

    statement
        .query_map([], |row| {
            Ok(ProcontractInterestCandidate {
                notice_id: row.get(0)?,
                process_id: row.get(1)?,
                source_url: row.get(2)?,
                title: row.get(3)?,
                summary: row.get(4)?,
                main_category: row.get(5)?,
                deadline_at: row.get(6)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()
        .context("loading ProContract interest-registration candidates")
}

fn build_procontract_interest_evaluations(
    candidates: Vec<ProcontractInterestCandidate>,
) -> Vec<ProcontractInterestEvaluation> {
    candidates
        .into_iter()
        .map(|candidate| {
            let goods_check = is_likely_goods_contract(
                &candidate.main_category,
                &candidate.title,
                &candidate.summary,
            );
            let (expiry_check, days_until_deadline) =
                candidate_deadline_check(candidate.deadline_at.as_deref());
            let action = if !expiry_check || !goods_check {
                "skipped"
            } else {
                "register"
            };
            let skip_reason = if !expiry_check {
                Some("expiry_too_close")
            } else if !goods_check {
                Some("pure_service")
            } else {
                None
            };

            ProcontractInterestEvaluation {
                notice_id: candidate.notice_id,
                process_id: candidate.process_id,
                source_url: candidate.source_url,
                title: candidate.title,
                action,
                skip_reason,
                pre_filter_result: json!({
                    "goods_check": goods_check,
                    "expiry_check": expiry_check,
                    "category": if candidate.main_category.is_empty() {
                        Value::Null
                    } else {
                        Value::String(candidate.main_category)
                    },
                    "days_until_deadline": days_until_deadline,
                }),
            }
        })
        .collect()
}

/// Minimum days remaining on a tender deadline for the interest-registration
/// candidate filter to admit the notice. Previously 30; lowered to 14 on
/// 2026-04-21 because the 30-day rule was starving the pipeline of candidates
/// relative to the deadline mix actually present on procontract (see
/// `docs/stage-9-supplier-price-enrichment-manifest.md` §7 command list for
/// the re-evaluation workflow). The threshold is a business-rule trade-off
/// between bid-prep lead time and pipeline recall; flip back via git history
/// if procurement discovers 14 days is too aggressive for their RFP cycles.
const INTEREST_REGISTRATION_MIN_DEADLINE_DAYS: i64 = 14;

fn candidate_deadline_check(deadline_at: Option<&str>) -> (bool, Option<i64>) {
    let Some(deadline_at) = deadline_at else {
        return (false, None);
    };
    let Ok(deadline) = DateTime::parse_from_rfc3339(deadline_at) else {
        return (false, None);
    };
    let now = Utc::now();
    let deadline_utc = deadline.with_timezone(&Utc);
    let days_until_deadline = deadline_utc.signed_duration_since(now).num_days();
    (
        deadline_utc > now + chrono::Duration::days(INTEREST_REGISTRATION_MIN_DEADLINE_DAYS),
        Some(days_until_deadline),
    )
}

fn is_likely_goods_contract(main_category: &str, title: &str, summary: &str) -> bool {
    let haystack = format!("{main_category}\n{title}\n{summary}").to_ascii_lowercase();
    let positive_keywords = [
        "goods",
        "supply",
        "supplies",
        "equipment",
        "products",
        "product",
        "hardware",
        "furniture",
        "vehicle",
        "vehicles",
        "tyre",
        "tyres",
        "fuel",
        "flooring",
        "signs",
        "water dispensers",
        "sports goods",
        "electrical machinery",
        "consumables",
        "uniform",
        "ppe",
        "cleaning",
        "play equipment",
        "gymnasium",
    ];
    let service_keywords = [
        "consultancy",
        "consultant",
        "design team",
        "design",
        "evaluation",
        "advisory",
        "strategy",
        "appointment",
        "management services",
        "legal services",
        "audit services",
        "architect",
        "engineering services",
        "professional services",
        "training services",
    ];

    let positive_hits = positive_keywords
        .iter()
        .filter(|keyword| haystack.contains(**keyword))
        .count();
    let service_hits = service_keywords
        .iter()
        .filter(|keyword| haystack.contains(**keyword))
        .count();

    positive_hits > 0 || (service_hits == 0 && haystack.contains("cpv"))
}

fn evaluation_report_entry(evaluation: &ProcontractInterestEvaluation) -> Value {
    json!({
        "notice_id": evaluation.notice_id,
        "process_id": evaluation.process_id,
        "source_url": evaluation.source_url,
        "title": evaluation.title,
        "action": if evaluation.action == "register" {
            "pending_registration"
        } else {
            "skipped"
        },
        "skip_reason": evaluation.skip_reason,
        "pre_filter_result": evaluation.pre_filter_result,
    })
}

fn registration_job_entry(evaluation: &ProcontractInterestEvaluation) -> Value {
    json!({
        "notice_id": evaluation.notice_id,
        "process_id": evaluation.process_id,
        "source_url": evaluation.source_url,
        "title": evaluation.title,
        "pre_filter_result": evaluation.pre_filter_result,
    })
}

fn persist_source_cursor(
    tx: &Transaction<'_>,
    source_id: &str,
    cursor_key: &str,
    cursor_value: &str,
) -> Result<()> {
    if cursor_value.is_empty() {
        return Ok(());
    }

    tx.execute(
        "\
        INSERT INTO source_cursors (source_id, cursor_key, cursor_value, updated_at)
        VALUES (?1, ?2, ?3, CURRENT_TIMESTAMP)
        ON CONFLICT(source_id, cursor_key) DO UPDATE SET
          cursor_value = excluded.cursor_value,
          updated_at = excluded.updated_at
        ",
        params![source_id, cursor_key, cursor_value],
    )?;
    Ok(())
}

fn scoped_stage_run_id(base_stage_run_id: &str, scope_id: &str) -> String {
    format!("{base_stage_run_id}-{}", slug_token(scope_id))
}

#[allow(clippy::too_many_arguments)]
fn persist_custom_process_stage_state(
    tx: &Transaction<'_>,
    process_id: &str,
    stage: &str,
    scope_kind: &str,
    scope_id: &str,
    stage_run_id: &str,
    stage_state: &str,
    visible_after_seq: i64,
) -> Result<()> {
    let now = Utc::now().to_rfc3339();
    tx.execute(
        "\
        INSERT INTO process_stage_status (
          id, process_id, stage, scope_kind, scope_id, stage_run_id, state,
          freshness_state, started_at, completed_at, owner_id, row_version, last_local_seq
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'fresh', ?8, ?9, ?10, 1, ?11)
        ",
        params![
            format!("{stage_run_id}-{stage_state}"),
            process_id,
            stage,
            scope_kind,
            scope_id,
            stage_run_id,
            stage_state,
            now,
            now,
            DEFAULT_OWNER_ID,
            visible_after_seq,
        ],
    )?;

    if stage_state != "failed" {
        tx.execute(
            "\
            INSERT INTO process_stage_heads (
              process_id, stage, scope_kind, scope_id, current_stage_run_id,
              freshness_state, owner_id, row_version, last_local_seq
            ) VALUES (?1, ?2, ?3, ?4, ?5, 'fresh', ?6, 1, ?7)
            ON CONFLICT(process_id, stage, scope_kind, scope_id) DO UPDATE SET
              current_stage_run_id = excluded.current_stage_run_id,
              freshness_state = excluded.freshness_state,
              owner_id = excluded.owner_id,
              row_version = process_stage_heads.row_version + 1,
              last_local_seq = excluded.last_local_seq,
              updated_at = CURRENT_TIMESTAMP
            ",
            params![
                process_id,
                stage,
                scope_kind,
                scope_id,
                stage_run_id,
                DEFAULT_OWNER_ID,
                visible_after_seq,
            ],
        )?;
    }

    Ok(())
}

fn resolve_process_id_for_notice(
    tx: &Transaction<'_>,
    notice_id: &str,
    hinted_process_id: Option<&str>,
) -> Result<String> {
    if let Some(process_id) = hinted_process_id {
        return Ok(process_id.to_owned());
    }

    tx.query_row(
        "SELECT process_id FROM contract_notices WHERE id = ?1",
        params![notice_id],
        |row| row.get(0),
    )
    .with_context(|| format!("loading process_id for notice {notice_id}"))
}

/// Resolve `notice_id` and `process_id` for enrichment results that may use
/// portal reference numbers instead of the canonical `pc-{advertId}` format.
/// Falls back to a title-based lookup within the procontract source when the
/// direct id lookup fails.
fn resolve_enrichment_notice(
    tx: &Transaction<'_>,
    notice_id: &str,
    title: &str,
    hinted_process_id: Option<&str>,
) -> Result<(String, String)> {
    if let Some(process_id) = hinted_process_id {
        return Ok((notice_id.to_owned(), process_id.to_owned()));
    }

    if let Ok(process_id) = tx.query_row(
        "SELECT process_id FROM contract_notices WHERE id = ?1",
        params![notice_id],
        |row| row.get::<_, String>(0),
    ) {
        return Ok((notice_id.to_owned(), process_id));
    }

    let prefixed = format!("pc-{notice_id}");
    if let Ok(process_id) = tx.query_row(
        "SELECT process_id FROM contract_notices WHERE id = ?1",
        params![prefixed],
        |row| row.get::<_, String>(0),
    ) {
        return Ok((prefixed, process_id));
    }

    if !title.is_empty() {
        // Exact title match
        if let Ok((resolved_id, process_id)) = tx.query_row(
            "SELECT id, process_id FROM contract_notices \
             WHERE source_id = 'procontract' AND title = ?1 LIMIT 1",
            params![title],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        ) {
            return Ok((resolved_id, process_id));
        }

        // Prefix match: portal appends " - Acceptance Round N" or similar suffixes
        if let Ok((resolved_id, process_id)) = tx.query_row(
            "SELECT id, process_id FROM contract_notices \
             WHERE source_id = 'procontract' AND ?1 LIKE title || '%' LIMIT 1",
            params![title],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        ) {
            return Ok((resolved_id, process_id));
        }
    }

    bail!("cannot resolve ProContract notice for id={notice_id} title={title}")
}

fn load_notice_payload(tx: &Transaction<'_>, notice_id: &str) -> Result<Option<Value>> {
    let payload: Option<String> = tx
        .query_row(
            "SELECT normalized_payload FROM contract_notices WHERE id = ?1",
            params![notice_id],
            |row| row.get(0),
        )
        .optional()?;
    payload
        .map(|raw| {
            serde_json::from_str::<Value>(&raw)
                .context("parsing contract_notices.normalized_payload")
        })
        .transpose()
}

fn merge_notice_payload(existing: Option<Value>, updates: &Value) -> Value {
    let mut base = existing
        .and_then(|value| value.as_object().cloned())
        .unwrap_or_default();
    if let Some(update_object) = updates.as_object() {
        for (key, value) in update_object {
            base.insert(key.clone(), value.clone());
        }
    }
    Value::Object(base)
}

fn infer_document_role_hint(filename: &str) -> Option<String> {
    let normalized = filename.to_ascii_lowercase();
    if normalized.contains("invitation to tender") || normalized.contains("itt") {
        Some("invitation_to_tender".to_owned())
    } else if normalized.contains("specification") || normalized.contains("brief") {
        Some("technical_specification".to_owned())
    } else if normalized.contains("pricing schedule") || normalized.contains("schedule") {
        Some("pricing_schedule".to_owned())
    } else if normalized.contains("response") || normalized.contains("submission") {
        Some("response_document".to_owned())
    } else if normalized.contains("terms") || normalized.contains("t&c") {
        Some("terms_and_conditions".to_owned())
    } else if Path::new(&normalized)
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("zip"))
    {
        Some("archive".to_owned())
    } else {
        None
    }
}

fn size_bytes_for_download(document: &ProcontractDownloadedDocument) -> Option<i64> {
    document.size_bytes.or_else(|| {
        fs::metadata(&document.local_path)
            .ok()
            .and_then(|metadata| i64::try_from(metadata.len()).ok())
    })
}

fn persist_interest_registrations(
    state_db: &mut StateDb,
    source_id: &str,
    result_json: &Value,
) -> Result<usize> {
    let contract: ProcontractRegistrationContract = serde_json::from_value(result_json.clone())
        .context("parsing ProContract registration result contract")?;
    let stage_run_id = allocate_stage_run_id();

    let (persisted_count, _) = state_db.with_write_transaction(|tx| {
        let visible_after_seq = next_commit_seq(tx)?;
        for registration in &contract.registrations {
            let process_id = resolve_process_id_for_notice(
                tx,
                &registration.notice_id,
                registration.process_id.as_deref(),
            )?;
            let scoped_run_id = scoped_stage_run_id(&stage_run_id, &process_id);
            let state = match registration.action.as_str() {
                "registered" | "already_registered" => "completed",
                "skipped" => "skipped",
                "failed" => "failed",
                other => bail!("unsupported ProContract registration action `{other}`"),
            };
            persist_custom_process_stage_state(
                tx,
                &process_id,
                "interest_registration",
                "process",
                &process_id,
                &scoped_run_id,
                state,
                visible_after_seq,
            )?;
        }
        persist_source_cursor(tx, source_id, "interest_cursor", &contract.cursor)?;
        Ok(contract.registrations.len())
    })?;

    Ok(persisted_count)
}

#[allow(clippy::too_many_lines)]
fn persist_tender_enrichments(
    state_db: &mut StateDb,
    source_id: &str,
    result_json: &Value,
) -> Result<usize> {
    let contract: ProcontractEnrichmentContract = serde_json::from_value(result_json.clone())
        .context("parsing ProContract enrichment result contract")?;
    let stage_run_id = allocate_stage_run_id();

    let (persisted_count, _) = state_db.with_write_transaction(|tx| {
        let visible_after_seq = next_commit_seq(tx)?;
        for tender in &contract.enriched_tenders {
            let Ok((resolved_notice_id, process_id)) = resolve_enrichment_notice(
                tx,
                &tender.notice_id,
                &tender.updated_title,
                tender.process_id.as_deref(),
            ) else {
                continue;
            };
            let merged_payload = merge_notice_payload(
                load_notice_payload(tx, &resolved_notice_id)?,
                &json!({
                    "rfx_id": tender.rfx_id,
                    "rfx_url": tender.rfx_url,
                    "buyer_contact": tender.buyer_contact,
                    "documents_downloaded": tender.documents_downloaded,
                    "documents_count": tender.documents_count,
                    "enriched_at": tender.enriched_at,
                    "updated_title": tender.updated_title,
                    "updated_description": tender.updated_description,
                    "updated_deadline_at": tender.updated_deadline_at,
                }),
            );
            let merged_payload_json = serde_json::to_string(&merged_payload)
                .context("serializing merged notice payload")?;
            let scoped_run_id = scoped_stage_run_id(&stage_run_id, &process_id);

            tx.execute(
                "\
                UPDATE procurement_processes
                SET title = ?1,
                    buyer_name = ?2,
                    latest_deadline_at = ?3,
                    latest_updated_at = ?4,
                    row_version = procurement_processes.row_version + 1,
                    last_local_seq = ?5,
                    stage_run_id = ?6,
                    updated_at = CURRENT_TIMESTAMP
                WHERE id = ?7
                ",
                params![
                    tender.updated_title,
                    tender.buyer_name,
                    tender.updated_deadline_at,
                    tender.enriched_at,
                    visible_after_seq,
                    scoped_run_id,
                    process_id,
                ],
            )?;

            tx.execute(
                "\
                UPDATE contract_notices
                SET title = ?1,
                    summary = ?2,
                    buyer_name = ?3,
                    deadline_at = ?4,
                    source_updated_at = ?5,
                    normalized_payload = ?6,
                    row_version = contract_notices.row_version + 1,
                    last_local_seq = ?7,
                    stage_run_id = ?8,
                    updated_at = CURRENT_TIMESTAMP
                WHERE id = ?9
                ",
                params![
                    tender.updated_title,
                    tender.updated_description,
                    tender.buyer_name,
                    tender.updated_deadline_at,
                    tender.enriched_at,
                    merged_payload_json,
                    visible_after_seq,
                    scoped_run_id,
                    resolved_notice_id,
                ],
            )?;

            let mut document_rows = Vec::new();
            let mut document_roles = Map::new();
            let mut has_pdf_document = false;
            for (index, document) in tender.documents_downloaded.iter().enumerate() {
                let size_bytes = size_bytes_for_download(document);
                let role_hint = document
                    .document_role_hint
                    .clone()
                    .or_else(|| infer_document_role_hint(&document.filename));
                let raw_document_id = format!(
                    "raw-{}-{}",
                    resolved_notice_id,
                    sha256_hex(
                        format!("{}:{}:{}", document.filename, document.local_path, index)
                            .as_bytes()
                    )
                );
                let metadata = json!({
                    "filename": document.filename,
                    "size_bytes": size_bytes,
                    "document_role_hint": role_hint,
                    "download_url": document.download_url,
                    "extracted_from_archive": document.extracted_from_archive,
                    "rfx_id": tender.rfx_id,
                    "rfx_url": tender.rfx_url,
                    "last_local_seq": visible_after_seq,
                });

                tx.execute(
                    "\
                    INSERT INTO raw_documents (
                      id, source_id, content_type, source_url, storage_path, metadata, created_at
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, CURRENT_TIMESTAMP)
                    ON CONFLICT(id) DO UPDATE SET
                      content_type = excluded.content_type,
                      source_url = excluded.source_url,
                      storage_path = excluded.storage_path,
                      metadata = excluded.metadata
                    ",
                    params![
                        raw_document_id,
                        source_id,
                        document.content_type,
                        document.download_url.as_deref().unwrap_or(&tender.rfx_url),
                        document.local_path,
                        metadata.to_string(),
                    ],
                )?;

                if is_extractable_content_type(&document.content_type) {
                    has_pdf_document = true;
                }

                if let Some(role_hint) = role_hint.clone() {
                    document_roles.insert(raw_document_id.clone(), Value::String(role_hint));
                }

                document_rows.push(json!({
                    "raw_document_id": raw_document_id,
                    "filename": document.filename,
                    "content_type": document.content_type,
                    "size_bytes": size_bytes,
                    "storage_path": document.local_path,
                    "source_url": document.download_url.as_deref().unwrap_or(&tender.rfx_url),
                    "document_role_hint": role_hint,
                    "extracted_from_archive": document.extracted_from_archive.unwrap_or(false),
                }));
            }

            let documents_json =
                serde_json::to_string(&document_rows).context("serializing tender documents")?;
            let document_roles_json = Value::Object(document_roles).to_string();
            let document_set_hash =
                sha256_hex(format!("{}:{}", tender.rfx_id, documents_json).as_bytes());
            let completeness_state = if document_rows.is_empty() {
                "none"
            } else {
                "complete"
            };
            let extraction_state = if has_pdf_document { "pending" } else { "none" };

            tx.execute(
                "\
                INSERT INTO tender_document_sets (
                  id, process_id, notice_id, document_set_hash, completeness_state,
                  document_roles_json, documents_json, acquired_at, extraction_state,
                  owner_id, row_version, last_local_seq, stage_run_id
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 1, ?11, ?12)
                ON CONFLICT(id) DO UPDATE SET
                  document_set_hash = excluded.document_set_hash,
                  completeness_state = excluded.completeness_state,
                  document_roles_json = excluded.document_roles_json,
                  documents_json = excluded.documents_json,
                  acquired_at = excluded.acquired_at,
                  extraction_state = excluded.extraction_state,
                  row_version = tender_document_sets.row_version + 1,
                  last_local_seq = excluded.last_local_seq,
                  stage_run_id = excluded.stage_run_id,
                  updated_at = CURRENT_TIMESTAMP
                ",
                params![
                    format!("tds-{}", resolved_notice_id),
                    process_id,
                    resolved_notice_id,
                    document_set_hash,
                    completeness_state,
                    document_roles_json,
                    documents_json,
                    tender.enriched_at,
                    extraction_state,
                    DEFAULT_OWNER_ID,
                    visible_after_seq,
                    scoped_run_id,
                ],
            )?;

            persist_custom_process_stage_state(
                tx,
                &process_id,
                "tender_enrichment",
                "process",
                &process_id,
                &scoped_run_id,
                "completed",
                visible_after_seq,
            )?;

            let document_set_id = format!("tds-{resolved_notice_id}");
            enqueue_extraction_jobs_for_document_set(
                tx,
                &document_rows,
                &process_id,
                &resolved_notice_id,
                &document_set_id,
                &scoped_run_id,
            )?;
        }

        persist_source_cursor(tx, source_id, "enrich_cursor", &contract.cursor)?;
        Ok(contract.enriched_tenders.len())
    })?;

    Ok(persisted_count)
}

/// Returns `true` if the content type is extractable by the Docling pipeline.
fn is_extractable_content_type(content_type: &str) -> bool {
    matches!(
        content_type,
        "application/pdf"
            | "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
            | "application/msword"
            | "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
            | "application/vnd.ms-excel"
    )
}

/// Enqueue `extract_document_structure` workflow jobs for every extractable
/// document in a set so the extraction pipeline picks them up automatically.
/// Uses an idempotency key per `raw_document` so re-runs are safe.
fn enqueue_extraction_jobs_for_document_set(
    tx: &Transaction<'_>,
    document_rows: &[Value],
    process_id: &str,
    notice_id: &str,
    document_set_id: &str,
    stage_run_id: &str,
) -> Result<usize> {
    let mut enqueued = 0_usize;
    for doc in document_rows {
        let content_type = doc
            .get("content_type")
            .and_then(Value::as_str)
            .unwrap_or("");
        if !is_extractable_content_type(content_type) {
            continue;
        }
        let Some(raw_document_id) = doc.get("raw_document_id").and_then(Value::as_str) else {
            continue;
        };
        let job_id = format!("job-extract-{}", sha256_hex(raw_document_id.as_bytes()));
        let _idempotency_key = format!("extract_document_structure::{raw_document_id}");

        // Skip if a job with this idempotency key already exists.
        let exists: bool = tx
            .query_row(
                "SELECT COUNT(*) > 0 FROM workflow_jobs WHERE id = ?1",
                params![job_id],
                |row| row.get(0),
            )
            .unwrap_or(false);
        if exists {
            continue;
        }

        tx.execute(
            "\
            INSERT INTO workflow_jobs (
              id, kind, process_id, notice_id, scope_kind, scope_id,
              status, input_payload, validation_required
            ) VALUES (?1, 'extract_document_structure', ?2, ?3, 'notice', ?4,
                      'pending', ?5, 0)
            ",
            params![
                job_id,
                process_id,
                notice_id,
                notice_id,
                json!({
                    "raw_document_id": raw_document_id,
                    "process_id": process_id,
                    "notice_id": notice_id,
                    "document_set_id": document_set_id,
                    "stage_run_id": stage_run_id,
                })
                .to_string(),
            ],
        )?;
        enqueued += 1;
    }
    Ok(enqueued)
}

/// Returns `true` when `skill_id` is a procurement-ingestion skill whose job
/// must fail loudly if the agent exited without writing a result contract.
/// Any other skill category (validation, pdf-extraction, etc.) is allowed to
/// complete without a result artefact.
fn procurement_skill_requires_result_contract(skill_id: &str) -> bool {
    skill_id.starts_with("procurement-ingestion/")
}

fn manual_procurement_persist_count_label(skill_id: &str) -> &'static str {
    match skill_id {
        "procurement-ingestion/procontract-register-interest" => "registrations",
        "procurement-ingestion/procontract-enrich-tender" => "enriched tenders",
        _ => "notices",
    }
}

fn persist_procurement_result_for_skill(
    state_db: &mut StateDb,
    skill_id: &str,
    job_parameters: &Value,
    result_json: &Value,
) -> Result<(String, usize)> {
    let source_id = procurement_manual_source_id(skill_id, job_parameters);
    let persisted_count = match skill_id {
        "procurement-ingestion/procontract-register-interest" => {
            persist_interest_registrations(state_db, &source_id, result_json)?
        }
        "procurement-ingestion/procontract-enrich-tender" => {
            persist_tender_enrichments(state_db, &source_id, result_json)?
        }
        _ => persist_ingestion_from_agent_output(state_db, &source_id, result_json)?,
    };
    Ok((source_id, persisted_count))
}

#[allow(clippy::future_not_send, clippy::too_many_lines)]
async fn run_bound_skill(
    runtime: &mut RuntimeBootstrap,
    skill_path: &Path,
    job_parameters: Value,
    cli: &Cli,
) -> Result<LiveLoopReport> {
    let skill = load_skill_from_path(skill_path)?;
    let adapter_config = adapter_runtime_config(cli, &skill)?;
    let job_id = create_manual_skill_job(
        &mut runtime.state_db,
        &skill,
        &job_parameters,
        adapter_config.kind,
    )?;
    let session_id = format!("session-manual-{}", sha256_hex(job_id.as_bytes()));
    let executed =
        execute_skill_session(runtime, &skill, &job_id, &session_id, &job_parameters, cli).await?;
    insert_executed_session_log(runtime.state_db.connection(), &skill, &job_id, &executed)?;

    let is_procurement_skill = procurement_skill_requires_result_contract(&skill.metadata.skill_id);

    if is_procurement_skill && executed.result_path.is_none() {
        let message = format!(
            "procurement skill `{}` exited without writing a result contract",
            skill.metadata.skill_id
        );
        let source_id = procurement_manual_source_id(&skill.metadata.skill_id, &job_parameters);
        return fail_with_manifest(
            runtime,
            &job_id,
            ErrorCode::AgentRunFailed,
            &message,
            None,
            Some(&session_id),
            Some(&skill.skill_path),
            "ingest",
            "agent_execute",
            json!({
                "skill_id": skill.metadata.skill_id,
                "source_id": source_id,
                "adapter_id": executed.adapter_id,
                "execution_mode": execution_mode_label(executed.execution_mode),
            }),
        );
    }

    if is_procurement_skill && let Some(ref result_path) = executed.result_path {
        let result_json = read_result_contract(result_path)?;
        let (source_id, persisted_count) = persist_procurement_result_for_skill(
            &mut runtime.state_db,
            &skill.metadata.skill_id,
            &job_parameters,
            &result_json,
        )?;
        eprintln!(
            "Persisted {persisted_count} {} from agent output for source {source_id}",
            manual_procurement_persist_count_label(&skill.metadata.skill_id)
        );
    } else if skill.metadata.skill_id.starts_with("pdf-extraction/")
        && let Some(ref result_path) = executed.result_path
    {
        let result_text = fs::read_to_string(result_path).with_context(|| {
            format!(
                "reading semantic extraction result contract from {}",
                result_path.display()
            )
        })?;
        let result_json: Value = serde_json::from_str(&result_text)
            .context("parsing semantic extraction result JSON")?;
        let document_extraction_id = job_parameters
            .get("document_extraction_id")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("pdf-extraction manual run missing document_extraction_id"))?;
        let process_id = job_parameters
            .get("process_id")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("pdf-extraction manual run missing process_id"))?;
        persist_extraction_from_agent_output(
            &mut runtime.state_db,
            document_extraction_id,
            process_id,
            &skill,
            &job_id,
            &session_id,
            &result_json,
        )?;
    }

    runtime.state_db.with_write_transaction(|tx| {
        tx.execute(
            "\
            UPDATE workflow_jobs
            SET status = 'completed',
                completed_at = ?1,
                output_payload = ?2
            WHERE id = ?3
            ",
            params![
                Utc::now().to_rfc3339(),
                json!({
                    "job_id": job_id,
                    "skill_id": skill.metadata.skill_id,
                    "execution_mode": execution_mode_label(executed.execution_mode),
                    "adapter_id": executed.adapter_id,
                    "session_id": session_id,
                })
                .to_string(),
                job_id,
            ],
        )?;
        Ok(())
    })?;

    Ok(LiveLoopReport {
        summary_line: format!(
            "Skill run completed: job_id={} skill_id={} session_id={} adapter={}",
            job_id, skill.metadata.skill_id, session_id, executed.adapter_id
        ),
        status: "completed".to_owned(),
    })
}

#[allow(clippy::future_not_send)]
async fn execute_skill_session(
    runtime: &RuntimeBootstrap,
    skill: &LoadedSkill,
    job_id: &str,
    session_id: &str,
    job_parameters: &Value,
    cli: &Cli,
) -> Result<ExecutedSession> {
    let adapter_config = adapter_runtime_config(cli, skill)?;
    let adapter = build_adapter(&adapter_config);
    let probe_ctx = ProbeCtx {
        working_directory: repo_root()?,
    };
    let caps = adapter.probe(&probe_ctx).await?;
    let mode = adapter.resolve_mode(&skill_manifest(skill)?, &caps)?;
    let started_at = Utc::now();
    let session_dir =
        ArtifactStore::new(&runtime.artifact_root)?.session_dir(session_id, started_at)?;
    let workdir = session_dir.join("workdir");
    fs::create_dir_all(&workdir).with_context(|| format!("creating {}", workdir.display()))?;
    let prompt = PromptRenderer::render(&PromptRendererInput {
        skill_id: skill.metadata.skill_id.clone(),
        skill_version: skill.metadata.skill_version.clone(),
        skill_markdown: skill.skill_markdown.clone(),
        job_id: job_id.to_owned(),
        session_id: session_id.to_owned(),
        execution_mode: execution_mode_label(mode).to_owned(),
        job_parameters: job_parameters.clone(),
        resume_token: None,
        company_profile: None,
    })?;
    let request = SpawnRequest {
        program: adapter_config
            .program
            .clone()
            .unwrap_or_else(|| PathBuf::from(adapter_config.kind.as_str())),
        args: launch_args_for_skill(&adapter_config, mode, &prompt),
        env: std::collections::BTreeMap::default(),
        cwd: workdir,
    };
    let launch_spec = adapter.build_launch_spec(&request, mode)?;
    let config_hash = sha256_hex(
        format!(
            "{}:{}",
            skill_config_hash(skill),
            launch_spec_hash(&launch_spec)
        )
        .as_bytes(),
    );
    let trace_identity = trace_identity_for(job_id, session_id, job_parameters);
    let forensic_identity = forensic_identity_for(job_id, session_id, job_parameters);
    let _lease_heartbeat = claimed_job_lease_heartbeat(runtime, job_id)?;
    let mut resource_lease =
        claimed_job_resource_lease(runtime, skill, job_id, session_id, mode, job_parameters)?;

    let runtime_trace_identity = trace_identity.clone();
    let mut result = match mode {
        ExecutionMode::Pty => {
            execute_pty_skill_session(
                &runtime.artifact_root,
                trace_identity.clone(),
                forensic_identity.clone(),
                session_id,
                &caps,
                adapter.as_ref(),
                &launch_spec,
                started_at,
                config_hash,
            )
            .await
        }
        ExecutionMode::Contract => {
            execute_contract_skill_session(
                &runtime.artifact_root,
                trace_identity,
                forensic_identity,
                session_id,
                &caps,
                adapter.as_ref(),
                &launch_spec,
                started_at,
                config_hash,
            )
            .await
        }
    };

    if result.is_ok()
        && let Some(resource_lease) = resource_lease.as_mut()
    {
        resource_lease.set_release_cause(ReleaseCause::Archived);
    }

    drop(resource_lease);

    if let Ok(executed) = result.as_mut() {
        executed.structured_trace = append_runtime_control_records(
            &executed.structured_trace,
            &runtime_trace_identity,
            executed.execution_mode,
            &runtime.resource_arbiter,
            executed.runtime_ids,
            &executed.cleanup_verdict,
        )?;
    }

    result
}

#[allow(dead_code)]
fn enqueue_validator_followup(
    runtime: &mut RuntimeBootstrap,
    runner: &JobRunner,
    mut request: ValidatorEnqueueRequest,
) -> Result<EnqueuedValidatorJob> {
    if request.idempotency_key.is_none() {
        request.idempotency_key = Some(format!(
            "{VALIDATOR_JOB_KIND}:{}:{}",
            request.worker_job_id, request.run_manifest_hash
        ));
    }
    runner.enqueue_validation_sub_job(&repo_root()?, &mut runtime.state_db, &request)
}

#[allow(dead_code)]
fn enqueue_validation_canary(
    runtime: &mut RuntimeBootstrap,
    runner: &JobRunner,
    mut request: ValidatorEnqueueRequest,
    canary_case: jobs::runner::ValidationCanaryCase,
) -> Result<EnqueuedValidatorJob> {
    if request.idempotency_key.is_none() {
        request.idempotency_key = Some(format!(
            "{VALIDATOR_JOB_KIND}:canary:{}:{}",
            request.worker_job_id, canary_case.case_id
        ));
    }
    runner.enqueue_validation_canary_job(&repo_root()?, &mut runtime.state_db, request, canary_case)
}

#[allow(dead_code)]
fn enqueue_retroactive_validation(
    runtime: &mut RuntimeBootstrap,
    runner: &JobRunner,
    mut request: ValidatorEnqueueRequest,
    backlog: jobs::runner::RetroactiveBacklogMetadata,
) -> Result<EnqueuedValidatorJob> {
    if request.idempotency_key.is_none() {
        request.idempotency_key = Some(format!(
            "{VALIDATOR_JOB_KIND}:retroactive:{}:{}",
            request.worker_job_id, backlog.observed_at
        ));
    }
    runner.enqueue_retroactive_backlog_job(&repo_root()?, &mut runtime.state_db, request, backlog)
}

fn load_live_validation_job_parameters(conn: &Connection, worker_job_id: &str) -> Result<Value> {
    let raw_payload: String = conn
        .query_row(
            "SELECT input_payload FROM workflow_jobs WHERE id = ?1",
            params![worker_job_id],
            |row| row.get(0),
        )
        .with_context(|| format!("loading workflow job input payload for {worker_job_id}"))?;
    let payload = serde_json::from_str::<Value>(&raw_payload)
        .with_context(|| format!("parsing workflow job input payload for {worker_job_id}"))?;
    Ok(payload.get("request").cloned().unwrap_or(payload))
}

fn live_validation_source_id(skill: &LoadedSkill, worker_job_parameters: &Value) -> String {
    worker_job_parameters
        .get("source_id")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .or_else(|| registry_source_id_for_skill(&skill.metadata.skill_id).map(str::to_owned))
        .unwrap_or_else(|| skill_source_scope(&skill.metadata.skill_id))
}

fn load_live_source_health(conn: &Connection, source_id: &str) -> Result<PolicyHealthState> {
    let registry = SourceRegistry::load(conn)
        .context("loading source registry for validation policy context")?;
    Ok(
        match registry
            .get(source_id)
            .map(|source| source.runtime_state.last_health_state)
        {
            Some(RuntimeHealthState::Amber) => PolicyHealthState::Amber,
            Some(RuntimeHealthState::Red) => PolicyHealthState::Red,
            _ => PolicyHealthState::Green,
        },
    )
}

fn load_run_manifest_appendix(
    manifest_path: &Path,
) -> Result<agents::artifacts::DaemonEvidenceAppendix> {
    let markdown = fs::read_to_string(manifest_path)
        .with_context(|| format!("reading {}", manifest_path.display()))?;
    let marker = "## Daemon Evidence Appendix\n```yaml\n";
    let start = markdown
        .find(marker)
        .ok_or_else(|| anyhow!("run manifest missing daemon evidence appendix"))?
        + marker.len();
    let end = markdown[start..]
        .find("\n```")
        .ok_or_else(|| anyhow!("run manifest daemon evidence appendix missing closing fence"))?
        + start;
    serde_yaml::from_str(&markdown[start..end]).with_context(|| {
        format!(
            "parsing daemon evidence appendix from {}",
            manifest_path.display()
        )
    })
}

fn recent_completed_live_outputs(
    conn: &Connection,
    source_id: &str,
    exclude_job_id: &str,
    limit: usize,
) -> Result<Vec<Value>> {
    let mut statement = conn.prepare(
        "\
        SELECT id, output_payload
        FROM workflow_jobs
        WHERE parent_job_id IS NULL
          AND status = 'completed'
          AND output_payload IS NOT NULL
        ORDER BY rowid DESC
        ",
    )?;
    let rows = statement.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;

    let mut outputs = Vec::new();
    for row in rows {
        let (job_id, raw_payload) = row?;
        if job_id == exclude_job_id {
            continue;
        }
        let payload = serde_json::from_str::<Value>(&raw_payload)
            .with_context(|| format!("parsing completed workflow output payload for {job_id}"))?;
        if payload.get("source_id").and_then(Value::as_str) != Some(source_id) {
            continue;
        }
        outputs.push(payload);
        if outputs.len() == limit {
            break;
        }
    }

    Ok(outputs)
}

fn recent_skill_telemetry_entry<'a>(payload: &'a Value, skill_id: &str) -> Option<&'a Value> {
    payload
        .get("skill_telemetry")
        .and_then(Value::as_array)?
        .iter()
        .find(|entry| entry.get("skill_id").and_then(Value::as_str) == Some(skill_id))
}

fn recent_fill_rate_baseline_bps(outputs: &[Value], skill_id: &str) -> Option<u16> {
    let fill_rates = outputs
        .iter()
        .filter_map(|payload| recent_skill_telemetry_entry(payload, skill_id))
        .filter_map(|entry| {
            entry["telemetry"]["extraction_fill_rate_bps"]
                .as_u64()
                .and_then(|value| u16::try_from(value).ok())
        })
        .collect::<Vec<_>>();
    if fill_rates.is_empty() {
        return None;
    }

    let sum = fill_rates
        .iter()
        .fold(0_u32, |acc, value| acc.saturating_add(u32::from(*value)));
    let average = sum / u32::try_from(fill_rates.len()).ok()?;
    u16::try_from(average).ok()
}

fn previous_worker_behavior_version(outputs: &[Value], skill_id: &str) -> Option<String> {
    outputs
        .iter()
        .filter_map(|payload| recent_skill_telemetry_entry(payload, skill_id))
        .find_map(|entry| {
            entry
                .get("worker_behavior_version")
                .and_then(Value::as_str)
                .map(str::to_owned)
        })
}

fn suspect_output_count_last_five(outputs: &[Value]) -> u8 {
    let count = outputs
        .iter()
        .take(5)
        .filter(|payload| {
            matches!(
                payload.get("trust_state").and_then(Value::as_str),
                Some("suspect" | "flagged")
            )
        })
        .count();
    u8::try_from(count).unwrap_or(u8::MAX)
}

fn parse_policy_basis_sufficiency(raw: &str) -> Option<PolicyBasisSufficiency> {
    match raw {
        "high" => Some(PolicyBasisSufficiency::High),
        "medium" => Some(PolicyBasisSufficiency::Medium),
        "low" => Some(PolicyBasisSufficiency::Low),
        _ => None,
    }
}

fn recent_basis_sufficiency_for_source(
    conn: &Connection,
    source_id: &str,
    exclude_job_id: &str,
) -> Result<Option<PolicyBasisSufficiency>> {
    let mut statement = conn.prepare(
        "\
        SELECT worker_job_id, basis_sufficiency
        FROM validation_verdicts
        ORDER BY datetime(verdict_rendered_at) DESC, rowid DESC
        ",
    )?;
    let rows = statement.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;

    for row in rows {
        let (worker_job_id, basis_sufficiency) = row?;
        if worker_job_id == exclude_job_id {
            continue;
        }
        let worker_job_parameters = load_live_validation_job_parameters(conn, &worker_job_id)?;
        if worker_job_parameters
            .get("source_id")
            .and_then(Value::as_str)
            != Some(source_id)
        {
            continue;
        }
        return Ok(parse_policy_basis_sufficiency(&basis_sufficiency));
    }

    Ok(None)
}

fn deterministic_sample_roll_bps(worker_job_id: &str) -> u16 {
    let digest = Sha256::digest(worker_job_id.as_bytes());
    let sample = u16::from_be_bytes([digest[0], digest[1]]);
    sample % 10_000
}

fn build_live_validation_policy_config(
    skill: &LoadedSkill,
    worker_job_parameters: &Value,
) -> ValidationPolicyConfig {
    let worker_behavior_version = skill.metadata.worker_behavior_version();
    ValidationPolicyConfig {
        worker_skill_id: skill.metadata.skill_id.clone(),
        source_id: live_validation_source_id(skill, worker_job_parameters),
        validation_policy: skill.metadata.validation_policy_mode(),
        validation_sample_rate: skill.metadata.validation_sample_rate_bps(),
        validation_triggers: skill.metadata.validation_triggers(),
        worker_behavior_version: worker_behavior_version.clone(),
        validator_behavior_version: skill.metadata.validator_behavior_version(),
        validator_overlay_compatible_versions: BTreeSet::from([worker_behavior_version]),
    }
}

const fn live_policy_promotion_stage(
    stage: SkillPromotionStageMetadata,
) -> PolicySkillPromotionStage {
    match stage {
        SkillPromotionStageMetadata::Draft => PolicySkillPromotionStage::Draft,
        SkillPromotionStageMetadata::Candidate => PolicySkillPromotionStage::Candidate,
        SkillPromotionStageMetadata::Canary => PolicySkillPromotionStage::Canary,
        SkillPromotionStageMetadata::Shadow => PolicySkillPromotionStage::Shadow,
        SkillPromotionStageMetadata::Stable => PolicySkillPromotionStage::Stable,
        SkillPromotionStageMetadata::Rollback => PolicySkillPromotionStage::Rollback,
    }
}

fn build_live_validation_policy_context(
    conn: &Connection,
    worker_job_id: &str,
    skill: &LoadedSkill,
    stored_manifest: &agents::artifacts::StoredManifest,
    fixture: &LiveLoopFixture,
) -> Result<ValidationPolicyContext> {
    let worker_job_parameters = load_live_validation_job_parameters(conn, worker_job_id)?;
    let source_id = live_validation_source_id(skill, &worker_job_parameters);
    let recent_outputs = recent_completed_live_outputs(conn, &source_id, worker_job_id, 5)?;
    let manifest_appendix = load_run_manifest_appendix(&stored_manifest.path)?;
    let current_fill_rate_bps = manifest_appendix
        .extraction_fill_rate_bps
        .and_then(|value| u16::try_from(value).ok());
    let previous_behavior_version =
        previous_worker_behavior_version(&recent_outputs, &skill.metadata.skill_id);
    let fill_rate_drop_bps = current_fill_rate_bps
        .zip(recent_fill_rate_baseline_bps(
            &recent_outputs,
            &skill.metadata.skill_id,
        ))
        .and_then(|(current, baseline)| baseline.checked_sub(current))
        .filter(|drop| *drop > 0);

    Ok(ValidationPolicyContext {
        schema_valid: true,
        promotion_stage: live_policy_promotion_stage(skill.metadata.promotion_stage),
        source_health: load_live_source_health(conn, &source_id)?,
        page_fingerprint_unknown: manifest_appendix.daemon_observed_fingerprint_state != "matched",
        first_run_after_worker_version_change: previous_behavior_version
            .is_some_and(|previous| previous != skill.metadata.worker_behavior_version()),
        validator_error_spike_same_slice: build_validation_status_snapshot(conn)?
            .validator_execution_failure_rate_bps
            > 2_000,
        continuation_chain_length: if fixture.continuation_required { 2 } else { 1 },
        locator_fallback_depth: manifest_appendix.locator_fallback_depth,
        locator_fallback_depth_threshold: 1,
        recent_basis_sufficiency: recent_basis_sufficiency_for_source(
            conn,
            &source_id,
            worker_job_id,
        )?,
        high_value_opportunity: false,
        suspect_output_count_last_five: suspect_output_count_last_five(&recent_outputs),
        operator_requested: worker_job_parameters
            .get("operator_requested")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        fill_rate_drop_bps,
        fill_rate_drop_threshold_bps: 500,
        sample_roll_bps: Some(deterministic_sample_roll_bps(worker_job_id)),
        base_risk_multiplier_bps: 0,
        calibration_thresholds_met: skill.metadata.gate_a_family_prequalified
            && skill.metadata.gate_b_exact_slice_ready,
        operator_override_spike: false,
        challenge_bank_regressed: false,
        anomaly_conditioned_error_spike: false,
        signature_limited_ready: skill.metadata.gate_a_family_prequalified,
        full_slice_enforcement_ready: skill.metadata.gate_b_exact_slice_ready,
        bootstrap_completed_runs: u32::try_from(recent_outputs.len()).unwrap_or(u32::MAX),
    })
}

fn update_job_validation_required(
    state_db: &mut StateDb,
    job_id: &str,
    validation_required: bool,
) -> Result<()> {
    let ((), _) = state_db.with_write_transaction(|tx| {
        let updated = tx.execute(
            "UPDATE workflow_jobs SET validation_required = ?1 WHERE id = ?2",
            params![i64::from(validation_required), job_id],
        )?;
        ensure!(
            updated == 1,
            "workflow job missing for validation flag: {job_id}"
        );
        Ok(())
    })?;
    Ok(())
}

fn complete_validator_job(
    state_db: &mut StateDb,
    validator_job_id: &str,
    status: &str,
    output_payload: &Value,
    last_error: Option<&str>,
) -> Result<()> {
    let ((), _) = state_db.with_write_transaction(|tx| {
        let updated = tx.execute(
            "\
            UPDATE workflow_jobs
            SET status = ?1,
                completed_at = ?2,
                output_payload = ?3,
                last_error = ?4
            WHERE id = ?5
            ",
            params![
                status,
                Utc::now().to_rfc3339(),
                output_payload.to_string(),
                last_error,
                validator_job_id,
            ],
        )?;
        ensure!(
            updated == 1,
            "validator workflow job missing for completion: {validator_job_id}"
        );
        Ok(())
    })?;
    Ok(())
}

fn merge_review_reason(existing: Option<&str>, validation_marker: Option<&str>) -> Option<String> {
    match (existing, validation_marker) {
        (None, None) => None,
        (Some(reason), None) => Some(reason.to_owned()),
        (None, Some(marker)) => Some(marker.to_owned()),
        (Some(reason), Some(marker)) if reason.split(',').any(|item| item == marker) => {
            Some(reason.to_owned())
        }
        (Some(reason), Some(marker)) => Some(format!("{reason},{marker}")),
    }
}

fn observe_host_health(
    fixture: &LiveLoopFixture,
    execution_mode: ExecutionMode,
) -> HostHealthSnapshot {
    let monotonic_gap = Duration::from_millis(fixture.host_sleep_monotonic_gap_ms);
    let sleep_threshold = Duration::from_millis(fixture.host_sleep_threshold_ms);
    let sleep_issue = detect_host_sleep(monotonic_gap, sleep_threshold);
    let resource_issue = detect_resource_pressure(HostResources {
        cpu_pressure: fixture.host_cpu_pressure,
        memory_pressure: fixture.host_memory_pressure,
    });
    let issue = resolve_health_issue(None, None, sleep_issue, resource_issue);

    HostHealthSnapshot {
        issue: issue.map(|value| value.code().to_owned()),
        tainted_active_browser_session: matches!(issue, Some(HealthIssue::HostSleepInterrupted))
            && execution_mode == ExecutionMode::Pty,
        blocks_new_admission: matches!(issue, Some(HealthIssue::HostResourceExhausted)),
        monotonic_gap_ms: fixture.host_sleep_monotonic_gap_ms,
        sleep_threshold_ms: fixture.host_sleep_threshold_ms,
        cpu_pressure: fixture.host_cpu_pressure,
        memory_pressure: fixture.host_memory_pressure,
    }
}

fn annotate_trace_with_host_health(
    raw_trace: &str,
    host_health: &HostHealthSnapshot,
) -> Result<String> {
    let mut trace = StructuredTrace::parse(raw_trace)?;
    trace.insert_metadata(
        "host_health",
        serde_json::to_value(host_health).context("serializing host health snapshot")?,
    );
    trace.to_json_string()
}

fn append_runtime_control_records(
    raw_trace: &str,
    identity: &TraceIdentity,
    execution_mode: ExecutionMode,
    arbiter: &Mutex<ResourceArbiter>,
    runtime_ids: Option<PtyRuntimeIds>,
    cleanup_verdict: &str,
) -> Result<String> {
    let mut trace = StructuredTrace::parse(raw_trace)?;
    let mut seq = trace
        .control_event_records
        .last()
        .map_or(0, |record| record.seq);

    {
        let arbiter = arbiter
            .lock()
            .map_err(|_| anyhow!("resource arbiter mutex poisoned"))?;
        for event in arbiter.events().iter().filter(|event| {
            event.job_id == identity.job_id
                && event.session_id.as_deref() == Some(identity.session_id.as_str())
        }) {
            seq += 1;
            trace.push_control_record(StructuredControlEventRecord {
                seq,
                event: structured_control_kind_for_arbiter_event(&event.kind),
                identity: identity.clone(),
                execution_mode: execution_mode_label(execution_mode).to_owned(),
                at: None,
                details: json!({
                    "priority": admission_priority_label(event.priority),
                    "denial_reason": event.denial_reason.as_ref().map(denial_reason_label),
                    "release_cause": event.release_cause.map(release_cause_label),
                    "browser_slots_in_use": event.browser_slots_in_use,
                    "agent_slots_in_use": event.agent_slots_in_use,
                    "source_lane_key": event.source_lane_key,
                    "source_lane_in_use": event.source_lane_in_use,
                }),
            });
        }
    }

    if let Some(runtime_ids) = runtime_ids {
        seq += 1;
        trace.push_control_record(StructuredControlEventRecord {
            seq,
            event: StructuredControlEventKind::Reap,
            identity: identity.clone(),
            execution_mode: execution_mode_label(execution_mode).to_owned(),
            at: None,
            details: json!({
                "pid": runtime_ids.pid,
                "pgid": runtime_ids.pgid,
                "sid": runtime_ids.sid,
            }),
        });
    }

    seq += 1;
    trace.push_control_record(StructuredControlEventRecord {
        seq,
        event: StructuredControlEventKind::CleanupVerdict,
        identity: identity.clone(),
        execution_mode: execution_mode_label(execution_mode).to_owned(),
        at: None,
        details: json!({
            "cleanup_verdict": cleanup_verdict,
        }),
    });

    trace.to_json_string()
}

const fn structured_control_kind_for_arbiter_event(
    event: &ArbiterEventKind,
) -> StructuredControlEventKind {
    match event {
        ArbiterEventKind::Grant => StructuredControlEventKind::ResourceGranted,
        ArbiterEventKind::Deny => StructuredControlEventKind::ResourceDenied,
        ArbiterEventKind::Release => StructuredControlEventKind::ResourceReleased,
    }
}

fn executed_session_duration_ms(executed: &ExecutedSession) -> Result<u64> {
    let completed_at = parse_timestamp(&executed.completed_at)?;
    let duration_ms = completed_at
        .signed_duration_since(executed.started_at)
        .num_milliseconds()
        .max(0);
    u64::try_from(duration_ms).context("converting executed session duration to u64")
}

fn auth_challenge_rate_bps(executed: &ExecutedSession) -> Result<u32> {
    let trace = StructuredTrace::parse(&executed.structured_trace)?;
    Ok(
        if trace.protocol_events.iter().any(|record| {
            record.event == tracing::structured::StructuredProtocolEventKind::AuthChallenge
        }) {
            10_000
        } else {
            0
        },
    )
}

fn build_skill_run_telemetry(
    skill: &LoadedSkill,
    executed: &ExecutedSession,
    fixture: &LiveLoopFixture,
    live_validation: Option<&LiveValidationExecution>,
) -> Result<SkillRunTelemetry> {
    let host_health = observe_host_health(fixture, executed.execution_mode);
    let page_fingerprint_delta_bps = if fixture.fingerprint_state == "matched" {
        0
    } else {
        2_500
    };
    let extraction_fill_rate_bps = if fixture.source_changed || fixture.notice_parse_failed {
        4_000
    } else {
        u32::from(fixture.extraction_fill_rate_bps)
    };
    let validation_rejection_rate_bps =
        live_validation.and_then(|validation| validation.run_scoped_validation_rejection_rate_bps);

    let telemetry = compute_skill_telemetry(&SkillTelemetryWindow {
        successful_runs: u32::from(executed.exit_code == 0),
        total_runs: 1,
        durations_ms: vec![executed_session_duration_ms(executed)?],
        cost_per_run_pence: None,
        token_count: None,
        checkpoint_count: Some(executed.last_checkpoint_seq.unwrap_or(0)),
        retry_count: None,
        extraction_fill_rate_bps: Some(extraction_fill_rate_bps),
        page_fingerprint_delta_bps: Some(page_fingerprint_delta_bps),
        auth_challenge_rate_bps: Some(auth_challenge_rate_bps(executed)?),
        listing_match_rate_bps: None,
        validation_rejection_rate_bps,
        drift_score_bps: Some(if fixture.source_changed {
            3_500
        } else {
            page_fingerprint_delta_bps
        }),
    });

    Ok(SkillRunTelemetry {
        skill_id: skill.metadata.skill_id.clone(),
        skill_version: skill.metadata.skill_version.clone(),
        worker_behavior_version: skill.metadata.worker_behavior_version(),
        validator_behavior_version: skill.metadata.validator_behavior_version(),
        telemetry,
        thresholds: skill_telemetry_thresholds(),
        host_health,
    })
}

fn build_live_validator_payload(fixture: &LiveLoopFixture) -> Value {
    let (verdict, confidence_bps, enforcement_eligible, issues, summary) =
        if fixture.validator_rejected {
            (
                ValidatorVerdict::Rejected.as_str(),
                9_200_u16,
                true,
                vec![json!({
                    "severity": "major",
                    "category": "consistency",
                    "description": "Fixture requested semantic rejection.",
                    "evidence_refs": ["run-manifest.md#Evidence"],
                })],
                "Fixture requested a rejected semantic validation verdict.",
            )
        } else if fixture.validator_flagged {
            (
                ValidatorVerdict::Flagged.as_str(),
                7_400_u16,
                false,
                vec![json!({
                    "severity": "observation",
                    "category": "completeness",
                    "description": "Fixture requested a flagged semantic validation verdict.",
                    "evidence_refs": ["run-manifest.md#Warnings"],
                })],
                "Fixture requested a flagged semantic validation verdict.",
            )
        } else {
            (
                ValidatorVerdict::Approved.as_str(),
                9_800_u16,
                true,
                Vec::new(),
                "Semantic validation approved the worker output.",
            )
        };

    json!({
        "verdict": verdict,
        "basis_sufficiency": if fixture.validator_low_basis {
            ValidatorBasisSufficiency::Low.as_str()
        } else {
            ValidatorBasisSufficiency::High.as_str()
        },
        "enforcement_eligible": enforcement_eligible,
        "confidence_bps": confidence_bps,
        "issues": issues,
        "summary": summary,
    })
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn run_live_semantic_validation(
    runtime: &mut RuntimeBootstrap,
    runner: &JobRunner,
    worker_job_id: &str,
    worker_session_id: &str,
    process_id: &str,
    skill: &LoadedSkill,
    stored_manifest: &agents::artifacts::StoredManifest,
    fixture: &LiveLoopFixture,
    items_processed: u32,
) -> Result<LiveValidationExecution> {
    let worker_job_parameters =
        load_live_validation_job_parameters(runtime.state_db.connection(), worker_job_id)?;
    let policy_config = build_live_validation_policy_config(skill, &worker_job_parameters);
    let policy_context = build_live_validation_policy_context(
        runtime.state_db.connection(),
        worker_job_id,
        skill,
        stored_manifest,
        fixture,
    )?;
    let policy_decision = evaluate_validation_policy(&policy_config, &policy_context);

    if !policy_decision.validation_required {
        return Ok(LiveValidationExecution {
            validation_required: false,
            validator_job_id: None,
            validation_verdict_id: None,
            trust_state: jobs::runner::ValidationTrustState::Unverified,
            run_scoped_validation_rejection_rate_bps: None,
            process_validation_state: ProcessValidationState::Unverified,
            validation_marker: None,
            validation_telemetry: build_validation_status_snapshot(runtime.state_db.connection())?,
        });
    }

    update_job_validation_required(&mut runtime.state_db, worker_job_id, true)?;
    let request = ValidatorEnqueueRequest {
        validator_job_id: None,
        idempotency_key: None,
        worker_job_id: worker_job_id.to_owned(),
        worker_session_id: Some(worker_session_id.to_owned()),
        process_id: Some(process_id.to_owned()),
        scope_kind: Some("process".to_owned()),
        scope_id: Some(process_id.to_owned()),
        continuation_chain_id: None,
        run_manifest_path: stored_manifest.path.clone(),
        run_manifest_hash: stored_manifest.sha256.clone(),
        drift_delta_json: Value::Null,
        oqg_anomaly_summary: Vec::new(),
        representative_item_sample: json!({
            "process_id": process_id,
            "items_processed": items_processed,
        }),
        corpus_provenance_tag: None,
        purpose: jobs::runner::ValidatorJobPurpose::LiveValidation,
        canary_case: None,
        retroactive_backlog: None,
    };
    let enqueued = enqueue_validator_followup(runtime, runner, request.clone())?;

    if fixture.validator_fail_open {
        complete_validator_job(
            &mut runtime.state_db,
            &enqueued.job_id,
            "failed",
            &json!({"error": VALIDATION_SKIPPED_VALIDATOR_ERROR}),
            Some(VALIDATION_SKIPPED_VALIDATOR_ERROR),
        )?;
        let failure = decide_validator_failure();
        let backlog = RetroactiveBacklogMetadata {
            trust_state: failure.trust_state,
            high_value_opportunity: false,
            operator_reviewed: false,
            enforcement_ready: false,
            observed_at: Utc::now().to_rfc3339(),
        };
        let mut backlog_request = request;
        backlog_request.validator_job_id = None;
        backlog_request.idempotency_key = None;
        let _ = enqueue_retroactive_validation(runtime, runner, backlog_request, backlog)?;
        return Ok(LiveValidationExecution {
            validation_required: true,
            validator_job_id: Some(enqueued.job_id),
            validation_verdict_id: None,
            trust_state: failure.trust_state,
            run_scoped_validation_rejection_rate_bps: None,
            process_validation_state: failure.process_validation_state,
            validation_marker: failure.validation_marker,
            validation_telemetry: build_validation_status_snapshot(runtime.state_db.connection())?,
        });
    }

    let verdict_payload = build_live_validator_payload(fixture);
    let Ok(persisted) = runner.persist_validation_verdict(
        &mut runtime.state_db,
        &PersistValidationVerdictRequest {
            verdict_id: format!("verdict-{}", sha256_hex(enqueued.job_id.as_bytes())),
            worker_job_id: worker_job_id.to_owned(),
            worker_session_id: Some(worker_session_id.to_owned()),
            validator_job_id: enqueued.job_id.clone(),
            validator_session_id: None,
            run_manifest_hash: stored_manifest.sha256.clone(),
            validator_skill: enqueued.skill.clone(),
            payload: verdict_payload.clone(),
            oqg_signals_summary: Some(json!({"status":"clean"})),
            incident_signature: None,
            quality_assessment_json: None,
            skill_improvement_suggestions_json: None,
            override_verdict: None,
            override_reason: None,
            override_by: None,
            override_at: None,
            verdict_rendered_at: Some(Utc::now().to_rfc3339()),
        },
    ) else {
        complete_validator_job(
            &mut runtime.state_db,
            &enqueued.job_id,
            "failed",
            &json!({"error": VALIDATION_SKIPPED_VALIDATOR_ERROR}),
            Some(VALIDATION_SKIPPED_VALIDATOR_ERROR),
        )?;
        let failure = decide_validator_failure();
        let backlog = RetroactiveBacklogMetadata {
            trust_state: failure.trust_state,
            high_value_opportunity: false,
            operator_reviewed: false,
            enforcement_ready: false,
            observed_at: Utc::now().to_rfc3339(),
        };
        let mut backlog_request = request;
        backlog_request.validator_job_id = None;
        backlog_request.idempotency_key = None;
        let _ = enqueue_retroactive_validation(runtime, runner, backlog_request, backlog)?;
        return Ok(LiveValidationExecution {
            validation_required: true,
            validator_job_id: Some(enqueued.job_id),
            validation_verdict_id: None,
            trust_state: failure.trust_state,
            run_scoped_validation_rejection_rate_bps: None,
            process_validation_state: failure.process_validation_state,
            validation_marker: failure.validation_marker,
            validation_telemetry: build_validation_status_snapshot(runtime.state_db.connection())?,
        });
    };

    complete_validator_job(
        &mut runtime.state_db,
        &enqueued.job_id,
        "completed",
        &verdict_payload,
        None,
    )?;
    let decision = decide_validator_verdict(
        &persisted.contract,
        ValidationHandlingMode {
            dry_run: true,
            enforcement_active: false,
        },
    );
    let run_scoped_validation_rejection_rate_bps =
        if matches!(persisted.contract.verdict, ValidatorVerdict::Rejected) {
            Some(10_000)
        } else {
            Some(0)
        };
    Ok(LiveValidationExecution {
        validation_required: true,
        validator_job_id: Some(enqueued.job_id),
        validation_verdict_id: Some(persisted.verdict_id),
        trust_state: decision.trust_state,
        run_scoped_validation_rejection_rate_bps,
        process_validation_state: decision.process_validation_state,
        validation_marker: decision.validation_marker,
        validation_telemetry: build_validation_status_snapshot(runtime.state_db.connection())?,
    })
}

fn claimed_job_lease_heartbeat(
    runtime: &RuntimeBootstrap,
    job_id: &str,
) -> Result<Option<JobLeaseHeartbeat>> {
    let lease_owner = runtime
        .state_db
        .connection()
        .query_row(
            "\
            SELECT lease_owner
            FROM workflow_jobs
            WHERE id = ?1
              AND status = 'running'
              AND lease_owner IS NOT NULL
              AND completed_at IS NULL
            ",
            params![job_id],
            |row| row.get::<_, String>(0),
        )
        .optional()?;
    let Some(lease_owner) = lease_owner else {
        return Ok(None);
    };

    let state_db_path = Path::new(&runtime.descriptor.state_root).join("state.db");
    Ok(Some(start_job_lease_heartbeat(
        state_db_path,
        job_id.to_owned(),
        lease_owner,
    )))
}

fn claimed_job_resource_lease<'a>(
    runtime: &'a RuntimeBootstrap,
    skill: &LoadedSkill,
    job_id: &str,
    session_id: &str,
    mode: ExecutionMode,
    job_parameters: &Value,
) -> Result<Option<ResourceLeaseGuard<'a>>> {
    let source_lane = source_lane_request(skill, job_parameters);
    let request = AdmissionRequest {
        job_id: job_id.to_owned(),
        session_id: Some(session_id.to_owned()),
        priority: admission_priority(skill),
        browser_session: matches!(mode, ExecutionMode::Pty),
        adapter_session: true,
        source_lane,
        validator_sub_job: false,
        concurrency_key: Some(skill.metadata.skill_id.clone()),
        max_concurrency: None,
    };

    let lease = {
        let mut arbiter = runtime
            .resource_arbiter
            .lock()
            .map_err(|_| anyhow!("resource arbiter mutex poisoned"))?;
        match arbiter.admit(&request) {
            ArbiterDecision::Granted(lease) => lease,
            ArbiterDecision::Denied(reason) => {
                bail!("resource arbiter denied admission for job {job_id}: {reason:?}");
            }
        }
    };

    Ok(Some(ResourceLeaseGuard::new(
        &runtime.resource_arbiter,
        lease,
        ReleaseCause::PreSpawnFailure,
    )))
}

fn admission_priority(skill: &LoadedSkill) -> AdmissionPriority {
    let skill_id = skill.metadata.skill_id.as_str();
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

fn source_lane_request(skill: &LoadedSkill, job_parameters: &Value) -> Option<SourceLaneRequest> {
    let source_id = job_parameters
        .get("source_id")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .or_else(|| {
            let skill_id = skill.metadata.skill_id.as_str();
            ["fts", "contracts-finder", "alibaba", "1688"]
                .into_iter()
                .find(|candidate| skill_id.contains(candidate))
                .map(str::to_owned)
        })?;

    let source_kind = match source_id.as_str() {
        "alibaba" | "1688" => SourceKind::Supplier,
        _ => SourceKind::Procurement,
    };

    Some(SourceLaneRequest {
        source_id,
        source_kind,
    })
}

fn process_id_from_job_parameters(job_parameters: &Value) -> Option<String> {
    job_parameters
        .get("process_id")
        .and_then(Value::as_str)
        .or_else(|| {
            job_parameters
                .get("job_parameters")
                .and_then(|value| value.get("process_id"))
                .and_then(Value::as_str)
        })
        .or_else(|| {
            job_parameters
                .get("request")
                .and_then(|value| value.get("process_id"))
                .and_then(Value::as_str)
        })
        .map(str::to_owned)
}

fn trace_identity_for(job_id: &str, session_id: &str, job_parameters: &Value) -> TraceIdentity {
    let mut identity = TraceIdentity::new(job_id, session_id);
    if let Some(process_id) = process_id_from_job_parameters(job_parameters) {
        identity = identity.with_process_id(process_id);
    }
    identity
}

fn forensic_identity_for(
    job_id: &str,
    session_id: &str,
    job_parameters: &Value,
) -> ForensicIdentity {
    let mut identity = ForensicIdentity::new(session_id).with_job_id(job_id);
    if let Some(process_id) = process_id_from_job_parameters(job_parameters) {
        identity = identity.with_process_id(process_id);
    }
    identity
}

fn build_structured_trace(
    identity: &TraceIdentity,
    execution_mode: ExecutionMode,
    phase_history: impl IntoIterator<Item = impl Into<String>>,
    caps: &CapabilityReport,
    events: &[ControlEvent],
    forensic_summary: &ForensicCaptureSummary,
) -> Result<String> {
    let mut trace =
        StructuredTrace::new(execution_mode, phase_history).with_capability_report(caps);
    trace.extend_from_adapter_events(identity, events);
    trace.insert_metadata(
        "forensic_artifact_refs",
        serde_json::to_value(&forensic_summary.artifact_refs)
            .context("serializing forensic artifact refs")?,
    );
    trace.insert_metadata(
        "forensic_root_path",
        Value::String(forensic_summary.root_path.to_string_lossy().into_owned()),
    );
    trace.insert_metadata(
        "forensic_terminal_output_path",
        Value::String(
            forensic_summary
                .terminal_output_path
                .to_string_lossy()
                .into_owned(),
        ),
    );
    trace.insert_metadata(
        "forensic_terminal_output_hash",
        Value::String(forensic_summary.terminal_output_hash.clone()),
    );
    trace.insert_metadata(
        "forensic_retention",
        serde_json::to_value(forensic_summary.retention)
            .context("serializing forensic retention")?,
    );
    trace.to_json_string()
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
async fn execute_pty_skill_session(
    artifact_root: &Path,
    identity: TraceIdentity,
    forensic_identity: ForensicIdentity,
    session_id: &str,
    caps: &CapabilityReport,
    adapter: &dyn AgentAdapter,
    launch_spec: &LaunchSpec,
    started_at: DateTime<Utc>,
    config_hash: String,
) -> Result<ExecutedSession> {
    let mut parser = adapter.parser(ExecutionMode::Pty);
    let mut session = PtySession::spawn(
        pty_command_from_launch_spec(launch_spec),
        SpawnOptions {
            spawn_path: SpawnPath::resolve(true),
            ..SpawnOptions::default()
        },
    )?;
    crate::ipc::handlers::sessions::register_live_pty_session(session_id, session.live_control());
    let _live_pty_registration = LivePtyRegistration { session_id };
    let mut capture = ForensicPtyCapture::new(
        artifact_root,
        forensic_identity,
        started_at,
        CaptureConfig::default(),
    )?;
    let mut events = vec![ControlEvent::Started {
        pid: session.runtime_ids.pid,
        pgid: session.runtime_ids.pgid,
        sid: session.runtime_ids.sid,
        mode: ExecutionMode::Pty,
    }];
    let mut seq = 1_u64;
    let mut last_checkpoint_seq = None;
    let mut tail = String::new();

    loop {
        let line = session.read_line()?;
        if line.is_empty() {
            break;
        }
        tail = line.trim_end().to_owned();
        capture.append(seq, line.as_bytes(), Utc::now())?;
        seq += 1;
        let derived = parser
            .on_output(&OutputFrame::Pty(Bytes::from(line.clone().into_bytes())))
            .await?;
        for event in &derived {
            if let ControlEvent::Checkpoint { seq, .. } = event {
                last_checkpoint_seq = Some(*seq);
            }
        }
        events.extend(derived);
    }
    let status = session.finish()?;
    let exit_code = i32::try_from(status.exit_code()).context("converting PTY exit code to i32")?;
    events.push(ControlEvent::Exited { status });
    events.extend(parser.finish().await?);
    let phase_history = session
        .phase_history
        .iter()
        .map(|phase| phase.as_str())
        .collect::<Vec<_>>();
    let forensic_summary = capture.finalize(tail.clone())?;
    let terminal_output_bytes =
        forensic_summary
            .artifact_refs
            .iter()
            .try_fold(0_i64, |acc, artifact_ref| {
                acc.checked_add(artifact_ref.size_bytes)
                    .ok_or_else(|| anyhow!("forensic artifact sizes exceeded i64 range"))
            })?;
    let structured_trace = build_structured_trace(
        &identity,
        ExecutionMode::Pty,
        phase_history,
        caps,
        &events,
        &forensic_summary,
    )?;

    // In PTY mode, look for result.json written by the agent in the workdir.
    let result_path = events
        .iter()
        .find_map(|event| match event {
            ControlEvent::FinalResultReady { path } => Some(path.clone()),
            _ => None,
        })
        .or_else(|| {
            let candidate = launch_spec.cwd.join("result.json");
            if candidate.exists() {
                Some(candidate)
            } else {
                None
            }
        });

    Ok(ExecutedSession {
        session_id: session_id.to_owned(),
        started_at,
        execution_mode: ExecutionMode::Pty,
        adapter_id: adapter.id().to_owned(),
        cli_vendor: adapter.id().to_owned(),
        config_hash,
        pty_session_id: Some(session_id.to_owned()),
        runtime_ids: Some(session.runtime_ids),
        completed_at: Utc::now().to_rfc3339(),
        terminal_output_path: Some(
            forensic_summary
                .terminal_output_path
                .to_string_lossy()
                .into_owned(),
        ),
        terminal_output_hash: Some(forensic_summary.terminal_output_hash.clone()),
        terminal_output_bytes: Some(terminal_output_bytes),
        terminal_output_tail: Some(forensic_summary.tail.clone()),
        structured_trace,
        exit_code,
        cleanup_verdict: "clean".to_owned(),
        last_checkpoint_seq,
        result_path,
        artifact_refs: forensic_summary
            .artifact_refs
            .iter()
            .map(|artifact_ref| artifact_ref.path.clone())
            .collect(),
    })
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
async fn execute_contract_skill_session(
    artifact_root: &Path,
    identity: TraceIdentity,
    forensic_identity: ForensicIdentity,
    session_id: &str,
    caps: &CapabilityReport,
    adapter: &dyn AgentAdapter,
    launch_spec: &LaunchSpec,
    started_at: DateTime<Utc>,
    config_hash: String,
) -> Result<ExecutedSession> {
    let mut command = std::process::Command::new(&launch_spec.program);
    command
        .args(&launch_spec.args)
        .envs(launch_spec.env.iter())
        .current_dir(&launch_spec.cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let child = command.spawn().with_context(|| {
        format!(
            "spawning contract session `{}`",
            launch_spec.program.display()
        )
    })?;
    let runtime_ids = child_runtime_ids(child.id())?;
    let output = child
        .wait_with_output()
        .context("waiting for contract child output")?;
    let mut parser = adapter.parser(ExecutionMode::Contract);
    let mut events = vec![ControlEvent::Started {
        pid: runtime_ids.pid,
        pgid: runtime_ids.pgid,
        sid: runtime_ids.sid,
        mode: ExecutionMode::Contract,
    }];
    events.extend(
        parser
            .on_output(&OutputFrame::Stdout(Bytes::from(output.stdout.clone())))
            .await?,
    );
    events.extend(
        parser
            .on_output(&OutputFrame::Stderr(Bytes::from(output.stderr.clone())))
            .await?,
    );
    events.push(ControlEvent::Exited {
        status: portable_pty::ExitStatus::from(output.status),
    });
    events.extend(parser.finish().await?);
    // Prefer workdir result.json (written by the skill) over the stream
    // `result` event (which may contain Claude's narrative text rather than
    // the structured result contract).
    let workdir_result = launch_spec.cwd.join("result.json");
    let result_path = if workdir_result.exists() {
        Some(workdir_result)
    } else {
        events.iter().find_map(|event| match event {
            ControlEvent::FinalResultReady { path } => Some(path.clone()),
            _ => None,
        })
    };
    let last_checkpoint_seq = events.iter().rev().find_map(|event| match event {
        ControlEvent::Checkpoint { seq, .. } => Some(*seq),
        _ => None,
    });
    let forensic_summary = summarize_contract_capture(capture_contract_output(
        artifact_root,
        forensic_identity,
        started_at,
        &output.stdout,
        &output.stderr,
    )?);
    let exit_code = output.status.code().unwrap_or(1);
    let terminal_output_bytes =
        forensic_summary
            .artifact_refs
            .iter()
            .try_fold(0_i64, |acc, artifact_ref| {
                acc.checked_add(artifact_ref.size_bytes)
                    .ok_or_else(|| anyhow!("forensic artifact sizes exceeded i64 range"))
            })?;
    let structured_trace = build_structured_trace(
        &identity,
        ExecutionMode::Contract,
        [
            "created",
            "spawned",
            "active",
            "terminating",
            "exited",
            "reaped",
            "archived",
        ],
        caps,
        &events,
        &forensic_summary,
    )?;

    Ok(ExecutedSession {
        session_id: session_id.to_owned(),
        started_at,
        execution_mode: ExecutionMode::Contract,
        adapter_id: adapter.id().to_owned(),
        cli_vendor: adapter.id().to_owned(),
        config_hash,
        pty_session_id: None,
        runtime_ids: Some(runtime_ids),
        completed_at: Utc::now().to_rfc3339(),
        terminal_output_path: Some(
            forensic_summary
                .terminal_output_path
                .to_string_lossy()
                .into_owned(),
        ),
        terminal_output_hash: Some(forensic_summary.terminal_output_hash.clone()),
        terminal_output_bytes: Some(terminal_output_bytes),
        terminal_output_tail: Some(forensic_summary.tail.clone()),
        structured_trace,
        exit_code,
        cleanup_verdict: "clean".to_owned(),
        last_checkpoint_seq,
        result_path,
        artifact_refs: forensic_summary
            .artifact_refs
            .iter()
            .map(|artifact_ref| artifact_ref.path.clone())
            .collect(),
    })
}

fn load_skill_from_path(path: &Path) -> Result<LoadedSkill> {
    let skill_md = if path.file_name().is_some_and(|name| name == "SKILL.md") {
        path.to_path_buf()
    } else {
        path.join("SKILL.md")
    };
    let skill_root = skill_md
        .parent()
        .ok_or_else(|| anyhow!("skill path {} has no parent directory", skill_md.display()))?;
    let repo = repo_root()?;
    let relative = skill_md
        .strip_prefix(&repo)
        .unwrap_or(&skill_md)
        .to_string_lossy()
        .into_owned();
    load_skill(skill_root, &relative)
}

fn create_manual_skill_job(
    state_db: &mut StateDb,
    skill: &LoadedSkill,
    job_parameters: &Value,
    adapter_kind: AgentCliKind,
) -> Result<String> {
    let job_id = format!(
        "job-{}",
        sha256_hex(&UuidSeed::from_time(Utc::now()).as_bytes())
    );
    let ((), _) = state_db.with_write_transaction(|tx| {
        tx.execute(
            "\
            INSERT INTO workflow_jobs (
              id, kind, scope_kind, scope_id, status, input_payload, validation_required,
              parent_job_id, validation_verdict_id
            ) VALUES (?1, 'run_skill', 'skill', ?2, 'pending', ?3, 0, NULL, NULL)
            ",
            params![
                job_id,
                skill.metadata.skill_id,
                json!({
                    "skill_binding": skill_binding_payload(skill),
                    "agent_cli": adapter_kind.as_str(),
                    "job_parameters": job_parameters,
                })
                .to_string(),
            ],
        )?;
        Ok(())
    })?;
    Ok(job_id)
}

fn insert_executed_session_log(
    conn: &Connection,
    skill: &LoadedSkill,
    job_id: &str,
    executed: &ExecutedSession,
) -> Result<()> {
    insert_session_log(
        conn,
        &SessionDraft {
            session_id: executed.session_id.clone(),
            job_id: job_id.to_owned(),
            skill_path: skill.skill_path.to_string_lossy().into_owned(),
            skill_version: skill.metadata.skill_version.clone(),
            execution_mode: execution_mode_label(executed.execution_mode).to_owned(),
            status: "completed".to_owned(),
            cli_vendor: Some(executed.cli_vendor.clone()),
            cli_version: None,
            adapter_id: Some(executed.adapter_id.clone()),
            adapter_version: Some(env!("CARGO_PKG_VERSION").to_owned()),
            config_hash: Some(executed.config_hash.clone()),
            pty_session_id: executed.pty_session_id.clone(),
            sid: executed.runtime_ids.map(|ids| ids.sid),
            pid: executed.runtime_ids.map(|ids| ids.pid),
            pgid: executed.runtime_ids.map(|ids| ids.pgid),
            started_at: executed.started_at.to_rfc3339(),
            completed_at: executed.completed_at.clone(),
            terminal_output_path: executed.terminal_output_path.clone(),
            terminal_output_hash: executed.terminal_output_hash.clone(),
            terminal_output_bytes: executed.terminal_output_bytes,
            terminal_output_tail: executed.terminal_output_tail.clone(),
            structured_trace: Some(executed.structured_trace.clone()),
            error_type: None,
            error_message: None,
            exit_code: Some(executed.exit_code),
            cleanup_verdict: Some(executed.cleanup_verdict.clone()),
            last_checkpoint_seq: executed.last_checkpoint_seq,
        },
    )
}

fn launch_args_for_skill(
    adapter_config: &AdapterRuntimeConfig,
    mode: ExecutionMode,
    prompt: &str,
) -> Vec<OsString> {
    match (adapter_config.kind, mode) {
        (AgentCliKind::Claude, ExecutionMode::Pty | ExecutionMode::Contract) => vec![
            OsString::from("-p"),
            OsString::from(prompt),
            OsString::from("--dangerously-skip-permissions"),
        ],
        (AgentCliKind::Codex, ExecutionMode::Pty | ExecutionMode::Contract) => {
            vec![OsString::from(prompt)]
        }
    }
}

fn pty_command_from_launch_spec(spec: &LaunchSpec) -> PtyCommand {
    let mut command = PtyCommand::new(spec.program.clone().into_os_string()).cwd(spec.cwd.clone());
    for (key, value) in &spec.env {
        command = command.env(key.clone(), value.clone());
    }
    for arg in &spec.args {
        command = command.arg(arg.clone());
    }
    command
}

fn child_runtime_ids(pid: u32) -> Result<PtyRuntimeIds> {
    let pid_handle = Pid::from_raw(i32::try_from(pid).context("converting child pid to i32")?);
    let session_id = getsid(Some(pid_handle)).context("reading child session id")?;
    let process_group_id = getpgid(Some(pid_handle)).context("reading child process group")?;
    Ok(PtyRuntimeIds {
        sid: u32::try_from(session_id.as_raw()).context("converting child sid to u32")?,
        pid,
        pgid: u32::try_from(process_group_id.as_raw()).context("converting child pgid to u32")?,
    })
}

fn launch_spec_hash(spec: &LaunchSpec) -> String {
    let env_pairs: Vec<String> = spec
        .env
        .iter()
        .map(|(key, value)| format!("{}={}", key.to_string_lossy(), value.to_string_lossy()))
        .collect();
    sha256_hex(
        json!({
            "program": spec.program,
            "args": spec.args.iter().map(|arg| arg.to_string_lossy().into_owned()).collect::<Vec<_>>(),
            "env": env_pairs,
            "cwd": spec.cwd,
        })
        .to_string()
        .as_bytes(),
    )
}

const fn execution_mode_label(mode: ExecutionMode) -> &'static str {
    match mode {
        ExecutionMode::Pty => "pty",
        ExecutionMode::Contract => "contract",
    }
}

#[allow(clippy::too_many_lines, clippy::future_not_send)]
async fn run_contracts_finder_live_loop(
    runtime: &mut RuntimeBootstrap,
    cli: &Cli,
) -> Result<LiveLoopReport> {
    let repo_root = repo_root()?;
    let ingest_skill = load_skill(
        &repo_root.join("skills/procurement-ingestion/contracts-finder"),
        "skills/procurement-ingestion/contracts-finder/SKILL.md",
    )?;

    // Read existing cursor from source_cursors table.
    let existing_cursor: Option<String> = runtime
        .state_db
        .connection()
        .query_row(
            "SELECT cursor_value FROM source_cursors \
             WHERE source_id = 'contracts-finder' AND cursor_key = 'notice_cursor'",
            [],
            |row| row.get(0),
        )
        .optional()?;

    // Extract publishedFrom from cursor JSON if present.
    let published_from: Option<String> = existing_cursor.as_deref().and_then(|raw| {
        serde_json::from_str::<Value>(raw).ok().and_then(|v| {
            v.get("cursor_after")
                .and_then(Value::as_str)
                .map(String::from)
        })
    });

    let job_parameters = json!({
        "source_id": "contracts-finder",
        "publishedFrom": published_from,
        "limit": 100,
        "stages": "tender,award",
    });

    // Create sync_source job.
    let adapter_config = adapter_runtime_config(cli, &ingest_skill)?;
    let job_id = format!(
        "job-{}",
        sha256_hex(&UuidSeed::from_time(Utc::now()).as_bytes())
    );
    runtime.state_db.with_write_transaction(|tx| {
        tx.execute(
            "\
            INSERT INTO workflow_jobs (
              id, kind, scope_kind, scope_id, status, input_payload, validation_required,
              parent_job_id, validation_verdict_id
            ) VALUES (?1, 'sync_source', 'source', 'contracts-finder', 'pending', ?2, 0, NULL, NULL)
            ",
            params![
                job_id,
                json!({
                    "source_id": "contracts-finder",
                    "trigger": "manual_cli",
                    "skill_binding": skill_binding_payload(&ingest_skill),
                    "agent_cli": adapter_config.kind.as_str(),
                    "job_parameters": job_parameters,
                    "durable_step_surfaces": live_loop_surfaces_json(),
                })
                .to_string(),
            ],
        )?;
        Ok(())
    })?;

    // Claim job.
    let runner = JobRunner::new(LOCAL_EXECUTOR_ID);
    let claimed = runner
        .claim_next_job(runtime.state_db.connection())?
        .ok_or_else(|| anyhow!("expected claimed workflow job for contracts-finder"))?;
    ensure!(claimed.id == job_id, "claimed unexpected workflow job");

    // Execute PTY skill session.
    let session_id = format!("session-pty-{}", sha256_hex(job_id.as_bytes()));
    let executed = execute_skill_session(
        runtime,
        &ingest_skill,
        &job_id,
        &session_id,
        &job_parameters,
        cli,
    )
    .await?;

    ensure!(
        executed.execution_mode == ExecutionMode::Pty,
        "contracts-finder ingestion skill must resolve to PTY"
    );

    insert_executed_session_log(
        runtime.state_db.connection(),
        &ingest_skill,
        &job_id,
        &executed,
    )?;

    // Read and parse the agent's result contract.
    let result_path = match executed.result_path.as_ref() {
        Some(path) => path.clone(),
        None => {
            return fail_job(
                runtime,
                &job_id,
                "contracts-finder skill did not produce a result contract",
                json!({"error_code": "AGENT_RUN_FAILED"}),
            );
        }
    };
    let result_text = fs::read_to_string(&result_path)
        .with_context(|| format!("reading result contract from {}", result_path.display()))?;
    let result_json: Value =
        serde_json::from_str(&result_text).context("parsing agent result contract JSON")?;

    // Persist ingestion data via the generic persistence function.
    let persisted_count = persist_ingestion_from_agent_output(
        &mut runtime.state_db,
        "contracts-finder",
        &result_json,
    )?;

    // Check continuation state from agent result.
    let continuation_required = result_json
        .get("continuation_required")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let result_cursor = result_json
        .get("cursor")
        .and_then(Value::as_str)
        .unwrap_or("");

    // Enqueue a continuation job if the agent signals more pages.
    if continuation_required && !result_cursor.is_empty() {
        let continuation_job_id = format!(
            "job-{}",
            sha256_hex(
                &UuidSeed::from_time(Utc::now() + chrono::Duration::milliseconds(1)).as_bytes()
            )
        );
        runtime.state_db.with_write_transaction(|tx| {
            tx.execute(
                "\
                INSERT INTO workflow_jobs (
                  id, kind, scope_kind, scope_id, status, input_payload, validation_required,
                  parent_job_id, validation_verdict_id
                ) VALUES (?1, 'sync_source', 'source', 'contracts-finder', 'pending', ?2, 0, ?3, NULL)
                ",
                params![
                    continuation_job_id,
                    json!({
                        "source_id": "contracts-finder",
                        "trigger": "continuation",
                        "skill_binding": skill_binding_payload(&ingest_skill),
                        "agent_cli": adapter_config.kind.as_str(),
                        "job_parameters": {
                            "source_id": "contracts-finder",
                            "publishedFrom": result_cursor,
                            "limit": 100,
                            "stages": "tender,award",
                        },
                        "continuation_chain_id": job_id,
                        "durable_step_surfaces": live_loop_surfaces_json(),
                    })
                    .to_string(),
                    job_id,
                ],
            )?;
            Ok(())
        })?;
        ::tracing::info!(
            continuation_job_id = %continuation_job_id,
            cursor = %result_cursor,
            "enqueued contracts-finder continuation job"
        );
    }

    // Finalize the job.
    let summary_payload = json!({
        "job_id": job_id,
        "source_id": "contracts-finder",
        "notices_ingested": persisted_count,
        "continuation_required": continuation_required,
        "continuation_cursor": result_cursor,
        "session_id": session_id,
        "durable_step_surfaces": live_loop_surfaces_json(),
    });

    runtime.state_db.with_write_transaction(|tx| {
        tx.execute(
            "\
            UPDATE workflow_jobs
            SET status = 'completed',
                completed_at = ?1,
                output_payload = ?2
            WHERE id = ?3
            ",
            params![Utc::now().to_rfc3339(), summary_payload.to_string(), job_id],
        )?;
        Ok(())
    })?;

    Ok(LiveLoopReport {
        summary_line: format!(
            "contracts-finder live loop completed: job_id={job_id} notices={persisted_count} continuation={continuation_required}",
        ),
        status: "completed".to_owned(),
    })
}

#[allow(clippy::future_not_send)]
async fn run_procontract_discover_live_loop(
    runtime: &mut RuntimeBootstrap,
    cli: &Cli,
) -> Result<LiveLoopReport> {
    let repo_root = repo_root()?;
    let ingest_skill = load_skill(
        &repo_root.join("skills/procurement-ingestion/procontract-discover"),
        "skills/procurement-ingestion/procontract-discover/SKILL.md",
    )?;
    let existing_cursor: Option<String> = runtime
        .state_db
        .connection()
        .query_row(
            "SELECT cursor_value FROM source_cursors \
             WHERE source_id = 'procontract' AND cursor_key = 'notice_cursor'",
            [],
            |row| row.get(0),
        )
        .optional()?;
    let published_from = existing_cursor.as_deref().and_then(|raw| {
        serde_json::from_str::<Value>(raw).ok().and_then(|cursor| {
            cursor
                .get("last_published")
                .and_then(Value::as_str)
                .map(str::to_owned)
        })
    });
    let job_parameters = json!({
        "source_id": "procontract",
        "cursor": existing_cursor,
        "publishedFrom": published_from,
        "limit": 100,
        "startPage": 1,
    });
    let adapter_config = adapter_runtime_config(cli, &ingest_skill)?;
    let job_id = create_source_sync_job_from_parameters(
        &mut runtime.state_db,
        "procontract",
        &ingest_skill,
        &job_parameters,
        adapter_config.kind,
    )?;
    let runner = JobRunner::new(LOCAL_EXECUTOR_ID);
    let claimed = runner
        .claim_next_job(runtime.state_db.connection())?
        .ok_or_else(|| anyhow!("expected claimed workflow job for procontract"))?;
    ensure!(claimed.id == job_id, "claimed unexpected workflow job");

    let session_id = format!("session-pty-{}", sha256_hex(job_id.as_bytes()));
    let executed = execute_skill_session(
        runtime,
        &ingest_skill,
        &job_id,
        &session_id,
        &job_parameters,
        cli,
    )
    .await?;
    ensure!(
        executed.execution_mode == ExecutionMode::Pty,
        "procontract discovery skill must resolve to PTY"
    );
    insert_executed_session_log(
        runtime.state_db.connection(),
        &ingest_skill,
        &job_id,
        &executed,
    )?;

    let result_path = executed
        .result_path
        .as_ref()
        .ok_or_else(|| anyhow!("procontract discovery skill did not produce a result contract"))?;
    let result_json = read_result_contract(result_path)?;
    let persisted_count =
        persist_ingestion_from_agent_output(&mut runtime.state_db, "procontract", &result_json)?;
    let continuation_required = result_json
        .get("continuation_required")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    complete_source_job(
        runtime,
        &job_id,
        &json!({
            "job_id": job_id,
            "source_id": "procontract",
            "notices_ingested": persisted_count,
            "continuation_required": continuation_required,
            "session_id": session_id,
            "durable_step_surfaces": live_loop_surfaces_json(),
        }),
        format!(
            "procontract discovery completed: job_id={job_id} notices={persisted_count} continuation={continuation_required}"
        ),
    )
}

#[allow(clippy::future_not_send)]
#[allow(clippy::too_many_lines)]
async fn run_procontract_register_interest_live_loop(
    runtime: &mut RuntimeBootstrap,
    cli: &Cli,
) -> Result<LiveLoopReport> {
    let repo_root = repo_root()?;
    let skill = load_skill(
        &repo_root.join("skills/procurement-ingestion/procontract-register-interest"),
        "skills/procurement-ingestion/procontract-register-interest/SKILL.md",
    )?;
    let evaluations = build_procontract_interest_evaluations(load_procontract_interest_candidates(
        runtime.state_db.connection(),
    )?);
    let registration_targets = evaluations
        .iter()
        .filter(|evaluation| evaluation.action == "register")
        .map(registration_job_entry)
        .collect::<Vec<_>>();
    let evaluation_report = evaluations
        .iter()
        .map(evaluation_report_entry)
        .collect::<Vec<_>>();
    let adapter_config = adapter_runtime_config(cli, &skill)?;
    let job_parameters = json!({
        "source_id": "procontract-interest",
        "contracts_to_register": registration_targets,
        "pre_filter_evaluated": evaluation_report,
    });
    let job_id = create_source_sync_job_from_parameters(
        &mut runtime.state_db,
        "procontract-interest",
        &skill,
        &job_parameters,
        adapter_config.kind,
    )?;
    let runner = JobRunner::new(LOCAL_EXECUTOR_ID);
    let claimed = runner
        .claim_next_job(runtime.state_db.connection())?
        .ok_or_else(|| anyhow!("expected claimed workflow job for procontract-interest"))?;
    ensure!(claimed.id == job_id, "claimed unexpected workflow job");

    if evaluations.is_empty() {
        return complete_source_job(
            runtime,
            &job_id,
            &json!({
                "job_id": job_id,
                "source_id": "procontract-interest",
                "registrations": 0,
                "summary": {
                    "total_evaluated": 0,
                    "registered": 0,
                    "already_registered": 0,
                    "skipped_service": 0,
                    "skipped_expiry": 0,
                    "failed": 0,
                },
                "durable_step_surfaces": live_loop_surfaces_json(),
            }),
            format!("procontract interest registration completed: job_id={job_id} evaluated=0"),
        );
    }

    if !evaluations
        .iter()
        .any(|evaluation| evaluation.action == "register")
    {
        let synthetic_result = json!({
            "cursor": json!({"last_processed_at": Utc::now().to_rfc3339()}).to_string(),
            "continuation_required": false,
            "registrations": evaluations.iter().map(|evaluation| {
                json!({
                    "notice_id": evaluation.notice_id,
                    "process_id": evaluation.process_id,
                    "source_url": evaluation.source_url,
                    "action": "skipped",
                    "skip_reason": evaluation.skip_reason,
                    "pre_filter_result": evaluation.pre_filter_result,
                })
            }).collect::<Vec<_>>(),
            "summary": {
                "total_evaluated": evaluations.len(),
                "registered": 0,
                "already_registered": 0,
                "skipped_service": evaluations.iter().filter(|evaluation| evaluation.skip_reason == Some("pure_service")).count(),
                "skipped_expiry": evaluations.iter().filter(|evaluation| evaluation.skip_reason == Some("expiry_too_close")).count(),
                "failed": 0,
            },
        });
        let persisted_count = persist_interest_registrations(
            &mut runtime.state_db,
            "procontract-interest",
            &synthetic_result,
        )?;
        return complete_source_job(
            runtime,
            &job_id,
            &json!({
                "job_id": job_id,
                "source_id": "procontract-interest",
                "registrations": persisted_count,
                "summary": synthetic_result.get("summary").cloned().unwrap_or_else(|| json!({})),
                "durable_step_surfaces": live_loop_surfaces_json(),
            }),
            format!(
                "procontract interest registration completed without portal actions: job_id={job_id} evaluated={} skipped_only=true",
                evaluations.len()
            ),
        );
    }

    let session_id = format!("session-pty-{}", sha256_hex(job_id.as_bytes()));
    let executed =
        execute_skill_session(runtime, &skill, &job_id, &session_id, &job_parameters, cli).await?;
    ensure!(
        executed.execution_mode == ExecutionMode::Pty,
        "procontract interest-registration skill must resolve to PTY"
    );
    insert_executed_session_log(runtime.state_db.connection(), &skill, &job_id, &executed)?;
    let result_path = executed.result_path.as_ref().ok_or_else(|| {
        anyhow!("procontract interest-registration skill did not produce a result contract")
    })?;
    let result_json = read_result_contract(result_path)?;
    let persisted_count = persist_interest_registrations(
        &mut runtime.state_db,
        "procontract-interest",
        &result_json,
    )?;

    let summary = result_json
        .get("summary")
        .cloned()
        .unwrap_or_else(|| json!({}));
    complete_source_job(
        runtime,
        &job_id,
        &json!({
            "job_id": job_id,
            "source_id": "procontract-interest",
            "registrations": persisted_count,
            "summary": summary,
            "session_id": session_id,
            "durable_step_surfaces": live_loop_surfaces_json(),
        }),
        format!(
            "procontract interest registration completed: job_id={job_id} evaluations={} persisted={persisted_count}",
            evaluations.len()
        ),
    )
}

#[allow(clippy::future_not_send)]
async fn run_procontract_enrich_tender_live_loop(
    runtime: &mut RuntimeBootstrap,
    cli: &Cli,
) -> Result<LiveLoopReport> {
    let repo_root = repo_root()?;
    let skill = load_skill(
        &repo_root.join("skills/procurement-ingestion/procontract-enrich-tender"),
        "skills/procurement-ingestion/procontract-enrich-tender/SKILL.md",
    )?;
    let existing_cursor: Option<String> = runtime
        .state_db
        .connection()
        .query_row(
            "SELECT cursor_value FROM source_cursors \
             WHERE source_id = 'procontract-enrich' AND cursor_key = 'enrich_cursor'",
            [],
            |row| row.get(0),
        )
        .optional()?;
    let download_root = fs::canonicalize(&runtime.artifact_root)
        .unwrap_or_else(|_| runtime.artifact_root.clone())
        .join("downloads")
        .join("procontract");
    fs::create_dir_all(&download_root)
        .with_context(|| format!("creating {}", download_root.display()))?;
    let job_parameters = json!({
        "source_id": "procontract-enrich",
        "cursor": existing_cursor,
        "download_root": download_root.to_string_lossy(),
    });
    let adapter_config = adapter_runtime_config(cli, &skill)?;
    let job_id = create_source_sync_job_from_parameters(
        &mut runtime.state_db,
        "procontract-enrich",
        &skill,
        &job_parameters,
        adapter_config.kind,
    )?;
    let runner = JobRunner::new(LOCAL_EXECUTOR_ID);
    let claimed = runner
        .claim_next_job(runtime.state_db.connection())?
        .ok_or_else(|| anyhow!("expected claimed workflow job for procontract-enrich"))?;
    ensure!(claimed.id == job_id, "claimed unexpected workflow job");

    let session_id = format!("session-pty-{}", sha256_hex(job_id.as_bytes()));
    let executed =
        execute_skill_session(runtime, &skill, &job_id, &session_id, &job_parameters, cli).await?;
    ensure!(
        executed.execution_mode == ExecutionMode::Pty,
        "procontract enrichment skill must resolve to PTY"
    );
    insert_executed_session_log(runtime.state_db.connection(), &skill, &job_id, &executed)?;
    let result_path = executed
        .result_path
        .as_ref()
        .ok_or_else(|| anyhow!("procontract enrichment skill did not produce a result contract"))?;
    let result_json = read_result_contract(result_path)?;
    let persisted_count =
        persist_tender_enrichments(&mut runtime.state_db, "procontract-enrich", &result_json)?;

    let extraction_jobs_processed = drain_pending_extraction_jobs(runtime, cli)
        .await
        .unwrap_or(0);

    let summary = result_json
        .get("summary")
        .cloned()
        .unwrap_or_else(|| json!({}));

    complete_source_job(
        runtime,
        &job_id,
        &json!({
            "job_id": job_id,
            "source_id": "procontract-enrich",
            "enriched_tenders": persisted_count,
            "extraction_jobs_processed": extraction_jobs_processed,
            "summary": summary,
            "session_id": session_id,
            "download_root": download_root.to_string_lossy(),
            "durable_step_surfaces": live_loop_surfaces_json(),
        }),
        format!(
            "procontract tender enrichment completed: job_id={job_id} enriched={persisted_count} extractions={extraction_jobs_processed}"
        ),
    )
}

/// Drain all pending `extract_document_structure` workflow jobs.  Called after
/// enrichment persistence so that downloaded PDFs are immediately processed
/// through the Docling structural extraction pipeline without a separate cron
/// trigger.  Each structural extraction job may in turn enqueue semantic
/// extraction jobs, which are also drained here.
#[allow(clippy::future_not_send)]
async fn drain_pending_extraction_jobs(runtime: &mut RuntimeBootstrap, cli: &Cli) -> Result<usize> {
    let runner = JobRunner::new(LOCAL_EXECUTOR_ID);
    let mut processed = 0_usize;
    loop {
        let Some(claimed) = runner.claim_next_job(runtime.state_db.connection())? else {
            break;
        };
        match claimed.kind.as_str() {
            "extract_document_structure" => {
                let report = run_extract_document_structure_job(runtime, &runner, claimed).await;
                if let Err(error) = &report {
                    ::tracing::warn!(%error, "extraction job failed, continuing drain");
                }
            }
            "extract_document_semantics" => {
                let report =
                    run_extract_document_semantics_job(runtime, &runner, claimed, cli).await;
                if let Err(error) = &report {
                    ::tracing::warn!(%error, "semantic extraction job failed, continuing drain");
                }
            }
            _ => {
                // Not an extraction job — unclaim by breaking out; the job
                // stays claimed but will expire and become re-claimable.
                break;
            }
        }
        processed += 1;
    }
    if processed > 0 {
        ::tracing::info!(processed, "extraction job drain complete");
    }
    Ok(processed)
}

#[allow(
    clippy::too_many_lines,
    clippy::needless_pass_by_value,
    clippy::future_not_send
)]
async fn run_fts_live_loop(
    runtime: &mut RuntimeBootstrap,
    fixture: LiveLoopFixture,
    cli: &Cli,
) -> Result<LiveLoopReport> {
    let repo_root = repo_root()?;
    let ingest_skill = load_skill(
        &repo_root.join("skills/procurement-ingestion/fts"),
        "skills/procurement-ingestion/fts/SKILL.md",
    )?;
    let manifest_skill = load_skill(
        &repo_root.join("skills/manifest-generation"),
        "skills/manifest-generation/SKILL.md",
    )?;

    if let Some(cursor_before) = &fixture.cursor_before {
        seed_cursor_before(&mut runtime.state_db, cursor_before)?;
    }

    let ingest_adapter_config = adapter_runtime_config(cli, &ingest_skill)?;
    let job_id = create_sync_source_job(
        &mut runtime.state_db,
        &ingest_skill,
        &fixture,
        ingest_adapter_config.kind,
    )?;
    let runner = JobRunner::new(LOCAL_EXECUTOR_ID);
    let claimed = runner
        .claim_next_job(runtime.state_db.connection())?
        .ok_or_else(|| anyhow!("expected claimed workflow job"))?;
    ensure!(claimed.id == job_id, "claimed unexpected workflow job");
    if fixture.capture_write_failure {
        return fail_job_with_code(
            runtime,
            &job_id,
            "failed writing PTY capture artifact",
            ErrorCode::AgentRunFailed,
            None,
            None,
            "capture",
            "pty_capture",
        );
    }
    let ingest_session_id = format!("session-pty-{}", sha256_hex(job_id.as_bytes()));
    let mut ingest_execution = execute_skill_session(
        runtime,
        &ingest_skill,
        &job_id,
        &ingest_session_id,
        &claimed.input_payload,
        cli,
    )
    .await?;
    let mut ingest_trace = serde_json::from_str::<Value>(&ingest_execution.structured_trace)
        .unwrap_or_else(|_| json!({}));
    if let Some(trace_object) = ingest_trace.as_object_mut() {
        trace_object.insert(
            "fingerprint_state".to_owned(),
            Value::String(fixture.fingerprint_state.clone()),
        );
        trace_object.insert(
            "locale_state".to_owned(),
            Value::String(fixture.locale_state.clone()),
        );
        trace_object.insert(
            "currency_state".to_owned(),
            Value::String(fixture.currency_state.clone()),
        );
    }
    ingest_execution.structured_trace = ingest_trace.to_string();
    let ingest_host_health = observe_host_health(&fixture, ingest_execution.execution_mode);
    ingest_execution.structured_trace =
        annotate_trace_with_host_health(&ingest_execution.structured_trace, &ingest_host_health)?;
    ensure!(
        ingest_execution.execution_mode == ExecutionMode::Pty,
        "FTS ingestion skill must resolve to PTY"
    );
    insert_executed_session_log(
        runtime.state_db.connection(),
        &ingest_skill,
        &job_id,
        &ingest_execution,
    )?;

    let ingest_run_manifest = write_run_manifest_for_session(
        runtime,
        &job_id,
        &ingest_session_id,
        &ingest_skill,
        execution_mode_label(ingest_execution.execution_mode),
        ingest_execution.started_at,
        &fixture,
        u32::try_from(fixture.notices.len()).context("converting notice count to u32")?,
        None,
        ingest_execution.artifact_refs.clone(),
    )?;
    persist_run_manifest_metadata(
        runtime.state_db.connection(),
        &ingest_session_id,
        &ingest_run_manifest,
    )?;

    if fixture.notices.is_empty() {
        return complete_empty_run(runtime, &job_id);
    }

    if fixture.source_changed {
        return fail_with_manifest(
            runtime,
            &job_id,
            ErrorCode::SourceChanged,
            "source page fingerprint drifted before extraction",
            None,
            Some(&ingest_session_id),
            Some(&ingest_skill.skill_path),
            "ingest",
            "pre_extract",
            json!({
                "fingerprint_state": fixture.fingerprint_state,
                "source_url": fixture.notices.first().map(|notice| notice.source_url.clone()),
            }),
        );
    }

    if fixture.notice_parse_failed {
        persist_raw_documents_only(&mut runtime.state_db, &fixture)?;
        return fail_with_manifest(
            runtime,
            &job_id,
            ErrorCode::NoticeParseFailed,
            "fixture requested notice parse failure",
            None,
            Some(&ingest_session_id),
            Some(&ingest_skill.skill_path),
            "ingest",
            "normalize",
            json!({
                "raw_document_preserved": true,
                "notice_count": fixture.notices.len(),
            }),
        );
    }

    if fixture.basis_hash_mismatch {
        runner.mark_obsolete_and_enqueue_rerun(
            &mut runtime.state_db,
            &ObsoleteJobRequest {
                job_id: job_id.clone(),
                idempotency_key: format!("obsolete-{job_id}"),
                rerun_job_id: format!("rerun-{job_id}"),
                next_job_kind: "sync_source".to_owned(),
                process_id: None,
                scope_kind: Some("source".to_owned()),
                scope_id: Some("fts".to_owned()),
                rerun_from_stage: "ingest".to_owned(),
                expected_input_basis_hash: "expected-basis-hash".to_owned(),
                actual_input_basis_hash: "actual-basis-hash".to_owned(),
            },
        )?;
        return Ok(LiveLoopReport {
            summary_line: format!(
                "FTS live loop obsolete: job_id={} rerun_enqueued=true",
                claimed.id
            ),
            status: "obsolete".to_owned(),
        });
    }

    if let Some(stage_name) = &fixture.set_replacement_failure_stage {
        let process_id = first_process_id_from_db(runtime.state_db.connection())?
            .or_else(|| first_process_id(&fixture).ok())
            .ok_or_else(|| anyhow!("fixture did not provide a process id"))?;
        let manifest_id = create_error_manifest(
            runtime,
            ErrorCode::SqliteWriteFailed,
            &job_id,
            Some(process_id.as_str()),
            Some(&ingest_session_id),
            Some(&ingest_skill.skill_path),
            stage_name,
            "set_replacement",
            "set-replacement stage failed before head movement",
            json!({"stage": stage_name}),
        )?;
        persist_failed_set_replacement_stage(
            &mut runtime.state_db,
            &process_id,
            stage_name,
            &manifest_id,
        )?;
        return fail_job(
            runtime,
            &job_id,
            "set-replacement stage failed before head movement",
            json!({"error_code": "SQLITE_WRITE_FAILED", "stage": stage_name}),
        );
    }

    let pipeline = match persist_pipeline_materialization(
        &mut runtime.state_db,
        &fixture,
        fixture.goods_classification_low_confidence
            || fixture.supplier_match_low_confidence
            || fixture.commodity_code_low_confidence
            || fixture.revenue_basis_uncertain
            || fixture.fpo_access_unavailable,
    ) {
        Ok(report) => report,
        Err(error) => {
            let process_id = first_process_id_from_db(runtime.state_db.connection())?;
            let _ = create_error_manifest(
                runtime,
                ErrorCode::SqliteWriteFailed,
                &job_id,
                process_id.as_deref(),
                Some(&ingest_session_id),
                Some(&ingest_skill.skill_path),
                "pipeline",
                "persist",
                &error.to_string(),
                json!({"error": error.to_string()}),
            )?;
            return fail_job(
                runtime,
                &job_id,
                &error.to_string(),
                json!({"error_code": "SQLITE_WRITE_FAILED"}),
            );
        }
    };

    let contract_session_id = format!(
        "session-contract-{}",
        sha256_hex(format!("{job_id}-manifest").as_bytes())
    );
    let manifest_computation =
        build_manifest_from_db(runtime.state_db.connection(), &pipeline.process_id)?;
    let mut result_payload = json!({
        "process_id": pipeline.process_id,
        "decisionBasisHash": manifest_computation.decision_basis_hash,
        "manifestMarkdown": manifest_computation.markdown,
        "candidate_count": manifest_computation.candidate_count,
    });
    if fixture.manifest_schema_invalid {
        result_payload
            .as_object_mut()
            .ok_or_else(|| anyhow!("result payload must be object"))?
            .remove("process_id");
    }

    let mut contract_execution = execute_skill_session(
        runtime,
        &manifest_skill,
        &job_id,
        &contract_session_id,
        &json!({"process_id": pipeline.process_id}),
        cli,
    )
    .await?;
    ensure!(
        contract_execution.execution_mode == ExecutionMode::Contract,
        "manifest generation skill must resolve to contract mode"
    );
    let contract_host_health = observe_host_health(&fixture, contract_execution.execution_mode);
    contract_execution.structured_trace = annotate_trace_with_host_health(
        &contract_execution.structured_trace,
        &contract_host_health,
    )?;
    insert_executed_session_log(
        runtime.state_db.connection(),
        &manifest_skill,
        &job_id,
        &contract_execution,
    )?;

    let result_artifact_path = match contract_execution.result_path.clone() {
        Some(path) => path,
        None => write_manifest_result_artifact(
            &runtime.artifact_root,
            &contract_session_id,
            contract_execution.started_at,
            &result_payload,
        )?,
    };
    let validated_result_payload = fs::read_to_string(&result_artifact_path)
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok())
        .unwrap_or_else(|| result_payload.clone());
    let validation_outcome = validate_final_result(
        &manifest_skill.result_schema,
        &validated_result_payload,
        &oqg_input(&fixture),
        &result_artifact_path,
    );
    if validation_outcome.disposition != ValidationDisposition::Accepted {
        let error_code = validation_outcome
            .error_code
            .ok_or_else(|| anyhow!("non-accepted validation outcome missing error code"))?;
        let mapped_error = match error_code {
            "AGENT_OUTPUT_CONTRACT_INVALID" => ErrorCode::AgentOutputContractInvalid,
            "AGENT_SUSPECT_OUTPUT" => ErrorCode::AgentSuspectOutput,
            _ => ErrorCode::AgentRunFailed,
        };
        return fail_with_manifest(
            runtime,
            &job_id,
            mapped_error,
            error_code,
            Some(&pipeline.process_id),
            Some(&contract_session_id),
            Some(&manifest_skill.skill_path),
            "manifest",
            "validate",
            json!({
                "artifact_path": result_artifact_path,
                "anomalies": validation_outcome.anomalies,
                "context": validation_outcome.error_context_json,
            }),
        );
    }

    let contract_run_manifest = write_run_manifest_for_session(
        runtime,
        &job_id,
        &contract_session_id,
        &manifest_skill,
        execution_mode_label(contract_execution.execution_mode),
        contract_execution.started_at,
        &fixture,
        manifest_computation.candidate_count,
        None,
        {
            let mut refs = contract_execution.artifact_refs.clone();
            refs.push(result_artifact_path.to_string_lossy().into_owned());
            refs
        },
    )?;
    persist_run_manifest_metadata(
        runtime.state_db.connection(),
        &contract_session_id,
        &contract_run_manifest,
    )?;
    let live_validation = run_live_semantic_validation(
        runtime,
        &runner,
        &job_id,
        &contract_session_id,
        &pipeline.process_id,
        &manifest_skill,
        &contract_run_manifest,
        &fixture,
        manifest_computation.candidate_count,
    )?;
    let skill_telemetry = vec![
        build_skill_run_telemetry(&ingest_skill, &ingest_execution, &fixture, None)?,
        build_skill_run_telemetry(
            &manifest_skill,
            &contract_execution,
            &fixture,
            Some(&live_validation),
        )?,
    ];

    if fixture.goods_classification_low_confidence {
        let _ = create_error_manifest(
            runtime,
            ErrorCode::GoodsClassificationLowConfidence,
            &job_id,
            Some(&pipeline.process_id),
            Some(&ingest_session_id),
            Some(&ingest_skill.skill_path),
            "qualify_lot",
            "goods_classification",
            "goods qualification confidence fell below the autonomous threshold",
            json!({"goods_confidence_bps": 4500}),
        )?;
    }
    if fixture.supplier_match_low_confidence {
        let _ = create_error_manifest(
            runtime,
            ErrorCode::SupplierMatchLowConfidence,
            &job_id,
            Some(&pipeline.process_id),
            Some(&contract_session_id),
            Some(&manifest_skill.skill_path),
            "price_match",
            "supplier_match",
            "supplier match confidence requires manual review",
            json!({"match_confidence_bps": 4200}),
        )?;
    }
    if fixture.commodity_code_low_confidence {
        let _ = create_error_manifest(
            runtime,
            ErrorCode::CommodityCodeLowConfidence,
            &job_id,
            Some(&pipeline.process_id),
            Some(&contract_session_id),
            Some(&manifest_skill.skill_path),
            "landed_cost",
            "commodity_code",
            "commodity-code confidence requires manual review",
            json!({"commodity_confidence_bps": 4300}),
        )?;
    }
    if fixture.revenue_basis_uncertain {
        let _ = create_error_manifest(
            runtime,
            ErrorCode::RevenueBasisUncertain,
            &job_id,
            Some(&pipeline.process_id),
            Some(&contract_session_id),
            Some(&manifest_skill.skill_path),
            "gross_margin",
            "revenue_basis",
            "revenue basis uncertainty reduced confidence",
            json!({"revenue_confidence_bps": 5600}),
        )?;
    }
    if fixture.fpo_access_unavailable {
        let _ = create_error_manifest(
            runtime,
            ErrorCode::FpoAccessUnavailable,
            &job_id,
            Some(&pipeline.process_id),
            Some(&contract_session_id),
            Some(&manifest_skill.skill_path),
            "landed_cost",
            "fpo",
            "FPO lookup unavailable; remaining recall sources used",
            json!({"customs_resolution_confidence_bps": 5200}),
        )?;
    }

    let manifest_path = write_opportunity_manifest(
        &runtime.artifact_root,
        &pipeline.process_id,
        &manifest_computation.markdown,
    )?;
    finalize_success(
        runtime,
        &job_id,
        &pipeline,
        &manifest_computation,
        &manifest_path,
        fixture.notices.len(),
        &live_validation,
        &skill_telemetry,
    )
}

fn create_sync_source_job(
    state_db: &mut StateDb,
    ingest_skill: &LoadedSkill,
    fixture: &LiveLoopFixture,
    agent_cli: AgentCliKind,
) -> Result<String> {
    let job_id = format!(
        "job-{}",
        sha256_hex(&UuidSeed::from_time(Utc::now()).as_bytes())
    );
    let ((), _) = state_db.with_write_transaction(|tx| {
        tx.execute(
            "\
            INSERT INTO workflow_jobs (
              id, kind, scope_kind, scope_id, status, input_payload, validation_required,
              parent_job_id, validation_verdict_id
            ) VALUES (?1, 'sync_source', 'source', 'fts', 'pending', ?2, 0, NULL, NULL)
            ",
            params![
                job_id,
                json!({
                    "source_id": "fts",
                    "trigger": "manual_cli",
                    "skill_binding": skill_binding_payload(ingest_skill),
                    "agent_cli": agent_cli.as_str(),
                    "operator_requested": fixture.operator_requested,
                    "cursor_before": fixture.cursor_before,
                    "cursor_after": fixture.cursor_after,
                    "durable_step_surfaces": live_loop_surfaces_json(),
                })
                .to_string(),
            ],
        )?;
        Ok(())
    })?;
    Ok(job_id)
}

fn persist_raw_documents_only(state_db: &mut StateDb, fixture: &LiveLoopFixture) -> Result<()> {
    state_db.with_write_transaction(|tx| {
        let visible_after_seq = next_commit_seq(tx)?;
        for notice in &fixture.notices {
            insert_raw_document(tx, notice, visible_after_seq)?;
        }
        Ok(())
    })?;
    Ok(())
}

#[allow(clippy::too_many_lines)]
fn persist_pipeline_materialization(
    state_db: &mut StateDb,
    fixture: &LiveLoopFixture,
    review_required: bool,
) -> Result<PipelinePersistence> {
    let first_notice = fixture
        .notices
        .first()
        .ok_or_else(|| anyhow!("at least one notice is required for pipeline materialization"))?;
    let process_id = first_notice.process_id.clone();
    let first_notice_title = first_notice.title.clone();
    let review_reason = review_reason(fixture);

    let (report, _) = state_db.with_write_transaction(|tx| {
        let visible_after_seq = next_commit_seq(tx)?;
        let stage_run_ids = stage_run_ids();

        if fixture.sqlite_write_failure {
            insert_raw_document(tx, first_notice, visible_after_seq)?;
            tx.execute(
                "\
                INSERT INTO procurement_processes (
                  id, current_stage, title, latest_published_at, latest_updated_at,
                  owner_id, last_local_seq, stage_run_id
                ) VALUES (?1, 'normalize', ?2, ?3, ?4, ?5, ?6, ?7)
                ",
                params![
                    process_id,
                    first_notice_title,
                    first_notice.published_at,
                    first_notice.source_updated_at,
                    DEFAULT_OWNER_ID,
                    visible_after_seq,
                    stage_run_ids.normalize,
                ],
            )?;
            bail!("SQLITE_WRITE_FAILED: forced rollback during pipeline persistence");
        }

        let mut gross_margin_bps = 0_i64;
        let mut net_margin_bps = 0_i64;

        for notice in &fixture.notices {
            insert_raw_document(tx, notice, visible_after_seq)?;
            tx.execute(
                "\
                INSERT INTO procurement_processes (
                  id, canonical_ocid, primary_notice_id, current_stage, title,
                  latest_published_at, latest_updated_at, owner_id, last_local_seq, stage_run_id
                ) VALUES (?1, ?2, ?3, 'manifest', ?4, ?5, ?6, ?7, ?8, ?9)
                ON CONFLICT(id) DO NOTHING
                ",
                params![
                    notice.process_id,
                    notice.ocid,
                    notice.notice_id,
                    notice.title,
                    notice.published_at,
                    notice.source_updated_at,
                    DEFAULT_OWNER_ID,
                    visible_after_seq,
                    stage_run_ids.normalize,
                ],
            )?;
            tx.execute(
                "\
                INSERT INTO contract_notices (
                  id, process_id, source_id, source_notice_id, raw_document_id, ocid, title,
                  summary, buyer_name, procurement_stage, access_route, published_at,
                  source_updated_at, normalized_payload, owner_id, last_local_seq, stage_run_id
                ) VALUES (?1, ?2, 'fts', ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'public_web', ?10, ?11,
                          ?12, ?13, ?14, ?15)
                ",
                params![
                    notice.notice_id,
                    notice.process_id,
                    notice.source_notice_id,
                    format!("raw-{}", notice.notice_id),
                    notice.ocid,
                    notice.title,
                    format!("Normalized {}", notice.title),
                    notice.buyer_name,
                    notice.procurement_stage,
                    notice.published_at,
                    notice.source_updated_at,
                    json!({"source_url": notice.source_url}).to_string(),
                    DEFAULT_OWNER_ID,
                    visible_after_seq,
                    stage_run_ids.normalize,
                ],
            )?;

            persist_stage(tx, &notice.process_id, "Normalize", "notice", &notice.notice_id, &stage_run_ids.normalize)?;
            persist_stage(tx, &notice.process_id, "Prequalify notice", "notice", &notice.notice_id, &stage_run_ids.prequalify_notice)?;
            persist_stage(tx, &notice.process_id, "Acquire docs", "notice", &notice.notice_id, &stage_run_ids.acquire_docs)?;
            persist_stage(tx, &notice.process_id, "Extract lots", "notice", &notice.notice_id, &stage_run_ids.extract_lots)?;

            for lot in &notice.lots {
                tx.execute(
                    "\
                    INSERT INTO notice_lots (
                      id, notice_id, source_lot_id, title, estimated_value_pence,
                      owner_id, last_local_seq, stage_run_id
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                    ",
                    params![
                        lot.lot_id,
                        notice.notice_id,
                        lot.source_lot_id,
                        lot.title,
                        lot.estimated_value_pence,
                        DEFAULT_OWNER_ID,
                        visible_after_seq,
                        stage_run_ids.extract_lots,
                    ],
                )?;
                tx.execute(
                    "\
                    INSERT INTO qualification_runs (
                      id, process_id, notice_id, lot_id, expiry_qualified, goods_qualified,
                      goods_confidence_bps, qualification_basis, rationale, owner_id,
                      last_local_seq, stage_run_id
                    ) VALUES (?1, ?2, ?3, ?4, 1, 1, ?5, 'fixture', ?6, ?7, ?8, ?9)
                    ",
                    params![
                        format!("qual-{}", lot.lot_id),
                        notice.process_id,
                        notice.notice_id,
                        lot.lot_id,
                        if fixture.goods_classification_low_confidence { 4500 } else { 9000 },
                        if fixture.goods_classification_low_confidence {
                            "Low-confidence goods classification retained for review"
                        } else {
                            "Goods classification passed"
                        },
                        DEFAULT_OWNER_ID,
                        visible_after_seq,
                        stage_run_ids.qualify_lot,
                    ],
                )?;

                persist_stage(tx, &notice.process_id, "Qualify lot", "lot", &lot.lot_id, &stage_run_ids.qualify_lot)?;
                persist_stage(tx, &notice.process_id, "Extract products", "lot", &lot.lot_id, &stage_run_ids.extract_products)?;

                for index in 0..lot.candidate_count {
                    let candidate_id = format!("{}-candidate-{}", lot.lot_id, index + 1);
                    let quote_id = format!("quote-{candidate_id}");
                    let source_pair_id = format!("pair-{candidate_id}");
                    let quote_scenario_id = format!("scenario-{candidate_id}");
                    gross_margin_bps = if fixture.revenue_basis_uncertain { 1800 } else { 3200 };
                    net_margin_bps = if fixture.fpo_access_unavailable { 900 } else { 2100 };

                    tx.execute(
                        "\
                        INSERT INTO product_candidates (
                          id, process_id, notice_id, lot_id, name, candidate_type, match_archetype,
                          head_noun, quoteability_score_bps, identity_confidence_bps,
                          spec_confidence_bps, quantity_confidence_bps, owner_id, last_local_seq,
                          stage_run_id
                        ) VALUES (?1, ?2, ?3, ?4, ?5, 'sku', 'direct_match', 'widget',
                                  7800, 8500, 8200, 8000, ?6, ?7, ?8)
                        ",
                        params![
                            candidate_id,
                            notice.process_id,
                            notice.notice_id,
                            lot.lot_id,
                            format!("Candidate {}", index + 1),
                            DEFAULT_OWNER_ID,
                            visible_after_seq,
                            stage_run_ids.extract_products,
                        ],
                    )?;
                    tx.execute(
                        "\
                        INSERT INTO supplier_quotes (
                          id, process_id, notice_id, product_candidate_id, product_name, region,
                          quote_class, source_name, source_url, unit_price_original_minor, currency,
                          unit_price_gbp_pence, price_signal_role, evidence, owner_id,
                          last_local_seq, stage_run_id
                        ) VALUES (?1, ?2, ?3, ?4, ?5, 'GB', 'spot', 'fixture-supplier', ?6,
                                  2500, 'GBP', 2500, 'primary', ?7, ?8, ?9, ?10)
                        ",
                        params![
                            quote_id,
                            notice.process_id,
                            notice.notice_id,
                            candidate_id,
                            format!("Candidate {}", index + 1),
                            notice.source_url,
                            json!({"source_pair_id": source_pair_id}).to_string(),
                            DEFAULT_OWNER_ID,
                            visible_after_seq,
                            stage_run_ids.price_signal_match,
                        ],
                    )?;
                    tx.execute(
                        "\
                        INSERT INTO candidate_pricing_assessments (
                          id, process_id, notice_id, product_candidate_id, selected_quote_id,
                          selected_quote_row_version, quote_cost_boundary, revenue_basis_type,
                          expected_addressable_revenue_ex_vat_pence, revenue_confidence_bps,
                          gross_margin_bps, gross_estimate_confidence_bps, identity_gate_passed,
                          usability_gate_passed, pricing_input_confidence_bps, owner_id,
                          last_local_seq, stage_run_id
                        ) VALUES (?1, ?2, ?3, ?4, ?5, 1, 'unit', 'expected_value',
                                  120000, ?6, ?7, 7600, 1, 1, 7400, ?8, ?9, ?10)
                        ",
                        params![
                            format!("pricing-{candidate_id}"),
                            notice.process_id,
                            notice.notice_id,
                            candidate_id,
                            quote_id,
                            if fixture.revenue_basis_uncertain { 5600 } else { 8400 },
                            gross_margin_bps,
                            DEFAULT_OWNER_ID,
                            visible_after_seq,
                            stage_run_ids.gross_margin_gate,
                        ],
                    )?;
                    tx.execute(
                        "\
                        INSERT INTO landed_cost_estimates (
                          id, process_id, notice_id, product_candidate_id, supplier_quote_id,
                          shipping_mode, mode_confidence_bps, declarable_commodity_code_10,
                          commodity_confidence_bps, classification_stability_bps,
                          customs_valuation_method, valuation_mode, customs_fx_rate, commercial_fx_rate,
                          fx_basis_delta_bps, supplier_cost_gbp_pence, freight_low_gbp_pence,
                          freight_base_gbp_pence, freight_high_gbp_pence, customs_duty_gbp_pence,
                          import_vat_gbp_pence, import_vat_treatment, uk_final_mile_cost_gbp_pence,
                          other_fees_gbp_pence,
                          economic_landed_cost_gbp_pence, cash_outlay_gbp_pence, cash_to_clear_gbp_pence,
                          revenue_basis_type, revenue_confidence_bps, customs_resolution_confidence_bps,
                          commercial_estimate_confidence_bps, net_margin_low_bps, net_margin_base_bps,
                          net_margin_high_bps, decision_grade, confidence_bps, warnings,
                          owner_id, last_local_seq, stage_run_id
                        ) VALUES (?1, ?2, ?3, ?4, ?5, 'parcel', 7200, '1234567890', ?6, 7000,
                                  'transaction_value', 'standard', 1.0, 1.0, 0, 2500, 200, 400, 600, 300,
                                  200, 'recoverable', 150, 50, 3600, 3600, 3600, 'expected_value', ?7, ?8,
                                  7800, 600, 900, 1200, 'reviewable', 7600, '[]', ?9, ?10, ?11)
                        ",
                        params![
                            format!("landed-{candidate_id}"),
                            notice.process_id,
                            notice.notice_id,
                            candidate_id,
                            quote_id,
                            if fixture.commodity_code_low_confidence { 4300 } else { 8100 },
                            if fixture.revenue_basis_uncertain { 5600 } else { 8400 },
                            if fixture.fpo_access_unavailable { 5200 } else { 8300 },
                            DEFAULT_OWNER_ID,
                            visible_after_seq,
                            stage_run_ids.landed_cost,
                        ],
                    )?;

                    persist_stage(
                        tx,
                        &notice.process_id,
                        "Score quoteability",
                        "candidate",
                        &candidate_id,
                        &stage_run_ids.score_quoteability,
                    )?;
                    persist_stage(
                        tx,
                        &notice.process_id,
                        "Price-signal match",
                        "candidate_source_pair",
                        &source_pair_id,
                        &stage_run_ids.price_signal_match,
                    )?;
                    persist_stage(
                        tx,
                        &notice.process_id,
                        "Gross margin gate",
                        "candidate",
                        &candidate_id,
                        &stage_run_ids.gross_margin_gate,
                    )?;
                    persist_stage(
                        tx,
                        &notice.process_id,
                        "Landed cost",
                        "candidate_quote_scenario",
                        &quote_scenario_id,
                        &stage_run_ids.landed_cost,
                    )?;
                    persist_stage(
                        tx,
                        &notice.process_id,
                        "Net margin gate",
                        "candidate",
                        &candidate_id,
                        &stage_run_ids.net_margin_gate,
                    )?;
                }
            }
        }

        persist_stage(
            tx,
            &process_id,
            "Relate",
            "process",
            &process_id,
            &stage_run_ids.relate,
        )?;

        if !fixture.notices.is_empty() {
            tx.execute(
                "\
                INSERT INTO source_cursors (source_id, cursor_key, cursor_value, updated_at)
                VALUES ('fts', 'notice_cursor', ?1, CURRENT_TIMESTAMP)
                ON CONFLICT(source_id, cursor_key) DO UPDATE SET
                  cursor_value = excluded.cursor_value,
                  updated_at = excluded.updated_at
                ",
                params![json!({
                    "cursor_before": fixture.cursor_before,
                    "cursor_after": fixture.cursor_after,
                    "backtrack_window_seconds": BACKTRACK_WINDOW_SECONDS,
                })
                .to_string()],
            )?;
        }

        for target in &fixture.soft_stale_targets {
            set_stage_head_freshness(
                tx,
                &process_id,
                &target.stage,
                &target.scope_kind,
                &target.scope_id,
                FreshnessState::SoftStale,
            )?;
        }
        for target in &fixture.hard_invalid_targets {
            set_stage_head_freshness(
                tx,
                &process_id,
                &target.stage,
                &target.scope_kind,
                &target.scope_id,
                FreshnessState::HardInvalid,
            )?;
        }

        Ok(PipelinePersistence {
            process_id,
            review_required,
            review_reason,
            gross_margin_bps,
            net_margin_bps,
            manifest_stage_run_id: stage_run_ids.manifest,
        })
    })?;

    Ok(report)
}

#[allow(clippy::too_many_lines)]
fn persist_ingestion_from_agent_output(
    state_db: &mut StateDb,
    source_id: &str,
    result_json: &Value,
) -> Result<usize> {
    let notices = result_json
        .get("notices")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("result contract missing notices array"))?;
    let cursor = result_json
        .get("cursor")
        .and_then(Value::as_str)
        .unwrap_or("");

    if notices.is_empty() {
        return Ok(0);
    }

    let stage_run_id = allocate_stage_run_id();

    let (persisted_count, _) = state_db.with_write_transaction(|tx| {
        let visible_after_seq = next_commit_seq(tx)?;
        let mut count = 0_usize;

        for notice in notices {
            let notice_id = notice
                .get("notice_id")
                .and_then(Value::as_str)
                .ok_or_else(|| anyhow!("notice missing notice_id"))?;
            let source_notice_id = notice
                .get("source_notice_id")
                .and_then(Value::as_str)
                .unwrap_or(notice_id);
            let ocid = notice.get("ocid").and_then(Value::as_str).unwrap_or("");
            let title = notice
                .get("title")
                .and_then(Value::as_str)
                .ok_or_else(|| anyhow!("notice missing title"))?;
            let summary = notice.get("summary").and_then(Value::as_str);
            let buyer_name = notice.get("buyer_name").and_then(Value::as_str);
            let procurement_stage = notice
                .get("procurement_stage")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            let main_category = notice.get("main_category").and_then(Value::as_str);
            // Awarded contracts have historical deadlines that are always in the past.
            // The queue page filters out processes with past deadlines, so clear the
            // deadline for awards so they remain visible.
            let deadline_at = notice
                .get("deadline_at")
                .and_then(Value::as_str)
                .filter(|_| procurement_stage != "awarded");
            let published_at = notice
                .get("published_at")
                .and_then(Value::as_str)
                .ok_or_else(|| anyhow!("notice missing published_at"))?;
            let source_updated_at = notice
                .get("source_updated_at")
                .and_then(Value::as_str)
                .ok_or_else(|| anyhow!("notice missing source_updated_at"))?;
            let estimated_value_low_pence = notice
                .get("estimated_value_low_pence")
                .and_then(Value::as_i64);
            let estimated_value_currency = notice
                .get("estimated_value_currency")
                .and_then(Value::as_str);
            let access_route = notice
                .get("access_route")
                .and_then(Value::as_str)
                .unwrap_or("public_web");
            let source_url = notice
                .get("source_url")
                .and_then(Value::as_str)
                .unwrap_or("");
            let raw_release_json = notice
                .get("raw_release_json")
                .and_then(Value::as_str)
                .unwrap_or("{}");

            let raw_doc_id = format!("raw-{notice_id}");
            let process_id = format!("proc-{ocid}");

            tx.execute(
                "\
                INSERT INTO raw_documents (
                  id, source_id, content_type, source_url, payload_json, metadata, created_at
                ) VALUES (?1, ?2, 'application/json', ?3, ?4, ?5, CURRENT_TIMESTAMP)
                ON CONFLICT(id) DO UPDATE SET
                  payload_json = excluded.payload_json,
                  source_url = excluded.source_url
                ",
                params![
                    raw_doc_id,
                    source_id,
                    source_url,
                    raw_release_json,
                    json!({"last_local_seq": visible_after_seq}).to_string(),
                ],
            )?;

            tx.execute(
                "\
                INSERT INTO procurement_processes (
                  id, canonical_ocid, primary_notice_id, current_stage, buyer_name, title,
                  latest_deadline_at, latest_published_at, latest_updated_at,
                  owner_id, row_version, last_local_seq, stage_run_id
                ) VALUES (?1, ?2, ?3, 'normalize', ?4, ?5, ?6, ?7, ?8, ?9, 1, ?10, ?11)
                ON CONFLICT(id) DO UPDATE SET
                  latest_published_at = CASE
                    WHEN excluded.latest_published_at > procurement_processes.latest_published_at
                    THEN excluded.latest_published_at
                    ELSE procurement_processes.latest_published_at
                  END,
                  latest_updated_at = CASE
                    WHEN excluded.latest_updated_at > procurement_processes.latest_updated_at
                    THEN excluded.latest_updated_at
                    ELSE procurement_processes.latest_updated_at
                  END,
                  latest_deadline_at = COALESCE(excluded.latest_deadline_at, procurement_processes.latest_deadline_at),
                  row_version = procurement_processes.row_version + 1,
                  last_local_seq = excluded.last_local_seq,
                  stage_run_id = excluded.stage_run_id,
                  updated_at = CURRENT_TIMESTAMP
                ",
                params![
                    process_id,
                    ocid,
                    notice_id,
                    buyer_name,
                    title,
                    deadline_at,
                    published_at,
                    source_updated_at,
                    LOCAL_EXECUTOR_ID,
                    visible_after_seq,
                    stage_run_id,
                ],
            )?;

            tx.execute(
                "\
                INSERT INTO contract_notices (
                  id, process_id, source_id, source_notice_id, raw_document_id, ocid, title,
                  summary, buyer_name, procurement_stage, access_route, deadline_at, published_at,
                  source_updated_at, main_category, estimated_value_original_minor,
                  estimated_value_currency, normalized_payload,
                  owner_id, row_version, last_local_seq, stage_run_id
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16,
                          ?17, ?18, ?19, 1, ?20, ?21)
                ON CONFLICT(source_id, source_notice_id) DO UPDATE SET
                  title = excluded.title,
                  summary = excluded.summary,
                  buyer_name = excluded.buyer_name,
                  procurement_stage = excluded.procurement_stage,
                  deadline_at = excluded.deadline_at,
                  source_updated_at = excluded.source_updated_at,
                  normalized_payload = excluded.normalized_payload,
                  row_version = contract_notices.row_version + 1,
                  last_local_seq = excluded.last_local_seq,
                  stage_run_id = excluded.stage_run_id,
                  updated_at = CURRENT_TIMESTAMP
                ",
                params![
                    notice_id,
                    process_id,
                    source_id,
                    source_notice_id,
                    raw_doc_id,
                    ocid,
                    title,
                    summary,
                    buyer_name,
                    procurement_stage,
                    access_route,
                    deadline_at,
                    published_at,
                    source_updated_at,
                    main_category,
                    estimated_value_low_pence,
                    estimated_value_currency,
                    raw_release_json,
                    LOCAL_EXECUTOR_ID,
                    visible_after_seq,
                    stage_run_id,
                ],
            )?;

            persist_stage(
                tx,
                &process_id,
                "Normalize",
                "notice",
                notice_id,
                &stage_run_id,
            )?;

            if let Some(lots) = notice.get("lots").and_then(Value::as_array) {
                for lot in lots {
                    let lot_id = lot
                        .get("lot_id")
                        .and_then(Value::as_str)
                        .unwrap_or_else(|| {
                            lot.get("source_lot_id")
                                .and_then(Value::as_str)
                                .unwrap_or("unknown")
                        });
                    let source_lot_id = lot.get("source_lot_id").and_then(Value::as_str);
                    let lot_title = lot.get("title").and_then(Value::as_str);
                    let lot_description = lot.get("description").and_then(Value::as_str);
                    let lot_value_pence = lot
                        .get("estimated_value_pence")
                        .and_then(Value::as_i64);
                    let lot_deadline = lot.get("deadline_at").and_then(Value::as_str);

                    tx.execute(
                        "\
                        INSERT INTO notice_lots (
                          id, notice_id, source_lot_id, title, description,
                          estimated_value_pence, deadline_at,
                          owner_id, row_version, last_local_seq, stage_run_id
                        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 1, ?9, ?10)
                        ON CONFLICT(id) DO UPDATE SET
                          title = excluded.title,
                          description = excluded.description,
                          estimated_value_pence = excluded.estimated_value_pence,
                          deadline_at = excluded.deadline_at,
                          row_version = notice_lots.row_version + 1,
                          last_local_seq = excluded.last_local_seq,
                          stage_run_id = excluded.stage_run_id,
                          updated_at = CURRENT_TIMESTAMP
                        ",
                        params![
                            lot_id,
                            notice_id,
                            source_lot_id,
                            lot_title,
                            lot_description,
                            lot_value_pence,
                            lot_deadline,
                            LOCAL_EXECUTOR_ID,
                            visible_after_seq,
                            stage_run_id,
                        ],
                    )?;
                }
            }

            count += 1;
        }

        if !cursor.is_empty() {
            tx.execute(
                "\
                INSERT INTO source_cursors (source_id, cursor_key, cursor_value, updated_at)
                VALUES (?1, 'notice_cursor', ?2, CURRENT_TIMESTAMP)
                ON CONFLICT(source_id, cursor_key) DO UPDATE SET
                  cursor_value = excluded.cursor_value,
                  updated_at = excluded.updated_at
                ",
                params![source_id, cursor],
            )?;
        }

        Ok(count)
    })?;

    Ok(persisted_count)
}

#[cfg(test)]
#[allow(dead_code)]
pub(crate) fn persist_ingestion_from_agent_output_for_test(
    state_db: &mut StateDb,
    source_id: &str,
    result_json: &Value,
) -> Result<usize> {
    persist_ingestion_from_agent_output(state_db, source_id, result_json)
}

#[allow(clippy::too_many_lines)]
fn persist_extraction_from_agent_output(
    state_db: &mut StateDb,
    document_extraction_id: &str,
    process_id: &str,
    skill: &LoadedSkill,
    job_id: &str,
    session_id: &str,
    result_json: &Value,
) -> Result<SemanticPersistenceOutcome> {
    let skill_id = skill.metadata.skill_id.as_str();
    let target_domain = target_domain_for_overlay(skill_id).to_owned();
    let extraction_scope = required_string_at(
        result_json,
        &["extraction_metadata", "extraction_scope"],
        "extraction_metadata.extraction_scope",
    )?
    .to_owned();
    let field_confidences_json = optional_value_at(result_json, &["field_confidences"])
        .cloned()
        .unwrap_or_else(|| json!({}))
        .to_string();
    let warnings_json = optional_value_at(result_json, &["warnings"])
        .cloned()
        .unwrap_or_else(|| json!([]))
        .to_string();
    let evidence_anchors_json = optional_value_at(result_json, &["evidence_anchors"])
        .cloned()
        .unwrap_or_else(|| json!([]))
        .to_string();
    let extraction_confidence_bps = required_i64_at(
        result_json,
        &["extraction_confidence_bps"],
        "extraction_confidence_bps",
    )?;
    let extracted_data_json = result_json.to_string();
    let semantic_extraction_id = format!("semantic-{}", Uuid::new_v4());

    let (outcome, _) = state_db.with_write_transaction(|tx| {
        let visible_after_seq = next_commit_seq(tx)?;
        let context = load_document_extraction_context(tx, document_extraction_id)?;
        let previous_semantic = load_current_semantic_snapshot(
            tx,
            document_extraction_id,
            skill_id,
            &extraction_scope,
        )?;
        let written_entity_refs = persist_semantic_domain_writes(
            tx,
            &SemanticDomainWriteContext {
                document_extraction_id,
                process_id,
                notice_id: context.notice_id.as_deref(),
                skill_id,
                stage_run_id: &context.stage_run_id,
                visible_after_seq,
            },
            result_json,
        )?;

        tx.execute(
            "\
            UPDATE document_semantic_extractions
            SET is_current = 0,
                updated_at = CURRENT_TIMESTAMP
            WHERE document_extraction_id = ?1
              AND skill_id = ?2
              AND extraction_scope = ?3
              AND is_current = 1
              AND id <> ?4
            ",
            params![
                document_extraction_id,
                skill_id,
                extraction_scope,
                semantic_extraction_id,
            ],
        )?;

        tx.execute(
            "\
            INSERT INTO document_semantic_extractions (
              id, document_extraction_id, process_id, skill_id, skill_version, job_id, session_id,
              extraction_scope, target_domain, is_current, extracted_data_json, field_confidences_json,
              extraction_confidence_bps, warnings_json, evidence_anchors_json, written_entity_refs_json,
              owner_id, row_version, last_local_seq, stage_run_id
            ) VALUES (
              ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 1, ?10, ?11, ?12, ?13, ?14, ?15,
              ?16, 1, ?17, ?18
            )
            ",
            params![
                semantic_extraction_id,
                document_extraction_id,
                process_id,
                skill_id,
                skill.metadata.skill_version.clone(),
                job_id,
                session_id,
                extraction_scope,
                target_domain,
                extracted_data_json,
                field_confidences_json,
                extraction_confidence_bps,
                warnings_json,
                evidence_anchors_json,
                json!(written_entity_refs).to_string(),
                DEFAULT_OWNER_ID,
                visible_after_seq,
                context.stage_run_id,
            ],
        )?;

        tx.execute(
            "\
            UPDATE document_semantic_extractions
            SET superseded_by_semantic_extraction_id = ?1,
                updated_at = CURRENT_TIMESTAMP
            WHERE document_extraction_id = ?2
              AND skill_id = ?3
              AND extraction_scope = ?4
              AND is_current = 0
              AND id <> ?1
              AND superseded_by_semantic_extraction_id IS NULL
            ",
            params![
                semantic_extraction_id,
                document_extraction_id,
                skill_id,
                extraction_scope,
            ],
        )?;

        let aggregate_state = recompute_semantic_extraction_state(
            tx,
            document_extraction_id,
            &context.expected_overlays,
        )?;
        tx.execute(
            "\
            UPDATE document_extractions
            SET semantic_extraction_state = ?1,
                updated_at = CURRENT_TIMESTAMP
            WHERE id = ?2
            ",
            params![aggregate_state, document_extraction_id],
        )?;
        let document_set_extraction_state =
            update_document_set_extraction_state(tx, context.document_set_id.as_deref())?;
        let downstream_refresh_required = authoritative_data_changed(
            &target_domain,
            previous_semantic.as_ref(),
            result_json,
            &written_entity_refs,
        )?;
        if downstream_refresh_required {
            mark_downstream_refresh_targets(tx, process_id, context.notice_id.as_deref())?;
        }

        Ok(SemanticPersistenceOutcome {
            semantic_extraction_state: aggregate_state,
            downstream_refresh_required,
            document_set_extraction_state,
        })
    })?;
    Ok(outcome)
}

#[cfg(test)]
#[allow(dead_code)]
pub(crate) fn persist_extraction_from_agent_output_for_test(
    state_db: &mut StateDb,
    document_extraction_id: &str,
    process_id: &str,
    skill_relative_path: &str,
    job_id: &str,
    session_id: &str,
    result_json: &Value,
) -> Result<()> {
    let skill = load_pdf_extraction_skill(skill_relative_path)?;
    persist_extraction_from_agent_output(
        state_db,
        document_extraction_id,
        process_id,
        &skill,
        job_id,
        session_id,
        result_json,
    )?;
    Ok(())
}

struct DocumentExtractionContext {
    notice_id: Option<String>,
    document_set_id: Option<String>,
    expected_overlays: Vec<String>,
    stage_run_id: String,
}

struct CurrentSemanticSnapshot {
    extracted_data_json: String,
    written_entity_refs_json: String,
}

struct SemanticDomainWriteContext<'a> {
    document_extraction_id: &'a str,
    process_id: &'a str,
    notice_id: Option<&'a str>,
    skill_id: &'a str,
    stage_run_id: &'a str,
    visible_after_seq: i64,
}

fn load_document_extraction_context(
    tx: &Transaction<'_>,
    document_extraction_id: &str,
) -> Result<DocumentExtractionContext> {
    let (notice_id, document_set_id, expected_overlays_json, stage_run_id): (
        Option<String>,
        Option<String>,
        String,
        Option<String>,
    ) = tx.query_row(
        "\
        SELECT notice_id, document_set_id, expected_overlays_json, stage_run_id
        FROM document_extractions
        WHERE id = ?1
        ",
        params![document_extraction_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
    )?;
    let expected_overlays = serde_json::from_str::<Vec<String>>(&expected_overlays_json)
        .context("parsing document_extractions.expected_overlays_json")?;
    Ok(DocumentExtractionContext {
        notice_id,
        document_set_id,
        expected_overlays,
        stage_run_id: stage_run_id.unwrap_or_else(allocate_stage_run_id),
    })
}

fn load_current_semantic_snapshot(
    tx: &Transaction<'_>,
    document_extraction_id: &str,
    skill_id: &str,
    extraction_scope: &str,
) -> Result<Option<CurrentSemanticSnapshot>> {
    tx.query_row(
        "\
        SELECT extracted_data_json, COALESCE(written_entity_refs_json, '[]')
        FROM document_semantic_extractions
        WHERE document_extraction_id = ?1
          AND skill_id = ?2
          AND extraction_scope = ?3
          AND is_current = 1
        LIMIT 1
        ",
        params![document_extraction_id, skill_id, extraction_scope],
        |row| {
            Ok(CurrentSemanticSnapshot {
                extracted_data_json: row.get(0)?,
                written_entity_refs_json: row.get(1)?,
            })
        },
    )
    .optional()
    .context("loading current semantic extraction snapshot")
}

fn authoritative_data_changed(
    target_domain: &str,
    previous: Option<&CurrentSemanticSnapshot>,
    result_json: &Value,
    written_entity_refs: &[Value],
) -> Result<bool> {
    if target_domain == "generic" || written_entity_refs.is_empty() {
        return Ok(false);
    }
    let Some(previous) = previous else {
        return Ok(true);
    };

    let previous_result: Value = serde_json::from_str(&previous.extracted_data_json)
        .context("parsing prior semantic extracted_data_json")?;
    if previous_result != *result_json {
        return Ok(true);
    }

    let previous_refs: Vec<Value> = serde_json::from_str(&previous.written_entity_refs_json)
        .context("parsing prior semantic written_entity_refs_json")?;
    Ok(previous_refs != written_entity_refs)
}

fn mark_downstream_refresh_targets(
    tx: &Transaction<'_>,
    process_id: &str,
    notice_id: Option<&str>,
) -> Result<()> {
    let Some(notice_id) = notice_id else {
        return Ok(());
    };

    try_set_stage_head_freshness(
        tx,
        process_id,
        "Extract lots",
        "notice",
        notice_id,
        FreshnessState::SoftStale,
    )?;

    let mut statement = tx.prepare(
        "\
        SELECT id
        FROM notice_lots
        WHERE notice_id = ?1
        ",
    )?;
    let lot_ids = statement
        .query_map(params![notice_id], |row| row.get::<_, String>(0))?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    for lot_id in lot_ids {
        try_set_stage_head_freshness(
            tx,
            process_id,
            "Extract products",
            "lot",
            &lot_id,
            FreshnessState::SoftStale,
        )?;
    }
    Ok(())
}

fn try_set_stage_head_freshness(
    conn: &Connection,
    process_id: &str,
    stage: &str,
    scope_kind: &str,
    scope_id: &str,
    freshness_state: FreshnessState,
) -> Result<()> {
    conn.execute(
        "\
        UPDATE process_stage_heads
        SET freshness_state = ?1,
            updated_at = CURRENT_TIMESTAMP
        WHERE process_id = ?2
          AND stage = ?3
          AND scope_kind = ?4
          AND scope_id = ?5
        ",
        params![
            freshness_state.as_str(),
            process_id,
            stage,
            scope_kind,
            scope_id,
        ],
    )?;
    Ok(())
}

fn update_document_set_extraction_state(
    conn: &Connection,
    document_set_id: Option<&str>,
) -> Result<Option<String>> {
    let Some(document_set_id) = document_set_id else {
        return Ok(None);
    };
    let extraction_state = recompute_document_set_extraction_state(conn, document_set_id)?;
    conn.execute(
        "\
        UPDATE tender_document_sets
        SET extraction_state = ?1,
            updated_at = CURRENT_TIMESTAMP
        WHERE id = ?2
        ",
        params![extraction_state, document_set_id],
    )?;
    Ok(Some(extraction_state))
}

fn recompute_document_set_extraction_state(
    conn: &Connection,
    document_set_id: &str,
) -> Result<String> {
    let document_set_inputs = load_document_set_extractable_pdf_inputs(conn, document_set_id)?;
    if document_set_inputs.completeness_state == "none"
        || document_set_inputs.extractable_raw_document_ids.is_empty()
    {
        return Ok("none".to_owned());
    }

    let expected_document_count = document_set_inputs.extractable_raw_document_ids.len();
    let mut statement = conn.prepare(
        "\
        SELECT raw_document_id, semantic_extraction_state
        FROM document_extractions
        WHERE document_set_id = ?1
          AND is_current = 1
        ",
    )?;
    let states = statement
        .query_map(params![document_set_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?
        .into_iter()
        .filter_map(|(raw_document_id, state)| {
            document_set_inputs
                .extractable_raw_document_ids
                .iter()
                .any(|expected| expected == &raw_document_id)
                .then_some(state)
        })
        .collect::<Vec<_>>();

    if states.is_empty() {
        return Ok("none".to_owned());
    }
    if states.iter().any(|state| state == "failed") {
        return Ok("failed".to_owned());
    }
    if states
        .iter()
        .all(|state| matches!(state.as_str(), "complete" | "skipped"))
        && states.len() >= expected_document_count
    {
        return Ok("complete".to_owned());
    }
    Ok("partial".to_owned())
}

struct DocumentSetExtractablePdfInputs {
    completeness_state: String,
    extractable_raw_document_ids: Vec<String>,
}

fn load_document_set_extractable_pdf_inputs(
    conn: &Connection,
    document_set_id: &str,
) -> Result<DocumentSetExtractablePdfInputs> {
    let (completeness_state, documents_json): (String, String) = conn.query_row(
        "\
        SELECT completeness_state, documents_json
        FROM tender_document_sets
        WHERE id = ?1
        ",
        params![document_set_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;

    if completeness_state == "none" {
        return Ok(DocumentSetExtractablePdfInputs {
            completeness_state,
            extractable_raw_document_ids: Vec::new(),
        });
    }

    let extractable_raw_document_ids = serde_json::from_str::<Vec<Value>>(&documents_json)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|entry| {
            entry
                .get("raw_document_id")
                .or_else(|| entry.get("id"))
                .and_then(Value::as_str)
                .map(str::to_owned)
        })
        .filter_map(|raw_document_id| {
            load_extractable_pdf_storage_path(conn, &raw_document_id)
                .transpose()
                .map(|maybe_path| maybe_path.map(|_| raw_document_id))
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(DocumentSetExtractablePdfInputs {
        completeness_state,
        extractable_raw_document_ids,
    })
}

fn load_extractable_pdf_storage_path(
    conn: &Connection,
    raw_document_id: &str,
) -> Result<Option<PathBuf>> {
    let row: Option<(String, Option<String>)> = conn
        .query_row(
            "SELECT content_type, storage_path FROM raw_documents WHERE id = ?1",
            params![raw_document_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;

    let Some((content_type, storage_path)) = row else {
        return Ok(None);
    };
    let Some(storage_path) = storage_path else {
        return Ok(None);
    };
    if !is_extractable_content_type(&content_type) {
        return Ok(None);
    }

    let path = PathBuf::from(storage_path);
    if path.is_file() {
        Ok(Some(path))
    } else {
        Ok(None)
    }
}

fn assess_structural_freshness(
    conn: &Connection,
    raw_document_id: &str,
    document_extraction_id: &str,
) -> Result<StructuralFreshnessAssessment> {
    let (source_content_hash, extraction_engine_version): (String, String) = conn.query_row(
        "\
        SELECT source_content_hash, extraction_engine_version
        FROM document_extractions
        WHERE id = ?1
        ",
        params![document_extraction_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;

    let mut hard_invalid_statement = conn.prepare(
        "\
        SELECT id
        FROM document_extractions
        WHERE raw_document_id = ?1
          AND superseded_by_extraction_id = ?2
          AND source_content_hash <> ?3
        ORDER BY created_at, id
        ",
    )?;
    let hard_invalidated_extraction_ids = hard_invalid_statement
        .query_map(
            params![raw_document_id, document_extraction_id, source_content_hash],
            |row| row.get::<_, String>(0),
        )?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    if !hard_invalidated_extraction_ids.is_empty() {
        return Ok(StructuralFreshnessAssessment {
            freshness_state: "soft_stale".to_owned(),
            freshness_reason: Some("content_hash_change".to_owned()),
            hard_invalidated_extraction_ids,
        });
    }

    let engine_upgrade_detected: Option<String> = conn
        .query_row(
            "\
            SELECT id
            FROM document_extractions
            WHERE raw_document_id = ?1
              AND id <> ?2
              AND is_current = 1
              AND source_content_hash = ?3
              AND extraction_engine_version <> ?4
            LIMIT 1
            ",
            params![
                raw_document_id,
                document_extraction_id,
                source_content_hash,
                extraction_engine_version,
            ],
            |row| row.get(0),
        )
        .optional()?;

    if engine_upgrade_detected.is_some() {
        return Ok(StructuralFreshnessAssessment {
            freshness_state: "soft_stale".to_owned(),
            freshness_reason: Some("engine_upgrade".to_owned()),
            hard_invalidated_extraction_ids,
        });
    }

    Ok(StructuralFreshnessAssessment {
        freshness_state: "fresh".to_owned(),
        freshness_reason: None,
        hard_invalidated_extraction_ids,
    })
}

#[allow(clippy::too_many_lines)]
fn persist_semantic_domain_writes(
    tx: &Transaction<'_>,
    context: &SemanticDomainWriteContext<'_>,
    result_json: &Value,
) -> Result<Vec<Value>> {
    let Some(notice_id) = context.notice_id else {
        return Ok(Vec::new());
    };

    match context.skill_id {
        "pdf-extraction/tender-specification" => {
            persist_tender_spec_domain_writes(tx, context, notice_id, result_json)
        }
        "pdf-extraction/pricing-schedule" => {
            persist_pricing_overlay_domain_writes(tx, context, notice_id, result_json)
        }
        "pdf-extraction/bill-of-quantities" => {
            persist_quantity_overlay_domain_writes(tx, context, notice_id, result_json)
        }
        _ => Ok(Vec::new()),
    }
}

#[allow(clippy::too_many_lines)]
fn persist_tender_spec_domain_writes(
    tx: &Transaction<'_>,
    context: &SemanticDomainWriteContext<'_>,
    notice_id: &str,
    result_json: &Value,
) -> Result<Vec<Value>> {
    let mut refs = Vec::new();
    let lots = required_array_at(result_json, &["lots"], "lots")?;

    // Gate product-candidate creation on the notice's qualification result.
    // When qualify has rejected the notice (services, works, DPS, expired),
    // skip enumerating product_candidates from the tender-spec seeds — lot
    // metadata still persists for audit, but no downstream supplier enrichment
    // budget is spent on candidates we've already decided not to bid on.
    // Unknown state (qualify hasn't run yet) defaults to ALLOW so we preserve
    // historical behavior until the pipeline is reordered.
    let notice_rejected_by_qualify = tx
        .query_row(
            "SELECT goods_qualified FROM qualification_runs \
             WHERE notice_id = ?1 ORDER BY created_at DESC LIMIT 1",
            params![notice_id],
            |row| row.get::<_, i32>(0),
        )
        .optional()?
        .is_some_and(|goods_q| goods_q == 0);

    for lot in lots {
        let lot_number_text =
            required_string_at(lot, &["lot_number"], "lots[].lot_number")?.to_owned();
        let lot_id = resolve_notice_lot_id(tx, notice_id, &lot_number_text)?;
        let title = optional_string_at(lot, &["title"]).unwrap_or_default();
        let description = optional_string_at(lot, &["description"]);
        let documents_json = json!([build_document_derived_evidence(
            "pdf-extraction/tender-specification",
            context.document_extraction_id,
            lot.get("evidence_anchor").cloned(),
        )])
        .to_string();

        tx.execute(
            "\
            INSERT INTO notice_lots (
              id, notice_id, source_lot_id, lot_key, lot_number, title, description, documents,
              owner_id, row_version, last_local_seq, stage_run_id
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 1, ?10, ?11)
            ON CONFLICT(id) DO UPDATE SET
              title = excluded.title,
              description = excluded.description,
              documents = excluded.documents,
              row_version = notice_lots.row_version + 1,
              last_local_seq = excluded.last_local_seq,
              stage_run_id = excluded.stage_run_id,
              updated_at = CURRENT_TIMESTAMP
            ",
            params![
                lot_id,
                notice_id,
                lot_number_text,
                lot_number_text,
                parse_optional_lot_number(&lot_number_text),
                title,
                description,
                documents_json,
                DEFAULT_OWNER_ID,
                context.visible_after_seq,
                context.stage_run_id,
            ],
        )?;
        refs.push(json!({"table": "notice_lots", "id": lot_id.clone()}));

        if notice_rejected_by_qualify {
            continue;
        }

        for seed in lot_product_requirement_seeds(result_json, &lot_number_text) {
            let candidate_type = seed_candidate_type(seed);

            // Skip service-shaped seeds that the PDF extraction flagged as
            // non-product. The qualify stage already rejects pure-services
            // notices; this handles mixed lots where a few seeds per lot are
            // services even though the notice itself cleared qualify.
            if candidate_type == "service_not_applicable" {
                continue;
            }

            let candidate_key = seed_key(seed);
            let candidate_id = format!("{lot_id}-candidate-{}", slug_token(&candidate_key));
            let candidate_name = seed_name(seed);
            let head_noun = seed_head_noun(seed, &candidate_name);
            let match_archetype = seed_match_archetype(seed);
            let material = optional_string_at(seed, &["material"]);
            let brand_model = optional_string_at(seed, &["brand_model"]);
            let critical_attributes_json = seed_object_json(seed, "critical_attributes");
            let numeric_specs_json = seed_object_json(seed, "numeric_specs");
            let preferred_terms_json = seed_string_array_json(seed, "preferred_terms");
            let excluded_terms_json = seed_string_array_json(seed, "excluded_terms");
            let identity_confidence_bps = seed_confidence_bps(seed, "identity_confidence_bps", 5000);
            let spec_confidence_bps = seed_confidence_bps(seed, "spec_confidence_bps", 9200);
            let source_evidence_json = json!([build_document_derived_evidence(
                "pdf-extraction/tender-specification",
                context.document_extraction_id,
                seed.get("evidence_anchor").cloned(),
            )])
            .to_string();
            tx.execute(
                "\
                INSERT INTO product_candidates (
                  id, process_id, notice_id, lot_id, candidate_key, name, candidate_type, match_archetype,
                  head_noun, material, brand_model,
                  quantity, quantity_unit, specification_text, standards_json,
                  critical_attributes_json, numeric_specs_json,
                  preferred_terms_json, excluded_terms_json,
                  source_evidence_json,
                  identity_confidence_bps, spec_confidence_bps, quantity_confidence_bps,
                  owner_id, row_version, last_local_seq, stage_run_id
                ) VALUES (
                  ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8,
                  ?9, ?10, ?11,
                  ?12, ?13, ?14, ?15,
                  ?16, ?17,
                  ?18, ?19,
                  ?20,
                  ?21, ?22, 9200,
                  ?23, 1, ?24, ?25
                )
                ON CONFLICT(id) DO UPDATE SET
                  name = excluded.name,
                  candidate_type = excluded.candidate_type,
                  match_archetype = excluded.match_archetype,
                  head_noun = excluded.head_noun,
                  material = excluded.material,
                  brand_model = excluded.brand_model,
                  quantity = excluded.quantity,
                  quantity_unit = excluded.quantity_unit,
                  specification_text = excluded.specification_text,
                  standards_json = excluded.standards_json,
                  critical_attributes_json = excluded.critical_attributes_json,
                  numeric_specs_json = excluded.numeric_specs_json,
                  preferred_terms_json = excluded.preferred_terms_json,
                  excluded_terms_json = excluded.excluded_terms_json,
                  source_evidence_json = excluded.source_evidence_json,
                  identity_confidence_bps = excluded.identity_confidence_bps,
                  spec_confidence_bps = excluded.spec_confidence_bps,
                  quantity_confidence_bps = excluded.quantity_confidence_bps,
                  row_version = product_candidates.row_version + 1,
                  last_local_seq = excluded.last_local_seq,
                  stage_run_id = excluded.stage_run_id,
                  updated_at = CURRENT_TIMESTAMP
                ",
                params![
                    candidate_id,
                    context.process_id,
                    notice_id,
                    lot_id,
                    candidate_key,
                    candidate_name,
                    candidate_type,
                    match_archetype,
                    head_noun,
                    material,
                    brand_model,
                    optional_numeric_at(seed, &["quantity"]),
                    optional_string_at(seed, &["quantity_unit"]),
                    seed_specification_text(seed),
                    json!(seed_technical_standards(seed)).to_string(),
                    critical_attributes_json,
                    numeric_specs_json,
                    preferred_terms_json,
                    excluded_terms_json,
                    source_evidence_json,
                    identity_confidence_bps,
                    spec_confidence_bps,
                    DEFAULT_OWNER_ID,
                    context.visible_after_seq,
                    context.stage_run_id,
                ],
            )?;
            refs.push(json!({"table": "product_candidates", "id": candidate_id}));
        }
    }

    Ok(refs)
}

fn persist_pricing_overlay_domain_writes(
    tx: &Transaction<'_>,
    context: &SemanticDomainWriteContext<'_>,
    notice_id: &str,
    result_json: &Value,
) -> Result<Vec<Value>> {
    let Some(line_items) = optional_array_at(result_json, &["line_items"]) else {
        return Ok(Vec::new());
    };

    let mut refs = Vec::new();
    for line_item in line_items {
        let lot_number =
            optional_string_at(line_item, &["lot_number"]).unwrap_or_else(|| "1".to_owned());
        let lot_id = resolve_notice_lot_id(tx, notice_id, &lot_number)?;
        let description = optional_string_at(line_item, &["description"]);
        let Some(candidate) = resolve_candidate_for_lot(tx, &lot_id, description.as_deref())?
        else {
            continue;
        };
        let numeric_specs_json = merge_json_object_text(
            candidate.numeric_specs_json.as_deref(),
            "pricing_overlay",
            json!({
                "item_number": optional_string_at(line_item, &["item_number"]),
                "unit_rate_pence": optional_i64_at(line_item, &["unit_rate_pence"]),
                "total_value_pence": optional_i64_at(line_item, &["total_value_pence"]),
                "currency": optional_string_at(line_item, &["currency"]),
                "source_table_key": optional_string_at(line_item, &["source_table_key"]),
            }),
        )?;
        let source_evidence_json = append_source_evidence(
            candidate.source_evidence_json.as_deref(),
            build_document_derived_evidence(
                "pdf-extraction/pricing-schedule",
                context.document_extraction_id,
                line_item.get("evidence_anchor").cloned(),
            ),
        )?;

        tx.execute(
            "\
            UPDATE product_candidates
            SET quantity = COALESCE(?2, quantity),
                quantity_unit = COALESCE(?3, quantity_unit),
                specification_text = COALESCE(?4, specification_text),
                numeric_specs_json = ?5,
                source_evidence_json = ?6,
                row_version = product_candidates.row_version + 1,
                last_local_seq = ?7,
                stage_run_id = ?8,
                updated_at = CURRENT_TIMESTAMP
            WHERE id = ?1
            ",
            params![
                candidate.id,
                optional_numeric_at(line_item, &["quantity"]),
                optional_string_at(line_item, &["unit"]),
                description,
                numeric_specs_json,
                source_evidence_json,
                context.visible_after_seq,
                context.stage_run_id,
            ],
        )?;
        refs.push(json!({"table": "product_candidates", "id": candidate.id}));
    }
    Ok(refs)
}

fn persist_quantity_overlay_domain_writes(
    tx: &Transaction<'_>,
    context: &SemanticDomainWriteContext<'_>,
    notice_id: &str,
    result_json: &Value,
) -> Result<Vec<Value>> {
    let Some(items) = optional_array_at(result_json, &["items"]) else {
        return Ok(Vec::new());
    };

    let mut refs = Vec::new();
    for item in items {
        let lot_number =
            optional_string_at(item, &["lot_number"]).unwrap_or_else(|| "1".to_owned());
        let lot_id = resolve_notice_lot_id(tx, notice_id, &lot_number)?;
        let description = optional_string_at(item, &["description"]);
        let Some(candidate) = resolve_candidate_for_lot(tx, &lot_id, description.as_deref())?
        else {
            continue;
        };
        let numeric_specs_json = merge_json_object_text(
            candidate.numeric_specs_json.as_deref(),
            "quantity_overlay",
            json!({
                "item_key": optional_string_at(item, &["item_key"]),
                "item_ref": optional_string_at(item, &["item_ref"]),
                "section_ref": optional_string_at(item, &["section_ref"]),
                "unit_rate_pence": optional_i64_at(item, &["unit_rate_pence"]),
                "total_pence": optional_i64_at(item, &["total_pence"]),
            }),
        )?;
        let source_evidence_json = append_source_evidence(
            candidate.source_evidence_json.as_deref(),
            build_document_derived_evidence(
                "pdf-extraction/bill-of-quantities",
                context.document_extraction_id,
                item.get("evidence_anchor").cloned(),
            ),
        )?;
        tx.execute(
            "\
            UPDATE product_candidates
            SET quantity = COALESCE(?2, quantity),
                quantity_unit = COALESCE(?3, quantity_unit),
                specification_text = COALESCE(?4, specification_text),
                numeric_specs_json = ?5,
                source_evidence_json = ?6,
                row_version = product_candidates.row_version + 1,
                last_local_seq = ?7,
                stage_run_id = ?8,
                updated_at = CURRENT_TIMESTAMP
            WHERE id = ?1
            ",
            params![
                candidate.id,
                optional_numeric_at(item, &["quantity"]),
                optional_string_at(item, &["unit"]),
                description,
                numeric_specs_json,
                source_evidence_json,
                context.visible_after_seq,
                context.stage_run_id,
            ],
        )?;
        refs.push(json!({"table": "product_candidates", "id": candidate.id}));
    }
    Ok(refs)
}

fn lot_product_requirement_seeds<'a>(result_json: &'a Value, lot_number: &str) -> Vec<&'a Value> {
    let Some(seeds) = optional_array_at(result_json, &["product_requirement_seeds"]) else {
        return Vec::new();
    };
    seeds
        .iter()
        .filter(|seed| optional_string_at(seed, &["lot_number"]).as_deref() == Some(lot_number))
        .collect()
}

fn resolve_notice_lot_id(
    tx: &Transaction<'_>,
    notice_id: &str,
    lot_number_text: &str,
) -> Result<String> {
    let lot_number = parse_optional_lot_number(lot_number_text);
    let existing: Option<String> = tx
        .query_row(
            "\
            SELECT id
            FROM notice_lots
            WHERE notice_id = ?1
              AND (lot_number = ?2 OR (lot_number IS NULL AND ?2 IS NULL))
            ORDER BY created_at, id
            LIMIT 1
            ",
            params![notice_id, lot_number],
            |row| row.get(0),
        )
        .optional()?;
    Ok(existing.unwrap_or_else(|| format!("{notice_id}-lot-{}", slug_token(lot_number_text))))
}

#[derive(Clone)]
struct CandidateProjection {
    id: String,
    name: String,
    numeric_specs_json: Option<String>,
    source_evidence_json: Option<String>,
}

fn resolve_candidate_for_lot(
    tx: &Transaction<'_>,
    lot_id: &str,
    preferred_name: Option<&str>,
) -> Result<Option<CandidateProjection>> {
    let mut statement = tx.prepare(
        "\
        SELECT id, name, numeric_specs_json, source_evidence_json
        FROM product_candidates
        WHERE lot_id = ?1
        ORDER BY created_at, id
        ",
    )?;
    let candidates = statement
        .query_map(params![lot_id], |row| {
            Ok(CandidateProjection {
                id: row.get(0)?,
                name: row.get(1)?,
                numeric_specs_json: row.get(2)?,
                source_evidence_json: row.get(3)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    if candidates.is_empty() {
        return Ok(None);
    }
    if candidates.len() == 1 {
        return Ok(candidates.into_iter().next());
    }

    let preferred = preferred_name.map(slug_token);
    Ok(candidates
        .iter()
        .find(|candidate| {
            preferred.as_ref().is_some_and(|needle| {
                let haystack = slug_token(&candidate.name);
                haystack.contains(needle) || needle.contains(&haystack)
            })
        })
        .cloned()
        .or_else(|| candidates.into_iter().next()))
}

fn build_document_derived_evidence(
    skill_id: &str,
    document_extraction_id: &str,
    anchor: Option<Value>,
) -> Value {
    json!({
        "evidence_basis": "document_derived",
        "skill_id": skill_id,
        "document_extraction_id": document_extraction_id,
        "anchor": anchor.unwrap_or(Value::Null),
    })
}

fn append_source_evidence(existing_json: Option<&str>, entry: Value) -> Result<String> {
    let mut items = existing_json
        .and_then(|raw| serde_json::from_str::<Vec<Value>>(raw).ok())
        .unwrap_or_default();
    items.push(entry);
    serde_json::to_string(&items).context("serializing source_evidence_json")
}

fn merge_json_object_text(existing_json: Option<&str>, key: &str, value: Value) -> Result<String> {
    let mut object = existing_json
        .and_then(|raw| serde_json::from_str::<Map<String, Value>>(raw).ok())
        .unwrap_or_default();
    object.insert(key.to_owned(), value);
    serde_json::to_string(&Value::Object(object)).context("serializing numeric_specs_json")
}

fn recompute_semantic_extraction_state(
    tx: &Transaction<'_>,
    document_extraction_id: &str,
    expected_overlays: &[String],
) -> Result<String> {
    if expected_overlays.is_empty() {
        return Ok("skipped".to_owned());
    }

    let mut statement = tx.prepare(
        "\
        SELECT skill_id
        FROM document_semantic_extractions
        WHERE document_extraction_id = ?1
          AND is_current = 1
        ",
    )?;
    let completed = statement
        .query_map(params![document_extraction_id], |row| {
            row.get::<_, String>(0)
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    let mut failed_statement = tx.prepare(
        "\
        SELECT DISTINCT json_extract(input_payload, '$.request.request.skill_id')
        FROM workflow_jobs
        WHERE kind = 'extract_document_semantics'
          AND status = 'failed'
          AND json_extract(input_payload, '$.request.request.document_extraction_id') = ?1
        ",
    )?;
    let failed = failed_statement
        .query_map(params![document_extraction_id], |row| {
            row.get::<_, String>(0)
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    if expected_overlays
        .iter()
        .any(|skill_id| failed.iter().any(|failed_id| failed_id == skill_id))
    {
        return Ok("failed".to_owned());
    }
    if expected_overlays
        .iter()
        .all(|skill_id| completed.iter().any(|done| done == skill_id))
    {
        return Ok("complete".to_owned());
    }
    if completed.iter().any(|skill_id| {
        expected_overlays
            .iter()
            .any(|expected| expected == skill_id)
    }) {
        return Ok("partial".to_owned());
    }
    Ok("pending".to_owned())
}

fn required_array_at<'a>(value: &'a Value, path: &[&str], label: &str) -> Result<&'a [Value]> {
    optional_array_at(value, path).ok_or_else(|| anyhow!("missing array `{label}`"))
}

fn optional_array_at<'a>(value: &'a Value, path: &[&str]) -> Option<&'a [Value]> {
    walk_json_path(value, path)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
}

fn required_string_at<'a>(value: &'a Value, path: &[&str], label: &str) -> Result<&'a str> {
    optional_string_ref_at(value, path).ok_or_else(|| anyhow!("missing string `{label}`"))
}

fn optional_string_at(value: &Value, path: &[&str]) -> Option<String> {
    optional_string_ref_at(value, path).map(str::to_owned)
}

fn optional_string_ref_at<'a>(value: &'a Value, path: &[&str]) -> Option<&'a str> {
    walk_json_path(value, path).and_then(Value::as_str)
}

fn required_i64_at(value: &Value, path: &[&str], label: &str) -> Result<i64> {
    walk_json_path(value, path)
        .and_then(Value::as_i64)
        .ok_or_else(|| anyhow!("missing integer `{label}`"))
}

fn optional_numeric_at(value: &Value, path: &[&str]) -> Option<f64> {
    walk_json_path(value, path).and_then(Value::as_f64)
}

fn optional_i64_at(value: &Value, path: &[&str]) -> Option<i64> {
    walk_json_path(value, path).and_then(Value::as_i64)
}

fn optional_value_at<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Value> {
    walk_json_path(value, path)
}

fn walk_json_path<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Value> {
    path.iter()
        .try_fold(value, |current, key| current.get(*key))
}

fn seed_key(seed: &Value) -> String {
    optional_string_at(seed, &["seed_key"])
        .or_else(|| optional_string_at(seed, &["candidate_key"]))
        .unwrap_or_else(|| "seed".to_owned())
}

fn seed_name(seed: &Value) -> String {
    optional_string_at(seed, &["candidate_name"])
        .or_else(|| optional_string_at(seed, &["name"]))
        .unwrap_or_else(|| "Unnamed candidate".to_owned())
}

fn seed_specification_text(seed: &Value) -> Option<String> {
    optional_string_at(seed, &["specification_text"])
        .or_else(|| optional_string_at(seed, &["summary"]))
}

fn seed_technical_standards(seed: &Value) -> Vec<String> {
    optional_array_at(seed, &["technical_standards"])
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

/// Reads the seed's `head_noun` if the PDF-extraction skill set one explicitly,
/// otherwise falls back to the legacy "first whitespace token" heuristic. The
/// fallback keeps old extractions (pre-2026-04-21 enrichment spec) readable
/// until their tenders are reprocessed against the updated skill prompt.
fn seed_head_noun(seed: &Value, candidate_name: &str) -> String {
    optional_string_at(seed, &["head_noun"])
        .map(|s| s.trim().to_ascii_lowercase())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| {
            candidate_name
                .split_whitespace()
                .next()
                .map_or_else(|| "item".to_owned(), str::to_ascii_lowercase)
        })
}

/// Candidate-type enum extracted from the seed. See
/// `skills/pdf-extraction/tender-specification/result.schema.json`. Legacy seeds
/// (which hardcoded `'sku'` on the Rust side) get `standard_commodity` — the
/// neutral open-brand default — instead of the invalid `sku` string.
fn seed_candidate_type(seed: &Value) -> &'static str {
    match optional_string_at(seed, &["candidate_type"]).as_deref() {
        Some("exact_sku") => "exact_sku",
        Some("standard_commodity") => "standard_commodity",
        Some("brand_family") => "brand_family",
        Some("bundle") => "bundle",
        Some("custom_or_fabricated") => "custom_or_fabricated",
        Some("installed_goods") => "installed_goods",
        Some("regulated_goods") => "regulated_goods",
        Some("service_not_applicable") => "service_not_applicable",
        Some("unknown") => "unknown",
        _ => "standard_commodity",
    }
}

/// Match-archetype enum extracted from the seed. Legacy seeds get `unknown`
/// (was `direct_match`, which is not a valid archetype per the research doc).
fn seed_match_archetype(seed: &Value) -> &'static str {
    match optional_string_at(seed, &["match_archetype"]).as_deref() {
        Some("standard_class_led") => "standard_class_led",
        Some("dimension_material_led") => "dimension_material_led",
        Some("electrical_spec_led") => "electrical_spec_led",
        Some("pack_count_led") => "pack_count_led",
        Some("compatibility_led") => "compatibility_led",
        Some("model_exact") => "model_exact",
        _ => "unknown",
    }
}

/// Serialize an optional seed object (e.g. `critical_attributes`, `numeric_specs`)
/// to its JSON string form for persistence. Returns `None` when the seed omits the
/// key or it is not an object — the DB column is nullable.
fn seed_object_json(seed: &Value, key: &str) -> Option<String> {
    seed.get(key)
        .filter(|v| v.is_object() && !v.as_object().is_some_and(serde_json::Map::is_empty))
        .map(ToString::to_string)
}

/// Serialize an optional string-array seed field (e.g. `preferred_terms`,
/// `excluded_terms`) to JSON. `None` when missing or empty.
fn seed_string_array_json(seed: &Value, key: &str) -> Option<String> {
    seed.get(key)
        .and_then(Value::as_array)
        .filter(|arr| !arr.is_empty())
        .map(|arr| {
            let strings: Vec<&str> = arr.iter().filter_map(Value::as_str).collect();
            json!(strings).to_string()
        })
}

/// Read `identity_confidence_bps` / `spec_confidence_bps` from the seed, clamping
/// into the 0-10000 range the schema mandates. Falls back to `default_bps` when
/// the seed omits the field (common for legacy extractions).
fn seed_confidence_bps(seed: &Value, key: &str, default_bps: i64) -> i64 {
    optional_numeric_at(seed, &[key])
        .map(|v| v.round() as i64)
        .unwrap_or(default_bps)
        .clamp(0, 10000)
}

fn slug_token(raw: &str) -> String {
    raw.chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_owned()
}

fn parse_optional_lot_number(raw: &str) -> Option<i64> {
    raw.parse::<i64>().ok()
}

#[allow(clippy::too_many_lines)]
#[allow(clippy::too_many_arguments)]
fn finalize_success(
    runtime: &mut RuntimeBootstrap,
    job_id: &str,
    pipeline: &PipelinePersistence,
    manifest: &ManifestComputation,
    manifest_path: &Path,
    notices_ingested: usize,
    live_validation: &LiveValidationExecution,
    skill_telemetry: &[SkillRunTelemetry],
) -> Result<LiveLoopReport> {
    let review_required = pipeline.review_required
        || live_validation.process_validation_state != ProcessValidationState::Validated;
    let review_reason = merge_review_reason(
        pipeline.review_reason.as_deref(),
        live_validation.validation_marker,
    );
    let review_reason_for_summary = review_reason.clone();
    let validation_verdict_id = live_validation.validation_verdict_id.clone();
    let validator_job_id = live_validation.validator_job_id.clone();
    let skill_telemetry_json =
        serde_json::to_value(skill_telemetry).context("serializing skill telemetry")?;
    let validation_telemetry_json = serde_json::to_value(&live_validation.validation_telemetry)
        .context("serializing validation telemetry")?;
    let summary_payload = json!({
        "job_id": job_id,
        "source_id": "fts",
        "process_id": pipeline.process_id,
        "notices_ingested": notices_ingested,
        "candidate_count": manifest.candidate_count,
        "manifest_path": manifest_path.to_string_lossy(),
        "review_required": review_required,
        "review_reason": review_reason_for_summary,
        "validation_required": live_validation.validation_required,
        "trust_state": live_validation.trust_state.as_str(),
        "validation_state": live_validation.process_validation_state.as_str(),
        "validation_verdict_id": validation_verdict_id,
        "validator_job_id": validator_job_id,
        "validation_marker": live_validation.validation_marker,
        "validation_telemetry": validation_telemetry_json,
        "skill_telemetry": skill_telemetry_json,
        "durable_step_surfaces": live_loop_surfaces_json(),
    });

    let (visible_after_seq, _) = runtime.state_db.with_write_transaction(|tx| {
        let committed_seq = next_commit_seq(tx)?;
        tx.execute(
            "\
            INSERT INTO manifests (
              id, process_id, markdown, artifact_hash, artifact_locator, owner_id,
              last_local_seq, stage_run_id
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            ",
            params![
                format!("manifest-{}", pipeline.process_id),
                pipeline.process_id,
                manifest.markdown,
                sha256_hex(manifest.markdown.as_bytes()),
                manifest_path.to_string_lossy(),
                DEFAULT_OWNER_ID,
                committed_seq,
                pipeline.manifest_stage_run_id,
            ],
        )?;
        tx.execute(
            "\
            INSERT INTO pipeline_assessments (
              process_id, pipeline_state, review_required, validation_state, gross_margin_bps,
              net_margin_bps, review_stage, review_reason, decision_basis_hash, owner_id,
              last_local_seq, stage_run_id
            ) VALUES (?1, 'manifest_ready', ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
            ON CONFLICT(process_id) DO UPDATE SET
              pipeline_state = excluded.pipeline_state,
              review_required = excluded.review_required,
              validation_state = excluded.validation_state,
              gross_margin_bps = excluded.gross_margin_bps,
              net_margin_bps = excluded.net_margin_bps,
              review_stage = excluded.review_stage,
              review_reason = excluded.review_reason,
              decision_basis_hash = excluded.decision_basis_hash,
              owner_id = excluded.owner_id,
              last_local_seq = excluded.last_local_seq,
              stage_run_id = excluded.stage_run_id,
              updated_at = CURRENT_TIMESTAMP
            ",
            params![
                pipeline.process_id,
                i64::from(review_required),
                live_validation.process_validation_state.as_str(),
                pipeline.gross_margin_bps,
                pipeline.net_margin_bps,
                if review_required {
                    Some("manifest".to_owned())
                } else {
                    None::<String>
                },
                review_reason,
                manifest.decision_basis_hash,
                DEFAULT_OWNER_ID,
                committed_seq,
                pipeline.manifest_stage_run_id,
            ],
        )?;

        persist_stage_validation(
            tx,
            &StageValidationWrite {
                process_id: &pipeline.process_id,
                stage: "Manifest",
                scope_kind: "process",
                scope_id: &pipeline.process_id,
                stage_run_id: &pipeline.manifest_stage_run_id,
                owner_id: DEFAULT_OWNER_ID,
            },
            &ValidationOutcome {
                disposition: ValidationDisposition::Accepted,
                error_code: None,
                error_context_json: None,
                anomalies: Vec::new(),
            },
        )?;

        tx.execute(
            "\
            UPDATE workflow_jobs
            SET status = 'completed',
                completed_at = ?1,
                output_payload = ?2
            WHERE id = ?3
            ",
            params![Utc::now().to_rfc3339(), summary_payload.to_string(), job_id],
        )?;
        Ok(committed_seq)
    })?;

    Ok(LiveLoopReport {
        summary_line: format!(
            "FTS live loop completed: job_id={} notices={} process_id={} manifest={} commit_seq={}",
            job_id,
            notices_ingested,
            pipeline.process_id,
            manifest_path.display(),
            visible_after_seq,
        ),
        status: "completed".to_owned(),
    })
}

#[allow(clippy::too_many_arguments)]
fn fail_with_manifest(
    runtime: &mut RuntimeBootstrap,
    job_id: &str,
    error_code: ErrorCode,
    message: &str,
    process_id: Option<&str>,
    session_id: Option<&str>,
    skill_path: Option<&Path>,
    pipeline_stage: &str,
    workflow_step: &str,
    error_context: Value,
) -> Result<LiveLoopReport> {
    let _ = create_error_manifest(
        runtime,
        error_code,
        job_id,
        process_id,
        session_id,
        skill_path,
        pipeline_stage,
        workflow_step,
        message,
        error_context,
    )?;
    fail_job(
        runtime,
        job_id,
        message,
        json!({"error_code": error_code.as_str()}),
    )
}

#[allow(clippy::too_many_arguments)]
fn fail_job_with_code(
    runtime: &mut RuntimeBootstrap,
    job_id: &str,
    message: &str,
    error_code: ErrorCode,
    process_id: Option<&str>,
    session_id: Option<&str>,
    pipeline_stage: &str,
    workflow_step: &str,
) -> Result<LiveLoopReport> {
    let _ = create_error_manifest(
        runtime,
        error_code,
        job_id,
        process_id,
        session_id,
        None,
        pipeline_stage,
        workflow_step,
        message,
        json!({"error_code": error_code.as_str()}),
    )?;
    fail_job(
        runtime,
        job_id,
        message,
        json!({"error_code": error_code.as_str()}),
    )
}

#[allow(clippy::needless_pass_by_value)]
fn fail_job(
    runtime: &mut RuntimeBootstrap,
    job_id: &str,
    message: &str,
    output_payload: Value,
) -> Result<LiveLoopReport> {
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
                message,
                Utc::now().to_rfc3339(),
                output_payload.to_string(),
                job_id
            ],
        )?;
        Ok(())
    })?;

    Ok(LiveLoopReport {
        summary_line: format!("FTS live loop failed: job_id={job_id} reason={message}"),
        status: "failed".to_owned(),
    })
}

fn complete_empty_run(runtime: &mut RuntimeBootstrap, job_id: &str) -> Result<LiveLoopReport> {
    runtime.state_db.with_write_transaction(|tx| {
        tx.execute(
            "\
            UPDATE workflow_jobs
            SET status = 'completed',
                completed_at = ?1,
                output_payload = ?2
            WHERE id = ?3
            ",
            params![
                Utc::now().to_rfc3339(),
                json!({
                    "job_id": job_id,
                    "source_id": "fts",
                    "notices_ingested": 0,
                    "manifest_path": Value::Null,
                    "durable_step_surfaces": live_loop_surfaces_json(),
                })
                .to_string(),
                job_id,
            ],
        )?;
        Ok(())
    })?;

    Ok(LiveLoopReport {
        summary_line: format!(
            "FTS live loop completed: job_id={} notices=0 process_id=none manifest=none commit_seq={}",
            job_id,
            runtime.state_db.current_commit_seq()?,
        ),
        status: "completed".to_owned(),
    })
}

#[allow(clippy::too_many_arguments)]
fn create_error_manifest(
    runtime: &RuntimeBootstrap,
    error_code: ErrorCode,
    job_id: &str,
    process_id: Option<&str>,
    session_id: Option<&str>,
    skill_path: Option<&Path>,
    pipeline_stage: &str,
    workflow_step: &str,
    error_message: &str,
    error_context: Value,
) -> Result<String> {
    let store = ErrorManifestStore::new(&runtime.artifact_root)?;
    let loaded_session_context = session_id
        .map(|value| {
            load_session_error_context(
                runtime.state_db.connection(),
                &runtime.artifact_root,
                value,
                &runtime.descriptor.instance_id,
                runtime.state_db.current_commit_seq()?,
            )
        })
        .transpose()?
        .flatten();
    let (runtime_context, session_artifact_path) = match loaded_session_context {
        Some(context) => context,
        None => (
            ErrorRuntimeContext {
                cli_vendor: Some(CLI_VENDOR.to_owned()),
                cli_version: Some(CLI_VERSION.to_owned()),
                adapter_version: Some(env!("CARGO_PKG_VERSION").to_owned()),
                model_id: None,
                config_hash: None,
                skill_version: None,
                pid: Some(process::id()),
                pgid: Some(current_runtime_ids()?.pgid),
                cleanup_verdict: Some("clean".to_owned()),
                last_valid_checkpoint_sequence: Some(1),
                daemon_instance_id: Some(runtime.descriptor.instance_id.clone()),
                commit_seq: Some(runtime.state_db.current_commit_seq()?),
            },
            None,
        ),
    };
    let stored =
        store.create(
            runtime.state_db.connection(),
            &ErrorManifestDraft {
                job_id: Some(job_id.to_owned()),
                process_id: process_id.map(str::to_owned),
                pipeline_stage: pipeline_stage.to_owned(),
                workflow_step: workflow_step.to_owned(),
                error_type: error_code,
                error_message: error_message.to_owned(),
                error_context,
                skill_path: skill_path.map(|path| path.to_string_lossy().into_owned()),
                agent_session_id: session_id.map(str::to_owned),
                runtime_context,
                session_artifact_path,
                next_debugging_action:
                    "Inspect the linked artifacts and rerun after correcting the root cause."
                        .to_owned(),
            },
            Utc::now(),
        )?;
    Ok(stored.id)
}

fn load_session_error_context(
    conn: &Connection,
    artifact_root: &Path,
    session_id: &str,
    daemon_instance_id: &str,
    commit_seq: i64,
) -> Result<Option<(ErrorRuntimeContext, Option<String>)>> {
    let row = conn
        .query_row(
            "\
            SELECT cli_vendor, cli_version, adapter_version, config_hash, skill_version,
                   pid, pgid, cleanup_verdict, last_checkpoint_seq, started_at
            FROM agent_session_logs WHERE id = ?1
            ",
            params![session_id],
            |row| {
                Ok((
                    row.get::<_, Option<String>>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, Option<u32>>(5)?,
                    row.get::<_, Option<u32>>(6)?,
                    row.get::<_, Option<String>>(7)?,
                    row.get::<_, Option<u32>>(8)?,
                    row.get::<_, String>(9)?,
                ))
            },
        )
        .optional()?;
    let Some((
        cli_vendor,
        cli_version,
        adapter_version,
        config_hash,
        skill_version,
        process_id,
        process_group_id,
        cleanup_verdict,
        last_checkpoint_seq,
        started_at,
    )) = row
    else {
        return Ok(None);
    };

    let started_at = parse_timestamp(&started_at)?;
    let session_path = ArtifactStore::new(artifact_root)?
        .session_dir(session_id, started_at)?
        .to_string_lossy()
        .into_owned();
    Ok(Some((
        ErrorRuntimeContext {
            cli_vendor,
            cli_version,
            adapter_version,
            model_id: None,
            config_hash,
            skill_version,
            pid: process_id,
            pgid: process_group_id,
            cleanup_verdict,
            last_valid_checkpoint_sequence: last_checkpoint_seq,
            daemon_instance_id: Some(daemon_instance_id.to_owned()),
            commit_seq: Some(commit_seq),
        },
        Some(session_path),
    )))
}

fn persist_failed_set_replacement_stage(
    state_db: &mut StateDb,
    process_id: &str,
    stage_name: &str,
    error_manifest_id: &str,
) -> Result<()> {
    state_db.with_write_transaction(|tx| {
        agents::contracts::persist_stage_validation(
            tx,
            &StageValidationWrite {
                process_id,
                stage: stage_name,
                scope_kind: "notice",
                scope_id: "failed-scope",
                stage_run_id: &allocate_stage_run_id(),
                owner_id: DEFAULT_OWNER_ID,
            },
            &ValidationOutcome {
                disposition: ValidationDisposition::Failed,
                error_code: Some("SQLITE_WRITE_FAILED"),
                error_context_json: Some(
                    json!({"error_manifest_id": error_manifest_id}).to_string(),
                ),
                anomalies: Vec::new(),
            },
        )?;
        Ok(())
    })?;
    Ok(())
}

fn build_manifest_from_db(conn: &Connection, process_id: &str) -> Result<ManifestComputation> {
    let title: String = conn.query_row(
        "SELECT title FROM procurement_processes WHERE id = ?1",
        params![process_id],
        |row| row.get(0),
    )?;
    let candidate_count: u32 = conn.query_row(
        "SELECT COUNT(*) FROM product_candidates WHERE process_id = ?1",
        params![process_id],
        |row| row.get(0),
    )?;
    let markdown = format!(
        "# Opportunity Manifest\n\n- Process: {process_id}\n- Title: {title}\n- Candidate count: {candidate_count}\n"
    );
    let decision_basis_hash = sha256_hex(markdown.as_bytes());
    Ok(ManifestComputation {
        markdown,
        decision_basis_hash,
        candidate_count,
    })
}

fn write_manifest_result_artifact(
    artifact_root: &Path,
    session_id: &str,
    started_at: DateTime<Utc>,
    payload: &Value,
) -> Result<PathBuf> {
    let session_dir = ArtifactStore::new(artifact_root)?.session_dir(session_id, started_at)?;
    let result_path = session_dir.join("result.json");
    fs::write(&result_path, serde_json::to_vec_pretty(payload)?)
        .with_context(|| format!("writing {}", result_path.display()))?;
    Ok(result_path)
}

fn write_opportunity_manifest(
    artifact_root: &Path,
    process_id: &str,
    markdown: &str,
) -> Result<PathBuf> {
    let dir = artifact_root.join("opportunity-manifests");
    fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
    let path = dir.join(format!("{process_id}.md"));
    fs::write(&path, markdown).with_context(|| format!("writing {}", path.display()))?;
    Ok(path)
}

#[allow(clippy::too_many_arguments)]
fn write_run_manifest_for_session(
    runtime: &RuntimeBootstrap,
    job_id: &str,
    session_id: &str,
    skill: &LoadedSkill,
    execution_mode: &str,
    started_at: DateTime<Utc>,
    fixture: &LiveLoopFixture,
    items_processed: u32,
    agent_narrative: Option<RunNarrative>,
    artifact_refs: Vec<String>,
) -> Result<agents::artifacts::StoredManifest> {
    let frontmatter = RunManifestFrontmatter {
        manifest_version: 1,
        skill_id: skill.metadata.skill_id.clone(),
        skill_version: skill.metadata.skill_version.clone(),
        job_id: job_id.to_owned(),
        session_id: session_id.to_owned(),
        execution_mode: execution_mode.to_owned(),
        started_at: started_at.to_rfc3339(),
        completed_at: Utc::now().to_rfc3339(),
        outcome_class: if fixture.continuation_required {
            OutcomeClass::ContinuationRequired
        } else {
            OutcomeClass::Completed
        },
        items_processed,
        items_skipped: 0,
        items_failed: 0,
        warnings_count: u32::from(
            fixture.goods_classification_low_confidence
                || fixture.supplier_match_low_confidence
                || fixture.commodity_code_low_confidence
                || fixture.revenue_basis_uncertain
                || fixture.fpo_access_unavailable,
        ),
        agent_claimed_pages_visited: 1,
        agent_claimed_source_url: fixture.notices.first().map_or_else(
            || "https://www.find-tender.service.gov.uk".to_owned(),
            |notice| notice.source_url.clone(),
        ),
        agent_claimed_fingerprint_state: fixture.fingerprint_state.clone(),
        agent_claimed_locale_state: fixture.locale_state.clone(),
        agent_claimed_currency_state: fixture.currency_state.clone(),
        agent_narrative_missing: agent_narrative.is_none(),
    };
    let appendix = DaemonEvidenceAppendix {
        observed_url_chain: fixture
            .notices
            .iter()
            .map(|notice| notice.source_url.clone())
            .collect(),
        observed_page_kind_sequence: vec!["fts".to_owned(), "notice_list".to_owned()],
        daemon_observed_fingerprint_state: fixture.fingerprint_state.clone(),
        daemon_observed_locale: fixture.locale_state.clone(),
        daemon_observed_currency: fixture.currency_state.clone(),
        auth_challenge_events: Vec::new(),
        locator_fallback_depth: 0,
        basis_sufficiency: "high".to_owned(),
        page_fingerprint_hash: Some(sha256_hex(fixture.fingerprint_state.as_bytes())),
        auth_challenge_detected: false,
        extraction_fill_rate_bps: Some(u32::from(fixture.extraction_fill_rate_bps)),
        checkpoint_count: 1,
        byte_count_compressed: None,
        validator_rubric_hash: None,
        drift_delta_json: Value::Null,
        result_item_count: items_processed,
        oqg_anomaly_summary: Vec::new(),
        representative_item_sample: json!({
            "notice_id": fixture.notices.first().map(|notice| notice.notice_id.clone()),
            "title": fixture.notices.first().map(|notice| notice.title.clone()),
        }),
        artifact_refs,
    };

    write_hybrid_run_manifest(ManifestWriteRequest {
        artifact_root: &runtime.artifact_root,
        started_at,
        frontmatter,
        narrative: agent_narrative,
        appendix,
        disagreements: Vec::new(),
        missing_narrative_reason: "agent narrative missing from live session output",
    })
}

fn insert_session_log(conn: &Connection, draft: &SessionDraft) -> Result<()> {
    conn.execute(
        "\
        INSERT INTO agent_session_logs (
          id, job_id, skill_path, skill_version, execution_mode, cli_vendor, cli_version,
          adapter_id, adapter_version, config_hash, pty_session_id, sid, pid, pgid, started_at,
          completed_at, status, terminal_output_path, terminal_output_hash,
          terminal_output_bytes, terminal_output_tail, structured_trace, error_type,
          error_message, exit_code, cleanup_verdict, last_checkpoint_seq
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15,
                  ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27)
        ON CONFLICT(id) DO UPDATE SET
          completed_at = excluded.completed_at,
          status = excluded.status,
          terminal_output_path = excluded.terminal_output_path,
          terminal_output_hash = excluded.terminal_output_hash,
          terminal_output_bytes = excluded.terminal_output_bytes,
          terminal_output_tail = excluded.terminal_output_tail,
          structured_trace = excluded.structured_trace,
          error_type = excluded.error_type,
          error_message = excluded.error_message,
          exit_code = excluded.exit_code,
          cleanup_verdict = excluded.cleanup_verdict,
          last_checkpoint_seq = excluded.last_checkpoint_seq
        ",
        params![
            draft.session_id,
            draft.job_id,
            draft.skill_path,
            draft.skill_version,
            draft.execution_mode,
            draft.cli_vendor,
            draft.cli_version,
            draft.adapter_id,
            draft.adapter_version,
            draft.config_hash,
            draft.pty_session_id,
            draft.sid,
            draft.pid,
            draft.pgid,
            draft.started_at,
            draft.completed_at,
            draft.status,
            draft.terminal_output_path,
            draft.terminal_output_hash,
            draft.terminal_output_bytes,
            draft.terminal_output_tail,
            draft.structured_trace,
            draft.error_type,
            draft.error_message,
            draft.exit_code,
            draft.cleanup_verdict,
            draft.last_checkpoint_seq,
        ],
    )
    .context("inserting agent_session_logs row")?;
    Ok(())
}

#[allow(dead_code)]
fn capture_transcript(
    artifact_root: &Path,
    session_id: &str,
    now: DateTime<Utc>,
    transcript_lines: &[String],
) -> Result<CaptureSummary> {
    let mut capture = PtyCapture::new(artifact_root, session_id, now, CaptureConfig::default())?;
    let lines = if transcript_lines.is_empty() {
        default_transcript_lines()
    } else {
        transcript_lines.to_vec()
    };

    for (seq, line) in (1_u64..).zip(lines.iter()) {
        let mut bytes = line.clone().into_bytes();
        bytes.push(b'\n');
        capture.append(seq, &bytes, now)?;
    }

    let entries = capture.finalize()?;
    let first_entry = entries
        .first()
        .ok_or_else(|| anyhow!("capture did not emit any stream index entries"))?;
    let first_chunk_path = capture.paths().session_root.join(&first_entry.chunk);
    let total_bytes = entries.iter().try_fold(0_i64, |acc, entry| {
        i64::try_from(entry.byte_length)
            .map(|value| acc + value)
            .map_err(|_| anyhow!("stream chunk length exceeded i64 range"))
    })?;

    Ok(CaptureSummary {
        index_path: capture.paths().index_path.clone(),
        first_chunk_hash: compressed_payload_sha256(&first_chunk_path)?,
        total_bytes,
        tail: lines.last().cloned().unwrap_or_default(),
        artifact_refs: entries.iter().map(|entry| entry.chunk.clone()).collect(),
    })
}

fn load_skill(skill_root: &Path, relative_skill_path: &str) -> Result<LoadedSkill> {
    let skill_markdown = fs::read_to_string(skill_root.join("SKILL.md"))
        .with_context(|| format!("reading {}", skill_root.join("SKILL.md").display()))?;
    let (frontmatter, _) = split_frontmatter(&skill_markdown)?;
    let result_schema_path = skill_root.join("result.schema.json");
    let result_schema = serde_json::from_str(
        &fs::read_to_string(&result_schema_path)
            .with_context(|| format!("reading {}", result_schema_path.display()))?,
    )
    .with_context(|| format!("parsing {}", result_schema_path.display()))?;

    Ok(LoadedSkill {
        skill_path: PathBuf::from(relative_skill_path),
        skill_markdown,
        metadata: frontmatter,
        result_schema,
    })
}

fn split_frontmatter(markdown: &str) -> Result<(SkillFrontmatter, &str)> {
    let mut sections = markdown.splitn(3, "---");
    let leading = sections
        .next()
        .ok_or_else(|| anyhow!("SKILL.md missing YAML frontmatter"))?;
    ensure!(
        leading.trim().is_empty(),
        "SKILL.md must start with YAML frontmatter"
    );
    let yaml = sections
        .next()
        .ok_or_else(|| anyhow!("SKILL.md missing YAML frontmatter body"))?;
    let body = sections
        .next()
        .ok_or_else(|| anyhow!("SKILL.md missing markdown body"))?;
    let frontmatter: SkillFrontmatter =
        serde_yaml::from_str(yaml).context("parsing skill frontmatter")?;
    Ok((frontmatter, body))
}

fn live_loop_surfaces_json() -> Value {
    Value::Array(
        LIVE_LOOP_STEP_SURFACES
            .iter()
            .map(|surface| {
                json!({
                    "step": surface.step,
                    "sqliteWrites": surface.sqlite_writes,
                    "artifactWrites": surface.artifact_writes,
                })
            })
            .collect(),
    )
}

fn seed_cursor_before(state_db: &mut StateDb, cursor_before: &str) -> Result<()> {
    state_db.with_write_transaction(|tx| {
        tx.execute(
            "\
            INSERT INTO source_cursors (source_id, cursor_key, cursor_value, updated_at)
            VALUES ('fts', 'notice_cursor', ?1, CURRENT_TIMESTAMP)
            ON CONFLICT(source_id, cursor_key) DO UPDATE SET
              cursor_value = excluded.cursor_value,
              updated_at = excluded.updated_at
            ",
            params![json!({"cursor_before": cursor_before}).to_string()],
        )?;
        Ok(())
    })?;
    Ok(())
}

fn insert_raw_document(
    tx: &Transaction<'_>,
    notice: &FixtureNotice,
    last_local_seq: i64,
) -> Result<()> {
    tx.execute(
        "\
        INSERT INTO raw_documents (
          id, source_id, content_type, source_url, payload_json, metadata, created_at
        ) VALUES (?1, 'fts', 'application/json', ?2, ?3, ?4, CURRENT_TIMESTAMP)
        ",
        params![
            format!("raw-{}", notice.notice_id),
            notice.source_url,
            json!({
                "notice_id": notice.notice_id,
                "title": notice.title,
                "published_at": notice.published_at,
                "source_updated_at": notice.source_updated_at,
            })
            .to_string(),
            json!({"last_local_seq": last_local_seq}).to_string(),
        ],
    )?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Stage 9 dispatcher: supplier-price-enrichment
// Pulls auto-search-eligible search_profiles from the DB, batches them into
// one job_parameters payload, and invokes the per-source enrichment skill via
// the existing PTY path. The full SearchProfile JSON (including head_noun,
// candidate_type, match_archetype, critical_attributes, numeric_specs,
// standards, noun_aliases, query_waves, recall_risk) travels unchanged into
// the rendered prompt — see `PromptRenderer::render` which stringifies
// `job_parameters` into the SKILL.md preamble verbatim.
// ---------------------------------------------------------------------------

/// Returns the repo-relative skill path for the named source. Every source the
/// dispatcher can target is registered here — add a match arm alongside a new
/// `skills/supplier-price-enrichment/<source>/` directory to enable a source.
fn enrichment_skill_path_for(source_id: &str) -> Result<PathBuf> {
    match source_id {
        "alibaba" => Ok(PathBuf::from("skills/supplier-price-enrichment/alibaba")),
        "1688" => Ok(PathBuf::from("skills/supplier-price-enrichment/1688")),
        "ebay" => Ok(PathBuf::from("skills/supplier-price-enrichment/ebay")),
        "amazon" => Ok(PathBuf::from("skills/supplier-price-enrichment/amazon")),
        other => bail!(
            "no supplier-price-enrichment skill registered for source '{other}' \
             — add one under skills/supplier-price-enrichment/ and register it \
             in enrichment_skill_path_for()"
        ),
    }
}

#[allow(clippy::future_not_send)]
async fn run_enrich_pricing(
    runtime: &mut RuntimeBootstrap,
    source_id: &str,
    limit: usize,
    cli: &Cli,
) -> Result<LiveLoopReport> {
    if limit == 0 {
        bail!("--enrich-pricing-limit must be >= 1");
    }

    let skill_path = enrichment_skill_path_for(source_id)?;

    // Pull auto_search-eligible profiles that have no supplier_quotes yet.
    // Select the full SearchProfile row shape so we can pass the rich context
    // straight through to the agent without a lossy projection.
    #[allow(clippy::type_complexity)]
    let profile_rows: Vec<(
        String,
        String,
        String,
        String,
        String,
        String,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<i64>,
        String,
        String,
    )> = {
        let conn = runtime.state_db.connection();
        let mut stmt = conn.prepare(
            "SELECT sp.id, sp.product_candidate_id,
                    pc.name, sp.head_noun, sp.candidate_type, sp.match_archetype,
                    sp.critical_attributes_json, sp.numeric_specs_json,
                    sp.standards_json, sp.noun_aliases_en,
                    sp.preferred_terms, sp.excluded_terms,
                    sp.query_waves_json, sp.recall_risk_score_bps,
                    sp.search_eligibility_state, pc.notice_id
             FROM search_profiles sp
             JOIN product_candidates pc ON pc.id = sp.product_candidate_id
             WHERE sp.search_eligibility_state = 'auto_search'
               AND NOT EXISTS (
                 SELECT 1 FROM supplier_quotes sq
                 WHERE sq.product_candidate_id = sp.product_candidate_id
                   AND sq.source_name = ?1
               )
             ORDER BY sp.recall_risk_score_bps ASC, sp.created_at ASC
             LIMIT ?2",
        )?;
        let source_name = source_name_for_source_id(source_id);
        stmt.query_map(params![source_name, limit as i64], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, Option<String>>(6)?,
                row.get::<_, Option<String>>(7)?,
                row.get::<_, Option<String>>(8)?,
                row.get::<_, Option<String>>(9)?,
                row.get::<_, Option<String>>(10)?,
                row.get::<_, Option<String>>(11)?,
                row.get::<_, Option<String>>(12)?,
                row.get::<_, Option<i64>>(13)?,
                row.get::<_, String>(14)?,
                row.get::<_, String>(15)?,
            ))
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?
    };

    if profile_rows.is_empty() {
        return Ok(LiveLoopReport {
            summary_line: format!(
                "no search_profiles eligible for supplier-price-enrichment on source='{source_id}' \
                 (all quoted already, or none scored high enough for auto_search)"
            ),
            status: "completed".to_owned(),
        });
    }

    let profiles_count = profile_rows.len();
    let mut profiles = Vec::with_capacity(profiles_count);
    let mut notice_ids: Vec<String> = Vec::with_capacity(profiles_count);

    for (
        profile_id,
        candidate_id,
        candidate_name,
        head_noun,
        candidate_type,
        match_archetype,
        critical_attributes_json,
        numeric_specs_json,
        standards_json,
        noun_aliases_en,
        preferred_terms,
        excluded_terms,
        query_waves_json,
        recall_risk_score_bps,
        eligibility,
        notice_id,
    ) in profile_rows
    {
        notice_ids.push(notice_id);
        profiles.push(json!({
            "id": profile_id,
            "product_candidate_id": candidate_id,
            "candidate_name": candidate_name,
            "head_noun": head_noun,
            "candidate_type": candidate_type,
            "match_archetype": match_archetype,
            "critical_attributes_json":
                critical_attributes_json.and_then(|s| serde_json::from_str::<Value>(&s).ok()),
            "numeric_specs_json":
                numeric_specs_json.and_then(|s| serde_json::from_str::<Value>(&s).ok()),
            "standards_json":
                standards_json.and_then(|s| serde_json::from_str::<Value>(&s).ok()),
            "noun_aliases_en":
                noun_aliases_en.and_then(|s| serde_json::from_str::<Value>(&s).ok()),
            "preferred_terms":
                preferred_terms.and_then(|s| serde_json::from_str::<Value>(&s).ok()),
            "excluded_terms":
                excluded_terms.and_then(|s| serde_json::from_str::<Value>(&s).ok()),
            "query_waves_json":
                query_waves_json.and_then(|s| serde_json::from_str::<Value>(&s).ok()),
            "recall_risk_score_bps": recall_risk_score_bps,
            "search_eligibility_state": eligibility,
        }));
    }

    let job_parameters = json!({
        "source_id": source_id,
        "search_profiles": profiles,
        "per_candidate_budget": {
            "max_query_waves": 3,
            "shortlist_size": 8,
            "max_detail_fetches": 3,
        },
        "notice_ids": notice_ids,
    });

    ::tracing::info!(
        source_id,
        profiles_count,
        "dispatching supplier-price-enrichment batch"
    );

    if cli.enrich_pricing_dry_run {
        // Emit a machine-parseable summary on stdout and a pretty version on
        // stderr so operators can diff routing across sources quickly.
        eprintln!(
            "[dry-run] source_id={source_id} skill_path={} profiles={profiles_count}",
            skill_path.display()
        );
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "dry_run": true,
                "source_id": source_id,
                "skill_path": skill_path.display().to_string(),
                "profiles_count": profiles_count,
                "job_parameters": job_parameters,
            }))?
        );
        return Ok(LiveLoopReport {
            summary_line: format!(
                "dry-run resolved source='{source_id}' -> skill='{}' with {profiles_count} profile(s); no agent invoked",
                skill_path.display(),
            ),
            status: "completed".to_owned(),
        });
    }

    run_bound_skill(runtime, &skill_path, job_parameters, cli).await
}

/// Maps an internal source_id (`alibaba`) to the verbatim `source_name` string
/// that downstream consumers (supplier_quotes.source_name) use. Kept separate
/// so `alibaba` -> `Alibaba` capitalization lives in one place.
fn source_name_for_source_id(source_id: &str) -> &'static str {
    match source_id {
        "alibaba" => "Alibaba",
        "1688" => "1688",
        "ebay" => "eBay",
        "amazon" => "Amazon",
        _ => "unknown",
    }
}

/// Routing matrix by `candidate_type`: which sources this platform searches
/// for each commercial shape. Mirrors research doc §4.3. Returned slice is
/// the canonical source order — dispatch code should iterate in this order
/// so price-signal quality stays consistent across runs.
#[allow(dead_code)]
fn supplier_sources_for_candidate_type(candidate_type: &str) -> &'static [&'static str] {
    match candidate_type {
        "exact_sku" => &["amazon", "ebay"], // UK retail only — brand+model is UK-discoverable
        "brand_family" => &["amazon", "ebay", "alibaba"],
        "standard_commodity" => &["amazon", "ebay", "alibaba", "1688"],
        "regulated_goods" => &["amazon", "ebay"], // compliance more trustworthy on UK sources
        "bundle" => &["alibaba", "1688"],         // wholesale bundling
        "installed_goods" | "custom_or_fabricated" | "service_not_applicable" | "unknown" => &[],
        _ => &[],
    }
}

// ---------------------------------------------------------------------------
// Deterministic pipeline stages — run on already-ingested data
// ---------------------------------------------------------------------------

fn run_deterministic_pipeline(
    runtime: &mut RuntimeBootstrap,
    source_id: &str,
) -> Result<LiveLoopReport> {
    ::tracing::info!(source_id, "running deterministic pipeline stages");

    let lots_created = run_extract_lots_stage(&mut runtime.state_db, source_id)?;
    ::tracing::info!(lots_created, "extract-lots stage complete");

    let prequalified = run_prequalify_stage(&mut runtime.state_db, source_id)?;
    ::tracing::info!(prequalified, "prequalify-notice stage complete");

    let lots_qualified = run_qualify_lot_stage(&mut runtime.state_db)?;
    ::tracing::info!(lots_qualified, "qualify-lot stage complete");

    let search_profiles_built =
        run_build_search_profiles_stage(&mut runtime.state_db, source_id)?;
    ::tracing::info!(
        search_profiles_built,
        "build-search-profiles stage complete"
    );

    let relations_found = run_relate_stage(&mut runtime.state_db)?;
    ::tracing::info!(relations_found, "relate stage complete");

    Ok(LiveLoopReport {
        summary_line: format!(
            "deterministic pipeline complete: lots_created={lots_created} \
             prequalified={prequalified} lots_qualified={lots_qualified} \
             search_profiles={search_profiles_built} relations={relations_found}"
        ),
        status: "completed".to_owned(),
    })
}

/// Stage: Extract Lots — create implicit single lots for notices without explicit lots.
#[allow(clippy::type_complexity)]
fn run_extract_lots_stage(state_db: &mut StateDb, source_id: &str) -> Result<usize> {
    // Find notices that have NO lots yet.
    let notice_rows: Vec<(String, String, String, Option<String>, Option<i64>)> = {
        let conn = state_db.connection();
        let mut stmt = conn.prepare(
            "SELECT cn.id, cn.process_id, cn.title, cn.deadline_at,
                    cn.estimated_value_original_minor
             FROM contract_notices cn
             WHERE cn.source_id = ?1
               AND NOT EXISTS (
                 SELECT 1 FROM notice_lots nl WHERE nl.notice_id = cn.id
               )",
        )?;
        stmt.query_map(params![source_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<i64>>(4)?,
            ))
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?
    };

    if notice_rows.is_empty() {
        return Ok(0);
    }

    let stage_run_id = allocate_stage_run_id();
    let (count, _) = state_db.with_write_transaction(|tx| {
        let visible_after_seq = next_commit_seq(tx)?;
        let mut created = 0_usize;

        for (notice_id, process_id, title, deadline_at, value_pence) in &notice_rows {
            let lot_id = format!("{notice_id}-lot-implicit");
            tx.execute(
                "INSERT INTO notice_lots (
                   id, notice_id, source_lot_id, lot_key, lot_number, title,
                   description, estimated_value_pence, deadline_at, is_bidable_separately,
                   owner_id, row_version, last_local_seq, stage_run_id
                 ) VALUES (?1, ?2, 'implicit', 'implicit-0', 1, ?3, NULL, ?4, ?5, 1,
                           ?6, 1, ?7, ?8)
                 ON CONFLICT(id) DO NOTHING",
                params![
                    lot_id,
                    notice_id,
                    title,
                    value_pence,
                    deadline_at,
                    LOCAL_EXECUTOR_ID,
                    visible_after_seq,
                    stage_run_id,
                ],
            )?;

            persist_stage(
                tx,
                process_id,
                "extract lots",
                "notice",
                notice_id,
                &stage_run_id,
            )?;
            created += 1;
        }
        Ok(created)
    })?;

    Ok(count)
}

/// Whole-word ASCII substring search. Pass lowercased inputs for case-insensitive match.
/// Returns true when `needle` appears in `haystack` bounded by non-alphanumeric characters
/// or string edges. Used by [`notice_looks_like_dps`] so "DPS" matches "Passenger Transport DPS"
/// but not "DPSi" or "XDPS".
fn contains_whole_word_ascii(haystack: &str, needle: &str) -> bool {
    let hb = haystack.as_bytes();
    let nb = needle.as_bytes();
    let n = hb.len();
    let m = nb.len();
    if m == 0 || m > n {
        return false;
    }
    for i in 0..=n - m {
        if &hb[i..i + m] == nb {
            let before_ok = i == 0 || !hb[i - 1].is_ascii_alphanumeric();
            let after_ok = i + m == n || !hb[i + m].is_ascii_alphanumeric();
            if before_ok && after_ok {
                return true;
            }
        }
    }
    false
}

/// Heuristically detects services / care / framework-services notices when CPV
/// classification is absent or unreliable (common for procontract-sourced data
/// where raw payloads don't carry OCDS `tender.classification.id`).
///
/// Two-stage match:
///   1. Strong phrase hit (`"consultancy"`, `"managed service(s)"`, `"care home"`,
///      etc.) — immediate match.
///   2. Whole-word `"service"` / `"services"` with NO countervailing goods cue
///      (`"supply of"`, `"supplies"`, `"purchase of"`) — match.
///
/// Designed for aggressive recall with bounded precision. False positives land
/// in `rejection_reason=services_title_heuristic` so operators can audit and
/// tune the phrase list; false negatives still filter via CPV when available.
fn notice_looks_like_services(title: &str) -> bool {
    let t = title.to_ascii_lowercase();

    const SERVICES_PHRASES: &[&str] = &[
        "consultancy",
        "consulting",
        "managed service",
        "maintenance contract",
        "maintenance of",
        "contract hire",
        "care home",
        "extra care",
        "social care",
        "mental health",
        "psychological and therapeutic",
        "placements for",
        "placements -",
        "placements flexible",
        "advisory service",
        "professional service",
        "support service",
        "framework for ",
        "framework services",
        "provider list",
        "agency & landlord",
        "event management",
        "training framework",
        "training and assessment",
        "education budget",
        "advocacy service",
        "catering service",
        "housing partnership",
        "leasing contract",
        "supported living",
        "commissioning",
        "procurement system",
        "flexible procurement",
        "franchising",
        "tenant census",
        "early intervention",
        "alternative provision",
        "alternative (education) provision",
        "alternative education",
        "quality control for",
        "evaluation and quality",
        "digital platform",
        "contract management project",
        "design team",
        "management system",
        "management solutions",
        "research management",
        "support fund",
        "approved list",
        "risk assessment",
        "adoption support",
        "open framework",
        "care & repair",
        "care and repair",
        "bed framework",
        "nursing framework",
        "development of",
        "training",
        "advertising",
        "postgraduate",
    ];

    for phrase in SERVICES_PHRASES {
        if t.contains(phrase) {
            return true;
        }
    }

    let service_wordhit = contains_whole_word_ascii(&t, "service")
        || contains_whole_word_ascii(&t, "services");

    if service_wordhit {
        // Countervailing goods cues — if present, treat as goods despite the
        // word "service" appearing. Keep this list tight; weakening it floods
        // the pipeline with services contracts again.
        let goods_counter = t.contains("supply of ")
            || t.contains(" supplies")
            || t.contains("supplies ")
            || t.contains("purchase of ")
            || t.contains(" products")
            || t.contains("equipment supply");
        if !goods_counter {
            return true;
        }
    }

    false
}

/// Heuristically detects construction / civil-engineering (CPV 45-ish) notices by
/// title, for sources (like procontract) that don't carry CPV codes. Keeps the
/// rejection reason distinct from services so operators can tune per-category.
fn notice_looks_like_works(title: &str) -> bool {
    let t = title.to_ascii_lowercase();
    const WORKS_PHRASES: &[&str] = &[
        "highways maintenance",
        "culvert",
        "junction improvement",
        "waterproofing",
        "refurbishment",
        "renovation",
        "construction of",
        "re-roofing",
        "reroofing",
        "demolition",
        "civil engineering",
        "civil works",
        "bridge works",
        "road improvement",
        "resurfacing",
        "building works",
        "groundworks",
        "ground works",
        "retender",
        "external works",
        "internal works",
        "fire risk assessment works",
        "fit out",
        "fit-out",
    ];
    WORKS_PHRASES.iter().any(|p| t.contains(p))
}

/// Heuristically detects Dynamic Purchasing System / Dynamic Market notices.
/// DPS notices require framework membership rather than a one-shot product bid,
/// so this platform's operator has opted to reject them at qualify time.
///
/// Detection sweeps the notice title plus common description paths in the raw
/// payload. Triggers on `"dynamic purchasing"` / `"dynamic market"` substrings,
/// or on `"dps"` / `"dm"` as whole ASCII words.
fn notice_looks_like_dps(title: &str, payload_json: Option<&str>) -> bool {
    let title_lower = title.to_ascii_lowercase();
    if title_lower.contains("dynamic purchasing") || title_lower.contains("dynamic market") {
        return true;
    }
    if contains_whole_word_ascii(&title_lower, "dps") {
        return true;
    }
    if let Some(raw) = payload_json {
        if let Ok(doc) = serde_json::from_str::<Value>(raw) {
            for path in [
                "/detail/description",
                "/tender/description",
                "/detail/title",
                "/listing/description",
            ] {
                if let Some(text) = doc.pointer(path).and_then(Value::as_str) {
                    let lowered = text.to_ascii_lowercase();
                    if lowered.contains("dynamic purchasing")
                        || lowered.contains("dynamic market")
                        || contains_whole_word_ascii(&lowered, "dps")
                    {
                        return true;
                    }
                }
            }
        }
    }
    false
}

/// Stage: Prequalify Notice — classify goods/services using CPV codes from raw OCDS payloads.
#[allow(clippy::too_many_lines, clippy::type_complexity)]
fn run_prequalify_stage(state_db: &mut StateDb, source_id: &str) -> Result<usize> {
    // Find notices that have Normalize succeeded but no prequalify stage head.
    // Also join the first lot for FK compliance in qualification_runs.
    #[allow(clippy::type_complexity)]
    let notice_rows: Vec<(
        String,
        String,
        String,
        Option<String>,
        Option<i64>,
        Option<String>,
        String,
    )> = {
        let conn = state_db.connection();
        let mut stmt = conn.prepare(
            "SELECT cn.id, cn.process_id, cn.title, cn.deadline_at,
                    cn.estimated_value_original_minor,
                    rd.payload_json,
                    (SELECT nl.id FROM notice_lots nl WHERE nl.notice_id = cn.id LIMIT 1) as lot_id
             FROM contract_notices cn
             JOIN raw_documents rd ON rd.id = ('raw-' || cn.id)
             WHERE cn.source_id = ?1
               AND EXISTS (
                 SELECT 1 FROM process_stage_heads h
                 WHERE h.process_id = cn.process_id
                   AND h.stage = 'Normalize'
                   AND h.scope_kind = 'notice'
               )
               AND NOT EXISTS (
                 SELECT 1 FROM process_stage_heads h
                 WHERE h.process_id = cn.process_id
                   AND h.stage = 'prequalify notice'
                   AND h.scope_kind = 'notice'
                   AND h.scope_id = cn.id
               )",
        )?;
        stmt.query_map(params![source_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<i64>>(4)?,
                row.get::<_, Option<String>>(5)?,
                row.get::<_, String>(6)?,
            ))
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?
    };

    if notice_rows.is_empty() {
        return Ok(0);
    }

    let stage_run_id = allocate_stage_run_id();
    let (count, _) = state_db.with_write_transaction(|tx| {
        let visible_after_seq = next_commit_seq(tx)?;
        let mut qualified_count = 0_usize;

        for (notice_id, process_id, title, deadline_at, value_pence, payload_json, lot_id) in
            &notice_rows
        {
            let cpv_division = payload_json
                .as_deref()
                .and_then(|raw| serde_json::from_str::<Value>(raw).ok())
                .and_then(|doc| {
                    doc.get("tender")
                        .and_then(|t| t.get("classification"))
                        .and_then(|c| c.get("id"))
                        .and_then(Value::as_str)
                        .and_then(|code| code.chars().take(2).collect::<String>().parse::<u32>().ok())
                });

            // CPV classification per EU procurement vocabulary:
            // Divisions 03-09: agriculture, mining, energy products (goods)
            // Divisions 14-44: manufactured goods
            // Division 45: construction works
            // Divisions 48-49: software packages and transport (goods/mixed)
            // Divisions 50-98: services
            let legal_category = match cpv_division {
                Some(d) if (3..=44).contains(&d) => "goods",
                Some(45) => "works",
                Some(d) if (48..=49).contains(&d) => "goods",
                Some(d) if (50..=98).contains(&d) => "services",
                _ => "unknown",
            };

            let goods_density_score_bps: i64 = match legal_category {
                "goods" => 8500,
                "mixed" => 6000,
                "works" => 3000,
                "services" => 1500,
                _ => 4500,
            };

            let expiry_qualified = deadline_at
                .as_deref()
                .and_then(|d| chrono::DateTime::parse_from_rfc3339(d).ok())
                .is_none_or(|deadline| {
                    let remaining = deadline.signed_duration_since(Utc::now());
                    remaining.num_days() >= 0
                });

            let goods_qualified_cpv = legal_category == "goods" || legal_category == "mixed";
            let dps_detected = notice_looks_like_dps(title, payload_json.as_deref());
            let services_title_detected = notice_looks_like_services(title);
            let works_title_detected = notice_looks_like_works(title);

            // Platform policy: product-only bidding. Reject DPS (framework membership),
            // services, and works at qualify time so downstream stages don't spend
            // document-acquisition or extraction budget on them. The services gate
            // runs twice: once via CPV (trusted for contracts-finder-style sources),
            // once via title heuristic (fallback for procontract which doesn't carry
            // CPV — see docs/session-log-2026-04-21).
            let (qualification_result, rejection_reason) = if dps_detected {
                ("reject", Some("dynamic_purchasing_system"))
            } else if !expiry_qualified {
                ("reject", Some("expired"))
            } else if legal_category == "services" {
                ("reject", Some("services_category"))
            } else if legal_category == "works" {
                ("reject", Some("works_category"))
            } else if works_title_detected {
                ("reject", Some("works_title_heuristic"))
            } else if services_title_detected {
                ("reject", Some("services_title_heuristic"))
            } else if goods_qualified_cpv && goods_density_score_bps >= 6000 {
                ("pass", None)
            } else if goods_qualified_cpv || legal_category == "unknown" {
                ("review", None)
            } else {
                ("reject", Some("non_goods_unclassified"))
            };

            // access_route is the durable semantic signal downstream stages read. DPS
            // is explicitly `dynamic_market`; otherwise mirror the category so
            // rejected routes (services/works) are distinguishable from pass/review.
            let access_route = if dps_detected {
                "dynamic_market"
            } else if legal_category == "services" || services_title_detected {
                "services"
            } else if legal_category == "works" || works_title_detected {
                "works"
            } else {
                "open_tender"
            };

            // Persist access_route back to the notice too, so any consumer that
            // reads `contract_notices.access_route` directly (UI, projections) sees
            // the correct value. This closes the "DPS notice tagged portal_authenticated"
            // gap surfaced 2026-04-21 during the qualify backfill audit.
            tx.execute(
                "UPDATE contract_notices SET access_route = ?1, \
                 row_version = row_version + 1, updated_at = CURRENT_TIMESTAMP \
                 WHERE id = ?2",
                params![access_route, notice_id],
            )?;

            // goods_qualified (persisted) is the "is this worth advancing?" signal.
            // Force 0 when rejected so downstream gates (e.g. tender-spec product
            // extraction) skip even when they don't read the rationale text.
            let goods_qualified_persisted =
                qualification_result == "pass" || qualification_result == "review";

            let addressable_value = if legal_category == "goods" && goods_qualified_persisted {
                *value_pence
            } else {
                None
            };

            let rationale = format!(
                "result={qualification_result}; reason={}; access_route={access_route}; \
                 legal_category={legal_category}; cpv_div={}; \
                 goods_density={goods_density_score_bps}bps; expiry_qualified={expiry_qualified}",
                rejection_reason.unwrap_or("-"),
                cpv_division.map_or_else(|| "unknown".to_owned(), |d| d.to_string()),
            );

            let qual_id = format!("qual-{notice_id}");

            tx.execute(
                "INSERT INTO qualification_runs (
                   id, process_id, notice_id, lot_id, expiry_qualified, goods_qualified,
                   goods_confidence_bps, legal_category, goods_density_score_bps,
                   qualification_basis, addressable_value_pence, rationale, access_route,
                   owner_id, row_version, last_local_seq, stage_run_id
                 ) VALUES (?1, ?2, ?3, ?14, ?4, ?5, ?6, ?7, ?8, 'notice_only', ?9, ?10,
                           ?15, ?11, 1, ?12, ?13)
                 ON CONFLICT(id) DO UPDATE SET
                   goods_qualified = excluded.goods_qualified,
                   goods_confidence_bps = excluded.goods_confidence_bps,
                   legal_category = excluded.legal_category,
                   access_route = excluded.access_route,
                   rationale = excluded.rationale,
                   row_version = qualification_runs.row_version + 1,
                   updated_at = CURRENT_TIMESTAMP",
                params![
                    qual_id,
                    process_id,
                    notice_id,
                    i32::from(expiry_qualified),
                    i32::from(goods_qualified_persisted),
                    goods_density_score_bps,
                    legal_category,
                    goods_density_score_bps,
                    addressable_value,
                    rationale,
                    LOCAL_EXECUTOR_ID,
                    visible_after_seq,
                    stage_run_id,
                    lot_id,
                    access_route,
                ],
            )?;

            persist_stage(
                tx,
                process_id,
                "prequalify notice",
                "notice",
                notice_id,
                &stage_run_id,
            )?;

            qualified_count += 1;
        }
        Ok(qualified_count)
    })?;

    Ok(count)
}

/// Stage: Qualify Lot — per-lot qualification inheriting from notice-level prequalification.
fn run_qualify_lot_stage(state_db: &mut StateDb) -> Result<usize> {
    // Find lots whose notice has been prequalified but the lot itself hasn't been qualified.
    let lot_rows: Vec<(String, String, String, String, Option<i64>)> = {
        let conn = state_db.connection();
        let mut stmt = conn.prepare(
            "SELECT nl.id, nl.notice_id, cn.process_id, cn.id as cn_id,
                    nl.estimated_value_pence
             FROM notice_lots nl
             JOIN contract_notices cn ON cn.id = nl.notice_id
             WHERE EXISTS (
               SELECT 1 FROM process_stage_heads h
               WHERE h.process_id = cn.process_id
                 AND h.stage = 'prequalify notice'
                 AND h.scope_kind = 'notice'
             )
             AND NOT EXISTS (
               SELECT 1 FROM process_stage_heads h
               WHERE h.process_id = cn.process_id
                 AND h.stage = 'qualify lot'
                 AND h.scope_kind = 'lot'
                 AND h.scope_id = nl.id
             )",
        )?;
        stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<i64>>(4)?,
            ))
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?
    };

    if lot_rows.is_empty() {
        return Ok(0);
    }

    let stage_run_id = allocate_stage_run_id();
    let (count, _) = state_db.with_write_transaction(|tx| {
        let visible_after_seq = next_commit_seq(tx)?;
        let mut qualified = 0_usize;

        for (lot_id, notice_id, process_id, _cn_id, lot_value) in &lot_rows {
            // Inherit from notice-level qualification, including access_route so
            // downstream lot-scoped consumers see the same rejection signal.
            let parent_qual: Option<(i32, i32, i64, String, Option<String>)> = tx
                .query_row(
                    "SELECT expiry_qualified, goods_qualified, goods_confidence_bps, \
                     legal_category, access_route
                     FROM qualification_runs
                     WHERE notice_id = ?1
                     ORDER BY created_at DESC LIMIT 1",
                    params![notice_id],
                    |row| {
                        Ok((
                            row.get::<_, i32>(0)?,
                            row.get::<_, i32>(1)?,
                            row.get::<_, i64>(2)?,
                            row.get::<_, String>(3)?,
                            row.get::<_, Option<String>>(4)?,
                        ))
                    },
                )
                .optional()?;

            let (expiry_q, goods_q, confidence, category, access_route) = parent_qual
                .unwrap_or_else(|| (1, 0, 4500, "unknown".to_owned(), None));

            let qual_id = format!("qual-lot-{lot_id}");
            let rationale = format!(
                "Inherited from notice {notice_id}: category={category}, \
                 access_route={}, confidence={confidence}bps",
                access_route.as_deref().unwrap_or("-"),
            );

            tx.execute(
                "INSERT INTO qualification_runs (
                   id, process_id, notice_id, lot_id, expiry_qualified, goods_qualified,
                   goods_confidence_bps, legal_category, qualification_basis,
                   addressable_value_pence, rationale, access_route,
                   owner_id, row_version, last_local_seq, stage_run_id
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'lot_level', ?9, ?10,
                           ?14, ?11, 1, ?12, ?13)
                 ON CONFLICT(id) DO NOTHING",
                params![
                    qual_id,
                    process_id,
                    notice_id,
                    lot_id,
                    expiry_q,
                    goods_q,
                    confidence,
                    category,
                    lot_value,
                    rationale,
                    LOCAL_EXECUTOR_ID,
                    visible_after_seq,
                    stage_run_id,
                    access_route,
                ],
            )?;

            persist_stage(tx, process_id, "qualify lot", "lot", lot_id, &stage_run_id)?;
            qualified += 1;
        }
        Ok(qualified)
    })?;

    Ok(count)
}

// ---------------------------------------------------------------------------
// Stage 8.5: build_search_profiles
// Projects qualify-passed product_candidates into search_profiles rows with
// pre-computed query waves, noun aliases, market framing terms, and
// recall-risk / quoteability scores. The downstream supplier-price-enrichment
// skill consumes these profiles wholesale — so the agent sees the entire
// structured product context, not just head_noun.
// ---------------------------------------------------------------------------

/// Picks the single strongest identity anchor the tender specifies for this
/// product. Priority: exact brand+model → first `required`-criticality critical
/// attribute value → first numeric spec rendered as `"<nominal><unit>"` → first
/// standards token. Returns `None` when the candidate has no structured hooks,
/// in which case query waves fall back to the candidate's free-text name.
fn strongest_search_anchor(
    brand_model: Option<&str>,
    critical_attributes_json: Option<&str>,
    numeric_specs_json: Option<&str>,
    standards_json: Option<&str>,
) -> Option<String> {
    if let Some(bm) = brand_model {
        let trimmed = bm.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_owned());
        }
    }

    if let Some(raw) = critical_attributes_json {
        if let Ok(Value::Object(map)) = serde_json::from_str::<Value>(raw) {
            for (_, entry) in &map {
                let crit = entry.get("criticality").and_then(Value::as_str);
                let value = entry.get("value").and_then(Value::as_str);
                if crit == Some("required")
                    && let Some(v) = value
                    && !v.trim().is_empty()
                {
                    return Some(v.trim().to_owned());
                }
            }
        }
    }

    if let Some(raw) = numeric_specs_json {
        if let Ok(Value::Object(map)) = serde_json::from_str::<Value>(raw) {
            for (_, entry) in &map {
                let nominal = entry.get("nominal").and_then(Value::as_f64);
                let unit = entry
                    .get("unit")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                if let Some(n) = nominal {
                    let trimmed_unit = unit.trim();
                    let rendered = if trimmed_unit.is_empty() {
                        format!("{n}")
                    } else {
                        format!("{n}{trimmed_unit}")
                    };
                    return Some(rendered);
                }
            }
        }
    }

    if let Some(raw) = standards_json {
        if let Ok(Value::Array(arr)) = serde_json::from_str::<Value>(raw) {
            for entry in &arr {
                if let Some(s) = entry.as_str() {
                    let trimmed = s.trim();
                    if !trimmed.is_empty() {
                        return Some(trimmed.to_owned());
                    }
                }
            }
        }
    }

    None
}

/// Generates the 5-wave query ladder the supplier-search agent walks when
/// searching each source (precise → broad). Wave 4 is only populated for
/// regulated_goods candidates per the research spec — for other archetypes a
/// compliance-off retry is a no-op.
///
/// Returned JSON has the shape the `search_profiles.query_waves_json` column
/// expects: `[{wave, strategy, query_en, query_zh?, filters_hint}]`.
#[allow(clippy::too_many_arguments)]
fn build_query_waves_json(
    head_noun: &str,
    candidate_name: &str,
    candidate_type: &str,
    brand_model: Option<&str>,
    material: Option<&str>,
    critical_attributes_json: Option<&str>,
    numeric_specs_json: Option<&str>,
    standards_json: Option<&str>,
    preferred_terms_json: Option<&str>,
) -> String {
    let anchor = strongest_search_anchor(
        brand_model,
        critical_attributes_json,
        numeric_specs_json,
        standards_json,
    );
    let anchor_str = anchor.as_deref();

    let preferred_terms: Vec<String> = preferred_terms_json
        .and_then(|raw| serde_json::from_str::<Vec<String>>(raw).ok())
        .unwrap_or_default();
    let alias = preferred_terms
        .first()
        .map(String::as_str)
        .unwrap_or(head_noun);

    let material_frag = material
        .map(|m| format!(" {m}"))
        .unwrap_or_default();

    let wave0 = match (brand_model, anchor_str) {
        (Some(bm), _) if !bm.trim().is_empty() => bm.trim().to_owned(),
        (_, Some(a)) => format!("{head_noun} {a}"),
        (_, None) => candidate_name.to_owned(),
    };

    let wave1 = match anchor_str {
        Some(a) => format!("{alias} {a}"),
        None => alias.to_owned(),
    };

    let wave2 = format!("industrial commercial {head_noun}{material_frag}");
    let wave3 = head_noun.to_owned();

    let mut waves = vec![
        json!({
            "wave": 0,
            "strategy": "exact_anchor",
            "query_en": wave0.trim(),
            "filters_hint": {
                "require_head_noun": true,
                "require_anchor": anchor.is_some(),
            },
        }),
        json!({
            "wave": 1,
            "strategy": "alias_swap",
            "query_en": wave1.trim(),
            "filters_hint": { "require_head_noun": true },
        }),
        json!({
            "wave": 2,
            "strategy": "market_frame",
            "query_en": wave2.trim(),
            "filters_hint": { "require_head_noun": true, "relaxed_material": true },
        }),
        json!({
            "wave": 3,
            "strategy": "broad_existence",
            "query_en": wave3.trim(),
            "filters_hint": { "require_head_noun": true, "existence_check_only": true },
        }),
    ];

    if candidate_type == "regulated_goods" {
        waves.push(json!({
            "wave": 4,
            "strategy": "compliance_off",
            "query_en": format!("{head_noun}{material_frag}").trim().to_owned(),
            "filters_hint": {
                "standards_as_post_filter": true,
            },
        }));
    }

    Value::Array(waves).to_string()
}

/// Deterministic quoteability score for gating marketplace search.
/// `auto_search` >= 6500, `manual_only` 4500..=6499, `skip` < 4500.
/// Mirrors the research doc §8 additive-penalty model — not an agent call.
fn compute_quoteability_score_bps(
    candidate_type: &str,
    match_archetype: &str,
    head_noun: &str,
    has_critical_attrs: bool,
    has_numeric_specs: bool,
    has_standards: bool,
    has_quantity_unit: bool,
    has_brand_model: bool,
    access_route: Option<&str>,
) -> i64 {
    let mut score: i64 = 5000;

    if !head_noun.trim().is_empty() && head_noun != "item" {
        score += 500;
    }
    if has_critical_attrs {
        score += 1000;
    }
    if has_numeric_specs {
        score += 1000;
    }
    if has_standards {
        score += 500;
    }
    if has_quantity_unit {
        score += 500;
    }
    if has_brand_model && candidate_type == "exact_sku" {
        score += 1500;
    }

    match candidate_type {
        "custom_or_fabricated" => score -= 3000,
        "installed_goods" => score -= 2000,
        "bundle" => score -= 500,
        _ => {}
    }

    if candidate_type == "regulated_goods" && !has_standards {
        score -= 1500;
    }
    if match_archetype == "standard_class_led" && !has_standards {
        score -= 1000;
    }
    if match_archetype == "dimension_material_led" && !has_numeric_specs {
        score -= 1000;
    }
    if match_archetype == "model_exact" && !has_brand_model {
        score -= 2000;
    }

    if matches!(access_route, Some("dynamic_market" | "services" | "works")) {
        score -= 2000;
    }

    score.clamp(0, 10000)
}

/// Deterministic recall-risk score (0-10000 bps). Higher = more vocabulary drift,
/// wider search budget required. Drives the supplier-search agent's shortlist
/// size (low risk → top 8 results; high risk → top 20).
fn compute_recall_risk_bps(
    candidate_type: &str,
    match_archetype: &str,
    has_critical_attrs: bool,
    has_numeric_specs: bool,
    has_standards: bool,
    has_brand_model: bool,
    has_preferred_terms: bool,
) -> i64 {
    let mut risk: i64 = 2000;

    if !has_critical_attrs {
        risk += 1500;
    }
    if !has_numeric_specs && match_archetype == "dimension_material_led" {
        risk += 2000;
    }
    if !has_standards && match_archetype == "standard_class_led" {
        risk += 2500;
    }
    if !has_brand_model && candidate_type == "exact_sku" {
        risk += 3000;
    }
    if !has_preferred_terms {
        risk += 1000;
    }
    if candidate_type == "unknown" {
        risk += 1500;
    }

    risk.clamp(0, 10000)
}

fn search_eligibility_state(quoteability_bps: i64) -> &'static str {
    if quoteability_bps >= 6500 {
        "auto_search"
    } else if quoteability_bps >= 4500 {
        "manual_only"
    } else {
        "skip"
    }
}

/// Derives English noun aliases from the head_noun. Placeholder for the full
/// lexicon work called out in the research doc §5.4 — for now just reflects the
/// head_noun plus any preferred terms the skill already emitted, so downstream
/// alias-swap queries have something to work with.
fn derive_noun_aliases_en(head_noun: &str, preferred_terms_json: Option<&str>) -> String {
    let mut aliases = vec![head_noun.to_owned()];
    if let Some(raw) = preferred_terms_json {
        if let Ok(arr) = serde_json::from_str::<Vec<String>>(raw) {
            for term in arr {
                if !aliases.iter().any(|a| a.eq_ignore_ascii_case(&term)) {
                    aliases.push(term);
                }
            }
        }
    }
    json!(aliases).to_string()
}

/// Stage: build_search_profiles — project qualify-passed product_candidates into
/// search_profiles rows. Gated on `qualification_runs.goods_qualified = 1` so
/// services / DPS / works / expired notices do not produce profiles.
fn run_build_search_profiles_stage(state_db: &mut StateDb, source_id: &str) -> Result<usize> {
    #[allow(clippy::type_complexity)]
    let candidate_rows: Vec<(
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
    )> = {
        let conn = state_db.connection();
        let mut stmt = conn.prepare(
            "SELECT pc.id, pc.process_id, pc.notice_id, pc.name, pc.head_noun,
                    pc.candidate_type, pc.match_archetype,
                    pc.brand_model, pc.material, pc.quantity_unit,
                    pc.critical_attributes_json, pc.numeric_specs_json,
                    pc.standards_json, pc.preferred_terms_json,
                    cn.access_route
             FROM product_candidates pc
             JOIN contract_notices cn ON cn.id = pc.notice_id
             JOIN qualification_runs qr ON qr.notice_id = pc.notice_id
               AND qr.qualification_basis = 'notice_only'
             WHERE cn.source_id = ?1
               AND qr.goods_qualified = 1
               AND NOT EXISTS (
                 SELECT 1 FROM search_profiles sp
                 WHERE sp.product_candidate_id = pc.id
               )",
        )?;
        stmt.query_map(params![source_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, Option<String>>(7)?,
                row.get::<_, Option<String>>(8)?,
                row.get::<_, Option<String>>(9)?,
                row.get::<_, Option<String>>(10)?,
                row.get::<_, Option<String>>(11)?,
                row.get::<_, Option<String>>(12)?,
                row.get::<_, Option<String>>(13)?,
                row.get::<_, Option<String>>(14)?,
            ))
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?
    };

    if candidate_rows.is_empty() {
        return Ok(0);
    }

    let stage_run_id = allocate_stage_run_id();
    let (count, _) = state_db.with_write_transaction(|tx| {
        let visible_after_seq = next_commit_seq(tx)?;
        let mut profiles_written = 0_usize;

        for (
            candidate_id,
            process_id,
            notice_id,
            candidate_name,
            head_noun,
            candidate_type,
            match_archetype,
            brand_model,
            material,
            quantity_unit,
            critical_attributes_json,
            numeric_specs_json,
            standards_json,
            preferred_terms_json,
            access_route,
        ) in &candidate_rows
        {
            let has_critical_attrs = critical_attributes_json
                .as_deref()
                .is_some_and(|s| s.trim() != "" && s.trim() != "{}" && s.trim() != "null");
            let has_numeric_specs = numeric_specs_json
                .as_deref()
                .is_some_and(|s| s.trim() != "" && s.trim() != "{}" && s.trim() != "null");
            let has_standards = standards_json.as_deref().is_some_and(|s| {
                let trimmed = s.trim();
                !trimmed.is_empty() && trimmed != "[]" && trimmed != "null"
            });
            let has_preferred_terms = preferred_terms_json.as_deref().is_some_and(|s| {
                let trimmed = s.trim();
                !trimmed.is_empty() && trimmed != "[]" && trimmed != "null"
            });
            let has_brand_model = brand_model
                .as_deref()
                .is_some_and(|v| !v.trim().is_empty());
            let has_quantity_unit = quantity_unit
                .as_deref()
                .is_some_and(|v| !v.trim().is_empty());

            let quoteability_bps = compute_quoteability_score_bps(
                candidate_type,
                match_archetype,
                head_noun,
                has_critical_attrs,
                has_numeric_specs,
                has_standards,
                has_quantity_unit,
                has_brand_model,
                access_route.as_deref(),
            );
            let recall_risk_bps = compute_recall_risk_bps(
                candidate_type,
                match_archetype,
                has_critical_attrs,
                has_numeric_specs,
                has_standards,
                has_brand_model,
                has_preferred_terms,
            );
            let eligibility = search_eligibility_state(quoteability_bps);

            let query_waves_json = build_query_waves_json(
                head_noun,
                candidate_name,
                candidate_type,
                brand_model.as_deref(),
                material.as_deref(),
                critical_attributes_json.as_deref(),
                numeric_specs_json.as_deref(),
                standards_json.as_deref(),
                preferred_terms_json.as_deref(),
            );

            let noun_aliases_en =
                derive_noun_aliases_en(head_noun, preferred_terms_json.as_deref());

            let profile_id = format!("sp-{candidate_id}");

            tx.execute(
                "INSERT INTO search_profiles (
                   id, product_candidate_id, head_noun, head_noun_family,
                   candidate_type, match_archetype,
                   critical_attributes_json, standards_json, numeric_specs_json,
                   noun_aliases_en, noun_aliases_zh, market_frame_en, market_frame_zh,
                   use_context, sector_context,
                   required_anchors, optional_anchors, must_not_have,
                   preferred_terms, excluded_terms,
                   query_waves_json, recall_risk_score_bps, search_eligibility_state,
                   owner_id, row_version, last_local_seq, stage_run_id
                 ) VALUES (
                   ?1, ?2, ?3, ?4, ?5, ?6,
                   ?7, ?8, ?9,
                   ?10, NULL, NULL, NULL,
                   NULL, NULL,
                   NULL, NULL, NULL,
                   ?11, NULL,
                   ?12, ?13, ?14,
                   ?15, 1, ?16, ?17
                 )
                 ON CONFLICT(id) DO UPDATE SET
                   head_noun = excluded.head_noun,
                   candidate_type = excluded.candidate_type,
                   match_archetype = excluded.match_archetype,
                   critical_attributes_json = excluded.critical_attributes_json,
                   standards_json = excluded.standards_json,
                   numeric_specs_json = excluded.numeric_specs_json,
                   noun_aliases_en = excluded.noun_aliases_en,
                   preferred_terms = excluded.preferred_terms,
                   query_waves_json = excluded.query_waves_json,
                   recall_risk_score_bps = excluded.recall_risk_score_bps,
                   search_eligibility_state = excluded.search_eligibility_state,
                   row_version = search_profiles.row_version + 1,
                   updated_at = CURRENT_TIMESTAMP",
                params![
                    profile_id,
                    candidate_id,
                    head_noun,
                    Option::<&str>::None, // head_noun_family: lexicon TBD
                    candidate_type,
                    match_archetype,
                    critical_attributes_json,
                    standards_json,
                    numeric_specs_json,
                    noun_aliases_en,
                    preferred_terms_json,
                    query_waves_json,
                    recall_risk_bps,
                    eligibility,
                    LOCAL_EXECUTOR_ID,
                    visible_after_seq,
                    stage_run_id,
                ],
            )?;

            // Mirror the quoteability score back to the candidate so any consumer
            // that reads product_candidates directly (desktop UI, projections) sees
            // the gated score rather than a stale null.
            tx.execute(
                "UPDATE product_candidates SET quoteability_score_bps = ?1, \
                 recall_risk_score_bps = ?2, row_version = row_version + 1, \
                 updated_at = CURRENT_TIMESTAMP WHERE id = ?3",
                params![quoteability_bps, recall_risk_bps, candidate_id],
            )?;

            persist_stage(
                tx,
                process_id,
                "build search profile",
                "notice",
                notice_id,
                &stage_run_id,
            )?;
            profiles_written += 1;
        }

        Ok(profiles_written)
    })?;

    Ok(count)
}

/// Stage: Relate — detect duplicate/related processes using OCID matching and title similarity.
fn run_relate_stage(state_db: &mut StateDb) -> Result<usize> {
    // Find processes that haven't been through relation resolution.
    let process_rows: Vec<(String, String, Option<String>, Option<String>)> = {
        let conn = state_db.connection();
        let mut stmt = conn.prepare(
            "SELECT p.id, p.title, p.canonical_ocid, p.buyer_name
             FROM procurement_processes p
             WHERE NOT EXISTS (
               SELECT 1 FROM process_stage_heads h
               WHERE h.process_id = p.id
                 AND h.stage = 'relate'
                 AND h.scope_kind = 'process'
             )",
        )?;
        stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
            ))
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?
    };

    if process_rows.is_empty() {
        return Ok(0);
    }

    // Find OCID-based duplicates: processes sharing the same canonical_ocid.
    let mut ocid_groups: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    for (process_id, _title, ocid, _buyer) in &process_rows {
        if let Some(oid) = ocid.as_deref().filter(|o| !o.is_empty()) {
            ocid_groups
                .entry(oid.to_owned())
                .or_default()
                .push(process_id.clone());
        }
    }

    let stage_run_id = allocate_stage_run_id();
    let (relations_count, _) = state_db.with_write_transaction(|tx| {
        let visible_after_seq = next_commit_seq(tx)?;
        let mut relations = 0_usize;

        // Create relations for processes sharing the same OCID.
        for (ocid, group) in &ocid_groups {
            if group.len() < 2 {
                continue;
            }
            for i in 0..group.len() {
                for j in (i + 1)..group.len() {
                    let rel_id = format!("rel-{}-{}", &group[i], &group[j]);
                    tx.execute(
                        "INSERT INTO process_relations (
                           id, process_id_a, process_id_b, relation_type,
                           confidence_bps, evidence,
                           owner_id, row_version, last_local_seq, stage_run_id
                         ) VALUES (?1, ?2, ?3, 'merged_same_process', 10000, ?4,
                                   ?5, 1, ?6, ?7)
                         ON CONFLICT(id) DO NOTHING",
                        params![
                            rel_id,
                            group[i],
                            group[j],
                            json!({"rule": "same_ocid", "ocid": ocid}).to_string(),
                            LOCAL_EXECUTOR_ID,
                            visible_after_seq,
                            stage_run_id,
                        ],
                    )?;
                    relations += 1;
                }
            }
        }

        // Mark all processes as having completed the relate stage.
        for (process_id, _title, _ocid, _buyer) in &process_rows {
            persist_stage(
                tx,
                process_id,
                "relate",
                "process",
                process_id,
                &stage_run_id,
            )?;
        }

        Ok(relations)
    })?;

    Ok(relations_count)
}

fn persist_stage(
    tx: &Transaction<'_>,
    process_id: &str,
    stage: &str,
    scope_kind: &str,
    scope_id: &str,
    stage_run_id: &str,
) -> Result<()> {
    let scoped_stage_run_id = format!(
        "{stage_run_id}-{}",
        scope_id.replace(|character: char| !character.is_ascii_alphanumeric(), "-")
    );
    persist_stage_validation(
        tx,
        &StageValidationWrite {
            process_id,
            stage,
            scope_kind,
            scope_id,
            stage_run_id: &scoped_stage_run_id,
            owner_id: DEFAULT_OWNER_ID,
        },
        &ValidationOutcome {
            disposition: ValidationDisposition::Accepted,
            error_code: None,
            error_context_json: None,
            anomalies: Vec::new(),
        },
    )
}

fn next_commit_seq(tx: &Transaction<'_>) -> Result<i64> {
    tx.query_row(
        "SELECT commit_seq FROM commit_clock WHERE id = 1",
        [],
        |row| row.get::<_, i64>(0),
    )
    .map(|value| value + 1)
    .context("reading next commit_seq")
}

fn current_runtime_ids() -> Result<RuntimeIds> {
    let process_id = process::id();
    let process_id_i32 = i32::try_from(process_id).context("converting pid to i32")?;
    let process_id_handle = Pid::from_raw(process_id_i32);
    let process_group_id =
        getpgid(Some(process_id_handle)).context("reading current process group")?;
    let session_id = getsid(Some(process_id_handle)).context("reading current session")?;
    Ok(RuntimeIds {
        sid: u32::try_from(session_id.as_raw()).context("converting sid to u32")?,
        pid: process_id,
        pgid: u32::try_from(process_group_id.as_raw()).context("converting pgid to u32")?,
    })
}

fn skill_config_hash(skill: &LoadedSkill) -> String {
    sha256_hex(
        json!({
            "skill_markdown": skill.skill_markdown,
            "skill_binding": skill_binding_payload(skill),
            "model_profile": skill.metadata.model_profile,
        })
        .to_string()
        .as_bytes(),
    )
}

fn review_reason(fixture: &LiveLoopFixture) -> Option<String> {
    let mut reasons = Vec::new();
    if fixture.goods_classification_low_confidence {
        reasons.push(ErrorCode::GoodsClassificationLowConfidence.as_str());
    }
    if fixture.supplier_match_low_confidence {
        reasons.push(ErrorCode::SupplierMatchLowConfidence.as_str());
    }
    if fixture.commodity_code_low_confidence {
        reasons.push(ErrorCode::CommodityCodeLowConfidence.as_str());
    }
    if fixture.revenue_basis_uncertain {
        reasons.push(ErrorCode::RevenueBasisUncertain.as_str());
    }
    if fixture.fpo_access_unavailable {
        reasons.push(ErrorCode::FpoAccessUnavailable.as_str());
    }
    if reasons.is_empty() {
        None
    } else {
        Some(reasons.join(","))
    }
}

fn oqg_input(fixture: &LiveLoopFixture) -> OqgCheckInput {
    OqgCheckInput {
        expected_fill_bps: 9000,
        observed_fill_bps: if fixture.manifest_oqg_suspect {
            4000
        } else {
            9100
        },
        duplicate_rate_bps: 0,
        max_duplicate_rate_bps: 500,
        observed_values: vec![2500, 3600],
        allowed_min_value: 1000,
        allowed_max_value: 200_000,
    }
}

fn first_process_id(fixture: &LiveLoopFixture) -> Result<String> {
    fixture
        .notices
        .first()
        .map(|notice| notice.process_id.clone())
        .ok_or_else(|| anyhow!("fixture did not provide a process id"))
}

fn first_process_id_from_db(conn: &Connection) -> Result<Option<String>> {
    conn.query_row(
        "SELECT id FROM procurement_processes ORDER BY created_at LIMIT 1",
        [],
        |row| row.get(0),
    )
    .optional()
    .context("loading first process id")
}

fn stage_run_ids() -> StageRunIds {
    StageRunIds {
        normalize: allocate_stage_run_id(),
        prequalify_notice: allocate_stage_run_id(),
        acquire_docs: allocate_stage_run_id(),
        extract_lots: allocate_stage_run_id(),
        qualify_lot: allocate_stage_run_id(),
        extract_products: allocate_stage_run_id(),
        score_quoteability: allocate_stage_run_id(),
        price_signal_match: allocate_stage_run_id(),
        gross_margin_gate: allocate_stage_run_id(),
        landed_cost: allocate_stage_run_id(),
        net_margin_gate: allocate_stage_run_id(),
        relate: allocate_stage_run_id(),
        manifest: allocate_stage_run_id(),
    }
}

#[cfg(test)]
fn live_loop_surface_names() -> Vec<&'static str> {
    LIVE_LOOP_STEP_SURFACES
        .iter()
        .map(|entry| entry.step)
        .collect()
}

fn repo_root() -> Result<PathBuf> {
    let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    crate_dir
        .ancestors()
        .nth(2)
        .map(Path::to_path_buf)
        .ok_or_else(|| anyhow!("failed to resolve repository root"))
}

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn default_fixture() -> LiveLoopFixture {
    LiveLoopFixture {
        cursor_before: Some("cursor-000".to_owned()),
        cursor_after: default_cursor_after(),
        continuation_required: false,
        fingerprint_state: default_fingerprint_state(),
        locale_state: default_locale_state(),
        currency_state: default_currency_state(),
        pty_transcript_lines: default_transcript_lines(),
        source_changed: false,
        notice_parse_failed: false,
        capture_write_failure: false,
        sqlite_write_failure: false,
        manifest_schema_invalid: false,
        manifest_oqg_suspect: false,
        validator_fail_open: false,
        validator_flagged: false,
        validator_rejected: false,
        validator_low_basis: false,
        extraction_fill_rate_bps: default_extraction_fill_rate_bps(),
        operator_requested: false,
        basis_hash_mismatch: false,
        goods_classification_low_confidence: false,
        supplier_match_low_confidence: false,
        commodity_code_low_confidence: false,
        revenue_basis_uncertain: false,
        fpo_access_unavailable: false,
        host_sleep_monotonic_gap_ms: 0,
        host_sleep_threshold_ms: default_host_sleep_threshold_ms(),
        host_cpu_pressure: false,
        host_memory_pressure: false,
        set_replacement_failure_stage: None,
        soft_stale_targets: Vec::new(),
        hard_invalid_targets: Vec::new(),
        notices: default_notices(),
    }
}

const fn default_extraction_fill_rate_bps() -> u16 {
    10_000
}

const fn default_host_sleep_threshold_ms() -> u64 {
    5_000
}

fn default_notices() -> Vec<FixtureNotice> {
    vec![FixtureNotice {
        process_id: "process-fts-001".to_owned(),
        notice_id: "notice-fts-001".to_owned(),
        source_notice_id: "ocds-001".to_owned(),
        source_url: "https://www.find-tender.service.gov.uk/notice/001".to_owned(),
        title: "Supply of industrial widgets".to_owned(),
        buyer_name: "Widgetshire Council".to_owned(),
        published_at: "2026-03-25T00:00:00Z".to_owned(),
        source_updated_at: "2026-03-25T00:30:00Z".to_owned(),
        ocid: Some("ocds-1234".to_owned()),
        procurement_stage: default_procurement_stage(),
        lots: default_fixture_lots(),
    }]
}

fn default_fixture_lots() -> Vec<FixtureLot> {
    vec![
        FixtureLot {
            lot_id: "lot-001".to_owned(),
            source_lot_id: Some("source-lot-001".to_owned()),
            title: "Widgets lot".to_owned(),
            candidate_count: 2,
            estimated_value_pence: default_lot_value(),
        },
        FixtureLot {
            lot_id: "lot-002".to_owned(),
            source_lot_id: Some("source-lot-002".to_owned()),
            title: "Replacement widgets".to_owned(),
            candidate_count: 1,
            estimated_value_pence: default_lot_value(),
        },
    ]
}

const fn default_candidate_count() -> u32 {
    1
}

fn default_cursor_after() -> String {
    "cursor-001".to_owned()
}

fn default_fingerprint_state() -> String {
    "matched".to_owned()
}

fn default_locale_state() -> String {
    "en-GB".to_owned()
}

fn default_currency_state() -> String {
    "GBP".to_owned()
}

fn default_procurement_stage() -> String {
    "tender".to_owned()
}

const fn default_lot_value() -> i64 {
    120_000
}

fn default_transcript_lines() -> Vec<String> {
    vec![
        "fingerprint matched".to_owned(),
        "notice batch checkpoint committed".to_owned(),
        "ingestion complete".to_owned(),
    ]
}

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
struct RuntimeIds {
    sid: u32,
    pid: u32,
    pgid: u32,
}

#[derive(Debug, Clone)]
struct StageRunIds {
    normalize: String,
    prequalify_notice: String,
    acquire_docs: String,
    extract_lots: String,
    qualify_lot: String,
    extract_products: String,
    score_quoteability: String,
    price_signal_match: String,
    gross_margin_gate: String,
    landed_cost: String,
    net_margin_gate: String,
    relate: String,
    manifest: String,
}

#[derive(Debug, Clone, Copy)]
struct UuidSeed(i128);

impl UuidSeed {
    fn from_time(now: DateTime<Utc>) -> Self {
        Self(now.timestamp_millis().into())
    }

    const fn as_bytes(self) -> [u8; 16] {
        self.0.to_be_bytes()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::fs;

    use anyhow::Result;
    use clap::Parser;
    use rusqlite::Connection;
    use rusqlite::params;
    use serde_json::{Value, json};
    use tempfile::tempdir;

    use super::agents::adapter::ExecutionMode;
    use super::agents::arbiter::{
        AdmissionPriority, AdmissionRequest, ArbiterDecision, ResourceArbiter, ResourceBudgets,
    };
    use super::jobs::runner::{CANONICAL_SCOPE_KINDS, JobRunner};
    use super::tracing::structured::{StructuredControlEventKind, StructuredTrace, TraceIdentity};
    use super::{
        ACTIVE_STAGES, Cli, ControlEvent, DEFAULT_AGENT_CLI, ExecutedSession, LOCAL_EXECUTOR_ID,
        LifecyclePhase, LiveLoopFixture, LiveValidationExecution, LoadedSkill,
        PolicyBasisSufficiency, ProcessValidationState, PtyRuntimeIds, SessionDraft,
        SkillFrontmatter, SkillPromotionStageMetadata, ValidationPolicyMode,
        ValidationStatusSnapshot, ValidationTrigger, append_runtime_control_records,
        backtrack_window_seconds, bootstrap_runtime, build_handshake_context,
        build_live_validation_policy_config, build_live_validation_policy_context,
        build_skill_run_telemetry, default_fixture, evaluate_validation_policy, insert_session_log,
        live_loop_surface_names, load_live_validation_job_parameters, observe_host_health,
        open_state_db, persisted_lifecycle_frames, reconcile_interrupted_sessions,
        run_live_semantic_validation, write_run_manifest_for_session,
    };
    use crate::db::sqlite::DEFAULT_SCHEMA_EPOCH;
    use crate::{persist_interest_registrations, persist_tender_enrichments};
    use chrono::Utc;

    type HistoryIdentityRow = (Option<i64>, Option<String>, Option<i64>);
    type CrashSessionRow = (
        String,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<i64>,
        Option<String>,
        Option<i64>,
    );

    fn insert_sync_source_job(conn: &Connection, job_id: &str, status: &str) -> Result<()> {
        conn.execute(
            "\
            INSERT INTO workflow_jobs (
              id, kind, scope_kind, scope_id, status, input_payload, validation_required,
              parent_job_id, validation_verdict_id
            ) VALUES (?1, 'sync_source', 'source', 'fts', ?2, ?3, 0, NULL, NULL)
            ",
            params![job_id, status, json!({"source_id": "fts"}).to_string()],
        )?;
        Ok(())
    }

    fn bootstrap_validation_runtime() -> Result<(tempfile::TempDir, super::RuntimeBootstrap)> {
        let temp = tempdir()?;
        let state_root = temp.path().join("state");
        let runtime_dir = temp.path().join("runtime");
        let artifact_root = temp.path().join("artifacts");
        let cli = Cli::parse_from([
            "daemon",
            "--serve",
            "--state-root",
            state_root.to_str().expect("utf8 state root"),
            "--runtime-dir",
            runtime_dir.to_str().expect("utf8 runtime dir"),
            "--artifact-root",
            artifact_root.to_str().expect("utf8 artifact root"),
        ]);
        let runtime = bootstrap_runtime(&cli)?;
        Ok((temp, runtime))
    }

    fn live_validation_test_skill(
        validation_policy: ValidationPolicyMode,
        validation_sample_rate: u16,
        validation_triggers: BTreeSet<ValidationTrigger>,
        worker_behavior_version: Option<&str>,
    ) -> LoadedSkill {
        LoadedSkill {
            skill_path: "skills/manifest-generation/SKILL.md".into(),
            skill_markdown: String::new(),
            metadata: SkillFrontmatter {
                skill_id: "manifest-generation".to_owned(),
                skill_version: "1.0.0".to_owned(),
                interaction_profile: "headless_contract".to_owned(),
                result_schema_version: "1.0.0".to_owned(),
                checkpoint_schema_version: None,
                validator_overlay_version: Some("2026.03.30-1".to_owned()),
                model_profile: "structured_extract_low_cost".to_owned(),
                agent_cli: Some(DEFAULT_AGENT_CLI.to_owned()),
                source_structure_version: None,
                worker_behavior_version: worker_behavior_version.map(str::to_owned),
                validator_behavior_version: None,
                validation_policy: Some(validation_policy),
                validation_sample_rate: Some(validation_sample_rate),
                validation_triggers: Some(validation_triggers),
                promotion_stage: SkillPromotionStageMetadata::Stable,
                version_change_affects_behavior: true,
                gate_a_family_key: None,
                gate_a_family_prequalified: true,
                gate_b_exact_slice_key: None,
                gate_b_exact_slice_ready: true,
            },
            result_schema: json!({}),
        }
    }

    fn insert_live_validation_worker_job(
        conn: &Connection,
        job_id: &str,
        source_id: &str,
        operator_requested: bool,
    ) -> Result<()> {
        conn.execute(
            "\
            INSERT INTO workflow_jobs (
              id, kind, scope_kind, scope_id, status, input_payload, validation_required,
              parent_job_id, validation_verdict_id
            ) VALUES (?1, 'sync_source', 'source', ?2, 'pending', ?3, 0, NULL, NULL)
            ",
            params![
                job_id,
                source_id,
                json!({
                    "source_id": source_id,
                    "operator_requested": operator_requested,
                })
                .to_string(),
            ],
        )?;
        Ok(())
    }

    fn seed_process(conn: &Connection, process_id: &str) -> Result<()> {
        conn.execute(
            "\
            INSERT INTO procurement_processes (
              id, current_stage, title, latest_published_at, latest_updated_at, owner_id
            ) VALUES (?1, 'manifest', 'Process title', '2026-03-30T00:00:00Z',
                      '2026-03-30T00:00:00Z', 'local-owner')
            ",
            params![process_id],
        )?;
        Ok(())
    }

    fn set_source_health(conn: &Connection, source_id: &str, health_state: &str) -> Result<()> {
        conn.execute(
            "UPDATE source_runtime_state SET last_health_state = ?1 WHERE source_id = ?2",
            params![health_state, source_id],
        )?;
        Ok(())
    }

    fn seed_completed_live_output(
        conn: &Connection,
        job_id: &str,
        source_id: &str,
        skill_id: &str,
        worker_behavior_version: &str,
        trust_state: &str,
        fill_rate_bps: u16,
    ) -> Result<()> {
        conn.execute(
            "\
            INSERT INTO workflow_jobs (
              id, kind, scope_kind, scope_id, status, input_payload, output_payload,
              validation_required, parent_job_id, validation_verdict_id
            ) VALUES (?1, 'sync_source', 'source', ?2, 'completed', ?3, ?4, 1, NULL, NULL)
            ",
            params![
                job_id,
                source_id,
                json!({"source_id": source_id}).to_string(),
                json!({
                    "source_id": source_id,
                    "trust_state": trust_state,
                    "skill_telemetry": [{
                        "skill_id": skill_id,
                        "worker_behavior_version": worker_behavior_version,
                        "validator_behavior_version": "2026.03.30-1",
                        "telemetry": {
                            "extraction_fill_rate_bps": fill_rate_bps,
                        },
                    }],
                })
                .to_string(),
            ],
        )?;
        Ok(())
    }

    fn seed_validation_verdict_row(
        conn: &Connection,
        worker_job_id: &str,
        basis_sufficiency: &str,
    ) -> Result<()> {
        let validator_job_id = format!("validator-{worker_job_id}");
        conn.execute(
            "\
            INSERT INTO workflow_jobs (
              id, kind, scope_kind, scope_id, status, input_payload, validation_required,
              parent_job_id, validation_verdict_id
            ) VALUES (?1, 'semantic_validate', 'process', 'process-fts-001', 'completed', '{}', 0, ?2, NULL)
            ",
            params![validator_job_id, worker_job_id],
        )?;
        conn.execute(
            "\
            INSERT INTO validation_verdicts (
              id, worker_job_id, validator_job_id, validator_skill_path, verdict, summary,
              basis_sufficiency, run_manifest_hash, verdict_rendered_at
            ) VALUES (?1, ?2, ?3, ?4, 'approved', ?5, ?6, ?7, ?8)
            ",
            params![
                format!("verdict-{worker_job_id}"),
                worker_job_id,
                validator_job_id,
                "skills/validation/base/SKILL.md",
                "seeded verdict",
                basis_sufficiency,
                format!("hash-{worker_job_id}"),
                "2026-03-30T12:00:00Z",
            ],
        )?;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn fixture_session_draft(
        session_id: &str,
        job_id: &str,
        status: &str,
        sid: Option<u32>,
        process_id: Option<u32>,
        process_group_id: Option<u32>,
        completed_at: &str,
        exit_code: Option<i32>,
        last_checkpoint_seq: Option<u32>,
    ) -> SessionDraft {
        SessionDraft {
            session_id: session_id.to_owned(),
            job_id: job_id.to_owned(),
            skill_path: "skills/procurement-ingestion/fts/SKILL.md".to_owned(),
            skill_version: "1.0.0".to_owned(),
            execution_mode: "pty".to_owned(),
            status: status.to_owned(),
            cli_vendor: Some("fixture-cli".to_owned()),
            cli_version: Some("1.0.0".to_owned()),
            adapter_id: Some(DEFAULT_AGENT_CLI.to_owned()),
            adapter_version: Some("daemon-test".to_owned()),
            config_hash: None,
            pty_session_id: Some(format!("pty-{session_id}")),
            sid,
            pid: process_id,
            pgid: process_group_id,
            started_at: "2026-03-25T11:00:00Z".to_owned(),
            completed_at: completed_at.to_owned(),
            terminal_output_path: None,
            terminal_output_hash: None,
            terminal_output_bytes: None,
            terminal_output_tail: None,
            structured_trace: None,
            error_type: None,
            error_message: None,
            exit_code,
            cleanup_verdict: None,
            last_checkpoint_seq,
        }
    }

    #[test]
    fn default_fixture_is_runnable() {
        let fixture = default_fixture();
        assert!(!fixture.notices.is_empty());
        assert_eq!(fixture.fingerprint_state, "matched");
    }

    #[test]
    fn active_stage_inventory_is_stable() {
        assert!(ACTIVE_STAGES.contains(&"Manifest"));
        assert_eq!(CANONICAL_SCOPE_KINDS.len(), 6);
    }

    #[test]
    fn durable_surface_map_has_fourteen_steps() {
        assert_eq!(live_loop_surface_names().len(), 14);
    }

    #[test]
    fn fixture_round_trips_through_json() -> Result<()> {
        let parsed: LiveLoopFixture = serde_json::from_value::<LiveLoopFixture>(Value::Object(
            serde_json::Map::from_iter([(
                "notices".to_owned(),
                serde_json::to_value(default_fixture().notices)?,
            )]),
        ))?;
        assert!(!parsed.cursor_after.is_empty());
        Ok(())
    }

    #[test]
    fn handshake_context_toggles_mutation_allowance_with_router_mode() -> Result<()> {
        let temp = tempdir()?;
        let state_root = temp.path().join("state");
        let runtime_dir = temp.path().join("runtime");
        let artifact_root = temp.path().join("artifacts");
        let cli = Cli::parse_from([
            "daemon",
            "--serve",
            "--state-root",
            state_root.to_str().expect("utf8 state root"),
            "--runtime-dir",
            runtime_dir.to_str().expect("utf8 runtime dir"),
            "--artifact-root",
            artifact_root.to_str().expect("utf8 artifact root"),
        ]);
        let runtime = bootstrap_runtime(&cli)?;

        let handshake_only = build_handshake_context(&runtime, false)?;
        let command_router_active = build_handshake_context(&runtime, true)?;

        assert!(!handshake_only.mutations_allowed);
        assert!(command_router_active.mutations_allowed);
        assert!(
            !handshake_only
                .operational_status
                .cron
                .definitions
                .is_empty()
        );
        assert!(
            !handshake_only
                .operational_status
                .source_health
                .sources
                .is_empty()
        );
        assert_eq!(
            handshake_only
                .operational_status
                .validation
                .validator_job_count,
            0
        );
        assert_eq!(
            handshake_only
                .operational_status
                .resource_arbiter
                .browser_slots_limit,
            2
        );
        assert_eq!(
            handshake_only
                .operational_status
                .resource_arbiter
                .agent_slots_limit,
            4
        );
        Ok(())
    }

    #[test]
    fn host_health_observation_prioritizes_sleep_and_taints_active_pty_sessions() {
        let mut fixture = default_fixture();
        fixture.host_sleep_monotonic_gap_ms = 10_000;
        fixture.host_sleep_threshold_ms = 5_000;
        fixture.host_cpu_pressure = true;

        let pty_health = observe_host_health(&fixture, ExecutionMode::Pty);
        assert_eq!(pty_health.issue.as_deref(), Some("HOST_SLEEP_INTERRUPTED"));
        assert!(pty_health.tainted_active_browser_session);
        assert!(!pty_health.blocks_new_admission);

        let contract_health = observe_host_health(&fixture, ExecutionMode::Contract);
        assert_eq!(
            contract_health.issue.as_deref(),
            Some("HOST_SLEEP_INTERRUPTED")
        );
        assert!(!contract_health.tainted_active_browser_session);
    }

    #[test]
    fn append_runtime_control_records_adds_resource_and_cleanup_events() -> Result<()> {
        let mut arbiter = ResourceArbiter::new(ResourceBudgets::default());
        let request = AdmissionRequest {
            job_id: "job-123".to_owned(),
            session_id: Some("session-123".to_owned()),
            priority: AdmissionPriority::ProcurementIngestion,
            browser_session: true,
            adapter_session: true,
            source_lane: None,
            validator_sub_job: false,
            concurrency_key: None,
            max_concurrency: None,
        };
        let lease = match arbiter.admit(&request) {
            ArbiterDecision::Granted(lease) => lease,
            ArbiterDecision::Denied(reason) => {
                anyhow::bail!("unexpected arbiter denial: {reason:?}")
            }
        };
        arbiter.release(&lease, super::ReleaseCause::Archived);

        let mut trace = StructuredTrace::new(ExecutionMode::Pty, ["created", "spawned", "active"]);
        trace.insert_metadata("probe", json!("ok"));
        let raw = trace.to_json_string()?;
        let updated = append_runtime_control_records(
            &raw,
            &TraceIdentity::new("job-123", "session-123"),
            ExecutionMode::Pty,
            &std::sync::Mutex::new(arbiter),
            Some(PtyRuntimeIds {
                sid: 10,
                pid: 11,
                pgid: 12,
            }),
            "clean",
        )?;
        let parsed = StructuredTrace::parse(&updated)?;
        assert!(
            parsed
                .control_event_records
                .iter()
                .any(|record| { record.event == StructuredControlEventKind::ResourceGranted })
        );
        assert!(
            parsed
                .control_event_records
                .iter()
                .any(|record| { record.event == StructuredControlEventKind::ResourceReleased })
        );
        assert!(
            parsed
                .control_event_records
                .iter()
                .any(|record| { record.event == StructuredControlEventKind::Reap })
        );
        assert!(
            parsed
                .control_event_records
                .iter()
                .any(|record| { record.event == StructuredControlEventKind::CleanupVerdict })
        );
        Ok(())
    }

    #[test]
    fn build_skill_run_telemetry_prefers_runtime_evidence_over_placeholders() -> Result<()> {
        let fixture = default_fixture();
        let skill = LoadedSkill {
            skill_path: "skills/manifest-generation/SKILL.md".into(),
            skill_markdown:
                "---\nskill_id: manifest-generation\nskill_version: 2026.03.30\ninteraction_profile: contract_required\nresult_schema_version: 1.0.0\nmodel_profile: structured_extract_low_cost\n---\nbody"
                    .to_owned(),
            metadata: SkillFrontmatter {
                skill_id: "manifest-generation".to_owned(),
                skill_version: "2026.03.30".to_owned(),
                interaction_profile: "contract_required".to_owned(),
                result_schema_version: "1.0.0".to_owned(),
                checkpoint_schema_version: None,
                validator_overlay_version: Some("2026.03.30-1".to_owned()),
                model_profile: "structured_extract_low_cost".to_owned(),
                agent_cli: None,
                source_structure_version: None,
                worker_behavior_version: None,
                validator_behavior_version: None,
                validation_policy: None,
                validation_sample_rate: None,
                validation_triggers: None,
                promotion_stage: SkillPromotionStageMetadata::Stable,
                version_change_affects_behavior: true,
                gate_a_family_key: None,
                gate_a_family_prequalified: false,
                gate_b_exact_slice_key: None,
                gate_b_exact_slice_ready: false,
            },
            result_schema: json!({}),
        };
        let mut trace = StructuredTrace::new(ExecutionMode::Contract, ["created", "active"]);
        trace.extend_from_adapter_events(
            &TraceIdentity::new("job-123", "session-123"),
            &[ControlEvent::AuthChallenge {
                kind: super::agents::adapter::AuthChallengeKind::ReLogin,
                detail: "reauth".to_owned(),
            }],
        );
        let executed = ExecutedSession {
            session_id: "session-123".to_owned(),
            started_at: chrono::DateTime::parse_from_rfc3339("2026-03-30T10:00:00Z")?
                .with_timezone(&Utc),
            execution_mode: ExecutionMode::Contract,
            adapter_id: DEFAULT_AGENT_CLI.to_owned(),
            cli_vendor: DEFAULT_AGENT_CLI.to_owned(),
            config_hash: "cfg".to_owned(),
            pty_session_id: None,
            runtime_ids: None,
            completed_at: "2026-03-30T10:00:02Z".to_owned(),
            terminal_output_path: None,
            terminal_output_hash: None,
            terminal_output_bytes: None,
            terminal_output_tail: None,
            structured_trace: trace.to_json_string()?,
            exit_code: 0,
            cleanup_verdict: "clean".to_owned(),
            last_checkpoint_seq: None,
            result_path: None,
            artifact_refs: Vec::new(),
        };
        let live_validation = LiveValidationExecution {
            validation_required: true,
            validator_job_id: Some("validator-job".to_owned()),
            validation_verdict_id: Some("verdict-id".to_owned()),
            trust_state: super::jobs::runner::ValidationTrustState::Verified,
            run_scoped_validation_rejection_rate_bps: Some(0),
            process_validation_state: ProcessValidationState::Validated,
            validation_marker: None,
            validation_telemetry: ValidationStatusSnapshot {
                validator_approval_rate_bps: 10_000,
                validator_rejection_rate_bps: 10_000,
                validator_flag_rate_bps: 0,
                validator_cost_per_run_pence: 0,
                operator_override_rate_bps: 0,
                validator_execution_failure_rate_bps: 0,
                validator_canary_pass_rate_bps: 10_000,
                validator_health_state: "healthy".to_owned(),
                pending_backlog_count: 0,
                pending_canary_count: 0,
                validator_job_count: 1,
            },
        };

        let telemetry =
            build_skill_run_telemetry(&skill, &executed, &fixture, Some(&live_validation))?;

        assert_eq!(telemetry.telemetry.cost_per_run_pence, None);
        assert_eq!(telemetry.telemetry.token_count, None);
        assert_eq!(telemetry.telemetry.retry_count, None);
        assert_eq!(telemetry.telemetry.auth_challenge_rate_bps, Some(10_000));
        assert_eq!(telemetry.telemetry.validation_rejection_rate_bps, Some(0));
        Ok(())
    }

    #[test]
    fn live_validation_skip_mode_without_trigger_skips_validator_enqueue() -> Result<()> {
        let (_temp, mut runtime) = bootstrap_validation_runtime()?;
        let runner = JobRunner::new(LOCAL_EXECUTOR_ID);
        let skill = live_validation_test_skill(
            ValidationPolicyMode::Skip,
            0,
            BTreeSet::from([ValidationTrigger::OperatorRequested]),
            None,
        );
        let fixture = default_fixture();
        insert_live_validation_worker_job(
            runtime.state_db.connection(),
            "worker-skip",
            "fts",
            false,
        )?;
        set_source_health(runtime.state_db.connection(), "fts", "green")?;
        let stored_manifest = write_run_manifest_for_session(
            &runtime,
            "worker-skip",
            "session-skip",
            &skill,
            "contract",
            Utc::now(),
            &fixture,
            3,
            None,
            Vec::new(),
        )?;

        let execution = run_live_semantic_validation(
            &mut runtime,
            &runner,
            "worker-skip",
            "session-skip",
            "process-fts-001",
            &skill,
            &stored_manifest,
            &fixture,
            3,
        )?;

        let (validation_required, validator_jobs): (i64, i64) = runtime
            .state_db
            .connection()
            .query_row(
                "\
                SELECT validation_required,
                       (SELECT COUNT(*) FROM workflow_jobs WHERE kind = 'semantic_validate' AND parent_job_id = ?1)
                FROM workflow_jobs
                WHERE id = ?1
                ",
                params!["worker-skip"],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )?;

        assert!(!execution.validation_required);
        assert_eq!(validation_required, 0);
        assert_eq!(validator_jobs, 0);
        assert!(execution.validation_verdict_id.is_none());
        Ok(())
    }

    #[test]
    fn live_validation_skip_mode_honors_operator_requested_override() -> Result<()> {
        let (_temp, mut runtime) = bootstrap_validation_runtime()?;
        let runner = JobRunner::new(LOCAL_EXECUTOR_ID);
        let skill = live_validation_test_skill(
            ValidationPolicyMode::Skip,
            0,
            BTreeSet::from([ValidationTrigger::OperatorRequested]),
            None,
        );
        let fixture = default_fixture();
        insert_live_validation_worker_job(
            runtime.state_db.connection(),
            "worker-override",
            "fts",
            true,
        )?;
        seed_process(runtime.state_db.connection(), "process-fts-001")?;
        set_source_health(runtime.state_db.connection(), "fts", "green")?;
        let stored_manifest = write_run_manifest_for_session(
            &runtime,
            "worker-override",
            "session-override",
            &skill,
            "contract",
            Utc::now(),
            &fixture,
            3,
            None,
            Vec::new(),
        )?;

        let execution = run_live_semantic_validation(
            &mut runtime,
            &runner,
            "worker-override",
            "session-override",
            "process-fts-001",
            &skill,
            &stored_manifest,
            &fixture,
            3,
        )?;

        let (validation_required, validator_jobs): (i64, i64) = runtime
            .state_db
            .connection()
            .query_row(
                "\
                SELECT validation_required,
                       (SELECT COUNT(*) FROM workflow_jobs WHERE kind = 'semantic_validate' AND parent_job_id = ?1)
                FROM workflow_jobs
                WHERE id = ?1
                ",
                params!["worker-override"],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )?;

        assert!(execution.validation_required);
        assert_eq!(validation_required, 1);
        assert_eq!(validator_jobs, 1);
        assert!(execution.validation_verdict_id.is_some());
        Ok(())
    }

    #[test]
    fn live_validation_sample_mode_amber_source_health_forces_full_probability() -> Result<()> {
        let (_temp, mut runtime) = bootstrap_validation_runtime()?;
        let runner = JobRunner::new(LOCAL_EXECUTOR_ID);
        let skill = live_validation_test_skill(
            ValidationPolicyMode::Sample,
            250,
            BTreeSet::from([ValidationTrigger::SourceHealthAmberOrRed]),
            None,
        );
        let fixture = default_fixture();
        insert_live_validation_worker_job(
            runtime.state_db.connection(),
            "worker-amber",
            "fts",
            false,
        )?;
        seed_process(runtime.state_db.connection(), "process-fts-001")?;
        set_source_health(runtime.state_db.connection(), "fts", "amber")?;
        let stored_manifest = write_run_manifest_for_session(
            &runtime,
            "worker-amber",
            "session-amber",
            &skill,
            "contract",
            Utc::now(),
            &fixture,
            3,
            None,
            Vec::new(),
        )?;
        let worker_job_parameters =
            load_live_validation_job_parameters(runtime.state_db.connection(), "worker-amber")?;
        let policy_config = build_live_validation_policy_config(&skill, &worker_job_parameters);
        let policy_context = build_live_validation_policy_context(
            runtime.state_db.connection(),
            "worker-amber",
            &skill,
            &stored_manifest,
            &fixture,
        )?;
        let policy_decision = evaluate_validation_policy(&policy_config, &policy_context);

        assert!(policy_decision.validation_required);
        assert_eq!(policy_decision.effective_validation_probability_bps, 10_000);

        let execution = run_live_semantic_validation(
            &mut runtime,
            &runner,
            "worker-amber",
            "session-amber",
            "process-fts-001",
            &skill,
            &stored_manifest,
            &fixture,
            3,
        )?;

        assert!(execution.validation_required);
        Ok(())
    }

    #[test]
    fn live_validation_context_derives_history_signals_from_persisted_runtime_state() -> Result<()>
    {
        let (_temp, runtime) = bootstrap_validation_runtime()?;
        let skill = live_validation_test_skill(
            ValidationPolicyMode::Sample,
            250,
            BTreeSet::from([
                ValidationTrigger::FillRateDrop,
                ValidationTrigger::FirstRunAfterVersionChange,
                ValidationTrigger::SuspectOutputInLastFive,
            ]),
            Some("manifest-generation@2026.03.31-1+1.0.0"),
        );
        let mut fixture = default_fixture();
        fixture.extraction_fill_rate_bps = 8_000;

        insert_live_validation_worker_job(
            runtime.state_db.connection(),
            "worker-history",
            "fts",
            false,
        )?;
        seed_completed_live_output(
            runtime.state_db.connection(),
            "worker-history-old-1",
            "fts",
            &skill.metadata.skill_id,
            "manifest-generation@2026.03.30-1+1.0.0",
            "suspect",
            9_500,
        )?;
        seed_validation_verdict_row(runtime.state_db.connection(), "worker-history-old-1", "low")?;
        seed_completed_live_output(
            runtime.state_db.connection(),
            "worker-history-old-2",
            "fts",
            &skill.metadata.skill_id,
            "manifest-generation@2026.03.30-1+1.0.0",
            "verified",
            10_000,
        )?;
        let stored_manifest = write_run_manifest_for_session(
            &runtime,
            "worker-history",
            "session-history",
            &skill,
            "contract",
            Utc::now(),
            &fixture,
            3,
            None,
            Vec::new(),
        )?;
        let worker_job_parameters =
            load_live_validation_job_parameters(runtime.state_db.connection(), "worker-history")?;
        let policy_config = build_live_validation_policy_config(&skill, &worker_job_parameters);
        let policy_context = build_live_validation_policy_context(
            runtime.state_db.connection(),
            "worker-history",
            &skill,
            &stored_manifest,
            &fixture,
        )?;
        let policy_decision = evaluate_validation_policy(&policy_config, &policy_context);

        assert!(policy_context.first_run_after_worker_version_change);
        assert_eq!(
            policy_context.recent_basis_sufficiency,
            Some(PolicyBasisSufficiency::Low)
        );
        assert_eq!(policy_context.suspect_output_count_last_five, 1);
        assert_eq!(policy_context.fill_rate_drop_bps, Some(1_750));
        assert!(policy_decision.validation_required);
        Ok(())
    }

    #[test]
    fn backtrack_window_is_positive() {
        assert!(backtrack_window_seconds() > 0);
    }

    #[test]
    fn persistent_serve_mode_conflicts_with_one_shot_handshake_flag() {
        let parsed = super::Cli::try_parse_from(["daemon", "--serve", "--serve-once"]);
        assert!(parsed.is_err());
    }

    #[test]
    fn persisted_lifecycle_frames_reflect_reaped_and_archived_metadata() -> Result<()> {
        let temp = tempdir()?;
        let db_path = temp.path().join("state.db");
        let state_db = open_state_db(&db_path, DEFAULT_SCHEMA_EPOCH)?;
        insert_sync_source_job(state_db.connection(), "job-history", "completed")?;
        let mut session_history = fixture_session_draft(
            "session-history",
            "job-history",
            "failed",
            Some(101),
            Some(101),
            Some(101),
            "2026-03-25T11:00:01Z",
            Some(0),
            Some(3),
        );
        session_history.structured_trace = Some(
            json!({
                "phase_history": [
                    "created",
                    "spawned",
                    "active",
                    "terminating",
                    "exited",
                    "reaped",
                    "archived"
                ]
            })
            .to_string(),
        );
        session_history.error_type = Some("AGENT_DAEMON_CRASH_DURING_SESSION".to_owned());
        session_history.error_message = Some("recovered".to_owned());
        session_history.cleanup_verdict = Some("clean".to_owned());
        insert_session_log(state_db.connection(), &session_history)?;
        let (sid, adapter_id, exit_code): HistoryIdentityRow = state_db.connection().query_row(
            "SELECT sid, adapter_id, exit_code FROM agent_session_logs WHERE id = 'session-history'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )?;
        assert_eq!(sid, Some(101));
        assert_eq!(adapter_id.as_deref(), Some(DEFAULT_AGENT_CLI));
        assert_eq!(exit_code, Some(0));

        let frames = persisted_lifecycle_frames(state_db.connection())?;
        let phases: Vec<LifecyclePhase> = frames.iter().map(|frame| frame.phase).collect();
        assert_eq!(
            phases,
            vec![
                LifecyclePhase::Created,
                LifecyclePhase::Spawned,
                LifecyclePhase::Active,
                LifecyclePhase::Terminating,
                LifecyclePhase::Exited,
                LifecyclePhase::Reaped,
                LifecyclePhase::Archived,
            ]
        );
        let reaped = frames
            .iter()
            .find(|frame| frame.phase == LifecyclePhase::Reaped)
            .expect("reaped frame");
        assert_eq!(reaped.cleanup_verdict.as_deref(), Some("clean"));
        let archived = frames
            .iter()
            .find(|frame| frame.phase == LifecyclePhase::Archived)
            .expect("archived frame");
        assert!(archived.archive_seq.is_some());
        Ok(())
    }

    #[test]
    fn startup_reconciliation_marks_interrupted_sessions_failed() -> Result<()> {
        let temp = tempdir()?;
        let state_root = temp.path().join("state");
        let artifact_root = temp.path().join("artifacts");
        fs::create_dir_all(&state_root)?;
        fs::create_dir_all(&artifact_root)?;
        let state_db = open_state_db(state_root.join("state.db"), DEFAULT_SCHEMA_EPOCH)?;
        insert_sync_source_job(state_db.connection(), "job-crash", "running")?;
        let session_crash = fixture_session_draft(
            "session-crash",
            "job-crash",
            "active",
            Some(999_999),
            Some(999_999),
            Some(999_999),
            "2026-03-25T11:00:01Z",
            None,
            Some(4),
        );
        insert_session_log(state_db.connection(), &session_crash)?;

        reconcile_interrupted_sessions(&state_db, &artifact_root, "daemon-instance-test")?;

        let (status, error_type, cleanup_verdict, structured_trace, sid, adapter_id, exit_code): CrashSessionRow =
            state_db.connection().query_row(
            "\
            SELECT status, error_type, cleanup_verdict, structured_trace, sid, adapter_id, exit_code
            FROM agent_session_logs WHERE id = 'session-crash'
            ",
            [],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                ))
            },
        )?;
        assert_eq!(status, "failed");
        assert_eq!(
            error_type.as_deref(),
            Some("AGENT_DAEMON_CRASH_DURING_SESSION")
        );
        assert_eq!(cleanup_verdict.as_deref(), Some("clean"));
        assert_eq!(sid, Some(999_999));
        assert_eq!(adapter_id.as_deref(), Some(DEFAULT_AGENT_CLI));
        assert_eq!(exit_code, None);
        let trace: Value = serde_json::from_str(
            structured_trace
                .as_deref()
                .expect("reconciled trace should be present"),
        )?;
        assert_eq!(trace["crash_recovered"], Value::Bool(true));
        assert_eq!(
            trace["phase_history"],
            json!([
                "created",
                "spawned",
                "active",
                "terminating",
                "exited",
                "reaped",
                "archived"
            ])
        );

        let frames = persisted_lifecycle_frames(state_db.connection())?;
        assert!(frames.iter().any(|frame| {
            frame.session_id == "session-crash"
                && frame.phase == LifecyclePhase::Reaped
                && frame.cleanup_verdict.as_deref() == Some("clean")
        }));
        assert!(frames.iter().any(|frame| {
            frame.session_id == "session-crash"
                && frame.phase == LifecyclePhase::Archived
                && frame.archive_seq.is_some()
        }));
        Ok(())
    }

    #[test]
    fn open_state_db_backfills_new_agent_session_log_columns_on_existing_db() -> Result<()> {
        let temp = tempdir()?;
        let db_path = temp.path().join("state.db");
        let conn = rusqlite::Connection::open(&db_path)?;
        conn.execute_batch(
            "\
            CREATE TABLE agent_session_logs (
              id TEXT PRIMARY KEY,
              job_id TEXT REFERENCES workflow_jobs(id),
              skill_path TEXT NOT NULL,
              skill_version TEXT,
              execution_mode TEXT NOT NULL,
              cli_vendor TEXT,
              cli_version TEXT,
              adapter_version TEXT,
              config_hash TEXT,
              pty_session_id TEXT,
              pid INTEGER,
              pgid INTEGER,
              started_at TEXT,
              completed_at TEXT,
              status TEXT NOT NULL,
              terminal_output_path TEXT,
              terminal_output_hash TEXT,
              terminal_output_bytes INTEGER,
              terminal_output_tail TEXT,
              structured_trace TEXT,
              error_type TEXT,
              error_message TEXT,
              cleanup_verdict TEXT,
              run_manifest_path TEXT,
              run_manifest_hash TEXT,
              last_checkpoint_seq INTEGER,
              created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );
            ",
        )?;
        drop(conn);

        let state_db = open_state_db(&db_path, DEFAULT_SCHEMA_EPOCH)?;
        let mut stmt = state_db
            .connection()
            .prepare("PRAGMA table_info(agent_session_logs)")?;
        let columns: Vec<String> = stmt
            .query_map([], |row| row.get::<_, String>(1))?
            .collect::<std::result::Result<_, _>>()?;
        assert!(columns.iter().any(|column| column == "adapter_id"));
        assert!(columns.iter().any(|column| column == "sid"));
        assert!(columns.iter().any(|column| column == "exit_code"));
        Ok(())
    }

    fn seed_minimal_procontract_notice(
        conn: &Connection,
        process_id: &str,
        notice_id: &str,
        title: &str,
    ) -> Result<()> {
        conn.execute(
            "\
            INSERT INTO procurement_processes (
              id, canonical_ocid, primary_notice_id, current_stage, buyer_name, title,
              latest_deadline_at, latest_published_at, latest_updated_at, owner_id
            ) VALUES (?1, ?2, ?3, 'normalize', 'Buyer', ?4, '2026-05-31T12:00:00Z',
                      '2026-04-01T00:00:00Z', '2026-04-01T00:00:00Z', 'local-owner')
            ",
            params![process_id, notice_id, notice_id, title],
        )?;
        conn.execute(
            "\
            INSERT INTO contract_notices (
              id, process_id, source_id, source_notice_id, raw_document_id, ocid, title,
              summary, buyer_name, procurement_stage, access_route, deadline_at, published_at,
              source_updated_at, main_category, normalized_payload, owner_id
            ) VALUES (?1, ?2, 'procontract', ?3, ?4, ?5, ?6, 'seed summary', 'Buyer',
                      'tender', 'portal_authenticated', '2026-05-31T12:00:00Z',
                      '2026-04-01T00:00:00Z', '2026-04-01T00:00:00Z', 'seed category',
                      '{}', 'local-owner')
            ",
            params![
                notice_id,
                process_id,
                notice_id,
                format!("raw-{notice_id}"),
                notice_id,
                title,
            ],
        )?;
        Ok(())
    }

    #[test]
    fn persist_interest_registrations_writes_stage_tracking_rows() -> Result<()> {
        let temp = tempdir()?;
        let db_path = temp.path().join("state.db");
        let mut state_db = open_state_db(&db_path, DEFAULT_SCHEMA_EPOCH)?;
        seed_minimal_procontract_notice(
            state_db.connection(),
            "proc-pc-test-001",
            "pc-test-001",
            "Supply of widgets",
        )?;

        let result_json = json!({
            "cursor": "{\"last_processed_at\":\"2026-04-13T15:00:00Z\"}",
            "continuation_required": false,
            "registrations": [
                {
                    "notice_id": "pc-test-001",
                    "process_id": "proc-pc-test-001",
                    "source_url": "https://example.invalid/pc-test-001",
                    "action": "registered",
                    "registered_at": "2026-04-13T15:02:30Z",
                    "pre_filter_result": {
                        "goods_check": true,
                        "expiry_check": true,
                        "category": "Goods",
                        "days_until_deadline": 45
                    }
                }
            ],
            "summary": {
                "total_evaluated": 1,
                "registered": 1,
                "already_registered": 0,
                "skipped_service": 0,
                "skipped_expiry": 0,
                "failed": 0
            }
        });

        let persisted =
            persist_interest_registrations(&mut state_db, "procontract-interest", &result_json)?;
        assert_eq!(persisted, 1);

        let state: String = state_db.connection().query_row(
            "\
            SELECT state
            FROM process_stage_status
            WHERE process_id = 'proc-pc-test-001' AND stage = 'interest_registration'
            ORDER BY created_at DESC, id DESC
            LIMIT 1
            ",
            [],
            |row| row.get(0),
        )?;
        assert_eq!(state, "completed");

        let head: String = state_db.connection().query_row(
            "\
            SELECT current_stage_run_id
            FROM process_stage_heads
            WHERE process_id = 'proc-pc-test-001' AND stage = 'interest_registration'
            ",
            [],
            |row| row.get(0),
        )?;
        assert!(head.contains("stage-run-"));
        Ok(())
    }

    #[test]
    fn persist_tender_enrichments_updates_notice_and_documents() -> Result<()> {
        let temp = tempdir()?;
        let db_path = temp.path().join("state.db");
        let mut state_db = open_state_db(&db_path, DEFAULT_SCHEMA_EPOCH)?;
        seed_minimal_procontract_notice(
            state_db.connection(),
            "proc-pc-test-002",
            "pc-test-002",
            "Supply of signs",
        )?;

        let pdf_path = temp.path().join("sample.pdf");
        let docx_path = temp.path().join("sample.docx");
        fs::write(&pdf_path, b"%PDF-1.4\n%seed pdf\n")?;
        fs::write(&docx_path, b"seed docx bytes")?;

        let result_json = json!({
            "cursor": "{\"last_checked_at\":\"2026-04-13T16:00:00Z\"}",
            "continuation_required": false,
            "enriched_tenders": [
                {
                    "notice_id": "pc-test-002",
                    "process_id": "proc-pc-test-002",
                    "rfx_id": "rfx-test-002",
                    "rfx_url": "https://procontract.due-north.com/RfxResponse/Index?rfxId=rfx-test-002",
                    "updated_title": "Supply of signs",
                    "updated_description": "Updated tender description",
                    "updated_deadline_at": "2026-06-01T12:00:00Z",
                    "buyer_name": "Buyer",
                    "buyer_contact": {
                        "name": "Alex Buyer",
                        "email": "alex@example.invalid"
                    },
                    "documents_downloaded": [
                        {
                            "filename": "Invitation.pdf",
                            "content_type": "application/pdf",
                            "size_bytes": 18,
                            "local_path": pdf_path,
                            "document_role_hint": "invitation_to_tender"
                        },
                        {
                            "filename": "Response.docx",
                            "content_type": "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
                            "size_bytes": 15,
                            "local_path": docx_path,
                            "document_role_hint": "response_document"
                        }
                    ],
                    "documents_count": 2,
                    "enriched_at": "2026-04-13T16:05:00Z"
                }
            ],
            "summary": {
                "total_checked": 1,
                "enriched": 1,
                "not_yet_responded": 0,
                "failed": 0
            }
        });

        let persisted =
            persist_tender_enrichments(&mut state_db, "procontract-enrich", &result_json)?;
        assert_eq!(persisted, 1);

        let (summary, deadline_at): (String, String) = state_db.connection().query_row(
            "\
            SELECT summary, deadline_at
            FROM contract_notices
            WHERE id = 'pc-test-002'
            ",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        assert_eq!(summary, "Updated tender description");
        assert_eq!(deadline_at, "2026-06-01T12:00:00Z");

        let raw_document_count: i64 = state_db.connection().query_row(
            "SELECT COUNT(*) FROM raw_documents WHERE source_id = 'procontract-enrich'",
            [],
            |row| row.get(0),
        )?;
        assert_eq!(raw_document_count, 2);

        let extraction_state: String = state_db.connection().query_row(
            "SELECT extraction_state FROM tender_document_sets WHERE id = 'tds-pc-test-002'",
            [],
            |row| row.get(0),
        )?;
        assert_eq!(extraction_state, "pending");
        Ok(())
    }

    /// Regression: `run_bound_skill` used to mark procurement ingestion jobs
    /// as `completed` even when the nested agent failed upstream and never
    /// produced a result contract. The guard now in place asks
    /// `procurement_skill_requires_result_contract` whether a missing
    /// result file should promote to a hard failure. Non-procurement skills
    /// (pdf-extraction/*, manifest-generation, etc.) can legitimately finish
    /// without a result file, so they must continue returning `false`.
    #[test]
    fn procurement_skills_require_result_contract_to_complete() {
        assert!(super::procurement_skill_requires_result_contract(
            "procurement-ingestion/procontract-discover"
        ));
        assert!(super::procurement_skill_requires_result_contract(
            "procurement-ingestion/procontract-register-interest"
        ));
        assert!(super::procurement_skill_requires_result_contract(
            "procurement-ingestion/procontract-enrich-tender"
        ));
        assert!(!super::procurement_skill_requires_result_contract(
            "pdf-extraction/extract-document-semantics"
        ));
        assert!(!super::procurement_skill_requires_result_contract(
            "manifest-generation"
        ));
        assert!(!super::procurement_skill_requires_result_contract(""));
    }

    /// Regression: verifies that when a procurement ingestion skill produces
    /// no result contract, the failure helper used by `run_bound_skill`
    /// transitions the workflow job from `running` to `failed` with
    /// `AGENT_RUN_FAILED` context. Before the fix, the job was
    /// unconditionally marked `completed`, hiding upstream agent crashes.
    #[test]
    fn missing_procurement_result_contract_marks_job_failed() -> Result<()> {
        let (_temp, mut runtime) = bootstrap_validation_runtime()?;
        let job_id = "job-regression-missing-result";
        runtime.state_db.connection().execute(
            "\
            INSERT INTO workflow_jobs (
              id, kind, scope_kind, scope_id, status, input_payload, validation_required,
              parent_job_id, validation_verdict_id
            ) VALUES (?1, 'sync_source', 'source', 'procontract', 'running',
                      ?2, 0, NULL, NULL)
            ",
            params![job_id, json!({"source_id": "procontract"}).to_string()],
        )?;

        let report = super::fail_with_manifest(
            &mut runtime,
            job_id,
            super::ErrorCode::AgentRunFailed,
            "procurement skill `procurement-ingestion/procontract-discover` exited without writing a result contract",
            None,
            None,
            None,
            "ingest",
            "agent_execute",
            json!({"skill_id": "procurement-ingestion/procontract-discover"}),
        )?;
        assert_eq!(report.status, "failed");

        let (status, last_error): (String, Option<String>) =
            runtime.state_db.connection().query_row(
                "SELECT status, last_error FROM workflow_jobs WHERE id = ?1",
                params![job_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )?;
        assert_eq!(status, "failed");
        let last_error = last_error.unwrap_or_default();
        assert!(
            last_error.contains("without writing a result contract"),
            "expected silent-success regression message, got: {last_error}"
        );

        let manifest_rows: i64 = runtime.state_db.connection().query_row(
            "SELECT COUNT(*) FROM error_manifests WHERE job_id = ?1",
            params![job_id],
            |row| row.get(0),
        )?;
        assert!(
            manifest_rows >= 1,
            "expected at least one error manifest row for failed procurement job"
        );
        Ok(())
    }
}

#[cfg(test)]
const fn backtrack_window_seconds() -> u32 {
    BACKTRACK_WINDOW_SECONDS
}
