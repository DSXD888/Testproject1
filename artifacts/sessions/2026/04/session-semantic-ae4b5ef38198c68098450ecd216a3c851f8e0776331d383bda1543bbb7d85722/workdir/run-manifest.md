---
manifest_version: 1
skill_id: pdf-extraction/tender-specification
skill_version: 1.0.0
job_id: job-5b388548-135e-4dd7-b32c-9d292e52c285
session_id: session-semantic-ae4b5ef38198c68098450ecd216a3c851f8e0776331d383bda1543bbb7d85722
execution_mode: contract
started_at: 2026-04-18T09:41:35Z
completed_at: 2026-04-18T09:44:00Z
outcome_class: success_with_warnings
items_processed: 1
items_skipped: 0
items_failed: 0
warnings_count: 6
agent_claimed_pages_visited: 0
agent_claimed_source_url: null
agent_claimed_fingerprint_state: not_applicable
agent_claimed_locale_state: en-GB
agent_claimed_currency_state: GBP
---

# Run Manifest — pdf-extraction/tender-specification

## Summary

Extracted a single authoritative lot, evaluation criteria and a small set of technical standards from Docling markdown for document_extraction `0bac372b-7a53-4d3b-b0e9-ba693be2e811` (raw_document_id `raw-pc-f0f30f94-e22c-f111-813c-005056b64545-4db9e133…`), which is the "3-Response-Document" attached to ProContract notice `pc-f0f30f94-e22c-f111-813c-005056b64545` (Stockport Town Centre School — Multi-disciplinary Design Team Appointment, contract reference 020417). The document is a Procurement Specific Questionnaire (PSQ) Stage 1 supplier response template, not the tender specification itself; the authoritative specification is referenced as "Document 2" and is not embedded here. No product_requirement_seeds were emitted because the contract is a consultancy/services appointment with no goods-style product requirements. Extraction scope is `full_document`, target_domain is `lots`, and the result contract is written to `result.json` in the session workdir.

## Steps Taken

1. Read `document_extractions.extracted_markdown` from `state.db` for id `0bac372b-7a53-4d3b-b0e9-ba693be2e811` (293,102 bytes, 150 lines, zero sections/images/pages in Docling output; 11 tables).
2. Loaded `contract_notices` row `pc-f0f30f94-e22c-f111-813c-005056b64545` to recover the notice title, buyer and `main_category` (150000 - Consultancy) for cross-reference only.
3. Read the single existing `notice_lots` row for this process; confirmed the pre-existing lot_number is "1" and scope covers Architect, MEP, Structures & Civils, BR Principal Designer and CDM Principal Designer. Treated as a cross-reference only per skill contract (do not accept as authoritative if contradicted).
4. Scanned the markdown for lot boundaries, CPV codes, standards, evaluation weightings and page markers. Confirmed:
   - Header table declares contract 020417 and a Competitive Flexible Procedure.
   - Preliminary lot-selection question is answered "N/A" ⇒ single-lot procurement; no sub-lots to split.
   - Three scored 0-6 criteria with percentage weights: Relevant experience and contract examples (35%), Workforce and Resources (35%), Funding Model Approach (30%) — found via grep for `\([0-9]+%\)`.
   - Pass/fail criteria blocks in Part 1 and Part 3A.
   - Explicit regulatory/standard references: UK General Data Protection Regulation (Q18), Procurement Act 2023 30-day payment terms (Q25), CDM Principal Designer role (Q20). No ISO/BS/EN/RIBA standard references appear in the markdown.
   - No CPV codes anywhere in the source.
   - No page markers — only a single `<!-- image -->` at the top.
5. Computed the source markdown hash `sha256:5a42fa991670ccb8ade4d9996a6d239d7958bc1c109afae6da4d2e0fcee1a8ab` over the exact markdown payload passed through the skill.
6. Drafted the result payload: one lot entry keyed to lot_number "1", eight procedural/compliance requirements, six evaluation criteria (three scored + three pass/fail blocks), empty cpv_codes, empty product_requirements, three technical_standards entries, and one primary evidence_anchor on the lot. Added six warnings covering document-role mismatch, single-lot confirmation, no-product-requirements, no-CPV, no-page-markers, and the referenced external Document 2.
7. Set `extraction_confidence_bps = 6500` and field_confidences weighted by the amount of text actually carrying that signal (title/eval_criteria high; cpv/product_requirements low because they are structurally empty by design, not by failure).
8. Wrote `result.json` to the session workdir and validated it parses as JSON.
9. Wrote this `run-manifest.md` for daemon finalisation.

## Decisions Made

