# Goods-Led Install/Maintenance — Optimize Beads (Fresh-Eyes Audit)

## Your Mission

You are performing a **comprehensive fresh-eyes audit** of the bead set before the dedicated audit step. The bead set is what agents actually execute — each must be self-contained and implementable from the bead alone. You are hunting two failures: (a) beads that aren't truly self-contained (vague ACs, missing file lists, no embedded test), and (b) **scope drift** — a spec feature with no bead, or late-phase epics reduced to stubs.

**You are reading, analyzing, and improving existing beads — not writing code.**

---

## What To Read (Do This First)

### The Beads to Audit (TARGET — read EVERY bead)
```bash
br list --label goods-led-install --json     # full inventory
br show <id> --json                           # each bead in detail
```

### Verify Against (SOURCE OF TRUTH)
```
docs/pipeline/goods-led-install-maintenance/artifacts/goods-led-install-maintenance-spec.md       — every feature needs a bead
docs/pipeline/goods-led-install-maintenance/artifacts/goods-led-install-maintenance-tdd.md          — every bead's ACs = real test IDs
docs/pipeline/goods-led-install-maintenance/artifacts/goods-led-install-maintenance-strategy.md     — phase → epic ordering
```

### Codebase (GROUND TRUTH — confirm bead file references are accurate)
```
packages/intelligence/src/qualification/goods-qualification.ts:319-372
packages/intelligence/src/customs/description-quality.ts:23-36
crates/daemon/src/main.rs:8114,8548,9136-9290
packages/contracts-schema/src/index.ts:36,166,216
```

---

## Think Independently

### 1. Self-containment
Can each bead be implemented from its own description? Does it embed the relevant spec text + the exact TDD test(s) with pence-exact numbers? Does it have a `### Files to Create/Modify` section?

### 2. Coverage completeness
Build the spec-feature → bead matrix. Is every spec requirement covered at the right tier (operator-decided 2026-06-02; see `manifest.yaml` SCOPE TIERS)? **Tier-A — full beads:** the NEW `description-quality.test.ts`; both rollup sites; the schema-epoch 4-constant bump; `MaintenancePeriodEstimate`; access-equipment auto-selection; rate-refresh skill; `overlay-labour-cost`; basic banded permit/TM. **Tier-B — one `research-spike` bead each (present, not over-scoped):** live enrichment lane; installer-quote ingestion; detailed per-authority/per-zone permit sub-model.

### 3. Phase coverage equality
Count beads per phase epic. Are late epics realistically sized vs early ones? A stub late epic is hidden scope drift.

### 4. Dependency correctness
Do schema/type beads block consumer beads? Does the Phase-2-gate bead depend on the rollup + labour-skill beads? Does the labour-skill bead depend on the reference-data bead?

### 5. Accuracy of file references
Spot-check bead file:line references against the codebase. A bead that says "edit the rollup" without naming both `main.rs:8114` and `:8548` is incomplete.

### 6. Test-bead completeness
Are the test beads embedding the actual assertions (TS-QUAL-003a/b split, the 8-row truth table, 11 Appendix-E manifests, 7 canary cases incl. the NW band)? Or do they just say "add tests"?

---

## Scope Drift Detection (MANDATORY — BLOCKING)

### Red-Flag Language Scan
Search ALL **Tier-A** beads for: "post-MVP," "Phase 2+," "stretch goal," "nice to have," "deferred," "out of scope," "can be added later," "simplified for initial implementation." BLOCKING if found. (Tier-B `research-spike` beads legitimately frame their work as research/design — correct, not drift.)

### Coverage Verification (two tiers — see `manifest.yaml` SCOPE TIERS)
For EVERY spec feature, identify the implementing bead(s). Tier-A with ZERO full beads → BLOCKING. Tier-B with no `research-spike` bead → BLOCKING. A Tier-B item ballooned into a full implementation epic → flag for downgrade (false precision).

### Phase Coverage Equality
Count beads per strategy phase. If later **Tier-A** phases have significantly fewer beads than their spec surface justifies → investigate and equalize. (Tier-B research-spike phases are *expected* small — that's correct.)

### Drift Checklist
- [ ] Every TIER-A feature → ≥1 full bead; every TIER-B item → exactly one `research-spike` bead
- [ ] Both rollup sites covered (main.rs:8114 + 8548)
- [ ] Schema-epoch bump bead covers all 4 constants + Supabase + projection migration
- [ ] NEW `description-quality.test.ts` has a bead
- [ ] `MaintenancePeriodEstimate` + per-period margin check has a bead
- [ ] TIER A: access-equipment, rate-refresh, overlay, basic banded permit/TM — each has a full bead
- [ ] TIER B: live-enrichment, installer-quote, detailed per-zone permit — each has exactly one `research-spike` bead (present, not expanded into an epic)
- [ ] Every bead labelled `goods-led-install`, parented under the implementation epic
- [ ] Late Tier-A phases are not stubs (Tier-B spikes legitimately are small)
- [ ] No red-flag language in any Tier-A bead

---

## Follow-Up Pass (Run After Fixes)
> After making your fixes, send this as a follow-up message in the same session.

Re-read the beads you edited or created. Did adding a bead break another's dependency chain (e.g. a new bead that should block the Phase-2 gate but isn't wired)? Re-run the coverage matrix — is it 100%? Re-run `bin/br-sync` and confirm `.beads/issues.jsonl` reflects your changes. Verify ZERO scope-drift language remains and every spec feature still maps to a bead.
