# L5 Expanded Commercial Intelligence - Test Bootstrap

## 1. Scope

This document defines how the Layer 5 Expanded Commercial Intelligence test plan is bootstrapped and run. It is the executable companion to:

- `docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-tdd.md`
- `docs/strategy/layers/layer-5-expanded-commercial-intelligence.md`

Layer 5 extends the existing Rust daemon, L4 sync/RLS surface, Bun `packages/intelligence`, existing landed-cost skills, and marketplace skill system. L5 is skill-first and public/free-data-first. Per the 2026-05-08 operator decision recorded in `recorded_decisions.external_api_account_state`, L5 has **zero paid commercial API procurements**.

No L5 bootstrap step creates vendor accounts, links to signup pages, requests API keys, or waits on commercial procurement. The environment contract covers:

- Public HMRC Trade Tariff APIs
- Public HMRC monthly exchange-rate publications
- Frankfurter public FX API
- Existing CDP browser skill for SeaRates
- Operator-built CDP browser skills for Freightos and EU TARIC
- Existing and promoted supplier marketplace skills

The official public references checked while preparing this bootstrap were:

- GOV.UK Trade Tariff API docs: `https://docs.trade-tariff.service.gov.uk/`
- HMRC monthly exchange rates: `https://www.trade-tariff.service.gov.uk/exchange_rates/monthly?year=2026`
- Frankfurter API docs: `https://www.frankfurter.app/docs/`
- PCS OCDS Web API help: `https://api.publiccontractsscotland.gov.uk/`

## 2. Success Criteria Coverage Summary

| DoD Item | Bootstrap Surface | Primary Tests / Commands |
|---------|-------------------|--------------------------|
| 1. Alibaba/1688 search skills operational | promoted skill dirs, canary/golden/rubric files, auth/source-health checks | `L5-T-freight_engine-009`, `010`, `013` |
| 2. Amazon search skill operational | browser-first Amazon source binding and canaries | `L5-T-freight_engine-011`, `013` |
| 3. PCS Scotland ingestion operational | `skills/procurement-ingestion/pcs/` six-file directory and registry entry | `L5-T-freight_engine-014` |
| 4. HMRC FPO/search-suggestion investigation | `l5-customs-hmrc-fpo-endpoint-discovery` bead and endpoint cassettes | `L5-T-hmrc_fpo-002`, `003`, `008` |
| 5. Multi-source recall from >=2 independent sources | recall cassettes and `candidateRecallAgreement` fixtures | `L5-T-hmrc_fpo-004`, `005`, `010` |
| 6. UK10 HMRC resolution | tariff graph cache and HMRC public commodity endpoint | `L5-T-customs_classification-001`-`004` |
| 7. Measure scanner and interactions | measure fixtures for additive, substitutive, conditional, unknown | `L5-T-measure_interaction-001`-`006` |
| 8. HMRC customs FX | monthly CSV/XML parser and rollover cron | `L5-T-fx_engine-001`, `005`, `009` |
| 9. Frankfurter commercial spot FX | public latest-rate parser and basis delta tests | `L5-T-fx_engine-002`, `003`, `011` |
| 10. Freightos + SeaRates freight data | CDP preflight, skill result schemas, discrepancy fixtures | `L5-T-freight_engine-003`, `004`, `005` |
| 11. Decision-grade landed cost | populated `landed_cost_estimates`, redaction and sync checks | `L5-T-customs_classification-009`, `012`, `L5-T-fx_engine-010` |
| 12. Gold-set category expansion | fasteners, packaging-cartons, examination-gloves calibration corpus | `L5-T-customs_classification-027`-`032` |

## 3. Environment Prerequisites

