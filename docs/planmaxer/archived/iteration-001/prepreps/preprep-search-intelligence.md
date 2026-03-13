# Pre-Prep: Search Intelligence & Product Matching Quality

## Step 1: Pre-Prep Research

Read the following documents in this exact order. Do not skip any.

1. **AGENTS.md** — project constraints, toolchain, rules
2. **The research document:** `docs/research/2026-03-08-public-procurement-intelligence-platform-research.md`
   - Read the ENTIRE document, but pay special attention to:
     - §4.3 (Supplier Price Intelligence — source routing, cross-language strategy, dual confidence)
     - §5.7 (Intelligence Pipeline Data Flow — Price-Signal Matching, Pipeline Cost Management)
     - §5.2 (Component Designs — Quoteability Scorer, Supplier Quote Engine, Product Extraction Engine)
     - §5.4 (Data Models — ProductCandidate, SearchProfile, SupplierQuote interfaces and tables)
     - §6.1 (Complexity Analysis — especially "Marketplace comparability scoring: Very High")
     - §6.2 (Risk Assessment — product match and HS code risks)
     - §4.2 (Procurement Intelligence Pipeline — product extraction, candidateType)
     - Appendix C (Margin logic — how match quality feeds into margin calculation)
3. **The handoff prompt that produced the feedback:** `docs/planmaxer/active/handoffs/handoff-search-intelligence.md`

After reading, you should be able to explain:
- The full identity chain from notice → lot → product candidate → search profile → supplier quote → landed cost
- The candidateType taxonomy and source routing rules
- The quoteability scoring gate and its thresholds
- The search profile structure (4 query variants: uk_precise, uk_broad, china_en, china_zh)
- The deterministic shortlisting and hard-conflict filter
- The dual confidence scoring formulas and their weights
- The cross-language matching strategy (structured slots, curated lexicon, never machine-translate prose)
- Pipeline cost constraints ($20-60/day target, max 8 auto-searched candidates per notice)

**File Ownership:** This pre-prep owns the following sections for editing:
- §4.3 (Supplier Price Intelligence) — FULL ownership
- §5.7 — Intelligence Pipeline subsections on Price-Signal Matching, shortlisting, and Pipeline Cost Management ONLY
- §5.2 — Quoteability Scorer, Supplier Quote Engine, Product Extraction Engine component designs ONLY
- §5.4 — ProductCandidate, SearchProfile, SupplierQuote data model interfaces ONLY (not SQL tables)
- §6.1 — Complexity ratings for marketplace comparability, product candidate extraction, quoteability scoring ONLY
- §6.2 — Risk entries related to product matching, HS codes, and search quality ONLY
- Appendix C — Margin logic if match confidence affects it

Do NOT edit: §4.2 (procurement pipeline upstream of product extraction), §4.7 (database), §5.4 SQL table definitions, §5.6 (ingestion flow), sync protocol, daemon internals, skill authoring patterns. If the reviewer suggests changes to those areas, note them as deferred.

---

## Step 2: Feedback Integration

<INSERT REVIEWER RESPONSE HERE>

For each change proposed by the reviewer, evaluate:

### Integration Rules

1. **For query generation strategy improvements:** Integrate into §4.3 (the source routing and cross-language strategy subsections) and §5.2 (Supplier Quote Engine component design). If the reviewer proposes query templates per candidateType, add them as concrete examples.

2. **For hard-conflict filter specifications:** Integrate into §5.7 (shortlisting subsection). The current plan lists conflict types but doesn't specify implementation. Add the reviewer's specifics: what triggers hard reject vs confidence penalty, edge case handling, absence-of-evidence rules.

3. **For cross-language matching improvements:** Integrate into §4.3. The current plan has the "translate structured slots, not prose" principle plus a curated lexicon concept. Expand with the reviewer's specifics on lexicon structure, CPV-to-marketplace vocabulary mapping, cultural framing differences, fallback strategies.

4. **For confidence calibration approach:** Integrate into §5.7 and §5.2. If the reviewer proposes category-specific weight profiles or operator feedback loops, add them. Update the matchConfidence and quoteUsabilityConfidence formulas if the reviewer proposes changes, but preserve the dual-score separation (never average them — this is a settled decision).

