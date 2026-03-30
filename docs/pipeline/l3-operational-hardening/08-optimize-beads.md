# L3 Operational Hardening — Optimize Beads (Fresh-Eyes Audit)

## Your Mission

You are performing a **comprehensive fresh-eyes audit** of all beads for L3 Operational Hardening. You're checking bead quality — are they self-contained, do they embed spec/TDD verbatim, are dependencies correct, are file paths exact?

**You are reading, analyzing, and improving beads — not writing code.**

---

## What To Read (Do This First)

### The Beads to Audit (TARGET)
```bash
br list --json                  # Full inventory — read EVERY bead
br show <id> --json             # Individual bead detail — spot-check at least 10
```

### Verify Against (SOURCE OF TRUTH)
```
docs/strategy/layers/layer-3-operational-hardening.md    — Spec (SOT #1) — Every requirement covered?
docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-tdd.md — TDD (SOT #2) — Every test ID assigned?
docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-strategy.md — Strategy (SOT #3) — Phase ordering respected?
```

### Codebase (GROUND TRUTH)
```
crates/daemon/src/main.rs                                — Module declarations (verify bead file paths match)
crates/daemon/src/jobs/runner.rs                         — JobRunner types (verify bead references)
crates/daemon/src/agents/artifacts.rs                    — Manifest types (verify bead references)
crates/daemon/src/db/sqlite.rs                           — StateDb (verify migration bead)
crates/daemon/Cargo.toml                                 — Dependencies (verify crate addition beads)
packages/intelligence/src/                               — TS module paths (verify TS bead file paths)
packages/manifests/                                      — TS manifest paths (verify TS bead file paths)
```

---

## Think Independently

### 1. Self-Containment Check
For every bead, ask: "Could an agent implement this from the bead description alone?"
- [ ] Does the bead embed the relevant spec section verbatim (not paraphrased)?
- [ ] Does the bead list specific TDD test IDs it must satisfy?
- [ ] Does the bead name EXACT file paths for targets?
- [ ] Does the bead specify the quality gate commands (build/test/lint)?
- [ ] Does the bead have concrete acceptance criteria?

Spot-check at least 10 beads (mix of Rust, TS, skill, migration).

### 2. Dependency DAG Validation
- [ ] No circular dependencies
- [ ] Migration bead has no predecessors (it's the root)
- [ ] Each bead lists all beads it's blocked by
- [ ] Phase ordering from strategy is respected
- [ ] Integration beads come after their component beads
- Run: `br ready` — only root beads should be ready initially

### 3. File Path Accuracy
For each bead that creates/modifies a file:
- [ ] Parent directory exists or is created by a dependency bead
- [ ] File path matches the module structure in `crates/daemon/src/`
- [ ] New Rust modules have corresponding `mod` declarations somewhere
- [ ] TS files are in the correct package directory
- [ ] Skill directories follow the `skills/{category}/{source}/` pattern

### 4. Coverage Completeness
- Walk all L3-FR-x.y.z requirement IDs from the spec
- Walk all L3-T-* test IDs from the TDD
- For each, verify a bead covers it
- Report: requirements covered / total, test IDs covered / total, any gaps

### 5. Bead Size Balance
- No bead should be so large it takes >2 hours to implement
- No bead should be trivial enough to not warrant a bead (combine with adjacent work)
- The migration bead should include ALL 5 tables (not one per table)
- Complex integration beads (wiring cron → runner → arbiter) may need explicit scope limits

### 6. Cross-Stack Consistency
- Rust beads use: `cargo check --all-targets`, `cargo test`, `cargo clippy --all-targets -- -D warnings`
- TS beads use: `bun run build`, `bun run test`, `bun run lint`
- Skill beads: no build/test commands (filesystem only), but may reference `br` skill lint
- Are the quality gates consistent across all beads of the same type?

---

## Drift Checklist

- [ ] Every spec requirement maps to at least one bead
- [ ] Every TDD test ID is assigned to at least one bead
- [ ] Bead ordering matches strategy phases
- [ ] No bead contains content not in the spec (no scope creep)
- [ ] No bead paraphrases spec content (must be verbatim)
- [ ] File paths in beads match actual codebase structure
- [ ] Dependency graph is a DAG (no cycles)
- [ ] Mixed-stack beads have correct quality gate commands
- [ ] Total bead count is in the 25-45 range

---

## Follow-Up Pass (Run After Fixes)

> After making your fixes, send this as a follow-up message in the same session.

ULTRATHINK. Re-read the fixes you just made:

1. **If you modified dependencies:** re-check the DAG. Did removing/adding a dependency create a cycle? Run `br ready` — do the right beads show as ready?

2. **If you merged/split beads:** did the coverage matrix stay 100%? No requirement or test ID should be orphaned.

3. **If you fixed file paths:** verify against the codebase. Does the parent directory exist?

4. **Count:** total beads, per-phase distribution, ready count. Report.

5. **Run `bin/br-sync`** — export to JSONL for git tracking.
