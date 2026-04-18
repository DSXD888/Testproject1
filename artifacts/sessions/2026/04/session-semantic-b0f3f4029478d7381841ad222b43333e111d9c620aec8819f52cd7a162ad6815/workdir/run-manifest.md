---
manifest_version: 1.0.0
skill_id: pdf-extraction/tender-specification
skill_version: 1.0.0
job_id: job-2897596c-675e-45fe-8370-305a21e8a200
session_id: session-semantic-b0f3f4029478d7381841ad222b43333e111d9c620aec8819f52cd7a162ad6815
execution_mode: contract
started_at: 2026-04-18T00:00:00Z
completed_at: 2026-04-18T00:00:00Z
outcome_class: partial_success_missing_subdocument
items_processed: 1
items_skipped: 0
items_failed: 0
warnings_count: 7
agent_claimed_pages_visited: 0
agent_claimed_source_url: https://procontract.due-north.com/RfxResponse/Index?rfxId=ea9d90b2-4833-f111-813c-005056b64545
agent_claimed_fingerprint_state: n/a
agent_claimed_locale_state: en-GB
agent_claimed_currency_state: GBP
---

## Summary

Ran `pdf-extraction/tender-specification` in headless contract mode against document_extraction `b8d95f0f-72d8-43f0-844a-f2e3568551ab`, which corresponds to raw document `raw-pc-dc1daee2-...-7881134be70e...` — the file `6278 - Draft T&CS - Lot 2.doc` for the London Borough of Bexley "6278 - Building Services Framework - Lots 1-11" notice (process `proc-pc-dc1daee2-772e-f111-813c-005056b64545`, document set `tds-pc-dc1daee2-772e-f111-813c-005056b64545`). Upstream docling (engine v2.88.0, processing strategy `standard`, page_count=0, 136,112 chars of markdown, source_content_hash `644a0672...74a`) was tagged `document_role=technical_specification` with expected overlay `pdf-extraction/tender-specification` and routed to this skill.

The document is in fact the draft Terms & Conditions of Contract for Lot 2 (Water Risk Assessments) — footer reads "FM Water Risk Assessment Services T&Cs 6 August 2020". The lot's authoritative Schedule 1 Specification, Schedule 2 Pricing Schedule, Schedule 3 ITT, and Schedule 4 Contractor's Tender appear only as empty placeholder headings at the end of the markdown; no body content is embedded. The tender-specification overlay nevertheless yields a single Lot 2 record with scope description, T&Cs-derived requirements, technical-standards references (generic BSI/ISO plus named statutes), and two conservative product-requirement seeds (the service itself and qualified assessor personnel). Evaluation criteria, specific water-hygiene standards (e.g. ACoP L8, HSG274, BS 8558), assessor qualification thresholds (Clause B4.2 references "paragraph 3.12 of the Specification" with a likely template-residual "fire" label), and CPV enumeration are not present in the document body and are either inherited from the notice or flagged as missing.

Overall extraction confidence is set to 5200 bps (52%), reflecting high confidence in document identity, buyer, lot subject and contract period (≥ 9500 bps) balanced against the absent Schedule bodies. Seven warnings are surfaced, the most load-bearing being `SCHEDULE_SPECIFICATION_MISSING`, `EVALUATION_CRITERIA_MISSING`, `PRICING_SCHEDULE_MISSING`, `TEMPLATE_RESIDUAL_FIRE_REFERENCE`, `DOCUMENT_ROLE_CLASSIFICATION_SUSPECT`, `PAGE_COUNT_UNKNOWN`, and `CPV_CODES_INHERITED_FROM_NOTICE`. Result contract `result.json` was written to the session workdir; schema validation against `skills/pdf-extraction/tender-specification/result.schema.json` passes. Outcome class is `partial_success_missing_subdocument` rather than `success` because the authoritative Specification subdocument (Schedule 1) is not carried by this raw document and would need to be ingested from the sibling `6278 - Draft specification - Lot 2.docx` before lot-scope extraction can be considered complete.

## Steps Taken

