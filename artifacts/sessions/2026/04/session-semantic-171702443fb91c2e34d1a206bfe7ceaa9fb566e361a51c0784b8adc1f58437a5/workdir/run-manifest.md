---
manifest_version: 1.0.0
skill_id: pdf-extraction/pricing-schedule
skill_version: 1.0.0
job_id: job-21168abc-9cae-4df4-a65c-b327a1209195
session_id: session-semantic-171702443fb91c2e34d1a206bfe7ceaa9fb566e361a51c0784b8adc1f58437a5
execution_mode: contract
started_at: 2026-04-18T00:05:10Z
completed_at: 2026-04-18T00:06:23Z
outcome_class: partial_blocked_missing_input_values
items_processed: 57
items_skipped: 0
items_failed: 0
warnings_count: 5
agent_claimed_pages_visited: 2
agent_claimed_source_url: document_extraction_id=3320074f-3b1f-4662-9c64-db917c4b7d20 (raw_document_id=raw-pc-3ba4e6b8-9cef-f011-813c-005056b64545-d66a199e74d66c60332eabd8c971de2d8901030b4725170ba98a13b96ba2105d)
agent_claimed_fingerprint_state: source_content_hash=cd2fb167f65bff0126abbbc02e2938c8ccf021d0ae761ff42001888b1f882af5 (content-addressed, docling)
agent_claimed_locale_state: en-GB
agent_claimed_currency_state: GBP (asserted explicitly in Instructions block: "Prices tendered are to be in Pounds Sterling (exclusive of VAT).")
---

## Summary

The bound document is ESPO framework 107_26 "Inspection & Maintenance of Indoor Gymnasium, Fitness & Sport Equipment and Outdoor Schools Play & Sport Equipment" — Appendix B Pricing Schedule. It is a tenderer-to-complete template. The structural payload contains two pricing basket tables (Inspection & Maintenance, p.1; Remedial Work, p.2) listing 57 service requirements across 9 sub-sections (INDOOR, FITNESS SUITE, OUTDOOR, ADDITIONAL VISITS, EMERGENCY VISITS, ROPES & TRAMPOLINES, RE-COVERING, NESTING TABLES, plus the basket total).

All 57 price cells in the yellow-highlighted tender column are empty in the source markdown. The "TOTAL TENDERED 'BASKET' PRICE" footer explicitly reads `0`, and the compliance meter reports "Number of service requirements priced = 0 / Fail". The overlay therefore emits 57 structural line items carrying the source description, section, unit basis, and evidence anchor, with `unit_rate_pence = 0` and `total_value_pence = 0` reflecting the observed absence of a quoted value (not a genuine zero-priced quote). Currency is set to GBP on explicit source statement. `pricing_summary.total_value_pence` is 0 matching the source footer; `lot_count` is 1 (framework-level — no lot decomposition present in this overlay). Five warnings surface the blank-cell state, lot-number ambiguity, repeated-header normalisation, section-header band treatment, and currency assertion.

## Steps Taken

1. Read bound SKILL.md (`pdf-extraction/pricing-schedule`, schema v1.0.0) and confirmed target_domain is `pricing` and expected output is `line_items` + `pricing_summary` + `field_confidences` + `warnings` + `evidence_anchors` + `extraction_metadata`.
2. Resolved the structural payload: the daemon stores Docling markdown in `document_extractions.extracted_markdown` keyed by `document_extraction_id=3320074f-3b1f-4662-9c64-db917c4b7d20`. Retrieved markdown (55,292 chars, 2 pages, 13 tables, 2 sections, `extraction_policy_version=2026.04.12`, `expected_overlays_json=["pdf-extraction/pricing-schedule"]`, `document_role=pricing_schedule`).
3. Parsed two pricing basket tables:
   - p.1: INSPECTION & MAINTENANCE basket (rows 1–42, further sub-grouped into INDOOR 1–20, FITNESS SUITE 21–26, OUTDOOR 27–35, ADDITIONAL VISITS 36–37, EMERGENCY VISITS 38–42).
   - p.2: REMEDIAL WORK basket (rows 43–57, sub-grouped into ROPES & TRAMPOLINES 43–47, RE-COVERING 48–54, NESTING TABLES 55–57) plus the basket total row.
4. Normalised repeated-header noise: the "Instructions for tenderers" cell is duplicated three times across columns in the Docling markdown — merged to a single instruction block for narrative only; no line-item de-duplication needed because the pricing basket table is a single-instance table.
5. Extracted rate basis (unit) from each row's description — every row states "price per X" (e.g., "per hall", "per machine", "per foot", "per rung", "each"). Set `quantity=1` on every row to represent the unit basis; this is an extracted rate card, not a quantity-bearing bill of quantities.
6. Observed that every yellow "Price" column cell is empty (the markdown row for item 1 ends in `| |` with no third-column value, and identical shape holds through item 57). Set `unit_rate_pence = 0` and `total_value_pence = 0` for every line item to faithfully reflect the blank cells, with a per-item note clarifying that `0` denotes an unfilled template cell.
7. Set `currency = "GBP"` based on explicit source text: "Prices tendered are to be in Pounds Sterling (exclusive of VAT)."
8. Set `pricing_summary.total_value_pence = 0` matching the footer cell "TOTAL TENDERED 'BASKET' PRICE = 0"; `lot_count = 1` (framework-level, no lot decomposition detected).
9. Emitted five warnings: `PRICING_CELLS_BLANK`, `LOT_NUMBER_NOT_ENUMERATED`, `HEADER_ROW_REPEATED_ACROSS_COLUMNS`, `SECTION_HEADER_BANDS_ARE_GROUP_LABELS`, `CURRENCY_ASSUMED_GBP`.
10. Captured 7 top-level evidence anchors (cover heading, appendix heading, currency statement, inspection basket header row, remedial basket header row, basket total footer, compliance meter) plus per-line-item anchors.
11. Wrote result.json in workdir and validated against `skills/pdf-extraction/pricing-schedule/result.schema.json` — passes all required-field constraints.

