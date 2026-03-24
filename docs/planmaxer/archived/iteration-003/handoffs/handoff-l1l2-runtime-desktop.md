# Handoff: L1+L2 Daemon Runtime & Desktop Experience

## Expert Framing

You are a **systems programmer and desktop application architect** with deep experience building Rust daemons with process supervision, PTY session management, and Tauri v2 desktop shells. You've built systems where a long-running daemon manages child processes (agents) via PTY terminals, streams their output to a desktop UI, and handles cleanup when things go wrong. You understand the tension between "let the agent do its thing" and "the daemon must know what happened."

Draw from: Rust async runtime (tokio) patterns, PTY management on macOS/Linux (openpty, SIGCHLD, process groups, EIO handling), Tauri v2 architecture (invoke commands, channels, plugins), IPC patterns (UDS/named-pipe), external store patterns for React (useSyncExternalStore), and desktop-daemon separation in production systems (VS Code + language server, Docker Desktop + dockerd).

---

## Mission

ULTRATHINK and be meticulous. You are reviewing **Layers 1 and 2** of a 7-layer strategy for a UK public procurement intelligence platform.

**Layer 1 (Thin Local Runner)** introduces:
- Rust daemon with PTY session management and process-group supervision
- Agent adapter trait (unified event stream for PTY and contract-mode execution)
- Job Runner with stage tracking (stage_run_id, process_stage_heads)
- Artifact store for compressed immutable session output
- Error manifest system
- Hybrid-authored run manifests (agent narrative + daemon evidence appendix)
- 7-phase session lifecycle (created → spawned → active → terminating → exited → reaped → archived)
- 14 core Rust files in `crates/daemon`

**Layer 2 (Single-Operator Desktop MVP)** introduces:
- Tauri v2 desktop shell with React SPA
- QueryService (read-only SQLite queries) and DaemonBridge (UDS/named-pipe RPC to daemon)
- Channel<T> live streaming for PTY/stdout/stderr
- Bootstrap state machine (healthy/browse-only/recovery-required)
- Shared UI components (packages/ui)
- Manifest rendering (packages/manifests)
- Operator decision capture with staleness tracking
- Coverage metrics display

Don't just answer my questions — **the questions I DON'T KNOW to ask are more important than the ones I do.** This is where the runtime either works reliably or creates cascading failures.

---

## User's Strong Beliefs (Engage With These)

1. **"The daemon MUST be separate from the Tauri desktop shell."** Not embedded, not a Tauri plugin — a separate process that the desktop communicates with via IPC. The user believes this is essential for reliability (daemon survives desktop restart) and architecture clarity (daemon is the authority, desktop is a client). **Is this separation correct? What are the operational costs?**

2. **"PTY execution from day 1 is non-negotiable."** Browser automation requires full terminal emulation. The user chose PTY as a first-class runtime mode alongside structured contract-mode, with an adapter boundary between them. **Is the adapter abstraction right? Where does it leak?**

3. **"The desktop is a control panel, not a browser."** Full read/write/trigger capability. Not a thin client. The operator should be able to do everything from the desktop without touching a terminal. **Does the L2 architecture deliver this promise?**

---

## Essential Context

### What This Project Is

A UK public procurement intelligence platform where a Rust daemon orchestrates AI agent sessions via PTY terminals. Agents follow "skills" (versioned playbooks) to scrape procurement portals and marketplaces. The daemon manages job scheduling, session lifecycle, artifact capture, and local SQLite storage. A Tauri v2 desktop provides the operator interface.

### Key Terms

- **PTY mode:** Agent runs in a pseudoterminal. Full terminal emulation. Raw bytes captured as forensic artifacts. Used for browser-automation skills.
- **Contract mode:** Agent runs as a subprocess with structured stdin/stdout JSON. No terminal. Used for deterministic tasks.
- **Adapter boundary:** A Rust trait (`AgentAdapter`) that provides a unified event stream regardless of execution mode. The daemon interacts with adapters, not raw PTY/process IO.
- **Session lifecycle:** 7 phases: created → spawned → active → terminating → exited → reaped → archived. Each has explicit entry/exit conditions.
- **Cleanup verdict:** How a session ended: `clean` (normal exit), `forced` (SIGTERM/SIGKILL required), `leaked` (orphan processes detected after cleanup).
- **Run manifest:** Hybrid-authored artifact — agent writes narrative sections (steps, decisions, evidence), daemon appends machine-observed facts (URL chain, fingerprint, checkpoint count). Field naming: `agent_claimed_*` vs `daemon_observed_*`.
- **Artifact store:** Compressed immutable files at `artifacts/sessions/YYYY/MM/<session_id>/`. PTY bytes, stdout/stderr, run manifests. NOT inline in SQLite.
- **QueryEnvelope<T>:** Desktop queries are read-only SQLite snapshots with metadata (snapshotSeq, sourceMeta).
- **DaemonBridge:** Desktop → daemon RPC via UDS (Unix Domain Socket) or named pipe. Returns MutationAck.
- **Channel<T>:** Tauri streaming channel for live session output. SessionFrame types: lifecycle, pty, stdout, stderr, event, gap.

### Technical Constraints

- Rust 2024 edition (nightly), `#![forbid(unsafe_code)]`
- Tauri v2 (NOT v1 — different architecture, no server-style Next.js)
- React + Vite SPA inside Tauri webview
- SQLite WAL mode (read-only from desktop via QueryService)
- Bun for TypeScript layers
- Desktop and daemon are separate OS processes
- No cloud/network dependency for local operation

---

## Current Architecture

### L1: Daemon Runtime (14 Rust files)

```
crates/daemon/src/
├── main.rs              — entry point, signal handlers, graceful shutdown
├── config.rs            — daemon configuration, paths, limits
├── db/
│   ├── schema.rs        — SQLite DDL, schema_epoch check
│   └── queries.rs       — typed query functions
├── jobs/
│   ├── runner.rs        — Job Runner: dequeue → spawn → supervise → reap
│   └── stage_tracker.rs — stage_run_id management, head advancement
├── adapters/
│   ├── trait.rs         — AgentAdapter trait (unified event stream)
│   ├── codex.rs         — Codex/GPT adapter (contract mode)
│   └── claude.rs        — Claude adapter (PTY mode)
├── sessions/
│   ├── lifecycle.rs     — 7-phase state machine
│   ├── pty.rs           — PTY creation, capture, health monitoring
│   └── reaper.rs        — Process-group cleanup, verdict assignment
├── artifacts/
│   └── store.rs         — Compressed artifact storage
└── errors/
    └── manifest.rs      — Error manifest generation
```

**Session lifecycle detail:**
1. **Created:** Job dequeued, adapter selected, resources allocated
2. **Spawned:** Child process created (PTY via openpty or subprocess), pid/pgid recorded
3. **Active:** Bytes flowing, checkpoints arriving, health checks passing
4. **Terminating:** Graceful shutdown requested (SIGTERM to process group)
5. **Exited:** Process exited (any code), exit status recorded
6. **Reaped:** All descendant processes confirmed dead, cleanup verdict assigned
7. **Archived:** Artifacts compressed and stored, session row finalized

**Process-group supervision:**
- Every session creates a new process group (setpgid)
- Cleanup escalation: SIGTERM → wait 5s → SIGKILL to pgid → orphan sweep
- Cleanup verdicts: clean / forced / leaked
- Leaked sessions trigger error manifest

**Run manifest generation (hybrid-authored):**
- Agent writes: Summary, Steps Taken, Decisions Made, Warnings, Evidence claims
- Daemon appends: observed URL chain, fingerprint/locale/currency state, checkpoint count, result item count, OQG anomaly summary, representative item sample
- If agent fails to emit narrative → daemon synthesizes fallback machine-authored manifest
- Stored at `artifacts/sessions/YYYY/MM/<session_id>/run-manifest.md`
- Path + SHA-256 hash recorded in `agent_session_logs`

### L2: Desktop Architecture (14 desktop files + 7 shared package files)

```
apps/desktop/
├── src-tauri/
│   ├── main.rs          — Tauri bootstrap, health check, state machine
│   ├── query_service.rs — Read-only SQLite (QueryEnvelope<T>)
│   └── daemon_bridge.rs — UDS/named-pipe RPC to daemon (MutationAck)
├── src/
│   ├── App.tsx           — Root, routing, global providers
│   ├── pages/
│   │   ├── ContractList.tsx    — Keyset pagination, coverage badges
│   │   ├── ContractDetail.tsx  — Full pipeline view, margin bands, candidates
│   │   ├── Sessions.tsx        — Active/completed sessions, cleanup verdicts
│   │   ├── SessionViewer.tsx   — Live PTY output streaming
│   │   └── Errors.tsx          — Error manifests, actionable context
│   └── stores/
│       ├── SessionRegistry.ts  — External store for session state
│       └── SessionBuffer.ts    — Ring buffer for PTY bytes

packages/ui/src/
├── ContractCardFrame.tsx  — Frame+slot composition for process summaries
├── ManifestViewer.tsx     — Markdown renderer with section navigation
├── PipelineBadge.tsx      — Pipeline state indicator
├── SyncAgeBadge.tsx       — Freshness indicator (stubs in L2)
└── ErrorSummaryList.tsx   — Error list with severity

packages/manifests/src/
├── render-opportunity.ts  — Deterministic pipeline → Markdown renderer
└── opportunity.md.ts      — Template: summary, lots, candidates, pricing, margins
```

**Desktop-Daemon Protocol:**

| Surface | Transport | Pattern | Data |
|---------|-----------|---------|------|
| Read queries | Tauri `invoke()` | Request-response | QueryEnvelope<T> |
| Mutations | DaemonBridge (UDS) | Request-response | MutationAck |
| Live streaming | Tauri Channel<T> | Push stream | SessionFrame variants |

**Bootstrap state machine:**
- **Healthy:** Daemon running, SQLite accessible, recent health check
- **Browse-only:** SQLite accessible but daemon unreachable. Read-only queries work, mutations disabled.
- **Recovery-required:** SQLite inaccessible or corrupt. Show recovery UI.

**Coverage metrics (operator-facing):**
- `pricedCoverageBps` — % of mandatory lots with any price signal
- `netQualifiedCoverageBps` — % with margin estimate
- Color: Green >=70%, Amber 30-69%, Red <30%
- Auto-qualification guard: >=50% coverage required

**Decision capture:**
- `pending_operator_intents` table (local-only in L2; remote CAS in L4)
- `decisionBasisHash` captures pipeline state at decision time
- Mismatch → stale warning to operator

**Keyset pagination:**
- Cursor-based, not offset
- Stable across live updates (no row jumping)

---

## Tensions to Resolve

### Tension 1: Daemon-Desktop Separation Complexity

The daemon and desktop are separate OS processes communicating via IPC. This provides isolation but adds significant complexity:

- **Discovery:** How does the desktop find the daemon? PID file? Well-known socket path? What if multiple daemons run?
- **Startup ordering:** Does the desktop launch the daemon? Or must the user start the daemon first? What if the daemon crashes mid-session?
- **State synchronization:** QueryService reads SQLite directly (shared file). DaemonBridge talks to daemon via IPC. These are two independent data paths. What if they disagree?
- **Stress test:** User opens desktop. Daemon is running a long scraping session. Desktop shows live PTY output via Channel<T>. User closes desktop. User reopens desktop. Does the streaming reconnect? Does the session survive? What about the SQLite read lock during a desktop restart?

### Tension 2: PTY Session Management Robustness

PTY sessions are inherently messy. The doc describes 7 lifecycle phases and cleanup verdicts, but:

- **EIO handling:** When the slave side of a PTY closes, the master gets EIO. This is expected. But the timing is racy — the daemon might try to read PTY bytes after the child exits but before EIO is delivered. How is this handled?
- **Terminal geometry:** Some agents (browser automation) may care about terminal size. But the "terminal" is a PTY with no real display. What geometry does the daemon set? What happens when an agent tries to render a TUI element?
- **Zombie processes:** Process-group kill (kill -pgid) should catch everything. But what about processes that changed their process group? Or background processes spawned by the agent that daemonize themselves?
- **macOS vs Linux:** PTY behavior differs subtly between platforms (macOS uses /dev/ptmx differently, signal handling differs). The doc doesn't mention platform-specific handling.
- **Stress test:** Agent spawns a headless Chrome instance. Chrome forks multiple processes. Agent gets SIGTERM. Chrome's subprocess group is different from the agent's. Does the cleanup escalation catch Chrome?

### Tension 3: Adapter Abstraction Leakiness

The AgentAdapter trait provides a unified event stream for PTY and contract modes. But:

- **PTY produces raw bytes; contract mode produces structured JSON.** Unifying these into one trait means either: (a) the trait is so abstract it's useless, or (b) it leaks implementation details.
- **Health monitoring differs:** PTY health = "bytes flowing, stall detection." Contract health = "heartbeat messages arriving." The adapter must abstract both, but the monitoring strategies are fundamentally different.
- **Error semantics differ:** PTY errors are "the terminal died" (EIO, SIGPIPE). Contract errors are "the JSON was malformed" or "the process exited non-zero." These need different handling.
- **What the trait actually looks like:** The doc specifies the trait must handle mode selection via `interaction_profile`, emit a unified event stream, and support checkpoint commits. But the concrete trait signature isn't provided.

### Tension 4: Desktop as Control Panel vs. Read-Only Queries

QueryService reads SQLite directly (read-only). But the operator needs to trigger mutations (start jobs, capture decisions). Mutations go through DaemonBridge → daemon → SQLite. This means:

- **Read-your-writes problem:** Operator clicks "Start Ingestion." DaemonBridge confirms. But QueryService reads SQLite directly and might not see the new job row yet (WAL mode, different connection). How long until the UI reflects the change?
- **Consistency model:** The desktop has two data paths with different latency characteristics. This is a source of confusing UX (mutation succeeded but list doesn't update).
- **Live streaming + historical data:** SessionViewer shows live PTY output via Channel<T>. But Sessions page shows completed sessions from SQLite. The transition from "live" to "historical" (when a session completes) requires coordinating two data paths.

### Tension 5: Shared UI Components Between Desktop and Web

L2 introduces `packages/ui` with 5 components shared between desktop (L2) and web (L4). The rule is: "sharing boundary stops at view-models and presentational components. Data-fetching is surface-specific."

- **Is this boundary maintainable?** The same `ContractCardFrame` component renders process data. In desktop, data comes from QueryService (SQLite). In web (L4), data comes from Supabase client. The component must be truly data-agnostic.
- **Does the frame+slot composition pattern work?** The doc proposes `ContractCardFrame` with slots for badges, metrics, actions. This is a compositional pattern. But if the frame needs to know about data freshness (e.g., to show SyncAgeBadge), the abstraction leaks.
- **Testing shared components:** How do you test components that must work with two different data-fetching backends?

---

## What I Don't Want (Anti-Patterns)

- **Don't propose embedding the daemon in Tauri.** The separation is decided (ADR-003). The question is how to make it work well, not whether to do it.
- **Don't suggest replacing PTY with a simpler execution model.** PTY is non-negotiable for browser automation (ADR-004).
- **Don't propose Electron instead of Tauri.** The stack choice is made.
- **Don't recommend TanStack Query for streaming state.** The doc explicitly uses external stores (useSyncExternalStore) for PTY streaming. The question is whether this pattern is sufficient, not whether to replace it.
- **Don't suggest adding a web server inside the daemon.** The daemon communicates via UDS/named-pipe, not HTTP.

---

## What I Do Want

1. **Daemon-desktop protocol review:** Is UDS/named-pipe the right IPC? What about the discovery/startup/reconnection story? Propose a concrete protocol if the current one is underspecified.

2. **PTY session management audit:** Where are the platform-specific gotchas? What about EIO races, zombie processes, terminal geometry? Propose hardening measures the doc is missing.

3. **Adapter trait design review:** What should the AgentAdapter trait signature actually look like? How do you unify PTY and contract mode without leaking abstractions? Propose a concrete trait design.

4. **Read-your-writes solution:** How should the desktop handle the latency between DaemonBridge mutations and QueryService reads? Is there a coordination mechanism that doesn't add excessive complexity?

5. **Bootstrap state machine validation:** Are 3 states (healthy/browse-only/recovery) sufficient? What about "daemon running but unhealthy" or "daemon running but overloaded"?

6. **Ideas I haven't asked about.** What operational concerns am I missing for a desktop+daemon architecture?

Output your review as structured analysis with specific, actionable recommendations. Reference specific file names and component names. Where you propose changes, describe the concrete modification.

---

## Context Docs to Attach

1. **`docs/strategy/layers/layer-1-thin-local-runner.md`** (~1300 lines) — L1 daemon runtime. Read cover to cover.
2. **`docs/strategy/layers/layer-2-single-operator-desktop.md`** (~700 lines) — L2 desktop architecture. Read cover to cover.
3. **`docs/strategy/layer-strategy.md`** (~2100 lines) — Cross-layer contracts, interface definitions. Read §10 (Interface Contracts) carefully.

# Layer Strategy — UK Public Procurement Intelligence Platform

> **Authority:** This document is the authoritative cross-layer strategy for the sequentially-shippable layer decomposition of the procurement intelligence platform. It supersedes ad-hoc milestone references elsewhere.
>
> **Source:** Derived from the 8,400-line research document (`docs/research/2026-03-08-public-procurement-intelligence-platform-research.md`) through structured investigation of all major subsystems, dependency mapping, and synthesis.
>
> **Layers:** 7 layers (0–6), each independently deployable, testable, and incrementally valuable.

---

## Glossary

Terms used throughout this document:

| Term | Definition |
|------|-----------|
| **PTY** | Pseudo-terminal — a Unix facility that emulates a terminal, enabling the daemon to run browser-automation agents as if they were typing in a real terminal. Required for interactive flows (login, CAPTCHAs, multi-step forms). |
| **Contract mode** | Non-interactive structured execution (JSON/JSONL I/O) — used for deterministic tasks that don't need a browser. Lighter than PTY. |
| **Push-owned** | A table whose authoritative copy lives in local SQLite, written by the daemon. A projection is pushed to Supabase for web viewing. The daemon is the authority; Supabase is a read-only mirror. |
| **Remote-owned** | A table whose authoritative copy lives in Supabase (e.g., operator decisions, shared config). The daemon pulls a local cache. |
| **CAS** | Compare-And-Swap — a concurrency pattern where an update includes the expected current version. If the version doesn't match (someone else changed it), the update fails rather than silently overwriting. Used for operator decisions. |
| **OQG** | Output Quality Gate — code-based checks (schema validation, statistical anomaly detection, drift detection) that run on every skill result before the semantic validator. |
| **RLS** | Row-Level Security — Postgres/Supabase feature that restricts which rows each database role can read/write. |
| **UDS** | Unix Domain Socket — local inter-process communication transport used between the desktop app and the daemon. |
| **FTS** | Find a Tender Service — UK government procurement notice portal. |
| **CF** | Contracts Finder — UK government procurement notice portal for lower-value contracts. |
| **PCS** | Public Contracts Scotland — Scottish procurement portal. |
| **OCDS** | Open Contracting Data Standard — structured format for procurement data used by UK government portals. |
| **`stage_run_id`** | An identifier for one atomic commit cohort within a pipeline stage. All outputs from one execution of one stage at one scope grain share a `stage_run_id`. Used for sync batch boundaries and rollback. |
| **`matchArchetype`** | Classification of how a product candidate's identity is recovered on marketplaces (e.g., `standard_class_led` for products found via standards, `dimension_material_led` for products found via physical specs, `model_exact` for branded products found by model number). Drives query strategy in marketplace search. |
| **Skill** | A versioned directory containing a Markdown playbook (SKILL.md), schemas, and test fixtures that tells an AI agent how to perform a specific task (e.g., scrape a procurement portal, extract products from tender documents). Skills are passive assets — the daemon orchestrates when and how they run. |
| **QueryService** | A Tauri-side Rust component that reads local SQLite directly (read-only) and returns typed `QueryEnvelope<T>` responses to the desktop frontend. No network dependency. |
| **DaemonBridge** | A Tauri-side Rust component that forwards desktop commands (mutations, triggers) to the background daemon process over UDS/named-pipe. Returns `MutationAck` responses. |
| **Channel\<T\>** | Tauri's typed streaming mechanism — used for live session output from the daemon to the desktop frontend (e.g., real-time terminal output from a running agent). |
| **Run manifest** | A structured Markdown document produced after every skill execution. Contains an agent-written narrative ("what I did") and a daemon-appended evidence appendix ("what the machine observed"). Used for debugging, operator review, and semantic validation. |

---

## Table of Contents

1. [Design Rationale](#1-design-rationale)
2. [Layer 0 — Gold Set + Evaluation Harness](#2-layer-0--gold-set--evaluation-harness)
3. [Layer 1 — Thin Local Runner](#3-layer-1--thin-local-runner)
4. [Layer 2 — Single-Operator Desktop MVP](#4-layer-2--single-operator-desktop-mvp)
5. [Layer 3 — Operational Hardening](#5-layer-3--operational-hardening)
6. [Layer 4 — Shared Visibility](#6-layer-4--shared-visibility)
7. [Layer 5 — Expanded Commercial Intelligence](#7-layer-5--expanded-commercial-intelligence)
8. [Layer 6 — Request Pipeline Automation](#8-layer-6--request-pipeline-automation)
9. [Cross-Layer: Schema Stability Matrix](#9-cross-layer-schema-stability-matrix)
10. [Cross-Layer: Interface Contracts](#10-cross-layer-interface-contracts)
11. [Cross-Layer: Testing Evolution](#11-cross-layer-testing-evolution)
12. [Cross-Layer: Milestone Mapping](#12-cross-layer-milestone-mapping)
13. [Cross-Layer: Stack Sequencing](#13-cross-layer-stack-sequencing)

---

## 1. Design Rationale

### Why These Boundaries

The layer decomposition follows three ordering principles from the research document's validation-first philosophy:

1. **Risk retirement, not architectural completeness.** The riskiest assumptions live in the intelligence pipeline — can the system extract product candidates, match them to listings, and produce margin estimates? Infrastructure is well-understood engineering that follows, not leads.

2. **Vertical capability slices, not horizontal stack layers.** Each layer delivers an end-to-end flow, not "all Rust components" or "all TypeScript." The first pipeline layer completes the full operational cycle from notice ingestion through verifiable output.

3. **Schema from day 1, features incrementally.** Sync tracking columns (`owner_id`, `row_version`, `last_local_seq`, `stage_run_id`) are present on all push-owned tables from their first creation in Layer 0. Validation fields (`parent_job_id`, `validation_required`, `validation_verdict_id`) exist as nullable columns on `workflow_jobs` from Layer 0. All columns needed in any future layer exist as nullable from table creation. The sync ENGINE and validation ENFORCEMENT are deferred — the schema provisions are not.

### Relationship to Research Document Milestones

The 7 layers map directly to the research document's M0–M6 milestones (§11). Deviations are minimal and justified inline. The research doc's milestone structure was validated through 3 planmaxer expert review iterations and 2 harmonymaxer feedback integrations, making it a reliable starting hypothesis that required no major reordering.

### Implementation Order Snapshot

If an implementation team starts tomorrow, the build order is:

1. **Layer 0 first**: establish the gold set, evaluation harness, screening-grade pipeline primitives, and schema skeleton.
2. **Layer 1 second**: make the first live FTS loop real under the thin runner and prove `ingest → process → verifiable local output`.
3. **Layer 2 third**: add the single-operator desktop surface and Contracts Finder so a real operator can use the system daily.
4. **Layer 3 fourth**: automate and harden what Layer 2 already proved useful.
5. **Layer 4 fifth**: activate sync, remote-owned shared state, and the read-only viewing portal.
6. **Layer 5 sixth**: deepen commercial intelligence quality, not platform breadth.
7. **Layer 6 last**: automate the request pipeline only after the intelligence stack is already trusted.

Stop/go rule between layers:

- Do not start Layer 1 until Layer 0 passes the gold-set experiments.
- Do not start Layer 2 until Layer 1 proves the first live loop and runtime reliability.
- Do not start Layer 3 until Layer 2 proves single-operator value on live data.
- Do not start Layer 4 until Layer 3 proves unattended local operation.
- Do not start Layer 6 until Layer 5 proves decision-grade commercial intelligence on validated categories.

### Non-Negotiable Constraints Respected in Every Layer

These constraints from the research document apply to ALL layers without exception:

| Constraint | Source | Enforcement |
|-----------|--------|-------------|
| Authority matrix — every table classified, no mixed writes | ADR-002, §4.7 | Schema DDL enforces classification at table creation |
| Integer pence for money (`_pence` suffix) | §5.4 | Type system + linting |
| Integer basis points for confidence/percentage (`_bps` suffix, 0–10000) | §5.4 | Type system + linting |
| Process-group supervision with cleanup verdicts | ADR-004, §4.9 | From Layer 1 (first agent execution) |
| Run manifests for every skill execution | §4.10 | From Layer 1 (fallback machine-authored if agent fails) |
| Error manifests for every pipeline failure | §5.11 | From Layer 1 |
| Skill metadata declarations (SKILL.md + schemas) | §4.10, ADR-005 | From Layer 0 (first skill authored) |
| Conservative customs defaults (MFN, residual, standard VAT) | ADR-006, §4.5 | From Layer 0 (screening-grade) |
| No real-money bids without human approval | ADR-007 | All layers |
| Sync tracking columns on push-owned tables from creation | §5.4 | Schema DDL |
| `pipeline_assessments.validation_state = 'unverified'` from introduction | §5.4 | Schema DDL |

### Maintained Doc Set

The maintained planning set is exactly **8 files**. No companion docs, ADR addenda, or side decision files outside this set are authoritative for layer decomposition.

| File | Authority |
|------|-----------|
| `docs/strategy/layer-strategy.md` | Cross-layer source: layer ordering, schema stability matrix, interface contracts, stage-scope matrix, testing evolution, maintained-doc-set policy |
| `docs/strategy/layers/layer-0-gold-set-evaluation.md` | Layer-0-local narrative, scope, acceptance criteria, risks, and open questions |
| `docs/strategy/layers/layer-1-thin-local-runner.md` | Layer-1-local narrative, scope, acceptance criteria, risks, and open questions |
| `docs/strategy/layers/layer-2-single-operator-desktop.md` | Layer-2-local narrative, scope, acceptance criteria, risks, and open questions |
| `docs/strategy/layers/layer-3-operational-hardening.md` | Layer-3-local narrative, scope, acceptance criteria, risks, and open questions |
| `docs/strategy/layers/layer-4-shared-visibility.md` | Layer-4-local narrative, scope, acceptance criteria, risks, and open questions |
| `docs/strategy/layers/layer-5-expanded-commercial-intelligence.md` | Layer-5-local narrative, scope, acceptance criteria, risks, and open questions |
| `docs/strategy/layers/layer-6-request-pipeline-automation.md` | Layer-6-local narrative, scope, acceptance criteria, risks, and open questions |

Authority split inside the maintained set is fixed:

- `docs/strategy/layer-strategy.md` owns all cross-layer material: schema stability matrix, interface contracts, stage-scope matrix, testing evolution, stack sequencing, and doc-set policy.
- Each layer doc owns only layer-local narrative, capability, components, data first activated in that layer, stubs, definition of done, risks, and open questions.
- Open questions must live in the owning layer doc, not in a separate scratch file or review artifact.
- Temporary prompt artifacts, review handoffs, and iteration notes are explicitly non-authoritative and must never become de facto companion docs.
- `docs/strategy/README.md` and `docs/strategy/cross-reference-index.md` are **supporting navigation aids**, not part of the 8 authoritative planning files. They provide reading order and topic lookup but carry no planning authority.
- All 7 per-layer docs have been authored and reviewed. This strategy file now serves as the cross-layer reference; layer-local detail lives in the layer docs.

### Performance Targets

| Metric | Target | Owning Layer | Notes |
|--------|--------|-------------|-------|
| Daemon startup | <500ms | L1 | SQLite open + schema check + cron init |
| Incremental ingestion | <5min/source | L1 | Per-source agent session |
| Desktop detail page | <2s | L2 | From warm local SQLite cache via QueryService |
| Binary size (release) | <30MB | L1 | LTO + strip enabled (see Cargo release profile) |
| Idle memory (daemon) | <100MB | L1 | No leaks on long sessions |

### Per-Layer Documentation

All layer docs are complete. Each is self-contained for implementation.

| Layer | File | Lines | Key Content |
|-------|------|-------|-------------|
| 0 | `layers/layer-0-gold-set-evaluation.md` | ~3,900 | Full DDL (43 tables), 15-stage pipeline logic, skill standard, evaluation harness |
| 1 | `layers/layer-1-thin-local-runner.md` | ~1,300 | Daemon core, PTY/contract adapters, stage tracking model, run manifests |
| 2 | `layers/layer-2-single-operator-desktop.md` | ~700 | Desktop architecture, QueryService/DaemonBridge/Channel interfaces, coverage UI |
| 3 | `layers/layer-3-operational-hardening.md` | ~900 | Worker-validator dry-run, skill promotion, source health, relation resolution |
| 4 | `layers/layer-4-shared-visibility.md` | ~800 | Sync engine, Supabase schema/RLS, web portal, validation enforcement |
| 5 | `layers/layer-5-expanded-commercial-intelligence.md` | ~600 | Full customs engine, FX/freight, multi-source marketplace |
| 6 | `layers/layer-6-request-pipeline-automation.md` | ~400 | Bid workflow, portal-specific skills, audit trail |

### Atomic Operation Pairs (Never Split Across Layers)

| Pair | First Appears | Rationale |
|------|---------------|-----------|
| Skill execution + run manifest | Layer 1 | Losing forensic data from any skill run is permanent |
| Job execution + stage status update | Layer 1 | Pipeline state machine becomes inconsistent without stage tracking |
| Local data mutation + outbox entry | Layer 4 | From sync activation, every push-owned write must create outbox entry in same SQLite transaction |
| Stage output write + `stage_run_id` + stage head advancement | Layer 1 | All outputs of a pipeline stage share a `stage_run_id`; stage head advances atomically |

---

## 2. Layer 0 — Gold Set + Evaluation Harness

### Narrative

Prove the intelligence thesis on a curated lot corpus before building any live-source runtime. This layer is evaluation infrastructure first: gold-set data, harness logic, skill authoring standards, and the screening-grade TypeScript pipeline exercised against curated inputs. No daemon, no desktop, no sync, and no remote authority. Skills run from the command line for benchmark execution only. Results land in a harness-local SQLite dataset and are verified by the automated evaluation harness and manual inspection. Layer 0 deliberately does **not** count as the first live pipeline layer; it is the measurement system that Layer 1 must satisfy.

**If we never build Layer 1:** The gold set and evaluation harness remain valuable as a benchmarking tool and intelligence pipeline prototype. The skills and TypeScript intelligence code can run standalone indefinitely.

### Capability Delivered

A curated gold-set lot corpus (50–100 lots) runs through extraction → matching → screening-grade cost estimation, producing per-lot precision/recall metrics against human-labeled ground truth. Three experiments are validated:

1. **Extraction quality** — ≥80% of lots yield usable product candidates
2. **Matching quality** — ≥70% of candidates with `matchArchetype` produce genuine matches
3. **Margin accuracy** — screening-grade margin bands agree with human expert on ≥75% of lots (±5pp)

### Components Touched

| Component | Action | Files |
|-----------|--------|-------|
| Workspace bootstrap | Create | `package.json`, `bunfig.toml`, `.env.example` |
| `packages/contracts-schema` | Create | `contracts.ts`, `jobs.ts`, `confidence.ts`, `zod.ts` |
| `packages/intelligence` | Create | 13 files (qualification, extraction, search, customs screening, margins, evaluation harness) |
| Skills | Create | 5 skills (document-acquisition, lot-extraction, product-extraction, preliminary-qualification, supplier-price-enrichment/ebay) |
| Database | Create | `0001_sqlite_state_initial.sql` — full pipeline schema including dormant tables for future layers (all columns present as nullable from creation) |

### Data Introduced

**SQLite pipeline tables (full schema — dormant tables included for forward compatibility):**

| Table | Authority | Purpose | Sync Tracking Columns |
|-------|-----------|---------|----------------------|
| `procurement_processes` | Push-owned (future) | Canonical internal record | `owner_id`, `row_version`, `last_local_seq`, `stage_run_id` present from creation |
| `contract_notices` | Push-owned (future) | Raw notice data | Same sync columns |
| `notice_lots` | Push-owned (future) | Per-lot entities | Same sync columns |
| `product_candidates` | Push-owned (future) | Extracted candidates | Same sync columns |
| `search_profiles` | Push-owned (future) | Structured search definitions | Same sync columns |
| `search_attempts` | Push-owned (future) | Per-wave search results | Same sync columns |
| `supplier_quotes` | Push-owned (future) | Normalized pricing | Same sync columns |
| `candidate_pricing_assessments` | Push-owned (future) | Gross margin results | Same sync columns |
| `pipeline_assessments` | Push-owned (future) | Process-level rollup | Same sync columns |
| `qualification_runs` | Push-owned (future) | Qualification decisions | Same sync columns |
| `landed_cost_estimates` | Push-owned (future) | Landed-cost results (screening-grade initially; all decision-grade columns present as nullable from creation) | Same sync columns |
| `bid_workflow_states` | Push-owned (future) | Bid workflow progression (dormant until Layer 6; schema present from creation) | Same sync columns |
| `manifests` | Push-owned (future) | Opportunity manifests (dormant until Layer 1 first live end-to-end local loop) | Same sync columns |
| `process_stage_status` | Push-owned (future) | Per-stage execution status records (dormant until Layer 1 activates stage tracking) | Same sync columns |
| `process_stage_heads` | Push-owned (future) | Latest successful stage run per scope (dormant until Layer 1) | Same sync columns |
| `tender_document_sets` | Push-owned (future) | Document acquisition output (dormant until Layer 1 first live-source document acquisition) | Same sync columns |

**Local-only orchestration tables (also in initial schema, dormant until later layers):**

| Table | Authority | Purpose |
|-------|-----------|---------|
| `workflow_jobs` | Local-only | Job lifecycle tracking (dormant until Layer 1). Forward-compatibility columns exist from creation: `parent_job_id` (nullable), `validation_required` (NOT NULL default `false`), `validation_verdict_id` (nullable). |
| `pending_operator_intents` | Local-only | Local decision-intent ledger/queue. Introduced early so Layer 2 can persist approve/reject/watch actions without violating the remote ownership of `operator_decision_heads`. Carries the future CAS payload (`process_id`, disposition, reviewed-basis metadata, idempotency key, status). |

**Junction tables (introduced with parent tables):**

| Table | Authority | Purpose |
|-------|-----------|---------|
| `notice_cpv_codes` | Push-owned (future) | Notice ↔ CPV code mapping |
| `product_commodity_codes` | Push-owned (future) | Product ↔ commodity code mapping |
| `lot_cpv_codes` | Push-owned (future) | Lot ↔ CPV code mapping |

**Critical schema decisions enforced at creation:**
- All monetary fields use integer pence (`_pence` suffix)
- All confidence/percentage fields use integer basis points (`_bps` suffix)
- All push-owned tables include `owner_id`, `row_version`, `last_local_seq`, `stage_run_id`, `created_at`, `updated_at`
- **All columns needed in later layers exist as nullable from table creation** — later layers populate existing columns, they never add new columns to tables introduced in earlier layers
- `pipeline_assessments.validation_state` exists from creation with default `'unverified'`
- `workflow_jobs.validation_required` exists from creation with default `false`; `parent_job_id` and `validation_verdict_id` are present and nullable from creation
- `product_candidates` requires `lotId` (not optional)
- `landed_cost_estimates` includes all decision-grade fields (duty breakdown, measure interaction results, exporter rate basis, supplementary units, valuation mode) as nullable — populated when Layer 5 activates the full customs engine
- `pipeline_assessments` includes all overlay and coverage fields (`reviewRequired`, `availabilityStatus`, `pricedCoverageBps`, `landedCostCoverageBps`, `netQualifiedCoverageBps`, `unresolvedMandatoryLots`, `decisionBasisHash`) as nullable — populated when Layer 2 activates coverage tracking

### Authority Classification

Layer 0 tables have three authority classifications, all permanent from creation:

- **Push-owned** (15 pipeline data tables + 3 junction tables): Daemon-authoritative, projected to Supabase in Layer 4.
- **Local-only** (`workflow_jobs`, `pending_operator_intents`): Never authoritative in Supabase. A redacted summary may be projected later, but the local tables themselves remain SQLite-only.
- **Remote-owned** tables are not introduced in Layer 0. Pre-sync operator decisions are captured only as local intents; the remote authoritative `operator_decision_heads` table arrives in Layer 4 and is remote-owned from introduction.

### Shared Library Additions

- `packages/contracts-schema/src/contracts.ts` — All TypeScript interfaces for pipeline entities
- `packages/contracts-schema/src/jobs.ts` — Job payload types
- `packages/contracts-schema/src/confidence.ts` — Confidence scoring types
- `packages/contracts-schema/src/zod.ts` — Runtime validation schemas

### Invariants Enforced

| Invariant | Type | Enforcement |
|-----------|------|-------------|
| Integer pence for all monetary values | Constitutional | DDL + type system |
| Integer basis points for confidence/percentage | Constitutional | DDL + type system |
| Authority matrix classification on all tables | Constitutional | DDL |
| Sync tracking columns on push-owned tables | Constitutional | DDL |
| Skill metadata declarations (SKILL.md + schemas) | Constitutional | Skill authoring guide + linting |
| Conservative customs defaults | Constitutional | Screening-grade implementation |
| `lotId` required on product candidates | Constitutional | DDL NOT NULL |
| Forward-compatibility nullable columns on `workflow_jobs` | Constitutional | DDL |

### What's Stubbed

| Feature | Stub | Full Implementation |
|---------|------|-------------------|
| Landed-cost engine | Screening-grade: HMRC Tariff API + flat freight + single FX | Layer 5: multi-source recall, 10-digit resolution, measure scanner |
| Marketplace search | Single source (eBay) | Layer 5: Alibaba/1688, Amazon |
| Skills execution | CLI invocation (`bun run skill-name`) | Layer 1: daemon-managed PTY/contract execution |
| Relation resolution | Gold set is pre-curated (no deduplication) | Layer 3: full heuristic resolution |
| Procurement sources | Gold set is pre-selected | Layer 1: first live FTS loop; Layer 2: adds Contracts Finder + desktop operator surface |
| Stage tracking | Implicit (no daemon) | Layer 1: explicit `stage_run_id` + stage heads |
| Sync | No sync — local SQLite only | Layer 4: push/pull sync engine |

### Definition of Done

1. Gold-set lot corpus contains ≥50 curated lots across whitelist categories (office supplies, cleaning products, basic PPE, hand tools, basic furniture, packaging materials) with human labels for product candidates, expected matches, and margin bands
2. Bilingual lexicon seeded for top 30–50 product families (EN/ZH noun aliases, market framing terms)
3. All 5 skills have complete SKILL.md + result.schema.json + checkpoint.schema.json + canary.inputs.jsonl + golden.assertions.yaml
4. Evaluation harness runs full gold-set corpus through extraction → matching → screening-grade cost, produces per-lot precision/recall report
5. Experiment 1 passes: ≥80% of gold-set lots yield at least one usable product candidate with correct unit, quantity, and specification
6. Experiment 2 passes: ≥70% of candidates with `matchArchetype` produce at least one genuine match (not false positive)
7. Experiment 3 passes: margin bands agree with human expert on ≥75% of gold-set lots (±5pp for screening-grade)
8. All 4 unit test suites pass (goods qualification, quoteability, tariff parsing, margin rules)
9. SQLite schema validates: all push-owned tables have sync tracking columns, all monetary fields are integer pence, all confidence fields are integer basis points
10. Each skill's SKILL.md includes instructions for the agent to write a structured narrative of what it did (a "run manifest") as part of its output — this prepares for the formal daemon-managed manifest system in Layer 1

### Test Scenarios

- **Fast inner loop:** Extraction parsers, margin math, scoring functions, data model validation — per-save, near-zero cost
- **Benchmark batch:** Full gold-set corpus through pipeline, compare to human labels — per-milestone, medium cost (~$2–10)
- Gold-set lot with 12 lots where 11 are irrelevant → system correctly identifies the 1 qualifying lot
- Mixed contract with separable goods → `addressableValuePence` reflects goods portion only
- Candidate with `matchArchetype = unknown` → system does not fabricate an archetype
- Screening-grade landed cost uses conservative defaults (MFN rate, residual duty, standard VAT)

### Dependencies

- None (greenfield)

### Forward-Compatibility Provisions

- Sync tracking columns on all tables (for Layer 4)
- Nullable validation columns on `workflow_jobs` (for Layer 3)
- `stage_run_id` column on pipeline tables (for Layer 1 stage tracking)
- Skill directory structure follows production standard (for Layer 1 skill linting)
- `SearchProfile` created for ALL candidates including below-threshold (for Layer 2 RFQ re-entry)
- `pending_operator_intents` exists from the start so local-only operator decisions can precede remote CAS without any table reclassification

### Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Gold-set too small for statistical significance | Medium | Start with 50 lots, expand per category; lot-level measurement (not notice-level) |
| Screening-grade landed cost too inaccurate for margin estimation | Medium | Conservative defaults ensure overestimation, not underestimation |
| eBay listings don't represent real supplier economics | High | eBay is first validation source only; Alibaba/1688 added in Layer 5 |

### Decisions Addressed

- ADR-001: Skills as primary data retrieval (first 5 skills authored)
- ADR-005: Skill directory standard established
- ADR-006: Internal landed-cost engine (screening-grade subset)

---

## 3. Layer 1 — Thin Local Runner

### Narrative

Minimal Rust runtime — just enough to launch skills, capture artifacts, supervise processes, and run the first live-source end-to-end local pipeline loop. PTY is available as a first-class capability, with contract mode for deterministic tasks. This remains a thin daemon layer, but it is also the first live pipeline layer: a manual CLI trigger ingests live FTS notices, runs the full local qualification/pricing/manifest loop, and emits verifiable local output without any UI or sync dependency.

**If we never build Layer 2:** The thin runner provides a reliable way to execute skills from the command line with full process supervision, artifact capture, and error manifests. It operates as a headless pipeline executor.

### Capability Delivered

10 consecutive batch runs of skills complete with ≥90% skill success rate and no orphaned processes. Skills are now daemon-managed with proper lifecycle: spawn → monitor → capture → validate → persist → reap. This layer also completes the first live operational loop:

`notice ingestion → pipeline processing → verifiable local output`

The verifiable output is a CLI-visible summary plus a local markdown opportunity manifest/file artifact for at least one live-source batch.

### Components Touched

| Component | Action | Files |
|-----------|--------|-------|
| Skills | Create | 2 (`procurement-ingestion/fts`, `manifest-generation`) |
| Rust workspace | Create | `Cargo.toml` (workspace root) |
| `crates/daemon` | Create | 14 core files (main, db/sqlite, pty/session+capture+health, jobs/runner, agents/adapter+codex+claude+artifacts+contracts, prompt/renderer, errors/manifest+markdown) |
| Database | Create | `0003_sqlite_artifacts.sql` (agent_session_logs, raw_documents, error_manifests) |

### Data Introduced

**SQLite tables (artifacts DB):**

| Table | Authority | Purpose |
|-------|-----------|---------|
| `agent_session_logs` | Local-only | Full session transcript/trace with `run_manifest_path`, `run_manifest_hash` |
| `raw_documents` | Local-only | Downloaded tender documents |
| `error_manifests` | Local-only | Full error context for pipeline failures |
| `executors` | Push-owned | Daemon instance identity (`owner_id`, `executor_id`, heartbeat) |
| `source_cursors` | Local-only | Per-source ingestion watermarks for incremental retrieval |

**Activated tables (created in Layer 0, now actively used):**

| Table | Change |
|-------|--------|
| `qualification_runs`, `product_candidates`, `search_profiles`, `search_attempts`, `supplier_quotes`, `candidate_pricing_assessments`, `landed_cost_estimates` | First **live-source** writes now active via the daemon-managed local pipeline loop |
| `tender_document_sets` | First live-source document acquisition writes now active |
| `manifests` | First live-source verifiable output now active (local markdown/file artifact + row) |
| `pipeline_assessments` | First live-source process rollups now active; `validation_state` starts as `unverified` from first write |
| `workflow_jobs` | Now actively used by Job Runner for job lifecycle; `parent_job_id`, `validation_required`, `validation_verdict_id` remain nullable |
| `process_stage_status` | Now actively written by Job Runner — records execution status per stage per scope |
| `process_stage_heads` | Now actively written — advances atomically when a stage succeeds for its full scope |

### Authority Classification

New tables are mostly **local-only** (no sync): `agent_session_logs` contains full PTY bytes and structured traces — too large and sensitive for sync; `error_manifests` contains full error context; `source_cursors` and `raw_documents` are per-daemon operational state. The exception is `executors` (push-owned) — the daemon's identity record that will be synced to Supabase in Layer 4. Redacted summaries of `agent_session_logs` and `error_manifests` will be pushed as separate projection tables in Layer 4. Source configuration is still bootstrapped locally in this layer (hardcoded/seeded runtime copy), not yet remote-owned shared config.

### Shared Library Additions

None (Rust-only layer).

### Invariants Enforced

| Invariant | Type | Enforcement |
|-----------|------|-------------|
| Process-group supervision | Constitutional | Every agent session in own pgid; cleanup verdict (clean/forced/leaked) mandatory |
| Run manifests for every skill run | Constitutional | Daemon fallback if agent fails to narrate |
| Error manifests for every failure | Constitutional | Error manifest system active |
| Session artifact capture | Constitutional | Raw bytes stored as compressed immutable artifacts |
| Adapter-mediated execution | Constitutional | PTY or contract mode per skill's `interaction_profile` |
| Stage tracking with `stage_run_id` | Constitutional | Job Runner scopes commits to correct grain |
| Scope-aware commit semantics | Constitutional | Set-replacement stages (e.g., lot extraction) must commit atomically for full scope or fail entirely; additive stages (e.g., per-candidate pricing) may commit independently per unit |

### What's Stubbed

| Feature | Stub | Full Implementation |
|---------|------|-------------------|
| Scheduler | CLI manual trigger only | Layer 3: cron scheduler |
| Resource arbiter | No concurrency limits | Layer 3: typed quotas |
| PTY hardening | Basic session creation | Layer 3: fd hygiene, EIO handling, terminal geometry |
| Job claiming/leases | Single-threaded execution | Layer 3: concurrent job claiming |
| Source registry | Bootstrapped local seed / hardcoded source definitions | Layer 3: typed registry; Layer 4: `shared_source_configs` pull |
| Procurement coverage | First live source only (FTS) | Layer 2: adds Contracts Finder; Layer 5: PCS if needed |
| Session reaper | Basic cleanup | Layer 3: full reaper with escalation |
| Tracing | Basic log capture | Layer 3: structured + forensic tracing |
| Validation | Fields exist, always `validation_required = false` | Layer 3: dry-run; Layer 4: enforcement |
| Operator-facing manifest rendering | Minimal local markdown/file artifact emitted directly by the thin runner + skill | Layer 2: shared `packages/manifests` renderer and desktop presentation |
| Desktop/operator surface | CLI summary + local file artifact only | Layer 2: desktop control panel |
| Sync | Tracking columns populated, no outbox entries | Layer 4: full sync engine |

### Definition of Done

1. `cargo check --all-targets` and `cargo clippy --all-targets -- -D warnings` pass
2. Rust binary launches skills via CLI (both PTY and contract mode)
3. Manual CLI trigger ingests a live FTS batch using source-batched skill execution
4. The first live end-to-end local loop completes for at least one notice: normalize → qualify → acquire docs → extract lots → extract products → price-signal match → gross gate → landed cost → manifest
5. Verifiable output exists without later layers: CLI summary and/or local markdown manifest artifact proves the loop completed
6. Process-group supervision verified: every session gets pgid, cleanup verdict recorded (clean/forced/leaked)
7. Artifact capture works: PTY bytes compressed and stored, session metadata in `agent_session_logs`
8. Run manifests generated: agent narrative + daemon evidence appendix; fallback machine-authored if agent fails
9. Error manifest system operational: pipeline failures produce Markdown error manifests with full context
10. Skill linting CI gate validates directory structure, metadata, schemas, trigger boundaries
11. Smoke canary harness executes: page fingerprint check + auth verification + minimal extraction per source
12. Experiment 4 passes: 10 consecutive batch runs, ≥90% success rate, 0 orphaned processes
13. `stage_run_id` correctly scoped per business unit grain for all live-pipeline stage commits

### Test Scenarios

- `agent_pty_session.rs` — PTY session creation, output capture, full lifecycle including reap and cleanup verdict
- `agent_contract_session.rs` — Contract-mode session, structured event capture, result contract validation
- `agent_procurement_ingestion.rs` — Live FTS batch produces a local end-to-end opportunity artifact/manifest without desktop or sync
- `adapter_codex.rs` — Codex adapter capability probe, JSONL parsing, mode selection
- `adapter_claude.rs` — Claude adapter capability probe, JSON parsing, mode selection
- `adapter_result_contract.rs` — Result contract validation, checkpoint sequencing, partial failure classification
- `skill_lint.rs` — Skill directory structure validation
- `skill_canary_harness.rs` — Live smoke canary execution
- Agent crashes mid-session → cleanup verdict is `forced`, run manifest has machine-authored fallback
- Orphaned grandchild process → detected and reaped by process supervisor

### Dependencies

- Layer 0 (skills, intelligence pipeline, contracts-schema, SQLite schema)

### Forward-Compatibility Provisions

- Adapter trait designed for extension (new agent CLIs)
- Error manifest format includes fields for validator reasoning (Layer 3)
- Agent session logs include `run_manifest_path` and `run_manifest_hash` (Layer 3 validation)
- Job Runner aware of `parent_job_id` (for Layer 3 validation sub-jobs)

### Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| PTY session reliability across macOS/Linux | Medium | Focus on macOS first (development platform); PTY hardening in Layer 3 |
| Agent CLI API changes break adapter | Low | Adapter abstraction isolates CLI changes; capability probe detects incompatibility |
| Process-group cleanup misses edge cases | Medium | Conservative kill escalation; leaked process detection |

### Decisions Addressed

- ADR-001: First live skill-based procurement ingestion loop
- ADR-002: Local-operational-authority half becomes live on real writes; sync engine remains deferred to Layer 4
- ADR-003: Tauri + separate Rust daemon (daemon starts here)
- ADR-004: PTY-based agent execution from day 1 (hybrid PTY + contract)
- ADR-005: Skills + explicit binding + daemon orchestration

---

## 4. Layer 2 — Single-Operator Desktop MVP

### Narrative

Prove repeatable operator value with a manual-trigger desktop sitting on top of the Layer-1 live local pipeline. This is the first user-facing layer — a single operator runs the system daily, reviews pipeline output in a desktop control panel, and makes accept/reject/watch decisions. This layer expands live procurement coverage to Contracts Finder, renders operator-facing manifests and coverage metrics, and persists single-operator decisions locally as queued intents without violating remote authority ownership.

**If we never build Layer 3:** A single operator can use the system with manual triggering for daily procurement intelligence. No cron, no automation, but the core value loop works.

### Capability Delivered

Single operator runs system daily for one week, making real accept/reject decisions. The pipeline surfaces genuine opportunities from live procurement data with screening-grade margin estimates. The desktop control panel provides list/detail views, agent session monitoring, and decision capture.

### Components Touched

| Component | Action | Files |
|-----------|--------|-------|
| Skills | Create | 1 (Contracts Finder ingestion) |
| `packages/intelligence` | Create | 1 (coverage-rollup.ts) |
| `packages/ui` | Create | 5 (index, contract-card-frame, manifest-viewer, pipeline-badge, sync-age-badge) |
| `packages/manifests` | Create | 2 (render-opportunity.ts, opportunity.md.ts template) |
| `apps/desktop` | Create | 14 files (full desktop shell) |

### Data Introduced

**Activated tables (created in prior layers, now actively used by the desktop/operator surface):**

| Table | Change |
|-------|--------|
| `tender_document_sets` | Existing live document sets are now rendered in desktop detail views with completeness surfaced to the operator |
| `manifests` | Existing Layer-1 local manifests are now rendered through the operator-facing manifest package |
| `pipeline_assessments` | Coverage metrics (`pricedCoverageBps`, `landedCostCoverageBps`, `netQualifiedCoverageBps`, `unresolvedMandatoryLots`, `decisionBasisHash`) and overlay fields (`reviewRequired`, `availabilityStatus`, `validationState`) are now surfaced to the operator |
| `product_candidates` | `candidateValueShareBps` is now surfaced for top-value selection and coverage rollups |
| `pending_operator_intents` | Now actively written as the local single-operator decision ledger/overlay queue for `approve` / `reject` / `watch`, including `reviewedBasisHash`, `reviewedManifestHash`, and `reviewedAtStageRunId` for later CAS flush |

### Authority Classification

No new remote-owned tables are introduced in this layer. The desktop writes only local data (`pending_operator_intents`) while the authoritative shared decision table remains deferred to Layer 4. This preserves the authority matrix: single-operator local mode first, remote CAS later.

### Shared Library Additions

- `packages/ui` — Frame + slot presentational components (contract cards, manifest viewer, pipeline badges, sync-age badges)
- `packages/manifests` — Opportunity manifest renderer

### Invariants Enforced

| Invariant | Type | Enforcement |
|-----------|------|-------------|
| Stage coherence | Constitutional | All stage outputs share `stage_run_id`; `process_stage_status` updated and `process_stage_heads` advanced atomically |
| Coverage metrics accuracy | Constitutional | Process-level state is rollup from candidate-level, not arbitrary |
| Decision staleness tracking | Constitutional | `reviewedBasisHash`, `reviewedManifestHash`, `reviewedAtStageRunId` captured on local decision intents from first operator use |
| Feature flags for new capabilities | Operational | Progressive enablement via flags |

### What's Stubbed

| Feature | Stub | Full Implementation |
|---------|------|-------------------|
| Scheduling | Manual trigger from desktop | Layer 3: cron scheduler |
| Relation resolution | Notices processed independently | Layer 3: merge/relate heuristics |
| Landed cost | Screening-grade (flat freight, single FX) | Layer 5: decision-grade |
| Search sources | eBay only (from Layer 0) | Layer 5: Alibaba/1688, Amazon |
| Sync to Supabase | None — local-only operation | Layer 4: full sync |
| Operator decisions | Local intent queue only — single-operator desktop overlay, no shared authority yet | Layer 4: remote-owned `operator_decision_heads` + Supabase CAS |
| Validation | Disabled (fields exist) | Layer 3: dry-run |
| Degraded-mode UX | Basic daemon-unavailable detection | Layer 4: three-state (healthy/degraded/offline) |

### Definition of Done

1. FTS ingestion skill scrapes live notices from Find a Tender
2. Contracts Finder ingestion skill scrapes live notices
3. Notice normalization produces canonical internal records
4. Goods/services classifier correctly categorizes notices (goods/services/works/mixed)
5. Desktop control panel launches with QueryService + DaemonBridge + bootstrap state machine
6. Notice list view with keyset pagination displays live pipeline output
7. Opportunity detail view shows margin bands, product candidates, supplier quotes
8. Decision capture (`reject` / `watch` / `approve`) persists locally in `pending_operator_intents` with reviewed-basis metadata and correct local overlay behavior
9. Agent session monitoring page shows live terminal output via Channel<T> streaming
10. Operator-facing coverage metrics displayed: `pricedCoverageBps`, `netQualifiedCoverageBps`
11. Decision-basis staleness indicators flag stale decisions
12. Single operator runs daily for 1 week and reports genuine opportunities surfaced

### Test Scenarios

- `agent_procurement_ingestion.rs` — Real FTS skill session, output verified
- `agent_supplier_enrichment.rs` — Real enrichment skill session, output verified
- `agent_health_detection.rs` — Stall detection, auth challenge detection
- `agent_error_manifest.rs` — Error manifest creation on failures
- `apps/desktop/e2e/contracts.spec.ts` — Desktop browsing and decision flow
- Notice with 12 lots → system processes each lot independently, displays per-lot results
- Mixed contract → goods-addressable value used for margin computation, not whole contract
- Expired notice → correctly filtered, not displayed as active opportunity
- Agent session crashes → error manifest created, operator can see failure context in desktop

### Dependencies

- Layer 0 (intelligence pipeline, skills, contracts-schema)
- Layer 1 (daemon runtime, artifact capture, process supervision)

### Forward-Compatibility Provisions

- Desktop bootstrap state machine detects daemon health modes (prep for Layer 4 degraded-mode UX)
- `packages/ui` frame + slot pattern designed for both desktop and web shells
- Coverage metrics on `pipeline_assessments` feed into Layer 4 sync projections
- `stage_run_id` on all stage outputs enables Layer 4 sync batch boundaries
- Local decision intents already carry the future CAS payload (`process_id`, disposition, reviewed-basis metadata, idempotency key), so Layer 4 can activate remote-owned decisions without any table reclassification

### Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Source website changes break FTS/CF skills | High | Error manifest flags failures; skill update cycle |
| Desktop UX reveals pipeline quality issues | Medium | Expected — this is why desktop comes before hardening |
| Single-source eBay pricing not representative | High | Known limitation; Layer 5 adds Alibaba/1688 |
| QueryService read performance on large datasets | Low | SQLite reads are fast; keyset pagination prevents full scans |

### Decisions Addressed

- ADR-003: Tauri v2 desktop shell (first desktop UI)
- ADR-008: Desktop = control panel with full read/write/trigger
- ADR-001: Live procurement source skills (FTS, Contracts Finder)

---

## 5. Layer 3 — Operational Hardening

### Narrative

Daily automation without babysitting. The daemon gains cron scheduling, batched ingestion, relation resolution, PTY hardening, resource management, and the worker-validator system in dry-run mode. The system runs unattended for 5 consecutive days. This layer transforms the single-operator tool into a production-grade pipeline.

**If we never build Layer 4:** A single operator has a fully automated, self-monitoring pipeline that runs daily, handles relation resolution, manages resources, and validates output quality via dry-run semantic checking. No multi-user visibility, but the core automation works.

### Capability Delivered

System runs unattended for 5 consecutive days with ≥95% skill success rate. Cron-scheduled ingestion, resource management, PTY hardening, source health monitoring, and dry-run worker-validator semantic validation are all operational. The skill promotion flow ensures quality gates for skill updates.

### Components Touched

| Component | Action | Files |
|-----------|--------|-------|
| `crates/daemon` | Create | 12 additions (scheduler/cron, sources/registry, jobs/claim, pty/reaper+hardening, agents/arbiter, db/checkpoints, tracing/structured+forensic, validation/policy+executor+manifest_writer) |
| Skills | Create | 10 (4 operational: auth-refresh/alibaba+1688, source-health/fts+alibaba; 6 validator-base files: SKILL.md, result.schema.json, checkpoint.schema.json, canary.inputs.jsonl, golden.assertions.yaml, validator-rubric.yaml) |
| `packages/intelligence` | Create | 1 (relations/resolve-process-relations.ts) |
| `packages/manifests` | Create | 2 (render-error-manifest.ts, error.md.ts template) |

### Data Introduced

**New tables:**

| Table | Authority | Purpose |
|-------|-----------|---------|
| `validation_verdicts` | Local-only | Full validator reasoning and evidence |
| `source_runtime_state` | Local-only | Circuit breaker state, cooldown tracking |
| `listing_evidence` | Local-only | Two-tier cache for search results + listing evidence |
| `process_relations` | Push-owned | Relation edges between processes |
| `process_aliases` | Push-owned | Redirect from retired process IDs |

**Modified tables:**

| Table | Change |
|-------|--------|
| `workflow_jobs` | `parent_job_id`, `validation_required`, `validation_verdict_id` now actively used for validation sub-jobs (still nullable — `validation_required` defaults to `false` except when policy triggers) |
| `agent_session_logs` | `run_manifest_path` and `run_manifest_hash` now actively populated by manifest writer |

### Authority Classification

| Table | Classification | Rationale |
|-------|---------------|-----------|
| `validation_verdicts` | Local-only | Full validator reasoning too large/sensitive for sync |
| `source_runtime_state` | Local-only | Per-daemon runtime state |
| `listing_evidence` | Local-only | Cache layer, not authoritative |
| `process_relations` | Push-owned | Shared visibility in Layer 4 |
| `process_aliases` | Push-owned | Stable deep links across desktop/web |

### Shared Library Additions

- `packages/intelligence/src/relations/resolve-process-relations.ts` — Relation resolution engine

### Invariants Enforced

| Invariant | Type | Enforcement |
|-----------|------|-------------|
| Explicit skill binding for cron | Constitutional | Scheduler binds skill id + version at enqueue time |
| Skill promotion flow gates | Constitutional | Draft → candidate → canary → shadow → stable → rollback |
| Fail-open validation | Constitutional | Validator failures never block pipeline; worker results advance |
| Dry-run isolation | Constitutional | Dry-run verdicts logged to scratch, no authoritative rows/outbox/stage heads advanced |
| Source circuit breaker | Operational | Repeated challenge-class failures → cooldown → only canary/auth jobs |
| Resource arbiter quotas | Operational | Browser slots, agent slots, source lanes |

### What's Stubbed

| Feature | Stub | Full Implementation |
|---------|------|-------------------|
| Validation enforcement | Dry-run only — verdicts logged, not enforced | Layer 4: enforcement mode |
| Domain-specific validators | Base validator only (generic semantic quality) | Layer 4: 4 domain overlays |
| Validator calibration | Initial calibration against gold-set manifests | Layer 4: formal calibration harness |
| Sync | Tracking columns populated, no outbox/push/pull | Layer 4: full sync |
| Multi-user | Single operator only | Layer 4: multi-user via web portal |

### Definition of Done

1. Cron scheduler runs on configured schedule with explicit skill binding
2. Batched ingestion: scheduled FTS + CF runs complete without manual intervention
3. Relation resolution engine correctly classifies notices (merged/related/possible_duplicate/separate)
4. PTY hardening: fd hygiene, EIO handling, terminal geometry stability verified
5. Host environment awareness: sleep/wake detection, catchup sweeps after resume
6. Resource arbiter enforces typed quotas (browser slots, agent slots, source lanes)
7. Skill promotion flow operational: draft → candidate → canary → shadow → stable
8. Source-health canaries and auth-refresh skills operational
9. Worker-validator dry-run: run manifests generated, base validator renders verdicts, verdicts logged without enforcement
10. Error manifest control-panel view displays failures with resolution context
11. System runs unattended for 5 consecutive days, ≥95% success rate, no manual intervention

### Test Scenarios

- `fault_process_cleanup.rs` — Orphaned grandchildren, dropped handle, cleanup timeout, leaked detection
- `fault_pty_edge_cases.rs` — PTY EOF/EIO handling, fd leak scenarios, terminal geometry
- `fault_host_events.rs` — Host sleep/monotonic gap, resource exhaustion, concurrent pressure
- `fault_partial_failure.rs` — Partial checkpoint then crash, result contract failures
- `replay_pty_sessions.rs` — Replay captured PTY byte streams against adapter parser
- `replay_contract_sessions.rs` — Replay captured sessions against adapter parser
- `cron_scheduler.rs` — Cron-based scheduling
- `job_claim.rs` — Concurrent job claiming and lease management
- `skill_golden_journeys.rs` — Invariant-based E2E per skill family
- `skill_shadow_compare.rs` — Quality metric diff between stable and candidate versions
- `skill_drift_monitor.rs` — Page fingerprint, fallback depth, fill-rate trending
- `validation_policy.rs` — All policy mode × trigger condition combinations
- `validation_executor.rs` — Validator sub-job spawning, kickoff prompt composition
- `validation_dry_run.rs` — Dry-run verdicts logged, pipeline not blocked
- `validation_golden_verdicts.rs` — Known-good/bad manifests → correct verdicts
- `resolve-process-relations.test.ts` — Relation scoring and hard blockers

### Dependencies

- Layer 0 (intelligence pipeline, gold set for validator calibration)
- Layer 1 (daemon runtime, agent adapter)
- Layer 2 (desktop shell for error manifest view, pipeline data model)

### Forward-Compatibility Provisions

- Validation policy engine ready for enforcement mode toggle (Layer 4)
- Run manifest corpus accumulates for validator calibration (Layer 4)
- `validation_verdicts` table schema includes all verdict metadata fields for Layer 4 projections
- Source circuit breaker state feeds into validation risk-stratified sampling

### Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Validator false positive rate too high for enforcement | Medium | Dry-run mode allows calibration before enforcement |
| Cron scheduling misses edge cases (timezone, DST) | Low | Catchup sweeps handle missed schedules |
| Relation resolution produces false merges | Medium | Conservative thresholds; operator can unmerge |
| Source changes break multiple skills simultaneously | Medium | Source-health canaries detect before production jobs |

### Decisions Addressed

- ADR-004: PTY hardening (full reliability)
- ADR-005: Explicit skill binding for cron (production reliability)

---

## 6. Layer 4 — Shared Visibility

### Narrative

Collaboration infrastructure. The sync engine activates, projecting pipeline data to Supabase for the read-only web viewing portal. Operator decisions flow through Supabase CAS. Worker-validator enforcement mode is enabled after dry-run calibration. Two users can now view the same opportunity pipeline — one via desktop (read-write), one via web (read-only).

**If we never build Layer 5:** Multi-user visibility works with screening-grade accuracy and single-marketplace pricing. The system is production-ready for the initial category whitelist.

### Capability Delivered

Two users view the same pipeline: desktop operator (read-write) and web viewer (read-only). Sync engine pushes pipeline data to Supabase. Operator decisions use compare-and-swap via Supabase. Validator rejections quarantine results in enforcement mode. Domain-specific validator overlays for procurement, supplier enrichment, document acquisition, and product extraction.

### Components Touched

| Component | Action | Files |
|-----------|--------|-------|
| Database | Create | 8 files (SQLite sync migration, 6 Supabase migrations, Supabase config) |
| `crates/daemon` | Create | 1 (db/sync.rs) |
| Skills | Create | 4 domain validator overlays (procurement-ingestion, supplier-enrichment, document-acquisition, product-extraction) |
| `apps/web` | Create | 7 files (full web shell) |

### Data Introduced

**SQLite sync infrastructure tables:**

| Table | Authority | Purpose |
|-------|-----------|---------|
| `local_outbox` | Local-only (SQLite sync infrastructure) | Transactional outbox entries with `content_hash` and `schema_epoch` from introduction |
| `sync_batches` | Local-only (SQLite sync infrastructure) | Batch metadata with `schema_epoch` |
| `pull_cursors` | Local-only (SQLite sync infrastructure) | Per-domain pull cursor position |
| `pending_operator_intents` | Local-only | Existing Layer-2 operator intent queue, now reconciled/flushed through remote CAS |

**Supabase projection tables (push-owned, mirroring local):**

All push-owned tables from Layers 0–3 get Supabase counterparts: `procurement_processes`, `contract_notices`, `notice_lots`, `tender_document_sets`, `qualification_runs`, `product_candidates`, `search_profiles`, `search_attempts`, `supplier_quotes`, `candidate_pricing_assessments`, `landed_cost_estimates`, `manifests`, `process_relations`, `process_aliases`, `pipeline_assessments`, `bid_workflow_states`, `process_stage_status`, `process_stage_heads`.

**Supabase summary tables (push-owned projections):**

| Table | Purpose |
|-------|---------|
| `workflow_job_summaries` | Redacted job status for web |
| `error_manifest_summaries` | Redacted error context for web |
| `validation_verdict_summaries` | Verdict status for web |
| `viewer_sync_status` | Sync freshness indicator |
| `executors` | Heartbeat summary |

**Supabase remote-owned tables:**

| Table | Purpose |
|-------|---------|
| `shared_company_profile` | Org-wide bidder profile |
| `shared_source_configs` | Source enablement and configuration |
| `operator_decision_heads` | Operator dispositions with CAS |

**Junction tables (Supabase):**

| Table | Purpose |
|-------|---------|
| `notice_cpv_codes` | Notice ↔ CPV code mapping |
| `product_commodity_codes` | Product ↔ commodity code mapping |
| `lot_cpv_codes` | Lot ↔ CPV code mapping |

### Authority Classification

Formalized and enforced at the Supabase database level via RLS:

| Role | Push projections | Push summaries | Remote config | Remote decisions |
|------|-----------------|----------------|---------------|-----------------|
| `viewer` | SELECT | SELECT | SELECT | SELECT |
| `operator` | SELECT | SELECT | SELECT | SELECT + UPDATE (CAS RPC) |
| `admin` | SELECT | SELECT | Full CRUD | Full CRUD |
| `daemon` | INSERT/UPDATE (batch RPC) | INSERT/UPDATE (batch RPC) | SELECT (pull) | SELECT (pull) |

### Shared Library Additions

- `apps/web` uses existing `packages/ui` components with web-specific data fetching hooks
- `packages/contracts-schema` types used for Supabase query typing

### Invariants Enforced

| Invariant | Type | Enforcement |
|-----------|------|-------------|
| Push batch atomicity | Constitutional | Single transactional RPC, not individual upserts |
| Outbox + data in same SQLite transaction | Constitutional | From activation, every push-owned mutation creates outbox entry atomically |
| CAS for operator decisions | Constitutional | `base_remote_revision` checked; mismatch → conflict |
| Active-writer lease | Constitutional | One daemon per org enforced at push protocol level |
| `schema_epoch` on every batch | Constitutional | Incompatible epoch → sync pauses, local operation continues |
| Batch boundaries never cross `stage_run_id` | Constitutional | Push dispatcher enforces |
| Single-in-flight batch | Constitutional | One batch at a time per push stream |
| Validation enforcement | Operational | Rejections quarantine results (after dry-run calibration) |
| RLS on all Supabase tables | Constitutional | Policy matrix enforced at database boundary |

### What's Stubbed

| Feature | Stub | Full Implementation |
|---------|------|-------------------|
| Schema epoch transitions | Initial epoch only (no transitions needed yet) | Future: expand/drain/upgrade/contract |
| Multi-org | Single org hardcoded | Future: org_id scoping |
| Full rebuild | Not implemented | Future: generation-based remote rebuild |
| Logical consistency auditor | Not implemented | Future: domain invariant checking |

### Definition of Done

1. Push protocol: transactional outbox entries created for all push-owned mutations in same SQLite transaction
2. Push batches applied to Supabase via single transactional RPC with `schema_epoch`, `row_version` ordering
3. Pull protocol: cursor-based pull for `shared_source_configs`, `shared_company_profile`, `operator_decision_heads`
4. Operator decisions: queued local intents flush through Supabase CAS RPC with `base_remote_revision`; conflicts returned (not silently overwritten) and local queue state updated
5. Active-writer lease: second daemon instance detected and rejected
6. RLS policies enforced for all 4 roles (viewer, operator, admin, daemon)
7. Web viewing portal displays opportunities, manifests, pipeline status (read-only)
8. Supabase Broadcast invalidation delivers live updates to web portal
9. `viewer_sync_status` shows sync freshness (green/amber/red)
10. Validation enforcement mode: validator rejections quarantine results as `AGENT_SUSPECT_OUTPUT`
11. 4 domain-specific validator overlays deployed and producing verdicts
12. Two users can view the same pipeline: desktop (read-write) + web (read-only)

### Test Scenarios

- `sqlite_schema.rs` — Full schema creation and migration
- `sqlite_supabase_sync.rs` — Push-pull round-trip sync
- `apps/web/e2e/contracts.spec.ts` — Web portal browsing
- Push batch with stale `row_version` → remote rejects (idempotent)
- Push batch with incompatible `schema_epoch` → sync pauses, local operation continues
- Two desktop operators make concurrent decisions → CAS detects conflict, second operator retries
- Validator rejects output in enforcement mode → results quarantined, error manifest created
- Validator itself crashes → fail-open, worker results advance with `validation_skipped_validator_error`
- Sync degraded → desktop shows browse-only mode, web shows staleness indicator
- Full push replay after sync recovery → idempotent, no duplicates

### Dependencies

- Layer 0–3 (all pipeline data, all push-owned tables with sync tracking columns)

### Forward-Compatibility Provisions

- `schema_epoch` exists for future schema evolution protocol
- RLS roles designed for future multi-org (`org_id` column planned)
- Projection tables designed for future materialized views

### Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Sync engine bugs corrupt remote data | Medium | Local copy is authoritative; remote can be republished |
| Validator enforcement blocks good data | Medium | Dry-run calibration in Layer 3; operator override available |
| Push batch size exceeds Supabase RPC limits | Low | Batch size bounded by `stage_run_id` scope |
| Network interruptions during push | Medium | Identical-envelope retries; degraded mode continues local operation |

### Decisions Addressed

- ADR-002: Sync / remote-projection half of the authority matrix fully operational
- ADR-008: Web viewing portal created (read-only)

---

## 7. Layer 5 — Expanded Commercial Intelligence

### Narrative

Broader categories, more supplier sources, decision-grade accuracy. The customs classification engine expands from screening-grade to full multi-source recall with 10-digit resolution, measure interaction, and trade remedy monitoring. Additional marketplace skills (Alibaba/1688, Amazon) provide multi-source pricing. FX engine gains dual-rate capability (HMRC customs + commercial spot). Freight engine gains real API integrations. Each new category requires its own gold-set validation before entering production.

**If we never build Layer 6:** The system is a mature procurement intelligence platform covering ≥3 goods categories with decision-grade landed cost and ≥2 supplier sources per category. Only request pipeline automation is missing.

### Capability Delivered

System handles ≥3 goods categories with decision-grade landed cost and ≥2 supplier sources per category. Full customs classification pipeline: multi-source recall (FPO + CPV priors + rule-based hints + tariff keyword search), multi-family expansion to UK 10-digit, exporter/manufacturer resolution, measure interaction engine, and trade remedy monitoring.

### Components Touched

| Component | Action | Files |
|-----------|--------|-------|
| Skills | Create | 4 (Amazon, Alibaba, 1688, PCS ingestion) |
| `packages/intelligence` | Create | 19 files (full customs, FX, freight, validation comparison) |

### Data Introduced

No new tables and no new columns — all decision-grade columns exist as nullable from Layer 0. Layer 5 populates previously-empty fields:

| Table | Columns Now Populated |
|-------|----------------------|
| `landed_cost_estimates` | Decision-grade fields: full duty breakdown, measure interaction results, exporter rate basis, supplementary unit requirements, valuation mode (all existed as nullable since L0) |
| `product_candidates` | Commodity code families from multi-source recall (via `product_commodity_codes` junction table, existed since L0) |
| `supplier_quotes` | Multi-source quotes (Alibaba, 1688, Amazon rows alongside existing eBay rows — same schema, more rows) |

### Authority Classification

No changes — all existing classifications hold.

### Shared Library Additions

- `packages/intelligence/src/customs/*` — Full customs classification engine (6 files)
- `packages/intelligence/src/fx/*` — Full FX engine (4 files)
- `packages/intelligence/src/logistics/*` — Full freight engine (5 files)
- `packages/intelligence/src/validation/*` — Landed-cost comparison adapters (3 files)

### Invariants Enforced

| Invariant | Type | Enforcement |
|-----------|------|-------------|
| Conservative customs defaults | Constitutional | MFN rates, residual duty, standard VAT when resolution is incomplete |
| Customs branch divergence as hard blocker | Constitutional | Materially different tariff branches → `CLASSIFICATION_AMBIGUOUS`, not averaged |
| Integer pence for all new cost fields | Constitutional | Type system |
| Per-category gold-set validation | Operational | New category requires gold-set expansion before production |

### What's Stubbed

Nothing significant — this is the full commercial intelligence implementation.

### Definition of Done

1. Alibaba/1688 search skills operational with Chinese keyword clusters
2. Amazon search skill operational
3. PCS (Scotland) ingestion skill operational
4. HMRC FPO API integration for HS code classification candidates
5. Multi-source recall produces candidates from ≥2 independent sources (FPO + CPV priors)
6. 10-digit resolution via HMRC Trade Tariff API produces UK-specific commodity codes
7. Measure scanner detects AD/CVD/safeguard/quota measures and models interactions
8. HMRC Exchange Rate Service provides customs valuation FX (monthly rolling)
9. Xe commercial FX provides spot rates
10. Freightos + SeaRates provide freight market data
11. Decision-grade landed cost per validated category (with full audit trail)
12. Category-specific gold-set expansion completed for ≥3 categories

### Test Scenarios

- `hmrc-fpo.test.ts` — FPO HS code classification
- Multi-family expansion: product with 3 plausible HS6 families → all UK10 descendants evaluated
- Trade remedy detection: steel product → AD/CVD measures detected, residual rate applied
- Compound duty: product with specific duty (£X/kg) → not flattened to percentage
- FX monthly rollover → forced recalculation of affected estimates
- Alibaba search with Chinese retry after English failure
- Category whitelist enforcement: medical device → rejected before search

### Dependencies

- Layer 0–4 (full pipeline, sync, web portal)

### Forward-Compatibility Provisions

- Classification freshness system (`tariffCheckedAt`, `classificationFreshUntil`, `tradeRemedyWatchState`) enables future auto-refresh
- CBAM forward placeholder for iron/steel/aluminium ready for future regulation

### Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| HMRC FPO API access restricted | Medium | FPO is not hard dependency; fallback to CPV-to-HS mapping |
| Anti-bot detection on Alibaba/1688 | High | Anti-bot classification table; source-health canaries; auth-refresh skills |
| Customs classification wrong for complex products | Medium | Multi-source recall + conservative defaults + abstention |
| Freight API rate variability | Medium | Dual-source cross-check (Freightos + SeaRates) |

### Decisions Addressed

- ADR-006: Internal landed-cost engine at full scope (decision-grade)
- ADR-001: Additional marketplace skills (Alibaba, 1688, Amazon)

---

## 8. Layer 6 — Request Pipeline Automation

### Narrative

Portal-specific bid skills automate the request pipeline — preparation and submission steps for tender portal applications. This does NOT place real-money bids; actual negotiations and fulfillment are manual. The system produces full audit trails (screenshots, logs, manifests) for every submission.

**If we never build this layer:** The system is fully functional for intelligence and decision-making. Operators manually prepare and submit portal applications based on system recommendations.

### Capability Delivered

System can prepare and submit a request on one portal (Find a Tender or Contracts Finder) with full audit trail. Includes agent-browser workflow, screenshot capture, and manifest generation for submitted requests.

### Components Touched

| Component | Action | Files |
|-----------|--------|-------|
| Skills | Create | 1 (bid-portal-proactis) |
| `packages/bidding` | Create | 3 (workflow registry, proactis workflow, company profile schema) |

### Data Introduced

**Modified tables:**

| Table | Enhancement |
|-------|-------------|
| `bid_workflow_states` | Fully active: `not_started` → `ready_to_bid` → `bid_in_progress` → `bid_submitted` / `bid_failed` |

### Authority Classification

`bid_workflow_states` is push-owned — daemon tracks bid progress, pushes to web portal.

### Shared Library Additions

- `packages/bidding` — Workflow registry and portal-specific workflow definitions

### Invariants Enforced

| Invariant | Type | Enforcement |
|-----------|------|-------------|
| No real-money bids without human approval | Constitutional | Bid workflow requires operator `approved` disposition |
| Bid workflow carries explicit scope | Constitutional | Which lots and candidates included |
| Full audit trail for submissions | Constitutional | Screenshots, logs, manifests captured |

### What's Stubbed

Nothing — this is the final layer.

### Definition of Done

1. Bid-portal skill navigates target portal via browser automation
2. Company profile data injected from `shared_company_profile`
3. Request preparation steps automated (form filling, document upload)
4. Submission step automated with screenshot capture at each stage
5. Full terminal output tracing for bid workflow
6. Manifest generated for submitted request with audit trail
7. Bid failure produces error manifest with portal-specific context
8. System can prepare and submit one request on the target portal end-to-end

### Test Scenarios

- Full bid workflow: approve → prepare → submit → manifest
- Bid failure mid-workflow: error manifest with screenshots at failure point
- Stale decision blocks bid: operator decision marked stale → bid workflow blocked until re-affirm
- Portal website changed: error manifest flags skill failure with step context

### Dependencies

- Layer 0–5 (full pipeline, desktop, web, sync, decision-grade intelligence)

### Forward-Compatibility Provisions

- Workflow registry pattern supports additional portal-specific skills
- Company profile schema designed for multiple portal requirements

### Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Portal UI changes break bid skill | High | Low impact — not real money; error manifest flags failures |
| Form structure varies by procurement type | Medium | Portal-specific workflow handles variance |

### Decisions Addressed

- ADR-007: Request pipeline automation (not real-money bids)

---

## 9. Cross-Layer: Schema Stability Matrix

### Table Introduction and Modification Across Layers

| Table | Created | First Active Use | Sync Activated | Authority |
|-------|---------|-----------------|----------------|-----------|
| `procurement_processes` | L0 | L0 | L4 | Push-owned |
| `contract_notices` | L0 | L0 | L4 | Push-owned |
| `notice_lots` | L0 | L0 | L4 | Push-owned |
| `product_candidates` | L0 | L0 | L4 | Push-owned |
| `search_profiles` | L0 | L0 | L4 | Push-owned |
| `search_attempts` | L0 | L0 | L4 | Push-owned |
| `supplier_quotes` | L0 | L0 | L4 | Push-owned |
| `candidate_pricing_assessments` | L0 | L0 | L4 | Push-owned |
| `pipeline_assessments` | L0 | L0 (gold-set harness), L1 (first live-source rollups), L2 (desktop coverage surface) | L4 | Push-owned |
| `qualification_runs` | L0 | L0 | L4 | Push-owned |
| `landed_cost_estimates` | L0 | L0 (screening-grade), L5 (decision-grade fields populated) | L4 | Push-owned |
| `tender_document_sets` | L0 | L1 (first live-source document acquisition) | L4 | Push-owned |
| `manifests` | L0 | L1 (first live-source verifiable output), L2 (desktop rendering) | L4 | Push-owned |
| `process_stage_status` | L0 | L1 (stage tracking) | L4 | Push-owned |
| `process_stage_heads` | L0 | L1 (stage tracking) | L4 | Push-owned |
| `bid_workflow_states` | L0 | L6 (bid automation) | L4 | Push-owned |
| `workflow_jobs` | L0 | L1 (job runner), L3 (validation fields) | Never (local-only) | Local-only |
| `notice_cpv_codes` | L0 | L0 | L4 | Push-owned (junction) |
| `product_commodity_codes` | L0 | L0 | L4 | Push-owned (junction) |
| `lot_cpv_codes` | L0 | L0 | L4 | Push-owned (junction) |
| `agent_session_logs` | L1 | L1 (run-manifest fields populated from introduction) | Never (local-only) | Local-only |
| `raw_documents` | L1 | L1 | Never (local-only) | Local-only |
| `error_manifests` | L1 | L1 | Never (local-only) | Local-only |
| `executors` | L1 | L1 | L4 | Push-owned |
| `source_cursors` | L1 | L1 | Never (local-only) | Local-only |
| `sources` | L1 | L1, L4 (pull from `shared_source_configs`) | Never (local-only) | Local-only (runtime copy) |
| `validation_verdicts` | L3 | L3 | Never (local-only) | Local-only |
| `source_runtime_state` | L3 | L3 | Never (local-only) | Local-only |
| `listing_evidence` | L3 | L3 | Never (local-only) | Local-only |
| `process_relations` | L3 | L3 | L4 | Push-owned |
| `process_aliases` | L3 | L3 | L4 | Push-owned |
| `local_outbox` | L4 | L4 | N/A (SQLite sync infra) | Local-only |
| `sync_batches` | L4 | L4 | N/A (SQLite sync infra) | Local-only |
| `pull_cursors` | L4 | L4 | N/A (SQLite sync infra) | Local-only |
| `pending_operator_intents` | L0 | L2 (desktop decision capture), L4 (CAS flush + conflict tracking) | Never (local-only) | Local-only |
| `operator_decision_heads` | L4 (Supabase + pulled local cache) | L4 | N/A (Supabase-native) | Remote-owned |
| `shared_company_profile` | L4 (Supabase) | L4, L6 (bid workflow fields populated) | N/A (Supabase-native) | Remote-owned |
| `shared_source_configs` | L4 (Supabase) | L4, L5 (new source entries) | N/A (Supabase-native) | Remote-owned |
| Summary tables (`workflow_job_summaries`, `error_manifest_summaries`, `validation_verdict_summaries`, `viewer_sync_status`) | L4 (Supabase) | L4 | N/A (push-only projections) | Push-owned |

### Schema Evolution Rules

1. **No table reclassification.** A table's authority classification is permanent from its introduction layer. `operator_decision_heads` is remote-owned from introduction in Layer 4; before that, local single-operator decisions exist only as `pending_operator_intents`.
2. **All columns exist from table creation.** Later layers populate existing nullable columns — they never add new columns to tables introduced in earlier layers. This eliminates schema migrations between layers.
3. **Sync tracking columns from day 1.** All push-owned tables include `owner_id`, `row_version`, `last_local_seq`, `stage_run_id`, `created_at`, `updated_at` from their creation in Layer 0, and those fields are populated on first write even before the outbox engine exists.
4. **No column renames.** Column names are stable across layers (important for sync field mapping).
5. **Junction tables introduced with parent tables.** `notice_cpv_codes` created alongside `contract_notices` in L0.
6. **Job kinds.** Each pipeline stage maps to a specific `job_kind` value in `workflow_jobs` (e.g., `sync_source`, `normalize_notice`, `qualify_notice`, `extract_products`, `enrich_supplier_prices`, `estimate_landed_cost`). The full mapping of 16 job kinds to pipeline stages is defined in the research document Appendix C.
7. **Validation forward-compatibility exists from job-table introduction.** `workflow_jobs.parent_job_id` is nullable from Layer 0, `workflow_jobs.validation_required` exists from Layer 0 with default `false`, and `workflow_jobs.validation_verdict_id` is nullable from Layer 0.
8. **Run-manifest columns exist from session-log introduction.** `agent_session_logs.run_manifest_path` and `agent_session_logs.run_manifest_hash` exist when `agent_session_logs` is introduced in Layer 1 and are populated from the first skill run.
9. **Validation state exists from assessment-table introduction.** `pipeline_assessments.validation_state` exists from Layer 0 with default `'unverified'`.

---

## 10. Cross-Layer: Interface Contracts

### Boundaries That Later Layers MUST NOT Break

| Interface | Introduced | Consumers | Contract |
|-----------|-----------|-----------|----------|
| Agent Adapter trait | L1 | All skill execution | Unified event stream regardless of PTY/contract mode; mode selection via `interaction_profile` |
| QueryService envelope | L2 | Desktop UI | `QueryEnvelope<T>` response format for all SQLite reads |
| DaemonBridge commands | L2 | Desktop UI | Mutation acknowledgment format; UDS/named-pipe transport |
| Channel<T> streaming | L2 | Desktop session viewer | Session streaming protocol |
| Skill directory standard | L0 | All skills, skill linting | SKILL.md + result.schema.json + checkpoint.schema.json + canary.inputs.jsonl + golden.assertions.yaml |
| Result contract format | L1 | Job Runner, OQG, validators | Schema-validated output format per skill |
| Push batch RPC | L4 | Sync engine → Supabase | `apply_push_batch(batch_json)` with `schema_epoch`, `row_version` ordering |
| CAS decision RPC | L4 | Desktop → Supabase | `update_operator_decision(process_id, disposition, base_revision, command_id)` — idempotent via `command_id` to handle lost-response retries |
| Pull-before-push ordering | L4 | Sync startup | Pull remote-owned state FIRST, then classify/coalesce pending intents, then resume push — ensures cross-authority consistency |
| `packages/contracts-schema` types | L0 | All TypeScript code | Canonical type definitions; changes must be backward-compatible |
| `packages/ui` frame + slot pattern | L2 | Desktop + web shells | Presentational components; data-fetching hooks are shell-specific |
| Evaluation harness API | L0 | Gold-set benchmarking | Pipeline runner + metrics computation interface |

### Stage Scope Matrix

`ProcessStageStatus.scopeKind` is constrained to exactly six values: `process`, `notice`, `lot`, `candidate`, `candidate_source_pair`, and `candidate_quote_scenario`. The strategy must keep every stage on one of these grains once a process/notice identity exists.

| Stage | Runtime Grain | `scopeKind` | Required Entities Already Introduced |
|-------|---------------|-------------|--------------------------------------|
| Ingest | source-batched precursor job | Not recorded in `process_stage_status` until notice/process identity exists | `sources`, `source_cursors`, procurement ingestion skill |
| Normalize | per notice | `notice` | `contract_notices` |
| Relate | per process cluster | `process` | `procurement_processes`, `contract_notices` |
| Qualify | per lot | `lot` | `notice_lots`, `qualification_runs` |
| Acquire docs | per notice | `notice` | `contract_notices`, `tender_document_sets` |
| Extract lots | per notice | `notice` | `contract_notices`, `notice_lots` |
| Extract products | per lot | `lot` | `notice_lots`, `product_candidates` |
| Score quoteability | per candidate | `candidate` | `product_candidates`, `search_profiles` |
| Price-signal match | per candidate-source pair | `candidate_source_pair` | `product_candidates`, `search_profiles`, `search_attempts`, `supplier_quotes` |
| Gross margin gate | per candidate | `candidate` | `candidate_pricing_assessments`, `supplier_quotes` |
| Landed cost | per candidate-quote scenario | `candidate_quote_scenario` | `candidate_pricing_assessments`, `landed_cost_estimates` |
| Net margin gate | per candidate | `candidate` | `landed_cost_estimates`, `pipeline_assessments` rollup inputs |
| Manifest | per process | `process` | `pipeline_assessments`, `manifests` |
| Operator decision | per process | `process` | `pending_operator_intents` first, `operator_decision_heads` once Layer 4 activates remote authority |
| Bid workflow | per process with explicit lot/candidate scope | `process` | `bid_workflow_states`, explicit bid scope payload |

Implementation note for the apparent ingest-grain mismatch:

- Source-batched ingestion is tracked at the `workflow_jobs` / `source_cursors` / `executors` level.
- `process_stage_status` begins once a notice or process identity exists, so persisted stage-head coherence still stays inside the six canonical `scopeKind` values from the research model.

Scope dependency rule:

- Lot-grain stages require `notice_lots` to exist in the same or an earlier layer.
- Candidate-grain stages require `product_candidates` to exist in the same or an earlier layer.
- Candidate-source-pair stages require `search_profiles` and `supplier_quotes` machinery to exist in the same or an earlier layer.
- Candidate-quote-scenario stages require a persisted candidate-scoped pricing assessment and selected quote before landed cost runs.

### Stage Boundary Contracts

These input/output contracts between pipeline stages are established in Layer 0 and must hold through all layers:

| Boundary | Required Input | Required Output | Established |
|----------|---------------|-----------------|-------------|
| qualify → acquire_docs | `QualificationResult` with lotId, expiryQualified, goodsQualified, accessRoute | Per-lot pass/review/reject | L0 |
| acquire_docs → extract_lots | Document set completeness, document roles | `TenderDocumentSet` with `documentSetHash` | L1 |
| extract_lots → extract_products | `NoticeLot` with title/description, lotNumber | At least one lot per qualified notice | L0 |
| extract_products → score_quoteability | `ProductCandidate` with lotId, headNoun, identityConfidence, candidateType, matchArchetype | At least one candidate per qualified lot | L0 |
| score_quoteability → price_match | quoteabilityScore, recallRiskScore, SearchProfile with queryWaves | quoteabilityScore ≥ 0.45 for auto-search | L0 |
| price_match → gross_gate | `SupplierQuote` with matchConfidence, quoteUsabilityConfidence, priceSignalRole | At least one quote per searched pair | L0 |
| gross_gate → landed_cost | Candidate-scoped pricing assessment, revenue basis, confidence gates | grossMarginBps, selected quote | L0 |
| landed_cost → net_gate | `LandedCostEstimate` anchored to candidate + quote | netMarginLowPct/BasePct/HighPct, blockers | L0 |
| net_gate → manifest | Process-level rollup with coverage metrics | Coverage-aware qualification | L1 |

---

## 11. Cross-Layer: Testing Evolution

### Testing Strategy Per Layer

| Layer | Fast Inner Loop | Live Smoke | Benchmark Batch | Release Gate |
|-------|----------------|------------|-----------------|--------------|
| L0 | Extraction parsers, margin math, scoring, data model | One skill + one lot through pipeline | Full gold-set corpus vs human labels | All skills, full pipeline, extended corpus |
| L1 | + Rust unit tests (adapter, PTY) | + Real skill session via daemon | + 10 consecutive batch runs | + Process supervision stress test |
| L2 | + Desktop component tests | + Desktop E2E flow | + Operator runs daily for 1 week | + Full desktop + pipeline E2E |
| L3 | + Validation policy logic, relation scoring | + Cron-triggered skill session | + 5 consecutive days unattended | + All fault injection + replay tests |
| L4 | + Sync protocol unit tests | + Push-pull round trip | + Multi-user concurrent access | + Full sync + web portal E2E |
| L5 | + Customs classification, FX, freight | + Multi-source search session | + Decision-grade accuracy vs gold set | + ≥3 categories, ≥2 sources each |
| L6 | + Bid workflow logic | + Portal navigation | + End-to-end bid submission | + Full audit trail verification |

### Test Count Growth

| Layer | New Test Files | Cumulative | Focus |
|-------|---------------|------------|-------|
| L0 | 4 | 4 | Intelligence pipeline unit tests |
| L1 | 7 | 11 | Adapter, PTY, skill linting |
| L2 | 5 | 16 | Agent E2E, desktop E2E |
| L3 | 16 | 32 | Fault injection, replay, validation, drift |
| L4 | 3 | 35 | Sync, schema, web E2E |
| L5 | 1 | 36 | Customs FPO |
| L6 | 0 | 36 | Covered by skill E2E tests |

---

## 12. Cross-Layer: Milestone Mapping

### Correspondence to Research Document Milestones (§11)

| Layer | Research Milestone | Deviation | Justification |
|-------|-------------------|-----------|---------------|
| L0 | M0: Gold Set + Evaluation Harness | None | Direct mapping |
| L1 | M1: Thin Local Runner | None | Direct mapping |
| L2 | M2: Single-Operator Desktop MVP | None | Direct mapping |
| L3 | M3: Operational Hardening | None | Direct mapping — includes dry-run validation |
| L4 | M4: Shared Visibility | None | Direct mapping — includes validation enforcement |
| L5 | M5: Expanded Commercial Intelligence | None | Direct mapping |
| L6 | M6: Request Pipeline Automation | None | Direct mapping |

**No deviations from M0–M6.** The research document's milestone structure was validated through 3 planmaxer expert review iterations and 2 harmonymaxer feedback integrations. The dependency analysis confirms that the milestone ordering correctly respects component dependencies, atomic operation pairs, and the validation-first delivery philosophy.

### Why No Merging or Splitting

- **M3/M4 remain separate:** M3 (dry-run validation) must accumulate a run-manifest corpus and calibrate false positive rates before M4 enables enforcement. Merging them would force simultaneous deployment of enforcement and calibration — a known anti-pattern.
- **M5/M6 remain separate:** Decision-grade landed cost (M5) is independently valuable without bid automation (M6). Merging forces the highest-complexity commercial intelligence and portal-specific bid skills into one layer.
- **M0 remains standalone:** The evaluation harness has ZERO runtime dependencies on the daemon. It must exist before pipeline code to anchor all quality metrics.

---

## 13. Cross-Layer: Stack Sequencing

### Technology Introduction Order

The research document's recommended stack sequencing (§11) is followed without deviation:

```
Layer 0: Skills (SKILL.md playbooks) + TypeScript intelligence
  ↓
Layer 1: + Thin Rust (daemon binary, PTY, process supervision)
  ↓
Layer 2: + Desktop UI (Tauri shell) + Shared UI (packages/ui)
  ↓
Layer 3: + Operational Rust (cron, resource arbiter, validation)
  ↓
Layer 4: + Supabase (sync engine, projections) + Web (Next.js portal)
  ↓
Layer 5: + Extended intelligence (customs, FX, freight APIs)
  ↓
Layer 6: + Bidding (portal-specific workflow skills)
```

### Rationale

1. **Skills first** — Cheapest to write, fastest to iterate, directly validate intelligence thesis. Run from CLI.
2. **TypeScript intelligence** — Pipeline logic lives here. This is where the product thesis is proven or disproven.
3. **Thin Rust** — Only enough daemon to validate runtime reliability. Not a full platform.
4. **Desktop UI** — Only after the pipeline produces results worth displaying.
5. **Operational Rust** — Hardening for daily unattended operation.
6. **Supabase + Web** — Only after a single operator has validated daily use.
7. **Extended intelligence** — Decision-grade accuracy for broader categories.
8. **Bidding** — Portal automation after the full intelligence stack is proven.

### Package Dependency Graph

```
packages/contracts-schema (L0)
  ← packages/intelligence (L0)
  ← packages/manifests (L2)
  ← packages/ui (L2)
  ← packages/bidding (L6)
  ← apps/desktop (L2)
  ← apps/web (L4)
  ← crates/daemon (L1, via SQLite schema alignment)
```

---

## Appendix: Component Activation Sequence

For quick reference, when each major component becomes active:

| Component | Layer | Mode |
|-----------|-------|------|
| Skill directory standard | L0 | Full |
| TypeScript intelligence pipeline | L0 | Screening-grade |
| Evaluation harness | L0 | Full |
| Gold-set lot corpus | L0 | Initial (50–100 lots) |
| Rust daemon binary | L1 | Thin (launch + capture + supervise) |
| Agent adapter (PTY + contract) | L1 | Full |
| Process-group supervision | L1 | Full |
| Run manifest writer | L1 | Full (with machine-authored fallback) |
| Error manifest system | L1 | Full |
| Stage tracking (`stage_run_id` + heads) | L1 | Full |
| Tauri desktop shell | L2 | Full control panel |
| QueryService (SQLite reads) | L2 | Full |
| DaemonBridge (daemon commands) | L2 | Full |
| Live session streaming | L2 | Full |
| Shared UI components | L2 | Full |
| Cron scheduler | L3 | Full |
| Resource arbiter | L3 | Full |
| PTY hardening | L3 | Full |
| Relation resolution | L3 | Full |
| Skill promotion flow | L3 | Full |
| Source health monitoring | L3 | Full |
| Validation policy engine | L3 | Dry-run |
| Validation executor | L3 | Dry-run |
| Base validator skill | L3 | Dry-run |
| Sync engine (push) | L4 | Full |
| Sync engine (pull) | L4 | Full |
| Supabase projections | L4 | Full |
| Web viewing portal | L4 | Full |
| Validation enforcement | L4 | Full |
| Domain validator overlays | L4 | Full |
| Full customs engine | L5 | Decision-grade |
| Full FX engine | L5 | Full |
| Full freight engine | L5 | Full |
| Multi-source marketplace search | L5 | Full |
| Bid workflow automation | L6 | Full |

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

## 3. Schema, Artifacts & Stage Tracking

> **Ownership:** This section owns the **stage tracking model** — the authoritative mapping of pipeline stages to scope grains and commit semantics. Stage **processing logic** (what each stage computes) is owned by Layer 0 §3. Full **DDL for all tables** is owned by Layer 0 §2 (constitutional schema).

### 3.1 Tables Introduced in Layer 1

These tables are defined in Layer 0 §2 (constitutional schema with all columns from creation) but become **first actively used** in Layer 1. Full CREATE TABLE statements are in L0 §2; this section documents what activates and why.

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
| 4 | **Qualify** | `lot` | Set-replacement | Lot-varying decisions cannot be process-level |
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

7 new Rust test files for Layer 1 (cumulative: 11 including L0's 4 TS test files):

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

**§10 Assumption #2: Desktop-local execution is non-negotiable (VALIDATED)**
- Evidence: Resolved in owner feedback and ADR-003/ADR-004 architecture.
- Remaining concern: Confirm what machine is expected to stay online for freshness SLAs.
- **Layer 1 impact:** Daemon is a local process, not a cloud service. PTY sessions run locally.

### 6.2 Stubs (What L1 Defers)

| Feature | L1 Stub | Full Implementation |
|---------|---------|-------------------|
| Scheduler | CLI manual trigger only | Layer 3: cron scheduler with explicit skill binding |
| Resource arbiter | No concurrency limits | Layer 3: typed quotas (browser slots, agent slots, source lanes) |
| PTY hardening | Basic session creation only | Layer 3: fd hygiene, EIO handling, terminal geometry stability |
| Job claiming/leases | Single-threaded sequential execution | Layer 3: concurrent job claiming with lease management |
| Source registry | Bootstrapped local seed / hardcoded source definitions | Layer 3: typed registry with cron binding; Layer 4: pull from shared_source_configs |
| Procurement coverage | FTS only (first live source) | Layer 2: adds Contracts Finder; Layer 5: PCS if in scope |
| Session reaper | Basic cleanup (pgid kill + verdict) | Layer 3: full reaper with escalation tiers, orphan sweep, cleanup timeout |
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

# Layer 2 — Single-Operator Desktop MVP

> **Authority:** This document is the execution-grade specification for Layer 2. An implementation agent should be able to build this layer from this document alone without reopening the research document.
>
> **Milestone:** M2
> **Dependencies:** Layer 0 (intelligence pipeline, skills, contracts-schema), Layer 1 (daemon runtime, artifact capture, process supervision)
> **Exit criteria:** Single operator runs system daily for 1 week, making real accept/reject decisions on genuine opportunities.

---

## 1. Overview, Capability & Desktop Architecture

### 1.1 Layer Narrative

Prove repeatable operator value with a manual-trigger desktop sitting on top of the Layer 1 live local pipeline. This is the **first user-facing layer** — a single operator runs the system daily, reviews pipeline output in a desktop control panel, and makes accept/reject/watch decisions. This layer expands live procurement coverage to Contracts Finder, renders operator-facing manifests and coverage metrics, and persists single-operator decisions locally as queued intents without violating remote authority ownership.

**Why this layer exists:** Layer 1 proved runtime reliability. Layer 2 proves the value proposition: can a real operator use this system daily and find genuine commercial opportunities? If the answer is no, later layers (sync, validation, expanded intelligence) are wasted investment.

**If we never build Layer 3:** A single operator can use the system with manual triggering for daily procurement intelligence. No cron, no automation, but the core value loop works.

### 1.2 Capability Delivered

Single operator runs system daily for one week, making real accept/reject decisions. The pipeline surfaces genuine opportunities from live procurement data with screening-grade margin estimates. The desktop control panel provides:

- Notice list view with keyset pagination
- Opportunity detail view with margin bands, product candidates, supplier quotes
- Agent session monitoring with live terminal output
- Decision capture (approve/reject/watch) with staleness tracking
- Coverage metrics display
- Error manifest view

### 1.3 Desktop Architecture

Tauri v2 with a **separate background daemon** — the desktop app is NOT the daemon. The daemon must survive UI close, restart independently, and have its own lifecycle.

```text
React SPA (Vite)
  -> Tauri invoke()          for snapshot queries + mutations
  -> Tauri Channel<T>        for live streams (sessions, jobs, daemon health)
  -> Desktop Rust bridge
       -> QueryService       (read-only SQLite, page-specific read models)
       -> DaemonBridge       (UDS/named-pipe RPC to daemon)
            -> Rust daemon   (writer, jobs, PTY, L1 runtime)
```

**Three communication channels:**

| Channel | Transport | Direction | Purpose |
|---------|-----------|-----------|---------|
| **Snapshot reads** | `invoke()` -> QueryService | Frontend -> SQLite (read-only) | List views, detail views, metrics — fast, no network dependency |
| **Mutations/triggers** | `invoke()` -> DaemonBridge -> UDS/pipe -> daemon | Frontend -> Daemon | Trigger ingestion, trigger enrichment, capture decisions |
| **Live streams** | Daemon -> DaemonBridge -> `Channel<T>` -> React | Daemon -> Frontend | Session terminal output, job progress, daemon health |

**Why not pure HTTP on localhost:** Port discovery/conflicts, auth/token management, extra attack surface, network semantics for a non-networked relationship.

**Why the Tauri backend is not the daemon:** The daemon is a background scheduler/executor. It must survive UI close, restart independently, and have its own lifecycle. Embedding it makes shutdown, crash recovery, and "app closed but cron still runs" (Layer 3) architecturally unsound.

### 1.4 QueryService Interface

The QueryService opens **read-only** SQLite connections and returns page-specific read models. It enforces: read-only/query-only connections, page-limited queries, no streaming cursors held open, no generic SQL from the frontend, no transcript/blob reads in hot queries.

```typescript
type QueryEnvelope<T> = {
  data: T
  snapshotSeq: number
  sourceMeta: {
    mode: 'healthy' | 'browse_only' | 'recovery_required'
    daemonConnected: boolean
    syncHealth: 'healthy' | 'degraded' | 'offline'  // always 'offline' in L2 (no sync yet)
  }
}
```

**Read model examples:**

| Query | Returns | Page |
|-------|---------|------|
| `list_processes(cursor, limit, filters)` | Keyset-paginated process list with pipeline state, coverage metrics, decision status | List view |
| `get_process_detail(process_id)` | Full process detail: notices, lots, candidates, quotes, costs, manifest, errors | Detail view |
| `list_sessions(cursor, limit, filters)` | Active and recent agent sessions with metadata | Sessions view |
| `list_errors(cursor, limit, filters)` | Unresolved error manifests with context | Errors view |

**SQLite concurrency rule:** SQLite is single-writer (daemon only) but supports many short readers. The QueryService opens read-only connections with short-lived transactions. Long-lived readers are prohibited because they prevent WAL checkpoints from draining. No transcript blobs or large artifacts in hot queries — those load on demand from the artifact store.

### 1.5 DaemonBridge Interface

The DaemonBridge is a thin RPC proxy — it does not hold state or perform writes itself. It forwards commands to the daemon over UDS (macOS/Linux) or named pipe (Windows).

```typescript
type MutationAck = {
  accepted: boolean
  idempotencyKey: string
  jobId?: string
  remoteState?: 'committed' | 'queued' | 'conflicted' | 'not_applicable'
  // L2: remoteState is always 'not_applicable' (no sync yet)
}
```

**Bridge RPC types:**

```typescript
type BridgeRequest =
  | { id: string; kind: 'command'; method: string; params: Record<string, unknown>; idempotencyKey: string }
  | { id: string; kind: 'subscribe'; topic: string; fromSeq?: number }
  | { id: string; kind: 'unsubscribe'; topic: string }

type BridgeEvent =
  | { topic: 'daemon.health'; payload: DaemonHealthPayload }
  | { topic: string; payload: JobProgressPayload }
  | { topic: string; seq: number; kind: 'pty' | 'stdout' | 'stderr' | 'event' | 'lifecycle' | 'gap'; payload: unknown }
```

**Commands available in Layer 2:**

| Command | Action | Returns |
|---------|--------|---------|
| `trigger_ingestion(source_id)` | Start ingestion job for a source | `MutationAck` with `jobId` |
| `trigger_enrichment(process_id)` | Re-run enrichment for a process | `MutationAck` with `jobId` |
| `capture_decision(process_id, disposition, basis)` | Record operator decision | `MutationAck` |
| `resolve_error(error_manifest_id)` | Mark error as resolved | `MutationAck` |

### 1.6 Channel\<T\> Live Streaming

Tauri's typed streaming mechanism for live session output from daemon to frontend.

**Session frame types:**

```typescript
type SessionFrame =
  | { seq: number; kind: 'lifecycle'; state: string; ts: string }
  | { seq: number; kind: 'pty'; chunk: Uint8Array; ts: string }
  | { seq: number; kind: 'stdout'; chunk: string; ts: string }
  | { seq: number; kind: 'stderr'; chunk: string; ts: string }
  | { seq: number; kind: 'event'; event: StructuredSessionEvent; ts: string }
  | { seq: number; kind: 'gap'; fromSeq: number; toSeq: number }
```

**Two subscription levels:**

| Level | What | Cost | Used By |
|-------|------|------|---------|
| All running sessions (metadata only) | Status, checkpoint count, last line excerpt, elapsed time | Cheap — session list can show 20+ concurrent sessions | Session list page |
| Selected session (full stream) | Full PTY/stdout/stderr via `Channel<T>` | Expensive — at most 2-3 sessions attached simultaneously | Session detail viewer |

**Buffering strategy:**
- Batch frames every 33-50ms before writing to terminal component
- Keep only last N lines / M MB hot in live buffer
- User scrolls up → switch from live-tail to history mode (lazy-loaded from artifact chunks)
- UI falls behind stream → emit `gap` frame and force resync from last known artifact position

**Dual viewer pattern:**

| Viewer | Default For | Description |
|--------|------------|-------------|
| Live terminal | PTY mode sessions | ANSI-faithful, auto-scroll, last few thousand lines |
| Transcript/history | Contract mode sessions | Virtualized line list, searchable, lazy-loaded from artifact chunks |

### 1.7 Bootstrap State Machine

On launch, the desktop determines its runtime mode:

```text
Start
  -> Check local DB readability + schema_epoch (PRAGMA user_version)
  -> Attempt daemon handshake over socket/pipe
  -> Handshake returns: daemon version, IPC protocol version, schema_epoch,
     sync health, active sessions summary
```

**Runtime modes:**

| Mode | Local DB | Daemon | Remote Sync | UX |
|------|----------|--------|-------------|-----|
| **Healthy** | OK | Connected | N/A (L2) | Full desktop: list, detail, trigger, decide, monitor |
| **Browse-only** | OK | Unavailable | N/A | Read-only local browsing: list, detail, manifests, historical transcripts. No reruns, no triggers, no live sessions. Clear banner: "Daemon unavailable — read-only local snapshot." |
| **Recovery required** | Missing/mismatch | Any | N/A | Repair/update screen. DB schema_epoch mismatch or IPC protocol version mismatch. |

**Daemon crash mid-session:**
1. Stream/heartbeat drops
2. Desktop shows reconnect overlay
3. Bootstrap attempts one automatic daemon restart
4. On reconnect: invalidate all snapshot queries, reattach session streams from last `seq`, daemon reconciles orphan sessions
5. If restart fails: fall back to browse-only mode

**Shutdown behavior:** Closing the main window does not silently kill the daemon.
- **Quit UI** — daemon keeps running (important for Layer 3 cron)
- **Stop background daemon and quit** — clean daemon shutdown

### 1.8 Desktop File Listing

**`apps/desktop/` — 14 files:**

| File | Responsibility |
|------|---------------|
| `src-tauri/src/main.rs` | Tauri Rust bootstrap: DB check, daemon handshake, QueryService + DaemonBridge init |
| `src-tauri/src/query_service.rs` | Read-only SQLite queries returning `QueryEnvelope<T>` |
| `src-tauri/src/daemon_bridge.rs` | RPC proxy to daemon over UDS/named-pipe; `Channel<T>` relay |
| `src-tauri/capabilities/default.json` | Allowed shell/sidecar commands |
| `src-tauri/tauri.conf.json` | Sidecar and desktop build configuration |
| `src/App.tsx` | Root React component with routing and bootstrap state |
| `src/pages/ContractList.tsx` | Keyset-paginated process list with pipeline badges and decision status |
| `src/pages/ContractDetail.tsx` | Full opportunity detail: candidates, quotes, margins, manifest, errors |
| `src/pages/Sessions.tsx` | Active/recent agent session list with metadata |
| `src/pages/SessionViewer.tsx` | Live terminal/transcript viewer with dual mode (PTY/contract) |
| `src/pages/Errors.tsx` | Unresolved error manifest list with drill-down |
| `src/stores/session-registry.ts` | External store: tracks all active/recent sessions with metadata |
| `src/stores/session-buffer.ts` | External store: per-session ring buffer for live frames |
| `src/hooks/use-daemon.ts` | `useSessionMeta()`, `useSessionTail()`, `useSessionTimeline()` — slice subscriptions via `useSyncExternalStore` |

### 1.9 React State Management

**Pattern:** Purpose-built external stores with `useSyncExternalStore()` for slice subscriptions. **Not** TanStack Query for streaming state.

| State Class | Approach |
|-------------|----------|
| Snapshot server-state | TanStack Query for snapshot reads only — no offline persistence, no normalized cache |
| Live streaming state | Custom external stores (`SessionRegistry`, `SessionBuffer`) + `useSyncExternalStore` |
| Ephemeral UI state | Local component state / tiny persisted store |

**Rule: never put raw transcript arrays in React state.** Session output is a log system problem, not a React state problem.

**Hooks:**
- `useSessionMeta(sessionId)` — metadata slice (status, elapsed, checkpoints)
- `useSessionTail(sessionId)` — last N lines for terminal view
- `useSessionTimeline(sessionId)` — structured event timeline for contract mode

### 1.10 Shared UI Components (`packages/ui`)

**Frame + slot composition pattern** (not boolean props):

```tsx
<ContractCardFrame
  summary={vm.summary}
  metrics={vm.metrics}
  rightRail={<SyncAgeBadge meta={vm.meta} />}
  actions={desktop ? <DesktopContractActions ... /> : null}
/>
```

**5 shared components:**

| Component | Purpose | Used By |
|-----------|---------|---------|
| `ContractCardFrame` | Process summary card with slot composition | Desktop list, web list (L4) |
| `ManifestViewer` | Markdown manifest renderer with section navigation | Desktop detail, web detail (L4) |
| `PipelineBadge` | Pipeline state badge (qualified/review/rejected/pending) | Both surfaces |
| `SyncAgeBadge` | Data freshness indicator (green/amber/red) | Both surfaces (desktop shows local freshness; web shows sync freshness in L4) |
| `ErrorSummaryList` | Compact error manifest list with severity/resolution status | Both surfaces |

**Sharing boundary:** Share down to view-models, presentational components, formatting/markdown/tables/badges. Stop sharing at repositories, hooks, realtime subscriptions, mutation flows.

### 1.11 Manifest Rendering (`packages/manifests`)

**2 files:**

| File | Purpose |
|------|---------|
| `render-opportunity.ts` | Deterministic opportunity manifest renderer — takes pipeline data, produces structured Markdown |
| `opportunity.md.ts` | Markdown template for opportunity manifests |

The canonical rendering lives here, not in the `manifest-generation` skill. The skill may contribute optional narrative synthesis, but the deterministic renderer is authoritative.

### 1.12 New Skill: Contracts Finder Ingestion

| Attribute | Value |
|-----------|-------|
| **Path** | `skills/procurement-ingestion/contracts-finder/` |
| **Purpose** | Ingest live notices from Contracts Finder |
| **Interaction profile** | `pty_browser` |
| **API** | OCDS/Search API: `publishedFrom`, `publishedTo`, `stages`, `limit`, `cursor`. Rate limiting documented. |
| **Licensing** | Aligned with open contracting/publication standards |
| **Cursor strategy** | Date-based filtering + cursor; rate-limit-aware polling |

### 1.13 What's Stubbed in This Layer

| Feature | Layer 2 Stub | Full Implementation |
|---------|-------------|-------------------|
| Scheduling | Manual trigger from desktop | Layer 3: cron scheduler |
| Relation resolution | Notices processed independently | Layer 3: merge/relate heuristics |
| Landed cost | Screening-grade (flat freight, single FX) | Layer 5: decision-grade |
| Search sources | eBay only (from Layer 0) | Layer 5: Alibaba/1688, Amazon |
| Sync to Supabase | None — local-only operation | Layer 4: full sync |
| Operator decisions | Local intent queue only — single-operator overlay, no shared authority | Layer 4: remote-owned `operator_decision_heads` + CAS |
| Validation | Disabled (fields exist) | Layer 3: dry-run |
| Degraded-mode UX | Basic daemon-unavailable detection | Layer 4: three-state (healthy/degraded/offline) with distinct sync/daemon states |
| Web portal | Not present | Layer 4: read-only viewing portal |

### 1.14 Things NOT Implemented in Layer 2

- Cron scheduling (Layer 3)
- Resource arbiter (Layer 3)
- Worker-validator system (Layer 3/4)
- Sync engine (Layer 4)
- Web viewing portal (Layer 4)
- Multi-source marketplace search (Layer 5)
- Decision-grade landed cost (Layer 5)
- Bid workflow automation (Layer 6)
- Multi-user / RBAC (not in MVP)

### 1.15 Accessibility

- Do NOT `aria-live` every transcript line — announce only state transitions ("session stalled", "new opportunities available")
- Do NOT use color as the only signal for margin/risk or sync health — provide text labels and icons alongside
- Provide plain-text transcript mode for screen readers alongside ANSI terminal view
- Keyset pagination is accessible by default (stable focus targets, no shifting content)
- Deep links resolve through `process_aliases` so bookmarked IDs survive later merges

---

## 2. Shared UI Components & Manifest Rendering

### 2.1 `packages/ui` — Shared Presentational Components

**5 files.** These components are shared between the desktop control panel (Layer 2) and the web viewing portal (Layer 4). They are **pure presentational** — they receive typed props and render UI. They must NOT import data-fetching hooks, Supabase clients, Tauri invoke wrappers, daemon bridge code, or mutation orchestration.

**Boundary rule:** Share down to view-models, presentational components, and formatting helpers. Stop sharing at repositories, hooks, realtime subscriptions, and mutation flows. Data-fetching is surface-specific: desktop uses QueryService (SQLite), web uses Supabase client.

#### `index.ts` — Barrel Export

Re-exports all shared components. Consumers import from `@project/ui`.

#### `contract-card-frame.tsx` — Process Summary Card

Frame + slot composition for process/opportunity cards in list views.

```tsx
interface ContractCardFrameProps {
  summary: {
    processId: string
    title: string
    buyerName: string
    publishedDate: string
    deadlineDate: string | null
    estimatedValuePence: number | null
    lotCount: number
    accessRoute: string
    legalCategory: 'goods' | 'services' | 'works' | 'mixed' | 'unknown'
  }
  metrics: {
    pipelineState: 'pending' | 'qualifying' | 'enriching' | 'qualified' | 'review' | 'rejected'
    pricedCoverageBps: number | null
    netQualifiedCoverageBps: number | null
    grossMarginBps: number | null     // best candidate
    netMarginBasePct: number | null   // best candidate
    candidateCount: number
    quotedCandidateCount: number
  }
  decision?: {
    disposition: 'approve' | 'reject' | 'watch' | null
    isStale: boolean   // decisionBasisHash changed since decision
    decidedAt: string | null
  }
  rightRail?: React.ReactNode   // Slot for SyncAgeBadge, custom badges
  actions?: React.ReactNode     // Slot for desktop-only action buttons (null on web)
}
```

**Rendering rules:**
- All monetary values displayed in `£X,XXX` format (from integer pence)
- All percentages displayed from basis points (`bps / 100` for display)
- Stale decisions show warning indicator
- Keyset pagination: card order is stable (no shifting on live updates)

#### `manifest-viewer.tsx` — Opportunity Manifest Renderer

Renders Markdown opportunity manifests with section navigation sidebar.

```tsx
interface ManifestViewerProps {
  manifestMarkdown: string
  manifestVersion: string
  basisHash: string
  generatedAt: string
  sections?: string[]           // Parsed section headings for sidebar nav
  highlightBlockers?: boolean   // Visually highlight blocker sections
  onSectionClick?: (sectionId: string) => void
}
```

**Features:**
- Markdown rendering (via `marked` or equivalent)
- Table formatting for pricing/margin data
- Blocker highlighting (red badge on sections with critical blockers)
- Section navigation sidebar
- Copy-to-clipboard for individual sections
- Print-friendly layout

#### `pipeline-badge.tsx` — Pipeline State Indicator

```tsx
interface PipelineBadgeProps {
  state: 'pending' | 'qualifying' | 'enriching' | 'qualified' | 'review' | 'rejected' | 'error'
  validationState?: 'unverified' | 'approved' | 'flagged' | 'rejected'  // Layer 3+
  size?: 'sm' | 'md' | 'lg'
}
```

**Display rules:**
- Color + text label + icon (never color alone — accessibility)
- `validationState` badge shown only when not `unverified` (Layer 3+)
- Tooltip with timestamp of last state change

#### `sync-age-badge.tsx` — Data Freshness Indicator

```tsx
interface SyncAgeBadgeProps {
  meta: {
    mode: 'healthy' | 'browse_only' | 'recovery_required'
    daemonConnected: boolean
    syncHealth: 'healthy' | 'degraded' | 'offline'
    lastSnapshotAt?: string
    lastSyncAt?: string           // Layer 4: last successful Supabase push
  }
}
```

**Thresholds (Layer 4 — for web portal sync freshness):**
- Green: < 90 seconds since last sync
- Amber: 90s–5min
- Red: > 5min or daemon offline
- Text: "Data may be behind desktop local state."

**Layer 2 behavior:** Shows daemon connection status and local data freshness only. Sync fields are null.

### 2.2 `packages/manifests` — Opportunity Manifest Renderer

**2 files.** Deterministic opportunity manifest rendering from pipeline data. The canonical rendering lives here — the `manifest-generation` skill may contribute narrative synthesis, but this package is authoritative.

#### `render-opportunity.ts` — Manifest Renderer

```typescript
interface ManifestInput {
  process: ProcurementProcess
  notices: ContractNotice[]
  lots: NoticeLot[]
  candidates: ProductCandidate[]
  quotes: SupplierQuote[]
  pricingAssessments: CandidatePricingAssessment[]
  landedCosts: LandedCostEstimate[]
  pipelineAssessment: PipelineAssessment
  coverageMetrics: {
    pricedCoverageBps: number
    landedCostCoverageBps: number
    netQualifiedCoverageBps: number
    unresolvedMandatoryLots: number
  }
}

interface ManifestOutput {
  markdown: string
  basisHash: string           // SHA-256 of inputs — staleness detection
  sections: string[]          // Parsed section headings
  blockerCount: number
  generatedAt: string
}

function renderOpportunityManifest(input: ManifestInput): ManifestOutput
```

#### `opportunity.md.ts` — Markdown Template

Sections in the generated manifest:

| Section | Content |
|---------|---------|
| **Summary** | Process title, buyer, value, lot count, deadline, access route |
| **Lot Analysis** | Per-lot qualification result, addressable value, goods classification |
| **Product Candidates** | Per-lot candidate list with archetype, identity confidence, quoteability score |
| **Pricing Evidence** | Per-candidate quotes with source, match confidence, usability confidence, price-signal role |
| **Margin Estimate** | Gross margin gate result, landed cost breakdown, net margin band (low/base/high) |
| **Coverage Metrics** | `pricedCoverageBps`, `netQualifiedCoverageBps`, `unresolvedMandatoryLots` |
| **Risk Flags** | Critical blockers, low-confidence components, stale data warnings |
| **Recommendation** | Auto-qualified / manual review required / not qualified — with reasoning |

**Formatting rules:**
- All monetary values in `£X,XXX.XX` (from integer pence)
- All percentages from basis points
- Blockers shown prominently with red indicators
- Confidence shown as bands (high/usable/manual), not decimal numbers
- Component scores shown in panel format, not as a single composite number

### 2.3 Component Design Rules (Constitutional)

1. **`packages/ui` components must NOT import** `packages/db`, desktop bridge code, daemon modules, Supabase clients, or data-fetching hooks
2. **`packages/manifests` must NOT import** daemon modules or Supabase clients — it operates on typed inputs only
3. **Data-fetching is surface-specific:** Desktop uses `QueryService` (SQLite via `invoke()`); Web (Layer 4) uses Supabase client
4. **No boolean prop explosion:** Use frame + slot composition, not `<Card canReject canBid readOnly showSync ... />`
5. **View-model types shared; data-access types not shared:** `ContractCardFrameProps` is shared; `useContractsList()` is not
6. **All monetary display from integer pence;** all percentage display from integer basis points — no floating-point formatting paths

---

## 3. Pipeline Surface, Coverage & Decision Capture

### 3.1 Coverage Metrics — Operator-Facing Presentation

> **Ownership:** L0 §3 owns the full pipeline-side derivation and production logic. Strategy §10 owns the cross-layer interface summary. This section owns **Layer 2 presentation**: thresholds, display format, and operator-facing behavior.

**Coverage metric interface (compact recap for self-containment):**

| Field | Type | Meaning | Denominator |
|-------|------|---------|-------------|
| `pricedCoverageBps` | `INTEGER` (0-10000) | Fraction of qualifying lots with >=1 priced candidate | Total qualifying lots |
| `landedCostCoverageBps` | `INTEGER` (0-10000) | Fraction of qualifying lots with a landed cost estimate | Total qualifying lots |
| `netQualifiedCoverageBps` | `INTEGER` (0-10000) | Fraction of qualifying lots passing the net margin gate | Total qualifying lots |
| `unresolvedMandatoryLots` | `INTEGER` | Count of mandatory lots without resolution | N/A (absolute count) |
| `decisionBasisHash` | `TEXT` | SHA-256 of the pipeline data the operator would review | N/A (staleness key) |

Display: `bps / 100` for percentage (e.g., 7500 bps = "75.0%").

**Color thresholds for `netQualifiedCoverageBps`:**

| Range | Color | Label |
|-------|-------|-------|
| >= 7000 bps (70%+) | Green | Strong coverage |
| 3000-6999 bps (30-69%) | Amber | Partial coverage -- review recommended |
| < 3000 bps (< 30%) | Red | Low coverage -- manual review required |

Always show both color AND text label (accessibility). Auto-qualification guard: `netQualifiedCoverageBps >= 5000` (50%) required. Desktop surfaces "Coverage below threshold" indicator when not met.

### 3.2 Decision Capture -- `pending_operator_intents`

Local single-operator overlay. NOT the authoritative shared decision table (that is `operator_decision_heads` in Layer 4 via CAS).

**Dispositions:** `approve` | `reject` | `watch`

**Decision metadata captured on every intent:**

| Field | Purpose |
|-------|---------|
| `process_id` | Target process |
| `disposition` | approve / reject / watch |
| `reviewedBasisHash` | SHA-256 of pipeline data at decision time |
| `reviewedManifestHash` | SHA-256 of the manifest reviewed |
| `reviewedAtStageRunId` | Current `stage_run_id` at decision time |
| `idempotencyKey` | UUID for future CAS deduplication (Layer 4) |
| `decidedAt` | Timestamp |
| `status` | `pending` (always in L2 -- no remote flush yet) |

**Latest-per-process coalescing:** Only the latest intent per process matters. If operator changes `approve` to `reject`, the `reject` supersedes. Desktop immediately reflects the queued intent as the current decision. In L2, the local intent IS the decision. In L4, it becomes a queue entry flushed to Supabase via CAS.

### 3.3 Decision Staleness Detection

1. Operator decides -> `reviewedBasisHash` captured from `pipeline_assessments.decisionBasisHash`
2. Pipeline re-runs -> `decisionBasisHash` changes
3. Desktop compares: mismatch = **stale**

**Stale decision UX:**
- Warning indicator: "Decision may be outdated -- pipeline data changed since review"
- Operator can re-affirm (new intent with updated hash) or change disposition
- **Stale decisions block bid workflow** (Layer 6) -- must re-affirm first

**Staleness triggers:** New/updated quotes, re-run landed cost, new documents changing candidates, relation resolution changes, any upstream change modifying `decisionBasisHash`.

### 3.4 Coverage Rollup Module

`packages/intelligence/src/coverage/coverage-rollup.ts` -- runtime implementation of L0 §3.14 formulas.

```typescript
interface CoverageInput {
  lots: Array<{
    lotId: string
    qualificationResult: 'pass' | 'review' | 'reject'
    isMandatory: boolean
    candidates: Array<{
      candidateId: string
      hasPricing: boolean
      hasLandedCost: boolean
      passesNetGate: boolean
      candidateValueShareBps: number
    }>
  }>
}

interface CoverageOutput {
  pricedCoverageBps: number
  landedCostCoverageBps: number
  netQualifiedCoverageBps: number
  unresolvedMandatoryLots: number
  decisionBasisHash: string
}

function computeCoverageRollup(input: CoverageInput): CoverageOutput
```

### 3.5 Keyset Pagination

All list views use **keyset/cursor pagination** (not offset). Offset causes row jumping on live updates.

- Client sends: `{ after: lastSeenCursor, limit: 25, filters: {...} }`
- Server returns: `{ items: [...], nextCursor: string | null, hasMore: boolean }`
- Cursor encodes sort position (e.g., `created_at + id`)
- Stable focus targets -- items don't shift while user reads

---

## 4. Testing & Acceptance

### 4.1 Definition of Done

| # | Criterion | Verification |
|---|----------|-------------|
| 1 | **FTS ingestion skill** scrapes live notices from Find a Tender | Automated: agent E2E test produces notices |
| 2 | **Contracts Finder ingestion skill** scrapes live notices | Automated: agent E2E test produces notices |
| 3 | **Notice normalization** produces canonical internal records | Automated: normalized records match expected schema |
| 4 | **Goods/services classifier** correctly categorizes notices | Automated: unit tests for CPV-based classification |
| 5 | **Desktop control panel** launches with QueryService + DaemonBridge + bootstrap state machine | Manual: app launches, state machine detects daemon |
| 6 | **Notice list view** with keyset pagination displays live pipeline output | E2E: Playwright test navigates list, verifies pagination |
| 7 | **Opportunity detail view** shows margin bands, product candidates, supplier quotes | E2E: Playwright test opens detail, verifies data |
| 8 | **Decision capture** (reject/watch/approve) persists locally in `pending_operator_intents` with reviewed-basis metadata | E2E: Playwright test captures decision, verifies DB |
| 9 | **Agent session monitoring** page shows live terminal output via Channel\<T\> streaming | Manual: observe live PTY output during ingestion |
| 10 | **Coverage metrics** displayed: `pricedCoverageBps`, `netQualifiedCoverageBps` with color thresholds | E2E: verify badges render with correct colors |
| 11 | **Decision-basis staleness** indicators flag stale decisions | Automated: re-run pipeline, verify staleness flag |
| 12 | **Single operator** runs daily for 1 week and reports genuine opportunities surfaced | Manual: operator validation over 5-7 working days |

### 4.2 Test Files

| File | Type | What It Tests |
|------|------|--------------|
| `agent_procurement_ingestion.rs` | Agent E2E | Real FTS skill session: PTY spawn, notice extraction, result contract validation, cleanup verdict |
| `agent_supplier_enrichment.rs` | Agent E2E | Real eBay enrichment session: search execution, quote extraction, dual confidence scoring |
| `agent_health_detection.rs` | Agent E2E | Stall detection (no output + no progress), auth challenge detection, cleanup escalation |
| `agent_error_manifest.rs` | Agent E2E | Error manifest creation on various failure types (skill failed, contract invalid, suspect output) |
| `apps/desktop/e2e/contracts.spec.ts` | Browser E2E | Desktop control panel: list browsing, detail view, decision capture, session monitoring, error view |

### 4.3 Test Scenarios

| Scenario | Expected Behavior |
|----------|------------------|
| Notice with 12 lots | System processes each lot independently; detail view shows per-lot results |
| Mixed contract with separable goods | Goods-addressable value used for margin computation, not whole contract value |
| Expired notice | Correctly filtered; not displayed as active opportunity |
| Agent session crashes mid-run | Error manifest created; operator sees failure context in desktop error view; cleanup verdict is `forced` |
| Decision on process, then pipeline re-runs | Decision marked stale; warning indicator shown; bid workflow blocked |
| DPS/Dynamic Market notice | 30-day expiry filter NOT applied; notice remains active |
| Agent auth challenge during ingestion | `AGENT_AUTH_CHALLENGE` classified; operator alerted via desktop |
| Daemon unavailable at desktop launch | Bootstrap state machine falls back to browse-only mode with clear banner |

### 4.4 Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Source website changes break FTS/CF skills | High | Medium | Error manifest flags failures; skill update cycle; page fingerprint drift detection |
| Desktop UX reveals pipeline quality issues | Medium | Medium | Expected -- this is why desktop comes before hardening (Layer 3). Operator feedback drives pipeline improvement. |
| Single-source eBay pricing not representative | High | Medium | Known limitation; Layer 5 adds Alibaba/1688 for wholesale pricing |
| QueryService read performance on large datasets | Low | Low | SQLite reads are fast; keyset pagination prevents full scans; short-lived read transactions |
| Desktop-daemon IPC failure | Medium | Medium | Bootstrap state machine detects failure; browse-only fallback; reconnect overlay |

---

## 5. Open Questions, Forward Compatibility & Dependencies

### 5.1 Open Questions

**§10 Further Research #5: Auto-qualification coverage floor calibration**
- Default `process_coverage_floor_bps` is 5000 (50%). May be too aggressive for mixed contracts with many service lots.
- **Layer 2 impact:** Affects coverage rollup thresholds and auto-qualification guard in the desktop UI.

**§10 Further Research #1: Exact procurement source list by commercial importance**
- Which portals matter for daily operator value beyond FTS and Contracts Finder?
- **Layer 2 impact:** Determines whether PCS or other sources should be added before Layer 5.

### 5.2 Forward-Compatibility Provisions

| Provision | What Exists in L2 | Target Layer |
|-----------|-------------------|-------------|
| Bootstrap state machine health modes | Detects daemon-unavailable; browse-only fallback | Layer 4: three-state degraded UX (healthy/degraded/offline with distinct sync/daemon states) |
| `packages/ui` frame + slot pattern | 5 components shared between desktop and web | Layer 4: web viewing portal uses same components |
| Coverage metrics on `pipeline_assessments` | Populated and displayed in desktop | Layer 4: sync projections push these to Supabase |
| `stage_run_id` on all stage outputs | Populated from Layer 1 | Layer 4: sync batch boundaries use `stage_run_id` |
| CAS payload on `pending_operator_intents` | `process_id`, disposition, reviewed-basis metadata, idempotency key | Layer 4: remote-owned `operator_decision_heads` with CAS flush |

### 5.3 Dependencies

- **Layer 0:** Intelligence pipeline, skills (5 initial), contracts-schema, SQLite schema
- **Layer 1:** Daemon runtime, artifact capture, process-group supervision, run manifests, error manifests

**Desktop configuration (from §9.2):**

| Config File | Purpose |
|-------------|---------|
| `apps/desktop/src-tauri/capabilities/default.json` | Allowed shell/sidecar commands only |
| `apps/desktop/src-tauri/tauri.conf.json` | Sidecar and desktop build configuration |

### 5.4 ADRs Addressed

| ADR | Decision | Layer 2 Action |
|-----|----------|---------------|
| **ADR-003** | Tauri v2 + separate daemon (not embedded) | Desktop shell created with QueryService + DaemonBridge architecture |
| **ADR-008** | Desktop = control panel (full read/write/trigger) | Full operator surface: list, detail, decide, trigger, monitor |
| **ADR-001** | Skills as primary data retrieval | FTS + Contracts Finder ingestion skills live |
