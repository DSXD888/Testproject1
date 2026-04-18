---
manifest_version: "1.0.0"
skill_id: "pdf-extraction/tender-specification"
skill_version: "1.0.0"
job_id: "job-5fb901ec-17a1-42eb-a84f-5eabb2343bde"
session_id: "session-semantic-35c22d34e5863991843cfe38a046d3b8d105d6e59b3d6465d829a7a2c845ab81"
execution_mode: "contract"
started_at: "2026-04-18T09:15:00Z"
completed_at: "2026-04-18T09:16:30Z"
outcome_class: "completed_with_warnings"
items_processed: 1
items_skipped: 0
items_failed: 0
warnings_count: 8
agent_claimed_pages_visited: 0
agent_claimed_source_url: "state.db://document_extractions/57660f2a-3eb3-4637-afe0-865023825801"
agent_claimed_fingerprint_state: "not_applicable_headless_contract"
agent_claimed_locale_state: "en-GB"
agent_claimed_currency_state: "GBP"
---

# Summary

Ran `pdf-extraction/tender-specification` v1.0.0 in headless contract mode against the Docling markdown stored on `document_extractions.id = 57660f2a-3eb3-4637-afe0-865023825801` (title "1A-Invitation-to-Participate"), for process `proc-pc-f0f30f94-e22c-f111-813c-005056b64545` / notice `pc-f0f30f94-e22c-f111-813c-005056b64545` (Stockport Town Centre School — Multi-disciplinary Design Team Appointment; buyer Stockport Metropolitan Borough Council via STAR Procurement; STAR Ref 020417 / Chest DN811263).

Extracted one authoritative lot (`lot_number = "1"`) covering the entire Contract, with zero `product_requirement_seeds` (professional services, no goods) and zero `technical_standards` (no product or compliance standards cited in this ITP). The document role recorded in `document_extractions` is `technical_specification`, but the actual content is an ITP (Invitation to Participate instructions, timetable, and scoring methodology) that cross-references the authoritative specification to Documents 2 and 7 — this mismatch is surfaced as a warning. `extraction_confidence_bps = 8400`.

Result written to `result.json`; validated against `skills/pdf-extraction/tender-specification/result.schema.json` and passed.

# Steps Taken

1. Resolved the job request (`document_extraction_id`, `target_domain = "lots"`, `extraction_scope = "full_document"`) and located the underlying Docling markdown via `state.db → document_extractions` (`raw-pc-f0f30f94-...`, `source_content_hash 0927b761...`).
2. Read the Docling markdown (428 lines, ~56.6 KB) and classified document type: Document 1A ITP for a services procurement under the Procurement Act 2023 Competitive Flexible Procedure.
3. Checked for explicit lot split — the ITP names a single "Contract" for the Services (design team appointment); no sub-lots are described. Cross-checked `notice_lots` in `state.db`: one lot already present (`pc-...-lot-1`) consistent with this finding.
4. Extracted evaluation criteria from the two scoring tables in §1.6 (PSQ Stage 1 weighting: Q19 35% / Q20 35% / Q21 30%) and §1.7 (Tender Stage 2 indicative: Quality 60% broken into four questions at 10% each + two 10% Social Value rows + Price 40%), plus the seven-point scoring methodology table.
5. Extracted material contractual terms: 48-month term; £1,800,000 max ex-VAT; RPI/CPI indexation capped at 3% from end of Year 3; 270-day bid validity; The Chest submission channel; Microsoft Word/Excel format; CDP registration; compliance with Document 4 T&Cs; optional Parent Company Guarantee per Document 8.
6. Checked for product requirements, technical standards, or CPV codes in the document — none found. Emitted empty arrays and raised explanatory warnings.
7. Computed `evidence_anchor` records using section-heading slugs because `page_count = 0` (no numeric page anchors available from Docling).
8. Composed `result.json` conforming to `result.schema.json` and validated it with jsonschema (passed).
9. Composed this run manifest.

# Decisions Made

- **Single lot emission.** Decided to emit one lot mirroring the whole Contract because the ITP explicitly describes a single "Contract" / "the Services" with no discipline split, stage split, or geographic split. This is consistent with the pre-existing single `notice_lots` row for this notice; the skill's "cross-reference only" rule was honoured — the decision is grounded in the tender document text, not the pre-existing lot row.
- **Empty `product_requirement_seeds`.** Professional services contracts (design team consultancy) produce no goods and no product candidates. Emitting speculative seeds would violate the skill's evidence rule. Warning `SERVICES_CONTRACT_NO_PRODUCTS` documents this.
- **Empty `technical_standards`.** The ITP cites no technical product/material standards (e.g., no BS/EN/ISO/RIBA/CIBSE references in this document). The actual technical/service specification is cross-referenced to Document 2 and Document 7 which are not included in this extraction packet.
- **Empty `cpv_codes`.** No CPV codes appear in the document body. Warning `NO_CPV_IN_DOCUMENT` directs downstream consumers to notice-level metadata (`main_category = 'Consultancy'`) rather than fabricating a code.
- **Document role mismatch surfaced, not silently corrected.** `document_extractions.document_role = 'technical_specification'` disagrees with the content of Document 1A. Raised `DOCUMENT_ROLE_MISMATCH_POSSIBLE` warning rather than mutating or overriding the persisted role.
- **Section slugs instead of page numbers.** Because `page_count = 0` on this extraction, numeric page anchors are unavailable. Used stable heading-text slugs (e.g., `section-1.6-stage-1-weighting-table`) so evidence anchors remain resolvable.
- **Stage 2 criteria flagged as indicative.** §1.7 is explicitly labelled "indicative only … subject to change prior to ITT". Included in `evaluation_criteria` but with a dedicated warning so downstream consumers do not treat them as final award weights.
- **Confidence score 8400 bps.** Lots/evaluation/contractual facts are high-confidence (direct quotes from structured tables). Downgraded overall from ~95% because (a) role mismatch is an interpretive red flag, (b) service-scope detail is external to this document, and (c) Stage 2 weights are non-authoritative.

