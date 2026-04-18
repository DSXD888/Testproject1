---
manifest_version: 1
skill_id: pdf-extraction/tender-specification
skill_version: 1.0.0
job_id: job-607d6a4a-2db3-44a9-80cf-a154d7aed3c0
session_id: session-semantic-489c27f59aa64d17170d64d417741a975ec6005bd05b32752bc736e790a621f1
execution_mode: contract
started_at: 2026-04-18T08:18:00Z
completed_at: 2026-04-18T08:18:25Z
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

Processed `document_extraction.id=829faa0c-2889-4171-af2d-a8cced2f9a90` for process `proc-pc-dc1daee2-772e-f111-813c-005056b64545` (London Borough of Bexley — "6278 - Building Services Framework - Lots 1-11"). The Docling payload has `page_count=0`, `section_count=4`, `table_count=3`, `image_count=0`, and resolved `document_role=technical_specification`.

The actual document body is a **PSQ (Pre-Qualification Questionnaire) reference appendix** scoped to "6278 - Lot 9: Provision of Reactive Repairs to Corporate Building". Appendix 1 is a reference form (Part A completed by the applicant, Part B completed by a referee). Appendix 2 is a cover letter to referees. The document does not carry primary tender-specification content — no product requirements, no technical standards, no scored evaluation model, no CPV codes in the body.

One lot row was emitted (Lot 9) using the in-document lot heading as its primary evidence anchor, plus seven reference-quality evaluation criteria extracted verbatim from Part B. `product_requirement_seeds` is empty; `technical_standards`, `cpv_codes`, and `product_requirements` on the lot are empty because the document does not cite any. `extraction_confidence_bps=3200` and seven warnings explain the administrative-only classification, the missing content categories, the qualitative (non-numeric) evaluation scale, the date-ambiguity risk, and the mismatch between the resolved `document_role=technical_specification` and the observed content.

## Steps Taken

1. Resolved execution context from job parameters — `skill_id=pdf-extraction/tender-specification`, `extraction_scope=full_document`, `target_domain=lots`, `policy_version=2026.04.12`, `document_extraction_id=829faa0c-2889-4171-af2d-a8cced2f9a90`, `document_set_id=tds-pc-dc1daee2-772e-f111-813c-005056b64545`, `stage_run_id=stage-run-43d02438-92da-4df3-bc14-2ff454300565-proc-pc-dc1daee2-772e-f111-813c-005056b64545`.
2. Loaded the Docling markdown (`length=15028`, `lines=34`) and structural payload (`section_count=4`, `table_count=3`, `image_count=0`, `empty_page_count=0`). Computed `source_markdown_hash=bbd1c82d8552ad619511a2b2f6298cfdd8a88abd3382d6a433c760d7a8c2c881` via sha256 of `extracted_markdown`.
3. Read parent notice context (`contract_notices.id=pc-dc1daee2-772e-f111-813c-005056b64545`) for cross-reference only — titled "6278 - Building Services Framework - Lots 1-11", buyer "London Borough of Bexley", notice-level CPV hints `50700000-2`, `50750000-7`, `71315000-9`. Did not inject notice CPVs into the lot row because the SKILL requires evidence anchors rooted in the document itself.
4. Scanned section headings reported in `extracted_sections_json`: `header-0` (`#/groups/0`), `Appendix 1` (`#/texts/0`), `TO BE COMPLETED BY REFEREE AND RETURNED TO THE ADDRESS BELOW BY 18th May 2026 by 5pm` (`#/texts/5`), `Appendix 2` (`#/texts/41`).
5. Classified document body as administrative (PSQ reference appendix) — neither a requirements catalogue, nor a scored evaluation matrix, nor a product specification.
6. Extracted the sole lot identity observable in the document from the line "6278 - Lot 9: Provision of Reactive Repairs to Corporate Building" at `#/texts/2`. Cross-checked against `document_title` on the extraction row, which agrees ("6278 - PSQ Reference Appendices- Lot 9 - Provision of Reactive Repairs to Corporate Building").
7. Cross-checked the observed lot identity against `notice_lots.lot_number=9` titled "Provision of Reactive Repairs to Corporate Building" for the parent notice — matches. Used the canonical title verbatim on the lot row.
8. Enumerated the seven referee-evaluation items that appear verbatim inside the Part B cell at `#/texts/5`: Supply chain resilience, Service Provision, Resource, Value for Money, Contract Management, Project Management, and the binary "Would you employ this company for similar services" reuse recommendation.
9. Left `product_requirements`, `technical_standards`, `cpv_codes`, and `product_requirement_seeds` empty, since the document body cites none.
10. Emitted seven warnings covering: administrative-only content, no product requirements, no technical standards, no CPV in body, non-numeric (qualitative) evaluation scale, reference-return date vs tender deadline ambiguity plus the `****INSERT DATE & TIME****` template placeholder in Appendix 2, and the document-role mismatch (`technical_specification` vs observed PSQ reference appendix).
11. Wrote `result.json` into the session workdir and validated it against `skills/pdf-extraction/tender-specification/result.schema.json` — schema validation passes (JSON Schema draft 2020-12).

## Decisions Made

