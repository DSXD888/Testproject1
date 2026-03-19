# Layer 5 — Expanded Commercial Intelligence

> **Authority:** This document is the execution-grade specification for Layer 5. An implementation agent should be able to build this layer from this document alone without reopening the research document.
>
> **Milestone:** M5
> **Dependencies:** Layers 0-4 (full pipeline, sync, web portal)
> **Exit criteria:** System handles >=3 goods categories with decision-grade landed cost and >=2 supplier sources per category.

---

## 1. Overview & Customs Classification Engine

### 1.1 Layer Narrative

Broader categories, more supplier sources, decision-grade accuracy. The customs classification engine expands from screening-grade (HS6, single MFN rate, flat freight) to full multi-source recall with 10-digit resolution, measure interaction, and trade remedy monitoring. Additional marketplace skills (Alibaba/1688, Amazon) provide multi-source pricing. FX engine gains dual-rate capability (HMRC customs + commercial spot). Freight engine gains real API integrations (Freightos + SeaRates). Each new category requires its own gold-set validation before entering production.

**Why this layer exists:** Layers 0-4 proved the platform works with screening-grade intelligence on a narrow category whitelist. Layer 5 expands the commercial intelligence to decision-grade accuracy across broader categories. This is where the system becomes commercially authoritative.

**If we never build Layer 6:** The system is a mature procurement intelligence platform covering >=3 goods categories with decision-grade landed cost and >=2 supplier sources per category. Only request pipeline automation is missing.

### 1.2 Capability Delivered

- System handles >=3 goods categories with decision-grade landed cost
- >=2 supplier sources per category (eBay + Alibaba minimum; Amazon and 1688 where applicable)
- Full customs classification: multi-source recall, multi-family expansion to UK 10-digit, exporter resolution, measure interaction, trade remedy monitoring
- Dual FX rates: HMRC customs valuation + Xe commercial spot
- Real freight estimates via Freightos + SeaRates APIs
- Per-category gold-set expansion and validation

### 1.3 Data Changes

**No new tables. No new columns.** All decision-grade columns exist as nullable from Layer 0. Layer 5 populates previously-empty fields:

| Table | Columns Now Populated |
|-------|----------------------|
| `landed_cost_estimates` | Full duty breakdown, measure interaction results, exporter rate basis, supplementary unit requirements, valuation mode, customs valuation method (all existed as nullable since L0) |
| `product_candidates` | Commodity code families from multi-source recall (via `product_commodity_codes` junction, existed since L0) |
| `supplier_quotes` | Multi-source quotes: Alibaba, 1688, Amazon rows alongside existing eBay rows (same schema, more rows) |

### 1.4 Multi-Source HS Classification Recall

**FPO is ONE recall source among several, not the primary classifier.** Five parallel recall sources:

| Source | Method | Output | Confidence Signal |
|--------|--------|--------|------------------|
| **HMRC FPO API** | ML-powered text-to-code from product description | HS6/CN8 candidates with retrieval scores | Score is retrieval relevance, NOT legal classification confidence |
| **CPV-to-chapter priors** | Deterministic mapping from procurement CPV codes to likely tariff chapters | Chapter/heading candidates | Statistical correlation, not legal mapping |
| **Rule-based chapter hints** | Match head noun + material + function against tariff chapter keyword tables | Chapter candidates | Pattern-based, high precision on known product families |
| **Tariff keyword search** | Search HMRC Trade Tariff API heading/subheading descriptions for extracted product terms | Heading/subheading matches | Text similarity, may miss specialized vocabulary |
| **ATR/codebook reuse** | Previously classified similar products with cached codebooks from prior ATRs | Full 10-digit codes + measure profiles | Highest confidence when available (legally binding) |

**Critical rule:** FPO agreement with other sources increases confidence. FPO alone does not constitute evidence. Correlated agreement between FPO, Taric Support, and HS Ping is not independent evidence — all three may return the same plausible-but-wrong family for vague input. Independent evidence requires slot-level compatibility checking against the actual tariff hierarchy.

