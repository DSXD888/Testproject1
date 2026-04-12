# PDF Extraction Toolkit — Implementation Strategy

Temporary strategy artifact for downstream beads creation. This document assumes the optimized feature research doc at `docs/research/2026-04-11-pdf-extraction-toolkit-research.md` is the controlling feature spec.

---

## 1. IMPLEMENTATION SURFACE MAP

### Core Feature: New Daemon Module (`crates/daemon/src/pdf/`)

| Code location | Current behavior | Required change | Complexity | Dependencies | Test impact |
|---|---|---|---|---|---|
| `crates/daemon/src/pdf/` (new module; likely `mod.rs` + docling wrapper + engine/policy/chunking subfiles) | No PDF/document extraction module exists. The daemon has no local subprocess abstraction for Docling, no structural cache/reuse logic, no extraction policy, and no chunk planner. | Add the structural extraction engine: Docling probe, subprocess launch/timeout/kill, output parsing, content-hash reuse lookup, current-row supersession logic, document-role resolution, expected-overlay policy, and chunk-plan generation for oversized docs. Keep this outside the `AgentAdapter` boundary. | Complex | `0006` migration; `db/sqlite.rs`; `main.rs` dispatch; skill result contracts | New structural-engine tests, reuse/supersession tests, timeout/OOM-path tests, real Docling E2E gate |

### Core Feature: New Skill Family (`skills/pdf-extraction/*`)

| Code location | Current behavior | Required change | Complexity | Dependencies | Test impact |
|---|---|---|---|---|---|
| `skills/pdf-extraction/base/` (new directory package) | No base PDF semantic mapping skill exists. | Add a base contract-mode package with the standard skill inventory (`SKILL.md`, `result.schema.json`, `checkpoint.schema.json`, `canary.inputs.jsonl`, `golden.assertions.yaml`). Base package owns shared run-manifest guidance, evidence-anchor expectations, and merge/chunk conventions. | Moderate | Prompt renderer; skill lint helpers; semantic result persister contract | New skill-inventory lint test; canary/golden validation |
| `skills/pdf-extraction/tender-specification/` | Does not exist. | Overlay for tender specs to lot/candidate/requirement extraction. Result contract must be designed for daemon-side persistence into authoritative procurement tables. | Complex | Base package; result persister; chunk-plan design | New contract-mode integration tests and goldens |
| `skills/pdf-extraction/pricing-schedule/` | Does not exist. | Overlay for rate/quantity/line-item extraction. Needs strong evidence anchors and conflict-friendly output because pricing tables are likely multi-page and repeated. | Complex | Base package; Docling table fidelity; merge logic | New contract-mode integration tests and goldens |
| `skills/pdf-extraction/bill-of-quantities/` | Does not exist. | Overlay for itemized quantity/unit/rate extraction with chunk-safe entity keys. | Complex | Base package; merge logic; result persister | New contract-mode integration tests and goldens |

### Core Feature: New Migration (`migrations/0006_*.sql`)

| Code location | Current behavior | Required change | Complexity | Dependencies | Test impact |
|---|---|---|---|---|---|
| `migrations/0006_pdf_extraction_toolkit.sql` (new file) | Latest migration is `0005_l3_operational_hardening.sql`; no extraction tables exist. | Create `document_extractions` and `document_semantic_extractions`; add `tender_document_sets.extraction_state`; respect authority-matrix rules by avoiding SQL FKs to local-only tables; add current-row and hash-lookup indexes. | Complex | Epoch decision; `db/sqlite.rs`; optimized feature doc schema | New migration-forward tests and schema assertions |
| `crates/daemon/src/db/sqlite.rs` | Registers migrations through `0005`; `DEFAULT_SCHEMA_EPOCH = 2`; tests assert the current migration inventory. | Add `0006` path constant and tuple to `MIGRATIONS`; bump `DEFAULT_SCHEMA_EPOCH` if we gate the feature; extend schema tests for the new tables/indexes. | Moderate | `0006` migration | Existing sqlite migration tests must be updated; add assertions for new tables/indexes |

### Supporting Changes: Job Runner Extensions

