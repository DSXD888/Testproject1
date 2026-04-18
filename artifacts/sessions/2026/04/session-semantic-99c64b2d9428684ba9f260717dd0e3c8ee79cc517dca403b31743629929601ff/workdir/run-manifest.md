---
manifest_version: 1
skill_id: pdf-extraction/tender-specification
skill_version: 1.0.0
job_id: job-8fa9c404-11c2-40aa-8267-7250402e6d80
session_id: session-semantic-99c64b2d9428684ba9f260717dd0e3c8ee79cc517dca403b31743629929601ff
execution_mode: contract
started_at: 2026-04-18T08:08:00Z
completed_at: 2026-04-18T08:16:02Z
outcome_class: success
items_processed: 1
items_skipped: 0
items_failed: 0
warnings_count: 8
agent_claimed_pages_visited: 0
agent_claimed_source_url: null
agent_claimed_fingerprint_state: n/a
agent_claimed_locale_state: en-GB
agent_claimed_currency_state: GBP
---

## Summary

Semantic extraction pass over the document-extraction record `bb8532b7-c296-40eb-9228-86a0cc5fc3bd` corresponding to raw document `raw-pc-f0f30f94-e22c-f111-813c-005056b64545-ed6bcc33d0fd769cafcdc7a918c1d249a9aa1398c7f3c6a433f58d5c4ce6daf6` for procurement process `proc-pc-f0f30f94-e22c-f111-813c-005056b64545` (Notice `pc-f0f30f94-e22c-f111-813c-005056b64545`, stage run `stage-run-43d02438-92da-4df3-bc14-2ff454300565-proc-pc-f0f30f94-e22c-f111-813c-005056b64545`).

The source document is Annex A.3 — Mechanical and Electrical Scope of Service (document title: "A.3 Mechanical and Electrical Scope of Service (To be read)") for the Stockport Metropolitan Borough Council procurement "020417 - Stockport Town Centre School - Multi-disciplinary Design Team Appointment" (Project E000-25-0031). It defines the MEP consultant scope across RIBA Stages 0–7, with detailed service-category enumerations in Appendix A (Mechanical and Electrical Services) and Appendix B (Life Engineering).

One authoritative lot was emitted (`lot_number: "A.3"`) covering the MEP scope, accompanied by 12 product-requirement seeds (one per MEP service family plus one for Life Engineering), 8 cited technical standards with evidence anchors, 8 warnings that describe material extraction caveats, and 23 document-level evidence anchors. No evaluation criteria or CPV codes were cited by the source document; both were deliberately left empty with explanatory warnings rather than guessed.

The result conforms to `result.schema.json` (validated with `jsonschema` — outcome: VALID). `extraction_metadata.target_domain = "lots"` and `extraction_scope = "full_document"` match the job request. `source_markdown_hash` was computed fresh over the `extracted_markdown` column and differs from the stored `source_content_hash` (which is the raw PDF hash); this is expected because the SKILL.md result contract asks for the markdown hash, not the raw-bytes hash.

## Steps Taken

