---
manifest_version: "1.0.0"
skill_id: "pdf-extraction/tender-specification"
skill_version: "1.0.0"
job_id: "job-aa836a54-a246-408d-913b-414f3f48a276"
session_id: "session-semantic-7129809681c4d96862d5a3bbbfe8fea6154da7ac23d9ecf18432b6763100d592"
execution_mode: "contract"
started_at: "2026-04-18T00:00:00Z"
completed_at: "2026-04-18T00:00:00Z"
outcome_class: "success"
items_processed: 1
items_skipped: 0
items_failed: 0
warnings_count: 6
agent_claimed_pages_visited: 0
agent_claimed_source_url: "https://procontract.due-north.com/RfxResponse/Index?rfxId=ea9d90b2-4833-f111-813c-005056b64545"
agent_claimed_fingerprint_state: "not_applicable_headless_contract"
agent_claimed_locale_state: "en-GB"
agent_claimed_currency_state: "GBP"
---

# Run Manifest

## Summary

Semantic extraction of one tender-specification document scoped to a single lot (Lot 1 — Provision of Fire Risk Assessments) for the London Borough of Bexley procurement process `proc-pc-dc1daee2-772e-f111-813c-005056b64545`. Source was a DOCX artifact (`6278 - Draft Specification - Lot 1.docx`) previously parsed by Docling 2.88.0 into structured markdown (26,459 characters, 17 numbered top-level sections). The skill produced one authoritative `lots[]` entry with 29 requirements, 5 evaluation-criteria themes, 8 service-deliverable product-requirement seeds, 17 technical-standard references, and 18 evidence anchors. No CPV codes were cited in the document text; this is flagged as a warning rather than silently left empty. `target_domain = "lots"` and `extraction_scope = "full_document"` as required by the result contract.

## Steps Taken

1. Loaded the bound SKILL.md and `result.schema.json` from `skills/pdf-extraction/tender-specification/` to confirm the structured-output contract and required field set.
2. Located the `document_extractions` row for `document_extraction_id = 96dee934-a4e8-4958-8578-81349a7ad302` in `state/state.db` and exported the `extracted_markdown` column (535 lines) for processing.
3. Read the backing `raw_documents` row `raw-pc-dc1daee2-...-4246662b...` to confirm document role (`technical_specification`), MIME type (`application/vnd.openxmlformats-officedocument.wordprocessingml.document`), original filename (`6278 - Draft Specification -  Lot 1.docx`), and upstream source URL (ProContract RfxResponse).
4. Cross-referenced the parent notice's `notice_lots` rows to verify that Lot 1 is "Provision of Fire Risk Assessments" and that 11 lots exist on the notice — confirming this specification's scope is deliberately single-lot.
5. Parsed the markdown's numbered section spine (§1 Introduction & Legislative framework → §17 Continuous Improvement) and enumerated: substantive requirements, legislative/standard references, tender-response submission obligations, and service-deliverable categories suitable as product-requirement seeds.
6. Derived `source_markdown_hash` from the authoritative `document_extractions.source_content_hash = e1bc6001a530c7f088f91305fd81fcab07f64da25652d0aeb6200359b54d62f0` (rather than re-hashing an exported copy, which would drift from canonical).
7. Classified each extracted artefact against the JSON schema required fields and populated `evidence_anchor` objects from visible section headings and verbatim anchor strings.
8. Built per-field confidence scores: high confidence on lot identity, title, requirements, and technical standards; low confidence on CPV (absent) and evaluation criteria (no scoring matrix present). Overall `extraction_confidence_bps = 8000` reflects solid lot and standards extraction tempered by missing CPV and missing page-number anchors.
9. Wrote `result.json` to the session workdir and validated it in-process against `result.schema.json` using jsonschema draft 2020-12 — validation passed.
10. Authored this run manifest with the required frontmatter and narrative sections, citing only evidence visible in the structural extraction packet.

## Decisions Made

- **Single-lot emission.** The document title explicitly reads "Draft Specification - Lot 1" and the body addresses only Fire Risk Assessments. Emitting additional lots (Water / Asbestos / etc.) from the notice-level table would fabricate authority this document does not grant; only Lot 1 is emitted, and a `DOCUMENT_SCOPED_TO_SINGLE_LOT` warning records the decision.
- **`cpv_codes` empty, not guessed.** The specification text contains no CPV citations. Guessing CPV (e.g., 71317100 Fire protection consultancy) would violate the "no claims without evidence anchors" rule in the SKILL.md. A `NO_CPV_CODES_IN_DOCUMENT` warning directs downstream stages to rely on notice-level CPV metadata instead.
- **`page_ref` uses section markers.** Docling reports `page_count = 0` for this DOCX source — there are no physical page numbers available. Every `evidence_anchor.page_ref` uses section markers ("§3.4", "§9.1", …) which remain stable against the numbered section spine. A `NO_PAGE_NUMBERS_AVAILABLE` warning documents the convention.
- **Product seeds modelled as service-deliverable categories.** This is a services contract. Rather than omit `product_requirement_seeds` (which would lose downstream signal for scope-matching) or invent goods SKUs (which would misrepresent the contract), seeds describe service-deliverable categories (Standard FRA, Intrusive FRA, FRAEW, PCFRA, Fire Door Check Regime, etc.). A `SERVICE_CONTRACT_SEED_ADAPTATION` warning signals this semantic to downstream product-matching logic.
- **Evaluation criteria emitted without `weighting_text`.** The document has no scoring matrix. Only qualitative submission requirements and thematic evaluation hooks exist. Entries carry `criterion` + `summary` + `evidence_anchor` but omit `weighting_text`; a `NO_FORMAL_EVALUATION_MATRIX` warning records this.
- **`source_markdown_hash` sourced from `document_extractions.source_content_hash`.** Re-hashing the shelled-out markdown would introduce a sqlite CLI trailing-newline drift. The canonical hash from the extraction row is used for deterministic downstream identity.
- **`extraction_scope = "full_document"`.** The job request specifies `extraction_scope = "full_document"` and no chunk plan hash is provided; no `chunk_merged:` suffix is applicable here.

