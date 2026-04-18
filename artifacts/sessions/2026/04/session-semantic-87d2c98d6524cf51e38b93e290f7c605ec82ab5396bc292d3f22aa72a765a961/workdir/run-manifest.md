---
manifest_version: 1
skill_id: pdf-extraction/tender-specification
skill_version: 1.0.0
job_id: job-f124349b-ec36-4b7a-82af-aba264f0716b
session_id: session-semantic-87d2c98d6524cf51e38b93e290f7c605ec82ab5396bc292d3f22aa72a765a961
execution_mode: contract
started_at: 2026-04-18T00:00:00Z
completed_at: 2026-04-18T00:00:00Z
outcome_class: ok
items_processed: 1
items_skipped: 0
items_failed: 0
warnings_count: 5
agent_claimed_pages_visited: 11
agent_claimed_source_url: document_extractions[dbbd1cd8-a9d8-4ba3-a8e5-8fedf5949f0d].extracted_markdown
agent_claimed_fingerprint_state: n/a_headless_contract
agent_claimed_locale_state: en-GB
agent_claimed_currency_state: GBP
---

## Summary

Extracted the `lots` target-domain payload from a full-document Docling markdown rendering of the Stockport Town Centre School — Multi-disciplinary Design Team Appointment briefing (process `proc-pc-f0f30f94-e22c-f111-813c-005056b64545`, notice `pc-f0f30f94-e22c-f111-813c-005056b64545`). The document is a single-commission design-services tender and does not declare numbered lots, so a single lot `1` was emitted covering the whole appointment, with requirements, Stage 1 PSQ criteria, Stage 2 award mechanics, and the three substantive technical-standard references that are anchored in the text (RIBA Stage 2, CDM Principal Designer role, 20% Biodiversity Net Gain). `product_requirement_seeds` is empty by design: this is a professional-services procurement with no discrete goods. Five warnings were emitted to pin provenance around the single-lot shape, the absent CPV codes in the document, the empty product seeds, an inconsistency in the Stage 2 weighting arithmetic (60 + 40 + 20 = 120), and OCR-style noise in the programme section.

## Steps Taken

1. Loaded Docling markdown (11 pages, 43 sections, 0 tables) for `document_extraction_id = dbbd1cd8-a9d8-4ba3-a8e5-8fedf5949f0d` and verified `source_content_hash = f880d06fce4ec2121773069cad36e628fdc1a809ed308e0baf66be1ab5a957f9`, `extraction_engine = docling@2.88.0`, `extraction_policy_version = 2026.04.12`, `document_role = technical_specification`, and `expected_overlays = ["pdf-extraction/tender-specification"]`.
2. Read joined process/notice context (`procurement_processes` + `contract_notices`) to capture buyer (`Stockport Metropolitan Borough Council`), Procontract category (`150000 - Consultancy`), and the notice description listing required disciplines (architect with educational-project experience, MEP, Principal Designer (CDM & BR), Structural Engineer).
3. Full-document scan for lot markers: no numbered-lot headings, lot tables, or `Lot N` references were found. Confirmed single-commission shape from the title page ("Briefing document for the appointment of Design Team") and the Workforce & Resources PSQ question referencing one multidisciplinary team.
4. Extracted Stage 1 PSQ question set from "Evaluation Criteria : Competitive Flexible Procedure" (p.10): three equally weighted questions — Relevant experience and contract examples, Workforce and Resources, Funding Model Approach — and captured the shortlist rule (top 3 to Stage 2) and the Stage 2 weighting statement verbatim.
5. Compiled lot requirements from the "Our Ambition", "The Vision", "Constraints: Opportunities & Challenges", "Skills & Expertise", and "Programme" sections, plus the notice-description disciplines as a cross-reference.
6. Identified technical standards with direct evidence: RIBA Plan of Work (Stage 2, anchored to programme table), CDM Principal Designer (anchored to "(Architecture, M&E, Structures & Civils, BR PD & CDM PD)"), and Biodiversity Net Gain 20% (anchored to constraints section). Did not promote vague text such as "green building standards" to a standards reference.
7. Wrote `result.json` in the session workdir and validated it locally against `skills/pdf-extraction/tender-specification/result.schema.json` (schema OK).
8. Assembled evidence-anchor list and field-level confidences, skewed downward for `cpv_codes`, `product_requirements`, and the `stage_2_award_weighting` field where the document itself is silent or inconsistent.

## Decisions Made

