# Pre-Prep: Landed-Cost & Trade Intelligence Engine

## Domain Ownership

This pre-prep owns modifications to these sections of the research document:
- **Section 4.4** — Shipping and Logistics Estimation
- **Section 4.5** — Customs, Tariff, and Commodity Classification
- **Section 4.6** — FX Conversion
- **Section 5.2** — High-Level Component Design (Landed-Cost Engine component)
- **Section 5.7** — Intelligence Pipeline Data Flow (Landed Cost subsection only)
- **Appendix A** — Suggested Margin Logic
- **Appendix B** — Suggested Confidence Model
- **ADR-006** — Build an Internal Landed-Cost Engine

**Shared with other domains:** Section 5.4 (data model) contains LandedCostEstimate and related entities — note changes but coordinate with data-architecture domain. Section 5.7 supplier enrichment subsection is owned by procurement-pipeline domain.

---

## Step 1: Pre-Prep Research

Before integrating the reviewer's feedback, read and deeply understand:

1. **The full research document** — `docs/research/2026-03-08-public-procurement-intelligence-platform-research.md`
   - Focus heavily on sections 4.4, 4.5, 4.6, 5.7 (Landed Cost subsection), Appendices A and B
   - Understand the HS classification pipeline: FPO → Taric Support → HS Ping → HMRC Trade Tariff
   - Understand the FX tiering: HMRC (customs) vs Xe (commercial) vs CurrencyBeacon (backup) vs OANDA (historical)
   - Understand the freight mode selection: parcel → air → LCL → FCL with source routing
   - Understand the margin logic: 30% gross gate, 20% net gate, 0.60 confidence floor
   - Understand the distinction between economic landed cost and cash outlay

2. **The handoff prompt** — `docs/planmaxer/active/handoffs/handoff-landed-cost-engine.md`
   - Understand the 5 tensions: FPO accuracy, freight mode selection, marketplace vs procurement pricing, dual FX semantics, confidence calibration
   - Note anti-patterns and user beliefs

3. **Pay attention to:**
   - How the customs valuation pipeline is specified (what's concrete vs. hand-waved)
   - The `LandedCostEstimate` entity schema in section 5.4
   - Missing cost components (insurance? port handling? broker fees? warehousing?)
   - The relationship between confidence scores and automated progression

---

## Step 2: Feedback Integration

<INSERT REVIEWER RESPONSE HERE>

### Integration Rules

For each proposed change from the reviewer:

1. **Agree strongly** — Integrate directly. Edit the relevant section.

2. **Partially agree** — Integrate core insight, adapt to document conventions.

3. **Disagree** — Do NOT integrate. Note why briefly.

### Integration Checklist

For each change you make:
- [ ] Preserve ADR-006 numbering
- [ ] If adding new cost components to the landed-cost calculation, update BOTH the flow in 5.7 AND the `LandedCostEstimate` entity description (note for 5.4 data model coordination)
- [ ] If modifying the HS classification pipeline, update ALL references (4.5, 5.7, and the Customs Recommendation block)
- [ ] If adding new FX sources or changing the tiering, update 4.6 AND the FX Recommendation
- [ ] If modifying margin thresholds, update Appendix A AND section 5.3 (Canonical Workflow) AND section 5.7
- [ ] If adding new confidence components, update Appendix B AND ensure the weights sum to 1.0
- [ ] Maintain the "API-based, not agent-scraped" distinction for ALL cost-estimation integrations
- [ ] Update Appendix D (Source References) if new APIs are added
- [ ] Update the Change Log in Appendix E

### What You Can Edit Directly
- Sections 4.4, 4.5, 4.6 — full ownership
- Section 5.2 Landed-Cost Engine component — full ownership
- Section 5.7 Landed Cost subsection — full ownership
- Appendices A, B — full ownership
- ADR-006 — can update
- Section 8.2 (Files to Create) — intelligence package section only
- Appendix D — can add new source references

### What Requires Coordination
- Section 5.4 (Data Model) — note field changes for `landed_cost_estimates`, `supplier_quotes` tables
- Section 5.3 (Canonical Workflow) — if margin gates change, note for holistic pass
- Section 6.1, 6.2 — can add complexity/risk items

---

## Step 3: Fresh-Eyes Optimization

After integrating:

1. **Pipeline completeness check:** Does the landed-cost calculation in 5.7 account for ALL cost components mentioned in the `LandedCostEstimate` entity?

2. **Source consistency check:** Is every API mentioned in 4.4/4.5/4.6 also referenced in section 9.3 (External Accounts)?

3. **Margin logic check:** Does Appendix A's margin formula match the `LandedCostEstimate` fields? Are all components accounted for?

4. **Confidence model check:** Do the weights in Appendix B still sum to 1.0? Are all confidence components defined?

5. **Cross-domain check:** If freight mode selection depends on product extraction (from procurement-pipeline domain), is the interface clear?

---

## Step 4: Sync Pass

1. Update Change Log in Appendix E.
2. If new API sources were added, ensure they appear in section 9.3 (External Accounts) and Appendix D (Source References).
3. If new files were added to section 8.2, verify naming patterns match existing intelligence package structure.
4. If the licensing table in 4.11 needs updates for new APIs, add them.