**Preprocessing before recall:**

1. **Description-quality gate** — reject bundles, service descriptions, generic nouns without qualification. Output archetype-keyed missing-facts requests on rejection (e.g., gloves: material, sterile/non-sterile, examination vs household; helmets: industrial vs sports, standard met, shell material)
2. **Bundle splitter** — decompose multi-item descriptions into separate candidates. If decomposition fails, mark as mixed/manual
3. **Attribute extraction** — head noun, material, principal function, sector, sterility/medical status, power/size, pack form
4. **Multi-query recall** — for each candidate, run multiple normalized description variants across ALL recall sources

### 1.5 Multi-Family Expansion to UK 10-Digit

**Never greedy descent.** The pipeline must expand **ALL plausible HS6/CN8 families** to **ALL declarable 10-digit UK descendants** via HMRC Trade Tariff API commodity hierarchy navigation. Top-candidate poisoning — where a recall source gives a plausible HS family, the engine descends it alone, and never tests the runner-up — is a primary silent failure mode.

**Reranking descendants:** Use slot compatibility (do extracted product attributes match this code's description and scope notes?) and measure compatibility (does this code's measure profile make sense for the claimed origin and use?).

**Validation chain:** Resolve -> validate via Taric Support API -> cross-check via HS Ping.

Track `candidateRecallAgreement` for every candidate to record which recall sources agreed and which diverged.

### 1.6 Exporter/Manufacturer Resolution

Trade remedy measures on Chinese goods frequently carry exporter-specific rates that are materially lower than the residual rate.

**Examples of rate variation:**
- Aluminium foil: 14.2% to 35.6% residual depending on exporter/additional-code/invoice path
- Aluminium extrusions: 0% to 35.1% residual
- Tin mill products: 27.85% cooperating exporter vs 49.98% all other Chinese exporters

**Resolution output:**

| Field | Values | Default |
|-------|--------|---------|
| `exporterRateBasis` | `named_exporter` / `annex_exporter` / `residual_default` | **`residual_default`** (constitutional — always default to residual when identity unverified) |
| `exporterIdentityConfidence` | 0.0-1.0 | Based on supplier listing data quality |

Systematic understatement of remedy exposure through assumed named-exporter rates is a **catastrophic failure mode**.

### 1.7 Measure Interaction Engine

AD/CVD/safeguard/quota duties do NOT always stack arithmetically. UK steel-family measures demonstrate that when quota is exceeded, only the safeguard duty or only the portion of AD/CVD above the safeguard is charged.

**Interaction rules per measure family:**
- Some measure combinations are additive (base duty + AD)
- Some are substitutive (greater of safeguard or AD, not both)
- Some are conditional (quota exhaustion triggers safeguard; within quota, no safeguard)
- The engine must model family-specific interaction rules, not assume simple stacking

### 1.8 Compound/Specific Duty Expressions

Preserve full duty expressions. **Never flatten to single `dutyPct`.**

HMRC's commodities API models duty as one or more measure components, each with its own type:
- **Ad valorem:** X% of customs value
- **Specific per unit:** GBP X per kg / per litre / per unit / per 1000 items
- **Compound:** X% + GBP Y per kg (both components apply)

Supplementary units (kg net, litres, pairs, number of items) are **first-class requirements** — some tariff lines require quantity in specific units for duty calculation. Missing supplementary-unit data can prevent a valid duty estimate. Emit `SUPPLEMENTARY_UNIT_REQUIRED` error when data is unavailable.

### 1.9 VAT Rate Determination

Separate VAT rate determination from VAT recovery treatment:

| Concept | What It Is | Fields |
|---------|-----------|--------|
| **VAT rate** | Which rate applies to this commodity | `vatRateState`: `standard` / `reduced` / `zero` / `unresolved` |
| **VAT recovery** | How much import VAT the importer can reclaim | `vatRecoveryRate`: 0.0-1.0 (not a binary flag) |

National additional codes (family 6/17) carry VAT relief paths including `VATZ` for commodity-driven zero-rate. Default to **standard rate** when unresolved.

