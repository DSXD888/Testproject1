---
manifest_version: 1
skill_id: pdf-extraction/tender-specification
skill_version: 1.0.0
job_id: job-11cb24eb-8bba-4043-836f-391280396bfd
session_id: session-semantic-2df276ee1c6882bc2f9ae6f12ce83ca364ef786f3667a0af24546acd810be991
execution_mode: contract
started_at: 2026-04-18T04:37:00Z
completed_at: 2026-04-18T04:37:06Z
outcome_class: success
items_processed: 1
items_skipped: 0
items_failed: 0
warnings_count: 4
agent_claimed_pages_visited: 0
agent_claimed_source_url: https://procontract.due-north.com/RfxResponse/Index?rfxId=ea9d90b2-4833-f111-813c-005056b64545
agent_claimed_fingerprint_state: not_applicable_headless_contract
agent_claimed_locale_state: en-GB
agent_claimed_currency_state: GBP
---

# Summary

Extracted the single-lot tender specification payload for Lot 10 of the London Borough of Bexley **6278 Building Services Framework** from the Docling-produced markdown of the Draft T&Cs document (`raw-pc-dc1daee2-772e-f111-813c-005056b64545-2e256af67e376316d10d59ab3eff24858a0330739e3926ddb97758721ec0b415`). The document is a JCT Measured Term Contract 2024 draft scoping planned, preventative and responsive residential property maintenance and compliance services across approximately 230 Council-owned street properties, 120 privately leased properties, and 3 hostels for an initial term of 1 November 2026 – 31 October 2030 (4 years fixed, no extension).

The extraction produced one authoritative lot record (lot 10), ten product-requirement seeds at service-category granularity (responsive repairs, void refurbishment, gas servicing with LGSR, electrical inspection & testing, fire/Legionella compliance, emergency call-out, pest control, waste disposal, asbestos reporting, general building/engineering maintenance), three evaluation criteria drawn from the document's KPI section (90% response & completion, 100% statutory compliance, 90% void turnaround), and twelve technical standards / legislative references (JCT MTC 2024, HGCR Act 1996, Building Regulations 2010, Building Safety Act 2022, CDM 2015, Gas Safety (I&U) Regs 1998, BS 7671, RR(FS) Order, Environmental Protection Act 1990, HSW Act 1974, SSIP/CHAS, Data Protection). One primary evidence anchor is attached to the lot, and 15 corpus-wide evidence anchors cover the document's numbered clauses.

Four warnings were raised: (1) CPV codes are not explicit in the document body and were inherited from the notice-level categorisation (`50700000-2`); (2) Docling reports page_count=0 for this DOCX, so anchors use section references with a placeholder `p.1` page_ref; (3) the document is explicitly a Draft; (4) the product specifications are deferred to the Schedule of Rates / Pricing Schedule referenced in §14, so product seeds are expressed at service-category rather than part-number granularity. The overall `extraction_confidence_bps` was set to 8600.

# Steps Taken

1. Loaded the bound SKILL.md contract (headless_contract profile, target_domain=lots, result schema v1.0.0) from `skills/pdf-extraction/tender-specification/`.
2. Inspected the job parameters to identify the document_extraction_id (`1d28c615-eef3-48db-92cd-9aaebcc7aab1`), raw_document_id, process_id (`proc-pc-dc1daee2-772e-f111-813c-005056b64545`), notice_id, stage_run_id, and extraction_scope (`full_document`).
3. Queried `state/state.db` for `document_extractions` row 1d28c615-eef3… to retrieve the Docling-produced markdown, structural metadata, `source_content_hash` (`374695981963f080c75192785de01602df81bdeb81fccad3d9580737a3dc36a0`), and the upstream `extraction_policy_version` (`2026.04.12`).
4. Pulled process, notice and sibling-lot context: notice title `6278 - Building Services Framework - Lots 1-11`, buyer `London Borough of Bexley`, notice-level main categories (`50700000-2`, `50750000-7`, `71315000-9`), and the existing 11 `notice_lots` rows — confirming that this document corresponds to **Lot 10** (residential MTC) and not any of the sibling lots.
5. Read the full extracted markdown (§1–§14 of the Draft T&Cs) including the priority-categories table. Verified that the table is a real table in `extracted_tables_json` (`#/tables/0`, page_start=1).
6. Classified the lot structure as single-lot for this document (Lot 10) with all substantive sub-scopes (gas, electrical, fire, Legionella, voids, waste, asbestos, pest, emergency, general) modelled as product-requirement seeds rather than child lots, because the document itself is unambiguously a per-lot T&Cs draft covering one MTC.
7. Drafted the lot record, product_requirement_seeds, evaluation_criteria (from §8 KPIs), and technical_standards (from §7, §9, §10, §11, §13), each backed by a section-anchored evidence record. Assigned stable seed keys prefixed `lot10-…` so that chunked or resumed runs can reuse the same identifiers.
8. Assigned per-field confidences (`field_confidences`), a document-level `extraction_confidence_bps` of 8600, and four warning codes covering the known evidence gaps (CPV inheritance, zero page_count, Draft status, specification deferral to Schedule of Rates).
9. Wrote `result.json` to the session workdir, validated it against `skills/pdf-extraction/tender-specification/result.schema.json` using Python's `jsonschema` (`OK - schema valid`).
10. Produced this run manifest with the required YAML frontmatter fields and narrative sections.

