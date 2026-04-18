---
manifest_version: 1
skill_id: pdf-extraction/tender-specification
skill_version: 1.0.0
job_id: job-3803e05a-a271-4140-8166-38bc5d3baa29
session_id: session-semantic-fd1f1eaffb6185880e951af584590a03b0c75390e0cc7b16c9a0b9aeed0cbfdb
execution_mode: contract
started_at: 2026-04-18T09:22:00Z
completed_at: 2026-04-18T09:27:19Z
outcome_class: success_with_warnings
items_processed: 9
items_skipped: 0
items_failed: 0
warnings_count: 6
agent_claimed_pages_visited: 0
agent_claimed_source_url: null
agent_claimed_fingerprint_state: not_applicable
agent_claimed_locale_state: not_applicable
agent_claimed_currency_state: not_applicable
---

## Summary

Extracted authoritative lot structure and per-lot vehicle specifications from ESPO framework 218_26 ("Specialist Vehicle Contract Hire/Leasing") Appendix B, which was bound to this skill with `document_role = technical_specification` despite self-titling as "Price Schedule (Appendix B)". Nine lots were identified; 39 vehicle specifications were emitted as product-requirement seeds. The Euro 6 emissions standard is the principal technical standard cited (on diesel specifications); compact-sweeper specifications use the softer phrase "diesel engine meeting current legislation", which is captured explicitly. Overall extraction confidence is 7800 bps, held below the optimistic ceiling because (a) CPV codes are inferred from notice-level signals rather than cited in the document, (b) the document is a price-schedule-in-technical-clothing and raw pricing cells are intentionally out of scope, and (c) LOT 6 has a source numbering anomaly (two rows labelled "Vehicle Specification 3") that required a stable local re-numbering decision.

## Steps Taken

1. Loaded the Docling-produced markdown (source_markdown_hash `3d41cb...653a06c6`, 186 tables, 12 sections, page_count 12) and the document's section map from `document_extractions.extracted_sections_json`. The source is an Excel workbook, so sections are "sheet: Title Sheet", "sheet: Instructions", "sheet: Lot 1..9", "sheet: Administrative Charges", each reporting page_start = page_end = 1 in Docling's output.
2. Read the notice-level cross-reference signals: `contract_notices.title = '218_26 Specialist Vehicle Contract Hire/Leasing'`, `buyer_name = 'ESPO'`, and `main_category` listing CPV 34114000-9 / 34121000-1 / 34134000-5. These were used as CPV priors on each lot, not as lot-level ground truth.
3. Read the single pre-existing `notice_lots` row (lot_number=1, "framework-wide scope") and deliberately did not treat it as authoritative, in line with the SKILL.md direction to treat pre-existing lot metadata as cross-reference only when the tender document contradicts it.
4. Scanned every "sheet: Lot N" section in the markdown for "LOT N - <title>" headings and per-spec "Vehicle Specification N <attributes>" rows. Identified nine lots and thirty-nine specs (LOT 1: 7 including info-only spec 7; LOT 2: 8; LOT 3: 3; LOT 4: 3; LOT 5: 5; LOT 6: 4; LOT 7: 6; LOT 8: 3; LOT 9: 0).
5. For each specification, extracted the attribute list (GVW band, powertrain, gearbox where stated, cab capacity, body/attachment, and vehicle weighting where stated) and paired it with a stable `seed_key` of form `218_26-lot-N-spec-M-<slug>`. This keeps seeds stable across future retries.
6. Emitted per-lot `evaluation_criteria` for the weighted-basket price rule (scored, max 20%) and the median-substitution rule, both citing the exact source strings.
7. Captured Euro 6 as the primary `technical_standards` reference on every lot where diesel specs cite it; for LOT 2 also captured "diesel engine meeting current legislation" as a separate entry, because the compact-sweeper specs deliberately avoid the Euro 6 citation.
8. Built `extraction_metadata` with `target_domain = lots`, `extraction_scope = full_document`, `source_markdown_hash` = sha256 of the extracted markdown, `policy_version = 2026.04.12`, and `document_role = technical_specification` (matching what the daemon set on the extraction row).
9. Validated the output against `skills/pdf-extraction/tender-specification/result.schema.json` with `jsonschema`; schema validation passed.

