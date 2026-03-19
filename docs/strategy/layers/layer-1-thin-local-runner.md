# Layer 1 — Thin Local Runner

> **Authority:** This document is the execution-grade specification for Layer 1. An implementation agent should be able to build this layer from this document alone without reopening the research document.
>
> **Milestone:** M1
> **Dependencies:** Layer 0 (skills, intelligence pipeline, contracts-schema, SQLite schema)
> **Exit criteria:** Experiment 4 passes (10 consecutive batch runs, >=90% success rate, 0 orphaned processes) AND first live end-to-end local loop completes.

---

## 1. Overview, Capability & Daemon Bootstrap

### 1.1 Layer Narrative

Minimal Rust runtime — just enough to launch skills, capture artifacts, supervise processes, and run the first live-source end-to-end local pipeline loop. PTY is available as a first-class capability from day 1, with contract mode for deterministic tasks. This remains a thin daemon layer: no cron scheduler (manual CLI trigger only), no resource arbiter (no concurrency limits), no desktop UI, no sync engine, no validation system.

But it is also the **first live pipeline layer**: a manual CLI trigger ingests live FTS notices, runs the full local qualification/pricing/manifest loop, and emits verifiable local output without any UI or sync dependency.

**Why this layer exists:** Layer 0 proved the intelligence thesis on curated data. Layer 1 proves the runtime thesis: can the daemon reliably spawn agents, supervise their lifecycle, capture forensic artifacts, and produce verifiable pipeline output from live sources? Infrastructure stability is the prerequisite for daily operator use (Layer 2).

**If we never build Layer 2:** The thin runner provides a reliable way to execute skills from the command line with full process supervision, artifact capture, and error manifests. It operates as a headless pipeline executor. The gold-set evaluation harness from Layer 0 continues to work. The daemon can ingest live procurement data and produce local opportunity artifacts without any user interface.

### 1.2 Capability Delivered

Two capabilities proven:

**Runtime reliability (Experiment 4):**
10 consecutive batch runs of skills complete with >=90% skill success rate and no orphaned processes. Skills are now daemon-managed with proper lifecycle:

```text
spawn -> monitor -> capture -> validate -> persist -> reap
```

**First live operational loop:**

```text
notice ingestion -> pipeline processing -> verifiable local output
```

The verifiable output is a CLI-visible summary plus a local markdown opportunity manifest/file artifact for at least one live-source batch. This proves the full pipeline works end-to-end on real data, not just gold-set data.

### 1.3 System Context (Layer 1 Scope)

```text
Procurement Sources (live)         Cost-Estimation APIs
+-----------------+               +------------------+
| Find a Tender   |               | HMRC Tariff API  |
| (OCDS API/portal|               | HMRC FX Rates    |
| via FTS skill)  |               +------------------+
+--------+--------+
         |
         v
+--------------------------------------------------+
| Rust Daemon (Local Machine)                      |
| +----------------------------------------------+ |
| | Agent Session Runtime (L1)                    | |
| | - manual CLI triggers (no cron yet)           | |
| | - adapter-mediated sessions (PTY or contract) | |
| | - skill loading + kickoff prompt injection    | |
| | - forensic capture + run manifests            | |
| | - process-group supervision                   | |
| +----------------------------------------------+ |
| +----------------------------------------------+ |
| | Local SQLite (Operational Authority)          | |
| | - pipeline entities (from L0 schema)          | |
| | - agent_session_logs, error_manifests (L1)    | |
| | - source_cursors, raw_documents (L1)          | |
| | - workflow_jobs (L1 activation)               | |
| | - process_stage_status/heads (L1 activation)  | |
| +----------------------------------------------+ |
+--------------------------------------------------+
         |
         v
    CLI output + local manifest artifact
    (no desktop, no web, no Supabase)
```

**Not in Layer 1:** Desktop UI, Supabase sync, cron scheduler, resource arbiter, PTY hardening, relation resolution, Contracts Finder, validation system, multi-source marketplace search.

### 1.4 Relationship to Layer 0

Layer 0's TypeScript intelligence code runs from CLI (`bun run skill-name`). Layer 1 wraps that same code in daemon-managed execution with a full lifecycle. The intelligence pipeline logic does not change — only the execution wrapper changes.

| Aspect | Layer 0 | Layer 1 |
|--------|---------|---------|
| Execution | CLI invocation (`bun run`) | Daemon-managed adapter-mediated sessions |
| Input | Curated gold-set corpus | Live FTS notices |
| Process supervision | None | Process-group supervision with cleanup verdicts |
| Artifact capture | None | Raw bytes compressed + stored; metadata in `agent_session_logs` |
| Run manifests | SKILL.md includes narrative instructions (preparation) | Formal daemon-managed manifest: agent narrative + daemon evidence appendix |
| Error handling | Test harness catches errors | Error manifest system: Markdown error manifests with full context |
| Stage tracking | Implicit | Explicit `stage_run_id` + `process_stage_status` + `process_stage_heads` |
| Output | Harness metrics report | CLI summary + local markdown manifest artifact |

### 1.5 Component Overview

**Rust workspace bootstrap:**
- `Cargo.toml` — workspace root
- `rust-toolchain.toml` — Rust 2024 edition (nightly)

**`crates/daemon` — 14 core files:**

| File | Responsibility |
|------|---------------|
| `main.rs` | CLI entry point, argument parsing, daemon bootstrap |
| `db/sqlite.rs` | SQLite connection management (WAL mode, read-only query connections, single-writer enforcement) |
| `pty/session.rs` | PTY session creation, process-group spawn, pgid recording |
| `pty/capture.rs` | Raw byte capture from PTY master fd; compressed artifact storage |
| `pty/health.rs` | Session health monitoring: stall detection (no-output timeout, no-progress timeout), auth challenge detection, host sleep detection |
| `jobs/runner.rs` | Job Runner: claim jobs from `workflow_jobs`, execute via adapter, persist results, update stage status/heads, handle `continuation_required` |
| `agents/adapter.rs` | Agent Adapter trait: unified event stream regardless of PTY/contract mode; mode selection via `interaction_profile` |
| `agents/codex.rs` | Codex adapter: launch command, capability probe, JSONL parsing, health classification |
| `agents/claude.rs` | Claude adapter: launch command, capability probe, JSON/stream-json parsing, health classification |
| `agents/artifacts.rs` | Artifact store: compressed raw session output, result contract files, checkpoint artifacts, run manifests |
| `agents/contracts.rs` | Result contract validator: schema validation of skill output against `result.schema.json` |
| `prompt/renderer.rs` | Kickoff prompt renderer: skill content injection, job parameters, resume tokens, run manifest instruction block |
| `errors/manifest.rs` | Error manifest system: structured Markdown error manifests for pipeline failures |
| `errors/markdown.rs` | Error manifest Markdown renderer |

