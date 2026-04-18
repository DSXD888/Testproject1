# Harmonization Handoff — PDF Extraction Toolkit (Retroactive)

> **Run:** 003
> **Mode:** RETROACTIVE (feature is already implemented in the codebase)
> **Source material:** `docs/research/2026-04-11-pdf-extraction-toolkit-research.md` (~776 lines, RESEARCH DOC MODE)
> **Target doc set:** `docs/strategy/` (10 files: 1 strategy + 7 layer docs + cross-reference-index + README)
> **Integration type:** FEATURE ADDITION + ARCHITECTURE EVOLUTION (hybrid)
> **Source-of-truth hierarchy:** Code implementation > Research doc > Layer docs

---

## 1. Mission Statement

You are the chief systems integrator for the procurement intelligence platform's strategy documentation. Your deliverable is an updated documentation set that fully absorbs the PDF Extraction Toolkit — a hybrid two-stage pipeline (Docling structural extraction + Claude semantic mapping skills) for extracting structured data from procurement PDF documents.

**You are editing documentation only. Do not change code.**

**Primary goal:** One coherent model across all 10 strategy docs. No contradictions, no stale content, no omissions. After your edits, an engineer should never need to read the research doc to understand how PDF extraction works in this system.

**Secondary goal:** Implementation-safe docs. No ambiguity that could produce incompatible implementations from different docs.

---

## 2. Retroactive Mode Context

The PDF Extraction Toolkit is **ALREADY FULLY IMPLEMENTED** in the codebase. The layer docs are stale — they do not yet describe this capability at all.

### Source-of-Truth Hierarchy

1. **CODE** (ground truth for what was actually built)
2. **Research doc** (design rationale and intent)
3. **Layer docs** (pre-feature system state — stale for this capability)

### Implementation Surface Map

The implementation matches the research doc closely with no significant divergences. Key surfaces:

| Component | Location | Status |
|-----------|----------|--------|
| Structural extraction engine | `crates/daemon/src/pdf/engine.rs` (683 lines) | Complete |
| Deterministic chunking | `crates/daemon/src/pdf/chunking.rs` (575 lines) | Complete |
| Extraction policy resolution | `crates/daemon/src/pdf/policy.rs` (310 lines) | Complete |
| Module declaration | `crates/daemon/src/pdf/mod.rs` | Complete |
| Database migration | `migrations/0006_pdf_extraction_toolkit.sql` | Complete |
| Job orchestration (structural) | `crates/daemon/src/main.rs` (~lines 1153-1457) | Complete |
| Job orchestration (semantic) | `crates/daemon/src/main.rs` (~lines 1459-1568) | Complete |
| Base skill | `skills/pdf-extraction/base/` | Complete |
| Tender specification skill | `skills/pdf-extraction/tender-specification/` | Complete |
| Pricing schedule skill | `skills/pdf-extraction/pricing-schedule/` | Complete |
| Bill of quantities skill | `skills/pdf-extraction/bill-of-quantities/` | Complete |
| Test: structural extraction | `crates/daemon/tests/pdf_structural_extraction.rs` | Complete |
| Test: semantic extraction | `crates/daemon/tests/pdf_semantic_extraction.rs` | Complete |
| Test: full cascade E2E | `crates/daemon/tests/pdf_extraction_cascade.rs` | Complete |
| Test: skill inventory | `crates/daemon/tests/pdf_extraction_skill_inventory.rs` | Complete |
| Test: lot/product refresh | `crates/daemon/tests/pdf_lot_product_refresh.rs` | Complete |
| Test support: mock Docling | `crates/daemon/tests/support/pdf.rs` | Complete |

### Implementation Drift Summary

**No significant divergences.** The codebase implements the research doc's design faithfully:

- Processing strategies match research doc (standard, force_full_page_ocr, fast_table_structure)
- Cache key is (source_content_hash, extraction_engine_version, processing_strategy) as specified
- Subprocess spawning model (not sidecar, not adapter extension) as specified
- Document role classification follows OCDS type -> filename heuristics -> structural content analysis as specified
- Extraction policy version is `"2026.04.12"` (hardcoded in policy.rs)
- All 4 semantic overlay skills implemented with headless_contract mode at structured_extract_low_cost model tier
- Chunking uses synopsis-based section-aware splitting as specified
- Supercession tracking via is_current/superseded_by as specified

### Unimplemented Portions (mark as future work in docs)

- No TypeScript interfaces in `packages/contracts-schema` for extraction entities yet
- XLSX backend (calamine) for spreadsheet pricing schedules — explicitly deferred in research doc
- DOCX/HTML structural backends — deferred but MIME-aware abstraction is in place

---

## 3. Mandatory Pre-Read Requirement

**CRITICAL: Before you touch any documentation, you MUST perform a deep-reading and comparison pass.** This is not optional. Do not start editing after reading only the research doc or only the layer docs.

### Required Steps

1. Read the **research doc** in full: `docs/research/2026-04-11-pdf-extraction-toolkit-research.md` (~776 lines). Focus on: architecture (§3), module design (§4), database schema (§5), pipeline integration (§6), future layer applicability (§8).

