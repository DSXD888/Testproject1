---
manifest_version: 1.0.0
skill_id: pdf-extraction/tender-specification
skill_version: 1.0.0
job_id: job-ea1f600c-e8b3-4e29-99e1-201c17507349
session_id: session-semantic-6fbad5299ce7c59ef1c1b6cbbbd3316fa0338853a604bdcb19f6e8a4bdf183e2
execution_mode: contract
started_at: 2026-04-17T22:30:24Z
completed_at: 2026-04-17T22:31:43Z
outcome_class: success_with_warnings
items_processed: 1
items_skipped: 0
items_failed: 0
warnings_count: 5
agent_claimed_pages_visited: 0
agent_claimed_source_url: "https://procontract.due-north.com/RfxResponse/Index?rfxId=2e1e258d-5c80-ed11-811c-005056b64545"
agent_claimed_fingerprint_state: not_applicable_contract_mode
agent_claimed_locale_state: en-GB
agent_claimed_currency_state: GBP
---

# Run Manifest — pdf-extraction/tender-specification

## Summary

Headless contract execution of the `pdf-extraction/tender-specification` skill against
document_extraction_id `9da44da4-cd76-40de-9162-356cb3bc1465` (raw_document
`raw-pc-b7aaab27-1f34-...-006b9ee6`, source DOCX
`Appendix-1-Evaluation-Criteria.docx`, ~343 KB extracted markdown,
4 tables, 5 sections, page_count=0 reported by Docling for this DOCX).

The document is `Appendix 1 — Evaluation Criteria` for the Bristol City Council
`BSH/HRA/Joinery Timber Supplies — DPS — Acceptance Round 7` Dynamic
Purchasing System (notice `pc-b7aaab27-1f34-ed11-8119-005056b64545`,
process `proc-pc-b7aaab27-...`, document set `tds-pc-b7aaab27-...`).

The document content is a **procurement evaluation criteria appendix** (a
supplier-selection questionnaire under Table A plus a call-off award scoring
methodology), not a technical product specification. It contains
**no product requirements, no quantities, no engineering attributes and no
referenced technical standards**. The persisted `document_role` of
`technical_specification` therefore appears mismatched against the actual
content, and one warning has been raised to flag this for the daemon /
operator.

A single overarching synthetic lot (`lot_number = "DPS-ALL"`) has been emitted
to faithfully represent the document's actual scope. It captures:
- 9 distinct evaluation criteria (4 selection criteria families,
  1 financial-standing rule, 4 award-criteria heads for call-offs above and
  below GBP 25,000, plus a tie-breaker rule),
- 9 hard requirements drawn directly from the document,
- empty `cpv_codes`, empty `product_requirements`, empty `technical_standards`
  (the document does not state any of these and the skill spec explicitly
  forbids guessing them).

`product_requirement_seeds` is correctly emitted as an empty array because
the document carries no product seeds. The result contract has been validated
against `result.schema.json` locally and passes.

## Steps Taken

1. Resolved working directory at
   `artifacts/sessions/2026/04/session-semantic-...-bdf183e2/workdir` and
   confirmed it contained only daemon storage (no pre-staged extraction
   payload). Located the canonical extraction record by
   `document_extraction_id` in `state/state.db.document_extractions`.
2. Loaded extraction metadata: `extraction_engine`, `processing_strategy`,
   `extraction_policy_version = 2026.04.12`,
   `expected_overlays_json = ["pdf-extraction/tender-specification"]`,
   `semantic_extraction_state = pending`,
   `document_title = "Appendix-1-Evaluation-Criteria"`,
   `document_role = "technical_specification"`,
   `page_count = 0`, `table_count = 4`, `section_count = 5`,
   `image_count = 0`, `has_ocr_content = 0`,
   `extracted_markdown` length = 342 722 bytes.
3. Cross-referenced parent context by reading
   `contract_notices(pc-b7aaab27-...)` and its `normalized_payload`. Confirmed
   notice title `BSH/HRA/Joinery Timber Supplies - DPS - Acceptance Round 7`,
   buyer `Bristol City Council`, CPV main category
   `03419000-0 - Timber | 44220000-8 - Builders' joinery`, deadline
   `2026-11-30T20:00:00Z`. Confirmed the notice's
   `documents_downloaded` array contains four DOCX files (this Appendix 1
   evaluation criteria, the ITT, the Schedule 7 Call-Off Contract and the DPS
   Agreement); product requirements would naturally live in one of the other
   three, not in this appendix.
