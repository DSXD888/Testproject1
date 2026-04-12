#![forbid(unsafe_code)]
#![allow(clippy::duplicate_mod)]

mod support;

#[path = "../src/pdf/engine.rs"]
#[allow(dead_code)]
mod engine_impl;

mod tests {
    use anyhow::{Context, Result, ensure};
    use rusqlite::{OptionalExtension, params};
    use serde_json::Value;
    use serial_test::serial;
    use sha2::{Digest, Sha256};
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::{Path, PathBuf};
    use tempfile::tempdir;

    use crate::engine_impl::{
        EngineConfig, PdfExtractionEngine, PdfInput, PersistStructuralExtractionRow,
        ProcessingStrategy, clone_reused_extraction, compute_source_hash,
        lookup_reusable_extraction, persist_structural_extraction,
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

    fn success_wrapper_payload() -> Value {
        let mut payload = sample_docling_output();
        payload["metadata"]["docling_version"] = serde_json::Value::String("2.85.0".to_owned());
        payload
    }

    struct CurrentDirGuard {
        prior_dir: PathBuf,
    }

    impl CurrentDirGuard {
        fn install(path: &Path) -> Result<Self> {
            let prior_dir = std::env::current_dir().context("reading current directory")?;
            std::env::set_current_dir(path)
                .with_context(|| format!("switching current directory to {}", path.display()))?;
            Ok(Self { prior_dir })
        }
    }

    impl Drop for CurrentDirGuard {
        fn drop(&mut self) {
            let _ = std::env::set_current_dir(&self.prior_dir);
        }
    }

    fn write_executable(path: &Path, contents: &str) -> Result<()> {
        fs::write(path, contents).with_context(|| format!("writing {}", path.display()))?;
        let mut permissions = fs::metadata(path)
            .with_context(|| format!("reading metadata for {}", path.display()))?
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions)
            .with_context(|| format!("marking {} executable", path.display()))?;
        Ok(())
    }

    fn write_probe_docling_module(dir: &Path) -> Result<PathBuf> {
        let docling_path = dir.join("docling.py");
        fs::write(&docling_path, "__version__ = '2.85.0'\n")
            .with_context(|| format!("writing {}", docling_path.display()))?;
        Ok(docling_path)
    }

    fn write_success_wrapper(dir: &Path, payload_path: &Path) -> Result<PathBuf> {
        let wrapper_path = dir.join("docling-wrapper");
        let contents = format!(
            concat!(
                "#!/usr/bin/env python3\n",
                "import pathlib\n",
                "import shutil\n",
                "import sys\n",
                "\n",
                "def main() -> int:\n",
                "    output_path = None\n",
                "    args = iter(sys.argv[1:])\n",
                "    for arg in args:\n",
                "        if arg == '--output':\n",
                "            output_path = next(args)\n",
                "        elif arg in ('--input', '--strategy', '--timeout'):\n",
                "            next(args)\n",
                "    if output_path is None:\n",
                "        raise SystemExit('missing --output')\n",
                "    shutil.copyfile(pathlib.Path(r'{}'), pathlib.Path(output_path))\n",
                "    print('mock-docling-stdout')\n",
                "    print('mock-docling-stderr', file=sys.stderr)\n",
                "    return 0\n",
                "\n",
                "if __name__ == '__main__':\n",
                "    raise SystemExit(main())\n"
            ),
            payload_path.display()
        );
        write_executable(&wrapper_path, &contents)?;
        Ok(wrapper_path)
    }

    fn write_timeout_wrapper(dir: &Path) -> Result<PathBuf> {
        let wrapper_path = dir.join("docling-wrapper-timeout");
        let contents = "#!/usr/bin/env python3\nimport time\n\ntime.sleep(5)\n";
        write_executable(&wrapper_path, contents)?;
        Ok(wrapper_path)
    }

    fn write_failing_wrapper(dir: &Path, exit_code: i32, stderr: &str) -> Result<PathBuf> {
        let wrapper_path = dir.join(format!("docling-wrapper-fail-{exit_code}"));
        let contents = format!(
            "#!/usr/bin/env python3\n\
import sys\n\
print({stderr:?}, file=sys.stderr)\n\
raise SystemExit({exit_code})\n"
        );
        write_executable(&wrapper_path, &contents)?;
        Ok(wrapper_path)
    }

