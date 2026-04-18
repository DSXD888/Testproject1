---
manifest_version: "1.0.0"
skill_id: "pdf-extraction/tender-specification"
skill_version: "1.0.0"
job_id: "job-455f70ee-c54a-43d2-9596-3f56578edec8"
session_id: "session-semantic-38f02e0506269b9e0670a0d535b12cd5398517b794686d37f589e2ea5d8c56fd"
execution_mode: "contract"
started_at: "2026-04-17T21:00:00Z"
completed_at: "2026-04-17T21:02:30Z"
outcome_class: "success"
items_processed: 1
items_skipped: 0
items_failed: 0
warnings_count: 5
agent_claimed_pages_visited: 11
agent_claimed_source_url: "https://procontract.due-north.com/RfxResponse/Index?rfxId=c3957abe-e22c-f111-813c-005056b64545"
agent_claimed_fingerprint_state: "not_applicable_contract_mode"
agent_claimed_locale_state: "en-GB"
agent_claimed_currency_state: "GBP"
---

# Run Manifest — pdf-extraction/tender-specification

## Summary

Executed the `pdf-extraction/tender-specification` skill (v1.0.0) in headless-contract mode against document_extraction `63a31fca-175b-4b6d-a07c-b4f13fe9c3d7` — the 11-page Docling-extracted markdown of `7-Stockport-Town-Centre-School-Brief.pdf`, the Design Team Briefing document bundled with ProContract notice `pc-f0f30f94-e22c-f111-813c-005056b64545` ("020417 - Stockport Town Centre School - Multi-disciplinary Design Team Appointment", Stockport Metropolitan Borough Council).

The brief is a single-lot procurement under the Competitive Flexible Procedure to appoint a multi-disciplinary Design Team (Architecture, M&E, Structures & Civils, Building Regulations Principal Designer, CDM Principal Designer) for a new town centre through school in Stockport. The extraction produced one authoritative `lots[]` entry (lot_number "1"), five `product_requirement_seeds[]` for the discipline-specific professional services, a structured `evaluation_criteria[]` covering the Stage 1 PSQ three equally-weighted questions, the Stage 2 quality/price/social value weighting, and six `technical_standards[]` anchored to the document (RIBA Stage 2, CDM 2015 Principal Designer, BR PD regime, Biodiversity Net Gain 20%, climate emergency / flood risk, and an unspecified green-building-standards placeholder). Overall `extraction_confidence_bps` is 8400. Five warnings are raised: CPV codes not stated in this document, unspecified green-building standard, Stage-2 weightings that sum to 120%, a cross-document lot-structure consistency note, and OCR fragmentation in the programme strip on p.11.

Result contract written to `result.json` in the session workdir and validated successfully against `skills/pdf-extraction/tender-specification/result.schema.json`.

## Steps Taken

1. **Inspected session workdir** (`ls`) and confirmed the job harness did not provide input files locally; expected input lives in the daemon state database.
2. **Resolved the bound raw document** by querying `state.db raw_documents` on the provided `raw_document_id`, yielding `storage_path = .../artifacts/downloads/procontract/783101/7-Stockport-Town-Centre-School-Brief.pdf` and `source_url = https://procontract.due-north.com/RfxResponse/Index?rfxId=c3957abe-e22c-f111-813c-005056b64545`.
3. **Loaded the Docling extraction** from `state.db document_extractions.id = 63a31fca-175b-4b6d-a07c-b4f13fe9c3d7` — `extraction_policy_version = 2026.04.12`, `document_role = technical_specification`, `page_count = 11`, `section_count = 43`, `source_content_hash = f880d06f...`, `extracted_markdown` length 14,413 chars.
4. **Read the full 313-line extracted markdown** via `sqlite3` → temp file; then streamed it from head to tail to build page-level evidence anchors.
5. **Enumerated the Docling sections** (`extracted_sections_json` → 43 heading entries) to cross-check page ranges against the markdown (e.g. Evaluation Criteria on p.10, Programme on p.11, Skills & Expertise on p.8, Constraints on p.7, Vision on p.6).
6. **Loaded `process_context`** from `contract_notices.id = pc-f0f30f94-e22c-f111-813c-005056b64545` — confirmed buyer ("Stockport Metropolitan Borough Council"), main_category ("150000 - Consultancy", a ProContract taxonomy code, not a CPV), keywords ("Multi-Disciplinary, Design Team"), and Stage 1 expression window (31/03/2026 10:00 → 27/04/2026 12:00).
7. **Consulted existing notice-level lot metadata** (`notice_lots` WHERE notice_id = …) for cross-reference only — found a single lot (lot_key "1") previously seeded from a different PSQ Stage 1 document extraction (`7cbb4200-8926-4850-b2ba-24d04747442b`). Per SKILL.md guidance, did not treat this as authoritative but verified it is consistent with the brief.
8. **Compared the prior semantic extraction shape** (`document_semantic_extractions` with skill_id = `pdf-extraction/tender-specification`, e.g. `semantic-83f67bdb-…`) to confirm field conventions for `lots[]`, `evaluation_criteria[]` and `evidence_anchors`.
9. **Read `result.schema.json`** in full and confirmed the required field set including `extraction_metadata.target_domain = "lots"` (const) and the `evidenceAnchor` sub-schema requiring `page_ref` and `anchor_text`.
10. **Computed the markdown SHA-256** → `5094393366089cde676db466a8f85e6f6c05e9820d903412c785b56876180ff3` and used it for `extraction_metadata.source_markdown_hash`.
11. **Drafted the lot entry, requirements, evaluation criteria, product-requirement seeds and technical standards** from the brief content, citing page-level evidence anchors for every claim.
12. **Wrote `result.json`** in the session workdir and **validated it with `jsonschema`** against the skill's `result.schema.json` — validation passed, 1 lot / 5 seeds / 13 evidence anchors / 5 warnings / confidence_bps 8400.
13. **Wrote this run manifest** (`run-manifest.md`) with frontmatter + required narrative sections for daemon finalization.

