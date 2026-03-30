# L3 Operational Hardening - Implementation Strategy

## Source Documents

| Document | Path | Role |
|----------|------|------|
| Spec | `docs/strategy/layers/layer-3-operational-hardening.md` | SOT #1 - WHAT to build |
| TDD | `docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-tdd.md` | SOT #2 - HOW to verify |
| Bootstrap | `docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-bootstrap.md` | Harness/bootstrap reference |
| Cross-layer sequencing | `docs/strategy/layer-strategy.md` Sections 5, 9-11 | Layer ordering, authority rules, interface boundaries, testing evolution |
| Quality-bar reference | `docs/pipeline/l1-daemon-core/artifacts/l1-daemon-core-strategy.md` | Structure and gate-specificity benchmark |
| Secondary structure reference | `docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-strategy.md` | Mixed-stack phase-layout benchmark |

## Strategy Guardrails

- This document defines order, dependency edges, phase ownership, and gate criteria only. The Layer 3 spec and TDD remain the content authorities.
- Primary-home rule: every new runtime surface, package surface, skill family, and TDD file has exactly one home phase. Later phases may extend the same surfaces without moving their home.
- Layer 3 stays at `6` phases. That is the smallest count that keeps the schema/admission, scheduler/health, PTY evidence, validation, TS package, and capstone integration seams acyclic.
- `migrations/0005_l3_operational_hardening.sql` and `crates/daemon/src/db/sqlite.rs` must land before any Layer 3 write path. No later phase is allowed to "discover" table shape through opportunistic runtime `ALTER TABLE`.
- `crates/daemon/src/main.rs` is the bootstrap choke point. New `mod` declarations and startup wiring may only reference modules that already landed in the same or earlier phase.
- `crates/daemon/src/main.rs`, `crates/daemon/src/jobs/runner.rs`, and `crates/daemon/src/ipc/command_router.rs` are merge-hot choke points. Each owning phase treats them as one terminal integration bead after the phase-local module contracts are frozen; no phase scatters repeated edits across those files.
- `crates/daemon/src/jobs/runner.rs` remains the authoritative enqueue/order/idempotency boundary. `jobs/claim.rs`, cron roots, resource admission, and validator sub-jobs must all route through that boundary rather than inventing a parallel queue.
- `crates/daemon/src/agents/artifacts.rs` and `crates/daemon/src/agents/contracts.rs` are extend-only surfaces. Layer 3 adds fields and dispositions around them; it does not fork alternate manifest or contract formats.
- `sources/registry.rs` must precede source-health skill directories and canary classification. Canary classification/persistence must precede circuit-breaker transitions, and breaker truth must precede cron suppression.
- `agents/arbiter.rs` must precede cron catchup and validator fan-out.
- `validation/manifest_writer.rs` freezes the bounded manifest contract first. `validation/policy.rs`, `crates/daemon/src/agents/contracts.rs`, and `crates/daemon/src/db/checkpoints.rs` must land before `validation/executor.rs`; `skills/validation/base/` may run in parallel once the manifest appendix and rubric contract are frozen.
- Skill-directory work may run in parallel with Rust module work once interaction profiles and contract surfaces are frozen; filesystem beads should not block Rust compile gates except where inventory or end-to-end behavior closes the phase.
- The relation-resolution and error-renderer package lane is intentionally off the daemon critical path. It may start after Phase 1 and run in parallel with daemon phases, but the final capstone gate waits for both the daemon and package lanes to converge.
- T1/T2 remain the cumulative per-commit gate. T3/T4 and the Manual Gate are capstone reliability gates, not substitutes for earlier phase exits.
- Target bead envelope: `32-38` beads.

## Overview

| Phase | Name | Prerequisites | Gate Type | Target Beads |
|------|------|---------------|-----------|--------------|
| 1 | Schema, Claim, And Admission Foundation | Layer 0-L2 schema/runtime present | SQLite + Rust foundation | 4-5 |
| 2 | Scheduler, Registry, And Source Health Control | Phase 1 | scheduler/health control | 5-6 |
| 3 | PTY Reliability And Four-Layer Evidence | Phase 1 | runtime reliability | 5-6 |
| 4 | Manifest And Validation Dry-Run Core | Phase 2 + Phase 3 | dry-run validator core | 9-10 |
| 5 | Relations And Error-Surface Packages | Phase 1 | TypeScript/shared-package | 3-4 |
| 6 | Promotion, Status Surfaces, And Unattended Gate | Phase 4 + Phase 5 | capstone integration + manual milestone | 6-7 |

## Critical Path And Parallelism

- Longest sequential chain: `1 -> 2 -> 4 -> 6` or `1 -> 3 -> 4 -> 6`.
- Critical-path length: `4` gated dependencies after the shared foundation.
- Phase `5` is the intentional side lane:
  - it depends only on Phase `1` schema activation,
  - it does not block scheduler/PTY/validation construction,
  - it rejoins only at the capstone integration gate in Phase `6`.
- The TS side lane is genuinely parallel:
  - `packages/intelligence` and `packages/manifests` are Bun-only package surfaces,
  - their current package scripts are independent of `crates/daemon`,
  - they depend on Layer 3 schema and contract semantics, not on Rust compile order.