### 1.10 Valuation Mode

| Mode | When | Accuracy |
|------|------|----------|
| `estimation` (`1_proxy`) | Marketplace-derived proxy for transaction value | Indicative only — listing prices are not invoice prices |
| `declaration` (`1_actual`) | Actual transaction price from invoice/contract | Decision-grade — but may require Method 2-6 if no-sale |

**Invoice price is NOT customs value.** Selling commissions, assists, royalties, and resale-proceeds shares can change duty liability. Record `customsValuationMethod` (`1_actual` / `1_proxy` / 2-6 / `unknown`).

### 1.11 Classification Freshness System

| Field | Purpose |
|-------|---------|
| `tariffCheckedAt` | When the tariff data was last verified against HMRC Trade Tariff API |
| `classificationFreshUntil` | Expiry date for the classification (based on product family volatility) |
| `tradeRemedyWatchState` | `clear` / `investigation_open` / `measure_pending` / `measure_active` — per commodity family |

**Daily TRA watcher:** Monitor Trade Remedies Authority case register for new investigations, provisional measures, and definitive measure changes affecting classified commodity families. Emit `TRA_INVESTIGATION_OPEN` when an investigation opens for a commodity family with cached classifications. FX monthly rollover triggers forced recalculation of affected estimates.

### 1.12 Conservative Defaults (Constitutional)

These are **non-negotiable** — they apply whenever resolution is incomplete:

| Parameter | Default | Rationale |
|-----------|---------|-----------|
| Origin preference | MFN rate (not preferential) | No origin proof available |
| Exporter rate | Residual remedy rate (not named-exporter) | No exporter proof |
| Quota allocation | Out-of-quota rate | No quota allocation proof |
| VAT rate | Standard rate | Unresolved VAT status |
| Incoterm | Boundary scenarios | No stated Incoterm |
| Supplementary units | Abstain on duty for that component | Unresolved unit data |

### 1.13 Customs Branch Divergence

When plausible UK10 branches produce materially different outcomes — e.g., one branch has no trade remedy and another has 48.5% residual AD duty — this is a **hard blocker, not a scenario band**.

Emit `CLASSIFICATION_AMBIGUOUS`. Do not average the branches. A 2.7%-48.5% spread is not actionable intelligence — it is classification failure.

**Screening-grade auto-pass only when:**
- `worstCaseNetMargin >= gate`
- AND `customsCostSpread` is immaterial (all branches in same measure family)
- AND no catastrophic legal branch exists

### 1.14 CBAM Forward Placeholder

UK CBAM trial rates expected Q4 2026, full rates from January 2027. A forward `cbamEstimateGbp` field exists on `landed_cost_estimates` (nullable, currently always null). The data source and calculation method are undefined — this is a placeholder, not an implementation.

**Affected commodity families:** Iron, steel, aluminium (plus potentially cement, fertilizers, hydrogen, electricity in future phases).

### 1.15 New Skills (Layer 5)

| Skill | Purpose | Profile |
|-------|---------|---------|
| `skills/supplier-price-enrichment/amazon/` | Amazon UK marketplace search | `pty_browser` |
| `skills/supplier-price-enrichment/alibaba/` | Alibaba.com marketplace search with Chinese retry | `pty_browser` |
| `skills/supplier-price-enrichment/1688/` | 1688 wholesale marketplace search (Chinese keywords only) | `pty_browser` |
| `skills/procurement-ingestion/pcs/` | Public Contracts Scotland ingestion (if Scotland in scope) | `pty_browser` |

### 1.16 Intelligence Package Additions (19 files)

