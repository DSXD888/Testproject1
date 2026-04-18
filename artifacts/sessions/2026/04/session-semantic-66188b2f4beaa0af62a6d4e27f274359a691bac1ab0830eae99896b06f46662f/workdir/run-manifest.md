---
manifest_version: 1
skill_id: pdf-extraction/tender-specification
skill_version: 1.0.0
job_id: job-69a68fb3-88c2-4bb3-ad31-800ee46ade62
session_id: session-semantic-66188b2f4beaa0af62a6d4e27f274359a691bac1ab0830eae99896b06f46662f
execution_mode: contract
started_at: 2026-04-18T00:00:00Z
completed_at: 2026-04-18T00:00:00Z
outcome_class: success
items_processed: 2
items_skipped: 0
items_failed: 0
warnings_count: 6
agent_claimed_pages_visited: 0
agent_claimed_source_url: https://cds.climate.copernicus.eu/
agent_claimed_fingerprint_state: not_applicable
agent_claimed_locale_state: en-GB
agent_claimed_currency_state: EUR
---

## Summary

Extracted lot structure and product-requirement seeds from Volume II "Specification of Requirements" of ECMWF Invitation to Tender `C3S2_413_bis` ("Maintenance and Extension of C3S Windstorm and Storm Surge Services"), using the Docling-produced markdown already persisted in `document_extractions.id=b0d01939-f2c1-428e-a7a5-881224f1f337` (`source_content_hash=82bb3bb1…1c21fe`, policy version `2026.04.12`). The document defines two authoritative lots — Lot 1 Operational Windstorm Service and Lot 2 Operational Storm Surge Products — each resolvable to a separate 28-month Framework Agreement and requiring independent submissions. A sixteen-entry product-requirement seed set was emitted alongside §8 evaluation dimensions (Executive Summary, Track Record, Quality of Resources, Technical Solution, Management & Implementation, plus the SMART KPI set from §7.8). No CPV codes are present in the specification text; CPV `73000000-2` is carried over from the contract-notice `main_category` and is flagged as such.

## Steps Taken

1. Resolved the Docling extraction for the target `document_extraction_id` in `state.db` and dumped the `extracted_markdown` (756 lines) to inspect the authoritative structural payload; confirmed role `technical_specification` and policy version `2026.04.12`.
2. Read the full specification markdown — §1 Introduction, §2 Context (incl. §2.1 CDS, §2.2 Earthkit), §3 Contract summary, §4 Lot 1, §5 Lot 2, §6 Joint products, §7 General Requirements (WP0, deliverables, code delivery, L2 support, ESOTC, communication, KPIs, payment plan), §8 Tender Format, §9 Acronyms.
3. Enumerated lot boundaries from §3 ("two distinct Lots"), §4 and §5 headings, and separate-submission requirement.
4. Extracted Lot 1 requirements across §4.2–§4.4, decomposing technical requirements §4.3.1 through §4.3.5 into discrete obligations and mapping each to an evidence anchor.
5. Extracted Lot 2 requirements across §5.1–§5.4, including §5.2.1 operational chain, §5.2.1.1 required capability, §5.2.2 AI methods, §5.2.3 indicator family, §5.2.4.2 CMIP/Atlas restructuring.
6. Attributed §6 Joint products as a cross-lot product-requirement seed, keyed to Lot 1 (windstorm catalogue anchors the joint diagnostics) with an explicit cross-lot warning.
7. Mapped §8.2.1–§8.2.5 proposal-section guidance to lot-level `evaluation_criteria` with page limits from Table 3 §8.1 as `weighting_text`; added §7.8 KPI obligations as an additional evaluation dimension.
8. Identified technical standards: ECMWF Codex (github.com/ecmwf/codex), Copernicus data licence, GCOS ECVs, CMIP6/CMIP7/CORDEX, C3S Atlas philosophy, European Commission travel cost rules C(2024)5405, Earthkit, NetCDF delivery format, Global Tide and Surge Model (ERA5 forcing).
9. Cross-checked pre-existing `notice_lots` metadata (only Lot 2 present) and emitted `PREEXISTING_LOT_METADATA_INCOMPLETE` warning; spec-derived lot set takes precedence.
10. Wrote `result.json` and validated it against `result.schema.json` via `jsonschema`; validation passed.

## Decisions Made

