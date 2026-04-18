---
manifest_version: 1
skill_id: pdf-extraction/tender-specification
skill_version: 1.0.0
job_id: job-13e7b464-3f98-4f0a-a048-daac1d0b232c
session_id: session-semantic-4bc75643063977ec5efd69ca28e3bd884b07b35e90b88436b493b9630c9431f6
execution_mode: contract
started_at: 2026-04-17T23:06:00Z
completed_at: 2026-04-17T23:08:37Z
outcome_class: partial_success
items_processed: 1
items_skipped: 0
items_failed: 0
warnings_count: 6
agent_claimed_pages_visited: 0
agent_claimed_source_url: "document_extraction:f2f30ba2-41a1-4fca-8ccf-89daad471c84"
agent_claimed_fingerprint_state: not_applicable
agent_claimed_locale_state: en-GB
agent_claimed_currency_state: not_applicable
---

# Run Manifest — pdf-extraction/tender-specification

## Summary

Processed Docling-extracted markdown for extraction `f2f30ba2-41a1-4fca-8ccf-89daad471c84`
(raw document `raw-pc-978a1774-1037-f111-813c-005056b64545-73c0d4374cc1416efa9c017a701d068122c277320d0082b2cd7f96e2af9b158e`,
10 pages, 41 sections, 1 table, markdown length 47 928 chars, source hash
`ea7eb8eb145bb958f7b6c82647aee134546fe712c314d76ce51b46cb3cc3f189`). The document is titled
"Volume-I-Instructions-Conditions-Jan2026" and its actual content is the ECMWF Copernicus ITT
"Volume I — Instructions for respondents and Conditions of tender" (dated 30 January 2026, ECMWF
Administration Department, Procurement Section). The extraction's `document_role` in the
daemon state store is `technical_specification`, but the content is tender instructions/conditions.
Emitted a single low-confidence lot entry (Lot 2) inferred from the parent notice title
("C3S2_413_bis Lot 2 Maintenance and Extension of C3S Windstorm and Storm Surge Services",
notice `pc-978a1774-1037-f111-813c-005056b64545`), with requirements and evaluation-framework
items drawn from sections 3.2 – 3.16, 5.x, and 6 of Volume I. No CPV codes, technical standards,
or product-requirement seeds were emitted: none exist in this document; they are explicitly
deferred to Volume II Specification (section 4.5). Overall extraction confidence 3 500 bps.
`outcome_class = partial_success` because the contract schema is fully satisfied and evidence is
sound, but the semantically useful fields (product requirements, CPV, standards) are empty due
to a document-role mismatch surfaced as a warning rather than a hard failure.

## Steps Taken

1. Located the Docling-produced markdown via `state.db::document_extractions.id =
   f2f30ba2-41a1-4fca-8ccf-89daad471c84` — 47 928 chars, 363 lines, 10 pages, 41 sections,
   1 table (the front-matter Table of Contents), extraction engine `docling`, policy version
   `2026.04.12`, expected overlays `["pdf-extraction/tender-specification"]`.
