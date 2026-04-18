---
manifest_version: 1
skill_id: pdf-extraction/tender-specification
skill_version: 1.0.0
job_id: job-323a6680-65f8-4a0e-b081-3a93dfc4344d
session_id: session-semantic-dd4c559530f67041457ec3f22271af8ff33a80c7c7ee58c21cecc64d5b5b58a7
execution_mode: contract
started_at: 2026-04-18T10:36:00Z
completed_at: 2026-04-18T10:40:00Z
outcome_class: completed_with_warnings
items_processed: 4
items_skipped: 0
items_failed: 0
warnings_count: 6
agent_claimed_pages_visited: ["sheet: 2FE Primary", "sheet: 2 class primary resource base", "sheet: 5FE Secondary", "sheet: 4 class secondary resource base", "sheet: Calcs", "sheet: Assumptions"]
agent_claimed_source_url: null
agent_claimed_fingerprint_state: not_applicable
agent_claimed_locale_state: en-GB
agent_claimed_currency_state: GBP
---

# Run Manifest — pdf-extraction/tender-specification

## Summary

Extracted four accommodation-programme lots from the document "Schedule of Accommodation & Calculations DRAFT (To be read)" (document_extraction_id `a665ce62-597c-4aec-b858-948a0bc363e3`, document_role `technical_specification`, raw_document_id `raw-pc-f0f30f94-e22c-f111-813c-005056b64545-cb70b8003bc3f8ad58d489845d04005486cfc31590f336177927c203871acc6c`) associated with the Stockport Town Centre School multi-disciplinary design team tender (process `proc-pc-f0f30f94-e22c-f111-813c-005056b64545`, notice `pc-f0f30f94-e22c-f111-813c-005056b64545`, buyer "Stockport Metropolitan Borough Council"). The four synthetic lots correspond to the four space-programme sheets the document presents: `SoA-1` 2FE Mainstream Primary (GIA 2,081.9 m² per BB103, uplifted by 15% to ≈2,400 m² on SMBC instruction), `SoA-2` 2-class Primary Resource Base (193 m² GIA, 16 SEND pupils), `SoA-3` 5FE Mainstream Secondary (5,889.7 m² per BB103, uplifted to ≈6,800 m², 750 pupils, Typical curriculum, 5 storey in 2 blocks as fixed in the DfE SoA tool), and `SoA-4` 4-class Secondary Resource Base (427 m² GIA, 32 SEND pupils). Fifteen `product_requirement_seeds` were emitted covering major specialty-room and building-fit-out deliverables (kitchen suite, hygiene room hoist, primary assembly hall, primary practical room, Resource Base classroom fit-outs, secondary science laboratories, 3-court sports hall, main hall with bleachers, resistant-materials DT workshop, food-tech room, drama studio, music suite, dining area). Two technical standards were cited consistently across the lots: Building Bulletin 103 (BB103) — Area guidelines for mainstream schools, and the Department for Education (DfE) Schedule of Accommodation tool / guidance. No `cpv_codes` and no `evaluation_criteria` were available in the source (flagged via warnings `NO_CPV_CODES_IN_SOURCE` and `NO_EVALUATION_CRITERIA_IN_SOURCE`); both are carried at the notice level or in sibling-annex documents rather than in this schedule. `extraction_confidence_bps` = 7600. Outcome class is `completed_with_warnings`.

## Steps Taken