**New skills (2):**

| Skill | Purpose | Interaction Profile |
|-------|---------|-------------------|
| `skills/procurement-ingestion/fts/` | Live FTS notice ingestion via browser automation | `pty_browser` |
| `skills/manifest-generation/` | Opportunity manifest generation from pipeline results | `contract_json` |

**Database migration:**

`0003_sqlite_artifacts.sql` — creates `agent_session_logs`, `raw_documents`, `error_manifests`, `executors`, `source_cursors` tables.

### 1.6 Ingestion Flow (End-to-End Live Loop)

The first live operational loop executes this sequence via manual CLI trigger:

**Step 1 — Trigger:** Operator runs CLI command (e.g., `cargo run -- ingest fts`).

**Step 2 — Job creation:** Daemon creates a `workflow_jobs` entry with `job_kind = 'sync_source'`, explicit skill id/version binding (`skills/procurement-ingestion/fts`).

**Step 3 — Adapter selection:** Agent Adapter selects execution mode based on skill's `interaction_profile: pty_browser`. For FTS ingestion, this is PTY mode (browser automation required).

**Step 4 — Session spawn:** Adapter spawns agent session in its own process group (POSIX `setpgid`). Records `pid`, `pgid`, `adapter_id`, CLI version, and config hash. Session state: `created -> spawned`.

**Step 5 — Skill execution:** Agent navigates to FTS portal via browser automation, following the `procurement-ingestion/fts/SKILL.md` playbook. Agent proves expected page fingerprint before extraction. Session state: `spawned -> active`.

**Step 6 — Incremental retrieval:** Agent uses cursor/watermark from previous run (stored in `source_cursors`). FTS API supports `updatedFrom`, `updatedTo`, `cursor`, `limit`. Durable commit boundaries are notice batches. Resume uses small cursor backtrack window to avoid skipping records.

**Step 7 — Capture:** Raw PTY bytes captured as immutable forensic artifact, compressed and stored at `artifacts/sessions/YYYY/MM/<session_id>/`. Session metadata in `agent_session_logs`.

**Step 8 — Checkpoints and result contract:** Agent writes checkpoints and final outputs through validated result contract. Workflow state advances from validated contracts, NOT from transcript parsing.

**Step 9 — Run manifest generation:** Agent writes narrative sections (Summary, Steps Taken, Decisions Made, Warnings, Evidence). Daemon appends machine-authored evidence appendix (observed URL chain, fingerprint state, checkpoint count, result item count, OQG anomaly summary, artifact refs). If agent fails to narrate -> daemon produces fallback machine-authored manifest with `agent_narrative_missing: true`. Stored at `artifacts/sessions/YYYY/MM/<session_id>/run-manifest.md`.

**Step 10 — Result validation:** Result Contract Validator checks output against `result.schema.json`. Output Quality Gate checks for semantic/statistical anomalies. Schema-valid outputs failing semantic checks are quarantined as `AGENT_SUSPECT_OUTPUT`.

**Step 11 — Pipeline processing:** Notices normalized into internal schema. For Layer 1, relation resolution is skipped (stubbed — full engine in Layer 3). Pipeline stages from Layer 0 §3 execute: qualify -> acquire docs -> extract lots -> extract products -> quoteability -> price match -> gross gate -> landed cost -> net gate -> coverage rollup -> manifest.

**Step 12 — Stage tracking:** In the same SQLite transaction that writes stage outputs, the daemon writes all rows with a shared `stage_run_id`, updates `process_stage_status`, and advances `process_stage_heads` atomically. Failed stages publish failure status but do NOT advance the stage head.

**Step 13 — Session cleanup:** Session state: `active -> terminating -> exited -> reaped -> archived`. Session Reaper performs descendant cleanup (SIGTERM group -> grace period -> SIGKILL group -> orphan sweep). Records cleanup verdict: `clean` | `forced` | `leaked`.

**Step 14 — Verifiable output:** CLI summary printed. Local markdown manifest artifact emitted proving the loop completed. Manifest stored in `manifests` table row.

### 1.7 Adapter-Mediated Hybrid Execution

The daemon triggers agent sessions through an **Agent Adapter** boundary with two execution modes:

#### PTY Mode

Required for browser automation, interactive auth flows, or any CLI behavior needing a controlling terminal. The daemon creates PTY pairs (master/slave), captures full terminal output as an immutable forensic artifact, and monitors session health.

**When to use:** Skill declares `interaction_profile: pty_browser` or `pty_cli`. Most procurement ingestion and supplier enrichment skills need PTY mode because they drive browsers.

#### Contract Mode

Preferred for deterministic tasks where the vendor supports structured non-interactive execution. Lower overhead, simpler cleanup, stronger result contracts.

**When to use:** Skill declares `interaction_profile: contract_json` or `contract_stream`. Manifest generation, qualification, and other deterministic skills use contract mode.

**Vendor adapters:**

| Adapter | PTY Mode | Contract Mode |
|---------|----------|---------------|
| Codex (`agents/codex.rs`) | Full terminal session | `exec` with `--json` JSONL output |
| Claude (`agents/claude.rs`) | Full terminal session | Headless with `json`/`stream-json` output + optional `--json-schema` validation |

**Adapter responsibilities:**
- Launch command construction (flags, env vars, working directory)
- Capability probing at startup (CLI version, auth health, structured output viability, PTY viability, browser/tool viability)
- Mode selection based on skill `interaction_profile` and probe results
- Parsing vendor-native JSON/event streams into daemon-native events
- Health classification from vendor-specific signals
- Final result contract validation

**Daemon-native event stream (canonical — all adapters produce this):**

| Event | Meaning |
|-------|---------|
| `SessionStarted` | Agent session spawned and active |
| `CheckpointWritten` | Validated checkpoint persisted |
| `AuthChallenge` | Agent hit an auth/MFA/SSO challenge |
| `SessionStalled` | No output or no progress for timeout period |
| `CleanupIncomplete` | Descendant processes survived group kill |
| `FinalResultValidated` | Result contract passed schema validation |

**Canary mode:** Unknown CLI versions enter canary mode — the adapter runs a smoke probe before accepting production traffic.

**Session isolation:** Concurrent sessions on the same workstation can contaminate each other through shared CLI config. The daemon must: pin `CODEX_HOME`/`.codex` config paths per session, control working directory per run, record CLI version/model/config hash/skill version/adapter version on every session, use per-session browser profiles for browser-backed flows.

