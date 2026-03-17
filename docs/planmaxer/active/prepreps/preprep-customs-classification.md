# Pre-Prep: Customs Classification & Landed-Cost Engine

## Step 1: Pre-Prep Research

Read the following documents in this exact order. Do not skip any.

1. **AGENTS.md** — project constraints, toolchain, rules
2. **The research document:** `docs/research/2026-03-08-public-procurement-intelligence-platform-research.md`
   - Read the ENTIRE document, but pay special attention to:
     - §4.5 (Customs and Tariff Classification — HS code lookup, duty rate resolution, HMRC Trade Tariff API, classification pipeline, commodity code hierarchy)
     - §4.6 (FX Rate Strategy — rate sources, caching, staleness windows, fallback logic)
     - §5.7 (Intelligence Pipeline Data Flow — landed-cost pipeline steps: HS classification, duty lookup, freight estimation, FX conversion, landed cost assembly, net margin gate)
     - §5.4 (Data Models — LandedCostEstimate entity, landed_cost_estimates table, customs-related fields in ProductCandidate)
     - §5.11 (Error Handling — customs-related error types: HS_LOOKUP_FAILED, DUTY_RATE_UNAVAILABLE, FX_RATE_STALE, FREIGHT_ESTIMATE_FAILED, and any classification-specific errors)
     - §5.2 (Component Designs — Landed Cost Engine component, any classification-related components)
     - Appendix A (Margin Logic — duty_rate, freight_per_unit, fx_rate, insurance, landed cost formula, net margin gate)
     - §6.1 (Complexity Analysis — Landed-Cost Engine complexity rating)
     - §6.2 (Risk Assessment — HS code risks, tariff risks, FX risks)
3. **The handoff prompt that produced the feedback:** `docs/planmaxer/active/handoffs/handoff-customs-classification.md`

After reading, you should be able to explain:
- The HS classification pipeline: how product descriptions map to HS codes (6-digit minimum, 10-digit commodity codes for UK), the role of HMRC Trade Tariff API, fallback strategies for ambiguous products
- The duty rate resolution chain: commodity code → measure lookup → duty rate (ad valorem, specific, compound), preferential vs MFN rates, trade remedy duties (anti-dumping, countervailing)
- The FX strategy: which rate sources, how stale is too stale, how FX volatility affects margin confidence, fallback to last-known rate
- The freight estimation model: per-unit freight based on product characteristics, source geography, shipping mode (sea, air, courier), weight/volume calculations
- The landed cost formula: CIF value + duty + insurance + handling = landed cost, and how this feeds the net margin gate
- The relationship between HS classification confidence and landed cost confidence (garbage classification in → garbage landed cost out)
- The LandedCostEstimate entity structure: all cost components stored separately for auditability
- The error types covering classification failures and their downstream impact on pipeline progression

**File Ownership:** This pre-prep owns the following sections for editing:
- §4.5 (Customs and Tariff Classification) — FULL ownership
- §4.6 (FX Rate Strategy) — FULL ownership
- §5.7 (Intelligence Pipeline Data Flow) — Landed-cost subsection ONLY (HS classification through net margin gate steps)
- §5.11 (Error Handling) — Customs and classification error types ONLY (HS_LOOKUP_FAILED, DUTY_RATE_UNAVAILABLE, FX_RATE_STALE, FREIGHT_ESTIMATE_FAILED, and any new classification errors)
- Appendix A (Margin Logic) — Landed cost formula, duty rate application, and net margin gate ONLY (not gross margin gate, not matching confidence)

Do NOT edit: §4.2 (pipeline overview), §4.3 (supplier price intelligence), §4.7 (database architecture), §5.3 (pipeline stage sequence), §5.4 (SQL table definitions — note changes for sync-engine domain), §5.5 (state model), §5.6 (ingestion flow). The classification and landed-cost engine operates within the pipeline flow defined by other domains. If the reviewer suggests changes to pipeline ordering, SQL schema, or daemon internals, note them as deferred with `<!-- CROSS-DOMAIN: ... -->` comments.

---

## Step 2: Feedback Integration

Here’s the hard answer.

Yes, automated classification can be reliable enough for **commercial screening** in the UK. No, it is **not** reliable enough as currently specified to produce a single duty number for broad procurement text unless the engine is allowed to **abstain aggressively**. HMRC’s FPO API is a GB→NI fast-parcel tool that returns only **HS6 or CN8** suggestions, its `score` is only “a rating for how well the code matches”, and access is restricted to invited FPOs. The legal tariff treatment you actually care about sits on **declarable 10-digit codes**, with **TARIC additional codes, national additional codes, geography, measure conditions, supplementary units, and multi-component duty expressions**. HMRC also warns that getting the code/additional-code path wrong can lead to penalties, rejection of the declaration, and loss of reliefs or simplifications. ([Trade Tariff API][1])

