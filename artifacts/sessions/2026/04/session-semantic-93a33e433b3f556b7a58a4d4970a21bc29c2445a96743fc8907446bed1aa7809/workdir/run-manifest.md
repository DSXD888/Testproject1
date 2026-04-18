---
manifest_version: "1.0.0"
skill_id: "pdf-extraction/tender-specification"
skill_version: "1.0.0"
job_id: "job-2763b78c-ebd6-4b81-a787-0f0041dc8d4a"
session_id: "session-semantic-93a33e433b3f556b7a58a4d4970a21bc29c2445a96743fc8907446bed1aa7809"
execution_mode: "contract"
started_at: "2026-04-18T00:00:00Z"
completed_at: "2026-04-18T00:00:00Z"
outcome_class: "FULL_SUCCESS"
items_processed: 1
items_skipped: 0
items_failed: 0
warnings_count: 5
agent_claimed_pages_visited: 0
agent_claimed_source_url: "local:document_extractions/c2132e95-8b56-4994-9350-3c84ff93e564"
agent_claimed_fingerprint_state: "not_applicable"
agent_claimed_locale_state: "en-GB"
agent_claimed_currency_state: "GBP"
---

## Summary

Executed `pdf-extraction/tender-specification` against document_extraction `c2132e95-8b56-4994-9350-3c84ff93e564` - a 4728-byte Docling markdown of the "6278 - Draft Specification - Lot 4 - Provision of Fire Door Surveys" (London Borough of Bexley Building Services Framework 6278). Emitted one lot (Lot 4) with 9 product requirement seeds, 7 technical standards, and 15 evidence anchors. No CPV codes and no explicit evaluation criteria are present in this specification; both are flagged as warnings for the daemon to source from the notice or ITT pack. Result written to `result.json` with `extraction_metadata.target_domain = "lots"` and `extraction_scope = "full_document"`.

## Steps Taken

1. Read `document_extractions.extracted_markdown` for id `c2132e95-8b56-4994-9350-3c84ff93e564` from `state.db`. Confirmed document_role = `technical_specification`, page_count = 0, table_count = 0, section_count = 13, expected_overlays = `["pdf-extraction/tender-specification"]`, extraction_policy_version = `2026.04.12`, extraction_scope param = `full_document`.
2. Cross-referenced notice-level lot metadata from `notice_lots` for notice `pc-dc1daee2-772e-f111-813c-005056b64545`. Verified the document corresponds to Lot 4 of framework 6278 (existing notice row: `4 | Fire Door Specification (Lot 4 of Building Services Framework 6278)`). Did not copy any notice-row content into the extraction; used only to confirm lot numbering.
3. Parsed the TOC (Contents block) and verified 8 top-level sections plus Appendices A-C, with TOC-declared pages 3-6.
4. Extracted lot identity and scope from the title block, §1 Introduction & Purpose, and §2 Scope of Works.
5. Extracted requirements from §§3.1-3.5 (Fire & Smoke Ratings, Seals & Smoke Control, Hardware & Ironmongery, Glazing, Installation), §4 (Inspection, Testing & Maintenance), §5 (Documentation & Record Keeping), §6 (Contractor Competence & H&S), §7 (Handover & QA), and Appendices A (inspection checklist), B (asset schedule columns), and C (certificate fields).
6. Derived 9 product_requirement_seeds (FD-rated door set; FD30S flat entrance door; intumescent/cold smoke seals; cold smoke brush-type seals; BS EN 1154 self-closing device; fire-rated hinges; fire door signage; fire-rated glazing; fire door inspection/test/maintenance service). Each seed carries a stable `seed_key` prefixed `bexley-6278-lot4-` and a dedicated evidence_anchor.
7. Catalogued 7 technical_standards (BS EN 1154; Fire Safety (England) Regulations 2022; Fire Safety Order; UK Building Regulations; UK HSE guidance; generic British Standards; FD30/FD60/FD90/FD120 and FD30S/FD60S classifications). Each cites a section anchor from the document itself.
8. Computed SHA-256 of the exact `extracted_markdown` string (UTF-8 encoded, 4728 bytes) = `cd57446f5db5096906410cb02b414f74a5851ad1865d493797834985ad3171dc` and placed it in `extraction_metadata.source_markdown_hash`.
9. Wrote `result.json` in the session workdir and validated it against `/skills/pdf-extraction/tender-specification/result.schema.json` via `jsonschema.validate` - returned VALID.
10. Authored this run manifest.

## Decisions Made