- High-risk surfaces are deliberately early:
  - schema activation and runner/claim ownership in Phase `1`
  - frozen skill binding, source suppression, and breaker logic in Phase `2`
  - PTY/reaper/evidence correctness in Phase `3`
  - manifest/verdict atomicity and fail-open correctness in Phase `4`
  - promotion, blackout, backlog, and status visibility in Phase `6`
- Recommended parallel work split once Phase `1` is green:
  - Lane A: Phase `2` scheduler/source-health work
  - Lane B: Phase `3` PTY/tracing work
  - Lane C: Phase `5` TypeScript relation/error package work
- Phase `1` has an internal parallel seam:
  - `crates/daemon/src/jobs/claim.rs` and `crates/daemon/src/agents/arbiter.rs` may proceed in parallel once the migration and `db/sqlite.rs` truth are frozen.
  - `crates/daemon/src/jobs/runner.rs` plus the phase-end `crates/daemon/src/main.rs` bead are the convergence point.
- Phase `2` has an internal filesystem/code split:
  - the Rust lane (`sources/registry.rs`, breaker state, scheduler wiring) and the operational skill-directory lane (`skills/auth-refresh/*`, `skills/source-health/*`) may proceed in parallel once source IDs, interaction profiles, and the six-check canary contract are frozen.
  - they converge only at the inventory/canary gate.
- Phase `4` has an internal validator split:
  - the manifest/policy/contracts/checkpoint lane and the `skills/validation/base/` filesystem lane may proceed in parallel once the appendix/rubric contract is frozen.
  - `validation/executor.rs`, `jobs/runner.rs`, and the phase-end `crates/daemon/src/main.rs` bead are the convergence seam.
- The merge-hot files are intentionally phase-end integration points:
  - `crates/daemon/src/main.rs` in phases `1`, `2`, `3`, `4`, and `6`
  - `crates/daemon/src/jobs/runner.rs` in phases `1`, `2`, `4`, and `6`
  - `crates/daemon/src/ipc/command_router.rs` in phase `6`

## Suggested Bead Fan-Out

| Phase | Target |
|------|--------|
| 1 | `4-5` beads |
| 2 | `5-6` beads |
| 3 | `5-6` beads |
| 4 | `9-10` beads |
| 5 | `3-4` beads |
| 6 | `6-7` beads |

**Expected total:** `32-38` beads.

## Implementation Unit Allocation

### New Runtime And Schema Surfaces

| Surface | Home Phase | Why it lives there |
|---------|------------|--------------------|
| `crates/daemon/Cargo.toml` | 1 | `cron` and `sysinfo` are compile-time foundation dependencies for later modules and should land before Phase 2/6 start importing them |
| `migrations/0005_l3_operational_hardening.sql` | 1 | All Layer 3 table activation and forward-compatible column use must become truthful before runtime work |
| `crates/daemon/src/jobs/claim.rs` | 1 | Claim SQL and lease ownership must be extracted before scheduler or validator fan-out touches the queue |
| `crates/daemon/src/agents/arbiter.rs` | 1 | Typed admission must exist before any cron catchup or validator admission path is allowed to scale |
| `crates/daemon/src/sources/registry.rs` | 2 | Source definitions must exist before health cadence, suppression, or frozen scheduler binding is meaningful |
| `crates/daemon/src/scheduler/cron.rs` | 2 | Frozen cron roots depend on the schema, registry, and arbiter already being real |
| `crates/daemon/src/pty/hardening.rs` | 3 | PTY hardening is the new runtime reliability root for all later observability and validation work |
| `crates/daemon/src/tracing/structured.rs` | 3 | Control/protocol trace truth belongs with PTY/reaper runtime closure |
| `crates/daemon/src/tracing/forensic.rs` | 3 | Forensic byte capture depends on the PTY/contract capture model being stable |
| `crates/daemon/src/db/checkpoints.rs` | 4 | Checkpoint artifact/path ownership only becomes gateable once partial-failure, manifest appendix, and validator evidence paths are real |
| `crates/daemon/src/validation/manifest_writer.rs` | 4 | The manifest writer is the first new validation-owned runtime surface and must exist before executor prompts |
| `crates/daemon/src/validation/policy.rs` | 4 | Policy decides whether executor work exists at all |
| `crates/daemon/src/validation/executor.rs` | 4 | Executor depends on manifest shape, policy decisions, and runner linkage already being stable |

### Existing Runtime Surfaces With Primary L3 Home

| Surface | Home Phase | Why it lives there |
|---------|------------|--------------------|
| `crates/daemon/src/main.rs` | 1 | Module declarations and L3 bootstrap sequencing start here; later phases only extend the same root |
| `crates/daemon/src/db/sqlite.rs` | 1 | Migration ordering and seeded runtime-state truth start with schema activation |
| `crates/daemon/src/jobs/runner.rs` | 1 | Claim delegation, arbiter admission, validator sub-job enqueue, and continuation semantics all pass through the runner |
| `crates/daemon/src/sessions/reaper.rs` | 3 | Group cleanup, orphan recovery, and archive gating first become fully L3-truthful with PTY hardening |
| `crates/daemon/src/pty/health.rs` | 3 | Host sleep/resource classifications and PTY/runtime health must align to the existing health surface |
| `crates/daemon/src/agents/artifacts.rs` | 4 | Run-manifest structs and size-budget rules are extended here before any executor or telemetry parity gate |
| `crates/daemon/src/agents/contracts.rs` | 4 | Validation dispositions, checkpoint sequencing, and contract alignment are extended here |
| `crates/daemon/src/ipc/command_router.rs` | 6 | Read-only L3 operational status methods become truthful only after cron, health, validation, and arbiter state all exist |

