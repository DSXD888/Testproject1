# Stage 11: Landed Cost - Research Document

> Generated: 2026-04-26 Europe/Uzhgorod
> Status: Research complete - ready for build planning
> Source-of-truth hierarchy: Code > Strategy docs > Research doc > Historical plan artifacts > External docs

## 0. Executive Summary (TLDR)

**What Stage 11 does:** Stage 11 takes the Stage 10 selected supplier quote and turns supplier price into a low/base/high landed-cost and net-margin band by adding freight, customs duty, trade remedies, import VAT, final-mile, other fees, and working-capital/cash-flow components. [L0 §12.5.11, lines 2943-3001] [research §5.7, lines 5397-5438] [layer-strategy, lines 1276-1291]

**Where it stands today:** The `landed_cost_estimates` schema exists, stage IDs and fixture rows exist, the current database has zero landed-cost rows, and the real deterministic pipeline does not call a production Stage 11 function. [migration 0001, lines 322-392] [main.rs:7307-7523] [main.rs:9840-9870] [SQLite read-only query 2026-04-25]

**To ship screening-grade Stage 11 (L1-L3 minimum):** The docs define screening-grade as HMRC Trade Tariff duty lookup, flat freight, single FX, and conservative defaults; the codebase has no production HMRC/freight/FX/classification client, so this is a medium build of roughly 4-7 person-days across a tariff client, commodity-code selection, flat freight table, FX cache, VAT computation, pipeline stage function, and deterministic wiring. [L0 §1.2, lines 50-62] [L0 §12.5.11, lines 2943-3001] [layer-strategy, lines 283-288] [code grep 2026-04-25]

**To ship decision-grade Stage 11 (L5):** L5 expands screening-grade into multi-source classification recall, UK10 expansion, measure interaction, trade-remedy monitoring, dual FX, Freightos/SeaRates logistics, exporter-rate basis, supplementary-unit enforcement, and richer decision-grade outputs; this is a large build of roughly 15-25 person-days because the docs list 18 new package files and multiple external integrations. [L5 §0, lines 15-38] [L5 §3.2, lines 226-276] [L5 §4.2, lines 321-359] [L5 §5, lines 380-401]

**Top 3 surprises from the research:**

1. The docs repeatedly describe `product_commodity_codes` as an L0/L4 entity, but the executable migration and current database do not contain that table. [L0 §4.4, lines 1397-1411] [layer-strategy, lines 1138-1157] [migration 0001, lines 322-392] [SQLite read-only query 2026-04-25]
2. The real deterministic pipeline currently stops before landed cost and net margin; only fixture/test paths write `landed_cost_estimates`. [main.rs:9840-9870] [main.rs:7307-7523] [crates/daemon/tests/agent_procurement_ingestion.rs:796-819]
3. The Stage 11 fixture values do not exactly match L5/L0 vocabulary: the fixture uses `valuation_mode = 'standard'` and `decision_grade = 'reviewable'`, while docs describe valuation modes like `1_proxy`/`1_actual` and decision grades like `screening`, `review`, `rfq_required`, and `broker_required`. [main.rs:7450-7523] [L5 §2.7, lines 150-157] [L5 §4.2, lines 321-359]

## 0.1 Decision Points (for the user)

| # | Decision | Options | Recommendation |
|---|----------|---------|----------------|
| 1 | Build screening-grade Stage 11 now? | Yes, to unblock Stage 12 / No, defer while pricing coverage is tiny | Build only if the next milestone requires Stage 12 or operator net-margin display; current DB has 4 pricing assessments and 0 landed-cost rows. [SQLite read-only query 2026-04-25] |
| 2 | Which L1-L3 freight model? | Flat internal rates / HMRC incidental VAT simplification / Freight API | Use flat internal per-mode bands; the research doc explicitly says HMRC incidental rates are VAT valuation simplifications, not commercial freight quotes. [research §2.4.3, lines 1168-1190] |
| 3 | Which classification source first? | HMRC Tariff keyword/search / CPV priors / FPO / manual seed | Start with HMRC Trade Tariff plus CPV/keyword priors; treat FPO as L5 because access is restricted to invited/eligible UK Carrier scheme organisations. [L5 §1.1, lines 40-54] [FPO API docs, lines 35-66] |
| 4 | VAT modeling complexity? | Standard UK VAT only / full VAT rate determination and recovery | For L1-L3, use standard VAT as conservative default and record recovery assumptions; L5 separates VAT rate state from VAT recovery. [L0 §12.5.11, lines 2943-3001] [L5 §2.6, lines 139-148] |
| 5 | Conservative defaults for missing data? | Apply MFN/residual/out-of-quota/standard / require all inputs | Apply conservative defaults and blockers; this is a constitutional rule in L0/L5. [L0 §12.5.11, lines 2943-3001] [L5 §2.9, lines 174-196] |
| 6 | Fix commodity-code storage now? | Add `product_commodity_codes` before Stage 11 / keep JSON-only candidates | Resolve the schema gap before L5; L1-L3 can use `candidate_commodity_codes_json`, but L5 needs the junction/table semantics described by the docs. [L0 §4.4, lines 1397-1411] [L5 §1.3, lines 63-84] |
| 7 | Normalize decision-grade vocabulary? | `screening/review/rfq_required/broker_required` / preserve fixture `reviewable` | Normalize to docs before production writes; downstream consumers should not inherit fixture-only vocabulary. [L5 §4.2, lines 321-359] [main.rs:7450-7523] |

## 1. Mission and Scope

Stage 11 is the landed-cost stage in the deterministic procurement pipeline. It sits after Stage 10 Gross Margin Gate and before Stage 12 Net Margin Gate. Stage 10 chooses a quote and computes gross margin; Stage 11 adds import/commercial cost components; Stage 12 consumes those landed-cost outputs to compute net margin and blockers. [L0 §12.5.10, lines 2888-2941] [L0 §12.5.11, lines 2943-3001] [L0 §12.5.12, lines 3002-3050] [layer-strategy, lines 1276-1291]

The stage grain is `candidate_quote_scenario`: each selected product/quote/shipping scenario can produce a separate landed-cost estimate, and Stage 11 should not recompute quote selection or expected revenue from Stage 10. [L1 §6, lines 1598-1610] [layer-strategy, lines 1224-1274] [research §5.7, lines 5383-5438]

The L1-L3 version is intentionally screening-grade: it uses HMRC tariff data, flat freight, a single FX model, and conservative defaults to avoid underestimating cost. [L0 §1.2, lines 50-62] [L0 §2.4, lines 215-220] [L0 §12.5.11, lines 2943-3001]

The L5 version is decision-grade: it adds multi-source classification recall, full UK10 commodity resolution, measure interaction, trade remedies, dual FX, and freight APIs without adding new columns because the nullable columns already exist from L0. [L5 §0, lines 15-38] [L5 §4.2, lines 321-359]

### 1.1 Stage boundary contract

| Boundary | Input/output | Contract |
|---|---|---|
| Stage 10 -> Stage 11 | `candidate_pricing_assessments`, selected quote, revenue basis, pricing confidence | Stage 11 reads the selected quote and revenue state from Stage 10; it should not re-run quote selection or gross margin. [L0 §12.5.10, lines 2888-2941] [research §5.7, lines 5383-5438] |
| Stage 11 internal | commodity candidates, freight assumptions, duty/VAT/FX data | Stage 11 turns the selected quote into low/base/high landed-cost scenarios and warnings/blockers. [L0 §12.5.11, lines 2943-3001] |
| Stage 11 -> Stage 12 | `landed_cost_estimates` row(s) | Stage 12 computes net margin, cash-to-clear, and blockers using landed-cost rows. [L0 §12.5.12, lines 3002-3050] [layer-strategy, lines 1276-1291] |
| Stage 11 -> Desktop | compact landed-cost view model | Desktop currently reads only id, candidate id, quote id, economic landed cost, base net margin, decision grade, and confidence. [apps/desktop/src-tauri/src/query_service.rs:506-516] [apps/desktop/src-tauri/src/query_service.rs:1185-1215] |
| Stage 11 -> coverage metrics | boolean/coverage contribution | TypeScript margin coverage currently only needs `hasLandedCost`, not detailed landed-cost math. [packages/intelligence/src/margins/coverage-rollup.ts:3-151] |

### 1.2 Scope exclusions

Stage 11 is not the ingestion skill, qualification classifier, quote scraper, bid workflow, or operator decision UI. The layer docs mention these nearby systems, but Stage 11's responsibility is cost estimation after a selected quote exists. [L0 §12.5.10-§12.5.12, lines 2888-3050] [layer-strategy, lines 1224-1291]

Stage 11 also is not a customs declaration engine in L1-L3. L5 moves closer to decision-grade customs reasoning, but even L5 uses conservative blockers and broker/RFQ grades for unresolved legal declarations. [L5 §0, lines 15-38] [L5 §6.1, lines 438-451]

### 1.3 Layer activation summary

| Layer | Stage 11 state |
|---|---|
| L0 | Schema and evaluation/gold-set intent; `landed_cost_estimates` exists with nullable future fields. [L0 §4.3, lines 862-935] [L0 §13.1, lines 2364-2389] |
| L1 | Live local runner should activate first live landed-cost writes, but code currently does not. [L1 §1.6, lines 277-294] [L1 §6.4, lines 1535-1555] [main.rs:9840-9870] |
| L2 | Desktop should display landed-cost breakdown/coverage, but still with screening-grade cost data. [L2 §5, lines 1578-1585] [L2 §5, lines 1629-1640] |
| L3 | No major Stage 11 feature expansion found; hardening affects reliability rather than new landed-cost logic. [docs grep 2026-04-25] |
| L4 | Sync semantics apply to rows/projections; no new decision-grade cost intelligence. [layer-strategy, lines 1138-1157] |
| L5 | Decision-grade landed cost, freight, FX, classification, remedies, and blockers. [L5 §0, lines 15-38] |
| L6 | Bid automation is gated on L5 freshness and no active blockers. [L6 §1, lines 52-75] |

## 2. Conceptual Model

### 2.1 The cost equation

```text
economic_landed_cost =
    supplier_cost
  + freight_cost
  + customs_duty
  + trade_remedy_duty
  + import_vat_nonrecoverable_component
  + uk_final_mile_cost
  + other_fees
  + working_capital_cost
```

The research doc formula includes supplier invoice price, shipping/freight, insurance/packing, duty, import VAT/tax exposure, customs/clearance fees, inland transport, and risk buffer; L0 narrows the executable Stage 11 cost components to supplier cost, freight low/base/high, customs duty, trade remedy duty, import VAT, final mile, other fees, economic landed cost, cash outlay, cash to clear, and working capital. [research §5.7, lines 5397-5438] [L0 §12.5.11, lines 2943-3001] [migration 0001, lines 322-392]