## Warnings

1. **NO_CPV_CODES_IN_DOCUMENT** — No CPV references appear in the extracted markdown; downstream stages should look to notice-level metadata.
2. **NO_PAGE_NUMBERS_AVAILABLE** — Docling page_count is zero for this DOCX; section numbers are used as page refs.
3. **NO_FORMAL_EVALUATION_MATRIX** — No weighted scoring criteria are present; evaluation_criteria entries lack `weighting_text`.
4. **DOCUMENT_SCOPED_TO_SINGLE_LOT** — Specification is authoritative for Lot 1 only; other lots on the notice are not emitted.
5. **SERVICE_CONTRACT_SEED_ADAPTATION** — `product_requirement_seeds` describe service deliverables, not goods.
6. **APPENDIX_1_PRICE_MATRIX_NOT_IN_SCOPE** — Appendix 1 (Price Matrix) and Appendix 1.2 (Property Schedule) are referenced but not included in the extraction packet; quantity hints remain qualitative.

## Evidence

- **Document title and scope.** `"London Borough of Bexley — Fire Risk Assessment — Specification — Contract Commencement April 2026"` appears as the stacked title at the top of the extracted markdown. `document_extractions.document_title = "6278 - Draft Specification -  Lot 1"` confirms single-lot scope. `raw_documents.metadata.document_role_hint = "technical_specification"` confirms role.
- **Contract term.** §1: *"The contract will be for a period of 2 years with an option to extend for a further period of up to 12 months to a maximum term of 3 years"*.
- **Legislative framework (lot technical standards).** §1 Legislative framework list cites RRFSO 2005 (as amended by BSA 2022 s.156), Fire Safety Act 2021, Fire Safety (England) Regs 2022, PAS 9980:2022, BS 9991:2024, BS EN 1838:2024, BS 8674:2025, BS 9792:2025. §3.3 cites BS ISO 19706:2007, IFE Assessor Register, BAFE SP205. §3.6 cites BS 5839. §9.1 cites HSWA 1974, COAR 2012, Equality Act 2010. §9.10 cites RIDDOR 2013.
- **Assessment typology.** §3.2: *"align the survey type to risk using the established Type 1–4 approach (common parts only through to intrusive inspections in dwellings) and Person-Centred FRAs (PCFRA) for residents requiring support. For external walls, determine whether a PAS 9980:2022 FRAEW is required"*.
- **Report contract.** §3.4: *"Reports shall be issued within 10 working days of site completion"*; *"Format must be compatible with TF Cloud"*; *"As part of the tender response the supplier should submit anonymised fire risk assessments for both a standard FRA and intrusive FRA with explanatory notes"*.
- **Residential duties.** §3.5: *"fire door checking regimes (quarterly common parts; annual flat doors)"*.
- **Mandatory DRA.** §10.2: *"completion of a Dynamic Risk Assessment is mandatory and forms a core compliance requirement under this Contract."*
- **Contract management.** §7: *"The Supplier shall appoint a Contract Manager who will act as the primary point of contact"*, plus monthly operational meetings and quarterly contract performance partnership meetings.
- **Subcontracting controls.** §13.1: *"The Supplier may engage subcontractors for specialist services, subject to prior Council approval following a technical and capability assessment."*
- **Parent notice lot structure.** `notice_lots` row `pc-dc1daee2-...-lot-1` title = *"Provision of Fire Risk Assessments"*; the same notice carries lots 2–11 covering adjacent compliance disciplines not addressed by this specification.
- **Source identity.** `raw_documents.id = raw-pc-dc1daee2-772e-f111-813c-005056b64545-4246662b3c0a28a1f82a90f9b5d344d97beabf0a3fee11b58b0a15c867b9f91e`; `source_url = https://procontract.due-north.com/RfxResponse/Index?rfxId=ea9d90b2-4833-f111-813c-005056b64545`; extraction engine `docling 2.88.0`; `source_content_hash = e1bc6001a530c7f088f91305fd81fcab07f64da25652d0aeb6200359b54d62f0`.
- **Schema validation.** `result.json` validated against `skills/pdf-extraction/tender-specification/result.schema.json` using jsonschema draft 2020-12 in-process — all required fields present and typed; `extraction_metadata.target_domain = "lots"` as required.
