---
manifest_version: "1.0.0"
skill_id: "pdf-extraction/tender-specification"
skill_version: "1.0.0"
job_id: "job-2272d4e3-6612-4d95-a496-986f4c199b59"
session_id: "session-semantic-b39263c9f2f026cd47fe67b2892e92cea6585438985dd43b79f0b2370fd7e726"
execution_mode: "contract"
started_at: "2026-04-17T20:22:00Z"
completed_at: "2026-04-17T20:23:25Z"
outcome_class: "success_with_warnings"
items_processed: 1
items_skipped: 0
items_failed: 0
warnings_count: 6
agent_claimed_pages_visited: 0
agent_claimed_source_url: "none"
agent_claimed_fingerprint_state: "not_applicable"
agent_claimed_locale_state: "en-GB"
agent_claimed_currency_state: "GBP"
---

## Summary

Extracted one authoritative lot record and five service-scope product requirement seeds from the ESPO Framework 107_26 Tender Submission Document (document_extraction_id `b0a5a106-6aff-4874-b487-a667a8a9b6e2`, raw_document_id `raw-pc-3ba4e6b8-9cef-f011-813c-005056b64545-80f0e2f2fda909488aeb60313f8137977d8dc12309bbe51a9cde656ff5c54a38`, source_content_hash `4ffe5f2383d289eecf0bf586c9d40034f0f7c529de97dbbb2c60aca0e6dcbd58`, markdown sha256 `0fe22bcd0541d1e53becd5bc06d99c7623d3aed2722a30a6095c1f3538e2b24d`, 229 markdown lines, 7 tables, 14 sections, policy_version `2026.04.12`). The document is a bidder response template rather than the Specification itself, so lot structure was inferred from the document title, scope statement, pre-existing notice-level lot record (single non-lotted call-off), and the explicit §1/§2/§3/§5/§6 scaffolding; detailed evaluation criteria came from the six method statements in §2 with their stated weightings (3/3/5/4/4/1). No CPV codes and no explicit technical standards (e.g. BS EN 1176/1177/913) are cited in the Docling markdown, so both arrays are empty and corresponding warnings were emitted. Overall extraction confidence is 7200 bps (72%), reflecting high confidence in lot structure and evaluation criteria and low confidence in CPV/technical-standards coverage because those fields are not authored in this document.

## Steps Taken

