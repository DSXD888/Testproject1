---
manifest_version: 1
skill_id: pdf-extraction/tender-specification
skill_version: 1.0.0
job_id: job-707cc6bf-28e3-4d2a-8510-dd12260bd605
session_id: session-semantic-178821567e785d926d9ab316de72aa189df31861e27f7f26ce626bd03107aa08
execution_mode: contract
started_at: 2026-04-18T00:14:00Z
completed_at: 2026-04-18T00:20:00Z
outcome_class: success
items_processed: 5
items_skipped: 0
items_failed: 0
warnings_count: 5
agent_claimed_pages_visited: 0
agent_claimed_source_url: "document_extraction:345bd45a-72be-46db-aaa4-bb90738e230a"
agent_claimed_fingerprint_state: not_applicable
agent_claimed_locale_state: en-GB
agent_claimed_currency_state: GBP
---

# Run Manifest — pdf-extraction/tender-specification

## Summary

Semantic extraction over a Bristol City Council "Metal Works – Dynamic Purchasing System (DPS)" Invitation to Tender (ProContract DN583419, extraction id `345bd45a-72be-46db-aaa4-bb90738e230a`, policy version `2026.04.12`). Full-document scope; Docling-produced markdown of 42,460 characters, SHA-256 `0c7844b0644f3f463e4e66ce2193ecddba6643853c02729083f95af6fd15f9db`. Extracted five lots (Lot 1–Lot 5), shared evaluation criteria (Stage 1 Selection, Stage 2 Technical Assessment, tiered Call-Off pricing weights), and six product-requirement seeds against Lots 3–5. `result.json` validates against `result.schema.json`. `target_domain = "lots"`, `extraction_scope = "full_document"`.

## Steps Taken

1. Read the bound SKILL.md and job parameters; confirmed `document_extraction_id=345bd45a-72be-46db-aaa4-bb90738e230a`, `target_domain=lots`, `extraction_scope=full_document`.
2. Loaded the Docling markdown and section outline from `state.db` (`document_extractions` row). Captured character length (42,460) and computed SHA-256 for `source_markdown_hash`.
3. Loaded cross-reference context from `state.db`:
   - `procurement_processes.proc-pc-7e2a6fb2-67a3-f011-813a-005056b64545` — buyer `Bristol City Council`, title `BSH/HRA/Metal Works - Dynamic Purchasing System (DPS) - Acceptance Round 6`.
   - `contract_notices.pc-7e2a6fb2-67a3-f011-813a-005056b64545` — notice-level CPVs (`44221300-8 Gates`, `44400000-4 Miscellaneous fabricated products`, `45223100-7 Assembly of metal structures`). Not asserted at lot level in the document, so not propagated into `lots.cpv_codes`.
   - `notice_lots` rows for five pre-existing lot records used for cross-reference only per SKILL guidance (document is authoritative).
4. Verified `result.schema.json` structure (required fields, `$defs.evidenceAnchor`, `target_domain` const `"lots"`).
5. Identified the five lots from Section 1 "Lots" and Section 4 stray bullet. Normalized lot numbering (Lot 1 appears out-of-order in Section 4; captured in warning `LOT_1_TITLE_IN_SECTION_FOUR`).
6. Identified evaluation criteria from Section 3: Stage 1 Selection (pass/fail against Table A of Appendix 1), Stage 2 Technical Assessment, Call-Off weightings (100% Price ≤£25k; 60–80% Price / 0–20% Quality / 20% Social Value >£25k). Applied shared criteria to each lot.
7. Extracted product-requirement seeds only where the ITT body explicitly named items:
   - Lot 3: `lockers for parking meters`, `water taps`, `signs`, `specialist artwork`.
   - Lot 4: `balcony balustrades`.
   - Lot 5: `refuse chute repair` (lot-level only, no components named).
8. Left `technical_standards = []` per lot — the document references only legal/procurement acts (PCR 2015, Bribery Act 2010, Fraud Act 2006, GDPR, Construction Act, Living Wage, CLOCS, FORS, Equality Act 2010), not product-level BS/EN/ISO standards.
9. Wrote `result.json` and validated it against the schema with `jsonschema.validate` — result: VALID.
10. Authored this run manifest.

## Decisions Made

- **Notice-level CPVs not propagated into `lots.cpv_codes`.** The ITT prose does not assign CPV codes to specific lots. Echoing the notice-level CPVs onto every lot would be an unevidenced claim. Raised warning `NO_LOT_LEVEL_CPV_CODES` and left the per-lot arrays empty; downstream can still read notice CPVs via `process_context`.
- **`technical_standards = []` at lot level.** No product-level technical standards (BS/EN/ISO) appear in the document. Legal/procurement statutes and operational safety schemes (CLOCS, FORS) are not product standards and are recorded as narrative, not as `technical_standards` entries. Warning `NO_TECHNICAL_PRODUCT_STANDARDS_STATED` raised.
- **Lot descriptions kept short and explicit about deferral.** The ITT body states only lot titles (plus Lot 3 examples). Richer lot descriptions in `notice_lots.description` were NOT lifted in, per SKILL guidance "do not treat pre-existing lot rows as authoritative truth if the tender document contradicts them" and the broader rule to not claim content without anchor evidence. Warning `LOT_DESCRIPTIONS_DEFERRED_TO_ATTACHMENTS` raised.
- **Product-requirement seeds scoped to what the ITT names.** Lot 3 yields four seeds (lockers for parking meters, water taps, signs, specialist artwork) directly quoted from the ITT. Lot 4 yields one seed (balcony balustrades) from its lot title. Lot 5 yields one coarse seed (refuse chute repair) from its lot title. Lots 1 and 2 yield no seeds — the ITT does not name items for either lot. `notice_lots` contains richer descriptions for Lots 1, 2, 4 and 5, but those are not asserted in the ITT body and are therefore not lifted.
- **Evidence anchors use `markdown:<section-slug>` page refs.** The underlying Docling extraction records `page_count=0` and all section entries report `page_start=1/page_end=1`, so true page numbers are unavailable. Captured this limitation as warning `SOURCE_PAGINATION_NOT_PRESERVED` and anchored evidence to section slugs plus verbatim `anchor_text` that matches the markdown.
- **Lot 1 title is sourced from a stray Section 4 bullet.** The Section 1 Lots enumeration begins at 1.1.2. The Lot 1 title `1.1.1. Lot 1 - Repairs and/or modification to metal items` appears later in Section 4 as what looks like a reflow artefact. Treated as in-scope (flagged via `LOT_1_TITLE_IN_SECTION_FOUR`).
- **`extraction_confidence_bps = 7200` (72%).** Lot titles and evaluation criteria are high-confidence (explicit in doc); lot descriptions, product seeds for Lots 4–5, and per-lot CPVs are low-to-moderate because the ITT defers detail to attachments.

