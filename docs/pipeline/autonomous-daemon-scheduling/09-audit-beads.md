# Autonomous Daemon Scheduling — Step 09: Bead Audit (Spec Drift Detection)

## First Action

Invoke the `/bead-plan-audit` skill.

---

## Your Mission

Perform a **dedicated, systematic audit** of all beads for autonomous daemon scheduling (label `autonomous-scheduling`). This is the LAST GATE before execution. Verify **zero spec drift** — every spec requirement covered, every TDD test assigned, no silent scope cuts, dependencies correct, beads self-contained and implementable.

---

## Source Documents

### Beads (TARGET — audit EVERY bead, not a sample)
```bash
br list --label autonomous-scheduling --json
br show <id> --json
br dep tree <epic-id>
```

### Verify Against
```
docs/pipeline/autonomous-daemon-scheduling/artifacts/autonomous-daemon-scheduling-spec.md      — every requirement must have a bead
docs/pipeline/autonomous-daemon-scheduling/artifacts/autonomous-daemon-scheduling-tdd.md       — every test ID must be assigned
docs/pipeline/autonomous-daemon-scheduling/artifacts/autonomous-daemon-scheduling-strategy.md  — phase ordering must match dependency edges
docs/pipeline/autonomous-daemon-scheduling/manifest.yaml                                        — feature_surface, verified file:lines, open_questions
```

### Codebase (verify bead references are accurate)
```
crates/daemon/src/main.rs:1703-1727, 3260, 3697, 3819, 15371, 23323
crates/daemon/src/scheduler/cron.rs:120-397
crates/daemon/src/jobs/{runner.rs:580/1626, claim.rs:72/176}
crates/daemon/src/agents/arbiter.rs:14/82-107
crates/daemon/src/db/sqlite.rs:89/215-218
crates/daemon/tests/  (confirm existing vs net-new test files)
```

---

## Audit Methodology

`/bead-plan-audit` handles the detailed methodology. Add these feature-specific checks.

### Feature-Specific Checks
- **Q1 resolution present** — the worker-exec-context decision (own connection per lane vs shared services) is either resolved in the spec/beads or has an explicit decision bead that BLOCKS the refactor beads. The refactor cannot start with Q1 floating (research §11).
- **Clock seam is a real bead** — `run_scheduler_tick(…, now)` + `--run-scheduler-once --tick-now` is its own bead and is a dependency of every E2E-gated bead (it's the test harness).
- **Live-loop refactor decomposed** — not one mega-bead; split sensibly but each self-contained, with the §4.3 per-source derivation table embedded.
- **6 worker kinds enumerated** — the routing bead lists all of sync_source/source_health/auth_refresh/validation_cross_model_audit/validation_calibration_readiness_sweep/commercial_sweep, with the new-handler vs reuse-refactored-body distinction.
- **G4 landmine + Q4 coupling** — create-row bead forbids overwriting `interest_registration.state`; if Q4=table, migration `0015_*` + epoch 10→11 bead is a dependency of downstream G4 beads.
- **commercial_sweep contract** — forbids `trigger_pipeline_rerun`; in-process coverage lane; two-layer dedup; one 07:00 slot + cron_scheduler.rs test 001/006 updates.
- **Anti-hollow tests** — worker_001 bead requires real-side-effect row assertions per kind.
- **Full per-file gate** — the verification on the epic (or each bead) is the full `for t in crates/daemon/tests/*.rs` loop, not `--bin daemon` alone.

### Scope Drift Detection (MANDATORY — BLOCKING DIMENSION)

**Red-flag language scan** — search ALL beads for: "post-MVP," "Phase 2+," "stretch goal," "nice to have," "future iteration," "out of scope," "can be added later," "simplified for initial implementation," "deferred to." If found: BLOCKING failure — remove and replace with full implementation content.

**Coverage verification** — for EVERY feature/subsystem in the spec + manifest `feature_surface`:
- Identify which bead(s) implement it.
- If ANY spec feature has ZERO corresponding beads → BLOCKING failure; create the missing bead before proceeding.

**Phase coverage equality** — count beads per strategy phase:
- Later phases (G4, G6, soak, Q7) MUST have depth equal to early phases.
- If a later phase has significantly fewer beads, investigate — phases are execution order, not importance tiers.

**Optional-item integrity** — Q6 (shadow flag), Q7 (desktop UX), Q8 (chained nudge), enrich_process routing must each be a gated/conditional bead naming the operator decision, NOT an omission.

---

## Follow-Up Pass

> After the audit is complete and all fixes applied. ULTRATHINK.

1. Re-build the spec→bead coverage matrix from scratch (`br list --json` vs spec requirement IDs). Is it still 100% after fixes? Did fixing one bead break another's dependency edge?
2. Re-run `br dep tree` — no cycles, Foundations-first ordering intact, Q4-migration-before-downstream-G4 intact, clock-seam-before-E2E-gated intact.
3. Re-verify bead count ≈ strategy phase task counts.
4. Re-run the red-flag scan across ALL beads one final time.
5. Confirm EVERY spec feature has ≥1 bead mapping and EVERY TDD test ID is assigned. If the audit changed any bead, run `bin/br-sync` and confirm `.beads/` is clean.
6. Produce a one-page audit verdict: PASS (100% coverage, zero drift, deps correct) or BLOCKED (list exact gaps). Do not pass with <100% coverage.
