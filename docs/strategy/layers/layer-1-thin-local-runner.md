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

**L1-FR-DAEMON.1 — Runtime Reliability Gate**
- AC1: Layer 1 is not complete until Experiment 4 executes 10 consecutive daemon-managed batch runs.
- AC2: The 10-run batch achieves at least 90% skill success rate.
- AC3: The 10-run batch finishes with zero orphaned descendant processes across all sessions.
- AC4: Failures during Experiment 4 produce error manifests and cleanup verdicts instead of being treated as silent harness noise.

**L1-FR-PIPELINE.1 — First Live Local Loop**
- AC1: A manual CLI trigger can execute one live-source batch from ingestion through manifest generation without any desktop, web, or sync dependency.
- AC2: The loop emits a CLI-visible completion summary and at least one local markdown opportunity manifest artifact.
- AC3: The manifest artifact is backed by persisted pipeline data written by the daemon, not by an ad hoc transcript summary.
- AC4: The same Layer 0 intelligence pipeline logic remains the computation authority; Layer 1 only adds runtime supervision, capture, and persistence.

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

**Requirement ID namespace (authoritative for this document):**

| Prefix | Primary files / scope | Coverage domain |
|--------|------------------------|-----------------|
| `DAEMON` | `main.rs` | Bootstrap, CLI triggers, runtime boundaries, signal-handling outcomes |
| `PTY` | `pty/session.rs`, `pty/capture.rs`, `pty/health.rs` | PTY creation, capture, terminal semantics, health monitoring |
| `ADAPTER` | `agents/adapter.rs`, `agents/codex.rs`, `agents/claude.rs` | Capability probe, mode selection, event parsing, session isolation |
| `ARTIFACTS` | `agents/artifacts.rs`, `agents/contracts.rs` | Artifact layout, contract validation, checkpoint and manifest persistence |
| `RUNNER` | `jobs/runner.rs` | Job claim/execute/commit flow, stage-run scoping, durability fence |
| `PROMPT` | `prompt/renderer.rs` | Kickoff prompt composition and manifest instructions |
| `ERRORS` | `errors/manifest.rs`, `errors/markdown.rs` | Error taxonomy, manifest generation, failure recording |
| `IPC` | `ipc/descriptor.rs`, `ipc/handshake.rs` | Descriptor, lock, handshake, peer validation |
| `REAPER` | `sessions/reaper.rs` | Waitpid drain loop, cleanup escalation, verdict assignment |
| `DB` | `db/sqlite.rs` | SQLite bootstrap, migration safety, commit clock, stage metadata |
| `SKILLS` | Skill directories + linting | Skill directory standard, canaries, lint gates, run-manifest inputs |
| `PIPELINE` | End-to-end live loop + stage tracking | Ingestion, stage scoping, pipeline write semantics |

**Traceability rule:** Every testable Layer 1 behavior in this document is identified by an inline `L1-FR-*` marker. Later sections may reference an existing requirement ID instead of restating the full contract.

**L1-FR-DAEMON.2 — Runtime Surface Matches the Layer 1 Component Inventory**
- AC1: The Layer 1 runtime surface is limited to the 17 daemon files, the two new skills, and the `0003_sqlite_runtime.sql` migration listed in this section.
- AC2: Behavior assigned to a subsystem prefix in this document maps to the files listed in the namespace table above.
- AC3: New functionality deferred to later layers is not treated as implicitly active in Layer 1 unless a requirement in this document says otherwise.

### 1.6 Ingestion Flow (End-to-End Live Loop)

The first live operational loop executes this sequence via manual CLI trigger:

**L1-FR-PIPELINE.2 — Manual Trigger Starts the Live Loop**
- AC1: An operator can start the FTS live loop with an explicit CLI command.
- AC2: The CLI trigger binds the run to the Layer 1 daemon, not to an unmanaged `bun run` invocation.
- AC3: Trigger failure before job creation exits non-zero and does not create a partially claimed job.

**L1-FR-PIPELINE.3 — Trigger Creates a Source-Sync Job**
- AC1: The daemon creates a `workflow_jobs` row with `job_kind = 'sync_source'`.
- AC2: The job stores the explicit skill binding for `skills/procurement-ingestion/fts`, including the resolved skill version used for the run.
- AC3: Job creation is durable before the daemon proceeds to session execution.

**L1-FR-PIPELINE.4 — Adapter Selection Honors the Skill Interaction Profile**
- AC1: The FTS ingestion skill resolves to PTY execution because its `interaction_profile` is `pty_interactive`.
- AC2: Mode selection uses the adapter probe result plus the skill declaration, not ad hoc operator flags.
- AC3: If PTY requirements cannot be met, the job fails explicitly instead of silently degrading to contract mode.

**L1-FR-PIPELINE.5 — Session Spawn Creates an Identified Runtime Session**
- AC1: The daemon transitions the session from `created` to `spawned` only after process creation succeeds.
- AC2: The session record stores `pid`, `pgid`, adapter identity, CLI version, and config hash.
- AC3: Spawn failure records an error outcome and does not leave an active process group behind.

**L1-FR-PIPELINE.6 — Skill Execution Proves Page Identity Before Extraction**
- AC1: The ingestion skill follows the bound `procurement-ingestion/fts/SKILL.md` playbook.
- AC2: The run proves the expected page fingerprint before extraction begins.
- AC3: The session transitions from `spawned` to `active` only after execution is underway.
- AC4: Fingerprint mismatch is treated as a classified failure, not as a warning-only condition.

**L1-FR-PIPELINE.7 — Incremental Retrieval Uses Durable Cursor State**
- AC1: The daemon reads the last successful FTS cursor from `source_cursors`.
- AC2: Retrieval requests use the supported FTS cursor parameters (`updatedFrom`, `updatedTo`, `cursor`, `limit`) or their current equivalent contract.
- AC3: Durable commit boundaries align to notice batches rather than arbitrary PTY output fragments.
- AC4: Resume uses a small cursor backtrack window so retries do not skip recently changed notices.
- AC5: If no new records are returned, the run completes normally and does not advance the cursor past unseen data.

**L1-FR-PIPELINE.8 — Raw Session Capture Is Immutable and Session-Scoped**
- AC1: Raw PTY bytes are captured for the full interactive session and stored under the session-specific artifact path.
- AC2: Capture metadata is persisted in `agent_session_logs` for the same `session_id`.
- AC3: Capture write failures surface as job failures and do not permit a false-success pipeline completion.

**L1-FR-PIPELINE.9 — Workflow State Advances From Contracts, Not Transcript Parsing**
- AC1: The agent emits checkpoints and final outputs through the declared contract files.
- AC2: The daemon validates checkpoints before using them for workflow advancement.
- AC3: The daemon never derives authoritative pipeline state from transcript text scraping.

**L1-FR-PIPELINE.10 — Every Session Produces a Run Manifest**
- AC1: Successful sessions include an agent-authored narrative with Summary, Steps Taken, Decisions Made, Warnings, and Evidence sections.
- AC2: The daemon appends a machine-authored evidence appendix containing observed URL/fingerprint and artifact references.
- AC3: If the agent fails to provide narrative content, the daemon still produces a fallback run manifest with `agent_narrative_missing: true`.
- AC4: The manifest is stored at `artifacts/sessions/YYYY/MM/<session_id>/run-manifest.md`.

**L1-FR-PIPELINE.11 — Final Outputs Pass Contract and OQG Validation Before Use**
- AC1: Final outputs are validated against the skill's `result.schema.json`.
- AC2: Schema-valid outputs are additionally checked by the Output Quality Gate for semantic and statistical anomalies.
- AC3: Outputs failing schema validation fail the job.
- AC4: Outputs passing schema validation but failing OQG are quarantined as `AGENT_SUSPECT_OUTPUT` and do not advance authoritative pipeline state.

**L1-FR-PIPELINE.12 — Layer 1 Executes the Live Processing Sequence Defined for This Layer**
- AC1: The live loop performs normalization before downstream qualification and manifest stages.
- AC2: Layer 1 may record the canonical stage model for `relate`, but full relation-resolution computation remains stubbed until Layer 3.
- AC3: The active Layer 1 live loop executes `prequalify_notice -> acquire_docs -> extract_lots -> qualify_lot -> extract_products -> quoteability -> price match -> gross gate -> landed cost -> net gate -> coverage rollup -> manifest`.
- AC4: Stage processing logic reused from Layer 0 stays authoritative for per-stage computation rules.

**L1-FR-PIPELINE.13 — Stage Writes Commit Atomically With Stage Tracking**
- AC1: The SQLite transaction that writes stage outputs also writes the shared `stage_run_id`.
- AC2: The same transaction updates `process_stage_status`.
- AC3: The same transaction advances `process_stage_heads` only for successful stage commits.
- AC4: Failed stages may record failure status but must not advance the head.

**L1-FR-PIPELINE.14 — Session Cleanup Completes Before the Run Is Considered Done**
- AC1: The session transitions through `active -> terminating -> exited -> reaped -> archived`.
- AC2: The Session Reaper performs descendant cleanup using group-based escalation and orphan sweep logic.
- AC3: The run records one cleanup verdict from `clean`, `forced`, or `leaked`.
- AC4: The daemon does not report the run as complete before cleanup is verified.

**L1-FR-PIPELINE.15 — The Live Loop Produces Verifiable Local Output**
- AC1: The daemon prints a CLI-visible completion summary for the run.
- AC2: The daemon writes a local markdown opportunity manifest artifact proving the loop completed.
- AC3: A `manifests` table row references the emitted opportunity manifest for the completed process.
- AC4: Failure before artifact emission leaves a failure record, not a missing-output ambiguity.

**L1-FR-PIPELINE.22 — Each Ingestion Step Has an Explicit Durable-Write Surface**
- AC1: Every step in the 14-step live loop either identifies its durable SQLite writes or explicitly states that no new durable SQLite write occurs in that step.
- AC2: Artifact-store writes are distinguished from SQLite writes.
- AC3: Failure-path writes route through explicit tables/artifacts (`workflow_jobs`, `agent_session_logs`, `error_manifests`, session artifacts) rather than operator inference.

**Step-to-write map (authoritative for the 14-step loop):**

| Step | Durable SQLite writes | Artifact / non-SQLite writes |
|------|-----------------------|------------------------------|
| 1. Trigger | None before job creation; CLI invocation only | None |
| 2. Job creation | `workflow_jobs` insert for `job_kind = 'sync_source'` with explicit skill binding | None |
| 3. Adapter selection | No new durable SQLite write required; selected mode/adapter is captured with the session record in step 4 | None |
| 4. Session spawn | `agent_session_logs` create/update with session identifiers, execution mode, adapter identity, CLI/config identity, and `spawned` status | PTY/session runtime established |
| 5. Skill execution | `agent_session_logs` status transition to `active` (plus timing/progress metadata as available) | Browser/CLI activity only |
| 6. Incremental retrieval | Reads `source_cursors`; writes `source_cursors` only at committed safe boundaries for successful batches or continuation-safe advancement | None |
| 7. Capture | `agent_session_logs.terminal_output_*` metadata updates | Session stream chunks under `artifacts/sessions/.../stream/` |
| 8. Checkpoints and result contract | `agent_session_logs.last_checkpoint_seq` update when checkpoints validate | `checkpoint-*.json` and `result.json` artifacts |
| 9. Run manifest generation | `agent_session_logs.run_manifest_path` and `agent_session_logs.run_manifest_hash` | `run-manifest.md` artifact |
| 10. Result validation | Success path: no additional mandatory domain-table write beyond session metadata; failure/quarantine path writes `error_manifests` and session error fields | Validation/quarantine artifacts as applicable |
| 11. Pipeline processing | Stage-domain writes as applicable: `procurement_processes`, `contract_notices`, `raw_documents` (when documents are acquired), `tender_document_sets`, `notice_lots`, `qualification_runs`, `product_candidates`, `search_profiles`, `search_attempts`, `supplier_quotes`, `candidate_pricing_assessments`, `landed_cost_estimates`, `pipeline_assessments` | Supporting downloaded documents and intermediate artifacts |
| 12. Stage tracking | In the same transaction as step 11 domain writes: `process_stage_status`, `process_stage_heads`, `commit_clock` | None |
| 13. Session cleanup | `agent_session_logs` status/cleanup-verdict finalization; `error_manifests` on cleanup failure paths | `cleanup-report.json` artifact |
| 14. Verifiable output | `manifests` row, `workflow_jobs` final status, final `agent_session_logs` archival status/metadata | CLI summary plus local markdown opportunity manifest artifact |

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

