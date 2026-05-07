PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS local_outbox (
  local_seq INTEGER PRIMARY KEY AUTOINCREMENT,
  executor_id TEXT NOT NULL,
  entity_kind TEXT NOT NULL,
  entity_id TEXT NOT NULL,
  op TEXT NOT NULL CHECK (op IN ('insert', 'update')),
  row_version INTEGER NOT NULL CHECK (row_version > 0),
  stage_run_id TEXT NOT NULL,
  schema_epoch INTEGER NOT NULL CHECK (schema_epoch > 0),
  commit_group_id TEXT,
  writer_epoch INTEGER,
  payload_json TEXT NOT NULL,
  content_hash TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CHECK (length(trim(entity_kind)) > 0),
  CHECK (length(trim(entity_id)) > 0),
  CHECK (length(trim(stage_run_id)) > 0),
  CHECK (commit_group_id IS NOT NULL AND length(trim(commit_group_id)) > 0),
  CHECK (writer_epoch IS NULL OR writer_epoch >= 0),
  CHECK (length(trim(content_hash)) > 0)
);

CREATE TABLE IF NOT EXISTS sync_batches (
  batch_id TEXT PRIMARY KEY,
  executor_id TEXT NOT NULL,
  stream_id TEXT NOT NULL,
  status TEXT NOT NULL CHECK (
    status IN ('in_flight', 'retry', 'acknowledged', 'rejected')
  ),
  first_local_seq INTEGER NOT NULL,
  last_local_seq INTEGER NOT NULL,
  schema_epoch INTEGER NOT NULL CHECK (schema_epoch > 0),
  stage_run_id TEXT NOT NULL,
  commit_group_ids_json TEXT NOT NULL DEFAULT '[]',
  writer_epoch INTEGER NOT NULL CHECK (writer_epoch >= 0),
  payload_json TEXT NOT NULL,
  payload_hash TEXT NOT NULL,
  retry_count INTEGER NOT NULL DEFAULT 0 CHECK (retry_count >= 0),
  last_attempt_at TEXT,
  acknowledged_at TEXT,
  rejected_at TEXT,
  response_json TEXT,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CHECK (length(trim(batch_id)) > 0),
  CHECK (length(trim(executor_id)) > 0),
  CHECK (length(trim(stream_id)) > 0),
  CHECK (first_local_seq > 0),
  CHECK (last_local_seq >= first_local_seq),
  CHECK (length(trim(stage_run_id)) > 0),
  CHECK (length(trim(payload_hash)) > 0)
);

