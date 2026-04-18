---
manifest_version: 1
skill_id: pdf-extraction/tender-specification
skill_version: 1.0.0
job_id: job-508b3762-f314-4315-8ea6-6788bfab4152
session_id: session-semantic-1fd11fed3a228dfef3002331052c52559cac4a37f8c2ff7a397cfc2aedd5648a
execution_mode: contract
started_at: 2026-04-18T00:00:00Z
completed_at: 2026-04-18T00:00:00Z
outcome_class: success
items_processed: 1
items_skipped: 0
items_failed: 0
warnings_count: 6
agent_claimed_pages_visited: 1
agent_claimed_source_url: none
agent_claimed_fingerprint_state: not_applicable
agent_claimed_locale_state: en-GB
agent_claimed_currency_state: GBP
---

# Run Manifest — pdf-extraction/tender-specification

## Summary

Performed semantic extraction over the Docling-produced markdown for document_extraction_id `78a695f2-300b-4c19-af62-93057df2e5e6` (raw `raw-pc-dc1daee2-772e-f111-813c-005056b64545-249109b472…`), scoped to process `proc-pc-dc1daee2-772e-f111-813c-005056b64545` and notice `pc-dc1daee2-772e-f111-813c-005056b64545`. The source document is the Service Specification for Lot 3 ("Provision of Asbestos Surveys Services") of London Borough of Bexley Building Services Framework 6278. One authoritative lot record was emitted (`lot_number = "3"`), along with 9 product-requirement seeds covering the distinct service lines identified in Section 3 (Asbestos Management Consultancy, Asbestos Consultancy — Contractor Supervision, Ad-hoc Air Monitoring, Training) and Section 5 (Management Surveys, Refurbishment Surveys, Demolition Surveys, Sampling Services, Re-inspection/Monitoring Service). `extraction_confidence_bps = 8400`. The extraction has `target_domain = "lots"` and `extraction_scope = "full_document"`.

## Steps Taken

