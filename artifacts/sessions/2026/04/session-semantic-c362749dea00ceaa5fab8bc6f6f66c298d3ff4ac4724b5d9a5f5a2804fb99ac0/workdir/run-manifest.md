---
manifest_version: 1
skill_id: pdf-extraction/tender-specification
skill_version: 1.0.0
job_id: job-33567b9e-273b-4865-af5d-9916727d845a
session_id: session-semantic-c362749dea00ceaa5fab8bc6f6f66c298d3ff4ac4724b5d9a5f5a2804fb99ac0
execution_mode: contract
started_at: 2026-04-18T09:55:00Z
completed_at: 2026-04-18T09:57:00Z
outcome_class: success
items_processed: 1
items_skipped: 0
items_failed: 0
warnings_count: 7
agent_claimed_pages_visited: 11
agent_claimed_source_url: null
agent_claimed_fingerprint_state: not_applicable_contract_mode
agent_claimed_locale_state: en-GB (document language not explicitly declared; document content is British English)
agent_claimed_currency_state: GBP (implied; no explicit currency symbols in the briefing text)
---

## Summary

Ran `pdf-extraction/tender-specification@1.0.0` in contract mode against Docling extraction `5703888e-5d3e-4cd0-a122-64751ce3ac4e` (11 pages, 43 sections, source-hash `f880d06f…a957f9`) for process `proc-pc-f0f30f94-e22c-f111-813c-005056b64545` — "020417 — Stockport Town Centre School — Multi-disciplinary Design Team Appointment" (buyer: Stockport Metropolitan Borough Council). Extraction scope is `full_document`.

The document is a short architectural briefing for the appointment of a single architecturally-led multi-disciplinary design team (Architecture, M&E, Structures & Civils, BR PD and CDM PD) under a Competitive Flexible Procedure. It does not subdivide scope into separately bidable lots. One synthetic lot (`lot_number = "1"`) was emitted. No CPV codes and no goods-style product requirement seeds are present in the document; `product_requirement_seeds` is therefore empty. Five technical-standard references with explicit evidence were emitted (Biodiversity Net Gain 20%, RIBA Stage 2, CDM PD, BR PD / Building Safety Act, 2019 Stockport climate emergency declaration). Five evaluation criteria were emitted: three Stage 1 PSQ questions, a shortlisting rule, and the Stage 2 weightings (quality 60% / price 40% / social value 20%, which sum to 120% as literally stated in the brief).

Overall `extraction_confidence_bps = 7700`. Confidence is reduced by: absence of CPV codes, the services-only nature of the brief (no product seeds), OCR ligature artefacts in the Docling output, noisy Gantt-style programme dates, and the 120% Stage 2 weighting anomaly.

## Steps Taken

1. Read the bound SKILL.md and confirmed the contract: emit exactly one `target_domain = "lots"` semantic extraction for the full document, with evidence-anchored lot(s), evaluation criteria, technical standards, product-requirement seeds (daemon-side translated), and warnings.
2. Loaded `result.schema.json` from the skill directory to confirm the required envelope and `$defs/evidenceAnchor` shape (required `page_ref`, `anchor_text`; optional `section_ref`).
3. Loaded the Docling-produced markdown from `document_extractions.id = 5703888e-5d3e-4cd0-a122-64751ce3ac4e` (14,413 chars, 11 pages, `document_role = technical_specification`, `source_content_hash = f880d06fce4ec2121773069cad36e628fdc1a809ed308e0baf66be1ab5a957f9`). Cross-referenced the per-page `extracted_sections_json` to derive page refs for anchors.
4. Cross-referenced pre-existing `notice_lots` rows for `pc-f0f30f94-e22c-f111-813c-005056b64545`. Confirmed that the daemon currently has a single lot row describing the multi-disciplinary design team appointment — consistent with this document. Used only as cross-reference; emission was driven by the document text, not by the pre-existing row.
5. Segmented the document into pages and canonical sections: Cover (p1), Contents (p2), Town Centre Regeneration (p3), Our Ambition / Five Big Things (p4), The Opportunity (p5), The Vision (p6), Constraints (p7), Skills & Expertise / Stockport Construction Hub (p8), The Client Team (p9), Evaluation Criteria (p10), Programme (p11).
6. Identified a single lot and synthesised its description, requirements, evaluation criteria, technical standards, and evidence anchor, anchoring each element to one or more observable lines from the markdown.
7. Identified services-only scope: there are no goods / equipment specifications, no BoQ items, no part-numbered components. Intentionally emitted an empty `product_requirement_seeds` array and a `SERVICES_ONLY_NO_PRODUCT_SEEDS` warning rather than inventing seeds.
8. Identified the 120% Stage-2 weighting anomaly (quality 60% + price 40% + social value 20%) and preserved it verbatim with a `STAGE2_WEIGHTINGS_SUM_120PCT` warning rather than silently re-scaling.
9. Tagged OCR / ligature artefacts (e.g. "benefft", "Opportunites", "difierence", "2th", "33₴", "STARI") and a fragmented Gantt-like programme table on p11 as low-confidence; retained as narrative context only.
10. Computed per-field confidences (see `field_confidences`) and an overall `extraction_confidence_bps = 7700`. Assembled the full evidence-anchor index (24 anchors) and emitted 7 warnings.
11. Wrote `result.json` to the session workdir. Validated against `result.schema.json` with `jsonschema`; schema check passed.
12. Authored this run manifest with required YAML frontmatter and narrative sections.

## Decisions Made

