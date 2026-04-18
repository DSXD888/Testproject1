---
manifest_version: 1
skill_id: pdf-extraction/tender-specification
skill_version: 1.0.0
job_id: job-a0cc2162-2d97-43a7-9211-aa52f38799f0
session_id: session-semantic-dfccb099a7d2feea4583b8471b5c2826fb86fa73be2fcb817bd8adc8178b90f8
execution_mode: contract
started_at: 2026-04-17T21:08:00Z
completed_at: 2026-04-17T21:13:10Z
outcome_class: success_with_warnings
items_processed: 1
items_skipped: 0
items_failed: 0
warnings_count: 8
agent_claimed_pages_visited: 0
agent_claimed_source_url: null
agent_claimed_fingerprint_state: not_applicable
agent_claimed_locale_state: en-GB
agent_claimed_currency_state: GBP
---

# Run Manifest — pdf-extraction/tender-specification

## Summary

Extracted DPS-umbrella lot structure, generic two-stage evaluation criteria, one technical-standard
reference (Public Contracts Regulations 2015), notice-context CPV codes, and a single
service-class product-requirement seed from a Docling-extracted markdown payload
(`document_extractions.id` = `38ae3255-ab8e-49a1-b1cb-4f449ad74760`,
`raw_document_id` =
`raw-pc-169ba131-3b3f-ed11-8119-005056b64545-9455b434015366b0c5e2e685756afb835d2bb513d46753b7c7c0ef3f76ba5a7a`,
`source_content_hash` =
`64558467b047de578bc14f9290e72d6a88f9e5b74e142614f4bcf02872cd4a62`,
markdown sha256 =
`55d705de3593b81c53dc40dc23dc0e5ed27ba5fa66a0a3809cf9fb3f73cb255e`,
`section_count` = 3, `table_count` = 2, `page_count` = 0, daemon-side
`document_role` hint = `technical_specification`).

The source PDF is `CCP074-Appendix-B-Instructions-to-Bidders` for the Derbyshire County Council
"Creation of a Print Services DPS" procurement (CCP074, notice
`pc-169ba131-3b3f-ed11-8119-005056b64545`). It is a procedural document describing the
two-stage FTS Restricted Procedure used to admit suppliers to a Dynamic Purchasing System,
the procurement timetable, contact details, and the council's portal URL. **It is not the
Specification of Requirements**: the document explicitly names a separate "Appendix A —
Specification of Requirements (Lots)" which contains the per-lot detail, and that document is
not present in this PDF. The originating notice declares the DPS is split into 10 lots, none of
which are enumerated here.

Per the skill's golden assertion `lot_outputs_must_be_non_empty: true` and
`require_product_requirement_seeds: true`, output was emitted at the only grain available in
this document — a single DPS-umbrella lot record (`lot_number = "1"`) with a matching
DPS-umbrella service-class seed (`seed_key = "ccp074-print-services-dps-umbrella"`). Eight
warnings flag the source-document mismatch and the deferred-to-Appendix-A nature of all
substantive lot detail. `extraction_confidence_bps = 2500` reflects that this document does
not actually carry the lot/specification payload the skill is intended to extract.

## Steps Taken

1. Loaded job parameters and `SKILL.md` from execution context; confirmed contract execution
   mode, `target_domain = "lots"`, `extraction_scope = "full_document"`, and
   `policy_version = "2026.04.12"` (matching `document_extractions.extraction_policy_version`).
2. Located the Docling extraction record by `document_extraction_id`
   `38ae3255-ab8e-49a1-b1cb-4f449ad74760` in `state/state.db.document_extractions`.
   Verified identifying fields:
   - `raw_document_id` =
     `raw-pc-169ba131-3b3f-ed11-8119-005056b64545-9455b434015366b0c5e2e685756afb835d2bb513d46753b7c7c0ef3f76ba5a7a`
   - `process_id` = `proc-pc-169ba131-3b3f-ed11-8119-005056b64545`
   - `notice_id` = `pc-169ba131-3b3f-ed11-8119-005056b64545`
   - `document_set_id` = `tds-pc-169ba131-3b3f-ed11-8119-005056b64545`
   - `document_title` = `CCP074-Appendix-B-Instructions-to-Bidders`
   - `document_role` = `technical_specification`
   - `expected_overlays_json` = `["pdf-extraction/tender-specification"]`
   - `extraction_engine` = `docling`
   - `page_count = 0`, `section_count = 3`, `table_count = 2`, `image_count = 0`
