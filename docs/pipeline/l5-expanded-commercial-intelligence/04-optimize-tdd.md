# L5 Expanded Commercial Intelligence — TDD Optimize (Fresh-Eyes Audit)

## Your Mission

A different agent has just produced the L5 TDD + bootstrap doc via `/spec-to-tdd`. You did NOT do that work. You are auditing both with fresh eyes — checking coverage, testability, fixture realism, bootstrap completeness, and absence of scope drift.

L5 is the most test-intensive layer so far: 5 new test files, cumulative target 57 tests, evidence-tiered accuracy harness across 3+ goods categories, marketplace skill canaries for 4 skills, real HTTP integration tier across 6 mandatory external APIs. The TDD must be sharp; weak assertions here become weak L5 implementation later.

---

## What To Read (Do This First)

### Documents to Audit (TARGET)
```
docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-tdd.md
docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-bootstrap.md
```

### Verify Against (SOURCE OF TRUTH)
```
docs/strategy/layers/layer-5-expanded-commercial-intelligence.md   — The validated spec; every L5-FR-X.Y.Z must be covered
```

### Quality Bar (MATCH OR EXCEED)
```
docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-tdd.md         — 623 lines; 158 checks; THE bar
docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-bootstrap.md   — comprehensive env runbook; THE bar
```

### Codebase (GROUND TRUTH)
```
crates/daemon/tests/                                               — 58 .rs files; pattern reference
crates/daemon/tests/skill_golden_journeys.rs                       — 607 lines; promotion-flow test pattern
tests/fixtures/calibration/                                        — existing canary + golden formats
crates/daemon/src/errors/manifest.rs:20-55                       — 44 existing ErrorCode variants
migrations/0001_sqlite_state_initial.sql:322-394                   — 36 nullable decision-grade columns
infra/supabase/migrations/0004_rls_policies.sql                    — RLS rules for redaction-boundary tests
```

---

## Think Independently — 11 Audit Dimensions

### 1. Coverage Matrix Completeness

Mechanical check:
```bash
SPEC_IDS=$(rg -o 'L5-FR-[0-9]+\.[0-9]+\.[0-9]+' docs/strategy/layers/layer-5-expanded-commercial-intelligence.md | sort -u)
TDD_COVERED=$(rg -o 'L5-FR-[0-9]+\.[0-9]+\.[0-9]+' docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-tdd.md | sort -u)
diff <(echo "$SPEC_IDS") <(echo "$TDD_COVERED")
```
Any spec ID missing from the TDD is a coverage gap. Surface and fix in audit.

### 2. Test ID Stability

Test IDs follow `L5-T-{file-stem}-{NNN}`. Walk the TDD:
- Every test ID is unique.
- Every test ID encodes the file it lives in (so a downstream agent can grep `L5-T-customs_classification` and find every test in that file).
- Sequential numbering within file (no gaps).
- The 5 expected files (per spec §3.2) are: `hmrc-fpo`, `customs-classification`, `measure-interaction`, `fx-engine`, `freight-engine`. Plus likely supplementary files (`customs_classification_accuracy`, `customs_freshness`). Verify file count is 5-8 (not 12+ — over-fragmentation hurts maintainability).

### 3. Assertion Specificity

Open the customs-classification test catalog. For each test, check assertions:
- Numeric thresholds use explicit values (e.g., "duty_outcome_max_gbp_pence = 4850" not "expect a high duty").
- Equality checks name exact field values (e.g., "exporter_rate_basis = 'residual_default'" not "exporter rate is conservative").
- Set comparisons use sorted lists (e.g., "candidate_commodity_codes_json contains exactly [HS1, HS2, HS3] sorted").

### 4. Fixture Cassette Realism

The TDD uses fixture cassettes (recorded API responses) for unit tests. Verify:
- Cassette structure described: HTTP request fingerprint (method + URL + headers + body) → response (status + headers + body).
- Real-API recording mechanism specified: how is a cassette created? when is it refreshed? L4 used a "record once, replay forever, refresh on schema-detected drift" pattern — mirror it.
- Cassette location specified: `tests/fixtures/cassettes/{api}/{scenario}.json` is reasonable.
- Cassette fixtures include the failure cases (rate-limit 429, 5xx, malformed JSON, schema drift) — not just happy paths.

### 5. Real HTTP Integration Tier Discipline