| Requirement | Why it is needed | Validation |
|------------|------------------|------------|
| Rust nightly with 2024 edition support | daemon crate and integration tests | `rustup toolchain list | rg nightly` |
| Cargo | Rust build/test/lint entrypoint | `cargo --version` |
| Bun compatible with root `packageManager` | TypeScript package tests and builds | `bun --version` |
| `curl` | public endpoint and CDP probes | `curl --version` |
| `jq` | JSON response shape checks | `jq --version` |
| `rg` | inventory and policy scans | `rg --version` |
| `ubs` | scoped changed-file bug scan before commit | `ubs --help` |
| writable temp filesystem | cassettes, temp SQLite, skill artifacts | create/write temp file |
| Layers 0-4 complete | L5 depends on pipeline, validation, sync, RLS, and web portal | L4 artifacts and Supabase migrations present |
| `crates/daemon/src/db/sync.rs` | `landed_cost_estimates` already in L4 push set | `rg landed_cost_estimates crates/daemon/src/db/sync.rs` |
| `infra/supabase/migrations/0004_rls_policies.sql` | redaction boundary for viewer/operator/admin/daemon roles | file exists and mentions `landed_cost_estimates` |
| HMRC Trade Tariff public API reachable | canonical UK commodity and measure data | unauthenticated commodity/search probe |
| HMRC monthly exchange rates reachable | customs valuation FX | unauthenticated monthly exchange-rate page probe |
| Frankfurter public API reachable | commercial spot FX | unauthenticated latest-rate probe |
| PCS OCDS Web API reachable | Scotland procurement ingestion | unauthenticated notice-list and notice-family probes |
| Chrome CDP on port 9222 | SeaRates/Freightos/TARIC skills use persistent browser state | `bin/ensure-cdp-9222` and `/json/version` |
| `agent-browser` available where CDP skills run | browser skill execution medium | `agent-browser --help` when installed |
| Existing HMRC skill | current screening-grade HMRC baseline to extend | `skills/landed-cost/hmrc-customs-classification/SKILL.md` |
| Existing simple FX skill | Frankfurter extension point | `skills/landed-cost/simple-fx-lookup/SKILL.md` |
| Existing SeaRates skill | first freight source | `skills/landed-cost/searates-freight-estimate/SKILL.md` |
| Operator-built Freightos skill | second freight source prerequisite | `skills/landed-cost/freightos-freight-estimate/SKILL.md` plus schemas |
| Operator-built TARIC skill | advisory cross-check prerequisite | `skills/landed-cost/taric-cross-check/SKILL.md` plus schemas |
| Marketplace supplier skill dirs | Alibaba, 1688, Amazon, eBay source coverage | `skills/supplier-price-enrichment/*` |
| Calibration corpus root | evidence-tiered accuracy and category promotion | `tests/fixtures/calibration/` |
| L5 artifact directory | TDD/bootstrap/strategy/bead outputs | `docs/pipeline/l5-expanded-commercial-intelligence/artifacts/` |

Allowed environment variables:

| Variable | Required? | Purpose |
|----------|-----------|---------|
| `HMRC_TRADE_TARIFF_BASE_URL` | No | endpoint override only; default `https://www.trade-tariff.service.gov.uk` |
| `HMRC_EXCHANGE_RATES_URL` | No | endpoint override only; default current-year monthly page |
| `FRANKFURTER_BASE_URL` | No | endpoint override only; default `https://api.frankfurter.app` |
| `PCS_OCDS_BASE_URL` | No | endpoint override only; default `https://api.publiccontractsscotland.gov.uk/v1` |
| `PCS_OCDS_WEB_API_ACCOUNT_STATE` | No | documentation sentinel only; default `public_no_account_required` |
| `CDP_PORT` / `AGENT_BROWSER_CDP_PORT` | No | CDP port override, default `9222` |
| `CDP_USER_DATA_DIR` | No | persistent Chrome profile override |
| `CDP_ENSURE_HELPER_PATH` | No | override for `bin/ensure-cdp-9222` |
| `L5_ENABLE_HMRC_TARIFF_NEWS_CRON` | No | enable weekly tariff structure/measure watcher |
| `L5_ENABLE_TRA_CASE_REGISTER_CRON` | No | enable daily TRA case-register watcher |
| `L5_ENABLE_HMRC_FX_ROLLOVER_CRON` | No | enable HMRC monthly FX rollover watcher |
| `L5_ENABLE_MARKETPLACE_SOURCE_HEALTH_CRON` | No | enable marketplace/CDP source-health canaries |

Forbidden paid API key variables in L5 are enforced by the committed deny-list in `scripts/l5-preflight.sh`. If any declined-vendor key variable is present, the preflight fails because its presence indicates scope drift toward paid commercial APIs. Keep the exact deny-list in the executable script, not duplicated across docs.

## 4. External Data Access Contract

| Service / Skill | Required? | Auth / Cost | Bootstrap Position |
|-----------------|-----------|-------------|--------------------|
| HMRC Trade Tariff | Yes | public API, no key, free | canonical UK commodity, hierarchy, measures, VAT, supplementary units |
| HMRC Exchange Rates for Customs and VAT | Yes | public monthly publication, no key, free | customs FX source |
| Frankfurter | Yes | public API, no key, free | commercial spot FX replacement for Xe |
| Xe Currency Data API | No | declined | replaced by Frankfurter |
| SeaRates paid API | No | declined | existing CDP skill `skills/landed-cost/searates-freight-estimate/` |
| Freightos paid API | No | declined | operator-built CDP skill `skills/landed-cost/freightos-freight-estimate/` |
| Taric Support paid vendor | No | declined | operator-built public EU TARIC CDP skill `skills/landed-cost/taric-cross-check/` |
| Dutify/Zonos/Avalara live vendor accounts | No | declined | comparison adapters use fixtures or captured samples only |

