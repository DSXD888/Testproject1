---
manifest_version: 1
skill_id: pdf-extraction/tender-specification
skill_version: 1.0.0
job_id: job-5ae2a86c-e84e-480c-90a7-db34dc50f776
session_id: session-semantic-7a27aca483bb1de4e1224ffa7016833ade88bf52a09fd8de92d585216a3bafa5
execution_mode: contract
started_at: 2026-04-18T09:52:00Z
completed_at: 2026-04-18T09:56:23Z
outcome_class: success
items_processed: 1
items_skipped: 0
items_failed: 0
warnings_count: 5
agent_claimed_pages_visited: 0
agent_claimed_source_url: n/a (contract mode — input is Docling markdown from document_extractions.id=bb0383e0-c57b-4f67-a336-fa00d3ed114c)
agent_claimed_fingerprint_state: not_applicable
agent_claimed_locale_state: en-GB
agent_claimed_currency_state: GBP
---

# Summary

Extracted lot structure, evaluation criteria, product-requirement seeds, and CPV context from the ESPO Framework 107_26 Tender Submission Document (document_extractions.id=bb0383e0-c57b-4f67-a336-fa00d3ed114c; 30,625 characters of Docling markdown; page_count=0 in extraction record).

Output contract written to `result.json` in the session workdir and validated against `skills/pdf-extraction/tender-specification/result.schema.json`. Target domain = `lots`, extraction scope = `full_document`.

The document is the tenderer's response form for a single-lot multi-supplier Framework covering inspection and maintenance of (a) indoor gymnasium equipment, (b) indoor fitness and sport equipment, and (c) outdoor schools play and sport equipment. One lot row was emitted (lot_number = "1") with three product-requirement seeds — one per equipment family. No explicit technical / safety standards appear in this submission form; the technical_standards array is correctly empty and flagged.

Overall extraction confidence: 7800 bps. Well-evidenced areas: evaluation-criteria weightings (Q1..Q6), geographical coverage, framework period, closing date, tendering declaration. Lower-confidence / out-of-document areas: technical standards (not present in this document), price weighting split (referenced out to Section 5 of the ITT), detailed specification (referenced out to Section 2 of the ITT).

# Steps Taken

1. Located session workdir under `artifacts/sessions/2026/04/session-semantic-…/workdir` and confirmed execution parameters (job, skill, scope).
2. Loaded the Docling-produced markdown for `document_extraction_id=bb0383e0-c57b-4f67-a336-fa00d3ed114c` from `state/state.db` → `document_extractions.extracted_markdown` (30,625 chars, 229 lines) and recomputed source hash for `extraction_metadata.source_markdown_hash` (SHA-256 `dfa99ecb39bd1aa2b1d39eb9be760465ea946758f8315840c2a448dbea5931f1`).
3. Read the full markdown and identified six top-level structural headings: Cover/Title, Section 1 Response to Requirement and Specification (compliance + geographical coverage), Section 2 Non-Price Method Statements (Q1..Q6 with weightings), Section 3 Price (references Appendix B), Section 4 Additional Benefits for Customers, Section 5 Confidential Information Schedule, Section 6 Tendering Declaration.
4. Pulled corroborating process context from the local state DB: `procurement_processes.title`, `contract_notices.main_category` (CPV strings "37400000-2 - Sports goods and equipment | 50000000-5 - Repair and maintenance services"), and the existing `notice_lots` row (single lot, lot_number=1) to cross-reference lot granularity.
5. Loaded `result.schema.json` and `golden.assertions.yaml` from `skills/pdf-extraction/tender-specification/` to lock output shape (required top-level keys; per-lot required arrays; evidence-anchor shape; target_domain const = "lots").
6. Composed one lot entity (lot_number "1", title and scope from the cover), requirements list (12 items, all anchorable to sections 1, 2, 3, 6), seven evaluation criteria (Q1..Q6 + Price), two CPV codes (imported from notice context, not printed in this document — flagged as a warning), three product-requirement seeds (indoor gymnasium equipment / indoor fitness & sport equipment / outdoor schools play & sport equipment), and empty technical_standards (none present; flagged).
7. Mirrored the three product-requirement seeds into the top-level `product_requirement_seeds` array per the schema contract.
8. Assembled `evidence_anchors` (14 anchors) using section headings as `page_ref` because the Docling extraction record reports `page_count = 0` (no reliable per-page coordinates); flagged this as a warning.
9. Wrote `result.json` to the session workdir and JSON-Schema-validated it against `result.schema.json` — validation passed with no errors.
10. Authored this run manifest for daemon finalisation.

