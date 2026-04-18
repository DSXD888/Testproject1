---
manifest_version: 1
skill_id: pdf-extraction/tender-specification
skill_version: 1.0.0
job_id: job-308163d1-b60e-40f6-8bac-fe49bc6610b7
session_id: session-semantic-7c0621097991d053d15435910d4630ca0f0a91bd8483b7d287f936ba5495003d
execution_mode: contract
started_at: 2026-04-18T00:00:00Z
completed_at: 2026-04-18T00:00:00Z
outcome_class: success_with_warnings
items_processed: 1
items_skipped: 0
items_failed: 0
warnings_count: 5
agent_claimed_pages_visited: 0
agent_claimed_source_url: not_applicable
agent_claimed_fingerprint_state: not_applicable
agent_claimed_locale_state: en-GB
agent_claimed_currency_state: GBP
---

# Run Manifest — pdf-extraction/tender-specification

## Summary

Extracted lot-level structure and evaluation criteria from ESPO Framework 107_26 Tender Submission Document (document_extraction_id `e88faa6c-a729-4029-95cd-8a32dbc00467`, source_content_hash `4ffe5f2383d289eecf0bf586c9d40034f0f7c529de97dbbb2c60aca0e6dcbd58`). Emitted exactly one lot (`lot_number = "1"`) covering the framework's combined scope, two `product_requirement_seeds` for the two equipment families, and zero CPV codes / zero technical standards because this document is the Tender Submission form and does not itself reproduce the specification or cite standards. Overall extraction confidence 6500 bps — confidence for the lot title, evaluation criteria and framework period is high; confidence for CPV codes, technical standards and product-level specifics is intentionally low because those details live in the companion Invitation to Tender (Section 2) not included in this payload. The execution mode was `contract`, so no pages were visited, no browser was involved, and no live source URL was fetched.

## Steps Taken