### Bootstrap Touch Points

| Touch Point | Home Phase | Why it lives there |
|------------|------------|--------------------|
| `crates/daemon/src/main.rs` top-level `mod` tree for `jobs::claim` and `agents::arbiter`, plus the phase-end admission/bootstrap integration bead | 1 | Foundation module declarations and admission-state bootstrap wiring should land once per phase after the claim/arbiter seams compile |
| `crates/daemon/src/main.rs` `bootstrap_runtime()` migration/open path and runtime struct initialization | 1 | Schema activation and admission/runtime roots are created here immediately after `open_state_db(...)` |
| `crates/daemon/src/main.rs` top-level `mod` tree for `sources` and `scheduler` plus the phase-end scheduler startup/catchup integration bead in `bootstrap_runtime()` | 2 | Registry and cron are introduced together and should touch bootstrap only after scheduler state and skill bindings are frozen |
| `crates/daemon/src/main.rs` top-level `mod` tree for `pty::hardening` and `tracing::*` plus crash-recovery/evidence wiring | 3 | PTY and evidence surfaces become truthful only when the runtime reliability lane lands |
| `crates/daemon/src/main.rs` top-level `mod` tree for `db::checkpoints` and `validation::*`, plus the phase-end validator/session-completion integration bead | 4 | Checkpoint and validation services should only register after manifest, policy, contract, and executor seams are frozen |
| `crates/daemon/src/main.rs` `serve_handshake_service()` / `build_handshake_context()` read-only status exposure and final rehearsal wiring | 6 | Cron, source health, validation, and arbiter status are only truthful after the capstone integration phase |

### JobRunner Touch Points

| Surface | Home Phase | Why it lives there |
|---------|------------|--------------------|
| `JobRunner::claim_next_job` | 1 | Claim SQL extraction, lease ownership, and arbiter-owned admission chokepoint begin here |
| `JobRunner::enqueue_job` | 2 | Cron root jobs first make this surface L3-truthful; Phase 4 extends the same surface for validator sub-jobs |
| `JobRunner::persist_success` | 4 | Validation-aware completion and worker/verdict linkage first become gateable here |
| `JobRunner::persist_continuation` | 4 | Continuation-chain validation semantics first become truthful with validator-core wiring |
| `JobRunner::mark_obsolete_and_enqueue_rerun` | 6 | Capstone stale/recovery flows are the first L3 phase that may need rerun orchestration against new trust/health state |

### Skill And Package Surfaces

| Surface | Home Phase | Why it lives there |
|---------|------------|--------------------|
| `skills/auth-refresh/alibaba/` | 2 | Source-health and breaker recovery depend on auth-refresh skills before promotion work exists |
| `skills/auth-refresh/1688/` | 2 | Same rationale as Alibaba auth refresh |
| `skills/source-health/fts/` | 2 | Canary cadence belongs with scheduler/registry/breaker activation |
| `skills/source-health/alibaba/` | 2 | Same rationale as FTS canary with source-specific blocker handling |
| `skills/validation/base/` | 4 | The full validator-base directory must exist before executor prompt composition and verdict tests |
| `packages/intelligence/src/relations/resolve-process-relations.ts` | 5 | The relation engine is the standalone TS side lane for L3 clustering semantics |
| `packages/manifests/render-error-manifest.ts` | 5 | Shared error rendering is package-level work, not daemon bootstrap work |
| `packages/manifests/error.md.ts` | 5 | Template surface belongs with the shared error-renderer lane |
| `packages/intelligence/package.json` | 5 | Bun test/build coverage for the new relations lane belongs with the package work, not the Rust foundation |
| `packages/manifests/package.json` | 5 | The renderer test script expands only when the new error-renderer test surface exists |

### Test Home Mapping

