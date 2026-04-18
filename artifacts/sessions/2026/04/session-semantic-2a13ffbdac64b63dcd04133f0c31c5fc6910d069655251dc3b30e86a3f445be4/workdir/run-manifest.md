---
manifest_version: "1.0"
skill_id: pdf-extraction/tender-specification
skill_version: 1.0.0
job_id: job-68b9431f-8f4e-4435-aa31-555e2f3379f4
session_id: session-semantic-2a13ffbdac64b63dcd04133f0c31c5fc6910d069655251dc3b30e86a3f445be4
execution_mode: contract
started_at: "2026-04-18T07:48:00Z"
completed_at: "2026-04-18T07:53:39Z"
outcome_class: success_with_warnings
items_processed: 1
items_skipped: 0
items_failed: 0
warnings_count: 10
agent_claimed_pages_visited: 0
agent_claimed_source_url: "/FileUploads/ViewAttachment?attachmentId=249afe7f-e62c-f111-813c-005056b64545&entityTypeId=9b214c2e-2d7d-432b-bced-4df247225fb3&attachmentTypeId=Rfx"
agent_claimed_fingerprint_state: not_applicable
agent_claimed_locale_state: en-GB
agent_claimed_currency_state: GBP
---

## Summary

Extracted one authoritative `lots` payload for process `proc-pc-f0f30f94-e22c-f111-813c-005056b64545` (notice `pc-f0f30f94-e22c-f111-813c-005056b64545` — "020417 - Stockport Town Centre School - Multi-disciplinary Design Team Appointment"). The bound raw document for this job is `raw-pc-f0f30f94-e22c-f111-813c-005056b64545-70b7ff34b5170c390c943d0350499b2247f03aec5232e1b7ac38a857d338d6d1`, which the tender document set identifies as `terms_and_conditions` (file `4. T&Cs (with novation) (To be read).DOCX`). Only `document_extractions.document_role` mislabels it as `technical_specification`; this is flagged as `DOCUMENT_ROLE_MISMATCH`.

The document is a DAC Beachcroft fill-in consultancy appointment template. It does not split the scope into multiple lots, so the extraction emits a single lot aligned with the pre-existing `notice_lots` row (`lot_key = 1`). Four professional-service product_requirement_seeds are emitted (Architect, MEP, Structures & Civils, CDM Principal Designer). Four technical standards are cited (British/EU Standard & Code of Practice, BCO 'Good Practice in the Selection of Construction Materials' + BRE Digest, CDM Regulations 2015, Public Contracts Regulations 2015). Evaluation criteria and CPV codes are empty: neither is defined in a T&Cs template. Overall `extraction_confidence_bps = 4800` reflects the role mismatch plus the template-with-placeholders nature of the source.

## Steps Taken

1. Read the bound SKILL.md and result.schema.json for `pdf-extraction/tender-specification` from `skills/pdf-extraction/tender-specification/`.
2. Fetched the document extraction row (id `30244ca2-7c79-4a7e-8e58-890221489095`) from `state/state.db`: confirmed `document_set_id`, `process_id`, `notice_id`, `document_role = technical_specification` (mislabelled), `document_title = 4-Terms-and-Conditions-Novation`, `page_count = 0`, `section_count = 37`, `table_count = 10`, `source_content_hash = ff29d376d6e11cce814aea5bcc89f8fba2f52d547e5950960a99e1e4ee08d69e`.
3. Cross-checked `tender_document_sets.document_roles_json`: bound `raw_document_id` is mapped to `terms_and_conditions`. Recorded mismatch as warning.
4. Streamed the full `extracted_markdown` (~202KB / 1348 lines) to a local file and read it in full — including the 28-clause body, Schedule 1 Contract Particulars, Schedule 2 Fees template, Schedule 3 Services template, Schedule 4 Consultant Collateral Warranty, Schedule 5 Sub-Consultant Collateral Warranty and Schedule 6 Deed of Novation.
5. Inspected the existing `notice_lots` row (`pc-f0f30f94-e22c-f111-813c-005056b64545-lot-1`) and preserved `lot_number = "1"` in the authoritative lot output.
6. Identified populated Schedule 1 fields (Project description, Consultant Discipline, Client identity, PI Insurance £10M, Liability cap £10M, CDM definition) as the richest evidence surface; everything else in Schedule 1 is a `[insert …]` placeholder and was flagged, not invented.
7. Derived the discipline list from Schedule 1 'Consultant Discipline' row (Architect, MEP, Structures & Civils) and augmented with CDM Principal Designer from clause 20 — noting the gap against the notice summary and the A.2/A.4 scope-of-service filenames (BR Principal Designer scope sits outside this document).
8. Built `technical_standards` only from literal references in the document body (clause 2.3, clause 20, clause 8.5). Did not hallucinate BS EN / ISO codes.
9. Authored all `evidence_anchor` entries with `page_ref = "n/a"` because `document_extractions.page_count = 0` for this DOCX-derived extraction; used `section_ref` values pointing at clause numbers or Schedule 1 rows that exist in the markdown.
10. Set `field_confidences` conservatively: high (85-95%) for lot identity and standards, mid (75%) for requirements, low (20-35%) for evaluation criteria and document_role. Set global `extraction_confidence_bps = 4800` to reflect the role mismatch and template-placeholder risks.
11. Wrote `result.json` to the current working directory and validated it against `skills/pdf-extraction/tender-specification/result.schema.json` with a Draft 2020-12 JSON Schema validator. No errors.
12. Produced this run manifest.

