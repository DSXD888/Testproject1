---
manifest_version: "1.0.0"
skill_id: pdf-extraction/tender-specification
skill_version: "1.0.0"
job_id: job-8b176959-ecae-45b2-8476-6e370eee9d37
session_id: session-semantic-06202924c221bfd38f58f44fc36dd07ff36dcc4e7c2d639be98c396135516f88
execution_mode: contract
started_at: "2026-04-18T00:00:00Z"
completed_at: "2026-04-18T00:00:00Z"
outcome_class: success_with_warnings
items_processed: 1
items_skipped: 0
items_failed: 0
warnings_count: 6
agent_claimed_pages_visited: 0
agent_claimed_source_url: "document_extraction://69a0f0f4-5e3c-484f-97ae-1829e7754d5e"
agent_claimed_fingerprint_state: not_applicable
agent_claimed_locale_state: en-GB
agent_claimed_currency_state: GBP
---

## Summary

Extracted one lot's worth of structured tender metadata from the Docling-produced markdown for ITT "25112 Library Services Engage, Design and Fit out Contract - Ely Library" (Cambridgeshire County Council, contract reference DN797266, notice id `pc-19574d11-0929-f111-813c-005056b64545`).

The procurement is explicitly single-lot per §1.7.1, so one lot record is emitted (`lot_number: "1"`). Seven product requirement seeds were derived from the ITT body and Appendix 3 pricing structure — covering the engagement/design service, the fit-out-and-supply service, the core library furniture line, the Children's Space fit-out, signage/waypoints, bespoke furniture, and handover/skills transfer. Thirteen technical standards were captured, anchored to Procurement Act 2023 clauses, UK GDPR / FOIA / EIR, TUPE, CCC Climate Charter, Match My Project registration, and the Find a Tender service / Central Digital Platform Unique Identifier requirement.

CPV codes (39155000-3 Library furniture; 79932000-6 Interior design services) were taken from the notice-level `main_category` because the ITT body does not carry explicit CPV codes — flagged via warning `CPV_CODES_FROM_NOTICE_ONLY`. Detailed product specification is carried in Appendix 4 Specification (external to this ITT document), flagged via warning `EXTERNAL_SPECIFICATION_REFERENCED`.

`extraction_confidence_bps`: 8500. Outcome class: success with warnings. `result.json` written to workdir at `/…/workdir/result.json`, target_domain = `lots`, extraction_scope = `full_document`.

## Steps Taken