The catastrophic misses are not “3.7% vs 4.7%”. They are “right family, wrong payable branch”. Right now, certain Chinese aluminium foil in low-weight rolls carries **14.2%, 14.6%, 15.6%, or 35.6% residual** depending on exporter/additional-code/invoice path; Chinese tin mill products used widely in packaging got a new UK anti-dumping measure on **12 March 2026** at **27.85% for the cooperating exporter and 49.98% for all other Chinese exporters**; Chinese ironing boards now run **18.1% to 42.3%**; certain aluminium extrusions can be **0%, 11.4%, 15.4%, 15.6%, or 35.1% residual**; and Chinese wire rod can be **7.9% or 24.0% residual**, again with invoice/declaration conditions. If your engine does not know the exporter/manufacturer and whether the required declaration text exists, it must assume the **residual** rate. ([GOV.UK][2])

It gets worse in steel-family measures. UK notices show that some anti-dumping and countervailing duties do **not** simply stack with safeguard duty: for organic coated steel, once the relevant quota is exceeded, only the safeguard duty, or only the portion of AD/CVD above the safeguard, is charged together with the safeguard. Separately, the current steel safeguard still applies a **25% out-of-quota** duty through **30 June 2026**. A naïve “base duty + AD + CVD + safeguard” calculator is wrong. ([GOV.UK][3])

## My verdict on your three tensions

**FPO is not the wrong tool; it is the wrong authority.** Keep it as **one recall source**. Do not let it be the thing that decides the path. The right test is: does FPO improve **recall@k** over a baseline built from CPV→chapter priors, tariff keyword search, and product-family codebooks? If yes, keep it. If you are descending only the top FPO candidate into a UK10 tree, it will poison the engine on vague industrial procurement text. HMRC’s own docs already tell you what it is: parcel-style English description in, HS6/CN8 candidate list out, best-match-first, score = better textual match. ([Trade Tariff API][1])

**Screening-grade vs decision-grade is the right split, but your boundary is underspecified.** The boundary should not be “confidence above X”. It should be: **can customs uncertainty still reverse the decision?** A 2 point duty difference on a £500k buy is tolerable only if the deal still clears the gate on the **worst plausible customs branch**. A **2.7%–4.7%** customs branch is a range. A **2.7%–48.5%** branch is not a range; it is a stop signal.

**The internal engine should stay canonical.** But the reason it will struggle is not that HMRC’s APIs are bad. It is that they are **reference APIs**, not fact-generators. They can tell you what happens once you know the right code, origin, exporter/additional code, measure conditions, units, and valuation basis. They cannot manufacture missing facts like material composition, sterility, shell type, substrate, exporter identity, or whether an Alibaba price reflects an actual sale for export.

## Where this pipeline will fail silently

1. **Top-candidate poisoning.** FPO gives a plausible HS family, you descend it greedily, and never expand the runner-up family that would have led to the right UK10.

2. **HS6 convergence that hides UK10 measure divergence.** HS6 agreement is the least interesting agreement once declarable codes split on remedy/no-remedy, additional-code requirement, or supplementary units.

3. **Exporter-specific rate fantasy.** The code is right, the measure is found, but the engine assumes a named-exporter or annex rate without a verified exporter/manufacturer identity and invoice declaration. Current UK notices for foil, extrusions, wire rod, ironing boards, and organic coated steel all show exactly this trap. ([GOV.UK][2])

4. **Specific/compound duty flattening.** HMRC’s commodities API explicitly models duty expressions with one or more measure components, and conditions may depend on weight, volume, or value thresholds. If your model flattens everything to `dutyPct`, it will silently misprice specific and compound duties. ([Trade Tariff API][4])

5. **VAT family miss.** The spec is good on PVA and recoverability, but weak on **VAT-rate determination**. HMRC’s CDS guidance makes national additional code **6/17** the place for VAT reduced rates and VAT zero-rating, including `VATZ` where the relief is commodity-driven. Defaulting everything to standard-rate VAT unless resolved is conservative; pretending the VAT rate is already known is not. ([GOV.UK][5])

6. **Temporal staleness.** HMRC’s declarable-measures dataset says the tariffs are a representation of **future events** and are **subject to change**; it also omits VAT and quota volumes. On top of that, March 2026 alone has tariff stop-press updates and new trade-remedy actions, including tin mill and code-structure changes. A cached classification without a freshness rule is a liability. ([API Catalogue][6])

