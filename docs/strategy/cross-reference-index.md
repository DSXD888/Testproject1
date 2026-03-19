# Cross-Reference Index — Layer Strategy

> Quick lookup: "if you're looking for X, it's in file Y, section Z."
>
> **Maintained doc set:** `layer-strategy.md` + 7 per-layer docs in `layers/` + this index + `README.md`.

## By Topic

### Data & Schema

| Topic | File | Section |
|-------|------|---------|
| Full SQLite DDL (20+ tables, all columns from creation) | `layers/layer-0-gold-set-evaluation.md` | §2 |
| Table authority classification (push-owned / local-only / remote-owned) | `layers/layer-0-gold-set-evaluation.md` | §2 |
| Sync tracking columns (`owner_id`, `row_version`, `last_local_seq`, `stage_run_id`) | `layers/layer-0-gold-set-evaluation.md` | §2 |
| All-columns-from-creation rule (no later-layer column adds) | `layer-strategy.md` | §9 Schema Evolution Rules |
| Schema stability matrix (all tables x all layers) | `layer-strategy.md` | §9 |
| TypeScript interfaces (`packages/contracts-schema`) | `layers/layer-0-gold-set-evaluation.md` | §5 |
| Supabase projection tables (18 push-owned mirrors) | `layers/layer-4-shared-visibility.md` | §2 |
| Supabase summary tables (5 redacted projections) | `layers/layer-4-shared-visibility.md` | §2 |
| Remote-owned tables (`operator_decision_heads`, `shared_*`) | `layers/layer-4-shared-visibility.md` | §2 |
| RLS policy matrix (4 roles x all tables) | `layers/layer-4-shared-visibility.md` | §2 |
| Supabase migrations (6 files) | `layers/layer-4-shared-visibility.md` | §2 |
| Executor identity (`executors`) | `layers/layer-1-thin-local-runner.md` | §1 |
| Source cursors (ingestion watermarks) | `layers/layer-1-thin-local-runner.md` | §1 |
| Sync infrastructure (`local_outbox`, `sync_batches`, `pull_cursors`) | `layers/layer-4-shared-visibility.md` | §1 |
| `pending_operator_intents` (local decision queue) | `layers/layer-2-single-operator-desktop.md` | §3 |
| New L3 tables (`validation_verdicts`, `source_runtime_state`, `listing_evidence`, `process_relations`, `process_aliases`) | `layers/layer-3-operational-hardening.md` | §4 |

### Pipeline

| Topic | File | Section |
|-------|------|---------|
| 15-stage pipeline processing logic | `layers/layer-0-gold-set-evaluation.md` | §3 |
| Stage scoping model (`stage_run_id`, commit semantics, scope grains) | `layer-strategy.md` | §10 Stage Scope Matrix |
| Stage boundary contracts | `layer-strategy.md` | §10 Interface Contracts |
| Coverage metric formulas (authoritative definitions) | `layers/layer-0-gold-set-evaluation.md` | §3 (§3.14 Coverage Rollup) |
| Coverage metric presentation (operator-facing thresholds) | `layers/layer-2-single-operator-desktop.md` | §3 |
| Margin logic (revenue normalization, gross/net gate formulas) | `layers/layer-0-gold-set-evaluation.md` | §3 (§3.20 Margin Logic) |
| matchArchetype classification (6 archetypes) | `layers/layer-0-gold-set-evaluation.md` | §3 (§3.17) |
| Dual confidence model & lexicographic gating | `layers/layer-0-gold-set-evaluation.md` | §3 (§3.19) |
| Pipeline cost management | `layers/layer-0-gold-set-evaluation.md` | §3 (§3.21) |
| Bilingual lexicon & cross-language matching | `layers/layer-0-gold-set-evaluation.md` | §3 (§3.18) |

### Skills & Agent Orchestration

