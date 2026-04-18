---
manifest_version: "1.0"
skill_id: pdf-extraction/tender-specification
skill_version: 1.0.0
job_id: job-e7acac10-f8a1-4dfe-ba8c-dbc863b8f453
session_id: session-semantic-fc3f2e4f81cbe5936f55aa56272b8ee4e46105cf497383864cb77e3822572587
execution_mode: contract
started_at: 2026-04-18T02:00:00Z
completed_at: 2026-04-18T02:09:30Z
outcome_class: success_with_warnings
items_processed: 1
items_skipped: 0
items_failed: 0
warnings_count: 9
agent_claimed_pages_visited: 0
agent_claimed_source_url: "sqlite://state/state.db/document_extractions/4ff7ed58-85d5-45a9-8ec8-61eac0490169"
agent_claimed_fingerprint_state: not_applicable
agent_claimed_locale_state: en-GB
agent_claimed_currency_state: GBP
---

## Summary

Ran `pdf-extraction/tender-specification` (contract mode) over Docling markdown for document_extraction `4ff7ed58-85d5-45a9-8ec8-61eac0490169` (raw doc `raw-pc-dc1daee2-772e-f111-813c-005056b64545-...`), title "6278 - Draft T&Cs - Lot 7", 63 pages, 99 sections, 166,027 chars of markdown, `document_role=technical_specification`.

The document is the **draft Form of Contract + General Terms and Conditions + Special Conditions** for Lot 7 of procurement 6278 (Lift Services, London Borough of Bexley, 1 Sept 2020 - 31 Aug 2023). Crucially, this markdown does NOT contain the actual specification: Schedule 1 (Specification), Schedule 2 (Pricing Schedule / PPM Matrix / Appendix 1 to ITT), and Schedule 3 (ITT / Appendix C KPIs) appear only as heading-only placeholders at the very end of the file (markdown lines 1222-1228).

Extraction returned:
- **1 lot** (Lot 7 - Provision of Lift Services) at medium confidence (extraction_confidence_bps = 5500)
- **3 product_requirement_seeds** (PPM annual fee service, call-off Works Orders service, external-specification-reference pointer to Schedule 1)
- **0 CPV codes** (none cited in this document - plausible CPV 50750000-7 NOT emitted because unsupported by evidence)
- **2 evaluation_criteria entries** (KPIs and Service Credits regime, both referenced but with values held externally in Appendix C to ITT)
- **25 requirements** (extracted from Clauses A-I and S1-S10 covering contract period, quality standards, CDM, H&S, data protection, TUPE, insurance, dispute resolution, termination etc.)
- **8 technical_standards entries** (generic BSI/ISO, British Standard materials, BCO construction materials guide, CDM 2015, HSWA 1974 + HSC/HSE codes, UK GDPR + DPA 2018 + LED, PCR 2015 procurement regime, with explicit note that no lift-specific standards such as BS EN 81, BS 7255, LOLER, PUWER are present)
- **9 warnings** covering the missing Schedule 1 / Schedule 2 / Appendix C content, the Recital A template-text inconsistency ('security services' vs 'LIFT SERVICES'), no lift-specific standards, no CPV codes, placeholder Annual Fee, placeholder insurance limits, and the fact that lot_number=7 is derived from document metadata rather than an in-markdown 'Lot 7' heading.

Wrote `result.json` (31,652 bytes) in the session workdir; result matches the `target_domain = "lots"` contract and `extraction_scope = "full_document"`.

## Steps Taken

1. **Located the Docling markdown**: queried `state/state.db.document_extractions` by `id = 4ff7ed58-85d5-45a9-8ec8-61eac0490169` and pulled `extracted_markdown` (166,027 chars, 63 pages, 99 sections, 0 tables, 0 images, `expected_overlays_json = ["pdf-extraction/tender-specification"]`, `document_role = technical_specification`).
2. **Computed markdown hash** (sha256): `d746197eb5aa991ff8474d5f05c9c5cc54843fa96405f96ea7d6bef00c2bc6fb` - recorded in `extraction_metadata.source_markdown_hash`.
3. **Structural scan**: grep'd for headings (`^##`), SCHEDULE / Specification / Lot / CPV / LOLER / PUWER / EN 81 / BS 7255 / lift markers. Confirmed the document is pure T&Cs template with schedule-only placeholders at end.
4. **Read key sections**:
   - Title block + Parties + Recital A + Clause 1.2 document precedence (lines 17-52)
   - A1 Definitions (Annual Fee Services, Works Order, Plant, Key Personnel, KPIs, Quality Standards, Service Credits) - lines 213-392
   - B1 The Services, B3 Quality Standards, B4 Contractor's Staff (lines 440-500)
   - D6 Health and Safety, E1 Data Protection (lines 634-730)
   - F5 KPIs and Remedies (line 883), F7 Monitoring (line 903)
   - S1 TUPE, S2 Contractor's Obligations (esp. S2.3 CDM, S2.4 Plant servicing, S2.6 deleterious materials), S3 Health and Safety, S5 Goods and Materials, S7 Sub-Contractors, S9 Equipment, S10 Adjudication/Arbitration (lines 1110-1220)
   - Schedule placeholders (lines 1222-1228)
