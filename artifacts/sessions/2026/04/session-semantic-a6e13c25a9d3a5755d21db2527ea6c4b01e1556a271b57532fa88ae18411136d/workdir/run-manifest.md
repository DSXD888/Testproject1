---
manifest_version: 1.0.0
skill_id: pdf-extraction/tender-specification
skill_version: 1.0.0
job_id: job-509a88bb-bba3-456f-abd8-b75eb9a62aa4
session_id: session-semantic-a6e13c25a9d3a5755d21db2527ea6c4b01e1556a271b57532fa88ae18411136d
execution_mode: contract
started_at: 2026-04-18T04:52:00Z
completed_at: 2026-04-18T04:53:00Z
outcome_class: success_with_warnings
items_processed: 1
items_skipped: 0
items_failed: 0
warnings_count: 6
agent_claimed_pages_visited: 63
agent_claimed_source_url: local:document_extractions/466caf74-b0de-452d-aa1a-b23c1ae72c75
agent_claimed_fingerprint_state: n/a_headless_contract
agent_claimed_locale_state: en-GB
agent_claimed_currency_state: GBP
---

## Summary

Extracted lot-level semantic metadata from a Docling-processed tender specification document titled "6278 - Draft T&Cs - Lot 7" (process `proc-pc-dc1daee2-772e-f111-813c-005056b64545`, London Borough of Bexley Building Services Framework 6278). The document is a Draft Terms & Conditions template for a single lot — Lot 7, Lift Servicing, Maintenance, Repair, Minor Refurbishment and Compliance Management — comprising Part 1 Form of Contract, Part 2 General Terms & Conditions and Part 3 Special Terms & Conditions, with Schedules 1-4 (Specification, Pricing, ITT, Contractor's Tender) appended only as placeholder headings.

One lot entry was emitted with 22 contractual/operational requirements, 4 product-requirement seeds covering lift servicing/maintenance, reactive repair, minor refurbishment and compliance management, and 15 technical/regulatory standards (BSI British Standards, CDM Regulations, Health and Safety at Work etc. Act 1974, TUPE 2006, Data Protection Legislation, Safeguarding Vulnerable Groups Act 2006, DBS requirements, Late Payment of Commercial Debts (Interest) Act 1998, Public Procurement (Electronic Invoices etc.) Regulations 2019, Local Government Act 1999 Best Value duty, Scheme for Construction Contracts SI 1998/649 as amended, CIMAR Rules, BSI Group standards/European directive, BCO Good Practice in Selection of Construction Materials, Rehabilitation of Offenders Act 1974 (Exceptions) Order 1975). No CPV codes or award/evaluation criteria were present in this draft template; six warnings document these gaps and the empty state of Schedules 1-4. Extraction confidence: 6800 bps — high on lot identity/title/standards, low on evaluation criteria and CPV given their absence from the document.

## Steps Taken

1. Resolved execution context from job parameters: `document_extraction_id = 466caf74-b0de-452d-aa1a-b23c1ae72c75`, `process_id = proc-pc-dc1daee2-772e-f111-813c-005056b64545`, `extraction_scope = full_document`, `target_domain = lots`.
2. Loaded the Docling extraction from `state.db.document_extractions` — 63 pages, 99 extracted sections, 0 structured tables, title "6278 - Draft T&Cs - Lot 7", document_role "technical_specification", source_content_hash `0e177f5c9186a8e54fef7794d36b0d3c04a9e887915793b56cc25e98bed70247`.
3. Cross-referenced notice-level lot metadata for process `proc-pc-dc1daee2-772e-f111-813c-005056b64545` (11 lots in framework 6278); identified that the document body itself is scoped to Lot 7 only (Form of Contract heading "RELATING TO THE PROVISION OF LIFT SERVICES").
4. Walked the markdown top-to-bottom identifying Part 1 (Form of Contract, p2), Part 2 (General Terms & Conditions, p5, sections A1-I2), Part 3 (Special Terms & Conditions, p54, sections S1-S10), and Schedules 1-4 (pp60-63, placeholder headings only).
5. Extracted 22 requirements from Parts 1-3 covering service delivery, staffing, DBS, CDM, TUPE, data protection, payment, electronic invoicing, KPIs/Service Credits, dispute resolution and Best Value.
6. Derived 4 product-requirement seeds from the lift-services scope: planned maintenance, reactive repair, minor refurbishment (Works Orders / CDM-scoped), and compliance management.
7. Catalogued 15 explicit technical/regulatory standards with direct textual anchors (section IDs B3, B4.10, B4.11, B4.17, B8.1, C1.4, C2.9, D5, E1.1, S1, S2.2.2, S2.3, S2.6.2, S2.6.3, S3.5, S5.1, S10.1, S10.6).
8. Recorded absence of evaluation criteria and CPV codes as warnings rather than fabricating values. Recorded placeholder state of Schedules 1-4.
9. Assigned field confidences: lot_number/title (9500/9200 bps — document title + Form of Contract heading), description/requirements (8500/7800 bps — extensive textual evidence), evaluation_criteria/cpv_codes (2000/1000 bps — absent from document), technical_standards (8500 bps — explicit references), product_requirements (6500 bps — derived from lift-services scope, not populated Schedule 1).
10. Computed overall extraction_confidence_bps = 6800 reflecting strong lot identity + standards extraction against absent Schedule 1/2/3/4 content.
11. Emitted `result.json` matching `result.schema.json` and validated the envelope with jsonschema prior to writing the manifest.

## Decisions Made

- **Single lot, not multi-lot.** Although notice-level metadata lists 11 lots for framework 6278, this document is scoped to Lot 7 only. The document title "6278 - Draft T&Cs - Lot 7" and Form of Contract heading "RELATING TO THE PROVISION OF LIFT SERVICES" make this unambiguous. Emitting additional lot rows for 1-6 and 8-11 would fabricate evidence the document does not carry.
- **No evaluation criteria, no CPV codes.** Draft T&Cs templates in Bexley's framework defer evaluation methodology and CPV classification to Schedule 3 (Council's ITT) and the contract notice respectively. Neither is populated in this document. Both fields are emitted as empty arrays with explanatory warnings rather than guessed from lot scope.
- **Product-requirement seeds derived from lift-services scope, not from Schedule 1.** Since Schedule 1 (Specification) is an empty placeholder here, the 4 seeds describe the service categories inferable from the Form of Contract title and operational obligations in Parts 2-3 (S2.4 servicing Plant, S2.3 CDM duties, S3 compliance, B2/S9 equipment). Each seed carries its own evidence anchor; none claim attributes beyond what the document substantiates.
- **KPIs/Service Credits not treated as evaluation criteria.** Clause F5 references KPIs and remedies, and C2.2/C3A reference Service Credits — these are post-award performance mechanisms, not award-stage evaluation criteria. They are captured within lot-level requirements, not the evaluation_criteria array.
- **No pre-existing notice-level lot metadata overridden.** The SKILL instruction explicitly says "do not treat pre-existing lot rows as authoritative truth if the tender document contradicts them." The document does not contradict the notice-level title for Lot 7 (they are consistent); no override was required.
- **Outcome class `success_with_warnings`.** Extraction succeeded for all inferable fields; the warnings describe legitimate absences in the source document (empty schedules, no CPV, no evaluation criteria), not extraction failures.

