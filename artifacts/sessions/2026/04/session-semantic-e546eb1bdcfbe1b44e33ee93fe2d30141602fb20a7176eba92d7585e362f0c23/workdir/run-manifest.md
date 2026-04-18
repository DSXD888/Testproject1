---
manifest_version: "1.0.0"
skill_id: "pdf-extraction/tender-specification"
skill_version: "1.0.0"
job_id: "job-a0750edb-b0b3-4a26-85e2-a53ef1f57f50"
session_id: "session-semantic-e546eb1bdcfbe1b44e33ee93fe2d30141602fb20a7176eba92d7585e362f0c23"
execution_mode: "contract"
started_at: "2026-04-18T06:55:00Z"
completed_at: "2026-04-18T06:58:00Z"
outcome_class: "success"
items_processed: 1
items_skipped: 0
items_failed: 0
warnings_count: 7
agent_claimed_pages_visited: "p.1 front matter, p.3 contents + section 1, p.4 section 2, p.5-6 section 3, p.7 section 4, p.8 section 5, p.9 section 6 + section 7, p.11 section 8 + section 9, p.12 section 10 + section 11 + section 12 + section 13, p.13 section 14"
agent_claimed_source_url: "https://procontract.due-north.com/RfxResponse/Index?rfxId=ea9d90b2-4833-f111-813c-005056b64545"
agent_claimed_fingerprint_state: "not_applicable_headless_contract"
agent_claimed_locale_state: "en-GB"
agent_claimed_currency_state: "GBP"
---

## Summary

Processed `6278 - Draft specification - Lot 2 - Provision of Water Risk Assessments.docx` for London Borough of Bexley Framework 6278 (process `proc-pc-dc1daee2-772e-f111-813c-005056b64545`). The document is a 14-section Specification ("Schedule 1") covering Water Risk Assessments / Legionella Surveys across the Council's operational estate, commencing April 2026 for a 3 + 2 year term. Produced one authoritative lot (Lot 2) with 35 service requirements, 4 product-requirement seeds (WRA service, Water Hygiene Management Plan + monitoring, Dynamic Risk Assessment service, Water Management training + ACOP L8 advisory), 12 technical / statutory standards, and 25 evidence anchors. Emitted seven warnings for missing CPV codes, missing evaluation criteria, missing contract value, unreachable Appendix A Building List / Appendix 1 Price Matrix / Appendix C Environmental Policy / Appendix D Code of Conduct, 3+2-year contract-option structure, and a minor numbering artefact (contents skips section 10). Overall extraction confidence 7800 bps (78%).

## Steps Taken

1. Loaded `document_extractions` row `7be80c25-905c-4b34-9abe-09f0a7c1f937` (26,177 bytes of Docling markdown, 32 sections, 0 tables, `document_role = technical_specification`, `extraction_policy_version = 2026.04.12`) and confirmed `target_domain = "lots"` with `extraction_scope = "full_document"`.
2. Confirmed the raw document is a `.docx` extracted from the ProContract ZIP bundle for RfX `ea9d90b2-4833-f111-813c-005056b64545`; `document_role_hint = technical_specification` per `raw_documents.metadata.document_role_hint`.
3. Cross-referenced the pre-existing `notice_lots` row for Lot 2 on notice `pc-dc1daee2-772e-f111-813c-005056b64545` (title `Provision of Water Risk Assessments`) and confirmed the Specification pertains to that lot; kept `lot_number = "2"` so downstream writes remain stable.
4. Read sections 1-14 end-to-end (contents on p.3; Introduction §1 p.3; General §2 p.4; Specification of Services §3 p.5-6; Job Completion §4 p.7; Health & Safety §5 p.8; Permits to Work §6 p.9; Energy & Environmental §7 p.9; Training §8 p.11; Organisational Structure §9 p.11; Sub-Contractors §10 p.12; Complaints §11 p.12; Customer Care §12 p.12; Mobilisation §13 p.12; Continuous Improvement §14 p.13).
5. Extracted 35 lot-level requirements grouped by section - preserved section-number citations inside each requirement string so downstream readers can trace to source.
6. Identified 4 distinct service seeds (WRA service, Water Hygiene Management Plan + monitoring, Dynamic Risk Assessment service, Water Management training + ACOP L8 advisory) and emitted them both under `lots[0].product_requirements` and at the top level as `product_requirement_seeds` with stable `seed_key`s prefixed `bexley-6278-lot2-`.
7. Captured 12 technical / statutory standards (ACOP L8, HSG 274 Parts 1-3, BS 8580:2010, HSW Act 1974, CAR 2012, RIDDOR, HSE Authorised Person protocols, disability-discrimination and equalities legislation, RAMS submission, DBS checks, Environmental Policy / Social Value, CAFM TF Cloud / Civica) with direct anchor-text evidence per standard.
8. Recorded missing-but-referenced items (Appendix A Building List, Appendix 1 Price Matrix, Appendix C Environmental Policy, Appendix D Code of Conduct) and absent-in-document items (CPV codes, evaluation criteria, contract value) as explicit warnings rather than guesses.
9. Emitted `result.json` in the session workdir and validated it against `pdf-extraction/tender-specification/result.schema.json` (Draft 2020-12) - validation passed.

