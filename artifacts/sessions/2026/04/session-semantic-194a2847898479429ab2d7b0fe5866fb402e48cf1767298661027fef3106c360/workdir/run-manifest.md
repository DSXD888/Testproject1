---
manifest_version: "1.0.0"
skill_id: "pdf-extraction/pricing-schedule"
skill_version: "1.0.0"
job_id: "job-488ac2a7-9dcc-488e-b35e-47622f4d6e9e"
session_id: "session-semantic-194a2847898479429ab2d7b0fe5866fb402e48cf1767298661027fef3106c360"
execution_mode: "contract"
started_at: "2026-04-18T00:00:00Z"
completed_at: "2026-04-18T00:00:00Z"
outcome_class: "success_with_warnings"
items_processed: 24
items_skipped: 0
items_failed: 0
warnings_count: 7
agent_claimed_pages_visited: 3
agent_claimed_source_url: "https://procontract.due-north.com/RfxResponse/Index?rfxId=28b81c96-0929-f111-813c-005056b64545"
agent_claimed_fingerprint_state: "not_applicable_offline_contract_extraction"
agent_claimed_locale_state: "en_GB"
agent_claimed_currency_state: "GBP_assumed_from_narrative_only"
---

# Run Manifest — pdf-extraction/pricing-schedule for job-488ac2a7

## Summary

Ran the `pdf-extraction/pricing-schedule` overlay (v1.0.0) in contract mode against the Docling-produced markdown for document extraction `74981553-c1e7-4c19-abbb-877535fd7e31` (raw document `raw-pc-19574d11-...-41f`, source file `Appendix-3a-Pricing-Schedule.xlsx`, Docling 2.88.0, policy version `2026.04.12`, 3 pages, 14 structural tables). Emitted 24 line items across two priceable sections (5 engage/design rows, 12 specified fit-out rows, 7 bidder-example rows) plus 13 evidence anchors covering tab headers, section headers, subtotals, and the grand total. The source is a **blank bidder-completable pricing schedule** — every unit rate, line total, section subtotal, and grand total is empty in the spreadsheet. All `unit_rate_pence` and `total_value_pence` fields are recorded as `0` and the top-level `pricing_summary.total_value_pence` is also `0`; warning `PRICING_TEMPLATE_BLANK` is raised so that downstream persistence does not treat these as real prices. Currency is recorded as `GBP` based on the single narrative mention of "£95k" in the Statement of Intention and the UK-public-sector context (Cambridgeshire County Council, Ely Library); warning `CURRENCY_ASSUMED` captures this. Lot number `25112` was assigned to all priceable rows because the priceable tabs (pages 2-3) are headed `25112 Library Services provision for Engage, Design, and Fit out Contract - Ely Library`; warning `PROJECT_CODE_MISMATCH` flags that page 1 uses the different project code `23105`. The overall `extraction_confidence_bps` is 7800 — description-level content is high-confidence (rows are clearly anchored in the Docling tables), but price and currency confidence is bounded by the blank-template condition.

## Steps Taken

1. Located the structural payload in `state.db/document_extractions` keyed by `id = 74981553-c1e7-4c19-abbb-877535fd7e31` (page_count=3, table_count=14, section_count=3, 121377-byte `extracted_markdown`). Confirmed `extraction_engine = docling`, version `2.88.0`, `extraction_policy_version = 2026.04.12`, `document_role = pricing_schedule`.
2. Inspected the raw source metadata: `raw_documents.storage_path = /Users/.../artifacts/downloads/procontract/782746/Appendix-3a-Pricing-Schedule.xlsx`, `content_type = application/vnd.openxmlformats-officedocument.spreadsheetml.sheet`. Source is an xlsx, not a native PDF — the pricing-schedule overlay still applies because we operate on Docling markdown.
3. Dumped `extracted_markdown` to `/tmp/pricing_md.md`. Observed massive cell duplication (single logical cells replicated ~5x across columns and 13x down rows in the intro tab). Applied a consecutive-duplicate dedupe pass to recover the intended single-column-wide rendering (`/tmp/pricing_dedup.md`, 25026 bytes). Marked every emitted line item with `repeated_header_normalized: true`.
4. Walked the deduplicated markdown linearly, mapping each logical section to a `table_ref` in the Docling table index:
   - page 1: `#/tables/0` (tab title "23105..."), `#/tables/1` (Statement of Intention narrative).
   - page 2: `#/tables/2` (Ely Library tab header "25112..."), `#/tables/3` (Engage/Design pricing table), `#/tables/4` (Fit-out pricing table), `#/tables/5` (fit-out subtotal "Total for Evaluated Items | 0"), `#/tables/7` (bidder-example list), `#/tables/8` (tab footer "Total |").
   - page 3: `#/tables/10` (summary header "Required Elements | Total cost"), `#/tables/11` (engage/design evaluation subtotal "| 0"), `#/tables/12` (fit-out evaluation subtotal "| 0"), `#/tables/13` ("Grand Total for Evaluation | 0").