## Warnings

- `DRAFT_TEMPLATE_SCHEDULES_EMPTY` — Schedules 1-4 appear only as placeholder headings (pp60-63, sections `#/texts/897`, `#/texts/900`, `#/texts/903`, `#/texts/906`). Product scope, pricing, ITT content and tender response are not derivable from this draft. Downstream pipeline should source those from related documents in `document_set_id = tds-pc-dc1daee2-772e-f111-813c-005056b64545`.
- `NO_EVALUATION_CRITERIA_FOUND` — No award criteria or weightings in the T&Cs body. F5 (KPIs) exists but is performance-phase, not award-phase.
- `NO_CPV_CODES_IN_DOCUMENT` — No CPV references; classification should come from the notice record.
- `NO_STRUCTURED_TABLES_DETECTED` — Docling quality warning ("No structured tables were detected in the exported document") preserved; any pricing or KPI tables that may exist in the original PDF are not structured.
- `CROSS_REFERENCE_NOTICE_LOTS_OK` — Document-claimed Lot 7 identity and title match the pre-existing `notice_lots` row for this process; no contradiction; notice metadata treated only as corroborating context.
- `LOT_SCOPE_DOCUMENT` — Single-lot contract template; other framework 6278 lots are out of scope for this extraction.

## Evidence

