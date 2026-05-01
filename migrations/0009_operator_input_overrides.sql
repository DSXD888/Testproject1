CREATE TABLE IF NOT EXISTS operator_input_overrides (
  id TEXT PRIMARY KEY,
  process_id TEXT NOT NULL REFERENCES procurement_processes(id),
  product_candidate_id TEXT NOT NULL REFERENCES product_candidates(id),
  reason_code TEXT NOT NULL,
  payload_json TEXT NOT NULL,
  notes TEXT,
  basis_hash_when_provided TEXT,
  provided_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  provided_by TEXT NOT NULL DEFAULT 'local-operator',
  withdrawn_at TEXT,
  superseded_by_override_id TEXT REFERENCES operator_input_overrides(id),
  owner_id TEXT NOT NULL,
  row_version INTEGER NOT NULL DEFAULT 1,
  last_local_seq INTEGER,
  stage_run_id TEXT,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  UNIQUE(process_id, product_candidate_id, reason_code)
);

CREATE INDEX IF NOT EXISTS idx_oio_active
  ON operator_input_overrides(process_id, product_candidate_id, reason_code)
  WHERE withdrawn_at IS NULL AND superseded_by_override_id IS NULL;

CREATE INDEX IF NOT EXISTS idx_oio_process ON operator_input_overrides(process_id);
