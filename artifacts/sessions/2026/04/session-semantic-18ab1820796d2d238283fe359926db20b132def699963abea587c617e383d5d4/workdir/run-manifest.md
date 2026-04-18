---
manifest_version: 1
skill_id: pdf-extraction/tender-specification
skill_version: 1.0.0
job_id: job-2d1c2a58-60ee-4ff4-8afa-b1190d6e91d5
session_id: session-semantic-18ab1820796d2d238283fe359926db20b132def699963abea587c617e383d5d4
execution_mode: contract
started_at: 2026-04-18T00:00:00Z
completed_at: 2026-04-18T00:00:00Z
outcome_class: success_with_warnings
items_processed: 1
items_skipped: 0
items_failed: 0
warnings_count: 7
agent_claimed_pages_visited: 0
agent_claimed_source_url: null
agent_claimed_fingerprint_state: not_applicable
agent_claimed_locale_state: en-GB
agent_claimed_currency_state: GBP
---

## Summary

Extracted a single authoritative lot record (Lot 3 — Asbestos Surveys Services, Framework 6278 London Borough of Bexley) from the Docling markdown of `document_extraction_id = f6ae7a40-1035-41c2-ad5d-d1ec813c042f`. The document is the Draft Terms & Conditions for Lot 3 and embeds only the Form of Contract (Part 1), General Terms & Conditions (Part 2) and Special Conditions (Part 3). Schedule 1 (Specification), Schedule 2 (Pricing Schedule), Schedule 3 (the Council's ITT) and Schedule 4 (the Contractor's Tender) are present only as heading-only placeholders at the tail of the document and carry no substantive scope, KPI, evaluation criteria, surveyor qualification or pricing content. No product requirement seeds were emitted because this is a services contract and the Schedule 1 specification is not embedded. Overall extraction confidence is low (3200 bps) reflecting the template-only nature of the available content.

## Steps Taken

1. Located the Docling markdown for `document_extraction_id = f6ae7a40-1035-41c2-ad5d-d1ec813c042f` in `document_extractions.extracted_markdown` (state.db), 1535 lines, 135 751 bytes.
2. Verified document identity from the title block ("ASBESTOS SURVEYS SERVICES") and document_title row field "6278 - Draft T&Cs - Lot 3"; cross-checked against the parent notice `pc-dc1daee2-772e-f111-813c-005056b64545` (title "6278 - Building Services Framework - Lots 1-11", buyer "London Borough of Bexley") and notice_lots row for Lot 3 ("Provision of Asbestos Assessments"), confirming lot identity.
3. Walked the document structurally:
   - Part 1 Form of Contract (Recitals, precedence list, execution) — extracted buyer identity, recital A contract purpose and period 1 Sept 2020 – 31 Aug 2024.
   - Part 2 General Terms & Conditions (Sections A–I, Definitions, Clauses B1–S) — extracted high-level contractor obligations relevant to lot-level requirements.
   - Part 3 Special Conditions (S1 TUPE).
   - Tail pages — observed Schedule 1/2/3/4 headings with no body content.