# Decisions Made

- **Single-lot emission.** The document title, §1 Contract Title, §3 Description of the Works, §4 Property Portfolio, and §8 KPIs all treat this as one cohesive lot. Emitted one lot record with `lot_number = "10"`, keyed to the parent notice's `notice_lots.lot_number=10` row.
- **Service-category seeds, not part numbers.** §14 Contract Documents defers pricing and part-level specification to the Schedule of Rates / Pricing Schedule and Contract Specification, which are not present in this document. Expressed ten product_requirement_seeds at service-category granularity with `requirement_type` values (`service_scope`, `statutory_compliance_service`). Warning `SPECIFICATION_DEFERRED_TO_SCHEDULE` records this explicitly.
- **CPV inheritance.** The document body does not enumerate CPV codes. Carried through `50700000-2` (Repair and maintenance services of building installations) from the notice-level main category, which is the closest applicable primary code for residential maintenance under a JCT MTC. Did not inherit `50750000-7` (Lift-maintenance) or `71315000-9` (Building services) because neither is supported by the document body. Recorded warning `CPV_NOT_EXPLICIT_IN_DOCUMENT`.
- **Evidence anchor format.** Upstream `document_extractions.page_count = 0` (DOCX with no explicit pagination in Docling output), so every `page_ref` uses the placeholder `p.1` and the operationally meaningful anchor lives in `section_ref` (e.g. `§8 Key Performance Indicators (KPIs) – KPI 1`) with verbatim `anchor_text` quotations. Recorded warning `PAGE_COUNT_ZERO_IN_STRUCTURE`.
- **Evaluation criteria from KPIs.** The document does not publish a scoring matrix (MEAT weightings, price/quality split). Emitted the three KPIs from §8 as `evaluation_criteria` because they are the only quantitative performance criteria stated in the document, with `weighting_text` capturing the numeric thresholds (90% / 100% / 90%).
- **Technical standards as enumerated legislation.** Treated §9 enumerated legislation plus §7 payment statute and §10/§11/§13 instrument references as `technical_standards`. Did not invent supplementary standards (e.g. specific BS 5839 fire detection standards) that are not in the document body.
- **Draft status preserved.** Document is explicitly "Draft Contract – JCT Measured Term Contract 2024". Kept portfolio counts (230/120/3), contract dates, and KPI thresholds exactly as-written in the draft; recorded warning `DRAFT_TEMPLATE_STATUS` so downstream logic can weight these correctly against the awarded contract.
- **Source hash.** Used `source_content_hash` from `document_extractions` as `source_markdown_hash` because it is the canonical content identity tracked by the daemon for this extraction (`374695981963f080c75192785de01602df81bdeb81fccad3d9580737a3dc36a0`).
- **Extraction scope.** `extraction_scope = "full_document"` because the job parameters explicitly set this and the Docling output represents the complete DOCX converted successfully (`conversion_status = success`).
- **Document role.** Set `document_role = "technical_specification"` to match the upstream `document_extractions.document_role` value; the filename and metadata `document_role_hint = "terms_and_conditions"` describe the formal genre but the per-document role resolved against the overlay policy is `technical_specification` with overlay `pdf-extraction/tender-specification`.

# Warnings

1. **CPV_NOT_EXPLICIT_IN_DOCUMENT** — CPV code `50700000-2` is inherited from the notice-level main category rather than asserted by the document body. Downstream persistence should mark `cpv_codes` as carrying provenance from the notice, not the document semantic extraction.
2. **PAGE_COUNT_ZERO_IN_STRUCTURE** — Docling reports `page_count = 0`. Evidence anchors therefore use section references from the document's numbered clause headings. `page_ref` is set to `p.1` as a placeholder to satisfy the `evidenceAnchor` schema.
3. **DRAFT_TEMPLATE_STATUS** — The document is a Draft T&Cs. Portfolio counts, KPI thresholds, and contract dates are taken verbatim from the draft; the awarded contract may differ.
4. **SPECIFICATION_DEFERRED_TO_SCHEDULE** — §14 Contract Documents refers substantive technical content to the Schedule of Rates / Pricing Schedule, the Contract Specification, and the Property Schedule, none of which are included in this document. Product-requirement seeds therefore capture service categories, not per-part or per-material specifications.

