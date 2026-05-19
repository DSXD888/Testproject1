# L5 Expanded Commercial Intelligence — Create TDD + Bootstrap

## First Action

Invoke the `/spec-to-tdd` skill.

---

## Your Mission

Generate the **TDD** (test specification covering all L5-FR requirements) and the **bootstrap doc** (environment prerequisites, validation script, external integration setup) for Layer 5.

The TDD is SOT #2: every L5 requirement must have at least one mapped test ID. The bootstrap doc is the environmental contract: a fresh agent should be able to follow it and reach a working L5 dev environment. **Per operator decision 2026-05-08 (manifest `recorded_decisions.external_api_account_state`), L5 is skill-first with ZERO paid commercial API procurements** — the bootstrap covers public APIs (HMRC Trade Tariff, HMRC FX monthly publication, Frankfurter), existing CDP browser skills (SeaRates), and operator-built CDP browser skills (Freightos, EU TARIC). No signup URLs, no key creation steps, no procurement lead times.

---

## Source Documents (Read In This Order)

### Primary Input (CREATE FROM THIS — READ FULLY)
```
docs/strategy/layers/layer-5-expanded-commercial-intelligence.md   — validated spec from steps 01-02 (target ~520-700 lines after IDs/ACs added)
```

### Source of Truth Context (REFERENCE)
```
docs/strategy/layer-strategy.md                                    — §10 Interface Contracts (L5↔L6 readiness gate)
docs/strategy/cross-reference-index.md                             — L5 cross-references (must stay consistent)
```

### Quality Bar (MATCH OR EXCEED)
```
docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-tdd.md         — 623 lines; 158 checks; 8 static gates; 4 test files; 4-tier CI tiering — THE BAR
docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-bootstrap.md   — 21 environment prerequisites; Supabase local stack; calibration corpus; THE BAR
docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-tdd.md — 557 lines; secondary reference
```

### Codebase Context (STUDY FOR PATTERNS)
```
crates/daemon/tests/skill_golden_journeys.rs                       — 607 lines; orchestrates skill execution + golden assertions
crates/daemon/tests/skill_canary_harness.rs                        — 255 lines; smoke-level coverage runner
crates/daemon/tests/validation_golden_verdicts.rs                  — validation calibration test pattern
crates/daemon/tests/sqlite_supabase_sync.rs                        — L4 sync integration tests
crates/daemon/tests/agent_supplier_enrichment.rs                   — supplier enrichment integration test pattern
tests/fixtures/calibration/                                        — JSONL canary.inputs + YAML golden.assertions per (domain, variant)
packages/intelligence/src/margins/coverage-rollup.test.ts          — 334 lines; vitest pattern in TS
packages/intelligence/src/qualification/goods-qualification.test.ts — 103 lines; vitest pattern in TS
skills/supplier-price-enrichment/alibaba/SKILL.md                   — 241 lines; most developed marketplace skill (informs canary inputs)
```

### Existing Test Inventory (FOR CUMULATIVE TARGETING)
```
crates/daemon/tests/                                               — 58 .rs files today
packages/intelligence/src/                                         — 8 .test.ts files today
apps/desktop/src/                                                  — 3 .test.ts files today
```

L5's spec §3.2 says cumulative test target is **57 tests** after L5 lands. Verify against current count to size the L5 contribution accurately.

---

## Cardinal Rule: ZERO SCOPE DRIFT

**If it's in the validated spec, it gets test coverage. No exceptions.**

Every `L5-FR-X.Y.Z` requirement gets at least one test ID. The test IDs follow the pattern `L5-T-{file-stem}-{NNN}` (e.g., `L5-T-customs_classification-001`). The TDD's coverage matrix maps every requirement ID to one or more test IDs.

You MAY:
- Group multiple closely-related requirements under a single test (with explicit AC table inside)
- Choose between unit / integration / E2E tier — but every requirement is covered SOMEWHERE
- Reuse calibration fixtures across tests when the input is the same