The operator-built Freightos and TARIC prerequisite skills are not L5 pipeline beads. They must exist before Phase 3 / Phase 4 beads can claim. Step 09 bead audit verifies `SKILL.md`, `result.schema.json`, and READY operator evidence reports under `docs/pipeline/l5-expanded-commercial-intelligence/operator-built-skills/test-evidence/`. Landed-cost prerequisite skills do not use `checkpoint.schema.json`; missing or BLOCKED empirical evidence is the hard execution blocker.

### 4.1 Account And Onboarding Contract

This section is intentionally explicit because L5 must not stall on "missing API key" assumptions. There are no L5 commercial onboarding steps.

| Surface | Account State | Onboarding Step | Failure Handling |
|---------|---------------|-----------------|------------------|
| HMRC Trade Tariff API | no account, no key | run HTTP probe against public endpoint | integration warning or stale classification path |
| HMRC monthly exchange rates | no account, no key | run HTTP probe against public gov.uk monthly page | `FX_RATE_MISSING` only if no valid snapshot exists |
| Frankfurter public FX | no account, no key | run HTTP probe against `api.frankfurter.app` | commercial confidence drops or last-valid snapshot path |
| SeaRates CDP skill | existing browser skill, no paid API | run CDP helper, verify skill directory, run canary | freight source warning; no paid API fallback |
| Freightos CDP skill | operator-built prerequisite, no paid API | verify skill files before Phase 3/4 claims | missing prerequisite blocks claim, not procurement |
| EU TARIC CDP skill | operator-built prerequisite, no paid vendor | verify skill files before Phase 3/4 claims | advisory source unavailable; HMRC remains authority |
| Alibaba / 1688 / Amazon / eBay | existing or promoted browser skills | persistent Chrome profile and source-health canaries | source-specific warning/blocker; no fabricated quotes |
| PCS procurement ingestion | public no-account OCDS Web API; `PCS_OCDS_WEB_API_ACCOUNT_STATE=public_no_account_required` | run unauthenticated list/family probes plus skill directory and canary/golden evidence | source unavailable warning; no Scotland de-scope |
| Dutify / Zonos / Avalara | captured-sample comparison only | no live account | fixture-only validation; no live credential request |

Forbidden onboarding actions:

- Do not create vendor accounts.
- Do not add signup links to the bootstrap.
- Do not request, document, or wait for API keys.
- Do not add lead-time rows for Xe, Freightos paid API, SeaRates paid API, Taric Support vendor, Dutify, Zonos, Avalara, HS Ping, Easyship, Shippo, CurrencyBeacon, OANDA, or any equivalent paid connector.
- Do not replace a blocked CDP skill with a paid API procurement item.

Required onboarding actions:

1. Confirm the repo toolchain (`cargo`, `bun`, `curl`, `jq`, `rg`, `ubs`).
2. Run `bin/ensure-cdp-9222` and verify `http://localhost:9222/json/version`.
3. Run the public HTTP probes in section 5.
4. Verify the existing and prerequisite skill directories in section 6.3.
5. Verify canary/golden files use JSONL inputs plus YAML assertions.
6. Seed the three closed gold-set categories: `fasteners`, `packaging-cartons`, `examination-gloves`.
7. Keep all five §3.6 open questions open and enforce their present-tense conservative behavior.

### 4.2 Open Questions Runtime Contract

The five L5 §3.6 open questions remain open. Bootstrap verifies the implementation has a current runtime behavior for each; it does not close the question or ask the operator to decide again.

| Open Question | Runtime Behavior While Open | Test Coverage |
|---------------|-----------------------------|---------------|
| Product-family-specific logistics assumptions | highest-cost feasible freight boundary when category-specific heuristics are not calibrated; block automated completion if weight/volume evidence is insufficient | `L5-T-freight_engine-002`, `006`, `007` |
| Chinese supplier source strategy | Alibaba/1688 remain floor evidence only; auth/source-health required; regulated-goods decisions need compliance/export evidence | `L5-T-freight_engine-009`, `010`, `016` |
| Customs cost spread materiality threshold | `customs_cost_spread_bps >= 1500` is the conservative placeholder; remedy/no-remedy and duty-type splits block regardless of bps | `L5-T-customs_classification-020`, `036` |
| Packaging materials sub-category reclassification | `packaging-cartons` is the easy validated slice; metal/glass/composite packaging requires broker/manual review until its own gold set passes | `L5-T-customs_classification-009`, `010`, `027` |
| CBAM rate integration timeline | emit exposure flag for confirmed 2027 sectors and keep `cbam_estimate_gbp_pence = null` until rate rules are final | `L5-T-customs_classification-025`, `026` |

