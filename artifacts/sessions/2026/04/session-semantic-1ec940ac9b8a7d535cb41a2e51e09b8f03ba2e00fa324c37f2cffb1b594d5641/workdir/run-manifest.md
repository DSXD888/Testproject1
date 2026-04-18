---
manifest_version: 1
skill_id: pdf-extraction/tender-specification
skill_version: 1.0.0
job_id: job-d6bedbf6-6051-4ff4-8a45-dd6968c06828
session_id: session-semantic-1ec940ac9b8a7d535cb41a2e51e09b8f03ba2e00fa324c37f2cffb1b594d5641
execution_mode: contract
started_at: 2026-04-17T21:47:00Z
completed_at: 2026-04-17T21:47:00Z
outcome_class: success
items_processed: 1
items_skipped: 0
items_failed: 0
warnings_count: 4
agent_claimed_pages_visited: 49
agent_claimed_source_url: null
agent_claimed_fingerprint_state: not_applicable
agent_claimed_locale_state: en-GB
agent_claimed_currency_state: GBP
---

# Run Manifest — pdf-extraction/tender-specification

## Summary

Extracted a single authoritative lot record plus 23 product-requirement seeds from the ESPO Framework 107_26 Invitation to Tender (`document_extraction_id=9bcb0725-605b-444b-a388-124a1e2c506a`, 49 pages, Docling 2.88.0 standard strategy). The document is a national, single-framework ITT for "Inspection & Maintenance of Indoor Gymnasium, Fitness & Sport Equipment and Outdoor Schools Play & Sport Equipment" and contains no intrinsic lot split (all Tenderers must supply all nine English regions; regional coverage of Scotland/Wales/NI is optional and does not create separate lots). Pricing detail for the 57 service requirements lives in Appendix B (a separate pricing-schedule document) and is intentionally not duplicated in this payload. CPV codes are carried from notice-level `process_context` because the tender markdown itself does not cite CPV numerics verbatim. Extraction confidence is reported as 8500 bps; four warnings document the single-lot assumption, the notice-sourced CPV provenance, the pricing-schedule deferral, and the optional devolved-nation scope.

## Steps Taken

1. Loaded `document_extractions` row `9bcb0725-605b-444b-a388-124a1e2c506a` from `state/state.db`; captured `source_content_hash=5f70928da86c6d562af8c3ddd3b32f6ab8f684ceae8e208b8449aff173456774`, `extraction_engine=docling 2.88.0`, `extraction_policy_version=2026.04.12`, `document_role=invitation_to_tender`, `page_count=49`, `section_count=114`, `table_count=14`, markdown length 148,527 bytes.
2. Read the Docling markdown end-to-end and walked the section index to map headings to page/section refs (e.g. `#/texts/175 "2. REQUIREMENT (including Specification)" p13`, `#/texts/206 "1. Counter Balanced Beans (Fixed and Hinged)" p15`, `#/texts/538 "Non-price 60%" p30`, `#/texts/569 "Price 40%" p33`, `#/texts/518 "Conditions of Participation" p26`).
3. Verified there is no per-lot narrative block in the document: §1 Scope (p6) states "Tenderers must be able to supply all regions"; the Framework is a single award to "a minimum of 4 Suppliers", and the Pricing Schedule references 57 priced service requirements in a single framework, not separate lots. Decided to emit a single lot (`lot_number="1"`) and raise `SINGLE_LOT_ASSUMED`.
4. Captured CPV signals (`37400000` Sports goods and equipment; `50000000` Repair and maintenance services) from `contract_notices.main_category` for the process; noted in `warnings` that these are notice-sourced rather than tender-body-sourced.
5. Extracted 21 equipment-category specifications from §2 Requirement (sections 1–21 of the spec list) plus two additional outdoor scopes called out at the top of §2 (`outdoor_schools_play_equipment`, `outdoor_sports_equipment`), for a total of 23 product_requirement_seeds. Each seed is keyed with a stable snake_case `seed_key`, anchored to its own `#/texts/...` section_ref and page_ref, and mirrored into `lots[0].product_requirements` to satisfy the schema's required `product_requirements` array on the lot.
6. Captured technical standards explicitly cited in §2 Requirement (HSW Act 1974, PUWER 1998, BS EN 1176, BS EN 1177, BS 1982 Parts I/II/IV-1986/2003, BS EN 916:2003) plus two contextual standards: afPE guidance (§14 Landing Mats) and the Electricity at Work Act exclusion (§6 Basketball Goals electric winch). Each is evidence-anchored.
7. Built the evaluation_criteria block from §5 Evaluation (p25–p33), preserving the official weightings (Price 40%, Non-price 60% = Q1×3 + Q2×3 + Q3×5 + Q4×4 + Q5×4 + Q6×1 for a weighted total of 60) and the Conditions-of-Participation pass/fail gate (Financial Capacity, Technical & Professional Ability, Insurance levels £5m/£10m/£10m/£2m, Health & Safety, Quality Management, Carbon Reduction Plans).
8. Wrote `result.json` into the session workdir, populated every required field of `result.schema.json` v1.0.0 (`target_domain=lots`, `extraction_scope=full_document`, `source_markdown_hash` matching the DB-stored hash, `policy_version=2026.04.12`), and validated the file against the schema locally with `jsonschema` (result: SCHEMA VALID).

## Decisions Made