- **Emit one lot rather than zero.** The document is Lot-9-scoped per its own heading. Emitting a single lot with empty `requirements`/`product_requirements`/`technical_standards` plus a high-visibility warning is more useful to downstream consumers than suppressing the lot entirely, and matches the result schema without fabricating content. This is consistent with the prior Lot 7 sibling run on the same notice.
- **Reference-form Part B categories surfaced as `evaluation_criteria`.** These are evaluation items *for a referee*, not award criteria for the tender itself. They are still the clearest evidence-bearing "evaluation_criteria" content in this document and are labelled as qualitative with no numeric weighting. A warning (`EVALUATION_WEIGHTING_NOT_NUMERIC`) and per-criterion `weighting_text` flag the qualitative 5-point scale so the daemon will not treat them as scored award criteria.
- **No CPV codes attached to the lot.** Notice-level CPVs (50700000-2 Repair and maintenance of building installations; 50750000-7 Lift-maintenance services; 71315000-9 Building services) are plausible but are not cited in the document body. The SKILL explicitly requires evidence anchors for claims, and the schema makes CPV rooted in the lot; I preferred empty + warning to unanchored carryover.
- **No product requirement seeds.** The document lists no goods, materials, or quantities. Emitting any seed would be speculative and would conflict with the "Do not guess" non-goal.
- **Confidence 3200 bps.** Reflects that the one lot and seven evaluation items are strongly grounded, but the document is off-profile for a tender specification and the downstream value of this extraction is limited. Per-field confidences are explicit (zeros for CPV / product requirements / standards / seeds).
- **`outcome_class = partial_success`.** The skill produced a schema-valid payload with anchored evidence, but major result fields are empty by design because of input content; warnings flag this clearly for the validator.

## Warnings

1. `ADMINISTRATIVE_CONTENT_ONLY` — body is a PSQ reference appendix, not a primary tender specification. Anchors: `#/texts/0`, `#/texts/1`, `#/texts/2`.
2. `NO_PRODUCT_REQUIREMENTS_FOUND` — `product_requirement_seeds` intentionally empty.
3. `NO_TECHNICAL_STANDARDS_FOUND` — no EN/BS/ISO/CIBSE or comparable standards references in the markdown.
4. `NO_CPV_IN_DOCUMENT` — CPVs present on the parent process are not cited in the document body; `cpv_codes` left empty to avoid unanchored carryover.
5. `EVALUATION_WEIGHTING_NOT_NUMERIC` — Part B uses a 5-point tick scale (Failed/Met/Exceeded); no percentage or points weighting.
6. `DATE_REFERENCE_IN_FORM` — "18th May 2026 by 5pm" is the *reference return* deadline, not the tender submission deadline; Appendix 2 also contains placeholder text `****INSERT DATE & TIME****`.
7. `DOCUMENT_ROLE_MISMATCH` — resolved `document_role=technical_specification` but observed content is a PSQ reference appendix; downstream routing should consider reclassifying.

## Evidence

All claims are rooted in observable fields of the Docling structural payload for extraction `829faa0c-2889-4171-af2d-a8cced2f9a90`:

- **Lot identity (Lot 9 — Provision of Reactive Repairs to Corporate Building):** anchor text `"6278 - Lot 9: Provision  of Reactive Repairs to Corporate Building"` at section_ref `#/texts/2` (within Appendix 1 on page 1). Reinforced by `document_extractions.document_title = "6278 - PSQ Reference Appendices- Lot 9 - Provision of Reactive Repairs to Corporate Building"` and by `notice_lots(lot_number=9)` on the parent notice, whose canonical title is "Provision of Reactive Repairs to Corporate Building".
- **Referee-scale evaluation criteria:** section_ref `#/texts/5` (Part B of Appendix 1). Verbatim items cited in `evaluation_criteria[].evidence_anchor.anchor_text` for Supply chain resilience, Service Provision, Resource, Value for Money, Contract Management, Project Management, and the Yes/No reuse recommendation.
- **Reference return deadline / address:** section_ref `#/texts/5`, anchor text `"TO BE COMPLETED BY REFEREE AND RETURNED TO THE ADDRESS BELOW BY 18th May 2026 by 5pm"` and `"Please return the reference to: Procurement@bexley.gov.uk"`.
- **Appendix 2 cover letter and template placeholder:** section_ref `#/texts/41` (heading "Appendix 2") with body text referencing `****INSERT DATE & TIME****`.
- **Absence claims:** the extracted markdown (15,028 bytes, 34 lines) was scanned in full; no occurrences of "CPV", "EN ", "BS ", "ISO ", "CIBSE", "LOLER", "specification", or "requirement" strings were observed on material content rows, so warnings `NO_PRODUCT_REQUIREMENTS_FOUND`, `NO_TECHNICAL_STANDARDS_FOUND`, and `NO_CPV_IN_DOCUMENT` are grounded in document-wide absence.
- **Daemon appendix refs:** `document_extractions.id=829faa0c-2889-4171-af2d-a8cced2f9a90`, `source_content_hash=9f785e1a1112307716e9c02dffd0749c0a76e2ec52673c5b8e7724a0b8f4c00c`, `extraction_policy_version=2026.04.12`, `expected_overlays_json=["pdf-extraction/tender-specification"]`, `document_set_id=tds-pc-dc1daee2-772e-f111-813c-005056b64545`, `raw_document_id=raw-pc-dc1daee2-772e-f111-813c-005056b64545-ec2ee263193eebfa9ab6c2fde62af50d168e9aa2ae8cdc5ef341655c1cff70aa`, `stage_run_id=stage-run-43d02438-92da-4df3-bc14-2ff454300565-proc-pc-dc1daee2-772e-f111-813c-005056b64545`.
- **Computed artifact hash:** `source_markdown_hash=bbd1c82d8552ad619511a2b2f6298cfdd8a88abd3382d6a433c760d7a8c2c881` (sha256 of `extracted_markdown`).
