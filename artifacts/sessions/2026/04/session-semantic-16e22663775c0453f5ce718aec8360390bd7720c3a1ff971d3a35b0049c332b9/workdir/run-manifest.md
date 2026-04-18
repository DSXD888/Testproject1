---
manifest_version: 1
skill_id: pdf-extraction/tender-specification
skill_version: 1.0.0
job_id: job-50b6a10a-2ccf-43a6-9ab3-de5bfe51b703
session_id: session-semantic-16e22663775c0453f5ce718aec8360390bd7720c3a1ff971d3a35b0049c332b9
execution_mode: contract
started_at: 2026-04-18T03:04:18Z
completed_at: 2026-04-18T03:12:00Z
outcome_class: success_with_warnings
items_processed: 2
items_skipped: 0
items_failed: 0
warnings_count: 10
agent_claimed_pages_visited: 0
agent_claimed_source_url: none (contract-mode skill; input was Docling markdown from document_extractions.id=09509590-3647-4725-ab12-3439f77738e4)
agent_claimed_fingerprint_state: n/a
agent_claimed_locale_state: en-GB
agent_claimed_currency_state: GBP
---

## Summary

Extracted lot-level semantic payload for the 6278 Building Services Framework T&Cs document covering Lots 8 and 9 (London Borough of Bexley). The input document (`document_extractions.id=09509590-3647-4725-ab12-3439f77738e4`, title "6278 - Draft T&Cs - Lots 8 & 9", role `technical_specification`, 64 pages, 253 sections) is a draft contract / terms-and-conditions template. Its Part 1 (Form of Contract), Part 2 (General T&Cs) and Part 3 (Special Conditions, S1-S10) are populated; its Schedule 1 (Specification), Schedule 2 (Pricing Schedule), Schedule 3 (ITT) and Schedule 4 (Contractor's Tender) are placeholder headings only.

Two lot records were emitted (Lot 8 = Planned and Preventative Corporate Building Maintenance (PPM) and Remedial Repairs excluding full replacements; Lot 9 = Reactive Repairs to Corporate Buildings), three product requirement seeds (Lot 8 PPM, Lot 8 Remedial, Lot 9 Reactive), and an extraction confidence of 5500 bps. Nine explicit technical standards were captured across both lots with anchored evidence (BSI/ISO Quality Standards, Good Industry Practice, CDM Regulations 2015, BSI Group codes of practice / European directive standards, BCO "Good Practice in Selection of Construction Materials", HSWA 1974 / HSE guidance, COSHH register, UK GDPR / DPA 2018 Processor role, Scheme for Construction Contracts / CIMAR Rules). Ten warnings explain what is deferred to the live specification pack (Schedule 1 stub, Pricing Schedule stub, no CPV in body, no evaluation criteria in body, template dates, KPI / Service Credits deferral, DBS level not fixed, lot attribution derived from document title, Works Order protocol not detailed, approximate pagination).

## Steps Taken

1. Read the bound `SKILL.md` and `result.schema.json` to confirm the result contract (target_domain = "lots", required lot and seed fields, evidence-anchor requirement for technical standards).
2. Located the Docling-produced markdown in `state.db.document_extractions` row `09509590-3647-4725-ab12-3439f77738e4` (166,744 bytes of markdown, 9,849 bytes of sections JSON, page_count=64, section_count=253, table_count=2, role=technical_specification, extraction_policy_version=2026.04.12).
3. Dumped the markdown to `/tmp/doc_md.md` and computed sha256 `55a5a7cc58029cba4a3d40b2ef0a828f3cb49d8315f3f0a5fd7711f9ba29a2ec` for `source_markdown_hash`.
4. Structurally scanned the markdown: identified Part 1 Form of Contract, Part 2 General T&Cs (Sections A-F), Part 3 Special Conditions (Sections S1-S10) and four Schedule stubs at the end of the file.
5. Confirmed that "Lot 8" / "Lot 9" / "Lots 8 & 9" do not appear anywhere in the document body - the attribution is carried only by the document title and the upstream raw-document filename.
6. Cross-referenced `state.db.notice_lots` for Framework 6278 (notice_id `pc-dc1daee2-772e-f111-813c-005056b64545`) to recover authoritative lot titles and notice-level lot descriptions for Lots 8 and 9 (used for cross-reference only, per SKILL.md: notice-level rows are not authoritative if contradicted by the document - which they are not here).
7. Inspected a prior tender-specification extraction against the same-framework Lot 6 T&Cs (`document_semantic_extractions` row keyed on skill `pdf-extraction/tender-specification`) to align with established field shapes for this document family.
8. Harvested document-anchored technical standards from the Definitions block (pp.6-11), Section B3 (p.26), Section D6 (p.27), Section E1 (p.33) and Part 3 Special Conditions (pp.61-66). Each standard_ref carries an evidence_anchor per the `standards_require_evidence_anchors` global assertion.
9. Emitted three product_requirement_seeds: (i) Lot 8 PPM (Annual Fee Services against Appendix B PPM Schedule), (ii) Lot 8 Remedial Works excluding full replacements (Works Orders under S2.3/S2.4), (iii) Lot 9 Reactive Repairs (timely-supply clause B1.3 and Works Order dispatch). Summaries reference notice-level lot descriptions (gutter clearance / boarding-up / inspections) for contextual completeness and flag Schedule 1 as a stub.
10. Emitted ten warnings mapping each missing-content area to the relevant Schedule stub or notice-level cross-reference.
11. Wrote `result.json` at the workdir root and validated required top-level and nested keys via an in-process JSON-schema-shape check (all required fields present; target_domain == "lots"; all technical_standards carry evidence anchors).

## Decisions Made

- **Two lots, not one.** The document name and the notice-level lot set identify this T&Cs pack as covering both Lot 8 and Lot 9. Emitting a single lot would lose the Lot 9 attribution and would miss the distinct product seeds (PPM + remedial vs reactive repairs).
- **Derive lot identity from the document title, not the body.** The body says only "provision of building services". The only in-document evidence that ties this T&Cs pack to Lots 8 and 9 is the document title. This is explicitly called out in `LOT_ATTRIBUTION_FROM_DOCUMENT_TITLE_ONLY`.
- **No CPV codes, no evaluation criteria.** The document carries neither in the body; emitting any would fabricate content. Warnings (`CPV_CODES_NOT_IN_DOCUMENT`, `EVALUATION_CRITERIA_NOT_IN_DOCUMENT`) document the deferral.
- **Extraction confidence = 5500 bps.** Lower than a specification with live Schedule 1 content (prior Lot 6 T&Cs: 6500 bps) because this document covers two lots with a shared T&Cs body and no body-level lot attribution.
- **Technical standards are lot-generic, not lot-specific.** The same T&Cs pack applies to both lots, so the same standards block is reproduced on Lot 8 and Lot 9 with anchors. The `S4.1` Unacceptable Practices list is flagged only on Lot 9 (reactive repairs involve more premises-level operational exposure) but this is a soft preference, not a hard document distinction.
- **Seed keys follow the `bexley-6278-lot{N}-<topic>` convention** to match prior seeds in this framework (Lot 6 cleaning seeds) and remain stable across chunked / retry runs per the SKILL.md Resume Strategy.
- **No CPV codes emitted at top-level seed level.** The notice-level CPVs (50700000-2, 50750000-7, 71315000-9) are framework-wide, not lot-specific; the warning explicitly calls this out and defers to the notice record.
- **Page references marked approximate.** Evidence anchors use textual section references (clause numbers, definition labels) plus approximate page labels derived from section placement and the 64-page total; `DOCUMENT_PAGINATION_APPROXIMATE` flags this so the daemon does not treat the page numbers as Docling-verified.

## Warnings

Ten warnings emitted in `result.json`, each with at least one evidence_ref:

1. `SCHEDULE_SPECIFICATION_STUB` - Schedule 1 is a placeholder heading only; authoritative service scope for Lots 8 and 9 lives outside this document.
2. `PRICING_SCHEDULE_STUB` - Schedule 2 is a placeholder heading only; no Contract Price, rates or indexation in this document.
3. `EVALUATION_CRITERIA_NOT_IN_DOCUMENT` - ITT/Schedule 3 is a stub; award criteria must come from the published notice or ITT pack.
4. `CPV_CODES_NOT_IN_DOCUMENT` - No CPV codes in body; framework-level CPVs do not resolve Lot 8 / Lot 9 specifically.
5. `CONTRACT_VALUE_AND_DATE_PLACEHOLDERS` - Dated 2020 boilerplate; `[CONTRACTOR]` placeholders; 1 Sep 2020 - 31 Aug 2024 template period.
6. `KPI_AND_SERVICE_CREDITS_DEFERRED` - KPI indicators and Service Credit tables live in Schedule 4 to the Specification (stub).
7. `DBS_LEVEL_UNSPECIFIED` - DBS level delegated to Specification; not fixed in the T&Cs.
8. `LOT_ATTRIBUTION_FROM_DOCUMENT_TITLE_ONLY` - Lot 8 / Lot 9 identity taken from the document title, not the body.
9. `WORKS_ORDER_PROTOCOL_NOT_DETAILED` - Works Orders referenced (S2.3) but dispatch / valuation / sign-off protocol lives in Schedule 1 / Schedule 2.
10. `DOCUMENT_PAGINATION_APPROXIMATE` - Docling produced markdown and section list; page references in anchors are approximate (textual-position derived).

No failure codes (`AGENT_OUTPUT_CONTRACT_INVALID`, `AGENT_SUSPECT_OUTPUT`, `AGENT_SKILL_FAILED`, `AGENT_RUN_FAILED`, `AGENT_PARTIAL_FAILURE`) apply; outcome class is `success_with_warnings`.

## Evidence

All substantive claims in `result.json` are anchored to in-document evidence; the 23 top-level `evidence_anchors` cover every cited clause. Key anchors:

- **Document identity and lot attribution**: Document title "6278 - Draft T&Cs - Lots 8 & 9" (from `document_extractions.document_title`); WHEREAS (A) "RELATING TO THE PROVISION OF BUILDING SERVICES" (p.1 front matter); no in-body "Lot 8" / "Lot 9" reference (`grep -n -i -E "lot ?8|lot ?9" /tmp/doc_md.md` returned zero matches).
- **Lot 8 PPM product seed**: Definitions "Annual Fee Services" => "services described in the Appendix B to the Specification (PPM Schedule)" (p.6); clause S2.4 "The Contractor shall service the Plant and maintain it in good working order" (p.61).
- **Lot 8 Remedial seed**: clause S2.3 references "Works Orders" (p.61); clause S2.4 (p.61); clause S2.5 warrants good and workmanlike manner (p.61).
- **Lot 9 Reactive seed**: clause B1.1 "supply the Services during the Contract Period in accordance with the Council's requirements as set out in the Specification" (pp.15-16); clause B1.3 "Timely supply of the Services shall be of the essence of the Contract" (p.16); clause S4.1 unacceptable practices list (p.62).
- **Technical standards anchors** (9 standards, each with a page_ref and anchor_text in `result.json`): Quality Standards definition (p.11) + clause B3.1 (p.26); Good Industry Practice definition (p.9); CDM Regulations definition (p.7) + clause S2.3 (p.61); BSI Group codes / European directive reference at clause S2.6.2 (p.62); BCO "Good Practice in Selection of Construction Materials" at clause S2.6.3 (p.62); HSWA 1974 / HSE guidance at clauses S2.2.2 and S3.5 (pp.61-62); COSHH register at clause S3.4 (p.62); Controller/Processor split at clause E1.1 (p.33); CIMAR Rules at clauses S10.5-S10.6 (p.66); Electronic Invoicing Regulations at clause C1.4 (p.22).
- **Schedules-are-stubs evidence**: "SCHEDULE 1 SPECIFICATION / SCHEDULE 2 PRICING SCHEDULE / SCHEDULE 3 THE COUNCIL'S INVITATION TO TENDER / SCHEDULE 4 THE CONTRACTOR'S TENDER" - four consecutive heading-only lines at document end (p.64 based on section placement; cf. `SCHEDULE 1 SPECIFICATION` at line 1231 of the extracted markdown, directly followed by three further Schedule headings with no body content).

Daemon appendix fields that should be populated from observed facts (for reference): `document_extraction_id=09509590-3647-4725-ab12-3439f77738e4`; `source_content_hash=0381d4cf609721dafaf145cf462528e893e54daa9b681e1e80883ae4908c39b0` (raw PDF hash from `raw_document_id`); `extracted_markdown_sha256=55a5a7cc58029cba4a3d40b2ef0a828f3cb49d8315f3f0a5fd7711f9ba29a2ec`; `stage_run_id=stage-run-43d02438-92da-4df3-bc14-2ff454300565-proc-pc-dc1daee2-772e-f111-813c-005056b64545`; `result_json_path=workdir/result.json` (50,486 bytes).
