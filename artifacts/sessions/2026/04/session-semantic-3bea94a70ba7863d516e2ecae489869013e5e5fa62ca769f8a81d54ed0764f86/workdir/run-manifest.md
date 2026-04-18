---
manifest_version: 1
skill_id: pdf-extraction/tender-specification
skill_version: 1.0.0
job_id: job-fc46a8f8-e146-4abc-afed-22c36366a4a9
session_id: session-semantic-3bea94a70ba7863d516e2ecae489869013e5e5fa62ca769f8a81d54ed0764f86
execution_mode: contract
started_at: 2026-04-18T01:36:29Z
completed_at: 2026-04-18T01:38:25Z
outcome_class: success
items_processed: 1
items_skipped: 0
items_failed: 0
warnings_count: 6
agent_claimed_pages_visited: 0
agent_claimed_source_url: null
agent_claimed_fingerprint_state: not_applicable
agent_claimed_locale_state: en-GB
agent_claimed_currency_state: not_applicable
---

# Run Manifest — pdf-extraction/tender-specification

## Summary

Extracted one authoritative lot record (Lot 4, "Fire Door Specification") plus eight product-requirement seeds, six fire-door technical standards, and twenty-two requirement statements from the Docling markdown of `document_extraction_id=a3c95c8d-442a-43b3-9218-c84fad1b55cd` (raw_document_id `raw-pc-dc1daee2-...-829876f3`). Document is the draft technical specification for Lot 4 ("Provision of Fire Door Surveys") of London Borough of Bexley framework 6278 (Building Services Framework — Lots 1-11), notice `pc-dc1daee2-772e-f111-813c-005056b64545`. `extraction_metadata.target_domain` is `lots`, `extraction_scope` is `full_document`, and `policy_version` is `2026.04.12`. No CPV codes were cited in the document body itself; the lot CPV (50700000-2) was inherited from notice metadata and product-seed CPVs (44221220-3 fire doors, 44521120-5 closer hardware) are heuristic — these are flagged with `CPV_NOT_CITED_IN_DOCUMENT`. No evaluation criteria are present in this draft specification (flagged with `EVALUATION_CRITERIA_NOT_PRESENT`).

## Steps Taken

1. Loaded job parameters and resolved `document_extraction_id=a3c95c8d-442a-43b3-9218-c84fad1b55cd` from the Layer 1 daemon SQLite (`state/state.db`). Confirmed `extraction_engine` was Docling, `is_current=1`, and `expected_overlays_json=["pdf-extraction/tender-specification"]` matched the bound skill.
2. Read the full `extracted_markdown` (4,728 bytes, 131 lines) and the structured section index (13 sections, all on `page_start=1` in the Docling structure).
3. Cross-referenced parent notice metadata (`contract_notices.id=pc-dc1daee2-...`): notice title "6278 - Building Services Framework - Lots 1-11", buyer "London Borough of Bexley", main_category CPVs `50700000-2`, `50750000-7`, `71315000-9`, and `notice_lots.lot_number=4` titled "Provision of Fire Door Surveys".
4. Used the Docling section index to anchor each output (lot-level evidence anchor, per-product seed evidence anchors, per-standard evidence anchors). Each citation references the corresponding `#/texts/N` section_ref taken directly from `document_extractions.extracted_sections_json`.
5. Walked §3 (Performance & Functional Requirements) sub-clauses 3.1–3.5 to enumerate product-requirement seeds: FD30S flat-entrance doors, general FD30/FD60/FD90/FD120 door sets, intumescent seals, cold-smoke brush seals, BS EN 1154 self-closers, fire-rated hinges, compliance signage, fire-rated glazing system. Each seed includes a stable `seed_key` of the form `lot4-<product-slug>` for resume-stability across reruns.
6. Captured technical standards explicitly cited in document text: BS EN 1154, Fire Safety (England) Regulations 2022, UK Building Regulations, Regulatory Reform (Fire Safety) Order 2005 (the "Fire Safety Order"), and the FD30/FD60/FD90/FD120 + FD30S/FD60S categorical fire-resistance ratings. Each standard has its own evidence_anchor pointing back to its citation in the markdown.
7. Resolved evaluation_criteria as an empty array — the document contains no scoring weights, scoring methodology, or award criteria language; this is a technical specification only.
8. Set `extraction_metadata.source_markdown_hash` to the `source_content_hash` value already canonicalised by the daemon (`8daf327fba87cde380a285e86da2578207cfb82cbc64e84eb3db2587facf19b8`), avoiding CLI-newline drift.
9. Validated `result.json` against `result.schema.json` (Draft 2020-12) — no schema violations.
10. Wrote `result.json` and this `run-manifest.md` to the contract workdir for daemon persistence.