- **Single lot.** The document is a single-framework ITT; there is no categorical or regional lot partition. Emitting multiple synthetic lots (e.g. one per region or one per equipment category) would misrepresent the procurement structure. Choice is recorded in `SINGLE_LOT_ASSUMED` warning so downstream consumers know the grain.
- **Equipment categories as product_requirement_seeds, not lots.** The 21 equipment categories in §2 are what will match against supplier products/services in the matching pipeline; they are the correct grain for candidate seeds. They are NOT independent lots — they are priced line items within a single framework basket.
- **CPV from notice metadata with provenance warning.** The tender body describes scope qualitatively; the CPV numerics come from the notice. Emitting them is correct for downstream matching, but provenance is flagged in a warning (`CPV_CODES_FROM_NOTICE_METADATA`).
- **Pricing-schedule content deferred.** The 57 service-requirement rows and unit rate bands live in Appendix B and will be extracted by `pdf-extraction/pricing-schedule`. This manifest intentionally does not enumerate them — doing so would duplicate and risk divergence. Warning `PRICING_DETAIL_DEFERRED_TO_APPENDIX_B` records the boundary.
- **Optional devolved nations not modelled as separate coverage lots.** Scotland/Wales/NI are declared optional extensions by a Tenderer, not distinct procurement lots. Warning `OPTIONAL_DEVOLVED_COVERAGE_NOTED`.
- **Stable seed_keys.** Used snake_case equipment-category names (`counter_balanced_beams`, `basketball_goals`, `trampolines`, etc.) so any resume/retry produces identical keys per SKILL.md resume strategy.

## Warnings

Four warnings emitted in `result.json#/warnings`:

1. `SINGLE_LOT_ASSUMED` — document describes one national framework; `lots[]` contains a single entry with `lot_number="1"`.
2. `CPV_CODES_FROM_NOTICE_METADATA` — CPV codes carried from `process_context` / `contract_notices.main_category`, not cited verbatim in the tender markdown.
3. `PRICING_DETAIL_DEFERRED_TO_APPENDIX_B` — 57 priced service requirements and unit rate bands live in Appendix B and are handled by the `pdf-extraction/pricing-schedule` overlay.
4. `OPTIONAL_DEVOLVED_COVERAGE_NOTED` — optional Scotland/Wales/NI coverage does not create distinct lots.

## Evidence

All major claims in this manifest are anchored to observable structural fields in the Docling payload. The result payload's `evidence_anchors` array carries 16 top-level anchors; every lot requirement, every evaluation criterion, every technical standard, and every product_requirement_seed carries its own `{page_ref, section_ref, anchor_text}` evidence anchor per the schema.

Key anchors grounding this manifest (Docling `section_ref` → page):

- `#/texts/1` → p1 "ESPO reference 107_26 Inspection & Maintenance of Indoor Gymnasium, Fitness & Sport Equipment and Outdoor Schools Play & Sport Equipment" (document identity).
- `#/texts/37` → p5 "1. INTRODUCTION" (overview: open procedure, Procurement Act 2023 §§19–24, min 4 suppliers).
- `#/texts/64` → p6 "Scope" (nine English regions mandatory; devolved nations optional).
- `#/texts/86` → p7 "Estimated Value" (£10m inclusive of VAT over maximum term).
- `#/texts/89` → p7 "Duration" (initial ~19 months from 24 June 2026 plus option to extend up to 23 June 2030, max 4 years).
- `#/texts/175` → p13 "2. REQUIREMENT (including Specification)" (scope summary, regulations HSW Act 1974, PUWER 1998, BS EN 1176, BS EN 1177; BS 1982 Parts I/II/IV-1986/2003; BS EN 916:2003).
- `#/texts/200` → p14 "Tenderers must provide pricing for a minimum of 40 out of the 57 service requirements" (pricing-schedule compliance threshold).
- `#/texts/206` – `#/texts/443` → p15–p20 equipment-category specifications 1–21 (one `section_ref` per category; each mapped to a product_requirement_seed with matching evidence_anchor).
- `#/texts/460` → p22 "Variant Bids" ("Variant bids will not be accepted.").
- `#/texts/482` → p24 "4. PROCUREMENT PROCESS" (open procedure; timetable: Tender Notice from 13 Apr 2026; clarifications close 8 May 2026; tender submission 15 May 2026 12:00; award 11 Jun 2026; start 24 Jun 2026).
- `#/texts/495` → p25 "5. EVALUATION OF TENDERS" (two-part evaluation; Part 1 exclusion grounds + conditions of participation; Part 2 award criteria).
- `#/texts/518` → p26 "Conditions of Participation" (Financial Capacity, Technical & Professional Ability, Insurance £5m/£10m/£10m/£2m, Health and Safety, Quality Management, Carbon Reduction Plans — all pass/fail).
- `#/texts/538` → p30 "Non-price 60%" (Q1–Q6 weightings: 3/3/5/4/4/1; max weighted 60).
- `#/texts/569` → p33 "Price 40%" (median-price basket scoring methodology; 3.5% framework fee included; 12-month price-hold).

Result artifact: `result.json` (written to session workdir, 23 product_requirement_seeds, 1 lot, 16 top-level evidence anchors, extraction_confidence_bps=8500). Validated locally against `skills/pdf-extraction/tender-specification/result.schema.json` v1.0.0 → SCHEMA VALID.