You MAY NOT:
- Defer a requirement to "post-L5", "Phase 2+", or "later iteration"
- Drop a test scenario silently because it's hard to construct
- Reduce assertion specificity below what the spec's AC defines
- Use mocks for HMRC/Xe/Freightos/SeaRates — the project convention is fixture cassettes (recorded responses) + real HTTP integration tests at the integration tier

---

## Key Context From Codebase Research

1. **No HTTP mocking framework is used.** Tests use fixture cassettes (recorded JSON responses) for unit-level tests and real HTTP calls (against staging/sandbox endpoints when available) for integration tests. L5 follows this convention.

2. **Existing landed-cost integration tests** are partial — `crates/daemon/tests/agent_supplier_enrichment.rs` exercises supplier enrichment, but there is no `landed_cost_*.rs` integration test today. L5 must create them from scratch.

3. **Calibration fixtures live at `tests/fixtures/calibration/`** with `canary.inputs.jsonl` + `golden.assertions.yaml` per (domain, variant). L5 extends this to cover ≥3 goods categories per DoD criterion 12. The operator pre-resolved the trio on 2026-05-08 (manifest `recorded_decisions.decision_4_gold_set_categories`): **fasteners + packaging-cartons + examination-gloves** (clear differentiation: remedy-active / packaging-easy / regulated medical). The TDD names these three categories directly — no "propose for owner decision" wording.

4. **`landed_cost_estimates` ALREADY in L4 sync push set** (`crates/daemon/src/db/sync.rs:35`). L5 mutations auto-flow to Supabase. TDD must include redaction-boundary tests (no decision-grade fields leak to viewer-role projection beyond what RLS policies allow — verify against `infra/supabase/migrations/0004_rls_policies.sql`).

5. **Existing 4-tier CI tiering pattern from L4 TDD §4** (T1<3min unit/static, T2<10min integration light, T3<25min integration full, T4<45min E2E). L5 follows this convention. New L5 tests are placed by tier based on cost/cadence, not by feature area.

6. **Skill canary harness pattern** (`skill_canary_harness.rs:255 lines`) — every L5 marketplace skill (alibaba/amazon/1688/[pcs]) gets a canary input set + golden assertions. The TDD specifies the canary input shape and assertion patterns.

7. **No `apps/web/` E2E additions in L5** — L4 already shipped the web portal. L5 adds backend logic that flows through the existing portal. If a viewer surface should display new fields (e.g., decision_grade, evidence_leverage), that's a UI follow-up tracked in step 05 strategy as a Phase 5 polish item, not a separate TDD test file.

8. **Reqwest already present in daemon** (added in L4 for Supabase). L5 reuses it for HMRC/Xe/Freightos/SeaRates clients — no new HTTP crate needed.

{ENRICHMENT: spec metadata — after step 02 enrichment, this section will name the exact L5-FR-X.Y.Z range, total requirement count, and per-subsection counts.}

---

## Step-by-Step Workflow

### Step 0: Read Source Documents (MANDATORY — before doing anything)

1. `docs/strategy/layers/layer-5-expanded-commercial-intelligence.md` — **READ FULLY**.
2. `docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-tdd.md` — **STUDY** (target this rigor).
3. `docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-bootstrap.md` — **STUDY** (target this completeness).
4. `crates/daemon/tests/skill_golden_journeys.rs` (607 lines) — **STUDY FOR PATTERNS**.
5. `crates/daemon/tests/sqlite_supabase_sync.rs` — **SKIM** (pattern for L4 sync integration tests).
6. `tests/fixtures/calibration/` — **EXPLORE** (existing canary + golden formats).
7. `skills/supplier-price-enrichment/alibaba/SKILL.md` — **READ** (informs canary inputs for marketplace skills).

### Step 1: Invoke `/spec-to-tdd`

Inputs:
- Source spec: `docs/strategy/layers/layer-5-expanded-commercial-intelligence.md`
- Output TDD: `docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-tdd.md`
- Output bootstrap: `docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-bootstrap.md`

