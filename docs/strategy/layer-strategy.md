# Layer Strategy — UK Public Procurement Intelligence Platform

> **Authority:** This document is the authoritative cross-layer strategy for the sequentially-shippable layer decomposition of the procurement intelligence platform. It supersedes ad-hoc milestone references elsewhere.
>
> **Source:** Derived from the 8,400-line research document (`docs/research/2026-03-08-public-procurement-intelligence-platform-research.md`) through structured investigation of all major subsystems, dependency mapping, and synthesis.
>
> **Layers:** 7 layers (0–6), each independently deployable, testable, and incrementally valuable.

---

## Glossary

Terms used throughout this document:

| Term | Definition |
|------|-----------|
| **PTY** | Pseudo-terminal — a Unix facility that emulates a terminal, enabling the daemon to run browser-automation agents as if they were typing in a real terminal. Required for interactive flows (login, CAPTCHAs, multi-step forms). |
| **Contract mode** | Non-interactive structured execution (JSON/JSONL I/O) — used for deterministic tasks that don't need a browser. Lighter than PTY. |
| **Push-owned** | A table whose authoritative copy lives in local SQLite, written by the daemon. A projection is pushed to Supabase for web viewing. The daemon is the authority; Supabase is a read-only mirror. |
| **Remote-owned** | A table whose authoritative copy lives in Supabase (e.g., operator decisions, shared config). The daemon pulls a local cache. |
| **CAS** | Compare-And-Swap — a concurrency pattern where an update includes the expected current version. If the version doesn't match (someone else changed it), the update fails rather than silently overwriting. Used for operator decisions. |
| **OQG** | Output Quality Gate — code-based checks (schema validation, statistical anomaly detection, drift detection) that run on every skill result before the semantic validator. |
| **RLS** | Row-Level Security — Postgres/Supabase feature that restricts which rows each database role can read/write. |
| **UDS** | Unix Domain Socket — local inter-process communication transport used between the desktop app and the daemon. |
| **FTS** | Find a Tender Service — UK government procurement notice portal. |
| **CF** | Contracts Finder — UK government procurement notice portal for lower-value contracts. |
| **PCS** | Public Contracts Scotland — Scottish procurement portal. |
| **OCDS** | Open Contracting Data Standard — structured format for procurement data used by UK government portals. |
| **`stage_run_id`** | An identifier for one atomic commit cohort within a pipeline stage. All outputs from one execution of one stage at one scope grain share a `stage_run_id`. Used for sync batch boundaries and rollback. |
| **`matchArchetype`** | Classification of how a product candidate's identity is recovered on marketplaces (e.g., `standard_class_led` for products found via standards, `dimension_material_led` for products found via physical specs, `model_exact` for branded products found by model number). Drives query strategy in marketplace search. |
| **Skill** | A versioned directory containing a Markdown playbook (SKILL.md), schemas, and test fixtures that tells an AI agent how to perform a specific task (e.g., scrape a procurement portal, extract products from tender documents). Skills are passive assets — the daemon orchestrates when and how they run. |
| **QueryService** | A Tauri-side Rust component that reads local SQLite directly (read-only) and returns typed `QueryEnvelope<T>` responses to the desktop frontend. No network dependency. |
| **DaemonBridge** | A Tauri-side Rust component that forwards desktop commands (mutations, triggers) to the background daemon process over UDS/named-pipe. Returns `MutationAck` responses. |
| **Channel\<T\>** | Tauri's typed streaming mechanism — used for live session output from the daemon to the desktop frontend (e.g., real-time terminal output from a running agent). |
| **Run manifest** | A structured Markdown document produced after every skill execution. Contains an agent-written narrative ("what I did") and a daemon-appended evidence appendix ("what the machine observed"). Used for debugging, operator review, and semantic validation. |

---

## Table of Contents