`economic_landed_cost_gbp_pence` is the commercial cost used for margin; `cash_outlay_gbp_pence` and `cash_to_clear_gbp_pence` model cash-flow timing and clearing obligations. [L0 §12.5.11, lines 2943-3001] [research Appendix A, lines 8071-8164] [migration 0001, lines 322-392]

### 2.2 Decision grade vs screening grade

Screening-grade L1-L3 means enough precision to decide whether a candidate obviously clears or fails margin gates, not enough precision for a legal customs declaration. It uses HS6/MFN-style lookup, flat freight, single FX, and conservative defaulting. [L0 §1.2, lines 50-62] [L0 §2.4, lines 215-220] [layer-strategy, lines 283-288]

Decision-grade L5 means the stage can withstand real bid pricing and broker/RFQ decisions: multiple recall sources, all plausible UK10 descendants, measure interaction, supplementary-unit requirements, trade remedies, exporter-specific rates, dual FX, and external freight estimates. [L5 §0, lines 15-38] [L5 §1.1-§2.4, lines 40-135] [L5 §3.2, lines 226-276]

Column-level diff: L1-L3 should populate identity, shipping mode, supplier/freight/duty/VAT/final-mile/aggregate/confidence/warnings/blockers fields; L5 additionally populates candidate commodity JSON, additional codes, supplementary units, valuation method/mode, dual FX fields, duty breakdown, measure interaction, exporter-rate basis, origin evidence, trade-remedy case JSON, freshness fields, CBAM exposure, and richer `decision_grade`. [L0 §13.1, lines 2364-2389] [L5 §4.2, lines 321-359] [migration 0001, lines 322-392]

### 2.3 Three scenarios

L0 defines freight low/base/high and net margin low/base/high fields; the fixture inserts low/base/high freight and net margin values; L5 says scenario values should be produced by freight APIs and uncertainty bands rather than a single flat value. [migration 0001, lines 322-392] [main.rs:7450-7523] [L5 §3.2, lines 262-276]

The screening-grade default should be pessimistic when unresolved: if the worst-case scenario clears the gate and no catastrophic branch exists, auto-pass may be allowed; otherwise Stage 11 should produce review/blocker output. [L5 §2.10, lines 198-207] [L0 §12.5.11, lines 2943-3001]

### 2.4 Constitutional conservative defaults

The conservative defaults are not optional; L0 and L5 both require them because landed cost underestimation is more dangerous than overestimation for margin gating. [L0 §12.5.11, lines 2943-3001] [L0 §20, lines 4163-4169] [L5 §2.9, lines 174-196]

| Default | Meaning | Layer |
|---|---|---|
| MFN / third-country duty | If preferential eligibility is not proven, use the third-country duty/MFN branch. | L1-L3 and L5. [L0 §12.5.11, lines 2943-3001] [L5 §2.9, lines 174-196] |
| Residual exporter rate | If named exporter/annex eligibility is unresolved, use residual/default anti-dumping or remedy rate. | L5, conservative default shared conceptually. [L5 §2.3, lines 96-112] |
| Out-of-quota assumption | If quota availability is unresolved, assume out-of-quota duty. | L1-L3 and L5. [L0 §12.5.11, lines 2943-3001] [L5 §2.9, lines 174-196] |
| Standard VAT | If VAT rate state is unresolved, default to standard VAT. | L1-L3 and L5. [L0 §12.5.11, lines 2943-3001] [L5 §2.6, lines 139-148] |
| Incoterm boundary scenarios | If Incoterm boundary is unclear, model conservative boundary scenarios instead of assuming seller-paid freight/duty. | L1-L3 and L5. [L0 §12.5.11, lines 2943-3001] [research §2.4.4, lines 1192-1200] |
| Supplementary-unit abstention | If required units are missing, abstain/block rather than flattening a duty. | L5, and L1-L3 should not pretend precision. [L0 §12.5.11, lines 2943-3001] [L5 §2.5, lines 126-135] |
| Additional-code unknown fallback | If an additional code unlocks lower duty and is unknown, default to the branch without the additional code. | L5. [L5 §2.4, lines 114-124] |
| No averaging across divergent customs branches | If plausible classifications produce materially different customs costs, block with branch divergence rather than averaging. | L5. [L5 §2.10, lines 198-207] |

### 2.5 Scenario lifecycle and staleness

Stage 11 should be rerun when source inputs stale: selected quote changes, tariff data changes, FX rolls over, freight assumptions change, or classification/freshness state expires. L1 docs explicitly say tariff/freight/FX freshness can rerun from landed cost onward. [L1 §6.5, lines 1649-1672]

L5 adds separate freshness vectors rather than one row timestamp because customs structure, measures, remedies, and FX can stale independently. [L5 §2.8, lines 159-172]

L6 bid workflow should not start from stale landed-cost intelligence; L6 readiness gates require complete L5 intelligence and no active blockers. [L6 §1, lines 52-75] [L6 §7, lines 286-294]

### 2.6 Confidence model shape

Confidence is a first-class output, not commentary. The schema has `confidence_bps`, `customs_resolution_confidence_bps`, `commercial_estimate_confidence_bps`, `revenue_confidence_bps`, and `confidence_breakdown`. [migration 0001, lines 322-392]

L0 describes confidence as weighted evidence and coverage rather than a single opaque grade; L5 adds `unlockable_margin_bps` and `evidence_leverage` to confidence breakdown. [L0 §12.6, lines 3058-3101] [L5 §2.9, lines 174-196]

For L1-L3, the safe default is to mark rows as screening/review quality even when numeric costs exist, because numeric output is not the same thing as decision-grade customs certainty. [L0 §1.2, lines 50-62] [L5 §4.2, lines 321-359]

## 3. Database Schema (`landed_cost_estimates`)

The executable schema is the SQLite migration, not the older research interface or strategy prose. The current migration defines 71 columns for `landed_cost_estimates`, and the live database currently has zero rows. [migration 0001, lines 322-392] [SQLite `.schema landed_cost_estimates`, 2026-04-25]

Legend: "L1-L3 populated" means the screening-grade flow should fill this column when enough data exists; "L5 populated" means decision-grade is the first layer expected to fill the field meaningfully.

### 3.1 Identity and ownership

| Column | Type | Layer population | Meaning/source |
|---|---:|---|---|
| `id` | TEXT PRIMARY KEY | L1-L3 | Estimate id generated by the stage writer. [migration 0001, lines 322-392] |
| `process_id` | TEXT NOT NULL | L1-L3 | Procurement process owning the candidate/quote. [migration 0001, lines 322-392] [L0 §4.3, lines 862-935] |
| `notice_id` | TEXT NOT NULL | L1-L3 | Source notice id for traceability. [migration 0001, lines 322-392] |
| `product_candidate_id` | TEXT NOT NULL | L1-L3 | Product candidate whose quote is being costed. [migration 0001, lines 322-392] [layer-strategy, lines 1224-1274] |
| `supplier_quote_id` | TEXT NOT NULL | L1-L3 | Selected quote from Stage 10. [migration 0001, lines 322-392] [L0 §12.5.10, lines 2888-2941] |
| `owner_id` | TEXT | L1-L3 | Local/sync ownership metadata. [migration 0001, lines 322-392] [L1 §1.6, lines 277-294] |
| `row_version` | INTEGER NOT NULL DEFAULT 0 | L1-L3 | Sync/concurrency row version. [migration 0001, lines 322-392] |
| `last_local_seq` | INTEGER | L1-L3 | Local sequence for sync projection. [migration 0001, lines 322-392] [L4 sync table references in layer-strategy, lines 1138-1157] |
| `stage_run_id` | TEXT | L1-L3 | Stage run that produced the row. [migration 0001, lines 322-392] |
| `updated_at` | TEXT NOT NULL | L1-L3 | Update timestamp. [migration 0001, lines 322-392] |
| `created_at` | TEXT NOT NULL | L1-L3 | Creation timestamp. [migration 0001, lines 322-392] |

### 3.2 Shipping and delivery

| Column | Type | Layer population | Meaning/source |
|---|---:|---|---|
| `shipping_mode` | TEXT | L1-L3 | Mode such as parcel/pallet/container/air/sea/road; fixture uses `parcel`. [migration 0001, lines 322-392] [main.rs:7450-7523] |
| `mode_confidence_bps` | INTEGER | L1-L3 | Confidence in selected shipping mode, in basis points. [migration 0001, lines 322-392] |
| `packing_scenario` | TEXT | L1-L3 | Packaging/load scenario for freight estimation. [migration 0001, lines 322-392] [L5 §3.2, lines 262-276] |
| `freight_low_gbp_pence` | INTEGER | L1-L3 | Low freight scenario. L1-L3 flat; L5 API-derived. [migration 0001, lines 322-392] [L5 §3.2, lines 262-276] |
| `freight_base_gbp_pence` | INTEGER | L1-L3 | Base freight scenario. [migration 0001, lines 322-392] |
| `freight_high_gbp_pence` | INTEGER | L1-L3 | High freight scenario. [migration 0001, lines 322-392] |
| `uk_final_mile_cost_gbp_pence` | INTEGER | L1-L3 | UK inland/final-mile delivery cost. [migration 0001, lines 322-392] [research §5.7, lines 5397-5438] |

The schema does not include explicit `lead_time_days` or `transit_time_days` columns in `landed_cost_estimates`. Supplier lead time exists on supplier quotes in the research model, while L5 logistics docs discuss freight scenario and APIs rather than a landed-cost lead-time column. [research §5.5.2, lines 3600-3636] [migration 0001, lines 322-392] [L5 §3.2, lines 262-276]

### 3.3 Classification and commodity resolution

| Column | Type | Layer population | Meaning/source |
|---|---:|---|---|
| `declarable_commodity_code_10` | TEXT | L1-L3 if available; L5 required | Declarable UK 10-digit commodity code. [migration 0001, lines 322-392] [L5 §1.3, lines 63-84] |
| `candidate_commodity_codes_json` | TEXT | L1-L3 optional; L5 required | Candidate classifications and evidence. [migration 0001, lines 322-392] [L5 §4.2, lines 321-359] |
| `additional_codes` | TEXT | L5 | Additional customs codes needed for specific measures/exporters/products. [migration 0001, lines 322-392] [Trade Tariff API, lines 326-333] |
| `additional_code_requirements_json` | TEXT | L5 | Evidence/requirements for additional-code satisfiability. [migration 0001, lines 322-392] [L5 §2.4, lines 114-124] |
| `supplementary_units_required_json` | TEXT | L5 | Required units such as kg, litres, pieces, or qualified units for specific/compound duties. [migration 0001, lines 322-392] [Trade Tariff API, lines 343-379] |
| `commodity_confidence_bps` | INTEGER | L1-L3 if inferred; L5 required | Classification confidence in basis points. [migration 0001, lines 322-392] [L5 §4.2, lines 321-359] |
| `classification_stability_bps` | INTEGER | L5 | Stability/confidence across branches and sources. [migration 0001, lines 322-392] [L5 §1.3, lines 63-84] |