3. Pulled the `extracted_markdown` payload (~22.5 KB, 13 lines after Docling collapsed the
   document body into a single bordered table cell at lines 11-13). Confirmed by inspection
   that the entire substantive document body is in the single table cell on line 11; lines
   3-9 carry the title block (`Appendix B`, `Instructions to Bidders`, `CCP074`,
   `Creation of a Print Services DPS`).
4. Pulled upstream context from `contract_notices` for notice
   `pc-169ba131-3b3f-ed11-8119-005056b64545`:
   - `title` = `CCP074 - Provision of a Print Service DPS - Acceptance Round 16`
   - `buyer_name` = `Derbyshire County Council`
   - `main_category` = `22000000-0 - Printed matter and related products | 79800000-2 - Printing and related services`
   - `summary` = "The Council are looking to procure a Print Service using a DPS, the DPS is split into 10 Lots …"
5. Confirmed that `notice_lots` is empty for this notice (no pre-existing lot rows),
   so the Appendix-A-derived 10-lot structure has not yet been ingested anywhere
   the daemon can read.
6. Loaded `result.schema.json` from
   `skills/pdf-extraction/tender-specification/result.schema.json` and
   `golden.assertions.yaml`, noting:
   - `min_lot_count = 1`
   - `lot_outputs_must_be_non_empty: true`
   - `require_evidence_anchors: true`
   - `require_product_requirement_seeds: true`
   - `standards_require_evidence_anchors: true`
   - `target_domain_must_equal: lots`
   - `warnings_must_be_array: true`
7. Surveyed the document body for any per-lot enumeration (lot titles, lot descriptions,
   per-lot requirements, per-lot CPV, per-lot evaluation criteria, per-lot product
   specifications, per-lot technical standards). Found: zero. Only generic references to
   "Details of Lots suppliers wish to be considered for" (Stage 1 SQ checklist line) and
   "the appropriate service category or lot" (Stage 2 award description). No per-lot detail.
8. Computed sha256 of `extracted_markdown` to populate
   `extraction_metadata.source_markdown_hash`:
   `55d705de3593b81c53dc40dc23dc0e5ed27ba5fa66a0a3809cf9fb3f73cb255e`.
9. Drafted a single DPS-umbrella lot record (`lot_number = "1"`) carrying:
   - 11 procedural / process requirements drawn verbatim-by-paraphrase from cited
     evidence anchors (DCC Universal Selection Questionnaire submission, ProActis portal
     enrolment, lot-of-interest declaration, Stage-1 evaluation dimensions, Stage-2 award
     criteria, the procurement timetable and clarification deadline, the cancellation
     reservation, and acceptance of DCC Terms and Conditions);
   - Two evaluation-criteria entries (Stage-1 pre-qualification and Stage-2
     further-competition award), both flagged as un-weighted in this document;
   - Two CPV codes inherited from notice context (`22000000-0`, `79800000-2`);
   - One product-requirement entry at DPS-umbrella grain;
   - One technical-standard reference (Public Contracts Regulations 2015) — the only
     normative reference cited in the body;
   - A primary lot evidence anchor on line 9 (document title block).
10. Drafted a matching DPS-umbrella `product_requirement_seeds[]` entry
    (`seed_key = "ccp074-print-services-dps-umbrella"`) keyed to the same `lot_number = "1"`
    with the same evidence anchor, marked explicitly as a placeholder pending enrichment
    from Appendix A.
11. Recorded eight warnings (see Warnings table below) covering: document is Instructions
    to Bidders not Specification; lot detail deferred to Appendix A; technical standards
    absent; evaluation criteria generic only; CPV inherited from notice context; product
    seeds at DPS-umbrella grain; Docling table-layout collapse; page_count zero.