| Package | Files | Purpose |
|---------|-------|---------|
| `packages/intelligence/src/customs/` | 6 files | Full customs classification engine: description-quality gate, multi-source recall, 10-digit resolution, measure scanner, exporter resolution, classification freshness |
| `packages/intelligence/src/fx/` | 4 files | Dual FX engine: HMRC customs rates (monthly rolling), Xe commercial rates, basis gap tracking, margin-triggered recalculation |
| `packages/intelligence/src/logistics/` | 5 files | Full freight engine: mode selection, Freightos integration, SeaRates integration, dual-packing scenarios, low/base/high estimates |
| `packages/intelligence/src/validation/` | 3 files | Landed-cost comparison adapters: Dutify, Zonos, Avalara (validation only, not canonical) |
| `packages/intelligence/src/customs/accuracy.ts` | 1 file | Evidence-tiered classification accuracy measurement (gold/silver/bronze labels) |

---

## 2. FX, Freight & Multi-Source Marketplace Search

### 2.1 Dual FX Engine (`packages/intelligence/src/fx/` -- 4 files)

Two FX rates serve different purposes:

| Rate | Source | Purpose | Update Frequency |
|------|--------|---------|-----------------|
| `customsFxRate` | HMRC Exchange Rate Service | Legally authoritative for customs duty calculation | Monthly rolling (published rates) |
| `commercialFxRate` | Xe Currency Data API | Real-time market rates for commercial margin estimation | On-demand / cached with TTL |

**`fxBasisDeltaPct`:** Stored difference between customs and commercial rates. Fed into margin stress scenarios -- when the gap is large, margin estimates are less reliable.

**FX monthly rollover:** When HMRC publishes new monthly rates, all affected `landed_cost_estimates` require forced recalculation. The system detects rate changes and enqueues recalculation jobs for affected candidates.

**4 files:**

| File | Purpose |
|------|---------|
| `hmrc-exchange-rates.ts` | HMRC Exchange Rate Service client: fetch monthly rates, cache locally, detect rollovers |
| `xe-commercial-rates.ts` | Xe API client: fetch spot rates, cache with TTL, handle rate-limit |
| `fx-engine.ts` | Dual-rate computation: select correct rate per use case, compute basis delta, trigger recalculation |
| `fx-types.ts` | Type definitions: `FxSnapshot`, `FxBasisDelta`, `FxRecalculationTrigger` |

### 2.2 Full Freight Engine (`packages/intelligence/src/logistics/` -- 5 files)

| File | Purpose |
|------|---------|
| `mode-selector.ts` | Determine shipping mode(s): sea, air, express, multimodal. Based on weight/volume/value/urgency/product type. Hard mode locks: hazmat, oversized, fragile, reefer, batteries |
| `freightos-client.ts` | Freightos API integration: price stats, route quotes, container/LCL pricing |
| `searates-client.ts` | SeaRates API integration: logistics explorer rates, container pricing |
| `freight-engine.ts` | Orchestrator: resolve Incoterm cost boundary, select mode(s), fetch from both providers, dual-packing scenarios when packaging state unknown, emit `freightLowGbp` / `freightBaseGbp` / `freightHighGbp` with `modeConfidence` |
| `freight-types.ts` | Type definitions: `FreightEstimate`, `ModeSelection`, `PackingScenario` |

**Dual-source cross-check:** Fetch estimates from BOTH Freightos and SeaRates. Use agreement/disagreement as a confidence signal. Large discrepancies trigger `FREIGHT_MODE_LOW_CONFIDENCE`.

**Conservative defaults when freight data unavailable:**
- Unknown packaging state -> run dual scenarios (e.g., palletized vs loose cartons)
- No Incoterm stated -> run boundary scenarios (EXW vs FOB vs CIF assumptions)
- Neither provider can quote -> emit `FREIGHT_PROVIDER_UNSUPPORTED`, use manual estimate

### 2.3 Multi-Source Marketplace Search Skills

**4 new skills** (all follow production skill directory standard from L0 S4):

#### `skills/supplier-price-enrichment/alibaba/`

| Attribute | Value |
|-----------|-------|
| Profile | `pty_browser` |
| Search | Filter-heavy: category, supplier type, MOQ range, shipping, certifications, price range. "Ready to Ship" listings get `quoteUsabilityConfidence` bump. |
| Language | English first, **Chinese retry** when English returns zero/accessories/consumer items |
| Anti-bot | Source-health canary before production jobs. Auth-refresh skill coordination. Circuit breaker on repeated challenges. |

