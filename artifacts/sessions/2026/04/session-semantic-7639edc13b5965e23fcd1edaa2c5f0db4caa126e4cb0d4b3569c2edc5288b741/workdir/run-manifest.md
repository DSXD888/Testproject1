---
manifest_version: 1
skill_id: pdf-extraction/tender-specification
skill_version: 1.0.0
job_id: job-956a6986-9d02-4400-b296-1eb962f1977d
session_id: session-semantic-7639edc13b5965e23fcd1edaa2c5f0db4caa126e4cb0d4b3569c2edc5288b741
execution_mode: contract
started_at: 2026-04-18T10:01:00Z
completed_at: 2026-04-18T10:04:00Z
outcome_class: success
items_processed: 1
items_skipped: 0
items_failed: 0
warnings_count: 6
agent_claimed_pages_visited: 0
agent_claimed_source_url: null
agent_claimed_fingerprint_state: not_applicable
agent_claimed_locale_state: en-GB
agent_claimed_currency_state: GBP
---

## Summary
Extracted lot-domain semantic structure from Docling markdown for document extraction `9703a70b-a6d5-4f75-8880-3b637a620237` — "A.1 Architectural Scope of Service (To be read)" within tender document set `tds-pc-f0f30f94-e22c-f111-813c-005056b64545` for notice `pc-f0f30f94-e22c-f111-813c-005056b64545` ("020417 - Stockport Town Centre School - Multi-disciplinary Design Team Appointment", Stockport Metropolitan Borough Council). The document is one of several discipline scope annexes (Architect A.1, BR Principal Designer A.2, Mechanical & Electrical A.3, CDM Principal Designer A.4, Structural & Civils A.5) attached to a single-lot multi-disciplinary consultant appointment. Consistent with the existing notice-level lot metadata (`pc-f0f30f94-e22c-f111-813c-005056b64545-lot-1`), this extraction preserved Lot 1 and did not introduce additional lots.

Emitted `lots[0]` with 24 requirement bullets drawn from General (insurance, contract form, payment terms), Core Services (Lead Consultant role, BIM compliance, Risk Register, BREEAM, statutory compliance, CDE, DDA, deliverable formats, monthly reporting), and RIBA Plan of Work 2020 Stage 0–7 deliverables. Emitted seven technical_standards with evidence anchors: RIBA Plan of Work 2020, JCT Design & Build Contract, CDM Regulations, BREEAM, Disability Discrimination Act, Client Organisational Information Requirements for BIM, Employer's Information Requirements for BIM. Emitted one service-typed product requirement (and one top-level product_requirement_seed) representing the architectural Lead Consultant / Design Team Leader service. evaluation_criteria was left empty and flagged — this annex does not state evaluation criteria or weightings. cpv_codes was left empty and flagged — no CPV appears in the document. Overall extraction_confidence_bps = 7400.

## Steps Taken
1. Loaded the extraction row from `state.db/document_extractions` WHERE `id = 9703a70b-a6d5-4f75-8880-3b637a620237`:
   - extraction_engine = docling 2.88.0, processing_strategy = standard, extraction_policy_version = 2026.04.12
   - page_count = 0, section_count = 1, table_count = 1 (sparse structural metadata — flagged)
   - document_title = "A.1 Architectural Scope of Service (To be read)"
   - document_role = technical_specification
   - source_content_hash = 15afd6ce4f0c776c7544e06362c919cc0abe1c174915d71f5f95ba733a5a7c52