## What it will get catastrophically wrong

The common pattern is: **the engine resolves a plausible code, but not the legally payable branch**.

* **Packaging: catering foil rolls.** A description like “foil rolls for catering use” can look simple and still land in a live anti-dumping family with 14.2%–35.6% duties plus TAP codes and invoice declaration requirements. ([GOV.UK][2])

* **Packaging: glass jars and bottles.** “Glass jars with lids” may look boring, but glass containers from China are under a new anti-dumping investigation opened on **5 March 2026** across multiple `701090****` codes. That means any cached “no remedy risk” assumption is already stale. ([trade-remedies.service.gov.uk][7])

* **Packaging: metal cans / tin mill products.** This changed **three days ago**. Tin mill products used in packaging now have a new anti-dumping measure, and HMRC’s own tariff news also shows live stop-press handling and code-structure maintenance in this family. ([GOV.UK][8])

* **Basic furniture: galvanised steel shelving.** This is a classic 9403 vs 7308 vs 7326 fight. It becomes catastrophic when the model treats it as generic furniture and never tests whether the goods are structurally classified steel products or coated-metal goods.

* **Basic furniture: modular office partition / frame systems.** A procurement notice might say “modular office furniture system” while the economic reality is aluminium profile systems, connectors, and panels. In today’s UK measures, aluminium extrusions are an active remedy family with exporter-specific and residual outcomes. ([GOV.UK][9])

* **Basic PPE: nitrile gloves.** “Gloves” is not enough. Medical examination, household cleaning, industrial gauntlet, sterile/non-sterile, powder-free, material, and pack basis all move the classification and sometimes the VAT and controls path.

* **Basic PPE: safety helmets.** Industrial vs sports is not cosmetic. Missing the standard/use split is exactly how a plausible-but-wrong HS family survives ranking.

* **Cleaning products: wipes.** “Cleaning wipes” may be paper, nonwoven textile, cellulose, or chemically impregnated preparations. The FPO-style text retrieval path will happily give you plausible-looking candidates while the legal branch depends on substrate and impregnation.

* **Hand tools: kits and spares.** “Tool kit”, “socket set”, “replacement blades”, “bits”, “heads”, and “parts” are where generic nouns and set/component rules create fake certainty.

## Whitelist reality check

**Office supplies:** easiest overall. A4 copier paper, envelopes, basic binders, plain notebooks are screening-grade candidates if the substrate and pack basis are explicit. The main traps are generic “stationery”, labels, adhesive-backed products, and paper-vs-textile wipe language. In the current official measures I reviewed, remedy concentration is not here.

**Cleaning products:** medium-hard. Buckets and simple brushes are fine. Wipes, cloths, chemically treated consumables, and mixed cleaning kits are not. The risk here is less live trade remedies and more substrate/preparation ambiguity.

**Basic PPE:** hard. Gloves and helmets are `standard_class_led` products and need exactly the kind of facts procurement notices omit. Direct remedy exposure in the current official cases I reviewed looks lower than packaging or metal goods, but false confidence risk is very high.

**Hand tools:** medium. Single non-powered tools are workable. Sets, replacement parts, blades, bits, kits, and anything edging toward powered/electrical are not. Direct current remedy exposure appears lower than packaging, but steel-family adjacency still matters.

**Basic furniture:** hard. Material and construction method matter too much. If the lot is wood-only and simple, screening can work. If it is metal shelving, office screens, partition systems, or modular frames, your remedy adjacency jumps sharply because coated steel, aluminium extrusions, and household metal goods already have live UK measures. ([GOV.UK][10])

**Packaging materials:** this is no longer a blanket “easy” whitelist category. Corrugated cartons and simple poly mailers are easy. Foil, glass containers, metal can/tin mill families, and composites are high-risk right now. This is the whitelist category I would reclassify from “generally safe” to “split: easy subfamilies / dangerous subfamilies”. ([GOV.UK][2])

## Trade-remedy snapshot for the whitelist

What is live or near-live **today** in or close to your whitelist:

* **Certain aluminium foil in rolls**: `7607111111`, `7607191011`, `7607191019`, `7607111119`; duty rates **14.2% to 35.6%**, with exporter-specific TAP codes and invoice declaration requirements; measure maintained to **6 June 2029**. ([GOV.UK][2])

* **Glass containers from China**: active **AD0087** investigation opened **5 March 2026** across multiple `701090****` codes including `7010901000`, `4100`, `4300`, `4500`, `4700`, `5100`, `5300`, `5500`, `5700`, `6100`, `6700`, `7100`, `7900`, `9100`, `9900`. No final duty yet, but commercially this already changes the freshness and risk state of packaging codebooks. ([trade-remedies.service.gov.uk][7])