## Decisions Made

- **Lot count = 1 with `lot_number = "4"`** (string per schema). The bound document is a per-lot specification (Lot 4 of an 11-lot framework). Did not synthesise additional lots from sibling notice metadata — those have their own per-lot specifications and should not be inferred here.
- **`description` records both notice scope and document scope** with an explicit note that the document scope is broader than the notice lot title (`Provision of Fire Door Surveys` vs. supply/install/inspect/maintain). Cross-flagged via `LOT_TITLE_SCOPE_MISMATCH` warning so downstream qualification can re-evaluate.
- **`cpv_codes` populated only with `50700000-2`** (the lot-relevant subset of the notice's three CPVs); did not include `50750000-7` (lifts) or `71315000-9` (general building services) because they are not anchored to fire-door scope. Noted CPV provenance is notice metadata, not document body, via `CPV_NOT_CITED_IN_DOCUMENT` warning. Product-seed CPVs (`44221220-3`, `44521120-5`) treated as low-confidence heuristics.
- **Eight product-requirement seeds**, each with stable `seed_key` and per-seed evidence anchor. Excluded asset-management deliverables (TF Cloud / Civica uploads, certificates, schedules) because those are service deliverables, not procurable goods, and would not yield product candidates.
- **Six technical standards** captured. Categorical fire ratings (FD30/FD60/FD90/FD120 and FD30S/FD60S) are emitted because the schema's `standard_ref` accepts free-form references and these are the principal compliance grades cited; treating them as standards keeps them visible to the technical-standard analytics path. Statutory instruments (Building Regulations, Fire Safety Order, Fire Safety (England) Regulations 2022) are cited verbatim and emitted as standards.
- **`extraction_confidence_bps = 8200`**. Document is short, well-structured, draft-named, and unambiguous about technical content; main confidence drag is the absence of cited CPV codes and evaluation criteria.
- **No `product_candidates`/`requirements` semantic rows synthesised.** Per skill contract, `target_domain="lots"` only; the daemon translates the `product_requirement_seeds` array into its own writes.
- **`extraction_scope = "full_document"`** because the document was processed in a single pass against the full Docling markdown — no chunked merge.

## Warnings

- `EVALUATION_CRITERIA_NOT_PRESENT` — Draft technical specification carries no scoring weights or award methodology. Evaluation criteria array is empty.
- `LOT_TITLE_SCOPE_MISMATCH` — Notice lot 4 title "Provision of Fire Door Surveys" understates the specification scope (supply/install/inspect/certify/maintain).
- `CPV_NOT_CITED_IN_DOCUMENT` — CPV codes are inherited from notice metadata (lot CPV) or heuristically inferred (product-seed CPVs), never cited in document text.
- `DOCUMENT_HARDWARE_TEXT_AMBIGUOUS` — §3.3 Hardware & Ironmongery sentence boundaries are uncertain in the Docling rendering between "Hinges", "Compliant signage", and the assembly-conformance clause.
- `DRAFT_DOCUMENT` — Document title contains "Draft Specification"; downstream consumers should reconcile against the final ITT pack when published.
- `PAGE_COUNT_ZERO` — `document_extractions.page_count = 0` and all 13 sections collapse to `page_start=1`. All evidence_anchor `page_ref` values use `"p1"`.

## Evidence

All major claims are grounded in `extracted_markdown` and `extracted_sections_json` of `document_extractions.id=a3c95c8d-442a-43b3-9218-c84fad1b55cd` (source_content_hash `8daf327fba87cde380a285e86da2578207cfb82cbc64e84eb3db2587facf19b8`).

- **Buyer & document identity** — `#/texts/1` "London Borough of Bexley", `#/texts/3` "Fire Door Specification", `document_extractions.document_title = "6278 - Draft Specification - Lot 4"`.
- **Lot identity** — `notice_lots.lot_number=4`, `notice_lots.title="Provision of Fire Door Surveys"`, parent notice `contract_notices.id=pc-dc1daee2-772e-f111-813c-005056b64545`, `contract_notices.title="6278 - Building Services Framework - Lots 1-11"`.
- **Statutory framing** — `#/texts/40` "compliance with current UK Health and Safety Executive (HSE) guidance, Building Regulations, and relevant British Standards, and to meet all statutory duties under the Fire Safety Order".
- **Scope of works** — `#/texts/42` "Supply and installation of new fire doors and frames Inspection, testing, and certification of existing fire doors", "Maintenance and remedial works to ensure continued compliance", "Provision of all test evidence, certification, and product data sheets", "Updating asset records and uploading all documentation to the Authority's compliance platform (TF Cloud / Civica)".
- **Performance & functional requirements** — `#/texts/47` covers FD ratings (FD30/FD60/FD90/FD120), FD30S for flat entrance doors, intumescent and cold smoke seals continuous and undamaged, FD30S/FD60S cold smoke brush-type seals on escape routes, "All hardware must be fire-rated and tested to relevant EN standards in force", "Include self-closing devices (BS EN 1154)", "Hinges Compliant signage ('Fire Door – Keep Shut / Locked')", "All hardware must form part of the tested and approved door set assembly", "Any glazing must use fire-rated glass installed with approved fire-resistant glazing systems", "Installation must be carried out by competent installers trained in fire door installation".
- **Inspection cadence** — `#/texts/63` "All fire doors must be inspected, as a minimum, annually" and "For domestic premises, communal doors must be checked quarterly in accordance with Fire Safety (England) Regulations 2022".
- **Inspection content** — `#/texts/63` "Inspections shall include checks for leaf/frame integrity, gaps, seals, signage, and correct closer operation".
- **Documentation regime** — `#/texts/67` "Contractors must submit installation service sheets, inspection logs, photographs, and asset schedules for each fire door. All documents must be uploaded to TFCloud / Civica for full audit trail and compliance verification".
- **Contractor competence** — `#/texts/73` "All contractors must be Competent and trained in the installation of fire doors and frames including all door furniture", "Risk Assessments and Method Statements must be approved before work begins".
- **Handover & QA** — `#/texts/76` joint verification inspection requirement, final installation/commissioning pack, door-by-door register, "Non-conformances shall be rectified at contractor's cost".
- **Inspection checklist** — Appendix A enumerates: door leaf condition, frame condition and fixings, gaps within tolerance ≤4mm, intumescent/smoke seals continuous and undamaged, self-closer operates effectively, signage correctly fitted, no obstructions or alterations, overall Pass/Fail rating.
- **Asset schedule template** — Appendix B columns: Property/Block Name, Door Location, Fire Rating (FD30/FD60), Door Type/Material, Installation Date, Inspection Date, Findings, Action Taken, Certification Reference.
- **Certificate template** — Appendix C fields: Certificate No, Door ID, Location, Installer, Date Installed, Fire Rating, Smoke Seal Yes/No, Inspector Signature, Date of Inspection.

No evidence supports any evaluation criterion, scoring weight, contract value, deadline, or named exporter/manufacturer in this document — those fields are correctly empty or absent.
