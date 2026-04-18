---
manifest_version: 1
skill_id: pdf-extraction/tender-specification
skill_version: 1.0.0
job_id: job-e385438e-ba3a-413c-8ef1-c7cb456a2c18
session_id: session-semantic-72ff9053ec212c96254a2504c114d396d21e06a76ea8c40a21009fa61dae3d10
execution_mode: contract
started_at: 2026-04-18T06:12:00Z
completed_at: 2026-04-18T06:12:20Z
outcome_class: partial_success
items_processed: 1
items_skipped: 0
items_failed: 0
warnings_count: 7
agent_claimed_pages_visited: 1
agent_claimed_source_url: null
agent_claimed_fingerprint_state: not_applicable
agent_claimed_locale_state: en-GB
agent_claimed_currency_state: not_applicable
---

# Run Manifest — pdf-extraction/tender-specification

## Summary

Processed document_extraction `2231204f-1cc0-4ee5-a4f6-80fad7e6c61e` for process `proc-pc-dc1daee2-772e-f111-813c-005056b64545` (London Borough of Bexley — "6278 - Building Services Framework - Lots 1-11"). The raw Docling payload has `page_count = 0`, `section_count = 4`, `table_count = 3`, and `document_role = technical_specification`.

The actual document body is a **PSQ (Pre-Qualification Questionnaire) reference appendix**: Appendix 1 is a reference form (Part A completed by the applicant, Part B completed by a referee), scoped to "6278 - Lot 7: Provision of Lift Services - Corporate Buildings"; Appendix 2 is a cover letter to referees. The document does not carry primary tender-specification content — no product requirements, no technical standards, no scored evaluation model, no CPV codes in the body.

One lot row was emitted (Lot 7) using the in-document lot heading as the evidence anchor, plus seven reference-quality evaluation criteria extracted verbatim from Part B. `product_requirement_seeds` is empty and `technical_standards` / `cpv_codes` on the lot are empty because the document does not cite any. `extraction_confidence_bps` is set to 3200 and seven warnings explain the administrative-only classification, the missing content categories, and the mismatch between resolved `document_role=technical_specification` and the observed content.

## Steps Taken

1. Resolved execution context — `skill_id=pdf-extraction/tender-specification`, `extraction_scope=full_document`, `target_domain=lots`, `policy_version=2026.04.12`, `document_extraction_id=2231204f-1cc0-4ee5-a4f6-80fad7e6c61e`.
2. Loaded the Docling markdown (15,025 bytes, 34 lines) and structural payload (`section_count=4`, `table_count=3`, `image_count=0`, `empty_page_count=0`). Computed `source_markdown_hash = d97018b070cb8b1c8510c4c126b12119f5ceb078fdf9a5679d069ca373fc24a6` locally.
3. Read notice-level context (`pc-dc1daee2-772e-f111-813c-005056b64545`) for cross-reference only — titled "6278 - Building Services Framework - Lots 1-11" with buyer "London Borough of Bexley" and process-level CPV hints `50700000-2`, `50750000-7`, `71315000-9`. Did not inject notice CPVs into the lot row because the SKILL requires evidence anchors rooted in the document itself.
4. Scanned section headings: `Appendix 1`, `TO BE COMPLETED BY REFEREE AND RETURNED TO THE ADDRESS BELOW BY 18th May 2026 by 5pm`, `Appendix 2` (and the root `header-0`).
5. Classified document body as administrative (PSQ reference appendix) — neither a requirements catalogue, nor a scored evaluation matrix, nor a product specification.
6. Extracted the sole lot identity observable in the document from the row "6278 - Lot 7: Provision of Lift Services  - Corporate Buildings" inside the Appendix 1 table header (section_ref `#/groups/0`).
7. Enumerated the seven referee-evaluation items that appear verbatim inside the Part B cell (section_ref `#/texts/4`): Supply chain resilience, Service Provision, Resource, Value for Money, Contract Management, Project Management, and the binary "Would you employ this company for similar services" reuse recommendation.
8. Left `product_requirements`, `technical_standards`, `cpv_codes`, and `product_requirement_seeds` empty, since the document body cites none.
9. Emitted seven warnings covering: administrative-only content, no product requirements, no technical standards, no CPV in body, non-numeric (qualitative) evaluation scale, reference-return date vs tender deadline ambiguity and the `****INSERT DATE & TIME****` template placeholder in Appendix 2, and the document-role mismatch (`technical_specification` vs observed PSQ reference appendix).
10. Wrote `result.json` into the session workdir and validated it against `skills/pdf-extraction/tender-specification/result.schema.json` (passes JSON Schema draft 2020-12).

## Decisions Made

