---
skill_id: validation/overlay-product-extraction
skill_version: 2026.05.06-1
interaction_profile: headless_contract
result_schema_version: 1.0
checkpoint_schema_version: 1.0
validator_overlay_version: 2026.05.06-1
validator_overlay_compatible_versions:
  worker_behavior_versions:
    - product-extraction/*@2026.05.06-1+1.0
model_profile: structured_extract_low_cost
agent_cli: claude
side_effect_class: read_only
promotion_stage: candidate
worker_behavior_version: product-extraction/*@2026.05.06-1+1.0
validator_behavior_version: validation/overlay-product-extraction@2026.05.06-1+1.0
version_change_affects_behavior: true
gate_a_family_key: product-extraction
gate_a_family_prequalified: false
gate_b_exact_slice_key: product-extraction::product-candidates::product-extraction/*@2026.05.06-1+1.0::2026.05.06-1
gate_b_exact_slice_ready: false
---

# validation/overlay-product-extraction

## Use when
Use this overlay when validating product-extraction worker manifests that convert procurement document evidence into quoteable product candidates, archetype slots, quantity evidence, and bundle decomposition. Apply it after the generic validator has checked manifest structure and daemon appendix consistency.

## Do not use when
Do not use this overlay for procurement notice ingestion, supplier enrichment, document acquisition, margin scoring, or bid workflow decisions. Do not mutate pipeline state or infer missing product details from live browsing.

## Inputs
- The worker run manifest and daemon evidence appendix for a product-extraction job.
- Product candidate rows, CPV family context, archetype assignments, slot evidence, quantity and unit evidence, bundle decomposition notes, and source document refs supplied by the daemon.
- Calibration provenance tags identifying clean-real, weird-valid, mutated-bad, and pairwise canary cases.

## Outputs
- A validator verdict payload matching `result.schema.json`.
- Issue entries that use the product-extraction domain categories and cite manifest or daemon appendix evidence refs.
- A low-basis or rejected verdict when product evidence is insufficient for a confident domain judgment.

## Domain Checks
- `candidate_type_plausibility`: extracted product candidates are plausible for the CPV family and notice scope.
- `archetype_assignment_consistency`: each candidate's archetype assignment is consistent with evidence and required downstream pricing logic.
- `slot_completeness`: required archetype slots are populated or explicitly marked unavailable with evidence.
- `quantity_unit_plausibility`: quantities, units, pack sizes, and basis conversions are plausible and internally consistent.
- `bundle_decomposition_correctness`: bundles, kits, lots, or mixed sets are decomposed without duplicating or dropping quoteable items.

## Failure Codes
- AGENT_OUTPUT_CONTRACT_INVALID
- AGENT_SUSPECT_OUTPUT
- AGENT_SKILL_FAILED
- PRODUCT_EXTRACTION_IMPLAUSIBLE
- SOURCE_CHANGED
- PRODUCT_EVIDENCE_INSUFFICIENT

## Resume Strategy
Resume from the same manifest and daemon evidence appendix. If candidate type, archetype, slot, quantity/unit, or bundle evidence is missing, emit a validator issue instead of inventing a product candidate.

## Canary
Run four canaries: clean-real approval, weird-valid non-rejection, mutated-bad rejection, and one pairwise clean/corrupted comparison over archetype slots and quantity/unit evidence.

## Run Manifest Instructions
Return only the structured validator result. Anchor every domain issue to evidence references from the worker manifest, product candidate rows, source document refs, or daemon appendix. Do not rely on hidden browser state or operator memory.
