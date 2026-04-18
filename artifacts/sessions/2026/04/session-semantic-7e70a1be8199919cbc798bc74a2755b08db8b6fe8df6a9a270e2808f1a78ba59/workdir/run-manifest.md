---
manifest_version: 1
skill_id: pdf-extraction/tender-specification
skill_version: 1.0.0
job_id: job-85b3d740-47b2-4409-a5a3-6dc0bd96bdba
session_id: session-semantic-7e70a1be8199919cbc798bc74a2755b08db8b6fe8df6a9a270e2808f1a78ba59
execution_mode: contract
started_at: 2026-04-18T00:20:00Z
completed_at: 2026-04-18T00:23:00Z
outcome_class: completed_with_warnings
items_processed: 5
items_skipped: 0
items_failed: 0
warnings_count: 6
agent_claimed_pages_visited: 0
agent_claimed_source_url: null
agent_claimed_fingerprint_state: not_applicable_contract_mode
agent_claimed_locale_state: en-GB_inferred
agent_claimed_currency_state: GBP_inferred
---

# Run Manifest – pdf-extraction/tender-specification

## Summary

Extracted lot structure from the Bristol City Council "Metal-Works-DPS-Final-Draft" Dynamic
Purchasing System Agreement (document_extraction_id `faf0cc0a-8ac5-4b15-aece-8dd5a0bde61c`,
raw_document_id `raw-pc-7e2a6fb2-67a3-f011-813a-005056b64545-…`, source_content_hash
`598f77608fe261333965acc47967a66be3ebecfc09aa916871ca8de85eb6da75`). Identified 5 lots
(Repairs/modification, Supply or Supply+Install, Bespoke fabrication, Balcony balustrades,
Chute repairs) directly from the cover page lot list and BACKGROUND clauses, captured the
two evaluation regimes (DPS-establishment PAS 91 Pass/Fail and Call-Off ITT-driven award),
and recorded the only two cross-cutting standards present in the document body (PAS 91,
JCT Minor Works Contract 2016). Emitted no `product_requirement_seeds` and no per-lot
`product_requirements`: the document is a contract template whose Schedule 1 Specification
is explicitly deferred to per-call-off ITTs. extraction_confidence_bps = 6800
(structural identification high; per-lot detail intentionally sparse and warned).

## Steps Taken

1. Resolved input identifiers from the bound job parameters (document_extraction_id, raw_document_id,
   process_id, notice_id, document_set_id, stage_run_id, target_domain=lots, extraction_scope=full_document).
2. Read the Docling-produced markdown for the document from `state.db.document_extractions`
   (180132 chars / 1297 lines / 25 sections / 71 tables / page_count=0).
3. Cross-referenced with `contract_notices` (title "BSH/HRA/Metal Works - Dynamic Purchasing
   System (DPS) - Acceptance Round 6", buyer "Bristol City Council", deadline 2027-09-30,
   currency GBP) and the 5 pre-existing rows in `notice_lots` for cross-reference only —
   not used as authoritative override of the document body.
4. Scanned headings (`grep '^## \|^### '`) and located: cover-page lot block (lines 9–17),
   BACKGROUND clauses (lines 39 and 41), Schedule listing (lines 218–230), Schedule 1
   body (lines 919–922 — specification deferred), Schedule 2 Award Criteria (lines 925–937),
   Schedule 3 Pricing (lines 939–941), Schedule 7 Call-Off Terms (lines 984–988), Order
   Form Appendix 1 reference (line 1027), Appendix 3 Data Processing standards (line 1142).
5. Searched the body for technical-standard tokens (BS, BS EN, EN, ISO, JCT, PAS) and
   found exactly three standard references: PAS 91 (Schedule 2), JCT Minor Works Contract
   2016 (Schedule 7), and BS EN 15713:2009 (Appendix 3 – data destruction; classified as
   contract-wide, not lot-specific).
6. Confirmed absence of CPV codes in the document body; CPVs (44221300-8, 44400000-4,
   45223100-7) exist only in process_context (notice metadata), so left lot.cpv_codes empty
   with a warning explaining the omission.
7. Confirmed absence of concrete product items, dimensions, materials and per-lot quantities
   in the document body; therefore set product_requirement_seeds = [] and per-lot
   product_requirements = [], each backed by a warning.
8. Built the result.json with lot-stable numeric `lot_number` strings ("1".."5"), one
   primary `evidence_anchor` per lot pointing at the cover-page lot heading, plus
   per-criterion / per-standard anchors. Validated against
   `skills/pdf-extraction/tender-specification/result.schema.json` (jsonschema.validate
   passed, 5 lots / 0 seeds / 6 warnings / 11 evidence_anchors).

## Decisions Made

- **Lot numbering**: used the document's own lot enumeration ("Lot 1" .. "Lot 5") as
  string `lot_number`. The document and the existing `notice_lots` rows agree on the
  set and ordering, so no renumbering was needed.
