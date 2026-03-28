# L2 Desktop MVP — Create TDD

## First Action

Invoke the `/spec-to-tdd` skill.

---

## Your Mission

Create the **Test-Driven Development specification** (SOT #2) and **Bootstrap document** for Layer 2 (Single-Operator Desktop MVP). The TDD maps every `L2-FR-*` requirement to concrete test IDs with exact assertions, organizes them into CI tiers, and produces a coverage matrix proving zero gaps. The bootstrap document defines environment setup: Bun workspace config, Tauri project scaffolding, Playwright configuration, and test harness setup.

This is a **mixed-stack layer** (Rust + TypeScript) with three distinct testing surfaces:
- **Playwright browser E2E** (Tier 1) — primary UI testing via dev bridge at `localhost:5173`
- **Tauri verification** (Tier 2) — desktop-specific behaviors requiring full Tauri build
- **Agent E2E in Rust** (Tier 3) — skill runtime tests via daemon test harness

---

## Source Documents (Read In This Order)

### Primary Input (CREATE FROM THIS)
```
docs/strategy/layers/layer-2-single-operator-desktop.md    — Spec (SOT #1, validated with L2-FR-* IDs) — READ FULLY
```

### Source of Truth Context
```
docs/strategy/layer-strategy.md                            — Cross-layer contracts, stage scope matrix — SKIM §9-§11
docs/strategy/cross-reference-index.md                     — Topic lookup — REFERENCE
```

### Codebase Context (STUDY FOR PATTERNS)
```
crates/daemon/src/ipc/handshake.rs                         — IPC protocol L2 extends — understand command dispatch
crates/daemon/src/agents/adapter.rs                        — AgentAdapter trait — understand event model for streaming tests
crates/daemon/src/jobs/runner.rs                           — Job Runner — understand stage_run_id for commit fence tests
crates/daemon/src/db/sqlite.rs                             — commit_clock / commit_seq — critical for fence assertions
crates/daemon/tests/agent_pty_session.rs                   — L1 agent E2E test pattern — STUDY for Tier 3 test structure
crates/daemon/tests/ipc_handshake.rs                       — L1 IPC test pattern — STUDY for DaemonBridge test structure
migrations/0001_sqlite_state_initial.sql                   — Full L0 schema — verify table/column names in test assertions
migrations/0003_sqlite_runtime.sql                         — L1 schema additions — verify integration surface
skills/procurement-ingestion/fts/SKILL.md                  — FTS skill standard — CF skill should match
```

### Quality Bar (MATCH OR EXCEED)
```
docs/pipeline/l1-daemon-core/artifacts/l1-daemon-core-tdd.md        — L1 TDD (150 test IDs, 4 tiers) — MATCH THIS QUALITY
docs/pipeline/l1-daemon-core/artifacts/l1-daemon-core-bootstrap.md  — L1 bootstrap — MATCH THIS QUALITY
```

---

## Key Context From Codebase Research

- **L1 has 150 test IDs across 4 tiers** (T1 <90s, T2 <4min, T3 <20min, T4 <30min) — L2 TDD should follow the same tier model but adapted for mixed stack
- **L1 test files are in `crates/daemon/tests/`** (10 integration test files + 6 support files) — L2 adds Tier 1 Playwright specs in `apps/desktop/e2e/` and extends Tier 3 Rust tests
- **L1 uses `#[test]` and `#[tokio::test]`** for Rust — L2 adds Playwright test runner for browser tests and Vitest for unit tests
- **The dev bridge (§1.8) enables Playwright testing** — tests interact with `localhost:5173` which proxies to real daemon via Vite middleware
- **L2 defines 16 Definition of Done criteria** (§4.2), **11 test files** (§4.3), **15 test scenarios** (§4.4) — all must be covered by test IDs
- **L2 has 3 Rust-side Tauri files** (`main.rs`, `query_service.rs`, `daemon_bridge.rs`) — these need integration testing against a real daemon

{ENRICHMENT: This section will be updated after Batch 1 with actual L2-FR-* requirement IDs, subsystem count, and section structure from the validated spec.}

---

## Step-by-Step Workflow

### Step 0: Read Source Documents (MANDATORY — before doing anything)
Read these IN THIS ORDER:
1. `docs/strategy/layers/layer-2-single-operator-desktop.md` — L2 spec — **READ FULLY**
2. `docs/pipeline/l1-daemon-core/artifacts/l1-daemon-core-tdd.md` — L1 TDD quality bar — **SKIM structure, tier model, coverage matrix format**
3. `docs/pipeline/l1-daemon-core/artifacts/l1-daemon-core-bootstrap.md` — L1 bootstrap quality bar — **SKIM structure**
4. `crates/daemon/tests/agent_pty_session.rs` — L1 agent E2E pattern — **STUDY FOR PATTERNS**
5. `crates/daemon/tests/ipc_handshake.rs` — L1 IPC test pattern — **STUDY FOR PATTERNS**
6. `migrations/0001_sqlite_state_initial.sql` — Schema — **REFERENCE for table/column assertions**

### Step 1: Invoke the Skill
Run `/spec-to-tdd`.

**Inputs:**
- Source: `docs/strategy/layers/layer-2-single-operator-desktop.md`
- Output TDD: `docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-tdd.md`
- Output Bootstrap: `docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-bootstrap.md`

**Key arguments/context to provide the skill:**
- This is a MIXED STACK layer: Rust (Tauri backend) + TypeScript/React (frontend) + Playwright (browser E2E)
- Three distinct test runners: `cargo test` (Rust), `bun run test` (Vitest), `bunx playwright test` (E2E)
- Primary UI testing surface is browser at `localhost:5173` via dev bridge (NOT native Tauri webview)
- L1 TDD used 4 tiers — L2 should use 3 tiers matching §4.1: Tier 1 (Playwright browser), Tier 2 (Tauri verification), Tier 3 (Agent E2E Rust)
- Bootstrap must cover: Bun workspace setup, Tauri v2 project scaffold, Playwright config, Vite config with dev bridge plugin
- The `L2-FR-*` requirement IDs from the validated spec are the source for the coverage matrix

### Step 2: Feature-Specific Guidance
While the skill runs, pay attention to:

- **Commit fence testing is critical:** The spec defines a precise `snapshotSeq >= visibleAfterSeq` protocol (§1.4). Tests must verify: mutation returns ack → query with `snapshotSeq < visibleAfterSeq` shows old data → query with `snapshotSeq >= visibleAfterSeq` shows new data. This is the most subtle test surface.
- **Gap recovery testing:** Channel<T> streaming (§1.6) defines gap frames. Tests must verify: slow consumer → gap frame emitted → client requests missing chunks from artifact store → stream resumes. This requires test infrastructure to simulate slow consumption.
- **Bootstrap state machine transitions:** 5 modes (§1.7) derived from `ControlPlaneState`. The TDD needs test transitions between all reachable mode pairs, not just happy-path startup.
- **Security sanitization tests:** Agent-authored content tests (§4.3 `security.spec.ts`) need specific XSS payloads, dangerous ANSI sequences, and `javascript:` links as test fixtures.
- **Decision staleness is a temporal test:** Capture decision (hash A) → trigger pipeline re-run → pipeline produces new data (hash B) → verify `isStale === true`. This requires controlling time/sequence in tests.
- **Contracts Finder skill needs a canary:** Like FTS, the CF skill needs `canary.inputs.jsonl` + `golden.assertions.yaml`. The TDD should define what the canary tests.
- **Dev bridge equivalence:** Some tests need to verify that the dev bridge (HTTP/WebSocket) produces identical behavior to the Tauri native bridge. This is a Tier 2 concern but should be called out in TDD.

### Step 3: Verify Output
After the skill completes, verify:
- [ ] TDD file exists at `docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-tdd.md`
- [ ] Bootstrap file exists at `docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-bootstrap.md`
- [ ] Every `L2-FR-*` requirement ID appears in the TDD coverage matrix
- [ ] Every test ID has: description, tier assignment, exact assertions, input fixtures
- [ ] Coverage matrix has zero gaps (every requirement maps to at least one test)
- [ ] Bootstrap covers: Bun workspace, Tauri project, Playwright config, Vite plugin, test harness
- [ ] Tier assignments match §4.1 (Tier 1 = Playwright, Tier 2 = Tauri, Tier 3 = Agent E2E)
- [ ] CI gate commands are explicit: `cargo check`, `cargo clippy`, `cargo fmt`, `cargo test`, `bun run build`, `bun run lint`, `bun run test`, `bunx playwright test`

### Step 4: Follow-Up Pass (Sub-Optimization)

> Send this as a follow-up message in the same session after the main work is complete. Use ULTRATHINK.

**Cross-check the TDD against the actual codebase:**

1. Open `crates/daemon/src/ipc/handshake.rs` — does the TDD's DaemonBridge test section account for the actual handshake protocol (hello/welcome)? Does it test protocol version negotiation and schema epoch mismatch?

2. Open `crates/daemon/src/db/sqlite.rs` — does the TDD's commit fence test section use the actual `commit_clock` table structure? Does it test `commit_seq` increment on write transactions?

3. Open `crates/daemon/src/agents/adapter.rs` — does the TDD map every `ControlEvent` variant to a `SessionFrame` variant in the streaming tests? Are there events in L1 that the TDD doesn't test?

4. Open `migrations/0001_sqlite_state_initial.sql` — does the TDD reference the actual `pending_operator_intents` table columns? Does the decision capture test verify all required metadata fields (reviewedBasisHash, reviewedManifestHash, reviewedAtStageRunId, idempotencyKey)?

5. Open `skills/procurement-ingestion/fts/SKILL.md` — does the TDD's Contracts Finder skill section match the FTS skill's canary/assertion pattern? Does the bootstrap create the same directory structure (`SKILL.md` + schemas + canary + golden)?

6. Count the test IDs. L1 had 150 test IDs for 17 production files. L2 has ~28+ production files with a broader surface (UI + streaming + decisions + coverage). **Expect 150-250 test IDs.** If you have fewer than 100, check for uncovered requirements.

---

## Key Reminders

- **Three test runners, one CI pipeline:** cargo test + bun test + playwright must ALL pass
- **Playwright tests need a running daemon:** The bootstrap must document how to start the daemon for E2E tests
- **Dev bridge is the testing surface:** All Tier 1 tests run in a real browser against `localhost:5173`, NOT inside the Tauri webview
- **Integer pence/basis points everywhere:** All monetary assertions use integer pence; all percentage assertions use integer basis points (0-10000)
- **Sanitization tests need adversarial fixtures:** Don't test with "hello world" — test with `<script>alert(1)</script>`, `[click](javascript:void(0))`, and OSC escape sequences
- **The L1 TDD is your quality bar:** Match its structure, depth, and coverage rigor
- **Bootstrap must be executable:** A fresh agent should be able to run the bootstrap and have a working dev environment
- **`#![forbid(unsafe_code)]` applies to Tauri Rust backend too** — no exceptions for the desktop bridge code
