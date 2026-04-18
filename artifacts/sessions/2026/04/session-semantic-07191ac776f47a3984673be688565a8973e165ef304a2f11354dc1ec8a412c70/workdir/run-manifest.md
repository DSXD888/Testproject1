---
manifest_version: 1.0.0
skill_id: pdf-extraction/tender-specification
skill_version: 1.0.0
job_id: job-b5db187e-3867-4eac-8118-9293b5bf2fa6
session_id: session-semantic-07191ac776f47a3984673be688565a8973e165ef304a2f11354dc1ec8a412c70
execution_mode: contract
started_at: 2026-04-18T10:18:00Z
completed_at: 2026-04-18T10:19:00Z
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

## Summary

Extracted one authoritative lot (`A.4` — CDM Principal Designer Scope of Service) from document_extraction `9091d878-c925-467b-9949-ede03cebdfe2`, the Annex A.4 of the multi-disciplinary design team tender pack for "Stockport Town Centre School" (notice `pc-f0f30f94-e22c-f111-813c-005056b64545`, process `proc-pc-f0f30f94-e22c-f111-813c-005056b64545`, stage_run `stage-run-43d02438-92da-4df3-bc14-2ff454300565-proc-pc-f0f30f94-e22c-f111-813c-005056b64545`).

The extraction is based on the full pre-normalized Docling markdown (17,578 bytes, sha256 `1af0c126962ab984003ef70e32648dc46f9e8b4e093de19d377340c5183f938c`) that the daemon stored in `document_extractions.extracted_markdown`. The document carries `document_role = technical_specification` and `expected_overlays_json = ["pdf-extraction/tender-specification"]`, so the single-overlay assumption in the bound SKILL holds.

Payload includes: 1 lot with 36 requirement bullets, 13 technical standards (all CDM 2015 regulation citations plus RIDDOR, SSIP/CHAS, BIM OIR/EIR, RIBA Plan of Work 2020 Stages 0–7, and CDE), 1 product_requirement seed representing the professional-services appointment, 13 evidence anchors, empty `cpv_codes` and empty `evaluation_criteria` with explanatory warnings, `extraction_confidence_bps = 7800`. The `extraction_metadata.target_domain = "lots"` and `extraction_scope = "full_document"` to match the skill contract and job request.

## Steps Taken

1. Resolved the job parameters (`document_extraction_id`, `process_id`, `notice_id`, `document_set_id`, `target_domain`, `extraction_scope`) against `state/state.db`.
2. Loaded `document_extractions` row `9091d878-c925-467b-9949-ede03cebdfe2`; verified `document_role = technical_specification`, `extraction_policy_version = 2026.04.12`, `expected_overlays_json = ["pdf-extraction/tender-specification"]`, `page_count = 0`, `section_count = 1`, `table_count = 1`, `quality_warnings_json = NULL`.
3. Dumped the extracted markdown to a working file and recomputed its sha256 (`1af0c126962ab984003ef70e32648dc46f9e8b4e093de19d377340c5183f938c`) for inclusion in `extraction_metadata.source_markdown_hash`.
4. Read the parent `contract_notices` row `pc-f0f30f94-e22c-f111-813c-005056b64545` and its `normalized_payload` for process context (buyer = Stockport Metropolitan Borough Council, title = "020417 - Stockport Town Centre School - Multi-disciplinary Design Team Appointment", 15 documents in the pack including 5 A.* scope annexes), and confirmed that notice-level lots for sibling annexes A.1 (Architect) and A.3 (MEP) already exist — this annex is A.4.
5. Enumerated the document structure from the markdown: cover block (Scope of Service / CDM Principal Designer / Project E000-25-0031 / Jan 2026 / Version Control table), §1 "General" (CDM 2015 introduction), §1 "Core Services" (0.x bullets with regulation citations), stage sections "Strategic Definition, Preparation and Briefing – RIBA Stages 0 & 1", "Concept Design – RIBA Stage 2", "Developed Design – RIBA Stage 3", "Technical Design – RIBA Stage 4", "Construction – RIBA Stage 5", "Handover and Close Out & In Use – RIBA Stage 6 and 7".
6. Built the `lots` array with a single lot (`lot_number = "A.4"`) matching the annex label used by the buyer. Requirements were preserved in source order with numeric list-prefixes normalized away (source reuses markers — see warning `SOURCE_NUMBERING_INCONSISTENT`).
7. Enumerated 13 `technical_standards`, each anchored to a specific sentence containing the regulation citation or standard reference. Statutory framework (CDM 2015) was emitted once and then individual regulation clusters that appear in the document (Regs 8(4)–8(6), 11(1)–11(3), 11(6)–11(7), 12(3)/12(5)/12(6)/12(8), 4[4], 6, 9(3)(b)) were emitted separately so downstream consumers can cite them individually.
8. Emitted one `product_requirement_seed` (`cdm-principal-designer-service-appointment`) and mirrored it into the lot's `product_requirements` list. Seed keeps `requirement_type = "professional_service"` because the document procures professional design services, not physical products.
9. Left `cpv_codes = []` and `evaluation_criteria = []` because the document does not cite any (warnings `NO_EXPLICIT_CPV`, `NO_EXPLICIT_EVALUATION_CRITERIA`).
10. Wrote `result.json` to the session workdir with `extraction_confidence_bps = 7800`, 6 warnings, 13 top-level evidence anchors, and the hash-bearing `extraction_metadata`. Validated the output against `skills/pdf-extraction/tender-specification/result.schema.json` using `jsonschema` Draft202012Validator — 0 errors.

