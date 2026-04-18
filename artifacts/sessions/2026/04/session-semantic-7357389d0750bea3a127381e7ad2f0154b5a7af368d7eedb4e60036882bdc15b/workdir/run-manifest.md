---
manifest_version: 1
skill_id: pdf-extraction/tender-specification
skill_version: 1.0.0
job_id: job-41b0b5b3-463f-4992-829a-990dbeb7264e
session_id: session-semantic-7357389d0750bea3a127381e7ad2f0154b5a7af368d7eedb4e60036882bdc15b
execution_mode: contract
started_at: 2026-04-17T21:08:00Z
completed_at: 2026-04-17T21:12:00Z
outcome_class: success
items_processed: 1
items_skipped: 0
items_failed: 0
warnings_count: 5
agent_claimed_pages_visited: 29
agent_claimed_source_url: procontract-attachment://raw-pc-19574d11-0929-f111-813c-005056b64545-05cb78245f7024d22b0a22067d6613a4adc6248ebfa5d421d87fdce2fbb5f6dd
agent_claimed_fingerprint_state: not_applicable
agent_claimed_locale_state: en-GB
agent_claimed_currency_state: GBP
---

## Summary

Processed one tender specification document — Cambridgeshire County Council ITT "25112 Library Services Engage, Design and Fit out Contract - Ely Library", contract reference DN797266, document_extraction_id `04f8fe6b-9f71-411f-991e-64218eba02fe`, source_content_hash `f4f0474fa46510628de580d8751112aa97c7cd549cef448eaae4ff19d1191795`. Docling produced 29 pages, 61 sections, 6 tables for this source.

The ITT explicitly states in §1.7 that the procurement is not split into lots, so one authoritative lot was emitted. Evaluation criteria (50% price via MAT formula, 50% quality across six weighted questions, minimum 30/50 quality threshold) were extracted directly from §§3.3–3.7 and Appendix Two. Five product requirement seeds were emitted: engagement & design services, furniture supply & fit-out, Children's Space, flexible/future-proofed space, and an Appendix 4 external-spec marker. CPV codes `39155000-3` (Library furniture) and `79932000-6` (Interior design services) were carried from upstream `contract_notices.main_category` — the ITT body does not cite CPV itself. No product-level technical standards (BS/EN/ISO/accessibility) appear in this ITT body; all standards recorded are procedural/regulatory (Procurement Act 2023, FOIA, EIR, UK GDPR, TUPE, Climate Charter, MMP).

Result file `result.json` has been written to the workdir and validates against `result.schema.json`.

## Steps Taken

1. Located the bound source: queried `state/state.db` → `document_extractions` for `id='04f8fe6b-9f71-411f-991e-64218eba02fe'`, confirming `document_role='technical_specification'`, `processing_strategy='standard'`, `extraction_policy_version='2026.04.12'`, `expected_overlays_json=["pdf-extraction/tender-specification"]`, `page_count=29`, `section_count=61`, `table_count=6`, and `source_content_hash=f4f0474fa46510628de580d8751112aa97c7cd549cef448eaae4ff19d1191795`.
2. Dumped the Docling markdown (`extracted_markdown`, 73,149 chars) to `/tmp/extracted_markdown.md` and read it in full (883 lines spanning the cover page, List of Appendices, Sections 1–5, and Appendices One–Three).
3. Cross-referenced `contract_notices` (`id='pc-19574d11-0929-f111-813c-005056b64545'`) to recover the buyer name (`Cambridgeshire County Council`) and the CPV signals stored on `main_category` (`39155000-3`, `79932000-6`).
4. Identified lot structure from §1.7 (not split). Extracted contract value, dates, insurance thresholds, tender validity, evaluation model, tie-break procedure, pass/fail suitability gates, and the six weighted award questions from Appendix Two.
5. Constructed five product requirement seeds tied to lot 1, with one marker seed flagging that authoritative product detail lives in Appendix 4 Specification which is a separate ProContract attachment and not present in this extraction.
6. Recorded per-field confidence (bps) biased down on CPV, product-requirement detail and technical standards because of the external-appendix dependency.
7. Wrote `result.json` and validated it against `skills/pdf-extraction/tender-specification/result.schema.json` with `jsonschema` — validation passed.

## Decisions Made