    fn write_pdf(path: &Path) -> Result<()> {
        fs::write(path, sample_pdf_bytes())
            .with_context(|| format!("writing sample pdf {}", path.display()))?;
        Ok(())
    }

    fn write_encrypted_pdf(path: &Path) -> Result<()> {
        fs::write(
            path,
            b"%PDF-1.4\n1 0 obj\n<< /Encrypt 2 0 R >>\nendobj\n%%EOF\n",
        )
        .with_context(|| format!("writing encrypted sample pdf {}", path.display()))?;
        Ok(())
    }

    const fn build_engine_config(wrapper_path: PathBuf, artifacts_dir: PathBuf) -> EngineConfig {
        EngineConfig {
            docling_wrapper_path: wrapper_path,
            model_artifacts_path: artifacts_dir,
            document_timeout_secs: 1,
            wall_clock_timeout_secs: 1,
        }
    }

    #[test]
    #[serial]
    fn test_health_probe_available() -> Result<()> {
        let temp = tempdir()?;
        let payload_path = temp.path().join("payload.json");
        fs::write(
            &payload_path,
            serde_json::to_vec_pretty(&success_wrapper_payload())?,
        )?;
        let wrapper_path = write_success_wrapper(temp.path(), &payload_path)?;
        let artifacts_dir = temp.path().join("artifacts");
        fs::create_dir_all(&artifacts_dir)?;
        let _dir_guard = CurrentDirGuard::install(temp.path())?;
        write_probe_docling_module(temp.path())?;

        let engine = PdfExtractionEngine::new(build_engine_config(wrapper_path, artifacts_dir));
        ensure!(engine.is_available()?);
        Ok(())
    }

