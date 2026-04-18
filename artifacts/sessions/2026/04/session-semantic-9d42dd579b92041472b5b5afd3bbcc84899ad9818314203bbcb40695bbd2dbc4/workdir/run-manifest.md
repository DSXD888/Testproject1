---
manifest_version: 1.0.0
skill_id: pdf-extraction/tender-specification
skill_version: 1.0.0
job_id: job-0590e32e-123c-4ef1-9abf-7566b3edf60c
session_id: session-semantic-9d42dd579b92041472b5b5afd3bbcc84899ad9818314203bbcb40695bbd2dbc4
execution_mode: contract
started_at: 2026-04-18T03:43:38Z
completed_at: 2026-04-18T03:45:17Z
outcome_class: success_with_warnings
items_processed: 1
items_skipped: 0
items_failed: 0
warnings_count: 6
agent_claimed_pages_visited: 0
agent_claimed_source_url: null
agent_claimed_fingerprint_state: not_applicable_headless_contract
agent_claimed_locale_state: en-GB
agent_claimed_currency_state: GBP
---

# Run Manifest — pdf-extraction/tender-specification

## Summary

Processed Docling markdown for document_extraction_id `6330db22-194b-41e9-96e3-8e60771e9be7` ("6278 - Draft specification - Lot 9"), which is the London Borough of Bexley Building Services Specification for **Reactive Works (Lot 9)** of Framework 6278. The document is single-lot in scope; one authoritative lot entry was produced with 21 requirement lines, 12 technical standards, 1 evaluation-criteria-style KPI framework entry, and 12 evidence anchors. No explicit CPV codes, no discrete product SKUs, and no formal tender award criteria were found in the document itself — each absence is reported as a structured warning. `result.json` was written to the workdir and matches the bound `result.schema.json` (target_domain=`lots`, extraction_scope=`full_document`).

## Steps Taken

1. Located the bound skill definition and `result.schema.json` under `skills/pdf-extraction/tender-specification/` and confirmed required fields and evidence-anchor shape.
2. Read the Docling-produced markdown from `document_extractions.extracted_markdown` for id `6330db22-194b-41e9-96e3-8e60771e9be7` (10,142 bytes, 180 lines).
3. Pulled document metadata: `document_role = technical_specification`, `extraction_policy_version = 2026.04.12`, `expected_overlays_json = ["pdf-extraction/tender-specification"]`, `source_content_hash = 9a59c6a303ce3bfb9475919cf94e65387d3f6310565f21ef023e20298d6ce05c`.
4. Cross-referenced the parent notice `pc-dc1daee2-772e-f111-813c-005056b64545` in `notice_lots` — confirmed Lot 9 title "Provision of Reactive Repairs to Corporate Building" under Framework 6278; this was used for cross-reference only and did not override the specification text.
5. Identified document scope as a single-lot specification (the document header names only "Reactive Works" / the stored doc title names Lot 9 explicitly). Did not attempt to split synthetic sub-lots.
6. Walked numbered sections 1–12 to harvest requirements, service SLAs, health & safety obligations, mobilisation timing, KPI thresholds, and reporting obligations.
7. Extracted explicit technical/regulatory standard references: SFG20, CIBSE Guide M, ISO 14001, BS EN (generic), HSE L8, CDM 2015, Fire Safety Act 2021, Control of Asbestos Regulations 2012, PUWER, LOLER, PSSR, and the top-line Health and Safety at Work obligation.
8. Captured the Section 11 KPI Framework as a single `evaluation_criteria` entry (flagged via warning that these are operational performance thresholds, not formal tender award criteria).
9. Left `product_requirement_seeds` empty and lot `product_requirements` empty with an explicit warning — the specification is a pure labour/service spec with no discrete goods, quantities, or bill of material items.
10. Left `cpv_codes` empty with a warning — the document text contains no CPV references; CPV should be sourced from the parent notice record, not inferred here.
11. Recorded 12 evidence anchors (one per major section 1–12, plus the document header) and attached the appropriate anchor to each claim on the lot.
12. Wrote `result.json` and validated top-level required keys, target_domain=`lots`, extraction_scope=`full_document`, counts, and JSON parse validity.

## Decisions Made

