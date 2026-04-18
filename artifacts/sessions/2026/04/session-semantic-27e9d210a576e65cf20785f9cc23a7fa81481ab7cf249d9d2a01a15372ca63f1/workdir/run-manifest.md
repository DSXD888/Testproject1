---
manifest_version: 1.0.0
skill_id: pdf-extraction/tender-specification
skill_version: 1.0.0
job_id: job-302cd1e1-1984-4efe-ba13-d067d8f9e5fd
session_id: session-semantic-27e9d210a576e65cf20785f9cc23a7fa81481ab7cf249d9d2a01a15372ca63f1
execution_mode: contract
started_at: 2026-04-18T00:00:00Z
completed_at: 2026-04-18T00:00:00Z
outcome_class: success_with_warnings
items_processed: 5
items_skipped: 0
items_failed: 0
warnings_count: 7
agent_claimed_pages_visited: 0
agent_claimed_source_url: https://procontract.due-north.com/RfxResponse/Index?rfxId=a25079fd-2f28-f111-813c-005056b64545
agent_claimed_fingerprint_state: not_applicable
agent_claimed_locale_state: en-GB
agent_claimed_currency_state: GBP
---

# Run Manifest — pdf-extraction/tender-specification

## Summary

Extracted a lots-domain semantic payload from document_extraction `1005b350-26cf-4899-886d-5a3a729fa080` ("Appendix-1-Evaluation-Criteria"), which belongs to the procurement process `proc-pc-7e2a6fb2-67a3-f011-813a-005056b64545` ("BSH/HRA/Metal Works – Dynamic Purchasing System (DPS) – Acceptance Round 6") published by Bristol City Council on the ProContract (Due North) source. The document is an evaluation-criteria appendix for a DPS. It defines five lots (1–5) in Section 14 "Organisational Technical and/or Professional Capability", each scoped to a distinct metal-works discipline (repair/modification; supply and installation; bespoke fabrication; balcony balustrade repair/replacement; chute repairs). Selection-stage and call-off award criteria are extracted from Sections 1–15 of Table A and the "Section 2 Award Criteria for Call-offs" subsection. Produced 5 lots, 16 product-requirement seeds, 11 evidence anchors, 7 warnings. Overall extraction confidence 8,200 bps, consistent with a well-structured tender pack where CPV codes are inherited from notice context (not cited in-document) and product attributes are category-level rather than SKU-level.

## Steps Taken

1. Read the bound SKILL.md and `result.schema.json` for `pdf-extraction/tender-specification` to confirm the result contract (target_domain = "lots", required lots / product_requirement_seeds / field_confidences / extraction_confidence_bps / warnings / evidence_anchors / extraction_metadata).
2. Queried the local state database (`state/state.db`) for the document extraction row keyed by `document_extraction_id=1005b350-26cf-4899-886d-5a3a729fa080`, and pulled `source_content_hash=f5282135b275a8f484f5fba4899bff23c468bcf52566efb3c8db853d5bff1beb`, `extraction_policy_version=2026.04.12`, `document_role=technical_specification`, and the full `extracted_markdown` (~263 KB).
3. Read the raw_documents row for context: source URL `https://procontract.due-north.com/RfxResponse/Index?rfxId=a25079fd-2f28-f111-813c-005056b64545`, storage path `artifacts/downloads/procontract/782604/Appendix-1-Evaluation-Criteria.docx`, content type `.docx` (Word-sourced — explains `page_count=0` from Docling).
4. Read the contract_notice row for `pc-7e2a6fb2-67a3-f011-813a-005056b64545` to capture the notice-level main_category CPV list: `44221300-8` (Gates), `44400000-4` (Miscellaneous fabricated products), `45223100-7` (Assembly of metal structures), plus the notice summary that explicitly enumerates only Lots 1–3.
5. Parsed the Docling markdown structurally: identified the "APPENDIX 1: EVALUATION CRITERIA" title, Section 1 "Selection Criteria" preamble, the large Table A containing rows 1–15 including Section 14 with 13 per-lot capability rows (14.1–14.13), Section 2 "Award Criteria for Call-offs" with price/quality/social-value weighting table and scoring matrix, and a Financial Standing Appraisal block with Low/Medium/High risk mapping.
6. Enumerated the five lots from Section 14's row-header structure: "Lot 1 — Repairs and/or modification to metal items", "Lot 2 — Supply only or Supply and Installation of metal items", "Lot 3 — Fabrication of bespoke metal work items…", "Lot 4 — Repair or Replace balcony balustrades", "Lot 5 — Chute Repairs". Each lot header row is followed by capability questions scoped to that lot.
7. For each lot, synthesised requirements from the capability question wording, evaluation_criteria from two layers (DPS-admission Pass/Fail from Table A + call-off award weightings from Section 2), product_requirements as category-level seeds citing the exact lot-row anchor text, and technical_standards covering CSCS, SSIP, insurance minima, Bribery Act, Equalities Act, GDPR, Living Wage, and (for Lots 4 & 5) hot-works + temporary fire prevention plan.
8. Built the 16-entry `product_requirement_seeds` array preserving the per-lot `lot_number` and keying each seed with a stable, human-legible `seed_key` (e.g., `lot2-manufactured-railings`, `lot5-chute-closure-plates`) so retries can merge stably.
9. Assembled `field_confidences`, `warnings`, `evidence_anchors`, and `extraction_metadata` (target_domain=lots, extraction_scope=full_document, source_markdown_hash + policy_version echoed from the DB row, document_role=technical_specification).
10. Wrote `result.json` to the session workdir, then validated against `result.schema.json` using `jsonschema`; schema validation passed (5 lots, 16 seeds, 11 anchors, 7 warnings).

