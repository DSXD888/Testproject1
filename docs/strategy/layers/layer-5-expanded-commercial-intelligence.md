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