12. Calibrated confidences: `extraction_confidence_bps = 2500` (low — source does not
    carry the specification payload). Field-level confidences range from 1500
    (`doc_role_hint_alignment` — the daemon classified this PDF as
    `technical_specification` but it is procedural Instructions-to-Bidders) to 8000
    (`evidence_anchors` — every emitted anchor cites verbatim text in the source markdown).
13. Wrote `result.json` to the working directory and validated it against
    `result.schema.json` with `jsonschema.validate` — returned `VALID`.
14. Authored this `run-manifest.md`.

## Decisions Made

- **Single DPS-umbrella lot, not 10 lots, not zero lots.** The notice context declares
  the DPS contains 10 lots, but this document does not enumerate any of them. Emitting
  10 lots would require fabrication; emitting zero lots would violate the
  `lot_outputs_must_be_non_empty` and `min_lot_count = 1` golden assertions. The chosen
  middle path matches the precedent set by prior tender-specification runs against
  scaffolding-only PDFs (e.g. session
  `eea72ac290d90410246ef77364c14369c256cdc27d57a38e46854ad1af44a349` against an ESPO
  Form-of-Contract template): emit one umbrella lot, flag the deferral with a high-severity
  warning (`LOT_DETAIL_DEFERRED_TO_APPENDIX_A`), and let downstream extraction split the
  umbrella once Appendix A is processed.
- **`document_role_hint` mismatch flagged but not corrected.** The daemon-side
  `document_role` is `technical_specification`; the document self-identifies on lines 3-5
  as `Appendix B — Instructions to Bidders`. The skill is not authorised to mutate the
  daemon-side classification, so the mismatch is recorded as a high-severity warning
  (`DOCUMENT_IS_INSTRUCTIONS_TO_BIDDERS_NOT_SPECIFICATION`) plus a low confidence on
  `doc_role_hint_alignment` (1500 bps), and `extraction_metadata.document_role` is
  preserved as the daemon-supplied value.
- **Use line-number / inline-heading anchors instead of page numbers.**
  `document_extractions.page_count = 0` and Docling reported only three sections
  (page header, page footer, textbox) on `extracted_sections_json`, so `page_ref` values
  use the form `line_<n>` referencing the markdown line, and `section_ref` values use
  the document's inline bold-prefixed headings (`Section 1 — Introduction`,
  `Period of Contract`, `Stage 1 ...`, `Stage 2`, `Section 2 — Procurement Timetable`,
  `Contact and Queries`). Documented in the `DOCUMENT_PAGE_COUNT_ZERO` and
  `DOCLING_TABLE_LAYOUT_COLLAPSED_INTO_SINGLE_CELL` warnings.
- **Evaluation criteria emitted as a non-empty array but explicitly un-weighted.**
  The document gives a qualitative four-dimension Stage-1 admission gate (Economic and
  Financial Standing, Technical/Professional Ability and capacity, Legal Status, Quality)
  and a generic Stage-2 award rubric ("price, quality and delivery lead times" with the
  caveat "including, but not be limited to"). Both are emitted, both carry a `weighting_text`
  noting the absence of disclosed weightings, and an `EVALUATION_CRITERIA_GENERIC_ONLY`
  warning flags the lack of per-lot criteria.
- **Single technical-standard reference.** Only one normative reference is literally
  cited in the body: "the Public Contracts Regulations 2015". Print-process / paper /
  colour / data-handling standards (ISO/IEC 19752, ISO 9706, ISO 12647, FSC/PEFC,
  GDPR-for-print, etc.) would be expected in Appendix A but are absent here. Emitting
  fabricated standards would breach `standards_require_evidence_anchors`. The
  `TECHNICAL_STANDARDS_NOT_PRESENT` warning records the absence.
- **CPV codes inherited from upstream notice context, not extracted from body.** Both CPV
  codes (`22000000-0 Printed matter and related products`,
  `79800000-2 Printing and related services`) come from `contract_notices.main_category`
  for the originating notice. The body subject-matter at line 9
  ("Creation of a Print Services DPS") is consistent with both. Documented in the
  `CPV_CODES_FROM_NOTICE_CONTEXT` warning.