**Adapter selection toggle:** Layer 1 supports an explicit agent-CLI selection toggle with two values: `claude` or `codex`. The daemon keeps a default adapter selection in operator-controlled runtime settings (desktop app and equivalent CLI flags), and each skill may optionally override that default with `agent_cli` in `SKILL.md`. Resolution order is: CLI override -> skill-level `agent_cli` -> daemon default setting -> fallback `claude`.

**L1-FR-ADAPTER.1 — Interaction Profile Selects the Execution Family**
- AC1: Skills declaring `pty_interactive` require PTY mode.
- AC2: Skills declaring `headless_contract` require contract mode unless an explicitly documented fallback policy says otherwise.
- AC3: Mode selection is deterministic for a given skill manifest and capability report.

**L1-FR-ADAPTER.2 — PTY Mode Preserves Interactive Terminal Semantics**
- AC1: PTY mode is used for browser automation, interactive auth flows, and CLIs that require a controlling terminal.
- AC2: PTY mode captures the full terminal stream as immutable forensic output.
- AC3: PTY mode health monitoring remains active for the full session lifetime.

**L1-FR-ADAPTER.3 — Contract Mode Preserves Structured Non-Interactive Execution**
- AC1: Contract mode is preferred for deterministic skills that can emit structured results without browser interaction.
- AC2: Contract mode exposes stdout/stderr as separate output-lane frames rather than pretending they are PTY data.
- AC3: Contract mode still emits daemon-native control events for checkpoints, progress, result readiness, and exit.

**L1-FR-ADAPTER.4 — Capability Probe Gates Production Traffic**
- AC1: Every adapter probe reports CLI version, auth health, structured-output viability, PTY viability, and browser/tool viability.
- AC2: Probe failure prevents job execution in the affected mode.
- AC3: Unknown CLI versions enter canary mode before production traffic is accepted.

**L1-FR-ADAPTER.5 — Mode Resolution Uses the Declared Fallback Matrix**
- AC1: `PtyRequired` selects PTY only when PTY viability is true.
- AC2: `ContractRequired` selects contract mode only when structured output is viable.
- AC3: `ContractPreferred { allow_pty_fallback: true }` falls back to PTY only when structured output is unavailable and PTY is viable.
- AC4: `ContractPreferred { allow_pty_fallback: false }` fails explicitly when contract mode is unavailable.
- AC5: No skill silently degrades from `ContractRequired` to PTY.

**L1-FR-ADAPTER.6 — The Output Lane Is Lossless and Transport-Specific**
- AC1: PTY sessions emit `Pty(Bytes)` frames only.
- AC2: Contract-mode sessions emit `Stdout(Bytes)` and `Stderr(Bytes)` frames without lossy coalescing.
- AC3: Output-lane frames are suitable for both forensic capture and future UI streaming.

**L1-FR-ADAPTER.7 — The Control Lane Is the Canonical Semantic Stream**
- AC1: All adapters emit the canonical `Started`, `Checkpoint`, `Progress`, `AuthChallenge`, `Warning`, `FinalResultReady`, and `Exited` events.
- AC2: The Job Runner and lifecycle manager consume the control lane instead of vendor-native event formats.
- AC3: `FinalResultReady` identifies a result path only; it does not certify the payload as valid.

**L1-FR-ADAPTER.8 — Result Validation Happens Outside the Adapter Boundary**
- AC1: Adapters do not validate `result.schema.json` contracts.
- AC2: Schema validation is owned by `agents/contracts.rs`.
- AC3: OQG semantic checks are owned by daemon pipeline code after the final result artifact is available.

**L1-FR-ADAPTER.9 — Unknown Versions Run in Canary Mode**
- AC1: A CLI version outside the known-good set is flagged as canary.
- AC2: Canary mode requires a smoke probe before the adapter can accept production jobs.
- AC3: Canary status is recorded with the session for later debugging.

**L1-FR-ADAPTER.10 — Session Isolation Prevents Cross-Run Contamination**
- AC1: Each session uses isolated config paths for the agent CLI.
- AC2: Each session records CLI version, model, config hash, skill version, and adapter version.
- AC3: Browser-backed flows use per-session browser profile directories.
- AC4: Working directory and environment are controlled per run rather than inherited opportunistically from the daemon process.

**L1-FR-ADAPTER.10A — Adapter Selection Is Explicit and Overridable**
- AC1: Layer 1 maintains a daemon-default agent CLI selection that can be set by operator-facing runtime configuration (desktop app and equivalent CLI flags).
- AC2: Skills may override the daemon default with optional `agent_cli: claude|codex` metadata in `SKILL.md`.
- AC3: A CLI flag override takes precedence over both the daemon default and the skill-level override for the current run only.
- AC4: If no override is present, the daemon falls back to `claude`.

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
4. For PTY sessions whose CLI honors terminal interrupts: write `^C` / equivalent to the PTY after any adapter in-band interrupt and before POSIX signal escalation
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

**L1-FR-PTY.1 — PTY Spawn Establishes Full Session and Controlling-Terminal Semantics**
- AC1: The daemon creates a PTY master/slave pair before spawning the child.
- AC2: The child calls `setsid()` before `exec`.
- AC3: The child establishes the slave as the controlling terminal via `login_tty()` or `TIOCSCTTY`.
- AC4: The child redirects stdin, stdout, and stderr to the PTY slave before `exec`.
- AC5: The daemon records `sid`, `pid`, and `pgid` for the session before it is considered spawned.
- AC6: PTY creation failure, `setsid()` failure, or controlling-terminal setup failure aborts spawn, leaves no active child/session group behind, and produces a classified failure record with manifest context.

**L1-FR-PTY.2 — PTY File-Descriptor Hygiene Prevents Hidden Hangs**
- AC1: The PTY master fd is marked `O_CLOEXEC` before child exec.
- AC2: Any daemon control fd not intended for the child is also marked close-on-exec.
- AC3: The implementation prevents grandchild inheritance of the PTY master from suppressing EOF delivery on the daemon side.

**L1-FR-PTY.3 — Fork Safety Is Preserved Under Tokio**
- AC1: Child-side work after `fork()` is limited to async-signal-safe operations until `execve()`.
- AC2: If the PTY library cannot express the required pre-exec behavior safely, Layer 1 uses a small spawn helper binary instead of unsafe in-daemon child logic.
- AC3: The final implementation documents which spawn path it uses and why.

**L1-FR-PTY.4 — Terminal Geometry Is Stable and Explicit**
- AC1: PTY sessions start with default geometry `120x40`.
- AC2: Browser viewport configuration is treated as separate from PTY geometry.
- AC3: `resize_pty(session_id, cols, rows)` is available for attached sessions.
- AC4: If no resize occurs, transcript geometry remains constant for reproducibility.

**L1-FR-REAPER.1 — Session Completeness Has a Strict Multi-Part Gate**
- AC1: Child exit must be observed.
- AC2: PTY mode must drain to EOF or reach a drain timeout.
- AC3: Descendant cleanup must be verified, not assumed.
- AC4: The final result contract must be validated if present.
- AC5: A cleanup verdict must be recorded before the session is complete.

**L1-FR-REAPER.2 — Cleanup Uses Ordered Escalation and Explicit Reap**
- AC1: Soft stop attempts an adapter-supported in-band interrupt first when available.
- AC2: PTY sessions may receive `^C` or equivalent before POSIX signal escalation.
- AC3: Cleanup escalates through `SIGINT`, `SIGTERM`, and `SIGKILL` against the process group.
- AC4: PTY draining continues until EOF or timeout, with PTY `EIO` treated as expected hangup.
- AC5: Completion is determined by `waitpid()`/reap logic, not by `kill(pid, 0)` or dropped child handles.

**L1-FR-REAPER.3 — Session Reaper Is Active in Layer 1**
- AC1: Layer 1 includes `SIGCHLD` handling and a `waitpid(-1, WNOHANG)` drain loop.
- AC2: The reaper maintains a `pid -> session_id` association for child exits.
- AC3: The reaper performs an orphan/leak scan after session exit.
- AC4: The reaper emits `cleanup_report.json` for every session.

**L1-FR-REAPER.4 — Cleanup Verdicts Are Explicit and Testable**
- AC1: `clean` means all descendants exited before timeout without forced kill.
- AC2: `forced` means cleanup required `SIGTERM` or `SIGKILL`.
- AC3: `leaked` means descendant processes survived the cleanup ladder.
- AC4: The verdict is persisted with the session and surfaced to downstream consumers.

**L1-FR-REAPER.5 — Platform-Specific Escape Cases Are Accounted For**
- AC1: Linux implementations may use `PR_SET_CHILD_SUBREAPER` or later cgroup scopes as additive containment, not as a replacement for group signaling.
- AC2: macOS implementations rely on process inspection and conservative cleanup because no Linux-style subreaper exists.
- AC3: Browser helper escapees are handled by `kill(-pgid, ...)` first, descendant sweep second, and `leaked` verdict if survivors remain.

### 1.9 Error Classification

Every pipeline failure produces a structured Markdown error manifest with full context.

This section is the **condensed runtime overview** for the Layer 1 session-execution error codes most central to daemon bootstrap, supervision, and cleanup. The **authoritative full active-code list** for Layer 0-1, including pipeline/source codes and all Layer 1 source/skill/bootstrap/storage codes, lives in §4.3.

**Primary runtime error types surfaced during Layer 1 session execution:**

| Requirement ID | Error Type | Trigger | Action |
|----------------|------------|---------|--------|
| `L1-FR-ERRORS.21` | `AGENT_STALLED` | No PTY/stdout bytes for timeout period, or no validated checkpoint progress despite output | Kill process group; create error manifest |
| `L1-FR-ERRORS.22` | `AGENT_AUTH_CHALLENGE` | Re-login prompt, MFA challenge, browser SSO failure, hidden permission prompt | Flag for operator; create error manifest |
| `L1-FR-ERRORS.23` | `AGENT_SKILL_FAILED` | Source website changed; skill playbook failed at specific step | Create error manifest with skill path and step |
| `L1-FR-ERRORS.24` | `AGENT_OUTPUT_CONTRACT_INVALID` | Agent output fails schema validation against `result.schema.json` | Preserve transcript + artifact; fail job |
| `L1-FR-ERRORS.25` | `AGENT_SUSPECT_OUTPUT` | Schema-valid output fails OQG semantic/drift checks | Quarantine results; keep pipeline state unchanged |
| `L1-FR-ERRORS.26` | `AGENT_PARTIAL_FAILURE` | Valid checkpoint exists before crash | Persist checkpoint outputs; never upgrade to success without final contract + clean exit + reap |
| `L1-FR-ERRORS.27` | `AGENT_INTERNAL_ERROR` | Error from structured events or terminal output | Create error manifest |
| `L1-FR-ERRORS.28` | `AGENT_RUN_FAILED` | Non-zero exit code with no specific error pattern | Create error manifest |
| `L1-FR-ERRORS.29` | `AGENT_CLEANUP_INCOMPLETE` | Descendant processes leaked after session exit | Record leaked pids; flag for operator |
| `L1-FR-ERRORS.30` | `AGENT_DAEMON_CRASH_DURING_SESSION` | Daemon process crashed while PTY session was active | On restart: detect sessions that were `spawned\|active\|terminating`, kill surviving descendants, synthesize error manifest with last known state; do NOT pretend PTY stream can be resumed |
| `L1-FR-ERRORS.31` | `AGENT_PROTOCOL_MISMATCH` | Unexpected CLI output format or parser framing incompatible with the adapter contract | Enter canary/diagnostic handling; create error manifest |
| `L1-FR-ERRORS.37` | `HOST_SLEEP_INTERRUPTED` | Monotonic time gap exceeds threshold during active session | Taint browser-backed sessions; create error manifest |
| `L1-FR-ERRORS.38` | `HOST_RESOURCE_EXHAUSTED` | Host memory/CPU pressure detected | May preempt lower-priority sessions |

**Additional active Layer 1 source/skill/bootstrap/storage errors summarized from §4.3:**