| Code location | Current behavior | Required change | Complexity | Dependencies | Test impact |
|---|---|---|---|---|---|
| `crates/daemon/src/jobs/runner.rs` | Handles claim/enqueue/persist for canonical pipeline stages and validation sub-jobs. Stage scope rules only cover the existing canonical stage list. No local-subprocess extraction path or skill-result persister registry exists. | Extend the runner-facing orchestration contract for `extract_document_structure` and `extract_document_semantics`. Important: do **not** add new canonical stage names. Add helper paths for subordinate capability jobs, supersession-safe persistence, and possibly a result-persister registry keyed by skill prefix. | Complex | `main.rs`; new pdf module; migration | New job-cascade tests and persistence-atomicity tests |
| `crates/daemon/src/jobs/claim.rs` | Claims any pending/retry job and renews leases; agnostic to job kind. | Likely no logic change, but verify lease/timeout behavior is appropriate for long-running Docling subprocesses. If extraction jobs need different lease cadence, this file becomes a light modification surface. | Trivial to Moderate | Structural runtime measurements | Covered by cascade/integration tests rather than bespoke unit tests |

### Supporting Changes: Main Runtime / Dispatch Routing

| Code location | Current behavior | Required change | Complexity | Dependencies | Test impact |
|---|---|---|---|---|---|
| `crates/daemon/src/main.rs` | Declares existing modules only; maps `pty_interactive` and `headless_contract`; manual skill flow has a hard-coded `procurement-ingestion/` persistence branch; `execute_skill_session()` only knows adapter-backed skill execution. | Add `mod pdf`; add workflow dispatch for `extract_document_structure` and `extract_document_semantics`; wire standalone/manual extraction entrypoints; keep semantic overlays on existing contract-mode execution; replace the one-off ingestion post-processing branch with a reusable result-persister surface (or at minimum add a parallel `pdf-extraction/` branch without regressing procurement-ingestion). Touchpoints are the module declarations, `skill_manifest()`, manual execution flow, `execute_skill_session()`, and the post-result persistence dispatch. | Complex | New pdf module; migration; skill packages; runner changes | Existing `agent_procurement_ingestion.rs` regression coverage plus new `pdf_semantic_extraction.rs` and `pdf_extraction_cascade.rs` |
| `crates/daemon/src/agents/adapter.rs` | Agent execution boundary is PTY vs Contract only. | Prefer no behavior change. This file is a guardrail: extraction must stay out of the adapter boundary. Only touch if the implementation accidentally tries to widen the enum, which should be rejected. | Trivial / guardrail | Design discipline | No new tests if unchanged; add negative design note in implementation review |
| `crates/daemon/src/agents/claude.rs` | Contract-mode launch path exists and is suitable for semantic overlays. | Reuse as-is for `pdf-extraction/*` skills. Only touch if a semantic overlay needs additional launch-env plumbing. | Trivial | Skill packages | Covered by semantic overlay integration tests |
| `crates/daemon/src/agents/contracts.rs` | Validates result contracts and OQG anomalies for contract-mode jobs. | Likely reuse, but confirm OQG inputs are sufficient for extraction overlays and add extraction-specific anomaly shaping only if needed. | Trivial to Moderate | Overlay result schemas | Covered by semantic overlay integration tests |
| `crates/daemon/src/agents/arbiter.rs` | Resource arbitration is framed around browser slots and agent slots. | Prefer no change for v1 because structural extraction consumes neither browser nor agent slots. If later we introduce `docling_slots`, this becomes a deliberate follow-up surface, not a prerequisite for the first implementation. | Trivial | Local subprocess path in `main.rs` | Indirect coverage via integration tests |
| `crates/daemon/src/prompt/renderer.rs` | Renders bound SKILL.md + job parameters + run-manifest instructions. | Likely no code change. Semantic overlays should fit the current prompt model. | Trivial | Skill packages | Covered by skill package tests/integration tests |
| `crates/daemon/src/validation/executor.rs` | Only base validator is loaded in Layer 3. | Likely no immediate feature change. The new overlays can run under the existing validation framework; domain-specific document validators can remain future work unless the feature implementation explicitly adds them now. | Trivial | Validation policy already in place | Existing validation tests should keep passing unchanged |

### Supporting Changes: Existing Table Modifications

