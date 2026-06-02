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
The input JSON object must include:

- `process_id`
- `notice_id`
- `product_candidate_id`
- `selected_quote_id`
- `destination_country`
- `destination_postcode`
- `installation_requirement`
- `goods_landed_cost_gbp_pence`
- `rate_table_version`

The `installation_requirement` object is derived from the product candidate when the notice implies installation, de-installation, or maintenance. It must carry:

- `requires_installation`
- `requires_maintenance`
- `installation_scope`
- `maintenance_schedule`
- `site_access`
- `at_height_m`
- `uk_region`
- `estimated_units`
- `equipment_hint`
- `permit_responsibility`
- `public_realm`
- `lane_rental_zone`

When there is no installation, de-installation, or maintenance signal, do not fabricate an `InstallationRequirement`. The daemon skips this worker for pure-goods candidates and leaves labour and permit/traffic-management pence at zero or null.

## Outputs
Write raw JSON to `result.json`, not Markdown. The result envelope must include:

- `status`
- `labour_status`
- `uk_region`
- `labour_line_items`
- `access_equipment_line_items`
- `labour_base_gbp_pence`
- `labour_low_gbp_pence`
- `labour_high_gbp_pence`
- `labour_confidence_bps`
- `permit_traffic_mgmt_line_items`
- `permit_traffic_mgmt_base_gbp_pence`
- `permit_traffic_mgmt_low_gbp_pence`
- `permit_traffic_mgmt_high_gbp_pence`
- `permit_responsibility`
- `vat_treatment`
- `excluded_cost_flags`
- `evidence`
- `warnings`
- `blockers`

All money values are integer pence. Confidence and share ratios are basis points.

## Rules
- Resolve `uk_region` from `destination_country` and `destination_postcode` before pricing. Unknown or unmapped UK regions fall back to the `national` baseline multiplier (`10000` bps), emit `LABOUR_REGION_UNKNOWN`, and keep `labour_status` as `estimated`.
- Resolve the installation profile catalogue-first from `reference/uk-installation-profiles-2026-q2.json` using, in order, an explicit `installation_requirement.profile_hint`, profile archetypes/head nouns/keywords, and the combined `installation_scope`/`scope_text`. A catalogue match supplies the required trade-hour map, crew size, access-equipment preference, complexity multiplier, maintenance model, permit hint, warnings, and confidence.
- The catalogue must keep first-class rows for flat-pack furniture, wall-fixed shelving/racking, standard glazing/windows, festive or LED lighting, signage, fencing, playground/outdoor equipment, CCTV/security equipment, traffic signals as the labour-heavy boundary archetype, and made-to-measure windows as the bespoke/no-list review archetype.
- If no catalogue profile matches, use the constrained fallback profile. The fallback may only choose trade keys present in `reference/uk-regional-labour-rates-2026-q2.json` under `national_trade_rates`; never invent a trade name. Cap fallback confidence below catalogue-match confidence, emit `LABOUR_SCOPE_UNRESOLVED`, and include evidence naming the inferred archetype.
- Every estimate must include profile-resolution evidence sufficient for the overlay to audit implausible mappings: `profile_id`, `match_basis` (`catalogue` or `constrained_fallback`), matched or inferred archetype/head noun, selected trades, access-equipment class, duration days, complexity multiplier, and any bespoke/no-list route reason.
- For any matched profile with `deinstallation.included`, model installation and de-installation as separate labour visits. Price the main `trade_hours` and the `deinstallation.trade_hours` independently; never amortise the work across a weekly hire or merge the de-installation hours into the installation visit.
- Emit separate `labour_line_items` for each labour event and trade. Each item must include at least `labour_type`, `trade`, `quantity_hours`, `rate_gbp_per_hour_pence`, `subtotal_gbp_pence`, `confidence_bps`, and `rate_source`. Use integer pence for rates and subtotals and basis points for confidence.
- Scenario 1 (`lights-nw-2day-01`) must resolve to `festive_led_lighting_public_realm`, emit both `installation` and `deinstallation` labour events for `electrician` and `labourer`, apply the NW multiplier (`9300` bps), and keep `installation_labour_cost_gbp_pence` within `210000..=265000`.
- Scenario 9 CCTV/security must resolve to the `cctv_security_install` profile, use `security_installer` plus `electrician`, apply the YH regional multiplier (`8800` bps), and select access equipment only for at-height camera mounts. Do not collapse it onto the Scenario 1 festive/LED lighting trade set.
- Default `labour_low_gbp_pence` to base minus 20 percent and `labour_high_gbp_pence` to base plus 20 percent unless narrower evidence-backed bounds are available.
- Keep permit and traffic-management costs separate from labour. Never fold permit or traffic-management pence into the labour line items.
- Set `excluded_cost_flags` for screening omissions, including firm pricing, subcontractor compliance, delivery responsibility, authority-resolved permit uncertainty, VAT treatment assumptions, and non-modelled plant or materials.
- Return `warnings` and `blockers` as arrays of objects with `code`, `message`, and `severity`.
- Use `vat_treatment` to state the VAT assumption explicitly; estimates remain screening-grade and are not firm quotes.

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
