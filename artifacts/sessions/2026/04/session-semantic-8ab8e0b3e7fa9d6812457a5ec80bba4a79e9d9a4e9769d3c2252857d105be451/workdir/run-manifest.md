---
manifest_version: 1.0.0
skill_id: pdf-extraction/tender-specification
skill_version: 1.0.0
job_id: job-41785c56-e751-4850-ab58-2cb710ffce0f
session_id: session-semantic-8ab8e0b3e7fa9d6812457a5ec80bba4a79e9d9a4e9769d3c2252857d105be451
execution_mode: contract
started_at: 2026-04-18T02:04:00Z
completed_at: 2026-04-18T02:05:23Z
outcome_class: success
items_processed: 1
items_skipped: 0
items_failed: 0
warnings_count: 5
agent_claimed_pages_visited: 0
agent_claimed_source_url: none
agent_claimed_fingerprint_state: not_applicable
agent_claimed_locale_state: en-GB
agent_claimed_currency_state: GBP
---

# Run Manifest — pdf-extraction/tender-specification

## Summary

Extracted a single-lot tender specification from the Docling markdown for document_extraction_id `92ebd8e4-4fe2-4e71-9807-80d5788fe195` (raw document `raw-pc-dc1daee2-...e27d1a4a1aa6ba92713bd751bee9b61bd44a313cb`). The document is `6278 - Draft T&Cs - Lot 11`, a draft JCT Measured Term Contract 2024 for the London Borough of Bexley covering the Provision of Minor Disability Adaptations (minor building works). Emitted one lot (Lot 11), 10 product-requirement seeds corresponding to the bulleted works list in Section 3, 14 technical-standards references, 2 KPI-based evaluation criteria, and 14 evidence anchors. Overall extraction confidence 8300 bps (83%). Target domain is `lots`; extraction scope is `full_document`.

## Steps Taken

1. Read job parameters from the session invocation: `document_extraction_id = 92ebd8e4-...`, `raw_document_id = raw-pc-dc1daee2-...`, `process_id = proc-pc-dc1daee2-...`, `notice_id = pc-dc1daee2-...`, `stage_run_id = stage-run-43d02438-...`, `target_domain = lots`, `extraction_scope = full_document`.
2. Inspected the workdir and confirmed the only local artifacts were agent-mail MCP storage (not the extraction payload), so queried the daemon state database at `state/state.db` for the `document_extractions` row keyed by the extraction id.
3. Pulled the Docling-produced markdown (`extracted_markdown` column) — 153 lines, document role `technical_specification`, page_count = 0, table_count = 1, section_count = 0, policy version `2026.04.12`.
4. Cross-referenced the parent notice `pc-dc1daee2-...`: buyer `London Borough of Bexley`, title `6278 - Building Services Framework - Lots 1-11`, CPVs `50700000-2 | 50750000-7 | 71315000-9`. Confirmed this document is only Lot 11 of an 11-lot framework.
5. Parsed the markdown section by section (Draft heading, Employer, Contract Form, §§1–13) to extract: contract form (JCT MTC 2024), contract title, term dates, description of works, property portfolio, order workflow, priority categories table, payment provisions, KPIs, statutory/regulatory compliance list, H&S duties, staff competence, waste management, contract documents.
6. Identified 10 minor-adaptation item types from §3 as product-requirement seeds; generated stable `seed_key` values of the form `lot11-<slug>` so that daemon resume paths and future chunk merges remain consistent.
7. Extracted 14 technical standards from §§Contract Form, 7, 9, 10, 11, 12 (JCT MTC 2024; HGCRA 1996; Building Regulations 2010; Building Safety Act 2022; CDM 2015; Gas Safety (Installation and Use) Regulations; BS 7671; Regulatory Reform (Fire Safety) Order; Environmental Protection Act 1990; Data Protection legislation; HSWA 1974; SSIP/CHAS; CSCS; enhanced DBS).
8. Captured evaluation criteria from §8 — two explicit KPIs (90% and 95%). Noted §8 claims "three core" KPIs but only two are defined.
9. Computed `source_markdown_hash` over the markdown text (SHA-256 = `44f65b2240ddf211a82c5326a9621550cefe251987cb11df1a273842d3b8cba6`).
10. Assembled `result.json` in the contract workdir, wrote evidence anchors for every lot/criterion/seed/standard, and validated against `skills/pdf-extraction/tender-specification/result.schema.json` using `jsonschema` — schema valid.

## Decisions Made

