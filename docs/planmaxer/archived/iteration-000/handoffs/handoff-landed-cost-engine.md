# Handoff: Landed-Cost & Trade Intelligence Engine

## Expert Framing

You are a **UK customs and trade compliance specialist** crossed with a **quantitative trade finance analyst**. You have built or used duty calculators, customs valuation engines, or landed-cost estimation software. You understand HMRC's Trade Tariff, the Procurement Act 2023, HS/CN commodity classification, Incoterms, customs valuation methods, import VAT recovery, and the real-world gap between theoretical duty rates and actual landed costs.

Draw from: HMRC Trade Tariff API behavior, the UK Global Tariff (UKGT), WCO HS nomenclature, Incoterms 2020, UK VAT recovery rules for registered importers, freight market pricing (Freightos/SeaRates data models), and the operational realities of importing goods from China to the UK.

## Mission

ULTRATHINK and be meticulous.

You are reviewing the **landed-cost and trade intelligence engine** of a UK procurement intelligence platform. This engine is the core business value — it determines whether a public contract opportunity is commercially viable by estimating the full landed cost of sourcing goods from cheaper suppliers (primarily China) and delivering them to the UK.

The core value proposition: an internal cost-estimation engine backed by authoritative UK government APIs (HMRC Trade Tariff, HMRC FPO for HS classification, HMRC Exchange Rates) combined with commercial freight APIs (Freightos, SeaRates) and FX data (Xe, CurrencyBeacon, OANDA). The engine produces explainable, auditable margin estimates — not black-box third-party estimates.

**Don't just answer my questions — the questions I DON'T KNOW to ask are more important than the ones I do.**

## User's Strong Beliefs (Engage With These)

1. **"An internal engine backed by HMRC is better than third-party black boxes."** The owner chose to build an internal customs/duty engine using HMRC Trade Tariff + FPO API rather than delegating to Dutify/Zonos/Avalara. Challenge or strengthen: is the engineering investment justified? Where does the internal engine's accuracy ceiling realistically fall vs. third-party services that have teams dedicated to this?

2. **"Both economic margin and cash-outlay views are needed."** The architecture distinguishes between economic margin (ex-VAT, treating recoverable import VAT as a cash-flow item) and cash outlay (including VAT). Challenge: are there other margin views that matter for procurement arbitrage? What about working capital cost, payment term assumptions, or currency hedging costs?

3. **"HMRC FPO API is the right first-pass HS code classifier."** The FPO API was originally built for Windsor Framework fast parcels. The owner plans to use it as the primary automated HS code classifier for ALL procurement goods. Challenge: how well does FPO handle non-parcel goods? Where are its blind spots? What's the accuracy for bulk/industrial goods typical in public procurement?

## Essential Context

**What the project is:** A procurement arbitrage intelligence engine. It discovers UK public contracts for physical goods, estimates whether sourcing from cheaper suppliers minus all landed costs still leaves a profitable margin (20%+ net), and surfaces opportunities to operators.

**Key terms:**
- **Landed cost:** Total cost to get goods from supplier to UK delivery, including: supplier price, freight, customs duty, import VAT, final-mile delivery, and other fees.
- **Economic margin:** (Revenue - landed cost excluding recoverable VAT) / Revenue. The "real" profitability metric.
- **Cash outlay:** Total money that must be spent upfront, including recoverable import VAT. A cash-flow metric.
- **HMRC FPO API:** Fast Parcel Operator API — accepts product descriptions and returns candidate commodity codes. ML-trained on tariff data.
- **HMRC Trade Tariff API:** Canonical UK tariff database — lookup commodity codes to get duty rates, VAT rates, quotas, and trade measures.
- **Confidence floor:** The minimum confidence score required for automated pipeline progression (suggested 0.60 initially).

**Technical constraints:**
- Cost-estimation is API-based (the ONE exception to the skill-based agent scraping architecture)
- Must handle ambiguous product descriptions from messy procurement notice text
- Must work with marketplace pricing (indicative, not executable) for early-stage filtering
- Must clearly distinguish HMRC rates (legally authoritative) from Xe rates (commercial)

## Current Architecture