    #[test]
    #[serial]
    fn test_health_probe_unavailable() -> Result<()> {
        let temp = tempdir()?;
        let artifacts_dir = temp.path().join("artifacts");
        fs::create_dir_all(&artifacts_dir)?;
        let _dir_guard = CurrentDirGuard::install(temp.path())?;
        write_probe_docling_module(temp.path())?;

        let engine = PdfExtractionEngine::new(build_engine_config(
            temp.path().join("missing-wrapper"),
            artifacts_dir,
        ));
        ensure!(!engine.is_available()?);
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn test_extract_success() -> Result<()> {
        let temp = tempdir()?;
        let pdf_path = temp.path().join("sample.pdf");
        let payload_path = temp.path().join("payload.json");
        let artifacts_dir = temp.path().join("artifacts");
        write_pdf(&pdf_path)?;
        fs::write(
            &payload_path,
            serde_json::to_vec_pretty(&success_wrapper_payload())?,
        )?;
        fs::create_dir_all(&artifacts_dir)?;
        let wrapper_path = write_success_wrapper(temp.path(), &payload_path)?;
        let _dir_guard = CurrentDirGuard::install(temp.path())?;
        write_probe_docling_module(temp.path())?;

        let engine = PdfExtractionEngine::new(build_engine_config(wrapper_path, artifacts_dir));
        let result = engine
            .extract(PdfInput {
                raw_document_id: Some("raw-success".to_owned()),
                source_path: pdf_path,
                page_range: None,
                processing_strategy: ProcessingStrategy::Standard,
            })
            .await?;

        ensure!(result.page_count == 1);
        ensure!(result.engine_version == "2.85.0");
        ensure!(result.processing_strategy == ProcessingStrategy::Standard);
        ensure!(result.extracted_markdown.contains("Tender specification"));
        ensure!(result.source_hash == compute_source_hash(&sample_pdf_bytes()));
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn test_extract_timeout() -> Result<()> {
        let temp = tempdir()?;
        let pdf_path = temp.path().join("sample.pdf");
        let artifacts_dir = temp.path().join("artifacts");
        write_pdf(&pdf_path)?;
        fs::create_dir_all(&artifacts_dir)?;
        let wrapper_path = write_timeout_wrapper(temp.path())?;
        let _dir_guard = CurrentDirGuard::install(temp.path())?;
        write_probe_docling_module(temp.path())?;

        let engine = PdfExtractionEngine::new(build_engine_config(wrapper_path, artifacts_dir));
        let error = engine
            .extract(PdfInput {
                raw_document_id: Some("raw-timeout".to_owned()),
                source_path: pdf_path,
                page_range: None,
                processing_strategy: ProcessingStrategy::Standard,
            })
            .await
            .expect_err("timeout wrapper should fail");
        ensure!(error.to_string().contains("timed out"));
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn test_timeout_handling() -> Result<()> {
        let temp = tempdir()?;
        let pdf_path = temp.path().join("sample.pdf");
        let artifacts_dir = temp.path().join("artifacts");
        write_pdf(&pdf_path)?;
        fs::create_dir_all(&artifacts_dir)?;
        let wrapper_path = write_timeout_wrapper(temp.path())?;
        let _dir_guard = CurrentDirGuard::install(temp.path())?;
        write_probe_docling_module(temp.path())?;

        let engine = PdfExtractionEngine::new(build_engine_config(wrapper_path, artifacts_dir));
        let error = engine
            .extract(PdfInput {
                raw_document_id: Some("raw-timeout-code".to_owned()),
                source_path: pdf_path,
                page_range: None,
                processing_strategy: ProcessingStrategy::Standard,
            })
            .await
            .expect_err("timeout wrapper should fail");
        ensure!(error.to_string().contains("EXTRACTION_TIMEOUT"));
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn test_extract_batch() -> Result<()> {
        let temp = tempdir()?;
        let pdf_a = temp.path().join("sample-a.pdf");
        let pdf_b = temp.path().join("sample-b.pdf");
        let payload_path = temp.path().join("payload.json");
        let artifacts_dir = temp.path().join("artifacts");
        write_pdf(&pdf_a)?;
        write_pdf(&pdf_b)?;
        fs::write(
            &payload_path,
            serde_json::to_vec_pretty(&success_wrapper_payload())?,
        )?;
        fs::create_dir_all(&artifacts_dir)?;
        let wrapper_path = write_success_wrapper(temp.path(), &payload_path)?;
        let _dir_guard = CurrentDirGuard::install(temp.path())?;
        write_probe_docling_module(temp.path())?;

        let engine = PdfExtractionEngine::new(build_engine_config(wrapper_path, artifacts_dir));
        let results = engine
            .extract_batch(vec![
                PdfInput {
                    raw_document_id: Some("raw-batch-a".to_owned()),
                    source_path: pdf_a,
                    page_range: None,
                    processing_strategy: ProcessingStrategy::Standard,
                },
                PdfInput {
                    raw_document_id: Some("raw-batch-b".to_owned()),
                    source_path: pdf_b,
                    page_range: None,
                    processing_strategy: ProcessingStrategy::FastTableStructure,
                },
            ])
            .await;

        ensure!(results.len() == 2);
        ensure!(results.iter().all(Result::is_ok));
        ensure!(results[1].as_ref().ok().is_some_and(
            |result| result.processing_strategy == ProcessingStrategy::FastTableStructure
        ));
        Ok(())
    }

    #[test]
    fn test_content_hash_computation() -> Result<()> {
        let pdf_bytes = sample_pdf_bytes();
        let expected = format!("{:x}", Sha256::digest(&pdf_bytes));
        ensure!(compute_source_hash(&pdf_bytes) == expected);
        ensure!(compute_source_hash(b"different") != expected);
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn test_encrypted_pdf_detection() -> Result<()> {
        let temp = tempdir()?;
        let pdf_path = temp.path().join("encrypted.pdf");
        write_encrypted_pdf(&pdf_path)?;
        let engine = PdfExtractionEngine::new(build_engine_config(
            temp.path().join("missing-wrapper"),
            temp.path().join("missing-artifacts"),
        ));

        let error = engine
            .extract(PdfInput {
                raw_document_id: Some("raw-encrypted".to_owned()),
                source_path: pdf_path,
                page_range: None,
                processing_strategy: ProcessingStrategy::Standard,
            })
            .await
            .expect_err("encrypted pdf should fail early");
        ensure!(error.to_string().contains("EXTRACTION_ENCRYPTED_PDF"));
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn test_empty_pdf_handling() -> Result<()> {
        let temp = tempdir()?;
        let pdf_path = temp.path().join("empty.pdf");
        fs::write(&pdf_path, b"")?;
        let engine = PdfExtractionEngine::new(build_engine_config(
            temp.path().join("missing-wrapper"),
            temp.path().join("missing-artifacts"),
        ));

        let error = engine
            .extract(PdfInput {
                raw_document_id: Some("raw-empty".to_owned()),
                source_path: pdf_path,
                page_range: None,
                processing_strategy: ProcessingStrategy::Standard,
            })
            .await
            .expect_err("empty pdf should fail early");
        ensure!(error.to_string().contains("EXTRACTION_EMPTY_PDF"));
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn test_engine_unavailable() -> Result<()> {
        let temp = tempdir()?;
        let pdf_path = temp.path().join("sample.pdf");
        let artifacts_dir = temp.path().join("artifacts");
        write_pdf(&pdf_path)?;
        fs::create_dir_all(&artifacts_dir)?;
        let wrapper_path = write_success_wrapper(temp.path(), &temp.path().join("payload.json"))?;
        let _dir_guard = CurrentDirGuard::install(temp.path())?;
        // Intentionally do not provide a local docling.py probe module.

        let engine = PdfExtractionEngine::new(build_engine_config(wrapper_path, artifacts_dir));
        let error = engine
            .extract(PdfInput {
                raw_document_id: Some("raw-engine-unavailable".to_owned()),
                source_path: pdf_path,
                page_range: None,
                processing_strategy: ProcessingStrategy::Standard,
            })
            .await
            .expect_err("missing docling runtime should fail");
        ensure!(error.to_string().contains("EXTRACTION_ENGINE_UNAVAILABLE"));
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn test_corrupt_pdf_error() -> Result<()> {
        let temp = tempdir()?;
        let pdf_path = temp.path().join("sample.pdf");
        let artifacts_dir = temp.path().join("artifacts");
        write_pdf(&pdf_path)?;
        fs::create_dir_all(&artifacts_dir)?;
        let wrapper_path = write_failing_wrapper(temp.path(), 1, "corrupt fixture")?;
        let _dir_guard = CurrentDirGuard::install(temp.path())?;
        write_probe_docling_module(temp.path())?;

        let engine = PdfExtractionEngine::new(build_engine_config(wrapper_path, artifacts_dir));
        let error = engine
            .extract(PdfInput {
                raw_document_id: Some("raw-corrupt".to_owned()),
                source_path: pdf_path,
                page_range: None,
                processing_strategy: ProcessingStrategy::Standard,
            })
            .await
            .expect_err("non-zero wrapper exit should fail");
        ensure!(error.to_string().contains("EXTRACTION_CORRUPT_PDF"));
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn test_oom_detection() -> Result<()> {
        let temp = tempdir()?;
        let pdf_path = temp.path().join("sample.pdf");
        let artifacts_dir = temp.path().join("artifacts");
        write_pdf(&pdf_path)?;
        fs::create_dir_all(&artifacts_dir)?;
        let wrapper_path = write_failing_wrapper(temp.path(), 137, "oom fixture")?;
        let _dir_guard = CurrentDirGuard::install(temp.path())?;
        write_probe_docling_module(temp.path())?;

        let engine = PdfExtractionEngine::new(build_engine_config(wrapper_path, artifacts_dir));
        let error = engine
            .extract(PdfInput {
                raw_document_id: Some("raw-oom".to_owned()),
                source_path: pdf_path,
                page_range: None,
                processing_strategy: ProcessingStrategy::Standard,
            })
            .await
            .expect_err("oom-style wrapper exit should fail");
        ensure!(error.to_string().contains("EXTRACTION_OOM"));
        Ok(())
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