### 1.8 Process-Group Supervision

The daemon supervises the **entire descendant tree**, not just the initial CLI process. Agent CLIs may spawn sub-processes (browser instances, tool servers, language runtimes).

**Session lifecycle (7 phases):**

```text
created -> spawned -> active -> terminating -> exited -> reaped -> archived
```

A session is only **complete** after ALL of:
1. Child exit observed
2. PTY drained to EOF or drain timeout (PTY mode only)
3. Descendant process tree cleaned up (verified, not assumed)
4. Final result contract validated (if present)
5. Cleanup verdict recorded

**Cleanup sequence:**

1. Spawn session in its own process group (`setpgid` on POSIX)
2. Record `pid`, `pgid`, PTY id, adapter id at spawn
3. On soft stop: send in-band interrupt if adapter supports it
4. On timeout: SIGTERM the **process group** (not just main pid)
5. After grace period: SIGKILL the group
6. Keep draining PTY master until EOF or drain timeout (PTY mode)
7. Wait/reap explicitly — do NOT rely on dropping Tokio child handles
8. Run orphan sweep and record cleanup verdict (`clean` | `forced` | `leaked`)

**Cleanup verdicts:**

| Verdict | Meaning |
|---------|---------|
| `clean` | All descendant processes exited cleanly before timeout |
| `forced` | Process group required SIGTERM or SIGKILL |
| `leaked` | Descendant processes survived group kill — operator attention needed |

**Platform notes:**
- macOS: process group signaling via `killpg` is the primary mechanism
- Linux: optionally use transient `systemd-run --scope` for per-session resource control, or register as child subreaper

### 1.9 Error Classification

Every pipeline failure produces a structured Markdown error manifest with full context.

**Error types active in Layer 1:**

| Error Type | Trigger | Action |
|------------|---------|--------|
| `AGENT_STALLED` | No PTY/stdout bytes for timeout period, or no validated checkpoint progress despite output | Kill process group; create error manifest |
| `AGENT_AUTH_CHALLENGE` | Re-login prompt, MFA challenge, browser SSO failure, hidden permission prompt | Flag for operator; create error manifest |
| `AGENT_SKILL_FAILED` | Source website changed; skill playbook failed at specific step | Create error manifest with skill path and step |
| `AGENT_OUTPUT_CONTRACT_INVALID` | Agent output fails schema validation against `result.schema.json` | Preserve transcript + artifact; fail job |
| `AGENT_SUSPECT_OUTPUT` | Schema-valid output fails OQG semantic/drift checks | Quarantine results; keep pipeline state unchanged |
| `AGENT_PARTIAL_FAILURE` | Valid checkpoint exists before crash | Persist checkpoint outputs; never upgrade to success without final contract + clean exit + reap |
| `AGENT_INTERNAL_ERROR` | Error from structured events or terminal output | Create error manifest |
| `AGENT_CLEANUP_INCOMPLETE` | Descendant processes leaked after session exit | Record leaked pids; flag for operator |
| `HOST_SLEEP_INTERRUPTED` | Monotonic time gap exceeds threshold during active session | Taint browser-backed sessions; create error manifest |
| `HOST_RESOURCE_EXHAUSTED` | Host memory/CPU pressure detected | May preempt lower-priority sessions |
| `AGENT_RUN_FAILED` | Non-zero exit code with no specific error pattern | Create error manifest |

### 1.10 What's Stubbed in This Layer

| Feature | Layer 1 Stub | Full Implementation |
|---------|-------------|-------------------|
| Scheduler | CLI manual trigger only | Layer 3: cron scheduler with explicit skill binding |
| Resource arbiter | No concurrency limits | Layer 3: typed quotas (browser slots, agent slots, source lanes) |
| PTY hardening | Basic session creation | Layer 3: fd hygiene, EIO handling, terminal geometry stability |
| Job claiming/leases | Single-threaded execution | Layer 3: concurrent job claiming with lease management |
| Source registry | Bootstrapped local seed / hardcoded source definitions | Layer 3: typed registry; Layer 4: `shared_source_configs` pull |
| Procurement coverage | First live source only (FTS) | Layer 2: adds Contracts Finder; Layer 5: PCS if needed |
| Session reaper | Basic cleanup (SIGTERM -> SIGKILL -> orphan sweep) | Layer 3: full reaper with escalation and diagnostics |
| Tracing | Basic log capture | Layer 3: structured + forensic tracing |
| Validation | Fields exist, always `validation_required = false` | Layer 3: dry-run; Layer 4: enforcement |
| Operator-facing manifest | Minimal local markdown/file artifact | Layer 2: shared `packages/manifests` renderer + desktop presentation |
| Desktop/operator surface | CLI summary + local file artifact only | Layer 2: full desktop control panel |
| Sync | Tracking columns populated on every write, no outbox entries | Layer 4: full sync engine (outbox + push + pull) |
| Relation resolution | Notices processed independently (no merge/relate) | Layer 3: full heuristic resolution engine |

### 1.11 Things NOT Implemented in Layer 1

These are designed but not active. They exist only as forward-compatible schema provisions or structural patterns:

- Cron scheduling (Layer 3)
- Resource arbiter with typed quotas (Layer 3)
- Worker-validator system (Layer 3 dry-run, Layer 4 enforcement)
- Sync engine / Supabase push-pull (Layer 4)
- Desktop control panel (Layer 2)
- Web viewing portal (Layer 4)
- Contracts Finder ingestion (Layer 2)
- Full PTY hardening: fd hygiene, EIO handling, terminal geometry (Layer 3)
- Source health monitoring and circuit breakers (Layer 3)
- Skill promotion flow (Layer 3)
- Multi-source marketplace search: Alibaba, 1688, Amazon (Layer 5)
- Decision-grade landed cost (Layer 5)
- Bid workflow automation (Layer 6)

---

## 2. Daemon Core, PTY & Process Supervision

### 2.1 Rust Crate Structure