| Code location | Current behavior | Required change | Complexity | Dependencies | Test impact |
|---|---|---|---|---|---|
| `tender_document_sets` via `0006` migration | Tracks completeness only; no extraction progress state. | Add `extraction_state` rollup and define how it is recomputed from per-document outcomes. | Moderate | Structural + semantic persistence | Migration tests; cascade tests |
| `raw_documents` (no schema change recommended) | Local-only source payload/storage tracking. | No schema expansion required for v1. Use existing `content_type`, `payload_json`, and `storage_path` as extraction inputs. | Trivial | pdf module | Indirect coverage |

### Supporting Changes: Downstream Lot/Product Consumers

| Code location | Current behavior | Required change | Complexity | Dependencies | Test impact |
|---|---|---|---|---|---|
| `crates/daemon/src/main.rs` (current deterministic lot/product materialization around `run_extract_lots_stage()` and the fixture-seeded `notice_lots` / `product_candidates` inserts) | The current live code path does not consume document-derived extraction outputs. It creates implicit lots and fixture-seeded candidates, which is sufficient for current L1-L3 tests but not for this feature. | Upgrade the downstream lot/product extraction path so Stage 6/7 consume `document_extractions` and `document_semantic_extractions` as first-class evidence. This is how the feature's semantic overlays become observable business behavior instead of dead-end side tables. | Complex | pdf module; semantic result persister; existing stage semantics | New `pdf_lot_product_refresh.rs` plus `pdf_extraction_cascade.rs` |

### Test Changes: New Test Files

| Code location | Current behavior | Required change | Complexity | Dependencies | Test impact |
|---|---|---|---|---|---|
| `crates/daemon/tests/pdf_structural_extraction.rs` (new) | No structural extraction test file exists. | Add subprocess-management, reuse-lookup, supersession, timeout, and schema-persistence tests for the pdf module. | Moderate | pdf module; migration | New |
| `crates/daemon/tests/pdf_semantic_extraction.rs` (new) | No semantic overlay integration test file exists. | Add contract-mode tests for `pdf-extraction/*` packages, result-contract validation, and daemon-side persistence into `document_semantic_extractions`. | Complex | Skill packages; result persister | New |
| `crates/daemon/tests/pdf_extraction_cascade.rs` (new) | No parent-child extraction cascade tests exist. | Add end-to-end cascade tests from `raw_documents`/`tender_document_sets` through structural extraction, semantic overlay execution, and downstream refresh enqueue decisions. | Complex | Runner/main wiring; migration; pdf module | New |
| `crates/daemon/tests/pdf_lot_product_refresh.rs` (new) | No test file verifies that Stage 6/7 consumers actually read extraction outputs. | Add focused tests proving lot/product refresh behavior consumes authoritative extraction rows rather than falling back to notice-only or fixture-only behavior. | Complex | main runtime wiring; extraction tables | New |
| `crates/daemon/tests/sqlite_pdf_extraction_migration.rs` (new or folded into existing sqlite tests) | Current sqlite tests only cover migrations through `0005`. | Add explicit migration coverage for `0006`, including authority-safe foreign-key choices and indexes. | Moderate | `db/sqlite.rs`; `0006` migration | New |
| `crates/daemon/tests/pdf_extraction_skill_inventory.rs` (new) | No lint/inventory tests for `skills/pdf-extraction/*` exist. | Add skill-directory linting and golden/canary presence checks for each new overlay package. | Moderate | Skill packages; `tests/support/skills.rs` | New |

### Test Changes: Test Harness Extensions

| Code location | Current behavior | Required change | Complexity | Dependencies | Test impact |
|---|---|---|---|---|---|
| `crates/daemon/tests/support/db.rs` | Opens temp state DBs only. | Add helpers for seeding `raw_documents`, `tender_document_sets`, and asserting extraction-table rows. | Moderate | `0006` migration | Used by all new extraction tests |
| `crates/daemon/tests/support/artifacts.rs` | Manages PTY capture artifacts. | Likely extend or supplement for local subprocess stdout/stderr artifact assertions. | Moderate | pdf module | Used by structural extraction tests |
| `crates/daemon/tests/support/skills.rs` | Lints standard skill packages. | Likely reusable as-is, but verify it supports the new pdf skill packages and add any metadata-bound checks only if the feature introduces new frontmatter fields. | Trivial | Skill packages | Used by skill inventory tests |
| `crates/daemon/tests/agent_procurement_ingestion.rs` | Exercises the current hard-coded procurement-ingestion post-processing path. | Update/regression-test if post-result persistence is refactored into a registry so existing ingestion behavior does not break. | Moderate | `main.rs` result persister refactor | Existing test file updated |
| `crates/daemon/tests/support/docling.rs` (new helper or equivalent) | No fake Docling subprocess boundary exists. | Add a test-only fake subprocess wrapper or fixture-fed adapter for fast supervision/persistence tests. Keep this clearly separate from the required real Docling E2E gate. | Moderate | pdf module | Used by structural/cascade tests |

