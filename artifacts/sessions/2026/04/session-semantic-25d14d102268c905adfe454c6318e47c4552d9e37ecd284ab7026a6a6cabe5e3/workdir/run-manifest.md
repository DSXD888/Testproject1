---
manifest_version: 1.0.0
skill_id: pdf-extraction/tender-specification
skill_version: 1.0.0
job_id: job-10c23e53-e7bf-48a7-adf5-0d5aee19ef3c
session_id: session-semantic-25d14d102268c905adfe454c6318e47c4552d9e37ecd284ab7026a6a6cabe5e3
execution_mode: contract
started_at: 2026-04-18T06:14:00Z
completed_at: 2026-04-18T06:16:00Z
outcome_class: success_with_warnings
items_processed: 1
items_skipped: 0
items_failed: 0
warnings_count: 8
agent_claimed_pages_visited: 1
agent_claimed_source_url: https://procontract.due-north.com/RfxResponse/Index?rfxId=ea9d90b2-4833-f111-813c-005056b64545
agent_claimed_fingerprint_state: not_applicable_offline_structural_extraction
agent_claimed_locale_state: en-GB
agent_claimed_currency_state: GBP
---

## Summary

Processed a single-lot PSQ reference-form appendix document (source file: `6278 - PSQ Reference Appendices- Lot 8 - Provision of Planned and Preventative  Corporate Building Maintenance (PPM) & Remedial .docx`) for procurement process `proc-pc-dc1daee2-772e-f111-813c-005056b64545` (London Borough of Bexley — 6278 Building Services Framework, 11 lots). The document is routed through the tender-specification overlay because its persisted `document_role` is `technical_specification`, but its actual content is Appendix 1 (the referee reference form for Lot 8) plus Appendix 2 (referee cover letter template). Accordingly, one lot record was emitted (Lot 8, "Provision of Planned and Preventative Corporate Building Maintenance (PPM) & Remedial Repairs excluding full replacements") capturing the referee rubric as `evaluation_criteria` and a minimal `requirements` list derived verbatim from the form prompts. No product-requirement seeds were produced because Lot 8 is a services lot and the document is administrative. Eight warnings were raised to keep downstream consumers honest about what this document can and cannot support. Extraction confidence was set to 7200 bps, reflecting high structural fidelity (clear anchors) but low substantive coverage of the tender-specification surface the overlay normally expects.

## Steps Taken

1. Read the bound SKILL.md, job parameters, and `result.schema.json` to confirm target_domain must be `lots`, extraction_scope is `full_document`, and the schema requires `lots[].{lot_number,title,description,requirements,evaluation_criteria,cpv_codes,product_requirements,technical_standards,evidence_anchor}`.
2. Queried `document_extractions` row `202549ba-b037-4dfa-9c7c-507683934afd` for the Docling output, confirming: page_count=0, table_count=3, section_count=4, expected_overlays=`["pdf-extraction/tender-specification"]`, policy_version=`2026.04.12`, source_content_hash=`ce067d33277b051b6ef2a259f0118c111cadbc4f087988864cafdea4cb618b18`, document_title=`6278 - PSQ Reference Appendices- Lot 8 - Provision of Planned and Preventative  Corporate Building Maintenance (PPM) & Remedial`, document_role=`technical_specification`.
3. Read the extracted markdown in full (15,126 chars, 34 lines). Identified the document as a two-part referee reference form (Appendix 1 at `#/texts/0`, Part B rubric at `#/texts/6`, Appendix 2 at `#/texts/38`).
4. Cross-referenced `contract_notices` and `notice_lots` for notice `pc-dc1daee2-772e-f111-813c-005056b64545`. Confirmed Lot 8 title matches the canonical `notice_lots` row (`pc-dc1daee2-772e-f111-813c-005056b64545-lot-8`): "Provision of Planned and Preventative Corporate Building Maintenance (PPM) & Remedial Repairs excluding full replacements". Noted buyer = London Borough of Bexley, deadline_at = 2026-05-18T12:00:00Z, 11 lots in total.
5. Derived one lot record. Requirements drawn verbatim from form prompts; evaluation_criteria derived verbatim from Part B rubric items; left cpv_codes, product_requirements, and technical_standards empty because none are anchored in the document itself.
6. Did not emit any `product_requirement_seeds` — services procurement with zero procurable SKUs.
7. Enumerated seven primary `evidence_anchors` (appendix headings, form header, rubric prompts, routing address, Appendix 2 stub).
8. Raised eight warnings spanning role mismatch, empty product/CPV/standards sets, single-lot scope, Appendix 2 being a stub, and the deadline-semantics clash between form-return time and notice submission time.
9. Validated the emitted JSON against `result.schema.json` before writing.

## Decisions Made