Important schema gap: L0 and layer-strategy describe a `product_commodity_codes` entity/junction, but the migration and current DB do not define that table. Future build work should treat this as an open design issue, not an implementation detail to ignore. [L0 §4.4, lines 1397-1411] [layer-strategy, lines 1138-1157] [migration 0001, lines 322-392] [SQLite read-only query 2026-04-25]

### 3.4 Customs valuation and FX

| Column | Type | Layer population | Meaning/source |
|---|---:|---|---|
| `customs_valuation_method` | TEXT | L1-L3 basic; L5 required | Method such as transaction value or proxy/fallback valuation. [migration 0001, lines 322-392] [L5 §2.7, lines 150-157] |
| `valuation_mode` | TEXT | L5 | Docs describe estimation vs declaration modes (`1_proxy`/`1_actual`); fixture currently writes `standard`. [migration 0001, lines 322-392] [L5 §2.7, lines 150-157] [main.rs:7450-7523] |
| `customs_fx_rate` | REAL | L1-L3 or L5 | HMRC monthly customs FX for customs valuation. [migration 0001, lines 322-392] [L5 §3.1, lines 240-251] [HMRC exchange rates, lines 36-45] |
| `commercial_fx_rate` | REAL | L5 | Commercial spot FX, intended Xe in L5. [migration 0001, lines 322-392] [L5 §3.1, lines 240-251] [Xe API, lines 25-27, 230-237] |
| `fx_basis_delta_bps` | INTEGER | L5 | Basis-point spread between customs FX and commercial FX. [migration 0001, lines 322-392] [L5 §3.1, lines 240-251] |
| `fx_rate_checked_at` | TEXT | L1-L3 if FX fetched; L5 required | Freshness timestamp for FX source. [migration 0001, lines 322-392] [L5 §2.8, lines 159-172] |

### 3.5 Cost components and outcomes

| Column | Type | Layer population | Meaning/source |
|---|---:|---|---|
| `supplier_cost_gbp_pence` | INTEGER | L1-L3 | Supplier quote cost converted to GBP. [migration 0001, lines 322-392] [L0 §12.5.10, lines 2888-2941] |
| `customs_duty_gbp_pence` | INTEGER | L1-L3 | Customs duty estimate. [migration 0001, lines 322-392] [L0 §12.5.11, lines 2943-3001] |
| `trade_remedy_duty_gbp_pence` | INTEGER | L5 | Anti-dumping/countervailing/safeguard remedy duty. [migration 0001, lines 322-392] [L5 §2.4, lines 114-124] |
| `duty_outcome_min_gbp_pence` | INTEGER | L5 | Minimum plausible duty branch. [migration 0001, lines 322-392] [L5 §2.10, lines 198-207] |
| `duty_outcome_max_gbp_pence` | INTEGER | L5 | Maximum plausible duty branch. [migration 0001, lines 322-392] [L5 §2.10, lines 198-207] |
| `customs_cost_spread_bps` | INTEGER | L5 | Spread across plausible customs branches; material divergence blocks. [migration 0001, lines 322-392] [L5 §2.10, lines 198-207] |
| `import_vat_gbp_pence` | INTEGER | L1-L3 | Import VAT estimate; rate vs recovery is separated in L5. [migration 0001, lines 322-392] [L5 §2.6, lines 139-148] |
| `other_fees_gbp_pence` | INTEGER | L1-L3 | Broker/clearance/other fees. [migration 0001, lines 322-392] [research §5.7, lines 5397-5438] |
| `economic_landed_cost_gbp_pence` | INTEGER | L1-L3 | Commercial landed cost for margin. [migration 0001, lines 322-392] [L0 §12.5.11, lines 2943-3001] |
| `cash_outlay_gbp_pence` | INTEGER | L1-L3 | Cash outlay exposure. [migration 0001, lines 322-392] [L0 §12.5.12, lines 3002-3050] |
| `cash_to_clear_gbp_pence` | INTEGER | L1-L3 | Cash needed to clear import/customs/VAT timing. [migration 0001, lines 322-392] [research Appendix A, lines 8071-8164] |
| `working_capital_cost_gbp_pence` | INTEGER | L1-L3 or L5 | Cost of capital from cash timing. [migration 0001, lines 322-392] [research Appendix A, lines 8071-8164] |

### 3.6 Duty detail, VAT detail, origin, and remedies

| Column | Type | Layer population | Meaning/source |
|---|---:|---|---|
| `trade_remedy_state` | TEXT | L5 | State of remedy exposure detection. [migration 0001, lines 322-392] [L5 §2.8, lines 159-172] |
| `trade_remedy_cases_json` | TEXT | L5 | Referenced remedy cases and evidence. [migration 0001, lines 322-392] [L5 §4.2, lines 321-359] |
| `duty_breakdown_json` | TEXT | L5 | Preserved ad valorem/specific/compound duty components. [migration 0001, lines 322-392] [L5 §2.5, lines 126-135] |
| `measure_interaction_json` | TEXT | L5 | AD/CVD/safeguard/quota/additional-code interaction result. [migration 0001, lines 322-392] [L5 §2.4, lines 114-124] |
| `exporter_rate_basis` | TEXT | L5 | `named_exporter`, `annex_exporter`, or `residual_default`. [migration 0001, lines 322-392] [L5 §2.3, lines 96-112] |
| `exporter_identity_confidence_bps` | INTEGER | L5 | Confidence in exporter/manufacturer identity match. [migration 0001, lines 322-392] [L5 §2.3, lines 96-112] |
| `origin_evidence_state` | TEXT | L5 | State of origin evidence needed for duty/remedy branch. [migration 0001, lines 322-392] [L5 §2.3, lines 96-112] |
| `vat_rate_state` | TEXT | L5 | VAT rate determination state: standard/reduced/zero/exempt/unknown. [migration 0001, lines 322-392] [L5 §2.6, lines 139-148] |
| `import_vat_treatment` | TEXT | L1-L3 basic; L5 richer | Recovery/postponed-accounting/cash treatment. [migration 0001, lines 322-392] [L5 §2.6, lines 139-148] |
| `vat_recovery_rate_bps` | INTEGER | L5 | Recoverable portion of import VAT in basis points. [migration 0001, lines 322-392] [L5 §2.6, lines 139-148] |

### 3.7 Revenue, margins, confidence, blockers

| Column | Type | Layer population | Meaning/source |
|---|---:|---|---|
| `revenue_basis_type` | TEXT | L1-L3 | Revenue basis passed through from Stage 10. [migration 0001, lines 322-392] [L0 §12.5.10, lines 2888-2941] |
| `revenue_confidence_bps` | INTEGER | L1-L3 | Revenue confidence from gross margin stage. [migration 0001, lines 322-392] [main.rs:11178-11325] |
| `customs_resolution_confidence_bps` | INTEGER | L1-L3 basic; L5 richer | Confidence in customs/classification branch. [migration 0001, lines 322-392] |
| `commercial_estimate_confidence_bps` | INTEGER | L1-L3 | Confidence in commercial cost components. [migration 0001, lines 322-392] |
| `net_margin_low_bps` | INTEGER | L1-L3 | Low net-margin scenario. [migration 0001, lines 322-392] [L0 §12.5.12, lines 3002-3050] |
| `net_margin_base_bps` | INTEGER | L1-L3 | Base net-margin scenario. [migration 0001, lines 322-392] |
| `net_margin_high_bps` | INTEGER | L1-L3 | High net-margin scenario. [migration 0001, lines 322-392] |
| `decision_grade` | TEXT | L1-L3 and L5 | Reliability/action grade. Docs describe `screening`, `review`, `rfq_required`, `broker_required`; fixture has `reviewable`. [migration 0001, lines 322-392] [L5 §4.2, lines 321-359] [main.rs:7450-7523] |
| `confidence_bps` | INTEGER | L1-L3 | Aggregated confidence. [migration 0001, lines 322-392] [L0 §12.5.12, lines 3058-3101] |
| `confidence_breakdown` | TEXT | L5 | JSON breakdown including unlockable margin/evidence leverage in L5. [migration 0001, lines 322-392] [L5 §2.9, lines 174-196] |
| `warnings` | TEXT | L1-L3 | JSON warning array. [migration 0001, lines 322-392] [main.rs:7450-7523] |
| `blockers` | TEXT | L1-L3 and L5 | JSON hard blockers such as ambiguous classification or missing supplementary units. [migration 0001, lines 322-392] [L5 §6.1, lines 438-451] |

### 3.8 Freshness and CBAM

| Column | Type | Layer population | Meaning/source |
|---|---:|---|---|
| `tariff_checked_at` | TEXT | L1-L3 if tariff queried; L5 required | Tariff structure lookup timestamp. [migration 0001, lines 322-392] [L5 §2.8, lines 159-172] |
| `measures_fresh_until` | TEXT | L5 | Measure freshness expiry. [migration 0001, lines 322-392] [L5 §2.8, lines 159-172] |
| `classification_fresh_until` | TEXT | L5 | Classification freshness horizon. [migration 0001, lines 322-392] [L5 §2.8, lines 159-172] |
| `trade_remedy_watch_state` | TEXT | L5 | Daily TRA/trade-remedy watcher state. [migration 0001, lines 322-392] [L5 §2.8, lines 159-172] |
| `cbam_exposure_flag` | INTEGER | L5 | Whether commodity is in UK CBAM scope. [migration 0001, lines 322-392] [L5 §2.11, lines 209-215] |
| `cbam_estimate_gbp_pence` | INTEGER | L5 future | Estimated CBAM liability; docs keep nullable because UK CBAM starts 2027. [migration 0001, lines 322-392] [L5 §2.11, lines 209-215] [UK CBAM factsheet, lines 122-130] |

### 3.9 Schema divergences to resolve before build

| Divergence | Evidence | Build implication |
|---|---|---|
| `product_commodity_codes` missing | L0/layer-strategy describe it; migration/current DB do not contain it. [L0 §4.4, lines 1397-1411] [layer-strategy, lines 1138-1157] [SQLite read-only query 2026-04-25] | L1-L3 can store candidate codes in JSON, but L5 needs a real multi-candidate structure or a deliberate JSON-only design decision. |
| Fixture `decision_grade = reviewable` | Fixture insert writes `reviewable`; L5 documents `screening/review/rfq_required/broker_required`. [main.rs:7450-7523] [L5 §4.2, lines 321-359] | Production should normalize before downstream UI/test contracts solidify around fixture vocabulary. |
| Fixture `valuation_mode = standard` | Fixture insert writes `standard`; L5 describes estimation/declaration modes and `1_proxy`/`1_actual`. [main.rs:7450-7523] [L5 §2.7, lines 150-157] | Build needs a canonical enum for screening and decision-grade valuation states. |
| No lead-time columns | Research supplier quote has lead time; landed-cost table lacks transit/lead columns. [research §5.5.2, lines 3600-3636] [migration 0001, lines 322-392] | Delivery-time estimates either remain on quote/logistics side or require migration. |
| Older research schema differs from migration | Research interface/SQL is older; migration is executable truth. [research §5.6, lines 3705-3779] [research §6.3, lines 4325-4390] [migration 0001, lines 322-392] | Build agents must code to migration/current DB, not older interface snippets. |

