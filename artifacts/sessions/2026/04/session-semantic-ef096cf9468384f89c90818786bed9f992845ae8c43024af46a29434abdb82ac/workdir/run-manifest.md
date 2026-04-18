---
manifest_version: 1.0.0
skill_id: pdf-extraction/tender-specification
skill_version: 1.0.0
job_id: job-172ba6b4-f369-4fa5-8cc6-3b438a317ee5
session_id: session-semantic-ef096cf9468384f89c90818786bed9f992845ae8c43024af46a29434abdb82ac
execution_mode: contract
started_at: 2026-04-18T00:55:00Z
completed_at: 2026-04-18T00:58:00Z
outcome_class: success_with_warnings
items_processed: 1
items_skipped: 0
items_failed: 0
warnings_count: 5
agent_claimed_pages_visited: none
agent_claimed_source_url: none
agent_claimed_fingerprint_state: not_applicable
agent_claimed_locale_state: en-GB
agent_claimed_currency_state: GBP
---

## Summary

Processed Docling-extracted tender document `document_extraction_id=4e09e77d-2b8e-4682-8901-35c15c727953` (`document_title = ITT-Glazed-Units-Sheet-Glass-DPS`, `document_role = technical_specification`, `source_content_hash = 0c39ce627306de09ea22221ad64033e13cb443826b6f19d37fc7bd85c62c4543`, ~42 KB / 41 sections / 2 tables) for Bristol City Council notice `pc-7046e8ec-2734-ed11-8119-005056b64545` ("BSH/HRA/Supply of Double Glazed Units and Sheet Glass – DPS"). The ITT enumerates **two named lots** in §1 SECTION ONE – Lots: Lot 1 "Supply of Double Glazed Units" and Lot 2 "Supply of Sheet Glass". Emitted both lots with shared procurement-level requirements (procedure, term, value, ProContract submission, validity, prohibitions, sustainability, GDPR, Living Wage, sub-contracting, monitoring) and shared evaluation criteria (Stage 1 Selection / Stage 2 Technical Assessment, plus the call-off price/quality/social-value mix for ≤£25k vs >£25k). Each lot carries one product-requirement seed pointing to the lot's DPS supply category; detailed product technical attributes (BS EN 1279, dimensions, lamination, lead times, etc.) are deferred to Appendix 1 – Evaluation Criteria and to per-Call-Off ITTs. CPV codes were attached from notice context (44221111-6 → Lot 1, 14820000-5 → Lot 2). Output written to `result.json` (`extraction_metadata.target_domain = "lots"`, `extraction_scope = "full_document"`, `source_markdown_hash = 0c39ce62...`, `policy_version = 2026.04.12`, `extraction_confidence_bps = 7800`).

## Steps Taken

1. Read the bound `SKILL.md` and `result.schema.json` to confirm required contract shape (one `lots` array, `product_requirement_seeds`, per-lot `evidence_anchor`, `extraction_metadata.target_domain = "lots"`, `extraction_scope` required).
2. Fetched job context from the local daemon state database (`state/state.db`):
   - `document_extractions` row for the supplied `document_extraction_id` (`page_count=0`, `section_count=41`, `table_count=2`, `document_role=technical_specification`, `extraction_policy_version=2026.04.12`).
   - Parent `contract_notices` row for buyer/title/CPV signals (`main_category = '14820000-5 - Glass | 44221111-6 - Double-glazed units'`).
   - Existing `notice_lots` rows for cross-reference only — confirms two lots already known to the system (Lot 1 DGUs, Lot 2 Sheet Glass) consistent with the ITT body.
