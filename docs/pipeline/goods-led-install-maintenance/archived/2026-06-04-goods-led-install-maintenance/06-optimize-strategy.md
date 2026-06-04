# Goods-Led Install/Maintenance — Optimize Strategy (Fresh-Eyes Audit)

## Your Mission

You are performing a **comprehensive fresh-eyes audit** of the strategy — SOT #3. The strategy is execution ORDER + gate criteria only. The two failure modes you are hunting: (a) a phase ordering that puts a consumer before its dependency, and (b) **scope drift via thin late phases** — where recurring-maintenance, the overlay, the gold-set, or the Future-tier items got hand-waved into a stub phase while the early phases are richly detailed.

**You are reading, analyzing, and improving an existing document — not writing code.**

---

## What To Read (Do This First)

### The Document to Audit (TARGET)
```
docs/pipeline/goods-led-install-maintenance/artifacts/goods-led-install-maintenance-strategy.md   — created in step 05
```

### Verify Against (SOURCE OF TRUTH)
```
docs/pipeline/goods-led-install-maintenance/artifacts/goods-led-install-maintenance-spec.md   — every requirement must have a phase
docs/pipeline/goods-led-install-maintenance/artifacts/goods-led-install-maintenance-tdd.md     — gate criteria must reference real test IDs
```

### Codebase (GROUND TRUTH — the real dependency order)
```
crates/daemon/src/main.rs:5361-5432,8086,8324,9136-9290   — write struct + rollup + live loop
packages/contracts-schema/src/index.ts                     — types consumed everywhere
packages/intelligence/src/qualification/goods-qualification.ts   — Phase-2 gate depends on rollup labour
```

### Quality Bar (MATCH OR EXCEED)
```
docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-strategy.md
```

---

## Think Independently

### 1. Dependency correctness
Trace every cross-phase edge. Does any phase read a field/type/skill that a later phase creates? The classic trap: the Phase-2 dominance gate reading labour cost before the rollup-integration phase produces it. The labour skill + its 4 reference JSON files before the rollup phase that invokes it.

### 2. Phase coverage equality (the scope-drift hotspot)
Count the requirements and estimated beads per phase. Are the late **Tier-A** phases (recurring maintenance, surfacing, overlay, gold-set, rate-refresh) treated with the SAME depth as the foundation/skill/qualification phases? A 1-bead "do the rest" Tier-A phase is a drift signal. **Tier-B** research-spike phases (live enrichment lane, installer-quote ingestion, detailed per-zone permit) are *expected* to be small — that's correct; just confirm they exist and are tracked, not dropped.

### 3. Gate criteria are real
Does each phase's gate reference actual TDD test IDs and the AGENTS.md stage gate — not generic "tests pass"? Is the Scenario-1 band hard-assert a gate on the labour-skill phase? Is the L0 expansion rule (new category needs its own gold-set) a gate on category go-live?

### 4. The "two halves ship together" invariant
Is it explicit that the qualification filter (Phase-2 gate) must not go live before the labour engine exists (ADR-001)? A strategy that ships the filter early would surface goods-led contracts with no labour cost — actively misleading.

### 5. Schema-epoch + migration safety as foundation gates
Is the 9→10 bump (all 4 constants + Supabase + projection migration) a single foundation-phase task? Is the migration-nullability constraint a foundation gate (so `decision_basis_hash.rs` / `customs_fx_freight_engine.rs` direct INSERTs never break)?

### 6. Backfill decision
Research §10 pressure-test #2: historical rows have no labour component. Did the strategy capture the MVP decision (additive columns default NULL/0; no forced backfill; new assessments only) as an explicit phase note rather than leaving it ambiguous?

---

## Scope Drift Detection (MANDATORY — BLOCKING)

### Red-Flag Language Scan
Search the strategy for: "post-MVP," "Phase 2+," "stretch goal," "nice to have," "optional phase," "if time permits," "future release," "deferred," "out of scope" attached to **TIER-A** content (MVP + §2.4 "Phase 2 / Should Have"). BLOCKING if found. (A **TIER-B** research-spike phase legitimately carries "research / later" framing — operator decision 2026-06-02; that is correct, not drift.)

### Coverage Verification (two tiers — see `manifest.yaml` SCOPE TIERS)
Enumerate every spec requirement. Verify each TIER-A requirement is assigned to exactly one delivery phase, and each TIER-B item is assigned to a tracked late phase or a research-spike bead. Nothing silently dropped.

### Drift Checklist
- [ ] Every TIER-A spec requirement assigned to exactly one delivery phase
- [ ] Every TIER-B item (live enrichment lane, installer-quote ingestion, detailed per-zone permit) assigned to a tracked late phase OR research-spike bead (research/design gate, not code-test gate)
- [ ] No TIER-A phase labelled optional; later Tier-A phases have full gate criteria
- [ ] TIER-A phase coverage is EQUAL — late delivery phases are not thinner than early ones
- [ ] Recurring-maintenance (`MaintenancePeriodEstimate`) has a real Tier-A phase with real gates
- [ ] Access-equipment auto-selection, rate-refresh skill, overlay-labour-cost, basic banded permit/TM each have a Tier-A phase
- [ ] Gate criteria reference real TDD test IDs + AGENTS.md stage gate
- [ ] "Two halves ship together" dependency is explicit
- [ ] Schema-epoch bump + migration nullability are foundation gates
- [ ] No phase ordering puts a consumer before its dependency

---

## Follow-Up Pass (Run After Fixes)
> After making your fixes, send this as a follow-up message in the same session.

Re-read your fixes. Did re-ordering a phase break a different dependency? Re-trace the spine: schema/types → reference data + labour skill → rollup integration → qualification/Phase-2 gate → recurring maintenance → surfacing → overlay/gold-set/rate-refresh → Tier-B research spikes. Re-count beads-per-phase: after equalizing, does any **Tier-A** phase still look like a stub? If the gold-set or recurring-maintenance Tier-A phase is thin, it's hiding dropped scope — expand it. (A small Tier-B research-spike phase is fine — don't pad it into a full delivery phase.)
