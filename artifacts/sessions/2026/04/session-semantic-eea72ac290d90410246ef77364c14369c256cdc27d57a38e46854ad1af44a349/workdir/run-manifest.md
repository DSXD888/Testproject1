---
manifest_version: 1
skill_id: pdf-extraction/tender-specification
skill_version: 1.0.0
job_id: job-d16d1b03-be65-4cf7-b4c4-19292b6d61d6
session_id: session-semantic-eea72ac290d90410246ef77364c14369c256cdc27d57a38e46854ad1af44a349
execution_mode: contract
started_at: 2026-04-17T20:03:00Z
completed_at: 2026-04-17T20:08:48Z
outcome_class: success_with_warnings
items_processed: 1
items_skipped: 0
items_failed: 0
warnings_count: 7
agent_claimed_pages_visited: 0
agent_claimed_source_url: null
agent_claimed_fingerprint_state: not_applicable
agent_claimed_locale_state: en-GB
agent_claimed_currency_state: GBP
---

# Run Manifest — pdf-extraction/tender-specification

## Summary

Extracted lot structure, contractual requirements, technical-standard references, CPV codes,
evidence anchors and two service-class product-requirement seeds from a single Docling-extracted
markdown document (document_extractions.id =
`3c622c6e-81d7-45bc-8a61-e56cca547976`, source_content_hash
`083c26c4f86d913df3c83f3f942831b6ce5e59923b26ffcd08b39165b69116b5`,
markdown sha256 `3469b5fd17c06fdfc47261b0f1dcd55e955714ee6ed6655d0623cc3caf4cab6e`,
section_count = 677, page_count = 0, document_role `technical_specification`).

The source PDF is "FRAMEWORK SCHEDULE 3 — FORM OF CONTRACT AND CALL-OFF TERMS" for ESPO Framework
107_26 ("Inspection & Maintenance of Indoor Gymnasium, Fitness & Sport Equipment and Outdoor
Schools Play & Sport Equipment"). It is a contractual template with placeholder fields, not a
substantive technical specification. Output was emitted at single-lot grain matching the upstream
notice context, with detailed equipment-level standards and product specifications explicitly
deferred to the Framework Agreement Schedule 1 / clause 13 and to the ESPO Invitation to Tender
Section 2 (Requirement). One lot, two service-class seeds, eight technical-standard refs (legal
frameworks plus a forward-reference to Framework Agreement clause 13), zero evaluation criteria
(absent), and 15 evidence anchors were written. `extraction_confidence_bps` = 5500 reflecting
the template nature of the source.

## Steps Taken

1. Loaded job parameters and SKILL.md from execution context; confirmed contract execution mode,
   target_domain "lots", extraction_scope "full_document", policy_version "2026.04.12".
2. Located the Docling extraction record by `document_extraction_id`
   `3c622c6e-81d7-45bc-8a61-e56cca547976` in `state/state.db.document_extractions`. Pulled
   `extracted_markdown` (~422 KB, 1632 lines) and confirmed identifying fields
   (`raw_document_id`, `process_id`, `notice_id`, `extraction_policy_version`).
3. Pulled upstream context from `contract_notices` (notice id
   `pc-3ba4e6b8-9cef-f011-813c-005056b64545`, title "107_26 Inspection and Maintenance of Indoor
   Gymnasium, Fitness and Sport Equipment, and Outdoor Schools Play and Sport Equipment", buyer
   "ESPO", main_category text containing CPV `37400000-2 - Sports goods and equipment` and
   `50000000-5 - Repair and maintenance services`) and from `notice_lots` (single existing lot,
   `lot_number = 1`, marked as a "single non-lotted arrangement").
4. Loaded `result.schema.json` from
   `skills/pdf-extraction/tender-specification/result.schema.json` and golden assertions
   (`min_lot_count = 1`, `require_evidence_anchors = true`,
   `require_product_requirement_seeds = true`,
   `standards_require_evidence_anchors = true`).
5. Surveyed markdown structure via the document's hierarchical headings (sections 1-46 covering
   Form of Contract, Master Contract Schedule, Definitions, Quality, Service Levels and Service
   Credits, Health and Safety, Environmental Requirements, Bribery, Schedule 1, Schedule 2).
   Confirmed all "specification-shaped" rows in the Master Contract Schedule are placeholder
   `[ ]` fields with Customer Drafting Notes — no substantive lot, product or evaluation detail
   embedded.
6. Re-read clauses 6.3 (Quality), 8.1 (BCDR), 9 (Monitoring), 11 (Service Levels and Remedies),
   §25 (Health & Safety), §26 (Environmental / GHG), §27 (Bribery), §16/§18.56 (Data Protection /
   PA23 Transparency), Schedule 1 (Service Levels), Schedule 2 (Implementation Plan) for
   substantive Supplier obligations.