Key context to provide the skill:
- L5 spec uses `L5-FR-X.Y.Z` requirement IDs.
- 5 new test files planned: `hmrc-fpo.test.ts`, `customs-classification.test.ts`, `measure-interaction.test.ts`, `fx-engine.test.ts`, `freight-engine.test.ts` (per spec §3.2). Cumulative target 57 tests.
- Test ID convention: `L5-T-{file-stem}-{NNN}` (e.g., `L5-T-customs_classification-007`).
- Test framework: vitest for TS (`packages/intelligence/`), cargo test for Rust (`crates/daemon/tests/`).
- No HTTP mocking framework. Use fixture cassettes + real-HTTP integration tier.
- Calibration corpus convention: `tests/fixtures/calibration/{domain}/{variant}/canary.inputs.jsonl` + `golden.assertions.yaml`.
- Quality bar: L4 TDD's 4-tier CI tiering (T1 unit <3min, T2 integration light <10min, T3 integration full <25min, T4 E2E <45min).
- **Skill-first architecture (operator-confirmed 2026-05-08, manifest `recorded_decisions.external_api_account_state`):** L5 has ZERO paid commercial API procurements. Three existing landed-cost skills already cover HMRC Trade Tariff (public API), commercial spot FX (Frankfurter), and SeaRates (CDP browser). Two new skills (Freightos CDP + Taric cross-check CDP) are operator-built prerequisites, NOT pipeline beads. TDD tests these existing/prerequisite skills as integration points; does NOT test against paid APIs.
- **HMRC FPO investigation bead:** TDD includes an investigation bead `l5-customs-hmrc-fpo-endpoint-discovery` for Step 03 to determine: (a) does Trade Tariff `/api/search` / `/api/search_suggestions` suffice for ML-powered HS suggestion? (b) does a public FPO endpoint exist? (c) only if both fail, defer to operator decision on developer portal registration. NOT a procurement item — a discovery item.

### Step 2: Feature-Specific Guidance (while skill runs)

Pay attention to:

- **Multi-source recall agreement testing.** The 5 recall sources (HMRC FPO, CPV-to-chapter priors, rule-based hints, tariff keyword search, ATR codebook reuse) interact non-trivially. Test scenarios must cover: 5 sources agree, 5 sources disagree, FPO-only "evidence" (must NOT count as evidence per spec §1.4 critical rule), Taric Support disagreeing with HMRC on UK national measure (Taric must NOT overrule per spec §1.4 UK-specificity rule). Each scenario gets a stable test ID.
- **Multi-family expansion correctness.** §1.5 explicit: "never greedy descent". Test must construct a case where the top-ranked HS family is wrong but a runner-up is right; assert ALL plausible families are descended. The `candidateRecallAgreement` field on `landed_cost_estimates` is the trace artifact — assert against it.
- **Measure interaction golden cases.** §1.7 specifies steel as a substitutive case (greater of safeguard or AD, not both); aluminium extrusions as named-exporter rate variation. The TDD must enumerate at least these two golden cases by name with exact expected duty values.
- **Conservative defaults under partial resolution.** Six rows in §1.12. Each becomes a parametric test — given an incomplete input vector, assert the engine outputs the conservative default and the `unlockable_margin_bps` / `evidence_leverage` fields are populated.
- **FX monthly rollover.** When HMRC publishes new monthly rates, all affected `landed_cost_estimates` need recalculation. Test must construct a snapshot that crosses the rollover boundary and assert the recalc job is enqueued. Mention `fx_basis_delta_bps` storage assertion.
- **Freight dual-source cross-check.** Freightos and SeaRates must BOTH be queried; large discrepancies emit `FREIGHT_MODE_LOW_CONFIDENCE`. Test must construct a synthetic case where the two providers disagree and assert the warning emission + the conservative band selected.
- **Classification freshness.** Four vectors (structure, measures, remedies, FX). Tests must verify each vector's freshness window independently — a stale measure does NOT invalidate a fresh structure check.
- **Evidence-tiered accuracy harness.** §3.4. Tests must cover: gold-tier label, silver-tier label, bronze-tier label, abstention as positive class. The evaluation harness is the test driver — define it as a top-level test target (likely `crates/daemon/tests/customs_classification_accuracy.rs` or `packages/intelligence/test/customs/accuracy.test.ts`).
- **Marketplace skill canaries.** Each of alibaba/amazon/1688/[pcs] gets a canary input set with at least 5 representative inputs and golden assertions covering: result schema validation, anti-bot detection, Chinese retry trigger (alibaba), Chinese-only enforcement (1688), alias family expansion (amazon).
- **CBAM forward placeholder.** Test for tender delivery window crossing 2027-01-01 — assert the future-regulatory-exposure flag emission for affected commodity families. Test for window not crossing — assert no flag.

