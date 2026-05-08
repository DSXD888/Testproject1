---
skill_id: validation/overlay-supplier-enrichment
skill_version: 2026.05.06-1
interaction_profile: headless_contract
result_schema_version: 1.0
checkpoint_schema_version: 1.0
validator_overlay_version: 2026.05.06-1
validator_overlay_compatible_versions:
  worker_behavior_versions:
    - supplier-enrichment/*@2026.05.06-1+1.0
model_profile: structured_extract_low_cost
agent_cli: claude
side_effect_class: read_only
promotion_stage: candidate
worker_behavior_version: supplier-enrichment/*@2026.05.06-1+1.0
validator_behavior_version: validation/overlay-supplier-enrichment@2026.05.06-1+1.0
version_change_affects_behavior: true
gate_a_family_key: supplier-enrichment
gate_a_family_prequalified: false
gate_b_exact_slice_key: supplier-enrichment::supplier-pricing::supplier-enrichment/*@2026.05.06-1+1.0::2026.05.06-1
gate_b_exact_slice_ready: false
---

# validation/overlay-supplier-enrichment

## Use when
Use this overlay when validating supplier-enrichment worker manifests that collect supplier listings, pricing signals, seller context, freshness evidence, and regulated-goods compliance evidence. Apply it after the generic validator has checked manifest structure and daemon appendix consistency.

## Do not use when
Do not use this overlay for procurement notice ingestion, document acquisition, product extraction, margin scoring, or bid workflow decisions. Do not mutate pipeline state or attempt live supplier scraping.

## Inputs
- The worker run manifest and daemon evidence appendix for a supplier-enrichment job.
- Supplier listing samples, quoted prices, currencies, pack or unit basis, seller identity set, listing timestamps, and regulated-goods evidence supplied by the daemon.
- Calibration provenance tags identifying clean-real, weird-valid, mutated-bad, and pairwise canary cases.

## Outputs
- A validator verdict payload matching `result.schema.json`.
- Issue entries that use the supplier-enrichment domain categories and cite manifest or daemon appendix evidence refs.
- A low-basis or rejected verdict when supplier evidence is insufficient for a confident domain judgment.

## Domain Checks
- `price_plausibility`: quoted prices are plausible for the product category, pack size, and market context.
- `currency_correctness`: claimed currencies match listing evidence and conversion context.
- `pack_basis_consistency`: unit, box, case, pallet, and minimum-order bases are internally consistent.
- `seller_diversity`: enrichment is not overfitted to a single ambiguous seller when multiple independent sellers are required.
- `listing_freshness`: listing timestamps, scrape timestamps, and freshness windows support the claimed price signal.
- `regulated_goods_compliance_evidence`: regulated-goods claims include required compliance, certification, or restriction evidence.

## Failure Codes
- AGENT_OUTPUT_CONTRACT_INVALID
- AGENT_SUSPECT_OUTPUT
- AGENT_SKILL_FAILED
- PRICE_SIGNAL_IMPLAUSIBLE
- SOURCE_CHANGED
- LISTING_EVIDENCE_INSUFFICIENT

## Resume Strategy
Resume from the same manifest and daemon evidence appendix. If price, currency, pack basis, seller, freshness, or compliance evidence is missing, emit a validator issue instead of fetching live data.

## Canary
Run four canaries: clean-real approval, weird-valid non-rejection, mutated-bad rejection, and one pairwise clean/corrupted comparison over pack and currency evidence.

## Run Manifest Instructions
Return only the structured validator result. Anchor every domain issue to evidence references from the worker manifest, supplier listing sample, or daemon appendix. Do not rely on hidden browser state or operator memory.