1. Verified execution context: job params declared `extraction_scope=full_document`, `target_domain=lots`, `document_extraction_id=69a0f0f4-5e3c-484f-97ae-1829e7754d5e`, `raw_document_id=raw-pc-19574d11-0929-f111-813c-005056b64545-928e79e035ca19643eb1b6e0a55062e1fdad888d6b9a8bece6a6a6967f3d5f7d`.
2. Located the Docling-produced markdown via `state.db` `document_extractions.extracted_markdown` (length 73,149 chars, 883 lines; `extraction_engine=docling`, `section_count=61`, `table_count=6`, `page_count=29`, `document_role=technical_specification`).
3. Loaded notice-level metadata from `contract_notices` for process context: buyer = Cambridgeshire County Council; title = "25112 Library Services Engage, Design and Fit out Contract - Ely Library"; `main_category = 39155000-3 - Library furniture | 79932000-6 - Interior design services`.
4. Walked the document end to end: List of Appendices (p.2) → Contents (p.2) → Scope and Context (p.6–8) → Instructions to Bidders (p.9–14) → Tender Evaluation Process (p.16–22) → Additional Information (p.23–24) → Due Diligence (p.25) → Appendix One Suitability (p.26) → Appendix Two Award Criteria (p.27) → Appendix Three Pricing Schedule (p.28).
5. Confirmed single-lot status from §1.7.1 ("This procurement is not being split into Lots"). Emitted one lot with `lot_number="1"` and primary lot evidence anchor pointing at §1.7.1.
6. Extracted evaluation structure: MAT methodology, 50/50 Price/Quality split, six method-statement questions with per-question sub-weightings (25/20/20/15/15/5), 0/2/4/6/8/10 even-only scoring scale, 30/50 minimum quality threshold (§3.6), price scoring formula (§3.3 Pricing), tie-breaker procedure (§3.7), and Abnormally Low Bids policy (§3.8). Captured as thirteen `evaluation_criteria` entries on the lot.
7. Extracted contract term, value, insurance, and social-value/climate/MMP requirements for the `requirements[]` array, each anchored to the originating section.
8. Derived seven product requirement seeds from Appendix 3 pricing structure (§3.3.1, §3.4) and related scope clauses (§1.4, Quality Q3 / Q6). Emitted stable `seed_key` values (`engagement-design-service`, `fit-out-furniture-supply`, `library-furniture-core`, `childrens-space-fit-out`, `signage-waypoints`, `bespoke-furniture`, `handover-skills-transfer`).
9. Pulled 13 technical/regulatory standards grounded in ITT clauses.
10. Computed `source_markdown_hash = sha256(extracted_markdown) = 1202badd27f3fb507928d60bb19628861151eeac703eec782f7185b5b8e02a7e`.
11. Wrote `result.json` to the workdir conforming to `result.schema.json`. Validated structurally with `json.load` (1 lot, 7 seeds, 6 warnings, 14 evidence anchors).

## Decisions Made

- **Single lot emission (lot_number "1")**: §1.7.1 explicitly states the procurement is not split into lots; the daemon notice-level lot metadata should be reconciled against this. Recorded as warning `SINGLE_LOT_PROCUREMENT`.
- **CPV sourced from notice-level `main_category`**: The ITT body contains no CPV codes. Rather than omit them (which would weaken downstream candidate generation), sourced them from the already-structured `contract_notices.main_category` and flagged via `CPV_CODES_FROM_NOTICE_ONLY`. Field confidence on CPV reduced to 0.75 to reflect the provenance.
- **Product seeds derived from Appendix 3 structure, not Appendix 4**: Appendix 4 Specification is not contained in this ITT document (it is a separate ProContract attachment per the List of Appendices on p.2). Seeds therefore represent category-level requirement anchors rather than itemised product specs; detailed enrichment must come from downstream Appendix 4 ingestion. Warning `EXTERNAL_SPECIFICATION_REFERENCED` raised. `lots.product_requirements` field confidence set to 0.65 to reflect this.
- **Seed key stability**: Chose kebab-case descriptive seed keys (`engagement-design-service`, `fit-out-furniture-supply`, etc.) rather than UUIDs so that any retry of this extraction or subsequent enrichment from Appendix 4 produces idempotent joins.
- **Evaluation criteria emitted per-question (Q1–Q6) in addition to aggregate criteria**: This preserves weightings at a granularity that the quoteability scoring gate and downstream scoring engines can consume directly without re-parsing a monolithic "award criteria" blob.
- **Technical standards include soft regulatory references (Climate Charter, MMP, FATS/CDP, TUPE)** alongside statutory instruments: Each is a binding requirement in the ITT even where not a classical "technical standard" (EN/ISO). Anchored explicitly.
- **`extraction_scope: "full_document"`**: Matches the job-parameter `extraction_scope` and the fact that the entire 29-page markdown was processed in a single pass — within overlay budget per the SKILL.md resume strategy guidance.
- **Typos in §2.3 Key Timescales table** ("30h June 2026" and defects period "6 th January 2027") retained verbatim and flagged via warning `MINOR_DATE_INCONSISTENCY` rather than silently corrected.
- **Appendix Three internal numbering overlap** (§3.1–§3.13 inside the appendix shadowing §3.1–§3.8 in the evaluation body) flagged via `APPENDIX_3_SECTION_RENUMBERING` so downstream persistence can disambiguate without guessing.