- **Single lot, not per-discipline lots.** The briefing treats the design team as one commission: one team, one contract, one progression through RIBA stages with termination options per stage. Splitting into sub-lots per discipline would be fabrication. Emitted lot `1` with `NO_LOT_SPLIT_IN_DOCUMENT` warning.
- **No CPV import from the Procontract category.** `150000 - Consultancy` is a portal taxonomy entry, not a CPV code, and the document itself carries no CPV citations. Left `cpv_codes` empty and warned.
- **No product seeds.** As a professional-services procurement there are no physical product candidates to seed. `product_requirements` and `product_requirement_seeds` intentionally empty; `NO_PRODUCT_CANDIDATES` warning emitted so downstream product-matching stages can gate this process off.
- **Stage 2 weightings captured verbatim, not reconciled.** Document states quality 60% / price 40% / social value 20% (sum = 120%). Emitted a dedicated warning (`STAGE2_WEIGHTING_SUM_INCONSISTENT`) rather than silently normalising — the reconciliation must come from the full Stage 2 tender pack, not this briefing.
- **Programme data captured to milestone granularity only.** OCR artefacts in the programme row (stray glyphs, fragmented calendar strip) mean finer durations are unreliable. Kept the three legible dates (17/11/2025 PID, 12/01/2026 brief finalisation, 31/08/2026 Stage 2 Cost Plan) and warned on the rest.
- **Confidence = 7400 bps overall.** Lot shape, evaluation criteria, and requirements are high-confidence; `cpv_codes`, `product_requirements`, and the Stage 2 weighting are dragged down by document silence / inconsistency.

## Warnings

1. `NO_LOT_SPLIT_IN_DOCUMENT` — single-lot shape; document declares no numbered lots.
2. `NO_CPV_CODES_IN_DOCUMENT` — CPV absent from the briefing; Procontract category is not a CPV code.
3. `NO_PRODUCT_CANDIDATES` — services tender; no physical product seeds exist.
4. `STAGE2_WEIGHTING_SUM_INCONSISTENT` — quality 60 + price 40 + social value 20 = 120 as written on p.10.
5. `PROGRAMME_OCR_NOISE` — programme section on p.11 contains stray glyphs and a fragmented calendar strip; milestones recovered, sub-durations approximate.

## Evidence

- **Source-of-truth markdown**: row `document_extractions.id = dbbd1cd8-a9d8-4ba3-a8e5-8fedf5949f0d`, `source_content_hash = f880d06fce4ec2121773069cad36e628fdc1a809ed308e0baf66be1ab5a957f9`, 11 pages, 43 sections, 0 tables, `document_role = technical_specification`, `expected_overlays = ["pdf-extraction/tender-specification"]`.
- **Process / notice provenance**: `procurement_processes.title` = "020417 - Stockport Town Centre School - Multi-disciplinary Design Team Appointment"; `contract_notices.buyer_name` = "Stockport Metropolitan Borough Council"; `contract_notices.main_category` = "150000 - Consultancy"; notice description names required disciplines.
- **Lot anchor** (p.1, `STOCKPORT TOWN CENTRE SCHOOL`): "Briefing document for the appointment of Design Team".
- **Skills & disciplines anchor** (p.8, `Skills & Expertise`): "The selected consultants should demonstrate that they have experience in developing through schools of a similar size for constrained, town and city centre locations."
- **Constraint anchor for BNG** (p.7, `CONSTRAINTS: OPPORTUNITES & CHALLENGES`): "Biodiversity Net Gain targets of 20%".
- **Stage 1 PSQ three-question anchor** (p.10, `Evaluation Criteria : Competitive Flexible Procedure`): "The Stage 1 Procurement Specific Questionnaire (PSQ) will have three equally weighted questions".
- **Workforce discipline anchor** (p.10): "(Architecture, M&E, Structures & Civils, BR PD & CDM PD)" — supports the CDM Principal Designer standard reference.
- **Shortlist anchor** (p.10): "The Council intends to shortlist the top 3 highest scoring Suppliers to be invited to participate in Stage 2."
- **Stage 2 weighting anchor** (p.10): "The Stage 2 Tender will be evaluated on the basis of quality (60%) price (40%) and social value (20%)." — supports `STAGE2_WEIGHTING_SUM_INCONSISTENT` warning.
- **RIBA standard anchor** (p.11, `Programme`): "RIBA Stage 2 Programme Stage 2 Cost Plan 15d 31/08/2026".
- **Result-contract fields** (see `result.json` in the session workdir): `extraction_metadata.target_domain = "lots"`, `extraction_metadata.extraction_scope = "full_document"`, `extraction_metadata.policy_version = "2026.04.12"`, `extraction_confidence_bps = 7400`, `warnings_count = 5`.
