---
manifest_version: 1
skill_id: pdf-extraction/tender-specification
skill_version: 1.0.0
job_id: job-a92f2421-0040-4360-a897-32a52bb21cb8
session_id: session-semantic-50948e28b4cb85e9e121849dc3baf77d30844218f2b825e239c3d83e86a46df2
execution_mode: contract
started_at: 2026-04-18T00:00:00Z
completed_at: 2026-04-18T00:00:00Z
outcome_class: success
items_processed: 1
items_skipped: 0
items_failed: 0
warnings_count: 8
agent_claimed_pages_visited: 0
agent_claimed_source_url: "(docx, source file: 6278 - Draft Specification - Lot 7 - Provision of Lift Services - Corporate Buildings)"
agent_claimed_fingerprint_state: not_applicable
agent_claimed_locale_state: en-GB
agent_claimed_currency_state: GBP
---

# Run Manifest — pdf-extraction/tender-specification

## Summary

Extracted lot-level semantic structure from a single-lot tender specification: "6278 - Draft specification - Lot 7 - Provision of Lift Services - Corporate Buildings" (London Borough of Bexley; process proc-pc-dc1daee2-772e-f111-813c-005056b64545). The source is a DOCX-converted Docling markdown payload (page_count=0, so page references are unavailable). Produced one authoritative `lots` entry for Lot 7, 9 product-requirement seeds, 20 technical-standard references, 31 distilled requirements, 16 evidence anchors, and 8 warnings. Extraction scope is `full_document`; target domain is `lots`. Overall extraction confidence: 8800 bps.

## Steps Taken

1. Read contract job parameters (document_extraction_id `2fa9aa5b-20b2-4d69-90c8-5a2b50021afa`, process_id, notice_id, document_set_id, stage_run_id, target_domain=lots, extraction_scope=full_document).
2. Loaded the Docling-produced extracted markdown and metadata from the daemon state database (`state/state.db` → `document_extractions`). Captured the canonical `source_content_hash` `436bc238fb5ea4004e5a52610e4a01a6915a1b763cca45f7a09024927b67f791` for the manifest's `source_markdown_hash` field.
3. Confirmed `page_count = 0` on the extraction row (docx source), so anchored all evidence by section heading rather than page number; recorded the decision as a `NO_PAGE_REFS_AVAILABLE` warning.
4. Read process- and notice-level context: process `6278 - Building Services Framework - Lots 1-11` (Bexley), notice CPVs `50700000-2`, `50750000-7`, `71315000-9`.
5. Analysed the markdown end-to-end and identified the document as a single-lot specification covering only Lot 7 (Lift Services). Issued a `SINGLE_LOT_DOCUMENT` warning so downstream consumers do not mistake this for a multi-lot catalogue.
6. Distilled lot metadata: title, description, term (3+2 years from Sept 2026), scope (24 lifts across 12 corporate sites, with call-off extension to schools and other sites), contract model (single supplier, CAFM-integrated).
7. Extracted 31 lot-level requirements across service operations, CAFM integration, statutory compliance, environmental/social obligations, reporting, mobilisation and handover.
8. Enumerated 20 technical-standard references, each with an on-page anchor, covering EN 81-20/50:2020, UK Lifts Regulations 2016, BS 9991:2024, ISO 8100-1:2019, LOLER, PUWER, EAWR, RIDDOR, BS 7255:2023, EN 13015, CDM 2015, CAR 2012, SAFed LG1, HSW Act 1974, UK GDPR/DPA 2018, Equality Act 2010, Approved Document M (2024), FOI Act 2000 and Workplace (H&S&W) Regs.
9. Generated 9 product-requirement seeds keyed to the lot: core service, traction parts, hydraulic parts, service/platform/access-lift servicing, LOLER/SAFed examination, 24/7 call-out, CAFM integration, minor refurbishment works, and mobilisation package. Each seed carries scoped CPV suggestions and a verbatim evidence anchor.
10. Set evaluation_criteria to an empty array and recorded `NO_EXPLICIT_AWARD_CRITERIA_IN_SPEC`: the specification describes KPIs (performance monitoring) but not award criteria/weightings, which would be expected to live in a separate ITT/questionnaire within the tender pack.
11. Decided CPV inheritance policy: since the specification text itself cites no CPV codes, inherited `50750000-7` (Lift-maintenance services, most specific match) and `50700000-2` (Repair and maintenance of building installations) at the lot level, and assigned seed-scoped CPVs (e.g., `71630000-3` for statutory inspection, `72000000-5` for the CAFM integration seed, `45313100-5` for refurbishment works). Logged `CPV_INHERITED_FROM_NOTICE`.
12. Scored field confidences: high (9200–9800) for title/lot_number/description/requirements/standards, moderate (7500) for CPV codes (inherited, not cited), low (4000) for evaluation_criteria (absent from document).
13. Wrote `result.json` conforming to `result.schema.json` and validated it against the schema with Python `jsonschema`.

## Decisions Made

