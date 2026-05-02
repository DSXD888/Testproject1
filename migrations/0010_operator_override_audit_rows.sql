ALTER TABLE operator_input_overrides RENAME TO operator_input_overrides_legacy;

CREATE TABLE operator_input_overrides (
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
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT INTO operator_input_overrides (
  id,
  process_id,
  product_candidate_id,
  reason_code,
  payload_json,
  notes,
  basis_hash_when_provided,
  provided_at,
  provided_by,
  withdrawn_at,
  superseded_by_override_id,
  owner_id,
  row_version,
  last_local_seq,
  stage_run_id,
  created_at,
  updated_at
)
SELECT
  id,
  process_id,
  product_candidate_id,
  reason_code,
  payload_json,
  notes,
  basis_hash_when_provided,
  provided_at,
  provided_by,
  withdrawn_at,
  superseded_by_override_id,
  owner_id,
  row_version,
  last_local_seq,
  stage_run_id,
  created_at,
  updated_at
FROM operator_input_overrides_legacy;

DROP TABLE operator_input_overrides_legacy;

CREATE INDEX IF NOT EXISTS idx_oio_active
  ON operator_input_overrides(process_id, product_candidate_id, reason_code)
  WHERE withdrawn_at IS NULL AND superseded_by_override_id IS NULL;

CREATE INDEX IF NOT EXISTS idx_oio_process ON operator_input_overrides(process_id);
