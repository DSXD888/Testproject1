---
manifest_version: 1
skill_id: pdf-extraction/tender-specification
skill_version: 1.0.0
job_id: job-41fd7c75-5d8a-4ae3-8f83-c0681edb31ff
session_id: session-semantic-6dc02aead6f750f9f88a265d7d7a81d6008382fd672091efbcb4618ad390ae4f
execution_mode: contract
started_at: 2026-04-18T04:29:30Z
completed_at: 2026-04-18T04:30:30Z
outcome_class: completed_with_warnings
items_processed: 1
items_skipped: 0
items_failed: 0
warnings_count: 10
agent_claimed_pages_visited: 57
agent_claimed_source_url: "https://procontract.due-north.com/RfxResponse/Index?rfxId=ea9d90b2-4833-f111-813c-005056b64545"
agent_claimed_fingerprint_state: not_applicable
agent_claimed_locale_state: en-GB
agent_claimed_currency_state: GBP
---

## Summary

Ran tender-specification semantic extraction on document_extraction_id `9c4efdfb-b0e8-4f76-8cb6-4fa0148b00ce` (raw doc `raw-pc-dc1daee2-...-36262bad`), filename `6278 - Draft T&Cs - Lot 1 Provision of Fire Risk Assessment.pdf`, 57 pages, document_role=`technical_specification` (per `document_extractions.document_role`), source_content_hash `8ceea88c7c1944eb0b656cf91f20385e6299204ded72433d1ac454cc7df7eacf`.

Outcome: `completed_with_warnings`. The PDF is a contract terms-and-conditions template for Lot 1 (Fire Risk Assessment Services) of the London Borough of Bexley 6278 Building Services Framework. The semantic specification content (Schedule 1) is an empty heading placeholder, as are Schedules 2, 3, and 4. Result extracts the one identifiable lot (Lot 1) with title and high-level contractual requirements but emits zero product_requirement_seeds, zero evaluation_criteria, and zero CPV codes (none cited within the document body). `extraction_confidence_bps` = 3000 to reflect that the bulk of the tender-specification target signal is absent from this PDF and lives in a separate document in the document set.

## Steps Taken

1. Read job parameters from execution context (skill_id, document_extraction_id, target_domain=`lots`, extraction_scope=`full_document`, stage_run_id, process_id, notice_id).
2. Loaded the Docling-produced markdown, sections JSON, tables JSON, and metadata for `9c4efdfb-...` from the daemon state DB at `state/state.db` (extracted_markdown 145,925 bytes, sections JSON 8,503 bytes, tables JSON 3 bytes — i.e. zero tables).
3. Confirmed process and notice context: process `proc-pc-dc1daee2-...` titled `6278 - Building Services Framework - Lots 1-11`, buyer London Borough of Bexley, latest deadline 2026-05-18, 11 child notice_lots in `notice_lots`. Notice CPV codes (50700000-2, 50750000-7, 71315000-9) live in `contract_notices.main_category` only — not in this PDF body.
4. Walked the document section index (88 sections). Identified Form of Contract (Part 1, p.1-4), General Terms and Conditions (Part 2, p.5-52, clauses A1-I2), Special Terms and Conditions (Part 3, p.53, S1 TUPE), and Schedule headings 1-4 (p.54-57).
5. Inspected the four Schedule headings — confirmed each is a bare `##` heading with no body content. The Specification (Schedule 1) is therefore unavailable from this PDF.
6. Searched the body for fire-, standard-, ISO-, BS-, CPV-, and evaluation-related terms. Found only:
   - Cover page subject line declaring Fire Risk Assessment Services scope (p.1, line 17 of markdown).
   - WHEREAS recital A referring to "security services" — boilerplate template drift, contradicts cover page.
   - Quality Standards definition (clause A1, p.10) citing BSI / ISO generically.
   - Clause B3.1 obliging compliance with Quality Standards (p.16).
   - Clause B4.2 forward-referencing "paragraph 3.12 of the Specification" for fire risk assessor qualifications — paragraph 3.12 is not present here.
   - Clause B4.11 requiring DBS registration where the Specification mandates it.
7. Composed the `lots` array with one Lot 1 record carrying title, description, the six contractually-grounded requirements above, two generic technical_standards entries (BSI, ISO), zero CPV codes, zero product_requirements, zero evaluation_criteria, and an evidence anchor on the cover-page subject line.
8. Set `product_requirement_seeds` to an empty array (no extractable seeds — Schedule 1 is empty and Fire Risk Assessment is a service, not a procurable product).
9. Recorded 10 warnings covering document role mismatch, empty schedule placeholders, template content drift, generic-only standards, unresolved forward references, missing evaluation criteria, missing CPV in body, and template dates.
10. Computed `extraction_confidence_bps` = 3000 and per-field confidences reflecting high lot-identity certainty but near-zero scope-content certainty.
11. Wrote `result.json` to the workdir root and validated against `result.schema.json` with `jsonschema.validate` — VALID.
12. Wrote this run manifest to the workdir root.

## Decisions Made

