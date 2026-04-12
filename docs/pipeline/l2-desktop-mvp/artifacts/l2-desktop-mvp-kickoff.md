# L2 Desktop MVP — Agent Kickoff Prompt

> **Copy this entire file into a fresh Claude Code / Codex session to onboard an implementation agent.**

---

## 1. Your Mission

You are implementing **Layer 2 — Single-Operator Desktop MVP** of the Procurement Intelligence Platform. This is the **first user-facing layer**: a Tauri v2 desktop control panel with shared UI packages, live session streaming, decision capture, and a new Contracts Finder ingestion skill.

Layer 2 builds on **Layer 0** (SQLite schema, TypeScript contracts-schema, intelligence pipeline, 5 skills, evaluation harness) and **Layer 1** (Rust daemon — IPC, PTY sessions, artifact capture, job runner, error manifests, commit fence). **L2 does NOT modify L0/L1 production code** — it extends the daemon's IPC surface and adds the desktop shell + React SPA + shared packages.

**Deliverable:** Working desktop control panel at `apps/desktop/` (~28+ files), shared packages at `packages/ui` + `packages/manifests` + `packages/intelligence`, Contracts Finder skill at `skills/procurement-ingestion/contracts-finder/`, and daemon extensions for persistent serving + command dispatch.

**Key constraint:** This is a **MIXED-STACK layer**. Both `cargo test` and `bun run test` / `bunx playwright test` must pass. Rust 2024 (nightly) + TypeScript (Bun).

**Completion milestone:** Single operator runs system daily for 1 week, making real accept/reject/watch decisions on genuine procurement opportunities via the desktop UI.

**Organization: FLAT.** No team hierarchy. Pick the next ready bead, implement it, move on. Never ask "what should I work on next?" — run `br ready` and pick the highest-priority unblocked task.

**Coordination: AGENTMAIL-FIRST.** Before touching ANY code: register with AgentMail, check inbox, announce which bead you're claiming, reserve the files you'll edit. This prevents conflicts and keeps the swarm coherent.

**Beads are SELF-CONTAINED.** Each bead has full file paths, type definitions, test specs, acceptance criteria, and dependency info. Implement from the bead alone. The spec is only needed as a tiebreaker.

---

## 2. What Layer 2 Does NOT Include

- Cron scheduling (Layer 3)
- Resource arbiter (Layer 3)
- Validation pipeline enforcement (Layer 3)
- Sync engine / Supabase push-pull (Layer 4)
- Web viewing portal (Layer 4)
- Multi-source search beyond FTS + Contracts Finder (Layer 5)
- Decision-grade landed cost (Layer 5)
- Bid automation (Layer 6)

---

## 3. Source of Truth Reading List (MANDATORY)

Read these IN THIS ORDER before writing any code:

1. **`AGENTS.md`** — **READ FULLY** — coding conventions, toolchain (`br` not `bd`, Bun not npm), git rules, AgentMail, beads workflow, session protocol
2. **Spec (SOT #1)** — `docs/strategy/layers/layer-2-single-operator-desktop.md` — **READ for tiebreaker** — 118 L2-FR-* requirement IDs across BOOT/QS/DB/CH/PAGE/STATE/UI/MAN/CF/DEV subsystems
3. **TDD (SOT #2)** — `docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-tdd.md` — **REFERENCE** — 158 total checks: 7 static CI gates, 61 Playwright E2E, 12 Tauri verification, 78 mixed backend/package/agent tests
4. **Strategy (SOT #3)** — `docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-strategy.md` — **REFERENCE** — 12 phases, 28 beads, phase ordering and gate criteria
5. **Bootstrap** — `docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-bootstrap.md` — **REFERENCE** — environment setup, scaffold plan, runner commands
6. **Manifest** — `docs/pipeline/l2-desktop-mvp/manifest.yaml` — **REFERENCE** — all paths, bead prefix `l2`, tech stack config
7. **Key codebase files** (study for integration surface):
   - `crates/daemon/src/main.rs` — daemon entry point, CLI args, RuntimeBootstrap, IPC server setup
   - `crates/daemon/src/ipc/handshake.rs` — HandshakeContext, Hello/Welcome/Rejection protocol, LifecyclePhase
   - `crates/daemon/src/agents/adapter.rs` — AgentAdapter trait, ControlEvent, OutputFrame, ExecutionMode
   - `crates/daemon/src/db/sqlite.rs` — StateDb, commit_clock pattern, `with_write_transaction`
   - `crates/daemon/src/jobs/runner.rs` — JobRunner, ClaimedJob, MutationAck, PersistSuccessRequest
   - `crates/daemon/src/pty/session.rs` — SessionPhase (7 phases), PtyCommand, SpawnOptions
   - `crates/daemon/src/sessions/reaper.rs` — CleanupVerdict (Clean/Forced/Leaked), ReaperConfig
   - `crates/daemon/src/agents/artifacts.rs` — ArtifactStore, RunManifestDraft, StoredManifest
   - `migrations/0001_sqlite_state_initial.sql` — L0 schema (47 tables)
   - `migrations/0003_sqlite_runtime.sql` — L1 additions (commit_clock, agent_session_logs, error_manifests)

---

## 4. Architecture Overview

### Desktop Architecture (Three Communication Channels)

```
┌──────────────────────────────────────────────────────────────────┐
│  React SPA (Vite)                                                │
│  ┌─────────────┐  ┌─────────────────┐  ┌──────────────────────┐ │
│  │ Pages       │  │ Stores/Hooks    │  │ Shared Packages      │ │
│  │ (6 routes)  │  │ (session-reg,   │  │ (ui, manifests,      │ │
│  │             │  │  control-plane) │  │  intelligence)       │ │
│  └──────┬──────┘  └────────┬────────┘  └──────────────────────┘ │
│         │                  │                                     │
│         └──────────┬───────┘                                     │
│                    │ transport.ts (single entry point)            │
│         ┌──────────┴──────────┐                                  │
│         │                     │                                  │
│    Tauri invoke()       Dev HTTP/WS                              │
│    (production)         (browser dev)                             │
└─────────┬─────────────────────┬──────────────────────────────────┘
          │                     │
┌─────────┴─────────┐  ┌───────┴──────────┐
│ Tauri Rust Shell   │  │ Vite Dev Bridge  │
│ (src-tauri/)       │  │ (dev only)       │
│ ┌───────────────┐  │  └───────┬──────────┘
│ │ QueryService  │  │          │
│ │ (read-only    │  │          │
│ │  SQLite)      │  │          │
│ ├───────────────┤  │          │
│ │ DaemonBridge  │──┼──────────┘
│ │ (UDS → daemon)│  │
│ ├───────────────┤  │
│ │ Channel<T>    │  │
│ │ (live streams)│  │
│ └───────────────┘  │
└────────────────────┘
          │
     UDS / named-pipe
          │
┌─────────┴─────────┐
│  Daemon (L1)       │
│  (separate process)│
└────────────────────┘
```

### Transport Split

| Channel | Path | Purpose | L1 Type |
|---------|------|---------|---------|
| Snapshot reads | `QueryService` → read-only SQLite | Page-scoped queries, `QueryEnvelope<T>` with `snapshotSeq` | `StateDb.current_commit_seq()` |
| Mutations | `DaemonBridge` → UDS → daemon | Commands (`trigger_ingestion`, `capture_decision`, etc.) → `MutationAck` with `visibleAfterSeq` | `JobRunner.persist_success()` |
| Live streams | Daemon → `Channel<T>` → React | `SessionFrame` (lifecycle/pty/stdout/stderr/event/gap) | `ControlEvent`, `OutputFrame` |

### Commit Fence Pattern (CRITICAL)

Every mutation follows: frontend sends command → daemon commits to SQLite, increments `commit_seq`, returns `MutationAck` with `visibleAfterSeq` → frontend does NOT treat mutation as visible until `snapshotSeq >= visibleAfterSeq`. No optimistic UI.

### Key L1 Types the Desktop Integrates With

- **`SessionPhase`**: `Created | Spawned | Active | Terminating | Exited | Reaped | Archived`
- **`CleanupVerdict`**: `Clean | Forced | Leaked`
- **`ControlEvent`**: `Started | Checkpoint | Progress | AuthChallenge | Warning | FinalResultReady | Exited`
- **`OutputFrame`**: `Pty(Bytes) | Stdout(Bytes) | Stderr(Bytes)`
- **`ExecutionMode`**: `Pty | Contract`
- **`MutationAck`**: `{ accepted, visibleAfterSeq, jobId?, sessionId? }`
- **`HandshakeContext`**: `{ protocolVersion, instanceId, schemaEpoch, commitSeq, healthState, activeSessions }`
- **`ReconnectOutcome`**: `FreshConnection | SameDaemonReconnect | DaemonRestartInvalidated`

### Contracts Finder Skill Structure

Must mirror the existing FTS skill at `skills/procurement-ingestion/fts/`:
- `SKILL.md` — same top-level sections
- `checkpoint.schema.json` / `result.schema.json` — same canary + golden pairing
- `canary.inputs.jsonl` / `golden.assertions.yaml` — same run-manifest frontmatter contract

---

## 5. AgentMail Registration (BEFORE ANY CODE)

**Project key:** `/Users/alinaaitova/Desktop/Testproject1`

### 5a. Boot Your Session

```
# Option A: Granular
ensure_project(human_key="/Users/alinaaitova/Desktop/Testproject1")
register_agent(
  project_key="/Users/alinaaitova/Desktop/Testproject1",
  program="l2-desktop-executor",
  model="<your-model>",
  task_description="Implementing L2 desktop MVP beads"
)

# Option B: Macro (faster)
macro_start_session(
  project_key="/Users/alinaaitova/Desktop/Testproject1",
  program="l2-desktop-executor",
  model="<your-model>"
)
```

### 5b. Situational Awareness (Check Before Picking Work)

```
# Check inbox — other agents may have claimed beads or flagged issues
fetch_inbox(
  project_key="/Users/alinaaitova/Desktop/Testproject1",
  agent_name="<your-name>",
  include_bodies=true
)

# Search for active work
search_messages(
  project_key="/Users/alinaaitova/Desktop/Testproject1",
  query="in_progress OR claimed"
)
```

```bash
# Current work queue
br ready --json

# Full bead state
br list --json
```

**KEY PRINCIPLE:** If another agent claimed a bead, DO NOT take it. Pick a different ready bead.

### 5c. Claim Protocol (Announce + Reserve)

```bash
# 1. Claim in beads
br update <id> --status=in_progress
```

```
# 2. Reserve files (MANDATORY)
file_reservation_paths(
  project_key="/Users/alinaaitova/Desktop/Testproject1",
  agent_name="<your-name>",
  paths=[<list of file paths from bead>],
  ttl_seconds=3600,
  exclusive=true,
  reason="<bead-id>"
)

# 3. Announce via mail
send_message(
  project_key="/Users/alinaaitova/Desktop/Testproject1",
  sender_name="<your-name>",
  to=["<peers>"],  # from list_contacts — NEVER "broadcast"
  thread_id="<bead-id>",
  subject="[<bead-id>] Claimed: <title>",
  body_md="Starting work on <bead-id>. Editing:\n- <file list>\n\nExpected duration: ~1-2 hours.",
  ack_required=true
)
```

**FILE_RESERVATION_CONFLICT:** Pick a different ready bead that touches different files.

### 5d. Beads-AgentMail Mapping

| Concept | Value |
|---------|-------|
| Mail `thread_id` | Bead ID (e.g., `testproject1-dr6.28`) |
| Mail subject prefix | `[testproject1-dr6.28]` |
| File reservation `reason` | Bead ID |
| Commit message prefix | Bead ID |

---

## 6. Beads & Triage Tools

**IMPORTANT:** Use `br` (beads_rust), NOT `bd`. Only pick beads with `l2-` prefix in their title. Bead IDs are `testproject1-dr6.*`.

```bash
br ready              # Beads with no blockers
br ready --json       # Machine-readable
br show <id>          # Full bead — THE BEAD IS SELF-CONTAINED
br update <id> --status=in_progress
br close <id> --reason "Completed"
br sync --flush-only  # Export to JSONL, then git add .beads/
```

```bash
bv --robot-triage     # Ranked picks, health
bv --robot-next       # Single top pick
bv --robot-plan       # Parallel execution tracks
```

---

## 7. Current State (28 beads, 12 phases)

### Phase 1: Workspace Roots And Desktop Package Shells (2 beads)
```
testproject1-dr6.28  l2-01a: Bun/TS Workspace Root Files              [READY]
testproject1-dr6.1   l2-01b: Desktop Package Shell And Native Crate   [blocked by l2-01a]
```

### Phase 2: Desktop Tooling Configs And Native Baselines (2 beads)
```
testproject1-dr6.2   l2-02a: Frontend Tooling Configs                 [blocked by l2-01b]
testproject1-dr6.3   l2-02b: Native Tauri Baseline Config             [blocked by l2-01b]
```

### Phase 3: Daemon Service Mode And Handshake Wire (2 beads)
```
testproject1-dr6.4   l2-03a: Persistent Daemon Hosting Path           [blocked by l2-01a]
testproject1-dr6.5   l2-03b: Additive Handshake And Descriptor        [blocked by l2-03a]
```

### Phase 4: Daemon Control, Jobs, And Session Command Lane (3 beads)
```
testproject1-dr6.6   l2-04a: Command Router And Control Handler       [blocked by l2-03b]
testproject1-dr6.7   l2-04b: Jobs Handler And Runner Reuse            [blocked by l2-04a]
testproject1-dr6.8   l2-04c: Session Control Handler                  [blocked by l2-04a]
```

### Phase 5: Shared Data Packages (2 beads)
```
testproject1-dr6.9   l2-05a: Manifest Renderer And Template           [blocked by l2-01a]
testproject1-dr6.10  l2-05b: Coverage Rollup And Qualification        [blocked by l2-01a]
```

### Phase 6: Shared UI Surfaces (2 beads)
```
testproject1-dr6.11  l2-06a: List And Detail Shared Cards And Badges  [blocked by l2-01a]
testproject1-dr6.12  l2-06b: Sanitized Manifest And Error Renderers   [blocked by l2-01a]
```

### Phase 7: Frontend Transport And App Shell Foundation (2 beads)
```
testproject1-dr6.13  l2-07a: Transport Shim, Dev Bridge, UI Barrel    [blocked by l2-02a, l2-02b, l2-06a, l2-06b]
testproject1-dr6.14  l2-07b: Control-Plane Hook And App Shell         [blocked by l2-07a]
```

### Phase 8: Desktop Rust Bridge And Native Bootstrap (3 beads)
```
testproject1-dr6.15  l2-08a: Workspace Update And Native Bootstrap    [blocked by l2-04b, l2-04c, l2-07b]
testproject1-dr6.16  l2-08b: QueryService Read-Only Bridge            [blocked by l2-08a]
testproject1-dr6.17  l2-08c: DaemonBridge RPC Proxy                   [blocked by l2-08a]
```

### Phase 9: Read-Only Operator Pages (2 beads)
```
testproject1-dr6.18  l2-09a: Contract List And Detail Pages           [blocked by l2-05a, l2-05b, l2-07b, l2-08b, l2-08c]
testproject1-dr6.19  l2-09b: Errors And Daemon Status Pages           [blocked by l2-07b, l2-08b, l2-08c]
```

### Phase 10: Streaming And Session UX Lane (3 beads)
```
testproject1-dr6.20  l2-10a: Session Registry And Buffer Stores       [blocked by l2-04c, l2-07b, l2-08c]
testproject1-dr6.21  l2-10b: Session Hooks And List Page              [blocked by l2-08b, l2-10a]
testproject1-dr6.22  l2-10c: Session Viewer And Replay                [blocked by l2-08b, l2-10b]
```

### Phase 11: Decisions, Coverage, Staleness, And Operator Validation (2 beads)
```
testproject1-dr6.23  l2-11a: Decision Migration And Handler           [blocked by l2-05b, l2-08b, l2-09a, l2-09b]
testproject1-dr6.24  l2-11b: Error Actions, Page Wiring, Validation   [blocked by l2-11a]
```

### Phase 12: CF Skill, Hardening, And Milestone Gate (3 beads)
```
testproject1-dr6.26  l2-12a: Contracts Finder Skill Package           [blocked by l2-04b]
testproject1-dr6.27  l2-12b: Bridge-Failure And Security Hardening    [blocked by l2-09a, l2-09b, l2-10c, l2-11b]
testproject1-dr6.29  l2-12c: Native Parity And Milestone Gate         [blocked by l2-11b, l2-12a, l2-12b]
```

**Entry point:** `testproject1-dr6.28` (l2-01a: Bun/TS Workspace Root Files) — the ONLY task with no blockers at start.

**Parallel tracks after Phase 1:** Phases 3+5+6 can start in parallel once l2-01a completes. Phase 3 is the Rust daemon lane; Phases 5+6 are the TypeScript shared packages lane.

---

## 8. Quality Gates (Actual Commands)

Run these before EVERY commit:

```bash
# Rust gates (ALWAYS — even if you only changed TS files, daemon code must still compile)
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo fmt --check
cargo test

# TypeScript gates (when TS files changed)
bun install
bun run build
bun run lint
bun run test

# Playwright gates (when UI files changed)
bunx playwright test

# Bug scanner (scope to changed files)
ubs <changed-files>
```

### Constitutional Constraints

- **`#![forbid(unsafe_code)]`** on ALL `apps/desktop/src-tauri/src/*.rs` and test files
- **No `unwrap()` in production code** — use `?`, `expect()` with context, or explicit error handling
- **Integer pence/bps** — no floating-point money
- **No mixed-write tables** — local SQLite is operational authority
- **Bun, not npm/yarn/pnpm** — for all TypeScript
- **`br`, not `bd`** — for beads CLI

---

## 9. Execution Loop

For each bead, follow this exact sequence:

1. **Check inbox** — `fetch_inbox(project_key, agent_name, include_bodies=true)`
2. **Find ready work** — `br ready` (only `l2-*` title prefix beads)
3. **Read the bead** — `br show <id>` (self-contained)
4. **Study integration surface** — read existing code the bead touches
5. **Claim** — `br update <id> --status=in_progress`
6. **Reserve files** — `file_reservation_paths(...)` with exclusive=true
7. **Announce** — `send_message(...)` with thread_id=bead-id
8. **Implement** — write code following bead specs exactly
9. **Verify** — run ALL applicable quality gates (Rust AND TypeScript as applicable)
10. **Close bead** — `br close <id> --reason "Completed"`
11. **Sync + Commit + Push** — `br sync --flush-only && git add .beads/ <files> && git commit -m "<bead-id>: <summary>" && git pull --rebase && git push`
12. **Release reservations** — `release_file_reservations(project_key, agent_name)`
13. **Announce completion** — `send_message(...)` with `[<bead-id>] Completed: <title>`
14. **Loop** — go back to step 1. Take initiative. Don't wait. Don't ask.

---

## 10. Session Landing Protocol (MANDATORY)

When fully blocked (no ready `l2-*` beads) or ending your session:

```bash
# 1. Run quality gates on anything you changed
cargo check --all-targets && cargo clippy --all-targets -- -D warnings && cargo fmt --check && cargo test
bun run build && bun run lint && bun run test  # if TS changed

# 2. File issues for any discovered work
br create --title="..." --type=task --priority=2

# 3. Close finished beads
br close <id> --reason "Completed"

# 4. Sync and push (MANDATORY)
br sync --flush-only
git add .beads/ <other-changed-files>
git commit -m "<bead-id>: <description>"
git pull --rebase
git push
git status  # MUST show "up to date with origin"
```

```
# 5. Release all file reservations
release_file_reservations(
  project_key="/Users/alinaaitova/Desktop/Testproject1",
  agent_name="<your-name>"
)

# 6. Announce session handoff
send_message(
  project_key="/Users/alinaaitova/Desktop/Testproject1",
  sender_name="<your-name>",
  to=["<peers>"],  # from list_contacts — NEVER "broadcast"
  subject="Session handoff: <your-name> signing off",
  body_md="## Completed\n- <beads done>\n\n## In Progress\n- None (all pushed)\n\n## Notes for next agent\n- <any context>",
  importance="normal"
)
```

**CRITICAL:** Work is NOT complete until `git push` succeeds. NEVER stop before pushing.

---

## 11. Phase Structure & Parallel Execution

```
Phase 1 (l2-01a → l2-01b)
  ├── Phase 2 (l2-02a ∥ l2-02b)
  │     └── Phase 7 (l2-07a → l2-07b) ──┐
  │                                       │
  ├── Phase 3 (l2-03a → l2-03b)          │
  │     └── Phase 4 (l2-04a → l2-04b ∥   │
  │                           l2-04c)     │
  │           ├── Phase 12a (l2-12a)      │
  │           └──────────────────────┐    │
  │                                  ▼    ▼
  ├── Phase 5 (l2-05a ∥ l2-05b)   Phase 8 (l2-08a → l2-08b ∥ l2-08c)
  │                                  │
  └── Phase 6 (l2-06a ∥ l2-06b)     ├── Phase 9 (l2-09a ∥ l2-09b)
                                     ├── Phase 10 (l2-10a → l2-10b → l2-10c)
                                     └── Phase 11 (l2-11a → l2-11b)
                                               └── Phase 12 (l2-12b → l2-12c)
```

---

## 12. Critical Rules

1. **Beads are self-contained** — implement from the bead alone
2. **Spec is the tiebreaker** — when bead and spec disagree, spec wins
3. **One bead = one commit** — atomic, traceable changes
4. **No fake tests** — tests must exercise real logic with meaningful assertions
5. **No optimistic UI** — mutation is NOT visible until `snapshotSeq >= visibleAfterSeq`
6. **QueryService is read-only** — `PRAGMA query_only = ON`, no writes
7. **Dev bridge matches Tauri** — same code paths, never a Vite-only fork of business logic
8. **Shared packages are hermetic** — no daemon IPC, no backend deps, fixture-only tests
9. **Security uses hostile fixtures** — XSS payloads, ANSI escapes, not benign content
10. **AgentMail before code** — register, check inbox, announce, reserve, THEN implement
11. **Phase gates are hard barriers** — dependencies enforce via `br ready`
12. **Use `br`, not `bd`** — this project uses beads_rust CLI
13. **Bun, not npm** — for all TypeScript layers

---

## 13. Multi-Agent File Conflict Rule

When you see uncommitted changes you didn't make — **this is normal**. Other agents work simultaneously.

- NEVER stash, revert, overwrite, or disturb changes made by other agents
- Treat them identically to changes you made yourself
- If there's a merge conflict during `git pull --rebase`, resolve carefully — don't discard either side

---

## Quick Reference

### Documents

| Resource | Path |
|----------|------|
| Agent conventions | `AGENTS.md` |
| Spec (SOT #1) | `docs/strategy/layers/layer-2-single-operator-desktop.md` |
| TDD (SOT #2) | `docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-tdd.md` |
| Strategy (SOT #3) | `docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-strategy.md` |
| Bootstrap | `docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-bootstrap.md` |
| Manifest | `docs/pipeline/l2-desktop-mvp/manifest.yaml` |

### Phase Summary

| Phase | Beads | Depends On |
|-------|-------|------------|
| 1: Workspace Roots | l2-01a, l2-01b | none |
| 2: Tooling Configs | l2-02a, l2-02b | Phase 1 |
| 3: Daemon Service | l2-03a, l2-03b | l2-01a |
| 4: Daemon Commands | l2-04a, l2-04b, l2-04c | Phase 3 |
| 5: Shared Data Pkgs | l2-05a, l2-05b | l2-01a |
| 6: Shared UI | l2-06a, l2-06b | l2-01a |
| 7: Transport + Shell | l2-07a, l2-07b | Phases 2, 6 |
| 8: Native Bridge | l2-08a, l2-08b, l2-08c | Phases 4, 7 |
| 9: Operator Pages | l2-09a, l2-09b | Phases 5, 7, 8 |
| 10: Session UX | l2-10a, l2-10b, l2-10c | Phases 4, 7, 8 |
| 11: Decisions | l2-11a, l2-11b | Phases 5, 8, 9 |
| 12: CF + Hardening | l2-12a, l2-12b, l2-12c | Phases 4, 9-11 |

### Tools

| Tool | Key Commands |
|------|-------------|
| `br` (beads) | `br ready`, `br show <id>`, `br update <id> --status=in_progress`, `br close <id>`, `br sync --flush-only` |
| `bv` (triage) | `bv --robot-triage`, `bv --robot-next`, `bv --robot-plan` |
| `ubs` (bugs) | `ubs <files>`, `ubs $(git diff --name-only --cached)` |
| Rust | `cargo check --all-targets`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`, `cargo test` |
| Bun/TS | `bun install`, `bun run build`, `bun run lint`, `bun run test`, `bunx playwright test` |
| AgentMail | `ensure_project`, `register_agent`, `fetch_inbox`, `send_message`, `file_reservation_paths`, `release_file_reservations` |
