# L1 Daemon Core — Optimize Strategy (Fresh-Eyes Audit)

## Your Mission

You are performing a **comprehensive fresh-eyes audit** of the implementation strategy for Layer 1. The strategy was created in Step 05 from the spec and TDD. Your job is to find ordering bugs, missing dependencies, unrealistic gate criteria, and phase sizing problems.

**You are reading, analyzing, and improving an existing document — not writing code.**

---

## What To Read (Do This First)

### The Document to Audit (TARGET)

```
docs/pipeline/l1-daemon-core/artifacts/l1-daemon-core-strategy.md  — Strategy (created in previous step) — READ FULLY
```

### Verify Against (SOURCE OF TRUTH)

```
docs/strategy/layers/layer-1-thin-local-runner.md                   — Spec (SOT #1) — every file must be in a phase
docs/pipeline/l1-daemon-core/artifacts/l1-daemon-core-tdd.md        — TDD (SOT #2) — gate test IDs must exist
```

### Codebase (GROUND TRUTH)

```
AGENTS.md                                                            — Beads workflow, coding conventions
docs/strategy/layer-strategy.md                                      — §13 Stack Sequencing
```

---

## Think Independently

### 1. Rust Compilation Order

Rust modules have implicit dependencies through `use` statements. Verify:
- Can each phase actually compile independently? (`mod adapter;` in `main.rs` requires `adapter.rs` to exist)
- Are modules declared in the right order? (You can't `use crate::agents::adapter::AgentAdapter` if the `agents/` module doesn't exist yet)
- Does the strategy handle `mod.rs` files (or `mod` declarations in parent modules)?
- What about the `lib.rs` vs `main.rs` split — is the daemon a binary crate, a library crate, or both?

### 2. Phase Sizing for Beads

Each phase should map to 1-3 beads. Verify:
- No phase has more than 5 files (too large for a single bead)
- No phase has fewer than 1 file (too granular, creates coordination overhead)
- Estimated effort per phase is roughly balanced (no phase is 10x larger than another)
- The "walking skeleton" phase (daemon starts, opens DB, binds socket) is early and small

### 3. Test Infrastructure Bootstrap

Tests need infrastructure that may not be in the same phase as the code under test. Verify:
- Test helper binaries (for process supervision tests) — when are they created?
- SQLite test fixtures (L0 schema + L1 migration) — when are they set up?
- Test assertion helpers (for comparing chunked artifacts, Markdown output) — when are they created?
- Dev-dependencies in `Cargo.toml` — are they listed in the bootstrap phase?

### 4. Skill Phase Placement

Skills need the daemon runtime to be testable. Verify:
- The FTS ingestion skill phase comes AFTER: adapter + runner + PTY + capture
- The manifest generation skill phase comes AFTER: adapter + runner + artifact store
- Skill linting comes BEFORE or PARALLEL WITH skills (since it validates them)
- The L0 TypeScript pipeline dependency is accounted for (skills invoke TypeScript code via the adapter)

### 5. Integration Phase Completeness

The final phase should prove the full loop. Verify:
- Experiment 4 (10 consecutive batch runs) is in the final phase
- The full ingestion flow (§1.6, 14 steps) is tested end-to-end
- All 13 Definition of Done criteria from §5.1 are covered by the cumulative gates
- The strategy's final gate = the spec's exit criteria (Experiment 4 passes AND first live E2E loop completes)

### 6. Parallel vs Sequential Ordering

Some phases can be done in parallel by different agents. Verify:
- Are parallelizable phases clearly marked?
- Do parallel phases have no shared file dependencies?
- Is there a clear merge point where parallel streams reconverge?

---

## Drift Checklist

- [ ] Every one of the 17 daemon files from §1.5 is assigned to a phase
- [ ] Both skills are assigned to a phase
- [ ] The migration file (`0003_sqlite_runtime.sql`) is in Phase 1
- [ ] All 10 test files are assigned to phases
- [ ] `Cargo.toml` (workspace + daemon crate) is in Phase 1
- [ ] `rust-toolchain.toml` is in Phase 1
- [ ] Gate criteria reference actual TDD test IDs (not generic "tests pass")
- [ ] No phase requires features from a later phase
- [ ] The final phase produces all 13 Definition of Done outcomes

---

## Follow-Up Pass (Run After Fixes)

> After making your fixes, send this as a follow-up message in the same session.

1. Draw the phase dependency DAG on paper (or mentally). Is it a clean linear chain, a wide parallel fan, or a messy web? The ideal shape for L1 is: narrow foundation → parallel middle → convergent integration.
2. Count phases. 8-12 is ideal. Fewer means phases are too large; more means coordination overhead exceeds implementation time.
3. For each gate, mentally run `cargo test` with only the files created up to that phase. Would it actually pass?
4. Check that no phase accidentally includes L3+ code (cron scheduler, resource arbiter, validation enforcement).