# Decisions Made

- **Single lot, lot_number="1".** The document consistently treats the Framework scope as one unit — no sub-lots, no regional lots, no ring-fenced categories. Existing `notice_lots` row for the notice (`pc-3ba4e6b8-9cef-f011-813c-005056b64545-lot-1`) corroborates. Inventing extra lots would contradict on-document evidence.
- **Three product-requirement seeds rather than one.** The Framework title explicitly enumerates three distinct equipment families ("Indoor Gymnasium", "Fitness & Sport Equipment", "Outdoor Schools Play & Sport Equipment"), each of which drives a different candidate/source landscape downstream. Splitting into three seeds preserves that structure for the product-matching pipeline while keeping them under the same lot_number.
- **CPV codes sourced from notice context, not from the document body.** This document does not print CPV codes. The two CPVs (37400000 Sports goods and equipment; 50000000 Repair and maintenance services) come from `contract_notices.main_category` for the linked notice. This is called out explicitly in a warning (`CPV_SOURCED_FROM_NOTICE_CONTEXT`) so the daemon can attribute the signal correctly.
- **Empty `technical_standards` with warning.** No safety / technical standards (e.g. EN 1176, EN 957, BS 5696, BS EN 12572) appear anywhere in this submission form; they would normally sit in the Invitation to Tender main Specification, which is a different document. Guessing standards without an anchor would violate skill instructions ("Do not claim … a standard reference unless the evidence section cites the relevant anchor"). A warning records the out-of-document scope.
- **Price as an evaluation criterion with `weighting_text` stating the weighting is not in this document.** Section 3 Price defers evaluation methodology to Section 5 of the Invitation to Tender, which is not in scope here. Emitting Price with an honest "weighting not stated in this document" is more faithful than either omitting it (loses a real evaluation axis) or fabricating a split.
- **Per-section `page_ref` rather than page numbers.** `document_extractions.page_count = 0` for this extraction, so Docling did not materialise per-page coordinates. All evidence anchors cite section headings rather than page numbers, and a warning (`DOCLING_PAGE_COUNT_MISSING`) records this.
- **Field confidences**: weightings and geographical coverage → high (≥9400 bps), because they are directly quoted in the markdown; product requirements → moderate (6500 bps) because the three seeds are inferred from the equipment-family enumeration in the title rather than from dedicated requirement sentences; technical standards → low (2500 bps) because the evidence base is an absence rather than a presence.
- **`document_role` reported as `technical_specification`** in extraction_metadata, matching the persisted `document_extractions.document_role` value for this row (the document is classified as a technical spec upstream, even though the form itself is a submission response template). The scope warning `TECHNICAL_STANDARDS_NOT_IN_SCOPE_OF_DOCUMENT` captures the nuance.

# Warnings

1. `TECHNICAL_STANDARDS_NOT_IN_SCOPE_OF_DOCUMENT` — No technical / safety standards are printed in this submission form; they belong to the Invitation to Tender main Specification (a different document). `technical_standards` is emitted as an empty array.
2. `CPV_SOURCED_FROM_NOTICE_CONTEXT` — CPV codes 37400000 and 50000000 are imported from the contract-notice categories, not from the document body.
3. `PRICE_WEIGHTING_NOT_IN_DOCUMENT` — Detailed price / quality evaluation split is referenced out to Section 5 of the Invitation to Tender (Evaluation of Tenders) and the Pricing Schedule (Appendix B). Weighting for the Price evaluation criterion is therefore recorded as "not stated in this document".
4. `SINGLE_LOT_FRAMEWORK_NO_LOT_BREAKDOWN` — Framework is structured as a single multi-supplier lot (ESPO 107_26). Three product-requirement seeds are emitted under that one lot, keyed by equipment family.
5. `DOCLING_PAGE_COUNT_MISSING` — `document_extractions.page_count = 0`; evidence anchors reference section headings rather than page numbers.

