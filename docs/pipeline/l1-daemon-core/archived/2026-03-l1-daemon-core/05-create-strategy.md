# L1 Daemon Core — Create Implementation Strategy

## First Action

Invoke the `/spec-to-strategy` skill.

---

## Your Mission

Create a lightweight **implementation strategy** (SOT #3) for Layer 1 — the Rust daemon core. The strategy defines **build order, phase groupings, dependency graphs, and gate criteria** — NOT what to build (that's the spec) or how to test it (that's the TDD).

This is an ordering document. It answers: "In what sequence do we implement the 17 daemon files, 2 skills, 1 migration, and 10 test files such that each phase is independently testable and builds on the previous?"

---

## Source Documents (Read In This Order)

### Primary Input (CREATE FROM THIS)

```
docs/strategy/layers/layer-1-thin-local-runner.md   — Spec (SOT #1) — READ FULLY
```

### Source of Truth Context

```
docs/pipeline/l1-daemon-core/artifacts/l1-daemon-core-tdd.md       — TDD (SOT #2) — READ FULLY
docs/pipeline/l1-daemon-core/artifacts/l1-daemon-core-bootstrap.md  — Bootstrap — SKIM
```

### Codebase Context (STUDY FOR PATTERNS)

```
AGENTS.md                                            — Toolchain, coding discipline, beads workflow
docs/strategy/layer-strategy.md                      — §13 Stack Sequencing (Rust build order)
docs/strategy/cross-reference-index.md               — Component → section lookup
```

{ENRICHMENT: This section will be updated after Batches 1-2 with actual spec requirement IDs and TDD test counts. For now, read the artifacts directly.}

---

## Key Context From Codebase Research

- **Greenfield Rust workspace** — no existing code. Strategy must bootstrap from `Cargo.toml` + `rust-toolchain.toml`
- **17 daemon files** with clear dependency graph:
  - Foundation: `main.rs`, `db/sqlite.rs` (must exist first)
  - PTY layer: `pty/session.rs` → `pty/capture.rs` → `pty/health.rs` (sequential deps)
  - Adapter layer: `agents/adapter.rs` (trait) → `agents/codex.rs`, `agents/claude.rs` (parallel)
  - Artifact layer: `agents/artifacts.rs`, `agents/contracts.rs` (parallel, both depend on adapter)
  - Runner: `jobs/runner.rs` (depends on adapter + artifact + PTY + db)
  - Prompt: `prompt/renderer.rs` (depends on runner for invocation context)
  - Errors: `errors/manifest.rs`, `errors/markdown.rs` (parallel, depend on runner for context)
  - IPC: `ipc/descriptor.rs`, `ipc/handshake.rs` (parallel, depend on db + daemon lifecycle)
  - Reaper: `sessions/reaper.rs` (depends on PTY session for pid/pgid mapping)
- **Natural phase boundaries:**
  1. Workspace + DB + migration (can test: schema loads, PRAGMAs work)
  2. PTY session + capture (can test: spawn process, capture bytes, cleanup)
  3. Adapter trait + one adapter (can test: launch spec, capability probe, event parsing)
  4. Artifact store + result contracts (can test: chunk storage, schema validation)
  5. Session reaper (can test: cleanup verdicts, orphan detection)
  6. Job runner + prompt renderer (can test: job lifecycle, kickoff composition)
  7. Error manifests (can test: error generation, Markdown rendering)
  8. IPC descriptor + handshake (can test: daemon discovery, handshake protocol)
  9. Skills (FTS ingestion + manifest generation) + skill linting
  10. Integration: full ingestion flow + Experiment 4

---

## Step-by-Step Workflow

### Step 0: Read Source Documents (MANDATORY)

1. `docs/strategy/layers/layer-1-thin-local-runner.md` — **READ FULLY** (the spec with requirement IDs)
2. `docs/pipeline/l1-daemon-core/artifacts/l1-daemon-core-tdd.md` — **READ FULLY** (test IDs for gate criteria)
3. `AGENTS.md` — **SKIM** (beads workflow, coding discipline)
4. `docs/strategy/layer-strategy.md` §13 — **SKIM** (stack sequencing)

### Step 1: Invoke the Skill

Run `/spec-to-strategy`.

**Inputs:**
- Spec: `docs/strategy/layers/layer-1-thin-local-runner.md`
- TDD: `docs/pipeline/l1-daemon-core/artifacts/l1-daemon-core-tdd.md`
- Output: `docs/pipeline/l1-daemon-core/artifacts/l1-daemon-core-strategy.md`

**Key arguments/context to provide the skill:**
- This is a Rust-only layer (no TypeScript builds in L1 scope)
- Single crate: `crates/daemon` — no inter-crate dependencies to resolve
- Build order follows Rust module dependency graph (can't use a type before defining it)
- Gate criteria should reference specific TDD test IDs
- Each phase should be independently compilable (`cargo check` passes after each phase)
- Beads will be created from phases — each phase becomes one or more beads

### Step 2: Feature-Specific Guidance

While the skill runs, pay attention to:

- **The PTY layer has a strict internal dependency chain.** `session.rs` must exist before `capture.rs` (capture reads from the PTY master fd that session creates). `health.rs` needs both session state and capture metrics. Don't parallelize these.

- **The two adapters (codex.rs, claude.rs) CAN be parallelized** — they both implement the same trait independently. But the trait (`adapter.rs`) must exist first.

- **The Session Reaper is L1-critical, not deferred.** It must be in an early phase because the cleanup verdict is required for session completeness. Don't push it to the end.

- **The IPC layer is optional for L1 functionality** (daemon works without desktop connection), but the spec says the socket must bind at startup. Place it after the core runtime works but before the full integration phase.

- **Skill linting (`skill_lint.rs`) is a standalone validation pass** — it can be implemented in parallel with the daemon runtime since it doesn't depend on the daemon being runnable.

- **The `0003_sqlite_runtime.sql` migration depends on L0's `0001_sqlite_state_initial.sql`** — the strategy must account for this dependency.

### Step 3: Verify Output

After the skill completes, verify:

- [ ] Strategy output file exists at `docs/pipeline/l1-daemon-core/artifacts/l1-daemon-core-strategy.md`
- [ ] All 17 daemon files are assigned to phases
- [ ] All 2 skills are assigned to phases
- [ ] All 10 test files are assigned to phases
- [ ] The migration file is assigned to Phase 1
- [ ] Each phase has explicit gate criteria (specific test IDs that must pass)
- [ ] No circular dependencies between phases
- [ ] Each phase is independently compilable (`cargo check` passes)
- [ ] The final phase includes the full integration test (Experiment 4)
- [ ] Total phases: 8-12 (fewer = too large to parallelize; more = too fragmented)

### Step 4: Follow-Up Pass (Sub-Optimization)

> Send this as a follow-up message in the same session after the main work is complete.

ULTRATHINK and verify:

1. Open the strategy and trace the dependency graph. Can you draw a DAG from Phase 1 to the final phase? Are there any phases that could be reordered without breaking dependencies?
2. Open `docs/strategy/layers/layer-1-thin-local-runner.md` §1.5 — the 17-file listing. Is every file present in the strategy phases?
3. For each phase gate, verify the test IDs actually exist in the TDD document.
4. Is there a "walking skeleton" phase early on? (A phase where you can `cargo run` the daemon and it starts up, opens SQLite, binds the socket, and exits cleanly)
5. Are the skill files (`skills/procurement-ingestion/fts/`, `skills/manifest-generation/`) in a phase that comes AFTER the adapter and runner phases? (Skills need the execution infrastructure to be testable)

---

## Key Reminders

- Strategy is ORDERING ONLY — no spec content, no TDD content. Reference by ID.
- Each phase should be achievable in 1-3 beads (not monolithic multi-day phases)
- Gate criteria must be specific test IDs, not "all tests pass"
- The spec's Definition of Done (§5.1) is the FINAL gate — the strategy's gates are intermediate
- Account for the L0 dependency: the TypeScript intelligence pipeline must be available when the daemon tries to invoke skills
- Bead prefix is `l1` — phases will map to bead groups like `l1-phase-1`, `l1-phase-2`, etc.
