# Autonomous Daemon Scheduling — Step 06: Optimize Strategy (Fresh-Eyes Audit)

## First Action

**No skill to invoke.** This is a manual fresh-eyes audit — you read, analyze, and edit the existing strategy directly. Do NOT run `/spec-to-strategy` or any other skill. Use ULTRATHINK throughout.

---

## Your Mission

Perform a **comprehensive fresh-eyes audit** of the strategy at `docs/pipeline/autonomous-daemon-scheduling/artifacts/autonomous-daemon-scheduling-strategy.md` — the SOT #3 (ordering only) for autonomous daemon scheduling. Verify phase coverage equality, a correct dependency spine, real gate criteria, and zero deferred features.

**You are reading, analyzing, and improving an existing document — not writing code.**

---

## What To Read (Do This First)

### The Document to Audit (TARGET)
```
docs/pipeline/autonomous-daemon-scheduling/artifacts/autonomous-daemon-scheduling-strategy.md   — READ FULLY
```

### Verify Against (SOURCE OF TRUTH)
```
docs/pipeline/autonomous-daemon-scheduling/artifacts/autonomous-daemon-scheduling-spec.md   — every feature must appear in a phase
docs/pipeline/autonomous-daemon-scheduling/artifacts/autonomous-daemon-scheduling-tdd.md    — gate criteria must tie to test IDs
docs/research/2026-06-04-autonomous-daemon-scheduling-research.md §4.4, §11                  — dependency spine + Q1-first
docs/pipeline/autonomous-daemon-scheduling/manifest.yaml                                     — feature_surface
```

### Codebase (GROUND TRUTH)
```
crates/daemon/src/main.rs:3260,1703  — the patterns the early phases build on
crates/daemon/tests/                  — confirm named gate test files (existing vs net-new)
```

### Quality Bar (MATCH OR EXCEED)
```
docs/pipeline/goods-led-install-maintenance/artifacts/goods-led-install-maintenance-strategy.md  (740 lines)
```

---

## Think Independently — Audit These Dimensions

### 1. Phase coverage EQUALITY (the cardinal check)
Count tasks/requirements per phase. Later phases (G4 ProContract resumption, G6 commercial sweep, G5 soak, Q7 desktop) must have depth EQUAL to early phases (G1 tick, G2 worker). If G4 — a 5-state lifecycle + net-new test suite + skill contract change — has fewer tasks than the scheduler-tick phase, that's a scope-drift signal. Investigate and rebalance.

### 2. Dependency spine correctness
Verify: Foundations (Q1 + clock seam + `--run-scheduler-once` entrypoint + live-loop refactor) comes FIRST; the scheduler tick + worker follow; G4 comes AFTER G1/G2 (research §4.4 — enrich must run on schedule before awaiting-access matters); the M5 concurrency soak comes once all writers exist. A spine that lets a phase be gated by an E2E manifest before the one-shot entrypoint exists is broken.

### 3. Q1 resolved in Foundations
Verify the strategy treats Q1 (worker exec context) as the first decision (research §11), resolved before the refactor phase. If Q1 is still floating, the refactor phase has no foundation.

### 4. Gate criteria are real and test-tied
Each phase gate should be runnable TDD test IDs (e.g. tick_001..005 green, M1 green), not "phase complete." Cross-check the named test files against `crates/daemon/tests/` — existing ones (cron_scheduler.rs, job_claim.rs, agent_procurement_ingestion.rs) vs net-new (scheduler_tick.rs, worker_pump.rs, procontract_resumption.rs).

### 5. Q4 conditional ordering
Verify the G4 phase notes: if the dedicated-table option is chosen, the migration `0015_*` + epoch bump 10→11 across all 4 constants is the FIRST task of that phase (everything else depends on the table existing).

### 6. The single new slot
Verify the strategy adds exactly ONE schedule slot (07:00 commercial_sweep) and does NOT retune existing values (FTS 04:15 etc.) — and that the cron_scheduler.rs test 001/006 updates are tasked in the right phases.

### 7. Optional items named, not dropped
Q6 (shadow flag), Q7 (desktop UX), Q8 (chained nudge), enrich_process routing — each must appear in a phase or an explicit operator-gated sub-section. Named + bounded, never silently dropped.

---

## Scope Drift Detection (MANDATORY — BLOCKING)

### Red-Flag Language Scan
Search the ENTIRE strategy for: "post-MVP," "Phase 2+," "stretch goal," "nice to have," "future iteration," "out of scope," "can be added later," "simplified for initial implementation," "deferred to," "if time permits," "optional phase," "lower priority phase." Any found → BLOCKING; remove and replace.

### Coverage Verification
Enumerate every manifest `feature_surface` item. For each, confirm it lands in exactly one phase with full depth:
- [ ] scheduler_tick → its phase (8 items)
- [ ] worker_pump + new_job_kind_handlers → their phase (6 + 4 items)
- [ ] live_loop_refactor → Foundations (5 items)
- [ ] commercial_sweep → its phase (5 items)
- [ ] procontract_resumption → its phase (7 items) — FULL depth
- [ ] concurrency_correctness → soak phase (R1–R8, Q1)
- [ ] desktop_status_surfacing → named (Q7)
- [ ] tests → gate criteria across all phases

### Drift Checklist
- [ ] Every spec feature appears in exactly one phase
- [ ] Later phases have SAME depth as early phases
- [ ] Phases are labeled execution order, NOT importance tiers
- [ ] Dependency spine matches research §4.4 (G4 after G1/G2)
- [ ] Foundations lands the clock seam + one-shot entrypoint first
- [ ] Exactly one new slot (07:00); existing schedules untouched

---

## Follow-Up Pass (Run After Fixes)

> After making your fixes, send as a follow-up in the same session. ULTRATHINK.

1. Re-count tasks per phase after your edits. Is depth now balanced across G1–G6 + soak + Q7? Did fixing one phase make another look thin?
2. Re-trace the dependency spine end-to-end: can each phase's gate actually be evaluated given what prior phases built? Specifically — is the `--run-scheduler-once` entrypoint available before any phase gated on M1/M2?
3. Re-verify the optional items (Q6/Q7/Q8/enrich_process) are still named in phases, not dropped during rebalancing.
4. Re-run the red-flag scan on your own edits.
5. Final: would an execution agent reading only this strategy know exactly what order to build in, what each phase's done-gate is, and that nothing is "for later"? If a phase reads as skippable, fix it.
