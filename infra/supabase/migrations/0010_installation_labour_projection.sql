ALTER TABLE IF EXISTS landed_cost_estimates
  ADD COLUMN IF NOT EXISTS goods_landed_cost_gbp_pence bigint NOT NULL DEFAULT 0
    CHECK (goods_landed_cost_gbp_pence >= 0),
  ADD COLUMN IF NOT EXISTS installation_labour_cost_gbp_pence bigint NOT NULL DEFAULT 0
    CHECK (installation_labour_cost_gbp_pence >= 0),
  ADD COLUMN IF NOT EXISTS installation_labour_low_gbp_pence bigint NOT NULL DEFAULT 0
    CHECK (installation_labour_low_gbp_pence >= 0),
  ADD COLUMN IF NOT EXISTS installation_labour_high_gbp_pence bigint NOT NULL DEFAULT 0
    CHECK (installation_labour_high_gbp_pence >= 0),
  ADD COLUMN IF NOT EXISTS installation_labour_confidence_bps integer NOT NULL DEFAULT 0
    CHECK (installation_labour_confidence_bps BETWEEN 0 AND 10000),
  ADD COLUMN IF NOT EXISTS permit_traffic_mgmt_cost_gbp_pence bigint NOT NULL DEFAULT 0
    CHECK (permit_traffic_mgmt_cost_gbp_pence >= 0),
  ADD COLUMN IF NOT EXISTS permit_traffic_mgmt_low_gbp_pence bigint NOT NULL DEFAULT 0
    CHECK (permit_traffic_mgmt_low_gbp_pence >= 0),
  ADD COLUMN IF NOT EXISTS permit_traffic_mgmt_high_gbp_pence bigint NOT NULL DEFAULT 0
    CHECK (permit_traffic_mgmt_high_gbp_pence >= 0),
  ADD COLUMN IF NOT EXISTS permit_responsibility text
    CHECK (
      permit_responsibility IS NULL
      OR permit_responsibility IN (
        'authority_provided',
        'contractor_provided',
        'shared',
        'unknown'
      )
    );

ALTER TABLE IF EXISTS pipeline_assessments
  ADD COLUMN IF NOT EXISTS goods_value_share_bps integer
    CHECK (goods_value_share_bps BETWEEN 0 AND 10000),
  ADD COLUMN IF NOT EXISTS goods_value_share_basis text
    CHECK (
      goods_value_share_basis IS NULL
      OR goods_value_share_basis IN ('contract_value', 'cost_proxy')
    ),
  ADD COLUMN IF NOT EXISTS installation_labour_coverage_bps integer
    CHECK (installation_labour_coverage_bps BETWEEN 0 AND 10000),
  ADD COLUMN IF NOT EXISTS installation_labour_value_coverage_bps integer
    CHECK (installation_labour_value_coverage_bps BETWEEN 0 AND 10000),
  ADD COLUMN IF NOT EXISTS permit_traffic_mgmt_coverage_bps integer
    CHECK (permit_traffic_mgmt_coverage_bps BETWEEN 0 AND 10000),
  ADD COLUMN IF NOT EXISTS permit_traffic_mgmt_value_coverage_bps integer
    CHECK (permit_traffic_mgmt_value_coverage_bps BETWEEN 0 AND 10000);

CREATE OR REPLACE VIEW landed_cost_estimates_viewer
WITH (security_barrier = true) AS
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
  customs_fx_rate,
  commercial_fx_rate,
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
  trade_remedy_cases_json,
  duty_breakdown_json,
  measure_interaction_json,
  exporter_rate_basis,
  exporter_identity_confidence_bps,
  origin_evidence_state,
  import_vat_gbp_pence,
  vat_rate_state,
  import_vat_treatment,
  vat_recovery_rate_bps,
  goods_landed_cost_gbp_pence,
  uk_final_mile_cost_gbp_pence,
  installation_labour_cost_gbp_pence,
  installation_labour_low_gbp_pence,
  installation_labour_high_gbp_pence,
  installation_labour_confidence_bps,
  permit_traffic_mgmt_cost_gbp_pence,
  permit_traffic_mgmt_low_gbp_pence,
  permit_traffic_mgmt_high_gbp_pence,
  permit_responsibility,
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

GRANT SELECT ON landed_cost_estimates_viewer TO anon, authenticated;

INSERT INTO schema_metadata (org_id, schema_epoch, projection_generation)
VALUES ('default', 10, 0)
ON CONFLICT (org_id) DO UPDATE
SET schema_epoch = 10,
    updated_at = now();
