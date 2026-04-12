#![forbid(unsafe_code)]
#![allow(dead_code)]

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

pub const DEFAULT_DOCLING_VERSION: &str = "2.85.0";
pub const DEFAULT_PROCESSING_STRATEGY: &str = "standard";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExtractionQuality {
    pub table_count: u32,
    pub section_count: u32,
    pub image_count: u32,
    pub empty_page_count: u32,
    pub has_ocr_content: bool,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExtractionResult {
    pub extraction_id: String,
    pub source_hash: String,
    pub page_count: u32,
    pub markdown: String,
    pub sections: Value,
    pub tables: Value,
    pub metadata: Value,
    pub extraction_engine: String,
    pub extraction_version: String,
    pub processing_strategy: String,
    pub quality: ExtractionQuality,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MockDocling {
    pub exit_code: i32,
    pub stderr: String,
    pub output: Value,
}

impl Default for MockDocling {
    fn default() -> Self {
        Self::success()
    }
}

impl MockDocling {
    #[must_use]
    pub fn success() -> Self {
        Self {
            exit_code: 0,
            stderr: String::new(),
            output: sample_docling_output(),
        }
    }

    #[must_use]
    pub fn import_error() -> Self {
        Self {
            exit_code: 3,
            stderr: "docling import unavailable".to_owned(),
            output: json!({}),
        }
    }

    #[must_use]
    pub fn with_output(output: Value) -> Self {
        Self {
            output,
            ..Self::success()
        }
    }

    pub fn run(&self, output_path: &Path) -> Result<i32> {
        if self.exit_code == 0 {
            fs::write(output_path, serde_json::to_vec_pretty(&self.output)?).with_context(
                || format!("writing mock Docling output to {}", output_path.display()),
            )?;
        }
        Ok(self.exit_code)
    }
}

#[must_use]
pub fn sample_pdf_bytes() -> Vec<u8> {
    b"%PDF-1.4
1 0 obj
<< /Type /Catalog /Pages 2 0 R >>
endobj
2 0 obj
<< /Type /Pages /Count 1 /Kids [3 0 R] >>
endobj
3 0 obj
<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 144] >>
endobj
trailer
<< /Root 1 0 R >>
%%EOF
"
    .to_vec()
}

#[must_use]
pub fn sample_docling_output() -> Value {
    json!({
        "markdown": "# Tender specification\n\n## Lot 1\nStainless steel storage cabinets compliant with BS EN 16121.\n\n## Pricing schedule\n| Item | Qty | Unit |\n| Cabinets | 12 | each |",
        "sections": [
            {
                "section_ref": "sec-1",
                "heading": "Tender specification",
                "page_start": 1,
                "page_end": 1
            },
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
                "page_end": 1,
                "headers": ["Item", "Qty", "Unit"],
                "rows": [
                    ["Cabinets", "12", "each"]
                ]
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
            "section_count": 2,
            "image_count": 0,
            "empty_page_count": 0,
            "has_ocr_content": false,
            "warnings": []
        }
    })
}

#[must_use]
pub fn sample_extraction_result() -> ExtractionResult {
    let output = sample_docling_output();
    let pdf_bytes = sample_pdf_bytes();
    ExtractionResult {
        extraction_id: "document-extraction-sample".to_owned(),
        source_hash: format!("{:x}", Sha256::digest(&pdf_bytes)),
        page_count: 1,
        markdown: output["markdown"].as_str().unwrap_or_default().to_owned(),
        sections: output["sections"].clone(),
        tables: output["tables"].clone(),
        metadata: output["metadata"].clone(),
        extraction_engine: "docling".to_owned(),
        extraction_version: DEFAULT_DOCLING_VERSION.to_owned(),
        processing_strategy: DEFAULT_PROCESSING_STRATEGY.to_owned(),
        quality: ExtractionQuality {
            table_count: 1,
            section_count: 2,
            image_count: 0,
            empty_page_count: 0,
            has_ocr_content: false,
            warnings: Vec::new(),
        },
    }
}

#[must_use]
pub fn sample_semantic_result(skill_id: &str) -> Value {
    match skill_id {
        "pdf-extraction/tender-specification" => sample_tender_spec_semantic_result(),
        "pdf-extraction/pricing-schedule" => sample_pricing_schedule_semantic_result(),
        "pdf-extraction/bill-of-quantities" => sample_boq_semantic_result(),
        _ => sample_base_semantic_result(),
    }
}

fn sample_tender_spec_semantic_result() -> Value {
    json!({
        "lots": [
            {
                "lot_number": "1",
                "title": "Storage cabinets",
                "description": "Supply and delivery of stainless steel storage cabinets.",
                "requirements": [
                    "Corrosion-resistant finish",
                    "Lockable doors"
                ],
                "evaluation_criteria": [
                    "Technical compliance",
                    "Delivery lead time"
                ],
                "cpv_codes": ["39150000"],
                "product_requirements": [
                    {
                        "candidate_key": "cabinet-std-01",
                        "name": "Stainless steel storage cabinet",
                        "quantity": 12,
                        "quantity_unit": "each"
                    }
                ],
                "technical_standards": ["BS EN 16121"],
                "evidence_anchor": {
                    "page_ref": "p1",
                    "anchor_text": "BS EN 16121"
                }
            }
        ],
        "product_requirement_seeds": [
            {
                "candidate_key": "cabinet-std-01",
                "lot_number": "1",
                "name": "Stainless steel storage cabinet",
                "specification_text": "BS EN 16121 compliant stainless steel cabinet",
                "quantity": 12,
                "quantity_unit": "each",
                "evidence_anchor": {
                    "page_ref": "p1",
                    "anchor_text": "Supply and delivery of stainless steel storage cabinets"
                }
            }
        ],
        "field_confidences": {
            "lots[0].title": 9400,
            "product_requirement_seeds[0].specification_text": 9100
        },
        "extraction_confidence_bps": 9200,
        "warnings": [],
        "evidence_anchors": [
            {
                "page_ref": "p1",
                "anchor_text": "Lot 1"
            }
        ],
        "extraction_metadata": {
            "extraction_scope": "full_document"
        }
    })
}

fn sample_pricing_schedule_semantic_result() -> Value {
    json!({
        "line_items": [
            {
                "item_number": "1",
                "description": "Stainless steel storage cabinet",
                "lot_number": "1",
                "quantity": 12,
                "unit": "each",
                "unit_rate_pence": 250_000,
                "total_value_pence": 3_000_000,
                "currency": "GBP",
                "source_table_key": "tbl-pricing-1",
                "repeated_header_normalized": true,
                "evidence_anchor": {
                    "page_ref": "p1",
                    "anchor_text": "Cabinets | 12 | each"
                }
            }
        ],
        "pricing_summary": {
            "total_value_pence": 3_000_000,
            "currency": "GBP",
            "lot_count": 1
        },
        "field_confidences": {
            "line_items[0].unit_rate_pence": 9300
        },
        "extraction_confidence_bps": 9150,
        "warnings": [],
        "evidence_anchors": [
            {
                "page_ref": "p1",
                "anchor_text": "Pricing schedule"
            }
        ],
        "extraction_metadata": {
            "extraction_scope": "full_document"
        }
    })
}

fn sample_boq_semantic_result() -> Value {
    json!({
        "items": [
            {
                "item_key": "boq-1",
                "item_ref": "1.1",
                "description": "Install stainless steel storage cabinet",
                "quantity": 12,
                "unit": "each",
                "unit_rate_pence": 250_000,
                "total_pence": 3_000_000,
                "lot_number": "1",
                "section_ref": "lot-1",
                "continuation_of_item_key": null,
                "evidence_anchor": {
                    "page_ref": "p1",
                    "anchor_text": "Install stainless steel storage cabinet"
                }
            }
        ],
        "summary": {
            "item_count": 1,
            "total_value_pence": 3_000_000
        },
        "field_confidences": {
            "items[0].item_key": 9500
        },
        "extraction_confidence_bps": 9100,
        "warnings": [],
        "evidence_anchors": [
            {
                "page_ref": "p1",
                "anchor_text": "Lot 1"
            }
        ],
        "extraction_metadata": {
            "extraction_scope": "full_document"
        }
    })
}

fn sample_base_semantic_result() -> Value {
    json!({
        "extracted_entities": [
            {
                "entity_type": "requirement",
                "entity_key": "req-1",
                "summary": "Stainless steel storage cabinets compliant with BS EN 16121.",
                "confidence_bps": 9200,
                "evidence_anchors": [
                    {
                        "page_ref": "p1",
                        "anchor_text": "BS EN 16121"
                    }
                ],
                "attributes": {
                    "lot_number": "1"
                }
            }
        ],
        "document_classification": {
            "document_role": "technical_specification",
            "classification_confidence_bps": 9300,
            "target_domain": "generic",
            "routing_rationale": "Contains product requirements and technical standards."
        },
        "extraction_warnings": [],
        "extraction_metadata": {
            "target_domain": "generic",
            "extraction_scope": "full_document",
            "source_markdown_hash": "sample-markdown-hash",
            "policy_version": "2026.04.12",
            "items_examined": 1
        }
    })
}
