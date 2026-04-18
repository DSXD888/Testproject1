---
manifest_version: 1
skill_id: pdf-extraction/tender-specification
skill_version: 1.0.0
job_id: job-c8217747-5cb5-4ceb-a7af-32eefee30073
session_id: session-semantic-07e47005c358e5398fdd6bf7a8d01f2574db03b6adf335a71ef5cfb83847ad79
execution_mode: contract
started_at: 2026-04-17T21:22:00Z
completed_at: 2026-04-17T21:25:00Z
outcome_class: completed_success
items_processed: 1
items_skipped: 0
items_failed: 0
warnings_count: 5
agent_claimed_pages_visited: 10
agent_claimed_source_url: "n/a (semantic extraction over pre-parsed Docling markdown; source content hash 9fe0fc2fe3af35923fee75b751af5f90089a8959fa91e39dbbc574de1c9a03ea)"
agent_claimed_fingerprint_state: not_applicable
agent_claimed_locale_state: en-GB
agent_claimed_currency_state: GBP
---

# Run Manifest — pdf-extraction/tender-specification

## Summary

Performed full-document semantic extraction of the Ely Library tender specification
(`document_extraction_id = bac5bf60-edbf-40b3-9336-d646f5316a3b`,
`raw_document_id = raw-pc-19574d11-0929-f111-813c-005056b64545-2e8c1b47a495…`,
`document_role = technical_specification`, 10 pages, 24 sections, 1 table).
The extraction produced one authoritative lot record (lot_number `1`, matching the
pre-existing `notice_lots` row on notice `pc-19574d11-0929-f111-813c-005056b64545`)
because this procurement is explicitly single-lot and Appendix 4 describes internal
functional "zones" rather than separate bidding lots.

Output emitted as `result.json` conforming to
`skills/pdf-extraction/tender-specification/result.schema.json`:

- `lots`: 1 entry with 35 requirement bullets, 2 evaluation criteria, 2 CPV codes
  (inherited from notice process context, anchored to §1.1 scope language),
  12 product requirements, and 5 technical standards.
