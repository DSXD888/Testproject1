---
manifest_version: "1.0.0"
skill_id: pdf-extraction/tender-specification
skill_version: 1.0.0
job_id: job-4a1cb868-3925-4d92-a2aa-a786ce8dd18f
session_id: session-semantic-f85c859f62a6655c425414bc09b0107cfe028074f27bab63d65a92e36102de0f
execution_mode: contract
started_at: "2026-04-18T10:47:00Z"
completed_at: "2026-04-18T10:50:00Z"
outcome_class: completed_empty
items_processed: 1
items_skipped: 0
items_failed: 0
warnings_count: 6
agent_claimed_pages_visited: 0
agent_claimed_source_url: "null"
agent_claimed_fingerprint_state: "not_applicable_headless_contract"
agent_claimed_locale_state: "en-GB"
agent_claimed_currency_state: "GBP"
---

# Summary

Processed Docling markdown for document_extraction 83924daa-1a97-4c7c-b345-4a150d3c2419 (raw_document_id raw-pc-218726bd-9dda-f011-813c-005056b64545-e45f0777f68a08e1dbb621e7c0fa1661b436985e0ce5645654aa6ca05074c028) under process pc-218726bd-9dda-f011-813c-005056b64545 (ESPO notice "218_26 Specialist Vehicle Contract Hire/Leasing"). The document is the ESPO "Framework Schedule 3 — Form of Contract and Call-Off Terms" template (filename: "218_26 Form of Contract and Call Off Terms FINAL.docx"), a legal/contractual boilerplate to be populated at call-off time, not a tender technical specification. No lots, no product requirements, no evaluation criteria, and no technical standards are expressed in the document body — every such field is a bracketed placeholder deferred to a later buyer-side completion step or explicitly cross-referenced to the (separate) Framework Agreement. Result therefore contains `lots: []` and `product_requirement_seeds: []`, with six warnings and eight evidence anchors explaining the empty extraction. Document role reclassified to `call_off_terms_template` in `extraction_metadata.document_role` (the upstream `document_role = "technical_specification"` label appears to be incorrect for this file).

# Steps Taken

1. Read the bound SKILL.md and `result.schema.json` (277 lines) from `skills/pdf-extraction/tender-specification/` to anchor the output contract (required top-level keys, nested `lot.*` keys, evidenceAnchor `$def`).
2. Located input: queried `state.db.document_extractions` by id to retrieve `document_title`, `document_role`, `page_count` (0), `section_count` (704), `table_count` (10), `expected_overlays_json = ["pdf-extraction/tender-specification"]`, and extracted_markdown length 468,170 chars.
3. Dumped the Docling markdown to `/tmp/extracted.md` (1,737 lines) and computed `sha256 = 25ee1f4d39f1bd8cd3e9c6b53bd58a77e964c691d6d524be798e199b120c31f5` over the exact bytes stored in `document_extractions.extracted_markdown`.
4. Surveyed the document structure: enumerated 46 `## N` headings covering: Form of Contract; Master Contract Schedule; General Provisions (definitions, interpretation); Due Diligence; Hire Period; Supply of Goods and/or Services; Disaster Recovery; Monitoring; Disruption; Service Levels; Payment; Key Personnel; Staff; TUPE; IP & Data; Warranties; Liabilities/Insurance; Termination; Anti-discrimination; H&S; Environmental; Bribery; Records & Audit; Fraud; Transfer/Sub-contracting; Force Majeure; Variation; Entire Agreement; Third Parties; Notices; Disputes & Law; Schedule 1 (Service Levels); Schedule 2 (Implementation Plan & Milestones).
5. Cross-checked process_context on `contract_notices` row: title "218_26 Specialist Vehicle Contract Hire/Leasing", main_category string lists three CPV codes (34114000-9 Specialist vehicles; 34121000-1 Buses and coaches; 34134000-5 Flatbed and Tipper trucks). Confirmed no `notice_lots` rows exist for this notice, so there is no upstream lot metadata to cross-reference (and per skill rules, pre-existing lot rows would not be authoritative anyway).
6. Read candidate "specification-bearing" sections (§2 Master Contract Schedule Goods/Services/Standards cells; §45 Schedule 1 Service Levels table; §46 Schedule 2 Implementation Plan). Verified every specification-shaped cell is either a bracketed placeholder (`[     ]`, `[DD Month YYYY]`, `[X] [months/years]`, `[insert the Supplier's name]`, etc.) or boilerplate-only, and that the document explicitly defers substantive content: "Details of the Goods listed above can be found in schedule 1 of the Framework Agreement" (line 91) and "Details of technical standards can be found in clause 13 of the Framework Agreement" (line 93).
7. Decided against fabricating lots or product_requirement_seeds from the framework-title alone ("SPECIALIST VEHICLE CONTRACT HIRE / LEASING"), because the skill's explicit non-goal is "Guessing lot boundaries, standards, or product requirements without citing evidence anchors". The framework title is too abstract to anchor any seed.
8. Wrote `result.json` with empty `lots`/`product_requirement_seeds`, six warnings (DOCUMENT_ROLE_MISMATCH, ONLY_PLACEHOLDER_FIELDS, LOT_SIGNAL_EXTERNAL, CPV_FROM_NOTICE_ONLY, NO_EVALUATION_CRITERIA, NO_PRODUCT_REQUIREMENTS_IN_BODY), eight evidence_anchors, and `extraction_metadata.document_role = "call_off_terms_template"`.
9. Validated the JSON parses cleanly and all top-level required keys are present.