### 4.3 Readiness Before Claiming Implementation Beads

Before any L5 implementation bead is claimed, the agent must be able to answer "yes" to each row:

| Check | Required Evidence |
|-------|-------------------|
| Public HMRC commodity probe works | `curl -sfS "$HMRC_TRADE_TARIFF_BASE_URL/api/v2/commodities/7008002000"` or accepted fallback returns JSON |
| Public HMRC search probe works | `curl -sfS "$HMRC_TRADE_TARIFF_BASE_URL/api/v2/search?q=screws"` or accepted fallback returns JSON |
| HMRC monthly FX page works | `curl -sfS "$HMRC_EXCHANGE_RATES_URL"` returns HTML/CSV source material |
| Frankfurter spot probe works | `curl -LfsS "$FRANKFURTER_BASE_URL/latest?from=USD&to=GBP"` returns JSON with `.rates.GBP` |
| PCS OCDS notice-list probe works | `curl -LfsS "${PCS_OCDS_BASE_URL:-https://api.publiccontractsscotland.gov.uk/v1}/Notices?dateFrom=05-2026&noticeType=2&outputType=0"` returns OCDS JSON with `releases[]` |
| PCS OCDS notice-family probe works | `curl -LfsS "${PCS_OCDS_BASE_URL:-https://api.publiccontractsscotland.gov.uk/v1}/Notice?id=ocds-r6ebe6-0000832398"` returns OCDS JSON with `records[]` |
| CDP browser is alive | `/json/version` returns browser metadata |
| Freightos prerequisite skill exists | `SKILL.md`, `result.schema.json`, and READY operator evidence report present |
| TARIC prerequisite skill exists | `SKILL.md`, `result.schema.json`, and READY operator evidence report present |
| Paid API keys absent | forbidden env-var scan returns empty |
| Gold categories fixed | exactly `fasteners`, `packaging-cartons`, `examination-gloves`; no owner-choice prompt |
| Coverage matrix parity | TDD `L5-FR-*` unique count is greater than or equal to spec unique count |

### 4.4 Expected Preflight Outcomes

The preflight script in section 5 is a readiness gate, not a production data run. Expected outcomes:

| Probe Class | Green Output | Yellow Output | Red Output |
|-------------|--------------|---------------|------------|
| Toolchain | all tools found | `ubs` absent on non-commit research machines | missing Cargo or Bun |
| Public HTTP | HMRC and Frankfurter return parsable body | transient 429/5xx with previous cassette available | all public endpoints unreachable and no cassette fallback |
| CDP | browser metadata returned | browser launches slowly but responds within timeout | no CDP response after helper timeout |
| Skill files | required skill files present | canary/golden missing before stable promotion | `SKILL.md` or schema missing for prerequisite skill |
| Env scan | no forbidden key vars set | harmless unrelated vendor env outside L5 namespace | L5 tries to consume a forbidden paid API key |
| Fixture corpus | three categories present | category missing metadata before implementation starts | category decision reopened or renamed |

## 5. Validation Script

Run this committed preflight from the repository root before L5 implementation or before executing the full L5 suite:

```bash
scripts/l5-preflight.sh
```

The script is the executable copy of this bootstrap contract. It checks toolchain availability, public HMRC/Frankfurter reachability with cassette fallback warnings, CDP readiness through `bin/ensure-cdp-9222`, declined-vendor key absence, L4 sync/RLS prerequisites, Freightos/TARIC READY evidence reports, skill artifact presence, and the three fixed gold-set category roots. Keep behavior changes in `scripts/l5-preflight.sh` and summarize them here rather than duplicating the full shell body in docs.

## 6. Dependency And Manifest Additions

### 6.1 Package Surface

L5 creates exactly 19 production files under `packages/intelligence`:

```text
packages/intelligence/src/customs/
  description-quality.ts
  recall-sources.ts
  tariff-graph.ts
  measure-scanner.ts
  exporter-resolution.ts
  freshness.ts
  accuracy.ts
packages/intelligence/src/fx/
  hmrc-exchange-rates.ts
  frankfurter-commercial-rates.ts
  fx-engine.ts
  fx-types.ts
packages/intelligence/src/logistics/
  mode-selector.ts
  freightos-skill-adapter.ts
  searates-skill-adapter.ts
  freight-engine.ts
  freight-types.ts
packages/intelligence/src/validation/
  dutify-comparison.ts
  zonos-comparison.ts
  avalara-comparison.ts
```

The three validation comparison files are fixture/captured-sample adapters only. They must not require live vendor credentials.

