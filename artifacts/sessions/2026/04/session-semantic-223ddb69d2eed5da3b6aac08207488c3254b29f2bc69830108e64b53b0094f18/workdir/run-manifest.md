---
manifest_version: 1
skill_id: pdf-extraction/tender-specification
skill_version: 1.0.0
job_id: job-e6fd7516-e0d7-4d4d-b6a4-215433511579
session_id: session-semantic-223ddb69d2eed5da3b6aac08207488c3254b29f2bc69830108e64b53b0094f18
execution_mode: contract
started_at: 2026-04-18T00:00:00Z
completed_at: 2026-04-18T00:00:00Z
outcome_class: partial
items_processed: 1
items_skipped: 0
items_failed: 0
warnings_count: 5
agent_claimed_pages_visited: 0
agent_claimed_source_url: "state.db://document_extractions/6fc3882b-e594-4bcc-95a1-4c20e775148c"
agent_claimed_fingerprint_state: not_applicable
agent_claimed_locale_state: en-GB
agent_claimed_currency_state: GBP
---

# Run Manifest — pdf-extraction/tender-specification

## Summary
Processed a single full_document extraction packet for document_extraction_id
`6fc3882b-e594-4bcc-95a1-4c20e775148c` under process `proc-pc-dc1daee2-772e-f111-813c-005056b64545`
(notice `pc-dc1daee2-772e-f111-813c-005056b64545`, document set
`tds-pc-dc1daee2-772e-f111-813c-005056b64545`). Notice title is "6278 - Building Services Framework -
Lots 1-11" (London Borough of Bexley, deadline 2026-05-18). The Docling markdown loaded from
`document_extractions.extracted_markdown` is only ~13KB / 4 sections / 39 lines, and its actual
content is a PSQ (pre-qualification) **reference questionnaire form** for Lot 11 (Appendix 1) plus a
generic referee cover letter (Appendix 2). The document does not contain a technical specification,
product schedule, CPV-to-lot mapping, or quantitative requirement list despite its recorded
`document_role = technical_specification`. One lot (Lot 11 — Provision of Minor Disability
Adaptations in privately owned properties) was extracted with high-confidence identity and seven
evaluation criteria drawn verbatim from Part B of the reference form; no product requirements, no
technical standards, and no CPV codes were emitted because none are cited in this document.

## Steps Taken
1. Resolved the extraction packet from `state.db.document_extractions` using the `document_extraction_id`
   in the job parameters (no separate docling markdown file exists on disk for this job — markdown
   is stored inline in `document_extractions.extracted_markdown`).
2. Read process/notice metadata (`contract_notices.title`, `buyer_name`, `main_category`) to frame
   the extraction (used as context only; not asserted into lot rows).
3. Computed the full markdown body length (13,229 chars, 4 sections, 0 page_count per upstream
   Docling) and scanned for lot boundaries, evaluation blocks, CPVs, standards and product
   requirements.
4. Identified a single lot (Lot 11) from the document title strings "6278 – Building Services
   Framework" and "Lot 11 Provision of Minor Disability Adaptations (minor building works) in
   privately owned properties". No other lot numbers appear in the body.
5. Extracted Part B of the reference form as seven evaluation_criteria entries, each with verbatim
   anchor_text from the markdown.
6. Left `product_requirements`, `technical_standards`, `cpv_codes`, and `product_requirement_seeds`
   empty — the document provides no such content and fabricating seeds would violate the no-guessing
   constraint in the SKILL.md.
7. Raised five structured warnings covering: document-role mismatch, absence of product requirements,
   absence of CPV codes, zero page count, and lot scope being reconstructed from header rather than
   a substantive scope paragraph.
8. Wrote `result.json` to the session workdir and produced this run manifest.

## Decisions Made
- **Emit one lot, not zero.** The document itself is a reference form, but it unambiguously names
  Lot 11 and scopes its reference process to Lot 11 only. Emitting zero lots would lose the
  Lot-11 identity signal; emitting one lot with a clear `LOT_SCOPE_INFERRED_FROM_TITLE` warning
  preserves both the identity and the honest confidence.