- **Single lot, not zero.** §1.7.1 explicitly instructs "not being split into Lots", so the authoritative shape is one lot rather than an empty `lots[]`. `lot_number="1"` chosen as a stable key for downstream product_candidates linkage.
- **CPV codes attached but flagged as inferred.** The ITT body contains no CPV string; the codes came from `contract_notices.main_category`. Attached to preserve referential continuity, but `CPV_FROM_NOTICE_METADATA` warning emitted so the daemon can mark the provenance correctly.
- **Technical standards intentionally separated from product technical standards.** Only procedural/regulatory frameworks are cited in the ITT — listed them with accurate context, and raised `STANDARDS_ARE_PROCEDURAL_NOT_PRODUCT` so the pipeline does not misuse these as product-grade standards.
- **Appendix 4 treated as a first-class external reference rather than guessed at.** Added a marker seed and an `EXTERNAL_SPEC_NOT_EXTRACTED` warning so the daemon knows this payload is deliberately high-level and must be re-expanded once Appendix 4 is ingested — rather than risking hallucinated product requirements.
- **TOC / marginalia rendering artifacts downgraded.** Lines 55-120 and 160-190 of the markdown contain an out-of-order cascade of section numbers which appear to be a TOC rendering artifact. Emitted a `TABLE_OF_CONTENTS_HAS_STRUCTURAL_ARTIFACTS` warning and relied on header text rather than those loose numeric tokens.
- **Date OCR typos noted, not silently normalised.** The `§2.3` timetable shows `"30 h June 2026"` and `"7 th July 2026"` with OCR whitespace artefacts. Canonical dates were listed in a `DATE_OCR_AMBIGUITY` warning rather than asserted without provenance.
- **`extraction_confidence_bps=7200` (72%).** Pulled down from a structural baseline of ~95% by the external-spec dependency and the CPV inference.

## Warnings

1. `EXTERNAL_SPEC_NOT_EXTRACTED` — Appendix 4 Specification (and Appendix 3a Pricing Schedule, Appendix 5 Floor Plan, Appendix 8 MMP Operations Manual) are separate ProContract attachments and absent from this markdown; downstream must re-expand seeds after they are extracted.
2. `CPV_FROM_NOTICE_METADATA` — CPV codes `39155000-3` and `79932000-6` were carried from the notice, not cited in the ITT body.
3. `STANDARDS_ARE_PROCEDURAL_NOT_PRODUCT` — The only "standards" in the ITT body are regulatory frameworks, not product-level technical standards.
4. `TABLE_OF_CONTENTS_HAS_STRUCTURAL_ARTIFACTS` — Docling output between lines 55-120 and 160-190 contains misordered section numbers consistent with TOC / margin rendering; ignored for content extraction.
5. `DATE_OCR_AMBIGUITY` — Ordinal-suffix dates in §2.3 OCR'd with minor typos (e.g. `"30 h June 2026"`); canonical dates provided in the warning body.

## Evidence

All major claims in `result.json` are anchored to observable spans of the Docling markdown. Key anchors:

- **Lot count == 1** — §1.7 Lotting, anchor_text: "1.7.1 This procurement is not being split into Lots as it would be impractical to try and split what is one service and system required." (markdown line ~263).
- **Title & reference** — cover page, anchor_text: "Contract Name: 25112 Library Services Engage, Design and Fit out Contract - Ely Library Contract Reference No.: DN797266" (markdown line ~9).
- **Contract value** — §1.6 Contract Details - Contract value, anchor_text: "estimated total value ... £180,000 (inclusive of VAT) ... exclusive of VAT of £150,000" (markdown lines 248-249).
- **Contract term** — §1.5, anchor_text: "anticipated to commence on 7 th July 2026 ... period of six (6) months ... potential for a contract extension of up to six (6) months ... twelve (12) month defects period" (markdown lines 242-244).
- **50/50 split and scoring scale** — §3.3.4 and scoring table 0/2/4/6/8/10 Very weak…Exceptional (markdown lines 567-582).
- **Quality-question weightings and minimum threshold** — Appendix Two table (markdown lines 832-838); §3.6 "the minimum quality score to achieve is 30 points" (markdown line 672).
- **Tender validity 180 days** — §2.8.1 (markdown line 371).
- **Insurance thresholds** — §4.5.1 (markdown lines 761-764).
- **Pass/fail suitability gates** — Appendix One table including Schedule 6/7 exclusions, GDPR, Climate Charter, Insurance, MMP (markdown lines 805-812).
- **Pricing evaluation rule (cells D13 and J21)** — Appendix Three §3.7 (markdown line 870).
- **Appendix 4 Specification as external source of product detail** — List of Appendices row "Appendix 4 | Specification | Within the attachments section of the project on ProContract" (markdown line 24) and §1.4.3 (markdown line 237).
- **CPV provenance (inferred)** — upstream `contract_notices.main_category = "39155000-3 - Library furniture | 79932000-6 - Interior design services"` (daemon-observed metadata, no matching ITT anchor).
