---
manifest_version: 1.0.0
skill_id: pdf-extraction/tender-specification
skill_version: 1.0.0
job_id: job-e78862f1-ba38-447f-ae7c-22a278dbf459
session_id: session-semantic-f7945ca4cf726089ed1ce47adc66d9e97b48cf8bdd8aad05c7b7ddda7ebaa36b
execution_mode: contract
started_at: 2026-04-18T05:38:00Z
completed_at: 2026-04-18T05:40:00Z
outcome_class: success_with_warnings
items_processed: 1
items_skipped: 0
items_failed: 0
warnings_count: 6
agent_claimed_pages_visited: 1
agent_claimed_source_url: https://procontract.due-north.com/RfxResponse/Index?rfxId=ea9d90b2-4833-f111-813c-005056b64545
agent_claimed_fingerprint_state: not_applicable
agent_claimed_locale_state: en-GB
agent_claimed_currency_state: GBP
---

# Run Manifest — Tender Specification Extraction (Lot 1, Tender 6278)

## Summary

Extracted lot-scope semantic data from the Docling-rendered markdown of `6278 - PSQ Reference Appendices - Lot 1 Provision of Fire Risk Assessments.docx` (document_extraction_id `f23035d4-d707-4b22-8acc-bc74191b7a67`, document role `technical_specification` per stored extraction metadata). One lot was emitted (Lot 1 = Provision of Fire Risk Assessments), with one service-typed product requirement seed and seven referee evaluation criteria. The document is structurally a PSQ reference form (Appendix 1) plus a referee cover letter (Appendix 2) — its substantive technical content is sparse, so six warnings were emitted to flag administrative-form framing, absence of standards/CPV codes, and a referee-deadline placeholder inconsistency between Appendix 1 and Appendix 2. Overall extraction confidence is 6,200 bps, dominated by high confidence on lot identity and evaluation criteria but pulled down by absent standards and CPV signals.

## Steps Taken

1. Located the bound document via `document_extractions.id = f23035d4-d707-4b22-8acc-bc74191b7a67`; confirmed `raw_document_id`, `process_id`, `notice_id`, and `document_set_id` match the job request.
2. Read the entire `extracted_markdown` payload from `document_extractions` (34 lines) — this is the canonical Docling output for the bound raw document, not a chunk slice; therefore `extraction_scope = full_document` was set.
3. Cross-referenced the parent notice (`contract_notices.id = pc-dc1daee2-772e-f111-813c-005056b64545`) for buyer identity (London Borough of Bexley), framework title ("6278 - Building Services Framework - Lots 1-11"), and the canonical lot inventory; this established that Lot 1 is unambiguously "Provision of Fire Risk Assessments" both in the document body and in the parent notice summary.
4. Identified evaluation-axis text inside Appendix 1 / PART B and emitted each as a discrete `evaluation_criteria` entry, anchored to the verbatim referee question.
5. Constructed a single service-shaped `product_requirement_seeds` entry; declined to fabricate goods-shaped seeds because the document never mentions tangible goods or quantities.
6. Left `cpv_codes` and `technical_standards` empty for the lot because the document does not declare them; emitted explicit warnings rather than inheriting parent-notice CPVs (which span the whole 11-lot framework, not Fire Risk Assessments specifically).
7. Validated the assembled JSON against `result.schema.json` using `jsonschema.validate`; produced a 14-anchor evidence set tied to verbatim markdown phrases.

## Decisions Made