#### `skills/supplier-price-enrichment/1688/`

| Attribute | Value |
|-----------|-------|
| Profile | `pty_browser` |
| Search | Chinese-keyword-only. Short clusters (2-4 terms). Domestic wholesale floor prices. |
| Limitations | Results without explicit compliance/export evidence should NOT drive landed-cost decisions for regulated goods |
| Auth | Requires auth-refresh/1688 skill for session maintenance |

#### `skills/supplier-price-enrichment/amazon/`

| Attribute | Value |
|-----------|-------|
| Profile | `pty_browser` |
| Search | Alias families (multiple short noun-phrase variants). Target structured title vocabulary. |
| Notes | PA-API 5.0 deprecated 30 April 2026 -- browser-first strategy avoids API sunset risk |

#### `skills/procurement-ingestion/pcs/`

| Attribute | Value |
|-----------|-------|
| Profile | `pty_browser` |
| API | OCDS Web API with notice list and notice family retrieval. Month-based query model. |
| Scope | Only if Scotland confirmed in MVP scope (see Open Questions) |

### 2.4 Existing Field Activation (Self-Contained for L5 Implementer)

Layer 5 does NOT add columns or tables. It populates existing nullable fields created in Layer 0. An implementer reading this section should know exactly which fields become active:

**`landed_cost_estimates` -- decision-grade fields now populated:**

| Field | Type | Now Contains |
|-------|------|-------------|
| `duty_breakdown_json` | TEXT (JSON) | Full duty breakdown: base tariff rate, trade remedy duties (AD/CVD/safeguard), preferential eligibility, compound/specific components |
| `measure_interaction_json` | TEXT (JSON) | Measure interaction results: which measures stack, which substitute, family-specific rules applied |
| `exporter_rate_basis` | TEXT | `named_exporter` / `annex_exporter` / `residual_default` |
| `exporter_identity_confidence_bps` | INTEGER | 0-10000 bps confidence in exporter identification |
| `supplementary_unit_type` | TEXT | Required unit type (kg_net, litres, pairs, units, etc.) or NULL if not required |
| `supplementary_unit_value` | REAL | Quantity in required supplementary units |
| `valuation_mode` | TEXT | `estimation` / `declaration` |
| `customs_valuation_method` | TEXT | `1_actual` / `1_proxy` / 2-6 / `unknown` |
| `customs_fx_rate` | NUMERIC | HMRC customs valuation FX rate used |
| `commercial_fx_rate` | NUMERIC | Xe commercial spot rate used |
| `fx_basis_delta_pct` | NUMERIC | Difference between customs and commercial rates |
| `duty_outcome_min_pence` | INTEGER | Minimum duty across plausible branches |
| `duty_outcome_max_pence` | INTEGER | Maximum duty across plausible branches |
| `customs_cost_spread_bps` | INTEGER | Spread in basis points -- triggers blocker if material |
| `freight_low_pence` | INTEGER | Low freight estimate |
| `freight_base_pence` | INTEGER | Base freight estimate |
| `freight_high_pence` | INTEGER | High freight estimate |
| `freight_mode` | TEXT | Selected mode (sea/air/express/multimodal) |
| `freight_mode_confidence_bps` | INTEGER | Mode selection confidence |
| `tariff_checked_at` | TEXT | ISO-8601 timestamp of last tariff verification |
| `classification_fresh_until` | TEXT | ISO-8601 expiry for classification freshness |
| `trade_remedy_watch_state` | TEXT | `clear` / `investigation_open` / `measure_pending` / `measure_active` |
| `vat_rate_state` | TEXT | `standard` / `reduced` / `zero` / `unresolved` |
| `vat_recovery_rate` | REAL | 0.0-1.0 recovery fraction |
| `decision_grade` | TEXT | `screening` / `review` / `rfq_required` / `broker_required` |

**`product_commodity_codes` junction -- now actively populated:**

