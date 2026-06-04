# Goods-Led Install/Maintenance — Optimize TDD (Fresh-Eyes Audit)

## Your Mission

You are performing a **comprehensive fresh-eyes audit** of the TDD — SOT #2. The TDD is the acceptance contract the implementation is written against (test-first). For a feature that changes what the platform bids on, the test suite is the safety net: a weak negative-control corpus lets service-only contracts leak through (silent failure), and a weak Scenario-1 band assert lets the labour engine ship wrong.

**You are reading, analyzing, and improving an existing document — not writing code.**

---

## What To Read (Do This First)

### The Document to Audit (TARGET)
```
docs/pipeline/goods-led-install-maintenance/artifacts/goods-led-install-maintenance-tdd.md         — created in step 03
docs/pipeline/goods-led-install-maintenance/artifacts/goods-led-install-maintenance-bootstrap.md    — companion
```

### Verify Against (SOURCE OF TRUTH)
```
docs/pipeline/goods-led-install-maintenance/artifacts/goods-led-install-maintenance-spec.md   — SOT #1 — every Tier-A requirement needs a test (Tier-B items are research-spikes)
docs/research/2026-06-01-goods-led-install-maintenance-pipeline-research.md                   — §6.4 + Appendices A/E/F (the authored test strategy)
```

### Codebase (GROUND TRUTH — confirm the tests are runnable as written)
```
crates/daemon/tests/customs_fx_freight_engine.rs   — does the planned installation_labour_engine.rs mirror this exactly?
crates/daemon/tests/decision_basis_hash.rs          — direct-INSERT pattern + hash mutation
crates/daemon/tests/skill_canary_harness.rs         — required fixture files + row_count assertion
crates/daemon/tests/skill_lint.rs                   — frontmatter contract for the labour skill
packages/intelligence/src/qualification/goods-qualification.test.ts   — the real TS-QUAL-003 to split
packages/intelligence/src/customs/customs-classification.test.ts      — the bun:test style for the NEW description-quality.test.ts
```

### Quality Bar (MATCH OR EXCEED)
```
docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-tdd.md
```

---

## Think Independently

### 1. Is the coverage matrix actually complete?
Walk every spec requirement ID. Is each **Tier-A** requirement in the matrix with ≥1 test? Pay attention to the easy-to-forget Tier-A ones: recurring maintenance (`MaintenancePeriodEstimate`), access-equipment auto-selection, quarterly rate-refresh skill, `overlay-labour-cost`, basic banded permit/TM. Are the three **Tier-B** items (live enrichment lane, installer-quote ingestion, detailed per-authority/per-zone permit sub-model) present in the matrix marked "research-spike, no code test" — not silently dropped, but NOT given invented tests either (operator decision 2026-06-02; see `manifest.yaml` SCOPE TIERS)?

### 2. Are the negative controls strong enough?
Scenarios 5/6/7 MUST reject. Is each pinned by BOTH a `description-quality.test.ts` row AND an `installation_labour_engine.rs` manifest? Scenario 6 (managed cleaning + trivial consumables) is the one that exercises the value-share floor doing real work — does the manifest seed labour ≫ goods and assert `share < GOODS_DOMINANCE_FLOOR_BPS`?

### 3. Is the regression control airtight?
Scenario 8 (pure A4 paper) must `pass` exactly as today: no `InstallationRequirement`, labour=0, `economic_landed_cost_gbp_pence` unchanged. Does the TDD assert the labour columns are 0/NULL and the rollup total equals the goods-only subtotal? This is the guard for audit finding #4.

### 4. Does the TS-QUAL-003 split avoid silently retaining the old expectation?
Verify 003a (≥540_000 → `pass`) and 003b (360_000 → `review`) are both present and that the old `pass`-at-40% expectation is gone.

### 5. Is the Scenario-1 hard assert pence-exact?
`210000 ≤ installation_labour_cost_gbp_pence ≤ 265000` (NW band). Is it in both the `installation_labour_engine.rs` manifest AND the `lights-nw-2day-01` canary golden assertion?

### 6. Do the canary fixtures satisfy the harness contract?
`metadata.json` `row_count` must equal the JSONL line count (7 cases in Appendix F.1). `validator-rubric.yaml` must carry `rubric_version`. `manifest.md` must mention each `Scenario`. Are all three pinned in the TDD?

### 7. Are all 8 warning codes pinned?
`LABOUR_REGION_UNKNOWN`, `LABOUR_SCOPE_UNRESOLVED`, `ACCESS_AT_HEIGHT_ASSUMED_MEWP`, `MAINTENANCE_TERM_ASSUMED`, `RATE_TABLE_STALE`, `PERMIT_RESPONSIBILITY_UNKNOWN`, `LANE_RENTAL_ZONE`, `TTRO_LEAD_TIME_RISK` — each mapped to a canary case (Appendix F.1)?

### 8. Is the bootstrap doc executable?
Does it list the full AGENTS.md stage-gate command sequence verbatim, the schema-epoch 4-constant bump checklist, and the skill-local reference-JSON seeding step (the 4 files must ship before the skill can run)?

---

## Scope Drift Detection (MANDATORY — BLOCKING)

### Red-Flag Language Scan
Search the TDD for: "optional test," "skip for now," "deferred coverage," "post-MVP," "stretch," "nice to have," "future iteration," "out of scope." Any test-coverage gap dressed as a deferral is a BLOCKING failure.

### Coverage Verification
Enumerate every spec requirement. Verify each **Tier-A** requirement has a named test (the §2.4 "Phase 2 / Should Have" items — recurring maintenance, access-equipment auto-selection, rate-refresh skill, overlay, basic banded permit/TM — need tests just like MVP). The three **Tier-B** items appear in the matrix as research-spikes (no code test) — present, not silently dropped, not given invented tests.

### Drift Checklist
- [ ] Every spec requirement → ≥1 named test
- [ ] All 11 Appendix-A scenarios → tests; all 11 Appendix-E manifests → `installation_labour_engine.rs` cases
- [ ] All 8 §5.6 warning codes pinned
- [ ] TS-QUAL-003a/b split; old expectation removed
- [ ] Negative controls (5/6/7) double-pinned; regression control (8) asserts labour=0 + unchanged total
- [ ] Scenario-1 band hard-assert in manifest AND canary
- [ ] TIER A: recurring-maintenance, access-equipment, rate-refresh, overlay, basic banded permit/TM — all have named tests
- [ ] TIER B: live-enrichment, installer-quote, detailed per-zone permit — present in matrix as "research-spike, no code test" (not dropped, not over-tested)
- [ ] No coverage labelled optional/deferred
- [ ] CI-tier table maps each test to a real command

---

## Follow-Up Pass (Run After Fixes)
> After making your fixes, send this as a follow-up message in the same session.

Re-read your fixes. Did adding a test create an inconsistency in the coverage matrix or duplicate a test ID? Open `crates/daemon/tests/skill_canary_harness.rs` and confirm your final canary count and the `metadata.json` `row_count` still match. Re-run the §2.3 criterion → test mapping one final time: is every criterion still pinned, and did any edit orphan a requirement?
