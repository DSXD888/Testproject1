# Autonomous Daemon Scheduling — Step 03: Create TDD

## First Action

Invoke the `/spec-to-tdd` skill.

---

## Your Mission

Produce the **TDD** (SOT #2) + **bootstrap** doc for autonomous daemon scheduling. The project rule is **real E2E, no mocks/stubs** — the ONLY permitted fake is the deterministic `fake-claude` CLI (it stands in for the agent/PTY, never the daemon under test). Every test runs against real SQLite (tempdir), real cron parsing, and the real `daemon` binary subprocess where the flow demands it.

The research doc already contains a near-complete test design in §7 (existing-test impacts, new unit/integration tests with named exemplars, 5 E2E manifests M1–M5, verification criteria per goal, schema-migration test). Your job is to formalize §7 into a TDD with stable test IDs and a spec→test coverage matrix, losing nothing.

---

## Source Documents (Read In This Order)

### Primary Inputs
```
docs/pipeline/autonomous-daemon-scheduling/artifacts/autonomous-daemon-scheduling-spec.md  — SOT #1 — READ FULLY
docs/research/2026-06-04-autonomous-daemon-scheduling-research.md §7                        — READ FULLY (the test design — §7a..§7e)
docs/pipeline/autonomous-daemon-scheduling/manifest.yaml                                    — REFERENCE (test_harnesses, quality_gates)
```

### Test Infrastructure to Copy (STUDY — these are the exemplars §7 names)
```
crates/daemon/tests/cron_scheduler.rs        — ts() fixed-instant helper, CronScheduler::from_registry, refresh_next_run_at, dispatch_due_jobs, TestStateDb::new(), seed_source; schedule-anchor tests 001/006
crates/daemon/tests/job_claim.rs             — seed_job, JobRunner::claim_next_job, Arc<Barrier> multi-thread pattern (test 001), claim-ordering (test 002)
crates/daemon/tests/resource_arbiter.rs      — admit/release, DenialReason variants, browser-slot hard-cap test
crates/daemon/tests/agent_procurement_ingestion.rs — RunHarness + fake-claude (DAEMON_TEST_CLAUDE_RESULT_PATH/_CHECKPOINT_PATH, --version/probe); post-run SQLite assertion idiom; RunHarness.run() uses Command::output() (BLOCKING)
crates/daemon/tests/sqlite_schema.rs         — PRAGMA user_version == DEFAULT_SCHEMA_EPOCH assertion; schema_migrations row check; logical_schema_for_tables
crates/daemon/tests/support/db.rs            — TestStateDb::new(), seed_process, seed_raw_document, logical_schema_for_tables
```

### Quality Bar (MATCH OR EXCEED)
```
docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-tdd.md  — Best existing TDD (881 lines)
docs/pipeline/goods-led-install-maintenance/artifacts/goods-led-install-maintenance-tdd.md     — Concise TDD (318 lines)
```

---

## Cardinal Rule: ZERO SCOPE DRIFT

**Every spec requirement must have test coverage.** You may improve HOW a thing is tested, resequence WHEN tests run, and add missing edge-case tests. You may NEVER drop a test from research §7, reduce assertion strength, or leave a spec requirement untested. If a requirement seems untestable, that's a signal the requirement (or the test) is wrong — flag it, don't skip it.

---

## Key Context From Codebase Research

- **The Stage Validation Test Gate is non-negotiable** (AGENTS.md + research §7 gate): `cargo test -p daemon --bin daemon` SILENTLY SKIPS every integration file (~61 `.rs` targets today, growing to ~64 once this feature's 3 net-new suites land). The TDD's gate MUST be the full per-file loop:
  ```bash
  cargo fmt --check
  cargo check --all-targets
  cargo clippy --all-targets -- -D warnings
  cargo test -p daemon --bin daemon
  for t in crates/daemon/tests/*.rs; do cargo test --test "$(basename "$t" .rs)" || exit 1; done
  ```
  The new suites (`scheduler_tick.rs`, `worker_pump.rs`, `procontract_resumption.rs`) are ONLY exercised by the `for` loop. State this in the TDD's gate section verbatim.
- **`RunHarness.run()` blocks on `Command::output()`** — a live `--serve` daemon never exits, so M1/M2 MUST use the one-shot `--run-scheduler-once --tick-now <rfc3339>` entrypoint (evaluate → enqueue → drain → exit 0), which fits `.output()` exactly. The TDD must specify M1/M2 against this entrypoint, with the spawn-poll-terminate serve harness as the documented fallback only.
- **Anti-hollow-handler requirement** (research §7b worker_001): `status='completed'` ALONE is insufficient — an empty handler that flips status would pass falsely. EACH cron kind's test must assert the handler's **real side-effect row** (sync_source→source-produced row; source_health→canary result row; auth_refresh→auth-state row; validation_*→verdict/summary row; commercial_sweep→landed-cost/net-margin/manifest rows). Bake this into every worker_001 assertion.
- **No ProContract integration test exists today** — `procontract_resumption.rs` is built FROM SCRATCH (M3). Size it as net-new, not "extend an existing test."

> **{ENRICHMENT — after Batch 1}** This handoff works as-is, but it improves once the spec exists: per the EXECUTION-GUIDE enrichment step, replace the generic "every spec requirement" language below with the spec's actual requirement-ID range and count (FR-G1.x … NFR-x across G1–G6 + FR-RT + concurrency), and add spec-specific verification questions to the coverage matrix. Until enriched, read `…-spec.md` directly — it is listed as a primary input.

---

## Step-by-Step Workflow

### Step 0: Read Source Documents
1. `autonomous-daemon-scheduling-spec.md` — **READ FULLY**
2. research doc **§7 in full** (§7a existing-test impact; §7b new unit/integration; §7c E2E manifests; §7d verification criteria; §7e schema-migration test) — **READ FULLY**
3. `crates/daemon/tests/cron_scheduler.rs` + `agent_procurement_ingestion.rs` — **STUDY** (the idioms every new test copies)
4. `l6-request-pipeline-automation-tdd.md` — **MATCH OR EXCEED**

### Step 1: Invoke the Skill
Run `/spec-to-tdd`.
- Spec input: `docs/pipeline/autonomous-daemon-scheduling/artifacts/autonomous-daemon-scheduling-spec.md`
- TDD output: `docs/pipeline/autonomous-daemon-scheduling/artifacts/autonomous-daemon-scheduling-tdd.md`
- Bootstrap output: `docs/pipeline/autonomous-daemon-scheduling/artifacts/autonomous-daemon-scheduling-bootstrap.md`

### Step 2: Feature-Specific Guidance
Translate research §7 into stable test IDs and a coverage matrix. Required test families:
- **`scheduler_tick.rs`** — tick_001 (due slot enqueues exactly one row), tick_002 (repeated ticks idempotent), tick_003 (long-gap catch-up), tick_004 (clock seam — `fn(…, now)`, no inline `Utc::now()`), tick_005 (registry-reload on generation bump).
  - **tick_002 precision (verified against `runner.rs:1626`):** the dedup matches `status IN ('pending','running','retry','completed')` and EXCLUDES `'failed'`. So tick_002 asserts idempotency across the pending/completed path (3 ticks → still 1 row). Do NOT add an assertion that a *failed* slot stays suppressed — re-enqueue-after-failure is correct retry behavior. If the TDD wants to pin that, it is a SEPARATE positive test ("a failed slot is re-enqueueable on the next tick"), not part of tick_002.
- **`worker_pump.rs`** — worker_001 (routes each cron kind + real side-effect assertion, anti-hollow), worker_002 (defer-on-denial, no attempt bump), worker_003 (source-lane serializes same source, R7), worker_004 (head-of-line: commercial_sweep completes before slow PTY sync_source — the falsifiable lane claim), worker_005 (arbiter quota respected).
- **`procontract_resumption.rs`** — M3 as one ordered 4-step scenario (register → enrich-keep+backfill → enrich-clear-via-worklist → staleness-expire).
- **E2E manifests** M1 (autonomy G1+G2), M2 (commercial autonomy G6, no daemon stop), M3 (above), M4 (manual-path regression + live `trigger_ingestion` button via `--run-next-job` drain), M5 (concurrency soak: N×M counter == no-lost-writes + no permanent SQLITE_BUSY + R7 same-source).
- **§7a existing-test impacts** — document each: cron_scheduler.rs (add 07:00 slot to test 001 anchors + test 006 order), agent_procurement_ingestion.rs / agent_contracts_finder_ingestion.rs (refactor regression — must pass UNCHANGED), job_claim.rs (add new-kind tests, don't mutate ordering tests), sqlite_schema.rs (only if Q4 table).
- **§7e schema-migration test** — conditional on Q4 (table → user_version==11 + schema_migrations '0015_*' + columns; state-string → widened re-fire guard test).

### Step 2b: Bootstrap Doc — Required Content (do not leave it thin)
The bootstrap doc is the environment-setup SOT an executing agent reads before writing a single test. For THIS feature it must contain, concretely:
- **Toolchain:** Rust 2024 nightly (per `rust-toolchain.toml`), `#![forbid(unsafe_code)]`, cargo only (never npm/yarn for Rust). Bun only if Q7 desktop surfacing is taken.
- **The full Stage Validation Test Gate verbatim** (fmt → check → clippy → `--bin daemon` → the `for t in crates/daemon/tests/*.rs` loop) with the explicit warning that `--bin daemon` alone silently skips every integration file (~61+ targets), including the 3 net-new suites.
- **The fake-claude harness:** how `RunHarness` (in `agent_procurement_ingestion.rs`) wires the deterministic fake-claude CLI via `DAEMON_TEST_CLAUDE_RESULT_PATH` / `DAEMON_TEST_CLAUDE_CHECKPOINT_PATH` + `--version`/`probe`; that `RunHarness.run()` uses `Command::output()` (BLOCKING) so E2E tests drive the one-shot `--run-scheduler-once --tick-now <rfc3339>` entrypoint, never a live `--serve`.
- **Test fixtures/helpers to reuse:** `support/db.rs` (`TestStateDb::new()`, `seed_process`, `seed_raw_document`, `logical_schema_for_tables`), `cron_scheduler.rs` `ts()` fixed-instant helper, `job_claim.rs` `seed_job` + `Arc<Barrier>`, `resource_arbiter.rs` `admit`/`release`.
- **DB setup:** real SQLite in a tempdir; WAL + `busy_timeout=5000` are applied by `open_state_db`; `DEFAULT_SCHEMA_EPOCH` currently 10. If Q4=table, how to apply `migrations/0015_*` and that the test asserts `PRAGMA user_version == 11`.
- **How to run a single new suite fast:** `cargo test --test scheduler_tick` / `--test worker_pump` / `--test procontract_resumption` (debug builds for iteration).
- **Env auto-skip discipline:** env-dependent tests auto-skip via runtime probes, NEVER `#[ignore]` (AGENTS.md).

### Step 3: Build the Coverage Matrix
A table mapping every spec requirement → test ID(s). Mark any requirement with no test as a BLOCKING gap. Mirror research §7d's per-goal "correct proof / regression looks like" table.

### Step 4: Verify Output
- [ ] TDD exists with stable test IDs and a 100% spec→test coverage matrix
- [ ] The full-loop gate (not `--bin daemon` alone) is stated verbatim
- [ ] Every new test names the exemplar file it copies idioms from
- [ ] worker_001 includes the anti-hollow real-side-effect assertion for every kind
- [ ] M1/M2 are specified against `--run-scheduler-once --tick-now`, with the blocking-`output()` rationale
- [ ] M5 proves no-lost-writes concretely (N×M count), not hand-wavily
- [ ] Conditional §7e test branch is present for BOTH Q4 options
- [ ] Negative controls present (M1: a tick before the slot enqueues nothing)

### Step 5: Follow-Up Pass (Sub-Optimization)

> Send as a follow-up in the same session. ULTRATHINK.

1. Open `crates/daemon/tests/agent_procurement_ingestion.rs` and find test 001's post-run SQLite assertion (it checks `kind='sync_source'`, `status='completed'`). Your TDD's M1 verify-step must use the SAME idiom: `SELECT kind,status FROM workflow_jobs WHERE kind='sync_source' AND parent_job_id IS NULL ORDER BY created_at DESC LIMIT 1` → `('sync_source','completed')` PLUS the FTS-produced rows. Does it? And does it confirm `RunHarness.run()` uses `Command::output()` so M1 cannot point at `--serve`?
2. Open `crates/daemon/tests/cron_scheduler.rs` test 001 and test 006. Your §7a impact notes claim these break when the 07:00 commercial_sweep slot is added. Verify the TDD names the EXACT change (test 001's anchor set gains the 07:00 slot; test 006's catch-up priority-order list gains the new key) — not a vague "update the scheduler tests."
3. Re-read worker_001. For EACH of the 6 kinds, is there a named real-side-effect row assertion? An empty handler that just flips `status='completed'` must FAIL the test. If any kind only asserts status, that kind's test is hollow — fix it.
4. Re-check the coverage matrix: is EVERY spec requirement mapped? Walk the spec top to bottom; for each requirement ID, find its test ID. Any unmapped requirement is a blocking gap — add the test.
5. Open `crates/daemon/tests/resource_arbiter.rs` (the defer-on-denial + source-lane exemplar) and `crates/daemon/tests/sqlite_schema.rs` (the §7e migration-test exemplar). Does worker_002 (defer-on-denial) copy the arbiter's `admit`/`release`/`DenialReason` idioms, and assert `attempt_count` UNCHANGED + `lease_owner IS NULL` + not-re-claimed-next-iteration? Does the §7e branch (if Q4=table) copy sqlite_schema.rs's `PRAGMA user_version` + `schema_migrations` row-check + `logical_schema_for_tables` idioms for `0015_*`? If either test invents its own scaffolding instead of copying the exemplar, fix it.
6. Scope-drift scan on the TDD itself: did any test get softened to "smoke test only" or marked "can add assertions later"? Remove and strengthen.

### Step 6: Coverage-Preservation Check (MANDATORY — BLOCKING)

Do this last, as a gate:
1. **Enumerate** every spec requirement ID. **Verify** each has ≥1 test ID in the coverage matrix. Any requirement with zero tests is a BLOCKING gap — add the test.
2. **Enumerate** research §7's named tests (tick_001..005, worker_001..005, M1–M5, the §7a impacts, the §7e branch). **Verify** none was silently dropped.
3. **Scan** the TDD for red-flag language ("post-MVP," "Phase 2+," "deferred," "smoke test only," "assertions TBD," "can add later"). Any hit → BLOCKING; strengthen.
4. **Confirm** the gate is the FULL per-file loop, never `cargo test -p daemon --bin daemon` alone.
