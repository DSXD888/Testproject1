# Layer 5 — Expanded Commercial Intelligence

> **Authority:** This document is the execution-grade specification for Layer 5. An implementation agent should be able to build this layer from this document alone without reopening the research document.
>
> **Milestone:** M5
> **Dependencies:** Layers 0-4 (full pipeline, sync, web portal)
> **Exit criteria:** System handles >=3 goods categories with decision-grade landed cost and >=2 supplier sources per category.

> **Implementation Status (as of 2026-05-02):** Layer 5 is **PLANNED — not yet built**. No production decision-grade customs classification engine, expanded supplier-source portfolio, or L5 commercial-intelligence authority exists yet. This doc remains a planned execution-grade specification and intentionally has no inline implementation markers.

> **3-SOT Validation (2026-05-08):** Validated for downstream TDD, bead creation, and completion-audit coverage by the L5 pipemaxxer pipeline. Requirement IDs use `L5-FR-{section}.{subsection}.{item}` and acceptance criteria are listed immediately under the requirements they verify. Operator decisions are recorded in `## L5 Pipeline Decisions Recorded 2026-05-08`.

---

## 1. Overview & Customs Classification Engine

### 1.1 Layer Narrative
L5-FR-1.1.1 — Layer 5 expands screening-grade intelligence to decision-grade commercial authority
- AC1: The implementation upgrades HS6/single-rate/flat-freight screening into UK10 classification, measure interaction, exporter-rate resolution, dual FX, freight source cross-checking, and trade-remedy freshness.
- AC2: The implementation keeps Layer 6 optional: after L5, the platform is commercially authoritative for >=3 goods categories even if bid workflow automation is not built.

Broader categories, more supplier sources, decision-grade accuracy. The customs classification engine expands from screening-grade (HS6, single MFN rate, flat freight) to full multi-source recall with 10-digit resolution, measure interaction, and trade remedy monitoring. Additional marketplace skills (Alibaba/1688, Amazon) provide multi-source pricing. FX engine gains dual-rate capability (HMRC customs + commercial spot). Freight engine gains dual CDP/browser source capability (Freightos + SeaRates). Each new category requires its own gold-set validation before entering production.

**Why this layer exists:** Layers 0-4 proved the platform works with screening-grade intelligence on a narrow category whitelist. Layer 5 expands the commercial intelligence to decision-grade accuracy across broader categories. This is where the system becomes commercially authoritative.

**If we never build Layer 6:** The system is a mature procurement intelligence platform covering >=3 goods categories with decision-grade landed cost and >=2 supplier sources per category. Only request pipeline automation is missing.

### 1.2 Capability Delivered
L5-FR-1.2.1 — L5 handles at least three goods categories with decision-grade landed cost
- AC1: Fasteners, packaging-cartons, and examination-gloves each have calibration fixtures and decision-grade landed-cost outputs.
- AC2: Decision-grade outputs include customs, FX, freight, VAT, confidence, blocker, and evidence-leverage fields listed in §2.4.
L5-FR-1.2.2 — L5 provides at least two supplier sources per validated category
- AC1: Each validated category has eBay plus at least one of Alibaba, Amazon, or 1688 supplier quote rows.
- AC2: Quote roles remain explicit: marketplace ceilings do not silently become executable wholesale quotes.
L5-FR-1.2.3 — L5 provides full customs classification
- AC1: Multi-source recall, multi-family expansion, UK10 resolution, exporter resolution, measure interaction, additional-code satisfiability, VAT rate determination, trade-remedy watch state, and conservative defaults all run before net margin gating.
L5-FR-1.2.4 — L5 provides dual FX and dual freight source intelligence through skill-first and public/free data access
- AC1: HMRC customs FX and Frankfurter commercial spot FX populate dual-rate fields.
- AC2: Freightos and SeaRates CDP/browser skills provide low/base/high freight estimates and disagreement signals.
L5-FR-1.2.5 — L5 requires per-category gold-set expansion before production use
- AC1: A category cannot enter production until its `customs-classification/<category>/` calibration corpus exists and passes §3.4 metrics.

### 1.3 Data Changes
L5-FR-1.3.1 — L5 populates existing `landed_cost_estimates` columns without adding columns to that table
- AC1: All decision-grade `landed_cost_estimates` fields listed in §2.4 are populated through existing nullable/non-null columns from `migrations/0001_sqlite_state_initial.sql`.
- AC2: L5 does not add new columns to `landed_cost_estimates`.
L5-FR-1.3.2 — L5 introduces the local SQLite `product_commodity_codes` junction
- AC1: L5 creates `product_commodity_codes` in `migrations/0013_l5_product_commodity_codes.sql` or the next-free `NNNN_l5_product_commodity_codes.sql` at implementation time.
- AC2: The Supabase projection table of the same name, created in L4, is the push target.
- AC3: Local rows use set-replacement semantics per candidate when classification reruns, matching the L5 classification lattice contract in `docs/strategy/layer-strategy.md` §10.
L5-FR-1.3.3 — L5 activates additional `supplier_quotes` rows without changing quote schema
- AC1: Alibaba, 1688, and Amazon rows join existing eBay rows in `supplier_quotes`.
- AC2: Each source row carries source-specific `price_signal_role`, origin/dispatch, quote confidence, unit basis, and evidence fields already present in L0 DDL.

| Table | Layer 5 Change |
|-------|----------------|
| `landed_cost_estimates` | Full duty breakdown, measure interaction results, exporter rate basis, supplementary unit requirements, valuation mode, customs valuation method, FX, freight, CBAM exposure, freshness, and confidence fields are populated from existing columns |
| `product_commodity_codes` | L5 creates the local SQLite junction; L4 Supabase projection becomes the push target |
| `supplier_quotes` | Multi-source quotes: Alibaba, 1688, Amazon rows alongside existing eBay rows |

### 1.4 Multi-Source HS Classification Recall
L5-FR-1.4.1 — Multi-source recall uses five parallel recall sources
- AC1: Every classification run attempts all applicable recall sources in the table below and records source availability, candidates, retrieval scores, and failure mode.
- AC2: If one recall source is unavailable, remaining sources still run and confidence is penalized rather than fabricated.
- AC3: Independent-source agreement means agreement across distinct source families with distinct data origin and retrieval logic; multiple query waves, translated terms, or endpoints inside one source family count once.

| Source | Method | Output | Confidence Signal |
|--------|--------|--------|------------------|
| **HMRC FPO or public search-suggestion endpoint** | ML/search-powered commodity suggestion from product description; endpoint name is unresolved until TDD investigation | HS6/CN8 candidates with retrieval scores | Retrieval relevance, not legal confidence |
| **CPV-to-chapter priors** | Deterministic mapping from procurement CPV codes to likely tariff chapters | Chapter/heading candidates | Statistical correlation, not legal mapping |
| **Rule-based chapter hints** | Match head noun + material + function against tariff chapter keyword tables | Chapter candidates | Pattern-based, high precision on known product families |
| **HMRC tariff keyword search** | Search HMRC Trade Tariff heading/subheading descriptions for extracted product terms | Heading/subheading matches | Text similarity, may miss specialized vocabulary |
| **ATR/codebook reuse** | Previously classified similar products with cached codebooks from prior ATRs | Full 10-digit codes + measure profiles | Highest confidence when available |
L5-FR-1.4.2 — FPO agreement increases confidence but FPO alone is not evidence
- AC1: FPO/search-suggestion output alone never unlocks auto-classification.
- AC2: Agreement must be evaluated through independent source families and slot-level compatibility against the tariff hierarchy.
- AC3: Step 03 TDD creates investigation bead `l5-customs-hmrc-fpo-endpoint-discovery` to determine whether `/api/search`, `/api/search_suggestions`, a public FPO endpoint, or an operator decision about Developer Portal registration is required. HMRC Trade Tariff commodity APIs are already public and require no Developer Portal key.
L5-FR-1.4.3 — HMRC remains the UK legal authority
- AC1: EU TARIC cross-checks and any advisory recall tools never overrule HMRC on UK national measures, VAT additional codes, UK quotas, or UK trade remedies.
- AC2: A cross-check disagreement is recorded as evidence divergence and cannot overwrite HMRC UK measure data.
L5-FR-1.4.4 — Preprocessing before recall rejects low-quality descriptions and extracts classification facts
- AC1: Description-quality gate rejects bundles, service descriptions, and generic nouns without required archetype facts, and emits archetype-keyed missing-facts requests.
- AC2: Bundle splitter decomposes multi-item descriptions when safe and marks mixed/manual or GIR 3 candidate when decomposition is unsafe.
- AC3: Attribute extraction captures head noun, material, principal function, sector, sterility/medical status, power/size, and pack form.
- AC4: Multi-query recall runs normalized variants across all available recall sources.
L5-FR-1.4.5 — Recall source error paths are explicit
- AC1: HMRC Trade Tariff rate-limit, downtime, and schema drift produce a blocked or stale classification path, not guessed tariff data.
- AC2: FPO/search-suggestion unavailable or malformed responses reuse existing daemon code `FPO_ACCESS_UNAVAILABLE` and continue with remaining sources.
- AC3: EU TARIC disagreement is advisory evidence divergence; HS Ping is declined for L5 and not required for green status.

