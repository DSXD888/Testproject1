---
manifest_version: 1
skill_id: pdf-extraction/pricing-schedule
skill_version: 1.0.0
job_id: job-c9d3a7a2-5e80-45c9-938b-54011fc5b2c8
session_id: session-semantic-3672e85436825c255b6fc12efb702a2f697919a360c8cb524dbb117ca55957da
execution_mode: contract
started_at: 2026-04-18T08:33:00Z
completed_at: 2026-04-18T08:34:00Z
outcome_class: success
items_processed: 57
items_skipped: 0
items_failed: 0
warnings_count: 8
agent_claimed_pages_visited: 0
agent_claimed_source_url: "sqlite:state.db/document_extractions/ad495aab-6c9d-4cde-a454-2daea79563f2"
agent_claimed_fingerprint_state: not_applicable
agent_claimed_locale_state: en-GB
agent_claimed_currency_state: GBP
---

# Run Manifest - pdf-extraction/pricing-schedule

## Summary

Extracted 57 line items from ESPO Framework 107_26 Appendix B pricing schedule — a blank, tenderer-to-complete rate-card template for "Inspection & Maintenance of Indoor Gymnasium, Fitness & Sport Equipment and Outdoor Schools Play & Sport Equipment" (document extraction `ad495aab-6c9d-4cde-a454-2daea79563f2`, 2 pages, 13 Docling table fragments, ~54 KB of markdown). Every priced cell in the source is blank; the document itself states `TOTAL TENDERED 'BASKET' PRICE = 0` and `Number of service requirements priced = 0 | Fail`, so `pricing_summary.total_value_pence = 0` and every `unit_rate_pence`/`total_value_pence = 0` reflects the observed empty cell rather than a quoted zero. One lot was emitted (`lot_number = "1"`, the framework itself) since the schedule declares a single framework and no multi-lot split. 22 evidence anchors cover cover-sheet metadata, the three repeated instruction/header panels, every section banner, the basket total, and the compliance counter. Eight warnings surface blank pricing cells, the basket-total sentinel, the rate-card-not-BoQ nature of the document, Docling's repeated-header artefact, the empty structural table payload, omitted section banner rows, a missing "per X" unit suffix on the multigym rows (12–15), and the VAT-rate absence. Overall extraction confidence 7500 bps.

## Steps Taken

1. Read the bound `SKILL.md` (`pdf-extraction/pricing-schedule@1.0.0`) and its companion `result.schema.json`, `golden.assertions.yaml`, `canary.inputs.jsonl`, and `checkpoint.schema.json` to confirm required fields, the single permitted `target_domain = "pricing"`, the `unit_rate_pence`/`total_value_pence` integer-pence contract, and the repeated-header-normalization flag.
2. Loaded job parameters: `document_extraction_id=ad495aab-6c9d-4cde-a454-2daea79563f2`, `target_domain=pricing`, `extraction_scope=full_document`, `process_id=proc-pc-3ba4e6b8-9cef-f011-813c-005056b64545`, `document_set_id=tds-pc-3ba4e6b8-9cef-f011-813c-005056b64545`, `stage_run_id=stage-run-43d02438-92da-4df3-bc14-2ff454300565-proc-pc-3ba4e6b8-9cef-f011-813c-005056b64545`.
3. Read `state/state.db::document_extractions` row `ad495aab-6c9d-4cde-a454-2daea79563f2` to obtain the Docling markdown (`extracted_markdown` column, 55292 characters / 55446 UTF-8 bytes), document title (`107_26-Appendix-B-Pricing-Schedule`), role (`pricing_schedule`), page count (2), section count (2), table count (13), engine (`docling 2.88.0`), policy version (`2026.04.12`). Computed SHA-256 of the retrieved markdown as `27d85e61044862740b57a13a3676c8937220628b076ea769be76624ca3aa0db6` for `extraction_metadata.source_markdown_hash`.
4. Read `document_extractions.extracted_sections_json` (two sections: `sheet: Cover sheet` p.1, `sheet: Pricing` p.1) and `extracted_tables_json` (13 fragments, 8 on p.1 and 5 on p.2, all with empty `headers` and `rows` — confirmed in `STRUCTURAL_TABLE_PAYLOAD_EMPTY` warning). Since structural rows are unavailable, parsed every priced row from the markdown directly and used a stable synthetic `source_table_key = "appendix_b_pricing_basket_inspection_maintenance"` instead of a `#/tables/N` index.
5. Walked the markdown section by section:
   - Cover sheet rows 1–32 — framework identity, period, cover-sheet submission instructions, Leicester address, submission deadline.
   - Instruction header row 34–35 — tenderer instructions repeated three times across three columns (Docling merged-cell artefact → `REPEATED_COLUMN_HEADERS_IN_DOCLING` warning; `repeated_header_normalized=true` on every line item).
   - `INSPECTION & MAINTENANCE` table header (row 40–42).
   - `INDOOR` banner → rows 1–20 (Nursery, School halls, Gymnasium, Sports Hall, Trampoline, Table Tennis, Dance Studio, Cricket Matting, High-Level Inspection, Traverse/Climbing Walls, 16/13/9/6-Station Multigyms, Single-Station Resistance, Cycles, Treadmills/Crosstrainers, Rower, Stepper).
   - `FITNESS SUITE` banner → rows 21–26 (Bands 1–6 by equipment-piece count).
   - `OUTDOOR` banner → rows 27–35 (Astro Turf/MUGA, Trim Trail, Fitness Machines, Primary Playground, Outdoor Table Tennis, Outdoor Cricket Matting, Goals, Mountain Bike, Kayak/Canoe).
   - `ADDITIONAL VISITS` banner → rows 36–37 (Scheduled General Inspection; Interim Visit).
   - `EMERGENCY VISITS` banner → rows 38–42 (call-outs: 1-day, 2-day, out-of-hours midweek, weekend, Bank Holiday).
   - `REMEDIAL WORK` header → `ROPES & TRAMPOLINES` → rows 43–47 (replacement ropes per foot, rope-ladder rungs new/existing, 13mm web bed, 100-spring set).
   - `RE-COVERING` → rows 48–54 (vaulting boxes hide/vinyl, vaulting buck, vaulting horse, wooden/metal frame benches, carpet-covered balance beams).
   - `NESTING TABLES` → rows 55–57 (Small/Medium/Large).
   - Basket total row ("TOTAL TENDERED 'BASKET' PRICE | 0") and the compliance counter panel on p.2 ("Number of service requirements priced | 0 | Fail") captured as top-level evidence anchors rather than line items.
