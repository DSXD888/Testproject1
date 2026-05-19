---
skill_id: validation/overlay-customs-classification
skill_version: 1.0.0
interaction_profile: headless_contract
result_schema_version: 1.0
checkpoint_schema_version: 1.0
validator_overlay_version: 1.0.0
validator_overlay_compatible_versions:
  worker_behavior_versions:
    - customs-classification/*@1.0.0+1.0
overlay_target_skills:
  - landed-cost/hmrc-customs-classification
  - landed-cost/taric-cross-check
  - customs/multi-source-recall
  - customs/measure-scanner
worker_behavior_version_compat:
  - "1.x"
model_profile: tier_2_audit
agent_cli: claude
auth_requirements: none
side_effect_class: read_only
promotion_stage: candidate
worker_behavior_version: customs-classification/*@1.0.0+1.0
validator_behavior_version: validation/overlay-customs-classification@1.0.0+1.0
version_change_affects_behavior: true
gate_a_family_key: customs-classification
gate_a_family_prequalified: false
gate_b_exact_slice_key: customs-classification::commercial-intelligence::customs-classification/*@1.0.0+1.0::1.0.0
gate_b_exact_slice_ready: false
---

# validation/overlay-customs-classification

## Use when
Use this overlay when validating L5 customs-classification worker manifests that claim multi-source recall, declarable UK10 resolution, measure interaction, exporter-rate basis, tariff branch divergence, decision grade, or customs redaction-boundary evidence. Apply it after the generic validator has checked manifest structure and daemon appendix consistency.

## Do not use when
Do not use this overlay for live HMRC or TARIC lookup, broker adjudication, supplier marketplace extraction, freight estimation, FX lookup, or any workflow that mutates authoritative pipeline state. Do not use FPO evidence as a primary classifier or relax conservative customs defaults.

## Inputs
- The worker run manifest and daemon evidence appendix for an L5 customs classification job.
- Candidate recall evidence from all five source families, including source-family identity, slot compatibility, and any FPO access status.
- Declarable UK10 branch evidence, HMRC measure evidence, TARIC cross-check evidence, supplementary-unit evidence, exporter identity evidence, origin evidence, and trade-remedy watch evidence supplied by the daemon.
- Worker and validator behavior versions for the promoted source being validated.
- Calibration provenance tags identifying clean, weird-valid, mutated-bad, and pairwise customs-classification cases.

## Outputs
- A validator verdict payload matching `result.schema.json`.
- Structured reasons and issue entries that cite manifest or daemon appendix evidence refs.
- `basis_sufficiency_state: insufficient` when fewer than two independent recall source families exist or when daemon-observed evidence is too thin to validate the worker claim.
- Rejected verdicts for hard-blocking customs failures such as branch divergence, unknown measure interaction, stale trade-remedy watch, or incompatible behavior versions.

## Domain Checks
- `candidate_recall_agreement`: exactly five source-family entries are present and at least two independent families support the accepted candidate family.
- `declarable_commodity_code_10`: any non-null declarable commodity code is a UK10 code and is supported by local expansion rather than greedy descent.
- `exporter_rate_basis`: exporter basis is one of `named_exporter`, `annex_exporter`, or `residual_default`, with residual default used when proof is insufficient.
- `duty_breakdown_json`: ad valorem, specific, and compound duty components are preserved without flattening to a single duty percentage.
- `customs_cost_spread_bps`: branch spread, remedy/no-remedy split, or duty-type split produces `TARIFF_BRANCH_DIVERGENCE` rather than an averaged estimate.
- `decision_grade`: decision grade is one of `auto_pass`, `review_required`, or `broker_required`, with hard blockers forcing `broker_required`.
- `redaction_boundary`: viewer-facing payloads do not expose exporter basis, origin evidence, remedy cases, duty breakdown, measure interaction, or evidence-leverage fields.
- `behavior_version_compatibility`: worker and validator behavior versions match an explicit compatible pair before promotion can become authoritative.

## Failure Codes
- CLASSIFICATION_AMBIGUOUS
- TARIFF_BRANCH_DIVERGENCE
- MEASURE_INTERACTION_UNKNOWN
- EXPORTER_UNRESOLVED
- SUPPLEMENTARY_UNIT_MISSING
- TRADE_REMEDY_WATCH_STALE
- FX_RATE_MISSING
- FREIGHT_ESTIMATE_FAILED
- FPO_ACCESS_UNAVAILABLE
- AGENT_OUTPUT_CONTRACT_INVALID
- AGENT_SUSPECT_OUTPUT
- AGENT_SKILL_FAILED

## Resume Strategy
Resume from the same manifest and daemon evidence appendix. If recall, UK10, measure, supplementary-unit, exporter, freshness, redaction, or behavior-version evidence is missing, emit a validator issue or insufficient-basis verdict instead of fetching live customs data or synthesizing missing proof.

## Canary
Run five canaries: clean fasteners approval, mutated fasteners rejection, clean cartons approval, pairwise corrupted cartons rejection, and weird-valid gloves flagged for operator review without automatic rejection.

## Run Manifest Instructions
Return only the structured validator result. Required manifest tokens for the worker evidence are `manifest_version`, `skill_id`, `skill_version`, `job_id`, `session_id`, and `execution_mode`. Required narrative sections are `Summary`, `Steps Taken`, `Decisions Made`, `Warnings`, and `Evidence`. Anchor every customs-domain issue to evidence references from the worker manifest, daemon appendix, HMRC/TARIC observation, calibration fixture, or behavior-version binding. Do not rely on hidden browser state, operator memory, or unstated broker judgment.
