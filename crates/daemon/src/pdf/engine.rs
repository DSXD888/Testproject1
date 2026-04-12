#![forbid(unsafe_code)]

use std::fs;
use std::io::ErrorKind;
use std::path::PathBuf;

use anyhow::{Result, anyhow};

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
        let wrapper_available = match fs::metadata(&self.config.docling_wrapper_path) {
            Ok(metadata) => metadata.is_file(),
            Err(error) if error.kind() == ErrorKind::NotFound => false,
            Err(error) => return Err(error.into()),
        };

        let artifacts_available = match fs::metadata(&self.config.model_artifacts_path) {
            Ok(metadata) => metadata.is_dir(),
            Err(error) if error.kind() == ErrorKind::NotFound => false,
            Err(error) => return Err(error.into()),
        };

        Ok(wrapper_available && artifacts_available)
    }

    pub async fn extract(&self, _input: PdfInput) -> Result<ExtractionResult> {
        std::future::ready(()).await;
        Err(anyhow!(
            "pdf extraction engine scaffold only; subprocess execution lands in testproject1-vt96.2.2"
        ))
    }

    pub async fn extract_batch(&self, inputs: Vec<PdfInput>) -> Vec<Result<ExtractionResult>> {
        let mut results = Vec::with_capacity(inputs.len());
        for input in inputs {
            results.push(self.extract(input).await);
        }
        results
    }
}