# Warnings

1. `NO_LOT_SPLIT_DETECTED` — Single lot emitted; consistent with existing notice-level lot.
2. `SERVICES_CONTRACT_NO_PRODUCTS` — No products; empty seed/product-requirement arrays by design.
3. `NO_CPV_IN_DOCUMENT` — No CPV codes in the document body.
4. `DOCUMENT_ROLE_MISMATCH_POSSIBLE` — Stored role `technical_specification` vs actual ITP instructions content.
5. `SPECIFICATION_DETAIL_EXTERNAL` — Authoritative spec content lives in Documents 2 and 7, not included here.
6. `STAGE_2_CRITERIA_INDICATIVE` — §1.7 criteria are non-authoritative until ITT issue.
7. `PAGE_REF_FALLBACK_TO_SECTION` — Numeric page anchors unavailable (`page_count = 0`); used section slugs.
8. `SECTION_NUMBERING_IRREGULAR` — Source auto-numbering contains duplicated / anomalous labels (e.g., multiple "1.1.1." and "1.6.1.1." lines, a "0.1." label); used heading text rather than literal numbering in anchors.

# Evidence

All lot-level claims, evaluation criteria, contractual terms, and warnings are anchored to section references drawn from the Docling markdown carried on `document_extractions.extracted_markdown` for `id = 57660f2a-3eb3-4637-afe0-865023825801`:

- **Lot identity and scope** — `section-header-document-1a`: "DOCUMENT 1A: INVITATION TO PARTICIPATE ('ITP') INSTRUCTIONS — Stockport Council — Stockport Town Centre School – Multi – disciplinary Design Team Appointment (the Services) — STAR Refence: 020417 — Chest Reference: DN811263".
- **Procurement procedure** — `section-1.1-competitive-flexible-procedure-table`: four-stage Competitive Flexible Procedure (PSQ / ITT / Due Diligence / Award).
- **Agreement term** — Text "The Contract will be for a term of 48 months (the 'Contract Period')." under section "Agreement Term" (§1.0.1).
- **Contract value and indexation** — §1.3.1: "maximum estimated value of the Contract over its maximum possible term is £1,800,000.00 (exclusive of VAT) … Prices shall be set for a fixed period of 4 contract years, after which prices may increase in each subsequent contract year by RPI/CPI indexation … lower of 3% and the Index … first increase shall take effect as from the expiry of the third Contract Year."
- **Timetable** — §1.4 table: Issue of ITP 31 March '26 → Application Response 27 April '26 12:00 → ITT Issue 18 May '26 → Award W/C 13 July '26 → Contract Start W/C 24 August '26.
- **Bid validity** — 270-day validity clause in §1.4 ("Validity of bid" paragraph).
- **PSQ evaluation criteria (Stage 1)** — §1.6 weighting table: Q19 35% / Q20 35% / Q21 30% = 100%.
- **Scoring methodology** — §1.6 scoring table: Unacceptable (0), Poor (1), Deficient (2) all trigger exclusion; Satisfactory (3) through Outstanding (6).
- **Shortlisting rule** — "The Council intends to shortlist the top 3 highest scoring Suppliers to be invited to participate in Stage 2."
- **Stage 2 criteria (indicative)** — §1.7 table: Quality 60% (4×10% quality questions + 2×10% Social Value) + Price 40% = 100%.
- **Submission channel** — "The Chest" paragraph and QR-code exclusion clause.
- **Council initiatives** — Section 3: Social Value (National TOMs / SVP for contracts >£50,000), Modern Slavery and Responsible Procurement, Real Living Wage, Priority Account Service.
- **TUPE position** — "It the Council's view that TUPE is unlikely to apply to this procurement exercise in the event of this agreement being awarded to persons other than those currently providing all or part of the services."
- **Related/external spec documents** — ITP document list at top: Document 2 (Specification), Document 6 (Priority Cohort and Themes), Document 7 (Stockport Town Centre School Brief), plus appendices — all cross-referenced but not included in this extraction.

Absence evidence (used to justify empty arrays):

- No occurrence of "CPV" in the Docling markdown for this extraction → `cpv_codes = []`.
- No occurrence of "BS ", "EN ", "ISO ", "RIBA", "CIBSE", "NBS", "Part L", "Part M", or other standards-reference patterns → `technical_standards = []`.
- No occurrence of quantities, specifications of goods, bill-of-materials items, or product identifiers → `product_requirements = []` and `product_requirement_seeds = []`.
