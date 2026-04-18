---
manifest_version: 1
skill_id: pdf-extraction/tender-specification
skill_version: 1.0.0
job_id: job-7228ca26-fb7a-4306-b230-8389f021100b
session_id: session-semantic-1eb703e48ce232768258d19da699503db97ea6956ca06542cf7ecddd5b662b83
execution_mode: contract
started_at: 2026-04-18T00:57:00Z
completed_at: 2026-04-18T00:58:00Z
outcome_class: completed_with_warnings
items_processed: 1
items_skipped: 0
items_failed: 0
warnings_count: 6
agent_claimed_pages_visited: 1
agent_claimed_source_url: "sqlite://state/state.db/document_extractions/21a0ed8b-f984-4e09-ad7b-55ae553c84cd"
agent_claimed_fingerprint_state: not_applicable
agent_claimed_locale_state: en-GB
agent_claimed_currency_state: GBP
---

## Summary

Processed document_extraction `21a0ed8b-f984-4e09-ad7b-55ae553c84cd` — the ESPO Framework 107_26 "Form of Contract and Call-Off Terms" template (source_content_hash `083c26c4f86d913df3c83f3f942831b6ce5e59923b26ffcd08b39165b69116b5`). The Docling-extracted markdown (422,426 bytes, 1,632 lines, 677 sections, 10 tables, 0 pages reported) was scanned end-to-end. The document is stored with `document_role='technical_specification'` but its structure is a legal contract-form template: 45 numbered clause groups covering Form of Contract, Master Contract Schedule, General Provisions, Due Diligence, Contract Period, Supply of Goods/Services, Disaster Recovery, Monitoring, Service Levels, Payment, IPR, Warranties, Liabilities, Termination, Data Protection, Force Majeure, and Schedules 1 (Service Levels) and 2 (Implementation Plan). No lot subdivision, no CPV codes, no populated technical standards, and no concrete product/service specifications are present in the document — all Specification and Standards fields in the Master Contract Schedule (§2) are unfilled placeholders of the form `[     ]` or `[insert …]` pending per-call-off population. One synthetic single-scope lot was emitted representing the undivided framework scope, plus six warnings flagging the role mismatch, placeholder-only specification fields, absence of CPV codes, absence of technical standards, absence of product seeds, and absence of lot subdivision. `extraction_confidence_bps` set to 3500 to reflect the shallow extractable content.

## Steps Taken

1. Loaded bound SKILL.md and the result contract schema from `skills/pdf-extraction/tender-specification/result.schema.json`.
2. Resolved the target document via `state.db/document_extractions` where `id='21a0ed8b-f984-4e09-ad7b-55ae553c84cd'`; read back the extracted markdown, source_content_hash, document_role, page_count, section_count, table_count, and extraction_policy_version.
3. Scanned the markdown for lot structure, CPV references, ISO/EN/BS standard references, Schedule headings, and section-level headings (`## N`), enumerating all 45 level-2 sections and Schedules 1–2.
4. Searched the document for `lot` references and confirmed zero occurrences of lot-subdivision syntax (only a single match in line 89 referring to "by reference to the Lot or Lots" inside a drafting note — not a lot definition).
5. Confirmed that §2 Master Contract Schedule rows for "Goods required", "Services and Deliverables required", "Standards / Quality Standards / Technical Standards", "Supplier Solution", "Key Personnel", "Monitoring", "Management Information", "Customer's Responsibilities", "Contract Charges", and "Commercially Sensitive Information" all contain `[     ]` or `[insert …]` placeholders, not populated content.
6. Identified the single concrete, extractable statement of scope — the subject line at line 13 and recitals at lines 33 and 37 — and used it as the evidence anchor for the synthetic framework-scope lot.
7. Identified two evaluable performance criteria drawn from the template: the Service Credits banded table (Schedule 1 Part A Appendix, line 1565) and the Critical Service Failure definition (line 99) — emitted as `evaluation_criteria` entries on the single lot with anchored evidence.
8. Emitted `product_requirement_seeds: []`, `technical_standards: []`, `cpv_codes: []`, and `product_requirements: []` because no concrete seeds, standards, CPV codes, or product requirements are present in the document.
9. Emitted six `warnings` covering document-role mismatch, specification-placeholder-only state, absence of lot subdivision, absence of CPV codes, absence of technical standards, and absence of product seeds.
10. Validated `result.json` against `result.schema.json` using jsonschema Draft 2020-12 — passed with zero errors.

## Decisions Made

