---
manifest_version: 1.0.0
skill_id: pdf-extraction/tender-specification
skill_version: 1.0.0
job_id: job-56a9b8cf-5c23-4e93-a301-8452de1f6838
session_id: session-semantic-4e87487520b9c192aec0d65901bccb11f682f6b48cb441df793fd43c858b84e4
execution_mode: contract
started_at: 2026-04-17T18:57:00Z
completed_at: 2026-04-17T19:02:00Z
outcome_class: success_with_caveats
items_processed: 2
items_skipped: 0
items_failed: 0
warnings_count: 7
agent_claimed_pages_visited: 21
agent_claimed_source_url: https://procontract.due-north.com/RfxResponse/Index?rfxId=a12aa4e2-1037-f111-813c-005056b64545
agent_claimed_fingerprint_state: not_applicable_contract_mode
agent_claimed_locale_state: en-GB_document_english
agent_claimed_currency_state: no_monetary_values_present_in_document
---

## Summary

Extracted the lot structure from ECMWF ITT C3S2_413_bis Volume II ("Specification of Requirements", 21 pages, `document_role = technical_specification`). The specification defines two distinct lots sharing the same ITT reference:

- **Lot 1**: Maintenance and extension of the Operational Windstorm Service (Section 4).
- **Lot 2**: Maintenance and extension of operational storm surge products (Section 5).

Section 6 defines cross-Lot joint windstorm-storm surge products; they are captured as a joint-product seed under Lot 2 rather than a third lot, because Section 6 explicitly frames them as a coordination obligation between the two successful tenderers, not an independently-awarded lot. Ten product-requirement seeds were derived (five under Lot 1, five under Lot 2 including the joint-products seed). Seven warnings were raised, all concerning information that is deliberately deferred to Volume IIIA / Volume IIIB (evaluation weightings, pricing, milestone/deliverable tables) or inherited from the parent notice (CPV code `73000000-2`). The notice under processing carries only "Lot 2" in its title, but the specification document authoritatively defines both lots; the title-vs-document mismatch is flagged as a warning.

The extraction ran single-pass in contract mode using the full structural extraction packet produced by Docling (66 sections, 3 tables, 0 OCR, no empty pages). `extraction_scope = full_document`. `extraction_confidence_bps = 8600`.

## Steps Taken

1. Resolved job inputs from the daemon-provided parameters: `document_extraction_id = a81c5c54-2e11-4172-bd23-ae36f53d3d46`, `raw_document_id = raw-pc-978a1774-1037-f111-813c-005056b64545-e7a6ce23556264d3f26de36dbf774fba1cc667345264d21bee4cbacbdc90ee9e`, `process_id = proc-pc-978a1774-1037-f111-813c-005056b64545`, `document_set_id = tds-pc-978a1774-1037-f111-813c-005056b64545`, `extraction_scope = full_document`, `target_domain = lots`.
2. Read the Docling-produced markdown (~46.8 KB, 756 lines) and section index (66 headings) from the `document_extractions` row.
3. Read process context (`buyer = ECMWF`, `title = C3S2_413_bis Lot 2 Maintenance and Extension of C3S Windstorm and Storm Surge Services`) and notice CPV category (`73000000-2 - Research and development services and related consultancy services`) from `procurement_processes` and `contract_notices`.
4. Confirmed no pre-existing `notice_lots` rows for this notice; treated the document as the authoritative source for lot structure.
5. Parsed §1-§3 (Introduction, Context, Contract summary) to confirm two-lot structure with `Tenderers are invited to submit their Tenders for either one or both Lots`, each requiring a separate submission.
6. Parsed §4 (Lot 1) including §4.1 Specific background, §4.2 Scope, §4.3 Technical requirements with sub-sections §4.3.1 (ETC diagnostics maintenance + ERA6 prep), §4.3.2 (service extension: geographic, variables, statistics, indicators, Atlas consistency), §4.3.3 (user engagement), §4.3.4 (online demonstrators), §4.3.5 (dissemination, training, EQC), and §4.4 Deliverables. Derived 9 lot-level requirement bullets plus 9 product requirement entries.
7. Parsed §5 (Lot 2) including §5.1 background, §5.2 Technical requirements (§5.2.1 operational running + §5.2.1.1 capability, §5.2.2 AI methods, §5.2.3 indicator maintenance, §5.2.4 service extension incl. §5.2.4.2 CMIP/Atlas restructure), §5.3 Other activities, §5.4 Deliverables. Derived 8 product requirement entries (including the Section 6 joint-products seed).
8. Parsed §6 (Joint products cross-Lot coordination) and modelled it as `joint-windstorm-surge-products` seed on Lot 2 with cross-reference to Lot 1.
9. Parsed §7 General Requirements (Implementation Schedule, WP0, Deliverables & Milestones guidance, Code delivery aligned with the ECMWF Codex, L2 support, ESOTC contribution, Communication, KPIs, Payment Plan) and §8 Tender Format (page limits, team composition, compliance statement). These cross-cutting items were factored into each lot's `requirements` and `evaluation_criteria` arrays rather than duplicated as a separate lot.
10. Catalogued technical standards referenced in the document (ECMWF Codex, SMP, Codex Containerisation/Helm, ERA5, ERA6, CMIP6/CMIP7, CORDEX, C3S Atlas, NetCDF, FAIR, ECMWF Framework Agreement clauses 2.3/2.8/2.9/2.4.6 with Annexes 5/6, European Commission Decision C(2024)5405 for travel pricing).
11. Assigned CPV `73000000-2` inherited from the notice to both lots and flagged the inheritance as a warning.
12. Computed per-field confidences: lot titles and lot numbers at 9800 bps (verbatim from Section 4/5 headings), lot descriptions at 9200 bps, evaluation criteria at 6500 bps (because weights live in Volume IIIB), product requirements at 8800 bps, extraction metadata at 9900 bps. Aggregate `extraction_confidence_bps = 8600`.
13. Emitted 23 evidence anchors covering one primary anchor per lot, seed, and each major technical standard / evaluation-criterion reference.
14. Wrote `result.json` to the session working directory and validated it against `skills/pdf-extraction/tender-specification/result.schema.json` with `jsonschema` before finalising.