Multi-source recall produces multiple HS6/CN8/UK10 candidates per product. Each row links a `product_candidate` to a commodity code with recall source and agreement metadata.

**`supplier_quotes` -- additional marketplace rows:**

Same schema, more rows. Alibaba, 1688, Amazon quotes join existing eBay quotes. Each carries source-specific `priceSignalRole`:
- Amazon/eBay: `benchmark_ceiling`
- Alibaba: `optimistic_floor`
- 1688: `domestic_wholesale_floor`

---

## 3. Testing, DoD, Risks & Open Questions

### 3.1 Definition of Done

| # | Criterion |
|---|----------|
| 1 | Alibaba/1688 search skills operational with Chinese keyword clusters |
| 2 | Amazon search skill operational |
| 3 | PCS (Scotland) ingestion skill operational (if Scotland in scope) |
| 4 | HMRC FPO API integration for HS code classification candidates |
| 5 | Multi-source recall produces candidates from >=2 independent sources |
| 6 | 10-digit resolution via HMRC Trade Tariff API produces UK-specific commodity codes |
| 7 | Measure scanner detects AD/CVD/safeguard/quota measures and models interactions |
| 8 | HMRC Exchange Rate Service provides customs valuation FX (monthly rolling) |
| 9 | Xe commercial FX provides spot rates |
| 10 | Freightos + SeaRates provide freight market data |
| 11 | Decision-grade landed cost per validated category (with full audit trail) |
| 12 | Category-specific gold-set expansion completed for >=3 categories |

### 3.2 Test Scenarios

| Scenario | Expected Behavior |
|----------|------------------|
| Multi-family expansion: product with 3 plausible HS6 families | ALL UK10 descendants evaluated across all 3 families (not just top candidate) |
| Trade remedy detection: steel product | AD/CVD measures detected; residual rate applied (not named-exporter); `TRADE_REMEDY_RISK` emitted |
| Compound duty: product with specific duty (GBP X/kg) | Duty expression preserved as compound; NOT flattened to single percentage |
| FX monthly rollover | Forced recalculation of all affected landed cost estimates |
| Alibaba search with Chinese retry | English search fails -> Chinese retry on Alibaba produces results |
| Category whitelist enforcement: medical device | Rejected before search -- not in whitelist |
| `hmrc-fpo.test.ts` | FPO API response parsing, candidate code extraction, multi-query variants |

### 3.3 Evidence-Tiered Classification Accuracy

| Tier | Evidence Source | Weight |
|------|---------------|--------|
| **Gold** | ATR outcome, actual import declaration, broker-confirmed family (no later amendment) | Authoritative |
| **Silver** | Two independent internal reviewers with documented reasoning | High confidence |
| **Bronze** | Best-effort label from tender docs only | Indicative |

**Primary metrics (scored by evidence tier):**
- Duty-outcome accuracy (not just code accuracy)
- Trade-remedy yes/no accuracy
- Additional-code required yes/no accuracy
- Supplementary-unit required yes/no accuracy
- Review-trigger recall
- Catastrophic understatement rate (engine emitted a number; actual was materially higher)
- Unsafe auto-classification rate (engine auto-classified; should have escalated)
- **Abstention as positive class** -- if the description lacked facts for legal classification and the engine escalated, that is a success

### 3.4 Customs-Specific Error Types (Layer 5 Extension)

| Code | Trigger | Action |
|------|---------|--------|
| `CLASSIFICATION_AMBIGUOUS` | Top HS6 candidates diverge across recall sources | Block auto-qualification; escalate to broker |
| `TARIFF_BRANCH_DIVERGENCE` | UK10 branches split on remedy/no-remedy or duty type | Hard blocker -- NOT a scenario band |
| `MEASURE_INTERACTION_UNKNOWN` | AD/CVD/safeguard interaction rules unclear for this measure family | Flag for manual customs review |
| `EXPORTER_UNRESOLVED` | Active trade remedy but exporter/manufacturer unidentified | Residual rate assumed; operator alerted |
| `SUPPLEMENTARY_UNIT_MISSING` | Tariff line requires supplementary units; data unavailable | Abstain on duty for that component |
| `VALUATION_MODE_CONFLICT` | Marketplace proxy and declaration mode produce materially different duty | Flag uncertainty range |
| `FPO_UNAVAILABLE` | HMRC FPO API unreachable | Continue with remaining recall sources; confidence penalty |
| `TRADE_REMEDY_WATCH_STALE` | TRA investigation opened since last classification | Reclassification required |
| `FX_RATE_MISSING` | Neither HMRC nor Xe rate available | Block landed-cost computation |
| `FREIGHT_ESTIMATE_FAILED` | Neither Freightos nor SeaRates can quote | Use manual freight estimate; emit warning |

