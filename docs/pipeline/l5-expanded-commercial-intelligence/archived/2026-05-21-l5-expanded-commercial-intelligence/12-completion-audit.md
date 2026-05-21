# L5 Expanded Commercial Intelligence — Completion Audit

## First Action

Invoke the `/bead-completion-audit` skill.

---

## Your Mission

Deep-read the L5 spec, TDD, and codebase. Find EVERY gap between what the spec requires and what's actually implemented. Create fix beads (`audit-fix` label) for gaps. Do NOT execute them — just create and scope.

**CRITICAL: The L5 spec is the source of truth.** It went through three planmaxer cycles + harmonymaxer Run 002. Every requirement must be implemented in the codebase. If something is missing, it's a gap — not a "nice to have" that was intentionally skipped. Create a fix bead.

---

## Source Documents

```
docs/strategy/layers/layer-5-expanded-commercial-intelligence.md   — Spec (SOT #1) — walk EVERY L5-FR
docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-tdd.md  — TDD (SOT #2) — verify EVERY test exists with correct assertions
docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-bootstrap.md  — Bootstrap — verify env state matches expected
```

### Codebase (THE IMPLEMENTATION — read deeply)

```
packages/intelligence/src/customs/      — should contain 6 files (description-quality-gate, multi-source-recall, ten-digit-resolution, measure-scanner, exporter-resolution, classification-freshness)
packages/intelligence/src/fx/           — should contain 4 files (hmrc-exchange-rates, xe-commercial-rates, fx-engine, fx-types)
packages/intelligence/src/logistics/    — should contain 5 files (mode-selector, freightos-client, searates-client, freight-engine, freight-types)
packages/intelligence/src/validation/   — should contain 3 comparison adapters (dutify, zonos, avalara)
packages/intelligence/src/customs/accuracy.ts  — evidence-tiered accuracy harness

migrations/                             — confirm migrations/NNNN_l5_product_commodity_codes.sql created
crates/daemon/src/errors/manifest.rs    — confirm 10 new ErrorCode variants present at lines 20-55+ (enum + as_str() matcher)
crates/daemon/src/sources/registry.rs   — confirm 4-5 new RegisteredSource entries
crates/daemon/src/scheduler/cron.rs     — confirm 4 new CronJobDefinition entries
crates/daemon/src/db/sync.rs            — confirm PUSH_OWNED_RESEED_TABLES includes product_commodity_codes
crates/daemon/src/main.rs:5639-6350+    — confirm landed-cost paths refactored to delegate to packages/intelligence/src/customs/

skills/landed-cost/hmrc-customs-classification/            — confirm extended for multi-family expansion
skills/landed-cost/simple-fx-lookup/                       — confirm extended for dual-rate (Frankfurter + HMRC customs monthly)
skills/landed-cost/searates-freight-estimate/              — confirm extended for dual-source disagreement signaling
skills/landed-cost/freightos-freight-estimate/             — confirm operator-built prerequisite exists with working CDP browser GraphQL/scrape path
skills/landed-cost/taric-cross-check/                      — confirm operator-built prerequisite exists with working CDP browser scrape of taric.ec.europa.eu
skills/supplier-price-enrichment/{alibaba,amazon,1688}/   — confirm promotion_stage updated to stable; specific deltas applied per spec §1.15 + §2.3
skills/procurement-ingestion/pcs/                          — MUST exist (Scotland IN-scope per operator decision 2026-05-08; full skill directory standard with SKILL.md + result.schema.json + checkpoint.schema.json + canary.inputs.jsonl + golden.assertions.yaml)
skills/validation/overlay-customs-classification/         — confirm exists if recommended

tests/fixtures/calibration/customs-classification/        — confirm ≥3 categories with canary.inputs.jsonl + golden.assertions.yaml
crates/daemon/tests/                                       — confirm new L5 test files: customs_classification.rs, customs_measure_interaction.rs, customs_fx_engine.rs, customs_freight_engine.rs, customs_classification_accuracy.rs (or co-located TS variants)
```

