---
manifest_version: 1.0.0
skill_id: pdf-extraction/tender-specification
skill_version: 1.0.0
job_id: job-3d18ecc0-7bd5-4c1c-9986-baa9f4f9e0b6
session_id: session-semantic-5d0def73fb91edd735cda9ca7c4a298de65c6dccf42b770e58cb4e7cedb322fc
execution_mode: contract
started_at: 2026-04-18T11:36:00Z
completed_at: 2026-04-18T11:37:00Z
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

## Summary

Extracted lot-level structure from document_extraction `a2695a10-e5ad-4c06-87c9-98bb278a5310`, a Docling markdown rendering of "FRAMEWORK SCHEDULE 3 – FORM OF CONTRACT AND CALL-OFF TERMS, CONTRACT REF 107_26" associated with ESPO contract notice `pc-3ba4e6b8-9cef-f011-813c-005056b64545` (title: "107_26 Inspection and Maintenance of Indoor Gymnasium, Fitness and Sport Equipment, and Outdoor Schools Play and Sport Equipment"). The document is a call-off contract template rather than a tender specification, so the run completes with `DOCUMENT_ROLE_MISMATCH` flagged and extraction_confidence_bps = 3000. A single lot row was emitted (mirroring the one notice-level lot) with generic contract-derived requirements, two non-scored evaluation-criterion placeholders, inherited CPV codes (37400000, 50000000), zero product_requirements / product_requirement_seeds (no product specs are present in the source document), and five "technical standards" that are all normative references rather than specific standard numbers.

## Steps Taken

1. Read `skills/pdf-extraction/tender-specification/SKILL.md` and `skills/pdf-extraction/tender-specification/result.schema.json` to confirm the required output shape and the `target_domain = "lots"` invariant.
2. Opened `state/state.db`; selected from `document_extractions` where id = `a2695a10-e5ad-4c06-87c9-98bb278a5310`. Confirmed `extraction_engine = docling`, `extraction_policy_version = 2026.04.12`, `page_count = 0`, `table_count = 10`, `section_count = 677`, `source_content_hash = 083c26c4f86d913df3c83f3f942831b6ce5e59923b26ffcd08b39165b69116b5`, `expected_overlays_json = ["pdf-extraction/tender-specification"]`, `document_role = technical_specification`.
3. Exported `extracted_markdown` (~422 KB, 1632 lines) to `/tmp/tender-spec-markdown.md` and surveyed the heading tree via `grep` on `^## ` / `^### ` markers.
4. Queried `contract_notices` (`id = pc-3ba4e6b8-9cef-f011-813c-005056b64545`) to load notice title, buyer (`ESPO`), and pipe-delimited `main_category = "37400000-2 - Sports goods and equipment | 50000000-5 - Repair and maintenance services"`.
5. Queried `notice_lots` for the same notice and confirmed a single pre-existing lot row (`lot_number = 1`, title matches the notice title).
6. Inspected clauses 1, 3.1 definitions (Equipment, Quality Standards, Technical Standards, Good Industry Practice), 6.1–6.5, 11.1–11.2, 13.2.2, 15.7–15.11, 18, 26, 28, and Schedule 1 Part A + Schedule 2 for concrete Goods/Services content. No Master Contract Schedule cells were populated (`Quality Standards [     ]`, `Technical Standards [     ]`, `Customer's equipment [     ]`, Service Level table rows all blank).
7. Decided to emit one lot driven by the notice context, zero product_requirement_seeds, and a `DOCUMENT_ROLE_MISMATCH` warning rather than fabricate product rows or specific standards.
8. Wrote `result.json` validating against `result.schema.json`; ran a Python `json.load` parse check to confirm structural validity and counted lots/warnings/anchors.
9. Authored this run-manifest.md per the skill's Run Manifest Instructions.

## Decisions Made

