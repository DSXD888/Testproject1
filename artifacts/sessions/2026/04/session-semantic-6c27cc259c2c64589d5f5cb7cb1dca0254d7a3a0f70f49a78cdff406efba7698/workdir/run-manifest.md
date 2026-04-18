---
manifest_version: 1.0.0
skill_id: pdf-extraction/tender-specification
skill_version: 1.0.0
job_id: job-a73cd56e-b274-40ab-a449-6dd278bf8d56
session_id: session-semantic-6c27cc259c2c64589d5f5cb7cb1dca0254d7a3a0f70f49a78cdff406efba7698
execution_mode: contract
started_at: 2026-04-18T02:47:00Z
completed_at: 2026-04-18T02:47:30Z
outcome_class: partial_success_scope_mismatch
items_processed: 1
items_skipped: 0
items_failed: 0
warnings_count: 4
agent_claimed_pages_visited: 10
agent_claimed_source_url: /FileUploads/ViewAttachment?attachmentId=36ca52fe-1037-f111-813c-005056b64545&entityTypeId=9b214c2e-2d7d-432b-bced-4df247225fb3&attachmentTypeId=Rfx
agent_claimed_fingerprint_state: not_applicable_contract_mode
agent_claimed_locale_state: en-GB
agent_claimed_currency_state: not_applicable
---

## Summary

Processed Docling markdown for raw document `raw-pc-978a1774-1037-f111-813c-005056b64545-5e5da3be6f2e59eb22bcf1bf610fc859d1855fe1bee50c2bd7b7efd0e10c2211` (source file `Volume I Instructions and Conditions final January 2026.pdf`, 10 pages, 41 sections, source markdown hash `982304ae02cae9dd63283bb146359198706f2aa239778930cba54ec092ab43f3`, extraction policy `2026.04.12`). The document is the procedural overlay of the ECMWF Copernicus C3S2_413_bis ITT and explicitly defers all lot, product, evaluation, CPV, and technical-standard detail to a companion document, Volume II Specification (§4.5). Because the skill contract forbids asserting lot splits, product-requirement seeds, or standards without an anchored in-document evidence citation, the authoritative output is an empty `lots` array, an empty `product_requirement_seeds` array, four warnings, and five evidence anchors that localise the scope-mismatch rationale. Outcome class `partial_success_scope_mismatch` reflects that the document was read and understood, but contains no semantic content the tender-specification overlay is designed to harvest.

## Steps Taken

1. Loaded the bound execution parameters: `document_extraction_id=e1c89592-1e9a-48ce-a315-eead7eef81d2`, `target_domain=lots`, `extraction_scope=full_document`, `stage_run_id=stage-run-43d02438-92da-4df3-bc14-2ff454300565-proc-pc-978a1774-1037-f111-813c-005056b64545`.
2. Retrieved Docling markdown (47,928 chars) and structural metadata (sections, tables) from `document_extractions` row keyed by the extraction id. Recorded the declared `document_role='technical_specification'` and `expected_overlays_json=["pdf-extraction/tender-specification"]`.
3. Joined `raw_documents` to confirm the source file is `Volume I Instructions and Conditions final January 2026.pdf` and recovered `source_content_hash=982304ae02cae9dd63283bb146359198706f2aa239778930cba54ec092ab43f3` for the result `source_markdown_hash` field.
4. Retrieved notice-level context from `contract_notices` (title `C3S2_413_bis Lot 2 Maintenance and Extension of C3S Windstorm and Storm Surge Services`, buyer `ECMWF`) and cross-referenced `notice_lots` (two pre-existing lot rows: Lot 1 Windstorm, Lot 2 Storm Surge) strictly for sanity, not as authoritative truth.
5. Read the full markdown end-to-end. Mapped the ITT structure surfaced in §4: Volume I (this document) / Volume IA timetable / online questionnaire / Contract Notice / Volume II Specification / Volume III templates / Volume IV forms / Volume V Framework Agreement / §4.9 checklist.
6. Searched the markdown for anchor-triggering content: lot headings, requirement tables, KPI/Milestone concrete instances, evaluation weight tables, CPV codes, EN/ISO/IEC standards. None found. The token `Lot` does not appear anywhere in the document body; `CPV` does not appear; no EN/ISO/IEC standard reference exists; no evaluation weighting is stated.
7. Located the pivotal scope-deferral anchors: §4.5 Volume II Specification (declares spec is external), §4 Structure of this ITT (enumerates the document set), §3.3 Evaluation (declares evaluation is against Volume II and portal questionnaire), §4.6 Volume III (confirms pricing/deliverable templates are separate artefacts).
8. Decided to emit zero lots, zero product requirement seeds, zero technical standards, low `extraction_confidence_bps` (1500), four warnings, and five anchor citations. Flagged document-role reclassification as a fourth warning so the daemon's routing policy can demote this document off the tender-specification overlay on future runs.
9. Wrote `result.json` to the session workdir root, then re-loaded it via a JSON parser to confirm syntactic validity against the contract.
10. Wrote this `run-manifest.md` with all required frontmatter keys plus the five required narrative sections.

## Decisions Made