1. Read the result-contract schema at `skills/pdf-extraction/tender-specification/result.schema.json` to confirm required fields, evidence-anchor shape, and the `target_domain = "lots"` constant.
2. Loaded the `document_extractions` row for `a665ce62-597c-4aec-b858-948a0bc363e3` from `state/state.db`, confirming document_title, document_role = `technical_specification`, page_count = 6, section_count = 6, table_count = 64, extraction_policy_version = `2026.04.12`, and expected_overlays = `["pdf-extraction/tender-specification"]`.
3. Computed the SHA-256 hash of `extracted_markdown` (`9ade44c14bcd59d844e69aba4acd7e22399f3a30c24126ae1c7ebe8f5602ca9e`) for inclusion in `extraction_metadata.source_markdown_hash`. Verified the source_content_hash (`3fbeb3c0...`) is distinct — it is the raw PDF hash, not the markdown hash.
4. Inspected `extracted_sections_json` — 6 Docling groups: `#/groups/0` "sheet: 2FE Primary", `#/groups/1` "sheet: 2 class primary resource base", `#/groups/2` "sheet: 5FE Secondary", `#/groups/3` "sheet: 4 class secondary resource base", `#/groups/4` "sheet: Calcs", `#/groups/5` "sheet: Assumptions". All sheets report `page_start = page_end = 1`, i.e. Docling flattened the multi-sheet xlsx origin to a single PDF page surface; evidence anchors therefore use `page_ref = sheet: <name>` plus the section_ref.
5. Cross-referenced the notice at `contract_notices` / `procurement_processes` levels — buyer is Stockport Metropolitan Borough Council, notice category is "150000 - Consultancy", and five `notice_lots` already exist (Annex A.1 Architect, Annex A.4 CDM Principal Designer, Annex A.5 Structural/Civils, etc.). These are per-discipline scope lots orthogonal to the four accommodation programmes in this document. Per skill guidance, pre-existing lots are cross-reference only; they are neither extended nor overwritten by this extraction.
6. Skimmed the markdown (`/tmp/doc_markdown.md`, 663 lines, 187,845 bytes) for GIA totals, section structure and landmark rows: primary GIA 2,081.9 m² @ row 65, primary RB GIA 193 m² @ row 104, secondary header "750 Places, Typical Curriculum Type" @ row 106, secondary GIA 5,889.7 m² @ row 240, secondary RB GIA 427 m² @ row 318, SMBC 15% uplift statements @ rows 67 (primary) and 266 (secondary), BB103 and DfE SoA references at rows 67, 135, 266, 454, 558, 571, 585, 627, 656.
7. Built four lots with stable synthetic identifiers `SoA-1` … `SoA-4` (one per mainstream/Resource Base programme). Each lot's `evidence_anchor` points at the sheet header row; each lot lists between 3 and 8 `requirements` items, 0 `evaluation_criteria` (empty — not present in source), 0 `cpv_codes` (empty — not present in source), 2 `technical_standards` (BB103 and DfE SoA) where relevant, and 1–8 `product_requirements`.
8. Emitted 15 `product_requirement_seeds` with stable seed_keys (`primary-kitchen-full-service-suite`, `primary-hygiene-room-hoist`, `primary-practical-room-food-science-dt`, `primary-assembly-hall-dining`, `primary-rb-classroom-fit-out`, `primary-rb-hygiene-room`, `secondary-science-laboratory-standard`, `secondary-sports-hall-3court`, `secondary-main-hall-bleachers`, `secondary-resistant-materials-workshop`, `secondary-food-tech-room`, `secondary-drama-studio`, `secondary-music-suite`, `secondary-dining-area`, `secondary-rb-classroom-fit-out`) so that daemon-side `product_candidates` writes can dedupe on re-runs.
9. Populated `field_confidences` with per-field bps scores reflecting high confidence for titles/descriptions/standards (8,200–8,500), medium for requirements (8,000) and seeds (6,500), very low for evaluation_criteria (2,000) and cpv_codes (1,000) because those fields are not present in source. Set overall `extraction_confidence_bps` = 7,600.
10. Added six warnings flagging: (a) document is a technical schedule not a lot-defining tender pack; (b) no evaluation criteria in source; (c) no CPV codes in source; (d) no lot numbering in source (lot IDs are synthetic); (e) product seeds are building-fit-out not goods (tender is a design consultancy); (f) Docling page collapse for xlsx input (all sheets report page 1, anchors use sheet-name + section_ref).
11. Validated `result.json` against `result.schema.json` using `jsonschema`; validation passed. Wrote `result.json` to the workdir.

## Decisions Made

- **Lot granularity = four accommodation programmes, not one flat lot.** The document presents four clearly-separated schedules (Docling groups 0..3) each with its own header, its own GIA total, and its own design assumptions (storey count, block count, curriculum type, pupil count). Emitting them as four lots preserves the structure a downstream consumer needs to reason about primary vs secondary vs SEND Resource Base requirements; collapsing them into one "overall SoA" lot would have lost that structure.
- **Synthetic lot identifiers `SoA-1`..`SoA-4`.** The document does not assign procurement-lot numbering to its schedules. Per schema, `lot_number` is a string, not a positional integer — `SoA-<n>` communicates the section origin while signalling (together with warning `NO_LOT_NUMBERING_IN_SOURCE`) that these are not numbered procurement lots. This also prevents collision with the pre-existing notice_lots rows for the per-discipline scopes (lot-1 Architect, lot-5 Structural, lot-a-3 CDM).
- **No mutation of existing notice_lots rows.** The skill instructs us not to treat pre-existing lot rows as authoritative truth if the tender document contradicts them — but the Schedule of Accommodation does not contradict the five per-discipline scope lots, it is orthogonal to them. The extraction emits a target_domain = "lots" payload; persistence logic is owned by the daemon (per the "Do not use when" skill section).
- **`evaluation_criteria: []` and `cpv_codes: []` for every lot.** These fields are required by the schema as arrays; when the source document contains none, empty arrays are the honest answer. Inferring notice-level CPV into lot-level cpv_codes would fabricate evidence not present in this document.
- **Evidence anchor strategy: sheet name + Docling section_ref.** Because Docling collapsed every xlsx sheet to page 1, page numbers are uninformative. `page_ref = "sheet: 2FE Primary"` (etc.) tracks the Docling section heading verbatim, and `section_ref = "#/groups/0"` (etc.) points at the exact Docling structural node. This is also what `extracted_sections_json` uses.
- **Product-requirement seeds modelled as building fit-out elements, not goods.** Because the procurement is a Consultancy / design-team appointment, seeds represent the spaces and installations the design team will specify (sports hall, science lab, hygiene room hoist, drama studio, kitchen, DT workshop). They are not goods the buyer will purchase under THIS tender; warning `PRODUCT_SEEDS_ARE_BUILDING_FIT_OUT_NOT_GOODS` signals this to the daemon so downstream matching logic can route accordingly.
- **`extraction_confidence_bps = 7600`.** The document content is highly structured and directly readable; lot structure is self-evident. Deductions: absent evaluation_criteria and cpv_codes (−400 bps), seed-to-candidate mapping uncertainty for a consultancy tender (−800 bps), Docling page-collapse ambiguity (−200 bps).
- **`outcome_class = completed_with_warnings`.** All four lots were successfully populated with evidence anchors and field confidences; no items were skipped or failed. The six warnings document source limitations, not extraction failures.

