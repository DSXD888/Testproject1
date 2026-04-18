---
manifest_version: 1
skill_id: pdf-extraction/tender-specification
skill_version: 1.0.0
job_id: job-2622d999-393d-4d6b-89d0-b1b37f109073
session_id: session-semantic-2b8bbd1ba524e234606703e6963d1ffc794f97907397d2d767767820d44ebc50
execution_mode: contract
started_at: 2026-04-18T08:22:00Z
completed_at: 2026-04-18T06:31:08Z
outcome_class: success
items_processed: 1
items_skipped: 0
items_failed: 0
warnings_count: 6
agent_claimed_pages_visited: 79
agent_claimed_source_url: null
agent_claimed_fingerprint_state: not_applicable_offline_markdown_extraction
agent_claimed_locale_state: en-GB
agent_claimed_currency_state: GBP
---

# Run Manifest — pdf-extraction/tender-specification

## Summary

Extracted semantic lot structure, service-level product-requirement seeds, technical-standards references and evidence anchors from the 79-page Docling-produced markdown of ESPO Framework Agreement `107_26 Appendix A` (`document_extraction_id: a8b82b7e-1b2e-4ce5-90df-086bc9f6a783`). The document is the Framework Agreement template for the ESPO 107_26 Framework for Inspection & Maintenance of Indoor Gymnasium, Fitness & Sport Equipment and Outdoor Schools Play & Sport Equipment (notice `pc-3ba4e6b8-9cef-f011-813c-005056b64545`, buyer ESPO / Leicestershire County Council, published 2026-01-12, deadline 12:00 2026-05-15, Commencement Date 09 June 2026). Written `result.json` conforming to `result.schema.json` with `target_domain=lots` and `extraction_scope=full_document`.