* **Tin mill products from China**: new definitive anti-dumping measure imposed **12 March 2026**; **27.85%** for the cooperating company and **49.98%** for all other Chinese exporters; widely used in packaging. HMRC also published a stop-press update the same day. ([GOV.UK][8])

* **Ironing boards from China**: `3924900010`, `4421999910`, `7323930010`, `7323990010`, `8516797010`, `8516900051`; **18.1% to 42.3%** through **3 October 2029**. This is not office furniture, but it is proof that “finished household metal goods” can sit in brutal duty ranges. ([GOV.UK][11])

* **Organic coated steel from China**: Chapter 72 metal goods/input family relevant to metal furniture and fixtures; anti-dumping rates include **0%, 5.9%, 16.2%, 26.1%**, and countervailing rates include **13.7%, 23.8%, 26.8%, 29.7%, 44.7% residual**, with safeguard interaction rules and invoice/declaration dependency. ([GOV.UK][10])

* **Aluminium extrusions from China**: multi-code Chapter 76 / `7610` family used in frames, partitions, shelving systems, and fit-out components; current UK outcomes include **0%, 11.4%, 15.4%, 15.6%, 35.1% residual**, plus additional codes and invoice declaration requirements. ([GOV.UK][9])

I did **not** see comparable current official remedy concentration in the materials I reviewed on ordinary office stationery, common cleaning consumables, or simple non-powered hand tools. Today’s concentrated UK risk is packaging plus metal goods/components. ([trade-remedies.service.gov.uk][12])

## How I’d measure accuracy

Do not measure this engine like a search engine. Measure it like a **legal-risk filter**.

1. **Build three label tiers.**
   `gold` = ATR / actual import declaration outcome / broker-confirmed family with no later amendment.
   `silver` = two independent internal reviewers with documented reasoning and supporting technical docs.
   `bronze` = best-effort label from tender docs only.
   ATRs are legally binding and HMRC says they usually run for 3 years; AVRs give valuation certainty and HMRC says accepted applications get a decision within 90 days. ([GOV.UK][13])

2. **Score by evidence tier, not just by exact code.** A text-only procurement notice should not be judged by the same standard as a case with manufacturer datasheets and an actual invoice.

3. **Track the right outcome metrics.**
   Exact UK10 accuracy matters, but for this engine the more important metrics are:

   * `duty-outcome accuracy`
   * `trade-remedy yes/no accuracy`
   * `additional-code required yes/no accuracy`
   * `supplementary-unit required yes/no accuracy`
   * `review-trigger recall`
   * `catastrophic understatement rate`
   * `unsafe auto-classification rate`

4. **Treat abstention as a positive class.** If the description lacked the facts needed to classify legally and the engine escalated, that is a success, not a miss.

5. **A/B test FPO itself.** Keep it only if it improves recall@k or lowers abstention on the benchmark without increasing catastrophic misses.

## When the description is too vague

When the quality gate rejects, do **not** just output “manual review”. Output a **missing-facts request** keyed to the archetype:

* **Gloves:** material, sterile/non-sterile, examination vs household/industrial, powder-free, pack basis.
* **Helmets:** industrial vs sports, standard met, shell material, end use.
* **Shelving/furniture:** material, free-standing vs structural, knock-down vs component, load-bearing role.
* **Wipes:** substrate, impregnation/chemical treatment, disposable vs reusable.
* **Packaging:** foil thickness and roll weight, glass capacity, tinplate/ECCS vs other coated steel, polymer composition.

Sparse extraction should not bottom out at “low confidence but still scoreable”. In customs terms, sparse extraction often means **no legal confidence**.

## Margin logic: where your current spec is still too optimistic

Your Appendix A is close, but it still lets scenario bands do work they should not do.

A simple rule fixes most of it:

* If all plausible UK10 branches share the **same measure family**, no specific/compound duty is unresolved, no additional code is missing, and worst-case net margin still clears the gate, screening is fine.
* If plausible branches differ only modestly inside the same legal family, show a range and force review.
* If any plausible branch introduces **trade remedy vs no-trade-remedy**, **specific vs ad valorem**, **unresolved supplementary units**, **quota/safeguard interaction**, or **different VAT family**, suppress the margin and mark the case **customs unresolved**.

Put bluntly: **a 2.7%–48.5% customs branch is not a scenario band. It is a blocker.**

## The things you did not ask about, but need in the spec

1. **Exporter-rate basis.**
   `named_exporter | annex_exporter | residual_default`.
   Without this, you will systematically understate remedy exposure.

