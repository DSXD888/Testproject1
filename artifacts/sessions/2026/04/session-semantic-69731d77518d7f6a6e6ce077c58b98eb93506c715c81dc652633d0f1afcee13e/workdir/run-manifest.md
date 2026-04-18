---
manifest_version: 1.0.0
skill_id: pdf-extraction/tender-specification
skill_version: 1.0.0
job_id: job-0ceaf291-26ec-4c47-849c-aff5cc1e3025
session_id: session-semantic-69731d77518d7f6a6e6ce077c58b98eb93506c715c81dc652633d0f1afcee13e
execution_mode: contract
started_at: 2026-04-18T04:40:00Z
completed_at: 2026-04-18T04:42:50Z
outcome_class: success
items_processed: 1
items_skipped: 0
items_failed: 0
warnings_count: 7
agent_claimed_pages_visited: 0
agent_claimed_source_url: null
agent_claimed_fingerprint_state: not_applicable
agent_claimed_locale_state: en_GB
agent_claimed_currency_state: GBP
---

# Run Manifest — pdf-extraction/tender-specification

## Summary

Single-pass tender-specification extraction over the full Docling markdown of document_extractions row `ae6317de-b5e4-404e-933e-05b6dc92873b` (title "6278 - Draft T&Cs - Lot 11", document_role `technical_specification`, extraction_policy_version `2026.04.12`, source_content_hash `5e4fe6531138a2d0ce6f5a2713fe5a5cd1d44ecf562cb8ecf62c386f4b76c258`) for process `proc-pc-dc1daee2-772e-f111-813c-005056b64545` (London Borough of Bexley – "6278 - Building Services Framework - Lots 1-11") and notice `pc-dc1daee2-772e-f111-813c-005056b64545`.

The document is the draft JCT Measured Term Contract 2024 Terms & Conditions scoped exclusively to Lot 11 – "Provision of Minor Disability Adaptations (Minor Building Works) in Privately Owned Properties". One lot was emitted (lot_number = "11"), carrying 11 contract-level requirements, 2 in-contract KPIs, 10 product-requirement entries aligned to the work-type list in §3, and 13 technical-standard references (contract form, statutes and regulations, competence/accreditation). 10 matching `product_requirement_seeds` were emitted for daemon-side `product_candidates` creation. No CPV codes are present in the document and none were guessed; cpv_codes arrays are empty on the lot and every seed. The emitted evaluation criteria are in-contract KPIs, not tender-award criteria, because this document is draft T&Cs and does not contain award evaluation machinery. extraction_metadata.target_domain = "lots", extraction_scope = "full_document".

## Steps Taken

1. Opened the Docling markdown from `document_extractions.extracted_markdown` for id `ae6317de-b5e4-404e-933e-05b6dc92873b` (length 4,430 bytes) and confirmed source_content_hash `5e4fe6531138a2d0ce6f5a2713fe5a5cd1d44ecf562cb8ecf62c386f4b76c258` and policy_version `2026.04.12` from the same row.
2. Cross-read the process context (`procurement_processes.id = proc-pc-...`) – buyer `London Borough of Bexley`, notice title `6278 - Building Services Framework - Lots 1-11` – and the notice-level `notice_lots` row for Lot 11 to understand the surrounding framework but treated the document as authoritative where the two disagree.
3. Walked the numbered sections of the draft T&Cs (Contract Form → §13 Contract Documents) and lifted one verbatim lot scope (§1 + §3 + §4), one contract period (§2), one priority-category table (§6), one payment block (§7), two KPIs (§8), nine statutory references (§9), the H&S block (§10), staff competence block (§11), waste management (§12), and the list of contract documents (§13).
4. Converted §3's bulleted "works include but are not limited to" list into 10 `product_requirements` entries on the lot and 10 matching `product_requirement_seeds`, each keyed `lot11-<slug>`, with the §3 bullet as its evidence_anchor.
5. Promoted the JCT 2024 contract form, Housing Grants / Construction & Regeneration Act 1996, and every item in §9/§10/§11 into `technical_standards`, each with a section_ref-scoped evidence_anchor. Did not emit any standard not cited in the document.
6. Classified the KPIs (§8) as in-contract evaluation criteria (KPI 1 – 90% minor adaptations; KPI 2 – 95% hospital discharge) and raised a warning that §8 references "three core KPIs" but only two are substantively defined in this extraction.
7. Raised a warning documenting the divergence between the document's "1 November 2026 – 31 October 2030, 4 Years Fixed Term, No extension option" (authoritative) and the existing `notice_lots` description for Lot 11 which states "1 Nov 2026 - 31 Oct 2028, with two one-year extension options".
8. Raised additional warnings for no CPV codes in the document, no formal award evaluation criteria (document is draft T&Cs not ITT), the Contract Specification / Pricing Schedule / Property Schedule being held as separate documents, the absence of page markers in the Docling output (page_count = 0) leading to `page_ref = "p.1"` across all anchors, and the lot-11-only scope of this document.
9. Wrote `result.json` in the workdir matching `skills/pdf-extraction/tender-specification/result.schema.json`; structurally validated all required keys and sub-object shapes.
10. Produced this run manifest for daemon finalization.