### 3.10 Column population matrix by layer

| Group | L1-L3 screening-grade minimum | L5 decision-grade expansion |
|---|---|---|
| Identity | All identity/sync/stage columns. [migration 0001, lines 322-392] | Same columns. [L5 §0, lines 30-38] |
| Shipping | `shipping_mode`, mode confidence, packing scenario if inferred, flat freight low/base/high. [L0 §2.4, lines 215-220] | API-backed freight scenarios, packaging ambiguity branches, possibly delivery-time metadata outside table. [L5 §3.2, lines 262-276] |
| Classification | One declarable/default commodity branch or JSON candidates with low confidence. [L0 §12.5.11, lines 2943-3001] | Multi-source recall, UK10 families, additional codes, supplementary units, stability. [L5 §1, lines 40-84] |
| Customs/Fx | Basic customs value and one FX model. [L0 §2.4, lines 215-220] | Valuation method/mode plus HMRC customs FX and Xe commercial FX. [L5 §3.1, lines 240-251] |
| Duties | Customs duty and VAT; trade remedy default if unresolved. [L0 §12.5.11, lines 2943-3001] | Duty breakdown, measure interaction, trade remedies, branch spread. [L5 §2.4-§2.5, lines 114-135] |
| Aggregates | Supplier/freight/duty/VAT/final-mile/other/economic/cash/net margin. [migration 0001, lines 322-392] | Same plus better scenario derivation and confidence. [L5 §4.2, lines 321-359] |
| Freshness | Tariff/FX checked timestamps if fetched. [L1 §6.5, lines 1649-1672] | Full tariff/measures/classification/remedy/FX freshness vectors. [L5 §2.8, lines 159-172] |

## 4. Customs Classification

### 4.1 Multi-source recall (5 parallel sources)

L5 requires five recall sources and explicitly says FPO is not the sole authority. The goal is recall, not immediate legal confidence; the legal confidence comes after UK10 expansion, measure resolution, and evidence checks. [L5 §1.1, lines 40-54]

| Source | Role | Built today? |
|---|---|---|
| HMRC FPO | Suggests HS6/CN8 candidates from English item descriptions. | No integration; only a fixture warning mentions FPO unavailable. [FPO API docs, lines 35-46] [main.rs:7195-7196] |
| CPV-to-chapter priors | Maps procurement CPV/category signals to likely HS chapters. | No Stage 11 implementation found. [L5 §1.1, lines 40-54] [code grep 2026-04-25] |
| Rule-based chapter hints | Extracts attributes/material/function hints from product text. | No Stage 11 implementation found. [L5 §1.2, lines 56-61] [code grep 2026-04-25] |
| Tariff keyword search | Uses tariff descriptions/search to retrieve candidate codes. | No Stage 11 implementation found. [L5 §1.1, lines 40-54] [Trade Tariff API, lines 42-56] |
| ATR/codebook reuse | Reuses prior rulings/internal classification memory. | No Stage 11 implementation found; docs treat as a recall source. [L5 §1.1, lines 40-54] [research §4.5, lines 1424-1535] |

Historical planmaxer docs place more emphasis on FPO as a first-pass classifier, but current L5 says FPO is one recall source among five, so future build planning should follow L5. [planmaxer iteration-003 customs handoff, lines 1527-1606] [L5 §1.1, lines 40-54]

### 4.2 Multi-family expansion (HS6 -> CN8 -> UK 10-digit)

L5 forbids greedy descent from a single HS6/CN8 candidate. The required algorithm expands all plausible HS6/CN8 families into all declarable UK10 descendants through the HMRC Trade Tariff API, then shortlists based on evidence and cost divergence. [L5 §1.3, lines 63-84]

HMRC's commodities API is for declarable 10-digit commodities, and it states `goods_nomenclature_item_id` is the 10-digit commodity code and declarable commodities have `producline_suffix = 80`. [Trade Tariff API, lines 50-63] [Trade Tariff API, lines 74-99]

The intended `product_commodity_codes` junction would store multiple candidate codes with source, score, rank, and evidence, but the current schema lacks it; the existing table only has `candidate_commodity_codes_json`. [L0 §4.4, lines 1397-1411] [migration 0001, lines 322-392]

### 4.3 Conservative defaults applied

Classification defaults are: prefer the higher-cost branch when evidence is missing, use MFN if preference is not proven, use residual exporter rate if exporter specificity is unresolved, use out-of-quota when quota evidence is absent, and block if supplementary units or branch divergence prevent a reliable estimate. [L0 §12.5.11, lines 2943-3001] [L5 §2.9, lines 174-196] [L5 §2.10, lines 198-207]

### 4.4 Classification freshness

L5 defines four freshness vectors: tariff structure (`tariffCheckedAt`), measures (`measuresFreshUntil`), trade remedies (`tradeRemedyWatchState`), and FX (`fxRateCheckedAt`). [L5 §2.8, lines 159-172]

The HMRC Trade Tariff API page says the tariff is updated daily and recommends caching requests for 24 hours, which aligns with the L5 freshness model. [Trade Tariff API About, lines 74-80]

The L6 readiness gate requires no active L5 blockers and freshness before bid workflow automation may start. [L6 §1, lines 52-75] [L6 §7, lines 286-294]

### 4.5 Screening-grade classification algorithm sketch

This is a research recommendation, not a documented function. It follows the L0/L1 scope by minimizing external dependencies while preserving conservative behavior. **[Unsourced - inferred from cited docs]** [L0 §12.5.11, lines 2943-3001] [L5 §2.9, lines 174-196]

| Step | Action | Output |
|---|---|---|
| 1 | Read selected quote/product text from Stage 10 context. | Product description and supplier quote inputs. [research §5.7, lines 5383-5438] |
| 2 | Use HMRC tariff search/commodity lookup or CPV/category priors to identify one or more plausible commodity codes. | Candidate code JSON or selected declarable code. [Trade Tariff API, lines 42-63] |
| 3 | If confidence is low or multiple cost-divergent candidates exist, write blocker/warning instead of silent pass. | `warnings`, `blockers`, `commodity_confidence_bps`. [L5 §2.10, lines 198-207] |
| 4 | Query tariff measures for the selected/default branch. | Duty/VAT measures. [Trade Tariff API, lines 125-183] |
| 5 | Apply conservative defaults where preference/quota/remedy/source data is missing. | Safe duty branch. [L0 §12.5.11, lines 2943-3001] |

### 4.6 Decision-grade classification algorithm sketch

L5's decision-grade flow is more explicit: run preprocessing, split bundles, extract attributes, run multi-query recall, expand all plausible HS6/CN8 families into UK10 descendants, then evaluate measures and branch divergence. [L5 §1.2-§1.3, lines 56-84]

The algorithm should preserve multiple candidates until the cost/evidence stage. Greedy descent is forbidden because the highest text-retrieval score may not be the legally safest or economically material branch. [L5 §1.3, lines 63-84]

The final output may still be `review`, `rfq_required`, or `broker_required`; decision-grade does not mean always auto-pass. [L5 §4.2, lines 321-359] [L5 §6.1, lines 438-451]

## 5. Duty Computation

### 5.1 Duty types preserved

L5 explicitly requires preserving duty expressions and forbids flattening everything into `dutyPct`. Duty may be ad valorem, specific per unit, or compound. [L5 §2.5, lines 126-135]

The HMRC commodities API documents duty expressions, measure components, and compound duties. It shows specific duties with measurement units and an example compound duty of percentage plus GBP per unit/qualifier. [Trade Tariff API, lines 145-174] [Trade Tariff API, lines 359-379] [Trade Tariff API, lines 445-453]

### 5.2 Measure interaction

Measure interaction is a decision-grade requirement because AD/CVD/safeguard/quota/additional-code measures do not always stack arithmetically. Unknown additional-code eligibility defaults to the higher/no-additional-code rate. [L5 §2.4, lines 114-124]

The HMRC API guide shows that measures have type, series, geographic applicability, conditions, additional codes, and permutations; correct computation requires resolving those relationships, not just reading a headline duty. [Trade Tariff API, lines 125-183] [Trade Tariff API, lines 223-266] [Trade Tariff API, lines 566-639]

### 5.3 Trade remedy detection

Trade remedies include anti-dumping, countervailing/anti-subsidy, safeguard, quota, and related cases; L5 stores remedy state, case JSON, duty components, and a daily watcher state. [L5 §2.4, lines 114-124] [L5 §2.8, lines 159-172] [migration 0001, lines 322-392]

The public Trade Remedies Service exposes active and completed investigation pages with case number, commodity, country, type, initiated date, and updated date; no repository integration was found. [TRA public cases, lines 82-107] [code grep 2026-04-25]

### 5.4 CBAM scope

L5 treats CBAM as a nullable future placeholder, with exposure flag and estimate, because UK CBAM becomes financially relevant from 1 January 2027. [L5 §2.11, lines 209-215] [migration 0001, lines 322-392]

Current UK official factsheet scope is aluminium, cement, fertiliser, hydrogen, and iron & steel; glass and ceramics are not in scope from 2027. [UK CBAM factsheet, lines 122-130]

The current pipeline data is mostly ProContract, has zero landed-cost rows, and has only 4 candidate pricing assessments; no evidence in current DB suggests CBAM-heavy goods are an immediate blocker. [SQLite read-only query 2026-04-25]

### 5.5 Customs branch divergence

L5 says material customs branch divergence is a hard blocker: do not average plausible branches; if cost spread is material, emit `CLASSIFICATION_AMBIGUOUS`/`TARIFF_BRANCH_DIVERGENCE` style blockers. [L5 §2.10, lines 198-207] [L5 §6.1, lines 438-451]

The docs do not specify the exact `customs_cost_spread_bps` materiality threshold; L5 lists this as an open question. [L5 §6.3, lines 453-473]

### 5.6 Exporter rate basis

L5 requires exporter/manufacturer resolution because trade remedies can depend on a named exporter, annex exporter, or residual/default rate. The field values are `named_exporter`, `annex_exporter`, and `residual_default`, with residual as the conservative default. [L5 §2.3, lines 96-112] [migration 0001, lines 322-392]

### 5.7 Duty computation sequence