2. **Origin evidence state.**
   Seller country, dispatch country, and origin country are not the same thing. Preference should default to **MFN** unless proof exists.

3. **Measure interaction engine.**
   Not all duties stack arithmetically. Steel-family notices already prove that.

4. **Classification freshness.**
   Add `tariffCheckedAt`, `classificationFreshUntil`, `tradeRemedyWatchState`, and a daily TRA + tariff stop-press watcher. HMRC’s declarable dataset itself says future events are subject to change, and March 2026 has already shown how fast code structures and remedy states move. ([API Catalogue][6])

5. **VAT-rate resolution.**
   Separate `vatRateState` from `vatRecoveryRate`. One is legal rate determination; the other is recovery economics. HMRC’s CDS guidance on national additional codes makes that distinction real. ([GOV.UK][5])

6. **CBAM placeholder.**
   For iron/steel and aluminium goods, your medium-term engine needs a forward field now. UK CBAM starts **1 January 2027**, with trial rates in **Q4 2026**. For packaging metals, steel shelving, coated steel, and aluminium systems, that becomes a real landed-cost dimension very soon. ([GOV.UK][14])

## Redline patch set

This is the patch I’d apply.

```diff
@@ Section 4.5 — Customs, Tariff, and Commodity Classification
- Use HMRC FPO API as the primary automated first-pass HS code candidate generator
+ Use HMRC FPO only as one candidate-recall source.
+ Add parallel recall sources:
+   - CPV->chapter prior
+   - rule-based chapter hints from head noun/material/function
+   - tariff keyword search
+   - ATR/codebook reuse
+ Persist candidateSource[] and candidateRecallAgreement.
+ State explicitly: FPO score is retrieval relevance, not legal confidence.

+ Add exporter/manufacturer resolution:
+   exporterRateBasis = named_exporter | annex_exporter | residual_default
+   exporterIdentityConfidence
+ Default to residual rate whenever exporter/additional-code/invoice evidence is missing.

+ Add tradeRemedyState:
+   none | investigation_open | provisional | definitive | review_open | adjacent_risk

+ Add measureInteractionEngine:
+   handles AD/CVD/safeguard/quota interactions, not simple additive stacking

+ Add supplementaryUnitsRequired[] and measureConditionPermutations[]
+ Add classificationFreshUntil / tariffCheckedAt / tradeRemedyWatchState
```

```diff
@@ Section 4.6 — FX Conversion
- Store fxBasisDeltaPct for customs vs commercial rate difference
+ Feed fxBasisDeltaPct into margin stress scenarios and freshness gating.
+ Force recalculation at HMRC monthly-rate rollover even if commercial FX is unchanged.
```

```diff
@@ Section 5.4 — LandedCostEstimate / CandidateCommodityCode
+ tradeRemedyState
+ tradeRemedyCases[]
+ exporterRateBasis
+ exporterIdentityConfidence
+ originEvidenceState = proved | claimed | unknown
+ additionalCodeRequirements[]   // not just resolved additionalCodes[]
+ supplementaryUnitsRequired[]
+ vatRateState = standard | reduced | zero | unresolved
+ dutyOutcomeMinGbp
+ dutyOutcomeMaxGbp
+ customsCostSpreadBps
+ customsResolutionConfidence

- customsValuationMethod: "1" | "2" | ... | "6"
+ valuationMode: estimation | declaration
+ customsValuationMethod:
+   1_actual | 1_proxy | 2 | 3 | 4 | 5 | 6 | unknown
```

```diff
@@ Section 5.7 — Landed Cost (API-Based)
- Resolve top HS6/CN8 candidate to a 10-digit code
+ Expand all plausible HS6/CN8 families to all declarable UK10 descendants.
+ Rerank descendants with slot compatibility + measure compatibility.
+ Never descend only the top FPO candidate.

+ Conservative defaults:
+   no origin proof        -> MFN
+   no exporter proof      -> residual remedy rate
+   no quota allocation    -> out-of-quota rate
+   unresolved VAT status  -> standard VAT rate
+   unresolved Incoterm    -> boundary scenarios, not point normalization

+ If plausible branches split on:
+   remedy / no-remedy
+   specific / ad-valorem
+   different VAT family
+   unresolved supplementary unit
+ then suppress single-margin output and escalate.
```

```diff
@@ Section 5.11 — Error Handling Strategy
+ EXPORTER_IDENTITY_UNRESOLVED
+ RESIDUAL_RATE_ASSUMED
+ ORIGIN_PROOF_MISSING
+ SUPPLEMENTARY_UNIT_REQUIRED
+ MEASURE_INTERACTION_COMPLEX
+ VAT_RATE_UNRESOLVED
+ CLASSIFICATION_STALE
+ TRA_INVESTIGATION_OPEN
+ ESTIMATE_MODE_VALUATION_ONLY
```