### 1.5 Multi-Family Expansion to UK 10-Digit
L5-FR-1.5.1 — Classification never uses greedy descent
- AC1: The engine expands all plausible HS6/CN8 families to all declarable UK10 descendants through HMRC tariff hierarchy data.
- AC2: A plausible runner-up family is not discarded until local scoring and shortlisting have run.
L5-FR-1.5.2 — Descendant reranking uses slot and measure compatibility
- AC1: Extracted product attributes are scored against code descriptions, exclusions, scope notes, unit requirements, and measure profiles.
- AC2: `candidateRecallAgreement` records which recall sources agreed, diverged, or were unavailable for every candidate.
L5-FR-1.5.3 — Expansion separates classification lattice from commercial scenario lattice
- AC1: Recall collects HS6/CN8 families; local cache expands descendants; local scoring shortlists top 3-5 non-dominated branches per family; external validation runs only on shortlisted candidates.
- AC2: Commercial variables (quotes, packing, Incoterms, exporter bases, quota states, FX) bind only after code-family pruning.
L5-FR-1.5.4 — HMRC tariff data is locally cached as an indexed graph
- AC1: Runtime classification reads hierarchy, measures, supplementary-unit requirements, and additional-code requirements from local cache.
- AC2: Live HMRC verification runs for shortlisted candidates and cache refresh follows §1.11.
L5-FR-1.5.5 — Operator presentation groups branches into family clusters
- AC1: UI/API output groups 50+ descendant sets by family label, count, conservative duty outcome, most-supported branch, blockers, and missing facts.
- AC2: Flat ungrouped UK10 lists are not an acceptable operator surface.

### 1.6 Exporter/Manufacturer Resolution
L5-FR-1.6.1 — Trade-remedy exporter resolution defaults to residual rate when identity is unverified
- AC1: `exporterRateBasis` is one of `named_exporter`, `annex_exporter`, or `residual_default`.
- AC2: `residual_default` is used whenever exporter/manufacturer proof is absent or insufficient.
- AC3: `exporterIdentityConfidenceBps` records 0-10000 bps confidence based on supplier listing evidence quality.
L5-FR-1.6.2 — Named-exporter relief requires manufacturer/exporter evidence
- AC1: Named or annex exporter rates require listing, invoice, certificate, or supplier-profile evidence tying the product to the eligible manufacturer/exporter.
- AC2: Distributor names, marketplace seller names, or vague "factory direct" claims do not satisfy named-exporter proof by themselves.
- AC3: Insufficient evidence emits `EXPORTER_UNRESOLVED`, applies the residual rate, and records the missing evidence in `confidence_breakdown.evidence_leverage`.

Trade remedy measures on Chinese goods frequently carry exporter-specific rates that are materially lower than the residual rate. Examples: aluminium foil 14.2%-35.6% residual depending on exporter/additional-code/invoice path; aluminium extrusions 0%-35.1% residual; tin mill products 27.85% cooperating exporter versus 49.98% all other Chinese exporters. Systematic understatement through assumed named-exporter rates is catastrophic.

### 1.7 Measure Interaction Engine
L5-FR-1.7.1 — Measure interaction is family-specific, not simple stacking
- AC1: Additive families compute base duty plus AD/CVD or other additive measures.
- AC2: Substitutive families compute the specified substitution rule, such as greater-of safeguard or AD, without double-charging.
- AC3: Conditional families model quota exhaustion, additional-code eligibility, and rate switches.
- AC4: Unknown interaction rules emit `MEASURE_INTERACTION_UNKNOWN` and block auto-decision.
L5-FR-1.7.2 — Additional-code satisfiability is modeled as a legal prerequisite
- AC1: HMRC DE 6/17 and other national additional-code requirements record prerequisites and documentary evidence.
- AC2: When satisfiability is unknown, the engine applies the rate that applies without the additional code.
- AC3: `additional_codes` and `additional_code_requirements_json` distinguish measure existence from truthful eligibility to claim the code.
L5-FR-1.7.3 — Measure scanner classifies every active measure into an interaction family
- AC1: Each active HMRC measure on a shortlisted UK10 branch is labeled `additive`, `substitutive`, `conditional`, `documentary_only`, or `unknown` before duty computation.
- AC2: Unknown or unmapped interaction families emit `MEASURE_INTERACTION_UNKNOWN` and cannot auto-approve the affected branch.
L5-FR-1.7.4 — Golden interaction cases cover additive, substitutive, and conditional families
- AC1: §3.3 scenarios cover steel AD/CVD, aluminium exporter-specific residual rates, quota/additional-code branching, and compound specific-duty examples.
- AC2: `measure-interaction.test.ts` maps each interaction family to at least one fixture before L5 implementation beads are created.

### 1.8 Compound/Specific Duty Expressions
L5-FR-1.8.1 — Duty expressions preserve full component structure
- AC1: Ad valorem, specific per-unit, and compound measures are stored in `duty_breakdown_json`.
- AC2: Compound duties are never flattened to a single `dutyPct`.
L5-FR-1.8.2 — Supplementary units are first-class blockers
- AC1: Required units such as kg net, litres, pairs, item count, or per-1000 quantity are recorded in `supplementary_units_required_json`.
- AC2: Missing required unit data emits `SUPPLEMENTARY_UNIT_MISSING`, abstains on that duty component, and does not average or estimate the missing unit.
L5-FR-1.8.3 — Bundle splitter includes a GIR 3 set-candidate escape hatch
- AC1: Retail sets or kits that may classify by essential character or last-in-tariff-order are flagged for manual classification.
- AC2: Unsafe forced decomposition is prohibited.
L5-FR-1.8.4 — Specific-duty unit conversion requires explicit compatible quantity evidence
- AC1: Specific or compound duties use only quantity units explicitly present in supplier quotes, extracted lot documents, packing scenarios, or operator-entered evidence.
- AC2: Incompatible conversions, such as pieces to kg without net-weight evidence, emit `SUPPLEMENTARY_UNIT_MISSING` rather than deriving an average weight.

### 1.9 VAT Rate Determination
L5-FR-1.9.1 — VAT rate and VAT recovery are separate decisions
- AC1: `vatRateState` is one of `standard`, `reduced`, `zero`, or `unresolved`.
- AC2: `vatRecoveryRateBps` records 0-10000 bps recovery fraction and is not a binary flag.
- AC3: Unresolved VAT status defaults to standard rate.
L5-FR-1.9.2 — VAT additional-code relief paths require evidence
- AC1: VATZ/VATR and family 6/17 relief paths are recorded only when commodity and documentary prerequisites are satisfied.
- AC2: Missing relief evidence preserves standard-rate treatment and records evidence leverage.

### 1.10 Valuation Mode
L5-FR-1.10.1 — Valuation mode distinguishes estimation from declaration
- AC1: `estimation` uses marketplace-derived proxy values and stores `customs_valuation_method = 1_proxy`.
- AC2: `declaration` uses invoice/contract transaction values and stores `customs_valuation_method = 1_actual` unless Method 2-6 or `unknown` applies.

| Mode | When | Accuracy |
|------|------|----------|
| `estimation` (`1_proxy`) | Marketplace-derived proxy for transaction value | Indicative only; listing prices are not invoice prices |
| `declaration` (`1_actual`) | Actual transaction price from invoice/contract | Decision-grade, but may require Method 2-6 if no-sale |
L5-FR-1.10.2 — Invoice price is not automatically customs value
- AC1: Selling commissions, assists, royalties, and resale-proceeds shares are represented in valuation evidence or flagged unknown.
- AC2: Material divergence between proxy and declaration mode emits `VALUATION_MODE_CONFLICT`.

### 1.11 Classification Freshness System
L5-FR-1.11.1 — L5 tracks four independent freshness vectors
- AC1: Structure, measures, remedies, and FX freshness are tracked separately through `tariffCheckedAt`, `measuresFreshUntil`, `tradeRemedyWatchState`, and `fxRateCheckedAt`.
- AC2: Expiry in one vector cannot be masked by freshness in another vector.