### HS Code Classification Pipeline
```
Product description from notice text
  → HMRC FPO API (first-pass classification, returns candidate codes)
  → Taric Support API (code validation, completeness checking)
  → HS Ping (cross-check for additional confidence)
  → HMRC Trade Tariff API (duty measures, VAT rates, quotas)
  → Output: best-estimate code, confidence, duty estimate, VAT estimate, warnings
```

### Landed-Cost Calculation Flow
```
Select best supplier quote pairs
  → HS code classification (above pipeline)
  → Select logistics mode by quantity/weight/volume
  → Fetch freight estimates (Freightos + SeaRates)
  → FX conversion:
      - HMRC Exchange Rates for customs duty calculation
      - Xe for commercial margin estimation
  → Compute economic landed cost + cash outlay
  → Store warnings and confidence
```

### FX Strategy
- **Customs valuation:** HMRC Exchange Rate Service (MANDATORY, legally authoritative, monthly)
- **Commercial margin:** Xe Currency Data API (real-time market rates)
- **Backup/validation:** CurrencyBeacon
- **Historical audit:** OANDA

### Margin Logic
```
Gross margin gate: ≥30% (supplier cost vs. contract revenue, ex-VAT)
Net margin gate:   ≥20% (full landed cost vs. contract revenue, ex-VAT)
Confidence floor:  ≥0.60 for automated progression
```

### Confidence Model
```
final_confidence =
  (0.25 × goods_classification) +
  (0.25 × product_match) +
  (0.20 × commodity_code) +
  (0.20 × freight) +
  (0.10 × fx_freshness)
```

### LandedCostEstimate Entity
```ts
{
  shippingMode: "parcel" | "air_freight" | "lcl_ocean" | "fcl_ocean" | "mixed",
  commodityCode, commodityConfidence,
  fxRate,
  supplierCostGbp, freightCostGbp, customsDutyGbp,
  importVatGbp, importVatRecoverable,
  ukFinalMileCostGbp, otherFeesGbp,
  economicLandedCostGbp, cashOutlayGbp,
  confidence, warnings
}
```

## Tensions to Resolve

### Tension 1: HMRC FPO Accuracy for Procurement-Scale Goods
**The conflict:** HMRC FPO was built for Windsor Framework fast parcels (consumer goods, ecommerce). The platform will classify procurement-scale goods: industrial equipment, bulk office supplies, medical devices, construction materials, IT hardware in quantity.

**Current resolution:** FPO as first-pass, with Taric Support and HS Ping validation.

**Why it might be wrong:** FPO's ML model was trained on parcel-level data. Its accuracy on Chapter 84 (machinery), Chapter 90 (medical instruments), or Chapter 94 (furniture) goods typical in public procurement may be poor. A 6-digit HS code that's wrong by one digit can mean a 15%+ duty rate difference.

**Stress test:** A contract for "specialist laboratory equipment including centrifuges, sample preparation devices, and associated consumables." How does FPO handle this? What's the fallback?

### Tension 2: Freight Mode Selection with Sparse Data
**The conflict:** The plan says "select logistics mode by quantity, weight, and volume assumptions." But contract notices rarely specify exact weight/volume — they describe goods by name and quantity.

**Current resolution:** Product extraction engine provides "estimatedUnitWeightKg" and "estimatedUnitVolumeM3" — but these are estimates from text extraction.

**Why it might be wrong:** The difference between parcel and FCL ocean freight for the same goods can be 10x. Getting the mode wrong is getting the freight cost wrong by an order of magnitude. The mode-selection heuristics are barely specified beyond "by quantity, weight, and volume."

**Stress test:** A contract for "500 office chairs." How do you estimate unit weight? How do you decide between LCL and FCL? What if the same contract includes "500 office chairs and 200 desks"?

### Tension 3: Marketplace Prices vs. Procurement-Scale Pricing
**The conflict:** Supplier quotes come from marketplace scraping (eBay, Amazon, Alibaba, 1688) and are explicitly acknowledged as "indicative signals, not executable procurement offers." But the margin calculation uses these prices as if they represent actual sourcing costs.