---

## 2. SEQUENCING ANALYSIS

Execution order only: every item listed below ships as part of the feature. None of these steps are optional tiers, future phases, or scope cuts.

### What must be built first

1. `0006` schema + `db/sqlite.rs` registration + epoch decision.
2. `crates/daemon/src/pdf/` structural engine contract and reuse/supersession model.
3. `main.rs` / runner dispatch and result-persister surface, because semantic skills cannot land anything without it.

### What can be built in parallel

- Structural extraction engine implementation and pdf skill authoring can proceed in parallel once the schema contract is fixed.
- Skill package authoring for `tender-specification`, `pricing-schedule`, and `bill-of-quantities` can proceed independently after the base package and chunk/output conventions are frozen.
- Test fixture collection (sample PDFs + expected structural outputs) can run alongside daemon work.

### What must come last

1. End-to-end cascade wiring from `raw_document_id` / `tender_document_sets` through structural + semantic extraction.
2. Real Docling runtime gates on representative PDFs.
3. Migration / schema epoch release gate validation against a fresh DB and an upgraded DB.

### Critical path

1. Freeze schema contract.
2. Freeze chunk-plan + merge contract.
3. Implement structural engine.
4. Implement daemon result persister.
5. Author semantic overlay packages to the frozen result contracts.
6. Wire cascade and refresh behavior.
7. Run real-runtime gates.

---

## 3. DEPENDENCY SPINE

```text
0006 migration + sqlite registration + epoch decision
  -> pdf module can persist/read extraction rows
  -> main.rs can dispatch structural jobs against real tables
  -> result persister can write semantic rows and downstream entities
  -> semantic skill packages can finalize their result schemas
  -> cascade tests can assert authoritative outcomes

chunk-plan / merge design
  -> overlay result.schema.json shape
  -> main.rs semantic job payload shape
  -> document_semantic_extractions.extraction_scope semantics
  -> real E2E coverage for 100-page documents

result persister surface
  -> procurement-ingestion regression coverage
  -> pdf-extraction semantic overlay viability
  -> lot/product downstream refresh behavior

standalone raw_document_id entrypoint
  -> development without document-acquisition skill
  -> fixture-driven tests
  -> backfill/retry tooling
```

Blocking summary:

- Schema blocks everything else.
- Chunk/merge contract blocks overlay schemas and authoritative persistence.
- Result persister blocks semantic overlays from becoming real system outputs.
- Real Docling benchmark gate blocks release confidence even if unit/integration tests pass.

---

## 4. TESTING STRATEGY

### a. Existing tests to update

- `crates/daemon/tests/agent_procurement_ingestion.rs`
  Current purpose: validates the existing procurement-ingestion execution + hard-coded persistence path.
  Required change: if persistence becomes registry-driven, keep these tests green and add a regression assertion that `procurement-ingestion/*` still maps to the correct persister.

- `crates/daemon/src/db/sqlite.rs` tests
  Current purpose: migration inventory / reopen behavior through `0005`.
  Required change: assert `0006` applies cleanly, new tables exist, and the epoch bump behavior matches the chosen policy.

- `crates/daemon/tests/support/skills.rs`
  Current purpose: standard skill-directory linting.
  Required change: verify it cleanly accepts the new `skills/pdf-extraction/*` packages; extend only if the feature introduces new mandatory metadata.

### b. New tests to create

- Structural extraction engine tests
  Cover probe success/failure, missing Docling runtime, timeout kill path, content-hash reuse, clone-without-rerun cache hit, supersession of older current rows, and persistence of structural metrics/warnings.

