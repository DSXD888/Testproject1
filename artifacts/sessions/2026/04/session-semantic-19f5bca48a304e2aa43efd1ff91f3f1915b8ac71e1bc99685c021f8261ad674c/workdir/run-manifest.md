---
manifest_version: 1
skill_id: pdf-extraction/tender-specification
skill_version: 1.0.0
job_id: job-c92fee55-503e-4ecd-b4f4-26d697271506
session_id: session-semantic-19f5bca48a304e2aa43efd1ff91f3f1915b8ac71e1bc99685c021f8261ad674c
execution_mode: contract
started_at: 2026-04-18T00:00:00Z
completed_at: 2026-04-18T00:00:00Z
outcome_class: completed_with_warnings
items_processed: 1
items_skipped: 0
items_failed: 0
warnings_count: 8
agent_claimed_pages_visited: 0
agent_claimed_source_url: null
agent_claimed_fingerprint_state: not_applicable
agent_claimed_locale_state: en-GB
agent_claimed_currency_state: GBP
---

# Summary

Extracted lot structure, product-requirement seeds, and technical-standard references from document_extraction 9ba50211-b4a9-4f1f-8830-307212a53a32 (raw_document raw-pc-19574d11-0929-f111-813c-005056b64545-088e587eaf1264f5d99e290543c6aea5ff8eb080c8abcc5995b1ff0050d495ad), titled "Appendix-7-Draft-Contract" (59 pages, policy_version 2026.04.12). The document is classified in the extraction record as role `technical_specification`, but its actual content is a draft Services Agreement for the Cambridgeshire County Council "Ely Library - Engage, Design and Fit Out" procurement (process proc-pc-19574d11-0929-f111-813c-005056b64545). The "Schedule 1 Specification" heading on p.47 is an empty placeholder. Substantive scope was therefore extracted from operative contract clauses 4-7. One lot was emitted, consistent with the single existing notice_lot (pc-19574d11-0929-f111-813c-005056b64545-lot-1). Four product_requirement_seeds were emitted (library furniture, signage, carpet, design service) - all qualitative. Nine technical_standards were captured (regulatory/legal frameworks and accessibility principles; no BS/EN/ISO product standards are cited in the source). No tender evaluation criteria are present - this is a post-award contract. Overall extraction_confidence_bps = 5500 reflects the role/content mismatch and the absence of quantitative product specifications; field-level confidences are captured in result.field_confidences.

# Steps Taken

