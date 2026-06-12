# Autonomous Daemon Scheduling — Step 08: Optimize Beads (Fresh-Eyes Audit)

## First Action

**No skill to invoke.** This is a manual fresh-eyes audit — you read every bead (`br show <id> --json`), analyze, and edit beads directly with `br update`/`br dep` (via the wrappers where claiming is involved). Do NOT run `/beads-from-sources` or any other skill. Use ULTRATHINK throughout.

---

## Your Mission

Perform a **comprehensive fresh-eyes audit** of the bead graph for autonomous daemon scheduling (label `autonomous-scheduling`). Verify 100% spec→bead coverage, self-contained beads (verbatim spec/TDD, not pointers), correct dependencies, and zero scope drift.

**You are reading, analyzing, and improving beads — not writing code.**

---

## What To Read (Do This First)

### The Beads to Audit (TARGET)
```bash
br list --label autonomous-scheduling --json     # full inventory
br show <id> --json                               # individual bead detail — read EVERY bead
br dep tree <epic-id>                             # dependency structure
```

### Verify Against (SOURCE OF TRUTH)
```
docs/pipeline/autonomous-daemon-scheduling/artifacts/autonomous-daemon-scheduling-spec.md   — every requirement → bead
docs/pipeline/autonomous-daemon-scheduling/artifacts/autonomous-daemon-scheduling-tdd.md    — every test → assigned to a bead
docs/pipeline/autonomous-daemon-scheduling/artifacts/autonomous-daemon-scheduling-strategy.md — phase ordering → dependency edges
docs/pipeline/autonomous-daemon-scheduling/manifest.yaml                                     — feature_surface, verified file:lines
```

### Codebase (GROUND TRUTH — verify bead file:line refs are accurate)
```
crates/daemon/src/main.rs:1703-1727, scheduler/cron.rs:120-397, jobs/runner.rs:580/1626, jobs/claim.rs:72/176, agents/arbiter.rs:14/82-107
```

---

## Think Independently — Audit These Dimensions

### 1. 100% spec→bead coverage (the cardinal check)
Walk the spec requirement-by-requirement. For EACH, identify the bead(s) implementing it. ANY requirement with zero beads is a BLOCKING failure — create the bead. Build the explicit coverage matrix.

### 2. Beads are self-contained
Each bead must embed its spec requirement + TDD test verbatim. A bead that says "implement per the spec" or "see TDD test X" is NOT self-contained — fix it. An agent must be able to implement from the bead alone.

### 3. Accurate file:line references
Spot-check bead file references against the codebase. E.g. a worker-routing bead should cite `main.rs:1703` and the `other => fail_job` arm at `:1722`; a scheduler bead should cite `cron.rs:120` (dispatch_due_jobs). Wrong line numbers waste agent time.

### 4. Dependency correctness
Verify `br dep tree`: Foundations (Q1, clock seam + `--run-scheduler-once`, live-loop refactor) are dependencies of tick/worker beads; G4 beads depend on G1/G2; the Q4 migration bead (if table option) is a dependency of downstream G4 beads. `br ready` should surface Foundations first. No cycles.

### 5. Phase coverage equality
Count beads per phase/sub-epic. Later phases (G4, G6, soak, Q7) must have depth equal to early phases. G4 (5-state lifecycle + net-new suite + skill contract) should NOT have fewer beads than the scheduler-tick phase. Flag imbalance.

### 6. The landmine & hard contracts as ACs
Verify these appear as blocking acceptance criteria in the right beads:
- §4.4 landmine: don't overwrite `interest_registration.state` (G4 create-row bead)
- commercial_sweep MUST NOT call `trigger_pipeline_rerun`; in-process on coverage lane (commercial bead)
- defer-on-denial: no attempt_count bump, no lease (worker bead)
- R7: source-lane serializes same-source (worker/soak bead)
- the full per-file test gate (every bead's verification, or the epic)

### 7. Net-new test beads exist
scheduler_tick.rs, worker_pump.rs, procontract_resumption.rs each have their own bead. The §7a existing-test updates (cron_scheduler.rs test 001/006) are explicit beads.

---

## Scope Drift Detection (MANDATORY — BLOCKING)

### Red-Flag Language Scan
Search ALL bead bodies for: "post-MVP," "Phase 2+," "stretch goal," "nice to have," "future iteration," "out of scope," "can be added later," "simplified for initial implementation," "deferred to." Any found → BLOCKING; remove and replace with full implementation content.

### Coverage Verification
For EVERY feature/subsystem in manifest `feature_surface`, identify which bead(s) implement it. If ANY has ZERO beads → BLOCKING; create it.
- [ ] scheduler_tick (8) → beads
- [ ] worker_pump (6) + new_job_kind_handlers (4) → beads
- [ ] live_loop_refactor (5) → beads
- [ ] commercial_sweep (5) → beads
- [ ] procontract_resumption (7) → beads — FULL depth
- [ ] concurrency_correctness (R1–R8, Q1) → beads
- [ ] desktop_status_surfacing (Q7) → gated bead, not omitted
- [ ] tests (3 new suites + M1–M5 + 7a + 7e) → beads

### Phase Coverage Equality
Count beads per phase. Later phases must NOT be thinner. Phases are execution order, not importance tiers.

---

## Follow-Up Pass (Run After Fixes)

> After applying fixes, send as a follow-up in the same session. ULTRATHINK.

1. Re-run `br list --label autonomous-scheduling --json` and re-build the coverage matrix. Still 100%? Did fixing one bead orphan a requirement?
2. Re-check `br dep tree` — did adding/editing beads introduce a cycle or break the Foundations-first ordering? Does `br ready` still surface the clock-seam/refactor beads first?
3. Re-count beads per phase — is depth balanced after edits?
4. Re-run the red-flag scan on every bead you touched.
5. Verify bead count is sane vs the strategy's phase task counts (they should roughly match). A large gap means a phase was under- or over-beaded.
6. Confirm `bin/br-sync` was run and `.beads/` is committed (NEVER raw `br sync`).