| Test Surface | Home Phase | Why it starts there |
|--------------|------------|---------------------|
| `crates/daemon/tests/schema_activation.rs` | 1 | Migration and authority activation are the first L3 truth gates |
| `crates/daemon/tests/job_claim.rs` | 1 | Claim/lease/arbiter ownership begins with the foundation phase |
| `crates/daemon/tests/cron_scheduler.rs` | 2 | Frozen binding, DST, catchup, and suppression first become truthful with the scheduler lane |
| `crates/daemon/tests/skill_drift_monitor.rs` | 2 | Canaries, breaker state, and source-specific mitigation begin with the health-control phase |
| `crates/daemon/tests/fault_process_cleanup.rs` | 3 | Reaper escalation and orphan recovery begin with PTY/runtime hardening |
| `crates/daemon/tests/fault_pty_edge_cases.rs` | 3 | PTY hardening is the file's first truthful home |
| `crates/daemon/tests/replay_pty_sessions.rs` | 3 | PTY capture and replay start with the evidence lane |
| `crates/daemon/tests/replay_contract_sessions.rs` | 3 | Contract replay becomes truthful once structured/forensic evidence exists |
| `crates/daemon/tests/validation_policy.rs` | 4 | Policy semantics begin once manifest inputs and risk signals are real |
| `crates/daemon/tests/validation_executor.rs` | 4 | Executor linkage, prompt composition, and verdict persistence begin in the validation phase |
| `crates/daemon/tests/validation_dry_run.rs` | 4 | Dry-run isolation and fail-open worker advancement begin with validator-core wiring |
| `crates/daemon/tests/validation_golden_verdicts.rs` | 4 | Base-validator corpus and rubric gates start with the validator lane |
| `crates/daemon/tests/fault_partial_failure.rs` | 4 | Partial-failure semantics depend on checkpoint store plus validation bookkeeping |
| `packages/intelligence/src/relations/resolve-process-relations.test.ts` | 5 | Relation heuristics and canonical-root determinism begin with the TS relation engine |
| `packages/manifests/tests/render-error-manifest.test.ts` | 5 | Shared error-renderer behavior begins with the package lane |
| `crates/daemon/tests/skill_golden_journeys.rs` | 6 | Promotion-stage behavior only becomes fully truthful after validation, health, and skills converge |
| `crates/daemon/tests/skill_shadow_compare.rs` | 6 | Shadow/canary/promotion metrics are capstone rollout behavior |
| `crates/daemon/tests/validation_canary_health.rs` | 6 | Synthetic canaries, blackout, and auto-demotion require the full validator subsystem |
| `crates/daemon/tests/validation_retro_backlog.rs` | 6 | Backlog accumulation and recovery are capstone failure-mode behavior |
| `crates/daemon/tests/observability_telemetry.rs` | 2 | ErrorCode/source-health observability assertions start with scheduler/health control; manifest parity expands in Phase 4; the full four-layer join path and status IPC surfaces close in Phase 6 |
| `crates/daemon/tests/fault_host_events.rs` | 6 | Host sleep, pressure, catchup, and source suppression are cross-phase integration behavior |

## Coverage Summary

- New daemon/runtime surfaces assigned: `13 / 13`
- L3-activated tables assigned to a home phase: `5 / 5`
- Shared package surfaces assigned: `5 / 5`
- Skill roots assigned: `5 / 5`
- TDD test files assigned: `21 / 21`
- Critical-path phases: `1, 2, 3, 4, 6`
- Side-lane phase: `5`

## Phase 1: Schema, Claim, And Admission Foundation

**Target beads:** `4-5`

### Prerequisites

- Layer 0-L2 migrations and the current single-DB `StateDb` open path already exist.
- `workflow_jobs`, `agent_session_logs`, and `sources` are the existing authority surfaces to extend rather than replace.
- `crates/daemon/Cargo.toml` is still the single dependency manifest for daemon-only Rust work.

### Ordering

- Sequential: `crates/daemon/Cargo.toml` (`cron`, `sysinfo`) -> `migrations/0005_l3_operational_hardening.sql` -> `crates/daemon/src/db/sqlite.rs`
- Parallel after schema/runtime truth is frozen: `crates/daemon/src/jobs/claim.rs` and `crates/daemon/src/agents/arbiter.rs`
- Sequential convergence: `crates/daemon/src/jobs/runner.rs::claim_next_job` claim/admission delegation -> single Phase 1 `crates/daemon/src/main.rs` integration bead (top-level `mod` additions for `jobs::claim` / `agents::arbiter` plus `bootstrap_runtime()` admission-state insertion point)
- Parallel: `crates/daemon/tests/schema_activation.rs` and `crates/daemon/tests/job_claim.rs` may grow in parallel once the migration file name and the runner-owned claim/admission seam are fixed.

### Phase Assets

- Dependency and schema activation: `crates/daemon/Cargo.toml`, `migrations/0005_l3_operational_hardening.sql`, `crates/daemon/src/db/sqlite.rs`
- Admission lane: `crates/daemon/src/jobs/claim.rs`, `crates/daemon/src/agents/arbiter.rs`, `crates/daemon/src/jobs/runner.rs`
- Bootstrap root: `crates/daemon/src/main.rs`
- Test homes entered: `crates/daemon/tests/schema_activation.rs`, `crates/daemon/tests/job_claim.rs`

### Gate Criteria

- Rust compile gate: `cargo check --all-targets`, `cargo clippy --all-targets -- -D warnings`, and `cargo fmt --check` pass; `CI-L3-001`, `CI-L3-002`, `CI-L3-003`, and `CI-L3-005` are green.
- `L3-T-schema_activation-001` through `L3-T-schema_activation-008` pass.
- `L3-T-job_claim-001` through `L3-T-job_claim-009` pass.
- The runner still owns the only enqueue/order/idempotency lane, with `jobs/claim.rs` becoming the single SQL claim/lease surface rather than a second job system.
- `bootstrap_runtime()` remains the only startup insertion point for L3 schema/admission initialization; no second daemon bootstrap path is introduced.

