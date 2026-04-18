---
manifest_version: 1.0.0
skill_id: pdf-extraction/tender-specification
skill_version: 1.0.0
job_id: job-88b0218f-2353-4a24-90db-9eb787aff223
session_id: session-semantic-a59df10e5be8d40fd8e32bec85bbc5d62084b650bf06ec1f58f97f9117e555c5
execution_mode: contract
started_at: 2026-04-18T05:51:05Z
completed_at: 2026-04-18T05:52:01Z
outcome_class: partial_success_document_role_mismatch
items_processed: 1
items_skipped: 0
items_failed: 0
warnings_count: 6
agent_claimed_pages_visited: 1
agent_claimed_source_url: n/a_headless_contract_run
agent_claimed_fingerprint_state: n/a_headless_contract_run
agent_claimed_locale_state: en-GB_inferred_from_document
agent_claimed_currency_state: GBP_inferred_from_buyer_jurisdiction
---

# Run Manifest — pdf-extraction/tender-specification

## Summary

Ran the tender-specification skill against `document_extraction_id = 0703c5ca-2a21-4204-9a30-dfd1317f38a5` (raw document `raw-pc-dc1daee2-...-dacd26f8...`, title "6278 - PSQ Reference Appendices - Lot 2 - Provision of Water Risk Assessments", source hash `0841827fce7d3b4a1039949205aa17e100a2b72d02b486a9e87108889ea73a02`, page_count = 0 as persisted / 1 page of structural content observed, extraction_policy_version `2026.04.12`, document_role as persisted = `technical_specification`).

The document is misclassified upstream: its content is a pre-selection Reference Questionnaire (Appendix 1 reference form to be completed by applicant + referee, plus Appendix 2 cover letter to the referee). It is not a tender technical specification. It contains no scope of works, no product/goods requirements, no quantities, no KPIs, no technical standards (BS/ISO/EN/HSG/ACOP), and no CPV codes in-body. The only tender-spec-shaped signal is the lot header `6278 - Lot 2: Provision of Water Risk Assessments`, which identifies a single lot scope for the document.

Extraction produced one lot entry (Lot 2, Provision of Water Risk Assessments) with evidence anchors, six referee-assessment criteria captured under `evaluation_criteria` with a clear warning that they are supplier-reference dimensions and not tender award criteria, empty `product_requirements`, empty `technical_standards`, empty `cpv_codes`, and empty `product_requirement_seeds`. Six warnings explain the degraded output. `extraction_confidence_bps = 3500`. `extraction_scope = "full_document"`; `target_domain = "lots"`.

## Steps Taken

