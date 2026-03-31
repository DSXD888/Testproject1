# L3 Operational Hardening — Create Strategy

## First Action

Invoke the `/spec-to-strategy` skill.

---

## Your Mission

Generate the **implementation strategy** (SOT #3) for Layer 3 Operational Hardening. The strategy defines WHAT ORDER to build things — phase ordering, dependency graphs, domain batches, and gate criteria. It does NOT duplicate spec or TDD content; it provides ORDERING ONLY.

**Context:** L3 adds 12 new Rust modules, 5 DB tables, 10 skills, 3 TS modules, and 18 test files. The existing daemon has 25 Rust source files. The strategy must order implementation so each phase produces a shippable increment and nothing is built before its dependencies.

---

## Source Documents (Read In This Order)

### Primary Input (CREATE FROM THIS)
```
docs/strategy/layers/layer-3-operational-hardening.md    — Spec (SOT #1) — READ FULLY
```

### Source of Truth Context
```
docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-tdd.md — TDD (SOT #2) — READ FULLY
docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-bootstrap.md — Bootstrap — SKIM
docs/strategy/layer-strategy.md                          — Cross-layer strategy — SKIM §5, §9-§11
```

### Codebase Context (STUDY FOR PATTERNS)
```
crates/daemon/src/main.rs                                — Module declarations, bootstrap flow (where new mods register)
crates/daemon/src/jobs/runner.rs                         — Job lifecycle (claim/persist/enqueue) — central integration point
crates/daemon/src/sessions/reaper.rs                     — Session cleanup — extended by L3
crates/daemon/src/agents/artifacts.rs                    — Run manifest types — extended by L3
crates/daemon/src/agents/contracts.rs                    — Contract validation — extended by L3
crates/daemon/src/db/sqlite.rs                           — StateDb, migrations — extended by L3
crates/daemon/src/ipc/command_router.rs                  — IPC dispatch — new handlers needed
packages/intelligence/src/qualification/goods-qualification.ts — Existing TS module pattern
```

### Quality Bar (MATCH OR EXCEED)
```
docs/pipeline/l1-daemon-core/artifacts/l1-daemon-core-strategy.md — L1 Strategy — MATCH THIS STRUCTURE
docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-strategy.md — L2 Strategy — REFERENCE
```

---

## Key Context From Codebase Research

- **Critical dependency: `main.rs` (131K)** is the daemon entry point. Every new module needs `mod` declarations here. The bootstrap flow runs migrations, opens SQLite, and enters the serve loop. L3 must hook cron scheduling and resource arbiter into this bootstrap.
- **`JobRunner` is the central integration point.** Cron scheduler enqueues via `EnqueueJobRequest`. Resource arbiter intercepts at `claim_next_job`. Validation executor uses `EnqueueJobRequest` for sub-jobs. Everything flows through `jobs/runner.rs`.
- **Session lifecycle is well-defined.** `SessionPhase` (7 phases) and `SessionReaper` exist. PTY hardening extends the existing lifecycle, not replaces it.
- **Run manifests are partially implemented.** `RunManifestFrontmatter`, `DaemonEvidenceAppendix`, `ArtifactStore` exist in `agents/artifacts.rs`. L3 extends with Layer 3-specific fields and the manifest writer module.
- **DB migrations are sequential.** Current: 0001 (L0), 0003 (L1), 0004 (L2). L3 migration will be 0005. Must include all 5 new tables + ALTER for modified tables.
- **The relation resolution engine is TypeScript** (in `packages/intelligence`), separate from the Rust daemon. It communicates via IPC or direct SQLite access. This is a relatively independent module.
- **Skills are filesystem-based.** Each skill is a directory with `SKILL.md`, schemas, canary inputs, golden assertions. L3 adds 4 operational + 6 validator-base files.

{ENRICHMENT: This section will be updated after Batch 2 with actual TDD metadata —
test ID count, test file distribution, coverage matrix summary. For now, read the TDD directly.}

---

## Step-by-Step Workflow

### Step 0: Read Source Documents (MANDATORY)
Read these IN THIS ORDER:
1. `docs/strategy/layers/layer-3-operational-hardening.md` — Spec — **READ FULLY**
2. `docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-tdd.md` — TDD — **READ FULLY**
3. `docs/pipeline/l1-daemon-core/artifacts/l1-daemon-core-strategy.md` — L1 Strategy — **READ for structure**
4. `crates/daemon/src/main.rs` — Module declarations — **SKIM first 200 lines**
5. `crates/daemon/src/jobs/runner.rs` — Job types — **STUDY for dependency ordering**

### Step 1: Invoke the Skill
Run `/spec-to-strategy`.

**Inputs:**
- Spec: `docs/strategy/layers/layer-3-operational-hardening.md`
- TDD: `docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-tdd.md`
- Output: `docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-strategy.md`

**Key arguments/context to provide the skill:**
- Strategy provides ORDERING ONLY — never duplicates spec or TDD content
- Phase gates must reference TDD test IDs (not generic "tests pass")
- Expected phase count: 4-6 phases (based on L3 scope and dependency graph)
- Expected bead count: 25-45 beads (based on 12 new modules + 5 tables + 10 skills + 3 TS modules + 18 test files)

### Step 2: Feature-Specific Guidance
While the skill runs, pay attention to:

**Critical dependency ordering:**
1. **DB migration MUST come first** — 5 new tables + ALTER statements. Everything depends on schema.
2. **Run manifest writer extends existing types** — must come before validation executor (which consumes manifests).
3. **Resource arbiter must come before cron scheduler** — cron enqueues jobs, arbiter gates admission.
4. **Source registry must come before source health** — health canaries need source definitions.
5. **Validation policy engine must come before validation executor** — executor uses policy decisions.
6. **Base validator skill must come before validation executor tests** — executor composes kickoff prompts referencing the skill.
7. **Relation resolution (TS) is relatively independent** — can run in parallel with daemon work.
8. **Skill promotion is the capstone** — depends on validation verdicts, source health, and skill metadata.

**Suggested phase structure (verify/adjust):**
- Phase 1: Foundation — DB migration, new module scaffolding, resource arbiter, checkpoint store
- Phase 2: Scheduling & Health — Cron scheduler, source registry, source health canaries, circuit breaker
- Phase 3: PTY & Reliability — PTY hardening, session reaper extensions, structured/forensic tracing
- Phase 4: Validation Pipeline — Run manifest writer extensions, validation policy, validation executor, base validator skill, dry-run mode
- Phase 5: Intelligence & Relations — Relation resolution (TS), error manifest rendering, operational skills
- Phase 6: Integration & Promotion — Skill promotion flow, IPC extensions, telemetry, end-to-end integration tests

### Step 3: Verify Output
After the skill completes, verify:
- [ ] Strategy file exists at `docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-strategy.md`
- [ ] Phase count is reasonable (4-6 phases)
- [ ] Each phase has explicit entry criteria and gate criteria
- [ ] Gate criteria reference TDD test IDs (e.g., "L3-T-cron_scheduler-001 through -008 pass")
- [ ] Dependency graph is acyclic — no circular dependencies between phases
- [ ] Every spec section maps to at least one phase
- [ ] Every TDD test file maps to a phase
- [ ] The strategy does NOT duplicate spec content — only references section numbers
- [ ] Parallel work opportunities are identified (e.g., TS relation resolution || Rust daemon work)
- [ ] Estimated bead count per phase is noted

### Step 4: Follow-Up Pass (Sub-Optimization)

> Send this as a follow-up message in the same session after the main work is complete.

ULTRATHINK. Re-read the strategy with the codebase in mind:

1. **Open `crates/daemon/src/main.rs`** — trace the bootstrap flow. Does the strategy's Phase 1 account for where new modules register in `main.rs`? Module declarations, migration calls, cron scheduler startup, resource arbiter initialization — all need specific insertion points.

2. **Open `crates/daemon/src/jobs/runner.rs`** — `claim_next_job`, `enqueue_job`, `persist_success`, `persist_continuation`. The strategy must show when each function gets modified. Are these changes spread across phases or concentrated?

3. **Check `crates/daemon/Cargo.toml`** — adding `cron` and `sysinfo` crates. Are these in Phase 1 (with foundation) or deferred to when they're first needed?

4. **Verify parallel tracks.** The TS relation resolution module (`packages/intelligence/src/relations/`) has no Rust dependency. Can it truly run in parallel? What about the error manifest TS module (`packages/manifests/`)?

5. **Count beads per phase.** Does the distribution make sense? Phase 1 (foundation) should have fewer, larger beads. Phase 4 (validation) should have more, focused beads.

6. **Check the exit gate.** The spec's exit criteria is "5 consecutive days unattended with >=95% success rate." This is a manual gate, not automatable. Does the strategy's final phase gate account for this?

---

## Key Reminders

- Strategy is SOT #3 — it provides ORDERING ONLY, never content
- Beads will embed spec + TDD content verbatim — strategy only tells beads what order to execute
- Phase gates must reference specific TDD test IDs
- The strategy must account for BOTH Rust and TypeScript work streams
- Dependencies flow downward — no phase may depend on work in a later phase
- Parallel opportunities should be explicitly flagged (saves execution time with multiple agents)
- The skill promotion flow is a capstone that depends on most other L3 systems
