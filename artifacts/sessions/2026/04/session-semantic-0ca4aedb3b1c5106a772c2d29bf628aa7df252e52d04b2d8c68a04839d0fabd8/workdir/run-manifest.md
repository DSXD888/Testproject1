---
manifest_version: 1
skill_id: pdf-extraction/tender-specification
skill_version: 1.0.0
job_id: job-e9ae1535-ae67-4c71-941e-d8e92c02b2ed
session_id: session-semantic-0ca4aedb3b1c5106a772c2d29bf628aa7df252e52d04b2d8c68a04839d0fabd8
execution_mode: contract
started_at: 2026-04-18T07:55:00Z
completed_at: 2026-04-18T07:56:00Z
outcome_class: success_with_warnings
items_processed: 1
items_skipped: 0
items_failed: 0
warnings_count: 5
agent_claimed_pages_visited: 1
agent_claimed_source_url: "n/a (contract mode, no network fetch; source document resolved via document_extractions.id=48d5d08c-cf28-438c-b287-205a887ecd1c)"
agent_claimed_fingerprint_state: not_applicable
agent_claimed_locale_state: en-GB
agent_claimed_currency_state: GBP
---

## Summary

Ran pdf-extraction/tender-specification overlay on document_extraction_id=48d5d08c-cf28-438c-b287-205a887ecd1c for process proc-pc-dc1daee2-772e-f111-813c-005056b64545 (London Borough of Bexley, 6278 - Building Services Framework - Lots 1-11). The Docling markdown (15,105 chars, 1 page, 3 tables, 4 sections) identifies a single lot — Lot 3: Provision of Asbestos Assessments — via the Appendix 1 reference-form heading. The document body is a PSQ reference questionnaire + referee cover letter and contains no technical specification, scope of works, KPIs, CPV codes, pricing, or product-level requirements. Extraction therefore returned 1 lot with 6 referee-assessment evaluation criteria (Part B), 3 procedural requirements (PSQ form handling), zero CPV codes, zero technical standards, and zero product_requirement_seeds. Overall extraction_confidence_bps=3500, consistent with the document_role vs. document_content mismatch. Five warnings emitted including DOCUMENT_ROLE_MISMATCH so the daemon and downstream validator can treat this as a limited-basis extraction.

## Steps Taken