- `product_requirement_seeds`: 12 seeds (carpet, adult shelving, children's shelving,
  study desks, bespoke staff desk, self-service trolleys/bins, social soft seating,
  lobby bar-desk, leaflet display, signage/graphics package, children's picture-book
  stand, children's activity storage), each with `lot_number=1` and evidence anchor.
- `extraction_metadata.target_domain = "lots"`, `extraction_scope = "full_document"`,
  `source_markdown_hash = 9fe0fc2fe3af35923fee75b751af5f90089a8959fa91e39dbbc574de1c9a03ea`,
  `policy_version = "2026.04.12"`, `document_role = "technical_specification"`.
- `extraction_confidence_bps = 8200`; 5 structured warnings.

## Steps Taken

1. Discovered the Docling-produced markdown in
   `/Users/alinaaitova/Desktop/Testproject1/state/state.db`, table
   `document_extractions`, row id `bac5bf60-edbf-40b3-9336-d646f5316a3b`. Confirmed
   `extraction_policy_version = 2026.04.12` and `expected_overlays_json =
   ["pdf-extraction/tender-specification"]` — overlay assignment matches bound skill.
2. Retrieved `extracted_markdown` (23,019 bytes, 269 lines) and wrote it to disk for
   review. Retrieved `extracted_sections_json` (24 sections) to obtain the
   heading→page_start/page_end map used to anchor evidence.
3. Cross-referenced `procurement_processes` and `notice_lots` for the same process
   (`proc-pc-19574d11-0929-f111-813c-005056b64545`). Confirmed existence of one lot
   row (`lot_number=1`, title "25112 Library Services Engage, Design and Fit out
   Contract - Ely Library"). Preserved this lot numbering.
4. Read `contract_notices.main_category`
   (`39155000-3 - Library furniture | 79932000-6 - Interior design services`) for
   CPV cross-reference. Since no CPV is literally printed inside Appendix 4, emitted
   both codes with a corresponding `CPV_NOT_IN_DOCUMENT` warning and anchored to
   §1.1 scope language which matches both codes.
5. Walked §§1-16 of the markdown end-to-end. Translated each numbered subsection into
   lot-level requirement bullets, preserving the source section reference and the
   spec's own counts (18 desks / 10 powered, 6 public PCs, 210/120/18/20 shelving,
   38/62/10 junior/non-fiction/kinder, 3 Lyngsoe Phoenix + 2 dump bins, 8-10 soft
   seating, 850sqm footprint, etc.).
6. Extracted 12 product requirement seeds from the tangible-supply paragraphs
   (§4.6, §5.1.3, §5.1.6-5.1.8, §5.2, §5.3, §5.5, §5.6.2, §5.8, §5.9, §5.10.2,
   §5.10.3, §4.5). Each seed is keyed by a stable slug (`ely-…`) and anchored to a
   specific section.
7. Emitted evaluation criteria from §10.1 (sustainability/carbon) and §10.5
   (case studies), marked with `weighting_text` noting the numeric weight is not
   disclosed inside this appendix; raised `EVALUATION_WEIGHTING_ABSENT` warning.
8. Emitted five technical standards (Modern Slavery Act, Equality Act / UK EDI,
   UK GDPR, RAMS, accessibility / dementia- and autism-friendly design).
9. Built a top-level `evidence_anchors` array covering 26 anchors across pages 1-10
   to support daemon-side audit and seed→anchor traceability.
10. Validated `result.json` against `result.schema.json` using a Draft 2020-12
    validator; 0 errors.
11. Wrote this run manifest with YAML frontmatter and narrative sections.

## Decisions Made

- **Single lot emission.** The tender is explicitly single-lot: (a) the only
  `notice_lots` row has `lot_number=1`, and (b) §5 ("Distinct Zones") describes
  internal functional zones inside one contract, not separable bidding lots. I
  preserved `lot_number=1` instead of inventing zone-level lots, and raised the
  `SINGLE_LOT_CONFIRMED` warning. Rationale: SKILL.md explicitly warns against
  "guessing lot boundaries".
- **CPV policy.** Emitted the two notice-level CPVs (39155000-3 Library furniture,
  79932000-6 Interior design services) on the lot, because §1.1 scope language
  ("design, furniture, signage, and carpet") is a clean literal match. To stay
  honest about provenance, I attached a `CPV_NOT_IN_DOCUMENT` warning noting that
  the codes come from notice process context, not from a literal quote inside
  Appendix 4. Seed-level CPV assignment: used 39155000-3 on every furniture seed;
  used 79932000-6 only on the signage/graphics package which is the design-service
  bucket.
- **Evaluation weighting.** §10 states two criteria are "part of the scored
  submission" but does not disclose numeric weightings. I recorded the criteria
  with `weighting_text` noting the weighting is not disclosed in this document,
  and surfaced a `EVALUATION_WEIGHTING_ABSENT` warning so the daemon/downstream
  pipeline does not assume weightings are complete. Full evaluation matrix is
  expected to live in the ITT, not this appendix.
- **Seed granularity.** Favoured zone-level seeds over hyper-fine-grained ones
  (e.g. one `ely-study-desks` seed covering all 18 desks rather than 18 separate
  seeds). This matches how an operator would tender-source furniture and keeps
  seed keys stable across re-runs (SKILL.md Resume Strategy). I did split out
  distinct SKUs where the specification treated them as distinct
  (trolleys-vs-dump-bins, bespoke staff desk, picture-book stand), and kept
  intangible design deliverables (e.g. "design methodology") out of the seeds.
- **Power/data exclusion.** §2.2 and §5.2.4/§5.2.6 put the installation of power
  and data lines out of scope. I recorded this on the lot's requirements bullets
  and raised `POWER_DATA_PROVISION_OUT_OF_SCOPE` so downstream matching does not
  treat electrical installation as a bidder obligation; furniture seeds still
  carry the declared power/data counts they must accommodate.
- **OCR anomalies.** Preserved normalised text in seeds/requirements while
  keeping the original misspellings in anchor quotes so anchors still match the
  source markdown byte-for-byte. Raised `CHAR_OCR_NOISE` warning.
- **Confidence scoring.** Set `extraction_confidence_bps = 8200` (82%). The lot
  structure, requirements and seeds are high-confidence (direct quotes); the
  evaluation-criteria completeness and CPV provenance are the main uncertainty
  sources, reflected in their lower per-field confidences (6500 and 5500 bps).

## Warnings

The same five structured warnings are emitted inside `result.json.warnings`:

1. `EVALUATION_WEIGHTING_ABSENT` — §10 identifies scored criteria but omits
   numeric weightings (ITT expected to carry them).
2. `CPV_NOT_IN_DOCUMENT` — CPVs are derived from notice process context; they
   cleanly match §1.1 but are not literally printed in Appendix 4.
3. `SINGLE_LOT_CONFIRMED` — Internal zones in §5 are functional zones, not
   separate lots; `notice_lots.lot_number=1` preserved.
4. `POWER_DATA_PROVISION_OUT_OF_SCOPE` — Power/data provision is delivered by
   the Authority's contractors (§2.2, §5.2.4-6, §5.4.4); bidder still
   coordinates and supplies furniture that accommodates declared counts.
5. `CHAR_OCR_NOISE` — OCR/typographic anomalies in the Docling output
   (`Lynsgoe`→`Lyngsoe`, `Morden`→`Modern`, `interesting`→`interested`) were
   normalised in seed text; anchor quotes retain the original form.

## Evidence

Every lot claim, seed and standard carries a dedicated `evidence_anchor` with
`page_ref`, `anchor_text` and `section_ref` inside `result.json`. Top-level
`result.json.evidence_anchors` holds 26 additional document-level anchors
spanning pages 1-10. Representative anchors backing the key claims in this
manifest:

- **Document identity.** page 1, "Appendix 4 - Specification" (document
  title). §1.1 "For the delivery of design, furniture, signage, and carpet
  to refresh the public library service in Ely library ..." (scope one-liner).
- **Single-lot structure.** `notice_lots` row `lot_number=1` for
  `notice_id=pc-19574d11-0929-f111-813c-005056b64545` (state.db).
  §5 "The library is a large single open plan space, with distinct zones as
  highlighted in this specification to support core activity." (zones ≠ lots).
- **CPV alignment.** §1.1 enumerates "design, furniture, signage, and carpet"
  — maps to CPV 39155000-3 (Library furniture) and 79932000-6 (Interior design
  services) from `contract_notices.main_category`.
- **Quantitative requirements.** §5.2.2-§5.2.3 ("eighteen (18) distinct desks
  ... at least 10 must have access to power"), §5.3 ("210 Adult Fiction ...
  120 Adult non-fiction ... 18 young adult ... 20 local studies"), §5.1.8
  ("38 shelves of junior fiction, 62 shelves of non-fiction ... 10 of kinder
  boxes"), §5.4.2-§5.4.3 ("six (6) public PCs ... One (1) desk should be wheel
  chair accessible/height adjustable"), §5.5.3 ("three(3) power points and
  one(1) data port for each station"), §5.6.2 ("three Lynsgoe Phoenix devices
  each supported by a double-sided trolley and two dump bins between the
  three"), §5.8.2 ("soft seating for 8-10 people including table for drinks"),
  §3.3 ("just under 850sqm").
- **Evaluation criteria.** §10.1 ("robust strategies to reduce the carbon
  footprint ... This will be part of the scored submission"), §10.3
  ("warranties ... especially when answering question 2 on sustainability"),
  §10.5 ("Case studies from similar work ... form part of the scored
  submission").
- **Technical standards.** §13.1 Modern Slavery Act (page 9), §14.1 Equality
  Act and EDI regulations (page 9), §15.1-§15.2 UK GDPR with 72-hour breach
  reporting (page 10), §6.2 / §7.1.a Risk Assessments and Method Statements,
  §4.4 fully accessible / dementia- and autism-friendly.
- **Out-of-scope boundary.** §2.1 (2022 main entrance/stairway), §2.2
  (power/data and specialist IT such as public network PCs and self-service
  machines delivered by Authority's IT and electrical contractors).
- **Delivery timetable.** §9 project/contract timetable table on page 8:
  site visit 6 July 2026 → handover 4-8 January 2027 → 12-month defects
  period from handover.