5. Emitted line items ED-01..ED-05 for the engage/design pricing section, preserving the stated fractional quantities (0.5 day) for Onsite Visit, Staff and stakeholder engagement, and Engagement on design. Two rows ("Developing design draft", "Adjustments & final design") have no quantity in the source; these are emitted with `quantity: 0` and flagged by warning `BLANK_QUANTITY_CELL`.
6. Emitted line items FO-01..FO-12 for the specified fit-out rows, preserving every stated quantity (500, 12, 2, 18, 25, 1, 4, 2, 2, 1, 12, 1) and incorporating each row's "Minimum standard" text into the item's `notes` so that downstream consumers see the exact governing spec. Unit is `sqm` for the carpet line (anchored in-cell) and `day` for engage/design (anchored in the column header "Quantity day"); for the other fit-out rows, no unit column exists in the source, so unit is recorded as `each` and `UNIT_INFERRED` warning is raised.
7. Emitted example rows EX-01..EX-07 for the "bidders to include any and all other items..." block (Poster Display Unit, Leaflet Display Unit, PC Desk, PC Chairs, Benching Managers office, Graphics, Signage). These are illustrative and have `quantity: 0, unit_rate_pence: 0, total_value_pence: 0` with notes explaining their role.
8. Set `pricing_summary.total_value_pence = 0, currency = GBP, lot_count = 1`. Computed SHA-256 of the raw Docling markdown (`0e4943f6f37ded757bd12d088dcf558f8e9341378bda73c6d00185682d900afb`) as `extraction_metadata.source_markdown_hash`. Populated `target_domain = pricing`, `extraction_scope = full_document`, `policy_version = 2026.04.12`, `items_examined = 24`.
9. Validated the emitted `result.json` against `skills/pdf-extraction/pricing-schedule/result.schema.json` with `jsonschema.validate`. No violations.

## Decisions Made

- **All unit rates and line totals set to 0 pence rather than omitted**: the schema requires `unit_rate_pence` and `total_value_pence` as `integer, minimum 0` on every line item. The source cells are genuinely blank (bidder-completable). `0` is truthful in the sense that nothing was quoted, and the `PRICING_TEMPLATE_BLANK` warning explicitly prevents downstream consumers from treating these as real £0 prices.
- **Included the 7 "for example" rows as line items rather than discarding them**: the overlay's contract is to surface evidence-bearing rows, and these rows are explicitly listed in the spreadsheet as the bidder-augmentation bucket. Flagging them as `EX-*` item numbers and including the "( for example)" suffix in the description makes their illustrative nature obvious.
- **Lot number chosen as `25112`, not `23105`**: pages 2-3 (the pages where priceable rows live) are headed `25112 Library Services provision for Engage, Design, and Fit out Contract - Ely Library`; page 1 uses `23105 ...`. All line items descend from the 25112 tabs. The discrepancy is surfaced via warning `PROJECT_CODE_MISMATCH` so that tender-metadata reconciliation can happen in the daemon.
- **Currency set to `GBP` with explicit warning `CURRENCY_ASSUMED`**: no currency symbol appears inside any pricing cell in the deduped markdown. The only currency anchor is narrative text "(aprox £95k)" on page 1. Given the UK public-sector context, GBP is the correct default, but the skill is honest about this being an inference rather than an in-cell observation.
- **Unit `each` for unlabelled fit-out rows**: the fit-out column header is `Quantity (if applicable)` with no unit column. Inferring `each` from item semantics is less ambiguous than emitting empty string. `UNIT_INFERRED` warning documents the inference.
- **`repeated_header_normalized: true` on every line item**: because the Docling markdown exhibited extreme cell-wise duplication (intro cell replicated 65+ times), normalization was applied universally rather than per-row, and every emitted row transits through the dedupe.
- **`source_table_key` values point to Docling's structural table indices** (`#/tables/3`, `#/tables/4`, `#/tables/7`) rather than invented keys, preserving resume-strategy stability if this overlay is re-run on the same structural payload.
- **`extraction_confidence_bps = 7800`**: description and quantity confidence are high on the specified fit-out rows; unit/currency/price confidences are bounded by the blank-template and inferred-unit conditions; overall aggregate is mid-high.