| Vector | Field | What It Watches | Update Frequency |
|--------|-------|----------------|-----------------|
| **Structure** | `tariffCheckedAt` | Commodity-code structure changes | Weekly HMRC tariff update notice check |
| **Measures** | `measuresFreshUntil` | Duty rates, VAT measures, preferences, MFN corrections, additional-code requirements | Weekly HMRC correction check |
| **Remedies** | `tradeRemedyWatchState` | TRA investigations, provisional measures, definitive measure changes | Daily TRA case register check |
| **FX** | `fxRateCheckedAt` | HMRC monthly customs FX rate changes | Monthly rollover detection |
L5-FR-1.11.2 — Freshness watchers enqueue recalculation or reclassification
- AC1: TRA investigation openings set `tradeRemedyWatchState = investigation_open` and enqueue affected commodity families for review.
- AC2: HMRC tariff structure or measure changes emit a tariff-change control event and enqueue affected cached classifications.
- AC3: FX monthly rollover enqueues affected landed-cost estimates for recalculation.
- AC4: Implementation adds four new `CronJobDefinition` entries in `crates/daemon/src/scheduler/cron.rs` following the existing `VALIDATION_CALIBRATION_READINESS_SWEEP_CRON` and `VALIDATION_CROSS_MODEL_AUDIT_CRON` pattern.
L5-FR-1.11.3 — Expired classification freshness downgrades bid readiness
- AC1: If `classification_fresh_until` expires before the expected import window, the estimate records a blocker and cannot satisfy the L5→L6 readiness gate.
- AC2: Expired structure, measure, or remedy vectors cannot be cleared by fresh FX alone.
L5-FR-1.11.4 — Freshness watcher failures are observable and conservative
- AC1: A missed watcher run beyond its freshness window marks affected commodity families stale and enqueues catch-up work.
- AC2: A multi-day daemon outage creates stale classification blockers on affected cached estimates until watcher catch-up completes.

### 1.12 Conservative Defaults (Constitutional)
L5-FR-1.12.1 — Conservative defaults apply whenever resolution is incomplete
- AC1: The defaults in the table below are mandatory and cannot be relaxed to improve apparent margin.
- AC2: Each active default is recorded in `confidence_breakdown`.

| Parameter | Default | Rationale |
|-----------|---------|-----------|
| Origin preference | MFN rate, not preferential | No origin proof available |
| Exporter rate | Residual remedy rate, not named-exporter | No exporter proof |
| Quota allocation | Out-of-quota rate | No quota allocation proof |
| VAT rate | Standard rate | Unresolved VAT status |
| Incoterm | Boundary scenarios | No stated Incoterm |
| Supplementary units | Abstain on duty for that component | Unresolved unit data |
L5-FR-1.12.2 — Conservative defaults surface unlockable margin instead of relaxing assumptions
- AC1: `confidence_breakdown.unlockable_margin_bps` stores margin improvement available if documented evidence replaces resolvable conservative defaults.
- AC2: `confidence_breakdown.evidence_leverage` stores per-parameter evidence needed and margin delta.
- AC3: Evidence leverage is computed for named exporter proof, origin proof, quota allocation, VAT relief, Incoterm clarification, and supplementary units when applicable.
L5-FR-1.12.3 — Each conservative default has a deterministic trigger and fixture
- AC1: The six default rows above are triggered by absence of origin proof, exporter proof, quota allocation proof, VAT status, Incoterm, or compatible supplementary-unit data respectively.
- AC2: `customs-classification.test.ts` or `measure-interaction.test.ts` includes at least one fixture for each default row.
L5-FR-1.12.4 — Evidence leverage margin deltas are differential recalculations
- AC1: For each leverage item, `margin_delta_bps` is the difference between `net_margin_base_bps` under the evidence-resolved branch and `net_margin_base_bps` under the conservative default, using the same quote, freight, and FX snapshot.
- AC2: Step 03 TDD must lock any category-specific arithmetic details before bead creation, and L5 implementation must emit decision-grade `confidence_breakdown.evidence_leverage[]` entries rather than prose-only hints.
- AC3: If evidence cannot change the conservative outcome, `margin_delta_bps = 0` and the leverage item is omitted unless the missing evidence remains a hard blocker.

### 1.13 Customs Branch Divergence
L5-FR-1.13.1 — Material branch divergence is a hard blocker
- AC1: UK10 branches split on remedy/no-remedy, duty type, or material duty spread emit `CLASSIFICATION_AMBIGUOUS` or `TARIFF_BRANCH_DIVERGENCE`.
- AC2: Divergent branches are never averaged into one actionable estimate.
- AC3: Until gold-set calibration replaces it with a documented test constant, `customs_cost_spread_bps >= 1500` emits `TARIFF_BRANCH_DIVERGENCE`; remedy/no-remedy and duty-type splits block regardless of spread bps.
L5-FR-1.13.2 — Screening-grade auto-pass is allowed only under all three safeguards
- AC1: `worstCaseNetMargin >= gate`.
- AC2: `customsCostSpread` is immaterial under the calibrated threshold and all branches remain in the same measure family.
- AC3: No catastrophic legal branch exists, including residual trade remedy, missing supplementary unit, unresolved additional-code satisfiability, or expired freshness for the expected import window.

### 1.14 CBAM Forward Placeholder
L5-FR-1.14.1 — CBAM remains a forward placeholder until UK rate rules are final
- AC1: `cbamEstimateGbp`/`cbam_estimate_gbp_pence` remains null until secondary legislation finalises rates and calculation method.
- AC2: Confirmed 2027 sector scope is aluminium, cement, fertiliser, hydrogen, iron and steel; glass and ceramics are not in 2027 scope.
L5-FR-1.14.2 — Tender windows crossing 2027-01-01 surface future regulatory exposure
- AC1: Tenders with delivery windows crossing 2027-01-01 and affected commodity families set `cbam_exposure_flag = 1` even when the estimate remains null.
- AC2: The flag is an exposure warning, not a quantified cost.
L5-FR-1.14.3 — CBAM exposure flag has explicit clear conditions
- AC1: `cbam_exposure_flag` is left `0` or null when the commodity family is outside confirmed 2027 scope.
- AC2: `cbam_exposure_flag` is left `0` or null when the expected import or delivery window ends before 2027-01-01.
- AC3: `cbam_estimate_gbp_pence` remains null while exposure flagging can still be active.

### 1.15 Layer 5 Skill Work

Repository baseline verified during validation: `skills/supplier-price-enrichment/{ebay,alibaba,amazon,1688}/SKILL.md` all exist and currently declare `promotion_stage: candidate`. L5 matures Alibaba, 1688, and Amazon because they are the expanded-source deltas; eBay remains the existing baseline supplier source.
All L5 production-bound skill directories follow the L0/L4 directory standard: `SKILL.md`, `result.schema.json`, `checkpoint.schema.json`, `canary.inputs.jsonl`, `golden.assertions.yaml`, and `validator-rubric.yaml`.
L5-FR-1.15.1 — Existing marketplace skills mature from candidate to decision-grade
- AC1: `skills/supplier-price-enrichment/alibaba/`, `1688/`, and `amazon/` are promoted from `promotion_stage: candidate` to stable only after the deltas below and validator overlay binding are complete.
- AC2: Phase 5 bead scope is limited to the listed deltas and promotion; no speculative gap-fill work is added.
- AC3: eBay baseline plus Alibaba, 1688, and Amazon production bindings include canary fixtures, golden assertions, and validator rubric artifacts before stable promotion.

| Mature from candidate | L5 Delta |
|-----------------------|----------|
| `skills/supplier-price-enrichment/alibaba/` | Chinese retry on zero/accessory/consumer hits; Ready-to-Ship filter bumps `quoteUsabilityConfidence`; source-health canary expansion; anti-bot circuit breaker; validator overlay binding |
| `skills/supplier-price-enrichment/1688/` | Short Chinese-keyword-only clusters (2-4 terms); domestic wholesale floor pricing; auth-refresh/1688 coordination; validator overlay binding |
| `skills/supplier-price-enrichment/amazon/` | Alias families; structured title vocabulary; browser-first strategy after PA-API 5.0 sunset on 2026-04-30; validator overlay binding |
L5-FR-1.15.2 — PCS Scotland ingestion is truly new and in scope
- AC1: Phase 5 creates `skills/procurement-ingestion/pcs/` with `SKILL.md`, `result.schema.json`, `checkpoint.schema.json`, `canary.inputs.jsonl`, `golden.assertions.yaml`, and `validator-rubric.yaml`.
- AC2: `crates/daemon/src/sources/registry.rs` gains the PCS source entry near existing procurement ingestion sources after `procontract-enrich` and before supplier sources.
- AC3: PCS is not conditional in L5 implementation; the 2026-05-08 operator decision closes the scope question as IN-SCOPE.

### 1.16 Intelligence Package Additions (19 files)
L5-FR-1.16.1 — L5 adds customs, FX, logistics, validation, and accuracy modules under `packages/intelligence`
- AC1: The 19-file package surface below exists and is covered by §3.2 tests.
- AC2: Validation comparison adapters remain comparison-only and use fixtures or captured samples, with no external vendor credentials.
L5-FR-1.16.2 — Intelligence package file counts are fixed for bead planning
- AC1: The package additions total exactly 19 files: 6 customs, 4 FX, 5 logistics, 3 validation, and 1 accuracy module.
- AC2: Any additional file proposed during implementation must be justified as new functionality in the owning bead and cannot replace one of the listed files by omission.