4. Confirmed there are no pre-existing `notice_lots` rows for this notice
   (`SELECT … FROM notice_lots WHERE notice_id = 'pc-b7aaab27-…'` returns 0
   rows), so there was no prior lot scaffolding to either reuse or contradict.
5. Dumped `extracted_markdown` to a scratch file and walked the structural
   skeleton (`grep ^#`):
   - `## APPENDIX 1: EVALUATION CRITERIA` (line 1)
   - `### 1 Selection Criteria` (line 3)
   - `### 2 Award Criteria for Call-offs` (line 153)
   Read both halves of the document in full (table A selection criteria +
   the financial-standing narrative + the call-off scoring methodology +
   the tie-breaker clause).
6. Classified the document content. Determined that:
   - Section 1 = a supplier-selection questionnaire (Table A: questions
     1.1-1.20 = supplier identity info-only; section 2 = prime/main contractor
     info-only; section 3 = pass/fail compliance; followed by financial
     standing narrative resolving to a Low/Medium/High Risk Pass/Fail).
   - Section 2 = a call-off award methodology (Price 100% under GBP 25,000;
     Price 20-80% / Quality 0-60% / Social Value 20% at or above GBP 25,000;
     5-point quality scoring matrix; tie-breaker clause).
   - The document does **not** contain lot enumerations, product
     specifications, quantities, technical attributes, or engineering
     standards. Question 1.20 references "categories" the bidder may
     elect, but their definitions are not in this document.
7. Computed `source_markdown_hash` over the persisted `extracted_markdown`
   bytes (SHA-256:
   `7574ef83be90228fa51e28017d0f1839d60e676f58758b81c8dff4ae72e94634`).
8. Authored `result.json` with one synthetic `DPS-ALL` lot, empty
   `product_requirement_seeds`, 9 evaluation criteria each with a per-criterion
   `evidence_anchor`, 9 document-level `evidence_anchors`, 5 warnings, and an
   `extraction_metadata` block with `target_domain = "lots"`,
   `extraction_scope = "full_document"`, `policy_version = "2026.04.12"`
   and `document_role = "technical_specification"` echoed for audit.
9. Validated the result file against
   `skills/pdf-extraction/tender-specification/result.schema.json` using
   `jsonschema` — passed. Confirmed all `required` arrays satisfied and
   every per-lot `evaluation_criteria` item carries the required
   `criterion` and `summary` fields.

## Decisions Made

- **One synthetic lot vs zero lots vs many guessed lots.** Emitting zero
  lots would discard real evaluation-criteria content the daemon needs;
  emitting per-category lots ("Hardwood Timber", "Softwood Timber",
  "Boards & Door Blanks") would violate the skill's explicit instruction
  not to guess lot boundaries without evidence. Chose to emit a single
  `DPS-ALL` lot whose `description` openly states the document does not
  enumerate categories, so downstream `notice_lots` writes are not seeded
  with phantom rows but the evaluation criteria are still captured.
