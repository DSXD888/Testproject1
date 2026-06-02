PRAGMA foreign_keys = ON;

ALTER TABLE landed_cost_estimates
  ADD COLUMN goods_landed_cost_gbp_pence INTEGER NOT NULL DEFAULT 0 CHECK (
    goods_landed_cost_gbp_pence >= 0
  );

ALTER TABLE landed_cost_estimates
  ADD COLUMN installation_labour_cost_gbp_pence INTEGER NOT NULL DEFAULT 0 CHECK (
    installation_labour_cost_gbp_pence >= 0
  );

ALTER TABLE landed_cost_estimates
  ADD COLUMN installation_labour_low_gbp_pence INTEGER NOT NULL DEFAULT 0 CHECK (
    installation_labour_low_gbp_pence >= 0
  );

ALTER TABLE landed_cost_estimates
  ADD COLUMN installation_labour_high_gbp_pence INTEGER NOT NULL DEFAULT 0 CHECK (
    installation_labour_high_gbp_pence >= 0
  );

ALTER TABLE landed_cost_estimates
  ADD COLUMN installation_labour_confidence_bps INTEGER NOT NULL DEFAULT 0 CHECK (
    installation_labour_confidence_bps BETWEEN 0 AND 10000
  );

ALTER TABLE landed_cost_estimates
  ADD COLUMN permit_traffic_mgmt_cost_gbp_pence INTEGER NOT NULL DEFAULT 0 CHECK (
    permit_traffic_mgmt_cost_gbp_pence >= 0
  );

ALTER TABLE landed_cost_estimates
  ADD COLUMN permit_traffic_mgmt_low_gbp_pence INTEGER NOT NULL DEFAULT 0 CHECK (
    permit_traffic_mgmt_low_gbp_pence >= 0
  );

ALTER TABLE landed_cost_estimates
  ADD COLUMN permit_traffic_mgmt_high_gbp_pence INTEGER NOT NULL DEFAULT 0 CHECK (
    permit_traffic_mgmt_high_gbp_pence >= 0
  );

ALTER TABLE landed_cost_estimates
  ADD COLUMN permit_responsibility TEXT CHECK (
    permit_responsibility IS NULL
    OR permit_responsibility IN (
      'authority_provided',
      'contractor_provided',
      'shared',
      'unknown'
    )
  );

CREATE TABLE IF NOT EXISTS maintenance_period_estimates (
  id TEXT PRIMARY KEY,
  process_id TEXT NOT NULL REFERENCES procurement_processes(id),
  notice_id TEXT NOT NULL REFERENCES contract_notices(id),
  product_candidate_id TEXT NOT NULL REFERENCES product_candidates(id),
  landed_cost_estimate_id TEXT NOT NULL REFERENCES landed_cost_estimates(id),
  supplier_quote_id TEXT REFERENCES supplier_quotes(id),
  visit_scope TEXT,
  visits_per_year INTEGER NOT NULL DEFAULT 0 CHECK (visits_per_year >= 0),
  term_years INTEGER NOT NULL DEFAULT 0 CHECK (term_years >= 0),
  per_visit_labour_cost_gbp_pence INTEGER NOT NULL DEFAULT 0 CHECK (
    per_visit_labour_cost_gbp_pence >= 0
  ),
  annualised_opex_gbp_pence INTEGER NOT NULL DEFAULT 0 CHECK (
    annualised_opex_gbp_pence >= 0
  ),
  total_visible_term_opex_gbp_pence INTEGER NOT NULL DEFAULT 0 CHECK (
    total_visible_term_opex_gbp_pence >= 0
  ),
  maintenance_confidence_bps INTEGER NOT NULL DEFAULT 0 CHECK (
    maintenance_confidence_bps BETWEEN 0 AND 10000
  ),
  margin_signal TEXT,
  warnings TEXT NOT NULL DEFAULT '[]',
  evidence_json TEXT NOT NULL DEFAULT '{}',
  owner_id TEXT NOT NULL DEFAULT 'local-owner',
  row_version INTEGER NOT NULL DEFAULT 1,
  last_local_seq INTEGER,
  stage_run_id TEXT,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  UNIQUE (product_candidate_id, landed_cost_estimate_id),
  CHECK (length(trim(id)) > 0),
  CHECK (length(trim(process_id)) > 0),
  CHECK (length(trim(notice_id)) > 0),
  CHECK (length(trim(product_candidate_id)) > 0),
  CHECK (length(trim(landed_cost_estimate_id)) > 0),
  CHECK (supplier_quote_id IS NULL OR length(trim(supplier_quote_id)) > 0),
  CHECK (visit_scope IS NULL OR length(trim(visit_scope)) > 0),
  CHECK (margin_signal IS NULL OR length(trim(margin_signal)) > 0),
  CHECK (length(trim(owner_id)) > 0),
  CHECK (row_version > 0),
  CHECK (stage_run_id IS NULL OR length(trim(stage_run_id)) > 0)
);

CREATE INDEX IF NOT EXISTS idx_maintenance_period_estimates_candidate
  ON maintenance_period_estimates(product_candidate_id);

CREATE INDEX IF NOT EXISTS idx_maintenance_period_estimates_landed_cost
  ON maintenance_period_estimates(landed_cost_estimate_id);