- **Lots: Lot 1 and Lot 2 only.** §3 and the section headings at §4 and §5 are unambiguous. §6 is cross-lot coordination, not a third lot, so it is surfaced as a `joint_cross_lot_product` seed rather than a new lot object (decision preserves correct downstream set-replacement semantics for `notice_lots`).
- **Lot 1 cross-lot joint-product attribution.** The seed `joint-windstorm-stormsurge-products` is keyed to Lot 1 because the joint products explicitly build upon "the windstorm event catalogue and diagnostics developed and maintained under Lot 1" (§6.1), which anchors the event identity. A `JOINT_PRODUCTS_CROSS_LOT` warning documents the cross-lot responsibility so the daemon does not double-count.
- **CPV passthrough with a warning.** The tender text provides no CPV codes (§9.1 Acronyms defines abbreviations but no CPV). CPV `73000000-2` from `contract_notices.main_category` is attached to each lot with `CPV_NOT_IN_DOCUMENT` warning; daemon retains authority to reconcile against structured notice sources.
- **Evaluation criteria without numeric weightings.** §8 is explicitly "a guide to the minimum content expected" with page limits only — no numeric scoring weights are published in Volume II. Evaluation criteria are emitted with `weighting_text` = page-limit excerpt; `EVALUATION_WEIGHTINGS_NOT_SPECIFIED` warns downstream that weights may live in Volume IIIB which this skill does not process.
- **KPI SMART block is a sixth evaluation dimension.** §7.8 defines SMART KPI categories (system performance, search effectiveness, user satisfaction) that the proposal must address; surfaced as a `Key Performance Indicators (SMART)` criterion rather than as prose inside a technical requirement.
- **Sixteen product-requirement seeds.** Seed granularity chosen to match promotable units (datasets, applications, notebooks, methodologies, service operationalisation, indicator families, geographic extension, projections restructuring, joint products). Collapsed `4.3.3 User engagement` + `4.3.4 Demonstrators` + `4.3.5 Training/EQC/website` into two seeds (online demonstrators and training/dissemination) to avoid over-fragmenting engagement-only requirements.
- **`requirement_type` taxonomy.** Reused labels: `dataset`, `application`, `notebooks`, `r_and_d_methodology`, `indicator_family`, `service_maintenance_and_migration`, `service_operationalisation`, `service_migration`, `service_extension_geographic`, `service_extension_projections`, `derived_product_extension`, `engagement_and_studies`, `training_and_comms`, `training_and_support`, `joint_cross_lot_product`. Consistent across both lots.
- **`extraction_scope = full_document`.** Job request specifies `extraction_scope: full_document`; the underlying Docling payload is the complete `extracted_markdown`, so no chunk-merged scope applies.
- **Confidence bps = 8600.** High on structural items (lot count, titles, requirements), moderately lower on CPV (notice-level passthrough) and on evaluation weightings (not present). Granular per-field confidences captured in `field_confidences`.

## Warnings

- `CPV_NOT_IN_DOCUMENT` — CPV code on emitted lots sourced from notice metadata, not from the specification text. (Anchor: `md:§9.1`.)
- `EVALUATION_WEIGHTINGS_NOT_SPECIFIED` — Only proposal-section guidance and page limits are published in Volume II; numeric scoring weights not extractable here. (Anchors: `md:§8`, `md:§8.1`, `md:§8.2`.)
- `PRICING_CONTENT_ABSENT` — Pricing schedule and budget are declared to live in Volume IIIA / Volume IIIB; no price/quantity rows to extract. (Anchors: `md:§7.3`, `md:§7.9`.)
- `JOINT_PRODUCTS_CROSS_LOT` — §6 joint-product seed is cross-lot by design; attributed to Lot 1 for indexing. (Anchors: `md:§6`, `md:§6.1`, `md:§6.3`.)
- `PREEXISTING_LOT_METADATA_INCOMPLETE` — Existing `notice_lots` only records Lot 2; the specification authoritatively defines Lot 1 and Lot 2. Daemon should treat this extraction as authoritative for spec-derived lot structure. (Anchors: `md:§3`, `md:§4`, `md:§5`.)
- `KPI_AND_L2_SUPPORT_CROSS_LOT` — §§7.5, 7.6, 7.8, 7.9 apply to both lots; represented at the lot level without duplicating shared-section prose. (Anchors: `md:§7.5`, `md:§7.6`, `md:§7.8`, `md:§7.9`.)

## Evidence

Key anchors cited by the result contract (`result.json#/evidence_anchors` carries the full list):

- `md:§3` — §3 Contract summary: "Invitation to Tender … two distinct Lots"; drives lot count and separate-submission rule.
- `md:§4` / `md:§4.1` — Lot 1 header + background describing the Windstorm tracks and footprints dataset and the two prototype applications.
- `md:§4.3.1.1` — Catalogue of products to maintain: CDS dataset, two prototype applications, Jupyter Notebooks.
- `md:§4.3.1.3` — AI-based storm-tracking methodological requirement with benchmarking constraints.
- `md:§4.3.1.5` — ERA6 migration preparation.
- `md:§4.3.2.1` / `md:§4.3.2.2` / `md:§4.3.2.3` / `md:§4.3.2.4` — Geographic extension, precipitation & return periods, statistics augmentation (UNSEEN, synthetic events, AI generative), ETC indicators for reanalysis + projections.
- `md:§4.3.3` / `md:§4.3.4` / `md:§4.3.5` — User engagement, online demonstrators, training / dissemination / EQC / website.
- `md:§5` / `md:§5.1` — Lot 2 header + background describing current storm-surge product built on Global Tide and Surge Model forced by ERA5.
- `md:§5.2.1` / `md:§5.2.1.1` — Operational chain with NRT delivery, QA/QC, ERA6 preparation.
- `md:§5.2.2` — AI-based storm-surge methods (surrogate/emulation/hybrid).
- `md:§5.2.3` — Indicator family (percentiles, return periods, confidence bounds).
- `md:§5.2.4.2` — Climate-projections restructuring, broad CMIP6/7 selection, C3S Atlas alignment.
- `md:§6` / `md:§6.1` — Joint windstorm–storm-surge products as cross-lot coordination.
- `md:§7.1` — 28-month Framework Agreement, expected start Q3 2026.
- `md:§7.4` — ECMWF Codex governs all software development (SMP, PR workflow, Helm containerisation, documentation).
- `md:§7.5` — Level-2 support with 85% / 15-business-day KPI example.
- `md:§7.8` — SMART KPIs spanning system performance, search effectiveness, user satisfaction.
- `md:§8.1` / `md:§8.2` — Evaluation-structure guidance and page limits (Table 3), mapped to lot `evaluation_criteria`.
- `md:§9.1` — Acronyms — used to confirm absence of numeric CPV codes in the specification body.