## Warnings

1. `DOCUMENT_ROLE_TECHNICAL_SCHEDULE_NOT_PROCUREMENT_LOT_DEFINITION` — the document is a technical-specification annex / design brief, not a lot-defining tender pack. The four emitted lots are space-programme deliverables, not independently biddable procurement lots.
2. `NO_EVALUATION_CRITERIA_IN_SOURCE` — evaluation criteria are carried in sibling-annex documents (Invitation to Participate / Instructions, Response Document), not in this schedule.
3. `NO_CPV_CODES_IN_SOURCE` — CPV assignment exists at the notice level (category "150000 - Consultancy"); no CPV codes appear in this schedule and no CPV inference was performed.
4. `NO_LOT_NUMBERING_IN_SOURCE` — the source document does not number its schedules as procurement lots; `SoA-1` … `SoA-4` are synthetic labels derived from Docling groups 0..3. The five pre-existing per-discipline `notice_lots` rows are orthogonal and should not be merged with these.
5. `PRODUCT_SEEDS_ARE_BUILDING_FIT_OUT_NOT_GOODS` — seeds are upstream design requirements (building fit-out / specialist rooms), not goods purchased under this Consultancy appointment. They inform later construction-stage product discovery, not current matching.
6. `DOCLING_PAGE_COLLAPSE_FOR_XLSX` — Docling converted a multi-sheet .xlsx workbook and collapsed all six sheets to `page_start = page_end = 1`; evidence anchors rely on sheet heading + Docling `section_ref` instead of PDF page numbers.

## Evidence

Each evidence anchor below cites the Docling `section_ref` it came from and the exact markdown header/row substring used as the anchor text. These anchors are duplicated in `result.json#/evidence_anchors` and are also referenced per-lot in `result.json#/lots[n].evidence_anchor` and per-seed in `result.json#/product_requirement_seeds[n].evidence_anchor`.

### Lot SoA-1 (2FE Primary Mainstream) — section_ref #/groups/0, page_ref "sheet: 2FE Primary"
- Lot header anchor: `2FE Mainstream Primary Element | No. Spaces | Av. Area (m2) | Total Area (m2) | Notes and assumptions` (source row 1 of the Docling markdown).
- GIA anchor: `Total Gross Internal Area (GIA)* | | | 2081.9 | Excludes 15% uplift* | 2394.18 | Including 15% uplift` (row 65).
- SMBC uplift standard anchor: `* SMBC have instructed that the GIA should be uplifted to 15% above BB103 recommended GIA i.e. a total of approximately 2,400m2.` (row 67) — cites BB103 as the baseline standard.
- Kitchen seed anchor: `Food prep areas, kitchen (prep/ servery suite) | 1 | 58 | 58 | Includes capacity for primary Resource Base pupils. Area stated is recommended min. for full service kitchen` (row 51).
- Hygiene room seed anchor: `Hygiene rooms (small - 1 door) | 1 | 8 | 8 | With hoist, changing bed and accessible WC. Should have either a sluice or some other facility for dealing with waste` (rows 56–57 area).
- Primary practical seed anchor: `Primary practical (food/ science/ DT) | 1 | 69 | 69 | 62m2 - 69m2 for full class teaching ... Suitable bench, sink and cooker arrangements e.g.one work station per 3-5 pupils.` (row 9).
- Assembly hall seed anchor: `Assembly halls, primary | 1 | 150 | 150 | Suitable for 40 or 60 minute dining in 2 or 3 sittings.` (row 12).