```diff
@@ Section 6.2 — Risk Assessment
+ Correlated-source false confidence:
+   FPO + Taric Support + HS Ping agreement is not independent evidence
+ Exporter-specific remedy understatement
+ National VAT additional-code miss
+ Tariff stop-press / code-structure drift
+ Trade-remedy investigation staleness
+ Measure-interaction arithmetic error
```

```diff
@@ ADR-006 — Build an Internal Landed-Cost Engine
- Internal engine computes canonical landed cost
+ Internal engine is canonical only as a conservative legal-bounding engine.
+ It must prefer abstention over false precision.
+ It must not emit a single duty number when customsResolutionState is unresolved.
```

```diff
@@ Appendix A — Suggested Margin Logic
- margin scenarios with critical blockers
+ Add hard rule:
+   if any plausible customs branch introduces remedy/no-remedy split,
+   unresolved specific duty,
+   quota/safeguard interaction,
+   or unresolved VAT family,
+   do not show a decision-grade margin.

+ Screening-grade auto-pass only if:
+   worstCaseNetMargin >= gate
+   and customsCostSpread is immaterial
+   and no catastrophic legal branch exists
```

```diff
@@ Appendix B — Suggested Confidence Model
- single blended confidence used as main gate
+ Split confidence into:
+   customsResolutionConfidence
+   commercialEstimateConfidence
+ Use lexicographic gating:
+   legal resolution -> customs spread -> commercial confidence
+ Weighted composite remains secondary, never primary, for customs go/no-go.
```

These edits are driven by what the official data actually looks like: FPO is an invite-only HS6/CN8 suggestion API with a text-match score, HMRC requires additional and national additional codes for anti-dumping and VAT paths, declarable-code tariff data is future-event data and omits VAT/quotas, and the commodities API exposes multi-component duties plus measure conditions and geography rather than a single flat rate. ([Trade Tariff API][1])

If you want the sharpest single takeaway, it’s this: **the engine should be judged by how often it refuses to lie, not by how often it produces a number.** That is the line between a useful screening engine and a customs-liability trap.

[1]: https://api.trade-tariff.service.gov.uk/fpo.html "https://api.trade-tariff.service.gov.uk/fpo.html"
[2]: https://www.gov.uk/government/publications/trade-remedies-notices-anti-dumping-duty-on-aluminium-foil-from-china/trade-remedies-notice-202524-anti-dumping-duty-on-certain-aluminium-foil-in-rolls-originating-from-the-peoples-republic-of-china "https://www.gov.uk/government/publications/trade-remedies-notices-anti-dumping-duty-on-aluminium-foil-from-china/trade-remedies-notice-202524-anti-dumping-duty-on-certain-aluminium-foil-in-rolls-originating-from-the-peoples-republic-of-china"
[3]: https://www.gov.uk/government/publications/trade-remedies-notices-countervailing-duty-on-organic-coated-steel-products-from-china/suspension-notice-202011-countervailing-duty-on-certain-organic-coated-steel-products-originating-in-the-peoples-republic-of-china "https://www.gov.uk/government/publications/trade-remedies-notices-countervailing-duty-on-organic-coated-steel-products-from-china/suspension-notice-202011-countervailing-duty-on-certain-organic-coated-steel-products-originating-in-the-peoples-republic-of-china"
[4]: https://api.trade-tariff.service.gov.uk/the-commodities-api.html "https://api.trade-tariff.service.gov.uk/the-commodities-api.html"
[5]: https://www.gov.uk/guidance/using-commodity-codes-and-related-additional-codes-in-the-customs-declaration-service "https://www.gov.uk/guidance/using-commodity-codes-and-related-additional-codes-in-the-customs-declaration-service"
[6]: https://www.api.gov.uk/dbt/uk-global-tariff-measures-on-declarable-commodities/ "https://www.api.gov.uk/dbt/uk-global-tariff-measures-on-declarable-commodities/"
[7]: https://www.trade-remedies.service.gov.uk/public/case/AD0087/ "https://www.trade-remedies.service.gov.uk/public/case/AD0087/"
[8]: https://www.gov.uk/government/news/new-anti-dumping-measure-on-chinese-tin-mill-imposed "https://www.gov.uk/government/news/new-anti-dumping-measure-on-chinese-tin-mill-imposed"
[9]: https://www.gov.uk/government/publications/trade-remedies-notices-anti-dumping-duty-on-certain-aluminium-extrusions-from-china/taxation-notice-202211-definitive-anti-dumping-duties-on-certain-aluminium-extrusions-originating-from-china "https://www.gov.uk/government/publications/trade-remedies-notices-anti-dumping-duty-on-certain-aluminium-extrusions-from-china/taxation-notice-202211-definitive-anti-dumping-duties-on-certain-aluminium-extrusions-originating-from-china"
[10]: https://www.gov.uk/government/publications/trade-remedies-notices-anti-dumping-duty-on-organic-coated-steel-products-from-china/trade-remedies-notice-202514-anti-dumping-duty-on-organic-coated-steel-products-originating-from-china "https://www.gov.uk/government/publications/trade-remedies-notices-anti-dumping-duty-on-organic-coated-steel-products-from-china/trade-remedies-notice-202514-anti-dumping-duty-on-organic-coated-steel-products-originating-from-china"
[11]: https://www.gov.uk/government/publications/trade-remedies-notices-anti-dumping-duty-on-ironing-boards-from-china/trade-remedies-notice-202609-definitive-anti-dumping-duty-on-ironing-boards-originating-from-china "https://www.gov.uk/government/publications/trade-remedies-notices-anti-dumping-duty-on-ironing-boards-from-china/trade-remedies-notice-202609-definitive-anti-dumping-duty-on-ironing-boards-originating-from-china"
[12]: https://www.trade-remedies.service.gov.uk/public/cases/ "https://www.trade-remedies.service.gov.uk/public/cases/"
[13]: https://www.gov.uk/guidance/check-what-youll-need-to-get-a-legally-binding-decision-on-a-commodity-code "https://www.gov.uk/guidance/check-what-youll-need-to-get-a-legally-binding-decision-on-a-commodity-code"
[14]: https://www.gov.uk/government/publications/carbon-border-adjustment-mechanism-cbam-policy-summary/carbon-border-adjustment-mechanism-cbam-policy-summary "https://www.gov.uk/government/publications/carbon-border-adjustment-mechanism-cbam-policy-summary/carbon-border-adjustment-mechanism-cbam-policy-summary"


