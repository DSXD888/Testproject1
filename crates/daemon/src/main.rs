#![forbid(unsafe_code)]

mod db {
    pub mod sqlite;
}

mod ipc {
    pub mod descriptor;
    pub mod handshake;
}

#[allow(dead_code)]
mod agents {
    pub mod adapter;
    pub mod artifacts;
    pub mod contracts;
}

#[allow(dead_code)]
mod errors {
    pub mod manifest;
    pub mod markdown;
}

#[allow(dead_code)]
mod jobs {
    pub mod runner;
}

#[allow(dead_code)]
mod pty {
    pub mod capture;
}

#[allow(dead_code)]
mod sessions {
    pub mod reaper;
}

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

use std::convert::TryFrom;
use std::fs;
use std::path::{Path, PathBuf};
use std::process;

use agents::adapter::{
    CapabilityReport, ExecutionMode, InteractionProfile, SkillManifest, resolve_execution_mode,
};
use agents::artifacts::{
    ArtifactStore, DaemonEvidenceAppendix, OutcomeClass, RunManifestDraft, RunManifestFrontmatter,
    RunNarrative, persist_run_manifest_metadata,
};
use agents::contracts::{
    OqgCheckInput, StageValidationWrite, ValidationDisposition, ValidationOutcome,
    persist_stage_validation, validate_final_result,
};
use anyhow::{Context, Result, anyhow, bail, ensure};
use chrono::{DateTime, Utc};
use clap::Parser;
use db::sqlite::{DEFAULT_OWNER_ID, DEFAULT_SCHEMA_EPOCH, StateDb, open_state_db};
use errors::manifest::{ErrorCode, ErrorManifestDraft, ErrorManifestStore, ErrorRuntimeContext};
use ipc::descriptor::{
    DaemonDescriptor, LockGuard, RuntimePaths, bind_listener, write_descriptor_atomic,
};
use ipc::handshake::{
    ActiveSessions, HandshakeContext, HealthState, LifecycleFrame, LifecyclePhase,
};
use jobs::runner::{
    FreshnessState, JobRunner, ObsoleteJobRequest, allocate_stage_run_id, set_stage_head_freshness,
};
use nix::unistd::{Pid, getpgid};
use pty::capture::{CaptureConfig, PtyCapture, compressed_payload_sha256};
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sessions::reaper::{CrashRecoveryContext, SessionReaper};
use sha2::{Digest, Sha256};
use tracing_subscriber::EnvFilter;

const BACKTRACK_WINDOW_SECONDS: u32 = 60;
const CLI_VENDOR: &str = "fixture-cli";
const CLI_VERSION: &str = "1.0.0";
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

    /// When enabled, accept exactly one handshake before exiting.
    #[arg(long, default_value_t = false)]
    serve_once: bool,

    /// Run the manual live-loop trigger for the declared source id.
    #[arg(long)]
    ingest: Option<String>,

    /// Fixture file used by tests to drive deterministic live-loop execution.
    #[arg(long, hide = true)]
    fixture_path: Option<PathBuf>,
}

struct RuntimeBootstrap {
    _lock: LockGuard,
    listener: std::os::unix::net::UnixListener,
    descriptor: DaemonDescriptor,
    state_db: StateDb,
    artifact_root: PathBuf,
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
    model_profile: String,
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
    adapter_version: Option<String>,
    config_hash: Option<String>,
    pty_session_id: Option<String>,
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
    cleanup_verdict: Option<String>,
    last_checkpoint_seq: Option<u32>,
}

#[derive(Debug, Clone)]
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

    let mut runtime = bootstrap_runtime(&cli)?;

    if cli.serve_once {
        serve_handshake_once(&runtime)?;
        return Ok(());
    }

    if let Some(source_id) = &cli.ingest {
        ensure!(source_id == "fts", "only `fts` is supported in Layer 1");
        let fixture = load_fixture(cli.fixture_path.as_deref())?;
        let report = run_fts_live_loop(&mut runtime, fixture)?;
        if report.status == "completed" {
            println!("{}", report.summary_line);
            return Ok(());
        }

        eprintln!("{}", report.summary_line);
        bail!(report.summary_line);
    }

    if cli.dry_run {
        tracing::info!("dry-run bootstrap completed");
    }

    Ok(())
}