| Step | L1-L3 screening | L5 decision-grade |
|---|---|---|
| Commodity branch | One selected/default branch or blocker. [L0 §12.5.11, lines 2943-3001] | Multiple plausible UK10 branches until divergence resolved. [L5 §1.3, lines 63-84] |
| Valuation basis | Supplier cost plus conservative freight/boundary assumptions. [L0 §12.5.11, lines 2943-3001] | Explicit `customs_valuation_method` and `valuation_mode`. [L5 §2.7, lines 150-157] |
| Measure retrieval | Tariff duty/VAT lookup. [Trade Tariff API, lines 125-183] | Duty, VAT, quotas, conditions, additional codes, remedy measures. [L5 §2.4, lines 114-124] |
| Expression evaluation | Prefer ad valorem/simpler branch where possible; block if units missing. [L0 §12.5.11, lines 2943-3001] | Preserve ad valorem/specific/compound expressions in JSON. [L5 §2.5, lines 126-135] |
| Remedy handling | Residual/conservative default if unresolved. [L0 §12.5.11, lines 2943-3001] | Named/annex/residual exporter basis and case JSON. [L5 §2.3, lines 96-112] |
| Output | Cost components, warnings, blockers, screening/review grade. [migration 0001, lines 322-392] | Branch spread, freshness, confidence breakdown, RFQ/broker grades. [L5 §4.2, lines 321-359] |

### 5.8 Supplementary units and blockers

Specific and compound duties can depend on non-price quantities such as kilograms, litres, square metres, or pieces. HMRC examples include measurement units and qualifiers in duty expressions. [Trade Tariff API, lines 343-379] [Trade Tariff API, lines 445-453]

L5 treats missing supplementary units as a blocker because flattening a specific duty into a percentage can materially understate cost. [L5 §2.5, lines 126-135] [L5 §6.1, lines 438-451]

## 6. FX Rate Engine

### 6.1 Dual FX model

The decision-grade model uses two FX rates: HMRC monthly customs FX for customs valuation and Xe commercial spot for economic/commercial cost. The basis delta is stored in `fx_basis_delta_bps`. [L5 §3.1, lines 240-251] [migration 0001, lines 322-392]

HMRC publishes monthly customs exchange rates on the penultimate Thursday, applying to the following calendar month, with CSV/XML downloads. [HMRC exchange rates, lines 36-45]

Xe's Currency Data API is the documented commercial FX source in research/L5; Xe describes real-time exchange-rate data from many sources and package/request limits. [research §4.6, lines 1604-1734] [L5 §3.1, lines 240-251] [Xe API, lines 25-27] [Xe pricing, lines 84-97]

### 6.2 Currency flow

The intended flow is supplier price in quote currency -> GBP customs valuation using HMRC customs FX -> GBP commercial/economic cost using commercial FX -> basis delta recorded as risk/variance. [L5 §3.1, lines 240-251] [research §4.6, lines 1604-1734]

### 6.3 L1-L3 vs L5

L0 and layer-strategy describe screening-grade as single FX; L5 upgrades it to dual FX. This is a documented layer difference, although L0 Stage 11 prose also mentions HMRC/Xe in the cost formula, creating mild ambiguity for the L1-L3 build. [L0 §2.4, lines 215-220] [L0 §12.5.11, lines 2943-3001] [L5 §3.1, lines 240-251]

### 6.4 FX cache and recalculation expectations

HMRC customs FX is monthly, so a cache keyed by currency/month is enough for the customs side. HMRC says rates are published on the penultimate Thursday and apply for the following calendar month. [HMRC exchange rates, lines 36-45]

L1 says FX refresh can stale/re-run downstream from landed cost; L5 says monthly rollover should trigger recalculation. [L1 §6.5, lines 1649-1672] [L5 §3.1, lines 240-251]

Commercial FX in L5 should be treated as a separate freshness vector because it may move daily/intraday while customs FX is fixed monthly. [L5 §2.8, lines 159-172] [L5 §3.1, lines 240-251]

### 6.5 FX open implementation choice

The docs select Xe for commercial FX, but no code exists and no user account/API key is visible in the repo. A build agent should avoid hardcoding Xe until credentials/config policy is defined. [research §4.6, lines 1604-1734] [L5 §3.1, lines 240-251] [code grep 2026-04-25]

## 7. Freight Engine

### 7.1 L1-L3 screening-grade

L1-L3 freight is flat per mode and screening-grade, not a live freight quote. L0 says flat freight plus single FX; L2 says landed cost remains screening-grade flat freight/single FX until L5. [L0 §2.4, lines 215-220] [L2 §4, lines 1304-1311]

The original research warns not to use HMRC incidental expense simplifications as commercial freight quotes; those simplifications are for VAT valuation only. [research §2.4.3, lines 1168-1190]

### 7.2 L5 decision-grade

L5 adds logistics modules and clients for Freightos and SeaRates, dual-source cross-checking, unknown packaging dual scenarios, and no silent Incoterm boundary assumptions. [L5 §3.2, lines 262-276] [L5 §4.2, lines 321-359]

Freightos Price Stats API endpoint is `https://api.freightos.com/fd_external_apis/price_stats/`, with a documented 10 requests/minute limit, 2-16 second typical response, route quotas, and subscription routes. [Freightos API, lines 31-36]

SeaRates Logistics Explorer API says it displays best rates for all modes and uses bearer-token authentication supplied through a manager/API key flow. [SeaRates Logistics Explorer, lines 710-725]

### 7.3 Lead time / transit time

Supplier quote lead time is part of the quote-side data model, not the landed-cost table; Stage 11 can consume it but cannot store it directly in `landed_cost_estimates` without schema change. [research §5.5.2, lines 3600-3636] [migration 0001, lines 322-392]

SeaRates Distance & Time API can estimate route duration/distance by transport mode through `https://distanceandtime.searates.com`, with API-key query authentication and a `get/v3/api` distance/transit-time endpoint. [SeaRates Distance & Time, lines 712-743] [SeaRates Distance & Time, lines 754-787]

### 7.4 Incoterms and boundary scenarios

The research doc says Incoterms affect which costs are inside supplier price versus buyer cost; hard mode locks and boundary checks are required so freight/duty is not counted twice or omitted. [research §2.4.4, lines 1192-1200]

L0's conservative defaults include Incoterm boundary scenarios, meaning uncertainty should produce a range or warning rather than a silent assumption that the supplier bears all costs. [L0 §12.5.11, lines 2943-3001]

### 7.5 Flat freight table requirements for L1-L3

The docs do not provide actual flat rates. They only define the strategy: flat freight for screening, then Freightos/SeaRates in L5. [L0 §2.4, lines 215-220] [L5 §3.2, lines 262-276]

A build handoff should therefore require user-approved rates or a documented internal calibration table. Using HMRC incidental rates as freight quotes would conflict with the research warning. [research §2.4.3, lines 1168-1190]

## 8. VAT Treatment

### 8.1 VAT rate determination (separate from recovery)

L5 separates VAT rate determination from VAT recovery. VAT rate state covers standard/reduced/zero/exempt/unknown style classification; recovery rate covers the buyer/importer's recoverability. [L5 §2.6, lines 139-148] [migration 0001, lines 322-392]

### 8.2 VAT recovery model

The schema stores `import_vat_treatment` and `vat_recovery_rate_bps`; the fixture writes `recoverable`, but production rules for the platform's importer/customer VAT profile are not implemented. [migration 0001, lines 322-392] [main.rs:7450-7523] [code grep 2026-04-25]

### 8.3 National additional codes

L5 notes national additional-code families 6/17 for VAT rate determination. The HMRC commodities API documents additional codes generally as a further subdivision of a 10-digit commodity by purpose, properties, or overseas company. [L5 §2.6, lines 139-148] [Trade Tariff API, lines 326-333]

## 9. Decision Grade Output

### 9.1 The `decision_grade` field

Docs describe decision-grade values `screening`, `review`, `rfq_required`, and `broker_required`; L5 also lists `review`, `rfq_required`, and `broker_required` as decision-grade outputs. The current fixture value `reviewable` is not in that documented set. [L5 §4.2, lines 321-359] [main.rs:7450-7523]

### 9.2 Hard blockers

L5 blocker/error codes include `CLASSIFICATION_AMBIGUOUS`, `TARIFF_BRANCH_DIVERGENCE`, `MEASURE_INTERACTION_UNKNOWN`, `EXPORTER_UNRESOLVED`, `SUPPLEMENTARY_UNIT_MISSING`, `VALUATION_MODE_CONFLICT`, `FPO_UNAVAILABLE`, `TRADE_REMEDY_WATCH_STALE`, `FX_RATE_MISSING`, and `FREIGHT_ESTIMATE_FAILED`. [L5 §6.1, lines 438-451]

L6 bid workflow readiness requires no active L5 blockers and fresh intelligence; active landed-cost blockers prevent request-pipeline automation. [L6 §1, lines 52-75] [L6 §7, lines 286-294]

### 9.3 Confidence aggregation

L0 defines a confidence model with evidence weights and coverage metrics, including landed-cost coverage and value-weighted landed-cost coverage. Stage 11 should contribute `confidence_bps` and `confidence_breakdown` instead of a binary pass/fail. [L0 §12.6, lines 3058-3101] [L2 §5, lines 1629-1640] [packages/intelligence/src/margins/coverage-rollup.ts:3-151]

### 9.4 Downstream behavior expected by Stage 12

Stage 12 computes net margin from landed cost and expected revenue. It should treat active blockers, stale intelligence, or low confidence as reasons to withhold automation-grade approval even if a numeric margin is positive. [L0 §12.5.12, lines 3002-3050] [L6 §1, lines 52-75]

The current codebase has no production Stage 12 run wired after landed cost; any Stage 11 build that writes rows will still need Stage 12 integration before the full gross -> landed -> net chain is live. [main.rs:9840-9870] [L0 §12.5.12, lines 3002-3050]

## 10. External Dependencies (APIs)