- **Emit one lot rather than zero.** The document is Lot-7-scoped per its own heading. Emitting a lot with empty `requirements`/`product_requirements`/`technical_standards` plus a high-visibility warning is more useful to downstream consumers than suppressing the lot entirely, and matches the result schema without fabricating content.
- **Reference-form Part B categories surfaced as `evaluation_criteria`.** These are evaluation items *for a referee*, not award criteria for the tender itself. They are still the clearest evidence-bearing "evaluation_criteria" content in this document and are labelled as qualitative with no numeric weighting. A warning (`EVALUATION_WEIGHTING_NOT_NUMERIC`) and per-criterion `weighting_text` flag the qualitative 5-point scale so the daemon will not treat them as scored award criteria.
- **No CPV codes attached to the lot.** Notice-level CPVs (50750000-7 Lift-maintenance services and two neighbours) are plausible but are not cited in the document body. SKILL explicitly requires evidence anchors for claims, and the schema makes CPV rooted in the lot; I preferred empty + warning to unanchored carryover.
- **No product requirement seeds.** The document lists no goods, materials, or quantities. Emitting any seed would be speculative and would conflict with the "Do not guess" non-goal.
- **Confidence 3200 bps.** Reflects that the one lot and seven evaluation items are strongly grounded, but the document is off-profile for a tender specification and the downstream value of this extraction is limited. Per-field confidences are explicit (zeros for CPV / product requirements / standards / seeds).
- **`outcome_class = partial_success`.** The skill produced a schema-valid payload with anchored evidence, but major result fields are empty by design because of input content; warnings flag this clearly for the validator.

## Warnings

1. `ADMINISTRATIVE_CONTENT_ONLY` — body is a PSQ reference appendix, not a primary tender specification. Anchors: `#/texts/0`, `#/texts/1`, `#/groups/0`.
2. `NO_PRODUCT_REQUIREMENTS_FOUND` — `product_requirement_seeds` intentionally empty.
3. `NO_TECHNICAL_STANDARDS_FOUND` — no EN/BS/ISO/LOLER or comparable standards references in the markdown.
4. `NO_CPV_IN_DOCUMENT` — CPVs present on the parent process are not cited in the document body; `cpv_codes` left empty to avoid unanchored carryover.
5. `EVALUATION_WEIGHTING_NOT_NUMERIC` — Part B uses a 5-point tick scale (Failed/Met/Exceeded); no percentage or points weighting.
6. `DATE_REFERENCE_IN_FORM` — "18th May 2026 by 5pm" is the *reference return* deadline, not the tender submission deadline; Appendix 2 also contains placeholder text `****INSERT DATE & TIME****`.
7. `DOCUMENT_ROLE_MISMATCH` — resolved `document_role=technical_specification` but observed content is a PSQ reference appendix; downstream routing should consider reclassifying.

## Evidence

All claims are rooted in observable fields of the Docling structural payload for extraction `2231204f-1cc0-4ee5-a4f6-80fad7e6c61e`:

- **Lot identity (Lot 7 — Provision of Lift Services - Corporate Buildings):** anchor text `"6278 - Lot 7: Provision of Lift Services  - Corporate Buildings"` at section_ref `#/groups/0` (Appendix 1 table header on page 1). Also reinforced by `document_title = "6278 - PSQ Reference Appendices Lot 7 - Provision of Lift Services  - Corporate Buildings"` in the extraction record.
- **Referee-scale evaluation criteria:** section_ref `#/texts/4` (Part B of Appendix 1). Verbatim items cited in `evaluation_criteria[].evidence_anchor.anchor_text` for Supply chain resilience, Service Provision, Resource, Value for Money, Contract Management, Project Management, and the Yes/No reuse recommendation.
- **Reference return deadline / address:** section_ref `#/texts/4`, anchor text `"TO BE COMPLETED BY REFEREE AND RETURNED TO THE ADDRESS BELOW BY 18th May 2026 by 5pm"` and `"Please return the reference to: Procurement@bexley.gov.uk"`.
- **Appendix 2 cover letter and template placeholder:** section_ref `#/texts/34` (heading "Appendix 2") and section_ref `#/texts/36` (body text referencing `****INSERT DATE & TIME****`).
- **Absence claims:** the extracted markdown (15,025 bytes) was scanned in full; no occurrences of "CPV", "EN ", "BS ", "ISO ", "LOLER", "specification", or "requirement" strings were observed, so warnings `NO_PRODUCT_REQUIREMENTS_FOUND`, `NO_TECHNICAL_STANDARDS_FOUND`, and `NO_CPV_IN_DOCUMENT` are grounded in document-wide absence.
- **Daemon appendix refs:** `document_extractions.id=2231204f-1cc0-4ee5-a4f6-80fad7e6c61e`, `source_content_hash=1595ec7b32a52078c7e476370fdcd53f9e851c923a646899a09ea72b3b30c326`, `extraction_policy_version=2026.04.12`, `expected_overlays_json=["pdf-extraction/tender-specification"]`.
- **Computed artifact hash:** `source_markdown_hash=d97018b070cb8b1c8510c4c126b12119f5ceb078fdf9a5679d069ca373fc24a6` (sha256 of `extracted_markdown`).
