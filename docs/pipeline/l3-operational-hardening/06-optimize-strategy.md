# L3 Operational Hardening — Optimize Strategy (Fresh-Eyes Audit)

## Your Mission

You are performing a **comprehensive fresh-eyes audit** of the L3 Operational Hardening strategy — the document that defines the implementation order, phase gates, and dependency graph for building Layer 3.

**You are reading, analyzing, and improving an existing document — not writing code.**

---

## What To Read (Do This First)

### The Document to Audit (TARGET)
```
docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-strategy.md — Strategy (SOT #3, created in step 05)
```
Read fully. This is the primary audit target.

### Verify Against (SOURCE OF TRUTH)
```
docs/strategy/layers/layer-3-operational-hardening.md    — Spec (SOT #1) — Every spec section must map to a phase
docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-tdd.md — TDD (SOT #2) — Every test file must map to a phase
```

### Codebase (GROUND TRUTH)
```
crates/daemon/src/main.rs                                — Module structure, bootstrap flow
crates/daemon/src/jobs/runner.rs                         — Central integration point — verify modification ordering
crates/daemon/src/agents/artifacts.rs                    — Run manifest types — verify extension ordering
crates/daemon/src/agents/contracts.rs                    — Validation types — verify integration ordering
crates/daemon/src/db/sqlite.rs                           — Migration point — verify Phase 1 includes this
crates/daemon/src/ipc/command_router.rs                  — IPC dispatch — verify new command ordering
crates/daemon/Cargo.toml                                 — Dependency additions — verify crate ordering
```

### Quality Bar (MATCH OR EXCEED)
```
docs/pipeline/l1-daemon-core/artifacts/l1-daemon-core-strategy.md — Best existing strategy
```

---

## Think Independently

### 1. Dependency Graph Validation
Draw the dependency graph mentally:
- DB migration → everything else
- Resource arbiter → cron scheduler (arbiter gates what cron enqueues)
- Source registry → source health canaries → circuit breaker
- Run manifest writer → validation policy → validation executor → base validator skill
- Validation executor → Job Runner modifications (sub-job spawning)
- Everything → skill promotion (capstone)
- Relation resolution (TS) → independent track (only needs DB tables)

**Check:** Is the strategy's phase ordering consistent with this graph? Are there hidden dependencies the strategy misses?

### 2. Phase Size Balance
Each phase should produce a meaningful shippable increment. Check:
- Is any phase too large (>12 beads)? It should be split.
- Is any phase too small (<3 beads)? It could be merged.
- Do phase sizes roughly correspond to implementation effort?
- Does Phase 1 (foundation) have the right scope? Too little = later phases blocked. Too much = long time to first gate.

### 3. Gate Criteria Specificity
Every phase gate must have SPECIFIC pass criteria:
- [ ] References TDD test IDs (not "relevant tests pass")
- [ ] References build commands (`cargo check --all-targets`, `cargo test`, `bun run test`)
- [ ] Has a clear definition of "done" for the phase
- [ ] Distinguishes between "code compiles" and "tests pass" gates

### 4. Parallel Execution Opportunities
Multiple agents can work simultaneously. Check:
- Can Rust daemon work and TS relation resolution run in parallel?
- Within the Rust daemon, can resource arbiter and PTY hardening run in parallel?
- Can skill directory creation (filesystem) run in parallel with Rust module work?
- Are parallel opportunities EXPLICITLY marked in the strategy?

### 5. Risk-Ordered Implementation
The spec identifies 8 risks (§5.4). Does the strategy address the highest risks first?
- "Validator false positive rate too high" → validation system should be early enough to start calibration
- "Source changes break multiple skills" → source health should come early
- "Cron edge cases" → cron scheduling should include timezone tests in its gate
- "Late merge invalidates decisions" → relation resolution should include late-merge scenario tests

### 6. Integration Points — Where Does `main.rs` Change?
`main.rs` (131K) is the daemon's spine. Track when it changes:
- Phase 1: Add module declarations, migration call, StateDbConfig changes
- Phase 2: Add cron scheduler startup, source health scheduling
- Phase 3: PTY hardening hooks in session spawn path
- Phase 4: Validation executor hooks in job completion path
- Phase N: IPC command additions for new features

**Is each `main.rs` modification in the right phase?** Modifying `main.rs` in every phase creates merge conflicts if agents work in parallel. Can modifications be batched?

### 7. Bead Granularity Preview
The strategy should give enough information to predict bead structure:
- Each new Rust module → 1-2 beads (implementation + tests)
- Each new DB table → typically part of a migration bead
- Each new skill → 1 bead per skill
- Each new TS module → 1 bead (implementation + tests)
- Integration beads → connect components together

Estimate total beads. Is it in the 25-45 range?

---

## Drift Checklist

- [ ] Every spec section (§1-§6) maps to at least one phase
- [ ] Every TDD test file maps to a phase
- [ ] No phase depends on work from a later phase
- [ ] Phase gates reference specific TDD test IDs
- [ ] DB migration is in Phase 1 (not deferred)
- [ ] `cron` and `sysinfo` crate additions are in the right phase
- [ ] Relation resolution (TS) is correctly identified as parallelizable
- [ ] Skill promotion is in the final or penultimate phase (it's a capstone)
- [ ] The strategy does not duplicate spec or TDD content
- [ ] The exit criteria (5 days unattended) is in the final gate

---

## Follow-Up Pass (Run After Fixes)

> After making your fixes, send this as a follow-up message in the same session.

ULTRATHINK. Re-read the fixes you just made:

1. **If you reordered phases:** trace the dependency graph again. Did reordering introduce any cycles?

2. **If you added gate criteria:** do the referenced TDD test IDs actually exist in the TDD? Grep mentally.

3. **If you merged/split phases:** does each phase still produce a shippable increment? Can you describe in one sentence what the daemon can do at the end of each phase that it couldn't before?

4. **Phase-to-bead mapping:** for each phase, estimate the bead count. Total should be 25-45. Report per-phase breakdown.

5. **Verify against L1 strategy structure:** does the L3 strategy use the same section headings, gate format, and dependency notation as L1? Consistency matters for the agents that execute these.

Report: phase count, estimated bead count per phase, total beads, any remaining ordering concerns.
