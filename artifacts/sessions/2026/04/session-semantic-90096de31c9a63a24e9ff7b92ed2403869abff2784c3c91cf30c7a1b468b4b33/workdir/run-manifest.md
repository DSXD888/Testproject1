---
manifest_version: 1.0.0
skill_id: pdf-extraction/pricing-schedule
skill_version: 1.0.0
job_id: job-fe71d993-fdf5-4f5e-a577-17ed53992b13
session_id: session-semantic-90096de31c9a63a24e9ff7b92ed2403869abff2784c3c91cf30c7a1b468b4b33
execution_mode: contract
started_at: 2026-04-18T00:58:00Z
completed_at: 2026-04-18T01:00:10Z
outcome_class: success_with_warnings
items_processed: 24
items_skipped: 0
items_failed: 0
warnings_count: 8
agent_claimed_pages_visited: 3
agent_claimed_source_url: state.db://document_extractions/df59ffab-c7d6-4135-a9dd-390266fd6ac0
agent_claimed_fingerprint_state: stable
agent_claimed_locale_state: en-GB
agent_claimed_currency_state: inferred_GBP_not_declared
---

# Run Manifest — pdf-extraction/pricing-schedule

## Summary
Extracted pricing line items from a 3-page unpriced pricing-schedule template
(`Appendix-3a-Pricing-Schedule`) for process
`proc-pc-19574d11-0929-f111-813c-005056b64545` (Cambridgeshire County Council,
notice `pc-19574d11-0929-f111-813c-005056b64545`, lot 1 "25112 Library Services
Engage, Design and Fit out Contract - Ely Library"). The document is a bidder
fill-in template: every rate and total is blank or zero in the source. Recorded
24 line items (5 engage/design service rows, 12 fit-out/furniture rows, 7
indicative bidder add-on rows) with `unit_rate_pence = 0` and
`total_value_pence = 0`, flagged via a `BLANK_PRICING_TEMPLATE` warning and
downgraded `extraction_confidence_bps = 3000` so downstream gates do not treat
the zeros as real prices. `pricing_summary.total_value_pence = 0`,
`pricing_summary.currency = "GBP"` (inferred — document never states a
currency), `pricing_summary.lot_count = 1`.

## Steps Taken
1. Read the bound SKILL.md and the `result.schema.json` to confirm the required
   fields and the pence-integer money convention.
2. Loaded job parameters; identified the extraction row
   `df59ffab-c7d6-4135-a9dd-390266fd6ac0` and read `extracted_markdown`,
   `extracted_tables_json`, and metadata from `state.db/document_extractions`.
3. Computed the canonical `source_markdown_hash =
   f1a2c238ea34cb2ba9dd9bfd6b4a2b49a1f8d17dd4cbc0170c8314edab5f8a01`
   (SHA-256 of the 121,377-byte column value).
4. Inspected `extracted_tables_json`: 14 table fragments, all with empty
   `headers` and `rows` arrays. Concluded the entire table surface is only in
   the markdown body; table_key page mapping used the structural fragment
   page_start/page_end.
5. Walked the markdown page-by-page, collapsing Docling repeated-header
   artifacts (title bar duplicated across every cell; first data row identical
   to the header row). Classified three table bodies on page 2
   (engage/design, fit-out/furniture, bidder add-on) and a summary block on
   page 3.
6. Resolved lot metadata via `notice_lots` (single row, lot_number = 1, title
   `25112 Library Services Engage, Design and Fit out Contract - Ely Library`)
   and assigned `lot_number = "1"` to every line item.
7. Emitted 24 `line_items` with explicit evidence anchors (page + anchor_text),
   `repeated_header_normalized = true` for rows drawn from the two main
   repeated-header tables, `false` for the add-on section (the add-on block
   has no repeated-header artefact).
8. Built `pricing_summary`, `field_confidences`, `warnings`, `evidence_anchors`,
   and `extraction_metadata`; validated the result against
   `result.schema.json` (passed) and wrote `result.json` to the workdir.

## Decisions Made
- Treated the document as a template awaiting bidder completion rather than a
  priced schedule. All `unit_rate_pence` and `total_value_pence` are `0` (the
  lowest integer the schema allows), paired with `BLANK_PRICING_TEMPLATE` so a
  downstream gate sees "present but unpriced" instead of "priced at zero".
- Kept the 7 "bidder-supplied add-on" items as extracted `line_items` rather
  than dropping them, because they name specific required-category placeholders
  (Poster/Leaflet Display Unit, PC Desk, PC Chairs, Benching, Graphics,
  Signage). Flagged `INDICATIVE_ADDON_SECTION` and marked them
  `repeated_header_normalized = false` so a reviewer can distinguish them from
  the true rate-card rows.
- For the two engage/design rows with blank Quantity cells (`Developing design
  draft`, `Adjustments & final design`), wrote `quantity = 0` (schema requires
  a number) and down-weighted quantity confidence via
  `MISSING_QUANTITY_CELLS`. Did **not** guess 0.5 by analogy to the three
  siblings that are 0.5 days.
- Inferred `unit = "day"` for engage/design rows from the column header
  "Quantity day", and `unit = "each"` for fit-out rows because the fit-out
  header reads "Quantity (if applicable)" with no unit. Confidence on `unit`
  is deliberately mid-range (4500 bps).
- Inferred `currency = "GBP"` from UK local-authority context
  (Cambridgeshire County Council) but flagged the absence of an ISO code /
  symbol via `CURRENCY_NOT_DECLARED`. Currency field confidence is 3000 bps
  to signal to downstream landed-cost / margin gates that this was not an
  explicit declaration.
- Used `source_table_key` values matching the structural fragment keys
  (`#/tables/3` for engage/design, `#/tables/4` for fit-out, `#/tables/6` for
  add-ons) rather than inventing new identifiers, per the Resume Strategy
  guidance in SKILL.md.
- Set `extraction_confidence_bps = 3000` (not higher) to reflect:
  (a) fully unpriced source, (b) undeclared currency,
  (c) empty structural table payloads forcing markdown-only parsing.

## Warnings
See `result.json.warnings`. Emitted codes:
- `BLANK_PRICING_TEMPLATE` — every rate and total is zero/blank in the source.
- `CURRENCY_NOT_DECLARED` — GBP inferred from UK council context only.
- `REPEATED_HEADER_NOISE` — Docling duplicated lot titles across every cell
  and duplicated the first data row of each main table as a header echo.
- `STRUCTURAL_TABLE_ROWS_EMPTY` — all 14 `extracted_tables_json` entries have
  empty `headers` and `rows`; markdown-only extraction.
- `UNIT_INFERENCE` — fit-out column header "Quantity (if applicable)" does
  not state a unit; `each` inferred.
- `MISSING_QUANTITY_CELLS` — 2 engage/design rows have blank quantity cells.
- `INDICATIVE_ADDON_SECTION` — the "( for example)" sub-list is a bidder
  prompt, not a mandated rate-card.
- `DUPLICATE_SUMMARY_BLOCK` — two consecutive blank `|Total|||` tables at the
  foot of page 2, repeated in structure on page 3.

## Evidence
- `state.db/document_extractions/df59ffab-c7d6-4135-a9dd-390266fd6ac0`:
  `document_title = "Appendix-3a-Pricing-Schedule"`,
  `document_role = "pricing_schedule"`, `page_count = 3`,
  `extraction_policy_version = "2026.04.12"`,
  `extracted_markdown` length = 121,377 chars,
  `extracted_tables_json` = 14 empty fragments,
  `section_count = 3`, `table_count = 14`.
- `state.db/notice_lots` for `notice_id =
  pc-19574d11-0929-f111-813c-005056b64545`: single row, `lot_number = 1`,
  `title = "25112 Library Services Engage, Design and Fit out Contract - Ely
  Library"`.
- Markdown anchors used for every line item and for the summary roll-up are
  enumerated in `result.json.evidence_anchors` (13 anchors covering: the two
  lot banners, both main pricing table headers, both "Total for Evaluated
  Items | 0" subtotals, the add-on sub-header, the blank `|Total|||` blocks,
  the page-3 Required-Elements summary header, the two summary subtotal rows,
  and the "Grand Total for Evaluation | 0" row).
- Source-markdown SHA-256:
  `f1a2c238ea34cb2ba9dd9bfd6b4a2b49a1f8d17dd4cbc0170c8314edab5f8a01`.
- Result contract file: `result.json` in the session workdir; validated
  against `skills/pdf-extraction/pricing-schedule/result.schema.json`
  (Draft 2020-12) with zero errors.
