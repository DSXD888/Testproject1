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
    pub mod session;
}

#[allow(dead_code)]
mod prompt {
    pub mod renderer;
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

use agents::adapter::{
    AgentAdapter, CapabilityReport, ControlEvent, ExecutionMode, InteractionProfile, LaunchSpec,
    OutputFrame, ProbeCtx, SkillManifest, SpawnRequest,
};
use agents::arbiter::{
    AdmissionPriority, AdmissionRequest, ArbiterDecision, ReleaseCause, ResourceArbiter,
    ResourceBudgets, ResourceLease, SourceKind, SourceLaneRequest,
};
use agents::artifacts::{
    ArtifactStore, DaemonEvidenceAppendix, OutcomeClass, RunManifestFrontmatter, RunNarrative,
    persist_run_manifest_metadata,
};
use agents::claude::ClaudeAdapter;
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
    ActiveSessions, HandshakeContext, HealthState, LifecycleFrame, LifecyclePhase,
};
use jobs::runner::{
    EnqueuedValidatorJob, FreshnessState, JobLeaseHeartbeat, JobRunner, ObsoleteJobRequest,
    ValidatorEnqueueRequest, allocate_stage_run_id, set_stage_head_freshness,
    start_job_lease_heartbeat,
};
use nix::unistd::{Pid, getpgid, getsid};
use prompt::renderer::{PromptRenderer, PromptRendererInput};
use pty::capture::{CaptureConfig, PtyCapture, compressed_payload_sha256};
use pty::session::{PtyCommand, PtySession, RuntimeIds as PtyRuntimeIds, SpawnOptions, SpawnPath};
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use scheduler::cron::CronScheduler;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sessions::reaper::{CrashRecoveryContext, SessionReaper};
use sha2::{Digest, Sha256};
use sources::registry::SourceRegistry;
use tracing::forensic::{
    ForensicCaptureSummary, ForensicIdentity, ForensicPtyCapture, capture_contract_output,
    summarize_contract_capture,
};
use tracing::structured::{StructuredTrace, TraceIdentity, parse_phase_history};
use tracing_subscriber::EnvFilter;
use validation::executor::VALIDATOR_JOB_KIND;
use validation::manifest_writer::{ManifestWriteRequest, write_hybrid_run_manifest};

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
        conflicts_with_all = ["serve_once", "ingest", "skill_path", "dry_run"]
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
        ensure!(source_id == "fts", "only `fts` is supported in Layer 1");
        let fixture = load_fixture(cli.fixture_path.as_deref())?;
        let report = run_fts_live_loop(&mut runtime, fixture, &cli).await?;
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
        mutations_allowed: command_router_active,
        expected_peer_uid: None,
        lifecycle_frames: persisted_lifecycle_frames(runtime.state_db.connection())?,
    })
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

#[allow(clippy::future_not_send)]
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

    let result = match mode {
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
        result_path: None,
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
    let result_path = events.iter().find_map(|event| match event {
        ControlEvent::FinalResultReady { path } => Some(path.clone()),
        _ => None,
    });
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
    state_db.with_write_transaction(|tx| {
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

    let contract_execution = execute_skill_session(
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
            let mut refs = contract_execution.artifact_refs;
            refs.push(result_artifact_path.to_string_lossy().into_owned());
            refs
        },
    )?;
    persist_run_manifest_metadata(
        runtime.state_db.connection(),
        &contract_session_id,
        &contract_run_manifest,
    )?;

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
    state_db.with_write_transaction(|tx| {
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
fn finalize_success(
    runtime: &mut RuntimeBootstrap,
    job_id: &str,
    pipeline: &PipelinePersistence,
    manifest: &ManifestComputation,
    manifest_path: &Path,
    notices_ingested: usize,
) -> Result<LiveLoopReport> {
    let summary_payload = json!({
        "job_id": job_id,
        "source_id": "fts",
        "process_id": pipeline.process_id,
        "notices_ingested": notices_ingested,
        "candidate_count": manifest.candidate_count,
        "manifest_path": manifest_path.to_string_lossy(),
        "review_required": pipeline.review_required,
        "review_reason": pipeline.review_reason,
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
            ) VALUES (?1, 'manifest_ready', ?2, 'unverified', ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
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
                i64::from(pipeline.review_required),
                pipeline.gross_margin_bps,
                pipeline.net_margin_bps,
                if pipeline.review_required {
                    Some("manifest".to_owned())
                } else {
                    None::<String>
                },
                pipeline.review_reason,
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
        extraction_fill_rate_bps: Some(10_000),
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
        basis_hash_mismatch: false,
        goods_classification_low_confidence: false,
        supplier_match_low_confidence: false,
        commodity_code_low_confidence: false,
        revenue_basis_uncertain: false,
        fpo_access_unavailable: false,
        set_replacement_failure_stage: None,
        soft_stale_targets: Vec::new(),
        hard_invalid_targets: Vec::new(),
        notices: default_notices(),
    }
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
    use std::fs;

    use anyhow::Result;
    use clap::Parser;
    use rusqlite::Connection;
    use rusqlite::params;
    use serde_json::{Value, json};
    use tempfile::tempdir;

    use super::jobs::runner::CANONICAL_SCOPE_KINDS;
    use super::{
        ACTIVE_STAGES, Cli, DEFAULT_AGENT_CLI, LifecyclePhase, LiveLoopFixture, SessionDraft,
        backtrack_window_seconds, bootstrap_runtime, build_handshake_context, default_fixture,
        insert_session_log, live_loop_surface_names, open_state_db, persisted_lifecycle_frames,
        reconcile_interrupted_sessions,
    };
    use crate::db::sqlite::DEFAULT_SCHEMA_EPOCH;

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
}

#[cfg(test)]
const fn backtrack_window_seconds() -> u32 {
    BACKTRACK_WINDOW_SECONDS
}