Daemon-facing HTTP work reuses the existing `reqwest` dependency and client patterns already present in the Rust workspace. L5 must not add a second HTTP client crate unless an implementation bead includes empirical evidence that `reqwest` cannot satisfy the public HMRC/Frankfurter path.

### 6.2 Test Workspace Layout

```text
packages/intelligence/src/customs/
  hmrc-fpo.test.ts
  customs-classification.test.ts
  measure-interaction.test.ts
packages/intelligence/src/fx/
  fx-engine.test.ts
packages/intelligence/src/logistics/
  freight-engine.test.ts

crates/daemon/tests/
  customs_classification.rs
  customs_fx_freight_engine.rs
  customs_classification_accuracy.rs

tests/fixtures/cassettes/l5/
  hmrc/
    commodity-7008002000-cn.json
    commodity-7008002000-cn-429.json
    commodity-7008002000-cn-5xx.json
    search-screws.json
    search-suggestions-fasteners.json
    malformed-json.json
    schema-drift-missing-commodity-id.json
  fx/
    hmrc-monthly-current.json
    hmrc-monthly-missing-month.json
    frankfurter-usd-gbp.json
    frankfurter-usd-gbp-429.json
    frankfurter-usd-gbp-malformed.json

tests/fixtures/l5/
  hmrc/
    fpo-endpoint-discovery.md
  customs/
    multi-family-three-families.json
    recall-five-sources-agree.json
    recall-five-sources-disagree.json
    fpo-only-not-evidence.json
    taric-disagrees-hmrc-uk-measure.json
    conservative-defaults.json
    freshness-vectors.json
    cbam-window-crossing.json
  measures/
    steel-substitutive-greater-of.json
    aluminium-extrusions-exporter-rates.json
    compound-specific-duty.json
    quota-additional-code-unknown.json
  fx/
    hmrc-monthly-2026-05.csv
    hmrc-monthly-2026-06.csv
    hmrc-correction-2026-04.csv
    frankfurter-usd-gbp.json
  freight/
    freightos-base-120000.json
    searates-base-185000.json
    freightos-no-quote.json
    searates-schema-drift.json

tests/fixtures/calibration/customs-classification/
  fasteners/
    canary.inputs.jsonl
    golden.assertions.yaml
    metadata.json
    manifest.md
    clean/{metadata.json,manifest.md,cases.jsonl}
    weird/{metadata.json,manifest.md,cases.jsonl}
    mutated/{metadata.json,manifest.md,cases.jsonl}
    pairwise/{metadata.json,manifest.md,pairs.jsonl}
  packaging-cartons/
    canary.inputs.jsonl
    golden.assertions.yaml
    metadata.json
    manifest.md
    clean/{metadata.json,manifest.md,cases.jsonl}
    weird/{metadata.json,manifest.md,cases.jsonl}
    mutated/{metadata.json,manifest.md,cases.jsonl}
    pairwise/{metadata.json,manifest.md,pairs.jsonl}
  examination-gloves/
    canary.inputs.jsonl
    golden.assertions.yaml
    metadata.json
    manifest.md
    clean/{metadata.json,manifest.md,cases.jsonl}
    weird/{metadata.json,manifest.md,cases.jsonl}
    mutated/{metadata.json,manifest.md,cases.jsonl}
    pairwise/{metadata.json,manifest.md,pairs.jsonl}

tests/fixtures/calibration/marketplace-search/
  alibaba/{canary.inputs.jsonl,golden.assertions.yaml}
  1688/{canary.inputs.jsonl,golden.assertions.yaml}
  amazon/{canary.inputs.jsonl,golden.assertions.yaml}
  pcs/{canary.inputs.jsonl,golden.assertions.yaml}
```

### 6.3 Skill Directory Standard

Every production-bound L5 worker or validator skill directory must contain:

```text
SKILL.md
result.schema.json
checkpoint.schema.json
canary.inputs.jsonl
golden.assertions.yaml
validator-rubric.yaml
```

Required L5 skill directories:

```text
skills/supplier-price-enrichment/alibaba/
skills/supplier-price-enrichment/1688/
skills/supplier-price-enrichment/amazon/
skills/supplier-price-enrichment/ebay/
skills/procurement-ingestion/pcs/
skills/landed-cost/freightos-freight-estimate/
skills/landed-cost/searates-freight-estimate/
skills/landed-cost/taric-cross-check/
skills/landed-cost/hmrc-customs-classification/
skills/landed-cost/simple-fx-lookup/
```