- **`cpv_codes`: emitted as empty array.** The document body itself does
  not cite any CPV code. The notice carries
  `03419000-0 - Timber | 44220000-8 - Builders' joinery` at notice
  level, but per the skill spec ("Existing notice-level lot metadata for
  cross-reference only") and the explicit
  "Do not guess … without citing evidence anchors" rule, those CPVs were
  not back-propagated into the lot row.
- **`product_requirements` (within the lot) and top-level
  `product_requirement_seeds`: both empty.** No product specifications,
  attributes, quantities or standards were present anywhere in the
  document.
- **`technical_standards`: empty.** The document contains zero standards
  references (no BS, EN, ISO, FSC, PEFC, CE, UKCA citations; no species,
  grade, dimensional or moisture-content requirements).
- **Confidence at 6500 bps.** Ground-truth on the evaluation criteria is
  high (text and tables are unambiguous), but the overall extraction
  confidence is dampened because (a) the bound skill is
  `tender-specification` and the document is functionally an
  evaluation-criteria appendix, so there is a known role mismatch; (b)
  the document deliberately defers per-call-off weighting tailoring with
  template placeholders (`xx%`, `xx marks`); (c) supplier-elected
  categories are referenced but not defined.
- **`agent_claimed_pages_visited = 0`.** This is contract-mode execution
  against a Docling-extracted DOCX whose own `page_count` is reported as
  `0`; no browser navigation occurred.

## Warnings

Five warnings were attached to `result.warnings`:

1. `AGENT_DOCUMENT_ROLE_MISMATCH` — content is an evaluation-criteria
   appendix, not a technical specification.
2. `AGENT_NO_EXPLICIT_LOTS_IN_DOCUMENT` — supplier-elected categories
   exist (per question 1.20 and section 2's "the DPS for the relevant
   category") but are not defined here; a single `DPS-ALL` lot was
   emitted to avoid inventing per-category rows.
3. `AGENT_NO_PRODUCT_REQUIREMENTS_PRESENT` — no product seeds; product
   detail must come from the ITT, Schedule 7 Call-Off Contract or DPS
   Agreement DOCX bundled in the same notice.
4. `AGENT_DOCUMENT_HAS_TEMPLATE_PLACEHOLDERS` — the at-or-above-GBP-25,000
   pricing subsection contains literal `xx%` / `xx marks` placeholders;
   the captured weighting range was taken from the upstream
   weighting-range table.
5. `AGENT_NUMBERING_ANOMALY_IN_SOURCE` — the source DOCX repeatedly
   re-uses `1.1.` numbering inside multiple narrative subsections; this
   is an authoring artefact, not a parser fault.

No `AGENT_OUTPUT_CONTRACT_INVALID`, `AGENT_SUSPECT_OUTPUT`,
`AGENT_SKILL_FAILED`, `AGENT_RUN_FAILED` or `AGENT_PARTIAL_FAILURE`
conditions were triggered.

## Evidence

All major claims map to observable fields in the persisted extraction record
or to anchors recorded in `result.evidence_anchors`:

- Document identity and structural shape:
  `state.db.document_extractions(id='9da44da4-…-1465')` with
  `document_title = 'Appendix-1-Evaluation-Criteria'`,
  `document_role = 'technical_specification'`, `table_count = 4`,
  `section_count = 5`, `extraction_policy_version = '2026.04.12'`,
  `expected_overlays_json = '["pdf-extraction/tender-specification"]'`.
- Source DOCX: `state.db.raw_documents(id='raw-pc-b7aaab27-…-006b9ee6')`
  with `storage_path =
  artifacts/downloads/procontract/621558/Appendix-1-Evaluation-Criteria.docx`
  and `source_url =
  https://procontract.due-north.com/RfxResponse/Index?rfxId=2e1e258d-5c80-ed11-811c-005056b64545`.
- Parent notice context:
  `state.db.contract_notices(id='pc-b7aaab27-…-b64545')` with title
  `BSH/HRA/Joinery Timber Supplies - DPS - Acceptance Round 7`, buyer
  `Bristol City Council`, main_category
  `03419000-0 - Timber | 44220000-8 - Builders' joinery`. The notice's
  `normalized_payload.documents_downloaded` lists the four DOCX files in
  the pack.
- Section anchors recorded in `result.evidence_anchors`:
  `APPENDIX 1: EVALUATION CRITERIA` (`#/texts/0`),
  `1 Selection Criteria` (`#/texts/1`),
  `2 Award Criteria for Call-offs` (`#/texts/164`).
- Table anchors recorded in `result.evidence_anchors`:
  `Table A - Selection Criteria`,
  `Definition / Scoring methodology` (Low / Medium / High risk →
  Pass/Pass/Fail),
  `Weighting Range for call-offs with a value below GBP 25,000`
  (`Price | 100%`),
  `Weighting Range for call-offs with a value of GBP 25,000 or above`
  (`Price | 20% - 80% | Quality | 0% - 60% | Social Value | 20%`),
  `Scoring Matrix` (Unacceptable=0 … Excellent=5),
  `Tie Breaker` (highest social value score wins).
- Source markdown digest:
  `sha256:7574ef83be90228fa51e28017d0f1839d60e676f58758b81c8dff4ae72e94634`,
  computed over the bytes returned by
  `SELECT extracted_markdown FROM document_extractions WHERE id = '9da44da4-…-1465'`.
- No-lot-rows-yet evidence:
  `SELECT id, lot_number, title, length(description), is_bidable_separately
   FROM notice_lots WHERE notice_id = 'pc-b7aaab27-…'` returned 0 rows
   prior to this run, so the extracted lot is not contradicting any
   pre-existing authoritative `notice_lots` content.
- Result contract artefact:
  `artifacts/sessions/2026/04/session-semantic-…-bdf183e2/workdir/result.json`
  validated by `jsonschema` against
  `skills/pdf-extraction/tender-specification/result.schema.json`.
