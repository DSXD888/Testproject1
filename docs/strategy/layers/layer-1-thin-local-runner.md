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

**`crates/daemon` — 17 core files:**

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
| `ipc/descriptor.rs` | Daemon runtime descriptor: atomic `daemon.info.json` write on ready, exclusive `daemon.lock` via OS-level file lock, stale-socket cleanup |
| `ipc/handshake.rs` | Desktop↔daemon handshake protocol: version negotiation, schema epoch check, `commit_seq` exchange, health summary |
| `sessions/reaper.rs` | Session Reaper (L1-critical): `SIGCHLD` handling, `waitpid(-1, WNOHANG)` drain loop, `pid→session_id` map, orphan/leak scan, cleanup verdict assignment, `cleanup_report.json` emission |

**New skills (2):**

| Skill | Purpose | Interaction Profile |
|-------|---------|-------------------|
| `skills/procurement-ingestion/fts/` | Live FTS notice ingestion via browser automation | `pty_interactive` |
| `skills/manifest-generation/` | Opportunity manifest generation from pipeline results | `headless_contract` |

**Database migration:**

`0003_sqlite_runtime.sql` — creates `agent_session_logs`, `raw_documents`, `error_manifests`, `executors`, `source_cursors`, `commit_clock` tables (all in `state.db` — single-database model).

### 1.6 Ingestion Flow (End-to-End Live Loop)

The first live operational loop executes this sequence via manual CLI trigger:

**Step 1 — Trigger:** Operator runs CLI command (e.g., `cargo run -- ingest fts`).

**Step 2 — Job creation:** Daemon creates a `workflow_jobs` entry with `job_kind = 'sync_source'`, explicit skill id/version binding (`skills/procurement-ingestion/fts`).

**Step 3 — Adapter selection:** Agent Adapter selects execution mode based on the skill's `interaction_profile: pty_interactive`. For FTS ingestion, this is PTY mode (browser automation required).

**Step 4 — Session spawn:** Adapter spawns agent session in its own process group (POSIX `setpgid`). Records `pid`, `pgid`, `adapter_id`, CLI version, and config hash. Session state: `created -> spawned`.

**Step 5 — Skill execution:** Agent navigates to FTS portal via browser automation, following the `procurement-ingestion/fts/SKILL.md` playbook. Agent proves expected page fingerprint before extraction. Session state: `spawned -> active`.

**Step 6 — Incremental retrieval:** Agent uses cursor/watermark from previous run (stored in `source_cursors`). FTS API supports `updatedFrom`, `updatedTo`, `cursor`, `limit`. Durable commit boundaries are notice batches. Resume uses small cursor backtrack window to avoid skipping records.

**Step 7 — Capture:** Raw PTY bytes captured as immutable forensic artifact, compressed and stored at `artifacts/sessions/YYYY/MM/<session_id>/`. Session metadata in `agent_session_logs`.

**Step 8 — Checkpoints and result contract:** Agent writes checkpoints and final outputs through validated result contract. Workflow state advances from validated contracts, NOT from transcript parsing.

**Step 9 — Run manifest generation:** Agent writes narrative sections (Summary, Steps Taken, Decisions Made, Warnings, Evidence). Daemon appends machine-authored evidence appendix (observed URL chain, fingerprint state, checkpoint count, result item count, OQG anomaly summary, artifact refs). If agent fails to narrate -> daemon produces fallback machine-authored manifest with `agent_narrative_missing: true`. Stored at `artifacts/sessions/YYYY/MM/<session_id>/run-manifest.md`.

**Step 10 — Result validation:** Result Contract Validator checks output against `result.schema.json`. Output Quality Gate checks for semantic/statistical anomalies. Schema-valid outputs failing semantic checks are quarantined as `AGENT_SUSPECT_OUTPUT`.

**Step 11 — Pipeline processing:** Notices normalized into internal schema. For Layer 1, relation resolution is skipped (stubbed — full engine in Layer 3). Pipeline stages from Layer 0 §3 execute: prequalify_notice -> acquire_docs -> extract_lots -> qualify_lot -> extract_products -> quoteability -> price match -> gross gate -> landed cost -> net gate -> coverage rollup -> manifest.

**Step 12 — Stage tracking:** In the same SQLite transaction that writes stage outputs, the daemon writes all rows with a shared `stage_run_id`, updates `process_stage_status`, and advances `process_stage_heads` atomically. Failed stages publish failure status but do NOT advance the stage head.

**Step 13 — Session cleanup:** Session state: `active -> terminating -> exited -> reaped -> archived`. Session Reaper performs descendant cleanup (SIGTERM group -> grace period -> SIGKILL group -> orphan sweep). Records cleanup verdict: `clean` | `forced` | `leaked`.

**Step 14 — Verifiable output:** CLI summary printed. Local markdown manifest artifact emitted proving the loop completed. Manifest stored in `manifests` table row.

### 1.7 Adapter-Mediated Hybrid Execution

The daemon triggers agent sessions through an **Agent Adapter** boundary with two execution modes:

#### PTY Mode

Required for browser automation, interactive auth flows, or any CLI behavior needing a controlling terminal. The daemon creates PTY pairs (master/slave), captures full terminal output as an immutable forensic artifact, and monitors session health.

**When to use:** Skill declares `interaction_profile: pty_interactive`. Most procurement ingestion and supplier enrichment skills need PTY mode because they drive browsers.

#### Contract Mode

Preferred for deterministic tasks where the vendor supports structured non-interactive execution. Lower overhead, simpler cleanup, stronger result contracts.

**When to use:** Skill declares `interaction_profile: headless_contract`. Manifest generation, qualification, and other deterministic skills use contract mode.

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

**Two-lane event model (canonical — all adapters produce this):**

The adapter produces two independent event lanes:

*Output lane — lossless raw transport for UI/forensics:*

| Frame | Meaning |
|-------|---------|
| `Pty(Bytes)` | Raw PTY byte chunk (PTY mode) |
| `Stdout(Bytes)` | Raw stdout chunk (contract mode) |
| `Stderr(Bytes)` | Raw stderr chunk (contract mode) |

*Control lane — semantic daemon events:*

| Event | Meaning |
|-------|---------|
| `Started` | Agent session spawned and active (includes pid, pgid, sid, mode) |
| `Checkpoint` | Validated checkpoint persisted (includes seq, path) |
| `Progress` | Progress update (completed, total) |
| `AuthChallenge` | Agent hit an auth/MFA/SSO challenge |
| `Warning` | Non-fatal issue detected |
| `FinalResultReady` | Result contract written (path only — validation is NOT adapter responsibility) |
| `Exited` | Process exited (includes exit status) |

This separation means PTY stays raw and contract mode stays structured without pretending they are the same transport. The output lane feeds capture and UI streaming; the control lane feeds the Job Runner and session lifecycle.

**Result validation is NOT an adapter responsibility.** Schema validation against `result.schema.json` lives in `agents/contracts.rs`. OQG semantic/drift checks live in daemon pipeline code. Adapters emit `FinalResultReady` with the artifact path; the daemon validates afterward.

**Canary mode:** Unknown CLI versions enter canary mode — the adapter runs a smoke probe before accepting production traffic.

**Session isolation:** Concurrent sessions on the same workstation can contaminate each other through shared CLI config. The daemon must: pin `CODEX_HOME`/`.codex` config paths per session, control working directory per run, record CLI version/model/config hash/skill version/adapter version on every session, use per-session browser profiles for browser-backed flows.

### 1.8 Process-Group Supervision & Session Semantics

The daemon supervises the **entire descendant tree**, not just the initial CLI process. Agent CLIs may spawn sub-processes (browser instances, tool servers, language runtimes).

#### Session and Controlling-Terminal Setup (L1-Critical)

`setpgid` alone is not enough. For a real interactive PTY, the daemon must establish full **session** and **controlling terminal** semantics:

1. Create PTY pair (master/slave) via `openpty()`
2. Fork child process
3. In child: `setsid()` — creates a new session and new process group
4. In child: establish slave as controlling terminal (via `login_tty()` or `TIOCSCTTY`)
5. In child: redirect stdin/stdout/stderr to PTY slave
6. Record `sid` (session ID), `pid`, and `pgid`
7. Only then `exec` the agent CLI

Without session and controlling-terminal setup, some interactive CLIs (especially those using job control or checking `isatty()`) will behave incorrectly.

**`CLOEXEC` hygiene (L1-Critical):** The PTY master fd and all daemon control fds not intended for the child must be marked `O_CLOEXEC`. Otherwise a grandchild that inherits the master fd will prevent EOF from ever arriving on the daemon's read side — a silent hang that is extremely difficult to debug.