## Decisions Made

- Treated the Appendix B document as a lot-structure source even though it self-titles as a Price Schedule. Rationale: the document carries the authoritative 9-lot structure and the per-lot specification basket that the daemon needs to wire up product candidate seeds; pricing cells remain the province of `pdf-extraction/pricing-schedule`. Flagged via `DOCUMENT_ROLE_MISMATCH` warning so downstream readers can see the role/title tension without the extraction being rejected.
- Assigned CPV codes at both lot and seed level by combining notice-level priors (34114000, 34121000, 34134000) with standard UK CPV sub-codes for the vehicle family (e.g., 34144511 for RCVs, 34144430 for road sweepers, 34144420 for gritters, 34144500 for gully emptiers, 34121100/34121400 for buses/minibuses, 34134200 for tipper trucks, 34142000 for hook-loader chassis, 34144700 for skip trucks, 34114200/34114400 for mobile library/healthcare vehicles). Flagged via `CPV_CODES_INFERRED_NOT_CITED` so the daemon can preserve these as suggestions rather than overwrite any future lot-level CPVs coming from a true tender specification document.
- Renumbered the second "Vehicle Specification 3" in LOT 6 (the 3.5-4.5t electric dropside tipper) to spec 4 in the stable seed_key, because the source spreadsheet has no row labelled "Vehicle Specification 4" despite a "Vehicle Specification 4 Total Weighted Price" header row. Flagged via `LOT_6_SPEC_NUMBERING_ANOMALY`; a reviewer should confirm intent against the underlying Excel file.
- Emitted LOT 9 with empty `product_requirements` and `product_requirement_seeds` (and flagged `LOT_9_NO_SPECS_IN_THIS_DOCUMENT`) because the source explicitly defers Lot 9 specification to the Tender Submission Document, page 4. Inventing specs here would violate the "do not guess lot boundaries, standards, or product requirements without citing evidence anchors" constraint.
- Captured the compact-sweeper vs truck-mounted-sweeper standards distinction (LOT 2) as two distinct `technical_standards` entries rather than collapsing them to a single Euro 6 claim. This is structurally honest to the source and will matter later for landed-cost / compliance flows.
- Set page_ref to `"sheet: <name> / Page 1"` rather than numeric page ranges because Docling reports every sheet as page 1; flagged via `PAGE_REF_SHEET_BASED` so downstream consumers do not interpret "Page 1" as implying a single-page PDF.
- Chose `extraction_confidence_bps = 7800`: lot-structure identification is very high confidence (explicit headings, unambiguous), but the two caveats above (CPV inference, document-role mismatch, LOT 6 anomaly) pull the overall score below the 9000+ range that would be appropriate for a clean tender specification with cited CPVs.

## Warnings

- `DOCUMENT_ROLE_MISMATCH`: bound skill is `pdf-extraction/tender-specification`, document self-identifies as "Price Schedule (Appendix B)". Lot structure and specifications are extracted; pricing cells are left to `pdf-extraction/pricing-schedule`.
- `CPV_CODES_INFERRED_NOT_CITED`: CPV codes are inferred from notice-level CPV signals plus standard sub-codes for each vehicle family. Not cited at lot level in the document.
- `LOT_6_SPEC_NUMBERING_ANOMALY`: two rows labelled "Vehicle Specification 3" in LOT 6; second occurrence (3.5-4.5t electric) emitted as spec 4 to keep `seed_keys` stable.
- `LOT_9_NO_SPECS_IN_THIS_DOCUMENT`: LOT 9 product_requirements intentionally empty; source defers to Tender Submission Document, page 4.
- `STANDARD_REFERENCE_AMBIGUOUS_FOR_COMPACT_SWEEPERS`: LOT 2 compact sweepers (Specs 1-5) use "diesel engine meeting current legislation" not "Euro 6"; captured as a distinct standard entry alongside Euro 6.
- `PAGE_REF_SHEET_BASED`: source is an Excel workbook; Docling reports every sheet as page 1, so page_ref anchors cite the sheet name.