All major claims in `result.json` and this manifest are anchored to explicit sections in the Docling output.

- **Lot identity (Lot 7, Lift Services)** — Form of Contract title "RELATING TO THE PROVISION OF LIFT SERVICES" at `#/texts/1` on page 1; document title "6278 - Draft T&Cs - Lot 7" in `document_extractions.document_title`.
- **Document structure** — `PART 1. FORM OF CONTRACT` (`#/texts/11`, p2), `PART 2 - GENERAL TERMS AND CONDITIONS` (`#/texts/69`, p5), `PART 3. SPECIAL TERMS AND CONDITIONS` (`#/texts/805`, p54).
- **Service scope** — `B1 The Services` (`#/texts/223`, p15); `S2. CONTRACTOR'S OBLIGATIONS` (`#/texts/814`, p54) including S2.1 (Working Hours + Specification), S2.3 (CDM principal designer/contractor), S2.4 (servicing Plant per manufacturer recommendations), S2.6 (prohibited materials: deleterious, BSI-contravening, non-BCO-compliant).
- **Quality & standards** — `B3 Quality Standards` (`#/texts/239`, p16); `S5. GOODS AND MATERIALS` S5.1 (`#/texts/859`, p57: "comply with the latest applicable British Standard").
- **Staffing / DBS** — B4.2 (qualifications), B4.11 (DBS registration + 3-yearly rechecks), B4.17-B4.18 (Safeguarding Vulnerable Groups Act 2006).
- **Health & safety** — `D6 Health and Safety` (p26); `S3. HEALTH AND SAFETY` S3.1 (hazard notification), S3.3 (incident notification), S3.4 (COSHH register), S3.5 (HASAWA 1974 health & safety policy statement).
- **Data protection** — `E1 Data Protection` E1.1 (`#/texts/377`, p32: Council=Controller, Contractor=Processor under Data Protection Legislation); Protective Measures definition in A1.
- **TUPE** — `S1 TRANSFER OF UNDERTAKINGS ('TUPE')` (`#/texts/805`, p54), S1.1-S1.7.
- **Payment mechanics** — C1.1 (fixed Contract Price), C1.4 (Public Procurement Electronic Invoices Regulations 2019/624), C2.1 (30-day payment), C2.2 (monthly invoicing of 1/12 Annual Fee + Works Orders + Service Credits), C2.9 (Late Payment of Commercial Debts (Interest) Act 1998), C2.10 (30-day sub-contract payment).
- **Best Value / continuous improvement** — B8.1 (Local Government Act 1999).
- **Dispute resolution** — S10.1 (Scheme for Construction Contracts SI 1998/649 as amended by SI 2011/2333), S10.6 (CIMAR Rules, Chartered Institute of Building).
- **KPIs / Service Credits** — F5 (`#/texts/779`, p51) + C2.2 + C3A.
- **Empty schedules (negative evidence)** — `SCHEDULE 1 SPECIFICATION` (`#/texts/897`, p60), `SCHEDULE 2 PRICING SCHEDULE` (`#/texts/900`, p61), `SCHEDULE 3 THE COUNCIL'S INVITATION TO TENDER` (`#/texts/903`, p62), `SCHEDULE 4 THE CONTRACTOR'S TENDER` (`#/texts/906`, p63) — each appears only as a heading with no body content in the Docling output.
- **Source markdown hash** — `0e177f5c9186a8e54fef7794d36b0d3c04a9e887915793b56cc25e98bed70247` (from `document_extractions.source_content_hash`), recorded in `extraction_metadata.source_markdown_hash`.
- **Notice-level cross-reference** — pre-existing `notice_lots` row `pc-dc1daee2-772e-f111-813c-005056b64545-lot-7` has `title = "Lift Servicing, Maintenance, Repair, Minor Refurbishment and Compliance Management"`, consistent with the document's single-lot scope.
