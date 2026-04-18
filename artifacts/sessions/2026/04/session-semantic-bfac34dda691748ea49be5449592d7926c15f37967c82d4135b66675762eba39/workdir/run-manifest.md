---
manifest_version: 1.0.0
skill_id: pdf-extraction/tender-specification
skill_version: 1.0.0
job_id: job-df28b29b-da99-4749-9148-e9efc24a218b
session_id: session-semantic-bfac34dda691748ea49be5449592d7926c15f37967c82d4135b66675762eba39
execution_mode: contract
started_at: 2026-04-18T00:00:00Z
completed_at: 2026-04-18T00:00:00Z
outcome_class: success
items_processed: 1
items_skipped: 0
items_failed: 0
warnings_count: 6
agent_claimed_pages_visited: 0
agent_claimed_source_url: null
agent_claimed_fingerprint_state: not_applicable
agent_claimed_locale_state: en-GB
agent_claimed_currency_state: GBP
---

# Run Manifest — pdf-extraction/tender-specification

## Summary

Single-pass semantic extraction of the tender specification document `cefe1b40-9596-4084-97be-51009925fb77` (raw doc `raw-pc-dc1daee2-772e-f111-813c-005056b64545-611c79650fd0cf3bae6b04fa120790c4a8bd0b2646106eef8f44d0ca515b4ab7`) for process `proc-pc-dc1daee2-772e-f111-813c-005056b64545` (London Borough of Bexley, "6278 - Building Services Framework - Lots 1-11").

The input document is titled "6278 - Draft specification - Lot 7" (document_role `technical_specification`) and covers lift servicing, maintenance, repair, minor refurbishment and compliance management for the Council's 24 lift assets across 12 sites. Output `result.json` emits exactly one authoritative lot (Lot 7) consistent with the parent notice (`notice_lots` already holds `lot_number = 7` under `pc-dc1daee2-772e-f111-813c-005056b64545-lot-7`), with 7 product-requirement seeds, 20 technical standards, 25 textual requirements, 12 top-level evidence anchors and 6 warnings. `target_domain = "lots"` and `extraction_scope = "full_document"` as required by the job parameters and skill contract.

Extraction confidence: 8200 bps (high). Principal confidence discounts are on (a) CPV codes, which are not cited anywhere in the document body, and (b) evaluation criteria, which the document frames as KPI-based performance monitoring rather than a scored tender matrix.

## Steps Taken

1. Located the source Docling markdown in the local state database: `sqlite3 state/state.db` → `document_extractions` row `cefe1b40-9596-4084-97be-51009925fb77`. Verified `page_count = 0`, `table_count = 0`, `section_count = 0` (Docling emitted body-text only), `extraction_engine` recorded, `semantic_extraction_state = pending`.
2. Computed content hash over the exact UTF-8 bytes of `extracted_markdown` (length 16 713 bytes): `sha256:66cd85c1cb5b0709d0ed00d1cbc95eea15b1b0277ea33db13c5c1f26deb27dec`. Recorded in `extraction_metadata.source_markdown_hash`.
3. Read the process and notice context from `procurement_processes` (`buyer_name = "London Borough of Bexley"`, `title = "6278 - Building Services Framework - Lots 1-11"`) and surveyed `notice_lots` for the parent notice to confirm this is a multi-lot framework with 11 lots and that Lot 7 already exists at the notice level with a services-contract description consistent with the body text. Did not copy any fields from existing notice-lot rows; used them only as cross-reference per the skill's "do not treat pre-existing lot rows as authoritative truth" rule.
4. Segmented the markdown by heading line (Docling collapses every section to a `1.` prefix, so anchoring is by heading text + line range, not numeric prefix) and classified sections into: scope & governance, service levels & operations, PPM & examination regimes, inclusive parts (traction / hydraulic), safety & legislation, data protection & telephony, systems (CAFM integration), and reporting/KPI.
5. Built the single lot record:
   - `lot_number = "7"`, `title` synthesized from the header "Lift Specification" and the scope sentence in the Introduction.
   - `description` summarizes scope, portfolio size (24 assets / 12 sites), 24/7 entrapment response, CAFM (Civica/TF Cloud) integration, 3+2 year term, Sept 2026 start.
   - `requirements` = 25 bullet-level operational requirements covering service levels (1h/4h/NWD), PPM cadence (monthly passenger, quarterly service/platform), LOLER cadence (6-monthly passenger, 12-monthly goods-only), SAFed LG1 triggers, OEM-equivalent parts, CDM 2015, asbestos (CAR 2012), CAFM/mobile platform, telephony, RAMS, training (EN 81-20/50, BS 7255:2023), data protection (UK GDPR / DPA 2018), equality, sustainability, sub-contracting, BCP, social value, mobilisation Operating Manuals (6-week window), monthly consolidated invoicing.
   - `evaluation_criteria` captured as two KPI-based entries with a warning that the document does not publish a scored evaluation matrix.
   - `cpv_codes` left empty with a warning — the body cites none.
   - `technical_standards` = 20 explicit references with context + anchor (EN 81-20:2020, EN 81-50:2020, Lifts Regulations 2016 (UK), ISO 8100-1:2019, BS 9991:2024, LOLER 1998, PUWER 1998, EAWR 1989, RIDDOR 2013, HSW Act 1974, CAR 2012, CDM 2015, EN 13015, SAFed LG1, BS 7255:2023, UK GDPR, DPA 2018, FOIA 2000, Equality Act 2010, Approved Document M (2024 amendments)).
   - `product_requirements` = 7 seeds spanning inclusive consumables (traction, hydraulic), EN 81-20 light-curtain door protection, lift energy-efficiency components (LED, standby, regenerative drives), Supplier CAFM-integration platform, 24/7 telephony/entrapment answer service, and SAFed LG1 supplementary testing service.
