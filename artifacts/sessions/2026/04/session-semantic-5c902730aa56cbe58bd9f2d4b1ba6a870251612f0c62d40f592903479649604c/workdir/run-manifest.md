---
manifest_version: "1.0.0"
skill_id: "pdf-extraction/tender-specification"
skill_version: "1.0.0"
job_id: "job-3c112126-c75d-47d2-9ec4-4153123b8ea8"
session_id: "session-semantic-5c902730aa56cbe58bd9f2d4b1ba6a870251612f0c62d40f592903479649604c"
execution_mode: "contract"
started_at: "2026-04-18T00:00:00Z"
completed_at: "2026-04-18T00:00:00Z"
outcome_class: "success"
items_processed: 1
items_skipped: 0
items_failed: 0
warnings_count: 6
agent_claimed_pages_visited: 0
agent_claimed_source_url: "local://document_extractions/e6ae86e6-225c-4783-9ca4-2a828e521b6f"
agent_claimed_fingerprint_state: "not_applicable"
agent_claimed_locale_state: "en-GB"
agent_claimed_currency_state: "GBP"
---

# Run Manifest - pdf-extraction/tender-specification

## Summary

Parsed the Docling-produced markdown for document_extraction `e6ae86e6-225c-4783-9ca4-2a828e521b6f` and emitted a single authoritative lot payload for **LOT 5 - Provision of Building Condition Surveys** under the **London Borough of Bexley - Building Services Framework 6278**. Output captures 40 requirement statements, 14 technical standards/references, and 19 evidence anchors keyed against section headings. No evaluation criteria, no CPV codes, and no product-requirement seeds were emitted because the source is a services specification with no weightings, no CPV citations, and no tangible goods scope - each exclusion is warned in the result payload. Overall extraction confidence is 8500 bps; the dominant uncertainty is the absence of an Appendix A body and page-level anchors (docling `page_count = 0` because the source appears to have been DOCX-derived).

## Steps Taken

