---
manifest_version: 1
skill_id: pdf-extraction/tender-specification
skill_version: 1.0.0
job_id: job-4c6f50c1-d6e1-4642-8255-96027a683e07
session_id: session-semantic-c542d868aa74f0105f9a42e45595b196980f4bf7d85cd90cdf4d1e8366894b48
execution_mode: contract
started_at: 2026-04-17T22:44:10Z
completed_at: 2026-04-17T22:45:46Z
outcome_class: success_with_warnings
items_processed: 3
items_skipped: 0
items_failed: 0
warnings_count: 7
agent_claimed_pages_visited: 0
agent_claimed_source_url: null
agent_claimed_fingerprint_state: not_applicable
agent_claimed_locale_state: en-GB
agent_claimed_currency_state: GBP
---

## Summary

Extracted lot structure for `document_extraction_id = b3267105-0386-4f88-8d7d-74b7c70e720d`, a DOCX rendered to Docling markdown (179,373 chars, 67 sections, 25 tables, 0 native pages). Source file: `Timber-Supplies-DPS-Agreement.docx` inside document-set `tds-pc-b7aaab27-1f34-ed11-8119-005056b64545` for process `proc-pc-b7aaab27-1f34-ed11-8119-005056b64545` — Bristol City Council's "BSH/HRA/Joinery Timber Supplies - DPS" Dynamic Purchasing System framework.

Three lots were identified directly from the document's cover-page Lot list and confirmed in Background clauses 1 and 3:

- Lot 1 — Supply and delivery of Softwood Timbers
- Lot 2 — Supply and delivery of Plywood and Door Blanks
- Lot 3 — Supply and delivery of Hardwood and Hemlock

The file is a DPS framework agreement rather than a true technical specification. Schedule 1 (Specification), Schedule 2 (Award Criteria weighting table), Schedule 3 (Pricing Matrices) and Schedule 7 (Call-Off T&Cs) are all either placeholders or references to separate documents. Consequently, `product_requirement_seeds` are emitted as `product_family_placeholder` entries at Lot grain, `technical_standards` arrays are empty, and `cpv_codes` arrays are left empty with a warning (notice-level CPVs 03419000 and 44220000 are not quoted in the document body itself). Overall `extraction_confidence_bps = 6800` reflects very high confidence on the lot skeleton but low confidence on product-level detail that is not present in this source.

## Steps Taken

