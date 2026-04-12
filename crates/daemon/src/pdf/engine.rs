#![forbid(unsafe_code)]

use std::ffi::OsString;
use std::fs;
use std::io::ErrorKind;
use std::os::unix::process::ExitStatusExt;
use std::path::PathBuf;
use std::process::{Command as StdCommand, Stdio};

use anyhow::{Context, Result, anyhow, bail};
use nix::sys::signal::{Signal, killpg};
use nix::unistd::Pid;
use rusqlite::{Connection, OptionalExtension, Transaction, TransactionBehavior, params};
use serde_json::Value;
use sha2::{Digest, Sha256};
use tokio::fs::{self as tokio_fs, File};
use tokio::process::{Child, Command};
use tokio::time::{Duration, timeout};
use uuid::Uuid;

const EXTRACTION_ENCRYPTED_PDF: &str = "EXTRACTION_ENCRYPTED_PDF";
const EXTRACTION_EMPTY_PDF: &str = "EXTRACTION_EMPTY_PDF";
const EXTRACTION_CORRUPT_PDF: &str = "EXTRACTION_CORRUPT_PDF";
const EXTRACTION_ENGINE_UNAVAILABLE: &str = "EXTRACTION_ENGINE_UNAVAILABLE";
const EXTRACTION_TIMEOUT: &str = "EXTRACTION_TIMEOUT";
const EXTRACTION_OOM: &str = "EXTRACTION_OOM";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessingStrategy {
    Standard,
    ForceFullPageOcr,
    FastTableStructure,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PdfInput {
    pub raw_document_id: Option<String>,
    pub source_path: PathBuf,
    pub page_range: Option<(u32, u32)>,
    pub processing_strategy: ProcessingStrategy,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractionQuality {
    pub table_count: u32,
    pub section_count: u32,
    pub image_count: u32,
    pub empty_page_count: u32,
    pub has_ocr_content: bool,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractionResult {
    pub extraction_id: String,
    pub source_hash: String,
    pub page_count: u32,
    pub extracted_markdown: String,
    pub sections_json: serde_json::Value,
    pub tables_json: serde_json::Value,
    pub metadata_json: serde_json::Value,
    pub quality: ExtractionQuality,
    pub engine_version: String,
    pub processing_strategy: ProcessingStrategy,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineConfig {
    pub docling_wrapper_path: PathBuf,
    pub model_artifacts_path: PathBuf,
    pub document_timeout_secs: u64,
    pub wall_clock_timeout_secs: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PdfExtractionEngine {
    config: EngineConfig,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReusableStructuralExtraction {
    pub extraction_id: String,
    pub source_content_hash: String,
    pub extraction_engine_version: String,
    pub processing_strategy: ProcessingStrategy,
    pub page_count: u32,
    pub document_title: Option<String>,
    pub document_language: Option<String>,
    pub document_role: Option<String>,
    pub extraction_policy_version: String,
    pub expected_overlays_json: String,
    pub extracted_markdown: String,
    pub extracted_sections_json: String,
    pub extracted_tables_json: String,
    pub extracted_metadata_json: String,
    pub table_count: u32,
    pub section_count: u32,
    pub image_count: u32,
    pub has_ocr_content: bool,
    pub empty_page_count: u32,
    pub quality_warnings_json: Option<String>,
    pub owner_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistStructuralExtractionRow {
    pub extraction_id: String,
    pub raw_document_id: String,
    pub process_id: String,
    pub notice_id: Option<String>,
    pub document_set_id: Option<String>,
    pub source_content_hash: String,
    pub extraction_engine_version: String,
    pub processing_strategy: ProcessingStrategy,
    pub page_count: u32,
    pub document_title: Option<String>,
    pub document_language: Option<String>,
    pub document_role: Option<String>,
    pub extraction_policy_version: String,
    pub expected_overlays_json: String,
    pub extracted_markdown: String,
    pub extracted_sections_json: String,
    pub extracted_tables_json: String,
    pub extracted_metadata_json: String,
    pub table_count: u32,
    pub section_count: u32,
    pub image_count: u32,
    pub has_ocr_content: bool,
    pub empty_page_count: u32,
    pub quality_warnings_json: Option<String>,
    pub semantic_extraction_state: String,
    pub owner_id: String,
    pub stage_run_id: String,
}

impl PdfExtractionEngine {
    #[must_use]
    pub const fn new(config: EngineConfig) -> Self {
        Self { config }
    }

    pub fn is_available(&self) -> Result<bool> {
        if !self.wrapper_exists()? || !self.artifacts_dir_exists()? {
            return Ok(false);
        }

        Ok(Self::probe_docling_version()?.is_some())
    }

    pub async fn extract(&self, input: PdfInput) -> Result<ExtractionResult> {
        let source_bytes = tokio_fs::read(&input.source_path)
            .await
            .with_context(|| format!("reading PDF bytes from {}", input.source_path.display()))?;
        validate_source_pdf_bytes(&source_bytes)?;
        self.ensure_runtime_ready()?;

        let scratch_dir = scratch_dir();
        tokio_fs::create_dir_all(&scratch_dir)
            .await
            .with_context(|| format!("creating scratch dir {}", scratch_dir.display()))?;

        let output_path = scratch_dir.join("docling-output.json");
        let stdout_path = scratch_dir.join("stdout.log");
        let stderr_path = scratch_dir.join("stderr.log");

        let stdout_file = File::create(&stdout_path)
            .await
            .with_context(|| format!("creating stdout capture {}", stdout_path.display()))?
            .into_std()
            .await;
        let stderr_file = File::create(&stderr_path)
            .await
            .with_context(|| format!("creating stderr capture {}", stderr_path.display()))?
            .into_std()
            .await;

        let mut command = Command::new("python3");
        command
            .arg(&self.config.docling_wrapper_path)
            .arg("--input")
            .arg(&input.source_path)
            .arg("--output")
            .arg(&output_path)
            .arg("--strategy")
            .arg(strategy_name(input.processing_strategy))
            .arg("--timeout")
            .arg(self.config.document_timeout_secs.to_string())
            .env("DOCLING_ARTIFACTS_PATH", &self.config.model_artifacts_path)
            .env("HF_HUB_OFFLINE", "1")
            .env("TRANSFORMERS_OFFLINE", "1")
            .stdout(Stdio::from(stdout_file))
            .stderr(Stdio::from(stderr_file))
            .process_group(0);

        let mut child = command.spawn().context("spawning Docling wrapper")?;
        let wait_result = timeout(
            Duration::from_secs(self.config.wall_clock_timeout_secs),
            child.wait(),
        )
        .await;

        let status = if let Ok(status) = wait_result {
            status.context("waiting for Docling wrapper")?
        } else {
            terminate_process_group(&mut child, Duration::from_secs(5)).await?;
            return extraction_error(
                EXTRACTION_TIMEOUT,
                &format!(
                    "Docling wrapper timed out after {}s; stderr captured at {}",
                    self.config.wall_clock_timeout_secs,
                    stderr_path.display()
                ),
            );
        };

        if !status.success() {
            let stderr = tokio_fs::read_to_string(&stderr_path)
                .await
                .unwrap_or_default();
            let code = classify_failed_wrapper_status(status, stderr.trim());
            return extraction_error(
                code,
                &format!(
                    "Docling wrapper failed with status {:?}: {}",
                    status.code(),
                    stderr.trim()
                ),
            );
        }

        let source_hash = compute_source_hash(&source_bytes);

        let output_text = tokio_fs::read_to_string(&output_path)
            .await
            .with_context(|| format!("reading Docling output {}", output_path.display()))?;
        let payload: Value =
            serde_json::from_str(&output_text).context("parsing Docling wrapper JSON output")?;

        let result = parse_extraction_result(&payload, source_hash, input.processing_strategy)?;
        validate_extraction_result(&result)?;
        Ok(result)
    }

    pub async fn extract_batch(&self, inputs: Vec<PdfInput>) -> Vec<Result<ExtractionResult>> {
        let mut results = Vec::with_capacity(inputs.len());
        for input in inputs {
            results.push(self.extract(input).await);
        }
        results
    }

    fn wrapper_exists(&self) -> Result<bool> {
        match fs::metadata(&self.config.docling_wrapper_path) {
            Ok(metadata) => Ok(metadata.is_file()),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(false),
            Err(error) => Err(error.into()),
        }
    }

    fn artifacts_dir_exists(&self) -> Result<bool> {
        match fs::metadata(&self.config.model_artifacts_path) {
            Ok(metadata) => Ok(metadata.is_dir()),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(false),
            Err(error) => Err(error.into()),
        }
    }

    fn probe_docling_version() -> Result<Option<String>> {
        let output = StdCommand::new("python3")
            .arg("-c")
            .arg("import docling; print(docling.__version__)")
            .output()
            .context("spawning python3 for Docling health probe")?;

        if !output.status.success() {
            return Ok(None);
        }

        let version = String::from_utf8(output.stdout)
            .context("decoding Docling version probe output")?
            .trim()
            .to_owned();
        if version.is_empty() {
            return Ok(None);
        }

        Ok(Some(version))
    }

    fn ensure_runtime_ready(&self) -> Result<()> {
        if !self.wrapper_exists()? {
            return extraction_error(
                EXTRACTION_ENGINE_UNAVAILABLE,
                &format!(
                    "Docling wrapper script is unavailable at {}",
                    self.config.docling_wrapper_path.display()
                ),
            );
        }
        if !self.artifacts_dir_exists()? {
            return extraction_error(
                EXTRACTION_ENGINE_UNAVAILABLE,
                &format!(
                    "Docling model artifacts directory is unavailable at {}",
                    self.config.model_artifacts_path.display()
                ),
            );
        }
        if Self::probe_docling_version()?.is_none() {
            return extraction_error(
                EXTRACTION_ENGINE_UNAVAILABLE,
                "Docling runtime probe failed or docling is not installed",
            );
        }
        Ok(())
    }
}

const fn strategy_name(strategy: ProcessingStrategy) -> &'static str {
    match strategy {
        ProcessingStrategy::Standard => "standard",
        ProcessingStrategy::ForceFullPageOcr => "force_full_page_ocr",
        ProcessingStrategy::FastTableStructure => "fast_table_structure",
    }
}

fn scratch_dir() -> PathBuf {
    let mut dir = std::env::temp_dir();
    dir.push(OsString::from(format!("daemon-pdf-{}", Uuid::new_v4())));
    dir
}

fn extraction_error<T>(code: &str, detail: &str) -> Result<T> {
    bail!("{code}: {detail}");
}

fn validate_source_pdf_bytes(bytes: &[u8]) -> Result<()> {
    if bytes.is_empty() || bytes.iter().all(u8::is_ascii_whitespace) {
        return extraction_error(EXTRACTION_EMPTY_PDF, "input PDF bytes are empty");
    }
    if is_encrypted_pdf(bytes) {
        return extraction_error(
            EXTRACTION_ENCRYPTED_PDF,
            "input PDF appears to be encrypted",
        );
    }
    Ok(())
}

fn is_encrypted_pdf(bytes: &[u8]) -> bool {
    bytes
        .windows("/Encrypt".len())
        .any(|window| window == b"/Encrypt")
}

fn classify_failed_wrapper_status(status: std::process::ExitStatus, stderr: &str) -> &'static str {
    if status.code() == Some(137) || status.signal() == Some(9) {
        return EXTRACTION_OOM;
    }
    if stderr.contains("No such file or directory") || stderr.contains("docling import unavailable")
    {
        return EXTRACTION_ENGINE_UNAVAILABLE;
    }
    EXTRACTION_CORRUPT_PDF
}

#[must_use]
pub fn compute_source_hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub fn lookup_reusable_extraction(
    conn: &Connection,
    source_content_hash: &str,
    extraction_engine_version: &str,
    processing_strategy: ProcessingStrategy,
) -> Result<Option<ReusableStructuralExtraction>> {
    conn.query_row(
        "\
        SELECT id, source_content_hash, extraction_engine_version, processing_strategy,
               page_count, document_title, document_language, document_role,
               extraction_policy_version, expected_overlays_json,
               extracted_markdown, extracted_sections_json, extracted_tables_json,
               extracted_metadata_json, table_count, section_count, image_count,
               has_ocr_content, empty_page_count, quality_warnings_json, owner_id
        FROM document_extractions
        WHERE source_content_hash = ?1
          AND extraction_engine = 'docling'
          AND extraction_engine_version = ?2
          AND processing_strategy = ?3
          AND is_current = 1
        ORDER BY datetime(updated_at) DESC, datetime(created_at) DESC
        LIMIT 1
        ",
        params![
            source_content_hash,
            extraction_engine_version,
            strategy_name(processing_strategy),
        ],
        |row| {
            Ok(ReusableStructuralExtraction {
                extraction_id: row.get(0)?,
                source_content_hash: row.get(1)?,
                extraction_engine_version: row.get(2)?,
                processing_strategy,
                page_count: row.get(4)?,
                document_title: row.get(5)?,
                document_language: row.get(6)?,
                document_role: row.get(7)?,
                extraction_policy_version: row.get(8)?,
                expected_overlays_json: row.get(9)?,
                extracted_markdown: row.get(10)?,
                extracted_sections_json: row.get(11)?,
                extracted_tables_json: row.get(12)?,
                extracted_metadata_json: row.get(13)?,
                table_count: row.get(14)?,
                section_count: row.get(15)?,
                image_count: row.get(16)?,
                has_ocr_content: row.get::<_, i64>(17)? == 1,
                empty_page_count: row.get(18)?,
                quality_warnings_json: row.get(19)?,
                owner_id: row.get(20)?,
            })
        },
    )
    .optional()
    .context("looking up reusable structural extraction")
}

pub fn persist_structural_extraction(
    conn: &Connection,
    row: &PersistStructuralExtractionRow,
) -> Result<()> {
    let tx = Transaction::new_unchecked(conn, TransactionBehavior::Immediate)
        .context("starting structural extraction persistence transaction")?;
    supersede_current_structural_row(&tx, row)?;
    insert_structural_extraction_row(&tx, row)?;
    tx.commit()
        .context("committing structural extraction persistence transaction")?;
    Ok(())
}

#[must_use]
pub fn clone_reused_extraction(
    existing: &ReusableStructuralExtraction,
    new_raw_document_id: &str,
    new_process_id: &str,
    new_notice_id: Option<&str>,
    new_document_set_id: Option<&str>,
    new_stage_run_id: &str,
) -> PersistStructuralExtractionRow {
    PersistStructuralExtractionRow {
        extraction_id: Uuid::new_v4().to_string(),
        raw_document_id: new_raw_document_id.to_owned(),
        process_id: new_process_id.to_owned(),
        notice_id: new_notice_id.map(str::to_owned),
        document_set_id: new_document_set_id.map(str::to_owned),
        source_content_hash: existing.source_content_hash.clone(),
        extraction_engine_version: existing.extraction_engine_version.clone(),
        processing_strategy: existing.processing_strategy,
        page_count: existing.page_count,
        document_title: existing.document_title.clone(),
        document_language: existing.document_language.clone(),
        document_role: existing.document_role.clone(),
        extraction_policy_version: existing.extraction_policy_version.clone(),
        expected_overlays_json: existing.expected_overlays_json.clone(),
        extracted_markdown: existing.extracted_markdown.clone(),
        extracted_sections_json: existing.extracted_sections_json.clone(),
        extracted_tables_json: existing.extracted_tables_json.clone(),
        extracted_metadata_json: existing.extracted_metadata_json.clone(),
        table_count: existing.table_count,
        section_count: existing.section_count,
        image_count: existing.image_count,
        has_ocr_content: existing.has_ocr_content,
        empty_page_count: existing.empty_page_count,
        quality_warnings_json: existing.quality_warnings_json.clone(),
        semantic_extraction_state: "pending".to_owned(),
        owner_id: existing.owner_id.clone(),
        stage_run_id: new_stage_run_id.to_owned(),
    }
}

fn supersede_current_structural_row(
    tx: &Transaction<'_>,
    row: &PersistStructuralExtractionRow,
) -> Result<()> {
    tx.execute(
        "\
        UPDATE document_extractions
        SET is_current = 0,
            superseded_by_extraction_id = ?1,
            updated_at = CURRENT_TIMESTAMP
        WHERE raw_document_id = ?2
          AND extraction_engine = 'docling'
          AND extraction_engine_version = ?3
          AND processing_strategy = ?4
          AND is_current = 1
          AND id <> ?1
        ",
        params![
            row.extraction_id,
            row.raw_document_id,
            row.extraction_engine_version,
            strategy_name(row.processing_strategy),
        ],
    )
    .context("superseding current structural extraction rows")?;
    Ok(())
}

fn insert_structural_extraction_row(
    tx: &Transaction<'_>,
    row: &PersistStructuralExtractionRow,
) -> Result<()> {
    tx.execute(
        "\
        INSERT INTO document_extractions (
          id, raw_document_id, process_id, notice_id, document_set_id, source_content_hash,
          extraction_engine, extraction_engine_version, processing_strategy, is_current,
          page_count, document_title, document_language, document_role, extraction_policy_version,
          expected_overlays_json, extracted_markdown, extracted_sections_json, extracted_tables_json,
          extracted_metadata_json, table_count, section_count, image_count, has_ocr_content,
          empty_page_count, quality_warnings_json, semantic_extraction_state, owner_id,
          row_version, last_local_seq, stage_run_id
        ) VALUES (
          ?1, ?2, ?3, ?4, ?5, ?6, 'docling', ?7, ?8, 1, ?9, ?10, ?11, ?12, ?13, ?14, ?15,
          ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, 1, NULL, ?27
        )
        ",
        params![
            row.extraction_id,
            row.raw_document_id,
            row.process_id,
            row.notice_id,
            row.document_set_id,
            row.source_content_hash,
            row.extraction_engine_version,
            strategy_name(row.processing_strategy),
            i64::from(row.page_count),
            row.document_title,
            row.document_language,
            row.document_role,
            row.extraction_policy_version,
            row.expected_overlays_json,
            row.extracted_markdown,
            row.extracted_sections_json,
            row.extracted_tables_json,
            row.extracted_metadata_json,
            i64::from(row.table_count),
            i64::from(row.section_count),
            i64::from(row.image_count),
            i64::from(u8::from(row.has_ocr_content)),
            i64::from(row.empty_page_count),
            row.quality_warnings_json,
            row.semantic_extraction_state,
            row.owner_id,
            row.stage_run_id,
        ],
    )
    .context("inserting structural extraction row")?;
    Ok(())
}

fn value_u32(payload: &Value, field: &str) -> Result<u32> {
    let raw = payload
        .get(field)
        .and_then(Value::as_u64)
        .ok_or_else(|| anyhow!("Docling output missing `{field}`"))?;
    u32::try_from(raw).with_context(|| format!("converting `{field}` to u32"))
}

fn parse_quality(payload: &Value) -> Result<ExtractionQuality> {
    let warnings = payload
        .get("warnings")
        .and_then(Value::as_array)
        .map(|entries| {
            entries
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    Ok(ExtractionQuality {
        table_count: value_u32(payload, "table_count")?,
        section_count: value_u32(payload, "section_count")?,
        image_count: value_u32(payload, "image_count")?,
        empty_page_count: value_u32(payload, "empty_page_count")?,
        has_ocr_content: payload
            .get("has_ocr_content")
            .and_then(Value::as_bool)
            .ok_or_else(|| anyhow!("Docling quality missing `has_ocr_content`"))?,
        warnings,
    })
}

fn parse_extraction_result(
    payload: &Value,
    source_hash: String,
    processing_strategy: ProcessingStrategy,
) -> Result<ExtractionResult> {
    let page_count = value_u32(payload, "page_count")?;
    let extracted_markdown = payload
        .get("markdown")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("Docling output missing `markdown`"))?
        .to_owned();
    let sections_json = payload
        .get("sections")
        .cloned()
        .ok_or_else(|| anyhow!("Docling output missing `sections`"))?;
    let tables_json = payload
        .get("tables")
        .cloned()
        .ok_or_else(|| anyhow!("Docling output missing `tables`"))?;
    let metadata_json = payload
        .get("metadata")
        .cloned()
        .ok_or_else(|| anyhow!("Docling output missing `metadata`"))?;
    let quality = parse_quality(
        payload
            .get("quality")
            .ok_or_else(|| anyhow!("Docling output missing `quality`"))?,
    )?;
    let engine_version = metadata_json
        .get("docling_version")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("Docling metadata missing `docling_version`"))?
        .to_owned();

    Ok(ExtractionResult {
        extraction_id: Uuid::new_v4().to_string(),
        source_hash,
        page_count,
        extracted_markdown,
        sections_json,
        tables_json,
        metadata_json,
        quality,
        engine_version,
        processing_strategy,
    })
}

fn validate_extraction_result(result: &ExtractionResult) -> Result<()> {
    let zero_metrics = result.page_count == 0
        || (result.quality.table_count == 0
            && result.quality.section_count == 0
            && result.quality.image_count == 0
            && result.quality.empty_page_count == 0
            && result.extracted_markdown.trim().is_empty());
    if zero_metrics {
        return extraction_error(
            EXTRACTION_EMPTY_PDF,
            "Docling returned zero extraction metrics for the input PDF",
        );
    }
    Ok(())
}

async fn terminate_process_group(child: &mut Child, grace_period: Duration) -> Result<()> {
    let Some(pid_u32) = child.id() else {
        return Ok(());
    };
    let pid_raw = i32::try_from(pid_u32).context("converting child pid to i32")?;
    let process_group = Pid::from_raw(pid_raw);

    let _ = killpg(process_group, Signal::SIGTERM);
    if let Ok(status) = timeout(grace_period, child.wait()).await {
        status.context("waiting for Docling wrapper after SIGTERM")?;
    } else {
        let _ = killpg(process_group, Signal::SIGKILL);
        child
            .wait()
            .await
            .context("waiting for Docling wrapper after SIGKILL")?;
    }
    Ok(())
}
