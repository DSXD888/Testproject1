# Cross-Reference Index — Layer Strategy

> Quick lookup: "if you're looking for X, it's in file Y, section Z."
>
> **Normative planning docs:** `layer-strategy.md` + 7 per-layer docs in `layers/`. **Navigation aids (no planning authority):** this index + `README.md`.

## Implementation Status Snapshot (2026-05-02)

Entries in this index that point at L1-L3 strategy sections describe **built** code unless their layer doc says otherwise in an inline status callout. Use the implementation references below before treating historical strategy prose as future work.

| Layer / Topic | Strategy Entry | Impl |
|---------------|----------------|------|
| L1 daemon bootstrap and runtime state | `layers/layer-1-thin-local-runner.md` §§1-2 | `crates/daemon/src/main.rs:2148`, `crates/daemon/src/main.rs:2172` |
| L1 PTY and adapter execution | `layers/layer-1-thin-local-runner.md` §§1.7-1.8, §2.3 | `crates/daemon/src/agents/adapter.rs:12`, `crates/daemon/src/agents/adapter.rs:103`, `crates/daemon/src/pty/session.rs:19` |
| L1 artifact and run-manifest system | `layers/layer-1-thin-local-runner.md` §§2.5, 4.2 | `crates/daemon/src/agents/artifacts.rs:15`, `crates/daemon/src/validation/manifest_writer.rs:9`, `migrations/0003_sqlite_runtime.sql:3` |
| L1 stage tracking | `layers/layer-1-thin-local-runner.md` §3.3 | `migrations/0001_sqlite_state_initial.sql:453`, `migrations/0003_sqlite_runtime.sql:99`, `crates/daemon/src/main.rs:259` (`candidate_quote` landed-cost scope) |
| L2 Tauri desktop shell and command surface | `layers/layer-2-single-operator-desktop.md` §§1.3-1.5 | `apps/desktop/src-tauri/src/main.rs:66`, `apps/desktop/src-tauri/src/main.rs:1919`, `crates/daemon/src/ipc/command_router.rs:95` |
| L2 read models and manifest inbox | `layers/layer-2-single-operator-desktop.md` §§1.4, 1.9, 3 | `apps/desktop/src-tauri/src/query_service.rs:219`, `apps/desktop/src-tauri/src/query_service.rs:593`, `apps/desktop/src/pages/ManifestInbox.tsx:143` |
| L2 operator decisions and clarifications | `layers/layer-2-single-operator-desktop.md` §3.2 | `migrations/0004_l2_pending_operator_intents.sql:3`, `migrations/0009_operator_input_overrides.sql:1`, `crates/daemon/src/ipc/handlers/decisions.rs:56`, `crates/daemon/src/ipc/handlers/clarifications.rs:90`, `apps/desktop/src/pages/OperatorDecisionPanel.tsx:247` |
| L3 scheduler, source registry, and resource arbiter | `layers/layer-3-operational-hardening.md` §§1.3-1.5, 3.4-3.5 | `crates/daemon/src/scheduler/cron.rs:15`, `crates/daemon/src/scheduler/cron.rs:140`, `crates/daemon/src/sources/registry.rs:210`, `crates/daemon/src/agents/arbiter.rs:152` |
| L3 job claiming and validation dry-run | `layers/layer-3-operational-hardening.md` §§1.3, 2, 4.1-4.2 | `crates/daemon/src/jobs/claim.rs:72`, `crates/daemon/src/validation/policy.rs:238`, `crates/daemon/src/validation/executor.rs:25`, `migrations/0005_l3_operational_hardening.sql:14` |
| L3 relation resolution | `layers/layer-3-operational-hardening.md` §3.6 | Implemented narrowly as OCID-exact `merged_same_process` only in `crates/daemon/src/main.rs:19175`; heuristic relation classes remain deferred. |
| L3 landed-cost hardening | `layers/layer-3-operational-hardening.md` (implementation addendum) | `OrderQuantityBasis::BasisUndetermined` and operator overrides in `crates/daemon/src/main.rs:3473`, `crates/daemon/src/main.rs:4058`, `crates/daemon/src/main.rs:4197`; see `docs/pipeline/stage-11-hardening-order-quantity-validation-manifest.md`. |
| L4 shared visibility | `layers/layer-4-shared-visibility.md` | **BUILT** 2026-05-08. Sync engine in `crates/daemon/src/db/sync.rs`, 8 Supabase migrations under `infra/supabase/migrations/`, web portal under `apps/web/`, validator overlays under `skills/validation/overlay-*`. Pipeline archived at `docs/pipeline/l4-shared-visibility/archived/2026-05-08-l4-shared-visibility/`; artifacts (spec/TDD/strategy/kickoff/push/status) remain in place at `docs/pipeline/l4-shared-visibility/artifacts/`. |
| L5-L6 planned layers | `layers/layer-5-expanded-commercial-intelligence.md`, `layers/layer-6-request-pipeline-automation.md` | L5 validation anchored at `L5-FR-1.1.1`; L6 remains blocked by `prompts/stage-15-bid-workflow-handoff-DEFERRED.md`. |