| Requirement ID | Error Type | Trigger | Action |
|----------------|------------|---------|--------|
| `L1-FR-ERRORS.32` | `SOURCE_SOFT_BLOCKED` | Source returns an apparently successful page/response with no usable data | Capture artifacts; set source cooldown |
| `L1-FR-ERRORS.33` | `SOURCE_CAPTCHA_BLOCKED` | Human challenge prevents autonomous progress | Emit `blocked_human_action`; alert operator |
| `L1-FR-ERRORS.34` | `AUTH_STATE_STALE` | Session loads but required privileged state is missing | Trigger auth refresh / retry path |
| `L1-FR-ERRORS.35` | `PAGE_SHAPE_DRIFT` | Page fingerprint mismatches expected source shape | Flag skill review; create error manifest |
| `L1-FR-ERRORS.36` | `LOCALE_OR_CURRENCY_DRIFT` | Wrong locale, currency, or market variant detected | Fail fast; require config review |
| `L1-FR-ERRORS.40` | `SKILL_DIRECTORY_INVALID` | Required skill files/metadata/schemas are missing or malformed | Fail binding/lint before spawn; surface lint context |
| `L1-FR-ERRORS.41` | `SQLITE_WRITE_FAILED` | SQLite write/commit fails (disk full, I/O error, corruption, busy-timeout exhaustion) | Fail mutation/job, advance no heads/fence, surface storage error |
| `L1-FR-ERRORS.42` | `DAEMON_STARTUP_FAILED` | Lock, migration, integrity, descriptor, or socket readiness step fails | Do not advertise readiness; emit explicit startup failure |

Detailed acceptance criteria for every error requirement ID are defined in §4.3. This section is the condensed runtime overview.

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
| `async-trait` | Async trait support | Required by the `AgentAdapter` / `AdapterParser` traits in §2.3 |
| `bytes` | Byte-buffer type for output lane | Required for `OutputFrame::{Pty,Stdout,Stderr}` in §2.3 |
| `serde_yaml` | YAML frontmatter rendering/parsing | Required for run-manifest frontmatter in §2.5 / §4.2 |
| `fs4` | File locking helper | Convenience crate for `daemon.lock`; direct platform locking is also acceptable |

**NOT needed until later layers:**

| Crate | Layer | Purpose |
|-------|-------|---------|
| `reqwest` | L0 (intelligence) / L5 (freight/FX APIs) | HTTP client for cost-estimation APIs |
| `sqlx` (Postgres feature) | L4 | Supabase/Postgres sync |
| `cron` | L3 | Cron expression parsing for scheduler |
| `sysinfo` | L3 | Host resource pressure detection for resource arbiter |

**L1-FR-DAEMON.3 — The Dependency Set Covers Every Described Layer 1 Capability**
- AC1: The production dependency set includes crates for async runtime, SQLite, PTY/process supervision, serialization, hashing, compression, CLI parsing, and structured logging.
- AC2: Dependencies required by the Rust API examples and contracts in this document (`async-trait`, `bytes`, YAML frontmatter support) are declared explicitly instead of being left implicit.
- AC3: Later-layer-only dependencies remain excluded from the Layer 1 runtime unless they are needed for an active Layer 1 behavior.

### 2.3 Agent Adapter Trait (Two-Lane Design)

The adapter trait is the abstraction boundary between the daemon and vendor-specific agent CLIs. The key design principle: **unify the control plane, not the transport.**

Two independent event lanes:

- **Output lane**: lossless raw transport for UI streaming and forensic capture. `Pty(Bytes) | Stdout(Bytes) | Stderr(Bytes)`
- **Control lane**: semantic daemon events for job runner and lifecycle. `Started | Checkpoint | Progress | AuthChallenge | Warning | FinalResultReady | Exited`

This separation means PTY stays raw and contract mode stays structured without pretending they are the same thing. The output lane feeds `pty/capture.rs` and desktop `Channel<T>` streaming; the control lane feeds `jobs/runner.rs` and session lifecycle.

**Lifecycle / ownership model (normative):**

- `AgentAdapter` instances are process-wide immutable capability objects in Layer 1. The daemon may share one adapter instance across sessions; there is no separate adapter initialization or teardown hook in this layer.
- `AdapterParser` instances are session-scoped and own the mutable parse state for one run. `parser()` must therefore be cheap and side-effect-free: no network I/O, no filesystem discovery, and no mutation of shared global state.
- In PTY mode, adapter-semantic `ControlEvent`s derived from raw PTY bytes come only from the `AdapterParser`; the runner/reaper must not synthesize adapter-level meaning by scraping transcript text out of band.
- In contract mode, structured stdout remains lossless on the raw output lane **and** may also yield `ControlEvent`s through the parser. Stderr always remains lossless on the output lane even when it yields no semantic event.
- Parsers consume frames in capture order. Any `ControlEvent`s derived from frame `N` are emitted only after frame `N` has been sequenced for capture. No stronger cross-lane total-order guarantee is promised.

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

**L1-FR-ADAPTER.11 — `probe()` Produces a Capability Report the Daemon Can Enforce**
- AC1: `probe()` reports CLI version, auth health, structured-output viability, PTY viability, browser-tool availability, and canary status.
- AC2: A failed probe prevents execution in the unsupported mode and surfaces an explicit pre-spawn failure instead of crashing the daemon.
- AC3: Probe results are durable enough to be recorded in session/debug artifacts for later failure analysis.
- AC4: If the daemon detects that the installed CLI version changed after a prior probe, it refreshes the capability report and re-evaluates canary status before the next launch.

**L1-FR-ADAPTER.12 — `resolve_mode()` Is a Pure Policy Decision**
- AC1: `resolve_mode()` consumes only the skill manifest and capability report.
- AC2: The method returns either an explicit `ExecutionMode` or an error; it never mutates process state.
- AC3: The method enforces the fallback matrix defined in this document with no silent downgrade.

**L1-FR-ADAPTER.13 — `build_launch_spec()` Returns Spawn Inputs, Not a Live Process**
- AC1: `build_launch_spec()` returns program path, arguments, environment, and working directory only.
- AC2: PTY/session/process-group setup remains centralized in daemon-owned spawn code.
- AC3: The launch spec includes session-isolated config paths and other adapter-required environment values.

**L1-FR-ADAPTER.14 — `parser()` Converts Vendor Output Into Canonical Control Events**
- AC1: The parser consumes output-lane frames and may emit zero or more `ControlEvent`s for each frame.
- AC2: The parser has a `finish()` phase so adapters can emit terminal semantic events after process exit.
- AC3: Vendor-specific syntax stays inside the parser implementation rather than leaking into job-runner logic.
- AC4: In PTY mode, adapter-semantic control events derived from transcript bytes are emitted only through the parser boundary.
- AC5: In contract mode, stdout bytes remain present on the output lane even when the parser also derives semantic events from them; stderr bytes remain present on the output lane regardless of whether they yield semantic events.
- AC6: The daemon invokes parsers in raw-frame capture order and emits derived control events only after the source frame has been sequenced for capture.

**L1-FR-ADAPTER.15 — `CapabilityReport` and `InteractionProfile` Fields Are Normative Inputs**
- AC1: Every `CapabilityReport` field shown in this section is available to mode resolution and diagnostics.
- AC2: `InteractionProfile` values are treated as a closed set for Layer 1: `PtyRequired`, `ContractRequired`, and `ContractPreferred`.
- AC3: A skill that cannot be executed under its declared interaction policy fails before launch.

**L1-FR-ADAPTER.16 — Adapter Identity Is Stable Across Sessions**
- AC1: `id()` returns a stable adapter identifier string suitable for persistence in `agent_session_logs`.
- AC2: The adapter identifier is not generated per session or per process launch.
- AC3: The same adapter implementation reports the same identifier across runs unless the implementation itself changes.

**L1-FR-ADAPTER.17 — Adapter Implementations Are Safe to Share Across Sessions**
- AC1: Because `AgentAdapter: Send + Sync`, the implementation is safe for concurrent use by the daemon across multiple jobs.
- AC2: Per-session mutable state lives in `AdapterParser` instances or spawn-time inputs, not in shared adapter-global mutable state.
- AC3: Adapter behavior does not depend on ambient mutable process state outside the declared launch spec, env vars, cwd, and session-isolated config paths.
- AC4: `parser()` creation is lightweight and side-effect-free enough to occur per session without hidden global initialization or teardown work.

**L1-FR-ADAPTER.18 — Cancellation and Timeouts Are Daemon-Owned, Adapter-Aware**
- AC1: Wall-clock timeouts and lifecycle deadlines are enforced by the daemon session lifecycle, not buried inside adapter-specific retry loops.
- AC2: If an adapter supports a structured in-band interrupt, that behavior is deterministic enough for the daemon to invoke during `terminating`.
- AC3: If no structured cancel path exists, the daemon proceeds directly to the documented PTY interrupt step and then, if needed, process-group signal escalation without waiting for adapter-specific cleanup semantics.
- AC4: `AdapterParser.finish()` runs exactly once on normal exit or forced termination so abrupt stream endings still produce a final semantic classification or explicit parse failure.

### 2.4 Session Lifecycle (7 Phases)

Layer 1 exposes **seven canonical public phases**. A session that successfully acquires a child process passes through all seven in order. A run that fails before any child exists reuses the same vocabulary but may omit the inapplicable runtime phases (`spawned`, `active`) and must still normalize to a terminal archived record.

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
| **archived** | Session complete. All artifacts stored and indexed. Session metadata finalized in `agent_session_logs`. Job status updated in `workflow_jobs`. Stage status/heads updated if pipeline job. | Daemon crash after `reaped` but before archive finalization → startup reconciliation resumes archival idempotently |

**Fixed Layer 1 timing budgets (non-configurable until Layer 3):**

- `terminating_grace_timeout = 10s` from cleanup initiation to authoritative exit before escalation continues
- `drain_timeout = 5s` for PTY/stdout drain after exit or hangup
- `reap_completion_budget = 25s` from authoritative exit to finished descendant sweep and cleanup verdict
- `archive_finalization_budget = 10s` from `reaped` to durable `archived`; if the daemon crashes in this window, startup reconciliation resumes archival idempotently

**Session completeness rule:** A session is only complete after ALL of:
1. Child exit observed (via `waitpid`, not `kill(pid, 0)`)
2. PTY drained to EOF or drain timeout (PTY mode only); `EIO` is expected hangup
3. Descendant process tree verified clean (not assumed) — reaper confirms
4. Final result contract validated in `agents/contracts.rs` (if present)
5. Cleanup verdict recorded (`clean` | `forced` | `leaked`)
6. `cleanup_report.json` emitted to artifact store
7. Run manifest stored (agent narrative + daemon evidence appendix; machine-authored fallback if agent fails to narrate)

**L1-FR-DAEMON.4 — `created` Means the Run Is Fully Planned but Not Yet Spawned**
- AC1: The daemon creates the `agent_session_logs` row before attempting process spawn.
- AC2: Adapter selection, kickoff prompt composition, skill loading, working-directory preparation, and session-isolated config setup complete in this phase.
- AC3: Failure in `created` fails the job before any child process exists.

**L1-FR-DAEMON.5 — `spawned` Means Runtime Session Semantics Are Established**
- AC1: The child process is launched in its own session/process group before the phase becomes `spawned`.
- AC2: `sid`, `pid`, `pgid`, and PTY identity (when applicable) are recorded durably.
- AC3: PTY capture starts immediately for PTY sessions.

**L1-FR-DAEMON.6 — `active` Means Execution and Health Monitoring Are Live**
- AC1: Raw output capture is active for the selected execution mode.
- AC2: No-output timeout, no-progress timeout, auth-challenge detection, and host-sleep detection are active in this phase.
- AC3: Validated checkpoints may advance session progress while the phase remains `active`.

**L1-FR-DAEMON.7 — `terminating` Means Graceful Shutdown Has Started**
- AC1: The daemon enters `terminating` when it has initiated cleanup rather than waiting passively for exit.
- AC2: Adapter-aware in-band interrupt is attempted first when supported.
- AC3: Timeout during `terminating` triggers the configured signal-escalation ladder.
- AC4: Layer 1 uses a fixed `terminating_grace_timeout = 10s` before the ladder advances beyond the graceful-stop attempt.

**L1-FR-DAEMON.8 — `exited` Means Process Exit Is Known but Cleanup May Still Be Outstanding**
- AC1: Child exit is observed through `waitpid` or equivalent authoritative exit reporting.
- AC2: PTY sessions continue draining output to EOF or timeout after exit.
- AC3: PTY `EIO` during drain is treated as expected hangup rather than as a session failure by itself.
- AC4: A session may remain in `exited` only long enough to satisfy the fixed Layer 1 drain and reap budgets; it must not remain indefinitely between `exited` and `reaped`.