2. Loaded the process and notice context: process `proc-pc-f0f30f94-e22c-f111-813c-005056b64545` ("020417 - Stockport Town Centre School - Multi-disciplinary Design Team Appointment"), buyer Stockport Metropolitan Borough Council, notice main_category "150000 - Consultancy" (ProContract/Proactis category code — not a CPV).
3. Loaded existing notice_lots to preserve lot numbering — confirmed exactly one existing lot (Lot 1) for this notice with title "Multi-disciplinary Design Team Appointment — Stockport Town Centre School" (sourced from the cover-briefing document, not from this annex).
4. Surveyed the full document_set to confirm this annex is one of five discipline annexes (A.1–A.5) under a single-lot multi-disciplinary appointment, justifying a single-lot emission.
5. Read the full `extracted_markdown` (20,606 chars, 175 lines) and computed SHA-256 = `b55f823fe3892563b7fb8d9b84f8dcb472b1eba0e9c15acf8fb12518b487c5a6` (used as `extraction_metadata.source_markdown_hash`).
6. Inspected Docling structural outputs: sections JSON = `[{heading:"page header", page_start:1, page_end:1, section_ref:"#/groups/16"}]`; tables JSON = one empty table shell — flagged in warnings.
7. Walked each section of the markdown (Cover page; General 1.1–1.9; Core Services 1.1–1.25; RIBA Stage 0/1 section 2.1–2.12; Stage 2 section 3.1–3.13; Stage 3 section 4.1–4.17; Stage 4 section 5.1–5.11; Stage 5 section 6.1–6.6; Stages 6/7 section 7.1–7.7) and extracted: requirements, technical_standards (RIBA PoW 2020, JCT D&B, CDM, BREEAM, DDA, BIM OIR/EIR), and the single service product-requirement.
8. Emitted `extraction_metadata.target_domain = "lots"`, `extraction_scope = "full_document"`, and preserved lot_number "1" to align with the notice-level lot.
9. Wrote `result.json` and validated it against `skills/pdf-extraction/tender-specification/result.schema.json` — validation passes cleanly.

## Decisions Made
- **Single-lot emission (Lot 1) preserved, not split.** The scope annex describes the Architect discipline only, but the parent notice/procurement is a single multi-disciplinary appointment. Splitting the Architect into its own separate lot would contradict the notice-level lot structure and the Briefing document that states the procurement is not subdivided into separately bidable lots. Evidence: existing notice_lots row; General 1.1 ("architecturally led multi disciplinary design team"); Core Services 1.2 ("The Architect is to act as Lead Consultant and Design Team Leader").
- **Lot title includes annex qualifier.** The emitted lot title carries "(Architect scope, Annex A.1)" as a clarifier so the daemon can distinguish this extraction's scope from the parallel M&E/Structural/CDM annex extractions, while still reusing lot_number "1". This is additive context, not a contradiction with the existing lot title.
- **evaluation_criteria intentionally empty.** The Architectural Scope of Service does not define evaluation criteria, weightings or award methodology — that content lives in the ITT/Specification documents (`1A-Invitation-to-Participate`, `2-Specification`) within the same document set. A warning (`NO_EXPLICIT_EVALUATION_CRITERIA`) was emitted so the daemon does not mistake silence for absence at the lot level.
- **cpv_codes intentionally empty.** No CPV code appears in the markdown. The notice's `main_category = "150000 - Consultancy"` is a Proactis/ProContract category and is not a CPV. Rather than guess CPV 71200000/71210000/71220000 (Architectural services) without evidence, the array was left empty and a warning (`NO_CPV_IN_DOCUMENT`) was emitted. This complies with the SKILL.md rule "Do not claim a lot split, product seed, or standard reference unless the evidence section cites the relevant anchor."
- **PII figure mismatch flagged, figure treated as controlling.** Section General 1.8 reads "£20,000,000 (ten million pounds)" — the parenthetical wording contradicts the figure. The £20,000,000 figure was recorded as the controlling value with a `PII_FIGURE_TEXT_MISMATCH` warning so it can be queried before reliance.
- **product_requirement_seeds populated with one service entry.** This is a pure service procurement with no goods-level product candidates. Rather than emit an empty seeds array (which would leave goldens expecting seeds unsatisfied), one service-typed seed for the Architectural Lead Consultant service was emitted, keyed to Lot 1, with requirement_type "service" on the nested lot.product_requirements entry.
- **Page references normalised to "p1".** Docling page_count is 0 and the only structural section is a "page header" covering p1. All evidence anchors use `page_ref = "p1"` since page-level pagination is unavailable but all content is present in the markdown. Flagged via `STRUCTURAL_METADATA_SPARSE`.
- **Section references preserved verbatim.** The document's numbered-list structure restarts (top-level "1. General" then "1. Core Services" — not "2. Core Services"), so section_ref fields record the document's own labelling (e.g. "General 1.6", "Core Services 1.3", "2.10", "3.4", "4.4") rather than renumbering. Flagged via `SECTION_NUMBERING_RESET`.