2. Read the **implementation code** for factual accuracy (do NOT modify code):
   - `crates/daemon/src/pdf/engine.rs` — structural extraction engine (focus on public API, types, processing strategies)
   - `crates/daemon/src/pdf/policy.rs` — document role classification and extraction policy (focus on DocumentRole enum, role-to-overlay mapping)
   - `crates/daemon/src/pdf/chunking.rs` — chunking design (focus on synopsis, chunk plan, merge approach)
   - `migrations/0006_pdf_extraction_toolkit.sql` — actual DDL as implemented (focus on columns, indexes, constraints)
   - `skills/pdf-extraction/base/SKILL.md` — base skill metadata and playbook
   - `skills/pdf-extraction/tender-specification/SKILL.md` — domain skill example

3. Read **every target strategy doc** in full (all 10 files):
   - `docs/strategy/layer-strategy.md` (~1400 lines)
   - `docs/strategy/layers/layer-0-gold-set-evaluation.md` (~4170 lines)
   - `docs/strategy/layers/layer-1-thin-local-runner.md` (~1590 lines)
   - `docs/strategy/layers/layer-2-single-operator-desktop.md` (~860 lines)
   - `docs/strategy/layers/layer-3-operational-hardening.md` (~1140 lines)
   - `docs/strategy/layers/layer-4-shared-visibility.md` (~790 lines)
   - `docs/strategy/layers/layer-5-expanded-commercial-intelligence.md` (~520 lines)
   - `docs/strategy/layers/layer-6-request-pipeline-automation.md` (~380 lines)
   - `docs/strategy/cross-reference-index.md`
   - `docs/strategy/README.md`