# Evidence

Major claims in this extraction are anchored to the following observable structure in the Docling output and the accompanying daemon-side metadata:

- **Contract form and title.** "Draft Contract – JCT Measured Term Contract 2024" (document preamble) and §1 Contract Title "Measured Term Contract – Planned, Preventative and Responsive Residential Property Maintenance and Compliance Services" — anchors lot title.
- **Contract period.** §2 Contract Period: "Initial Contract Term: 1 November 2026 – 31 October 2030"; "Duration: 4 Years Fixed Term"; "No extension option." — anchors lot description.
- **Description of the works.** §3 enumerates the eleven service sub-scopes (responsive repairs, voids, general maintenance, engineering, gas, electrical, fire/Legionella, emergency, pest control, waste, asbestos) that seed `product_requirements` and `product_requirement_seeds`.
- **Property portfolio.** §4: "230 Council-owned residential street properties; 120 privately leased properties managed by the Council; 3 hostels" — anchors `quantity_hint` on the responsive-repairs seed and the description paragraph.
- **Priority categories (table).** §6 Priority Categories and Response Times renders the priority/response/completion matrix. Evidence for standard/urgent/emergency/void SLAs in the responsive-repairs, void-refurbishment, and emergency-call-out seeds. Matches `extracted_tables_json` entry `#/tables/0` (page_start=1).
- **Payment provisions.** §7: statutory reference to the Housing Grants, Construction and Regeneration Act 1996 and Section 4 of JCT MTC 2024; 30-day final date for payment, 6-month invoice window.
- **KPIs (§8).** KPI 1 (90% response/completion, measured monthly), KPI 2 (100% statutory compliance; Gas, Electrical, Energy Performance Certification), KPI 3 (small voids ≤10 working days, large voids ≤28 working days, 90% monthly compliance). Sole source of `evaluation_criteria`.
- **Statutory and regulatory compliance (§9).** Enumerated legislation feeds `technical_standards` (Building Regulations 2010, Building Safety Act 2022, CDM 2015, Gas Safety (I&U) Regulations, IET Wiring Regulations BS 7671, Regulatory Reform (Fire Safety) Order, Environmental Protection Act, Data Protection legislation).
- **Health & safety (§10).** HSW Act 1974, CDM 2015 duties, Principal Contractor role, SSIP-accredited H&S certification (e.g. CHAS) — feeds the SSIP/CHAS and HSW Act standards.
- **Gas servicing (§11).** Annual servicing, annual gas safety inspections, LGSR, 24-hour breakdown, compliance with Gas Safety (Installation and Use) Regulations 1998 — anchors the gas servicing seed and the 1998 regulations standard.
- **Staff competence (§12).** Trade qualifications, CSCS, photo ID, enhanced DBS — reflected in lot-level requirements.
- **Waste management (§13).** Environmental Protection Act 1990, registered waste carrier, waste transfer documentation on request — anchors the waste-disposal seed and EPA 1990 standard.
- **Contract documents (§14).** Lists JCT MTC 2024 Conditions, Contract Particulars, Contract Specification, Schedule of Rates / Pricing Schedule, Property Schedule — used to justify the `SPECIFICATION_DEFERRED_TO_SCHEDULE` warning.
- **Daemon appendix / state references.** `document_extractions.id = 1d28c615-eef3-48db-92cd-9aaebcc7aab1`; `source_content_hash = 374695981963f080c75192785de01602df81bdeb81fccad3d9580737a3dc36a0`; `extraction_policy_version = 2026.04.12`; `expected_overlays_json = ["pdf-extraction/tender-specification"]`; `document_role = technical_specification`; `page_count = 0`; `semantic_extraction_state = pending`. Parent notice `pc-dc1daee2-772e-f111-813c-005056b64545` title `6278 - Building Services Framework - Lots 1-11`, buyer `London Borough of Bexley`, notice main_category `50700000-2 - Repair and maintenance services of building installations | 50750000-7 - Lift-maintenance services | 71315000-9 - Building services`. Sibling `notice_lots` row for lot 10: `id=pc-dc1daee2-772e-f111-813c-005056b64545-lot-10`, matches document scope.
- **Artifact refs.** Result payload written to `workdir/result.json` (validated against `skills/pdf-extraction/tender-specification/result.schema.json`). Upstream DOCX stored per `raw_documents.metadata.filename = "6278 - Draft T&Cs - Lot 10 - Provision of Planned, Preventative & Responsive Residential Property Maintenance and Compliance Ser.docx"` extracted from archive `6278 - Adendum 1 -  Draft Tender Documents (FOR INFORMATION ONLY).zip`.
