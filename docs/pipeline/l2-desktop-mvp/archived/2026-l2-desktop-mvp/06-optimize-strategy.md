# L2 Desktop MVP — Optimize Strategy (Fresh-Eyes Audit)

## Your Mission

You are performing a **comprehensive fresh-eyes audit** of the Layer 2 implementation strategy. The strategy defines the build order, phase dependencies, and bead grouping for a mixed Rust + TypeScript layer creating a Tauri v2 desktop app with shared UI packages, live streaming, and a new procurement skill.

**You are reading, analyzing, and improving an existing document — not writing code.**

---

## What To Read (Do This First)

### The Document to Audit (TARGET)
```
docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-strategy.md      — Strategy (created in previous step)
```
Read fully. This is the primary audit target.

### Verify Against (SOURCE OF TRUTH)
```
docs/strategy/layers/layer-2-single-operator-desktop.md                — Spec (SOT #1) — Every file must appear in some phase
docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-tdd.md          — TDD (SOT #2) — Every test must be assigned to a gate
```

### Codebase (GROUND TRUTH)
```
crates/daemon/src/ipc/handshake.rs         — Can the IPC extension phase actually work with the current protocol?
crates/daemon/src/main.rs                  — Does adding IPC commands require changes to main.rs?
crates/daemon/src/jobs/runner.rs           — Does the CF skill need Job Runner changes?
Cargo.toml                                 — Workspace members — does the strategy add src-tauri correctly?
skills/procurement-ingestion/fts/          — Does the CF skill phase match the FTS structure?
```

### Quality Bar (MATCH OR EXCEED)
```
docs/pipeline/l1-daemon-core/artifacts/l1-daemon-core-strategy.md      — L1 strategy (10 phases, 22 beads)
```

---

## Think Independently

### 1. Dependency DAG Correctness
Draw the dependency graph on paper (or in your head). For each edge:
- Is this dependency real (would removing it cause a compilation error or test failure)?
- Is there a shorter path (transitively implied dependency)?
- Are there missing edges (phase N uses types from phase M but doesn't declare the dependency)?

### 2. Parallelization Analysis
Which phases can run concurrently?
- Rust phases (IPC extension, CF skill) have no TypeScript dependencies
- `packages/*` phases have no `apps/desktop` dependencies
- The CF skill has no desktop dependencies at all
- Can multiple agents work on different phases simultaneously? The strategy should make this clear.

### 3. Build Order Feasibility
Does the phase ordering produce a compilable project at every gate?
- After Phase 1: `bun install` works, `cargo check` works
- After Phase 2: `bun run build` works for packages
- After Phase 5: `cargo check` works for Tauri backend
- After Phase 6: `bun run dev` starts and serves the SPA
- After Phase 10: `bunx playwright test` passes

### 4. Bead Size Calibration
For each phase, estimate the number and size of beads:
- A bead should represent 1-4 hours of work
- Files in the same phase should have high cohesion (changed together)
- A single React page + its hooks is ~1 bead
- A single shared component is ~1 bead (or 2-3 small ones grouped)
- IPC commands should be grouped by related functionality, not one-per-bead

### 5. Gate Criteria Specificity
Each phase gate should name:
- Specific quality commands (cargo check, cargo test, bun run build, etc.)
- Specific test files or test ID ranges that must pass
- Whether the gate is Rust-only, TS-only, or both
- Whether Playwright tests are part of the gate (only for later phases)

### 6. File Assignment Completeness
Cross-check against the manifest's `key_files_to_create`:
- Every file in the manifest must appear in exactly one phase
- No file should appear in multiple phases (single-owner principle)
- Test files should be in the same phase as or one phase after the code they test

### 7. Critical Path Optimization
What's the longest sequential chain?
- If scaffold → packages → transport → QueryService → pages → E2E → Tauri is the critical path, that's 7 sequential phases minimum
- Can any phase on the critical path be split to allow earlier work on later phases?
- The CF skill should be OFF the critical path (fully parallel)

### 8. Risk Phasing
Does the strategy tackle high-risk items early?
- DaemonBridge ↔ daemon IPC is the highest-risk integration point (mismatched protocols)
- Commit fence correctness is the highest-risk data integrity concern
- Live streaming (Channel<T>) is the highest-risk real-time concern
- These should NOT be in the last phases

---

## Drift Checklist

- [ ] Every file from the spec's §1.9 file listing appears in exactly one phase
- [ ] Every shared component from §2.1 appears in a phase
- [ ] Every DaemonBridge command from §1.5 is covered by the IPC extension phase
- [ ] Every TDD test file is assigned to a phase gate
- [ ] The CF skill phase has no desktop dependencies
- [ ] No phase has more than 5 production files (if so, split it)
- [ ] Gate criteria include both Rust and TypeScript commands where applicable
- [ ] The strategy does NOT duplicate spec content (routes to L2-FR-* IDs)
- [ ] Expected total bead count is 20-30

---

## Follow-Up Pass (Run After Fixes)

> After making your fixes, send this as a follow-up message.

1. Re-read the dependency DAG. Did your fixes create any cycles?
2. Count the phases. Is the count reasonable? (Target: 8-12 phases, matching L1's granularity)
3. Run through a mental simulation: "If I had 3 agents, what would each do in parallel?" Does the strategy support this?
4. Check that gate criteria don't reference test IDs that don't exist in the TDD
5. Verify the critical path length. If it's > 8 sequential phases, look harder for parallelism.
