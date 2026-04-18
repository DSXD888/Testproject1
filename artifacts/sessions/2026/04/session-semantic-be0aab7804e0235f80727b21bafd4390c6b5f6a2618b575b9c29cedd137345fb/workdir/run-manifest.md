---
manifest_version: 1
skill_id: pdf-extraction/tender-specification
skill_version: 1.0.0
job_id: job-28060b02-0254-4a01-b60d-f65dd985afa5
session_id: session-semantic-be0aab7804e0235f80727b21bafd4390c6b5f6a2618b575b9c29cedd137345fb
execution_mode: contract
started_at: 2026-04-18T00:00:00Z
completed_at: 2026-04-18T00:00:00Z
outcome_class: completed_ok
items_processed: 1
items_skipped: 0
items_failed: 0
warnings_count: 6
agent_claimed_pages_visited: 0
agent_claimed_source_url: null
agent_claimed_fingerprint_state: not_applicable
agent_claimed_locale_state: en_GB
agent_claimed_currency_state: GBP
---

# Run Manifest — Tender Specification Extraction

## Summary

Executed the `pdf-extraction/tender-specification` skill in headless contract mode against the Docling-derived markdown for document_extraction `66b64a72-7773-42c8-b725-a915e4639a9c` (raw_document `raw-pc-dc1daee2-772e-f111-813c-005056b64545-303fabd5…`), part of tender document set `tds-pc-dc1daee2-772e-f111-813c-005056b64545` on process `proc-pc-dc1daee2-772e-f111-813c-005056b64545`.

The source document is "6278 - Draft specification - Lot 9 - Provision of Reactive Repairs to Corporate Building" (London Borough of Bexley Building Services Framework, Lot 9 — Reactive Works; contract commencement SEPT 2026). Because the document is scoped entirely to Lot 9 of an 11-lot framework, the extraction emitted a single `lots[]` entry for lot_number `9` plus seven service-oriented `product_requirement_seeds` covering the asset classes in scope. Lot and seed evidence anchors cite section headings observable in the Docling markdown (§1, §2, §3, §5, §6, §7, §9, §10, §11, §12). The `extraction_scope` is `full_document`, and `target_domain = lots`, as required by the skill contract.

`result.json` was written to the session workdir and validated against `skills/pdf-extraction/tender-specification/result.schema.json` before finalisation.

## Steps Taken

1. Inspected the session workdir at `artifacts/sessions/2026/04/session-semantic-be0aab7804e0235f80727b21bafd4390c6b5f6a2618b575b9c29cedd137345fb/workdir` and confirmed no pre-staged structural extraction packet was present in the workdir itself.
2. Located the Docling-extracted markdown for the bound `document_extraction_id` in `state/state.db` (table `document_extractions`) and confirmed the `source_content_hash` (`9a10b6805047eca6227756ee5a73de376352e6deb756d5a3c44bd7e52c8d1921`), `extraction_engine = docling` (v2.88.0), `expected_overlays_json = ["pdf-extraction/tender-specification"]`, and `document_role = technical_specification`.
3. Read the full markdown for the document (180 lines, ~8 KB) covering 12 numbered specification sections (section 4 is absent from the source text).
4. Cross-referenced existing `notice_lots` rows (lot_number 9 title in DB: "Building Services Specification - Reactive Works (Lot 9)") for context only; did not treat the DB row as authoritative for the lot scope.
5. Derived one authoritative lot (lot_number `9`) from the document, compiled requirements from sections 1–3 and 5–12, mapped SLA priorities and KPI targets into `evaluation_criteria` as surrogate award-proxy criteria, and enumerated seven `product_requirement_seeds` for the service classes in scope (M&E reactive, building fabric, life safety systems, specialist FM assets, legionella/ACoP L8 remediation, CAFM integration, 24/7 helpdesk).
6. Catalogued technical standards explicitly named in the document: SFG20, CIBSE Guide M, ISO 14001, Fire Safety Act 2021, CDM 2015, Control of Asbestos Regulations 2012, PUWER, LOLER, PSSR, HSE ACoP L8, BS EN (relevant), and Health and Safety at Work.
7. Recorded six warnings covering absent CPV codes, KPI-only evaluation surrogate, mobilisation duration inconsistency, coarse page_ref (Docling did not surface page-break markers in the extracted markdown), a §4 section-numbering gap, and the draft-document caveat.
8. Wrote `result.json` to the workdir and validated it successfully against the skill's result schema using jsonschema; wrote this run manifest.

## Decisions Made