1. Inspected the execution workdir and confirmed no file-system inputs; located inputs by querying `state.db` (authoritative operational store) rather than relying on a pre-materialised JSON packet. Source rows: `document_extractions.id = b0a5a106-6aff-4874-b487-a667a8a9b6e2`, `procurement_processes.id = proc-pc-3ba4e6b8-9cef-f011-813c-005056b64545`, `contract_notices.id = pc-3ba4e6b8-9cef-f011-813c-005056b64545`, `notice_lots` (1 row already present for lot `1`).
2. Dumped the full `extracted_markdown` (229 lines, ~30.6 kB) and re-hashed it with `sha256sum` to produce a stable `source_markdown_hash` — daemon's `source_content_hash` is the PDF byte hash, which differs from the markdown hash and is therefore not reused for this field.
3. Loaded the process/notice context: title `107_26 Inspection and Maintenance of Indoor Gymnasium, Fitness and Sport Equipment, and Outdoor Schools Play and Sport Equipment`, buyer `ESPO`, notice `main_category` hints `37400000-2` (Sports goods and equipment) and `50000000-5` (Repair and maintenance services), pre-existing single-lot record keyed `1` with `stage_run_id = stage-run-57a5f226-b1b8-4236-9510-0821c0ebd20d-proc-pc-3ba4e6b8-9cef-f011-813c-005056b64545` (matches job parameter `stage_run_id`).
4. Read the Docling markdown end-to-end; identified six structural blocks: title block (lines 1–22), §1 Response to Requirement and Specification (lines 41–64), §2 Non-Price Method Statements (lines 66–107), §3 Price (lines 109–139), §4 Additional Benefits (lines 141–153), §5 Confidential Information Schedule (lines 155–185), §6 Tendering Declaration (lines 187–229). Treated `line_N` as the stable `page_ref` format (document is a collapsed single-page Docling projection; same pattern already used by the existing notice_lots row).
5. Concluded single-lot structure: the document title, §1 compliance-with-specification block, and §2 method statements describe one undifferentiated scope; no lot enumeration (e.g. "Lot 1 …", "Lot 2 …") appears anywhere in the markdown. Kept `lot_number = "1"` to align with the existing `notice_lots.lot_key = "1"` row and avoid churning lot identity.
6. Captured lot requirements by walking §1 (compliance + geographical coverage), §2 (method-statement submission discipline, word limits, MI supply), §3 (Pricing Schedule completeness and Framework Fee allowance), §5 (confidentiality identification), §6 (tendering declaration clauses including 120-day validity and Central Digital Platform registration) — 16 discrete requirement statements in total.
7. Captured evaluation criteria from §2 question headers and their explicit weightings: Q1 Customer Engagement (3), Q2 Customer Site Visit Process (3), Q3 Inspection Report (5), Q4 Services Following Completion (4), Q5 Consistency of Standards (4), Q6 Social Value (1). Added a seventh criterion for Price pointing at Section 5 of the ITT as the external authority (this document only asserts that the Pricing Schedule must be completed and names no formula).
8. Derived five product-requirement seeds by splitting the document's compound scope phrase into its semantically distinct service categories: indoor gymnasium equipment, indoor fitness equipment, indoor sport equipment, outdoor schools play equipment, outdoor schools sport equipment. Each seed is explicitly a service-scope candidate (not a specific SKU), with `requirement_type = "service"`, installation-context attribute, and the same title-block evidence anchor.
9. Left `cpv_codes` and `technical_standards` empty on the lot and all seeds because neither is cited in the Docling markdown; recorded corresponding warnings (`NO_CPV_CODES_IN_DOCUMENT`, `NO_TECHNICAL_STANDARDS_CITED`) with evidence references so a downstream agent or reviewer can see exactly which text was (and was not) available.
10. Populated `field_confidences` per-field (0–10000 bps), setting high confidence (8500–9500) for lot identity, description, requirements and evaluation criteria, and low confidence (2000) for CPV and technical-standards coverage; rolled up to `extraction_confidence_bps = 7200`.
11. Assembled 28 `evidence_anchors` covering the title block, submission instructions, §1/§2/§3/§5/§6 headings, and key phrases that back the most material claims (framework dates, 9-regions mandatory coverage, method statement weightings, 120-day offer validity, FOIA/EIR handling, generic compliance-certificate clause).
12. Serialised the payload to `result.json` in the session workdir and validated it against `skills/pdf-extraction/tender-specification/result.schema.json` with `jsonschema` — passed (no schema violations, all required fields present, `target_domain = "lots"`, `extraction_scope = "full_document"`).
13. Authored this run manifest with the required frontmatter and narrative sections; all claims below are anchored to lines in the Docling markdown or to result-contract fields.

## Decisions Made

