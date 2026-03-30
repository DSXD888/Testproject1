# L2 Desktop MVP — Create Beads

## First Action

Invoke the `/beads-from-sources` skill.

---

## Your Mission

Create **executable beads** for Layer 2 (Single-Operator Desktop MVP) from the three Source of Truth documents. Each bead is a self-contained unit of work that embeds spec requirements and TDD test IDs verbatim — an implementation agent can build from the bead alone without reading the full spec.

The strategy provides ORDERING ONLY — it determines which phase each bead belongs to and what dependencies it has. All content comes from the spec and TDD.

---

## Source Documents (Read In This Order)

### Primary Inputs (CONTENT COMES FROM THESE)
```
docs/strategy/layers/layer-2-single-operator-desktop.md                — Spec (SOT #1) — READ FULLY
docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-tdd.md          — TDD (SOT #2) — READ FULLY
```

### Ordering Input (PHASE STRUCTURE FROM THIS)
```
docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-strategy.md      — Strategy (SOT #3) — READ FULLY
```

### Codebase Context (STUDY FOR FILE PATHS AND TYPES)
```
crates/daemon/src/agents/adapter.rs        — Types that L2 beads reference (ControlEvent, OutputFrame)
crates/daemon/src/ipc/handshake.rs         — IPC protocol that IPC extension beads modify
crates/daemon/src/jobs/runner.rs           — Job Runner that CF skill bead extends
crates/daemon/src/db/sqlite.rs             — commit_clock that QueryService bead reads
migrations/0001_sqlite_state_initial.sql   — Schema tables that QueryService beads query
skills/procurement-ingestion/fts/          — Skill directory structure that CF bead follows
```

### Quality Bar
```
docs/pipeline/l1-daemon-core/manifest.yaml                             — L1 had 22 task beads — match quality
```

---

## Key Context From Codebase Research

- **L1 created 30 beads** (1 epic + 22 tasks + 7 fix-beads) — L2 should target 20-30 task beads
- **Bead prefix: `l2`** — bead CLI is `br`
- **Mixed-stack beads:** Some beads are Rust-only, some TypeScript-only, some mixed. Each bead must declare which quality gates apply.
- **The bead description must embed:** file listing, `L2-FR-*` requirement IDs (verbatim from spec), test IDs (verbatim from TDD), acceptance criteria, and dependencies on other beads
- **File reservation guidance:** Each bead should list the files it touches so agents can reserve them

{ENRICHMENT: This section will be updated after Batch 3 with actual phase names, phase count, tasks per phase, and gate criteria from the strategy.}

---

## Step-by-Step Workflow

### Step 0: Read Source Documents (MANDATORY — before doing anything)
Read these IN THIS ORDER:
1. `docs/strategy/layers/layer-2-single-operator-desktop.md` — Spec — **READ FULLY**
2. `docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-tdd.md` — TDD — **READ FULLY**
3. `docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-strategy.md` — Strategy — **READ FULLY**
4. `crates/daemon/src/ipc/handshake.rs` — **STUDY for IPC bead content**
5. `migrations/0001_sqlite_state_initial.sql` — **REFERENCE for SQL query beads**

### Step 1: Invoke the Skill
Run `/beads-from-sources`.

**Inputs:**
- Spec: `docs/strategy/layers/layer-2-single-operator-desktop.md`
- TDD: `docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-tdd.md`
- Strategy: `docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-strategy.md`
- Bead prefix: `l2`
- Bead CLI: `br`

**Key arguments/context to provide the skill:**
- Each bead must declare its language: `rust`, `typescript`, or `mixed`
- Each bead must declare its quality gate commands (which subset of cargo/bun commands apply)
- Each bead must list the files it creates or modifies (for file reservation)
- Beads should embed spec text VERBATIM — don't paraphrase requirement descriptions
- Beads should reference TDD test IDs directly — "Implement to pass tests L2-T-QS-001 through L2-T-QS-005"