| API | URL | Auth | Rate limit | Cost | Used for | L1-L3 / L5 | Integration status |
|---|---|---|---|---|---|---|---|
| HMRC Trade Tariff API | `https://www.trade-tariff.service.gov.uk/api/`; commodities URL shape `https://www.trade-tariff.service.gov.uk/uk/api/commodities/{10-digit-commodity}` | Docs say Trade Tariff APIs do not require authentication. | Commodities API rate limiting begins September 2026; tariff docs recommend 24-hour cache. | Free/public under GOV.UK terms. | Commodity lookup, duty, VAT, measures, quotas, trade measures. | L1-L3 and L5. | Zero production code. [Trade Tariff API About, lines 35-80] [Trade Tariff API, lines 50-63] [code grep 2026-04-25] |
| HMRC FPO Commodity Code Identification Tool | `https://search.trade-tariff.service.gov.uk/fpo-code-search` | `X-Api-Key`; restricted to invited/eligible FPO/UKC organisations. | Docs state 100 requests/second per FPO for FPO API. | Free-to-use for eligible UK Carrier scheme registered organisations. | HS6/CN8 suggestions from English descriptions. | L5 recall source; optional/blocked for L1-L3. | Zero production code; fixture warning only. [FPO API docs, lines 35-66] [FPO API docs, lines 69-115] [main.rs:7195-7196] |
| HMRC monthly customs FX | `https://www.trade-tariff.service.gov.uk/exchange_rates/monthly` | Public page/download. | Monthly publication; no API rate limit found in docs. | Free. | Customs valuation FX. | L1-L3 single FX candidate; L5 customs side of dual FX. | Zero production code. [HMRC exchange rates, lines 36-45] [code grep 2026-04-25] |
| Xe Currency Data API | `https://www.xe.com/en-us/platform/xecurrencydata/` | Paid account/API credentials. | Package request limits; example Lite 10,000 requests/month. | Paid; Lite shown at $799/year; higher tiers and add-ons exist. | Commercial spot FX, historical rates, monthly averages, volatility. | L5. | Zero production code. [Xe API, lines 84-97] [Xe pricing help, lines 25-39] [code grep 2026-04-25] |
| Freightos Price Stats | `https://api.freightos.com/fd_external_apis/price_stats/` | API credentials/subscription; related Freightos docs show API key/secret-key headers for Freightos data APIs. | 10 requests/minute on Price Stats; route quota per subscription. | Paid/subscription; public pricing shows paid plans/custom enterprise packages. | Freight price benchmarks by route/mode/load. | L5. | Zero production code. [Freightos API, lines 31-36] [Freightos pricing, lines 375-383] [Freightos terms, lines 156-176] [code grep 2026-04-25] |
| SeaRates Logistics Explorer | `https://docs.searates.com/reference/logistics/introduction`; token URL pattern `https://www.searates.com/auth/platform-token?id=XXXX&api_key=XXXX` | Bearer token. | Public page references customer support/subscription; specific rate limit not found for Logistics Explorer page. | Paid/commercial; docs say token comes from manager/API key flow. | Best rates across modes, logistics rate estimates. | L5. | Zero production code. [SeaRates Logistics Explorer, lines 710-725] [code grep 2026-04-25] |
| SeaRates Distance & Time | `https://distanceandtime.searates.com` | Query parameter `api_key`; docs say contact to purchase subscription if no API key. | Not found on opened page. | Paid/subscription. | Route distance and estimated delivery time by mode. | L5, possibly future delivery-time enrichment. | Zero production code. [SeaRates Distance & Time, lines 712-752] [code grep 2026-04-25] |
| TRA public case file | `https://www.trade-remedies.service.gov.uk/public/cases/` | Public website for case files; account needed to take part/register interest. | No formal API/rate limit found. | Free public page. | Trade-remedy watcher and case evidence. | L5. | Zero production code. [TRA public cases, lines 82-107] [research §4.5, lines 1543-1557] [code grep 2026-04-25] |
| UK CBAM official guidance | GOV.UK CBAM factsheet/policy pages | Public. | Not an API in current docs. | Free public guidance; tax begins 2027. | CBAM exposure flag and future estimate. | L5 future/nullable. | Zero production code. [UK CBAM factsheet, lines 122-149] [code grep 2026-04-25] |

**Cost summary for budgeting:** L1-L3 can be built with public HMRC tariff and HMRC monthly FX plus internal flat freight assumptions, so ongoing API cost can be zero; L5 adds Xe, Freightos, and SeaRates, which are paid/subscription or account-gated services. [Trade Tariff API About, lines 74-80] [HMRC exchange rates, lines 36-45] [Xe API, lines 84-97] [Freightos API, lines 31-36] [SeaRates Logistics Explorer, lines 710-725]

## 10A. DPS / Service-Heavy Data Reality Check

The current DB has 133 `contract_notices`, all from `procontract`; 308 qualification runs split across unknown, services, goods, and works categories; 10 supplier quotes; 4 pricing assessments; and 0 landed-cost estimates. [SQLite read-only query 2026-04-25]

The user's stated context says the pipeline is ProContract-heavy and DPS/service-heavy; the current DB supports ProContract-only and service/unknown heaviness, but a simple keyword scan found 26/133 notices with DPS/dynamic wording and 6/25 goods-qualified processes with DPS/dynamic wording. That means the exact "~90% qualified contracts are DPS" claim is not directly represented in the current SQLite fields inspected here. [SQLite read-only query 2026-04-25]

Implication: A pragmatic L1-L3 landed-cost implementation may be enough for the current dataset because there are few priced candidates and no landed-cost consumers beyond coverage/desktop display. [SQLite read-only query 2026-04-25] [packages/intelligence/src/margins/coverage-rollup.ts:3-151] [apps/desktop/src-tauri/src/query_service.rs:506-516]

L5 features most likely to be deferred for current data are CBAM, exporter-specific remedy rates, complex quota/additional-code interactions, dual freight API cross-checks, and daily TRA watchers, unless new direct-order goods contracts in remedy/CBAM-heavy categories enter the pipeline. [L5 §2.3-§2.11, lines 96-215] [UK CBAM factsheet, lines 122-130] [SQLite read-only query 2026-04-25]

## 10B. External dependency notes

### 10B.1 HMRC Trade Tariff API

The official docs say Trade Tariff APIs are exposed under `https://www.trade-tariff.service.gov.uk/api/` and currently do not require authentication. [Trade Tariff API About, lines 35-59]

The docs distinguish UK and XI endpoints; this platform's current UK public procurement/import assumption points to the UK endpoint unless Northern Ireland/XI routing becomes explicit in product data. [Trade Tariff API About, lines 35-45]

The commodities API supports retrieving a 10-digit declarable commodity and included entities such as measures, geographical areas, footnotes, quotas, additional codes, and measure conditions. [Trade Tariff API, lines 50-63] [Trade Tariff API, lines 125-183]

### 10B.2 HMRC FPO

FPO is a Commodity Code Identification Tool that returns recommended commodity codes for English descriptions, but access is restricted: docs say it is only available to invited or eligible FPO organisations and requires `X-Api-Key`. [FPO API docs, lines 35-66] [FPO API docs, lines 69-115]

This means a future build should not make FPO a hard dependency for L1-L3 unless credentials and eligibility are confirmed. [FPO API docs, lines 35-66] [L5 §1.1, lines 40-54]

### 10B.3 Freightos

The original research preferred Freightos as a freight benchmark while also warning the public API status had changed. The current opened API docs show Price Stats still has an endpoint and documented limits, but access is subscription/credential-bound. [research §2.4.1, lines 1039-1075] [Freightos API, lines 31-36]

Freightos data terms require an API key and secret key for access to Freightos Data APIs. [Freightos terms, lines 156-176]

### 10B.4 SeaRates

SeaRates Logistics Explorer covers rate discovery, while SeaRates Distance & Time covers route duration/distance. These are different products, so L5 freight cost and delivery-time estimates may need two integrations if both values are required. [SeaRates Logistics Explorer, lines 710-725] [SeaRates Distance & Time, lines 712-787]

The current landed-cost schema can store freight cost scenarios but not transit-time outputs, which reinforces the schema/open-question split for delivery time. [migration 0001, lines 322-392]

## 11. Current Implementation Status

### 11.1 What exists

- `landed_cost_estimates` schema exists in the initial SQLite migration with the full nullable L5-ready column set. [migration 0001, lines 322-392]
- Fixture materialization in `crates/daemon/src/main.rs` inserts one hardcoded landed-cost row for test/harness paths. [main.rs:7307-7523]
- Stage run IDs include landed cost and net margin fields, but this is structural support rather than execution. [main.rs:11571-11587] [main.rs:11738-11750]
- Desktop query service can read a small view model from `landed_cost_estimates`: estimate id, candidate id, quote id, economic landed cost, net margin base, decision grade, and confidence. [apps/desktop/src-tauri/src/query_service.rs:506-516] [apps/desktop/src-tauri/src/query_service.rs:1185-1215]
- TypeScript intelligence coverage rollup consumes only a boolean `hasLandedCost`, not detailed landed-cost columns. [packages/intelligence/src/margins/coverage-rollup.ts:3-151]
- Tests insert/read landed-cost fixture rows in daemon and desktop test surfaces. [crates/daemon/tests/agent_procurement_ingestion.rs:796-819] [crates/daemon/tests/agent_supplier_enrichment.rs:70-112] [apps/desktop/src-tauri/tests/query_service.rs:218-231]

### 11.2 What is not built for L1-L3 (screening-grade)

- No production `run_landed_cost_stage()` function was found. [main.rs:9840-9870] [code grep 2026-04-25]
- `run_deterministic_pipeline()` does not call landed cost or net margin; it currently runs extraction/qualification/quoteability/gross-margin/relation stages. [main.rs:9840-9870]
- No HMRC Trade Tariff client exists in `crates/`, `packages/`, or `apps/`. [code grep 2026-04-25]
- No freight client or flat-rate table exists for Stage 11. [code grep 2026-04-25]
- No FX fetcher/cache exists for HMRC monthly FX or Xe. [code grep 2026-04-25]
- No commodity classification implementation exists beyond fixture warning text and schema fields. [main.rs:7195-7196] [code grep 2026-04-25]
- No production VAT computation exists for landed cost. [code grep 2026-04-25]
- No `product_commodity_codes` table exists despite docs describing it. [SQLite read-only query 2026-04-25] [L0 §4.4, lines 1397-1411]

### 11.3 What is deferred to L5 (decision-grade)

The L5-deferred set includes multi-source classification recall, all plausible UK10 family expansion, measure interaction, trade remedy daily watcher, CBAM placeholder/estimate, dual FX engine, Freightos + SeaRates freight APIs, exporter-rate basis resolution, supplementary unit enforcement, decision-grade scenarios, and customs branch divergence. [L5 §1-§4, lines 40-359]

### 11.4 Fixture-materialization row shape

The fixture row in `main.rs` is useful as a column-population example, but it is not evidence that Stage 11 runs in production. It is inside fixture materialization code and uses hardcoded values. [main.rs:7307-7523]

| Fixture field/value | Interpretation |
|---|---|
| `shipping_mode = parcel`, `mode_confidence_bps = 7200` | Demonstrates shipping-mode columns can be populated. [main.rs:7450-7523] |
| `declarable_commodity_code_10 = 1234567890` | Placeholder declarable code, not a real classification integration. [main.rs:7450-7523] |
| `customs_valuation_method = transaction_value`, `valuation_mode = standard` | Shows valuation columns, but `standard` conflicts with L5 mode vocabulary. [main.rs:7450-7523] [L5 §2.7, lines 150-157] |
| `customs_fx_rate = 1.0`, `commercial_fx_rate = 1.0` | Placeholder rates; no FX fetching exists. [main.rs:7450-7523] [code grep 2026-04-25] |
| `freight_low/base/high = 200/400/600` | Scenario shape only; no flat freight table. [main.rs:7450-7523] |
| `customs_duty = 300`, `import_vat = 200`, `trade_remedy_duty = 0` | Demonstrates cost columns, not real duty computation. [main.rs:7450-7523] |
| `decision_grade = reviewable` | Fixture-only vocabulary issue. [main.rs:7450-7523] [L5 §4.2, lines 321-359] |
| `confidence_bps = 7600`, `warnings = []`, `blockers = []` | Demonstrates output shape. [main.rs:7450-7523] |

