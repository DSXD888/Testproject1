# Handoff: Customs Classification & Landed-Cost Engine Maturity

## Expert Framing

You are a **customs and trade compliance technologist** who has built automated commodity classification systems for UK imports. You have hands-on experience with the HMRC Trade Tariff API, the HMRC FPO (Fast Parcel Operator) classification service, and the operational reality of getting commodity codes right at scale. You understand the HS/CN/TARIC classification hierarchy from the WCO 6-digit level through EU CN 8-digit to UK-specific 10-digit declarable codes, and you know that legal duty treatment, additional codes, measure conditions, and trade remedy liability all live at the 10-digit layer.

You draw from experience with: HMRC customs declarations (CDS/CHIEF), advance tariff rulings (ATRs), the UK Trade Remedies Authority (TRA) investigations pipeline, anti-dumping and countervailing duty orders on Chinese goods, customs valuation methods 1-6, the broadened VAT base for import VAT calculation, postponed VAT accounting (PVA) eligibility rules, and Incoterms 2020 cost boundary normalization. You understand the Procurement Act 2023 regime and how contract values are structured.

Critically, you know that classification errors do not just produce wrong duty numbers. A misclassification can trigger HMRC compliance investigations, post-clearance audits, duty demands with interest, civil penalties under the Customs (Import Duty) (EU Exit) Regulations 2018, and in extreme cases seizure of goods. You know the difference between "the system estimated 4.2% duty" and "HMRC says this is actually 48.5% because you missed the anti-dumping order." You have seen automated classification systems fail in production and know where the failure modes cluster.

---

## Mission

ULTRATHINK and be meticulous.

The platform computes landed cost for physical goods imported to the UK, primarily from China. The classification pipeline is the most critical subsystem in the entire landed-cost engine: it starts with a natural-language product description extracted from a procurement notice, runs it through HMRC FPO for candidate HS codes, resolves to 10-digit declarable commodity codes via the HMRC Trade Tariff API, scans for measures (trade remedies, quotas, controls, additional codes), and computes duty + VAT. Every downstream margin calculation depends on the classification pipeline producing correct or conservatively-bounded results.

The landed-cost engine was reviewed at a high level in iteration 000, but the architecture has expanded dramatically since then. The current specification includes: FPO as a candidate generator (not a final classifier), classification preprocessing (description-quality gate, bundle splitter, attribute extraction, multi-query FPO), 10-digit resolution via commodity hierarchy navigation, measure scanning for trade remedies and additional codes, customs valuation method awareness (methods 1-6), expanded import VAT treatment with PVA eligibility, dual FX rates (HMRC for customs valuation, Xe for commercial margin), margin scenarios with critical blockers, special procedure scenarios (inward processing, temporary admission, duty suspensions, ATQs), and a four-tier decision grade system.

Your job: stress-test this classification pipeline against the messy reality of UK customs. Can automated classification produce reliable enough duty estimates for commercial screening decisions? Where will it fail silently? What will it get catastrophically wrong? What is missing from the specification?

**Don't just answer my questions -- the questions I DON'T KNOW to ask are more important than the ones I do.**

---

## User's Strong Beliefs (Tensions to Engage With)

1. **"HMRC FPO is the right first-pass classifier."** The FPO API was designed for Windsor Framework parcel classifications -- specifically for GB-to-NI goods movements by fast parcel operators. Its ML model was trained on parcel-level classification data. Its accuracy on industrial procurement goods -- Chapter 84 machinery, Chapter 94 furniture, Chapter 73 steel articles, Chapter 90 instruments -- is unproven. The plan already acknowledges FPO as a "candidate generator, not a final classifier" and proposes multi-query variants plus downstream resolution. But is FPO even the right starting point for procurement goods, or is it a false shortcut that will produce plausible-looking but wrong candidates that poison the entire downstream pipeline?

2. **"Screening-grade vs decision-grade is the right split."** The plan distinguishes two operating grades: screening-grade (automated across the whole market using indicative data) and decision-grade (automated only for repeated product families with proven codebooks). This is formalized in ADR-006. But where exactly is the boundary? A 2 percentage-point duty difference on a 500K contract is 10K. Is that screening-grade tolerance or decision-grade consequence? The specification does not define what error margin is acceptable for screening-grade classification, which means the system has no way to know when it is producing dangerously wrong screening results.