5. **Cross-checked sample prior extraction** of another tender-spec run (Ely Library ITT) from `document_semantic_extractions` to align field names/shape (`lots[*].lot_number / description / cpv_codes / evaluation_criteria / product_requirements / requirements / technical_standards / evidence_anchor`, plus top-level `field_confidences / extraction_confidence_bps / warnings / evidence_anchors / extraction_metadata / product_requirement_seeds`).
6. **Assembled result.json** with 1 lot, 3 product_requirement_seeds, 17 field confidences, 9 warnings and 14 top-level evidence anchors.
7. **Validated result.json** (`python3 -m json.tool`) - parsed successfully, 31,652 bytes.
8. **Wrote this run manifest** (`run-manifest.md`) alongside `result.json` in the session workdir.

## Decisions Made

- **One lot, not many**: the document is scoped to Lot 7 only (title metadata: "6278 - Draft T&Cs - Lot 7"). Other lots in procurement 6278 are out of scope of this extraction and not guessed at. `lot_number = "7"` is derived from the document title metadata (noted as warning `LOT_NUMBER_FROM_TITLE_METADATA`) because the markdown body contains no explicit "Lot 7" heading.
- **No CPV code guess**: plausible CPV 50750000-7 (Lift-maintenance services) / 42416100-6 (Lifts) / 50711000-2 (Electrical installations repair) are all defensible inferences for a lift services contract, but the skill's Non-goals clause forbids "Guessing lot boundaries, standards, or product requirements without citing evidence anchors." Since no CPV code is cited in the visible markdown, `cpv_codes = []` and a dedicated warning (`NO_CPV_CODES_PRESENT`) flags the gap with confidence 1000 bps (field_confidences.cpv_codes).
- **No lift-specific engineering standards emitted**: BS EN 81 series, BS EN 13015, BS 7255, LOLER 1998, PUWER 1998 are the canonical standards for a lift PPM contract. None appear in this markdown; only generic BSI/ISO/British Standard references do. The extraction therefore emits generic standards only, with warning `NO_LIFT_SPECIFIC_STANDARDS` and low confidence (field_confidences.lift_specific_standards = 500 bps). These standards are expected to live in Schedule 1 Specification, which is not included.
- **Product_requirement_seeds are thin, on purpose**: the document is T&Cs, not a specification. Only two service-level seeds are emitted (`lot-7-lift-services-ppm`, `lot-7-lift-services-works-orders`) tied directly to the A1 definitions of "Annual Fee Services" and "Works Order". A third pointer-type seed (`lot-7-lift-services-specification-placeholder`, `requirement_type = external_specification_reference`) is emitted so that the daemon's pipeline knows a downstream extraction of Schedule 1 is required before `product_candidates` can be populated with confidence. No asset-level seeds (individual lifts / buildings / component lists) were invented.
- **Evaluation criteria without weightings**: the KPIs and Service Credit amounts are explicitly defined as living in Appendix C to the Invitation to Tender (Schedule 3), which is a heading-only placeholder here. Rather than returning a misleading empty array, the extraction returns two high-level entries describing the regimes and flagging where values live, with explicit `weighting_text` of "Not present in this document - see Appendix C". Field confidences for `kpi_weightings` and `service_credits_values` are 500 bps each.
- **Recital A template inconsistency surfaced, not "corrected"**: Recital A (line 37) says "security services" while the Form of Contract title (line 17) and file title say "LIFT SERVICES". This is plainly a template-repurposing error in the source document. The extraction records the lot scope as lift services (matching the title block) but emits a dedicated warning (`TEMPLATE_TEXT_NOT_CUSTOMISED_RECITAL_A`) and preserves the Recital A text verbatim as evidence anchor #3 so reviewers can see both readings. No silent correction was made.
- **Overall extraction confidence = 5500 bps (medium)**: rewards the many high-confidence structural facts (buyer, contract period, price structure, contract terms) but heavily penalises the absence of the actual Specification, PPM schedule, pricing schedule, KPIs and lift-specific standards - the fields that matter most for downstream product_candidate generation.
- **Document role kept as `technical_specification`**: matches the pre-classified `document_role` on `document_extractions.document_role`. A `document_subtype = "draft_contract_terms_and_conditions"` is added in `extraction_metadata` so the daemon can see the mismatch between the overlay classification and the actual content shape.

## Warnings

