---
manifest_version: 1.0.0
skill_id: pdf-extraction/pricing-schedule
skill_version: 1.0.0
job_id: job-3d1f84fd-f5ba-478d-93ce-3fcfca4ab6db
session_id: session-semantic-f76f260d7ca3bf56d1775d31f72c37983946ea9c450e97bbaabb4d78b3650a09
execution_mode: contract
started_at: 2026-04-17T19:17:00Z
completed_at: 2026-04-17T19:21:00Z
outcome_class: partial
items_processed: 17
items_skipped: 7
items_failed: 0
warnings_count: 7
agent_claimed_pages_visited: 0
agent_claimed_source_url: "https://procontract.due-north.com/RfxResponse/Index?rfxId=28b81c96-0929-f111-813c-005056b64545"
agent_claimed_fingerprint_state: not_applicable
agent_claimed_locale_state: en-GB
agent_claimed_currency_state: inferred_GBP
---

# Summary

Extracted pricing-schedule semantic fields from the Docling markdown supplied for `document_extraction_id=a05c321a-e99b-4bdb-8eb9-07877c963aec` (raw document `Appendix-3a-Pricing-Schedule.xlsx`, 3 pages / 3 sheets per upstream Docling structural extraction). Seventeen authoritative line items were emitted: five from the 'Engage, Design services pricing' table on page 2 and twelve from the 'Fit out and furniture supply pricing' table on page 2. Seven optional 'for example' rows on page 2 (under the 'bidders to include any and all other items that are required to ensure successful delivery' table) were intentionally skipped.

The document is a bidder-facing pricing template — every unit-rate cell and every total-cost cell in both pricing tables is blank, and the document's own 'Grand Total for Evaluation' on the page 3 'Evaluation Only' sheet is `0`. SKILL.md non-goals forbid guessing rates, so `unit_rate_pence` and `total_value_pence` were emitted as `0` with very low field-level confidence (200/10000) and a top-level `EMPTY_PRICING_TEMPLATE` warning. `extraction_confidence_bps` set to 2500 to reflect that quantities and descriptions are recoverable but pricing is not. Currency inferred as GBP from the 'aprox £95k' wording on the Summary sheet (Statement of intention to tender) and from the UK buyer (Cambridgeshire County Council) — flagged via `CURRENCY_INFERRED` warning. A single synthetic `lot_number='1'` is used for all line items because the document does not enumerate lots — flagged via `NO_EXPLICIT_LOT_STRUCTURE` warning.

Result contract written to `result.json` and validated structurally against `result.schema.json`.

# Steps Taken