**Fork safety with Tokio:** The daemon is a multithreaded Tokio process. After `fork()`, the child may safely call only async-signal-safe functions until `execve()`. If the PTY crate (`portable-pty` / `pty-process`) cannot express the exact pre-exec logic needed (setsid, TIOCSCTTY, fd setup), use a tiny spawn helper executable rather than growing custom child-side Rust logic inside the daemon.

**Terminal geometry:** Set a stable default geometry of `120x40` at PTY creation. PTY geometry is for terminal/UI behavior; browser viewport is a separate explicit browser-launch setting. Do not let browser viewport silently depend on PTY size. A `resize_pty(session_id, cols, rows)` command is available for attached sessions; if never resized, the geometry remains constant so transcripts are reproducible.

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

1. Spawn session in its own session (`setsid`) and process group on POSIX
2. Record `sid`, `pid`, `pgid`, PTY id, adapter id at spawn
3. On soft stop: send adapter in-band interrupt if supported (e.g., structured cancel command)
4. If appropriate: write `^C` / equivalent to PTY
5. Escalation: `SIGINT` to process group
6. Escalation: `SIGTERM` to process group
7. After grace period: `SIGKILL` to process group
8. Keep draining PTY master until EOF or drain timeout (PTY mode); treat `EIO` as expected hangup, not error
9. Wait/reap explicitly via `waitpid()` — do NOT use `kill(pid, 0)` as process-completion check, do NOT rely on dropping Tokio child handles
10. Run orphan sweep and record cleanup verdict (`clean` | `forced` | `leaked`)
11. Emit `cleanup_report.json` to artifact store

**Reaper (L1-Critical):** The Session Reaper (`sessions/reaper.rs`) is not deferred to Layer 3. It owns:
- `SIGCHLD` signal handling
- `waitpid(-1, WNOHANG)` drain loop for all daemon children
- `pid → session_id` mapping for associating child exits with sessions
- Explicit orphan/leak scan after session exit
- `cleanup_report.json` emission per session

**Cleanup verdicts:**

| Verdict | Meaning |
|---------|---------|
| `clean` | All descendant processes exited cleanly before timeout |
| `forced` | Process group required SIGTERM or SIGKILL |
| `leaked` | Descendant processes survived group kill — operator attention needed |

**Platform notes:**

- **Linux:** Optional `PR_SET_CHILD_SUBREAPER` is genuinely useful for catching double-fork escapees and daemonized grandchildren. Later, `systemd-run --scope` provides cgroup containment. Process-group signaling via `killpg` is the primary mechanism.
- **macOS:** No Linux subreaper equivalent. Leak detection must come from process inspection and conservative session cleanup. Peer credential verification over UDS uses `getpeereid()`, not Linux socket options.

**Chrome/browser stress case:** If the agent launches a headless browser that forks helper processes, many children stay in the same session/pgid if launched normally, but some helpers can escape. Process-group kill is necessary but not sufficient. The answer is: `kill(-pgid, …)` first, descendant sweep second, per-session browser profile directory, `leaked` verdict if anything survives. On Linux, subreaper or cgroup scope provides stronger containment.

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
| `AGENT_DAEMON_CRASH_DURING_SESSION` | Daemon process crashed while PTY session was active | On restart: detect sessions that were `spawned\|active\|terminating`, kill surviving descendants, synthesize error manifest with last known state; do NOT pretend PTY stream can be resumed |
| `AGENT_RUN_FAILED` | Non-zero exit code with no specific error pattern | Create error manifest |

### 1.10 What's Stubbed in This Layer

| Feature | Layer 1 Stub | Full Implementation |
|---------|-------------|-------------------|
| Scheduler | CLI manual trigger only | Layer 3: cron scheduler with explicit skill binding |
| Resource arbiter | No concurrency limits | Layer 3: typed quotas (browser slots, agent slots, source lanes) |
| PTY hardening | Session/ctty setup, CLOEXEC, EIO/drain handling, reaper, default geometry | Layer 3: advanced fd hygiene edge cases, terminal geometry resize stability under load |
| Job claiming/leases | Single-threaded execution | Layer 3: concurrent job claiming with lease management |
| Source registry | Bootstrapped local seed / hardcoded source definitions | Layer 3: typed registry; Layer 4: `shared_source_configs` pull |
| Procurement coverage | First live source only (FTS) | Layer 2: adds Contracts Finder; Layer 5: PCS if needed |
| Session reaper | SIGCHLD/waitpid drain loop, pid→session map, orphan scan, cleanup verdicts, cleanup_report.json | Layer 3: full escalation tiers with configurable timeouts, diagnostic introspection |
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
- Advanced PTY hardening: resize stability under load, advanced fd edge cases (Layer 3)
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
      main.rs              # CLI entry, argument parsing, daemon bootstrap, signal handling,
                           #   exclusive daemon.lock acquisition, daemon.info.json write on ready,
                           #   session reconciliation on startup (kill surviving PTY descendants,
                           #   synthesize AGENT_DAEMON_CRASH_DURING_SESSION error manifests)
      db/
        sqlite.rs          # Connection management: WAL mode, read-only query pool,
                           #   single-writer enforcement, migration runner, PRAGMA init,
                           #   commit_seq increment on every write transaction,
                           #   db.commit event emission
      pty/
        session.rs         # PTY pair creation (master/slave), session setup (setsid,
                           #   controlling terminal via login_tty/TIOCSCTTY), CLOEXEC on
                           #   master fd, process-group spawn, sid/pid/pgid recording,
                           #   terminal geometry (default 120x40), write_input() for
                           #   interactive PTY, resize() for attached sessions
        capture.rs         # Chunked byte capture from PTY master fd; zstd compression;
                           #   chunked artifact storage at artifacts/sessions/YYYY/MM/<sid>/stream/;
                           #   stream.index.jsonl with per-chunk metadata (first_seq, last_seq,
                           #   byte_length, wall_ts_range, sha256); incremental hashing during
                           #   capture; atomic write (temp file then rename); EIO treated as
                           #   expected hangup signal, not error
        health.rs          # Session health monitoring: no-output stall detection,
                           #   no-progress stall detection, auth challenge detection,
                           #   host sleep/monotonic-gap detection
      jobs/
        runner.rs          # Job Runner: claim from workflow_jobs, execute via adapter,
                           #   persist results, update process_stage_status/heads,
                           #   stage_run_id scoping, parent_job_id awareness for future
                           #   validation sub-jobs, continuation_required handling,
                           #   commit_seq/visible_after_seq on every ack,
                           #   durable commit before MutationAck
      agents/
        adapter.rs         # Agent Adapter trait: two-lane event model (OutputFrame for
                           #   transport, ControlEvent for semantics); mode selection via
                           #   InteractionProfile; LaunchSpec returned (not Command);
                           #   result validation NOT in adapters (lives in contracts.rs)
        codex.rs           # Codex adapter: launch spec construction, capability probe
                           #   (CLI version, auth health, structured output viability),
                           #   AdapterParser implementation for JSONL parsing and
                           #   health hint emission, canary mode
        claude.rs          # Claude adapter: launch spec construction, capability probe,
                           #   AdapterParser implementation for JSON/stream-json parsing
                           #   and health hint emission, canary mode
        artifacts.rs       # Artifact store: chunked compressed immutable session output,
                           #   result contract files, checkpoint artifacts, run manifests;
                           #   stored at artifacts/sessions/YYYY/MM/<session_id>/
        contracts.rs       # Result contract validator: schema validation against
                           #   result.schema.json, checkpoint sequencing, partial failure
                           #   classification; OQG semantic/drift checks — THIS is where
                           #   result validation lives (not in adapter code)
      prompt/
        renderer.rs        # Kickoff prompt composition: SKILL.md content injection,
                           #   job parameters, resume tokens, run manifest instruction
                           #   block, company profile injection (future)
      errors/
        manifest.rs        # Error manifest generation: structured Markdown error records
                           #   with CLI vendor+version, adapter version, model id, config
                           #   hash, skill version, sid/pid/pgid, cleanup verdict, last
                           #   valid checkpoint sequence, daemon instance id, commit_seq
        markdown.rs        # Error manifest Markdown rendering engine
      ipc/
        descriptor.rs      # Daemon runtime descriptor: atomic daemon.info.json write on
                           #   ready (instanceId, pid, socketPath, protocolVersion,
                           #   schemaEpoch, stateRoot, startedAt, ready); exclusive
                           #   daemon.lock via OS-level file lock; stale-socket cleanup
        handshake.rs       # Desktop↔daemon handshake protocol: hello/welcome exchange,
                           #   version negotiation, schema epoch check, commit_seq
                           #   exchange, active sessions summary, health state
      sessions/
        reaper.rs          # Session Reaper (L1-critical): SIGCHLD handling, waitpid drain
                           #   loop, pid→session_id map, orphan/leak scan, cleanup verdict
                           #   assignment, cleanup_report.json emission per session
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

