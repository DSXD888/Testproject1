---
manifest_version: 1.0.0
skill_id: pdf-extraction/tender-specification
skill_version: 1.0.0
job_id: job-21b69afe-33ff-41e1-a0a4-f757fc569e1c
session_id: session-semantic-76dc9257ce99ff783d007e94365506e07799ff3c6b8fb4f117898162929ef17c
execution_mode: contract
started_at: 2026-04-17T20:46:00Z
completed_at: 2026-04-17T20:51:00Z
outcome_class: partial
items_processed: 1
items_skipped: 0
items_failed: 0
warnings_count: 6
agent_claimed_pages_visited: 0
agent_claimed_source_url: "https://procontract.due-north.com/RfxResponse/Index?rfxId=a12aa4e2-1037-f111-813c-005056b64545"
agent_claimed_fingerprint_state: not_applicable
agent_claimed_locale_state: en-GB
agent_claimed_currency_state: not_applicable
---

# Summary

Extracted tender-specification semantic fields from the Docling markdown supplied for `document_extraction_id=f0d8d894-dd2a-478c-9094-930e122dd578` (raw document `Volume-I-Instructions-Conditions-Jan2026.pdf`, 10 pages, 41 sections per upstream Docling structural extraction). One authoritative lot record was emitted representing the overall ITT scope (`C3S2_413_bis Lot 2 Maintenance and Extension of C3S Windstorm and Storm Surge Services`) as declared in the process/notice context. Zero `product_requirement_seeds` were emitted and zero `technical_standards` were emitted because the document itself is Volume I (Instructions for respondents and Conditions of Tender), not Volume II (Specification); the input document explicitly defers detailed technical content to the separate Volume II Specification document at section 4.5. Extraction confidence set to 2500 bps to reflect the document-role mismatch. Six warnings were raised and evidence anchors were cited from ten in-document section headings. Result contract written to `result.json` and validated structurally against the required top-level keys declared in `result.schema.json`.

# Steps Taken

1. Resolved the input extraction packet: queried `state.db:document_extractions` for `id=f0d8d894-dd2a-478c-9094-930e122dd578` and confirmed the bound artefact is the ECMWF ITT `Volume-I-Instructions-Conditions-Jan2026.pdf`, classified upstream as `document_role=technical_specification`, `expected_overlays_json=["pdf-extraction/tender-specification"]`, `extraction_policy_version=2026.04.12`, `semantic_extraction_state=pending`, `page_count=10`, `section_count=41`. The source markdown content hash (`source_content_hash` on the extraction row) is `982304ae02cae9dd63283bb146359198706f2aa239778930cba54ec092ab43f3` and was reused verbatim for `extraction_metadata.source_markdown_hash`.
2. Read the full Docling-produced markdown (`length(extracted_markdown)=47928 bytes`, 363 lines) and parsed it sequentially against the skill's non-goals checklist.
3. Identified document shape: Table of Contents (sections 1..7), then body sections 1. Introduction, 2. Definitions, 3.1..3.16 Terms and Conditions of Tender, 4.1..4.9 Structure of this ITT, 5.1..5.2 Submissions, 6. Clarification questions, 7. Status of the ECMWF. No section contains product-like items, technical standards (EN/ISO/BS/IEC), pricing schedules, or explicit lot boundaries.
4. Cross-referenced the process row in `state.db:contract_notices` for `id=pc-978a1774-1037-f111-813c-005056b64545` and observed that the notice title encodes the contractual scope: `C3S2_413_bis Lot 2 Maintenance and Extension of C3S Windstorm and Storm Surge Services` (buyer `ECMWF`, `main_category=73000000-2`). Used this only for ambient context — did not promote the notice-level CPV into `lots[0].cpv_codes` because the CPV does not appear in the in-document text and SKILL.md forbids claims without evidence anchors.
5. Constructed `lots[0]` capturing: (a) lot_number/title synthesised from the notice-level scope with a `NO_EXPLICIT_LOT_STRUCTURE` warning; (b) twelve procedural requirements extracted verbatim in substance from sections 3.1, 3.2.1, 3.2.2, 3.4, 3.5, 3.6, 3.7, 3.8, 3.9, 3.11, 5.1, 5.2, 6; (c) two evaluation criteria from section 3.3 (Volume II compliance) and section 3.16 (diversity/gender tiebreaker with explicit <1% weighting text); (d) empty CPV, product_requirement and technical_standard arrays with accompanying warnings; (e) a primary `evidence_anchor` pointing at section 4.5 because that section explicitly names Volume II Specification as the carrier of technical content.
6. Populated ten top-level `evidence_anchors` spanning the document title, section 1 (Introduction), section 3.3 (Evaluation), section 3.8 (Validity), section 3.11 (Language), section 3.16 (Diversity and inclusion), section 4.5 (Volume II Specification), section 5.1 (Supplier registration), section 5.2 (Online questionnaire), and section 6 (Clarification questions).
7. Scored field confidences: `requirements` highest (7500) because they are verbatim anchored; `cpv_codes`, `product_requirements`, `technical_standards` and `product_requirement_seeds` zero because the document contains no supporting evidence; `lots` aggregate at 3500 to reflect role mismatch.
8. Emitted six warnings with in-document evidence refs: `DOCUMENT_ROLE_MISMATCH`, `NO_EXPLICIT_LOT_STRUCTURE`, `NO_CPV_IN_DOCUMENT`, `NO_TECHNICAL_STANDARDS_IN_DOCUMENT`, `NO_PRODUCT_REQUIREMENT_SEEDS`, `NO_EXPLICIT_EVALUATION_WEIGHTINGS`.
9. Wrote `result.json` to the session working directory. Validated structurally: all seven required top-level keys present, `extraction_metadata.target_domain="lots"`, `extraction_metadata.extraction_scope="full_document"` (matching the job parameter `request.extraction_scope="full_document"`), `policy_version="2026.04.12"` matching the upstream `extraction_policy_version`.