| Package | Files | Purpose |
|---------|-------|---------|
| `packages/intelligence/src/customs/` | 6 files | Description-quality gate, multi-source recall, 10-digit resolution, measure scanner, exporter resolution, classification freshness |
| `packages/intelligence/src/fx/` | 4 files | HMRC customs rates, Frankfurter commercial rates, basis gap tracking, margin-triggered recalculation |
| `packages/intelligence/src/logistics/` | 5 files | Mode selection, Freightos skill adapter, SeaRates skill adapter, dual-packing scenarios, low/base/high estimates |
| `packages/intelligence/src/validation/` | 3 files | Landed-cost comparison adapters for Dutify/Zonos/Avalara fixtures or captured samples only; no external vendor credentials |
| `packages/intelligence/src/customs/accuracy.ts` | 1 file | Evidence-tiered classification accuracy measurement |

---

## 2. FX, Freight & Multi-Source Marketplace Search

### 2.1 Dual FX Engine (`packages/intelligence/src/fx/` -- 4 files)
L5-FR-2.1.1 — Dual FX uses HMRC customs rates and Frankfurter commercial spot rates
- AC1: `customsFxRate` comes from HMRC Exchange Rates for Customs and VAT monthly publication.
- AC2: `commercialFxRate` comes from Frankfurter through the existing `skills/landed-cost/simple-fx-lookup/` extension.
- AC3: No commercial FX account is required.

| Rate | Source | Purpose | Update Frequency |
|------|--------|---------|-----------------|
| `customsFxRate` | HMRC Exchange Rate Service | Legally authoritative for customs duty calculation | Monthly rolling |
| `commercialFxRate` | Frankfurter (ECB-backed) | Spot rate for commercial margin estimation | On-demand/cache TTL |
L5-FR-2.1.2 — FX basis delta and rollover behavior are explicit
- AC1: `fxBasisDeltaBps` stores the customs/commercial rate gap in basis points.
- AC2: In-flight estimates use the HMRC monthly rate effective for the calculation timestamp; if rollover publishes before decision, the estimate is marked stale and recalculated.
- AC3: HMRC corrections for a historical month create a new rate snapshot, preserve the prior snapshot for audit, and enqueue affected historical estimates.
L5-FR-2.1.3 — FX error paths block unsafe landed-cost computation
- AC1: HMRC missing month or parse failure emits `FX_RATE_MISSING` when no valid customs rate snapshot exists.
- AC2: Frankfurter rate-limit or downtime uses the last valid commercial snapshot only if TTL-valid; otherwise commercial margin confidence is reduced and blocker/warning state is recorded.
L5-FR-2.1.4 — FX basis delta can trigger margin recalculation or review
- AC1: When `abs(fx_basis_delta_bps)` crosses the Step 03 TDD threshold, the engine recomputes `net_margin_low_bps`, `net_margin_base_bps`, and `net_margin_high_bps` under both customs and commercial FX bases.
- AC2: The selected threshold is a named test constant in `fx-engine.test.ts`; untested hardcoded FX-delta thresholds are prohibited.

| File | Purpose |
|------|---------|
| `hmrc-exchange-rates.ts` | Fetch monthly HMRC rates, cache locally, detect rollovers/corrections |
| `frankfurter-commercial-rates.ts` | Fetch commercial spot rates through the free public endpoint |
| `fx-engine.ts` | Select correct rate per use case, compute basis delta, trigger recalculation |
| `fx-types.ts` | `FxSnapshot`, `FxBasisDelta`, `FxRecalculationTrigger` |

### 2.2 Full Freight Engine (`packages/intelligence/src/logistics/` -- 5 files)
L5-FR-2.2.1 — Freight mode selection covers sea, air, express, and multimodal with hard locks
- AC1: Mode selection considers weight, volume, value, urgency, product type, hazmat, oversized, fragile, reefer, batteries, and packaging state.
- AC2: Hard locks override cheaper modes when safety or feasibility requires.
L5-FR-2.2.2 — Freight estimates use Freightos and SeaRates CDP skills
- AC1: The engine invokes `skills/landed-cost/freightos-freight-estimate/` and `skills/landed-cost/searates-freight-estimate/` for dual-source cross-check.
- AC2: Large provider disagreement records `FREIGHT_MODE_LOW_CONFIDENCE` in `warnings` or `blockers`; §3.5 remains the 10-code daemon error-vocabulary extension listed there.
- AC3: Freight access remains skill-first/CDP-browser based and requires no commercial connector account.
L5-FR-2.2.3 — Freight conservative defaults are explicit
- AC1: Unknown packaging runs dual scenarios such as palletized versus loose cartons.
- AC2: Missing Incoterm runs EXW/FOB/CIF boundary scenarios.
- AC3: If neither provider can quote, emit `FREIGHT_ESTIMATE_FAILED`, preserve a manual-estimate requirement, and do not fabricate provider data.
- AC4: While product-family logistics heuristics remain uncalibrated, unknown item class or missing dimensional data uses the highest-cost feasible boundary scenario and records the missing logistics facts.
L5-FR-2.2.4 — Freight error paths are source-specific
- AC1: Freightos no quote, unsupported mode, or page/schema drift is recorded with source name and transcript evidence.
- AC2: SeaRates no quote, unsupported mode, or schema drift is recorded separately.
- AC3: A single-provider failure degrades confidence; dual-provider failure blocks automated landed-cost completion.

| File | Purpose |
|------|---------|
| `mode-selector.ts` | Shipping mode selection with hard locks |
| `freightos-skill-adapter.ts` | Freightos CDP skill invocation and result normalization |
| `searates-skill-adapter.ts` | SeaRates CDP skill invocation and result normalization |
| `freight-engine.ts` | Incoterm boundary, provider cross-check, dual packing, low/base/high estimates |
| `freight-types.ts` | `FreightEstimate`, `ModeSelection`, `PackingScenario` |

### 2.3 Multi-Source Marketplace Search Skills
L5-FR-2.3.1 — Alibaba search remains an optimistic-floor signal with stricter recovery
- AC1: English query waves run first; Chinese retry runs when English returns zero, accessory, or consumer-item results.
- AC2: Ready-to-Ship listings bump `quoteUsabilityConfidenceBps` only when unit/MOQ/shipping clarity is present.
- AC3: Source-health canary expansion and anti-bot circuit breaker run before production jobs.
- AC4: Alibaba rows keep `price_signal_role = optimistic_floor`.
- AC5: Chinese retry results must pass a back-translation/head-noun compatibility check before supplier quote emission; ambiguous translations become warnings, not quotes.
L5-FR-2.3.2 — 1688 search is domestic wholesale floor evidence, not regulated-goods execution evidence
- AC1: Queries use Chinese-only short clusters of 2-4 terms.
- AC2: Results without explicit compliance/export evidence cannot drive landed-cost decisions for regulated goods.
- AC3: Auth refresh uses `auth-refresh/1688` before production search.
- AC4: 1688 rows keep `price_signal_role = domestic_wholesale_floor`.
- AC5: `VOCABULARY_MISMATCH_EN_ONLY` or equivalent zero-result vocabulary failures require lexicon expansion rather than treating the product as unavailable.
L5-FR-2.3.3 — Amazon search is browser-first benchmark-ceiling evidence
- AC1: Query waves use alias families and structured title vocabulary.
- AC2: PA-API 5.0 is not used after its 2026-04-30 sunset; browser-first operation is required.
- AC3: Amazon rows keep `price_signal_role = benchmark_ceiling`.
L5-FR-2.3.4 — Marketplace source registration is explicit
- AC1: Alibaba and 1688 existing registry entries gain production bindings when promoted.
- AC2: Amazon gains a supplier source entry in `crates/daemon/src/sources/registry.rs` adjacent to Alibaba/1688 supplier registrations.
- AC3: Validator overlay binding records worker/validator behavior versions per source.
L5-FR-2.3.5 — Skill promotion uses the full production directory and validator standard
- AC1: eBay, Alibaba, 1688, Amazon, and PCS production bindings include `SKILL.md`, `result.schema.json`, `checkpoint.schema.json`, `canary.inputs.jsonl`, `golden.assertions.yaml`, and `validator-rubric.yaml`.
- AC2: Stable promotion records compatible `worker_behavior_version` and `validator_behavior_version`; incompatible versions block promotion rather than silently recalibrating.
L5-FR-2.3.6 — Alibaba maturation preserves the running SearchProfile and output contract
- AC1: Alibaba reads every `search_profiles[]` field required by the existing skill, including critical attributes, numeric specs, standards, aliases, exclusions, query waves, and eligibility state; implementations that collapse a profile to only head noun or candidate name fail validation.
- AC2: Alibaba verifies authenticated persistent-profile browser state before the first query; unauthenticated sessions, CAPTCHA blocks, locale/currency drift, page-shape drift, and hard-conflict-zero-survivor cases emit source-specific failure codes and do not fabricate quotes.
- AC3: Alibaba filtering preserves the existing hard-conflict rules for head-noun/alias conformance, required critical attributes, numeric-spec tolerance, exclusion terms, and regulated-goods standards conflicts.
- AC4: Alibaba shortlist/detail behavior preserves max 2 listings per seller, max 2 per query wave, max 1 per normalized title family, and top 3 detail-page enrichment for specs, seller metadata, shipping/Incoterm facts, and compliance evidence tier.
- AC5: Alibaba confidence scoring remains formula-backed in bps for `match_confidence_bps` and `quote_usability_confidence_bps`; Step 03 TDD must assert the current weighted formulas or record a deliberate formula replacement before bead creation.
- AC6: Every Alibaba run emits schema-valid `result.json` with `search_attempts[]` and `supplier_quotes[]` plus `run-manifest.md`; failed batches still emit `supplier_quotes: []`, attempted-search metadata, and non-null `failure_code`.