- **Did not invent product seeds**: the SKILL forbids claiming seeds without evidence
  anchors. Schedule 1 in the body explicitly defers detail ("AS SPECIFIED IN ANY CALL FOR
  FURTHER COMPETITION"), and the only per-lot signal in the body is the lot title.
  Empty `product_requirement_seeds` + a warning is the correct outcome.
- **Did not propagate notice CPVs into lots**: the SKILL says pre-existing notice metadata
  is for cross-reference only; the document does not assert CPVs per lot, so per-lot
  cpv_codes arrays are empty and a warning records the process-level CPVs the daemon may
  inherit if its policy allows.
- **Cross-cutting standards (PAS 91, JCT MW 2016) replicated per lot**: both apply across
  all 5 lots by construction (selection step / call-off contract form). They are listed
  on each lot with explicit `context` calling out that they are cross-cutting, rather than
  hidden at a top-level only, because the schema places `technical_standards` at lot level.
- **BS EN 15713:2009 deliberately excluded from per-lot tech standards**: it appears only
  in Appendix 3 for secure data destruction at end of contract — not a metalworks
  technical standard. Recorded in the warnings instead of in lot.technical_standards.
- **Did not use `notice_lots` row descriptions as authoritative**: e.g. the existing Lot 3
  row mentions "lockers for parking meters, water taps, signs, and specialist artwork" —
  that text originates from notice metadata, not from this Docling extraction of the DPS
  Agreement. Per-lot description here is therefore terser than the notice_lots row.
- **page_ref convention**: page_count = 0 in the docling extraction record, so all
  evidence_anchor.page_ref values use `line:N` markdown line numbers. Recorded as a
  suspect-output warning so downstream consumers do not treat them as PDF page numbers.
- **Outcome class**: `completed_with_warnings`. Lot identification is high-confidence
  (extraction_confidence_bps = 6800; field_confidences.lots.title = 9800), but per-lot
  detail is intentionally sparse with 6 warnings explaining the deferred spec.

## Warnings

Six warnings emitted in `result.json.warnings`, summarised:

1. AGENT_PARTIAL_FAILURE — Schedule 1 Specification deferred to call-off; no concrete
   product items in document body; product_requirement_seeds and per-lot
   product_requirements are empty by design.
2. AGENT_PARTIAL_FAILURE — Schedule 2 quality criteria/weightings table is empty;
   Schedule 3 marked "NOT REQUIRED UNTIL CALL FOR FURTHER COMPETITION STAGE"; only the
   PAS 91 Pass/Fail tag is populated.
3. AGENT_PARTIAL_FAILURE — No CPV codes in the document body; process-level CPVs are
   not authoritatively mapped to lots by this skill.
4. AGENT_PARTIAL_FAILURE — Per-lot descriptions in the document body consist solely of
   lot titles repeated in three places; richer descriptions in `notice_lots` originate
   from notice metadata, not this DPS Agreement document.
5. AGENT_SUSPECT_OUTPUT — page_count=0 in document_extractions; page_ref values use
   markdown line numbers, not PDF pages.
6. AGENT_SUSPECT_OUTPUT — BS EN 15713:2009 is a data-destruction standard from
   Appendix 3 and was deliberately excluded from per-lot technical_standards arrays.

## Evidence

All major claims in `result.json` are anchored to the following observable evidence in
the Docling markdown for `document_extractions.id = faf0cc0a-8ac5-4b15-aece-8dd5a0bde61c`:

- **Lot identification (5 lots)** — cover-page heading block at `line:1-17` (`## Lot 1
  – Repairs and/or modification to metal items` … `## Lot 5 – Chute Repairs`); reinforced
  by BACKGROUND clause 1 at `line:39` and BACKGROUND clause 3 (per-provider admission
  template) at `line:41`. Replicates exactly the 5 rows already present in
  `notice_lots WHERE notice_id = 'pc-7e2a6fb2-67a3-f011-813a-005056b64545'`.
- **Schedule list / document role** — `line:218-230` lists Schedules 1–7 by name,
  confirming this is a DPS Agreement template with a deferred specification structure.
- **Specification deferred** — `line:919-922`: "THE GOODS, SERVICES AND/OR WORKS
  SPECIFICATION AS SPECIFIED IN ANY CALL FOR FURTHER COMPETITION IN RESPECT OF THE LOTS
  FOR WHICH THE PROVIDER IS ADMITTED TO THE DPS." Plus Order Form §2.1 at `line:1027`:
  "The Contract Goods, Services or Works required are as set out in the Specification
  attached at Appendix 1". Both used to justify empty product seeds.
- **Schedule 2 Award Criteria (PAS 91 + ITT)** — `line:925-937`, including the "[Pas91
  Pass/Fail]" tag (`line:933`) and the call-off statement (`line:935-937`).
- **Schedule 3 Pricing (deferred)** — `line:939-941`: "NOT REQUIRED UNTIL CALL FOR
  FURTHER COMPETITION STAGE".
- **Schedule 7 Call-Off Terms / JCT MW 2016** — `line:984-988`: "Agreement Incorporating
  and Amending JCT Minor Works Contract 2016 Edition. … Long Form Contract".
- **Insurance minimums** — `line:118-122` (Articles of Agreement clause 7): EL £10M,
  PL £5M, PI £2.5M, used as cross-cutting requirement on every lot.
- **BS EN 15713:2009 (data destruction, excluded)** — `line:1142` (Appendix 3 Data
  Processing), used to justify the exclusion-warning rather than a per-lot listing.
- **Source identity / hash** — `document_extractions.source_content_hash =
  598f77608fe261333965acc47967a66be3ebecfc09aa916871ca8de85eb6da75`; raw_document_id =
  `raw-pc-7e2a6fb2-67a3-f011-813a-005056b64545-0d23f1eb92f27a81301f5d1ec4f4d4b298438f05b411a5abd812f036a4bf4058`.
- **Process / notice context** — `contract_notices.id = pc-7e2a6fb2-67a3-f011-813a-…`,
  buyer "Bristol City Council", title "BSH/HRA/Metal Works - Dynamic Purchasing System
  (DPS) - Acceptance Round 6", deadline 2027-09-30, currency GBP, CPV signals
  "44221300-8 | 44400000-4 | 45223100-7" (notice-level only, intentionally not
  promoted into per-lot cpv_codes).
- **Schema validation** — `python3 jsonschema.validate(result.json,
  skills/pdf-extraction/tender-specification/result.schema.json)` returned without
  error: 5 lots / 0 seeds / 6 warnings / 11 evidence_anchors.
