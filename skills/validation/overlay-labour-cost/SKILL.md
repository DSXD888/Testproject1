---
skill_id: validation/overlay-labour-cost
skill_version: 0.1.0
interaction_profile: headless_contract
result_schema_version: 1.0.0
checkpoint_schema_version: 1.0.0
validator_overlay_version: 0.1.0
validator_overlay_compatible_versions:
  worker_behavior_versions:
    - landed-cost/uk-installation-labour-estimate@0.1.0
model_profile: structured_extract_low_cost
agent_cli: claude
side_effect_class: read_only
promotion_stage: candidate
worker_behavior_version: landed-cost/uk-installation-labour-estimate@0.1.0
validator_behavior_version: validation/overlay-labour-cost@0.1.0
version_change_affects_behavior: true
gate_a_family_key: uk-installation-labour
gate_a_family_prequalified: false
gate_b_exact_slice_key: uk-installation-labour::overlay::landed-cost/uk-installation-labour-estimate@0.1.0::0.1.0
gate_b_exact_slice_ready: false
---

# validation/overlay-labour-cost

## Use when
Use this overlay after the generic validator has accepted the structure of a `landed-cost/uk-installation-labour-estimate` result. It validates screening-grade UK installation, de-installation, access-equipment, permit/traffic-management, and maintenance-warning evidence before the daemon trusts the worker's labour cost line in landed-cost synthesis.

## Do not use when
Do not use this overlay for pure-goods candidates with no installation requirement, service-only qualification decisions, freight, customs, VAT, supplier pricing, or any live scraping workflow. Do not fetch external rate data, mutate production reference files, or decide operator disposition directly.

## Inputs
- The worker JSON result envelope from `landed-cost/uk-installation-labour-estimate`.
- The original landed-cost labour job input, including destination postcode, UK region hints, installation requirement, goods-landed-cost pence, and rate-table version.
- Skill-local evidence emitted by the worker: profile-resolution evidence, rate-table provenance, access-equipment selection evidence, permit/TM evidence, warnings, and blockers.
- Calibration provenance tags from `canary.inputs.jsonl` and expected assertions from `golden.assertions.yaml`.

## Outputs
- A validator verdict payload matching `result.schema.json`.
- Domain issues anchored to worker evidence references.
- `downstream_route: review` whenever the worker output is rejected, flagged with insufficient basis, blocked, or unavailable.
- A concise summary naming the checked labour-cost domains.

## Domain Checks
Spec coverage: hours versus scope; regional rate bounds; postcode/region plausibility; equipment proportionality; permit responsibility handling; warning completeness.

- `hours_scope_consistency`: labour hours, crew size, trades, and duration are plausible for the resolved installation profile and visible scope.
- `regional_rate_bounds`: hourly/day rates, UK-region multipliers, and rate-table versions stay within the worker's declared reference-table bounds.
- `postcode_region_plausibility`: destination postcode, declared `uk_region`, fallback region handling, and `LABOUR_REGION_UNKNOWN` warning evidence are mutually plausible.
- `access_equipment_proportionality`: access-equipment class, hire days, operated-hire default, and at-height warnings are proportional to the installation requirement.
- `permit_responsibility_handling`: permit and traffic-management costs remain separate from labour, respect responsibility, and emit review warnings for unknown public-realm exposure.
- `warning_completeness`: unresolved region, scope, access, stale-rate, permit, lane-rental, TTRO, and maintenance-term assumptions appear as explicit warning or blocker codes.

## Non-goals
- This overlay does not produce a firm installer quote.
- This overlay does not update labour-rate, access-equipment, profile, or permit/TM reference tables.
- This overlay does not capitalise recurring maintenance into the labour denominator.
- This overlay does not bypass downstream qualification, margin, or operator review gates.

## Failure Codes
- AGENT_OUTPUT_CONTRACT_INVALID
- AGENT_SUSPECT_OUTPUT
- AGENT_SKILL_FAILED
- LABOUR_HOURS_SCOPE_MISMATCH
- LABOUR_RATE_OUT_OF_BOUNDS
- LABOUR_REGION_INCONSISTENT
- ACCESS_EQUIPMENT_IMPLAUSIBLE
- PERMIT_TM_NOT_SEPARATE
- WARNING_COVERAGE_INCOMPLETE

## Resume Strategy
Resume from the same worker result, input payload, and daemon evidence appendix. If worker output or reference evidence is missing, emit a flagged or rejected validator result with `downstream_route: review` instead of recomputing labour, fetching live data, or treating the estimate as passed.

## Canary
Run three canaries: a clean public-realm lighting result that approves, a mutated result with hours/rate/equipment/permit/warning defects that rejects and routes to review, and a worker-outage case that flags insufficient basis and routes to review without crashing.

## Run Manifest Instructions
Return only the structured validator result. Required manifest tokens for audit are `manifest_version`, `skill_id`, `skill_version`, `job_id`, `session_id`, and `execution_mode`. Required narrative sections are `Summary`, `Steps Taken`, `Decisions Made`, `Warnings`, and `Evidence`. Cite evidence references from the worker result, job input, calibration fixture, or daemon appendix for every labour-domain issue. Do not rely on hidden browser state, operator memory, or unstated installer judgment.