**L1-FR-DAEMON.9 — `reaped` Means Descendants, Contracts, and Cleanup Verdict Are Finalized**
- AC1: The Session Reaper has completed descendant cleanup and assigned a cleanup verdict.
- AC2: `cleanup_report.json` has been emitted.
- AC3: Final result contract validation runs in this phase if a result artifact exists.
- AC4: Run-manifest finalization completes before the session leaves `reaped`.
- AC5: If the daemon crashes after `reaped` but before `archived`, startup reconciliation resumes archive finalization idempotently rather than re-running the session.

**L1-FR-DAEMON.10 — `archived` Means the Session Is Terminal and Queryable**
- AC1: All artifacts and metadata for the session are stored and indexed before the session enters `archived`.
- AC2: `workflow_jobs` reflects the final session/job outcome by the time the session is archived.
- AC3: No later phase transition is permitted after `archived`.

**L1-FR-DAEMON.11 — Lifecycle Uses Only the Seven Canonical Public Phase Names**
- AC1: Layer 1 exposes only `created`, `spawned`, `active`, `terminating`, `exited`, `reaped`, and `archived` as public session-phase values.
- AC2: Internal drain substates may exist, but they do not replace or invent public phase names.
- AC3: Pre-spawn failures may omit `spawned` and `active` because no runtime process ever existed; once a session reaches `spawned`, it continues through the remaining canonical phases in order.

**L1-FR-DAEMON.12 — Transition Normalization Makes Failure Paths Mechanically Testable**
- AC1: `created -> spawned` occurs only after child/session/process-group creation and identifier recording succeed.
- AC2: If no child process is ever created, the daemon records a pre-spawn failure and still normalizes the session through `terminating -> exited -> reaped -> archived` with null runtime identifiers instead of leaving the row stranded in `created`.
- AC3: `spawned -> active` occurs as soon as runtime session establishment is confirmed; if the child exits before useful work is produced, the daemon still records a zero-progress `active` phase before continuing to termination/exit handling so the public lifecycle does not skip from `spawned` to `exited`.
- AC4: `active -> terminating` occurs on natural completion, operator stop, or daemon-detected fatal condition; `terminating` may be zero-duration for naturally exiting processes, but it is still emitted.
- AC5: `exited -> reaped` requires drain/cleanup completion gates, and `reaped -> archived` requires artifact/session/job finalization gates.

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

**Crash-recovery rules:**
- Only chunks present under their final filenames are considered durable; temp files are ignored or garbage-collected during recovery.
- If the daemon crashes mid-capture, already-renamed chunks remain authoritative and the trailing open temp chunk may be discarded without exposing a partial final-name file.
- Startup/archive recovery truncates any torn trailing JSON line in `stream.index.jsonl` and regenerates missing entries from durable chunk files before the session is marked archived.
- Readers must observe only the longest contiguous committed sequence prefix after recovery.
- Layer 1 imposes no fixed per-session chunk cap; long-running sessions continue rotating chunk ordinals while keeping at most one open chunk per transport stream.

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

**L1-FR-ARTIFACTS.1 — Session Output Lives in the Filesystem Artifact Store**
- AC1: Raw session output is stored outside SQLite under the per-session artifact directory.
- AC2: PTY and contract-mode captures use the same session-root layout with transport-specific chunk suffixes.
- AC3: Artifact paths are stable enough to be referenced from `agent_session_logs` and error manifests.

**L1-FR-ARTIFACTS.2 — Capture Is Chunked From the First Layer**
- AC1: Session output is written as chunk-addressable files rather than a single monolithic transcript blob.
- AC2: Chunking supports replay, reconnect, lazy history fetch, and gap recovery requirements referenced by later layers.
- AC3: The chunking contract is active in Layer 1 and not deferred as an optimization.

**L1-FR-ARTIFACTS.3 — `stream.index.jsonl` Is the Authoritative Chunk Map**
- AC1: Each chunk has an index entry with path, first/last sequence numbers, byte length, wall-clock range, and SHA-256 hash.
- AC2: Sequence ranges are contiguous with no gaps inside a finished stream.
- AC3: Readers can locate capture data by sequence range using the index alone.
- AC4: After crash recovery or archive finalization, readers see only the longest contiguous committed sequence prefix; torn trailing index entries are truncated or regenerated from durable chunks.

**L1-FR-ARTIFACTS.4 — Chunk Writes Are Crash-Safe**
- AC1: Chunks are written to temporary files and atomically renamed into place.
- AC2: Hashing occurs incrementally during capture rather than as an after-the-fact repair step.
- AC3: `stream.index.jsonl` is append-only during capture and finalized at archive time.
- AC4: A crash mid-chunk never exposes a partial chunk under its final filename.
- AC5: Recovery ignores or garbage-collects temp files and rebuilds any missing/torn index entries from the durable chunk set before the session becomes `archived`.
- AC6: Long-running sessions are supported without a fixed per-session chunk cap; capture continues chunk rotation while maintaining only one open chunk per transport stream.

**L1-FR-ARTIFACTS.5 — SQLite Stores Artifact Metadata, Not Bulk Payloads**
- AC1: `agent_session_logs`, `raw_documents`, and `error_manifests` store artifact pointers and summary metadata only.
- AC2: Large PTY/stdout/stderr payloads are not inlined into SQLite blobs.
- AC3: Querying artifact metadata does not require reading the full payload files.

**L1-FR-ARTIFACTS.6 — Run Manifests Have a Stable Storage and Size Contract**
- AC1: Every run manifest is stored as `run-manifest.md` under the session artifact directory.
- AC2: The session record stores both manifest path and SHA-256 hash.
- AC3: The target manifest size remains under 50 KB.

**L1-FR-ARTIFACTS.7 — Run Manifest Frontmatter Separates Claimed and Observed Facts**
- AC1: Agent-authored frontmatter includes the fields listed in this section, including execution identity and item counts.
- AC2: Ambiguous factual claims use the `agent_claimed_*` prefix in frontmatter.
- AC3: Daemon-observed facts are recorded separately in the evidence appendix under `daemon_observed_*` or other machine-authored fields.

**L1-FR-ARTIFACTS.8 — Fallback Manifests Exist Even for Failed Sessions**
- AC1: If the agent cannot provide a narrative, the daemon synthesizes a machine-authored fallback manifest.
- AC2: Fallback manifests set `agent_narrative_missing: true`.
- AC3: Failed or interrupted runs never end without either a hybrid or machine-authored run manifest artifact.

### 2.6 Job Runner

The Job Runner claims queued jobs from `workflow_jobs`, executes them via the Agent Adapter, and persists results with stage tracking.

**Commit fence model:**

Every successful daemon write transaction increments a monotonic `commit_seq` (stored in `commit_clock` table). After commit, the daemon emits a `db.commit { commitSeq, touchedScopes }` event. Every `MutationAck` returns `visibleAfterSeq` — the `commit_seq` at which the mutation became durable. The desktop uses this to implement read-your-writes consistency without routing reads through the daemon (see Layer 2 §1.4).

`commit_seq` is **global to `state.db`**, not per table, per job kind, or per subsystem. Ordering is the SQLite commit order of successful write transactions. A rolled-back or failed write leaves `commit_seq` unchanged and returns no new `visibleAfterSeq`.

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

**L1-FR-RUNNER.1 — Every Durable Write Advances the Commit Fence**
- AC1: Successful daemon write transactions increment the singleton `commit_clock.commit_seq`.
- AC2: After commit, the daemon can emit `db.commit { commitSeq, touchedScopes }`.
- AC3: The returned `visibleAfterSeq` always matches the commit fence reached by the durable write.
- AC4: `commit_seq` is a single database-wide monotonic counter for all successful write transactions in `state.db`, never a per-table fence.
- AC5: Commit ordering for visibility semantics is the SQLite commit order of successful transactions.

**L1-FR-RUNNER.2 — Mutation Acknowledgement Means SQLite Durability**
- AC1: `MutationAck.accepted = true` is never sent for in-memory-only state.
- AC2: A write ack is emitted only after the SQLite transaction commits successfully.
- AC3: Retried acknowledgements with the same idempotency context do not create duplicate writes.
- AC4: Failed or rolled-back writes return no new `visibleAfterSeq` and do not advance `commit_seq`.

**L1-FR-RUNNER.3 — The Job Runner Owns the Claim-Execute-Persist Loop**
- AC1: The runner claims queued jobs from `workflow_jobs`.
- AC2: The runner reads the job payload, binds the correct skill, and invokes the adapter in the resolved mode.
- AC3: The runner monitors the full seven-phase lifecycle for the session it started.
- AC4: Success and failure both update `workflow_jobs` durably.

**L1-FR-RUNNER.4 — Successful Stage Writes Are Atomic Cohorts**
- AC1: Stage output rows, `process_stage_status`, and `process_stage_heads` updates commit in the same transaction.
- AC2: The shared `stage_run_id` is written in that same transaction.
- AC3: A failed transaction leaves neither partial authoritative outputs nor an advanced stage head.

**L1-FR-RUNNER.5 — `stage_run_id` Scope Rules Are Enforced**
- AC1: One `stage_run_id` corresponds to one atomic commit cohort for one scoped business unit.
- AC2: Set-replacement stages require one complete `stage_run_id` cohort for the full scope before head advancement.
- AC3: Additive stages may produce multiple `stage_run_id` cohorts from one agent session, one per completed scope unit.

**L1-FR-RUNNER.6 — Failure and Continuation Outcomes Diverge Cleanly**
- AC1: Failure creates an error manifest and does not advance the stage head.
- AC2: `continuation_required` persists committed work and enqueues the next segment with a resume token.
- AC3: Partial checkpoints from failed runs survive only as explicitly classified partial results, never as implicit success.

**L1-FR-RUNNER.7 — Sync Tracking Columns Are Populated From First Live Write**
- AC1: Every push-owned row written in Layer 1 sets `owner_id`, `row_version`, and `last_local_seq`.
- AC2: Layer 1 does not create `local_outbox` entries yet.
- AC3: Sync activation in Layer 4 does not require backfilling these columns for Layer 1 data.

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

In Layer 1's single-writer model, `commit_clock` is not a contention hotspot. Later layers may introduce concurrent claimers or writers, but they must preserve this same database-wide fence by updating `commit_clock` inside the authoritative write transaction rather than introducing per-table clocks.

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

**L1-FR-DB.1 — Layer 1 Uses One Metadata Database**
- AC1: All Layer 1 metadata tables live in `state.db`.
- AC2: Session byte streams, manifests, checkpoints, and downloaded documents stay in the filesystem artifact store.
- AC3: Layer 1 does not introduce or depend on a second SQLite metadata database.

**L1-FR-DB.2 — `commit_clock` Is the Global Visibility Fence**
- AC1: `commit_clock` exists as a singleton table with one authoritative `commit_seq`.
- AC2: Every successful daemon write transaction increments `commit_seq`.
- AC3: Desktop read-your-writes behavior can rely on this fence without routing reads through the daemon.
- AC4: Later layers may add concurrency, but they must preserve `commit_clock` as one database-wide visibility fence rather than introducing per-table or per-subsystem visibility counters.

**L1-FR-DB.3 — SQLite Boot Uses the Declared WAL/Integrity Pragmas**
- AC1: Startup configures WAL mode, synchronous mode, foreign keys, busy timeout, and cache size as listed here or an equivalent stricter setting.
- AC2: Startup runs `PRAGMA quick_check` before accepting write traffic.
- AC3: The daemon stores `schema_epoch` in `PRAGMA user_version`.

**L1-FR-DB.4 — SQLite Startup Failures Are Fatal and Explicit**
- AC1: Migration failure, `quick_check` failure, or incompatible schema epoch prevents the daemon from advertising readiness.
- AC2: The daemon does not bind itself as writable if startup integrity checks fail.
- AC3: Startup failures are surfaced with explicit operator-visible error output rather than deferred into later job failures.

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

**L1-FR-IPC.1 — The Daemon Publishes an Atomic Runtime Descriptor**
- AC1: `daemon.info.json` is written only after lock acquisition, database readiness, and socket bind succeed.
- AC2: The descriptor contains instance identity, PID, socket path, protocol version, schema epoch, state root, start time, and readiness flag.
- AC3: Descriptor writes are atomic so readers never observe a torn readiness file.

**L1-FR-IPC.2 — Single-Writer Uniqueness Is Enforced Before Write Traffic Starts**
- AC1: The daemon acquires `daemon.lock` before opening the write connection and before binding the socket.
- AC2: Failure to acquire the lock prevents a second writable daemon instance from starting.
- AC3: Local uniqueness is enforced by OS-level locking rather than by the `executors` table alone.

