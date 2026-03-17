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

# UK Public Procurement Intelligence Platform - Research Document

> Generated by idea-to-research skill on 2026-03-08
> Project: Testproject1
> Current Repo State: Greenfield repository with no application code yet
> Recommended Stack: Rust daemon (SQLite local + Supabase/Postgres sync) + TypeScript frontend + skill-based agent scraping

---

## Table of Contents

1. [Executive Summary](#1-executive-summary)
2. [Intent & Requirements Analysis](#2-intent--requirements-analysis)
3. [Internal Codebase Analysis](#3-internal-codebase-analysis)
4. [External Technology Research](#4-external-technology-research)
5. [Technical Architecture Blueprint](#5-technical-architecture-blueprint)
6. [Implementation Considerations](#6-implementation-considerations)
7. [Architectural Decisions](#7-architectural-decisions)
8. [File Reference Map](#8-file-reference-map)
9. [Dependencies & Prerequisites](#9-dependencies--prerequisites)
10. [Open Questions](#10-open-questions)
11. [Next Steps](#11-next-steps)
12. [Appendices](#appendices)

---

## 1. Executive Summary

### Feature Overview

The product is a procurement intelligence platform for UK public-sector contracts. It should ingest official contract notice data every day, normalize and store it, qualify it for relevance to physical-goods supply opportunities, enrich qualified notices with supplier pricing plus landed-cost estimates, surface high-margin opportunities in the desktop control panel and viewing portal, and optionally launch request pipeline automation workflows using agent skills and browser automation (the platform automates preparation and submission steps; it does not place real-money bids without human approval).

**Terminology note:** Throughout this document, a **contract notice** is the source publication, a **procurement process** is the canonical internal record assembled from one or more notices, and an **opportunity** is the operator-facing label for that procurement process. A **search profile** is the persisted structured search definition; **query waves** are the concrete search attempts generated from it. A **match** is a candidate-listing identity judgment; a **supplier quote** is the normalized commercial record persisted only after a listing survives screening and pricing checks.

This is not one system. It is four tightly connected systems:

1. Procurement ingestion.
2. Intelligence enrichment and margin estimation.
3. User-facing review and decision surfaces.
4. Agent-assisted request pipeline automation.

### Key Recommendations

- **Primary data strategy:** Use skill-based agent scraping via browser automation for all procurement and supplier data retrieval. The agent runs on the local machine, powered by the skill system and triggered by the Rust daemon. APIs are reserved for cost-estimation data (customs, freight, FX) where they are more reliable. The skill layer is operationalized as a production contract system: each skill is a versioned directory with structured schemas, canary inputs, and golden assertions; skills follow a 6-stage promotion flow (draft → candidate → canary → shadow → stable → rollback); and return 6 outcome classes (not binary success/failure) enabling the daemon to handle continuation, degradation, and quarantine of suspect output. Every skill run produces a human-readable **run manifest** — a structured Markdown narrative of what the agent did, what it found, and how it interprets its results. Validator skills review these run manifests to catch semantic errors invisible to code-based checks (e.g., schema-valid data extracted from the wrong page or wrong locale).
- **System of record:** Authority-matrix database architecture — local SQLite as local operational authority (pipeline data, jobs, logs), Supabase/Postgres as remote shared authority (config, operator decisions) and remote projections (viewer-facing pipeline facts). Mixed-write tables prohibited. Push-only pipeline replication via transactional outbox; pull-only shared config/decisions via cursor-based sync.
- **Desktop/runtime architecture:** Use Tauri v2 with a separate Rust daemon process for local agent execution. Desktop communicates with the daemon via a hybrid transport: Tauri `invoke()` for snapshot reads (QueryService → read-only SQLite) and mutations (DaemonBridge → UDS/named-pipe → daemon), Tauri `Channel<T>` for live session streaming.
- **Frontend strategy:** The Tauri desktop control panel is the full operator surface — trigger workflows, manage agents, make decisions. The viewing portal is read-only with Supabase Broadcast invalidation for live updates. Both share presentational React components and view-model types via `packages/ui`, but data-fetching hooks are surface-specific (SQLite-via-QueryService vs Supabase-client). The desktop control panel supports browse-only degraded mode when the daemon is unavailable.
- **Cost-estimation engine:** Build an internal landed-cost engine backed by HMRC Trade Tariff, HMRC FPO API for first-pass HS code classification, Taric Support and HS Ping for validation, HMRC Exchange Rate Service for customs valuation FX, Xe for commercial FX, Freightos and SeaRates for freight-market data, and CurrencyBeacon/OANDA plus optional third-party landed-cost services as secondary comparison sources.
- **Agent orchestration:** Use adapter-mediated agent execution with PTY as a first-class runtime from day 1. The Rust daemon triggers Codex/Claude sessions on cron schedules and manual triggers from the desktop control panel, selecting PTY or contract mode per job. PTY-first remains a design constraint for browser-heavy and interactive flows.
- **Automation policy:** The platform automates the request pipeline, not real-money bid submissions. Actual negotiations and fulfillment are handled manually by humans. An error manifest system flags failures with enough context to debug. Human review remains valuable for commercial decisions but risk is low since no real money changes hands automatically.

### Primary Recommendation in One Sentence

Build a greenfield monorepo around skill-based agent scraping for data retrieval, a Rust daemon with local SQLite and Supabase/Postgres sync, adapter-mediated agent orchestration with PTY-first support, and an expanded API-powered internal cost-estimation engine — with the desktop control panel as the writable operator surface and the viewing portal as the read-only remote surface.

### Critical Decisions Required

1. Whether MVP scope is England/Wales/Northern Ireland only or full UK coverage including Scotland-specific procurement flows.
2. ~~Whether the first release should optimize for desktop-first operators or equal desktop/web parity.~~ **RESOLVED:** Desktop = control panel (full read/write/trigger), viewing portal = read-only.
3. ~~How much autonomy is acceptable in bidding workflows before manual review is mandatory.~~ **RESOLVED:** Full automation of the request pipeline is acceptable — this is not real-money bidding. Human negotiation handles actual commercial decisions.
4. Whether the margin model should default to a VAT-recoverable importer-of-record assumption.
5. ~~Whether Chinese supplier pricing is allowed to rely on browser automation when official APIs are insufficient.~~ **RESOLVED:** Yes — browser automation via agent skills is the primary data retrieval strategy for all procurement and supplier data.

### Quick Stats

| Metric | Value |
|--------|-------|
| Project Type | Greenfield |
| Existing App Files | 0 meaningful application files |
| New Files | 155 planned files (123 implementation + 32 tests) |
| Modified Files | 0 required beyond bootstrap |
| New Dependencies | ~20-35 across Rust + TS |
| Estimated Effort | XL |
| Estimated Complexity | Very High |
| Highest-Risk Area | Agent/runtime orchestration reliability + product identity/comparability chain (documents → lots → candidates → archetype-driven query-wave matching → evidence-state filtering → adaptive shortlisting) + sync integrity |

### Recommended Delivery Phases

Delivery is sequenced by risk retirement, not architectural completeness. A **Minimum Viable Experiment (MVE)** validates the intelligence thesis before infrastructure investment.

0. **Phase 0 — Gold Set + Evaluation Harness:** Document acquisition, lot/product extraction, supplier matching, screening-grade margin bands — proving the intelligence thesis on a curated lot corpus before building infrastructure.
1. **Phase 1 — Thin Local Runner:** Minimal Rust runtime with agent launch, artifact capture, and local SQLite — PTY as a capability, not a full daemon platform.
2. **Phase 2 — Single-Operator Desktop MVP:** Manual-trigger desktop UI with list/detail review, decision capture, and FTS ingestion — proving repeatable operator value.
3. **Phase 3 — Operational Hardening:** Cron scheduling, batched ingestion, health monitoring, PTY hardening, relation resolution — daily automation without babysitting.
4. **Phase 4 — Shared Visibility:** Supabase sync, projection tables, viewing portal — collaboration infrastructure.
5. **Phase 5 — Expanded Commercial Intelligence:** Broader categories, more supplier sources, decision-grade landed cost.
6. **Phase 6 — Request Pipeline Automation:** Portal-specific bid skills.

### What Will Make or Break the Product

- Reliable notice ingestion via agent skills is easy compared with supplier-price normalization via agent skills.
- Supplier-price normalization is easy compared with correct product matching.
- Product matching is easy compared with accurate landed-cost estimation at contract scale.
- Landed-cost estimation is easy compared with robust PTY-based agent orchestration that works every day.
- **But the real bottleneck is the identity chain:** documents/lots → candidate identity → quoteability → comparable listing shortlist → price-signal usability. The architecture compresses this into concrete stages with deterministic gates, `matchArchetype`-driven query planning, four-state evidence comparison, adaptive diversified shortlisting, compliance evidence tiers, and dual confidence scoring with lexicographic gating to prevent fake precision from propagating downstream.

The worker-validator architecture addresses the most insidious failure mode: schema-valid garbage from skills that navigated to the wrong page, extracted data from the wrong locale, or interpreted the wrong context. Code-based checks (Output Quality Gate) catch structural and statistical anomalies; the semantic validator catches contextual errors invisible to code — an agent reads the run manifest as a human would and judges whether the output makes sense.

This means the platform must preserve confidence scores, raw evidence, full-fidelity agent execution traces (raw PTY output in PTY mode; raw stdout/stderr plus structured protocol events and result artifacts in contract mode), run manifests as human-readable forensic narratives, and an error manifest system from day one.

---

## 2. Intent & Requirements Analysis

### 2.1 Problem Statement

**The Problem:**

You want to identify UK public contracts for physical goods where your business can source from lower-cost suppliers, especially China-linked marketplaces or wholesale suppliers, calculate realistic landed cost in GBP, and decide whether the arbitrage margin justifies pursuing the opportunity.

**Why It Matters:**

Without a structured pipeline:

- high-value opportunities remain hidden in noisy procurement feeds,
- manual screening takes too long,
- expiration windows are missed,
- margin calculations are inconsistent,
- bidding effort gets spent on contracts that are not commercially viable.

**Current State:**

There is no implementation yet. The repo is effectively empty, so the research must define the foundational architecture instead of fitting into existing code.

### 2.2 Deep Intent Summary

The user is not asking for a generic contract-search interface. The real target is a **lead-qualification and arbitrage intelligence engine** with these business goals:

1. Pull in public contract opportunities daily.
2. Filter to contracts that are still actionable.
3. Filter further to physical-goods opportunities.
4. Estimate whether UK/EU sourcing is materially more expensive than Chinese sourcing.
5. Include logistics, customs, and FX to reach a true landed margin.
6. Present only the contracts worth human attention.
7. Let an operator reject, review, or trigger request pipeline automation.

### 2.3 User Stories

```gherkin
As a procurement operator
I want new public contract notices ingested automatically
So that I do not rely on manual searching across multiple portals
```

```gherkin
As a sourcing analyst
I want the platform to estimate supplier prices, shipping, duty, VAT, and net margin
So that I can decide whether a contract is commercially viable
```

```gherkin
As a business user
I want a clear manifest showing why a contract is attractive or unattractive
So that I can review evidence instead of raw system output
```

```gherkin
As an operator
I want to press "bid" and trigger a portal-specific workflow
So that repetitive bidding preparation is partially automated
```

**Primary User Stories (Desktop Operator):**

1. As a desktop operator, I want daily ingestion and manual reruns triggered from the desktop control panel so I can trust pipeline freshness.
2. As an analyst, I want only active goods-based contracts surfaced so I do not waste time on services or near-expiry notices.
3. As a decision-maker, I want landed-cost and net-margin estimates in GBP with evidence so I can decide whether to pursue the opportunity.
4. As a desktop operator, I want clear reject/bid controls and auditable history so contract handling is disciplined.
5. As a desktop operator, I want to see an error manifest when pipeline steps fail so I can debug issues quickly.

**Viewing-Portal Stories:**

1. As a web viewer, I want to browse opportunities, view manifests, and see pipeline status without needing the desktop app installed.
2. As a web viewer, I want to see error status and pipeline health so I understand the current system state.

**Secondary User Stories:**

1. As an administrator, I want skills, prompts, source credentials, and company profile data centrally managed so local agents run consistent workflows.
2. As a compliance-aware operator, I want confidence scores and human-override points so I know when the system is uncertain.
3. As an operator, I want comprehensive agent tracing logs including full session output so I can trace exactly where errors come from.

### 2.4 Success Criteria

**Functional Criteria:**

- [ ] Daily ingestion pulls all relevant notices from selected official sources with cursor/date tracking.
- [ ] Contracts expiring within 30 days are automatically excluded from active qualified views.
- [ ] Goods-vs-services classification is available for every normalized contract with evidence and confidence.
- [ ] Supplier-price enrichment stores at least one UK/EU and one China-oriented sourcing estimate for qualified goods contracts where feasible.
- [ ] Landed-cost calculation includes FX, customs duty, import VAT logic, logistics estimate, and confidence metadata.
- [ ] Contracts are only highlighted as final qualified opportunities when estimated net margin exceeds 20% and the estimate confidence clears the review threshold.
- [ ] Every final opportunity has a readable Markdown manifest stored as an artifact.
- [ ] Operators can reject, watch, or start a bid workflow from the desktop control panel.

**Non-Functional Criteria:**

- [ ] Freshness: due ingestion jobs are triggered by the daemon's local cron scheduler within 24 hours of publication cadence, and completed within SLA while the daemon is online.
- [ ] Reliability: job execution is idempotent and stored in local SQLite with sync to Supabase.
- [ ] Explainability: every automated decision stores evidence, source URLs, timestamps, and model confidence.
- [ ] Security: secret-bearing actions run in the daemon, not in the browser.
- [ ] Observability: every ingestion, enrichment, and bidding run is traceable.

### 2.5 Scope Definition

**MVP Scope (Must Have):**

- Official-source procurement ingestion for selected UK sources.
- Normalized notice + contract storage.
- Preliminary qualification by expiry and goods relevance.
- Supplier-price enrichment from selected sources.
- Landed-cost engine with shipping/customs/FX.
- Desktop control panel + viewing portal for browse/search/review.
- Manifest generation.
- Manual and scheduled execution.
- Portal-specific bid workflow framework, with at least one example implementation.

**Later Milestones (Should Have):**

- More procurement source scraping skills.
- More supplier marketplace scraping skills.
- Better product matching using embeddings + structured attributes.
- Freight broker quote escalation when confidence is low.
- Multi-user collaboration and task assignment (via Supabase sync).

**Future Considerations (Could Have):**

- Cloud fallback execution when the local daemon is offline.
- Automated RFQ emails/messages to suppliers.
- Historical win-rate and bid-quality analytics.
- Portfolio optimization across contracts.
- Inventory-aware sourcing recommendations.

### 2.6 Non-Goals & Exclusions

**Explicitly Out of Scope for MVP:**

- Fully autonomous bid submission for all portals.
- Replacing customs brokers or freight forwarders in legally sensitive cases.
- Full ERP/accounting integration.
- Guaranteed exact landed-cost precision for every SKU without human verification.
- Unsupported blanket scraping of marketplaces that prohibit or technically block it.

**Deferred:**

- Invoicing and fulfillment operations after a bid is accepted.
- Full tender drafting beyond structured pre-bid submission support.
- Broad non-UK procurement expansion.

### 2.7 Constraints

- Procurement data is heterogeneous across portals and legal regimes.
- China-oriented supplier marketplaces require skill-based browser scraping; structured data access is limited.
- Customs classification accuracy is business-critical and sometimes legally sensitive.
- Freight estimation depends on shipment mode, Incoterms, density, route, seasonality, and carrier conditions.
- Local daemon execution introduces availability and concurrency concerns.
- Tauri desktop constraints mean server-style Next.js patterns should be avoided in the desktop shell.

### 2.8 Key Ambiguities Identified

| Ambiguity | Why It Matters | Working Assumption |
|-----------|---------------|--------------------|
| Full UK or only non-Scottish procurement in MVP | Source skills and legal variation differ | Start with England/Wales/NI plus optional PCS scraping skill |
| Exact supplier sources in China | Marketplace access and ToS differ sharply | Skill-based agent scraping via browser automation is the primary approach |
| Web and desktop parity | Affects frontend architecture | **RESOLVED:** Desktop = control panel, viewing portal = read-only |
| Fully automatic bidding or semi-automatic | Major risk and compliance impact | **RESOLVED:** Full automation of request pipeline; human negotiation handles commercial decisions |
| Use of pure local compute | Affects reliability of scheduled ingestion | **RESOLVED:** Local-first with Rust daemon, SQLite, PTY agent execution. Supabase for shared sync. |

### 2.9 Research Direction

Based on the intent analysis, the most important research areas were:

1. Official UK procurement data access because ingestion quality is foundational.
2. Customs and freight estimation because margin decisions depend on landed cost, not ex-works price.
3. Supplier pricing retrieval and source constraints because China-side sourcing intelligence is the noisiest part of the system and relies on skill-based scraping.
4. Desktop/daemon/shared-database architecture because the user explicitly wants local agent execution with centralized storage.
5. Skills and agent CLI orchestration because this is core to the desired workflow model.

---

## 3. Internal Codebase Analysis

### 3.1 Architecture Overview

**Project Type:** Greenfield repository

**Current Tech Stack:**

- Language: None established
- Framework: None established
- Database: None established
- Build: None established
- Test: None established

**Observed Repository State:**

```text
/Users/alinaaitova/Desktop/Testproject1
├── .git/
└── yolo                # placeholder file with "test"
```

### 3.2 Current Reality

There is no meaningful existing codebase to integrate with. That changes the purpose of this section:

- there are no current patterns to reuse,
- there are no integration points to preserve,
- there is no legacy data model to migrate,
- there are no tests to mirror,
- there is no runtime to extend.

This research document therefore acts as a **foundational architecture brief** for implementation, not as an extension design for an existing application.

### 3.3 Files Present Today

#### File: `yolo`
**Location:** `/Users/alinaaitova/Desktop/Testproject1/yolo`

**Relevance:** None to the target application. Appears to be a placeholder file.

**Recommendation:** Ignore during bootstrap. Delete only during explicit repo cleanup approved by the operator.

### 3.4 Proposed Greenfield Project Shape

Because the repo is empty, the most valuable internal design output is a proposed structure:

```text
Testproject1/
├── apps/
│   ├── desktop/                 # Tauri desktop shell — CONTROL PANEL (full read/write/trigger)
│   ├── web/                     # hosted viewing portal — READ-ONLY
│   └── api/                     # optional thin hosted API for privileged operations
├── crates/
│   ├── daemon/                  # Rust daemon: PTY agent orchestration, SQLite storage,
│   │                            #   cron scheduling, Supabase sync, log capture
│   └── common/                  # optional later extraction for shared runtime code
├── packages/
│   ├── contracts-schema/        # shared TS domain types / zod schemas
│   ├── db/                      # SQL migrations (Supabase), typed queries, generated types
│   ├── intelligence/            # cost-estimation APIs: HMRC tariff, freight, FX integration
│   ├── manifests/               # Markdown manifest generation
│   ├── bidding/                 # bid workflow orchestration definitions
│   ├── ui/                      # shared React components
│   └── config/                  # optional later extraction for shared config helpers
├── skills/
│   ├── procurement-ingestion/   # agent scraping skills for procurement sources
│   │   ├── fts/                 # Find a Tender scraping skill
│   │   ├── contracts-finder/    # Contracts Finder scraping skill
│   │   └── pcs/                 # Public Contracts Scotland scraping skill
│   ├── supplier-price-enrichment/
│   │   ├── ebay/                # eBay UK browser scraping skill
│   │   ├── amazon/              # Amazon UK browser scraping skill
│   │   ├── alibaba/             # Alibaba.com browser scraping skill
│   │   └── 1688/                # 1688 wholesale browser scraping skill
│   ├── preliminary-qualification/
│   ├── manifest-generation/
│   └── bid-portal-*/            # portal-specific bid automation skills
├── docs/
│   ├── research/
│   ├── specs/
│   └── runbooks/
└── infra/
    ├── supabase/                # Supabase config for shared sync target
    ├── monitoring/
    └── ci/
```

`apps/api` is a future-only escape hatch for a hosted broker surface and is intentionally absent from the MVP file map and milestone plan. `crates/common` and `packages/config` are also intentional later extractions, not MVP commitments; keep shared code in place until repetition justifies the split. The core shared packages (`packages/contracts-schema`, `packages/ui`) are boundary packages and should remain free of daemon-specific or Supabase-specific implementation details. Concretely: they must not import `packages/db`, desktop bridge code, daemon modules, or Supabase clients.

The primary implementation path in this document assumes **skill-based agent scraping for data retrieval** (procurement notices and supplier pricing), **API-based integration for cost estimation** (customs, freight, FX), and a **Rust daemon for PTY-based agent orchestration with local SQLite storage and Supabase sync**. The `packages/connectors/` directory from the original design is replaced by the skills system — each data source has a corresponding skill that the agent uses to scrape it via browser automation.

Only procurement/supplier retrieval and workflow playbooks live under `skills/`. The landed-cost engine itself remains in `packages/intelligence` because cost estimation is API-based and deterministic at its core, not an agent-scraping workflow.

### 3.5 Code Patterns to Follow

Because no project code exists, patterns should be created deliberately instead of discovered accidentally.

#### Pattern: Raw payload retention

Every external fetch should store:

- source name,
- endpoint or page URL,
- fetched timestamp,
- raw response payload or storage pointer,
- normalized projection,
- parse confidence and parse version.

This is essential because procurement notices change, supplier prices change, and AI decisions need replayability.

#### Pattern: Confidence-scored enrichment

Every enrichment result should store:

- value,
- method,
- evidence,
- confidence score,
- expiration/TTL,
- manual override fields.

If this is not built into the schema early, the platform will drift into fake precision.

#### Pattern: Orchestration state separate from agent output

Agent skills and manifests should be artifacts. The authoritative record for job status, retries, leases, and workflow transitions should stay in database tables. Name the queue generically, because most jobs are workflow jobs, not agent runs.

#### Pattern: Deterministic fallbacks

Use rule-based gating before AI:

- expiry date rule,
- procurement category rule,
- presence/absence of quantity/unit,
- duty/VAT calculation formulas,
- FX snapshot rules.

Reserve AI primarily for:

- goods vs services classification in ambiguous notices,
- product matching,
- extracting structured line items from messy text,
- generating human-readable manifests,
- browser interaction for bid flows.

### 3.6 Files to Study Before Implementation

There are no current implementation files to study. The practical equivalent is:

#### Must Read (Critical Context)

| File | Purpose | Key Sections |
|------|---------|--------------|
| `/Users/alinaaitova/Desktop/Testproject1/docs/research/2026-03-08-public-procurement-intelligence-platform-research.md` | Foundational system design | All |

#### Should Read (Important Context)

| File | Purpose |
|------|---------|
| `/Users/alinaaitova/.codex/skills/idea-to-research/SKILL.md` | Defines expected research depth and handoff quality |

### 3.7 Files to Modify

#### Definite Modifications

| File | Changes Needed | Impact |
|------|----------------|--------|
| `/Users/alinaaitova/Desktop/Testproject1/yolo` | Ignore during bootstrap; only delete during explicit repo cleanup | Low |

#### Likely Bootstrap Files

| File | Probable Changes | Confidence |
|------|------------------|------------|
| `/Users/alinaaitova/Desktop/Testproject1/package.json` | Workspace/bootstrap setup | High |
| `/Users/alinaaitova/Desktop/Testproject1/bunfig.toml` | Bun workspace/runtime settings | High |
| `/Users/alinaaitova/Desktop/Testproject1/Cargo.toml` | Rust workspace definition | High |

### 3.8 Potential Refactoring Needs

There is no technical debt in the app code because there is no app code yet. The real risk is different:

- if bootstrap is done without a clean package layout, refactoring pain will arrive immediately,
- if Supabase schema and RLS are improvised later, security and agent-write paths will become messy,
- if skills are written before workflow boundaries are modeled, the agent layer will become the architecture by accident.

---

## 4. External Technology Research

Scores in this section are rough decision aids, not statistically calibrated models. Unless otherwise stated, the total shown for each option is a simple equal-weight average of the listed criteria. Small score gaps should not be over-read; differences of a few tenths are directional, not mathematically significant.

### 4.1 Procurement Notice Ingestion Sources

#### Need

Reliable daily access to UK public procurement notices. Data retrieval is performed by skill-based agent scraping via browser automation, running on the local machine and orchestrated by the Rust daemon. The agent navigates to the official procurement sources and extracts notice data using source-specific skills.

#### Option A: Find a Tender OCDS API

**Official evidence:**

- Find a Tender publishes OCDS record and release package APIs.
- Release package endpoint supports `limit`, `cursor`, `updatedFrom`, `updatedTo`, and stage filters.
- Data is available under the Open Government Licence.

**Key references:**

- https://www.find-tender.service.gov.uk/Developer/Documentation
- https://www.find-tender.service.gov.uk/apidocumentation/1.0/GET-ocdsReleasePackages
- https://www.find-tender.service.gov.uk/apidocumentation/1.0/GET-ocdsRecordPackages

| Criterion | Score (1-5) | Notes |
|-----------|-------------|-------|
| Data structure | 5 | OCDS-native |
| Freshness support | 5 | `updatedFrom`/`updatedTo` + cursor |
| Licensing clarity | 5 | Open Government Licence |
| Coverage | 4 | High-value and new-regime notices; not whole UK on its own |
| Operational reliability | 4 | Official API, structured docs |
| **Average Score** | **4.6** | |

**Pros:**

- Official, structured, versioned API.
- Maps notice data to OCDS 1.1.5 with extensions.
- Supports incremental polling.
- Central post-2025 procurement regime source for England, Wales, Northern Ireland high-value flows.

**Cons:**

- Does not fully cover all below-threshold or Scotland-specific notice ecosystems by itself.
- Historical/transitional notices can span legacy platforms.

**Recommendation:** **RECOMMENDED**

Use as a primary ingestion target for the procurement-ingestion agent skill. The agent navigates to the Find a Tender API/portal and extracts notice data using the skill-defined playbook.

#### Option B: Contracts Finder OCDS/Search API

**Official evidence:**

- Contracts Finder exposes API help docs.
- Published/OCDS search supports `publishedFrom`, `publishedTo`, `stages`, `limit`, `cursor`.
- Daily CSV harvester endpoint exists and explicitly mentions rate limiting.
- Contracts Finder uses OCDS in its REST API and publishes data under government open standards.

**Key references:**

- https://www.contractsfinder.service.gov.uk/apidocumentation
- https://www.contractsfinder.service.gov.uk/apidocumentation/Notices/1/GET-Published-Notice-OCDS-Search
- https://www.contractsfinder.service.gov.uk/apidocumentation/Notices/1/GET-Harvester-Notices-Data-CSV-Daily
- https://www.gov.uk/government/publications/open-standards-for-government/open-contracting-data-standard-profile

| Criterion | Score (1-5) | Notes |
|-----------|-------------|-------|
| Data structure | 4 | OCDS available, plus CSV |
| Freshness support | 4 | date filters + cursor + daily export |
| Licensing clarity | 4 | aligned with open contracting/publication standards |
| Coverage | 4 | lower-value/public opportunities complement FTS |
| Operational reliability | 4 | official docs; rate limits exist |
| **Average Score** | **4.0** | |

**Pros:**

- Strong complement to Find a Tender.
- Supports daily and filtered retrieval.
- Useful for below-threshold and broader discovery.

**Cons:**

- API docs still reflect legacy evolution and mixed versions.
- Need rate-limit-aware polling.

**Recommendation:** **RECOMMENDED**

Use as a primary ingestion target for a dedicated agent skill alongside Find a Tender.

#### Option C: Public Contracts Scotland OCDS API

**Official evidence:**

- PCS exposes an OCDS Web API with notice list and notice family retrieval.
- Query supports output type, notice type, and date windows.

**Key references:**

- https://api.publiccontractsscotland.gov.uk/v1
- https://www.publiccontractsscotland.gov.uk/Default.aspx

| Criterion | Score (1-5) | Notes |
|-----------|-------------|-------|
| Data structure | 4 | OCDS available |
| Freshness support | 3 | month-based query model is weaker than FTS cursoring |
| Licensing clarity | 3 | official public API, but licensing should be re-confirmed in implementation |
| Coverage | 5 | essential for Scotland coverage |
| Operational reliability | 4 | official endpoint |
| **Average Score** | **3.8** | |

**Pros:**

- Required for true UK coverage.
- Official API rather than HTML scraping.

**Cons:**

- Separate legal/process ecosystem.
- Weaker incremental retrieval ergonomics than FTS.

**Recommendation:** **RECOMMENDED FOR FULL-UK SCOPE**

Include in the early ingestion wave only if Scotland is in initial scope. Otherwise defer it to a later commercial-intelligence milestone.

#### Option D: HTML scraping of notice portals

| Criterion | Score (1-5) | Notes |
|-----------|-------------|-------|
| Data structure | 1 | brittle |
| Freshness support | 2 | possible, but operationally noisy |
| Licensing clarity | 2 | page terms vary |
| Coverage | 3 | helpful only where API is absent |
| Operational reliability | 1 | selectors drift |
| **Average Score** | **1.8** | |

**Recommendation:** **SUBSUMED BY SKILL-BASED APPROACH**

With skill-based agent scraping as the primary architecture, HTML scraping is inherently handled by the agent's browser automation. Each source gets a dedicated skill rather than a generic HTML scraper.

#### Procurement Ingestion Recommendation

**Selected approach: Skill-based agent scraping**

1. `Find a Tender` ingestion skill — agent navigates FTS API/portal via browser automation.
2. `Contracts Finder` ingestion skill — agent navigates CF API/portal via browser automation.
3. Optional `Public Contracts Scotland` ingestion skill if full UK coverage is in scope.

All procurement data retrieval is constrained to the skill-based agent system running on the local machine. This simplifies the architecture: all data retrieval is based on browser-enabled agents running locally, powered by skills (easier to maintain, upgrade, and change) instead of maintaining separate API client packages for each data source.

**Ingestion is not the hard problem.** The official sources already expose structured incremental retrieval: FTS has OCDS release/record packages with `limit`, `cursor`, `updatedFrom`, `updatedTo`, and stage filters under the Open Government Licence; Contracts Finder similarly exposes `publishedFrom`, `publishedTo`, `stages`, `limit`, and `cursor` with documented rate limiting. Ingestion should be source-batched and incremental — one agent session per source per run, not one session per notice. The architecture's real complexity lives downstream in product identity, comparability, and pricing.

**Important architectural consequence:**

Each procurement source has a dedicated skill that defines the agent's scraping playbook. The daemon orchestrates execution via adapter-mediated agent sessions, which are typically PTY-backed for procurement retrieval. The underlying platform still provides:

- source registry (which skills to run and when),
- cursor/watermark tracking (persisted in local SQLite),
- raw payload retention,
- OCDS normalization,
- relation resolution and merge rules across sources.

This retrieval strategy is formalized in **ADR-001** and operationalized through the skill/runtime boundary in **ADR-005**.

### 4.2 Procurement Rules and Notice Semantics

#### What changed recently

Find a Tender states that for procurement processes starting on or after **24 February 2025**, notices specified by the Procurement Act 2023 apply, with transitional treatment for earlier processes and jurisdiction-specific exceptions.

**Official references:**

- https://www.find-tender.service.gov.uk/Home/NoticeTypes
- https://www.gov.uk/government/publications/procurement-act-2023-short-guides/central-digital-platform-factsheet-html
- https://www.gov.uk/government/publications/procurement-act-2023-guidance-documents-procure-phase/guidance-central-digital-platform-and-publication-of-information-html

#### Why this matters for the product

- Notice types changed after 24 February 2025.
- Expiry logic cannot depend on one legacy field name.
- Contract lifecycle stages are richer than “tender open” vs “closed”.
- Transitional notices may continue under earlier legislation.

#### Notice Relation Semantics

FTS's notice type documentation shows that linked notices share an OCID, but the relationship between notices is not always “same process → merge.” Specifically:

- **Linked notices (same OCID):** Updates, amendments, and award notices within the same procurement process share an OCID and should be merged into the same `ProcurementProcess` record.
- **Framework awards:** Framework establishment and subsequent call-offs follow different notice sequences. A framework award does not include a UK4 tender notice; the sequence differs from open-tender flows. Call-offs under an established framework are commercially separate opportunities even though they reference the parent framework.
- **Direct award after unsuccessful competition:** FTS explicitly treats this as a separate process with a different OCID. This is not a merge candidate — it is a genuinely new procurement process.
- **Related but separate:** A planning notice and a later tender notice from the same buyer for the same subject may have different OCIDs. These are related but not the same process.

This means the platform needs four persisted relation classifications, not two:

1. **`merged_same_process`** — same OCID or explicit linked notice chain.
2. **`related_not_merged`** — framework/call-off, planning/tender, or same-buyer same-subject but different OCIDs.
3. **`possible_duplicate`** — high-similarity pair held for operator review instead of forced merge.
4. **`separate`** — no meaningful relationship.

See §5.6 for the relation resolution algorithm.

#### Access Route Classification

Not all procurement notices represent actionable opportunities. The platform should classify each notice's access route early — before pricing — to suppress non-bidable opportunities:

| Access Route | Description | Actionability |
|---|---|---|
| `open_tender` | Open competitive procedure; any qualifying supplier can bid | Fully actionable |
| `restricted_tender` | Restricted procedure; requires pre-qualification/shortlisting | Actionable if pre-qualification route is open |
| `framework_call_off` | Call-off under an established framework | Actionable only if you are on the framework |
| `dynamic_market` | Dynamic Purchasing System / Dynamic Market (Procurement Act 2023) | Actionable if you can join the DPS/DM |
| `direct_award` | Direct award without competition | Not actionable |
| `closed_framework_award` | Award under closed framework | Not actionable unless on framework |
| `not_bidable` | Notice type does not support supplier participation | Not actionable |

**Important:** DPS/Dynamic Market notices do not “expire” in the same way as standard tenders — they are open-ended. The 30-day expiry filter should not apply to DPS notices that accept new supplier applications.

#### Lot Structure in Notices

Many UK procurement notices contain multiple lots. A lot — not the notice — is the unit that determines:

- bidability (lots may have different access routes),
- CPV specificity (lots often have more specific CPV codes than the notice-level code),
- value (each lot has its own estimated value),
- deadline (lots may have different submission deadlines),
- technical scope (goods vs services vs works may differ per lot),
- whether goods are separable from services/works.

The platform must extract lots from notices and treat each lot as a separate qualification and pricing unit. If a notice has no explicit lots, create a synthetic whole-notice lot. See §5.4 for the `NoticeLot` entity and §5.7 for lot extraction in the intelligence pipeline.

#### Document Attachments

In UK procurement, the notice body is often the **least useful product document**. The real signal for product extraction lives in:

- specification PDFs,
- pricing schedules,
- lot documents,
- bills of quantities (BoQ) spreadsheets,
- tender appendices.

The platform should attempt to acquire publicly accessible tender documents before product extraction. Without this, the extractor will turn vague notice summaries into garbage product candidates. See §5.7 for the document acquisition stage.

#### Recommendation

Normalize notice lifecycle into internal stages:

- `pipeline`
- `planning`
- `tender_open`
- `award`
- `contract`
- `termination`

Then derive operator-facing eligibility rules from normalized state, not from raw portal-specific labels. Classify access route early in the pipeline to suppress non-actionable notices before pricing.

### 4.3 Supplier Price Intelligence Sources

#### Need

Retrieve or infer comparable market prices for goods in:

- UK/EU-facing marketplaces,
- China-oriented supplier marketplaces,
- using skill-based agent scraping via browser automation on the local machine.

All supplier pricing data retrieval uses the same skill-based agent architecture as procurement ingestion. The agent navigates to marketplace websites using source-specific skills and extracts pricing data. No API client packages are maintained for supplier sources.

#### Category A: eBay Browse API

**Official evidence:**

- Browse API supports keyword, GTIN, category, and item search.
- Item detail responses include price, shipping, seller, availability, and estimated delivery.
- Release notes show ongoing maintenance and added shipping-estimate support in 2025.

**References:**

- https://developer.ebay.com/api-docs/buy/static/api-browse.html
- https://developer.ebay.com/api-docs/buy/browse/static/overview.html
- https://developer.ebay.com/api-docs/buy/browse/resources/item/methods/getItem
- https://developer.ebay.com/api-docs/buy/browse/release-notes.html

| Criterion | Score (1-5) | Notes |
|-----------|-------------|-------|
| Search quality | 4 | keyword + category + GTIN |
| Structured pricing | 4 | strong item detail support |
| Maintenance | 5 | active release notes |
| UK relevance | 4 | marketplace support is strong |
| Integration risk | 3 | some APIs are partner-gated depending on use case |
| **Average Score** | **4.0** | |

**Pros:**

- Strong structured API surface.
- Good fit for spot price baselines and resale-market checks.
- Rich item metadata.

**Cons:**

- Not ideal for wholesale benchmarking.
- Marketplace/partner approval details must be validated during implementation.

**Recommendation:** **RECOMMENDED**

Use for UK retail/resale pricing baselines via agent skill-based browser scraping.

#### Category B: Amazon Product Advertising API / Creators API Transition

**Official evidence:**

- Amazon states Product Advertising API 5.0 will be deprecated on **30 April 2026**.
- The documentation is explicitly marked outdated.
- New customers are directed to Creators API.
- Amazon updated Associates terms in late 2025 to include Creators API and new agent-related terms.

**References:**

- https://webservices.amazon.com/paapi5/documentation/
- https://webservices.amazon.com/paapi5/documentation/register-for-pa-api.html
- https://affiliate-program.amazon.com/help/operating/compare

| Criterion | Score (1-5) | Notes |
|-----------|-------------|-------|
| Search quality | 4 | historically strong |
| Structured pricing | 4 | strong catalog detail |
| Maintenance | 2 | PA-API sunsets on 2026-04-30 |
| UK relevance | 5 | very high |
| Integration risk | 2 | migration and policy risk |
| **Average Score** | **3.4** | |

**Pros:**

- Amazon pricing is commercially useful for UK baselines.
- Historically mature API behavior.

**Cons:**

- PA-API is effectively legacy now.
- New integrations should not be designed around a surface that ends on 30 April 2026.
- Access is tied to Amazon affiliate ecosystem rules and changing API terms.

**Recommendation:** **USE WITH TRANSITION PLAN**

Amazon pricing can be collected via agent skill-based browser scraping, avoiding the PA-API sunset risk entirely. If official API access is desirable later, isolate behind a generic skill interface.

#### Category C: Taobao Global Open Platform / Alibaba Open APIs / 1688 APIs

**Official evidence:**

- Taobao Global Open Platform exists and exposes seller/reseller-oriented APIs for products, orders, logistics, and settlements.
- Alibaba.com Open API exists for product/order synchronization.
- 1688-related APIs exist in the Taobao/Alibaba ecosystem, but they are auth-heavy and oriented toward distributor/seller workflows.

**References:**

- https://open.taobao.global/doc/doc.htm
- https://open.taobao.global/doc/api.htm
- https://developer.alibaba.com/docs/doc.htm?articleId=118416&docType=1&treeId=684
- https://open.alitrip.com/docs/api.htm?apiId=29432

| Criterion | Score (1-5) | Notes |
|-----------|-------------|-------|
| Search quality | 2 | not a clean public catalog search solution for arbitrary procurement matching |
| Structured pricing | 3 | available in some scenarios, but partner/auth complexity is high |
| Maintenance | 4 | official platform docs exist |
| UK relevance | 4 | strong for China sourcing |
| Integration risk | 1 | high auth, partner, language, and workflow complexity |
| **Average Score** | **2.8** | |

**Pros:**

- Official channels exist.
- Better long-term legitimacy than raw scraping.
- Strong fit for supplier-side workflows when access is granted.

**Cons:**

- Not a plug-and-play public price intelligence API.
- Many endpoints assume seller/distributor relationships or credentials.
- Operational complexity is high.

**Recommendation:** **SKILL-BASED SCRAPING**

Chinese sourcing intelligence is collected via agent skill-based browser scraping:

1. Agent navigates to Alibaba/1688/Taobao using dedicated skills.
2. Skills handle authentication flows where needed.
3. All captured data includes confidence scoring and sourcing method.

The skill-based approach eliminates the need to manage complex API auth flows — the agent handles website interactions just as a human would, guided by the skill playbook.

#### Supplier Price Intelligence Recommendation

Use a **skill-based supplier-source model**:

1. UK sources: eBay, Amazon — dedicated browser scraping skills.
2. China sources: Alibaba, 1688 — dedicated browser scraping skills.
3. All collection via agent skills with browser automation on the local machine.

Do not flatten these into one “price” field. Every quote needs:

- source,
- region,
- quote class,
- quoted quantity and unit,
- quantity basis,
- unit assumptions,
- MOQ assumptions,
- shipping included or excluded,
- listing pack quantity and unit (the listing's native basis),
- normalized unit basis (the common basis used for comparison),
- VAT-inclusive flag,
- used/refurbished flag,
- confidence (split: see below),
- validity window.

The persisted shapes for this layer are defined in §5.4 under `SearchProfile`, `SearchAttempt`, `ListingEvidence`, and `SupplierQuote`.

**Price-signal role classification:** Marketplace listings serve different roles depending on source:

| Source | Typical Price-Signal Role | Implication |
|---|---|---|
| Amazon/eBay UK | Benchmark ceiling — UK retail/resale price, usually VAT-inclusive | Establishes the contract-revenue proxy or upper-bound UK market price |
| Alibaba | Optimistic source-side floor — listed MOQ prices, often ex-works/FOB | Lower bound for supplier cost; actual procurement-scale price will differ |
| 1688 | Domestic China wholesale floor — even more optimistic | Useful for commodity verification but not directly executable from UK |
| RFQ/negotiated | Executable or near-executable quote | Decision-grade pricing |

The system should compute a **margin band** (floor to ceiling) rather than fake precision from one UK listing and one China listing.

**Confidence split:** Every `SupplierQuote` should carry two separate confidence scores:

- `matchConfidence` — how well the marketplace listing matches the product candidate (product identity).
- `quoteUsabilityConfidence` — how commercially usable the observed price is (correct unit basis, relevant MOQ, shipping visibility, listing completeness).

A listing can be a strong product match and still be a weak pricing signal (right product, wrong pack basis), and vice versa (usable market price range, but only a near-match). These should not be averaged for control flow.

**Compliance evidence tiers:** For `regulated_goods` and `standard_class_led` archetypes, raw confidence scores are insufficient. Each supplier quote carries a `complianceEvidenceTier` that classifies the strength of compliance evidence found on the marketplace listing:

| Tier | Evidence Source | Matching Implication |
|---|---|---|
| `structured_attribute` | Standards/certifications appear in the listing's structured spec table or attribute fields | Strong — eligible for automatic match |
| `title_body_mention` | Standards/certifications mentioned in title or body text only | Moderate — eligible for automatic match with confidence penalty |
| `absent` | No compliance evidence found on listing | No automatic match for `regulated_goods`; confidence cap for other types |

The compliance evidence tier is determined during the detail-page enrichment step (see §5.7), not during the initial search-card screen. Missing compliance on the search card alone does not disqualify a result — many compliant listings omit standards from the title but include them in the detail page.

**Lexicographic gating for dual confidence:** The `matchConfidence` and `quoteUsabilityConfidence` scores drive downstream decisions through lexicographic gating — identity gates first, then commercial usability, then price-signal role:

| matchConfidence | quoteUsabilityConfidence | priceSignalRole | Action |
|---|---|---|---|
| High (≥ 0.75) | High (≥ 0.70) | any | Landed-cost candidate |
| High (≥ 0.75) | Medium (0.45–0.69) | `benchmark_ceiling` / `optimistic_floor` | Screening-only price signal |
| High (≥ 0.75) | Low (< 0.45) | any | Identity evidence only, not pricing |
| Medium (0.50–0.74) | High (≥ 0.70) | any | AI/manual review of identity |
| Low (< 0.50) | any | any | Reject |

**Regulated override:** For `regulated_goods`, the lexicographic gate additionally requires `complianceEvidenceTier` of `structured_attribute` or `title_body_mention` before automatic match acceptance, regardless of confidence scores.

**Archetype-specific confidence floors:** Global thresholds produce mediocre results across diverse product categories. `matchArchetype` drives differentiated identity floors:

| Match Archetype | Identity Floor | Key Requirement |
|---|---|---|
| `model_exact` | Very high (≥ 0.85) | Exact brand/model match required |
| `standard_class_led` | High (≥ 0.75) | Explicit compliance evidence required |
| `compatibility_led` | High (≥ 0.75) | Parent product reference must resolve |
| `electrical_spec_led` | High (≥ 0.70) | Mandatory electrical specs must match |
| `dimension_material_led` | Medium-high (≥ 0.65) | Key dimensions within tolerance window |
| `pack_count_led` | Medium (≥ 0.55) | Higher pack/unit usability requirements |

**Source routing by candidate type:** Not every product candidate should be searched on every source. Route by `candidateType` (see §5.7):

| Candidate Type | UK Sources | China Sources | Notes |
|---|---|---|---|
| `exact_sku` | eBay, Amazon (primary) | Only if “equivalent” allowed and no OEM lock | SKU/GTIN search |
| `standard_commodity` | eBay, Amazon | Alibaba, 1688 | Full search |
| `regulated_goods` | eBay, Amazon | Alibaba, 1688 only when standards/compliance slots populated | Must verify compliance equivalence |
| `brand_family` | eBay, Amazon | Alibaba only if generic equivalent exists | Brand-specific search |
| `bundle` | Decompose first | Decompose first | Search top 2–3 sub-items only |
| `custom_or_fabricated` | Skip marketplaces | Skip marketplaces | RFQ/manual only |
| `installed_goods` | Skip marketplaces | Skip marketplaces | RFQ/manual only |

This routing stops the search explosion: instead of 4 sources × N candidates, the system only hits sources where results are meaningful.

**Match archetype classification:** `candidateType` describes the commercial/procurement shape of a product candidate. `matchArchetype` describes how product identity is recovered on marketplaces — the matching strategy axis. These are orthogonal: a `standard_commodity` could be matched by standards compliance, by physical dimensions, by electrical specifications, by pack count, by compatibility reference, or by exact model number. The `matchArchetype` drives query formulation templates, hard-conflict weighting, confidence floor profiles, and shortlist budgets.

| Match Archetype | Identity Recovery Strategy | Example Products |
|---|---|---|
| `standard_class_led` | Standards family + class + compliance evidence dominate identity | PPE, medical consumables, workwear, fire safety equipment |
| `dimension_material_led` | Physical dimensions + material + fire/load rating dominate | Desks, lockers, bins, shelving, racking, panel systems |
| `electrical_spec_led` | Voltage, wattage, plug type, IP rating, connector dominate | Work lights, PSUs, chargers, switchgear, cable |
| `pack_count_led` | Unit/pack basis + material + disposability dominate | Gloves, wipes, bags, cable ties, bin liners |
| `compatibility_led` | Parent product reference + part number + compatibility chain | Filters, toner, printer parts, replacement blades, fittings |
| `model_exact` | Brand + model number + generation/revision | IT equipment, branded power tools, instruments |

The product extraction stage assigns `matchArchetype` alongside `candidateType`. If extraction cannot determine the archetype, it defaults to `unknown`, which triggers generic matching with wider shortlist budgets.

**Source-specific search behavior:** Each marketplace has distinct search semantics. The query planner adapts per source:

- **eBay:** Search supports keyword, GTIN, category, and aspect-based refinement. The planner derives a UK category prior (using eBay's category suggestion from the head noun) and searches within it for cleaner results. eBay's Browse stack defaults to fixed-price listings, which produces cleaner ceiling-price signals than auction results. Queries should be short title-like phrases, typically 3–7 tokens.
- **Amazon:** Product titles are constrained to be concise and non-repetitive. Amazon's search rewards synonyms, spelling variations, and abbreviations over verbose descriptions. The planner generates **alias families** rather than procurement text — multiple short noun-phrase variants of the same product. Queries should target the structured title vocabulary, not specification paragraphs. The PA-API 5.0 deprecation (30 April 2026) reinforces the browser-first strategy.
- **Alibaba:** Search is filter-heavy and sourcing-oriented. Category, supplier type, MOQ range, shipping option, certifications, price range, and supplier location are first-class search parameters, not just post-processing. "Ready to Ship" listings (upfront shipping, fixed price, fast dispatch) deserve a `quoteUsabilityConfidence` bump over generic custom/OEM listings. The planner uses structured filters alongside keyword queries.
- **1688:** Search is Chinese-keyword-only. Short keyword clusters (2–4 terms), not translated prose. Results serve primarily as commodity existence signals and domestic wholesale floor prices. For regulated goods, 1688 results without explicit compliance/export evidence should not drive landed-cost decisions.

**Slot criticality model:** Each extracted attribute on a `ProductCandidate` carries a criticality role that governs how it participates in matching:

| Role | Meaning | Matching Behavior |
|---|---|---|
| `required` | Tender mandates this attribute; absence or conflict is disqualifying | Hard reject on conflict; review-flag on absence |
| `preferred` | Tender mentions preference but does not mandate | Confidence penalty on conflict; no penalty on absence |
| `descriptive` | Informational only; does not affect commercial suitability | No matching impact; stored for evidence |
| `exclusion` | Explicitly excluded attribute or product form | Hard reject on presence |

Slot criticality is determined during product extraction using tender language signals ("must", "shall", "required" → `required`; "preferred", "desirable" → `preferred`; "not", "excluding" → `exclusion`). When ambiguous, the extraction stage defaults to `preferred` rather than `required` to avoid false rejections. The `matchArchetype` provides category-level defaults: for `standard_class_led` products, standards slots default to `required`; for `dimension_material_led`, key dimension slots default to `required`.

**Cross-language matching strategy:** Do not translate whole procurement sentences into Chinese and search them. Translate structured slots **plus market framing and use context**. Pure slot translation fails when the marketplace term is not a literal translation of the procurement noun — procurement says "illumination equipment" while marketplaces say "LED work light"; PPE is often framed as `劳保用品` (labor protection supplies) rather than literal item translation; sector-specific consumables carry different market framing in medical vs janitorial vs food-prep contexts.

Core rules:

1. Normalize the product candidate into structured slots (head noun, critical attributes, standards, numeric specs).
2. Expand English synonyms for marketplace search terms, including marketplace-native vocabulary (not just procurement vocabulary).
3. Translate the head noun + critical modifiers + **market framing context** + standards-relevant descriptors to Chinese.
4. Keep numerics and standards codes unchanged (EN 397, EN 455, dimensions, voltages are universal anchors).
5. Do **not** translate prose, do **not** rely on machine-translated paragraphs.
6. Run dual-language queries for China sources: English search first on Alibaba (broad recall), Chinese search on 1688 (where listings are primarily in Chinese).
7. **Chinese retry on Alibaba:** When English search on Alibaba returns zero results, returns only accessories, or returns clearly consumer/non-industrial items, retry with Chinese queries on Alibaba (many Alibaba listings have both English and Chinese content). This is a recovery step, not a first-pass strategy.

**Market framing and use context slots:** In addition to literal noun translation, each search profile carries:

- `marketFrameEn` — English marketplace framing terms (e.g., "industrial", "commercial", "heavy duty", "professional")
- `marketFrameZh` — Chinese marketplace framing terms (e.g., `劳保` for PPE, `工地` for construction, `工业` for industrial)
- `useContext` — end-use context that disambiguates otherwise identical products (e.g., "medical", "janitorial", "food prep" for disposable gloves)
- `sectorContext` — industry sector that affects both terminology and compliance expectations

For PPE, Chinese search includes both the item noun and the industrial frame: a helmet query uses not just `安全帽` but also variants like `劳保 安全帽` and `工地 安全帽` when the lot is construction-related. For medical consumables, sector context determines whether `检查 手套 丁腈 无粉` (examination glove nitrile powder-free) or `一次性 丁腈 手套` (disposable nitrile glove) is the right market framing.

**Practical lexicon:** Do not build a grand taxonomy. The bilingual layer is a small curated lexicon per CPV/product family — English marketplace aliases, Chinese marketplace aliases (including market-frame variants), critical attributes, exclusion vocabulary. This is a search aid, not a taxonomy.

For MVP, seed the **top 30–50 spend-heavy searchable product families**. For each family, maintain roughly:

- 5–15 EN noun aliases (marketplace vocabulary, not procurement vocabulary)
- 5–15 ZH noun aliases / market-frame aliases
- 5–20 negative terms
- 3–10 unit/pack aliases
- a few standard/class patterns

That is **hundreds of curated entries**, not tens of thousands.

**Lexicon maintenance model:** Seed manually from procurement category knowledge. Then bootstrap from operational data: accepted matches contribute confirmed aliases, rejected false positives contribute negative terms, repeated zero-result cases identify vocabulary gaps, operator edits refine framing. An alias is promoted into the production lexicon only when it is operator-confirmed or repeatedly associated with accepted matches.

**Bilingual unit dictionary:** For quantity and MOQ handling, maintain a bilingual unit normalization dictionary:

- `件 / 个 / 只 / 双 / 套 / 箱 / 包 / 卷 / 瓶 / 桶 / 台`

Normalize Chinese marketplace units to the candidate's procurement unit. This is where gloves, wipes, cable ties, bin liners, and workwear quantity comparisons go wrong. Do not search the tender quantity — search the product identity; use quantity later for tier-price selection, MOQ compatibility, and quote realism assessment.

For large tenders, marketplace listings should usually be treated as **indicative signals**, not executable procurement offers. Once an opportunity survives early filtering, the system should escalate toward RFQs, negotiated quotes, or supplier-confirmed volume pricing before treating margin as decision-grade.

### 4.4 Shipping and Logistics Estimation

#### Need

Estimate transport cost from origin to UK with different shipment modes:

- parcel/courier,
- air freight,
- LCL ocean,
- FCL ocean,
- domestic final-mile or pallet delivery.

#### Option A: Freightos APIs / Freightos Terminal

**Official evidence:**

- Freightos API exposes price statistics endpoints for route-level freight data.
- Freightos documentation highlights route, mode, load, and confidence levels.

**References:**

- https://developers.freightos.com/apis
- https://apidocs.freightos.com/reference/get_price-stats
- https://apidocs.freightos.com/reference/getpricestats-1

| Criterion | Score (1-5) | Notes |
|-----------|-------------|-------|
| Freight relevance | 5 | designed for freight, not just parcel labels |
| Route intelligence | 5 | route-based pricing and confidence |
| Parcel coverage | 2 | not the main use case |
| Bulk-contract usefulness | 5 | very strong |
| Integration risk | 3 | commercial access and quota considerations |
| **Average Score** | **4.2** | |

**Pros:**

- Best fit for large-volume China-to-UK freight estimation.
- Confidence metrics are valuable for downstream margin scoring.
- Route-level freight market data is materially better than parcel-label APIs for big contracts.

**Cons:**

- Not a complete landed-cost engine on its own.
- Requires route and load assumptions.

**Recommendation:** **RECOMMENDED**

Use as the primary freight benchmark source for air/ocean.

#### Option B: Easyship APIs

**Official evidence:**

- Easyship exposes tax-and-duty calculation endpoints.
- Shipping APIs are strong for ecommerce shipping scenarios.

**References:**

- https://developers.easyship.com/reference/taxes_and_duties_calculate

| Criterion | Score (1-5) | Notes |
|-----------|-------------|-------|
| Freight relevance | 2 | more parcel/ecommerce oriented |
| Parcel coverage | 5 | strong |
| Customs add-on value | 4 | helpful complementary signal |
| Bulk-contract usefulness | 2 | weak for containers/palletized freight |
| Integration risk | 3 | commercial integration still required |
| **Average Score** | **3.2** | |

**Recommendation:** **SUPPORTING SOURCE**

Use for small shipments, sample shipments, and as a secondary customs sanity check, not as the core estimator for large tenders.

#### Option C: Shippo API

**Official evidence:**

- Shippo offers normalized carrier rates, labels, and international shipping workflows.
- International shipping docs emphasize customs declarations and parcel-level shipment creation.

**References:**

- https://docs.goshippo.com/
- https://docs.goshippo.com/shippoapi/public-api
- https://docs.goshippo.com/docs/international_shipping/internationalshipping

| Criterion | Score (1-5) | Notes |
|-----------|-------------|-------|
| Freight relevance | 2 | parcel-first |
| Parcel coverage | 5 | strong |
| Customs documentation support | 3 | useful |
| Bulk-contract usefulness | 1 | not sufficient for large goods tenders |
| Integration risk | 4 | mature developer surface |
| **Average Score** | **3.0** | |

**Recommendation:** **SUPPORTING SOURCE**

Useful for parcel-mode estimates, samples, and accessory shipments. Not sufficient for containerized landed-cost modeling.

#### Option D: SeaRates Logistics Explorer API

**Official evidence:**

- SeaRates provides a GraphQL API at `rates.searates.com/graphql`.
- Covers FCL, LCL, air freight, and land/road transport across 190+ countries.
- Automatic mode detection based on weight and locations (`Get Auto Rates` endpoint).
- Additional capabilities: container tracking, ship schedules, load calculator, carbon emissions, distance & time, freight index.
- Active development with ongoing updates (Feb 2026 release notes).

**References:**

- https://docs.searates.com/reference/logistics/get-rates

| Criterion | Score (1-5) | Notes |
|-----------|-------------|-------|
| Freight relevance | 5 | covers FCL, LCL, air, road |
| Route intelligence | 4 | auto mode detection, 190+ countries |
| Parcel coverage | 2 | not the main use case |
| Bulk-contract usefulness | 4 | strong for multimodal and LCL routes |
| Integration risk | 3 | GraphQL API, commercial access |
| **Average Score** | **3.6** | |

**Pros:**

- Complementary coverage to Freightos, especially for LCL and multimodal routes.
- Auto-rate mode detection simplifies mode selection logic.
- Additional tools (load calculator, carbon emissions) add value.

**Cons:**

- GraphQL API has a learning curve compared to REST.
- May overlap with Freightos for FCL/air.

**Recommendation:** **RECOMMENDED AS SECONDARY**

Use alongside Freightos for complementary coverage, especially for LCL and multimodal route estimation.

#### Freightos API Status Warning

Freightos's developer landing page indicates its APIs are being rebuilt (as of 2025) and are currently unsupported, even though the Price Stats documentation remains live and describes route-level USD statistics, rate limits, and native confidence levels based on data-point counts. Treat Freightos as the preferred freight data source where available, but do not treat it as a single operational dependency. A provider-abstracted freight layer is required.

#### HMRC Incidental Expense Simplifications (VAT Valuation Only)

HMRC publishes simplified nationally agreed rates for incidental transport expenses when actual costs are unknown:

- **Air freight:** 40p per air kilo (minimum £100)
- **Ocean/groupage:** £90 per gross-weight tonne plus £80 for groupage/consolidation (minimum £170)
- **Full load:** £550 per full load

These are for **import VAT valuation purposes only** — they determine the VAT base when actual incidental expenses are unknown. They must never be used as commercial freight quotes for margin estimation.

**Reference:** https://www.gov.uk/guidance/working-out-the-vat-value-using-the-customs-value-of-the-imported-goods

#### Shipping Recommendation

Build a **provider-abstracted, mode-aware logistics estimator**:

- `parcel` -> Easyship/Shippo (only for clearly courier-sized consignments, not as the fallback for unknowns)
- `air_freight` -> Freightos + SeaRates (for urgency/high-value/small-consignment cases)
- `lcl_ocean` -> Freightos + SeaRates (SeaRates has strong LCL coverage)
- `fcl_ocean` -> Freightos + SeaRates
- `uk_final_mile` -> internal rate cards / carrier quotes

Never use one API for all logistics cases. Treat Freightos as the preferred freight benchmark where available, and SeaRates as complementary coverage. Both providers expose native confidence metadata (Freightos: data-point counts; SeaRates: indicative rate flags) — use provider-native confidence signals rather than homemade guesses alone.

#### Mode Selection Decision Tree

Before selecting freight mode, resolve the **Incoterm cost boundary** from the supplier quote. If the quote is DDP, UK duty/VAT are already included in supplier cost; if FOB, inland freight to origin port is included but ocean freight is not. Different Incoterms change which cost lines the landed-cost engine must add separately.

**Hard mode locks (override weight/volume rules):**

Apply before any economic mode selection. Hazmat, oversized, fragile, reefer/temperature-controlled, lithium batteries, or installation-sensitive goods must override pure weight/volume routing.

**MVP mode selection heuristics when weight/volume data is sparse:**

1. **Infer packaging state.** Flat-pack vs assembled matters more than raw quantity for furniture and equipment. If packaging state is unknown, run **dual scenarios** (compact-pack and bulky-pack) and gate on the worse landed margin.
2. **Parcel:** Only for clearly courier-sized consignments (single items under ~30 kg, under 1.5 m longest side). Not the default for unknowns.
3. **Air freight:** For urgency, high value-to-weight, or small total consignment cases. Not the default for bulk.
4. **Ocean (LCL vs FCL):** Default for ambiguous bulky goods. Run both LCL and FCL scenarios when volume is uncertain. For furniture (cube-first goods), always default to ocean scenarios.
5. **Mixed mode:** Only when the basket naturally splits — e.g., accessories via parcel, main consignment via ocean.
6. **Always emit `freightLowGbp` / `freightBaseGbp` / `freightHighGbp`** and a separate `modeConfidence` score.

**Stress test — "500 office chairs + 200 desks":** Furniture is cube-first. If pack state is unknown, run flat-pack and assembled scenarios. Default to ocean (LCL + FCL comparison). Gate on the worse landed margin. Do not allow parcel as the default.

#### Freight vs VAT-Base Separation

Separate **commercial freight pricing** (what you actually pay the carrier) from **VAT-base transport estimation** (what HMRC requires for import VAT calculation). HMRC says import VAT value must add incidental expenses to the first UK destination, plus duty and other charges. Where actual incidental expenses are unknown, use HMRC's simplified nationally agreed rates (above) only for the VAT valuation calculation, never as commercial freight estimates.

### 4.5 Customs, Tariff, and Commodity Classification

#### Need

Accurately estimate duty and VAT in the UK, with explainable linkage to commodity code and valuation method.

#### Option A: HMRC Trade Tariff APIs + HMRC valuation guidance

**Official evidence:**

- HMRC Trade Tariff API provides commodity, duty, VAT, quota, and measure data.
- HMRC guidance explains customs value and import VAT valuation methods.
- HMRC provides Advance Tariff Ruling guidance for legally binding classification decisions.

**References:**

- https://hub.trade-tariff.service.gov.uk/
- https://api.trade-tariff.service.gov.uk/getting-started.html
- https://api.trade-tariff.service.gov.uk/the-commodities-api.html
- https://www.gov.uk/trade-tariff
- https://www.gov.uk/guidance/prepare-to-work-out-the-customs-value-of-your-imported-goods
- https://www.gov.uk/guidance/valuing-imported-goods-using-method-1-transaction-value
- https://www.gov.uk/guidance/working-out-the-vat-value-using-the-customs-value-of-the-imported-goods
- https://www.gov.uk/guidance/apply-for-an-advance-tariff-ruling

| Criterion | Score (1-5) | Notes |
|-----------|-------------|-------|
| Authoritativeness | 5 | official UK source |
| Explainability | 5 | full trace to code/measures/guidance |
| Coverage | 5 | tariff + VAT + measures |
| Implementation effort | 3 | you must build the engine yourself |
| Accuracy ceiling | 5 | highest available |
| **Average Score** | **4.6** | |

**Recommendation:** **MANDATORY CORE**

This should be the canonical customs engine.

#### Option B: HMRC FPO (Fast Parcel Operator) API

**Official evidence:**

- Official HMRC API that accepts English-language product descriptions and returns candidate commodity codes.
- Free to use for registered organizations.
- Uses ML model trained on tariff data (source: `github.com/trade-tariff/trade-tariff-lambdas-fpo-search`).
- Originally for Windsor Framework GB→NI parcels but the classification logic is broadly applicable.
- Available via HMRC Trade Tariff Developer Portal.

**References:**

- https://hub.trade-tariff.service.gov.uk/
- https://api.trade-tariff.service.gov.uk/fpo.html

| Criterion | Score (1-5) | Notes |
|-----------|-------------|-------|
| Authoritativeness | 5 | official HMRC, ML-trained on tariff data |
| Explainability | 4 | returns candidate codes with rationale |
| Coverage | 4 | broad commodity classification from text descriptions |
| Implementation effort | 5 | REST API, free, straightforward |
| Accuracy ceiling | 4 | strong first-pass, needs human review for edge cases |
| **Average Score** | **4.4** | |

**Pros:**

- Official HMRC source with ML-powered classification.
- Accepts plain English product descriptions — ideal for automated first-pass HS code classification.
- Free and accessible.

**Cons:**

- Originally designed for Windsor Framework GB→NI parcels; accuracy on procurement-scale industrial goods (Chapter 84 machinery, Chapter 90 medical instruments, Chapter 94 furniture) may be weaker than on consumer goods.
- Returns only 6-digit HS or 8-digit CN candidates with a match score. The score is retrieval relevance, not legal classification confidence — HMRC's own public repo example shows kitchen towels ranking below textile-towel codes despite eventually matching paper-towel codes.
- Does not return 10-digit declarable commodity codes. The UK tariff's legal duty treatment, additional codes, measure conditions, and specific measurement units live at the 10-digit commodity layer.
- Access may be restricted to UK Carrier-scheme FPOs invited to test. Verify access availability before depending on this as the primary automated classifier.
- Should be treated as a candidate generator, not a final classifier or authority.

**Recommendation:** **STRONGLY RECOMMENDED AS CANDIDATE GENERATOR**

Use as the primary automated first-pass HS code **candidate generator**, not the final classifier. Feed extracted product descriptions into the FPO API to get candidate commodity codes (HS6/CN8), then resolve candidates to 10-digit declarable codes via HMRC Trade Tariff API and validate against Taric Support and HS Ping. The FPO match score indicates retrieval relevance, not classification confidence — downstream resolution and measure scanning determine actual classification quality.

#### Option C: HS Ping

**Official evidence:**

- Commercial HS/HTS code lookup API using official data sources.
- Versioned responses for auditability.

**References:**

- https://hsping.com

| Criterion | Score (1-5) | Notes |
|-----------|-------------|-------|
| Authoritativeness | 3 | uses official sources, commercial wrapper |
| Explainability | 3 | code lookup with descriptions |
| Coverage | 4 | HS and HTS codes |
| Implementation effort | 4 | REST API |
| Accuracy ceiling | 3 | validation/cross-check, not primary classification |
| **Average Score** | **3.4** | |

**Recommendation:** **SUPPORTING SOURCE**

Use as a validation/cross-check source against HMRC FPO classifications.

#### Option D: Taric Support API

**Official evidence:**

- All-in-one customs compliance API for UK and EU.
- HS code browsing, validation, description retrieval, suggestions for incomplete codes.
- Real-time tariff data.

**References:**

- https://taricsupport.com/api/

| Criterion | Score (1-5) | Notes |
|-----------|-------------|-------|
| Authoritativeness | 4 | real-time tariff data from official sources |
| Explainability | 4 | browsable code hierarchy, validation, suggestions |
| Coverage | 4 | UK and EU tariff data |
| Implementation effort | 4 | REST API |
| Accuracy ceiling | 4 | strong for code validation and completeness checking |
| **Average Score** | **4.0** | |

**Recommendation:** **RECOMMENDED FOR VALIDATION**

Use for code validation, completeness checking, and UK/EU tariff data cross-referencing.

#### Option E: Third-party duty/tax estimation APIs

Easyship and similar services can provide convenience estimates, but they are not authoritative in the same way as HMRC tariff + valuation logic for UK imports.

| Criterion | Score (1-5) | Notes |
|-----------|-------------|-------|
| Authoritativeness | 2 | third-party abstraction |
| Explainability | 2 | usually black-box-ish |
| Coverage | 3 | useful convenience layer |
| Implementation effort | 4 | easier |
| Accuracy ceiling | 3 | acceptable for rough estimates, weaker for borderline deals |
| **Average Score** | **2.8** | |

**Recommendation:** **SECONDARY ONLY**

Use only as a comparison or fallback signal.

#### Option F: Third-party Landed Cost Calculator Services

For comparison and validation against the internal engine:

**Dutify**

- AI-powered product classification + landed cost calculator.
- API available for duty and tax calculation in one call.
- Reference: https://docs.dutify.com/docs/landed-cost-calculator

| Criterion | Score (1-5) | Notes |
|-----------|-------------|-------|
| Authoritativeness | 2 | commercial wrapper, not legal authority |
| Explainability | 3 | better than most black-box tools, still secondary |
| Coverage | 4 | classification + duty/tax in one service |
| Implementation effort | 4 | easy comparison integration |
| Accuracy ceiling | 3 | useful comparison signal, not canonical |
| **Average Score** | **3.2** | |

**Recommendation:** **USE AS COMPARISON SIGNAL**

**Zonos**

- Cross-border duty/tax calculation API.
- $299+/month, instant setup.
- Reference: https://zonos.com

| Criterion | Score (1-5) | Notes |
|-----------|-------------|-------|
| Authoritativeness | 2 | commercial service, not HMRC authority |
| Explainability | 3 | usable for validation, less transparent than internal engine |
| Coverage | 4 | broad cross-border support |
| Implementation effort | 4 | quick to stand up |
| Accuracy ceiling | 3 | strong sanity check, not canonical |
| **Average Score** | **3.2** | |

**Recommendation:** **USE AS SECONDARY VALIDATION**

**Avalara**

- Enterprise compliance solution.
- $16.5K+/year, 2-4 week setup.
- Reference: https://avalara.com

| Criterion | Score (1-5) | Notes |
|-----------|-------------|-------|
| Authoritativeness | 3 | enterprise-grade, still third-party |
| Explainability | 4 | stronger audit/compliance posture |
| Coverage | 5 | broadest enterprise feature set |
| Implementation effort | 2 | expensive and heavier onboarding |
| Accuracy ceiling | 4 | high, but overpowered for MVP |
| **Average Score** | **3.6** | |

**Recommendation:** **CONSIDER ONLY AT HIGHER SCALE**

Use third-party services ONLY as secondary validation signals, not as the primary engine. The internal HMRC-based engine must remain the canonical source.

#### AI-Based HS Classification (Future Enhancement)

The academic paper "ATLAS: Benchmarking and Adapting LLMs for Global Trade via Harmonized Tariff Code Classification" (2025) demonstrates fine-tuned LLMs achieving strong HS code classification. This approach could complement the HMRC FPO API as a future enhancement path for complex classification cases.

#### Classification Pipeline Preprocessing

Before any API call, apply preprocessing to improve classification quality:

1. **Description-quality gate.** Reject raw strings that are bundles ("centrifuges, sample preparation devices, and associated consumables"), service descriptions, or generic nouns ("goods", "samples", "parts", "equipment" without qualification). FPO's own documentation warns against generic words.
2. **Bundle splitter.** Decompose multi-item descriptions into separate product candidates before classification. "Centrifuges, sample preparation devices, and associated consumables" must become three separate classification requests. If decomposition fails, mark the basket as mixed/manual and stop automatic net-margin qualification.
3. **Attribute extraction.** Before lookup, extract: head noun, material, principal function, sector, sterility/medical status, power/size, and pack form. These attributes improve both FPO query quality and downstream 10-digit resolution.
4. **Multi-query FPO.** For each product candidate, run multiple normalized description variants: head noun only, noun+material, noun+function, noun+end-use. This reduces dependence on a single description phrasing.

#### 10-Digit Resolution and Measure Scanning

FPO returns only 6-digit HS or 8-digit CN candidates. The UK tariff's legal duty treatment lives at the **10-digit commodity code** layer and can require:

- **Additional 4-character codes** for trade remedies, preferences, quotas, or excise
- **Measure conditions** that change duty or documentary requirements based on price, weight, volume, or certificates
- **Specific measurement units** (e.g., duty per kg, per litre, per unit) that make weight/volume a duty input, not just a freight input
- **Geographic applicability** — different rates for different origin countries

The pipeline must resolve from HS6/CN8 candidates to a declarable 10-digit UK commodity code, then scan import measures for:

- Standard duty rates and any compound/specific duty components
- Trade remedy duties (anti-dumping, countervailing, safeguard) — additional duties that can dwarf base tariff rates, not cancelled by duty suspensions or ATQs
- Preferential tariff eligibility (requires rules-of-origin analysis and proof, not just supplier location)
- Quota/order numbers and remaining quota availability
- Import controls, prohibitions, restrictions, and required certificates/documents
- Additional codes required for declaration
- Measure condition thresholds (price, weight, volume triggers)

**Escalation logic:** Escalate automatically when:
- Top HS6 candidates do not converge (code dispersion)
- Candidates converge at HS6 but diverge materially at 10-digit measure level (measure dispersion)
- Additional codes or measure conditions are present that the engine cannot resolve
- Trade remedy duties are detected
- A positive margin coexists with an unsatisfied control measure (operational blocker)

#### Customs Recommendation

The landed-cost engine classification pipeline should:

1. Apply **description-quality gate and bundle splitter** to decompose and clean product descriptions before API calls.
2. Extract **structured attributes** (head noun, material, function, sector) from cleaned descriptions.
3. Use **HMRC FPO API** as the first-pass **candidate generator** — run multi-query variants and collect HS6/CN8 candidate codes with retrieval scores.
4. **Cluster and rank candidates** at HS6/CN8 level. If top candidates do not converge, flag classification instability.
5. **Resolve to 10-digit declarable commodity codes** via **HMRC Trade Tariff API** commodity hierarchy navigation.
6. Validate and cross-reference via **Taric Support API** for code completeness and hierarchy checking.
7. Cross-check with **HS Ping** for additional confidence signal.
8. **Scan import measures** via HMRC Trade Tariff API for: duty rates (including compound/specific), trade remedies, preferences, quotas, controls, additional codes, measurement units, and condition thresholds.
9. Apply customs valuation rules. **Method 1 (transaction value) is not universal** — no-sale scenarios, free samples, consignment stock, branch-office transfers, hire/lease, and condition-dependent or provisional pricing can require fallback valuation methods 2–6.
10. Note that **invoice price is not customs value** — selling commissions, assists, certain royalties/licence fees, seller's share of resale proceeds, and separately shown post-import deductions can all change duty liability.
11. Compute import VAT using the broadened VAT base: customs value + incidental expenses to first UK destination + duty + other import charges except VAT itself.
12. Determine **import VAT treatment** — postponed VAT accounting (PVA) is available only where the importer is UK VAT-registered, imports for business use, has the right to dispose of the goods, and includes its VAT number on the declaration. Non-VAT-registered importers cannot reclaim. Partially exempt or mixed business/non-business use can reduce recovery rates.
13. Produce per product candidate:
    - declarable 10-digit commodity code (or candidate set if unresolved),
    - additional codes required,
    - classification stability score,
    - duty estimate (base tariff + any trade remedy duties),
    - import VAT estimate with treatment classification,
    - VAT recovery rate estimate,
    - measure warnings (trade remedies, quotas, controls, operational blockers),
    - decision grade: `screening` | `review` | `rfq_required` | `broker_required`.

For repeated high-value product families, formalise the fallback:
- Request **Advance Tariff Rulings (ATRs)** — HMRC replies in roughly 30–120 days; legally binding for three years.
- Request **Advance Valuation Rulings (AVRs)** where the valuation method is complex — HMRC issues within 90 days; binding for three years.
- Cache approved product-family codebooks (HS10 + measures + ruling references) and reuse for matching products.
- Track classification accuracy: chapter accuracy, HS6 accuracy, 10-digit accuracy, duty-outcome accuracy, and review-trigger recall.

**References:**
- Advance Tariff Ruling: https://www.gov.uk/guidance/apply-for-an-advance-tariff-ruling
- Customs valuation Method 1: https://www.gov.uk/guidance/valuing-imported-goods-using-method-1-transaction-value
- PVA eligibility: https://www.gov.uk/guidance/check-when-you-can-account-for-import-vat-on-your-vat-return
- Trade remedies: https://www.gov.uk/government/publications/the-uk-trade-remedies-investigations-process/introduction-to-trade-remedies

#### Special Procedures and Alternative Duty Scenarios

The engine should expose special procedures as explicit **scenario switches**:

- **Inward Processing:** Can suspend customs duty and import VAT on goods imported for processing/repair and re-export.
- **Temporary Admission:** Can remove duty for temporary imports that are re-exported. Partial relief still leaves import VAT payable.
- **Duty suspensions and ATQs:** Can reduce or eliminate base duty for specific goods. ATQs are quantity-limited and do not remove VAT or trade remedy duties.

These can flip the economics of a deal. The engine should flag when a special procedure might be available and what the margin impact would be.

**References:**
- Inward Processing: https://www.gov.uk/guidance/special-procedure-inward-processing/rules-and-processes
- Origin proof: https://www.gov.uk/guidance/get-proof-of-origin-for-your-goods

**Note:** This is the ONE area where APIs are explicitly preferred over agent scraping. Cost-estimation data sources (customs, freight, FX) benefit from API reliability and structured responses. The skill-based agent scraping strategy applies to procurement and supplier data, not to cost-estimation APIs.

This internal-engine choice is formalized in **ADR-006**.

### 4.6 FX Conversion

#### Need

Everything should be evaluated in GBP and re-evaluated as FX changes.

#### Option A: Xe Currency Data API

**Official evidence:**

- Xe’s Currency Data API supports rates against a chosen base currency.
- Xe documentation discusses request-based packaging and update frequency.
- Xe states the API supports large fiat currency coverage and historical/point-in-time use cases.

**References:**

- https://www.xe.com/en-us/xecurrencydata/
- https://help.xe.com/hc/en-gb/articles/4411624652177-What-is-the-Currency-Data-API
- https://help.xe.com/hc/en-gb/articles/4411633342225-How-does-the-Currency-Data-API-work
- https://help.xe.com/hc/en-gb/articles/4414092026769-Currency-Data-API-packages-pricing-and-payment

| Criterion | Score (1-5) | Notes |
|-----------|-------------|-------|
| Requirement fit | 5 | explicit user preference |
| Historical support | 4 | point-in-time support implied by product docs |
| Commercial reliability | 4 | established provider |
| Implementation risk | 3 | pricing/request limits apply |
| **Average Score** | **4.0** | |

**Recommendation:** **SELECTED FOR COMMERCIAL FX**

Store FX snapshots in the database and re-run margin calculations when rate drift crosses a threshold. Use Xe for commercial margin estimation and real-time FX needs.

#### Option B: HMRC Exchange Rate Service

**Official evidence:**

- Official HMRC monthly exchange rates used for customs valuation calculations.
- Now integrated directly into the Online Trade Tariff service.
- Published penultimate Thursday of every month, applies to following calendar month.
- Available in CSV and XML formats.
- HMRC Developer Hub also provides XML API.

**References:**

- https://trade-tariff.service.gov.uk/exchange_rates/monthly
- https://developer.service.hmrc.gov.uk/api-documentation/docs/api/xml/Exchange%20rates%20from%20HMRC

| Criterion | Score (1-5) | Notes |
|-----------|-------------|-------|
| Authoritativeness | 5 | legally authoritative for UK customs duty computation |
| Historical support | 4 | monthly archives available |
| Commercial reliability | 5 | official government service |
| Implementation risk | 5 | CSV/XML, free, straightforward |
| **Average Score** | **4.8** | |

**Recommendation:** **MANDATORY FOR CUSTOMS VALUATION**

This is the legally authoritative rate for UK customs duty computation. The customs engine MUST use HMRC exchange rates for duty calculation. Use Xe for commercial margin estimation separately. The system should clearly distinguish between:
- **HMRC rates** → customs duty calculation (legally required)
- **Xe rates** → commercial margin estimation (real-time market rates)

#### Option C: CurrencyBeacon

**Official evidence:**

- 200+ currencies, 6000+ cryptocurrencies.
- Real-time and historical data.
- 99.9% uptime guarantee, sub-100ms response times.
- Institutional-grade data from ECB, Bank of Canada, etc.

**References:**

- https://currencybeacon.com

| Criterion | Score (1-5) | Notes |
|-----------|-------------|-------|
| Requirement fit | 4 | broad coverage, good performance |
| Historical support | 4 | historical data available |
| Commercial reliability | 4 | institutional-grade sources |
| Implementation risk | 4 | REST API, competitive pricing |
| **Average Score** | **4.0** | |

**Recommendation:** **BACKUP/COMPARISON**

Useful as a backup FX source or for cross-checking Xe rates.

#### Option D: OANDA Historical Currency Converter

**Official evidence:**

- 31 years of history, 200+ currencies, 38,000 FX pairs.
- Strong for auditing and historical analysis.

**References:**

- https://fxds-hcc.oanda.com

| Criterion | Score (1-5) | Notes |
|-----------|-------------|-------|
| Requirement fit | 3 | historical focus |
| Historical support | 5 | 31 years of data |
| Commercial reliability | 4 | established provider |
| Implementation risk | 3 | commercial access required |
| **Average Score** | **3.8** | |

**Recommendation:** **SUPPORTING SOURCE**

Useful for historical margin recalculation and audit trails.

#### FX Recommendation

Use a **tiered FX strategy**:

- **Customs valuation:** HMRC Exchange Rate Service (MANDATORY — legally authoritative)
- **Commercial margin estimation:** Xe Currency Data API (SELECTED — real-time market rates)
- **Backup/validation:** CurrencyBeacon (secondary signal)
- **Historical audit:** OANDA (for historical recalculation and audit trails)

Store `customsFxRate` and `commercialFxRate` **separately** in the database with timestamps, source identifiers, and HMRC publication periods. Do not collapse them into a single `fxRate` field — the customs rate is monthly (published penultimate Thursday, applies to the following calendar month) while the commercial rate is near-real-time.

**Basis gap tracking:** Compute and store `fxBasisDeltaPct` — the percentage gap between the HMRC customs rate and the Xe commercial rate at the time of estimation. For example, if HMRC CNY rate is 9.3778/£1 and Xe mid-market is 9.2029/£1, the basis gap is ~1.9%. This gap is a real cost/risk that affects margin accuracy.

**Recalculation trigger:** Trigger margin recalculation when the projected margin **moves materially or crosses a gate threshold**, not when raw FX drift alone exceeds a fixed percentage. A 2% FX move that doesn't change the margin decision is noise; a 0.5% move that flips a marginal opportunity across the 20% net gate is material.

**Operator display:** Present operators with both HMRC customs FX and market FX side by side, with the basis gap clearly shown. Do not hide the dual-rate complexity behind a single number.

### 4.7 Database Architecture — Local SQLite + Supabase/Postgres Sync

#### Need

A dual-database architecture that supports:

- **local-first operation** with the Rust daemon owning its own data store,
- **multi-user visibility** through a shared sync target,
- **corruption resilience** — a local non-corrupted copy protects against shared DB corruption,
- relational querying and auditability,
- storage of structured and unstructured artifacts,
- push-pull synchronization between local and remote.

#### Architecture: Authority Matrix — Local SQLite + Supabase/Postgres

**Design rationale:**

The owner requires multiple users to access the system's data. However, relying solely on a shared remote Postgres creates a single point of corruption risk. The architecture uses a **domain-owned authority model** rather than a singular "source of truth":

- **Local operational authority** (SQLite, bundled with the Rust daemon): Pipeline execution state, job leasing, source cursors, full raw session-output artifacts, full error context, raw documents. The daemon that ran the job is authoritative for its outputs.
- **Remote shared authority** (Supabase/Postgres): Shared configuration (company profile, source configs) and shared human decisions (operator dispositions). These originate remotely and are pulled to local.
- **Remote projections** (Supabase/Postgres): Viewer-facing copies of pipeline facts and status. These are push-only from local to remote and are never authoritative — the local copy can always republish.

**Terminology note:** In this document, prefer **local operational authority**, **remote shared authority**, and **shared sync target**. Historical mentions of "source of truth" appear only when contrasting this model with the superseded singular-authority design.

**Mixed-write tables are prohibited.** Every table has exactly one authority domain. If a concept spans domains (e.g., a pipeline-computed margin AND an operator disposition on the same process), it must be split into separate tables with separate authority.

**MVP constraint:** Exactly one active pipeline daemon per organization. Multi-executor coordination (remote source leases, primary election) is a future extension, not an MVP requirement. The viewing portal is read-only; shared human decisions come only from other desktop apps if they exist.

**Corruption protection scope:** The dual-database design protects against remote-only failures (bad Supabase migration, accidental admin update, stale restore, RLS policy mistakes). It does NOT protect against application-level logical corruption in the sync/replication layer — a bug in normalization, merge heuristics, or sync mapping will write bad data to both sides. The sync layer is therefore the highest-risk component for data integrity and must be treated accordingly.

#### Authority Split — Per-Table Mapping

| Authority | Tables | Sync Direction |
|-----------|--------|----------------|
| Local-only (no sync) | `raw_documents`, `sources` (local runtime copy), `source_runtime_state`, `source_cursors`, `listing_evidence`, `pending_operator_intents`, `workflow_jobs` (full rows), `agent_session_logs` (full transcript/trace), `error_manifests` (full context), `validation_verdicts` (full validator reasoning), local job leases, local runtime config/secrets | None |
| Push-only (local → remote projections) | `executors` (heartbeat summary), `procurement_processes`, `contract_notices`, `notice_lots`, `qualification_runs`, `product_candidates`, `search_profiles`, `search_attempts`, `supplier_quotes`, `landed_cost_estimates`, `manifests`, `process_relations`, `process_aliases`, `pipeline_assessments`, `bid_workflow_states`, `process_stage_status`, `process_stage_heads`, `workflow_job_summaries`, `error_manifest_summaries`, `validation_verdict_summaries`, `viewer_sync_status` | Push only |
| Pull-only (remote → local) | `shared_company_profile`, `shared_source_configs`, `operator_decision_heads` | Pull only |
| Mixed-write | **None — prohibited by design** | N/A |

#### Push Protocol (Local → Remote)

Every local mutation to a push-owned entity writes to both business tables and a **local outbox** in the same SQLite transaction. Crashes cannot create ghost writes or missing replications.

- **Push unit:** Full-row projections (not field patches), batched by `local_seq` range.
- **Ordering:** Monotonic `local_seq` per executor. Remote rejects gaps in `local_seq` — if remote has seq 4810 and receives 4812 first, it fails closed until 4811 arrives.
- **Idempotency:** Remote deduplicates by `(executor_id, batch_id)` and `(executor_id, local_seq)`. Replays are harmless.
- **Row versioning:** For mutable push-owned rows, apply only if incoming `row_version > stored row_version` for the same owner. Equal version with matching `content_hash` is a replay (ignore). Equal version with differing hash is quarantined as a protocol bug.
- **Atomicity:** Remote applies each push batch through a single transactional RPC / stored procedure, not a spray of individual upserts.
- **Stage coherence:** All outputs from a pipeline stage share a `stage_run_id`. The stage's `process_stage_status` is updated and `process_stage_heads` is advanced in the same transaction. Failed stages publish failure status + redacted error summary but do NOT advance the head.
- **Schema compatibility:** Every push batch carries a `schema_epoch`. If epochs are incompatible, sync pauses but local operation continues.
- **Delivery timeout:** A push batch must either ack or fail within 60 seconds. After 3 consecutive failed attempts or 15 minutes in `pending`, the daemon marks sync `degraded`, surfaces an operator alert, and keeps local execution running.

#### Pull Protocol (Remote → Local)

- Pull applies only to remote-owned domains (shared config, operator decisions).
- Remote-owned tables carry a monotonic `remote_change_seq` and `remote_revision`, bumped on every insert/update/delete.
- Local stores a pull cursor per domain (`last_pulled_config_seq`, `last_pulled_decision_seq`).
- Pull query fetches changes where `remote_change_seq > cursor`, ordered by seq, in bounded batches.
- Apply locally in order, transactionally, advance cursor only after successful commit.
- **Operator decisions** (when multiple desktops exist): Written with compare-and-swap — the update includes `base_remote_revision` and succeeds only if current revision matches. On mismatch, return conflict for manual reconciliation. Last-write-wins is NOT used for human commercial decisions.

**Convergence guarantees:** Ordering comes from `local_seq` per executor (push), `remote_change_seq` (pull), and `row_version` / `remote_revision` (row freshness). `updated_at` is for human audit only, never for conflict resolution — clock-based conflict rules create false simplicity.

#### Recovery Model

**Remote corrupted or lost → republish from local:**

The daemon can republish all push-owned projections and stage heads from local state. Two distinct recovery modes:

- **Incremental sync:** Normal outbox replay for recent changes.
- **Full remote rebuild:** Republish current state of all local-owned tables + stage heads. Not just recent outbox entries.

Shared config that originated in Supabase and was only cached locally may be lost if Supabase is the only copy.

**Local SQLite lost → partial rebuild from Supabase:**

From Supabase, the daemon can recover: shared config, shared operator decisions, remote projections of pipeline facts, redacted job/error summaries. The following are **permanently lost**: full raw session-output artifacts (PTY terminal bytes or contract-mode stdout/stderr), full error context, source cursors (daemon will re-ingest from scratch — expensive but recoverable), local job leases, raw documents if stored only on the same disk.

Two recovery tiers:

- **Warm rebuild:** Local DB file damaged but disk/artifact store still available. Run `PRAGMA quick_check` to assess damage, restore from hot backup (Online Backup API or `VACUUM INTO`) if available, then repull remote-owned state and republish local-owned projections.
- **Cold rebuild:** Full machine loss. Requires off-box retention of critical raw procurement artifacts and optionally full transcripts if operationally critical.

**Local corruption detection:** Run `PRAGMA quick_check` on startup; run `PRAGMA integrity_check` on maintenance windows. Use `.recover` only as a salvage path, not the primary recovery plan. Never take backups by copying the main DB file while the process is live — use the Online Backup API or `VACUUM INTO`.

#### Tombstone Policy

Tombstones (`deleted_at`) are used **only** for entities with real shared delete semantics:

- Shared configuration (remote-owned)
- Shared operator decisions (remote-owned)
- Viewer-facing projections where delete must propagate

High-volume pipeline history (quotes, estimates, qualification runs, manifests) is **append-only** and **superseded by stage head pointers**, not tombstoned.

Tombstone garbage collection is safe only when:

- Every active replica has pulled past the delete's `remote_change_seq`, AND
- The tombstone is older than `max_offline_window + safety_margin` (e.g., 30 days + 7 days = 37 days)

Replicas offline past the window become **stale** and must full-bootstrap instead of incremental pull.

#### Local SQLite

| Criterion | Score (1-5) | Notes |
|-----------|-------------|-------|
| Local-first fit | 5 | bundled with Rust daemon, no network dependency |
| Rust compatibility | 5 | `rusqlite` and `sqlx` with SQLite feature are excellent |
| Corruption resilience | 5 | local copy is independent of shared DB state |
| Multi-user access | 1 | single-user by design |
| Relational power | 4 | full SQL, JSON via json1 extension, but no native arrays |
| **Average Score** | **4.0** | |

**SQLite compatibility notes:**

- No native array types — use TEXT with JSON arrays or junction tables.
- JSON support via json1 extension (built into most SQLite distributions).
- No `gen_random_uuid()` — use application-generated UUIDs.
- No `timestamptz` — store timestamps as TEXT (ISO 8601) or INTEGER (Unix epoch).
- No native `boolean` type — use `INTEGER` (0/1). SQLite accepts `boolean` in DDL but stores as integer.
- No `bigint` distinction — use `INTEGER` (SQLite integers are 64-bit when needed).
- No `numeric` with precision — `REAL` or `INTEGER` (prefer integer pence for monetary values per §5.4).
- No `jsonb` — use `TEXT` with json1 functions. `json_valid()` for check constraints.
- No concurrent write access — single-writer model suits the daemon architecture.
- Use `STRICT` tables where json1 extension is not needed (enforces type affinity).

#### Supabase/Postgres (Shared Sync Target)

**Official evidence:**

- Supabase uses Postgres as its core database.
- RLS is a first-class security model.
- Realtime can stream Postgres changes and supports authorization.
- Self-hosting exists if future control is needed.

**References:**

- https://supabase.com/docs/guides/getting-started/architecture
- https://supabase.com/docs/guides/database/postgres/row-level-security
- https://supabase.com/features/realtime-postgres-changes
- https://supabase.com/docs/guides/realtime/authorization

| Criterion | Score (1-5) | Notes |
|-----------|-------------|-------|
| Multi-user shared access | 5 | ideal for shared visibility |
| Audit/query power | 5 | excellent for joins, analytics, history |
| Realtime | 4 | strong enough for viewing-portal updates |
| Auth/RLS | 5 | mature and explicit |
| Rust compatibility | 4 | Postgres/sqlx is straightforward |
| **Average Score** | **4.6** | |

**Role in authority matrix:** Supabase/Postgres serves two distinct roles:

- **Remote shared authority:** Shared configuration (company profile, source configs) and shared human decisions (operator dispositions) originate here and are pulled to local.
- **Remote projections:** Viewer-facing copies of pipeline facts, stage status, and summaries are pushed here from local. The viewing portal reads these projections. Multiple operators can see data from each other's local pipelines.

#### Sync Protocol (Supabase-Specific)

The sync engine implements authority-based replication as described in section 4.7 above. Supabase-specific considerations:

- Push batches are applied through a single transactional RPC / stored procedure (not a spray of individual upserts from the daemon client).
- For viewing-portal live updates, prefer small projection-change or status broadcast events over raw Postgres Changes subscriptions on every hot table. Supabase Broadcast is the recommended path for scalability; Postgres Changes is simpler but less scalable.
- Do not have the viewing portal query all raw sync tables directly — provide dedicated projection tables keyed to portal use-cases.

#### Database Recommendation

Use **local SQLite** with authority-matrix-based replication to **Supabase/Postgres** for multi-user visibility. Convex and SpacetimeDB are not selected (see original evaluation — good products, wrong fit for this workload).

This authority split is formalized in **ADR-002**.

### 4.8 Desktop Runtime and Frontend Shell

#### Need

A desktop client with:

- ergonomic UI,
- secure local agent execution,
- background daemon/sidecar support,
- optional shared UI with the viewing portal.

#### Option A: Tauri v2 + Next.js static export

**Official evidence:**

- Tauri’s Next.js guide requires static exports.
- Tauri does not natively support server-based solutions inside the desktop shell.

**References:**

- https://v2.tauri.app/start/frontend/nextjs/
- https://v2.tauri.app/start/frontend

| Criterion | Score (1-5) | Notes |
|-----------|-------------|-------|
| User preference alignment | 5 | user explicitly likes this path |
| Desktop fit | 3 | works, but only in static/client-heavy mode |
| Shared code potential | 4 | good with web app |
| Complexity | 3 | extra friction if trying to reuse SSR assumptions |
| **Average Score** | **3.8** | |

#### Option B: Tauri v2 + React/Vite desktop + separate Next.js web app

**Official evidence:**

- Tauri docs explicitly recommend Vite for SPA frameworks.
- Tauri supports frontend-agnostic static bundles.

**References:**

- https://v2.tauri.app/start/frontend

| Criterion | Score (1-5) | Notes |
|-----------|-------------|-------|
| Desktop fit | 5 | simplest and cleanest |
| Operational clarity | 5 | avoids SSR mismatch |
| Shared code potential | 4 | shared UI package still easy |
| User preference alignment | 3 | deviates from stated Next preference |
| **Average Score** | **4.4** | |

#### Option C: Electron + Next.js

| Criterion | Score (1-5) | Notes |
|-----------|-------------|-------|
| Desktop maturity | 5 | mature |
| Security footprint | 2 | heavier |
| Resource profile | 2 | larger |
| Rust sidecar synergy | 3 | okay, but less attractive |
| **Average Score** | **3.0** | |

#### Desktop Recommendation

**Selected:** Tauri v2.

**Strong recommendation:** Use **shared React UI packages** and avoid binding the whole product architecture to “one Next app inside Tauri”. If you insist on Next for desktop, keep it strictly client-side/static and do not design around SSR/server actions inside the Tauri shell.

This shell/runtime choice is formalized in **ADR-003**.

### 4.9 Local Agent Orchestration Runtime

#### Need

Run Codex/Claude agents locally on user machines with:

- reusable skills,
- dynamic kickoff prompts,
- adapter-mediated agent execution with PTY and contract modes from day 1,
- full raw session-output capture as forensic infrastructure,
- validated result contracts as the authoritative workflow-state record,
- cron-triggered and manual-triggered sessions,
- robust daily operation without interruption,
- process-tree supervision and reliable cleanup.

Agent orchestration is the most critical infrastructure piece. The entire data ingestion, intelligence, and bidding pipeline depends on it working every time. PTY execution is a first-class runtime capability from day 1, but not the mandatory transport for every job — the daemon selects the appropriate execution mode per job via an adapter boundary.

#### Selected Approach: Adapter-Mediated Hybrid Execution

**Design decision:**

The Rust daemon triggers Codex/Claude sessions through an **Agent Adapter** boundary with two execution modes:

1. **PTY mode:** Required for browser automation, interactive auth flows, or any CLI behavior that depends on a controlling terminal. The daemon creates PTY pairs (master/slave), captures full terminal output as an immutable forensic artifact, and monitors session health.
2. **Contract mode:** Preferred for deterministic tasks where the vendor supports structured non-interactive execution (Codex `exec` with `--json` JSONL output; Claude headless with `json`/`stream-json` output and optional `--json-schema` validation). Lower overhead, simpler cleanup, stronger result contracts.

The daemon selects mode per job based on the skill's declared `interaction_profile`. PTY remains a first-class runtime from day 1 — it is not deferred or simplified away.

| Criterion | Score (1-5) | Notes |
|-----------|-------------|-------|
| Flexibility | 5 | PTY handles all CLI modes; contract mode handles deterministic flows with less overhead |
| Robustness | 5 | adapter boundary isolates vendor changes; mode selection matches job requirements |
| Log capture | 5 | full raw session output captured in both modes, with structured events captured in contract mode |
| Automation fit | 5 | cron + manual triggers, headless operation in both modes |
| Complexity | 3 | adapter + dual-mode is more complex than single-mode, but reduces blast radius |
| **Average Score** | **4.6** | |

**Agent Adapter boundary:**

Each vendor (Claude, Codex) has a dedicated adapter (`agents/codex.rs`, `agents/claude.rs`) that produces a **canonical daemon event stream**. The adapter owns:

- Launch command construction (flags, env vars, working directory).
- Capability probing at startup (CLI version, auth health, structured output viability, PTY viability, browser/tool viability).
- Mode selection (`pty` or `contract`) based on skill `interaction_profile` and probe results.
- Parsing vendor-native JSON/event streams into daemon events.
- Health classification from vendor-specific signals.
- Final result contract validation.

Everything above the adapter speaks only in daemon-native events: `SessionStarted`, `CheckpointWritten`, `AuthChallenge`, `SessionStalled`, `CleanupIncomplete`, `FinalResultValidated`. Unknown CLI versions enter **canary mode** — the adapter runs a smoke probe before accepting production traffic.

**Session isolation:**

Concurrent sessions on the same workstation can contaminate each other through shared CLI config and state. The daemon must:

- Pin `CODEX_HOME` / `.codex` config paths per session or verify isolation.
- Control working directory per run.
- Record CLI version, model, config hash, skill version, and adapter version on every session.
- Use per-session browser profiles for browser-backed flows (prevent cookie/lock contention).

**Session lifecycle (expanded):**

Each agent session has a lifecycle with explicit phases:

```text
created → spawned → active → terminating → exited → reaped → archived
```

A session is only **complete** after all of:

1. Child exit observed.
2. PTY drained to EOF or drain timeout (PTY mode only).
3. Descendant process tree cleaned up (verified, not assumed).
4. Final result contract validated (if present).
5. Cleanup verdict recorded.

**Validator sessions:** Validator agent sessions follow the same lifecycle and adapter-mediated execution as all other sessions. They use `headless_contract` mode exclusively (they read run manifests, not browse websites). The daemon treats them identically for supervision, cleanup, and observability — they simply have a different skill type and are spawned as sub-jobs linked to a worker job via `parent_job_id`.

- **Cron-triggered execution:** The daemon's built-in cron scheduler creates sessions on schedule, selects the appropriate execution mode via the adapter, injects the kickoff prompt and skill reference, and monitors execution.
- **Manual triggers from desktop:** The Tauri desktop control panel sends trigger commands to the daemon, which creates a session (PTY or contract mode) and streams progress back to the desktop control panel.
- **Full output capture (PTY mode):** Every byte of terminal output is captured and stored as an immutable forensic artifact. This includes agent thinking, tool calls, errors, and results. Terminal output is a forensic channel for debugging — it does not drive workflow state.
- **Full output capture (contract mode):** The daemon captures the raw stdout/stderr stream as an immutable forensic artifact while the adapter parses vendor JSONL/JSON events into daemon-native events. Parsed events drive workflow state; raw bytes remain for debugging and replay.
- **Session capture invariant:** Every agent session must leave behind a complete forensic record appropriate to its mode: raw PTY bytes for PTY mode, or raw stdout/stderr bytes plus structured protocol events and result artifacts for contract mode.
- **Auth challenge detection:** The daemon detects when an agent session fails due to expired credentials, MFA challenges, browser SSO failures, or hidden permission prompts. These are classified as `AGENT_AUTH_CHALLENGE` (distinct from `AGENT_STALLED`).

**Why PTY remains a first-class runtime (not the only runtime):**

PTY mode is required when:

1. The skill uses browser automation (agent needs real terminal semantics for tool interactions).
2. The agent CLI session involves interactive auth flows or re-authentication.
3. The vendor CLI does not support structured non-interactive output for the required task.
4. The skill explicitly declares `interaction_profile: pty_interactive`.

Contract mode is preferred when:

1. The task is deterministic and non-interactive (e.g., API-driven qualification).
2. The vendor supports structured JSON/JSONL output (Codex `exec --json`, Claude headless `json`/`stream-json`).
3. The skill declares `interaction_profile: headless_contract`.
4. Simpler cleanup and stronger result contracts outweigh terminal fidelity.

The adapter chooses mode per job. If a contract-mode session encounters an unexpected interactive prompt or auth challenge, the adapter can escalate the failure rather than silently stalling.

**Process-tree supervision:**

The daemon supervises the **entire descendant tree**, not just the initial CLI process. Agent CLIs may spawn sub-processes (browser instances, tool servers, language runtimes).

The cleanup sequence:

1. Spawn session in its own process group (POSIX) or job object (Windows).
2. Record pid, pgid, PTY id, adapter id at spawn.
3. On soft stop: send in-band interrupt if the adapter supports it.
4. On timeout: SIGTERM the **process group**, not just the main pid.
5. After grace period: SIGKILL the group.
6. Keep draining the PTY master until EOF or drain timeout (PTY mode).
7. Wait/reap explicitly — do not rely on dropping Tokio child handles.
8. Run orphan sweep and record cleanup verdict (`clean`, `forced`, `leaked`).

On Linux, each session can optionally run in a transient `systemd-run --scope` unit for resource control, or the daemon can register as a child subreaper. On macOS, process group signaling via `killpg` is the primary mechanism.

**PTY hardening:**

- **File descriptor hygiene:** Aggressively close inherited fds before exec. macOS Big Sur/Cocoa and GNOME/Mutter can leak fds into child processes, causing intermittent failures.
- **EIO handling:** Treat `EIO` on the PTY master read (after slave closes) as a normal termination path, not an internal error.
- **Signal handling:** Use signals only as wakeups, then drain `waitpid`/wait loops until there is nothing left to reap. Signals may be coalesced by Tokio's Unix signal streams.
- **Terminal geometry:** Fix PTY size for headless runs (e.g., 120x40) so CLI formatting and browser/tool behavior are stable. Only resize when an operator attaches for live viewing.

**Host environment awareness:**

- **Sleep/wake:** If the host suspends during an active session, assume browser-driven PTY sessions are tainted. Use monotonic time for stall detection and emit `HOST_SLEEP_INTERRUPTED` if a suspend gap crosses a configurable threshold.
- **Missed runs:** When the daemon starts after extended downtime, it checks for overdue scheduled jobs and runs a catchup sweep. Jobs are executed in priority order, not all at once.

This runtime model is formalized in **ADR-004**.

### 4.10 Skills and Browser Automation — Core Data Retrieval Architecture

Skills are the **primary data retrieval mechanism** for the platform. All procurement data and supplier pricing is collected via skill-based agent scraping with browser automation. This is the core architectural decision that simplifies the entire system: instead of maintaining API client packages for each data source, the platform maintains skills (way easier to maintain, upgrade, and change).

#### Skills

**Official evidence:**

- OpenAI Codex skills and Claude skills both use directory-based `SKILL.md` packaging and progressive disclosure.

**Terminology note:** In this document, **skill** means the versioned directory asset bound to a job, while **skill playbook** means the human-readable instructions inside that skill's `SKILL.md`.

**References:**

- https://developers.openai.com/codex/skills/
- https://platform.claude.com/docs/en/agents-and-tools/agent-skills/overview

#### Browser automation reference

**Reference:**

- https://github.com/vercel-labs/agent-browser/blob/main/skills/agent-browser/SKILL.md

#### Skill Types

The platform requires skills across three categories:

**1. Procurement Ingestion Skills** (agent scrapes procurement sources via browser):

| Skill | Target Source | Approach |
|-------|--------------|----------|
| `procurement-ingestion/fts` | Find a Tender | Navigate to FTS API/portal, extract OCDS notice data |
| `procurement-ingestion/contracts-finder` | Contracts Finder | Navigate to CF API/portal, extract notice data |
| `procurement-ingestion/pcs` | Public Contracts Scotland | Navigate to PCS API/portal, extract notice data |

**2. Supplier Price Enrichment Skills** (agent scrapes marketplace websites via browser):

| Skill | Target Source | Approach |
|-------|--------------|----------|
| `supplier-price-enrichment/ebay` | eBay UK | Search products, extract pricing, seller info, shipping |
| `supplier-price-enrichment/amazon` | Amazon UK | Search products, extract pricing, availability |
| `supplier-price-enrichment/alibaba` | Alibaba.com | Search products, extract pricing, MOQ, shipping |
| `supplier-price-enrichment/1688` | 1688.com | Search products, extract wholesale pricing |

**3. Intelligence Pipeline Skills** (agent performs extraction and classification):

| Skill | Purpose | Approach |
|-------|---------|----------|
| `document-acquisition` | Acquire public tender documents | Navigate to notice source portal, identify and download spec PDFs, pricing schedules, BoQ spreadsheets, lot documents |
| `lot-extraction` | Extract lot structure from documents | Parse unstructured lot descriptions when OCDS lot data is absent |
| `product-extraction` | Extract product candidates from lot text/docs | Identify items, classify candidateType, extract structured slots, decompose bundles |
| `preliminary-qualification` | Classify goods/services for ambiguous cases | Analyze notice text only when deterministic expiry/category/CPV rules leave ambiguity |

**4. Workflow Skills** (agent performs workflow actions via browser):

| Skill | Purpose | Approach |
|-------|---------|----------|
| `bid-portal-*` | Portal-specific bid preparation | Navigate to eSourcing portal, fill forms, capture evidence |
| `manifest-generation` | Generate opportunity manifests | Fill narrative sections for the canonical Markdown manifest produced by the manifest package |

**5. Operational Skills** (maintain source health and authentication):

| Skill | Target Source | Approach |
|-------|--------------|----------|
| `auth-refresh/alibaba` | Alibaba.com | Establish or refresh authenticated session state, publish auth-state handle for enrichment skills |
| `auth-refresh/1688` | 1688.com | Establish or refresh authenticated session state, publish auth-state handle for enrichment skills |
| `source-health/<source>` | Any configured source | Pre-flight canary: verify reachability, geo-correctness, auth state, page fingerprint, challenge rate |

Auth-refresh skills do one thing: establish or refresh authenticated state and publish a new auth-state handle. They are easier to stabilize than forcing every enrichment skill to own login logic. Source-health canary skills run before expensive scheduled jobs to detect whether a source is reachable, geo-correct, authenticated (if needed), and not actively challenging — if health is red, the daemon skips the big job and preserves the account/IP.

**6. Validator Skills** (agent reviews run manifests for semantic quality):

| Skill | Purpose | Approach |
|-------|---------|----------|
| `validation/base` | Generic run manifest quality checks | Review run manifest completeness, narrative consistency, cross-check manifest claims against result contract data, verify fingerprint/locale/currency assertions |
| `validation/procurement-ingestion` | Procurement-specific validation overlay | Verify extracted data matches expected source (OCID structure, date ranges, notice types), detect cross-source contamination |
| `validation/supplier-enrichment` | Supplier-specific validation overlay | Verify pricing plausibility, currency consistency, locale correctness, detect wrong-market data |
| `validation/document-acquisition` | Document acquisition validation overlay | Verify downloaded documents match expected types, content is non-empty, format is correct |
| `validation/product-extraction` | Product extraction validation overlay | Verify extracted candidates are plausible given lot descriptions, detect fabricated or implausible structured data |

Validator skills are the semantic validation layer. They read the worker agent's run manifest — a human-readable MD narrative of what the agent did — and render a verdict (`approved`, `rejected`, `flagged`). They use `headless_contract` mode (no browser needed) with `structured_extract_low_cost` model profile. Base validator checks apply to all runs; domain-specific overlays add source-specific criteria loaded from the worker skill's directory. Validator skills follow the same production directory standard and promotion flow as all other skills.

**Boundary note:** There is intentionally no `validation/landed-cost` overlay in the current file map because the landed-cost engine remains deterministic and API-based in this architecture. If a future landed-cost stage becomes skill-executed, it adopts the same validator-overlay pattern.

**Relationship between OQG and semantic validation:** The existing Output Quality Gate performs three validation layers: syntactic (schema), semantic (business invariants), and behavioral/drift (statistical envelope). These are code-based, deterministic checks. Validator skills perform a **fourth layer**: agent-based semantic validation — an agent reads the run manifest as a human would and judges whether the output makes sense contextually. OQG catches structural and statistical anomalies (wrong fields, null-rate spikes, duplicate rate changes). The semantic validator catches contextual errors invisible to code-based checks (navigated to wrong page but extracted valid-looking data, Scottish notices from an England-only source, prices from consumer listings when industrial was required). OQG signals feed INTO the validator as inputs, not compete with it.

#### Skill Lifecycle

1. **Creation:** Skills are authored as directories containing `SKILL.md` plus supporting artifacts (schemas, canary inputs, validators). Markdown-first authoring is preserved — the `SKILL.md` remains the primary human-readable playbook.
2. **Promotion:** New or updated skills follow a promotion flow: draft → candidate (CI lint + smoke canaries pass) → canary (small percentage or single source shard) → shadow (runs in parallel with stable, comparing quality metrics) → stable (scheduler default). Rollback rebinds the source to the prior stable version. Promotion from canary → shadow → stable can require validation pass rates (from the worker-validator system) meeting thresholds — validator approval is a promotion gate alongside quality metrics.
3. **Execution:** The daemon triggers an agent session (PTY or contract mode) via the adapter with the skill loaded. Scheduled jobs bind an explicit `stable_version` (plus optional `canary_version` and `shadow_version`). The agent follows the skill's playbook. After completing its work, the agent writes a **run manifest** — a structured Markdown narrative describing what it did, what it found, what decisions it made, and any warnings. The run manifest is part of the skill's output contract and is stored as an immutable artifact.
4. **Manual mode:** Draft skills can be explored without scheduler involvement. The Kickoff Prompt Renderer emits the exact kickoff prompt, including the standardized run manifest instruction block, for copy-paste into a fresh Codex/Claude session. Manual mode is for early exploration and does not commit pipeline state.
5. **Dry-run mode:** The daemon can execute the full worker + validator path against a scratch artifact workspace at `artifacts/scratch/<run_id>/`. Dry-run mode records run manifests and validation verdicts for review but does not advance authoritative pipeline rows, outbox entries, or stage heads. It is the default calibration path before validator enforcement is enabled.
6. **Validation:** After the result contract passes the Output Quality Gate, the daemon's Validation Policy Engine checks whether semantic validation is required. If so, a validator sub-job is spawned: a validator skill reads the run manifest, OQG signals, and result contract summary, then renders a verdict (`approved`, `rejected`, `flagged`). See §5.2 for the full validation pipeline.
7. **Maintenance:** When a source website changes layout/flow, only the skill needs updating — no API client code changes. Source-health canary skills detect drift before expensive extraction runs.
8. **Versioning:** Skills are versioned in the repo. The daemon records which skill version was used for each execution. Behavior version, result schema version, checkpoint schema version, validator version, and normalizer version are tracked independently — a playbook update does not force a schema version bump unless the contract shape changes.
9. **Upgrade:** Updating a skill is as simple as editing the `SKILL.md` and supporting files — no recompilation, no package upgrades. Schema migrations use dual-accept windows: the daemon accepts both old and new schema versions during transition, with shadow comparison to validate quality parity.

#### Skill Design Principles

- Skills define **what data to extract** and **how to navigate** the source.
- Skills include **validation rules** so the agent can verify it extracted correct data — at four levels: syntactic (schema match), semantic (business invariants), behavioral/drift (statistical envelope checks), and agent-based semantic validation (validator skill reviews the run manifest to catch contextual errors invisible to code-based checks). The first three are code-based (Output Quality Gate); the fourth is agent-based (worker-validator system, see §5.2).
- Skills include **error handling instructions** so the agent can report failures clearly, using the standard outcome classes and error taxonomy in §5.11.
- Skills are **source-specific** — one skill per source, not generic "scrape anything" skills.
- Skills output a validated result contract (JSON payload, artifact file, or NDJSON checkpoint stream) that the daemon validates against a declared schema — workflow state is never driven by transcript scraping.
- **Scheduled jobs bind explicit skill id and version.** Implicit skill discovery is disabled for production cron runs. Discovery may be used for ad hoc operator-triggered sessions.
- Every skill defines explicit **trigger boundaries**: when the skill should run, when it should not, what inputs it expects, what outputs it promises, and what is explicitly out of scope.
- Every browser skill defines **page fingerprints** for each important page state: required anchors, forbidden anchors, expected locale/currency/persona. Page fingerprint checks run before extraction to prevent "clean run, wrong page" failures.
- Navigation uses a **tiered locator strategy**: semantic locator → anchored relative locator → text/content fallback → CSS/XPath escape hatch. Every navigation step has a goal, proof, fallback, abort condition, and checkpoint rule.
- Every extracted business item carries **per-item evidence**: source URL, raw text fragments used, capture timestamp, page fingerprint ID, normalization version, evidence snippet or artifact pointer, and item digest.
- Checkpoints align to **idempotency units** (notice batch, candidate-source pair, document, lot), not browser state (DOM index, scroll position, tab number). Resume tokens are business-level identifiers.
- Skills declare explicit **cost budgets**: max pages, max scroll cycles, max detail page visits, max query variants, target confident hits, time budget, and degrade/continue thresholds.
- **If the same input should always produce the same intermediate check or normalization, that part belongs in deterministic helper code** (scripts/ directory), not in model prose.

#### Skill Metadata

Every skill declares metadata that the daemon uses for execution planning:

| Field | Purpose | Example |
|-------|---------|---------|
| `interaction_profile` | Execution mode selection | `pty_interactive` or `headless_contract` |
| `result_schema_version` | Result contract versioning | `1.0` |
| `auth_requirements` | What credentials the agent needs | `browser_session`, `api_key`, `none` |
| `browser_required` | Whether the skill needs browser automation | `true` / `false` |
| `max_concurrency` | Maximum concurrent sessions for this skill's source | `1` |
| `expected_duration` | Typical runtime for resource planning | `5m`, `30m` |
| `model_profile` | Model capability tier for daemon-side model selection | `browse_stable_low_cost`, `structured_extract_low_cost`, `ambiguous_match_high_reasoning`, `portal_form_high_reliability` |
| `egress_profile` | Expected network egress for geo-sensitive skills | `uk_residential`, `cn_exit`, `default` |
| `validation_policy` | When semantic validation runs for this skill | `always`, `sample`, `trigger`, `skip` |
| `validation_sample_rate` | Sample rate when `validation_policy` is `sample` | `0.1` (10%) |
| `validation_triggers` | Conditions that force validation regardless of policy | `fill_rate_drop > 0.2`, `first_run_after_version_change`, `suspect_output_in_last_5` |

Skills that declare `interaction_profile: headless_contract` must also declare a `result_schema_version` that the daemon uses for result contract validation.

**Validation policy** is declared per-skill in skill metadata, not in daemon config. This preserves plug-and-play composability: adding validation to a skill requires only setting the `validation_policy` field and optionally adding a `validator-overlay.md` file to the skill directory — no daemon code changes, no schema changes, no recompilation. Skills in draft/candidate/canary/shadow stages always validate regardless of declared policy.

Skills declare a `model_profile` instead of a vendor-specific model string. The daemon maps the profile to the current best-fit model. This allows model upgrades without touching skill files and enables cost-tier optimization (cheap models for classification/extraction, premium models for ambiguous adjudication).

#### Architecture Distinction

Skills are the primary data retrieval mechanism, but they are **still not the workflow engine**. The distinction:

- **Skills define:** How the agent behaves when scraping a source, what data to extract, how to handle errors, and what execution mode they require.
- **Daemon/database defines:** When to run which skill, what job state to track, retry logic, scheduling, sync, and which skill version to bind.
- **Orchestration stays in the daemon:** The daemon creates sessions (PTY or contract mode via the adapter), selects skills by explicit binding, monitors execution, validates result contracts, stores results in SQLite, and syncs to Supabase.
- **Adapter mediates:** The Agent Adapter layer handles vendor-specific launch, structured output parsing, health classification, and capability probing — insulating the daemon from CLI version changes.

These boundaries are formalized in **ADR-005**.
#### Production Skill Directory Standard

A production skill is not "just `SKILL.md`." The maintained unit is the **skill directory**: `SKILL.md` plus supporting artifacts that operationalize the skill as a contract. This is consistent with how both Codex and Claude define skills: filesystem directories with `SKILL.md` plus optional scripts and resources, loaded on demand from metadata.

**Required for production-bound skills:**

| File | Purpose |
|------|---------|
| `SKILL.md` | Human-readable playbook — the primary authoring artifact |
| `result.schema.json` | JSON Schema for the final result contract |
| `checkpoint.schema.json` | JSON Schema for checkpoint payloads |
| `canary.inputs.jsonl` | Small set of known inputs for smoke testing |
| `golden.assertions.yaml` | Invariant-based assertions for golden journey tests |

**Optional supporting files:**

| File | Purpose |
|------|---------|
| `scripts/` | Deterministic helper code for validation, normalization, field extraction, query fingerprinting |
| `references/` | Source notes, screenshots of expected page layouts, challenge fingerprints |
| `validator-overlay.md` | Domain-specific validation criteria for the semantic validator — what "correct" looks like for this skill's source and output. Loaded by the validator skill alongside the base validator. Not required if the base validator's generic checks are sufficient. |

**When markdown-only is enough:**

Instruction-only skills suffice for short, non-authenticated, non-resumable tasks where correctness is obvious from the final artifact (e.g., manifest narrative synthesis, simple qualification checks).

**When deterministic helpers are needed:**

The moment a skill requires any of the following, prose instructions alone are insufficient:

- Auth/session state reuse
- Resumability from checkpoints
- Deterministic field validation or normalization
- Stable query fingerprinting
- Challenge classification
- Field-level evidence extraction
- Canary assertions that must be bit-exact

The rule: **if the same input should always produce the same intermediate check or normalization, that part belongs in deterministic helper code, not in model prose.**

#### Skill Authoring Guide

Every production skill answers eight questions explicitly in its `SKILL.md`.

**1. Trigger Boundary**

Every `SKILL.md` includes:

- **Use when:** conditions under which this skill is appropriate
- **Do not use when:** explicit exclusions and non-goals
- **Inputs expected:** schema reference, required context, cursor/resume data
- **Outputs promised:** result contract shape, artifact types
- **Known non-goals:** what this skill explicitly does not attempt

Vague trigger boundaries are the single largest source of skill unreliability. Half of "skill unreliability" is bad trigger boundaries, not bad extraction logic.

**2. Page Fingerprint**

Before a skill clicks or extracts, it proves the page is the page it expects. For each important page state, the skill defines:

| Check | Purpose | Example |
|-------|---------|---------|
| Required anchors | Prove correct page loaded | Search box present, results container present, at least one result card visible |
| Forbidden anchors | Detect blocks or wrong page | Login form, CAPTCHA text, "verify you are human," "access denied" |
| Expected locale | Prevent locale drift | `en-GB` |
| Expected currency | Prevent currency contamination | `GBP` |
| Expected persona/auth state | Verify correct account | Logged in as expected persona, or anonymous where expected |
| Expected page kind | Distinguish result types | Search results vs category page vs single-product detail |

This stops "clean run, wrong page" failures — the most insidious skill failure mode because the output looks schema-valid.

**3. Navigation Strategy**

Navigation uses a **tiered locator strategy**, following Playwright's best-practice model: prefer user-facing locators and explicit contracts (`role`, `label`, `text`, `testid`) because they re-resolve at action time and come with auto-waiting/actionability checks; use raw CSS/XPath only as the last fallback.

The locator tier:

1. Semantic locator (role, label, accessible name)
2. Anchored relative locator (relative to a stable landmark)
3. Text/content fallback (visible text matching)
4. CSS/XPath escape hatch (fragile, last resort)

Every navigation step in `SKILL.md` specifies:

- **Goal:** what this step accomplishes
- **Proof:** how the skill verifies the step succeeded
- **Fallback:** what to try if the primary locator fails
- **Abort condition:** when to stop trying and classify the failure
- **Checkpoint rule:** whether completing this step is a valid checkpoint boundary

**4. Overlay and Interrupt Handling**

Unexpected overlays (cookie banners, login nags, survey popups, interstitials) are common enough to deserve first-class handling:

- **Predictable overlays** (cookie consent, login prompts on known pages): explicit step in the normal flow — handle as part of the expected page state, not as an exception.
- **Unpredictable overlays** (surveys, promotional interstitials, A/B test popups): handled via a known overlay handler map in the skill. Each handler specifies the detection pattern and dismissal action.
- Every overlay handler invocation increments a metric.
- Repeated handler invocations in one run (e.g., >3 overlay dismissals of the same type) raise a suspicion signal.

**5. Extraction Validation**

Each skill defines three validation layers:

**Syntactic validation:** Required fields present, types valid, URLs parse and belong to expected domains, schema matches `result.schema.json`.

**Semantic validation:** Deadline after publication date, currency consistent with expected locale, source IDs unique within extraction batch, price/unit normalization plausible, lot value totals plausible relative to notice total, quantity × unit price within reasonable range.

**Behavioral/drift validation:** Null-rate within historical envelope (sudden spike signals extraction breakage), duplicate rate within envelope (sudden spike signals pagination loop), result count not collapsing unexpectedly, page fingerprint hash not newly divergent, locator fallback depth not suddenly much higher than usual (signals DOM structure change).

These three code-based layers are the first line of defense against schema-valid garbage. A skill can produce valid JSON with completely wrong data and still look "successful" without behavioral validation. The daemon enforces this through an **Output Quality Gate** that runs after schema validation and before DB commit; outputs that fail semantic or drift checks are quarantined as `AGENT_SUSPECT_OUTPUT` and never promoted into pipeline state. For contextual errors that code-based checks cannot detect (e.g., navigated to the wrong page but extracted valid-looking data), the worker-validator system provides an additional agent-based semantic validation layer — see §5.2 Validation Executor.

**6. Per-Item Evidence**

Every extracted business item (notice, quote, document reference, product candidate) carries evidence alongside the normalized record:

| Field | Purpose |
|-------|---------|
| `sourceUrl` | Exact URL the data was extracted from |
| `rawTextFragments` | Key text snippets used for extraction (not full page HTML) |
| `captureTimestampUtc` | When the extraction occurred |
| `pageFingerprint` | Hash of the page fingerprint at extraction time |
| `normalizationVersion` | Version of the normalization logic applied |
| `evidenceSnippet` | Human-readable excerpt for debugging |
| `artifactPointer` | Reference to the forensic session artifact if deeper inspection needed |
| `itemDigest` | Content hash of the normalized record for deduplication and change detection |

**7. Checkpoint Unit**

Checkpoints align to the **idempotency unit** of the business operation, not browser state.

| Skill Type | Durable Unit | Checkpoint Cadence | Resume Token |
|------------|-------------|-------------------|--------------|
| Procurement ingestion | Notice batch / source page | Every page or every 25 notices | `last_source_notice_id`, `cursor_value` |
| Supplier enrichment | Candidate-source pair | After shortlist complete, then after normalized quotes | `candidate_id`, `query_fingerprint`, `after_listing_id` |
| Document acquisition | Document | Per downloaded document | `document_checksum`, `document_url` |
| Lot/product extraction | Lot or document | Per lot/document processed | `lot_id`, `document_id` |
| Bid portal automation | Verified save boundary | After each server-verified save/upload | `last_verified_portal_section`, `form_checkpoint_id` |

**Bad checkpoint units** (browser state, non-resumable): DOM element index, scroll pixel count, current tab number, "I clicked something."

Resume tokens are business-level identifiers. On resume, the daemon injects the resume token into the kickoff prompt so the agent can skip already-committed work.

**Concrete checkpoint format:**

```json
{
  "schema_version": "1.0",
  "job_id": "job-123",
  "skill_id": "supplier-price-enrichment/alibaba",
  "skill_version": "2026.03.12-4",
  "seq": 12,
  "kind": "candidate_source_batch",
  "stage": "shortlist_complete",
  "resume_from": {
    "candidate_id": "cand-7",
    "query_fingerprint": "qf-abc",
    "after_listing_id": "listing-443"
  },
  "progress": { "units_total": 5, "units_done": 3 },
  "payload_ref": "artifact://session/.../cp-12.json",
  "payload_digest": "sha256:..."
}
```

Daemon acceptance rules: schema valid, monotonic `seq`, skill/job identity match, digest valid, resume token present when required, idempotency keys valid, payload stored before checkpoint accepted.

**Resume logic examples:**

- **45 of 50 notices ingested, then website errors:** Keep the 45 if each batch was durably checkpointed. Resume from the last committed cursor with a small overlap/backtrack window. Do not restart from scratch unless cursor semantics are broken.
- **3 of 5 product prices found before failure:** Keep the 3 if the durable unit is candidate-source pair. Retry only the 2 missing units. Do not discard committed quotes because a later unit failed.
- **Bid portal failed after upload but before submit:** Resume only if the last safe boundary was server-verified. If there is any chance the submit boundary was crossed, resume enters **verify-only** mode first.

**8. Outcome Classes**

Skills return one of six standard outcome classes, not a binary success/failure:

| Status | Meaning | Daemon Action |
|--------|---------|--------------|
| `completed` | Full scope finished, all validation passed | Accept results, advance stage |
| `degraded_completed` | Partial but usable results with quality warnings | Accept results with degradation flag, surface warnings |
| `continuation_required` | Clean exit at a safe boundary with remaining work | Accept committed results, enqueue continuation segment |
| `blocked_human_action` | Human intervention required (CAPTCHA, MFA, manual approval) | Emit appropriate error code, alert operator |
| `suspect_output` | Schema-valid output that failed semantic or drift validation | Quarantine results, do not promote to pipeline, surface for review |
| `failed` | Unrecoverable error with specific failure code | Create error manifest, apply retry/cooldown policy |

The `failed` status carries structured failure metadata:

```json
{
  "status": "failed",
  "failure_code": "PAGE_SHAPE_DRIFT | AUTH_STATE_STALE | SOURCE_CAPTCHA_BLOCKED | SOURCE_SOFT_BLOCKED | LOCALE_OR_CURRENCY_DRIFT | AGENT_OUTPUT_CONTRACT_INVALID | ...",
  "retry_advice": "step_retry | session_retry | cooldown | resume | manual | none",
  "cooldown_seconds": 0,
  "resume_from": {},
  "warnings": []
}
```

The critical distinction `continuation_required` provides: a long-running skill that hits its time budget after 45 notices is not a failure. It emits a valid final contract with continuation data, and the Job Runner automatically enqueues the next segment using the emitted resume token. Crashes with only prior checkpoints remain `AGENT_PARTIAL_FAILURE`. Clean exits with continuation contract are a different, healthy class.

#### Browser Automation Reliability Patterns

**Anti-Bot, CAPTCHA, and Rate Limit Classification**

Skills do not "try harder" when blocked. They classify the block and apply the appropriate response:

| Condition | Immediate Action | Auto Retry |
|-----------|-----------------|------------|
| Navigation timeout before page fingerprint | Retry same step | 2 times: 5s then 20s backoff |
| Repeated timeout with no challenge markers | Restart clean session | 1 session restart, then fail |
| Soft block / challenge page / suspicious empty shell | Capture artifacts, report `SOURCE_SOFT_BLOCKED` | No immediate retry, set source cooldown |
| Hard CAPTCHA / MFA | Emit `blocked_human_action` with artifacts | None — requires human |
| Auth expired | One rehydrate attempt from stored auth state | 1 time, then `AUTH_STATE_STALE` |
| Geo / locale mismatch | Fail fast with `LOCALE_OR_CURRENCY_DRIFT` | None — requires different egress |
| Empty results with valid page fingerprint | Record as legitimate empty result set | None — valid business outcome |

The daemon backs this with a **source circuit breaker**: three challenge-class events in a rolling 30-minute window open the circuit for that source, set a cooldown in `source_runtime_state`, and suppress normal extraction jobs. During cooldown, only `auth-refresh/*` and `source-health/*` can run until a canary clears the source.

**Authentication Persistence**

Per-session browser isolation is the correct baseline. Re-authenticating from scratch every session is not.

The pattern:

1. Maintain **source-scoped auth state** per persona/account, managed by dedicated `auth-refresh/<source>` skills.
2. Inject stored auth state (cookies, local storage, IndexedDB snapshot) into a fresh isolated browser context at session start.
3. Verify logged-in identity before doing extraction work — if persona is wrong or auth is stale, fail fast with `AUTH_STATE_STALE`.
4. On auth expiry mid-run, attempt one rehydrate from stored auth state. If rehydrate fails, emit `blocked_human_action`.
5. Auth-refresh skills publish new auth-state handles that enrichment skills consume. This isolates login complexity from extraction complexity.

For multi-account sources (e.g., multiple Alibaba accounts), concurrent runs use unique accounts to avoid session interference.

**Dynamic Content and Infinite Scroll**

Never use "wait N seconds" as the primary readiness strategy. Use a readiness algorithm:

1. Page fingerprint passes (required anchors present, forbidden anchors absent).
2. First result card visible and content-populated (not skeleton loader).
3. Capture visible item IDs/titles.
4. Wait/scroll once.
5. Compare visible item IDs with previous capture.
6. Stop when no new IDs appear across two consecutive scroll cycles, or max scroll budget hit.

For list/search pages, skills declare: `maxScrollCycles` (hard cap), `maxDetailPageVisits` (drill-down cap), `stopCondition` ("no new IDs twice" or equivalent), `skeletonTimeout` (if skeleton loaders remain with no content change, classify as `PAGE_SHAPE_DRIFT`).

**Geo-Restriction and Locale Drift**

This is a data correctness problem, not just a networking problem. If Amazon flips from GBP to EUR, or Alibaba serves a different locale, the skill produces valid-looking garbage.

Every browser skill asserts at run start: expected country, expected currency, expected language, expected signed-in persona (if authenticated), expected seller/market variant. If any assertion fails, the skill fails fast with `LOCALE_OR_CURRENCY_DRIFT`.

The `egress_profile` in source config (not buried in skill prose) controls whether UK residential, CN exit, or default egress is used. Skills **detect and report** geo mismatch; the daemon/source config **decides** egress routing.

**Session Contamination**

Per-session browser profiles prevent cookie/lock contention, but additional isolation is required: isolated browser context (no shared live profile across concurrent sessions), isolated downloads directory per session, persona verification at run start (not assumed from profile contents), cache keys that include persona + locale + currency + egress profile (not just source + query). Do not randomize fingerprints aggressively — for production automation, stable and boring is safer than "stealthy and weird."

#### Skill Versioning and Lifecycle Management

**Version Binding at Enqueue**

At enqueue time, the daemon resolves each source to its bound skill versions:

| Binding | Purpose |
|---------|---------|
| `stable_version` | Scheduler default — runs authoritative extraction |
| `canary_version` | Optional — runs on small percentage or single source shard for validation |
| `shadow_version` | Optional — runs in parallel with stable for quality comparison without writing authoritative results |

"Latest on disk" never determines production behavior. Scheduled jobs bind explicit versions.

**Promotion Flow**

| Stage | Gate | Scope |
|-------|------|-------|
| **Draft** | Dev-only | Manual runs only, not scheduler-eligible |
| **Candidate** | CI lint + smoke canaries pass | CI-validated but not production-eligible |
| **Canary** | Smoke canaries pass against live source | Small percentage of production traffic or single source shard |
| **Shadow** | Quality comparison against stable on real workload | Full production inputs, but results are not authoritative |
| **Stable** | Shadow metrics meet promotion thresholds | Scheduler default for all production runs |
| **Rollback** | Any time | Rebind source to prior stable version immediately |

The rollback mechanism is "change source binding," not "edit markdown again." This makes rollback instantaneous and safe.

**Independent Version Dimensions**

| Dimension | What It Tracks | When It Bumps |
|-----------|---------------|---------------|
| `skill_version` | Playbook instructions and behavior | Any change to SKILL.md or scripts/ |
| `result_schema_version` | Final result contract shape | Contract-breaking changes to result.schema.json |
| `checkpoint_schema_version` | Checkpoint payload shape | Changes to checkpoint.schema.json |
| `validator_version` | Output quality validation logic | Changes to golden.assertions.yaml or validation scripts |
| `normalizer_version` | Deterministic data cleanup/normalization | Changes to normalization helpers in scripts/ |

A playbook update does not force a schema version bump unless the contract shape changes. This separation enables identifying whether a regression came from the playbook, the schema, or the validator.

**Schema Migration**

For result schema transitions (e.g., `v1.0 → v1.1`): daemon accepts both versions during a transition window, shadow comparison validates quality parity, dual-read/dual-validate if downstream consumers need adaptation time, promote when all consumers accept the new version, retire old version after the promotion window closes. Skill version and schema version move in lockstep only when the change is genuinely contract-breaking.

**Cache Invalidation**

Cache keys for supplier results include all relevant dimensions:

| Key Component | Why It Matters |
|--------------|---------------|
| Source | Different sources return different data |
| Query fingerprint | Different queries return different results |
| Candidate normalization version | Changed normalization invalidates cached matches |
| Skill major version | Major skill changes may extract different fields |
| Validator version | Changed validation may reclassify cached results |
| Persona | Different accounts may see different pricing/availability |
| Locale / currency | Market variant affects all pricing data |
| Egress profile | Different exit nodes may trigger different geo behavior |

For procurement normalization changes (not source changes), prefer **reprocessing from stored raw artifacts** over re-scraping. Raw artifact retention enables replay, not just forensics.

#### Cost Efficiency Patterns

The cheapest skill is the one that knows when to stop.

**Budget Declarations**

Every browser skill declares explicit cost budgets:

| Budget | Purpose | Example |
|--------|---------|---------|
| `maxPages` | Hard cap on paginated navigation | `10` |
| `maxScrollCycles` | Hard cap on infinite scroll iterations | `5` |
| `maxDetailPageVisits` | Cap on detail-page drill-downs | `15` |
| `maxQueryVariants` | Cap on search query attempts | `3` (exact → broad → stop) |
| `targetConfidentHits` | Early-stop threshold on strong matches | `3` |
| `timeBudgetMinutes` | Session time budget (triggers `continuation_required`) | `15` |
| `degradeThreshold` | When to accept degraded results and stop | Fill-rate < 20% after half the budget |

**Concrete Cost Patterns**

- Stop supplier search after 3 strong comparable hits with high `matchConfidence`, not after exhausting 30 weak candidates.
- Only visit detail pages for shortlisted result cards — list-page data is often sufficient for screening-grade quotes.
- Try the exact query wave sequence first, then one broader fallback, then stop.
- If shipping information is hidden behind login or interaction, record a screening-grade quote and move on.
- Segment long jobs with `continuation_required` rather than letting sessions bloat past their time budget.
- For procurement ingestion, batch by source: one session per source per run, not one session per notice.

**Cost Observability**

Cost per session alone is noisy. The meaningful metric is **cost per committed useful record**: cost per committed notice, per accepted quote, per qualified opportunity. Tracked per skill, per source, and trended over time to detect efficiency regressions after skill updates.

#### Skill Composition and Model Policy

**Composition Rules**

Skills can compose conceptually, but a production skill must not recursively invoke other skills inside one opaque session. Uncontrolled sub-skill recursion breaks cost attribution, resume logic, and debugging.

Composition uses one of two paths:

1. **Shared references/scripts within the skill directory:** Deterministic helper scripts in `scripts/` and reference documents in `references/` loaded into the agent's context. This is intra-skill composition.
2. **Daemon-orchestrated sub-jobs:** When a workflow requires multiple skill invocations (e.g., auth-refresh before enrichment, or document acquisition before product extraction), the daemon orchestrates them as separate jobs with independent result contracts and observability. This is inter-skill composition.

The daemon owns composition sequencing. Skills are leaf-level execution units.

**Model Policy**

Skills declare a `model_profile` that specifies the capability tier needed, not a vendor-specific model string:

| Profile | Use Case | Optimization |
|---------|----------|-------------|
| `browse_stable_low_cost` | Procurement ingestion, document acquisition, simple navigation | Cheapest model that can follow browser playbooks reliably |
| `structured_extract_low_cost` | Lot extraction, product extraction, field normalization | Cheapest model with strong structured output |
| `ambiguous_match_high_reasoning` | Borderline product matching, qualification of mixed contracts | Premium model for reasoning-heavy adjudication |
| `portal_form_high_reliability` | Bid portal form filling, submission preparation | Premium model where errors have operational consequences |

The daemon maps profiles to current best-fit models. This allows model upgrades without touching skill files, enables A/B testing of model assignments, and concentrates cost optimization in one place.

#### Failure Modes Requiring Explicit Skill-Level Detection

Beyond the standard error taxonomy, skills detect and report these production failure modes:

| Failure Mode | Detection | Why It's Dangerous |
|-------------|-----------|-------------------|
| Schema-valid garbage | Semantic + drift validation | Clean exit, valid JSON, but wrong page or bad extraction |
| Soft blocks disguised as empty results | Page fingerprint + challenge marker check | HTTP 200, no useful data, not technically an error |
| Auth half-expired state | Persona verification + privileged data presence check | Session loads but privileged data missing or persona changed |
| Locale/currency/persona drift | Pre-extraction assertion of expected market state | Same site, different market mode, silently poisoning comparisons |
| Pagination loops / duplicate scroll churn | Business-item dedup across scroll cycles | Skill keeps "working" but not discovering new items |
| Wrong artifact acquisition | Post-download content-type and structure validation | File is wrong format, corrupted, login HTML, or empty PDF |
| Unsafe resume after external side effect | Side-effect boundary tracking in portal skills | Especially dangerous in bid portal automation |
| Cache poisoning across versions/personas | Cache key includes all relevant dimensions | Old cached results reused under different normalization assumptions |


### 4.11 Licensing and Compliance Summary

| Dependency / Data Source | License / Terms Signal | Compatible | Notes |
|--------------------------|------------------------|------------|-------|
| Find a Tender data | Open Government Licence | Yes | official public data, accessed via agent skill scraping |
| Contracts Finder data | government open publication model | Yes | accessed via agent skill scraping |
| PCS API | official public API | Likely | accessed via agent skill scraping |
| HMRC Trade Tariff API | official public government API | Yes | direct API access for cost estimation |
| HMRC FPO API | official public government API | Yes | direct API access for HS code classification |
| HMRC Exchange Rate Service | official public government data | Yes | free, published monthly |
| Xe API | commercial | Yes | paid integration for commercial FX |
| Freightos | commercial | Yes | sales/commercial access required |
| SeaRates | commercial | Yes | commercial API access required |
| Easyship | commercial | Yes | parcel-mode freight and duty estimation (supporting source) |
| Shippo | commercial | Yes | parcel-mode carrier rates and international shipping (supporting source) |
| CurrencyBeacon | commercial | Yes | backup/comparison FX provider (optional) |
| OANDA | commercial | Yes | historical FX audit trail (optional) |
| Taric Support | commercial | Review | commodity-code validation — verify API terms for automated use |
| HS Ping | commercial | Review | HS-code comparison service — verify API terms |
| Dutify / Zonos / Avalara | commercial | Review | optional landed-cost comparison providers — used for validation only |
| eBay (browser scraping) | website terms of use | Review | agent skill scraping — review ToS for automated access |
| Amazon (browser scraping) | website terms of use | Review | agent skill scraping — review ToS for automated access |
| Alibaba/1688 (browser scraping) | website terms of use | Review | agent skill scraping — review ToS for automated access |

**License Summary:**

The core recommended architecture is license-compatible for cost-estimation APIs (HMRC, Xe, Freightos, SeaRates, Easyship, Shippo, CurrencyBeacon, OANDA). For procurement sources, the data is under open government licenses. APIs marked "Review" (Taric Support, HS Ping, Dutify/Zonos/Avalara) require verification of commercial terms for automated pipeline use. The main compliance consideration for the skill-based scraping approach is **website terms of use** for supplier marketplaces (eBay, Amazon, Alibaba). Agent skill scraping should be low-volume and source-specific, with ToS reviewed for each marketplace before implementation.

---

## 5. Technical Architecture Blueprint

### 5.1 System Context Diagram

```text
┌──────────────────────────────────────────────────────────────────────────────────┐
│                           SYSTEM CONTEXT                                         │
├──────────────────────────────────────────────────────────────────────────────────┤
│                                                                                  │
│   Procurement Sources (scraped via agent skills)                                 │
│   ┌────────────────────┐  ┌────────────────────┐  ┌────────────────────┐        │
│   │ Find a Tender      │  │ Contracts Finder   │  │ Public Contracts   │        │
│   │ (viewing portal/API)│  │ (viewing portal/API)│ │ Scotland           │        │
│   └─────────┬──────────┘  └─────────┬──────────┘  └─────────┬──────────┘        │
│             └──────────────┬────────┴──────────────┬────────┘                    │
│                            ▼                       ▼                             │
│   Supplier Sources (scraped via agent skills)                                    │
│   ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐                       │
│   │ eBay     │  │ Amazon   │  │ Alibaba  │  │ 1688     │                       │
│   └────┬─────┘  └────┬─────┘  └────┬─────┘  └────┬─────┘                       │
│        └──────┬──────┴──────┬──────┴──────┬──────┘                              │
│               ▼             ▼             ▼                                      │
│   ┌─────────────────────────────────────────────────────────────┐                │
│   │ Rust Daemon (Local Machine)                                 │                │
│   │ ┌─────────────────────────────────────────────────────────┐ │                │
│   │ │ Agent Session Runtime                                   │ │                │
│   │ │ - cron scheduler + manual triggers                      │ │                │
│   │ │ - adapter-mediated sessions (PTY or contract)           │ │                │
│   │ │ - skill loading + kickoff prompt injection              │ │                │
│   │ │ - forensic capture + protocol tracing                   │ │                │
│   │ │ - agent health monitoring                               │ │                │
│   │ └─────────────────────────┬───────────────────────────────┘ │                │
│   │                           ▼                                  │                │
│   │ ┌─────────────────────────────────────────────────────────┐ │                │
│   │ │ Local SQLite (Local Operational Authority)              │ │                │
│   │ │ - contracts, jobs, manifests, artifacts                 │ │                │
│   │ │ - agent session logs, error manifests                   │ │                │
│   │ │ - cursor/watermark state                                │ │                │
│   │ └─────────────────────────┬───────────────────────────────┘ │                │
│   │                           │ authority-based sync             │                │
│   └───────────────────────────┼──────────────────────────────────┘                │
│                               ▼                                                  │
│   ┌─────────────────────────────────────────────────────────────┐                │
│   │ Supabase / PostgreSQL (Shared Sync Target)                  │                │
│   │ - multi-user visibility                                     │                │
│   │ - shared configuration                                      │                │
│   │ - viewing portal data source                                │                │
│   └───────────┬───────────────────────────┬─────────────────────┘                │
│               │                           │                                      │
│               ▼                           ▼                                      │
│   ┌───────────────────────┐   ┌────────────────────────────────┐                │
│   │ Desktop (Tauri)       │   │ Viewing Portal                 │                │
│   │ CONTROL PANEL         │   │ READ-ONLY                      │                │
│   │ - trigger workflows   │   │ - browse opportunities         │                │
│   │ - manage agents       │   │ - view manifests               │                │
│   │ - make decisions      │   │ - view pipeline status         │                │
│   │ - view local data     │   │ - view errors                  │                │
│   │ (reads local SQLite)  │   │ (reads Supabase)               │                │
│   └───────────────────────┘   └────────────────────────────────┘                │
│                                                                                  │
│   Cost-Estimation APIs (direct API access, not agent-scraped)                    │
│   ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐        │
│   │ HMRC     │  │ HMRC FPO │  │ HMRC     │  │Freightos │  │ Xe /     │        │
│   │ Tariff   │  │ (HS code)│  │ FX Rates │  │+ SeaRates│  │ Currency │        │
│   └──────────┘  └──────────┘  └──────────┘  └──────────┘  └──────────┘        │
│                                                                                  │
└──────────────────────────────────────────────────────────────────────────────────┘
```

### 5.2 High-Level Component Design

Unless noted otherwise, the component inputs and outputs named below correspond directly to the §5.4 entities with the same names.
This section describes the target-state architecture. Early milestones intentionally implement subsets of these components per §11 and the file assignments in §8.4.

#### Component: Source Registry

**Responsibility:** Define procurement and supplier sources, associated skills, schedules, version bindings, and health/cooldown status. Static source definitions live in local SQLite `sources`; mutable challenge-rate and cooldown state lives in `source_runtime_state`.

**Interface:**

```ts
export interface SourceDefinition {
  id: string;
  kind: "fts" | "contracts_finder" | "pcs" | "ebay" | "amazon" | "alibaba" | "1688";
  category: "procurement" | "supplier";
  enabled: boolean;
  skillPath: string;  // path to the skill directory
  stableVersion?: string;
  canaryVersion?: string;
  shadowVersion?: string;
  scheduleCron?: string;
  lastHealthState?: "green" | "amber" | "red";
  cooldownUntil?: string;
  lastRunAt?: string;
  lastCursorValue?: string;
}
```

#### Component: PTY Session Manager

**Responsibility:** Create, monitor, and manage live PTY-backed sessions for agent execution. Captures PTY-mode raw terminal bytes as immutable forensic artifacts and manages the active PTY lifecycle (`created → spawned → active → terminating → exited`). Performs PTY-specific process-group supervision and hardening (fd hygiene, EIO handling, fixed terminal geometry). Final descendant cleanup, orphan sweep, and end-of-life cleanup verdicts are owned by `Session Reaper`.

#### Component: Daemon Cron Scheduler

**Responsibility:** Schedule and enqueue agent work based on cron definitions. Runs locally in the Rust daemon — no hosted scheduler dependency. Binds explicit skill id and version for scheduled jobs (no implicit skill discovery for cron). Determines when a job should exist; it does NOT execute the job payload itself. Performs catchup sweeps for overdue jobs when the daemon starts after extended downtime.

#### Component: Job Runner

**Responsibility:** Claim queued jobs from local SQLite, acquire the required resource budget from the Resource Arbiter, invoke the Agent Adapter in the correct execution mode, and persist stage outputs plus job status. After the Output Quality Gate passes, the Job Runner consults the Validation Policy Engine. If validation is required, it spawns a validator sub-job with `parent_job_id` linking back to the worker job. The Job Runner holds the worker job's final stage advancement until the verdict is rendered (or the validation policy says skip). On verdict: `approved` → advance stage normally; `rejected` → quarantine results as `AGENT_SUSPECT_OUTPUT`, create error manifest with validator's reasoning; `flagged` → advance stage with flag for operator review. Validator failure (crash, timeout, invalid verdict) is fail-open by default — the worker's results advance as if validation were skipped, with a `validation_skipped_validator_error` flag. The Job Runner owns execution of already-enqueued work; it does not decide schedules or replication semantics.

#### Component: Skill-Based Ingestion Runner

**Responsibility:** Domain-specific execution logic for procurement-ingestion jobs running under the Job Runner. It selects the procurement skill, invokes the Agent Adapter in the correct mode (PTY or contract per skill), normalizes the retrieved notice data, and writes immutable session artifacts to the artifact store. It is intentionally a job-type handler, not a second scheduler or runtime subsystem, and can remain a module under `jobs/runner.rs` until procurement-specific complexity justifies a separate handler.

#### Component: Log/Trace Capture Engine

**Responsibility:** Capture session data across the first three observability layers: (1) **control events** from the daemon (spawn, signal, exit, reap, cleanup verdict), (2) **protocol events** from the adapter (vendor JSONL/JSON events parsed into daemon-native events, validated checkpoints), and (3) **forensic bytes** (immutable raw PTY capture in PTY mode, raw stdout/stderr capture in contract mode). Control and protocol events drive workflow state; forensic bytes are for debugging and replay. Structured traces are derived primarily from control/protocol events; transcript parsing is best-effort enrichment only. The fourth observability layer (run manifests) is agent-generated and stored by the Run Manifest Writer — see §6.6 for the full four-layer model.

#### Component: SQLite Local Store

**Responsibility:** Local operational authority for all daemon-owned data. Stores pipeline data, jobs, manifests, agent session logs, error manifests, cursor state, and the sync outbox. Bundled with the Rust daemon. Split into two databases for operational isolation:

- **`state.db`**: Hot relational state — pipeline entities, jobs, manifests, outbox, sync metadata. WAL mode enabled. STRICT tables where json1 is not required.
- **`artifacts.db`** (or `artifacts/` directory): Large session transcripts (raw PTY bytes or raw stdout/stderr streams), raw document payloads, error context blobs. Separating these from the hot state DB prevents checkpoint and vacuum overhead from large writes.

Operational requirements: keep on local disk only (never NFS/cloud-sync folders), run `PRAGMA quick_check` on startup, run `PRAGMA optimize` on maintenance cadence, use Online Backup API or `VACUUM INTO` for safe hot backups. Store `schema_epoch` in `PRAGMA user_version`.

#### Component: Sync Engine

**Responsibility:** Authority-based replication between local SQLite and Supabase/Postgres. NOT a generic bidirectional sync engine. The SQLite Local Store owns persistence and local transactionality; the Sync Engine owns outbox construction, remote apply, pull cursors, and schema-epoch compatibility. Comprises:

- **Local outbox writer:** Writes outbox entries in the same SQLite transaction as business mutations. Every push-owned entity change produces an outbox row.
- **Push batch builder:** Reads outbox entries by `local_seq` range, constructs full-row projection batches with `schema_epoch`, `content_hash`, and `stage_run_id`.
- **Transactional remote apply RPC:** Submits each push batch to Supabase as a single atomic RPC. Handles dedup, gap rejection, and row-version comparison.
- **Remote pull cursor store:** Maintains per-domain pull cursors (`last_pulled_config_seq`, `last_pulled_decision_seq`). Applies remote-owned changes locally in order.
- **Stage publisher:** On pipeline stage completion, writes all stage outputs with shared `stage_run_id`, updates `process_stage_status` and advances `process_stage_heads` — all in the same transaction.
- **Full republish / remote rebuild mode:** For recovery scenarios — republishes all current local-owned state to remote, or bootstraps local from remote projections.
- **Schema epoch guard:** On startup and before each sync batch, validates `schema_epoch` compatibility between local and remote. Pauses sync (but not local operation) on mismatch.

#### Component: Relation Resolution Engine

**Responsibility:** Classify notice-pair relationships as `merged_same_process`, `related_not_merged`, `possible_duplicate`, or `separate`. Uses OCID matching, explicit notice references, hard blockers, and weighted heuristic scoring. Maintains `ProcessRelation` edges between related-but-unmerged processes to enable downstream cache sharing. See §4.2 for relation semantics and §5.6 for the resolution algorithm.

#### Component: Preliminary Qualification Engine

**Responsibility:** Reject near-expiry, non-goods, and non-bidable opportunities as early as possible using multi-dimensional classification. Evaluates four dimensions per lot:

- `legalCategory` — goods / services / works / mixed / unknown (from CPV codes and notice text).
- `goodsDensityScore` — proportion of the opportunity that is physical goods (0.0–1.0).
- `goodsSeparability` — whether goods can be bid on separately from bundled services/works.
- `accessRoute` — open tender / restricted / framework call-off / DPS / direct award / not-bidable (see §4.2).

Deterministic rules run first (CPV-based heuristics, expiry check, access route check); the `preliminary-qualification` skill is only used for mixed/unknown cases that need AI assistance.

#### Component: Document Acquisition Engine

**Responsibility:** After preliminary qualification, acquire publicly accessible tender documents associated with qualified notices/lots. Targets: specification PDFs, pricing schedules, lot documents, BoQ spreadsheets, tender appendices. The `document-acquisition` skill navigates to the notice source and downloads available public documents. Stores documents as raw artifacts linked to the notice. Without this stage, product extraction operates on vague notice summaries and produces low-quality candidates.

#### Component: Lot Extraction Engine

**Responsibility:** Extract lot structure from normalized notices and acquired documents. Each lot becomes a `NoticeLot` entity with its own CPV codes, estimated value, deadline, and scope description. If a notice has no explicit lots, create a synthetic whole-notice lot. Lot extraction is primarily deterministic (parsing structured OCDS lot data) with AI assist for unstructured lot descriptions in documents.

#### Component: Product Extraction Engine

**Responsibility:** Convert lot-level descriptions and acquired documents into structured `ProductCandidate` entities. Each candidate gets:

- A `candidateType` classification: `exact_sku`, `standard_commodity`, `brand_family`, `bundle`, `custom_or_fabricated`, `installed_goods`, `regulated_goods`, `unknown`.
- A `matchArchetype` classification: `standard_class_led`, `dimension_material_led`, `electrical_spec_led`, `pack_count_led`, `compatibility_led`, `model_exact`, `unknown`. This describes how product identity is recovered on marketplaces — the matching strategy axis — and is orthogonal to `candidateType` (see §4.3).
- Structured slots: `headNoun`, `criticalAttributes`, `numericSpecs`, `standards`, `material`, `brandModel`, each annotated with a `slotCriticality` role (`required`, `preferred`, `descriptive`, `exclusion`).
- Separate confidence scores: `identityConfidence` (how well the product is identified), `specConfidence` (how complete the specification is), `quantityConfidence` (how reliable the quantity estimate is).

Slot criticality is determined from tender language signals ("must", "shall", "required" → `required`; "preferred", "desirable" → `preferred`; "not", "excluding" → `exclusion`). When ambiguous, defaults to `preferred`. The `matchArchetype` provides category-level defaults (e.g., standards slots default to `required` for `standard_class_led` products).

Bundles are decomposed into constituent candidate families before downstream processing.

#### Component: Quoteability Scorer

**Responsibility:** Determine whether each `ProductCandidate` is worth sending to marketplace search. Uses a deterministic rules-based score:

```text
quoteabilityScore =
  + commodity-like head noun present
  + critical attributes present for the category
  + numeric specs present where category needs them
  + standards/certifications present where category needs them
  + quantity/unit or pack basis present
  + public tender docs available
  + open/bidable access route
  - custom fabrication
  - installation-dominant scope
  - closed framework/call-off
  - regulated goods with missing compliance detail
```

Policy thresholds:

- `>= 0.65` → search automatically
- `0.45–0.64` → search only top-value/top-risk candidates
- `< 0.45` → skip marketplace search; route to watch/RFQ/manual

This gate saves more pipeline cost than most infrastructure optimizations.

Additionally, the Quoteability Scorer computes a `recallRiskScore` per candidate that drives adaptive shortlist budgets during price-signal matching (see §5.7). `recallRiskScore` is computed from: alias drift for the `matchArchetype`, compliance criticality (`regulated_goods` = high), accessory contamination risk for the product family, spec sparsity on the candidate, expected source noisiness per routed source, and language mismatch risk for cross-language queries.

#### Component: Supplier Quote Engine

**Responsibility:** Trigger agent skills for supplier price scraping via adapter-mediated sessions, routed by candidate type (see §4.3 source routing table). In practice these runs are usually PTY-backed because marketplace search is browser-driven, but the component consumes the adapter abstraction rather than a PTY-only contract.

The search pipeline executes in stages: (1) query-wave planning from the search profile's structured `queryWaves`, generating a sequence of queries from most precise to broadest per candidate × source; (2) search execution collecting de-duplicated results across all waves; (3) search-card screening applying four-state evidence comparison (`match`/`missing`/`ambiguous`/`conflict`) with only `conflict` triggering hard rejection; (4) detail-page enrichment for plausible results to extract structured attributes, compliance evidence, and pack/unit details not visible on search cards; (5) diversified deterministic shortlisting with adaptive budget driven by `recallRiskScore`; (6) AI adjudication on borderline cases only, with budget scaling by recall risk.

Produces two separate confidence scores per quote: `matchConfidence` (product identity, with archetype-specific weight profiles) and `quoteUsabilityConfidence` (commercial usability of the observed price). Assigns `complianceEvidenceTier` (`structured_attribute`, `title_body_mention`, `absent`) for regulated goods. Applies lexicographic gating: identity gate first, then usability gate, then price-signal role assignment (see §4.3). Maintains a two-tier cache (search-result + listing-evidence) for cost optimization.

When search returns zero usable results, applies the zero-result recovery ladder (see §5.7) and classifies failures into `searchFailureMode` categories for operational visibility and lexicon improvement.

#### Component: Landed-Cost Engine

**Responsibility:** Combine ex-works or seller price with freight, duty, VAT, fees, and FX to produce gross and net margin estimates at the **line-item level**, then roll up to process-level estimates. Uses cost-estimation APIs (HMRC Tariff, HMRC FPO, Freightos, SeaRates, Xe, HMRC Exchange Rates) — this is the one area where direct API access is preferred over agent scraping.

The engine operates at two grades:
- **Screening grade:** Automated across the whole market. Uses indicative marketplace pricing, FPO-generated HS6/CN8 candidates, and scenario-based freight estimates. Sufficient for early-stage filtering and gross margin gating.
- **Decision grade:** Automated only for repeated product families where the system has a proven codebook (confirmed 10-digit codes, cached ATRs/AVRs), confirmed supplier economics (RFQ or negotiated quotes), and known freight patterns. Everything else requires operator review or RFQ before final qualification.

Key subsystems:
- **Classification pipeline:** Description preprocessing → FPO candidate generation → 10-digit resolution → measure risk scanning
- **Revenue normalizer:** Convert notice values to expected addressable revenue on an ex-VAT basis, accounting for framework ceilings, dynamic-market structures, and VAT-inclusive notice values
- **Quote normalizer:** Normalize supplier quotes to a common cost boundary based on Incoterm before comparison
- **Freight estimator:** Provider-abstracted, mode-aware, with dual packing scenarios and low/base/high estimates
- **Customs engine:** Duty, trade remedies, VAT treatment, valuation method, special procedures, operational blockers
- **FX engine:** Dual-rate (customs + commercial), basis gap tracking, margin-triggered recalculation

#### Component: Manifest Generator

**Responsibility:** Produce human-readable Markdown summarizing the opportunity, assumptions, evidence, and recommendation. The canonical structure lives in `packages/manifests`; the `manifest-generation` skill can supply narrative synthesis, but it does not replace the deterministic manifest renderer.

#### Component: Bid Workflow Executor

**Responsibility:** Launch portal-specific agent skills via adapter-mediated sessions for request pipeline automation. Portal workflows are normally PTY-backed because they are browser-driven, but the daemon still enters through the adapter boundary so lifecycle, health, and result validation stay consistent with the rest of the runtime. Note: this automates the request pipeline, not real-money bid submissions. Actual negotiations and fulfillment are handled manually by humans.

#### Component: Error Manifest System

**Responsibility:** Track and flag pipeline failures with structured error-manifest records. For each failure, records: which workflow step failed, the error message, the pipeline stage, enough context to debug. Generates error manifest files (Markdown) and surfaces errors in the desktop control panel and viewing portal. Error manifests include CLI vendor + version, adapter version, model id, config hash, skill version, pid/pgid, cleanup verdict, and last valid checkpoint sequence.

#### Component: Agent Adapter Layer

**Responsibility:** Vendor-specific agent launch, capability probing, auth probing, mode selection (PTY vs. contract), protocol parsing, and result validation for Claude and Codex CLIs. Each adapter produces a canonical daemon event stream (`SessionStarted`, `CheckpointWritten`, `AuthChallenge`, `SessionStalled`, `CleanupIncomplete`, `FinalResultValidated`). Unknown CLI versions enter canary mode until the adapter proves them safe. The adapter isolates the rest of the daemon from vendor CLI changes.

#### Component: Result Contract Validator

**Responsibility:** Validate agent checkpoints and final result artifacts against the skill's declared `result_schema_version`, then pass accepted artifacts through the Output Quality Gate for semantic and behavioral/drift checks. Enforces the rule: **success = valid final result contract + clean process exit + complete cleanup/reap.** Partial results with valid checkpoints before a crash are classified as `partial_failure`, never upgraded to success. Outputs that are schema-valid but fail the quality gate are classified as `suspect_output`, quarantined, and never committed to pipeline state. Validates schema version, job identity, checksum, and checkpoint sequencing. OQG signals (syntactic/semantic/behavioral-drift check results) are packaged as inputs to the semantic validator when validation runs — OQG remains the first gate; the semantic validator is the second gate. The two serve different purposes: OQG catches structural and statistical anomalies via deterministic code; the semantic validator catches contextual errors via agent-based run manifest review.

#### Component: Resource Arbiter

**Responsibility:** Admit agent jobs using typed resource budgets instead of a flat session cap. Manages three quota types: **browser slots** (hard cap — each browser-backed session consumes a slot), **agent slots** (soft cap — total concurrent adapter-mediated sessions), and **source/domain lanes** (rate cap — prevent overwhelming a single target). Admission priority is: (1) procurement ingestion, (2) auth refresh / health probes, (3) qualification, (4) supplier enrichment, (5) manifest generation, (6) bid automation.

**MVP operating limits:** Start with `browser_slots = 2`, `agent_slots = 4`, `procurement_source_lane = 1` per source, and `supplier_source_lane = 2` per source. Skill-level `max_concurrency` can lower these ceilings but never raise them.

#### Component: Artifact Store

**Responsibility:** Store immutable compressed raw session-output streams, result contract files, checkpoint artifacts, run manifests, and large session outputs outside of hot SQLite tables. SQLite stores session metadata, byte counts, tail excerpts, artifact paths, SHA-256 hashes, and derived event summaries. Raw artifacts are chunked, compressed, and stored on the local filesystem at `artifacts/sessions/YYYY/MM/<session_id>/`. Run manifests (structured Markdown narratives of what a skill run did) are stored at `artifacts/sessions/YYYY/MM/<session_id>/run-manifest.md` and referenced from `agent_session_logs` via `run_manifest_path` and `run_manifest_hash`.

#### Component: Session Reaper

**Responsibility:** Descendant process-tree cleanup after session completion or failure. Performs kill escalation (in-band interrupt → SIGTERM group → SIGKILL group), PTY drain, explicit wait/reap, and orphan sweep. Records cleanup verdict (`clean`, `forced`, `leaked`) as a first-class session attribute. On daemon startup, sweeps for orphaned sessions from prior runs and cleans up leaked processes. On Linux, optionally leverages `systemd-run --scope` for per-session resource control.

#### Component: Kickoff Prompt Renderer

**Responsibility:** Compose the standardized kickoff prompts used by worker, validator, manual-mode, and dry-run executions. The renderer owns the shared preamble, context injection (skill content, job parameters, resume token, company profile), the run manifest instruction block, and scratch-path routing for dry-run artifacts.

**Manual mode:** Emits copy-paste-ready prompts for a fresh Codex/Claude session so draft skills can be explored without scheduler involvement or authoritative state changes.

**Dry-run mode:** Uses the same prompt templates but routes worker and validator artifacts to `artifacts/scratch/<run_id>/`, records run manifests and validation verdicts for review, and suppresses authoritative pipeline writes, outbox entries, and stage-head advancement.

#### Component: Run Manifest Writer

**Responsibility:** After a skill completes (success or failure), the agent writes a human-readable Markdown narrative describing what it did, what it found, what decisions it made, and any warnings. The run manifest is written BY the agent as part of the skill's output — it is embedded in the kickoff prompt as a standardized instruction block. The daemon stores the produced manifest as an immutable artifact and records its path and SHA-256 hash in `agent_session_logs`. The run manifest is the primary input for semantic validation — it is purpose-built for a validator to review, unlike raw forensic bytes which are too large and noisy.

**Run manifest format:** YAML frontmatter with structured metadata (manifest_version, skill_id, skill_version, job_id, session_id, execution_mode, started_at, completed_at, outcome_class, items_processed/skipped/failed, warnings_count, checkpoint_count, pages_visited, source_url, page_fingerprint_matched, locale_verified, currency_verified) followed by Markdown sections: Summary, Steps Taken, Decisions Made, Warnings, Evidence. Target size: < 50 KB per run manifest — the manifest is a focused narrative summary, not a log dump. If processing hundreds of items, the manifest summarizes patterns and highlights anomalies rather than enumerating every item.

**Terminology disambiguation:** Three things are called "manifest" in this architecture: (1) **opportunity manifest** — the Markdown summary of an opportunity (§5.2 Manifest Generator, stored in `manifests` table); (2) **error manifest** — the structured pipeline failure record (§5.2 Error Manifest System, stored in `error_manifests` table); (3) **run manifest** — the MD narrative of what a skill run did (this component, stored in artifact store). Always use the full term "run manifest" when referring to this concept.

#### Component: Validation Policy Engine

**Responsibility:** Determine whether a completed job requires semantic validation based on the skill's `validation_policy` metadata, the skill's promotion stage, and trigger conditions. The engine evaluates the `always` / `sample` / `trigger` / `skip` policy modes and returns a concrete runtime decision: validate or skip.

**Decision flow:**

1. Schema validation failed? → No validator (already quarantined as `AGENT_OUTPUT_CONTRACT_INVALID`).
2. Skill in draft/candidate/canary/shadow stage? → **ALWAYS VALIDATE** regardless of declared policy.
3. Skill in stable with `validation_policy: always`? → **VALIDATE**.
4. Skill in stable with `validation_policy: sample`? → Roll random against `validation_sample_rate`, validate if match.
5. Skill in stable with `validation_policy: trigger`? → Check trigger conditions (fill_rate_drop, first_run_after_version_change, suspect_output_in_last_5, operator_requested). Validate if any trigger fires.
6. Skill in stable with `validation_policy: skip`? → **SKIP** (OQG code checks still run).
7. Any trigger condition met regardless of policy? → **VALIDATE**.

**Composability:** The policy engine reads validation policy from skill metadata — no daemon code changes are needed to adjust validation behavior for any skill. Adding validation to a new skill requires only setting the `validation_policy` field in the skill's metadata.

#### Component: Validation Executor

**Responsibility:** Spawn validator sub-jobs via the Job Runner. Load the base validator skill (`skills/validation/base/SKILL.md`) plus any domain-specific overlay from the worker skill's directory (`validator-overlay.md`). Compose the validator's kickoff prompt with: the run manifest (full text), result contract (full or sampled), `golden.assertions.yaml` from the worker skill directory, OQG semantic and drift signals, and historical run summary (last N outcomes for trend context). The validator skill does NOT see raw forensic bytes (too large, too noisy) or the worker's internal reasoning (prevents anchoring bias).

**Verdict processing:** The validator renders a verdict with structured issues:

- **`approved`** — Worker's original outcome class preserved; normal commit.
- **`rejected`** — Results quarantined as `AGENT_SUSPECT_OUTPUT`; error manifest created with validator's reasoning and suggested fixes; operator alerted.
- **`flagged`** — Results accepted with flag for operator review; non-critical issues recorded.

**Fail-open principle:** If the validator itself fails (crash, timeout, invalid verdict schema), the daemon treats this as a validator error, NOT a worker error. Worker results advance as if validation were skipped, with `validation_skipped_validator_error` flag. Broken validators never block the pipeline — workers operated without validators before; validation is an improvement layer, not a hard dependency. Validation is non-recursive: validator skills do NOT have their own validators. If a validator skill is unreliable, it is fixed through the same skill promotion flow as worker skills (golden verdict tests, calibration against human judgments), not by adding a meta-validator.

**Cost profile:** Validators use `headless_contract` mode with `structured_extract_low_cost` model profile. Typical cost: ~$0.01–0.03 per validation. Even "always validate" adds only 5–15% overhead over worker cost.

**Multi-segment validation:** When a worker skill exits with `continuation_required`, each segment is validated independently. The validator does NOT wait for all segments to complete — each segment's run manifest is self-contained, and each segment's results are independently committed via checkpoints. If a segment is rejected, only that segment is quarantined; prior committed segments remain. This prevents validation from blocking long-running multi-segment jobs.

#### Architectural Coherence Notes

- **Component purpose / merge guidance:** Every component remains justified, but some are intentionally layered rather than fully standalone. `Skill-Based Ingestion Runner` is best understood as a procurement-specific job handler under `Job Runner`, not a parallel execution stack. `Manifest Generator` remains separate from the `manifest-generation` skill because deterministic rendering is authoritative and the skill contributes optional narrative synthesis only. `Log/Trace Capture Engine` and `Artifact Store` stay separate because capture decides what to record while storage decides where/how to retain it.
- **Qualification vs extraction boundary:** Preliminary Qualification decides whether a notice/lot is commercially worth deeper work using expiry, access route, and goods-shape signals. Product Extraction begins only after a notice survives that gate and documents/lots are available; it does not decide bidability.
- **Scheduler vs runner boundary:** Daemon Cron Scheduler decides when work should be enqueued. Job Runner claims and executes queued work. This keeps schedule policy separate from runtime execution and retry behavior.
- **Store vs sync boundary:** SQLite Local Store is the local persistence authority. Sync Engine is the only component that translates local rows into remote projections or pulls remote-owned state back down.
- **Dependency direction:** Skills are passive assets loaded by the daemon/runtime; the daemon does not depend on skill internals for state management. The viewing portal depends only on Supabase projections, never on local SQLite. The landed-cost engine depends on upstream candidate/quote outputs, while upstream extraction and qualification do not depend on landed-cost outputs. Downstream review states may consume landed-cost outputs, but landed-cost computation itself does not feed back into extraction or matching logic.
- **Coupling / cycle rule:** The intended graph is acyclic for automatic processing: ingestion → qualification → documents/lots → product extraction → quoteability → supplier matching → landed cost → manifest/review. Human/operator actions may trigger reruns, but no automatic component should depend on a downstream component that already depends on it.
- **Complexity allocation:** The architecture intentionally spends the most component detail on the two hardest problems: PTY/agent orchestration (`PTY Session Manager`, `Agent Adapter Layer`, `Log/Trace Capture Engine`, `Resource Arbiter`, `Session Reaper`) and product/pricing comparability (`Product Extraction Engine`, `Quoteability Scorer`, `SearchProfile`, `Supplier Quote Engine`). The landed-cost engine remains Very High complexity, but it is deliberately kept downstream and API-driven so it does not distort the core orchestration boundaries.

### 5.3 Canonical Workflow

```text
new notice
  -> normalize
  -> relation resolution (merge | related_not_merged | possible_duplicate | separate)
  -> preliminary qualification (per lot)
     -> reject if expiry < 30 days (exception: DPS/dynamic market notices)
     -> classify access route (open_tender / restricted / framework_call_off / dynamic_market / direct_award / closed_framework_award / not_bidable)
     -> reject if not-bidable access route
     -> classify goods dimensions:
        - legalCategory (goods / services / works / mixed / unknown)
        - goodsDensityScore (0.0–1.0)
        - goodsSeparability (true / false / uncertain)
     -> reject if pure services or works with negligible goods density
     -> mixed/unknown -> AI classification via skill
  -> acquire public documents (spec PDFs, pricing schedules, BoQ, lot docs)
  -> extract lots (or create synthetic whole-notice lot)
  -> extract product candidates (per lot)
     -> classify candidateType (exact_sku / standard_commodity / brand_family / bundle / custom_or_fabricated / installed_goods / regulated_goods / unknown)
     -> decompose bundles into constituent candidate families
     -> extract structured slots: headNoun, criticalAttributes, numericSpecs, standards, material, brandModel
  -> score quoteability (deterministic rules-based gate)
     -> >= 0.65: search automatically
     -> 0.45–0.64: search only top-value/top-risk candidates
     -> < 0.45: skip marketplace search; route to watch/RFQ/manual
  -> build structured search profiles (multilingual query waves, not raw text)
  -> price-signal matching (via agent skill scraping, routed by candidate type)
     -> query-wave search: exact anchor → alias swap → market framing → broad → compliance-off
     -> search-card screening: four-state evidence (match/missing/ambiguous/conflict), reject on conflict only
     -> detail-page enrichment for plausible results (compliance evidence tier determination)
     -> adaptive diversified shortlist: budget driven by recallRiskScore (top-8 to top-20)
     -> AI adjudication: borderline only, budget scales with recallRiskScore
     -> score matchConfidence and quoteUsabilityConfidence separately
     -> classify priceSignalRole (benchmark_ceiling / optimistic_floor / executable)
  -> gross margin estimate (normalized ex-VAT revenue vs normalized supplier cost)
     -> if gross_margin >= 30% and gross_estimate_confidence >= confidence_floor, continue
     -> if gross_estimate_confidence < confidence_floor, route to review
     -> otherwise reject or deprioritize
  -> shipping + customs + FX (landed-cost engine)
  -> net margin band estimate
     -> if netMarginLowPct >= 20% and no critical blockers, qualify
     -> if netMarginBasePct >= 20% but pricing or blocker uncertainty remains, keep as review / RFQ
     -> otherwise keep as non-qualified
  -> generate opportunity manifest
  -> semantic validation (if validation policy requires: worker run manifest reviewed by validator skill after OQG)
     -> approved: proceed to operator review
     -> rejected: quarantine results, create error manifest with validator reasoning
     -> flagged: proceed with flag for operator review
     -> dry-run mode: verdicts logged, scratch artifacts retained, no authoritative pipeline advancement
  -> operator review
     -> reject
     -> watch
     -> escalate to RFQ (for high-value opportunities with indicative-only pricing)
     -> bid workflow
```

### 5.4 Data Model Design

#### SQLite Compatibility Notes

The primary data store is local SQLite bundled with the Rust daemon. Key differences from Postgres:

- **No native array types:** Use TEXT with JSON arrays (via json1 extension) or junction tables instead of `text[]`.
- **No `gen_random_uuid()`:** Generate UUIDs in the application layer (Rust `uuid` crate).
- **No `timestamptz`:** Store timestamps as TEXT (ISO 8601 format) for readability and portability.
- **No `jsonb`:** Use TEXT with JSON. SQLite's json1 extension provides json_extract(), json_each(), etc.
- **Single-writer model:** SQLite supports one writer at a time — this suits the single-daemon architecture.
- **WAL mode:** Enable WAL (Write-Ahead Logging) for concurrent reads during writes.

The SQL tables below are shown in Postgres syntax for the Supabase sync target. The daemon's SQLite schema mirrors these with appropriate type substitutions.

#### Sync Authority and Change Tracking

The authority matrix (see §4.7) governs which database owns each table. This section specifies the concrete change-tracking columns.

**Worker-validator additions in this section:** the data model carries the worker-validator system through four concrete changes: (1) `workflow_jobs` adds `parent_job_id`, `validation_required`, and `validation_verdict_id`; (2) `agent_session_logs` adds `run_manifest_path` and `run_manifest_hash`; (3) `validation_verdicts` stores the full local validator verdict; and (4) `validation_verdict_summaries` stores the push-owned remote projection for viewing-portal visibility.

**Push-owned mutable entities** (local → remote projections) carry:

- `owner_executor_id text not null` — which executor produced this row
- `row_version integer not null default 1` — monotonic per row, incremented only by the owner; used for remote apply (NOT `updated_at`)
- `last_local_seq integer` — the outbox sequence that last changed this row (for sync correlation)
- `stage_run_id text` — for stage-produced rows, links all outputs of a single stage execution
- `updated_at text not null` — ISO 8601, for human audit only, never for conflict resolution
- `created_at text not null` — ISO 8601

**Pull-owned entities** (remote → local) carry:

- `remote_change_seq bigint` — monotonic, assigned by remote on every insert/update/delete
- `remote_revision bigint not null default 1` — per-row revision, used for compare-and-swap on shared human decisions
- `deleted_at timestamptz` — tombstone for shared-delete semantics (only on remote-owned entities; see §4.7 Tombstone Policy)

**Local-only entities** (no sync) carry only standard `created_at` / `updated_at` as needed.

**Schema migration coordination:**

- Define a canonical logical schema; generate/map from it into SQLite and Postgres DDL.
- Store `schema_epoch` in SQLite `PRAGMA user_version`, in a Postgres `schema_metadata` table, and in every sync batch envelope.
- On daemon startup, if epochs are incompatible, pause sync but keep local operation.
- Use **expand / migrate / contract** rollouts only — no one-step destructive hot migrations on synced tables.
- In CI: apply both SQLite and Postgres migrations from scratch, introspect both, run a sync round-trip test, fail if logical schema hash differs.

**Monetary values:** Store money as **integer pence** (e.g., `supplier_cost_gbp_pence integer`) and percentages as **basis points** (e.g., `gross_margin_bps integer`) for synced tables. This eliminates floating-point divergence between SQLite and Postgres numeric semantics.

**Array columns:** Use **junction tables** for query-heavy array fields (`cpv_codes`, `candidate_commodity_codes`). Use JSON TEXT only for opaque evidence payloads and raw data where relational querying is not needed.

#### Data Model Notes for Data-Architecture Domain

The following new entities and field changes are required based on the procurement pipeline and product matching review. These should be coordinated with the data-architecture domain:

**New entities required:**

- **`NoticeLot`** — lot-level decomposition of notices. Fields: `id`, `noticeId`, `sourceLotId`, `lotNumber`, `title`, `description`, `cpvCodes`, `estimatedValue`, `deadlineAt`, `isBidableSeparately`, `documents[]`. If no explicit lots, create a synthetic whole-notice lot. `ProductCandidate` should reference `lotId`, not just `noticeId`.
- **`ProcessRelation`** — edges between related-but-unmerged processes. Fields: `id`, `processIdA`, `processIdB`, `relationType` (`merged_same_process` | `related_not_merged` | `possible_duplicate` | `separate`), `confidence`, `evidence`, `createdAt`. Enables cache sharing across related clusters without forced merges.
- **`SearchProfile`** — structured search profile per product candidate. Fields: `id`, `productCandidateId`, `headNoun`, `headNounFamily`, `candidateType`, `matchArchetype`, `criticalAttributesJson`, `standardsJson`, `numericSpecsJson`, `nounAliasesEn`, `nounAliasesZh`, `marketFrameEn`, `marketFrameZh`, `useContext`, `sectorContext`, `requiredAnchors`, `optionalAnchors`, `mustNotHave`, `preferredTerms`, `excludedTerms`, `queryWavesJson`, `recallRiskScore`, `createdAt`. Persisted for debugging, recall measurement, and lexicon improvement.
- **`SearchAttempt`** — tracks each query execution for debugging, recall measurement, and failure diagnosis. Fields: `id`, `productCandidateId`, `searchProfileId`, `source`, `locale`, `waveNumber`, `queryText`, `filtersJson`, `isRecoveryWave`, `resultCount`, `hardRejectedCount`, `shortlistedCount`, `searchFailureMode`, `executedAt`, `sessionId`. Push-owned.
- **`ListingEvidence`** — cached listing evidence for re-scoring across candidates without re-fetching. Fields: `id`, `source`, `sourceListingId`, `normalizedTitle`, `structuredAttributesJson`, `parsedStandardsJson`, `parsedUnitsJson`, `sellerIdentifier`, `minimumOrderQuantity`, `shippingTerms`, `condition`, `complianceEvidenceTier`, `intendedUseLanguage`, `isReadyToShip`, `fetchedAt`, `detailPageFetched`. Local-only cache (no sync).

**Field additions to existing entities:**

- **`ContractNotice`**: add `accessRoute` (`open_tender` | `restricted_tender` | `framework_call_off` | `dynamic_market` | `direct_award` | `closed_framework_award` | `not_bidable`).
- **`ProductCandidate`**: add `lotId`, `candidateType` (`exact_sku` | `standard_commodity` | `brand_family` | `bundle` | `custom_or_fabricated` | `installed_goods` | `regulated_goods` | `unknown`), `matchArchetype` (`standard_class_led` | `dimension_material_led` | `electrical_spec_led` | `pack_count_led` | `compatibility_led` | `model_exact` | `unknown`), `headNoun`, `criticalAttributesJson` (now with slot criticality annotations), `numericSpecsJson` (now with tolerance windows and criticality), `standardsJson` (now structured: family/part/edition/amendment/class with criticality), `excludedTermsJson`, `quoteabilityScore`, `recallRiskScore`, `identityConfidence`, `specConfidence`, `quantityConfidence`. The existing single `extractionConfidence` should be replaced by the three granular confidence fields.
- **`SupplierQuote`**: add `matchConfidence`, `quoteUsabilityConfidence` (replacing single `confidence`), `priceSignalRole` (`benchmark_ceiling` | `optimistic_floor` | `executable`), `listingPackQty`, `listingPackUnit`, `normalizedUnitBasis`, `isUsedOrRefurbished`, `isReadyToShip`, `vatInclusive`, `shippingBasisConfidence`, `complianceEvidenceTier` (`structured_attribute` | `title_body_mention` | `absent`), `rawMatchScore`, `searchAttemptId`, `sourceListingId`, `evidenceStatesJson`, `queryWaveNumber`.
  - **Cross-domain note:** The data-architecture domain added `quoteRealismConfidence` (how realistic the observed price is for procurement-volume orders) and `negotiationUpliftPct` (expected uplift from indicative to negotiated price) to the `SupplierQuote` interface. These are **complementary to, not duplicates of**, the procurement pipeline's `quoteUsabilityConfidence` and `priceSignalRole`. The procurement pipeline fields assess the listing's commercial usability (unit basis, MOQ, shipping visibility); the data-architecture fields assess volume-adjusted realism and negotiation gap. Implementers should compute and store all four fields independently.
- **`QualificationResult`**: add `legalCategory`, `goodsDensityScore`, `goodsSeparability` (`true` | `false` | `uncertain`), `accessRoute`. The existing `goodsQualified` boolean should be supplemented (not replaced) by these dimensions.

#### Core Entities

Source notices and canonical opportunities should not be conflated. A single procurement process can have multiple notices across time and, in some cases, across publication surfaces. The source-specific notice model should therefore feed a canonical process/opportunity model.

The TypeScript interfaces below describe the logical application contract. Persisted SQLite/Postgres columns use integer pence and basis points where noted in the SQL schema.

```ts
export interface ProcurementProcess {
  id: string;
  canonicalOcid?: string;
  primaryNoticeId?: string;
  currentStage:
    | "pipeline"
    | "planning"
    | "tender_open"
    | "award"
    | "contract"
    | "termination";
  buyerName?: string;
  title: string;
  latestDeadlineAt?: string;
  latestPublishedAt: string;
  latestUpdatedAt: string;
}
```

```ts
export interface ContractNotice {
  id: string;
  processId: string;
  source: "fts" | "contracts_finder" | "pcs" | string;
  sourceNoticeId: string;
  ocid?: string;
  title: string;
  summary?: string;
  buyerName?: string;
  procurementStage:
    | "pipeline"
    | "planning"
    | "tender_open"
    | "award"
    | "contract"
    | "termination";
  accessRoute?:
    | "open_tender"
    | "restricted_tender"
    | "framework_call_off"
    | "dynamic_market"
    | "direct_award"
    | "closed_framework_award"
    | "not_bidable";
  deadlineAt?: string;
  publishedAt: string;
  updatedAt: string;
  mainCategory?: "goods" | "services" | "works" | "mixed" | "unknown";
  cpvCodes: string[];
  estimatedValueOriginal?: number;
  estimatedValueCurrency?: string;
  estimatedValueGbpExVat?: number;
  estimatedValueVatInclusive?: boolean;
  rawDocumentId: string;
}
```

```ts
export interface NoticeLot {
  id: string;
  noticeId: string;
  sourceLotId?: string;
  lotNumber?: number;
  title?: string;
  description?: string;
  cpvCodes: string[];
  estimatedValue?: number;
  deadlineAt?: string;
  isBidableSeparately: boolean;
  documents?: string[];
}
```

```ts
export interface QualificationResult {
  processId: string;
  noticeId: string;
  expiryQualified: boolean;
  goodsQualified: boolean;
  goodsConfidence: number;
  legalCategory?: "goods" | "services" | "works" | "mixed" | "unknown";
  goodsDensityScore?: number;
  goodsSeparability?: boolean | "uncertain";
  accessRoute?:
    | "open_tender"
    | "restricted_tender"
    | "framework_call_off"
    | "dynamic_market"
    | "direct_award"
    | "closed_framework_award"
    | "not_bidable";
  rationale: string[];
  qualifiedAt: string;
}
```

```ts
export interface ProductCandidate {
  id: string;
  processId: string;
  noticeId: string;
  lotId?: string;
  name: string;
  candidateType?:
    | "exact_sku"
    | "standard_commodity"
    | "brand_family"
    | "bundle"
    | "custom_or_fabricated"
    | "installed_goods"
    | "regulated_goods"
    | "unknown";
  matchArchetype?:                                          // NEW: how identity is recovered on marketplaces
    | "standard_class_led"
    | "dimension_material_led"
    | "electrical_spec_led"
    | "pack_count_led"
    | "compatibility_led"
    | "model_exact"
    | "unknown";
  headNoun?: string;
  quantity?: number;
  quantityUnit?: string;
  specificationText?: string;
  estimatedUnitWeightKg?: number;
  estimatedUnitVolumeM3?: number;
  candidateCommodityCodes: string[];
  criticalAttributes?: Record<string, CriticalAttribute>;  // CHANGED: annotated with criticality
  numericSpecs?: Record<string, NumericSpec>;               // CHANGED: annotated with criticality
  standards?: StandardSlot[];                               // CHANGED: structured standard references
  material?: string;
  brandModel?: string;
  preferredTerms?: string[];
  excludedTerms?: string[];
  quoteabilityScore?: number;
  recallRiskScore?: number;                                 // NEW: drives adaptive shortlist budgets
  identityConfidence: number;
  specConfidence: number;
  quantityConfidence: number;
}

// NEW: slot criticality annotation
export interface CriticalAttribute {
  value: string | number | boolean;
  criticality: "required" | "preferred" | "descriptive" | "exclusion";
}

export interface NumericSpec {
  value: string | number;
  unit?: string;
  toleranceWindow?: { min?: number; max?: number };         // NEW: acceptable range for matching
  criticality: "required" | "preferred" | "descriptive";
}

// NEW: structured standard reference for normalization
export interface StandardSlot {
  raw: string;                       // original text from tender (e.g., "EN 397:2012+A1:2012")
  family: string;                    // standard family (e.g., "EN 397")
  part?: string;                     // part number if applicable
  editionYear?: string;              // edition year (e.g., "2012")
  amendment?: string;                // amendment (e.g., "A1:2012")
  classLevel?: string;               // class or level (e.g., "Class 2")
  criticality: "required" | "preferred" | "descriptive";
}
```

<!-- COORDINATION NOTE (landed-cost-engine review, 2026-03-12):
     SupplierQuote and LandedCostEstimate fields below were expanded by landed-cost
     domain review. Data-architecture domain should verify SQL table alignment and
     sync-projection design for these additions. -->

<!-- CROSS-DOMAIN NOTE (landed-cost → procurement-pipeline, 2026-03-12):
     The freight mode decision tree (§4.4) depends on ProductCandidate fields that the
     procurement-pipeline domain produces. Specifically, mode selection needs:
     - estimatedUnitWeightKg and estimatedUnitVolumeM3 (already on ProductCandidate)
     - candidateType (from §5.4 data model notes — e.g. bundle triggers decomposition)
     - Packaging state inference (flat-pack vs assembled) — currently derived heuristically
       from candidateType + headNoun, no explicit field
     - Hazmat/oversized/fragile/reefer/battery classification — no explicit field yet
     The landed-cost engine treats missing weight/volume as "unknown" and runs dual
     packing scenarios. If the procurement-pipeline domain adds explicit packingHint
     or handlingRestrictions fields to ProductCandidate, freight mode selection can
     use them directly instead of inferring from text. -->

```ts
export interface ProcessRelation {
  id: string;
  processIdA: string;
  processIdB: string;
  relationType: "merged_same_process" | "related_not_merged" | "possible_duplicate" | "separate";
  confidence: number;
  evidence?: string[];
  createdAt: string;
}
```

```ts
export interface SearchProfile {
  id: string;
  productCandidateId: string;
  headNoun?: string;
  headNounFamily?: string;                                  // NEW: broader noun family for fallback queries
  candidateType?: ProductCandidate["candidateType"];
  matchArchetype?: ProductCandidate["matchArchetype"];      // NEW: drives query templates and scoring
  criticalAttributes?: Record<string, CriticalAttribute>;   // CHANGED: with criticality
  standards?: StandardSlot[];                                // CHANGED: structured standards
  numericSpecs?: Record<string, NumericSpec>;                // CHANGED: with criticality + tolerance
  nounAliasesEn?: string[];                                 // NEW: English marketplace noun aliases
  nounAliasesZh?: string[];                                 // NEW: Chinese marketplace noun aliases
  marketFrameEn?: string[];                                 // NEW: English market framing terms
  marketFrameZh?: string[];                                 // NEW: Chinese market framing terms
  useContext?: string;                                       // NEW: end-use context (medical, janitorial, etc.)
  sectorContext?: string;                                    // NEW: industry sector
  requiredAnchors?: string[];                                // NEW: must-have search anchors
  optionalAnchors?: string[];                                // NEW: nice-to-have anchors
  mustNotHave?: string[];                                    // NEW: explicit exclusion terms
  preferredTerms?: string[];
  excludedTerms?: string[];
  queryWaves?: QueryWave[];                                  // NEW: replaces flat queryPack
  recallRiskScore?: number;                                  // NEW: drives adaptive shortlist budget
  createdAt: string;
}

// NEW: structured query wave for staged search
export interface QueryWave {
  waveNumber: number;                // 0=exact anchor, 1=alias swap, 2=market framing, 3=broad, 4=compliance-off
  source: "ebay" | "amazon" | "alibaba" | "1688";
  locale: "en" | "zh";
  queryText: string;
  filters?: Record<string, string>; // source-specific structured filters (category, MOQ range, etc.)
  isRecoveryWave: boolean;          // true if generated by zero-result recovery
}
```

```ts
export interface SupplierQuote {
  id: string;
  processId: string;
  noticeId?: string;
  productCandidateId: string;
  searchAttemptId?: string;                                  // NEW: trace to SearchAttempt that produced this quote
  productName?: string;            // denormalized convenience copy for display/debugging
  region: "uk" | "eu" | "china";
  quoteClass: "indicative_marketplace" | "distributor_listing" | "rfq_quote" | "negotiated_quote";
  sourceName: string;
  sourceUrl?: string;
  sourceListingId?: string;                                  // NEW: marketplace-specific listing identifier for cache
  shipFromCountry?: string;
  countryOfOrigin?: string;           // actual origin for duty/preference, not dispatch location
  countryOfDispatch?: string;         // where goods ship from (may differ from origin)
  incoterm?: "EXW" | "FOB" | "CIF" | "DAP" | "DDP" | string;
  quoteCostBoundary?: "EXW" | "FOB" | "CIF" | "CIP" | "DAP" | "DDP" | "UNKNOWN"; // normalized cost boundary
  quotedQuantity?: number;
  quotedQuantityUnit?: string;
  unitPriceOriginal: number;
  currency: string;
  unitPriceGbp: number;
  minimumOrderQuantity?: number;
  leadTimeDays?: number;
  includesShipping?: boolean;         // legacy hint only; quoteCostBoundary is authoritative
  priceSignalRole?: "benchmark_ceiling" | "optimistic_floor" | "executable";
  listingPackQty?: number;
  listingPackUnit?: string;
  normalizedUnitBasis?: string;
  isUsedOrRefurbished?: boolean;
  isReadyToShip?: boolean;                                   // NEW: Alibaba Ready-to-Ship flag (usability bump)
  vatInclusive?: boolean;
  complianceEvidenceTier?: "structured_attribute" | "title_body_mention" | "absent"; // NEW: strength of compliance evidence
  rawMatchScore?: number;                                    // NEW: pre-calibration deterministic score
  matchConfidence: number;
  quoteUsabilityConfidence: number;
  shippingBasisConfidence?: number;
  quoteRealismConfidence: number;     // how realistic is this price for procurement volumes?
  negotiationUpliftPct?: number;      // expected uplift from indicative to negotiated price
  evidenceStates?: Record<string, "match" | "missing" | "ambiguous" | "conflict">; // NEW: per-slot evidence states
  queryWaveNumber?: number;                                  // NEW: which wave produced this match
  evidence: string[];
  validUntil?: string;
}
```

```ts
// NEW: tracks each search execution for debugging, recall measurement, and failure diagnosis
export interface SearchAttempt {
  id: string;
  productCandidateId: string;
  searchProfileId: string;
  source: "ebay" | "amazon" | "alibaba" | "1688";
  locale: "en" | "zh";
  waveNumber: number;
  queryText: string;
  filters?: Record<string, string>;
  isRecoveryWave: boolean;
  resultCount: number;
  hardRejectedCount: number;
  shortlistedCount: number;
  searchFailureMode?: "vocabulary_gap" | "overconstrained_query" | "accessory_swamp"
    | "compliance_not_surfaceable" | "no_marketplace_presence"
    | "low_extraction_quality" | "source_specific_failure";
  executedAt: string;
  sessionId?: string;                // link to agent session for tracing
}
```

```ts
// NEW: cached listing evidence for re-scoring across candidates without re-fetching
export interface ListingEvidence {
  id: string;
  source: "ebay" | "amazon" | "alibaba" | "1688";
  sourceListingId: string;           // marketplace-specific listing identifier
  normalizedTitle: string;
  structuredAttributes?: Record<string, string>;  // parsed spec table / product attributes
  parsedStandards?: StandardSlot[];
  parsedUnits?: { packQty?: number; packUnit?: string; normalizedBasis?: string };
  sellerIdentifier?: string;
  minimumOrderQuantity?: number;
  shippingTerms?: string;
  condition?: "new" | "used" | "refurbished" | "unknown";
  complianceEvidenceTier: "structured_attribute" | "title_body_mention" | "absent";
  intendedUseLanguage?: string;      // detected use-context from listing
  isReadyToShip?: boolean;
  fetchedAt: string;                 // for freshness decay
  detailPageFetched: boolean;        // whether detail-page enrichment was performed
}
```

```ts
export interface LandedCostEstimate {
  id: string;
  processId: string;
  noticeId?: string;
  productCandidateId?: string;
  supplierQuoteId?: string;                                            // NEW: trace to source quote
  shippingMode: "parcel" | "air_freight" | "lcl_ocean" | "fcl_ocean" | "mixed";
  modeConfidence: number;                                              // NEW: confidence in mode selection
  packingScenario?: "compact" | "bulky" | "unknown";                   // NEW: packing assumption used
  declarableCommodityCode10?: string;                                  // RENAMED: 10-digit declarable code
  candidateCommodityCodes?: CandidateCommodityCode[];                  // NEW: full candidate set
  additionalCodes?: string[];                                          // NEW: 4-char additional codes
  commodityConfidence: number;
  classificationStability: number;                                     // NEW: do candidates converge?
  customsValuationMethod?: "1" | "2" | "3" | "4" | "5" | "6" | "unknown"; // NEW
  customsFxRate: number;                                               // RENAMED: HMRC rate for duty
  commercialFxRate: number;                                            // NEW: Xe rate for margin
  fxBasisDeltaPct: number;                                             // NEW: customs vs commercial gap
  supplierCostGbp: number;
  freightLowGbp: number;                                               // NEW: low scenario
  freightBaseGbp: number;                                              // RENAMED from freightCostGbp
  freightHighGbp: number;                                              // NEW: high scenario
  customsDutyGbp: number;
  tradeRemedyDutyGbp?: number;                                         // NEW: AD/CVD/safeguard
  importVatGbp: number;
  importVatTreatment: "postponed_full" | "postponed_partial"           // NEW: replaces boolean
    | "cash_paid_full" | "cash_paid_partial" | "non_recoverable" | "unknown";
  vatRecoveryRate?: number;                                            // NEW: 0.0-1.0
  ukFinalMileCostGbp: number;
  otherFeesGbp: number;
  economicLandedCostGbp: number;
  cashOutlayGbp: number;
  cashToClearGbp: number;                                              // NEW: total upfront cash
  workingCapitalCostGbp?: number;                                      // NEW: optional
  revenueBasisType: "firm_contract" | "estimated_contract"             // NEW
    | "framework_ceiling" | "dynamic_market" | "concession" | "unknown";
  revenueConfidence: number;                                           // NEW
  netMarginLowPct: number;                                             // NEW: conservative
  netMarginBasePct: number;                                            // NEW: base
  netMarginHighPct: number;                                            // NEW: optimistic
  decisionGrade: "screening" | "review" | "rfq_required" | "broker_required"; // NEW
  confidence: number;
  confidenceBreakdown?: ConfidenceBreakdown;                           // NEW
  warnings: string[];
  blockers: string[];                                                  // NEW: critical blockers
}

export interface CandidateCommodityCode {
  code: string;
  digits: 6 | 8 | 10;
  source: "fpo" | "taric_support" | "hs_ping" | "manual" | "atr";
  score: number;
  description?: string;
}

export interface ConfidenceBreakdown {
  classificationStability: number;
  quoteRealism: number;
  commodityCodeConfidence: number;
  freightModeStability: number;
  revenueReliability: number;
  fxFreshness: number;
}
```

```ts
// Pipeline-computed assessment (daemon-owned, push-only)
export interface PipelineAssessment {
  processId: string;
  pipelineState:
    | "discovered"
    | "normalized"
    | "relation_resolved"
    | "prelim_rejected"
    | "prelim_qualified"
    | "docs_acquired"
    | "lots_extracted"
    | "products_extracted"
    | "quoteability_scored"
    | "needs_review"
    | "priced"
    | "gross_qualified"
    | "landed_costed"
    | "net_qualified"
    | "expired";
  grossMarginBps?: number;   // basis points (e.g., 1500 = 15.00%)
  netMarginBps?: number;     // basis points
  reviewStage?: "qualification" | "quoteability" | "pricing" | "landed_cost" | "manifest";
  reviewReason?: string;
  assessedAt: string;
}
```

```ts
// Human operator disposition (remote-owned, pull-only, compare-and-swap)
export interface OperatorDecisionHead {
  processId: string;
  disposition:
    | "pending"         // no human decision yet
    | "watching"
    | "approved"
    | "rejected";
  decisionNote?: string;
  decidedBy?: string;
  decidedAt?: string;
  remoteRevision: number;  // for compare-and-swap
}
```

```ts
// Bid workflow state (daemon-owned, push-only)
export interface BidWorkflowState {
  processId: string;
  bidState:
    | "not_started"
    | "ready_to_bid"
    | "bid_in_progress"
    | "bid_submitted"
    | "bid_failed";
  bidJobId?: string;
  startedAt?: string;
  completedAt?: string;
}
```

```ts
// Job orchestration row (local-only, mirrored to summaries for remote visibility)
export interface WorkflowJob {
  id: string;
  kind:
    | "procurement_ingestion"
    | "qualification"
    | "enrichment"
    | "landed_cost"
    | "bidding"
    | "validation";
  processId?: string;
  noticeId?: string;
  parentJobId?: string;
  validationRequired: boolean;
  validationVerdictId?: string;
  status: string;
  scheduledFor?: string;
  attemptCount: number;
  inputPayload: Record<string, unknown>;
  outputPayload?: Record<string, unknown>;
  lastError?: string;
  startedAt?: string;
  completedAt?: string;
  createdAt: string;
}
```

```ts
// Full local session log row (local-only; remote gets summaries/redactions)
export interface AgentSessionLog {
  id: string;
  jobId?: string;
  skillPath: string;
  skillVersion?: string;
  executionMode: "pty" | "contract";
  cliVendor?: string;
  cliVersion?: string;
  adapterVersion?: string;
  configHash?: string;
  ptySessionId: string;
  pid?: number;
  pgid?: number;
  startedAt: string;
  completedAt?: string;
  status: "created" | "spawned" | "active" | "terminating" | "exited" | "reaped" | "archived";
  terminalOutputPath?: string;
  terminalOutputHash?: string;
  terminalOutputBytes?: number;
  terminalOutputTail?: string;
  structuredTrace?: Record<string, unknown>;
  errorType?: string;
  errorMessage?: string;
  cleanupVerdict?: "clean" | "forced" | "leaked";
  lastCheckpointSeq?: number;
  runManifestPath?: string;
  runManifestHash?: string;
}
```

```ts
export interface ValidationIssue {
  severity: "critical" | "warning" | "info";
  category:
    | "wrong_source"
    | "wrong_locale"
    | "data_fabrication"
    | "data_quality"
    | "missing_data"
    | "semantic_error"
    | "suspicious_pattern"
    | "manifest_inconsistency"
    | "coverage_gap"
    | "policy_violation";
  description: string;
  evidence?: string;
  suggestion?: string;
}
```

```ts
// Full validator verdict row (local-only)
export interface ValidationVerdict {
  id: string;
  workerJobId: string;
  workerSessionId?: string;
  validatorJobId: string;
  validatorSessionId?: string;
  validatorSkillPath: string;
  validatorSkillVersion?: string;
  verdict: "approved" | "rejected" | "flagged";
  confidenceBps?: number;
  summary: string;
  issues: ValidationIssue[];
  qualityAssessment?: {
    manifestCompleteness?: "complete" | "partial" | "minimal" | "missing";
    narrativeConsistency?: "consistent" | "minor_inconsistencies" | "contradictory";
    evidencePresent?: boolean;
    fingerprintVerified?: boolean;
    localeVerified?: boolean;
    resultCountMatchesManifest?: boolean;
  };
  skillImprovementSuggestions?: string[];
  runManifestHash: string;
  oqgSignalsSummary?: string;
  overrideVerdict?: "approved" | "rejected";
  overrideReason?: string;
  overrideBy?: string;
  overrideAt?: string;
  verdictRenderedAt: string;
  createdAt: string;
}
```

```ts
// Remote-facing validation summary projection (push-owned)
export interface ValidationVerdictSummary {
  id: string;
  workerJobId: string;
  verdict: "approved" | "rejected" | "flagged";
  confidenceBps?: number;
  summary?: string;
  issueCount: number;
  criticalIssueCount: number;
  overrideVerdict?: "approved" | "rejected";
  verdictRenderedAt: string;
}
```

```ts
// Stage execution tracking (daemon-owned, push-only)
export interface ProcessStageStatus {
  processId: string;
  stage: "ingestion" | "qualification" | "enrichment" | "landed_cost" | "manifest" | "bidding";
  stageRunId: string;
  state: "running" | "succeeded" | "failed";
  startedAt: string;
  completedAt?: string;
  errorManifestId?: string;
}
```

`ProcessStageStatus.stage` is intentionally **coarser** than `PipelineAssessment.pipelineState`: several business states can occur inside one execution stage. For example, `docs_acquired`, `lots_extracted`, `products_extracted`, and `quoteability_scored` all roll up under the broader execution flow rather than forcing one orchestration stage row per business-state hop.

```ts
export interface ViewerSyncStatus {
  executorId: string;
  daemonLastSeenAt: string;
  lastLocalSeqAcked: number;
  lastSuccessAt?: string;
  syncHealth: "healthy" | "degraded" | "offline";
  schemaEpoch: number;
}
```

#### Suggested SQL Tables

```sql
create table raw_documents (
  id uuid primary key default gen_random_uuid(),
  source_id text not null references sources(id),
  fetched_at timestamptz not null default now(),
  content_type text,
  source_url text,
  payload_json jsonb,
  storage_path text,
  metadata jsonb not null default '{}'::jsonb,
  created_at timestamptz not null default now(),
  check (payload_json is not null or storage_path is not null)
);

-- Local runtime copy of source definitions. Populated from shared_source_configs
-- during pull sync. Exists locally so raw_documents, source_cursors, and
-- contract_notices can have local FK references without network dependency.
-- The authoritative version is shared_source_configs (remote-owned, pull-only).
create table sources (
  id text primary key,
  kind text not null,
  enabled boolean not null default true,
  config jsonb not null,
  created_at timestamptz not null default now()
);

create table source_cursors (
  source_id text not null references sources(id),
  cursor_key text not null,
  cursor_value text,
  updated_at timestamptz not null default now(),
  primary key (source_id, cursor_key)
);

create table source_runtime_state (
  source_id text primary key references sources(id),
  last_health_state text not null default 'green',
  challenge_event_count integer not null default 0,
  challenge_window_started_at timestamptz,
  circuit_opened_at timestamptz,
  cooldown_until timestamptz,
  last_canary_at timestamptz,
  last_error_code text,
  updated_at timestamptz not null default now(),
  created_at timestamptz not null default now()
);

-- Single-daemon MVP: one row representing the local daemon.
-- Multi-executor support is a future-consideration extension.
create table executors (
  id uuid primary key default gen_random_uuid(),
  device_name text not null,
  status text not null,
  last_heartbeat_at timestamptz not null default now(),
  -- sync tracking (push-owned: heartbeat summary pushed to remote)
  owner_executor_id text not null,  -- self-referential for consistency
  row_version integer not null default 1,
  last_local_seq integer,
  updated_at timestamptz not null default now(),
  created_at timestamptz not null default now()
);

create table procurement_processes (
  id uuid primary key default gen_random_uuid(),
  canonical_ocid text,
  primary_notice_id uuid,
  current_stage text not null,
  buyer_name text,
  title text not null,
  latest_deadline_at timestamptz,
  latest_published_at timestamptz not null,
  latest_updated_at timestamptz not null,
  -- sync tracking (push-owned)
  owner_executor_id text not null,
  row_version integer not null default 1,
  last_local_seq integer,
  stage_run_id text,
  updated_at timestamptz not null default now(),
  created_at timestamptz not null default now()
);

create table contract_notices (
  id uuid primary key default gen_random_uuid(),
  process_id uuid references procurement_processes(id),
  source_id text not null references sources(id),
  source_notice_id text not null,
  raw_document_id uuid not null references raw_documents(id),
  ocid text,
  title text not null,
  summary text,
  buyer_name text,
  procurement_stage text not null,
  access_route text,  -- open_tender, restricted_tender, framework_call_off, dynamic_market, direct_award, closed_framework_award, not_bidable
  deadline_at timestamptz,
  published_at timestamptz not null,
  source_updated_at timestamptz not null,
  main_category text,
  -- cpv_codes: use junction table notice_cpv_codes instead of array column
  estimated_value_original_pence integer,  -- integer pence
  estimated_value_currency text,
  estimated_value_gbp_ex_vat_pence integer,  -- integer pence
  estimated_value_vat_inclusive boolean,
  normalized_payload jsonb not null,
  -- sync tracking (push-owned)
  owner_executor_id text not null,
  row_version integer not null default 1,
  last_local_seq integer,
  stage_run_id text,
  updated_at timestamptz not null default now(),
  created_at timestamptz not null default now(),
  unique (source_id, source_notice_id)
);
```

```sql
create table qualification_runs (
  id uuid primary key default gen_random_uuid(),
  process_id uuid not null references procurement_processes(id),
  notice_id uuid not null references contract_notices(id),
  expiry_qualified boolean not null,
  goods_qualified boolean not null,
  goods_confidence_bps integer not null,  -- basis points (0-10000)
  legal_category text,            -- from procurement pipeline review
  goods_density_score_bps integer,  -- basis points (0-10000)
  goods_separability text,        -- true, false, uncertain (tri-state classification stored as text)
  access_route text,              -- open_tender, restricted_tender, etc.
  rationale jsonb not null,       -- renamed from 'evidence' to match TS interface
  qualified_at timestamptz not null default now(),
  -- sync tracking (push-owned)
  owner_executor_id text not null,
  row_version integer not null default 1,
  last_local_seq integer,
  stage_run_id text,
  updated_at timestamptz not null default now(),
  created_at timestamptz not null default now()
);

create table product_candidates (
  id uuid primary key default gen_random_uuid(),
  process_id uuid not null references procurement_processes(id),
  notice_id uuid not null references contract_notices(id),
  lot_id uuid references notice_lots(id),
  name text not null,
  candidate_type text,  -- exact_sku, standard_commodity, brand_family, bundle, custom_or_fabricated, installed_goods, regulated_goods, unknown
  match_archetype text, -- standard_class_led, dimension_material_led, electrical_spec_led, pack_count_led, compatibility_led, model_exact, unknown
  head_noun text,
  quantity numeric,
  quantity_unit text,
  specification_text text,
  estimated_unit_weight_kg numeric,
  estimated_unit_volume_m3 numeric,
  material text,
  brand_model text,
  -- candidate_commodity_codes: use junction table product_commodity_codes instead of array column
  critical_attributes_json jsonb,
  numeric_specs_json jsonb,
  standards_json jsonb,
  preferred_terms_json jsonb,
  excluded_terms_json jsonb,
  quoteability_score_bps integer,   -- basis points (0-10000)
  recall_risk_score_bps integer,    -- basis points (0-10000)
  identity_confidence_bps integer,  -- basis points (0-10000)
  spec_confidence_bps integer,      -- basis points (0-10000)
  quantity_confidence_bps integer,  -- basis points (0-10000)
  -- sync tracking (push-owned)
  owner_executor_id text not null,
  row_version integer not null default 1,
  last_local_seq integer,
  stage_run_id text,
  updated_at timestamptz not null default now(),
  created_at timestamptz not null default now()
);

create table search_attempts (
  id uuid primary key default gen_random_uuid(),
  product_candidate_id uuid not null references product_candidates(id),
  search_profile_id uuid not null references search_profiles(id),
  source text not null,
  locale text not null,
  wave_number integer not null,
  query_text text not null,
  filters_json jsonb,
  is_recovery_wave boolean not null default false,
  result_count integer not null default 0,
  hard_rejected_count integer not null default 0,
  shortlisted_count integer not null default 0,
  search_failure_mode text,
  session_id uuid references agent_session_logs(id),
  executed_at timestamptz not null default now(),
  -- sync tracking (push-owned)
  owner_executor_id text not null,
  row_version integer not null default 1,
  last_local_seq integer,
  stage_run_id text,
  updated_at timestamptz not null default now(),
  created_at timestamptz not null default now()
);

create table supplier_quotes (
  id uuid primary key default gen_random_uuid(),
  process_id uuid not null references procurement_processes(id),
  notice_id uuid not null references contract_notices(id),
  product_candidate_id uuid references product_candidates(id),
  search_attempt_id uuid references search_attempts(id),
  product_name text not null,                 -- denormalized convenience copy for display/debugging
  region text not null,
  quote_class text not null,
  source_name text not null,
  source_url text,
  source_listing_id text,
  ship_from_country text,
  country_of_origin text,
  country_of_dispatch text,
  incoterm text,
  quote_cost_boundary text,
  quoted_quantity numeric,
  quoted_quantity_unit text,
  unit_price_original_pence integer not null,  -- integer pence
  currency text not null,
  unit_price_gbp_pence integer not null,       -- integer pence
  minimum_order_quantity numeric,
  lead_time_days integer,
  includes_shipping boolean not null default false,
  -- cross-domain fields (procurement pipeline review)
  price_signal_role text,           -- benchmark_ceiling, optimistic_floor, executable
  listing_pack_qty numeric,
  listing_pack_unit text,
  normalized_unit_basis text,
  is_used_or_refurbished boolean not null default false,
  is_ready_to_ship boolean,
  vat_inclusive boolean,
  compliance_evidence_tier text,
  raw_match_score_bps integer,
  match_confidence_bps integer,     -- basis points (0-10000)
  quote_usability_confidence_bps integer,  -- basis points (0-10000)
  shipping_basis_confidence_bps integer,   -- basis points (0-10000)
  -- cross-domain fields (data architecture review)
  quote_realism_confidence_bps integer,  -- basis points (0-10000)
  negotiation_uplift_bps integer,        -- basis points
  evidence_states_json jsonb,
  query_wave_number integer,
  evidence jsonb not null,
  valid_until timestamptz,
  -- sync tracking (push-owned)
  owner_executor_id text not null,
  row_version integer not null default 1,
  last_local_seq integer,
  stage_run_id text,
  updated_at timestamptz not null default now(),
  created_at timestamptz not null default now()
);
```


```sql
create table landed_cost_estimates (
  id uuid primary key default gen_random_uuid(),
  process_id uuid not null references procurement_processes(id),
  notice_id uuid references contract_notices(id),
  product_candidate_id uuid references product_candidates(id),
  supplier_quote_id uuid references supplier_quotes(id),
  shipping_mode text not null,
  mode_confidence_bps integer not null,            -- basis points (0-10000)
  packing_scenario text,
  declarable_commodity_code_10 text,
  additional_codes jsonb,
  commodity_confidence_bps integer not null,        -- basis points (0-10000)
  classification_stability_bps integer not null,   -- basis points (0-10000)
  customs_valuation_method text,
  customs_fx_rate numeric not null,                -- exchange rate (not money — keep numeric)
  commercial_fx_rate numeric not null,             -- exchange rate (not money — keep numeric)
  fx_basis_delta_bps integer not null,             -- basis points
  supplier_cost_gbp_pence integer not null,        -- integer pence
  freight_low_gbp_pence integer not null,          -- integer pence
  freight_base_gbp_pence integer not null,         -- integer pence
  freight_high_gbp_pence integer not null,         -- integer pence
  customs_duty_gbp_pence integer not null,         -- integer pence
  trade_remedy_duty_gbp_pence integer,             -- integer pence
  import_vat_gbp_pence integer not null,           -- integer pence
  import_vat_treatment text not null,              -- postponed_full, postponed_partial, cash_paid_full, cash_paid_partial, non_recoverable, unknown
  vat_recovery_rate_bps integer,                   -- basis points
  uk_final_mile_cost_gbp_pence integer not null,   -- integer pence
  other_fees_gbp_pence integer not null,           -- integer pence
  economic_landed_cost_gbp_pence integer not null,  -- integer pence
  cash_outlay_gbp_pence integer not null,          -- integer pence
  cash_to_clear_gbp_pence integer not null,        -- integer pence
  working_capital_cost_gbp_pence integer,          -- integer pence
  revenue_basis_type text not null,
  revenue_confidence_bps integer not null,          -- basis points (0-10000)
  net_margin_low_bps integer not null,              -- basis points
  net_margin_base_bps integer not null,             -- basis points
  net_margin_high_bps integer not null,             -- basis points
  decision_grade text not null,
  confidence_bps integer not null,                  -- basis points (0-10000)
  confidence_breakdown jsonb,
  warnings jsonb not null,
  blockers jsonb not null default '[]',
  -- sync tracking (push-owned)
  owner_executor_id text not null,
  row_version integer not null default 1,
  last_local_seq integer,
  stage_run_id text,
  updated_at timestamptz not null default now(),
  created_at timestamptz not null default now()
);
```

```sql
create table manifests (
  id uuid primary key default gen_random_uuid(),
  process_id uuid not null references procurement_processes(id),
  markdown text not null,
  artifact_path text,
  -- sync tracking (push-owned)
  owner_executor_id text not null,
  row_version integer not null default 1,
  last_local_seq integer,
  stage_run_id text,
  updated_at timestamptz not null default now(),
  created_at timestamptz not null default now()
);

-- Pipeline-computed assessment (daemon-owned, push-only)
create table pipeline_assessments (
  process_id uuid primary key references procurement_processes(id),
  pipeline_state text not null,  -- discovered, normalized, relation_resolved, prelim_rejected, prelim_qualified, docs_acquired, lots_extracted, products_extracted, quoteability_scored, needs_review, priced, gross_qualified, landed_costed, net_qualified, expired
  gross_margin_bps integer,      -- basis points (1500 = 15.00%)
  net_margin_bps integer,        -- basis points
  review_stage text,
  review_reason text,
  assessed_at timestamptz not null default now(),
  -- sync tracking (push-owned)
  owner_executor_id text not null,
  row_version integer not null default 1,
  last_local_seq integer,
  stage_run_id text,
  updated_at timestamptz not null default now(),
  created_at timestamptz not null default now()
);

-- Human operator disposition (remote-owned, pull-only, compare-and-swap)
create table operator_decision_heads (
  process_id uuid primary key references procurement_processes(id),
  disposition text not null default 'pending',  -- pending, watching, approved, rejected
  decision_note text,
  decided_by uuid,
  decided_at timestamptz,
  -- remote-owned tracking
  remote_change_seq bigint,
  remote_revision bigint not null default 1,
  deleted_at timestamptz,  -- tombstone for shared delete semantics
  created_at timestamptz not null default now()
);

-- Bid workflow state (daemon-owned, push-only)
create table bid_workflow_states (
  process_id uuid primary key references procurement_processes(id),
  bid_state text not null default 'not_started',  -- not_started, ready_to_bid, bid_in_progress, bid_submitted, bid_failed
  bid_job_id uuid references workflow_jobs(id),
  started_at timestamptz,
  completed_at timestamptz,
  -- sync tracking (push-owned)
  owner_executor_id text not null,
  row_version integer not null default 1,
  last_local_seq integer,
  updated_at timestamptz not null default now(),
  created_at timestamptz not null default now()
);

create table workflow_jobs (
  id uuid primary key default gen_random_uuid(),
  kind text not null,            -- procurement_ingestion, qualification, enrichment, landed_cost, bidding, validation
  process_id uuid references procurement_processes(id),
  notice_id uuid references contract_notices(id),
  parent_job_id uuid references workflow_jobs(id),  -- links validator sub-job to worker job
  validation_required boolean not null default false,  -- set by Validation Policy Engine
  validation_verdict_id uuid,    -- populated after validation completes (references validation_verdicts.id)
  status text not null,
  scheduled_for timestamptz,
  lease_owner uuid references executors(id), -- authoritative in local SQLite; mirrored to Supabase for visibility only
  lease_expires_at timestamptz, -- authoritative in local SQLite; mirrored to Supabase for visibility only
  attempt_count integer not null default 0,
  input_payload jsonb not null,
  output_payload jsonb,
  last_error text,
  started_at timestamptz,
  completed_at timestamptz,
  created_at timestamptz not null default now()
);
```

```sql
create table agent_session_logs (
  id uuid primary key default gen_random_uuid(),
  job_id uuid references workflow_jobs(id),
  skill_path text not null,
  skill_version text,
  -- execution identity (from agent orchestration review)
  execution_mode text not null default 'pty',  -- 'pty' or 'contract'
  cli_vendor text,
  cli_version text,
  adapter_version text,
  config_hash text,
  -- process supervision (from agent orchestration review)
  pty_session_id text not null,
  pid integer,
  pgid integer,
  started_at timestamptz not null,
  completed_at timestamptz,
  status text not null,  -- created, spawned, active, terminating, exited, reaped, archived
  -- raw session output (artifact-based; legacy column names retained)
  terminal_output_path text,   -- artifact file path for PTY bytes or contract-mode stdout/stderr (local only; not synced)
  terminal_output_hash text,   -- SHA-256 of raw session output
  terminal_output_bytes integer,
  terminal_output_tail text,   -- last N KB of raw session output for quick inspection (local only; Supabase gets redacted excerpt or null)
  structured_trace jsonb,      -- full parsed trace locally; Supabase stores only redacted summaries
  -- error handling (expanded taxonomy from agent orchestration review)
  error_type text,  -- AGENT_AUTH_CHALLENGE, AGENT_INTERNAL_ERROR, AGENT_STALLED, AGENT_SKILL_FAILED,
                    -- AGENT_OUTPUT_CONTRACT_INVALID, AGENT_PROTOCOL_MISMATCH,
                    -- AGENT_PARTIAL_FAILURE, AGENT_CLEANUP_INCOMPLETE, AGENT_RUN_FAILED,
                    -- HOST_SLEEP_INTERRUPTED, HOST_RESOURCE_EXHAUSTED
  error_message text,
  cleanup_verdict text,        -- clean, forced, leaked
  last_checkpoint_seq integer, -- last valid checkpoint sequence before failure
  -- run manifest (worker-validator system)
  run_manifest_path text,      -- artifact path to the agent-generated run manifest MD file
  run_manifest_hash text,      -- SHA-256 of the run manifest for integrity verification
  created_at timestamptz not null default now()
);

create table error_manifests (
  id uuid primary key default gen_random_uuid(),
  job_id uuid references workflow_jobs(id),
  process_id uuid references procurement_processes(id),
  pipeline_stage text not null, -- 'ingestion', 'qualification', 'enrichment', 'landed_cost', 'bidding'
  workflow_step text not null, -- specific step within the stage
  error_type text not null,
  error_message text not null,
  error_context jsonb not null, -- full context locally; redacted context in Supabase
  skill_path text,
  agent_session_id uuid references agent_session_logs(id),
  markdown_path text, -- local Markdown artifact path for human-friendly debugging
  resolved boolean not null default false,
  resolved_at timestamptz,
  resolved_by text,
  created_at timestamptz not null default now()
);
```

```sql
-- Semantic validation verdicts from validator skills (local-only — full verdict reasoning stays local).
create table validation_verdicts (
  id uuid primary key default gen_random_uuid(),
  worker_job_id uuid not null references workflow_jobs(id),
  worker_session_id uuid references agent_session_logs(id),
  validator_job_id uuid not null references workflow_jobs(id),
  validator_session_id uuid references agent_session_logs(id),
  validator_skill_path text not null,
  validator_skill_version text,
  verdict text not null,                        -- approved, rejected, flagged
  confidence_bps integer,                       -- basis points (0-10000)
  summary text not null,                        -- one-paragraph human-readable summary (max 2000 chars)
  issues_json jsonb not null default '[]',      -- JSON array of ValidationIssue objects
  quality_assessment_json jsonb,                -- JSON quality scores (manifest completeness, narrative consistency, etc.)
  skill_improvement_suggestions_json jsonb,     -- JSON array of improvement suggestions for the worker skill
  run_manifest_hash text not null,              -- SHA-256 of the run manifest that was validated
  oqg_signals_summary text,                     -- OQG inputs provided to validator
  override_verdict text,                        -- operator override (if any): approved, rejected
  override_reason text,
  override_by text,
  override_at timestamptz,
  verdict_rendered_at timestamptz not null,
  created_at timestamptz not null default now()
);
```

```sql
-- Validation verdict summaries pushed to Supabase for web visibility (push-owned projection).
create table validation_verdict_summaries (
  id uuid primary key,                          -- references validation_verdicts(id)
  worker_job_id uuid not null,
  verdict text not null,                        -- approved, rejected, flagged
  confidence_bps integer,
  summary text,
  issue_count integer not null default 0,
  critical_issue_count integer not null default 0,
  override_verdict text,
  verdict_rendered_at timestamptz not null,
  -- sync tracking (same pattern as other push-owned projections)
  owner_executor_id text not null,
  row_version integer not null default 1,
  last_local_seq integer,
  updated_at timestamptz not null default now(),
  created_at timestamptz not null default now()
);
```

```sql
-- Remote-facing projection summaries for web visibility.
create table workflow_job_summaries (
  job_id uuid primary key references workflow_jobs(id),
  process_id uuid references procurement_processes(id),
  kind text not null,
  status text not null,
  scheduled_for timestamptz,
  started_at timestamptz,
  completed_at timestamptz,
  last_error text,
  owner_executor_id text not null,
  row_version integer not null default 1,
  last_local_seq integer,
  updated_at timestamptz not null default now(),
  created_at timestamptz not null default now()
);

create table error_manifest_summaries (
  id uuid primary key references error_manifests(id),
  process_id uuid references procurement_processes(id),
  pipeline_stage text not null,
  workflow_step text not null,
  error_type text not null,
  error_message text not null,
  redacted_context jsonb,
  resolved boolean not null default false,
  resolved_at timestamptz,
  owner_executor_id text not null,
  row_version integer not null default 1,
  last_local_seq integer,
  updated_at timestamptz not null default now(),
  created_at timestamptz not null default now()
);

create table viewer_sync_status (
  executor_id uuid primary key references executors(id),
  daemon_last_seen_at timestamptz not null,
  last_local_seq_acked integer not null default 0,
  last_success_at timestamptz,
  sync_health text not null,
  schema_epoch integer not null,
  owner_executor_id text not null,
  row_version integer not null default 1,
  last_local_seq integer,
  updated_at timestamptz not null default now(),
  created_at timestamptz not null default now()
);
```

#### Stage Tracking Tables

```sql
create table process_stage_status (
  id uuid primary key default gen_random_uuid(),
  process_id uuid not null references procurement_processes(id),
  stage text not null,
  stage_run_id text not null,
  state text not null,
  started_at timestamptz not null,
  completed_at timestamptz,
  error_manifest_id uuid references error_manifests(id),
  input_revision integer,
  output_revision integer,
  owner_executor_id text not null,
  row_version integer not null default 1,
  last_local_seq integer,
  updated_at timestamptz not null default now(),
  created_at timestamptz not null default now()
);

create table process_stage_heads (
  process_id uuid not null references procurement_processes(id),
  stage text not null,
  current_stage_run_id text not null,
  owner_executor_id text not null,
  row_version integer not null default 1,
  last_local_seq integer,
  updated_at timestamptz not null default now(),
  primary key (process_id, stage)
);
```

#### Sync Infrastructure Tables

```sql
-- SQLite-only: local outbox for transactional sync
create table local_outbox (
  local_seq integer primary key autoincrement,
  executor_id text not null,
  entity_kind text not null,
  entity_id text not null,
  op text not null,
  row_version integer not null,
  stage_run_id text,
  schema_epoch integer not null,
  payload_json text not null,
  content_hash text not null,
  created_at text not null
);

-- SQLite-only: tracks push batches
create table sync_batches (
  batch_id text primary key,
  executor_id text not null,
  from_local_seq integer not null,
  to_local_seq integer not null,
  schema_epoch integer not null,
  sent_at text not null,
  ack_at text,
  status text not null default 'pending'
);

-- SQLite-only: pull cursors per remote domain
create table pull_cursors (
  domain text primary key,
  last_pulled_seq bigint not null default 0,
  pulled_at text not null
);

-- SQLite-only: queued remote-authoritative mutations while sync is degraded
create table pending_operator_intents (
  id text primary key,
  process_id text not null,
  desired_disposition text not null,
  base_remote_revision bigint not null,
  idempotency_key text not null unique,
  status text not null default 'queued',
  queued_at text not null,
  last_attempt_at text,
  last_error text
);

-- Supabase-side: schema version tracking
create table schema_metadata (
  key text primary key,
  value text not null,
  updated_at timestamptz not null default now()
);
```

#### Remote-Owned Tables

```sql
create table shared_company_profile (
  id uuid primary key default gen_random_uuid(),
  profile_data jsonb not null,
  remote_change_seq bigint,
  remote_revision bigint not null default 1,
  deleted_at timestamptz,
  updated_at timestamptz not null default now(),
  created_at timestamptz not null default now()
);

create table shared_source_configs (
  source_id text primary key,
  kind text not null,
  enabled boolean not null default true,
  config jsonb not null,
  remote_change_seq bigint,
  remote_revision bigint not null default 1,
  deleted_at timestamptz,
  updated_at timestamptz not null default now(),
  created_at timestamptz not null default now()
);
```

#### Junction Tables

```sql
create table notice_cpv_codes (
  notice_id uuid not null references contract_notices(id),
  cpv_code text not null,
  primary key (notice_id, cpv_code)
);

create table product_commodity_codes (
  product_candidate_id uuid not null references product_candidates(id),
  commodity_code text not null,
  primary key (product_candidate_id, commodity_code)
);
```

#### Cross-Domain Entity Tables

```sql
-- Lot-level decomposition of notices (from procurement pipeline review).
-- If no explicit lots, a synthetic whole-notice lot is created.
create table notice_lots (
  id uuid primary key default gen_random_uuid(),
  notice_id uuid not null references contract_notices(id),
  source_lot_id text,
  lot_number integer,
  title text,
  description text,
  estimated_value_pence integer,  -- integer pence
  deadline_at timestamptz,
  is_bidable_separately boolean not null default true,
  documents jsonb,  -- opaque array of document references
  -- sync tracking (push-owned)
  owner_executor_id text not null,
  row_version integer not null default 1,
  last_local_seq integer,
  stage_run_id text,
  updated_at timestamptz not null default now(),
  created_at timestamptz not null default now()
);

-- Junction: lot-level CPV codes
create table lot_cpv_codes (
  lot_id uuid not null references notice_lots(id),
  cpv_code text not null,
  primary key (lot_id, cpv_code)
);

-- Edges between related-but-unmerged processes (from procurement pipeline review).
-- Enables cache sharing across related clusters without forced merges.
create table process_relations (
  id uuid primary key default gen_random_uuid(),
  process_id_a uuid not null references procurement_processes(id),
  process_id_b uuid not null references procurement_processes(id),
  relation_type text not null,  -- merged_same_process, related_not_merged, possible_duplicate, separate
  confidence_bps integer not null,  -- basis points (0-10000)
  evidence jsonb,
  -- sync tracking (push-owned)
  owner_executor_id text not null,
  row_version integer not null default 1,
  last_local_seq integer,
  stage_run_id text,
  updated_at timestamptz not null default now(),
  created_at timestamptz not null default now()
);

create table process_aliases (
  alias_process_id uuid primary key,
  canonical_process_id uuid not null references procurement_processes(id),
  alias_reason text not null default 'merged_same_process',
  owner_executor_id text not null,
  row_version integer not null default 1,
  last_local_seq integer,
  updated_at timestamptz not null default now(),
  created_at timestamptz not null default now()
);

-- Structured search profile per product candidate (from procurement pipeline review).
-- Persisted for debugging and skill improvement.
create table search_profiles (
  id uuid primary key default gen_random_uuid(),
  product_candidate_id uuid not null references product_candidates(id),
  head_noun text,
  head_noun_family text,
  candidate_type text,
  match_archetype text,
  critical_attributes_json jsonb,
  standards_json jsonb,
  numeric_specs_json jsonb,
  noun_aliases_en jsonb,
  noun_aliases_zh jsonb,
  market_frame_en jsonb,
  market_frame_zh jsonb,
  use_context text,
  sector_context text,
  required_anchors jsonb,
  optional_anchors jsonb,
  must_not_have jsonb,
  preferred_terms jsonb,
  excluded_terms jsonb,
  query_waves_json jsonb,
  recall_risk_score_bps integer,
  -- sync tracking (push-owned)
  owner_executor_id text not null,
  row_version integer not null default 1,
  last_local_seq integer,
  stage_run_id text,
  updated_at timestamptz not null default now(),
  created_at timestamptz not null default now()
);

create table listing_evidence (
  id uuid primary key default gen_random_uuid(),
  source text not null,
  source_listing_id text not null,
  normalized_title text not null,
  structured_attributes_json jsonb,
  parsed_standards_json jsonb,
  parsed_units_json jsonb,
  seller_identifier text,
  minimum_order_quantity numeric,
  shipping_terms text,
  condition text,
  compliance_evidence_tier text not null,
  intended_use_language text,
  is_ready_to_ship boolean,
  fetched_at timestamptz not null,
  detail_page_fetched boolean not null default false,
  created_at timestamptz not null default now(),
  unique (source, source_listing_id)
);
```

### 5.5 Contract State Model (Split Authority)

The original single state machine has been decomposed into three separate authority domains to eliminate mixed-write conflicts:

**Pipeline State** (daemon-owned, in `pipeline_assessments`):
```text
discovered
  -> normalized
  -> relation_resolved
  -> prelim_rejected
  -> prelim_qualified
  -> docs_acquired
  -> lots_extracted
  -> products_extracted
  -> quoteability_scored
  -> needs_review
  -> priced
  -> gross_qualified
  -> landed_costed
  -> net_qualified
  -> expired
```

**Operator Disposition** (remote-owned, in `operator_decision_heads`):
```text
pending
  -> watching
  -> approved
  -> rejected
```

**Bid Workflow State** (daemon-owned, in `bid_workflow_states`):
```text
not_started
  -> ready_to_bid
  -> bid_in_progress
  -> bid_submitted
  -> bid_failed
```

**State transition notes:**

- `relation_resolved` is reached after the relation resolution engine classifies the notice's relationship to existing processes (merge / related_not_merged / possible_duplicate / separate).
- `prelim_rejected` is reached when any qualification dimension fails: expiry within 30 days, non-bidable access route, or pure services/works with negligible goods density. The `QualificationResult` records which dimension(s) caused rejection.
- `prelim_qualified` requires passing all four qualification dimensions: expiry, access route, goods classification, and lot-level goods separability. Processes routed to `needs_review` from qualification (e.g., mixed contracts with uncertain separability) remain at `prelim_qualified` with a review flag until the operator resolves them.
- `docs_acquired` → `lots_extracted` → `products_extracted` → `quoteability_scored` form the document-to-candidate pipeline.
- **Below-quoteability path:** Candidates with quoteabilityScore < 0.45 advance to `quoteability_scored` in pipeline state but do NOT enter `priced`. They remain at `quoteability_scored` and the operator disposition defaults to `pending` for operator triage. The operator may then set disposition to `watching`, `rejected`, or escalate to RFQ manually.
- **RFQ escalation** is an operator disposition action (setting disposition to `approved` with a sub-action or note), not a separate pipeline state. The pipeline continues to `priced` if/when executable quotes are obtained.
- `needs_review` is a pipeline state reachable from multiple stages: qualification ambiguity, gross margin below threshold with high uncertainty, or landed-cost blocker (see §5.7 Landed Cost step 15). The `pipeline_assessments` record should include `review_reason` and `review_stage` to distinguish these.
- `expired` is set by a background daemon job that periodically scans active processes and transitions those whose `latestDeadlineAt` has passed. Expiry check runs on daemon startup and on a configurable interval (default: every 6 hours).

### 5.6 Ingestion Data Flow

#### Data Flow: Scheduled Ingestion (Skill-Based Agent Scraping)

1. Daemon's local cron scheduler determines that an ingestion job is due for a source.
2. Scheduler reads the enabled source definition from the Source Registry and enqueues a job with explicit skill id/version binding. If `source_runtime_state` shows an open circuit or active cooldown, the scheduler suppresses normal ingestion and only allows `source-health/*` or `auth-refresh/*` jobs for that source.
3. Job Runner claims the queued job from local SQLite and asks the Resource Arbiter for the relevant source lane before execution.
4. Agent Adapter selects execution mode (PTY or contract) based on the skill's `interaction_profile` and capability probe results.
5. Adapter spawns the agent session in its own process group, recording pid, pgid, adapter id, CLI version, and config hash.
6. The appropriate procurement ingestion skill is loaded into the agent (e.g., `skills/procurement-ingestion/fts/SKILL.md`).
7. Agent navigates to the source via browser automation, following the skill playbook and proving the expected page fingerprint before extraction begins.
8. Agent extracts notice data from the source, using cursor/watermark from the previous run. Durable commit boundaries are notice batches (or source pages), and resume uses a small cursor backtrack window to avoid skipping records when the upstream cursor semantics are imperfect.
9. Raw session output is captured as an immutable forensic artifact in both modes: PTY bytes in PTY mode, stdout/stderr bytes in contract mode. Structured events are also captured via adapter protocol parsing in contract mode.
10. Agent writes checkpoints and final outputs through a validated result contract (artifact file, JSON payload, or structured stream).
11. Workflow state advances from validated checkpoints/final contracts, not from transcript parsing.
12. Result contract is validated by the Result Contract Validator, then checked by the Output Quality Gate. Raw artifacts are stored in the artifact store with hashes and paths referenced from SQLite. Schema-valid outputs that fail semantic or drift checks are quarantined as `AGENT_SUSPECT_OUTPUT` instead of being committed.
12a. **Run manifest generation:** Agent writes a run manifest — a human-readable MD narrative of what happened during the ingestion run (pages visited, data extracted, decisions made, warnings encountered). The run manifest is stored as an immutable artifact at `artifacts/sessions/YYYY/MM/<session_id>/run-manifest.md`; the path and SHA-256 hash are recorded in `agent_session_logs`.
12b. **Validation policy check:** The Validation Policy Engine checks the skill's `validation_policy` metadata. If `skip`, proceed directly to step 13. If `always`, or if trigger conditions are met (fill_rate_drop, first_run_after_version_change, suspect_output_in_last_5), or if the skill is in a pre-stable promotion stage — spawn a validator sub-job.
12c. **Semantic validation:** The validator agent reads the run manifest, OQG signals, result contract summary, and `golden.assertions.yaml`. It renders a verdict: `approved`, `rejected`, or `flagged`.
12d. **Verdict processing:** `approved` → proceed to step 13. `rejected` → quarantine results as `AGENT_SUSPECT_OUTPUT`, create error manifest with validator's reasoning and suggested fixes, alert operator. `flagged` → proceed to step 13 with flag for operator review. In **dry-run mode**, steps 12b–12d run against `artifacts/scratch/<run_id>/`; verdicts are logged without enforcement, and no authoritative pipeline rows, outbox entries, or stage heads are advanced.
13. Notices are normalized into internal schema.
14. Relation resolution engine classifies the notice's relationship to existing processes:
    - **Step 14a: Hard blockers check.** Before scoring, check for hard contradictions that prevent merging:
      - Conflicting exact reference numbers.
      - One notice is framework establishment and the other is a call-off/award under framework.
      - Explicit different OCIDs where source semantics indicate separate processes (e.g., direct award after unsuccessful competition).
      - Incompatible stage chronology.
      - Same buyer and similar title but clearly different lot/value/reference structure.
    - **Step 14b: Exact-match rules.** If exact OCID match → `merged_same_process`. If explicit related-notice reference with plausible stage sequence → `merged_same_process`. If framework/call-off relation detected → `related_not_merged`.
    - **Step 14c: Heuristic scoring** (only for non-blocked, non-exact pairs):
      ```text
      merge_score =
        0.35 * reference_match
        + 0.20 * buyer_match
        + 0.20 * title_similarity
        + 0.10 * cpv_overlap
        + 0.10 * date_proximity
        + 0.05 * value_proximity
      ```
      - `>= 0.95` with no blocker → auto-merge (`merged_same_process`).
      - `0.75–0.94` → `related_not_merged` (or `possible_duplicate` for operator review).
      - `< 0.75` → `separate`.
    - **Step 14d:** For `related_not_merged` pairs, create a `ProcessRelation` edge. This enables sharing search caches, extracted product families, and price signals across the cluster without forcing a false canonical merge.
    - **Step 14e:** Map each notice into a canonical `procurement_processes` record. Merged notices update the existing record; separate notices create new records. If a previously published process ID is retired by a later merge, write a `process_aliases` redirect from the retired ID to the surviving canonical process so desktop/web deep links remain stable.
15. Job Runner persists normalized outputs and queues qualification jobs for new or materially updated processes, retaining notice-level provenance. When the skill exits `continuation_required`, the Job Runner persists the committed notice batches and immediately enqueues the next segment using the emitted resume token.
16. Session Reaper performs descendant cleanup and records cleanup verdict.
17. Stage sync: In the same SQLite transaction that writes stage outputs, the daemon writes all rows with a shared `stage_run_id`, updates `process_stage_status` to `succeeded`, advances `process_stage_heads`, and writes outbox entries. The sync engine then pushes one atomic batch to Supabase. Failed stages publish failure status + redacted error summary but do NOT advance the stage head.

#### Data Flow: Manual Trigger

1. Operator clicks "sync source" in the desktop control panel.
2. Desktop sends trigger command to the daemon.
3. Same flow as scheduled ingestion from step 2 onward.
4. Desktop control panel shows real-time progress via daemon status updates.

#### Error Cases

- Agent stalls (no output) -> daemon detects no PTY/stdout-stderr bytes for configurable timeout, marks `AGENT_STALLED`, kills process group, creates error manifest.
- Agent stalls (no progress) -> adapter detects no validated checkpoint progress despite ongoing output, marks `AGENT_STALLED`, kills process group, creates error manifest.
- Auth challenge -> daemon detects re-login prompt, MFA challenge, browser SSO failure, or hidden permission prompt, marks `AGENT_AUTH_CHALLENGE`, flags for operator.
- Host sleep during session -> daemon detects monotonic time gap exceeding threshold, marks `HOST_SLEEP_INTERRUPTED`, taints browser-backed sessions.
- Source website changed -> skill-based error, marks `AGENT_SKILL_FAILED`, creates error manifest with skill path and step.
- Result contract invalid -> agent output fails schema validation, marks `AGENT_OUTPUT_CONTRACT_INVALID`, preserves transcript + artifact, fails job without transcript-based recovery.
- Output quality gate failed -> agent output is schema-valid but fails semantic or drift validation, marks `AGENT_SUSPECT_OUTPUT`, quarantines results, and keeps pipeline state unchanged.
- Time budget reached at a safe batch boundary -> skill returns `continuation_required`; daemon commits the completed batches and enqueues the next segment automatically.
- Partial failure with checkpoint -> valid checkpoint exists before crash, persists checkpoint outputs, marks `AGENT_PARTIAL_FAILURE`, never upgrades to success without final contract + clean exit + reap.
- Agent internal error -> adapter captures error from structured events or terminal output, marks `AGENT_INTERNAL_ERROR`.
- Cleanup failure -> descendant processes leaked after session exit, marks `AGENT_CLEANUP_INCOMPLETE`, records leaked pids for operator attention.
- Resource exhaustion -> host memory/CPU pressure detected, marks `HOST_RESOURCE_EXHAUSTED`, may preempt lower-priority sessions.
- Repeated challenge-class failures -> source circuit breaker opens, source enters cooldown, and only auth-refresh/health canaries remain eligible until the source recovers.
- Daemon not running -> jobs remain in local SQLite queue; freshness SLA depends on the daemon being online. On restart, the daemon performs a catchup sweep for overdue jobs.
- Validator rejects output -> results quarantined as `AGENT_SUSPECT_OUTPUT`, error manifest created with validator's reasoning and suggested fixes, operator alerted. The error manifest includes which gate triggered quarantine (OQG vs semantic validator).
- Validator itself fails -> validator crash, timeout, or invalid verdict schema is treated as a validator error, NOT a worker error. Worker results advance as if validation were skipped, with `validation_skipped_validator_error` flag. Broken validators never block the pipeline.

### 5.7 Intelligence Pipeline Data Flow

#### Data Flow: Preliminary Qualification

1. Read normalized notice and its lot structure (if available from OCDS data).
2. **Expiry check:**
   - Determine deadline and expiry horizon per lot (or notice-level if no lots yet).
   - Reject if deadline is within 30 days.
   - **Exception:** DPS/Dynamic Market notices that accept new supplier applications do not expire in the same way — do not apply the 30-day filter to these.
   - If deadline is missing or cannot be normalized confidently, route to manual review rather than passing automatically.
3. **Access route classification:**
   - Classify the notice's access route: `open_tender`, `restricted_tender`, `framework_call_off`, `dynamic_market`, `direct_award`, `closed_framework_award`, `not_bidable`.
   - Reject `direct_award`, `closed_framework_award`, and `not_bidable` notices immediately.
   - Flag `framework_call_off` and `restricted_tender` for access-route review (bidable only if the operator is on the framework or can pre-qualify).
4. **Multi-dimensional goods classification** (per lot where lots are available, otherwise per notice):
   - **`legalCategory`:** Use CPV-based deterministic lookup first:
     - CPV Divisions 14–44 → `goods`
     - CPV Division 45 → `works`
     - CPV Divisions 50–98 → `services`
     - Special cases: software/licensing CPVs despite goods-like codes → `services`
     - Multiple CPV codes spanning categories → `mixed`
     - No CPV or ambiguous → `unknown`
   - **`goodsDensityScore`** (0.0–1.0): Combine CPV signal with text analysis:
     - Physical noun density in title/description (supply, deliver, provide vs install, maintain, design, manage)
     - Quantity/unit presence
     - Lot titles
     - Attachment filenames (spec sheets, pricing schedules suggest goods)
   - **`goodsSeparability`**: Can the goods component be bid on separately?
     - Explicit separate goods lots → `true`
     - Mixed notice but goods packages identifiable → `uncertain`
     - Goods deeply bundled with services/installation → `false`
   - **`accessRoute`**: Already classified in step 3.
5. **Qualification decision per lot:**
   - **Pass:** Pure goods, or mixed with goods separable and adequate goods density, and open/accessible bid route.
   - **Review:** Mixed/unknown with strong goods density but weak separability, or works notices with identifiable goods packages but unclear award structure.
   - **Reject:** Services only, works with negligible goods density, or closed/non-bidable access route.
6. For mixed contracts with separable goods, price only the goods package — not the whole contract value. Adjust `estimatedValue` to reflect the goods portion.

#### Data Flow: Document Acquisition

1. For each `prelim_qualified` notice, check for publicly accessible tender documents.
2. Daemon creates an adapter-mediated agent session for the `document-acquisition` skill (normally PTY-backed because the flow is browser-driven).
3. Agent navigates to the notice source portal and identifies available documents (spec PDFs, pricing schedules, BoQ spreadsheets, lot documents, tender appendices).
4. Agent downloads publicly accessible documents and outputs structured references.
5. Documents stored as raw artifacts in local SQLite, linked to the notice.
6. If no public documents are available, proceed with notice text only (lower extraction confidence).

#### Data Flow: Lot Extraction

1. Parse OCDS lot data from the normalized notice (if structured lot fields are present).
2. If no structured lots, use the `lot-extraction` skill to extract lot structure from acquired documents.
3. Create `NoticeLot` entities with: lot number, title, description, CPV codes, estimated value, deadline, documents, bidability.
4. If no lots are identifiable, create a synthetic whole-notice lot.
5. Run per-lot qualification if the notice-level qualification was provisional.

#### Data Flow: Product Extraction

1. For each qualified lot, extract `ProductCandidate` entities from lot descriptions and acquired documents.
2. Classify `candidateType`: `exact_sku`, `standard_commodity`, `brand_family`, `bundle`, `custom_or_fabricated`, `installed_goods`, `regulated_goods`, `unknown`.
3. Classify `matchArchetype`: `standard_class_led`, `dimension_material_led`, `electrical_spec_led`, `pack_count_led`, `compatibility_led`, `model_exact`, `unknown`. The match archetype describes how product identity is recovered on marketplaces and is orthogonal to `candidateType` (see §4.3). If extraction cannot determine the archetype, default to `unknown`.
4. **Decompose bundles** into constituent candidate families before downstream processing.
5. Extract structured slots: headNoun, criticalAttributes, standards, numericSpecs, material, brandModel, preferredTerms, excludedTerms.
6. **Assign slot criticality** per extracted attribute: `required` (tender mandates it), `preferred` (tender mentions preference), `descriptive` (informational only), `exclusion` (explicitly excluded). Use tender language signals ("must", "shall", "required" → `required`; "preferred", "desirable" → `preferred`; "not", "excluding" → `exclusion`). When ambiguous, default to `preferred`. The `matchArchetype` provides category-level defaults: for `standard_class_led`, standards slots default to `required`; for `dimension_material_led`, key dimension slots default to `required`.
7. **Normalize standard references** into structured components: family (EN 397), part, edition year, amendment (A1:2012), and class/level. This structured representation enables precise standard comparison during hard-conflict filtering (see §5.7).
8. Use **category-specific critical attribute tables** keyed off CPV family (PPE: standards/material; Electrical: voltage/plug/wattage; Furniture: dimensions/fire-rating; IT: brand/model/specs; Medical: standard/classification/sterile).
9. Assign separate confidence scores: `identityConfidence`, `specConfidence`, `quantityConfidence`.
10. Store candidates in local SQLite linked to their lot.

#### Data Flow: Quoteability Scoring

1. Compute deterministic `quoteabilityScore` per candidate based on: commodity-like head noun, critical attributes, numeric specs, standards, quantity/unit, public docs, open access route; minus penalties for custom fabrication, installation scope, closed framework, missing compliance detail.
2. Thresholds: `>= 0.65` search automatically; `0.45-0.64` search top-value only; `< 0.45` skip marketplace, route to watch/RFQ/manual.
3. Compute `recallRiskScore` per candidate from: alias drift for the `matchArchetype`, compliance criticality, accessory contamination risk for the product family, spec sparsity, expected source noisiness, and language mismatch risk. This score drives adaptive shortlist budgets during price-signal matching.
4. Generate structured search profile with multilingual `queryWaves` (see §5.7) for passing candidates. Each profile includes: head noun family, `matchArchetype`, noun aliases (EN/ZH), market framing terms (EN/ZH), required/optional anchors, exclusion terms, use context, and sector context. Query waves are generated per source, from most precise to broadest. Translate only structured product slots plus market framing, not whole sentences.
5. Store search profiles in local SQLite for debugging and lexicon improvement.

#### Data Flow: Price-Signal Matching (Skill-Based Agent Scraping)

1. **Source routing** by `candidateType` (see section 4.3). Not every source for every candidate.
2. **Session batching:** Resource Arbiter grants supplier-source lanes; run at most one automated search session per source per notice (multiple candidates batched together). The durable business unit is the **candidate-source pair**: a single session may work many pairs, but checkpoints, resumability, and downstream commits are tracked per pair.
3. **Query-wave planning:** For each candidate × source, the query planner generates a sequence of query waves from the search profile, ordered from most precise to broadest:
   - **Wave 0 — exact anchor:** Head noun alias + strongest anchor (model number, standard family, key numeric spec). High precision, low recall.
   - **Wave 1 — alias swap:** Change the noun phrase, keep the anchor. Tests marketplace vocabulary variants.
   - **Wave 2 — market framing:** Add use-context / industry framing terms from `marketFrameEn`/`marketFrameZh`. Recovers results behind different market vocabulary.
   - **Wave 3 — broad existence check:** Drop soft modifiers. Keep only head noun family + one anchor. Tests whether the product category exists on the source at all.
   - **Wave 4 — compliance-off search (regulated goods only):** Remove the standard token from the query, keep it as a deterministic post-filter. Critical because many compliant listings omit the standard from the title but include it on the detail page.
   Each wave records its query text, source, locale, and wave number for debugging and recall measurement.
4. **Search execution:** Agent searches using query waves from the search profile. Full raw session output is captured. Results across all waves for a candidate × source are collected into a union set, de-duplicated by listing identifier. A zero-result page only enters the recovery ladder when the page fingerprint is valid and no challenge markers are present; blocked empty shells are classified as source failure, not as business-level zero results.
5. **Search-card screening:** Parse search results and apply hard-conflict filters using **four evidence states** per required slot:
   - `match` — listing evidence explicitly satisfies the slot requirement.
   - `missing` — listing does not mention the attribute (no evidence either way).
   - `ambiguous` — listing evidence is present but unclear or partially matching.
   - `conflict` — listing evidence explicitly contradicts the slot requirement.
   Only `conflict` triggers hard rejection. `missing` and `ambiguous` are confidence penalties, not disqualifications.

   **Hard-conflict matrix:**

   | Conflict Type | Hard Reject When | Penalty / Review When |
   |---|---|---|
   | Brand / model | Explicit different brand/model on `exact_sku` or locked brand family | Brand/model absent |
   | Standard / certification | Wrong standard family; wrong part; class below required; explicit non-equivalent substitute | Same family but edition/amendment absent; class not stated |
   | Voltage / frequency / plug | Explicit incompatible mandatory electrical spec | Missing electrical spec |
   | Material | Explicit contradiction on mandatory material (`nitrile` vs `latex`) | Material missing or non-mandatory |
   | Dimensions / size | Explicit incompatible fixed size or range | Size missing; adjustable size |
   | Condition | Used/refurbished when not allowed | Condition unclear |
   | Accessory vs primary | Explicit accessory / liner / refill / replacement / shell-only / compatible-with listing | Bundle or unclear title |
   | Consumer vs industrial | Explicit sports / toy / fashion / domestic variant conflicting with required industrial use | Weak industrial signal only |
   | Pack basis | Wrong product form (single vs pair, consumable vs kit) | Unclear pack/unit basis |

   **Standards normalization:** Before comparing, normalize standard references into components: family (EN 397), part, edition year, amendment (A1:2012), and class/level. Rules: wrong family or part = hard reject; same family with amendment absent = soft conflict; same family with edition absent = penalty; class below required = hard reject; class absent = review/penalty.

   **Consumer vs industrial detection:** Do not use price as the primary signal. Use: category path on marketplace, standard family (EN 1078 cycling vs EN 397 industrial), title vocabulary (bike / climbing / kids = hard reject for industrial PPE), seller assortment, detail-page intended-use language, and structured spec attributes.

6. **Detail-page enrichment:** For results that survive search-card screening as plausible matches (not hard-rejected, not obvious garbage), fetch the detail page to extract:
   - Structured spec table / product attributes
   - Standards/certification mentions not visible on search card
   - Seller information and assortment context
   - Pack/unit details
   - Shipping and MOQ specifics
   - Intended-use language

   This is where `complianceEvidenceTier` is determined: `structured_attribute` (spec table), `title_body_mention` (text only), or `absent`. For `regulated_goods`, missing compliance evidence on the search card alone does not disqualify — the detail-page check is mandatory before rejection for absent compliance. If list-page results are usable but detail pages become unavailable, auth-gated, or rate-limited after shortlist completion, the skill may exit `degraded_completed` with list-only quotes plus explicit confidence penalties instead of failing the whole candidate-source pair.

7. **Diversified deterministic shortlist:** After hard-conflict filtering and detail-page enrichment, build a shortlist from the de-duplicated union of results across all query waves. Shortlist budget is adaptive, driven by `recallRiskScore`:
   - Low recall risk: deterministic top 8, AI adjudication on 0–2 borderline
   - Medium recall risk: deterministic top 12, AI adjudication on 3
   - High recall risk: deterministic top 20, AI adjudication on 5–8

   **Diversity enforcement rules** (applied before rank truncation):
   - Max 2 results per seller
   - Max 2 results per query wave
   - Max 1 result per normalized title family (listings with near-identical titles after brand/seller removal)

   Without diversity enforcement, the shortlist for noisy categories (PPE, lighting, accessories) will contain near-duplicates that waste AI adjudication budget.

   **`recallRiskScore`** is computed from: alias drift for the archetype, compliance criticality (`regulated_goods` = high), accessory contamination risk for the product family, spec sparsity on the candidate, source noisiness (1688 > Alibaba > Amazon > eBay for search noise), and language mismatch risk (Chinese sources = higher).

8. **AI adjudication (borderline only):** Agent judges pre-filtered borderline results from the deterministic shortlist. AI does not perform new searches — it evaluates identity and usability of candidates that the deterministic scorer could not confidently classify. Budget scales with `recallRiskScore`.

9. **Dual confidence scoring:** Apply per result:
   - `matchConfidence` = 0.40 × product_type_match + 0.30 × critical_attribute_match + 0.20 × numeric_spec_match + 0.10 × evidence_quality. Weights are MVP priors; archetype-specific profiles adjust them (e.g., `model_exact` shifts weight toward product_type_match; `dimension_material_led` shifts weight toward numeric_spec_match).
   - `quoteUsabilityConfidence` = 0.35 × unit_basis_clarity + 0.25 × MOQ_compatibility + 0.20 × shipping_visibility + 0.20 × completeness.
   Never average these. Apply lexicographic gating: identity gate first (matchConfidence floor by archetype), then usability gate, then price-signal role assignment (see §4.3).

10. **Price-signal role:** `benchmark_ceiling` (UK retail), `optimistic_floor` (China wholesale), `executable` (RFQ/negotiated).
11. Save quotes with dual confidence, compliance evidence tier, price-signal role, and `SearchAttempt` metadata to local SQLite. Persist each wave execution in `search_attempts`; cache reusable detail-page evidence in local-only `listing_evidence`. A listing can be a valid match without becoming a `SupplierQuote`; only screened, normalized, commercially usable results are persisted as quote rows.
12. **Cache sharing:** Reuse listing evidence (not just search results) across related-but-unmerged clusters via `ProcessRelation` edges. See cache strategy below. If a source is in cooldown, use only fresh cached evidence and suppress new detail-page fetches until the circuit closes.
13. Stage sync: write outbox entries in same transaction as quote storage, update stage status/heads, push atomic batch to Supabase.

#### Data Flow: Zero-Result Recovery

When marketplace search returns zero usable results for a candidate × source combination, the system applies a staged recovery ladder rather than treating zero results as a verdict:

1. **Round 1 — drop weakest modifier:** Remove the least-critical optional anchor. Keep head noun + one hard anchor.
2. **Round 2 — alias swap:** Change the noun phrase using the next alias from the search profile. Keep the hard anchor.
3. **Round 3 — market framing:** Add use-context / industry framing terms from `marketFrameEn`/`marketFrameZh`.
4. **Round 4 — compliance-off search:** For regulated goods, remove the standard token from the query, keep it as a post-filter on results.
5. **Round 5 — China language escalation:** Alibaba EN → Alibaba ZH → 1688 ZH (when routed).
6. **Round 6 — broad existence check:** Search on category-family term only (head noun family + source).

**Stop conditions:**

- Max 2 recovery rounds per source beyond the initial query waves
- Per-candidate query budget ceiling (higher for high-value or high-risk candidates)
- Stop immediately if broad category queries find plausible items — the problem is the specific query, not the market

**Search failure classification:** Every zero-result or low-quality-result outcome is stored with a `searchFailureMode`:

| Failure Mode | Diagnosis | Implication |
|---|---|---|
| `vocabulary_gap` | Broad queries find products but specific queries miss | Query planner needs alias expansion for this family |
| `overconstrained_query` | Too many anchors for the source's search quality | Relax constraints; more recovery rounds |
| `accessory_swamp` | Source returns mostly accessories/parts, not primary products | Add stronger negative terms; adjust source routing |
| `compliance_not_surfaceable` | Product exists but compliance evidence is not marketplace-searchable | Detail-page-only compliance check; longer search budget |
| `no_marketplace_presence` | All broad queries across all routed sources return nothing | Genuine absence — route to RFQ/manual |
| `low_extraction_quality` | Product candidate description is too vague for meaningful search | Upstream issue — confidence penalty, not a search failure |
| `source_specific_failure` | Source website error, rate limit, or layout change | Retry later; error manifest |

**Key distinction:** If broad category queries find lots of plausible items but the specific query finds none, that is `vocabulary_gap` — a query problem, not a market absence. If all broad queries across all routed sources return nothing, that is closer to genuine `no_marketplace_presence`.
Blocked empty shells, soft blocks, and challenge pages never count as `no_marketplace_presence`; they stay in the source-failure path and can open the source circuit breaker.

#### Data Flow: Recall Measurement and Calibration

For every operator-accepted match, log:

- source, query wave number, raw rank in source results, reranked position in shortlist
- whether the match was inside the AI adjudication pool, the deterministic shortlist budget, or the extended shortlist window
- `matchArchetype` and `candidateType`

Compute weekly/monthly:

- recall@3 for AI pool, recall@10 deterministic, recall@20 deterministic — by `matchArchetype × source`
- operator override rate (matches accepted that the system would have rejected)
- false-positive reason mix
- score histogram and precision by score decile
- reliability curve (predicted confidence vs actual acceptance rate)

If 80% of scores land in the 0.50–0.70 middle band, the confidence weights are not informative enough. If the distribution is bimodal (clearly good or clearly bad), thresholds can be tightened. This data feeds back into archetype-specific confidence floor calibration.

**Calibration approach:** For MVP, keep deterministic rules with explicit feature logging. When sufficient operator data accumulates (target: 200+ labeled decisions per archetype × source), fit a simple interpretable calibration layer (isotonic regression or Platt scaling) per source × `matchArchetype`, backed off to a pooled calibrator when data is sparse. Store `rawMatchScore` (the deterministic pre-calibration score) alongside the calibrated `matchConfidence` for auditability. Calibration recalibrates scores to probabilities without replacing the deterministic feature set.

#### Data Flow: Pipeline Cost Management

Naive design: 1,200+ sessions/day at $0.18-$0.41 = $75-$170/day. Required optimizations:

1. **Batch by source:** One ingestion session per source per run. One supplier session per source per notice (searching multiple candidates).
2. **Search selectively:** In typical MVP runs, search the top 2-3 candidates per notice by quoteability/spend share. The default operating hard cap is 8 auto-searched candidates per notice. Skip 1688 unless commodity-type.
3. **Deterministic first:** Hard-conflict filters and shortlisting are deterministic. AI adjudication budget scales with `recallRiskScore` (0–2 for low risk, up to 5–8 for high risk), not a fixed top-3 for all categories.
4. **Two-tier cache:** Separate search-result cache from listing-evidence cache for maximum reuse (see below).
5. **Model tiering:** Mini/Haiku for classification/extraction. Premium models only for ambiguous adjudication.
6. **Realistic throughput:** 20-30 priced notices/day after filtering. Target $20-$60/day metered.
7. **Default operating limits (MVP):** start with 2 browser slots, 4 total agent slots, 1 concurrent procurement session per source, 2 concurrent supplier sessions per source, max 25 extracted candidates per notice, max 8 auto-searched candidates per notice, and at most 3 supplier sources searched automatically per product candidate unless an operator forces a wider sweep. Zero-result recovery adds at most 2 extra query rounds per source beyond initial waves.

**Two-tier cache strategy:**

The cache separates into two layers with different invalidation semantics:

**A. Search-result cache:** Stores per query execution:
- source, locale, query text, query wave number
- returned listing IDs with rank/order
- result count, timestamp

**B. Listing-evidence cache:** Stores per listing ID (far more valuable):
- normalized title, structured attributes / spec table
- seller identity, MOQ, shipping terms, condition
- parsed standards, parsed units/packs
- compliance evidence tier
- detail-page extraction timestamp

The listing-evidence cache lets the system re-score the same listing against a new candidate without paying the fetch cost again. This is the primary cost saver for related candidates within the same product family.

**Query fingerprint design:** Fingerprints are structured, not raw-string hashes. Split into:

- **Hard fingerprint** (identity-defining): source, locale, `headNounFamily`, `matchArchetype`, required anchors, compliance-required flag, negative family. Changing a hard anchor invalidates cache reuse entirely.
- **Soft fingerprint** (optional modifiers): optional anchors, market framing terms, cosmetic descriptors. Changing a soft anchor triggers re-ranking of cached results, not a full cache miss.

Example: `gloves` → `nitrile gloves` = hard change (invalidates). `nitrile gloves` → `blue nitrile gloves` = soft change (re-rank only).

**Source-specific freshness TTLs:** A single 72-hour TTL is too blunt. Different sources have different price volatility:

| Source | Search-Result TTL | Listing-Evidence TTL | Rationale |
|---|---|---|---|
| eBay UK | 24h | 48h | Price signals are volatile (auction/fixed mix, stock turnover) |
| Amazon UK | 36h | 48h | Prices change frequently but less than eBay |
| Alibaba | 48h | 72h | MOQ prices are relatively stable; shipping options change |
| 1688 | 72h | 96h | Domestic wholesale prices are the most stable |
| Ready-to-Ship (Alibaba) | 36h | 48h | Fixed-price, decays faster than custom listings |

Cached results carry **decayed commercial confidence**: identity evidence from the listing-evidence cache remains mostly stable, but price freshness, shipping freshness, and stock freshness decay with age. The `quoteUsabilityConfidence` of a cached result is penalized proportionally to age relative to the source-specific TTL.

**Cache sharing across process clusters:** Reuse results across related-but-unmerged clusters via `ProcessRelation` edges. When Process A searched "nitrile gloves EN 455" and Process B has "nitrile examination gloves EN 455", the listing-evidence cache allows re-scoring cached listings against Process B's candidate without re-fetching, provided the hard fingerprint overlaps. The search-result cache is not shared across different hard fingerprints.

#### Data Flow: Landed Cost (API-Based)

**Revenue normalization (before any margin gate):**

1. Determine **revenue basis type**: firm contract, estimated contract value, framework ceiling, dynamic-market admission, or concession. Under Procurement Act guidance, estimated contract value is the maximum value payable and is inclusive of VAT. For frameworks, the estimate is the value of all call-offs and must not be exceeded, but this does not make it guaranteed revenue.
2. Normalize to `expected_addressable_revenue_ex_vat`. Do not use raw notice value — framework ceiling or dynamic-market values are not firm revenue.
3. Assign `revenueConfidence` based on basis type. Firm contracts score higher than framework ceilings.

**Per-line-item estimation (do not force one commodity code onto a mixed basket):**

4. Split mixed notices into separate product candidate line items (bundle splitting should have occurred at the product extraction stage).
5. Select best supplier quote pairs per line item.
6. **Normalize each supplier quote to a common cost boundary based on Incoterm.** Under EXW, buyer pays all transport; under FOB, seller pays to port; under CIF, seller pays freight + insurance; under DDP, seller handles everything including import clearance. Without normalization, the gross-margin gate is mathematically inconsistent across quotes with different Incoterms.
7. **Classification pipeline:** Apply description-quality gate → attribute extraction → multi-query **HMRC FPO API** for HS6/CN8 candidate codes → cluster and rank candidates → resolve to 10-digit declarable codes via **HMRC Trade Tariff API** → validate via **Taric Support API** → cross-check via **HS Ping**.
8. **Measure risk scan:** Query HMRC Trade Tariff API import measures for each resolved 10-digit code. Check for: base duty (including compound/specific), trade remedy duties (anti-dumping, countervailing, safeguard), preferential eligibility, quotas, import controls/prohibitions, additional codes, condition thresholds, and required measurement units.
9. **Freight estimation:** Resolve Incoterm cost boundary → apply hard mode locks (hazmat, oversized, fragile, reefer, batteries) → infer packaging state → select mode(s) → fetch estimates from **Freightos** and **SeaRates** APIs → if packaging state unknown, run dual scenarios → emit `freightLowGbp` / `freightBaseGbp` / `freightHighGbp` with `modeConfidence`.
10. Convert to GBP using:
    - **HMRC Exchange Rate Service** for customs duty calculation (legally authoritative, `customsFxRate`),
    - **Xe** for commercial margin estimation (real-time market rates, `commercialFxRate`).
    - Compute and store `fxBasisDeltaPct` between the two rates.
11. Determine **import VAT treatment**: postponed (full/partial), cash-paid (full/partial), or non-recoverable. Assign `vatRecoveryRate` rather than a binary flag. Record `customsValuationMethod` used (Method 1–6 or unknown).
12. **Estimate remaining cost components:**
    - `ukFinalMileCostGbp` — UK domestic delivery from port/airport to contract delivery address. Use internal rate cards, carrier quotes, or heuristic by weight/volume and distance.
    - `otherFeesGbp` — customs brokerage fees (~£30–150 per entry), port/terminal handling charges (~£150–400 per container), insurance (typically 0.5–2% of CIF value), quality inspection if applicable.
13. **Compute landed cost and margin scenarios** per line item:
    - `economicLandedCostGbp` = supplierCost + freightBase + customsDuty + tradeRemedyDuty + ukFinalMile + otherFees (ex-recoverable-VAT)
    - `netMarginLowPct`, `netMarginBasePct`, `netMarginHighPct` (economic, ex-VAT basis, using freight low/base/high and duty variation from classification uncertainty),
    - `cashToClearGbp` (total upfront cash including non-recoverable or delayed-recovery VAT),
    - `cashOutlayGbp` (total cost including all VAT whether recoverable or not),
    - `workingCapitalCostGbp` (optional — cost of capital tied up between payment and revenue).
14. **Compute confidence breakdown:** Calculate each of the 6 components (`classificationStability`, `quoteRealism`, `commodityCodeConfidence`, `freightModeStability`, `revenueReliability`, `fxFreshness`) and the weighted composite. Assign `decisionGrade`: `screening` | `review` | `rfq_required` | `broker_required`.
15. **Blocker check:** Stop auto-qualification when:
    - Classification is unstable (HS6 candidates diverge or 10-digit code unresolved)
    - Revenue basis is uncertain (framework ceiling, dynamic market, no firm commitment)
    - Freight mode remains unresolved or mode confidence is low
    - Trade remedy duties detected
    - Unsatisfied measure conditions or import controls present (operational blockers)
    - Quote class is only `indicative_marketplace` and margin is within 10 percentage points of gate
16. Roll up line-item estimates to process-level aggregate estimates.
17. Store warnings, confidence breakdown, blockers, and decision grade in local SQLite.
18. Sync results to Supabase.

**Validation note for intelligence pipeline skills:** The same run manifest generation and semantic validation flow described in §5.6 steps 12a–12d applies to all skill-based pipeline stages above (document acquisition, lot extraction, product extraction, supplier enrichment). Each skill produces a run manifest; the Validation Policy Engine determines whether a validator sub-job is required. The validation step is inserted after each skill's OQG check and before results are committed to pipeline state. Intelligence pipeline skills that perform browser-based extraction (document acquisition, supplier enrichment) benefit most from semantic validation because they are most susceptible to "clean run, wrong page" failures.

### 5.8 Control Panel and Viewing Portal Data Flow

**Terminology note:** In this document, **desktop app**, **desktop shell**, and **desktop control panel** all refer to the same writable Tauri client. **Web app**, **web shell**, and **viewing portal** all refer to the same read-only hosted client. **Procurement process** is the canonical internal record; the UI may call it an **opportunity**. A **contract notice** is the source document that feeds a procurement process.

The desktop/web capability split is formalized in **ADR-008**.

**Architectural framing:** The desktop is not "React talking to SQLite." It is React talking to a **local control plane**. The viewing portal is not "React talking to the same hooks." It is React talking to a **read replica with live invalidation**. These two surfaces share presentational components and view-model types but have fundamentally different data access patterns, latency characteristics, and capability sets.

```text
DESKTOP
React SPA (Vite)
  → Tauri invoke()          for snapshot queries + mutations
  → Tauri Channel<T>        for live streams (sessions, jobs, daemon health)
  → Desktop Rust bridge
       → QueryService       (read-only SQLite, page-specific read models)
       → DaemonBridge       (UDS/named-pipe RPC to daemon)
            → Rust daemon   (writer, jobs, PTY, sync, Supabase)

WEB
Next.js / React
  → Supabase snapshot queries (projection tables)
  → Supabase Broadcast private channels (invalidation hints)
  → refetch / patch read-only projections
```

#### Desktop ↔ Daemon Communication Protocol

The desktop communicates with the daemon through a hybrid transport. The logical API resources are defined in §5.10; this section describes the transport layer that delivers them.

- **Snapshot reads:** `invoke()` from JS → Tauri-side Rust **QueryService** that opens **read-only** SQLite connections and returns page-specific read models. The QueryService enforces: read-only/query-only connections, page-limited queries, no streaming cursors held open, no generic SQL from the frontend, no transcript/blob reads in hot queries. The desktop reads local operational authority directly, but in a controlled way. In `recovery_required` mode, the QueryService returns error responses — it does not serve stale data from a mismatched schema.
- **Writes and triggers:** `invoke()` from JS → Tauri-side Rust **DaemonBridge**, which forwards commands (the logical endpoints in §5.10) to the daemon over a Unix domain socket (macOS/Linux) or named pipe (Windows). The DaemonBridge is a thin RPC proxy — it does not hold state or perform writes itself.
- **Real-time streams (desktop only):** Daemon → DaemonBridge → **Tauri `Channel<T>`** → React. Channels are Tauri's fast ordered streaming primitive, used for the stream surfaces defined in §5.10 (session output, job progress, daemon health). The DaemonBridge constructs `Channel<T>` instances and relays daemon subscription events through them. The Tauri event system is reserved for smaller JSON-string payloads (notifications, state transitions), not high-throughput streaming.
- **Debug/dev adapter:** An optional localhost HTTP adapter exposes the same `/api/*` logical resources for CLI tooling and development. This is not the production desktop transport.

**Why not pure HTTP on localhost:** A production loopback HTTP server adds port discovery/conflicts, auth/token management, another attack surface for same-user local processes, and network semantics for something that is not a networked relationship.

**Why the Tauri backend is not the daemon:** The daemon is a background scheduler/executor. It must survive UI close, restart independently, and have its own lifecycle. Embedding it into the Tauri app makes shutdown, crash recovery, and "app closed but cron still runs" architecturally unsound.

**SQLite concurrency rule:** SQLite is single-writer (daemon only), but supports many short readers. The desktop QueryService opens read-only connections with short-lived transactions. Long-lived readers are prohibited because they prevent WAL checkpoints from draining. The QueryService never reads transcript blobs or large artifacts in hot queries — those load on demand from the artifact store.

**App-facing protocol contracts:**

```ts
type QueryEnvelope<T> = {
  data: T
  snapshotSeq: number
  sourceMeta: {
    mode: 'healthy' | 'browse_only' | 'recovery_required'
    daemonConnected: boolean
    syncHealth: 'healthy' | 'degraded' | 'offline'
  }
}

type MutationAck = {
  accepted: boolean
  idempotencyKey: string
  jobId?: string
  remoteState?: 'committed' | 'queued' | 'conflicted' | 'not_applicable'
}
```

**Bridge RPC types (desktop Rust ↔ daemon):**

```ts
type BridgeRequest =
  | { id: string; kind: 'command'; method: string; params: Record<string, unknown>; idempotencyKey: string }
  | { id: string; kind: 'subscribe'; topic: string; fromSeq?: number }
  | { id: string; kind: 'unsubscribe'; topic: string }

type BridgeEvent =
  | { topic: 'daemon.health'; payload: DaemonHealthPayload }
  | { topic: string; payload: JobProgressPayload }
  | { topic: string; seq: number; kind: 'pty' | 'stdout' | 'stderr' | 'event' | 'lifecycle' | 'gap'; payload: unknown }
```

#### Data Flow: Desktop Control Panel (Full Read/Write/Trigger)

1. Operator opens the Tauri desktop app. Tauri Rust bootstrap starts, checks local DB readability and schema epoch, attempts daemon handshake over socket/pipe.
2. Desktop queries canonical procurement processes by state through `invoke()` → QueryService → **read-only local SQLite** (fast, no network dependency). QueryService returns `QueryEnvelope<T>` with `snapshotSeq` and `sourceMeta`.
3. Operator opens opportunity detail.
4. Detail view loads (all via QueryService snapshot reads):
   - canonical process summary,
   - normalized source notices,
   - source links,
   - pricing evidence,
   - logistics/customs assumptions,
   - manifest,
   - agent session history plus on-demand terminal/trace views,
   - error manifest entries (if any).
5. Operator chooses:
   - reject (→ DaemonBridge → daemon, returns `MutationAck`),
   - watch (→ DaemonBridge → daemon, returns `MutationAck`),
   - rerun enrichment (→ DaemonBridge → daemon creates adapter-mediated agent session),
   - start bid workflow (→ DaemonBridge → daemon creates portal skill session).
6. Operator can also:
   - trigger manual source syncs,
   - manage source configs, skill bindings, and local runtime settings,
   - view comprehensive agent tracing logs,
   - view and resolve error manifests.
7. All lists use stable **keyset/cursor pagination**, not offset pagination. Offset pagination causes rows to jump and duplicate when live updates modify the underlying data.

#### Real-Time Event Streaming Architecture

Live agent session monitoring is the hardest frontend problem in the system. The design treats session output as a **log system**, not a React state problem.

**Rule: never put raw transcript arrays in React state.**

**Three-layer streaming model:**

1. **Daemon buffer + artifact writer:** For each session, the daemon maintains immutable artifact chunks on disk, an in-memory hot ring buffer, monotonic `seq` numbers, normalized structured events, and session summary metadata.
2. **Desktop stream store:** Purpose-built external stores per session, outside React's render cycle.
3. **Terminal/timeline views:** Thin React wrappers that subscribe to store slices.

**Session frame types** (lifecycle states correspond to the 7-phase session lifecycle in §4.9: `created → spawned → active → terminating → exited → reaped → archived`):

```ts
type SessionFrame =
  | { seq: number; kind: 'lifecycle'; state: string; ts: string }
  | { seq: number; kind: 'pty'; chunk: Uint8Array; ts: string }
  | { seq: number; kind: 'stdout'; chunk: string; ts: string }
  | { seq: number; kind: 'stderr'; chunk: string; ts: string }
  | { seq: number; kind: 'event'; event: StructuredSessionEvent; ts: string }
  | { seq: number; kind: 'gap'; fromSeq: number; toSeq: number }
```

**Attach semantics — two subscription levels:**

- **All running sessions (metadata only):** Status, checkpoint count, last line excerpt, elapsed time. Cheap enough for the session list page to show 20+ concurrent sessions.
- **Selected session(s) (full stream attach):** Full PTY/stdout/stderr stream via `Channel<T>`. At most 2–3 sessions attached simultaneously (primary viewer + optional detached window).

**Buffering strategy for the live viewer:**

- Batch frames every 33–50ms before writing to the terminal component.
- Keep only the last N lines / M MB hot in the live buffer.
- If the user scrolls up, switch from live-tail mode to history mode (lazy-loaded from artifact chunks).
- If the UI falls behind the stream, emit a `gap` frame and force a resync from the last known artifact position.

**Dual viewer pattern:**

1. **Live terminal view:** ANSI-faithful, auto-scroll, optimized for the last few thousand lines. Default for PTY-mode sessions.
2. **Transcript/history view:** Virtualized line list, searchable, lazy-loaded from artifact chunks. For reviewing past output or searching large transcripts.

**PTY mode vs contract mode — different default views:**

- **PTY mode default:** Live terminal tail (ANSI terminal emulator component).
- **Contract mode default:** Structured timeline showing checkpoints, tool invocations, browser events, artifact emissions, final contract validation, and cleanup verdict. Stdout/stderr available as secondary tabs.

**React state pattern:** Use purpose-built external stores per session with `useSyncExternalStore()` for slice subscriptions:

- `SessionRegistry` — tracks all active/recent sessions with metadata.
- `SessionBuffer` — per-session ring buffer for live frames.
- `useSessionMeta(sessionId)` — metadata slice (status, elapsed, checkpoints).
- `useSessionTail(sessionId)` — last N lines for terminal view.
- `useSessionTimeline(sessionId)` — structured event timeline for contract mode.

This gives slice subscriptions instead of rerendering the whole page on every chunk.

#### Data Flow: Viewing Portal (Read-Only)

1. Viewer opens the viewing portal.
2. Web gets an initial snapshot from **Supabase projection tables** (designed for portal use-cases, not raw sync tables — see §4.7 for the push-pull sync protocol that populates them), plus `workflow_job_summaries` and `error_manifest_summaries` for operator-safe status visibility.
3. Web attaches **Supabase Broadcast private-channel subscriptions** for live invalidation hints (see Web Portal Subscription Architecture below).
4. Every page displays **`viewer_sync_status`** so stale remote data is explicit.
5. Viewer opens opportunity detail.
6. Detail view loads (all from Supabase projection tables):
   - canonical process summary,
   - normalized source notices,
   - source links,
   - pricing evidence,
   - logistics/customs assumptions,
   - manifest,
   - pipeline status,
   - error status.
7. **No write operations.** Portal remains read-only in MVP; do not build generic cross-device edit sync before a second writable desktop exists. The viewing portal does not support:
   - reject/bid/rerun controls,
   - agent management,
   - configuration changes.
8. All lists use stable **keyset/cursor pagination**, not offset pagination.

These are desktop-only operations.

#### Web Portal Subscription Architecture

The viewing portal uses **Supabase Broadcast as an invalidation plane**, not as a database subscription. Broadcast payloads are small invalidation hints, not full rows.

**Why Broadcast over Postgres Changes:** Postgres Changes is simpler but scales worse (single-threaded for ordering). Broadcast supports private channels, custom payloads via `realtime.send`, and better security scoping. The portal never subscribes to raw row changes on hot tables.

**Topic scheme (page-level, not row-level):**

| Page | Topics |
|------|--------|
| `/contracts` | `org:{orgId}:contracts`, `org:{orgId}:sync` |
| `/contracts/[id]` | `process:{processId}:updates`, `org:{orgId}:sync` |
| `/status` | `org:{orgId}:status`, `org:{orgId}:errors`, `org:{orgId}:sync` |

**Invalidation payload types:**

```ts
type PortalUpdate =
  | {
      event: 'contracts.changed'
      processIds: string[]
      changed: Array<'assessment' | 'decision' | 'manifest' | 'error'>
      maxSeq: number
    }
  | {
      event: 'process.updated'
      processId: string
      changed: Array<'quotes' | 'landed_cost' | 'manifest' | 'jobs' | 'errors'>
      major: boolean
      maxSeq: number
    }
  | {
      event: 'sync.updated'
      syncHealth: 'healthy' | 'degraded' | 'offline'
      lastAckedSeq: number
      lastSuccessAt: string
    }
```

**Emission point:** The daemon emits Broadcast notifications **after** the remote projection batch is successfully applied. The apply flow coalesces affected process/org topics so that 20 quote row upserts produce one notification per affected topic, not 20 websocket messages.

**Per-page viewing-portal behavior:**

- **List page:** Patch cheap badges/counts if present in the invalidation payload. Otherwise invalidate the list query. Do NOT reorder the visible list live while the user is reading — show a "N new / updated opportunities" pill that applies on click or on next refetch.
- **Detail page (minor change — jobs, status, sync):** Auto-refresh the affected section.
- **Detail page (major change — manifest, landed cost, canonical process):** Show a non-destructive refresh chip. Do not silently swap the full manifest out from under the user.

**Sync lag handling:**

If daemon sync is delayed, the portal is stale by definition. No subscription design fixes that. The portal is honest about staleness:

- Every portal page renders `viewer_sync_status` globally.
- Suggested thresholds: **green** `< 90s`, **amber** `90s–5min`, **red** `> 5min` or daemon offline.
- Display text: "Data may be behind desktop local state."
This badge is backed by the push-owned `viewer_sync_status` projection table in Supabase, updated after each successful remote apply.

#### Shared React Component Architecture

The shared layer uses a **snapshot + overlay** model:

- **Snapshot:** Durable read model from SQLite (desktop) or Supabase (web).
- **Overlay:** Ephemeral live data, pending intents, freshness state.
- **View:** Shared presentational React component consuming view-model props.

**State management by class:**

| State Class | Desktop | Web |
|-------------|---------|-----|
| Snapshot server-state | Small query cache (TanStack Query for snapshot reads only) | Same |
| Live streaming state | Custom external stores + `useSyncExternalStore` | Broadcast invalidation manager |
| Ephemeral UI state | Local component state / tiny persisted store | Same |

TanStack Query is used for **snapshot reads only** — no offline persistence, no normalized cache gymnastics, no session logs in the query cache.

**What to share in `packages/ui`:**

- `ContractListView`, `ContractDetailView` (presentational frames)
- `ManifestViewer`, `PipelineBadge`, `SyncAgeBadge`, `ErrorSummaryList`
- Formatting helpers, Markdown renderers, table components, badges
- Prop types and view-model types

**What NOT to share:**

- `useContractsList()` hooks (data-fetching is surface-specific)
- Supabase subscriptions or Tauri invoke wrappers
- Mutation orchestration
- Session stream stores
- Desktop-only capability checks

**Component composition rule:** Do not make one component with fifteen boolean props (`canReject`, `canBid`, `readOnly`, ...). Use frame + slot composition:

```tsx
<ContractCardFrame
  summary={vm.summary}
  metrics={vm.metrics}
  rightRail={<SyncAgeBadge meta={vm.meta} />}
  actions={desktop ? <DesktopContractActions ... /> : null}
/>
```

**Sharing boundary:** Share down to **view-models**, **presentational components**, and **formatting/markdown/tables/badges**. Stop sharing at **repositories**, **hooks**, **realtime subscriptions**, and **mutation flows**.

#### Desktop Lifecycle Management

**Startup sequence:**

1. Tauri Rust bootstrap starts.
2. Check local DB readability and `schema_epoch` (via `PRAGMA user_version`).
3. Attempt daemon handshake over socket/pipe.
4. If daemon not found, start daemon:
   - Preferred: user-scoped service/agent installed with app.
   - Fallback: bundled sidecar/external binary launch.
5. Handshake returns: daemon version, IPC protocol version, schema epoch, sync health, active sessions summary.
6. If IPC protocol version mismatch or schema epoch mismatch between desktop and daemon, enter recovery-required mode.

**Runtime modes:**

| Mode | Local DB | Daemon | Remote Sync | UX |
|------|----------|--------|-------------|-----|
| Healthy | OK | Connected | Healthy | Full desktop |
| Remote degraded | OK | Connected | Degraded/offline | Full local ops, sync warning banner |
| Browse-only | OK | Unavailable | N/A | Read-only local browsing, clear banner |
| Recovery required | Missing/mismatch | Any | Any | Repair/update screen |

**Browse-only degraded mode (daemon down at launch):** Do not block the whole UI if local data exists. In browse-only mode: contracts list/detail works, manifests and historical transcripts load, but no reruns, no bid triggers, no live session output. Clear banner: "Daemon unavailable — read-only local snapshot."

**Remote-degraded mode (Supabase sync down):** Local-authoritative actions (rerun enrichment, start bid, source sync, session monitoring) continue working. Remote-authoritative actions (operator decisions, shared config writes) use a **local intent queue**: the desktop overlays the queued intent immediately; the viewing portal does not show it until the remote CAS succeeds after sync recovery. This preserves the authority model without freezing the operator.
Queued remote-authoritative mutations persist in local SQLite `pending_operator_intents` until the remote CAS succeeds, conflicts, or is cancelled by the operator.

**Daemon crash mid-session:**

1. Stream/heartbeat drops.
2. Desktop shows reconnect overlay.
3. Bootstrap attempts one automatic daemon restart.
4. On reconnect: invalidate all snapshot queries, reattach session streams from last `seq`, daemon reconciles orphan sessions and cleanup verdicts.
5. If restart fails: desktop falls back to browse-only mode.

**Shutdown behavior:** Closing the main window does not silently kill the daemon. Two explicit choices:

- **Quit UI** — daemon keeps running, cron schedules continue.
- **Stop background daemon and quit** — clean daemon shutdown.

If background schedules exist, default to "quit UI, keep daemon."

#### Accessibility

- Do not `aria-live` every transcript line — announce only state transitions ("session stalled", "sync degraded", "new opportunities available").
- Do not use color as the only signal for margin/risk or sync health — provide text labels and icons alongside color indicators.
- Provide a plain-text transcript mode for screen readers and copy/search alongside the ANSI terminal view.
- Keyset pagination is accessible by default (stable focus targets, no shifting content).
Deep links and bookmarked detail routes resolve through `process_aliases` before loading the canonical opportunity so previously published IDs stay navigable after a later merge.

### 5.9 Request Pipeline Automation Flow

**Important clarification:** This section describes **request pipeline automation**, not real-money bid submissions. The platform automates the preparation and submission of requests/expressions of interest. Actual negotiations and fulfillment are handled manually by humans using the pipeline output. The cost of pipeline failure is low — no real money changes hands automatically.

**Validation policy:** Despite the low financial risk, bid automation skills should default to `validation_policy: always` given their operational consequence (portal form filling, document submission). A validator checking the run manifest can catch scenarios like submitting to the wrong tender, filling forms with data from the wrong opportunity, or uploading incorrect documents — errors that are embarrassing even if not financially costly.

This automation-risk posture is formalized in **ADR-007**.

In many cases the notice source and the submission portal are different. `Find a Tender` and `Contracts Finder` are discovery/publication sources, while the actual bidder interaction often happens on downstream eSourcing portals such as Jaggaer, Atamis, Proactis, PCS-Tender, or authority-specific procurement systems. Bid workflows should therefore be modeled around the **underlying submission portal**, not the notice discovery source.

```text
operator clicks "bid" in desktop control panel
  -> create bid job in local SQLite
  -> daemon creates adapter-mediated agent session (normally PTY for portal/browser flows)
  -> portal-specific skill loaded
  -> company profile injected into kickoff prompt
  -> agent navigates to downstream portal via browser automation
  -> agent fills portal forms using skill playbook
  -> draft submission or request-for-bid prepared
  -> evidence/screenshot/log artifacts stored in local SQLite
  -> full raw session output captured for tracing
  -> job marked ready_to_bid, needs_review, or bid_submitted depending on workflow stage
  -> results synced to Supabase for visibility
  -> if failure: error manifest entry created with context for debugging
```

### 5.10 API Design

#### Transport Architecture

The daemon exposes logical API resources through multiple transport adapters:

| Transport | Purpose | Production Use |
|-----------|---------|---------------|
| **Tauri commands** (`invoke()`) | Desktop app snapshot reads + mutations | Yes — primary desktop path |
| **Tauri channels** (`Channel<T>`) | Desktop app live streams (sessions, jobs, health) | Yes — primary streaming path |
| **UDS / named-pipe RPC** | Desktop Rust bridge ↔ daemon communication | Yes — DaemonBridge transport |
| **Localhost HTTP** | CLI tooling, development, debugging | Dev/test only — not the production desktop transport |

The `/api/*` endpoint definitions below describe **logical resources** shared across all transports. The desktop app accesses them through Tauri commands and the DaemonBridge; the optional HTTP adapter exposes the same resources for dev tooling.
In this document, **daemon API** refers to these logical resources independent of transport. The optional localhost HTTP surface is only a development adapter over the same daemon API.
The surface is intentionally page-oriented and task-oriented: contracts, jobs, sessions, errors, settings, and health. It deliberately excludes generic SQL access, raw artifact-blob reads in hot paths, and arbitrary skill-execution RPCs.

#### Logical Endpoints

| Endpoint | Method | Purpose | Auth |
|----------|--------|---------|------|
| `/api/contracts` | GET | Search/browse canonical opportunities | user |
| `/api/contracts/:processId` | GET | Opportunity detail | user |
| `/api/contracts/:processId/manifest` | GET | Read generated manifest | user |
| `/api/contracts/:processId/sessions` | GET | Session-history summary for an opportunity | user |
| `/api/contracts/:processId/errors` | GET | Error-manifest summaries for an opportunity | user |
| `/api/contracts/:processId/reject` | POST | Reject an opportunity | user (desktop only) |
| `/api/contracts/:processId/watch` | POST | Move to watch state | user (desktop only) |
| `/api/contracts/:processId/bid` | POST | Trigger request pipeline workflow | user (desktop only) |
| `/api/jobs/sync-source` | POST | Trigger manual source sync | user/admin (desktop only) |
| `/api/jobs/:id` | GET | Job status/log summary | user/admin |
| `/api/sessions` | GET | Browse/filter session history | user/admin |
| `/api/sessions/:id` | GET | Session detail, structured trace summary, artifact pointers | user/admin |
| `/api/error-manifests` | GET | Browse/filter error manifests | user/admin |
| `/api/error-manifests/:id` | GET | Error-manifest detail | user/admin |
| `/api/error-manifests/:id/resolve` | POST | Mark an error manifest resolved | user/admin (desktop only) |
| `/api/daemon/health` | GET | Daemon health, sync health, runtime mode | user/admin |
| `/api/skills` | GET/PUT | Inspect available skill versions and manage source skill bindings | admin (desktop only) |
| `/api/runtime-settings` | GET/PUT | Manage local runtime settings (CLI paths, concurrency caps, local overrides) | admin (desktop only) |
| `/api/company-profile` | GET/PUT | Manage bidder company profile | admin (desktop only) |
| `/api/sources` | GET/PUT | Manage source configs | admin (desktop only) |

`/api/company-profile` is part of the target-state daemon API because request-pipeline automation needs it, but the underlying company-profile contract is only required once the bidding package enters the plan in Milestone 6. Earlier milestones can omit this surface without weakening the core intelligence-validation path.

#### Stream Surfaces (Desktop Only)

These are subscription-based endpoints consumed via the DaemonBridge → `Channel<T>` path. The viewing portal does not use these — it uses Supabase Broadcast invalidation (see §5.8 Web Portal Subscription Architecture):

| Stream | Purpose | Payload |
|--------|---------|---------|
| `stream/session.attach(sessionId, fromSeq)` | Attach to live session output | `SessionFrame` (see §5.8) |
| `stream/job.watch(jobId)` | Watch job progress | Job status + stage updates |
| `stream/daemon.health()` | Daemon health + sync status | `DaemonHealthPayload` |

#### Mutation Acknowledgment Contract

All write operations return a `MutationAck`:

```ts
type MutationAck = {
  accepted: boolean
  idempotencyKey: string
  jobId?: string
  remoteState?: 'committed' | 'queued' | 'conflicted' | 'not_applicable'
}
```

`remoteState` indicates whether the mutation has been synced to Supabase (`committed`), is pending sync (`queued`), hit a CAS conflict on a remote-authoritative table (`conflicted`), or does not involve remote state (`not_applicable`).

#### Request / Response Example

```json
POST /api/contracts/123e4567-e89b-12d3-a456-426614174000/bid
{
  "portalWorkflow": "proactis-v1",
  "notes": "Proceed with standard company profile and attach existing sourcing evidence."
}
```

```json
{
  "accepted": true,
  "idempotencyKey": "bid-123e4567-1710288000",
  "jobId": "2ac6d0ac-f7e4-47e6-9a33-d1da267f4e1c",
  "remoteState": "queued"
}
```

### 5.11 Error Handling Strategy

#### Pipeline Error Types

| Error | Code | Handling |
|-------|------|----------|
| Source website changed | `SOURCE_CHANGED` | flag skill needs update, create error manifest |
| Notice parse failed | `NOTICE_PARSE_FAILED` | store raw payload, mark manual review, create error manifest |
| Goods classification uncertain | `GOODS_CLASSIFICATION_LOW_CONFIDENCE` | keep notice, require review |
| Supplier match weak | `SUPPLIER_MATCH_LOW_CONFIDENCE` | do not auto-qualify |
| Commodity code uncertain | `COMMODITY_CODE_LOW_CONFIDENCE` | route to manual review |
| Freight estimate weak | `LOGISTICS_LOW_CONFIDENCE` | use manual quote/broker escalation |
| FPO API unavailable | `FPO_ACCESS_UNAVAILABLE` | skip FPO, use manual classification or Taric Support lookup |
| HS10 code unresolved | `HS10_UNRESOLVED` | cannot determine declarable 10-digit code; block auto-qualification, escalate to broker |
| Additional code required | `ADDITIONAL_CODE_REQUIRED` | 10-digit code requires additional 4-char codes for declaration; block auto-qualification |
| Measure condition unsatisfied | `MEASURE_CONDITION_UNSATISFIED` | trade tariff measure condition cannot be resolved; operational blocker |
| Trade remedy risk | `TRADE_REMEDY_RISK` | anti-dumping/countervailing/safeguard duty detected; flag for review, add to duty estimate |
| Revenue basis uncertain | `REVENUE_BASIS_UNCERTAIN` | framework ceiling or dynamic market; margin gate uses discounted revenue |
| Incoterm cost overlap | `INCOTERM_COST_OVERLAP` | supplier quote Incoterm includes costs also being added separately; double-counting risk |
| PVA ineligible | `PVA_INELIGIBLE` | importer cannot use postponed VAT accounting; import VAT is real cash cost |
| Freight provider unsupported | `FREIGHT_PROVIDER_UNSUPPORTED` | Freightos/SeaRates cannot quote this route/mode; use manual freight estimate |
| Freight mode low confidence | `FREIGHT_MODE_LOW_CONFIDENCE` | packaging state unknown or weight/volume too uncertain for reliable mode selection |

#### Agent-Specific Error Types

| Error | Code | Handling |
|-------|------|----------|
| Auth challenge | `AGENT_AUTH_CHALLENGE` | Agent needs re-login, MFA, browser SSO, or permission grant. Flag for operator. Desktop control panel shows alert. Distinct from stall — the agent is blocked on auth, not frozen. |
| Agent internal error | `AGENT_INTERNAL_ERROR` | API error inside Claude/Codex. Store full session artifacts. Create error manifest with trace. |
| Agent stalled | `AGENT_STALLED` | Agent stopped producing output or validated progress. Daemon detects via configurable timeout. Kill process group, create error manifest. |
| Agent skill failed | `AGENT_SKILL_FAILED` | Skill execution failed within agent (e.g., website layout changed). Store session artifacts. Create error manifest with skill path, step, and skill version. |
| Agent workflow failed | `AGENT_RUN_FAILED` | General agent failure not classifiable as a more specific type. Retry if safe; store logs/screenshots. Create error manifest. |
| Result contract invalid | `AGENT_OUTPUT_CONTRACT_INVALID` | Agent output failed schema validation against declared `result_schema_version`. Preserve raw transcript + artifact. Fail job without transcript-based recovery. |
| Protocol mismatch | `AGENT_PROTOCOL_MISMATCH` | Adapter detected unexpected CLI output format or unsupported protocol version. Enter canary mode for the vendor. Create error manifest with CLI version details. |
| Partial failure | `AGENT_PARTIAL_FAILURE` | Valid checkpoints exist before crash but no final result contract. Persist checkpoint outputs. Never upgrade to success. |
| Cleanup incomplete | `AGENT_CLEANUP_INCOMPLETE` | Descendant processes leaked after session exit. Record leaked pids/pgids. Operator may need manual intervention. |
| Suspect output | `AGENT_SUSPECT_OUTPUT` | Agent exited cleanly with valid result contract, but output failed code-based semantic or behavioral/drift validation (Output Quality Gate) OR agent-based semantic validation (validator rejection). Results quarantined, not promoted to pipeline. Surface for operator review. Error manifest indicates which gate triggered quarantine. |

#### Host/Runtime Error Types

| Error | Code | Handling |
|-------|------|----------|
| Host sleep interrupted | `HOST_SLEEP_INTERRUPTED` | Host suspended during active session. Detected via monotonic time gap. Taint browser-backed sessions. May require job re-run. |
| Host resource exhausted | `HOST_RESOURCE_EXHAUSTED` | Host memory/CPU pressure detected by resource arbiter. May preempt lower-priority sessions. Create error manifest with resource metrics. |


#### Source and Skill-Level Error Types

| Error | Code | Handling |
|-------|------|----------|
| Source soft blocked | `SOURCE_SOFT_BLOCKED` | HTTP 200 but no useful data and no explicit error page. Capture artifacts, set source cooldown, do not retry immediately. |
| Source CAPTCHA blocked | `SOURCE_CAPTCHA_BLOCKED` | Hard CAPTCHA requiring human intervention. Emit `blocked_human_action`, alert operator. |
| Auth state stale | `AUTH_STATE_STALE` | Session loads but privileged data is missing or account persona has changed. Trigger auth-refresh skill, then retry. |
| Page shape drift | `PAGE_SHAPE_DRIFT` | Page fingerprint no longer matches expected anchors. Flag skill needs review, create error manifest with old/new fingerprint hashes. |
| Locale or currency drift | `LOCALE_OR_CURRENCY_DRIFT` | Source serving different locale, currency, or market variant than expected. Fail fast, require egress/config review. |
| Resume unsafe after submit | `RESUME_UNSAFE_AFTER_SUBMIT` | Cannot safely resume a portal workflow after a potential side-effect boundary (e.g., form submission). Switch to verify-only mode or require human confirmation. |

#### Validation-Specific Error Types

| Error | Code | Handling |
|-------|------|----------|
| Validation rejected | `VALIDATION_REJECTED` | Validator agent found critical semantic issues in the run manifest. Worker results quarantined as `AGENT_SUSPECT_OUTPUT`. Error manifest created with validator's issues, reasoning, and suggested fixes. Operator alerted for review. |
| Validation flagged | `VALIDATION_FLAGGED` | Validator agent found non-critical issues in the run manifest. Worker results accepted and committed, but flagged for operator review. Issues recorded in `validation_verdicts`. |
| Validation skipped (validator error) | `VALIDATION_SKIPPED_VALIDATOR_ERROR` | Validator itself failed (crash, timeout, invalid verdict schema). This is a validator error, NOT a worker error. Worker results advance as if validation were skipped. Operator alerted about validator health. Fail-open by default. |

**Key distinction:** `AGENT_SUSPECT_OUTPUT` can now be set by either the Output Quality Gate (existing code-based checks) or a validator rejection (new agent-based semantic check). Both paths create error manifests. The error manifest includes which gate triggered the quarantine. Validator verdicts are NOT skill outcome classes — they are a separate verdict layer. A validator skill itself uses the standard outcome classes (completed/failed/etc). The verdict (approved/rejected/flagged) is the validator's assessment of the WORKER's output, carried in the result contract.

#### Skill Outcome Classes (Non-Error)

Not every non-success outcome is an error. Skills return structured outcome classes that the daemon uses for continuation and quality decisions:

| Outcome | When | Daemon Action |
|---------|------|--------------|
| `continuation_required` | Skill exited cleanly at a safe boundary with remaining work (e.g., time budget hit after 45 notices) | Accept committed results, enqueue continuation segment with resume token |
| `degraded_completed` | Skill completed but with reduced coverage or confidence (e.g., detail pages unavailable, only list-level data) | Accept results with degradation flag, apply confidence penalty to downstream calculations |
| `blocked_human_action` | Human intervention required beyond auth challenges (e.g., CAPTCHA, manual approval step, bid portal requiring human review) | Alert operator, pause job, preserve session state for potential human-assisted resume |

These are distinct from `AGENT_PARTIAL_FAILURE`. A `continuation_required` outcome has a valid final contract with continuation data. An `AGENT_PARTIAL_FAILURE` has only prior checkpoints and no final contract.

#### Error Manifest System

Every pipeline and agent failure produces:

1. an entry in the `error_manifests` table, and
2. a local Markdown artifact at `artifacts/error-manifests/YYYY/MM/<error_manifest_id>.md`.

Each error manifest contains:

- **Pipeline stage:** Which stage failed (ingestion, qualification, enrichment, landed_cost, bidding).
- **Workflow step:** The specific step within the stage.
- **Error type and message:** What went wrong.
- **Error context:** Enough information to debug — including skill path, agent session ID, relevant data.
- **Runtime context:** CLI vendor + version, adapter version, model id, config hash, skill version, pid/pgid, cleanup verdict, last valid checkpoint sequence.
- **Markdown artifact:** Human-readable header, pipeline summary, redacted context, linked local session artifact path, and next debugging action.
- **Resolution tracking:** Whether the error has been resolved, by whom, and when.

The desktop control panel provides an error manifest view where operators can:

- See all unresolved errors.
- Filter by pipeline stage, error type, and time.
- Drill into error details including session artifacts (forensic PTY capture or structured event log).
- Mark errors as resolved.

Supabase receives the error-manifest metadata plus redacted context so the viewing portal can show error status without receiving full raw session transcripts.

#### Error Handling Example

```ts
type ConfidenceGate =
  | "pass"
  | "manual_review"
  | "reject";

function resolveConfidenceGate(score: number, floor = 0.6): ConfidenceGate {
  if (score >= floor) return "pass";
  if (score >= 0.45) return "manual_review";
  return "reject";
}
```

---

## 6. Implementation Considerations

### 6.1 Complexity Analysis

**Overall Complexity:** Very High

**Complexity Breakdown:**

| Component | Complexity | Reason |
|-----------|------------|--------|
| Procurement ingestion | Medium | skill-based scraping of structured official sources; incremental retrieval well-supported |
| Notice normalization | Medium | OCDS helps, but not all sources are identical |
| Relation resolution (merge / related_not_merged / possible_duplicate / separate) | High | OCID matching is straightforward; heuristic scoring for cross-source pairs without shared OCID is harder; framework/call-off detection requires notice-type semantics |
| Goods qualification (multi-dimensional) | Medium-High | CPV heuristics cover most cases; mixed contracts with separable goods require more nuanced analysis; access route classification adds a new dimension |
| Document acquisition | Medium-High | navigating procurement portals to find and download public documents; handling varied document formats |
| Lot extraction | Medium | structured OCDS lot data is straightforward; unstructured lot descriptions in documents require AI assist |
| Product candidate identity extraction | High | noisy natural language, quantities often ambiguous; bundle decomposition; candidate typing; match archetype classification; slot criticality assignment |
| Quoteability scoring | Medium-High | deterministic rules-based; main complexity is tuning the scoring weights per category; now also computes recallRiskScore for adaptive shortlisting |
| Marketplace comparability scoring | Very High | query-wave planning per archetype × source; four-state evidence comparison; detail-page enrichment for compliance evidence; cross-language matching with market framing; category-specific attribute matching; hard-conflict detection with standards normalization; adaptive diversified shortlisting; zero-result recovery ladder with failure taxonomy; two-tier cache with structured fingerprinting |
| Commercial usability of observed prices | High | pack/unit basis normalization; MOQ compatibility; VAT/shipping inclusion detection; price-signal role classification |
| RFQ escalation strategy | Medium | identifying when indicative pricing is insufficient and executable quotes are needed |
| Landed-cost engine | Very High | logistics + customs + FX + assumptions; expanded API surface; revenue normalization |
| Operator surfaces | Medium | standard CRUD/search/detail flows; desktop control panel + read-only viewing portal |
| Request pipeline automation | High | portal-specific skills, brittle websites, but low risk since not real money |
| Daemon orchestration (PTY + SQLite + sync) | Very High | PTY session management, SQLite storage, Supabase sync engine, cron scheduling, agent health monitoring, full session-output capture |
| Skill maintenance and reliability | Medium-High | updating SKILL.md when sources change is straightforward; the added complexity is four-layer validation (syntactic/semantic/behavioral-drift via OQG, plus agent-based semantic validation via worker-validator), page fingerprint maintenance, schema migrations with dual-accept windows, promotion flow gates, golden assertion curation, and auth-refresh skill coordination |
| Worker-validator semantic validation | Medium-High | Run manifest generation is straightforward (agent writes a structured narrative as part of its output). The validation policy engine is config-driven. The hard part is writing good validator skills that catch real semantic issues without excessive false positives. Validator skill quality improves iteratively via the same promotion flow as worker skills. |

**Architecture-shape note:** The component graph is intentionally denser around PTY/runtime orchestration and product/pricing comparability than around generic CRUD or projection concerns. That is deliberate: those two areas dominate failure risk, operational cost, and correctness.

**Delivery-priority annotation:** Components cluster into three delivery tiers based on risk retirement:

- **MVE-critical (validate first):** Product candidate extraction (High), Marketplace comparability scoring (Very High), Commercial usability of observed prices (High), Lot extraction (Medium), Document acquisition (Medium-High), Quoteability scoring (Medium-High — gates which candidates enter marketplace search), Landed-cost engine screening-grade subset (Medium — needed to produce margin bands for validation). These components determine whether the product thesis is viable. Without them, nothing downstream produces real signal.
- **MVP-critical (build second):** Procurement ingestion (Medium), Notice normalization (Medium), Goods qualification (Medium-High), Landed-cost engine full scope (Very High), Operator surfaces (Medium).
- **Operational (build third):** Daemon orchestration (Very High at full scope, but a thin-runner subset is Medium), Relation resolution (High), Skill maintenance (Medium), Worker-validator semantic validation (Medium-High — dry-run mode is Operational tier; enforcement is later), Request pipeline automation (High), RFQ escalation (Medium).

This ordering means the highest-complexity components (marketplace comparability, landed-cost, daemon orchestration) are addressed in phases: screening-grade subsets first for validation, full complexity later for production.

**Risk Areas:**

1. Document/attachment acquisition — without tender documents, product extraction operates on vague summaries and produces garbage candidates. This is the biggest silent failure mode.
2. Product matching between contract descriptions and marketplace listings — now decomposed into identity extraction, comparability scoring, and price-signal usability.
3. Lot blindness — without lot structure, mixed contracts and framework notices contaminate each other.
4. Commodity-code accuracy and duty/VAT correctness.
5. Freight-mode selection and cost assumptions for large contracts.
6. PTY session reliability and agent health monitoring.
7. Skill drift when source websites change — mitigated by page fingerprint monitoring, production drift detection, source-health canaries, and the error manifest system.
8. SQLite-to-Supabase sync consistency.

### 6.2 Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Wrong product match inflates margin | High | High | dual confidence scoring with lexicographic gating + archetype-specific floors + four-state evidence comparison + compliance evidence tiers + operator review |
| Incorrect notice-to-process merging conflates separate opportunities | Medium | High | conservative merge heuristics + merge confidence + manual unmerge tooling |
| HS code is wrong | Medium | Critical | HMRC FPO first-pass + candidate-code ranking + ATR workflow + broker escalation |
| Freight estimate too optimistic | High | Critical | mode-aware engine + Freightos + SeaRates cross-check + manual quote threshold |
| VAT recoverability is modeled incorrectly | Medium | High | store VAT-registration assumptions explicitly and show economic-margin vs cash-outlay views separately |
| Daemon not running for queued work | Medium | Medium | local cron + error manifest flags missed schedules + at least one designated always-on machine |
| Source website layout changes break scraping skills | High | Medium | error manifest flags failures + skill versioning + fast skill update cycle |
| Agent auth challenge blocks a run | High | Medium | daemon health monitoring detects `AGENT_AUTH_CHALLENGE` + operator alert |
| Agent stalls mid-execution | Medium | Medium | daemon timeout detection + auto-kill + error manifest |
| SQLite-to-Supabase sync protocol drift / schema mismatch | Low | Medium | `schema_epoch` guard + transactional outbox ordering + sync round-trip tests in CI |
| Portal UI changes break request pipeline skills | High | Low | portal-specific skills + screenshots + health checks. **Impact downgraded:** this automates the request pipeline, not real-money bids |
| FX volatility changes decision outcome | High | Medium | stored rate snapshots + HMRC official rates for customs + recalculation triggers |
| Attachment blindness — extraction from notice text only | High | Critical | document acquisition stage before product extraction; confidence penalty when no docs available |
| Lot blindness — no lot structure in data model | High | High | NoticeLot entity; synthetic lot for notices without explicit lots; per-lot qualification |
| False bidability on framework call-offs | High | Medium | access route classification early in pipeline; reject closed/non-bidable routes before pricing |
| Compliance-critical false positive (regulatory goods matched without standards verification) | Medium | High | regulated_goods candidate type + complianceEvidenceTier (structured_attribute / title_body_mention / absent) + detail-page enrichment before rejection + standard normalization (family/part/edition/amendment/class) |
| Pack-basis normalization error (comparing per-unit vs per-box vs per-pallet) | High | High | listing pack qty/unit extraction; normalized unit basis on every quote; quoteUsabilityConfidence penalizes unclear basis |
| Price-signal false precision (single marketplace listing treated as decision-grade pricing) | High | Medium | margin bands not point estimates; priceSignalRole classification; RFQ escalation for high-value opportunities |
| HMRC FPO API access not secured (restricted to invited UK Carrier scheme organisations) | Medium | High | do not make FPO a hard dependency; fallback to CPV-to-HS mapping heuristics + manual classification |
| Missing evidence treated as conflicting evidence in hard-conflict filter | High | High | four-state evidence model (match/missing/ambiguous/conflict); only `conflict` triggers hard rejection; `missing` and `ambiguous` are confidence penalties |
| Static shortlist truncation drops valid matches in noisy categories | High | Medium | adaptive shortlist budget driven by `recallRiskScore`; diversity enforcement (max 2 per seller, max 2 per wave, max 1 per title family); recall@k measurement per archetype × source |
| Query vocabulary mismatch between procurement and marketplace terminology | High | Medium | `matchArchetype`-driven query templates; market framing slots; noun alias families; query-wave pattern with staged recovery; stored `searchFailureMode` for vocabulary gap detection |
| Zero-result classified as no marketplace presence when query is bad | Medium | High | formal recovery ladder with 6 rounds; broad existence check distinguishes vocabulary gap from genuine absence; `searchFailureMode` taxonomy |
| Slot criticality too flat — nice-to-have attributes treated as hard requirements | High | Medium | `slotCriticality` roles (required/preferred/descriptive/exclusion) on each extracted attribute; archetype-specific defaults |
| Cross-language market framing gap — literal slot translation misses marketplace vocabulary | Medium | Medium | `marketFrameEn`/`marketFrameZh` slots; curated lexicon with industry framing; Chinese retry on Alibaba after English failure |
| Accessory contamination in search results dominates shortlist | High | Medium | aggressive negative vocabulary per product family; diversity enforcement limits per-seller and per-title-family; `accessory_swamp` failure mode classification |
| Infrastructure-first delivery delays thesis validation | High | High | validate intelligence pipeline (extraction, matching, margin accuracy) before investing in daemon hardening, cron, sync, and web — wrong answers at scale are worse than no automation |
| Premature category breadth before depth | High | High | start with a narrow goods-category whitelist (commodity-like, low regulatory burden); expand only after the system proves reliable on the initial wedge |
| No labeled benchmark corpus for calibration | High | Medium | curate a gold-set lot corpus with human labels before building confidence scoring; without it every demo is a cherry-picked success story |
| Schema-valid garbage from skills (silent wrong-page or wrong-locale extraction) | High | High | four-layer validation: syntactic/semantic/behavioral-drift (OQG, code-based) PLUS agent-based semantic validation via worker-validator system; page fingerprint checks before extraction; locale/currency/persona assertions at run start; `AGENT_SUSPECT_OUTPUT` quarantine path; run manifests enable human and validator review of what the agent actually did |
| Anti-bot escalation burns accounts or IP addresses | Medium | High | anti-bot classification table with proportional responses (no blind retry); source-health canaries before expensive jobs; source cooldown on challenge detection; dedicated auth-refresh skills isolate login complexity |
| Skill promotion regression (new version faster but dumber) | Medium | Medium | mandatory shadow comparison before promotion; shadow metrics must meet or exceed stable version; fill-rate, duplicate-rate, suspect-output-rate, and cost-per-committed-item all gated; validator approval rate is an additional promotion gate |
| Validator produces false rejections, blocking good data | Medium | Medium | dry-run mode first (log verdicts without enforcement); track validator approval/rejection rates per skill; operator override for rejected results; false positive rate monitored via operator override count |
| Validator itself fails or is unreliable | Medium | Low | validator skills follow same promotion flow as worker skills; validator failures are fail-open (don't block worker results); validator health monitored separately |
| Run manifest quality too low for meaningful validation | Medium | Medium | run manifest is part of the skill contract — skill promotion requires run manifests that validators can meaningfully review; manifest completeness checked by base validator |
| Validation cost overhead per job | Low | Low | validator skills use headless_contract mode with cheap models (~$0.01–0.03 per validation); validation policy can sample instead of always-validate; even "always validate" adds only 5–15% overhead |

### 6.3 Security Considerations

**Authentication:**

- Viewing-portal users authenticate through Supabase Auth or compatible OIDC (read-only access).
- Desktop operators authenticate locally; the daemon uses a device-scoped token for Supabase sync.
- Local SQLite requires no authentication (protected by OS-level file permissions).
- Agent CLIs use locally managed credentials, never browser-exposed secrets.

**Authorization:**

- RLS for all user-facing tables in Supabase — push-owned projection tables, remote-owned shared tables, and any portal-facing views.
- Do not distribute the Supabase service-role key to desktop clients.
- Prefer a dedicated `daemon` role exposed through RLS-safe RPCs or a thin hosted broker API for privileged operations. Push batches should be applied through a single transactional RPC, not direct table upserts from the daemon client.
- Distinct roles:
  - `viewer` — read-only access to projection tables via viewing portal
  - `operator` — read access to projections + write access to `operator_decision_heads` (via compare-and-swap RPC)
  - `admin` — full access to shared config tables
  - `daemon` — push-only access to projection tables via batch RPC, pull-only access to remote-owned tables

**RLS policy specifications (Supabase/Postgres):**

All tables scoped by `org_id` (implicit — single-org MVP, explicit column added when multi-org is needed). Policies below use Supabase `auth.role()` and `auth.uid()`.

| Table Category | Tables | viewer | operator | admin | daemon |
|---|---|---|---|---|---|
| Push-owned projections | `procurement_processes`, `contract_notices`, `notice_lots`, `qualification_runs`, `product_candidates`, `search_profiles`, `search_attempts`, `supplier_quotes`, `landed_cost_estimates`, `manifests`, `process_relations`, `process_aliases`, `pipeline_assessments`, `bid_workflow_states`, `process_stage_status`, `process_stage_heads` | SELECT | SELECT | SELECT | INSERT, UPDATE (via batch RPC) |
| Push-owned summaries | `workflow_job_summaries`, `error_manifest_summaries`, `viewer_sync_status`, `executors` | SELECT | SELECT | SELECT | INSERT, UPDATE (via batch RPC) |
| Remote-owned shared config | `shared_company_profile`, `shared_source_configs` | SELECT | SELECT | SELECT, INSERT, UPDATE, DELETE | SELECT (pull) |
| Remote-owned decisions | `operator_decision_heads` | SELECT | SELECT, UPDATE (via CAS RPC) | SELECT, INSERT, UPDATE, DELETE | SELECT (pull) |
| Sync infrastructure | `schema_metadata` | — | — | SELECT, UPDATE | SELECT, UPDATE |

**Implementation notes:**
- Push-owned tables: daemon writes through a single `apply_push_batch(batch_json)` RPC that validates `schema_epoch`, checks `row_version` ordering, and applies atomically. RLS on the underlying tables grants `INSERT`/`UPDATE` only to `daemon` role.
- Operator decisions: `update_operator_decision(process_id, disposition, base_revision)` RPC enforces compare-and-swap semantics. Direct table `UPDATE` is blocked; only the RPC path is exposed.
- All `SELECT` policies include `WHERE deleted_at IS NULL` for tombstoned tables to exclude soft-deleted rows from portal views.
- `viewer` and `operator` roles cannot see sync infrastructure tables (`local_outbox`, `sync_batches`, `pull_cursors` are SQLite-only; `schema_metadata` is admin/daemon only).

**Sync Security:**

- Every push batch carries `schema_epoch` — remote rejects batches with incompatible epochs.
- Push batches carry `content_hash` per row — replay detection prevents duplicate application.
- Outbox integrity: business writes and outbox entries are in the same SQLite transaction — crashes cannot create orphaned sync state.
- Remote-owned writes (operator decisions) use compare-and-swap with `remote_revision` — prevents silent overwrites of human commercial decisions.
- Remote config must not overwrite machine-local secrets or runtime overrides. Split: remote = org-wide config; local = keychain refs, cookies, CLI paths, concurrency caps, temporary source disable flags.
- Snapshot config revision into each job at enqueue time, so long-running jobs are reproducible even if config changes mid-run.

**Data Protection:**

- Store secrets in desktop OS keychain / Tauri secure storage strategy, not plain-text configs. Local secrets and runtime overrides must never be synced to Supabase.
- Local SQLite file should have restrictive file permissions (0600).
- Encrypt especially sensitive company profile fields if stored centrally in Supabase.
- Redact agent logs before syncing to Supabase — full raw session output may contain sensitive data. Supabase receives only redacted excerpts or null for `terminal_output` and `structured_trace`.
- Agent session-output artifacts stored locally should be access-controlled.

**Security Risks:**

| Risk | Mitigation |
|------|------------|
| Prompt injection via source content | strict extraction schemas + sandboxed agent context |
| Secret leakage to agent logs | scoped env vars + redaction + non-browser secret handling |
| Overpowered Tauri shell execution | explicit Tauri plugin permissions and sidecar allowlists |
| Cross-tenant data exposure | RLS + broker/service-tier isolation + organization scoping |
| Unsafe CLI execution | launch via bounded commands, fixed working dirs, strict envs |

### 6.4 Performance Considerations

**Performance Targets:**

- Incremental ingestion per source should finish in minutes, not hours.
- Opportunity detail page should load in under 2 seconds from warm cache.
- Rerunning enrichment for one contract should begin immediately.
- Job queue should remain responsive even when long agent tasks are running.

**Optimization Strategies:**

- Poll sources incrementally with cursors (stored in local SQLite).
- Keep raw payloads and normalized projections separate.
- Use async worker queues for heavy enrichment.
- Cache FX and tariff lookups for short TTL windows.
- Local SQLite: enable WAL mode for concurrent reads during writes; use appropriate indexes.
- Supabase: precompute viewing-portal summary tables/materialized views for the viewing portal.

**SQLite Performance Characteristics:**

- Reads are very fast (local disk, no network latency).
- Single-writer model is sufficient for single-daemon architecture.
- WAL mode allows concurrent reads while writing, but long-lived readers can prevent checkpoints from fully draining the WAL. Keep desktop read transactions short.
- Database size should stay manageable (procurement data is relatively small volume).
- **Storage split (design decision):** Store large PTY transcripts and raw document payloads in a separate `artifacts.db` or `artifacts/` directory. The hot `state.db` should contain only relational state, jobs, manifests, and the sync outbox. This prevents checkpoint and vacuum overhead from large blob writes.

**SQLite Operational Guidance:**

- Keep SQLite databases on real local disk only — never NFS, cloud-sync folders (Dropbox, iCloud), or network-mounted storage. WAL requires same-host shared memory.
- Run `PRAGMA quick_check` on startup to detect corruption early. Run `PRAGMA integrity_check` on maintenance windows.
- Run `PRAGMA optimize` on a maintenance cadence (recommended over manually scheduling `ANALYZE`).
- Monitor WAL size, checkpoint latency, and SQLITE_BUSY counts.
- Use the Online Backup API or `VACUUM INTO` for safe hot backups. Never take backups by copying the main DB file while the process is live — missing WAL/journal sidecars can cause corruption.
- Store `schema_epoch` in `PRAGMA user_version` for fast compatibility checks.
- Use STRICT tables where json1 is not required for additional type safety.

**Supabase Performance Guidance:**

- Precompute viewing-portal summary tables/materialized views for the viewing portal.
- Do not turn on Postgres Changes subscriptions for every hot table. For live viewing-portal updates, prefer small projection-change or status broadcast events (Supabase Broadcast) over raw row-by-row Postgres Changes streams.
- Do not have the viewing portal query all raw sync tables directly — provide dedicated projection tables keyed to portal use-cases.

**Potential Bottlenecks:**

| Bottleneck | Mitigation |
|------------|------------|
| Supplier search explosion | limit search permutations, use line-item heuristics |
| Agent runtime saturation on user machine | daemon concurrency caps on PTY-backed browser sessions and total adapter-mediated sessions |
| Large manifest generation | generate async and cache artifacts |
| Control-panel and portal search over large JSON payloads | normalized columns + indexes (SQLite for desktop, GIN + FTS for Supabase/web) |
| SQLite-to-Supabase sync volume | outbox-based batch sync, push only changed records via local_seq ordering |
| Terminal output storage growth | store in separate artifacts DB, compress old logs, rotate after configurable retention |
| WAL file growth | monitor checkpoint latency, keep desktop reads short, periodic checkpoint |
| Tombstone accumulation | GC only after all replicas have pulled past delete seq + offline window (37 days default) |
| Session stream UI freezing | batch frames 33–50ms, external stores outside React state, gap frames on backpressure |
| List pagination instability | keyset/cursor pagination, not offset — offset causes duplicates and jumps with live updates |

**Frontend Performance Guidance:**

- **Desktop search:** Use SQLite FTS5 for full-text search on contracts, manifests, and error manifests. Do not load thousands of rows into React and filter client-side.
- **Web search:** Use Postgres text search with indexed projection columns and GIN indexes. Dedicated search-optimized columns in projection tables.
- **Session streaming budget:** Terminal viewer batches frames at 33–50ms intervals. Live buffer retains only the last N lines / M MB. History mode lazy-loads from artifact chunks. Never put raw transcript arrays in React state.
- **Snapshot query budget:** Desktop `QueryEnvelope<T>` queries should complete in under 50ms from warm SQLite (page-specific read models, not generic table scans). Transcript/blob reads are excluded from snapshot queries and loaded on demand.
- **Broadcast payload budget:** Web portal invalidation payloads are small hints (process IDs + change categories), never full rows or manifests. One coalesced notification per topic per push batch.

### 6.5 Testing Strategy

**Core principle:** Prioritize **real end-to-end tests** over mocks and stubs. Real E2E agent tests are the ultimate success criterion — they determine whether the pipeline actually works. Avoid creating mocks or stub test functionality.

For hybrid-runtime coverage, "real E2E" means exercising both PTY-backed sessions and contract-mode sessions against real agent CLIs where the workflow supports each mode.

**Test Priority (highest to lowest):**

| Priority | Type | Coverage Target | Tools |
|----------|------|-----------------|-------|
| 1 | **Agent E2E** | Real agent sessions (PTY and contract mode) with sample prompts and skills | cargo test + real agent CLIs |
| 2 | **Integration** | Real database + real agent processes | SQLite + Supabase local stack |
| 3 | **Unit** | Deterministic cost/qualification logic | Vitest, cargo test |
| 4 | **Browser E2E** | Key operator control-panel and viewing-portal flows | Playwright |

**Agent E2E Testing (PRIMARY — determines success):**

The most critical tests verify that the full agent pipeline works:

1. Send real kickoff prompts to real agent sessions (PTY-backed or contract-mode as appropriate for the skill).
2. Load actual skills into the agent session.
3. Verify the agent is working, outputting, and proceeding with its tasks.
4. Verify structured data output from the agent.
5. Verify the daemon correctly captures raw session output and validated protocol events.
6. Verify error detection (stalls, failures, expired sessions).

A real agent E2E test fails if any of these conditions are not met within the test budget: raw session output begins, structured result output is produced, expected job state is reached, and the correct health/error classification is recorded.

```rust
#[tokio::test]
async fn agent_procurement_ingestion_produces_notices() {
    // Create a real PTY session
    let session = DaemonPty::spawn_agent(
        "skills/procurement-ingestion/fts/SKILL.md",
        "Ingest the latest 5 notices from Find a Tender"
    ).await;

    // Verify agent is producing output
    let output = session.wait_for_output(Duration::from_secs(60)).await;
    assert!(!output.is_empty(), "Agent should produce terminal output");

    // Verify structured data was captured
    let notices = session.wait_for_completion().await.unwrap();
    assert!(!notices.is_empty(), "Agent should extract at least one notice");

    // Verify terminal output was captured for tracing
    let trace = session.full_terminal_output();
    assert!(trace.contains("notice"), "Terminal output should show agent working on notices");
}

#[tokio::test]
async fn agent_stall_is_detected() {
    let session = DaemonPty::spawn_agent(
        "skills/test-stall/SKILL.md",
        "This skill intentionally stalls"
    ).await;

    let result = session.wait_for_completion_with_timeout(Duration::from_secs(30)).await;
    assert!(matches!(result, Err(AgentError::Stalled)));
}
```

**Integration Testing (no mocks):**

All integration tests use real databases and real processes:

- Real SQLite databases for daemon storage tests.
- Real Supabase local stack for sync tests.
- Real agent processes for skill execution tests.
- No recorded fixtures, no schema assertions against mocked responses.

**Unit Testing (deterministic logic only):**

Unit tests are reserved for purely deterministic logic that does not depend on external systems:

```ts
describe("net margin gating", () => {
  it("qualifies only when net margin is at least 20%", () => {
    const estimate = {
      revenueGbpExVatPence: 100_000,
      economicLandedCostGbpPence: 78_000,
    };

    const netMargin =
      (estimate.revenueGbpExVatPence - estimate.economicLandedCostGbpPence) / estimate.revenueGbpExVatPence;
    expect(netMargin).toBeGreaterThanOrEqual(0.2);
  });
});
```

```rust
#[test]
fn expires_within_30_days_is_rejected() {
    let days_until_expiry = 14_i64;
    assert!(days_until_expiry < 30);
}
```

**Deterministic Replay Testing (parser and health-monitor regression):**

Real vendor E2E remains the acceptance gate, but captured real PTY/JSON sessions are replayed for regression coverage:

- Record real PTY byte streams and contract-mode JSONL from passing E2E runs.
- Replay captured sessions against the adapter parser, health monitor, and result contract validator.
- Detect regressions when parser/validator code changes without needing live agent CLIs.

**Fault-Injection Testing:**

Test daemon resilience against failure modes that real E2E tests rarely trigger:

| Fault | Test Coverage |
|-------|---------------|
| Dropped child handle | Verify orphan sweep detects and reaps leaked processes |
| Orphaned grandchildren | Verify process-group kill reaches descendants spawned by agent |
| PTY EOF/EIO handling | Verify daemon treats EIO after slave close as normal termination |
| Host sleep / monotonic gap | Verify `HOST_SLEEP_INTERRUPTED` classification when time jumps |
| Partial checkpoint then crash | Verify `AGENT_PARTIAL_FAILURE` — valid checkpoints preserved, job not upgraded to success |
| Auth challenge mid-run | Verify `AGENT_AUTH_CHALLENGE` classification distinct from `AGENT_STALLED` |
| Cleanup timeout after main pid exit | Verify `AGENT_CLEANUP_INCOMPLETE` when descendants survive group kill |
| Concurrent session resource pressure | Verify resource arbiter correctly preempts lower-priority sessions |
| Unclassifiable agent exit | Verify `AGENT_RUN_FAILED` classification when exit code is non-zero but no specific error pattern matches |

**Adapter Contract Testing:**

Each vendor adapter is tested for:

- Capability probe correctness (CLI version detection, auth health check, structured output viability).
- Mode selection logic (PTY vs. contract based on skill `interaction_profile` and probe results).
- Canary mode behavior (unknown CLI versions blocked from production until probe passes).
- Structured output parsing (vendor JSONL/JSON parsed into daemon-native events without loss).


#### Skill-Specific Testing

These tests operationalize the production skill-directory standard, checkpoint rules, and outcome classes defined in §4.10.

**Skill Linting (CI gate — required on every skill change):**

Cheap, deterministic, runs on every commit that touches a skill directory:

- Required metadata present (interaction_profile, result_schema_version, model_profile, etc.)
- Trigger boundary sections present in SKILL.md (use when, do not use when, inputs, outputs, non-goals)
- `result.schema.json` present and valid JSON Schema
- `checkpoint.schema.json` present and valid JSON Schema
- Failure codes enumerated in SKILL.md
- Resume strategy declared
- For portal/bid skills: unsafe side-effect boundary declared
- Bounded pagination/scroll rules exist (maxPages, maxScrollCycles, etc.)
- `canary.inputs.jsonl` exists and is non-empty
- `golden.assertions.yaml` exists

**Live Smoke Canaries (per critical source, frequent):**

Cheap, narrow, per-source validation runs:

- Page fingerprint still matches expected anchors
- Auth still works (if authenticated source)
- One small extraction completes (e.g., first page of results, single known notice)
- Result shape + semantic invariants hold
- Expected locale/currency/persona confirmed

Run schedule: on every skill change (before promotion gate), before promotion from candidate → canary, on a recurring schedule during production hours (catch source-side changes between skill updates).

**Invariant-Based Golden Live Journeys:**

Real E2E tests pinned to known stable tasks, using **invariant assertions** (not brittle literal output matching):

| Skill Family | Golden Journey | Invariant Assertions |
|-------------|----------------|---------------------|
| Procurement ingestion (FTS) | Known OCID or fixed recent time window | Minimum result count, required fields present, locale correct, duplicate rate below threshold |
| Supplier enrichment (eBay/Amazon) | Stable commodity search (e.g., "A4 paper 500 sheets") | Results found, currency GBP, at least one price extracted, page fingerprint correct |
| Supplier enrichment (Alibaba/1688) | Known logged-in search + one detail extraction | Auth verified, results found, MOQ extracted, at least one artifact type acquired |
| Document acquisition | Public tender with known PDF + spreadsheet | Documents downloaded, content-type correct, file size non-zero, checksum stable |
| Bid portal | Sandbox tenant or stop-at-draft-save path | Draft saved, no submission triggered, form fields populated, screenshot captured |

**Shadow Runs (promotion gate):**

When skill version `vNext` is a candidate for promotion:

1. Stable version continues to write authoritative results.
2. `vNext` runs in shadow on the same inputs.
3. Compare: fill-rate, duplicate rate, suspect-output rate, challenge rate, result count, cost per committed item.
4. Promote only after shadow metrics meet or exceed stable version across a minimum sample window.
5. If shadow is materially worse on any dimension, block promotion and flag for investigation.

Shadow runs catch "new skill is faster but dumber" — the most common regression pattern.

**Production Drift Monitoring:**

Every production run emits a quality scorecard:

| Metric | What It Detects |
|--------|----------------|
| Page fingerprint hash | Source layout changes |
| Locator fallback depth | DOM structure changes (tier 1 → tier 3 = degradation) |
| Challenge count | Anti-bot escalation |
| Auth-refresh count | Auth stability regression |
| Fill-rate | Extraction completeness changes |
| Duplicate rate | Pagination/dedup problems |
| Median result count | Source yield changes |
| Time to first checkpoint | Startup performance regression |
| Cost per committed item | Efficiency regression |
| Suspect-output rate | Silent quality degradation |

Treat "flaky" as a failing skill health signal, not a minor annoyance. Skills with intermittent failures are skills with unhandled failure modes.

**Validator Testing:**

| Test Type | What It Covers |
|-----------|---------------|
| Validator canary tests | Feed known-good and known-bad run manifests to validator skills, verify correct verdicts (approved for good, rejected for bad) |
| Validator golden journeys | Run full worker → validator pipeline on gold-set inputs; verify end-to-end verdict correctness |
| Dry-run validation tests | Verify that dry-run mode logs verdicts without enforcement — rejected results still advance in dry-run |
| False positive monitoring | Track validator rejection rate against operator overrides; high override rate signals validator is too strict and needs tuning |
| Validator regression tests | When validator skill is updated, replay historical run manifests; verify verdicts don't regress |
| Calibration tests | Compare validator verdicts against human judgments on a curated set; track agreement rate as validator quality metric |

**Browser Debugging Artifacts:**

For canary and golden journey tests, retain: Playwright trace (or equivalent), screenshot at each navigation step, video on retry or failure runs (not routine passes), JSON reporter output with timing and assertion results, artifact links in the failure record for drill-down.

### 6.6 Operational Observability

Comprehensive agent tracing and health monitoring is **critical infrastructure**, not a nice-to-have. The entire pipeline runs through agent sessions, so observability of those sessions is essential for debugging and reliability.

#### Four-Layer Observability Model

Observability is structured into four distinct layers, each with a clear role:

**Layer 1: Control Events (daemon-generated)**

Events the daemon itself produces about session lifecycle:

- `session_spawned` (pid, pgid, adapter, mode, skill_id, skill_version, cli_version, config_hash)
- `session_active` (first output/event received)
- `signal_sent` (signal type, target pid/pgid)
- `session_exited` (exit code, exit signal)
- `session_reaped` (reap method, descendants found)
- `cleanup_verdict` (`clean`, `forced`, `leaked`)
- `host_event` (sleep_detected, resource_pressure)

**Layer 2: Protocol Events (adapter-generated)**

Events parsed from vendor-specific structured output by the adapter:

- Vendor JSONL events (Codex: `turn.started`, `turn.completed`, `item.*`, `error`)
- Vendor JSON/stream-json output (Claude headless structured output)
- Validated checkpoints (schema version, job id, skill id, sequence, status, outputs)
- Auth challenges detected from structured signals
- Final result contract validated

**Layer 3: Forensic Bytes (immutable raw capture)**

Raw session-output bytes captured in both execution modes. In PTY mode this includes:

- Agent thinking/reasoning text.
- Tool calls and their results.
- Browser navigation steps.
- Data extraction outputs.
- Error messages and stack traces.
- ANSI escape sequences (stripped for derived summaries, preserved in raw artifact for replay).

In contract mode, Layer 3 additionally includes the raw stdout/stderr stream and any vendor-emitted JSONL/JSON framing before adapter normalization.

**Layer 4: Narrative (agent-generated run manifest)**

A structured Markdown document produced by the agent at the end of every skill run, narrating what the agent did, what decisions it made, what warnings it encountered, and what evidence supports the results. Stored as an immutable artifact at `artifacts/sessions/YYYY/MM/<session_id>/run-manifest.md`. Layer 4 fills a gap the other layers cannot: Layer 1 knows the session started/ended but not what the agent chose to do; Layer 2 knows data was extracted but not whether the agent navigated correctly; Layer 3 has every byte but is too large and noisy for automated semantic review. Layer 4 is the purposeful, structured summary designed for both human and validator-agent consumption. See §5.2 Run Manifest Writer for format and §5.2 Validation Executor for how the validator consumes it.

**Critical rule:** Only Layer 1 and Layer 2 drive workflow state. Layer 3 is for post-hoc debugging and replay. Layer 4 informs the validator's semantic judgment, which influences state indirectly (via verdicts), but the run manifest itself never drives state directly. A transcript can explain a failure; it must never upgrade a job to success.

#### Artifact Storage Model

Raw session-output bytes and large session outputs are stored as immutable compressed artifact files, **not inline in hot SQLite tables**:

- **Raw artifacts:** Chunked, compressed, immutable files at `artifacts/sessions/YYYY/MM/<session_id>/`. PTY mode stores raw terminal bytes; contract mode stores raw stdout/stderr bytes alongside parsed protocol events. Rotate capture chunks every 8 MiB and compress immediately. Preserve full raw capture by default; if a single session exceeds 256 MiB of raw bytes, stop capture only after writing an explicit truncation marker and surfacing an operator warning.
- **SQLite stores:** Session header, byte counts, last 64 KB tail excerpt, redaction markers, artifact path, SHA-256 hash, and derived control/protocol event summaries.
- **Supabase receives:** Redacted summary, event counts, and status only — no raw transcripts.

This preserves full session-output capture as critical infrastructure without turning the operational database into a log sink.

#### Agent Health Detection

The daemon actively monitors every running session for:

| Health Signal | Detection Method | Response |
|---------------|-----------------|----------|
| **No output bytes** | Configurable timeout since last PTY byte or stdout/stderr byte | Mark `AGENT_STALLED`, kill process group, create error manifest |
| **No validated progress** | No checkpoint written within progress timeout (both modes) | Mark `AGENT_STALLED`, kill process group, create error manifest |
| **Auth challenge** | Pattern match on raw output stream or structured event for login/MFA/SSO/permission prompts | Mark `AGENT_AUTH_CHALLENGE`, alert operator via desktop control panel |
| **API error inside agent** | Adapter parses error from structured events or terminal patterns | Mark `AGENT_INTERNAL_ERROR`, capture full context |
| **Skill execution failed** | Agent exits with error or produces error output | Mark `AGENT_SKILL_FAILED`, capture skill path, step, and skill version |
| **Result contract invalid** | Final output fails schema validation | Mark `AGENT_OUTPUT_CONTRACT_INVALID`, preserve artifacts |
| **Host sleep detected** | Monotonic time gap exceeds threshold | Mark `HOST_SLEEP_INTERRUPTED`, taint browser-backed sessions |
| **Host resource pressure** | Resource arbiter detects memory/CPU pressure exceeding threshold | Mark `HOST_RESOURCE_EXHAUSTED`, preempt lower-priority sessions |
| **Protocol mismatch** | Adapter detects unexpected CLI output format or unsupported protocol version during parsing | Mark `AGENT_PROTOCOL_MISMATCH`, enter canary mode for vendor, create error manifest |
| **General run failure** | Agent exits with non-zero code, not classifiable as a more specific error type | Mark `AGENT_RUN_FAILED`, retry if safe, create error manifest |
| **Normal completion** | Valid final result contract + clean exit + complete reap | Mark completed, archive session |

#### Structured Tracing

Structured traces are derived primarily from Layer 1 (control) and Layer 2 (protocol) events, stored in `agent_session_logs.structured_trace`:

```json
{
  "layers": {
    "control": [
      {"type": "session_spawned", "pid": 12345, "pgid": 12345, "mode": "pty", "timestamp": "..."},
      {"type": "session_active", "timestamp": "..."},
      {"type": "session_exited", "exit_code": 0, "timestamp": "..."},
      {"type": "cleanup_verdict", "verdict": "clean", "timestamp": "..."}
    ],
    "protocol": [
      {"type": "navigate", "url": "https://www.find-tender.service.gov.uk/...", "timestamp": "..."},
      {"type": "extract", "count": 12, "entity": "notices", "timestamp": "..."},
      {"type": "checkpoint", "sequence": 1, "status": "partial", "timestamp": "..."},
      {"type": "final_result", "schema_version": "1.0", "status": "success", "timestamp": "..."}
    ]
  },
  "forensic_ref": "artifacts/sessions/2026/03/abc123/raw.zst"
}
```

Transcript-derived enrichment (best-effort parsing of raw PTY output for tool calls, browser steps, etc.) may supplement the structured trace but does not override control/protocol events.

#### Per-Session Metrics

From day one, record per session:

- pid / pgid,
- CLI vendor + version,
- adapter version,
- config hash,
- skill id + skill version,
- session duration,
- bytes captured (raw artifact size),
- checkpoint count,
- cleanup latency (time from exit to reap),
- cleanup verdict,
- auth challenge count,
- stall phase (which timeout triggered, if any),
- redaction count (sensitive data markers),
- peak resource estimate (if available from resource arbiter).

#### Skill-Level Telemetry

In addition to per-session metrics, record skill-specific operational telemetry from day one:

| Metric | Source | What It Reveals |
|--------|--------|----------------|
| Page fingerprint hash | Skill extraction phase | Source layout stability — sudden changes signal skill maintenance needed |
| Locator fallback depth | Skill navigation phase | DOM stability — rising fallback depth predicts skill breakage |
| Challenge count | Skill navigation phase | Anti-bot escalation — rising count signals account/IP risk |
| Auth-refresh count | Auth-refresh skills | Auth stability — rising count signals credential or session management issues |
| Time to first checkpoint | Skill execution timeline | Startup reliability — increasing time signals source or auth degradation |
| Checkpoint interval | Skill execution timeline | Throughput stability — widening intervals signal source throttling or extraction difficulty |
| Fill-rate | Skill extraction validation | Extraction completeness — dropping fill-rate signals silent breakage |
| Duplicate rate | Skill extraction validation | Dedup health — rising duplicates signal pagination loops |
| Cost per committed item | Post-run calculation | Efficiency trend — cost spikes signal skill regression or source changes |
| Suspect-output rate | Output quality gate | Silent quality degradation — non-zero rate requires investigation |
| Continuation-required rate | Outcome classification | Segmentation frequency — rising rate may signal time budget too tight or source too slow |
| Overlay handler invocation count | Skill navigation phase | Source behavioral changes — rising count signals new overlays or anti-bot patterns |
| Degraded-completed rate | Outcome classification | Partial success frequency — high rate signals source access problems |
| Validation approval rate | Validation executor | Validator quality — low approval rate signals overly strict validators or degrading worker quality |
| Validation rejection rate | Validation executor | Worker quality issues — rising rejection rate signals worker skill regression or source changes |
| Validation flag rate | Validation executor | Borderline case frequency — high flag rate signals validator uncertainty or edge cases in source data |
| Validator cost per job | Post-run calculation | Validation overhead efficiency — cost of the validator sub-job relative to the worker job |
| Operator override rate (of rejections) | Operator actions | False positive rate — high override rate signals validator needs tuning; low rate validates validator quality |

These metrics are aggregated per skill, per source, and trended over time. Sudden discontinuities trigger alerts. The desktop control panel surfaces skill health dashboards alongside session-level observability.


#### Standard Operational Metrics

From day one, record aggregate:

- job start/end with session ID,
- daemon hostname/device id,
- prompt hash / skill version / skill path,
- agent session duration and output volume,
- result contract validation success/failure rate,
- parse failure counts per source,
- source scraping latency,
- quote confidence distribution,
- margin distribution,
- request pipeline success/failure by portal,
- SQLite-to-Supabase sync latency and failure counts,
- error manifest creation rate by type and stage,
- resource arbiter utilization (browser slots, agent slots, source lanes),
- cleanup verdict distribution (clean / forced / leaked).

#### Surface Integration

The desktop control panel provides an observability view:

- **Active agent sessions:** Real-time view of running sessions with live terminal output (PTY mode) or structured event stream (contract mode).
- **Session history:** Browse completed sessions with structured traces and forensic artifact drill-down.
- **Error manifest:** Filterable list of all pipeline errors with drill-down.
- **Health status:** Agent auth status, daemon uptime, sync status, resource arbiter utilization.
- **Metrics:** Charts for job throughput, success rates, latency, and cleanup health.

If this is not instrumented, the platform will be impossible to tune and debug.

---

## 7. Architectural Decisions

### ADR-001: Use Skill-Based Agent Scraping as Primary Data Retrieval

**Status:** Revised (supersedes original "Use Official Procurement APIs Before Scraping")

**Context:**

The platform needs to retrieve data from procurement sources and supplier marketplaces. The original approach was API-first connectors — separate TypeScript API client packages for each source. The owner’s feedback directs that all data retrieval should be constrained to the skill-based agent system running on the local machine, using browser automation. This simplifies the architecture: skills are easier to maintain, upgrade, and change than API client packages.

**Exception:** Cost-estimation data (customs, freight, FX) uses direct API access, where APIs are more reliable and structured.

**Decision:**

All procurement and supplier data retrieval uses skill-based agent scraping via browser automation. The agent runs on the local machine, powered by source-specific skills, and is orchestrated by the Rust daemon through adapter-mediated agent sessions, typically PTY-backed for retrieval flows. No API client packages are maintained for procurement or supplier sources. The official procurement sources (Find a Tender, Contracts Finder, PCS) are still the target data sources — but they are accessed via browser automation, not via TypeScript API clients.

**Options Considered:**

| Option | Pros | Cons |
|--------|------|------|
| A: Skill-based agent scraping (SELECTED) | single retrieval architecture for all sources, easy to maintain/upgrade skills, handles auth-heavy sources naturally | depends on agent reliability, skill maintenance when sites change |
| B: API-first connectors (original) | structured, stable | requires per-source TypeScript packages, harder to maintain, doesn’t handle auth-heavy sources well |
| C: Third-party aggregators | less engineering | vendor dependency, reduced control |

**Rationale:**

Skill-based agent scraping provides a unified data retrieval architecture. Instead of maintaining separate API client packages for each source (with their own auth flows, rate limiting, parsing), the platform maintains skills — markdown playbooks that tell the agent how to navigate each source. When a source changes, only the skill needs updating. This is materially simpler to maintain.

**Consequences:**

- Positive: unified architecture for all data retrieval, easier skill maintenance vs API client packages.
- Negative: depends on agent reliability and browser automation stability. Mitigated by comprehensive tracing and error manifest system.
- Cost management: the skill-based approach requires deliberate session batching (one session per source per notice, not per candidate), quoteability gating to suppress low-value searches, deterministic shortlisting before AI adjudication, and query-fingerprint caching. Without these optimizations, naive enrichment can reach $75–$170/day; with them, $20–$60/day is achievable. See §5.7 (Pipeline Cost Management) for details.

**User Input Required:** No

### ADR-002: Authority-Matrix Database Architecture — Local SQLite + Supabase/Postgres

**Status:** Revised (supersedes original "Use Supabase/Postgres as the Shared Control Plane"; updated to replace singular "source of truth" with authority matrix)

**Context:**

The system needs local-first operation with multi-user visibility. The original approach used Supabase/Postgres as the sole source of truth. The owner's feedback directs a dual-database architecture. Review feedback (planmaxer cycle 1) identified that a singular "source of truth" model hides the real bug class: mixed-authority rows where pipeline-generated data and human decisions coexist.

**Rationale for change:** Multiple users need access to the system's data, but relying solely on a shared remote Postgres creates corruption risk. However, the dual-database design introduces its own dominant risk: application-level logical corruption in the replication layer. The authority matrix model mitigates both by ensuring every table has exactly one authority domain with no mixed writes.

**Decision:**

- **Authority matrix** replaces singular "source of truth" — three authority domains: local operational, remote shared, remote projections.
- **Mixed-write tables are prohibited.** Concepts that span domains are split into separate tables (e.g., `pipeline_assessments` + `operator_decision_heads` + `bid_workflow_states` instead of a single `opportunity_decisions`).
- **Push-only pipeline replication:** Local → remote via transactional outbox with monotonic per-executor ordering and idempotent remote apply.
- **Pull-only shared config/decisions:** Remote → local via cursor-based pull on monotonic `remote_change_seq`. Operator decisions use compare-and-swap (not last-write-wins).
- **MVP constraint:** One active pipeline daemon per org. No generic conflict engine until a second writable desktop exists.
- **Convergence:** Via `local_seq`, `row_version`, `remote_change_seq`, `remote_revision` — never clock-based.
- **Schema coordination:** Canonical logical schema with `schema_epoch` in both databases and all sync envelopes.

**Options Considered:**

| Option | Pros | Cons |
|--------|------|------|
| A: Authority-matrix replication (SELECTED) | local-first resilience, corruption protection, no mixed-write bugs, idempotent sync | sync engine complexity, eventual consistency for web viewers |
| B: Singular source-of-truth local SQLite (previous revision) | simpler mental model | hides mixed-authority bugs, underspecified sync protocol |
| C: Supabase/Postgres only (original) | simpler architecture | network dependency, corruption risk, no local resilience |
| D: Convex | great realtime | weaker fit for SQL-heavy control plane |
| E: SpacetimeDB | exciting realtime model | maturity/fit risks for this workload |

**Rationale:**

The daemon runs locally and writes pipeline data continuously. A local database eliminates network dependency during operation and provides corruption resilience. The authority matrix ensures that the sync layer — now the highest-risk component for data integrity — has clear, testable rules per table. Multi-user visibility is achieved through push-only projections to Supabase.

**Consequences:**

- Positive: local-first resilience, corruption protection, no mixed-write ambiguity, idempotent sync, clear recovery boundaries.
- Negative: must design and maintain authority-based sync engine. Schema must be coordinated across two databases. Web viewers see eventually-consistent data. Recovery from local loss is explicitly lossy (full PTY output, error context permanently lost).

**User Input Required:** No

### ADR-003: Use Tauri v2 With a Rust Daemon

**Status:** Accepted (expanded with communication architecture)

**Context:**

The user wants local background execution and values Rust robustness. The desktop must communicate with a background daemon for writes, job triggers, and live session streaming while reading local SQLite for fast snapshot queries.

**Decision:**

Use Tauri v2 for the desktop shell with a **separate** Rust daemon process for background work. The desktop communicates with the daemon through a hybrid transport: Tauri `invoke()` commands for snapshot reads (via a Tauri-side QueryService reading SQLite directly) and mutations (via a DaemonBridge forwarding to the daemon over UDS/named-pipe), and Tauri `Channel<T>` for live streaming (session output, job progress, daemon health).

The daemon is NOT embedded in the Tauri app process. It runs independently, survives UI close, and has its own lifecycle. An optional localhost HTTP adapter is available for dev tooling only.

**Options Considered:**

| Option | Pros | Cons |
|--------|------|------|
| A: Tauri + separate Rust daemon (SELECTED) | lightweight desktop, daemon survives UI close, clean lifecycle separation | two-language stack, IPC complexity |
| B: Electron + Node daemon | familiar web tooling | heavier runtime |
| C: Pure web app only | simpler deploy | cannot satisfy local execution requirement |
| D: Embed daemon in Tauri process | simpler deployment | shutdown/crash/cron-while-closed problems |
| E: Desktop talks to daemon via localhost HTTP only | simpler transport | port conflicts, auth overhead, wrong abstraction for local IPC |

**Rationale:**

Tauri aligns with the desktop-first ops model and Rust is a good fit for process management and deterministic cost logic. Keeping the daemon separate means: closing the desktop window does not stop cron schedules, daemon crashes do not corrupt UI state, and the desktop can enter browse-only degraded mode when the daemon is unavailable. UDS/named-pipe transport avoids the security and discovery problems of localhost HTTP.

**Communication Architecture:**

- Snapshot reads: `invoke()` → QueryService (read-only SQLite) → `QueryEnvelope<T>`
- Writes/triggers: `invoke()` → DaemonBridge → UDS/named-pipe → daemon → `MutationAck`
- Live streams: daemon → DaemonBridge → `Channel<T>` → React
- See §5.8 for full protocol contracts and §5.10 for transport architecture.

**IPC Protocol Versioning:**

The daemon handshake returns an `ipc_protocol_version` separate from `schema_epoch`. A desktop binary and daemon binary drifting out of lockstep is guaranteed pain — the IPC protocol version catches this at startup and routes to a recovery/update screen rather than producing silent failures.

**Multi-Window Support:**

Tauri v2 supports per-window capability scoping. The desktop provides a detached **Session Monitor** window for multi-monitor setups, scoped to read-only session streaming permissions. A **Contract Compare** window may be added later with similar read-only scoping.

**Consequences:**

- Positive: lighter desktop footprint, strong systems language for daemon, clean lifecycle separation, browse-only degraded mode when daemon is down.
- Negative: desktop and web frontends should share UI, not runtime assumptions. IPC adds complexity vs embedded daemon.

**User Input Required:** No

### ADR-004: Use PTY-Based Agent Execution from Day 1

**Status:** Updated (refined from "PTY for everything" to "PTY as first-class capability with hybrid execution")

**Context:**

The entire data ingestion, intelligence, and bidding pipeline runs through agent sessions triggered by the Rust daemon. PTY execution is a first-class runtime from day 1 for browser automation, interactive auth, and TTY-dependent flows. However, both Codex (`exec` with `--json` JSONL output) and Claude (headless mode with `json`/`stream-json` and `--json-schema`) now support stable structured non-interactive execution. A hybrid model reduces blast radius: PTY for interactive flows, contract mode for deterministic tasks.

**Decision:**

Launch agent jobs through an adapter-mediated execution model with two transport modes from day 1:

1. **PTY mode:** Required for browser automation, interactive auth, or TTY-dependent CLI behavior.
2. **Contract mode:** Preferred for deterministic tasks where the vendor supports structured non-interactive execution.

The daemon selects mode per job based on the skill's declared `interaction_profile` and the adapter's capability probe. Raw session output is captured as a forensic artifact in both modes (PTY bytes in PTY mode, stdout/stderr in contract mode); structured events drive workflow state in both modes.

**Options Considered:**

| Option | Pros | Cons |
|--------|------|------|
| A: Hybrid PTY + contract mode via adapter (SELECTED) | mode matches job requirements, reduces blast radius, stronger result contracts for deterministic work, PTY still available for all interactive flows | adapter boundary adds design work; two code paths to maintain |
| B: PTY-only for everything | consistent single model, handles all CLI modes | pays PTY overhead for deterministic tasks, terminal parsing as control plane is fragile |
| C: Non-interactive only | simplest | misses TTY-dependent flows, cannot handle browser automation or interactive auth |

**Rationale:**

The pipeline must function without interruption every day. PTY provides real terminal semantics needed for browser automation and interactive flows. Contract mode provides stronger result contracts and simpler cleanup for deterministic work. The adapter boundary isolates the daemon from vendor CLI changes. The hybrid model gives the right execution semantics for each job type.

**Consequences:**

- Positive: robust daily operation, full terminal output capture for interactive flows, stronger result contracts for deterministic flows, vendor isolation via adapter boundary, process-tree supervision with explicit cleanup verdicts.
- Negative: two execution paths to maintain. Mitigated by the adapter abstraction producing a unified daemon event stream regardless of mode.

**User Input Required:** No

### ADR-005: Skills Are the Primary Data Retrieval Mechanism and Workflow Assets, Not the Workflow Engine

**Status:** Updated (adds explicit binding and skill metadata requirements)

**Context:**

Skills serve a dual role in the architecture:

1. **Primary data retrieval mechanism:** All procurement and supplier data is collected via skill-based agent scraping. Skills define how the agent navigates sources and extracts data.
2. **Workflow behavioral assets:** Skills define how the agent behaves during qualification, manifest generation, and bid automation.

Skills are NOT the workflow engine — orchestration (scheduling, job state, retries, sync) stays in the daemon and database. Scheduled production jobs bind explicit skill id and version; implicit skill discovery is disabled for cron runs to prevent reliability issues at scale.

3. **Validator skills** are a new skill category that follows the same boundary rules. The daemon orchestrates validator sessions just like worker sessions — spawning them as sub-jobs with the same adapter, lifecycle, and cleanup semantics. Validator skills are passive directory assets loaded by the daemon; they do not contain orchestration logic. Adding a new validator overlay to a worker skill requires only adding a `validator-overlay.md` file to the worker skill's directory — no daemon code changes.

**Decision:**

Skills are BOTH the primary data retrieval mechanism AND workflow behavioral assets. The daemon/database owns orchestration: when to run which skill, job state tracking, retry logic, scheduling, sync to Supabase. Skills define: how the agent navigates a source, what data to extract, how to handle errors during execution, and what execution mode they require (`interaction_profile`). Every skill declares metadata (interaction profile, result schema version, auth requirements) that the daemon uses for execution planning.

**Options Considered:**

| Option | Pros | Cons |
|--------|------|------|
| A: skills as data retrieval + assets with explicit binding, DB orchestration (SELECTED) | unified retrieval architecture, reliable orchestration, deterministic cron execution | more skill maintenance work, metadata overhead |
| B: skills as orchestration | fast to prototype | state becomes opaque and fragile |
| C: skills with implicit discovery for all runs | less config | cron reliability depends on ambient discovery behavior — broken by CLI updates |

**Rationale:**

Agent prompts are not a durable job scheduler. But agent skills ARE an excellent mechanism for defining source-specific data retrieval playbooks. The architecture cleanly separates: skills handle the "how" of data retrieval and workflow execution, while the daemon/database handles the "when", "why", and "what happened". Explicit skill binding for scheduled jobs prevents implicit discovery failures from disrupting production pipelines.

**Consequences:**

- Positive: unified data retrieval, replayability, observability, easy skill maintenance, deterministic cron execution via explicit binding.
- Negative: requires explicit workflow modeling, skill versioning, and skill metadata declarations.

**User Input Required:** No

### ADR-006: Build an Internal Landed-Cost Engine

**Status:** Accepted (revised scope clarification)

**Context:**

Third-party shipping/tax APIs are useful, but margin decisions need explainable and auditable numbers. The internal engine uses HMRC Trade Tariff for duty/VAT, HMRC FPO for HS classification candidates, HMRC Exchange Rates for customs FX, and commercial APIs for freight and commercial FX.

**Decision:**

Implement customs, valuation, shipping mode selection, and FX combination as internal logic using authoritative and commercial source inputs. The engine operates at two grades:
- **Screening grade** — automated across the whole market using indicative data
- **Decision grade** — automated only for repeated product families with proven codebooks and confirmed supplier economics

**Options Considered:**

| Option | Pros | Cons |
|--------|------|------|
| A: internal engine + official data | explainable, auditable | more engineering |
| B: third-party black-box estimator | quick | opaque and weaker for high-stakes decisions |

**Rationale:**

The core business value is the decision engine. That cannot be delegated blindly.

**Consequences:**

- Positive: higher trust, better override behavior, explainable decisions with full audit trail.
- Positive: screening-grade automation filters the whole market; decision-grade grows as product-family codebooks accumulate.
- Negative: implementation complexity is high — 10-digit resolution, measure scanning, dual FX, Incoterm normalization, scenario-based margins.
- Negative: accuracy ceiling depends on classification and freight estimation quality, requiring ongoing calibration.

**User Input Required:** No

### ADR-007: Automate the Request Pipeline, Flag Errors for Debugging

**Status:** Revised (relaxes original "Human Review Is Required for Low-Confidence Commercial Decisions")

**Context:**

The platform automates the **request pipeline**, not real-money bid submissions. Actual negotiations and fulfillment are handled manually by humans using the pipeline output. The cost of pipeline failure is low — no real money changes hands automatically. Human review remains valuable for commercial decisions, but the automation risk framing is significantly relaxed.

**Decision:**

Full automation of the request pipeline is acceptable. When pipeline steps fail, the error manifest system flags: which step failed, the error message, the pipeline stage, and enough context to debug. Human operators handle the actual commercial decisions (negotiations, fulfillment) manually.

Confidence-gated review is still valuable for:
- Commodity code classification (getting HS codes right matters for cost estimation accuracy).
- Supplier matching (ensuring the right products are being compared).

But these are quality gates for better intelligence, not safety gates against financial risk.

**Options Considered:**

| Option | Pros | Cons |
|--------|------|------|
| A: Full pipeline automation + error manifest (SELECTED) | high throughput, low risk since not real money, structured error debugging | may produce some low-quality intelligence |
| B: Confidence-gated human review at every stage (original) | highest quality | too much operator involvement for the actual risk level |

**Rationale:**

Since the platform automates request preparation (not real-money transactions), the cost of errors is low. The error manifest system provides structured debugging when things go wrong, and human operators handle all actual commercial decisions.

**Consequences:**

- Positive: higher throughput, simpler operator workflow, structured error debugging via manifests.
- Negative: some pipeline outputs may be lower quality. Mitigated by confidence scoring and the error manifest system.

**User Input Required:** No

### ADR-008: Desktop is Control Panel, Web is Read-Only Viewing Portal

**Status:** Revised (answers original "Separate Desktop and Web Shells" question, expanded with shared component architecture)

**Context:**

The desktop control panel and viewing portal serve fundamentally different roles. They have fundamentally different data sources (local SQLite vs Supabase), different capabilities (write vs read-only), and different latency characteristics.

**Decision:**

Separate shells with clearly different capabilities:

- **`apps/desktop`** (Tauri + static frontend) — **CONTROL PANEL**: Full read/write/trigger capabilities. Trigger workflows, manage agents, configure skills, make reject/bid/watch decisions, view local data from SQLite via QueryService, view agent tracing logs via session streaming, manage error manifests.
- **`apps/web`** (hosted) — **READ-ONLY VIEWING PORTAL**: Browse opportunities, view manifests, view pipeline status, view errors. No write operations. Reads from Supabase projection tables. Live updates via Supabase Broadcast invalidation.
- **`packages/ui`** for shared **presentational** React components — view-model-driven frames, formatting helpers, badges, Markdown renderers. Desktop shell includes additional control components not shared with web.
- **`packages/contracts-schema`** for shared domain models and view-model types.

**Shared Component Boundary:**

Share down to: view-models, presentational components, formatting/markdown/tables/badges.

Stop sharing at: repositories, data-fetching hooks, realtime subscriptions, mutation flows. The abstraction breaks when hooks pretend SQLite and Supabase are the same thing — they are not. Use frame + slot composition (e.g., `ContractCardFrame` with pluggable `actions` slot) rather than boolean-prop components (`canReject`, `canBid`, `readOnly`).

See §5.8 Shared React Component Architecture for the full specification.

**Options Considered:**

| Option | Pros | Cons |
|--------|------|------|
| A: Split shells with different roles (SELECTED) | clear responsibility split, web is simple, desktop has full power | some duplicated UI shell code |
| B: Single Next app everywhere with equal capabilities | less duplication | desktop constraints leak into product architecture, web gets unnecessary write capabilities |

**Rationale:**

The desktop control panel runs alongside the daemon and has direct access to local SQLite and agent controls. The viewing portal is for remote viewing only — it reads from Supabase and provides no write operations. This clear split simplifies the web shell significantly. The shared component boundary is deliberately narrow: share views, not data access lies.

**Consequences:**

- Positive: web shell is simple and safe (no write operations to protect). Desktop has full local power. Shared presentational components avoid duplication without false abstraction.
- Negative: data-fetching hooks are duplicated per surface (SQLite-via-QueryService vs Supabase-client). This is intentional — the alternative is a leaky abstraction.

**User Input Required:** No (resolved by owner feedback)

---

## 8. File Reference Map

### 8.1 Files to Read (Pre-Implementation)

**Critical Reading:**

| Priority | File | Purpose |
|----------|------|---------|
| 1 | `/Users/alinaaitova/Desktop/Testproject1/docs/research/2026-03-08-public-procurement-intelligence-platform-research.md` | Foundational architecture and decisions |
| 2 | `/Users/alinaaitova/.codex/skills/idea-to-research/SKILL.md` | Skill workflow and expected research handoff standard |

### 8.2 Files to Create

#### Workspace Bootstrap

| File | Purpose | Template |
|------|---------|----------|
| `/Users/alinaaitova/Desktop/Testproject1/package.json` | root workspace scripts | custom |
| `/Users/alinaaitova/Desktop/Testproject1/bunfig.toml` | Bun workspace/runtime settings | custom |
| `/Users/alinaaitova/Desktop/Testproject1/Cargo.toml` | Rust workspace | custom |
| `/Users/alinaaitova/Desktop/Testproject1/.env.example` | environment contract | custom |

#### Database and Infra

| File | Purpose | Template |
|------|---------|----------|
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/migrations/0001_sqlite_state_initial.sql` | Local SQLite state.db schema bootstrap — pipeline entities (procurement_processes, contract_notices, notice_lots, qualification_runs, product_candidates, search_profiles, search_attempts, supplier_quotes, landed_cost_estimates, manifests, pipeline_assessments, bid_workflow_states, process_relations, process_aliases, executors), plus local-only runtime/cache tables (`source_runtime_state`, `listing_evidence`) | based on section 5.4 (SQLite syntax) |
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/migrations/0002_sqlite_state_sync.sql` | Local outbox, sync batches, pull cursors, pending_operator_intents queue, stage tracking (process_stage_status, process_stage_heads) | section 5.4 sync infrastructure |
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/migrations/0003_sqlite_artifacts.sql` | Artifacts DB schema — agent_session_logs (with orchestration domain columns), raw_documents, error_manifests | custom |
| `/Users/alinaaitova/Desktop/Testproject1/packages/db/migrations/0001_initial.sql` | Supabase/Postgres projection tables (all push-owned entities including notice_lots, process_relations, process_aliases, search_profiles, search_attempts, workflow_job_summaries, error_manifest_summaries, viewer_sync_status) + remote-owned tables (shared_company_profile, shared_source_configs, operator_decision_heads) | based on section 5.4 |
| `/Users/alinaaitova/Desktop/Testproject1/packages/db/migrations/0002_rls.sql` | RLS policies per authority role (viewer, operator, admin, daemon) — see §6.3 for policy matrix | section 6.3 |
| `/Users/alinaaitova/Desktop/Testproject1/packages/db/migrations/0003_sync_apply.sql` | Transactional sync apply RPC for push batches + operator decision CAS RPC | custom |
| `/Users/alinaaitova/Desktop/Testproject1/packages/db/migrations/0004_schema_metadata.sql` | Schema metadata table + initial schema_epoch | custom |
| `/Users/alinaaitova/Desktop/Testproject1/packages/db/migrations/0005_stage_tracking.sql` | process_stage_status, process_stage_heads projection tables | section 5.4 stage tracking |
| `/Users/alinaaitova/Desktop/Testproject1/packages/db/migrations/0006_junction_tables.sql` | notice_cpv_codes, product_commodity_codes, lot_cpv_codes | section 5.4 junction tables |
| `/Users/alinaaitova/Desktop/Testproject1/infra/supabase/config.toml` | local Supabase config | Supabase standard |

#### Shared Domain

| File | Purpose | Template |
|------|---------|----------|
| `/Users/alinaaitova/Desktop/Testproject1/packages/contracts-schema/src/contracts.ts` | canonical contract types | section 5.4 |
| `/Users/alinaaitova/Desktop/Testproject1/packages/contracts-schema/src/jobs.ts` | job payloads and states | custom |
| `/Users/alinaaitova/Desktop/Testproject1/packages/contracts-schema/src/confidence.ts` | confidence models | custom |
| `/Users/alinaaitova/Desktop/Testproject1/packages/contracts-schema/src/zod.ts` | runtime validation | custom |

#### Shared UI

| File | Purpose | Template |
|------|---------|----------|
| `/Users/alinaaitova/Desktop/Testproject1/packages/ui/src/index.ts` | shared presentational exports for desktop + viewing portal | custom |
| `/Users/alinaaitova/Desktop/Testproject1/packages/ui/src/contracts/contract-card-frame.tsx` | frame + slot base component for contract/opportunity cards | custom |
| `/Users/alinaaitova/Desktop/Testproject1/packages/ui/src/manifest/manifest-viewer.tsx` | shared manifest presentation component | custom |
| `/Users/alinaaitova/Desktop/Testproject1/packages/ui/src/status/pipeline-badge.tsx` | shared pipeline-state badge component | custom |
| `/Users/alinaaitova/Desktop/Testproject1/packages/ui/src/status/sync-age-badge.tsx` | shared sync-freshness badge component | custom |

#### Cost-Estimation Intelligence Engine (API-Based)

| File | Purpose | Template |
|------|---------|----------|
| `/Users/alinaaitova/Desktop/Testproject1/packages/intelligence/src/qualification/expiry.ts` | 30-day expiry filter | custom |
| `/Users/alinaaitova/Desktop/Testproject1/packages/intelligence/src/qualification/access-route.ts` | access-route classification and bidability rules | custom |
| `/Users/alinaaitova/Desktop/Testproject1/packages/intelligence/src/qualification/goods.ts` | goods/services qualification | custom |
| `/Users/alinaaitova/Desktop/Testproject1/packages/intelligence/src/qualification/quoteability.ts` | deterministic quoteability scoring | custom |
| `/Users/alinaaitova/Desktop/Testproject1/packages/intelligence/src/relations/resolve-process-relations.ts` | OCID + heuristic relation resolution | custom |
| `/Users/alinaaitova/Desktop/Testproject1/packages/intelligence/src/extraction/products.ts` | line-item/product extraction | custom |
| `/Users/alinaaitova/Desktop/Testproject1/packages/intelligence/src/search/build-search-profile.ts` | multilingual query-wave construction from structured candidate slots | custom |
| `/Users/alinaaitova/Desktop/Testproject1/packages/intelligence/src/customs/hmrc-tariff.ts` | HMRC Trade Tariff API integration | official HMRC docs |
| `/Users/alinaaitova/Desktop/Testproject1/packages/intelligence/src/customs/hmrc-fpo.ts` | HMRC FPO candidate code generator API | official HMRC FPO docs |
| `/Users/alinaaitova/Desktop/Testproject1/packages/intelligence/src/customs/classification-preprocessing.ts` | description-quality gate, bundle splitter, attribute extraction, multi-query FPO dispatch | custom (§4.5) |
| `/Users/alinaaitova/Desktop/Testproject1/packages/intelligence/src/customs/ten-digit-resolver.ts` | 10-digit commodity code resolution from 6/8-digit FPO candidates via Trade Tariff API | custom (§4.5) |
| `/Users/alinaaitova/Desktop/Testproject1/packages/intelligence/src/customs/measure-scanner.ts` | trade remedy detection, preferential eligibility, quota checks, additional code requirements | custom (§4.5) |
| `/Users/alinaaitova/Desktop/Testproject1/packages/intelligence/src/customs/taric-support.ts` | Taric Support API for code validation | Taric Support docs |
| `/Users/alinaaitova/Desktop/Testproject1/packages/intelligence/src/customs/hs-ping.ts` | HS Ping comparison lookup | HS Ping docs |
| `/Users/alinaaitova/Desktop/Testproject1/packages/intelligence/src/fx/xe.ts` | Xe commercial FX integration | official Xe docs |
| `/Users/alinaaitova/Desktop/Testproject1/packages/intelligence/src/fx/hmrc-exchange-rates.ts` | HMRC official exchange rates for customs valuation | HMRC docs |
| `/Users/alinaaitova/Desktop/Testproject1/packages/intelligence/src/fx/currencybeacon.ts` | CurrencyBeacon backup/comparison FX integration | CurrencyBeacon docs |
| `/Users/alinaaitova/Desktop/Testproject1/packages/intelligence/src/fx/oanda.ts` | OANDA historical FX integration | OANDA docs |
| `/Users/alinaaitova/Desktop/Testproject1/packages/intelligence/src/logistics/freightos.ts` | Freightos freight API integration | official Freightos docs |
| `/Users/alinaaitova/Desktop/Testproject1/packages/intelligence/src/logistics/searates.ts` | SeaRates freight API integration | SeaRates docs |
| `/Users/alinaaitova/Desktop/Testproject1/packages/intelligence/src/logistics/mode-selection.ts` | freight mode decision tree with hard mode locks and dual packing scenarios | custom (§4.4) |
| `/Users/alinaaitova/Desktop/Testproject1/packages/intelligence/src/logistics/easyship.ts` | Easyship parcel-mode freight and duty estimation | Easyship docs |
| `/Users/alinaaitova/Desktop/Testproject1/packages/intelligence/src/logistics/shippo.ts` | Shippo parcel-mode carrier rates and international shipping | Shippo docs |
| `/Users/alinaaitova/Desktop/Testproject1/packages/intelligence/src/validation/dutify.ts` | Dutify landed-cost comparison adapter | Dutify docs |
| `/Users/alinaaitova/Desktop/Testproject1/packages/intelligence/src/validation/zonos.ts` | Zonos landed-cost comparison adapter | Zonos docs |
| `/Users/alinaaitova/Desktop/Testproject1/packages/intelligence/src/validation/avalara.ts` | Avalara landed-cost comparison adapter | Avalara docs |
| `/Users/alinaaitova/Desktop/Testproject1/packages/intelligence/src/margins/revenue-normalizer.ts` | revenue basis classification and expected_addressable_revenue_ex_vat extraction | custom (Appendix A) |
| `/Users/alinaaitova/Desktop/Testproject1/packages/intelligence/src/margins/quote-normalizer.ts` | Incoterm cost boundary normalization (EXW/FOB/CIF/DDP → comparable basis) | custom (Appendix A) |
| `/Users/alinaaitova/Desktop/Testproject1/packages/intelligence/src/margins/margin-scenarios.ts` | low/base/high margin scenario calculation with critical blocker checks | custom (Appendix A) |

#### Evaluation Harness

| File | Purpose | Template |
|------|---------|----------|
| `/Users/alinaaitova/Desktop/Testproject1/packages/intelligence/src/evaluation/harness.ts` | gold-set pipeline runner — feeds curated lots through extraction → matching → cost pipeline, collects per-lot results | custom |
| `/Users/alinaaitova/Desktop/Testproject1/packages/intelligence/src/evaluation/metrics.ts` | lot-level precision/recall computation, archetype-stratified reporting, pass/fail against threshold | custom |

#### Manifests

| File | Purpose | Template |
|------|---------|----------|
| `/Users/alinaaitova/Desktop/Testproject1/packages/manifests/src/render-opportunity.ts` | Markdown manifest generation | custom |
| `/Users/alinaaitova/Desktop/Testproject1/packages/manifests/src/templates/opportunity.md.ts` | manifest template | custom |
| `/Users/alinaaitova/Desktop/Testproject1/packages/manifests/src/render-error-manifest.ts` | Markdown error manifest generation | custom |
| `/Users/alinaaitova/Desktop/Testproject1/packages/manifests/src/templates/error.md.ts` | error manifest template | custom |

#### Bidding

| File | Purpose | Template |
|------|---------|----------|
| `/Users/alinaaitova/Desktop/Testproject1/packages/bidding/src/workflows/index.ts` | workflow registry | custom |
| `/Users/alinaaitova/Desktop/Testproject1/packages/bidding/src/workflows/proactis-v1.ts` | initial downstream tender-portal workflow | custom |
| `/Users/alinaaitova/Desktop/Testproject1/packages/bidding/src/company-profile/schema.ts` | bidder profile contract | custom |

#### Desktop Shell (CONTROL PANEL)

| File | Purpose | Template |
|------|---------|----------|
| `/Users/alinaaitova/Desktop/Testproject1/apps/desktop/package.json` | desktop app package | Tauri bootstrap |
| `/Users/alinaaitova/Desktop/Testproject1/apps/desktop/src-tauri/src/main.rs` | Tauri entrypoint | Tauri bootstrap |
| `/Users/alinaaitova/Desktop/Testproject1/apps/desktop/src-tauri/src/query_service.rs` | read-only SQLite QueryService — page-specific read models, QueryEnvelope responses | custom (§5.8) |
| `/Users/alinaaitova/Desktop/Testproject1/apps/desktop/src-tauri/src/daemon_bridge.rs` | DaemonBridge — UDS/named-pipe RPC proxy to daemon, Channel<T> stream relay | custom (§5.8) |
| `/Users/alinaaitova/Desktop/Testproject1/apps/desktop/src-tauri/capabilities/default.json` | shell/sidecar permissions with per-window capability scoping | Tauri permissions docs |
| `/Users/alinaaitova/Desktop/Testproject1/apps/desktop/src/lib/daemon.ts` | frontend bridge to daemon (Tauri invoke wrappers, typed command definitions) | custom |
| `/Users/alinaaitova/Desktop/Testproject1/apps/desktop/src/lib/session-stream-store.ts` | SessionRegistry + SessionBuffer external stores for live session streaming | custom (§5.8) |
| `/Users/alinaaitova/Desktop/Testproject1/apps/desktop/src/lib/terminal-buffer.ts` | ring buffer, gap detection, live-tail/history mode switching for terminal viewer | custom (§5.8) |
| `/Users/alinaaitova/Desktop/Testproject1/apps/desktop/src/lib/bootstrap-state.ts` | desktop startup state machine — daemon handshake, mode detection (healthy/remote-degraded/browse-only/recovery-required) | custom (§5.8) |
| `/Users/alinaaitova/Desktop/Testproject1/apps/desktop/src/pages/contracts.tsx` | contracts list (reads local SQLite via QueryService, keyset pagination) | custom |
| `/Users/alinaaitova/Desktop/Testproject1/apps/desktop/src/pages/contract-detail.tsx` | opportunity detail with reject/bid/rerun controls via DaemonBridge | custom |
| `/Users/alinaaitova/Desktop/Testproject1/apps/desktop/src/pages/agent-sessions.tsx` | agent session monitoring with live terminal output via Channel<T> streaming | custom |
| `/Users/alinaaitova/Desktop/Testproject1/apps/desktop/src/pages/error-manifest.tsx` | error manifest viewer with resolution controls | custom |
| `/Users/alinaaitova/Desktop/Testproject1/apps/desktop/src/pages/settings.tsx` | source configs, skill bindings, and local runtime settings | custom |

#### Web Shell (READ-ONLY VIEWING PORTAL)

| File | Purpose | Template |
|------|---------|----------|
| `/Users/alinaaitova/Desktop/Testproject1/apps/web/package.json` | web app package | custom |
| `/Users/alinaaitova/Desktop/Testproject1/apps/web/src/app/contracts/page.tsx` | contracts list (reads Supabase projections, keyset pagination) | custom |
| `/Users/alinaaitova/Desktop/Testproject1/apps/web/src/app/contracts/[id]/page.tsx` | opportunity detail (read-only, no controls) | custom |
| `/Users/alinaaitova/Desktop/Testproject1/apps/web/src/app/status/page.tsx` | pipeline status and error overview (read-only) | custom |
| `/Users/alinaaitova/Desktop/Testproject1/apps/web/src/lib/realtime-subscriptions.ts` | Supabase Broadcast subscription manager — topic attachment, invalidation dispatch | custom (§5.8) |
| `/Users/alinaaitova/Desktop/Testproject1/apps/web/src/lib/staleness-store.ts` | viewer_sync_status polling, staleness threshold rendering (green/amber/red) | custom (§5.8) |
| `/Users/alinaaitova/Desktop/Testproject1/apps/web/src/lib/topic-map.ts` | page-to-Broadcast-topic mapping (org:contracts, process:updates, org:sync, etc.) | custom (§5.8) |

#### Rust Daemon

| File | Purpose | Template |
|------|---------|----------|
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/Cargo.toml` | daemon crate definition | custom |
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/src/main.rs` | daemon entrypoint | custom |
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/src/db/sqlite.rs` | local SQLite connection and schema management | custom |
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/src/db/sync.rs` | SQLite↔Supabase push-pull sync engine | custom |
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/src/db/checkpoints.rs` | local sync checkpoints and recovery cursors | custom |
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/src/sources/registry.rs` | source registry loading, enablement, and schedule binding | custom |
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/src/scheduler/cron.rs` | local cron scheduler for agent jobs with catchup sweep | custom |
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/src/jobs/claim.rs` | job claiming/leases (local SQLite) | custom |
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/src/jobs/runner.rs` | job executor via adapter-mediated sessions | custom |
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/src/pty/session.rs` | PTY session creation, lifecycle, and process-group supervision | custom |
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/src/pty/capture.rs` | raw PTY byte capture to immutable artifact files | custom |
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/src/pty/health.rs` | agent health monitoring (stall, auth challenge, sleep detection) | custom |
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/src/pty/reaper.rs` | session reaper — process-group cleanup, kill escalation, orphan sweep | custom |
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/src/pty/hardening.rs` | PTY hardening — fd hygiene, EIO handling, terminal geometry | custom |
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/src/agents/adapter.rs` | agent adapter trait — canonical event stream, mode selection, capability probing | custom |
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/src/agents/codex.rs` | Codex adapter — launch, JSONL parsing, auth probing | local CLI contract |
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/src/agents/claude.rs` | Claude adapter — launch, JSON/stream-json parsing, auth probing | local CLI contract |
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/src/agents/artifacts.rs` | artifact store — compressed immutable session artifacts | custom |
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/src/agents/contracts.rs` | result contract validator — schema validation, checkpoint sequencing | custom |
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/src/agents/arbiter.rs` | resource arbiter — browser slots, agent slots, source lanes, priority scheduling | custom |
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/src/errors/manifest.rs` | error manifest creation and management | custom |
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/src/errors/markdown.rs` | error manifest Markdown artifact writer | custom |
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/src/tracing/structured.rs` | structured trace derivation from control/protocol events | custom |
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/src/tracing/forensic.rs` | forensic transcript enrichment (best-effort PTY output parsing) | custom |
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/src/prompt/renderer.rs` | Kickoff Prompt Renderer: renders worker/validator/manual-mode/dry-run kickoff prompts | custom |
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/src/validation/policy.rs` | Validation policy engine: evaluates `always` / `sample` / `trigger` / `skip` policy modes and returns validate/skip | custom |
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/src/validation/executor.rs` | Validation executor: spawns validator sub-jobs, injects run manifest + OQG signals, processes verdicts | custom |
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/src/validation/manifest_writer.rs` | Run manifest artifact writer: stores agent-produced MD narratives as compressed immutable artifacts | custom |

#### Skills (Primary Data Retrieval + Workflow Playbooks)

| File | Purpose | Template |
|------|---------|----------|
| `/Users/alinaaitova/Desktop/Testproject1/skills/procurement-ingestion/fts/SKILL.md` | Find a Tender agent scraping playbook | custom |
| `/Users/alinaaitova/Desktop/Testproject1/skills/procurement-ingestion/contracts-finder/SKILL.md` | Contracts Finder agent scraping playbook | custom |
| `/Users/alinaaitova/Desktop/Testproject1/skills/procurement-ingestion/pcs/SKILL.md` | PCS agent scraping playbook | custom |
| `/Users/alinaaitova/Desktop/Testproject1/skills/supplier-price-enrichment/ebay/SKILL.md` | eBay pricing agent scraping playbook | custom |
| `/Users/alinaaitova/Desktop/Testproject1/skills/supplier-price-enrichment/amazon/SKILL.md` | Amazon pricing agent scraping playbook | custom |
| `/Users/alinaaitova/Desktop/Testproject1/skills/supplier-price-enrichment/alibaba/SKILL.md` | Alibaba pricing agent scraping playbook | custom |
| `/Users/alinaaitova/Desktop/Testproject1/skills/supplier-price-enrichment/1688/SKILL.md` | 1688 wholesale pricing agent scraping playbook | custom |
| `/Users/alinaaitova/Desktop/Testproject1/skills/document-acquisition/SKILL.md` | public tender document download playbook | custom |
| `/Users/alinaaitova/Desktop/Testproject1/skills/lot-extraction/SKILL.md` | lot structure extraction from unstructured documents | custom |
| `/Users/alinaaitova/Desktop/Testproject1/skills/product-extraction/SKILL.md` | product candidate extraction with candidateType and structured slots | custom |
| `/Users/alinaaitova/Desktop/Testproject1/skills/preliminary-qualification/SKILL.md` | expiry + goods reasoning | custom |
| `/Users/alinaaitova/Desktop/Testproject1/skills/manifest-generation/SKILL.md` | manifest creation workflow | custom |
| `/Users/alinaaitova/Desktop/Testproject1/skills/bid-portal-proactis/SKILL.md` | first bidding portal workflow | custom |
| `/Users/alinaaitova/Desktop/Testproject1/skills/auth-refresh/alibaba/SKILL.md` | Alibaba auth state refresh playbook | custom |
| `/Users/alinaaitova/Desktop/Testproject1/skills/auth-refresh/1688/SKILL.md` | 1688 auth state refresh playbook | custom |
| `/Users/alinaaitova/Desktop/Testproject1/skills/source-health/fts/SKILL.md` | Find a Tender source health canary | custom |
| `/Users/alinaaitova/Desktop/Testproject1/skills/source-health/alibaba/SKILL.md` | Alibaba source health canary | custom |

| `/Users/alinaaitova/Desktop/Testproject1/skills/validation/base/SKILL.md` | Base validator skill: generic run manifest quality checks (completeness, consistency, cross-check against result contract) | custom |
| `/Users/alinaaitova/Desktop/Testproject1/skills/validation/base/result.schema.json` | Validator result contract schema (verdict + issues + quality assessment) | custom |
| `/Users/alinaaitova/Desktop/Testproject1/skills/validation/base/checkpoint.schema.json` | Validator checkpoint schema placeholder (validators do not checkpoint) | custom |
| `/Users/alinaaitova/Desktop/Testproject1/skills/validation/base/canary.inputs.jsonl` | Known-good and known-bad run manifests for validator smoke tests | custom |
| `/Users/alinaaitova/Desktop/Testproject1/skills/validation/base/golden.assertions.yaml` | Golden verdict expectations for validator regression tests | custom |
| `/Users/alinaaitova/Desktop/Testproject1/skills/validation/procurement-ingestion/SKILL.md` | Procurement-specific validation overlay (OCID structure, date ranges, source verification) | custom |
| `/Users/alinaaitova/Desktop/Testproject1/skills/validation/supplier-enrichment/SKILL.md` | Supplier-specific validation overlay (pricing plausibility, currency, locale) | custom |
| `/Users/alinaaitova/Desktop/Testproject1/skills/validation/document-acquisition/SKILL.md` | Document-acquisition validation overlay (artifact type, content, completeness) | custom |
| `/Users/alinaaitova/Desktop/Testproject1/skills/validation/product-extraction/SKILL.md` | Product-extraction validation overlay (candidate plausibility, structured-slot consistency) | custom |

Each production skill directory includes supporting files beyond `SKILL.md`: `result.schema.json`, `checkpoint.schema.json`, `canary.inputs.jsonl`, `golden.assertions.yaml`, and optional `scripts/`, `references/`, and `validator-overlay.md` files. These artifacts operationalize the skill as a testable, versionable contract (see §4.10 Production Skill Directory Standard).

`skills/validation/landed-cost/SKILL.md` is intentionally absent from the current file map because landed-cost remains deterministic and API-based in this architecture rather than skill-executed.

### 8.3 Test Files Required

#### Agent E2E Tests (HIGHEST PRIORITY)

| Test File | Coverage |
|-----------|----------|
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/tests/agent_pty_session.rs` | PTY session creation, output capture, full lifecycle including reap and cleanup verdict |
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/tests/agent_contract_session.rs` | Contract-mode session creation, structured event capture, result contract validation |
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/tests/agent_procurement_ingestion.rs` | Real agent session running procurement ingestion skill, verifying output and result contract |
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/tests/agent_supplier_enrichment.rs` | Real agent session running supplier enrichment skill, verifying output and result contract |
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/tests/agent_health_detection.rs` | Stall detection, auth challenge detection, sleep interruption, error classification |
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/tests/agent_error_manifest.rs` | Error manifest creation on agent failures with full runtime context |

#### Adapter Contract Tests

| Test File | Coverage |
|-----------|----------|
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/tests/adapter_codex.rs` | Codex adapter capability probe, JSONL parsing, mode selection, canary mode |
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/tests/adapter_claude.rs` | Claude adapter capability probe, JSON parsing, mode selection, canary mode, protocol-mismatch detection |
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/tests/adapter_result_contract.rs` | Result contract validation, checkpoint sequencing, partial failure classification |

#### Fault-Injection Tests

| Test File | Coverage |
|-----------|----------|
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/tests/fault_process_cleanup.rs` | Orphaned grandchildren, dropped child handle, cleanup timeout, leaked process detection |
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/tests/fault_pty_edge_cases.rs` | PTY EOF/EIO handling, fd leak scenarios, terminal geometry stability |
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/tests/fault_host_events.rs` | Host sleep/monotonic gap, resource exhaustion, concurrent session pressure |
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/tests/fault_partial_failure.rs` | Partial checkpoint then crash, result contract validation failures, auth challenge mid-run |

#### Deterministic Replay Tests

| Test File | Coverage |
|-----------|----------|
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/tests/replay_pty_sessions.rs` | Replay captured PTY byte streams against adapter parser and health monitor |
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/tests/replay_contract_sessions.rs` | Replay captured JSONL/JSON sessions against adapter parser and result contract validator |

#### Integration Tests (Real Databases, No Mocks)

| Test File | Coverage |
|-----------|----------|
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/tests/sqlite_schema.rs` | SQLite schema creation and migration |
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/tests/sqlite_supabase_sync.rs` | Push-pull sync between SQLite and Supabase |
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/tests/job_claim.rs` | Job claiming and lease management in SQLite |
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/tests/cron_scheduler.rs` | Cron-based job scheduling |

#### Unit Tests (Deterministic Logic)

| Test File | Coverage |
|-----------|----------|
| `/Users/alinaaitova/Desktop/Testproject1/packages/intelligence/src/relations/resolve-process-relations.test.ts` | relation-resolution scoring and hard blockers |
| `/Users/alinaaitova/Desktop/Testproject1/packages/intelligence/src/qualification/goods.test.ts` | goods/services qualification |
| `/Users/alinaaitova/Desktop/Testproject1/packages/intelligence/src/qualification/quoteability.test.ts` | quoteability thresholds and candidate routing |
| `/Users/alinaaitova/Desktop/Testproject1/packages/intelligence/src/customs/hmrc-tariff.test.ts` | tariff parsing |
| `/Users/alinaaitova/Desktop/Testproject1/packages/intelligence/src/customs/hmrc-fpo.test.ts` | FPO HS code classification |
| `/Users/alinaaitova/Desktop/Testproject1/packages/intelligence/src/margins/net-margin.test.ts` | gross and net margin rules |

#### Browser E2E Tests

| Test File | Coverage |
|-----------|----------|
| `/Users/alinaaitova/Desktop/Testproject1/apps/desktop/e2e/contracts.spec.ts` | desktop control panel browsing and decision flow |
| `/Users/alinaaitova/Desktop/Testproject1/apps/web/e2e/contracts.spec.ts` | viewing-portal read-only browsing |


#### Skill Testing

| Test File | Coverage |
|-----------|----------|
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/tests/skill_lint.rs` | Skill directory structure validation — required files present, metadata valid, trigger boundaries declared, schemas parseable |
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/tests/skill_canary_harness.rs` | Live smoke canary framework — page fingerprint check, auth verification, minimal extraction, result shape validation |
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/tests/skill_golden_journeys.rs` | Invariant-based golden live journey tests per skill family — real E2E against known stable inputs |
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/tests/skill_shadow_compare.rs` | Shadow run comparison framework — quality metric diff between stable and candidate skill versions |
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/tests/skill_drift_monitor.rs` | Production drift detection — page fingerprint hash, fallback depth, fill-rate, duplicate rate trending |

#### Validator Tests

| Test File | Coverage |
|-----------|----------|
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/tests/validation_policy.rs` | Validation policy engine decision logic — all combinations of skill stage, policy mode, trigger conditions |
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/tests/validation_executor.rs` | Validator sub-job spawning, kickoff prompt composition, verdict processing (approved/rejected/flagged) |
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/tests/validation_dry_run.rs` | Dry-run mode — verdicts logged but not enforced, rejected results still advance |
| `/Users/alinaaitova/Desktop/Testproject1/crates/daemon/tests/validation_golden_verdicts.rs` | Known-good and known-bad run manifests fed to validator → verify correct verdicts |

### 8.4 Milestone Assignment Map

Files from §8.2 and §8.3 mapped to the delivery milestones defined in §11. No files are deleted — files not needed for early milestones are assigned to later milestones.

#### Milestone 0: Gold Set + Evaluation Harness (MVE) — ~25-35 files

**Workspace (partial):**
- `package.json`, `bunfig.toml`, `.env.example`

**Shared Domain (all 4):**
- `contracts-schema/src/contracts.ts`, `jobs.ts`, `confidence.ts`, `zod.ts`

**Skills (5):**
- `skills/document-acquisition/SKILL.md`
- `skills/lot-extraction/SKILL.md`
- `skills/product-extraction/SKILL.md`
- `skills/preliminary-qualification/SKILL.md`
- `skills/supplier-price-enrichment/ebay/SKILL.md`

**Intelligence (12):**
- `intelligence/src/qualification/expiry.ts`, `access-route.ts`, `goods.ts`, `quoteability.ts`
- `intelligence/src/extraction/products.ts`
- `intelligence/src/search/build-search-profile.ts`
- `intelligence/src/customs/hmrc-tariff.ts` (screening-grade only)
- `intelligence/src/margins/revenue-normalizer.ts`, `quote-normalizer.ts`, `margin-scenarios.ts`
- `intelligence/src/evaluation/harness.ts`, `metrics.ts`

**Database (1):**
- `crates/daemon/migrations/0001_sqlite_state_initial.sql` (MVE subset)

**Tests (4):**
- `intelligence/src/qualification/goods.test.ts`
- `intelligence/src/qualification/quoteability.test.ts`
- `intelligence/src/customs/hmrc-tariff.test.ts`
- `intelligence/src/margins/net-margin.test.ts`

**Note:** Evaluation harness skills should produce run manifests (embedded in kickoff prompt instructions) so that validation infrastructure can be developed against real examples from Milestone 0 onward.

#### Milestone 1: Thin Local Runner — ~18 files

**Workspace:**
- `Cargo.toml` (workspace root)

**Database (1):**
- `crates/daemon/migrations/0003_sqlite_artifacts.sql`

**Rust Daemon (core subset — 14):**
- `crates/daemon/Cargo.toml`, `src/main.rs`
- `src/db/sqlite.rs`
- `src/pty/session.rs`, `src/pty/capture.rs`, `src/pty/health.rs`
- `src/jobs/runner.rs`
- `src/agents/adapter.rs`, `src/agents/codex.rs`, `src/agents/claude.rs`
- `src/agents/artifacts.rs`, `src/agents/contracts.rs`
- `src/prompt/renderer.rs`
- `src/errors/manifest.rs`, `src/errors/markdown.rs`

**Tests (7):**
- `crates/daemon/tests/agent_pty_session.rs`
- `crates/daemon/tests/agent_contract_session.rs`
- `crates/daemon/tests/adapter_codex.rs`
- `crates/daemon/tests/adapter_claude.rs`
- `crates/daemon/tests/adapter_result_contract.rs`
- `crates/daemon/tests/skill_lint.rs`
- `crates/daemon/tests/skill_canary_harness.rs`

#### Milestone 2: Single-Operator Desktop MVP — ~25 files

**Skills (3):**
- `skills/procurement-ingestion/fts/SKILL.md`
- `skills/procurement-ingestion/contracts-finder/SKILL.md`
- `skills/manifest-generation/SKILL.md`

**Shared UI (5):**
- `packages/ui/src/index.ts`
- `packages/ui/src/contracts/contract-card-frame.tsx`
- `packages/ui/src/manifest/manifest-viewer.tsx`
- `packages/ui/src/status/pipeline-badge.tsx`
- `packages/ui/src/status/sync-age-badge.tsx`

**Desktop Shell (all 14):**
- All files in §8.2 "Desktop Shell" table

**Manifests (2):**
- `manifests/src/render-opportunity.ts`, `manifests/src/templates/opportunity.md.ts`

**Tests (5):**
- `crates/daemon/tests/agent_procurement_ingestion.rs`
- `crates/daemon/tests/agent_supplier_enrichment.rs`
- `crates/daemon/tests/agent_health_detection.rs`
- `crates/daemon/tests/agent_error_manifest.rs`
- `apps/desktop/e2e/contracts.spec.ts`

#### Milestone 3: Operational Hardening — ~15 files

**Skills (4):**
- `skills/auth-refresh/alibaba/SKILL.md`
- `skills/auth-refresh/1688/SKILL.md`
- `skills/source-health/fts/SKILL.md`
- `skills/source-health/alibaba/SKILL.md`

**Rust Daemon (additions — 12):**
- `src/scheduler/cron.rs`
- `src/sources/registry.rs`
- `src/jobs/claim.rs`
- `src/pty/reaper.rs`, `src/pty/hardening.rs`
- `src/agents/arbiter.rs`
- `src/db/checkpoints.rs`
- `src/tracing/structured.rs`, `src/tracing/forensic.rs`
- `src/validation/policy.rs`, `src/validation/executor.rs`, `src/validation/manifest_writer.rs`

**Skills (additions — 5):**
- `skills/validation/base/SKILL.md`, `skills/validation/base/result.schema.json`, `skills/validation/base/checkpoint.schema.json`, `skills/validation/base/canary.inputs.jsonl`, `skills/validation/base/golden.assertions.yaml` (base validator skill package — dry-run mode)

**Intelligence (addition — 1):**
- `intelligence/src/relations/resolve-process-relations.ts`

**Manifests (addition — 2):**
- `manifests/src/render-error-manifest.ts`, `manifests/src/templates/error.md.ts`

**Tests (16):**
- `crates/daemon/tests/fault_process_cleanup.rs`
- `crates/daemon/tests/fault_pty_edge_cases.rs`
- `crates/daemon/tests/fault_host_events.rs`
- `crates/daemon/tests/fault_partial_failure.rs`
- `crates/daemon/tests/replay_pty_sessions.rs`
- `crates/daemon/tests/replay_contract_sessions.rs`
- `crates/daemon/tests/cron_scheduler.rs`
- `crates/daemon/tests/job_claim.rs`
- `intelligence/src/relations/resolve-process-relations.test.ts`
- `crates/daemon/tests/skill_golden_journeys.rs`
- `crates/daemon/tests/skill_shadow_compare.rs`
- `crates/daemon/tests/skill_drift_monitor.rs`
- `crates/daemon/tests/validation_policy.rs`
- `crates/daemon/tests/validation_executor.rs`
- `crates/daemon/tests/validation_dry_run.rs`
- `crates/daemon/tests/validation_golden_verdicts.rs`

**Validation milestone note:** Validation runs in **dry-run mode** in Milestone 3 — verdicts are logged but do not block worker results. This allows calibrating validator skills and monitoring false positive rates before enabling enforcement.

#### Milestone 4: Shared Visibility — ~15 files

**Database (8):**
- `crates/daemon/migrations/0002_sqlite_state_sync.sql`
- `packages/db/migrations/0001_initial.sql`, `0002_rls.sql`, `0003_sync_apply.sql`, `0004_schema_metadata.sql`, `0005_stage_tracking.sql`, `0006_junction_tables.sql`
- `infra/supabase/config.toml`

**Rust Daemon (addition — 1):**
- `src/db/sync.rs`

**Skills (additions — 4):**
- `skills/validation/procurement-ingestion/SKILL.md`
- `skills/validation/supplier-enrichment/SKILL.md`
- `skills/validation/document-acquisition/SKILL.md`
- `skills/validation/product-extraction/SKILL.md`

**Web Shell (all 7):**
- All files in §8.2 "Web Shell" table

**Tests (3):**
- `crates/daemon/tests/sqlite_schema.rs`
- `crates/daemon/tests/sqlite_supabase_sync.rs`
- `apps/web/e2e/contracts.spec.ts`

**Validation note:** Milestone 4 adds `validation_verdict_summaries` to the Supabase projection tables. The viewing portal can now display validation verdict status (approved/rejected/flagged) alongside job summaries.

**Validation enforcement:** After Milestone 3 dry-run calibration, Milestone 4 enables enforcement mode — validator rejections quarantine results. The four domain-specific validator overlays above are introduced alongside their source-specific validation criteria.

#### Milestone 5: Expanded Commercial Intelligence — ~25 files

**Skills (4):**
- `skills/supplier-price-enrichment/amazon/SKILL.md`
- `skills/supplier-price-enrichment/alibaba/SKILL.md`
- `skills/supplier-price-enrichment/1688/SKILL.md`
- `skills/procurement-ingestion/pcs/SKILL.md`

**Intelligence (19):**
- `customs/hmrc-fpo.ts`, `classification-preprocessing.ts`, `ten-digit-resolver.ts`, `measure-scanner.ts`, `taric-support.ts`, `hs-ping.ts`
- `fx/xe.ts`, `hmrc-exchange-rates.ts`, `currencybeacon.ts`, `oanda.ts`
- `logistics/freightos.ts`, `searates.ts`, `mode-selection.ts`, `easyship.ts`, `shippo.ts`
- `validation/dutify.ts`, `zonos.ts`, `avalara.ts`

**Tests (1):**
- `intelligence/src/customs/hmrc-fpo.test.ts`

#### Milestone 6: Request Pipeline Automation — ~4 files

**Skills (1):**
- `skills/bid-portal-proactis/SKILL.md`

**Bidding (all 3):**
- `bidding/src/workflows/index.ts`, `workflows/proactis-v1.ts`, `company-profile/schema.ts`

---

## 9. Dependencies & Prerequisites

### 9.1 New Dependencies

**Production (TypeScript):**

| Package | Purpose |
|---------|---------|
| `@supabase/supabase-js` | auth, realtime, database access |
| `zod` | runtime validation |
| `react` | UI |
| `next` or `vite` shell dependencies | frontend shell |
| `marked` or equivalent | manifest rendering |
| `playwright` runtime libs where needed | browser automation integration helpers |

**Production (Rust):**

| Package | Purpose |
|---------|---------|
| `tokio` | async runtime |
| `rusqlite` or `sqlx` (SQLite feature) | local SQLite access |
| `sqlx` (Postgres feature) | Supabase/Postgres sync |
| `reqwest` | HTTP clients (cost-estimation APIs) |
| `serde` / `serde_json` | serialization |
| `tracing` | structured logging |
| `chrono` | date/time |
| `anyhow` / `thiserror` | error handling |
| `nix` | POSIX process-group and signal handling |
| `portable-pty` or `pty-process` | PTY session management |
| `cron` | cron expression parsing for local scheduler |
| `sysinfo` | host resource pressure detection for the resource arbiter |
| `uuid` | UUID generation for SQLite (no gen_random_uuid) |
| `sha2` | SHA-256 hashes for artifacts and result contracts |
| `zstd` | compressed transcript and artifact storage |

**Development:**

| Package | Purpose |
|---------|---------|
| `vitest` | unit tests |
| `playwright` | E2E and source workflow tests |
| `typescript` | TS build |
| `turbo` or equivalent | monorepo task orchestration |
| `supabase` CLI | local stack and migrations |

### 9.2 Configuration Changes

| Config File | Changes |
|-------------|---------|
| `.env.example` | Supabase URLs/keys (sync target), Freightos, SeaRates, Xe, Taric Support, optional HS Ping/CurrencyBeacon/OANDA/comparison-provider keys, SQLite path, agent CLI paths |
| `apps/desktop/src-tauri/capabilities/default.json` | allow specific shell/sidecar commands only |
| `apps/desktop/src-tauri/tauri.conf.json` | sidecar and desktop build configuration |
| `infra/supabase/*` | local database and RLS policies |

### 9.3 External Accounts / Access

| Service | Need |
|---------|------|
| Supabase | shared sync target DB/auth/storage |
| HMRC Trade Tariff Developer Portal | tariff API + FPO API access |
| HMRC Exchange Rate Service | customs valuation FX rates (free, no registration needed) |
| Xe | commercial currency API |
| Freightos | freight market pricing access |
| SeaRates | complementary freight API access |
| Easyship | parcel-mode freight and tax/duty estimation (supporting source) |
| Shippo | parcel-mode carrier rates and international shipping (supporting source) |
| Taric Support | commodity-code validation API access |
| HS Ping | optional HS-code comparison access |
| CurrencyBeacon | optional backup FX access |
| OANDA | optional historical FX access |
| Dutify / Zonos / Avalara | optional landed-cost comparison providers |
| Codex/Claude CLI credentials | agent CLI authentication for PTY sessions |
| (No marketplace API accounts needed) | supplier data collected via agent skill scraping, not APIs |

### 9.4 Migration Requirements

**Schema coordination rule:** Both SQLite and Postgres schemas must derive from the same canonical logical schema. CI must apply both migration sets from scratch, introspect both, and fail if the logical schema hash differs.

| Migration | Database | Purpose | Reversible |
|-----------|----------|---------|------------|
| SQLite state initial | SQLite state.db | Pipeline entities (procurement_processes, contract_notices, notice_lots, qualification_runs, product_candidates, search_profiles, search_attempts, supplier_quotes, landed_cost_estimates, manifests, pipeline_assessments, bid_workflow_states, process_relations, process_aliases), local-only runtime/cache tables (`source_runtime_state`, `listing_evidence`), sync tracking columns, junction tables | Yes |
| SQLite sync infra | SQLite state.db | local_outbox, sync_batches, pull_cursors, pending_operator_intents queue, stage tracking (process_stage_status, process_stage_heads) | Yes |
| SQLite artifacts | SQLite artifacts.db | Agent session logs (with orchestration domain columns), raw documents, error manifests | Yes |
| Postgres initial | Supabase | Projection tables (all push-owned entities including notice_lots, process_relations, process_aliases, search_profiles, search_attempts, workflow_job_summaries, error_manifest_summaries, viewer_sync_status), remote-owned tables (shared_company_profile, shared_source_configs, operator_decision_heads) | Yes |
| Postgres RLS | Supabase | Authority-based RLS per role (viewer, operator, admin, daemon) — see §6.3 for policy matrix | Yes |
| Postgres sync apply | Supabase | Transactional RPC for push batch application + operator decision CAS RPC | Yes |
| Postgres schema metadata | Supabase | schema_metadata table with schema_epoch | Yes |
| Postgres stage tracking | Supabase | process_stage_status, process_stage_heads projections | Yes |
| Postgres junction tables | Supabase | notice_cpv_codes, product_commodity_codes, lot_cpv_codes | Yes |

---

## 10. Open Questions

### Requiring User Input

1. **Is Scotland in the MVP ingestion scope?**
   - Context: This determines whether PCS is a first-wave scraping skill or a later phase.
   - Options: England/Wales/NI only, or full UK including Scotland.
   - Recommendation: Include PCS skill in Phase A only if Scotland matters commercially on day one.

2. ~~**Do you want one shared web/desktop shell or separate shells with shared packages?**~~
   - **RESOLVED:** Separate shells with clearly different roles. Desktop = control panel (full read/write/trigger). Viewing portal = read-only.

3. ~~**What level of autonomy is acceptable for bid workflows in MVP?**~~
   - **RESOLVED:** Full automation of the request pipeline is acceptable. This is not real-money bidding — actual negotiations and fulfillment are handled manually by humans. Error manifest system flags failures for debugging.

4. **Should the margin model assume you are importing through a UK VAT-registered entity that can normally recover import VAT?**
   - Context: This materially changes whether import VAT is treated as an economic cost or primarily a cash-flow item.
   - Recommendation: model both views, but set the default based on your real importer-of-record structure.

5. **Which machine is expected to stay online for scheduled execution and freshness SLAs?**
   - Context: The plan now explicitly assumes one active local daemon per organization and catchup sweeps after downtime, but freshness still depends on at least one machine staying online.
   - Recommendation: identify one designated always-on workstation or mini-server-class host for the MVP.

6. ~~**Are browser-automation-based supplier price captures acceptable for high-value opportunities when official APIs are insufficient?**~~
   - **RESOLVED:** Yes — browser automation via agent skills is the primary data retrieval strategy for ALL procurement and supplier data, not just a fallback. This is a core architectural decision.

### Requiring Further Research

1. **Exact procurement source list by commercial importance**
   - Why unresolved: You described the platform broadly, but not which portals matter most beyond government contract sites.
   - Research needed: rank target sources by contract volume, goods density, and bidability.

2. **Product-family-specific logistics assumptions**
   - Why unresolved: shipping mode depends heavily on item class and packaging density.
   - Research needed: define default freight-mode heuristics for your target product families.

3. **Chinese supplier source strategy**
   - Why unresolved: official APIs exist, but practical access and data quality depend on your commercial relationships.
   - Research needed: validate which supplier platforms you can lawfully and reliably integrate.

### Assumptions Made

1. **Assumption:** UK procurement is the primary geography.
   - Impact if wrong: source layer design changes significantly.
   - Validation: confirm MVP geography.

2. **Validated:** Desktop-local execution is non-negotiable.
   - Evidence: resolved owner feedback and integrated ADR-003/ADR-004 architecture.
   - Remaining concern: confirm what machine is expected to stay online for freshness SLAs.

3. **Assumption:** Margin thresholds are business gates, not just UI labels.
   - Impact if wrong: workflow state design may be too rigid.
   - Validation: confirm whether 30% gross and 20% net are hard filters.

4. **Assumption:** Import VAT is usually recoverable because the operating importer is UK VAT-registered.
   - Impact if wrong: economic-margin modeling could materially overstate opportunity quality.
   - Validation: confirm importer-of-record and VAT-registration structure.

5. **Assumption:** Company profile data can be centrally stored with appropriate protections.
   - Impact if wrong: bid workflows need stronger local-only secret partitioning.
   - Validation: confirm data residency and storage comfort level.

---

## 11. Next Steps

### Validation-First Delivery Philosophy

Delivery is sequenced by **risk retirement**, not architectural completeness. The riskiest assumptions are in the intelligence pipeline — can the system extract usable product candidates, match them to real marketplace listings, and produce margin estimates accurate enough to act on? Infrastructure (daemon orchestration, cron, sync, web) is well-understood systems engineering that follows, not leads.
This ordering follows the complexity and risk profile in §§6.1–6.2: validate the very-high-complexity intelligence and matching chain before scaling the lower-risk operational shell around it.

A **Minimum Viable Experiment (MVE)** validates the intelligence thesis before infrastructure investment. The MVE proves the product can surface high-precision, non-fake opportunities on a curated lot corpus. Only after the MVE demonstrates credible signal does infrastructure scale it.

Wrong answers at scale are worse than no automation. Building a reliable daemon that produces unreliable intelligence is wasted work. Building credible intelligence first means every infrastructure component has a clear reason to exist.

### Core Validation Order

Four experiments, run in sequence. Each must pass before the next is worth pursuing.

| # | Experiment | Pass Criteria | What It Proves |
|---|-----------|--------------|----------------|
| 1 | **Extraction quality** — Can we pull structured lot/product candidates from real procurement documents? | ≥80% of lots in the gold set yield at least one usable product candidate with correct unit, quantity, and specification | The system can read procurement intent |
| 2 | **Matching quality** — Can marketplace search find relevant supplier listings for extracted candidates? | ≥70% of product candidates with a `matchArchetype` produce at least one genuine match (not a false positive from keyword overlap) | The system can connect demand to supply |
| 3 | **Margin accuracy** — Can screening-grade landed cost produce margin bands that distinguish real opportunities from noise? | Margin bands agree with human expert assessment on ≥75% of gold-set lots (within ±5pp for screening-grade) | The system produces actionable commercial signal |
| 4 | **Runtime reliability** — Can the thin runner execute a batch of skills without manual intervention? | 10 consecutive batch runs complete with ≥90% skill success rate and no orphaned processes | The system can operate unattended |

### Minimum Viable Experiment (MVE)

The MVE is ~25-35 files proving experiments 1-3 on a curated lot corpus. No daemon, no cron, no desktop UI, no viewing portal, no Supabase sync. Skills run from CLI. Results land in local SQLite. Evaluation is via test harness and manual inspection.

**MVE scope:**
- Gold-set lot corpus (50-100 curated lots with human labels)
- Document acquisition for gold-set notices
- Lot extraction with product candidate generation
- `matchArchetype` assignment and slot criticality scoring
- Quoteability scoring with `recallRiskScore`
- One marketplace search skill (eBay or Amazon) with query generation
- Search-card screening with four-state evidence comparison
- Screening-grade landed-cost bands (simplified duty + freight + FX)
- Evaluation harness comparing system output to human labels
- Local SQLite for results (no sync, no projections)

**MVE explicitly excludes:**
- Rust daemon (skills run via CLI)
- Cron scheduling (manual trigger)
- Desktop or web UI (CLI + test harness)
- Supabase sync
- Relation resolution (gold-set lots are pre-curated)
- Multiple procurement sources (gold set is pre-selected)
- Full landed-cost engine (screening-grade only)
- Bid workflow automation

### Category Scope

Start with a narrow goods-category whitelist: commodity-like products with low regulatory burden, high marketplace availability, and straightforward landed-cost estimation.

**Initial whitelist:** office supplies, cleaning products, basic PPE, hand tools, basic furniture, packaging materials.

**Explicit no-list (defer until system proves reliable on whitelist):** medical devices, pharmaceuticals, IT hardware (complex specifications), vehicles, construction materials, food/perishables, chemicals (regulatory burden), bespoke/custom-manufactured goods.

The whitelist expands only after the system demonstrates reliable extraction, matching, and margin estimation on the initial categories. Each new category requires its own gold-set validation before entering production.

### MVP-Alpha Definition

After MVE validation, the MVP-Alpha proves repeatable operator value with a single operator running the system daily. It adds infrastructure around the validated intelligence pipeline.

**MVP-Alpha scope (in addition to MVE):**
- Thin Rust runner (agent launch, artifact capture, process supervision — not full daemon)
- Local SQLite with operational schema
- Manual-trigger desktop UI (Tauri) with list/detail review, QueryService + DaemonBridge + bootstrap state machine
- Decision capture (reject/watch/shortlist)
- FTS ingestion skill (live notices, not just gold set)
- Batched ingestion (Contracts Finder as second source)
- Basic health monitoring and error manifests

### Recommended Delivery Order

#### Milestone 0: Gold Set + Evaluation Harness (MVE)

Prove the intelligence thesis on a curated lot corpus before building infrastructure.

- Establish skill authoring guide (8-question template) and production skill directory standard (SKILL.md + result.schema.json + checkpoint.schema.json + canary.inputs.jsonl + golden.assertions.yaml) before writing first production skill
- Curate gold-set lot corpus (50-100 lots across whitelist categories, with human labels for product candidates, expected matches, and margin bands)
- Seed bilingual lexicon for top 30-50 spend-heavy product families (EN/ZH noun aliases, market framing terms, negative terms, unit aliases)
- Document acquisition skill (PDF/attachment download from notice URLs)
- Lot extraction with product candidate generation
- `matchArchetype` assignment and slot criticality scoring
- Structured standard normalization (family/part/edition/amendment/class)
- Quoteability scoring with `recallRiskScore`
- One marketplace search skill (category-prior search with alias-family query generation)
- Search-card screening with four-state evidence comparison
- Detail-page enrichment and compliance evidence tier extraction
- Screening-grade landed-cost bands (duty via HMRC Tariff API, flat-rate freight estimate, single FX rate)
- Evaluation harness: lot-level precision/recall against gold-set labels
- Recall measurement instrumentation (recall@k by archetype × source)
- Local SQLite for intermediate results
- All skills run from CLI (no daemon)

**Exit criteria:** Experiments 1-3 pass on gold-set corpus.

#### Milestone 1: Thin Local Runner

Minimal Rust runtime — just enough to launch skills, capture artifacts, and supervise processes. PTY is available as a capability, not a full daemon platform.

- Rust binary with skill-launch command (PTY session for interactive skills, direct exec for deterministic skills)
- Process-group supervision with cleanup verdicts
- Artifact capture (session bytes to compressed files, not inline in SQLite)
- Local SQLite operational schema (sessions, artifacts, errors)
- Error manifest system
- Agent health monitoring and log capture
- CLI interface for manual skill triggering
- Skill linting CI gate (validate directory structure, metadata, schemas, trigger boundaries on every skill change)
- Smoke canary harness (page fingerprint check, auth verification, minimal extraction for each critical source)

**Exit criteria:** Experiment 4 passes (10 consecutive batch runs, ≥90% success rate).

#### Milestone 2: Single-Operator Desktop MVP

Prove repeatable operator value with manual-trigger desktop and live procurement data.

- FTS ingestion skill (Find a Tender)
- Contracts Finder ingestion skill
- Notice normalization and lot-level data model
- Goods/services classifier and expiry filter
- Adaptive diversified shortlisting with diversity enforcement
- Zero-result recovery ladder and failure-mode classification
- Query-wave planning for multi-source search
- Net-margin view with screening-grade landed cost
- Basic Tauri desktop shell (control panel) with QueryService + DaemonBridge + bootstrap state machine
- Notice list/detail views with keyset pagination
- Agent session monitoring page with live terminal viewer and session stream stores
- Opportunity list with margin bands
- Decision capture (reject/watch/shortlist) with decision ledger
- Feature flags for progressive capability enablement
- Agent E2E tests verifying real skill sessions

**Exit criteria:** Single operator runs system daily for one week, makes real accept/reject decisions, reports the pipeline surfaces genuine opportunities.

#### Milestone 3: Operational Hardening

Daily automation without babysitting.

- Daemon cron scheduler with explicit skill binding
- Batched ingestion (scheduled FTS + CF runs)
- Relation resolution (notice → parent-notice → framework linkage)
- PTY hardening (fd hygiene, EIO handling, terminal geometry)
- Host environment awareness (sleep/wake detection, catchup sweeps)
- Two-tier cache (search-result + listing-evidence)
- Resource arbiter with typed quotas (browser slots, agent slots, source lanes)
- Job history and agent session tracing in desktop
- Error manifest control-panel view
- Pipeline cost tracking and budget controls
- Shadow mode: system runs alongside operator decisions, compares recommendations to human choices
- Skill promotion flow (draft → candidate → canary → shadow → stable → rollback) with version binding at enqueue
- Source-health canary skills (pre-flight reachability, geo, auth, page fingerprint checks before scheduled jobs)
- Auth-refresh skills for authenticated sources (Alibaba, 1688)
- Production drift monitoring (page fingerprint hash, locator fallback depth, fill-rate, duplicate-rate trending)
- Golden live journey tests per skill family (invariant-based, not brittle literal matching)
- **Worker-validator validation layer (dry-run mode):** Run manifest generation in all skills (embedded in kickoff prompts), base validator skill for generic semantic quality checks, validation policy engine, dry-run mode for initial deployment (verdicts logged but not enforced). Calibrate validator false positive rate before enabling enforcement.

**Exit criteria:** System runs unattended for 5 consecutive days with ≥95% skill success rate and no manual intervention required.

#### Milestone 4: Shared Visibility

Collaboration infrastructure.

- Supabase schema and sync engine
- Transactional outbox push (local → remote)
- Cursor-based pull (remote → local) with CAS for shared decisions
- Projection tables for web consumption (including `validation_verdict_summaries`)
- **Validation enforcement mode:** Validator rejections quarantine results. Domain-specific validator skills for procurement ingestion, supplier enrichment, document acquisition, and product extraction. Validation trend dashboard in desktop. Auto-rollback on consecutive rejections as a promotion gate.
- Viewing portal (read-only, reading from Supabase) with Broadcast invalidation and staleness rendering
- Multi-user decision visibility

**Exit criteria:** Two users can view the same opportunity pipeline, one via desktop (read-write) and one via web (read-only).

#### Milestone 5: Expanded Commercial Intelligence

Broader categories, more sources, decision-grade accuracy.

- Additional marketplace search skills (Alibaba/1688 with Chinese keyword clusters, Amazon)
- HMRC FPO API for HS code classification
- HMRC Exchange Rate Service integration (dual-rate: spot + HMRC monthly)
- Xe commercial FX integration
- Freightos + SeaRates freight API integration
- Decision-grade landed cost per validated category (replacing screening-grade)
- Category-specific gold-set expansion and validation
- Expanded bilingual lexicon coverage

**Exit criteria:** System handles ≥3 goods categories with decision-grade landed cost and ≥2 supplier sources per category.

#### Milestone 6: Request Pipeline Automation

Portal-specific bid skills.

- One portal-specific bid skill (Find a Tender or Contracts Finder)
- Agent-browser request pipeline flow
- Screenshot/log artifact capture for bid workflows
- Full terminal output tracing for bid workflows
- Manifest generation for submitted requests

**Exit criteria:** System can prepare and submit a request on one portal with full audit trail.

### Stack Sequencing

Build stacks in order of risk retirement, not in parallel:

1. **Skills first** — SKILL.md playbooks for document acquisition, lot extraction, marketplace search. Skills are the cheapest to write, the fastest to iterate, and they validate the intelligence thesis directly. Run from CLI.
2. **TypeScript intelligence** — Pipeline logic (extraction, matching, scoring, landed cost), evaluation harness, data model types, SQLite integration. This is where the product thesis lives.
3. **Thin Rust** — Minimal runner for skill launch, artifact capture, process supervision. Kept deliberately thin until the intelligence pipeline is validated.
4. **Desktop UI** — Tauri shell with list/detail views, decision capture. Only after the pipeline produces results worth displaying.
5. **Web + infrastructure** — Supabase sync, viewing portal, cron, operational hardening. Only after a single operator has validated daily use.

### MVE Testing Approach

Four testing lanes, each serving a different feedback loop:

| Lane | Scope | Trigger | Cost | Purpose |
|------|-------|---------|------|---------|
| **Fast inner loop** | Deterministic logic: extraction parsers, margin math, scoring functions, data model validation | Every save / pre-commit | Near zero (no network, no LLM) | Catch regressions instantly |
| **Live smoke** | One skill against one real source, one lot through the pipeline | Per-change (manual or CI) | Low (~$0.10-0.30 per run) | Verify the pipeline still produces real results |
| **Benchmark batch** | Full gold-set corpus through the pipeline, compare to human labels | Periodic (daily or per-milestone) | Medium (~$2-10 per run) | Track precision/recall trends, catch quality drift |
| **Release gate** | All skills, full pipeline, extended corpus | Pre-milestone release | Higher (~$20-50 per run) | Confidence that a milestone is shippable |

The "no mocks" principle applies to the **live smoke** and **release gate** lanes. The **fast inner loop** tests deterministic functions that don't need mocks — they operate on concrete data structures. The **benchmark batch** uses real pipeline components against the gold set.

### Validation Techniques

**Gold set:** A curated corpus of 50-100 lots with human-labeled product candidates, expected marketplace matches, and margin assessments. The gold set is the ground truth for all quality metrics. It must be maintained as categories expand.

**Lot-level measurement:** The lot is the minimum commercial unit. All precision/recall metrics are measured per-lot, not per-notice. A notice with 12 lots where 11 are irrelevant and 1 is gold — the system should find the 1, not report "notice matched."

**Precision over recall:** For MVE, false positives (fake opportunities) are more damaging than false negatives (missed opportunities). The system should abstain rather than guess. An explicit abstain path ("not enough evidence to score") is better than a low-confidence score that looks like a real answer.

**Shadow mode:** After Milestone 2, run the system alongside operator decisions. Compare system recommendations to human choices. Track agreement rate, false positive rate, and cases where the system found opportunities the operator missed.

**Decision ledger:** Every operator accept/reject/watch decision is stored with the system's recommendation at time of decision. This creates a training signal for future calibration and a quality audit trail.

**Feature flags:** New capabilities (additional categories, new supplier sources, upgraded cost models) are deployed behind flags. They run in shadow mode before becoming the default. This prevents quality regressions when expanding scope.

**Raised auto-pass bar:** As the gold set grows and confidence metrics stabilize, raise the threshold for what counts as "passing." Early MVE: ≥70% lot-level precision. Mature system: ≥85%.

### Recommended Pipeline

```text
This Document
  -> MVE implementation (Milestone 0, ~25-35 files)
  -> MVE validation against gold set
  -> research-to-spec (informed by MVE learnings)
  -> milestone execution (1-6, spec-driven)
```

### Pre-requisites for Implementation

- [ ] Gold-set lot corpus curated (50-100 lots across whitelist categories, with human labels)
- [ ] Scope geography confirmed (Scotland in/out of MVP)
- [x] Desktop/web shell decision made — desktop = control panel, viewing portal = read-only
- [ ] Initial procurement sources ranked by commercial importance
- [ ] Bilingual lexicon seeded for top 30-50 product families
- [x] Data retrieval strategy confirmed — skill-based agent scraping for procurement + supplier data
- [x] Cost-estimation API strategy confirmed — HMRC Tariff + FPO + Exchange Rates, Freightos + SeaRates, Xe
- [x] Agent orchestration strategy confirmed — adapter-mediated hybrid runtime with PTY as a first-class capability from day 1
- [x] Risk policy for automation confirmed — request pipeline automation, not real-money bids
- [ ] Codex/Claude CLI credentials set up for agent sessions
- [ ] VAT entity structure confirmed for margin modeling
- [ ] Category whitelist validated (confirm initial goods categories are viable for arbitrage)
- [ ] Supabase project created (needed by Milestone 4, not before)

---

## Appendices

### A. Suggested Margin Logic

#### Margin Gate Overview

#### Revenue Normalization (Prerequisite)

Before any margin gate, normalize the revenue basis:

```text
revenue_basis_type = classify(notice):
  "firm_contract"       -> direct award or single-winner tender with firm value
  "estimated_contract"  -> tender with estimated (not guaranteed) value
  "framework_ceiling"   -> framework agreement ceiling (not guaranteed revenue)
  "dynamic_market"      -> DPS/dynamic market admission (no volume guarantee)
  "concession"          -> concession contract
  "unknown"             -> insufficient data

# Under Procurement Act guidance, estimated contract value is the maximum value
# payable and is inclusive of VAT. Framework ceilings are not guaranteed revenue.
expected_addressable_revenue_ex_vat = normalize(
  raw_notice_value,
  vat_inclusive = notice.estimatedValueVatInclusive,
  basis_type = revenue_basis_type
)

revenue_confidence = {
  "firm_contract": 0.90, "estimated_contract": 0.70,
  "framework_ceiling": 0.40, "dynamic_market": 0.30,
  "concession": 0.50, "unknown": 0.20
}[revenue_basis_type]
```

**Reference:** https://www.gov.uk/government/publications/procurement-act-2023-guidance-documents-define-phase/guidance-valuation-of-contracts-html

#### Quote Normalization (Prerequisite)

Normalize all supplier quotes to a common cost boundary before comparison:

```text
# Incoterm determines what is already included in the supplier price.
# EXW: buyer pays everything from factory gate
# FOB: seller pays inland transport to port + export clearance
# CIF: seller pays freight + insurance to destination port
# DDP: seller pays everything including import duty + VAT
# Without normalization, comparing EXW to CIF quotes is inconsistent.

normalized_supplier_cost = normalize_to_exw(
  quoted_price, incoterm = quote.quoteCostBoundary,
  estimated_inland_freight, estimated_ocean_freight, estimated_insurance
)
```

**Reference:** https://iccwbo.org/business-solutions/incoterms-rules/

#### Gross Margin Gate

Use normalized ex-VAT revenue and normalized supplier cost:

```text
gross_margin = (expected_addressable_revenue_ex_vat - normalized_supplier_cost)
             / expected_addressable_revenue_ex_vat
gross_estimate_confidence = min(revenue_confidence, quote_realism_confidence)

if gross_margin >= 0.30 and gross_estimate_confidence >= confidence_floor:
  qualify_for_landed_cost_stage
else if gross_estimate_confidence < confidence_floor:
  route_to_manual_review
else:
  reject_or_deprioritize
```

#### Net Margin Gate

Use an ex-VAT economic margin view with **low/base/high scenarios**. Do not blindly subtract recoverable import VAT from economic margin if the importing entity is VAT-registered and can reclaim it.

**Note on duty scenarios:** The `customs_duty_high/base/low` values below are computed at
margin-calculation time from classification uncertainty (candidate codes with different duty
rates) and trade remedy risk. The stored `LandedCostEstimate` entity records `customsDutyGbp`
(base estimate) and `tradeRemedyDutyGbp` separately; the scenarios are derived and stored as
final `netMarginLowPct`/`netMarginBasePct`/`netMarginHighPct` values.

```text
net_margin_low = (
  expected_addressable_revenue_ex_vat
  - normalized_supplier_cost * (1 + negotiation_uplift_pct)
  - freight_high
  - customs_duty_high   # includes trade remedy duties if applicable
  - uk_final_mile - other_fees
) / expected_addressable_revenue_ex_vat

net_margin_base = (
  expected_addressable_revenue_ex_vat
  - normalized_supplier_cost
  - freight_base - customs_duty_base
  - uk_final_mile - other_fees
) / expected_addressable_revenue_ex_vat

net_margin_high = (
  expected_addressable_revenue_ex_vat
  - normalized_supplier_cost * (1 - volume_discount_pct)
  - freight_low - customs_duty_low
  - uk_final_mile - other_fees
) / expected_addressable_revenue_ex_vat

cash_to_clear = (
  normalized_supplier_cost + freight_base + customs_duty_base
  + uk_final_mile + other_fees
  + import_vat * (1 - vat_recovery_rate)    # non-recoverable portion
  + import_vat * vat_recovery_rate           # recoverable but ties up cash
)

# Optional: working-capital-adjusted margin
working_capital_cost = cash_to_clear * cost_of_capital * (days_cash_tied / 365)
wc_adjusted_margin = net_margin_base
  - (working_capital_cost / expected_addressable_revenue_ex_vat)

# Gate logic: use CONSERVATIVE (low) estimate for auto-qualification
if net_margin_low >= 0.20 and no critical_blockers:
  final_qualified
else if net_margin_base >= 0.20 and (quote_class == "indicative_marketplace"
                                     or has_blockers):
  manual_review_required / rfq_required
else:
  not_qualified

# Critical blockers override margin regardless of value:
critical_blockers = [
  classification_unstable,        # HS6 diverges or 10-digit unresolved
  trade_remedy_detected,          # anti-dumping/countervailing/safeguard
  measure_condition_unsatisfied,  # operational blocker
  revenue_basis_uncertain,        # framework ceiling or dynamic market
  pva_ineligible,                 # cannot recover import VAT
]
```

### B. Suggested Confidence Model

Use a weighted confidence score as an internal heuristic only. The weights below are priors for MVP and should be recalibrated against observed operator decisions and realized sourcing outcomes rather than treated as statistically validated probabilities.

```text
final_confidence =
  (0.20 * classification_stability) +
  (0.20 * quote_realism) +
  (0.20 * commodity_code_confidence) +
  (0.15 * freight_mode_stability) +
  (0.15 * revenue_reliability) +
  (0.10 * fx_freshness)
```

This composite applies only after a candidate has already passed the upstream pipeline gates in §5.7 (document acquisition, lot extraction, quoteability, and price-signal matching). Those earlier stages determine whether the system has enough evidence to compute a commercial-confidence score at all; they are not flattened into the final weighted number.

Then apply:

- `>= 0.80` -> high confidence
- `0.60 - 0.79` -> usable with review
- `< 0.60` -> manual review required

**Critical blocker override:** A high composite score MUST NOT hide a red classification, revenue, or trade-remedy blocker. If any critical blocker is present (see Appendix A critical_blockers list), the opportunity cannot auto-qualify regardless of the composite score.

**Operator display:** Do not show operators a single decimal confidence number (e.g., `0.72`). Instead, show:
- **Band:** high / usable / manual review
- **Component panel** with individual scores for:
  - Classification stability (are HS6 candidates converging? is 10-digit resolved?)
  - Quote realism (marketplace indicative vs distributor vs negotiated?)
  - Freight mode stability (is the mode resolved? is packaging state known?)
  - Revenue reliability (firm contract vs framework ceiling vs dynamic market?)
  - FX freshness (how old is the rate data? what is the basis gap?)
- **Blockers:** Any critical blockers that prevent auto-qualification, shown prominently

Keep the simple heuristic internally for pipeline routing, but surface bands, component scores, and blockers in the UI — not fake decimal precision.

For implementation, `gross_estimate_confidence` and `net_estimate_confidence` should be stage-specific projections of the same confidence framework. A reasonable initial `confidence_floor` for automated progression is `0.60`, but this should be calibrated against operator outcomes.

### C. Suggested Job Kinds

| Job Kind | Purpose |
|----------|---------|
| `sync_source` | pull procurement notices (one session per source per run) |
| `normalize_notice` | normalize into internal schema |
| `resolve_relations` | classify notice-to-process relations (`merged_same_process` / `related_not_merged` / `possible_duplicate` / `separate`) |
| `qualify_notice` | expiry + access route + multi-dimensional goods classification |
| `acquire_documents` | download public tender documents (spec PDFs, BoQ, lot docs) |
| `extract_lots` | extract lot structure from notices and documents |
| `extract_products` | per-lot product candidate extraction with candidateType and structured slots |
| `score_quoteability` | deterministic quoteability gate before marketplace search |
| `enrich_supplier_prices` | price-signal matching via marketplace scraping (batched per source per notice) |
| `estimate_landed_cost` | customs/logistics/FX with margin bands |
| `generate_manifest` | Markdown summary |
| `run_bid_workflow` | portal-specific bid assistance |
| `escalate_rfq` | escalate to RFQ for high-value opportunities with indicative-only pricing |

### D. Source References

#### Procurement Data

- Find a Tender data and API docs: https://www.find-tender.service.gov.uk/Developer/Documentation
- Find a Tender OCDS release packages: https://www.find-tender.service.gov.uk/apidocumentation/1.0/GET-ocdsReleasePackages
- Find a Tender OCDS record packages: https://www.find-tender.service.gov.uk/apidocumentation/1.0/GET-ocdsRecordPackages
- Find a Tender notice types: https://www.find-tender.service.gov.uk/Home/NoticeTypes
- Central Digital Platform factsheet: https://www.gov.uk/government/publications/procurement-act-2023-short-guides/central-digital-platform-factsheet-html
- Central Digital Platform guidance: https://www.gov.uk/government/publications/procurement-act-2023-guidance-documents-procure-phase/guidance-central-digital-platform-and-publication-of-information-html
- Contracts Finder API help: https://www.contractsfinder.service.gov.uk/apidocumentation
- Contracts Finder OCDS search: https://www.contractsfinder.service.gov.uk/apidocumentation/Notices/1/GET-Published-Notice-OCDS-Search
- Contracts Finder daily CSV: https://www.contractsfinder.service.gov.uk/apidocumentation/Notices/1/GET-Harvester-Notices-Data-CSV-Daily
- UK OCDS profile note: https://www.gov.uk/government/publications/open-standards-for-government/open-contracting-data-standard-profile
- PCS API: https://api.publiccontractsscotland.gov.uk/v1

#### Customs / Tariff / VAT

- Trade Tariff developer portal: https://hub.trade-tariff.service.gov.uk/
- Trade Tariff API getting started: https://api.trade-tariff.service.gov.uk/getting-started.html
- Trade Tariff Commodities API: https://api.trade-tariff.service.gov.uk/the-commodities-api.html
- Trade Tariff service: https://www.gov.uk/trade-tariff
- Prepare customs value: https://www.gov.uk/guidance/prepare-to-work-out-the-customs-value-of-your-imported-goods
- Method 1 valuation: https://www.gov.uk/guidance/valuing-imported-goods-using-method-1-transaction-value
- VAT from customs value: https://www.gov.uk/guidance/working-out-the-vat-value-using-the-customs-value-of-the-imported-goods
- Advance Tariff Ruling: https://www.gov.uk/guidance/apply-for-an-advance-tariff-ruling

#### Supplier Pricing

- eBay Browse API: https://developer.ebay.com/api-docs/buy/static/api-browse.html
- eBay Browse overview: https://developer.ebay.com/api-docs/buy/browse/static/overview.html
- eBay getItem: https://developer.ebay.com/api-docs/buy/browse/resources/item/methods/getItem
- eBay release notes: https://developer.ebay.com/api-docs/buy/browse/release-notes.html
- Amazon PA-API docs: https://webservices.amazon.com/paapi5/documentation/
- Amazon PA-API deprecation/register page: https://webservices.amazon.com/paapi5/documentation/register-for-pa-api.html
- Amazon Associates term changes / Creators API transition: https://affiliate-program.amazon.com/help/operating/compare
- Taobao Global Open Platform: https://open.taobao.global/doc/doc.htm
- Taobao Global API reference: https://open.taobao.global/doc/api.htm
- Alibaba Open API intro: https://developer.alibaba.com/docs/doc.htm?articleId=118416&docType=1&treeId=684

#### Freight / FX / Platform

- Freightos APIs: https://developers.freightos.com/apis
- Freightos price stats: https://apidocs.freightos.com/reference/get_price-stats
- Freightos price stats detail: https://apidocs.freightos.com/reference/getpricestats-1
- SeaRates Logistics Explorer API: https://docs.searates.com/reference/logistics/get-rates
- Easyship tax and duty: https://developers.easyship.com/reference/taxes_and_duties_calculate
- Shippo docs: https://docs.goshippo.com/
- Shippo public API: https://docs.goshippo.com/shippoapi/public-api
- Shippo international shipping: https://docs.goshippo.com/docs/international_shipping/internationalshipping
- XE Currency Data API landing page: https://www.xe.com/en-us/xecurrencydata/
- XE API overview: https://help.xe.com/hc/en-gb/articles/4411624652177-What-is-the-Currency-Data-API
- XE API behavior: https://help.xe.com/hc/en-gb/articles/4411633342225-How-does-the-Currency-Data-API-work
- XE API packaging: https://help.xe.com/hc/en-gb/articles/4414092026769-Currency-Data-API-packages-pricing-and-payment
- HMRC Exchange Rate Service: https://trade-tariff.service.gov.uk/exchange_rates/monthly
- HMRC Exchange Rate XML API: https://developer.service.hmrc.gov.uk/api-documentation/docs/api/xml/Exchange%20rates%20from%20HMRC
- CurrencyBeacon: https://currencybeacon.com
- OANDA Historical Currency Converter: https://fxds-hcc.oanda.com

#### HS Code Classification

- HMRC FPO API: https://api.trade-tariff.service.gov.uk/fpo.html
- HMRC FPO source code: https://github.com/trade-tariff/trade-tariff-lambdas-fpo-search
- HS Ping: https://hsping.com
- Taric Support API: https://taricsupport.com/api/

#### Third-Party Landed Cost Services

- Dutify: https://docs.dutify.com/docs/landed-cost-calculator
- Zonos: https://zonos.com
- Avalara: https://avalara.com

#### Customs Valuation / VAT / Trade Remedies / Special Procedures

- PVA eligibility: https://www.gov.uk/guidance/check-when-you-can-account-for-import-vat-on-your-vat-return
- Customs valuation Method 1: https://www.gov.uk/guidance/valuing-imported-goods-using-method-1-transaction-value
- Import VAT value from customs value: https://www.gov.uk/guidance/working-out-the-vat-value-using-the-customs-value-of-the-imported-goods
- Trade remedies introduction: https://www.gov.uk/government/publications/the-uk-trade-remedies-investigations-process/introduction-to-trade-remedies
- Inward Processing rules: https://www.gov.uk/guidance/special-procedure-inward-processing/rules-and-processes
- Origin proof requirements: https://www.gov.uk/guidance/get-proof-of-origin-for-your-goods
- Advance Tariff Ruling: https://www.gov.uk/guidance/apply-for-an-advance-tariff-ruling

#### Procurement Act / Contract Valuation

- Contract valuation guidance: https://www.gov.uk/government/publications/procurement-act-2023-guidance-documents-define-phase/guidance-valuation-of-contracts-html

#### Incoterms

- ICC Incoterms rules: https://iccwbo.org/business-solutions/incoterms-rules/

#### Runtime / Database / Skills

- Tauri Next guide: https://v2.tauri.app/start/frontend/nextjs/
- Tauri frontend guidance: https://v2.tauri.app/start/frontend
- Tauri shell plugin: https://v2.tauri.app/plugin/shell
- Tauri sidecar docs: https://v2.tauri.app/develop/sidecar/
- Tauri plugin permissions: https://v2.tauri.app/learn/security/using-plugin-permissions/
- Supabase architecture: https://supabase.com/docs/guides/getting-started/architecture
- Supabase RLS: https://supabase.com/docs/guides/database/postgres/row-level-security
- Supabase realtime changes: https://supabase.com/features/realtime-postgres-changes
- Supabase realtime auth: https://supabase.com/docs/guides/realtime/authorization
- Convex docs: https://docs.convex.dev/home
- Convex realtime: https://docs.convex.dev/realtime
- Convex auth: https://docs.convex.dev/auth
- Convex scheduling: https://docs.convex.dev/scheduling
- SpacetimeDB Maincloud: https://spacetimedb.com/docs/deploying/maincloud/
- SpacetimeDB self-hosting: https://spacetimedb.com/docs/deploying/spacetimedb-standalone
- SpacetimeDB auth: https://spacetimedb.com/docs/spacetimeauth
- OpenAI Codex skills: https://developers.openai.com/codex/skills/
- Claude agent skills: https://platform.claude.com/docs/en/agents-and-tools/agent-skills/overview
- Agent-browser skill reference: https://github.com/vercel-labs/agent-browser/blob/main/skills/agent-browser/SKILL.md
- Playwright locators (tiered locator strategy reference): https://playwright.dev/docs/locators
- Playwright best practices: https://playwright.dev/docs/best-practices

### E. Change Log

| Date | Author | Change |
|------|--------|--------|
| 2026-03-08 | idea-to-research | Initial research document |
| 2026-03-11 | harmonymaxer (Run 001) | Harmonized owner feedback: skill-first agent scraping, PTY-first execution, local SQLite + Supabase sync, desktop control panel + web viewing portal, expanded cost-estimation sources (HMRC FPO, SeaRates, HMRC Exchange Rates, CurrencyBeacon, OANDA, HS Ping, Taric Support, Dutify, Zonos, Avalara), real E2E testing, comprehensive agent tracing, error manifest system, relaxed automation policy. Revised ADR-001, ADR-002, ADR-004, ADR-005, ADR-007, ADR-008. |
| 2026-03-12 | planmaxer pre-prep (Agent Orchestration) | Integrated expert review: hybrid PTY + contract execution model via adapter boundary, three-layer observability (control/protocol/forensic), process-group supervision with cleanup verdicts, explicit skill binding for cron, skill metadata declarations, artifact store for session bytes outside SQLite, resource arbiter with typed quotas, expanded error taxonomy (AUTH_CHALLENGE, OUTPUT_CONTRACT_INVALID, PROTOCOL_MISMATCH, PARTIAL_FAILURE, CLEANUP_INCOMPLETE, HOST_SLEEP_INTERRUPTED, HOST_RESOURCE_EXHAUSTED), deterministic replay + fault-injection + adapter contract tests. Updated ADR-004, ADR-005. Sections modified: 4.9, 4.10, 5.2, 5.4 (note), 5.6, 5.11, 6.5, 6.6, 8.2, 8.3. |
| 2026-03-12 | planmaxer pre-prep (procurement pipeline) | Integrated expert review feedback on procurement pipeline and product matching. Major changes: (1) Four-way relation resolution replacing binary merge (merged_same_process / related_not_merged / possible_duplicate / separate) with hard blockers and conservative heuristics in §5.6. (2) Multi-dimensional goods classification (legalCategory, goodsDensityScore, goodsSeparability, accessRoute) replacing single goods/services label in §5.7. (3) New pipeline stages: document acquisition, lot extraction, product extraction with candidateType, quoteability scoring, price-signal matching with source routing in §5.3/§5.7. (4) NoticeLot entity, ProcessRelation edges, SearchProfile entity, expanded ProductCandidate and SupplierQuote fields noted for §5.4 data-architecture domain. (5) Dual confidence scoring (matchConfidence + quoteUsabilityConfidence) and price-signal role classification in §4.3/§5.7. (6) Cross-language matching strategy (translate slots not prose) in §4.3. (7) Pipeline cost management and throughput optimization in §5.7. (8) New risks in §6.2: attachment blindness, lot blindness, false bidability, compliance false positives, pack-basis errors, price-signal false precision, FPO access risk. (9) Updated complexity analysis in §6.1. (10) New skills: document-acquisition, lot-extraction, product-extraction in §4.10. (11) Updated state model in §5.5 and job kinds in Appendix C. Sections modified: 4.1, 4.2, 4.3, 4.10, 5.2, 5.3, 5.4 (notes), 5.5, 5.6, 5.7, 6.1, 6.2, Appendix C, Appendix E. |
| 2026-03-12 | planmaxer (landed-cost review) | Integrated expert review: FPO reframed as candidate generator, 10-digit code resolution + measure scanning, classification preprocessing pipeline, Incoterm normalization, revenue basis normalization, low/base/high margin scenarios with critical blockers, expanded VAT treatment (PVA eligibility), dual FX with basis gap, freight mode decision tree, special procedures, screening/decision grade distinction, confidence model with blocker override, 12 new error codes. Updated 4.4, 4.5, 4.6, 5.2, 5.4 (data model + SQL), 5.7, 5.11, A, B, D, ADR-006. |
| 2026-03-12 | planmaxer steps 3–4 (landed-cost) | Fresh-eyes optimization + sync pass: added missing pipeline steps (ukFinalMileCostGbp, otherFeesGbp, customsValuationMethod, confidenceBreakdown population), updated 5.4 SQL landed_cost_estimates table to match TypeScript interface (27 new/renamed columns), added Easyship and Shippo to 9.3 External Accounts, added cross-domain coordination note for ProductCandidate → freight mode data flow, added 8 new intelligence engine files to 8.2 (classification preprocessing, 10-digit resolution, measure scanning, revenue/quote normalization, margin scenarios, Easyship/Shippo adapters), added missing APIs to 4.11 licensing table. Sections modified: 4.11, 5.4, 5.7, 8.2, 9.3, Appendix D. |
| 2026-03-12 | planmaxer pre-prep (data architecture) | Integrated expert review on data architecture and sync. Major changes: (1) Authority matrix replacing singular "source of truth" — three domains: local operational, remote shared, remote projections. Mixed-write tables prohibited. (2) Split opportunity_decisions into pipeline_assessments (daemon-owned), operator_decision_heads (remote-owned, CAS), bid_workflow_states (daemon-owned). (3) Transactional outbox pattern for push sync. (4) Stage coherence via process_stage_status + process_stage_heads. (5) Push protocol: monotonic local_seq, row_version, content_hash, atomic remote RPC. (6) Pull protocol: remote_change_seq cursors, compare-and-swap for decisions. (7) Schema migration coordination with schema_epoch. (8) Recovery model honesty: warm vs cold rebuild. (9) SQLite operational guidance: state.db/artifacts split. (10) Tombstone policy: only remote-owned entities. (11) Junction tables for cpv_codes and commodity_codes. (12) Integer pence for money, basis points for percentages. (13) MVP: one daemon per org. Sections modified: 4.7, 5.2, 5.4, 5.5, 5.6, 5.7, 5.8, 6.3, 6.4, 8.2, 9.4, ADR-002, Appendix E. |
| 2026-03-12 | planmaxer step 3 (data architecture) | Fresh-eyes optimization: added sync tracking columns to 8 push-owned tables, converted monetary columns to integer pence and confidence to basis points, incorporated cross-domain fields (NoticeLot, ProcessRelation, SearchProfile entities; expanded ProductCandidate, SupplierQuote, QualificationResult, agent_session_logs), removed array columns in favor of junction tables, added RLS policy specifications, updated SQLite compatibility notes, added raw_documents/sources to authority matrix. Sections modified: 4.7 (authority matrix table, SQLite compat notes), 5.4 (all SQL tables, new cross-domain entity tables), 6.3 (RLS policy specs). |
| 2026-03-12 | planmaxer step 4 (data architecture) | Sync pass: updated §8.2 migration file listings and §9.4 migration requirements to reflect new cross-domain entity tables (notice_lots, lot_cpv_codes, process_relations, search_profiles) and expanded column sets. |
| 2026-03-12 | planmaxer step 3+4 (Agent Orchestration) | Fresh-eyes optimization + sync pass: fixed hybrid-model terminology in 4.10 skill lifecycle and 6.5 test priority table, updated error manifest drill-down to reference session artifacts (5.11), added 3 missing health detection rows to 6.6 (HOST_RESOURCE_EXHAUSTED, AGENT_PROTOCOL_MISMATCH, AGENT_RUN_FAILED), added missing fault-injection test for AGENT_RUN_FAILED in 6.5, verified 8.2/8.3 naming patterns and ADR-004/ADR-005 status fields, verified ToC anchors. At that point, Supplier Quote Engine and Bid Workflow Executor still referenced PTY-only wording, and 5.6 still had a step-numbering collision; both were resolved later in the holistic pass. |
| 2026-03-12 | planmaxer steps 3–4 (procurement pipeline) | Fresh-eyes optimization and sync pass. §5.5: resolved rfq_escalated ghost state (clarified as operator disposition action), below-quoteability path, needs_review disambiguation (review_reason + review_stage), expired trigger mechanism (daemon background job), prelim_rejected entry conditions. §5.6: fixed step numbering collision — relation resolution renumbered to steps 13/13a–13e (was 10/10a–10e, colliding with data-arch agent adapter steps 10–12). §5.4: added cross-domain reconciliation note (quoteRealismConfidence from data-arch and quoteUsabilityConfidence from procurement pipeline are complementary, not duplicates). §8.2: added 3 missing intelligence pipeline skill file entries (document-acquisition, lot-extraction, product-extraction). Appendix C job kinds verified complete. §5.3 canonical workflow verified coherent. |
| 2026-03-12 | planmaxer holistic pass | Cross-plan unification pass. Fixed stale PTY-only terminology in the system context and workflow-trigger sections to match the adapter-mediated hybrid runtime; aligned §5.3 margin gates with Appendix A; updated §5.4 TypeScript entities and SQL comments to match the integrated procurement/data-architecture/landed-cost model; added missing `workflow_job_summaries` and `error_manifest_summaries` projection tables to resolve orphaned references in §§4.7/6.3/5.11; added explicit MVP operating limits and quote staleness invariants; corrected Bun workspace file references; extended §8.2/§8.3 and §9.1/§9.4 so file maps, tests, and dependencies cover relation resolution, quoteability, search-profile building, artifact compression, and runtime resource detection. |
| 2026-03-12 | planmaxer holistic pass (step 2 follow-up) | Completed the remaining cross-plan consistency cleanup. Added terminology notes for skill/playbook, process/opportunity, desktop/web surfaces, and shared-sync-target authority language; referenced Source Registry, Resource Arbiter, and summary projections directly in the runtime flows; switched the residual procurement/browser runtime wording from PTY-only to adapter-mediated sessions where appropriate; tightened the session-capture invariant so contract-mode runs also retain raw stdout/stderr forensic output; added sync delivery timeout and transcript chunking/truncation guardrails; made automatic search limits explicit per notice and per candidate; added missing `sha2` dependency and `sources/registry.rs` file mapping; removed the orphaned `operator_decision_events` mention; added the missing TypeScript interfaces for `NoticeLot`, `ProcessRelation`, and `SearchProfile`, and aligned `goodsSeparability` with the tri-state model used elsewhere; clarified that local execution is now a validated assumption and added a new user-input question for the designated always-on execution machine; clarified in Appendix B that the final confidence score applies after upstream qualification gates. |
| 2026-03-12 | planmaxer holistic pass (step 3) | Architectural coherence pass. Added the missing `Job Runner` component and clarified the scheduler-vs-runner split; tightened the SQLite Local Store vs Sync Engine boundary; added explicit dependency-direction notes (skills loaded by daemon, web portal depends only on Supabase projections, landed-cost stays downstream of extraction/matching); documented that complexity is intentionally concentrated in PTY/runtime orchestration and product/pricing comparability; aligned workflow/state/component text with the four-way relation model (`possible_duplicate` included). |
| 2026-03-12 | planmaxer holistic pass (step 4) | Sync pass. Verified the Table of Contents against the actual section structure; checked section/ADR/appendix cross-references and found them consistent; confirmed Appendix D category coverage and 9.3 external-account coverage; updated Quick Stats to reflect the current planned file count and expanded highest-risk area; corrected Appendix C job-kind wording to the four-way relation model. |
| 2026-03-12 | planmaxer holistic pass (step 2 residual) | Final residual consistency cleanup. Normalized remaining PTY-only wording where the runtime now preserves raw output in both PTY and contract modes; clarified recovery, security, and ADR-004 language to refer to full raw session-output artifacts rather than only PTY terminal transcripts; updated legacy `terminal_output_*` SQL comments to match the hybrid runtime contract. |
| 2026-03-12 | planmaxer holistic pass (step 2 residual 2) | Finalized the last hybrid-runtime consistency edits. Updated `Artifact Store` and Layer 3 observability wording to describe raw session-output capture in both PTY and contract modes, and aligned the illustrative `resolveConfidenceGate` example with the document-wide `0.60` starting confidence floor. |
| 2026-03-12 | planmaxer holistic pass (step 2 residual 3) | Closed the last remaining live wording gaps in the current document. Updated supplier-search and bid-workflow flows plus the daemon-complexity row to refer to full raw session output rather than PTY-only output where the runtime is hybrid, and made Appendix C use the canonical four-way relation labels. |
| 2026-03-12 | planmaxer holistic pass (step 3 residual) | Tightened the remaining architecture-language gaps. Added an explicit acyclic coupling rule to the coherence notes, updated the relation-resolution complexity row to the four-way model, and normalized the last user-facing / portal-facing wording to refer to full session output or raw session transcripts rather than PTY-only terminology where the runtime is hybrid. |
| 2026-03-12 | planmaxer holistic pass (step 3 residual 2) | Finalized the last boundary clarifications. Narrowed `PTY Session Manager` to live PTY-backed supervision while leaving final cleanup ownership to `Session Reaper`, reframed `Resource Arbiter` as admission control rather than a second scheduler, and made the complexity-allocation note explicit that the landed-cost engine remains very high complexity while intentionally staying downstream of the orchestration-critical boundaries. |
| 2026-03-12 | planmaxer holistic pass (step 4 residual) | Final sync pass. Rechecked the Table of Contents against the current section layout, verified section/ADR/appendix cross-references still line up, confirmed Appendix D and 9.3 remain complete for the cited services, and corrected the Quick Stats file-count total to the then-current planned file map. |
| 2026-03-12 | planmaxer pre-prep (frontend integration) | Integrated expert review on frontend and desktop-daemon integration architecture. Major changes: (1) Desktop ↔ daemon hybrid communication protocol — Tauri invoke() for snapshot reads via QueryService (read-only SQLite), DaemonBridge for writes/triggers via UDS/named-pipe RPC, Tauri Channel<T> for live streams. (2) Real-time event streaming architecture — three-layer model (daemon buffer → desktop stream store → terminal/timeline views), SessionFrame types, two-level attach semantics (metadata-only for session list, full stream for selected), dual viewer (live terminal + searchable history), PTY vs contract mode default views, useSyncExternalStore pattern. (3) Web portal subscription architecture — Supabase Broadcast as invalidation plane with page-level topic scheme, small PortalUpdate payloads (not full rows), per-page behavior (list pill, detail refresh chip), coalesced post-batch emission, viewer_sync_status staleness thresholds. (4) Shared React component architecture — snapshot+overlay model, TanStack Query for snapshots only, frame+slot composition pattern, explicit sharing boundary (views yes, hooks no). (5) Desktop lifecycle management — startup sequence with daemon handshake, four runtime modes (healthy/remote-degraded/browse-only/recovery-required), local intent queue for remote-degraded operator decisions, daemon crash recovery, explicit shutdown behavior. (6) Accessibility guidance for live views. (7) IPC protocol versioning separate from schema_epoch. (8) Multi-window support (detached session monitor). (9) Keyset/cursor pagination on both surfaces. (10) SQLite FTS5 for desktop search, Postgres text search for web. (11) Updated transport architecture in §5.10 with stream surfaces and mutation ack contract. Cross-domain notes deferred: viewer_sync_status table, pending_operator_intents table, process_aliases for relation resolution — all need data-architecture domain coordination. Sections modified: 1 (exec summary), 5.8 (full rewrite), 5.10 (expanded), 6.4 (frontend performance), 8.2 (8 new files), ADR-003 (expanded), ADR-008 (expanded), Appendix E. |
| 2026-03-13 | planmaxer steps 3–4 (frontend integration) | Fresh-eyes optimization + sync pass. Verified 7 cross-reference consistency criteria: (1) §5.8↔§5.10 — endpoint names, stream surfaces, MutationAck contract consistent. (2) §5.8↔§4.9 — 7-phase session lifecycle matches exactly, adapter events aligned. (3) §8.2 file map — fixed bootstrap-state.ts mode list from 3 modes to 4 (added remote-degraded, corrected recovery to recovery-required). (4) ADR-003↔§5.8 — communication architecture summary, IPC versioning, multi-window aligned. (5) §5.8↔§4.7 — authority matrix, CAS for decisions, sync protocol references consistent. (6) §6.4 performance — all 5 frontend guidance items and 2 bottleneck rows match §5.8 specifications. (7) Terminology consistent across all edited sections per §5.8 terminology note. Sync pass: verified §1 exec summary reflects communication protocol and frontend strategy; §8.2 complete with 8 new files; §11 milestones 2 and 4 include frontend-specific items; validated 16 cross-references to §5.8/§5.10/ADR-003/ADR-008. Section modified: 8.2 (bootstrap-state.ts description). |
| 2026-03-13 | planmaxer pre-prep (skill reliability) | Integrated expert review on skill authoring and agent reliability. Major changes: (1) Production skill directory standard — skill is a versioned directory (SKILL.md + result.schema.json + checkpoint.schema.json + canary.inputs.jsonl + golden.assertions.yaml + optional scripts/ and references/), not just SKILL.md. (2) 8-question skill authoring guide (trigger boundary, page fingerprint, navigation strategy with tiered locators, overlay handling, three-layer extraction validation, per-item evidence, checkpoint unit with concrete format, 6 outcome classes). (3) Browser automation reliability patterns — anti-bot classification table, source-scoped auth persistence via dedicated auth-refresh skills, readiness algorithm for dynamic content, geo/locale/currency drift detection, session contamination prevention. (4) Skill versioning and lifecycle management — 6-stage promotion flow (draft → candidate → canary → shadow → stable → rollback), 5 independent version dimensions, version binding at enqueue, schema migration with dual-accept windows, cache invalidation across 8 key dimensions. (5) Cost efficiency patterns — explicit budget declarations (maxPages, maxScrollCycles, etc.), cost-per-committed-useful-record metric. (6) Skill composition and model policy — intra-skill (scripts/) vs inter-skill (daemon-orchestrated sub-jobs), model_profile abstraction (4 tiers). (7) Failure modes table — 8 production failure modes requiring explicit skill-level detection. (8) New error types in §5.11: AGENT_SUSPECT_OUTPUT + 6 source/skill codes (SOURCE_SOFT_BLOCKED, SOURCE_CAPTCHA_BLOCKED, AUTH_STATE_STALE, PAGE_SHAPE_DRIFT, LOCALE_OR_CURRENCY_DRIFT, RESUME_UNSAFE_AFTER_SUBMIT). (9) Skill outcome classes (non-error): continuation_required, degraded_completed, blocked_human_action. (10) Skill-specific testing strategy: CI linting, live smoke canaries, invariant-based golden journeys, shadow runs, production drift monitoring, browser debugging artifacts. (11) Skill-level telemetry: 13 metrics. (12) Operational skills category: auth-refresh/alibaba, auth-refresh/1688, source-health/<source>. (13) 3 new risks in §6.2. (14) §11 milestones updated with skill authoring prerequisites. Cross-domain notes deferred: output quality gate (§5.2), continuation re-enqueue (§4.9), source circuit breaker (§4.9), ingestion flow changes (§5.6), intelligence pipeline changes (§5.7). Sections modified: 1, 4.10, 5.11, 6.1, 6.2, 6.5, 6.6, 8.2, 8.3, 11, Appendix D, Appendix E. |
| 2026-03-13 | planmaxer steps 3–4 (search intelligence) | Fresh-eyes optimization + sync pass. Fixed stale terminology: §5.3 canonical workflow updated from "query packs" to "query waves", from static "top-10" shortlist to adaptive budget (top-8 to top-20), from fixed "top-3" AI adjudication to recallRiskScore-scaled budget. Updated §5.3 price-signal matching summary to reflect current pipeline (query-wave search, four-state evidence screening, detail-page enrichment, adaptive diversified shortlisting, scaled AI adjudication). Updated §8.2 file reference from "query-pack construction" to "query-wave construction". Added cross-domain note flagging SQL column `query_pack_json` → `query_waves_json` rename. Verified consistent: §4.3 ↔ §5.7 source routing and query waves, §5.2 ↔ §5.4 interfaces, §5.7 ↔ §6.1/§6.2 complexity and risks, confidence weights (both sum to 1.0), pipeline cost ($11.70–$51.25/day within target), Appendix A margin logic. §1, §4.2, §11, Appendix A verified current — no changes needed. Sections modified: 5.3, 5.4 (cross-domain note), 8.2, Appendix E. |
| 2026-03-12 | planmaxer pre-prep (search intelligence) | Integrated expert review on search intelligence and product matching quality. Major changes: (1) `matchArchetype` classification added to ProductCandidate and SearchProfile — orthogonal to `candidateType`, drives query templates, hard-conflict weights, confidence floors, and shortlist budgets. Six archetypes: standard_class_led, dimension_material_led, electrical_spec_led, pack_count_led, compatibility_led, model_exact. (2) Query-wave planning replacing flat queryPack — staged queries from exact anchor → alias swap → market framing → broad → compliance-off, per candidate × source. (3) Four-state evidence model (match/missing/ambiguous/conflict) for hard-conflict filtering — only `conflict` triggers hard rejection; `missing` and `ambiguous` are confidence penalties. (4) Detail-page enrichment step before rejection — compliance evidence tiers (structured_attribute/title_body_mention/absent) determined from detail pages, not search cards alone. (5) Adaptive diversified shortlisting with `recallRiskScore` — budget scales from top-8 to top-20 based on category noise; diversity rules (max 2 per seller, max 2 per wave, max 1 per title family). (6) Zero-result recovery ladder with 6 rounds and formal `searchFailureMode` taxonomy (vocabulary_gap, overconstrained_query, accessory_swamp, compliance_not_surfaceable, no_marketplace_presence, low_extraction_quality, source_specific_failure). (7) Slot criticality model (required/preferred/descriptive/exclusion) on each extracted attribute. (8) Structured standard normalization (family/part/edition/amendment/class) for precise comparison. (9) Market framing slots (marketFrameEn/Zh, useContext, sectorContext) expanding the cross-language matching strategy beyond literal slot translation. (10) Chinese retry on Alibaba after English failure. (11) Source-specific search behavior (eBay category-prior search, Amazon alias families, Alibaba filter-heavy sourcing, 1688 Chinese keyword clusters). (12) Lexicographic gating for dual confidence — identity gate → usability gate → role gate, with archetype-specific confidence floors. (13) Two-tier cache (search-result + listing-evidence) with structured hard/soft fingerprints and source-specific TTLs. (14) Recall measurement framework (recall@k by archetype × source, operator override tracking, calibration pipeline). (15) Curated bilingual lexicon maintenance model with operator feedback loop. (16) Bilingual unit normalization dictionary. (17) New entities: SearchAttempt, ListingEvidence. (18) 8 new risks in §6.2. Sections modified: 1, 4.3, 5.2, 5.4 (TypeScript interfaces + data model notes), 5.7, 6.1, 6.2, Appendix E. |
| 2026-03-13 | planmaxer pre-prep (MVP delivery strategy) | Integrated expert review on delivery strategy and vertical slicing. Core change: replaced infrastructure-first milestone ordering with validation-first ordering (intelligence thesis → thin runner → desktop MVP → hardening → shared visibility → expanded intelligence → bid automation). Major changes: (1) Validation-first delivery philosophy. (2) MVE definition (~25-35 files, gold-set validation before infrastructure). (3) 4 sequenced experiments with quantitative pass criteria. (4) Category whitelist + no-list. (5) MVP-Alpha definition. (6) 7 milestones (0-6) replacing 6 (1-6). (7) Stack sequencing (Skills → TS → Rust → Desktop → Web). (8) 4-lane testing approach. (9) Validation techniques (gold set, shadow mode, decision ledger, feature flags). (10) §8.4 milestone assignment map. (11) Evaluation harness files in §8.2. (12) 3 new delivery risks in §6.2. (13) §6.1 MVE-critical tier updated. (14) Exit criteria for every milestone. Sections modified: 1, 6.1, 6.2, 8.2, 8.4 (new), 11 (full restructure), Appendix E. |
| 2026-03-13 | codex holistic pass (iteration 001) | Completed the post-integration cross-plan unification pass. Resolved the remaining deferred cross-domain items by integrating the Output Quality Gate, automatic continuation re-enqueue, and source circuit breaker into the authoritative runtime flows; aligned ingestion and supplier-enrichment flows with durable commit units, suspect-output quarantine, degraded list-only outcomes, and blocked-empty-shell handling; brought §5.4 SQL into parity with the current TypeScript/data-flow model (search_attempts, listing_evidence, process_aliases, viewer_sync_status, pending_operator_intents, source_runtime_state, expanded search/profile/quote fields, `query_waves_json`, expanded pipeline states); updated authority mapping, RLS, migration/file-map descriptions, and prerequisites to match the integrated architecture; and corrected Quick Stats to the current 150 planned files. Sections modified: 1, 2.5, 3.3, 4.7, 4.10, 5.2, 5.4, 5.6, 5.7, 5.8, 6.3, 8.2, 9.4, 11, Appendix E. |
| 2026-03-13 | codex holistic pass (iteration 001, step 2) | Cross-plan improvement pass with fresh-eyes consistency cleanup. Added a top-level terminology note clarifying notice/process/opportunity, search-profile/query-wave, and match/supplier-quote distinctions; fixed stale milestone terminology (`Phase B`, `query pack`, fixed AI-pool sizing wording); added missing cross-references from supplier pricing to §5.4, skills to §5.11, testing to §4.10, component designs to §5.4, and delivery strategy to §§6.1–6.2; aligned the daemon API surface in §5.10 with the control-panel flows in §5.8 by adding session/error-manifest/health endpoints; clarified that terminal traces load on demand; and standardized remaining live “web portal” references to the primary “viewing portal” term outside historical changelog entries. Sections modified: 1, 4.3, 4.10, 5.2, 5.7, 5.8, 5.10, 6.5, 11, Appendix E. |
| 2026-03-13 | codex holistic pass (iteration 001, step 2 follow-up) | Final targeted consistency cleanup. Resolved the remaining settings/API mismatch by adding daemon API endpoints for skill bindings and local runtime settings and aligning the desktop settings page description; clarified the typical-vs-hard-cap supplier-search budget in pipeline cost management; and updated the deterministic margin unit-test example to use integer pence, matching the persisted-money invariant in §5.4. Sections modified: 5.8, 5.10, 6.5, 8.2, Appendix E. |
| 2026-03-13 | codex holistic pass (iteration 001, step 2 residual) | Final terminology normalization pass. Replaced the remaining live authoritative uses of “web viewing portal” / “Web = read-only viewing portal” with the primary “viewing portal” term in executive summary, MVP constraint, realtime guidance, user-input resolutions, the viewing-portal flow, and the browser E2E file description. Sections modified: 1, 4.7, 4.8, 5.8, 8.3, 10, Appendix E. |
| 2026-03-13 | codex holistic pass (iteration 001, step 3) | Architectural-coherence pass. Marked `apps/api` as future-only so the project shape no longer over-promises a broker surface absent from the milestone/file map; clarified that §5.2 is target-state architecture and that the daemon API is intentionally page-oriented/task-oriented rather than generic; and aligned §8.4 milestone assignments with §11 by moving artifacts/error-manifest/skill-lint/smoke-canary work into Milestone 1 and auth-refresh/source-health/golden-journey/shadow/drift assets into Milestone 3. Sections modified: 3.4, 5.2, 5.10, 8.4, Appendix E. |
| 2026-03-13 | codex holistic pass (iteration 001, step 3 residual) | Final architecture-shape cleanup. Demoted `crates/common` and `packages/config` from implied MVP structure to later extraction points so the repo shape no longer carries “just in case” packages; clarified that `Skill-Based Ingestion Runner` can remain a module under the Job Runner until complexity justifies extraction; normalized the system-context and flow heading to the primary “viewing portal” term; and added the missing `packages/ui` file entries plus Milestone 2 assignments so the shared-UI boundary described in ADR-008 and §5.8 is reflected in the file map and delivery plan. Sections modified: 3.4, 5.1, 5.2, 5.8, 8.2, 8.4, Appendix E. |
| 2026-03-13 | codex holistic pass (iteration 001, step 3 final) | Final boundary-clarity cleanup. Added an explicit dependency rule that `packages/contracts-schema` and `packages/ui` must not import `packages/db`, daemon modules, bridge code, or Supabase clients; and clarified that `ProcessStageStatus.stage` is intentionally coarser than `PipelineAssessment.pipelineState` so orchestration stages and business progression cannot be mistaken for competing models. Sections modified: 3.4, 5.4, Appendix E. |
| 2026-03-13 | codex holistic pass (iteration 001, step 3 sync note) | Sync cleanup after the architecture passes. Updated Quick Stats to the current 155 planned files after the shared-UI additions, and clarified in §5.10 that `/api/company-profile` is a target-state endpoint tied to the Milestone 6 bidding package rather than an early-MVP requirement. Sections modified: 1, 5.10, Appendix E. |
| 2026-03-15 | harmonymaxer (Run 002) | Integrated worker-validator and run manifest system from standalone research doc. Fresh-eyes pass corrections: (a) §4.10 extraction validation updated from "three levels" to "four levels" with OQG/validator distinction; (b) §4.10 OQG paragraph reframed as "first line of defense" not sole defense; (c) §5.11 AGENT_SUSPECT_OUTPUT definition updated to list both OQG and validator as sources; (d) §6.1 complexity table updated to "four-layer validation"; (e) §6.6 updated from "Three-Layer Observability Model" to "Four-Layer" with Layer 4 (Narrative / run manifests); (f) §5.2 Log/Trace Capture Engine clarified as covering first three layers with cross-reference to Layer 4; (g) §5.2 Validation Executor: multi-segment validation rule added (validate each segment independently); (h) SQL comment fixed from "the manifest" to "the run manifest"; (i) AGENTS.md observability reference updated to four-layer model. Full integration details follow. |
| 2026-03-15 | harmonymaxer (Run 002, continued) | Integrated worker-validator and run manifest system from standalone research doc (`docs/research/2026-03-15-worker-validator-run-manifest-research.md`). Major changes: (1) Run manifests — every skill run produces a human-readable MD narrative stored as an immutable artifact; agent_session_logs gets run_manifest_path + run_manifest_hash columns. (2) Validator skills — new category 6 in skill types; base validator + domain-specific overlays; headless_contract mode with structured_extract_low_cost model profile. (3) Validation policy engine — per-skill validation_policy metadata (always/sample/trigger/skip); pre-stable stages always validate; trigger conditions for anomaly detection. (4) Three new §5.2 components: Run Manifest Writer, Validation Policy Engine, Validation Executor. (5) Job Runner modified: consults validation policy after OQG, spawns validator sub-jobs, processes verdicts (approved/rejected/flagged), fail-open on validator failure. (6) New DB entities: validation_verdicts (local-only), validation_verdict_summaries (push-owned projection). (7) Modified DB entities: workflow_jobs (parent_job_id, validation_required, validation_verdict_id), agent_session_logs (run_manifest_path, run_manifest_hash). (8) §4.7 authority split updated with new tables. (9) §5.3 canonical workflow: semantic validation step after opportunity manifest generation. (10) §5.6 ingestion flow: steps 12a-12d for run manifest generation and semantic validation. (11) §5.7 intelligence pipeline: validation note for all skill-based stages. (12) §5.9 bid automation: always-validate default for portal skills. (13) §5.11: VALIDATION_REJECTED, VALIDATION_FLAGGED, VALIDATION_SKIPPED_VALIDATOR_ERROR error types; validator verdicts distinguished from skill outcome classes. (14) §6.1: worker-validator complexity entry (Medium-High). (15) §6.2: 4 new risks (false rejections, validator unreliability, run manifest quality, validation cost). (16) §6.5: validator testing subsection (canaries, golden journeys, dry-run, false positive monitoring, regression, calibration). (17) §6.6: 5 new telemetry metrics (approval/rejection/flag rates, validator cost, operator override rate). (18) §8.2: 3 daemon files (policy.rs, executor.rs, manifest_writer.rs), 4 validator skill files. (19) §8.3: 4 validator test files. (20) §8.4: validation files assigned to Milestone 3 (dry-run) and Milestone 4 (enforcement + Supabase projection). (21) §11: Milestone 3 adds dry-run validation layer; Milestone 4 adds enforcement mode. (22) ADR-005 updated: validator skills as new category following same boundary rules. (23) OQG/Result Contract Validator: clarified relationship with semantic validator — OQG is first gate (code-based), validator is second gate (agent-based); OQG signals feed into validator. (24) §4.9: validator sessions follow same lifecycle, use headless_contract mode. (25) Terminology: "run manifest" always used as full term to disambiguate from opportunity manifest and error manifest. Sections modified: 1, 4.7, 4.9, 4.10, 5.2, 5.3, 5.4, 5.6, 5.7, 5.9, 5.11, 6.1, 6.2, 6.5, 6.6, 8.2, 8.3, 8.4, 11, ADR-005, Appendix E. |
| 2026-03-15 | harmonymaxer (Run 002, optimization audit) | Fresh-eyes harmonization audit after Run 002 integration. Closed the remaining documentation gaps by adding manual mode and dry-run semantics to the skill lifecycle, adding a Kickoff Prompt Renderer component and scratch-workspace behavior, disambiguating `generate opportunity manifest` from `run manifest`, adding TypeScript interfaces for `WorkflowJob`, `AgentSessionLog`, `ValidationVerdict`, and `ValidationVerdictSummary`, aligning the validator file map and milestone assignments with the skill-type table (base validator support files plus document-acquisition/product-extraction overlays), and replacing the residual policy-wording leak with the authoritative "validation policy" term. Sections modified: 4.10, 5.2, 5.3, 5.4, 5.6, 8.2, 8.4, Appendix E. |

---

*Generated by idea-to-research skill*