3. **"The internal engine must be canonical, third-party services are secondary."** Building an internal customs engine from HMRC APIs is more work but more explainable and auditable. But HMRC APIs were not designed for bulk automated classification -- they are designed for individual declaration support. The FPO API was built for a specific Windsor Framework use case. The Trade Tariff API is a reference lookup tool, not a classification engine. The plan is asking these APIs to serve a purpose they were not designed for. Challenge this: where will the API-as-classification-engine approach hit fundamental limitations?

---

## Essential Context

### The Classification Pipeline (End to End)

```
Natural-language product description from procurement notice/documents
  -> Description-quality gate (reject bundles, generic nouns, service descriptions)
  -> Bundle splitter (decompose multi-item descriptions into separate candidates)
  -> Attribute extraction (head noun, material, function, sector, sterility, power/size, pack form)
  -> Multi-query HMRC FPO API (noun-only, noun+material, noun+function, noun+end-use variants)
  -> Candidate clustering and ranking at HS6/CN8 level
  -> 10-digit resolution via HMRC Trade Tariff API commodity hierarchy navigation
  -> Validation via Taric Support API (code completeness, hierarchy)
  -> Cross-check via HS Ping (additional confidence signal)
  -> Measure scanning via HMRC Trade Tariff API:
       - Standard duty rates (ad valorem, specific, compound)
       - Trade remedy duties (anti-dumping, countervailing, safeguard)
       - Preferential tariff eligibility
       - Quotas and order numbers
       - Import controls, prohibitions, restrictions
       - Additional codes required for declaration
       - Measure condition thresholds (price, weight, volume triggers)
       - Geographic applicability (origin-specific rates)
  -> Customs valuation (Method 1 transaction value, fallbacks 2-6)
  -> Import VAT calculation (broadened VAT base)
  -> PVA eligibility and VAT recovery rate determination
  -> Decision grade assignment: screening | review | rfq_required | broker_required
  -> Output: per-candidate classification result with duty estimate, VAT estimate,
     measure warnings, blockers, and classification stability score
```

### Key APIs

- **HMRC Trade Tariff API** (`api.trade-tariff.service.gov.uk`): Canonical UK tariff database. Lookup commodity codes to get duty rates, VAT rates, quotas, measures, geographic applicability. This is the authority.
- **HMRC FPO API** (`api.trade-tariff.service.gov.uk/fpo.html`): ML-powered commodity code suggestion from English-language product descriptions. Returns HS6/CN8 candidates with retrieval scores. Originally for Windsor Framework GB-to-NI parcels.
- **Taric Support API** (`taricsupport.com/api`): Commercial API for code validation, completeness checking, hierarchy browsing, and suggestions for incomplete codes.
- **HS Ping** (`hsping.com`): Commercial HS/HTS code lookup for cross-checking.

### Key Concepts

- **Commodity code hierarchy:** WCO HS6 (international) -> CN8 (EU/UK extended) -> UK10 (UK-specific 10-digit declarable code). Legal duty treatment lives at UK10.
- **Additional codes:** 4-character codes required alongside the 10-digit commodity code for trade remedies, preferences, quotas, excise. Without the correct additional code, the declaration is incomplete.
- **Measure conditions:** Conditions attached to duty rates that change liability based on price thresholds, weight, volume, certificates, or documentation. A single commodity code can have multiple conditional measures.
- **Trade remedies:** Anti-dumping duties, countervailing duties, safeguard duties. These are additional duties on top of the base tariff rate and can be 30-80% for Chinese goods. They are NOT cancelled by duty suspensions or ATQs.
- **Decision grades:** `screening` (automated, indicative), `review` (human checks needed), `rfq_required` (executable pricing needed), `broker_required` (professional customs advice needed).
- **Classification stability:** Whether FPO candidate codes converge at HS6 or diverge, indicating classification uncertainty.

---

## Current Architecture

### Classification Preprocessing (from section 4.5)

Before any API call, the pipeline applies preprocessing:

1. **Description-quality gate:** Reject raw strings that are bundles, service descriptions, or unqualified generic nouns ("goods", "samples", "parts", "equipment"). FPO documentation warns against generic words.
2. **Bundle splitter:** Decompose multi-item descriptions into separate product candidates. "Centrifuges, sample preparation devices, and associated consumables" becomes three separate classification requests. Failed decomposition marks the basket as mixed/manual.
3. **Attribute extraction:** Extract head noun, material, principal function, sector, sterility/medical status, power/size, and pack form before lookup.
4. **Multi-query FPO:** Run multiple normalized description variants per candidate: head noun only, noun+material, noun+function, noun+end-use. Reduces dependence on single phrasing.

### FPO as Candidate Generator (from section 4.5)

FPO returns HS6/CN8 candidates with retrieval relevance scores. The plan explicitly notes:
- The FPO match score indicates retrieval relevance, not legal classification confidence.
- HMRC's own repo shows kitchen towels ranking below textile-towel codes despite eventually matching paper-towel codes.
- FPO does not return 10-digit declarable codes.
- Access may be restricted to invited UK Carrier-scheme FPOs.
- Should be treated as a candidate generator, not a final classifier.

### 10-Digit Resolution and Measure Scanning (from section 4.5)

The pipeline resolves HS6/CN8 candidates to declarable 10-digit UK commodity codes via HMRC Trade Tariff API hierarchy navigation, then scans for:
- Standard duty rates and compound/specific duty components
- Trade remedy duties (anti-dumping, countervailing, safeguard)
- Preferential tariff eligibility
- Quota/order numbers and remaining availability
- Import controls, prohibitions, restrictions, certificates
- Additional codes required for declaration
- Measure condition thresholds
- Geographic applicability

### Customs Valuation (from section 4.5)

Method 1 (transaction value) is the default but explicitly noted as not universal. The plan identifies no-sale scenarios, free samples, consignment stock, branch-office transfers, hire/lease, and condition-dependent pricing as requiring fallback methods 2-6. The plan also notes that invoice price is not customs value -- selling commissions, assists, royalties, and post-import deductions can change duty liability.

### Import VAT Calculation (from section 4.5 and section 5.7 step 11)

Import VAT uses the broadened VAT base: customs value + incidental expenses to first UK destination + duty + other import charges (excluding VAT itself). PVA eligibility requires UK VAT registration, business-use import, right to dispose, and VAT number on declaration. Non-VAT-registered importers cannot reclaim. Partially exempt or mixed business/non-business use reduces recovery rates.

### Dual FX (from section 4.6 and section 5.7 step 10)

- **HMRC Exchange Rate Service** for customs duty calculation (legally authoritative, published monthly on penultimate Thursday, applicable for the following calendar month).
- **Xe** for commercial margin estimation (real-time market rates).
- `fxBasisDeltaPct` stored to track the gap between customs and commercial rates.

### Margin Scenarios and Critical Blockers (from section 5.7 step 13-15 and Appendix A)

The landed-cost estimate produces `netMarginLowPct`/`netMarginBasePct`/`netMarginHighPct` using freight low/base/high and duty variation from classification uncertainty. Critical blockers that override margin regardless of value:
- Classification unstable (HS6 diverges or 10-digit unresolved)
- Trade remedy detected
- Measure condition unsatisfied (operational blocker)
- Revenue basis uncertain
- PVA ineligible

### Special Procedures (from section 4.5)

The engine exposes special procedures as scenario switches:
- **Inward Processing:** Suspends duty and import VAT for goods processed and re-exported.
- **Temporary Admission:** Removes duty for temporary imports re-exported.
- **Duty suspensions and ATQs:** Reduce or eliminate base duty (but NOT trade remedy duties or VAT).

### Decision Grades and Escalation (from section 4.5 and section 5.7 step 14)

Four decision grades: `screening` | `review` | `rfq_required` | `broker_required`. Escalation triggers:
- HS6 candidates do not converge (code dispersion)
- Candidates converge at HS6 but diverge at 10-digit measure level (measure dispersion)
- Additional codes or measure conditions that the engine cannot resolve
- Trade remedy duties detected
- Positive margin with unsatisfied control measure (operational blocker)

### ATR/AVR Workflow (from section 4.5)

For repeated high-value product families: request Advance Tariff Rulings (legally binding for 3 years, 30-120 day response) and Advance Valuation Rulings (binding for 3 years, 90-day response). Cache approved product-family codebooks and reuse.

### LandedCostEstimate Data Model (from section 5.4)

