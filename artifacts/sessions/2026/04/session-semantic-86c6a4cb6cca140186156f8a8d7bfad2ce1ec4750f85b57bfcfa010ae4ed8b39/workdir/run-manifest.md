---
manifest_version: 1.0.0
skill_id: pdf-extraction/tender-specification
skill_version: 1.0.0
job_id: job-9bca8ff5-1896-448e-bdcd-0f4a0522c6e9
session_id: session-semantic-86c6a4cb6cca140186156f8a8d7bfad2ce1ec4750f85b57bfcfa010ae4ed8b39
execution_mode: contract
started_at: 2026-04-18T00:48:00Z
completed_at: 2026-04-18T00:52:00Z
outcome_class: success_with_warnings
items_processed: 1
items_skipped: 0
items_failed: 0
warnings_count: 5
agent_claimed_pages_visited: none
agent_claimed_source_url: none
agent_claimed_fingerprint_state: not_applicable
agent_claimed_locale_state: en-GB
agent_claimed_currency_state: GBP
---

## Summary

Processed a single Docling-extracted tender document (`document_extraction_id=ef9b577c-59b9-4696-b5f3-1d251ce6bca1`, title `Appendix-1-Evaluation-Criteria-Sheet-Glass`) associated with Bristol City Council notice `pc-7046e8ec-2734-ed11-8119-005056b64545` — a DPS for "Supply of Double Glazed Units and Sheet Glass". The document is a DPS Appendix 1 (evaluation criteria) that does not enumerate numbered lots. Emitted one synthetic lot covering the full DPS scope, two product-requirement seeds (DGU per BS EN 1279 and 6.4mm laminated Stippolytte single glazing), six evaluation-criteria entries (selection gate, financial standing, and four award criteria for the below-/above-£25k tiers), and BS EN 1279 as a technical standard. Output written to `result.json` (`extraction_metadata.target_domain = "lots"`, `extraction_scope = "full_document"`, `source_markdown_hash = 79e2eedb878fb6085ab91cd41032147301d7f221070b931c951c1eeb8c54b905`, `policy_version = 2026.04.12`, `extraction_confidence_bps = 8200`).

## Steps Taken

1. Read the bound SKILL.md and `result.schema.json` to confirm required contract shape (one `lots` array, `product_requirement_seeds`, `evidence_anchors`, `extraction_metadata.target_domain` = `lots`).
2. Fetched job context from the local daemon state database (`state/state.db`): `document_extractions` row for the supplied `document_extraction_id`, and the parent `procurement_processes` / `contract_notices` rows for buyer/title/CPV signals. Notice has no existing `notice_lots` rows — consistent with a single-scope DPS.
3. Streamed the Docling markdown (`extracted_markdown`, 218 528 bytes) into a scratch file and scanned it in full (220 markdown lines) covering Table A (selection criteria), Financial Standing Appraisal, §2 Award Criteria for Call-offs, Scoring Matrix, Pricing, Tie Breaker, and the contract-specific Section 14 (Q14.1.1–14.1.12).
4. Derived lot structure: document enumerates no numbered lots; Q1.20 references DPS "categories" but does not list them, so one synthetic `lot_number = "1"` was emitted for the whole DPS scope. Flagged the inference via `LOT_STRUCTURE_INFERRED` warning.
5. Derived evaluation criteria from (a) Table A pass/fail selection gate, (b) Financial Standing Appraisal Low/Med/High table, (c) the two price/quality/social-value weighting tables in §2.
6. Derived product-requirement seeds from Q14.1.1 + Q14.1.3 (DGU to BS EN 1279) and Q14.1.4 (6.4mm laminated Stippolytte). Populated `technical_attributes` using the explicit spec wording in Q14.1.3.
7. Attached CPV codes 14820000-5 (Glass) and 44221111-6 (Double-glazed units) from the parent notice's `main_category` — tagged with `CPV_FROM_NOTICE_CONTEXT` since they do not appear in the document body.
8. Wrote `result.json` to the workdir and verified it structurally (required top-level fields, required per-lot fields, required nested keys on evaluation-criteria / product-requirements / technical-standards / product-requirement-seeds / evidence-anchors / extraction-metadata — all present).
9. Wrote this run manifest.

## Decisions Made

