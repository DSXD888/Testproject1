# Autonomous Daemon Scheduling — Step 05: Create Strategy

## First Action

Invoke the `/spec-to-strategy` skill.

---

## Your Mission

Produce the **strategy** (SOT #3) for autonomous daemon scheduling — a lightweight ~500–800 line routing table that defines phase ordering, dependency graphs, domain batches, and gate criteria. It does NOT duplicate spec content; it routes execution order only.

The research doc already gives strong sequencing signal (§4.4 "Sequencing": G4 hardening only pays off after G1/G2 land; §11 "spec's first decision" = Q1 worker exec context). Encode that into a dependency spine.

---

## Source Documents (Read In This Order)

### Primary Inputs
```
docs/pipeline/autonomous-daemon-scheduling/artifacts/autonomous-daemon-scheduling-spec.md  — SOT #1 — READ FULLY
docs/pipeline/autonomous-daemon-scheduling/artifacts/autonomous-daemon-scheduling-tdd.md   — SOT #2 — READ FULLY (gate criteria come from here)
docs/pipeline/autonomous-daemon-scheduling/manifest.yaml                                    — REFERENCE (feature_surface, recorded_decisions, open_questions)
```

### Sequencing Signal From Research
```
docs/research/2026-06-04-autonomous-daemon-scheduling-research.md §4.4 (Sequencing), §11 (weakest point / spec's first decision)
```

### Quality Bar (MATCH OR EXCEED)
```
docs/pipeline/goods-led-install-maintenance/artifacts/goods-led-install-maintenance-strategy.md  — Best existing strategy (740 lines)
```

---

## Cardinal Rule: ZERO SCOPE DRIFT — Phases Are Execution ORDER, Not Importance Tiers

**Every spec feature appears in a phase. Later phases get the SAME depth as early phases. Every phase ships.**

This is the step most prone to disguised scope cuts. Forbidden:
- Treating a later phase as "optional" or "if time permits"
- Giving G4 (ProContract resumption) or G6 (commercial sweep) a thin phase because they come later in the dependency spine
- Labeling any phase "post-MVP" or "Phase 2+"

The dependency spine determines WHEN, never WHETHER. G4 depends on G1/G2 (research §4.4) — that makes it a LATER phase, with FULL depth, not a lesser one.

---

## Key Context From Codebase Research

- **Q1 is the spec's first decision** (research §11): worker execution context (own `StateDb` connection per lane vs shared services handle) shapes the §4.3 refactor surface and §6 R8 WAL-contention behavior. Phase 0 of the strategy must resolve/confirm Q1 before the refactor phase, because everything in §4.2/§4.3 hangs off it.
- **Natural dependency spine** (derive the phases from this):
  1. **Foundations** — Q1 decision; clock-injection seam + `--run-scheduler-once` one-shot entrypoint (unblocks ALL E2E tests); the live-loop refactor (§4.3) extracting per-source bodies (everything downstream rides on this).
  2. **Scheduler tick** (G1) — the tick thread, fold of startup-enqueue (Q2), registry-reload (Q9), catch-up.
  3. **Worker pump** (G2) — two lanes, idle backoff, defer-on-denial, arbiter respect; plus the new handlers (source_health, auth_refresh, validation_*).
  4. **Commercial sweep** (G6) — new 07:00 slot + coverage body on the deterministic lane. Depends on the worker + lanes existing.
  5. **ProContract resumption** (G4) — awaiting-access modeling (Q4), lifecycle, work-list sweep, skill contract. Depends on G1/G2 (research §4.4) so enrich actually runs on schedule.
  6. **Concurrency hardening + E2E soak** (G5) — the M5 soak, R7/R8 verification; runs once the writers all exist.
  7. **Desktop status surfacing** (Q7, optional/operator-decided) — if taken, last, depends on the tick/awaiting-access state existing.
- **The clock seam + one-shot entrypoint must land EARLY** — it's the test harness for everything else (research §7c). Put it in Foundations.

> **{ENRICHMENT — after Batches 1 & 2}** This handoff works as-is, but it sharpens once the spec + TDD exist: per the EXECUTION-GUIDE enrichment step, set each phase's gate criteria to the TDD's actual test IDs (tick_001..005, worker_001..005, M1–M5) and map each phase to its spec requirement-ID ranges, so coverage is mechanically checkable. Until enriched, read `…-spec.md` and `…-tdd.md` directly — both are listed as primary inputs.

---

## Step-by-Step Workflow

### Step 0: Read Source Documents
1. `autonomous-daemon-scheduling-spec.md` — **READ FULLY**
2. `autonomous-daemon-scheduling-tdd.md` — **READ FULLY** (phase gates = the TDD's test families)
3. research §4.4 + §11 — **READ** (sequencing + Q1-first)
4. `goods-led-install-maintenance-strategy.md` — **MATCH OR EXCEED** (study its dependency-graph + gate-criteria format)

### Step 1: Invoke the Skill
Run `/spec-to-strategy`.
- Spec input: `…/autonomous-daemon-scheduling-spec.md`
- TDD input: `…/autonomous-daemon-scheduling-tdd.md`
- Strategy output: `…/autonomous-daemon-scheduling-strategy.md`

### Step 2: Feature-Specific Guidance
- Open each phase with its **gate criteria** drawn from the TDD's test IDs (e.g. Phase "Scheduler tick" gate = `scheduler_tick.rs` tick_001..005 green + M1 green).
- Encode the dependency spine above. Make the Foundations phase (Q1 + clock seam + refactor) a HARD prerequisite — the others can't be tested without the one-shot entrypoint.
- Each phase lists which spec requirement IDs it covers, so coverage is checkable.
- The **schema-epoch decision (Q4)** affects phase ordering for G4: if the table option is chosen, the migration (`0015_*`) + epoch bump 10→11 across all constants is the FIRST task in the G4 phase (everything else reads/writes the new table). Note this conditional.
- Note the **§7a existing-test impacts** as tasks in the relevant phases (cron_scheduler.rs test updates land in the Scheduler-tick + Commercial-sweep phases).

### Step 3: Verify Output
- [ ] Strategy exists, ~500–800 lines, routing-only (no spec content duplicated)
- [ ] Every spec feature appears in exactly one phase; coverage is mappable
- [ ] Later phases (G4, G6, G5 soak, Q7) have EQUAL depth to early phases
- [ ] Each phase has gate criteria tied to TDD test IDs
- [ ] Foundations phase resolves Q1 and lands the clock seam + one-shot entrypoint + refactor first
- [ ] Dependency spine matches research §4.4 (G4 after G1/G2)
- [ ] No "post-MVP / Phase 2+ / optional phase / if time permits" language

### Step 4: Follow-Up Pass (Sub-Optimization)

> Send as a follow-up in the same session. ULTRATHINK.

1. Count tasks/requirements per phase. Is the distribution balanced, or did a later phase (G4/G6) end up thin? G4 is a 5-state lifecycle + a net-new test suite + a skill contract change — if its phase has fewer tasks than the scheduler-tick phase, you've under-scoped it. Rebalance.
2. Re-open research §4.4 ("Sequencing"). Does your spine correctly place G4 AFTER G1/G2 (so enrich runs on schedule before the awaiting-access set matters)? And does the Foundations phase land the clock seam + `--run-scheduler-once` before any phase whose gate needs an E2E manifest?
3. Verify the gate criteria are real: each phase's gate should be runnable TDD test IDs, not "looks done." Cross-check against `crates/daemon/tests/` — do the named test files exist or are they correctly marked net-new (scheduler_tick.rs, worker_pump.rs, procontract_resumption.rs)?
4. Scope-drift scan: walk manifest `feature_surface` and confirm every item is in a phase. The optional operator-decided items (Q6 shadow flag, Q7 desktop UX, Q8 chained nudge, enrich_process routing) must be NAMED in a phase (or an explicit "operator-gated" sub-section), not dropped.
5. Cross-check the spine against the codebase precedents the phases ride on: `crates/daemon/src/main.rs:3260` (`SyncLeaseHeartbeat::spawn` — the Foundations tick/worker thread model), `crates/daemon/src/main.rs:23323` (`run_deterministic_pipeline` — the in-process precedent the Commercial-sweep phase reuses), and `crates/daemon/src/jobs/claim.rs:72` (`claim_next_job` — what the Worker phase filters by lane). Does each phase's first task actually have a real anchor in these, or is a phase floating without a code foothold? A phase whose work can't be located in the codebase is under-specified.
6. Re-read every phase title and intro. Any phrasing that implies a phase is less important than another? Fix it — phases are order, not priority.

### Step 5: Scope-Preservation Check (MANDATORY — BLOCKING)

Do this last, as a gate:
1. **Enumerate** every manifest `feature_surface` item. **Verify** each lands in exactly one phase with full depth (later phases NOT thinner).
2. **Scan** the strategy for red-flag language ("post-MVP," "Phase 2+," "optional phase," "if time permits," "lower priority phase," "deferred," "stretch goal"). Any hit → BLOCKING; replace.
3. **Confirm** the dependency spine matches research §4.4 (G4 after G1/G2) and Foundations lands the clock seam + `--run-scheduler-once` before any E2E-gated phase.
4. **Confirm** the optional operator-decided items (Q6/Q7/Q8/enrich_process) are named in a phase or an explicit operator-gated sub-section, never dropped.
