---
manifest_version: 1.0.0
skill_id: pdf-extraction/tender-specification
skill_version: 1.0.0
job_id: job-7079b723-32e8-4712-a829-e896f9687c1d
session_id: session-semantic-1d418c47de0e443b5360361f761307c79684deaf85d59ae4d07da5fa63300293
execution_mode: contract
started_at: 2026-04-18T00:00:00Z
completed_at: 2026-04-18T00:00:00Z
outcome_class: success
items_processed: 1
items_skipped: 0
items_failed: 0
warnings_count: 4
agent_claimed_pages_visited: 0
agent_claimed_source_url: https://procontract.due-north.com/RfxResponse/Index?rfxId=ea9d90b2-4833-f111-813c-005056b64545
agent_claimed_fingerprint_state: not_applicable
agent_claimed_locale_state: en-GB
agent_claimed_currency_state: GBP
---

# Summary

Extracted a single-lot tender specification — London Borough of Bexley Cleaning & Hygiene Specification, Lot 6 of the "6278 - Building Services Framework - Lots 1-11" notice — from Docling-derived markdown of `6278 - Draft Specification - Lot 6.docx`. Produced one authoritative lot entry (Lot 6, "Provision of Cleaning Services - Corporate Buildings & Public facilities in Parks & Open Spaces") with 21 requirements, 4 KPI-based evaluation criteria, 9 product requirements, 7 technical standards and 16 evidence anchors, plus 9 product requirement seeds mirroring the lot's candidate acquisitions. Target domain is `lots` and extraction scope is `full_document`. Sibling lots 1-5 and 7-11 exist on the parent notice but are out of scope for this specification document and are not emitted.

# Steps Taken

1. Read job parameters from the contract-session prompt: `document_extraction_id=fe20c894-6f08-4231-a77b-8c8bda0de88d`, `extraction_scope=full_document`, `target_domain=lots`, `process_id=proc-pc-dc1daee2-772e-f111-813c-005056b64545`, `notice_id=pc-dc1daee2-772e-f111-813c-005056b64545`, `document_set_id=tds-pc-dc1daee2-772e-f111-813c-005056b64545`, `raw_document_id=raw-pc-dc1daee2-772e-f111-813c-005056b64545-e9b7b69db64da6...`.
2. Loaded the Docling-extracted markdown and metadata from `state/state.db` (`document_extractions.extracted_markdown`, `source_content_hash=df2277b38c39f46c466571d2f7eda3a472a0523900774d903e9617e9f1ea9e1d`, `extraction_policy_version=2026.04.12`, `document_role=technical_specification`, `document_title=6278 - Draft Specification - Lot 6`, `page_count=0`, `table_count=1`, `section_count=3`).
3. Cross-referenced existing notice-level metadata (`contract_notices` row for `pc-dc1daee2-772e-f111-813c-005056b64545`: buyer "London Borough of Bexley", title "6278 - Building Services Framework - Lots 1-11", parent main_category contains CPV codes `50700000-2 | 50750000-7 | 71315000-9`) and the eleven pre-existing `notice_lots` rows (1-11) for orientation only, without treating them as authoritative if the tender specification contradicted them.
4. Read and segmented the full markdown (Sections 1-15, KPI table in 12.2, plus header preamble covering buyer and contract commencement date). Mapped Section 2 scope, Sections 3-11 standards/compliance, Section 12 monitoring/KPIs, Section 13 pricing, Section 14 mobilisation, Section 15 (stub heading only).
5. Derived the lot record for Lot 6 directly from the specification body — not from the pre-existing `notice_lots` row — capturing 21 evidence-anchored requirements covering outcomes, frequencies, reactive SLAs, keyholding, mobile provision, consumables, waste, H&S, environmental/social value, monitoring, reporting, pricing structure, mobilisation and TUPE.
6. Lifted evaluation criteria from the KPI table in Section 12.2 verbatim, preserving the blank target cell on "Quality score" and the blank remedy cell on "Customer satisfaction" rather than inferring values.
7. Derived 9 product-requirement seeds for items the Contractor must procure or supply: washroom consumables; colour-coded cleaning equipment; low-moisture mobile kit; UN-approved sharps/clinical containers; environmentally preferable chemicals (bleach-free); electric/ULEV mobile-round vehicles; NFC/QR proof-of-service platform; biohazard response kit with barriers/signage; specialist periodic/variable works (floor care, jet-wash, gum removal, blinds/curtains, medical-standard cleans). Seeds are mirrored in the lot's `product_requirements` array and the top-level `product_requirement_seeds` array with matching `seed_key`s so the daemon can translate them into `product_candidates` writes.
8. Captured 7 technical standards each with an evidence anchor (golden assertion: `standards_require_evidence_anchors`): BICSc-aligned training, COSHH risk assessments, RAMS, DBS clearance, UN-approved sharps/clinical waste containers, London Living Wage, and TUPE.
9. Populated `warnings` with four notes: (a) no CPV codes are cited in the specification body so `cpv_codes` is left empty rather than inferred from the higher-level Building-Services parent-notice codes; (b) KPI table has partial blanks preserved verbatim; (c) Docling `page_count=0` because the source is DOCX rendered as a single logical page, so anchors reference section headings not numeric pages; (d) this is a single-lot document — siblings are out of scope.
10. Set `extraction_metadata.target_domain="lots"`, `extraction_scope="full_document"`, `source_markdown_hash` equal to the `document_extractions.source_content_hash` already recorded by the extractor (`df2277b38c39f46c466571d2f7eda3a472a0523900774d903e9617e9f1ea9e1d`), `policy_version="2026.04.12"` and `document_role="technical_specification"`.
11. Wrote `result.json` to the session workdir and confirmed it validates against `skills/pdf-extraction/tender-specification/result.schema.json` using `jsonschema.validate` (all required fields present: `lots`, `product_requirement_seeds`, `field_confidences`, `extraction_confidence_bps`, `warnings`, `evidence_anchors`, `extraction_metadata`; each lot carries every required sub-field and every standard entry carries a non-empty `evidence_anchor`).

