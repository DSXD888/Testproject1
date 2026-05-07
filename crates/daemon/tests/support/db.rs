#![forbid(unsafe_code)]
#![allow(dead_code)]

#[allow(dead_code)]
#[allow(clippy::missing_const_for_fn)]
#[path = "../../src/db/sqlite.rs"]
mod sqlite_impl;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Result, ensure};
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tempfile::TempDir;

#[allow(unused_imports)]
pub use sqlite_impl::{DEFAULT_SCHEMA_EPOCH, FTS_SOURCE_ID, StateDb};

const DEFAULT_DOCLING_VERSION: &str = "2.85.0";
const DEFAULT_OWNER_ID: &str = "local-owner";
const DEFAULT_PROCESSING_STRATEGY: &str = "standard";

pub type LogicalSchema = BTreeMap<String, Vec<LogicalColumn>>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogicalColumn {
    pub name: String,
    pub logical_type: String,
    pub nullable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentExtractionSeed<'a> {
    pub id: &'a str,
    pub raw_document_id: &'a str,
    pub process_id: &'a str,
    pub notice_id: Option<&'a str>,
    pub document_set_id: Option<&'a str>,
    pub source_content_hash: &'a str,
    pub document_role: Option<&'a str>,
    pub semantic_extraction_state: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentSemanticExtractionSeed<'a> {
    pub id: &'a str,
    pub document_extraction_id: &'a str,
    pub process_id: &'a str,
    pub skill_id: &'a str,
    pub skill_version: &'a str,
    pub job_id: &'a str,
    pub session_id: Option<&'a str>,
    pub extraction_scope: &'a str,
    pub target_domain: &'a str,
}

#[allow(dead_code)]
pub struct TestStateDb {
    _temp: TempDir,
    db_path: PathBuf,
    state_db: StateDb,
}

#[allow(dead_code)]
impl TestStateDb {
    pub fn new() -> Result<Self> {
        let temp = tempfile::tempdir()?;
        let db_path = temp.path().join("state.db");
        let state_db = sqlite_impl::open_state_db(&db_path, DEFAULT_SCHEMA_EPOCH)?;
        Ok(Self {
            _temp: temp,
            db_path,
            state_db,
        })
    }

    pub fn connection(&self) -> &Connection {
        self.state_db.connection()
    }

    pub fn db_path(&self) -> &Path {
        &self.db_path
    }
}

#[allow(dead_code)]
pub struct L4TestStateDb {
    inner: TestStateDb,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::struct_field_names)]
pub struct PushOwnedSeedIds {
    pub process_id: String,
    pub notice_id: String,
    pub lot_id: String,
}

#[allow(dead_code)]
impl L4TestStateDb {
    pub fn new() -> Result<Self> {
        let inner = TestStateDb::new()?;
        ensure!(
            inner.state_db.schema_epoch() == DEFAULT_SCHEMA_EPOCH,
            "L4 test DB opened at schema epoch {}, expected {DEFAULT_SCHEMA_EPOCH}",
            inner.state_db.schema_epoch()
        );
        Ok(Self { inner })
    }

    pub fn connection(&self) -> &Connection {
        self.inner.connection()
    }

    pub fn db_path(&self) -> &Path {
        self.inner.db_path()
    }

    pub fn seed_push_owned_rows(&self) -> Result<PushOwnedSeedIds> {
        let ids = PushOwnedSeedIds {
            process_id: "l4-process-fixture".to_owned(),
            notice_id: "l4-notice-fixture".to_owned(),
            lot_id: "l4-lot-fixture".to_owned(),
        };
        let conn = self.connection();

        conn.execute(
            "\
            INSERT INTO procurement_processes (
              id, current_stage, title, latest_published_at, latest_updated_at, owner_id,
              row_version, last_local_seq, stage_run_id
            ) VALUES (?1, 'qualification', 'L4 fixture process', '2026-05-06T00:00:00Z',
                      '2026-05-06T00:00:00Z', ?2, 1, 1, 'l4-stage-run')
            ON CONFLICT(id) DO NOTHING
            ",
            params![&ids.process_id, DEFAULT_OWNER_ID],
        )?;

