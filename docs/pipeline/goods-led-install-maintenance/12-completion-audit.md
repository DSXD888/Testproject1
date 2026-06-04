# Goods-Led Install/Maintenance — Completion Audit

## First Action

Invoke the `/bead-completion-audit` skill.

---

## Your Mission

Deep-read the spec, TDD, and codebase. Find EVERY gap between what the spec requires and what's actually implemented. Create fix beads for gaps. Do NOT execute them — just create and scope.

**CRITICAL: The spec is the source of truth. The research doc was deeply iterated on before this pipeline. EVERY feature in the spec must be implemented. If something is missing, it's a gap — not a "nice to have" that was intentionally skipped. Create a fix bead.**

---

## Source Documents

```
docs/pipeline/goods-led-install-maintenance/artifacts/goods-led-install-maintenance-spec.md   — SOT #1 — walk EVERY requirement
docs/pipeline/goods-led-install-maintenance/artifacts/goods-led-install-maintenance-tdd.md      — SOT #2 — verify EVERY test exists with correct assertions
docs/pipeline/goods-led-install-maintenance/artifacts/goods-led-install-maintenance-bootstrap.md — verify environment matches
docs/pipeline/goods-led-install-maintenance/manifest.yaml                                        — feature_surface checklist
```

### Codebase (THE IMPLEMENTATION — read deeply)
```
packages/intelligence/src/qualification/goods-qualification.ts        — Phase-1 branch present? separable-mixed downgraded? no-list/bespoke flag?
packages/intelligence/src/qualification/goods-qualification.test.ts   — TS-QUAL-003a/b split? TS-QUAL-006/007/008?
packages/intelligence/src/customs/description-quality.ts              — goods_led_mixed variant added to the union?
packages/intelligence/src/customs/description-quality.test.ts         — NEW file exists with the 8-row truth table?
packages/intelligence/src/margins/coverage-rollup.ts + .test.ts       — goods_value_share_bps + TS-COV-007/008/009?
crates/daemon/src/main.rs                                             — labour summed at BOTH 8114 + 8548? labour skill loaded in run_landed_cost_live_loop? Phase-2 gate at synthesis? goods-only subtotal persisted?
crates/daemon/tests/installation_labour_engine.rs                    — NEW target with all 11 Appendix-E manifests + Scenario-1 NW band?
skills/landed-cost/uk-installation-labour-estimate/                  — SKILL.md (http_public) + result.schema.json + 4 reference JSONs?
skills/validation/overlay-labour-cost/                              — overlay present (headless_contract + checkpoint.schema.json)?
tests/fixtures/calibration/landed-cost/uk-installation-labour/      — 7 canary cases + metadata.json row_count=7 + validator-rubric.yaml?
packages/contracts-schema/src/index.ts                             — labour/permit fields + goods_value_share_bps + InstallationRequirement?
docs/strategy/layers/layer-0-gold-set-evaluation.md                — broadened categories + goods-led-mixed note?
apps/desktop/src-tauri/src/{main.rs,daemon_bridge.rs,query_service.rs} + crates/daemon/src/db/sqlite.rs — schema-epoch bumped to 10 across ALL 4?
packages/ui/{manifest-viewer.tsx,contract-card-frame.tsx}          — labour/permit breakdown + labourCoverageBps rows?
```

---

## Audit Methodology

Walk the spec requirement by requirement. For each:
1. Find the corresponding implementation in the codebase.
2. Verify it matches the spec's acceptance criteria.
3. Find the corresponding TDD test(s).
4. Verify tests exist with correct assertions (pence-exact where the TDD specifies).
5. If a gap is found → create a fix bead (label `goods-led-install` + `audit-fix`).

**Do NOT accept "this was intentionally deferred" as an excuse for a TIER-A gap.** If a Tier-A feature (MVP + §2.4 "Phase 2 / Should Have": recurring maintenance, access-equipment auto-selection, rate-refresh skill, overlay, basic banded permit/TM) is in the spec and not in the codebase, it's a gap → fix bead. **TIER-B items** (the three §2.4 "Future" / §10 "requiring further research" items: live enrichment lane, installer-quote ingestion, detailed per-authority/per-zone permit sub-model) were operator-scoped (2026-06-02) as research spikes, NOT full delivery — so for these, the audit confirms the `research-spike` bead exists and its research/design output (or extension point) is present; do NOT raise a "missing implementation" fix bead for a Tier-B item that was never meant to ship full behaviour in this pipeline. See `manifest.yaml` SCOPE TIERS.

### Feature-Specific Gap Hunts (high-risk areas — research §6.6)
- **Service gate (highest risk):** does `description-quality.test.ts` exist and pass all 8 truth-table rows? Does "Supply of maintenance kits" → `accept` and "Installation services" → `reject_service`? A silent over-admission here is the #1 failure mode.
- **Both rollup sites:** run the AGENTS.md stage gate. Does a Rust `#[test]` prove labour is summed at BOTH `build_domestic_landed_cost_estimate` and `build_import_landed_cost_estimate`?
- **Scenario 8 non-regression:** is `economic_landed_cost_gbp_pence` unchanged for pure paper (labour=0, no `InstallationRequirement`)?
- **Goods-only subtotal reconstructable:** can `goods_value_share_bps` be recomputed from a single persisted row?
- **Scenario-1 band:** does the engine produce labour in 210000–265000 pence for the NW festive-lights case?
- **Schema epoch:** do desktop reads succeed (no `{code,message}` envelopes)? Are all 4 constants at 10? (`customs_fx_freight_engine.rs` asserts `user_version == DEFAULT_SCHEMA_EPOCH`.)
- **Recurring maintenance:** is `MaintenancePeriodEstimate` a distinct entity, NOT capitalised into the dominance denominator?
- **All 8 warning codes** emitted by the canary corpus?

---

## Follow-Up Pass
> After creating all fix beads:

Re-read the fix beads you created. Are they specific enough — does each reference the right spec section + TDD test IDs? Could an agent implement each from the bead alone? Is each AC concrete ("add field X of type Y to struct Z at main.rs:NNNN"), not "fix this"? Did you walk EVERY spec requirement, or stop early? Run a final count: spec requirements vs (fix beads + already-complete beads) = 100%? Run the full AGENTS.md stage gate one more time and record the result in the audit report at `docs/pipeline/goods-led-install-maintenance/artifacts/goods-led-install-maintenance-completion-audit-YYYY-MM-DD.md`.

⛔ **MANDATORY STOP** — Human authorizes fix-bead execution (step 13) before any fix bead is claimed.