### Quality Bar
```
docs/pipeline/l4-shared-visibility/artifacts/l4-completion-audit-2026-05-08.md   — L4's COMPLETE verdict; reference for thoroughness
docs/pipeline/l4-shared-visibility/archived/2026-05-08-l4-shared-visibility/12-completion-audit.md  — L4's audit handoff
```

---

## Audit Methodology

Walk the spec requirement by requirement. For each L5-FR-X.Y.Z:

1. **Find the corresponding implementation** in the codebase. Use grep, file inspection, type signatures.
2. **Verify it matches** the spec's acceptance criteria. Check field names, default values, error code emission, conservative-default behavior, freshness window logic.
3. **Find the corresponding TDD test(s).** Verify they exist at the path the TDD specifies, with the assertions the TDD specifies.
4. **Verify tests pass.** Run the verification commands from the bootstrap doc:
   ```bash
   cargo fmt --check && cargo check --all-targets && cargo clippy --all-targets -- -D warnings
   cargo test -p daemon --bin daemon
   for t in crates/daemon/tests/*.rs; do cargo test --test "$(basename "$t" .rs)" || exit 1; done
   bun run build && bun run lint && bun run test
   ```
5. **If gap found** → create a fix bead under label `audit-fix`. Bead must include: spec ID, test ID, exact gap description, target files, acceptance criteria.

**Do NOT accept "this was intentionally deferred" as an excuse.** The spec is the contract. If it's in the spec and not in the code, it's a gap. Create a fix bead.

---

## L5-Specific Audit Dimensions

### 1. Schema Population Verification

For every column on `landed_cost_estimates` (per `migrations/0001_sqlite_state_initial.sql:322-394`):

- [ ] Find the L5 code path that populates this column.
- [ ] Run a DB query against a test fixture: `SELECT <column> FROM landed_cost_estimates WHERE id = '<fixture_id>'`. Confirm non-null where the spec says it should be populated, conservative default where the spec says default.
- [ ] If a column is still NULL in fixtures where it should be populated → fix bead.

Critical columns:
- `candidate_commodity_codes_json` — multi-source recall results.
- `additional_codes` / `additional_code_requirements_json` — additional-code requirement state.
- `supplementary_units_required_json` — supplementary-unit handling.
- `customs_valuation_method` / `valuation_mode` — valuation classification.
- `customs_fx_rate` / `commercial_fx_rate` / `fx_basis_delta_bps` — dual FX.
- `duty_breakdown_json` / `duty_outcome_min_gbp_pence` / `duty_outcome_max_gbp_pence` / `customs_cost_spread_bps` — duty resolution.
- `trade_remedy_state` / `trade_remedy_cases_json` / `measure_interaction_json` — measure scanner output.
- `exporter_rate_basis` / `exporter_identity_confidence_bps` — exporter resolution.
- `freight_low/base/high_gbp_pence` / `shipping_mode` / `mode_confidence_bps` — freight engine.
- `vat_rate_state` / `vat_recovery_rate_bps` — VAT.
- `tariff_checked_at` / `measures_fresh_until` / `classification_fresh_until` / `trade_remedy_watch_state` / `fx_rate_checked_at` — classification freshness.
- `decision_grade` — final grading.
- `confidence_breakdown` — must include `unlockable_margin_bps` and `evidence_leverage` keys.
- `cbam_exposure_flag` / `cbam_estimate_gbp_pence` — CBAM placeholder (flag may be true; estimate stays null until regulation).

### 2. ErrorCode Emission Verification

For each of the 10 new ErrorCode variants:

- [ ] Find the variant in `crates/daemon/src/errors/manifest.rs` (the enum lives at lines 20-55+ alongside its `as_str()` matcher in the same file — NOT `validation/policy.rs`, which holds `ValidationPolicyMode` for L3 validation routing).
- [ ] Find the code path that emits it (likely in `packages/intelligence/src/customs/*.ts` or `fx/*.ts` or `logistics/*.ts`).
- [ ] Find the test that verifies emission.
- [ ] Run the emission test; confirm pass.

If any variant is added but never emitted → fix bead.
If any variant emits in a wrong condition → fix bead.
If any variant is missing from the `as_str()` matcher → fix bead (clippy should have surfaced this during Phase 1).

