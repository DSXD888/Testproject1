CREATE TABLE IF NOT EXISTS candidate_quote_coverage (
  id TEXT PRIMARY KEY,
  process_id TEXT NOT NULL REFERENCES procurement_processes(id),
  notice_id TEXT NOT NULL REFERENCES contract_notices(id),
  product_candidate_id TEXT NOT NULL REFERENCES product_candidates(id),
  required_role TEXT NOT NULL,
  attempted INTEGER NOT NULL DEFAULT 0,
  usable_quote_count INTEGER NOT NULL DEFAULT 0,
  latest_source TEXT,
  latest_failure_mode TEXT,
  coverage_state TEXT NOT NULL,
  evidence_json TEXT NOT NULL DEFAULT '{}',
  owner_id TEXT NOT NULL,
  row_version INTEGER NOT NULL DEFAULT 1,
  last_local_seq INTEGER,
  stage_run_id TEXT,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  UNIQUE(product_candidate_id, required_role)
);

CREATE INDEX IF NOT EXISTS idx_candidate_quote_coverage_candidate
  ON candidate_quote_coverage(product_candidate_id);

ALTER TABLE candidate_pricing_assessments
  ADD COLUMN commercial_basis_kind TEXT NOT NULL DEFAULT 'legacy_single_quote';

ALTER TABLE candidate_pricing_assessments
  ADD COLUMN normal_quote_coverage INTEGER NOT NULL DEFAULT 0;

ALTER TABLE candidate_pricing_assessments
  ADD COLUMN uk_quote_id TEXT REFERENCES supplier_quotes(id);

ALTER TABLE candidate_pricing_assessments
  ADD COLUMN china_import_quote_id TEXT REFERENCES supplier_quotes(id);

ALTER TABLE candidate_pricing_assessments
  ADD COLUMN benchmark_quote_id TEXT REFERENCES supplier_quotes(id);

ALTER TABLE candidate_pricing_assessments
  ADD COLUMN selected_supply_quote_id TEXT REFERENCES supplier_quotes(id);

ALTER TABLE candidate_pricing_assessments
  ADD COLUMN supplier_spread_bps INTEGER;

ALTER TABLE candidate_pricing_assessments
  ADD COLUMN fallback_discount_bps INTEGER;

ALTER TABLE candidate_pricing_assessments
  ADD COLUMN missing_required_roles_json TEXT NOT NULL DEFAULT '[]';

ALTER TABLE candidate_pricing_assessments
  ADD COLUMN coverage_evidence_json TEXT NOT NULL DEFAULT '{}';