- **DECISION**: Accept `document_role=technical_specification` as routing truth even though content is administrative. Did not attempt to override the classification from within the skill — that is a daemon/policy-layer concern. Instead, emitted a `DOCUMENT_ROLE_MISMATCH` warning so downstream consumers (and the validator) can treat this extraction as low substantive content.
- **DECISION**: Used the referee rubric items as `evaluation_criteria`. These are the only structured evaluation-like items in the document. However, they scope the referee's past-performance judgment, not the live tender evaluation. Captured this scope caveat in `EVALUATION_CRITERIA_SCOPE_NOTE` so the validator/harmoniser does not conflate them with actual tender evaluation weightings.
- **DECISION**: Left `cpv_codes` empty inside the lot despite the notice having CPV codes at the process level. The overlay contract requires evidence anchors, and no CPV text appears in the markdown. `CPV_NOT_STATED_IN_DOCUMENT` warning emitted to make this explicit rather than silently inheriting notice-level CPV.
- **DECISION**: Derived `requirements` conservatively — only bullets I could anchor to verbatim form prompts (referee contact routing, annual value disclosure, self-delivery capability for small and medium project works). Did not invent scope items (e.g. concrete PPM service categories, response-time SLAs, asset scope) that the document does not state.
- **DECISION**: For the Project Management rubric criterion, captured the Lot 8 wording "self-deliver small and medium project works" (note: the parallel Lot 5 form narrows this to "Condition Survey project works" — this Lot 8 form does NOT, so the generic wording is used).
- **DECISION**: `extraction_confidence_bps = 7200`. Confidence is high on the structural facts that are present (lot number, title, evidence anchors) but low on the absence of typical tender-specification content, and that distribution produces an overall mid-70s bps score. Per-field confidences encoded in `field_confidences` reflect the split.
- **DECISION**: `extraction_scope = full_document` per job parameters; source markdown hash reused from the `document_extractions.source_content_hash` row value so it is cryptographically pinnable, not re-derived.
- **DECISION**: Classified outcome as `success_with_warnings` (not `partial_failure`). The schema is satisfied and the lot was correctly identified; the caveats are about document content, not extraction failure.

## Warnings

1. **DOCUMENT_ROLE_MISMATCH** — Document is a PSQ referee reference form, not a substantive tender specification, despite routing. Evidence: `#/texts/0`, `#/texts/38`.
2. **NO_PRODUCT_REQUIREMENTS** — Services lot + administrative doc → `product_requirement_seeds` intentionally empty. Evidence: `#/texts/0`.
3. **EVALUATION_CRITERIA_SCOPE_NOTE** — Six criteria are referee scoring rubric, not tender award criteria. Evidence: `#/texts/6`.
4. **CPV_NOT_STATED_IN_DOCUMENT** — No CPV codes appear inside the document body; notice-level CPVs not inherited. Evidence: none (absence).
5. **NO_TECHNICAL_STANDARDS** — No standards (e.g. BS, ISO, BSRIA, CIBSE) cited in text; kept empty rather than invented. Evidence: `#/texts/6`.
6. **SINGLE_LOT_DOCUMENT** — Only Lot 8 is in scope; the notice has 11 lots, the other 10 require their own extractions. Evidence: `#/texts/0`.
7. **APPENDIX_2_STUB_ONLY** — Appendix 2 is a mostly-placeholder cover letter template adding no scope/requirements. Evidence: `#/texts/38`.
8. **DEADLINE_REFERS_TO_REFERENCE_RETURN_ONLY** — "18th May 2026 by 5pm" is the referee return deadline, not the tender submission deadline (notice `deadline_at` is 2026-05-18T12:00:00Z / noon). Evidence: `#/texts/6`.

## Evidence

- **Lot identity** — Header text "6278 - Lot 8: Provision of Planned and Preventative  Corporate Building Maintenance (PPM) & Remedial Repairs excluding full replacements" at `#/texts/0`, `p1`. This matches the canonical title in `notice_lots` for `pc-dc1daee2-772e-f111-813c-005056b64545-lot-8`.
- **Referee rubric (source for evaluation_criteria)** — The embedded Part B cell at `#/texts/6`, `p1`, contains the rubric band ("Failed to meet standard in all areas / … / Exceeded standard in all areas") and the six prompt lines (Supply chain resilience, Service Provision, Resource, Value for Money, Contract Management, Project Management) that were lifted verbatim.
- **Routing address** — "Please return the reference to: Procurement@bexley.gov.uk" at `#/texts/6`, `p1`. Also present: full Bexley procurement address (Civic Offices, Bexleyheath, Broadway, Kent, DA6 7LB).
- **Reference return deadline (not tender deadline)** — "TO BE COMPLETED BY REFEREE AND RETURNED TO THE ADDRESS BELOW BY 18th May 2026 by 5pm" at `#/texts/6`. Compared against `contract_notices.deadline_at = 2026-05-18T12:00:00Z` to flag the time mismatch.
- **Appendix 2 content** — "The company named on the attached Reference Questionnaire has applied for a Bexley Procurement exercise …" at `#/texts/38` is a template cover letter with a placeholder "INSERT DATE & TIME" field; no lot scope content added.
- **Source markdown fingerprint** — Full markdown is 15,126 chars with sha256 `ce067d33277b051b6ef2a259f0118c111cadbc4f087988864cafdea4cb618b18` (read directly from `document_extractions.source_content_hash`, not re-hashed). Emitted as `extraction_metadata.source_markdown_hash`.
- **Raw source path** — `/Users/alinaaitova/Desktop/Testproject1/artifacts/downloads/procontract/afb3b5dc-772e-f111-813c-005056b64545/6278 - Reference Appendices_extracted/Reference Appendicies/6278 - PSQ Reference Appendices- Lot 8 - Provision of Planned and Preventative  Corporate Building Maintenance (PPM) & Remedial .docx` (from `raw_documents.storage_path`).
- **Overlay routing** — `document_extractions.expected_overlays_json = ["pdf-extraction/tender-specification"]` confirms this skill is the correct binding for the job.
