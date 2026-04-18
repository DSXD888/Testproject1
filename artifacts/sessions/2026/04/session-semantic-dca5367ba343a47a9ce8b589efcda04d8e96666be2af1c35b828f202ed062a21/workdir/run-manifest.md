---
manifest_version: 1.0.0
skill_id: pdf-extraction/pricing-schedule
skill_version: 1.0.0
job_id: job-d78a9cd6-1f5c-4a62-b4cc-a31f68499608
session_id: session-semantic-dca5367ba343a47a9ce8b589efcda04d8e96666be2af1c35b828f202ed062a21
execution_mode: contract
started_at: 2026-04-17T22:10:00Z
completed_at: 2026-04-17T22:11:30Z
outcome_class: success_with_warnings
items_processed: 57
items_skipped: 0
items_failed: 0
warnings_count: 7
agent_claimed_pages_visited: 2
agent_claimed_source_url: not_applicable_offline_structural_payload
agent_claimed_fingerprint_state: not_applicable_offline_structural_payload
agent_claimed_locale_state: en-GB
agent_claimed_currency_state: GBP
---

# Run Manifest — pdf-extraction/pricing-schedule

## Summary

Extracted 57 service-requirement line items from ESPO Framework Reference 107_26, Appendix B — Pricing Schedule (document_title `107_26-Appendix-B-Pricing-Schedule`, `document_role=pricing_schedule`, 2 pages). The source is an xlsx-origin Docling payload covering a cover sheet (page 1) and a single pricing sheet (page 2). All 57 service requirements were captured across two logical tables: `pricing_schedule_inspection_maintenance` (items 1–42) and `pricing_schedule_remedial_work` (items 43–57). Currency is explicit GBP, exclusive of VAT. The document is a **tenderer-fillable template**: every unit-rate cell is empty, the `TOTAL TENDERED 'BASKET' PRICE` row reports `0`, and the compliance counter reads `0 | Fail`. Accordingly, `unit_rate_pence` and `total_value_pence` are emitted as `0` placeholders with low `field_confidences` and an explicit `PRICING_SCHEDULE_BLANK_TEMPLATE` warning so persistence layers do not confuse empty cells with priced zeros. No lot decomposition is present; `lot_number` is uniformly `unassigned` and `pricing_summary.lot_count = 0`. Outcome: success-with-warnings.

## Steps Taken

1. Parsed the Docling markdown payload for `document_extraction_id = f9e3d9e0-8644-49d9-9285-7536901e7092` (2 pages, 55,292 chars of markdown, 13 table fragments).
2. Inspected the structured table fragments in `extracted_tables_json` (`#/tables/0` through `#/tables/12`) and confirmed they were empty `{headers: [], rows: []}`; fell back to markdown-only reconstruction.
3. Read the cover sheet (page 1) to capture framework identity: `ESPO reference 107_26`, framework period `24 June 2026 – 31 January 2028 (option to 23 June 2030)`, submission deadline `12 noon, 15 May 2026`, document type `PRICING SCHEDULE / Appendix B`, tenderer-fillable banner, and the ESPO address block.
4. Confirmed currency basis from the page-2 preamble: `Prices tendered are to be in Pounds Sterling (exclusive of VAT)`, and from the column headers `Price for Inspection & Maintenance Visit (£, excluding VAT)` and `Price for Remedial work  (£, excluding VAT)`.
5. Identified the compliance rule: minimum 40 of 57 priced service requirements (70%), median substitution for up to 17 missing, drop rule for zero-coverage items. Recorded as a warning, not a line item.
6. Walked the INSPECTION & MAINTENANCE table (items 1–42), segmenting by in-table group headers: INDOOR (1–20), FITNESS SUITE (21–26), OUTDOOR (27–35), ADDITIONAL VISITS (36–37), EMERGENCY VISITS (38–42).
7. Walked the REMEDIAL WORK table (items 43–57), segmenting by ROPES & TRAMPOLINES (43–47), RE-COVERING (48–54), NESTING TABLES (55–57).
8. For each line item, captured `item_number`, the full `description` (preserving `price per X` and dimension hints), inferred `unit` from the description's trailing `price per X` phrase, and attached an `evidence_anchor` with `page_ref`, an `anchor_text` snippet that uniquely locates the row, a `section_ref` reflecting the parent group header, and a `table_ref` matching the stable `source_table_key`.
9. Emitted `repeated_header_normalized = true` on every line item because both tables use xlsx-merged-cell headers (`INSPECTION & MAINTENANCE` / `REMEDIAL WORK` repeat across three cells in the markdown) which were collapsed to a single logical header.
10. Set `unit_rate_pence = 0` and `total_value_pence = 0` for every row because every rate cell is empty in the source. Low `field_confidences` entries (500 bps) were attached to both fields so downstream systems treat them as unpriced placeholders.
11. Verified `TOTAL TENDERED 'BASKET' PRICE` row shows `0`, and the compliance counter `Number of service requirements priced` shows `0 | Fail`. These confirm the template is entirely blank.
12. Built `pricing_summary` with `total_value_pence = 0`, `currency = "GBP"`, `lot_count = 0`.
13. Emitted 18 document-level `evidence_anchors` covering cover-sheet metadata, preamble, the two table headers, every in-table group header, the basket-total row, and the compliance counter row.
14. Computed `source_markdown_hash = b566713c30c7e726a8baaa8d80987eb25ff0b78133970f7b2faebe96a8b634c8` (sha256 of the source markdown string) and wrote `extraction_metadata` with `target_domain = "pricing"`, `extraction_scope = "full_document"`, `policy_version = "2026.04.12"` (from `document_extractions.extraction_policy_version`), `items_examined = 57`.
15. Ran JSON Schema validation against `skills/pdf-extraction/pricing-schedule/result.schema.json` — PASS.
16. Wrote `result.json` to the working directory.