```
crates/
  daemon/
    Cargo.toml
    src/
      main.rs              # CLI entry, argument parsing, daemon bootstrap, signal handling
      db/
        sqlite.rs          # Connection management: WAL mode, read-only query pool,
                           #   single-writer enforcement, migration runner, PRAGMA init
      pty/
        session.rs         # PTY pair creation (master/slave), process-group spawn,
                           #   pgid recording, session state machine (7 phases)
        capture.rs         # Raw byte capture from PTY master fd; zstd compression;
                           #   chunked artifact storage at artifacts/sessions/YYYY/MM/<sid>/
        health.rs          # Session health monitoring: no-output stall detection,
                           #   no-progress stall detection, auth challenge detection,
                           #   host sleep/monotonic-gap detection
      jobs/
        runner.rs          # Job Runner: claim from workflow_jobs, execute via adapter,
                           #   persist results, update process_stage_status/heads,
                           #   stage_run_id scoping, parent_job_id awareness for future
                           #   validation sub-jobs, continuation_required handling
      agents/
        adapter.rs         # Agent Adapter trait: unified daemon event stream regardless
                           #   of PTY/contract mode; mode selection via interaction_profile
        codex.rs           # Codex adapter: launch command construction, capability probe
                           #   (CLI version, auth health, structured output viability),
                           #   JSONL parsing, health classification, canary mode
        claude.rs          # Claude adapter: launch command construction, capability probe,
                           #   JSON/stream-json parsing, health classification, canary mode
        artifacts.rs       # Artifact store: compressed immutable session output,
                           #   result contract files, checkpoint artifacts, run manifests;
                           #   stored at artifacts/sessions/YYYY/MM/<session_id>/
        contracts.rs       # Result contract validator: schema validation against
                           #   result.schema.json, checkpoint sequencing, partial failure
                           #   classification; OQG semantic/drift checks
      prompt/
        renderer.rs        # Kickoff prompt composition: SKILL.md content injection,
                           #   job parameters, resume tokens, run manifest instruction
                           #   block, company profile injection (future)
      errors/
        manifest.rs        # Error manifest generation: structured Markdown error records
                           #   with CLI vendor+version, adapter version, model id, config
                           #   hash, skill version, pid/pgid, cleanup verdict, last
                           #   valid checkpoint sequence
        markdown.rs        # Error manifest Markdown rendering engine
```

### 2.2 Rust Crate Dependencies

**Production dependencies (Layer 1 scope only):**

| Crate | Purpose | Notes |
|-------|---------|-------|
| `tokio` | Async runtime | Full features: rt-multi-thread, macros, signal, process, io-util, time |
| `rusqlite` | Local SQLite access | With `bundled` feature for zero-dep SQLite. Alternative: `sqlx` with SQLite feature |
| `serde` | Serialization framework | With `derive` feature |
| `serde_json` | JSON serialization | For result contracts, checkpoint parsing, config |
| `tracing` | Structured logging | Async-aware structured logging |
| `tracing-subscriber` | Log output | Console + file output layers |
| `chrono` | Date/time | ISO-8601 timestamps, duration tracking |
| `anyhow` | Error context | For application-level error propagation |
| `thiserror` | Typed errors | For library-level error enums |
| `nix` | POSIX process-group and signal handling | `killpg`, `setpgid`, `waitpid`, signal constants |
| `portable-pty` | PTY session management | Cross-platform PTY creation. Alternative: `pty-process` |
| `uuid` | UUID generation | v4 UUIDs for session IDs, job IDs (SQLite has no `gen_random_uuid`) |
| `sha2` | SHA-256 hashes | For artifact content hashes, result contract verification, `decisionBasisHash` |
| `zstd` | Compression | For PTY byte stream and artifact compression |
| `clap` | CLI argument parsing | For daemon CLI interface |

**NOT needed until later layers:**

| Crate | Layer | Purpose |
|-------|-------|---------|
| `reqwest` | L0 (intelligence) / L5 (freight/FX APIs) | HTTP client for cost-estimation APIs |
| `sqlx` (Postgres feature) | L4 | Supabase/Postgres sync |
| `cron` | L3 | Cron expression parsing for scheduler |
| `sysinfo` | L3 | Host resource pressure detection for resource arbiter |

### 2.3 Agent Adapter Trait

The adapter trait is the abstraction boundary between the daemon and vendor-specific agent CLIs. Everything above the adapter speaks only in daemon-native events.

```rust
/// Canonical daemon event stream — all adapters produce this
#[derive(Debug, Clone)]
pub enum DaemonEvent {
    SessionStarted { session_id: Uuid, pid: u32, pgid: u32, mode: ExecutionMode },
    CheckpointWritten { seq: u32, artifact_path: PathBuf, hash: String },
    ProgressUpdate { items_processed: u32, items_total: Option<u32> },
    AuthChallenge { challenge_type: String, details: String },
    SessionStalled { reason: StallReason, last_output_ago: Duration },
    HealthWarning { warning: String },
    FinalResultValidated { artifact_path: PathBuf, hash: String, item_count: u32 },
    SessionExited { exit_code: Option<i32>, duration: Duration },
    CleanupIncomplete { leaked_pids: Vec<u32> },
}

#[derive(Debug, Clone, Copy)]
pub enum ExecutionMode {
    Pty,       // Full PTY session with terminal semantics
    Contract,  // Structured JSON/JSONL non-interactive session
}

#[derive(Debug, Clone, Copy)]
pub enum StallReason {
    NoOutput,    // No PTY/stdout bytes for timeout period
    NoProgress,  // Output present but no validated checkpoints advancing
}

/// Agent Adapter trait — each vendor implements this
#[async_trait]
pub trait AgentAdapter: Send + Sync {
    /// Probe CLI capabilities: version, auth health, structured output
    /// viability, PTY viability, browser/tool viability.
    /// Unknown versions enter canary mode.
    async fn capability_probe(&self) -> Result<CapabilityReport>;

    /// Select execution mode based on skill's interaction_profile
    /// and probe results.
    fn select_mode(&self, profile: &InteractionProfile, probe: &CapabilityReport)
        -> ExecutionMode;

    /// Construct the launch command: binary path, flags, env vars,
    /// working directory, config paths (session-isolated).
    fn build_launch_command(
        &self,
        skill: &SkillManifest,
        job: &WorkflowJob,
        kickoff_prompt: &str,
        mode: ExecutionMode,
    ) -> Command;

    /// Parse vendor-native output into daemon events.
    /// For PTY mode: best-effort parsing of terminal output.
    /// For contract mode: structured JSONL/JSON parsing.
    async fn parse_events(
        &self,
        raw_output: &mut dyn AsyncRead,
        mode: ExecutionMode,
    ) -> Result<Vec<DaemonEvent>>;

    /// Classify health from vendor-specific signals.
    fn classify_health(&self, events: &[DaemonEvent], exit_code: Option<i32>)
        -> HealthClassification;

    /// Validate the final result contract against the skill's
    /// result.schema.json.
    async fn validate_result(
        &self,
        artifact_path: &Path,
        schema: &serde_json::Value,
    ) -> Result<ValidationResult>;
}

#[derive(Debug, Clone)]
pub struct CapabilityReport {
    pub cli_version: String,
    pub auth_healthy: bool,
    pub structured_output: bool,  // Can produce JSON/JSONL
    pub pty_viable: bool,         // Terminal semantics available
    pub browser_tools: bool,      // Browser automation tools available
    pub is_canary: bool,          // Unknown version — canary mode
}

#[derive(Debug, Clone, Copy)]
pub enum InteractionProfile {
    PtyBrowser,       // Browser automation — requires PTY
    PtyCli,           // Interactive CLI — requires PTY
    ContractJson,     // Structured JSON output — prefers contract mode
    ContractStream,   // Streaming JSON output — prefers contract mode
}
```