4. Scanned for lot-specific facets: CPV codes, evaluation criteria, KPIs, surveyor qualifications, technical standards, product/equipment specifications. None of these are embedded; only references to "Appendix E to the Invitation to Tender" (KPIs, Service Credits), "Appendix D" (Code of Conduct), "Appendix C" (environmental policy) and "Schedule 1 Specification" were found.
5. Catalogued legal/regulatory standards that are explicitly cited in Parts 2–3 (BSI, ISO, Health and Safety at Work etc. Act 1974, Public Contracts Regulations 2015, Public Procurement (Electronic Invoices etc.) Regulations 2019, Equality Act 2010, Human Rights Act 1998, TUPE 2006, Data Protection Legislation, Directive 2014/24/EU). Emitted these in `technical_standards` with per-standard evidence anchors. Did not invent asbestos-specific standards (e.g. CAR 2012, HSG264, BOHS P402/P404, UKATA, UKAS) because they are not cited in the document body.
6. Decided against emitting CPV codes at lot level because the document does not contain lot-scoped CPV. The parent notice's three CPV descriptors span all 11 lots and none is uniquely tied to Lot 3 inside this document. Daemon may hydrate from notice-level data if required; a warning is raised.
7. Decided against emitting `product_requirement_seeds`. The contract is a services engagement, Clause B2 treats equipment as contractor-supplied-as-required, and Clause D6.2 treats chemicals/materials in open terms. Any itemisable product seeds would live in the non-embedded Schedule 1. A warning is raised.
8. Recorded six additional quality/diagnostic warnings (see Warnings section) including the stale "fire risk assessors" template artefact in Clause B4.2, the likely-stale 2020–2024 contract period, and the divergence from the pre-existing notice_lots title wording.
9. Wrote `result.json` to the session `workdir` and validated it against `/skills/pdf-extraction/tender-specification/result.schema.json` with `jsonschema` — validation passed.

## Decisions Made

- **Single lot record (Lot 3).** The document is explicitly the Draft T&Cs for Lot 3, not a multi-lot pack, so only one lot is emitted; pre-existing lots for Lots 1-2 and 4-11 in state.db were not re-extracted because they are out of scope for this extraction.
- **Empty `evaluation_criteria` rather than speculative criteria.** All evaluation content is referenced-only (Appendix E to the ITT) and not present in the markdown. Emitting a guessed price/quality split or fabricated KPI table would violate the "do not claim … without citing evidence anchors" non-goal.
- **Empty `cpv_codes` at lot level.** Parent-notice CPV codes are framework-wide and not document-level assertions; hydration is the daemon's concern. A warning documents the gap so the daemon can decide whether to inherit.
- **Empty `product_requirement_seeds`.** No itemised product/equipment specifications exist in this document; seeds would need to come from the un-embedded Schedule 1.
- **`document_role = technical_specification`** preserved from `document_extractions.document_role` even though the document is in practice a Draft T&Cs pack; this matches the existing daemon-side role hint and avoids silent role rewrites.
- **`source_markdown_hash` set to `document_extractions.source_content_hash` (947e5a63…)** which is the hash the daemon stores for this markdown; a locally computed shasum of an exported copy differs by only trailing whitespace and would not match the daemon's stored hash.
- **Confidence 3200 bps (low).** Identity and high-level requirements are well-evidenced (per-field confidences 7500–9500), but the absent Schedule 1, absent evaluation criteria and absent lot-level CPV drag overall confidence into the lower band. Per-field confidences for evaluation_criteria, cpv_codes and product_requirements are deliberately low (1500–2000) to surface the gap.

## Warnings

1. `SCHEDULE_1_SPECIFICATION_NOT_EMBEDDED` — Schedule 1 and Schedules 2–4 appear only as heading-only placeholders at the document tail; scope-defining content (surveyor qualifications, survey methodology, deliverables, KPI table, sampling regimes, asbestos-specific technical standards) is not present.
2. `EVALUATION_CRITERIA_NOT_EMBEDDED` — Award / evaluation criteria and KPI table are referenced as appendices to the ITT but the ITT is not embedded.
3. `STALE_TEMPLATE_TEXT_FIRE_RISK_ASSESSORS` — Clause B4.2 references "fire risk assessors" and "paragraph 3.12 of the Specification" — boilerplate copied from the Lot 1 template and not retargeted for asbestos.
4. `CONTRACT_PERIOD_LIKELY_STALE` — Recital A dates (1 Sept 2020 – 31 Aug 2024) are almost certainly the template's original 2020 dates and do not match the 2026 commencement implied by the parent notice; the document is labelled "Draft".
5. `CPV_CODES_NOT_ASSIGNED_PER_LOT` — No lot-specific CPV block in this document; the parent notice's three CPV descriptors span all 11 lots.
6. `NO_PRODUCT_REQUIREMENT_SEEDS` — Services contract with no itemised products in the embedded T&Cs; any seeds would come from the non-embedded Schedule 1.
7. `PRE_EXISTING_LOT_METADATA_DIVERGENCE` — This document's title is "ASBESTOS SURVEYS SERVICES" while the pre-existing notice_lots row describes Lot 3 as "Provision of Asbestos Assessments" at £60k/annum PSQ Group 1. Scope is consistent (asbestos surveying/assessment); the title-wording divergence is treated as template-stale rather than contradictory.