- **Single lot, not multi-lot.** The document is explicitly titled and scoped to Reactive Works / Lot 9 of Framework 6278. No attempt was made to enumerate sibling lots (1-8, 10-11) that appear in the parent notice record — those are out of scope for this document's extraction.
- **Stable lot numbering.** Used `lot_number: "9"` consistent with the notice's existing `notice_lots.lot_number` for Lot 9 (so downstream joins remain stable across re-runs).
- **KPIs treated as evaluation_criteria (single entry) with warning.** Section 11 KPIs are operational performance thresholds, not tender award criteria. Captured as a single evaluation_criteria entry so downstream consumers still get a structured handle on the KPI framework, while warning `NO_FORMAL_TENDER_EVALUATION_CRITERIA` communicates the semantic distinction.
- **No CPV fabrication.** Document does not cite CPV codes; left `cpv_codes` empty rather than inferring (e.g. 50000000 or 50700000) from services described. Warning emitted.
- **No product seed fabrication.** Reactive-maintenance service contracts do not itemise procurable products; left `product_requirement_seeds` and lot `product_requirements` empty with warning. Personnel, PPE, and spare parts are implied labour inputs, not procurement targets expressible as candidate seeds.
- **Preserved source idiosyncrasies.** The markdown has a missing Section 4 and a mobilisation-period discrepancy (6-12 vs 6-8 weeks). Both are captured as warnings (`SECTION_NUMBERING_GAP`, `MOBILISATION_PERIOD_AMBIGUITY`) rather than silently smoothing over.
- **Document role carried through.** Set `extraction_metadata.document_role = technical_specification` matching `document_extractions.document_role`.
- **Source markdown hash.** Used the `source_content_hash` from the document_extractions row (`9a59c6a30...`) as the canonical `source_markdown_hash` so daemon de-duplication and resume logic align with existing state.

## Warnings

All warnings are also captured in `result.json` with evidence refs:

- `NO_EXPLICIT_CPV_IN_DOCUMENT` — Spec text contains no CPV references; daemon should prefer notice-record CPV.
- `NO_FORMAL_TENDER_EVALUATION_CRITERIA` — Section 11 is operational KPIs, not award-evaluation criteria/weightings.
- `NO_DISCRETE_PRODUCT_ITEMS` — No SKUs, quantities, or bill-of-material items; product_requirement_seeds is intentionally empty.
- `DOCUMENT_ROLE_PARTIAL` — Source is flagged "Draft specification"; content may not be final.
- `SECTION_NUMBERING_GAP` — Numbered sections jump 3 → 5; no Section 4 present in the markdown.
- `MOBILISATION_PERIOD_AMBIGUITY` — Section 9 states both "6-12 weeks" and "6-8 weeks" for the mobilisation window.

## Evidence

All claims on the lot entry are anchored to specific sections of the Docling markdown. Representative anchors used:

- Lot identity / commencement — `document-title`: "London Borough of Bexley — Building Services Specification — Reactive Works — Contract Commencement SEPT 2026".
- Scope of services — `section-2`: "24/7 emergency cover; Reactive repairs to all M&E, fabric, and life-safety systems; Fault investigations and root cause analysis; Make-safe actions; Follow-on works as required."
- Standards (SFG20, CIBSE Guide M) — `section-1`: "Deliver services in accordance with industry best practice, including SFG20, CIBSE Guide M, and manufacturer recommendations."
- Legislative obligations (Fire Safety Act 2021, CDM 2015, Asbestos 2012, PUWER, LOLER, PSSR, L8, BS EN) — `section-3`: the bulleted Legislative & Regulatory Compliance list.
- SLA priorities — `section-5`: "Priority 1 Emergency — Response 2 hours / Resolution 24 hours (make safe); Priority 2 Urgent — 1 working day / 2 working days; Priority 3 Standard — 2 working days / 4 working days; Priority 4 Non-Urgent — As agreed."
- H&S permits/RAMS — `section-6`: "site-specific Risk Assessment and Method Statements (RAMS)... permit-to-work systems for: Hot works; Electrical isolations; Confined spaces; Roof access."
- ISO 14001 EMS — `section-7`: "The Supplier shall operate an ISO 14001-compliant Environmental Management System".
- Mobilisation — `section-9`: "mobilisation plan within 10 working days of contract award. The mobilisation period (typically 6-12 weeks)".
- CAFM integration (Civica/TF Cloud) — `section-1`, `section-5`, `section-10`: repeated Council CAFM references with 48-hour data-upload SLA.
- KPI framework — `section-11`: "95% PPM completion monthly; 90-95% reactive response compliance; 80% first-time fix rate; Data upload within 48 hours; Zero major H&S breaches; Quarterly social value delivery."
- Reporting obligations — `section-12`: Statutory Compliance Tracker, Risk Register, Social Value Tracker, Performance Dashboard, Exception Reporting, Continuous Improvement, Financial Summary, Forward Look.

Cross-reference evidence (for lot identity, not used as authoritative text): parent notice `pc-dc1daee2-772e-f111-813c-005056b64545` row in `notice_lots` where `lot_number=9` has title "Provision of Reactive Repairs to Corporate Building" — consistent with, but not overriding, the specification's self-description.