### 2.3 Agent Adapter Trait (Two-Lane Design)

The adapter trait is the abstraction boundary between the daemon and vendor-specific agent CLIs. The key design principle: **unify the control plane, not the transport.**

Two independent event lanes:

- **Output lane**: lossless raw transport for UI streaming and forensic capture. `Pty(Bytes) | Stdout(Bytes) | Stderr(Bytes)`
- **Control lane**: semantic daemon events for job runner and lifecycle. `Started | Checkpoint | Progress | AuthChallenge | Warning | FinalResultReady | Exited`

This separation means PTY stays raw and contract mode stays structured without pretending they are the same thing. The output lane feeds `pty/capture.rs` and desktop `Channel<T>` streaming; the control lane feeds `jobs/runner.rs` and session lifecycle.

```rust
/// Output lane — lossless raw transport
#[derive(Debug)]
pub enum OutputFrame {
    Pty(bytes::Bytes),
    Stdout(bytes::Bytes),
    Stderr(bytes::Bytes),
}

/// Control lane — semantic daemon events
#[derive(Debug)]
pub enum ControlEvent {
    Started { pid: u32, pgid: u32, sid: u32, mode: ExecutionMode },
    Checkpoint { seq: u32, path: PathBuf },
    Progress { completed: u32, total: Option<u32> },
    AuthChallenge { kind: AuthChallengeKind, detail: String },
    Warning { code: String, detail: String },
    FinalResultReady { path: PathBuf },
    Exited { status: ExitStatus },
}

#[derive(Debug, Clone, Copy)]
pub enum ExecutionMode {
    Pty,       // Full PTY session with terminal semantics
    Contract,  // Structured JSON/JSONL non-interactive session
}

/// What the adapter returns — daemon handles PTY/session/process setup
pub struct LaunchSpec {
    pub program: PathBuf,
    pub args: Vec<OsString>,
    pub env: BTreeMap<OsString, OsString>,
    pub cwd: PathBuf,
}

/// Agent Adapter trait — each vendor implements this
#[async_trait]
pub trait AgentAdapter: Send + Sync {
    /// Adapter identity
    fn id(&self) -> &'static str;

    /// Probe CLI capabilities: version, auth health, structured output
    /// viability, PTY viability, browser/tool viability.
    /// Unknown versions enter canary mode.
    async fn probe(&self, ctx: &ProbeCtx) -> Result<CapabilityReport>;

    /// Select execution mode based on skill's interaction_profile
    /// and probe results.
    fn resolve_mode(
        &self,
        skill: &SkillManifest,
        caps: &CapabilityReport,
    ) -> Result<ExecutionMode>;

    /// Construct the launch specification: binary path, flags, env vars,
    /// working directory, config paths (session-isolated).
    /// Returns LaunchSpec — the daemon handles PTY/session/process setup.
    fn build_launch_spec(
        &self,
        req: &SpawnRequest,
        mode: ExecutionMode,
    ) -> Result<LaunchSpec>;

    /// Return a parser that converts vendor-native output into
    /// ControlEvents. The output lane (raw bytes) flows separately.
    fn parser(&self, mode: ExecutionMode) -> Box<dyn AdapterParser>;
}

/// Parser sub-trait — vendor-specific output → ControlEvent translation
#[async_trait]
pub trait AdapterParser: Send {
    /// Called with each output frame. Returns any ControlEvents derived
    /// from the output (e.g., parsing structured JSON from contract mode,
    /// detecting auth challenge patterns in PTY output).
    async fn on_output(&mut self, frame: &OutputFrame) -> Result<Vec<ControlEvent>>;

    /// Called after session exits. Returns final ControlEvents
    /// (e.g., summarizing what was observed).
    async fn finish(&mut self) -> Result<Vec<ControlEvent>>;
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

/// Interaction profile — no silent fallback
#[derive(Debug, Clone)]
pub enum InteractionProfile {
    PtyRequired,                              // Browser automation — must have PTY
    ContractRequired,                         // Structured JSON — must have contract mode
    ContractPreferred { allow_pty_fallback: bool }, // Prefers contract, may fall back
}
```

**Key design decisions:**

1. **Adapters return `LaunchSpec`, not `Command`.** PTY/session/process-group setup is centralized in `pty/session.rs`, not scattered across vendor adapters.
2. **Result validation is NOT adapter responsibility.** `result.schema.json` is a skill contract, not a vendor contract. Validation lives in `agents/contracts.rs`. Adapters emit `FinalResultReady` with the artifact path; the daemon validates afterward.
3. **No silent contract→PTY fallback.** A skill declared as `ContractRequired` never silently slides into PTY mode. Only skills with `ContractPreferred { allow_pty_fallback: true }` may degrade.
4. **Adapters emit health hints, not final verdicts.** Health classification from vendor-specific signals is advisory; the daemon session lifecycle owns the authoritative health state.

**Mode selection logic:**

| `interaction_profile` | Probe: structured output | Probe: PTY viable | Selected Mode |
|----------------------|-------------------------|-------------------|---------------|
| `PtyRequired` | any | must be true | PTY |
| `ContractRequired` | must be true | any | Contract |
| `ContractPreferred { allow_pty_fallback: true }` | true | any | Contract |
| `ContractPreferred { allow_pty_fallback: true }` | false | true | PTY (explicit fallback) |
| `ContractPreferred { allow_pty_fallback: false }` | false | any | Error: cannot execute |
| any | false | false | Error: cannot execute |

A skill declared as `ContractRequired` must never silently slide into PTY mode. The `ContractPreferred` profile with `allow_pty_fallback: false` is the safe default for deterministic skills. Only skills that explicitly opt in to PTY fallback via `allow_pty_fallback: true` may degrade.

### 2.4 Session Lifecycle (7 Phases)

Every agent session transitions through exactly 7 phases. No phase can be skipped.

```text
created -> spawned -> active -> terminating -> exited -> reaped -> archived
```

| Phase | Responsibilities | Failure Mode |
|-------|-----------------|-------------|
| **created** | Session record created in `agent_session_logs`. Adapter selected. Kickoff prompt composed. Skill loaded. Working directory prepared. Session-isolated config paths set. | Config error → job fails before spawn |
| **spawned** | Process launched in own session (`setsid`) and process group. Controlling terminal established (`login_tty`/`TIOCSCTTY`). `sid`, `pid`, `pgid`, PTY id recorded. PTY master fd opened with `CLOEXEC` (PTY mode). Chunked capture stream started. | Spawn failure → `AGENT_RUN_FAILED` |
| **active** | Agent executing. Raw bytes captured. Health monitoring active: no-output timeout, no-progress timeout, auth challenge detection. Checkpoints validated as they arrive. | Stall → `AGENT_STALLED`; auth → `AGENT_AUTH_CHALLENGE`; host sleep → `HOST_SLEEP_INTERRUPTED` |
| **terminating** | Clean shutdown initiated. In-band interrupt sent if adapter supports it. Wait for graceful exit with timeout. | Timeout → escalate to SIGTERM group |
| **exited** | Child process exited (detected via `waitpid`, not `kill(pid,0)`). Exit code recorded. PTY master drained to EOF or drain timeout (PTY mode) — `EIO` treated as expected hangup (slave closed first), not error. Raw stdout/stderr capture finalized (contract mode). Internal drain substates: `pty_hangup_seen` → `draining` → `drain_deadline`. | EIO on PTY read → expected hangup; drain timeout → proceed to reap |
| **reaped** | Session Reaper cleans up descendant tree: SIGINT group → SIGTERM group → grace period → SIGKILL group → orphan sweep. Cleanup verdict recorded: `clean` / `forced` / `leaked`. `cleanup_report.json` emitted. Final result contract validated (in `agents/contracts.rs`, not adapter). Run manifest finalized (agent narrative + daemon evidence appendix). | Leaked descendants → `AGENT_CLEANUP_INCOMPLETE` |
| **archived** | Session complete. All artifacts stored and indexed. Session metadata finalized in `agent_session_logs`. Job status updated in `workflow_jobs`. Stage status/heads updated if pipeline job. | None — terminal state |