## Decisions Made

- **Lot number "2", matching the existing `notice_lots` row.** The document is Framework 6278 Lot 2 (`Provision of Water Risk Assessments`). Preserving `lot_number = "2"` rather than re-numbering to "1" keeps continuity with the pre-existing lot row and supports the SKILL Resume Strategy (stable lot numbering across retries).
- **Title aligned with notice title, not document front matter.** The front matter says "Legionella Survey / Specification"; the notice and lot description say "Provision of Water Risk Assessments". Merged to `Provision of Water Risk Assessments (Legionella Survey)` so downstream matching can anchor either way without contradicting the notice_lots row.
- **Empty `cpv_codes` and `evaluation_criteria` on the lot.** The Specification contains no CPV classifications and no award-criteria / weightings framework. Emitting `[]` plus explicit warnings (`CPV_CODES_NOT_IN_DOCUMENT`, `EVALUATION_CRITERIA_NOT_IN_DOCUMENT`) is faithful to the source; inventing CPVs from service nouns or copying from the notice would violate the "cite evidence anchor" rule.
- **Four product-requirement seeds (services, not goods).** The scope is services delivery (WRA inspection, plan production, monitoring, DRA, training, advisory). Emitted one seed per distinct deliverable type - Water Risk Assessment service, Water Hygiene Management Plan + monitoring programme, Dynamic Risk Assessment service, Water Management training + ACOP L8 advisory - with `requirement_type` values that reflect service deliverables rather than physical goods. The schema permits free-text `requirement_type`, so chose descriptive types so downstream product-matching can correctly classify them as `service` archetype.
- **Technical standards captured beyond pure-product standards.** ACOP L8, HSG 274 Parts 1-3, and BS 8580:2010 are core service standards; HSW Act 1974, CAR 2012, RIDDOR, HSE Authorised Person protocols, disability-discrimination and equalities legislation, RAMS submission, DBS checks, the Environmental Policy + Social Value obligation and the CAFM platform (TF Cloud / Civica) all frame the service-delivery envelope and are emitted under `technical_standards` with direct anchor text. Consistent with the SKILL outputs spec (standards that govern how the service is delivered).
- **Page references derived from the embedded contents table.** The Docling markdown has no explicit page markers but the contents table on p.3 maps each section to a page number. Used those numbers as `page_ref` (e.g. `page_ref = "5"` for section 3 content) so downstream reviewers can locate evidence in the printed PDF.
- **Contract commencement and duration captured; value and extension mechanics deferred.** Section 1 says "The contract will be for a period of 3 + 2 years"; the front matter says "Contract Commencement / April 2026". Emitted these as a lot-level requirement plus the `CONTRACT_DURATION_AS_OPTION_STRUCTURE` warning. Financial value, firm extension notice, and termination mechanics are not in this Specification - they live in the Form of Contract / Terms and Conditions and the Price Matrix.
- **Confidence at 7800 bps (78%).** Strong signal on lot identity, scope, service deliverables, statutory standards, and operational obligations - these are stated verbatim. Down-scored for missing CPV codes, missing evaluation criteria, missing contract value, and several appendices referenced but absent (Appendix A Building List, Appendix 1 Price Matrix, Appendix C Environmental Policy, Appendix D Code of Conduct). Higher than the draft-contract 6800 bps reference run because the Specification body is populated here (not a stub) and obligations are concrete.

