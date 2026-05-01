CREATE TABLE IF NOT EXISTS candidate_net_margin_assessments (
  id TEXT PRIMARY KEY,
  process_id TEXT NOT NULL REFERENCES procurement_processes(id),
  notice_id TEXT NOT NULL REFERENCES contract_notices(id),
  lot_id TEXT REFERENCES notice_lots(id),
  product_candidate_id TEXT NOT NULL REFERENCES product_candidates(id),
  pricing_assessment_id TEXT REFERENCES candidate_pricing_assessments(id),
  landed_cost_estimate_id TEXT REFERENCES landed_cost_estimates(id),

  verdict TEXT NOT NULL,
  reasons_json TEXT NOT NULL DEFAULT '[]',
  re_evaluation_eligible INTEGER NOT NULL DEFAULT 0,

  net_margin_low_bps INTEGER,
  net_margin_base_bps INTEGER,
  net_margin_high_bps INTEGER,

  revenue_used_pence INTEGER,
  revenue_basis_type TEXT,
  economic_landed_cost_used_pence INTEGER,

  revenue_confidence_bps INTEGER,
  pricing_input_confidence_bps INTEGER,
  customs_resolution_confidence_bps INTEGER,
  commercial_estimate_confidence_bps INTEGER,

  pass_threshold_bps INTEGER NOT NULL,
  reject_threshold_bps INTEGER NOT NULL,
  thresholds_version TEXT NOT NULL,
  basis_hash TEXT NOT NULL,

  stage_run_id TEXT NOT NULL,
  owner_id TEXT NOT NULL,
  row_version INTEGER NOT NULL DEFAULT 1,
  last_local_seq INTEGER,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_cnma_process ON candidate_net_margin_assessments(process_id);
CREATE INDEX IF NOT EXISTS idx_cnma_verdict ON candidate_net_margin_assessments(verdict);
CREATE INDEX IF NOT EXISTS idx_cnma_basis ON candidate_net_margin_assessments(basis_hash);