- **Confidence calibration.** `extraction_confidence_bps = 2500` reflects that the source
  document does not actually carry the lot / specification payload the skill is intended
  to extract. Field-level confidences are deliberately bimodal: high (6500-8000) for items
  the document literally evidences (lot title, document inventory, evidence anchors, lot
  description, lot requirements drawn from the Stage-1/Stage-2 prose, the procurement
  timetable); low (1500-3000) for items the document does not actually contain
  (`product_requirements`, `product_requirement_seeds`, `lot_count`, `evaluation_criteria`,
  `doc_role_hint_alignment`).
- **`outcome_class = success_with_warnings`.** Schema validation passed and all golden
  assertions are met (`lot_count >= 1`, evidence anchors emitted, product-requirement
  seeds emitted, technical-standard reference carries an evidence anchor, `target_domain`
  is `"lots"`, warnings is an array). Eight warnings document the scope limitations of
  this particular source PDF.

## Warnings

| Code                                                | Severity | Anchor (line / section)                                                              |
|-----------------------------------------------------|----------|--------------------------------------------------------------------------------------|
| DOCUMENT_IS_INSTRUCTIONS_TO_BIDDERS_NOT_SPECIFICATION | high   | line_3, line_5, line_11 (document inventory naming Appendix A vs Appendix B)         |
| LOT_DETAIL_DEFERRED_TO_APPENDIX_A                   | high     | process_context.notice.summary, line_11 (DCC SQ checklist; Stage-2 award sentence)   |
| TECHNICAL_STANDARDS_NOT_PRESENT                     | high     | line_11 (only the Public Contracts Regulations 2015 is cited in body)                |
| EVALUATION_CRITERIA_GENERIC_ONLY                    | high     | line_11 (Stage-1 four-dimension list; Stage-2 "include, but not be limited to" prose) |
| CPV_CODES_FROM_NOTICE_CONTEXT                       | medium   | process_context.cpv; line_9 (document title block)                                   |
| PRODUCT_SEEDS_AT_DPS_UMBRELLA_GRAIN                 | medium   | line_9, line_11 (Appendix A inventory entry)                                         |
| DOCLING_TABLE_LAYOUT_COLLAPSED_INTO_SINGLE_CELL     | low      | document_extractions.section_count = 3; table_count = 2; markdown lines 11-13        |
| DOCUMENT_PAGE_COUNT_ZERO                            | low      | document_extractions.page_count = 0                                                  |

Each warning is also present in `result.warnings[]` with structured `evidence_refs`.

## Evidence

All major claims in this manifest and `result.json` are anchored either to specific markdown
line numbers in the Docling-extracted markdown payload (sha256
`55d705de3593b81c53dc40dc23dc0e5ed27ba5fa66a0a3809cf9fb3f73cb255e`), to verbatim
`document_extractions` row fields, or to verbatim `contract_notices` row fields for the
originating notice. Key anchors:

- `line_3`: "Appendix B" — confirms the document is Appendix B, not the
  Specification of Requirements (which the document inventory names as Appendix A).
- `line_5`: "Instructions to Bidders" — confirms the document type is procedural
  guidance, not a technical specification.
- `line_7`: "CCP074" — links the document to the Derbyshire County Council
  CCP074 procurement reference.
- `line_9`: "Creation of a Print Services DPS" — drives the lot title and the
  DPS-umbrella product-requirement seed.
- `line_11` (document inventory paragraph): "Appendix A - Specification of
  Requirements (Lots)" and "Appendix B - Instruction to Bidders including DPS conditions
  for guidance" — primary evidence that the actual lot specification is held in a
  separate document (Appendix A) which is not part of this PDF.
- `line_11` (Section 1 Introduction): "Under the Public Contracts Regulations 2015,
  the following documents are being made available by Derbyshire County Council …" —
  anchors the only technical-standard reference emitted.
- `line_11` (Period of Contract): "It is envisaged that this DPS will be available for
  an initial period of 60 months commencing 1 st December 2022 – 30 th November 2027" —
  anchors the contract-term description in the lot description.
- `line_11` (DCC Universal Selection Questionnaire checklist): "1. Quality
  Questionnaire (QQ) 2. Supplier Information Security (SISQ) 3. Quality Questionnaire (QQ)
  4. Details of Lots suppliers wish to be considered for" — anchors the lot-of-interest
  requirement and confirms the document's awareness of multiple lots without enumerating
  them.