        conn.execute(
            "\
            INSERT INTO contract_notices (
              id, process_id, source_id, source_notice_id, raw_document_id, title,
              procurement_stage, published_at, source_updated_at, normalized_payload,
              owner_id, row_version, last_local_seq, stage_run_id
            ) VALUES (?1, ?2, ?3, 'l4-source-notice-fixture', 'l4-raw-document-fixture',
                      'L4 fixture notice', 'tender', '2026-05-06T00:00:00Z',
                      '2026-05-06T00:00:00Z', ?4, ?5, 1, 2, 'l4-stage-run')
            ON CONFLICT(id) DO NOTHING
            ",
            params![
                &ids.notice_id,
                &ids.process_id,
                FTS_SOURCE_ID,
                json!({"fixture": "l4-push-owned"}).to_string(),
                DEFAULT_OWNER_ID
            ],
        )?;

        conn.execute(
            "\
            INSERT INTO notice_lots (
              id, notice_id, lot_number, title, estimated_value_pence, owner_id,
              row_version, last_local_seq, stage_run_id
            ) VALUES (?1, ?2, 1, 'L4 fixture lot', 120000, ?3, 1, 3, 'l4-stage-run')
            ON CONFLICT(id) DO NOTHING
            ",
            params![&ids.lot_id, &ids.notice_id, DEFAULT_OWNER_ID],
        )?;

        Ok(ids)
    }
}

pub fn seed_process(conn: &Connection, process_id: &str) -> Result<()> {
    conn.execute(
        "\
        INSERT INTO procurement_processes (
          id, current_stage, title, latest_published_at, latest_updated_at, owner_id
        ) VALUES (?1, 'qualification', 'Process title', '2026-03-25T00:00:00Z',
                  '2026-03-25T00:00:00Z', 'local-owner')
        ON CONFLICT(id) DO NOTHING
        ",
        params![process_id],
    )?;
    Ok(())
}

pub fn seed_raw_document(conn: &Connection, raw_document_id: &str) -> Result<()> {
    conn.execute(
        "\
        INSERT INTO raw_documents (
          id, source_id, content_type, storage_path, metadata
        ) VALUES (?1, ?2, 'application/pdf', ?3, ?4)
        ON CONFLICT(id) DO NOTHING
        ",
        params![
            raw_document_id,
            FTS_SOURCE_ID,
            format!("artifacts/raw/{raw_document_id}.pdf"),
            json!({"fixture": "pdf-support"}).to_string(),
        ],
    )?;
    Ok(())
}

pub fn assert_raw_document_present(conn: &Connection, raw_document_id: &str) -> Result<()> {
    let present: Option<String> = conn
        .query_row(
            "SELECT id FROM raw_documents WHERE id = ?1",
            [raw_document_id],
            |row| row.get(0),
        )
        .optional()?;
    ensure!(
        present.as_deref() == Some(raw_document_id),
        "expected raw_document `{raw_document_id}`"
    );
    Ok(())
}

pub fn seed_document_extraction(
    conn: &Connection,
    seed: &DocumentExtractionSeed<'_>,
) -> Result<()> {
    let docling_output = sample_docling_output();
    conn.execute(
        "\
        INSERT INTO document_extractions (
          id, raw_document_id, process_id, notice_id, document_set_id, source_content_hash,
          extraction_engine, extraction_engine_version, processing_strategy, is_current,
          page_count, document_role, extraction_policy_version, expected_overlays_json,
          extracted_markdown, extracted_sections_json, extracted_tables_json, extracted_metadata_json,
          table_count, section_count, semantic_extraction_state, owner_id
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'docling', ?7, ?8, 1, ?9, ?10, ?11, ?12, ?13, ?14,
                  ?15, ?16, ?17, ?18, ?19, 'local-owner')
        ",
        params![
            seed.id,
            seed.raw_document_id,
            seed.process_id,
            seed.notice_id,
            seed.document_set_id,
            seed.source_content_hash,
            DEFAULT_DOCLING_VERSION,
            DEFAULT_PROCESSING_STRATEGY,
            docling_output["page_count"].as_i64().unwrap_or(1),
            seed.document_role,
            "2026.04.12",
            json!([]).to_string(),
            docling_output["markdown"].as_str().unwrap_or_default().to_owned(),
            docling_output["sections"].to_string(),
            docling_output["tables"].to_string(),
            docling_output["metadata"].to_string(),
            docling_output["quality"]["table_count"].as_i64().unwrap_or(0),
            docling_output["quality"]["section_count"].as_i64().unwrap_or(0),
            seed.semantic_extraction_state,
        ],
    )?;
    Ok(())
}