2. Read the full markdown and confirmed it is ECMWF's Volume I ITT instructions document, not
   a technical specification. Confirmed by the cover (line 15 "VOLUME I: Instructions for
   respondents and Conditions of tender"), the section list (§§1–7), and the explicit redirect
   at §4.5 ("Volume II Specification. This contains detailed technical specifications and
   requirements for each ITT.").
3. Queried `contract_notices` for the parent notice `pc-978a1774-1037-f111-813c-005056b64545`
   and retrieved title "C3S2_413_bis Lot 2 Maintenance and Extension of C3S Windstorm and
   Storm Surge Services", buyer "ECMWF", main category CPV 73000000-2. Used this only for lot
   identity (`lot_number = "2"`), per the skill's non-goal of "Guessing lot boundaries,
   standards, or product requirements without citing evidence anchors."
4. Verified `notice_lots` table contained no pre-existing rows for this notice, so there was
   no authoritative lot row to cross-reference or contradict.
5. Extracted procurement conditions from §§3.2 – 3.16 (eligibility, EDES, restrictive measures,
   conditionality, code of conduct, validity, language, right to reject, diversity tiebreaker),
   §§5.1 – 5.2 (portal submission mechanics), §6 (clarifications deadline), anchoring each to
   its section heading and page range implied by the Table of Contents on p.1.
6. Derived a four-item evaluation-criteria list (§3.3 scope, §3.4 – 3.6 eligibility screening,
   §3.14 substantial-compliance reject, §3.16 diversity tiebreaker). Recorded that numeric
   weightings are absent.
7. Confirmed no CPV codes, no ISO/EN/BS references, and no product attributes appear anywhere
   in the markdown. Did NOT import the notice-level CPV (73000000-2) onto the lot to avoid
   fabricating anchored evidence.
8. Emitted `product_requirement_seeds = []` and empty `technical_standards` / `cpv_codes`
   arrays with matching `field_confidences = 0` and explanatory warnings.
9. Computed `source_markdown_hash = ea7eb8eb145bb958f7b6c82647aee134546fe712c314d76ce51b46cb3cc3f189`
   via `shasum -a 256` over the exact `extracted_markdown` field and set `extraction_scope =
   "full_document"`, `target_domain = "lots"`, `policy_version = "2026.04.12"`.
10. Validated `result.json` against
    `skills/pdf-extraction/tender-specification/result.schema.json` with jsonschema — passes.

## Decisions Made

- Emit exactly one lot (`lot_number = "2"`) rather than zero lots. Golden-assertion
  `lot_outputs_must_be_non_empty: true` would otherwise fail the skill, and the parent notice
  title unambiguously identifies this ITT as covering Lot 2 of C3S2_413_bis. Empty `lots` was
  therefore not an allowed outcome; low-confidence inferred lot with a `LOT_INFERRED_FROM_CONTEXT`
  warning is the correct failure-mode.
- Do NOT copy the notice-level CPV 73000000-2 onto the lot. The skill explicitly forbids
  "Guessing lot boundaries, standards, or product requirements without citing evidence
  anchors." Notice-level category is not evidence drawn from the tender document itself.
- Set `outcome_class = partial_success` (not `success`, not `failed`). The contract is valid
  and populated, evidence is sound, but the semantically load-bearing fields (product
  requirements, standards, CPV, evaluation weightings) are empty because the document simply
  does not contain them — a classification/role mismatch, not an extraction failure.
- Set `extraction_confidence_bps = 3500`. This reflects high confidence in the procurement-
  conditions extraction but near-zero recovery on the product/standards/CPV fields the
  downstream pipeline actually needs. A higher number would overstate the usefulness of the
  extraction to the matching pipeline.
- Preserve `document_role = "technical_specification"` in `extraction_metadata` verbatim
  from the daemon-side row rather than silently correcting it. The `DOCUMENT_ROLE_MISMATCH`
  warning documents the divergence so downstream operators / validators can decide whether to
  re-classify or re-route the document (e.g., to an `instructions-to-tender` overlay).
- Kept locale `en-GB` based on §3.11 ("All Tender documentation… shall be written in English")
  and the document-language column being empty — safe default, no other language cues.

## Warnings

Six warnings were attached to `result.json`, corresponding to the structural gaps in the
source document:

1. `DOCUMENT_ROLE_MISMATCH` — daemon-stored role is `technical_specification`; actual content
   is a Volume I instructions/conditions document.
2. `LOT_INFERRED_FROM_CONTEXT` — Lot 2 pulled from the notice title because Volume I does not
   itself enumerate lot technical scope.
3. `MISSING_PRODUCT_REQUIREMENTS` — no product specifications, quantities, technical
   attributes, or standards references appear in Volume I; Volume II (§4.5) is not present.
4. `MISSING_CPV_CODES` — no CPV codes appear in the document text; notice-level CPV
   deliberately not imported onto the lot.
5. `MISSING_TECHNICAL_STANDARDS` — no ISO / EN / BS / ASTM references appear in Volume I.
6. `EVALUATION_WEIGHTINGS_NOT_STATED` — §3.3, §3.14, §3.16 describe the evaluation framework
   in prose but publish no numeric weightings.

## Evidence

All claims above are anchored to observable evidence in the Docling-produced markdown:

- Cover / role statement: line 15 — "VOLUME I: Instructions for respondents and Conditions of
  tender"; line 17 — "ISSUED BY: ECMWF Administration Department Procurement Section".
- Volume II redirect: §4.5 (line 293) — "This contains detailed technical specifications and
  requirements for each ITT."
- Evaluation framework: §3.3 (line 165) — "ECMWF will evaluate the Tender based on the
  Tenderer's responses to ECMWF's requirements (Volume II of this ITT) and the questions in
  the eProcurement Portal as well as Tenderer's compliance with paragraphs 3, 4, 5, 6 and 7 of
  this Volume I."
- Substantial-compliance reject right: §3.14 (line 259) — "ECMWF reserves the right to reject
  a Tender that does not substantially comply with the conditions that are part of the ITT."
- Diversity tiebreaker: §3.16 (line 271) — "In the event that multiple Tenderers present
  equally qualified Tenders (discrepancy lower than 1%), ECMWF will take into consideration
  the diversity and gender balance of each Tenderer's organisation as a tiebreaker when
  making the final decision."
- EDES eligibility check: §3.4 (line 186) — "must accept that the ECMWF will reject any
  Tenderer who is so listed."
- Restrictive measures form: §3.5 (line 205) — "Tenderers must complete the Restrictive
  Measures form in the eProcurement Portal for a minimum of four persons".
- Validity: §3.8 (line 235) — "the Tender shall remain valid for nine months after the closing
  date for this ITT."
- Language: §3.11 (line 247) — "All Tender documentation, manuals and diagram labelling shall
  be written in English."
- UK eligibility: §3.2.2 (line 159) — "from 1 January 2024 the UK is therefore eligible to
  benefit from Copernicus services and products in the same way as other participating
  countries."
- Portal submission: §5.1 (line 325), §5.2 (line 337) — "submissions in other formats… will
  not be accepted" / "The questionnaire asks for details about your organisation and your
  Tender. It must be answered online."
- Clarification deadline: §6 (line 349) — "must be received by ECMWF more than fifteen (15)
  days before the closing date."
- Lot identity: notice `pc-978a1774-1037-f111-813c-005056b64545` title = "C3S2_413_bis Lot 2
  Maintenance and Extension of C3S Windstorm and Storm Surge Services" (from
  `contract_notices` row; referenced by the extraction row's `notice_id`).
- Absence evidence (grep of markdown): no CPV-pattern string (8-digit codes), no `ISO `,
  `EN `, `BS EN`, `ASTM` tokens, no explicit quantity or technical-attribute phrasing
  detected. Evidence of absence is documented alongside the corresponding `MISSING_*`
  warnings so the validator overlay can re-check.
