---
skill_id: validation/overlay-procurement-ingestion
skill_version: 2026.05.06-1
interaction_profile: headless_contract
result_schema_version: 1.0
checkpoint_schema_version: 1.0
validator_overlay_version: 2026.05.06-1
validator_overlay_compatible_versions:
  worker_behavior_versions:
    - procurement-ingestion/*@2026.05.06-1+1.0
model_profile: structured_extract_low_cost
agent_cli: claude
side_effect_class: read_only
promotion_stage: candidate
worker_behavior_version: procurement-ingestion/*@2026.05.06-1+1.0
validator_behavior_version: validation/overlay-procurement-ingestion@2026.05.06-1+1.0
version_change_affects_behavior: true
gate_a_family_key: procurement-ingestion
gate_a_family_prequalified: false
gate_b_exact_slice_key: procurement-ingestion::official-notice-ingestion::procurement-ingestion/*@2026.05.06-1+1.0::2026.05.06-1
gate_b_exact_slice_ready: false
---

# validation/overlay-procurement-ingestion

## Use when
Use this overlay when validating procurement-ingestion worker manifests for official public-notice collection, normalization, and source evidence. Apply it after the generic validator has checked manifest structure and daemon appendix consistency.

## Do not use when
Do not use this overlay for supplier pricing, document acquisition, product extraction, margin scoring, or bid workflow decisions. Do not mutate pipeline state or attempt source scraping.

## Inputs
- The worker run manifest and daemon evidence appendix for a procurement-ingestion job.
- The normalized notice sample, source URL, source locale, notice type, expiry fields, CPV codes, and OCDS fragments supplied by the daemon.
- Calibration provenance tags identifying clean-real, weird-valid, mutated-bad, and pairwise canary cases.

## Outputs
- A validator verdict payload matching `result.schema.json`.
- Issue entries that use the procurement-ingestion domain categories and cite manifest or daemon appendix evidence refs.
- A low-basis or rejected verdict when source evidence is insufficient for a confident domain judgment.

## Domain Checks
- `notice_completeness`: required notice identity, buyer, title, published date, deadline, and value evidence are present or explicitly absent with rationale.
- `ocds_field_presence`: normalized data contains the expected OCDS identity and tender fragments for the claimed notice type.
- `source_locale`: source country, language, currency, and date conventions match the declared source locale.
- `notice_type_consistency`: the manifest, source page, and normalized payload agree on tender/planning/award/dynamic-market semantics.
- `expiry_plausibility`: deadlines and expiry dates are internally ordered and plausible relative to publication time.
- `cpv_validity`: CPV codes are well-formed and compatible with the notice family.

## Failure Codes
- AGENT_OUTPUT_CONTRACT_INVALID
- AGENT_SUSPECT_OUTPUT
- AGENT_SKILL_FAILED
- PAGE_SHAPE_DRIFT
- SOURCE_CHANGED
- NOTICE_PARSE_FAILED

## Resume Strategy
Resume from the same manifest and daemon evidence appendix. If evidence for source locale, notice type, expiry, or CPV validity is missing, emit a validator issue instead of fetching live data.

## Canary
Run four canaries: clean-real approval, weird-valid non-rejection, mutated-bad rejection, and one pairwise clean/corrupted comparison over source identity.

## Run Manifest Instructions
Return only the structured validator result. Anchor every domain issue to evidence references from the worker manifest, normalized notice sample, or daemon appendix. Do not rely on hidden browser state or operator memory.
