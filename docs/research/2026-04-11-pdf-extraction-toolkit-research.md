# PDF Extraction Toolkit — Research & Design

> Research document covering the gap analysis, tool evaluation, recommended architecture, database schema implications, and reusable module design for PDF data extraction in the procurement intelligence pipeline.

**Date:** 2026-04-11
**Status:** DRAFT — awaiting owner review
**Trigger:** QA of L1-L3 revealed that most valuable contract data (specifications, pricing schedules, lot breakdowns, bills of quantities) lives inside PDF documents attached to procurement notices, not in the API-surface metadata the pipeline currently ingests. The current pipeline builds product candidates from thin notice summaries, producing low-quality extraction. A reusable PDF extraction toolkit is needed.

---

## Table of Contents

1. [Gap Analysis](#1-gap-analysis)
2. [Tool Evaluation](#2-tool-evaluation)
3. [Recommended Architecture: Hybrid Docling + Claude Skill](#3-recommended-architecture)
4. [Reusable Module Design](#4-reusable-module-design)
5. [Database Schema Implications](#5-database-schema-implications)
6. [Pipeline Integration](#6-pipeline-integration)
7. [Cost Model](#7-cost-model)
8. [Future Layer Applicability](#8-future-layer-applicability)
9. [Open Questions](#9-open-questions)

---

## 1. Gap Analysis

### What Exists Today

| Component | Status | Notes |
|-----------|--------|-------|
| `tender_document_sets` table | Schema exists (L0 migration 0001) | Stores document metadata: `documents_json`, `document_roles_json`, `completeness_state`, `document_set_hash`. No content extraction fields. |
| `raw_documents` table | Schema exists (L1 migration 0003) | Stores fetched content: `payload_json` OR `storage_path`, `content_type`. Can hold PDF bytes. No extraction output. |
| `document-acquisition` skill | Referenced in strategy/research docs | Not yet implemented. Mentioned in L0 §4.6.1 and the cross-layer stage-boundary docs as the PTY-interactive skill that should navigate procurement portals and download tender documents. No `skills/document-acquisition/` directory exists in the repo today. |
| PDF extraction infrastructure | **Does not exist** | No library, no skill, no schema, no daemon module for extracting structured data from PDF content. |
| Product candidate extraction | Fixture-materialized in L1-L3 | `product_candidates` are currently seeded from test fixtures, not computed by agent skills. When live extraction is built, the only available text inputs would be `contract_notices.summary` and `notice_lots.description` — thin API-surface text, not actual tender document content. |

### The Gap

The 15-stage pipeline (per L0 §3) positions `acquire_documents` at Stage 5, followed by `extract_lots` at Stage 6 and `extract_products` at Stage 7. The missing capability sits inside the document-acquisition boundary and must materialize before Stage 6 can operate on rich evidence:

```
Ingestion → Dedup → Relation Resolution → Preliminary Qualification
    → Acquire Documents (Stage 5)
    → [STRUCTURAL + SEMANTIC DOCUMENT EXTRACTION CAPABILITY — MISSING]
    → Extract Lots (Stage 6) → Extract Products (Stage 7) → Quoteability
    → Marketplace Search → Pricing → Customs → Margins → Decision
```

The pipeline has no document acquisition or extraction capability today — both the `document-acquisition` skill and PDF extraction infrastructure are unbuilt. When lot/product extraction is built for live operation, the only available inputs would be notice-level summary text (unless this toolkit is built first). The research doc explicitly warns against that approach:

> "The platform should attempt to acquire publicly accessible tender documents before product extraction. Without this, the extractor will turn vague notice summaries into garbage product candidates." (Research doc §5.7)

### What Procurement PDFs Contain

From QA of actual Contracts Finder and FTS notices, the valuable data in attached PDFs includes:

| Document Type | Key Data | Current Coverage |
|---------------|----------|-----------------|
| Specification PDFs | Detailed product requirements, technical standards (ISO/EN/BS), materials, dimensions, tolerances | None — only notice summary text |
| Pricing Schedules | Line-item pricing with quantities, unit rates, total values per lot | None — only estimated_value from API |
| Bills of Quantities (BoQ) | Itemized quantity breakdowns with unit measurements | None |
| Lot-specific documents | Per-lot terms, specific requirements, evaluation criteria | None — lots extracted from API metadata only |
| Framework agreements | Call-off terms, pricing mechanisms, volume commitments | None |
| Compliance matrices | Required certifications, standards, regulatory requirements | None |

---

## 2. Tool Evaluation

### Candidates Assessed

| Tool | Type | License | Accuracy Metric | Speed | Cost/Page |
|------|------|---------|----------------|-------|-----------|
| **Docling (IBM / Docling Project)** | Structural extraction (layout + tables + OCR) | MIT (codebase; model artifacts have separate licenses) | Official model-card / eval artifacts show strong TableFormer TEDS scores (for example ~90.1 complex / 93.6 overall on benchmark tables) | Hardware-dependent; benchmark locally on representative procurement PDFs | $0 (local compute) |
| **Claude native PDF** | Semantic extraction (LLM vision) | Proprietary API | 78% edit similarity (whole-doc); ~95% on legal/contract docs | Seconds/page | $0.05-0.06 |
| **Unstructured.io** | Structural extraction (layout detection) | Apache 2.0 (limited) | 75% on complex tables (TEDS) | 51s/page (hi_res) | $0 (local) |
| **LlamaParse** | Cloud NLP extraction | Proprietary | 81% ChrF++ (whole-doc robustness) | 6s constant | $0.003 |
| **Marker** | PDF-to-markdown (OCR + optional LLM) | Open source | No published benchmark | 25 pages/s batch (H100) | $0 (local) |
| **pdf-parse / PDF.js** | Basic text extraction (JS) | Open source | N/A — no structure | Fast | $0 |
| **Apache Tika** | Text/metadata extraction (Java) | Apache 2.0 | N/A — no structure | Moderate | $0 |

> **Metric caveat:** These accuracy numbers are NOT directly comparable. TEDS (Tree Edit Distance Similarity) measures table structure fidelity. Edit similarity and ChrF++ measure whole-document text reproduction. The takeaway is directional: purpose-built structural models outperform general LLM vision on raw table structure extraction, while LLMs excel at semantic interpretation. Also note that the earlier draft's headline Docling figures (`97.9% TEDS`, `114ms/page`, `v2.72.0`) could not be verified against the current official docs and releases on 2026-04-11, so they are intentionally replaced here with upstream-traceable claims.

### Detailed Assessment

#### Docling (IBM) — RECOMMENDED for structural extraction

- **What it does:** Purpose-built ML models (Heron for layout, TableFormer for tables, optional Granite VLM). Extracts text, tables, sections, headings, reading order, formulas, charts from PDFs.
- **Current upstream API surface:** `DocumentConverter`, `PdfPipelineOptions`, `TableStructureOptions`, OCR-engine options, `document_timeout`, and `PdfBackend.DOCLING_PARSE`. These are the knobs the daemon wrapper should target; the draft's `auto` / `hi_res` / `ocr_only` vocabulary is not the current Docling API.
- **Output format:** `DoclingDocument` → exportable as Markdown, HTML, DocTags, and lossless JSON.
- **Table handling:** Current official model-card / eval artifacts support the directional conclusion we need: TableFormer is materially better than plain text extraction for structured tables and is strong enough to justify a structural-first design. However, there is no official benchmark on UK procurement PDFs, so acceptance must rely on our own golden corpus rather than a borrowed headline metric.
- **Runtime:** Python 3.10+. Runs locally on macOS, Linux, Windows, and supports x86_64 + arm64. Apple-Silicon MLX is documented for Docling VLM paths; the standard PDF pipeline instead exposes accelerator and backend options that must be benchmarked on our workload.
- **Integrations:** MCP server available, LangChain/LlamaIndex/CrewAI adapters.
- **Maturity:** Current repo/docs show active releases and a latest release at `v2.85.0` (2026-04-07). Treat the installed package version as part of the cache key rather than hard-coding a specific upstream release in design text.
- **Strengths for this project:** (1) strong structural table fidelity for pricing schedules and BoQs, (2) runs locally so published tender packs do not need to leave the machine, (3) deterministic enough for version-keyed caching within the same engine/policy, (4) supports the broader document family we will eventually meet (`PDF`, `DOCX`, `XLSX`, `HTML`) even if this feature stays PDF-authoritative.
- **Weaknesses:** No procurement-domain semantics; needs a downstream mapping step. Real-world throughput is hardware- and document-mix-dependent. Page-spanning tables and low-quality scans remain must-test failure cases.

#### Claude Native PDF — RECOMMENDED for semantic mapping

- **What it does:** Processes PDFs by converting each page to image + extracted text, analyzes both via vision. Can be instructed to extract specific fields matching any target schema.
- **Constraints:** Max 600 pages/request, max 32MB, no encrypted PDFs. ~1,500-3,000 tokens/page (text + vision).
- **Benchmark:** 78% edit similarity on diverse documents (Applied AI, 800+ docs). The same benchmark reports ~95% accuracy on the "legal contracts" document category — procurement docs share structural similarities (formal language, numbered clauses, standardized layouts), though this is an indirect indicator, not a direct benchmark on UK procurement PDFs.
- **Cost reduction via hybrid:** When fed pre-extracted markdown (from Docling) instead of raw PDF pages, vision token cost is eliminated entirely. A 50-page document as pre-extracted markdown is ~25k-50k text tokens, vs ~75k-150k tokens (text + vision) for raw PDF. At Haiku tier, this translates to roughly $0.03-0.08 per document vs $0.50-1.00 for Haiku-on-raw-PDF, or $2.50-3.00 for Sonnet-on-raw-PDF. Per single semantic overlay, the reduction is **~30-100x vs Sonnet-on-raw-PDF, ~6-30x vs Haiku-on-raw-PDF**, depending on document complexity and output size. With multiple overlays per document (typical: 1-2), the effective savings are lower — see §7 Cost Model for full breakdown.
- **Strengths for this project:** (1) Unmatched semantic understanding — can interpret context, infer fields, handle ambiguity in contract language, (2) maps extracted content to any target schema in a single pass, (3) handles varied government PDF layouts without training, (4) can reason about lot breakdowns, cross-references, pricing structures, (5) native to existing daemon architecture.
- **Weaknesses:** Cost at scale, latency, occasional hallucination on numerical data (prices, dates, reference numbers). All mitigated by the hybrid approach where Docling handles the deterministic structural work.

#### Unstructured.io — NOT RECOMMENDED

- **Why not:** Slowest processing (51s/page for hi_res), worst complex table accuracy (75%), heaviest dependency burden (Poppler, Tesseract, Detectron2). The open-source version lacks VLM, fine-tuned OCR, advanced chunking, and image extraction. Does not justify the complexity over Docling.

#### LlamaParse — VIABLE FALLBACK / VALIDATION

- **Why consider:** $0.003/page with good quality metrics (81% ChrF++). Consistent speed. Could serve as A/B comparison against Docling or fallback for documents Docling handles poorly.
- **Why not primary:** Cloud-only — adds an external service dependency for data that could be processed locally. (Not a privacy concern since UK procurement PDFs are published under Open Government Licence, but adds latency, availability risk, and a billing relationship.) Structured output feature deprecated. Not open source.

#### pdf-parse / Tika — UNSUITABLE

- **Why not:** No table structure preservation at all. Tables become jumbled text lines. Useless for the core use case (pricing schedules, BoQs, spec tables).

---

## 3. Recommended Architecture

### Hybrid Two-Stage Pipeline: Docling + Claude Agent Skill

```
┌─────────────────────────────────────────────────────────┐
│                  PDF Extraction Toolkit                  │
│                                                         │
│  ┌─────────────────────┐   ┌──────────────────────────┐ │
│  │  Stage 1: Structure │   │  Stage 2: Semantics      │ │
│  │  (Deterministic)    │   │  (Agent Skill)           │ │
│  │                     │   │                          │ │
│  │  Docling engine     │   │  Claude headless_contract│ │
│  │  ─ Layout detection │   │  ─ Domain schema mapping │ │
│  │  ─ Table extraction │   │  ─ Cross-reference       │ │
│  │  ─ Section parsing  │   │    resolution            │ │
│  │  ─ Reading order    │   │  ─ Ambiguity handling    │ │
│  │  ─ Metadata extract │   │  ─ Field inference       │ │
│  │                     │   │  ─ Quality assessment    │ │
│  │  Input: PDF bytes   │   │                          │ │
│  │  Output: DocJSON    │   │  Input: DocJSON          │ │
│  │                     │   │  Output: result.json     │ │
│  │  Cost: $0           │   │  Cost: ~$0.001/page      │ │
│  │  Speed: benchmark-gated local compute │   │  Speed: ~1-3s/page       │ │
│  │  Deterministic: ~YES│   │  Deterministic: NO       │ │
│  └─────────────────────┘   └──────────────────────────┘ │
└─────────────────────────────────────────────────────────┘
```

### Why Two Stages

1. **Cost:** Raw PDF → Claude Sonnet costs ~$0.05-0.06/page (vision tokens). Pre-extracted markdown → Claude Haiku costs ~$0.001-0.002/page per skill run (text-only, no vision). With 2-3 semantic overlay runs per document, effective cost is ~$0.002-0.005/page. **~10-30x cheaper than Sonnet-on-raw-PDF.**
2. **Table accuracy:** Docling's purpose-built TableFormer path is specifically designed for structural table extraction. Current upstream artifacts justify the structural-first decision, but not a universal accuracy guarantee on UK procurement packs. For pricing schedules and BoQs where exact cell-to-column mapping matters, a dedicated table model is still the right first stage.
3. **Determinism:** Stage 1 is practically deterministic — same Docling version on the same hardware with the same input produces the same structural output. This enables caching, diffing, and re-extraction detection without re-running the agent. (Not bit-identical across hardware/versions due to ML floating-point, but stable enough for content-hash caching.)
4. **Separation of concerns:** Structural extraction has mature, purpose-built ML tooling (Docling, etc.) that handles well-formatted documents reliably. Agent reasoning should focus on semantic interpretation, not layout parsing — that's a better use of token budget and latency.
5. **Reusability:** Stage 1 output (structured DocJSON/markdown) is useful for any downstream consumer, not just Claude. Future skills, UI rendering, search indexing can all consume the same extraction.

### Why Not Agent-Only (Approach 1)

- Significantly more expensive per page — vision tokens for raw PDF vs text-only tokens for pre-extracted markdown (exact multiplier depends on model tier and document, but order-of-magnitude more expensive)
- LLM vision not purpose-built for structural table extraction — cell-to-column mapping errors on dense pricing schedules
- Non-deterministic — can't reliably diff or cache extractions
- Wastes agent reasoning capacity on mechanical layout parsing
- Rate limits constrain throughput for bulk re-extraction

### Why Not Structural-Only (No Agent)

- Cannot map extracted content to the procurement domain schema (lots, candidates, CPV codes, pricing structures)
- Cannot reason about ambiguity ("is this a lot boundary or a section header?")
- Cannot handle cross-references ("see Appendix B for pricing" → must link to correct table)
- Cannot infer missing fields from context
- Cannot produce confidence scores on extracted values

---

## 4. Reusable Module Design

### Design Principle: Capability, Not Stage

The PDF extraction toolkit is a **capability** available to any skill or pipeline stage, not a fixed pipeline step. Any daemon component that encounters a PDF can invoke extraction. This is important because PDFs appear at multiple points:

- Document acquisition (tender packs)
- Supplier certificates (compliance verification)
- Bid templates (response preparation, L6)
- Letters of intent (future layers)
- Framework call-off schedules

### Component 1: Structural Extraction Engine (Daemon Module)

**Location:** `crates/daemon/src/pdf/` (new Rust module)

**Responsibility:** Wrap Docling invocation as a subprocess. Accept PDF bytes or path, return structured extraction result.

**Interface sketch:**

```
PdfExtractionEngine
├── extract(input: PdfInput) -> ExtractionResult
│   ├── PdfInput { raw_document_id?, source_path | source_bytes, page_range?, processing_strategy }
│   └── ExtractionResult { 
│         extraction_id, 
│         source_hash,
│         page_count,
│         sections: Vec<Section>,
│         tables: Vec<Table>,
│         metadata: DocumentMetadata,
│         extraction_engine,     // "docling"
│         extraction_version,    // installed docling package version
│         processing_strategy,   // "standard" | "force_full_page_ocr" | "fast_table_structure"
│         quality: ExtractionQuality {
│           table_count, section_count, image_count,
│           empty_page_count, has_ocr_content, warnings
│         }
│       }
├── extract_batch(inputs: Vec<PdfInput>) -> Vec<ExtractionResult>
└── is_available() -> bool  // Docling installation probe
```

**Processing strategies** (daemon-owned abstraction mapped onto current Docling options):
- `standard` — `PdfBackend.DOCLING_PARSE` + `do_table_structure=true` + `TableStructureOptions.mode=accurate` + OCR enabled with automatic engine selection
- `force_full_page_ocr` — same as `standard`, but `ocr_options.force_full_page_ocr=true` for scanned or badly-layered PDFs
- `fast_table_structure` — same as `standard`, but `TableStructureOptions.mode=fast` for lower-cost retries or low-value docs

The feature keeps the non-negotiable `processing_strategy` cache dimension, but the strategy names must reflect actual Docling controls rather than invented upstream modes.

Note: Docling extracts ALL structure (text, tables, sections, headings) in one pass regardless of strategy. Domain-specific interpretation (e.g., "this table is a pricing schedule") belongs in Stage 2 (semantic mapping skills), not here. Do not confuse Docling processing strategies with document role classification.

**Subprocess management:**
- **Resolved design:** use direct subprocess spawning, not a long-lived sidecar and not an `AgentAdapter` execution mode. The daemon already supervises external binaries; Docling should fit that model instead of widening the adapter contract.
- One extraction job = one supervised Docling process group. Do **not** introduce a persistent process pool in the first implementation; startup overhead is preferable to leaked memory, poisoned Python state, or hung warm workers.
- Input/output via temp files in the job scratch directory. Capture stdout/stderr as artifacts for audit and error manifests.
- Enforce both Docling-side `document_timeout` and daemon-side wall-clock timeout. On timeout: terminate the process group, escalate to hard kill if needed, and persist a structured failure code.
- Health probe on daemon startup and before admitting queued extraction jobs: verify Python runtime, import `docling`, verify the configured wrapper entrypoint, and verify model/artifact availability.
- ML model download is an installation/bootstrap concern, not a lazy first-run surprise. The daemon should surface a clear unavailable state if models are missing instead of silently triggering a multi-GB download inside a production job.

**Caching:**
- The **reuse lookup** is keyed by the non-negotiable tuple `source_content_hash` + `extraction_engine_version` + `processing_strategy`.
- The **authoritative row identity** is still per `raw_document_id`, because the same bytes may legitimately appear in multiple notices/processes. The daemon may reuse an identical prior extraction by cloning persisted structural output into a new authoritative row for the target `raw_document_id` rather than rerunning Docling.
- Re-extraction only when source content changes, engine version changes, or processing strategy changes
- Cache invalidation on Docling version bump

### Component 2: Semantic Mapping Skills (Agent Skills)

**Location:** `skills/pdf-extraction/` (new skill family)

**Design:** One base skill + domain-specific overlay skills, following the existing validation/base pattern:

```
skills/pdf-extraction/
├── base/
│   ├── SKILL.md              # Base PDF semantic mapping skill
│   ├── result.schema.json    # Generic extraction result contract
│   └── canary.inputs.jsonl
├── tender-specification/
│   ├── SKILL.md              # Overlay: tender spec → lots + candidates + requirements
│   ├── result.schema.json    # Domain-specific result contract
│   ├── canary.inputs.jsonl
│   └── golden.assertions.yaml
├── pricing-schedule/
│   ├── SKILL.md              # Overlay: pricing → line items + rates + quantities
│   ├── result.schema.json
│   ├── canary.inputs.jsonl
│   └── golden.assertions.yaml
└── bill-of-quantities/
    ├── SKILL.md              # Overlay: BoQ → itemized quantities + units + rates
    ├── result.schema.json
    ├── canary.inputs.jsonl
    └── golden.assertions.yaml
```

**Skill metadata:**
```yaml
skill_id: pdf-extraction/tender-specification
skill_version: 1.0.0
interaction_profile: headless_contract
result_schema_version: 1.0.0
model_profile: structured_extract_low_cost
agent_cli: claude
side_effect_class: none
```

Key points:
- All overlay skills use `headless_contract` mode — no browser, pure reasoning over pre-extracted text
- `model_profile: structured_extract_low_cost` — Haiku tier is sufficient because the hard structural work is done by Docling
- Each overlay produces a domain-specific result contract
- Same promotion flow as existing skills (draft → candidate → canary → shadow → stable)
- Same validation framework (validation/base can validate these runs)

**Document role classification (prerequisite for skill routing):**

Before running a semantic skill, the system must know the document's role (specification, pricing_schedule, boq, etc.). Sources for this:
- **OCDS API metadata:** The `documentType` field in OCDS releases classifies documents (e.g., `tenderNotice`, `biddingDocuments`, `technicalSpecifications`). Available from Contracts Finder. Reliability varies.
- **Filename heuristics:** Common patterns like "Pricing Schedule", "Specification", "BoQ" in document filenames.
- **Structural content analysis:** If OCDS metadata and filename are both absent/ambiguous, the Docling output itself (section headings, table structures) can be classified by a lightweight heuristic or a cheap LLM call.

The `document_role` column on `document_extractions` stores the resolved classification. If the role is uncertain, the base pdf-extraction skill can do a classification pass before routing to a domain-specific overlay.

**Extraction policy (required, versioned):**

The aggregate `semantic_extraction_state` is only meaningful if the system knows which overlays were expected. Therefore the structural extraction row must carry a versioned extraction policy result:

- `technical_specification` / `invitation_to_tender` / `statement_of_requirements` → expected overlays: `["pdf-extraction/tender-specification"]`
- `pricing_schedule` → expected overlays: `["pdf-extraction/pricing-schedule"]`
- `boq` → expected overlays: `["pdf-extraction/bill-of-quantities"]`
- `mixed_tender_pack` / `combined_pack` → expected overlays chosen from structural signals (for example, rate/quantity tables + spec headings may produce more than one overlay)
- `administrative`, `cover_letter`, `terms_and_conditions` → expected overlays: `[]` and `semantic_extraction_state = 'skipped'`

Store both `document_role` and `expected_overlays_json` on `document_extractions`, plus `extraction_policy_version`. That is the contract the aggregate-state rollup must evaluate against.

**Large-document chunking (resolved design):**

- Build a lightweight document synopsis first: headings, section tree, table titles/headers, page anchors, and token estimates.
- If the rendered markdown fits inside the overlay budget, run the overlay once with `extraction_scope = full_document`.
- If it does not fit, create a deterministic chunk plan that prefers section boundaries and never splits a table unless the table itself exceeds the budget.
- Enqueue per-chunk semantic jobs using the same overlay skill plus a chunk-scoped input packet.
- After all chunk jobs complete, run a final merge invocation in `headless_contract` mode over the chunk outputs plus the synopsis only.
- Only the merged result becomes authoritative in `document_semantic_extractions`; chunk-local payloads stay in artifacts / job payloads for audit and retry, which avoids proliferating partial authoritative rows just to satisfy context limits.

**Input to semantic mapping skills:**
- The Docling-extracted document structure (markdown or JSON)
- Resolved document role classification
- Process context (buyer, title, CPV codes from `contract_notices`)
- Existing lot and candidate data (for cross-referencing)

**Output from semantic mapping skills:**
- Domain-specific structured data matching result.schema.json
- Confidence scores per extracted field
- Extraction warnings (ambiguous sections, missing expected data)
- Evidence anchors (page numbers, section references for auditability)

### Component 3: Extraction Orchestration (Job Runner Extension)

**Integration with existing Job Runner:**

New job kinds:
- `extract_document_structure` — runs Docling on a PDF, writes to `document_extractions`
- `extract_document_semantics` — runs a pdf-extraction skill on pre-extracted content, writes to domain tables

**Orchestration flow:**
```
Entry point A: document acquisition completes
    → tender_document_sets row written
    → enqueue extract_document_structure per PDF raw_document_id

Entry point B: standalone/manual extraction
    → raw_documents row already exists
    → enqueue extract_document_structure directly for raw_document_id

extract_document_structure
    → Docling subprocess runs
    → document_extractions row written
    → resolve document_role + expected_overlays_json
    → if expected_overlays_json == []:
         → mark semantic_extraction_state = skipped
      else:
         → enqueue extract_document_semantics jobs (one per overlay, or chunk plan + merge)

extract_document_semantics
    → Claude overlay skill runs in headless_contract mode
    → daemon validates result contract + OQG
    → daemon persistence layer writes:
         - document_semantic_extractions row
         - authoritative domain-table mutations
         - updated aggregate semantic_extraction_state on document_extractions
    → downstream lot/product refresh jobs enqueue only if authoritative data changed
```

This is a **parent-child job cascade** — the structural extraction job is the parent, and the semantic overlay jobs (plus optional chunk-merge job) are children. The Job Runner already supports `parent_job_id` for this pattern. Note that this is distinct from `continuation_chain_id`, which tracks same-skill checkpoint resumption within procurement-ingestion. Extraction orchestration is cross-skill cascading, not single-skill continuation.

**Critical persistence boundary (previous draft gap):**

Semantic skills are `side_effect_class: none`. Therefore the daemon, not the skill, must own authoritative writes after a successful contract run. This feature needs a new persistence surface in Rust. The minimal viable version is a `pdf-extraction/`-specific persister; the cleaner version is a small registry keyed by `skill_id` prefix so the daemon stops growing one-off `if skill_id.starts_with(...)` branches.

---

## 5. Database Schema Implications

### New Table: `document_extractions`

This is the core new table. It stores the structural extraction output from Docling, linking raw documents to their extracted content.

```sql
CREATE TABLE IF NOT EXISTS document_extractions (
  id TEXT PRIMARY KEY,
  raw_document_id TEXT NOT NULL,             -- opaque ref to local-only raw_documents.id; no SQL FK (constitutional rule 15)
  process_id TEXT NOT NULL REFERENCES procurement_processes(id),
  notice_id TEXT REFERENCES contract_notices(id),
  document_set_id TEXT REFERENCES tender_document_sets(id),

  -- Source identity (for cache invalidation)
  source_content_hash TEXT NOT NULL,         -- SHA256 of input PDF bytes

  -- Extraction method
  extraction_engine TEXT NOT NULL,           -- 'docling'
  extraction_engine_version TEXT NOT NULL,   -- installed docling package version
  processing_strategy TEXT NOT NULL,         -- 'standard', 'force_full_page_ocr', 'fast_table_structure'

  -- Current-selection semantics (needed because this is a capability table, not a pipeline stage-head table)
  is_current INTEGER NOT NULL DEFAULT 1 CHECK (is_current IN (0, 1)),
  superseded_by_extraction_id TEXT REFERENCES document_extractions(id),

  -- Document metadata
  page_count INTEGER NOT NULL,
  document_title TEXT,
  document_language TEXT,                    -- detected language
  document_role TEXT,                        -- 'specification', 'pricing_schedule', 'boq', etc.
  extraction_policy_version TEXT NOT NULL,
  expected_overlays_json TEXT NOT NULL DEFAULT '[]',

  -- Extracted content
  extracted_markdown TEXT NOT NULL,          -- Full document as structured markdown
  extracted_sections_json TEXT NOT NULL,     -- JSON array of sections with hierarchy
  extracted_tables_json TEXT NOT NULL,       -- JSON array of tables with row/col structure
  extracted_metadata_json TEXT NOT NULL,     -- Document-level metadata

  -- Quality metrics (objective, not confidence-scored — Docling is deterministic)
  table_count INTEGER NOT NULL,
  section_count INTEGER NOT NULL,
  image_count INTEGER NOT NULL DEFAULT 0,
  has_ocr_content INTEGER NOT NULL DEFAULT 0, -- 1 if OCR was needed (scanned pages)
  empty_page_count INTEGER NOT NULL DEFAULT 0, -- pages with no extractable content
  quality_warnings_json TEXT,               -- JSON array of extraction warnings (e.g., "page 7: no text extracted")

  -- Semantic mapping tracking
  -- NOTE: Per-overlay state lives on document_semantic_extractions rows,
  -- not here. This field is an aggregate summary for quick filtering.
  -- It reflects the WORST state across all expected overlays.
  semantic_extraction_state TEXT NOT NULL DEFAULT 'pending',
    -- pending: no semantic extraction run yet
    -- partial: at least one overlay complete, others pending or in-progress
    -- complete: all expected overlays complete
    -- failed: at least one overlay failed (others may be complete)
    -- skipped: no semantic mapping needed for this document type

  -- Sync tracking (L0 constitutional requirement)
  owner_id TEXT NOT NULL,
  row_version INTEGER NOT NULL DEFAULT 1,
  last_local_seq INTEGER,
  stage_run_id TEXT,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- One authoritative row per raw_document x engine x version x strategy.
CREATE UNIQUE INDEX idx_doc_extraction_current
  ON document_extractions(raw_document_id, extraction_engine, extraction_engine_version, processing_strategy)
  WHERE is_current = 1;

-- Content-addressed reuse lookup for clone-without-rerun caching.
CREATE INDEX idx_doc_extraction_hash_lookup
  ON document_extractions(source_content_hash, extraction_engine, extraction_engine_version, processing_strategy);

-- Join path: find extractions for a process
CREATE INDEX idx_doc_extraction_process
  ON document_extractions(process_id);

-- Find pending semantic extractions
CREATE INDEX idx_doc_extraction_semantic_state
  ON document_extractions(semantic_extraction_state)
  WHERE semantic_extraction_state IN ('pending', 'failed');
```

**Authority:** Push-owned (daemon-authoritative). Same authority class as `tender_document_sets`.

**Key design decisions:**
1. `extracted_markdown` stores the full Docling output as markdown — this is what gets fed to the semantic mapping skill. Compact, human-readable, LLM-friendly.
2. `extracted_sections_json` and `extracted_tables_json` store the structured breakdown — enables direct programmatic access to specific tables or sections without parsing markdown.
3. `source_content_hash` + engine version + processing strategy remains the reuse key, but **not** the table's unique key. A single PDF can legitimately appear under multiple `raw_document_id`s, so authoritative identity stays document-scoped while reuse stays content-scoped.
4. `expected_overlays_json` + `extraction_policy_version` make `semantic_extraction_state` computable and auditable.
5. `is_current` / supersession fields are required because this table is append-only history plus current selection, but it does not participate in `process_stage_heads`.
6. All sync tracking columns present from creation (L0 constitutional requirement §2.2 rule 4).

### New Table: `document_semantic_extractions`

Stores the output of semantic mapping skills — the domain-specific structured data extracted from documents. This is separate from `document_extractions` because one structural extraction can produce multiple semantic extractions (e.g., a single tender pack PDF might yield lot data, candidate data, and pricing data from different sections).

```sql
CREATE TABLE IF NOT EXISTS document_semantic_extractions (
  id TEXT PRIMARY KEY,
  document_extraction_id TEXT NOT NULL REFERENCES document_extractions(id),
  process_id TEXT NOT NULL REFERENCES procurement_processes(id),

  -- Skill that produced this
  skill_id TEXT NOT NULL,                    -- 'pdf-extraction/tender-specification'
  skill_version TEXT NOT NULL,
  job_id TEXT NOT NULL,                      -- opaque ref to local-only workflow_jobs.id; no SQL FK
  session_id TEXT,                           -- opaque ref to local-only agent_session_logs.id; no SQL FK

  -- Extraction scope
  extraction_scope TEXT NOT NULL,            -- 'full_document', 'chunk_merged:<plan_hash>', etc.
  target_domain TEXT NOT NULL,               -- 'lots', 'candidates', 'pricing', 'requirements'
  is_current INTEGER NOT NULL DEFAULT 1 CHECK (is_current IN (0, 1)),
  superseded_by_semantic_extraction_id TEXT REFERENCES document_semantic_extractions(id),

  -- Extracted domain data
  extracted_data_json TEXT NOT NULL,          -- Domain-specific structured output
  field_confidences_json TEXT NOT NULL,       -- Per-field confidence scores

  -- Quality
  extraction_confidence_bps INTEGER NOT NULL,
  warnings_json TEXT,
  evidence_anchors_json TEXT,                -- Page numbers, section refs for auditability

  -- Lineage: which domain rows were written from this extraction
  written_entity_refs_json TEXT,             -- [{"table": "notice_lots", "id": "..."}]

  -- Sync tracking
  owner_id TEXT NOT NULL,
  row_version INTEGER NOT NULL DEFAULT 1,
  last_local_seq INTEGER,
  stage_run_id TEXT,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_doc_semantic_extraction
  ON document_semantic_extractions(document_extraction_id);

CREATE INDEX idx_doc_semantic_process
  ON document_semantic_extractions(process_id);

CREATE UNIQUE INDEX idx_doc_semantic_current
  ON document_semantic_extractions(document_extraction_id, skill_id, extraction_scope)
  WHERE is_current = 1;
```

### Modified Table: `tender_document_sets`

Add extraction tracking:

```sql
-- New column
extraction_state TEXT NOT NULL DEFAULT 'none'
  -- none: no documents extracted yet
  -- partial: some documents extracted
  -- complete: all documents extracted
  -- failed: extraction failed for one or more documents
```

### Modified Table: `raw_documents`

The existing schema is sufficient. `content_type` already distinguishes PDFs (`application/pdf`). `storage_path` already points to the file on disk. Do **not** add `page_count` here yet; structural extraction owns document introspection, and duplicating page metadata into a local-only table does not improve the push-owned extraction contract.

### Entity Relationship: Document Extraction Chain

```
raw_documents (PDF bytes on disk)
    │
    ├── 1:N ──→ document_extractions (Docling structural output)
    │               │   (one current row per raw_document x engine x version x strategy;
    │               │    older rows retained for audit and marked non-current)
    │               │
    │               ├── 1:N ──→ document_semantic_extractions (Claude domain mapping)
    │               │               │
    │               │               └── writes to ──→ notice_lots
    │               │                                 product_candidates
    │               │                                 (other domain tables as needed)
    │               │
    │               └── reusable via (source_content_hash, engine_version, strategy)
    │
    └── N:1 ──→ tender_document_sets (document collection metadata)
```

---

## 6. Pipeline Integration

### Enhanced Pipeline Flow

The existing 15-stage pipeline already has a canonical stage map. This feature does **not** add new top-level `process_stage_status.stage` values. Instead, it adds a capability that executes inside or immediately after `acquire_docs`, and whose outputs become new evidence inputs to Stage 6/7:

```
Stages 1-4: Ingestion → Dedup → Relation Resolution → Preliminary Qualification
    │
Stage 5: Acquire docs
    │
    ├── 5a: Document Download (existing, needs implementation)
    │   │  Downloads PDFs from FTS/CF/PCS portals
    │   │  Writes: raw_documents + tender_document_sets
    │   │
    ├── 5b: Structural Extraction capability (NEW — Docling)
    │   │  Processes each PDF in the document set
    │   │  Writes: document_extractions
    │   │  Deterministic enough for caching; local subprocess, not agent mode
    │   │
    └── 5c: Semantic Extraction capability (NEW — Claude pdf-extraction/* skill)
        │  Maps structural output to domain schema
        │  Writes: document_semantic_extractions + domain tables
        │  Agent-based, uses headless_contract mode
    │
Stage 6: Extract lots (ENHANCED)
    │  Now reads from document_extractions + notice metadata
    │  Previously: would only have notice summary text
    │  Richer lot identification from actual tender documents
    │
Stage 7: Extract products (ENHANCED)
    │  Now reads from document_extractions + lot data
    │  Previously: would only have lot description text
    │  Richer candidate identification from specs, BoQs
    │
Downstream: Quoteability → Search → Pricing → Customs → Margins → Decision
```

### Job Runner Integration

The extraction stages integrate with the existing Job Runner via new job kinds:

```
Job kind: extract_document_structure
├── Trigger: (a) document acquisition completion, or (b) explicit manual enqueue for raw_document_id
├── Resource: No browser slot, no agent slot (daemon-internal subprocess)
├── Execution: runner-native local subprocess branch. Do NOT add
│   `ExecutionMode::Subprocess` to the Agent Adapter; Docling is not an
│   agent CLI and should not pollute the PTY/contract boundary.
├── Output: document_extractions row
├── Continuation: resolves expected overlays and enqueues extract_document_semantics
├── Validation: Daemon-side checks (page_count > 0, table_count
│   plausible for document size, no fully-empty extraction)
├── Stage semantics: capability-local lineage only; no new canonical
│   `process_stage_status.stage` value

Job kind: extract_document_semantics
├── Trigger: structural extraction completion
├── Resource: 1 agent slot (headless_contract), no browser slot
├── Execution: Claude pdf-extraction/* skill — routed through the
│   standard Agent Adapter / Job Runner pipeline (same as manifest-generation)
├── Output: document_semantic_extractions row + domain table writes
├── Validation: Same OQG + semantic validator overlay
├── Continuation: may fan out into chunk jobs + merge job when the
│   document exceeds the overlay token budget
├── Persistence: daemon-owned result persister writes authoritative rows;
│   the skill itself remains side-effect-free
├── Follow-on: triggers lot/product extraction refresh if authoritative data changed
```

### Freshness & Re-extraction

Extraction results participate in the existing freshness system, but they do so as a capability lineage rather than as new pipeline stages:

- **Soft stale:** Upstream document re-downloaded with different content hash → structural extraction needs re-run
- **Soft stale (engine upgrade):** Docling minor/patch version changed → existing extractions are likely still valid but should be re-extracted opportunistically (not urgently). Major version changes with new models warrant higher-priority re-extraction.
- **Hard invalid:** Source content hash changed AND a newer extraction exists → old extraction superseded
- **Cascade:** Structural re-extraction → semantic re-extraction → lot/product re-extraction

The `stage_run_id` on `document_extractions` and `document_semantic_extractions` is therefore lineage metadata: it ties an extraction to the `acquire_docs` run (or manual extraction run) that produced the current authoritative row, but it does not imply a new canonical `process_stage_status.stage`.

---

## 7. Cost Model

### Per-Document Cost (typical 50-page tender pack)

Cost estimates below are order-of-magnitude. Actual cost depends on document complexity, output size, and whether multiple semantic overlays run per document. Token counts assume ~500-1,000 tokens/page for pre-extracted markdown.

| Approach | Structural | Semantic | Total per doc | vs Sonnet-raw-PDF |
|----------|-----------|----------|---------------|-------------------|
| **Hybrid (recommended):** Docling + Claude Haiku (single overlay) | $0.00 | $0.03-0.08 | **$0.03-0.08** | ~30-100x cheaper |
| **Hybrid (typical):** Docling + Claude Haiku (1-2 overlays per doc) | $0.00 | $0.03-0.16 | **$0.03-0.16** | ~15-100x cheaper |
| Claude Sonnet raw PDF | N/A | $2.50-3.00 | $2.50-3.00 | baseline |
| Claude Haiku raw PDF | N/A | $0.50-1.00 | $0.50-1.00 | ~2.5-6x cheaper |
| Docling only (no semantic) | $0.00 | $0.00 | $0.00 | no schema mapping possible |

> **Derivation:** 50 pages × ~700 tokens/page = ~35k input tokens. System prompt + schema + context ≈ 5k tokens. Output ≈ 3-5k tokens. Haiku: ~$1/MTok input, ~$5/MTok output → (0.040 × 1) + (0.004 × 5) ≈ $0.06 per skill run. Multiple overlays (tender-spec + pricing + BoQ) on one document = 2-3x that.

### Volume Estimates

Assuming ~100 qualified processes/week, ~3 documents per process, ~30 pages average:

| Metric | Hybrid (1-2 overlays avg) | Sonnet-raw-PDF |
|--------|---------------------------|----------------|
| Documents/week | 300 | 300 |
| Pages/week | 9,000 | 9,000 |
| Cost/week | $9-48 | $450-540 |
| Cost/month | $36-192 | $1,800-2,160 |
| Structural extraction time | Benchmark-gated local compute; treat this as an adoption gate, not a paper estimate | N/A |
| Semantic mapping time (serialized) | ~2.5-7.5 hrs | ~7.5-22.5 hrs |

> **Assumption caveat:** The 100 processes/week, 3 docs/process, 30 pages/doc are unvalidated estimates. Actual numbers depend on qualification pass rate, document attachment rate on Contracts Finder/FTS, and document size distribution. These should be validated against real source data before budgeting.

The batch API (50% discount) and prompt caching (90% reduction on cached prefixes) further reduce the semantic mapping cost. With 4 concurrent agent slots, semantic mapping parallelizes to ~40-110 minutes/week.

---

## 8. Future Layer Applicability

The toolkit is designed to be reused across layers. Each use case adds a new semantic mapping skill overlay but reuses the same structural extraction engine and database tables.

| Layer | Use Case | Document Type | New Skill Needed |
|-------|----------|---------------|-----------------|
| **L3 (current)** | Tender document extraction | Specifications, pricing, BoQ | `pdf-extraction/tender-specification`, `pdf-extraction/pricing-schedule`, `pdf-extraction/bill-of-quantities` |
| **L5** | Expanded category extraction | Same types, more categories | Same skills, new golden assertions per category |
| **L6** | Bid template processing | RFQ templates, compliance matrices | `pdf-extraction/bid-template` |
| **L6** | Letter of intent generation | Outgoing documents (generation, not extraction) | Different module (document generation, not extraction) |
| **Future** | Supplier certificate verification | ISO certs, test reports, CoO documents | `pdf-extraction/compliance-certificate` |
| **Future** | Framework agreement analysis | Call-off schedules, pricing mechanisms | `pdf-extraction/framework-agreement` |

### Adding a New Document Type

To support a new PDF document type:

1. Create `skills/pdf-extraction/<document-type>/SKILL.md` with domain-specific playbook
2. Create `result.schema.json` defining the extraction output contract
3. Create `canary.inputs.jsonl` with sample Docling outputs for that document type
4. Create `golden.assertions.yaml` with expected extraction behavior
5. Register the document role → skill mapping in the extraction policy

**No schema changes. No recompilation for the semantic side.** Adding a new semantic mapping skill follows the existing skill-first principle — the daemon loads skills dynamically. However, if a new document type requires a different structural strategy (for example `force_full_page_ocr` for scanned certificates), the daemon's routing logic should come from versioned extraction policy/config rather than hard-coded branches.

---

## 9. Open Questions

1. **Docling installation & management — RESOLVED.** Use direct subprocess spawning with one supervised process group per extraction job. Do not introduce a sidecar in the first implementation. Do not extend the `AgentAdapter` execution-mode enum. Model downloads are install-time/bootstrap work, not implicit runtime behavior.

2. **Hardware benchmark gate — OPEN, but now sharply scoped.** The system does not need a generic "GPU vs CPU philosophy"; it needs a benchmark gate on representative UK procurement PDFs. Acceptance should be phrased as: "establish p50/p95 wall-clock time, RSS, timeout rate, and table fidelity on the golden corpus for the target deployment hardware." Do not rely on upstream marketing throughput numbers.

3. **Extraction granularity — RESOLVED.** Docling extracts ALL structure in one pass regardless of document role. One current `document_extractions` row exists per `raw_document_id × engine × version × strategy`. Multiple semantic overlays can hang off that row.

4. **Large document chunking strategy — RESOLVED.** Semantic overlays support two modes: single-pass when the extracted markdown fits the budget, or chunked map/merge when it does not. Chunk plans are deterministic and section-aware; only the merged result becomes authoritative.

5. **Existing lot/product extraction consumers — RESOLVED.** This feature is not satisfied by adding extraction tables alone. The downstream lot/product extraction paths must be upgraded to consume `document_extractions` / `document_semantic_extractions` as first-class evidence inputs. This is a real behavior change, not a prompt-only tweak, because the current live code path still fixture-seeds `notice_lots` / `product_candidates` rather than reading document-derived data.

6. **Document acquisition dependency / priority — RESOLVED.** The toolkit must work standalone from any existing `raw_document_id`; document acquisition is only one trigger path. This is required for development, test seeding, and backfills while the browser download skill remains unimplemented. This also means document acquisition does **not** block implementation order even though it remains part of the full live pipeline.

7. **Fallback when no PDF is available or processable — RESOLVED.** If `tender_document_sets.completeness_state = 'none'`, skip extraction and retain `qualificationBasis = notice_only`. If a PDF is encrypted/corrupt/timeout-prone, persist a structured failure code, keep the document visible for operator review, and degrade to notice-text-only extraction instead of stalling the process.

8. **Raw document storage — RESOLVED.** Plain filesystem storage via `raw_documents.storage_path` is sufficient for this feature. The extraction toolkit should consume those files directly and record hashes/paths; it does not need a second artifact store just for source PDFs.

9. **Non-PDF tender-pack documents — PARTIALLY RESOLVED.** This feature remains PDF-authoritative, but the abstraction must be MIME-aware so future backends can slot in without schema rewrites. `DOCX`/`HTML` can likely reuse the same structural route. `XLSX` pricing schedules are the exception: they may deserve a direct cell parser (for example `calamine`) instead of Docling if golden-corpus accuracy is materially better.

10. **Result persistence boundary — RESOLVED.** Semantic skills stay side-effect-free. The daemon must own authoritative writes through a new result-persister surface for `pdf-extraction/*` outputs. Without this, `document_semantic_extractions` and downstream domain tables can never be updated safely.

11. **Page-spanning tables / split schedules — OPEN and must be tested explicitly.** Multi-page pricing schedules and BoQs are a likely failure mode. The chunk planner, merge step, and golden corpus must all assume that one business table may arrive as multiple structural fragments, with repeated headers and broken row continuity.

---

## Appendix A. Extension Path and MIME-Aware Backend Notes

This appendix is the implementation-facing handoff for adding new semantic overlays after the initial PDF toolkit lands. It is intentionally shorter and more operational than the research narrative above.

### A1. Five-Step Guide for Adding a New Semantic Overlay Skill

1. Create `skills/pdf-extraction/<new-type>/` with the standard five-file package:
   - `SKILL.md`
   - `result.schema.json`
   - `checkpoint.schema.json`
   - `canary.inputs.jsonl`
   - `golden.assertions.yaml`
2. Define the overlay contract in `result.schema.json` so one `document_semantic_extractions` row can persist the output with a single authoritative `target_domain`.
3. Add or update the extraction-policy mapping so the resolved `document_role` can route to the new overlay and record it in `expected_overlays_json`.
4. Add canaries, goldens, and inventory/integration coverage so the new overlay is linted and exercised before it is admitted into live routing.
5. Update daemon-side persistence only if the new overlay introduces new authoritative entity writes; if it only extends existing semantic skill inventory plus policy routing, no runtime binary changes are required.

### A2. Layer Applicability Beyond the Initial Rollout

| Layer | Reuse path | Typical new overlay |
|------|------------|---------------------|
| **L3** | Tender document extraction for lots, product requirements, pricing, and BoQs | `pdf-extraction/tender-specification`, `pdf-extraction/pricing-schedule`, `pdf-extraction/bill-of-quantities` |
| **L5** | Compliance certificates and supplier-verification evidence | `pdf-extraction/compliance-certificate` |
| **L6** | Bid templates, response preparation, and compliance matrices | `pdf-extraction/bid-template` |
| **Additional future layers** | Framework agreements, call-off schedules, and related commercial analysis | `pdf-extraction/framework-agreement` |

### A3. Schema and Build Rules for New Overlays

- New semantic overlays do **not** require schema changes by default. The existing `document_semantic_extractions` table is intentionally generic enough to store new overlay payloads as long as they fit the single-row `target_domain` contract.
- Semantic-side additions do **not** require daemon recompilation when they only add skill packages, schemas, canaries, goldens, and extraction-policy mappings. In that case the daemon continues loading the new package dynamically from the skill directory.
- Daemon recompilation becomes necessary only when an overlay needs new authoritative persistence logic, new runtime dispatch behavior, or a new structural backend capability.

### A4. MIME-Aware Backend Design Rule

- PDF remains the authoritative format in this feature and is the only fully committed backend in the initial rollout.
- `DOCX` and `HTML` should reuse the same structural abstraction and feed the same semantic overlay model once a structural backend is added for those MIME types.
- `XLSX` pricing schedules are the explicit exception case. They may bypass Docling and use a direct spreadsheet parser such as `calamine` if the golden corpus shows materially better cell fidelity and merge behavior than the generic structural route.
- The backend boundary therefore stays MIME-aware even though the first production implementation is PDF-first.
