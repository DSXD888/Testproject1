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