## Warnings

1. `EVALUATION_CRITERIA_NOT_IN_DOCUMENT` - no award / scoring criteria or weightings are stated in the Specification; they belong to the ITT / award pack.
2. `CPV_CODES_NOT_IN_DOCUMENT` - no CPV codes are stated; classification must be taken from the notice / Find a Tender record.
3. `CONTRACT_VALUE_NOT_IN_DOCUMENT` - contract value / budget is not stated; volumes and pricing are held in the separately issued Appendix 1 - Bexley Water Risk Assessment - Price Matrix.
4. `BUILDING_LIST_APPENDIX_NOT_IN_EXTRACTION` - Appendix A Building List (site schedule) is referenced by §3 but not present in this extraction.
5. `ENVIRONMENTAL_POLICY_APPENDIX_NOT_IN_EXTRACTION` - Appendix C (Environmental Policy) and Appendix D (Code of Conduct) referenced in §7 and §9 are not present in this extraction.
6. `CONTRACT_DURATION_AS_OPTION_STRUCTURE` - "3 + 2 years" (commencement April 2026) captured; firm extension notice and termination payment mechanics live in the Form of Contract, not the Specification.
7. `NUMBERING_GAP_IN_CONTENTS` - contents skips section 10 (jumps from 9 to 11); section 10 (Sub Contractors) exists in the body. Minor structural artefact only; does not affect extraction.

## Evidence

All major claims above are anchored to the following observable items in the Docling markdown for `document_extraction_id = 7be80c25-905c-4b34-9abe-09f0a7c1f937` (`source_content_hash = sha256:5b458f2e44333e1f77430edc846b3d0d065c5c53bd56ae90fd81edd8903a7681`, `extraction_policy_version = 2026.04.12`, `document_role = technical_specification`):