1. Read Docling markdown for document_extraction_id `e88faa6c-a729-4029-95cd-8a32dbc00467` from `state.document_extractions.extracted_markdown` (policy_version 2026.04.12, 14 sections, 7 tables, ~30.6 KB markdown).
2. Resolved process / notice context from `state.procurement_processes` / `state.contract_notices` / `state.notice_lots` for cross-reference only (not used as authoritative lot/CPV source because the skill's contract says pre-existing lot rows must not override the document).
3. Parsed the document's contents list (Sections 1–6) to identify the six canonical sections: Response to Requirement & Specification, Non-Price Method Statements (Q1–Q6), Price, Additional Benefits, Confidential Information Schedule, Tendering Declaration.
4. Identified the framework title, reference (`107_26`), framework period (`24 June 2026 – 31 January 2028`, extendable to `23 June 2030`) and tender closing date (`12 noon, 15 May 2026`) from the front-matter.
5. Extracted the six evaluation criteria Q1–Q6 with their declared weightings (3, 3, 5, 4, 4, 1) directly from Section 2 Method Statement rows. Added a seventh `Price` criterion anchored at Section 3 with `weighting_text` marked as not stated in this document (defers to Section 5 of the ITT).
6. Extracted geographical coverage requirements (mandatory nine regions of England; optional Scotland, Wales, Northern Ireland) and administrative requirements (120-day tender validity, Central Digital Platform registration, Framework Fee allowance, early-payment discount option at 7/14/21 days).
7. Decomposed the framework scope into two `product_requirement_seeds`: `indoor-gymnasium-fitness-sport-equipment` and `outdoor-schools-play-sport-equipment`, each cited against the front-matter framework title; kept both seeds under a single lot (`lot_number = "1"`) because the document does not split the framework into multiple lots.
8. Computed `source_markdown_hash = 0fe22bcd0541d1e53becd5bc06d99c7623d3aed2722a30a6095c1f3538e2b24d` over the docling markdown.
9. Wrote `result.json` into the session workdir and validated it in-process against `skills/pdf-extraction/tender-specification/result.schema.json` — JSON Schema validation passes.
10. Produced this run manifest for daemon finalization.

## Decisions Made

- **Single lot, not two.** The document's title combines two equipment families under one ESPO framework reference (`107_26`) and Section 1 speaks of "the requirement and specification" (singular) plus "Goods and/or Services" uniformly. The pre-existing `notice_lots` row also stores a single lot. Emitted `lot_number = "1"` covering both families rather than inventing two lots without evidence for a split.
- **CPV codes left empty.** The notice metadata carries CPVs `37400000-2` and `50000000-5`, but the Tender Submission Document does not cite them. Per the skill's "do not guess without citing evidence" rule, `lots[0].cpv_codes` and each seed's `cpv_codes` are `[]`. A warning (`NO_EXPLICIT_CPV_CODES`) records this decision for the daemon.
- **Technical standards left empty.** No named standards (BS EN 1176 / 1177, BS EN 913, BS EN 957, BS 7188 etc.) appear anywhere in the markdown. Rather than fabricate domain-plausible standards, emitted `technical_standards: []` and flagged it with `NO_EXPLICIT_TECHNICAL_STANDARDS`.
- **Price emitted as a separate evaluation criterion with weighting_text deferring to ITT Section 5.** Section 3 PRICE clearly establishes Price as an evaluated dimension but the weighting methodology is explicitly outside this document. Added a `PRICE_WEIGHTING_NOT_STATED` warning.
- **Two product requirement seeds despite this being a services framework.** The skill targets `target_domain = "lots"` with candidate seeds for downstream product_candidates. Modelled each equipment family as a `service_bundle` requirement_type seed anchored to the framework title.
- **Page references use logical section tokens** (e.g. `front-matter`, `section-1`, `section-2`, `section-3`, `section-6`) because the Docling markdown in this payload does not expose reliable page numbers (`page_count` on the extraction row is `0`), and the document's own structure is its Contents list.
- **outcome_class = `success_with_warnings`.** The result.json is complete and schema-valid; all five warnings are about content-absence decisions that are expected for a Tender Submission form rather than failures.

## Warnings

1. `SPEC_DETAIL_REFERENCED_NOT_INCLUDED` — detailed requirement/specification sits in Section 2 of the Invitation to Tender, not in this document; product-level and standards detail deferred.
2. `NO_EXPLICIT_CPV_CODES` — notice metadata has CPVs but the document itself does not cite them; emitted empty.
3. `NO_EXPLICIT_TECHNICAL_STANDARDS` — no named standards appear in the document; emitted empty.
4. `SINGLE_LOT_INFERRED` — framework treated as one lot (matching pre-existing `notice_lots` row) because the document does not split into lots.
5. `PRICE_WEIGHTING_NOT_STATED` — Price is evaluated but its weighting is deferred to ITT Section 5.

## Evidence

- **Framework title / lot primary anchor** — front-matter: "ESPO reference 107_26 Inspection & Maintenance of Indoor Gymnasium, Fitness & Sport Equipment and Outdoor Schools Play & Sport Equipment".
- **Framework period** — front-matter: "Framework Period 24 June 2026 – 31 January 2028 with the option to extend up to 23 June 2030".
- **Closing date** — front-matter: "Closing date for submission of tender 12 noon, 15 May 2026".
- **Compliance with specification (defers to ITT §2)** — Section 1: "I confirm I comply with all essential elements of the requirement and specification as outlined in Section 2 of the Invitation to Tender, Requirement (including Specification)".
- **Geographical coverage (mandatory)** — Section 1: "The mandatory requirement is for Suppliers to provide Goods and/or Services to the nine defined regions of England".
- **Method Statement anchors** — Section 2 rows Q1–Q6 with weightings (Weighting 3, 3, 5, 4, 4, 1 respectively).
- **Price defers to ITT §5** — Section 3: "Tenderers are required to detail in the Pricing Schedule (Appendix B) their prices … advised to ensure they fully understand the evaluation methodology to be used to assess price, as described in Section 5 of the Invitation to Tender, Evaluation of Tenders".
- **120-day tender validity** — Section 6 (Tendering Declaration, point 6): "this offer shall remain valid and open for acceptance for a period of 120 days from the date of submission".
- **Daemon appendix cross-refs** — `state.document_extractions.id = e88faa6c-a729-4029-95cd-8a32dbc00467`, `state.document_extractions.source_content_hash = 4ffe5f2383d289eecf0bf586c9d40034f0f7c529de97dbbb2c60aca0e6dcbd58`, `state.document_extractions.extraction_policy_version = 2026.04.12`, `state.notice_lots` confirms pre-existing single-lot structure for `notice_id = pc-3ba4e6b8-9cef-f011-813c-005056b64545`.
- **Result contract fields** — `result.json` emits `lots.length = 1`, `product_requirement_seeds.length = 2`, `warnings.length = 5`, `evidence_anchors.length = 8`, `extraction_confidence_bps = 6500`, `extraction_metadata.target_domain = "lots"`, `extraction_metadata.extraction_scope = "full_document"`, `extraction_metadata.source_markdown_hash = 0fe22bcd0541d1e53becd5bc06d99c7623d3aed2722a30a6095c1f3538e2b24d`.
