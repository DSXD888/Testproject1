---
manifest_version: 1.0.0
skill_id: pdf-extraction/tender-specification
skill_version: 1.0.0
job_id: job-eee5b15c-4c6b-4bbc-91d7-71d6ed78e6e7
session_id: session-semantic-49651affa56d28c49c0bb3f744d7c35ced0353008c1be946863ca8c9b94812c0
execution_mode: contract
started_at: 2026-04-18T00:00:00Z
completed_at: 2026-04-18T00:00:00Z
outcome_class: success
items_processed: 1
items_skipped: 0
items_failed: 0
warnings_count: 8
agent_claimed_pages_visited: 0
agent_claimed_source_url: "sqlite:document_extractions/9e8c24cc-519b-4e6c-802c-5fcf0b27ad9a"
agent_claimed_fingerprint_state: not_applicable_contract_mode
agent_claimed_locale_state: en-GB
agent_claimed_currency_state: GBP
---

# Run Manifest - pdf-extraction/tender-specification

## Summary

Extracted a single authoritative tender-specification lot from the Docling markdown associated with document_extraction `9e8c24cc-519b-4e6c-802c-5fcf0b27ad9a` (`raw-pc-dc1daee2-772e-f111-813c-005056b64545-...`), a 25,199-character markdown body titled "6278 - Draft specification - Lot 5 - Provision of Building Condition Surveys" for London Borough of Bexley. The document is self-evidently the specification for Lot 5 only of the parent Framework notice (Lots 1-11). Output payload: 1 lot (`lot_number = "5"`), 6 lot-level product_requirements, 5 top-level product_requirement_seeds, 12 technical_standards (RICS, CIPFA, MRICS, BCIS, SPONS, AutoCAD, RPI/CPI, A-D condition grading, P1-P4 priority grading, EPC/DEC/asbestos/electrical/gas/water/fire safety obligations, asbestos awareness training, TF Cloud/CIVICA compatibility), and 18 evidence anchors. `evaluation_criteria` and `cpv_codes` intentionally left empty with documented warnings because the specification body does not contain them; CPV and evaluation weightings live at notice / ITT level. `extraction_metadata.target_domain = "lots"`, `extraction_scope = "full_document"`, `source_markdown_hash` reuses the daemon-recorded `source_content_hash` for this extraction. Overall `extraction_confidence_bps = 8100`.

## Steps Taken

1. Confirmed working-directory state (`pwd`, `ls`) and read the contract output index `../contract-output.index.json` to verify session/job identity alignment (`session-semantic-4965...`, `job-eee5b15c-...`).
2. Queried `state/state.db` for the `document_extractions` row via the `document_extraction_id` carried in job params, retrieving `document_title`, `document_role = technical_specification`, `extraction_policy_version = 2026.04.12`, `expected_overlays_json = ["pdf-extraction/tender-specification"]`, `page_count = 0`, `section_count = 58`, `table_count = 2`, `source_content_hash = 2e1eed67144c8c783755b0cb50e09a1c1a07de2573f032eb8ebc805b21cc4e02`, and `semantic_extraction_state = pending`.
3. Dumped the full `extracted_markdown` (25,199 chars, 353 lines) to `/tmp/extracted_markdown.md` and read it in three passes covering the title block and summary list, sections 1-15, and sections 16-30 plus the life-expectancy / obligations block.
4. Cross-referenced against `contract_notices.pc-dc1daee2-...` and `notice_lots` to confirm the document maps to the single existing lot `pc-dc1daee2-...-lot-5` titled `LOT 5 - Condition Surveys` (`lot_number = 5` integer; emitted as string per schema). Other lots (1-4, 6-11) were confirmed to belong to other specifications and must not be fabricated here.
5. Read `skills/pdf-extraction/tender-specification/result.schema.json`, `golden.assertions.yaml` and `canary.inputs.jsonl` to align field shape, evidence-anchor schema, and the `target_domain = "lots"` global assertion.
6. Authored `result.json` conforming to the schema with section-scoped evidence anchors (`section:<n>_<slug>`) given `page_count = 0`, then validated against the schema with `jsonschema.Draft202012Validator` - 0 errors.
7. Authored this run manifest with the required YAML frontmatter and narrative sections.

