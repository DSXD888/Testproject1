# Autonomous Daemon Scheduling — Step 07: Create Beads

## First Action

Invoke the `/beads-from-sources` skill.

---

## Your Mission

Create the **executable bead graph** for autonomous daemon scheduling, directly from the 3 Sources of Truth (spec, TDD, strategy). Each bead embeds the relevant spec + TDD content **verbatim** (not paraphrased) so it is self-contained — an agent implements from the bead alone. The strategy provides ordering only, never content.

---

## Source Documents (Read In This Order)

### Primary Inputs (the 3 SOT)
```
docs/pipeline/autonomous-daemon-scheduling/artifacts/autonomous-daemon-scheduling-spec.md      — SOT #1 (WHAT) — READ FULLY
docs/pipeline/autonomous-daemon-scheduling/artifacts/autonomous-daemon-scheduling-tdd.md       — SOT #2 (verify) — READ FULLY
docs/pipeline/autonomous-daemon-scheduling/artifacts/autonomous-daemon-scheduling-strategy.md  — SOT #3 (order only) — READ FULLY
docs/pipeline/autonomous-daemon-scheduling/manifest.yaml                                        — REFERENCE (bead_label, key_files, verified file:lines)
```

### Project Bead Conventions (MANDATORY)
```
AGENTS.md §"Beads (br)" + §"RULE NUMBER 2"   — wrappers (bin/br-claim, bin/br-complete), label discipline, bin/br-sync
docs/pipeline/goods-led-install-maintenance/manifest.yaml  — REFERENCE the epic/sub-epic pattern (1 root epic + N phase sub-epics + task beads)
```

### Codebase Context (so beads carry real file:lines)
```
crates/daemon/src/main.rs, scheduler/cron.rs, jobs/runner.rs, jobs/claim.rs, agents/arbiter.rs, db/sqlite.rs, sources/registry.rs, ipc/handlers/jobs.rs
(see manifest.yaml `key_files` for the exact verified line numbers — beads must cite these, never "find the file")
```

---

## Cardinal Rule: ZERO SCOPE DRIFT — 100% Spec Coverage Is MANDATORY

**If any spec feature has zero corresponding beads, this is a BLOCKING failure.**

- Every spec requirement → ≥1 bead.
- Every TDD test → assigned to a bead.
- Beads embed spec + TDD content VERBATIM — no "see the spec for details."
- No bead may carry "post-MVP / Phase 2+ / stretch goal / nice to have / deferred" language.
- Later-phase beads (G4, G6, soak) get the SAME implementation depth as early-phase beads.

The optional operator-decided items (Q6 shadow flag, Q7 desktop UX, Q8 chained nudge, enrich_process routing) get beads too — gated/conditional beads that name the operator decision, NOT omission. If the operator deferred a decision at spec time, the bead captures the decision-point and the recommended default; it is not dropped.

---

## Key Context From Codebase Research

- **Bead tracking model** (this repo): label-based, NOT ID-prefix. All IDs are `testproject1-XXXX`; the feature is tagged with label **`autonomous-scheduling`**. Create ONE root epic, then phase sub-epics matching the strategy phases, then task beads under each (the L4/L5/L6/goods-led pattern).
  ```bash
  br create --type=epic --priority=P0 \
    --title="Epic: Autonomous Daemon Scheduling & ProContract Resumption" \
    --labels autonomous-scheduling
  ```
  Capture the resulting epic ID into the status file's `implementation_epic` and the manifest.
- **Every bead cites VERIFIED file:lines** from manifest `key_files` (e.g. "extend `run_next_workflow_job` at `main.rs:1703`, replacing the `other => fail_job` arm at `:1722`"). No generic "find the file."
- **Sequencing-sensitive beads**: the Foundations beads (Q1 decision, clock seam + `--run-scheduler-once`, live-loop refactor) MUST be dependencies of the tick/worker beads. Use `br dep` to encode this so `br ready` surfaces them first.
- **The §4.3 refactor is one of the largest beads** — extracting per-source bodies that accept a claimed job. Don't let it become a single mega-bead; split per-source-family (fts/contracts-finder shared; procontract discover/interest/enrich; the param-derivation table) but keep each self-contained.
- **G4 conditional on Q4**: if the table option is chosen, the FIRST G4 bead is the migration `0015_procontract_pending_enrichment.sql` + epoch bump 10→11 across all 4 constants (manifest `schema_epoch_constants`) + the §7e migration test. Make downstream G4 beads depend on it.

> **{ENRICHMENT — after Batches 1–3}** This handoff works as-is, but it sharpens once spec + TDD + strategy exist: per the EXECUTION-GUIDE enrichment step, map the strategy's actual phase names to sub-epics, embed the real spec requirement IDs + TDD test IDs verbatim into each bead, and set the expected bead count from the strategy's per-phase task counts. Until enriched, read the three SOT docs directly — they are listed as primary inputs.

---

## Step-by-Step Workflow

### Step 0: Read Source Documents
1. spec, TDD, strategy — **READ FULLY**
2. `AGENTS.md` §Beads + §Rule 2 — **READ** (wrapper discipline, bin/br-sync)
3. `manifest.yaml` `key_files` + `goods-led` manifest's epic pattern — **REFERENCE**