**Session completeness rule:** A session is only complete after ALL of:
1. Child exit observed (via `waitpid`, not `kill(pid, 0)`)
2. PTY drained to EOF or drain timeout (PTY mode only); `EIO` is expected hangup
3. Descendant process tree verified clean (not assumed) — reaper confirms
4. Final result contract validated in `agents/contracts.rs` (if present)
5. Cleanup verdict recorded (`clean` | `forced` | `leaked`)
6. `cleanup_report.json` emitted to artifact store
7. Run manifest stored (agent narrative + daemon evidence appendix; machine-authored fallback if agent fails to narrate)

### 2.5 Artifact Store

Raw session output is stored as compressed immutable artifacts **outside SQLite** — session byte streams are too large for inline database storage.

**Storage layout (chunked from day 1):**

The artifact store uses **chunked capture** from Layer 1. This is not a future optimization — it is required for L2 desktop reconnect, gap recovery, lazy scrollback, and the live→historical handoff.

```
artifacts/
  sessions/
    YYYY/
      MM/
        <session_id>/
          stream/
            000001.pty.zst     # Compressed PTY byte chunk (PTY mode)
            000002.pty.zst
            000003.pty.zst
            ...
          stream/
            000001.stdout.zst  # Compressed stdout chunk (contract mode)
            000001.stderr.zst  # Compressed stderr chunk (contract mode)
            ...
          stream.index.jsonl   # Chunk index: per-line JSON with metadata
          run-manifest.md      # Hybrid-authored run manifest
          result.json          # Validated result contract
          checkpoint-001.json  # Checkpoint artifacts
          checkpoint-002.json
          cleanup-report.json  # Session reaper cleanup report
          ...
```

**Chunk index format (`stream.index.jsonl`):**

Each line is a JSON object describing one chunk:

```json
{"chunk": "stream/000001.pty.zst", "first_seq": 1, "last_seq": 847, "byte_length": 32768, "wall_ts_start": "2026-03-19T10:00:00Z", "wall_ts_end": "2026-03-19T10:00:33Z", "sha256": "abc123..."}
```

**Why chunked:** Monolithic `raw-pty.zst` is incompatible with the L2 viewer model. The desktop needs: reconnect after desktop restart (resume from `lastSeq`), gap recovery after slow consumer (fetch chunks covering `fromSeq..toSeq`), lazy history fetch (load only visible chunks), and live-tail handing off to historical transcript. All of these require chunk-addressable capture.

**Capture rules:**
- Chunks are written to temp files, then atomically renamed (crash-safe)
- Hashing is incremental during capture, not computed after the fact
- `stream.index.jsonl` is append-only during capture, finalized at session archive
- Target chunk size: ~32 KB compressed (tunable)
- Each chunk covers a contiguous `seq` range with no gaps

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

**Commit fence model:**

Every successful daemon write transaction increments a monotonic `commit_seq` (stored in `commit_clock` table). After commit, the daemon emits a `db.commit { commitSeq, touchedScopes }` event. Every `MutationAck` returns `visibleAfterSeq` — the `commit_seq` at which the mutation became durable. The desktop uses this to implement read-your-writes consistency without routing reads through the daemon (see Layer 2 §1.4).

**Durability rule:** `MutationAck.accepted = true` means "durably committed in SQLite," never "I have it in RAM." The ack is sent only after the write transaction commits.

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
   - Increment `commit_seq` in `commit_clock`
   - Return `visibleAfterSeq` in ack
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

**Single-database model:**

All metadata lives in **one SQLite database** (`state.db`). Large immutable payloads (PTY byte streams, run manifests, checkpoint files, downloaded documents) live in the filesystem artifact store.

The previous two-database split (`state.db` + `artifacts.db`) was rejected because SQLite does not provide cross-database atomicity — even attached-database transactions are atomic per database, not across all databases together. This created an ugly class of crash-consistency bugs: "job row committed but session metadata missing" or "error manifest exists but stage status did not advance." Collapsing all metadata into one database eliminates this failure class entirely while the big payloads are already outside SQLite anyway.

| Database | Contents |
|----------|----------|
| `state.db` | Pipeline entities, jobs, manifests, stage tracking, outbox (future), sync metadata (future), agent session logs (metadata), raw documents (metadata), error manifests, executors, source cursors, commit clock |

Actual session byte streams (chunked `.zst` files), run manifests, checkpoint files, and downloaded documents are stored in the filesystem artifact store, not in SQLite.

**Commit clock table:**

```sql
CREATE TABLE commit_clock (
  id INTEGER PRIMARY KEY CHECK (id = 1),  -- singleton row
  commit_seq INTEGER NOT NULL DEFAULT 0
);
```

Every successful daemon write transaction increments `commit_seq`. This provides the global visibility fence for desktop read-your-writes consistency.

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

### 2.8 Daemon Discovery & IPC Protocol

#### Runtime Descriptor File

When the daemon is fully ready (lock acquired, DB open, socket bound), it atomically writes `daemon.info.json`:

```json
{
  "instanceId": "uuid",
  "pid": 12345,
  "socketPath": "/path/to/daemon.sock",
  "protocolVersion": 1,
  "schemaEpoch": 1,
  "stateRoot": "/path/to/state",
  "startedAt": "2026-03-19T10:00:00Z",
  "ready": true
}
```

Stored alongside `daemon.lock` and `daemon.sock` in the runtime directory. This solves discovery (desktop reads descriptor to find socket), stale-socket detection (compare PID with running processes), version checks (protocol version in descriptor), and multiple-daemon confusion (lock file prevents duplicates).

#### OS-Level Single-Writer Lock

The `executors` table is not sufficient for local uniqueness. The daemon must acquire an exclusive OS-level file lock (`daemon.lock`) before opening the write connection and before binding the socket. This prevents two daemon instances from running concurrently — a failure mode that would corrupt SQLite (single-writer assumption).

#### Desktop↔Daemon Handshake

On connection, the desktop sends a `hello` message:

| Field | Purpose |
|-------|---------|
| `desktopVersion` | Desktop build version |
| `supportedProtocolRange` | Min/max protocol versions supported |
| `expectedSchemaEpoch` | Schema epoch the desktop expects |
| `lastSeenInstanceId` | Previous daemon instance (for reconnect detection) |

The daemon responds with `welcome`:

| Field | Purpose |
|-------|---------|
| `selectedProtocolVersion` | Negotiated protocol version |
| `daemonVersion` | Daemon build version |
| `instanceId` | Current daemon instance UUID |
| `schemaEpoch` | Current schema epoch |
| `commitSeq` | Current commit clock value |
| `healthState` | Daemon health summary |
| `mutationsAllowed` | Whether mutations are currently accepted |
| `activeSessions` | Summary of active sessions (count, ids, states) |

If `lastSeenInstanceId` differs from `instanceId`, the desktop knows the daemon restarted — it must not attempt to resume PTY streams from previous sessions.

#### Daemon Crash vs Desktop Restart

These are fundamentally different recovery scenarios:

**Desktop restart (daemon stays alive):**
- Read descriptor file, handshake, pull active-session summary
- Subscribe from last seen `seq` — streams are resumable
- No SQLite read-lock issue if QueryService transactions are short-lived

**Daemon crash (desktop may or may not be running):**
- PTY master fds die with the daemon process — streams are NOT resumable
- On daemon restart: detect sessions that were `spawned|active|terminating`
- Kill any surviving descendants
- Synthesize `AGENT_DAEMON_CRASH_DURING_SESSION` error manifests
- The desktop detects the restart via `instanceId` mismatch in handshake

The docs must never conflate these two scenarios. Desktop reconnect is cheap; daemon crash recovery is lossy.

#### L1 IPC Scope

In Layer 1, the daemon **binds the UDS socket and listens** from startup. However, the only command handled in L1 is the `hello`/`welcome` handshake (for tooling, monitoring, and future L2 connection). No mutation commands are accepted in L1 — those are added when L2 activates the DaemonBridge command handlers. The socket is active in L1 so that daemon health can be queried programmatically, but the operator surface is CLI-only.

#### Lifecycle Events vs Adapter Events

The `ControlEvent` enum (§2.3) is the **adapter-level** semantic stream. The daemon session lifecycle manager emits **additional lifecycle frames** for phases that are above the adapter boundary:

| Phase | Source | How It Reaches `Channel<T>` |
|-------|--------|----------------------------|
| `created` | Daemon session manager | Direct lifecycle frame (before adapter runs) |
| `spawned` | Adapter → `ControlEvent::Started` | Translated to lifecycle frame |
| `active` | Adapter → `ControlEvent::Started` | Same event, phase transition |
| `terminating` | Daemon session manager | Direct lifecycle frame (daemon initiates cleanup) |
| `exited` | Adapter → `ControlEvent::Exited` | Translated to lifecycle frame |
| `reaped` | Session Reaper | Direct lifecycle frame with `cleanupVerdict` field |
| `archived` | Daemon session manager | Direct lifecycle frame with `archiveSeq` field |

The `reaped` lifecycle frame **must include `cleanupVerdict`** (`clean`/`forced`/`leaked`) so the desktop can display the verdict in real-time without waiting for a query refresh.

#### UDS Security

Use a **pathname UDS** (not abstract Linux socket). Pathname sockets let directory ownership/permissions control access:

- Runtime directory: `0700` ownership
- Socket inside that directory
- Verify peer UID on connect (macOS: `getpeereid()`; Linux: `SO_PEERCRED`)
- Reject any peer not matching the daemon/desktop user

---

## 3. Schema, Artifacts & Stage Tracking

> **Ownership:** This section owns the **stage tracking model** — the authoritative mapping of pipeline stages to scope grains and commit semantics. Stage **processing logic** (what each stage computes) is owned by Layer 0 §3. Full **DDL for all tables** is owned by Layer 0 §2 (constitutional schema).

### 3.1 Tables Introduced in Layer 1

These tables are created in the Layer 1 migration (`0003_sqlite_runtime.sql`) within `state.db`. All tables share one database for crash-consistent atomic transactions.

#### 3.1.1 `agent_session_logs` (Local-Only)

**Purpose:** Full session transcript/trace for every agent execution. Contains execution identity, process supervision metadata, raw output artifact pointers, error handling, and run manifest references.

**Key columns activated in L1:**

| Column | Purpose |
|--------|---------|
| `job_id` | Links session to the workflow job that triggered it |
| `skill_path`, `skill_version` | Which skill ran and at what version |
| `execution_mode` | `'pty'` or `'contract'` — adapter mode used |
| `cli_vendor`, `cli_version`, `adapter_version` | Execution identity for debugging adapter issues |
| `pty_session_id`, `pid`, `pgid` | Process-group supervision identifiers |
| `status` | Session lifecycle: `created` -> `spawned` -> `active` -> `terminating` -> `exited` -> `reaped` -> `archived` |
| `terminal_output_path`, `terminal_output_hash`, `terminal_output_bytes` | Artifact pointers to compressed raw PTY bytes or contract-mode stdout/stderr |
| `terminal_output_tail` | Last N KB for quick inspection (local only; Supabase gets redacted excerpt) |
| `structured_trace` | Full parsed trace locally; Supabase stores redacted summaries only |
| `cleanup_verdict` | `clean` / `forced` / `leaked` — mandatory for every session |
| `run_manifest_path`, `run_manifest_hash` | Artifact path and SHA-256 of the hybrid-authored run manifest |
| `last_checkpoint_seq` | Last valid checkpoint before failure (for resume logic) |

**Authority:** Local-only. Too large and sensitive for sync. A redacted summary is pushed as `workflow_job_summaries` in Layer 4.

#### 3.1.2 `raw_documents` (Local-Only)

**Purpose:** Downloaded tender document payloads from live source acquisition. Either stored inline as `payload_json` or as file reference via `storage_path`.

**Key columns:** `source_id`, `content_type`, `source_url`, `payload_json` OR `storage_path`, `metadata`.

**CHECK constraint:** `payload_json IS NOT NULL OR storage_path IS NOT NULL` — at least one storage form required.

**Authority:** Local-only. Per-daemon operational state.

#### 3.1.3 `error_manifests` (Local-Only)

**Purpose:** Full error context for every pipeline failure. Includes job reference, pipeline stage, workflow step, structured error context, agent session link, and Markdown artifact path for human-friendly debugging.

**Key columns:** `job_id`, `process_id`, `pipeline_stage`, `workflow_step`, `error_type`, `error_message`, `error_context` (JSON — full locally, redacted in Supabase), `skill_path`, `agent_session_id`, `markdown_path`, `resolved`, `resolved_at`, `resolved_by`.

**Authority:** Local-only. Summaries pushed as `error_manifest_summaries` in Layer 4.

#### 3.1.4 `executors` (Push-Owned)

**Purpose:** Daemon instance identity. Single row in MVP (one daemon per org).

**Key columns:** `device_name`, `status`, `last_heartbeat_at`, `owner_id`, `row_version`, `last_local_seq`.

**Executor identity split** (future-proofed for multi-executor):

| Field | Purpose | When Populated |
|-------|---------|----------------|
| `owner_id` | Org-level identity — which organization this daemon belongs to | From first boot (single value in MVP) |
| `executors.id` | Daemon instance UUID — which physical daemon process | From first boot |
| `push_stream_id` | Push stream identity for sync batching (planned L4) | From sync activation (L4) |

In MVP (single daemon per org), `owner_id` is a constant, `executors.id` is the single daemon's UUID, and `push_stream_id` is not yet used. The active-writer lease (one daemon per org enforced at push protocol level) is activated in Layer 4.

**Authority:** Push-owned. Heartbeat summary synced to Supabase in Layer 4.

#### 3.1.5 `source_cursors` (Local-Only)

**Purpose:** Per-source ingestion watermarks for incremental retrieval. Each source has one or more cursor keys (e.g., `last_published_date`, `last_page_number`) with cursor values.

**Key columns:** `source_id`, `cursor_key`, `cursor_value`, `updated_at`. Primary key: `(source_id, cursor_key)`.

**Authority:** Local-only. Per-daemon operational state.

### 3.2 Tables Activated in Layer 1

These tables were created in Layer 0 (with full schema including all future-layer columns as nullable) and first used by the gold-set evaluation harness. In Layer 1, they receive their **first live-source writes** via the daemon-managed pipeline loop.

| Table | What Changes in L1 |
|-------|-------------------|
| `workflow_jobs` | Now actively used by Job Runner for job lifecycle. `parent_job_id`, `validation_required`, `validation_verdict_id` remain nullable (validation system activates in L3). |
| `process_stage_status` | Now actively written by Job Runner — records execution status per stage per scope grain. Every pipeline stage execution creates a row. |
| `process_stage_heads` | Now actively written — advances atomically when a stage succeeds for its full scope. Points to the latest successful `stage_run_id`. |
| `tender_document_sets` | First live-source document acquisition writes (gold-set was pre-curated). |
| `manifests` | First live-source verifiable output — local markdown/file artifact + database row. |
| `pipeline_assessments` | First live-source process rollups. `validation_state` starts as `'unverified'` from first write. |
| `procurement_processes` | First live-source process creation from FTS notices. |
| `contract_notices` | First live-source notice records. |
| `notice_lots` | First live-source lot decomposition. |
| `product_candidates` | First live-source extraction results. |
| `search_profiles`, `search_attempts`, `supplier_quotes` | First live-source marketplace search results. |
| `candidate_pricing_assessments`, `landed_cost_estimates` | First live-source margin computation results. |
| `qualification_runs` | First live-source qualification decisions. |

**Sync tracking columns** (`owner_id`, `row_version`, `last_local_seq`, `stage_run_id`) are **populated on every write** from Layer 1, even though the sync engine is not yet active. This is a constitutional requirement — sync tracking is always populated, outbox entries are deferred to Layer 4.

### 3.3 Stage Tracking Model (Authoritative)

> **This section is the authoritative source for the stage-to-scope-grain mapping.** Layer 0 §3 owns stage processing logic. This section owns stage tracking infrastructure.

#### 3.3.1 Core Concepts

**`stage_run_id`** = one atomic commit cohort for one scoped business unit. A single agent session (`session_execution_id`) may produce multiple `stage_run_id` commits. For example, one eBay enrichment session may produce 5 committed `stage_run_id`s (one per candidate-source pair that completed).

**`process_stage_status`** records every execution of every stage at every scope grain. Each row has a `stage_run_id`, state, freshness tracking, and basis hash.

**`process_stage_heads`** tracks the current (latest successful) stage run per `(process_id, stage, scope_kind, scope_id)` tuple. Updated atomically when a stage succeeds for its full scope.

**`scope_kind`** is constrained to exactly six values: `process`, `notice`, `lot`, `candidate`, `candidate_source_pair`, `candidate_quote_scenario`.

#### 3.3.2 Stage-to-Scope-Grain Mapping