### Roll Back If

- The migration still requires runtime table-shape drift beyond deterministic `0005` activation.
- `jobs/runner.rs` cannot delegate claim/lease work without keeping competing ad hoc claim SQL alive.
- Typed admission cannot be inserted ahead of execution without bypassing runner-owned job visibility and continuation semantics.

## Phase 2: Scheduler, Registry, And Source Health Control

**Target beads:** `5-6`

### Prerequisites

- Phase 1 gate passed.

### Ordering

- Sequential Rust trunk: `crates/daemon/src/sources/registry.rs` -> canary classification / `source_runtime_state` breaker-write rules -> circuit-breaker transition logic -> `crates/daemon/src/scheduler/cron.rs` -> `crates/daemon/src/jobs/runner.rs::enqueue_job` cron-root use -> single Phase 2 `crates/daemon/src/main.rs` integration bead (`sources` / `scheduler` mod tree plus `bootstrap_runtime()` scheduler startup and catchup/suppression wiring)
- Parallel filesystem lane: operational skill roots under `skills/auth-refresh/*` and `skills/source-health/*` after source IDs, interaction profiles, and the six-check canary contract are frozen; this lane should not block Rust compile work except at the inventory/canary gate.
- Parallel: `crates/daemon/tests/cron_scheduler.rs` and `crates/daemon/tests/skill_drift_monitor.rs` may proceed in parallel once registry IDs, skill-binding rules, and breaker columns are fixed.

### Phase Assets

- Source-definition lane: `crates/daemon/src/sources/registry.rs`
- Scheduler lane: `crates/daemon/src/scheduler/cron.rs`, `crates/daemon/src/main.rs`
- Operational skill lane: `skills/auth-refresh/alibaba/`, `skills/auth-refresh/1688/`, `skills/source-health/fts/`, `skills/source-health/alibaba/`
- Test homes entered: `crates/daemon/tests/cron_scheduler.rs`, `crates/daemon/tests/skill_drift_monitor.rs`, `crates/daemon/tests/observability_telemetry.rs`

### Gate Criteria

- Rust compile gate: `cargo check --all-targets`, `cargo clippy --all-targets -- -D warnings`, and `cargo fmt --check` pass; `CI-L3-001`, `CI-L3-002`, `CI-L3-003`, and `CI-L3-005` are green.
- `L3-T-cron_scheduler-001` through `L3-T-cron_scheduler-010` pass.
- `L3-T-skill_drift_monitor-001` through `L3-T-skill_drift_monitor-006` pass.
- `L3-T-observability_telemetry-012` passes.
- Frozen skill binding, catchup ordering, and source suppression all route through persisted scheduler/runtime state rather than in-memory-only shortcuts.
- Scheduler startup is anchored in the daemon bootstrap path, not the ad hoc live-loop path.

### Roll Back If

- Cron roots need late-bound skill resolution at execution time instead of enqueue-time binding.
- Source suppression cannot be expressed from persisted `source_runtime_state` and instead depends on hidden process memory.
- Source registry identity/version tracking requires remote-owned config before local Layer 3 automation can function.

## Phase 3: PTY Reliability And Four-Layer Evidence

**Target beads:** `5-6`

### Prerequisites

- Phase 1 gate passed.

### Ordering

- Sequential: `crates/daemon/src/pty/hardening.rs` -> `crates/daemon/src/pty/health.rs` and `crates/daemon/src/sessions/reaper.rs` L3 extensions -> `crates/daemon/src/tracing/structured.rs` -> `crates/daemon/src/tracing/forensic.rs` -> single Phase 3 `crates/daemon/src/main.rs` integration bead
- Parallel: PTY cleanup tests and replay tests may advance in parallel after the hardening/tracing surfaces are stubbed because they share the same runtime harnesses.

### Phase Assets

- PTY/reaper lane: `crates/daemon/src/pty/hardening.rs`, `crates/daemon/src/pty/health.rs`, `crates/daemon/src/sessions/reaper.rs`
- Evidence lane: `crates/daemon/src/tracing/structured.rs`, `crates/daemon/src/tracing/forensic.rs`
- Bootstrap root: `crates/daemon/src/main.rs`
- Test homes entered: `crates/daemon/tests/fault_process_cleanup.rs`, `crates/daemon/tests/fault_pty_edge_cases.rs`, `crates/daemon/tests/replay_pty_sessions.rs`, `crates/daemon/tests/replay_contract_sessions.rs`

### Gate Criteria

- Rust compile gate: `cargo check --all-targets`, `cargo clippy --all-targets -- -D warnings`, and `cargo fmt --check` pass; `CI-L3-001`, `CI-L3-002`, and `CI-L3-003` remain green.
- `L3-T-fault_process_cleanup-001` through `L3-T-fault_process_cleanup-008` pass.
- `L3-T-fault_pty_edge_cases-001` through `L3-T-fault_pty_edge_cases-008` pass.
- `L3-T-replay_pty_sessions-001` through `L3-T-replay_pty_sessions-006` pass.
- `L3-T-replay_contract_sessions-001` through `L3-T-replay_contract_sessions-003` and `L3-T-replay_contract_sessions-006` pass.
- PTY sessions still follow the existing seven-phase lifecycle; hardening and tracing add evidence and cleanup guarantees without inventing a second state machine.