### Step 1: Invoke the Skill
Run `/beads-from-sources`.
- Inputs: the 3 SOT docs above
- Label every bead: `--labels autonomous-scheduling`
- Use `br` (NOT `bd`); sync with `bin/br-sync` (NEVER raw `br sync`)

### Step 2: Feature-Specific Guidance
- Mirror the strategy's phases as sub-epics. Each task bead embeds its spec requirement + TDD test verbatim.
- Encode dependencies: Foundations (Q1, clock seam, refactor) → tick → worker → {commercial sweep, G4} → soak → (Q7 desktop). Use `br dep add`.
- Each bead needs a `### Files to Create/Modify` section with verified paths (so `bin/br-claim` can reserve them) and an `### Acceptance Criteria` section with the TDD test ID(s) that prove it.
- New-test-file beads (scheduler_tick.rs, worker_pump.rs, procontract_resumption.rs) are explicit beads, not folded into impl beads.
- §7a existing-test-update beads (cron_scheduler.rs test 001/006) are explicit beads in the relevant phases.

### Step 3: Verify Output
- [ ] One root epic + phase sub-epics + task beads, all labeled `autonomous-scheduling`
- [ ] `br list --label autonomous-scheduling --json` shows the full graph
- [ ] Every spec requirement → ≥1 bead (build a coverage check)
- [ ] Every TDD test → assigned to a bead
- [ ] Dependencies encoded (`br dep`); `br ready` surfaces Foundations first
- [ ] Each bead cites verified file:lines, has Files-to-Modify + AC-with-test-IDs
- [ ] Epic ID captured in the status file `implementation_epic`
- [ ] No scope-drift language in any bead

### Step 4: Follow-Up Pass (Sub-Optimization)

> Send as a follow-up in the same session. ULTRATHINK.

1. Run `br list --label autonomous-scheduling --json` and walk the spec requirement-by-requirement. For each requirement, find the bead(s) that implement it. Any requirement with ZERO beads is a BLOCKING gap — create the bead. Build the coverage matrix explicitly.
2. Open the worker-routing bead and cross-check `crates/daemon/src/main.rs:1703-1727`. Does the bead enumerate ALL 6 new kinds the match must add, and does it note that source_health/auth_refresh/validation_* are NEW handlers (no executor today), while sync_source reuses the refactored body? If it says "add the cron kinds" generically, expand it.
3. Open the G4 beads. Is the §4.4 landmine (don't overwrite `interest_registration.state`) stated as a blocking AC in the create-row bead? Is the Q4 migration/epoch-bump a dependency of the downstream G4 beads? Is the skill-contract bead (awaitingAccessTargets + union merge) present and pointing at `skills/procurement-ingestion/procontract-enrich-tender/`?
4. Open the commercial_sweep bead. Does it explicitly forbid `trigger_pipeline_rerun` and require in-process execution on the coverage/deterministic lane? Does it capture the stage-shape nuance (net-gate = pure DB sweep via `run_net_margin_gate_once`; landed-cost + manifest invoke skill runners → still under the `ResourceArbiter`, so the coverage lane is NOT arbiter-free)? Does it note the reconcile-not-reuse interaction (a manual `trigger_pipeline_rerun` mid-sweep PID-kills the daemon — safe via per-process-scoped txns + coverage-idempotent next sweep)? Does a bead add the 07:00 slot AND update `crates/daemon/tests/cron_scheduler.rs` test 001/006?
5. Check dependency edges: claim a few leaf beads mentally — does `br ready` order put Foundations (clock seam, refactor) before anything that needs them? Are the net-new test beads (`crates/daemon/tests/scheduler_tick.rs`, `crates/daemon/tests/worker_pump.rs`, `crates/daemon/tests/procontract_resumption.rs`) present as their OWN beads, and do the §7a existing-test-update beads cite the exact targets (`crates/daemon/tests/cron_scheduler.rs` test 001/006; `crates/daemon/tests/agent_procurement_ingestion.rs` test 001 as the refactor regression guard)?
6. Scope-drift scan across ALL beads: grep the bead bodies for red-flag phrases. Verify the optional items (Q6/Q7/Q8/enrich_process) exist as gated beads, not omissions.

### Step 5: Scope-Preservation Check (MANDATORY — BLOCKING)

Do this last, as a gate before declaring beads done:
1. **Build** the spec→bead coverage matrix from `br list --label autonomous-scheduling --json`. **Verify** every spec requirement has ≥1 bead AND every TDD test ID is assigned to a bead. Any gap → BLOCKING; create the bead.
2. **Count** beads per phase/sub-epic. **Verify** later phases (G4, G6, soak, Q7) are NOT thinner than early phases.
3. **Scan** ALL bead bodies for red-flag language ("post-MVP," "Phase 2+," "stretch goal," "nice to have," "deferred," "out of scope," "future iteration," "simplified for initial implementation," "can be added later"). Any hit → BLOCKING; replace with full implementation content.
4. **Confirm** `br dep tree` has no cycles, Foundations-first ordering holds, and (if Q4=table) the `0015_*`/epoch-bump bead is a dependency of downstream G4 beads.
5. **Run** `bin/br-sync` and confirm `.beads/` is committed (NEVER raw `br sync`).