# Decisions Made

- **Exactly one lot emitted.** The specification document covers only Lot 6; the global golden assertion `lot_outputs_must_be_non_empty` is satisfied with a single authoritative record, and sibling lots 1-5 and 7-11 are deliberately omitted because they are covered by separate specification documents outside this extraction scope.
- **CPV codes left empty.** The document body cites zero CPV codes. The parent notice's three CPV codes are all building-services/lift-maintenance codes unrelated to cleaning services, and while CPV `90910000-9` (cleaning services) is the obvious fit, emitting it without evidence in the document would violate the skill's non-goal of "guessing lot boundaries, standards, or product requirements without citing evidence anchors". The `CPV_CODES_NOT_CITED_IN_DOCUMENT` warning preserves this reasoning.
- **KPI blank cells preserved verbatim.** The "Quality score" target cell and the "Customer satisfaction" remedy cell are blank in the KPI table; reproduced as-is rather than inferred, with the `KPI_TARGETS_PARTIAL` warning making the gap explicit.
- **Product requirement seeds scoped to tangible acquisitions.** Seeds describe items the Contractor must source/supply (consumables, equipment, vehicles, a technology platform, response kits, specialist works), not abstract service obligations. This keeps the daemon's downstream `product_candidates` writes aligned with the intended scope of the tender-specification overlay.
- **Evidence anchors reference section headings, not pages.** The DOCX source renders as one logical page in Docling (`page_count=0`), so numeric `page_ref` values would be meaningless; instead each anchor uses a `page_ref` of the form `section-N[.M]-slug` and `section_ref` of the form `#section/N[.M]`, matching the actual heading structure in the Docling markdown and giving downstream consumers a stable, human-readable target.
- **Source markdown hash adopted from the extractor row** (`df2277b38c39f46c466571d2f7eda3a472a0523900774d903e9617e9f1ea9e1d`) rather than recomputing against the text pulled back through sqlite3 (which added a trailing newline); this keeps the skill's `source_markdown_hash` aligned with the authoritative `document_extractions.source_content_hash` value that downstream tooling already references.
- **Confidence level.** Overall `extraction_confidence_bps=8500` reflects high confidence on lot identity, title, description, requirements, standards and seeds (all directly anchored), moderate confidence on evaluation-criteria completeness (KPI table has acknowledged blanks), and explicit low confidence (`cpv_codes: 4000`) on CPV enumeration because the document does not cite any.

# Warnings