### 2.4 Existing Field Activation (Self-Contained for L5 Implementer)
L5-FR-2.4.1 — L5 populates every `landed_cost_estimates` column or preserves its L4 sync semantics
- AC1: Every row below maps each L0 DDL column to the requirement that populates or preserves it.
- AC2: No listed column is treated as dead schema during L5 implementation.

| Field(s) | Populating / Preserving Requirement(s) | Now Contains |
|----------|----------------------------------------|-------------|
| `id`, `process_id`, `notice_id`, `product_candidate_id`, `supplier_quote_id` | L5-FR-2.4.1 | Existing identity and foreign-key chain for candidate-quote scenario |
| `shipping_mode`, `mode_confidence_bps`, `packing_scenario` | L5-FR-2.2.1, L5-FR-2.2.3 | Selected mode, confidence, and packaging scenario |
| `declarable_commodity_code_10`, `candidate_commodity_codes_json` | L5-FR-1.4.1, L5-FR-1.5.1 | UK10 declarable code and full recall candidate set |
| `additional_codes`, `additional_code_requirements_json` | L5-FR-1.7.2 | Additional-code requirements and satisfiability |
| `supplementary_units_required_json` | L5-FR-1.8.2 | Required supplementary units or blocker |
| `commodity_confidence_bps`, `classification_stability_bps` | L5-FR-1.5.2, L5-FR-3.4.1 | Classification confidence and stability |
| `customs_valuation_method`, `valuation_mode` | L5-FR-1.10.1, L5-FR-1.10.2 | Valuation basis and mode |
| `customs_fx_rate`, `commercial_fx_rate`, `fx_basis_delta_bps`, `fx_rate_checked_at` | L5-FR-2.1.1, L5-FR-2.1.2, L5-FR-2.1.4 | Dual FX rates, basis delta, checked timestamp, and FX-delta recalculation triggers |
| `supplier_cost_gbp_pence` | L5-FR-2.3.1, L5-FR-2.3.2, L5-FR-2.3.3, L5-FR-2.3.6 | Normalized supplier quote cost |
| `freight_low_gbp_pence`, `freight_base_gbp_pence`, `freight_high_gbp_pence` | L5-FR-2.2.2, L5-FR-2.2.3 | Freight low/base/high estimates |
| `customs_duty_gbp_pence`, `trade_remedy_duty_gbp_pence`, `duty_outcome_min_gbp_pence`, `duty_outcome_max_gbp_pence`, `customs_cost_spread_bps` | L5-FR-1.7.1, L5-FR-1.8.1, L5-FR-1.13.1 | Duty outcomes and spread across branches |
| `trade_remedy_state`, `trade_remedy_cases_json` | L5-FR-1.6.1, L5-FR-1.6.2, L5-FR-1.11.2 | Remedy applicability, exporter evidence, and case references |
| `duty_breakdown_json`, `measure_interaction_json` | L5-FR-1.7.1, L5-FR-1.8.1 | Full duty expression and interaction outcome |
| `exporter_rate_basis`, `exporter_identity_confidence_bps`, `origin_evidence_state` | L5-FR-1.6.1, L5-FR-1.6.2, L5-FR-1.12.1 | Exporter and origin evidence basis |
| `import_vat_gbp_pence`, `vat_rate_state`, `import_vat_treatment`, `vat_recovery_rate_bps` | L5-FR-1.9.1, L5-FR-1.9.2 | VAT amount, treatment, and recovery fraction |
| `uk_final_mile_cost_gbp_pence`, `other_fees_gbp_pence`, `economic_landed_cost_gbp_pence`, `cash_outlay_gbp_pence`, `cash_to_clear_gbp_pence`, `working_capital_cost_gbp_pence` | L5-FR-2.2.3, L5-FR-1.12.2 | Cost rollup, cash outlay, clearance cash, and working-capital cost |
| `revenue_basis_type`, `revenue_confidence_bps`, `net_margin_low_bps`, `net_margin_base_bps`, `net_margin_high_bps` | L5-FR-1.2.1, L5-FR-1.13.2 | Revenue basis and net margin band |
| `customs_resolution_confidence_bps`, `commercial_estimate_confidence_bps`, `decision_grade`, `confidence_bps`, `confidence_breakdown`, `warnings`, `blockers` | L5-FR-1.12.2, L5-FR-1.12.4, L5-FR-1.13.1, L5-FR-3.4.2 | Confidence, evidence leverage, decision grade, warnings, and blockers |
| `tariff_checked_at`, `measures_fresh_until`, `classification_fresh_until`, `trade_remedy_watch_state` | L5-FR-1.11.1, L5-FR-1.11.2, L5-FR-1.11.3, L5-FR-1.11.4 | Independent freshness vectors and expired-classification blockers |
| `cbam_exposure_flag`, `cbam_estimate_gbp_pence` | L5-FR-1.14.1, L5-FR-1.14.2, L5-FR-1.14.3 | CBAM exposure flag and nullable forward estimate |
| `owner_id`, `row_version`, `last_local_seq`, `stage_run_id`, `updated_at`, `created_at` | L4 sync contract, L5-FR-1.3.1 | Existing push-owned sync and audit fields preserved on L5 writes |
L5-FR-2.4.2 — `product_commodity_codes` is actively populated from recall lattice output
- AC1: Each row links a `product_candidate` to a commodity code with recall source, agreement metadata, rank, score, and stage-run identity.
- AC2: Reruns atomically replace the candidate's prior commodity-code set.
L5-FR-2.4.3 — `supplier_quotes` carries additional marketplace rows
- AC1: Alibaba, 1688, and Amazon quotes join existing eBay quotes.
- AC2: `priceSignalRole` values are Amazon/eBay `benchmark_ceiling`, Alibaba `optimistic_floor`, and 1688 `domestic_wholesale_floor`.

---

## 3. Testing, DoD, Risks & Open Questions

### 3.1 Definition of Done
L5-FR-3.1.1 — DoD 1: Alibaba/1688 search skills operational with Chinese keyword clusters
- AC1: Both skills emit valid `supplier_quotes` rows and pass their validator overlay bindings.
L5-FR-3.1.2 — DoD 2: Amazon search skill operational
- AC1: Amazon emits valid benchmark-ceiling rows through browser-first search.
L5-FR-3.1.3 — DoD 3: PCS Scotland ingestion skill operational
- AC1: PCS skill directory exists, canary passes, and source registry entry is enabled.
L5-FR-3.1.4 — DoD 4: HMRC FPO/search-suggestion endpoint investigation completed
- AC1: Step 03 TDD records the selected endpoint path and tests implemented recall behavior.
L5-FR-3.1.5 — DoD 5: Multi-source recall produces candidates from >=2 independent sources when available
- AC1: Candidate output records source agreement and source independence.
L5-FR-3.1.6 — DoD 6: 10-digit resolution via HMRC Trade Tariff data produces UK-specific commodity codes
- AC1: UK10 output includes HMRC measure data and does not rely on non-UK authority for UK national measures.
L5-FR-3.1.7 — DoD 7: Measure scanner detects AD/CVD/safeguard/quota measures and models interactions
- AC1: Measure interaction golden cases pass additive, substitutive, and conditional examples.
L5-FR-3.1.8 — DoD 8: HMRC Exchange Rate Service provides customs valuation FX
- AC1: Monthly customs FX snapshots populate landed-cost estimates.
L5-FR-3.1.9 — DoD 9: Frankfurter commercial FX provides spot rates
- AC1: Commercial spot FX populates `commercial_fx_rate` and `fx_basis_delta_bps`.
L5-FR-3.1.10 — DoD 10: Freightos + SeaRates CDP skills provide freight market data
- AC1: Both provider adapters normalize low/base/high freight estimates or record source-specific failure.
L5-FR-3.1.11 — DoD 11: Decision-grade landed cost per validated category with full audit trail
- AC1: Each validated category has traceable customs, FX, freight, VAT, confidence, warning, and blocker evidence.
L5-FR-3.1.12 — DoD 12: Gold-set expansion completed for fasteners, packaging-cartons, and examination-gloves
- AC1: Each category has `canary.inputs.jsonl`, `golden.assertions.yaml`, `metadata.json`, and `manifest.md`.
- AC2: Each category passes the §3.4 evidence-tiered accuracy metrics before production use.