For each change proposed by the reviewer, evaluate:

### Integration Rules

1. **For HS classification pipeline improvements:** Integrate into §4.5. If the reviewer proposes multi-stage classification (coarse → fine), confidence scoring for HS predictions, ambiguity resolution strategies, or fallback hierarchies (10-digit → 8-digit → 6-digit → chapter-level), integrate with concrete examples. Classification confidence directly impacts landed cost reliability — make this dependency explicit.

2. **For duty rate resolution improvements:** Integrate into §4.5. If the reviewer proposes handling preferential tariff regimes (GSP, DCTS, UK-specific trade agreements), compound duty calculation, trade remedy surcharges (anti-dumping duties on Chinese goods), or seasonal duty variations, integrate while preserving the HMRC Trade Tariff API as the primary source. Note which duty complexities matter for the China-to-UK corridor specifically.

3. **For FX strategy improvements:** Integrate into §4.6. If the reviewer proposes multiple rate sources for cross-validation, volatility-adjusted margin buffers, rate pinning for quote validity windows, or currency pair optimization (CNY/GBP vs CNY/USD/GBP triangulation), integrate while preserving simplicity for MVP. FX must not become a trading engine.

4. **For freight estimation improvements:** Integrate into §5.7 (landed-cost subsection). If the reviewer proposes weight-break pricing tiers, dimensional weight calculations, multi-modal shipping comparisons, or port-pair-specific rate tables, integrate while noting that freight is one of the highest-uncertainty components. Conservative estimates (overestimate freight) are safer than optimistic estimates.

5. **For landed cost formula improvements:** Integrate into Appendix A. If the reviewer proposes additional cost components (insurance calculation method, handling fees by port, customs clearance fees, VAT treatment), integrate while keeping the formula auditable. Every cost component must be stored separately in LandedCostEstimate for transparency.

6. **For classification confidence → margin confidence propagation:** Integrate into §5.7 and Appendix A. If the reviewer proposes that low HS classification confidence should widen the margin uncertainty band or trigger operator review before the net margin gate, integrate as a concrete confidence propagation rule.

7. **For customs error handling improvements:** Integrate into §5.11. If the reviewer proposes graduated degradation (use chapter-level duty if commodity-level lookup fails), partial landed cost estimates, or error recovery strategies, integrate while preserving the principle that bad data should be flagged, not silently accepted.

8. **For changes touching SQL schema, pipeline stage ordering, component designs, or skill definitions:** Do NOT integrate directly. Add `<!-- CROSS-DOMAIN: Reviewer suggested [X] for [component]. Needs coordination with [domain]. -->` notes.