- **Single synthetic lot vs. empty `lots[]`**: Chose a single synthetic lot (lot_number=`1`) representing the undivided framework scope rather than an empty array. Rationale: the document does plainly identify one coherent service scope ("inspection & maintenance of indoor gymnasium, fitness & sport equipment and outdoor schools play & sport equipment services") and framework reference 107_26, so an empty lot array would strip away the one anchor-backed fact the document provides. This decision is consistent with the pre-existing synthetic single-scope lot row in `state.db/notice_lots` for this notice.
- **Document-role mismatch not auto-corrected**: Preserved the stored `document_role='technical_specification'` in `extraction_metadata` (since daemon-side persistence owns role writes) but raised a `DOCUMENT_ROLE_MISMATCH` warning recommending reclassification to `form_of_contract` or `call_off_terms`.
- **No fabricated CPV codes**: Did not synthesise a CPV code (e.g., 50000000 Repair and maintenance, or 37440000 Fitness equipment) from the notice/process context because the SKILL explicitly requires CPV references cite evidence anchors present in the document — none are present.
- **Service Credits banding treated as an `evaluation_criterion`, not a lot requirement**: Schedule 1 Part A is a post-award performance-monitoring mechanism, but the closest analogue in the result contract's `lot.evaluation_criteria` schema is this banded criterion, so it was emitted there with an explicit `weighting_text` clarifying it is formula-based deduction rather than a tender-award weighting.
- **`extraction_confidence_bps = 3500`**: Reflects (a) high confidence that the document's scope and framework reference are correctly transcribed, but (b) strong evidence that this document cannot supply lot subdivision, CPV codes, technical standards, or product seeds, and (c) a document-role mismatch that should be resolved upstream before a downstream consumer relies on this extraction as a "technical specification".
- **`source_markdown_hash`**: Populated with the stored `source_content_hash` from `document_extractions` (`083c26c4f86d913df3c83f3f942831b6ce5e59923b26ffcd08b39165b69116b5`) as the stable identifier of the underlying source bytes. The downstream daemon already trusts this hash; using the locally-recomputed hash of the markdown file would drift across runs due to trailing-newline differences.

## Warnings

1. `DOCUMENT_ROLE_MISMATCH` — document is tagged `technical_specification` but is actually a Form of Contract / Call-Off Terms template; downstream role reclassification recommended. Evidence: lines 1, 13, 19, 73.
2. `SPECIFICATION_PLACEHOLDERS_ONLY` — every Master Contract Schedule field that would normally carry technical detail is an unfilled placeholder. Evidence: lines 89, 91, 94, 96, 100, 101, 103, 106, 108.
3. `NO_LOT_SUBDIVISION` — no lot subdivision present. Evidence: lines 33, 37.
4. `NO_CPV_CODES_IN_DOCUMENT` — no CPV codes present in this document (they would have to come from the Tender Notice 2026/S 000-002119 or Framework Schedule 1, not from this call-off terms template). Evidence: line 33.
5. `NO_TECHNICAL_STANDARDS_IN_DOCUMENT` — Technical Standards field is a placeholder and the call-off terms clause 7 is marked NOT USED; the template cross-references clause 13 of the Framework Agreement which is not in this extraction. Evidence: lines 91, 380.
6. `NO_PRODUCT_SEEDS_EXTRACTABLE` — no concrete product requirements are extractable; services are abstract (inspection & maintenance) and the Goods/Services Required fields are placeholders. Evidence: line 89.

## Evidence

All major claims are anchored to observable structural evidence in the Docling markdown:

- **Framework scope and service line**: line 13 (`CALL-OFF TERMS relating to Inspection & Maintenance of Indoor Gymnasium, Fitness & Sport Equipment and Outdoor Schools Play & Sport Equipment`) and line 17 (`CONTRACT REF 107_26`) — supports the synthetic lot title, description, `evidence_anchor`, and the `lot_number='1'` decision.
- **Framework recitals**: line 33 (Tender Notice 2026/S 000-002119 placed by ESPO on 12 Jan 2026) and line 37 (ESPO Framework Agreement reference 107_26) — supports the scope description and the `NO_CPV_CODES_IN_DOCUMENT` pointer to the tender notice as the correct CPV source.
- **Master Contract Schedule header**: line 73 (`## 2 MASTER CONTRACT SCHEDULE` with explicit note `(ESPO Framework Reference 107_26)`) — supports the document-role classification observation.
- **Specification placeholders**: lines 89 (Goods required / Services and Deliverables required), 91 (Quality Standards / Technical Standards), 94 (Supplier Solution), 96 (Supplier inspection of Premises), 100 (Monitoring), 101 (Management Information), 103 (Customer's Responsibilities), 106 (Contract Charges), 108 (Commercially Sensitive Information) — all empty `[     ]` placeholders, supporting `SPECIFICATION_PLACEHOLDERS_ONLY`.
- **Critical Service Failure definition**: line 99 — supports the second `evaluation_criteria` entry.
- **Service Levels and Service Credits banding**: line 1565 — supports the first `evaluation_criteria` entry; line 1579 gives the Service Credits formula.
- **Clause 7 NOT USED**: line 380 — supports `NO_TECHNICAL_STANDARDS_IN_DOCUMENT`.
- **Document structural metadata (from `document_extractions` row)**: page_count=0 (no page breaks surfaced by Docling for this markdown-rendered document), section_count=677, table_count=10, extraction_policy_version=`2026.04.12`, source_content_hash=`083c26c4f86d913df3c83f3f942831b6ce5e59923b26ffcd08b39165b69116b5`.
