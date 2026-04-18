---
manifest_version: 1
skill_id: pdf-extraction/tender-specification
skill_version: 1.0.0
job_id: job-0fce9841-6573-46ec-aa41-5155a7927e32
session_id: session-semantic-3e9fae1c34049680b363b75b0e31a02d9257dc3f6c8d10b9284ab9f157632b48
execution_mode: contract
started_at: 2026-04-17T19:59:27Z
completed_at: 2026-04-17T20:01:38Z
outcome_class: success
items_processed: 1
items_skipped: 0
items_failed: 0
warnings_count: 6
agent_claimed_pages_visited: 79
agent_claimed_source_url: none
agent_claimed_fingerprint_state: n/a
agent_claimed_locale_state: en-GB
agent_claimed_currency_state: GBP
---

## Summary

Ran pdf-extraction/tender-specification in contract mode against Docling markdown for document_extraction `373c087c-dccb-4253-b1b4-d7d716c9f193` (`107_26-Appendix-A-Framework-Agreement`, 79 pages, 13 tables, 207 sections, role `technical_specification`). The document is the ESPO 107_26 Framework Agreement contractual template rather than the substantive product-level specification, so extraction produced one lot (`lot_number="1"`) aligned with the single-lot notice, two service-level product-requirement seeds, five contractual/regulatory technical standards, and the evaluation-criteria machinery (call-off + FCSP). Six warnings were raised to flag that line-item specifications, specific BS EN standards and tender-stage evaluation weightings live in the accompanying Invitation to Tender (Section 2) and Appendix B (Pricing Schedule) rather than in this document. Result written as `result.json` (schema-validated) with `extraction_confidence_bps=6000`.

## Steps Taken

1. Located the extraction input in `state.db.document_extractions` by `id=373c087c-...`; captured document_title, role, page_count, section/table counts, policy_version and expected overlays (`["pdf-extraction/tender-specification"]`).
2. Dumped `extracted_markdown` (238,584 chars) and `extracted_sections_json` (207 entries) to a scratch directory for inspection.
3. Walked the markdown top-to-bottom to identify the contractual table of contents and the Framework Schedules (1–11). Confirmed Framework Schedule 1 body is a placeholder ("ESPO will incorporate into this Framework Schedule the Specification contained within Section 2 of the Invitation to Tender").
4. Verified notice-level context in `state.db.contract_notices` (`id=pc-3ba4e6b8-...`): CPV main_category `37400000-2 | 50000000-5`, buyer `ESPO`, explicit statement "The Framework is NOT divided into lots."
5. Cross-checked the existing `notice_lots` row (single row, `lot_number=1`) and a prior tender-specification extraction on the ITT (`9bcb0725-...`) to understand where the line-item specifications actually live.
6. Computed a SHA-256 of the extracted markdown (`2ca06eb0...af855`) for `extraction_metadata.source_markdown_hash`.
7. Wrote `result.json` with: one lot (title/description/requirements/evaluation_criteria/cpv_codes/product_requirements/technical_standards/evidence_anchor), two product requirement seeds, six warnings, sixteen top-level evidence anchors, and extraction_metadata at `target_domain="lots"`, `extraction_scope="full_document"`, `policy_version="2026.04.12"`.
8. Validated the output against `skills/pdf-extraction/tender-specification/result.schema.json` using `jsonschema`; validation passed.

## Decisions Made

- **One lot, `lot_number="1"`.** Honoured the notice-level declaration that the framework is not lotted and the title/Background language in this document that treats the framework as a single agreement, while also aligning with the existing `notice_lots` row for cross-reference continuity.
- **No fabricated per-category product list.** The framework agreement document does not enumerate the 21+ equipment categories or their inspection scopes; those live in Section 2 of the ITT (already extracted under document_extraction `9bcb0725-...`). Emitted only two service-level product requirement seeds covering indoor and outdoor scopes to avoid duplicating or mis-attributing line items to this document.
- **Requirements sourced from supplier obligations in-document.** Populated `lots.requirements` from clauses actually present (Framework Fee, MI reporting, Framework Manager, Standards compliance, Framework Review Meetings, Benchmarking, Continuous Improvement, marketing, anti-bribery, 12-month price-increase moratorium), rather than inventing service-level duties.
- **Evaluation criteria described at the right level.** Captured the three mechanisms actually defined in this document (Call-Off Without Competition objective mechanism, FCSP Award Criteria as applied, KPI monitoring), and raised `EVALUATION_CRITERIA_DEFERRED_TO_ITT` to signal that price/non-price weightings belong to the ITT.
- **Technical standards kept generic.** Emitted only the standards this document actually cites (the 'Standards' definition naming BSI/international standards, PA23, UK GDPR/DPA 2018/PECR, FOIA/EIR). Did **not** back-fill BS EN 1176 / 1177 / 916, PUWER 1998 or HSWA 1974 from the ITT extraction — those must be re-anchored when the ITT overlay runs, not claimed against this document.
- **CPV codes carried from notice metadata with an explicit warning.** The codes do not appear verbatim in the markdown; `CPV_CODES_FROM_NOTICE_METADATA` reduces `lots.cpv_codes` field confidence to 5000.
- **Confidence set to 6000 bps (60%).** The document structurally exists and is cleanly parsed, but the substantive specification content is deliberately placeholder-only, so a moderate-to-low confidence is warranted to avoid overstating extraction coverage.