**L1-FR-IPC.3 — Desktop Handshake Negotiates Runtime Compatibility**
- AC1: The desktop sends `desktopVersion`, supported protocol range, expected schema epoch, and `lastSeenInstanceId`.
- AC2: On successful handshake, the daemon responds with selected protocol version, daemon version, current instance id, schema epoch, commit sequence, health summary, mutation availability, and active-session summary.
- AC3: Protocol-range mismatch returns an explicit handshake rejection and blocks further command use.
- AC4: `expectedSchemaEpoch != schemaEpoch` returns an explicit schema-epoch mismatch response containing both values and blocks further command or stream use until the client and daemon are upgraded into alignment.

**L1-FR-IPC.4 — Desktop Restart and Daemon Restart Have Different Recovery Rules**
- AC1: Desktop restart with the daemon still alive allows session-stream resume from the last seen sequence.
- AC2: Daemon restart invalidates PTY-stream resumability for pre-crash sessions.
- AC3: On daemon restart, sessions left in `spawned`, `active`, or `terminating` are reconciled by descendant cleanup plus synthesized crash error manifests.

**L1-FR-IPC.5 — Layer 1 Exposes a Handshake-Only Socket Surface**
- AC1: The daemon binds the socket from startup in Layer 1.
- AC2: Layer 1 accepts the `hello`/`welcome` handshake for tooling and health checks.
- AC3: Layer 1 does not accept mutation commands until Layer 2 activates the bridge handlers.

**L1-FR-IPC.6 — Lifecycle Frames Include Daemon-Owned Phase Transitions**
- AC1: Adapter `ControlEvent`s cover the adapter-owned semantic transitions.
- AC2: The daemon emits additional lifecycle frames for `created`, `terminating`, `reaped`, and `archived`.
- AC3: The `reaped` lifecycle frame includes `cleanupVerdict`.
- AC4: The `archived` lifecycle frame includes `archiveSeq`.

**L1-FR-IPC.7 — UDS Access Is Restricted to the Local Operator Identity**
- AC1: Layer 1 uses a pathname UDS in a `0700` runtime directory.
- AC2: The daemon verifies peer credentials on connect.
- AC3: Peers whose UID does not match the daemon/desktop user are rejected.

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

**L1-FR-DB.5 — `agent_session_logs` Is the Authoritative Session Metadata Record**
- AC1: Every agent session has exactly one `agent_session_logs` row for its Layer 1 execution.
- AC2: The row includes execution identity, lifecycle status, capture pointers, cleanup verdict, and run-manifest references listed in this section.
- AC3: Sensitive local-only fields stay local even though later layers may project redacted summaries elsewhere.

#### 3.1.2 `raw_documents` (Local-Only)

**Purpose:** Downloaded tender document payloads from live source acquisition. Either stored inline as `payload_json` or as file reference via `storage_path`.

**Key columns:** `source_id`, `content_type`, `source_url`, `payload_json` OR `storage_path`, `metadata`.

**CHECK constraint:** `payload_json IS NOT NULL OR storage_path IS NOT NULL` — at least one storage form required.

**Authority:** Local-only. Per-daemon operational state.

**L1-FR-DB.6 — `raw_documents` Preserves Notice-Document Payload Provenance**
- AC1: Every stored raw document records source identity, content type, and source URL metadata.
- AC2: Each row stores payload content either inline as `payload_json` or by `storage_path`, but never neither.
- AC3: The CHECK constraint in this section is enforced by the schema and by Layer 1 writes.

#### 3.1.3 `error_manifests` (Local-Only)

**Purpose:** Full error context for every pipeline failure. Includes job reference, pipeline stage, workflow step, structured error context, agent session link, and Markdown artifact path for human-friendly debugging.

**Key columns:** `job_id`, `process_id`, `pipeline_stage`, `workflow_step`, `error_type`, `error_message`, `error_context` (JSON — full locally, redacted in Supabase), `skill_path`, `agent_session_id`, `markdown_path`, `resolved`, `resolved_at`, `resolved_by`.

**Authority:** Local-only. Summaries pushed as `error_manifest_summaries` in Layer 4.

**L1-FR-ERRORS.13 — `error_manifests` Stores Full Failure Context**
- AC1: Every classified pipeline or runtime failure writes an `error_manifests` row.
- AC2: The row links the failure to job, process, workflow step, error type, and session context when available.
- AC3: The row stores the Markdown artifact path for human review.

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

**L1-FR-DB.7 — `executors` Separates Org Identity From Daemon Instance Identity**
- AC1: Layer 1 records the executor identity split of `owner_id`, `executors.id`, and future `push_stream_id`.
- AC2: MVP operation may use a single constant `owner_id`, but the schema preserves distinct daemon-instance identity.
- AC3: Later sync activation does not require reshaping the executor identity model.

#### 3.1.5 `source_cursors` (Local-Only)

**Purpose:** Per-source ingestion watermarks for incremental retrieval. Each source has one or more cursor keys (e.g., `last_published_date`, `last_page_number`) with cursor values.

**Key columns:** `source_id`, `cursor_key`, `cursor_value`, `updated_at`. Primary key: `(source_id, cursor_key)`.

**Authority:** Local-only. Per-daemon operational state.

**L1-FR-DB.8 — `source_cursors` Is the Durable Incremental Retrieval Store**
- AC1: Cursor state is keyed per source and cursor key.
- AC2: Cursor writes occur only at committed safe boundaries defined by the ingestion skill.
- AC3: A failed run never advances the source cursor past uncommitted work.

### 3.2 Tables Activated in Layer 1

These tables were created in Layer 0 (with full schema including all future-layer columns as nullable) and first used by the gold-set evaluation harness. In Layer 1, they receive their **first live-source writes** via the daemon-managed pipeline loop.

| Table | What Changes in L1 |
|-------|-------------------|
| `workflow_jobs` | Now actively used by Job Runner for job lifecycle. `parent_job_id` and `validation_verdict_id` remain `NULL`, while `validation_required` remains `false` in Layer 1 (validation system activates in L3). |
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

**L1-FR-DB.9 — Activated Layer 0 Tables Receive First Live Writes Without Schema Drift**
- AC1: Layer 1 writes live-source data into the activated Layer 0 tables listed in this section.
- AC2: Those writes populate sync-tracking columns on first write.
- AC3: Layer 1 activates usage without redefining Layer 0 table shape or authority classification.

### 3.3 Stage Tracking Model (Authoritative)

> **This section is the authoritative source for the stage-to-scope-grain mapping.** Layer 0 §3 owns stage processing logic. This section owns stage tracking infrastructure.

#### 3.3.1 Core Concepts

**`stage_run_id`** = one atomic commit cohort for one scoped business unit. A single agent session (`session_execution_id`) may produce multiple `stage_run_id` commits. For example, one eBay enrichment session may produce 5 committed `stage_run_id`s (one per candidate-source pair that completed).

**`process_stage_status`** records every execution of every stage at every scope grain. Each row has a `stage_run_id`, state, freshness tracking, and basis hash.

**`process_stage_heads`** tracks the current (latest successful) stage run per `(process_id, stage, scope_kind, scope_id)` tuple. Updated atomically when a stage succeeds for its full scope.

**`scope_kind`** is constrained to exactly six values: `process`, `notice`, `lot`, `candidate`, `candidate_source_pair`, `candidate_quote_scenario`.

**Execution-record rules:**

- `stage_run_id` is allocated before the commit transaction opens for the cohort it names. Retries and reruns always get a fresh `stage_run_id`; failed or obsolete cohorts are never reused.
- `process_stage_status` is append-only by execution attempt. A rerun inserts a new row with a new `stage_run_id`; it does not rewrite prior attempts into the new cohort.
- Additive stages may produce a mix of `succeeded`, `failed`, and `obsolete` `process_stage_status` rows from one agent session because each scope unit is committed independently.
- Failed set-replacement attempts may persist only failure-status/debug context; authoritative push-owned domain rows from the failed cohort must roll back with the transaction and must not linger as orphan rows.

**L1-FR-PIPELINE.16 — `stage_run_id` Is the Atomic Commit Cohort Identifier**
- AC1: One `stage_run_id` represents one atomic commit cohort for one scoped business unit.
- AC2: A single session may produce multiple `stage_run_id` values when additive stages commit multiple scoped units.
- AC3: `process_stage_status` and `process_stage_heads` treat `stage_run_id` as the authoritative cohort key.
- AC4: Each retry or rerun receives a fresh `stage_run_id`; failed or obsolete cohort identifiers are never reused.

**L1-FR-PIPELINE.17 — `scope_kind` Uses Only the Six Canonical Values**
- AC1: Layer 1 stage tracking uses only `process`, `notice`, `lot`, `candidate`, `candidate_source_pair`, and `candidate_quote_scenario`.
- AC2: Ingest remains a precursor tracked at job/cursor level until notice/process identity exists.
- AC3: Layer 1 does not invent extra scope kinds for convenience.

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

**L1-FR-PIPELINE.18 — Stage-to-Scope Mapping Matches the Cross-Layer Stage Scope Matrix**
- AC1: Every stage in this table uses the same runtime grain and `scope_kind` defined in `layer-strategy.md` §10.
- AC2: Set-replacement vs additive semantics match the cross-layer contract for the same stage.
- AC3: Layer 1 does not redefine scope grain for a stage that later layers also use.

#### 3.3.3 Commit Semantics

**Set-replacement stages** must commit atomically for their full scope. Partial results from set-replacement stages (e.g., lots 1-7 of 12 extracted) are stored as checkpoint/debug artifacts but do NOT advance the stage head. The stage must succeed for the complete scope or fail entirely.

**Additive stages** may commit independently per their scope grain. If product extraction succeeds for lots 1-2 but fails on lot 3, lots 1-2 are authoritative. If eBay search succeeds for candidates 1-5 but fails on candidate 6, candidates 1-5 have valid quotes.

**L1-FR-PIPELINE.19 — Commit Semantics Distinguish Set-Replacement From Additive Stages**
- AC1: Set-replacement stages never advance the head from a partial scope result.
- AC2: Additive stages may leave previously committed successful scope units authoritative while sibling units fail.
- AC3: Checkpoints/debug artifacts from partial work do not masquerade as committed stage outputs.
- AC4: Failed set-replacement attempts leave no orphaned authoritative push-owned domain rows committed under the failed `stage_run_id`.
- AC5: Additive stages record per-scope-unit outcomes so one agent session can show both succeeded and failed scope units explicitly.

#### 3.3.4 Stage Head Advancement Rules

1. **Set-replacement:** Stage head advances only after ALL outputs for the scope unit are committed in a single transaction. The `stage_run_id` ties all outputs together.
2. **Additive:** Stage head advances per individual scope unit. Each committed unit gets its own `stage_run_id`.
3. **Atomic pair:** Stage output write + `stage_run_id` assignment + stage head advancement happen in the same SQLite transaction. Never split across transactions.
4. **Freshness tracking:** `process_stage_heads.freshness_state` starts as `'fresh'`. Upstream changes may set it to `'soft_stale'` or `'hard_invalid'` (see staleness model below).

**L1-FR-PIPELINE.20 — Stage-Head Advancement Is Atomic and Scope-Aware**
- AC1: Stage-head advancement occurs in the same transaction as stage output writes.
- AC2: Set-replacement stages advance only after the full scope unit succeeds.
- AC3: Additive stages advance per committed scope unit.
- AC4: `freshness_state` is initialized and updated according to the staleness model instead of being left implicit.
- AC5: A rerun inserts a new `process_stage_status` row with a fresh `stage_run_id`; `process_stage_heads` move by pointing to the new successful cohort rather than mutating historical status rows in place.

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

**L1-FR-PIPELINE.21 — Staleness and Basis-Hash Guards Prevent Stale Commits**
- AC1: Hard-invalid outputs are excluded from current rollups and trigger rerun from the earliest invalid stage.
- AC2: Soft-stale outputs remain viewable but constrain downstream automation as described here.
- AC3: Every job carries `expected_input_basis_hash`.
- AC4: Basis-hash mismatch causes the current job to be marked obsolete and replaced with a rerun from the correct stage boundary.

### 3.4 Source Configuration (Layer 1 Stub)

In Layer 1, source configuration is **bootstrapped locally** — hardcoded or seeded runtime definitions in the `sources` table. There is no remote-owned `shared_source_configs` pull yet (that activates in Layer 4).

The `sources` table is a local-only runtime copy. In Layer 4, it will be populated from `shared_source_configs` during pull sync. In Layer 1, it contains:

| Source | `kind` | `config` contents |
|--------|--------|-------------------|
| Find a Tender | `procurement` | Base URL, OCDS API endpoint, rate limits |