6. For each of the 57 priced rows emitted `lot_number="1"`, `quantity=1`, `unit` derived from the "price per …" suffix (or `each` / `per set of 100` where stated), `unit_rate_pence=0`, `total_value_pence=0`, `currency="GBP"`, `repeated_header_normalized=true`, and an `evidence_anchor` with `page_ref="p.1"`, `anchor_text = "Row {n} | {description-prefix}"`, `section_ref` set to the governing banner (`INDOOR` / `FITNESS SUITE` / `OUTDOOR` / `ADDITIONAL VISITS` / `EMERGENCY VISITS` / `ROPES & TRAMPOLINES` / `RE-COVERING` / `NESTING TABLES`) and `table_ref = "appendix_b_pricing_basket_inspection_maintenance"`. Added two `notes` per row explaining (a) the observed-blank-cell rationale for `unit_rate_pence=0` and (b) the unit-basis rationale for `quantity=1`.
7. Populated the top-level `evidence_anchors` array with 22 anchors: cover-sheet metadata (6), instruction panel and column header (4), every section banner (9), basket total (1), compliance counter on p.2 (1), and the `INSPECTION & MAINTENANCE` / `REMEDIAL WORK` header rows (2 of the 9 banners). Each major section banner has at least one anchor and the on-p.2 compliance counter is anchored explicitly.
8. Recorded `field_confidences` with low values where the source is deliberately blank (`line_items.unit_rate_pence`, `line_items.total_value_pence` = 5000) or inferred (`line_items.quantity` = 6000 since the source has no procured volumes), medium values where the key is synthetic (`line_items.source_table_key` = 7500 because Docling rows are empty), and high values where the source is unambiguous (currency 9800, lot count 9500, item numbers 9800, descriptions 9500, unit basis 9000).
9. Emitted eight warnings covering every material divergence from an ideal BoQ extraction: blank pricing cells, zero basket total sentinel, rate-card-not-BoQ framing, repeated column headers, empty structural table payload, omitted section banners, missing "per X" suffix on multigym rows, VAT-exclusive-but-rate-not-stated.
10. Set `extraction_metadata.target_domain="pricing"`, `extraction_scope="full_document"` (from job params), `policy_version="2026.04.12"` (from `document_extractions.extraction_policy_version`), `source_markdown_hash="27d85e61…0db6"`, `items_examined=57`.
11. Validated the emitted `result.json` against `result.schema.json` (passed) and against `golden.assertions.yaml` (`min_line_items=2` ✓, `require_currency=true` ✓, `require_evidence_anchor_per_line_item=true` ✓, `require_repeated_header_normalization=true` ✓, all `required_metadata_fields` present ✓).