fn bootstrap_runtime(cli: &Cli) -> Result<RuntimeBootstrap> {
    let runtime_paths = RuntimePaths::new(cli.state_root.clone(), cli.runtime_dir.clone());
    let lock_guard = LockGuard::acquire(&runtime_paths)?;
    let state_db = open_state_db(runtime_paths.state_db_path(), DEFAULT_SCHEMA_EPOCH)?;
    let listener = bind_listener(&runtime_paths)?;
    let descriptor = DaemonDescriptor::new(
        runtime_paths.socket_path(),
        runtime_paths.state_root(),
        DEFAULT_SCHEMA_EPOCH,
    );
    fs::create_dir_all(&cli.artifact_root)
        .with_context(|| format!("creating {}", cli.artifact_root.display()))?;
    reconcile_interrupted_sessions(&state_db, &cli.artifact_root, &descriptor.instance_id)?;
    write_descriptor_atomic(runtime_paths.descriptor_path(), &descriptor)?;

    tracing::info!(
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
    })
}

fn serve_handshake_once(runtime: &RuntimeBootstrap) -> Result<()> {
    let handshake_context = HandshakeContext {
        protocol_version: runtime.descriptor.protocol_version,
        daemon_version: env!("CARGO_PKG_VERSION").to_owned(),
        instance_id: runtime.descriptor.instance_id.clone(),
        schema_epoch: DEFAULT_SCHEMA_EPOCH,
        commit_seq: runtime.state_db.current_commit_seq()?,
        health_state: HealthState {
            status: "ready".to_owned(),
        },
        active_sessions: load_active_sessions(runtime.state_db.connection())?,
        mutations_allowed: false,
        expected_peer_uid: None,
        lifecycle_frames: persisted_lifecycle_frames(runtime.state_db.connection())?,
    };
    ipc::handshake::serve_one(&runtime.listener, &handshake_context)
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
    structured_trace
        .and_then(|raw| serde_json::from_str::<Value>(raw).ok())
        .and_then(|value| {
            value
                .get("phase_history")
                .and_then(Value::as_array)
                .map(|array| {
                    array
                        .iter()
                        .filter_map(Value::as_str)
                        .map(str::to_owned)
                        .collect::<Vec<_>>()
                })
        })
        .filter(|history| !history.is_empty())
        .unwrap_or_else(|| {
            default_phase_history_for_status(status)
                .iter()
                .map(|phase| (*phase).to_owned())
                .collect()
        })
}