- **Single lot emitted.** Although the parent framework has 11 lots, this draft document is explicitly scoped to Lot 11 only. Emitting speculative rows for Lots 1–10 would be fabrication, so `lots` contains one element with `lot_number = "11"`.
- **`cpv_codes` left empty at lot level.** The tender document does not cite any CPV code. Framework-level CPVs on the parent notice are available via `process_context` but cannot be anchored to this document's text. A `NO_CPV_IN_DOCUMENT` warning records the fact so a downstream enrichment step can attach them with its own provenance.
- **Evidence `page_ref` uses section identifiers.** Docling produced this extraction with `page_count = 0` and no inline page markers. Rather than fabricate page numbers, every `page_ref` is a section token (e.g. `section-3`, `title-line`). A `NO_PAGE_METADATA_IN_MARKDOWN` warning records this convention.
- **Seed keys are stable and lot-scoped.** Each seed uses `lot11-<slug>` so that if the daemon retries in `chunk_merged:<plan_hash>` scope on the same input, the keys collide instead of duplicating rows.
- **Product requirements duplicated inside the lot and as top-level seeds.** The schema demands both `lots[].product_requirements[]` and a top-level `product_requirement_seeds[]`. The two arrays share identical `seed_key` values so the daemon can map the denormalised seeds back to their lot context without a second extraction pass.
- **KPI 3 omitted but warned.** Rather than hallucinate a third KPI to reconcile the "three core" wording, only KPI 1 and KPI 2 are emitted and a `KPI_COUNT_MISMATCH` warning is raised.
- **Draft status surfaced.** A `DRAFT_DOCUMENT` warning records that the document is explicitly marked "Draft T&Cs / Draft Contract" so downstream decision layers treat the extraction as provisional.

## Warnings

Five structured warnings are included in `result.json`:

- `KPI_COUNT_MISMATCH` — Section 8 promises three KPIs but defines only two; third KPI absent from document.
- `NO_CPV_IN_DOCUMENT` — No CPV codes cited in the tender text; framework-level CPVs on the parent notice are available as context but not emitted.
- `DRAFT_DOCUMENT` — Document is explicitly a draft T&Cs; all facts are provisional.
- `SINGLE_LOT_OF_MULTI_LOT_FRAMEWORK` — Parent notice covers Lots 1–11; this extraction only sees Lot 11.
- `NO_PAGE_METADATA_IN_MARKDOWN` — Docling did not produce page numbers (page_count = 0); evidence anchors use section-level refs in the `page_ref` slot.

## Evidence

- Document identity (`title-line`): `6278 - Draft T&Cs - Lot 11` — taken from `document_extractions.document_title` for id `92ebd8e4-4fe2-4e71-9807-80d5788fe195`.
- Contract form (`contract-form`): "This Contract shall be executed under the JCT Measured Term Contract 2024" (markdown line 9).
- Lot title (`section-1`): "Measured Term Contract – Provision of Minor Disability Adaptations (minor building works)" (line 19).
- Term (`section-2`): "1 November 2026 – 31 October 2030 ... 4 Years Fixed Term ... No extension option." (lines 25–31).
- Scope (`section-3`): "The Contract covers the provision of minor disability adaptations (under £1k) to residential properties within the London Borough of Bexley." (line 35) followed by a 10-bullet works list (lines 39–48) — source of every product-requirement seed.
- Priority matrix (`section-6`): markdown table at lines 69–74 — source of response-time requirement entry.
- Payment provisions (`section-7`): Housing Grants, Construction and Regeneration Act 1996 reference (line 80); 30-day final date for payment and 6-month invoice window (lines 87–89).
- KPIs (`section-8`): "three core Key Performance Indicators" (line 93) versus only KPI 1 (90%, line 97) and KPI 2 (95%, line 103) defined — source of `KPI_COUNT_MISMATCH` warning.
- Statutory compliance (`section-9`): 8-item bulleted list at lines 111–118 — source of the statute/standard references in `technical_standards`.
- H&S (`section-10`): HSWA 1974, CDM 2015, Principal Contractor, SSIP/CHAS (lines 124–127).
- Staff competence (`section-11`): CSCS cards, enhanced DBS (lines 133–136).
- Waste (`section-12`): Environmental Protection Act 1990 and registered waste carrier (lines 140–142).
- Contract documents (`section-13`): 6-item list at lines 148–153 — enumerates the contract-pack components and supports the top-level `requirements` list.
- Parent notice context (daemon state, not document text): `contract_notices.title = "6278 - Building Services Framework - Lots 1-11"`, buyer `London Borough of Bexley`, CPVs `50700000-2 | 50750000-7 | 71315000-9` — source of `SINGLE_LOT_OF_MULTI_LOT_FRAMEWORK` and `NO_CPV_IN_DOCUMENT` warnings.
- Markdown hash basis: SHA-256 over the markdown text returned by `SELECT extracted_markdown FROM document_extractions WHERE id = '92ebd8e4-...'` → `44f65b2240ddf211a82c5326a9621550cefe251987cb11df1a273842d3b8cba6`; persisted into `extraction_metadata.source_markdown_hash`.
- Schema validation: `result.json` validated against `skills/pdf-extraction/tender-specification/result.schema.json` with `jsonschema.validate` — no errors.
