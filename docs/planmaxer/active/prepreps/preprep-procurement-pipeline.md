# Pre-Prep: Procurement Pipeline & Product Matching

## Domain Ownership

This pre-prep owns modifications to these sections of the research document:
- **Section 4.1** — Procurement Notice Ingestion Sources
- **Section 4.2** — Procurement Rules and Notice Semantics
- **Section 4.3** — Supplier Price Intelligence Sources
- **Section 5.2** — High-Level Component Design (Preliminary Qualification Engine, Product Extraction Engine, Supplier Quote Engine components)
- **Section 5.3** — Canonical Workflow
- **Section 5.7** — Intelligence Pipeline Data Flow (Preliminary Qualification and Supplier Price Enrichment subsections)
- **Section 6.1** — Complexity Analysis
- **Section 6.2** — Risk Assessment
- **ADR-001** — Skill-Based Agent Scraping as Primary Data Retrieval

**Shared with other domains:** Section 5.4 data model (ContractNotice, QualificationResult, ProductCandidate, SupplierQuote entities) — note field changes for data-architecture domain. Section 5.7 landed-cost subsection is owned by landed-cost domain.

---

## Step 1: Pre-Prep Research

Before integrating the reviewer's feedback, read and deeply understand:

1. **The full research document** — `docs/research/2026-03-08-public-procurement-intelligence-platform-research.md`
   - Focus heavily on sections 4.1, 4.2, 4.3, 5.3, 5.7
   - Understand the notice normalization pipeline
   - Understand the dedupe/merge logic using OCID and heuristics
   - Understand the goods/services qualification rules (expiry, category, CPV)
   - Understand the product extraction interface (ProductCandidate entity)
   - Understand the supplier matching flow (search profile → skill scraping → comparability scoring)
   - Understand the opportunity state machine (discovered → ... → bid_submitted)

2. **The handoff prompt** — `docs/planmaxer/active/handoffs/handoff-procurement-pipeline.md`
   - Understand the 5 tensions: product matching accuracy, notice merging, goods/services edge cases, cross-language matching, pipeline throughput
   - Note anti-patterns and user beliefs

3. **Pay attention to:**
   - How product matching is currently (under)specified
   - The relationship between CPV codes and HS codes
   - How "generate search profile" works (or doesn't)
   - The scalability of agent-per-match-per-source approach
   - The notice lifecycle normalization (section 4.2)

---

## Step 2: Feedback Integration

<INSERT REVIEWER RESPONSE HERE>

### Integration Rules

For each proposed change from the reviewer:

1. **Agree strongly** — Integrate directly.
2. **Partially agree** — Integrate core insight, adapt to conventions.
3. **Disagree** — Do NOT integrate. Note why.

### Integration Checklist

For each change you make:
- [ ] Preserve ADR-001 numbering
- [ ] If modifying the canonical workflow (5.3), update ALL downstream references (5.7, 5.5 state model)
- [ ] If adding new states to the opportunity state machine (5.5), ensure they fit the existing transition logic
- [ ] If modifying the notice normalization pipeline, update section 5.6 (Ingestion Data Flow)
- [ ] If adding new product matching approaches, specify them concretely (not just "use embeddings")
- [ ] If modifying the qualification rules, update BOTH 5.3 and 5.7 (Preliminary Qualification subsection)
- [ ] If adding new entity fields (ProductCandidate, SupplierQuote), note them for the data-architecture domain
- [ ] If adding new skills, add them to the skill tables in section 4.10
- [ ] Maintain the "marketplace prices are indicative signals" framing
- [ ] Update the Change Log in Appendix E

### What You Can Edit Directly
- Sections 4.1, 4.2, 4.3 — full ownership
- Section 5.2 (Preliminary Qualification Engine, Product Extraction Engine, Supplier Quote Engine) — full ownership
- Section 5.3 — full ownership
- Section 5.5 — full ownership
- Section 5.7 (Preliminary Qualification and Supplier Price Enrichment subsections) — full ownership
- Section 6.1, 6.2 — full ownership
- ADR-001 — can update
- Section 8.2 (Files to Create) — Skills section (procurement and supplier skills)
- Appendix C (Suggested Job Kinds) — can add/modify

### What Requires Coordination
- Section 5.4 (Data Model) — note field changes for data-architecture domain
- Section 4.10 (Skills) — shared with agent-orchestration domain; can add skills but don't change skill design principles

---

## Step 3: Fresh-Eyes Optimization

After integrating:

1. **Workflow completeness check:** Does every state in 5.5 have a clear entry condition, exit condition, and transition? Are there dead-end states?

2. **Matching specification check:** Is the product matching approach now concretely specified? Can an implementer build it from the description?

3. **Merge logic check:** Is the notice-to-process merge algorithm specified concretely enough to implement?

4. **Classification check:** Are goods/services/mixed/works classification rules complete? Do the edge cases have resolution paths?

5. **Cross-domain check:** Do your field additions to ProductCandidate or SupplierQuote need to be reflected in the data model (section 5.4)?

---

## Step 4: Sync Pass

1. Update Change Log in Appendix E.
2. If new skills were added, ensure they appear in section 4.10 skill tables AND section 8.2 file listings.
3. If new job kinds were added, ensure they appear in Appendix C.
4. Verify the Canonical Workflow in 5.3 still reads coherently as a narrative.
