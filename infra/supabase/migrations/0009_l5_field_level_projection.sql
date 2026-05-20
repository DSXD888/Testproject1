CREATE OR REPLACE FUNCTION l5_viewer_confidence_breakdown(payload jsonb)
RETURNS jsonb
LANGUAGE sql
IMMUTABLE
AS $$
  SELECT CASE
    WHEN payload IS NULL THEN NULL
    ELSE payload - 'evidence_leverage'
  END;
$$;

COMMENT ON FUNCTION l5_viewer_confidence_breakdown(jsonb) IS
  'L5-T-measure_interaction-020: strips automation-sensitive evidence leverage from viewer projection payloads.';

DROP POLICY IF EXISTS landed_cost_estimates_viewer_operator_admin_select
  ON landed_cost_estimates;
DROP POLICY IF EXISTS landed_cost_estimates_operator_admin_daemon_select
  ON landed_cost_estimates;
CREATE POLICY landed_cost_estimates_operator_admin_daemon_select
  ON landed_cost_estimates
  FOR SELECT
  USING (l4_is_any_role(ARRAY['operator', 'admin', 'daemon']));

DROP POLICY IF EXISTS product_commodity_codes_viewer_operator_admin_select
  ON product_commodity_codes;
DROP POLICY IF EXISTS product_commodity_codes_operator_admin_daemon_select
  ON product_commodity_codes;
CREATE POLICY product_commodity_codes_operator_admin_daemon_select
  ON product_commodity_codes
  FOR SELECT
  USING (l4_is_any_role(ARRAY['operator', 'admin', 'daemon']));

CREATE OR REPLACE VIEW landed_cost_estimates_viewer
WITH (security_barrier = true)
AS
SELECT
  id,
  process_id,
  notice_id,
  product_candidate_id,
  supplier_quote_id,
  shipping_mode,
  mode_confidence_bps,
  packing_scenario,
  declarable_commodity_code_10,
  candidate_commodity_codes_json,
  additional_codes,
  additional_code_requirements_json,
  supplementary_units_required_json,
  commodity_confidence_bps,
  classification_stability_bps,
  customs_valuation_method,
  valuation_mode,
  round(customs_fx_rate, 6) AS customs_fx_rate,
  round(commercial_fx_rate, 6) AS commercial_fx_rate,
  fx_basis_delta_bps,
  fx_rate_checked_at,
  supplier_cost_gbp_pence,
  freight_low_gbp_pence,
  freight_base_gbp_pence,
  freight_high_gbp_pence,
  customs_duty_gbp_pence,
  trade_remedy_duty_gbp_pence,
  duty_outcome_min_gbp_pence,
  duty_outcome_max_gbp_pence,
  customs_cost_spread_bps,
  trade_remedy_state,
  import_vat_gbp_pence,
  vat_rate_state,
  import_vat_treatment,
  vat_recovery_rate_bps,
  uk_final_mile_cost_gbp_pence,
  other_fees_gbp_pence,
  economic_landed_cost_gbp_pence,
  cash_outlay_gbp_pence,
  cash_to_clear_gbp_pence,
  working_capital_cost_gbp_pence,
  revenue_basis_type,
  revenue_confidence_bps,
  customs_resolution_confidence_bps,
  commercial_estimate_confidence_bps,
  net_margin_low_bps,
  net_margin_base_bps,
  net_margin_high_bps,
  decision_grade,
  confidence_bps,
  l5_viewer_confidence_breakdown(confidence_breakdown) AS confidence_breakdown,
  warnings,
  blockers,
  tariff_checked_at,
  measures_fresh_until,
  classification_fresh_until,
  trade_remedy_watch_state,
  cbam_exposure_flag,
  cbam_estimate_gbp_pence,
  owner_id,
  row_version,
  last_local_seq,
  stage_run_id,
  updated_at,
  created_at
FROM landed_cost_estimates;

COMMENT ON VIEW landed_cost_estimates_viewer IS
  'L5-T-measure_interaction-020: viewer-safe landed-cost projection; operators read landed_cost_estimates directly.';

CREATE OR REPLACE VIEW product_commodity_codes_viewer
WITH (security_barrier = true)
AS
SELECT
  id,
  product_candidate_id,
  digits,
  source,
  score_bps,
  rank_ordinal,
  description,
  owner_id,
  row_version,
  last_local_seq,
  stage_run_id,
  updated_at,
  created_at
FROM product_commodity_codes;

COMMENT ON VIEW product_commodity_codes_viewer IS
  'L5-T-measure_interaction-020: viewer-safe commodity-code projection; candidate code evidence remains operator-only.';

GRANT SELECT ON landed_cost_estimates_viewer TO anon, authenticated;
GRANT SELECT ON product_commodity_codes_viewer TO anon, authenticated;
GRANT EXECUTE ON FUNCTION l5_viewer_confidence_breakdown(jsonb) TO anon, authenticated, service_role;
