---
manifest_version: 1
skill_id: pdf-extraction/tender-specification
skill_version: 1.0.0
job_id: job-083597e2-0c97-48df-9ef9-40d1ac4018c9
session_id: session-semantic-dc304c8f33133ed94c5362e95dfb69eed2e6d40bcde8ffa81b49a2a793f9f7e5
execution_mode: contract
started_at: 2026-04-18T02:20:00Z
completed_at: 2026-04-18T02:22:50Z
outcome_class: completed
items_processed: 10
items_skipped: 0
items_failed: 0
warnings_count: 7
agent_claimed_pages_visited: 0
agent_claimed_source_url: https://procontract.due-north.com/RfxResponse/Index?rfxId=a9dc718e-aa2d-f111-813c-005056b64545
agent_claimed_fingerprint_state: not_applicable
agent_claimed_locale_state: en-GB
agent_claimed_currency_state: GBP
---

## Summary

Extracted the lot structure from Appendix A — Specification (CCP074 "Provision of a Print Service DPS"), a Derbyshire County Council Dynamic Purchasing System. The document splits the DPS into ten service lots (Lots 1–10). For each lot I produced an authoritative row with title, description, requirements, technical standards, CPV codes (inherited from process-level CPV signal and filtered by service-vs-goods character), an `evidence_anchor` pointing to the lot heading in the Docling markdown, and a set of product-requirement seeds drawn from each lot's explicit "Examples include (but not limited to)" list. The result-contract's `product_requirement_seeds` collection is populated in parallel with 26 seed rows spanning all ten lots, keyed against the same evidence anchors. `extraction_metadata.target_domain = "lots"` and `extraction_metadata.extraction_scope = "full_document"` match the job parameter. Result-contract `extraction_confidence_bps = 8200`.

## Steps Taken

1. Loaded the Docling markdown for `document_extraction_id = 7cc700e6-bb60-4ab6-ab71-e625312bc4d5` (engine docling 2.88.0, strategy standard) from `state.document_extractions.extracted_markdown`. Length 19,999 chars; `page_count = 0` because the source is a `.doc` converted via Docling and no page markers were preserved.
2. Cross-referenced the process (`proc-pc-169ba131-3b3f-ed11-8119-005056b64545`, title "CCP074 - Provision of a Print Service DPS - Acceptance Round 16", buyer "Derbyshire County Council") and notice (`pc-169ba131-3b3f-ed11-8119-005056b64545`) to pick up CPV codes `22000000-0` (Printed matter and related products) and `79800000-2` (Printing and related services) and to understand the DPS framing. Observed existing `notice_lots` row `pc-169ba131-3b3f-ed11-8119-005056b64545-lot-1` as an umbrella lot noting Appendix A is external to that PDF and downstream extraction must split into 10 lots — this extraction supplies that split.
3. Parsed the "The DPS is divided into the following service categories:" block in the Introduction to enumerate the ten lots. Parsed "Part 1 - Specification by Category and Lot" to build per-lot title, description and scope, and Parts 2–10 to identify DPS-wide cross-lot obligations (packaging, service delivery, KPIs, insurance, contract management) which I intentionally did NOT duplicate per lot.
4. For each lot, extracted `product_requirements` seeds from the "Examples include (but not limited to):" bullet blocks. Kept seed_keys stable across `lots[*].product_requirements` and the top-level `product_requirement_seeds` array so downstream persistence can idempotently translate seeds into `product_candidates`.
5. Extracted `technical_standards` only where explicitly cited in the lot paragraph: GBS for paper (Lots 1, 2, 5) and ISO 9001 + ISO/IEC 27001 (Lots 7, 9, 10). Encoded each standard with its own `evidence_anchor` citing the literal sentence in the markdown.
6. Encoded `evaluation_criteria` only where the specification itself imposes a mandatory gate that scopes supplier admission at Lot grain — ISO certifications and on-premises / no-third-party production for Lots 7, 9, 10; proximity/delivery-speed for statutory turnaround jobs in Lots 1 and 7; and the signage travel-expense disclosure rule for Lot 8. Did not fabricate per-lot MEAT weightings, which are not present in Appendix A (those are decided at each Stage-2 further-competition call).
7. Assembled `field_confidences` per schema (lot_split 9600, lot_titles 9600, technical_standards 9200, product_requirements 7500, evaluation_criteria 6500, cpv_codes 6000, quantity_hints 7000, evidence_anchors 9300) and aggregate `extraction_confidence_bps = 8200`.
8. Wrote `result.json` to the workdir and ran a schema validation pass against `skills/pdf-extraction/tender-specification/result.schema.json` (jsonschema draft 2020-12) — result: VALID.