6. Mirrored `product_requirements` into top-level `product_requirement_seeds` with the same `seed_key` values (stable seed keys: `lot7-traction-inclusive-parts`, `lot7-hydraulic-inclusive-parts`, `lot7-door-light-curtain`, `lot7-energy-efficiency-components`, `lot7-cafm-integration-platform`, `lot7-telephony-service`, `lot7-safed-lg1-supplementary-tests`) and added per-seed requirements, technical standards and an evidence anchor.
7. Assigned field confidences in basis points (10000 = 100%): lot number/title/description ≥8800, requirements 8500, product requirements 7800, standards 9200, evaluation criteria 5000, cpv codes 2000. Set overall `extraction_confidence_bps = 8200`.
8. Recorded 6 warnings (lot-number inference, no CPV in document, evaluation criteria not scored, line-based page refs, duplicated Docling section numbering, document marked "Draft").
9. Validated `result.json` against `skills/pdf-extraction/tender-specification/result.schema.json` using `jsonschema`: PASS.

## Decisions Made

- **Scope = single lot (Lot 7).** The extraction is `full_document` over a single-lot specification. Emitting multiple lots would not be evidence-bearing; the document never claims to cover any other lot. Warning `LOT_NUMBER_INFERRED_FROM_TITLE` records the provenance.
- **`cpv_codes = []`.** No CPV string appears in the body. Although 50750000 "Lift-maintenance services" is the natural notice-level match, emitting it from this skill would fabricate evidence. Daemon-side enrichment / notice-level CPV is the correct source.
- **Evaluation criteria expressed as KPI regime, not scored matrix.** The body defines KPI categories (availability, first-time fix, statutory compliance) but no weightings. Recorded two evidenced entries with confidence 5000 bps and warning `EVALUATION_CRITERIA_NOT_SCORED`.
- **Line-range anchors over page anchors.** Docling stored `page_count = 0`; anchors therefore use `lines-<start>-<end>` with a `section_ref` carrying the heading text — both audit-recoverable from the stored markdown. Warning `PAGE_REFS_LINE_BASED` records this.
- **Technical standards kept broad and specific.** Twenty standards emitted because the "Legislation & Standards" section (lines 225-227) enumerates an exact list and other sections reinforce each citation. Every standard entry points to a body sentence, not only the list.
- **Product-requirement seeds treat services (telephony, CAFM platform, SAFed LG1) as seeds too.** The skill's "product-requirement seeds" are candidate seeds for downstream matching; in a services framework these include service components, not only physical goods. Each seed cites a specific section.
- **`result_schema_version` alignment.** Output matches the `result.schema.json` at `1.0.0` (skill frontmatter). `target_domain = "lots"` is the only allowed constant in `extraction_metadata.target_domain`.

## Warnings