**Mode selection logic:**

| `interaction_profile` | Probe: structured output | Probe: PTY viable | Selected Mode |
|----------------------|-------------------------|-------------------|---------------|
| `pty_browser` | any | must be true | PTY |
| `pty_cli` | any | must be true | PTY |
| `contract_json` | must be true | any | Contract |
| `contract_stream` | must be true | any | Contract |
| any | false | true | PTY (fallback) |
| any | false | false | Error: cannot execute |

### 2.4 Session Lifecycle (7 Phases)

Every agent session transitions through exactly 7 phases. No phase can be skipped.

```text
created -> spawned -> active -> terminating -> exited -> reaped -> archived
```

| Phase | Responsibilities | Failure Mode |
|-------|-----------------|-------------|
| **created** | Session record created in `agent_session_logs`. Adapter selected. Kickoff prompt composed. Skill loaded. Working directory prepared. Session-isolated config paths set. | Config error → job fails before spawn |
| **spawned** | Process launched in own process group (`setpgid`). `pid`, `pgid`, PTY id recorded. PTY master fd opened (PTY mode). Capture stream started. | Spawn failure → `AGENT_RUN_FAILED` |
| **active** | Agent executing. Raw bytes captured. Health monitoring active: no-output timeout, no-progress timeout, auth challenge detection. Checkpoints validated as they arrive. | Stall → `AGENT_STALLED`; auth → `AGENT_AUTH_CHALLENGE`; host sleep → `HOST_SLEEP_INTERRUPTED` |
| **terminating** | Clean shutdown initiated. In-band interrupt sent if adapter supports it. Wait for graceful exit with timeout. | Timeout → escalate to SIGTERM group |
| **exited** | Child process exited. Exit code recorded. PTY master drained to EOF or drain timeout (PTY mode). Raw stdout/stderr capture finalized (contract mode). | EIO on PTY read → normal (slave closed first); drain timeout → proceed to reap |
| **reaped** | Descendant process tree cleaned up. Kill escalation: SIGTERM group → grace period → SIGKILL group → orphan sweep. Cleanup verdict recorded: `clean` / `forced` / `leaked`. Final result contract validated. Run manifest finalized (agent narrative + daemon evidence appendix). | Leaked descendants → `AGENT_CLEANUP_INCOMPLETE` |
| **archived** | Session complete. All artifacts stored and indexed. Session metadata finalized in `agent_session_logs`. Job status updated in `workflow_jobs`. Stage status/heads updated if pipeline job. | None — terminal state |

**Session completeness rule:** A session is only complete after ALL of:
1. Child exit observed
2. PTY drained to EOF or drain timeout (PTY mode only)
3. Descendant process tree verified clean (not assumed)
4. Final result contract validated (if present)
5. Cleanup verdict recorded (`clean` | `forced` | `leaked`)
6. Run manifest stored (agent narrative + daemon evidence appendix; machine-authored fallback if agent fails to narrate)

### 2.5 Artifact Store

Raw session output is stored as compressed immutable artifacts **outside SQLite** — session byte streams are too large for inline database storage.

**Storage layout:**

```
artifacts/
  sessions/
    YYYY/
      MM/
        <session_id>/
          raw-pty.zst          # Compressed PTY byte stream (PTY mode)
          raw-stdout.zst       # Compressed stdout (contract mode)
          raw-stderr.zst       # Compressed stderr (contract mode)
          run-manifest.md      # Hybrid-authored run manifest
          result.json          # Validated result contract
          checkpoint-001.json  # Checkpoint artifacts
          checkpoint-002.json
          ...
```

**SQLite stores metadata only:**
- `agent_session_logs`: session ID, skill ID, job ID, pid, pgid, adapter, mode, start/end times, exit code, cleanup verdict, byte counts, tail excerpts, artifact paths, SHA-256 hashes, `run_manifest_path`, `run_manifest_hash`
- `raw_documents`: document metadata and paths (actual files in artifact store)
- `error_manifests`: error context, skill path, step, resolution hints

**Run manifest format:**

YAML frontmatter with structured metadata followed by agent narrative sections, then daemon evidence appendix. Target size: < 50 KB.

**Frontmatter — agent-claimed fields** (self-reported, `agent_claimed_*` prefix):
- `manifest_version`, `skill_id`, `skill_version`, `job_id`, `session_id`, `execution_mode`
- `started_at`, `completed_at`, `outcome_class`
- `items_processed`, `items_skipped`, `items_failed`, `warnings_count`
- `agent_claimed_pages_visited`, `agent_claimed_source_url`
- `agent_claimed_fingerprint_state` (`matched` | `mismatched` | `unknown`)
- `agent_claimed_locale_state`, `agent_claimed_currency_state`

**Agent narrative sections** (human-readable):
- Summary, Steps Taken, Decisions Made, Warnings, Evidence (with claim/evidence references)

**Daemon evidence appendix** (machine-authored, immutable, < 2 KB):
- `observed_url_chain`, `observed_page_kind_sequence`
- `daemon_observed_fingerprint_state`, `daemon_observed_locale`, `daemon_observed_currency`
- `auth_challenge_events`, `locator_fallback_depth`
- `checkpoint_count`, `result_item_count`
- `oqg_anomaly_summary`, `representative_item_sample`, `artifact_refs`

**Critical naming rule:** Self-reported fields use `agent_claimed_*`. Daemon-observed fields use `daemon_observed_*`. Never ambiguous between "agent claims" and "daemon observed."

**Fallback mode:** If agent fails to emit narrative (crash, timeout) → daemon synthesizes minimum machine-authored manifest from observability layers 1-3 alone, with `agent_narrative_missing: true`. The worst runs must not leave the thinnest review surface.

### 2.6 Job Runner

The Job Runner claims queued jobs from `workflow_jobs`, executes them via the Agent Adapter, and persists results with stage tracking.

**Job execution flow:**