Key fields: `declarableCommodityCode10`, `candidateCommodityCodes[]`, `additionalCodes[]`, `classificationStability`, `customsValuationMethod`, `customsFxRate`, `commercialFxRate`, `fxBasisDeltaPct`, `customsDutyGbp`, `tradeRemedyDutyGbp`, `importVatTreatment`, `vatRecoveryRate`, `decisionGrade`, `confidenceBreakdown`, `blockers[]`.

### Confidence Model (from Appendix B)

```
final_confidence =
  (0.20 * classificationStability) +
  (0.20 * quoteRealism) +
  (0.20 * commodityCodeConfidence) +
  (0.15 * freightModeStability) +
  (0.15 * revenueReliability) +
  (0.10 * fxFreshness)
```

Classification stability gets 20% weight plus commodity code confidence gets another 20% -- meaning classification-related factors account for 40% of the final confidence. Critical blockers override the composite score.

### Error Handling (from section 5.11)

Customs-specific error types: `COMMODITY_CODE_LOW_CONFIDENCE`, `FPO_ACCESS_UNAVAILABLE`, `HS10_UNRESOLVED`, `ADDITIONAL_CODE_REQUIRED`, `MEASURE_CONDITION_UNSATISFIED`, `TRADE_REMEDY_RISK`, `INCOTERM_COST_OVERLAP`, `PVA_INELIGIBLE`.

---

## Tensions to Resolve

### Tension A: FPO Accuracy on Procurement Goods

**The conflict:** FPO was trained on parcel-classification data for the Windsor Framework. Its training corpus is consumer parcels moving GB to NI -- smartphones, clothing, cosmetics, food items. The platform will classify procurement-scale industrial goods described in vague, bundled public-sector language.

**Why this matters:** Procurement descriptions are fundamentally different from parcel descriptions. A parcel label says "Samsung Galaxy S25 Ultra 512GB." A procurement notice says "supply of communication devices for field operatives including ruggedised handsets with IP68 rating." The vocabulary, specificity, and structure are different worlds.

**Concrete risk scenarios:**

- **Chapter 84 machinery:** "Laboratory centrifuges and associated sample preparation equipment" -- FPO may return 8421.19 (centrifuges for lab use) but miss the distinction between benchtop centrifuges (8421.19.20) and ultracentrifuges (8421.19.70), which have different duty rates and may have different trade remedy exposure.
- **Chapter 94 furniture:** "Modular office furniture system including desks, pedestals, and screen dividers" -- FPO may return 9403.30 (wooden office furniture) when the goods are actually 9403.10 (metal) or 9403.20 (other metal). The material distinction matters for trade remedy coverage.
- **Chapter 73 steel articles:** "Galvanised steel shelving units for warehouse storage" -- multiple potential codes depending on whether these are classified as furniture (9403), articles of iron/steel (7326), or structural steel (7308). Different chapters, different duty rates, potentially different trade remedy exposure.

**What to stress-test:** What is the realistic accuracy ceiling for FPO on procurement-scale industrial goods? Should there be a fallback classifier for when FPO candidates have low confidence or high dispersion? What about a CPV-to-HS mapping heuristic as a second opinion?

### Tension B: 10-Digit Resolution Ambiguity

**The conflict:** At HS6, "safety helmets" is one code (6506.10). At 10-digit level, the same helmet may have different codes depending on material (plastic shell vs fibre-reinforced composite), construction method, and whether it is industrial (EN 397) or sports (EN 1078). The product description from a procurement notice may not contain enough detail to resolve to a specific 10-digit code.

**Why this matters:** The difference between two 10-digit codes under the same HS6 heading can include: different duty rates, different additional code requirements, different measure conditions, and critically different trade remedy coverage. Two 10-digit codes under 6506.10 might have duty rates of 2.7% and 4.7% respectively -- and one might have an anti-dumping order while the other does not.

**The fundamental problem:** 10-digit resolution often requires information that is NOT in the product description: material composition percentages, manufacturing method, specific standards met, intended end-use. The plan's "attribute extraction" step extracts what is available, but for many procurement descriptions, the extracted attributes will be insufficient for unambiguous 10-digit resolution.

