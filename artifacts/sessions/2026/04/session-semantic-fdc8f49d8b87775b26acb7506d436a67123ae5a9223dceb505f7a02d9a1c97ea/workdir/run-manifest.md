---
manifest_version: 1
skill_id: pdf-extraction/tender-specification
skill_version: 1.0.0
job_id: job-da70aff0-4ede-4eba-9caa-318d4dc6661c
session_id: session-semantic-fdc8f49d8b87775b26acb7506d436a67123ae5a9223dceb505f7a02d9a1c97ea
execution_mode: contract
started_at: 2026-04-18T00:00:00Z
completed_at: 2026-04-18T00:00:00Z
outcome_class: success_partial
items_processed: 1
items_skipped: 0
items_failed: 0
warnings_count: 9
agent_claimed_pages_visited: 0
agent_claimed_source_url: null
agent_claimed_fingerprint_state: not_applicable
agent_claimed_locale_state: en-GB
agent_claimed_currency_state: GBP
---

# Run Manifest — pdf-extraction/tender-specification

## Summary

Extracted Lot 3 "Provision of Asbestos Surveys Services" from the Bexley Framework 6278 draft Terms & Conditions document (document_extractions.id = 5fb2697d-7184-485b-ad5a-fb1be202b859, raw_document_id = raw-pc-dc1daee2-772e-f111-813c-005056b64545-9b83c49c10cb93b5b48b3ee994d65ae0e7856bbb280d20f0dfabfda08ae0d519). The Docling markdown covers Part 1 Form of Contract, Part 2 General Terms & Conditions (clauses A–I), Part 3 Special Conditions (TUPE) and the Schedule 1–4 header placeholders only. One authoritative lot record was emitted along with one coarse product_requirement_seed. The document is a single-lot T&Cs template for Lot 3 of an 11-lot framework, so no other lot entries were fabricated.

Outcome is `success_partial` because the substantive specification content (Schedule 1), the pricing schedule (Schedule 2), the ITT with Appendix D Code of Conduct and Appendix E KPI/Service-Credit table (Schedule 3), and the Contractor's tender (Schedule 4) are referenced throughout the clauses but are absent from the extracted markdown body. Nine warnings were raised to flag missing schedules, stale template dates, cross-lot template residue, and the fact that Docling produced 0 sections / 0 tables / 0 pages (so page_refs use clause numbers and approximate line anchors rather than true pagination).

## Steps Taken

