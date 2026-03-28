PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS pending_operator_intents (
  id TEXT PRIMARY KEY,
  process_id TEXT NOT NULL REFERENCES procurement_processes(id),
  desired_disposition TEXT NOT NULL,
  reviewed_basis_hash TEXT NOT NULL,
  reviewed_manifest_hash TEXT NOT NULL,
  reviewed_at_stage_run_id TEXT NOT NULL,
  reviewed_at TEXT NOT NULL,
  base_remote_revision INTEGER NOT NULL DEFAULT 0,
  idempotency_key TEXT NOT NULL UNIQUE,
  status TEXT NOT NULL DEFAULT 'queued',
  superseded_by_intent_id TEXT REFERENCES pending_operator_intents(id),
  queued_at TEXT NOT NULL,
  decision_note TEXT,
  last_attempt_at TEXT,
  last_error TEXT,
  CHECK (desired_disposition IN ('approve', 'reject', 'watch'))
);

CREATE TABLE IF NOT EXISTS pending_operator_intent_receipts (
  idempotency_key TEXT PRIMARY KEY,
  intent_id TEXT NOT NULL REFERENCES pending_operator_intents(id),
  visible_after_seq INTEGER NOT NULL,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_pending_operator_intents_latest
  ON pending_operator_intents(process_id, queued_at DESC, id DESC)
  WHERE superseded_by_intent_id IS NULL;