1. Resolved workdir to `artifacts/sessions/2026/04/session-semantic-99c64b2d9428684ba9f260717dd0e3c8ee79cc517dca403b31743629929601ff/workdir` and confirmed only `storage.sqlite3` (agent-mail) was present — no document artefacts staged locally.
2. Located the authoritative state database at `state/state.db` and confirmed the relevant tables: `document_extractions`, `procurement_processes`, `contract_notices`, `notice_lots`.
3. Fetched the document-extraction row by id `bb8532b7-c296-40eb-9228-86a0cc5fc3bd`. Observed `document_role = technical_specification`, `extraction_policy_version` present on schema, `page_count = 0`, `section_count = 1`, `table_count = 2`, `expected_overlays_json = ["pdf-extraction/tender-specification"]`, `document_title = "A.3 Mechanical and Electrical Scope of Service (To be read)"`.
4. Read the entire `extracted_markdown` (22,372 chars, 215 lines) into `/tmp/tender_md.txt` for analysis. Markdown has a header block, seven numbered RIBA-stage sections (General; Core Services; RIBA 0&1; RIBA 2; RIBA 3; RIBA 4; RIBA 5; RIBA 6&7), and two appendices (A: MEP Services, B: Life Engineering).
5. Cross-referenced the process row (`procurement_processes.id = proc-pc-f0f30f94-e22c-f111-813c-005056b64545`) → buyer "Stockport Metropolitan Borough Council", title "020417 - Stockport Town Centre School - Multi-disciplinary Design Team Appointment".
6. Cross-referenced the notice row (`contract_notices.id = pc-f0f30f94-e22c-f111-813c-005056b64545`) → summary confirms disciplines to appoint: architect, MEP, Principal Designer (CDM & BR), Structural Engineer. `main_category = "150000 - Consultancy"`, no CPV code present.
7. Examined `notice_lots` for this notice — one pre-existing row (lot-1) titled "Multi-disciplinary Design Team Appointment — Stockport Town Centre School (Architect scope, Annex A.1)". Treated this as cross-reference context rather than authoritative ground truth because the source document is Annex A.3 (MEP), not the Architect scope.
8. Computed SHA-256 over the `extracted_markdown` string (`c7369723103f5af2d02c0d38841589964e92eafc10280b2c5514e5ad4658ccad`) and confirmed it differs from the stored `source_content_hash` (`adda92d954e9f65916e93db6e554aa2029cb0ad560479f4ea32dbfc2c3a07ba3` — raw PDF bytes). Used the freshly computed markdown hash for `extraction_metadata.source_markdown_hash`.
9. Read `skills/pdf-extraction/tender-specification/result.schema.json` and `golden.assertions.yaml` to confirm the result contract, required fields, and global assertions (target_domain must equal "lots"; lot_outputs_must_be_non_empty; standards_require_evidence_anchors; require_evidence_anchors; require_product_requirement_seeds).
10. Structured the extraction as one lot (`lot_number: "A.3"`) rather than many, because the document is a single discipline-scoped annex. The existing notice lot-1 captures the bundled multi-disciplinary appointment — reconciliation with the MEP annex is flagged as a warning for the daemon rather than silently overridden.
11. Drafted 12 product-requirement seeds grouping the ~30 Appendix A/B enumerations into coherent MEP service families (HVAC; public health/plumbing; electrical distribution & lighting; fire safety; heating/steam; fuel gas & utilities; telecoms/data/AV; refrigeration & food service; acoustics/vibration; environmental/sustainable; other specialised plant; Life Engineering advisory). Each seed carries per-family evidence anchor citing the first representative Appendix bullet.
12. Emitted 8 technical standards with full evidence anchors: RIBA Plan of Work 2020; BSRIA BG6/2018 (5th Edition) Pro-Formas 1–7; JCT Design & Build Contract; Client OIR + EIR for BIM; BREEAM; CDM Regulations; Disability Discrimination Act; Stockport CAN Plan + Sustainable Build Standard.
13. Left `cpv_codes` and `evaluation_criteria` as empty arrays with explicit warnings (`no_cpv_codes_cited`, `no_evaluation_criteria_in_scope_annex`) — the scope annex contains neither, so guessing would violate the skill's "do not guess without citing evidence" rule.
14. Set `page_ref = "p.?"` on every evidence anchor and encoded the authoritative locator in `section_ref` (e.g. `§2.1`, `Core Services §1.8`, `Appendix A`) because Docling reported `page_count = 0`. Warning `page_references_unavailable` makes this explicit.
15. Wrote `result.json` to the workdir and validated against `result.schema.json` with the `jsonschema` library. Outcome: VALID.
16. Wrote this run manifest with required YAML frontmatter and narrative sections.

## Decisions Made

