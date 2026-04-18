---
manifest_version: 1
skill_id: pdf-extraction/tender-specification
skill_version: 1.0.0
job_id: job-83e9a06b-cc90-4a86-a387-e78f164d5e53
session_id: session-semantic-b023fca897d59028b31e6d7f32bfa1830802664a559afadc2f4a7ae3e6248735
execution_mode: contract
started_at: 2026-04-17T23:30:00Z
completed_at: 2026-04-17T23:33:17Z
outcome_class: success
items_processed: 1
items_skipped: 0
items_failed: 0
warnings_count: 6
agent_claimed_pages_visited: 0
agent_claimed_source_url: "sqlite:state.db/document_extractions/94c38619-08b5-48a0-9402-b8fab56521c8"
agent_claimed_fingerprint_state: not_applicable
agent_claimed_locale_state: en-GB
agent_claimed_currency_state: not_applicable
---

# Run Manifest - pdf-extraction/tender-specification

## Summary

Extracted a single-lot tender-specification payload for process `proc-pc-19574d11-0929-f111-813c-005056b64545` (Cambridgeshire County Council - "25112 Library Services Engage, Design and Fit out Contract - Ely Library") from document extraction `94c38619-08b5-48a0-9402-b8fab56521c8` (document role `technical_specification`, 10 pages, 24 sections, 1 table, ~23 KB of Docling markdown). The document is Appendix 4 - Specification, a single coherent brief for a design-and-build library refresh covering design engagement, furniture, signage, flooring/carpet across a ~850sqm first-floor footprint. No additional lot boundaries were found inside the document, so the output describes one authoritative lot (lot_number "1") matching the pre-existing notice lot inventory. Produced 11 product requirement seeds (flooring/carpet, children's zone, study desks, adult shelving, IT/printing, bespoke staff desk, self-service support, events space, social seating, first-floor lobby, signage/graphics), 5 evaluation-criteria entries (sustainability, case studies, design principles, accessibility/inclusion, warranties/defects), 3 technical standards (Modern Slavery Act, Equality Act, UK GDPR) and 31 evidence anchors. Overall extraction confidence 7500 bps.

## Steps Taken

1. Read the bound SKILL.md (`pdf-extraction/tender-specification@1.0.0`) and its companion `result.schema.json` and `golden.assertions.yaml` to confirm required fields, the single permissible `target_domain = "lots"`, and structural constraints (evidence anchors on standards, non-empty lots, warnings as array).
2. Loaded job parameters: `document_extraction_id=94c38619-08b5-48a0-9402-b8fab56521c8`, `target_domain=lots`, `extraction_scope=full_document`, `process_id=proc-pc-19574d11-0929-f111-813c-005056b64545`, `document_set_id=tds-pc-19574d11-0929-f111-813c-005056b64545`, `stage_run_id=stage-run-57a5f226-b1b8-4236-9510-0821c0ebd20d-proc-pc-19574d11-0929-f111-813c-005056b64545`.
3. Read `state.db` `document_extractions` row to obtain the Docling markdown (`extracted_markdown` column), document title ("Appendix-4-Specification"), role (`technical_specification`), page count (10), section count (24) and table count (1). Computed SHA-256 of the retrieved markdown (`a78c3e5de185651e9203bfcc130aff51a099cf2948f0c5eb7a38cc1492d63a08`) for `extraction_metadata.source_markdown_hash`.
4. Read `procurement_processes` for process context (buyer `Cambridgeshire County Council`, title `25112 Library Services Engage, Design and Fit out Contract - Ely Library`) and `notice_lots` for the cross-reference-only pre-existing lot row (lot_number 1, "Ely Library Services - Engage, Design and Fit Out Contract"). Confirmed the document describes the same single scope and did not need to override the existing lot split.
5. Parsed the Docling markdown by section:
   - §1 Scope, §2 Out of Scope - framed lot scope and out-of-scope exclusions.
   - §3 Service Requirements - library footprint (~850sqm, ~150k visits/yr), design engagement, sustainability expectations.
   - §4 Outcome - accessibility/inclusion, dementia/autism-friendliness, signage as integral design, carpet replacement.
   - §5 Distinct Zones - decomposed into 11 product-requirement seeds: children's library (§5.1), study desks (§5.2), adult shelving (§5.3), IT & printing (§5.4), bespoke staff desk (§5.5), self-service (§5.6), events space (§5.7), social seating (§5.8), flooring (§5.9), first-floor lobby (§5.10), thoroughfares/sightlines (§5.11) rolled into lot-level requirements.
   - §6-§8 Contractual info, H&S, mobilisation - captured as lot requirements.
   - §9 Timetable - captured as lot requirement (milestones Jul 2026 - Jan 2027 + 12-month defects period).
   - §10 Quality & Performance - extracted evaluation-criteria entries (sustainability scored submission, case studies scored, warranty details, defects).
   - §11 Subcontracting, §12 Handover - captured as lot requirements.
   - §13-§15 Modern Slavery, Equality Act, UK GDPR - captured as technical standards.
   - §16 Payment schedule - captured as lot requirement (ITT section 4.1 cross-reference, milestone linkage, no pre-payment).
6. For each seed and standard, anchored an `evidence_anchor` with `page_ref`, `anchor_text` (verbatim substring from the markdown) and `section_ref`. Built a top-level `evidence_anchors` array spanning the document's major section boundaries (31 anchors, at least one per major numbered section).
7. Recorded `field_confidences` with lower values where the spec document itself is thin (CPV codes: 2000; evaluation weightings: 5500; technical standards: 4500) and higher values where the spec is rich and unambiguous (lot count: 9500; requirements: 8500).
8. Emitted warnings (`warnings`): missing CPV, missing evaluation weightings, no explicit BS/EN/ISO standards, a Docling layout-continuation artefact on page 2, the approximate floor area (§3.3), and provisional furniture counts ("8-10 people", power-point totals agreed at final design).
9. Set `extraction_metadata.target_domain = "lots"`, `extraction_scope = "full_document"` (from job params), `policy_version = "2026.04.12"` (from SKILL.md example header), and `document_role = "technical_specification"` (from `document_extractions.document_role`).
10. Wrote `result.json` to the session workdir and validated it against `result.schema.json` using `jsonschema` (validation passed).

## Decisions Made

- **Single-lot output.** The specification describes one coherent scope (design-and-build refresh of Ely Library) with internal zone decomposition; no multi-lot boundaries are asserted in the text. Emitting one lot aligns with the schema expectation (`min_lot_count: 1`) and the pre-existing notice lot, but the decision rests on the document's own language ("single-lot", §1 scope framing) rather than the notice-lot row, per the skill's "cross-reference only" guidance.
- **Zones as seeds, not sub-lots.** The distinct zones in §5 are design regions within one deliverable, not separately-bidable lots. They are emitted as `product_requirement_seeds` so the daemon can translate them into candidate rows without losing granularity.
- **Evaluation criteria captured in plain text with an explicit "weighting_text" disclaimer.** Weightings are not present in this document; the companion ITT presumably carries them. Including the criteria anyway preserves the signal for downstream qualification while the accompanying warning flags the gap.
- **Technical standards limited to statutory frameworks actually cited.** The document cites Modern Slavery Act, Equality Act and UK GDPR by name; no BS/EN/ISO product standards are called out, so nothing was invented. Plain-English performance expectations (dementia-/autism-friendly, sustainability, warranty, defects) were kept as lot requirements rather than being upgraded into fake standards.
- **CPV codes emitted as empty array.** The specification body carries none. Populating CPVs from the process record would be unsafe; the skill's job is to extract from this document.
- **Layout artefact preserved, not "fixed".** The Docling output shows §2 "Out of Scope" continuing across the "Appendix 4 - Specification" running header. This is a layout running-header interaction and is logged as a warning rather than silently renumbered.
- **Quantity hints preserved in suggested/agreed language.** Counts such as "soft seating for 8-10" and "number of power points... agreed during final design" are retained as provisional to reflect the document's stated intent rather than flattened into single values.

## Warnings

- `CPV_CODES_NOT_PRESENT_IN_DOCUMENT` - no CPV codes are in the specification body; downstream consumers must source them from the notice/process record if required.
- `EVALUATION_WEIGHTINGS_NOT_PRESENT` - sustainability (question 2), case studies, and warranty bid elements are referenced but no scoring weightings or rubric are included; these are assumed to live in the companion ITT.
- `NO_EXPLICIT_TECHNICAL_STANDARDS_CITED` - only statutory frameworks (Modern Slavery, Equality, UK GDPR) are referenced; no product/industry standards such as BS/EN/ISO/BREEAM are named.
- `LAYOUT_CONTINUATION_ACROSS_APPENDIX_HEADING` - Docling markdown shows §2 "Out of Scope" text continuing after the "Appendix 4 - Specification" header at line 24, indicating a running-header layout interaction.
- `FLOOR_AREA_APPROXIMATE` - the ~850sqm footprint is expressed as "just under 850sqm" (§3.3); treat as approximate for costing.
- `FURNITURE_COUNTS_PROVISIONAL` - several quantities (social seating 8-10, power-point count at bespoke desk, number of study desks on power beyond the 10 minimum) are stated as suggested or "to be agreed during final design".

## Evidence

Every claim in the output is anchored to a section heading, list item or table row that appears verbatim in the Docling markdown for extraction `94c38619-08b5-48a0-9402-b8fab56521c8` (title "Appendix-4-Specification", role `technical_specification`, 10 pages). Key anchor references used:

- Lot framing: `§1.1` "For the delivery of design, furniture, signage, and carpet to refresh the public library service in Ely library to modern standards. This is a design and build project." and `§1.2` "This procurement is looking for a bid submission which provides the bidder's design principles, methodologies, insights, thoughts, inspiration...".
- Out-of-scope bounding: `§2.1` "The façade, graphics, flooring and decorations in the main entrance and stairway into the library (recently completed in 2022)" and `§2.2` on power/data and specialist IT.
- Library context: `§3.3` "The core library footprint is just under 850sqm, it receives over 150,000 visits a year...".
- Outcome / accessibility / carpet: `§4.4` dementia-/autism-friendliness, `§4.5` signage as integral, `§4.6` carpet replacement across main footprint.
- Product-seed anchors: `§5.1.8` junior/non-fiction/kinder-box shelving counts; `§5.2.2-§5.2.3` study-desk 12-to-18 uplift with ≥10 on power; `§5.3` adult shelving 210/120/18/20; `§5.4.2-§5.4.5` six PCs, wheelchair-accessible desk, MFD; `§5.5.2-§5.5.8` bespoke staff desk power/data/storage; `§5.6.2` "three Lynsgoe Phoenix devices each supported by a double-sided trolley and two dump bins between the three"; `§5.7.1` "Veterans group approx. 25"; `§5.8.2` "soft seating for 8-10 people including table for drinks"; `§5.9.3-§5.9.4` "flooring MUST be easy to maintain" with darker-colour preference; `§5.10.2-§5.10.3` bar-stool desk, information display.
- Timetable: `§9` table rows (Site visit 6th July 2026; Fit out works 2nd November 2026 - 20th December 2026; Training/handover 4th-8th January 2027; Defect period 12 months).
- Evaluation and scored submission: `§10.1` "This will be part of the scored submission" (sustainability), `§10.3` "especially when answering question 2 on sustainability" (warranties), `§10.5` "case studies... form part of the scored submission", `§10.4` 12-month defects period.
- Standards: `§13.1` Modern Slavery Act, `§14.1` Equality Act/EDI, `§15.1-§15.2` UK GDPR with 72-hour breach reporting.
- Commercial: `§16.1` ITT section 4.1 purchase-order/invoicing cross-reference, `§16.4` "no pre-payment or a form of a deposit made by the Council".

Source markdown SHA-256 of the content returned from `state.db::document_extractions.extracted_markdown`: `a78c3e5de185651e9203bfcc130aff51a099cf2948f0c5eb7a38cc1492d63a08` (also recorded in `result.json::extraction_metadata.source_markdown_hash`).