# Decisions Made

- **Emit empty `lots` rather than synthesising a single catch-all lot.** The document does not define lot boundaries; the framework (218_26) clearly has lots (implied by "Lot or Lots" drafting-note language in §2 goods/services rows), but they live in the Framework Agreement's Schedule 1, not in this document. Emitting a synthetic "Lot 1 — Specialist vehicle hire" would fabricate structure and violate the "do not guess lot boundaries without citing evidence anchors" rule. Preferred correct emptiness over invented completeness.
- **Emit empty `product_requirement_seeds` rather than minting a generic "specialist vehicle hire" seed.** A single seed keyed only on the framework title would have no attributes, no quantities, no specialisations, and no evidence anchor beyond the cover page. Per skill contract, that is not useful to downstream product_candidate writes.
- **Do not propagate process-level CPV codes into the (empty) `lots` array.** The skill explicitly accepts pre-existing notice-level CPV as cross-reference but requires lot-level CPV to be evidence-anchored to the tender document itself. The three CPVs on `contract_notices.main_category` come from the notice listing, not this document body, so they are noted in a warning instead of being attached to a fabricated lot.
- **Override upstream `document_role`.** The `document_extractions.document_role` column stores `"technical_specification"`, but the file is plainly a call-off terms template. Captured the corrected classification in `extraction_metadata.document_role = "call_off_terms_template"` and logged `DOCUMENT_ROLE_MISMATCH` so the daemon/ingestion layer can route this document correctly downstream (the `pdf-extraction/tender-specification` overlay binding was likely incorrect for this file).
- **Set `extraction_confidence_bps = 9200`.** This is confidence in the *negative* finding (that no lot/product/standards content is present), which is high because the determination rests on unambiguous textual evidence (bracketed placeholders, boilerplate drafting notes). High per-field confidences on `lots`/`product_requirement_seeds`/`evaluation_criteria`/`technical_standards` (9500 bps) reflect certainty that these fields are empty, not certainty that any specific items were extracted.
- **Outcome class `completed_empty`.** The skill ran to completion, the result.json is contract-valid, and the emptiness is a deliberate evidence-based decision, not a failure. This is distinct from `AGENT_SKILL_FAILED` or `AGENT_PARTIAL_FAILURE`.

# Warnings

Six warnings emitted in result.json, summarised here:

- `DOCUMENT_ROLE_MISMATCH` — Document is a Form of Contract / Call-Off Terms template, not a tender specification. Evidence: §1, §2, line 91 `Goods required [     ]`, line 93 `Technical Standards [     ]`.
- `ONLY_PLACEHOLDER_FIELDS` — All spec-shaped fields are bracketed placeholders. Evidence: §2 Master Contract Schedule; §45 Schedule 1; §46 Schedule 2.
- `LOT_SIGNAL_EXTERNAL` — Lot structure is explicitly cross-referenced to the Framework Agreement's Schedule 1 and technical standards to clause 13. Evidence: lines 91, 93.
- `CPV_FROM_NOTICE_ONLY` — Three CPVs exist on the parent notice row but are not cited in the document body. Not attached to any lot.
- `NO_EVALUATION_CRITERIA` — Schedule 1 Part A appendix is a blank 4-row table; no criteria. Evidence: §45.1.1, lines 1664–1671.
- `NO_PRODUCT_REQUIREMENTS_IN_BODY` — Only framework-title-level signal, too abstract to emit a seed. Evidence: lines 13, 15–17.

# Evidence

All claims are grounded in specific anchors inside the structural extraction packet:

- **Document identity as call-off terms template** (supports DOCUMENT_ROLE_MISMATCH, ONLY_PLACEHOLDER_FIELDS):
  - line 1: `FRAMEWORK SCHEDULE 3 – FORM OF CONTRACT AND CALL-OFF TERMS`
  - line 9: `**CALL-OFF TERMS**`
  - line 13: `**SPECIALIST VEHICLE CONTRACT HIRE / LEASING**`
  - line 23: `**ESPO BUYER DRAFTING NOTE:** Ensure that all drafting notes throughout this document that are intended for ESPO are removed from the final version issued with the ITT.` — proving this is a template shipped pre-ITT.
- **Bracketed placeholders in spec cells** (supports ONLY_PLACEHOLDER_FIELDS):
  - line 89: `Commencement Date [DD Month YYYY]` / `Expiry Date [DD Month YYYY]` / `Extension Period [X] [months/years]`
  - line 91: `Goods required [     ]` / `Services and Deliverables required [     ]`
  - line 92: `Performance/Delivery Location/Premises [     ]`
  - line 93: `Standards / Quality Standards [     ] / Technical Standards [     ]`
  - line 94: `Disaster Recovery and Business Continuity [     ]`
  - line 96: `Supplier Solution [         ]`
  - line 100: Milestones table row is captions-only ("Milestone Deliverables … Duration … Milestone Date … Customer Responsibilities … Delay Payments") with no data rows.
  - line 105: `Customer's Responsibilities [           ]`
  - line 108: `Contract Charges payable by the Customer [             ]`
  - line 110: Commercially Sensitive Information `[             ]`
- **Explicit deferral to the Framework Agreement** (supports LOT_SIGNAL_EXTERNAL):
  - line 91: `Details of the Goods listed above can be found in schedule 1 of the Framework Agreement.`
  - line 91: `Details of the Services listed above can be found in schedule 1 of the Framework Agreement.`
  - line 93: `Details of technical standards can be found in clause 13 of the Framework Agreement.`
- **Schedule 1 Appendix A is an empty service-level grid** (supports NO_EVALUATION_CRITERIA):
  - lines 1664–1671: column headers `60-70% | 71-90% | 91-95% | 96-100%` with four numbered rows `1.`, `2.`, `3.`, `4.` and all cells empty.
  - line 1672: `**CUSTOMER DRAFTING NOTE** : Insert the appropriate Service Levels above.`
- **No framework-level lot enumeration in the body** (supports LOT_SIGNAL_EXTERNAL):
  - Search across the 46 `##` headings surfaces no heading of the form `Lot <N>` / `Lot A/B/C` / `CPV <code>` / `ISO <NNNN>` / `BS EN <...>`.
- **CPV codes present on notice but not in document body** (supports CPV_FROM_NOTICE_ONLY):
  - `contract_notices.main_category = "34114000-9 - Specialist vehicles | 34121000-1 - Buses and coaches | 34134000-5 - Flatbed and Tipper trucks"` (process_context input).
  - No matching occurrences of `34114000`, `34121000`, or `34134000` inside `/tmp/extracted.md`.
- **Source bytes integrity** (supports all evidence claims):
  - `sha256(extracted_markdown) = 25ee1f4d39f1bd8cd3e9c6b53bd58a77e964c691d6d524be798e199b120c31f5` — recorded in `extraction_metadata.source_markdown_hash` for daemon-side verification.