1. Read bound SKILL.md, result.schema.json and job parameters. Confirmed `target_domain=lots`, `extraction_scope=full_document`, required policy version `2026.04.12`.
2. Resolved identifiers against local state (`state/state.db`): looked up `document_extractions.id = b8d95f0f-72d8-43f0-844a-f2e3568551ab` to obtain extraction engine (docling 2.88.0), processing strategy (standard), source content hash (`644a0672…74a`), expected overlays (`["pdf-extraction/tender-specification"]`), policy version (`2026.04.12`), document_title (`6278 - Draft T&CS - Lot 2`), document_role (`technical_specification`), and page_count (0 — structural-only packet).
3. Exported the 136,112-character docling markdown to a scratch file for analysis (1,537 lines).
4. Cross-referenced `contract_notices.id = pc-dc1daee2-772e-f111-813c-005056b64545` to retrieve notice title, buyer, and CPV codes provided via `process_context` (50700000-2, 50750000-7, 71315000-9).
5. Confirmed raw-document origin: `raw_documents.source_url = https://procontract.due-north.com/RfxResponse/Index?rfxId=ea9d90b2-4833-f111-813c-005056b64545` and storage path `…/6278 - Draft T&CS - Lot 2.doc`.
6. Mapped the markdown: identified Part 1 Form of Contract, Part 2 General Terms & Conditions (Sections A-I), Part 3 Special Conditions (Section S), and Schedules 1-4 as empty placeholders.
7. Extracted lot scope: Recital A (line 43) establishes Lot 2 = Water Risk Assessments for Council Buildings, contract period 1 Sept 2020 – 31 Aug 2022 subject to extension.
8. Compiled T&Cs-derived requirements from Clauses B1-B8 (Supply, Quality, Staff, Key Personnel, Contract Manager, Best Value), C1-C3A (Payment, Service Credits), D5-D6 (Environmental, H&S), E1 (Data Protection), F3/F5/F7 (Variations, KPIs, Monitoring), G1 (Insurance), S1.6-S1.7 (TUPE).
9. Extracted technical-standards references: BSI + ISO (Clause B3.1/Definitions), Health and Safety at Work etc. Act 1974 (D6.7), British/European standards for chemicals with HSE/EA approval (D6.2), UK GDPR/Data Protection Act (E1.1), Local Government Act 1999 Best Value (B8.1), Protection of Freedoms Act 2012 / DBS regime (definitions + B4.11-B4.13), TUPE Regulations (S1.6-S1.7).
10. Identified the template-residual "fire risk assessor" wording in Clause B4.2 as a copy from a sibling lot and flagged it as a warning rather than treating it literally.
11. Generated two product-requirement seeds — a service seed (Water Risk Assessment deliverable) and a labour-capability seed (qualified assessor personnel) — each with stable `seed::bexley-6278::lot2::*` keys.
12. Determined that evaluation criteria, weightings, pricing structure, and specific water-hygiene standards are absent from the document body; recorded warnings rather than fabricating values.
13. Wrote `result.json` to the session workdir and validated it against `skills/pdf-extraction/tender-specification/result.schema.json` via `jsonschema` — schema validation passed.
14. Produced this run manifest with YAML frontmatter and required narrative sections.

## Decisions Made

- **Kept CPV codes attached to Lot 2 despite inheritance from the notice.** The three framework-level codes were present in `process_context` and are structurally the only CPV signal; I retained all three on the lot record but flagged `CPV_CODES_INHERITED_FROM_NOTICE` and noted that 50750000-7 (Lift-maintenance) almost certainly belongs to Lot 7, so downstream consumers can narrow. I did not invent a more specific CPV (e.g. 71621000-7 "Technical analysis or consultancy services" or 71317100-4 "Fire and explosion protection and control consultancy services") because no such code is cited in the document and the overlay forbids evidence-less claims.
- **Did not fabricate water-hygiene standards.** ACoP L8, HSG274, BS 8558:2015 and BS 7592 are the industry-standard references for UK water-risk assessments, but none appear in the document body. Emitting them would violate the skill's "do not claim a standard reference unless the evidence section cites the relevant anchor" rule, so I limited `technical_standards` to the generic Quality Standards definition plus statutes/instruments that ARE cited.
- **Treated the document as lot-scoped even though there is no "LOT 2" heading inside.** Lot identity is established unambiguously by (a) the Part 1 heading block "WATER RISK ASSESSMENTS" (line 17), (b) Recital A's scope language (line 43), (c) the footer "FM Water Risk Assessment Services T&Cs 6 August 2020" (line 1535), and (d) the notice summary's "Lot 2: Provider of Water Risk Assessments". Accordingly `lot_number = "2"` was chosen with confidence 9800 bps.
- **Chose `partial_success_missing_subdocument` as outcome class** rather than `success`. The skill produced a schema-valid payload with two seeds and eight technical-standards entries, but the authoritative Specification (Schedule 1) is not carried by this raw document. Downstream overlay pipelines routing Schedule 1's `.docx` sibling will supplement this record; flagging the partial state avoids silent data-thinning in the lot projection.
- **Recorded a dedicated `DOCUMENT_ROLE_CLASSIFICATION_SUSPECT` warning** rather than silently reclassifying. The docling extraction row has `document_role='technical_specification'`; the document itself is T&Cs. Role correction belongs to daemon-side reclassification logic, not to this skill's payload — so I surfaced it as a warning plus set `extraction_metadata.document_role = "terms_and_conditions"` for human review.
- **Used markdown line references (`md:line:<n>`) in `page_ref`.** Docling page_count was 0, so page numbers are not meaningful. Line references provide a reproducible anchor index into the stored `extracted_markdown`.
- **Kept `extraction_scope = "full_document"`** as specified in job parameters — the skill read and considered all 1,537 lines of the markdown, not a chunk-merged subset.