- **Single synthetic lot.** The brief describes one unified architecturally-led multi-disciplinary commission, not separately bidable lots. Emit `lot_number = "1"` rather than fabricating sub-lots per discipline. Warning `NO_EXPLICIT_LOT_SUBDIVISION` documents the decision and its evidence.
- **Empty `product_requirement_seeds`.** This is a professional-services appointment. No goods / quantities / part-number descriptions are present. Emit zero seeds plus warning `SERVICES_ONLY_NO_PRODUCT_SEEDS` rather than inventing pseudo-seeds.
- **Empty `cpv_codes`.** The document cites no CPV codes. Leave the array empty and emit `NO_CPV_IN_DOCUMENT`; CPV should be derived by the daemon from notice-level signals, not inferred here.
- **Technical standards whitelist.** Only explicitly named frameworks emitted as `technical_standards`: BNG 20%, RIBA Stage 2, CDM PD, BR PD (Building Safety Act), 2019 Stockport climate-emergency declaration. Generic phrases like "Green building standards" were intentionally excluded and flagged via `NO_NAMED_GREEN_BUILDING_STANDARD`.
- **Stage 2 weighting preserved verbatim.** The stated 60% / 40% / 20% = 120% was kept as-is; a warning records the arithmetic anomaly so the daemon / downstream consumers can surface or reconcile without the skill making an opinionated rescale.
- **Programme dates not structured.** OCR artefacts in the Gantt-like programme section (durations like `33₴`, truncated headers like `STARI`, interleaved strings) made confident structured milestone extraction infeasible. Dates retained inside the warning text only; not emitted as structured fields.
- **Evidence anchors.** Each lot field, each evaluation criterion, and each technical standard carries its own `evidence_anchor` with `page_ref` (e.g. `p10`) and a verbatim anchor string; the document-level `evidence_anchors` list gives 24 navigational entries across all 11 pages.

## Warnings

1. `NO_EXPLICIT_LOT_SUBDIVISION` — document is a single-lot services appointment; synthetic `lot_number = "1"` emitted.
2. `NO_CPV_IN_DOCUMENT` — no CPV codes cited anywhere in the brief.
3. `SERVICES_ONLY_NO_PRODUCT_SEEDS` — professional-services procurement; no goods-style product requirement seeds.
4. `NO_NAMED_GREEN_BUILDING_STANDARD` — "Green building standards" referenced without a named scheme (BREEAM / Passivhaus / NZEB / DfE output spec).
5. `STAGE2_WEIGHTINGS_SUM_120PCT` — stated Stage 2 weighting sums to 120%; preserved verbatim.
6. `OCR_QUALITY_NOISE` — Docling output contains ligature/OCR artefacts ("benefft", "Opportunites", "difierence", "2th", "33₴", "STARI", etc.).
7. `PROGRAMME_DATES_LOW_CONFIDENCE` — Gantt-like programme block on p11 retained as narrative context only; not emitted as structured milestones.

## Evidence

Every major claim is anchored to verbatim text from the Docling markdown. Key anchors (see `evidence_anchors` in `result.json` for the full list):

- **Cover / single appointment:** `p1` — "STOCKPORT TOWN CENTRE SCHOOL"; "Briefing document for the appointment of Design Team".
- **Multi-disciplinary scope:** `p10` — "Architecture, M&E, Structures & Civils, BR PD & CDM PD" (Workforce and Resources section).
- **Procurement route:** `p10` — "Evaluation Criteria :" / "Competitive Flexible Procedure"; "The Stage 1 Procurement Specific Questionnaire (PSQ) will have three equally weighted questions".
- **Stage 1 PSQ Q1 — Relevant experience:** `p10` — "Relevant experience and contract examples" … "Maximum of 500 words" … "three case studies … separate 250-word narrative … separate single A4 sheet".
- **Stage 1 PSQ Q2 — Workforce and Resources:** `p10` — "Please name each member of the proposed multidisciplinary team" … "Maximum of 500 words" / "Maximum of 1000 Words" … "Single A4 Sheet per CV maximum of 5 CVs".
- **Stage 1 PSQ Q3 — Funding Model Approach:** `p10` — "Funding Model Approach" … "(Maximum 500 words)".
- **Shortlisting rule:** `p10` — "The Council intends to shortlist the top 3 highest scoring Suppliers to be invited to participate in Stage 2."
- **Stage 2 weighting:** `p10` — "The Stage 2 Tender will be evaluated on the basis of quality (60%) price (40%) and social value (20%)." (verbatim; 120% sum).
- **Biodiversity Net Gain 20%:** `p7` — "Biodiversity Net Gain targets of 20%" (Constraints section).
- **Climate emergency reference:** `p7` — "The climate emergency, declared in 2019".
- **RIBA Stage 2 programme:** `p11` — "RIBA Stage 2 Programme" / "The adjacent programme indicates RIBA stage 2 design."
- **Client-team governance:** `p9` — "THE CLIENT TEAM Reporting & Governance" … "SMBC Project Board – Estates, Education, MDC" … "the Head of Capital Design & Technical Services".
- **Construction Hub / social value context:** `p8` — "Stockport Construction Hub".
- **Partner-level consistency:** `p11` — "SMBC are looking to drive change, working with a design team who can provide consistency, especially at Partner level, for the design programme."
- **Services-only scope:** no page in the document cites goods, part numbers, BoQ items, or equipment specifications — supporting the empty `product_requirement_seeds` array.

Source-hash pin: `source_markdown_hash = f880d06fce4ec2121773069cad36e628fdc1a809ed308e0baf66be1ab5a957f9` (matches `document_extractions.source_content_hash`). Extraction scope: `full_document`. Policy version: `2026.04.12`.