### Roll Back If

- PTY hardening requires new lifecycle phases or a parallel host-health enum instead of extending the existing surfaces.
- Cleanup completeness still depends on later validator or scheduler code to assign truthful verdicts.
- Control/protocol/forensic evidence cannot stay separated into replayable layers and instead collapses into one transcript-only surface.

## Phase 4: Manifest And Validation Dry-Run Core

**Target beads:** `9-10`

### Prerequisites

- Phase 2 gate passed.
- Phase 3 gate passed.

### Ordering

- Sequential trunk: `crates/daemon/src/agents/artifacts.rs` Layer 3 field extensions -> `crates/daemon/src/validation/manifest_writer.rs`
- Parallel after the manifest appendix contract is frozen:
  - contract/checkpoint lane: `crates/daemon/src/agents/contracts.rs` Layer 3 validation/checkpoint additions -> `crates/daemon/src/db/checkpoints.rs`
  - policy lane: `crates/daemon/src/validation/policy.rs`
  - validator skill lane: `skills/validation/base/`
- Sequential convergence: `crates/daemon/src/validation/executor.rs` -> `crates/daemon/src/jobs/runner.rs::enqueue_job` validator-subjob linkage -> `crates/daemon/src/jobs/runner.rs::persist_success` / `persist_continuation` validation-aware completion wiring -> single Phase 4 `crates/daemon/src/main.rs` integration bead (`db::checkpoints` / `validation::*` mod tree plus validator/runtime service wiring)
- Parallel: policy tests can proceed once decision keys are frozen; golden-verdict and dry-run suites can proceed once the base-validator directory and executor payload contract exist.

### Phase Assets

- Manifest lane: `crates/daemon/src/agents/artifacts.rs`, `crates/daemon/src/validation/manifest_writer.rs`
- Validation lane: `crates/daemon/src/validation/policy.rs`, `crates/daemon/src/validation/executor.rs`, `crates/daemon/src/agents/contracts.rs`, `crates/daemon/src/db/checkpoints.rs`, `crates/daemon/src/jobs/runner.rs`, `crates/daemon/src/main.rs`
- Validator skill lane: `skills/validation/base/`
- Test homes entered: `crates/daemon/tests/validation_policy.rs`, `crates/daemon/tests/validation_executor.rs`, `crates/daemon/tests/validation_dry_run.rs`, `crates/daemon/tests/validation_golden_verdicts.rs`, `crates/daemon/tests/fault_partial_failure.rs`
- Test surface expansion: `crates/daemon/tests/observability_telemetry.rs` for manifest-field and appendix-parity coverage (`L3-T-observability_telemetry-008..011`)

### Gate Criteria

- Rust compile gate: `cargo check --all-targets`, `cargo clippy --all-targets -- -D warnings`, and `cargo fmt --check` pass; `CI-L3-001`, `CI-L3-002`, `CI-L3-003`, `CI-L3-005`, and `CI-L3-007` are green.
- `L3-T-validation_policy-001` through `L3-T-validation_policy-018` pass.
- `L3-T-validation_executor-001` through `L3-T-validation_executor-012` pass.
- `L3-T-validation_dry_run-001` through `L3-T-validation_dry_run-010` pass.
- `L3-T-validation_golden_verdicts-001` through `L3-T-validation_golden_verdicts-007` pass.
- `L3-T-fault_partial_failure-001` through `L3-T-fault_partial_failure-007` pass.
- `L3-T-observability_telemetry-008` through `L3-T-observability_telemetry-011` pass.
- Validator sub-jobs still enqueue through `EnqueueJobRequest`, manifests stay under the bounded contract, and verdict persistence remains atomic with worker linkage.
- `persist_success` and `persist_continuation` are the only completion chokepoints extended for validator-linked worker semantics; the strategy does not scatter those semantics across unrelated runtime files.

### Roll Back If

- Validator execution needs a validator-only path that bypasses the existing adapter/runner/contracts boundaries.
- Run manifests cannot satisfy the bounded-evidence contract without falling back to raw forensic bytes or unbounded result payloads.
- Verdict persistence cannot remain atomic with `workflow_jobs.validation_verdict_id` linkage.

## Phase 5: Relations And Error-Surface Packages

**Target beads:** `3-4`

### Prerequisites

- Phase 1 gate passed.

### Ordering

- Parallel: `packages/intelligence/src/relations/resolve-process-relations.ts` plus `packages/intelligence/package.json` and the `packages/manifests` error-renderer/template lane plus `packages/manifests/package.json` may proceed independently once the L3 schema tables exist.
- Sequential inside the relation lane: deterministic blocker/rule/scoring core -> canonical-root and alias handling -> late-merge stale-decision handling.
- Sequential inside the manifests lane: error template -> renderer -> package tests.

### Phase Assets