| Topic | File | Section |
|-------|------|---------|
| Skill directory standard (SKILL.md + schemas) | `layers/layer-0-gold-set-evaluation.md` | §4 |
| 8-question skill authoring guide | `layers/layer-0-gold-set-evaluation.md` | §4 |
| Agent Adapter trait (full Rust interface) | `layers/layer-1-thin-local-runner.md` | §2 (§2.3) |
| Rust crate structure (14 daemon files) | `layers/layer-1-thin-local-runner.md` | §2 (§2.1) |
| Rust crate dependencies | `layers/layer-1-thin-local-runner.md` | §2 (§2.2) |
| PTY + contract mode execution | `layers/layer-1-thin-local-runner.md` | §1 (§1.7) |
| Process-group supervision (cleanup verdicts) | `layers/layer-1-thin-local-runner.md` | §1 (§1.8) |
| Session lifecycle (7 phases) | `layers/layer-1-thin-local-runner.md` | §2 (§2.4) |
| Run manifest system (authoritative definition) | `layers/layer-1-thin-local-runner.md` | §4 (§4.2) |
| Error taxonomy & error manifest system | `layers/layer-1-thin-local-runner.md` | §4 (§4.3) |
| Skill linting CI gate | `layers/layer-1-thin-local-runner.md` | §4 (§4.4) |
| Ingestion flow (end-to-end live loop) | `layers/layer-1-thin-local-runner.md` | §1 (§1.6) |
| Cron scheduler (explicit skill binding) | `layers/layer-3-operational-hardening.md` | §1 (§1.5) |
| Resource arbiter (typed quotas) | `layers/layer-3-operational-hardening.md` | §1 (§1.4) |
| PTY hardening (fd hygiene, EIO, geometry) | `layers/layer-3-operational-hardening.md` | §1 (§1.6) |
| Skill promotion flow (draft -> stable) | `layers/layer-3-operational-hardening.md` | §3 |
| Source-health canaries, auth-refresh skills | `layers/layer-3-operational-hardening.md` | §1 (§1.9) |
| Artifact store (storage layout, run manifest format) | `layers/layer-1-thin-local-runner.md` | §2 (§2.5) |
| Job Runner (stage_run_id scoping) | `layers/layer-1-thin-local-runner.md` | §2 (§2.6) |

### Validation (Worker-Validator)

| Topic | File | Section |
|-------|------|---------|
| Validation fields on `workflow_jobs` (nullable from L0) | `layers/layer-0-gold-set-evaluation.md` | §2 |
| Validation policy engine (always/sample/trigger/skip) | `layers/layer-3-operational-hardening.md` | §2 (§2.3) |
| Validation executor (kickoff prompt composition) | `layers/layer-3-operational-hardening.md` | §2 (§2.4) |
| Dry-run mode (isolation guarantees) | `layers/layer-3-operational-hardening.md` | §2 (§2.6) |
| Fail-open semantics (constitutional) | `layers/layer-3-operational-hardening.md` | §2 (§2.7) |
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
| Conservative defaults (constitutional -- MFN, residual, standard VAT) | `layers/layer-5-expanded-commercial-intelligence.md` | §1 (§1.12) |
| Multi-source HS classification recall (5 sources) | `layers/layer-5-expanded-commercial-intelligence.md` | §1 (§1.4) |
| Multi-family expansion to UK 10-digit | `layers/layer-5-expanded-commercial-intelligence.md` | §1 (§1.5) |
| Exporter/manufacturer resolution | `layers/layer-5-expanded-commercial-intelligence.md` | §1 (§1.6) |
| Measure interaction engine (AD/CVD/safeguard/quota) | `layers/layer-5-expanded-commercial-intelligence.md` | §1 (§1.7) |
| Compound/specific duty expressions | `layers/layer-5-expanded-commercial-intelligence.md` | §1 (§1.8) |
| VAT rate determination + recovery | `layers/layer-5-expanded-commercial-intelligence.md` | §1 (§1.9) |
| Classification freshness system | `layers/layer-5-expanded-commercial-intelligence.md` | §1 (§1.11) |
| Customs branch divergence (hard blocker) | `layers/layer-5-expanded-commercial-intelligence.md` | §1 (§1.13) |
| CBAM forward placeholder | `layers/layer-5-expanded-commercial-intelligence.md` | §1 (§1.14) |
| Dual FX engine (HMRC customs + Xe commercial) | `layers/layer-5-expanded-commercial-intelligence.md` | §2 (§2.1) |
| Full freight engine (Freightos + SeaRates) | `layers/layer-5-expanded-commercial-intelligence.md` | §2 (§2.2) |
| Customs error types (10 domain-specific) | `layers/layer-5-expanded-commercial-intelligence.md` | §3 (§3.4) |
| Evidence-tiered accuracy (gold/silver/bronze) | `layers/layer-5-expanded-commercial-intelligence.md` | §3 (§3.3) |

### Frontend

| Topic | File | Section |
|-------|------|---------|
| Desktop architecture (QueryService + DaemonBridge + Channel\<T\>) | `layers/layer-2-single-operator-desktop.md` | §1 |
| QueryEnvelope\<T\> interface | `layers/layer-2-single-operator-desktop.md` | §1 (§1.4) |
| MutationAck interface | `layers/layer-2-single-operator-desktop.md` | §1 (§1.5) |
| Channel\<T\> streaming (SessionFrame types) | `layers/layer-2-single-operator-desktop.md` | §1 (§1.6) |
| Bootstrap state machine (healthy/browse-only/recovery) | `layers/layer-2-single-operator-desktop.md` | §1 (§1.7) |
| Shared UI components (`packages/ui` -- 5 components) | `layers/layer-2-single-operator-desktop.md` | §2 |
| Manifest rendering (`packages/manifests`) | `layers/layer-2-single-operator-desktop.md` | §2 (§2.2) |
| Decision capture (`pending_operator_intents`) | `layers/layer-2-single-operator-desktop.md` | §3 (§3.2) |
| Decision staleness detection | `layers/layer-2-single-operator-desktop.md` | §3 (§3.3) |
| Web viewing portal (Next.js + Supabase) | `layers/layer-4-shared-visibility.md` | §3 (§3.1) |
| Supabase Broadcast invalidation | `layers/layer-4-shared-visibility.md` | §3 (§3.2) |
| Degraded-mode UX (3 states) | `layers/layer-4-shared-visibility.md` | §3 (§3.3) |