One lot was emitted (matching the existing `notice_lots` record and the document's single compound title) because the Framework Agreement template itself does not label a 'Lot 1 / Lot 2' split; however, the title names two distinct equipment families and `clause 26.16` (Partial Termination) refers to 'Lots' (plural), so a `LOT_STRUCTURE_UNCERTAIN` warning is emitted asking downstream to cross-reference the ITT Specification (Section 2 of the Invitation to Tender, which is NOT contained in this document). Five product-requirement seeds were emitted covering the four service variants (inspection vs maintenance × indoor gym/fitness/sport vs outdoor schools play/sport) and a fifth seed for replacement parts/spares.

No explicit price-vs-quality evaluation criteria, no specific BS EN standards (BS EN 1176/1177, BS EN 957), and no specific CPV codes are printed in this document — Schedule 1 is a stub that defers to the separately-issued Specification pack. Process-context CPV codes (`37400000-2`, `50000000-5`) held by the daemon were used as hints on service-seed CPV lists but NOT recorded at the lot level because they are not cited in the document body. Reported `extraction_confidence_bps = 7000`.

## Steps Taken

1. Listed the session workdir and the parent session directory; confirmed the workdir contains only `storage.sqlite3*` scratch artifacts and no pre-placed markdown or job payload file.
2. Compared sibling `session-semantic-*` workdirs to confirm the agent is responsible for locating the Docling markdown in the daemon state database rather than reading a file from the workdir.
3. Located the daemon state database at `/Users/alinaaitova/Desktop/Testproject1/state/state.db` (the `runtime/state.db` and `Testproject1/state.db` paths are empty / placeholder).
4. Queried `contract_notices` for `id='pc-3ba4e6b8-9cef-f011-813c-005056b64545'` to resolve the process context: notice title, buyer (ESPO), `main_category` (`37400000-2 - Sports goods and equipment | 50000000-5 - Repair and maintenance services`) and `published_at` (`2026-01-12T09:30:00Z`).
5. Queried `notice_lots` for the notice, finding a single pre-existing lot record for the whole framework (noted as cross-reference only per skill contract).
6. Queried `document_extractions` for `id='a8b82b7e-1b2e-4ce5-90df-086bc9f6a783'` to confirm: `raw_document_id`, `document_set_id=tds-pc-3ba4e6b8-9cef-f011-813c-005056b64545`, `document_title='107_26-Appendix-A-Framework-Agreement'`, `page_count=79`, `extraction_engine=docling`, `expected_overlays_json=["pdf-extraction/tender-specification"]`, and `source_content_hash=00eb7cea0ff7a09b28b1dedb2f1471fc9baa518ebf99eac0e0f3f58209f9e23b`.
7. Exported `document_extractions.extracted_markdown` to `/tmp/tender-spec-md.md` (238,584 chars, 1,470 lines) and mapped structural headings (`##`-lead framework clauses and Framework Schedules 1–11).
8. Read the title block, `clause 1.1` definitions (`Standards`, `Data Protection Legislation`, `GHG Emissions`, `Equality Legislation`, `PA23`, `Prohibited Act`, `Commencement Date`), clauses 7, 9–18, 21–26, 28, and Framework Schedules 1, 2, 4, 5, 6, 7, 10 Annex A to confirm obligations, KPIs, and standards references.
9. Identified the inspection/maintenance service families and replacement-parts sub-scope from the framework title and from `clause 7.5`, `clause 12`, `clause 13.2`, `clause 24.1`, and Schedule 10 paragraph 2.2.4, since the actual Specification is not reproduced in this document.
10. Read the sample output from an unrelated prior run (`session-semantic-00032c3a...`) to confirm the result format expected by the daemon and the schema shape for `lots[].product_requirements` vs top-level `product_requirement_seeds`.
11. Read `skills/pdf-extraction/tender-specification/result.schema.json` to verify required fields and shape for `evaluation_criteria` (objects with `criterion` + `summary`, not bare strings) and `product_requirement_seeds` (top-level with `candidate_name` + `lot_number`).
12. Authored `result.json` in the workdir with 1 lot, 5 top-level product-requirement seeds, 11 technical-standards entries on the lot, 6 warnings, 18 evidence anchors, per-field confidences, and `extraction_metadata.source_markdown_hash` set to the daemon-recorded `source_content_hash`.
13. Validated `result.json` against the skill's `result.schema.json` using `jsonschema.Draft202012Validator` — reported `VALID`.

## Decisions Made

- **Single-lot emission (instead of two-lot split by equipment family).** The framework title names two equipment families joined by 'AND' and `clause 26.16` references 'Lots' (plural), but this Framework Agreement template does not label 'Lot 1' / 'Lot 2' boundaries inline and Schedule 1 (the formal Goods/Services container) is a stub. The existing `notice_lots` record is also a single lot. I treated the current document as insufficient evidence to split, emitted a single lot matching the title, and raised `LOT_STRUCTURE_UNCERTAIN` with explicit evidence references so downstream can revisit after ingesting the ITT Specification.
- **Evaluation criteria captured as one evidence-only entry.** Framework Schedule 5 does not list specific criteria or weightings — it only describes that Customers 'may adopt a scheme similar to that used by ESPO' at call-off. I emitted one evaluation_criteria entry with a neutral summary + evidence anchor and added `EVALUATION_CRITERIA_NOT_LISTED` warning rather than fabricating named criteria.
- **CPVs withheld at lot level, surfaced on service seeds.** No CPV codes are printed in the document; I kept `lots[0].cpv_codes = []` and surfaced the process-context CPVs (`37400000-2`, `50000000-5`) only on the product-requirement seeds as hints, with `CPV_NOT_IN_DOCUMENT` warning.
- **Technical standards emitted as generic BSI / UK statutes only.** The `Standards` definition (clause 1.1) is generic; no specific BS EN numbers appear. I listed the generic BSI/international standards entry plus the specific UK statutes actually cited in clauses 9, 12, 13, 14, 16, 21, 22, 26 and the Kyoto Protocol Annex A reference. The BS EN 1176/1177 families likely applying to outdoor play equipment are referenced only in free-text on the outdoor-play seeds with an explicit 'not explicitly named in this Framework Agreement template' caveat.
- **Product-requirement seeds enumerated by service family × equipment family.** Four service seeds (inspection indoor-gym, maintenance indoor-gym, inspection outdoor-play, maintenance outdoor-play) plus one goods seed (replacement parts/spares). Seed keys are stable `espo-107_26-lot1-<kind>-<family>` identifiers for idempotent re-runs.
- **Page references use the printed document page numbers** (front-matter, 1, 7, 9–15, 17–19, 24, 26, 29, 34, 36, 43, 56, 57, 58, 67, 77, 78) — matching how Docling labels the document — instead of synthetic markdown line numbers, because the skill's evidence model anchors on document pages.
- **Confidence = 7000 bps (70%).** Reflects high confidence on the title, scope, obligations, fees, KPI rows and standards references actually written in the document; and low confidence on lot structure, detailed specification content, specific BS EN numbers, and CPV codes — because those items are either ambiguous or not present. Field confidences range from 2000 (cpv_codes) to 9800 (title).

## Warnings

1. `LOT_STRUCTURE_UNCERTAIN` — Framework Schedule 1 is a stub; clause 26.16 implies plural Lots but the document does not label them. Cross-reference the ITT Specification (Section 2 of the ITT) before treating the single-lot structure as final.
2. `EVALUATION_CRITERIA_NOT_LISTED` — Framework Schedule 5 contains no specific criteria names or weightings. Framework-award evaluation criteria likely sit in the ITT, not in this Appendix A.
3. `SPECIFICATION_NOT_PRESENT` — Technical specification content (equipment catalogues, inspection frequencies, response SLAs, named BS EN standards) is not reproduced in this template.
4. `CPV_NOT_IN_DOCUMENT` — No CPV codes printed in this document. Process-context CPV hints (`37400000-2`, `50000000-5`) surfaced on seeds only.
5. `STANDARDS_REFERENCE_GENERIC` — `Standards` definition is generic (BSI Group / international standards / ESPO internal policies / Government codes). No specific BS EN numbers cited.
6. `KPI_TARGETS_CUSTOMER_SERVICE_ONLY` — The explicit KPI table (Schedule 10 Annex A) covers service-management KPIs only (complaints, MI reporting, contact accuracy, invoicing accuracy, Social Value). Technical inspection/maintenance performance KPIs are referenced only conceptually in paragraph 2.2.4.

## Evidence

All extraction claims are anchored to result-contract evidence anchors emitted in the `evidence_anchors` array of `result.json` and on each lot/seed/standard entry. Key anchors (all citing pages present in the Docling-produced markdown of this 79-page document; `source_markdown_hash = 00eb7cea0ff7a09b28b1dedb2f1471fc9baa518ebf99eac0e0f3f58209f9e23b` matches `document_extractions.source_content_hash`):

- **Title / scope** — Cover page 1 title block, `FRAMEWORK AGREEMENT FOR INSPECTION & MAINTENANCE OF INDOOR GYMNASIUM, FITNESS & SPORT EQUIPMENT AND OUTDOOR SCHOOLS PLAY & SPORT EQUIPMENT ... Agreement Ref: 107_26`.
- **Term** — clause 2.1 page 17, `twenty (20) Months (Initial Term) ... up to a maximum period of twenty eight (28) Months (Extended Term)`.
- **Commencement Date** — clause 1.1 page 7, `'Commencement Date' means 09 June 2026`.
- **Standards / statutory compliance** — clause 1.1 `Standards` definition page 15; clause 13.2 page 26 (`The Supplier shall comply with the Standards and the Law`).
- **Conformance of Goods** — clause 7.5 page 19 (`Goods supplied to the Customers pursuant to a Contract shall conform to a Specification and the Call-Off Terms`).
- **Schedule 1 stub and deferred Specification** — Framework Schedule 1 paragraph 1.1 page 56 and final paragraph page 57 (`ESPO will incorporate ... the Specification (contained within Section 2 of the Invitation to Tender, Requirement) and the Supplier's response to the Statement of Requirements`).
- **Framework Fee** — Framework Schedule 2 paragraph 1.4 page 58 (`The Framework Fee is 3.5%`).
- **Evaluation criteria deferral** — Framework Schedule 5 page 67 (`Customers may adopt a scheme similar to that used by ESPO ... or may advise Suppliers that a revised scheme will be used`).
- **Multiple lots implied** — clause 26.16 page 43 (`it may (at its absolute discretion) terminate some and not all of the Lots`).
- **Environmental management system / GHG reporting** — clause 12.5 page 24; clause 12.6–12.7 page 25; `GHG Emissions` definition clause 1.1 page 10.
- **KPIs** — Schedule 10 Annex A page 78 (Complaints Resolution, MI Reports, Contact accuracy, Invoicing, Social Value); Schedule 10 paragraph 2.2.4 page 77 (technical performance KPIs referenced conceptually).
- **Brand equivalence policy** — clause 24.1 page 36.
- **DBS / Regulated Activity** — clauses 13.3–13.6 page 26.
- **Social Value** — clause 12.10 page 26.

Process context fields (`notice_id`, `process_id`, buyer, published_at, CPV) were read from `contract_notices` and `document_extractions` rows in `state/state.db`; the resulting `result.json` records `notice_id` / `process_id` / `stage_run_id` via the daemon's job envelope (not duplicated into the payload). `document_extractions.extracted_markdown` was exported to `/tmp/tender-spec-md.md` for inspection; the authoritative hash used in `extraction_metadata.source_markdown_hash` is the daemon-recorded `source_content_hash`, not the local file hash (the local file's sha256 differs due to SQLite text-export newline handling).