## Decisions Made

- **Single lot (`lot_number="1"`).** The schedule declares one framework ("ESPO Framework 107_26") and no lot boundaries inside the priced table. `pricing_summary.lot_count=1` follows the document's own structure rather than notice-lot rows, per the skill's "cross-reference only" guidance on lot metadata.
- **Blank cells encoded as `unit_rate_pence=0` / `total_value_pence=0` with explicit warning and low confidence (5000 bps).** The schema requires integer-pence fields ≥ 0 on every line item, and the document itself reports a basket total of `0`. Emitting the sentinel with a loud `PRICING_CELLS_BLANK_IN_TEMPLATE` warning and per-row `notes` preserves schema compliance while telling downstream consumers unambiguously that no tenderer prices are present. Omitting the required fields would be a contract violation; guessing non-zero rates would violate the skill's "do not guess rates, quantities, or totals when the source is incomplete" rule.
- **`quantity=1` as unit-basis encoding, not procured volume.** The source is a rate card of "price per hall / per visit / each / per set / per foot / per rung / per m² wall" rates. The schema requires `quantity: number` on every row; 1 is the most honest value for "one unit of the stated basis". A loud `NO_PROCURED_QUANTITIES_IN_SOURCE` warning plus a per-row `notes` entry preserves the distinction for downstream persistence.
- **`repeated_header_normalized=true` on every row.** The Docling markdown repeats the full "Instructions for tenderers" paragraph identically across all three columns in the top banner (a merged-cell rendering artefact), and the `INSPECTION & MAINTENANCE` / `REMEDIAL WORK` header rows are also three-column replicas. The skill's header normalization contract applies uniformly across the whole basket, so `repeated_header_normalized=true` is set for all 57 rows and logged once in the warnings.
- **Synthetic `source_table_key = "appendix_b_pricing_basket_inspection_maintenance"` instead of a Docling `#/tables/N` reference.** `extracted_tables_json` has 13 fragments but every one has empty `headers` and `rows`, so no single Docling index faithfully represents the pricing table and indices would not be stable across re-extractions. A stable synthetic key is robust to retry (per SKILL.md resume strategy: "preserve stable `source_table_key` naming instead of inventing new row identities from partial output").
- **Currency `GBP` from explicit source language.** The instructions state "Prices tendered are to be in Pounds Sterling (exclusive of VAT)" on three repeated columns of the banner row, so `currency` is set to `GBP` throughout with high confidence (9800 bps). VAT rate is intentionally not inferred and flagged via `VAT_EXCLUSIVE_STATED_VAT_RATE_NOT_STATED`.
- **`page_ref = "p.1"` on every line-item evidence anchor despite `page_count = 2`.** Docling places the "sheet: Pricing" section on page_start=1 and all 57 priced rows render within that section in the markdown. Table-fragment metadata indexes 5 of 13 fragments on page 2, but their content is empty; the compliance-counter table is the only anchor explicitly tagged `p.2`.
- **Section banners (`INDOOR`, `FITNESS SUITE`, …) preserved as `section_ref`, not as line items.** They are organising headings, not priced rows. Including them as line items would inflate the count and violate the "57 service requirements" count stated in the source ("Tenderers must provide pricing for a minimum of 40 out of the 57 service requirements"). Logged as `SECTION_BANNER_ROWS_OMITTED`.
- **Rows 12–15 multigym unit inferred as `per multigym`.** These rows state only "16/13/9/6 Station Multigym" without the "price per …" suffix carried by most other rows. The governing column header "Price for Inspection & Maintenance Visit (£, excluding VAT)" and adjacent row 16 ("Single Station Resistance Machine - price per machine") establish the per-unit rate-card basis, so `per multigym` is recorded and flagged via `MULTIGYM_ROWS_LACK_EXPLICIT_UNIT`.

## Warnings

