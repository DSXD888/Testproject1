---
manifest_version: 1
skill_id: pdf-extraction/tender-specification
skill_version: 1.0.0
job_id: job-f2364536-6e3a-40c1-830f-95deb46a6114
session_id: session-semantic-056aef98bfa524a7eeb1135218e49025c2728233d9d5f15d3d3792f6ecc3fbbe
execution_mode: contract
started_at: 2026-04-18T00:00:00Z
completed_at: 2026-04-18T00:00:00Z
outcome_class: completed_with_warnings
items_processed: 1
items_skipped: 0
items_failed: 0
warnings_count: 7
agent_claimed_pages_visited: 0
agent_claimed_source_url: "file:///Users/alinaaitova/Desktop/Testproject1/artifacts/downloads/procontract/afb3b5dc-772e-f111-813c-005056b64545/6278 - Draft Tender Documents (FOR INFORMATION ONLY)_extracted/Draft Tender Documents (FOR INFORMATION ONLY)/6278 - Draft T&CS - Lot 2 - Provision of Water Risk Assessments.doc"
agent_claimed_fingerprint_state: not_applicable
agent_claimed_locale_state: en-GB
agent_claimed_currency_state: GBP
---

# Run Manifest — pdf-extraction/tender-specification — job-f2364536

## Summary