The first Phase 3/4 claim gate for operator-built landed-cost prerequisites is evidence-based: Freightos and TARIC require `SKILL.md`, `result.schema.json`, and a READY `TEST_REPORT.md` under `docs/pipeline/l5-expanded-commercial-intelligence/operator-built-skills/test-evidence/`. No `checkpoint.schema.json` is expected for these landed-cost prerequisite skills. The stable-production gate additionally requires canary, golden, and rubric evidence where the owning phase calls for promotion.

## 7. Fixture And Corpus Bootstrap

### 7.1 Gold-Set Categories

The gold-set categories are already decided by the 2026-05-08 operator decision:

- `fasteners`
- `packaging-cartons`
- `examination-gloves`

No implementation handoff should ask the owner to choose these categories again.

Each category directory under `tests/fixtures/calibration/customs-classification/` must contain:

```json
{
  "category": "fasteners",
  "evidence_tier_mix": {
    "gold_min": 10,
    "silver_min": 15,
    "bronze_allowed_for_training_only": true
  },
  "metrics": [
    "duty_outcome_accuracy",
    "trade_remedy_yes_no",
    "additional_code_required_yes_no",
    "supplementary_unit_required_yes_no",
    "review_trigger_recall",
    "catastrophic_understatement_rate",
    "unsafe_auto_classification_rate",
    "abstention_positive_class"
  ]
}
```

`golden.assertions.yaml` must separate gold, silver, and bronze assertions. Bronze-only labels cannot mark a category production-ready.

Each category also extends the existing calibration fixture convention (`clean`, `weird`, `mutated`, `pairwise`) with customs-specific mutation families. At minimum:

| Family | Required L5 Mutation Coverage |
|--------|-------------------------------|
| `clean` | gold/silver labeled cases with stable evidence URLs and resolved commodity families |
| `weird` | valid hard cases: aliases, mixed materials, translation variance, low result count |
| `mutated` | HS6 perturbation, exporter swap, country-of-origin swap, currency swap, packaging weight perturbation, supplementary-unit removal, Incoterm swap, additional-code evidence removal, VAT relief evidence removal |
| `pairwise` | sibling records sharing `pair_id`, with exactly one material evidence dimension changed per pair |
| audit overlays | source identity, page kind, currency/locale, unit basis, product identity, warning omission, count mismatch, fabricated evidence, continuation integrity |

The production gate requires all three decided categories to carry the root canary/golden files plus the mutation-family evidence above before stable category promotion.

### 7.2 Marketplace Canaries

Each of Alibaba, 1688, Amazon, and PCS gets at least five representative inputs. eBay retains a baseline production-binding check. Required canary coverage:

| Source | Required Assertion Families |
|--------|-----------------------------|
| Alibaba | result schema, Chinese retry trigger, anti-bot/auth failure, Ready-to-Ship confidence bump, full SearchProfile preservation |
| 1688 | result schema, Chinese-only clusters, auth-refresh hook, vocabulary mismatch, domestic wholesale floor role |
| Amazon | result schema, browser-first search, alias families, concrete variant selection, benchmark ceiling role |
| PCS | result schema, notice-list retrieval, notice-family retrieval, OCDS fields, source registry binding |
| eBay | stable baseline source, benchmark ceiling role, schema compatibility |

### 7.3 Golden Arithmetic Constants

The following constants are test-owned until category-specific calibration replaces them:

| Constant | Value | Test |
|----------|-------|------|
| `CUSTOMS_BRANCH_DIVERGENCE_BPS` | `1500` | `L5-T-customs_classification-020` |
| `FX_DELTA_RECALC_THRESHOLD_BPS` | `150` | `L5-T-fx_engine-004` |
| `FREIGHT_PROVIDER_DISAGREEMENT_THRESHOLD_BPS` | `2500` | `L5-T-freight_engine-004` |
| Steel substitutive fixture | greater of 25.0% safeguard or 21.1% AD on GBP 10,000 | `L5-T-measure_interaction-002` |
| Aluminium exporter fixture | 0.0% named rate vs 35.1% residual on GBP 10,000 | `L5-T-measure_interaction-012` |

## 8. Cron Configuration Seeding

L5 adds four scheduler-owned cron entries in `crates/daemon/src/scheduler/cron.rs`, following the existing `VALIDATION_CALIBRATION_READINESS_SWEEP_CRON` and `VALIDATION_CROSS_MODEL_AUDIT_CRON` pattern.