- Semantic mapping skill integration tests
  Cover each overlay's contract shape, evidence anchors, chunked vs single-pass behavior, and daemon persistence into `document_semantic_extractions`.

- Job cascade integration tests
  Cover `raw_document_id` standalone path and `tender_document_sets` path.
  Assert parent-child job linkage, semantic job enqueue decisions, and downstream refresh enqueue only when authoritative outputs changed.

- Schema migration tests
  Cover fresh DB apply and upgraded DB apply, plus authority-safe absence of SQL FKs from push-owned extraction tables to local-only tables.

- Skill metadata lint tests
  Cover `skills/pdf-extraction/base/`, `.../tender-specification/`, `.../pricing-schedule/`, and `.../bill-of-quantities/` for standard package completeness and valid frontmatter.

### c. E2E / real-runtime gates

- End-to-end happy path
  Real sample PDF -> real Docling structural output -> `document_extractions` row -> real Claude/fake-approved overlay path -> `document_semantic_extractions` row -> authoritative domain-table writes.

- Docling health probe gate
  Fresh runtime validates Python import, package version detection, configured artifacts/model path, and wrapper executability before admitting structural jobs.

- Contract-mode result validation gate
  Semantic overlay output must pass result-schema validation and OQG before daemon-side persistence occurs.

- Large-document gate
  At least one representative 80-100+ page tender pack must exercise the chunk-plan + merge path.

- Multi-page table gate
  At least one pricing schedule or BoQ with a page-spanning table must be part of the gold corpus.

### d. Test infrastructure gaps

- Fake Docling subprocess boundary
  Acceptable for fast supervision/persistence tests because it substitutes an external binary boundary, not an internal domain dependency.
  This does **not** replace the required real Docling E2E gate.

- Sample PDF fixtures
  Need representative UK procurement docs: clean born-digital spec, scanned spec, pricing schedule with dense tables, BoQ with repeated headers, and a failure fixture (encrypted/corrupt).

- Expected extraction-output fixtures
  Need precomputed structural outputs for deterministic contract tests and expected semantic assertions for goldens.

### e. Surface-to-test coverage matrix

Every named implementation surface above must be touched by at least one concrete test. Minimum coverage plan:

| Surface | Test(s) touching it |
|---|---|
| `crates/daemon/src/pdf/` | `pdf_structural_extraction.rs`, `pdf_extraction_cascade.rs`, real Docling E2E gate |
| `skills/pdf-extraction/base/` | `pdf_extraction_skill_inventory.rs`, `pdf_semantic_extraction.rs` |
| `skills/pdf-extraction/tender-specification/` | `pdf_extraction_skill_inventory.rs`, `pdf_semantic_extraction.rs`, `pdf_lot_product_refresh.rs` |
| `skills/pdf-extraction/pricing-schedule/` | `pdf_extraction_skill_inventory.rs`, `pdf_semantic_extraction.rs` |
| `skills/pdf-extraction/bill-of-quantities/` | `pdf_extraction_skill_inventory.rs`, `pdf_semantic_extraction.rs` |
| `migrations/0006_pdf_extraction_toolkit.sql` | `sqlite_pdf_extraction_migration.rs`, existing `db/sqlite.rs` migration tests |
| `crates/daemon/src/db/sqlite.rs` | existing sqlite migration tests, `sqlite_pdf_extraction_migration.rs` |
| `crates/daemon/src/jobs/runner.rs` | existing `agent_contract_session.rs`, new `pdf_extraction_cascade.rs` |
| `crates/daemon/src/jobs/claim.rs` | existing `job_claim.rs`, new `pdf_extraction_cascade.rs` |
| `crates/daemon/src/main.rs` | existing `agent_procurement_ingestion.rs` regression coverage, new `pdf_semantic_extraction.rs`, `pdf_extraction_cascade.rs`, `pdf_lot_product_refresh.rs` |
| `crates/daemon/src/agents/adapter.rs` | existing `adapter_claude.rs`, existing `agent_contract_session.rs` |
| `crates/daemon/src/agents/claude.rs` | existing `adapter_claude.rs`, new `pdf_semantic_extraction.rs` |
| `crates/daemon/src/agents/contracts.rs` | existing `agent_contract_session.rs`, new `pdf_semantic_extraction.rs` |
| `crates/daemon/src/agents/arbiter.rs` | existing `resource_arbiter.rs`; if behavior changes, extend `pdf_extraction_cascade.rs` to assert semantic jobs consume agent slots while structural jobs do not |
| `crates/daemon/src/prompt/renderer.rs` | existing `agent_contract_session.rs`, new `pdf_extraction_skill_inventory.rs` |
| `crates/daemon/src/validation/executor.rs` | existing `validation_executor.rs`, new `pdf_semantic_extraction.rs` if extraction overlays run under live validation |
| `crates/daemon/tests/support/db.rs` | all new extraction tests |
| `crates/daemon/tests/support/artifacts.rs` | `pdf_structural_extraction.rs` |
| `crates/daemon/tests/support/skills.rs` | `pdf_extraction_skill_inventory.rs` |
| `crates/daemon/tests/support/docling.rs` | `pdf_structural_extraction.rs`, `pdf_extraction_cascade.rs` |

