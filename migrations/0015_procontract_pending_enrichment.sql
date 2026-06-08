PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS procontract_pending_enrichment (
  id TEXT PRIMARY KEY,
  process_id TEXT NOT NULL,
  notice_id TEXT NOT NULL,
  advert_id TEXT NOT NULL,
  source_notice_id TEXT,
  state TEXT NOT NULL CHECK (
    state IN ('awaiting_access', 'enriched', 'expired')
  ),
  interest_registered_at TEXT NOT NULL,
  deadline_at TEXT,
  observation_count INTEGER NOT NULL DEFAULT 0 CHECK (
    observation_count >= 0
  ),
  last_checked_at TEXT,
  enriched_at TEXT,
  expiry_reason TEXT,
  owner_id TEXT NOT NULL DEFAULT 'local-owner',
  row_version INTEGER NOT NULL DEFAULT 1 CHECK (row_version > 0),
  last_local_seq INTEGER,
  stage_run_id TEXT,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  UNIQUE (process_id, notice_id, advert_id),
  CHECK (length(trim(id)) > 0),
  CHECK (length(trim(process_id)) > 0),
  CHECK (length(trim(notice_id)) > 0),
  CHECK (length(trim(advert_id)) > 0),
  CHECK (source_notice_id IS NULL OR length(trim(source_notice_id)) > 0),
  CHECK (length(trim(interest_registered_at)) > 0),
  CHECK (deadline_at IS NULL OR length(trim(deadline_at)) > 0),
  CHECK (last_checked_at IS NULL OR length(trim(last_checked_at)) > 0),
  CHECK (enriched_at IS NULL OR length(trim(enriched_at)) > 0),
  CHECK (expiry_reason IS NULL OR length(trim(expiry_reason)) > 0),
  CHECK (length(trim(owner_id)) > 0),
  CHECK (stage_run_id IS NULL OR length(trim(stage_run_id)) > 0)
);

CREATE INDEX IF NOT EXISTS idx_procontract_pending_enrichment_state
  ON procontract_pending_enrichment(state, updated_at);

CREATE INDEX IF NOT EXISTS idx_procontract_pending_enrichment_advert
  ON procontract_pending_enrichment(advert_id);