- **Single-lot output.** This PDF is the specification document for a single lot (Lot 4). Although framework 6278 has 11 lots in the notice record, the document contradicts none of that metadata; it scopes only Lot 4. Emitting one lot is correct per SKILL.md guidance ("do not treat pre-existing lot rows as authoritative truth if the tender document contradicts them" - here there is no contradiction, but only Lot 4 content is present).
- **Evaluation criteria = empty array with warning.** The document is a technical specification, not an ITT / selection questionnaire. It contains no weightings, scoring tables, or MEAT criteria. Emitting an empty `evaluation_criteria` array plus warning `EVALUATION_CRITERIA_ABSENT` preserves auditability without fabricating criteria.
- **CPV codes = empty array with warning.** No CPV code appears in the document text. CPV assignment for Lot 4 should come from the notice or the ITT's CPV schedule; flagged as warning `CPV_CODES_NOT_PRESENT_IN_DOCUMENT` rather than invented.
- **Page references in section-based form.** `document_extractions.page_count = 0` (Docling did not emit page boundaries for this PDF). Evidence anchors therefore carry `page_ref` values derived from the document's own Contents block (p.3 / p.4 / p.5 / p.6) with section anchors as the primary locator, flagged via warning `PAGE_ANCHORS_APPROXIMATE`.
- **Draft document handling.** The filename says "Draft Specification". All extracted requirements are treated as authoritative for the purposes of this extraction, but the warning `DOCUMENT_MARKED_DRAFT` tells downstream consumers that re-extraction may be required once a finalised version arrives.
- **Generic standards preserved, not over-specified.** Where the document says "relevant British Standards" or "relevant EN standards in force" I did not guess at BS 8214 / BS EN 1634-1 / BS 476-22 / BS 5839 etc., even though those are the canonical UK fire-door standards. Warning `STANDARDS_PARTIALLY_NAMED` signals this deliberate non-expansion.
- **Product seeds include a service seed.** `bexley-6278-lot4-fire-door-inspection-service` is typed as `service_inspection` (not a good) because inspection/testing/maintenance is explicitly in-scope for the lot. This matches the "Provision of Fire Door Surveys" framing in the notice row and keeps the lot's recurring service component addressable for product matching.
- **Confidence calibration.** `extraction_confidence_bps = 8400` overall. Lot identity/description are high (95/90) because the lot number is unambiguous and the scope is explicit. Product requirements at 82 and standards at 88 reflect confidence in extraction completeness vs the document's brevity; evaluation_criteria at 20 and CPV at 10 reflect the absence of those fields in the source.

## Warnings

Five warnings in `result.json`:

- `EVALUATION_CRITERIA_ABSENT` - no explicit award criteria / weightings in the document.
- `CPV_CODES_NOT_PRESENT_IN_DOCUMENT` - no CPV codes in the document.
- `DOCUMENT_MARKED_DRAFT` - document filename contains "Draft Specification".
- `STANDARDS_PARTIALLY_NAMED` - several standards cited generically without specific BS/EN numbers beyond BS EN 1154.
- `PAGE_ANCHORS_APPROXIMATE` - Docling produced `page_count = 0`; page references are reconstructed from the document's own Contents block.

## Evidence

All claims in `result.json` trace back to observable features of the Docling markdown (`document_extractions.id = c2132e95-8b56-4994-9350-3c84ff93e564`, `source_content_hash = 51d4c062...`):

- **Lot identity** - anchored to the cover title block ("London Borough of Bexley / Fire Door Specification / Contract Commencement / April 2026") and the source filename recorded in `document_extractions.document_title` ("6278 - Draft Specification - Lot 4 - Provision of Fire Door Surveys"). Cross-confirmed against `notice_lots` row `4 | Fire Door Specification (Lot 4 of Building Services Framework 6278)`.
- **Scope & Purpose claims** - §1 Introduction & Purpose ("supply, installation, inspection and maintenance of fire doors and fire door sets") and §2 Scope of Works (bulleted scope list including TF Cloud / Civica).
- **FD30/FD60/FD90/FD120 ratings** - §3.1 ("Doors achieve minimum performance ratings as specified within design drawings or fire strategy documents: FD30, FD60, FD90, or FD120").
- **FD30S flat entrance door** - §3.1 ("flat entrance doors must be a minimum FD30S rating (30 minutes fire and smoke resistance)").
- **Intumescent & cold smoke seals** - §3.2 ("Intumescent and cold smoke seals must be continuous and undamaged") and ("cold smoke brush-type seals to achieve FD30S or FD60S compliance").
- **BS EN 1154 self-closing devices** - §3.3 ("Include self-closing devices (BS EN 1154)").
- **Fire-rated hinges and tested door set assembly** - §3.3 ("All hardware must be fire-rated and tested to relevant EN standards in force. ... Hinges ... All hardware must form part of the tested and approved door set assembly.").
- **Fire door signage** - §3.3 ("Compliant signage ('Fire Door - Keep Shut / Locked')").
- **Fire-rated glazing** - §3.4 ("Any glazing must use fire-rated glass installed with approved fire-resistant glazing systems.").
- **Inspection cadence (annual / quarterly)** - §4 ("inspected, as a minimum, annually" / "communal doors must be checked quarterly in accordance with Fire Safety (England) Regulations 2022").
- **Inspection checklist items** - Appendix A (bulleted list including "Gaps within tolerance (<=4mm)").
- **Asset schedule columns** - Appendix B (numbered column list 1-9).
- **Certificate fields** - Appendix C (Certificate No, Door ID, Location, Installer, Date Installed, Fire Rating, Smoke Seal Yes/No, Inspector Signature, Date of Inspection).
- **Statutory framework references** - §1 ("Health and Safety Executive (HSE) guidance, Building Regulations, and relevant British Standards, and to meet all statutory duties under the Fire Safety Order").
- **TF Cloud / Civica platform** - §2 scope list and §5 Documentation & Record Keeping ("All documents must be uploaded to TFCloud / Civica for full audit trail and compliance verification").
- **Source markdown hash** - SHA-256 of the exact `extracted_markdown` cell from `state.db` row `c2132e95-8b56-4994-9350-3c84ff93e564` computed during this run = `cd57446f5db5096906410cb02b414f74a5851ad1865d493797834985ad3171dc` (UTF-8, 4728 bytes).
- **Schema validation** - `result.json` validated against `/skills/pdf-extraction/tender-specification/result.schema.json` (jsonschema 2020-12 draft) - result: VALID.