Contracts Finder is added in Layer 2.

**L1-FR-DB.10 — Source Configuration Is Locally Seeded in Layer 1**
- AC1: Layer 1 runs with a local runtime copy of source configuration.
- AC2: FTS is the only procurement source active in Layer 1.
- AC3: Remote-owned `shared_source_configs` are not required until Layer 4.

### 3.5 Job Runner Integration

The Job Runner uses the tables above as follows:

1. **Job creation:** A trigger (CLI in L1, cron in L3) creates a `workflow_jobs` row with `kind`, `scope_kind`, `scope_id`, `input_payload` (including `expected_input_basis_hash`).
2. **Job claim:** The runner claims the job (sets `lease_owner`, `lease_expires_at`, `started_at`, increments `attempt_count`). Single-threaded execution in L1 (concurrent claiming in L3).
3. **Skill execution:** The runner selects the adapter (PTY or contract) based on the bound skill's `interaction_profile` and launches the agent session. An `agent_session_logs` row is created.
4. **Stage output commit:** On skill completion, the runner validates the result contract, writes pipeline table outputs with `stage_run_id`, creates a `process_stage_status` row, and advances `process_stage_heads` — all in one SQLite transaction.
5. **Run manifest capture:** The runner stores the agent-authored narrative + daemon evidence appendix as an immutable artifact. `run_manifest_path` and `run_manifest_hash` are written to `agent_session_logs`.
6. **Error handling:** On failure, an `error_manifests` row is created with full context. The `agent_session_logs` row gets `error_type`, `error_message`, and `cleanup_verdict`.
7. **Process-group cleanup:** The runner ensures the session's process group is fully reaped. `cleanup_verdict` is mandatory: `clean` (normal exit), `forced` (kill escalation needed), `leaked` (orphaned processes detected).

**L1-FR-RUNNER.8 — Job Claims Are Basis-Hash Guarded and Cleanup-Complete**
- AC1: Job claim records lease/attempt metadata before execution begins.
- AC2: The runner validates `expected_input_basis_hash` against current stage heads before committing outputs.
- AC3: Success, failure, and cleanup verdict are all persisted before the job is considered finished.

**L1-FR-RUNNER.9 — Set-Replacement Partial Success Never Leaks Into Authoritative Heads**
- AC1: If a set-replacement stage fails after producing only part of its intended scope, authoritative push-owned domain-table writes from that failed attempt are rolled back from the transaction and remain absent from current stage heads.
- AC2: Salvageable partial data from a failed set-replacement attempt may persist only as checkpoint/debug artifacts or explicit failure context, not as current authoritative stage outputs.
- AC3: A later successful retry of the same set-replacement scope advances the head with a fresh successful `stage_run_id`, not by reusing a partially failed cohort.
- AC4: The retry inserts a new `process_stage_status` row with that fresh `stage_run_id`; prior failed attempts remain audit-visible but are never rewritten into success.

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

Reserved forward-compat columns are **schema slots, not active behavior**. In Layer 1, writers must preserve the null/default state described below rather than opportunistically populating later-layer fields.

| Provision | Target Layer | Status in L1 |
|-----------|-------------|-------------|
| Sync tracking columns on all push-owned writes | L4 | Populated from first write; no outbox entries yet |
| `workflow_jobs.parent_job_id` | L3 | Column exists but L1 writers MUST leave it `NULL`; L1 control flow ignores it |
| `workflow_jobs.validation_required` | L3 | Constitutional invariant in L1: always `false`; L1 writers MUST NOT set it `true` |
| `workflow_jobs.validation_verdict_id` | L3 | Column exists but L1 writers MUST leave it `NULL` |
| `pipeline_assessments.validation_state` | L3 | Default `'unverified'` from first write; L1 writers MUST NOT emit validator-derived states |
| `agent_session_logs.run_manifest_path/hash` | L3 | Populated from first skill run |
| Adapter trait designed for extension | Future | New agent CLIs via new adapter implementations |
| Error manifest format includes validator fields | L3 | Fields exist but remain unpopulated in L1 |

**L1-FR-DB.11 — Reserved Forward-Compatibility Columns Stay Reserved in Layer 1**
- AC1: Layer 1-created or Layer 1-updated `workflow_jobs` rows leave `parent_job_id` and `validation_verdict_id` as `NULL` unless this document explicitly says otherwise.
- AC2: `workflow_jobs.validation_required` remains `false` for all Layer 1 jobs, and Layer 1 control flow does not branch on validator-only fields.
- AC3: `pipeline_assessments.validation_state` remains `'unverified'` in Layer 1; validator-derived states are reserved for later layers.
- AC4: Later-layer-only fields may be surfaced for diagnostics, but Layer 1 does not treat them as active behavior.

---

## 4. Skills, Run Manifests & Error Manifests

### 4.1 New Skills (Layer 1)

Two skills are introduced in Layer 1. Both follow the production skill directory standard established in Layer 0 §4 (SKILL.md + result.schema.json + checkpoint.schema.json + canary.inputs.jsonl + golden.assertions.yaml).

**L1-FR-SKILLS.1 — Layer 1 Skills Conform to the Layer 0 Production Skill Directory Standard**
- AC1: Each Layer 1 skill directory includes the required Layer 0 production files unless this document explicitly classifies the skill as markdown-only sufficient.
- AC2: Each skill declares an interaction profile, model profile, output contract, and resume behavior compatible with daemon orchestration.
- AC3: The daemon binds jobs to explicit skill id/version pairs instead of executing unversioned playbooks.

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

**L1-FR-SKILLS.2 — The FTS Ingestion Skill Is a PTY-Interactive, Cursor-Driven Skill**
- AC1: The skill requires PTY execution because it depends on browser-backed navigation/extraction.
- AC2: The skill accepts cursor input from `source_cursors` and emits checkpoints at notice-batch boundaries.
- AC3: The skill declares a live canary that checks basic FTS/OCDS viability before broader traffic depends on it.

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

**L1-FR-SKILLS.3 — The Manifest-Generation Skill Is a Deterministic Contract Skill**
- AC1: The skill executes in `headless_contract` mode.
- AC2: The skill consumes local pipeline data only and requires no browser or external auth.
- AC3: The skill emits a process-scoped opportunity manifest and computes `decisionBasisHash`.

### 4.2 Run Manifest System (Authoritative Layer 1 Definition)

> **Ownership:** This is the authoritative definition of the run manifest system. Layer-strategy.md §10 carries a compact cross-layer summary. Later layers may include concise recaps for self-containment but must not redefine or contradict this contract.

Every skill execution produces a **hybrid-authored run manifest** — an agent-written narrative combined with a daemon-appended evidence appendix. This is the primary input for semantic validation (Layer 3+) and operator review.

**L1-FR-PROMPT.1 — Kickoff Prompts Carry the Inputs Needed to Produce a Valid Run Manifest**
- AC1: The kickoff prompt injects the bound skill content, job parameters, and any resume token required for the run.
- AC2: The kickoff prompt includes explicit run-manifest instructions so the agent knows the required narrative sections and frontmatter expectations.
- AC3: Prompt rendering does not rely on operator memory or out-of-band context to produce the required manifest structure.

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

**L1-FR-ARTIFACTS.9 — Every Skill Run Produces a Hybrid-Authored Run Manifest Contract**
- AC1: Every skill execution yields either a hybrid-authored run manifest or a machine-authored fallback.
- AC2: The run manifest is the primary review artifact for later validation and operator inspection.
- AC3: The run manifest is immutable once finalized by the daemon.

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

**L1-FR-ARTIFACTS.10 — Run-Manifest Frontmatter Is Structurally Validatable**
- AC1: Frontmatter includes the execution identity, timing, outcome class, item counts, and agent-claimed environment fields shown in this section.
- AC2: Fields whose truth is self-reported rather than daemon-observed use `agent_claimed_*` naming.
- AC3: Missing required frontmatter fields cause the daemon to classify the narrative as incomplete and fall back or fail accordingly.

#### Agent Narrative Sections

Human-readable, agent-authored Markdown:

- **Summary:** What the agent did and what it found.
- **Steps Taken:** Navigation and extraction sequence.
- **Decisions Made:** Choices the agent made during the run with reasoning.
- **Warnings:** Anomalies, degradations, or issues encountered.
- **Evidence:** Key evidence supporting the results, with claim/evidence references pointing to result contract fields, daemon appendix anchors, or artifact refs. Not every sentence needs a reference — major claims (correct page, correct locale, extraction completeness, pricing plausibility) should be anchored.

**L1-FR-ARTIFACTS.11 — Agent Narrative Sections Are Required and Evidence-Anchored**
- AC1: The narrative contains Summary, Steps Taken, Decisions Made, Warnings, and Evidence sections.
- AC2: Major claims about page correctness, locale/currency, extraction completeness, and pricing plausibility are anchorable to contract fields, daemon appendix data, or artifact refs.
- AC3: Missing or empty required sections cause the daemon to treat the narrative as incomplete.

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

**L1-FR-ARTIFACTS.12 — The Daemon Evidence Appendix Contains Independent Verification Anchors**
- AC1: The daemon appends machine-observed URL, page-kind, fingerprint, locale/currency, checkpoint, result-count, OQG, and artifact-reference fields listed in this section.
- AC2: Appendix values come from observability layers, validated contracts, or artifact metadata rather than agent prose.
- AC3: Later validation logic can compare appendix values to agent-claimed values without reparsing the transcript.

#### `agent_claimed_*` vs `daemon_observed_*` Naming Convention

**Critical rule:** Self-reported fields use `agent_claimed_*`. Daemon-observed fields use `daemon_observed_*`. Fields that sound objective (e.g., `page_fingerprint_matched`, `locale_verified`) must never be ambiguous between "the agent claims it verified this" and "the daemon observed and verified this."

This naming convention prevents false certainty from anchoring operators and validators. Discrepancies between agent-claimed and daemon-observed states are **high-signal issues** that validators should cross-check.

**L1-FR-ARTIFACTS.13 — Claimed Facts and Observed Facts Never Share Ambiguous Field Names**
- AC1: Agent claims and daemon observations use distinct prefixes as defined in this section.
- AC2: Fields that could be mistaken for objective verification are not named ambiguously.
- AC3: Divergence between claimed and observed values remains visible to validators and operators.

#### Fallback Mode

If the worker agent fails to emit a run manifest (crash, timeout, invalid output), the daemon synthesizes a **minimum machine-authored run manifest** from observability layers 1-3 alone:

- Contains only the daemon evidence appendix (no agent narrative)
- `agent_narrative_missing: true` in frontmatter
- Ensures some structured review artifact always exists
- Enables validators and operators to assess what happened even when the agent could not narrate

The worst runs must not leave the thinnest review surface.

**L1-FR-ARTIFACTS.14 — Fallback Run Manifests Are Sufficient for Failure Review**
- AC1: Fallback manifests contain the daemon evidence appendix even when agent prose is absent.
- AC2: Fallback manifests explicitly mark `agent_narrative_missing: true`.
- AC3: Validators and operators can still determine what the daemon observed from the fallback artifact alone.

**L1-FR-ARTIFACTS.15 — Claim/Observation Disagreements Are Surfaced, Not Hidden**
- AC1: When an `agent_claimed_*` field and its corresponding daemon-observed fact disagree, the final manifest preserves both values rather than overwriting one with the other.
- AC2: The daemon appends a machine-authored disagreement summary identifying each mismatched field and both values.
- AC3: A disagreement marks the run as warning-level review-required for validator/operator consumption, even if the result contract otherwise validates.
- AC4: Disagreement alone does not silently mutate authoritative pipeline state; any quarantine or failure outcome must still come from explicit validation or error rules elsewhere in this document.

#### Terminology Disambiguation

Three things called "manifest" in this architecture:

| Term | What It Is | Stored In |
|------|-----------|-----------|
| **Opportunity manifest** | Markdown summary of an opportunity for operator review | `manifests` table |
| **Error manifest** | Structured pipeline failure record for debugging | `error_manifests` table |
| **Run manifest** | Hybrid-authored narrative of what a skill run did (this system) | Artifact store, referenced from `agent_session_logs` |

Always use the full term "run manifest" when referring to this concept.

### 4.3 Error Taxonomy & Error Manifest System

> **Ownership:** This section defines the error taxonomy framework and Layer 0-1 base error types. Customs-specific error types (`TARIFF_BRANCH_DIVERGENCE`, `MEASURE_INTERACTION_UNKNOWN`, and related customs-layer codes) are defined in Layer 5 §3. Validation-specific error types (`VALIDATION_FLAGGED_INSUFFICIENT_BASIS`, `VALIDATION_REJECTED`, and related validator-layer codes) are defined in Layer 3 §2. This section defines the framework; later layers extend it with domain-specific codes.