- **Single-lot modelling**: Emitted exactly one entry in `lots` (lot_number `"7"`) rather than synthesising lots 1–11. The notice title ("Building Services Framework - Lots 1-11") refers to the overall process; this extraction payload contains only the Lot 7 specification text. Lot numbering preserved exactly as shown in the document title so that chunk-merge or re-run operations land on a stable key.
- **Evidence anchoring by section, not page**: page_ref is `"n/a"` throughout because the source is a DOCX and `page_count=0`. Section headings are used as `section_ref`, and verbatim quotes as `anchor_text`. Flagged via `NO_PAGE_REFS_AVAILABLE` and `SECTION_NUMBERING_COLLAPSED` warnings (Docling rendered every top-level heading as `1.` because each is an individual ordered-list item in the source).
- **CPV inheritance**: CPVs emitted are inherited from the notice rather than quoted from the document; documented via `CPV_INHERITED_FROM_NOTICE`. Seed-level CPVs were additionally scoped where the candidate clearly deviates from pure lift-maintenance (e.g., IT platform → 72000000-5, installation works → 45313100-5, technical inspection → 71630000-3).
- **No award criteria emitted**: `evaluation_criteria: []` because nothing in the specification text states award weightings. Confidence on that field dropped to 4000 bps and a `NO_EXPLICIT_AWARD_CRITERIA_IN_SPEC` warning was emitted so downstream quoteability scoring does not treat absence as implicit unweighted criteria.
- **Seed keys use `lot7-` prefix** to remain stable across resumes/chunk-merges per SKILL.md resume-strategy guidance.
- **Quantity hints**: Included "24 lift assets across 12 sites (subject to change)" on the core service seed with a `QUANTITY_MUTABLE` warning so downstream pricing treats the quantity as a range that can expand via call-off.
- **Draft-document disclosure**: Document title contains "Draft specification"; emitted `DRAFT_DOCUMENT` warning so validators and downstream consumers know final-issue amendments may exist.

## Warnings

1. `NO_EXPLICIT_AWARD_CRITERIA_IN_SPEC` — specification covers KPIs only, not award evaluation criteria.
2. `CPV_INHERITED_FROM_NOTICE` — CPV codes sourced from notice metadata, not the spec text.
3. `DRAFT_DOCUMENT` — title flagged as DRAFT; amendments possible in the issued version.
4. `NO_PAGE_REFS_AVAILABLE` — DOCX extraction, `page_count=0`; section-based anchoring only.
5. `SINGLE_LOT_DOCUMENT` — covers Lot 7 only; lots 1-6 and 8-11 of framework 6278 are separate specs.
6. `SECTION_NUMBERING_COLLAPSED` — Docling rendered every top-level heading as `1.`; anchors rely on heading text.
7. `QUANTITY_MUTABLE` — lift count explicitly "subject to change" and call-off scope is open.
8. `OPTIONAL_STANDARDS_WATCH` — supplier must track ISO 8100-1/-2 convergence; standards set may expand mid-contract.

## Evidence

All major claims are anchored to sections present in the extracted markdown payload (agent-claimed anchors; daemon may verify against the stored `extracted_markdown` whose `source_content_hash` is `436bc238fb5ea4004e5a52610e4a01a6915a1b763cca45f7a09024927b67f791`):

- Lot identity — document title line: `London Borough of Bexley / Lift Specification / Contract Commencement Sept 2026` (Docling heading region). Title field `6278 - Draft specification - Lot 7 - Provision of Lift Services - Corporate Buildings` from `document_extractions.document_title`.
- Scope and asset count — section "Introduction and Context": *"There are currently 24 lift assets across 12 sites (subject to change)"*.
- Standards baseline — section "Introduction and Context": *"All new or substantially modified equipment must comply with EN 81-20:2020 and EN 81-50:2020 (or UK-designated equivalents), and the Lifts Regulations 2016 (UK)"*.
- Term — section "Introduction and Context": *"The initial contract term is 3+2 years, subject to performance"*.
- Service-hour SLAs — section "Service Hours and Priorities": *"Operate 24/7/365. Minimum attendance times: Entrapment/Life Safety: attend within 1 hour (target); Operational Failure: within 4 hours; Nuisance/Minor Fault: next working day"*.
- LOLER obligations — section "Examination & Testing of Lifts (LOLER & SAFed)": *"All lifts used by people at work require LOLER thorough examination by a competent person at statutory intervals (typically 6-monthly for passenger/goods-passenger lifts; 12-monthly for goods-only)"*.
- Traction parts scope — section "Traction Lifts - Inclusive Parts": *"Inclusive consumables/components typical to traction systems (e.g., oils/greases, fuses, brushes, standard lamps, selected contacts/switch gear)"*.
- Hydraulic parts scope — section "Hydraulic Lifts - Inclusive Parts": *"Inclusive items such as seals, packings, hoses, pumps, valves, and motors unless expressly excluded. Buried piping and direct-acting cylinders may be excluded where specified"*.
- Service/platform/access lifts — section "Routine Servicing - Service Lifts, Platforms, Access Lifts & Associated Equipment": *"Quarterly (or manufacturer-recommended) servicing… Where equipment carries persons (e.g., platform lifts), set LOLER thorough examination interval to 6 months"*.
- CAFM integration — section "Suppliers' System Capability Requirements": *"Supplier to operate a compatible work management platform with mobile workflows, photo evidence, asset updates and API/CSV exchange with Civica/TF Cloud"*.
- Consolidated legislation/standards list — section "Legislation & Standards" quoted verbatim in the technical_standards array.

Result contract emitted: `result.json` (validated against `result.schema.json`, includes `extraction_metadata.target_domain="lots"`, `extraction_scope="full_document"`, `source_markdown_hash=436bc238fb5ea4004e5a52610e4a01a6915a1b763cca45f7a09024927b67f791`, `policy_version="2026.04.12"`).