pub fn assert_current_document_extraction(
    conn: &Connection,
    raw_document_id: &str,
    expected_extraction_id: &str,
) -> Result<()> {
    let current_id: Option<String> = conn
        .query_row(
            "\
            SELECT id
            FROM document_extractions
            WHERE raw_document_id = ?1
              AND extraction_engine = 'docling'
              AND extraction_engine_version = ?2
              AND processing_strategy = ?3
              AND is_current = 1
            ",
            params![
                raw_document_id,
                DEFAULT_DOCLING_VERSION,
                DEFAULT_PROCESSING_STRATEGY
            ],
            |row| row.get(0),
        )
        .optional()?;
    ensure!(
        current_id.as_deref() == Some(expected_extraction_id),
        "expected current document_extraction `{expected_extraction_id}` for `{raw_document_id}`"
    );
    Ok(())
}

pub fn seed_document_semantic_extraction(
    conn: &Connection,
    seed: &DocumentSemanticExtractionSeed<'_>,
) -> Result<()> {
    conn.execute(
        "\
        INSERT INTO document_semantic_extractions (
          id, document_extraction_id, process_id, skill_id, skill_version, job_id, session_id,
          extraction_scope, target_domain, is_current, extracted_data_json, field_confidences_json,
          extraction_confidence_bps, warnings_json, evidence_anchors_json, written_entity_refs_json,
          owner_id
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 1, ?10, ?11, ?12, ?13, ?14, ?15,
                  'local-owner')
        ",
        params![
            seed.id,
            seed.document_extraction_id,
            seed.process_id,
            seed.skill_id,
            seed.skill_version,
            seed.job_id,
            seed.session_id,
            seed.extraction_scope,
            seed.target_domain,
            sample_semantic_result(seed.skill_id).to_string(),
            json!({"root": 9100}).to_string(),
            9100,
            json!([]).to_string(),
            json!([
                {
                    "page_ref": "p1",
                    "anchor_text": "fixture-anchor"
                }
            ])
            .to_string(),
            json!([]).to_string(),
        ],
    )?;
    Ok(())
}