- Relation engine: `packages/intelligence/src/relations/resolve-process-relations.ts`, `packages/intelligence/package.json`
- Shared error rendering: `packages/manifests/render-error-manifest.ts`, `packages/manifests/error.md.ts`, `packages/manifests/package.json`
- Test homes entered: `packages/intelligence/src/relations/resolve-process-relations.test.ts`, `packages/manifests/tests/render-error-manifest.test.ts`

### Gate Criteria

- Bun gate: `bun run test`, `(cd packages/intelligence && bun run test)`, and `(cd packages/manifests && bun test)` pass; `CI-L3-004` is green.
- `L3-T-resolve_process_relations-001` through `L3-T-resolve_process_relations-015` pass.
- `L3-T-render_error_manifest-001` through `L3-T-render_error_manifest-004` pass.
- Relation evidence, canonical-root choice, and alias stability stay deterministic and replayable from persisted evidence rather than ad hoc runtime memory.
- The Bun package lane remains independent of Rust/Cargo gates apart from the shared schema/contract assumptions fixed in earlier phases.

### Roll Back If

- Relation resolution requires daemon-owned mutable global state and therefore cannot remain a deterministic package lane.
- Canonical-root/alias stability cannot be preserved without destructive rewrite of historical process identity.
- Shared error rendering requires direct dependence on local-only SQLite rows instead of a package-level input contract.

## Phase 6: Promotion, Status Surfaces, And Unattended Gate

**Target beads:** `6-7`

### Prerequisites

- Phase 4 gate passed.
- Phase 5 gate passed.

### Ordering

- Sequential: promotion-stage and behavior-version bookkeeping across the existing runtime/skill surfaces -> validator canary health and blackout/backlog orchestration -> read-only operational snapshot builders in `crates/daemon/src/main.rs` / `build_handshake_context()` -> `crates/daemon/src/ipc/command_router.rs` operational status methods -> final telemetry/observability drill-down and unattended rehearsal wiring in `crates/daemon/src/main.rs`
- Parallel: promotion journey tests and shadow-comparison tests may proceed in parallel once stage transitions and behavior-version rules are fixed; status/telemetry and backlog tests may proceed in parallel once the read-only snapshot contracts and command-router method names are frozen.

### Phase Assets

- Capstone integration surfaces: `crates/daemon/src/ipc/command_router.rs`, `crates/daemon/src/main.rs`, `crates/daemon/src/jobs/runner.rs`
- Promotion and health state consumers: `skills/*` runtime metadata, `validation_verdicts`, `source_runtime_state`
- Test homes entered: `crates/daemon/tests/skill_golden_journeys.rs`, `crates/daemon/tests/skill_shadow_compare.rs`, `crates/daemon/tests/validation_canary_health.rs`, `crates/daemon/tests/validation_retro_backlog.rs`, `crates/daemon/tests/observability_telemetry.rs`, `crates/daemon/tests/fault_host_events.rs`

### Gate Criteria

- Capstone automated gate: `cargo check --all-targets`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`, `bun run test`, `(cd packages/intelligence && bun run test)`, and `(cd packages/manifests && bun test)` pass; `CI-L3-001` through `CI-L3-008` are green.
- `L3-T-skill_golden_journeys-001` through `L3-T-skill_golden_journeys-008` pass.
- `L3-T-skill_shadow_compare-001` through `L3-T-skill_shadow_compare-007` pass.
- `L3-T-validation_canary_health-001` through `L3-T-validation_canary_health-008` pass.
- `L3-T-validation_retro_backlog-001` through `L3-T-validation_retro_backlog-006` pass.
- `L3-T-observability_telemetry-001` through `L3-T-observability_telemetry-015` pass.
- `L3-T-fault_host_events-001` through `L3-T-fault_host_events-010` pass.
- Capstone milestone hold: the TDD Manual Gate and spec Section 5.1 unattended-operation exit remain open until the separate five-consecutive-day rehearsal reaches `>=95%` skill success; they do not substitute for the automated phase gate above.

### Roll Back If

- Promotion flow requires stage skipping or cannot use validator approval plus source-health signals as first-class gates.
- Operational status methods mutate runtime state or require non-authoritative shadow state outside cron/source-health/validation/arbiter tables.
- Blackout and retro-backlog handling cannot preserve fail-open worker availability while still surfacing trust debt and stale decisions explicitly.

## Dependency Diagram

```text
Phase 1 -> Phase 2 -> Phase 4 -> Phase 6
    |           ^          ^
    |           |          |
    +-> Phase 3 +----------+
    |
    +-> Phase 5 -----------+
