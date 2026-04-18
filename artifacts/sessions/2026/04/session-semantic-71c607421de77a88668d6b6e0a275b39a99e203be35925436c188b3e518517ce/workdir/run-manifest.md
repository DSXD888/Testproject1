---
manifest_version: 1
skill_id: pdf-extraction/tender-specification
skill_version: 1.0.0
job_id: job-661bc426-2d47-44b8-a282-00149600e903
session_id: session-semantic-71c607421de77a88668d6b6e0a275b39a99e203be35925436c188b3e518517ce
execution_mode: contract
started_at: 2026-04-18T11:42:00Z
completed_at: 2026-04-18T11:45:00Z
outcome_class: success
items_processed: 25
items_skipped: 0
items_failed: 0
warnings_count: 5
agent_claimed_pages_visited: 49
agent_claimed_source_url: "local://document_extractions/af28a826-6e4e-4555-9024-ccd6ed3c0250"
agent_claimed_fingerprint_state: n/a
agent_claimed_locale_state: en-GB
agent_claimed_currency_state: GBP
---

## Summary

Extracted lot-level semantic structure from the ESPO Invitation to Tender for Framework 107_26 (Inspection & Maintenance of Indoor Gymnasium, Fitness & Sport Equipment and Outdoor Schools Play & Sport Equipment). The document is a single-lot, national-coverage Framework (Open Procedure under Procurement Act 2023) with a pre-existing `notice_lots` row already classifying it as a single lot. One authoritative lot record is emitted, together with 24 product requirement seeds covering the 21 equipment categories enumerated in §2 (Counter Balanced Beams through Cardiovascular and Strength Equipment) plus three cross-cutting seeds for outdoor schools playground equipment, associated remedial services, and emergency call-outs. Evaluation criteria (6 non-price questions with their weightings, and Price 40% basket-median methodology) and technical standards (BS EN 1176, BS EN 1177, BS 1892/1982, BS EN 916:2003, HSAWA 1974, PUWER 1998, plus the explicitly-excluded Electricity at Work Regulations 1989 on electric winch testing) are captured with evidence anchors. CPV codes (37400000-2 and 50000000-5) are sourced from the notice-level `main_category` field because the ITT body itself does not enumerate CPV codes. Source markdown hash set to the DB-stored `source_content_hash` (5f70928d...).

## Steps Taken

1. Read the bound skill `pdf-extraction/tender-specification` v1.0.0 and its result schema.
2. Located the Docling-extracted markdown in `document_extractions.id = af28a826-6e4e-4555-9024-ccd6ed3c0250` (148,527 chars, 49 pages, 14 tables, 114 sections, role=invitation_to_tender).
3. Gathered process/notice context from `procurement_processes` and `contract_notices` tables (ESPO, 107_26, deadline 2026-05-15T12:00:00Z, notice-level CPV `37400000-2 | 50000000-5`).
4. Confirmed existing notice-level lot metadata: single lot already classified in `notice_lots` (pc-…-lot-1) - did NOT contradict it since the ITT text is consistent with a single-lot national framework.
5. Read §§1-5 of the ITT in detail: cover page, Introduction (scope, regions, estimated value, duration, framework fee, social value), §2 Requirement/Specification (indicative services list, standards paragraph, 21 numbered equipment categories, remedial/emergency call-outs), §3 Preliminary Market Engagement, §4 Procurement Process (timetable), §5 Evaluation of Tenders (Part 1 conditions, Part 2 award criteria, price scoring methodology).
6. Synthesised the lot record with 21 requirements (general operational and contractual obligations), 7 evaluation-criteria entries (Q1-Q6 non-price plus Price 40%), 11 technical-standards entries with verbatim anchor text and cross-reference context, and a primary evidence anchor at the cover page.
7. Synthesised 24 `product_requirement_seeds` - one per equipment category (21) plus outdoor schools playground equipment, remedial services, and emergency call-outs - each with seed_key, candidate_name, summary, concrete requirements (imperative verbs), relevant standards, CPV codes, and evidence anchor referencing the exact heading.
8. Captured 5 warnings: BS 1892/1982 standard-reference ambiguity, notice-only CPV sourcing, pricing schedule not enumerated in this overlay, single-lot national coverage note, and inline exclusions callout.
9. Wrote `result.json` to the workdir matching `result.schema.json` exactly (verified JSON parsability post-write). Wrote this run manifest.

## Decisions Made

- **Single lot (not 9 regional lots):** The Framework is structured around nine English regions but tenderers "must be able to supply all regions as specified above" and submit national pricing. Per the evidence (§1 Scope and §3 Preliminary Market Engagement bullet "Suppliers to provide national pricing"), regional splitting is a national-coverage constraint, not a lot structure. Kept lot count = 1, matching the pre-existing `notice_lots` row.
- **CPV source = notice-level, with warning:** The ITT body does not enumerate CPV codes. Used the notice-level `main_category` value and surfaced `CPV_LIST_FROM_NOTICE_ONLY` warning so the daemon/validator can trace provenance.
- **BS 1892 vs BS 1982:** Preserved the document's verbatim rendering "BS 1982 Parts I, II and IV-1986/2003" in the evidence anchor but noted in context and warnings that the authoritative series for gymnasium equipment is BS 1892. Did not silently correct.
- **Pricing Schedule out of scope:** The ITT references 57 service requirements in Appendix B but does not enumerate individual SKU-style line items in the body text. Did not attempt to reconstruct them - that belongs to the pricing-schedule overlay. Warning emitted.
- **Electricity at Work Act handled as exclusion:** The basketball-goal electric-winch inspection explicitly excludes Electricity at Work Act testing. Captured the Act in `technical_standards` with context "by exclusion" rather than as a positive compliance obligation, and inlined the exclusion on the `basketball-goals` seed.
- **Outdoor schools playground equipment is a distinct seed:** Although the numbered equipment list (items 1-21) is indoor-focused, the indicative-list paragraph §2 explicitly lists "Outdoor schools fixed playground equipment, low-level and low risk inspection of trim trail equipment, activity play equipment and surfacing equipment", and BS EN 1176/1177 apply there. Added seed `outdoor-schools-playground-equipment` to keep that scope discoverable.
- **Remedial services and emergency call-outs are ancillary_service seeds:** Both are scoped explicitly in §2 and carry non-obvious commercial constraints (separate pricing and approval, scaffolding itemisation) that product-matching downstream needs to see; included as seeds with `requirement_type: ancillary_service`.
- **extraction_confidence_bps = 8800:** High confidence on lot identity, evaluation criteria and standards; moderated by CPV provenance warning and BS 1892/1982 ambiguity.