| # | Stage | `scope_kind` | Commit Semantics | Rationale |
|---|-------|-------------|------------------|-----------|
| 1 | **Ingest** | *(not in `process_stage_status`)* | Source-batched precursor | Tracked at `workflow_jobs` / `source_cursors` level; `process_stage_status` begins once notice/process identity exists |
| 2 | **Normalize** | `notice` | Set-replacement | Normalization produces one authoritative version per notice |
| 3 | **Relate** | `process` | Set-replacement | Relation graph is one coherent classification per process cluster |
| 4a | **Prequalify notice** | `notice` | Set-replacement | Notice-grain provisional qualification enables early rejection before document work |
| 4b | **Qualify lot** | `lot` | Set-replacement | Lot-varying final decisions cannot be process-level |
| 5 | **Acquire docs** | `notice` | Set-replacement | Document set is one coherent acquisition per notice |
| 6 | **Extract lots** | `notice` | Set-replacement | Partial lot set is semantically unsafe — atomic for all lots in notice |
| 7 | **Extract products** | `lot` | Additive | Lots are independent; failure on lot 3 does not invalidate lots 1-2 |
| 8 | **Score quoteability** | `candidate` | Additive | Candidates are independent |
| 9 | **Price-signal match** | `candidate_source_pair` | Additive | Completed pairs survive when others fail |
| 10 | **Gross margin gate** | `candidate` | Additive | Gate decision is per-candidate |
| 11 | **Landed cost** | `candidate_quote_scenario` | Additive | Each scenario is an independent estimate |
| 12 | **Net margin gate** | `candidate` | Additive | Gate decision is per-candidate |
| 13 | **Manifest** | `process` | Set-replacement | Manifest is one coherent narrative per process |
| 14 | **Operator decision** | `process` | Set-replacement | Decision is one coherent judgment per process |
| 15 | **Bid workflow** | `process` | Set-replacement | Bid is one coherent submission per process |

#### 3.3.3 Commit Semantics

**Set-replacement stages** must commit atomically for their full scope. Partial results from set-replacement stages (e.g., lots 1-7 of 12 extracted) are stored as checkpoint/debug artifacts but do NOT advance the stage head. The stage must succeed for the complete scope or fail entirely.

**Additive stages** may commit independently per their scope grain. If product extraction succeeds for lots 1-2 but fails on lot 3, lots 1-2 are authoritative. If eBay search succeeds for candidates 1-5 but fails on candidate 6, candidates 1-5 have valid quotes.

#### 3.3.4 Stage Head Advancement Rules

1. **Set-replacement:** Stage head advances only after ALL outputs for the scope unit are committed in a single transaction. The `stage_run_id` ties all outputs together.
2. **Additive:** Stage head advances per individual scope unit. Each committed unit gets its own `stage_run_id`.
3. **Atomic pair:** Stage output write + `stage_run_id` assignment + stage head advancement happen in the same SQLite transaction. Never split across transactions.
4. **Freshness tracking:** `process_stage_heads.freshness_state` starts as `'fresh'`. Upstream changes may set it to `'soft_stale'` or `'hard_invalid'` (see staleness model below).

#### 3.3.5 Staleness and Invalidation Model

**Hard invalidation** — the semantic input changed. Downstream outputs are no longer authoritative.

Examples: new document set acquired, lot set changed, product extraction re-run, relation resolution changed process cluster, quoteability formula changed AND score crossed a band boundary or query fingerprint changed.

Action: mark affected downstream scoped heads as `hard_invalid`, keep old rows for audit, exclude from current rollup, enqueue rerun from earliest invalid stage.

**Soft staleness** — semantics still valid, but time-sensitive inputs have aged.

Examples: quote TTL expired (source-specific: 24h-96h), FX snapshots aged out, freight quotes aged out, tariff snapshot changed without product semantics changing.

Action: mark affected rows as `soft_stale`, keep them viewable, block auto-qualification if soft-stale input is decision-critical (margin within 10pp of gate), rerun only affected later stages.

**Cascade rules:**

| Upstream Change | Re-run From | Cascade Condition |
|-----------------|-------------|-------------------|
| Notice material content changed | normalize -> relate -> qualify | Only if normalized fields materially changed |
| Document set hash changed | extract_lots onward | Full cascade for affected notice/lots |
| Lot set changed | extract_products onward | For affected lots only |
| Candidate lineage/archetype changed | score_quoteability onward | For affected candidates only |
| Quoteability formula version changed | score_quoteability | Cascade to price_match only if band boundary crossed or query fingerprint changed |
| Supplier quote expired / refreshed | gross_gate onward | For affected candidate only |
| FX / freight / tariff refresh | landed_cost onward | Soft staleness only unless rate change exceeds threshold |
| Manual RFQ quote arrives | gross_gate onward | For that candidate only — new SupplierQuote(quoteClass=rfq_quote) |

**Guard rule:** Every job carries `expected_input_basis_hash`. At claim time, the Job Runner compares it to current scope heads. If mismatched, mark job obsolete and enqueue a new job from the correct earliest invalid stage. This prevents out-of-order execution and stale retries.

### 3.4 Source Configuration (Layer 1 Stub)

In Layer 1, source configuration is **bootstrapped locally** — hardcoded or seeded runtime definitions in the `sources` table. There is no remote-owned `shared_source_configs` pull yet (that activates in Layer 4).

The `sources` table is a local-only runtime copy. In Layer 4, it will be populated from `shared_source_configs` during pull sync. In Layer 1, it contains:

| Source | `kind` | `config` contents |
|--------|--------|-------------------|
| Find a Tender | `procurement` | Base URL, OCDS API endpoint, rate limits |

Contracts Finder is added in Layer 2.

### 3.5 Job Runner Integration

The Job Runner uses the tables above as follows:

1. **Job creation:** A trigger (CLI in L1, cron in L3) creates a `workflow_jobs` row with `kind`, `scope_kind`, `scope_id`, `input_payload` (including `expected_input_basis_hash`).
2. **Job claim:** The runner claims the job (sets `lease_owner`, `lease_expires_at`, `started_at`, increments `attempt_count`). Single-threaded execution in L1 (concurrent claiming in L3).
3. **Skill execution:** The runner selects the adapter (PTY or contract) based on the bound skill's `interaction_profile` and launches the agent session. An `agent_session_logs` row is created.
4. **Stage output commit:** On skill completion, the runner validates the result contract, writes pipeline table outputs with `stage_run_id`, creates a `process_stage_status` row, and advances `process_stage_heads` — all in one SQLite transaction.
5. **Run manifest capture:** The runner stores the agent-authored narrative + daemon evidence appendix as an immutable artifact. `run_manifest_path` and `run_manifest_hash` are written to `agent_session_logs`.
6. **Error handling:** On failure, an `error_manifests` row is created with full context. The `agent_session_logs` row gets `error_type`, `error_message`, and `cleanup_verdict`.
7. **Process-group cleanup:** The runner ensures the session's process group is fully reaped. `cleanup_verdict` is mandatory: `clean` (normal exit), `forced` (kill escalation needed), `leaked` (orphaned processes detected).

### 3.6 Index Design

Beyond primary keys, the following indices support critical query paths in Layer 1:

```sql
-- Job scheduling and claiming
CREATE INDEX idx_workflow_jobs_status_scheduled ON workflow_jobs(status, scheduled_for) WHERE status IN ('pending', 'retry');
CREATE INDEX idx_workflow_jobs_process ON workflow_jobs(process_id);

-- Stage tracking lookups
CREATE INDEX idx_process_stage_status_process ON process_stage_status(process_id, stage, scope_kind);
CREATE INDEX idx_process_stage_status_run ON process_stage_status(stage_run_id);

-- Session logs by job
CREATE INDEX idx_agent_session_logs_job ON agent_session_logs(job_id);

-- Error manifests by process
CREATE INDEX idx_error_manifests_process ON error_manifests(process_id);
CREATE INDEX idx_error_manifests_unresolved ON error_manifests(resolved, created_at) WHERE resolved = false;

-- Source cursors for incremental ingestion
CREATE INDEX idx_source_cursors_source ON source_cursors(source_id);
```

### 3.7 Forward-Compatibility Provisions

| Provision | Target Layer | Status in L1 |
|-----------|-------------|-------------|
| Sync tracking columns on all push-owned writes | L4 | Populated from first write; no outbox entries yet |
| `workflow_jobs.parent_job_id` | L3 | Nullable, unused |
| `workflow_jobs.validation_required` | L3 | Default `false`, unused |
| `workflow_jobs.validation_verdict_id` | L3 | Nullable, unused |
| `pipeline_assessments.validation_state` | L3 | Default `'unverified'` from first write |
| `agent_session_logs.run_manifest_path/hash` | L3 | Populated from first skill run |
| Adapter trait designed for extension | Future | New agent CLIs via new adapter implementations |
| Error manifest format includes validator fields | L3 | Fields exist, not populated |