**Current resolution:** Quote class taxonomy (indicative_marketplace, distributor_listing, rfq_quote, negotiated_quote) with confidence scoring.

**Why it might be wrong:** A contract for 10,000 units sourced at Alibaba's listing price of £2/unit may actually cost £3.50/unit at negotiated procurement volumes (or £1.20 — marketplace prices are noisy in both directions). Using marketplace prices for net margin gating means the gate itself is unreliable.

**Stress test:** The platform qualifies an opportunity at 25% net margin based on Alibaba prices. The actual negotiated price is 40% higher. The true net margin is 5%. How often will this happen?

### Tension 4: Multiple FX Sources with Different Semantics
**The conflict:** The architecture uses HMRC rates for customs duty calculation and Xe for commercial margin. But the HMRC rate is published monthly (penultimate Thursday) and applies to the following calendar month. There can be a significant gap between HMRC's rate and Xe's real-time rate.

**Current resolution:** "Store FX snapshots with source clearly identified" and recalculate when rate drift crosses a threshold.

**Why it might be wrong:** A GBP/CNY movement of 5% between HMRC publication and commercial execution changes the margin calculation materially. The plan doesn't specify what "rate drift threshold" triggers recalculation, or how to present the FX risk to operators.

**Stress test:** HMRC rate says £1 = ¥9.20 for customs. Xe says £1 = ¥8.75 today. The margin estimate uses which rate for which calculation? What does the operator see?

### Tension 5: Confidence Score Calibration
**The conflict:** The confidence model uses fixed weights (0.25, 0.25, 0.20, 0.20, 0.10) and is explicitly labeled "priors for MVP" that should be "recalibrated against observed operator decisions."

**Current resolution:** Start with heuristic weights, calibrate later.

**Why it might be wrong:** Uncalibrated confidence scores create false precision. A score of 0.72 vs. 0.68 means nothing if the weights are arbitrary. Operators may over-trust high scores or under-trust low scores because the numbers look precise.

**Stress test:** Two opportunities: (A) has 0.75 confidence with perfect HS code match but weak freight estimate, (B) has 0.65 confidence with weak HS code but strong freight. The weighted average hides the composition. What should the operator actually see?

## What I Don't Want (Anti-Patterns)

- **Don't suggest replacing the internal engine with a third-party landed-cost service.** The internal engine is a deliberate architectural choice.
- **Don't suggest removing the distinction between HMRC and commercial FX rates.** This is legally required for customs.
- **Don't propose a generic ML model for HS classification to replace FPO.** FPO is the starting point; complement it, don't replace it.
- **Don't over-optimize confidence scoring before there's real data.** The model needs to start simple and calibrate.

## What I Do Want

1. **Practical HS classification pipeline improvements** — what should the FPO fallback chain look like for procurement-scale goods? What data sources or heuristics can improve classification confidence for industrial/bulk goods?

2. **Freight mode selection heuristics** — concrete decision trees for mode selection when weight/volume data is sparse. What defaults are reasonable?

3. **Margin reliability analysis** — how should the system communicate margin uncertainty to operators? Is the current two-gate (30% gross, 20% net) model right for indicative marketplace pricing?

4. **VAT and customs valuation edge cases** — what real-world edge cases in UK customs valuation will this engine encounter? Anti-dumping duties? Preferential tariffs? Quota management? Temporary admissions?

5. **Ideas I haven't asked about** — cost components I've missed, customs scenarios that will break the model, freight estimation traps, FX risk management approaches that matter for procurement arbitrage.

6. **Recommendations in git-diff style** — reference specific sections of the research document (sections 4.4, 4.5, 4.6, 5.4, 5.7, Appendices A and B) when proposing changes.

## Context Documents to Attach

When sending this to a reviewer, attach:
- The full research document: `docs/research/2026-03-08-public-procurement-intelligence-platform-research.md`
- Pay special attention to: Sections 4.4 (Shipping), 4.5 (Customs/Tariff), 4.6 (FX), 5.4 (Data Model), 5.7 (Intelligence Pipeline), 5.11 (Error Handling), Appendix A (Margin Logic), Appendix B (Confidence Model)