## Evidence

- `document_extractions.extracted_sections_json` (for id `06382574-87f6-4acd-aee9-016bae4090f1`): twelve sheet-level sections — `sheet: Title Sheet`, `sheet: Instructions`, `sheet: Lot 1` through `sheet: Lot 9`, `sheet: Administrative Charges`.
- Title sheet anchor (`#/groups/0`): "ESPO reference 218_26", "Specialist Vehicle Contract Hire/Leasing", "Framework Period 01 July 2026 to 30 June 2030", "PRICING SCHEDULE", "Closing date for submission of tender 12 noon, 15 May 2026".
- Instructions sheet anchor (`#/groups/1`): "Tenderers may bid for one, some or all of the Lots available", "Tendered pricing should be inclusive of the 1% ESPO framework fee", "Pricing per Lot will be evaluated based on the total weighted price for a 'basket' of various vehicle specifications", "60 and 84 months", "median price of all other compliant Tenderers' prices for the missing vehicle specifications will be used in substitution".
- LOT 1 anchor (`#/groups/2`): "LOT 1 - RCV's  Yellow cells are for completion to enable evaluation - both yellow cells must be completed per Vehicle Specification for a bid to be compliant." Seven vehicle specifications in source, six scored + one "FOR INFORMATION ONLY (NOT SCORED)" (Spec 7, 26t diesel with underground-bin crane).
- LOT 2 anchor (`#/groups/3`): "LOT 2 - Sweepers". Eight vehicle specifications; Specs 1-5 cite "Diesel engine meeting current legislation", Specs 6-7 cite "Diesel Euro 6 engine Hydrostatic gearbox", Spec 8 is electric.
- LOT 3 anchor (`#/groups/4`): "LOT 3 -  Winter maintenance vehicles". Three specs with "Snowplough attachment" and "single chamber gritting body" (6m³ diesel, 9m³ diesel, 6m³ electric demountable).
- LOT 4 anchor (`#/groups/5`): "LOT 4 - Gully emptiers". Three specs, all with "Permanently mounted body of a non-corrosive construction Medium volume tank capacity".
- LOT 5 anchor (`#/groups/6`): "LOT 5 - Buses and minibuses". Five specs across 16/17-seat minibus (diesel & electric), 24-seat midibus, 40-seat single-deck bus (diesel & electric).
- LOT 6 anchor (`#/groups/7`): "LOT 6 - Tippers". Four specs; two are both labelled "Vehicle Specification 3" (one 26t diesel dropside, one 3.5-4.5t electric dropside) — logged as `LOT_6_SPEC_NUMBERING_ANOMALY`.
- LOT 7 anchor (`#/groups/8`): "LOT 7 - Loaders". Six specs — three hook loaders (18t diesel 14t, 26t diesel 14t+, 18t electric 14t) and three skip loaders (18t diesel, 26t diesel, 18t electric) with extending arms.
- LOT 8 anchor (`#/groups/9`): "LOT 8 - Other specialist vehicles". Three specs — mobile library (diesel fitted), mobile healthcare (diesel non-fitted single room), mobile library (electric fitted).
- LOT 9 anchor (`#/groups/10`): "LOT 9 - Mixed specialist vehicle fleet  Pricing not required for this lot - see Tender Submission Document, page 4" — no vehicle basket in this document.
- Overall evaluation anchor: "Overall Total Weighted Basket Price (scored, max. score 20%)" repeated at the foot of each scored lot.
- Cross-reference evidence from `contract_notices` (`pc-218726bd-9dda-f011-813c-005056b64545`): `main_category = '34114000-9 - Specialist vehicles | 34121000-1 - Buses and coaches | 34134000-5 - Flatbed and Tipper trucks'` — used as CPV priors per the `CPV_CODES_INFERRED_NOT_CITED` warning.
- Result contract artifact: `result.json` (same working directory). Schema validation against `skills/pdf-extraction/tender-specification/result.schema.json` passed at completion time; lots=9, product_requirement_seeds=39, warnings=6, evidence_anchors=12, extraction_confidence_bps=7800.