## Decisions Made

- **Single-lot output.** The source document is self-scoped to Lot 5 (title block: "LOT 5 - Specification For - Condition Surveys"). Emitting additional lots would contradict the document and the pre-existing `notice_lots` row. Warning `SERVICES_ORIENTED_LOT` records that this is a professional-services engagement.
- **`lot_number = "5"` (string).** Schema requires string; existing `notice_lots.lot_number = 5` integer is the source of truth and rendered as its decimal string.
- **`evaluation_criteria = []` with warning.** The specification body contains condition-grading and priority-grading rubrics (scoring of building condition, not scoring of tender responses) but no tender evaluation criteria or weightings. Per SKILL.md ("Do not claim ... without citing evidence anchors"), I did not fabricate MEAT / price-quality splits. Warning `EVALUATION_CRITERIA_NOT_PRESENT` recorded.
- **`cpv_codes = []` with warning.** The notice carries CPV codes `50700000-2 / 50750000-7 / 71315000-9` at framework level, but the specification body does not cite any CPV code. I declined to propagate notice-level CPVs into the lot as unevidenced claims; warning `LOT_CPV_CODES_NOT_IN_DOCUMENT` records the deliberate abstention.
- **`product_requirement_seeds` favour deliverable/artefact seeds over physical goods.** Lot 5 is a surveying services lot; the only procurement-relevant "products" are the data-capture proforma, the handheld/tablet device, the AutoCAD drawing package, the BCIS/SPONS-based costing deliverable, and the financial plan deliverable. Each seed carries a `lot_number = "5"` back-reference and an evidence anchor. Warning `SERVICES_ORIENTED_LOT` flags this to downstream classification.
- **Evidence anchors keyed by section rather than page.** Docling reported `page_count = 0` so anchors use `section:<n>_<slug>` form. Warning `PAGE_NUMBERS_UNAVAILABLE` captures this.
- **`source_markdown_hash` reuses `source_content_hash` from `document_extractions`.** Re-hashing the daemon-extracted sqlite `extracted_markdown` column was not necessary because the daemon already records a content hash over that same body; aligning with it reduces drift risk between projector and extractor.
- **Stable seed keys.** Seed keys embed lot number and artefact family (`lot5-data-input-proforma`, `lot5-handheld-data-capture-device`, `lot5-autocad-drawings-deliverable`, `lot5-costing-schedule-of-rates`, `lot5-financial-planning-deliverable`) so that resume/retry runs reuse the same keys per the SKILL's resume strategy.

## Warnings

Eight warnings emitted in `result.json.warnings`:

1. `EVALUATION_CRITERIA_NOT_PRESENT` - specification body carries no tender evaluation criteria / weightings; these are expected in the ITT.
2. `LOT_CPV_CODES_NOT_IN_DOCUMENT` - CPV codes exist only at notice level; not propagated as unevidenced claims.
3. `APPENDIX_A_REFERENCED_BUT_NOT_INCLUDED_IN_EXTRACT` - Sections 11, 17, 18 reference Appendix A; inline definition tables present but standalone Appendix A not in extract.
4. `SECTION_NUMBERING_DUPLICATE` - Source uses both a "1./1./..." summary list and body-level numbering; body numbering preserved.
5. `PAGE_NUMBERS_UNAVAILABLE` - Docling `page_count = 0`; anchors use section refs.
6. `GOVERNANCE_NUMBERING_INCONSISTENT` - "21 Governance" block with 21.1-21.4 sub-items is numerically out of sequence (appears between 26 and 27); source-document artefact, not extraction error.
7. `SERVICES_ORIENTED_LOT` - professional-services engagement; product seeds represent deliverables/artefacts not physical goods.
8. `LIFE_EXPECTANCY_PLACEHOLDERS` - Section 30 lists three "life expectancy (tba)" rows; preserved as placeholder text, not imputed values.