- **Emit one lot, lot_number "1".** The document is explicit that no separate lots apply ("Please confirm which lot(s) you wish to bid for? — N/A") and the notice carries a single lot. Inventing sub-lots from evaluation questions would be a fabrication. Stable lot numbering is preserved across any future resume.
- **Keep `product_requirement_seeds = []` and `product_requirements = []`.** The contract is a services consultancy appointment. The PSQ response template does not specify goods, quantities or technical product attributes. Per the skill's non-goals, guessing product seeds from the description would be unsupported by evidence anchors.
- **Leave `cpv_codes = []`.** The source markdown contains no CPV code. The notice-level `main_category` of "150000 - Consultancy" is a ProContract internal category, not an 8-digit CPV, so it is not copied in.
- **Include Part 1 and Part 3A pass/fail criteria alongside the three scored criteria.** The Part 3A Criteria block explicitly states "All Responses will be evaluated in terms of understanding and meeting the requirements as set out in the Specification (Document 2)"; this is the closest thing to an award-framework statement in the document and is worth preserving with an evidence anchor for downstream review.
- **Treat UK GDPR, Procurement Act 2023 and CDM PD as `technical_standards`.** These are explicit normative references in the document with named section/question anchors. RIBA Stages (mentioned in the pre-existing notice-level lot description) do not appear in this markdown and were therefore not included — the skill forbids guessing standards without anchors.
- **Flag document_role mismatch rather than silently accepting `technical_specification`.** The extraction record labels this document as `technical_specification`, but the content is a supplier response template. Downstream consumers should route further tender-spec extraction to Document 2.
- **Use `p.unknown` for every `page_ref`.** Docling did not emit page markers for this document. Anchoring via section/question text preserves audit utility without fabricating page numbers.
- **`extraction_scope = full_document`.** The job parameters request a full-document run; the payload fits comfortably within budget and was extracted in a single pass with no chunking.

## Warnings

- `document_role_mismatch` — PSQ response template, not the specification itself. Authoritative scope lives in referenced "Document 2".
- `single_lot_confirmed` — preliminary lot-selection question answered "N/A"; single-lot procurement.
- `no_product_requirements` — services/consultancy contract with no goods-style product seeds.
- `no_cpv_in_document` — no CPV code present in the source markdown; notice-level `main_category` "150000 - Consultancy" is not an 8-digit CPV and was not mapped.
- `no_page_markers` — Docling output carries no page-break annotations; all `page_ref` fields are `p.unknown`.
- `referenced_external_document` — the Specification (Document 2) is referenced by Part 3A Criteria but is not embedded here; it should be the target of a separate tender-specification extraction run.

## Evidence

| Claim | Anchor (section / text) |
|---|---|
| Document is a PSQ Stage 1 response template | Document header: "Procurement Specific Questionnaire (PSQ) - Stage 1" |
| Contract identity (020417, Stockport Town Centre School, Competitive Flexible Procedure) | Document header / Contract table: "Contract: Stockport Town Centre School – Multi-disciplinary design team appointment | Contract Reference: 020417 | Procedure: Competitive Flexible Procedure" |
| Single-lot procurement | Preliminary Question / Lot selection: "Please confirm which lot(s) you wish to bid for? | Insert details: N/A" |
| Part 1 Economic & Financial Standing pass/fail + Company Watch | Part 1 Criteria: "The information provided in response to Part 1 – Confirmation of Core Supplier Information will be used by the Council to assess the Economic & Financial Standing of Bidders …" |
| Q18 UK GDPR technical resources requirement | Part 3A / Q18 Data Protection: "Please confirm that you have in place, or that you will have in place by the award of the contract, the human and technical resources to perform the contract to ensure compliance with the UK General Data Protection Regulation …" |
| Part 3B Technical Ability (Scored 0-6) header | Part 3B header: "Technical Ability (Scored 0-6)" |
| Q19 Relevant experience 35% weighting | Part 3B / Q19: "Relevant experience and contract examples (35%)" |
| Q20 Workforce & Resources 35% weighting + multidisciplinary roles | Part 3B / Q20: "Workforce and Resources (35%) Part 1: Please name each member of the proposed multidisciplinary team (company, practice etc) covering each of the roles outlined in the Stage 1 tender documents (Architecture, M&E, Structures & Civils, BR PD & CDM PD)." |
| Q21 Funding Model Approach 30% weighting | Part 3B / Q21: "Funding Model Approach (30%) Please describe your proposed approach to developing and supporting a viable funding model for the Town Centre School project." |
| Q22–27 sub-contractor management pass/fail | Part 3A / Experience of Sub-Contractor Management: "The answers provided in your Responses to Questions 22 – 27 will be evaluated on the basis of pass/fail." |
| Q25 Procurement Act 2023 / 30-day payment terms | Part 3A / Q25: "Please confirm that for public sector contracts awarded under the Procurement Act 2023 you have systems in place to include (as a minimum) 30 day payment terms in all of your supply chain contracts …" |
| Specification external reference (Document 2) | Part 3A Standard Questions / Criteria: "All Responses will be evaluated in terms of understanding and meeting the requirements as set out in the Specification (Document 2)." |
| Source markdown hash | sha256 computed over the exact markdown passed through the skill: `sha256:5a42fa991670ccb8ade4d9996a6d239d7958bc1c109afae6da4d2e0fcee1a8ab` |
| Result contract artefact | `result.json` in session workdir; validated parseable JSON; matches `skills/pdf-extraction/tender-specification/result.schema.json` required-field set |