| Entry | Env Var | Cron Expression | Purpose |
|-------|---------|-----------------|---------|
| `hmrc-tariff-news-weekly` | `L5_ENABLE_HMRC_TARIFF_NEWS_CRON` | `0 15 3 * * 1` | weekly HMRC tariff structure and measure update check |
| `tra-case-register-daily` | `L5_ENABLE_TRA_CASE_REGISTER_CRON` | `0 45 2 * * *` | daily TRA investigation/provisional/definitive measure watcher |
| `hmrc-fx-monthly-rollover` | `L5_ENABLE_HMRC_FX_ROLLOVER_CRON` | `0 10 4 18-25 * *` | check HMRC monthly FX publication window and enqueue affected recalculations |
| `marketplace-source-health-canary` | `L5_ENABLE_MARKETPLACE_SOURCE_HEALTH_CRON` | `0 0 5 * * *` | daily source-health canaries for marketplace and CDP freight/TARIC skills |

Seed example:

```json
[
  {
    "source_id": "l5-commercial-intelligence",
    "skill_id": "landed-cost/hmrc-tariff-news-watch",
    "skill_version": "2026.05.19-1",
    "cron_expression": "0 15 3 * * 1",
    "enabled_env": "L5_ENABLE_HMRC_TARIFF_NEWS_CRON"
  },
  {
    "source_id": "l5-commercial-intelligence",
    "skill_id": "landed-cost/tra-case-register-watch",
    "skill_version": "2026.05.19-1",
    "cron_expression": "0 45 2 * * *",
    "enabled_env": "L5_ENABLE_TRA_CASE_REGISTER_CRON"
  },
  {
    "source_id": "l5-commercial-intelligence",
    "skill_id": "landed-cost/hmrc-fx-monthly-rollover",
    "skill_version": "2026.05.19-1",
    "cron_expression": "0 10 4 18-25 * *",
    "enabled_env": "L5_ENABLE_HMRC_FX_ROLLOVER_CRON"
  },
  {
    "source_id": "l5-commercial-intelligence",
    "skill_id": "source-health/l5-marketplace-commercial-intelligence",
    "skill_version": "2026.05.19-1",
    "cron_expression": "0 0 5 * * *",
    "enabled_env": "L5_ENABLE_MARKETPLACE_SOURCE_HEALTH_CRON"
  }
]
```

## 9. Command Matrix

Local-only quality gate sequence:

```bash
cargo fmt --check && cargo check --all-targets && cargo clippy --all-targets -- -D warnings && cargo test -p daemon --bin daemon && bun run build && bun run test
```

| Lane | Command |
|------|---------|
| Static | `cargo fmt --check` |
| Static | `cargo check --all-targets` |
| Static | `cargo clippy --all-targets -- -D warnings` |
| Static | `bun run build` |
| Static | `bun run test` |
| Static | `ubs docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-tdd.md docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-bootstrap.md` |
| Stage Gate | full mandatory block from TDD section 3 |
| HMRC/FPO package tests | `bun test packages/intelligence/src/customs/hmrc-fpo.test.ts` |
| Customs classification tests | `bun test packages/intelligence/src/customs/customs-classification.test.ts` |
| Measure interaction tests | `bun test packages/intelligence/src/customs/measure-interaction.test.ts` |
| FX tests | `bun test packages/intelligence/src/fx/fx-engine.test.ts` |
| Freight tests | `bun test packages/intelligence/src/logistics/freight-engine.test.ts` |
| Daemon binary tests | `cargo test -p daemon --bin daemon` |
| Daemon integration loop | `for t in crates/daemon/tests/*.rs; do cargo test --test "$(basename "$t" .rs)" || exit 1; done` |
| CDP health | `bin/ensure-cdp-9222 && curl -fsS http://localhost:9222/json/version | jq .Browser` |

Mandatory Stage Validation Test Gate:

```bash
cargo fmt --check
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test -p daemon --bin daemon
for t in crates/daemon/tests/*.rs; do
  cargo test --test "$(basename "$t" .rs)" || exit 1
done
bun run build
bun run test
```

## 10. Artifact Checklist

Before the full L5 suite can pass, these artifacts must exist:

- `docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-tdd.md`
- `docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-bootstrap.md`
- five L5 package test files listed in section 6.2
- three supplementary L5 daemon integration test targets listed in section 6.2
- 19 production package files listed in section 6.1
- `migrations/0013_l5_product_commodity_codes.sql` or next-free `NNNN_l5_product_commodity_codes.sql`
- updated `crates/daemon/src/errors/manifest.rs` with L5 error vocabulary and no duplicate `FPO_UNAVAILABLE`
- updated `crates/daemon/src/scheduler/cron.rs` with four L5 cron definitions
- updated `crates/daemon/src/sources/registry.rs` for Amazon and PCS source bindings
- `tests/fixtures/cassettes/l5/` HTTP cassette tree and `tests/fixtures/l5/` domain fixture tree
- `tests/fixtures/calibration/customs-classification/fasteners/`
- `tests/fixtures/calibration/customs-classification/packaging-cartons/`
- `tests/fixtures/calibration/customs-classification/examination-gloves/`
- promoted marketplace skill canary/golden/rubric files
- Freightos and TARIC prerequisite skill files plus READY operator evidence reports
- SeaRates, Freightos, and TARIC CDP source-health canary evidence

