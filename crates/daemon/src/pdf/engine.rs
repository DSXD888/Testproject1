#![forbid(unsafe_code)]

use std::ffi::OsString;
use std::fs;
use std::io::ErrorKind;
use std::path::PathBuf;
use std::process::{Command as StdCommand, Stdio};

use anyhow::{Context, Result, anyhow, bail};
use nix::sys::signal::{Signal, killpg};
use nix::unistd::Pid;
use serde_json::Value;
use sha2::{Digest, Sha256};
use tokio::fs::{self as tokio_fs, File};
use tokio::process::{Child, Command};
use tokio::time::{Duration, timeout};
use uuid::Uuid;

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
            bail!(
                "Docling wrapper timed out after {}s; stderr captured at {}",
                self.config.wall_clock_timeout_secs,
                stderr_path.display()
            );
        };

        if !status.success() {
            let stderr = tokio_fs::read_to_string(&stderr_path)
                .await
                .unwrap_or_default();
            bail!(
                "Docling wrapper failed with status {:?}: {}",
                status.code(),
                stderr.trim()
            );
        }

        let source_bytes = tokio_fs::read(&input.source_path)
            .await
            .with_context(|| format!("reading PDF bytes from {}", input.source_path.display()))?;
        let source_hash = format!("{:x}", Sha256::digest(&source_bytes));

        let output_text = tokio_fs::read_to_string(&output_path)
            .await
            .with_context(|| format!("reading Docling output {}", output_path.display()))?;
        let payload: Value =
            serde_json::from_str(&output_text).context("parsing Docling wrapper JSON output")?;

        parse_extraction_result(&payload, source_hash, input.processing_strategy)
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
            bail!(
                "Docling wrapper script is unavailable at {}",
                self.config.docling_wrapper_path.display()
            );
        }
        if !self.artifacts_dir_exists()? {
            bail!(
                "Docling model artifacts directory is unavailable at {}",
                self.config.model_artifacts_path.display()
            );
        }
        if Self::probe_docling_version()?.is_none() {
            bail!("Docling runtime probe failed or docling is not installed");
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