# Evidence

Structural anchors used in `result.json` (also listed in `evidence_anchors` top-level array):

- **Cover / Title** — markdown lines 1–2: "ESPO reference 107_26 Inspection & Maintenance of Indoor Gymnasium, Fitness & Sport Equipment and Outdoor Schools Play & Sport Equipment". Drives lot_number="1" title and the three product-requirement seeds.
- **Cover / Framework Period** — lines 4–7: "Framework Period 24 June 2026 – 31 January 2028 with the option to extend up to 23 June 2030". Drives lot description (term).
- **Cover / Closing date** — lines 20–21: "Closing date for submission of tender 12 noon, 15 May 2026". Drives lot description (closing date).
- **Section 1 compliance table** — around lines 44–48: "I confirm I comply with all essential elements of the requirement and specification as outlined in Section 2 of the Invitation to Tender, Requirement (including Specification)". Drives requirement "compliance with specification" and the warning that standards live in the ITT main spec.
- **Section 1 Geographical Coverage table** — around lines 50–64: "The mandatory requirement is for Suppliers to provide Goods and/or Services to the nine defined regions of England" plus three optional-coverage confirmations for Scotland, Wales, Northern Ireland. Drives mandatory and optional geographical-coverage requirements.
- **Section 2 Method Statements intro** — lines 66–78: "There are 6 questions in total (5 quality based and 1 Social Value)" with word-limit guidance (700 / 500). Establishes evaluation-criteria count and scope.
- **Section 2 method-statement table** — Q1..Q6 blocks (lines 80–93):
  - Q1 "Customer Engagement and Framework Management (Weighting 3)"
  - Q2 "Customer Site Visit Process (Weighting 3)"
  - Q3 "Inspection Report (Weighting 5)"
  - Q4 "Services following completion of work (Weighting 4)"
  - Q5 "Consistency of Standards (Weighting 4)"
  - Q6 "Your Organisation's Current Approach to Social Value (Weighting 1)"
- **Section 2 Social Value % table** — lines 95–107: Customer Contract Value bands (£0.01 – £50,000 through £400,001+). Confirms the social-value % disclosure requirement.
- **Section 3 Price** — lines 109–140: "See the attached: 107_26 Pricing Schedule (Appendix B)" and Framework-Fee obligation. Drives Price evaluation criterion and the `PRICE_WEIGHTING_NOT_IN_DOCUMENT` warning.
- **Section 5 Confidential Information Schedule** — lines 155–186. Drives the confidentiality-declaration requirement.
- **Section 6 Tendering Declaration** — lines 187–229 — "undertake that this offer shall remain valid and open for acceptance for a period of 120 days from the date of submission"; Central Digital Platform registration; signatory fields. Drives the tender-validity, Central Digital Platform, and signatory requirements.

Process-context corroboration (not from the document body — flagged as warnings where used):

- `procurement_processes.title` for `proc-pc-3ba4e6b8-9cef-f011-813c-005056b64545`: "107_26 Inspection and Maintenance of Indoor Gymnasium, Fitness and Sport Equipment, and Outdoor Schools Play and Sport Equipment" (matches the document cover).
- `contract_notices.main_category` for `pc-3ba4e6b8-9cef-f011-813c-005056b64545`: "37400000-2 - Sports goods and equipment | 50000000-5 - Repair and maintenance services" (source for CPV codes).
- Existing `notice_lots` row `pc-3ba4e6b8-9cef-f011-813c-005056b64545-lot-1` (single lot, lot_number=1) corroborates the "one lot" emission.

Artifact refs:

- `result.json` — `<session_workdir>/result.json` — validated against `skills/pdf-extraction/tender-specification/result.schema.json` (python jsonschema, no errors).
- `extraction_metadata.source_markdown_hash` = `dfa99ecb39bd1aa2b1d39eb9be760465ea946758f8315840c2a448dbea5931f1` (SHA-256 of the 30,625-char markdown body read from `document_extractions.extracted_markdown` for id `bb0383e0-c57b-4f67-a336-fa00d3ed114c`).
- `extraction_metadata.policy_version` = `2026.04.12` (from `document_extractions.extraction_policy_version`).