1. Located the input markdown via `state.db -> document_extractions.id = 'e6ae86e6-225c-4783-9ca4-2a828e521b6f'` and read the full `extracted_markdown` column (353 lines, 25,199 bytes). Source-content hash `4c2cf71b2ef7be9193b731294a2a8c28ce649008f2facad31416e0b624cbeab7` adopted as `source_markdown_hash`.
2. Cross-checked process/notice context from `state.db` (`contract_notices.title = '6278 - Building Services Framework - Lots 1-11'`, buyer `London Borough of Bexley`, 11 pre-existing `notice_lots` rows). Confirmed that this specification describes Lot 5 only (`Provision of Building Condition Surveys`) and must not overwrite sibling lot rows.
3. Walked the markdown section-by-section (`## 1 Overview` -> `## 30 Financial Planning`) and recorded one requirement per first-class obligation, preserving contract-relevant quantitative specifics (e.g. 3 working days' notice, 08:00-17:00 site hours, 5-year planning horizon, 60-year pitched-roofing life expectancy).
4. Separated prescriptive technical references (RICS, CIPFA, MRICS, BCIS, SPONS, AutoCAD, CIVICA, TF Cloud, metric millimetres, RPI/CPI, EPC/DEC, Asbestos Management Survey, condition grading A-D, priority grading P1-P4) into the `technical_standards` array, each anchored to its originating section.
5. Set `evaluation_criteria = []`, `cpv_codes = []`, `product_requirements = []`, and `product_requirement_seeds = []`, and emitted explicit warnings explaining the rationale for each empty collection.
6. Computed per-field confidences and a rolled-up `extraction_confidence_bps = 8500`, lower than a schema-table extraction would score because evaluation, CPV, and product-seed fields have no evidence.
7. Wrote `result.json` to the session workdir and validated it against `skills/pdf-extraction/tender-specification/result.schema.json` (jsonschema: VALID).
8. Produced this run manifest with the required frontmatter and narrative sections.

## Decisions Made

- **Single-lot emission (lot_number = "5")**: The title block `LOT 5 - Specification For Condition Surveys` is unambiguous; the document does not describe lots 1-4 or 6-11. Emitting one lot avoids polluting sibling `notice_lots` rows. Documented in warning `LOT_DOC_SINGLE_LOT_SCOPE`.
- **No CPV codes on the lot**: The specification body contains zero CPV/Common Procurement Vocabulary citations. Parent-notice CPV codes (50700000-2, 50750000-7, 71315000-9) are framework-level and not lot-5 specific; the topically-plausible `71315300-4` (Building surveying services) is not asserted by document evidence and was deliberately omitted. Documented in warning `CPV_CODES_NOT_CITED_IN_DOCUMENT`.
- **No evaluation criteria**: This document is a *specification*, not an Invitation to Tender. It contains no `Evaluation`, `Award Criteria`, or `Weightings` headings or tables. Evaluation is presumed to live in the separate ITT/instructions document for Framework 6278. Documented in warning `EVALUATION_CRITERIA_NOT_PRESENT`.
- **No product_requirement_seeds**: Lot 5 is a professional-services scope. Its "deliverables" are reports, condition-graded data, AutoCAD drawings, and financial plans, all charged via schedule of rates - not tangible products with SKU-grade attributes (quantity, material, dimension, compliance class). Creating product-candidate seeds here would generate junk signals for downstream product matching. Documented in warning `PRODUCT_SEEDS_NONE_SERVICES_SCOPE`.
- **Section-ref page anchors**: `document_extractions.page_count = 0` (DOCX source); the Docling `pages` object is empty (`{}`). Evidence anchors use `page_ref = "section:<n>"` keyed to the numbered headings in the extracted markdown rather than fabricating PDF page numbers. Documented in warning `NO_PAGE_ANCHORS_AVAILABLE`.
- **Appendix A treated as out-of-scope narrative**: The grading definitions for A-D and P1-P4 are inlined as markdown tables in the body (sections 17 and 18) and captured verbatim in the requirements/technical_standards arrays. Any additional Appendix A content referenced but not embedded is flagged via warning `APPENDIX_A_REFERENCED_NOT_INLINED`.

## Warnings

All six warnings are present verbatim in `result.json` under the `warnings` array:

- `LOT_DOC_SINGLE_LOT_SCOPE` - source is Lot-5 only; do not overwrite sibling `notice_lots`.
- `EVALUATION_CRITERIA_NOT_PRESENT` - specification document does not enumerate evaluation criteria.
- `CPV_CODES_NOT_CITED_IN_DOCUMENT` - no CPV codes appear in the source text.
- `PRODUCT_SEEDS_NONE_SERVICES_SCOPE` - services scope; no tangible product candidates emitted.
- `NO_PAGE_ANCHORS_AVAILABLE` - docling page_count = 0; section refs used instead of page numbers.
- `APPENDIX_A_REFERENCED_NOT_INLINED` - Appendix A referenced but not embedded in markdown.

## Evidence

Key evidence anchors for the Lot 5 claim and its main requirement clusters (page_ref uses `section:<n>` where `<n>` is the numbered heading in the extracted markdown):

- Title block - `section:0` `"London Borough of Bexley - LOT 5 - Specification For Condition Surveys"` (establishes lot_number = 5 and lot title).
- `section:1` (Overview) `"seeking to appoint a suitably qualified and accredited Chartered Surveying company to deliver a programme of Condition Surveys ..."` (services scope + consultant qualification requirement).
- `section:1` (Overview) `"220 street properties and 140 office/community type properties and associated assets"` (portfolio scale).
- `section:1` (Overview) `"anticipated that the assignment will commence on the 01 September 2026"` (commencement date).
- `section:11` (Purpose) `"The Condition Surveys will assess all main elements of the building including, but not limited to the following"` followed by the 11-element list (core scope of inspection).
- `section:13` (Resources & Conditions) `"Building Surveyors should be either MRICS qualified or similar professional accreditation"` (workforce-qualification standard).
- `section:13` (Resources & Conditions) `"the Councils' asset management database (CIVICA)"` (target system + proforma data flow).
- `section:17` (Condition Grading) `"grades A - D"` and inlined definition table (A=Good, B=Fair, C=Poor, D=Failed).
- `section:18` (Priority Grading) `"priority grades 1, 2, 3 and 4 ... five (5) year planning period"` with inlined P1-P4 definition table.
- `section:19` (Works Identified) `"Costings will be produced using a recognised schedule of rates as Building Cost Information Service (BCIS) or SPONS pricing matrix"` (cost-estimation standards).
- `section:21` (Pilot Survey) `"A site/property will be identified by the Council for a pilot survey"` (sequencing requirement).
- `section:22` (Measurement) `"Dimensions will be taken and recorded in metric millimetres"` (measurement precision).
- `section:23` (Data Collection) `"can be automatically uploaded into the Council's Asset Management system, TF Cloud"` (data pipeline target).
- `section:28` (Asbestos Awareness) `"all consultants ... must consult the Asbestos Management Survey, on site, before starting their planned inspection"` (statutory safety requirement).
- `section:29` (Drawings) `"provided with not to scale CAD (Computer Aided Design) or line drawings in digital format (AutoCAD or PDF)"` and subsequent clauses on new-drawing creation (deliverable format standard).
- `section:30` (Financial Planning) `"medium to long term financial plan ... holistic review of the backlog maintenance and forward maintenance plan"` plus inlined life-expectancy table and statutory-obligation list (EPC/DEC, asbestos, electrical, gas, water, fire).

Daemon appendix reference: `document_extractions.id = 'e6ae86e6-225c-4783-9ca4-2a828e521b6f'`, `source_content_hash = 4c2cf71b2ef7be9193b731294a2a8c28ce649008f2facad31416e0b624cbeab7`, `extraction_policy_version = 2026.04.12`, `page_count = 0`, `section_count = 58`, `table_count = 2`. These fields together identify the exact bytes this run processed.