- **Treat the document as a Form of Contract, not a tender specification.** The title block "FRAMEWORK SCHEDULE 3 – FORM OF CONTRACT AND CALL-OFF TERMS" plus the boilerplate structure and pervasive `CUSTOMER DRAFTING NOTE` placeholders determine this. The `document_extractions.document_role = "technical_specification"` classification is treated as provisional and surfaced via a warning for buyer-side reclassification.
- **Emit one lot, seeded from notice context.** The notice records a single lot and the document's preamble (WHEREAS) explicitly names the same service scope. Inventing additional lots would contradict both sources.
- **Do not fabricate product_requirement_seeds or specific technical standards.** Neither the Master Contract Schedule nor the body clauses supply product-level detail, quantities, or specific standard numbers. Emitting guessed seeds (EN 1176, EN 957, LOLER, PUWER etc.) would violate the SKILL.md rule "Do not claim a lot split, product seed, or standard reference unless the evidence section cites the relevant anchor."
- **Inherit CPV codes from the notice and disclose that inheritance.** The document itself does not declare CPV codes; the two-way pipe-delimited `main_category` on the notice provides 37400000 and 50000000. The inheritance is explicitly flagged via `CPV_INFERRED_FROM_NOTICE_CONTEXT`.
- **Low extraction_confidence_bps (3000).** The lot identity and subject-matter string are well-supported (title block + notice), but the core deliverables of this skill (product_requirements, evaluation_criteria, specific technical standards) are largely unavailable in this document, so the bps reflect low net informational yield rather than parser failure.
- **`extraction_metadata.document_role` set to `"form_of_contract_call_off_terms_template"`** to give the daemon a corrected role hint without mutating the source row.

## Warnings

1. `DOCUMENT_ROLE_MISMATCH` — source is a call-off contract template, not a tender specification.
2. `NO_PRODUCT_REQUIREMENT_SEEDS` — Master Contract Schedule cells that would hold product-level detail are blank placeholders.
3. `EVALUATION_CRITERIA_NOT_PRESENT` — award criteria live in the ITT (ref 2026/S 000-002119), not in this form of contract.
4. `TECHNICAL_STANDARDS_PLACEHOLDER_ONLY` — Technical Standards and Quality Standards rows are blank `[     ]` placeholders; domain-specific standards (EN 1176, EN 957, LOLER, PUWER, HSE guidance) are not cited in this document.
5. `CPV_INFERRED_FROM_NOTICE_CONTEXT` — CPV codes were inherited from `contract_notices.main_category`, not asserted in the document.
6. `EXTRACTED_METADATA_DOCUMENT_ROLE_REVIEW` — provisional `document_role = "technical_specification"` on the extraction row should be re-classified to a contract-template role.

## Evidence

| Claim | Evidence anchor |
|---|---|
| Document is a framework call-off contract template for contract ref 107_26 | `sheet: Main / document title block` → "FRAMEWORK SCHEDULE 3 – FORM OF CONTRACT AND CALL-OFF TERMS ... Inspection & Maintenance of Indoor Gymnasium, Fitness & Sport Equipment and Outdoor Schools Play & Sport Equipment ... CONTRACT REF 107_26" |
| Tender evaluation was performed via a separate ITT, not this document | `sheet: Main / clause 1 (WHEREAS A)` → "ESPO placed a Tender Notice 2026/S 000-002119 on 12 January 2026 ... ESPO issued an Invitation to Tender for the provision of the Goods and/or Services" |
| Technical Standards and Quality Standards are unpopulated placeholders | `sheet: Main / Master Contract Schedule 'Standards' row` → "Standards Quality Standards [     ] Technical Standards [     ] CUSTOMER DRAFTING NOTE: Insert the appropriate standards above ..." |
| Supplier is bound to comply with Technical / Quality Standards and Good Industry Practice | `sheet: Main / clause 6.3.1` → "The Supplier shall at all times comply with the Technical Standards and the Quality Standards ... in accordance with the Law and Good Industry Practice" |
| Service Credits formula is the only quantitative performance metric; underlying Service Level rows are blank | `sheet: Main / Schedule 1 Part A appendix` → "Formula: 100% minus % of Service Level achieved = x% of the Contract Charges to be deducted from the next invoice" |
| Safeguarding / DBS checks are a mandatory compliance surface because Goods/Services may be delivered on school premises | `sheet: Main / clause 15.7` → "the Supplier will make checks in respect of such employees and volunteers with the Disclosure & Barring Service for the purpose of checking at an enhanced level of disclosure ... Children's Barred List" |
| CPV inheritance from notice context | `contract_notices.main_category` = "37400000-2 - Sports goods and equipment | 50000000-5 - Repair and maintenance services" |
| `source_content_hash` used as `source_markdown_hash` | `document_extractions.source_content_hash` = `083c26c4f86d913df3c83f3f942831b6ce5e59923b26ffcd08b39165b69116b5` |
