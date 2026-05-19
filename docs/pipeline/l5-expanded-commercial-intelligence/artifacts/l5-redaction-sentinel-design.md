# L5 Redaction Sentinel Design

Status: Phase 4 design artifact. Phase 6 implements the field-level projection boundary.

## Contract

L5 activates decision-grade landed-cost and commodity-code fields that already exist in the L0/L4 schemas. L5 does not add columns to `landed_cost_estimates`. Viewer-facing projections may expose display fields and rounded decision-grade control values. Viewer-facing projections must not expose automation-sensitive fields by field name or by seeded sentinel value. Operator projections may expose display, control, and automation-sensitive fields, but push-owned mutation remains behind the approved L4 RPC path.

The Phase 6 implementation must not treat table-level RLS as sufficient evidence for `L5-T-measure_interaction-020`. A redacted view, role-scoped RPC serializer, or equivalent field-level projection is required.

## Classification Rules

| Classification | Viewer projection | Operator projection |
| --- | --- | --- |
| display | Present as stored. | Present as stored. |
| control | Present only as rounded or decision-grade values; raw artifacts absent. | Present as stored. |
| automation-sensitive | Absent by field name and sentinel value. | Present as stored. |
| local-only sensitive | Absent from shared projection. | Absent from shared projection; desktop may link to local artifact paths. |

## `landed_cost_estimates`

| Field | Classification | Rationale |
| --- | --- | --- |
| `id`, `process_id`, `notice_id`, `product_candidate_id`, `supplier_quote_id` | control | Join keys needed for operator audit; viewer should consume stable summaries. |
| `shipping_mode`, `mode_confidence_bps`, `packing_scenario` | control | Freight decision state; viewer may see normalized mode and confidence only. |
| `declarable_commodity_code_10` | display | Decision-grade commodity code is a first-class visible result. |
| `candidate_commodity_codes_json` | control | Recall lattice is useful for operator review but can expose ranking internals. |
| `additional_codes`, `additional_code_requirements_json`, `supplementary_units_required_json` | control | Required for operator review; viewer should see blockers/warnings instead of raw satisfiability payloads. |
| `commodity_confidence_bps`, `classification_stability_bps` | control | Numeric decision support; viewer may see rounded confidence. |
| `customs_valuation_method`, `valuation_mode` | control | Valuation basis, not raw automation reasoning. |
| `customs_fx_rate`, `commercial_fx_rate`, `fx_basis_delta_bps`, `fx_rate_checked_at` | control | Viewer may see rounded FX basis state; source artifacts stay absent. |
| `supplier_cost_gbp_pence` | control | Commercial cost input; viewer should see derived margin bands. |
| `freight_low_gbp_pence`, `freight_base_gbp_pence`, `freight_high_gbp_pence` | control | Freight decision-grade estimates. |
| `customs_duty_gbp_pence`, `trade_remedy_duty_gbp_pence`, `duty_outcome_min_gbp_pence`, `duty_outcome_max_gbp_pence`, `customs_cost_spread_bps` | control | Duty amounts and spread are review values; raw expression remains sensitive. |
| `trade_remedy_state` | control | Summary state is reviewable; detailed case payload is sensitive. |
| `trade_remedy_cases_json` | automation-sensitive | Raw remedy case evidence and matching logic. |
| `duty_breakdown_json` | automation-sensitive | Full duty expression and branch arithmetic. |
| `measure_interaction_json` | automation-sensitive | Measure interaction reasoning and branch-specific behavior. |
| `exporter_rate_basis` | automation-sensitive | Named-exporter and residual-rate basis can leak automation leverage. |
| `exporter_identity_confidence_bps` | automation-sensitive | Exporter identity score is automation-sensitive. |
| `origin_evidence_state` | automation-sensitive | Origin proof state can leak evidence gaps. |
| `import_vat_gbp_pence`, `vat_rate_state`, `import_vat_treatment`, `vat_recovery_rate_bps` | control | VAT review values. |
| `uk_final_mile_cost_gbp_pence`, `other_fees_gbp_pence`, `economic_landed_cost_gbp_pence`, `cash_outlay_gbp_pence`, `cash_to_clear_gbp_pence`, `working_capital_cost_gbp_pence` | control | Landed-cost rollup values. |
| `revenue_basis_type`, `revenue_confidence_bps` | control | Revenue basis and confidence for operator review. |
| `net_margin_low_bps`, `net_margin_base_bps`, `net_margin_high_bps` | display | Viewer-facing margin band. |
| `customs_resolution_confidence_bps`, `commercial_estimate_confidence_bps`, `confidence_bps` | control | Viewer may see rounded confidence. |
| `decision_grade`, `warnings`, `blockers` | display | Bid-readiness decision state. |
| `confidence_breakdown` | control | Summary may be exposed to operator; nested `evidence_leverage` is sensitive. |
| `confidence_breakdown.evidence_leverage` | automation-sensitive | Per-parameter unlockable evidence and margin leverage. |
| `tariff_checked_at`, `measures_fresh_until`, `classification_fresh_until`, `trade_remedy_watch_state` | control | Freshness vectors for operator review; viewer should see decision-grade freshness state. |
| `cbam_exposure_flag` | display | Forward exposure flag only. |
| `cbam_estimate_gbp_pence` | control | Nullable forward estimate; remains null in L5. |
| `owner_id`, `row_version`, `last_local_seq`, `stage_run_id`, `updated_at`, `created_at` | control | Push-owned sync and audit metadata. |

## `product_commodity_codes`

| Field | Classification | Rationale |
| --- | --- | --- |
| `id`, `product_candidate_id`, `owner_id`, `row_version`, `last_local_seq`, `stage_run_id`, `created_at`, `updated_at` | control | Sync and audit metadata. |
| `commodity_code`, `digits`, `source`, `score_bps`, `rank_ordinal`, `description` | control | Recall-lattice review state; viewer should consume the selected display code from landed cost. |
| `evidence_json` | automation-sensitive | Raw recall-source agreement and evidence payload. |

## Automation-Sensitive Sentinels

The canonical sentinel fixture is `tests/fixtures/l5/redaction/sentinels.yaml`. Phase 6 tests seed these exact values and must fail if a viewer-scoped projection returns any of them:

- `exporter_rate_basis`
- `exporter_identity_confidence_bps`
- `origin_evidence_state`
- `trade_remedy_cases_json`
- `duty_breakdown_json`
- `measure_interaction_json`
- `confidence_breakdown.evidence_leverage`
- `product_commodity_codes.evidence_json`

## L4 Policy Mapping

The existing policy file is `infra/supabase/migrations/0004_rls_policies.sql`. It grants viewer/operator/admin SELECT on push-owned projection tables. That remains valid as a table-access baseline, but it is not sufficient for L5. Phase 6 must add a field-level redaction layer for `landed_cost_estimates` and `product_commodity_codes` before exposing decision-grade rows to the viewer role.

Approved mutation remains the existing daemon/admin sync RPC path in `infra/supabase/migrations/0005_sync_rpcs.sql`. Viewer and operator roles stay read-only for push-owned L5 projection rows outside approved RPCs.

## Local-Only Sensitive Artifacts

Browser transcripts, raw marketplace pages, skill checkpoints, service keys, credential state, and raw CDP artifacts are never projected to Supabase. Operator desktop flows may link to local artifacts, but the shared projection must not contain those paths or payloads.