## Decisions Made

- **One lot, not several.** The document is a single consultancy appointment framework and does not propose divisible lots. Multiple lots would be inventing structure. Chose to preserve `lot_number = "1"` so downstream daemon persistence stays stable against the existing `notice_lots` row.
- **Service-type seeds, not physical products.** Notice main_category is `150000 - Consultancy` and the appointment is for professional services. All four `product_requirement_seeds` carry `requirement_type = "professional_service"` and include a `SERVICES_NOT_PRODUCTS` warning so downstream product-matching / landed-cost flows treat these correctly.
- **No invented CPV codes.** The document asserts no CPV codes. The notice carries `main_category = "150000 - Consultancy"`, but the skill is explicit that CPV codes must be cited with evidence anchors from the tender document itself. Left `cpv_codes = []`.
- **No invented evaluation criteria.** Award criteria are not present in a T&Cs template. Emitted empty `evaluation_criteria` with a `NO_EVALUATION_CRITERIA_PRESENT` warning pointing to the ITT document (1A) and Response Document (3) as the correct sources.
- **Evidence anchors use section_ref, not page_ref.** `page_count` is 0 for this DOCX-derived extraction; page references would be fabricated. Used clause numbers and Schedule 1 row names that actually appear in the markdown.
- **Included CDM Principal Designer as a seed even though Schedule 1 names only three disciplines.** Clause 20 explicitly binds the Consultant to CDM 2015 designer duties, which is stronger evidence than the filename-based hint from the A.4 scope document. Flagged the Schedule 1 vs notice-summary discrepancy as `DISCIPLINE_LIST_INCOMPLETE_VS_NOTICE`.
- **Did not emit a BR Principal Designer seed.** No clause in this document references Building Regulations Principal Designer duties. The A.2 scope-of-service document sits outside this extraction and should be covered by its own skill run.
- **Confidence ~48% (4800 bps).** Lower than the 55-85% range observed in prior tender-specification runs because (a) role mismatch and (b) the document is a legal template rather than a genuine specification. Honest downgrade, not punitive.

## Warnings

| Code | Summary |
|---|---|
| DOCUMENT_ROLE_MISMATCH | `document_extractions.document_role = technical_specification` but the source is the T&Cs template; `tender_document_sets` correctly labels it `terms_and_conditions`. |
| TEMPLATE_WITH_PLACEHOLDERS | Schedule 2 Fees, Schedule 3 Services, Site, Principal Designer, Key Personnel, dates, Consultant identity are all bracketed placeholders. |
| NO_EVALUATION_CRITERIA_PRESENT | Award criteria live in the ITT (1A) and Response Document (3), not in the T&Cs. |
| NO_CPV_CODES_IN_DOCUMENT | No CPV codes asserted in the document; prior notice data is not extrapolated. |
| SERVICES_NOT_PRODUCTS | Procurement is professional consultancy; seeds are services, not goods. |
| SINGLE_LOT_ALIGNS_WITH_NOTICE | One authoritative lot, lot_number preserved to match the existing `notice_lots` row. |
| DISCIPLINE_LIST_INCOMPLETE_VS_NOTICE | Schedule 1 lists three disciplines; notice summary and A.2/A.4 filenames imply five. |
| PAGE_REFERENCES_UNAVAILABLE | `page_count = 0` for this DOCX-derived extraction; all anchors use `section_ref`. |
| PI_INSURANCE_AND_LIABILITY_CAP_FIXED | £10M PI per claim / £10M liability cap per claim — material commercial terms. |
| NOVATION_OPTION_PRESENT | Schedule 6 Deed of Novation and collateral warranty cascade (Schedules 4–5) apply. |