7. Computed sha256 of the markdown payload to populate
   `extraction_metadata.source_markdown_hash`.
8. Drafted one lot record consistent with the single-lot framework arrangement, populated
   16 contractual requirements drawn verbatim-by-paraphrase from cited clauses, two service-class
   product-requirement seeds (indoor gym/fitness/sport; outdoor schools play/sport), eight
   technical-standard references (Health and Safety at Work Act 1974, Sale of Goods Act 1979 ss.
   12-14, Supply of Goods and Services Act 1982 s.2, Bribery Act 2010, UK GDPR / DPA 2018, PA23,
   Government GHG conversion factors, Framework Agreement clause 13), and 15 lot/document-level
   evidence anchors keyed by markdown line number and section ref (page_count was 0).
9. Recorded seven warnings covering: document is a call-off template; evaluation criteria absent;
   technical-standards placeholders; CPV inherited from notice context; single-lot inferred;
   product seeds at service-class grain only; page_count zero so anchors use line numbers.
10. Wrote `result.json` to the working directory and validated it against
    `result.schema.json` using `jsonschema.validate` (returned `VALID`).
11. Authored this `run-manifest.md`.

## Decisions Made

- **Single lot, not multi-lot.** The Form of Contract template references "the Lot or Lots"
  generically (line 89), the recitals describe one combined subject-matter (line 13), and the
  pre-existing `notice_lots` row marks the framework as a "single non-lotted arrangement". No
  contradicting structural signal found in the document; emitted lot_number = "1".
- **Use line-number / clause-number anchors instead of page numbers.** `document_extractions
  .page_count = 0`, so `page_ref` values follow the form `line_<n>` (referencing the markdown
  line) and `section_ref` values follow the document's hierarchical clause numbering (e.g.
  `§6.3.1 Quality`, `§25.4 Health and Safety`, `§45 Schedule 1`). Documented in the
  DOCUMENT_PAGE_COUNT_ZERO warning.
- **Evaluation criteria emitted as empty array.** Source contains an unpopulated Service Levels
  table (rows 1-4 blank, line 1567-1574) and no MEAT criteria block. Award/evaluation criteria
  for the framework / further-competitive selection live in the ESPO ITT, not this template.
  Emitted `evaluation_criteria: []` plus an explicit EVALUATION_CRITERIA_NOT_PRESENT warning
  rather than fabricating criteria.
- **Technical standards split into legal frameworks vs. equipment standards.** Concrete
  equipment-level standards (e.g. BS EN 1176 for play equipment, BS EN 957 for fitness, BS EN
  1177 for impact-attenuating surfaces, RPII registration) are NOT present in this document
  and were deliberately omitted to avoid hallucination. Eight references were emitted: a forward
  reference to Framework Agreement clause 13, plus seven legal-framework references that are
  literally cited in the body. Documented in the TECHNICAL_STANDARDS_PLACEHOLDERS warning.
- **Product-requirement seeds at service-class grain.** Two seeds emitted (indoor gym/fitness/
  sport inspection-and-maintenance; outdoor schools play/sport inspection-and-maintenance) keyed
  off the document's title-block subject. Concrete product candidates require enrichment from
  Framework Agreement Schedule 1 / ITT Section 2 and are explicitly out-of-scope of this
  extraction. Documented in the PRODUCT_SEEDS_ARE_SERVICE_CLASS warning.
- **CPV codes inherited from upstream notice context, not extracted from body.** Both CPV codes
  (`37400000-2`, `50000000-5`) come from `contract_notices.main_category` for the originating
  notice. The body subject-matter at line 13 is consistent with both. Documented in the
  CPV_CODES_FROM_NOTICE_CONTEXT warning.
- **Confidence calibration.** `extraction_confidence_bps = 5500` reflects the template nature
  of the source. Field-level confidences range from 1000 (evaluation_criteria, absent) to 8500
  (lot_count, lot_title — directly evidenced). Anything tied to the placeholder rows scored
  below 6000.
- **`outcome_class = success_with_warnings`.** Schema validation passed and required outputs
  were emitted, but seven warnings flag scope limitations of this source document.

## Warnings