### Sync Engine

| Topic | File | Section |
|-------|------|---------|
| Push protocol (transactional outbox) | `layers/layer-4-shared-visibility.md` | §1 |
| Pull protocol (cursor-based + CAS) | `layers/layer-4-shared-visibility.md` | §1 |
| Active-writer lease | `layer-strategy.md` | §6 Invariants Enforced |
| `schema_epoch` guard | `layer-strategy.md` | §6 Invariants Enforced |
| Batch boundaries (never cross `stage_run_id`) | `layer-strategy.md` | §6 Invariants Enforced |
| `viewer_sync_status` table DDL | `layers/layer-4-shared-visibility.md` | §3 (§3.7) |
| Cross-authority consistency | `layers/layer-4-shared-visibility.md` | §2 (§2.7) |

### Bid Workflow

| Topic | File | Section |
|-------|------|---------|
| Bid workflow state machine | `layers/layer-6-request-pipeline-automation.md` | §1 (§1.3) |
| Bid scope payload (explicit lot/candidate) | `layers/layer-6-request-pipeline-automation.md` | §1 (§1.4) |
| Bid portal skill (Proactis, 11-step playbook) | `layers/layer-6-request-pipeline-automation.md` | §1 (§1.7) |
| `packages/bidding` (workflow registry, company profile) | `layers/layer-6-request-pipeline-automation.md` | §1 (§1.6) |
| Audit trail (constitutional) | `layers/layer-6-request-pipeline-automation.md` | §1 (§1.8) |

### Testing

| Topic | File | Section |
|-------|------|---------|
| Evaluation harness (gold-set benchmarking) | `layers/layer-0-gold-set-evaluation.md` | §5, §6 |
| L0 test suites (4: qualification, quoteability, tariff, margin) | `layers/layer-0-gold-set-evaluation.md` | §6 (§6.2) |
| Agent E2E tests | `layers/layer-1-thin-local-runner.md` | §5 |
| Desktop E2E tests | `layers/layer-2-single-operator-desktop.md` | §4 (§4.2) |
| Fault injection + replay tests | `layers/layer-3-operational-hardening.md` | §5 |
| Validator calibration tests | `layers/layer-3-operational-hardening.md` | §2 (§2.9) |
| Sync round-trip tests | `layers/layer-4-shared-visibility.md` | §4 |
| Testing evolution (all layers) | `layer-strategy.md` | §11 |

### Open Questions

| Topic | File | Origin |
|-------|------|--------|
| Scotland in MVP scope? | `layers/layer-0-gold-set-evaluation.md` | §10 User Input #1 |
| VAT-recoverable importer assumption? | `layers/layer-0-gold-set-evaluation.md` | §10 User Input #4 |
| Margin thresholds as hard filters? | `layers/layer-0-gold-set-evaluation.md` | §10 Assumption #3 |
| Auto-qualification coverage floor? | `layers/layer-2-single-operator-desktop.md` | §10 Further Research #5 |
| Customs cost spread materiality? | `layers/layer-5-expanded-commercial-intelligence.md` | §10 Further Research #8 |
| Packaging sub-category reclassification? | `layers/layer-5-expanded-commercial-intelligence.md` | §10 Further Research #9 |
| CBAM rate integration timeline? | `layers/layer-5-expanded-commercial-intelligence.md` | §10 Further Research #10 |

## By ADR

| ADR | Decision | Layer(s) |
|-----|----------|----------|
| ADR-001 | Skill-based agent scraping | L0 (first 5 skills), L1 (live FTS), L2 (Contracts Finder), L5 (Alibaba/1688/Amazon/PCS) |
| ADR-002 | Authority matrix database | L0 (classification), L4 (sync enforcement + RLS) |
| ADR-003 | Tauri + separate Rust daemon | L1 (daemon), L2 (desktop shell) |
| ADR-004 | PTY-based execution (hybrid) | L1 (basic), L3 (hardened) |
| ADR-005 | Skills as passive assets + explicit binding | L0 (standard), L1 (runtime), L3 (promotion + cron binding) |
| ADR-006 | Internal landed-cost engine | L0 (screening-grade), L5 (decision-grade) |
| ADR-007 | Request pipeline automation (not real-money bids) | L6 |
| ADR-008 | Desktop = control panel, web = read-only | L2 (desktop), L4 (web portal) |