| Code | Severity | Summary |
|------|----------|---------|
| SCHEDULE_1_SPECIFICATION_MISSING | high | Schedule 1 Specification is heading-only at line 1222. Substantive service scope, lift asset inventory, visit frequencies and lift-specific engineering standards are not extractable from this document. |
| SCHEDULE_2_PRICING_SCHEDULE_MISSING | high | Schedule 2 Pricing Schedule is heading-only at line 1224. Annual Fee value at line 213 is the template placeholder `£ [insert figure]`. |
| APPENDIX_C_ITT_KPIS_MISSING | high | Schedule 3 / Appendix C KPIs + Service Credits are heading-only at line 1226. No KPI thresholds, scoring methodology or penalty amounts available. |
| TEMPLATE_TEXT_NOT_CUSTOMISED_RECITAL_A | medium | Recital A says "security services"; title says "LIFT SERVICES". Template repurposing error preserved as evidence for reviewers. |
| NO_LIFT_SPECIFIC_STANDARDS | medium | No BS EN 81 / BS EN 13015 / BS 7255 / LOLER / PUWER references in visible markdown. Only generic BSI/ISO references appear. |
| NO_CPV_CODES_PRESENT | medium | No CPV codes cited. Empty array returned rather than guessed. |
| INSURANCE_LIMITS_NOT_SPECIFIED | low | Clause G1.10 refers minimum insurance cover to the Specification / Special Conditions; no numeric limits in this markdown. |
| ANNUAL_FEE_PLACEHOLDER | low | Annual Fee definition contains `£ [insert figure]` template placeholder. |
| LOT_NUMBER_FROM_TITLE_METADATA | low | Lot 7 is derived from `document_extraction.document_title = "6278 - Draft T&Cs - Lot 7"`; no "Lot 7" label exists inside the markdown body. |

## Evidence

All major claims in the result and in this manifest are anchored to specific markdown line ranges in the Docling-produced `extracted_markdown`. Source content hash: `d746197eb5aa991ff8474d5f05c9c5cc54843fa96405f96ea7d6bef00c2bc6fb`. Key anchors:

- **Lot scope / title**: line 17 - "RELATING TO THE PROVISION OF LIFT SERVICES" (PART 1 FORM OF CONTRACT title block).
- **Buyer identity and address**: line 27 - "THE MAYOR AND BURGESSES OF THE LONDON BOROUGH OF BEXLEY of Civic Offices, 2 Watling Street, Bexleyheath, Kent DA6 7AT".
- **Contract period**: line 37 (Recital A, period 1 Sept 2020 - 31 Aug 2023) and line 54 (Clause 1.3 commencement).
- **Document precedence / schedule list**: lines 44-51 (Clause 1.2 - Form of Contract, GTCs, Special Conditions, Schedules 1-4).
- **Template-text inconsistency**: line 37 (Recital A says "security services") vs line 17 (title says "LIFT SERVICES") - preserved as evidence #3 with the `TEMPLATE_TEXT_NOT_CUSTOMISED_RECITAL_A` warning.
- **Annual Fee Services (PPM)**: line 215 - "...the services described in the Specification and Appendix 1 to the Invitation to Tender (Pricing Matrix containing the Planned Maintenance/PPM Schedule)".
- **Works Order (non-PPM scope)**: line 392 - "'Works Order' means works which are instructed by the Council but fall outside the scope of the Annual Fee Services".
- **Plant (Council's lifts) obligation to service**: line 1135 - "The Contractor shall service the Plant and maintain it in good working order and in accordance with the provisions of any manufacturer's guarantees or recommendations...".
- **KPIs regime (externally located)**: line 314 - "'Key Performance Indicators' or 'KPIs' means the key performance indicators included in Appendix C to the Invitation to Tender."; and Clause F5.1 at line 885.
- **Service Credits regime (externally located)**: line 368 - "'Service Credits' means a sum which the Council is entitled to deduct or invoice for a Service Failure pursuant to Clause C3A (Service Credits) as specified in the column headed 'penalty per failure' in Appendix C of the Invitation to Tender"; Clause C3A at line 572.
- **Generic Quality Standards framework**: line 354 - A1 Definitions "Quality Standards".
- **Materials must comply with latest British Standard**: line 1174 - Clause S5.1 Goods and Materials.
- **BSI Group / European directive deleterious-materials standard**: line 1144 - Clause S2.6.2.
- **BCO 'Good Practice in Selection of Construction Materials'**: line 1145 - Clause S2.6.3.
- **CDM Regulations 2015**: line 221 (A1 definition) and line 1134 (Clause S2.3 principal designer / principal contractor).
- **Health and Safety at Work etc. Act 1974 + HSC/HSE codes**: line 643 (Clause D6.7); line 1131 (Clause S2.2.2).
- **Data Protection Legislation (UK GDPR / DPA 2018 / LED)**: line 265 (A1 definition).
- **PCR 2015 procurement regime signal**: line 239 (A1 "Contracting Authority" definition).
- **Schedule placeholders (content missing)**: lines 1222-1228 - "SCHEDULE 1 SPECIFICATION / SCHEDULE 2 PRICING SCHEDULE / SCHEDULE 3 THE COUNCIL'S INVITATION TO TENDER / SCHEDULE 4 THE CONTRACTOR'S TENDER".

Daemon-observable artifacts produced by this run:
- `result.json` (31,652 bytes) in session workdir - satisfies result contract `target_domain = "lots"`, `extraction_scope = "full_document"`, `source_markdown_hash = d746197eb5aa991ff8474d5f05c9c5cc54843fa96405f96ea7d6bef00c2bc6fb`, `policy_version = 2026.04.12`.
- This `run-manifest.md` - satisfies run manifest requirements with required YAML frontmatter and the five required narrative sections.
