---
manifest_version: 1.0.0
skill_id: pdf-extraction/tender-specification
skill_version: 1.0.0
job_id: job-9afaec45-7773-4f22-8622-a1c66f5af97a
session_id: session-semantic-fa7dfc506fdf964b3754cffa5d7752a989f791a61c3342e03bfb254f3ef0369e
execution_mode: contract
started_at: 2026-04-17T20:28:00Z
completed_at: 2026-04-17T20:32:31Z
outcome_class: success_full
items_processed: 1
items_skipped: 0
items_failed: 0
warnings_count: 7
agent_claimed_pages_visited: 0
agent_claimed_source_url: https://procontract.due-north.com/RfxResponse/Index?rfxId=c3957abe-e22c-f111-813c-005056b64545
agent_claimed_fingerprint_state: not_applicable_offline_docling
agent_claimed_locale_state: en-GB
agent_claimed_currency_state: GBP
---

## Summary

Extracted semantic lot, evaluation criteria, and technical-standards payload from the Docling markdown for document_extraction ee0d9604-0754-46d0-8425-5281ddb123cf (raw_document
raw-pc-f0f30f94-...-0bc6d68016a2375b, file "1A-Invitation-to-Participate.docx") belonging to tender document set tds-pc-f0f30f94-e22c-f111-813c-005056b64545
(process proc-pc-f0f30f94-e22c-f111-813c-005056b64545, notice "020417 - Stockport Town Centre School - Multi-disciplinary Design Team Appointment").
The document is Stockport Council's Document 1A Invitation to Participate (ITP) instructions under the Procurement Act 2023 Competitive Flexible Procedure.
One implicit lot is emitted covering the full design-team appointment; twelve evaluation criteria (three PSQ Stage 1 quality questions, six ITT Stage 2 quality/Social Value
questions, one Price, plus the pass/fail gate and the 0-6 scoring methodology note) were anchored; ten technical/legal standards were enumerated. No CPV codes and
no product-level requirements are present in Document 1A (it is a services procurement), so cpv_codes and product_requirement_seeds are emitted empty with warnings.

## Steps Taken

1. Resolved inputs from the job envelope: document_extraction_id, raw_document_id, document_set_id, notice_id, process_id, stage_run_id, target_domain=lots, extraction_scope=full_document.
2. Read document_extractions row ee0d9604-... from /Users/alinaaitova/Desktop/Testproject1/state/state.db and confirmed source_content_hash 0927b761e52e0a485e59c418e4d0fc7fc7e0956b0058e6e0a69a13fa66e7f4cd, expected_overlays_json=["pdf-extraction/tender-specification"], extracted_markdown length 56562 bytes, section_count 6, page_count 0 (DOCX source).
3. Cross-checked raw_documents metadata — raw_document doc_role_hint recorded as "invitation_to_tender" while extracted document_role was overwritten to "technical_specification"; recorded this as a warning (DOCUMENT_ROLE_MISMATCH) and preserved the more accurate "invitation_to_tender" value in extraction_metadata.document_role.
4. Confirmed no pre-existing notice_lots rows exist for notice pc-f0f30f94-...; no existing lot metadata to cross-reference.
5. Read the full Docling markdown (428 lines, 56 KB) and scanned for: lot structure, procurement procedure, contract term and value, PSQ/ITT evaluation tables, scoring methodology, statutory references, submission rules, Social Value framework.
6. Identified a single implicit lot (no explicit lot numbering). Captured lot title from the document header and derived description from Sections 1.1–1.3.
7. Extracted the PSQ Stage 1 quality weighting table (Q19/Q20/Q21) and the Stage 2 ITT quality/price table (Quality 60% split 4×10% plus Social Value Quant/Qual 10% each, Price 40%). Added the pass/fail gate and the 0–6 scoring methodology as separate evaluation_criteria entries.
8. Extracted statutory/legal references: Procurement Act 2023, Procurement Regulations 2024, FOIA 2000, EIR 2004, TUPE 2006, Public Services (Social Value) Act 2012, Modern Slavery Act (implied by Modern Slavery clause), Electronic Signatures Regulations 2002, Competition Act 1998 / Procurement Act Schedules 6/7, National TOMs framework.
9. Emitted empty cpv_codes and product_requirement_seeds arrays with explanatory warnings. Document 1A contains no CPV codes and no product-level requirements — this is a professional-services design-team appointment.
10. Assembled 14 evidence_anchors (including the four used inline per lot/criterion/standard as $ref) covering header identity, procedure, contract term, value, evaluation tables, statutory basis, and submission format rules.
11. Wrote result.json to the session workdir and validated the file against /Users/alinaaitova/Desktop/Testproject1/skills/pdf-extraction/tender-specification/result.schema.json using jsonschema.validate — passed.
12. Wrote this run-manifest.md alongside result.json.