### Lot SoA-2 (2-class Primary Resource Base) — section_ref #/groups/1, page_ref "sheet: 2 class primary resource base"
- Lot header anchor: `2 Class Primary Resource Base Element | No. Spaces | Av. Area (m2) | Total Area (m2) | Notes and assumptions` (row 75).
- GIA anchor: `Total Gross Internal Area (GIA) | | | 193` (row 104).
- Classroom seed anchor: `Resource base classrooms. | 2 | 52 | 104 | With sink and drainer. For 8 pupils per class, zone used is sufficient for any key stage, including EYFS.` (row 77).
- Conditional hygiene-room seed anchor: `Hygiene rooms (small - 1 door) | 1 | 8 | 8 | Resource Base should have easy access to main school hygiene room. If not, re-provide in Resource Base.` (rows 100–102 area).
- Primary SoA back-reference anchor: `See Primary SoA.` (row 80 area) — grounds the cross-reference to the primary schedule for dining/halls access.

### Lot SoA-3 (5FE Mainstream Secondary) — section_ref #/groups/2, page_ref "sheet: 5FE Secondary"
- Lot header anchor: `5FE Mainstream Secondary Element | 750 Places, "Typical" Curriculum Type` (row 106) — cites pupil count and curriculum type.
- GIA anchor: `Total Gross Internal Area (GIA)** | | | 5889.7 | Excludes 15% uplift* | 6773.15 | Including 15% uplift` (row 240).
- SMBC uplift standard anchor: `** SMBC have instructed that the GIA should be uplifted to 15% above BB103 recommended GIA i.e. a total of approximately 6,800m2.` (row 266) — cites BB103 as the baseline standard.
- Science laboratory seed anchor: `general science laboratory | 6 | 83 | 498` (row 119).
- Sports hall seed anchor: `school sports hall (3-court, 18m x 28m) | 1 | 504 | 504` (row 132).
- Main hall with bleachers seed anchor: `main hall (secondary, with bleachers) | 1 | 197 | 197` (row 131).
- Resistant materials workshop seed anchor: `resistant materials workshop (heavy practical) | 1 | 111 | 111` (row 126).
- Food room seed anchor: `food room (heavy practical) | 1 | 97 | 97` (row 127).
- Drama studio seed anchor: `drama studio | 1 | 97 | 97` (row 123).
- Music suite seed anchor: `music room (fitted extensive) | 1 | 69 | 69` (row 122).
- Dining standard anchor: `Dining and Social Areas ... Minimum total dining area for 60 minute continuous dining is 205m2 (equivalent to apx 3 sittings) per DfE SoA.` (row 135).

### Lot SoA-4 (4-class Secondary Resource Base) — section_ref #/groups/3, page_ref "sheet: 4 class secondary resource base"
- Lot header anchor: `4 Class Secondary Resource Base Element | No. Spaces | Av. Area (m2) | Total Area (m2) | Notes and assumptions` (row 284).
- GIA anchor: `Total Gross Internal Area (GIA) | | | 427` (row 318).
- Classroom seed anchor: `Resource base classrooms. | 4 | 49 | 196 | For 8 pupils per class. Assumption is that there will be sufficient specialist practical capacity in the main school.` (row 288).

### Sheet: Calcs — section_ref #/groups/4
- BB103 calc anchor: `Per BB103 calc` (row 558, 571, 585) — grounds the BB103 standard reference applied in the sizing calculations underpinning the primary and secondary sheets.

### Sheet: Assumptions — section_ref #/groups/5
- Primary assumptions anchor: `Primary | Restricted site (although this does not change the overall GIA) | 1 and 2 storey new construction | 1 block | All general classrooms standard size` (rows 639–651).
- Secondary assumptions anchor: `Secondary | Restricted site | 5 storey new construction | 2 blocks (fixed in DfE SoA tool) | Typical curriculum type | Group sizes of 30 pupils unless otherwise stated | Assembly hall with bleacher seating | Dining hall for 60 minute continous dining (considred optimal)` (rows 652–662) — grounds the DfE SoA tool reference and the secondary design-brief assumptions.

### Source identity (daemon-observable)
- `document_extraction_id = a665ce62-597c-4aec-b858-948a0bc363e3`
- `raw_document_id = raw-pc-f0f30f94-e22c-f111-813c-005056b64545-cb70b8003bc3f8ad58d489845d04005486cfc31590f336177927c203871acc6c`
- `source_content_hash (raw) = 3fbeb3c00101a5a4071295fe7bcbefc1acb9b45afd72e6adfea9885102cdab3b`
- `source_markdown_hash (agent-computed) = 9ade44c14bcd59d844e69aba4acd7e22399f3a30c24126ae1c7ebe8f5602ca9e`
- `document_title = "Schedule of Accommodation & Calculations DRAFT (To be read)"`
- `document_role = technical_specification`
- `extraction_policy_version = 2026.04.12`
- `expected_overlays = ["pdf-extraction/tender-specification"]`