### 3.5 Open Questions

**S10 Further Research #2: Product-family-specific logistics assumptions**
- Shipping mode depends on item class and packaging density. Need default freight-mode heuristics per target product family.
- **L5 impact:** Directly affects freight engine mode selection defaults.

**S10 Further Research #3: Chinese supplier source strategy**
- Validate which supplier platforms can be lawfully and reliably integrated.
- **L5 impact:** Determines operational scope of Alibaba/1688 skills.

**S10 Further Research #8: Customs cost spread materiality threshold**
- What `customsCostSpreadBps` triggers `customs_branch_divergent` blocker? 2.7%-4.7% may be tolerable; 2.7%-48.5% is clearly a blocker. Calibrate against whitelist categories.
- **L5 impact:** Directly affects blocker logic in customs engine.

**S10 Further Research #9: Packaging materials sub-category reclassification**
- Split easy (cartons, poly mailers) vs dangerous (aluminium foil, glass, tin mill) subfamilies?
- **L5 impact:** Determines customs engine treatment of packaging category.

**S10 Further Research #10: CBAM rate integration timeline**
- UK CBAM trial rates expected Q4 2026, full rates January 2027. When to compute non-zero `cbamEstimateGbp`?
- **L5 impact:** Determines activation timeline for CBAM field.

### 3.6 Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| HMRC FPO API access restricted | Medium | High | FPO is not hard dependency; fallback to CPV-to-HS mapping + other recall sources |
| Anti-bot detection on Alibaba/1688 | High | Medium | Anti-bot classification; source-health canaries; auth-refresh skills; circuit breaker |
| Customs classification wrong for complex products | Medium | Critical | Multi-source recall + conservative defaults + abstention + broker escalation |
| Freight API rate variability | Medium | Medium | Dual-source cross-check (Freightos + SeaRates); confidence scoring |

### 3.7 Forward-Compatibility

- Classification freshness system enables future auto-refresh of stale classifications
- CBAM forward placeholder ready for future regulation implementation
- Evidence-tiered accuracy framework supports future calibration against declared outcomes

### 3.8 Dependencies

**Layers 0-4** (full pipeline, sync, web portal).

**External accounts (from S9.3):**

| Service | Type | Required |
|---------|------|----------|
| HMRC Trade Tariff Developer Portal | Government | Yes -- tariff API + FPO API |
| HMRC Exchange Rate Service | Government (free) | Yes -- customs FX |
| Xe Currency Data API | Commercial | Yes -- commercial FX |
| Freightos | Commercial | Yes -- freight pricing |
| SeaRates | Commercial | Yes -- freight cross-check |
| Taric Support | Commercial | Yes -- code validation |
| HS Ping | Commercial | Optional -- cross-check |
| Easyship, Shippo | Commercial | Optional -- parcel-mode |
| CurrencyBeacon, OANDA | Commercial | Optional -- backup FX |
| Dutify, Zonos, Avalara | Commercial | Optional -- comparison only |

### 3.9 ADRs Addressed

| ADR | Decision | Layer 5 Action |
|-----|----------|---------------|
| **ADR-006** | Internal landed-cost engine | Full decision-grade scope: multi-source recall, 10-digit resolution, measure interaction, dual FX, real freight |
| **ADR-001** | Skills as primary data retrieval | Additional marketplace skills: Alibaba, 1688, Amazon, PCS |
