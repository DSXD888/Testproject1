---
manifest_version: "1.0"
skill_id: pdf-extraction/tender-specification
skill_version: 1.0.0
job_id: job-71a361bf-a749-4254-bcd5-88408e2f9f89
session_id: session-semantic-7650ef7b22101eee942c3b56a2d0043c760a35aad49a5ef44e09ddb59172a762
execution_mode: contract
started_at: "2026-04-18T10:52:00Z"
completed_at: "2026-04-18T10:55:00Z"
outcome_class: partial_success_with_warnings
items_processed: 1
items_skipped: 0
items_failed: 0
warnings_count: 6
agent_claimed_pages_visited: 79
agent_claimed_source_url: "local://document_extractions/4a9c75b0-fe06-460d-a4c8-86a9e542ea74"
agent_claimed_fingerprint_state: not_applicable_headless_contract
agent_claimed_locale_state: en-GB
agent_claimed_currency_state: GBP
---

# Run Manifest — pdf-extraction/tender-specification

## Summary

Performed one-pass semantic extraction over document_extraction `4a9c75b0-fe06-460d-a4c8-86a9e542ea74` — the 79-page Docling markdown rendering of "218_26 Framework Agreement (Appendix A) FINAL" (ESPO framework 218_26 for Specialist Vehicle Contract Hire / Leasing). The input document is a legal framework-agreement template, not a per-lot specification: it does not enumerate lot numbers or product-level technical specifications. Framework Schedule 1 explicitly defers the actual Specification to "Section 2 of the Invitation to Tender (Requirement)", which is not contained in this document.

To satisfy `min_lot_count: 1` and `lot_outputs_must_be_non_empty`, one framework-scope lot (`lot_number = "1"`) was emitted, anchored to the framework-wide scope statements (title block, Background A, clause 3 Scope, and Framework Schedule 1 Introduction). The three CPV codes from `process_context` (34114000, 34121000, 34134000) were attached at framework-lot level with a `CPV_CODES_FROM_PROCESS_CONTEXT_NOT_DOCUMENT` warning. One product_requirement_seed (`218_26-framework-specialist-vehicles`) captures the pricing-bundle composition expressly enumerated in Framework Schedule 1. Six warnings document the deferred-specification pattern, the absence of numeric award-criteria weightings, the statutory-not-technical nature of the captured standards, and a table-as-image observation.

## Steps Taken

1. Resolved input by reading `state/state.db` `document_extractions` row `4a9c75b0-fe06-460d-a4c8-86a9e542ea74` (process `proc-pc-218726bd-9dda-f011-813c-005056b64545`, notice `pc-218726bd-9dda-f011-813c-005056b64545`, raw_document hash `b83fbdbe…`). Extracted the full 230,230-character markdown to `/tmp/tender-spec.md` (1678 lines, UTF-8).
2. Confirmed document-set metadata: `document_title` = "218_26 Framework Agreement (Appendix A) FINAL"; `document_role` = `technical_specification`; `extraction_engine` = `docling`; `extraction_policy_version` = `2026.04.12`; `expected_overlays_json` = `["pdf-extraction/tender-specification"]`; `source_content_hash` = `a649ec7ddecc…862ae` (PDF-level).
3. Enumerated structural headings (90+ `##` headings). Observed no lot enumeration markers (no "Lot 1"/"Lot 2"/"LOT N") anywhere in the document; "Lot" appears only as a defined term, in the call-off-without-competition procedure, and in the MI-template header.
4. Located and read the three load-bearing sections of interest: clause 3 (Scope), clause 13 (Statutory Requirements and Standards), Framework Schedules 1, 2, 4, 5, 6, 7, and 10-Annex A (KPIs).
5. Cross-checked `notice_lots` for notice `pc-218726bd-9dda-f011-813c-005056b64545` — empty set (no pre-existing lot rows to reconcile).
6. Pulled `process_context` CPV codes from `contract_notices.main_category` for this notice: "34114000-9 - Specialist vehicles | 34121000-1 - Buses and coaches | 34134000-5 - Flatbed and Tipper trucks" — attached to the framework-scope lot with explicit provenance warning.
7. Computed source_markdown_hash (SHA-256 of the retrieved markdown) = `13b6dd577f2c742764c097a2122865695d90c8dcd8abc52918f008918002c3e8`.
8. Drafted `result.json` and validated it against `skills/pdf-extraction/tender-specification/result.schema.json` via `jsonschema.validate` — passed.
9. Wrote this `run-manifest.md`.

## Decisions Made