## Decisions Made

- **Single lot (lot_number="1"), consistent with notice-level metadata.** The brief describes a single multi-disciplinary Design Team appointment for the Stockport Town Centre School. Stage 1 PSQ Question 5 ("Please confirm which lot(s) you wish to bid for?") is referenced elsewhere in the tender pack but the brief itself does not enumerate sub-lots. Treated the commission as one lot; captured the five disciplines (Architecture, M&E, Structures & Civils, BR PD, CDM PD) as `product_requirement_seeds[]` under that lot rather than splitting them into lots, because the procurement explicitly appoints them as one integrated team.
- **`cpv_codes = []` (empty) rather than inferred.** The brief does not state CPV codes. The contract notice's `categories` field is `"150000 - Consultancy"`, which is a ProContract internal taxonomy code (6-digit), not a CPV 2008 code (8-digit). Guessing values such as 71200000 / 71220000 / 71240000 (architectural / engineering design services) would be unanchored. Raised `CPV_CODES_NOT_STATED` warning instead.
- **Stage 2 weightings captured verbatim despite summing to 120%.** The document states "quality (60%) price (40%) and social value (20%)". Rather than silently renormalize, captured the weights as written and raised `STAGE2_CRITERIA_SUM_EXCEEDS_100` warning — likely social value is an overlay on top of 60/40, but that is a downstream policy decision, not an extraction decision.
- **Green building standards captured as unspecified placeholder.** The brief mentions "Green building standards" without naming BREEAM / Passivhaus / LETI / NZCB. Recorded as a `technical_standards[]` entry with a `GREEN_BUILDING_STANDARD_UNSPECIFIED` warning rather than fabricating a framework.
- **Biodiversity Net Gain 20% kept as-written.** The statutory England BNG minimum is 10% (Environment Act 2021 / Schedule 7A TCPA 1990). The brief's 20% target is a stronger local constraint, included verbatim in requirements and technical_standards.
- **RIBA Stage 2 captured as the primary programme standard.** Funding from GMCA has been secured only to deliver RIBA Stage 2, and the programme strip on p.11 is explicitly "RIBA Stage 2 Programme". Later RIBA stages are in-scope per the overall appointment with termination options, but not evidenced in this brief, so only Stage 2 was cited.
- **Programme OCR fragments flagged, not reconstructed.** Tokens "33₴", "Od", "STARI" and loose digit strings on p.11 were preserved in evidence anchors but not used as authoritative milestone claims; primary milestones (17/11/2025, 12/01/2026 – 26/01/2026, 31/08/2026) were cited from the legible calendar text via the Programme section.
- **`extraction_scope = "full_document"`.** Matches the job parameters and the fact that Docling produced one packet for all 11 pages.
- **Overall confidence 8400 bps.** Reflects strong structural extraction (headings, sections, evaluation criteria all clearly anchored) reduced for (a) unstated CPV codes, (b) OCR fragmentation in the programme strip, (c) non-specific sustainability standard, and (d) the Stage-2 weighting arithmetic anomaly.

## Warnings

