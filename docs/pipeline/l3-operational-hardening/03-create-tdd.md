# L3 Operational Hardening — Create TDD

## First Action

Invoke the `/spec-to-tdd` skill.

---

## Your Mission

Generate the **TDD specification and bootstrap document** (SOT #2) for Layer 3 Operational Hardening. This document defines EXACTLY what tests must exist before implementation begins — every requirement gets a test ID, every test has a concrete assertion, and a coverage matrix proves 100% requirement coverage.

**Context:** Layer 3 adds 12 new Rust daemon modules, 5 new DB tables, 10 new skills, and 3 new TS modules to a UK public procurement intelligence platform. The existing daemon has 25 Rust source files with real PTY sessions, SQLite state, and agent adapter patterns. The L1 TDD (quality bar) has 174 tests for 138 requirements — L3 should achieve comparable density for its ~80-100 requirement IDs.

---

## Source Documents (Read In This Order)

### Primary Input (CREATE FROM THIS)
```
docs/strategy/layers/layer-3-operational-hardening.md    — Spec (SOT #1, validated) — READ FULLY
```

### Source of Truth Context
```
docs/strategy/layer-strategy.md                          — Cross-layer strategy — SKIM §5, §9, §11 (test evolution)
docs/strategy/cross-reference-index.md                   — Topic lookup — REFERENCE
```

### Codebase Context (STUDY FOR PATTERNS)
```
crates/daemon/src/main.rs                                — Module declarations, bootstrap, session lifecycle (131K)
crates/daemon/src/jobs/runner.rs                         — JobRunner, ClaimedJob, EnqueueJobRequest, PersistSuccessRequest (25K)
crates/daemon/src/sessions/reaper.rs                     — SessionReaper, CleanupVerdict, ReaperConfig, CompletionState (17K)
crates/daemon/src/agents/artifacts.rs                    — RunManifestFrontmatter, DaemonEvidenceAppendix, ArtifactStore (15K)
crates/daemon/src/agents/contracts.rs                    — ValidationOutcome, OqgCheckInput, StageValidationWrite (9.2K)
crates/daemon/src/errors/manifest.rs                     — ErrorCode (29 variants), ErrorRuntimeContext (14K)
crates/daemon/src/pty/session.rs                         — PtySession, SessionPhase, PtyGeometry (15K)
crates/daemon/src/pty/capture.rs                         — PtyCapture, CaptureConfig, StreamIndexEntry (11K)
crates/daemon/src/pty/health.rs                          — HealthIssue, StallTracker, detect_auth_challenge (3.3K)
crates/daemon/src/agents/adapter.rs                      — AgentAdapter trait, InteractionProfile, ExecutionMode (3.7K)
crates/daemon/src/db/sqlite.rs                           — StateDb, open_in_memory, with_write_transaction (9.3K)
crates/daemon/src/ipc/command_router.rs                  — CommandRouter, DispatchOutcome, 11 commands (9.2K)
packages/intelligence/src/qualification/goods-qualification.ts — Existing TS pattern
packages/intelligence/src/qualification/goods-qualification.test.ts — Existing TS test pattern
```

### Quality Bar (MATCH OR EXCEED)
```
docs/pipeline/l1-daemon-core/artifacts/l1-daemon-core-tdd.md — L1 TDD: 174 tests, 138 requirements, 5 CI tiers, 11 fixtures — MATCH THIS QUALITY
```

---

## Key Context From Codebase Research

- **Rust test framework:** `#[test]` for sync, `#[tokio::test]` for async. Uses `rusqlite::Connection::open_in_memory()` for unit scope, temp-file databases for integration/E2E. `serial_test` for sequential execution, `tempfile` for isolation, `assert_cmd` for binary tests.
- **Existing test harnesses (from L1 TDD):** `TestStateDb`, `ArtifactSandbox`, `PtySessionHarness`, `ContractSessionHarness`, `DaemonHarness`, `TestSkillDir`, `ProcessTreeHelper`, `ProcessIsolationGuard`, `ManifestAssert`
- **TS test framework:** Vitest with `environment: 'node'`, `globals: true`, `include: 'src/**/*.{test,spec}.{ts,tsx}'`. Playwright for E2E.
- **CI tiering (from L1):** T1 (<90s, every commit), T2 (<4min, every commit), T3 (<20min, nightly), T4 (<30min, pre-release), Manual Gate (milestone signoff)
- **Spec §5.2 pre-defines 18 test files** in 6 categories: fault injection (4), replay (2), scheduling (2), skill lifecycle (3), validation (6), relations (1). The TDD must define specific test IDs within these files.
- **The spec has ~80-100 requirement IDs** (L3-FR-x.y.z) after validation step — every one needs at least one test ID
- **Key test surfaces for L3:** cron scheduling, resource arbitration, PTY hardening, relation resolution, source health, circuit breaker, skill promotion, run manifests, validation policy, validation executor, dry-run verdicts, fail-open semantics, calibration, telemetry
- **Non-negotiable test rules (from L1 TDD §2):** No mocking SQLite. No mocking process supervision. Real helper binaries for PTY/reaper tests. No `.skip` or `#[ignore]`. Per-commit CI (T1+T2) under 5 minutes.

{ENRICHMENT: This section will be updated after Batch 1 with actual spec metadata —
requirement ID count, subsystem count, section structure. For now, the spec path is
docs/strategy/layers/layer-3-operational-hardening.md and the agent should read it directly.}

---

## Step-by-Step Workflow

### Step 0: Read Source Documents (MANDATORY — before doing anything)
Read these IN THIS ORDER:
1. `docs/strategy/layers/layer-3-operational-hardening.md` — L3 spec — **READ FULLY**
2. `docs/pipeline/l1-daemon-core/artifacts/l1-daemon-core-tdd.md` — L1 TDD quality bar — **READ §1-§6 for structure**
3. `crates/daemon/src/jobs/runner.rs` — Job lifecycle types — **STUDY FOR PATTERNS**
4. `crates/daemon/src/agents/artifacts.rs` — Manifest types — **STUDY FOR PATTERNS**
5. `crates/daemon/src/agents/contracts.rs` — Validation types — **STUDY FOR PATTERNS**
6. `docs/strategy/layer-strategy.md` §11 — Test evolution across layers — **SKIM**

### Step 1: Invoke the Skill
Run `/spec-to-tdd`.

**Inputs:**
- Source: `docs/strategy/layers/layer-3-operational-hardening.md`
- Output: `docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-tdd.md` and `docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-bootstrap.md`

**Key arguments/context to provide the skill:**
- This is a **mixed Rust + TypeScript** project. Rust tests use `cargo test`, TS tests use `bun test` (vitest).
- Follow the L1 TDD structure exactly: Scope, Non-Negotiable Principles, Test Pyramid, CI Tiering, Shared Infrastructure, File Inventory, then per-file test definitions.
- The spec pre-defines 18 test files (§5.2) — use these filenames as the starting point, but add more if requirement coverage demands it.
- L3 cumulative test count should start from L2's cumulative (20 from L0+L1+L2) and add L3's new tests. The spec claims 18 new, cumulative 38.
- Preserve the existing test harnesses from L1 and extend them as needed (e.g., `ValidatorHarness`, `CronSchedulerHarness`, `ResourceArbiterFixture`, `SourceHealthFixture`).

### Step 2: Feature-Specific Guidance
While the skill runs, pay attention to:
- **Validation system testing is the heart of L3 testing.** The 6 validation test files (§5.2) need the most test IDs. Each validation policy mode × trigger condition × verdict outcome needs coverage.
- **The relation resolution engine is TypeScript** — it needs Vitest tests, not Rust tests. File: `resolve-process-relations.test.ts`. Cover all 4 classifications, hard blockers, heuristic scoring, cluster identity.
- **Cron scheduler tests must cover timezone/DST edge cases.** The spec calls this out as a risk (§5.4). Include test cases for UTC+0 → UTC+1 transition and for skipped/repeated cron ticks.
- **Dry-run isolation must be testable.** §2.6 lists 6 isolation guarantees — each one needs a dedicated test that proves the guarantee holds.
- **Circuit breaker state machine** has 3 states × multiple transitions — this is a classical state machine test surface.
- **Fail-open semantics** (§2.7) need negative tests: prove that validator failures DO NOT block the pipeline.

### Step 3: Verify Output
After the skill completes, verify:
- [ ] TDD file exists at `docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-tdd.md`
- [ ] Bootstrap file exists at `docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-bootstrap.md`
- [ ] Every L3-FR-x.y.z requirement ID from the spec has at least one test ID
- [ ] Coverage matrix is present and complete — no orphan requirement IDs
- [ ] CI tiering is defined — every test ID has a tier assignment (T1/T2/T3/T4/Manual)
- [ ] T1+T2 budget is <5 minutes (per-commit budget from L1 conventions)
- [ ] All 18 pre-defined test files from §5.2 are present with test IDs inside
- [ ] New test harnesses/fixtures are defined for L3-specific infrastructure
- [ ] The bootstrap document defines: prerequisite crates (`cron`, `sysinfo`), migration for 5 new tables, skill directory setup, cron configuration seeding
- [ ] Total test count is in the 150-250 range (L1 had 174 for comparable scope)

### Step 4: Follow-Up Pass (Sub-Optimization)

> Send this as a follow-up message in the same session after the main work is complete.

ULTRATHINK. Re-read the generated TDD with fresh eyes against the codebase:

1. **Open `crates/daemon/src/agents/artifacts.rs`** — the `RunManifestFrontmatter` struct has ~15 fields and `DaemonEvidenceAppendix` has ~12 fields. Does the TDD have tests for every manifest field? Missing field tests mean missing coverage.

2. **Open `crates/daemon/src/errors/manifest.rs`** — `ErrorCode` has 29 variants. The spec adds new error codes. Does the TDD have a test that validates the COMPLETE error code list matches the spec?

3. **Open `crates/daemon/src/pty/health.rs`** — `HealthIssue` enum has 4 variants. The spec adds host sleep detection and resource pressure. Are there tests for the interaction between `StallTracker` and the new health issues?

4. **Open `crates/daemon/src/ipc/command_router.rs`** — the router dispatches 11 commands. L3 should add new IPC commands for: cron status, source health status, validation status, resource arbiter status. Does the TDD account for new IPC commands?

5. **Count test IDs.** Does every test have a unique ID? Is the coverage matrix 100%? Run a mental diff: requirements without tests = gaps. Tests without requirements = orphans. Report both.

6. **Check the bootstrap document.** Does it list the exact `cargo add` commands for `cron` and `sysinfo`? Does it include the SQL migration for the 5 new tables? Does it set up the skill directory structure for the 10 new skills?

---

## Key Reminders

- The TDD is SOT #2 — beads will embed TDD test IDs verbatim. Every test ID must be stable and unique.
- Follow the L1 TDD naming convention: `L3-T-{file_stem}-{sequence}` (e.g., `L3-T-cron_scheduler-001`)
- Do NOT mock SQLite — use `open_in_memory()` for unit scope, temp-file for integration
- Do NOT mock process supervision — use real helper binaries
- Relation resolution tests are TypeScript/Vitest — different framework than Rust tests
- Per-commit CI (T1+T2) must stay under 5 minutes — keep heavy tests in T3/T4
- The validation system has the most complex test surface — budget proportionally
- 5 new DB tables need migration tests (bootstrap succeeds, constraints enforced, FK integrity)