3. Streamed the Docling markdown into a scratch file and scanned it in full (~418 markdown lines) covering Title block, Foreword (Climate / Sustainability / Equalities / Living Wage / Safeguarding / GDPR), §1 SECTION ONE (Introduction, Procurement Process, Proposed Contract, Lots), §2 SECTION TWO (General, Timescales, Bid Validity, Preparation, Submission, Clarification, FOI, Bribery, Fraud, GDPR, Living Wage, Engagement, Construction Act, Collaboration / Lead-partner / J&S consortia / Sub-contracting / Multiple Bids), §3 SECTION THREE (J&S consortium bids, Sub-contracting, Evaluation – Stage 1 / Stage 2, Call Off Process, Pricing, Price Adjustment, Abnormally Low Tenders, Quality, Social Value, Monitoring), §4 SECTION FOUR (Contract).
4. Identified the lot enumeration: §1 SECTION ONE – Lots, 1.1 ("The council intents to split the contract into the following lots:") with bullets `1.1.1. Lot 1 - Supply of Double Glazed Units` and `1.1.2. Lot 2 - Supply of Sheet Glass`. The two lot bullets render in the Docling output below §4 (a Docling list-numbering artefact) but the text content is the lot enumeration belonging to §1 – verified by reading the surrounding `### Lots` heading and by cross-checking against the parent `notice_lots` rows.
5. Built a shared procurement-level requirements list (19 items) and a shared evaluation-criteria list (7 items: Stage 1 Selection, Stage 2 Award headline, two Call-Off tiers, Pricing rules, Quality scoring, Social Value) and applied them to both lots so each lot is self-contained for downstream consumers.
6. Allocated CPV codes from notice context: `44221111-6` (Double-glazed units) → Lot 1, `14820000-5` (Glass) → Lot 2. Tagged with `CPV_FROM_NOTICE_CONTEXT`.
7. Emitted one `product_requirement_seed` per lot pointing to the lot's DPS supply category. Did **not** invent detailed technical attributes (no BS EN 1279, dimensions, etc.) because the ITT body does not state them. Surfaced the deferral via `DETAILED_PRODUCT_SPEC_DEFERRED`.
8. Emitted no `technical_standards` for either lot — none are cited in this ITT body. (BS EN 1279 is cited in the sibling Appendix 1 document, which was extracted under its own session.)
9. Wrote `result.json` to the workdir; validated it structurally and against `result.schema.json` using the local `jsonschema` package (validation passed).
10. Wrote this run manifest.

## Decisions Made

- **Two lots emitted (lot_number "1" and "2") with evidence per lot.** The ITT explicitly enumerates two named lots in §1 SECTION ONE – Lots; both lot bullets quote the lot title verbatim (`Lot 1 - Supply of Double Glazed Units`, `Lot 2 - Supply of Sheet Glass`). This contrasts with the prior session that processed `Appendix 1 – Evaluation Criteria` (which did not enumerate numbered lots and emitted a single synthetic lot). The notice's existing `notice_lots` rows agree with this two-lot split.
- **Shared requirements / evaluation-criteria duplicated per lot.** All procurement-level rules (procedure, term, value, ProContract submission, FOI, bribery/fraud, GDPR, Living Wage, sustainability, sub-contracting, monitoring, call-off price/quality/social-value mix) apply to both lots; the ITT does not differentiate by lot. Duplicating into both lots' arrays makes downstream lot-scoped consumers self-contained.
- **CPV codes sourced from notice context, not document body.** ITT body cites no CPV numbers. Parent notice's `main_category` lists `14820000-5 - Glass | 44221111-6 - Double-glazed units`. Allocated 44221111-6 to Lot 1 (DGUs match) and 14820000-5 to Lot 2 (Sheet Glass matches). Flagged with `CPV_FROM_NOTICE_CONTEXT`.
- **No technical standards emitted.** The ITT body does not cite any British / European / ISO standards. BS EN 1279 (which applies to DGUs) is present in the sibling Appendix 1 – Evaluation Criteria document and is surfaced by that document's extraction; per the skill's "do not claim a standard reference unless evidence section cites the relevant anchor" rule, this extraction must not pre-emit it.
- **Product-requirement seeds kept minimal (one per lot).** The ITT identifies the product family per lot but defers detailed attributes. Emitted one seed per lot keyed `lot1-double-glazed-units-dps` / `lot2-sheet-glass-dps`. `technical_attributes` are limited to the product-family identification with an explicit note that the ITT body does not enumerate detailed specs. Detailed seeds (e.g. `glass-dgu-bs-en-1279`, `glass-stippolytte-6-4mm`) are emitted by the sibling Appendix 1 extraction.
- **Evidence anchors use section refs, `page_ref = "unknown"`.** Docling `page_count = 0` for this extraction; the markdown contains no page delimiters. Every anchor uses `section_ref` (e.g. `§1 SECTION ONE – Lots, 1.1.1`) with `page_ref = "unknown"`. Surfaced via `PAGE_ANCHORS_UNAVAILABLE`.
- **`extraction_confidence_bps = 7800`.** Very high confidence on lot identification and procurement-level evaluation criteria (direct quotes). Reduced because (a) CPVs come from notice context, (b) detailed product attributes are deferred to Appendix 1 / per-call-off ITTs, (c) Stage 2 Award detail is in Appendix 1, (d) Docling list-numbering quirks make some clause refs ambiguous (e.g. lot bullets rendered out-of-order under §4).
- **`extraction_metadata.source_markdown_hash`** uses the daemon-supplied `document_extractions.source_content_hash` (`0c39ce627306de09ea22221ad64033e13cb443826b6f19d37fc7bd85c62c4543`) — the canonical hash already attached to the extraction record. Re-shasumming the SQLite-streamed markdown produced a different value due to a trailing newline added by `sqlite3` output and is not used.