# Decisions Made

1. Emit exactly one synthesised lot rather than zero lots. The Docling markdown contains no explicit lot heading, but the skill's golden assertions mandate `min_lot_count: 1` and the document is itself an Invitation to Tender that scopes a single procurement whose identifier (`C3S2_413_bis Lot 2 ...`) is carried on the bound notice row. Emitting zero lots would either violate the golden contract or force the daemon to treat the document as out-of-scope; emitting one ITT-scoped lot with an explicit `NO_EXPLICIT_LOT_STRUCTURE` warning preserves auditability while satisfying the contract. Evidence anchor on the lot points at section 4.5, the only in-document reference that defines the specification boundary.
2. Do not promote the notice-level CPV `73000000-2` onto `lots[0].cpv_codes`. SKILL.md non-goals explicitly forbid "guessing ... without citing evidence anchors"; the CPV string does not appear anywhere in the Docling markdown. Emitted `NO_CPV_IN_DOCUMENT` warning instead so downstream persistence can reconcile the notice-level CPV through a separate path without inheriting it through this skill's authority.
3. Emit zero `product_requirement_seeds`. The document contains no items, deliverables with technical attributes, or quantities that would satisfy the `product_requirement_seeds[*]` schema (requires `seed_key`, `candidate_name`, `summary`, `requirements`, `evidence_anchor`). The only deliverable-like language is the abstract definition at section 2 ("'Deliverable' means all those things to be created, manufactured or engineered for ECMWF as part of the Services ..."); this is definitional, not a requirement seed. Emitted `NO_PRODUCT_REQUIREMENT_SEEDS` warning so downstream stages do not treat this extraction as authoritative for product candidates.
4. Emit zero `technical_standards`. No EN/ISO/BS/IEC/IEEE/ASTM/ETSI references exist in the markdown. Emitted `NO_TECHNICAL_STANDARDS_IN_DOCUMENT` warning rather than inferring from domain knowledge about weather services.
5. Set `extraction_confidence_bps=2500` (low) to signal to the daemon and the validator overlay that this extraction does not carry the specification payload that a Volume II run would. This gives the daemon the latitude to mark the document_semantic_extraction as partial and prefer a Volume II extraction when that document is ingested under the same document_set.
6. Classify `outcome_class=partial` in the run manifest. The skill completed the contract cleanly and produced a schema-valid result, but the absence of product requirement seeds and technical standards for a document that upstream believed to be a technical specification is a recoverable partial outcome rather than a success.
7. Use `document_role=tender_instructions_and_conditions` in `extraction_metadata.document_role` (the schema allows an arbitrary string) rather than parroting the upstream `technical_specification` classification, so a downstream role-reconciliation step can detect the mismatch and flag the `document_extractions.document_role` column for reclassification.

# Warnings

1. `DOCUMENT_ROLE_MISMATCH` — Upstream classified the document as `technical_specification`, but the document is `Volume I: Instructions for respondents and Conditions of tender`. Evidence: document title, section 4.5 (Volume II Specification referenced as the separate carrier of technical specifications), section 3.3 (Evaluation defers to Volume II).
2. `NO_EXPLICIT_LOT_STRUCTURE` — Volume I does not enumerate lots. The emitted lot is synthesised from the notice title. Evidence: section 4. Structure of this ITT and section 4.5.
3. `NO_CPV_IN_DOCUMENT` — Zero CPV codes appear in the markdown. The notice-level CPV `73000000-2` was deliberately not propagated into `lots[0].cpv_codes`.
4. `NO_TECHNICAL_STANDARDS_IN_DOCUMENT` — Zero standards references (EN/ISO/BS/IEC/etc.) in the markdown. Volume II is the expected carrier.
5. `NO_PRODUCT_REQUIREMENT_SEEDS` — No product-like items, quantities, or technical attributes in Volume I. Volume III A (Pricing and deliverables) and Volume II Specification carry those fields.
6. `NO_EXPLICIT_EVALUATION_WEIGHTINGS` — Volume I does not disclose quality/price weightings. Only the section 3.3 Volume II reference and the section 3.16 diversity tiebreaker are in-scope signals.