#### Error Categories

| Category | Prefix | Scope | Examples |
|----------|--------|-------|---------|
| Pipeline errors | various | Data quality issues in the intelligence pipeline | `SOURCE_CHANGED`, `NOTICE_PARSE_FAILED`, `COMMODITY_CODE_LOW_CONFIDENCE` |
| Agent errors | `AGENT_*` | Runtime failures in agent sessions | `AGENT_STALLED`, `AGENT_AUTH_CHALLENGE`, `AGENT_SUSPECT_OUTPUT` |
| Host errors | `HOST_*` | Environment/infrastructure issues | `HOST_SLEEP_INTERRUPTED`, `HOST_RESOURCE_EXHAUSTED` |
| Source errors | `SOURCE_*` | Source website/API issues | `SOURCE_SOFT_BLOCKED`, `SOURCE_CAPTCHA_BLOCKED`, `PAGE_SHAPE_DRIFT` |
| Bootstrap/storage errors | various | Readiness, skill-integrity, and durable-write failures | `SKILL_DIRECTORY_INVALID`, `SQLITE_WRITE_FAILED`, `DAEMON_STARTUP_FAILED` |
| Validation errors | `VALIDATION_*` | Semantic validation outcomes (Layer 3+) | `VALIDATION_REJECTED`, `VALIDATION_FLAGGED` |

#### Layer 0-1 Base Error Types

**Pipeline errors (active from Layer 0 intelligence pipeline):**

| Requirement ID | Code | Trigger | Action |
|----------------|------|---------|--------|
| `L1-FR-ERRORS.14` | `SOURCE_CHANGED` | Source website changed | Flag skill needs update; create error manifest |
| `L1-FR-ERRORS.15` | `NOTICE_PARSE_FAILED` | Notice parse failed | Store raw payload; mark manual review |
| `L1-FR-ERRORS.16` | `GOODS_CLASSIFICATION_LOW_CONFIDENCE` | Goods classification uncertain | Keep notice; require review |
| `L1-FR-ERRORS.17` | `SUPPLIER_MATCH_LOW_CONFIDENCE` | Supplier match weak | Do not auto-qualify |
| `L1-FR-ERRORS.18` | `COMMODITY_CODE_LOW_CONFIDENCE` | Commodity code uncertain | Route to manual review |
| `L1-FR-ERRORS.19` | `REVENUE_BASIS_UNCERTAIN` | Framework ceiling or dynamic market | Margin gate uses discounted revenue |
| `L1-FR-ERRORS.20` | `FPO_ACCESS_UNAVAILABLE` | HMRC FPO API unavailable | Skip FPO; continue with remaining recall sources; confidence penalty |

**Agent errors (active from Layer 1 daemon):**

| Requirement ID | Code | Trigger | Action |
|----------------|------|---------|--------|
| `L1-FR-ERRORS.21` | `AGENT_STALLED` | No output or no progress for timeout | Kill process group; create error manifest |
| `L1-FR-ERRORS.22` | `AGENT_AUTH_CHALLENGE` | Re-login, MFA, SSO, permission prompt | Flag for operator; create error manifest |
| `L1-FR-ERRORS.23` | `AGENT_SKILL_FAILED` | Skill playbook failed at specific step | Create error manifest with skill path and step |
| `L1-FR-ERRORS.24` | `AGENT_OUTPUT_CONTRACT_INVALID` | Output fails schema validation | Preserve transcript + artifact; fail job |
| `L1-FR-ERRORS.25` | `AGENT_SUSPECT_OUTPUT` | Schema-valid but fails OQG semantic/drift checks | Quarantine results; keep pipeline unchanged |
| `L1-FR-ERRORS.26` | `AGENT_PARTIAL_FAILURE` | Valid checkpoint before crash, no final contract | Persist checkpoints; never upgrade to success |
| `L1-FR-ERRORS.27` | `AGENT_INTERNAL_ERROR` | Internal error from agent CLI | Create error manifest with trace |
| `L1-FR-ERRORS.28` | `AGENT_RUN_FAILED` | Non-zero exit, no specific pattern | Create error manifest |
| `L1-FR-ERRORS.29` | `AGENT_CLEANUP_INCOMPLETE` | Descendant processes leaked | Record leaked pids; flag for operator |
| `L1-FR-ERRORS.30` | `AGENT_DAEMON_CRASH_DURING_SESSION` | Daemon crashed while PTY session was active | On restart: detect running/orphaned sessions, kill descendants, synthesize error manifest; PTY stream is NOT resumable |
| `L1-FR-ERRORS.31` | `AGENT_PROTOCOL_MISMATCH` | Unexpected CLI output format | Canary mode; create error manifest |

**Source/skill errors (active from Layer 1):**

| Requirement ID | Code | Trigger | Action |
|----------------|------|---------|--------|
| `L1-FR-ERRORS.32` | `SOURCE_SOFT_BLOCKED` | HTTP 200 but no useful data | Capture artifacts; fail the run explicitly for operator review (later layers may add cooldown policy) |
| `L1-FR-ERRORS.33` | `SOURCE_CAPTCHA_BLOCKED` | Hard CAPTCHA requiring human | Emit `blocked_human_action`; alert operator |
| `L1-FR-ERRORS.34` | `AUTH_STATE_STALE` | Session loads but privileged data missing | Preserve evidence; fail explicitly for operator re-authentication (later layers may automate refresh/retry) |
| `L1-FR-ERRORS.35` | `PAGE_SHAPE_DRIFT` | Page fingerprint mismatch | Flag skill needs review; create error manifest |
| `L1-FR-ERRORS.36` | `LOCALE_OR_CURRENCY_DRIFT` | Wrong locale/currency/market variant | Fail fast; require config review |

**Host errors (active from Layer 1):**

| Requirement ID | Code | Trigger | Action |
|----------------|------|---------|--------|
| `L1-FR-ERRORS.37` | `HOST_SLEEP_INTERRUPTED` | Monotonic time gap during session | Taint browser sessions; create error manifest |
| `L1-FR-ERRORS.38` | `HOST_RESOURCE_EXHAUSTED` | Memory/CPU pressure | May preempt lower-priority sessions |

**Bootstrap/storage errors (active from Layer 1):**

| Requirement ID | Code | Trigger | Action |
|----------------|------|---------|--------|
| `L1-FR-ERRORS.40` | `SKILL_DIRECTORY_INVALID` | Required skill files, metadata, schemas, or lint invariants are missing or malformed at bind/lint time | Fail CI or runtime binding before spawn; surface lint context explicitly |
| `L1-FR-ERRORS.41` | `SQLITE_WRITE_FAILED` | SQLite write/commit fails because of disk full, I/O error, corruption, or busy-timeout exhaustion | Fail mutation/job, emit no new `visibleAfterSeq`, advance no stage head, surface storage failure |
| `L1-FR-ERRORS.42` | `DAEMON_STARTUP_FAILED` | Lock acquisition, migration, integrity check, descriptor write, or socket readiness step fails before readiness | Do not advertise readiness; emit explicit operator-visible startup failure |

#### Skill Outcome Classes (Non-Error)

Not every non-success outcome is an error:

| Outcome | When | Daemon Action |
|---------|------|--------------|
| `continuation_required` | Skill exited cleanly at safe boundary with remaining work | Accept committed results; enqueue continuation segment with resume token |
| `degraded_completed` | Skill completed with reduced coverage/confidence | Accept results with degradation flag; apply confidence penalty |
| `blocked_human_action` | Human intervention required (CAPTCHA, manual approval) | Alert operator; pause job; preserve session state |

**Key distinction:** `continuation_required` has a valid final contract with continuation data. `AGENT_PARTIAL_FAILURE` has only prior checkpoints and no final contract.

**Common acceptance criteria for `L1-FR-ERRORS.14` through `L1-FR-ERRORS.42`:**
- AC1: When the trigger condition in a row is met, Layer 1 classifies the failure with that specific code when sufficient evidence exists, rather than collapsing it into an unspecified generic failure.
- AC2: Layer 1 performs the row's declared action before treating the job/session as complete.
- AC3: The resulting error manifest includes the row-specific trigger context required to debug or route the issue.
- AC4: If the row says pipeline state is quarantined, retried, kept reviewable, or left unchanged, the Job Runner enforces that exact outcome explicitly.

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

**L1-FR-ERRORS.39 — Every Error Manifest Includes Runtime and Resolution Context**
- AC1: Error manifests record stage, workflow step, error code, human-readable message, and structured debugging context.
- AC2: Runtime context includes CLI vendor/version, adapter version, model/config identity, process identifiers, cleanup verdict, and last valid checkpoint when available.
- AC3: Error manifests link to related session artifacts and include a next debugging action.
- AC4: Resolution tracking fields support later operator follow-up without changing the original failure record.

### 4.4 Skill Linting CI Gate

A CI gate runs on every commit that touches a skill directory. It validates compliance with the production skill directory standard (see Layer 0 §4).

**Checks:**

| Check | Validation |
|-------|-----------|
| `SKILL.md` present | File exists in skill directory root |
| Required metadata present | `interaction_profile`, `result_schema_version`, `model_profile` fields in SKILL.md; optional `agent_cli` may pin `claude` or `codex` per skill |
| Trigger boundary sections | "Use when", "Do not use when", "Inputs", "Outputs", "Non-goals" sections in SKILL.md |
| `result.schema.json` present | Valid JSON Schema file |
| `checkpoint.schema.json` present | Valid JSON Schema file |
| `canary.inputs.jsonl` present | Non-empty JSONL file |
| `golden.assertions.yaml` present | Valid YAML file |
| Failure codes enumerated | SKILL.md lists possible failure codes |
| Resume strategy declared | SKILL.md describes resume behavior |
| Unsafe side-effect class | Portal/bid skills declare side-effect class per step |
| Bounded pagination/scroll rules | `maxPages`, `maxScrollCycles`, and equivalent explicit bounds present |
| Run manifest instructions | SKILL.md includes narrative instruction block for agent |

**Implementation:** `skill_lint.rs` in `crates/daemon` — a standalone validation pass that can run in CI without the full daemon.

**L1-FR-SKILLS.4 — Skill Linting Is a Standalone CI Gate**
- AC1: The lint gate validates the required files, metadata, trigger-boundary sections, schemas, canary input file, golden assertions, failure-code enumeration, resume strategy, side-effect declarations, pagination bounds, and run-manifest instructions listed in this section.
- AC2: The lint gate runs without requiring the full daemon runtime to boot.
- AC3: A commit touching a skill directory fails CI if any required check in this section fails.
---

## 5. Testing & Acceptance

### 5.1 Definition of Done

All items must pass before Layer 1 is complete:

