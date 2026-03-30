# L3 Operational Hardening — Create Beads

## First Action

Invoke the `/beads-from-sources` skill.

---

## Your Mission

Generate **self-contained executable beads** for Layer 3 Operational Hardening from the three Sources of Truth. Each bead embeds spec and TDD content verbatim — an agent implements a bead without reopening the spec. The strategy provides ordering only.

**Context:** L3 is a mixed Rust + TypeScript layer adding 12 new daemon modules, 5 DB tables, 10 skills, 3 TS modules, and 18 test files. Expected bead count: 25-45.

---

## Source Documents (Read In This Order)

### Primary Inputs (CREATE FROM THESE)
```
docs/strategy/layers/layer-3-operational-hardening.md    — Spec (SOT #1) — READ FULLY
docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-tdd.md — TDD (SOT #2) — READ FULLY
docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-strategy.md — Strategy (SOT #3) — READ FULLY
```

### Codebase Context (STUDY FOR PATTERNS)
```
crates/daemon/src/main.rs                                — Module declarations, bootstrap, serve loop (131K)
crates/daemon/src/jobs/runner.rs                         — JobRunner, EnqueueJobRequest, ClaimedJob (25K)
crates/daemon/src/sessions/reaper.rs                     — SessionReaper, CleanupVerdict (17K)
crates/daemon/src/agents/artifacts.rs                    — RunManifestFrontmatter, ArtifactStore (15K)
crates/daemon/src/agents/contracts.rs                    — ValidationOutcome, OqgCheckInput (9.2K)
crates/daemon/src/errors/manifest.rs                     — ErrorCode (29 variants) (14K)
crates/daemon/src/pty/session.rs                         — PtySession, SessionPhase (15K)
crates/daemon/src/pty/capture.rs                         — PtyCapture, CaptureConfig (11K)
crates/daemon/src/pty/health.rs                          — HealthIssue, StallTracker (3.3K)
crates/daemon/src/agents/adapter.rs                      — AgentAdapter trait (3.7K)
crates/daemon/src/db/sqlite.rs                           — StateDb, migrations (9.3K)
crates/daemon/src/ipc/command_router.rs                  — CommandRouter, 11 commands (9.2K)
crates/daemon/Cargo.toml                                 — Dependencies
packages/intelligence/src/qualification/goods-qualification.ts — TS module pattern
packages/manifests/render-opportunity.ts                 — TS manifest rendering pattern
```

### Quality Bar
```
.beads/beads.db                                          — Existing beads (study structure via `br list --json`)
```

---

## Key Context From Codebase Research

- **Bead CLI is `br`** — use `br create`, `br dep add`, `bin/br-sync`
- **Bead prefix: `l3`** — all beads for this layer use this prefix
- **Beads embed spec + TDD verbatim** — each bead has the relevant spec section + TDD test IDs inline
- **Strategy provides ordering** — beads are created in strategy phase order, with dependencies between phases
- **The daemon has 25 existing Rust files** — beads that modify existing files must name the EXACT file path
- **12 new Rust modules** → each typically gets its own bead (implementation) + shared test beads
- **5 new DB tables** → one migration bead for all tables (atomic DDL change)
- **10 new skills** → skill directory beads (filesystem, not compiled code)
- **3 new TS modules** → separate beads (different build pipeline)
- **18 new test files** → test beads paired with implementation beads or grouped by subsystem

{ENRICHMENT: This section will be updated after Batch 3 with actual strategy metadata —
phase names, phase count, expected bead count, gate criteria. For now, read the strategy directly.}

---

## Step-by-Step Workflow

### Step 0: Read Source Documents (MANDATORY)
Read these IN THIS ORDER:
1. `docs/strategy/layers/layer-3-operational-hardening.md` — Spec — **READ FULLY**
2. `docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-tdd.md` — TDD — **READ FULLY**
3. `docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-strategy.md` — Strategy — **READ FULLY**
4. `crates/daemon/src/main.rs` — first 200 lines — **SKIM for module declarations**
5. `crates/daemon/src/jobs/runner.rs` — **STUDY types for integration beads**
6. Run `br list --json | head -20` — **STUDY existing bead structure**

### Step 1: Invoke the Skill
Run `/beads-from-sources`.

**Inputs:**
- Spec: `docs/strategy/layers/layer-3-operational-hardening.md`
- TDD: `docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-tdd.md`
- Strategy: `docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-strategy.md`