CREATE TABLE IF NOT EXISTS pull_cursors (
  domain TEXT PRIMARY KEY CHECK (
    domain IN (
      'shared_source_configs',
      'shared_company_profile',
      'operator_decision_heads'
    )
  ),
  last_remote_change_seq INTEGER NOT NULL DEFAULT 0 CHECK (last_remote_change_seq >= 0),
  last_pulled_at TEXT,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT OR IGNORE INTO pull_cursors (domain)
VALUES
  ('shared_source_configs'),
  ('shared_company_profile'),
  ('operator_decision_heads');

CREATE TABLE IF NOT EXISTS provisional_worker_outputs (
  id TEXT PRIMARY KEY,
  worker_job_id TEXT NOT NULL REFERENCES workflow_jobs(id),
  payload_json TEXT NOT NULL,
  result_contract_blob_path TEXT,
  validation_status TEXT NOT NULL DEFAULT 'pending' CHECK (
    validation_status IN ('pending', 'approved', 'rejected')
  ),
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CHECK (length(trim(id)) > 0),
  CHECK (length(trim(worker_job_id)) > 0)
);

CREATE TABLE IF NOT EXISTS enforcement_readiness (
  slice_id TEXT PRIMARY KEY,
  worker_skill_id TEXT NOT NULL,
  source_id TEXT NOT NULL,
  worker_behavior_version TEXT NOT NULL,
  validator_overlay_version TEXT NOT NULL,
  enforcement_state TEXT NOT NULL DEFAULT 'dry_run' CHECK (
    enforcement_state IN ('eligible', 'dry_run', 'demoted')
  ),
  last_evaluated_at TEXT,
  wilson_lower_bound_bps INTEGER NOT NULL DEFAULT 0 CHECK (
    wilson_lower_bound_bps BETWEEN 0 AND 10000
  ),
  false_pass_upper_bound_bps INTEGER NOT NULL DEFAULT 10000 CHECK (
    false_pass_upper_bound_bps BETWEEN 0 AND 10000
  ),
  adjudicated_count INTEGER NOT NULL DEFAULT 0 CHECK (adjudicated_count >= 0),
  audited_approvals_count INTEGER NOT NULL DEFAULT 0 CHECK (audited_approvals_count >= 0),
  dominant_failure_families_json TEXT NOT NULL DEFAULT '[]',
  missing_failure_families_json TEXT NOT NULL DEFAULT '[]',
  demotion_reason TEXT,
  demotion_thresholds_json TEXT NOT NULL DEFAULT '{}',
  active_demotion_trigger_json TEXT,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  UNIQUE (
    worker_skill_id,
    source_id,
    worker_behavior_version,
    validator_overlay_version
  ),
  CHECK (length(trim(slice_id)) > 0),
  CHECK (length(trim(worker_skill_id)) > 0),
  CHECK (length(trim(source_id)) > 0),
  CHECK (length(trim(worker_behavior_version)) > 0),
  CHECK (length(trim(validator_overlay_version)) > 0)
);

CREATE TABLE IF NOT EXISTS projection_generations (
  generation_id INTEGER PRIMARY KEY,
  schema_epoch INTEGER NOT NULL CHECK (schema_epoch > 0),
  status TEXT NOT NULL CHECK (status IN ('started', 'published', 'failed')),
  started_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  completed_at TEXT,
  max_local_seq_before_reseed INTEGER NOT NULL DEFAULT 0 CHECK (
    max_local_seq_before_reseed >= 0
  ),
  remote_acked_at TEXT,
  error_json TEXT
);

CREATE TABLE IF NOT EXISTS validation_calibration_cases (
  case_id TEXT PRIMARY KEY,
  slice_id TEXT NOT NULL,
  corpus_kind TEXT NOT NULL CHECK (
    corpus_kind IN (
      'clean',
      'weird',
      'mutated',
      'pairwise',
      'live',
      'override',
      'cross_model'
    )
  ),
  source_artifact_path TEXT,
  expected_verdict TEXT CHECK (
    expected_verdict IS NULL
    OR expected_verdict IN ('approved', 'rejected', 'needs_review')
  ),
  actual_verdict TEXT CHECK (
    actual_verdict IS NULL
    OR actual_verdict IN ('approved', 'rejected', 'needs_review')
  ),
  ground_truth_label TEXT,
  mutation_family TEXT,
  pair_id TEXT,
  model_family TEXT,
  worker_job_id TEXT REFERENCES workflow_jobs(id),
  validation_verdict_id TEXT REFERENCES validation_verdicts(id),
  metadata_json TEXT NOT NULL DEFAULT '{}',
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  adjudicated_at TEXT,
  CHECK (length(trim(case_id)) > 0),
  CHECK (length(trim(slice_id)) > 0)
);

CREATE TABLE IF NOT EXISTS validation_incident_clusters (
  incident_signature TEXT PRIMARY KEY,
  dominant_failure_family TEXT,
  affected_slice_ids_json TEXT NOT NULL DEFAULT '[]',
  candidate_mutation_template_json TEXT NOT NULL DEFAULT '{}',
  override_case_ids_json TEXT NOT NULL DEFAULT '[]',
  calibration_case_count INTEGER NOT NULL DEFAULT 0 CHECK (calibration_case_count >= 0),
  override_count INTEGER NOT NULL DEFAULT 0 CHECK (override_count >= 0),
  first_seen_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  last_seen_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CHECK (length(trim(incident_signature)) > 0)
);

CREATE INDEX IF NOT EXISTS idx_local_outbox_dispatch
  ON local_outbox(executor_id, schema_epoch, stage_run_id, commit_group_id, local_seq);

CREATE INDEX IF NOT EXISTS idx_local_outbox_entity
  ON local_outbox(entity_kind, entity_id, row_version);

CREATE INDEX IF NOT EXISTS idx_sync_batches_in_flight
  ON sync_batches(stream_id, status, first_local_seq)
  WHERE status IN ('in_flight', 'retry');

CREATE INDEX IF NOT EXISTS idx_provisional_worker_outputs_worker_job
  ON provisional_worker_outputs(worker_job_id);

CREATE UNIQUE INDEX IF NOT EXISTS idx_enforcement_readiness_slice
  ON enforcement_readiness(slice_id);

CREATE INDEX IF NOT EXISTS idx_validation_calibration_cases_slice
  ON validation_calibration_cases(slice_id, corpus_kind);

CREATE INDEX IF NOT EXISTS idx_validation_calibration_cases_pair
  ON validation_calibration_cases(pair_id)
  WHERE pair_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_validation_incident_clusters_family
  ON validation_incident_clusters(dominant_failure_family);