- **Lot identity and scope.** Front matter p.1: `London Borough of Bexley / Legionella Survey / Specification / Contract Commencement April 2026`. §1 p.3: `This Service Specification provides an outline of Water Management Surveys ... The contract will be for a period of 3 + 2 years.` §2 p.4: `The Contractor shall conduct Water Risk Assessments as instructed`. Cross-ref: pre-existing `notice_lots` row for notice `pc-dc1daee2-772e-f111-813c-005056b64545` Lot 2 - `Provision of Water Risk Assessments`.
- **Core water-hygiene compliance standards (ACOP L8 / HSG 274 / BS 8580:2010).** §3 p.5: `The Water Hygiene service must comply with the Approved Code of Practice (ACOP) L8, HSG 274 Parts 1-3, and BSI 8580:2010.` Anchored to the `bexley-6278-lot2-water-risk-assessment-service` seed.
- **Inspection scope and report deliverables.** §3 p.5: `The contractor shall inspect hot and cold water systems, including mains distribution, tanks, calorifiers, and outlets.` / `Each site must receive a Water Hygiene Assessment report within ten days, including risk summaries, site details, schematic diagrams, and technical data.` / `Immediate risks must be reported on-site to the FM Team, and high-priority remedial actions must be emailed within 48 hours.` / `Risks must be rated (high, medium, low), and reports must include photographic evidence and clear recommendations.`
- **Water Hygiene Management Plan + monitoring programme.** §3 p.5: `The contractor must develop and maintain a site-specific Water Hygiene Management Plan for each location, incorporating findings from Water Risk Assessments (WRAs).` / `A proactive monitoring schedule must be established, including temperature checks, inspection of tanks and calorifiers, flushing regimes, and biocide dosing where applicable.` Anchors the second seed.
- **RAG Excel programme and reporting media.** §3 p.6: `A RAG report must be produced electronically in Microsoft Excel format from the start of the contract.` / `Monitoring and reporting must be electronic via CAFM (TF Cloud) or email.`
- **CAFM / Asset Management System (TF Cloud / Civica) as authoritative invoicing and record-keeping channel.** §4 p.7: `The contractor is required to use the Council's Asset Management System(TF Cloud/Civica) to complete work and manage invoicing.` / `Applications for payment must be submitted through the Asset Management System. The Council will not accept invoices in any other format.` / `the schedule of buildings where inspections may be instructed is detailed in Appendix 1 - Bexley Water Risk Assessment - Price Matrix.`
- **Health & Safety statutory framework.** §5 p.8: `the Health and Safety at Work Act 1974, all relevant statutory provisions under that Act, and all other applicable safety legislation, codes of practice, and Health and Safety Executive (HSE) Good Practice Guidance Notes` / `Prior to commencing any works on site, the Contractor shall submit a Risk Assessment and Method Statement (RAMS) for Council approval` / `which incidents are reportable under the Reporting of Injuries, Diseases and Dangerous Occurrences Regulations (RIDDOR)`.
- **Asbestos handling obligation.** §2 p.4: `If asbestos is discovered or suspected, the Contractor must immediately cease work and notify the Council ... full compliance with the Control of Asbestos Regulations 2012` / §5 p.8: `the Contractor shall assume the presence of asbestos-containing materials in plant rooms and appliances unless proven otherwise.`
- **Dynamic Risk Assessment regime.** §6 p.9: `The Contractor will provide a Dynamic Risk Assessment which MUST be completed prior to any works (planned or reactive). The format and copy will be made available during mobilisation ... For the avoidance of doubt, the completion of this risk assessment is mandatory.` Anchors the third seed.
- **Environmental Policy / Social Value.** §7 p.9: `The Contractor will agree to adhere to the Environmental Policy (Appendix C) where possible and commit to reducing the Boroughs Carbon Emissions, supporting the Council's Social Value objectives.`
- **Water Management training and ACOP L8 advisory.** §8 p.11: `The ability to provide Water Management awareness training courses for Council staff and suppliers` / `Training and informing the Council and its partners in relation to existing and/or future changes in Regulation and its duties under the (ACOP) L8` / `Provide specialist advice and assistance on all matters relating to (ACOP) L8`. Anchors the fourth seed.
- **Staffing, DBS and organisational obligations.** §9 p.11: `The Contractor is responsible for ensuring that all staff have undergone appropriate Disclosure and Barring Service (DBS) checks.` + replacement within five (5) working days of underperforming staff.
- **Complaints and customer care.** §11 p.12: `All complaints must be immediately referred to the FM Client Unit.` + 24-hour notification. §12 p.12: formal complaints procedure required in tender submission.
- **Mobilisation obligations.** §13 p.12: `During the mobilisation period, the Contractor shall: Configure all necessary IT and reporting systems; Agree the format and content of reports; ... Finalise the Water Risk Assessment programme of work; Confirm Key Performance Indicators (KPIs)`.
- **Continuous improvement obligation.** §14 p.13: `The Contractor shall proactively propose new technologies, materials and methods that support greater efficiency, improved service outcomes, and better value for money.`
- **Missing-appendix warnings evidence.** §2 p.4: `aligned with this Specification and Appendix 1 - Bexley Water Risk Assessment - Price Matrix` (not in extraction). §3 p.5: `as outlined in the Building List (Appendix A)` (not in extraction). §7 p.9: `adhere to the Environmental Policy (Appendix C)` (not in extraction). §9 p.11 narrative references `Code of Conduct (Appendix D)` (not in extraction).

Result artefact written to `result.json` in the session workdir; schema-validated against `skills/pdf-extraction/tender-specification/result.schema.json` (Draft 2020-12).