- **Single-lot output (Lot 1 only).** The bound document is explicitly scoped to Lot 1 by its filename, its bolded title (`**6278: Lot 1: Provision of Fire Risk Assessments**`), and the appendix structure. Other framework lots (2–11) are referenced only in the parent notice metadata — emitting them here would violate the "do not treat pre-existing lot rows as authoritative truth if the tender document contradicts them" guidance in reverse (it would manufacture lots from notice metadata rather than from this document's evidence).
- **`cpv_codes` left empty.** The notice carries three framework-wide CPVs (`50700000-2`, `50750000-7`, `71315000-9`) but none of these specifically denote fire-risk-assessment services (which would more plausibly fall under 71317100-4 or 71317200-5). Inheriting the framework CPVs at Lot 1 scope without textual evidence would inject misleading procurement signals; they are surfaced as a warning instead.
- **`technical_standards` left empty.** The document text contains no BS / EN / ISO / RICS / IFE / PAS references. Per the skill contract, standards must be evidence-anchored; emitting plausible standards (e.g. PAS 79) without textual evidence would constitute fabrication.
- **Evaluation criteria modelled as referee-rating dimensions, not as award criteria.** A warning explicitly labels these so downstream stages do not mistake the qualification-screening axes for the formal MEAT scoring matrix.
- **Service-typed seed, not goods-typed.** The lot purchases a recurring professional service. The seed is tagged `requirement_type = "service"` and downstream matching rules should not treat it as a sourceable good.
- **Confidence assignments.** Lot identity (`lot_number`, `title`) at 9500 bps because the document text is unambiguous. Evaluation criteria at 9000 bps (verbatim from the form). Description at 7000 bps (synthesised from notice cross-reference + document header). CPV at 1000 bps (no in-document evidence). Standards at 2000 bps (absence of evidence is strong, but downstream may still find some). Composite extraction_confidence_bps = 6200 reflects the thin substantive content.
- **Schema metadata.** `target_domain = "lots"` (skill-mandated). `extraction_scope = "full_document"` because the entire Docling markdown was processed. `source_markdown_hash` set to the canonical `source_content_hash` field stored on the `document_extractions` row (`a861…7aa7`) to remain stable across re-runs of the same Docling output.

## Warnings

1. `DOCUMENT_ROLE_MISMATCH_ADMINISTRATIVE_FORM` — Bound document is a PSQ reference form, not a substantive technical specification.
2. `NO_TECHNICAL_STANDARDS_PRESENT` — No BS / EN / ISO / RICS / IFE references in the document body.
3. `NO_INLINE_CPV_CODES` — Document declares no CPV codes; notice-level CPVs are framework-wide and not lot-specific.
4. `EVALUATION_CRITERIA_FRAMING` — Captured criteria are referee-rating dimensions for prior-experience screening, not the formal MEAT/award matrix.
5. `PRODUCT_REQUIREMENT_SEEDS_THIN` — Only one service-typed seed was emitted; document discloses no component-level technical attributes.
6. `REFEREE_DEADLINE_TEMPLATE_INCONSISTENCY` — Appendix 1 PART B states the referee deadline as "18th May 2026 by 5pm" while Appendix 2 cover letter retains the unresolved template placeholder `****INSERT DATE & TIME****`. The two appendices disagree.

## Evidence

- **Lot identity anchor:** `**6278: Lot 1: Provision of Fire Risk Assessments**` (Appendix 1 / Reference Form Header, p.1 of the Docling markdown).
- **Form scope anchors:** `## Appendix 1`, `**REFERENCE FORM**` (Appendix 1 heading, p.1).
- **Referee criteria anchors:** Six verbatim referee questions inside `Appendix 1 / PART B` covering supply chain resilience, service provision, resource, value for money, contract management, project management; plus the explicit "Would you employ this company for similar services (please select)  Yes  No" anchor.
- **Return-channel anchor:** `Please return the reference to: Procurement@bexley.gov.uk` (Appendix 1 / PART B / Return Instructions).
- **Deadline conflict anchors:** `TO BE COMPLETED BY REFEREE AND RETURNED TO THE ADDRESS BELOW BY 18th May 2026 by 5pm` (Appendix 1 / PART B) vs `before ****INSERT DATE & TIME****` (Appendix 2 / Cover Letter to Referee).
- **Cross-reference fact (notice level, not document body):** `contract_notices.id = pc-dc1daee2-772e-f111-813c-005056b64545` row stores `title = "6278 - Building Services Framework - Lots 1-11"` and `summary` enumerating Lot 1 as `"Provision of Fire Risk Assessments"`. Used for description corroboration only; not relied on for in-document claims.
- **Result-contract field references:** `result.json` `lots[0].lot_number = "1"`, `lots[0].title = "Provision of Fire Risk Assessments"`, `extraction_metadata.target_domain = "lots"`, `extraction_metadata.extraction_scope = "full_document"`, `extraction_metadata.source_markdown_hash = "a8612dc9a8fff0024bfa5d70a0d6609e5fa2833d6035c516a3c1fa22e3697aa7"`, `extraction_confidence_bps = 6200`, `warnings.length = 6`, `evidence_anchors.length = 14`.
- **Daemon appendix references:** `document_extractions.source_content_hash = a8612dc9a8fff0024bfa5d70a0d6609e5fa2833d6035c516a3c1fa22e3697aa7`; `document_extractions.extraction_engine = docling@2.88.0`; `document_extractions.expected_overlays_json = ["pdf-extraction/tender-specification"]`; `document_extractions.extraction_policy_version = 2026.04.12`; `raw_documents.storage_path` resolves under `artifacts/downloads/procontract/afb3b5dc-772e-f111-813c-005056b64545/6278 - Reference Appendices_extracted/Reference Appendicies/`.
