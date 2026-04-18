---
manifest_version: 1
skill_id: pdf-extraction/tender-specification
skill_version: 1.0.0
job_id: job-bc9872cd-52f6-4162-bfd0-6a4478c8d625
session_id: session-semantic-c8a7a7fc375066234e2f2afa201a42cb8138bc6ef554975e235f19dcf5ee5d0c
execution_mode: contract
started_at: 2026-04-18T00:33:00Z
completed_at: 2026-04-18T00:34:30Z
outcome_class: success_with_warnings
items_processed: 3
items_skipped: 0
items_failed: 0
warnings_count: 6
agent_claimed_pages_visited: 0
agent_claimed_source_url: "local://state.db/document_extractions/fef1b5d5-5ebc-4847-bf65-373e72c98858"
agent_claimed_fingerprint_state: not_applicable
agent_claimed_locale_state: en-GB
agent_claimed_currency_state: GBP
---

# Run Manifest — pdf-extraction/tender-specification

## Summary

Semantic extraction over the Docling-produced markdown for Bristol City Council's
"Joinery Timber Supplies — DPS" Invitation to Tender (ProContract reference DN625127;
FTS notice BRISTOLCC001-DN625127-70301139 dated 14/09/2022). Three lots were
recovered from the document's own numbered enumeration (1.1.1, 1.1.2, 1.1.3)
covering softwood timbers (Lot 1), plywood + door blanks (Lot 2), and hardwood +
hemlock (Lot 3). Five product requirement seeds were emitted — one per distinct
material/product family named in the lot titles. Evaluation criteria were
recovered in full: two-stage evaluation (selection gate via Appendix 1 Table A,
then Technical Assessment) plus the call-off split (100% price under £25k;
60-80% price / 0-20% quality / 20% social value above £25k). No product-level
technical standards (BS/EN/ISO/FSC/PEFC) appear in this document; only
procedural/legal standards are cited. CPV codes (03419000-0 Timber, 44220000-8
Builders' joinery) were attributed from notice-level `main_category` passed in
process_context since the tender-spec markdown does not restate them. One
Docling ordering anomaly was observed: Lot 1 (1.1.1) is emitted physically
later in the markdown than Lots 2 and 3, producing an out-of-sequence warning
but not preventing recovery.

Outcome class is `success_with_warnings`: the three-lot structure, evaluation
model, and product seeds are confidently recovered from the document; the
warnings reflect genuine gaps in the *source* document (missing product
standards, unitemised lot values, absent lot-level CPV), not extraction defects.
Overall extraction confidence: 7000 bps (70%).

## Steps Taken

1. Read bound SKILL.md and `result.schema.json` to lock the output contract
   (required keys: `lots`, `product_requirement_seeds`, `field_confidences`,
   `extraction_confidence_bps`, `warnings`, `evidence_anchors`,
   `extraction_metadata`; target_domain const `"lots"`).
2. Located the Docling extraction row in `state.db.document_extractions`
   keyed by `document_extraction_id = fef1b5d5-5ebc-4847-bf65-373e72c98858`.
   Confirmed: `raw_document_id = raw-pc-b7aaab27...`, `page_count=0`
   (Docling reported zero explicit page anchors for this .docx-sourced
   extraction), `document_title = ITT-Joinery-Timber-Supplies-DPS`,
   `document_role = technical_specification`, `extraction_policy_version
   = 2026.04.12`, `source_content_hash = c4913611dfdd1bc1186cb11c192f9ffd82015dcd806acc255d5720d9e5c58aa9`.
3. Read `contract_notices` row for `pc-b7aaab27-1f34-ed11-8119-005056b64545`
   to recover CPV / `main_category` signals
   (`03419000-0 - Timber | 44220000-8 - Builders' joinery`) and process title
   (`BSH/HRA/Joinery Timber Supplies - DPS - Acceptance Round 7`).
4. Dumped the full 42,279-char markdown (419 lines) to /tmp for analysis.
   Scanned for lot markers, evaluation sections, pricing, standards keywords
   (BS/EN/ISO/FSC/PEFC/grade/class) and safety schemes. Confirmed no product
   standards are cited; only procedural/legal (Public Contracts Regulations
   2015, Equality Act 2010, Bribery Act 2010, Fraud Act 2006, Data Protection
   Act 2018, GHG Protocol, Social Value Act 2012) plus fleet safety schemes
   (CLOCS, FORS).
5. Reconstructed lot structure from the three numbered markers:
   - `1.1.1. Lot 1 - Supply and delivery of Soft wood timbers` (markdown line 383, §4)
   - `1.1.2. Lot 2 - Supply and delivery of Plywood and Door Blanks` (line 153, §1)
   - `1.1.3. Lot 3 - Supply and delivery of Hardwood and Hemlock` (line 154, §1)
   Logged the out-of-sequence ordering as a warning.
6. Derived five product requirement seeds by splitting compound-named lots
   into distinct candidate families (Lot 2 → plywood + door blanks; Lot 3 →
   hardwood + hemlock). Each seed was anchored to the specific lot-title line.
7. Captured seven evaluation criteria records per lot where applicable
   (Stage-1 selection gate, Stage-2 Technical Assessment, call-off ≤£25k
   rule, call-off >£25k blended rule, pricing rules, abnormally-low-tender
   process, Social Value gate — the last four being shared across lots).
8. Attributed notice-level CPVs to each lot and warned that the attribution
   is from `process_context`, not the tender-spec markdown itself.
9. Page refs estimated from the document's Table of Contents page numbers
   (ToC at markdown lines 23-94). Emitted `ESTIMATED_PAGE_REFS` warning so
   downstream consumers don't over-trust the page_ref values.
10. Populated `source_markdown_hash` with the daemon-computed
    `source_content_hash` from `document_extractions` to keep hash semantics
    aligned with the daemon's own ingestion record.
11. Validated the draft JSON against `result.schema.json` using
    `jsonschema.validate()` — passed.
12. Wrote `result.json` to the working directory alongside `storage.sqlite3`
    as the skill contract requires, then authored this manifest.

## Decisions Made

- **Three lots, not one.** The pre-existing `notice_lots` row for this notice
  (`pc-b7aaab27-...-lot-dps-all`) is a single placeholder "DPS Admission and
  Call-off Award Criteria (single document scope)". Per SKILL.md's inputs
  section ("do not treat pre-existing lot rows as authoritative truth if the
  tender document contradicts them"), I did not mirror that single-lot shape.
  The tender document explicitly lists three lots; the extraction reflects
  three lots.
- **Lot ordering.** Output order follows lot *number* (1, 2, 3) as declared
  in the document, not the physical order of markdown appearance. A
  `LOT_ORDERING_OUT_OF_SEQUENCE` warning records the physical-vs-logical
  mismatch for downstream audit.
- **CPV inheritance.** Since the tender-spec markdown does not repeat CPV
  codes at the lot level, CPVs were carried down from the notice-level
  `main_category` value passed in process_context. A dedicated warning
  (`LOT_LEVEL_CPV_NOT_CITED_IN_DOCUMENT`) flags this provenance so the
  downstream pipeline can decide whether to treat lot CPVs as
  notice-derived evidence or as document-direct evidence.
- **Product requirement seed granularity.** Lot 2 is a compound name
  ("Plywood and Door Blanks") and Lot 3 similarly ("Hardwood and Hemlock").
  I split each compound lot into its constituent product families
  (plywood + door_blanks; hardwood + hemlock) giving five seeds in total.
  Lot 1 is single-headed (Softwood timbers) — one seed.
- **Technical standards emitted as empty arrays.** The document is
  procedurally oriented and contains no product technical standards. I
  emitted `technical_standards: []` per lot and per seed rather than
  shoehorning procedural/legal standards into that slot, plus a
  `PRODUCT_TECHNICAL_STANDARDS_MISSING` warning so the pipeline knows
  to look elsewhere (Appendix 1 Evaluation Criteria, or the DPS Agreement
  / Call-Off Contract documents referenced in the notice attachments).
- **Document-role vs. content mismatch.** The extraction metadata
  preserves `document_role = technical_specification` as passed, but a
  `DOCUMENT_ROLE_VS_CONTENT_MISMATCH` warning records that the body is
  structurally an ITT (Instructions to Tenderers) rather than a
  technical product specification.
- **No lot-level valuations.** The document states a single £1,400,000
  ex-VAT total (§1.1.1.3) across all lots and 48 months. I did not
  invent per-lot splits; emitted `LOT_VALUES_NOT_ITEMISED` warning.
- **Confidence budget.** 7000 bps overall: lot identity and numbering
  are high-confidence (9500), description/requirements mid-high (7000),
  CPV attribution reduced (6000) because it's inferred from process_context,
  technical_standards low (3000) because we are inferring absence.

## Warnings

1. `LOT_ORDERING_OUT_OF_SEQUENCE` — Lot 1 marker (1.1.1) appears in §4 of
   the markdown while Lots 2 & 3 appear together in §1. Lot structure is
   reconstructed from enumeration markers; downstream consumers should
   not rely on markdown order to infer lot sequence. Evidence refs:
   `§1 Lots / 1.1.2`, `§1 Lots / 1.1.3`, `§4 SECTION FOUR: CONTRACT / 1.1.1`.
2. `PRODUCT_TECHNICAL_STANDARDS_MISSING` — No BS/EN/ISO/FSC/PEFC (or
   equivalent product) standards are cited anywhere in this document.
   Product-level standards likely live in the tender's separate
   Evaluation Criteria appendix, DPS Agreement, or Call-Off Contract
   documents (present in the notice's `documents_downloaded` manifest but
   not supplied as input to this extraction).
3. `DOCUMENT_ROLE_VS_CONTENT_MISMATCH` — Document tagged
   `document_role = technical_specification` but content is an
   Invitation to Tender (procedural). Recoverable fields: lot titles,
   evaluation gates, call-off mechanics, pricing rules. Non-recoverable:
   species grades, dimensional tolerances, supply frequency, kiln-drying
   requirements.
4. `LOT_LEVEL_CPV_NOT_CITED_IN_DOCUMENT` — Lot CPV attribution drawn from
   notice-level `main_category` (03419000-0, 44220000-8) rather than
   document-direct evidence. Pipeline should treat these as notice-derived.
5. `LOT_VALUES_NOT_ITEMISED` — Only process-total £1,400,000 ex-VAT is
   stated; per-lot values not broken out. Evidence: §1 Proposed Contract
   1.3.
6. `ESTIMATED_PAGE_REFS` — Docling produced `page_count = 0` for this
   extraction (the source was a .docx); `page_ref` values are estimated
   from the document's own Table-of-Contents page numbers. `section_ref`
   is the authoritative anchor.

## Evidence

Lot identity (authoritative anchors):

- Lot 1: markdown §4 SECTION FOUR: CONTRACT, line `- 1.1.1. Lot 1 - Supply and delivery of Soft wood timbers`.
- Lot 2: markdown §1 Lots, line `- 1.1.2. Lot 2 - Supply and delivery of Plywood and Door Blanks`.
- Lot 3: markdown §1 Lots, line `- 1.1.3. Lot 3 - Supply and delivery of Hardwood and Hemlock`.

Contract envelope (§1 Proposed Contract):

> The contract length will be for 48 months, with no option to extend. The
> anticipated commencement date is 1 December 2022. The estimated total
> value of this contract (inclusive of options) is £1,400,000.00.

Regulatory frame (§1 Procurement Process):

> The procurement process will be conducted in accordance with the
> requirements of the Public Contracts Regulations 2015. Specifically,
> the Council will be using the Open procedure. Find a Tender Service
> (FTS) Contract Notice reference number BRISTOLCC001-DN625127-70301139
> dated 14/09/2022.

Evaluation model (§3 Evaluation of the Bidder Submission):

> Stage 1: Selection Criteria ... Bidders will be assessed against the
> Selection Criteria as set out in Table A of Appendix 1 ...
> Stage 2: Award Criteria ... The council will evaluate on the basis of
> Technical Assessment.

Call-off mechanics (§3 Call Off Process):

> Call Off between £0 and £25k – Will be 100% Price
> Call Off above £25k – Will be 60% - 80% Price and Quality 0% - 20% and
> Social Value 20%

Pricing rules (§3 Pricing 1.1–1.6):

> Pricing will be evaluated against the cost to provide the
> goods/works/service in the specification. The pricing must include and
> allow for everything ... All rates and prices must be quoted in pounds
> sterling (GBP). VAT should not be included in the bid rates and prices.

CPV attribution source (from `contract_notices.normalized_payload.categories`
of notice `pc-b7aaab27-1f34-ed11-8119-005056b64545`):

> 03419000-0 - Timber | 44220000-8 - Builders' joinery

Daemon appendix refs:

- document_extractions.id = fef1b5d5-5ebc-4847-bf65-373e72c98858
- document_extractions.source_content_hash = c4913611dfdd1bc1186cb11c192f9ffd82015dcd806acc255d5720d9e5c58aa9
- document_extractions.extraction_policy_version = 2026.04.12
- document_extractions.extraction_engine = docling / 2.88.0 / standard
- document_extractions.document_role = technical_specification (tagged)
- document_extractions.page_count = 0 (Docling reported no explicit page anchors)
- contract_notices.id = pc-b7aaab27-1f34-ed11-8119-005056b64545
- procurement_processes.id = proc-pc-b7aaab27-1f34-ed11-8119-005056b64545
- stage_run_id = stage-run-57a5f226-b1b8-4236-9510-0821c0ebd20d-proc-pc-b7aaab27-1f34-ed11-8119-005056b64545
- result artifact: ./result.json (validated against result.schema.json)