## Decisions Made

- **10 distinct lots, replacing the existing umbrella lot row.** The Introduction explicitly enumerates Lots 1–10 and Part 1 gives each lot its own heading and body paragraph. The current `notice_lots.lot-1` row is a placeholder umbrella that references Appendix A as "NOT contained in this PDF"; this extraction is that split. Warning `EXISTING_UMBRELLA_LOT_ROW` flags the required replace/supersede operation for downstream persistence.
- **CPV codes inherited from process-level signal, then filtered per lot.** Appendix A does not assign CPV per lot. I attached `22000000-0` to lots that produce a physical printed product, `79800000-2` to lots that deliver a service, and both to lots that do both. Warning `CPV_PER_LOT_NOT_IN_DOCUMENT` documents this inheritance rule.
- **Product requirement seeds are representative, not exhaustive.** Every seed is sourced from the document's own "Examples include (but not limited to):" lists. Scope is broader than the examples, so downstream matching must treat seeds as recall starters, not closed sets. Warning `PRODUCT_EXAMPLES_NON_EXHAUSTIVE` records this.
- **`evaluation_criteria` intentionally limited to lot-scoped mandatories.** Per-lot price-quality award weightings are not present; Appendix A only states "selection criteria will include proximity and delivery speed" for statutory-turnaround work (Lot 1, Lot 7), and supplier-admission mandatories (ISO 9001, ISO 27001, on-premises production, BC&DR) for Lots 7, 9, 10. Warning `PER_LOT_AWARD_CRITERIA_NOT_SPECIFIED` records this.
- **Cross-lot DPS-wide obligations stay at process grain.** Parts 2–10 (Packaging, Service Delivery, Complaints, Invoicing, Contract Management, Performance, Innovation, Insurance minimums £5m employers' liability + £5m public liability) were NOT duplicated onto each lot's `requirements` array. Warning `PACKAGING_AND_CROSS_LOT_OBLIGATIONS_NOT_ATTACHED_PER_LOT` records the choice.
- **`page_ref` consistently `unpaginated`.** The source is a `.doc` / Docling markdown with no page markers. Evidence anchors therefore use `section_ref` against the Part/Lot structure (e.g. `"Part 1 — Lot 7 — supplier requirements"`) plus a verbatim `anchor_text` excerpted from the markdown. Warning `DOC_SOURCE_UNPAGINATED` records this.
- **Lot 3 distribution volumes quoted as `quantity_hint` not as a hard number.** The document says "Approx. 361,000 but likely to increase as new houses are built" and "Up to 10,000". Encoded as `quantity_hint` fields on the lot-3 seeds rather than committing to a point value.
- **Cross-lot product routing for Lot 3 preserved.** The document says "all printed elements for Lot 3 will be provided via Lot 2". Therefore Lot 3 seeds are distribution-only services and the corresponding print product seed (`lot2-door-drops`) is attached to Lot 2, with an evidence anchor on the cross-reference sentence to preserve the routing.

## Warnings

Seven warnings were emitted into `result.json#warnings`. Codes: `PER_LOT_AWARD_CRITERIA_NOT_SPECIFIED`, `PRODUCT_EXAMPLES_NON_EXHAUSTIVE`, `NO_PER_LOT_ESTIMATED_VALUE`, `DOC_SOURCE_UNPAGINATED`, `CPV_PER_LOT_NOT_IN_DOCUMENT`, `EXISTING_UMBRELLA_LOT_ROW`, `PACKAGING_AND_CROSS_LOT_OBLIGATIONS_NOT_ATTACHED_PER_LOT`. None are error-severity; all are scope/coverage caveats that a validator or operator should see when this extraction is reviewed or diffed against prior lot rows. In particular, `EXISTING_UMBRELLA_LOT_ROW` requires the daemon-side persistence step to replace/supersede the current umbrella `notice_lots.lot-1` under a single atomic commit cohort for this stage_run.

## Evidence

- Lot split enumerated in the Introduction block: `"**The DPS is divided into the following service categories:**"` then `"Lot 1     Standard Print"` … `"Lot 10   Business Continuity & Disaster Recovery – Response Handling/digital Mail Room"`.
- Each per-lot description anchored on the bold Part-1 heading, e.g. `"**Print and Distribution – Lot 1 - Standard Print**"`, `"Print and Distribution – Lot 2 - High Volume Print"`, `"**Print and Distribution – Lot 7 - Business Continuity & Disaster Recovery – Standard & Security Printing/Confidential Print**"`, `"**Large Format & Promotional Products - Lot 8 - Large format, Signage, Display & POS Printing**"`, `"**Scanning - Lot 9 - Scanning – Questionnaires, photos, plans, maps, etc.**"`, `"**Lot 10   Business Continuity & Disaster Recovery – Response Handling/digital Mail Room**"`.
- Per-lot product-requirement seeds anchored on the bullet items under each lot's "Examples include (but not limited to):" list, e.g. `"A4 / A5 / 99 x 210mm brochures, leaflets or flyers"` (Lot 1), `"C4  / C5 / DL  - pocket, wallet, window, non-window, gummed, opaque"` (Lot 5), `"A2 – A0 size 4 colour inkjets onto satin photo paper 170gsm mounted on 3mm – 5mm Foamex board."` (Lot 8), `"Payroll, P60's, Statements, Pension Payslips & Statements etc."` (Lot 7).
- Technical standards anchored on the literal sentences in the markdown:
  - GBS paper (Lots 1, 2, 5): `"Suppliers shall ensure printed products supplied under the Framework meet the mandatory minimum standards set out by the Government buying Standards"`.
  - ISO 9001 + ISO/IEC 27001 (Lots 7, 9, 10): `"All suppliers for Lot 7 must have ISO 9001 & ISO27001."`, `"All suppliers for Lot 9 must have ISO 9001 & ISO27001."`, `"All suppliers for Lot 10 must have ISO 9001 & ISO27001."`.
- Evaluation-criterion evidence:
  - Lots 1, 7 proximity/speed rule: `"*Where printed materials are subject to statutory timing requiring same-day or 24-hour turnaround, the selection criteria will include the supplier's proximity to the buying authority and speed of delivery service offered."`.
  - Lot 7/9/10 on-premises rule: `"All production must be completed on-premises within the Company and no 3 rd party involvement."`.
  - Lot 8 site-visit rule: `"*For signage and livery; where site visits and on-site measurements are required, this will be requested through quote process and any travel expenses should be quoted within the price estimate."`.
- Lot 2 ⇄ Lot 3 routing evidence: `"**Please note - all printed elements for Lot 3 will be provided via Lot 2 of this Framework.**"`.
- Insurance evidence (DPS-wide, not per-lot): `"Employer's (Compulsory) Liability Insurance = £5 million"` (Part 10).
- Source-markdown hash `f312a8d3f2c0575b50d7b0f6df6143a97059cdbe9e0e2d45632719749c70558c` (from `state.document_extractions.source_content_hash` for `id = 7cc700e6-bb60-4ab6-ab71-e625312bc4d5`) carried into `extraction_metadata.source_markdown_hash`.
- Policy version `2026.04.12` carried through from `state.document_extractions.extraction_policy_version` into `extraction_metadata.policy_version`.
- Raw-document bookkeeping: `raw_document_id = raw-pc-169ba131-3b3f-ed11-8119-005056b64545-5495a5881184f7b5874bdf11e5ea351f784a3550f284313ea9ca67610bb71862`, source-file path `artifacts/downloads/procontract/783238/CCP074-Specification-for-Print-Service-DPS.doc`, content-type `application/msword`, published under ProContract notice URL `https://procontract.due-north.com/RfxResponse/Index?rfxId=a9dc718e-aa2d-f111-813c-005056b64545`.
