# L3 Operational Hardening — Bead Audit (Spec Drift Detection)

## First Action

Invoke the `/bead-plan-audit` skill.

---

## Your Mission

You are performing a **dedicated, systematic audit** of all beads for L3 Operational Hardening. This is the LAST GATE before execution. Your job is to verify **zero spec drift** — every spec requirement is covered, every TDD test is assigned, no silent scope cuts.

**This audit is different from the optimize pass (08).** The optimize pass checked bead quality. THIS audit checks spec-to-bead coverage with zero tolerance for gaps.

---

## Source Documents

### Beads (TARGET — audit EVERY bead)
```bash
br list --json                  # Full inventory
br show <id> --json             # Individual bead detail
br ready                        # Work queue
```
Read EVERY bead. Not a sample — every single one.

### Verify Against
```
docs/strategy/layers/layer-3-operational-hardening.md    — Spec (SOT #1) — Every requirement must have a bead
docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-tdd.md — TDD (SOT #2) — Every test ID must be assigned
docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-strategy.md — Strategy (SOT #3) — Phase ordering must match
```

### Codebase
```
crates/daemon/src/main.rs                                — Verify bead file path accuracy
crates/daemon/src/jobs/runner.rs                         — Verify JobRunner modification beads
crates/daemon/src/agents/artifacts.rs                    — Verify manifest extension beads
crates/daemon/src/agents/contracts.rs                    — Verify validation integration beads
crates/daemon/src/errors/manifest.rs                     — Verify error code addition beads
crates/daemon/src/pty/session.rs                         — Verify PTY hardening beads
crates/daemon/src/sessions/reaper.rs                     — Verify reaper extension beads
crates/daemon/src/db/sqlite.rs                           — Verify migration bead
crates/daemon/src/ipc/command_router.rs                  — Verify IPC extension beads
packages/intelligence/src/                               — Verify TS bead paths
```

---

## Audit Methodology

Invoke `/bead-plan-audit` which handles the detailed methodology. Add these L3-specific checks:

### L3-Specific Checks

**1. Twelve New Daemon Modules — Is Each Covered?**
Walk the 12 modules from §1.3:
- [ ] `scheduler/cron.rs` → bead exists, names correct path, embeds §1.5 spec
- [ ] `sources/registry.rs` → bead exists, names correct path, embeds source registry spec
- [ ] `jobs/claim.rs` → bead exists, names correct path, embeds concurrent claiming spec
- [ ] `pty/reaper.rs` → bead exists (extending existing `sessions/reaper.rs`), embeds §1.3 reaper spec
- [ ] `pty/hardening.rs` → bead exists, names correct path, embeds §1.6 PTY hardening spec
- [ ] `agents/arbiter.rs` → bead exists, names correct path, embeds §1.4 resource arbiter spec
- [ ] `db/checkpoints.rs` → bead exists, names correct path, embeds checkpoint spec
- [ ] `tracing/structured.rs` → bead exists, names correct path, embeds §4.4 observability spec
- [ ] `tracing/forensic.rs` → bead exists, names correct path, embeds §4.4 forensic spec
- [ ] `validation/policy.rs` → bead exists, names correct path, embeds §2.3 policy spec
- [ ] `validation/executor.rs` → bead exists, names correct path, embeds §2.4 executor spec
- [ ] `validation/manifest_writer.rs` → bead exists, names correct path, embeds §2.1-2.2 manifest spec

**2. Five DB Tables — Migration Bead Complete?**
The migration bead must include ALL of:
- [ ] `validation_verdicts` DDL (from §4.1)
- [ ] `source_runtime_state` DDL (from §4.1)
- [ ] `listing_evidence` DDL (from §4.1)
- [ ] `process_relations` DDL (from §4.1)
- [ ] `process_aliases` DDL (from §4.1)
- [ ] ALTER statements for `workflow_jobs` and `agent_session_logs` (§4.2)

**3. Ten New Skills — All Directories Covered?**
- [ ] `skills/auth-refresh/alibaba/` → bead with SKILL.md, schemas
- [ ] `skills/auth-refresh/1688/` → bead with SKILL.md, schemas
- [ ] `skills/source-health/fts/` → bead with SKILL.md, schemas
- [ ] `skills/source-health/alibaba/` → bead with SKILL.md, schemas
- [ ] `skills/validation/base/` (6 files) → bead(s) covering all: SKILL.md, result.schema.json, checkpoint.schema.json, canary.inputs.jsonl, golden.assertions.yaml, validator-rubric.yaml

**4. Three TS Modules — Correct Paths and Tests?**
- [ ] `packages/intelligence/src/relations/resolve-process-relations.ts` → bead with `bun test` gate
- [ ] `packages/manifests/render-error-manifest.ts` → bead with `bun test` gate
- [ ] `packages/manifests/error.md.ts` → bead (may be combined with render-error-manifest)

**5. Eighteen Test Files — All Assigned?**
Walk §5.2's test file list:
- [ ] 4 fault injection tests → assigned to beads
- [ ] 2 replay tests → assigned to beads
- [ ] 2 scheduling tests → assigned to beads
- [ ] 3 skill lifecycle tests → assigned to beads
- [ ] 6 validation tests → assigned to beads
- [ ] 1 relation resolution test → assigned to bead

**6. Worker-Validator System — Deep Coverage?**
The validation system is L3's most complex subsystem. Verify:
- [ ] Run manifest fields from §2.1-2.2 are in manifest writer bead
- [ ] Policy engine modes (always/sample/trigger/skip) from §2.3 are in policy bead
- [ ] Risk-stratified sampling factors from §2.3 are in policy bead
- [ ] Validator executor kickoff composition from §2.4 is in executor bead
- [ ] Verdict handling by Job Runner from §2.4 is in an integration bead
- [ ] Outcome classes from §2.5 are in a bead (likely artifacts.rs extension)
- [ ] Dry-run isolation guarantees from §2.6 are in validation dry-run test beads
- [ ] Fail-open semantics from §2.7 are in dedicated test beads
- [ ] Trust-state lattice from §2.7 is in a bead
- [ ] Base validator skill from §2.8 is in skill directory bead
- [ ] Calibration from §2.9 is in test/fixture beads

---

## Follow-Up Pass

> After the audit is complete and all fixes applied:

ULTRATHINK.

1. Re-read your own fixes. Did fixing one bead break another's dependencies?
2. Re-run the coverage matrix. Is it still 100%?
3. Verify bead count matches strategy phase task count.
4. Run `br ready` — what's the current ready queue?
5. Run `bin/br-sync` — export to JSONL.

Report: total beads, coverage percentage (requirements and tests), any remaining gaps, confidence level (high/medium/low) that beads are ready for execution.
