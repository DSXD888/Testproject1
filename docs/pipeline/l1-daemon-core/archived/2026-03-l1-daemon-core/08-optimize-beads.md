# L1 Daemon Core — Optimize Beads (Fresh-Eyes Audit)

## Your Mission

You are performing a **comprehensive fresh-eyes audit** of the beads created for Layer 1. Your job is to find scope drift (bead describes something not in the spec), content gaps (spec requirement not covered by any bead), dependency errors, and sizing problems.

**You are reading bead definitions and improving them — not writing code.**

---

## What To Read (Do This First)

### The Beads to Audit (TARGET)

```bash
br list --json           # Full bead inventory
br show <id> --json      # Individual bead detail (read EVERY bead)
```

Read EVERY bead. Not a sample — every single one.

### Verify Against (SOURCE OF TRUTH)

```
docs/strategy/layers/layer-1-thin-local-runner.md                   — Spec (SOT #1) — every L1-FR-* must be in a bead
docs/pipeline/l1-daemon-core/artifacts/l1-daemon-core-tdd.md        — TDD (SOT #2) — every test ID must be assigned
docs/pipeline/l1-daemon-core/artifacts/l1-daemon-core-strategy.md   — Strategy (SOT #3) — phase ordering must match
```

{ENRICHMENT: This section will be updated after Batch 4 with actual bead prefix, count, and phase distribution.}

---

## Think Independently

### 1. Spec Coverage — Zero Gaps

Walk every `L1-FR-*` requirement ID in the spec. For each one:
- Is it assigned to at least one bead?
- Does the bead embed the requirement's acceptance criteria verbatim?
- Is the bead's file listing sufficient to implement the requirement?

Flag any requirements with ZERO bead coverage.

### 2. TDD Coverage — Every Test Assigned

Walk every test ID in the TDD. For each one:
- Is it assigned to at least one bead?
- Does the bead include the test file in its file listing?
- Is the test in the same bead as the code it tests, or in a later bead?

Flag any test IDs with ZERO bead assignment.

### 3. Bead Sizing

- No bead should create more than 5 source files (too large for one agent session)
- No bead should create fewer than 1 source file (too granular)
- Estimate lines of code per bead: 100-500 lines is ideal, 500-1000 is acceptable, >1000 needs splitting
- The foundation bead (workspace + Cargo.toml + migration) may be large but is straightforward
- The integration bead should primarily be a TEST, not new source code

### 4. Dependency Correctness

- Run `bv --robot-insights | jq '.Cycles'` — there MUST be zero cycles
- Verify each bead's dependencies match the strategy phase ordering
- Verify no bead depends on something created in a later phase
- Check for missing dependencies: does the runner bead depend on the adapter bead?

### 5. Self-Containment Quality

Read 3 beads in detail. For each:
- Can an agent implement this bead WITHOUT opening any other document?
- Are the spec requirements quoted verbatim (not paraphrased)?
- Are the TDD test IDs listed with their full descriptions?
- Are the file paths absolute to project root?
- Are the build/test commands included?
- Is the `#![forbid(unsafe_code)]` constraint mentioned?

### 6. Naming and Labels

- Are bead titles descriptive? ("l1-pty-session" is good; "l1-phase-2-item-1" is bad)
- Are subsystem labels applied consistently?
- Are priorities set? (Foundation beads = P0, integration bead = P1, IPC = P2)

---

## Drift Checklist

- [ ] Total beads between 15-25
- [ ] Zero `L1-FR-*` requirements without bead coverage
- [ ] Zero TDD test IDs without bead assignment
- [ ] Zero dependency cycles
- [ ] All 17 daemon source files assigned to beads
- [ ] Both skills assigned to beads
- [ ] Migration file assigned to a bead
- [ ] All 10 test files assigned to beads
- [ ] `Cargo.toml` and `rust-toolchain.toml` in the foundation bead
- [ ] No bead references L3+ features (cron, resource arbiter, validation enforcement)

---

## Follow-Up Pass (Run After Fixes)

> After making your fixes, send this as a follow-up message in the same session.

1. Run `br list --json | jq 'length'` — is the count between 15-25?
2. Run `bv --robot-triage` — what does it recommend as the first bead to implement?
3. Verify the first recommended bead is the foundation (workspace + DB + migration), not something that depends on it.
4. Check that fixing one bead didn't break another's dependency chain.
