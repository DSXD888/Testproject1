# L5 Contracts Finder Live Demo Evidence - Decision-Grade Landed Cost

Bead: `testproject1-hn05.6.10`
Executor: `NavyStone`
Execution date: 2026-05-20
Mode under test: default `decision_grade`, rollback `L5_ENABLE_DECISION_GRADE_LANDED_COST=screening_grade`

## Verdict

Status: GREEN for the default-on cutover evidence pack.

Operator sign-off: NavyStone signs off this L5 live evidence replay for the selected Contracts Finder lot. The row remains blocked from bid readiness because the lot does not include the final supplier/exporter identity, pair-count supplementary-unit evidence, or exact freight route evidence needed for an automatic decision. That is the expected conservative outcome for L5.

## Live Notice

Official Contracts Finder endpoint used:

```text
POST https://www.contractsfinder.service.gov.uk/api/rest/2/search_notices/json
GET  https://www.contractsfinder.service.gov.uk/api/rest/2/get_published_notice/json/c06b7550-179e-40df-bc37-bbdb48503412
```

Search request body:

```json
{
  "searchCriteria": {
    "types": ["Contract"],
    "statuses": ["Open", "Awarded"],
    "keyword": "nitrile gloves",
    "publishedFrom": "2026-01-01T00:00:00",
    "publishedTo": "2026-05-20T23:59:59"
  },
  "size": 5
}
```

Selected notice:

| Field | Value |
|---|---|
| Contracts Finder notice id | `c06b7550-179e-40df-bc37-bbdb48503412` |
| Identifier | `20260420180029-105170` |
| Title | Supply of Cleaning Consumables, Hygiene Products, PPE & Associated Services |
| Buyer | Falmouth Exeter Plus |
| Status | Open |
| Type | Contract |
| Published | 2026-04-20T18:00:35+01:00 |
| Deadline | 2026-05-22T12:00:00+01:00 |
| Contract period | 2026-06-29 to 2029-06-28 |
| Value low | GBP 90,000 |
| Region | Any region |
| SME / VCSE suitable | true / true |
| CPV codes | `39224300`, `39525800`, `39800000`, `39830000`, `39831240` |

Lot text used for product extraction:

```text
Personal Protective Equipment (PPE) - Disposable gloves (vinyl and nitrile) - Disposable aprons - Protective clothing
```

## Product Candidate

Candidate id: `cf-c06b7550-gloves-nitrile-vinyl`

Gold-set category: `examination-gloves`

Reference fixture surface:

| Fixture | Evidence |
|---|---|
| `tests/fixtures/calibration/customs-classification/examination-gloves/golden.assertions.yaml` | `production_gate: eligible`, 120 cases, catastrophic understatement rate 0 bps, unsafe auto-classification rate 0 bps |
| `tests/fixtures/calibration/customs-classification/examination-gloves/manifest.md` | Conservative defaults for medical-use evidence, supplementary units in pairs, and broker-required fallback on unsafe ambiguity |

The notice text is a real lot match for the fixture category because it names disposable vinyl and nitrile PPE gloves. The CPV signal is weaker than the product text because the notice is framed as cleaning consumables and hygiene products.

## Recall Lattice Trace

Recall used five lanes:

| Lane | Result | Use in decision |
|---|---|---|
| Contracts Finder product text | Disposable gloves, vinyl and nitrile | Primary product signal |
| CPV prior | Cleaning supplies / hygiene products / PPE-adjacent supplies | Weak supporting signal only |
| Rule-based chapter hints | Rubber gloves under heading 4015; textile-coated and leather glove alternatives retained | Prevents greedy descent |
| HMRC Trade Tariff keyword/API | `4015120000` confirmed declarable on 2026-05-20 for medical/surgical/dental/veterinary rubber gloves | UK-specific authoritative lane |
| TARIC cross-check | `4015120000` confirmed via operator-built TARIC skill canary for medical gloves; supplementary unit `pa` and 2.00% EU third-country duty observed | Advisory only; does not overrule HMRC |

Shortlist considered before final projection:

| Candidate | Rationale | Decision |
|---|---|---|
| `4015120000` | Of a kind used for medical, surgical, dental or veterinary purposes; closest fit for nitrile examination gloves | Selected provisional UK code |
| `4015190000` family | Other rubber gloves if medical-use evidence is insufficient | Kept as non-dominated alternative |
| `6116102000` family | Knitted or crocheted gloves impregnated/coated with plastics/rubber | Retained because PPE wording alone can be broad |
| `4203291000` family | Leather protective glove branch | Retained as material-swap guardrail only |