Ran the `pdf-extraction/tender-specification` overlay against Docling markdown of the **draft Terms & Conditions** document for **Lot 2 — Provision of Water Risk Assessments** under the London Borough of Bexley Building Services Framework **6278**. The document resolved by the daemon as `document_role = technical_specification` is in substance a contractual T&Cs template: Part 1 (Form of Contract), Part 2 (General Terms and Conditions A-I), Part 3 (Special Conditions incl. TUPE S1), and four Schedule headings. Critically, the **Schedule 1 SPECIFICATION body is absent** — only the heading appears in the supplied markdown (lines 1515-1517), and Schedule 2 (Pricing Schedule), Schedule 3 (ITT) and Schedule 4 (Contractor's Tender) are likewise placeholder headings.

Because of this, the extraction emits **one lot row** (Lot 2), a **single service-level product requirement seed**, and a comprehensive set of **contractual requirements** that are visible in the T&Cs body. It emits **empty** evaluation_criteria, cpv_codes and technical_standards for the lot, each backed by an explicit warning. Overall `extraction_confidence_bps = 4200` (42%) reflects high confidence on lot identity/title but low confidence on technical-specification payload because the substantive specification is not in this document. Evaluation criteria, CPV codes, HSG274/ACoP L8/BS 8580/BS 7592 standards and per-building frequencies must be recovered from the standalone Specification file (likely `6278 - Lot 2 - Service Specification.docx` or similar) when it is ingested separately.

## Steps Taken

1. **Located and read the Docling markdown.** Retrieved `extracted_markdown` for `document_extractions.id = d8b408a7-671e-46ce-b69a-b3d8d3f7c0a9` from `state/state.db` (136,112 characters, 1,537 lines). Confirmed the authoritative `source_content_hash = cf5f21b8680d3abd9673431f623307df6d717a98d1289718eca3fa3d5b4a54ca` from the same row.
2. **Confirmed the process/notice context.** Read `contract_notices.id = pc-dc1daee2-772e-f111-813c-005056b64545` (`6278 - Building Services Framework - Lots 1-11`, buyer `London Borough of Bexley`, deadline `2026-05-18T12:00:00Z`, start date `01/09/2026`, end date `30/08/2030`). Read `notice_lots` for the notice and confirmed Lot 2 title `Provision of Water Risk Assessments (Legionella Survey)`.
3. **Resolved the physical document.** Confirmed `raw_documents.id = raw-pc-dc1daee2-772e-f111-813c-005056b64545-2fd504fd059ee4941979b523880f217b30c267cc498839d82d7e974ce48688a2` resolves to `6278 - Draft T&CS - Lot 2 - Provision of Water Risk Assessments.doc` (content_type `application/msword`), with storage path in the `afb3b5dc-772e-f111-813c-005056b64545` notice download.
4. **Structurally scanned the markdown.** Extracted the document-structure skeleton: Part 1 cover page and Recital A, the clause 1.2 enumeration of contract documents, Part 2 TOC (A-I and the list of clauses from A1 to I2), the body of Parts 2 and 3, Schedule 1-4 headings and the document footer `FM Water Risk Assessment Services T&Cs 6 August 2020`.
5. **Classified content type.** Determined the document is in substance a contractual T&Cs template scoped to Lot 2, not a substantive technical specification. Decision recorded below and warned on role mismatch.
6. **Searched for technical standards and CPV.** Ran targeted grep for `BS\s*\d`, `ISO\s*\d`, `HSG`, `L8`, `ACoP`, `legionella`, `BSRIA`, `HTM`, `SFG20`, `NEBOSH`, `IOSH`, `PAS`, `CPV`. No water-risk-specific standards and no CPV codes were found in the markdown body.
7. **Searched for evaluation-criteria signal.** Ran grep for `evaluation`, `award`, `scoring`, `weighting`, `criterion`, `criteria`, `Appendix E`, `KPI`, `Service Credit`. Found only contractual references deferring evaluation/scoring to `Appendix E to the Invitation to Tender` and to Schedule 1. No in-document criteria to extract.
8. **Composed the lot payload.** Produced one lot row (lot_number `2`) with title from Recital A and the lot subject line. Populated `requirements` with 13 evident contractual obligations keyed to specific clause IDs (B1.1, B2.1/B2.3/B2.6, B3.2, B4.9/B4.11-B4.13, B4.17, A7.1, D6.4/D6.7, E1.1, E1A.9/E1A.10, F5/F7, S1.6/S1.7, Recital A dates). Left `evaluation_criteria`, `cpv_codes`, `technical_standards` as empty arrays, each with an explicit warning.
9. **Composed a single product-requirement seed.** `seed_key = bexley-6278-lot2-water-risk-assessment-service`. The seed is the overarching Water Risk Assessment service for the Council estate; detailed technical attributes and volumes are flagged as held in Schedule 1 which is placeholder-only.
10. **Computed confidences and wrote 7 warnings.** `extraction_confidence_bps = 4200`. Per-field confidences are high on lot identity (lot_number 0.98, title 0.95) but low on evaluation/CPV/standards (0.05 each) — these are not "low confidence guesses" but "evidence absent" signals.
11. **Wrote `result.json`.** Validated against `result.schema.json` (Draft 2020-12): `jsonschema.validate` passed.

## Decisions Made

- **Lot scoping: one lot only (Lot 2).** The document is lot-specific (filename, cover page, Recital A all say Lot 2 — Water Risk Assessments). The parent Framework 6278 has 11 lots, but each other lot has its own document. Synthesising rows for Lots 1 and 3-11 from this file would be fabrication.
- **No fabrication of technical standards.** Water Risk Assessment programmes in UK public bodies commonly reference HSG274 Parts 1-3, ACoP L8, BS 8580, BS 7592 and HTM 04-01. None of these strings appear in the supplied markdown, so none were emitted. A warning explicitly names them as the kind of standards that would be expected in the Schedule 1 body when it is supplied.
- **CPV codes left empty at lot level, not propagated from the notice.** Per SKILL.md guidance that "Existing notice-level lot metadata for cross-reference only; do not treat pre-existing lot rows as authoritative truth if the tender document contradicts them", and because this document itself cites zero CPV codes, emitting notice-level CPVs into the lot row would misrepresent which facts came from the document. Warning emitted to direct downstream consumers to the Contract Notice/FTS record for CPV.
- **Evaluation criteria left empty.** The T&Cs document repeatedly defers evaluation to `Appendix E to the Invitation to Tender` (see Clauses F5.1 and F7.1) and to Schedule 1. No substantive criterion/weighting pair is statable from this document.
- **Document role mismatch surfaced, not silently corrected.** The daemon resolved this as `document_role = technical_specification`, but the supplied content is functionally `draft_tcs`. Rather than relabel the document locally, this was surfaced as warning `DOCUMENT_ROLE_MISMATCH_SUSPECTED` so daemon-side role classification and overlay selection can be audited.
- **Template contract dates called out, not overridden.** Recital A states "1 September 2020 to 31 August 2022" but the live notice advertises 01/09/2026 - 30/08/2030. This is a 2020 template re-used for the 2026 procurement (footer: `6 August 2020`). Warning emitted so downstream consumers do not treat Recital A as authoritative on contract dates.
- **One product requirement seed, not many.** Because the Schedule 1 technical body is absent, there is no basis for decomposing the service into multiple seeds (e.g. "legionella sampling", "written scheme update", "re-inspection"). Emitting a single service-level seed with the note that scope lives in Schedule 1 is more honest than synthesising sub-seeds.
- **`extraction_scope = full_document`.** Confirms single-pass run over the whole Docling markdown, matching the `extraction_scope` field in the job params.

## Warnings

| # | Code | Short description |
|---|------|-------------------|
| 1 | SPECIFICATION_BODY_PLACEHOLDER | Schedule 1 body is absent; detailed scope/standards/frequencies not extractable from this document. |
| 2 | NO_EVALUATION_CRITERIA_IN_DOCUMENT | Evaluation criteria deferred to Appendix E of the ITT; none statable here. |
| 3 | NO_CPV_CODES_IN_DOCUMENT | Zero CPV codes in the markdown body. |
| 4 | NO_TECHNICAL_STANDARDS_IN_DOCUMENT | No HSG274 / ACoP L8 / BS 8580 / BS 7592 / HTM 04-01 references in the markdown. |
| 5 | DOCUMENT_ROLE_MISMATCH_SUSPECTED | Resolved role `technical_specification` but content is draft T&Cs. |
| 6 | CONTRACT_DATES_ARE_TEMPLATE_PLACEHOLDERS | Recital A dates (2020-2022) are template; live notice covers 2026-2030. |
| 7 | SINGLE_LOT_SCOPE | Extraction covers Lot 2 only; Lots 1 and 3-11 have their own documents. |

No failure codes were triggered; outcome is `completed_with_warnings`. The result contract and the run manifest are both present and validate.

## Evidence

All major claims are anchored to observable structural features of the supplied Docling markdown. The primary anchors are:

- **Cover page and Recital A** (markdown lines 15-19 and line 43) — establish the document's explicit scope: "RELATING TO THE PROVISION OF WATER RISK ASSESSMENTS SERVICES" and "The Council wishes to secure the provision of Water Risk Assessments services in Council Buildings as defined in the attached Specification".
- **Part 1 Clause 1.2** (lines 51-67) — the four-document precedence list (Form of Contract, General T&Cs, Special Conditions, Schedule 1 Specification, Schedule 2 Pricing, Schedule 3 ITT, Schedule 4 Contractor's Tender). This is the anchor for "Schedule 1 holds the Specification".
- **Part 2 Clause B1.1** (line 535) — "The Contractor shall supply the Services during the Contract Period in accordance with the Council's requirements as set out in the Specification ...". Anchors the general "comply with Spec/Code of Conduct" requirement.
- **Part 2 Clause B3.2** (line 565) — anchors staff-qualification requirement and the fact that detailed qualifications live in the (placeholder) Schedule 1.
- **Part 2 Clauses B4.9-B4.13** (lines 591-599) — DBS check regime for all Staff engaged in the Services.
- **Part 2 Clauses F5.1-F5.4 and F7.1-F7.2** (lines 1167-1197) — KPI and Management Report regime, with evaluation/Service Credit detail deferred to Appendix E of the ITT.
- **Part 2 Clauses D6.4-D6.7** (lines 789-799) — health and safety compliance anchored to HSWA 1974 and to the Specification.
- **Part 2 Clauses E1.1, E1.15 and E1A.9-E1A.10** (lines 807, 917, 941-943) — Processor-role data protection obligations and anti-virus/malware cooperation.
- **Schedule 1 / 2 / 3 / 4 headings** (lines 1515-1529) — the four schedule headings with empty bodies. This is the direct evidence for `SPECIFICATION_BODY_PLACEHOLDER`, `NO_EVALUATION_CRITERIA_IN_DOCUMENT` and `NO_TECHNICAL_STANDARDS_IN_DOCUMENT`.
- **Document footer** (line 1535) — "FM Water Risk Assessment Services T&Cs 6 August 2020" — direct evidence that this is a 2020 template being reused for the 2026 procurement, anchoring `CONTRACT_DATES_ARE_TEMPLATE_PLACEHOLDERS`.
- **Targeted greps returning zero hits** on `BS\s*\d`, `ISO\s*\d`, `HSG`, `L8`, `ACoP`, `legionella`, `BSRIA`, `HTM`, `SFG20`, `NEBOSH`, `IOSH`, `PAS\s*\d`, and on `CPV` / `cpv` — negative evidence underpinning warnings 3 and 4.

No claim in `result.json` — lot identity, requirement, seed, warning, or metadata field — was made without a corresponding anchor in the supplied markdown or an explicit note that the evidence is absent.
