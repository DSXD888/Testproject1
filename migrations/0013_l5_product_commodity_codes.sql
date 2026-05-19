PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS product_commodity_codes (
  id TEXT PRIMARY KEY,
  product_candidate_id TEXT NOT NULL REFERENCES product_candidates(id),
  commodity_code TEXT NOT NULL,
  digits INTEGER NOT NULL CHECK (digits IN (6, 8, 10)),
  source TEXT NOT NULL CHECK (
    source IN (
      'hmrc_fpo_or_search_suggestion',
      'cpv_chapter_prior',
      'rule_based_chapter_hint',
      'hmrc_tariff_keyword_search',
      'atr_codebook_reuse'
    )
  ),
  score_bps INTEGER NOT NULL CHECK (score_bps BETWEEN 0 AND 10000),
  rank_ordinal INTEGER,
  description TEXT,
  evidence_json TEXT NOT NULL DEFAULT '{}',
  owner_id TEXT NOT NULL,
  row_version INTEGER NOT NULL DEFAULT 1,
  last_local_seq INTEGER,
  stage_run_id TEXT,
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  updated_at TEXT NOT NULL DEFAULT (datetime('now')),
  UNIQUE (product_candidate_id, commodity_code, source),
  CHECK (length(trim(id)) > 0),
  CHECK (length(trim(product_candidate_id)) > 0),
  CHECK (length(trim(commodity_code)) > 0),
  CHECK (length(trim(source)) > 0),
  CHECK (row_version > 0),
  CHECK (stage_run_id IS NULL OR length(trim(stage_run_id)) > 0)
);

CREATE INDEX IF NOT EXISTS idx_product_commodity_codes_candidate_stage
  ON product_commodity_codes(product_candidate_id, stage_run_id);

CREATE INDEX IF NOT EXISTS idx_product_commodity_codes_code_source
  ON product_commodity_codes(commodity_code, source);