5. **For zero-result recovery strategy:** Integrate into §5.7. Add query broadening logic, retry limits, and "no marketplace presence" classification. This is currently a gap.

6. **For cache strategy improvements:** Integrate into §5.7 (Pipeline Cost Management subsection). If the reviewer proposes different TTLs per source or smarter query fingerprinting, add specifics.

7. **For changes to data model interfaces (ProductCandidate, SearchProfile, SupplierQuote):** Integrate into §5.4 TypeScript interfaces. If new fields are needed, add them with types and documentation. Do NOT modify SQL table definitions — add a `<!-- CROSS-DOMAIN: New field [X] needs corresponding column in [table]. Coordinate with data-architecture domain. -->` note.

8. **For changes touching upstream extraction (§4.2) or downstream landed cost (§5.7 Landed Cost subsection):** Do NOT integrate directly. Add cross-domain notes.

### Evaluation Framework

For each proposed change, assess:
- **Agree strongly:** Addresses a real gap in matching quality or cost efficiency. Apply directly.
- **Agree partially:** Good direction but may conflict with cost targets or pipeline complexity. Apply with modifications, noting cost implications.
- **Disagree:** Conflicts with settled decisions (structured slots over ML, deterministic shortlisting, no additional marketplaces, no custom taxonomy). Note disagreement with rationale.

### Preservation Rules
- Preserve the dual confidence scoring separation (matchConfidence + quoteUsabilityConfidence)
- Preserve the candidateType taxonomy (exact_sku, standard_commodity, regulated_goods, brand_family, bundle, custom_or_fabricated, installed_goods)
- Preserve source routing by candidateType
- Preserve the pipeline cost target ($20-60/day)
- Preserve the "translate structured slots, not prose" principle
- Preserve the deterministic-first-then-AI approach (hard-conflict filter → deterministic scoring → AI adjudication on borderline only)
- Use "as-if-always" language

---

## Step 3: Fresh-Eyes Optimization

After completing Step 2, re-read the sections you edited with fresh eyes. Check for:

1. **§4.3 ↔ §5.7 consistency:** Does the source routing described in §4.3 match the pipeline flow in §5.7? Do query pack variants referenced in both sections agree?

2. **§5.2 ↔ §5.4 consistency:** Do the component designs reference the correct data model interfaces? If you added fields to ProductCandidate or SearchProfile, are they used in the component designs?

3. **§5.7 ↔ §6.1 consistency:** If the reviewer changed the complexity assessment for marketplace matching, does §6.1 reflect this?

4. **§5.7 ↔ §6.2 consistency:** If new risk scenarios were identified (e.g., "query generation produces zero results for standard commodities"), add them to §6.2 with appropriate likelihood/impact ratings.

5. **Confidence formula consistency:** If matchConfidence or quoteUsabilityConfidence weights were adjusted, verify the weights still sum to 1.0. Verify all referenced fields exist in the data models.

6. **Pipeline cost arithmetic:** If the reviewer proposed changes to batching, retry limits, or query broadening, verify the cost implications stay within the $20-60/day target. Show the math if numbers changed.

7. **Terminology:** Ensure consistent use of "search profile" vs "query pack", "shortlist" vs "filter", "match" vs "quote" throughout all edited sections.

Fix any inconsistencies found. Make targeted edits, not wholesale rewrites.

---

## Step 4: Sync Pass

1. **Update §1 (Executive Summary):** If the matching intelligence gained significant new specifics (query generation templates, calibration approach, zero-result recovery), ensure the executive summary reflects the maturity of the matching strategy.

2. **Update §4.2 cross-references:** If the reviewer's changes affect how product candidates are structured, verify that §4.2 (upstream) and §4.3 (this domain) agree on the handoff format.

3. **Update Appendix C (Margin Logic):** If match confidence thresholds changed or new confidence-to-margin-impact rules were added, update the appendix.

4. **Update §11 (Next Steps):** If the reviewer identified new implementation prerequisites for the matching pipeline (e.g., "curated lexicon must exist before China-source search works"), add them to the relevant milestone.

5. **Appendix E (Changelog):** Add a concise entry describing what was integrated in this review cycle.