- `CPV_CODES_NOT_CITED_IN_DOCUMENT` — no CPV codes in the spec body; parent-notice codes (`50700000-2`, `50750000-7`, `71315000-9`) are building/lift-service codes unrelated to cleaning, so the lot's `cpv_codes` array is intentionally empty.
- `KPI_TARGETS_PARTIAL` — Section 12.2 KPI table contains two intentionally blank cells (Quality-score target; Customer-satisfaction remedy); reproduced verbatim.
- `DOCLING_PAGE_COUNT_ZERO` — the DOCX source renders as a single logical page; evidence anchors reference section headings (`section-N[.M]-slug`) rather than numeric page numbers.
- `SINGLE_LOT_DOCUMENT` — this is the Draft Specification for Lot 6 only; sibling lots 1-5 and 7-11 on the parent Building Services Framework notice are out of scope for this document.

# Evidence

- `document_extractions.id=fe20c894-6f08-4231-a77b-8c8bda0de88d` — the Docling extraction row providing the source markdown, document role (`technical_specification`), policy version (`2026.04.12`), source content hash (`df2277b38c39f46c466571d2f7eda3a472a0523900774d903e9617e9f1ea9e1d`) and expected overlay (`pdf-extraction/tender-specification`).
- `contract_notices.id=pc-dc1daee2-772e-f111-813c-005056b64545` — parent notice "6278 - Building Services Framework - Lots 1-11" with buyer "London Borough of Bexley", confirming buyer identity referenced in the specification header.
- `notice_lots` row `pc-dc1daee2-772e-f111-813c-005056b64545-lot-6` — cross-reference only: confirms the lot number and a compatible sibling title ("Provision of Cleaning Services - Corporate Buildings & Public facilities in Parks & Open Spaces"); not treated as authoritative truth.
- Document header anchor (`section-header`): "London Borough of Bexley - Cleaning & Hygiene Specification - Contract commencement: Sept 2026" — source for buyer, title and commencement date.
- Section 2 (`section-2-scope-of-services`) — source for the corporate/parks/external scope partitioning in the lot description and requirements.
- Section 3.1 (`section-3.1-service-outcomes`) — source for colour-coding anti-cross-contamination requirement and the colour-coded cleaning-equipment seed.
- Section 3.3 (`section-3.3-materials-and-chemicals`) — source for the bleach prohibition, SDS/COSHH requirement and the environmentally preferable chemicals seed.
- Section 4 (`section-4-frequencies`) — source for the frequency requirements (corporate daily/weekly/monthly, public conveniences summer vs winter cadence, changing rooms, function rooms, external approaches).
- Section 5 (`section-5-reactive-and-emergency-cleaning`) — source for the 30-minute make-safe / 2-hour full-sanitisation biohazard SLA underpinning the biohazard KPI criterion and biohazard response-kit seed.
- Section 6 (`section-6-keyholding`) — source for the keyholding, access control and alarm-management requirement.
- Section 7 (`section-7-mobile-cleaning-provision`) — source for the mobile-team requirement and the low-moisture mobile kit seed.
- Section 8 (`section-8-equipment-and-consumables`) — source for the washroom-consumables seed and the Contractor-supplied equipment obligation.
- Section 9 (`section-9-waste-streams`) — source for the UN-approved sharps/clinical waste containers requirement, seed and technical standard.
- Section 10 (`section-10-health-safety-and-compliance`) — source for the BICSc training, RAMS and DBS clearance technical standards.
- Section 11 (`section-11-environmental-and-social-value-commitments`) — source for the electric/ULEV mobile-round vehicles seed and the London Living Wage technical standard.
- Section 12.1 (`section-12.1-proof-of-service-and-audits`) — source for the NFC/QR proof-of-service platform seed.
- Section 12.2 KPI table (`section-12.2-kpi-table`) — source for the four evaluation criteria (Quality score, Public toilet attendance, Biohazard response, Customer satisfaction), with the verbatim per-row anchors recorded in each criterion's `evidence_anchor`.
- Section 13 (`section-13-pricing-structure`) — source for the Fixed Price Service Fee / Variable Works split and the specialist Variable Works seed.
- Section 14 (`section-14-mobilisation-and-data-pack`) — source for the mobilisation deliverables requirement and the TUPE technical standard.
- `result.json` artifact in session workdir — validates against `skills/pdf-extraction/tender-specification/result.schema.json`; contains 1 lot, 9 product requirement seeds, 4 evaluation criteria, 7 technical standards (each with evidence anchor), 16 top-level evidence anchors, 4 warnings and `extraction_metadata.target_domain="lots"`.