### 3. Conservative Defaults Verification

For each of the 6 default rows in spec §1.12:

- [ ] Construct a test fixture with the resolution incomplete in the way that triggers this default.
- [ ] Run the customs engine.
- [ ] Verify the output matches the conservative default.

If defaults are bypassed, that's a critical fix bead — silent understatement of duty exposure.

### 4. Multi-Source Recall Independence Verification

- [ ] Construct a fixture where FPO returns one HS family but no other source agrees.
- [ ] Verify the engine does NOT count FPO alone as evidence.
- [ ] Construct a fixture where FPO + Taric Support + HS Ping all return the same plausible-but-wrong family.
- [ ] Verify slot-level compatibility checking catches the wrong family.

If FPO-alone counts as evidence → critical fix bead.

### 5. Multi-Family Expansion Verification

- [ ] Construct a fixture with ≥3 plausible HS6 families for a single product.
- [ ] Verify ALL families are descended to UK10 (not greedy descent on the top candidate).
- [ ] Verify shortlist contains top 3-5 non-dominated branches per family.
- [ ] Verify external HMRC/Taric/HS-Ping validation runs ONLY on shortlisted candidates.

If greedy descent observed → critical fix bead.

### 6. Measure Interaction Golden Cases

- [ ] Steel AD/CVD/safeguard scenario: when quota exceeded, only safeguard OR (AD - safeguard) charged. Verify against spec §1.7.
- [ ] Aluminium named-exporter scenario: residual rate (35.1%) applied by default; named-exporter rate available with proof.
- [ ] Compound duty preservation: GBP X/kg specific component is NOT flattened to ad valorem.
- [ ] Supplementary unit missing: `SUPPLEMENTARY_UNIT_MISSING` emitted; abstain on that component (NOT averaged or estimated).

### 7. Marketplace Skill Maturation Verification

For each of alibaba/amazon/1688:
- [ ] `promotion_stage` in SKILL.md is `stable` (not `candidate`).
- [ ] Specific deltas applied:
  - Alibaba: Chinese retry on zero/accessory hits; Ready-to-Ship filter; source-health canary expansion.
  - 1688: Chinese-only short clusters; domestic wholesale floor in result.
  - Amazon: alias families; structured title vocabulary; browser-first.
- [ ] Canary harness passes for all 4 (alibaba + amazon + 1688 + pcs — Scotland IN-scope).
- [ ] Validator overlay binding (if added) calibrated.

### 8. Gold-Set Coverage Verification

- [ ] ≥3 categories present at `tests/fixtures/calibration/customs-classification/{category}/`.
- [ ] Each has `canary.inputs.jsonl` + `golden.assertions.yaml`.
- [ ] Each has corpus per L4 convention: clean / weird-but-valid / mutated bad / pairwise.
- [ ] Evidence-tier labels (gold/silver/bronze) applied.
- [ ] Calibration harness passes Wilson-bound precision gates per slice.

### 9. CBAM Placeholder Verification

- [ ] `cbam_exposure_flag` emits true for tender delivery windows crossing 2027-01-01 affecting in-scope commodity families (aluminium, cement, fertiliser, hydrogen, iron and steel).
- [ ] `cbam_exposure_flag` emits false for non-crossing windows.
- [ ] `cbam_exposure_flag` emits false for out-of-scope commodity families (glass, ceramics).
- [ ] `cbam_estimate_gbp_pence` remains NULL.

### 10. L5→L6 Readiness Gate Verification

- [ ] `CLASSIFICATION_AMBIGUOUS` produced when sources diverge.
- [ ] `TARIFF_BRANCH_DIVERGENCE` produced when UK10 branches differ on remedy/duty.
- [ ] `SUPPLEMENTARY_UNIT_MISSING` produced when required supp unit absent.
- [ ] `decision_grade = broker_required` set under appropriate conditions.
- [ ] Classification freshness expiry detected and stale records flagged.

If any gate condition is unproduceable → critical fix bead (L6 implementation will be blocked).