### Evaluation Framework

For each proposed change, assess:
- **Agree strongly:** Improves classification accuracy, landed cost reliability, or makes the China-to-UK corridor handling more realistic. Apply directly.
- **Agree partially:** Good direction but may over-engineer for MVP (e.g., handling 50 trade agreements when we only care about China-to-UK). Apply scoped to the primary corridor, noting future expansion.
- **Disagree:** Conflicts with settled decisions (HMRC API as primary source, API-based cost estimation not agent scraping, conservative estimation bias, auditability of cost components). Note disagreement with rationale.

### Preservation Rules
- Preserve HMRC Trade Tariff API as the primary duty rate source
- Preserve API-based data retrieval for cost estimation (not agent scraping — this is the exception to the skill-first rule)
- Preserve the landed cost formula's auditability (every component stored separately)
- Preserve the net margin gate as the final pipeline gate before manifest generation
- Preserve the LandedCostEstimate entity structure (all cost components as individual fields)
- Preserve the conservative estimation bias (overestimate costs when uncertain)
- Preserve the distinction between classification confidence and landed cost confidence
- Use "as-if-always" language

---

## Step 3: Fresh-Eyes Optimization

After completing Step 2, re-read the sections you edited with fresh eyes. Check for:

1. **§4.5 classification pipeline ↔ §5.7 landed-cost steps consistency:** Does the classification pipeline described in §4.5 match the HS classification step in §5.7? Same inputs, same outputs, same fallback logic? If §4.5 describes a multi-stage classification, does §5.7 reference all stages?

2. **§5.11 error types ↔ classification failure modes consistency:** Does every classification failure mode described in §4.5 (ambiguous product, API timeout, no matching HS code, multiple candidate codes) have a corresponding error type in §5.11? Are there error types in §5.11 that don't correspond to any described failure?

3. **Appendix A margin formula ↔ §5.4 LandedCostEstimate fields consistency:** Does every cost component in the Appendix A formula (duty, freight, insurance, handling, FX conversion) have a corresponding field in the LandedCostEstimate entity in §5.4? Are there fields in the entity that aren't used in the formula, or formula components without a field?

4. **§4.6 FX strategy ↔ LandedCostEstimate consistency:** Does the FX strategy in §4.6 (which rate, which source, staleness window) match how FX is applied in the landed cost calculation? Is the fx_rate field in LandedCostEstimate documented with which rate source and timestamp it represents?

5. **§4.5 ↔ §4.3 handoff consistency:** The handoff from supplier price intelligence (§4.3) to customs classification (§4.5) requires product descriptions and candidate commodity codes. Does §4.5 consume exactly what §4.3 produces? Are there missing fields needed for classification that aren't in the ProductCandidate entity?

6. **Net margin gate consistency:** Does the net margin gate described in §5.7 use the same formula and thresholds as Appendix A? Does the gate correctly reference all cost components from LandedCostEstimate?

7. **Terminology:** Ensure consistent use of "HS code" (not "tariff code" or "customs code" interchangeably), "commodity code" (when referring to UK 10-digit codes specifically), "duty rate" (not "tariff rate" when referring to the percentage), "landed cost" (not "total cost" or "import cost"), "CIF" (when referring to cost-insurance-freight specifically).

Fix any inconsistencies found. Make targeted edits, not wholesale rewrites.

---

## Step 4: Sync Pass

1. **Update §1 (Executive Summary):** If the customs classification or landed-cost pipeline changed materially (new classification stages, new cost components, changed FX strategy), update the executive summary to reflect the current state.

2. **Update Appendix E (Changelog):** Add a concise entry describing what was integrated in this review cycle. Include: which sections were modified, what the reviewer's key contributions were, and any deferred cross-domain items.

3. **Verify §6.1 (Complexity Analysis):** If the reviewer's feedback changes the complexity assessment for "Landed-Cost Engine" (currently rated Very High), update it with rationale.

4. **Verify §6.2 (Risk Assessment):** If new customs/classification risks were identified (e.g., "anti-dumping duties on specific Chinese product categories not detected," "HS code misclassification cascading to incorrect duty rate"), add them with likelihood/impact ratings.

5. **Check §10 (Open Questions):** If any open questions about customs, tariffs, FX, or landed cost have been resolved by this review cycle, mark them as resolved. If new questions emerged, add them.

6. **Verify Appendix D (Source References):** If the reviewer referenced specific HMRC APIs, trade agreement databases, FX rate providers, or freight rate sources, add them to Appendix D.