---

## 4. Skills, Run Manifests & Error Manifests

### 4.1 New Skills (Layer 1)

Two skills are introduced in Layer 1. Both follow the production skill directory standard established in Layer 0 §4 (SKILL.md + result.schema.json + checkpoint.schema.json + canary.inputs.jsonl + golden.assertions.yaml).

#### `skills/procurement-ingestion/fts/`

| Attribute | Value |
|-----------|-------|
| **Purpose** | Ingest live procurement notices from Find a Tender (FTS) via browser automation |
| **Interaction profile** | `pty_interactive` — requires PTY mode (browser-driven navigation) |
| **Model profile** | `browse_stable_low_cost` — stable browser automation on a public source |
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
| **Interaction profile** | `headless_contract` — deterministic, no browser needed |
| **Model profile** | `structured_extract_low_cost` — structured narrative synthesis from pipeline data |
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
| `AGENT_DAEMON_CRASH_DURING_SESSION` | Daemon crashed while PTY session was active | On restart: detect running/orphaned sessions, kill descendants, synthesize error manifest; PTY stream is NOT resumable |
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
| Unsafe side-effect class | Portal/bid skills declare side-effect class per step |
| Bounded pagination/scroll rules | `maxPages`, `maxScrollCycles`, etc. present |
| Run manifest instructions | SKILL.md includes narrative instruction block for agent |

**Implementation:** `skill_lint.rs` in `crates/daemon` — a standalone validation pass that can run in CI without the full daemon.
---

## 5. Testing & Acceptance

### 5.1 Definition of Done

All items must pass before Layer 1 is complete:

| # | Criterion | Verification |
|---|----------|-------------|
| 1 | `cargo check --all-targets` and `cargo clippy --all-targets -- -D warnings` pass | Automated: CI gate |
| 2 | Rust binary launches skills via CLI (both PTY and contract mode) | Manual: `./daemon --run-skill skills/supplier-price-enrichment/ebay --mode pty` |
| 3 | Manual CLI trigger ingests a live FTS batch using source-batched skill execution | Manual: `./daemon --ingest fts` produces notice records |
| 4 | First live end-to-end local loop completes for at least one notice: normalize -> qualify -> acquire docs -> extract lots -> extract products -> price-signal match -> gross gate -> landed cost -> manifest | Manual: verify manifest artifact exists |
| 5 | Verifiable output exists without later layers: CLI summary and/or local markdown manifest artifact proves the loop completed | Manual: inspect output file |
| 6 | Process-group supervision verified: every session gets pgid, cleanup verdict recorded (clean/forced/leaked) | Automated: test + manual inspection of `agent_session_logs` |
| 7 | Artifact capture works: PTY bytes compressed and stored, session metadata in `agent_session_logs` | Automated: test verifies compressed artifact exists |
| 8 | Run manifests generated: agent narrative + daemon evidence appendix; fallback machine-authored if agent fails to narrate | Automated: test verifies manifest exists and has both sections |
| 9 | Error manifest system operational: pipeline failures produce Markdown error manifests with full context | Automated: test injects failure, verifies manifest |
| 10 | Skill linting CI gate validates directory structure, metadata, schemas, trigger boundaries | Automated: `skill_lint.rs` test |
| 11 | Smoke canary harness executes: page fingerprint check + auth verification + minimal extraction per source | Automated: `skill_canary_harness.rs` test |
| 12 | Experiment 4 passes: 10 consecutive batch runs, >=90% success rate, 0 orphaned processes | Semi-automated: harness runs 10 batches, checks orphans |
| 13 | `stage_run_id` correctly scoped per business unit grain for all live-pipeline stage commits | Automated: test verifies stage_run_id uniqueness per scope grain |

### 5.2 Test File Listing