- `line_11` (Stage 1 Pre-qualification): "Economic and Financial Standing -
  Technical/Professional Ability and capacity - Legal Status - Quality" — anchors the
  Stage-1 evaluation_criteria entry.
- `line_11` (Stage 2): "based on an evaluation of the offers submitted, which would
  include, but not be limited to, price, quality and delivery lead times" — anchors the
  Stage-2 evaluation_criteria entry and the EVALUATION_CRITERIA_GENERIC_ONLY warning.
- `line_11` (How does a DPS work?): "the URL for which is
  https://www.eastmidstenders.org/" — anchors the portal URL in the lot description.
- `line_11` (clarification window): "Please raise any 1 st round clarification questions
  by 16:00 hours on the 21 st October 2022" — anchors the Round-1 clarification deadline.
- `line_11` (Section 2 Procurement Timetable): the date list ("Contract notice …
  30/09/2022", "Clarification request deadline (1st round only) 21/10/2022", "Deadline for
  submissions (1st round only) 01/11/2022", "Anticipated contracts start date first round
  only 01/12/2022", "Round 2 to commence 01/12/2022") — anchors the timetable
  requirement in the lot record.
- `line_11` (Contact and Queries): "Procurement officer: Julia Apperley
  Email: julia.apperley@derbyshire.gov.uk Tel: 01629 536854" — anchors the procurement
  contact (recorded as a low-priority contextual evidence anchor only).

Process / notice context anchors (not in the document body, used to populate CPV and to
inform the LOT_DETAIL_DEFERRED_TO_APPENDIX_A warning):

- `contract_notices.main_category` for notice
  `pc-169ba131-3b3f-ed11-8119-005056b64545`:
  `22000000-0 - Printed matter and related products | 79800000-2 - Printing and related services`.
- `contract_notices.summary` for the same notice: "The Council are looking to procure a
  Print Service using a DPS, the DPS is split into 10 Lots being available." — primary
  evidence that the DPS contains 10 lots even though this PDF does not enumerate them.
- `contract_notices.buyer_name` = `Derbyshire County Council`.

Result-contract-level evidence:

- `result.extraction_metadata.source_markdown_hash` =
  `55d705de3593b81c53dc40dc23dc0e5ed27ba5fa66a0a3809cf9fb3f73cb255e` (sha256 of
  `extracted_markdown` for `document_extractions.id =
  38ae3255-ab8e-49a1-b1cb-4f449ad74760`, computed via `shasum -a 256`).
- `result.extraction_metadata.target_domain = "lots"` — matches job parameter
  `target_domain` and skill SKILL.md frontmatter.
- `result.extraction_metadata.extraction_scope = "full_document"` — matches job parameter
  `extraction_scope`.
- `result.extraction_metadata.policy_version = "2026.04.12"` — matches
  `document_extractions.extraction_policy_version`.
- `result.extraction_metadata.document_role = "technical_specification"` — preserved from
  the daemon-supplied `document_extractions.document_role` (mismatch with the document's
  self-described role flagged in the DOCUMENT_IS_INSTRUCTIONS_TO_BIDDERS_NOT_SPECIFICATION
  warning rather than silently corrected).
- `result.lots[0].lot_number = "1"` — synthetic DPS-umbrella record emitted to satisfy the
  `lot_outputs_must_be_non_empty` and `min_lot_count >= 1` golden assertions; `notice_lots`
  for this notice is currently empty so there is no pre-existing lot row to reconcile
  against.
- `result.product_requirement_seeds[0].lot_number = "1"` — keyed to the same umbrella lot.
- Schema validation: `python3 -c "jsonschema.validate(result, schema)"` against
  `skills/pdf-extraction/tender-specification/result.schema.json` returned `VALID`.

Daemon-observed appendix fields (e.g. wall-clock duration, exit code, stdout/stderr capture,
result.json size on disk, run-manifest.md size on disk, validator verdict if any) are to be
appended by the daemon after this manifest is finalised; they are intentionally not asserted
by the agent.