1. Resolved the input extraction packet: queried `state.db:document_extractions` for `id=a05c321a-e99b-4bdb-8eb9-07877c963aec` and confirmed the bound artefact is `Appendix-3a-Pricing-Schedule.xlsx`, classified upstream as `document_role=pricing_schedule`, `expected_overlays_json=["pdf-extraction/pricing-schedule"]`, `extraction_policy_version=2026.04.12`, `semantic_extraction_state=pending`, `page_count=3`, `table_count=14`, `section_count=3`. The upstream `source_content_hash` is `35768d1e3ca8e2757089b14879768cf6d8735494a6d15761c40c8c9e8cc9078f` and was reused verbatim for `extraction_metadata.source_markdown_hash`.
2. Read the full Docling-produced markdown (`length(extracted_markdown)=121377` bytes, 97 lines). Identified that the markdown has 14 table fragments and several heavily duplicated narrative cells on the Summary sheet (page 1: the 'Statement of intention to tender' block is repeated across columns and rows by Docling's Excel-to-markdown rendering).
3. Read `extracted_tables_json` (14 entries) and observed that every table object had `headers=[]` and `rows=[]` despite declaring page numbers — i.e. the structural payload was empty for this XLSX. Reconstructed line items from the markdown table fragments instead. Emitted `STRUCTURAL_TABLES_PAYLOAD_EMPTY` warning so downstream stages know the structural payload is not authoritative for this extraction.
4. Read `extracted_metadata_json` and used the docling_document body order to assign stable `source_table_key` values: `#/tables/2` for the 'Engage, Design services pricing' table on the 'Comprehensive price list' sheet, `#/tables/3` for the 'Fit out and furniture supply pricing' table on the same sheet.
5. Identified document shape: three sheets (Summary on page 1, Comprehensive price list on page 2, Evaluation Only on page 3). The pricing-bearing tables are all on page 2; page 1 carries the contract intention narrative and the page 3 sheet is a roll-up evaluation grid showing `Grand Total for Evaluation = 0` (because the template is unfilled).
6. Extracted 17 line items: 5 from the Engage/Design table (Onsite Visit, Staff and stakeholder engagement, Developing design draft, Engagement on design, Adjustments & final design) and 12 from the Fit-out table (Library Fully replaced carpet, Kinder Boxes, Freestanding book display, Study tables, Study Chairs, Lobby study bar, Bar stools, Coffee Table, Armchair, Meeting table, Meeting area chairs, Youth Sofa). Quantities preserved verbatim from the source cells; unit normalised to 'day' for the Engage/Design table (column header is 'Quantity day') and to 'sqm' for the carpet row (cell value is '500 sqm') and to 'each' for bare-integer fit-out rows.
7. Skipped seven 'for example' rows under the 'bidders to include any and all other items that are required to ensure successful delivery' table on page 2 (Poster Display Unit, Leaflet Display Unit, PC Desk - Public Use, PC Chairs - Public Use, Benching Managers office, Graphics, Signage). These are open-ended placeholders for bidder-added items and have no quantities; emitting them as line items would mis-represent the buyer's authoritative pricing requirements. Recorded as `OPTIONAL_BIDDER_ITEMS_SKIPPED` warning.
8. Emitted `pricing_summary.total_value_pence=0` and `pricing_summary.lot_count=1`, anchored to the 'Grand Total for Evaluation = 0' cell on page 3.
9. Scored field confidences: `description` 9500 (verbatim from source rows); `quantity` 7000 (numerically present for 15 of 17 items); `unit` 5500 (mixture of explicit 'day'/'sqm' and inferred 'each'); `lot_number` 3000 (synthetic); `unit_rate_pence` and `total_value_pence` 200 (template empty, value 0 is a defensive default not a measurement); `currency` 6500 (inferred from contextual GBP cues, not stated in the pricing tables); `pricing_summary.total_value_pence` 4500 (the document explicitly shows 0 for an unfilled template, so the literal value is correct but its meaning is 'no bid yet').
10. Set `extraction_confidence_bps=2500` to reflect that the document is structurally recoverable but the pricing payload is empty.
11. Emitted seven warnings with in-document evidence refs: `EMPTY_PRICING_TEMPLATE`, `MISSING_QUANTITY`, `CURRENCY_INFERRED`, `NO_EXPLICIT_LOT_STRUCTURE`, `REPEATED_HEADER_FRAGMENTS`, `OPTIONAL_BIDDER_ITEMS_SKIPPED`, `STRUCTURAL_TABLES_PAYLOAD_EMPTY`.
12. Emitted twelve top-level `evidence_anchors` covering the document title block, the Statement of intention to tender (currency inference anchor), both pricing-table headers on page 2, the Engage/Design 'Total for Evaluated Items = 0' row, the page 3 sheet header, the page 3 'Required Elements / Total cost' header, and the three roll-up rows on page 3 culminating in 'Grand Total for Evaluation = 0'.
13. Wrote `result.json` to the session working directory. Validated structurally with the JSON Schema (`jsonschema.validate(result, result_schema)` passes): all seven required top-level keys present, `extraction_metadata.target_domain="pricing"` (matches schema const), `extraction_metadata.extraction_scope="full_document"` (matches the job parameter `request.extraction_scope`), `policy_version="2026.04.12"` (matches the upstream `extraction_policy_version`).

# Decisions Made

1. Emit `unit_rate_pence=0` and `total_value_pence=0` for every line item rather than omit the line items. The schema requires both fields as non-negative integers, and the document's own page 3 roll-up explicitly states `Grand Total for Evaluation = 0`. Setting these to 0 with a field-level confidence of 200/10000 and a top-level `EMPTY_PRICING_TEMPLATE` warning communicates "no price was supplied" without forcing the daemon to invent values, and preserves the line items so downstream stages can persist the buyer's pricing requirements (descriptions, quantities, units, minimum standards) for matching and evaluation purposes.
2. Skip the seven 'for example' rows on page 2 under the 'bidders to include any and all other items' table. They are open-ended placeholders for bidder-added items, have no quantities and no minimum standards, and would mis-represent the buyer's authoritative pricing requirements. Recorded as `OPTIONAL_BIDDER_ITEMS_SKIPPED` warning with full enumeration of skipped descriptions.
3. Use a single synthetic `lot_number='1'` for all 17 line items. The document does not enumerate lots; the contract reference '25112' appears as a scheme/notice number on page 2 and should not be treated as a lot identifier. Surfaced via `NO_EXPLICIT_LOT_STRUCTURE` warning. This avoids burning the lot dimension of the schema for an artificial split while keeping `pricing_summary.lot_count=1` honest.
4. Infer currency as GBP. The pricing tables themselves contain no currency symbol or ISO code, but the page 1 Statement of intention to tender refers to 'aprox £95k', and the buyer (Cambridgeshire County Council) is a UK contracting authority. Surfaced via `CURRENCY_INFERRED` warning so the daemon can downgrade reliance on currency if needed.
5. Reconstruct line items from the markdown table fragments rather than from `extracted_tables_json`. The structural payload declared 14 table objects but every one had `headers=[]` and `rows=[]`. Recorded via `STRUCTURAL_TABLES_PAYLOAD_EMPTY` warning so an upstream Docling fix or a re-extraction can be scheduled. Stable `source_table_key` values reused from the docling_document body order in `extracted_metadata_json`.
6. Use `unit='day'` for all five Engage/Design rows because the column header is 'Quantity day'. Use `unit='sqm'` for the carpet row because the cell value is literally '500 sqm'. Use `unit='each'` for the eleven other fit-out rows whose quantity cells are bare integers; flagged via lower `line_items.unit` field confidence (5500/10000).
7. Quantity for two Engage/Design rows ('Developing design draft' and 'Adjustments & final design') is blank in the source and the schema requires a `number`; emitted as `0` with `MISSING_QUANTITY` warning rather than guessing, and `line_items.quantity` field confidence reflects the mixed evidence (7000/10000 aggregate).
8. Classify `outcome_class=partial` in the run manifest. The skill completed the contract cleanly and produced a schema-valid result with 17 line items, but the absence of any priced cells is a recoverable partial outcome rather than full success. The daemon is expected to mark the document_semantic_extraction as partial and prefer a downstream bid-response extraction (filled-in template returned by a bidder) when one becomes available under the same document_set.

# Warnings

1. `EMPTY_PRICING_TEMPLATE` — All unit-rate and total-cost cells in both pricing tables are blank. The document's own 'Grand Total for Evaluation' on the page 3 sheet shows 0. This is a bidder-facing pricing template, not a completed pricing schedule. Evidence: page 2 'Engage, Design services pricing' Total cost column, page 2 'Fit out and furniture supply pricing' Total cost column, page 3 'Grand Total for Evaluation = 0'.
2. `MISSING_QUANTITY` — Two Engage/Design rows ('Developing design draft' and 'Adjustments & final design') have no quantity cell value; quantity emitted as 0 with low confidence.
3. `CURRENCY_INFERRED` — No currency symbol or ISO code is present in either pricing table. Currency inferred as GBP from the 'aprox £95k' wording on the Summary sheet and from the UK buyer (Cambridgeshire County Council).
4. `NO_EXPLICIT_LOT_STRUCTURE` — The document does not enumerate lots. A single synthetic `lot_number='1'` is used for all line items.
5. `REPEATED_HEADER_FRAGMENTS` — The Docling-produced markdown contains heavily duplicated rows for narrative cells on the Summary sheet (page 1 'Statement of intention to tender' block is repeated 13 times across columns and rows). Pricing-table rows on page 2 also repeat the header in the first row before the data rows; repeated headers were normalised and only the first instance was treated as the header. `repeated_header_normalized=true` on every emitted line item.
6. `OPTIONAL_BIDDER_ITEMS_SKIPPED` — Seven 'for example' rows under the 'bidders to include any and all other items that are required to ensure successful delivery' table on page 2 were intentionally not emitted. Skipped descriptions: Poster Display Unit, Leaflet Display Unit, PC Desk - Public Use, PC Chairs - Public Use, Benching Managers office, Graphics, Signage.
7. `STRUCTURAL_TABLES_PAYLOAD_EMPTY` — The upstream Docling structural payload (`extracted_tables_json`) declared 14 tables but every table object had empty headers and zero rows. Line items were therefore reconstructed from the markdown table fragments rather than from the structural payload.

# Evidence

Major claims in the result are each anchored to a specific table fragment in the supplied Docling markdown. Each evidence anchor below carries `page_ref`, `section_ref`, optional `table_ref` (the stable `#/tables/N` key derived from the docling_document body order), and `anchor_text` (a literal excerpt):

- Document identity and contract reference — page 2: `25112 Library Services provision for Engage, Design, and Fit out Contract - Ely Library` (top-level anchor #3, used to anchor `NO_EXPLICIT_LOT_STRUCTURE` warning).
- Currency inference — page 1, Statement of intention to tender: `Being mindful that the final agreed contract is limited by the funds available (aprox £95k)` (top-level anchor #2, used to anchor `CURRENCY_INFERRED` warning).
- Engage/Design table header — page 2, table `#/tables/2`: `| Engage, Design services pricing | Quantity day | Total cost |` (top-level anchor #4, used to anchor unit normalisation to 'day').
- Engage/Design empty roll-up — page 2, table `#/tables/2`: `| Total for Evaluated Items | Total for Evaluated Items | 0 |` (top-level anchor #5, used to anchor `EMPTY_PRICING_TEMPLATE` warning at the section level).
- Fit-out table header — page 2, table `#/tables/3`: `| Fit out and furniture supply pricing | Quantity (if applicable) | Total cost | Minimum standard |` (top-level anchor #6, used to anchor unit defaults).
- Optional bidder-added items header — page 2: `bidders to include any and all other items that are required to ensure successful delivery` (top-level anchor #7, used to anchor `OPTIONAL_BIDDER_ITEMS_SKIPPED` warning).
- Page 3 sheet identity — page 3: `25112 Library Services provision for Engage, Design, and Fit out Contract - Ely Library` (top-level anchor #8).
- Page 3 roll-up header — page 3: `| Required Elements | Total cost |` (top-level anchor #9).
- Page 3 Engage/Design roll-up — page 3: `| Engage, Design services pricing | 0 |` (top-level anchor #10).
- Page 3 Fit-out roll-up — page 3: `| Fit out and furniture supply pricing | 0 |` (top-level anchor #11).
- Page 3 Grand Total — page 3: `| Grand Total for Evaluation |   0 |` (top-level anchor #12, used to anchor `pricing_summary.total_value_pence=0` and the central `EMPTY_PRICING_TEMPLATE` warning).
- Each of the 17 line items carries its own per-row `evidence_anchor` with the exact source row text from the markdown table fragment and the stable `source_table_key` (`#/tables/2` or `#/tables/3`).

Daemon-observable facts supporting this manifest (available for the daemon evidence appendix):
- `document_extraction.id = a05c321a-e99b-4bdb-8eb9-07877c963aec` (from `state.db:document_extractions`).
- `document_extraction.source_content_hash = 35768d1e3ca8e2757089b14879768cf6d8735494a6d15761c40c8c9e8cc9078f` (reused as `extraction_metadata.source_markdown_hash`).
- `document_extraction.page_count = 3`, `section_count = 3`, `table_count = 14`, `image_count = 0` (per upstream Docling row).
- `document_extraction.document_role = "pricing_schedule"` (upstream classification, consistent with this overlay's expected role).
- `document_extraction.extraction_policy_version = "2026.04.12"` (reused as `extraction_metadata.policy_version`).
- `document_extraction.expected_overlays_json = ["pdf-extraction/pricing-schedule"]` (this skill is the bound overlay).
- `raw_document.storage_path = /Users/alinaaitova/Desktop/Testproject1/artifacts/downloads/procontract/782746/Appendix-3a-Pricing-Schedule.xlsx`.
- `raw_document.source_url = https://procontract.due-north.com/RfxResponse/Index?rfxId=28b81c96-0929-f111-813c-005056b64545`.
- `raw_document.content_type = application/vnd.openxmlformats-officedocument.spreadsheetml.sheet` (XLSX, not PDF — the skill is named pdf-extraction/pricing-schedule but operates on the Docling markdown produced by the upstream structural extraction stage regardless of source MIME).
- `contract_notice.id = pc-19574d11-0929-f111-813c-005056b64545`, `title = "25112 Library Services Engage, Design and Fit out Contract - Ely Library"`, `buyer_name = "Cambridgeshire County Council"`, `main_category = "39155000-3 - Library furniture | 79932000-6 - Interior design services"` (used as input to the GBP currency inference; CPV codes deliberately not propagated into line items because they are notice-level, not in-document evidence).
- `tender_document_set.id = tds-pc-19574d11-0929-f111-813c-005056b64545`, `process_id = proc-pc-19574d11-0929-f111-813c-005056b64545`, `stage_run_id = stage-run-57a5f226-b1b8-4236-9510-0821c0ebd20d-proc-pc-19574d11-0929-f111-813c-005056b64545` (per job parameters).
- `executor_id = local-daemon`, `idempotency_key = extract_document_structure::job-extract-e4cec2d983fe8d159aa85c816057b5c364c80220f5558fce21014b715d56eb21::pdf-extraction/pricing-schedule` (per `runner_meta`).