pub fn assert_current_document_semantic_extraction(
    conn: &Connection,
    document_extraction_id: &str,
    skill_id: &str,
    extraction_scope: &str,
    expected_semantic_id: &str,
    expected_target_domain: &str,
) -> Result<()> {
    let current: Option<(String, String)> = conn
        .query_row(
            "\
            SELECT id, target_domain
            FROM document_semantic_extractions
            WHERE document_extraction_id = ?1
              AND skill_id = ?2
              AND extraction_scope = ?3
              AND is_current = 1
            ",
            params![document_extraction_id, skill_id, extraction_scope],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    let (semantic_id, target_domain) = current.ok_or_else(|| {
        anyhow::anyhow!(
            "expected current document_semantic_extractions row for extraction `{document_extraction_id}`"
        )
    })?;
    ensure!(
        semantic_id == expected_semantic_id,
        "expected semantic extraction `{expected_semantic_id}`, got `{semantic_id}`"
    );
    ensure!(
        target_domain == expected_target_domain,
        "expected target_domain `{expected_target_domain}`, got `{target_domain}`"
    );
    Ok(())
}

pub fn logical_schema_for_tables(conn: &Connection, table_names: &[&str]) -> Result<LogicalSchema> {
    let mut schema = LogicalSchema::default();

    for table_name in table_names {
        let pragma = format!("PRAGMA table_info({})", sqlite_identifier(table_name));
        let mut stmt = conn.prepare(&pragma)?;
        let mut columns = stmt
            .query_map([], |row| {
                let name: String = row.get("name")?;
                let raw_type: String = row.get("type")?;
                let not_null: i64 = row.get("notnull")?;
                let primary_key_position: i64 = row.get("pk")?;
                Ok(LogicalColumn {
                    name: name.clone(),
                    logical_type: normalize_sql_type(&raw_type, &name),
                    nullable: not_null == 0 && primary_key_position == 0,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        ensure!(
            !columns.is_empty(),
            "expected SQLite table `{table_name}` to exist"
        );
        columns.sort_by(|left, right| left.name.cmp(&right.name));
        schema.insert((*table_name).to_owned(), columns);
    }

    Ok(schema)
}

pub fn logical_schema_hash(schema: &LogicalSchema) -> String {
    let mut canonical = String::new();
    for (table, columns) in schema {
        canonical.push_str("table:");
        canonical.push_str(table);
        canonical.push('\n');
        for column in columns {
            canonical.push_str("  column:");
            canonical.push_str(&column.name);
            canonical.push(':');
            canonical.push_str(&column.logical_type);
            canonical.push(':');
            canonical.push_str(if column.nullable {
                "nullable"
            } else {
                "not-null"
            });
            canonical.push('\n');
        }
    }
    format!("{:x}", Sha256::digest(canonical.as_bytes()))
}

pub fn normalize_sql_type(raw_type: &str, column_name: &str) -> String {
    let lower_type = raw_type.trim().to_ascii_lowercase();
    let lower_name = column_name.to_ascii_lowercase();

    if lower_type.contains("json")
        || lower_name.ends_with("_json")
        || matches!(
            lower_name.as_str(),
            "normalized_payload" | "documents" | "warnings" | "blockers" | "confidence_breakdown"
        )
    {
        return "json".to_owned();
    }

    if lower_type.contains("timestamp")
        || lower_type.contains("timestamptz")
        || lower_name.ends_with("_at")
        || lower_name.ends_with("_until")
        || lower_name.ends_with("_for")
    {
        return "timestamp".to_owned();
    }

    if lower_type.contains("int") || lower_type.contains("bool") {
        return "integer".to_owned();
    }

    if lower_type.contains("numeric")
        || lower_type.contains("real")
        || lower_type.contains("double")
    {
        return "numeric".to_owned();
    }

    if lower_type.contains("text") || lower_type.contains("char") || lower_type.is_empty() {
        return "text".to_owned();
    }

    lower_type
        .split('(')
        .next()
        .unwrap_or(lower_type.as_str())
        .trim()
        .to_owned()
}

fn sqlite_identifier(identifier: &str) -> String {
    format!("\"{}\"", identifier.replace('"', "\"\""))
}

fn sample_docling_output() -> Value {
    json!({
        "markdown": "# Tender specification\n\n## Lot 1\nStainless steel storage cabinets compliant with BS EN 16121.",
        "sections": [
            {
                "section_ref": "lot-1",
                "heading": "Lot 1",
                "page_start": 1,
                "page_end": 1
            }
        ],
        "tables": [
            {
                "table_key": "tbl-pricing-1",
                "title": "Pricing schedule",
                "page_start": 1,
                "page_end": 1
            }
        ],
        "metadata": {
            "document_title": "Tender specification",
            "document_language": "en",
            "source": "test-fixture"
        },
        "page_count": 1,
        "quality": {
            "table_count": 1,
            "section_count": 1
        }
    })
}

fn sample_semantic_result(skill_id: &str) -> Value {
    match skill_id {
        "pdf-extraction/tender-specification" => json!({
            "lots": [
                {
                    "lot_number": "1",
                    "title": "Storage cabinets",
                    "evidence_anchor": {
                        "page_ref": "p1",
                        "anchor_text": "Lot 1"
                    }
                }
            ],
            "product_requirement_seeds": [],
            "field_confidences": {},
            "extraction_confidence_bps": 9200,
            "warnings": [],
            "evidence_anchors": [],
            "extraction_metadata": {
                "extraction_scope": "full_document"
            }
        }),
        "pdf-extraction/pricing-schedule" => json!({
            "line_items": [],
            "pricing_summary": {
                "total_value_pence": 0,
                "currency": "GBP",
                "lot_count": 0
            },
            "field_confidences": {},
            "extraction_confidence_bps": 9100,
            "warnings": [],
            "evidence_anchors": [],
            "extraction_metadata": {
                "extraction_scope": "full_document"
            }
        }),
        "pdf-extraction/bill-of-quantities" => json!({
            "items": [],
            "summary": {
                "item_count": 0,
                "total_value_pence": 0
            },
            "field_confidences": {},
            "extraction_confidence_bps": 9100,
            "warnings": [],
            "evidence_anchors": [],
            "extraction_metadata": {
                "extraction_scope": "full_document"
            }
        }),
        _ => json!({
            "extracted_entities": [],
            "document_classification": {
                "document_role": "technical_specification",
                "classification_confidence_bps": 9300,
                "target_domain": "generic"
            },
            "extraction_warnings": [],
            "extraction_metadata": {
                "target_domain": "generic",
                "extraction_scope": "full_document",
                "source_markdown_hash": "sample-markdown-hash",
                "policy_version": "2026.04.12"
            }
        }),
    }
}