- **One lot, not eleven.** The PDF's title, filename, and cover-page subject all scope it to Lot 1 only. Although the parent notice has 11 lots in `notice_lots`, those are not authoritative for this document. Per SKILL.md ("do not treat pre-existing lot rows as authoritative truth if the tender document contradicts them"), I emitted only Lot 1.
- **Lot description includes the 2020-2022 contract period as "template dates".** The cover page is "Dated ... 2020" and recital A names Sep 2020 - Aug 2022, but the parent notice published 2026-04-02 and the contract is clearly being re-let. I noted these as template placeholders rather than treating them as authoritative dates.
- **Did not import notice-level CPVs (50700000-2 etc.) into the lot's `cpv_codes`.** The tender-specification skill is meant to extract CPV references that appear *in the document*. None do. Importing them from `contract_notices.main_category` would be hallucination relative to this document.
- **Treated WHEREAS recital A ("security services") as boilerplate template drift, not as scope evidence.** The cover page ("Fire Risk Assessment Services") and source filename are corroborating signals; the recital is unedited template text.
- **Emitted two generic standards entries (BSI, ISO) instead of zero.** Clause A1 and B3.1 do create a real obligation to comply with BSI/ISO standards, but no specific standard numbers (e.g., BS 9999, PAS 79) are cited. Confidence on `lots.technical_standards` is therefore low (3000 bps).
- **Outcome class = `completed_with_warnings`, not `failed`.** The skill ran cleanly and the extracted content is correct as far as it goes; the caveat is the source PDF lacks the actual specification. The job did not fail per the contract — it succeeded in identifying that there is no extractable specification content here.
- **`document_role` retained as `technical_specification` in extraction_metadata** (matching `document_extractions.document_role` in state) but flagged with a `DOCUMENT_ROLE_MISMATCH` warning so the daemon / validator can decide whether to reroute future overlays.

## Warnings

10 warnings emitted in `result.json.warnings`:

1. `DOCUMENT_ROLE_MISMATCH` — daemon-classified as `technical_specification`; filename and `raw_documents.metadata.document_role_hint` say `terms_and_conditions`.
2. `SPECIFICATION_PLACEHOLDER_EMPTY` — Schedule 1 is a bare heading on p.54.
3. `EMPTY_SCHEDULE_PLACEHOLDERS` — Schedules 2, 3, 4 also empty placeholders.
4. `TEMPLATE_CONTENT_DRIFT` — recital A says "security services" instead of fire risk.
5. `STANDARDS_GENERIC_ONLY` — only BSI/ISO genericisms; no specific standard numbers.
6. `FORWARD_REFERENCE_UNRESOLVED` — clause B4.2 cites paragraph 3.12 of Specification, which is missing.
7. `NO_EVALUATION_CRITERIA` — none in document; would normally live in Schedule 3 (empty).
8. `NO_CPV_IN_BODY` — CPVs only in notice metadata, not in PDF text.
9. `NO_PRODUCT_SEEDS` — service deliverable; no product candidates extractable.
10. `TEMPLATE_DATES` — Sep 2020 - Aug 2022 dates are template placeholders, not the live 6278 framework period.

## Evidence

All major claims are anchored to observable structural elements in the Docling output:

- **Lot identity (Lot 1, Fire Risk Assessment Services)** — anchor: cover page text "RELATING TO THE PROVISION OF FIRE RISK ASSESSMENT SERVICES" (p.1, markdown line 17, `extracted_sections_json` heading "AGREEMENT" at `#/texts/1`); filename `6278 - Draft T&Cs - Lot 1 Provision of Fire Risk Assessment.pdf` (raw_documents.metadata.filename).
- **Contract structure references Specification** — anchor: clause 1.2(d) "The Specification appended hereto as Schedule 1" (p.2, markdown line 48).
- **Generic Quality Standards (BSI, ISO)** — anchor: A1 Definitions, "'Quality Standards' means the quality standards published by BSI British Standards Institution ... the International Organisation for Standardisation" (p.10, markdown line 320, section `A1 Definitions and Interpretation` at `#/texts/69`).
- **Compliance obligation** — anchor: clause B3.1 (p.16, markdown line 430, section `B3 Quality Standards` at `#/texts/227`).
- **Fire risk assessor qualifications forward-reference** — anchor: clause B4.2 "Staff who are fire risk assessors should hold the qualifications set out in paragraph 3.12 of the Specification" (p.16, markdown line 436, section `B4 Contractor's Staff` at `#/texts/230`).
- **Schedule 1 placeholder** — anchor: heading `## SCHEDULE 1 SPECIFICATION` at p.54 (markdown line 1082, section_ref `#/texts/795`) followed immediately by `## SCHEDULE 2 PRICING SCHEDULE` at line 1084, confirming no body content between them.
- **Template drift in WHEREAS** — anchor: p.2 recital A "Council wishes to secure the provision of security services in Council Buildings" (markdown line 37).
- **Template dates** — anchor: cover page "Dated ... 2020" (markdown line 5), clause 1.3 commencement 1 September 2020 (line 54), recital A period "1 September 2020 to 31 August 2022" (line 37).
- **Process / notice context** (used only to confirm Lot 1 belongs to the 11-lot 6278 framework, not to import lot data into the extraction): `procurement_processes.id=proc-pc-dc1daee2-...` titled "6278 - Building Services Framework - Lots 1-11"; `contract_notices.main_category` carries the three CPVs; `notice_lots` has 11 lot rows with lot_key 1 = "Provision of Fire Risk Assessments".
- **Extraction policy and document hash** — `extraction_policy_version=2026.04.12`, `source_content_hash=8ceea88c7c1944eb0b656cf91f20385e6299204ded72433d1ac454cc7df7eacf` (from `document_extractions` row). Both echoed in `extraction_metadata`.
- **Result contract** — `result.json` written to workdir root, validated against `skills/pdf-extraction/tender-specification/result.schema.json` with `jsonschema.validate` returning VALID.
