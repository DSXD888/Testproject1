#![forbid(unsafe_code)]

use std::collections::BTreeSet;
use std::fmt::Write as _;

use anyhow::{Result, ensure};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::pdf::engine::ExtractionResult;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentSynopsis {
    pub section_tree: serde_json::Value,
    pub table_headers: Vec<String>,
    pub page_anchors: Vec<u32>,
    pub estimated_tokens: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticChunk {
    pub chunk_index: usize,
    pub plan_hash: String,
    pub page_start: u32,
    pub page_end: u32,
    pub section_refs: Vec<String>,
    pub markdown: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChunkPlan {
    pub plan_hash: String,
    pub synopsis: DocumentSynopsis,
    pub chunks: Vec<SemanticChunk>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SectionBlock {
    section_ref: String,
    page_start: u32,
    page_end: u32,
    markdown: String,
    estimated_tokens: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TableMeta {
    table_key: String,
    page_start: u32,
    page_end: u32,
    header_summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ChunkDraft {
    page_start: u32,
    page_end: u32,
    section_refs: Vec<String>,
    markdown: String,
}

pub fn build_document_synopsis(result: &ExtractionResult) -> DocumentSynopsis {
    let tables = table_metadata(result);
    let mut page_anchors = BTreeSet::from([1_u32, result.page_count.max(1)]);
    for section in section_blocks(result) {
        page_anchors.insert(section.page_start);
        page_anchors.insert(section.page_end);
    }
    for table in &tables {
        page_anchors.insert(table.page_start);
        page_anchors.insert(table.page_end);
    }

    DocumentSynopsis {
        section_tree: result.sections_json.clone(),
        table_headers: tables
            .iter()
            .map(|table| {
                format!(
                    "{}|pages:{}-{}|{}",
                    table.table_key, table.page_start, table.page_end, table.header_summary
                )
            })
            .collect(),
        page_anchors: page_anchors.into_iter().collect(),
        estimated_tokens: estimate_tokens(&result.extracted_markdown),
    }
}

pub fn build_chunk_plan(result: &ExtractionResult, token_budget: usize) -> Result<ChunkPlan> {
    ensure!(token_budget > 0, "token_budget must be greater than zero");

    let synopsis = build_document_synopsis(result);
    let sections = section_blocks(result);
    let tables = table_metadata(result);

    let drafts = if synopsis.estimated_tokens <= token_budget || sections.len() <= 1 {
        vec![build_full_document_chunk(result, &sections, &tables)]
    } else {
        let mut drafts = Vec::new();
        let mut current = Vec::new();
        let mut current_tokens = 0_usize;

        for section in sections {
            let block_tokens = section.estimated_tokens.max(1);
            if !current.is_empty() {
                let current_end = current
                    .last()
                    .map_or(section.page_end, |block: &SectionBlock| block.page_end);
                let crosses_table_boundary = tables.iter().any(|table| {
                    table.page_start <= current_end
                        && table.page_end > current_end
                        && table.page_end >= section.page_start
                });
                if current_tokens + block_tokens > token_budget && !crosses_table_boundary {
                    drafts.push(build_chunk_draft(&current, &tables));
                    current.clear();
                    current_tokens = 0;
                }
            }
            current_tokens += block_tokens;
            current.push(section);
        }

        if !current.is_empty() {
            drafts.push(build_chunk_draft(&current, &tables));
        }
        drafts
    };

    let plan_hash = compute_plan_hash(&synopsis, &drafts);
    let chunks = drafts
        .into_iter()
        .enumerate()
        .map(|(chunk_index, draft)| SemanticChunk {
            chunk_index,
            plan_hash: plan_hash.clone(),
            page_start: draft.page_start,
            page_end: draft.page_end,
            section_refs: draft.section_refs,
            markdown: draft.markdown,
        })
        .collect();

    Ok(ChunkPlan {
        plan_hash,
        synopsis,
        chunks,
    })
}

fn build_full_document_chunk(
    result: &ExtractionResult,
    sections: &[SectionBlock],
    tables: &[TableMeta],
) -> ChunkDraft {
    let mut draft = build_chunk_draft(sections, tables);
    draft.page_start = 1;
    draft.page_end = result.page_count.max(1);
    draft.markdown.clone_from(&result.extracted_markdown);
    draft
}

fn section_blocks(result: &ExtractionResult) -> Vec<SectionBlock> {
    let raw_sections = result
        .sections_json
        .as_array()
        .cloned()
        .unwrap_or_else(|| vec![json!({})]);
    let located = locate_section_boundaries(&result.extracted_markdown, &raw_sections);

    located
        .into_iter()
        .enumerate()
        .map(|(index, (section, start, end))| {
            let section_ref = section
                .get("section_ref")
                .and_then(Value::as_str)
                .map_or_else(|| format!("section-{index}"), str::to_owned);
            let page_start = section
                .get("page_start")
                .and_then(Value::as_u64)
                .and_then(|value| u32::try_from(value).ok())
                .unwrap_or(1);
            let page_end = section
                .get("page_end")
                .and_then(Value::as_u64)
                .and_then(|value| u32::try_from(value).ok())
                .unwrap_or(page_start);
            let markdown = result.extracted_markdown[start..end].trim().to_owned();
            let markdown = if markdown.is_empty() {
                section
                    .get("heading")
                    .and_then(Value::as_str)
                    .map_or_else(|| result.extracted_markdown.clone(), str::to_owned)
            } else {
                markdown
            };

            SectionBlock {
                section_ref,
                page_start,
                page_end,
                estimated_tokens: estimate_tokens(&markdown),
                markdown,
            }
        })
        .collect()
}

fn locate_section_boundaries(markdown: &str, sections: &[Value]) -> Vec<(Value, usize, usize)> {
    if sections.is_empty() {
        return vec![(json!({}), 0, markdown.len())];
    }

    let mut offsets = Vec::new();
    let mut search_from = 0_usize;
    for (index, section) in sections.iter().enumerate() {
        let heading = section
            .get("heading")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let start = find_heading_offset(markdown, heading, search_from).unwrap_or(search_from);
        let end = sections
            .get(index + 1)
            .and_then(|next| {
                next.get("heading")
                    .and_then(Value::as_str)
                    .and_then(|heading| find_heading_offset(markdown, heading, start + 1))
            })
            .unwrap_or(markdown.len());
        search_from = end;
        offsets.push((section.clone(), start, end.max(start)));
    }
    offsets
}

fn find_heading_offset(markdown: &str, heading: &str, from: usize) -> Option<usize> {
    if heading.is_empty() || from >= markdown.len() {
        return None;
    }

    let patterns = [
        format!("## {heading}"),
        format!("# {heading}"),
        heading.to_owned(),
    ];
    patterns.iter().find_map(|pattern| {
        markdown[from..]
            .find(pattern)
            .map(|offset| offset.saturating_add(from))
    })
}

fn table_metadata(result: &ExtractionResult) -> Vec<TableMeta> {
    result
        .tables_json
        .as_array()
        .into_iter()
        .flatten()
        .enumerate()
        .map(|(index, table)| {
            let table_key = table
                .get("table_key")
                .and_then(Value::as_str)
                .map_or_else(|| format!("table-{index}"), str::to_owned);
            let headers = table
                .get("headers")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join("|");
            let page_start = table
                .get("page_start")
                .and_then(Value::as_u64)
                .and_then(|value| u32::try_from(value).ok())
                .unwrap_or(1);
            let page_end = table
                .get("page_end")
                .and_then(Value::as_u64)
                .and_then(|value| u32::try_from(value).ok())
                .unwrap_or(page_start);
            TableMeta {
                table_key,
                page_start,
                page_end,
                header_summary: headers,
            }
        })
        .collect()
}

fn build_chunk_draft(sections: &[SectionBlock], tables: &[TableMeta]) -> ChunkDraft {
    let page_start = sections.first().map_or(1, |section| section.page_start);
    let page_end = sections
        .last()
        .map_or(page_start, |section| section.page_end);
    let mut section_refs = sections
        .iter()
        .map(|section| section.section_ref.clone())
        .collect::<Vec<_>>();
    let chunk_tables = tables
        .iter()
        .filter(|table| table.page_start <= page_end && table.page_end >= page_start)
        .collect::<Vec<_>>();
    for table in &chunk_tables {
        section_refs.push(format!(
            "table:{}:pages:{}-{}:headers:{}",
            table.table_key, table.page_start, table.page_end, table.header_summary
        ));
    }
    let mut markdown = sections
        .iter()
        .map(|section| section.markdown.as_str())
        .collect::<Vec<_>>()
        .join("\n\n");
    for table in chunk_tables
        .iter()
        .filter(|table| table.page_end > table.page_start)
    {
        let _ = write!(
            markdown,
            "\n\n<!-- merge-table:{} pages:{}-{} headers:{} -->",
            table.table_key, table.page_start, table.page_end, table.header_summary
        );
    }

    ChunkDraft {
        page_start,
        page_end,
        section_refs,
        markdown,
    }
}

fn estimate_tokens(markdown: &str) -> usize {
    let whitespace_tokens = markdown.split_whitespace().count();
    whitespace_tokens.max(markdown.len().div_ceil(4)).max(1)
}

fn compute_plan_hash(synopsis: &DocumentSynopsis, drafts: &[ChunkDraft]) -> String {
    let payload = json!({
        "synopsis": {
            "section_tree": synopsis.section_tree,
            "table_headers": synopsis.table_headers,
            "page_anchors": synopsis.page_anchors,
            "estimated_tokens": synopsis.estimated_tokens,
        },
        "chunks": drafts.iter().map(|chunk| json!({
            "page_start": chunk.page_start,
            "page_end": chunk.page_end,
            "section_refs": chunk.section_refs,
            "markdown": chunk.markdown,
        })).collect::<Vec<_>>(),
    });
    format!("{:x}", Sha256::digest(payload.to_string().as_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pdf::engine::{ExtractionQuality, ExtractionResult, ProcessingStrategy};

    fn short_result() -> ExtractionResult {
        ExtractionResult {
            extraction_id: "extract-short".to_owned(),
            source_hash: "hash-short".to_owned(),
            page_count: 2,
            extracted_markdown: "# Tender specification\n\n## Lot 1\nCompact requirement set.\n\n## Lot 2\nShort pricing notes.".to_owned(),
            sections_json: json!([
                {
                    "section_ref": "lot-1",
                    "heading": "Lot 1",
                    "page_start": 1,
                    "page_end": 1
                },
                {
                    "section_ref": "lot-2",
                    "heading": "Lot 2",
                    "page_start": 2,
                    "page_end": 2
                }
            ]),
            tables_json: json!([
                {
                    "table_key": "tbl-short",
                    "page_start": 2,
                    "page_end": 2,
                    "headers": ["Item", "Qty", "Unit"]
                }
            ]),
            metadata_json: json!({}),
            quality: ExtractionQuality {
                table_count: 1,
                section_count: 2,
                image_count: 0,
                empty_page_count: 0,
                has_ocr_content: false,
                warnings: Vec::new(),
            },
            engine_version: "2.85.0".to_owned(),
            processing_strategy: ProcessingStrategy::Standard,
        }
    }

    fn long_result() -> ExtractionResult {
        ExtractionResult {
            extraction_id: "extract-long".to_owned(),
            source_hash: "hash-long".to_owned(),
            page_count: 4,
            extracted_markdown: [
                "# Tender specification",
                "## Lot 1\n".repeat(20).as_str(),
                "## Lot 2\n".repeat(20).as_str(),
                "## Lot 3\n".repeat(20).as_str(),
                "## Lot 4\n".repeat(20).as_str(),
            ]
            .join("\n"),
            sections_json: json!([
                {
                    "section_ref": "lot-1",
                    "heading": "Lot 1",
                    "page_start": 1,
                    "page_end": 1
                },
                {
                    "section_ref": "lot-2",
                    "heading": "Lot 2",
                    "page_start": 2,
                    "page_end": 2
                },
                {
                    "section_ref": "lot-3",
                    "heading": "Lot 3",
                    "page_start": 3,
                    "page_end": 3
                },
                {
                    "section_ref": "lot-4",
                    "heading": "Lot 4",
                    "page_start": 4,
                    "page_end": 4
                }
            ]),
            tables_json: json!([]),
            metadata_json: json!({}),
            quality: ExtractionQuality {
                table_count: 0,
                section_count: 4,
                image_count: 0,
                empty_page_count: 0,
                has_ocr_content: false,
                warnings: Vec::new(),
            },
            engine_version: "2.85.0".to_owned(),
            processing_strategy: ProcessingStrategy::Standard,
        }
    }

    fn page_spanning_table_result() -> ExtractionResult {
        ExtractionResult {
            extraction_id: "extract-table".to_owned(),
            source_hash: "hash-table".to_owned(),
            page_count: 4,
            extracted_markdown: [
                "# Pricing schedule",
                "## Lot 1\nLine item table continues.\n",
                "## Lot 2\nRepeated pricing header.\n",
                "## Lot 3\nTrailing section.\n",
            ]
            .join("\n"),
            sections_json: json!([
                {
                    "section_ref": "lot-1",
                    "heading": "Lot 1",
                    "page_start": 1,
                    "page_end": 2
                },
                {
                    "section_ref": "lot-2",
                    "heading": "Lot 2",
                    "page_start": 3,
                    "page_end": 3
                },
                {
                    "section_ref": "lot-3",
                    "heading": "Lot 3",
                    "page_start": 4,
                    "page_end": 4
                }
            ]),
            tables_json: json!([
                {
                    "table_key": "tbl-pricing-1",
                    "page_start": 2,
                    "page_end": 3,
                    "headers": ["Item", "Qty", "Rate"]
                }
            ]),
            metadata_json: json!({}),
            quality: ExtractionQuality {
                table_count: 1,
                section_count: 3,
                image_count: 0,
                empty_page_count: 0,
                has_ocr_content: false,
                warnings: Vec::new(),
            },
            engine_version: "2.85.0".to_owned(),
            processing_strategy: ProcessingStrategy::Standard,
        }
    }

    #[test]
    fn test_synopsis_generation() {
        let synopsis = build_document_synopsis(&page_spanning_table_result());
        assert_eq!(synopsis.section_tree[0]["section_ref"], "lot-1");
        assert!(synopsis.table_headers.iter().any(|header| {
            header.contains("tbl-pricing-1") && header.contains("Item|Qty|Rate")
        }));
        assert_eq!(synopsis.page_anchors, vec![1, 2, 3, 4]);
        assert!(synopsis.estimated_tokens > 0);
    }

    #[test]
    fn test_chunk_plan_short_doc() -> Result<()> {
        let plan = build_chunk_plan(&short_result(), 200)?;
        assert_eq!(plan.chunks.len(), 1);
        assert_eq!(plan.chunks[0].page_start, 1);
        assert_eq!(plan.chunks[0].page_end, 2);
        assert_eq!(plan.chunks[0].markdown, short_result().extracted_markdown);
        Ok(())
    }

    #[test]
    fn test_chunk_plan_long_doc() -> Result<()> {
        let plan = build_chunk_plan(&long_result(), 30)?;
        assert!(plan.chunks.len() > 1);
        assert_eq!(plan.chunks[0].chunk_index, 0);
        assert_eq!(plan.chunks[0].page_start, 1);
        assert!(
            plan.chunks
                .iter()
                .all(|chunk| chunk.plan_hash == plan.plan_hash)
        );
        Ok(())
    }

    #[test]
    fn test_chunk_plan_determinism() -> Result<()> {
        let result = page_spanning_table_result();
        let first = build_chunk_plan(&result, 25)?;
        let second = build_chunk_plan(&result, 25)?;
        assert_eq!(first, second);
        Ok(())
    }

    #[test]
    fn test_page_spanning_table_keeps_merge_metadata() -> Result<()> {
        let plan = build_chunk_plan(&page_spanning_table_result(), 20)?;
        assert!(plan.chunks.iter().any(|chunk| {
            chunk.page_start <= 2
                && chunk.page_end >= 3
                && chunk
                    .section_refs
                    .iter()
                    .any(|reference| reference.contains("table:tbl-pricing-1"))
                && chunk.markdown.contains("merge-table:tbl-pricing-1")
        }));
        Ok(())
    }
}