**What to stress-test:** How should the pipeline handle irreducible 10-digit ambiguity? Should it compute duty ranges across all plausible 10-digit candidates? How many of the initial whitelist categories (office supplies, cleaning products, basic PPE, hand tools, basic furniture, packaging materials) have material 10-digit ambiguity?

### Tension C: Trade Remedy Detection for China-Origin Goods

**The conflict:** Anti-dumping duties on Chinese goods can add 30-80% on top of the base tariff rate. For a platform that primarily sources from China, trade remedies are potentially the single largest cost factor in the entire landed-cost calculation -- larger than freight, larger than base tariff, larger than VAT.

**Why this matters:** The UK TRA maintains active investigations and existing EU-era transitional measures. Some product categories in the initial whitelist may already be covered by trade remedies. New investigations can be initiated at any time. An anti-dumping order that was not active when the system classified a product family may be imposed before the goods are actually imported.

**Current approach:** The plan scans measures via the HMRC Trade Tariff API. This catches existing duties but does not monitor the TRA investigations pipeline for upcoming measures.

**What to stress-test:** Is the current detection approach (static measure scanning at classification time) sufficient, or does the system need active monitoring of TRA investigations? Which whitelist categories have the highest trade remedy risk? How should the system handle products that are trade-remedy-adjacent (not currently covered but in categories where new measures are being investigated)?

### Tension D: Incoterm Cost Boundary Normalization

**The conflict:** The plan normalizes all supplier quotes to EXW (ex-works) for consistent comparison. But real-world Alibaba listings use a variety of Incoterms: FOB Shenzhen, FOB Shanghai, CIF Felixstowe, DDP UK, and frequently no stated Incoterm at all. The cost elements included vary by Incoterm, and many marketplace sellers use terms loosely or incorrectly.

**Why this matters:** Normalizing FOB Shenzhen to EXW requires estimating Chinese inland transport and export clearance costs. Normalizing CIF UK to EXW requires decomposing freight and insurance components from the seller's price. When the seller's price is already an indicative marketplace listing (not a real transaction), adding further estimation layers to normalize the Incoterm creates compounding uncertainty.

**Real-world complications:**
- Many Alibaba listings state "FOB" without specifying a port.
- Some listings offer multiple Incoterms at different prices (FOB $5.20, CIF $6.80) -- which one does the system capture?
- "DDP UK" from an Alibaba seller often means the seller uses a logistics partner who handles customs, but the duty liability still falls on the importer of record.
- Grouped invoicing across multiple SKUs with a single Incoterm header.
- Partial DDP where the seller handles freight but not duty.

**What to stress-test:** How robust is the normalization against real-world Incoterm complexity? What happens when no Incoterm is stated? What is the cost estimation error introduced by Incoterm normalization for each common case?

### Tension E: Customs Valuation for Marketplace Indicative Pricing

**The conflict:** Method 1 customs valuation (transaction value) is defined as "the price actually paid or payable for the goods when sold for export to the customs territory." But when the system computes a landed-cost estimate from an Alibaba listing price, there is no sale and no export. The listing price is an indicative signal, not a transaction.

**Why this matters:** The customs engine must distinguish between estimation and declaration. When estimating for screening purposes, using the listing price as a proxy for transaction value is reasonable. But the specification does not make this distinction explicit. The `customsValuationMethod` field on `LandedCostEstimate` suggests the system records which method was used, but the landed-cost pipeline (section 5.7 step 9) says "Apply customs valuation rules" without distinguishing between estimation-mode (using indicative pricing) and declaration-mode (using actual transaction values).

**The deeper issue:** If the system eventually handles RFQ-quoted or negotiated prices (`executable` price-signal role), it needs to apply customs valuation correctly at that point. The transition from "we estimated duty using a marketplace price" to "we computed duty using a real transaction price" is a significant operational boundary that the specification does not address.

**What to stress-test:** Does the customs engine correctly model the distinction between estimation and declaration? Should `customsValuationMethod` indicate "estimated_method_1_proxy" vs "method_1_actual"? What valuation method applies when there is genuinely no transaction (e.g., goods shipped on approval, consignment stock)?

---

## What I Don't Want (Anti-Patterns)