## Decisions Made

- **One lot, lot_number "11".** The document title (`1. Contract Title`) and the process notice both scope this draft T&Cs to Lot 11 only; no cross-lot inference was attempted, matching the skill's "do not guess lot boundaries without evidence" rule.
- **Document overrides notice_lots row on period and extension option.** Per skill input guidance ("do not treat pre-existing lot rows as authoritative truth if the tender document contradicts them"), the document's "4 Years Fixed Term, No extension option" is carried into the `requirements` and the discrepancy is surfaced as a warning (`NOTICE_LEVEL_LOT_METADATA_DIVERGES_FROM_DOCUMENT`).
- **Empty `cpv_codes` everywhere.** The document contains zero CPV references and the skill forbids guessing; daemon-side reconciliation or notice-level carry-over is the correct downstream behaviour, recorded as warning `NO_CPV_CODES_IN_DOCUMENT`.
- **KPIs modelled as evaluation_criteria.** The result-schema's `evaluation_criteria` shape is the closest legitimate home for §8's KPIs, but a warning `NO_FORMAL_AWARD_EVALUATION_CRITERIA` is raised because these are in-contract performance criteria, not award/MEAT criteria.
- **`page_ref = "p.1"` across all anchors.** `document_extractions.page_count = 0` and the Docling markdown has no page markers; section_ref carries the real locator and a warning (`NO_PAGE_NUMBERS_IN_DOCLING_OUTPUT`) is raised so downstream consumers do not treat "p.1" as load-bearing.
- **Confidence = 8600 bps.** High for the text that is present (lot scope, works list, standards, KPIs are all verbatim and anchored) but dampened by the absence of CPVs, formal award criteria, the referenced schedules, and page markers. Per-field confidences reflect the same shape (≥9000 bps for items taken verbatim, 8500 for KPIs-as-criteria, 2000 for cpv_codes).
- **`extraction_scope = "full_document"`.** Matches both the `extraction_scope` in the job parameters and the skill's contract for single-pass extraction of a short document; no chunk merging occurred.

## Warnings

1. `NO_CPV_CODES_IN_DOCUMENT` — no CPV references in the full document; cpv_codes arrays deliberately empty.
2. `NO_FORMAL_AWARD_EVALUATION_CRITERIA` — document is draft T&Cs, not ITT; items under `evaluation_criteria` are in-contract KPIs.
3. `KPI_ROW_MISSING_IN_SOURCE` — §8 references three KPIs but only KPI 1 and KPI 2 are substantively defined in the extracted markdown.
4. `SCHEDULE_BODIES_NOT_IN_THIS_DOCUMENT` — Contract Specification, Schedule of Rates / Pricing Schedule, and Property Schedule are referenced but held as separate schedule documents for this lot.
5. `NOTICE_LEVEL_LOT_METADATA_DIVERGES_FROM_DOCUMENT` — existing `notice_lots` row says "1 Nov 2026 – 31 Oct 2028 with two one-year extensions"; document says "1 November 2026 – 31 October 2030, 4 Years Fixed Term, No extension option" and is treated as authoritative.
6. `NO_PAGE_NUMBERS_IN_DOCLING_OUTPUT` — document_extractions.page_count = 0 and markdown carries no page markers; `page_ref = "p.1"` throughout, section_ref is the authoritative locator.
7. `DOCUMENT_SCOPE_LIMITED_TO_LOT_11` — this document covers only Lot 11 of Framework 6278; no other lots were inferred or emitted.

