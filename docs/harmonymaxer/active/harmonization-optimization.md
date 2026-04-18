# Harmonization Optimization — PDF Extraction Toolkit (Retroactive)

> **Run:** 003
> **Mode:** RETROACTIVE
> **Purpose:** Fresh-eyes verification and consistency audit after the harmonization handoff has been executed. This prompt is run by a DIFFERENT agent session than the one that performed the harmonization.
> **Source material:** `docs/research/2026-04-11-pdf-extraction-toolkit-research.md` (~776 lines)
> **Target doc set:** `docs/strategy/` (10 files)

---

## 1. Your Role

You are a documentation quality auditor performing a post-integration review of the strategy documentation for the UK Procurement Intelligence Platform. A prior session just harmonized the strategy docs to reflect a new capability: the **PDF Extraction Toolkit** — a hybrid two-stage pipeline (Docling structural extraction + Claude semantic mapping) for extracting structured data from procurement PDF documents.

The feature is ALREADY IMPLEMENTED in the codebase. The strategy docs should now accurately describe the system as it exists.

**Your job:** verify the harmonization was done correctly, find any remaining inconsistencies, and fix them.

**You are editing documentation only. Do not change code.**

---

## 2. Deep Re-Read (Mandatory Before Any Audit)

Before auditing, you must build a fresh mental model:

1. **Read the research doc** in full: `docs/research/2026-04-11-pdf-extraction-toolkit-research.md`
   - Note every concept, table, component, skill, job kind, and pipeline change it describes

2. **Read the implementation code** for ground truth (do NOT modify):
   - `crates/daemon/src/pdf/engine.rs` — structural extraction engine API
   - `crates/daemon/src/pdf/policy.rs` — document role classification + extraction policy
   - `crates/daemon/src/pdf/chunking.rs` — deterministic chunking design
   - `migrations/0006_pdf_extraction_toolkit.sql` — actual DDL
   - `skills/pdf-extraction/base/SKILL.md` — base skill metadata
   - `skills/pdf-extraction/tender-specification/SKILL.md` — domain skill pattern

3. **Read every target strategy doc** in full — ALL 10 files, not just the ones marked as modified:
   - `docs/strategy/layer-strategy.md`
   - `docs/strategy/layers/layer-0-gold-set-evaluation.md`
   - `docs/strategy/layers/layer-1-thin-local-runner.md`
   - `docs/strategy/layers/layer-2-single-operator-desktop.md`
   - `docs/strategy/layers/layer-3-operational-hardening.md`
   - `docs/strategy/layers/layer-4-shared-visibility.md`
   - `docs/strategy/layers/layer-5-expanded-commercial-intelligence.md`
   - `docs/strategy/layers/layer-6-request-pipeline-automation.md`
   - `docs/strategy/cross-reference-index.md`
   - `docs/strategy/README.md`

4. **Note any remaining divergences** — places where the docs don't match the research doc or the code.

---

## 3. Contradiction Hunt

For each term below, search EVERY strategy doc for mentions and verify consistency:

### 3.1 Table Names
- **`document_extractions`** — Must appear in: L1 data introduced, layer-strategy schema stability matrix, layer-4 Supabase projections, cross-reference-index. Must NOT appear in L0 DDL (it's an L1-introduced table).
- **`document_semantic_extractions`** — Same locations as above.
- **`extraction_state`** — Must appear ONLY on `tender_document_sets` in L0 DDL as a forward-compatibility column. Must not be confused with `semantic_extraction_state` (which is a column on `document_extractions`).
- **`tender_document_sets`** — Verify that everywhere this table is mentioned, the `extraction_state` column is acknowledged if relevant to the context.

### 3.2 Job Kinds
- **`extract_document_structure`** — Must appear in: layer-strategy §9 rule 6 (job kind taxonomy). Must be described as runner-native (daemon subprocess, no agent adapter). Must NOT consume browser_slots or agent_slots.
- **`extract_document_semantics`** — Must appear alongside above. Must be described as standard agent adapter execution (headless_contract). Consumes 1 agent_slot.

### 3.3 Daemon Components and Counts
- **"17 core files"** → must be **"20 core files"** in ALL 7 occurrences across 5 files. Search for the literal "17" in every strategy doc and verify. The 7 known locations are:
  1. `layer-1-thin-local-runner.md` §1.5 heading
  2. `layer-1-thin-local-runner.md` requirement `L1-FR-DAEMON.2 AC1`
  3. `layer-strategy.md` §2 per-layer doc table
  4. `layer-strategy.md` §3 L1 Components Touched
  5. `layer-strategy.md` Appendix: Component Activation Sequence
  6. `README.md` L1 description
  7. `cross-reference-index.md` Skills section
- **`pdf/engine.rs`**, **`pdf/chunking.rs`**, **`pdf/policy.rs`** — Must appear in L1's daemon file table.
- **"47 tables"** → must be **"49 tables"** in:
  1. `cross-reference-index.md` first entry
  2. `layer-strategy.md` §2 per-layer doc table (L0 row)

### 3.4 Skills
- **`skills/pdf-extraction/`** — Must appear in L0 monorepo layout (§1.7). Must appear in cross-reference-index.
- **`pdf-extraction/tender-specification`**, **`pdf-extraction/pricing-schedule`**, **`pdf-extraction/bill-of-quantities`**, **`pdf-extraction/base`** — Must be mentioned in L1 skills listing.
- All skills must be described with `interaction_profile: headless_contract` and `model_profile: structured_extract_low_cost`.

### 3.5 Pipeline
- **"15-stage pipeline"** — The number of stages must NOT change. PDF extraction is a capability within Stage 5, not new stages.
- **"Acquire docs"** / **Stage 5** — Must mention structural and semantic extraction sub-capabilities wherever the pipeline is described in detail (L0 §3, layer-strategy §10).
- **"capability not stage"** — The language must make clear that PDF extraction is a reusable capability, not a fixed pipeline step.

### 3.6 Testing
- Test count numbers in layer-strategy §11 — L1 must show **15** new test files (was 10, +5 extraction). All cumulative counts shift by +5: L1=19, L2=30, L3=48, L4=52, L5=57, L6=60. Verify the math adds up.

### 3.8 Ingestion Flow and Definition of Done
- **L1 ingestion flow step 11** (pipeline processing table list) — Must include `document_extractions` and `document_semantic_extractions` alongside existing tables.
- **L1 Definition of Done flow** — Must mention "structural extraction → semantic extraction" between "acquire docs" and "extract lots."
- **L1-FR-DAEMON.2** — Requirement AC1 must update ALL three stale values: 17→20 daemon files, "two new skills"→"six skills", and add 0006 migration alongside existing 0003.
- **L1 "New skills" heading** — Must say "New skills (6)" (was "New skills (2)") and include 4 pdf-extraction skill rows.

### 3.9 Supabase Projection Completeness
- **layer-strategy.md §6 (L4 section)** inline enumeration (~line 785) — Lists all push-owned tables that get Supabase counterparts. Must include `document_extractions` and `document_semantic_extractions`.
- **L4 layer doc** projection table section — Same tables must appear.

### 3.10 Components Touched Table (layer-strategy.md §3, L1 section)
- **Skills row**: Must show 6 skills (was 2). Must include the 4 pdf-extraction skills alongside existing FTS and manifest-generation.
- **`crates/daemon` row**: Must show "20 core files" (was 17). Must mention pdf/ module in the parenthetical.
- **Database row**: Must mention `0006_pdf_extraction_toolkit.sql` alongside existing `0003_sqlite_runtime.sql`.

### 3.11 Cross-References
- Every new concept in the cross-reference-index must point to the correct file and section.
- Verify no stale section references (section numbers may have shifted after edits).

---

## 4. Doc Boundary Sanity

Verify concept ownership hasn't drifted:

- **L0** owns: constitutional DDL (including `extraction_state` forward-compatibility column on `tender_document_sets`), monorepo layout, pipeline stage logic, skill standard
- **L1** owns: daemon component listing (including pdf/ module), new tables introduced with the daemon, job kind implementation, test scenarios, run manifest + error manifest for extraction jobs
- **L3** owns: resource arbiter treatment of extraction jobs, extraction validation through dry-run validator
- **L4** owns: Supabase projections for extraction tables
- **layer-strategy.md** owns: schema stability matrix, schema evolution rules (job kinds), stage scope matrix, stage boundary contracts, testing evolution, component activation sequence
- **cross-reference-index** owns: navigation only (no planning authority)

Check:
- [ ] No layer doc has accidentally absorbed material that belongs in another layer doc
- [ ] Cross-references between docs are correct and bidirectional where appropriate
- [ ] Authority split (layer-strategy vs layer docs) is maintained — layer-strategy has cross-layer tables, layer docs have layer-local detail

---

## 5. Implementation Readiness

Ask: could an engineer implement PDF extraction from only the strategy docs (without reading the research doc)?

- [ ] Are the extraction tables fully described (columns, indexes, authority classification)?
- [ ] Is the daemon pdf/ module clearly specified (responsibilities, public API sketch)?
- [ ] Are the skill directories and their purpose clear?
- [ ] Are the job kinds clearly specified (trigger, resource consumption, execution mode, output)?
- [ ] Is the extraction orchestration flow clear (structural → semantic → domain table writes)?
- [ ] Is the caching/reuse mechanism described (content hash + engine version + strategy)?
- [ ] Is the extraction policy (document role → overlay mapping) described?
- [ ] Is the chunking strategy described?
- [ ] Could engineers reading L1 vs L3 vs layer-strategy derive compatible implementations?

If any answer is "no," add the missing detail. But be careful not to duplicate implementation details that belong in the research doc — the strategy docs should describe WHAT and WHERE, not HOW at the code level.

---

## 6. Overreach Check

Verify the harmonization didn't:

- [ ] Add implementation sequencing or task planning ("first implement the engine, then the skills")
- [ ] Include "first implement X, then Y" language
- [ ] Blur into execution guidance
- [ ] Over-specify implementation choices that should remain at the code level (e.g., specific Python Docling API calls, subprocess management details)
- [ ] Add cost model numbers from the research doc (those belong in the research doc only)
- [ ] Add benchmark accuracy numbers (those belong in the research doc only)

---

## 7. Absorption Completeness

Verify every concept from the research doc has a home in the strategy docs:

| Research Doc Concept | Expected Location | Status |
|---------------------|-------------------|--------|
| Two-stage hybrid architecture (Docling + Claude) | L1 component description | Check |
| Structural extraction engine (PdfExtractionEngine) | L1 daemon file table | Check |
| Deterministic chunking (synopsis, section-aware) | L1 daemon file table | Check |
| Extraction policy (document role → overlay mapping) | L1 daemon file table | Check |
| `document_extractions` table | L1 data introduced + layer-strategy §9 | Check |
| `document_semantic_extractions` table | Same | Check |
| `extraction_state` on `tender_document_sets` | L0 DDL | Check |
| Processing strategies (standard, force_full_page_ocr, fast_table_structure) | L1 component description | Check |
| Content-hash cache/reuse | L1 component or data description | Check |
| Supercession tracking (is_current, superseded_by) | L1 data description | Check |
| Base skill + 3 domain overlay skills | L1 skills listing + L0 monorepo | Check |
| headless_contract execution for semantic skills | L1 skill listing | Check |
| extract_document_structure job kind | layer-strategy §9 rule 6 | Check |
| extract_document_semantics job kind | Same | Check |
| Extraction substages within Stage 5 | L0 §3 pipeline + layer-strategy §10 | Check |
| Stage boundary: structural extraction → lot/product extraction | layer-strategy §10 | Check |
| Resource consumption: structural = no slots, semantic = agent_slot | L3 §1.4 | Check |
| Supabase projection for extraction tables | L4 projections | Check |
| Future overlays: compliance-certificate (L5), bid-template (L6) | L5/L6 forward-compatibility | Check |
| MIME-aware backend abstraction (future DOCX/HTML/XLSX) | L1 forward-compatibility | Check |
| 5 test files | layer-strategy §11 + L1 tests | Check |
| Migration 0006 | L1 data introduced | Check |
| Parent-child job cascade (first consumer of parent_job_id) | L1 data introduced or component desc | Check |
| Daemon-side persistence boundary (skills are side_effect_class: none) | L1 component or skill desc | Check |
| Entity relationship chain (raw_documents → extractions → semantic) | L1 data introduced | Check |
| Cross-authority FK discipline (raw_document_id is opaque ref, no SQL FK) | L1 data introduced | Check |
| "47 tables" → "49 tables" (2 locations) | cross-reference-index + layer-strategy §2 | Check |
| "17 core files" → "20 core files" (7 locations, 5 files) | L1 + layer-strategy + README + index | Check |
| L1-FR-DAEMON.2 requirement updated (17 → 20) | layer-1 §1.5 | Check |
| L1 ingestion flow step 11 includes extraction tables | layer-1 §1.6 | Check |
| L1 Definition of Done flow includes extraction steps | layer-1 DoD | Check |
| L4 Supabase inline enumeration (layer-strategy §6) | layer-strategy ~line 785 | Check |
| layer-strategy §3 Components Touched: Skills=6, daemon=20, DB adds 0006 | layer-strategy §3 (L1 table) | Check |

If anything is missing, add it. If something was intentionally excluded (e.g., cost model stays in research doc only), verify the exclusion is correct.

---

## 8. Terminology Consistency

Verify all documents use identical terminology for:

- "structural extraction" (not "structure extraction" or "structural parse")
- "semantic extraction" / "semantic mapping" (these may both appear — verify consistent usage)
- "PDF extraction toolkit" (the overall capability name)
- "document extraction" (may refer to the full pipeline or just structural — verify no ambiguity)
- "extraction overlay" / "semantic overlay" (verify consistent naming for the Claude skills)
- "document_extractions" (table name — singular "extraction" in column refs, plural in table name)
- "processing_strategy" (not "extraction_strategy" or "processing_mode")
- "document_role" (not "document_type" — avoid confusion with MIME type)
- "extraction_state" on tender_document_sets vs "semantic_extraction_state" on document_extractions — these must be clearly distinct

---

## 9. Fix and Final Fresh-Eyes Pass

Fix any issues found above with **targeted edits, not wholesale rewrites**.

After all fixes, re-read everything ONE MORE TIME. Imagine you are the implementation engineer and the strategy docs are the only thing you get. Could you still misread anything about:

- Where the PDF extraction code lives in the daemon?
- What tables it uses and what they store?
- How extraction fits into the 15-stage pipeline?
- What skills perform semantic mapping and how they're routed?
- How extraction jobs interact with the resource arbiter?
- What the extraction_state vs semantic_extraction_state distinction is?
- Whether extraction adds new pipeline stages (it doesn't)?

Fix anything that fails this test.

---

## 10. Required Output

Provide a concise summary:

1. **Documents changed** — list each file and what was modified
2. **Contradictions found and resolved** — specific contradictions removed
3. **Terminology corrections** — terms that were inconsistent and how they were unified
4. **Ambiguities tightened** — vague language that was made precise
5. **Missing content added** — concepts that were absent and where they were placed
6. **Issues NOT resolved** — anything that requires user input or further investigation

---

## 11. Follow-Up Fresh-Eyes Pass

> **Send this as a follow-up message in the same session after the optimization audit is complete.**

With completely fresh eyes, re-read just these sections (the highest-risk areas for post-harmonization inconsistency):

1. **layer-strategy.md §9** — Schema stability matrix. Count the table rows. Does the count match the "N tables" mentioned anywhere else? Are `document_extractions` and `document_semantic_extractions` in the right position (alphabetical or grouped with related tables)?

2. **layer-strategy.md §10** — Stage scope matrix. Does "Acquire docs" now mention extraction? Does the stage boundary contract for acquire_docs → extract_lots reference extraction output?

3. **layer-strategy.md §11** — Test count growth. Do the cumulative numbers add up? Does L1's count include the 5 new extraction test files?

4. **layer-1 §1.5** — Daemon file table. Count the files. Does the stated total match the actual count? Is the pdf/ module entry formatted like the existing entries?

5. **layer-0 §2** — DDL. Does `tender_document_sets` have `extraction_state`? Is it described as forward-compatibility (not as an active feature in L0)?

6. **cross-reference-index** — Do all new entries point to valid files and sections? Do any section numbers reference stale locations (shifted by prior edits)?

7. **Grep for "17" across ALL strategy docs** — Run: search every file in `docs/strategy/` for the literal string "17". Every hit that refers to daemon file count must now say "20." If any survive, fix them.

8. **Grep for "47" across ALL strategy docs** — Similarly, every reference to "47 tables" must now say "49 tables."

9. **L1 ingestion flow step 11** — Does the pipeline processing table list include `document_extractions` and `document_semantic_extractions`?

10. **L1 requirement L1-FR-DAEMON.2** — Does AC1 say "20 daemon files" (not 17)?

Fix any final issues. Report what you fixed.