All warnings have `evidence_refs` populated in `result.json` where a concrete anchor exists.

## Evidence

Every major claim in `result.json` is anchored to content that physically exists in the structural extraction payload. Representative anchors:

- **Lot scope — Project description** → Schedule 1 Contract Particulars, 'Project' row (Clause 1.1):  
  "Design and Technical Services are looking to procure an architecturally led multi disciplinary design team to develop a feasibility through to concept design of a new school, in a town centre location." (line 645 of the extracted markdown)

- **Disciplines** → Schedule 1 Contract Particulars, 'Consultant Discipline' row (Clause 1.1 and 2.2):  
  "Architect MEP Structures & Civils" (line 646 of the extracted markdown).

- **Client identity** → Schedule 1 Contract Particulars, 'the Client' row:  
  "Metropolitan Borough Council of Stockport of Town Hall, Stockport SK1 3XE" (line 647).

- **British / EU Standard + BRE Digest + BCO Good Practice** → Clause 2.3:  
  "contravene any British or EU Standard or Code of Practice current at the date of specification … 'Good Practice in the Selection of Construction Materials' published by the British Council of Offices and the BRE Digest" (line 156).

- **CDM Regulations 2015** → Clause 20 and Schedule 1 'CDM Regulations' row:  
  "it shall be a 'designer' of the Project as defined in the CDM Regulations and shall fulfil in relation to the Project all the obligations for a designer as set out in or reasonably to be inferred from the CDM Regulations and the guidance published by the Health and Safety Executive" (line 476); definition row (line 659).

- **Public Contracts Regulations 2015** → Clause 8.5:  
  "Where this Agreement is subject to the Public Contracts Regulations 2015 ('PC Regulations') … include in any sub-contract entered into by it, suitable provisions to impose the requirements of regulation 113 (2) (c) (i) and (ii)" (line 300).

- **PI Insurance cap** → Schedule 1 'Professional indemnity insurance' row:  
  "Ten million pounds (£10,000,000) for any one claim in any one period of insurance." (line 656).

- **Liability cap** → Schedule 1 'Consultant's limitation of liability' row:  
  "Ten million pounds (£10,000,000) for any one claim." (line 664).

- **Novation option** → Schedule 6 recitals and clause 9.7:  
  "the Consultant shall … execute and deliver to the Client a deed or deeds of novation in the form attached at Schedule 6" (line 340); Schedule 6 cover (line 1178).

- **Template placeholders** → Schedule 2 Fees table with "[insert amount in words] (£ [insert amount in figures])" (line 672) and Schedule 3 "The Services" block with "[insert relevant list of services]" (line 701).

### Daemon appendix fields referenced

- `document_extractions.id = 30244ca2-7c79-4a7e-8e58-890221489095`
- `document_extractions.source_content_hash = ff29d376d6e11cce814aea5bcc89f8fba2f52d547e5950960a99e1e4ee08d69e` (used verbatim as `extraction_metadata.source_markdown_hash`)
- `document_extractions.extraction_policy_version = 2026.04.12` (carried into `extraction_metadata.policy_version`)
- `document_extractions.page_count = 0`, `section_count = 37`, `table_count = 10` (basis for `PAGE_REFERENCES_UNAVAILABLE`)
- `tender_document_sets.document_roles_json[raw-…70b7ff34…] = terms_and_conditions` (basis for `DOCUMENT_ROLE_MISMATCH`)
- `notice_lots.pc-f0f30f94-e22c-f111-813c-005056b64545-lot-1` / `lot_key = 1` (basis for `SINGLE_LOT_ALIGNS_WITH_NOTICE`)
- `contract_notices.main_category = "150000 - Consultancy"` (basis for `SERVICES_NOT_PRODUCTS`)

### Artifact refs

- Result contract: `result.json` (this workdir).
- Source markdown: inline in `document_extractions.extracted_markdown` for extraction id `30244ca2-7c79-4a7e-8e58-890221489095`.