- `PRICING_CELLS_BLANK_IN_TEMPLATE` — all 57 price cells are blank in the tenderer-to-complete template; `unit_rate_pence` and `total_value_pence` are 0 sentinels, not quoted zero prices.
- `BASKET_TOTAL_DOCUMENT_STATES_ZERO` — `TOTAL TENDERED 'BASKET' PRICE | 0` is the template sentinel, not a tendered total; `pricing_summary.total_value_pence` mirrors the observed value.
- `NO_PROCURED_QUANTITIES_IN_SOURCE` — the document is a rate card, not a bill of quantities; `quantity=1` encodes the unit basis.
- `REPEATED_COLUMN_HEADERS_IN_DOCLING` — Docling repeats the instruction banner identically across all three columns; deduplicated and all line items carry `repeated_header_normalized=true`.
- `STRUCTURAL_TABLE_PAYLOAD_EMPTY` — all 13 Docling table fragments have empty `headers`/`rows`; extraction performed on the markdown rendering; synthetic `source_table_key` used.
- `SECTION_BANNER_ROWS_OMITTED` — organising banners (INDOOR / FITNESS SUITE / OUTDOOR / ADDITIONAL VISITS / EMERGENCY VISITS / REMEDIAL WORK / ROPES & TRAMPOLINES / RE-COVERING / NESTING TABLES) are preserved only as `evidence_anchor.section_ref`.
- `MULTIGYM_ROWS_LACK_EXPLICIT_UNIT` — rows 12–15 omit the "price per …" suffix; unit inferred as "per multigym" from the governing column header and adjacent rows.
- `VAT_EXCLUSIVE_STATED_VAT_RATE_NOT_STATED` — "exclusive of VAT" stated but applicable VAT rate is not declared; downstream must resolve via the ITT.

## Evidence

Every claim in the output is anchored to substrings that appear verbatim in the Docling markdown for extraction `ad495aab-6c9d-4cde-a454-2daea79563f2` (title `107_26-Appendix-B-Pricing-Schedule`, role `pricing_schedule`, 2 pages, SHA-256 `27d85e61044862740b57a13a3676c8937220628b076ea769be76624ca3aa0db6`, also recorded in `result.json::extraction_metadata.source_markdown_hash`). Key anchors used:

- Framework identity: `ESPO reference 107_26 | Inspection & Maintenance of Indoor Gymnasium, Fitness & Sport Equipment and Outdoor Schools Play & Sport Equipment`.
- Framework period: `24 June 2026 – 31 January 2028 (with the option to extend up to 23 June 2030)`.
- Template marker: `THIS DOCUMENT IS TO BE COMPLETED BY THE TENDERER`.
- Submission deadline: `Closing date for submission of tender | 12 noon, 15 May 2026`.
- Pricing-evaluation methodology: `The maximum score available for the price element is 40%.` and `Tenderers must provide pricing for a minimum of 40 out of the 57 service requirements (70%) in the yellow highlighted cells.`
- Substitution rule: `For any of the (up to) 17 service requirements that compliant Tenderers cannot submit prices for, the median of all other compliant Tenderers' prices for the missing service requirements will be used in substitution, for evaluation purposes.`
- Currency rule: `Prices tendered are to be in Pounds Sterling (exclusive of VAT).`
- Column header: `INSPECTION & MAINTENANCE | Service Requirement | Type of Area / Establishment | Price for Inspection & Maintenance Visit (£, excluding VAT)`.
- Section banners (all recorded as `evidence_anchors` entries): `INDOOR`, `FITNESS SUITE  To include the following types of equipment: • Cardiovascular Machine (Treadmill, Bike, Cross Trainer, Rower etc.) …`, `OUTDOOR`, `ADDITIONAL VISITS`, `EMERGENCY VISITS`, `REMEDIAL WORK | REMEDIAL WORK | REMEDIAL WORK`, `ROPES & TRAMPOLINES`, `RE-COVERING`, `NESTING TABLES`.
- Representative row evidence: Row 5 `Trampoline - price per foldable trampoline`; Row 12 `16 Station Multigym`; Row 36 `Scheduled General Inspection Visit - price per visit (i.e. a non-urgent visit that can be scheduled in by the supplier)`; Row 43 `Replacement ropes for handrings and trapeze ropes - price per foot`; Row 47 `100 steel springs/cables for 77A trampolines - price set`; Row 57 `Large (over 760mm high) - price each`.
- Basket total sentinel: `TOTAL TENDERED 'BASKET' PRICE | TOTAL TENDERED 'BASKET' PRICE | 0`.
- Compliance counter on p.2: `Number of service requirements priced | 0 | Fail`.

All 57 line items carry a row-level `evidence_anchor` of the form `Row {n} | {description prefix}` pointing at the verbatim description text in the markdown and scoped to the governing section banner via `section_ref`. Structural provenance is further recorded in `extraction_metadata.source_markdown_hash` (SHA-256 of the source markdown) and `extraction_metadata.items_examined=57`.