---

## 5. SCHEMA EPOCH DECISION

### Recommendation: bump to epoch `3`

Reasoning:

- This feature is not "just another table." It introduces new push-owned authoritative entities, new workflow job kinds, new dispatch logic, and a new result-persister surface.
- The current daemon binary does not know how to execute or persist these jobs. Letting an epoch-2 binary open an epoch-2 database that now contains queued extraction jobs or partially migrated developer state invites silent mismatch.
- The project explicitly does not value backwards compatibility at this stage. A clean schema/runtime gate is therefore preferable to permissive mixed-version behavior.

Concrete implication:

- Add `0006_pdf_extraction_toolkit.sql`.
- Bump `DEFAULT_SCHEMA_EPOCH` from `2` to `3`.
- Register `0006` in `MIGRATIONS` with epoch `3`.
- Treat the feature rollout as "new daemon binary + new schema together," which is cleaner than pretending old binaries are valid observers of the new extraction state.

---

## 6. RISK ASSESSMENT

| Risk | Type | Why it matters | Mitigation |
|---|---|---|---|
| Docling subprocess hangs, OOMs, or runs with missing models | Technical | Structural extraction is the first gate; if it wedges, the whole cascade stalls | One-process-per-job model, explicit timeout + kill escalation, startup health probe, install-time model bootstrap, artifact capture for failure triage |
| Page-spanning tables fragment into multiple structural tables | Technical / quality | Pricing schedules and BoQs are the highest-value inputs; broken structure poisons semantic extraction | Gold corpus must include multi-page tables; chunk/merge path must tolerate repeated headers and split fragments; keep fallback review path |
| Result persistence remains hard-coded and brittle | Integration | Without a real persister abstraction, each new skill family becomes another `main.rs` special case | Introduce a small skill-prefix-to-persister registry, and regression-test procurement-ingestion while adding pdf-extraction |
| Aggregate semantic state becomes meaningless | Integration | `semantic_extraction_state` is useless unless "complete relative to what?" is answerable | Persist `expected_overlays_json` + `extraction_policy_version`; recompute aggregate state only against that policy result |
| Old authoritative rows remain query-visible after reruns | Data correctness | Without current-row semantics, downstream readers may see stale structural or semantic rows | Add `is_current` + supersession fields and unique partial indexes; enforce current-row flip in the same transaction as new row creation |
| Chunking design arrives too late | Technical / delivery | Semantic overlays will fail on the first 100-page tender pack if chunking is bolted on after the fact | Freeze chunk-plan + merge contract before authoring overlay schemas; require a large-document test fixture before sign-off |
| Real Docling throughput differs materially from plan | Performance | Local-compute cost is low, but wall-clock time and memory may still be unacceptable on target hardware | Replace paper estimates with benchmark gates on target hardware and representative PDFs; use those numbers for admission/concurrency tuning |
| Document-acquisition skill absence delays integration | Delivery | The full live-source flow does not exist yet | Build against `raw_document_id` first; use standalone/manual path plus seeded fixtures; keep acquisition as a trigger path, not a prerequisite |
| XLSX pricing schedules prove materially better with direct cell parsing | Scope / architecture | PDF-first design may underperform on spreadsheet-native pricing packs | Keep the abstraction MIME-aware from the start; document the XLSX exception explicitly; compare Docling vs direct parser on a gold subset before locking the spreadsheet path |
