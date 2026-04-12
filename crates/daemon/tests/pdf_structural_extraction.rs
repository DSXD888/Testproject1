#![forbid(unsafe_code)]
#![allow(clippy::duplicate_mod)]

mod support;

#[path = "../src/pdf/engine.rs"]
#[allow(dead_code)]
mod engine_impl;

mod tests {
    use std::fs;

    use anyhow::{Context, Result, ensure};
    use rusqlite::{OptionalExtension, params};
    use tempfile::tempdir;

    use crate::engine_impl::{
        PersistStructuralExtractionRow, ProcessingStrategy, clone_reused_extraction,
        compute_source_hash, lookup_reusable_extraction, persist_structural_extraction,
    };
    use crate::support::db::{
        DocumentExtractionSeed, TestStateDb, assert_current_document_extraction,
        seed_document_extraction, seed_process, seed_raw_document,
    };
    use crate::support::pdf::{MockDocling, sample_docling_output, sample_pdf_bytes};

    fn build_row(
        extraction_id: &str,
        raw_document_id: &str,
        process_id: &str,
        source_content_hash: &str,
        stage_run_id: &str,
    ) -> Result<PersistStructuralExtractionRow> {
        let payload = sample_docling_output();
        let quality = &payload["quality"];
        Ok(PersistStructuralExtractionRow {
            extraction_id: extraction_id.to_owned(),
            raw_document_id: raw_document_id.to_owned(),
            process_id: process_id.to_owned(),
            notice_id: None,
            document_set_id: None,
            source_content_hash: source_content_hash.to_owned(),
            extraction_engine_version: "2.85.0".to_owned(),
            processing_strategy: ProcessingStrategy::Standard,
            page_count: u32::try_from(payload["page_count"].as_u64().unwrap_or(1))
                .context("converting sample page_count")?,
            document_title: payload["metadata"]["document_title"]
                .as_str()
                .map(str::to_owned),
            document_language: payload["metadata"]["document_language"]
                .as_str()
                .map(str::to_owned),
            document_role: Some("technical_specification".to_owned()),
            extraction_policy_version: "2026.04.12".to_owned(),
            expected_overlays_json: "[]".to_owned(),
            extracted_markdown: payload["markdown"].as_str().unwrap_or_default().to_owned(),
            extracted_sections_json: payload["sections"].to_string(),
            extracted_tables_json: payload["tables"].to_string(),
            extracted_metadata_json: payload["metadata"].to_string(),
            table_count: u32::try_from(quality["table_count"].as_u64().unwrap_or(0))
                .context("converting sample table_count")?,
            section_count: u32::try_from(quality["section_count"].as_u64().unwrap_or(0))
                .context("converting sample section_count")?,
            image_count: u32::try_from(quality["image_count"].as_u64().unwrap_or(0))
                .context("converting sample image_count")?,
            has_ocr_content: quality["has_ocr_content"].as_bool().unwrap_or(false),
            empty_page_count: u32::try_from(quality["empty_page_count"].as_u64().unwrap_or(0))
                .context("converting sample empty_page_count")?,
            quality_warnings_json: Some(quality["warnings"].to_string()),
            semantic_extraction_state: "pending".to_owned(),
            owner_id: "local-owner".to_owned(),
            stage_run_id: stage_run_id.to_owned(),
        })
    }

    #[test]
    fn test_cache_hit_clones_without_rerunning_docling() -> Result<()> {
        let state_db = TestStateDb::new()?;
        let conn = state_db.connection();
        let source_hash = compute_source_hash(&sample_pdf_bytes());

        seed_process(conn, "proc-cache-hit")?;
        seed_raw_document(conn, "raw-original")?;
        seed_raw_document(conn, "raw-clone")?;
        seed_document_extraction(
            conn,
            &DocumentExtractionSeed {
                id: "extract-original",
                raw_document_id: "raw-original",
                process_id: "proc-cache-hit",
                notice_id: None,
                document_set_id: None,
                source_content_hash: &source_hash,
                document_role: Some("technical_specification"),
                semantic_extraction_state: "complete",
            },
        )?;

        let reusable =
            lookup_reusable_extraction(conn, &source_hash, "2.85.0", ProcessingStrategy::Standard)?
                .context("expected reusable extraction")?;
        let cloned = clone_reused_extraction(
            &reusable,
            "raw-clone",
            "proc-cache-hit",
            None,
            None,
            "stage-run-cache-hit",
        );
        persist_structural_extraction(conn, &cloned)?;

        assert_current_document_extraction(conn, "raw-clone", &cloned.extraction_id)?;
        let cloned_state: Option<String> = conn
            .query_row(
                "SELECT semantic_extraction_state FROM document_extractions WHERE id = ?1",
                params![cloned.extraction_id],
                |row| row.get(0),
            )
            .optional()?;
        ensure!(cloned_state.as_deref() == Some("pending"));
        Ok(())
    }