## Evidence

All evidence anchors cite the extracted markdown for `document_extraction_id = 9e8c24cc-519b-4e6c-802c-5fcf0b27ad9a` (`source_content_hash = 2e1eed67144c8c783755b0cb50e09a1c1a07de2573f032eb8ebc805b21cc4e02`, 25,199 chars, `section_count = 58`, `page_count = 0`, `table_count = 2`).

Per-claim anchors:

- **Lot identity & scope** - title block ("London Borough of Bexley - LOT 5 - Specification For - Condition Surveys") and `section:1_overview` ("London Borough of Bexley is seeking to appoint a suitably qualified and accredited Chartered Surveying company to deliver a programme of Condition Surveys"). Existing row `notice_lots.pc-dc1daee2-...-lot-5` concurs (`lot_number = 5`, `title = 'LOT 5 - Condition Surveys'`).
- **RICS / CIPFA standards** - `section:1_overview` ("RICS and CIPFA guidance and best practice"), `section:10_objective` ("RICS and CIPFA best practice processes should be followed").
- **MRICS qualification** - `section:13_resources_and_conditions` ("Building Surveyors should be either MRICS qualified or similar professional accreditation").
- **BCIS / SPONS schedule of rates** - `section:19_works_identified` ("a recognised schedule of rates as Building Cost Information Service (BCIS) or SPONS pricing matrix").
- **A-D condition grading** - `section:17_condition_grading` table (A Good / B Fair / C Poor / D Failed).
- **P1-P4 priority grading** - `section:18_priority_grading` table (P1 Urgent / P2 Essential / P3 Desirable Mid-Term / P4 Long Term; 5-year planning period).
- **Proforma / CIVICA upload & tablet option** - `section:13_resources_and_conditions` and `section:23_data_collection` ("uploaded into the Council's property asset management database ... Survey data can be collected onsite using Tablet PC or in hard copy").
- **AutoCAD drawings** - `section:29_drawings` ("CAD ... drawings in digital format (AutoCAD or PDF) ... a new building drawing to be prepared in AutoCAD").
- **Financial planning & life expectancy indices** - `section:30_financial_planning` ("medium to long term financial plan ... life cycle expectancy / asset replacement programme ... RPI, CPI etc").
- **Statutory obligations** - `section:30_financial_planning` obligations list (EPC/DEC, Asbestos Management, Electrical Services Certification, Gas Safety, Water Safety, Fire Safety).
- **Asbestos awareness training** - `section:28_asbestos_awareness` ("all surveyors/engineers have received/completed adequate asbestos awareness training").
- **Site-visit / access rules** - `section:12_site_visits` and `section:20_access` (3m vertical-height access threshold; site rules enumerated 1-7).
- **Measurement discipline** - `section:22_measurement` ("Dimensions will be taken and recorded in metric millimetres").
- **Pilot survey** - `section:21_pilot_survey` ("A site/property will be identified by the Council for a pilot survey").

Daemon-observable corroboration:

- `document_extractions.id = 9e8c24cc-519b-4e6c-802c-5fcf0b27ad9a` with `semantic_extraction_state = pending`, `expected_overlays_json = ["pdf-extraction/tender-specification"]`, `document_role = technical_specification`, `extraction_policy_version = 2026.04.12`.
- `contract_notices.pc-dc1daee2-...` `title = "6278 - Building Services Framework - Lots 1-11"`, `buyer_name = "London Borough of Bexley"`; lot inventory in `notice_lots` enumerates Lots 1-11 with Lot 5 = "LOT 5 - Condition Surveys" (integer lot_number 5).
- Contract index `contract-output.index.json` confirms `identity.session_id` and `identity.job_id` match the execution context headers, tying this manifest to `result.json` under the same session.