1. Resolved input artifacts: read extracted_markdown, extracted_sections_json, extracted_metadata_json and source_content_hash from document_extractions WHERE id='48d5d08c-cf28-438c-b287-205a887ecd1c'.
2. Confirmed document identity: document_title='6278 - PSQ Reference Appendices - Lot 3 - Provision of Asbestos Assessments', document_role='technical_specification' (assigned upstream, but body is clearly a PSQ form), page_count=0 (Docling reported 0 for this DOCX), extraction_engine='docling 2.88.0', extraction_policy_version='2026.04.12'.
3. Mapped section anchors from extracted_sections_json: #/texts/0 (Appendix 1 header + "6278 - Lot 3: Provision of Asbestos Assessments"), #/texts/5 (Part B referee instructions), #/texts/35 (Appendix 2 cover letter).
4. Cross-referenced parent process context: procurement_processes.title='6278 - Building Services Framework - Lots 1-11', buyer_name='London Borough of Bexley'. notice_lots already carries Lot 3 row with description mentioning "£60k per annum (Group 1)" — that value did NOT come from this PSQ document and was not included in the extracted description here.
5. Identified lot 3 by direct heading read: "6278 - Lot 3: Provision of Asbestos Assessments" (bold heading immediately below the Appendix 1 label, #/texts/0).
6. Harvested Part B evaluation criteria (6 rows) from the rich cell #/texts/5 containing the referee questionnaire: Supply chain resilience, Service Provision, Resource, Value for Money, Contract Management, Project Management. Recorded per-criterion evidence anchors verbatim from the cell text.
7. Noted that the Project Management criterion explicitly references "small and medium asbestos project works", corroborating the lot subject matter; no other body text references asbestos scope.
8. Enumerated applicant PSQ requirements (3 procedural items: Part A information items, Part B return instructions, and the Part A confirmation row).
9. Verified absence of: CPV codes, technical standards (HSG264, CAR 2012, BS ISO 17020, UKAS, licensable asbestos references, any other BS/ISO/EN standard), quantities, volumes, unit prices, product schedules. Emitted matching NO_* warnings.
10. Returned empty product_requirement_seeds. A PSQ reference form produces no quoteable candidates; synthesising them would be fabrication.
11. Computed source_markdown_hash = SHA-256 of extracted_markdown (hex digest) = 41d3f046d404867b578a2cad5d06be9c03807ed168956afe1974486caca41bf1.
12. Set extraction_metadata.target_domain='lots', extraction_scope='full_document', policy_version='2026.04.12', document_role='technical_specification' (preserving the upstream-assigned role, while the DOCUMENT_ROLE_MISMATCH warning flags that the body does not match that role).
13. Validated result.json against skills/pdf-extraction/tender-specification/result.schema.json (jsonschema validate returned OK).

## Decisions Made

- Lot 3 identity asserted from the Appendix 1 heading only, which is the sole direct evidence in the document body. All other Lot 3 metadata (annual value, group classification, list of other lots) was deliberately NOT pulled from the parent notice to keep the extraction strictly document-derived. Why: the skill's non-goals explicitly forbid guessing without citing evidence anchors in the source markdown, and the daemon already holds parent-process facts via notice_lots/contract_notices.
- Evaluation criteria were mapped from the referee-assessment dimensions (Part B) rather than left empty. Why: although these are not traditional scoring weightings, they are the only evaluation-adjacent statements in the document and they will be useful for validator context and audit traceability. Each criterion carries an evidence_anchor at #/texts/5. Confidence kept modest (6000) because they are PSQ reference dimensions, not formal tender evaluation criteria.
- product_requirement_seeds kept empty. Why: no product schedule, no SKU list, no quantities, no technical specifications; emitting seeds would introduce speculative candidates that the quoteability scoring gate would have to reject, wasting downstream cost.
- technical_standards kept empty. Why: the only domain-adjacent phrase is "small and medium asbestos project works"; it is not a standard reference. Adding a speculative standard (e.g. HSG264) would breach the skill's evidence-anchor requirement.
- cpv_codes kept empty. Why: no CPV code is printed anywhere in the body text. Parent notice likely carries CPV 90650000 / 71317100 range for asbestos survey services, but that is outside this skill's scope.
- Warning code set: DOCUMENT_ROLE_MISMATCH, NO_CPV_CODES_IN_DOCUMENT, NO_TECHNICAL_STANDARDS_IN_DOCUMENT, NO_PRODUCT_REQUIREMENTS_IN_DOCUMENT, REFEREE_RETURN_DEADLINE_ATTACHED_TO_REFERENCE_PROCESS_NOT_LOT. Why: each warning documents a concrete, actionable gap so the validator overlay and the downstream pricing/matching stages can route correctly (notably, treating this as a non-blocking limited-basis extraction rather than a failure).
- extraction_confidence_bps=3500. Why: identity + referee dimensions are high-signal, but the core payload (scope, products, standards, CPV, pricing) is zero; the composite score must reflect a limited-basis extraction rather than a full tender spec.

## Warnings

1. DOCUMENT_ROLE_MISMATCH — body is a PSQ reference appendix set, not a technical specification. Extraction was best-effort on the heading.
2. NO_CPV_CODES_IN_DOCUMENT — no CPV codes are present in the markdown body.
3. NO_TECHNICAL_STANDARDS_IN_DOCUMENT — no HSG264 / CAR 2012 / ISO 17020 / UKAS / other asbestos or general standards appear in the body.
4. NO_PRODUCT_REQUIREMENTS_IN_DOCUMENT — document does not describe any product, unit, volume, or quantity; product_requirement_seeds kept empty.
5. REFEREE_RETURN_DEADLINE_ATTACHED_TO_REFERENCE_PROCESS_NOT_LOT — the 18th May 2026 5pm date is for referee form return, not for the underlying Lot 3 tender deadline.

## Evidence

- Source document identity: document_extractions.id=48d5d08c-cf28-438c-b287-205a887ecd1c, raw_documents.id=raw-pc-dc1daee2-772e-f111-813c-005056b64545-9bd8ffb12803efa572185d229025b7f90a263961266a9dc33b6e8aa58fd5f3b8, storage_path="/Users/alinaaitova/Desktop/Testproject1/artifacts/downloads/procontract/afb3b5dc-772e-f111-813c-005056b64545/6278 - Reference Appendices_extracted/Reference Appendicies/6278 - PSQ Reference Appendices - Lot 3 - Provision of Asbestos Assessments.docx", content_type=application/vnd.openxmlformats-officedocument.wordprocessingml.document.
- Source markdown SHA-256: 41d3f046d404867b578a2cad5d06be9c03807ed168956afe1974486caca41bf1 (15,105 bytes).
- Docling section anchors used: #/texts/0 (Appendix 1 + "6278 - Lot 3: Provision of Asbestos Assessments"), #/texts/5 (Part B referee cell with the 6 assessment rows and the 18th May 2026 return instruction), #/texts/35 (Appendix 2 referee cover letter header).
- Lot identity anchor (verbatim): "6278 - Lot 3: Provision of Asbestos Assessments" at #/texts/0.
- Evaluation-criterion anchors (verbatim): "Supply chain resilience: Has the contractor provided a resilient and sustainable supply chain...", "Service Provision: Has the contractor consistently delivered...", "Resource: Have the contractor's staff held the necessary qualifications, certifications, and experience...", "Value for Money: Has the contractor shown a transparent and well-evidenced approach to securing value for money", "Contract Management: Has the contractor demonstrated effective contract management, including performance monitoring and oversight of subcontractors", "Project Management: Has the contractor provided evidence of their capability to self-deliver small and medium asbestos project works" — all at #/texts/5.
- Return-address anchor: "Please return the reference to: Procurement@bexley.gov.uk", "London Borough of Bexley, Procurement Team, Civic Offices, Bexleyheath, Broadway, Kent, DA6 7LB" — #/texts/5.
- Absence evidence (negative-space claims): no CPV token "[0-9]{8}", no "HSG", no "BS ", no "ISO ", no "EN ", no "UKAS", no "CAR 2012", no "licensable asbestos", no "£" unit prices, no quantity units ("m2", "unit", "per visit") found in the extracted markdown by inspection.
- Artifact refs produced by this run: workdir/result.json (validated against skills/pdf-extraction/tender-specification/result.schema.json), workdir/run-manifest.md (this file).
- Daemon appendix fields consumed: job_id, session_id, stage_run_id=stage-run-43d02438-92da-4df3-bc14-2ff454300565-proc-pc-dc1daee2-772e-f111-813c-005056b64545, execution_mode=contract, idempotency_key=extract_document_structure::job-extract-7465018e81d0a10c085d134e28bae0c1c682faf044306e0f11c171c1bb017ff9::pdf-extraction/tender-specification.