- Read the daemon-supplied extraction record (document_extractions.id = 9ba50211-b4a9-4f1f-8830-307212a53a32) to retrieve title, document_role, page_count, source_content_hash and extracted_markdown. Pulled the markdown content (length 112 302 chars) and the 64-entry extracted_sections_json table of contents.
- Cross-referenced process_context (procurement_processes.id = proc-pc-19574d11-0929-f111-813c-005056b64545: buyer "Cambridgeshire County Council", title "25112 Library Services Engage, Design and Fit out Contract - Ely Library") and contract_notices metadata (main_category = "39155000-3 - Library furniture | 79932000-6 - Interior design services") to confirm buyer/scope/CPV context.
- Inspected the existing notice_lots entry (pc-19574d11-0929-f111-813c-005056b64545-lot-1, "Ely Library - Engage, Design and Fit Out") as cross-reference only; did not adopt its description verbatim but confirmed single-lot structure and reused lot numbering.
- Surveyed the ## heading outline in the markdown (Contents Clause, Parties, Background, Clauses 1-44, Schedule 1 Specification, Schedule 4 payment, Schedule 5 contract management, Schedule 7 change control) to classify document type.
- Searched for specification signals (BS/ISO/EN references, Schedule 1 content, evaluation criteria, award weightings). Confirmed: (a) Schedule 1 Specification on p.47 contains only drafter comment FB20 suggesting future warranty clauses; (b) no evaluation criteria are present (contract is post-award; clause references Supplier's Tender at Schedule 3 as already submitted); (c) no BS/EN/ISO product standards are cited.
- Extracted operative obligations from clauses 1 (Definitions: Best Industry Practice, Data Protection Legislation, EIRs, Relevant Requirements), 4 (Supply of services: design engagement, accessibility, final-design approval gate, RAMS, delivery and install), 5 (Compliance: applicable Law, Health and Safety Policy), 6 (Environmental obligations: Climate Charter, waste minimisation, carbon reduction), 7 (Premises, handover documentation and warranties on exit), 21 (Data processing), 28 (Bribery), 29 (Modern slavery) into the lots[0].requirements array and mirrored the regulatory/legal citations into lots[0].technical_standards.
- Derived four product_requirement_seeds from the recital ("design, furniture, signage, and carpet") and clauses 4.3-4.6: `ely-library-furniture`, `ely-library-signage`, `ely-library-carpet`, `ely-library-design-service`. Each seed is tagged with the relevant CPV alignment (furniture/signage → 39155000-3 Library furniture; design service → 79932000-6 Interior design services; carpet left without CPV since 39155000-3 is a furniture code, not a floor-covering code, and the draft contract provides no narrower descriptor).
- Populated 20 evidence_anchors with page refs, anchor text and section refs. Populated 8 warnings covering document-role/content mismatch, empty Schedule 1, absent evaluation criteria, CPV origin, unquantified product requirements, regulatory-vs-product standards, Docling table-detection note, and the fact that Lot 1 is carried from the existing notice lot.
- Computed extraction_confidence_bps = 5500 (honest-low) and field-level confidences; set document_role_match = 2500 to flag the mismatch.
- Wrote result.json to the session workdir and validated it structurally against skills/pdf-extraction/tender-specification/result.schema.json (all required keys present; all required evidence anchors populated; required extraction_metadata.target_domain == "lots" and extraction_scope == "full_document").

# Decisions Made

- **One lot emitted, reusing notice_lot number "1".** The draft contract is a single-scope engagement and the notice already carries one lot. Inventing additional lots would contradict the source. Lot title matches the existing notice_lots title for stable downstream identifier reuse (skill resume contract: preserve stable lot numbering and seed keys across runs).
- **No evaluation_criteria emitted.** This is a post-award contract; award criteria belong in a separate ITT or specification document. Emitting fabricated criteria would violate the skill's evidence-anchoring rule.
- **Product requirements captured as qualitative seeds only.** The draft contract provides no quantities, dimensions, material grades, unit counts, or coverage areas. quantity_hint = "unspecified" for all goods seeds; detail will be generated during the post-contract design engagement stage.
- **Carpet seed emitted without CPV.** CPV 39155000-3 is Library furniture, which does not cover floor coverings. Rather than mis-classify, the seed's cpv_codes is left empty; downstream classification can assign a more appropriate CPV (e.g. 39531000-3 class) if needed.
- **Technical standards list captures regulatory/legal frameworks, not product standards.** Best Industry Practice, accessibility principles (dementia-friendly, autism-friendly), health & safety law, RAMS, environmental law / Climate Charter, UK GDPR / DPA 2018 / PECR, EIRs / FOIA, Bribery Act 2010, and Modern Slavery Act 2015 are all the contract cites. A warning (STANDARDS_ARE_REGULATORY_NOT_PRODUCT) explicitly flags the absence of BS/EN/ISO product standards so downstream consumers do not mistake the list for a product-standard manifest.
- **CPV codes reused from process_context.** The contract body does not restate CPV codes, but notice-level metadata (39155000-3, 79932000-6) is clearly consistent with the contract scope. A warning (CPV_CODES_FROM_PROCESS_CONTEXT) records the provenance.
- **Confidence set honestly low.** extraction_confidence_bps = 5500 reflects that the document is a draft contract rather than a specification; product_requirements field confidence = 6000; document_role_match = 2500. Title/description/lot count are higher because they align with authoritative process/notice context.
- **extraction_metadata.document_role preserved as "technical_specification".** The daemon's role classification is preserved so the daemon-side semantic_extraction state transitions remain consistent; the role/content mismatch is surfaced through the DOCUMENT_ROLE_CONTENT_MISMATCH warning rather than by silently changing the role.

# Warnings

See result.warnings. Eight warnings were emitted:

1. `DOCUMENT_ROLE_CONTENT_MISMATCH` - Source document is a draft Services Agreement, not a technical specification; Schedule 1 Specification is empty.
2. `SPECIFICATION_SCHEDULE_EMPTY` - Schedule 1 Specification (p.47) contains only drafter comment FB20 suggesting warranty clauses.
3. `EVALUATION_CRITERIA_ABSENT` - No tender award criteria or weightings are present; this is a post-award document.
4. `CPV_CODES_FROM_PROCESS_CONTEXT` - CPV codes 39155000-3 and 79932000-6 carried forward from notice metadata, not restated in the contract body.
5. `PRODUCT_REQUIREMENTS_UNQUANTIFIED` - All four seeds are category-level only; no quantities, dimensions, materials, or unit counts.
6. `STANDARDS_ARE_REGULATORY_NOT_PRODUCT` - No BS/EN/ISO/UKCA product standards are cited; standards list is regulatory/legal only.
7. `OCR_AND_TABLE_NOTES` - Docling reported "No structured tables were detected". Contents page rendered as text with dot leaders; no impact on substantive extraction.
8. `LOT_REUSED_FROM_NOTICE` - Lot 1 title/number preserved to match existing notice_lots row for identifier stability.

# Evidence

All major claims in this manifest are anchored to observable features of the structural extraction payload:

- **Lot structure (single lot)** - anchored to document cover p.1 ("AGREEMENT RELATING TO ELY LIBRARY SERVICES - ENGAGE, DESIGN AND FIT OUT CONTRACT") and the BACKGROUND recital p.1 ("provision of design, furniture, signage, and carpet to refresh the public library offer in Ely library"). Cross-referenced to existing notice_lots.pc-19574d11-0929-f111-813c-005056b64545-lot-1 as supplied in process_context.
- **Empty Schedule 1 Specification** - anchored to section #/texts/611 at p.47 ("## Schedule 1 Specification" immediately followed by drafter comment "Commented [FB20]: I would suggest including the following into the Specification: ... Minimum 12-month warranty on installation workmanship. Manufacturer warranties for furniture, signage, and carpet. Warranty pack and care instructions at handover." with no operative clause body).
- **Supply of services scope** - anchored to section #/texts/170 at p.13 ("4. Supply of services"), in particular clauses 4.2 (Best Industry Practice, defect-free goods), 4.3 (design engagement: site visit, stakeholder consultation, initial proposals, one round of feedback), 4.4 (accessibility - dementia-friendly, autism-friendly), 4.5 (no manufacture before Authority written approval of final design), 4.6 ("deliver and install all furniture, signage, carpet and any associated items"), 4.7 (RAMS specific to Ely Library, first-floor access constraints).
- **Compliance and health & safety** - anchored to section #/texts/189 at p.15 ("5. Compliance and change in laws"): clause 5.2(a)-(b) applicable health-and-safety Law and Authority Health and Safety Policy.
- **Environmental obligations** - anchored to section "6. Supplier environmental obligations" at p.15: clauses 6.1(a) applicable environmental Law; 6.2(b) Climate Charter carbon-reduction measures; 6.2(c) waste-minimisation measures.
- **Handover and warranty documentation** - anchored to section #/texts/224 at p.17 ("7. Use of Authority Premises and Assets"): clause 7.9(a) exit handover of materials / components / equipment documentation including manufacturer warranties, maintenance instructions and compliance certificates.
- **Data protection framework** - anchored to section "21. Data processing" at p.29: clause 21.1 compliance with Data Protection Legislation; clause 21.2 Controller/Processor roles; definition at p.7 ("UK GDPR; the Data Protection Act 2018 (DPA 2018); ... Privacy and Electronic Communications Regulations 2003").
- **Bribery / modern slavery / FOIA / EIRs** - anchored to Clause 28 Prevention of bribery (p.35, Relevant Requirements definition including Bribery Act 2010); Clause 29 Modern slavery (p.37, Modern Slavery Act 2015); Clause 20 Freedom of information (p.28); EIRs definition at p.8 (Environmental Information Regulations 2004 SI 2004/3391).
- **CPV codes** - not anchored to the draft contract body (warning CPV_CODES_FROM_PROCESS_CONTEXT). Anchored to contract_notices.main_category metadata: "39155000-3 - Library furniture | 79932000-6 - Interior design services".
- **source_markdown_hash** - extracted from document_extractions.source_content_hash = `aea7731418bba2a5ba7e688f4925f75ed184aa65aafcead349fe042020dd57af` (daemon-observed fact from the extraction record; not a claim about agent execution).

Docling quality note (daemon-observed): document_extractions.quality_warnings_json = `["No structured tables were detected in the exported document."]`. Reflected in warning OCR_AND_TABLE_NOTES.

This run did not visit any URLs, render any web pages, or perform any browser interactions. It is a contract-mode structured extraction from a daemon-provided markdown payload, so fingerprint/locale/currency lifecycle claims in the frontmatter are either `not_applicable` or inferred from document scope (en-GB, GBP - UK public-sector contract denominated in sterling per clause 4 charges/Schedule 4).