10 new Rust test files for Layer 1 (cumulative: 14 including L0's 4 TS test files):

| Test File | Coverage |
|-----------|----------|
| `agent_pty_session.rs` | PTY session creation, output capture, full lifecycle (created -> spawned -> active -> terminating -> exited -> reaped -> archived), cleanup verdict recording, pgid verification, compressed artifact storage |
| `agent_contract_session.rs` | Contract-mode session, structured JSONL/JSON event capture, result contract validation against `result.schema.json`, checkpoint sequencing, partial failure classification |
| `agent_procurement_ingestion.rs` | Live FTS batch ingestion: daemon triggers FTS skill, notice records created in SQLite, pipeline runs through normalize -> qualify -> extract -> match -> gate -> landed cost -> manifest. Verifies: end-to-end artifact produced, stage_run_id scoped correctly per stage, pipeline_assessments updated, workflow_jobs lifecycle tracked |
| `adapter_codex.rs` | Codex adapter capability probe (detects CLI presence + version), JSONL parsing from contract-mode output, mode selection (pty vs contract based on skill interaction_profile), error classification on probe failure |
| `adapter_claude.rs` | Claude adapter capability probe, JSON/stream-JSON parsing, mode selection, error classification |
| `adapter_result_contract.rs` | Result contract validation against Zod schemas, checkpoint sequencing (monotonic seq, no gaps), partial failure classification (which checkpoints committed, which failed), degraded_completed vs failed outcome distinction |
| `skill_lint.rs` | Skill directory structure validation: SKILL.md exists, result.schema.json valid JSON Schema, checkpoint.schema.json valid, canary.inputs.jsonl parseable, golden.assertions.yaml parseable, metadata fields present (interaction_profile, result_schema_version, auth_requirements, model_profile) |
| `skill_canary_harness.rs` | Live smoke canary execution: page fingerprint check (source hasn't restructured), auth verification (credentials work), minimal extraction (at least one record returned). Per-source canary for FTS. |
| `session_reaper.rs` | Session Reaper correctness: SIGCHLD handling, waitpid drain, orphan detection, cleanup verdict assignment, cleanup_report.json emission, daemon-crash recovery (detect and reap orphaned sessions on restart) |
| `ipc_handshake.rs` | Daemon↔desktop IPC: descriptor file write/read, lock file exclusivity, handshake protocol (version negotiation, schema epoch, commit_seq exchange), stale socket detection, peer credential verification |

### 5.3 Test Scenarios

#### Edge Case Scenarios

| Scenario | Expected Behavior |
|----------|------------------|
| Agent crashes mid-session (SIGKILL) | Cleanup verdict = `forced`, run manifest has machine-authored fallback narrative, error manifest created, pgid group killed, no orphaned processes |
| Orphaned grandchild process | Detected by pgid sweep after session exit, reaped by process supervisor, logged as cleanup warning |
| PTY EOF received unexpectedly | Session transitions to `terminating`, cleanup runs, verdict = `clean` if output was captured, `forced` if mid-extraction |
| Agent CLI not found on PATH | Capability probe fails gracefully, adapter reports `AGENT_INTERNAL_ERROR`, job marked failed with clear error |
| Agent produces partial output then stalls | Stall detection triggers after timeout, session force-killed, partial checkpoints preserved, verdict = `forced` |
| FTS API returns empty results (no new notices) | Normal completion with zero records, no error manifest, cursor not advanced if no data |
| FTS API returns rate limit or error | Source-specific error classified, error manifest created with HTTP context, job retryable |
| Skill result.schema.json validation fails on output | Result contract validator rejects output, `AGENT_OUTPUT_CONTRACT_INVALID` error, error manifest with validation details |
| Multiple candidates in single notice | Each stage runs at correct scope grain: lot extraction per-notice (set-replacement), product extraction per-lot (additive), pricing per-candidate-source-pair (additive) |
| stage_run_id collision | UUID-based, should never collide. Test verifies uniqueness across 100 concurrent stage commits |
| Child exits before PTY master reports hangup | Daemon handles race correctly: waits for both child exit AND PTY drain before advancing to `reaped` |
| PTY master gets EIO before child exit is reaped | EIO treated as expected hangup; session waits for `waitpid` before cleanup |
| Master fd accidentally inherited by grandchild, preventing EOF | CLOEXEC hygiene prevents this; test verifies master fd not leaked to descendants |
| Daemon crashes while PTY session is active | On restart: detect `spawned\|active\|terminating` sessions, kill surviving descendants, synthesize `AGENT_DAEMON_CRASH_DURING_SESSION` error manifest; PTY stream NOT resumed |
| Lost MutationAck after DB commit, retried with same idempotency key | Idempotent: returns same ack, no duplicate writes |
| Auth challenge requiring PTY input | Desktop can send input to attached PTY session via `write_input()` |

### 5.4 Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| PTY session reliability across macOS/Linux | Medium | High | Focus on macOS first (primary development platform); basic PTY only in L1, full hardening (fd hygiene, EIO, terminal geometry) in Layer 3 |
| Agent CLI API changes break adapter | Low | Medium | Adapter abstraction isolates CLI specifics; capability probe detects version incompatibility before session launch; adapter_version tracked in agent_session_logs |
| Process-group cleanup misses edge cases | Medium | Medium | Conservative kill escalation (SIGTERM -> wait -> SIGKILL); pgid sweep after session exit; `leaked` verdict for uncleanable processes; Layer 3 adds full reaper with escalation |
| Live FTS source changes between gold-set validation and first live run | Medium | Medium | Smoke canary harness detects source structure changes before production run; page fingerprint check in canary |
| First live run produces different quality than gold-set evaluation | High | Medium | Expected — gold set was curated, live data is noisy. Error manifests capture failures for debugging. Screening-grade accuracy is acceptable. |

---

## 6. Open Questions, Forward Compatibility & Dependencies

### 6.1 Open Questions

These questions are routed from the research document §10. They are explicitly unresolved.

**§10 User Input #5: Which machine is expected to stay online for scheduled execution and freshness SLAs?**
- Context: Layer 1 introduces the daemon. While L1 uses manual CLI trigger (no cron), the daemon host requirements affect all subsequent layers.
- Recommendation: Identify one designated always-on workstation or mini-server-class host.
- **Layer 1 impact:** Daemon binary must run on this machine. PTY session creation depends on host terminal capabilities.

**§10 Further Research #4: Expiry behavior for in-flight jobs**
- Should expiry (availabilityStatus = expired) cancel in-flight jobs, or let them complete and just prevent auto-qualification?
- Current design: let jobs complete but block downstream progression.
- **Layer 1 impact:** Job Runner needs clear expiry-handling semantics even for manual triggers.

**Resolved, not tracked as open here:** §10 Assumption #2 ("Desktop-local execution is non-negotiable") is already settled by ADR-003 and ADR-004. Layer 1 therefore treats local daemon execution as a fixed architectural premise, not an open question.

### 6.2 Stubs (What L1 Defers)

| Feature | L1 Stub | Full Implementation |
|---------|---------|-------------------|
| Scheduler | CLI manual trigger only | Layer 3: cron scheduler with explicit skill binding |
| Resource arbiter | No concurrency limits | Layer 3: typed quotas (browser slots, agent slots, source lanes) |
| PTY hardening | Session/ctty setup, CLOEXEC, EIO/drain handling, reaper, default 120x40 geometry | Layer 3: advanced fd edge cases, resize stability under load, full terminal geometry management |
| Job claiming/leases | Single-threaded sequential execution | Layer 3: concurrent job claiming with lease management |
| Source registry | Bootstrapped local seed / hardcoded source definitions | Layer 3: typed registry with cron binding; Layer 4: pull from shared_source_configs |
| Procurement coverage | FTS only (first live source) | Layer 2: adds Contracts Finder; Layer 5: PCS if in scope |
| Session reaper | SIGCHLD/waitpid drain loop, pid→session map, orphan scan, cleanup verdicts, cleanup_report.json | Layer 3: configurable escalation timeouts, diagnostic introspection, per-session resource accounting |
| Tracing | Basic stdout/stderr log capture | Layer 3: structured trace derivation + forensic transcript enrichment |
| Validation | Fields exist, always validation_required = false | Layer 3: dry-run; Layer 4: enforcement |
| Manifest rendering | Minimal local markdown artifact | Layer 2: shared packages/manifests renderer + desktop presentation |
| Desktop/operator surface | CLI summary + local file artifact | Layer 2: Tauri desktop control panel |
| Sync | Tracking columns populated, no outbox entries | Layer 4: transactional outbox + push/pull sync engine |

### 6.3 Forward-Compatibility Provisions

| Provision | What Exists in L1 | Target Layer |
|-----------|-------------------|-------------|
| Adapter trait designed for extension | Generic trait with codex + claude implementations; new agent CLIs add new adapter implementations without runtime changes | Future CLIs |
| Error manifest format includes validator fields | Incident signature, resolution context, and skill path fields present even though validator doesn't exist yet | Layer 3 (validation) |
| Agent session logs include run_manifest_path and run_manifest_hash | Populated from first skill run; daemon produces machine-authored fallback if agent fails | Layer 3 (validation reviews manifests) |
| Job Runner aware of parent_job_id | Field exists and is queryable; null in L1 because no validation sub-jobs yet | Layer 3 (validator sub-jobs) |
| stage_run_id on all pipeline writes | Populated from first live write; scope grain tracked in process_stage_status | Layer 4 (sync batch boundaries never cross stage_run_id) |
| Sync tracking columns populated on every write | owner_id, row_version, last_local_seq set even before outbox exists | Layer 4 (sync activation) |

### 6.4 Dependencies

**Layer dependency:** Layer 0 (skills, intelligence pipeline, contracts-schema, SQLite schema)

#### Rust Production Dependencies

| Crate | Purpose |
|-------|---------|
| `tokio` | Async runtime for daemon event loop |
| `rusqlite` (or `sqlx` with SQLite feature) | Local SQLite access |
| `serde` / `serde_json` | Serialization for skill payloads, config, structured traces |
| `tracing` | Structured logging framework |
| `chrono` | Date/time handling (ISO 8601 timestamps) |
| `anyhow` / `thiserror` | Error handling (anyhow for application, thiserror for library errors) |
| `nix` | POSIX process-group and signal handling (setpgid, killpg, waitpid) |
| `portable-pty` (or `pty-process`) | PTY session management (pseudo-terminal creation, resize, read/write) |
| `uuid` | UUID generation for SQLite (replaces gen_random_uuid) |
| `sha2` | SHA-256 hashing for artifact integrity, content hashing, run manifest hashing |
| `zstd` | Compressed transcript and artifact storage |

#### External Accounts / Credentials

| Account | Purpose |
|---------|---------|
| Codex CLI credentials | Agent CLI for skill execution (PTY or contract mode) |
| Claude CLI credentials | Alternative agent CLI |

#### Configuration

| Config | L1 Entries |
|--------|-----------|
| `.env.example` | All L0 entries + `DAEMON_LOG_LEVEL` (tracing verbosity), `ARTIFACT_STORE_PATH` (compressed session artifacts), `DAEMON_PID_FILE` (process management) |
| `Cargo.toml` | Workspace root + daemon crate with all dependencies above |
| `rust-toolchain.toml` | Rust 2024 edition, nightly channel |

### 6.5 ADRs Addressed

| ADR | Decision | Layer 1 Action |
|-----|----------|---------------|
| **ADR-001** | Skill-based agent scraping | First live skill-based procurement ingestion loop (FTS) |
| **ADR-002** | Authority-matrix database | Local-operational-authority half becomes live on real writes; sync engine deferred to Layer 4 |
| **ADR-003** | Tauri + separate Rust daemon | Daemon binary starts here as a standalone process |
| **ADR-004** | PTY-based agent execution from day 1 | Hybrid PTY + contract mode via adapter boundary |
| **ADR-005** | Skills as passive assets, daemon orchestrates | Skills + explicit binding + daemon-managed execution lifecycle |

<!-- CROSS-DOMAIN NOTE FOR L3: The adapter trait is now a two-lane design (OutputFrame + ControlEvent) with LaunchSpec return and AdapterParser sub-trait. L3's validation executor spawns validator sub-jobs via the same adapter — it must use LaunchSpec, not Command, and parse ControlEvents, not the old unified DaemonEvent. L3's PTY hardening is additive on top of L1's session/ctty/CLOEXEC/reaper (L1 now includes setsid, controlling terminal, CLOEXEC hygiene, SIGCHLD/waitpid reaper, EIO drain handling, default 120x40 geometry). L3 adds: configurable escalation timeouts, diagnostic introspection, resize stability under load, advanced fd edge cases. The Session Reaper is L1-critical with cleanup_report.json — L3 extends it, not replaces it. -->

<!-- CROSS-DOMAIN NOTE FOR L0: L1 migration `0003_sqlite_runtime.sql` creates tables in the same `state.db` as L0's `0001_sqlite_state_initial.sql`. Single-database model — no `artifacts.db`. The `commit_clock` table (singleton row, monotonic `commit_seq`) is L1-introduced and does not affect L0 DDL. -->