## Decisions Made

- **Treat the document as authoritative over notice metadata for lot count.** The notice summary mentions only Lots 1–3, but Appendix 1 Section 14 explicitly contains header rows and lot-specific questions for Lots 1–5. Per the SKILL.md input rule "do not treat pre-existing lot rows as authoritative truth if the tender document contradicts them", I extracted five lots and raised warning `NOTICE_LOT_METADATA_INCOMPLETE`.
- **Inherit notice-level CPV codes rather than leave `cpv_codes` empty.** The document contains no explicit CPV references. Because the skill output requires `cpv_codes` on each lot, I attached the three notice CPVs and removed those that were a poor fit per lot (e.g., `44221300-8` "Gates" dropped from Lot 3 bespoke-fabrication and Lot 5 chute-only lots). Raised warning `CPV_INFERRED_FROM_PROCESS_CONTEXT`.
- **Use section/table-row references for `page_ref`.** Docling returned `page_count=0` for the `.docx` source (no page numbers). I used section numbers and table-row identifiers (e.g., `table-A-14.12`, `section-2`) so downstream readers can still locate anchors inside the structural payload. Raised warning `PAGE_REF_SECTION_BASED`.
- **Product_requirement_seeds are category-level, not SKU-level.** This Appendix is an evaluation-criteria document, not a product spec, so no dimensional, material, or finish attributes are present. I captured the item families named in each lot row ("manufactured railings", "bespoke signs", "chute closure plates", etc.) and flagged this via warning `PRODUCT_SPEC_ABSTRACTION_LEVEL` so downstream product-matching does not over-trust these seeds.
- **Model Social Value as a 20% award-criterion weighting only.** Section 2 defers the Social Value methodology to a separate appendix not present in this payload. Raised warning `SOCIAL_VALUE_APPENDIX_REFERENCED_BUT_NOT_EMBEDDED`.
- **Encode call-off award weightings as ranges.** The Section 2 weighting table specifies Price 20–80%, Quality 0–60%, Social Value 20% for ≥£25,000 call-offs, and Price 100% for <£25,000 call-offs. Exact weightings are tailored per call-off ITT; I preserved the range in `weighting_text` and raised warning `CALL_OFF_WEIGHTINGS_ARE_RANGES`.
- **Mirror CSCS/SSIP/insurance/Living-Wage standards into every lot.** These are cross-lot selection-criteria standards applying to all bidders on all lots, so they appear in each lot's `technical_standards` (with a single evidence anchor each). Lot 4 and Lot 5 additionally carry a "Hot Works Procedure + Temporary Fire Prevention Plan" standard because their lot-specific questions (14.10, 14.12) explicitly require it.
- **Set extraction_confidence_bps = 8,200.** Lots, requirements, and evaluation criteria are high-confidence (explicit headings + table rows). CPVs are inferred (−), product-requirement detail is coarse (−), and page-ref granularity is section-level (−). 8,200 bps reflects a well-structured but coverage-light extraction relative to a full tender-spec pack.

## Warnings

- `CPV_INFERRED_FROM_PROCESS_CONTEXT` — no explicit CPV codes in the document; inherited from notice main_category.
- `PAGE_REF_SECTION_BASED` — Docling page_count=0 for .docx source; section refs used.
- `NOTICE_LOT_METADATA_INCOMPLETE` — notice summary lists only Lots 1–3; Appendix 1 defines Lots 1–5; document treated as authoritative.
- `PRODUCT_SPEC_ABSTRACTION_LEVEL` — seeds are category-level item names, not SKU-level specs.
- `SOCIAL_VALUE_APPENDIX_REFERENCED_BUT_NOT_EMBEDDED` — Social Value methodology deferred to an appendix not present in this extraction.
- `CALL_OFF_WEIGHTINGS_ARE_RANGES` — ≥£25,000 call-off weightings are ranges (tailored per ITT), not single weights.
- `SELECTION_CRITERIA_DOCUMENT_TYPE` — document_role=technical_specification upstream, but Appendix 1 is primarily selection/award criteria; downstream should not expect dimensional product specifications here.