### 3.2 Test Files (5 New, Cumulative 57)
L5-FR-3.2.1 — L5 adds five focused test files with cumulative target 57
- AC1: The test files below exist and map test IDs back to `L5-FR-*` requirements.
- AC2: Test coverage includes unit/integration behavior for customs, FX, freight, and marketplace skill contracts without browser E2E unless L5 touches desktop/web.

| File | Coverage |
|------|---------|
| `hmrc-fpo.test.ts` | FPO/search-suggestion parsing, candidate extraction, multi-query variants, recall agreement, endpoint investigation fixtures |
| `customs-classification.test.ts` | Multi-source recall, multi-family expansion, local cache, shortlisting, conservative defaults, ambiguity blockers, evidence tiers |
| `measure-interaction.test.ts` | AD/CVD/safeguard stacking/substitution, supplementary units, exporter basis, additional-code satisfiability |
| `fx-engine.test.ts` | Dual rates, monthly rollover, historical correction, basis delta, recalculation triggers, missing-rate blockers |
| `freight-engine.test.ts` | Mode locks, Freightos/SeaRates cross-check, dual packing, Incoterm boundary scenarios, provider failures |
L5-FR-3.2.2 — Local quality gate is the only automated repository gate because no `.github/workflows/` exists
- AC1: Verification uses `cargo fmt --check && cargo check --all-targets && cargo clippy --all-targets -- -D warnings && cargo test -p daemon --bin daemon && bun run build && bun run test`.
- AC2: Adding CI workflow coverage is a separate owner-authorized scope addition and is not silently included in L5.

### 3.3 Test Scenarios
L5-FR-3.3.1 — Scenario: multi-family expansion with 3 plausible HS6 families
- AC1: All UK10 descendants are evaluated across all 3 families; shortlist selects top 3-5 non-dominated branches per family.
L5-FR-3.3.2 — Scenario: trade remedy detection for steel product from China
- AC1: AD/CVD measures are detected, residual rate is applied, blocker/warning is emitted, and `unlockable_margin_bps` shows named-exporter delta.
L5-FR-3.3.3 — Scenario: compound duty with specific per-unit duty
- AC1: Duty expression is preserved as compound and not flattened to a single percentage.
L5-FR-3.3.4 — Scenario: supplementary unit required but missing
- AC1: `SUPPLEMENTARY_UNIT_MISSING` is emitted; duty abstains on that component and does not average.
L5-FR-3.3.5 — Scenario: named exporter versus residual for aluminium extrusions
- AC1: Residual rate applies by default; `evidence_leverage` shows named-exporter proof delta.
L5-FR-3.3.6 — Scenario: additional-code satisfiability unknown
- AC1: Rate without additional code applies and manual customs review blocker is emitted.
L5-FR-3.3.7 — Scenario: bundle that should be GIR 3 set
- AC1: Bundle splitter flags potential set-candidate and does not force decomposition.
L5-FR-3.3.8 — Scenario: FX monthly rollover and historical correction
- AC1: Affected landed-cost estimates are recalculated with auditable old/new snapshots.
L5-FR-3.3.9 — Scenario: Alibaba search with Chinese retry
- AC1: English search failure or accessory swamp triggers Chinese retry and produces or records evidence-backed results.
L5-FR-3.3.10 — Scenario: category whitelist enforcement for medical device outside validated slice
- AC1: Candidate is rejected before search unless examination-gloves calibration slice authorizes it.
L5-FR-3.3.11 — Scenario: tariff structure mid-cycle change
- AC1: Tariff-change event is emitted and affected classifications are enqueued.
L5-FR-3.3.12 — Scenario: conservative-to-actual uplift measurement
- AC1: Evidence-tier comparison measures conservative estimate versus gold-label actual.
L5-FR-3.3.13 — Scenario: 1688 Chinese-only wholesale floor with vocabulary mismatch
- AC1: Chinese short-cluster search emits domestic-wholesale-floor quotes when vocabulary matches and emits vocabulary-mismatch failure when no Chinese alias can be validated.
L5-FR-3.3.14 — Scenario: Amazon browser-first benchmark ceiling
- AC1: Browser-first Amazon search emits benchmark-ceiling GBP quotes, selects a concrete ASIN variant, and rejects ambiguous variant-family pseudo-prices.
L5-FR-3.3.15 — Scenario: PCS OCDS Web API ingestion
- AC1: PCS notice-list and notice-family retrieval creates procurement source rows and passes canary assertions with PCS marked IN-SCOPE.
L5-FR-3.3.16 — Scenario: Frankfurter commercial spot and FX basis delta
- AC1: Frankfurter spot rate populates `commercial_fx_rate`, computes `fx_basis_delta_bps`, and triggers the configured FX-delta margin recalculation when threshold is crossed.
L5-FR-3.3.17 — Scenario: Freightos and SeaRates disagreement and dual failure
- AC1: Provider disagreement records `FREIGHT_MODE_LOW_CONFIDENCE`; dual provider failure emits `FREIGHT_ESTIMATE_FAILED` and preserves the manual-estimate requirement.
L5-FR-3.3.18 — Scenario: gold-set category promotion for the three decided categories
- AC1: Fasteners, packaging-cartons, and examination-gloves each have canary, golden assertion, metadata, manifest, and §3.4 metric outputs.
L5-FR-3.3.19 — Scenario: CBAM exposure flag for a 2027-crossing tender window
- AC1: An in-scope aluminium or iron/steel family with delivery crossing 2027-01-01 sets `cbam_exposure_flag = 1` and keeps `cbam_estimate_gbp_pence = null`.

### 3.4 Evidence-Tiered Classification Accuracy
L5-FR-3.4.1 — Classification accuracy is scored by evidence tier
- AC1: Gold labels are ATR outcomes, actual import declarations, or broker-confirmed family with no later amendment.
- AC2: Silver labels require two independent internal reviewers with documented reasoning.
- AC3: Bronze labels are best-effort tender-doc labels and cannot certify auto-classification safety alone.
L5-FR-3.4.2 — Accuracy metrics include abstention and catastrophic-error measures
- AC1: Metrics include duty-outcome accuracy, trade-remedy yes/no, additional-code required yes/no, supplementary-unit required yes/no, review-trigger recall, catastrophic understatement rate, unsafe auto-classification rate, and abstention as a positive class.
- AC2: Gold-set categories are exactly fasteners, packaging-cartons, and examination-gloves per 2026-05-08 operator decision.

| Metric | Definition |
|--------|------------|
| `duty_outcome_accuracy` | Gold/silver duty outcome falls inside the predicted min/max duty range and the selected branch's duty expression matches the labeled measure family. |
| `trade_remedy_yes_no` | Active trade remedy presence/absence matches the label for the commodity-origin pair. |
| `additional_code_required_yes_no` | Requirement for national/additional codes matches the label and missing satisfiability is not reported as eligible. |
| `supplementary_unit_required_yes_no` | Requirement for kg/litre/pair/item/per-1000 or other supplementary unit matches the label. |
| `review_trigger_recall` | A labeled ambiguous, high-remedy, missing-unit, or missing-additional-code case emits the expected blocker/review path. |
| `catastrophic_understatement_rate` | Auto-produced duty maximum is below the labeled duty by >=1000 bps of customs value or >=100 GBP equivalent, whichever is lower. |
| `unsafe_auto_classification_rate` | Engine emits an auto-usable decision when the label requires broker/manual review. |
| `abstention_positive_class` | Correct abstention on hard negatives or insufficient-evidence cases counts as success; fabricated actionable estimates count as failure. |
L5-FR-3.4.3 — Evidence-tiered metrics gate category production
- AC1: Bronze-only labels cannot satisfy auto-classification safety gates for a category.
- AC2: Gold and silver metric reports must separate correct-abstention successes from wrong-code successes and failures.

### 3.5 Customs-Specific Error Types (Layer 5 Extension)