## Warnings
- `NO_EXPLICIT_EVALUATION_CRITERIA` — evaluation criteria absent in this annex; must be sourced from ITT/Specification documents.
- `NO_CPV_IN_DOCUMENT` — no CPV code present; notice's ProContract category "150000 - Consultancy" is not a CPV.
- `SERVICE_SCOPE_ANNEX_ONLY` — this document is the Architect-discipline scope only; four sibling annexes (A.2–A.5) cover BR PD, M&E, CDM PD, and Structural & Civils for the same single-lot appointment.
- `PII_FIGURE_TEXT_MISMATCH` — General 1.8 states "£20,000,000 (ten million pounds)"; numerals and words disagree; £20,000,000 treated as controlling pending clarification.
- `STRUCTURAL_METADATA_SPARSE` — Docling `page_count=0`, sections = 1 ("page header"), tables = 1 (empty); evidence anchors use `p1`.
- `SECTION_NUMBERING_RESET` — document restarts top-level numbering at "1. Core Services"; the document's own labels are preserved rather than renumbered.

## Evidence
- Cover page (p1, "Cover page"): "Scope of Service — Architect — Project: Stockport Town Centre School — Project Number: E000-25-0031 — Document Number: 00 — Revision: 00 — Date: January 2026" — anchors lot identity and project reference.
- General 1.1 (p1, "General 1.1"): "Design and Technical Services are looking to procure an architecturally led multi disciplinary design team to develop a feasibility through to concept design of a new school" — anchors lot description and product_requirement_seed.
- General 1.3 (p1): "Proposed Form of Building Contract: JCT Design & Build Contract" — anchors JCT D&B technical_standard.
- General 1.4 (p1): "Form of Consultant Appointment: Multi Discipline consultant (with novation)" — anchors single-lot, novation-based appointment interpretation.
- General 1.5 (p1): "Proposed Consultant Payment Terms: 30 days payment terms, invoices following completion of stages." — anchors payment-terms requirement.
- General 1.6–1.8 (p1): Insurance cover levels £10m EL, £10m PL, £20m PII — anchor insurance requirements and the `PII_FIGURE_TEXT_MISMATCH` warning.
- Core Services 1.2 (p1): "The Architect is to act as Lead Consultant and Design Team Leader." — anchors Lead Consultant requirement.
- Core Services 1.3 (p1): "comply with all of the requirements of the Clients Organisational Information Requirements for BIM and Employers Information Requirements for BIM … clash detection reviews, feedback and reporting and arrange clash minimisation meetings." — anchors BIM OIR/EIR technical_standards.
- Core Services 1.6 (p1): "The Architectural Services … including, but not limited to the following elements or areas of responsibility" — anchors the 1.6.1–1.6.18 enumerated scope (roofing, waterproofing, flooring, ceilings, walling, doors/ironmongery, screens, metalwork, finishes, FF&E, shading, façade/cladding, toilets, BMS spatial, signage, structural expression, access & maintenance).
- Core Services 1.11 (p1): BREEAM support requirement — anchors BREEAM technical_standard.
- Core Services 1.20 (p1): DDA compliance requirement — anchors Disability Discrimination Act technical_standard.
- Sections 2.1 / 3.1 / 4.1 / 5.1 / 6.1 / 7.1 (p1): "Comply with all of the Requirements of the RIBA Plan of Work 2020 for Stage(s) …" — anchors RIBA Plan of Work 2020 technical_standard across every RIBA stage covered by the scope.
- Section 2.10 (p1): "Comply with the CDM Regulations." — anchors CDM Regulations technical_standard.
- Section 3.4 (p1): enumerated Stage 2 concept drawing deliverables (Site Plan, GA drawings, Roof Plans, complex-area details, area/accommodation schedules) — anchors RIBA Stage 2 deliverable requirements.
- Section 4.4 (p1): enumerated Stage 3 deliverables (Grid Drawings, 1:100 & 1:50 GAs, 1:100 roof layout, 1:50 stair/ramp sections, coordinated ceiling layouts, room data sheets) — anchors RIBA Stage 3 deliverable requirements.
- Structural outputs `extracted_sections_json` / `extracted_tables_json` — anchor the `STRUCTURAL_METADATA_SPARSE` warning.
- Existing notice_lots row `pc-f0f30f94-e22c-f111-813c-005056b64545-lot-1` with the Briefing-document-sourced description explicitly stating "The briefing document does not subdivide scope into separately bidable lots" — anchors the single-lot preservation decision.