- **Single synthetic lot rather than multi-lot split.** Document text is a DPS evaluation-criteria appendix and does not enumerate numbered lots. Q1.20 names "categories" but the category list is not present in this appendix; the parent notice has no `notice_lots` rows either. Per the skill's "do not invent lot boundaries without evidence" rule, emitted one lot (`lot_number = "1"`) and surfaced the inference as `LOT_STRUCTURE_INFERRED`.
- **CPV codes sourced from notice context, not document body.** The document body does not cite CPV numbers. The parent notice's `main_category` is `14820000-5 - Glass | 44221111-6 - Double-glazed units`, which is consistent with the document's Q14.1.x technical scope. Attached both and flagged with `CPV_FROM_NOTICE_CONTEXT`.
- **Evidence anchors use section/question refs, `page_ref = "unknown"`.** Docling `page_count = 0` for this extraction and the markdown contains no page delimiters. Every evidence anchor therefore uses `section_ref` (e.g. `§14 Contract Specific Questions, Q14.1.3`) with `page_ref = "unknown"`, and the condition is surfaced as `PAGE_ANCHORS_UNAVAILABLE`.
- **Award-criteria split into four distinct entries.** §2 tabulates different weightings for call-offs below £25k (price 100%) versus ≥£25k (price 20%–80%, quality 0%–60%, social value 20%), with quality scored via a 0–5 matrix. Emitted four separate evaluation-criteria entries so downstream consumers can see the tiered structure without having to re-parse the weighting table.
- **`extraction_confidence_bps = 8200`.** High confidence on evaluation criteria and technical standards (direct quotes from the body). Moderate reduction because (a) lot structure is inferred, (b) CPVs come from notice context, (c) the ≥£25k price weighting is a range with `xx%` placeholders, and (d) section 14 is described as "(Response Repairs & Bulk Orders)" on some questions and unqualified on others — suggesting the DPS may split scope internally in ways not visible in this appendix.
- **Stippolytte product requirement kept as a seed even though only one question references it.** Q14.1.4 is a direct Pass/Fail supply question, so the DPS expects suppliers to be capable of delivering 6.4mm laminated Stippolytte. Emitted it as a distinct product-requirement seed with an empty `technical_standards` list (no specific British/European standard cited for this product in-document).

## Warnings

1. **LOT_STRUCTURE_INFERRED** — Single synthetic lot emitted; document does not enumerate numbered lots. Evidence: `§1.20 Please indicate which categories you are applying for` (categories unlisted) and notice title `BSH/HRA/Supply of Double Glazed Units and Sheet Glass - DPS - Acceptance Round 5`.
2. **PAGE_ANCHORS_UNAVAILABLE** — Source extraction has `page_count = 0` with no page markers; evidence anchors use section refs with `page_ref = "unknown"`.
3. **CPV_FROM_NOTICE_CONTEXT** — CPV codes 14820000-5 and 44221111-6 come from the parent notice's `main_category`, not from the document body.
4. **PRICING_WEIGHTING_TEMPLATE_PLACEHOLDER** — The ≥£25,000 "Pricing Document" section carries unresolved `xx%` placeholders; exact price weighting is tailored per call-off ITT within the 20%–80% range published in §2.
5. **DUPLICATE_SECTION_NUMBERING** — Docling output contains repeated `1.1.` list-number prefixes within §1 Selection Criteria, the Financial Standing Appraisal, and §2 Award Criteria (source formatting quirk). Evidence anchors use explicit question numbers (1.20, 14.1.x, etc.) where available and human-readable section refs otherwise.

## Evidence

All major extraction claims are anchored to observable text in the Docling markdown. Full anchor list is available in `result.json` (`evidence_anchors` + per-lot / per-criterion / per-seed anchors). Key anchors backing the major decisions:

- **DPS scope** — `APPENDIX 1: EVALUATION CRITERIA` heading (document top); notice context (`BSH/HRA/Supply of Double Glazed Units and Sheet Glass - DPS - Acceptance Round 5`).
- **Selection gate** — Table A heading and §1.1 (`All bidders must achieve the minimum selection criteria in accordance within Table A below, to be admitted to the DPS.`).
- **Financial standing** — Scoring table (`Low risk | Pass; Medium Risk | Pass; High Risk | Fail`).
- **Below-£25k award** — Weighting table (`Weighting Range for call-offs with a value below £25,000 | Price | 100%`).
- **≥£25k award** — Weighting table (`Price | 20% - 80% | Quality | 0% - 60% | Social Value | 20%`).
- **Quality scoring matrix** — 0–5 matrix table covering Unacceptable through Excellent; §1.3 quality-question exclusion rule (`If the project evaluation team scores any area of a bid as either 0 (unacceptable) or 1 (serious reservations) in the Quality Questions section, the Council reserves the right to exclude that bidder from further consideration.`).
- **Social Value tie-breaker** — `In the event of a tie between two or more bidders, the bidder with the highest social value score will be awarded the contract.`
- **DGU technical spec + BS EN 1279** — Q14.1.1 (`BS EN 1279`) and the full Q14.1.3 spec line (dual seals, 28mm max, Al spacer, desiccant, A-rated warm edge, argon-filled, 5-yr guarantee).
- **Response-repair lead time** — Q14.1.2 (same-day collection within Bristol) and Q14.1.5 (within 2 working days for operative orders).
- **Bulk-order lead time** — Q14.1.6 (`Most bulk orders will be placed by the joiners shop at Wellington Road, St Pauls, Bristol. Orders from the joiners shop for planned programmes of work, will be made available within 4 working days of order.`).
- **Stippolytte** — Q14.1.4 (`Are you able to supply 6.4mm laminated Stippolytte single glazing?`).
- **Delivery/rejection service levels** — Q14.1.8 (packaging + delivery-note content) and Q14.1.9 (24h replacement at supplier cost).
- **CSCS / Living Wage / GDPR obligations** — Q11.1, Q12.1–12.2, Q13.1 in Table A.
- **CPV linkage** — `contract_notices.main_category = '14820000-5 - Glass | 44221111-6 - Double-glazed units'` (daemon-supplied context, not document body).