These customs error requirements are daemon error-vocabulary requirements, not skill-overlay-only codes. L5 updates `crates/daemon/src/errors/manifest.rs` (`ErrorCode` enum at line 20+ and `as_str()` matcher) plus any exhaustive match sites surfaced by `cargo clippy --all-targets -- -D warnings`, typically `validation/policy.rs`, `validation/executor.rs`, `errors/markdown.rs`, `tracing/structured.rs`, and `ipc/handlers/*`. The skill plug-and-play rule (L0 S4) governs skill manifests, not daemon enum vocabulary. `FPO_ACCESS_UNAVAILABLE` already exists as `FpoAccessUnavailable`; L5 reuses it and must not add a duplicate `FPO_UNAVAILABLE`.
L5-FR-3.5.1 — Error code `CLASSIFICATION_AMBIGUOUS`
- AC1: Trigger is top HS6 candidates diverging across recall sources; action blocks auto-qualification and escalates to broker.
L5-FR-3.5.2 — Error code `TARIFF_BRANCH_DIVERGENCE`
- AC1: Trigger is UK10 branches splitting on remedy/no-remedy or duty type; action is hard blocker, not scenario band.
L5-FR-3.5.3 — Error code `MEASURE_INTERACTION_UNKNOWN`
- AC1: Trigger is unclear AD/CVD/safeguard interaction rules; action is manual customs review.
L5-FR-3.5.4 — Error code `EXPORTER_UNRESOLVED`
- AC1: Trigger is active trade remedy with unidentified exporter/manufacturer; action assumes residual rate and alerts operator.
L5-FR-3.5.5 — Error code `SUPPLEMENTARY_UNIT_MISSING`
- AC1: Trigger is required supplementary unit data unavailable; action abstains on that duty component.
L5-FR-3.5.6 — Error code `VALUATION_MODE_CONFLICT`
- AC1: Trigger is material proxy/declaration divergence; action flags uncertainty range.
L5-FR-3.5.7 — Error code `FPO_ACCESS_UNAVAILABLE`
- AC1: Trigger is FPO/search-suggestion endpoint unreachable or access-restricted; action continues with remaining sources and confidence penalty.
- AC2: Implementation reuses existing `FpoAccessUnavailable` enum variant and does not add duplicate `FPO_UNAVAILABLE`.
L5-FR-3.5.8 — Error code `TRADE_REMEDY_WATCH_STALE`
- AC1: Trigger is TRA investigation or measure change affecting cached classification; action requires reclassification.
L5-FR-3.5.9 — Error code `FX_RATE_MISSING`
- AC1: Trigger is no valid HMRC customs FX snapshot; action blocks landed-cost computation.
L5-FR-3.5.10 — Error code `FREIGHT_ESTIMATE_FAILED`
- AC1: Trigger is neither Freightos nor SeaRates can quote; action requires manual freight estimate and records warning/blocker.

### 3.6 Open Questions
L5-FR-3.6.1 — Open questions remain explicit and research-backed
- AC1: The five questions below are the only L5 open questions.
- AC2: Step 05 strategy converts each into either a pre-implementation decision or a risk-managed unknown; they cannot become silent de-scopings.
L5-FR-3.6.2 — Open questions have present-tense runtime behavior
- AC1: Each open question below includes current L5 behavior that a live implementation follows while the research-backed question remains open.
- AC2: Runtime behavior cannot wait for an open question to close; it must choose a conservative default, blocker, or explicit warning path.

**§10 Further Research #2: Product-family-specific logistics assumptions**
- Shipping mode depends on item class and packaging density. Need default freight-mode heuristics per target product family.
- **L5 impact:** Directly affects freight engine mode selection defaults.
- **Current L5 behavior:** Until category-specific heuristics are calibrated, §2.2 boundary scenarios choose the highest-cost feasible mode, record missing logistics facts, and block automated landed-cost completion when provider quotes cannot be grounded in weight/volume evidence.

**§10 Further Research #3: Chinese supplier source strategy**
- Validate which supplier platforms can be lawfully and reliably integrated.
- **L5 impact:** Determines operational scope of Alibaba/1688 skills.
- **Scotland scope decision:** IN-SCOPE — confirmed by operator 2026-05-08 via pipemaxxer AskUserQuestion. PCS skill is implemented in Phase 5; full bead set assumes Scotland coverage. Research back-reference preserved as historical context.
- **Current L5 behavior:** Alibaba and 1688 remain in scope as floor evidence only; they require auth/source-health checks, validator overlay binding, and cannot create executable regulated-goods decisions without explicit compliance/export evidence.

**§10 Further Research #8: Customs cost spread materiality threshold**
- What `customsCostSpreadBps` triggers `customs_branch_divergent` blocker? 2.7%-4.7% may be tolerable; 2.7%-48.5% is clearly a blocker. Calibrate against whitelist categories.
- **L5 impact:** Directly affects blocker logic in customs engine.
- **Current L5 behavior:** §1.13 uses `customs_cost_spread_bps >= 1500` as the conservative placeholder threshold, while remedy/no-remedy and duty-type splits block regardless of spread size.

**§10 Further Research #9: Packaging materials sub-category reclassification**
- Split easy (cartons, poly mailers) vs dangerous (aluminium foil, glass, tin mill) subfamilies?
- **L5 impact:** Determines customs engine treatment of packaging category.
- **Current L5 behavior:** Packaging-cartons is the validated easy slice; aluminium foil, glass, tin mill, metal cans, and composite packaging are treated as dangerous subfamilies requiring broker/manual review until their own gold-set slice passes.

**§10 Further Research #10: CBAM rate integration timeline**
- UK CBAM trial rates expected Q4 2026, full rates January 2027. When to compute non-zero `cbamEstimateGbp`?
- **L5 impact:** Determines activation timeline for CBAM field.
- **Current L5 behavior:** §1.14 emits only exposure flags for confirmed 2027 sectors and keeps `cbam_estimate_gbp_pence` null until rate rules and calculation method are final.

### 3.7 Risks
L5-FR-3.7.1 — Risk: HMRC FPO/search-suggestion endpoint is unavailable or not public
- AC1: Mitigation is endpoint discovery bead choosing public Trade Tariff search, public FPO, or operator decision path; FPO is not a hard dependency.
L5-FR-3.7.2 — Risk: anti-bot detection on Alibaba/1688
- AC1: Mitigation is source-health canaries, auth-refresh skills, and circuit breaker.
L5-FR-3.7.3 — Risk: customs classification wrong for complex products
- AC1: Mitigation is multi-source recall, conservative defaults, abstention, and broker escalation.
L5-FR-3.7.4 — Risk: freight source variability or CDP page drift
- AC1: Mitigation is dual-source cross-check, per-source schema drift errors, and confidence scoring.
L5-FR-3.7.5 — Risk: tariff cache freshness gap after daemon downtime
- AC1: Mitigation is separate freshness vectors, missed-watcher stale marking, and catch-up reclassification before L6 bid readiness.
L5-FR-3.7.6 — Risk: cross-language marketplace matching corruption
- AC1: Mitigation is Chinese alias provenance, back-translation/head-noun checks, and validator overlay review before quote emission.
L5-FR-3.7.7 — Risk: worker and validator overlay version drift
- AC1: Mitigation is explicit worker/validator behavior version binding and stable-promotion blocking on incompatible versions.
L5-FR-3.7.8 — Risk: CBAM rate publication delay
- AC1: Mitigation is keeping rate-bearing estimates null while exposure flags remain active and visible for affected 2027-window tenders.

### 3.8 Forward-Compatibility
L5-FR-3.8.1 — L5 exposes the L6 bid-readiness gate
- AC1: L6 cannot reach `ready_to_bid` while `CLASSIFICATION_AMBIGUOUS`, `TARIFF_BRANCH_DIVERGENCE`, required-component `SUPPLEMENTARY_UNIT_MISSING`, `decision_grade = broker_required`, or expired `classification_fresh_until` is active.
- AC2: Quote-role quality gate blocks opportunities backed only by `domestic_wholesale_floor` or `optimistic_floor` unless the operator explicitly overrides.
- AC3: Gate producers are explicit: `L5-FR-3.5.1`, `L5-FR-3.5.2`, `L5-FR-3.5.5`, `L5-FR-2.4.1`/`L5-FR-1.13.2` for `decision_grade = broker_required`, and `L5-FR-1.11.3` for expired `classification_fresh_until`.
L5-FR-3.8.2 — L1 PDF extraction can later support customs evidence overlays
- AC1: Compliance certificate extraction is a semantic overlay extension and does not require schema changes or daemon recompilation.

### 3.9 Dependencies
L5-FR-3.9.1 — L5 depends on Layers 0-4 and skill-first external data access
- AC1: Layers 0-4 must be complete: pipeline data, validation infrastructure, sync engine, projection tables, and web portal.
- AC2: L5 external access is satisfied by existing skills, operator-built prerequisite skills, and public/free endpoints.