## Warnings

1. **PAGE_ANCHORS_UNAVAILABLE** — Source extraction has `page_count = 0` with no inline page markers; every evidence anchor uses section/clause refs with `page_ref = "unknown"`. Evidence: `document_extractions.page_count = 0`, no page delimiters in `extracted_markdown`.
2. **DETAILED_PRODUCT_SPEC_DEFERRED** — ITT body identifies two lots by name only; detailed product technical attributes (BS EN 1279, dimensions, lamination, lead times) are deferred to per-Call-Off ITTs and to the sibling 'Appendix 1 – Evaluation Criteria' document. Evidence: `§1 SECTION ONE – Lots, 1.1.1 / 1.1.2`; `§4 SECTION FOUR: CONTRACT, 1.1`; notice context lists `Appendix 1 - Evaluation Criteria Sheet Glass.docx` and `Schedule 7 - Call-Off Contract - Glazing Supplies.docx` as siblings.
3. **CPV_FROM_NOTICE_CONTEXT** — CPV codes 14820000-5 and 44221111-6 attached from `contract_notices.main_category`, not from the ITT body. Allocation: 44221111-6 → Lot 1, 14820000-5 → Lot 2. Evidence: `contract_notices.main_category = '14820000-5 - Glass | 44221111-6 - Double-glazed units'`.
4. **STAGE2_AWARD_DETAILS_IN_APPENDIX** — Stage 2 Award Criteria are described in the ITT only at the headline level ('Technical Assessment' + Call-Off price/quality/social-value mix). Detailed weightings, scoring matrices, and pricing-document mechanics live in Appendix 1 – Evaluation Criteria. Evidence: `§3 SECTION THREE – Stage 2: Award Criteria, 1.1`; `§3 SECTION THREE – Call Off Process`.
5. **DOCLING_LIST_NUMBERING_QUIRK** — Docling output reuses '1.1.', '1.1.2' across many sub-sections (formatting artefact of the source DOCX). Lot 1 / Lot 2 bullets are rendered separately from the surrounding `### Lots` heading and appear out-of-order under §4 SECTION FOUR. Evidence anchors quote the exact text and pin the section ref to make retrieval unambiguous.

## Evidence

All major extraction claims are anchored to observable text in the Docling markdown. Full anchor list lives in `result.json` (`evidence_anchors` plus per-lot / per-criterion / per-seed anchors). Key anchors backing the major decisions:

- **Procurement scope** — Document title block: `Supply of Double Glazed Units and Sheet Glass – DPS`; `ProContract Reference ID: DN625136`.
- **Procedure & FTS notice** — §1 SECTION ONE – Procurement Process, 1.1: `The procurement process will be conducted in accordance with the requirements of the Public Contracts Regulations 2015. Specifically, the Council will be using the Open procedure. Find a Tender Service (FTS) Contract Notice reference number BRISTOLCC001-DN625136-76775102 dated 14/09/2022.`
- **Term & value** — §1 SECTION ONE – Proposed Contract: `The contract length will be for 48 months, with no option to extend.`; `The anticipated commencement date is 1 December 2022.`; `The estimated total value of this contract (inclusive of options) is £700,000.00.`
- **Lot enumeration** — §1 SECTION ONE – Lots: `The council intents to split the contract into the following lots:`; bullets `Lot 1 - Supply of Double Glazed Units`; `Lot 2 - Supply of Sheet Glass`.
- **Timescales** — §2 SECTION TWO – Timescales table: `Bid & clarification process opens | 14/09/2022`; `Closing date and time for receipt by the Council of completed bidder responses via the ProContract tendering system. | 12:00hrs Noon 14/102022 Local Time`; `Commencement date of contract. | 01/12/2022`.
- **Bid validity** — §2 SECTION TWO – Bid Validity, 1.1: `The bid is an unconditional offer and should remain open for acceptance for a period of 180 days from the tender return closing date.`
- **Stage 1 Selection** — §3 SECTION THREE – Stage 1: `Bidders will be assessed against the Selection Criteria as set out in Table A of Appendix 1: Evaluation Criteria. Should a bidder not satisfactorily confirm any of the questions in Table A they will not be considered for further evaluation.`; consortium rule §3 1.2: `if any member of a consortium fails any question within the Evaluation Criteria (Stage 1), the whole consortium will fail`.
- **Stage 2 Award** — §3 SECTION THREE – Stage 2: `The council will evaluate on the basis of Technical Assessment.`
- **Call-Off mix** — §3 SECTION THREE – Call Off Process: `Call Off between £0 and £25k – Will be 100% Price`; `Call Off above £25k – Will be 60% - 80% Price and Quality 0% - 20% and Social Value 20%`.
- **Pricing rules** — §3 SECTION THREE – Pricing: `All rates and prices must be quoted in pounds sterling (GBP).`; `VAT should not be included in the bid rates and prices.`; `lump sum price that is over 100% higher than the lowest submitted lump sum price this bidder will receive a score of zero for that section`.
- **Price stability / abnormally low** — §3 Price Adjustment 1.1; §3 Abnormally Low Tenders 1.1.
- **Quality / Social Value** — §3 Quality 1.1: `It is important that bidder's demonstrate relevant experience…`; §3 Social Value lead-in: `Social Value will only apply for project over £25,000`.
- **Monitoring & contract docs** — §3 Monitoring 1.1; §4 SECTION FOUR: `Contract documents are located in the attachments tab on ProContract`; `Bidder submissions will form part of the successful bidder's Contract Documents.`
- **Sustainability / Living Wage / GDPR / Bribery** — FOREWORD §1 Climate Change 1.1.1 (carbon neutral by 2030); §2 Living Wage 1.3; §2 Data Protection (GDPR) 1.1; §2 Bribery Act 1.1.
- **CPV linkage (notice context)** — `contract_notices.main_category = '14820000-5 - Glass | 44221111-6 - Double-glazed units'` (daemon-supplied notice context, not in ITT body).
- **Sibling-document references** — notice `documents_downloaded` lists `Appendix 1 - Evaluation Criteria Sheet Glass.docx`, `Glazing Supplies - DPS Agreement.docx`, `Schedule 7 - Call-Off Contract - Glazing Supplies.docx` — these are the sources for the deferred detailed product specifications.