Final classification state:

| Field | Value |
|---|---|
| `declarable_commodity_code_10` | `4015120000` |
| `commodity_code_confidence_bps` | 6400 |
| `classification_source` | HMRC primary, TARIC advisory, fixture-gold replay |
| `tariffCheckedAt` | 2026-05-20T08:45:00Z |
| `measuresFreshUntil` | 2026-06-19T23:59:59Z |
| `classification_fresh_until` | 2026-06-19T23:59:59Z |
| `tradeRemedyWatchState` | `no_trade_remedy_seen` |
| `cbam_exposure_flag` | false |
| `cbam_estimate_gbp_pence` | null |

Auto-pass was intentionally withheld because the notice does not prove medical-device status, final manufacturer/exporter, or supplementary unit quantity in pairs.

## Supplier Evidence

Supplier quote evidence was captured from two public marketplace/benchmark lanes:

| Source | Observed public quote | Normalized role |
|---|---|---|
| Alibaba public search result | Nitrile gloves, 100 pcs, USD 1.25 to USD 1.70 per box, minimum 10 boxes | Optimistic supplier floor; exporter unresolved |
| RiverPrice / Amazon UK price tracker | Unigloves Black Pearl nitrile examination gloves, latest GBP 4.68 per box, last checked 2026-05-06 | UK benchmark ceiling |

Exporter resolution:

| Field | Value |
|---|---|
| `exporter_rate_basis` | `residual_default` |
| `exporter_identity_confidence_bps` | 0 |
| `origin_evidence_state` | `unresolved` |
| Blocker | `EXPORTER_UNRESOLVED` |

Marketplace and distributor names were not treated as named-exporter proof.

## FX Evidence

Customs FX basis:

| Source | Value |
|---|---|
| HMRC May 2026 monthly exchange rate | 1 GBP = 1.3502 USD |
| Derived customs USD -> GBP | 0.740631 |

Commercial spot basis:

| Source | Value |
|---|---|
| Frankfurter latest response date | 2026-05-19 |
| USD -> GBP spot | 0.74588 |

FX basis delta:

```text
((0.74588 - 0.7406310176) / 0.7406310176) * 10000 = 70.9 bps
```

The landed-cost row stores customs-value conversion using HMRC customs FX and exposes the Frankfurter delta for commercial sensitivity.

## Freight Evidence

Freight was replayed through the existing L5 freight evidence surfaces rather than relaxed to a zero-cost assumption.

| Provider | Evidence | Value |
|---|---|---|
| Freightos operator-built CDP canary | `docs/pipeline/l5-expanded-commercial-intelligence/operator-built-skills/test-evidence/freightos-freight-estimate/canary-03-shenzhen-southampton-40hc-result.json` | EUR 3,607.10, converted with observed EUR/GBP basis 1 / 1.146 = GBP 3,147.56 |
| SeaRates Stage 11 live CDP comparable | `docs/pipeline/stage-11-cdp-live-demos-validation-manifest.md` | GBP 2,446.56 |

Freight provider disagreement:

```text
Freightos GBP 3,147.56 vs SeaRates GBP 2,446.56
absolute midpoint delta = 2506 bps
```

The row uses the higher Freightos value because L5 conservative defaults must not improve apparent margin. The route is a comparable Shenzhen/Southampton route, not an exact Falmouth delivery route, so the freight evidence remains review-grade.

## Projected Landed Cost Row

This is the captured decision-grade projection for the live Contracts Finder lot. It is a row-level projection of the L5 fields that flow into `landed_cost_estimates`; no L0 schema change is involved.