# Evidence

Major claims in the result are each anchored to a specific section heading or passage in the supplied Docling markdown. Each evidence anchor below carries `page_ref` (best-effort reference since Docling stripped explicit page numbers in this extraction), `section_ref` (the exact section heading as emitted by Docling), and `anchor_text` (a literal excerpt):

- Lot scope synthesis and specification boundary — section 4.5: "Volume II Specification - This contains detailed technical specifications and requirements for each ITT." (`lots[0].evidence_anchor`, top-level anchor #7).
- Evaluation criterion #1 (Volume II compliance) — section 3.3: "ECMWF will evaluate the Tender based on the Tenderer's responses to ECMWF's requirements (Volume II of this ITT) and the questions in the eProcurement Portal as well as Tenderer's compliance with paragraphs 3, 4, 5, 6 and 7 of this Volume I." (`lots[0].evaluation_criteria[0].evidence_anchor`, top-level anchor #3).
- Evaluation criterion #2 (diversity tiebreaker with <1% qualification) — section 3.16: "In the event that multiple Tenderers present equally qualified Tenders (discrepancy lower than 1%), ECMWF will take into consideration the diversity and gender balance of each Tenderer's organisation as a tiebreaker when making the final decision." (`lots[0].evaluation_criteria[1].evidence_anchor`, top-level anchor #6).
- Tender validity requirement — section 3.8: "Unless otherwise specified in Volume IA, the Tender shall remain valid for nine months after the closing date for this ITT." (top-level anchor #4).
- Language requirement — section 3.11: "All Tender documentation, manuals and diagram labelling shall be written in English." (top-level anchor #5).
- Portal registration requirement — section 5.1: "To take part in any procurement exercise and to be able to express an interest in an opportunity, first you must register on the eProcurement Portal (ProContract) with details of your organisation." (top-level anchor #8).
- Submission status gate — section 5.2: "You must click the 'Submit response' button and the status of your response should read 'Submitted', otherwise your response will not be accessible to ECMWF and hence will be ineligible for evaluation." (top-level anchor #9).
- Clarification deadline — section 6: "Any questions concerning this ITT ('Clarification Questions') shall be submitted via the eProcurement Portal and must be received by ECMWF more than fifteen (15) days before the closing date." (top-level anchor #10).
- Buyer identity and programme — section 1 (Introduction): "The European Centre for Medium-Range Weather Forecasts (ECMWF) is an independent intergovernmental organisation supported by 35 states." (top-level anchor #2).
- Document identity — title block: "VOLUME I: Instructions for respondents and Conditions of tender - ISSUED BY: ECMWF Administration Department Procurement Section - Date: 30 January 2026 - Version: Final" (top-level anchor #1).

Daemon-observable facts supporting this manifest (available for the daemon evidence appendix):
- `document_extraction.id = f0d8d894-dd2a-478c-9094-930e122dd578` (from `state.db:document_extractions`).
- `document_extraction.source_content_hash = 982304ae02cae9dd63283bb146359198706f2aa239778930cba54ec092ab43f3` (reused as `extraction_metadata.source_markdown_hash`).
- `document_extraction.page_count = 10`, `section_count = 41`, `table_count = 1`, `image_count = 0` (per upstream Docling row).
- `document_extraction.document_role = "technical_specification"` (upstream classification, contradicted by in-document evidence — see `DOCUMENT_ROLE_MISMATCH` warning).
- `raw_document.storage_path = /Users/alinaaitova/Desktop/Testproject1/artifacts/downloads/procontract/783996/Volume-I-Instructions-Conditions-Jan2026.pdf`.
- `raw_document.source_url = https://procontract.due-north.com/RfxResponse/Index?rfxId=a12aa4e2-1037-f111-813c-005056b64545`.
- `contract_notice.title = "C3S2_413_bis Lot 2 Maintenance and Extension of C3S Windstorm and Storm Surge Services"` (used as input to lot synthesis, cited in the `NO_EXPLICIT_LOT_STRUCTURE` warning).
- `contract_notice.main_category = "73000000-2 - Research and development services and related consultancy services"` (not propagated to the lot CPV array — see `NO_CPV_IN_DOCUMENT` warning).
- Result artefact written: `result.json` in the session working directory; seven required top-level keys present; `extraction_metadata.target_domain = "lots"` (schema `const`-compliant); `extraction_metadata.extraction_scope = "full_document"`.