| Code                                            | Severity | Anchor (line / section)                                    |
|-------------------------------------------------|----------|------------------------------------------------------------|
| DOCUMENT_IS_CALLOFF_TEMPLATE_NOT_TENDER_SPEC    | high     | line_1, line_89, line_91                                   |
| EVALUATION_CRITERIA_NOT_PRESENT                 | high     | line_1567-1574, line_1575                                  |
| TECHNICAL_STANDARDS_PLACEHOLDERS                | high     | line_91, line_336                                          |
| CPV_CODES_FROM_NOTICE_CONTEXT                   | medium   | process_context, line_13                                   |
| LOT_STRUCTURE_INFERRED_SINGLE_LOT               | medium   | line_13, line_89                                           |
| PRODUCT_SEEDS_ARE_SERVICE_CLASS_NOT_PRODUCT_LEVEL | medium | line_13, line_89                                           |
| DOCUMENT_PAGE_COUNT_ZERO                        | low      | document_extractions.page_count                            |

Each warning is also present in `result.warnings[]` with structured `evidence_refs`.

## Evidence

All major claims in this manifest and `result.json` are anchored to specific markdown line numbers
in the Docling-extracted markdown payload (sha256
`3469b5fd17c06fdfc47261b0f1dcd55e955714ee6ed6655d0623cc3caf4cab6e`). Key anchors:

- `line_1`: "FRAMEWORK SCHEDULE 3 — FORM OF CONTRACT AND CALL-OFF TERMS" — confirms the document
  is the Call-Off template, not an ITT or Specification.
- `line_13`: combined subject-matter "Inspection & Maintenance of Indoor Gymnasium, Fitness &
  Sport Equipment and Outdoor Schools Play & Sport Equipment" — drives the lot title and both
  product-requirement seeds.
- `line_17`: "CONTRACT REF 107_26" — links the document to the ESPO framework reference seen on
  the originating notice (process_id `proc-pc-3ba4e6b8-9cef-f011-813c-005056b64545`).
- `line_33`: WHEREAS recital citing "Tender Notice 2026/S 000-002119 on 12 January 2026 ...
  pursuant to section 95(4) of the PA23" — anchors PA23 standard reference and notice provenance.
- `line_89`: Master Contract Schedule §2 — "Details of the Goods listed above can be found in
  schedule 1 of the Framework Agreement" — primary evidence that Goods/Services specifications are
  externalised, not contained here.
- `line_91`: Master Contract Schedule §2 Standards row — "Quality Standards [ ] Technical
  Standards [ ] ... Details of technical standards can be found in clause 13 of the Framework
  Agreement" — primary evidence that the specification is a placeholder; anchors the Framework
  Agreement clause 13 standard reference.
- `line_99`: Master Contract Schedule §4 — Critical Service Failure thresholds (24 consecutive
  hours / 48 hours in any 12 months) — anchors service-level requirements.
- `line_336`: §6.3.1 Quality — "The Supplier shall at all times comply with the Technical
  Standards and the Quality Standards" — anchors the binding-but-blank standards obligation.
- `line_348`: §6.3.4 Quality — Sale of Goods Act 1979 sections 12-14 and Supply of Goods and
  Services Act 1982 section 2 statutory obligations.
- `line_386`: §8.1 BCDR — business-continuity arrangements requirement.
- `line_408`: §11.1 Service Levels and Remedies — Service-Level / Service-Credit / Critical
  Service Failure structure.
- `line_1241`: §25.4 Health & Safety — Health and Safety at Work etc. Act 1974.
- `line_1271`: §26.5.1 Environmental — annual GHG report using government conversion factors.
- `line_1283`: §27.1.2 Anti-Bribery — Prohibited Act under Bribery Act 2010.
- `line_1539`: §45 Schedule 1 — Service Levels and Service Credits scope.
- `line_1620`: §46 Schedule 2 — Implementation Plan and Milestones.

Result-contract-level evidence:

- `result.extraction_metadata.source_markdown_hash` =
  `3469b5fd17c06fdfc47261b0f1dcd55e955714ee6ed6655d0623cc3caf4cab6e` (sha256 of
  `extracted_markdown`).
- `result.extraction_metadata.target_domain = "lots"`,
  `result.extraction_metadata.extraction_scope = "full_document"`, matching job parameters.
- `result.extraction_metadata.policy_version = "2026.04.12"` — matches
  `document_extractions.extraction_policy_version`.
- `result.lots[].lot_number = "1"` — matches existing `notice_lots.lot_number = 1` for notice
  `pc-3ba4e6b8-9cef-f011-813c-005056b64545`; no contradicting structural evidence in the source.
- Schema validation: `jsonschema.validate(result, result.schema.json)` returned `VALID`.

Daemon-observed appendix fields (e.g. wall-clock duration, exit code, stdout/stderr capture) are
to be appended by the daemon after this manifest is emitted; they are intentionally not asserted
by the agent.