## Warnings

1. `SCHEDULE_SPECIFICATION_MISSING` — Schedule 1 body is absent; 18+ clauses cross-reference it, so lot scope cannot be fully grounded without the sibling spec doc. Evidence: md:line:1515 ("SCHEDULE 1"), md:line:1517 ("SPECIFICATION" with no body).
2. `EVALUATION_CRITERIA_MISSING` — Award criteria/weightings are in Schedule 3 (ITT), which is also an empty placeholder. A sentinel single-entry was placed in `evaluation_criteria` with confidence 1000 bps so consumers do not mistake absence for zero criteria.
3. `PRICING_SCHEDULE_MISSING` — Schedule 2 body is absent; pricing extraction must come from the separate Lot 2 pricing-schedule document.
4. `TEMPLATE_RESIDUAL_FIRE_REFERENCE` — Clause B4.2 still says "fire risk assessors / paragraph 3.12 of the Specification". For Lot 2 (Water) this is a template residue; treat as water-risk-assessor qualifications once Schedule 1 is available.
5. `DOCUMENT_ROLE_CLASSIFICATION_SUSPECT` — Upstream role=technical_specification is inconsistent with the document body, which is Terms & Conditions. Recommend daemon-side role correction to `terms_and_conditions`.
6. `PAGE_COUNT_UNKNOWN` — page_count=0 in docling output; anchors use md:line references instead of page refs.
7. `CPV_CODES_INHERITED_FROM_NOTICE` — The three CPV codes come from the framework notice, not from this document body; 50750000-7 almost certainly belongs to Lot 7, not Lot 2.

## Evidence

- `contract_notices.title` = "6278 - Building Services Framework - Lots 1-11" (source: state/state.db).
- `contract_notices.buyer_name` = "London Borough of Bexley".
- `contract_notices.cpv_codes` = "50700000-2 | 50750000-7 | 71315000-9" → passed through `process_context` to this run.
- `document_extractions.source_content_hash` = `644a0672b6d9ea2fc4dd76b3cc3a5180846e39a1f3890f3ff21b54fbbbf6e74a` → copied verbatim into `extraction_metadata.source_markdown_hash`.
- `document_extractions.extraction_policy_version` = `2026.04.12` → copied verbatim into `extraction_metadata.policy_version`.
- `document_extractions.expected_overlays_json` = `["pdf-extraction/tender-specification"]` → confirms overlay binding.
- `document_extractions.page_count` = 0 → drove the page-ref convention and warning 6.
- `raw_documents.source_url` = `https://procontract.due-north.com/RfxResponse/Index?rfxId=ea9d90b2-4833-f111-813c-005056b64545`.
- `raw_documents.storage_path` = `.../6278 - Adendum 1 - Draft Tender Documents (FOR INFORMATION ONLY)_extracted/Adendum 1 - Draft Tender Documents (FOR INFORMATION ONLY)/6278 - Draft T&CS - Lot 2.doc`.
- Lot subject / title anchor: md:line:17 "WATER RISK ASSESSMENTS" (Part 1 heading).
- Lot scope + period: md:line:43 Recital A ("Water Risk Assessments services in Council Buildings ... 1 September 2020 to 31 August 2022").
- Schedule precedence list: md:line:59-67 (Clause 1.2(a)-(h)).
- Quality Standards definition (BSI/ISO generic): md:line:433.
- Clause B1.1 supply obligation: md:line:535.
- Clause B3.1 Quality Standards compliance: md:line:563.
- Clause B4.2 template-residual "fire risk assessors" reference: md:line:571.
- Clause B4.11 DBS obligation: md:line:595.
- Clause D6.2 chemicals standards: md:line:789.
- Clause D6.7 Health and Safety at Work etc. Act 1974: md:line:799.
- Clause E1.1 Data Protection (Council=Controller, Contractor=Processor): md:line:807.
- Clause F7.1 Management Reports against KPIs: md:line:1195.
- Schedules list (Schedule 1 = Specification): md:line:1217.
- Empty Schedule placeholders: md:lines 1515, 1517, 1519, 1521, 1523, 1525, 1527, 1529.
- Document footer confirming T&Cs role + subject: md:line:1535 "FM Water Risk Assessment Services T&Cs 6 August 2020".
- Schema validation: `result.json` validated against `/Users/alinaaitova/Desktop/Testproject1/skills/pdf-extraction/tender-specification/result.schema.json` via Python `jsonschema.validate` — result: `schema valid`.