## By Topic

### Data & Schema

| Topic | File | Section |
|-------|------|---------|
| Full SQLite DDL (49 tables, all columns from creation) | `layers/layer-0-gold-set-evaluation.md` | §2 |
| Evaluation tables (gold_set_cases, evaluation_runs, evaluation_case_results, evaluation_metric_summaries) | `layers/layer-0-gold-set-evaluation.md` | §2 (§2.5.12) |
| Cross-authority FK discipline (rule 15) | `layers/layer-0-gold-set-evaluation.md` | §2 (§2.2) |
| Rounding policy and BPS constraint families (rules 17-18) | `layers/layer-0-gold-set-evaluation.md` | §2 (§2.2) |
| Original-currency minor units (rule 16) | `layers/layer-0-gold-set-evaluation.md` | §2 (§2.2) |
| Table authority classification (push-owned / local-only / remote-owned) | `layers/layer-0-gold-set-evaluation.md` | §2 |
| Sync tracking columns (`owner_id`, `row_version`, `last_local_seq`, `stage_run_id`) | `layers/layer-0-gold-set-evaluation.md` | §2 |
| All-columns-from-creation rule (no later-layer column adds) | `layer-strategy.md` | §9 Schema Evolution Rules |
| Constitutional schema ownership vs activation matrix (`Created` vs `First Active Use`) | `layer-strategy.md` | §9 |
| Schema stability matrix (all tables x all layers) | `layer-strategy.md` | §9 |
| TypeScript interfaces (`packages/contracts-schema`) | `layers/layer-0-gold-set-evaluation.md` | §5 |
| Supabase projection tables (20 push-owned mirrors) | `layers/layer-4-shared-visibility.md` | §2 |
| Supabase summary tables (6 redacted projections incl. operator_decision_summaries) | `layers/layer-4-shared-visibility.md` | §2 |
| CAS command ledger (`operator_decision_commands`) | `layers/layer-4-shared-visibility.md` | §1 (§1.5) |
| Remote-owned tables (`operator_decision_heads`, `shared_*`) | `layers/layer-4-shared-visibility.md` | §2 |
| RLS policy matrix (4 roles x all tables, web read-only as security boundary) | `layers/layer-4-shared-visibility.md` | §2 |
| Supabase migrations (8 files) | `layers/layer-4-shared-visibility.md` | §2 |
| Executor identity (`executors`) | `layers/layer-1-thin-local-runner.md` | §1 |
| Source cursors (ingestion watermarks) | `layers/layer-1-thin-local-runner.md` | §1 |
| Sync infrastructure (`local_outbox`, `sync_batches`, `pull_cursors`; SQLite migration `migrations/0012_l4_sync_state.sql`) | `layers/layer-4-shared-visibility.md` | §1 |
| `pending_operator_intents` (local decision queue with reviewed-basis fields) | `layers/layer-0-gold-set-evaluation.md` | §2 (§2.5.8 DDL), `layers/layer-2-single-operator-desktop.md` §3 (usage) |
| Basis-total pricing on `supplier_quotes` (offer_total fields for sub-penny accuracy) | `layers/layer-0-gold-set-evaluation.md` | §2 (§2.5.2) |
| `product_commodity_codes` expanded entity (digits, source, score, rank for L5 multi-source recall) | `layers/layer-5-expanded-commercial-intelligence.md` | `L5-FR-1.3.2`, `L5-FR-2.4.2` |
| "All columns from creation" scope clarification (what it does and doesn't prevent) | `layers/layer-0-gold-set-evaluation.md` | §2 (§2.2.1) |
| L3 schema-activation reconciliation note (repo migration drift vs constitutional shape) | `layers/layer-3-operational-hardening.md` | §4 (§4.1) |
| Branded scalar types recommendation (`Pence`, `MinorUnits`, `BpsBounded`, `BpsSigned`) | `layers/layer-0-gold-set-evaluation.md` | §5 (§5.3.5) |
| Harness DB isolation (separate SQLite from operational DB) | `layers/layer-0-gold-set-evaluation.md` | §5 (§5.3.4) |
| New L3 tables (`validation_verdicts`, `source_runtime_state`, `listing_evidence`, `process_relations`, `process_aliases`) | `layers/layer-3-operational-hardening.md` | §4 |
| Document extraction tables (`document_extractions`, `document_semantic_extractions`) | `layers/layer-1-thin-local-runner.md` | §1 (§1.5) |
| `extraction_state` on `tender_document_sets` (forward-compatibility) | `layers/layer-0-gold-set-evaluation.md` | §2 |
| `semantic_extraction_state` on `document_extractions` (distinct from `tender_document_sets.extraction_state`) | `layers/layer-1-thin-local-runner.md` | §1 (§1.5) |
| Document extraction entity relationship chain | `layers/layer-1-thin-local-runner.md` | §1 (§1.5) |

### Pipeline

| Topic | File | Section |
|-------|------|---------|
| 15-stage pipeline processing logic (qualification split into prequalify_notice + qualify_lot) | `layers/layer-0-gold-set-evaluation.md` | §3 |
| Qualification two-phase model (notice-grain → lot-grain) | `layers/layer-0-gold-set-evaluation.md` | §3 (§3.4) |
| Canonical scope_id format per scope_kind | `layer-strategy.md` | §10 Stage Scope Matrix |
| Commit semantics: decision surface vs audit history | `layer-strategy.md` | §10 Stage Scope Matrix |
| Value-weighted coverage metrics | `layers/layer-0-gold-set-evaluation.md` | §3 (§3.13 Coverage Rollup) |
| Stage scoping model (`stage_run_id`, commit semantics, scope grains) | `layer-strategy.md` | §10 Stage Scope Matrix |
| Stage boundary contracts | `layer-strategy.md` | §10 Interface Contracts |
| Coverage metric formulas (authoritative definitions, value-weighted) | `layers/layer-0-gold-set-evaluation.md` | §3 (§3.13 Coverage Rollup) |
| Coverage metric presentation (operator-facing thresholds) | `layers/layer-2-single-operator-desktop.md` | §3 |
| Margin logic (revenue normalization, gross/net gate formulas) | `layers/layer-0-gold-set-evaluation.md` | §3 (§3.10-§3.12) |
| matchArchetype classification (6 domain archetypes + `unknown` fallback) | `layers/layer-0-gold-set-evaluation.md` | §3 (§3.7) |
| Dual confidence model & lexicographic gating | `layers/layer-0-gold-set-evaluation.md` | §3 (§3.9.7) |
| Pipeline cost management | `layers/layer-0-gold-set-evaluation.md` | §3 (§3.18) |
| Bilingual lexicon & cross-language matching | `layers/layer-0-gold-set-evaluation.md` | §3 (§3.17) |
| PDF structural extraction (Docling engine, processing strategies) | `layers/layer-1-thin-local-runner.md` | §1 (§1.5) |
| PDF semantic mapping (Claude overlay skills) | `layers/layer-1-thin-local-runner.md` | §1 (§1.5) |
| Extraction orchestration (job cascade: structural → semantic → lot/product refresh) | `layers/layer-1-thin-local-runner.md` | §1 (§1.5) |
| Extraction policy (document role classification, overlay routing) | `layers/layer-1-thin-local-runner.md` | §1 (§1.5) |
| Deterministic document chunking (synopsis, section-aware splits) | `layers/layer-1-thin-local-runner.md` | §1 (§1.5) |

### Skills & Agent Orchestration

| Topic | File | Section |
|-------|------|---------|
| Skill directory standard (SKILL.md + schemas) | `layers/layer-0-gold-set-evaluation.md` | §4 |
| 8-question skill authoring guide | `layers/layer-0-gold-set-evaluation.md` | §4 |
| `skills/pdf-extraction/` family path in the monorepo layout | `layers/layer-0-gold-set-evaluation.md` | §1.7 |
| Agent Adapter trait (two-lane: OutputFrame + ControlEvent) | `layers/layer-1-thin-local-runner.md` | §2 (§2.3) |
| PDF extraction skill family (base + 3 domain overlays) | `layers/layer-1-thin-local-runner.md` | §1 (§1.5) |
| Rust crate structure (20 daemon files) | `layers/layer-1-thin-local-runner.md` | §2 (§2.1) |
| Rust crate dependencies | `layers/layer-1-thin-local-runner.md` | §2 (§2.2) |
| PTY + contract mode execution | `layers/layer-1-thin-local-runner.md` | §1 (§1.7) |
| Process-group supervision & session semantics (setsid, ctty, CLOEXEC, cleanup verdicts) | `layers/layer-1-thin-local-runner.md` | §1 (§1.8) |
| Daemon discovery & IPC protocol (descriptor, lock, handshake) | `layers/layer-1-thin-local-runner.md` | §2 (§2.8) |
| Commit fence model (`commit_seq`, `visibleAfterSeq`) | `layers/layer-1-thin-local-runner.md` | §2 (§2.6) |
| Session Reaper (L1-critical, SIGCHLD/waitpid) | `layers/layer-1-thin-local-runner.md` | §2 (§2.1), §1 (§1.8) |
| Session lifecycle (7 phases) | `layers/layer-1-thin-local-runner.md` | §2 (§2.4) |
| Lifecycle events → Channel\<T\> mapping (all 7 phases, adapter vs daemon-managed) | `layers/layer-1-thin-local-runner.md` | §2 (§2.8) |
| L1 IPC scope (socket active, CLI-only triggers, handshake-only commands) | `layers/layer-1-thin-local-runner.md` | §2 (§2.8) |
| Run manifest system (authoritative definition) | `layers/layer-1-thin-local-runner.md` | §4 (§4.2) |
| Error taxonomy & error manifest system | `layers/layer-1-thin-local-runner.md` | §4 (§4.3) |
| Skill linting CI gate | `layers/layer-1-thin-local-runner.md` | §4 (§4.4) |
| Ingestion flow (end-to-end live loop) | `layers/layer-1-thin-local-runner.md` | §1 (§1.6) |
| Cron scheduler (explicit skill binding) | `layers/layer-3-operational-hardening.md` | §1 (§1.5) |
| Resource arbiter (typed quotas) | `layers/layer-3-operational-hardening.md` | §1 (§1.4) |
| Resource admission/release semantics + validator resource consumption | `layers/layer-3-operational-hardening.md` | §1 (§1.4) |
| PTY hardening (fd hygiene, EIO, geometry) | `layers/layer-3-operational-hardening.md` | §1 (§1.6) |
| Skill promotion flow (draft -> stable) | `layers/layer-3-operational-hardening.md` | §3 |
| Source-health canaries, auth-refresh skills | `layers/layer-3-operational-hardening.md` | §1 (§1.9) |
| Cluster identity (`canonical_process_id`) + late merge | `layers/layer-3-operational-hardening.md` | §3 (§3.6) |
| Four-layer observability model + retention/correlation | `layers/layer-3-operational-hardening.md` | §4 (§4.4) |
| Artifact store (chunked storage layout, stream.index.jsonl, run manifest format) | `layers/layer-1-thin-local-runner.md` | §2 (§2.5) |
| Single-database model (`state.db`, commit_clock) | `layers/layer-1-thin-local-runner.md` | §2 (§2.7) |
| Job Runner (stage_run_id scoping) | `layers/layer-1-thin-local-runner.md` | §2 (§2.6) |

### Validation (Worker-Validator)

| Topic | File | Section |
|-------|------|---------|
| Validation fields on `workflow_jobs` (nullable from L0) | `layers/layer-0-gold-set-evaluation.md` | §2 |
| Validation policy engine (always/sample/trigger/skip) | `layers/layer-3-operational-hardening.md` | §2 (§2.3) |
| Validation executor (kickoff prompt composition) | `layers/layer-3-operational-hardening.md` | §2 (§2.4) |
| Dry-run mode (isolation guarantees) | `layers/layer-3-operational-hardening.md` | §2 (§2.6) |
| Fail-open semantics (constitutional) + trust-state lattice | `layers/layer-3-operational-hardening.md` | §2 (§2.7) |
| Validator health states (healthy/degraded/blind) | `layers/layer-3-operational-hardening.md` | §2 (§2.7) |
| Retroactive validation backlog | `layers/layer-3-operational-hardening.md` | §2 (§2.7) |
| Synthetic validator canaries | `layers/layer-3-operational-hardening.md` | §2 (§2.7) |
| Unknown-verdict / invalid-verdict fail-open handling | `layers/layer-3-operational-hardening.md` | §2 (§2.4), §2 (§2.7) |
| Transactional validator verdict persistence | `layers/layer-3-operational-hardening.md` | §2 (§2.4) |
| Calibration timeline formula (per-slice) | `layers/layer-3-operational-hardening.md` | §2 (§2.9) |
| Behavior version vs raw version | `layers/layer-3-operational-hardening.md` | §3 (§3.2) |
| Process-level `validation_state` mapping from finer-grained trust states | `layers/layer-3-operational-hardening.md` | §2 (§2.7) |
| Base validator skill | `layers/layer-3-operational-hardening.md` | §2 (§2.8) |
| Outcome classes (6 standard) | `layers/layer-3-operational-hardening.md` | §2 (§2.5) |
| Initial calibration (L3) | `layers/layer-3-operational-hardening.md` | §2 (§2.9) |
| Validation enforcement mode (L4 transition) | `layers/layer-4-shared-visibility.md` | §3 (§3.4) |
| Domain-specific validator overlays (4) | `layers/layer-4-shared-visibility.md` | §3 (§3.5) |
| Formal calibration harness (3 corpora, mutation bank) | `layers/layer-4-shared-visibility.md` | §3 (§3.6) |

### Customs & Landed Cost

| Topic | File | Section |
|-------|------|---------|
| Screening-grade landed cost (HMRC Tariff + flat freight + single FX) | `layers/layer-0-gold-set-evaluation.md` | §3 (§3.12) |
| Conservative defaults (constitutional -- MFN, residual, standard VAT) | `layers/layer-5-expanded-commercial-intelligence.md` | `L5-FR-1.12.1`, `L5-FR-1.12.2` |
| Multi-source HS classification recall (5 sources) | `layers/layer-5-expanded-commercial-intelligence.md` | `L5-FR-1.4.1`-`L5-FR-1.4.5` |
| Multi-family expansion to UK 10-digit | `layers/layer-5-expanded-commercial-intelligence.md` | `L5-FR-1.5.1`, `L5-FR-1.5.2` |
| Exporter/manufacturer resolution | `layers/layer-5-expanded-commercial-intelligence.md` | `L5-FR-1.6.1` |
| Measure interaction engine (AD/CVD/safeguard/quota) | `layers/layer-5-expanded-commercial-intelligence.md` | `L5-FR-1.7.1` |
| Compound/specific duty expressions | `layers/layer-5-expanded-commercial-intelligence.md` | `L5-FR-1.8.1` |
| VAT rate determination + recovery | `layers/layer-5-expanded-commercial-intelligence.md` | `L5-FR-1.9.1`, `L5-FR-1.9.2` |
| Classification freshness system | `layers/layer-5-expanded-commercial-intelligence.md` | `L5-FR-1.11.1`, `L5-FR-1.11.2` |
| Customs branch divergence (hard blocker) | `layers/layer-5-expanded-commercial-intelligence.md` | `L5-FR-1.13.1`, `L5-FR-1.13.2` |
| CBAM forward placeholder | `layers/layer-5-expanded-commercial-intelligence.md` | `L5-FR-1.14.1`, `L5-FR-1.14.2` |
| Dual FX engine (HMRC customs + Frankfurter commercial) | `layers/layer-5-expanded-commercial-intelligence.md` | `L5-FR-2.1.1`-`L5-FR-2.1.3` |
| Full freight engine (Freightos + SeaRates CDP skills) | `layers/layer-5-expanded-commercial-intelligence.md` | `L5-FR-2.2.1`-`L5-FR-2.2.4` |
| Customs error types (10 domain-specific requirements; 9 new enum variants + existing `FPO_ACCESS_UNAVAILABLE`) | `layers/layer-5-expanded-commercial-intelligence.md` | `L5-FR-3.5.1`-`L5-FR-3.5.10` |
| Evidence-tiered accuracy (gold/silver/bronze) | `layers/layer-5-expanded-commercial-intelligence.md` | `L5-FR-3.4.1`, `L5-FR-3.4.2` |
| Multi-family expansion tractability (local cache, shortlist, scenarioing) | `layers/layer-5-expanded-commercial-intelligence.md` | `L5-FR-1.5.3`, `L5-FR-1.5.4` |
| Classification freshness (4 vectors: structure, measures, remedies, FX) | `layers/layer-5-expanded-commercial-intelligence.md` | `L5-FR-1.11.1` |
| Unlockable margin and evidence leverage | `layers/layer-5-expanded-commercial-intelligence.md` | `L5-FR-1.12.2` |
| Additional-code satisfiability (DE 6/17) | `layers/layer-5-expanded-commercial-intelligence.md` | `L5-FR-1.7.2` |
| Bundle/set classification (GIR 3 escape hatch) | `layers/layer-5-expanded-commercial-intelligence.md` | `L5-FR-1.8.3` |
| CBAM scope (aluminium, cement, fertiliser, hydrogen, iron & steel) | `layers/layer-5-expanded-commercial-intelligence.md` | `L5-FR-1.14.1` |
| Classification lattice stage contract (candidate sub-grain) | `layer-strategy.md` | §10 Stage Scope Matrix |

### Frontend

| Topic | File | Section |
|-------|------|---------|
| Desktop architecture (QueryService + DaemonBridge + Channel\<T\>) | `layers/layer-2-single-operator-desktop.md` | §1 (§1.3) |
| Dev-mode browser transport (Vite middleware + transport shim for Playwright testing) | `layers/layer-2-single-operator-desktop.md` | §1 (§1.8) |
| QueryEnvelope\<T\> interface | `layers/layer-2-single-operator-desktop.md` | §1 (§1.4) |
| MutationAck interface | `layers/layer-2-single-operator-desktop.md` | §1 (§1.5) |
| Channel\<T\> streaming (SessionFrame types) | `layers/layer-2-single-operator-desktop.md` | §1 (§1.6) |
| Bootstrap state machine (5-state structured health from ControlPlaneState) | `layers/layer-2-single-operator-desktop.md` | §1 (§1.7) |
| ControlPlaneState (storage/daemon/control/load dimensions) | `layers/layer-2-single-operator-desktop.md` | §1 (§1.4) |
| Commit fence (desktop read-your-writes via snapshotSeq/visibleAfterSeq) | `layers/layer-2-single-operator-desktop.md` | §1 (§1.4) |
| Desktop-daemon handshake (hello/welcome, instanceId, PTY resume rules) | `layers/layer-2-single-operator-desktop.md` | §1 (§1.5) |
| Desktop session controls (stop/input/resize/retry/restart) | `layers/layer-2-single-operator-desktop.md` | §1 (§1.5) |
| FreshnessBadge (surface-neutral FreshnessBadgeVM) | `layers/layer-2-single-operator-desktop.md` | §2 (§2.1) |
| ErrorSummaryList (typed ErrorSummaryItem + severity derivation rule) | `layers/layer-2-single-operator-desktop.md` | §2 (§2.1) |
| Error severity derivation (error_type category → critical/warning/info) | `layers/layer-2-single-operator-desktop.md` | §2 (§2.1) |
| Shared UI components (`packages/ui` — 5 components with typed props) | `layers/layer-2-single-operator-desktop.md` | §2 |
| Manifest rendering (`packages/manifests`) | `layers/layer-2-single-operator-desktop.md` | §2 (§2.2) |
| Decision capture (`pending_operator_intents`) | `layers/layer-2-single-operator-desktop.md` | §3 (§3.2) |
| Decision staleness detection | `layers/layer-2-single-operator-desktop.md` | §3 (§3.3) |
| Web viewing portal (Next.js + Supabase) | `layers/layer-4-shared-visibility.md` | §3 (§3.1) |
| Supabase Broadcast invalidation | `layers/layer-4-shared-visibility.md` | §3 (§3.2) |
| Degraded-mode UX (L4 adds sync dimension to ControlPlaneState) | `layers/layer-4-shared-visibility.md` | §3 (§3.3) |

### Sync Engine

| Topic | File | Section |
|-------|------|---------|
| Push protocol (transactional outbox + commit_group_id + writer_epoch) | `layers/layer-4-shared-visibility.md` | §1 |
| Pull protocol (hi-water fenced cursor + CAS with command ledger) | `layers/layer-4-shared-visibility.md` | §1 |
| Active-writer lease (dedicated `active_writer_lease`, fenced with writer_epoch) | `layers/layer-4-shared-visibility.md` | §1 (§1.6), §2 (§2.6) |
| Provisional promotion buffer (`provisional_worker_outputs` for enforcement slices) | `layers/layer-4-shared-visibility.md` | §1 (§1.3) |
| Per-slice enforcement readiness (`enforcement_readiness`) | `layers/layer-4-shared-visibility.md` | §3 (§3.4) |
| Equal-version retry semantics | `layers/layer-4-shared-visibility.md` | §1 (§1.3) |
| `schema_epoch` guard + epoch-repair reproject | `layers/layer-4-shared-visibility.md` | §1 (§1.7), §5 (§5.2) |
| Batch boundaries (never cross `stage_run_id` or `commit_group_id`) | `layer-strategy.md` | §6 Invariants Enforced |
| Remote config staleness classes (safety-critical TTLs) | `layers/layer-4-shared-visibility.md` | §1 (§1.8) |
| Tombstone retention policy | `layers/layer-4-shared-visibility.md` | §3 (§3.6) |
| Redaction boundary testing | `layers/layer-4-shared-visibility.md` | §2 (§2.5) |
| `viewer_sync_status` table DDL | `layers/layer-4-shared-visibility.md` | §3 (§3.7) |
| Cross-authority consistency | `layers/layer-4-shared-visibility.md` | §2 (§2.7) |

### Bid Workflow

| Topic | File | Section |
|-------|------|---------|
| Bid workflow state machine (6 states incl. draft_saved, portal_state_unknown) | `layers/layer-6-request-pipeline-automation.md` | §1 (§1.3) |
| Bid scope payload (explicit lot/candidate + sideEffectCap + decisionBasisHash) | `layers/layer-6-request-pipeline-automation.md` | §1 (§1.4) |
| Side-effect classes (READ_ONLY → FUNDS_COMMITTED) | `layers/layer-6-request-pipeline-automation.md` | §1 (§1.4, §1.7) |
| Bid portal skill (Proactis, 11-step playbook with per-step side-effect class) | `layers/layer-6-request-pipeline-automation.md` | §1 (§1.7) |
| `packages/bidding` (workflow registry, company profile, BidDocument, SubmissionResult) | `layers/layer-6-request-pipeline-automation.md` | §1 (§1.6) |
| Audit trail (constitutional) | `layers/layer-6-request-pipeline-automation.md` | §1 (§1.8) |
| L5→L6 readiness gate (blocker checks, quote-role quality) | `layers/layer-6-request-pipeline-automation.md`, `layers/layer-5-expanded-commercial-intelligence.md` | §1 (§1.3); `L5-FR-3.8.1` |
| Portal submission fail-closed on validation | `layers/layer-6-request-pipeline-automation.md` | §1 (§1.3) |
| L5→L6 readiness gate (cross-layer contract) | `layer-strategy.md`, `layers/layer-5-expanded-commercial-intelligence.md` | §10 Interface Contracts; `L5-FR-3.8.1` |

### Testing

| Topic | File | Section |
|-------|------|---------|
| Evaluation harness (gold-set benchmarking, gating/challenge corpus split) | `layers/layer-0-gold-set-evaluation.md` | §5 (§5.3) |
| Evaluation experiments (1–3b: extraction, matching, margin, hard-negative correctness) | `layers/layer-0-gold-set-evaluation.md` | §5 (§5.3.3) |
| Evaluation tables (DDL) | `layers/layer-0-gold-set-evaluation.md` | §2 (§2.5.12) |
| Definition of Done (L0, 10+ criteria incl. experiments) | `layers/layer-0-gold-set-evaluation.md` | §6 |
| L0 test suites (4: qualification, quoteability, tariff, margin) | `layers/layer-0-gold-set-evaluation.md` | §6 (§6.2) |
| Agent E2E tests | `layers/layer-1-thin-local-runner.md` | §5 |
| Playwright browser E2E tests (primary UI test surface on localhost) | `layers/layer-2-single-operator-desktop.md` | §4 (§4.1, §4.3) |
| Tauri integration tests (desktop-only behaviors) | `layers/layer-2-single-operator-desktop.md` | §4 (§4.3) |
| Fault injection + replay tests | `layers/layer-3-operational-hardening.md` | §5 |
| Validator calibration tests | `layers/layer-3-operational-hardening.md` | §2 (§2.9) |
| Validator canary health + retro-validation tests | `layers/layer-3-operational-hardening.md` | §5 (§5.2) |
| Sync round-trip tests (writer fencing, command ledger, provisional promotion) | `layers/layer-4-shared-visibility.md` | §4 |
| Redaction boundary tests (summary projection leakage) | `layers/layer-4-shared-visibility.md` | §4 (§4.2) |
| L5 customs test files (5: classification, measure interaction, FX, freight, FPO/search-suggestion) | `layers/layer-5-expanded-commercial-intelligence.md` | `L5-FR-3.2.1` |
| L6 bid workflow test files (3: states, side-effects, readiness gate) | `layers/layer-6-request-pipeline-automation.md` | §2 (§2.2) |
| No mock-only release gates (real sessions, real portals/APIs, or captured evidence bundles) | `layer-strategy.md` | §11 |
| Testing evolution (all layers, cumulative 4→60) | `layer-strategy.md` | §11 |

### Open Questions

| Topic | File | Origin |
|-------|------|--------|
| Scotland in MVP scope? | `layers/layer-0-gold-set-evaluation.md` | §7.1 User Input #1 |
| VAT-recoverable importer assumption? | `layers/layer-0-gold-set-evaluation.md` | §7.1 User Input #4 |
| Margin thresholds as hard filters? | `layers/layer-0-gold-set-evaluation.md` | §7.1 Assumption #3 |
| Auto-qualification coverage floor? | `layers/layer-2-single-operator-desktop.md` | §5.1 Further Research #5 |
| Customs cost spread materiality? | `layers/layer-5-expanded-commercial-intelligence.md` | `L5-FR-3.6.1` / Further Research #8 |
| Packaging sub-category reclassification? | `layers/layer-5-expanded-commercial-intelligence.md` | `L5-FR-3.6.1` / Further Research #9 |
| CBAM rate integration timeline? | `layers/layer-5-expanded-commercial-intelligence.md` | `L5-FR-3.6.1` / Further Research #10 |
| Late merge cluster rollup reconciliation (partially resolved) | `layers/layer-3-operational-hardening.md` | §6 (§6.1) |
| Epoch mismatch recovery and reproject | `layers/layer-4-shared-visibility.md` | §5 (§5.1) |

## By ADR

| ADR | Decision | Layer(s) |
|-----|----------|----------|
| ADR-001 | Skill-based agent scraping | L0 (first 5 skills), L1 (live FTS), L2 (Contracts Finder), L5 (`L5-FR-3.10.2`) |
| ADR-002 | Authority matrix database | L0 (classification), L4 (sync enforcement + RLS) |
| ADR-003 | Tauri + separate Rust daemon | L1 (daemon), L2 (desktop shell) |
| ADR-004 | PTY-based execution (hybrid) | L1 (basic), L3 (hardened) |
| ADR-005 | Skills as passive assets + explicit binding | L0 (standard), L1 (runtime), L3 (promotion + cron binding) |
| ADR-006 | Internal landed-cost engine | L0 (screening-grade), L5 (`L5-FR-3.10.1`) |
| ADR-007 | Request pipeline automation (not real-money bids) | L6 |
| ADR-008 | Desktop = control panel, web = read-only | L2 (desktop), L4 (web portal) |