1. Read job parameters from the kickoff payload: `document_extraction_id`, `raw_document_id`, `process_id`, `notice_id`, `document_set_id`, `stage_run_id`, `extraction_scope=full_document`, `target_domain=lots`.
2. Loaded the Docling-produced markdown, sections JSON, and tables JSON for the referenced extraction from `state/state.db.document_extractions`.
3. Confirmed the persisted `document_role = technical_specification`, `extraction_policy_version = 2026.04.12`, and `source_content_hash = 0841827fce7d3b4a1039949205aa17e100a2b72d02b486a9e87108889ea73a02`. Took that hash as `extraction_metadata.source_markdown_hash`.
4. Read the parent notice (`pc-dc1daee2-...`) and process (`proc-pc-dc1daee2-...`) to understand cross-reference context only: 11-lot Building Services Framework, London Borough of Bexley, CPV 50700000-2 / 50750000-7 / 71315000-9 at the notice level.
5. Read `notice_lots` to cross-reference Lot 2 identity (`lot_key=2`, `title=Provision of Water Risk Assessments`). Per SKILL.md, treated these as cross-reference only — did not copy them into the extraction as though they were in-document truth.
6. Inspected the document body. Identified four structural sections: `header-0`, `Appendix 1` (`#/texts/0`), the Part B referee block (`#/texts/6`), and `Appendix 2` (`#/texts/36`). Three tables exist structurally but have empty header/row arrays in Docling output (`#/tables/0`, `#/tables/1`, `#/tables/2`) — the referee rating matrix is inlined into Part B text rather than rendered as a parsed table.
7. Extracted the one present tender-spec signal: the Lot 2 header `6278 - Lot 2: Provision of Water Risk Assessments`. Emitted a single lot entry with `lot_number="2"`, title, and a description that makes the document role explicit.
8. Extracted the six Part-B referee dimensions and rendered them under `evaluation_criteria` with their five-point rating scale as `weighting_text`. Added `EVALUATION_CRITERIA_ARE_REFEREE_ASSESSMENT_NOT_AWARD` warning so downstream scoring does not mistake them for tender award criteria.
9. Left `product_requirements`, `technical_standards`, `cpv_codes`, and `product_requirement_seeds` empty — no anchors exist for any of them. Added corresponding warnings.
10. Set `extraction_confidence_bps = 3500` to reflect the degraded extraction: lot identity is high-confidence, everything else is absent-evidence rather than present-evidence.
11. Wrote `result.json` to the workdir and validated it against `skills/pdf-extraction/tender-specification/result.schema.json` with `jsonschema` — schema valid.

## Decisions Made

- **Emitted one lot rather than zero.** The lot-2 header is explicit and cited verbatim in the document body, so emitting `lots: []` would have violated the golden assertion `lot_outputs_must_be_non_empty` and dropped the only durable tender-spec signal in the file. The lot entry is anchored to `6278 - Lot 2: Provision of Water Risk Assessments` at `#/texts/0`.
- **Did not copy notice-level CPV codes into the lot.** SKILL.md says existing notice-level lot metadata is for cross-reference only and must not be treated as in-document truth; those CPV codes (50700000-2, 50750000-7, 71315000-9) were not extracted from the document body. Recorded this choice in warning `NO_CPV_CODES_IN_DOCUMENT`.
- **Captured the referee criteria, but as referee-assessment not award criteria.** The schema field is `evaluation_criteria`, and the document does contain evaluation-shaped language. Dropping them would hide useful signal about what buyer quality dimensions apply to Lot 2. Keeping them without a warning would let a downstream scorer misuse them as award weightings. Resolved by keeping them with a prominent warning.
- **Did not guess product_requirement_seeds.** For water-risk-assessment work, plausible product requirements (ACOP L8, HSG274, BS 8580 risk-assessment standards, sampling equipment) could have been synthesized, but SKILL.md forbids guessing lot boundaries, standards, or product requirements without citing evidence anchors, and none of these strings appear in the document. `product_requirement_seeds` is empty.
- **Chose `extraction_scope = "full_document"` over `chunk_merged`.** The job parameters specify `extraction_scope=full_document`, the document fits well within the overlay budget (~15 KB of markdown), and a single pass was sufficient.
- **Confidence = 3500 bps (35%).** High confidence on lot identity (~95%) is pulled down by the emptiness of product-requirement, standards, and CPV fields that a real tender spec would populate. Reported as a single rollup; per-field confidences are in `field_confidences`.
- **Outcome class = `partial_success_document_role_mismatch`.** The skill ran cleanly and produced a schema-valid result, but the input did not match the expected document role. Downstream policy should either (a) remove this document from the tender-specification role, (b) trigger acquisition of the actual specification document, or (c) proceed with degraded Lot 2 identity only.

## Warnings