### Step 3: Verify Output

After `/spec-to-tdd` completes:
- [ ] `docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-tdd.md` exists.
- [ ] TDD line count is in the range 600-900 (L4 was 623; L5 surface is similar in breadth, deeper in customs).
- [ ] Coverage matrix maps every `L5-FR-X.Y.Z` to ≥1 test ID. Use grep:
      ```bash
      rg -o 'L5-FR-[0-9]+\.[0-9]+\.[0-9]+' docs/strategy/layers/layer-5-expanded-commercial-intelligence.md | sort -u | wc -l   # Spec count
      rg -o 'L5-FR-[0-9]+\.[0-9]+\.[0-9]+' docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-tdd.md | sort -u | wc -l  # TDD coverage
      ```
      The TDD count must be ≥ spec count.
- [ ] Test ID range is `L5-T-*-001` through `L5-T-*-NNN` with ≥1 ID per spec scenario row in §3.3 and ≥1 per DoD criterion in §3.1.
- [ ] CI tiering table (T1/T2/T3/T4) exists with budget/cadence/inclusions per L4 pattern.
- [ ] Test pyramid section enumerates: static gates, unit, integration, E2E, manual smoke (for marketplace skills' anti-bot behavior, manual is acceptable).
- [ ] Bootstrap doc exists at `docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-bootstrap.md`.
- [ ] Bootstrap reflects skill-first architecture (operator-confirmed 2026-05-08): **ZERO paid API procurements**. The spec §3.9 Dependencies table is corrected to:
      - HMRC Trade Tariff: Required, public API, NO key (existing skill `skills/landed-cost/hmrc-customs-classification/` already uses it without auth).
      - HMRC Exchange Rate Service: Required, free monthly publication via gov.uk, NO key.
      - Xe: DECLINED — Frankfurter replacement.
      - SeaRates paid API: DECLINED — existing CDP skill.
      - Freightos paid API: DECLINED — operator-built CDP skill (`skills/landed-cost/freightos-freight-estimate/`).
      - Taric Support paid vendor: DECLINED — operator-built CDP skill (`skills/landed-cost/taric-cross-check/`).
- [ ] Bootstrap lists NO API key env vars beyond CDP browser config. Possibly `HMRC_TRADE_TARIFF_BASE_URL` for endpoint override only.
- [ ] Bootstrap explicitly states the two operator-built prerequisite skills (Freightos + Taric) must exist before Phase 3 / Phase 4 beads can claim, and the bead audit (step 09) verifies their presence.
- [ ] Bootstrap states the 4 cron entries L5 adds (HMRC tariff news, TRA case register, FX monthly rollover, marketplace source-health canary) — name each cron expression and the env var that enables it.
- [ ] Bootstrap names the 3 pre-resolved gold-set categories verbatim (per DoD #12, manifest `recorded_decisions.decision_4_gold_set_categories`): **fasteners + packaging-cartons + examination-gloves**. No "propose for owner decision" wording — the decision is closed.
- [ ] Bootstrap preflight script verifies HTTP/CDP connectivity (HMRC public endpoints, Frankfurter, CDP browser availability for SeaRates/Freightos/Taric skills) — NOT auth checks against paid APIs (none are used).

### Step 4: Follow-Up Pass (Sub-Optimization)

> Run after Step 3. Do not skip.

Re-read your TDD output specifically:

1. Open `docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-tdd.md` and your L5 TDD side-by-side. Are the section structures parallel? Did you skip any section the L4 TDD has? L4 has: Scope, Non-Negotiable Test Principles (11 rules), Test Pyramid (5 layers), CI Tiering (4 tiers, plus stage validation gate script). L5 must have all four. Add anything missing.

2. Open `crates/daemon/src/errors/manifest.rs:20-55` and list the existing 44 ErrorCode variants. For each L5 spec error code in §3.5, write a test ID that exercises it. Verify the matcher: `FpoAccessUnavailable` (existing) vs your spec's `FPO_UNAVAILABLE` (new) — are they actually distinct, or did the validate-spec step decide they're the same? Reflect that decision in the TDD's coverage matrix.

3. Open `migrations/0001_sqlite_state_initial.sql:322-394`. Walk every column on `landed_cost_estimates`. For each column, find the test ID that asserts it's populated. If a column has NO test asserting population, that's a gap — add a test. Common omissions: `customs_cost_spread_bps`, `mode_confidence_bps`, `vat_recovery_rate_bps`. They're easy to forget because they're derived fields.

4. Open `tests/fixtures/calibration/` and pick the largest existing variant (likely under `procurement-ingestion/contracts-finder/`). Read its `canary.inputs.jsonl` and `golden.assertions.yaml`. Your L5 marketplace skills' canary inputs must follow the same shape — confirm your TDD specifies the shape correctly. If you described them differently (e.g., one big YAML instead of JSONL + YAML pair), you've drifted from convention. Realign.

5. Open `crates/daemon/tests/skill_golden_journeys.rs:1-50`. Note the JourneyStage enum (Draft → Candidate → Canary → Shadow → Stable → Rollback). Your TDD's marketplace-skill maturation tests must exercise the promotion flow — a test that runs the same skill at `candidate` and `stable` stages and asserts the promotion gate is honored. Check: do you have those tests? If not, add them.

6. Did you specify the calibration harness mutation families per spec §3.5 + L4 TDD precedent (clean / weird / mutated / pairwise — 9 mutation families per L4)? If not, the customs evidence-tiered accuracy test is under-specified. Add the mutation taxonomy.

7. Did you list the new daemon test files explicitly? L5 will likely add: `crates/daemon/tests/customs_classification.rs`, `customs_measure_interaction.rs`, `customs_fx_engine.rs`, `customs_freight_engine.rs`, `customs_classification_accuracy.rs`. Plus the TS test files in `packages/intelligence/src/customs/*.test.ts` etc. List both kinds.

### Step 5: Scope-Preservation Check (MANDATORY — BLOCKING)

1. **Enumerate** every requirement ID in the validated spec.
2. **Verify** each one has ≥1 test ID in the TDD coverage matrix.
3. **Scan** the TDD for: `post-MVP`, `Phase 2+`, `stretch goal`, `nice to have`, `deferred`, `out of scope`, `future iteration`, `simplified`, `for now we skip`.
4. **If any requirement is missing** from coverage, add tests. **If any red-flag language** is present, remove it.

---

## Output

Two files:

| File | Path | Approx Size |
|------|------|-------------|
| TDD | `docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-tdd.md` | 600-900 lines |
| Bootstrap | `docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-bootstrap.md` | 700-1000 lines |

---

## Key Reminders

- L5's customs engine is intricate (5 recall sources, multi-family expansion, measure interaction, exporter resolution, classification freshness). The TDD is the artifact that prevents under-testing — be exhaustive on coverage, lean on assertion repetition.
- No HTTP mocking framework. Cassettes (`tests/fixtures/`) for unit; real HTTP for integration. Document this in the TDD's principles section.
- Bootstrap doc is the gate against "agent doesn't have an API key" stalls. List every env var, every account, every onboarding step. Treat it as an executable runbook.
- The 5 §3.6 Open Questions stay open. The TDD does NOT close them. It records the present-tense behavior that holds while they're open.
- ZERO SCOPE DRIFT: every L5 spec requirement gets test coverage.