## Decisions Made

- **Lot numbering as annex label (`A.4`).** The parent notice's existing `notice_lots` rows use annex labels (`lot-1` for A.1, `lot-a-3` for A.3). Using `"A.4"` preserves cross-annex consistency and is traceable to the document title ("A.4 CDM Principal Designer Scope of Service"). Warning `MULTI_PART_TENDER_PACK_CONTEXT` disambiguates this from a procurement-regulation "Lot".
- **No invented CPV codes.** Source text contains no CPV citations; the parent notice's `main_category = "150000 - Consultancy"` is a portal-specific code, not a CPV. CPV is left empty and warning `NO_EXPLICIT_CPV` names plausible downstream candidates (71317210, 71540000) without asserting them as evidence.
- **Evaluation criteria deferred.** The annex is a scope-of-service document, not the evaluation/award schedule. Criteria should come from `1A. Invitation to Participate` or `3. Response Document` (both listed in `normalized_payload.documents_downloaded`). Warning `NO_EXPLICIT_EVALUATION_CRITERIA` records this.
- **Single product seed.** The document procures one indivisible professional-services appointment with quantity "one appointment covering RIBA Stages 0–7". Exploding the scope into many service seeds would invent granularity the buyer did not commit to price separately ("these should be allowed priced within any tender submission").
- **Page references use `"page:1"`.** The extraction has `page_count = 0` and Docling exposes only one "page header" anchor, so real page numbers are unavailable. Anchor texts cite document sentences verbatim; section_ref values carry navigational meaning in place of page numbers. Warning `NO_PAGE_STRUCTURE` records this.
- **Confidence = 7800 bps.** Content extraction is high fidelity (requirements, standards are explicit and verbatim); the lot/pack structural interpretation carries the most residual uncertainty (annex-as-lot convention, missing CPV/evaluation criteria), driving the confidence below 8500.

## Warnings

1. `NO_EXPLICIT_CPV` — Source has no CPV codes. Parent notice's `main_category` is portal-specific, not CPV. CPV list emitted empty.
2. `NO_EXPLICIT_EVALUATION_CRITERIA` — Annex defines scope only; award criteria sit in sibling documents.
3. `MULTI_PART_TENDER_PACK_CONTEXT` — This document is annex A.4 of a 15-document pack. Sibling notice_lots already exist for A.1 and A.3; the daemon appears to treat each annex as a pseudo-lot, but A.4 is not a formal separately-awardable Lot in the procurement-regulation sense.
4. `NO_PAGE_STRUCTURE` — `page_count = 0` in `document_extractions`. Evidence `page_ref` values are approximated as `"page:1"` with descriptive `section_ref`.
5. `SOURCE_NUMBERING_INCONSISTENT` — Docling markdown reuses list markers (`0.x`, duplicate `1.`, Concept/Developed/Technical stages each restart at `1.`/`2.`/`3.`). Requirement order is preserved; numeric prefixes have been normalized away in the output.
6. `SECTION_9_NOT_PRESENT_IN_ANNEX` — Intro references "Section 9 … performance monitoring and key performance indicators" but Section 9 content is absent from this annex extraction.

