---
manifest_version: 1
skill_id: pdf-extraction/tender-specification
skill_version: 1.0.0
job_id: job-79515af8-4d9f-45c2-8b5a-83b9eaa5a1d1
session_id: session-semantic-6a28d1cffd0835c2bc7b2646376630306e5facfa06d82738a97766fd72afc1bc
execution_mode: contract
started_at: 2026-04-18T00:00:00Z
completed_at: 2026-04-18T00:00:00Z
outcome_class: success
items_processed: 1
items_skipped: 0
items_failed: 0
warnings_count: 6
agent_claimed_pages_visited: 0
agent_claimed_source_url: null
agent_claimed_fingerprint_state: n/a
agent_claimed_locale_state: en-GB
agent_claimed_currency_state: GBP
---

# Run Manifest — pdf-extraction/tender-specification

## Summary

Extracted one lot-scoped tender-specification payload from Docling markdown for `document_extractions.id=fc6316d9-c10c-4224-aa2b-fe80c853a908` (raw_document `raw-pc-dc1daee2-772e-f111-813c-005056b64545-...`, document title `6278 - Draft specification - Lot 2`). The document is the London Borough of Bexley Water Risk Assessments (Legionella Survey) specification for Lot 2 of Framework 6278, with contract commencement April 2026 for a 3+2 year term. One lot record (`lot_number=2`, title "Provision of Water Risk Assessments (Legionella Survey)") was produced with 46 requirements, 4 evaluation-criterion submission requirements, 6 technical standards and 9 product-requirement seeds. Extraction scope = `full_document`; target_domain = `lots`; no CPV codes are stated inside the document itself so `cpv_codes` is emitted as an empty array with an explicit warning.

## Steps Taken

1. Read Docling markdown via `document_extractions.extracted_markdown` for `id=fc6316d9-c10c-4224-aa2b-fe80c853a908` from the local state database (`state/state.db`) — 266 lines, 14 numbered sections, 0 tables, 0 images, 0 Docling-reported page count.
2. Cross-checked the parent notice (`contract_notices.id=pc-dc1daee2-772e-f111-813c-005056b64545`, title "6278 - Building Services Framework - Lots 1-11") and existing `notice_lots` rows; confirmed `lot 2 = Provision of Water Risk Assessments` — the spec is authoritative for Lot 2's detailed scope whereas `notice_lots.lot-2.description` in the database is derived from the draft T&Cs and does not embed Schedule 1 content.
3. Walked the document sections in order (1 Introduction, 2 General, 3 Specification of Services, 4 Job Completion, 5 Health and Safety, 6 Permits to Work, 7 Energy & Environmental, 8 Training, 9 Organisational Structure, 10 Sub Contractors, 11 Complaints, 12 Customer Care, 13 Mobilisation, 14 Continuous Improvement) and enumerated requirements, technical standards and evaluation-adjacent bid-submission obligations with evidence anchors keyed to section headings and contents-page references.
4. Distilled nine product-requirement seeds that are concretely procurable deliverables (Water Hygiene Assessment report, RAG-rated remedial programme, site-specific Water Hygiene Management Plan, proactive monitoring programme, Legionella training, Dynamic Risk Assessment process, RAMS submission, CAFM / TF Cloud / Civica integration, DBS-checked workforce) and anchored each to its section.
5. Deduplicated the verbatim repetition of the ACOP L8 / HSG 274 / BSI 8580:2010 compliance paragraph in section 3.
6. Validated the emitted JSON against `skills/pdf-extraction/tender-specification/result.schema.json` (jsonschema 2020-12 validator) before persisting.
7. Wrote `result.json` to the session workdir with `extraction_metadata.target_domain=lots`, `extraction_scope=full_document`, `policy_version=2026.04.12`, and `source_markdown_hash=61ceefa07f282a5fb21c1d2304224143ed52643615c1c5033f45be105e38cc9b` (sha256 of the markdown text as exported from `document_extractions.extracted_markdown`).

## Decisions Made

- Treated the document as a single-lot specification (Lot 2 only) despite the parent notice covering Lots 1-11: the file's own title ("6278 - Draft specification - Lot 2") and body content are scoped to Water Risk Assessments / Legionella, and the `document_set_id=tds-pc-dc1daee2-772e-f111-813c-005056b64545` plus existing lot-2 row confirm this.
- Left `cpv_codes` empty on the lot rather than inheriting notice-level CPVs (50700000-2 / 50750000-7 / 71315000-9): none are stated in the document and the skill contract requires evidence-anchored claims; raised `CPV_NOT_IN_DOCUMENT` warning so the daemon can pull CPV from notice-level metadata rather than from this extraction.
- Emitted `evaluation_criteria` only for bid-submission obligations that are explicitly called out in the spec (sample reporting documentation, environmental policy, formal complaints procedure, continuous-improvement initiatives). Flagged `EVALUATION_CRITERIA_NOT_FULLY_ENUMERATED` because no weighted MEAT / scoring matrix is embedded — the formal evaluation lives in Schedule 3 / ITT which is referenced but not present.
- Page refs on evidence anchors are derived from the contents index (`p.3` ... `p.13`) rather than Docling metadata, because `document_extractions.page_count=0`. Warning `DOCLING_PAGE_COUNT_UNKNOWN` records this.
- Treated the contents-list gap ("9 ... 11 Complaints") as a typo: section body clearly contains `## 10 Sub Contractors` between §9 and §11. Section 10 was included in the extraction; `TOC_NUMBERING_ANOMALY` records the reasoning so the daemon doesn't mis-detect a missing section.
- Seed keys are stable, kebab-case, and shared between `lots[0].product_requirements[*].seed_key` and top-level `product_requirement_seeds[*].seed_key` so the daemon can idempotently translate to `product_candidates` writes on resume.