1. [Design Rationale](#1-design-rationale)
2. [Layer 0 — Gold Set + Evaluation Harness](#2-layer-0--gold-set--evaluation-harness)
3. [Layer 1 — Thin Local Runner](#3-layer-1--thin-local-runner)
4. [Layer 2 — Single-Operator Desktop MVP](#4-layer-2--single-operator-desktop-mvp)
5. [Layer 3 — Operational Hardening](#5-layer-3--operational-hardening)
6. [Layer 4 — Shared Visibility](#6-layer-4--shared-visibility)
7. [Layer 5 — Expanded Commercial Intelligence](#7-layer-5--expanded-commercial-intelligence)
8. [Layer 6 — Request Pipeline Automation](#8-layer-6--request-pipeline-automation)
9. [Cross-Layer: Schema Stability Matrix](#9-cross-layer-schema-stability-matrix)
10. [Cross-Layer: Interface Contracts](#10-cross-layer-interface-contracts)
11. [Cross-Layer: Testing Evolution](#11-cross-layer-testing-evolution)
12. [Cross-Layer: Milestone Mapping](#12-cross-layer-milestone-mapping)
13. [Cross-Layer: Stack Sequencing](#13-cross-layer-stack-sequencing)

---

## 1. Design Rationale

### Why These Boundaries

The layer decomposition follows three ordering principles from the research document's validation-first philosophy:

1. **Risk retirement, not architectural completeness.** The riskiest assumptions live in the intelligence pipeline — can the system extract product candidates, match them to listings, and produce margin estimates? Infrastructure is well-understood engineering that follows, not leads.

2. **Vertical capability slices, not horizontal stack layers.** Each layer delivers an end-to-end flow, not "all Rust components" or "all TypeScript." The first pipeline layer completes the full operational cycle from notice ingestion through verifiable output.

3. **Schema from day 1, features incrementally.** Sync tracking columns (`owner_id`, `row_version`, `last_local_seq`, `stage_run_id`) are present on all push-owned tables from their first creation in Layer 0. Validation fields (`parent_job_id`, `validation_required`, `validation_verdict_id`) exist as nullable columns on `workflow_jobs` from Layer 0. All columns needed in any future layer exist as nullable from table creation. The sync ENGINE and validation ENFORCEMENT are deferred — the schema provisions are not.

### Relationship to Research Document Milestones

The 7 layers map directly to the research document's M0–M6 milestones (§11). Deviations are minimal and justified inline. The research doc's milestone structure was validated through 3 planmaxer expert review iterations and 2 harmonymaxer feedback integrations, making it a reliable starting hypothesis that required no major reordering.

### Non-Negotiable Constraints Respected in Every Layer

These constraints from the research document apply to ALL layers without exception:

| Constraint | Source | Enforcement |
|-----------|--------|-------------|
| Authority matrix — every table classified, no mixed writes | ADR-002, §4.7 | Schema DDL enforces classification at table creation |
| Integer pence for money (`_pence` suffix) | §5.4 | Type system + linting |
| Integer basis points for confidence/percentage (`_bps` suffix, 0–10000) | §5.4 | Type system + linting |
| Process-group supervision with cleanup verdicts | ADR-004, §4.9 | From Layer 1 (first agent execution) |
| Run manifests for every skill execution | §4.10 | From Layer 1 (fallback machine-authored if agent fails) |
| Error manifests for every pipeline failure | §5.11 | From Layer 1 |
| Skill metadata declarations (SKILL.md + schemas) | §4.10, ADR-005 | From Layer 0 (first skill authored) |
| Conservative customs defaults (MFN, residual, standard VAT) | ADR-006, §4.5 | From Layer 0 (screening-grade) |
| No real-money bids without human approval | ADR-007 | All layers |
| Sync tracking columns on push-owned tables from creation | §5.4 | Schema DDL |
| `pipeline_assessments.validation_state = 'unverified'` from introduction | §5.4 | Schema DDL |

### Maintained Doc Set

The maintained planning set is exactly **8 files**. No companion docs, ADR addenda, or side decision files outside this set are authoritative for layer decomposition.

| File | Authority |
|------|-----------|
| `docs/strategy/layer-strategy.md` | Cross-layer source: layer ordering, schema stability matrix, interface contracts, stage-scope matrix, testing evolution, maintained-doc-set policy |
| `docs/strategy/layers/layer-0-gold-set-evaluation.md` | Layer-0-local narrative, scope, acceptance criteria, risks, and open questions |
| `docs/strategy/layers/layer-1-thin-local-runner.md` | Layer-1-local narrative, scope, acceptance criteria, risks, and open questions |
| `docs/strategy/layers/layer-2-single-operator-desktop.md` | Layer-2-local narrative, scope, acceptance criteria, risks, and open questions |
| `docs/strategy/layers/layer-3-operational-hardening.md` | Layer-3-local narrative, scope, acceptance criteria, risks, and open questions |
| `docs/strategy/layers/layer-4-shared-visibility.md` | Layer-4-local narrative, scope, acceptance criteria, risks, and open questions |
| `docs/strategy/layers/layer-5-expanded-commercial-intelligence.md` | Layer-5-local narrative, scope, acceptance criteria, risks, and open questions |
| `docs/strategy/layers/layer-6-request-pipeline-automation.md` | Layer-6-local narrative, scope, acceptance criteria, risks, and open questions |

Authority split inside the maintained set is fixed:

- `docs/strategy/layer-strategy.md` owns all cross-layer material: schema stability matrix, interface contracts, stage-scope matrix, testing evolution, stack sequencing, and doc-set policy.
- Each layer doc owns only layer-local narrative, capability, components, data first activated in that layer, stubs, definition of done, risks, and open questions.
- Open questions must live in the owning layer doc, not in a separate scratch file or review artifact.
- Temporary prompt artifacts, review handoffs, and iteration notes are explicitly non-authoritative and must never become de facto companion docs.

### Atomic Operation Pairs (Never Split Across Layers)

| Pair | First Appears | Rationale |
|------|---------------|-----------|
| Skill execution + run manifest | Layer 1 | Losing forensic data from any skill run is permanent |
| Job execution + stage status update | Layer 1 | Pipeline state machine becomes inconsistent without stage tracking |
| Local data mutation + outbox entry | Layer 4 | From sync activation, every push-owned write must create outbox entry in same SQLite transaction |
| Stage output write + `stage_run_id` + stage head advancement | Layer 1 | All outputs of a pipeline stage share a `stage_run_id`; stage head advances atomically |

---

## 2. Layer 0 — Gold Set + Evaluation Harness

### Narrative

Prove the intelligence thesis on a curated lot corpus before building any live-source runtime. This layer is evaluation infrastructure first: gold-set data, harness logic, skill authoring standards, and the screening-grade TypeScript pipeline exercised against curated inputs. No daemon, no desktop, no sync, and no remote authority. Skills run from the command line for benchmark execution only. Results land in a harness-local SQLite dataset and are verified by the automated evaluation harness and manual inspection. Layer 0 deliberately does **not** count as the first live pipeline layer; it is the measurement system that Layer 1 must satisfy.

**If we never build Layer 1:** The gold set and evaluation harness remain valuable as a benchmarking tool and intelligence pipeline prototype. The skills and TypeScript intelligence code can run standalone indefinitely.

### Capability Delivered

A curated gold-set lot corpus (50–100 lots) runs through extraction → matching → screening-grade cost estimation, producing per-lot precision/recall metrics against human-labeled ground truth. Three experiments are validated:

1. **Extraction quality** — ≥80% of lots yield usable product candidates
2. **Matching quality** — ≥70% of candidates with `matchArchetype` produce genuine matches
3. **Margin accuracy** — screening-grade margin bands agree with human expert on ≥75% of lots (±5pp)

### Components Touched

| Component | Action | Files |
|-----------|--------|-------|
| Workspace bootstrap | Create | `package.json`, `bunfig.toml`, `.env.example` |
| `packages/contracts-schema` | Create | `contracts.ts`, `jobs.ts`, `confidence.ts`, `zod.ts` |
| `packages/intelligence` | Create | 13 files (qualification, extraction, search, customs screening, margins, evaluation harness) |
| Skills | Create | 5 skills (document-acquisition, lot-extraction, product-extraction, preliminary-qualification, supplier-price-enrichment/ebay) |
| Database | Create | `0001_sqlite_state_initial.sql` — full pipeline schema including dormant tables for future layers (all columns present as nullable from creation) |

### Data Introduced

**SQLite pipeline tables (full schema — dormant tables included for forward compatibility):**

| Table | Authority | Purpose | Sync Tracking Columns |
|-------|-----------|---------|----------------------|
| `procurement_processes` | Push-owned (future) | Canonical internal record | `owner_id`, `row_version`, `last_local_seq`, `stage_run_id` present from creation |
| `contract_notices` | Push-owned (future) | Raw notice data | Same sync columns |
| `notice_lots` | Push-owned (future) | Per-lot entities | Same sync columns |
| `product_candidates` | Push-owned (future) | Extracted candidates | Same sync columns |
| `search_profiles` | Push-owned (future) | Structured search definitions | Same sync columns |
| `search_attempts` | Push-owned (future) | Per-wave search results | Same sync columns |
| `supplier_quotes` | Push-owned (future) | Normalized pricing | Same sync columns |
| `candidate_pricing_assessments` | Push-owned (future) | Gross margin results | Same sync columns |
| `pipeline_assessments` | Push-owned (future) | Process-level rollup | Same sync columns |
| `qualification_runs` | Push-owned (future) | Qualification decisions | Same sync columns |
| `landed_cost_estimates` | Push-owned (future) | Landed-cost results (screening-grade initially; all decision-grade columns present as nullable from creation) | Same sync columns |
| `bid_workflow_states` | Push-owned (future) | Bid workflow progression (dormant until Layer 6; schema present from creation) | Same sync columns |
| `manifests` | Push-owned (future) | Opportunity manifests (dormant until Layer 1 first live end-to-end local loop) | Same sync columns |
| `process_stage_status` | Push-owned (future) | Per-stage execution status records (dormant until Layer 1 activates stage tracking) | Same sync columns |
| `process_stage_heads` | Push-owned (future) | Latest successful stage run per scope (dormant until Layer 1) | Same sync columns |
| `tender_document_sets` | Push-owned (future) | Document acquisition output (dormant until Layer 1 first live-source document acquisition) | Same sync columns |

**Local-only orchestration tables (also in initial schema, dormant until later layers):**

| Table | Authority | Purpose |
|-------|-----------|---------|
| `workflow_jobs` | Local-only | Job lifecycle tracking (dormant until Layer 1). Forward-compatibility columns exist from creation: `parent_job_id` (nullable), `validation_required` (NOT NULL default `false`), `validation_verdict_id` (nullable). |
| `pending_operator_intents` | Local-only | Local decision-intent ledger/queue. Introduced early so Layer 2 can persist approve/reject/watch actions without violating the remote ownership of `operator_decision_heads`. Carries the future CAS payload (`process_id`, disposition, reviewed-basis metadata, idempotency key, status). |

**Junction tables (introduced with parent tables):**

| Table | Authority | Purpose |
|-------|-----------|---------|
| `notice_cpv_codes` | Push-owned (future) | Notice ↔ CPV code mapping |
| `product_commodity_codes` | Push-owned (future) | Product ↔ commodity code mapping |
| `lot_cpv_codes` | Push-owned (future) | Lot ↔ CPV code mapping |

**Critical schema decisions enforced at creation:**
- All monetary fields use integer pence (`_pence` suffix)
- All confidence/percentage fields use integer basis points (`_bps` suffix)
- All push-owned tables include `owner_id`, `row_version`, `last_local_seq`, `stage_run_id`, `created_at`, `updated_at`
- **All columns needed in later layers exist as nullable from table creation** — later layers populate existing columns, they never add new columns to tables introduced in earlier layers
- `pipeline_assessments.validation_state` exists from creation with default `'unverified'`
- `workflow_jobs.validation_required` exists from creation with default `false`; `parent_job_id` and `validation_verdict_id` are present and nullable from creation
- `product_candidates` requires `lotId` (not optional)
- `landed_cost_estimates` includes all decision-grade fields (duty breakdown, measure interaction results, exporter rate basis, supplementary units, valuation mode) as nullable — populated when Layer 5 activates the full customs engine
- `pipeline_assessments` includes all overlay and coverage fields (`reviewRequired`, `availabilityStatus`, `pricedCoverageBps`, `landedCostCoverageBps`, `netQualifiedCoverageBps`, `unresolvedMandatoryLots`, `decisionBasisHash`) as nullable — populated when Layer 2 activates coverage tracking

### Authority Classification

Layer 0 tables have three authority classifications, all permanent from creation:

- **Push-owned** (15 pipeline data tables + 3 junction tables): Daemon-authoritative, projected to Supabase in Layer 4.
- **Local-only** (`workflow_jobs`, `pending_operator_intents`): Never authoritative in Supabase. A redacted summary may be projected later, but the local tables themselves remain SQLite-only.
- **Remote-owned** tables are not introduced in Layer 0. Pre-sync operator decisions are captured only as local intents; the remote authoritative `operator_decision_heads` table arrives in Layer 4 and is remote-owned from introduction.

### Shared Library Additions

- `packages/contracts-schema/src/contracts.ts` — All TypeScript interfaces for pipeline entities
- `packages/contracts-schema/src/jobs.ts` — Job payload types
- `packages/contracts-schema/src/confidence.ts` — Confidence scoring types
- `packages/contracts-schema/src/zod.ts` — Runtime validation schemas

### Invariants Enforced

| Invariant | Type | Enforcement |
|-----------|------|-------------|
| Integer pence for all monetary values | Constitutional | DDL + type system |
| Integer basis points for confidence/percentage | Constitutional | DDL + type system |
| Authority matrix classification on all tables | Constitutional | DDL |
| Sync tracking columns on push-owned tables | Constitutional | DDL |
| Skill metadata declarations (SKILL.md + schemas) | Constitutional | Skill authoring guide + linting |
| Conservative customs defaults | Constitutional | Screening-grade implementation |
| `lotId` required on product candidates | Constitutional | DDL NOT NULL |
| Forward-compatibility nullable columns on `workflow_jobs` | Constitutional | DDL |

### What's Stubbed

| Feature | Stub | Full Implementation |
|---------|------|-------------------|
| Landed-cost engine | Screening-grade: HMRC Tariff API + flat freight + single FX | Layer 5: multi-source recall, 10-digit resolution, measure scanner |
| Marketplace search | Single source (eBay) | Layer 5: Alibaba/1688, Amazon |
| Skills execution | CLI invocation (`bun run skill-name`) | Layer 1: daemon-managed PTY/contract execution |
| Relation resolution | Gold set is pre-curated (no deduplication) | Layer 3: full heuristic resolution |
| Procurement sources | Gold set is pre-selected | Layer 1: first live FTS loop; Layer 2: adds Contracts Finder + desktop operator surface |
| Stage tracking | Implicit (no daemon) | Layer 1: explicit `stage_run_id` + stage heads |
| Sync | No sync — local SQLite only | Layer 4: push/pull sync engine |

### Definition of Done

1. Gold-set lot corpus contains ≥50 curated lots across whitelist categories (office supplies, cleaning products, basic PPE, hand tools, basic furniture, packaging materials) with human labels for product candidates, expected matches, and margin bands
2. Bilingual lexicon seeded for top 30–50 product families (EN/ZH noun aliases, market framing terms)
3. All 5 skills have complete SKILL.md + result.schema.json + checkpoint.schema.json + canary.inputs.jsonl + golden.assertions.yaml
4. Evaluation harness runs full gold-set corpus through extraction → matching → screening-grade cost, produces per-lot precision/recall report
5. Experiment 1 passes: ≥80% of gold-set lots yield at least one usable product candidate with correct unit, quantity, and specification
6. Experiment 2 passes: ≥70% of candidates with `matchArchetype` produce at least one genuine match (not false positive)
7. Experiment 3 passes: margin bands agree with human expert on ≥75% of gold-set lots (±5pp for screening-grade)
8. All 4 unit test suites pass (goods qualification, quoteability, tariff parsing, margin rules)
9. SQLite schema validates: all push-owned tables have sync tracking columns, all monetary fields are integer pence, all confidence fields are integer basis points
10. Each skill's SKILL.md includes instructions for the agent to write a structured narrative of what it did (a "run manifest") as part of its output — this prepares for the formal daemon-managed manifest system in Layer 1

### Test Scenarios

- **Fast inner loop:** Extraction parsers, margin math, scoring functions, data model validation — per-save, near-zero cost
- **Benchmark batch:** Full gold-set corpus through pipeline, compare to human labels — per-milestone, medium cost (~$2–10)
- Gold-set lot with 12 lots where 11 are irrelevant → system correctly identifies the 1 qualifying lot
- Mixed contract with separable goods → `addressableValuePence` reflects goods portion only
- Candidate with `matchArchetype = unknown` → system does not fabricate an archetype
- Screening-grade landed cost uses conservative defaults (MFN rate, residual duty, standard VAT)

### Dependencies

- None (greenfield)

### Forward-Compatibility Provisions

- Sync tracking columns on all tables (for Layer 4)
- Nullable validation columns on `workflow_jobs` (for Layer 3)
- `stage_run_id` column on pipeline tables (for Layer 1 stage tracking)
- Skill directory structure follows production standard (for Layer 1 skill linting)
- `SearchProfile` created for ALL candidates including below-threshold (for Layer 2 RFQ re-entry)
- `pending_operator_intents` exists from the start so local-only operator decisions can precede remote CAS without any table reclassification

### Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Gold-set too small for statistical significance | Medium | Start with 50 lots, expand per category; lot-level measurement (not notice-level) |
| Screening-grade landed cost too inaccurate for margin estimation | Medium | Conservative defaults ensure overestimation, not underestimation |
| eBay listings don't represent real supplier economics | High | eBay is first validation source only; Alibaba/1688 added in Layer 5 |

### Decisions Addressed

- ADR-001: Skills as primary data retrieval (first 5 skills authored)
- ADR-005: Skill directory standard established
- ADR-006: Internal landed-cost engine (screening-grade subset)

---

## 3. Layer 1 — Thin Local Runner

### Narrative

Minimal Rust runtime — just enough to launch skills, capture artifacts, supervise processes, and run the first live-source end-to-end local pipeline loop. PTY is available as a first-class capability, with contract mode for deterministic tasks. This remains a thin daemon layer, but it is also the first live pipeline layer: a manual CLI trigger ingests live FTS notices, runs the full local qualification/pricing/manifest loop, and emits verifiable local output without any UI or sync dependency.

**If we never build Layer 2:** The thin runner provides a reliable way to execute skills from the command line with full process supervision, artifact capture, and error manifests. It operates as a headless pipeline executor.

### Capability Delivered

10 consecutive batch runs of skills complete with ≥90% skill success rate and no orphaned processes. Skills are now daemon-managed with proper lifecycle: spawn → monitor → capture → validate → persist → reap. This layer also completes the first live operational loop:

`notice ingestion → pipeline processing → verifiable local output`

The verifiable output is a CLI-visible summary plus a local markdown opportunity manifest/file artifact for at least one live-source batch.

### Components Touched

| Component | Action | Files |
|-----------|--------|-------|
| Skills | Create | 2 (`procurement-ingestion/fts`, `manifest-generation`) |
| Rust workspace | Create | `Cargo.toml` (workspace root) |
| `crates/daemon` | Create | 14 core files (main, db/sqlite, pty/session+capture+health, jobs/runner, agents/adapter+codex+claude+artifacts+contracts, prompt/renderer, errors/manifest+markdown) |
| Database | Create | `0003_sqlite_artifacts.sql` (agent_session_logs, raw_documents, error_manifests) |

### Data Introduced

**SQLite tables (artifacts DB):**

| Table | Authority | Purpose |
|-------|-----------|---------|
| `agent_session_logs` | Local-only | Full session transcript/trace with `run_manifest_path`, `run_manifest_hash` |
| `raw_documents` | Local-only | Downloaded tender documents |
| `error_manifests` | Local-only | Full error context for pipeline failures |
| `executors` | Push-owned | Daemon instance identity (`owner_id`, `executor_id`, heartbeat) |
| `source_cursors` | Local-only | Per-source ingestion watermarks for incremental retrieval |

**Activated tables (created in Layer 0, now actively used):**

| Table | Change |
|-------|--------|
| `qualification_runs`, `product_candidates`, `search_profiles`, `search_attempts`, `supplier_quotes`, `candidate_pricing_assessments`, `landed_cost_estimates` | First **live-source** writes now active via the daemon-managed local pipeline loop |
| `tender_document_sets` | First live-source document acquisition writes now active |
| `manifests` | First live-source verifiable output now active (local markdown/file artifact + row) |
| `pipeline_assessments` | First live-source process rollups now active; `validation_state` starts as `unverified` from first write |
| `workflow_jobs` | Now actively used by Job Runner for job lifecycle; `parent_job_id`, `validation_required`, `validation_verdict_id` remain nullable |
| `process_stage_status` | Now actively written by Job Runner — records execution status per stage per scope |
| `process_stage_heads` | Now actively written — advances atomically when a stage succeeds for its full scope |

### Authority Classification

New tables are mostly **local-only** (no sync): `agent_session_logs` contains full PTY bytes and structured traces — too large and sensitive for sync; `error_manifests` contains full error context; `source_cursors` and `raw_documents` are per-daemon operational state. The exception is `executors` (push-owned) — the daemon's identity record that will be synced to Supabase in Layer 4. Redacted summaries of `agent_session_logs` and `error_manifests` will be pushed as separate projection tables in Layer 4. Source configuration is still bootstrapped locally in this layer (hardcoded/seeded runtime copy), not yet remote-owned shared config.

### Shared Library Additions

None (Rust-only layer).

### Invariants Enforced

| Invariant | Type | Enforcement |
|-----------|------|-------------|
| Process-group supervision | Constitutional | Every agent session in own pgid; cleanup verdict (clean/forced/leaked) mandatory |
| Run manifests for every skill run | Constitutional | Daemon fallback if agent fails to narrate |
| Error manifests for every failure | Constitutional | Error manifest system active |
| Session artifact capture | Constitutional | Raw bytes stored as compressed immutable artifacts |
| Adapter-mediated execution | Constitutional | PTY or contract mode per skill's `interaction_profile` |
| Stage tracking with `stage_run_id` | Constitutional | Job Runner scopes commits to correct grain |
| Scope-aware commit semantics | Constitutional | Set-replacement stages (e.g., lot extraction) must commit atomically for full scope or fail entirely; additive stages (e.g., per-candidate pricing) may commit independently per unit |

### What's Stubbed

| Feature | Stub | Full Implementation |
|---------|------|-------------------|
| Scheduler | CLI manual trigger only | Layer 3: cron scheduler |
| Resource arbiter | No concurrency limits | Layer 3: typed quotas |
| PTY hardening | Basic session creation | Layer 3: fd hygiene, EIO handling, terminal geometry |
| Job claiming/leases | Single-threaded execution | Layer 3: concurrent job claiming |
| Source registry | Bootstrapped local seed / hardcoded source definitions | Layer 3: typed registry; Layer 4: `shared_source_configs` pull |
| Procurement coverage | First live source only (FTS) | Layer 2: adds Contracts Finder; Layer 5: PCS if needed |
| Session reaper | Basic cleanup | Layer 3: full reaper with escalation |
| Tracing | Basic log capture | Layer 3: structured + forensic tracing |
| Validation | Fields exist, always `validation_required = false` | Layer 3: dry-run; Layer 4: enforcement |
| Desktop/operator surface | CLI summary + local file artifact only | Layer 2: desktop control panel |
| Sync | Tracking columns populated, no outbox entries | Layer 4: full sync engine |

### Definition of Done

1. `cargo check --all-targets` and `cargo clippy --all-targets -- -D warnings` pass
2. Rust binary launches skills via CLI (both PTY and contract mode)
3. Manual CLI trigger ingests a live FTS batch using source-batched skill execution
4. The first live end-to-end local loop completes for at least one notice: normalize → qualify → acquire docs → extract lots → extract products → price-signal match → gross gate → landed cost → manifest
5. Verifiable output exists without later layers: CLI summary and/or local markdown manifest artifact proves the loop completed
6. Process-group supervision verified: every session gets pgid, cleanup verdict recorded (clean/forced/leaked)
7. Artifact capture works: PTY bytes compressed and stored, session metadata in `agent_session_logs`
8. Run manifests generated: agent narrative + daemon evidence appendix; fallback machine-authored if agent fails
9. Error manifest system operational: pipeline failures produce Markdown error manifests with full context
10. Skill linting CI gate validates directory structure, metadata, schemas, trigger boundaries
11. Smoke canary harness executes: page fingerprint check + auth verification + minimal extraction per source
12. Experiment 4 passes: 10 consecutive batch runs, ≥90% success rate, 0 orphaned processes
13. `stage_run_id` correctly scoped per business unit grain for all live-pipeline stage commits

### Test Scenarios

- `agent_pty_session.rs` — PTY session creation, output capture, full lifecycle including reap and cleanup verdict
- `agent_contract_session.rs` — Contract-mode session, structured event capture, result contract validation
- `agent_procurement_ingestion.rs` — Live FTS batch produces a local end-to-end opportunity artifact/manifest without desktop or sync
- `adapter_codex.rs` — Codex adapter capability probe, JSONL parsing, mode selection
- `adapter_claude.rs` — Claude adapter capability probe, JSON parsing, mode selection
- `adapter_result_contract.rs` — Result contract validation, checkpoint sequencing, partial failure classification
- `skill_lint.rs` — Skill directory structure validation
- `skill_canary_harness.rs` — Live smoke canary execution
- Agent crashes mid-session → cleanup verdict is `forced`, run manifest has machine-authored fallback
- Orphaned grandchild process → detected and reaped by process supervisor

### Dependencies

- Layer 0 (skills, intelligence pipeline, contracts-schema, SQLite schema)

### Forward-Compatibility Provisions

- Adapter trait designed for extension (new agent CLIs)
- Error manifest format includes fields for validator reasoning (Layer 3)
- Agent session logs include `run_manifest_path` and `run_manifest_hash` (Layer 3 validation)
- Job Runner aware of `parent_job_id` (for Layer 3 validation sub-jobs)

### Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| PTY session reliability across macOS/Linux | Medium | Focus on macOS first (development platform); PTY hardening in Layer 3 |
| Agent CLI API changes break adapter | Low | Adapter abstraction isolates CLI changes; capability probe detects incompatibility |
| Process-group cleanup misses edge cases | Medium | Conservative kill escalation; leaked process detection |

### Decisions Addressed

- ADR-001: First live skill-based procurement ingestion loop
- ADR-002: Local-operational-authority half becomes live on real writes; sync engine remains deferred to Layer 4
- ADR-003: Tauri + separate Rust daemon (daemon starts here)
- ADR-004: PTY-based agent execution from day 1 (hybrid PTY + contract)
- ADR-005: Skills + explicit binding + daemon orchestration

---

## 4. Layer 2 — Single-Operator Desktop MVP

### Narrative

Prove repeatable operator value with a manual-trigger desktop sitting on top of the Layer-1 live local pipeline. This is the first user-facing layer — a single operator runs the system daily, reviews pipeline output in a desktop control panel, and makes accept/reject/watch decisions. This layer expands live procurement coverage to Contracts Finder, renders operator-facing manifests and coverage metrics, and persists single-operator decisions locally as queued intents without violating remote authority ownership.

**If we never build Layer 3:** A single operator can use the system with manual triggering for daily procurement intelligence. No cron, no automation, but the core value loop works.

### Capability Delivered

Single operator runs system daily for one week, making real accept/reject decisions. The pipeline surfaces genuine opportunities from live procurement data with screening-grade margin estimates. The desktop control panel provides list/detail views, agent session monitoring, and decision capture.

### Components Touched

| Component | Action | Files |
|-----------|--------|-------|
| Skills | Create | 2 (Contracts Finder ingestion, manifest generation) |
| `packages/intelligence` | Create | 1 (coverage-rollup.ts) |
| `packages/ui` | Create | 5 (index, contract-card-frame, manifest-viewer, pipeline-badge, sync-age-badge) |
| `packages/manifests` | Create | 2 (render-opportunity.ts, opportunity.md.ts template) |
| `apps/desktop` | Create | 14 files (full desktop shell) |

### Data Introduced

**Activated tables (created in prior layers, now actively used by the desktop/operator surface):**

| Table | Change |
|-------|--------|
| `tender_document_sets` | Existing live document sets are now rendered in desktop detail views with completeness surfaced to the operator |
| `manifests` | Existing Layer-1 local manifests are now rendered through the operator-facing manifest package |
| `pipeline_assessments` | Coverage metrics (`pricedCoverageBps`, `landedCostCoverageBps`, `netQualifiedCoverageBps`, `unresolvedMandatoryLots`, `decisionBasisHash`) and overlay fields (`reviewRequired`, `availabilityStatus`, `validationState`) are now surfaced to the operator |
| `product_candidates` | `candidateValueShareBps` is now surfaced for top-value selection and coverage rollups |
| `pending_operator_intents` | Now actively written as the local single-operator decision ledger/overlay queue for `approve` / `reject` / `watch`, including `reviewedBasisHash`, `reviewedManifestHash`, and `reviewedAtStageRunId` for later CAS flush |

### Authority Classification

No new remote-owned tables are introduced in this layer. The desktop writes only local data (`pending_operator_intents`) while the authoritative shared decision table remains deferred to Layer 4. This preserves the authority matrix: single-operator local mode first, remote CAS later.

### Shared Library Additions

- `packages/ui` — Frame + slot presentational components (contract cards, manifest viewer, pipeline badges, sync-age badges)
- `packages/manifests` — Opportunity manifest renderer

### Invariants Enforced

| Invariant | Type | Enforcement |
|-----------|------|-------------|
| Stage coherence | Constitutional | All stage outputs share `stage_run_id`; `process_stage_status` updated and `process_stage_heads` advanced atomically |
| Coverage metrics accuracy | Constitutional | Process-level state is rollup from candidate-level, not arbitrary |
| Decision staleness tracking | Constitutional | `reviewedBasisHash`, `reviewedManifestHash`, `reviewedAtStageRunId` captured on local decision intents from first operator use |
| Feature flags for new capabilities | Operational | Progressive enablement via flags |

### What's Stubbed

| Feature | Stub | Full Implementation |
|---------|------|-------------------|
| Scheduling | Manual trigger from desktop | Layer 3: cron scheduler |
| Relation resolution | Notices processed independently | Layer 3: merge/relate heuristics |
| Landed cost | Screening-grade (flat freight, single FX) | Layer 5: decision-grade |
| Search sources | eBay only (from Layer 0) | Layer 5: Alibaba/1688, Amazon |
| Sync to Supabase | None — local-only operation | Layer 4: full sync |
| Operator decisions | Local intent queue only — single-operator desktop overlay, no shared authority yet | Layer 4: remote-owned `operator_decision_heads` + Supabase CAS |
| Validation | Disabled (fields exist) | Layer 3: dry-run |
| Degraded-mode UX | Basic daemon-unavailable detection | Layer 4: three-state (healthy/degraded/offline) |

### Definition of Done

1. FTS ingestion skill scrapes live notices from Find a Tender
2. Contracts Finder ingestion skill scrapes live notices
3. Notice normalization produces canonical internal records
4. Goods/services classifier correctly categorizes notices (goods/services/works/mixed)
5. Desktop control panel launches with QueryService + DaemonBridge + bootstrap state machine
6. Notice list view with keyset pagination displays live pipeline output
7. Opportunity detail view shows margin bands, product candidates, supplier quotes
8. Decision capture (`reject` / `watch` / `approve`) persists locally in `pending_operator_intents` with reviewed-basis metadata and correct local overlay behavior
9. Agent session monitoring page shows live terminal output via Channel<T> streaming
10. Operator-facing coverage metrics displayed: `pricedCoverageBps`, `netQualifiedCoverageBps`
11. Decision-basis staleness indicators flag stale decisions
12. Single operator runs daily for 1 week and reports genuine opportunities surfaced

### Test Scenarios

- `agent_procurement_ingestion.rs` — Real FTS skill session, output verified
- `agent_supplier_enrichment.rs` — Real enrichment skill session, output verified
- `agent_health_detection.rs` — Stall detection, auth challenge detection
- `agent_error_manifest.rs` — Error manifest creation on failures
- `apps/desktop/e2e/contracts.spec.ts` — Desktop browsing and decision flow
- Notice with 12 lots → system processes each lot independently, displays per-lot results
- Mixed contract → goods-addressable value used for margin computation, not whole contract
- Expired notice → correctly filtered, not displayed as active opportunity
- Agent session crashes → error manifest created, operator can see failure context in desktop

### Dependencies

- Layer 0 (intelligence pipeline, skills, contracts-schema)
- Layer 1 (daemon runtime, artifact capture, process supervision)

### Forward-Compatibility Provisions

- Desktop bootstrap state machine detects daemon health modes (prep for Layer 4 degraded-mode UX)
- `packages/ui` frame + slot pattern designed for both desktop and web shells
- Coverage metrics on `pipeline_assessments` feed into Layer 4 sync projections
- `stage_run_id` on all stage outputs enables Layer 4 sync batch boundaries
- Local decision intents already carry the future CAS payload (`process_id`, disposition, reviewed-basis metadata, idempotency key), so Layer 4 can activate remote-owned decisions without any table reclassification

### Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Source website changes break FTS/CF skills | High | Error manifest flags failures; skill update cycle |
| Desktop UX reveals pipeline quality issues | Medium | Expected — this is why desktop comes before hardening |
| Single-source eBay pricing not representative | High | Known limitation; Layer 5 adds Alibaba/1688 |
| QueryService read performance on large datasets | Low | SQLite reads are fast; keyset pagination prevents full scans |

### Decisions Addressed

- ADR-003: Tauri v2 desktop shell (first desktop UI)
- ADR-008: Desktop = control panel with full read/write/trigger
- ADR-001: Live procurement source skills (FTS, Contracts Finder)

---

## 5. Layer 3 — Operational Hardening

### Narrative

Daily automation without babysitting. The daemon gains cron scheduling, batched ingestion, relation resolution, PTY hardening, resource management, and the worker-validator system in dry-run mode. The system runs unattended for 5 consecutive days. This layer transforms the single-operator tool into a production-grade pipeline.

**If we never build Layer 4:** A single operator has a fully automated, self-monitoring pipeline that runs daily, handles relation resolution, manages resources, and validates output quality via dry-run semantic checking. No multi-user visibility, but the core automation works.

### Capability Delivered

System runs unattended for 5 consecutive days with ≥95% skill success rate. Cron-scheduled ingestion, resource management, PTY hardening, source health monitoring, and dry-run worker-validator semantic validation are all operational. The skill promotion flow ensures quality gates for skill updates.

### Components Touched

| Component | Action | Files |
|-----------|--------|-------|
| `crates/daemon` | Create | 12 additions (scheduler/cron, sources/registry, jobs/claim, pty/reaper+hardening, agents/arbiter, db/checkpoints, tracing/structured+forensic, validation/policy+executor+manifest_writer) |
| Skills | Create | 9 (4 operational: auth-refresh/alibaba+1688, source-health/fts+alibaba; 5 validator base: SKILL.md + schemas + canary + golden + rubric) |
| `packages/intelligence` | Create | 1 (relations/resolve-process-relations.ts) |
| `packages/manifests` | Create | 2 (render-error-manifest.ts, error.md.ts template) |

### Data Introduced

**New tables:**

| Table | Authority | Purpose |
|-------|-----------|---------|
| `validation_verdicts` | Local-only | Full validator reasoning and evidence |
| `source_runtime_state` | Local-only | Circuit breaker state, cooldown tracking |
| `listing_evidence` | Local-only | Two-tier cache for search results + listing evidence |
| `process_relations` | Push-owned | Relation edges between processes |
| `process_aliases` | Push-owned | Redirect from retired process IDs |

**Modified tables:**

| Table | Change |
|-------|--------|
| `workflow_jobs` | `parent_job_id`, `validation_required`, `validation_verdict_id` now actively used for validation sub-jobs (still nullable — `validation_required` defaults to `false` except when policy triggers) |
| `agent_session_logs` | `run_manifest_path` and `run_manifest_hash` now actively populated by manifest writer |

### Authority Classification

| Table | Classification | Rationale |
|-------|---------------|-----------|
| `validation_verdicts` | Local-only | Full validator reasoning too large/sensitive for sync |
| `source_runtime_state` | Local-only | Per-daemon runtime state |
| `listing_evidence` | Local-only | Cache layer, not authoritative |
| `process_relations` | Push-owned | Shared visibility in Layer 4 |
| `process_aliases` | Push-owned | Stable deep links across desktop/web |

### Shared Library Additions

- `packages/intelligence/src/relations/resolve-process-relations.ts` — Relation resolution engine

### Invariants Enforced

| Invariant | Type | Enforcement |
|-----------|------|-------------|
| Explicit skill binding for cron | Constitutional | Scheduler binds skill id + version at enqueue time |
| Skill promotion flow gates | Constitutional | Draft → candidate → canary → shadow → stable → rollback |
| Fail-open validation | Constitutional | Validator failures never block pipeline; worker results advance |
| Dry-run isolation | Constitutional | Dry-run verdicts logged to scratch, no authoritative rows/outbox/stage heads advanced |
| Source circuit breaker | Operational | Repeated challenge-class failures → cooldown → only canary/auth jobs |
| Resource arbiter quotas | Operational | Browser slots, agent slots, source lanes |

### What's Stubbed

| Feature | Stub | Full Implementation |
|---------|------|-------------------|
| Validation enforcement | Dry-run only — verdicts logged, not enforced | Layer 4: enforcement mode |
| Domain-specific validators | Base validator only (generic semantic quality) | Layer 4: 4 domain overlays |
| Validator calibration | Initial calibration against gold-set manifests | Layer 4: formal calibration harness |
| Sync | Tracking columns populated, no outbox/push/pull | Layer 4: full sync |
| Multi-user | Single operator only | Layer 4: multi-user via web portal |

### Definition of Done

1. Cron scheduler runs on configured schedule with explicit skill binding
2. Batched ingestion: scheduled FTS + CF runs complete without manual intervention
3. Relation resolution engine correctly classifies notices (merged/related/possible_duplicate/separate)
4. PTY hardening: fd hygiene, EIO handling, terminal geometry stability verified
5. Host environment awareness: sleep/wake detection, catchup sweeps after resume
6. Resource arbiter enforces typed quotas (browser slots, agent slots, source lanes)
7. Skill promotion flow operational: draft → candidate → canary → shadow → stable
8. Source-health canaries and auth-refresh skills operational
9. Worker-validator dry-run: run manifests generated, base validator renders verdicts, verdicts logged without enforcement
10. Error manifest control-panel view displays failures with resolution context
11. System runs unattended for 5 consecutive days, ≥95% success rate, no manual intervention

### Test Scenarios

- `fault_process_cleanup.rs` — Orphaned grandchildren, dropped handle, cleanup timeout, leaked detection
- `fault_pty_edge_cases.rs` — PTY EOF/EIO handling, fd leak scenarios, terminal geometry
- `fault_host_events.rs` — Host sleep/monotonic gap, resource exhaustion, concurrent pressure
- `fault_partial_failure.rs` — Partial checkpoint then crash, result contract failures
- `replay_pty_sessions.rs` — Replay captured PTY byte streams against adapter parser
- `replay_contract_sessions.rs` — Replay captured sessions against adapter parser
- `cron_scheduler.rs` — Cron-based scheduling
- `job_claim.rs` — Concurrent job claiming and lease management
- `skill_golden_journeys.rs` — Invariant-based E2E per skill family
- `skill_shadow_compare.rs` — Quality metric diff between stable and candidate versions
- `skill_drift_monitor.rs` — Page fingerprint, fallback depth, fill-rate trending
- `validation_policy.rs` — All policy mode × trigger condition combinations
- `validation_executor.rs` — Validator sub-job spawning, kickoff prompt composition
- `validation_dry_run.rs` — Dry-run verdicts logged, pipeline not blocked
- `validation_golden_verdicts.rs` — Known-good/bad manifests → correct verdicts
- `resolve-process-relations.test.ts` — Relation scoring and hard blockers

### Dependencies

- Layer 0 (intelligence pipeline, gold set for validator calibration)
- Layer 1 (daemon runtime, agent adapter)
- Layer 2 (desktop shell for error manifest view, pipeline data model)

### Forward-Compatibility Provisions

- Validation policy engine ready for enforcement mode toggle (Layer 4)
- Run manifest corpus accumulates for validator calibration (Layer 4)
- `validation_verdicts` table schema includes all verdict metadata fields for Layer 4 projections
- Source circuit breaker state feeds into validation risk-stratified sampling

### Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Validator false positive rate too high for enforcement | Medium | Dry-run mode allows calibration before enforcement |
| Cron scheduling misses edge cases (timezone, DST) | Low | Catchup sweeps handle missed schedules |
| Relation resolution produces false merges | Medium | Conservative thresholds; operator can unmerge |
| Source changes break multiple skills simultaneously | Medium | Source-health canaries detect before production jobs |

### Decisions Addressed

- ADR-004: PTY hardening (full reliability)
- ADR-005: Explicit skill binding for cron (production reliability)

---

## 6. Layer 4 — Shared Visibility

### Narrative

Collaboration infrastructure. The sync engine activates, projecting pipeline data to Supabase for the read-only web viewing portal. Operator decisions flow through Supabase CAS. Worker-validator enforcement mode is enabled after dry-run calibration. Two users can now view the same opportunity pipeline — one via desktop (read-write), one via web (read-only).

**If we never build Layer 5:** Multi-user visibility works with screening-grade accuracy and single-marketplace pricing. The system is production-ready for the initial category whitelist.

### Capability Delivered

Two users view the same pipeline: desktop operator (read-write) and web viewer (read-only). Sync engine pushes pipeline data to Supabase. Operator decisions use compare-and-swap via Supabase. Validator rejections quarantine results in enforcement mode. Domain-specific validator overlays for procurement, supplier enrichment, document acquisition, and product extraction.

### Components Touched

| Component | Action | Files |
|-----------|--------|-------|
| Database | Create | 8 files (SQLite sync migration, 6 Supabase migrations, Supabase config) |
| `crates/daemon` | Create | 1 (db/sync.rs) |
| Skills | Create | 4 domain validator overlays (procurement-ingestion, supplier-enrichment, document-acquisition, product-extraction) |
| `apps/web` | Create | 7 files (full web shell) |

### Data Introduced

**SQLite sync infrastructure tables:**

| Table | Authority | Purpose |
|-------|-----------|---------|
| `local_outbox` | Local-only (SQLite sync infrastructure) | Transactional outbox entries with `content_hash` and `schema_epoch` from introduction |
| `sync_batches` | Local-only (SQLite sync infrastructure) | Batch metadata with `schema_epoch` |
| `pull_cursors` | Local-only (SQLite sync infrastructure) | Per-domain pull cursor position |
| `pending_operator_intents` | Local-only | Existing Layer-2 operator intent queue, now reconciled/flushed through remote CAS |

**Supabase projection tables (push-owned, mirroring local):**

All push-owned tables from Layers 0–3 get Supabase counterparts: `procurement_processes`, `contract_notices`, `notice_lots`, `tender_document_sets`, `qualification_runs`, `product_candidates`, `search_profiles`, `search_attempts`, `supplier_quotes`, `candidate_pricing_assessments`, `landed_cost_estimates`, `manifests`, `process_relations`, `process_aliases`, `pipeline_assessments`, `bid_workflow_states`, `process_stage_status`, `process_stage_heads`.

**Supabase summary tables (push-owned projections):**

| Table | Purpose |
|-------|---------|
| `workflow_job_summaries` | Redacted job status for web |
| `error_manifest_summaries` | Redacted error context for web |
| `validation_verdict_summaries` | Verdict status for web |
| `viewer_sync_status` | Sync freshness indicator |
| `executors` | Heartbeat summary |

**Supabase remote-owned tables:**

| Table | Purpose |
|-------|---------|
| `shared_company_profile` | Org-wide bidder profile |
| `shared_source_configs` | Source enablement and configuration |
| `operator_decision_heads` | Operator dispositions with CAS |

**Junction tables (Supabase):**

| Table | Purpose |
|-------|---------|
| `notice_cpv_codes` | Notice ↔ CPV code mapping |
| `product_commodity_codes` | Product ↔ commodity code mapping |
| `lot_cpv_codes` | Lot ↔ CPV code mapping |

### Authority Classification

Formalized and enforced at the Supabase database level via RLS:

| Role | Push projections | Push summaries | Remote config | Remote decisions |
|------|-----------------|----------------|---------------|-----------------|
| `viewer` | SELECT | SELECT | SELECT | SELECT |
| `operator` | SELECT | SELECT | SELECT | SELECT + UPDATE (CAS RPC) |
| `admin` | SELECT | SELECT | Full CRUD | Full CRUD |
| `daemon` | INSERT/UPDATE (batch RPC) | INSERT/UPDATE (batch RPC) | SELECT (pull) | SELECT (pull) |

### Shared Library Additions

- `apps/web` uses existing `packages/ui` components with web-specific data fetching hooks
- `packages/contracts-schema` types used for Supabase query typing

### Invariants Enforced

| Invariant | Type | Enforcement |
|-----------|------|-------------|
| Push batch atomicity | Constitutional | Single transactional RPC, not individual upserts |
| Outbox + data in same SQLite transaction | Constitutional | From activation, every push-owned mutation creates outbox entry atomically |
| CAS for operator decisions | Constitutional | `base_remote_revision` checked; mismatch → conflict |
| Active-writer lease | Constitutional | One daemon per org enforced at push protocol level |
| `schema_epoch` on every batch | Constitutional | Incompatible epoch → sync pauses, local operation continues |
| Batch boundaries never cross `stage_run_id` | Constitutional | Push dispatcher enforces |
| Single-in-flight batch | Constitutional | One batch at a time per push stream |
| Validation enforcement | Operational | Rejections quarantine results (after dry-run calibration) |
| RLS on all Supabase tables | Constitutional | Policy matrix enforced at database boundary |

### What's Stubbed

| Feature | Stub | Full Implementation |
|---------|------|-------------------|
| Schema epoch transitions | Initial epoch only (no transitions needed yet) | Future: expand/drain/upgrade/contract |
| Multi-org | Single org hardcoded | Future: org_id scoping |
| Full rebuild | Not implemented | Future: generation-based remote rebuild |
| Logical consistency auditor | Not implemented | Future: domain invariant checking |

### Definition of Done

1. Push protocol: transactional outbox entries created for all push-owned mutations in same SQLite transaction
2. Push batches applied to Supabase via single transactional RPC with `schema_epoch`, `row_version` ordering
3. Pull protocol: cursor-based pull for `shared_source_configs`, `shared_company_profile`, `operator_decision_heads`
4. Operator decisions: queued local intents flush through Supabase CAS RPC with `base_remote_revision`; conflicts returned (not silently overwritten) and local queue state updated
5. Active-writer lease: second daemon instance detected and rejected
6. RLS policies enforced for all 4 roles (viewer, operator, admin, daemon)
7. Web viewing portal displays opportunities, manifests, pipeline status (read-only)
8. Supabase Broadcast invalidation delivers live updates to web portal
9. `viewer_sync_status` shows sync freshness (green/amber/red)
10. Validation enforcement mode: validator rejections quarantine results as `AGENT_SUSPECT_OUTPUT`
11. 4 domain-specific validator overlays deployed and producing verdicts
12. Two users can view the same pipeline: desktop (read-write) + web (read-only)

### Test Scenarios

- `sqlite_schema.rs` — Full schema creation and migration
- `sqlite_supabase_sync.rs` — Push-pull round-trip sync
- `apps/web/e2e/contracts.spec.ts` — Web portal browsing
- Push batch with stale `row_version` → remote rejects (idempotent)
- Push batch with incompatible `schema_epoch` → sync pauses, local operation continues
- Two desktop operators make concurrent decisions → CAS detects conflict, second operator retries
- Validator rejects output in enforcement mode → results quarantined, error manifest created
- Validator itself crashes → fail-open, worker results advance with `validation_skipped_validator_error`
- Sync degraded → desktop shows browse-only mode, web shows staleness indicator
- Full push replay after sync recovery → idempotent, no duplicates

### Dependencies

- Layer 0–3 (all pipeline data, all push-owned tables with sync tracking columns)

### Forward-Compatibility Provisions

- `schema_epoch` exists for future schema evolution protocol
- RLS roles designed for future multi-org (`org_id` column planned)
- Projection tables designed for future materialized views

### Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Sync engine bugs corrupt remote data | Medium | Local copy is authoritative; remote can be republished |
| Validator enforcement blocks good data | Medium | Dry-run calibration in Layer 3; operator override available |
| Push batch size exceeds Supabase RPC limits | Low | Batch size bounded by `stage_run_id` scope |
| Network interruptions during push | Medium | Identical-envelope retries; degraded mode continues local operation |

### Decisions Addressed

- ADR-002: Sync / remote-projection half of the authority matrix fully operational
- ADR-008: Web viewing portal created (read-only)

---

## 7. Layer 5 — Expanded Commercial Intelligence

### Narrative

Broader categories, more supplier sources, decision-grade accuracy. The customs classification engine expands from screening-grade to full multi-source recall with 10-digit resolution, measure interaction, and trade remedy monitoring. Additional marketplace skills (Alibaba/1688, Amazon) provide multi-source pricing. FX engine gains dual-rate capability (HMRC customs + commercial spot). Freight engine gains real API integrations. Each new category requires its own gold-set validation before entering production.

**If we never build Layer 6:** The system is a mature procurement intelligence platform covering ≥3 goods categories with decision-grade landed cost and ≥2 supplier sources per category. Only request pipeline automation is missing.

### Capability Delivered

System handles ≥3 goods categories with decision-grade landed cost and ≥2 supplier sources per category. Full customs classification pipeline: multi-source recall (FPO + CPV priors + rule-based hints + tariff keyword search), multi-family expansion to UK 10-digit, exporter/manufacturer resolution, measure interaction engine, and trade remedy monitoring.

### Components Touched

| Component | Action | Files |
|-----------|--------|-------|
| Skills | Create | 4 (Amazon, Alibaba, 1688, PCS ingestion) |
| `packages/intelligence` | Create | 19 files (full customs, FX, freight, validation comparison) |

### Data Introduced

No new tables and no new columns — all decision-grade columns exist as nullable from Layer 0. Layer 5 populates previously-empty fields:

| Table | Columns Now Populated |
|-------|----------------------|
| `landed_cost_estimates` | Decision-grade fields: full duty breakdown, measure interaction results, exporter rate basis, supplementary unit requirements, valuation mode (all existed as nullable since L0) |
| `product_candidates` | Commodity code families from multi-source recall (via `product_commodity_codes` junction table, existed since L0) |
| `supplier_quotes` | Multi-source quotes (Alibaba, 1688, Amazon rows alongside existing eBay rows — same schema, more rows) |

### Authority Classification

No changes — all existing classifications hold.

### Shared Library Additions

- `packages/intelligence/src/customs/*` — Full customs classification engine (6 files)
- `packages/intelligence/src/fx/*` — Full FX engine (4 files)
- `packages/intelligence/src/logistics/*` — Full freight engine (5 files)
- `packages/intelligence/src/validation/*` — Landed-cost comparison adapters (3 files)

### Invariants Enforced

| Invariant | Type | Enforcement |
|-----------|------|-------------|
| Conservative customs defaults | Constitutional | MFN rates, residual duty, standard VAT when resolution is incomplete |
| Customs branch divergence as hard blocker | Constitutional | Materially different tariff branches → `CLASSIFICATION_AMBIGUOUS`, not averaged |
| Integer pence for all new cost fields | Constitutional | Type system |
| Per-category gold-set validation | Operational | New category requires gold-set expansion before production |

### What's Stubbed

Nothing significant — this is the full commercial intelligence implementation.

### Definition of Done

1. Alibaba/1688 search skills operational with Chinese keyword clusters
2. Amazon search skill operational
3. PCS (Scotland) ingestion skill operational
4. HMRC FPO API integration for HS code classification candidates
5. Multi-source recall produces candidates from ≥2 independent sources (FPO + CPV priors)
6. 10-digit resolution via HMRC Trade Tariff API produces UK-specific commodity codes
7. Measure scanner detects AD/CVD/safeguard/quota measures and models interactions
8. HMRC Exchange Rate Service provides customs valuation FX (monthly rolling)
9. Xe commercial FX provides spot rates
10. Freightos + SeaRates provide freight market data
11. Decision-grade landed cost per validated category (with full audit trail)
12. Category-specific gold-set expansion completed for ≥3 categories

### Test Scenarios

- `hmrc-fpo.test.ts` — FPO HS code classification
- Multi-family expansion: product with 3 plausible HS6 families → all UK10 descendants evaluated
- Trade remedy detection: steel product → AD/CVD measures detected, residual rate applied
- Compound duty: product with specific duty (£X/kg) → not flattened to percentage
- FX monthly rollover → forced recalculation of affected estimates
- Alibaba search with Chinese retry after English failure
- Category whitelist enforcement: medical device → rejected before search

### Dependencies

- Layer 0–4 (full pipeline, sync, web portal)

### Forward-Compatibility Provisions

- Classification freshness system (`tariffCheckedAt`, `classificationFreshUntil`, `tradeRemedyWatchState`) enables future auto-refresh
- CBAM forward placeholder for iron/steel/aluminium ready for future regulation

### Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| HMRC FPO API access restricted | Medium | FPO is not hard dependency; fallback to CPV-to-HS mapping |
| Anti-bot detection on Alibaba/1688 | High | Anti-bot classification table; source-health canaries; auth-refresh skills |
| Customs classification wrong for complex products | Medium | Multi-source recall + conservative defaults + abstention |
| Freight API rate variability | Medium | Dual-source cross-check (Freightos + SeaRates) |

### Decisions Addressed

- ADR-006: Internal landed-cost engine at full scope (decision-grade)
- ADR-001: Additional marketplace skills (Alibaba, 1688, Amazon)

---

## 8. Layer 6 — Request Pipeline Automation

### Narrative

Portal-specific bid skills automate the request pipeline — preparation and submission steps for tender portal applications. This does NOT place real-money bids; actual negotiations and fulfillment are manual. The system produces full audit trails (screenshots, logs, manifests) for every submission.

**If we never build this layer:** The system is fully functional for intelligence and decision-making. Operators manually prepare and submit portal applications based on system recommendations.

### Capability Delivered

System can prepare and submit a request on one portal (Find a Tender or Contracts Finder) with full audit trail. Includes agent-browser workflow, screenshot capture, and manifest generation for submitted requests.

### Components Touched

| Component | Action | Files |
|-----------|--------|-------|
| Skills | Create | 1 (bid-portal-proactis) |
| `packages/bidding` | Create | 3 (workflow registry, proactis workflow, company profile schema) |

### Data Introduced

**Modified tables:**

| Table | Enhancement |
|-------|-------------|
| `bid_workflow_states` | Fully active: `not_started` → `ready_to_bid` → `bid_in_progress` → `bid_submitted` / `bid_failed` |

### Authority Classification

`bid_workflow_states` is push-owned — daemon tracks bid progress, pushes to web portal.

### Shared Library Additions

- `packages/bidding` — Workflow registry and portal-specific workflow definitions

### Invariants Enforced

| Invariant | Type | Enforcement |
|-----------|------|-------------|
| No real-money bids without human approval | Constitutional | Bid workflow requires operator `approved` disposition |
| Bid workflow carries explicit scope | Constitutional | Which lots and candidates included |
| Full audit trail for submissions | Constitutional | Screenshots, logs, manifests captured |

### What's Stubbed

Nothing — this is the final layer.

### Definition of Done

1. Bid-portal skill navigates target portal via browser automation
2. Company profile data injected from `shared_company_profile`
3. Request preparation steps automated (form filling, document upload)
4. Submission step automated with screenshot capture at each stage
5. Full terminal output tracing for bid workflow
6. Manifest generated for submitted request with audit trail
7. Bid failure produces error manifest with portal-specific context
8. System can prepare and submit one request on the target portal end-to-end

### Test Scenarios

- Full bid workflow: approve → prepare → submit → manifest
- Bid failure mid-workflow: error manifest with screenshots at failure point
- Stale decision blocks bid: operator decision marked stale → bid workflow blocked until re-affirm
- Portal website changed: error manifest flags skill failure with step context

### Dependencies

- Layer 0–5 (full pipeline, desktop, web, sync, decision-grade intelligence)

### Forward-Compatibility Provisions

- Workflow registry pattern supports additional portal-specific skills
- Company profile schema designed for multiple portal requirements

### Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Portal UI changes break bid skill | High | Low impact — not real money; error manifest flags failures |
| Form structure varies by procurement type | Medium | Portal-specific workflow handles variance |

### Decisions Addressed

- ADR-007: Request pipeline automation (not real-money bids)

---

## 9. Cross-Layer: Schema Stability Matrix

### Table Introduction and Modification Across Layers

| Table | Created | First Active Use | Sync Activated | Authority |
|-------|---------|-----------------|----------------|-----------|
| `procurement_processes` | L0 | L0 | L4 | Push-owned |
| `contract_notices` | L0 | L0 | L4 | Push-owned |
| `notice_lots` | L0 | L0 | L4 | Push-owned |
| `product_candidates` | L0 | L0 | L4 | Push-owned |
| `search_profiles` | L0 | L0 | L4 | Push-owned |
| `search_attempts` | L0 | L0 | L4 | Push-owned |
| `supplier_quotes` | L0 | L0 | L4 | Push-owned |
| `candidate_pricing_assessments` | L0 | L0 | L4 | Push-owned |
| `pipeline_assessments` | L0 | L0 (gold-set harness), L1 (first live-source rollups), L2 (desktop coverage surface) | L4 | Push-owned |
| `qualification_runs` | L0 | L0 | L4 | Push-owned |
| `landed_cost_estimates` | L0 | L0 (screening-grade), L5 (decision-grade fields populated) | L4 | Push-owned |
| `tender_document_sets` | L0 | L1 (first live-source document acquisition) | L4 | Push-owned |
| `manifests` | L0 | L1 (first live-source verifiable output), L2 (desktop rendering) | L4 | Push-owned |
| `process_stage_status` | L0 | L1 (stage tracking) | L4 | Push-owned |
| `process_stage_heads` | L0 | L1 (stage tracking) | L4 | Push-owned |
| `bid_workflow_states` | L0 | L6 (bid automation) | L4 | Push-owned |
| `workflow_jobs` | L0 | L1 (job runner), L3 (validation fields) | Never (local-only) | Local-only |
| `notice_cpv_codes` | L0 | L0 | L4 | Push-owned (junction) |
| `product_commodity_codes` | L0 | L0 | L4 | Push-owned (junction) |
| `lot_cpv_codes` | L0 | L0 | L4 | Push-owned (junction) |
| `agent_session_logs` | L1 | L1 (run-manifest fields populated from introduction) | Never (local-only) | Local-only |
| `raw_documents` | L1 | L1 | Never (local-only) | Local-only |
| `error_manifests` | L1 | L1 | Never (local-only) | Local-only |
| `executors` | L1 | L1 | L4 | Push-owned |
| `source_cursors` | L1 | L1 | Never (local-only) | Local-only |
| `sources` | L1 | L1, L4 (pull from `shared_source_configs`) | Never (local-only) | Local-only (runtime copy) |
| `validation_verdicts` | L3 | L3 | Never (local-only) | Local-only |
| `source_runtime_state` | L3 | L3 | Never (local-only) | Local-only |
| `listing_evidence` | L3 | L3 | Never (local-only) | Local-only |
| `process_relations` | L3 | L3 | L4 | Push-owned |
| `process_aliases` | L3 | L3 | L4 | Push-owned |
| `local_outbox` | L4 | L4 | N/A (SQLite sync infra) | Local-only |
| `sync_batches` | L4 | L4 | N/A (SQLite sync infra) | Local-only |
| `pull_cursors` | L4 | L4 | N/A (SQLite sync infra) | Local-only |
| `pending_operator_intents` | L0 | L2 (desktop decision capture), L4 (CAS flush + conflict tracking) | Never (local-only) | Local-only |
| `operator_decision_heads` | L4 (Supabase + pulled local cache) | L4 | N/A (Supabase-native) | Remote-owned |
| `shared_company_profile` | L4 (Supabase) | L4, L6 (bid workflow fields populated) | N/A (Supabase-native) | Remote-owned |
| `shared_source_configs` | L4 (Supabase) | L4, L5 (new source entries) | N/A (Supabase-native) | Remote-owned |
| Summary tables (`workflow_job_summaries`, `error_manifest_summaries`, `validation_verdict_summaries`, `viewer_sync_status`) | L4 (Supabase) | L4 | N/A (push-only projections) | Push-owned |

### Schema Evolution Rules

1. **No table reclassification.** A table's authority classification is permanent from its introduction layer. `operator_decision_heads` is remote-owned from introduction in Layer 4; before that, local single-operator decisions exist only as `pending_operator_intents`.
2. **All columns exist from table creation.** Later layers populate existing nullable columns — they never add new columns to tables introduced in earlier layers. This eliminates schema migrations between layers.
3. **Sync tracking columns from day 1.** All push-owned tables include `owner_id`, `row_version`, `last_local_seq`, `stage_run_id`, `created_at`, `updated_at` from their creation in Layer 0, and those fields are populated on first write even before the outbox engine exists.
4. **No column renames.** Column names are stable across layers (important for sync field mapping).
5. **Junction tables introduced with parent tables.** `notice_cpv_codes` created alongside `contract_notices` in L0.
6. **Job kinds.** Each pipeline stage maps to a specific `job_kind` value in `workflow_jobs` (e.g., `sync_source`, `normalize_notice`, `qualify_notice`, `extract_products`, `enrich_supplier_prices`, `estimate_landed_cost`). The full mapping of 16 job kinds to pipeline stages is defined in the research document Appendix C.
7. **Validation forward-compatibility exists from job-table introduction.** `workflow_jobs.parent_job_id` is nullable from Layer 0, `workflow_jobs.validation_required` exists from Layer 0 with default `false`, and `workflow_jobs.validation_verdict_id` is nullable from Layer 0.
8. **Run-manifest columns exist from session-log introduction.** `agent_session_logs.run_manifest_path` and `agent_session_logs.run_manifest_hash` exist when `agent_session_logs` is introduced in Layer 1 and are populated from the first skill run.
9. **Validation state exists from assessment-table introduction.** `pipeline_assessments.validation_state` exists from Layer 0 with default `'unverified'`.

---

## 10. Cross-Layer: Interface Contracts

### Boundaries That Later Layers MUST NOT Break

| Interface | Introduced | Consumers | Contract |
|-----------|-----------|-----------|----------|
| Agent Adapter trait | L1 | All skill execution | Unified event stream regardless of PTY/contract mode; mode selection via `interaction_profile` |
| QueryService envelope | L2 | Desktop UI | `QueryEnvelope<T>` response format for all SQLite reads |
| DaemonBridge commands | L2 | Desktop UI | Mutation acknowledgment format; UDS/named-pipe transport |
| Channel<T> streaming | L2 | Desktop session viewer | Session streaming protocol |
| Skill directory standard | L0 | All skills, skill linting | SKILL.md + result.schema.json + checkpoint.schema.json + canary.inputs.jsonl + golden.assertions.yaml |
| Result contract format | L1 | Job Runner, OQG, validators | Schema-validated output format per skill |
| Push batch RPC | L4 | Sync engine → Supabase | `apply_push_batch(batch_json)` with `schema_epoch`, `row_version` ordering |
| CAS decision RPC | L4 | Desktop → Supabase | `update_operator_decision(process_id, disposition, base_revision, command_id)` — idempotent via `command_id` to handle lost-response retries |
| Pull-before-push ordering | L4 | Sync startup | Pull remote-owned state FIRST, then classify/coalesce pending intents, then resume push — ensures cross-authority consistency |
| `packages/contracts-schema` types | L0 | All TypeScript code | Canonical type definitions; changes must be backward-compatible |
| `packages/ui` frame + slot pattern | L2 | Desktop + web shells | Presentational components; data-fetching hooks are shell-specific |
| Evaluation harness API | L0 | Gold-set benchmarking | Pipeline runner + metrics computation interface |

### Stage Scope Matrix

`ProcessStageStatus.scopeKind` is constrained to exactly six values: `process`, `notice`, `lot`, `candidate`, `candidate_source_pair`, and `candidate_quote_scenario`. The strategy must keep every stage on one of these grains once a process/notice identity exists.

| Stage | Runtime Grain | `scopeKind` | Required Entities Already Introduced |
|-------|---------------|-------------|--------------------------------------|
| Ingest | source-batched precursor job | Not recorded in `process_stage_status` until notice/process identity exists | `sources`, `source_cursors`, procurement ingestion skill |
| Normalize | per notice | `notice` | `contract_notices` |
| Relate | per process cluster | `process` | `procurement_processes`, `contract_notices` |
| Qualify | per lot | `lot` | `notice_lots`, `qualification_runs` |
| Acquire docs | per notice | `notice` | `contract_notices`, `tender_document_sets` |
| Extract lots | per notice | `notice` | `contract_notices`, `notice_lots` |
| Extract products | per lot | `lot` | `notice_lots`, `product_candidates` |
| Score quoteability | per candidate | `candidate` | `product_candidates`, `search_profiles` |
| Price-signal match | per candidate-source pair | `candidate_source_pair` | `product_candidates`, `search_profiles`, `search_attempts`, `supplier_quotes` |
| Gross margin gate | per candidate | `candidate` | `candidate_pricing_assessments`, `supplier_quotes` |
| Landed cost | per candidate-quote scenario | `candidate_quote_scenario` | `candidate_pricing_assessments`, `landed_cost_estimates` |
| Net margin gate | per candidate | `candidate` | `landed_cost_estimates`, `pipeline_assessments` rollup inputs |
| Manifest | per process | `process` | `pipeline_assessments`, `manifests` |
| Operator decision | per process | `process` | `pending_operator_intents` first, `operator_decision_heads` once Layer 4 activates remote authority |
| Bid workflow | per process with explicit lot/candidate scope | `process` | `bid_workflow_states`, explicit bid scope payload |

Scope dependency rule:

- Lot-grain stages require `notice_lots` to exist in the same or an earlier layer.
- Candidate-grain stages require `product_candidates` to exist in the same or an earlier layer.
- Candidate-source-pair stages require `search_profiles` and `supplier_quotes` machinery to exist in the same or an earlier layer.
- Candidate-quote-scenario stages require a persisted candidate-scoped pricing assessment and selected quote before landed cost runs.

### Stage Boundary Contracts

These input/output contracts between pipeline stages are established in Layer 0 and must hold through all layers:

| Boundary | Required Input | Required Output | Established |
|----------|---------------|-----------------|-------------|
| qualify → acquire_docs | `QualificationResult` with lotId, expiryQualified, goodsQualified, accessRoute | Per-lot pass/review/reject | L0 |
| acquire_docs → extract_lots | Document set completeness, document roles | `TenderDocumentSet` with `documentSetHash` | L2 |
| extract_lots → extract_products | `NoticeLot` with title/description, lotNumber | At least one lot per qualified notice | L0 |
| extract_products → score_quoteability | `ProductCandidate` with lotId, headNoun, identityConfidence, candidateType, matchArchetype | At least one candidate per qualified lot | L0 |
| score_quoteability → price_match | quoteabilityScore, recallRiskScore, SearchProfile with queryWaves | quoteabilityScore ≥ 0.45 for auto-search | L0 |
| price_match → gross_gate | `SupplierQuote` with matchConfidence, quoteUsabilityConfidence, priceSignalRole | At least one quote per searched pair | L0 |
| gross_gate → landed_cost | Candidate-scoped pricing assessment, revenue basis, confidence gates | grossMarginBps, selected quote | L0 |
| landed_cost → net_gate | `LandedCostEstimate` anchored to candidate + quote | netMarginLowPct/BasePct/HighPct, blockers | L0 |
| net_gate → manifest | Process-level rollup with coverage metrics | Coverage-aware qualification | L2 |

---

## 11. Cross-Layer: Testing Evolution

### Testing Strategy Per Layer

| Layer | Fast Inner Loop | Live Smoke | Benchmark Batch | Release Gate |
|-------|----------------|------------|-----------------|--------------|
| L0 | Extraction parsers, margin math, scoring, data model | One skill + one lot through pipeline | Full gold-set corpus vs human labels | All skills, full pipeline, extended corpus |
| L1 | + Rust unit tests (adapter, PTY) | + Real skill session via daemon | + 10 consecutive batch runs | + Process supervision stress test |
| L2 | + Desktop component tests | + Desktop E2E flow | + Operator runs daily for 1 week | + Full desktop + pipeline E2E |
| L3 | + Validation policy logic, relation scoring | + Cron-triggered skill session | + 5 consecutive days unattended | + All fault injection + replay tests |
| L4 | + Sync protocol unit tests | + Push-pull round trip | + Multi-user concurrent access | + Full sync + web portal E2E |
| L5 | + Customs classification, FX, freight | + Multi-source search session | + Decision-grade accuracy vs gold set | + ≥3 categories, ≥2 sources each |
| L6 | + Bid workflow logic | + Portal navigation | + End-to-end bid submission | + Full audit trail verification |

### Test Count Growth

| Layer | New Test Files | Cumulative | Focus |
|-------|---------------|------------|-------|
| L0 | 4 | 4 | Intelligence pipeline unit tests |
| L1 | 7 | 11 | Adapter, PTY, skill linting |
| L2 | 5 | 16 | Agent E2E, desktop E2E |
| L3 | 16 | 32 | Fault injection, replay, validation, drift |
| L4 | 3 | 35 | Sync, schema, web E2E |
| L5 | 1 | 36 | Customs FPO |
| L6 | 0 | 36 | Covered by skill E2E tests |

---

## 12. Cross-Layer: Milestone Mapping

### Correspondence to Research Document Milestones (§11)

| Layer | Research Milestone | Deviation | Justification |
|-------|-------------------|-----------|---------------|
| L0 | M0: Gold Set + Evaluation Harness | None | Direct mapping |
| L1 | M1: Thin Local Runner | None | Direct mapping |
| L2 | M2: Single-Operator Desktop MVP | None | Direct mapping |
| L3 | M3: Operational Hardening | None | Direct mapping — includes dry-run validation |
| L4 | M4: Shared Visibility | None | Direct mapping — includes validation enforcement |
| L5 | M5: Expanded Commercial Intelligence | None | Direct mapping |
| L6 | M6: Request Pipeline Automation | None | Direct mapping |

**No deviations from M0–M6.** The research document's milestone structure was validated through 3 planmaxer expert review iterations and 2 harmonymaxer feedback integrations. The dependency analysis confirms that the milestone ordering correctly respects component dependencies, atomic operation pairs, and the validation-first delivery philosophy.

### Why No Merging or Splitting

- **M3/M4 remain separate:** M3 (dry-run validation) must accumulate a run-manifest corpus and calibrate false positive rates before M4 enables enforcement. Merging them would force simultaneous deployment of enforcement and calibration — a known anti-pattern.
- **M5/M6 remain separate:** Decision-grade landed cost (M5) is independently valuable without bid automation (M6). Merging forces the highest-complexity commercial intelligence and portal-specific bid skills into one layer.
- **M0 remains standalone:** The evaluation harness has ZERO runtime dependencies on the daemon. It must exist before pipeline code to anchor all quality metrics.

---

## 13. Cross-Layer: Stack Sequencing

### Technology Introduction Order

The research document's recommended stack sequencing (§11) is followed without deviation:

```
Layer 0: Skills (SKILL.md playbooks) + TypeScript intelligence
  ↓
Layer 1: + Thin Rust (daemon binary, PTY, process supervision)
  ↓
Layer 2: + Desktop UI (Tauri shell) + Shared UI (packages/ui)
  ↓
Layer 3: + Operational Rust (cron, resource arbiter, validation)
  ↓
Layer 4: + Supabase (sync engine, projections) + Web (Next.js portal)
  ↓
Layer 5: + Extended intelligence (customs, FX, freight APIs)
  ↓
Layer 6: + Bidding (portal-specific workflow skills)
```

### Rationale

1. **Skills first** — Cheapest to write, fastest to iterate, directly validate intelligence thesis. Run from CLI.
2. **TypeScript intelligence** — Pipeline logic lives here. This is where the product thesis is proven or disproven.
3. **Thin Rust** — Only enough daemon to validate runtime reliability. Not a full platform.
4. **Desktop UI** — Only after the pipeline produces results worth displaying.
5. **Operational Rust** — Hardening for daily unattended operation.
6. **Supabase + Web** — Only after a single operator has validated daily use.
7. **Extended intelligence** — Decision-grade accuracy for broader categories.
8. **Bidding** — Portal automation after the full intelligence stack is proven.

### Package Dependency Graph

```
packages/contracts-schema (L0)
  ← packages/intelligence (L0)
  ← packages/manifests (L2)
  ← packages/ui (L2)
  ← packages/bidding (L6)
  ← apps/desktop (L2)
  ← apps/web (L4)
  ← crates/daemon (L1, via SQLite schema alignment)
```

---

## Appendix: Component Activation Sequence

For quick reference, when each major component becomes active:

| Component | Layer | Mode |
|-----------|-------|------|
| Skill directory standard | L0 | Full |
| TypeScript intelligence pipeline | L0 | Screening-grade |
| Evaluation harness | L0 | Full |
| Gold-set lot corpus | L0 | Initial (50–100 lots) |
| Rust daemon binary | L1 | Thin (launch + capture + supervise) |
| Agent adapter (PTY + contract) | L1 | Full |
| Process-group supervision | L1 | Full |
| Run manifest writer | L1 | Full (with machine-authored fallback) |
| Error manifest system | L1 | Full |
| Stage tracking (`stage_run_id` + heads) | L1 | Full |
| Tauri desktop shell | L2 | Full control panel |
| QueryService (SQLite reads) | L2 | Full |
| DaemonBridge (daemon commands) | L2 | Full |
| Live session streaming | L2 | Full |
| Shared UI components | L2 | Full |
| Cron scheduler | L3 | Full |
| Resource arbiter | L3 | Full |
| PTY hardening | L3 | Full |
| Relation resolution | L3 | Full |
| Skill promotion flow | L3 | Full |
| Source health monitoring | L3 | Full |
| Validation policy engine | L3 | Dry-run |
| Validation executor | L3 | Dry-run |
| Base validator skill | L3 | Dry-run |
| Sync engine (push) | L4 | Full |
| Sync engine (pull) | L4 | Full |
| Supabase projections | L4 | Full |
| Web viewing portal | L4 | Full |
| Validation enforcement | L4 | Full |
| Domain validator overlays | L4 | Full |
| Full customs engine | L5 | Decision-grade |
| Full FX engine | L5 | Full |
| Full freight engine | L5 | Full |
| Multi-source marketplace search | L5 | Full |
| Bid workflow automation | L6 | Full |