1. Resolved execution context from the job parameters: target_domain = lots, extraction_scope = full_document, document_extraction_id = 5fb2697d-..., notice_id = pc-dc1daee2-..., stage_run_id = stage-run-43d02438-...
2. Read the Docling-extracted markdown from state.db → document_extractions.extracted_markdown for the specified document_extraction_id (135 751 bytes, 1 535 lines).
3. Checked companion structural fields: extracted_sections_json = `[]`, extracted_tables_json = `[]`, page_count = 0, table_count = 0, section_count = 0 — warning NO_STRUCTURAL_TABLES_EXTRACTED raised because evidence anchors cannot use real section IDs or page numbers.
4. Cross-referenced the parent notice: contract_notices.title = "6278 - Building Services Framework - Lots 1-11"; 11 lot rows already exist in notice_lots for this notice (Lots 1–11). The document body, however, scopes Lot 3 only — warning SINGLE_LOT_DOCUMENT raised.
5. Scanned the body for lot identity evidence: Part 1 title block, Recital A, Clauses 1.2(d)–(g) and the Schedule 1 placeholder all corroborate "Lot 3 — Provision of Asbestos Surveys Services" as the sole in-scope lot.
6. Extracted lot-level `requirements` from Part 2 clauses B1.1, B3.1, B4.2, B4.9–B4.14, D6.2, D6.7, G1.7, G2, F5.3, F7.1–F7.2, E1.1, E1.15, E1A.4, E1A.9, C2.6, A2, C1.1, C4.1 — each anchored to a clause reference in the extracted markdown.
7. Extracted `evaluation_criteria` from Clause F5 (KPIs), Clause C3A (Service Credits), and Clause F5.5 (Default Notice remedy). Weightings were not stated in the body because they live in Appendix E to the ITT (missing) — captured in `weighting_text` = "Not stated in this document; Appendix E to ITT not present."
8. Extracted `technical_standards` from Clause A1.1 ("Quality Standards" definition → BSI, ISO) and Clause D6.7 (Health and Safety at Work etc. Act 1974). Specific asbestos technical standards (HSG264, UKAS ISO/IEC 17020/17025, BOHS P402/P404/P405, CAR 2012) were NOT asserted because none appear in the extracted body — warning SPECIFICATION_SCHEDULE_MISSING raised instead.
9. Did not populate `cpv_codes` on the lot. No CPV codes appear in the body of the draft T&Cs. Notice-level CPVs (50700000-2, 50750000-7, 71315000-9) are framework-wide, not lot-specific, and do not describe asbestos work — warning CPV_NOT_STATED_IN_DOCUMENT_BODY raised; left the array empty rather than fabricate.
10. Built one `product_requirement_seeds` entry (seed_key = `bexley-6278-lot3-asbestos-surveys`) tied to lot_number "3", summarising scope from Recital A and Clauses B1.1/B3.1/B4.2/G1.7. Did not invent per-survey-type seeds (management survey / refurbishment & demolition survey / re-inspection survey etc.) because the Schedule 1 Specification that would enumerate them is missing from the body.
11. Detected stale template dates: Recital A and Clause 1.3 reference a period of 1 September 2020 – 31 August 2024, whereas the parent notice deadline_at is 2026-05-18. Warning TEMPLATE_DATES_STALE_RELATIVE_TO_NOTICE raised.
12. Detected cross-lot template residue in Clause B4.2 (reference to "fire risk assessors" and "paragraph 3.12 of the Specification"). This belongs to Lot 1 Fire Risk Assessments, not Lot 3. Warning TEMPLATE_CROSS_LOT_RESIDUE raised so downstream pipeline does not mis-derive a fire-assessor requirement.
13. Computed source_markdown_hash by hashing the extracted_markdown column (`sha256:85d4c518c3cb852d8eecc8d50f6a9f60aa460cf56390ff37f3ef734e528688f5`). Set extraction_metadata.target_domain = "lots", extraction_scope = "full_document", policy_version = "2026.04.12", document_role = "technical_specification" (matches document_extractions.document_role).
14. Validated `result.json` against `/Users/alinaaitova/Desktop/Testproject1/skills/pdf-extraction/tender-specification/result.schema.json` using jsonschema.validate — passed.

## Decisions Made

- **Emitted one lot, not eleven.** This document is scoped explicitly to Lot 3 (Part 1 title block, Recital A, all clause references). The 11-lot notice enumeration is authoritative only for the parent notice_lots table, not for this document's semantic extraction. Pre-existing notice_lots rows were used for cross-reference only, per the skill's input rules.
- **Left `cpv_codes` empty on the lot.** The body contains no CPV references. Fabricating asbestos CPVs (90650000-8, 71317100-4) without textual evidence would violate the skill's evidence-grounding rule. Warning raised so downstream graders understand the gap.
- **Did not enumerate HSG264 / UKAS accreditation / BOHS P-series competencies.** Although these are the typical asbestos-survey technical standards, NONE of them appear in the extracted markdown. The Specification that would list them (Schedule 1) is a placeholder. Emitting them would be an evidence-free hallucination. Warning SPECIFICATION_SCHEDULE_MISSING captures this gap and recommends re-extraction against the actual Specification document when available.
- **Captured the Clause B4.2 fire-assessor reference as template residue, not as a Lot 3 requirement.** This protects downstream product_candidate derivation from creating a spurious fire-risk-assessor requirement on an asbestos lot.
- **Used clause numbers for `page_ref`.** Docling returned 0 pages / 0 sections / 0 tables on this document (likely a DOCX source or pagination stripped during conversion). Using clause refs (`Part 2 §B3.1`) plus approximate markdown line numbers is the best-available anchoring. Flagged with warning NO_STRUCTURAL_TABLES_EXTRACTED.
- **Set `outcome_class = success_partial`.** The document yielded a valid, evidence-grounded lot record, but 4 of 4 referenced schedules are placeholder-only, which limits downstream usefulness. Reporting partial success is more accurate than either success or failure.
- **Set `extraction_confidence_bps = 6800`.** Lot identity is high confidence (~95 %); clause-level requirements and insurance/DBS/H&S obligations are high confidence; evaluation criteria / CPV / detailed product requirements are low-to-medium because their anchoring documents are missing. 68 % is a conservative aggregate.
- **Extraction_scope is `full_document`.** The full Docling markdown was in scope; no chunking was performed; no plan_hash is required.