## Warnings

- `CPV_NOT_IN_DOCUMENT` — No CPV codes inside the document; daemon should rely on notice-level CPVs.
- `EVALUATION_CRITERIA_NOT_FULLY_ENUMERATED` — Formal weighted evaluation criteria not embedded; only bid-submission obligations captured.
- `ANCILLARY_APPENDICES_REFERENCED_BUT_NOT_EMBEDDED` — Appendices 1 (Price Matrix), A (Building List), B (process map), C (Environmental Policy), D (Code of Conduct) are referenced but not present in this file; substantive asset coverage and workflow detail live outside this extraction.
- `TOC_NUMBERING_ANOMALY` — Contents list skips "10"; section body does contain §10 Sub Contractors.
- `DOCLING_PAGE_COUNT_UNKNOWN` — `document_extractions.page_count=0`; page refs derived from contents index, not Docling page metadata.
- `DOCUMENT_DUPLICATED_PARAGRAPHS` — Section 3 duplicates the ACOP L8 / HSG 274 / BSI 8580:2010 compliance paragraph verbatim; output deduplicated.

## Evidence

All evidence references are anchored to section headings and/or contents-page numbers in the Docling markdown stored at `document_extractions.extracted_markdown` for `id=fc6316d9-c10c-4224-aa2b-fe80c853a908` (source_markdown_hash `61ceefa07f282a5fb21c1d2304224143ed52643615c1c5033f45be105e38cc9b`).

- Document identity — `## Legionella Survey` / `## Specification` / `## Contract Commencement April 2026` (document title block, p.1).
- Contract term and scope framing — `## 1 Introduction and Context`: "The contract will be for a period of 3 + 2 years ... The Supplier shall provide all expertise, staff, labour, materials, equipment, and resources for the undertaking of Legionella Surveys." (p.3).
- Annual schedule and Price Matrix reference — `## 2 General`: "In collaboration with the Council, the Contractor shall prepare an annual schedule of Water Risk Assessments, aligned with this Specification and Appendix 1 – Bexley Water Risk Assessment – Price Matrix." (p.4).
- Compliance basis — `## 3 Specification of Services`: "The Water Hygiene service must comply with the Approved Code of Practice (ACOP) L8, HSG 274 Parts 1–3, and BSI 8580:2010." (p.5). Same paragraph repeats verbatim twice — deduplicated in output.
- 10-day report SLA and contents — `## 3 Specification of Services`: "Each site must receive a Water Hygiene Assessment report within ten days, including risk summaries, site details, schematic diagrams, and technical data." (p.5).
- 48-hour high-priority email SLA — `## 3 Specification of Services`: "high-priority remedial work must be communicated to the FM Team via email within 48 hours of the inspection" (p.5-6).
- RAG remedial programme format and cadence — `## 3 Specification of Services`: "A RAG report must be produced electronically in Microsoft Excel format from the start of the contract." (p.6).
- Bid-submission requirement — `## 3 Specification of Services`: "Sample documentation demonstrating all the above requirements must be submitted with the bid." (p.6).
- Asset Management / CAFM integration — `## 4 Job Completion`: "The contractor is required to use the Council's Asset Management System(TF Cloud/Civica)  to complete work and manage invoicing." (p.7). Building list reference: "schedule of buildings ... is detailed in Appendix 1" (p.7).
- RAMS — `## 5 Health and safety Compliance`: "Prior to commencing any works on site, the Contractor shall submit a Risk Assessment and Method Statement (RAMS) for Council approval, covering all services detailed in the specification." (p.8).
- RIDDOR and asbestos assumption — `## 5 Health and safety Compliance`: "Reporting of Injuries, Diseases and Dangerous Occurrences Regulations (RIDDOR)" and "the Contractor shall assume the presence of asbestos-containing materials in plant rooms and appliances unless proven otherwise" (p.8).
- Dynamic Risk Assessment mandate — `## 6 Permits to work`: "The Contractor will provide a Dynamic Risk Assessment which MUST be completed prior to any works (planned or reactive)." (p.9).
- Environmental policy bid-submission — `## 7 Energy & Environmental Assessments`: "The Contractor is further invited to submit details of their Company Environmental Policy; complete with illustrations of how it has been practically applied." (p.10).
- Training scope — `## 8 Training`: "The ability to provide Water Management awareness training courses for Council staff and suppliers" (p.11).
- DBS and replacement SLA — `## 9 Organisational Structure and Mobilisation`: "The Contractor is responsible for ensuring that all staff have undergone appropriate Disclosure and Barring Service (DBS) checks." and "Replacement personnel must be suitably qualified and deployed within five (5) working days." (p.11).
- Sub-contractor regime — `## 10 Sub Contractors`: "All sub-contractors, and any changes to them, must be approved by the Council." (p.11-12).
- Complaints SLA — `## 11 Complaints`: "notify the Contract Administrator within 24 hours, providing full details of the complaint and the corrective actions taken." (p.12).
- Complaints procedure as a bid deliverable — `## 12 Customer Care`: "The Contractor shall implement a formal complaints procedure ... This procedure must be included as part of the tender submission." (p.12).
- Mobilisation outputs — `## 13 Mobilisation`: "Configure all necessary IT and reporting systems ... Confirm Key Performance Indicators (KPIs) ... Finalise the Water Risk Assessment programme of work." (p.12).
- Continuous improvement — `## 14 Continuous Improvement`: "The Contractor shall proactively propose new technologies, materials, and methods that support greater efficiency, improved service outcomes, and better value for money." (p.13).
