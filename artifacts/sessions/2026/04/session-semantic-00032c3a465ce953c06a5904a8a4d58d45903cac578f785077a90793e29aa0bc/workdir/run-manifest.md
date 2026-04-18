---
manifest_version: "1.0.0"
skill_id: "pdf-extraction/tender-specification"
skill_version: "1.0.0"
job_id: "job-cf02f1a6-e514-44d8-8331-93fb4b975f93"
session_id: "session-semantic-00032c3a465ce953c06a5904a8a4d58d45903cac578f785077a90793e29aa0bc"
execution_mode: "contract"
started_at: "2026-04-18T03:11:00Z"
completed_at: "2026-04-18T03:15:00Z"
outcome_class: "success"
items_processed: 1
items_skipped: 0
items_failed: 0
warnings_count: 5
agent_claimed_pages_visited: "p.1, p.3 (contents), p.4-5 (definitions), p.11-18 (clauses 2-8), p.19-21 (charges), p.22-28 (staff/management/insurance), p.29-31 (data processing), p.37-38 (modern slavery), p.45 (Schedule 1 stub), p.48 (Schedule 4), p.51 (Schedule 7)"
agent_claimed_source_url: "not_applicable_offline_pdf"
agent_claimed_fingerprint_state: "not_applicable_headless_contract"
agent_claimed_locale_state: "en-GB"
agent_claimed_currency_state: "GBP"
---

## Summary

Processed `Appendix-7-Draft-Contract` for Cambridgeshire County Council's Ely Library "Engage, Design and Fit Out" tender (process `proc-pc-19574d11-0929-f111-813c-005056b64545`). The document is a 59-page draft agreement template whose Schedule 1 (Specification) is a placeholder that refers out to the separately issued specification pack (Appendix 4). Produced one authoritative lot (Lot 1), four product-requirement seeds (furniture, signage, carpet, design services), twelve technical standards, and twenty-nine evidence anchors. Emitted five warnings covering the specification stub, absence of award criteria / CPV codes / contract value, and remaining `Error! Reference source not found.` cross-references in the draft.

## Steps Taken

1. Loaded `document_extractions` row `9765a520-2daa-4f86-8150-36dbdc2e58d7` (112,302 bytes of Docling markdown, 59 pages, 64 sections, 0 tables) from the daemon-provided extraction packet and confirmed `target_domain = "lots"` with `extraction_scope = "full_document"`.
2. Computed `source_markdown_hash = sha256:bc3b944c17736d82c70365e08a5134006e192948a566eaaf555c654e2439b7fa` from the extracted markdown.
3. Read the document front matter, definitions block (clause 1), operative clauses 2-29, and the Schedule block (Schedules 1-11). Confirmed the document is a draft contract template, not the Specification itself.
4. Derived a single lot (`lot_number = "1"`) from BACKGROUND (A) + clause 4 scope. Title `Ely Library Services - Engage, Design and Fit Out Contract` is lifted verbatim from the contract front matter. Description is grounded in clauses 4.3-4.7.
5. Grouped product-requirement seeds by the four delivery categories explicitly named at clause 4.6 (furniture, signage, carpet) and the design engagement at clause 4.3 (design services). Each seed carries a distinct `seed_key`.
6. Extracted twelve technical standards / regulatory obligations from clauses 4.2, 4.4, 4.7, 5.2, 5.4, 6.1, 6.2, 9.5, 19.1, 21.1, 21.4 and 29.1.
7. Recorded the absence of CPV codes, evaluation criteria, contract value and duration as explicit warnings rather than guessing — these belong to the ITT / award pack, not this draft contract.
8. Emitted `result.json` in the session workdir and validated it against `result.schema.json`.

## Decisions Made

- **Single lot, number "1".** The draft contract is a single-party agreement (Authority + Supplier) with a single set of Services and no lot partition. Stable `lot_number = "1"` matches the existing `notice_lots` row `pc-19574d11-0929-f111-813c-005056b64545-lot-1` and the prior extraction pass, preserving continuity for resume semantics per the SKILL Resume Strategy.
- **Empty `cpv_codes` and `evaluation_criteria` on the lot.** The draft contract states neither CPV classifications nor award criteria / weightings. Emitting `[]` plus explicit warnings (`CPV_CODES_NOT_IN_DOCUMENT`, `EVALUATION_CRITERIA_NOT_IN_DOCUMENT`) is faithful to the source; inventing CPVs from Services nouns or copying from the notice would violate the "cite evidence anchor" rule.
- **Four product-requirement seeds, categorical.** Clause 4.6 enumerates "furniture, signage, carpet and any associated items"; clause 4.3 adds design engagement. Produced one seed per category with stable `seed_key`s that match the prior extraction so downstream `product_candidates` can idempotently upsert on retries.
- **Technical standards captured as non-product obligations too.** Dementia-/autism-friendly design (clause 4.4), site-specific RAMS (clause 4.7), UK GDPR + DPA 2018 (clause 21), Modern Slavery Act 2015 (clause 29), Climate Charter (clause 6.2(b)), insurance minima (clause 19.1) and the "standard for electronic invoicing" (clause 9.5) are all emitted under `technical_standards` even when they frame the service delivery rather than a specific product - this is consistent with the SKILL outputs spec.
- **Schedule 1 stub flagged, not fabricated.** Page 45 has the heading `Schedule 1 Specification` with no requirement body and reviewer Comment [FB20] proposing warranty additions that are not yet in the contract. Recorded as `SCHEDULE_SPECIFICATION_STUB` warning and referred downstream to Appendix-4 Specification rather than guessing sizes, materials, quantities or warranty terms.
- **Confidence at 6800 bps (68%).** Strong signal on lot identity, scope, service categories, obligations and standards. Downgraded by the absence of CPV codes, award criteria, quantities, contract value, firm dates and the remaining internal cross-reference errors; those facts simply do not live in this document.