- **Single-lot emission with `lot_number = "1"`**: the document is a framework response form describing one undifferentiated scope, and the notice already carries a single `notice_lots` row with `lot_key = "1"`. Creating additional synthetic lots (e.g. splitting indoor vs outdoor) would fabricate structure the document does not assert and would destabilise the existing `lot_key` identity. Evidence: title block (line 2) and §1 heading (line 41) both treat the scope as one compliance block.
- **Seeds as service categories, not products**: the scope phrase `"Indoor Gymnasium, Fitness & Sport Equipment and Outdoor Schools Play & Sport Equipment"` cleanly decomposes into five installation/usage contexts. Each is emitted as its own seed so downstream search profiles can specialise (e.g. outdoor-play quotes vs indoor-fitness quotes), while the underlying service scope (inspection, maintenance, remedial work) remains shared. Evidence: title block (line 2), reinforced by §2 Q3 "inspection & maintenance report" (line 86).
- **CPV codes not copied into the lot**: the skill contract requires evidence anchors for CPV claims, and no CPV codes are present in the Docling markdown. The notice's `main_category` hints (37400000-2, 50000000-5) were deliberately not copied into `lots[].cpv_codes`; instead, a `NO_CPV_CODES_IN_DOCUMENT` warning names them as notice-level context so the daemon can route CPV assignment through the correct authority (notice/process, not this document).
- **Technical standards left empty**: the document cites "compliance certificates with relevant legislation and policy" (line 208) but names no specific standards. Emitting speculative standards (e.g. BS EN 1176 for outdoor play) would violate the "do not claim a standard reference unless the evidence section cites the relevant anchor" rule in SKILL.md. Added `NO_TECHNICAL_STANDARDS_CITED` warning listing the standards that would typically apply, flagged as expected-to-live-in-specification.
- **Price treated as a distinct evaluation criterion with external methodology**: §3 asserts that price is evaluated (via the Pricing Schedule, Appendix B) and that the methodology lives in Section 5 of the ITT. Emitted a price criterion without a `weighting_text` (none is present in this document) and raised `PRICE_EVALUATION_METHOD_EXTERNAL` so downstream readers do not mistake the absence for an omission.
- **`extraction_scope = "full_document"`**: job parameter explicitly requests full-document scope, Docling markdown is the full document (no chunk plan hash), and no retry/resume context was passed. Single-pass extraction per SKILL.md's "Resume Strategy" preference.
- **`document_role = "tender_submission_template"` rather than `"technical_specification"`**: the `document_extractions.document_role` field is set to `"technical_specification"` by the upstream ingestor, but the document is plainly a bidder response form referencing an external Specification. The skill result contract permits a role field in `extraction_metadata`, so the semantically correct role is reported here without attempting to mutate authority rows (per SKILL.md non-goals).
- **`source_markdown_hash` computed independently**: used sha256 of `extracted_markdown` (`0fe22bcd…`) rather than re-using the upstream `source_content_hash` (`4ffe5f23…`, PDF byte hash). These are two different artefacts and mixing them would misrepresent the payload that actually drove this extraction.
- **Confidence weighting biased away from speculation**: capped `extraction_confidence_bps` at 7200 so downstream consumers treat this record as "lot skeleton + weightings are trustworthy, product/CPV/standards breadth is not". Better to under-claim coverage this document cannot supply than to inflate the rollup.

## Warnings

Six warnings emitted in `result.json.warnings`:

1. `DOCUMENT_IS_SUBMISSION_TEMPLATE_NOT_SPECIFICATION` — payload is the bidder response form; authoritative specification content lives in Framework Schedule 1 / Section 2 of the ITT and Appendix B. Evidence: lines 25, 45, 113.
2. `NO_TECHNICAL_STANDARDS_CITED` — no explicit standards (BS EN 1176, BS EN 1177, BS EN 913, etc.) are named in the Docling markdown. Evidence: line 208 (generic compliance-certificate clause only).
3. `NO_CPV_CODES_IN_DOCUMENT` — no CPV codes appear in the document; notice-level hints (37400000-2, 50000000-5) noted but not copied into the lot record.
4. `SINGLE_LOT_INFERRED` — document describes a single scope; lot_number `1` chosen to match the pre-existing `notice_lots` row. Evidence: lines 1, 29.
5. `PRICE_EVALUATION_METHOD_EXTERNAL` — price evaluation methodology is referenced but not included (Section 5 of the ITT). Evidence: line 113.
6. `PRODUCT_SEEDS_ARE_SERVICE_CATEGORIES` — seeds are broad equipment service-scope categories, not product models. Evidence: line 2.