- **One lot, not many.** Emitted a single lot keyed `A.3`. The source document is a discipline-scoped scope-of-service annex, not a multi-lot ITT. Splitting it into several pseudo-lots (one per RIBA stage, one per MEP sub-discipline, etc.) would fabricate structure the document does not assert. The existing notice-level lot-1 is a bundled multi-disciplinary appointment; the `single_lot_annex_scope` warning asks the daemon to reconcile Annex A.3 as either a distinct lot, a sub-scope under lot-1, or a parallel workstream.
- **Lot number `A.3` matches annex code.** The document title explicitly labels the annex as "A.3". Using `A.3` as the lot_number preserves traceability to the source annex and avoids collision with the pre-existing notice lot-1 numeric scheme.
- **Product seeds grouped by service family, not one-per-bullet.** Appendix A enumerates ~30 individual service lines. Emitting one seed per bullet would create low-signal noise in downstream product matching; emitting one seed per coherent service family (12 total) preserves evidence fidelity while producing useful groupings. Each `technical_attributes` array retains the raw bullet wording from Appendix A / B so no detail is lost.
- **Empty `evaluation_criteria` and `cpv_codes` with warnings, not hallucinated values.** The scope annex does not cite award criteria or CPV codes; the skill's "non-goals" and "guessing without evidence" guardrails forbid invention. Downstream consumers read `no_cpv_codes_cited` and `no_evaluation_criteria_in_scope_annex` rather than misleading populated fields.
- **`page_ref = "p.?"`, `section_ref` authoritative.** Docling recorded `page_count = 0` so no page-number evidence exists. Section references (`§2.1`, `Core Services §1.8`, `Appendix A`) are preserved as the primary locator. Downstream readers follow `section_ref`; `p.?` is a durable signal that pagination was unavailable rather than a silent zero.
- **`tender_is_services_not_goods` warning.** This tender procures design services, not physical goods. Product seeds are explicitly typed `mep_service_scope` / `life_engineering_service_scope` so downstream product matching and quoteability scoring do not interpret these as quotable SKUs.
- **Markdown hash, not raw-bytes hash.** `extraction_metadata.source_markdown_hash` is SHA-256 over `extracted_markdown` (`c7369723…`). The DB's `source_content_hash` (`adda92d9…`) is the raw PDF hash and is not what the result contract asks for.
- **Confidence 7200 bps overall.** Section numbering is clean and standards citations are unambiguous (drives `lots.technical_standards` to 9200 bps and `evaluation_criteria` to 9500 bps since its emptiness is correct). Product-seed grouping (7500 bps) and lot-number assignment (5500 bps — only a weak convention, not an authoritative citation) pull the aggregate down to 7200 bps.

## Warnings

All eight structured warnings in `result.json` are reproduced here in summary form:

1. `tender_is_services_not_goods` — this is a design-services procurement; seeds are service families not SKUs.
2. `single_lot_annex_scope` — source is Annex A.3 of a multi-annex scope set; daemon must reconcile with notice-level lot-1 (Architect / bundled appointment).
3. `no_cpv_codes_cited` — source document cites no CPV codes; process-level `main_category` is the Contracts Finder taxonomy value "150000 - Consultancy", not a CPV code; `cpv_codes` deliberately empty.
4. `no_evaluation_criteria_in_scope_annex` — scope annexes do not carry evaluation criteria; `evaluation_criteria` deliberately empty.
5. `page_references_unavailable` — `document_extractions.page_count = 0`; anchors use `section_ref` as the authoritative locator.
6. `section_numbering_restarts` — the outer numbered list restarts at `1` three times (General block, Core Services block, Appendix block); references qualify with the containing section name.
7. `version_control_table_empty` — the document's Version Control table rows are blank; Document Number 00 and Revision 00 indicate initial release.
8. `procurement_project_context` — header identifies Project E000-25-0031 "Stockport Town Centre School", Document Owner: Design & Technical Services Manager.

## Evidence

The evidence for this extraction lives in two places: `evidence_anchors` (document-level, 23 anchors) and per-lot / per-seed / per-standard `evidence_anchor` fields in `result.json`. The most consequential evidence citations are:

- Document identity: `Header block` — "Design and Technical Services — Estates & Asset Management — Scope of Service — MEP — Project: Stockport Town Centre School — Project Number: E000-25-0031 — Document Number: 00 — Revision: 00" (result-contract field: `lots[0].evidence_anchor`; also replicated in `evidence_anchors[0]`).
- Contract form: `§1.3 General` — "Proposed Form of Building Contract: JCT Design & Build Contract" (`technical_standards[2]`; also `evidence_anchors[2]`).
- Consultant form: `§1.4 General` — "Form of Consultant Appointment: Multi Discipline consultant (with novation)" (cited in warning `tender_is_services_not_goods`; also `evidence_anchors[3]`).
- Insurance floors: `§1.6 / §1.7 / §1.8 General` — "£10,000,000 (ten million pounds)" for Employer Liability, Public Liability and Professional Indemnity Insurance (`evidence_anchors[5]`–`evidence_anchors[7]`).
- MEP scope of responsibility: `Core Services §1.4` — "The Mechanical and Electrical Services as described here to shall be provided including, but not limited to the following elements or areas of responsibility" (`evidence_anchors[9]`).
- Climate alignment: `Core Services §1.8` — "In March 2019 Stockport Council declared a climate emergency… Climate Action Now (CAN) plan… the M&E survey should include specific high-level recommendations that can be undertaken to meet the requirements of CAN plan and our sustainable build standard" (`technical_standards[7]`; also `evidence_anchors[10]`).
- BREEAM: `Core Services §1.10` — "support the work of the BREEAM assessor by issuing all required evidence and taking part in BREEAM meetings to ensure the target BREEAM rating is met" (`technical_standards[4]`; also `evidence_anchors[11]`).
- BIM: `Core Services §1.2` — "the designer is required to comply with all of the requirements of the Clients Organisational Information Requirements for BIM and Employers Information Requirements for BIM" (`technical_standards[3]`; also `evidence_anchors[8]`).
- CDM Regulations: `§2.11` — "Comply with the CDM Regulations." (`technical_standards[5]`; also `evidence_anchors[15]`).
- Disability Discrimination Act: `Core Services §1.19` — "ensure the proposals comply with the requirements of the Disability Discrimination Act" (`technical_standards[6]`; also `evidence_anchors[12]`).
- RIBA Plan of Work 2020: `§2.1 / §3.1 / §4.1 / §5.1 / §6.1 / §7.1` — six direct compliance citations across the staged sections (`technical_standards[0]`; also `evidence_anchors[13]`, `evidence_anchors[16]`–`evidence_anchors[20]`).
- BSRIA BG6/2018: `§2.2 / §3.2 / §4.2 / §5.2 / §6.2 / §7.2` — "Comply with all of the Requirements of the BSRIA BG6/2018 Design Framework for Building Services, 5 th Edition covering Pro-Forma …" (`technical_standards[1]`; also `evidence_anchors[14]`).
- MEP service breadth: `Appendix A — Mechanical and Electrical Services` — "Mechanical and Electrical services include: Acoustics Advice; Air conditioning and mechanical ventilation; …; Vibration control in connection with services" (`evidence_anchors[21]`; each of the 11 MEP seeds cites its representative bullet here).
- Life Engineering: `Appendix B — Life Engineering` — "Life Engineering services include: Advice on Electrical Distribution Services; Advice on Electric Lighting and Power Installations, including Lighting Fittings; Advice on Fire Engineering; Advice on Fire Detection and Alarm Services" (`evidence_anchors[22]`; cited by the `life-engineering-advisory` seed).

### Daemon observations

- `document_extractions.id = bb8532b7-c296-40eb-9228-86a0cc5fc3bd`
- `document_extractions.raw_document_id = raw-pc-f0f30f94-e22c-f111-813c-005056b64545-ed6bcc33d0fd769cafcdc7a918c1d249a9aa1398c7f3c6a433f58d5c4ce6daf6`
- `document_extractions.process_id = proc-pc-f0f30f94-e22c-f111-813c-005056b64545`
- `document_extractions.notice_id = pc-f0f30f94-e22c-f111-813c-005056b64545`
- `document_extractions.document_set_id = tds-pc-f0f30f94-e22c-f111-813c-005056b64545`
- `document_extractions.document_role = technical_specification`
- `document_extractions.source_content_hash = adda92d954e9f65916e93db6e554aa2029cb0ad560479f4ea32dbfc2c3a07ba3` (raw PDF bytes)
- `document_extractions.page_count = 0`; `section_count = 1`; `table_count = 2`; `image_count = 0`; `empty_page_count = 0`; `has_ocr_content = 0`
- `document_extractions.expected_overlays_json = ["pdf-extraction/tender-specification"]`
- `document_extractions.extracted_markdown` length: 22,372 chars; fresh SHA-256 = `c7369723103f5af2d02c0d38841589964e92eafc10280b2c5514e5ad4658ccad` (this value is written to `result.extraction_metadata.source_markdown_hash`)
- `contract_notices.main_category = "150000 - Consultancy"` (Contracts Finder taxonomy, not CPV)
- `notice_lots` for this notice: 1 pre-existing row (`pc-…-lot-1`, "Multi-disciplinary Design Team Appointment — Stockport Town Centre School (Architect scope, Annex A.1)")
- Result artefact path: `workdir/result.json`; validated against `result.schema.json` with `jsonschema.validate` — outcome: VALID.