## Evidence

All extraction claims are anchored in the Docling markdown cited below. Anchors are quoted verbatim or quasi-verbatim from the source; line numbers refer to positions in the exported markdown stream.

- **Lot identity & scope** — Line 17 title block: "**ASBESTOS SURVEYS SERVICES**". Line 41 Recital A: "The Council wishes to secure the provision of Asbestos Surveys services in Council Buildings … during the period 1 September 2020 to 31 August 2024". Line 1533 footer: "**FM Asbestos Surveys Services T&amp;Cs 6 August 2020**". Document title column of `document_extractions` row: "6278 - Draft T&Cs - Lot 3".
- **Schedule precedence and non-embedded schedules** — Lines 53–65 (Clause 1.2) enumerate Schedule 1 (Specification), Schedule 2 (Pricing Schedule), Schedule 3 (the Council's ITT), Schedule 4 (the Contractor's Tender). Lines 1513–1525 show the schedule headings as placeholders with no content. Line 1215 (Clause F9.2 precedence list) restates the order.
- **Technical / quality standards** — Lines 431–453 (Definitions) cite BSI British Standards and ISO in "Quality Standards". Line 561 (B3.1) imposes compliance with Quality Standards and where applicable accreditation. Line 797 (D6.7) cites "Health and Safety at Work etc. Act 1974". Line 671 (C1.4) cites "Public Procurement (Electronic Invoices etc.) Regulations 2019 (2019/624)". Line 755 (D3.1) cites the Equality Act 2010 and Human Rights Act 1998. Lines 1499–1505 (S1.1–S1.2) cite TUPE 2006. Lines 805–821 (E1.1) cite Data Protection Legislation. Line 1405 (H2A) cites "Public Contracts Directive (Directive 2014/24/EU)" and TFEU Article 258. Line 1403 (H2A) cites regulation 57 of the Public Contracts Regulations 2015. Line 787 (D6.2) cites "relevant British or European standard" and HSE/Environment Agency approval for chemicals.
- **KPI / evaluation references (not embedded)** — Line 385 (definition of KPIs): "means the key performance indicators included in Appendix E to the Invitation to Tender". Line 1163 (F5.1): "Where any part of the Services is stated in Appendix E to the Invitation to Tender to be the subject of a specific KPI". Line 533 (B1.1) cites Appendix D (Code of Conduct); Line 773 (D5.1) cites Appendix C (environmental policy).
- **Stale template artefact** — Line 569 (B4.2): "In particular Staff who are fire risk assessors should hold the qualifications set out in paragraph 3.12 of the Specification".
- **Cross-reference (daemon appendix / state store)** — `document_extractions.source_content_hash` = `947e5a63a1332acc09a25d7cf8ab354c1deb7445749dd5214d42afa5d3690b93` (used as `source_markdown_hash`). `document_extractions.document_role` = `technical_specification`. `document_extractions.extraction_policy_version` = `2026.04.12`. `contract_notices` row for `pc-dc1daee2-772e-f111-813c-005056b64545` provides buyer `London Borough of Bexley` and framework-wide CPV descriptors 50700000-2 / 50750000-7 / 71315000-9. `notice_lots` row `pc-dc1daee2-772e-f111-813c-005056b64545-lot-3` records the pre-existing lot as "Provision of Asbestos Assessments" (£60k per annum, PSQ Group 1) — cited only for cross-reference, not treated as authoritative over the document.
- **Schema validation** — `result.json` validated against `/skills/pdf-extraction/tender-specification/result.schema.json` with `jsonschema` (Python); result: `OK`.