- **Single lot scope.** The document title and body treat the specification as scoped to Lot 9 only; no other lots are described inside the body. The extraction therefore emits exactly one lot with `lot_number = "9"` and does not attempt to enumerate sibling lots, consistent with the skill contract direction that pre-existing notice_lots rows are for cross-reference only.
- **`extraction_scope = full_document`.** The Docling markdown was ingested in full; no chunking or resume continuation was used, so a stable `full_document` scope is appropriate.
- **CPV codes left empty.** The document body contains no explicit CPV references; rather than fabricate CPV codes from the title, the array was left empty and a `CPV_CODES_NOT_PRESENT` warning was emitted so the daemon can enrich from notice-level signals.
- **Evaluation criteria sourced from KPIs and SLAs.** The source document does not state formal award criteria with weightings. The KPI Framework (§11) and the Reactive Repairs SLA priority table (§5) were surfaced as `evaluation_criteria` because they are the only measurable, scorable commitments present; an `EVALUATION_CRITERIA_IMPLIED_FROM_KPIS` warning flags this provenance for downstream review.
- **Service-oriented seeds, not SKU seeds.** This is a facilities-management service contract; `product_requirement_seeds` therefore describe service bundles (reactive M&E, building fabric, life safety, specialist FM, legionella/ACoP L8, CAFM integration, 24/7 helpdesk) rather than physical goods, each with its own evidence anchor pointing back to the relevant section.
- **Page references conservative.** Docling output in this record lacks explicit page-break markers (`page_count` not populated); page_ref values are set to `p.1` with the section_ref carrying the discriminating signal, and a `PAGE_REF_COARSE` warning was raised.
- **Mobilisation inconsistency surfaced, not reconciled.** Section 9 contains both "6–12 weeks" and "6–8 weeks" language; both are carried through to requirements text and a `MOBILISATION_PERIOD_INCONSISTENT` warning captures the contradiction for human review.

## Warnings

- `CPV_CODES_NOT_PRESENT` — No CPV codes present in document body; `cpv_codes` left empty pending notice-level enrichment.
- `EVALUATION_CRITERIA_IMPLIED_FROM_KPIS` — KPIs (§11) and SLA priorities (§5) used as evaluation_criteria surrogate; no formal award criteria/weightings are stated in the draft.
- `MOBILISATION_PERIOD_INCONSISTENT` — §9 states mobilisation as "6–12 weeks" in paragraph 1 and "6–8 weeks" in its final paragraph.
- `PAGE_REF_COARSE` — Docling markdown lacks page-break markers; page_ref values default to `p.1`.
- `SECTION_NUMBERING_GAP` — Section 4 is absent from the source markdown (headings jump 3 → 5).
- `DRAFT_DOCUMENT` — Document title indicates "Draft specification"; content may evolve before tender issuance.

## Evidence

Evidence anchors emitted in `result.json` point to the following structural locations in the extracted markdown, each observable in the Docling payload for document_extraction `66b64a72-7773-42c8-b725-a915e4639a9c`:

- Cover / title — "London Borough of Bexley — Building Services Specification — Reactive Works — Contract Commencement SEPT 2026".
- §1 Introduction & Purpose — "This specification sets out the requirements for the delivery of Reactive Maintenance across the Corporate estate."
- §2 Scope of Services — 24/7 emergency cover; Reactive repairs to all M&E, fabric, and life-safety systems; Fault investigations and root cause analysis; Make-safe actions; Follow-on works.
- §3 Legislative & Regulatory Compliance — enumerated statute/standard list: Health and Safety at Work; Fire Safety Act 2021; CDM 2015; Control of Asbestos Regulations 2012; PUWER; LOLER; PSSR; HSE ACoP L8; relevant BS EN standards; CIBSE Guide M; SFG20.
- §5 Reactive Repairs — SLA priority table: P1 2h/24h make-safe; P2 1wd/2wd; P3 2wd/4wd; P4 as-agreed.
- §6 Health & Safety Requirements — permit-to-work systems for hot works, electrical isolations, confined spaces, roof access.
- §7 Environmental & Sustainability Requirements — "ISO 14001-compliant Environmental Management System".
- §9 Mobilisation Requirements — "Full onboarding and integration with the Council's CAFM system (TF Cloud), is mandatory, including data migration and validation."
- §10 Asset Verification Requirements — verified asset data upload to CAFM (TF Cloud).
- §11 KPI Framework — 95% PPM completion monthly; 90–95% reactive response compliance; 80% first-time fix rate; data upload within 48 hours; zero major H&S breaches; quarterly social value delivery.
- §12 Operational Reporting & Contract Management Requirements — monthly operational reports (PPM completion, reactive performance, statutory compliance tracker, risk register, social value tracker, energy-saving opportunities).

Daemon-observable references / artifact refs:
- `result.json` written to session workdir (validated against `skills/pdf-extraction/tender-specification/result.schema.json`).
- Source document fields (daemon-authoritative): `document_extractions.id = 66b64a72-7773-42c8-b725-a915e4639a9c`; `source_content_hash = 9a10b6805047eca6227756ee5a73de376352e6deb756d5a3c44bd7e52c8d1921`; `extraction_engine = docling`; `extraction_engine_version = 2.88.0`; `document_role = technical_specification`; `extraction_policy_version = 2026.04.12`; `expected_overlays_json = ["pdf-extraction/tender-specification"]`.