## Warnings

1. `SCHEDULE_SPECIFICATION_STUB` - Schedule 1 is an empty placeholder; authoritative product requirements must come from Appendix-4 Specification.
2. `EVALUATION_CRITERIA_NOT_IN_DOCUMENT` - award / scoring criteria with weightings are not in this contract; they belong to the ITT / Appendix-2a Award Criteria.
3. `CPV_CODES_NOT_IN_DOCUMENT` - no CPV codes stated; take classification from the notice / Find a Tender record and specification pack.
4. `CONTRACT_VALUE_AND_TERM_PLACEHOLDERS` - Commencement Date, Initial Term end, extension length and termination-payment thresholds are `[INSERT ...]` placeholders; value and duration must be resolved from the ITT / award pack.
5. `INTERNAL_CROSS_REFERENCE_ERRORS` - several `Error! Reference source not found.` and `Error! Bookmark not defined.` placeholders remain in the draft (clauses 4.1, 18.1, 21.2). Does not affect lot/seed extraction but is a downstream risk.

## Evidence

All major claims above are anchored to the following observable items in the Docling markdown for this document extraction (`document_extraction_id = 9765a520-2daa-4f86-8150-36dbdc2e58d7`, `source_markdown_hash = sha256:bc3b944c17736d82c70365e08a5134006e192948a566eaaf555c654e2439b7fa`):

- **Lot identity and scope (Lot 1).** Front matter p.1: `AGREEMENT RELATING TO ELY LIBRARY SERVICES - ENGAGE, DESIGN AND FIT OUT CONTRACT`. BACKGROUND (A), p.1: `... inviting prospective suppliers to submit proposals for the provision of design, furniture, signage, and carpet to refresh the public library offer in Ely library.` Authority identified p.1 as `CAMBRIDGESHIRE COUNTY COUNCIL`.
- **Service and goods scope (furniture + signage + carpet + design).** Clause 4.3(a)-(c) p.13 (site visit, stakeholder consultation, initial design, one feedback round); clause 4.5 p.13 (no manufacture before written design approval); clause 4.6 p.13 (deliver and install all furniture, signage, carpet and any associated items per approved design); clause 4.7 p.13 (RAMS specific to Ely Library with first-floor access constraints).
- **Quality and performance standards.** Clause 4.2(a) p.13 (`Best Industry Practice`); clause 4.2(b) p.13 (`best quality and free from defects in workmanship, installation and design`); clause 4.4 p.13 (`dementia friendly and autism friendly design principles`); clause 5.2 p.14 (health and safety Law + Authority Health and Safety Policy); clause 5.4 p.14 (equality Law); clause 6.1(a) p.15 (environmental Law); clause 6.2(b) p.15 (Climate Charter); clause 7.9 p.17 (manufacturer warranties, maintenance instructions, compliance certificates, in-person handover with on-site training); clause 9.5 p.19 (standard for electronic invoicing).
- **Commercial gates.** Clause 9.4 p.19 (three invoicing milestones: Design approval / Delivery of goods to site / Completion of installation and handover); clause 19.1 p.27 (Required Insurances - public liability GBP5m, employer's liability GBP10m, professional indemnity GBP2m, product liability GBP5m); clause 19.5 p.27 (6-year run-off cover).
- **Data and compliance obligations.** Clause 21.1 p.29 (Data Protection Legislation); clause 21.2 p.29 (Authority = Controller, Supplier = Processor); clause 21.4(g) p.30 (72-hour breach notification); clause 28 p.35 (Bribery Act 2010 / Prohibited Acts); clause 29.1(a) p.37 (Modern Slavery Act 2015).
- **Specification-stub warning source.** Schedule 1 heading on p.45 with no body; reviewer comment [FB20] proposing 12-month installation warranty, manufacturer warranties for furniture/signage/carpet, and a handover warranty pack.
- **Placeholders warning source.** Commencement Date `[INSERT DATE]` p.4; Initial Term `[INSERT DATE]` p.4; Termination Payment Default threshold `undisputed Charges of GBP[AMOUNT]` (Schedule 4, p.48); `Error! Reference source not found.` at clauses 4.1, 18.1 and 21.2.

Result artefact written to `result.json` in the session workdir; schema-validated against `pdf-extraction/tender-specification/result.schema.json`.