## Evidence

- **document_extractions.id = `9091d878-c925-467b-9949-ede03cebdfe2`** — row in `state/state.db`. Fields: `document_role = "technical_specification"`, `document_title = "A.4 CDM Principal Designer Scope of Service (To be read)"`, `extraction_policy_version = "2026.04.12"`, `expected_overlays_json = ["pdf-extraction/tender-specification"]`, `source_content_hash = "aa5293f427ac8ffe049e42dc238e9f9ae0c4624a5cf3cba86957468df2cd3925"`, `page_count = 0`, `section_count = 1`, `table_count = 1`, `quality_warnings_json = NULL`.
- **Markdown digest** — `sha256(extracted_markdown) = 1af0c126962ab984003ef70e32648dc46f9e8b4e093de19d377340c5183f938c`, size 17,578 bytes. Persisted into `result.extraction_metadata.source_markdown_hash`.
- **Cover anchor** — "Scope of Service / CDM Principal Designer / Project: Stockport Town Centre School / Project Number: E000-25-0031 / Document Number: 00 / Revision: 00 / Date: January 2026" (markdown lines 3–12).
- **Lot anchor** — the lot's `evidence_anchor.anchor_text` cites the cover heading; the core-service pricing commitment cites "The appointed Principal Designer & Construction Health & Safety Advisor shall be expected to carry out all of the services required in Sections 2 to 8 and these should be allowed priced within any tender submission." (markdown §General).
- **Statutory framework anchor** — "The Construction (Design and Management) Regulations 2015 (CDM 2015) came into force on 6 April 2015." (§General). Individual regulation anchors (Regs 8(4), 11(1), 11(6), 12(3)/12(5), 4[4], 6, 9(3)(b)) are carried on each corresponding `technical_standards` entry.
- **SSIP/CHAS anchor** — "Parties providing Principal Designer services to the council must hold a SSIP accreditation such as CHAS or similar." (§Core Services, bullet 0.11).
- **BIM anchor** — "The Principal Designer is required to comply with all of the requirements of the Clients Organisational Information Requirements for BIM and Employers Information Requirements for BIM" (§Core Services, bullet 0.15).
- **HSE F10 / Regulation 6 anchor** — "Ensure that a copy of the notification of the Development under Regulation 6 (and any revised notification) is provided to the Principal Contractor" (§RIBA Stage 5 Construction, bullet 4.2); "Prepare for and on behalf of the Council the HSE notification document (F10) and update this document as required." (§Core Services, bullet 0.4 — second occurrence of the list-marker reset).
- **RIDDOR anchor** — "should an incident which falls under the RIDDOR regulations take place the supplier will conduct a full review of the activities on site and shall provide the Council with a report detailing the circumstances of the incident and provide recommended mitigations for future actions." (§Core Services, bullet 0.15 — last core-services bullet).
- **RIBA stage headings** — "Strategic Definition, Preparation and Briefing – RIBA Stages 0 & 1", "Concept Design – RIBA Stage 2", "Developed Design – RIBA Stage 3", "Technical Design – RIBA Stage 4", "Construction – RIBA Stage 5", "Handover and Close Out & In Use – RIBA Stage 6 and 7" — all present as bold headings in the markdown and carried onto the corresponding technical-standard and requirement entries via section_ref.
- **Parent-notice context** — `contract_notices.normalized_payload.documents_downloaded` lists the pack's 15 files, including sibling scope annexes A.1, A.2, A.3, A.5, plus the main Specification (2), Response Document (3), T&Cs (4), Glossary (5), Priority Cohort & Themes (6), Brief (7), Appendices.zip, Schedule of Accommodation spreadsheet, and Programme PDF. `notice_lots` already contains rows for A.1 and A.3 — the daemon's lotting strategy is consistent with emitting `A.4` here.
- **Result artifact** — `/Users/alinaaitova/Desktop/Testproject1/artifacts/sessions/2026/04/session-semantic-07191ac776f47a3984673be688565a8973e165ef304a2f11354dc1ec8a412c70/workdir/result.json` (written and validated against `skills/pdf-extraction/tender-specification/result.schema.json` with `jsonschema` Draft202012Validator — 0 errors).