**Key arguments/context to provide the skill:**
- Bead prefix: `l3`
- Bead CLI: `br`
- Each bead must embed verbatim: the relevant spec section(s) and the TDD test IDs it covers
- Dependencies between beads follow strategy phase ordering
- Beads must name EXACT file paths for implementation targets
- Mixed stack: Rust beads use `cargo check/test/clippy`, TS beads use `bun build/test/lint`
- Quality gates per bead: build + test + lint (stack-specific)

### Step 2: Feature-Specific Guidance
While the skill runs, pay attention to:

**Bead structure for the 12 new Rust modules:**
Each module typically gets one bead containing:
- Implementation: create `crates/daemon/src/{module}/{file}.rs`
- Module declaration: add `mod {module}` to `main.rs`
- Unit tests: inline `#[cfg(test)]` module OR dedicated test file
- Integration with existing types (references to `JobRunner`, `SessionReaper`, etc.)

**The DB migration is ONE bead:**
- All 5 new tables in `migrations/0005_l3_operational_hardening.sql`
- ALTER statements for modified tables
- Must be the FIRST bead (everything depends on schema)

**Skill directory beads are filesystem beads:**
- Create directory structure, `SKILL.md`, schemas, canary inputs, golden assertions
- No compiled code — just files
- Can run in parallel with Rust/TS work

**TS module beads:**
- `packages/intelligence/src/relations/resolve-process-relations.ts` + test file
- `packages/manifests/render-error-manifest.ts` + template
- Use `bun test` not `cargo test`

**Integration beads (connecting components):**
- Cron scheduler → Job Runner → Resource Arbiter flow
- Source health → Circuit breaker → Scheduler suppression
- Run manifest writer → Validation executor → Verdict handling
- These are distinct from module beads — they wire things together

### Step 3: Verify Output
After the skill completes, verify:
- [ ] All beads created via `br list --json | grep l3`
- [ ] Bead count in range 25-45
- [ ] Every strategy phase has beads assigned
- [ ] Dependencies form a DAG (no cycles)
- [ ] Every bead names exact file paths (not "create the module")
- [ ] Every bead embeds spec section + TDD test IDs verbatim
- [ ] DB migration bead has no dependencies (it's the root)
- [ ] Skill directory beads are independent of Rust/TS work
- [ ] TS beads use `bun` commands, Rust beads use `cargo` commands
- [ ] Each bead has acceptance criteria (from spec) and test IDs (from TDD)
- [ ] Run `br ready` — at least the migration bead should be ready

### Step 4: Follow-Up Pass (Sub-Optimization)

> Send this as a follow-up message in the same session after the main work is complete.

ULTRATHINK. Re-read the beads against the codebase:

1. **Run `br list --json`** — count beads. Report: total, per strategy phase, ready count.

2. **Check dependency chain.** For each bead, verify its `blockedBy` list is accurate. Example: the validation executor bead MUST be blocked by the validation policy bead AND the run manifest writer bead. Missing dependencies = premature execution.

3. **Verify file paths.** For each bead that names a file path, verify the parent directory exists (or another bead creates it). Example: `crates/daemon/src/validation/policy.rs` requires the `validation/` directory — is directory creation in the bead or a dependency?

4. **Check `main.rs` modifications.** Multiple beads may modify `main.rs` (adding module declarations, migration calls, cron startup). Are these beads properly sequenced to avoid merge conflicts? Consider if a single "main.rs integration" bead should collect these changes.

5. **Verify spec coverage.** Walk every L3-FR-x.y.z requirement. Is there a bead that covers it? No requirement should be unassigned.

6. **Run `bin/br-sync`** — export to JSONL for git tracking.

Report: bead count, coverage gaps, dependency issues, parallel execution opportunities.

---

## Key Reminders

- Beads embed spec + TDD verbatim — NOT paraphrased
- Strategy provides ordering — beads follow phase sequence
- Every bead is self-contained — agent implements from bead alone, spec as tiebreaker only
- File paths must be EXACT — no "find the file" or "create somewhere"
- Quality gates are bead-specific: Rust beads = cargo check + test + clippy; TS beads = bun build + test + lint
- The migration bead is the root — everything depends on it
- Beads that modify `main.rs` need careful sequencing
- Run `bin/br-sync` after creation (NOT raw `br sync` which may run git commands)