1. Claim next job from `workflow_jobs` (single-threaded in L1 — concurrent claiming in L3)
2. Read job payload: `job_kind`, `scope_kind`, `scope_id`, skill binding
3. Request resource budget from Resource Arbiter (no-op in L1 — arbiter not yet implemented)
4. Invoke Agent Adapter in the correct execution mode
5. Monitor session through 7-phase lifecycle
6. On success: validate result contract, run OQG checks
7. Persist stage outputs in same SQLite transaction:
   - Write all output rows with shared `stage_run_id`
   - Update `process_stage_status` to `succeeded`
   - Advance `process_stage_heads`
   - Populate sync tracking columns (`owner_id`, `row_version`, `last_local_seq`) on every push-owned write
8. On failure: create error manifest, update job status, do NOT advance stage head
9. On `continuation_required`: persist committed batches, enqueue next segment with resume token

**`stage_run_id` scoping rules:**
- One `stage_run_id` = one atomic commit cohort for one scoped business unit
- A single agent session may produce multiple `stage_run_id` commits (e.g., one per candidate-source pair)
- Set-replacement stages: all outputs must share one `stage_run_id`; partial results do not advance the head
- Additive stages: each scope unit gets its own `stage_run_id`; committed units survive when others fail

**Sync tracking (L1 behavior):**
- Every push-owned table write populates `owner_id`, `row_version`, `last_local_seq`
- No `local_outbox` entries created yet (outbox engine activates in Layer 4)
- Columns are populated from first write so no migration is needed when sync activates

### 2.7 SQLite Configuration

**Two-database split:**

| Database | Contents | Rationale |
|----------|----------|-----------|
| `state.db` | Pipeline entities, jobs, manifests, outbox (future), sync metadata (future), stage tracking | Hot relational state — needs fast reads/writes |
| `artifacts.db` | Agent session logs (metadata), raw documents (metadata), error manifests | Large artifacts prevent checkpoint/vacuum overhead on hot state DB |

Actual session byte streams and run manifests are stored in the filesystem artifact store, not in either database.

**Initialization sequence:**

```sql
PRAGMA journal_mode = WAL;           -- Write-ahead logging for concurrent reads
PRAGMA synchronous = NORMAL;         -- Safe with WAL mode
PRAGMA foreign_keys = ON;            -- Enforce referential integrity
PRAGMA busy_timeout = 5000;          -- 5s retry on lock contention
PRAGMA cache_size = -64000;          -- 64MB page cache
```

**Operational rules:**
- Single writer (daemon only); multiple short-lived readers (desktop QueryService in L2)
- Run `PRAGMA quick_check` on startup
- Run `PRAGMA optimize` on maintenance cadence
- Use Online Backup API or `VACUUM INTO` for safe hot backups
- Store `schema_epoch` in `PRAGMA user_version`
- Keep on local disk only — never NFS/cloud-sync folders
- JSON1 extension for structured JSON columns
- Type affinity: TEXT for UUIDs, INTEGER for boolean/pence/bps, TEXT for ISO-8601 timestamps
- No `gen_random_uuid` — use application-side `uuid` crate

---

## 4. Skills, Run Manifests & Error Manifests

### 4.1 New Skills (Layer 1)

Two skills are introduced in Layer 1. Both follow the production skill directory standard established in Layer 0 §4 (SKILL.md + result.schema.json + checkpoint.schema.json + canary.inputs.jsonl + golden.assertions.yaml).

#### `skills/procurement-ingestion/fts/`

| Attribute | Value |
|-----------|-------|
| **Purpose** | Ingest live procurement notices from Find a Tender (FTS) via browser automation |
| **Interaction profile** | `pty_browser` — requires PTY mode (browser-driven navigation) |
| **Model profile** | `balanced` — needs reasoning for page structure changes but not deep analysis |
| **Input** | Cursor/watermark from `source_cursors` (last successful ingestion position) |
| **Output** | Batch of raw OCDS notice records with structured metadata |
| **Commit semantics** | Additive (notice batches); `continuation_required` for large result sets |
| **Auth requirements** | None — FTS is publicly accessible under Open Government Licence |
| **Resume strategy** | Cursor backtrack window (re-fetch small overlap to avoid record skipping) |

**Skill playbook summary:**
1. Navigate to FTS OCDS release package endpoint
2. Verify page fingerprint before extraction
3. Fetch notices since last cursor using `updatedFrom`, `updatedTo`, `cursor`, `limit` parameters
4. Parse OCDS release packages into structured notice records
5. Write checkpoints at notice batch boundaries
6. Emit `continuation_required` if time budget reached at a safe boundary
7. Update cursor position on success

**Canary test:** Fetch 5 most recent notices; verify OCDS structure, required fields present, notice count > 0, locale correct.

#### `skills/manifest-generation/`

| Attribute | Value |
|-----------|-------|
| **Purpose** | Generate opportunity manifest from pipeline results |
| **Interaction profile** | `contract_json` — deterministic, no browser needed |
| **Model profile** | `balanced` — needs reasoning to synthesize narrative from pipeline data |
| **Input** | Pipeline assessment + all candidate/quote/cost data for a process |
| **Output** | Markdown opportunity manifest with structured metadata |
| **Commit semantics** | Set-replacement per process |
| **Auth requirements** | None — operates on local data only |
| **Resume strategy** | Not applicable (single-process, fast execution) |

**Skill playbook summary:**
1. Read pipeline assessment and all supporting data for the target process
2. Synthesize human-readable narrative: opportunity summary, candidate list, pricing evidence, margin bands, coverage metrics, blockers, confidence breakdown
3. Compute `decisionBasisHash` (SHA-256 of the pipeline data the operator would review)
4. Emit structured manifest with YAML frontmatter + Markdown body
5. Store in `manifests` table row

**Note:** The canonical manifest rendering logic lives in `packages/manifests` (Layer 2). In Layer 1, the skill produces a minimal local markdown artifact directly. Layer 2's `packages/manifests` package provides the shared renderer used by both desktop and web surfaces.

### 4.2 Run Manifest System (Authoritative Layer 1 Definition)

> **Ownership:** This is the authoritative definition of the run manifest system. Layer-strategy.md §10 carries a compact cross-layer summary. Later layers may include concise recaps for self-containment but must not redefine or contradict this contract.

Every skill execution produces a **hybrid-authored run manifest** — an agent-written narrative combined with a daemon-appended evidence appendix. This is the primary input for semantic validation (Layer 3+) and operator review.

#### Structure