## Warnings

Nine warnings emitted (see `result.json.warnings[]`):

1. `SPECIFICATION_SCHEDULE_MISSING` — Schedule 1 (Specification) referenced 9+ times, body is placeholder only.
2. `PRICING_SCHEDULE_MISSING` — Schedule 2 (Pricing Schedule) is header-only.
3. `ITT_SCHEDULE_MISSING` — Schedule 3 ITT (and Appendices D Code of Conduct, E KPIs/Service Credits) is header-only.
4. `CONTRACTOR_TENDER_SCHEDULE_MISSING` — Schedule 4 (Contractor's Tender) is a draft placeholder.
5. `CPV_NOT_STATED_IN_DOCUMENT_BODY` — No CPV codes in the body; lot `cpv_codes` deliberately empty.
6. `TEMPLATE_DATES_STALE_RELATIVE_TO_NOTICE` — Period 2020-09-01 → 2024-08-31 is template residue; notice deadline is 2026-05-18.
7. `TEMPLATE_CROSS_LOT_RESIDUE` — Clause B4.2 fire-assessor reference belongs to Lot 1, not Lot 3.
8. `NO_STRUCTURAL_TABLES_EXTRACTED` — Docling produced 0 pages / 0 sections / 0 tables; evidence anchors use clause refs.
9. `SINGLE_LOT_DOCUMENT` — This document covers Lot 3 only; other 10 lots must be extracted from their own per-lot documents.

## Evidence

Every `lots[].requirements` entry, every `lots[].evaluation_criteria[]` entry, every `lots[].technical_standards[]` entry, and the single `product_requirement_seeds[]` entry cite a clause-level anchor from the extracted markdown. Representative anchors:

- **Lot identity** — Part 1 title block (lines 1–17): `RELATING TO THE PROVISION OF ASBESTOS SURVEYS SERVICES`; Recital A (line ~41): `The Council wishes to secure the provision of Asbestos Surveys services in Council Buildings ... during the period 1 September 2020 to 31 August 2024`.
- **Contract documents and precedence** — Part 1 §1.2 (lines 53–65): enumerates the Schedule 1 Specification, Schedule 2 Pricing Schedule, Schedule 3 ITT and Schedule 4 Contractor's Tender.
- **Quality Standards obligation** — Part 2 §B3.1 (line ~561): `The Contractor shall at all times comply with the Quality Standards, and where applicable shall maintain accreditation with the relevant Quality Standards authorisation body.`
- **Staff qualifications & DBS** — Part 2 §B4.2 (line ~569); §B4.9 (line ~589); §B4.11 (line ~593).
- **Chemicals / H&S** — Part 2 §D6.2 (line ~787); §D6.7 (line ~797).
- **Insurance minima** — Part 2 §G1.7 (line ~1291): `£5 million ... employer's liability ... £5 million ... professional indemnity ... £2 million`; §G2 (line ~1299): professional indemnity limit £2 000 000, maintained 6 years post-termination.
- **KPIs / Service Credits / Default remedy** — Part 2 §F5 (lines 1163–1175); §C3A (line ~703).
- **Monitoring / Management Reports** — Part 2 §F7 (lines 1193–1195).
- **Missing schedules** — Schedules 1–4 placeholders (lines 1513–1527): header-only entries confirming the referenced schedules are not populated in this extract.
- **Quality Standards definition** — Part 2 §A1.1: lists BSI British Standards Institution and International Organisation for Standardisation as the two reputable-body backbones for "Quality Standards", which is the textual basis for the two generic `technical_standards` entries.

Source markdown SHA-256: `85d4c518c3cb852d8eecc8d50f6a9f60aa460cf56390ff37f3ef734e528688f5`. Result file: `result.json` (schema-validated against `skills/pdf-extraction/tender-specification/result.schema.json` at policy version `2026.04.12`).