## Warnings

1. `DOCUMENT_IS_CONTRACTUAL_TEMPLATE` — this document is the framework agreement template; Framework Schedule 1 explicitly defers the Specification to ITT Section 2 (see result contract `warnings[0]`).
2. `SINGLE_LOT_NO_LOT_SPLIT` — framework is not divided into lots; single lot emitted (result contract `warnings[1]`).
3. `CPV_CODES_FROM_NOTICE_METADATA` — CPV codes 37400000, 50000000 are taken from notice metadata, not observed in the markdown (result contract `warnings[2]`).
4. `EVALUATION_CRITERIA_DEFERRED_TO_ITT` — tender-stage price/non-price weightings live in ITT Section 5, not in this document (result contract `warnings[3]`).
5. `TECHNICAL_STANDARDS_GENERIC_ONLY` — specific BS EN / PUWER / HSWA references are not in this document; re-anchor against the ITT overlay (result contract `warnings[4]`).
6. `COMMENCEMENT_DATE_CONDITIONAL` — Commencement Date is stated as 09 June 2026 but is qualified ("or, if this date is not completed in this Framework Agreement, it shall be the date stipulated in writing by ESPO") (result contract `warnings[5]`).

## Evidence

All major claims in `result.json` are anchored to observable artefact fields:

- **Title / scope** — result contract `lots[0].evidence_anchor` (`#/texts/772` FRAMEWORK SCHEDULE 1, p56) and top-level `evidence_anchors[0]` (`#/texts/7` FRAMEWORK AGREEMENT FOR INSPECTION & MAINTENANCE ..., p1).
- **Agreement reference 107_26** — `evidence_anchors[1]` (`#/texts/9`, p1).
- **Tender Notice / PA23 placement** — `evidence_anchors[2]` (`#/texts/42`, p5) covering Background A).
- **'Standards' definition** — `lots[0].technical_standards[0..1].evidence_anchor` and `evidence_anchors[3]` (`#/texts/47`, p12).
- **Commencement Date** — `evidence_anchors[4]` (`#/texts/47`, p12).
- **Scope/Term clauses** — `evidence_anchors[5..6]` (`#/texts/123`, `#/texts/125`, p17).
- **Standards compliance obligation (clause 13.2)** — `evidence_anchors[7]` (p26).
- **Framework Schedule 1 placeholder** — `evidence_anchors[8]` (`#/texts/772`, p56).
- **Framework Fee 3.5%** — `evidence_anchors[9]` (`#/texts/797`, p58).
- **Call-off award procedures** — `evidence_anchors[10..11]` (`#/texts/866`, p63).
- **FCSP award criteria** — `evidence_anchors[12]` (`#/texts/926`, p67) and `lots[0].evaluation_criteria[0].evidence_anchor`.
- **MI / Schedule 7** — `evidence_anchors[13]` (`#/texts/965`, p70).
- **Schedule 10 framework management & KPIs** — `evidence_anchors[14..15]` (`#/texts/1007` p75; `#/texts/1059` p78) and `lots[0].evaluation_criteria[2].evidence_anchor`.
- **Cross-document linkage to ITT Section 2** — captured in warning `DOCUMENT_IS_CONTRACTUAL_TEMPLATE` with the Framework Schedule 1 §1.8 quote, and in `process_context` / `notice_lots.documents[0]` which references `document_extraction_id=9bcb0725-...` (`2. REQUIREMENT (including Specification)`, p13) as the authoritative specification source.

No claim in `result.json` is made against the ITT document directly; all anchors are to `#/texts/*` identifiers within this document's own structural extraction payload.
