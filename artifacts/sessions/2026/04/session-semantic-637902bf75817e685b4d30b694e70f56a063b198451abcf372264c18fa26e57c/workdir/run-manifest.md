---
manifest_version: 1
skill_id: pdf-extraction/tender-specification
skill_version: 1.0.0
job_id: job-8a3878c7-d4d3-45cb-be49-6dc530f02849
session_id: session-semantic-637902bf75817e685b4d30b694e70f56a063b198451abcf372264c18fa26e57c
execution_mode: contract
started_at: 2026-04-18T00:00:00Z
completed_at: 2026-04-18T00:00:00Z
outcome_class: success_full
items_processed: 1
items_skipped: 0
items_failed: 0
warnings_count: 5
agent_claimed_pages_visited: 1
agent_claimed_source_url: not_applicable_offline_markdown_input
agent_claimed_fingerprint_state: not_applicable_offline
agent_claimed_locale_state: en_GB
agent_claimed_currency_state: GBP
---

# Run Manifest — pdf-extraction/tender-specification

## Summary

Extracted one authoritative lot (Lot 10) and ten product requirement seeds from the Docling markdown for `document_extractions.id = f6e5d4de-9ebc-4189-85b9-d4ba2102f5ce` ("6278 - Draft T&Cs - Lot 10"). The source document is the Draft Terms & Conditions for Lot 10 of London Borough of Bexley Building Services Framework 6278, executed under the JCT Measured Term Contract 2024 covering Planned, Preventative and Responsive Residential Property Maintenance and Compliance Services for ~353 residential properties (1 Nov 2026 to 31 Oct 2030, 4 years fixed, no extension). Output written to `result.json` with `target_domain = "lots"` and `extraction_scope = "full_document"`. Source markdown SHA-256 = `82ea89d3e0fbc896702a7bea28b99f6eee8aca0e465d7697cb6be20b5a91e7cb` (5,493 bytes).

`extraction_confidence_bps = 7200`. Five warnings raised covering missing evaluation criteria (this is the draft T&Cs, not the ITT/award pack), missing CPV codes (not in document body), absent detailed specification / SOR / property schedule attachments, parent-notice lot title divergence (notice says PPM-only, contract scope is broader), and the Employer's right to vary the property portfolio. Schema validation against `skills/pdf-extraction/tender-specification/result.schema.json` passes. No item-level failures; outcome class = `success_full`.

## Steps Taken

1. Resolved job context via `state.db`: confirmed `document_extractions.id`, `process_id`, `notice_id`, `document_set_id`, `document_role = technical_specification`, `extraction_policy_version = 2026.04.12`, `expected_overlays_json = ["pdf-extraction/tender-specification"]`.
2. Loaded the full extracted markdown (5,493 chars, single conceptual page; Docling reported 0 page_count and 1 empty-table). Computed source markdown SHA-256 = `82ea89d3e0fbc896702a7bea28b99f6eee8aca0e465d7697cb6be20b5a91e7cb` and embedded as `extraction_metadata.source_markdown_hash`.
3. Cross-referenced parent `contract_notices` row (Building Services Framework 6278, buyer = London Borough of Bexley) and `notice_lots` row 10 ("Provision of Planned & Preventative Residential Property Maintenance (PPM) & Remedial Repairs"). Did not import the notice-side title verbatim; the contract document's section 1 title was treated as authoritative for Lot 10 scope and the divergence was flagged as a warning rather than silently overwritten.
4. Walked the markdown sequentially section-by-section (Header / Contract Form / sections 1–14) and built one `lots[]` entry for Lot 10 with: `title` from section 1, narrative `description` synthesised from sections 1–4 + 14, `requirements[]` rolled up from sections 2–14 (each requirement carries its source section reference), empty `evaluation_criteria[]` and `cpv_codes[]` (with explicit warnings), `technical_standards[]` (15 entries — JCT MTC 2024, HGCRA 1996, Building Regs 2010, BSA 2022, CDM 2015, GSIUR 1998, BS 7671, RR(FS)O, EPA 1990, DPA, HSWA 1974, SSIP/CHAS, CSCS, enhanced DBS, EPC), and a lot-level `evidence_anchor` pointing at the Header / section 1 title.
5. Decomposed section 3 service lines plus section 11 (gas) and section 6 (priority categories) into ten `product_requirement_seeds[]` records, each with stable `seed_key` (`bexley-6278-lot10-<service>`), `lot_number = "10"`, candidate name, requirements rolled up from the relevant sections, technical_standards subset, empty `cpv_codes`, and an evidence anchor citing the originating section. Mirrored each seed into the `lots[0].product_requirements` array with `requirement_type` and `quantity_hint` (~353-property portfolio with explicit variation caveat). Service lines covered: responsive repairs/minor works; void property works; gas servicing/LGSR/24h breakdown; electrical inspection & testing; fire & Legionella compliance; emergency call-out; asbestos reporting & management; pest control; rubbish clearance & waste disposal; general building maintenance & engineering repairs.
6. Populated `evidence_anchors[]` (22 anchors) covering header, contract form, sections 1–14 and the priority/KPI tables. Populated `field_confidences{}` per top-level field and per seed. Set `extraction_confidence_bps = 7200` to reflect strong structural extraction but explicit downstream gaps (missing spec/SOR/property schedule attachments).
7. Produced 5 warnings: `EVALUATION_CRITERIA_NOT_IN_DOCUMENT`, `CPV_CODES_NOT_IN_DOCUMENT`, `DETAILED_SPECIFICATION_NOT_IN_DOCUMENT`, `PARENT_NOTICE_LOT_TITLE_DIVERGENCE`, `PROPERTY_PORTFOLIO_VARIATION_RIGHT`.
8. Validated `result.json` against `skills/pdf-extraction/tender-specification/result.schema.json` (passes; lots=1, seeds=10, anchors=22, warnings=5). Wrote `result.json` to the workdir for daemon pickup.