### 11.5 Codebase surface map

| Surface | File | What it does |
|---|---|---|
| Migration schema | `migrations/0001_sqlite_state_initial.sql` | Defines `landed_cost_estimates`; no `product_commodity_codes`. [migration 0001, lines 322-392] |
| Fixture writer | `crates/daemon/src/main.rs` | Inserts hardcoded candidate/quote/gross/landed-cost rows in fixture materialization. [main.rs:7307-7523] |
| Deterministic pipeline | `crates/daemon/src/main.rs` | Currently does not run landed cost or net margin. [main.rs:9840-9870] |
| Gross margin producer | `crates/daemon/src/main.rs` | Produces Stage 10 `candidate_pricing_assessments`. [main.rs:11178-11325] |
| Stage ID struct | `crates/daemon/src/main.rs` | Includes landed-cost/net-margin IDs without executing Stage 11. [main.rs:11571-11587] [main.rs:11738-11750] |
| Desktop query | `apps/desktop/src-tauri/src/query_service.rs` | Reads compact landed-cost VM. [apps/desktop/src-tauri/src/query_service.rs:506-516] [apps/desktop/src-tauri/src/query_service.rs:1185-1215] |
| Desktop tests | `apps/desktop/src-tauri/tests/query_service.rs` | Inserts fixture landed rows for UI query tests. [apps/desktop/src-tauri/tests/query_service.rs:218-231] |
| TS coverage | `packages/intelligence/src/margins/coverage-rollup.ts` | Rolls up `hasLandedCost` coverage. [packages/intelligence/src/margins/coverage-rollup.ts:3-151] |

### 11.6 Live database snapshot

The read-only DB snapshot on 2026-04-25 showed `landed_cost_estimates rows: 0`, `product_commodity_codes table exists? 0`, `candidate_pricing_assessments rows: 4`, and `supplier_quotes rows: 10`. [SQLite read-only query 2026-04-25]

The same snapshot showed all 133 current notices came from `procontract`, with qualification-run legal categories weighted toward unknown/services more than goods. [SQLite read-only query 2026-04-25]

This means Stage 11 would initially have very few real candidates to process; that is a product prioritization issue, not a code blocker. [SQLite read-only query 2026-04-25]

### 11.7 Expected production insertion point

The natural insertion point is after `run_gross_margin_stage` and before any future net-margin stage in `run_deterministic_pipeline()`. Current code runs gross margin and then relation logic; landed cost is absent. [main.rs:9840-9870] [main.rs:11178-11325]

Future production code should use Stage 10's selected quote and `selected_quote_row_version` style provenance rather than scanning all quotes again. [L0 §12.5.10, lines 2888-2941] [main.rs:11178-11325]

## 12. Build Roadmap (recommendations only - not a build plan)

### 12.1 L1-L3 minimum-viable Stage 11

| Item | Effort | Depends on | Unlocks |
|---|---:|---|---|
| Define Stage 11 input contract from `candidate_pricing_assessments` and selected quote | S | Stage 10 schema/code | Prevents recomputing gross stage. [L0 §12.5.10, lines 2888-2941] |
| HMRC Trade Tariff API client | M | HTTP/config/cache conventions | Real duty/VAT/measure lookup. [Trade Tariff API, lines 50-63] |
| Screening commodity-code resolver | M | Tariff client, product text | Declarable or conservative branch selection. [L0 §12.5.11, lines 2943-3001] |
| Flat freight table and scenario bands | S | Mode/packing assumptions | Freight low/base/high. [L0 §2.4, lines 215-220] |
| HMRC monthly FX fetcher/cache | S | HTTP/cache | GBP conversion for supplier/customs. [HMRC exchange rates, lines 36-45] |
| Standard VAT computation | S | Tariff/VAT default | VAT line item and cash-to-clear. [L5 §2.6, lines 139-148] |
| Conservative defaults/blockers | S | All above | Safety under uncertainty. [L5 §2.9, lines 174-196] |
| `run_landed_cost_stage()` deterministic writer | M | All above | Creates real rows. [main.rs:9840-9870] |
| Wire into `run_deterministic_pipeline()` after gross margin | S | Stage function | End-to-end Stage 11 execution. [layer-strategy, lines 1276-1291] |
| Desktop/API surface verification | S | Rows exist | Operator can see cost/margin. [apps/desktop/src-tauri/src/query_service.rs:506-516] |

**Total rough estimate (L1-L3 minimum):** 4-7 person-days and roughly 5-7 Rust modules/helpers, assuming no schema migration for `product_commodity_codes`. This estimate is inferred from the missing surfaces and scope size, not directly stated in docs. **[Unsourced - inferred]**

### 12.2 L5 decision-grade upgrades

| Item | Effort | Notes |
|---|---:|---|
| Multi-source classification recall | L | Five recall sources; FPO restricted. [L5 §1.1, lines 40-54] |
| Multi-family HS6/CN8/UK10 expansion | M | Requires cached HMRC graph and non-greedy descendants. [L5 §1.3, lines 63-84] |
| Measure interaction engine | L | Conditions, additional codes, quotas, AD/CVD/safeguards. [L5 §2.4, lines 114-124] |
| Duty expression preservation | M | Preserve ad valorem/specific/compound. [L5 §2.5, lines 126-135] |
| Trade remedy watcher | M | TRA/public case monitoring and freshness. [L5 §2.8, lines 159-172] |
| CBAM exposure/estimate | S initially, M later | Nullable until 2027 liabilities mature. [L5 §2.11, lines 209-215] |
| Dual FX engine | S-M | HMRC customs + Xe commercial. [L5 §3.1, lines 240-251] |
| Freightos + SeaRates logistics | M | Dual-source rate scenarios. [L5 §3.2, lines 262-276] |
| Exporter-rate basis resolution | M | Named/annex/residual. [L5 §2.3, lines 96-112] |
| Supplementary-unit enforcement | S-M | Block if required units missing. [L5 §2.5, lines 126-135] |
| Decision-grade output/confidence | M | Grade, blocker, freshness, confidence breakdown. [L5 §4.2, lines 321-359] |

**Total rough estimate (L5 decision-grade):** 15-25 person-days after L1-L3 exists. This estimate is inferred from the L5 file list, integration count, and test matrix. **[Unsourced - inferred]** [L5 §3.2, lines 226-276] [L5 §5, lines 380-418]

**Recommendation framework for the user:**

- If the goal is to unblock Stage 12/operator net-margin display, ship L1-L3 only. [L0 §12.5.12, lines 3002-3050]
- If the goal is production-quality bid pricing on complex imported goods, L5 is required. [L5 §0, lines 15-38]
- If the current data reality remains service-heavy/DPS-heavy with few priced goods candidates, defer most L5 features until contract mix justifies them. [SQLite read-only query 2026-04-25] [L5 §6.3, lines 453-473]

### 12.3 Suggested verification strategy for a future build

| Verification | Layer | Why |
|---|---|---|
| Unit tests for tariff response parsing | L1-L3 | Duty/VAT parsing is core to landed-cost correctness. [Trade Tariff API, lines 125-183] |
| Fixture replacement test proving real Stage 11 writes rows | L1-L3 | Current rows are fixture-only. [main.rs:7307-7523] [main.rs:9840-9870] |
| Conservative-default tests | L1-L3 and L5 | Defaults are constitutional behavior. [L0 §12.5.11, lines 2943-3001] [L5 §2.9, lines 174-196] |
| Compound/specific duty tests | L5 | L5 forbids flattening duty expressions. [L5 §2.5, lines 126-135] |
| Branch divergence tests | L5 | Material classification divergence must block. [L5 §2.10, lines 198-207] |
| FX rollover test | L5 | HMRC monthly rollover should recalculate. [L5 §3.1, lines 240-251] |
| Freight API failure tests | L5 | `FREIGHT_ESTIMATE_FAILED` is an L5 error. [L5 §6.1, lines 438-451] |

### 12.4 Minimal no-paid-API path

The lowest-cost path is HMRC Trade Tariff API, HMRC monthly exchange rates, and an internal flat freight table. This matches L1-L3 screening-grade scope and avoids Xe/Freightos/SeaRates account setup. [L0 §2.4, lines 215-220] [Trade Tariff API About, lines 35-80] [HMRC exchange rates, lines 36-45]

This path should still mark output as screening/review rather than decision-grade and should preserve blockers for low-confidence classification, missing units, or tariff ambiguity. [L5 §4.2, lines 321-359] [L5 §6.1, lines 438-451]

### 12.5 Hybrid path

A practical hybrid is: ship L1-L3 now, add schema/design cleanup for commodity candidates, and defer only the paid/commercial L5 APIs until the pipeline has enough direct goods candidates. This recommendation is inferred from current DB scarcity and the layer split. **[Unsourced - inferred]** [SQLite read-only query 2026-04-25] [L5 §0, lines 15-38]

The highest-leverage early L5 feature may be multi-candidate classification and branch divergence, because that protects against catastrophic underestimation even without paid freight APIs. [L5 §1.3, lines 63-84] [L5 §2.10, lines 198-207]

### 12.6 L1-L3 acceptance criteria

| Criterion | Why it matters | Source |
|---|---|---|
| A real pipeline run writes at least one non-fixture `landed_cost_estimates` row when a selected quote exists. | Current rows are fixture-only and DB count is zero. | [main.rs:7307-7523] [SQLite read-only query 2026-04-25] |
| Stage 11 runs after Stage 10 and before Stage 12/net-margin logic. | This is the documented stage order and boundary. | [layer-strategy, lines 1276-1291] |
| The stage never recomputes quote selection. | Stage 10 owns selected quote/gross margin. | [L0 §12.5.10, lines 2888-2941] |
| Screening-grade output populates supplier cost, freight low/base/high, customs duty, VAT, final mile, other fees, economic landed cost, cash outlay, net margin band, confidence, warnings, and blockers. | These are the L0/L1 fields needed downstream. | [migration 0001, lines 322-392] |
| Commodity classification uncertainty produces warnings/blockers, not silent low-cost assumptions. | Conservative defaults are constitutional. | [L0 §12.5.11, lines 2943-3001] [L5 §2.9, lines 174-196] |
| Missing supplementary units do not produce a fake precise duty. | Specific/compound duty can depend on units. | [L5 §2.5, lines 126-135] |
| Freight uses a documented flat table, not HMRC incidental VAT simplifications. | HMRC incidental rates are not commercial freight quotes. | [research §2.4.3, lines 1168-1190] |
| FX source and timestamp are stored. | FX freshness can rerun landed cost onward. | [L1 §6.5, lines 1649-1672] |
| `decision_grade` uses the documented vocabulary, not fixture-only `reviewable`. | Avoids production/test vocabulary drift. | [L5 §4.2, lines 321-359] [main.rs:7450-7523] |
| Desktop query surface can display the produced row. | The existing desktop view model already expects a row. | [apps/desktop/src-tauri/src/query_service.rs:506-516] |