1. `DOCUMENT_ROLE_MISMATCH` — document is a PSQ reference form, not a tender specification (anchors cited in result.json).
2. `NO_PRODUCT_REQUIREMENTS_PRESENT` — no goods/services requirement text in the document; `product_requirement_seeds` empty by design.
3. `NO_TECHNICAL_STANDARDS_CITED` — no BS / ISO / EN / HSG274 / ACOP L8 / BS 8580 / other standard references.
4. `NO_CPV_CODES_IN_DOCUMENT` — no CPV codes in body; notice-level CPVs intentionally not copied in.
5. `EVALUATION_CRITERIA_ARE_REFEREE_ASSESSMENT_NOT_AWARD` — the six criteria captured are past-performance reference dimensions, not tender award criteria.
6. `SINGLE_LOT_SCOPE` — document explicitly scopes to Lot 2 only; no other lots from the 11-lot framework described.

## Evidence

Claims in this manifest are anchored to observable artefacts.

- Lot-2 identity, document role, and extraction scope:
  - Lot header anchor: `page:1 | "6278 - Lot 2: Provision of Water Risk Assessments"` (section `#/texts/0`, present in extracted markdown line 5).
  - Document-title field persisted on `document_extractions.document_title = "6278 - PSQ Reference Appendices - Lot 2 - Provision of Water Risk Assessments"`.
  - Sections JSON: `Appendix 1` (`#/texts/0`), Part B referee block (`#/texts/6`), `Appendix 2` (`#/texts/36`) — confirms there is no separate specification section.

- Reference-form shape (Appendix 1):
  - Part A rows anchored at `page:1 | "Name and contact details of applicant"`, `"Name and contact details of referee"`, `"Brief description of services provided and dates"`, `"Value p/a"`.
  - Part B rating framework anchored at `page:1 | "TO BE COMPLETED BY REFEREE AND RETURNED TO THE ADDRESS BELOW BY 18th May 2026 by 5pm"` and `"Please give your view of the quality of these services by placing ticks in the applicable columns"`.

- Referee criteria (all at `#/texts/6`, page:1):
  - Supply chain resilience — `"Supply chain resilience: Has the contractor provided a resilient and sustainable supply chain for required products, resources, and services?"`
  - Service Provision — `"Service Provision: Has the contractor consistently delivered the contracted services in line with the required KPIs?"`
  - Resource — `"Resource: Have the contractor's staff held the necessary qualifications, certifications, and experience for the tasks assigned and fully met the resource levels required to manage the contract."`
  - Value for Money — `"Value for Money: Has the contractor shown a transparent and well-evidenced approach to securing value for money"`
  - Contract Management — `"Contract Management: Has the contractor demonstrated effective contract management, including performance monitoring and oversight of subcontractors"`
  - Project Management — `"Project Management Has the contractor demonstrated competence in self-delivering small and medium water-risk-assessment works"`

- Appendix 2 referee cover letter:
  - Anchor `page:1 | "Appendix 2"` (section `#/texts/36`), followed by `"The company named on the attached Reference Questionnaire has applied for a Bexley Procurement exercise..."` and return instruction `"procurement@bexley.gov.uk before INSERT DATE & TIME"` — confirms administrative/questionnaire role.

- Result-contract fields:
  - `result.json.extraction_metadata.source_markdown_hash = "0841827fce7d3b4a1039949205aa17e100a2b72d02b486a9e87108889ea73a02"` matches `document_extractions.source_content_hash`.
  - `result.json.extraction_metadata.target_domain = "lots"` and `extraction_scope = "full_document"` match job parameters.
  - `result.json.extraction_metadata.policy_version = "2026.04.12"` matches `document_extractions.extraction_policy_version`.
  - Schema validation passed against `skills/pdf-extraction/tender-specification/result.schema.json` (`python3 -c "import json, jsonschema; jsonschema.validate(...)"` returned `schema valid`).

- Cross-reference context (not treated as in-document truth, per SKILL.md):
  - Parent notice `pc-dc1daee2-772e-f111-813c-005056b64545` main_category contains CPV 50700000-2, 50750000-7, 71315000-9 at the notice level.
  - `notice_lots` holds 11 lots for this framework; only Lot 2 is in scope for this document.