1. Resolved the job parameters against `state/state.db`: fetched `document_extractions` row `b3267105-0386-4f88-8d7d-74b7c70e720d` (extraction_engine `docling` 2.88.0, processing_strategy `standard`, page_count 0, document_role `technical_specification`, policy_version `2026.04.12`).
2. Read process and notice context from `procurement_processes` and `contract_notices` — buyer "Bristol City Council", title "BSH/HRA/Joinery Timber Supplies - DPS - Acceptance Round 7", DPS framework, CPV codes present on notice metadata but not inside the document.
3. Read existing pre-populated `notice_lots` rows (lot-1 Softwood, lot-2 Plywood & Door Blanks, lot-3 Hardwood & Hemlock, plus a DPS-ALL umbrella row) as cross-reference only; did not treat them as authoritative — all three Lot titles were independently re-verified against the document's cover page and Background clauses.
4. Pulled the Docling markdown (179,373 chars) and computed source_markdown_hash = `5ddbab28177bafa48ee96a0e4c6918efd170993e93bea1b96a2310ca65cb23af`.
5. Grepped the markdown for Schedule/Lot/standard markers to confirm content scope. No BS EN / ISO / FSC / PEFC / grade / moisture / thickness references were found anywhere in the file, confirming the specification content is deferred to call-off stage.
6. Built one lot record per identified Lot, each carrying: title, description (synthesising the Lot scope with Bristol's BSH/HRA context), a requirements list keyed on DPS-level obligations (admission gate, insurance minimums, Good Industry Practice), evaluation_criteria covering the DPS-admission and Call-Off layers (with explicit notes that weightings are blank in this file), empty `cpv_codes`, placeholder `product_requirements`, empty `technical_standards`, and an evidence_anchor back to the cover-page heading.
7. Derived `product_requirement_seeds`: one per Lot for Lots 1 and 3 (softwood and hardwood respectively), plus a second seed for hemlock on Lot 3 (the Lot title explicitly names hemlock alongside hardwood), and two seeds for Lot 2 (plywood boards and door blanks, both named in the Lot title). All five seeds are `product_family_placeholder` candidates tied to the Cover / Background evidence anchors.
8. Captured seven structural warnings covering: empty Schedule 1 specification, blank Schedule 2 award-criteria weightings, CPVs not quoted in the document, deferred pricing schedule, external Schedule 7, absence of technical standards, and the document-role caveat (file is tagged `terms_and_conditions` by hint and `technical_specification` by extraction, but is effectively a framework agreement).
9. Emitted 17 global `evidence_anchors` covering cover page, Background clauses 1 and 3, Articles of Agreement (Term clause 2, Insurance table clauses 6-9), Clause 5 (Non-Exclusivity), Clause 9 (Tendering/Call-off), Clause 10 (Prices), Schedule 1, Schedule 2 (admission and call-off), Schedule 3, Schedule 5, and Schedule 7.
10. Wrote `result.json` at workdir root and validated it locally against `skills/pdf-extraction/tender-specification/result.schema.json` using `jsonschema` Draft 2020-12 — passes.

## Decisions Made

- **No new lot numbering or merging.** The document page-1 Lot list explicitly numbers three Lots; pre-existing `notice_lots` rows use the same numbering. I preserved `"1"`, `"2"`, `"3"` as string lot numbers; I did not emit a `DPS-ALL` lot because the document does not frame the DPS as a separate lot — it frames it as three lots only. The existing `DPS-ALL` row is an admin overlay from a different extraction.
- **Empty `cpv_codes` per lot.** The Lot titles and Background clauses do not quote CPV codes. Notice metadata supplies 03419000 (Timber) and 44220000 (Builders' joinery), but since the skill spec says "CPV references ... from Docling-produced structure", I declined to attach them here and flagged the absence in a `CPV_CODES_NOT_IN_DOCUMENT` warning.
- **Product-seed granularity driven by Lot titles.** Lot titles explicitly name sub-families (Plywood + Door Blanks; Hardwood + Hemlock), so I split Lot 2 into two seeds and Lot 3 into two seeds while keeping Lot 1 as a single softwood seed. I did not subdivide further (e.g. by species or thickness) because no such detail is present.
- **All product seeds are `product_family_placeholder`.** I deliberately did not claim concrete product specs (grades, sizes, moisture, certification) because the document defers all of that to call-off-time ITTs. Downstream candidate typing should treat these as placeholder families until a per-call-off specification document is processed.
- **Empty `technical_standards` arrays.** No BS EN / ISO / FSC / PEFC references exist in the file; inventing standards would breach the skill rule "Do not claim ... a standard reference unless the evidence section cites the relevant anchor."
- **Evaluation criteria kept at Lot level but marked weightings-unknown.** Schedule 2 has a blank admission-weightings table with only the text "[See Technical Assessment Criteria]", and the real evaluation grid is known to live in a separate companion doc (`Appendix 1 - Evaluation Criteria Joinery Timber Supplies.docx`). I mirrored that honestly in `weighting_text`.
- **Overall confidence 6800 bps.** Combines very high lot-structure confidence (≈9700) with low product-level confidence (≈4000) and very low CPV confidence (≈2000).
- **extraction_scope = "full_document".** The job request passed `extraction_scope = full_document` and the markdown was processed in one pass; no chunk merging occurred.

## Warnings

Seven warnings attached to `result.json` (all structural, none indicating data corruption):

1. `SPECIFICATION_SCHEDULE_EMPTY` — Schedule 1 is a placeholder; no product specs in-document.
2. `AWARD_CRITERIA_WEIGHTINGS_MISSING` — Schedule 2 admission criteria table blank; actual grid in separate Appendix 1.
3. `CPV_CODES_NOT_IN_DOCUMENT` — CPVs exist on the notice metadata but not in this file's body; per-lot `cpv_codes` left empty.
4. `PRICING_SCHEDULE_DEFERRED` — Schedule 3 explicitly marked "NOT REQUIRED UNTIL CALL FOR FURTHER COMPETITION STAGE".
5. `CALL_OFF_TERMS_EXTERNAL` — Schedule 7 points to the separate call-off contract DOCX.
6. `TECHNICAL_STANDARDS_ABSENT` — No BS EN / ISO / FSC / PEFC / industry standards named.
7. `DOCUMENT_ROLE_NOTE` — Bound as `technical_specification` by the DB role, but content is effectively a framework agreement; downstream consumers should expect product-level detail only from call-off-stage documents.

## Evidence

Major claims in the result payload are anchored to observable strings in the Docling markdown:

- **Three-lot split (cover page, top of document):** "A DYNAMIC PURCHASING SYSTEM AGREEMENT FOR THE PROVISION OF JOINERY TIMBER SUPPLIES DIVIDED INTO 3 CATEGORIES" followed immediately by "Lot 1 – Supply and delivery of Softwood Timbers", "Lot 2 – Supply and delivery of Plywood and Door Blanks", "Lot 3 – Supply and delivery of Hardwood and Hemlock". Confirmed by Background clauses 1 and 3 which enumerate the same three lots.
- **DPS framework nature, 4-year term:** Articles of Agreement §2 Term — "The DPS is expected to run for an initial period of 4 (four) years, from 1st December 2022." Background clause 1 describes it as a DPS; Clause 5 Non-Exclusivity — "no form of exclusivity or volume guarantee has been granted".
- **Insurance minimums (used for lot-level requirements):** Articles §6-9 Insurance table — "Employer's liability insurance | £10M each and every claim ... Public Liability Insurance | £5M each and every claim ... Professional indemnity Insurance | £2.5M each and every claim".
- **Call-off gating:** Clause 9.1.1 — "All admitted Providers shall be invited to submit a tender for each specific procurement under the DPS. Where the DPS has been divided into Lots, Providers which have been admitted to the relevant Lot corresponding to the specific procurement concerned will be invited to submit a Tender."
- **Pricing deferral:** Clause 10.1 — "The prices offered by the Provider for Call-Off Contracts ... shall be the prices listed in the Pricing Schedule submitted with the Provider's tender response for the Call-Off contract."
- **Schedule 1 placeholder (basis for SPECIFICATION_SCHEDULE_EMPTY warning):** "THE GOODS, SERVICES AND/OR WORKS SPECIFICATION — AS SPECIFIED IN ANY CALL FOR FURTHER COMPETITION IN RESPECT OF THE LOTS FOR WHICH THE PROVIDER IS ADMITTED TO THE DPS."
- **Schedule 2 blank weightings (basis for AWARD_CRITERIA_WEIGHTINGS_MISSING warning):** "Establishment on the DPS Criteria (Quality only) | Criterion Number | Criterion | Percentage Weightings" with an empty row, plus text "[See Technical Assessment Criteria]".
- **Schedule 3 deferral (basis for PRICING_SCHEDULE_DEFERRED warning):** "schedule 3 PRICING MATRICES — NOT REQUIRED UNTIL CALL FOR FURTHER COMPETITION STAGE".
- **Schedule 7 externalisation (basis for CALL_OFF_TERMS_EXTERNAL warning):** "SCHEDULE 7 CALL-OFF TERMS AND CONDITIONS — [Supply Call-Off Contract – See separate document]".
- **Source identity fingerprint:** Docling markdown SHA-256 `5ddbab28177bafa48ee96a0e4c6918efd170993e93bea1b96a2310ca65cb23af`, 179,373 characters, 67 extracted sections, 25 extracted table skeletons, 0 native pages (DOCX source).

Artifact references:
- Result contract: `workdir/result.json` (schema-validated against `skills/pdf-extraction/tender-specification/result.schema.json`, Draft 2020-12).
- Source markdown: `state.db` → `document_extractions.extracted_markdown` for id `b3267105-0386-4f88-8d7d-74b7c70e720d`.
- Document set: `state.db` → `tender_document_sets` id `tds-pc-b7aaab27-1f34-ed11-8119-005056b64545` (role map identifies this specific raw_document as hint `terms_and_conditions` within a four-document pack).