## Decisions Made

- **`unit_rate_pence=0`, `total_value_pence=0` on every line item, not omission.** The schema requires integer fields; omission was not permitted. Zero plus the `PRICING_CELLS_BLANK` warning + per-item note plus a very low (500 bps) field confidence on the rate fields is the honest representation of a blank tenderer template. This was chosen over inventing rates (prohibited by SKILL non-goals) or dropping rows (which would discard observable structure).
- **`quantity=1` on every line item.** Rows are rate-card entries ("price per X"), not bill-of-quantities rows. A neutral unit basis (`1` of the stated unit) was the least-misleading value; per-item note documents this framing. Confidence on `quantity` is low (3000 bps) to surface that it is a structural placeholder.
- **`lot_number="1"` on every line item with a `LOT_NUMBER_NOT_ENUMERATED` warning and low 2000 bps field confidence.** The pricing basket is framework-wide; no lot column or lot label is printed. Downstream overlay persistence will need to confirm against the tender-specification overlay if lot decomposition exists.
- **Two distinct `source_table_key` values, not one.** The Inspection & Maintenance basket and the Remedial Work basket are visually separated tables in the source (different column headers: "Price for Inspection & Maintenance Visit" vs "Price for Remedial work"). Preserving that split in `source_table_key` retains lineage for future quantity-merge / header-normalisation work.
- **Section-label rows (INDOOR, OUTDOOR, FITNESS SUITE, ADDITIONAL VISITS, EMERGENCY VISITS, ROPES & TRAMPOLINES, RE-COVERING, NESTING TABLES) are group headers, not line items.** They carry no item number and no description suitable for persistence as a service requirement. They are retained on each item as `evidence_anchor.section_ref`.
- **`extraction_confidence_bps=6500`.** Reflects high confidence in structure/descriptions/units/currency but low confidence in rate/total fields (which are blank) — a mid-range composite.
- **`outcome_class=partial_blocked_missing_input_values`.** The skill executed fully and emitted a schema-valid result, but the principal numeric fields (unit rates, totals) are absent from the source. This is not an agent failure; it is an input state that downstream consumers must be able to detect via the warning code.

## Warnings

- `PRICING_CELLS_BLANK` — All 57 tenderer price cells are empty; `0` values are placeholders, not quoted prices.
- `LOT_NUMBER_NOT_ENUMERATED` — No explicit lot numbering in the document; default `"1"`.
- `HEADER_ROW_REPEATED_ACROSS_COLUMNS` — Instructions block duplicated across three columns in Docling output; normalised.
- `SECTION_HEADER_BANDS_ARE_GROUP_LABELS` — Section bands retained as `section_ref`, not emitted as separate line_items.
- `CURRENCY_ASSUMED_GBP` — Currency asserted from explicit source statement.

See `result.json:warnings` for full messages and evidence refs.

## Evidence

Primary structural anchors (see `result.json:evidence_anchors`):

- `p.1` — "ESPO reference 107_26 — Inspection & Maintenance of Indoor Gymnasium, Fitness & Sport Equipment and Outdoor Schools Play & Sport Equipment" (cover_header).
- `p.1` — "Appendix B — PRICING SCHEDULE" (cover_header).
- `p.1` — "Prices tendered are to be in Pounds Sterling (exclusive of VAT)." (instructions block) → basis for `currency=GBP`.
- `p.1` — Inspection & Maintenance basket header row: `INSPECTION & MAINTENANCE | Service Requirement | Type of Area / Establishment | Price for Inspection & Maintenance Visit (£, excluding VAT)` (table_ref: `appendix_b_pricing_basket_inspection_maintenance`).
- `p.2` — Remedial Work basket header row: `REMEDIAL WORK | Service Requirement | Type of Remedial work | Price for Remedial work (£, excluding VAT)` (table_ref: `appendix_b_pricing_basket_remedial_work`).
- `p.2` — "TOTAL TENDERED 'BASKET' PRICE = 0" (table_ref: `appendix_b_basket_total`) → basis for `pricing_summary.total_value_pence=0`.
- `p.2` — "Number of service requirements priced = 0 / Fail" (table_ref: `compliance_meter`) → corroborates `PRICING_CELLS_BLANK` warning.

Per-line-item anchors are emitted on each `line_items[i].evidence_anchor` with `page_ref`, `anchor_text` (row number + description head), `section_ref` (INDOOR/OUTDOOR/FITNESS SUITE/…/NESTING TABLES), and `table_ref` (which of the two basket tables the row belongs to).

Artifact: `result.json` (written to session workdir, schema-valid against `skills/pdf-extraction/pricing-schedule/result.schema.json`).
