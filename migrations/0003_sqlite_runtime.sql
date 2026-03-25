PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS agent_session_logs (
  id TEXT PRIMARY KEY,
  job_id TEXT REFERENCES workflow_jobs(id),
  skill_path TEXT NOT NULL,
  skill_version TEXT,
  execution_mode TEXT NOT NULL,
  cli_vendor TEXT,
  cli_version TEXT,
  adapter_version TEXT,
  config_hash TEXT,
  pty_session_id TEXT,
  pid INTEGER,
  pgid INTEGER,
  started_at TEXT,
  completed_at TEXT,
  status TEXT NOT NULL,
  terminal_output_path TEXT,
  terminal_output_hash TEXT,
  terminal_output_bytes INTEGER,
  terminal_output_tail TEXT,
  structured_trace TEXT,
  error_type TEXT,
  error_message TEXT,
  cleanup_verdict TEXT,
  run_manifest_path TEXT,
  run_manifest_hash TEXT,
  last_checkpoint_seq INTEGER,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS raw_documents (
  id TEXT PRIMARY KEY,
  source_id TEXT NOT NULL REFERENCES sources(id),
  fetched_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  content_type TEXT,
  source_url TEXT,
  payload_json TEXT,
  storage_path TEXT,
  metadata TEXT NOT NULL DEFAULT '{}',
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CHECK (payload_json IS NOT NULL OR storage_path IS NOT NULL)
);

CREATE TABLE IF NOT EXISTS error_manifests (
  id TEXT PRIMARY KEY,
  job_id TEXT REFERENCES workflow_jobs(id),
  process_id TEXT REFERENCES procurement_processes(id),
  pipeline_stage TEXT NOT NULL,
  workflow_step TEXT NOT NULL,
  error_type TEXT NOT NULL,
  error_message TEXT NOT NULL,
  error_context TEXT NOT NULL,
  skill_path TEXT,
  agent_session_id TEXT REFERENCES agent_session_logs(id),
  markdown_path TEXT,
  resolved INTEGER NOT NULL DEFAULT 0,
  resolved_at TEXT,
  resolved_by TEXT,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS executors (
  id TEXT PRIMARY KEY,
  device_name TEXT NOT NULL,
  status TEXT NOT NULL,
  last_heartbeat_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  owner_id TEXT NOT NULL,
  row_version INTEGER NOT NULL DEFAULT 1,
  last_local_seq INTEGER,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS source_cursors (
  source_id TEXT NOT NULL REFERENCES sources(id),
  cursor_key TEXT NOT NULL,
  cursor_value TEXT,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  PRIMARY KEY (source_id, cursor_key)
);

CREATE TABLE IF NOT EXISTS commit_clock (
  id INTEGER PRIMARY KEY CHECK (id = 1),
  commit_seq INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_workflow_jobs_status_scheduled
  ON workflow_jobs(status, scheduled_for)
  WHERE status IN ('pending', 'retry');

CREATE INDEX IF NOT EXISTS idx_workflow_jobs_process
  ON workflow_jobs(process_id);

CREATE INDEX IF NOT EXISTS idx_process_stage_status_process
  ON process_stage_status(process_id, stage, scope_kind);

CREATE INDEX IF NOT EXISTS idx_process_stage_status_run
  ON process_stage_status(stage_run_id);

CREATE INDEX IF NOT EXISTS idx_agent_session_logs_job
  ON agent_session_logs(job_id);

CREATE INDEX IF NOT EXISTS idx_error_manifests_process
  ON error_manifests(process_id);

CREATE INDEX IF NOT EXISTS idx_error_manifests_unresolved
  ON error_manifests(resolved, created_at)
  WHERE resolved = 0;

CREATE INDEX IF NOT EXISTS idx_source_cursors_source
  ON source_cursors(source_id);
