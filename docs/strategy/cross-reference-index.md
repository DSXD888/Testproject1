# Cross-Reference Index — Layer Strategy

> Quick lookup: "if you're looking for X, it's in Layer N."

## By Topic

### Data & Schema

| Topic | Layer | Section |
|-------|-------|---------|
| Table authority classification (full matrix) | L0 | §2 Data Introduced — all pipeline tables created in L0 |
| Sync tracking columns (`owner_id`, `row_version`, etc.) | L0 | §2 Data Introduced (created on all tables), L4 §6 (outbox entries activated) |
| All-columns-from-creation rule | L0 | §2 Critical schema decisions + §9 Schema Evolution Rules |
| Push-owned projections to Supabase | L4 | §6 Data Introduced |
| Remote-owned tables (`operator_decision_heads`, `shared_*`) | L4 | §6 Data Introduced |
| Junction tables (`notice_cpv_codes`, etc.) | L4 | §6 Data Introduced |
| Executor identity (`executors`, `owner_id`, `executor_id`) | L1 | §3 Data Introduced |
| Source cursors (ingestion watermarks) | L1 | §3 Data Introduced |
| Sync infrastructure (`local_outbox`, `sync_batches`, `pull_cursors`) | L4 | §6 Data Introduced |
| Schema stability matrix (all tables × all layers) | Cross-layer | §9 |
| Schema evolution rules | Cross-layer | §9 |

### Pipeline

| Topic | Layer | Section |
|-------|-------|---------|
| 15-stage pipeline (first implementation) | L0 | §2 (screening-grade) |
| Stage scoping model (`stage_run_id`, commit semantics) | L1 | §3 Definition of Done |
| Stage boundary contracts | Cross-layer | §10 Interface Contracts |
| Coverage metrics (`pricedCoverageBps`, etc.) | L2 | §4 Data Introduced |
| Staleness and invalidation | L3 | §5 (staleness propagation) |

### Skills & Agent Orchestration

| Topic | Layer | Section |
|-------|-------|---------|
| Skill directory standard (SKILL.md + schemas) | L0 | §2 Forward-Compatibility |
| PTY + contract mode adapter | L1 | §3 Components Touched |
| Process-group supervision | L1 | §3 Invariants Enforced |
| Run manifest generation | L1 | §3 Invariants Enforced |
| Skill promotion flow (draft → stable) | L3 | §5 Invariants Enforced |
| Source-health canaries, auth-refresh | L3 | §5 Components Touched |
| Resource arbiter (typed quotas) | L3 | §5 Components Touched |

### Validation (Worker-Validator)

| Topic | Layer | Section |
|-------|-------|---------|
| Validation fields on `workflow_jobs` (nullable) | L0 | §2 Data Introduced |
| Dry-run validation mode | L3 | §5 |
| Base validator skill | L3 | §5 Components Touched |
| Validation enforcement mode | L4 | §6 |
| Domain-specific validator overlays | L4 | §6 Components Touched |

### Customs & Landed Cost

| Topic | Layer | Section |
|-------|-------|---------|
| Screening-grade landed cost | L0 | §2 (HMRC Tariff + flat freight + single FX) |
| Full customs engine (multi-source recall, 10-digit) | L5 | §7 |
| Full FX engine (dual-rate) | L5 | §7 Components Touched |
| Full freight engine (Freightos + SeaRates) | L5 | §7 Components Touched |
| Conservative defaults | L0 | §2 Invariants Enforced (permanent) |

### Frontend

| Topic | Layer | Section |
|-------|-------|---------|
| Desktop control panel (Tauri) | L2 | §4 |
| Shared UI components (`packages/ui`) | L2 | §4 Shared Library Additions |
| Web viewing portal (Next.js) | L4 | §6 Components Touched |
| Degraded-mode UX | L4 | §6 (three sync states) |

### Sync Engine

| Topic | Layer | Section |
|-------|-------|---------|
| Push protocol (transactional outbox) | L4 | §6 |
| Pull protocol (cursor-based, CAS) | L4 | §6 |
| Active-writer lease | L4 | §6 Invariants Enforced |
| `schema_epoch` guard | L4 | §6 Invariants Enforced |
| Recovery model (warm/cold) | L4 | §6 What's Stubbed |

### Testing

| Topic | Layer | Section |
|-------|-------|---------|
| Evaluation harness (gold-set benchmarking) | L0 | §2 |
| Agent E2E tests | L2 | §4 Test Scenarios |
| Fault injection tests | L3 | §5 Test Scenarios |
| Deterministic replay tests | L3 | §5 Test Scenarios |
| Sync round-trip tests | L4 | §6 Test Scenarios |
| Testing evolution (all layers) | Cross-layer | §11 |

## By Research Document Section

| Research §  | Topic | Layer(s) |
|-------------|-------|----------|
| §1 | Executive summary, MVE definition | L0 |
| §4.5 | Customs classification | L0 (screening), L5 (full) |
| §4.6 | FX | L0 (single rate), L5 (dual rate) |
| §4.7 | Authority matrix | L0 (classification), L4 (enforcement) |
| §4.9 | Agent orchestration | L1 |
| §4.10 | Skill system | L0 (standard), L1 (runtime), L3 (promotion) |
| §5.2 | Daemon components | L1–L3 |
| §5.3 | Stage scoping model | L1 |
| §5.4 | Data model | L0 (schema), L4 (sync columns active) |
| §5.5 | Contract state model | L2 (pipeline + overlays), L4 (decisions via sync) |
| §5.6 | Ingestion data flow | L2 |
| §5.7 | Intelligence pipeline | L0 (screening), L5 (decision-grade) |
| §5.8 | Shared React components | L2 |
| §5.10 | Desktop/web architecture | L2 (desktop), L4 (web) |
| §5.11 | Error types and skill directory | L1 (errors), L0 (directory) |
| §6.1 | Complexity analysis | Informs all layer scoping |
| §6.2 | Risk assessment | Distributed per layer |
| §6.3 | Security (RLS) | L4 |
| §8.2 | File reference map | §8.4 milestone assignment → layer mapping |
| §8.4 | Milestone assignment map | Cross-layer §12 |
| §11 | Next steps / milestones | Cross-layer §12 |
| ADR-001 | Skill-based agent scraping | L0 (first skills), L5 (additional sources) |
| ADR-002 | Authority matrix database | L0 (classification), L4 (sync enforcement) |
| ADR-003 | Tauri + Rust daemon | L1 (daemon), L2 (desktop) |
| ADR-004 | PTY-based execution | L1 (basic), L3 (hardened) |
| ADR-005 | Skills as assets, not engine | L0 (standard), L3 (binding + promotion) |
| ADR-006 | Internal landed-cost engine | L0 (screening), L5 (decision-grade) |
| ADR-007 | Request pipeline automation | L6 |
| ADR-008 | Desktop = control, web = view | L2 (desktop), L4 (web) |