## Decisions Made

- **Two lots, not one.** The notice row only mentions "Lot 2" in its title but the specification document is unambiguous that it covers both lots under one ITT. The skill instructs that the document contradicting pre-existing lot rows should be treated as authoritative, and there are no pre-existing `notice_lots` rows to defer to, so both lots were emitted.
- **No third lot for joint products.** Section 6 opens with "The Successful Tenderers for Lot 1 and Lot 2 shall ensure coordination and technical consistency to enable the development and maintenance of joint windstorm-storm surge products", which frames joint products as a coordination obligation across the two existing lots, not a separately-awarded scope. Joint products were modelled as a product-requirement seed instead of inventing a Lot 3.
- **CPV inheritance from notice.** No CPV codes appear in the document; `73000000-2` was inherited from `contract_notices.main_category` and applied uniformly to both lots. This was flagged as a warning so the daemon and any downstream validator know the value is extrinsic to the document.
- **Evaluation criteria derived from compliance / team / KPI / code-delivery / page-limit language in Volume II.** The document explicitly defers detailed evaluation weightings to Volume IIIB / IIIA. Rather than omit evaluation criteria entirely, the skill emitted the qualitative evaluation dimensions expressed inline (compliance statement per requirement, team composition thresholds, track record, KPI framework, ECMWF Codex consistency, page-limit compliance) and flagged the missing weightings as a warning.
- **Seed keys are stable and semantic.** Seed keys follow `lot<N>-<topic-slug>` for single-lot seeds and an unprefixed `joint-windstorm-surge-products` for the cross-lot seed, so that a resumed/chunked re-run can preserve identity without re-inventing identifiers.
- **No quantity hints emitted.** The tender procures a service rather than discrete goods; there are no unit volumes. Seeds omit `quantity_hint` accordingly.
- **No pricing, no currency, no monetary values.** Volume II explicitly points to Volume IIIA for pricing. `agent_claimed_currency_state` reflects this by declaring "no_monetary_values_present_in_document".

## Warnings

- `EVALUATION_CRITERIA_WEIGHTS_NOT_FOUND` - no weighted evaluation matrix in Volume II; weights are in Volume IIIB.
- `CPV_CODES_NOT_IN_TENDER_DOCUMENT` - CPV `73000000-2` inherited from parent notice.
- `NOTICE_TITLE_LOT_SCOPE_MISMATCH` - notice title cites "Lot 2" only; document covers both lots.
- `JOINT_PRODUCTS_NOT_SEPARATE_LOT` - joint windstorm-surge products modelled as a seed, not a third lot.
- `PRICING_AND_VALUE_NOT_PROVIDED` - no pricing table or budget ceiling in Volume II.
- `NO_QUANTITY_HINTS` - service contract, no discrete unit quantities.
- `DELIVERABLES_DEFERRED_TO_VOLUME_IIIA` - per-work-package deliverables and milestones live in Volume IIIA; only WP0 deliverables and milestones are enumerated in Volume II.

## Evidence

All major claims are anchored to observable structure in the Docling output:

- **Two-lot structure**: §3 Contract summary, p.5, anchor "This Invitation to Tender (ITT) invites Tenderers to submit Tenders for two distinct Lots: Lot 1 ... and Lot 2 ..." (`#/texts/33`).
- **Lot 1 title and scope**: §4, p.5, anchor "4 Lot 1: Maintenance and extension of the Operational Windstorm Service" (`#/texts/37`); §4.1 background (`#/texts/39`); §4.2 scope (`#/texts/54`); §4.3 technical requirements (`#/texts/66`).
- **Lot 1 products maintained**: §4.3.1.1, p.7, anchor "Windstorm tracks and footprints derived from reanalysis over Europe between 1940 to present" and "Two prototype applications ... Jupyter Notebooks" (`#/texts/73`).
- **Lot 1 ERA6 migration**: §4.3.1.5, p.7, anchor "prepare the migration of the CDS catalogue entry and associated workflows to ERA6" (`#/texts/98`).
- **Lot 1 geographic + variable extensions**: §4.3.2.1, §4.3.2.2, p.8, anchors "Extend the current service ... to cover the entire Northern Hemisphere midlatitudes" (`#/texts/107`) and "Add cumulated precipitation associated with windstorm events" (`#/texts/109`).
- **Lot 1 enhanced statistics**: §4.3.2.3, p.8, anchor "Develop and assess methodologies to enhance windstorm statistics for downstream use in risk and catastrophe modelling applications" (`#/texts/113`).
- **Lot 1 indicators**: §4.3.2.4, p.8-9, anchor "develop and assess extra-tropical storm-related indicators suitable for both historical reanalysis datasets (e.g. ERA5, ERA6) and climate projection datasets (CMIP / CORDEX based)" (`#/texts/125`).
- **Lot 2 title and scope**: §5, p.10, anchor "5 Lot 2: Maintenance and extension of operational storm surge products" (`#/texts/160`); §5.1 (`#/texts/162`); §5.2 (`#/texts/165`).
- **Lot 2 operational capability**: §5.2.1.1, p.11, anchor "The reanalysis product provides global time series of sea-level related variables (including storm surge residual and total water level) generated by a state of the art Global tides and surge model with ERA5 forcing" and "Implement an operational update cycle, with updates at least monthly" (`#/texts/174`).
- **Lot 2 AI exploration**: §5.2.2, p.11, anchor "the Successful Tenderer shall explore the potential use of AI-based methods" (`#/texts/188`).
- **Lot 2 indicators**: §5.2.3, p.12, anchor "statistical indicators (e.g., percentiles and return periods, including confidence bounds) for total water levels and surge levels" (`#/texts/202`).
- **Lot 2 projections restructure**: §5.2.4.2, p.12, anchor "restructure the projections work so that storm-surge / sea-level indicators can be produced and assessed from a broad selection of CMIP6 simulations available in the CDS and up to 2100 and CMIP7 simulations when they become available" (`#/texts/212`); "This must be aligned with the C3S Atlas philosophy" (`#/texts/212`).
- **Joint products**: §6, p.13-14, anchor "The Successful Tenderers for Lot 1 and Lot 2 shall ensure coordination and technical consistency to enable the development and maintenance of joint windstorm-storm surge products" (`#/texts/235`).
- **Technical standards**: §7.4, p.17, anchor "all software development is strictly consistent with the ECMWF Codex (see github.com/ecmwf/codex)" (`#/texts/295`); "Containerised Deployment" and "Software Management Plan (SMP)" same section. §9.1 acronym list, p.20-21, anchors for ERA5/6, CMIP6/7, CORDEX, FAIR, EQC.
- **Evaluation criteria (qualitative)**: §7.8, p.18, anchor "KPIs shall focus on measuring the performance of the service" (`#/texts/321`); §8.1, p.19, anchor "Table 3: Page limits" (`#/texts/334`); §8.2.3, p.20, anchor "A senior team member with more than 5 years of experience" (`#/texts/345`); §8.2.4, p.20, anchor "statement of compliance for each requirement formulated throughout this document" (`#/texts/351`).
- **Deferred pricing/deliverables**: §7.9, p.19, anchor "Tenderers can propose a Payment Plan in ITT Volume IIIA 'Pricing and deliverables'"; Tables 1 and 2 WP0, p.15-16 (`#/tables/1`, `#/tables/2`).
- **Source document identity**: `source_content_hash = 82bb3bb1fb3b6bf1764edd20fd743d09980ffbcfa2367c05cc8522dace1c21fe`, `extraction_engine = docling 2.88.0`, `page_count = 21`, `section_count = 66`, `table_count = 3`.