### 11. Sync Engine Hookup

- [ ] L4 sync.rs PUSH_OWNED_RESEED_TABLES includes `product_commodity_codes` (the new L5 table).
- [ ] L5 mutations to `landed_cost_estimates` flow to Supabase via `local_outbox`.
- [ ] Viewer-role projection respects RLS for new sensitive fields.

### 12. Stage 11 Live Demo

- [ ] A real Contracts Finder lot has been processed end-to-end through the new decision-grade landed-cost path.
- [ ] Output includes: full duty breakdown, dual FX, freight estimates, evidence_leverage, classification freshness, CBAM exposure (if applicable).
- [ ] Manifest renders correctly in desktop + web portal.

### 13. Definition of Done Walkthrough

Walk all 12 (or 13 if no-CI criterion was added) DoD criteria from spec §3.1. Each MUST be green:

| # | DoD Criterion | Status |
|---|---------------|--------|
| 1 | Alibaba/1688 search skills operational with Chinese keyword clusters | ☐ |
| 2 | Amazon search skill operational | ☐ |
| 3 | PCS (Scotland) ingestion skill operational | ☐ (REQUIRED — Scotland IN-scope per operator decision 2026-05-08) |
| 4 | HMRC FPO API integration for HS code classification candidates | ☐ |
| 5 | Multi-source recall produces candidates from ≥2 independent sources | ☐ |
| 6 | 10-digit resolution via HMRC Trade Tariff API produces UK-specific commodity codes | ☐ |
| 7 | Measure scanner detects AD/CVD/safeguard/quota measures and models interactions | ☐ |
| 8 | HMRC Exchange Rate Service provides customs valuation FX (monthly rolling) | ☐ |
| 9 | Commercial spot FX provided (Xe replaced by Frankfurter — free, ECB-backed — per operator decision 2026-05-08; verify equivalent functional outcome on `commercial_fx_rate` field) | ☐ |
| 10 | Freightos + SeaRates provide freight market data (via existing SeaRates CDP skill + operator-built Freightos CDP skill — both NO paid API per operator decision 2026-05-08) | ☐ |
| 11 | Decision-grade landed cost per validated category (with full audit trail) | ☐ |
| 12 | Category-specific gold-set expansion completed for ≥3 categories | ☐ |

---

## Output

A completion-audit report at `docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-completion-audit-{YYYY-MM-DD}.md` with:

- **Verdict:** COMPLETE (no gaps) | INCOMPLETE (N gaps; M fix beads created)
- **Per-dimension findings:** for each of the 13 L5-specific audit dimensions, status + gaps + fix-bead IDs.
- **Fix beads:** list of `audit-fix`-labeled beads created, with target ACs.
- **Test gate state:** snapshot of cargo + bun test output.

If verdict is INCOMPLETE → proceed to step 13 (fix-beads kickoff).
If verdict is COMPLETE → proceed to step 15 (archive).

---

## Follow-Up Pass (Sub-Optimization)

> After creating all fix beads:

Re-read the fix beads you created. For each:

1. Is it specific enough? "Fix multi-source recall" is too vague; "Add FPO-vs-Taric-Support disagreement test at `crates/daemon/tests/customs_classification.rs::test_fpo_alone_not_evidence` per spec §1.4" is right.
2. Does it reference the right spec section and TDD test ID?
3. Could a fresh agent implement the fix from the bead description alone?
4. Is the AC concrete — not "make this work" but "set `exporter_rate_basis` to `residual_default` when origin proof is absent, verified by `L5-T-customs_classification-014`"?
5. Did you cover EVERY spec L5-FR-X.Y.Z, or did you stop early?

Run a final count: spec requirement count vs. (passing tests + fix beads) — must equal 100%.

If verdict is INCOMPLETE: `bin/br-sync && git add .beads/ docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-completion-audit-*.md && git commit && git push`.

---

## Key Reminders

- The audit is the GATE that catches everything the bead-plan-audit (step 09) missed.
- Spec is the contract. No "intentionally deferred" excuses.
- Fix beads are surgical — one fix per bead.
- ZERO SCOPE DRIFT.