- **Empty `lots` array, not an invented placeholder.** The skill's critical rule forbids asserting a lot split without a cited in-document anchor; Volume I has no lot-level content. Emitting an empty array with warnings is the honest output and avoids polluting the notice's authority with downstream-invented rows. Notice-level `notice_lots` (Lot 1 Windstorm, Lot 2 Storm Surge) already exist from upstream ingestion and are unaffected by this empty payload.
- **Empty `product_requirement_seeds` array.** Nothing in the document names a product, quantity, performance attribute, or technical feature eligible to become a `product_candidates` seed. The definitions for Deliverable, KPI, Milestone, Performance Target in §2 are definitional scaffolding for Volume II, not concrete requirement text.
- **`extraction_confidence_bps = 1500` (15%).** Reflects high confidence that the document was correctly parsed and fully read, combined with near-zero extractable semantic content for this overlay. A higher score would misrepresent completeness; a lower score would misrepresent read fidelity.
- **`outcome_class = partial_success_scope_mismatch`.** Not `failure` (the skill ran cleanly, the contract is valid, the document was read), not `success` (zero lot/seed/standards extractions). The new class name communicates "the document is simply out of scope for the bound overlay" so the daemon's validator can treat this as a routing problem, not a skill defect.
- **Warning `DOCUMENT_ROLE_RECLASSIFICATION_SUGGESTED`.** Raised because the daemon's `document_extractions.document_role='technical_specification'` is a misclassification. Surfacing this as an explicit warning creates an observable signal for the semantic extraction router so future Volume-I-shaped documents stop getting bound to this skill.
- **No guessed CPV or ISO standards.** The markdown contains no CPV references and no standard identifiers; the inherited notice-level CPV signal was deliberately not injected into lot output because it is notice-scoped, not evidence-anchored in this document.

## Warnings

- `SKILL_SCOPE_MISMATCH_VOLUME_I` — Document is the administrative/procedural Volume I and defers all technical lot content to Volume II Specification per §4.5. No lot-level semantic content is anchorable from this document alone. Evidence refs: p.8:section-4.5, p.8:section-4, p.5:section-3.3.
- `LOT_STRUCTURE_ABSENT_FROM_VOLUME_I` — The token "Lot" does not appear in the document body. Volume I treats the ITT uniformly without elaborating the two-lot split that is visible in notice-level metadata (Lot 1 Windstorm, Lot 2 Storm Surge). Evidence ref: p.8:section-4.
- `EVALUATION_CRITERIA_REFERENCE_ONLY` — §3.3 Evaluation states evaluation is against "Tenderer's responses to ECMWF's requirements (Volume II of this ITT) and the questions in the eProcurement Portal". No weighted criteria, scoring rubric, or award method detail exists in this document. Evidence ref: p.5:section-3.3.
- `DOCUMENT_ROLE_RECLASSIFICATION_SUGGESTED` — `document_extractions.document_role='technical_specification'` is inconsistent with observed content (eligibility, tender conditions, submission mechanics, clarification procedure, ECMWF status). Suggested reclassification: `tender_instructions` or `conditions_of_tender`. Evidence refs: p.3:section-3, p.9:section-5, p.10:section-6, p.10:section-7.

## Evidence

All warnings, decisions, and the empty-output stance are anchored to the following observations directly present in the Docling markdown payload loaded from `document_extractions.extracted_markdown` (hash `982304ae02cae9dd63283bb146359198706f2aa239778930cba54ec092ab43f3`):

- **p.1 title-block**: `VOLUME I: Instructions for respondents and Conditions of tender` — `ISSUED BY: ECMWF Administration Department Procurement Section` — `Date: 30 January 2026` — `Version: Final`. Establishes the document's own self-identification as the instructions volume, not the specification volume.
- **p.8 §4 Structure of this ITT**: The ITT is structured in several parts, described below, all of which form the ITT. All documents that Tenderers will need are held on the eProcurement Portal.
- **p.8 §4.1**: `Volume I - Instructions for respondents and Conditions of Tender (this document) Contains an introduction to the ECMWF and the Copernicus Programme, lists the conditions of Tender and provides instructions on how to respond.` Establishes that Volume I's scope excludes technical specification.
- **p.8 §4.5 Volume II Specification**: `This contains detailed technical specifications and requirements for each ITT.` Primary anchor for `SKILL_SCOPE_MISMATCH_VOLUME_I` — the specification overlay target lives in a different document.
- **p.8 §4.6 Volume III Templates for Suppliers to Complete**: `Volume III A - Pricing and deliverables; Volume III B - Templates for Tenderers to complete.` Anchors that pricing/deliverable artefacts are also separate.
- **p.5 §3.3 Evaluation**: `ECMWF will evaluate the Tender based on the Tenderer's responses to ECMWF's requirements (Volume II of this ITT) and the questions in the eProcurement Portal as well as Tenderer's compliance with paragraphs 3, 4, 5, 6 and 7 of this Volume I.` Anchor for `EVALUATION_CRITERIA_REFERENCE_ONLY`.
- **Token-level negative evidence**: full-body search over 47,928 markdown characters returns zero occurrences of the tokens `Lot`, `CPV`, `EN ISO`, `IEC`, `BS EN`, `weighting`, `scoring criteria`, or a product quantity expression with a unit. Anchors the `LOT_STRUCTURE_ABSENT_FROM_VOLUME_I` warning and the decision to emit empty `lots` / `product_requirement_seeds` arrays.
- **Daemon appendix hooks** (to be merged in by the daemon's machine-observed evidence stage): `document_extraction_id=e1c89592-1e9a-48ce-a315-eead7eef81d2`, `raw_document_id=raw-pc-978a1774-1037-f111-813c-005056b64545-5e5da3be6f2e59eb22bcf1bf610fc859d1855fe1bee50c2bd7b7efd0e10c2211`, `source_content_hash=982304ae02cae9dd63283bb146359198706f2aa239778930cba54ec092ab43f3`, `page_count=10`, `section_count=41`, `table_count=1`, `policy_version=2026.04.12`, `expected_overlays_json=["pdf-extraction/tender-specification"]`, result artefact path `workdir/result.json` (4091 bytes, schema-valid per `pdf-extraction/tender-specification/result.schema.json`).