## Decisions Made

- **Single lot, not multi-lot.** The document is the Lot 10 T&Cs only; no other lot scopes are described inside it. Output a single `lots[]` entry rather than re-emitting the parent notice's 11 lots from outside-the-document context.
- **Lot 10 contract title preferred over notice lot title.** The contract scope (`Planned, Preventative and Responsive Residential Property Maintenance and Compliance Services`) is broader than the notice's `PPM & Remedial Repairs` title. Treated the contract section 1 title as authoritative and raised `PARENT_NOTICE_LOT_TITLE_DIVERGENCE` so downstream reconciliation is explicit.
- **No CPV in `lots[].cpv_codes`.** The document body contains no CPV code. The parent notice's framework-wide CPVs (50700000-2, 50750000-7, 71315000-9) were not back-filled into the lot output to avoid silent inference; the gap is recorded in a warning instead, with the notice-side codes captured there for the daemon's reconciliation step.
- **No `evaluation_criteria`.** KPIs in section 8 are post-award performance measures, not award/scoring criteria. Did not coerce KPIs into the `evaluation_criteria` slot; raised `EVALUATION_CRITERIA_NOT_IN_DOCUMENT` instead.
- **Service-line decomposition for seeds.** Although the document is service-led (no goods schedule), each major service line in section 3 plus the gas-specific obligations from section 11 was emitted as its own `product_requirement_seed` so downstream productisation/quoting has individually addressable units. Seeds use stable hyphenated `seed_key`s (`bexley-6278-lot10-<service>`) consistent with the schema's resume-stability requirement.
- **`page_ref = "1"` consistently.** Docling reported `page_count = 0` and a single conceptual table on page 1; the markdown is not paginated. Used `"1"` as `page_ref` and put the contract section number / heading in `section_ref` to keep anchors actionable.
- **`extraction_confidence_bps = 7200`.** Reflects high confidence on structural extraction (lot scope, requirements, technical standards, evidence anchors) but explicit penalty for the missing Specification, Schedule of Rates and Property Schedule attachments which are referenced (section 14) but not present in the markdown.

## Warnings

1. `EVALUATION_CRITERIA_NOT_IN_DOCUMENT` — This is the Draft Terms & Conditions, not the ITT/evaluation matrix. KPIs are performance measures, not award criteria. Evaluation criteria must be sourced from the ITT pack.
2. `CPV_CODES_NOT_IN_DOCUMENT` — No CPV codes present in document body. Parent notice carries 50700000-2, 50750000-7, 71315000-9 (framework-wide) which need to be filtered to Lot 10 from the contract notice / Find a Tender record.
3. `DETAILED_SPECIFICATION_NOT_IN_DOCUMENT` — Contract Specification, Schedule of Rates / Pricing Schedule and Property Schedule are referenced as forming part of the Contract (section 14) but not included in this extract. Required for productisation, pricing and any per-property task targeting.
4. `PARENT_NOTICE_LOT_TITLE_DIVERGENCE` — Parent notice says "Provision of Planned & Preventative Residential Property Maintenance (PPM) & Remedial Repairs". The contract document scopes Lot 10 more broadly (includes responsive repairs, void works, gas/electrical/fire/Legionella compliance, emergency call-outs). Contract document treated as authoritative for Lot 10 scope.
5. `PROPERTY_PORTFOLIO_VARIATION_RIGHT` — Section 4 reserves the Employer's right to vary the property count during the term, so the stated ~353 properties (230 + 120 + 3) is indicative only. Quantity hints on seeds are approximate.

## Evidence

All claims in `result.json` and this manifest are anchored to observable text in the source markdown. Key anchors:

- Header / Contract Form: `Draft Contract – JCT Measured Term Contract 2024`; `Employer ... London Borough of Bexley`; `This Contract shall be executed under the JCT Measured Term Contract 2024 including: Conditions of Contract; Contract Particulars; Schedule of Rates / Pricing Schedule; Contract Specification; All associated contract documents` — anchors `evidence_anchors[0..2]`, supports lot Contract Form and `JCT Measured Term Contract 2024` standard.
- Section 1 (Contract Title): `Measured Term Contract – Planned, Preventative and Responsive Residential Property Maintenance and Compliance Services` — anchor `evidence_anchors[3]`, supports lot title and the lot-level evidence anchor.
- Section 2 (Contract Period): `Initial Contract Term: 1 November 2026 – 31 October 2030 ... 4 Years Fixed Term ... No extension option.` — anchor `evidence_anchors[4]`, supports the term/duration requirement.
- Section 3 (Description of the Works): full service line list including responsive repairs, void works, gas/electrical/fire/Legionella, emergency call-outs, pest control, waste disposal, asbestos — anchors `evidence_anchors[5..6]`, supports every service-line seed.
- Section 4 (Property Portfolio): `230 Council-owned residential street properties; 120 privately leased properties managed by the Council; 3 hostels` and `The Employer reserves the right to vary the number of properties covered during the Contract period.` — anchors `evidence_anchors[7..8]`, supports quantity_hint values and warning #5.
- Section 5 (Orders and Work Instructions): `No works shall commence until a valid Order has been issued.` — anchor `evidence_anchors[9]`, supports the Order-driven instruction requirement and emergency/responsive seeds.
- Section 6 (Priority Categories and Response Times) table + emergency follow-on note — anchors `evidence_anchors[10..11]`, supports the priority/response requirements and the emergency call-out / void / responsive seeds.
- Section 7 (Payment Provisions): HGCRA 1996, JCT MTC 2024 §4, monthly applications, electronic invoices, 30-day final payment, 6-month invoice deadline — anchor `evidence_anchors[12]`, supports the payment requirement and the HGCRA 1996 standard.
- Section 8 (KPIs): KPI 1 (90% response/completion), KPI 2 (100% statutory compliance — gas, electrical, EPC; zero tolerance), KPI 3 (90% void turnaround; 10-day small / 28-day large) — anchors `evidence_anchors[13..15]`, supports the three KPI requirements and KPI cross-references in seeds.
- Section 9 (Statutory and Regulatory Compliance): Building Regs 2010, BSA 2022, CDM 2015, GSIUR, BS 7671, RR(FS)O, EPA, DPA — anchor `evidence_anchors[16]`, supports 8 of the 15 technical standards entries.
- Section 10 (Health and Safety): HSWA 1974, CDM 2015, Principal Contractor, SSIP / CHAS — anchor `evidence_anchors[17]`, supports the H&S and SSIP/CHAS standards.
- Section 11 (Gas Servicing and Compliance): annual servicing, annual safety inspections, LGSR, 24-hour breakdown service, GSIUR 1998 compliance — anchor `evidence_anchors[18]`, supports the gas-servicing seed and the GSIUR 1998 standard.
- Section 12 (Staff Competence and DBS): trade quals, CSCS, photo ID, enhanced DBS — anchor `evidence_anchors[19]`, supports the CSCS and enhanced DBS standards entries and the staff competence requirement.
- Section 13 (Waste Management): EPA 1990, registered waste carrier, waste transfer documentation — anchor `evidence_anchors[20]`, supports the waste-management seed and the EPA 1990 standard.
- Section 14 (Contract Documents): JCT MTC 2024 Conditions, Contract Particulars, Contract Specification, Schedule of Rates / Pricing Schedule, Property Schedule — anchor `evidence_anchors[21]`, supports requirement #16 and warning #3 (`DETAILED_SPECIFICATION_NOT_IN_DOCUMENT`).

Result-contract fields cited: `lots[0].lot_number = "10"`, `lots[0].title`, `lots[0].requirements[0..15]`, `lots[0].technical_standards[0..14]`, `lots[0].product_requirements[0..9]`, `lots[0].evidence_anchor`, `product_requirement_seeds[0..9].seed_key`, `extraction_confidence_bps = 7200`, `extraction_metadata.target_domain = "lots"`, `extraction_metadata.extraction_scope = "full_document"`, `extraction_metadata.source_markdown_hash = "sha256:82ea89d3e0fbc896702a7bea28b99f6eee8aca0e465d7697cb6be20b5a91e7cb"`, `extraction_metadata.policy_version = "2026.04.12"`, `extraction_metadata.document_role = "technical_specification"`. Daemon-observed fields available for the appendix: `document_extractions.id = f6e5d4de-9ebc-4189-85b9-d4ba2102f5ce`, `source_content_hash = 9858c077a46513efe8cb6fc26384c1d85ac173bcd05dadf8e67591c731378286`, `extraction_engine_version` and `extraction_policy_version = 2026.04.12` (matches manifest), `stage_run_id = stage-run-43d02438-92da-4df3-bc14-2ff454300565-proc-pc-dc1daee2-772e-f111-813c-005056b64545` (matches job parameters).