## Evidence

All material claims are anchored to lines in the Docling markdown of the bound `document_extraction_id = b0a5a106-6aff-4874-b487-a667a8a9b6e2` and/or to result-contract fields.

- **Framework identity and period** — lines 1–7 (`ESPO reference 107_26 ... Framework Period 24 June 2026 – 31 January 2028 ... with the option to extend up to 23 June 2030`) → drives lot title, lot description, and the framework-period narrative.
- **Submission channel and deadline** — lines 14, 21 (`uploaded ... www.eastmidstenders.org eProcurement Portal`; `Closing date for submission of tender 12 noon, 15 May 2026`) → drives requirements item 15.
- **Compliance-with-specification confirmation** — lines 45–47 → drives requirements item 1 and `DOCUMENT_IS_SUBMISSION_TEMPLATE_NOT_SPECIFICATION` warning.
- **Mandatory geographical coverage (9 regions of England)** — lines 52, 54–55 → drives requirements item 2 and the geographic attributes on all five product seeds.
- **Optional UK coverage (Scotland, Wales, NI)** — lines 58, 60–63 → drives requirements item 3.
- **Method-statement discipline (6 questions, 700/500-word caps)** — lines 72, 74 → drives requirements items 7–10 and the six evaluation-criteria records.
- **Method-statement weightings** — lines 82 (Q1, 3), 84 (Q2, 3), 86 (Q3, 5), 88 (Q4, 4), 90 (Q5, 4), 92 (Q6, 1) → drives `evaluation_criteria[].weighting_text`.
- **Pricing Schedule (Appendix B) requirement and Framework Fee** — lines 113, 115, 119, 125 → drives requirements items 4–6 and the Price evaluation criterion.
- **Social Value Percentage table** — lines 101–107 → drives requirements item 11 (five contract-value bands explicitly listed).
- **Discount settlement terms table** — lines 133–137 → drives requirements item 12.
- **Confidential Information Schedule (FOIA/EIR, PA23)** — lines 155–183 → drives requirements item 13.
- **Tendering Declaration clauses (CDP registration, non-collusion, 120-day validity, Framework Fee compliance)** — lines 187–229, with clause 6 (line 201) and clause 12 (line 208) explicitly cited → drives requirements items 14 and 16, plus the `NO_TECHNICAL_STANDARDS_CITED` warning's evidence_ref.
- **Scope decomposition into five service categories** — line 2 (`Indoor Gymnasium, Fitness & Sport Equipment and Outdoor Schools Play & Sport Equipment`) and line 86 (`inspection & maintenance report`) → drives the five entries in `lots[0].product_requirements` and `product_requirement_seeds`.

Daemon-observable evidence (to appear in the machine appendix):

- `result.json` present in session workdir and valid against `skills/pdf-extraction/tender-specification/result.schema.json` (1 lot, 5 seeds, 6 warnings, 28 evidence anchors, `extraction_confidence_bps = 7200`).
- `extraction_metadata.target_domain = "lots"`, `extraction_metadata.extraction_scope = "full_document"`, `extraction_metadata.source_markdown_hash = "sha256:0fe22bcd0541d1e53becd5bc06d99c7623d3aed2722a30a6095c1f3538e2b24d"` (matches sha256 of `document_extractions.extracted_markdown` for `b0a5a106-6aff-4874-b487-a667a8a9b6e2`).
- `extraction_metadata.policy_version = "2026.04.12"` matches `document_extractions.extraction_policy_version` for the bound extraction row.
- `stage_run_id` in the job request (`stage-run-57a5f226-…-proc-pc-3ba4e6b8-…`) matches the `notice_lots.stage_run_id` for lot `1`, confirming the extraction belongs to the current stage-run cohort.
- No side-effects performed: no writes to `state.db`, no writes to `storage.sqlite3` in the workdir (agent-mail unrelated), no network calls (`side_effect_class = none` per SKILL.md front matter).