- **Emit one framework-scope lot rather than fabricate per-CPV lots.** The document does not partition its scope by CPV or by vehicle family, and the skill's non-goals explicitly prohibit "Guessing lot boundaries … without citing evidence anchors." Inventing three lots from the process-level CPV list would manufacture a structure the document itself does not assert.
- **Carry process-context CPV codes forward, but warn.** The daemon caller supplied three CPV codes at notice level. Because they are not printed inside this document, they are attached with warning `CPV_CODES_FROM_PROCESS_CONTEXT_NOT_DOCUMENT` rather than silently promoted to authoritative per-lot CPVs.
- **Classify MOT / VED / first-registration-fee as "technical_standards" entries with statutory context.** They are the only externally-identifiable regulatory references in the document body; the alternative was an empty `technical_standards` array with a warning, but the skill rubric treats these as valid standard_refs when present. A warning (`NO_TECHNICAL_STANDARDS_WITH_NUMERIC_CODES`) documents that no ISO/EN/BS/DIN numeric codes appear.
- **Evaluation criteria entries carry `weighting_text` narratives rather than numeric weights.** Framework Schedule 5 expressly defers weightings to per-customer further-competitive-selection schemes; no weights are stated.
- **`extraction_confidence_bps = 5800`.** Calibrated to reflect genuine evidence of the framework-wide scope (~0.8) discounted by the absence of the actual Specification content (~0.7 confidence that lot boundaries / product requirements exist elsewhere and are not being captured here).
- **Did not emit per-schedule lots (e.g. Schedule 1 vs Schedule 10).** Framework Schedules are contractual annexes of one agreement, not commercial lots of a procurement; emitting one per schedule would mis-represent lot semantics.

## Warnings

1. `LOT_BOUNDARY_NOT_ENUMERATED_IN_DOCUMENT` — one placeholder framework-scope lot emitted; real lots (if any) live in the ITT Requirement document.
2. `SPECIFICATION_DEFERRED_TO_EXTERNAL_DOCUMENT` — per Framework Schedule 1, Specification is incorporated later from ITT Section 2.
3. `AWARD_CRITERIA_WEIGHTINGS_NOT_SPECIFIED` — Framework Schedule 5 does not set numeric weightings.
4. `CPV_CODES_FROM_PROCESS_CONTEXT_NOT_DOCUMENT` — CPV codes sourced from notice metadata, not printed in the document body.
5. `NO_TECHNICAL_STANDARDS_WITH_NUMERIC_CODES` — only UK statutory regimes and named legislation present; no ISO/EN/BS/DIN codes cited.
6. `MI_REPORTING_TEMPLATE_IS_IMAGE` — the Annex MI Reporting Template is partly an image placeholder; Docling output of that cell may be incomplete.

## Evidence

All major claims trace to specific markdown anchors in `document_extractions.4a9c75b0-fe06-460d-a4c8-86a9e542ea74.extracted_markdown` (paths expressed as section headings):

- Title / scope: `FRAMEWORK AGREEMENT FOR SPECIALIST VEHICLE CONTRACT HIRE / LEASING — Agreement Ref: 218_26` (title block) and `BACKGROUND A` ("… specialist vehicle contract hire / leasing to Customers …").
- Framework-scope obligations: `## 3. SCOPE OF FRAMEWORK AGREEMENT` (3.1-3.6), `## 7. WARRANTIES, REPRESENTATIONS AND UNDERTAKINGS` (7.5), `## 13 STATUTORY REQUIREMENTS AND STANDARDS` (13.1-13.2).
- Pricing-bundle composition: `## FRAMEWORK SCHEDULE 1 - THE GOODS AND/OR SERVICES`, "Tendered pricing shall include …" block; same schedule's administrative-charges list and closing paragraph deferring Specification to the ITT.
- Framework fee: `## FRAMEWORK SCHEDULE 2 - CHARGING STRUCTURE`, `The Framework Fee:` subsection, clause 1.4 ("The Framework Fee is 1%.").
- Ordering procedures: `## FRAMEWORK SCHEDULE 4 - ORDERING PROCEDURE`, `## 2. AWARD PROCEDURE (CALL OFF WITHOUT COMPETITION)` and `## 3. AWARD PROCEDURE (FURTHER COMPETITIVE SELECTION PROCESS)`.
- Award-criteria deferral: `## FRAMEWORK SCHEDULE 5 - FURTHER COMPETITIVE SELECTION PROCESS AWARD CRITERIA` (single narrative paragraph).
- Benchmarking / VfM: `## FRAMEWORK SCHEDULE 6 - VALUE FOR MONEY`, `## 2. BENCHMARKING` (2.1-2.2).
- MI reporting: `## FRAMEWORK SCHEDULE 7 - MANAGEMENT INFORMATION REQUIREMENTS`, `## 3. FREQUENCY AND COVERAGE`.
- KPIs: `## FRAMEWORK SCHEDULE 10 … ANNEX A — KEY PERFORMANCE INDICATORS` (Complaints Resolution, MI Report Accuracy/Timeliness, Information Update accuracy, Invoicing accuracy, Social Value).
- Legislation references: Definitions (`'Data Protection Legislation'`, `'Equality Legislation'`, `'PA23'`, `'Conduct Policy'`); clauses 7.1.8, 7.1.14, 9, 14, 21.
- CPV codes (process_context only): `state.db → contract_notices.main_category` for notice `pc-218726bd-9dda-f011-813c-005056b64545` = "34114000-9 | 34121000-1 | 34134000-5".
- `result.json` at workdir root; schema-validated against `skills/pdf-extraction/tender-specification/result.schema.json` via `jsonschema.validate`.
- `source_markdown_hash` = `13b6dd577f2c742764c097a2122865695d90c8dcd8abc52918f008918002c3e8` (SHA-256 of the 230,230-byte markdown payload).