4. Build a **private comparison matrix**:
   - Research doc describes what → does the layer doc mention it? → does the code implement it?
   - For each layer doc section, note what is stale (doesn't mention PDF extraction) vs what is already consistent

5. **Only THEN** begin harmonizing the docs.

---

## 4. What To Read (with exact paths)

### Primary Source (design rationale and intent)
- `docs/research/2026-04-11-pdf-extraction-toolkit-research.md` (~776 lines)
  - Focus on: §3 (architecture), §4 (module design), §5 (schema), §6 (pipeline integration), §8 (cross-layer applicability)
  - Key conclusions that must NOT be diluted: (1) two-stage hybrid is the right architecture, (2) capability not stage, (3) Docling structural + Claude semantic separation, (4) cost model justification, (5) forward extensibility via new semantic overlay skills

### Implementation Code (ground truth — read for accuracy, do NOT modify)
- `crates/daemon/src/pdf/engine.rs` — PdfExtractionEngine, ExtractionResult, PdfInput, ProcessingStrategy types. Subprocess management. Cache lookup + persist + clone-reuse.
- `crates/daemon/src/pdf/policy.rs` — DocumentRole enum (11 variants), ResolvedExtractionPolicy, resolve_document_role() with OCDS/filename/structural heuristic cascade.
- `crates/daemon/src/pdf/chunking.rs` — DocumentSynopsis, ChunkPlan, SemanticChunk. Synopsis-based deterministic splitting.
- `migrations/0006_pdf_extraction_toolkit.sql` — Actual DDL for `document_extractions` (38 columns) and `document_semantic_extractions` (18 columns). `tender_document_sets` ALTER TABLE for `extraction_state`.
- `skills/pdf-extraction/base/SKILL.md` — Skill metadata pattern (interaction_profile: headless_contract, model_profile: structured_extract_low_cost)
- `skills/pdf-extraction/tender-specification/SKILL.md` — Domain skill: lots + requirements extraction, target_domain = "lots"
- `crates/daemon/src/main.rs` (lines ~1153-1568) — Job orchestration: extract_document_structure and extract_document_semantics job kinds

### Target Docs (grouped by priority)

**PRIMARY TARGETS** (substantial changes needed):
1. `docs/strategy/layers/layer-0-gold-set-evaluation.md` — §2 DDL needs `extraction_state` column on `tender_document_sets`; §1.7 monorepo layout needs `skills/pdf-extraction/`; §3 pipeline logic needs extraction substages referenced
2. `docs/strategy/layers/layer-1-thin-local-runner.md` — §1.5 daemon component listing needs `pdf/` module (3 files); components touched table needs updating; data introduced needs new tables; definition of done needs extraction mention; test listing needs extraction tests
3. `docs/strategy/layer-strategy.md` — §9 schema stability matrix needs `document_extractions` and `document_semantic_extractions` rows; §9 schema evolution rules need new job kinds; §10 stage boundary contracts need acquire_docs→extraction boundary; §11 testing evolution needs extraction test file counts; appendix component activation sequence needs PDF extraction engine

**SECONDARY TARGETS** (consistency alignment needed):
4. `docs/strategy/cross-reference-index.md` — Needs new entries: document extraction tables, PDF extraction skills, extraction orchestration, extraction policy, document role classification
5. `docs/strategy/layers/layer-3-operational-hardening.md` — Extraction validation context (validator overlay for document-acquisition/extraction). Resource arbiter needs note that `extract_document_structure` does NOT consume agent/browser slots (daemon-internal subprocess). Skills listing in components may need pdf-extraction mentioned if operational skill binding applies.
6. `docs/strategy/layers/layer-4-shared-visibility.md` — Supabase projection table list needs `document_extractions` and `document_semantic_extractions` as push-owned projections

**LIGHT TOUCH** (only if specific contradictions exist):
7. `docs/strategy/layers/layer-5-expanded-commercial-intelligence.md` — Brief mention of future pdf-extraction overlay skills (compliance-certificate) if not contradicted
8. `docs/strategy/layers/layer-6-request-pipeline-automation.md` — Brief mention of pdf-extraction/bid-template overlay if not contradicted
9. `docs/strategy/README.md` — Verify layer summary table still accurate after changes; update line counts if significantly changed

---

## 5. Core Harmonization Goals

### Goal 1: Document extraction tables appear in the schema documentation
The `document_extractions` and `document_semantic_extractions` tables must appear in the schema stability matrix (layer-strategy.md §9) with correct Created/First Active Use/Sync/Authority classification. The `extraction_state` column must appear on `tender_document_sets` in L0's DDL as a forward-compatibility nullable column.

**Affected docs:** layer-strategy.md §9, layer-0 §2, layer-1 data introduced

### Goal 2: The daemon's PDF module appears in L1's component inventory
The daemon is currently described as having "17 core files" in L1. The pdf/ module adds 3 leaf files (engine.rs, chunking.rs, policy.rs) — `mod.rs` is not counted, matching existing convention. The L1 component listing must update to **"20 core files"** in all 7 locations across 5 files (see §6.2 for the complete list). The L1 narrative should mention that the daemon includes a structural PDF extraction engine.

**Affected docs:** layer-1 §1.5, layer-strategy.md appendix

### Goal 3: New job kinds are documented in the job kind taxonomy
`extract_document_structure` and `extract_document_semantics` must appear alongside existing job kinds (sync_source, normalize_notice, etc.) in the schema evolution rules (layer-strategy.md §9 rule 6) and anywhere else the job kind list is maintained.

**Affected docs:** layer-strategy.md §9 rule 6, layer-0 §5.1 (if job kind mapping exists there)

### Goal 4: Pipeline flow descriptions acknowledge extraction substages
The 15-stage pipeline descriptions in L0 §3 must acknowledge that Stage 5 (acquire_docs) now includes structural and semantic extraction substages (5a download, 5b structural extraction, 5c semantic extraction). These are NOT new top-level stages — they are capabilities within Stage 5. The stage scope matrix (layer-strategy.md §10) and stage boundary contracts must include the extraction boundaries.

**Affected docs:** layer-0 §3, layer-strategy.md §10

### Goal 5: PDF extraction skills appear in the skill inventory and monorepo layout
The `skills/pdf-extraction/` family (base, tender-specification, pricing-schedule, bill-of-quantities) must appear in L0's monorepo layout (§1.7) and the skill directory references. The skills follow the existing skill standard (SKILL.md + result.schema.json + checkpoint.schema.json + canary.inputs.jsonl + golden.assertions.yaml).

**Affected docs:** layer-0 §1.7, cross-reference-index

### Goal 6: Extraction tests appear in the testing evolution
The 5 new extraction test files must be reflected in the test count growth table (layer-strategy.md §11) and in the appropriate layer's test scenario listing.

**Affected docs:** layer-strategy.md §11, layer-1 test scenarios

### Goal 7: Cross-layer future applicability is documented
The research doc §8 describes future use of the toolkit in L5 (compliance certificates) and L6 (bid templates). These should be mentioned as forward-compatibility notes in L5 and L6 docs, and the toolkit's design-as-capability (not stage) philosophy should be clear.

**Affected docs:** layer-5 forward-compatibility, layer-6 forward-compatibility

### Goal 8: Sync projections include extraction tables
When L4 activates sync, `document_extractions` and `document_semantic_extractions` are push-owned tables that need Supabase projection. They must appear in L4's Supabase projection table listing.

**Affected docs:** layer-4 §2

### Goal 9: Resource arbiter knows about extraction jobs
`extract_document_structure` does NOT consume agent/browser slots — it is a daemon-internal subprocess (Docling). `extract_document_semantics` consumes 1 agent_slot (headless_contract). L3's resource arbiter documentation should reflect this.

**Affected docs:** layer-3 §1.4

---

## 6. Expected Document Changes

### 6.1 Highest Priority: `layer-0-gold-set-evaluation.md`

**§2 DDL (constitutional schema)**:
- Add `extraction_state text not null default 'none'` to the `tender_document_sets` table DDL as a forward-compatibility column. Insert it after `completeness_state` with a comment: `-- none, partial, complete, failed (populated from L1 when PDF extraction is active)`. The existing DDL uses PostgreSQL syntax (`uuid`, `jsonb`, `timestamptz`, `gen_random_uuid()`) — match that style exactly.
- **Constitutional tension note:** The "all columns from creation" rule (§2.2 rule 2) says later layers never add new columns to tables introduced in earlier layers. This `extraction_state` column follows the same pattern as `validation_required` on `workflow_jobs` — it is a forward-compatibility provision. The L0 DDL is retroactively updated to include it as if it were always planned. Document it in the DDL comment as a forward-compatibility column and add a one-line note in the §2.2.1 scope clarification section if one exists, explaining that extraction_state was added to the constitutional shape alongside the PDF extraction toolkit introduction.
- Do NOT add `document_extractions` or `document_semantic_extractions` to L0 DDL — these are L1-introduced tables (same pattern as `raw_documents`, `agent_session_logs`, which are created in L1's migration 0003).
- **Table count:** The cross-reference-index says "47 tables" and layer-strategy.md §2 says "47 tables." After adding `document_extractions` and `document_semantic_extractions` in L1, the total across all layers becomes **49 tables**. Update both occurrences:
  - `cross-reference-index.md` line 13: "Full SQLite DDL (47 tables" → "49 tables"
  - `layer-strategy.md` §2 per-layer doc table: "Full DDL (47 tables)" → "49 tables"

**§1.7 Project Shape (monorepo layout)**:
- Add `skills/pdf-extraction/` directory to the skills tree with subdirectories: `base/`, `tender-specification/`, `pricing-schedule/`, `bill-of-quantities/`. Add a comment like `# PDF semantic mapping (L1)`.

**§3 Pipeline Logic**:
- In the 15-stage pipeline description, clarify that Stage 5 (Acquire Docs) includes sub-capabilities: document download (5a), structural extraction via Docling (5b), semantic extraction via Claude pdf-extraction skills (5c). These are not new top-level stages but internal capabilities that enrich the document set before Stage 6 (Extract Lots) and Stage 7 (Extract Products).
- Add a brief note that Stages 6 and 7 now consume `document_extractions` and `document_semantic_extractions` as evidence inputs alongside notice metadata.

### 6.2 Highest Priority: `layer-1-thin-local-runner.md`

**§1.5 Component Overview — daemon file count:**

The existing listing counts ONLY leaf module files (not `mod.rs` declarations — verify: the existing 17 entries include `agents/adapter.rs` but not `agents/mod.rs`). The pdf/ module adds 3 leaf files. Update the heading from "17 core files" to **"20 core files"** and add these rows to the daemon file table:

| File | Responsibility |
|------|---------------|
| `pdf/engine.rs` | Structural PDF extraction engine: Docling subprocess wrapper with supervised process groups, extraction result parsing, content-hash cache lookup/persist/clone-reuse, processing strategy dispatch (standard, force_full_page_ocr, fast_table_structure) |
| `pdf/chunking.rs` | Deterministic document chunking: document synopsis extraction (section tree, table headers, page anchors, token estimates), section-aware chunk plans that never split tables, merge job coordination for per-chunk semantic overlays |
| `pdf/policy.rs` | Extraction policy resolution: document role classification via three-tier heuristic cascade (OCDS metadata → filename patterns → structural content analysis), role-to-overlay skill mapping with versioned extraction policy |

**CRITICAL: "17" appears in 7 locations across 5 files.** All must become "20":
1. `layer-1-thin-local-runner.md` §1.5: `**crates/daemon** — 17 core files:**`
2. `layer-1-thin-local-runner.md` requirement `L1-FR-DAEMON.2 AC1`: "limited to the 17 daemon files, the two new skills, and the `0003_sqlite_runtime.sql` migration" — update ALL three: 17→20 daemon files, two→six skills, and add `0006_pdf_extraction_toolkit.sql` alongside `0003`
3. `layer-strategy.md` §2 per-layer doc table: "Daemon core (17 files)"
4. `layer-strategy.md` §3 L1 Components Touched table: `crates/daemon` row "17 core files" → "20 core files" + append ", pdf/engine+chunking+policy" to the parenthetical
5. `layer-strategy.md` Appendix: "Thin (17 files: launch + capture + supervise + IPC + reaper)" → "20 files: ... + pdf extraction"
6. `README.md` L1 description: "Rust daemon (17 files)"
7. `cross-reference-index.md` Skills section: "Rust crate structure (17 daemon files)"

**ALSO update the Components Touched table in `layer-strategy.md` §3 (L1 section):**
- Skills row: "2 (`procurement-ingestion/fts`, `manifest-generation`)" → "6" (add the 4 pdf-extraction skills)
- Database row: append `0006_pdf_extraction_toolkit.sql` (`document_extractions`, `document_semantic_extractions`)

**§1.5 New skills** — update the heading from "New skills (2)" to "New skills (6)" and add these 4 alongside existing FTS and manifest-generation:

| Skill | Purpose | Interaction Profile |
|-------|---------|-------------------|
| `skills/pdf-extraction/base/` | Generic document semantic mapping (classification + entity extraction) | `headless_contract` |
| `skills/pdf-extraction/tender-specification/` | Lot structure + requirement extraction from tender specs, ITT, SoR | `headless_contract` |
| `skills/pdf-extraction/pricing-schedule/` | Line-item pricing extraction from pricing schedules, rate cards | `headless_contract` |
| `skills/pdf-extraction/bill-of-quantities/` | Itemized quantity breakdown extraction from BoQs | `headless_contract` |

All use `model_profile: structured_extract_low_cost` (Haiku tier) because the hard structural work is done by Docling. All are `side_effect_class: none` — the daemon, not the skill, owns authoritative database writes after a successful contract run. This daemon-side persistence is a new `pdf-extraction/`-specific result persister in the Job Runner.

**Data Introduced — new tables:**

| Table | Authority | Purpose | Key Columns |
|-------|-----------|---------|-------------|
| `document_extractions` | Push-owned | Structural extraction output from Docling — links raw documents to their extracted content (markdown, sections, tables, metadata, quality metrics) | `raw_document_id` (opaque ref, no SQL FK per rule 15), `source_content_hash`, `extraction_engine`/`_version`, `processing_strategy`, `extracted_markdown`, `extracted_tables_json`, `extracted_sections_json`, `document_role`, `expected_overlays_json`, `semantic_extraction_state`, `is_current`/`superseded_by_extraction_id` |
| `document_semantic_extractions` | Push-owned | Semantic mapping output from Claude pdf-extraction skills — domain-specific structured data per overlay | `document_extraction_id` (FK), `skill_id`, `extraction_scope`, `target_domain`, `extracted_data_json`, `field_confidences_json`, `evidence_anchors_json`, `written_entity_refs_json`, `is_current`/`superseded_by` |

Migration: `0006_pdf_extraction_toolkit.sql` (not contiguous with L1's 0003 — intervening migrations exist for other features).

**Entity relationship chain** (include in data introduced):
```
raw_documents (PDF bytes on disk)
    │
    ├── 1:N ──→ document_extractions (Docling structural output)
    │               │   (one current row per raw_document × engine × version × strategy)
    │               │
    │               └── 1:N ──→ document_semantic_extractions (Claude domain mapping)
    │                               └── writes to → notice_lots, product_candidates
    │
    └── N:1 ──→ tender_document_sets (document collection metadata)
```

Note: `document_extractions.raw_document_id` is an opaque reference (no SQL FK) per cross-authority FK discipline rule 15, since `raw_documents` is local-only and `document_extractions` is push-owned.

**Parent-child job cascade:** The extraction orchestration uses parent-child job cascading: the `extract_document_structure` job is the parent, and `extract_document_semantics` jobs (one per expected overlay, plus optional chunk-merge jobs) are children linked via `workflow_jobs.parent_job_id`. This is the first real consumer of the `parent_job_id` column that has existed as nullable since L0. This is DISTINCT from `continuation_chain_id`, which tracks same-skill checkpoint resumption.

**Activated tables**:
- Add `tender_document_sets.extraction_state` as now actively populated.

**Ingestion flow (§1.6) step 11:**
The pipeline processing table list in step 11 currently reads: `procurement_processes`, `contract_notices`, `raw_documents`, `tender_document_sets`, `notice_lots`, ... Add `document_extractions` and `document_semantic_extractions` to this list.

**Definition of Done:**
- The existing flow description says: "normalize → qualify → acquire docs → extract lots → extract products → price-signal match → gross gate → landed cost → manifest." Update to include extraction: "normalize → qualify → acquire docs → **structural extraction → semantic extraction** → extract lots → extract products → ..."
- Add an item: "Structural extraction completes for at least one PDF, producing a valid `document_extractions` row with extracted markdown and tables"
- Add an item: "Semantic extraction completes for at least one document, producing a valid `document_semantic_extractions` row with domain-specific structured output"

**Test Scenarios**:
- Add: `pdf_structural_extraction.rs` — Docling subprocess, cache hit/miss, persistence, supercession
- Add: `pdf_semantic_extraction.rs` — Semantic overlay skill execution via headless_contract
- Add: `pdf_extraction_cascade.rs` — Full pipeline cascade: structural → semantic → lot/product refresh
- Add: `pdf_extraction_skill_inventory.rs` — Skill linting, golden case verification for pdf-extraction family
- Add: `pdf_lot_product_refresh.rs` — End-to-end integration: extraction → lot extraction downstream

**Forward-Compatibility Provisions**:
- Add: PDF extraction toolkit designed as reusable capability — new document types add skill overlays without schema changes or daemon recompilation (see research doc §8)
- Add: MIME-aware backend abstraction ready for DOCX/HTML/XLSX future backends

### 6.3 Highest Priority: `layer-strategy.md`

**§9 Schema Stability Matrix**:
- Add rows:

| Table | Created | First Active Use | Sync Activated | Authority |
|-------|---------|-----------------|----------------|-----------|
| `document_extractions` | L1 | L1 (structural extraction from document acquisition) | L4 | Push-owned |
| `document_semantic_extractions` | L1 | L1 (semantic overlay extraction) | L4 | Push-owned |

- Update `tender_document_sets` row: add note that `extraction_state` column exists from L0 (forward-compatibility), first populated in L1.

**§9 Schema Evolution Rules, rule 6 (job kinds)**:
- Add `extract_document_structure` and `extract_document_semantics` to the job kind list. Note that `extract_document_structure` is runner-native (daemon subprocess, no agent adapter) and `extract_document_semantics` runs through the standard agent adapter pipeline.

**§10 Stage Scope Matrix**:
- The "Acquire docs" stage entry should note that it now includes structural and semantic extraction sub-capabilities within the notice scope. These do not introduce new scope_kind values.

**§10 Stage Boundary Contracts**:
- Add a new boundary: `acquire_docs (structural) → extract_lots / extract_products` — structural extraction output (`document_extractions` with extracted_markdown, extracted_tables_json, extracted_sections_json) feeds into lot and product extraction as enriched evidence input.
- Update the existing `acquire_docs → extract_lots` boundary to mention document extraction output as additional evidence alongside `TenderDocumentSet`.

**§11 Testing Evolution — test count growth table:**
Update L1 and ALL downstream cumulative counts. Current → Updated:

| Layer | New Test Files | Cumulative |
|-------|---------------|------------|
| L0 | 4 (unchanged) | 4 (unchanged) |
| L1 | 10 → **15** (+5 extraction) | 14 → **19** |
| L2 | 11 (unchanged) | 25 → **30** |
| L3 | 18 (unchanged) | 43 → **48** |
| L4 | 4 (unchanged) | 47 → **52** |
| L5 | 5 (unchanged) | 52 → **57** |
| L6 | 3 (unchanged) | 55 → **60** |

The 5 new L1 test files: `pdf_structural_extraction.rs`, `pdf_semantic_extraction.rs`, `pdf_extraction_cascade.rs`, `pdf_extraction_skill_inventory.rs`, `pdf_lot_product_refresh.rs`.

Add to L1's Focus column: "+ PDF extraction cascade, semantic overlay, skill inventory".

**Appendix: Component Activation Sequence**:
- Add entry: `PDF extraction engine (structural + semantic)` | L1 | Full (3 daemon modules + 4 skill directories + 2 job kinds)

### 6.4 Secondary: `cross-reference-index.md`

Add new entries to the "By Topic" tables:

**Data & Schema section:**

| Topic | File | Section |
|-------|------|---------|
| Document extraction tables (`document_extractions`, `document_semantic_extractions`) | `layers/layer-1-thin-local-runner.md` | Data Introduced |
| `extraction_state` on `tender_document_sets` (forward-compatibility) | `layers/layer-0-gold-set-evaluation.md` | §2 |
| Document extraction entity relationship chain | `layers/layer-1-thin-local-runner.md` | Data Introduced |

**Pipeline section:**

| Topic | File | Section |
|-------|------|---------|
| PDF structural extraction (Docling engine, processing strategies) | `layers/layer-1-thin-local-runner.md` | §1.5 |
| PDF semantic mapping (Claude overlay skills) | `layers/layer-1-thin-local-runner.md` | §1.5 |
| Extraction orchestration (job cascade: structural → semantic → lot/product refresh) | `layers/layer-1-thin-local-runner.md` | §1.5 |
| Extraction policy (document role classification, overlay routing) | `layers/layer-1-thin-local-runner.md` | §1.5 |
| Deterministic document chunking (synopsis, section-aware splits) | `layers/layer-1-thin-local-runner.md` | §1.5 |

**Skills & Agent Orchestration section:**

| Topic | File | Section |
|-------|------|---------|
| PDF extraction skill family (base + 3 domain overlays) | `layers/layer-1-thin-local-runner.md` | §1.5 |

### 6.5 Secondary: `layer-3-operational-hardening.md`

**§1.4 Resource Arbiter**:
- Add a note that `extract_document_structure` jobs do NOT consume `browser_slots` or `agent_slots` — they run as daemon-internal subprocesses (Docling). Only `extract_document_semantics` jobs consume `agent_slots` (1 per semantic overlay, headless_contract mode).
- Add `extract_document_semantics` to the admission priority ordering. It logically fits between qualification (3) and supplier enrichment (4), since document extraction feeds downstream pipeline stages.

**§1.3 Module listing or skills listing**:
- If L3's validator overlay listing mentions document-acquisition validation, note that PDF extraction outputs are also validated through the existing OQG + validator framework.

### 6.6 Secondary: `layer-4-shared-visibility.md`

**Supabase projection tables** — TWO locations need updating:

1. **In the L4 layer doc itself**: Add `document_extractions` and `document_semantic_extractions` to the push-owned Supabase projection table list.

2. **In `layer-strategy.md` §6 (L4 section), line ~785**: The inline enumeration reads "All push-owned tables from Layers 0–3 get Supabase counterparts: `procurement_processes`, `contract_notices`, `notice_lots`, `tender_document_sets`, ..." — add `document_extractions`, `document_semantic_extractions` to this list.

### 6.6b Light Touch: `layer-2-single-operator-desktop.md`

**Desktop detail views**: L2 currently mentions `tender_document_sets` as "Existing live document sets are now rendered in desktop detail views with completeness surfaced to the operator." The new `extraction_state` column is related but distinct from `completeness_state`. If the L2 doc lists specific columns surfaced to the operator, add `extraction_state` alongside `completeness_state`. If it's described generically, no change needed — the extraction state will naturally be surfaced when the desktop reads the table.

### 6.7 Light Touch: `layer-5-expanded-commercial-intelligence.md`

**Forward-Compatibility Provisions** (or a brief mention in the narrative):
- Note that the PDF extraction toolkit supports future compliance certificate extraction via a new `pdf-extraction/compliance-certificate` semantic overlay skill — no schema changes or daemon recompilation needed.

### 6.8 Light Touch: `layer-6-request-pipeline-automation.md`

**Forward-Compatibility Provisions** (or a brief mention in the narrative):
- Note that the PDF extraction toolkit supports future bid template processing via a new `pdf-extraction/bid-template` semantic overlay skill. Note that document generation (letters of intent) is a different module, not an extraction overlay.

---

## 7. Integration Rules

### What MUST be true after harmonization

- No strategy doc contradicts the research doc or the code on PDF extraction architecture
- `document_extractions` and `document_semantic_extractions` appear consistently wherever pipeline tables are listed
- The two-stage hybrid architecture (Docling structural + Claude semantic) is described consistently in every doc that mentions PDF extraction
- The "capability not stage" principle is clear — PDF extraction does NOT add new top-level pipeline stages
- The daemon file count in L1 is updated to reflect the pdf/ module
- The schema stability matrix has rows for both new tables
- The test count growth table reflects the 5 new test files
- The extraction_state column on tender_document_sets appears in L0 DDL

### Hardcoded counts that MUST be updated

These are the most dangerous harmonization failure mode — stale counts that look authoritative:

| Count | Old | New | Locations |
|-------|-----|-----|-----------|
| Daemon files | 17 | **20** | 7 locations across 5 files (see §6.2) |
| L1 skills | 2 | **6** | L1 §1.5 heading, L1-FR-DAEMON.2, layer-strategy §3 Components Touched |
| Total tables | 47 | **49** | 2 locations (see §6.1) |
| L1 test files | 10 | **15** | layer-strategy §11 |
| L1 cumulative tests | 14 | **19** | layer-strategy §11 |
| All downstream cumulative | +5 each | see §6.3 | layer-strategy §11 |

### What MUST stay OUT

- Implementation task planning or bead sequencing ("first implement X then Y")
- Tool-specific development instructions
- Design iteration narrative (the "how we got here" story)
- Cost model details from the research doc (belong in research doc, not strategy docs)
- Detailed Docling API surface or Python subprocess management details (implementation detail, not architecture)
- Benchmark numbers from the research doc (not verified, belong in research doc)

### What MUST be ADDED where existing docs are too vague

- L1's component listing is currently specific and complete — the pdf/ module must be added at the same level of detail
- L0's pipeline description currently does not mention extraction at all — a brief description of substages 5a/5b/5c is needed
- layer-strategy.md's schema stability matrix is currently exhaustive — both new tables need rows

### Style Rules

- Preserve each document's existing style and section structure
- Integrate into existing sections, not dump as appendices
- Match the level of detail in surrounding content (e.g., L1's daemon file table has one row per file with a one-line responsibility description)
- The pdf/ module entries should look like the existing daemon file entries (one row per file, one-line responsibility)
- New schema stability matrix rows should look like existing rows
- New cross-reference-index entries should follow the existing table format

---

## 8. What A Great Outcome Looks Like

- An engineer reading only the harmonized strategy docs gets one coherent model that includes PDF extraction as a first-class capability of the L1 daemon
- The engineer knows: (1) what tables store extraction output, (2) where the daemon code lives, (3) what skills perform semantic mapping, (4) how extraction fits into the 15-stage pipeline, (5) how extraction jobs interact with the resource arbiter
- The docs clearly distinguish PDF extraction as a reusable capability (not a fixed pipeline stage) that can be extended with new overlay skills
- The schema stability matrix, stage scope matrix, stage boundary contracts, and testing evolution are all internally consistent after the additions
- `tender_document_sets.extraction_state` appears in L0 DDL as a forward-compatibility column (matching the pattern of other forward-compatibility columns like `validation_required` on `workflow_jobs`)
- No doc implies extraction features that are not implemented (XLSX backend, DOCX backend are explicitly future work)
- The cross-reference index lets an engineer find any extraction topic in one lookup

---

## 9. Deliverable

Updated documentation files. At minimum expect to modify:

1. `docs/strategy/layers/layer-0-gold-set-evaluation.md`
2. `docs/strategy/layers/layer-1-thin-local-runner.md`
3. `docs/strategy/layer-strategy.md`
4. `docs/strategy/cross-reference-index.md`
5. `docs/strategy/layers/layer-3-operational-hardening.md`
6. `docs/strategy/layers/layer-4-shared-visibility.md`

Optionally:
7. `docs/strategy/layers/layer-5-expanded-commercial-intelligence.md`
8. `docs/strategy/layers/layer-6-request-pipeline-automation.md`
9. `docs/strategy/README.md`

Provide a concise change summary: which docs changed, what was resolved, what remains open.

---

## 10. Final Verification Checklist

Before stopping, verify:

- [ ] Every modified doc was re-read after editing
- [ ] No modified doc contradicts the research doc or the implemented code
- [ ] No modified doc contradicts another target doc
- [ ] No modified doc gained implementation sequencing content ("first implement X, then Y")
- [ ] `document_extractions` and `document_semantic_extractions` appear in schema stability matrix
- [ ] New job kinds appear in rule 6 of schema evolution rules
- [ ] **All 7 occurrences of "17" updated to "20"** across 5 files (L1 doc ×2, layer-strategy ×3, README ×1, cross-ref-index ×1)
- [ ] L1 daemon file table has pdf/engine.rs, pdf/chunking.rs, pdf/policy.rs entries
- [ ] L1 requirement `L1-FR-DAEMON.2` updated: 17→20 files, two→six skills, 0006 migration added
- [ ] L1 "New skills (2)" heading updated to "New skills (6)"
- [ ] `extraction_state` column appears in L0 DDL on `tender_document_sets` (PostgreSQL syntax)
- [ ] **"47 tables" updated to "49 tables"** in cross-reference-index AND layer-strategy §2
- [ ] `skills/pdf-extraction/` appears in L0 monorepo layout
- [ ] Pipeline description in L0 §3 mentions extraction substages
- [ ] L1 Definition of Done flow updated with "structural extraction → semantic extraction"
- [ ] L1 ingestion flow step 11 includes `document_extractions` and `document_semantic_extractions`
- [ ] Stage boundary contracts include extraction boundaries
- [ ] Test count growth reflects 5 new extraction test files with correct cumulative math (L1: 19, L2: 30, L3: 48, L4: 52, L5: 57, L6: 60)
- [ ] Cross-reference index has entries for all extraction topics
- [ ] L4 Supabase projections include extraction tables in BOTH the L4 doc AND layer-strategy §6 inline enumeration
- [ ] L3 resource arbiter notes extraction job resource consumption
- [ ] Component activation sequence includes PDF extraction engine
- [ ] Entity relationship chain (raw_documents → document_extractions → document_semantic_extractions) is in L1 data introduced
- [ ] layer-strategy §3 L1 Components Touched: Skills row shows 6 (was 2), Database row includes 0006 migration

---

## 11. Follow-Up Fresh-Eyes Pass

> **Send this as a follow-up message in the same session after the main harmonization is complete.**

Re-read the research doc (`docs/research/2026-04-11-pdf-extraction-toolkit-research.md`) and every modified strategy doc with completely fresh eyes. Then:

1. **Contradiction hunt** — search every strategy doc for these specific terms and verify consistency:
   - "document_extractions" — appears consistently in all listings (schema matrix, projection tables, entity relationships)
   - "document_semantic_extractions" — same
   - "extraction_state" — on tender_document_sets, not elsewhere
   - "extract_document_structure" / "extract_document_semantics" — job kinds in all job kind listings
   - "Docling" — only in L1 and research doc, not in other layer docs (implementation detail)
   - "pdf-extraction" — skill references consistent across all docs
   - "17 core files" — should be updated wherever L1 daemon file count is mentioned
   - "capability not stage" — extraction is described as a capability, not a new pipeline stage

2. **Stale count and wording check** — run literal searches across all strategy docs:
   - Grep for `"17"` → every daemon-file-count hit must now say 20
   - Grep for `"47"` → every table-count hit must now say 49
   - Grep for `"14"` in layer-strategy §11 → L1 cumulative must now say 19
   - Any layer doc that says "no document extraction" or implies documents are not processed
   - Any pipeline description that skips from "acquire docs" to "extract lots" without mentioning structural/semantic extraction
   - Any test count that doesn't include extraction tests
   - L1 requirement `L1-FR-DAEMON.2` still referencing 17 files or "two new skills"
   - L1 §1.5 still saying "New skills (2)" instead of "(6)"

3. **Implementation planning leak check** — verify no doc says:
   - "When PDF extraction is implemented..."
   - "Future: add PDF extraction..."
   - Any language implying extraction is not yet built

4. **Absorption completeness** — verify every concept from the research doc has a home:
   - Two-stage hybrid architecture → L1 component listing
   - Extraction policy and document role classification → L1 component listing
   - Deterministic chunking → L1 component listing
   - Cache/reuse via content hash → L1 data introduced or component description
   - Semantic overlay skill family → L1 skills listing + L0 monorepo layout
   - Future extensibility (new overlays without schema/recompile) → L1 forward-compatibility
   - Cost model → stays in research doc only (not strategy docs)
   - Database tables → schema stability matrix + L1 data introduced

Fix any issues found. After fixes, re-read everything ONE MORE TIME.