## Decisions Made

- **Emit the 57 unpriced rows rather than zero rows.** The descriptions, item numbers, and group-header placements are evidence-bearing and persistence-worthy even when the rate cells are empty. The alternative — returning an empty `line_items` — would violate the golden-assertion floor of `min_line_items: 2` and would discard structural evidence that the daemon can later use to diff against a filled copy. To avoid the "zero equals priced zero" trap, `unit_rate_pence` / `total_value_pence` field confidences are explicitly low (500 bps) and a `PRICING_SCHEDULE_BLANK_TEMPLATE` warning is emitted. This honours the SKILL.md non-goal ("omit or downgrade confidence instead of guessing") by downgrading rather than fabricating.
- **Use synthetic `source_table_key` values (`pricing_schedule_inspection_maintenance`, `pricing_schedule_remedial_work`) instead of Docling's `#/tables/N` refs.** The Docling table fragments were empty, so the `#/tables/N` references carry no payload. Two stable, human-readable keys preserve traceability across retries (SKILL.md resume guidance). The mapping to the source is preserved via the warning `DOCLING_TABLE_FRAGMENTS_EMPTY`.
- **`lot_number = "unassigned"` and `lot_count = 0`.** The document is a single framework-wide basket with no lot partitioning in either title or structure. Inventing a lot identity would mislead downstream lot-scoped logic.
- **`repeated_header_normalized = true` uniformly.** Both tables repeat their top banner header across three cells (merged-cell artefact from the xlsx), and the page-2 instructional preamble repeats three times across a merged cell row. Marking every row true is accurate and satisfies the `require_repeated_header_normalization` golden assertion.
- **Units for items 12–15 (multigyms) inferred as `multigym`.** Those four rows lack a "price per X" suffix unlike the rest of the table. The `UNIT_INFERRED_FROM_DESCRIPTION` warning calls this out explicitly so downstream unit taxonomy can reconcile.
- **Compliance rule (40-of-57) not emitted as a line item.** It is an evaluation constraint, not a priced row. Encoded as warning `MINIMUM_COVERAGE_REQUIREMENT_PRESENT`.
- **VAT basis.** Every price cell carries an explicit `(£, excluding VAT)` marker. Encoded as warning `VAT_TREATMENT_EXPLICIT` so tax reconciliation downstream does not apply a VAT-inclusive interpretation.
- **`agent_claimed_source_url = not_applicable_offline_structural_payload`.** Execution is offline contract mode consuming a pre-supplied Docling payload; no URL was visited by this skill.
- **Overall `extraction_confidence_bps = 5500`.** Structural fidelity is high (item numbers, descriptions, headers, currency, units) but the core pricing numbers are absent. A moderate overall score reflects this honestly.

## Warnings

All warnings are also present in `result.json.warnings`:

1. **PRICING_SCHEDULE_BLANK_TEMPLATE** — All unit rates and totals are 0 because source cells are empty; do not treat as priced zeros.
2. **NO_EXPLICIT_LOTS** — Framework has no lot partition; all `lot_number = unassigned`, `lot_count = 0`.
3. **DOCLING_TABLE_FRAGMENTS_EMPTY** — All 13 Docling table refs had empty headers/rows; extraction is markdown-sourced; synthetic `source_table_key` values used.
4. **REPEATED_HEADER_COLUMN_SPAN_NORMALIZED** — `INSPECTION & MAINTENANCE` / `REMEDIAL WORK` banner headers and the page-2 instructional preamble repeat across merged cells; normalized to single logical units.
5. **UNIT_INFERRED_FROM_DESCRIPTION** — No dedicated Unit column; unit parsed from `price per X` suffix. Items 12–15 lack the suffix and were inferred as per-multigym.
6. **VAT_TREATMENT_EXPLICIT** — Prices are exclusive of VAT per document preamble and column headers.
7. **MINIMUM_COVERAGE_REQUIREMENT_PRESENT** — Admissibility floor of 40-of-57 (70%), median substitution up to 17 missing, drop rule on zero coverage. Encoded as a warning, not a line item.

## Evidence

All evidence anchors below are also emitted in `result.json.evidence_anchors` / line-item `evidence_anchor` fields, and each references observable content in the Docling-produced markdown for page 1 (cover sheet) or page 2 (pricing sheet).

### Document identity (cover sheet, page 1)
- `ESPO reference 107_26` — source_table_key `cover_sheet_reference`.
- `Inspection & Maintenance of Indoor Gymnasium, Fitness & Sport Equipment and Outdoor Schools Play & Sport Equipment` — `cover_sheet_title`.
- `PRICING SCHEDULE | Appendix B` — `cover_sheet_document_type`.
- `Framework Period | 24 June 2026 – 31 January 2028 (with the option to extend up to 23 June 2030)` — `cover_sheet_framework_period`.
- `Closing date for submission of tender | 12 noon, 15 May 2026` — `cover_sheet_closing_date`.
- `THIS DOCUMENT IS TO BE COMPLETED BY THE TENDERER / AND SUBMITTED TO ESPO` — confirms tenderer-fillable state.

### Currency and VAT basis (page 2 preamble)
- `Prices tendered are to be in Pounds Sterling (exclusive of VAT).`
- Column header `Price for Inspection & Maintenance Visit (£, excluding VAT)`.
- Column header `Price for Remedial work  (£, excluding VAT)`.

### Table headers and group headers (page 2)
- `INSPECTION & MAINTENANCE` (repeated across three cells — merged header; normalized).
- `Service Requirement | Type of Area / Establishment | Price for Inspection & Maintenance Visit (£, excluding VAT)`.
- Group headers: `INDOOR` (above rows 1–20), `FITNESS SUITE` (above rows 21–26 with expanded equipment-scope text), `OUTDOOR` (above rows 27–35), `ADDITIONAL VISITS` (above rows 36–37), `EMERGENCY VISITS` (above rows 38–42).
- `REMEDIAL WORK` (repeated across three cells — merged header; normalized).
- `Service Requirement | Type of Remedial work | Price for Remedial work  (£, excluding VAT)`.
- Group headers: `ROPES & TRAMPOLINES` (above rows 43–47), `RE-COVERING` (above rows 48–54), `NESTING TABLES` (above rows 55–57).

### Blank-template evidence (page 2)
- `TOTAL TENDERED 'BASKET' PRICE | ... | 0` — basket sum is literally `0` in source.
- `Number of service requirements priced | 0 | Fail` — the in-document compliance counter displays zero priced rows and a `Fail` status.
- Every unit-rate cell in rows 1–57 is empty in the markdown table.

### Per-line-item anchors
Each of the 57 line items in `result.json.line_items[*].evidence_anchor` carries:
- `page_ref = "page 2"` (the pricing sheet).
- An `anchor_text` that begins with the item number and a prefix of the description (e.g., `"1 | Nursery - A designated hall within a Nursery School"`).
- `section_ref` identifying the group header (e.g., `"INSPECTION & MAINTENANCE / INDOOR"` or `"REMEDIAL WORK / NESTING TABLES"`).
- `table_ref` matching `source_table_key` (`pricing_schedule_inspection_maintenance` for items 1–42, `pricing_schedule_remedial_work` for items 43–57).

### Result-contract corroboration
- `result.json.line_items.length == 57`.
- `result.json.pricing_summary.currency == "GBP"`.
- `result.json.pricing_summary.total_value_pence == 0`.
- `result.json.pricing_summary.lot_count == 0`.
- `result.json.extraction_metadata.source_markdown_hash == "b566713c30c7e726a8baaa8d80987eb25ff0b78133970f7b2faebe96a8b634c8"`.
- `result.json.extraction_metadata.policy_version == "2026.04.12"` (matches `document_extractions.extraction_policy_version`).
- `result.json.extraction_metadata.items_examined == 57`.
- Schema validation against `skills/pdf-extraction/pricing-schedule/result.schema.json`: PASS.