Integration tests hit real APIs (HMRC sandbox where available, Xe sandbox, Freightos sandbox/SeaRates limited tier). Verify:
- The TDD's CI tiering places these in T3 (<25min) or T4 (<45min), not T1 (which would slow every commit).
- The TDD specifies cadence: nightly? weekly? on-demand? L4 used per-tier cadence — match it.
- The TDD names rate-limit budgets and explicit failure-handling for API outages (skip with WARN, not FAIL — outage is not test failure).

### 6. Calibration Harness Completeness

Spec §3.4 names the evidence-tiered accuracy framework. The TDD should specify:
- The corpus structure (gold / silver / bronze label per record).
- Mutation families for the calibration harness (per L4 TDD precedent: 9 mutation families). Customs-specific mutations: HS6 perturbation, exporter swap, country-of-origin swap, currency swap, packaging weight perturbation, etc. The TDD should enumerate these.
- Pairwise challenge sets — paired records that should produce different verdicts (e.g., "same product but different country-of-origin → different exporter_rate_basis").
- Wilson-bound precision gates per slice (worker × source × validator overlay).
- Minimum corpus sizes per category (≥3 categories per DoD #12).

### 7. Marketplace Skill Canaries

Each of alibaba/amazon/1688/[pcs] should have:
- `tests/fixtures/calibration/marketplace-search/{source}/canary.inputs.jsonl` with ≥5 representative inputs.
- `golden.assertions.yaml` with shape-validation + content-validation.
- Source-specific test scenarios:
  - Alibaba: Chinese retry trigger; "Ready to Ship" filter assertion; anti-bot canary.
  - 1688: Chinese-only enforcement (English input → reject or convert); domestic wholesale floor in result.
  - Amazon: alias family expansion; PA-API-5.0 sunset graceful handling (browser-first only).
  - PCS: IN-SCOPE (operator decision 2026-05-08). OCDS Web API integration; canary inputs + golden assertions in `tests/fixtures/calibration/marketplace-search/pcs/` (or `procurement-ingestion/pcs/`). Verify TDD includes PCS test scenarios.
- Promotion-flow tests: skill at `candidate` → run canary → assert acceptance criteria for promotion to `shadow` → `stable`. (Pattern from `skill_golden_journeys.rs`.)

### 8. Redaction-Boundary Tests

L5 enriches `landed_cost_estimates` with decision-grade fields. Some are commercially sensitive (named-exporter rate basis, evidence_leverage breakdown). Verify the TDD has tests asserting:
- Viewer-role projection of `landed_cost_estimates` does NOT include the sensitive fields (per `infra/supabase/migrations/0004_rls_policies.sql`).
- Operator-role projection DOES include them.
- New L5 fields are explicitly classified in the TDD as display / control / automation per L4's cross-authority classification (cross-reference-index.md row 229+).

### 9. Bootstrap Completeness

Walk the bootstrap doc:
- Every external API account from spec §3.9 is listed (HMRC tariff, HMRC FX, Xe, Freightos, SeaRates, Taric Support — 6 mandatory; HS Ping, Easyship/Shippo, CurrencyBeacon/OANDA, Dutify/Zonos/Avalara — 4 optional groups).
- Each account has: signup URL, account-tier requirements, key-creation steps, env-var name, dev-vs-prod separation.
- Preflight validation script (bash) verifies each env var is set + each endpoint reachable.
- Daemon dependencies: confirm `reqwest` is reused (no new HTTP crate added unless empirically needed).
- Cron entries listed with cron expressions and feature flags: HMRC tariff news watcher, TRA case register watcher, FX monthly rollover, marketplace source-health canary.
- Gold-set categories: ≥3 named (e.g., fasteners, packaging-cartons, examination-gloves) — surface for owner decision if any is undecided. Do NOT silently pick.
- Local-only quality gate sequence: `cargo fmt --check && cargo check --all-targets && cargo clippy --all-targets -- -D warnings && cargo test -p daemon --bin daemon && bun run build && bun run test`.

### 10. CBAM Forward Placeholder Tests

Today is 2026-05-08. Trial CBAM rates Q4 2026 (~6 months). Full from 2027-01-01.
- TDD has at least one test for tender delivery window crossing 2027-01-01 — assert future-regulatory-exposure flag emission.
- TDD has at least one test for non-crossing window — assert no flag.
- TDD acknowledges `cbamEstimateGbp` remains null until secondary legislation finalises rates — no rate test.
- Categories tested: aluminium, cement, fertiliser, hydrogen, iron and steel (the 2027 in-scope list per spec §1.14). Glass/ceramics explicitly not in scope.

### 11. Risk-Surface Tests

The customs engine has high consequence-of-error (residual rate vs named-exporter is a multi-percentage-point delta on landed cost). The TDD should have tests for:
- Catastrophic understatement: case where engine emitted a duty number, but actual was materially higher. The accuracy harness must be able to flag these.
- Unsafe auto-classification: engine auto-classified, should have escalated. Verify the test for `decision_grade = broker_required` is wired correctly.
- Trade remedy investigation opens during a tender window — TDD has a test for `TRA_INVESTIGATION_OPEN` emission and reclassification trigger.
- HMRC stop-press code change during a tender window — TDD has a test for `TARIFF_STRUCTURE_CHANGED` emission.

---

## Scope Drift Detection (MANDATORY — BLOCKING)

### Red-Flag Language Scan

```bash
rg -in 'post.?mvp|phase 2\+|stretch goal|nice to have|deferred|out of scope|future iteration|simplified for initial|can be added later|skip for now|not testing' \
  docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-tdd.md \
  docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-bootstrap.md
```

Allowed (rare):
- The CBAM placeholder note that mirrors the spec's regulation-bounded language.
- Notes that an Open Question from spec §3.6 will be calibrated post-implementation (with research back-reference).

Any other match is a BLOCKING failure.

### Coverage Verification

For every `L5-FR-X.Y.Z` in the spec — verify:
- ≥1 test ID in the TDD coverage matrix.
- The test specifies a concrete input, expected output, and assertion (not "verifies behavior").

### Drift Checklist

- [ ] All 12 DoD criteria have at least one test ID.
- [ ] All 10 customs error codes (§3.5) have at least one emission test.
- [ ] All 12 test scenarios (§3.3) appear as explicit test IDs in the catalog.
- [ ] All 6 conservative-default rows (§1.12) have parametric tests.
- [ ] All 4 freshness vectors (§1.11) have independent freshness tests.
- [ ] All 5 recall sources (§1.4) have agreement-and-disagreement tests.
- [ ] All 4 marketplace skills have canary input sets + golden assertions.
- [ ] CBAM crossing/non-crossing tests both present.
- [ ] Redaction-boundary tests cover ALL new sensitive fields.
- [ ] No "TODO" or "[placeholder]" anywhere.

---

## Follow-Up Pass (Run After Fixes)

> After making your audit fixes, send this as a follow-up message in the same session.

Re-read your fixes specifically:

1. Open the L5 TDD and your audit notes. For each gap you closed (added a missing test, tightened an assertion, added a fixture spec), verify the fix is internally consistent with the rest of the TDD. Adding a test ID `L5-T-fx_engine-022` means renumbering anything that came after — confirm the sequence is contiguous.

2. Open `crates/daemon/src/errors/manifest.rs:20-55`. Cross-reference your TDD's customs-error-code emission tests. Are any of the 10 new variants missing a test? If so, add it now.

3. Open `migrations/0001_sqlite_state_initial.sql:322-394`. Walk every column on `landed_cost_estimates`. For each, find the test ID asserting population. Re-verify nothing was dropped.

4. Open `tests/fixtures/calibration/`. Look at the existing largest variant. Did your bootstrap doc list the L5 calibration extension correctly (≥3 categories, mutation taxonomy)? If not, fix the bootstrap.

5. Open `infra/supabase/migrations/0004_rls_policies.sql` and identify which fields on `landed_cost_estimates` projection are viewer-visible vs operator-only. Cross-check against your redaction-boundary tests. If a new L5 field's classification is missing from the RLS policy, that's a cross-domain gap — note it as a risk in step 06 strategy optimization (don't silently fix it here; it's a sync-engine adjacent change).

6. Did your fixes accidentally remove anything from the bootstrap? Bootstrap completeness loss is silent scope reduction. Re-confirm all 6 mandatory + 4 optional API accounts are still listed.

7. Final check: would an implementation agent receiving this TDD + bootstrap know exactly what to build, what to test, and how to set up their environment, with zero clarifying questions? If not, fix the gap now.

---

## Output

Both files updated in place:
- `docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-tdd.md`
- `docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-bootstrap.md`

---

## Key Reminders

- The L4 TDD (623 lines, 158 checks) is the MINIMUM bar. L5's surface is comparable in breadth and deeper in customs — expect 700-900 lines.
- No mocks. Fixture cassettes for unit, real HTTP for integration. Bend the convention only if you have a strong reason — and document it.
- Customs is high-consequence. A wrong duty number affects bid pricing. Test specificity beats test count.
- ZERO SCOPE DRIFT: every spec requirement is covered.