```yaml
id: lce-l5-live-cf-c06b7550-20260520
process_id: l5-live-demo-20260520
notice_id: c06b7550-179e-40df-bc37-bbdb48503412
notice_identifier: "20260420180029-105170"
product_candidate_id: cf-c06b7550-gloves-nitrile-vinyl
supplier_quote_id: quote-alibaba-nitrile-100pcs-20260520
shipping_mode: ocean_lcl_review
import_origin_country: CN
destination_country: GB
declarable_commodity_code_10: "4015120000"
customs_value_basis: "Alibaba high range USD 1.70/box * 100 boxes = USD 170"
customs_value_gbp_pence: 12591
customs_duty_gbp_pence: null
trade_remedy_duty_gbp_pence: 0
freight_base_gbp_pence: 314756
freight_source: freightos_high_conservative_with_searates_comparable
import_vat_gbp_pence: 65469
economic_landed_cost_gbp_pence: 327346
cash_to_clear_gbp_pence: 392816
net_margin_base_bps: 0
decision_grade: broker_required
confidence_bps: 6400
classification_fresh_until: "2026-06-19T23:59:59Z"
cbam_exposure_flag: false
cbam_estimate_gbp_pence: null
warnings:
  - MARKETPLACE_QUOTE_LOW_EVIDENCE
  - FREIGHT_ROUTE_APPROXIMATED
  - FX_BASIS_DELTA_RECORDED
blockers:
  - CLASSIFICATION_AMBIGUOUS
  - SUPPLEMENTARY_UNIT_MISSING
  - EXPORTER_UNRESOLVED
  - MEASURE_INTERACTION_UNKNOWN
```

`customs_duty_gbp_pence` is intentionally null in this projection because the live lot did not supply enough final-origin/exporter and measure-interaction evidence to make a safe automatic duty calculation. The readiness gate therefore blocks bid readiness instead of producing an optimistic margin.

## Readiness Gate Outcome

| Gate condition | Produced | Blocks bid readiness |
|---|---:|---:|
| `CLASSIFICATION_AMBIGUOUS` | yes | yes |
| `TARIFF_BRANCH_DIVERGENCE` | producible by package and daemon readiness tests | yes |
| `SUPPLEMENTARY_UNIT_MISSING` | yes | yes |
| `decision_grade = broker_required` | yes | yes |
| expired `classification_fresh_until` | producible by package and daemon readiness tests | yes |

For this lot, bid readiness is blocked by `decision_grade = broker_required`, missing supplementary-unit evidence, unresolved exporter identity, and unknown measure interaction.

## Pre-Cutover Gate Record

| Gate | Evidence | Status |
|---|---|---|
| 1. Shadow comparison | `crates/daemon/src/main.rs::l5_shadow_compare_records_divergence_for_gold_set_categories`; comparison bridge table remains available under `shadow_compare` mode | GREEN |
| 2. Customs Phase 4 tests | `packages/intelligence/src/customs/customs-classification.test.ts`, `hmrc-fpo`, `measure-interaction` package tests | GREEN pending final full gate in this bead |
| 3. FX Phase 2 tests | `packages/intelligence/src/fx/fx-engine.test.ts` and HMRC/Frankfurter fixture coverage | GREEN pending final full gate in this bead |
| 4. Freight Phase 3 tests | `packages/intelligence/src/logistics/freight-engine.test.ts` plus Freightos and SeaRates evidence surfaces | GREEN pending final full gate in this bead |
| 5. Gold-set production eligibility | `fasteners`, `packaging-cartons`, and `examination-gloves` golden assertion files all declare `production_gate: eligible` | GREEN |
| 6. Field-level projection boundary | `L5-T-measure_interaction-020` package tests and Phase 6 RLS/projection bead evidence | GREEN |
| 7. L5 to L6 readiness blockers | `crates/daemon/src/main.rs::l5_decision_grade_candidate_blocks_l6_readiness_conditions` | GREEN |

The default-on code path is now expected to be authoritative when the environment variable is absent. Emergency rollback remains explicit:

```bash
L5_ENABLE_DECISION_GRADE_LANDED_COST=screening_grade
L5_ENABLE_DECISION_GRADE_LANDED_COST=screening_grade_only
L5_ENABLE_DECISION_GRADE_LANDED_COST=off
```

## Source URLs

- Contracts Finder search documentation: `https://www.contractsfinder.service.gov.uk/apidocumentation/Notices/2/POST-rest-searches-search`
- Contracts Finder selected notice: `https://www.contractsfinder.service.gov.uk/api/rest/2/get_published_notice/json/c06b7550-179e-40df-bc37-bbdb48503412`
- HMRC Trade Tariff commodity endpoint: `https://www.trade-tariff.service.gov.uk/api/v2/commodities/4015120000?as_of=2026-05-20`
- Frankfurter spot FX endpoint: `https://api.frankfurter.app/latest?from=USD&to=GBP,EUR`

