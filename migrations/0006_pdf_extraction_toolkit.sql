PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS document_extractions (
  id TEXT PRIMARY KEY,
  raw_document_id TEXT NOT NULL,
  process_id TEXT NOT NULL REFERENCES procurement_processes(id),
  notice_id TEXT REFERENCES contract_notices(id),
  document_set_id TEXT REFERENCES tender_document_sets(id),
  source_content_hash TEXT NOT NULL,
  extraction_engine TEXT NOT NULL,
  extraction_engine_version TEXT NOT NULL,
  processing_strategy TEXT NOT NULL,
  is_current INTEGER NOT NULL DEFAULT 1 CHECK (is_current IN (0, 1)),
  superseded_by_extraction_id TEXT REFERENCES document_extractions(id),
  page_count INTEGER NOT NULL,
  document_title TEXT,
  document_language TEXT,
  document_role TEXT,
  extraction_policy_version TEXT NOT NULL,
  expected_overlays_json TEXT NOT NULL DEFAULT '[]',
  extracted_markdown TEXT NOT NULL,
  extracted_sections_json TEXT NOT NULL,
  extracted_tables_json TEXT NOT NULL,
  extracted_metadata_json TEXT NOT NULL,
  table_count INTEGER NOT NULL,
  section_count INTEGER NOT NULL,
  image_count INTEGER NOT NULL DEFAULT 0,
  has_ocr_content INTEGER NOT NULL DEFAULT 0,
  empty_page_count INTEGER NOT NULL DEFAULT 0,
  quality_warnings_json TEXT,
  semantic_extraction_state TEXT NOT NULL DEFAULT 'pending',
  owner_id TEXT NOT NULL,
  row_version INTEGER NOT NULL DEFAULT 1,
  last_local_seq INTEGER,
  stage_run_id TEXT,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_doc_extraction_current
  ON document_extractions(raw_document_id, extraction_engine, extraction_engine_version, processing_strategy)
  WHERE is_current = 1;

CREATE INDEX IF NOT EXISTS idx_doc_extraction_hash_lookup
  ON document_extractions(source_content_hash, extraction_engine, extraction_engine_version, processing_strategy);

CREATE INDEX IF NOT EXISTS idx_doc_extraction_process
  ON document_extractions(process_id);

CREATE INDEX IF NOT EXISTS idx_doc_extraction_semantic_state
  ON document_extractions(semantic_extraction_state)
  WHERE semantic_extraction_state IN ('pending', 'failed');

CREATE TABLE IF NOT EXISTS document_semantic_extractions (
  id TEXT PRIMARY KEY,
  document_extraction_id TEXT NOT NULL REFERENCES document_extractions(id),
  process_id TEXT NOT NULL REFERENCES procurement_processes(id),
  skill_id TEXT NOT NULL,
  skill_version TEXT NOT NULL,
  job_id TEXT NOT NULL,
  session_id TEXT,
  extraction_scope TEXT NOT NULL,
  target_domain TEXT NOT NULL,
  is_current INTEGER NOT NULL DEFAULT 1 CHECK (is_current IN (0, 1)),
  superseded_by_semantic_extraction_id TEXT REFERENCES document_semantic_extractions(id),
  extracted_data_json TEXT NOT NULL,
  field_confidences_json TEXT NOT NULL,
  extraction_confidence_bps INTEGER NOT NULL,
  warnings_json TEXT,
  evidence_anchors_json TEXT,
  written_entity_refs_json TEXT,
  owner_id TEXT NOT NULL,
  row_version INTEGER NOT NULL DEFAULT 1,
  last_local_seq INTEGER,
  stage_run_id TEXT,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_doc_semantic_extraction
  ON document_semantic_extractions(document_extraction_id);

CREATE INDEX IF NOT EXISTS idx_doc_semantic_process
  ON document_semantic_extractions(process_id);

CREATE UNIQUE INDEX IF NOT EXISTS idx_doc_semantic_current
  ON document_semantic_extractions(document_extraction_id, skill_id, extraction_scope)
  WHERE is_current = 1;

ALTER TABLE tender_document_sets
  ADD COLUMN extraction_state TEXT NOT NULL DEFAULT 'none';