## Evidence

- Document identity — `document_extractions.id=1005b350-26cf-4899-886d-5a3a729fa080`, `source_content_hash=f5282135b275a8f484f5fba4899bff23c468bcf52566efb3c8db853d5bff1beb`, `extraction_policy_version=2026.04.12`, `document_role=technical_specification`, `page_count=0`, `section_count=7`, `table_count=4`, `extracted_markdown` length ≈ 269,154 characters.
- Document heading — "APPENDIX 1: EVALUATION CRITERIA" (markdown line 1).
- Section 1 preamble — "All bidders must achieve the minimum selection criteria in accordance within Table A below, to be admitted to the DPS." (markdown clause 1.1).
- Table A row 14 header — "Organisational Technical and/or Professional Capability — If not applying for a LOT, please state N/A".
- Table A Lot 1 header — "Lot 1 | Repairs and/or modification to metal items".
- Table A row 14.1 — "Please confirm you have carried out repairs and/or modification to metal items such as railings, posts, single/double gates, chutes, balcony balustrade, fencing".
- Table A row 14.2 — "To pass the organisation / consortium aggregated together must provide evidence and demonstrate in the last two years, have delivered similar projects."
- Table A Lot 2 header — "Lot 2 | Supply only or Supply and Installation of metal items".
- Table A row 14.3 — "supply only or supply and installation of metal items such as railings, posts, single/double gates, fencing, handrails, park benches and bin".
- Table A rows 14.4, 14.5, 14.6 — manufactured railings, bespoke railings, and multi-property gates/posts/railings examples required.
- Table A Lot 3 header — "Lot 3 | Fabrication of bespoke metal work items, such as lockers for parking meters, water taps, signs, specialist artwork".
- Table A rows 14.7, 14.8 — bespoke fabrication confirmation and last-two-years example.
- Table A Lot 4 header — "Lot 4 | Repair or Replace balcony balustrades".
- Table A row 14.10 — "risk assessment and method statement with details about the undertaking of hot works and the temporary fire prevention plan to be put in place for the duration of these works".
- Table A Lot 5 header — "Lot 5 | Chute Repairs".
- Table A row 14.12 — "onsite repairs to chutes and in particular the closure plates at the bottom end of the chute … The chutes form part of the fire prevention system and timely response would be required, please state your initial response time and completion of task time."
- Table A row 11.1 — CSCS certification requirement ("a suitable and current Construction Skills Certificate Scheme (CSCS) certification or equivalent").
- Table A row 6.1 — SSIP exemption clause ("registered membership of the Safety Schemes in Procurement (SSIP) forum").
- Table A row 4.1 — "Employers Liability being not less than £10 million".
- Table A row 4.3 — "Public Liability being not less than £5 million".
- Table A row 4.6 — Bribery Act 2011 compliance confirmation.
- Table A row 8.1 — Equalities Act 2010 evidence requirement.
- Table A row 13.1 — GDPR human/technical resources confirmation.
- Table A rows 12.1, 12.2 — Living Wage Foundation Rate.
- Section 2 "Award Criteria for Call-offs" weighting-range table — Price 100% (<£25,000) and Price 20–80% / Quality 0–60% / Social Value 20% (≥£25,000).
- Section 2 scoring matrix — 0 Unacceptable, 1 Serious reservations, 2 Minor reservations, 3 Satisfactory, 4 Good, 5 Excellent.
- Section 2 tie-breaker — highest Social Value score wins.
- Financial Standing Appraisal — Low risk Pass / Medium Risk Pass / High Risk Fail.
- Notice CPV main_category (contract_notices.id=pc-7e2a6fb2-67a3-f011-813a-005056b64545) — "44221300-8 - Gates | 44400000-4 - Miscellaneous fabricated products | 45223100-7 - Assembly of metal structures".
- Notice summary (contract_notices.summary) — "Lot 1 - Repairs and/or modification to metal items. Lot 2 - Supply only or Supply and Installation of metal items. Lot 3 - Fabrication of bespoke metal work items." (source of `NOTICE_LOT_METADATA_INCOMPLETE` warning — only 3 lots listed upstream vs 5 in the document).
- Raw source URL — `raw_documents.source_url = https://procontract.due-north.com/RfxResponse/Index?rfxId=a25079fd-2f28-f111-813c-005056b64545`.
- Raw storage path — `/Users/alinaaitova/Desktop/Testproject1/artifacts/downloads/procontract/782604/Appendix-1-Evaluation-Criteria.docx`.
- Result contract — `result.json` written to session workdir; `jsonschema.validate` against `skills/pdf-extraction/tender-specification/result.schema.json` returned no errors (5 lots, 16 seeds, 11 anchors, 7 warnings).