## Warnings

1. `STANDARD_REFERENCE_AMBIGUOUS` - "BS 1982 Parts I, II and IV-1986/2003" is likely a Docling / OCR artefact of "BS 1892". Flagged for daemon-side authoritative lookup.
2. `CPV_LIST_FROM_NOTICE_ONLY` - CPV codes derived from notice-level `main_category`, not from the ITT body.
3. `PRICING_SCHEDULE_NOT_ENUMERATED_HERE` - 57 service-requirement line items live in Appendix B pricing schedule; tender-specification overlay intentionally does not reconstruct them.
4. `SINGLE_LOT_NATIONAL_COVERAGE` - Nine English regions are coverage constraints, not lot partitions; UK-extra-territory coverage is an optional Tenderer indication.
5. `EXCLUSIONS_NOTED` - Multiple maintenance scopes carry explicit exclusions (Electricity at Work Act testing on electric winches; plastic re-coating of hook covers; agility-mattress cleaning/re-covering; complete wall-bar re-varnishing as separate order). Captured inline on the relevant seeds.

## Evidence

- Cover page / Title: "ESPO reference 107_26 Inspection & Maintenance of Indoor Gymnasium, Fitness & Sport Equipment and Outdoor Schools Play & Sport Equipment"; "Framework Period 24 June 2026 - 31 January 2028 with the option to extend up to 23 June 2030"; "Closing date for submission of tender 12 noon, 15 May 2026".
- §1 INTRODUCTION: "This is an Open Procedure conducted in accordance with sections 19 to 24 of the Procurement Act 2023."; "It is anticipated that a minimum of 4 Suppliers shall be awarded a place on the Framework"; "Tenderers must be able to supply all regions as specified above."; "The estimated value of the Framework ... is £10m (inclusive of VAT) over the maximum possible term"; "This Framework Fee will be at a rate of 3.5% and shall be payable every three months".
- §2 REQUIREMENT (including Specification): opening line "The requirement is for the inspection & maintenance of indoor gymnasium, fitness & sport equipment and outdoor schools, play & sport equipment, including associated remedial services (where required)"; standards paragraph "The Health and Safety at Work Act 1974, Provision and Use of Work Equipment Regulations 1998 (PUWER), BS EN 1176 and BS EN 1177" plus "BS 1982 Parts I, II and IV-1986/2003 and BSEN 916:2003"; pricing callout "Tenderers must provide pricing for a minimum of 40 out of the 57 service requirements (70%)"; 21 numbered headings (## 1. Counter Balanced Beans ... ## 21. Cardiovascular and Strength Equipment); explicit exclusion "N.B: This inspection excludes testing to comply with the current edition of the Electricity at Work Act" (item 6, Electric Winch); afPE guidance on landing-mat covers (item 14); "The cleaning and re-covering of agility mattresses is not a part of this Framework" (item 14).
- §3 PRELIMINARY MARKET ENGAGEMENT: "Suppliers to provide national pricing for services, rather than regional pricing."; "Evaluation award criteria weighting to be amended to 40% price and 60% non-price (including 5% social value)."
- §4 PROCUREMENT PROCESS - Indicative Timetable table: clarification-deadline 08 May 2026; tender deadline 15 May 2026; award decision 11 June 2026; Framework start 24 June 2026.
- §5 EVALUATION OF TENDERS - Part 2 Award Criteria weightings table: Q1 Customer Engagement and Framework Management (weight 3); Q2 Customer Site Visit Process (3); Q3 Inspection Report (5); Q4 Services Following Completion of Work (4); Q5 Consistency of Standards (4); Q6 Social Value (1); total 60. Price 40% scored via basket-median methodology (20 points at median, pro-rata above/below, floor 0).
- §5 Conditions of Participation: Part 3 Q6 Insurance (£5m EL, £10m PL, £10m Product, £2m PI); Part 3 Q12 Carbon Reduction Plans; Part 3 Q4 Technical & Professional Ability (3-year relevant experience).
- Daemon appendix references: `document_extractions.id = af28a826-6e4e-4555-9024-ccd6ed3c0250`; `source_content_hash = 5f70928da86c6d562af8c3ddd3b32f6ab8f684ceae8e208b8449aff173456774`; `expected_overlays_json = ["pdf-extraction/tender-specification"]`; pre-existing `notice_lots.id = pc-3ba4e6b8-9cef-f011-813c-005056b64545-lot-1` (single lot, is_bidable_separately = 1); notice-level CPV via `contract_notices.main_category` = "37400000-2 - Sports goods and equipment | 50000000-5 - Repair and maintenance services".