    #[test]
    fn test_cache_miss_runs_docling_and_persists_new_row() -> Result<()> {
        let state_db = TestStateDb::new()?;
        let conn = state_db.connection();
        seed_process(conn, "proc-cache-miss")?;
        seed_raw_document(conn, "raw-cache-miss")?;

        let source_hash = compute_source_hash(&sample_pdf_bytes());
        ensure!(
            lookup_reusable_extraction(conn, &source_hash, "2.85.0", ProcessingStrategy::Standard)?
                .is_none(),
            "unexpected reusable extraction on empty fixture db"
        );

        let temp = tempdir()?;
        let output_path = temp.path().join("docling.json");
        let mock = MockDocling::success();
        ensure!(mock.run(&output_path)? == 0);
        ensure!(output_path.is_file(), "expected mock Docling output file");
        let _: serde_json::Value =
            serde_json::from_slice(&fs::read(&output_path)?).context("parsing mock output")?;

        let row = build_row(
            "extract-cache-miss",
            "raw-cache-miss",
            "proc-cache-miss",
            &source_hash,
            "stage-run-cache-miss",
        )?;
        persist_structural_extraction(conn, &row)?;

        assert_current_document_extraction(conn, "raw-cache-miss", "extract-cache-miss")?;
        let persisted_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM document_extractions WHERE raw_document_id = ?1",
            params!["raw-cache-miss"],
            |row| row.get(0),
        )?;
        ensure!(persisted_count == 1);
        Ok(())
    }

    #[test]
    fn test_current_row_supersession_is_atomic() -> Result<()> {
        let state_db = TestStateDb::new()?;
        let conn = state_db.connection();
        let source_hash = compute_source_hash(&sample_pdf_bytes());

        seed_process(conn, "proc-supersession")?;
        seed_raw_document(conn, "raw-supersession")?;
        seed_document_extraction(
            conn,
            &DocumentExtractionSeed {
                id: "extract-existing",
                raw_document_id: "raw-supersession",
                process_id: "proc-supersession",
                notice_id: None,
                document_set_id: None,
                source_content_hash: &source_hash,
                document_role: Some("technical_specification"),
                semantic_extraction_state: "pending",
            },
        )?;

        let duplicate_id_row = build_row(
            "extract-existing",
            "raw-supersession",
            "proc-supersession",
            &source_hash,
            "stage-run-supersession",
        )?;
        let error = persist_structural_extraction(conn, &duplicate_id_row)
            .expect_err("duplicate id should roll back supersession transaction");
        ensure!(
            error
                .to_string()
                .contains("inserting structural extraction row")
        );

        let current: (i64, Option<String>) = conn.query_row(
            "SELECT is_current, superseded_by_extraction_id FROM document_extractions WHERE id = ?1",
            params!["extract-existing"],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        ensure!(current.0 == 1);
        ensure!(current.1.is_none());
        Ok(())
    }

    #[test]
    fn test_same_bytes_different_raw_document_id_clones_authoritative_row() -> Result<()> {
        let state_db = TestStateDb::new()?;
        let conn = state_db.connection();
        let source_hash = compute_source_hash(&sample_pdf_bytes());

        seed_process(conn, "proc-shared-bytes")?;
        seed_raw_document(conn, "raw-a")?;
        seed_raw_document(conn, "raw-b")?;
        seed_document_extraction(
            conn,
            &DocumentExtractionSeed {
                id: "extract-a",
                raw_document_id: "raw-a",
                process_id: "proc-shared-bytes",
                notice_id: None,
                document_set_id: None,
                source_content_hash: &source_hash,
                document_role: Some("technical_specification"),
                semantic_extraction_state: "pending",
            },
        )?;

        let reusable =
            lookup_reusable_extraction(conn, &source_hash, "2.85.0", ProcessingStrategy::Standard)?
                .context("expected reusable extraction")?;
        let clone = clone_reused_extraction(
            &reusable,
            "raw-b",
            "proc-shared-bytes",
            None,
            None,
            "stage-run-shared-bytes",
        );
        persist_structural_extraction(conn, &clone)?;

        let cloned_values: (String, String, String) = conn.query_row(
            "\
            SELECT source_content_hash, extracted_markdown, stage_run_id
            FROM document_extractions
            WHERE id = ?1
            ",
            params![clone.extraction_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )?;
        ensure!(cloned_values.0 == source_hash);
        ensure!(cloned_values.1 == reusable.extracted_markdown);
        ensure!(cloned_values.2 == "stage-run-shared-bytes");
        assert_current_document_extraction(conn, "raw-b", &clone.extraction_id)?;
        Ok(())
    }
}