| Service / Skill | Status | Required? | Auth / Cost | L5 Position |
|-----------------|--------|-----------|-------------|-------------|
| HMRC Trade Tariff classification API via `skills/landed-cost/hmrc-customs-classification/` | Existing public endpoint | Yes | No key, free | Canonical UK commodity/measures data |
| HMRC Exchange Rates for Customs and VAT | Public gov.uk monthly publication | Yes | No key, free | Customs FX source |
| Frankfurter via `skills/landed-cost/simple-fx-lookup/` | Existing skill extension | Yes | No key, free | Commercial spot FX source |
| `skills/landed-cost/searates-freight-estimate/` | Existing CDP skill | Yes | Browser session, free | SeaRates freight source |
| `skills/landed-cost/freightos-freight-estimate/` | Operator-built prerequisite | Yes | CDP/browser, free | Freightos freight source; step 09 verifies |
| `skills/landed-cost/taric-cross-check/` | Operator-built prerequisite | Yes | Public EU TARIC CDP/browser, free | EU TARIC cross-check source; step 09 verifies |
| HS Ping | Declined for L5 | No | N/A | Four other recall sources are sufficient |
| Dutify/Zonos/Avalara | Declined for L5 accounts | No | N/A | Comparison adapters use fixtures only |
| Easyship/Shippo, CurrencyBeacon/OANDA | Declined for L5 | No | N/A | Not on critical path |
L5-FR-3.9.2 — Operator-built prerequisites gate bead execution
- AC1: Phase 3 Freight beads cannot start until `skills/landed-cost/freightos-freight-estimate/` exists with `SKILL.md`, `result.schema.json`, and `checkpoint.schema.json`.
- AC2: Phase 4 Customs recall beads cannot start until `skills/landed-cost/taric-cross-check/` exists with `SKILL.md`, `result.schema.json`, and `checkpoint.schema.json`.
- AC3: These two prerequisite skills are not pipeline beads; step 09 audit verifies them before authorizing execution.
- AC4: Step 09 records missing `canary.inputs.jsonl`, `golden.assertions.yaml`, or `validator-rubric.yaml` as stable-production blockers even when the initial bead-claim prerequisite files exist.
L5-FR-3.9.3 — Bead and Agent Mail operational guidance uses `br`, not deprecated `bd`
- AC1: L5 handoffs cite AGENTS.md authoritative `br` block at lines 262-305.
- AC2: The duplicated AGENTS.md `bd` block at lines 666-723 is treated as deprecated working-note context and not used for L5 commands.

### 3.10 ADRs Addressed
L5-FR-3.10.1 — L5 implements ADR-006 internal landed-cost engine at decision-grade depth
- AC1: ADR-006 coverage includes multi-source recall, UK10 resolution, measure interaction, dual FX, and real freight source cross-checking.
L5-FR-3.10.2 — L5 implements ADR-001 skill-based data retrieval for added sources
- AC1: Alibaba, 1688, Amazon, PCS, Freightos, SeaRates, TARIC, HMRC, and FX inputs use skills or public/free endpoints, not external commercial connector accounts.

| ADR | Decision | Layer 5 Action |
|-----|----------|---------------|
| **ADR-006** | Internal landed-cost engine | Full decision-grade scope: multi-source recall, 10-digit resolution, measure interaction, dual FX, real freight |
| **ADR-001** | Skills as primary data retrieval | Mature Alibaba/1688/Amazon; add PCS; use landed-cost skills and public endpoints |

## L5 Pipeline Decisions Recorded 2026-05-08

These decisions were recorded by the operator via pipemaxxer AskUserQuestion on 2026-05-08. They are latest authority for resolving apparent contradictions in this spec.

DECISION 1 — product_commodity_codes ownership: Option A — L5 SQLite migration
  Recorded: 2026-05-08 by operator (via pipemaxxer AskUserQuestion)
  Rationale: Recommended default. Cleaner local state; no sync-engine impact. Supabase projection (created in L4) becomes the push target.
  Implementation: migrations/0013_l5_product_commodity_codes.sql (or next-free migrations/NNNN_l5_product_commodity_codes.sql if 0013 is no longer available at implementation time)

DECISION 2 — PCS Scotland scope: IN-SCOPE
  Recorded: 2026-05-08 by operator (via pipemaxxer AskUserQuestion)
  Implementation: skills/procurement-ingestion/pcs/ created in Phase 5 with full skill directory standard (SKILL.md + result.schema.json + checkpoint.schema.json + canary.inputs.jsonl + golden.assertions.yaml + validator-rubric.yaml). Daemon registry entry in sources/registry.rs. Bootstrap covers PCS OCDS Web API setup. Adds ~3-5 beads to Phase 5.

DECISION 3 — L5 implementation epic structure: Option (b) — sister epic
  Recorded: 2026-05-08 by operator (via pipemaxxer AskUserQuestion)
  Rationale: Recommended default. Clean separation between layer-doc-authoring epic (testproject1-4gh, children 4gh.1-4 already complete) and implementation work.
  Implementation: Step 07 runs `br create --type=epic --priority=P0 --title="Epic: L5 Implementation — Expanded Commercial Intelligence" --label l5` and captures the new epic ID into this section. All Phase 1-6 implementation beads thread under that new epic.
  New epic ID: testproject1-hn05 (created 2026-05-19 by step 07)

DECISION 4 — Gold-set categories (DoD #12): fasteners + packaging-cartons + examination-gloves
  Recorded: 2026-05-08 by operator (via pipemaxxer AskUserQuestion)
  Rationale: Clear differentiation — remedy-active (fasteners) / packaging-easy (cartons) / regulated medical (examination gloves).
  Implementation: Phase 6 creates tests/fixtures/calibration/customs-classification/{fasteners,packaging-cartons,examination-gloves}/{canary.inputs.jsonl,golden.assertions.yaml,metadata.json,manifest.md} per L4 calibration corpus convention.

External API Account State:
- L5 has zero paid commercial API procurements.
- Public APIs (no key, no portal): HMRC Trade Tariff classification API at `https://www.trade-tariff.service.gov.uk/uk/api/commodities/{code}` via existing `skills/landed-cost/hmrc-customs-classification/`; HMRC Exchange Rates for Customs and VAT via gov.uk monthly publication; Frankfurter commercial spot FX via existing `skills/landed-cost/simple-fx-lookup/`.
- Existing CDP skills: `skills/landed-cost/searates-freight-estimate/` provides browser-resident SeaRates Logistics Explorer estimates with no vendor API account.
- Operator-built prerequisites before step 07: `skills/landed-cost/freightos-freight-estimate/` and `skills/landed-cost/taric-cross-check/`; step 09 audit verifies them before Phase 3/4 execution.
- Declined vendor accounts / not required for L5: Xe Currency Data API, Freightos paid API, SeaRates paid API, Taric Support paid vendor, HS Ping, Easyship/Shippo, CurrencyBeacon/OANDA, Dutify/Zonos/Avalara vendor accounts. Estimated annual savings versus the earlier paid-API plan: $3,000-10,000+.
- HMRC FPO endpoint name remains a TDD investigation item: `l5-customs-hmrc-fpo-endpoint-discovery`.

Marketplace Skill Maturation Baseline:
- Alibaba: Chinese retry on zero/accessory hits + Ready-to-Ship filter + source-health canary expansion + validator overlay binding + candidate-to-stable promotion.
- 1688: Chinese-only short clusters (2-4 terms) + domestic wholesale floor pricing + auth-refresh/1688 binding + validator overlay binding + candidate-to-stable promotion.
- Amazon: alias families + structured title vocabulary + browser-first (PA-API 5.0 sunset 2026-04-30) + validator overlay binding + candidate-to-stable promotion.
- No additional gap-fill work and no speculative hardening beyond §1.15 + §2.3.

Cross-Domain Callout Resolution Ledger:
- Callout 1 `product_commodity_codes` locality: RESOLVED — Option A per Decision 1. L5 creates the local SQLite junction; L4 Supabase projection becomes the push target.
- Callout 2 marketplace skill authorship framing: RESOLVED — §1.15 and §2.3 frame Alibaba/1688/Amazon as candidate-to-stable maturation, PCS as truly new.
- Callout 3 PCS Scotland scope: RESOLVED — IN-SCOPE per Decision 2; §1.15, §3.1, and §3.6 are unconditional.
- Callout 4 ErrorCode enum extension framing: RESOLVED — §3.5 names daemon enum update requirements and reuses existing `FPO_ACCESS_UNAVAILABLE`.
- Callout 5 HMRC FPO endpoint verification: DELIBERATE NON-RESOLUTION — Step 03 TDD creates `l5-customs-hmrc-fpo-endpoint-discovery`.
- Callout 6 existing landed-cost skills: RESOLVED — §3.9 follows the skill-first/free-public endpoint model.
- Callout 7 operator-built prerequisites: RESOLVED — §3.9 gates Phase 3/4 on Freightos and TARIC prerequisite skill existence.
- Callout 8 cron scheduler extension: RESOLVED — §1.11 requires four new `CronJobDefinition` entries.
- Callout 9 no CI workflows: RESOLVED — §3.2 records the local-only quality gate and no `.github/workflows/` while DoD remains 12 criteria.

---