## Decisions Made

- Emit a single lot (lot_number = "1") rather than guessing multi-lot structure. The ITP narrative describes one unified "Services" appointment with no lot breakdown; inventing lots would violate the "do not guess lot boundaries without citing evidence anchors" rule in SKILL.md.
- Set extraction_metadata.document_role = "invitation_to_tender" (matching the raw_document doc_role_hint) rather than the extraction_engine's overridden "technical_specification", because the content is plainly the ITP instructions document; recorded the mismatch as DOCUMENT_ROLE_MISMATCH warning so downstream consumers can reconcile.
- Include the PSQ pass/fail gate and the 0–6 scoring methodology as evaluation_criteria entries. They materially control whether a supplier is evaluated at all, and the schema only requires {criterion, summary} per entry.
- Preserve both the PSQ Stage 1 and ITT Stage 2 weightings as separate criteria entries with weighting_text noting that Stage 2 values are indicative (the document explicitly reserves the right to amend Stage 2 criteria before ITT issue — captured as INDICATIVE_AWARD_WEIGHTINGS warning).
- Use "p.1" as page_ref for every evidence_anchor because the Docling-converted DOCX flattens all content to page 1 (page_start = page_end = 1 for every section). Logged PAGE_REF_FLATTENED warning rather than fabricating multi-page references.
- Keep product_requirement_seeds = [] (the top-level output) and each lot's product_requirements = []. The skill's Non-goals explicitly forbid emitting a separate "candidates" or "requirements" semantic row, and no product seeds are derivable from the ITP.
- Include National TOMs as a technical standard because it governs quantitative Social Value scoring and is referenced by name, even though it is a framework rather than a certification standard.
- Set extraction_confidence_bps = 7400 to reflect high confidence in procedural/evaluation content but reduced overall confidence because (a) the document is an ITP wrapper rather than the full technical specification (which lives in Document 2), (b) CPV codes are absent, and (c) the document_role mismatch exists.

## Warnings

1. NO_CPV_CODES_IN_DOCUMENT – Document 1A contains no inline CPV references; any CPV signals must come from the notice/process record or Document 2.
2. NO_PRODUCT_REQUIREMENTS_SERVICES_TENDER – This is a professional-services appointment; product_requirement_seeds is intentionally empty and must not be interpreted as a gap.
3. DOCUMENT_ROLE_MISMATCH – document_extractions.document_role is "technical_specification" but the content is the ITP; raw_document doc_role_hint is "invitation_to_tender" (the authoritative role).
4. SINGLE_IMPLICIT_LOT – No explicit lot numbering in the document; modelled as a single implicit lot.
5. INDICATIVE_AWARD_WEIGHTINGS – Section 1.7 explicitly marks Stage 2 award weightings as indicative; downstream consumers must not treat them as final.
6. PAGE_REF_FLATTENED – DOCX-sourced markdown is single-page in Docling; page_ref values are "p.1" placeholders.
7. EVALUATION_PANEL_COMPOSITION_UNCONFIRMED – The ITP names no specific evaluators; only a general panel composition is stated.

## Evidence

