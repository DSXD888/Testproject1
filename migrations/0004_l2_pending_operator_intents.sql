PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS pending_operator_intents (
  id TEXT PRIMARY KEY,
  process_id TEXT NOT NULL REFERENCES procurement_processes(id),
  desired_disposition TEXT NOT NULL,
  decision_note TEXT,
  reviewed_basis_hash TEXT NOT NULL,
  reviewed_manifest_hash TEXT NOT NULL,
  reviewed_at_stage_run_id TEXT NOT NULL,
  reviewed_at TEXT NOT NULL,
  base_remote_revision INTEGER NOT NULL DEFAULT 0,
  idempotency_key TEXT NOT NULL UNIQUE,
  status TEXT NOT NULL DEFAULT 'queued',
  superseded_by_intent_id TEXT REFERENCES pending_operator_intents(id),
  queued_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  last_attempt_at TEXT,
  last_error TEXT,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CHECK (desired_disposition IN ('approve', 'reject', 'watch'))
);

CREATE TABLE IF NOT EXISTS process_aliases (
  alias_process_id TEXT PRIMARY KEY,
  canonical_process_id TEXT NOT NULL REFERENCES procurement_processes(id),
  alias_reason TEXT NOT NULL DEFAULT 'merged_same_process',
  owner_id TEXT NOT NULL,
  row_version INTEGER NOT NULL DEFAULT 1,
  last_local_seq INTEGER,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_process_aliases_canonical
  ON process_aliases(canonical_process_id);

CREATE INDEX IF NOT EXISTS idx_pending_operator_intents_latest
  ON pending_operator_intents(process_id, superseded_by_intent_id, queued_at DESC);