fn default_phase_history_for_status(status: &str) -> &'static [&'static str] {
    match status {
        "created" => &["created"],
        "spawned" => &["created", "spawned"],
        "active" => &["created", "spawned", "active"],
        "terminating" => &["created", "spawned", "active", "terminating"],
        "exited" => &["created", "spawned", "active", "terminating", "exited"],
        "reaped" => &[
            "created",
            "spawned",
            "active",
            "terminating",
            "exited",
            "reaped",
        ],
        _ => &[
            "created",
            "spawned",
            "active",
            "terminating",
            "exited",
            "reaped",
            "archived",
        ],
    }
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

#[allow(clippy::too_many_lines, clippy::needless_pass_by_value)]
fn run_fts_live_loop(
    runtime: &mut RuntimeBootstrap,
    fixture: LiveLoopFixture,
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

    let job_id = create_sync_source_job(&mut runtime.state_db, &ingest_skill, &fixture)?;
    let runner = JobRunner::new("local-daemon");
    let claimed = runner
        .claim_next_job(runtime.state_db.connection())?
        .ok_or_else(|| anyhow!("expected claimed workflow job"))?;
    ensure!(claimed.id == job_id, "claimed unexpected workflow job");

    let ingest_mode = resolve_skill_mode(&ingest_skill)?;
    ensure!(
        ingest_mode == ExecutionMode::Pty,
        "FTS ingestion skill must resolve to PTY"
    );

    let started_at = Utc::now();
    let ingest_session_id = format!("session-pty-{}", sha256_hex(job_id.as_bytes()));
    let runtime_ids = current_runtime_ids()?;
    let capture = if fixture.capture_write_failure {
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
    } else {
        capture_transcript(
            &runtime.artifact_root,
            &ingest_session_id,
            started_at,
            &fixture.pty_transcript_lines,
        )?
    };

    let ingest_trace = json!({
        "fingerprint_state": fixture.fingerprint_state,
        "locale_state": fixture.locale_state,
        "currency_state": fixture.currency_state,
        "phase_history": ["created", "spawned", "active", "terminating", "exited", "reaped", "archived"],
    })
    .to_string();
    let ingest_completed_at = Utc::now().to_rfc3339();

    insert_session_log(
        runtime.state_db.connection(),
        &SessionDraft {
            session_id: ingest_session_id.clone(),
            job_id: job_id.clone(),
            skill_path: ingest_skill.skill_path.to_string_lossy().into_owned(),
            skill_version: ingest_skill.metadata.skill_version.clone(),
            execution_mode: "pty".to_owned(),
            status: "completed".to_owned(),
            cli_vendor: Some(CLI_VENDOR.to_owned()),
            cli_version: Some(CLI_VERSION.to_owned()),
            adapter_version: Some(env!("CARGO_PKG_VERSION").to_owned()),
            config_hash: Some(skill_config_hash(&ingest_skill)),
            pty_session_id: Some(ingest_session_id.clone()),
            pid: Some(runtime_ids.pid),
            pgid: Some(runtime_ids.pgid),
            started_at: started_at.to_rfc3339(),
            completed_at: ingest_completed_at,
            terminal_output_path: Some(capture.index_path.to_string_lossy().into_owned()),
            terminal_output_hash: Some(capture.first_chunk_hash.clone()),
            terminal_output_bytes: Some(capture.total_bytes),
            terminal_output_tail: Some(capture.tail.clone()),
            structured_trace: Some(ingest_trace),
            error_type: None,
            error_message: None,
            cleanup_verdict: Some("clean".to_owned()),
            last_checkpoint_seq: Some(1),
        },
    )?;

    let ingest_run_manifest = write_run_manifest_for_session(
        runtime,
        &ingest_session_id,
        &ingest_skill,
        "pty",
        started_at,
        &fixture,
        u32::try_from(fixture.notices.len()).context("converting notice count to u32")?,
        capture.artifact_refs,
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
    let contract_started_at = Utc::now();
    insert_session_log(
        runtime.state_db.connection(),
        &SessionDraft {
            session_id: contract_session_id.clone(),
            job_id: job_id.clone(),
            skill_path: manifest_skill.skill_path.to_string_lossy().into_owned(),
            skill_version: manifest_skill.metadata.skill_version.clone(),
            execution_mode: "contract".to_owned(),
            status: "completed".to_owned(),
            cli_vendor: Some(CLI_VENDOR.to_owned()),
            cli_version: Some(CLI_VERSION.to_owned()),
            adapter_version: Some(env!("CARGO_PKG_VERSION").to_owned()),
            config_hash: Some(skill_config_hash(&manifest_skill)),
            pty_session_id: None,
            pid: None,
            pgid: None,
            started_at: contract_started_at.to_rfc3339(),
            completed_at: Utc::now().to_rfc3339(),
            terminal_output_path: None,
            terminal_output_hash: None,
            terminal_output_bytes: None,
            terminal_output_tail: None,
            structured_trace: Some(json!({"mode":"contract","validated":true}).to_string()),
            error_type: None,
            error_message: None,
            cleanup_verdict: Some("clean".to_owned()),
            last_checkpoint_seq: Some(1),
        },
    )?;

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

    let result_artifact_path = write_manifest_result_artifact(
        &runtime.artifact_root,
        &contract_session_id,
        contract_started_at,
        &result_payload,
    )?;
    let validation_outcome = validate_final_result(
        &manifest_skill.result_schema,
        &result_payload,
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
        &contract_session_id,
        &manifest_skill,
        "contract",
        contract_started_at,
        &fixture,
        manifest_computation.candidate_count,
        vec![result_artifact_path.to_string_lossy().into_owned()],
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

    if fixture.basis_hash_mismatch {
        runner.mark_obsolete_and_enqueue_rerun(
            &mut runtime.state_db,
            &ObsoleteJobRequest {
                job_id: job_id.clone(),
                idempotency_key: format!("obsolete-{job_id}"),
                rerun_job_id: format!("rerun-{job_id}"),
                next_job_kind: "sync_source".to_owned(),
                process_id: Some(pipeline.process_id.clone()),
                scope_kind: Some("process".to_owned()),
                scope_id: Some(pipeline.process_id.clone()),
                rerun_from_stage: "manifest".to_owned(),
                expected_input_basis_hash: "expected-basis-hash".to_owned(),
                actual_input_basis_hash: manifest_computation.decision_basis_hash.clone(),
            },
        )?;
        return Ok(LiveLoopReport {
            summary_line: format!(
                "FTS live loop obsolete: job_id={} process_id={} rerun_enqueued=true",
                claimed.id, pipeline.process_id
            ),
            status: "obsolete".to_owned(),
        });
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
                    "skill_binding": {
                        "path": ingest_skill.skill_path.to_string_lossy(),
                        "version": ingest_skill.metadata.skill_version,
                    },
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

fn resolve_skill_mode(skill: &LoadedSkill) -> Result<ExecutionMode> {
    let interaction_profile = match skill.metadata.interaction_profile.as_str() {
        "pty_interactive" => InteractionProfile::PtyRequired,
        "headless_contract" => InteractionProfile::ContractRequired,
        other => bail!("unsupported interaction_profile `{other}`"),
    };
    resolve_execution_mode(
        &SkillManifest {
            interaction_profile,
        },
        &CapabilityReport {
            cli_version: CLI_VERSION.to_owned(),
            auth_healthy: true,
            structured_output: true,
            pty_viable: true,
            browser_tools: true,
            is_canary: false,
        },
    )
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
                runtime_context: ErrorRuntimeContext {
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
                session_artifact_path: session_id.map(|value| {
                    runtime
                        .artifact_root
                        .join("sessions")
                        .join(value)
                        .to_string_lossy()
                        .into_owned()
                }),
                next_debugging_action:
                    "Inspect the linked artifacts and rerun after correcting the root cause."
                        .to_owned(),
            },
            Utc::now(),
        )?;
    Ok(stored.id)
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
    session_id: &str,
    skill: &LoadedSkill,
    execution_mode: &str,
    started_at: DateTime<Utc>,
    fixture: &LiveLoopFixture,
    items_processed: u32,
    artifact_refs: Vec<String>,
) -> Result<agents::artifacts::StoredManifest> {
    let store = ArtifactStore::new(&runtime.artifact_root)?;
    store.write_run_manifest(
        &RunManifestDraft {
            frontmatter: RunManifestFrontmatter {
                manifest_version: 1,
                skill_id: skill.metadata.skill_id.clone(),
                skill_version: skill.metadata.skill_version.clone(),
                job_id: format!("job-for-{session_id}"),
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
                agent_narrative_missing: false,
            },
            narrative: Some(RunNarrative {
                summary: "Daemon-managed live loop completed using persisted local evidence."
                    .to_owned(),
                steps_taken:
                    "Created job/session rows, persisted pipeline state, validated result contract, and wrote the opportunity manifest."
                        .to_owned(),
                decisions_made:
                    "Used the durable SQLite state as the authority for manifest generation."
                        .to_owned(),
                warnings: if items_processed == 0 {
                    "No notices were processed in this run.".to_owned()
                } else {
                    "No blocking warnings.".to_owned()
                },
                evidence: "See the attached artifact refs and durable tables for verification."
                    .to_owned(),
            }),
            appendix: DaemonEvidenceAppendix {
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
                checkpoint_count: 1,
                result_item_count: items_processed,
                oqg_anomaly_summary: Vec::new(),
                representative_item_sample: json!({
                    "notice_id": fixture.notices.first().map(|notice| notice.notice_id.clone()),
                    "title": fixture.notices.first().map(|notice| notice.title.clone()),
                }),
                artifact_refs,
            },
            disagreements: Vec::new(),
        },
        started_at,
    )
}

fn insert_session_log(conn: &Connection, draft: &SessionDraft) -> Result<()> {
    conn.execute(
        "\
        INSERT INTO agent_session_logs (
          id, job_id, skill_path, skill_version, execution_mode, cli_vendor, cli_version,
          adapter_version, config_hash, pty_session_id, pid, pgid, started_at, completed_at,
          status, terminal_output_path, terminal_output_hash, terminal_output_bytes,
          terminal_output_tail, structured_trace, error_type, error_message, cleanup_verdict,
          last_checkpoint_seq
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15,
                  ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24)
        ",
        params![
            draft.session_id,
            draft.job_id,
            draft.skill_path,
            draft.skill_version,
            draft.execution_mode,
            draft.cli_vendor,
            draft.cli_version,
            draft.adapter_version,
            draft.config_hash,
            draft.pty_session_id,
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
            draft.cleanup_verdict,
            draft.last_checkpoint_seq,
        ],
    )
    .context("inserting agent_session_logs row")?;
    Ok(())
}

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
    Ok(RuntimeIds {
        pid: process_id,
        pgid: u32::try_from(process_group_id.as_raw()).context("converting pgid to u32")?,
    })
}

fn skill_config_hash(skill: &LoadedSkill) -> String {
    sha256_hex(
        json!({
            "skill_markdown": skill.skill_markdown,
            "result_schema_version": skill.metadata.result_schema_version,
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
struct RuntimeIds {
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
    use rusqlite::params;
    use serde_json::{Value, json};
    use tempfile::tempdir;

    use super::jobs::runner::CANONICAL_SCOPE_KINDS;
    use super::{
        ACTIVE_STAGES, LifecyclePhase, LiveLoopFixture, SessionDraft, backtrack_window_seconds,
        default_fixture, insert_session_log, live_loop_surface_names, open_state_db,
        persisted_lifecycle_frames, reconcile_interrupted_sessions,
    };
    use crate::db::sqlite::DEFAULT_SCHEMA_EPOCH;

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
    fn backtrack_window_is_positive() {
        assert!(backtrack_window_seconds() > 0);
    }

    #[test]
    fn persisted_lifecycle_frames_reflect_reaped_and_archived_metadata() -> Result<()> {
        let temp = tempdir()?;
        let db_path = temp.path().join("state.db");
        let state_db = open_state_db(&db_path, DEFAULT_SCHEMA_EPOCH)?;
        state_db.connection().execute(
            "\
            INSERT INTO workflow_jobs (
              id, kind, scope_kind, scope_id, status, input_payload, validation_required,
              parent_job_id, validation_verdict_id
            ) VALUES (?1, 'sync_source', 'source', 'fts', 'completed', ?2, 0, NULL, NULL)
            ",
            params!["job-history", json!({"source_id": "fts"}).to_string()],
        )?;
        insert_session_log(
            state_db.connection(),
            &SessionDraft {
                session_id: "session-history".to_owned(),
                job_id: "job-history".to_owned(),
                skill_path: "skills/procurement-ingestion/fts/SKILL.md".to_owned(),
                skill_version: "1.0.0".to_owned(),
                execution_mode: "pty".to_owned(),
                status: "failed".to_owned(),
                cli_vendor: Some("fixture-cli".to_owned()),
                cli_version: Some("1.0.0".to_owned()),
                adapter_version: Some("daemon-test".to_owned()),
                config_hash: None,
                pty_session_id: Some("pty-session-history".to_owned()),
                pid: Some(101),
                pgid: Some(101),
                started_at: "2026-03-25T11:00:00Z".to_owned(),
                completed_at: "2026-03-25T11:00:01Z".to_owned(),
                terminal_output_path: None,
                terminal_output_hash: None,
                terminal_output_bytes: None,
                terminal_output_tail: None,
                structured_trace: Some(
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
                ),
                error_type: Some("AGENT_DAEMON_CRASH_DURING_SESSION".to_owned()),
                error_message: Some("recovered".to_owned()),
                cleanup_verdict: Some("clean".to_owned()),
                last_checkpoint_seq: Some(3),
            },
        )?;

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
        state_db.connection().execute(
            "\
            INSERT INTO workflow_jobs (
              id, kind, scope_kind, scope_id, status, input_payload, validation_required,
              parent_job_id, validation_verdict_id
            ) VALUES (?1, 'sync_source', 'source', 'fts', 'running', ?2, 0, NULL, NULL)
            ",
            params!["job-crash", json!({"source_id": "fts"}).to_string()],
        )?;
        insert_session_log(
            state_db.connection(),
            &SessionDraft {
                session_id: "session-crash".to_owned(),
                job_id: "job-crash".to_owned(),
                skill_path: "skills/procurement-ingestion/fts/SKILL.md".to_owned(),
                skill_version: "1.0.0".to_owned(),
                execution_mode: "pty".to_owned(),
                status: "active".to_owned(),
                cli_vendor: Some("fixture-cli".to_owned()),
                cli_version: Some("1.0.0".to_owned()),
                adapter_version: Some("daemon-test".to_owned()),
                config_hash: None,
                pty_session_id: Some("pty-session-crash".to_owned()),
                pid: Some(999_999),
                pgid: Some(999_999),
                started_at: "2026-03-25T11:00:00Z".to_owned(),
                completed_at: "2026-03-25T11:00:01Z".to_owned(),
                terminal_output_path: None,
                terminal_output_hash: None,
                terminal_output_bytes: None,
                terminal_output_tail: None,
                structured_trace: None,
                error_type: None,
                error_message: None,
                cleanup_verdict: None,
                last_checkpoint_seq: Some(4),
            },
        )?;

        reconcile_interrupted_sessions(&state_db, &artifact_root, "daemon-instance-test")?;

        let (status, error_type, cleanup_verdict, structured_trace): (
            String,
            Option<String>,
            Option<String>,
            Option<String>,
        ) = state_db.connection().query_row(
            "\
            SELECT status, error_type, cleanup_verdict, structured_trace
            FROM agent_session_logs WHERE id = 'session-crash'
            ",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )?;
        assert_eq!(status, "failed");
        assert_eq!(
            error_type.as_deref(),
            Some("AGENT_DAEMON_CRASH_DURING_SESSION")
        );
        assert_eq!(cleanup_verdict.as_deref(), Some("clean"));
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
}

#[cfg(test)]
const fn backtrack_window_seconds() -> u32 {
    BACKTRACK_WINDOW_SECONDS
}