- Input envelope job params (daemon-supplied): request.document_extraction_id = ee0d9604-0754-46d0-8425-5281ddb123cf, request.raw_document_id = raw-pc-f0f30f94-e22c-f111-813c-005056b64545-0bc6d68016a2375b4c5e971e6aa63e5b3f9e79c22853eeb622c05daef135dc13, request.document_set_id = tds-pc-f0f30f94-e22c-f111-813c-005056b64545, request.target_domain = lots, request.extraction_scope = full_document, runner_meta.executor_id = local-daemon.
- Daemon appendix field (document_extractions.source_content_hash): 0927b761e52e0a485e59c418e4d0fc7fc7e0956b0058e6e0a69a13fa66e7f4cd – used verbatim as extraction_metadata.source_markdown_hash in result.json.
- Daemon appendix field (document_extractions.expected_overlays_json): ["pdf-extraction/tender-specification"] – matches the skill this session executed.
- Raw document storage_path: /Users/alinaaitova/Desktop/Testproject1/artifacts/downloads/procontract/783101/1A-Invitation-to-Participate.docx (filename "1A. Invitation to Participate and Instructions (To be read).docx", 272221 bytes, rfx_id c3957abe-e22c-f111-813c-005056b64545).
- Contract notice title (from contract_notices row id=pc-f0f30f94-e22c-f111-813c-005056b64545): "020417 - Stockport Town Centre School - Multi-disciplinary Design Team Appointment".
- Document header (Docling markdown, reported page 1): "DOCUMENT 1A: INVITATION TO PARTICIPATE ("ITP") INSTRUCTIONS / Stockport Council ("the Council") / Stockport Town Centre School – Multi – disciplinary Design Team Appointment / STAR Refence: 020417 / Chest Reference: DN811263". All lot-identity claims anchor to this header.
- Competitive Flexible Procedure table (Stages 1–3B) anchors the procedure description in lot.description and the pass/fail gate evaluation_criterion. Anchor text: "Return of PSQ (3A) response document including mandatory pass/fail criteria and scored selection questions."
- Agreement Term and Value anchors (Sections 1.1 / 1.3.1): "The Contract will be for a term of 48 months" and "The maximum estimated value of the Contract over its maximum possible term is £1,800,000.00 (exclusive of VAT)." Support the 48-month and £1.8M facts in lot.description.
- PSQ (Stage 1) weighting table (Section 1.6) anchors the three PSQ evaluation_criteria: Q19 Relevant Experience 35%, Q20 Workforce & Resource 35%, Q21 Funding Model 30%.
- 0–6 scoring methodology table (Section 1.6) anchors the scoring-methodology evaluation_criterion: "This score for any quality question will result in the full exclusion of the submission." appears for Unacceptable / Poor / Deficient.
- Tender Assessment Criteria table (Section 1.7) anchors the seven ITT Stage 2 evaluation_criteria (4 × Quality 10%, Social Value Quant 10%, Social Value Qual 10%, Price 40%) and the INDICATIVE_AWARD_WEIGHTINGS warning via "the information provided in relation to the Award Criteria is indicative only and the Council reserves the right to amend, refine or change the criteria prior to issuing the ITT."
- Legal/regulatory anchors: Procurement Act 2023 ("this procurement exercise is being undertaken in compliance with the Procurement Act 2023"), Procurement Regulations 2024 ("regulation 19(2)(d) of the Procurement Regulations 2024"), FOIA ("'Freedom of Information Act 2000' (FOIA)"), EIR ("'Environmental Regulations 2004 (EIR)"), TUPE ("Transfer of Undertakings (Protection of Employment) Regulations 2006, and as amended (\"TUPE\")"), Social Value Act 2012 ("our obligations under the Public Services (Social Value Act) 2012"), Electronic Signatures Regulations 2002, Competition Act 1998, National TOMs ("performance and evidence-based approach to Social Value based on the National TOMs").
- Submission rules anchors: "Applications must be submitted in Microsoft Word and Excel format only.", "All Applications must be in the English language.", "The Supplier's final tender submission must remain valid for acceptance for a period of 270 days from the date of its submission or until any procurement challenge/s have been resolved." Support requirements 2, 3, and 8 respectively in the lot.requirements list.
- Section listing (document_extractions.extracted_sections_json): all six parsed sections report page_start = page_end = 1, confirming the PAGE_REF_FLATTENED warning.
- Result-contract field (result.json): schema validation performed via jsonschema.validate against /Users/alinaaitova/Desktop/Testproject1/skills/pdf-extraction/tender-specification/result.schema.json – validated successfully.
- No external page visits or browser navigation performed (contract-mode run on pre-extracted Docling markdown); agent_claimed_pages_visited = 0 and agent_claimed_fingerprint_state = not_applicable_offline_docling reflect the offline nature of this semantic pass.