- **Do not inherit notice-level CPVs onto the lot.** The notice carries CPVs (50700000-2,
  50750000-7, 71315000-9) on the parent framework, but the document provides no evidence mapping
  those CPVs to Lot 11 specifically. Per the SKILL.md ("do not guess without citing evidence
  anchors"), the lot's `cpv_codes` array is left empty and a `NO_CPV_IN_SOURCE` warning explains
  why.
- **No product requirement seeds.** A PSQ reference form contains no product/part/SKU signals.
  Emitting speculative seeds ("disability adaptation kit", etc.) would pollute
  `product_candidates` downstream; the daemon should source Lot 11 seeds from the separate
  specification / scope-of-service documents in this `document_set_id` if they are processed under
  tender-specification or bill-of-quantities overlays.
- **Use section references as page anchors.** Upstream Docling recorded `page_count = 0`, so
  every `evidence_anchor` uses `page_ref = "Appendix 1"` or `"Appendix 2"` with a more specific
  `section_ref` — this preserves traceability without inventing page numbers.
- **`extraction_confidence_bps = 5200`.** The identity slice (lot number + title + evaluation
  criteria) is high-confidence, but the scope/product/standards slice is empty by source. A
  mid-range score signals to the validator and to the downstream trust-state logic that this
  extraction is authoritative for evaluation criteria only, not for specification content.
- **Outcome class = `partial`.** The packet was fully processed and the result contract is
  well-formed, but key fields (product requirements, technical standards, CPVs) are empty because
  the source lacks them. This matches the "partial" semantics better than "success".

## Warnings
See `result.json.warnings` for the structured list. Summary:
1. `DOCUMENT_ROLE_MISMATCH` — recorded role is `technical_specification`, actual content is a PSQ
   reference questionnaire + cover letter.
2. `NO_PRODUCT_REQUIREMENTS_IN_SOURCE` — no product seeds extractable.
3. `NO_CPV_IN_SOURCE` — no CPV codes cited in this document.
4. `PAGE_COUNT_ZERO` — upstream Docling extraction has `page_count = 0`; evidence anchors use
   section references.
5. `LOT_SCOPE_INFERRED_FROM_TITLE` — Lot 11 description reconstructed from header strings, not
   from a substantive scope paragraph in the body.

## Evidence
All claims in `result.json` cite evidence anchored in the Docling markdown for document_extraction
`6fc3882b-e594-4bcc-95a1-4c20e775148c` (source_content_hash
`7db3cbdf92cf9f46ac356a524190d2ba3319312f2d022948f49a62731a65bc2c`,
extraction_engine `docling 2.88.0`, policy version `2026.04.12`). Specifically:

- Lot 11 identity anchored at Appendix 1 header block: `"Lot 11 Provision of Minor Disability
  Adaptations (minor building works) in privately owned properties"`.
- Framework identity anchored at Appendix 1 header block: `"6278 – Building Services Framework"`.
- Evaluation criteria 1–5 anchored verbatim from Appendix 1, Part B rating rows.
- Evaluation criterion 6 (repeat-engagement) anchored at `"Would you employ this company for
  similar services (please select)  Yes  No"` in Part B.
- Evaluation criterion 7 (factual accuracy) anchored at the Part A / Part B boundary row `"Please
  confirm that the services provided, detailed by the applicant above, are accurate:"`.
- Requirement 1 (applicant must supply referee contact details) anchored in Part A rows `"Name and
  contact details of applicant:"`, `"Name and contact details of referee:"`.
- Requirement 2 (return deadline + no-return-to-applicant rule) anchored at `"TO BE COMPLETED BY
  REFEREE AND RETURNED TO THE ADDRESS BELOW BY 18th May 2026 by 5pm"` and `"DO NOT SEND BACK TO
  APPLICANT"`.
- Negative evidence (no product requirements, no standards, no CPVs) is supported by reading the
  entire 4-section Docling markdown body and logged via `NO_PRODUCT_REQUIREMENTS_IN_SOURCE` and
  `NO_CPV_IN_SOURCE` warnings in `result.json`.