| # | Criterion | Verification | Related Requirements |
|---|----------|-------------|----------------------|
| 1 | `cargo check --all-targets` and `cargo clippy --all-targets -- -D warnings` pass | Automated: CI gate | `L1-FR-DAEMON.2`, `L1-FR-DAEMON.3` |
| 2 | Rust binary launches skills via CLI (both PTY and contract mode) | Manual: `./daemon --run-skill skills/supplier-price-enrichment/ebay --mode pty` | `L1-FR-ADAPTER.1`-`L1-FR-ADAPTER.18`, `L1-FR-PTY.1`-`L1-FR-PTY.4` |
| 3 | Manual CLI trigger ingests a live FTS batch using source-batched skill execution | Manual: `./daemon --ingest fts` produces notice records | `L1-FR-PIPELINE.2`-`L1-FR-PIPELINE.8`, `L1-FR-PIPELINE.22`, `L1-FR-SKILLS.2` |
| 4 | First live end-to-end local loop completes for at least one notice: normalize -> prequalify_notice -> acquire_docs -> extract lots -> qualify_lot -> extract products -> price-signal match -> gross gate -> landed cost -> manifest | Manual: verify manifest artifact exists | `L1-FR-PIPELINE.9`-`L1-FR-PIPELINE.22`, `L1-FR-RUNNER.3`-`L1-FR-RUNNER.9` |
| 5 | Verifiable output exists without later layers: CLI summary plus a local markdown manifest artifact proves the loop completed | Manual: inspect output file | `L1-FR-PIPELINE.15`, `L1-FR-ARTIFACTS.6`-`L1-FR-ARTIFACTS.15`, `L1-FR-SKILLS.3` |
| 6 | Process-group supervision verified: every session gets pgid, cleanup verdict recorded (clean/forced/leaked) | Automated: test + manual inspection of `agent_session_logs` | `L1-FR-PTY.1`-`L1-FR-PTY.4`, `L1-FR-REAPER.1`-`L1-FR-REAPER.5`, `L1-FR-DAEMON.5`-`L1-FR-DAEMON.12` |
| 7 | Artifact capture works: PTY bytes compressed and stored, session metadata in `agent_session_logs` | Automated: test verifies compressed artifact exists | `L1-FR-PIPELINE.8`, `L1-FR-ARTIFACTS.1`-`L1-FR-ARTIFACTS.5`, `L1-FR-DB.5` |
| 8 | Run manifests generated: agent narrative + daemon evidence appendix; fallback machine-authored if agent fails to narrate | Automated: test verifies manifest exists and has both sections | `L1-FR-PIPELINE.10`, `L1-FR-PROMPT.1`, `L1-FR-ARTIFACTS.6`-`L1-FR-ARTIFACTS.14` |
| 9 | Error manifest system operational: pipeline failures produce Markdown error manifests with full context | Automated: test injects failure, verifies manifest | `L1-FR-ERRORS.13`-`L1-FR-ERRORS.42`, `L1-FR-IPC.3` |
| 10 | Skill linting CI gate validates directory structure, metadata, schemas, trigger boundaries | Automated: `skill_lint.rs` test | `L1-FR-SKILLS.1`, `L1-FR-SKILLS.4` |
| 11 | Smoke canary harness executes: page fingerprint check + required auth-state verification when applicable + minimal extraction per source | Automated: `skill_canary_harness.rs` test | `L1-FR-SKILLS.2`, `L1-FR-ADAPTER.4`, `L1-FR-ERRORS.31`, `L1-FR-ERRORS.35` |
| 12 | Experiment 4 passes: 10 consecutive batch runs, >=90% success rate, 0 orphaned processes | Semi-automated: harness runs 10 batches, checks orphans | `L1-FR-DAEMON.1`, `L1-FR-REAPER.1`-`L1-FR-REAPER.5`, `L1-FR-PIPELINE.14`-`L1-FR-PIPELINE.15` |
| 13 | `stage_run_id` correctly scoped per business unit grain for all live-pipeline stage commits | Automated: test verifies stage_run_id uniqueness per scope grain | `L1-FR-RUNNER.4`-`L1-FR-RUNNER.9`, `L1-FR-PIPELINE.16`-`L1-FR-PIPELINE.21` |

### 5.2 Test File Listing

10 new Rust test files for Layer 1 (cumulative: 14 including L0's 4 TS test files):

Coverage note: DoD 1 is enforced by CI static analysis rather than a Rust test file. The 10 files below cover DoD 2-13.

| Test File | Coverage | Primary Requirements | DoD Coverage |
|-----------|----------|----------------------|--------------|
| `agent_pty_session.rs` | PTY session creation, output capture, full lifecycle (created -> spawned -> active -> terminating -> exited -> reaped -> archived), cleanup verdict recording, pgid verification, compressed artifact storage, crash-safe chunk/index recovery | `L1-FR-PTY.1`-`L1-FR-PTY.4`, `L1-FR-REAPER.1`-`L1-FR-REAPER.5`, `L1-FR-DAEMON.4`-`L1-FR-DAEMON.12`, `L1-FR-ARTIFACTS.1`-`L1-FR-ARTIFACTS.5` | 2, 6, 7 |
| `agent_contract_session.rs` | Contract-mode session, structured JSON/JSONL event capture, result contract validation against `result.schema.json`, checkpoint sequencing, partial-failure classification, durable-write failure handling | `L1-FR-ADAPTER.1`-`L1-FR-ADAPTER.18`, `L1-FR-PIPELINE.9`, `L1-FR-PIPELINE.11`, `L1-FR-RUNNER.3`, `L1-FR-ERRORS.24`-`L1-FR-ERRORS.28`, `L1-FR-ERRORS.41` | 2, 8, 9 |
| `agent_procurement_ingestion.rs` | Live FTS batch ingestion and end-to-end pipeline execution through manifest generation, including `stage_run_id` scoping, reserved-column defaults, and Experiment 4 batch-harness assertions | `L1-FR-PIPELINE.2`-`L1-FR-PIPELINE.22`, `L1-FR-SKILLS.2`, `L1-FR-SKILLS.3`, `L1-FR-RUNNER.4`-`L1-FR-RUNNER.9`, `L1-FR-DB.11` | 3, 4, 5, 12, 13 |
| `adapter_codex.rs` | Codex adapter capability probe, JSONL parsing, mode selection, error classification on probe failure | `L1-FR-ADAPTER.4`-`L1-FR-ADAPTER.18`, `L1-FR-ERRORS.27`, `L1-FR-ERRORS.31` | 2, 9 |
| `adapter_claude.rs` | Claude adapter capability probe, JSON/stream-JSON parsing, mode selection, error classification | `L1-FR-ADAPTER.4`-`L1-FR-ADAPTER.18`, `L1-FR-ERRORS.27`, `L1-FR-ERRORS.31` | 2, 9 |
| `adapter_result_contract.rs` | Result contract validation, checkpoint sequencing, partial-failure classification, degraded vs failed outcome distinction | `L1-FR-PIPELINE.9`, `L1-FR-PIPELINE.11`, `L1-FR-ARTIFACTS.3`, `L1-FR-ERRORS.24`-`L1-FR-ERRORS.26` | 8, 9, 13 |
| `skill_lint.rs` | Skill directory structure validation: required files, metadata, schemas, canary inputs, golden assertions, trigger boundaries, resume strategy, manifest instructions, invalid-directory classification | `L1-FR-SKILLS.1`, `L1-FR-SKILLS.4`, `L1-FR-PROMPT.1`, `L1-FR-ERRORS.40` | 10 |
| `skill_canary_harness.rs` | Live smoke canary execution: page fingerprint, auth verification, minimal extraction, protocol/version drift detection | `L1-FR-SKILLS.2`, `L1-FR-ADAPTER.4`, `L1-FR-ERRORS.31`, `L1-FR-ERRORS.35` | 11 |
| `session_reaper.rs` | Session Reaper correctness: SIGCHLD handling, waitpid drain, orphan detection, cleanup verdict assignment, cleanup_report.json emission, daemon-crash recovery | `L1-FR-REAPER.1`-`L1-FR-REAPER.5`, `L1-FR-ERRORS.29`, `L1-FR-ERRORS.30`, `L1-FR-IPC.4` | 6, 9, 12 |
| `ipc_handshake.rs` | Daemon↔desktop IPC: descriptor file write/read, lock-file exclusivity, handshake protocol, stale socket detection, peer credential verification, explicit startup-failure non-readiness | `L1-FR-IPC.1`-`L1-FR-IPC.7`, `L1-FR-DB.2`, `L1-FR-DB.4`, `L1-FR-ERRORS.42` | 5, 9 |

### 5.3 Test Scenarios

#### Edge Case Scenarios

| Scenario | Expected Behavior | Related Requirements |
|----------|------------------|----------------------|
| Agent crashes mid-session (SIGKILL) | Cleanup verdict = `forced`, run manifest has machine-authored fallback narrative, error manifest created, pgid group killed, no orphaned processes | `L1-FR-REAPER.2`, `L1-FR-REAPER.4`, `L1-FR-ARTIFACTS.8`, `L1-FR-ERRORS.26`, `L1-FR-ERRORS.39` |
| Orphaned grandchild process | Detected by pgid sweep after session exit, reaped by process supervisor, logged as cleanup warning | `L1-FR-PTY.2`, `L1-FR-REAPER.3`, `L1-FR-REAPER.4` |
| PTY EOF received unexpectedly | Session transitions to `terminating`, cleanup runs, verdict = `clean` if output was fully captured, `forced` if mid-extraction | `L1-FR-DAEMON.7`, `L1-FR-DAEMON.8`, `L1-FR-REAPER.2` |
| Agent CLI not found on PATH | Capability probe fails gracefully, adapter reports `AGENT_INTERNAL_ERROR`, job marked failed with clear error | `L1-FR-ADAPTER.11`, `L1-FR-ERRORS.27`, `L1-FR-ERRORS.39` |
| Agent exits immediately after spawn before meaningful work | Daemon records a zero-progress `active` phase before `terminating`/`exited`, so the public lifecycle never skips from `spawned` to `exited` | `L1-FR-DAEMON.12` |
| Agent produces partial output then stalls | Stall detection triggers after timeout, session force-killed, partial checkpoints preserved, verdict = `forced` | `L1-FR-DAEMON.6`, `L1-FR-ERRORS.21`, `L1-FR-ERRORS.26` |
| FTS API returns empty results (no new notices) | Normal completion with zero records, no error manifest, cursor not advanced if no data | `L1-FR-PIPELINE.7`, `L1-FR-SKILLS.2`, `L1-FR-DB.8` |
| FTS API returns rate limit or error | Source-specific error classified, error manifest created with HTTP context, job retryable or cooled down per source policy | `L1-FR-ERRORS.32`, `L1-FR-ERRORS.35`, `L1-FR-ERRORS.39` |
| Skill `result.schema.json` validation fails on output | Result contract validator rejects output, `AGENT_OUTPUT_CONTRACT_INVALID` error, error manifest with validation details | `L1-FR-PIPELINE.11`, `L1-FR-ERRORS.24`, `L1-FR-ERRORS.39` |
| Multiple candidates in single notice | Each stage runs at correct scope grain: lot extraction per-notice (set-replacement), product extraction per-lot (additive), pricing per-candidate-source-pair (additive) | `L1-FR-PIPELINE.18`, `L1-FR-PIPELINE.19`, `L1-FR-RUNNER.5` |
| `stage_run_id` collision | UUID-based, should never collide. Test verifies uniqueness across concurrent stage commits for distinct scope units | `L1-FR-PIPELINE.16`, `L1-FR-RUNNER.5` |
| Child exits before PTY master reports hangup | Daemon handles race correctly: waits for both child exit AND PTY drain before advancing to `reaped` | `L1-FR-DAEMON.8`, `L1-FR-DAEMON.9`, `L1-FR-REAPER.1` |
| PTY master gets `EIO` before child exit is reaped | `EIO` treated as expected hangup; session waits for `waitpid` before cleanup | `L1-FR-DAEMON.8`, `L1-FR-REAPER.2` |
| Master fd accidentally inherited by grandchild, preventing EOF | `CLOEXEC` hygiene prevents this; test verifies master fd not leaked to descendants | `L1-FR-PTY.2` |
| Daemon crashes while PTY session is active | On restart: detect `spawned\|active\|terminating` sessions, kill surviving descendants, synthesize `AGENT_DAEMON_CRASH_DURING_SESSION` error manifest; PTY stream NOT resumed | `L1-FR-IPC.4`, `L1-FR-ERRORS.30`, `L1-FR-REAPER.3` |
| Desktop handshake sees schema epoch mismatch | Daemon returns an explicit schema-epoch mismatch response containing expected and actual epochs; the client must not proceed to commands or stream use | `L1-FR-IPC.3`, `L1-FR-DB.4` |
| Skill directory is missing required files or metadata | CI/runtime binding fails with `SKILL_DIRECTORY_INVALID`; no agent session is spawned from the malformed skill | `L1-FR-SKILLS.4`, `L1-FR-ERRORS.40` |
| SQLite commit fails during stage-output persistence | No `visibleAfterSeq` is returned, no stage head advances, and `SQLITE_WRITE_FAILED` is surfaced explicitly | `L1-FR-RUNNER.1`, `L1-FR-RUNNER.2`, `L1-FR-ERRORS.41` |
| Lock/migration/readiness step fails during daemon startup | Daemon does not publish `ready: true` in `daemon.info.json` and surfaces `DAEMON_STARTUP_FAILED` | `L1-FR-DB.4`, `L1-FR-ERRORS.42`, `L1-FR-IPC.1` |
| Lost `MutationAck` after DB commit, retried with same idempotency key | Idempotent: returns same ack, no duplicate writes | `L1-FR-RUNNER.2`, `L1-FR-DB.2` |
| Auth challenge requiring PTY input | Layer 1 classifies `AGENT_AUTH_CHALLENGE`, preserves session evidence, and stops awaiting human action; interactive PTY input remains a future attached-session surface activated in Layer 2 | `L1-FR-ADAPTER.2`, `L1-FR-ERRORS.22`, `L1-FR-IPC.5` |

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
