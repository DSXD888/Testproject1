# Autonomous Daemon Scheduling — Step 04: Optimize TDD (Fresh-Eyes Audit)

## First Action

**No skill to invoke.** This is a manual fresh-eyes audit — you read, analyze, and edit the existing TDD/bootstrap directly. Do NOT run `/spec-to-tdd` or any other skill. Use ULTRATHINK throughout.

---

## Your Mission

Perform a **comprehensive fresh-eyes audit** of the TDD at `docs/pipeline/autonomous-daemon-scheduling/artifacts/autonomous-daemon-scheduling-tdd.md` — the SOT #2 for autonomous daemon scheduling. The project rule is real E2E, no mocks (only the deterministic fake-claude CLI for the agent/PTY). Verify every spec requirement maps to a test, every test is real and provable, and the Stage Validation Test Gate is correctly the full per-file loop.

**You are reading, analyzing, and improving an existing document — not writing code.**

---

## What To Read (Do This First)

### The Document to Audit (TARGET)
```
docs/pipeline/autonomous-daemon-scheduling/artifacts/autonomous-daemon-scheduling-tdd.md   — READ FULLY
docs/pipeline/autonomous-daemon-scheduling/artifacts/autonomous-daemon-scheduling-bootstrap.md — READ FULLY
```

### Verify Against (SOURCE OF TRUTH)
```
docs/pipeline/autonomous-daemon-scheduling/artifacts/autonomous-daemon-scheduling-spec.md   — every requirement must have a test
docs/research/2026-06-04-autonomous-daemon-scheduling-research.md §7                         — no test from §7a..§7e silently dropped
docs/pipeline/autonomous-daemon-scheduling/manifest.yaml                                     — test_harnesses (new + extend/guard), quality_gates, verified file:lines
```

### Codebase (GROUND TRUTH — verify the test design is real)
```
crates/daemon/tests/cron_scheduler.rs        — exemplar for scheduler_tick.rs; test 001/006 anchors
crates/daemon/tests/job_claim.rs             — exemplar for worker_pump.rs claim tests; Arc<Barrier>
crates/daemon/tests/resource_arbiter.rs      — exemplar for defer-on-denial + source-lane tests
crates/daemon/tests/agent_procurement_ingestion.rs — RunHarness/fake-claude; Command::output() blocking
crates/daemon/tests/sqlite_schema.rs         — exemplar for the §7e migration test
```

### Quality Bar (MATCH OR EXCEED)
```
docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-tdd.md  (881 lines)
```

---

## Think Independently — Audit These Dimensions

### 1. The Gate is the FULL loop
The single most important check. Verify the TDD's quality gate is the full per-file integration loop, NOT `cargo test -p daemon --bin daemon` alone (which silently skips every integration file — ~61+ `.rs` targets — and caused two false-GREEN manifests historically). If the TDD lets `--bin daemon` count as green, that is a BLOCKING failure.

### 2. Anti-hollow-handler coverage (worker_001)
For each of the 6 worker kinds (`sync_source`, `source_health`, `auth_refresh`, `validation_cross_model_audit`, `validation_calibration_readiness_sweep`, `commercial_sweep`), verify there is a NAMED real-side-effect row assertion beyond `status='completed'`. An empty handler that flips status must fail. Any kind asserting only status is a hollow test — flag it.

### 3. E2E harness shape (M1/M2)
Verify M1/M2 drive the one-shot `--run-scheduler-once --tick-now <rfc3339>` entrypoint (not a live `--serve` daemon, which would hang `RunHarness.run()`'s blocking `Command::output()`). Verify the negative control (a tick before the slot enqueues nothing) is present. M2 must assert NO daemon stop/restart and NO `trigger_pipeline_rerun` call.

### 4. M5 no-lost-writes is concrete
Verify M5 proves no lost writes by a concrete count (N threads × M increments == N*M, or COUNT(*) == N*M), not a hand-wavy "no corruption." Verify it also asserts no PERMANENT `SQLITE_BUSY` (transient + busy_timeout retry is fine) and includes the R7 same-source-across-tick-boundary case.

### 5. The refactor regression guards (§7a)
Verify the TDD explicitly requires `agent_procurement_ingestion.rs` test 001 and `agent_contracts_finder_ingestion.rs` to pass UNCHANGED after the live-loop body moves behind the shared execution function. This is the regression guard that proves the refactor didn't break the manual path. If it's missing or weakened, add it.

### 6. cron_scheduler.rs test-001/006 updates are precise
Verify the TDD names the EXACT edits needed (add the 07:00 commercial_sweep slot to test 001's anchor assertions; add the new key to test 006's catch-up priority-order list) — not a vague "update scheduler tests."

### 7. Conditional §7e migration test for BOTH Q4 options
Verify the TDD has a branch for each Q4 outcome: table → `PRAGMA user_version == 11` + `schema_migrations` contains `0015_*` + the table's columns; state-string → a test that the widened re-fire guard still excludes `awaiting_access`. Both must be present (the operator decision lands at spec time; the TDD must be ready for either).

### 8. Coverage matrix is 100% and bidirectional
Verify EVERY spec requirement has ≥1 test, AND every test traces to a requirement. No orphan tests, no untested requirements.

---

## Scope Drift Detection (MANDATORY — BLOCKING)

### Red-Flag Language Scan
Search the ENTIRE TDD for: "post-MVP," "Phase 2+," "stretch goal," "nice to have," "future iteration," "out of scope," "can be added later," "simplified for initial implementation," "deferred to," "smoke test only," "assertions TBD." Any found → BLOCKING; remove and strengthen.

### Coverage Verification
- [ ] Every spec requirement → ≥1 test ID (walk the spec top to bottom)
- [ ] All of research §7b's named tests present: tick_001..005, worker_001..005
- [ ] M1–M5 all present with preconditions/steps/expected/verify
- [ ] §7a impacts documented for cron_scheduler.rs, job_claim.rs, agent_procurement_ingestion.rs, agent_contracts_finder_ingestion.rs, resource_arbiter.rs, sqlite_schema.rs
- [ ] §7e conditional test present for both Q4 options
- [ ] G1–G6 each have research §7d's "correct proof / regression looks like" pair

### Drift Checklist
- [ ] No test from research §7 silently dropped
- [ ] No assertion strength reduced below the research's
- [ ] The full per-file gate is mandated (not `--bin daemon` alone)
- [ ] worker_001 anti-hollow real-side-effect assertions present for all 6 kinds
- [ ] Real E2E, no mocks — only fake-claude for agent/PTY

---

## Follow-Up Pass (Run After Fixes)

> After making your fixes, send as a follow-up in the same session. ULTRATHINK.

1. Re-open `crates/daemon/tests/agent_procurement_ingestion.rs`. Confirm your TDD's M1 verify-query matches its test-001 idiom exactly, and that you didn't accidentally introduce a `--serve` poll anywhere M1/M2/M4 should use a one-shot entrypoint (`--run-scheduler-once` for M1/M2, `--run-next-job` for M4's button-drain).
2. Re-read worker_001 after your edits — is the real-side-effect assertion still present for ALL 6 kinds, or did an edit drop one?
3. Re-verify the coverage matrix is still 100% after your changes — fixing one test sometimes orphans a requirement. Walk it again.
4. Re-run the red-flag scan on your own edits.
5. Final check: would a Rust engineer, reading ONLY this TDD + the named exemplar files, be able to write every test without further questions? If any test is "write a test that checks scheduling works," it's too vague — make it concrete (file, helper, injected `now`, exact SQL assertion).