1. **CPV_CODES_NOT_STATED** — Brief does not state CPV codes; notice `categories` is a ProContract taxonomy string, not a CPV. Left `cpv_codes` empty.
2. **GREEN_BUILDING_STANDARD_UNSPECIFIED** — "Green building standards" referenced without a named framework; captured as unspecified placeholder.
3. **STAGE2_CRITERIA_SUM_EXCEEDS_100** — Stage 2 weights (60 + 40 + 20) sum to 120; captured verbatim, downstream must not renormalize silently.
4. **LOT_STRUCTURE_CROSS_DOCUMENT** — Existing `notice_lots` row was seeded from a different source document (PSQ Stage 1 response template). This brief is consistent (single lot) but does not itself carry a lot enumeration, so treated as cross-document consistency, not authoritative.
5. **PROGRAMME_DATES_OCR_FRAGMENT** — p.11 programme strip contains OCR-mangled tokens; core milestones are legible, intermediate date labels are not fully reconstructable from the extracted markdown.

## Evidence

Every major claim is anchored to an observable element in the Docling-produced markdown:

- **Single-lot multi-disciplinary appointment** — p.8 *"Architecturally led consultant team will need to comprise a range of relevant disciplines and expertise"*, p.10 *"Please name each member of the proposed multidisciplinary team (company, practice etc) covering each of the roles outlined in the Stage 1 tender documents (Architecture, M&E, Structures & Civils, BR PD & CDM PD)."*
- **Stage 1 PSQ three equally weighted questions** — p.10 *"The Stage 1 Procurement Specific Questionnaire (PSQ) will have three equally weighted questions, as set out below"*.
- **Stage 1 Q1 – Relevant experience** — p.10 *"Describe your experience of designing and delivering school projects on complex, constrained and urban sites … Provide three case studies …"* (500-word narrative + 3×250-word case studies + single A4 sheets).
- **Stage 1 Q2 – Workforce and Resources** — p.10 Part 1 (500 words on team + 1000 words on alignment) and Part 2 (CVs, max 5 × single A4).
- **Stage 1 Q3 – Funding Model Approach** — p.10 *"Please describe your proposed approach to developing and supporting a viable funding model … (Maximum 500 words)"*.
- **Stage 2 evaluation weighting** — p.10 *"The Stage 2 Tender will be evaluated on the basis of quality (60%) price (40%) and social value (20%)."*
- **Top-3 shortlisting rule** — p.10 *"The Council intends to shortlist the top 3 highest scoring Suppliers to be invited to participate in Stage 2."*
- **RIBA Stage 2 programme** — p.11 *"RIBA Stage 2 Programme Stage 2 Cost Plan 15d 31/08/2026 … The adjacent programme indicates RIBA stage 2 design."*; programme milestones *"Submit PID 17/11/2025"*, *"Finalise Brief & prepare scope for … 12/01/2026 … 26/01/2026"*.
- **CDM / BR Principal Designer obligations** — p.10 discipline list: *"Architecture, M&E, Structures & Civils, BR PD & CDM PD"*.
- **Biodiversity Net Gain 20%** — p.7 Constraints: *"Biodiversity Net Gain targets of 20%"*.
- **Climate emergency / flood risk** — p.7 Constraints: *"Ensuring climate resilience, including in relation to flood risk / The climate emergency, declared in 2019"*.
- **Brownfield remediation complexity** — p.7 Constraints: *"Complexity of brownfield development including high remediation costs"*.
- **Green building standards ambition (unspecified)** — p.4 *"Sustainability & Future Proofing - Green building standards"*.
- **Client governance and point of contact** — p.9 *"SMBC Project Board - Estates, Education, MDC … For points of clarification please contact the Head of Capital Design & Technical Services."*
- **Through school scope** — p.3 / p.5 / p.6 repeated *"through school"* references; p.8 *"The selected consultants should demonstrate that they have experience in developing through schools of a similar size for constrained, town and city centre locations."*

Cross-source evidence (outside this document, used only as context and explicitly flagged): notice `normalized_payload.detail.categories = "150000 - Consultancy"`, `contact = "Sophie Lamb"`, `expressionWindow.from = "31/03/2026 10:00"`, `endDate = "31/08/2030"`, `startDate = "01/09/2026"` — none of these were carried into the `lots[]` claims except as corroboration for the single-lot decision.

Artifacts produced:
- `result.json` (schema-validated) in session workdir — the authoritative result contract.
- This run manifest — hybrid-authored narrative + frontmatter for daemon finalization into durable `run-manifest.md`.