## Evidence

Every claim in this manifest and in `result.json` is anchored to one of the 17 `evidence_anchors` emitted in the result:

- Lot scope and title anchor to §1 `Contract Title` – "Measured Term Contract – Provision of Minor Disability Adaptations (minor building works)".
- Contract form and JCT 2024 standard ref anchor to the Contract Form block – "This Contract shall be executed under the JCT Measured Term Contract 2024 including: Conditions of Contract, Contract Particulars, Schedule of Rates / Pricing Schedule, Contract Specification, All associated contract documents".
- Contract period requirement and the `NOTICE_LEVEL_LOT_METADATA_DIVERGES_FROM_DOCUMENT` warning anchor to §2 – "Initial Contract Term: 1 November 2026 – 31 October 2030 … Duration: 4 Years Fixed Term … No extension option.".
- All 10 product_requirement_seeds (`lot11-grab-rails`, `lot11-mopstick-stair-rails`, `lot11-galvanised-hand-rails`, `lot11-half-steps`, `lot11-small-access-ramps`, `lot11-minor-electrical-power-sockets`, `lot11-furniture-moves-hospital-discharge`, `lot11-raised-height-toilets`, `lot11-shower-seats`, `lot11-key-safes`) anchor to §3 `Description of the Works` and cite the individual bullet from the works list as `anchor_text`.
- The priority-category requirement anchors to §6 `Priority Categories and Response Times` (Standard D / Urgent C / Emergency B / Hospital Discharge A response-completion table).
- Payment provision requirement and the Housing Grants, Construction and Regeneration Act 1996 standard anchor to §7 – "Payments shall be made in accordance with: Housing Grants, Construction and Regeneration Act 1996 … Section 4 of the JCT Measured Term Contract 2024 … Final date for payment: 30 days … invoices must be submitted within 6 months of order issue".
- KPI 1 and KPI 2 evaluation criteria anchor to §8 `Key Performance Indicators (KPIs)` – "Minimum 90% compliance" and "Minimum 95% compliance" respectively; the `KPI_ROW_MISSING_IN_SOURCE` warning cites §8's "monitored against three core Key Performance Indicators" opener.
- Building Regulations 2010, Building Safety Act 2022, Gas Safety (Installation and Use) Regulations, IET Wiring Regulations BS 7671, and the Regulatory Reform (Fire Safety) Order standards anchor to §9 `Statutory and Regulatory Compliance`.
- CDM Regulations 2015, Health and Safety at Work etc. Act 1974, Principal Contractor duty, and the SSIP-accredited (e.g. CHAS) requirement anchor to §9/§10 `Health and Safety`.
- CSCS and enhanced DBS standards anchor to §11 `Staff Competence and DBS`.
- Environmental Protection Act 1990 standard anchors to §12 `Waste Management`.
- The `SCHEDULE_BODIES_NOT_IN_THIS_DOCUMENT` warning anchors both to the Contract Form block (schedule names listed as parts of the contract) and to §13 `Contract Documents` (the full list of contract documents).
- The `NO_PAGE_NUMBERS_IN_DOCLING_OUTPUT` warning anchors to the daemon appendix field `document_extractions.page_count = 0`.
- The `NO_CPV_CODES_IN_DOCUMENT` warning cites the absence of any CPV references across the full markdown (4,430-byte extraction).