## 11. Structured Logging Requirements

Every L5 test emits:

- `[TEST_START] <test-id>`
- `[SETUP] <fixture names>`
- `[EXEC] <operation>`
- `[ASSERT] <specific assertion>`
- `[CLEANUP] <resource>`
- `[TEST_END] <test-id> PASS|FAIL`

Customs tests additionally log:

- product category
- candidate id
- recall source family
- `candidateRecallAgreement`
- selected HS6/CN8/UK10 branch
- HMRC source URL
- TARIC divergence state when present
- blocker/warning/error codes

Measure tests additionally log:

- customs value pence
- ad valorem rate bps
- specific-duty unit and amount
- interaction family
- selected duty pence
- evidence leverage delta bps

FX tests additionally log:

- HMRC snapshot month
- Frankfurter date/provider
- customs FX rate
- commercial FX rate
- `fx_basis_delta_bps`
- rollover/correction snapshot ids
- recalculation job ids

Freight and marketplace tests additionally log:

- provider skill id
- CDP state
- mode selection facts
- provider base/low/high amount pence
- discrepancy bps
- failure code
- source-health canary case id
- browser transcript artifact path when available

## 12. False Positive Prevention

| Risk | Prevention |
|------|------------|
| FPO-only candidate unlocks auto-classification | explicit FPO-only negative test keeps decision non-auto |
| Multiple query waves count as independent sources | `candidateRecallAgreement` counts source families, not waves |
| Greedy descent misses runner-up family | fixture where top family is wrong but runner-up is right; all plausible families must expand |
| TARIC overrides UK national measure | disagreement fixture asserts HMRC UK measure remains authoritative |
| Duty arithmetic double-charges substitutive remedies | steel golden case asserts greater-of result and rejects sum |
| Compound duties flatten to percent | compound fixture asserts structured components in `duty_breakdown_json` |
| Missing supplementary units get averaged | missing-unit tests assert abstention and `SUPPLEMENTARY_UNIT_MISSING` |
| Named-exporter relief accepted from seller name | aluminium fixture rejects seller/distributor name as proof |
| Conservative defaults hide upside | evidence leverage tests assert `unlockable_margin_bps` and per-parameter deltas |
| FX rollover leaves stale margins | rollover/correction tests assert recalculation jobs and snapshot preservation |
| Freight single-provider success hides discrepancy | dual-source tests require both providers or source-specific degradation |
| Dual-provider failure fabricates quote | failure tests assert `FREIGHT_ESTIMATE_FAILED` and manual-estimate requirement |
| Gold-set category promoted on bronze labels | accuracy harness separates bronze and blocks production readiness |
| Viewer sees local-only decision-grade evidence | redaction sentinels scan projection fields and RLS denials |
| Paid API scope creeps back in | preflight fails if paid key env vars or procurement instructions appear |
| Deprecated `bd` workflow appears in L5 handoffs | policy scan requires `br` wrapper terminology |

## 13. Notes For Implementation

- Keep the L5 automated test surface to the five package test files declared in the spec plus the three supplementary daemon targets named in section 6.2. Daemon integration files can be extended, but they should cite L5 test IDs rather than minting additional one-off L5 test files.
- Use fixture cassettes for deterministic unit tests and live public HTTP/CDP only in `T3`/`T4`.
- Keep `landed_cost_estimates` schema unchanged; populate existing fields only.
- `product_commodity_codes` is the one L5 local SQLite table addition and must use set-replacement semantics per candidate.
- `landed_cost_estimates` already syncs through L4; do not add a separate sync channel.
- HMRC FPO endpoint naming remains an investigation bead, not a procurement step.
- The existing `skills/landed-cost/simple-fx-lookup/` uses an older Frankfurter endpoint in its current text. L5 implementation should update it to the public latest-rate endpoint or preserve a documented compatibility fallback while tests target `latest?from=USD&to=GBP`.
- The current checkout has `skills/landed-cost/freightos-freight-estimate/` and `skills/landed-cost/taric-cross-check/` directories. If their `SKILL.md`, `result.schema.json`, or READY operator evidence reports are missing when Phase 3/4 begins, the preflight must block exactly as section 5 specifies; missing stable-production canary/golden/rubric artifacts remain later promotion warnings.
- Bead handoffs must use `bin/br-claim <bead-id>` and `bin/br-complete <bead-id>` for bead-tracked work, never raw `br update --status=in_progress` or raw `br close`.