## Warnings

| # | Code | Summary |
|---|---|---|
| 1 | `EXTERNAL_SPECIFICATION_REFERENCED` | Appendix 4 Specification and other appendices (2a, 3a, 5, 7, 8) carried as separate ProContract attachments — not inlined here. Product requirement seeds are intentionally coarse-grained and expect enrichment. |
| 2 | `SINGLE_LOT_PROCUREMENT` | §1.7.1 explicit non-lotting statement. One lot record emitted. |
| 3 | `CPV_CODES_FROM_NOTICE_ONLY` | 39155000-3 and 79932000-6 pulled from notice `main_category`; absent in ITT body. |
| 4 | `SCORING_ROUNDING_OBSERVED` | §3.3 example shows computed negative price scores floored at 0. Documented for scoring engines. |
| 5 | `APPENDIX_3_SECTION_RENUMBERING` | Appendix 3 reuses numbering 3.1–3.13 internally, overlapping with evaluation §3.1–§3.8. Requires disambiguation. |
| 6 | `MINOR_DATE_INCONSISTENCY` | "30h June 2026" and defects-period "6th January 2027" appear to be source PDF typos. Contract term (6 months from 7 July 2026) ends 7 January 2027, one day after stated defects period start. |

Total warnings: 6. No failures; no items skipped.

## Evidence

Every claim in `result.json` is anchored to a structural element present in the Docling markdown. The 14 top-level `evidence_anchors` cover:

- **Lot identification**: §1.7.1 Lotting — "This procurement is not being split into Lots" (p.8).
- **Contract identity**: cover page, contract reference DN797266 (p.1); List of Appendices (p.2).
- **Contract scope**: §1.4.1–1.4.3 (p.7) for engagement/design/fit-out framing; §1.5.1–1.5.3 (p.7) for term and defects period; §1.6.1–1.6.2 (p.7) for contract value.
- **Timetable**: §2.3 Key Activity and Timescales table (p.9) — submission deadline 12:00 noon, 22 May 2026.
- **Evaluation method**: §3.3.4.1–3.3.4.2 (p.17) for 50/50 Price/Quality split; §3.3.7 scoring table 0–10 (p.17); §3.6 minimum quality threshold 30/50 (p.20); §3.3 pricing formula (p.18).
- **Insurance**: §4.5.1 (p.24) for £5m/£5m/£2m/£1m insurance stack.
- **Suitability gate**: Appendix One table (p.26) covering Schedules 6 & 7 of Procurement Act, GDPR, Climate Charter, Insurance, MMP signup.
- **Award questions**: Appendix Two table (p.27) with six scored questions (Q1 25%, Q2 20%, Q3 20%, Q4 15%, Q5 15%, Q6 5%).
- **Pricing structure**: Appendix Three (p.28) — day-rate engagement/design and fit-out supply split, cell D13/J21 evaluated totals.

Each lot-level field (`requirements`, `evaluation_criteria`, `cpv_codes`, `product_requirements`, `technical_standards`), each product-requirement-seed, and each technical-standard entry carries its own `evidence_anchor` with `page_ref`, `section_ref`, and a verbatim `anchor_text` quotation present in the Docling markdown.

Artifact references:
- `result.json` — workdir-relative path, conforms to `pdf-extraction/tender-specification/result.schema.json`.
- Source markdown — `state.db/document_extractions.id=69a0f0f4-5e3c-484f-97ae-1829e7754d5e`, sha256 `1202badd27f3fb507928d60bb19628861151eeac703eec782f7185b5b8e02a7e`, 73,149 chars / 883 lines / 29 pages.
- Notice context — `state.db/contract_notices.id=pc-19574d11-0929-f111-813c-005056b64545`.

No operator memory or hidden context used. All claims derive from the Docling markdown, the result contract written this run, or daemon-provided job parameters.