```text
┌─────────────────────────────────────────┐
│ YAML Frontmatter (structured metadata)  │
├─────────────────────────────────────────┤
│ Agent Narrative Sections                │
│ - Summary                               │
│ - Steps Taken                            │
│ - Decisions Made                         │
│ - Warnings                               │
│ - Evidence (with claim/evidence refs)    │
├─────────────────────────────────────────┤
│ Daemon Evidence Appendix (< 2 KB)       │
│ - observed_url_chain                     │
│ - daemon_observed_fingerprint_state      │
│ - daemon_observed_locale/currency        │
│ - checkpoint_count, result_item_count    │
│ - oqg_anomaly_summary                   │
│ - artifact_refs                          │
└─────────────────────────────────────────┘
```

**Storage:** Immutable artifact at `artifacts/sessions/YYYY/MM/<session_id>/run-manifest.md`. Path and SHA-256 hash recorded in `agent_session_logs.run_manifest_path` and `agent_session_logs.run_manifest_hash`.

**Target size:** < 50 KB. The manifest is a focused narrative summary, not a log dump.

#### YAML Frontmatter — Agent-Claimed Fields

Self-reported by the agent. Explicitly labeled with `agent_claimed_*` prefix where ambiguity exists:

```yaml
manifest_version: "1.0"
skill_id: "procurement-ingestion/fts"
skill_version: "1.2.0"
job_id: "job-uuid-here"
session_id: "session-uuid-here"
execution_mode: "pty"
started_at: "2026-03-19T10:00:00Z"
completed_at: "2026-03-19T10:05:23Z"
outcome_class: "completed"   # completed | continuation_required | degraded_completed | blocked_human_action
items_processed: 47
items_skipped: 3
items_failed: 0
warnings_count: 1
agent_claimed_pages_visited: 12
agent_claimed_source_url: "https://www.find-tender.service.gov.uk/..."
agent_claimed_fingerprint_state: "matched"    # matched | mismatched | unknown
agent_claimed_locale_state: "matched"
agent_claimed_currency_state: "matched"
```

#### Agent Narrative Sections

Human-readable, agent-authored Markdown:

- **Summary:** What the agent did and what it found.
- **Steps Taken:** Navigation and extraction sequence.
- **Decisions Made:** Choices the agent made during the run with reasoning.
- **Warnings:** Anomalies, degradations, or issues encountered.
- **Evidence:** Key evidence supporting the results, with claim/evidence references pointing to result contract fields, daemon appendix anchors, or artifact refs. Not every sentence needs a reference — major claims (correct page, correct locale, extraction completeness, pricing plausibility) should be anchored.

#### Daemon Evidence Appendix

Machine-authored, immutable, appended by the daemon after agent narrative. Provides externally verifiable anchors the agent cannot fabricate:

| Field | Source | Purpose |
|-------|--------|---------|
| `observed_url_chain` | Layer 2 protocol events | Actual domains visited (independent of agent claim) |
| `observed_page_kind_sequence` | Adapter events | Page-kind classifications from navigation |
| `daemon_observed_fingerprint_state` | Page fingerprint check | `matched` / `mismatched` / `unknown` / `no_baseline` |
| `daemon_observed_locale` | Protocol events / page content | Detected locale (independent of agent claim) |
| `daemon_observed_currency` | Protocol events / page content | Detected currency |
| `auth_challenge_events` | Layer 1 control events | Count and types of auth/challenge events |
| `locator_fallback_depth` | Navigation events | Maximum fallback tier reached |
| `checkpoint_count` | Layer 2 protocol events | Number of validated checkpoints |
| `result_item_count` | Validated result contract | Item count from contract (independent of agent's claimed count) |
| `oqg_anomaly_summary` | Output Quality Gate | Fill-rate delta, duplicate-rate delta, value-range outliers |
| `representative_item_sample` | Result contract | IDs of median items + OQG-flagged outlier items |
| `artifact_refs` | Artifact store | Paths and hashes for forensic drill-down |

#### `agent_claimed_*` vs `daemon_observed_*` Naming Convention

**Critical rule:** Self-reported fields use `agent_claimed_*`. Daemon-observed fields use `daemon_observed_*`. Fields that sound objective (e.g., `page_fingerprint_matched`, `locale_verified`) must never be ambiguous between "the agent claims it verified this" and "the daemon observed and verified this."

This naming convention prevents false certainty from anchoring operators and validators. Discrepancies between agent-claimed and daemon-observed states are **high-signal issues** that validators should cross-check.

#### Fallback Mode

If the worker agent fails to emit a run manifest (crash, timeout, invalid output), the daemon synthesizes a **minimum machine-authored run manifest** from observability layers 1-3 alone:

- Contains only the daemon evidence appendix (no agent narrative)
- `agent_narrative_missing: true` in frontmatter
- Ensures some structured review artifact always exists
- Enables validators and operators to assess what happened even when the agent could not narrate

The worst runs must not leave the thinnest review surface.

#### Terminology Disambiguation

Three things called "manifest" in this architecture:

| Term | What It Is | Stored In |
|------|-----------|-----------|
| **Opportunity manifest** | Markdown summary of an opportunity for operator review | `manifests` table |
| **Error manifest** | Structured pipeline failure record for debugging | `error_manifests` table |
| **Run manifest** | Hybrid-authored narrative of what a skill run did (this system) | Artifact store, referenced from `agent_session_logs` |

Always use the full term "run manifest" when referring to this concept.

### 4.3 Error Taxonomy & Error Manifest System

> **Ownership:** This section defines the error taxonomy framework and Layer 0-1 base error types. Customs-specific error types (`TARIFF_BRANCH_DIVERGENCE`, `MEASURE_INTERACTION_UNKNOWN`, etc.) are defined in Layer 5 §3. Validation-specific error types (`VALIDATION_FLAGGED_INSUFFICIENT_BASIS`, etc.) are defined in Layer 3 §2. This section defines the framework; later layers extend it with domain-specific codes.

#### Error Categories

| Category | Prefix | Scope | Examples |
|----------|--------|-------|---------|
| Pipeline errors | various | Data quality issues in the intelligence pipeline | `SOURCE_CHANGED`, `NOTICE_PARSE_FAILED`, `COMMODITY_CODE_LOW_CONFIDENCE` |
| Agent errors | `AGENT_*` | Runtime failures in agent sessions | `AGENT_STALLED`, `AGENT_AUTH_CHALLENGE`, `AGENT_SUSPECT_OUTPUT` |
| Host errors | `HOST_*` | Environment/infrastructure issues | `HOST_SLEEP_INTERRUPTED`, `HOST_RESOURCE_EXHAUSTED` |
| Source errors | `SOURCE_*` | Source website/API issues | `SOURCE_SOFT_BLOCKED`, `SOURCE_CAPTCHA_BLOCKED`, `PAGE_SHAPE_DRIFT` |
| Validation errors | `VALIDATION_*` | Semantic validation outcomes (Layer 3+) | `VALIDATION_REJECTED`, `VALIDATION_FLAGGED` |

#### Layer 0-1 Base Error Types

**Pipeline errors (active from Layer 0 intelligence pipeline):**

| Code | Trigger | Action |
|------|---------|--------|
| `SOURCE_CHANGED` | Source website changed | Flag skill needs update; create error manifest |
| `NOTICE_PARSE_FAILED` | Notice parse failed | Store raw payload; mark manual review |
| `GOODS_CLASSIFICATION_LOW_CONFIDENCE` | Goods classification uncertain | Keep notice; require review |
| `SUPPLIER_MATCH_LOW_CONFIDENCE` | Supplier match weak | Do not auto-qualify |
| `COMMODITY_CODE_LOW_CONFIDENCE` | Commodity code uncertain | Route to manual review |
| `REVENUE_BASIS_UNCERTAIN` | Framework ceiling or dynamic market | Margin gate uses discounted revenue |
| `FPO_ACCESS_UNAVAILABLE` | HMRC FPO API unavailable | Skip FPO; continue with remaining recall sources; confidence penalty |

**Agent errors (active from Layer 1 daemon):**

| Code | Trigger | Action |
|------|---------|--------|
| `AGENT_STALLED` | No output or no progress for timeout | Kill process group; create error manifest |
| `AGENT_AUTH_CHALLENGE` | Re-login, MFA, SSO, permission prompt | Flag for operator; create error manifest |
| `AGENT_SKILL_FAILED` | Skill playbook failed at specific step | Create error manifest with skill path and step |
| `AGENT_OUTPUT_CONTRACT_INVALID` | Output fails schema validation | Preserve transcript + artifact; fail job |
| `AGENT_SUSPECT_OUTPUT` | Schema-valid but fails OQG semantic/drift checks | Quarantine results; keep pipeline unchanged |
| `AGENT_PARTIAL_FAILURE` | Valid checkpoint before crash, no final contract | Persist checkpoints; never upgrade to success |
| `AGENT_INTERNAL_ERROR` | Internal error from agent CLI | Create error manifest with trace |
| `AGENT_RUN_FAILED` | Non-zero exit, no specific pattern | Create error manifest |
| `AGENT_CLEANUP_INCOMPLETE` | Descendant processes leaked | Record leaked pids; flag for operator |
| `AGENT_PROTOCOL_MISMATCH` | Unexpected CLI output format | Canary mode; create error manifest |

**Source/skill errors (active from Layer 1):**

| Code | Trigger | Action |
|------|---------|--------|
| `SOURCE_SOFT_BLOCKED` | HTTP 200 but no useful data | Capture artifacts; set source cooldown |
| `SOURCE_CAPTCHA_BLOCKED` | Hard CAPTCHA requiring human | Emit `blocked_human_action`; alert operator |
| `AUTH_STATE_STALE` | Session loads but privileged data missing | Trigger auth-refresh skill; retry |
| `PAGE_SHAPE_DRIFT` | Page fingerprint mismatch | Flag skill needs review; create error manifest |
| `LOCALE_OR_CURRENCY_DRIFT` | Wrong locale/currency/market variant | Fail fast; require config review |

**Host errors (active from Layer 1):**

| Code | Trigger | Action |
|------|---------|--------|
| `HOST_SLEEP_INTERRUPTED` | Monotonic time gap during session | Taint browser sessions; create error manifest |
| `HOST_RESOURCE_EXHAUSTED` | Memory/CPU pressure | May preempt lower-priority sessions |

#### Skill Outcome Classes (Non-Error)

Not every non-success outcome is an error:

| Outcome | When | Daemon Action |
|---------|------|--------------|
| `continuation_required` | Skill exited cleanly at safe boundary with remaining work | Accept committed results; enqueue continuation segment with resume token |
| `degraded_completed` | Skill completed with reduced coverage/confidence | Accept results with degradation flag; apply confidence penalty |
| `blocked_human_action` | Human intervention required (CAPTCHA, manual approval) | Alert operator; pause job; preserve session state |

**Key distinction:** `continuation_required` has a valid final contract with continuation data. `AGENT_PARTIAL_FAILURE` has only prior checkpoints and no final contract.

#### Error Manifest Structure

Every pipeline and agent failure produces an entry in `error_manifests` table and a local Markdown artifact at `artifacts/error-manifests/YYYY/MM/<error_manifest_id>.md`.

**Contents:**

| Field | Description |
|-------|------------|
| Pipeline stage | Which stage failed (ingestion, qualification, enrichment, landed_cost, bidding) |
| Workflow step | Specific step within the stage |
| Error type + code | What went wrong |
| Error message | Human-readable description |
| Error context | Enough to debug: skill path, agent session ID, relevant data |
| Runtime context | CLI vendor + version, adapter version, model id, config hash, skill version, pid/pgid, cleanup verdict, last valid checkpoint sequence |
| Markdown artifact | Header, pipeline summary, redacted context, linked session artifact path, next debugging action |
| Resolution tracking | Resolved status, by whom, when |

Error manifests are debugging artifacts, not control flow. They don't drive pipeline state — they record what went wrong for operator review and debugging.

### 4.4 Skill Linting CI Gate

A CI gate runs on every commit that touches a skill directory. It validates compliance with the production skill directory standard (see Layer 0 §4).

**Checks:**

| Check | Validation |
|-------|-----------|
| `SKILL.md` present | File exists in skill directory root |
| Required metadata present | `interaction_profile`, `result_schema_version`, `model_profile` fields in SKILL.md |
| Trigger boundary sections | "Use when", "Do not use when", "Inputs", "Outputs", "Non-goals" sections in SKILL.md |
| `result.schema.json` present | Valid JSON Schema file |
| `checkpoint.schema.json` present | Valid JSON Schema file |
| `canary.inputs.jsonl` present | Non-empty JSONL file |
| `golden.assertions.yaml` present | Valid YAML file |
| Failure codes enumerated | SKILL.md lists possible failure codes |
| Resume strategy declared | SKILL.md describes resume behavior |
| Unsafe side-effect boundary | Portal/bid skills declare side-effect boundary |
| Bounded pagination/scroll rules | `maxPages`, `maxScrollCycles`, etc. present |
| Run manifest instructions | SKILL.md includes narrative instruction block for agent |

**Implementation:** `skill_lint.rs` in `crates/daemon` — a standalone validation pass that can run in CI without the full daemon.