## Warnings

1. `LOT_DESCRIPTIONS_DEFERRED_TO_ATTACHMENTS` — ITT body provides lot titles only; detail deferred to Appendix 1 and ProContract attachments.
2. `NO_LOT_LEVEL_CPV_CODES` — Document does not bind CPV codes to specific lots.
3. `NO_TECHNICAL_PRODUCT_STANDARDS_STATED` — Document cites legal/procurement acts and operational safety schemes, no product BS/EN/ISO standards.
4. `SOURCE_PAGINATION_NOT_PRESERVED` — Docling extraction reports `page_count=0`; page refs use section slugs.
5. `LOT_1_TITLE_IN_SECTION_FOUR` — Lot 1 title appears only as a stray bullet in Section 4, not in the Section 1 Lots enumeration.

## Evidence

All claims anchor to text that appears verbatim in the extracted markdown (`document_extractions.extracted_markdown`, SHA-256 `0c7844b0644f3f463e4e66ce2193ecddba6643853c02729083f95af6fd15f9db`):

- **Lot enumeration (Section 1, heading "Lots"):** "1.1. The council intents to split the contract into the following lots:" followed by `1.1.2. Lot 2 - Supply only or Supply and Installation of metal items`, `1.1.3. Lot 3 - Fabrication of bespoke metal work items, such as lockers for parking meters, water taps, signs, specialist artwork`, `1.1.4. Lot 4 - Repair or Replace balcony balustrades`, `1.1.5. Lot 5 - Chute Repairs` — anchors every lot and every Lot-3 product seed.
- **Lot 1 title (Section 4 bullet):** `1.1.1. Lot 1 - Repairs and/or modification to metal items` — anchor for Lot 1.
- **Proposed Contract (Section 1):** `The contract length will be for 36 months, with the option to extend for 24 months. ... The estimated total value of this contract (inclusive of options) is £600,000.00.` — anchors contract-value context referenced in extraction metadata.
- **Procurement route (Section 1):** `The procurement process will be conducted in accordance with the requirements of the Public Contracts Regulations 2015. Specifically, the Council will be using the Open procedure. Find a Tender Service (FTS) Contract Notice reference number BRISTOLCC001-DN583419-83047745 dated 26/8/2022.` — anchors regulatory posture.
- **Evaluation stages (Section 3):** `Stage 1: Selection Criteria ... Bidders will be assessed against the Selection Criteria as set out in Table A of Appendix 1: Evaluation Criteria.` and `Stage 2: Award Criteria ... The council will evaluate on the basis of Technical Assessment.` — anchor every lot's Stage 1 and Stage 2 evaluation criterion.
- **Call-Off weightings (Section 3):** `Call Off between £0 and £25k – Will be 100% Price` and `Call Off above £25k – Will be 60% - 80% Price and Quality 0% - 20% and Social Value 20%` — anchor every lot's Call-Off weighting criterion.
- **Pricing (Section 3):** `All rates and prices must be quoted in pounds sterling (GBP).` and `VAT should not be included in the bid rates and prices.` — anchor each lot's GBP-ex-VAT pricing requirement and the manifest's `agent_claimed_currency_state: GBP`.
- **Timescales (Section 2):** `12:00hrs Noon 23/09/2022 Local Time | Closing date and time for receipt by the Council of completed bidder responses via the ProContract tendering system.` — anchors each lot's closing-deadline requirement.
- **Bid Validity (Section 2):** `The bid is an unconditional offer and should remain open for acceptance for a period of 180 days from the tender return closing date.` — anchors each lot's 180-day validity requirement.
- **Source artefact:** `document_extraction:345bd45a-72be-46db-aaa4-bb90738e230a` (raw document `raw-pc-7e2a6fb2-67a3-f011-813a-005056b64545-a3ad690146721f876523749303b75cb54b9adb1e500954fe9569c81ce014354d`, source content hash `8764275b1b0c7aa8f3e0cdacc9f8668c628d72affbca979b146df5f0d5897be0`, extraction engine+version per `document_extractions` row).
- **Result artefact:** `result.json` in session workdir, validated against `skills/pdf-extraction/tender-specification/result.schema.json` — `target_domain=lots`, `extraction_scope=full_document`, five `lots` entries, six `product_requirement_seeds`.