### 12.7 L5 acceptance criteria

| Criterion | Why it matters | Source |
|---|---|---|
| Classification recall uses at least two independent sources before confidence rises above screening level. | L5 requires multi-source recall. | [L5 §1.1, lines 40-54] |
| HS6/CN8 candidates expand to all plausible declarable UK10 descendants before pruning. | L5 forbids greedy descent. | [L5 §1.3, lines 63-84] |
| Duty expressions are stored as structured JSON and preserve specific/compound components. | L5 forbids flattening into `dutyPct`. | [L5 §2.5, lines 126-135] |
| Exporter-rate basis is explicit. | Named/annex/residual rates can change economics. | [L5 §2.3, lines 96-112] |
| Trade-remedy watcher freshness is visible. | L5 has remedy freshness vector and L6 gates. | [L5 §2.8, lines 159-172] [L6 §1, lines 52-75] |
| Customs branch spread can hard-block. | L5 requires branch divergence blocker. | [L5 §2.10, lines 198-207] |
| Dual FX is recorded and basis delta computed. | L5 requires customs FX and commercial FX separation. | [L5 §3.1, lines 240-251] |
| Freightos and SeaRates failures produce explicit warning/blocker states. | L5 lists freight failure error. | [L5 §6.1, lines 438-451] |
| CBAM exposure is flagged without forcing nonzero estimate before policy maturity. | CBAM is nullable/future in schema. | [L5 §2.11, lines 209-215] |
| Bid workflow cannot start with active L5 blockers. | L6 readiness gate. | [L6 §1, lines 52-75] |

### 12.8 No-build/defer option

The evidence supports a defensible defer option if the immediate project goal is not Stage 12/net-margin display. The current DB has zero landed-cost rows, only four pricing assessments, and a ProContract-heavy/service-heavy current shape. [SQLite read-only query 2026-04-25]

If Stage 11 is deferred, the next highest-leverage preparatory work is likely pricing coverage, explicit DPS/direct-order classification, and schema decision cleanup for `product_commodity_codes`. This is a prioritization inference rather than a documented roadmap. **[Unsourced - inferred]** [SQLite read-only query 2026-04-25] [L0 §4.4, lines 1397-1411]

Deferral should still record that Stage 12 and L6 automation cannot be production-credible without landed cost, because net margin and bid readiness depend on Stage 11 outputs and blockers. [L0 §12.5.12, lines 3002-3050] [L6 §1, lines 52-75]

### 12.9 Build sequencing cautions

Do not start with Freightos/SeaRates before the product has enough priced goods candidates; freight APIs improve precision but do not solve missing quote/product volume. [SQLite read-only query 2026-04-25] [L5 §3.2, lines 262-276]

Do not start with FPO as the only classifier unless access is confirmed; official docs restrict access and require `X-Api-Key`. [FPO API docs, lines 35-66]

Do not encode `product_commodity_codes` assumptions in Rust until the schema contradiction is resolved. [L0 §4.4, lines 1397-1411] [migration 0001, lines 322-392]

Do not use fixture values as truth for enums. The fixture is a useful shape example but contains vocabulary that differs from current strategy docs. [main.rs:7450-7523] [L5 §2.7, lines 150-157] [L5 §4.2, lines 321-359]

### 12.10 Risk register for future build agents

| Risk | Impact | Mitigation |
|---|---|---|
| Underestimated duty because classification picked one low-cost branch too early. | False positive margin and bad bid decision. | Preserve candidates and block on material divergence. [L5 §1.3, lines 63-84] [L5 §2.10, lines 198-207] |
| Paid API work starts before data volume justifies it. | Time/cost spent on precision without candidates. | Ship public-source screening first or defer Stage 11. [SQLite read-only query 2026-04-25] |
| Schema drift between docs and migration is ignored. | Build blocks on missing `product_commodity_codes` or enum mismatch. | Resolve schema/vocabulary decisions before L5. [L0 §4.4, lines 1397-1411] [migration 0001, lines 322-392] |
| VAT recovery assumptions are silently wrong. | Cash/economic cost is misleading. | Make VAT treatment explicit and default conservative. [L5 §2.6, lines 139-148] |
| Freight/Incoterm boundary is double-counted or omitted. | Landed cost range is distorted. | Store warnings and model boundary scenarios. [research §2.4.4, lines 1192-1200] |

## 13. Open Questions

1. Should Stage 11 be built now when the current DB has 4 pricing assessments, 10 supplier quotes, and 0 landed-cost rows, or should effort first improve pricing/candidate volume? [SQLite read-only query 2026-04-25]
2. Should the missing `product_commodity_codes` table be added before any Stage 11 build, or should L1-L3 rely on `candidate_commodity_codes_json` and defer the table gap to L5? [L0 §4.4, lines 1397-1411] [migration 0001, lines 322-392]
3. What exact `customs_cost_spread_bps` threshold makes branch divergence material? L5 names the blocker but leaves the threshold open. [L5 §2.10, lines 198-207] [L5 §6.3, lines 453-473]
4. What is the canonical `decision_grade` vocabulary for production writes: `screening/review/rfq_required/broker_required`, or should fixture `reviewable` be migrated/ignored? [L5 §4.2, lines 321-359] [main.rs:7450-7523]
5. What are the exact L1-L3 `valuation_mode` values? L5 describes `1_proxy` and `1_actual`, while the fixture writes `standard`. [L5 §2.7, lines 150-157] [main.rs:7450-7523]
6. Does the user want FPO access pursued, given official docs restrict it to invited/eligible FPO/UKC organisations with `X-Api-Key`? [FPO API docs, lines 35-66]
7. What VAT recovery assumption should be used for the platform's customer/importer profile: fully recoverable, partially recoverable, nonrecoverable, or postponed VAT accounting? [L5 §2.6, lines 139-148] [main.rs:7450-7523]
8. What flat freight bands should be used for L1-L3 parcel/pallet/container, since docs specify the model but not the actual rates? [L0 §2.4, lines 215-220] [research §2.4.3, lines 1168-1190]
9. Should lead/transit time remain quote/logistics metadata outside `landed_cost_estimates`, or should a future migration add explicit transit-time columns? [research §5.5.2, lines 3600-3636] [migration 0001, lines 322-392]
10. Should L1-L3 use only public HMRC sources to avoid paid API cost, or should any commercial API be introduced earlier for user confidence? [Trade Tariff API About, lines 74-80] [HMRC exchange rates, lines 36-45] [Xe API, lines 84-97]
11. The current DB keyword scan does not confirm the user's "~90% qualified contracts are DPS" statement; should the platform add explicit DPS classification fields before using that assumption for Stage 11 prioritization? [SQLite read-only query 2026-04-25]
12. Are the rough effort estimates in this document acceptable as planning estimates, or should they be replaced by a separate task-by-task implementation plan? **[Unsourced - inferred]**
13. Should the L1-L3 screening algorithm be formally specified before build, or is the inferred HMRC+flat-freight+single-FX algorithm enough for an implementation handoff? **[Unsourced - inferred from cited docs]** [L0 §12.5.11, lines 2943-3001]
14. Is the hybrid recommendation acceptable, or should paid L5 APIs be procured before any Stage 11 work begins? **[Unsourced - inferred]** [SQLite read-only query 2026-04-25] [L5 §3.2, lines 262-276]
15. If Stage 11 is deferred, which upstream metric should trigger revisiting it: number of priced goods candidates, value of qualified goods pipeline, or a specific direct-order goods notice entering the funnel? **[Unsourced - inferred]** [SQLite read-only query 2026-04-25]

## 14. References

### Local strategy docs

- `docs/strategy/layers/layer-0-gold-set-evaluation.md`
- `docs/strategy/layers/layer-1-thin-local-runner.md`
- `docs/strategy/layers/layer-2-single-operator-desktop.md`
- `docs/strategy/layers/layer-4-shared-visibility.md`
- `docs/strategy/layers/layer-5-expanded-commercial-intelligence.md`
- `docs/strategy/layers/layer-6-request-pipeline-automation.md`
- `docs/strategy/layer-strategy.md`
- `docs/strategy/cross-reference-index.md`

### Local research and historical artifacts

- `docs/research/2026-03-08-public-procurement-intelligence-platform-research.md`
- `docs/harmonymaxer/archived/run-001/harmonization-handoff.md`
- `docs/planmaxer/archived/iteration-003/handoffs/handoff-customs-classification.md`
- `docs/planmaxer/archived/iteration-003/prepreps/preprep-l5l6-commercial-coherence.md`

### Local code and schema

- `migrations/0001_sqlite_state_initial.sql`
- `crates/daemon/src/main.rs`
- `crates/daemon/tests/agent_procurement_ingestion.rs`
- `crates/daemon/tests/agent_supplier_enrichment.rs`
- `apps/desktop/src-tauri/src/query_service.rs`
- `apps/desktop/src-tauri/tests/query_service.rs`
- `packages/intelligence/src/margins/coverage-rollup.ts`
- `state/state.db` read-only SQLite queries on 2026-04-25

### External docs

- HMRC Trade Tariff API About: <https://docs.trade-tariff.service.gov.uk/>
- HMRC Trade Tariff Commodities API: <https://docs.trade-tariff.service.gov.uk/the-commodities-api.html>
- HMRC FPO API: <https://docs.trade-tariff.service.gov.uk/fpo.html>
- HMRC monthly customs exchange rates: <https://www.trade-tariff.service.gov.uk/exchange_rates/monthly>
- Xe Currency Data API: <https://www.xe.com/en-us/platform/xecurrencydata/>
- Xe Currency Data pricing/help: <https://help.xe.com/hc/en-gb/articles/4414092026769-Currency-Data-API-packages-pricing-and-payment>
- Freightos Price Stats API: <https://apidocs.freightos.com/reference/postpricestats-1>
- Freightos data terms: <https://www.freightos.com/freightos-data-terms-conditions/>
- SeaRates Logistics Explorer: <https://docs.searates.com/reference/logistics/introduction>
- SeaRates Distance & Time API: <https://docs.searates.com/reference/distance/distance-and-transit-time-calculator>
- TRA public cases: <https://www.trade-remedies.service.gov.uk/public/cases/>
- UK CBAM factsheet: <https://www.gov.uk/government/publications/factsheet-carbon-border-adjustment-mechanism-cbam/factsheet-carbon-border-adjustment-mechanism>