1. Read job parameters and bound skill contract from the execution context; confirmed `target_domain = "lots"`, `extraction_scope = "full_document"`, and the expected overlay set `["pdf-extraction/tender-specification"]`.
2. Loaded the extraction row from `state.db → document_extractions` by `id = "78a695f2-300b-4c19-af62-93057df2e5e6"`, including `extracted_markdown`, `extracted_sections_json` (44 section entries), and key metadata (title `"6278 - Draft specification - Lot 3"`, `document_role = "technical_specification"`, `extraction_policy_version = "2026.04.12"`, `page_count = 0`).
3. Cross-referenced the parent `notice_lots` row for `notice_id = pc-dc1daee2-772e-f111-813c-005056b64545`, lot 3 ("Provision of Asbestos Surveys Services (Lot 3 of Framework 6278)"), confirming that the document scope is a single lot (no contradiction detected between the notice's lot 3 and the document title `- Lot 3`).
4. Read the full 564-line extracted markdown and walked the 17 numbered sections:
   - §1 Introduction & Context → contract period `"3 + 2 years"` commencing April 2026.
   - §2 General → FM coordination with fire / M&E / building fabric suppliers; site security / key handling.
   - §3 Specification of Services → four service streams: Asbestos Management Consultancy, Asbestos Consultancy (contractor supervision), Ad-hoc Air Monitoring (5 monitoring sub-types), and Training.
   - §4 Working Methods & Practices → pre-clean, smoke test, removal, 4-stage clearance phases.
   - §5 Job Completion → four survey archetypes (Asbestos Surveys — General, Refurbishment, Demolition, Management) plus standalone Sampling Services.
   - §6 Documentation → HSG264 / HSG248 / HSG227 references, per-ACM field list, Material Assessment Algorithm.
   - §7 Health & Safety Compliance → HSWA 1974, RIDDOR, UKAS/ISO 17025, two-samples-per-surface for textured coatings, access rules, re-inspection service.
   - §§8–17 → TF Cloud CAFM integration, equipment, contract management, permits, structure & resourcing, sub-contractors, complaints, customer care, mobilisation, continuous improvement & innovation.
5. Extracted 31 lot-level requirements, 12 evaluation capability criteria (unweighted — weightings not present in this document), 9 product-requirement items (`lot3-management-survey`, `lot3-refurbishment-survey`, `lot3-demolition-survey`, `lot3-sampling-services`, `lot3-air-monitoring`, `lot3-re-inspection-monitoring`, `lot3-consultancy-contractor-supervision`, `lot3-consultancy-spec-and-tender-prep`, `lot3-training-and-advisory`), and 11 technical standards (Control of Asbestos Regulations 2012 (enforced 2025), HSG264 (2025 update), HSG248 (2022 amendment), HSG227, ISO 17025, UKAS Asbestos Surveys/Analyst, HSWA 1974, Working at Height Regulations, RIDDOR, Disability Discrimination and Equalities legislation).
6. Computed `source_markdown_hash` (sha256) of the `extracted_markdown` column: `1d4b450b6674fe43441e79224ea1d866f0312b90a3584f9519dcdf786a039c8e`.
7. Assembled the result payload per `result.schema.json`, anchoring every lot-level description line, evaluation criterion, product-requirement seed and technical standard to a section heading present in the Docling output.
8. Validated the assembled payload against `/Users/alinaaitova/Desktop/Testproject1/skills/pdf-extraction/tender-specification/result.schema.json` using `jsonschema` — passed.
9. Wrote `result.json` to the session workdir (`artifacts/sessions/2026/04/session-semantic-…/workdir/result.json`) so that the daemon can pick it up as the contract output.

## Decisions Made

- **Single-lot emission**: The document is titled `"6278 - Draft specification - Lot 3"` and its body consistently scopes services to the Council's asbestos service rather than other framework lots. The existing `notice_lots` row for lot 3 matches. Therefore one lot with `lot_number = "3"` was emitted, and a warning (`DOCUMENT_IS_SINGLE_LOT_SCOPE`) records that the other lots in Framework 6278 are out of scope of this extraction.
- **Unweighted evaluation criteria**: The document does not publish numeric evaluation weightings. Rather than fabricate scores, the 12 evaluation criteria are emitted as qualitative capability areas without `weighting_text`, and `EVALUATION_CRITERIA_NOT_WEIGHTED` is surfaced as a warning.
- **CPV codes**: No CPV references were found anywhere in the document body. `cpv_codes` is returned as `[]` at both the lot and seed levels, with `CPV_CODES_NOT_PRESENT_IN_DOCUMENT` raised as a warning. CPV must be sourced from notice-level metadata.
- **Page references**: The extraction row reports `page_count = 0` and every section in `extracted_sections_json` has `page_start = 1`. All `evidence_anchor.page_ref` values are therefore recorded as `"page 1"`, with section identity carried by `section_ref`. The `PAGE_REFS_UNAVAILABLE` warning documents this constraint to downstream consumers.
- **Product-requirement seed granularity**: The document describes discrete service lines (4 survey types + standalone sampling + air monitoring + re-inspection + two consultancy streams + training). These are emitted as 9 separate seeds so that downstream candidate resolution can match on a per-service basis, rather than bundling them into a single "Asbestos Surveys" super-seed.
- **Technical standards list**: Regulatory instruments with explicit version markers ("2025 update", "2022 amendment") are preserved verbatim as they appear in the document so that downstream classification skills can detect amendment year drift.

## Warnings

1. `EVALUATION_CRITERIA_NOT_WEIGHTED` — weightings not present in this document; expected in ITT (Schedule 3).
2. `CPV_CODES_NOT_PRESENT_IN_DOCUMENT` — no CPV codes cited; sourcing must be from notice metadata.
3. `CONTRACT_PERIOD_PARTIAL` — "3 + 2 years" stated; pricing and building scope live in Appendix B and Appendix 1 which are not embedded.
4. `PAGE_REFS_UNAVAILABLE` — Docling reports `page_count = 0`; all section references carry `page_start = 1`.
5. `DOCUMENT_IS_SINGLE_LOT_SCOPE` — this specification covers lot 3 only; it does not define or supersede lots 1, 2, 4–11 of Framework 6278.
6. `APPENDICES_REFERENCED_NOT_EMBEDDED` — Appendix B (Price Matrix), Appendix 1 (Building List), Appendix C (Environmental Policy), Appendix D (Code of Conduct) and Section 25 (Social Value) are referenced but not embedded in the extracted markdown.

## Evidence

- Document identity: `document_extractions.id = 78a695f2-300b-4c19-af62-93057df2e5e6`, `document_title = "6278 - Draft specification - Lot 3"`, `document_role = "technical_specification"`, `extraction_policy_version = "2026.04.12"`.
- Source markdown hash (sha256): `1d4b450b6674fe43441e79224ea1d866f0312b90a3584f9519dcdf786a039c8e`.
- Parent notice lot row: `notice_lots.lot_number = 3`, title `"Provision of Asbestos Surveys Services (Lot 3 of Framework 6278)"`, `notice_id = pc-dc1daee2-772e-f111-813c-005056b64545`.
- Section anchors used (all at page 1 per Docling):
  - `"Asbestos Specification"` — document title block.
  - `"The contract will be for a period of 3 + 2 years."` — §1 Introduction and Context.
  - `"Asbestos Management Services - Preparation of Specifications and Tender documents"` — §3 Specification of Services.
  - `"Asbestos Consultancy Services - Managing contractors undertaking Project Works including Supervision, Control and Planning"` — §3 Specification of Services.
  - `"Ad-hoc Air Monitoring Services"` — §3 Specification of Services.
  - `"Training"` — §3 Specification of Services.
  - `"Management Surveys"` / `"Refurbishment Surveys"` / `"Demolition Survey"` / `"Sampling Services"` — §5 Job Completion sub-sections.
  - `"All asbestos sampling must be analyzed by UKAS-accredited laboratories (ISO 17025)."` — §5 Job Completion / §6 Documentation.
  - `"HSE Guidance HSG264 (2025 update)"` / `"HSG248 (2022 amendment)"` / `"Priority Assessments where required must be conducted according to HSG 227."` — §6 Documentation.
  - `"Technology Forge (TF Cloud)"` and `"All invoices must be sent electronically to: fmcu.buildingservices@bexley.gov.uk"` — §8 Completing Jobs and Invoicing.
  - `"Re-inspections and Monitoring Service"` — §7 Health and Safety Compliance / Requirements.
  - `"Health and Safety at Work Act 1974"`, `"reportable under RIDDOR"`, `"the latest version of the Working at Height Regulations"`, `"Disability Discrimination and Equalities legislation"` — §7 Health and Safety Compliance / Legislation & Standards.
  - `"13 Sub-contractors"`, `"16. Mobilisation Work"`, `"17.Continuous Improvement & Innovation"` — §§13/16/17.
- Result contract artefact: `result.json` validated against `skills/pdf-extraction/tender-specification/result.schema.json` (passed). `lots = 1`, `product_requirement_seeds = 9`, `warnings = 6`, `evidence_anchors = 23`, `extraction_confidence_bps = 8400`.
