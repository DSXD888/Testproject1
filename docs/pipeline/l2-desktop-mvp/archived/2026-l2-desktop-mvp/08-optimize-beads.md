# L2 Desktop MVP — Optimize Beads (Fresh-Eyes Audit)

## Your Mission

You are performing a **comprehensive fresh-eyes audit** of all Layer 2 beads. Each bead is a self-contained unit of work that an implementation agent will execute. The beads must be: self-contained (implementable from description alone), right-sized (1-4 hours), correctly ordered (dependencies are real), and complete (every spec requirement and TDD test is covered).

**You are reading, analyzing, and improving existing beads — not writing code.**

---

## What To Read (Do This First)

### The Beads to Audit (TARGET — read EVERY bead)
```bash
br list --json               # Full inventory
br show <id> --json          # Individual bead detail — read EVERY one
```
Read EVERY bead. Not a sample — every single one.

### Verify Against (SOURCE OF TRUTH)
```
docs/strategy/layers/layer-2-single-operator-desktop.md                — Spec (SOT #1) — Every L2-FR-* must have a bead
docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-tdd.md          — TDD (SOT #2) — Every test ID must be assigned
docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-strategy.md      — Strategy (SOT #3) — Phase ordering must match
```

### Codebase (GROUND TRUTH)
```
crates/daemon/src/ipc/handshake.rs         — Does the IPC bead reference the actual protocol?
crates/daemon/src/agents/adapter.rs        — Do streaming beads reference actual types?
crates/daemon/src/db/sqlite.rs             — Does the QueryService bead use the actual table?
migrations/0001_sqlite_state_initial.sql   — Do SQL beads reference actual columns?
skills/procurement-ingestion/fts/          — Does the CF bead match FTS structure?
```

---

## Audit Methodology

### 1. Coverage Matrix Cross-Check
For EVERY `L2-FR-*` requirement in the spec:
- [ ] Is it covered by at least one bead?
- [ ] Is the bead's description specific enough to verify the requirement?

For EVERY test ID in the TDD:
- [ ] Is it assigned to exactly one bead?
- [ ] Is the bead's description consistent with the test's assertions?

### 2. Self-Containment Test
For EVERY bead, answer:
- Could an agent implement this from the bead description alone?
- Does the bead embed the actual spec text (not "see §1.4")?
- Does the bead list exact files to create/modify?
- Does the bead list exact test IDs to satisfy?
- Does the bead list quality gate commands?

### 3. Dependency Correctness
For EVERY dependency edge:
- Is this a real compile-time dependency? (removing it causes a build error)
- Or is it a logical ordering preference? (should be soft, not hard)
- Are there missing dependencies? (bead uses types from another bead but doesn't declare the dependency)

### 4. Size Calibration
For EVERY bead:
- Estimate implementation time (1-4 hours target)
- If > 4 hours: SPLIT the bead
- If < 30 minutes: MERGE with an adjacent bead
- Check file count: 1-4 files per bead is typical

### 5. File Reservation Clarity
For EVERY bead:
- Are file paths specific and complete?
- Could `file_reservation_paths()` be called with these paths?
- Are there file conflicts between beads? (same file in multiple beads)

---

## Follow-Up Pass

> After the audit is complete and all fixes applied:

1. Re-read your own fixes. Did fixing one bead break another's dependencies?
2. Re-run the coverage matrix mentally. Is every `L2-FR-*` still covered?
3. Count beads by phase. Does the distribution make sense?
4. Verify bead count: 20-30 is the target. If significantly different, explain why.
5. Check that the workspace scaffold bead is still the root (no dependencies).
6. Check that the CF skill bead is still independent of desktop beads.