### Step 2: Feature-Specific Guidance
While the skill runs, pay attention to:

- **Workspace scaffold bead must be first and standalone:** It creates `package.json`, `tsconfig.json`, `bunfig.toml`, Tauri project structure. No other bead can start before this completes.
- **Shared package beads should be granular:** One bead per component (or 2-3 small components) — not "create all of packages/ui" as one bead.
- **IPC extension bead is critical:** It modifies `crates/daemon/` to add command dispatch for L2's 10 commands. This is a high-contention file area — the bead description must be very specific about which files change and how.
- **QueryService bead should list the exact SQL queries:** The bead should embed the read model interfaces from §1.4 so the agent knows exactly what queries to implement.
- **DaemonBridge bead should list all 10 commands:** Each command from §1.5 should be referenced by name with its return type.
- **Channel<T> streaming bead is complex:** It touches both Rust (daemon-side relay) and TypeScript (session-registry, session-buffer). Consider splitting into daemon-side and desktop-side beads.
- **CF skill bead is independent:** It should have NO dependencies on desktop beads. Only depends on workspace scaffold (Phase 1) and IPC extension (Phase 4).
- **Test beads should be paired with implementation beads:** Don't create separate "write tests for X" beads — each bead includes both implementation and tests.

### Step 3: Verify Output
After the skill completes, verify:
- [ ] Total bead count is 20-30 task beads (plus 1 epic container)
- [ ] Every `L2-FR-*` requirement is covered by at least one bead
- [ ] Every TDD test ID is assigned to exactly one bead
- [ ] Every file from the manifest's `key_files_to_create` appears in exactly one bead
- [ ] Bead dependencies match the strategy's phase DAG
- [ ] Each bead has: title, description, files, requirements (verbatim), test IDs, dependencies, language, quality gates
- [ ] Beads are right-sized: 1-4 hours of work each
- [ ] The workspace scaffold bead has no dependencies (it's the root)
- [ ] The CF skill bead is independent of desktop beads

### Step 4: Follow-Up Pass (Sub-Optimization)

> Send this as a follow-up message in the same session after the main work is complete. Use ULTRATHINK.

1. **Check bead self-containment:** Pick 3 random beads. Could an agent implement each one from the bead description alone, without reading the full spec? If the bead says "implement QueryService" without listing the 4 read models and their return types, it's not self-contained.

2. **Check file reservation feasibility:** For each bead, are the file paths specific enough for `file_reservation_paths()`? Vague paths like "apps/desktop/src/" are useless for reservations.

3. **Check dependency minimality:** For each dependency edge between beads, ask "is this a real compile-time or runtime dependency?" If removing the dependency just means the agent would need to read a different file first (not a compilation dependency), it should be a soft recommendation, not a hard dependency.

4. **Verify the IPC extension bead is complete:** It must add ALL 10 commands from §1.5 in one bead (or explicitly split across beads). Partial IPC extension creates untestable states.

5. **Verify Playwright test beads have daemon setup:** Any bead that includes Playwright E2E tests must describe how to start the daemon + dev server for testing. This is a bootstrap concern but must be called out in the bead.

6. **Run `br list --json | jq 'length'`** — confirm the bead count matches expectations.

---

## Key Reminders

- **Beads embed VERBATIM spec content** — no paraphrasing, no summarizing
- **Strategy provides ORDERING ONLY** — don't let strategy summaries replace spec content in beads
- **Each bead declares its language** (rust/typescript/mixed) and quality gates
- **Bead prefix: `l2`** — all beads use this prefix
- **File paths in beads must be specific** — "create `apps/desktop/src/pages/ContractList.tsx`", not "create contract list page"
- **Test IDs in beads must be specific** — "passes L2-T-PAGE-001 through L2-T-PAGE-008", not "all page tests pass"
- **L1 had 22 task beads for 17 files** — L2 should have 20-30 for ~28+ files
