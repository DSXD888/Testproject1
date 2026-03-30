PRAGMA foreign_keys = ON;

ALTER TABLE source_runtime_state
  ADD COLUMN circuit_state TEXT NOT NULL DEFAULT 'closed'
  CHECK (circuit_state IN ('closed', 'half_open', 'open'));

ALTER TABLE source_runtime_state
  ADD COLUMN last_canary_result TEXT
  CHECK (
    last_canary_result IS NULL
    OR last_canary_result IN ('passed', 'failed', 'timed_out')
  );

CREATE TABLE IF NOT EXISTS validation_verdicts (
  id TEXT PRIMARY KEY,
  worker_job_id TEXT NOT NULL REFERENCES workflow_jobs(id),
  worker_session_id TEXT REFERENCES agent_session_logs(id),
  validator_job_id TEXT NOT NULL REFERENCES workflow_jobs(id),
  validator_session_id TEXT REFERENCES agent_session_logs(id),
  validator_skill_path TEXT NOT NULL,
  validator_skill_version TEXT,
  verdict TEXT NOT NULL,
  confidence_bps INTEGER,
  summary TEXT NOT NULL,
  basis_sufficiency TEXT NOT NULL DEFAULT 'medium',
  anchor_conflict_count INTEGER NOT NULL DEFAULT 0,
  incident_signature TEXT,
  enforcement_eligible INTEGER NOT NULL DEFAULT 0,
  issues_json TEXT NOT NULL DEFAULT '[]',
  quality_assessment_json TEXT,
  skill_improvement_suggestions_json TEXT,
  run_manifest_hash TEXT NOT NULL,
  oqg_signals_summary TEXT,
  override_verdict TEXT,
  override_reason TEXT,
  override_by TEXT,
  override_at TEXT,
  verdict_rendered_at TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS listing_evidence (
  id TEXT PRIMARY KEY,
  source TEXT NOT NULL,
  source_listing_id TEXT NOT NULL,
  normalized_title TEXT NOT NULL,
  structured_attributes_json TEXT,
  parsed_standards_json TEXT,
  parsed_units_json TEXT,
  seller_identifier TEXT,
  minimum_order_quantity NUMERIC,
  shipping_terms TEXT,
  condition TEXT CHECK (
    condition IS NULL
    OR condition IN ('new', 'used', 'refurbished', 'unknown')
  ),
  compliance_evidence_tier TEXT NOT NULL CHECK (
    compliance_evidence_tier IN ('structured_attribute', 'title_body_mention', 'absent')
  ),
  intended_use_language TEXT,
  is_ready_to_ship INTEGER CHECK (
    is_ready_to_ship IS NULL
    OR is_ready_to_ship IN (0, 1)
  ),
  fetched_at TEXT NOT NULL,
  detail_page_fetched INTEGER NOT NULL DEFAULT 0 CHECK (detail_page_fetched IN (0, 1)),
  trust_state TEXT NOT NULL DEFAULT 'unverified' CHECK (
    trust_state IN ('verified', 'flagged', 'unverified', 'suspect')
  ),
  source_job_id TEXT REFERENCES workflow_jobs(id),
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  UNIQUE (source, source_listing_id)
);

CREATE TABLE IF NOT EXISTS process_relations (
  id TEXT PRIMARY KEY,
  process_id_a TEXT NOT NULL REFERENCES procurement_processes(id),
  process_id_b TEXT NOT NULL REFERENCES procurement_processes(id),
  relation_type TEXT NOT NULL CHECK (
    relation_type IN (
      'merged_same_process',
      'related_not_merged',
      'possible_duplicate'
    )
  ),
  confidence_bps INTEGER NOT NULL CHECK (confidence_bps BETWEEN 0 AND 10000),
  evidence TEXT,
  owner_id TEXT NOT NULL,
  row_version INTEGER NOT NULL DEFAULT 1,
  last_local_seq INTEGER,
  stage_run_id TEXT,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CHECK (process_id_a <> process_id_b)
);

CREATE TABLE IF NOT EXISTS process_aliases (
  alias_process_id TEXT PRIMARY KEY,
  canonical_process_id TEXT NOT NULL REFERENCES procurement_processes(id),
  alias_reason TEXT NOT NULL DEFAULT 'merged_same_process',
  owner_id TEXT NOT NULL,
  row_version INTEGER NOT NULL DEFAULT 1,
  last_local_seq INTEGER,
  stage_run_id TEXT,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_validation_verdicts_worker_job
  ON validation_verdicts(worker_job_id);

CREATE INDEX IF NOT EXISTS idx_process_relations_a
  ON process_relations(process_id_a);

CREATE INDEX IF NOT EXISTS idx_process_relations_b
  ON process_relations(process_id_b);

CREATE INDEX IF NOT EXISTS idx_process_aliases_canonical
  ON process_aliases(canonical_process_id);
