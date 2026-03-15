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

<INSERT REVIEWER RESPONSE HERE>

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
