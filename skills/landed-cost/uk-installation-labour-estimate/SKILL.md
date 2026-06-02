---
skill_id: landed-cost/uk-installation-labour-estimate
skill_version: 0.1.0
interaction_profile: http_public
result_schema_version: 1.0.0
model_profile: browse_stable_low_cost
requires_cdp: false
side_effect_class: none
agent_cli: claude
---

# landed-cost/uk-installation-labour-estimate

## Mission
Estimate UK installation and maintenance labour costs from the supplied landed-cost job input and skill-local reference tables.

## Use when
Use this skill for UK landed-cost jobs where a product candidate has an `InstallationRequirement` that calls for supply-and-install, de-installation, access equipment, public-realm permit or traffic management, or recurring maintenance labour estimates.

## Do not use when
Do not use this skill for pure-goods contracts with no installation requirement, service-only contracts, non-UK destination work, freight/customs/VAT calculation, or any workflow that requires browser access, sign-in, CDP, or external mutation.

## Inputs
- `process_id`
- `notice_id`
- `product_candidate_id`
- `selected_quote_id`
- `destination_country`
- `destination_postcode`
- `installation_requirement`
- `goods_landed_cost_gbp_pence`
- `rate_table_version`

## Outputs
Write raw JSON to `result.json`, not Markdown. The result envelope includes `status`, `labour_status`, low/base/high labour pence, permit and traffic-management low/base/high pence, warnings, blockers, and evidence. All money values are integer pence; confidence and share ratios are basis points.

## Non-goals
- This is a screening-grade estimate, not a firm installer quote.
- This skill does not perform, procure, schedule, manage, supervise, or warrant installation or maintenance delivery.
- This skill does not decide contract qualification, margin route, or operator disposition; downstream gates consume its estimate and evidence.
- This skill does not fold permit or traffic-management costs into labour totals.

## Failure Codes
- `LABOUR_REGION_UNKNOWN`: Region is missing or not mapped; use the national baseline and add review evidence.
- `LABOUR_SCOPE_UNRESOLVED`: Installation or maintenance scope cannot be resolved with enough specificity for a confident estimate.
- `ACCESS_AT_HEIGHT_ASSUMED_MEWP`: At-height work lacks enough access detail; assume a conservative MEWP band.
- `MAINTENANCE_TERM_ASSUMED`: Maintenance frequency or visible term is incomplete; use the documented default term assumption.
- `RATE_TABLE_STALE`: Requested or active rate table is stale; continue with the best available three-point estimate.
- `PERMIT_RESPONSIBILITY_UNKNOWN`: The buyer/supplier permit responsibility is unclear; estimate separately and route uncertainty downstream.
- `LANE_RENTAL_ZONE`: Site location appears to fall in a lane-rental-sensitive zone.
- `TTRO_LEAD_TIME_RISK`: Traffic regulation order or related public-realm lead time may affect delivery planning.

## Resume Strategy
The skill is deterministic for the same input payload and `rate_table_version`. On retry, reload the skill-local reference JSON files, recompute the full estimate from the original input, and overwrite only the current `result.json` output. If a reference table is missing, stale, or partially unusable, emit the relevant warning or blocker and return a fail-soft JSON envelope rather than crashing.

## Run Manifest Instructions
Every run manifest must include these tokens exactly so the daemon can audit the worker without transcript scraping:

- `manifest_version`
- `skill_id`
- `skill_version`
- `job_id`
- `session_id`
- `execution_mode`
- `Summary`
- `Steps Taken`
- `Decisions Made`
- `Warnings`
- `Evidence`