```

## Completeness Mapping

| Spec Section | Phase | Notes |
|-------------|-------|-------|
| Sec. 1.1-1.2 | 6 | Layer narrative and unattended exit target only become fully gateable at the capstone milestone |
| Sec. 1.3.3, 1.3.6 | 1 | claim and arbiter foundation |
| Sec. 1.3.7 | 4 | checkpoint persistence only becomes gateable once partial-failure, manifest appendix, and validator evidence paths are real |
| Sec. 1.4 | 1 | typed quota model first becomes truthful with admission ownership |
| Sec. 4.1-4.3 | 1 | all five tables and forward-compatible column activation start here |
| Sec. 1.3.1-1.3.2 | 2 | scheduler and registry first become gateable together |
| Sec. 1.5 | 2 | frozen skill binding and catchup are scheduler-owned |
| Sec. 3.4-3.5 | 2 | source health and breaker flow depend on registry plus scheduler suppression |
| Sec. 1.9 | 2 + 4 | operational auth-refresh/source-health roots land in Phase 2; the validator-base directory closes in Phase 4 |
| Sec. 3.7 | 2 | source-specific anti-bot difficulty and mitigation are upstream inputs to canary pacing and suppression |
| Sec. 1.3.4-1.3.5 | 3 | reaper extension and PTY hardening first become truthful here |
| Sec. 1.3.8-1.3.9 | 3 | structured and forensic evidence layers begin here |
| Sec. 1.6 | 3 | PTY hardening bundle is phase-owned runtime work |
| Sec. 1.3.10-1.3.12 | 4 | validation surfaces begin with manifest writer, policy, and executor |
| Sec. 1.10 | 4 | L3 dry-run stubs become truthful with validator-core activation |
| Sec. 2.1-2.2 | 4 | manifest recap and Layer 3 appendix additions are phase-owned by the writer contract |
| Sec. 2.3 | 4 | policy engine ownership |
| Sec. 2.4-2.6 | 4 | executor, outcome handling, and dry-run isolation |
| Sec. 2.8-2.9 | 4 | base validator and initial calibration corpus/rubric work |
| Sec. 1.8 | 5 | shared-library additions are the package lane |
| Sec. 3.6 | 5 | relation resolution is the standalone TS engine lane |
| Sec. 2.7 | 6 | fail-open blackout, trust-state rollups, canaries, and retro backlog only close at capstone |
| Sec. 3.1-3.3 | 6 | promotion flow and behavior-version/promotion gates are capstone rollout behavior |
| Sec. 3.8 | 6 | browser-automation isolation/readiness/fingerprint proof only close once operational skill roles, health gating, and promotion-state behavior converge |
| Sec. 1.7 | 6 | host sleep/wake, catchup recovery, and pressure gating are cross-phase runtime integration behavior |
| Sec. 4.4 | 6 | the full four-layer join path is only complete once manifests, bytes, traces, and status surfaces all exist |
| Sec. 4.5-4.6 | 6 | skill and validation telemetry thresholds are capstone visibility work |
| Sec. 5.1-5.4 | 6 | DoD, scenario closure, and release-risk ownership are capstone gates |
| Sec. 6.1-6.5 | 6 | forward-compatibility, dependency closure, and ADR reconciliation are final-layer routing context |

**Coverage:** complete. All implementation-bearing sections in `Sections 1-6` have a primary home or an explicit split-home where one spec section intentionally spans multiple Layer 3 lanes; context-only narrative and milestone sections are assigned to their capstone phase rather than left unmapped.

## Gates Summary

| Phase | Gate | Roll Back If |
|------|------|-------------|
| 1 | `cargo check --all-targets`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`, `CI-L3-005`, `L3-T-schema_activation-001..008`, `L3-T-job_claim-001..009` | schema still depends on runtime drift, or claim/admission bypasses runner truth |
| 2 | `cargo check --all-targets`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`, `L3-T-cron_scheduler-001..010`, `L3-T-skill_drift_monitor-001..006`, `L3-T-observability_telemetry-012` | scheduler needs late-bound skill resolution or source suppression cannot rely on persisted state |
| 3 | `cargo check --all-targets`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`, `L3-T-fault_process_cleanup-001..008`, `L3-T-fault_pty_edge_cases-001..008`, `L3-T-replay_pty_sessions-001..006`, `L3-T-replay_contract_sessions-001..003,006` | hardening requires a second lifecycle/health model or evidence layers collapse into transcript-only behavior |
| 4 | `cargo check --all-targets`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`, `CI-L3-007`, `L3-T-validation_policy-001..018`, `L3-T-validation_executor-001..012`, `L3-T-validation_dry_run-001..010`, `L3-T-validation_golden_verdicts-001..007`, `L3-T-fault_partial_failure-001..007`, `L3-T-observability_telemetry-008..011` | validator core bypasses runner/contracts, manifests cannot stay bounded, or verdict linkage is non-atomic |
| 5 | `bun run test`, `(cd packages/intelligence && bun run test)`, `(cd packages/manifests && bun test)`, `CI-L3-004`, `L3-T-resolve_process_relations-001..015`, `L3-T-render_error_manifest-001..004` | relation determinism or shared error rendering requires hidden daemon-only state |
| 6 | `cargo check --all-targets`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`, `bun run test`, `(cd packages/intelligence && bun run test)`, `(cd packages/manifests && bun test)`, `CI-L3-001..008`, `L3-T-skill_golden_journeys-001..008`, `L3-T-skill_shadow_compare-001..007`, `L3-T-validation_canary_health-001..008`, `L3-T-validation_retro_backlog-001..006`, `L3-T-observability_telemetry-001..015`, `L3-T-fault_host_events-001..010`, plus Manual Gate hold | promotion requires stage skipping, status surfaces mutate state, or blackout/backlog logic cannot preserve fail-open availability with explicit trust debt |
