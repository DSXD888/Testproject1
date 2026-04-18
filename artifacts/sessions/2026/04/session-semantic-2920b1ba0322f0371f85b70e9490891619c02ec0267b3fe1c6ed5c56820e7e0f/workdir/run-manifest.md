---
manifest_version: "1.0"
skill_id: pdf-extraction/tender-specification
skill_version: 1.0.0
job_id: job-0dea65c7-41f5-41ba-8015-a8b7eb217900
session_id: session-semantic-2920b1ba0322f0371f85b70e9490891619c02ec0267b3fe1c6ed5c56820e7e0f
execution_mode: contract
started_at: 2026-04-18T01:51:14Z
completed_at: 2026-04-18T01:51:14Z
outcome_class: succeeded_with_warnings
items_processed: 1
items_skipped: 0
items_failed: 0
warnings_count: 8
agent_claimed_pages_visited: 0
agent_claimed_source_url: ""
agent_claimed_fingerprint_state: not_applicable
agent_claimed_locale_state: en-GB
agent_claimed_currency_state: GBP
---

## Summary

Contract-mode run of `pdf-extraction/tender-specification` (v1.0.0) against document extraction
`2fcf938f-fa4e-45da-aa92-1f9ab97c0478` (`raw-pc-dc1daee2-...-2de24c9a...`), a 57-page / 86-section
Docling extraction titled `6278 - Draft T&Cs - Lot 1` for process
`proc-pc-dc1daee2-772e-f111-813c-005056b64545` (London Borough of Bexley, Building Services
Framework 6278). The document is a draft Form of Contract plus General Terms & Conditions template;
Schedules 1–4 (Specification, Pricing, ITT, Contractor's Tender) appear only as heading placeholders
with no body content. No CPV codes, evaluation criteria, KPIs, technical standards, or product
requirements are present in the extracted markdown. The skill emitted a single Lot 1 record
anchored to the page 1 cover preamble ("RELATING TO THE PROVISION OF FIRE RISK ASSESSMENT
SERVICES"), an empty `product_requirement_seeds` array, and eight warnings capturing the empty
schedules, a template placeholder mismatch ("security services" vs "fire risk assessment
services"), absent CPV, absent standards, and a suggestion to review the document role
classification. Overall extraction confidence 2000 bps.

## Steps Taken

1. Read bound SKILL.md and result.schema.json to fix the output contract.
2. Loaded the bound inputs from `document_extractions.id = 2fcf938f-fa4e-45da-aa92-1f9ab97c0478`
   (143KB markdown, 57 pages, 86 sections, 0 tables, `processing_strategy=standard`,
   `extraction_engine=docling@2.88.0`, `extraction_policy_version=2026.04.12`,
   `expected_overlays_json=["pdf-extraction/tender-specification"]`,
   `document_role=technical_specification`).
3. Computed SHA-256 of the exact markdown payload:
   `fbc2726ee7097df3e2a36ce2c3ac63d624704484f97086fe584db208c4a8ab92`.
4. Cross-referenced process and lot context from local state:
   - `procurement_processes.id = proc-pc-dc1daee2-772e-f111-813c-005056b64545`
     (`buyer_name = "London Borough of Bexley"`,
      `title = "6278 - Building Services Framework - Lots 1-11"`).
   - `notice_lots.id = pc-dc1daee2-772e-f111-813c-005056b64545-lot-1`
     (`lot_number = 1`, `title = "Provision of Fire Risk Assessments"`).
5. Scanned the extracted sections (`extracted_sections_json`) for SCHEDULE/SPECIFICATION
   headings; located `SCHEDULE 1 SPECIFICATION` (page 54, `#/texts/795`), `SCHEDULE 2 PRICING
   SCHEDULE` (page 55, `#/texts/797`), `SCHEDULE 3 THE COUNCIL'S INVITATION TO TENDER`
   (page 56, `#/texts/800`), `SCHEDULE 4 THE CONTRACTOR'S TENDER` (page 57, `#/texts/803`).
6. Scanned the markdown body for CPV codes, BS/ISO/PAS standards, fire-risk keywords (FRA,
   PAS 9980, BS 9999, BS 5839, BS 7974, fire door, compartmentation, survey type), evaluation
   criteria, MEAT / most-economically-advantageous, and KPI definitions. The only KPI reference
   is `F5.1` pointing to "Appendix E to the Invitation to Tender" which is not included in this
   document.
7. Detected a template placeholder mismatch at `WHEREAS A` on page 2 ("secure the provision of
   security services") contradicting the page 1 preamble and notice-level lot title ("Fire Risk
   Assessment Services"). Preferred the page 1 preamble and document title as authoritative.
8. Emitted `result.json`:
   - One lot record: `lot_number = "1"`, `title = "Provision of Fire Risk Assessments"`,
     descriptive body summarising the document's scope and the emptiness of Schedules 1–4, with
     all extractable sub-arrays (`requirements`, `evaluation_criteria`, `cpv_codes`,
     `product_requirements`, `technical_standards`) left empty to avoid fabrication.
   - `product_requirement_seeds = []` (the specification body is empty; fabricating seeds would
     violate the "do not claim a lot split, product seed, or standard reference unless the
     evidence section cites the relevant anchor" rule in SKILL.md).
   - `field_confidences` keyed per output path with integer bps values between 500 and 8500.
   - `extraction_confidence_bps = 2000` reflecting heading-only evidence plus eight warnings.
   - Eight warnings covering the four empty schedules, the template placeholder mismatch, the
     absence of CPV in the document, the absence of named technical standards, and a suggestion
     to review the document role classification.
   - Eight evidence anchors referencing the page 1 cover preamble, the buyer identity on page 2,
     the "WHEREAS A" clause, the list of constituent documents in clause 1.2, and the four empty
     schedule headings on pages 54–57.
   - `extraction_metadata`: `target_domain = "lots"`, `extraction_scope = "full_document"`,
     `source_markdown_hash = fbc2726ee7097df3e2a36ce2c3ac63d624704484f97086fe584db208c4a8ab92`,
     `policy_version = "2026.04.12"`, `document_role = "technical_specification"` (preserved
     from input metadata, not re-classified by this skill).
9. Wrote this run manifest for daemon finalisation.

## Decisions Made

- **Emit exactly one lot record (Lot 1) despite empty Schedule 1.** The page 1 cover preamble
  explicitly names "FIRE RISK ASSESSMENT SERVICES" and the document title declares "Lot 1"; this
  is cited evidence, so a single lot entry is emitted at low confidence rather than an empty
  `lots` array that would discard recoverable identity information.
- **Do not fabricate standards, CPV codes, product requirements, or evaluation criteria.** The
  SKILL.md non-goals and evidence rules require explicit anchors; the extraction emits empty
  arrays and explicit warnings instead of guessing from framework-level or lot-level priors.
- **Prefer page 1 preamble + notice lot title over the `WHEREAS A` "security services" phrasing.**
  The clause is a template placeholder that was not updated when the draft was copied for the
  fire-risk-assessment lot; treating it as authoritative would contradict the document title,
  the cover preamble, and the notice-level lot metadata. A warning is emitted so the daemon and
  any downstream validator can see the placeholder anomaly.
- **Do not emit `product_requirement_seeds`.** The document contains no candidate-level
  requirements (no specifications, no deliverable tables, no consumable schedule); seeding here
  would pollute `product_candidates` downstream.
- **Keep the skill-level `document_role` unchanged.** Reclassification is a routing concern; the
  skill surfaces the mismatch via `DOCUMENT_ROLE_REVIEW_SUGGESTED` so the owner of that routing
  authority can act on it.
- **Confidence set to 2000 bps** to reflect that only document-identity claims (lot number,
  title, buyer, service class) are supported by evidence, while the substantive specification
  content is absent.

## Warnings

Eight warnings emitted in `result.json`:

1. `SPECIFICATION_BODY_EMPTY` — Schedule 1 on page 54 is a heading with no body content.
2. `PRICING_SCHEDULE_EMPTY` — Schedule 2 on page 55 is a heading with no body content.
3. `ITT_SCHEDULE_EMPTY` — Schedule 3 on page 56 is a heading with no body content; KPIs cited in
   clause F5.1 live in "Appendix E to the Invitation to Tender" which is not included.
4. `CONTRACTOR_TENDER_EMPTY` — Schedule 4 on page 57 is a heading with no body content.
5. `TEMPLATE_PLACEHOLDER_MISMATCH` — `WHEREAS A` on page 2 says "security services" while page 1
   and the notice lot title say "Fire Risk Assessment Services"; authoritative scope taken from
   page 1 and document title.
6. `NO_CPV_IN_DOCUMENT` — no CPV codes appear in the extracted markdown.
7. `NO_EXPLICIT_TECHNICAL_STANDARDS` — no named BS/ISO/PAS references, Regulatory Reform (Fire
   Safety) Order references, or other explicit standards are present in the markdown.
8. `DOCUMENT_ROLE_REVIEW_SUGGESTED` — substantive content is Form of Contract + General Terms;
   reclassification to `draft_terms_and_conditions` / `contract_template` may be more honest.

No agent-side errors or retries occurred. Contract-mode run completed in a single pass within
the overlay budget; resume strategy not exercised.

## Evidence

All evidence anchors reference positions in the Docling extraction at
`document_extractions.id = 2fcf938f-fa4e-45da-aa92-1f9ab97c0478`
(`source_content_hash = 8ceea88c7c1944eb0b656cf91f20385e6299204ded72433d1ac454cc7df7eacf`;
markdown SHA-256 `fbc2726ee7097df3e2a36ce2c3ac63d624704484f97086fe584db208c4a8ab92`).

- **Page 1 cover preamble** (`section_ref = #/texts/1`): "RELATING TO THE PROVISION OF FIRE RISK
  ASSESSMENT SERVICES" — primary evidence for the Lot 1 title and service class. Document title
  field: "6278 - Draft T&Cs - Lot 1" corroborates lot number.
- **Page 2 parties clause** (`section_ref = #/texts/14`): "THE MAYOR AND BURGESSES OF THE LONDON
  BOROUGH OF BEXLEY of Civic Offices, 2 Watling Street, Bexleyheath, Kent DA6 7AT" — corroborates
  buyer identity against `procurement_processes.buyer_name`.
- **Page 2 WHEREAS A** (`section_ref = #/texts/19`): "The Council wishes to secure the provision
  of security services in Council Buildings as defined in the attached Specification ('the
  Services') during the period 1 September 2020 to 31 August 2022" — source of the contract
  period and of the template placeholder mismatch warning.
- **Page 2 clause 1.2** (`section_ref = #/texts/22`): enumerates "(a) The Form of Contract; (b)
  The General Terms and Conditions of Contract set out in Part 2 hereof; (c) The Special
  Conditions of Contract set out in Part 3 hereof; (d) The Specification appended hereto as
  Schedule 1; (e) The Pricing Schedule appended hereto as Schedule 2; (f) The Council's ITT
  appended hereto as Schedule 3; (g) The Contractor's Proposal as set out in its tender dated
  [ ] appended hereto as Schedule 4" — establishes that Schedules 1–4 are expected components;
  their empty headings below constitute the evidence of absence.
- **Page 45 clause F5.1**: "Where any part of the Services is stated in Appendix E to the
  Invitation to Tender to be the subject of a specific KPI the Contractor shall perform the
  Services in such a manner as will ensure that the Indicator is met" — source of the
  `ITT_SCHEDULE_EMPTY` warning (Appendix E lives in Schedule 3 which is empty here).
- **Page 54** (`section_ref = #/texts/795`): "SCHEDULE 1 SPECIFICATION" — heading only, empty
  body; basis for `SPECIFICATION_BODY_EMPTY`.
- **Page 55** (`section_ref = #/texts/797`): "SCHEDULE 2 PRICING SCHEDULE" — heading only,
  empty body; basis for `PRICING_SCHEDULE_EMPTY`.
- **Page 56** (`section_ref = #/texts/800`): "SCHEDULE 3 THE COUNCIL'S INVITATION TO TENDER" —
  heading only, empty body; basis for `ITT_SCHEDULE_EMPTY`.
- **Page 57** (`section_ref = #/texts/803`): "SCHEDULE 4 THE CONTRACTOR'S TENDER" — heading
  only, empty body; basis for `CONTRACTOR_TENDER_EMPTY`.

No browser was launched; `agent_claimed_pages_visited = 0` and `agent_claimed_source_url = ""`.
`agent_claimed_fingerprint_state = not_applicable` because this is a headless contract run with
no web session. `agent_claimed_locale_state = en-GB` and `agent_claimed_currency_state = GBP`
reflect the document's stated jurisdiction (London Borough of Bexley) and the governing law /
pricing convention implied by the Form of Contract — they are document-context claims, not
browser-session claims. The daemon's evidence appendix is the authoritative source for
machine-observed runtime facts.