1. `LOT_NUMBER_INFERRED_FROM_TITLE` — Lot 7 inferred from `document_title` and the header "Lift Specification"; body does not enumerate its lot number.
2. `NO_CPV_IN_DOCUMENT` — No CPV codes cited in the body; `cpv_codes` left empty.
3. `EVALUATION_CRITERIA_NOT_SCORED` — Document defines KPI monitoring but no scored evaluation matrix.
4. `PAGE_REFS_LINE_BASED` — Docling markdown has no page boundaries (`page_count = 0`); anchors use line ranges.
5. `DUPLICATED_SECTION_NUMBERS` — Every section renders as `1.` in the Docling output; anchors use heading text rather than numeric prefix.
6. `DOCUMENT_MARKED_DRAFT` — Source titled "Draft specification"; downstream decisions should treat requirements as provisional.

## Evidence

All major claims are anchored to specific lines / sections in `document_extractions.extracted_markdown` (id `cefe1b40-9596-4084-97be-51009925fb77`, source hash `sha256:66cd85c1cb5b0709d0ed00d1cbc95eea15b1b0277ea33db13c5c1f26deb27dec`):

- Scope, portfolio size (24 assets, 12 sites), contract term (3+2 years), Sept 2026 start, core compliance triad (EN 81-20:2020, EN 81-50:2020, Lifts Regulations 2016 (UK), BS 9991:2024) — lines 11–18, section **Introduction and Context**.
- ISO 8100-1/-2 convergence tracking — lines 17–18, **Introduction and Context**.
- 24/7 telephony and entrapment call routing — lines 49–51, **Telephony**.
- Service-level response times (1 hour / 4 hours / next working day) — lines 137–139, **Service Hours and Priorities**.
- Traction inclusive consumables (oils, greases, fuses, brushes, lamps, contacts/switch gear) and EN 81-20/50 performance requirement — lines 149–152, **Traction Lifts — Inclusive Parts**.
- Hydraulic inclusive components (seals, packings, hoses, pumps, valves, motors) with stated exclusions — lines 153–156, **Hydraulic Lifts — Inclusive Parts**.
- EN 13015 PPM execution — lines 165–167, **Servicing**.
- Non-contact curtain-of-light door detection under EN 81-20 on passenger service lifts — lines 173–175, **Routine Servicing — Passenger Service Lifts & Associated Equipment**.
- 6-month LOLER for person-carrying platform lifts — lines 177–179, **Routine Servicing — Service Lifts, Platforms, Access Lifts & Associated Equipment**.
- LOLER cadence (6-monthly passenger/goods-passenger, 12-monthly goods-only), SAFed LG1 supplementary tests, 28-day certificate deadline — lines 181–183, **Examination & Testing of Lifts (LOLER & SAFed)**.
- CAFM integration requirement (mobile workflows, photo evidence, API/CSV exchange with Civica/TF Cloud) — lines 213–215, **Suppliers' System Capability Requirements**.
- Legislation & standards enumeration (HSW Act 1974, LOLER 1998, PUWER 1998, EAWR 1989, RIDDOR 2013, Lifts Regulations 2016 (UK), EN 81-20:2020, EN 81-50:2020, ISO 8100-1:2019, Approved Document M (2024), BS 9991:2024, SAFed LG1, UK GDPR, DPA 2018) — lines 221–227, **Compliance / Legislation & Standards**.
- BS 7255:2023 training requirement — lines 97–99, **Training & Staff**.
- UK GDPR / DPA 2018 substitution for DPA 1998; FOIA 2000 routing — lines 43–47, **Data Protection and Freedom of Information**.
- Asbestos compliance (CAR 2012) — lines 23–24 and lines 191–195.
- CDM 2015 cooperation on refurbishment projects — lines 37–38, **Management and Organisation of Resources**.
- KPI framework — lines 39–41, **Continuous Improvement and Performance Management**; and lines 277–279, **Monitoring & Reporting**.
- Energy/environmental measures (LED, standby modes, regenerative drives) — lines 73–75 (**Environmental Requirements**) and lines 257–259 (**Energy & Environmental Assessments**).

Artifact refs:
- Input: `state/state.db::document_extractions[id=cefe1b40-9596-4084-97be-51009925fb77]` (extracted_markdown, length 16 713 bytes).
- Output: `workdir/result.json` (this session's output artifact; validates against `skills/pdf-extraction/tender-specification/result.schema.json`).
- Parent process: `state/state.db::procurement_processes[id=proc-pc-dc1daee2-772e-f111-813c-005056b64545]`.
- Notice lot cross-reference (not used as authority): `state/state.db::notice_lots[id=pc-dc1daee2-772e-f111-813c-005056b64545-lot-7]`.