## Warnings

The emitted `result.json` carries 7 explicit warnings. Each corresponds to a concrete observation in the source markdown (not a heuristic):

1. `PRICING_TEMPLATE_BLANK` — every Total cost cell is empty; "Total for Evaluated Items | 0" and "Grand Total for Evaluation | 0" appear on pages 2 and 3.
2. `PROJECT_CODE_MISMATCH` — page 1 says `23105`, pages 2-3 say `25112`.
3. `UNIT_INFERRED` — 11 of 12 specified fit-out rows (all except the carpet) plus all 7 example rows have no explicit unit in the source; `each` was inferred.
4. `REPEATED_HEADER_NORMALIZED` — Docling markdown is 121377 bytes but collapses to ~25026 bytes after consecutive-duplicate removal.
5. `BLANK_QUANTITY_CELL` — "Developing design draft", "Adjustments & final design", and all 7 example rows have no quantity.
6. `CURRENCY_ASSUMED` — currency inferred from narrative mention `£95k`, not from any pricing cell.
7. `SOURCE_IS_XLSX_NOT_PDF` — source file is `.xlsx` processed by Docling 2.88.0, not a native PDF.

No agent-runtime warnings (retries, partial output, schema violations) were raised — the single contract pass produced valid output on the first attempt.

## Evidence

All major claims in this manifest are anchored in the two durable artifacts the daemon can inspect:

- **`result.json`** (this session's workdir) — contains 24 line items each with an `evidence_anchor` naming `page_ref`, `anchor_text`, `section_ref`, and `table_ref` tying the row back to a specific Docling table index. Also contains 13 section-level `evidence_anchors` for document-level structure.
- **Docling structural payload** — rows in `state.db/document_extractions` keyed by `id = 74981553-c1e7-4c19-abbb-877535fd7e31`:
  - `raw_document_id = raw-pc-19574d11-0929-f111-813c-005056b64545-6d38bccae69cb64e8b124180b8b1be81a8b4cf24f765e8fa0e96b98a3a3a041f`
  - `source_content_hash = 35768d1e3ca8e2757089b14879768cf6d8735494a6d15761c40c8c9e8cc9078f`
  - `source_markdown_hash (this overlay) = 0e4943f6f37ded757bd12d088dcf558f8e9341378bda73c6d00185682d900afb`
  - `extraction_engine = docling`, `extraction_engine_version = 2.88.0`, `extraction_policy_version = 2026.04.12`
  - `page_count = 3`, `table_count = 14`, `section_count = 3`, `document_title = Appendix-3a-Pricing-Schedule`, `document_role = pricing_schedule`
  - `extracted_markdown` (121377 bytes) contains the exact header strings quoted in every line-item `evidence_anchor.anchor_text`.

Specific evidence anchors for the load-bearing claims in this manifest:

- "Engage, Design services pricing | Quantity day | Total cost" appears on page 2, table `#/tables/3` — this is the anchor for unit=`day` on ED-01..ED-05.
- "Library Fully replaced carpet | 500 sqm" appears on page 2, table `#/tables/4` — this is the in-cell anchor for FO-01's unit=`sqm` and quantity=500.
- "Total for Evaluated Items | 0" appears on page 2, table `#/tables/5`, and "Grand Total for Evaluation | 0" appears on page 3, table `#/tables/13` — these are the primary evidence for `PRICING_TEMPLATE_BLANK`.
- "limited by the funds available (aprox £95k)" appears on page 1, table `#/tables/1` — this is the only currency-bearing token in the document and the sole basis for `currency = GBP`.
- The project-code discrepancy is directly observable: `#/tables/0` (page 1) contains "23105 Library Services..." while `#/tables/2`, `#/tables/9` (pages 2-3) contain "25112 Library Services... - Ely Library".

Input parameters received in the job envelope (for traceability):
`document_extraction_id = 74981553-c1e7-4c19-abbb-877535fd7e31`, `document_set_id = tds-pc-19574d11-0929-f111-813c-005056b64545`, `extraction_scope = full_document`, `notice_id = pc-19574d11-0929-f111-813c-005056b64545`, `process_id = proc-pc-19574d11-0929-f111-813c-005056b64545`, `stage_run_id = stage-run-57a5f226-b1b8-4236-9510-0821c0ebd20d-proc-pc-19574d11-...`, `target_domain = pricing`.