- **Don't propose replacing the internal engine with a third-party service** (Avalara, Zonos, Dutify). The internal engine backed by HMRC APIs is a deliberate architectural decision formalized in ADR-006. Strengthen or refine it, don't replace it.
- **Don't suggest "just use a customs broker for everything."** The goal is automated screening-grade classification. Broker escalation is already the fallback for cases the engine cannot resolve. The question is how to make the automated path as reliable as possible.
- **Don't propose training a custom ML classifier.** FPO plus resolution is the approach. Complement it with heuristics, fallback logic, and validation -- not with a new ML model.
- **Don't suggest simplifying to HS6-only.** UK duty treatment, trade remedies, additional codes, and measure conditions all live at the 10-digit level. HS6-only classification is not useful for duty estimation.

---

## What I Do Want

1. **Concrete scenarios where the classification pipeline will produce wrong results**, with specific product examples from the initial whitelist categories (office supplies, cleaning products, basic PPE, hand tools, basic furniture, packaging materials). Not theoretical risks -- specific products where the pipeline will misclassify.

2. **Product category analysis for the initial whitelist** -- which of the six categories are classification-easy and which are classification-hard? Where does 10-digit ambiguity cluster? Which categories have material trade remedy risk from China?

3. **Proposals for measuring classification accuracy and tracking it over time.** The plan mentions tracking "chapter accuracy, HS6 accuracy, 10-digit accuracy, duty-outcome accuracy, and review-trigger recall" but does not specify how to establish a ground truth for comparison. How do you build a benchmark corpus when you do not have a customs broker checking every classification?

4. **Analysis of trade remedy risk for the initial whitelist categories.** Which commodity codes in these categories currently have active UK anti-dumping or countervailing duty measures? Which are under investigation? What is the approximate duty rate impact?

5. **Ideas for handling the "procurement description is too vague to classify" problem.** When the description quality gate rejects a description, what happens? When the bundle splitter cannot decompose, what happens? When attribute extraction produces sparse results, what is the realistic confidence floor?

6. **Analysis of the interaction between classification confidence and downstream margin gates.** If classification uncertainty creates a duty range of 2.7%-48.5% (base tariff vs trade remedy), the margin scenarios are meaningless. How should the margin gate handle extreme classification-driven duty uncertainty?

7. **Ideas I haven't asked about** -- especially around: the temporal validity of classification results (how long is a classification result "fresh"?), the interaction between FPO candidate ranking and 10-digit resolution (does FPO's top candidate always lead to the right 10-digit code?), whether the measure scanning step can handle compound and specific duties correctly (per-kg, per-unit, ad valorem + specific), and any blind spots in how the specification handles the difference between standard VAT rate goods and reduced/zero-rated imports.

8. **Git-diff style output referencing specific doc sections** -- primarily sections 4.5, 4.6, 5.4 (LandedCostEstimate interface), 5.7 (landed-cost data flow steps 7-18), 5.11 (customs error types), 6.1 (complexity), 6.2 (risk assessment), ADR-006, Appendix A (margin logic), and Appendix B (confidence model).

---

## Context Documents to Attach

When sending this to a reviewer, attach:

- **Full research document:** `docs/research/2026-03-08-public-procurement-intelligence-platform-research.md`
- **Pay special attention to:**
  - Section 4.5 — Customs, Tariff, and Commodity Classification (lines ~1212-1509)
  - Section 4.6 — FX Conversion (lines ~1510-1636)
  - Section 5.4 — Data Model, specifically `LandedCostEstimate`, `CandidateCommodityCode`, `ConfidenceBreakdown` interfaces (lines ~3360-3422)
  - Section 5.7 — Intelligence Pipeline Data Flow, specifically "Landed Cost (API-Based)" steps 4-18 (lines ~4816-4858)
  - Section 5.11 — Error Handling Strategy, specifically pipeline error types related to customs (lines ~5307-5329)
  - Section 6.1 — Complexity Analysis, landed-cost engine rated "Very High" (lines ~5431-5465)
  - Section 6.2 — Risk Assessment, especially HS code risk, trade remedy risk, VAT recoverability risk, FPO access risk (lines ~5478-5517)
  - ADR-006 — Build an Internal Landed-Cost Engine (lines ~6254-6287)
  - Appendix A — Suggested Margin Logic, especially net margin gate and critical blockers (lines ~7285-7418)
  - Appendix B — Suggested Confidence Model, classification weight = 40% of composite (lines ~7420-7456)
