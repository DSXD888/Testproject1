# L1 Daemon Core — Implementation Strategy

## Source Documents

| Document | Path | Role |
|----------|------|------|
| Spec | `docs/strategy/layers/layer-1-thin-local-runner.md` | SOT #1 — WHAT to build |
| TDD | `docs/pipeline/l1-daemon-core/artifacts/l1-daemon-core-tdd.md` | SOT #2 — HOW to verify |
| Bootstrap | `docs/pipeline/l1-daemon-core/artifacts/l1-daemon-core-bootstrap.md` | Harness/bootstrap reference |
| Cross-layer sequencing | `docs/strategy/layer-strategy.md` §13 | Rust stack introduction order |
| Cross-reference index | `docs/strategy/cross-reference-index.md` | Section lookup only |

## Strategy Guardrails

- This is a Rust-only layer. All implementation sequencing assumes one crate, `crates/daemon`, plus root workspace files.
- L1 is sequenced as a binary-first crate. No `lib.rs` split is assumed in this layer. Parent `mod` declarations must only name files already landed in earlier or same-phase work, using inline directory-module declarations in `main.rs` so the 17-file inventory does not silently expand with extra `mod.rs` files.
- Phase mappings use a primary-home rule: every file/section is assigned to the phase where it first becomes gateable. Later phases may extend earlier files without changing their home phase.
- Every phase has a compile gate. `cargo check --all-targets` stays green after each phase; `T1/T2` remain the cumulative per-commit gate, `T3` is nightly, `T4` is scheduled live-network coverage, and `Manual Gate` is target-host signoff.
- Layer 0 is a hard prerequisite. Phase 1 assumes `0001_sqlite_state_initial.sql`, the Layer 0 skill standard, and the TypeScript intelligence pipeline already exist.
- The phase DAG must stay acyclic. Internal parallelism is allowed only where the spec’s dependency graph permits it.
- Bead sizing is calculated from production surfaces first. Test/support files are phased with their owning runtime surface but do not force a separate phase by themselves.

## Overview

| Phase | Name | Prerequisites | Gate |
|-------|------|---------------|------|
| 1 | Workspace And SQLite Foundation | Layer 0 schema + skills available | Workspace bootstraps, `0003_sqlite_runtime.sql` applies after `0001`, and static Rust gates pass |
| 2 | Walking Skeleton Startup | Phase 1 | Daemon starts, opens `state.db`, binds the socket, writes `daemon.info.json`, and exits cleanly |
| 3 | PTY Spawn And Capture | Phase 2 | PTY session semantics and chunked capture work with real helper processes |
| 4 | Health And Reaper Closure | Phase 3 | Seven-phase lifecycle, cleanup ladder, verdicts, and cleanup reports are mechanically testable |
| 5 | Adapter Contract And Vendor Probes | Phase 3 | Shared adapter trait exists and both vendor adapters satisfy the mode/probe/parser contract |
| 6 | Artifact Contracts And Contract Validation | Phase 5 | Artifact store, run-manifest/result-contract handling, and contract-mode persistence are testable |
| 7 | Runner Commit And Stage Tracking | Phase 4 + Phase 6 | Job claim/execute/persist flow, `commit_seq`, idempotent acks, and `stage_run_id` cohort rules are durable |
| 8 | Error Manifests | Phase 7 | Runtime failures produce stored Markdown error manifests with stable runtime context |
| 9 | Prompt, Skills, And Canary Gates | Phase 7 | Real skill directories, lint gate, prompt renderer, and canary lanes are valid against the daemon runtime |
| 10 | Full Live Loop And Reliability | Phase 8 + Phase 9 | Full L1 loop passes end to end, restart semantics hold, and Experiment 4 is the final release gate |

## Implementation Unit Allocation

### Bootstrap And Harness Support

| Surface | Home Phase | Notes |
|---------|------------|-------|
| Root `Cargo.toml` | 1 | Workspace root exists before any Rust module work |
| `rust-toolchain.toml` | 1 | Nightly + `rustfmt`/`clippy` are phase-1 bootstrap surfaces |
| `crates/daemon/Cargo.toml` | 1 | Declares prod/dev dependencies and `test-support` helper feature |
| `crates/daemon/src/bin/test_agent.rs` | 3 | Real helper binary arrives before PTY/reaper tests |
| `crates/daemon/tests/support/mod.rs` | 1 | Base support module wiring |
| `crates/daemon/tests/support/db.rs` | 1 | `TestStateDb` bootstrap starts with migration work |
| `crates/daemon/tests/support/process.rs` | 3 | Process-tree fixtures needed before PTY/reaper gates |
| `crates/daemon/tests/support/artifacts.rs` | 3 | Chunk/index assertions begin with capture |
| `crates/daemon/tests/support/assertions.rs` | 6 | Manifest/result assertion helpers begin with artifact contracts |
| `crates/daemon/tests/support/skills.rs` | 9 | Skill-directory fixtures and canary contexts belong with real skills |

### Core Runtime Surfaces

| Surface | Home Phase | Why it lives there |
|---------|------------|--------------------|
| `migrations/0003_sqlite_runtime.sql` | 1 | Depends on L0 `0001_sqlite_state_initial.sql`; must exist before any runtime write path |
| `crates/daemon/src/db/sqlite.rs` | 1 | SQLite open/migrate/PRAGMA/commit-clock foundation |
| `crates/daemon/src/main.rs` | 2 | Walking-skeleton bootstrap starts here; later phases extend the same file |
| `crates/daemon/src/ipc/descriptor.rs` | 2 | Descriptor/lock readiness belongs in the startup skeleton |
| `crates/daemon/src/ipc/handshake.rs` | 2 | Handshake-only socket surface belongs in the startup skeleton |
| `crates/daemon/src/pty/session.rs` | 3 | PTY chain root; capture and reaper depend on it |
| `crates/daemon/src/pty/capture.rs` | 3 | Depends on session-owned PTY master/sequence surface |
| `crates/daemon/src/pty/health.rs` | 4 | Needs active capture/session metrics before it is useful |
| `crates/daemon/src/sessions/reaper.rs` | 4 | Depends on session pid/pgid mapping; required before any session can be complete |
| `crates/daemon/src/agents/adapter.rs` | 5 | Trait/enum contract must exist before vendor adapters |
| `crates/daemon/src/agents/codex.rs` | 5 | Parallel vendor implementation after adapter trait exists |
| `crates/daemon/src/agents/claude.rs` | 5 | Parallel vendor implementation after adapter trait exists |
| `crates/daemon/src/agents/artifacts.rs` | 6 | Depends on adapter/event model and capture layout |
| `crates/daemon/src/agents/contracts.rs` | 6 | Depends on adapter boundary and artifact/result paths |
| `crates/daemon/src/jobs/runner.rs` | 7 | Depends on DB + PTY/reaper + adapters + contracts/artifacts |
| `crates/daemon/src/errors/manifest.rs` | 8 | Depends on runner/session/runtime context being stable |
| `crates/daemon/src/errors/markdown.rs` | 8 | Parallel renderer once error-manifest shape exists |
| `crates/daemon/src/prompt/renderer.rs` | 9 | Depends on runner invocation context and real skill directories |

### Skill Directories

| Surface | Home Phase | Why it lives there |
|---------|------------|--------------------|
| `skills/procurement-ingestion/fts/` | 9 | Must come after PTY + adapter + runner surfaces are real |
| `skills/manifest-generation/` | 9 | Must come after contract mode, artifact store, and runner exist |

### Test File Homes

| Test File | Home Phase | Why it starts there |
|-----------|------------|---------------------|
| `crates/daemon/tests/ipc_handshake.rs` | 1 | Starts with migration/bootstrap coverage, then expands through startup/restart phases |
| `crates/daemon/tests/agent_pty_session.rs` | 3 | PTY semantics and capture are the first real runtime process surface |
| `crates/daemon/tests/session_reaper.rs` | 4 | Reaper-specific waitpid/cleanup gates start only after PTY session ownership exists |
| `crates/daemon/tests/adapter_codex.rs` | 5 | First vendor-adapter contract home |
| `crates/daemon/tests/adapter_claude.rs` | 5 | Second vendor-adapter contract home |
| `crates/daemon/tests/agent_contract_session.rs` | 6 | Contract-mode session persistence starts when artifacts/contracts exist |
| `crates/daemon/tests/adapter_result_contract.rs` | 6 | Result/run-manifest contract surface starts with artifact validation |
| `crates/daemon/tests/skill_lint.rs` | 9 | Needs real skill directories and prompt contract fields |
| `crates/daemon/tests/skill_canary_harness.rs` | 9 | Needs real skills plus runnable adapter/runtime surfaces |
| `crates/daemon/tests/agent_procurement_ingestion.rs` | 10 | Full live-loop test home; final convergence of all prior phases |

## Suggested Bead Fan-Out

| Phase | Primary production surfaces | Suggested bead fan-out |
|-------|-----------------------------|------------------------|
| 1 | Workspace manifests + migration + `db/sqlite.rs` | 2-3 beads |
| 2 | `main.rs` + `ipc/descriptor.rs` + `ipc/handshake.rs` | 1-2 beads |
| 3 | `pty/session.rs` + `pty/capture.rs` + helper binary | 2-3 beads |
| 4 | `pty/health.rs` + `sessions/reaper.rs` | 1-2 beads |
| 5 | `agents/adapter.rs` + `agents/codex.rs` + `agents/claude.rs` | 2-3 beads |
| 6 | `agents/artifacts.rs` + `agents/contracts.rs` | 1-2 beads |
| 7 | `jobs/runner.rs` | 1-2 beads |
| 8 | `errors/manifest.rs` + `errors/markdown.rs` | 1-2 beads |
| 9 | `prompt/renderer.rs` + 2 skill directories | 2-3 beads |
| 10 | Integration/reliability expansion of existing runtime surfaces | 1-3 beads |

## Phase 1: Workspace And SQLite Foundation

### Prerequisites

- Layer 0 `0001_sqlite_state_initial.sql` is already authoritative.
- Layer 0 skill standard and TypeScript pipeline exist, even though L1 does not invoke them yet.

### Ordering

- Sequential: root `Cargo.toml` -> `rust-toolchain.toml` -> `crates/daemon/Cargo.toml` -> `0003_sqlite_runtime.sql` -> `src/db/sqlite.rs`
- Parallel: `tests/support/mod.rs` and `tests/support/db.rs` after the workspace manifests exist

### Phase Assets

- Bootstrap surfaces: root manifests, `crates/daemon/Cargo.toml`, `tests/support/mod.rs`, `tests/support/db.rs`
- Core surfaces: `migrations/0003_sqlite_runtime.sql`, `src/db/sqlite.rs`
- Test home entered: `tests/ipc_handshake.rs` for `IT-IPC-013`

### Gate Criteria

- `CI-CORE-001` through `CI-CORE-006` pass.
- `IT-IPC-013` passes.
- `cargo check --all-targets` passes with only workspace/bootstrap + DB/migration surfaces active.

### Roll Back If

- `0003_sqlite_runtime.sql` cannot apply after `0001_sqlite_state_initial.sql` inside the same `state.db`.
- SQLite startup requires a second metadata database, contradicting the single-database model.

## Phase 2: Walking Skeleton Startup

### Prerequisites

- Phase 1 gate passed.

### Ordering

- Sequential: `src/main.rs` (minimal startup shell) -> `src/ipc/descriptor.rs` -> `src/ipc/handshake.rs`
- Parallel: expand `tests/ipc_handshake.rs` from DB-startup coverage into descriptor/lock/handshake coverage once both IPC files exist

### Phase Assets

- Core surfaces: `src/main.rs`, `src/ipc/descriptor.rs`, `src/ipc/handshake.rs`
- Test surface expansion: `tests/ipc_handshake.rs`

### Gate Criteria

- `IT-IPC-001` through `IT-IPC-005` pass.
- `IT-IPC-008` through `IT-IPC-010` pass.
- `cargo check --all-targets` passes.
- Walking skeleton criterion: the daemon can start, open SQLite, bind the UDS socket, write `daemon.info.json`, and exit cleanly without any session runtime.

### Roll Back If

- Descriptor/lock readiness cannot be determined before later session/runtime features exist.
- Handshake compatibility depends on job runner or skill surfaces instead of startup-owned state.

## Phase 3: PTY Spawn And Capture

### Prerequisites

- Phase 2 gate passed.

### Ordering

- Sequential: `src/pty/session.rs` -> `src/pty/capture.rs`
- Parallel: `src/bin/test_agent.rs`, `tests/support/process.rs`, `tests/support/artifacts.rs`, and the first PTY-session test surfaces after `session.rs` exists

### Phase Assets

- Core surfaces: `src/pty/session.rs`, `src/pty/capture.rs`
- Support surfaces: `src/bin/test_agent.rs`, `tests/support/process.rs`, `tests/support/artifacts.rs`
- Test home entered: `tests/agent_pty_session.rs`

### Gate Criteria

- `UT-PTY-001` through `UT-PTY-007` pass.
- `IT-PTY-009` through `IT-PTY-011` pass.
- `IT-PTY-019` and `IT-PTY-022` pass.
- `E2E-PTY-017` and `E2E-PTY-018` pass.
- `cargo check --all-targets` passes.

### Roll Back If

- PTY session semantics require bypassing the spec’s `setsid`/ctty/CLOEXEC constraints.
- Capture cannot stay chunked and crash-safe from the first committed implementation.

## Phase 4: Health And Reaper Closure

### Prerequisites

- Phase 3 gate passed.

### Ordering

- Sequential: `src/pty/health.rs` -> `src/sessions/reaper.rs`
- Parallel: expand `tests/agent_pty_session.rs` lifecycle coverage and add reaper-owned tests once the reaper exists

### Phase Assets

- Core surfaces: `src/pty/health.rs`, `src/sessions/reaper.rs`
- Test home entered: `tests/session_reaper.rs`
- Test surface expansion: `tests/agent_pty_session.rs`

### Gate Criteria

- `IT-PTY-012` through `IT-PTY-016` pass.
- `IT-RPR-001` through `IT-RPR-007` pass.
- `IT-RPR-009`, `IT-RPR-010`, `IT-RPR-011`, and `IT-RPR-013` pass.
- `cargo check --all-targets` passes.

### Roll Back If

- Cleanup completeness still depends on later runner/error-system code to assign verdicts.
- The reaper cannot verify descendant cleanup within the fixed L1 timing budgets.

## Phase 5: Adapter Contract And Vendor Probes

### Prerequisites

- Phase 3 gate passed.

### Ordering

- Sequential: `src/agents/adapter.rs`
- Parallel: `src/agents/codex.rs` and `src/agents/claude.rs` after the trait/enums are frozen for L1

### Phase Assets

- Core surfaces: `src/agents/adapter.rs`, `src/agents/codex.rs`, `src/agents/claude.rs`
- Test homes entered: `tests/adapter_codex.rs`, `tests/adapter_claude.rs`

### Gate Criteria

- `IT-PTY-008` passes.
- `UT-CON-004` and `UT-CON-005` pass.
- `UT-CDX-001` through `UT-CDX-010` and `IT-CDX-010` pass.
- `UT-CLD-001` through `UT-CLD-010` and `IT-CLD-010` pass.
- `cargo check --all-targets` passes.

### Roll Back If

- Mode resolution needs runner-owned mutable state instead of a pure `SkillManifest + CapabilityReport` decision.
- Vendor parsers cannot honor the two-lane model without leaking transport-specific behavior into the runner.

## Phase 6: Artifact Contracts And Contract Validation

### Prerequisites

- Phase 5 gate passed.

### Ordering

- Parallel: `src/agents/artifacts.rs` and `src/agents/contracts.rs`
- Sequential after both are present: manifest/result assertion helpers -> contract-mode session tests

### Phase Assets

- Core surfaces: `src/agents/artifacts.rs`, `src/agents/contracts.rs`
- Support surface: `tests/support/assertions.rs`
- Test homes entered: `tests/agent_contract_session.rs`, `tests/adapter_result_contract.rs`

### Gate Criteria

- `UT-CON-001` through `UT-CON-003` pass.
- `IT-CON-016` passes.
- `UT-RSL-001` through `UT-RSL-007` pass.
- `IT-RSL-008`, `IT-RSL-009`, `IT-RSL-011`, and `IT-RSL-012` pass.
- `cargo check --all-targets` passes.

### Roll Back If

- Result validation still needs to sit inside vendor adapters instead of `agents/contracts.rs`.
- Run-manifest fallback or artifact hashing cannot be made independent of the later error-manifest layer.

## Phase 7: Runner Commit And Stage Tracking

### Prerequisites

- Phase 4 gate passed.
- Phase 6 gate passed.

### Ordering

- Sequential: `src/jobs/runner.rs`
- Parallel within the phase: extend contract-session tests for commit fence, retries, continuation, stage tracking, and WAL contention once the runner exists

### Phase Assets

- Core surface: `src/jobs/runner.rs`
- Test surface expansion: `tests/agent_contract_session.rs`, `tests/adapter_result_contract.rs`

### Gate Criteria

- `IT-CON-006` through `IT-CON-018` pass.
- `cargo check --all-targets` passes.
- Runner gate: one job can claim -> execute -> persist -> acknowledge without needing the final prompt/skill layer.

### Roll Back If

- `stage_run_id`, `commit_seq`, or idempotent ack behavior require the prompt/skill layer to be present before they can be tested.
- Set-replacement vs additive cohort rules cannot be enforced before live-source integration.

## Phase 8: Error Manifests

### Prerequisites

- Phase 7 gate passed.

### Ordering

- Parallel: `src/errors/manifest.rs` and `src/errors/markdown.rs`
- Sequential after both exist: wire runtime context into the existing result/contract error tests

### Phase Assets

- Core surfaces: `src/errors/manifest.rs`, `src/errors/markdown.rs`
- Test surface expansion: `tests/adapter_result_contract.rs`, `tests/agent_contract_session.rs`

### Gate Criteria

- `IT-RSL-010` and `IT-RSL-014` pass.
- `IT-CON-009`, `IT-CON-010`, and `IT-CON-011` remain green with the real error-manifest path active.
- `cargo check --all-targets` passes.

### Roll Back If

- Error manifests cannot be rendered from runner/session context without inventing L3 validator state.
- Resolution tracking requires mutating original failure records instead of adding resolution metadata.

## Phase 9: Prompt, Skills, And Canary Gates

### Prerequisites

- Phase 7 gate passed.
- Phase 8 is not a prerequisite; this phase is intentionally parallel with Phase 8 after runner stability exists.

### Ordering

- Sequential: `src/prompt/renderer.rs`
- Parallel after prompt contract is stable: `skills/procurement-ingestion/fts/` and `skills/manifest-generation/`
- Sequential after production skill directories exist: `tests/support/skills.rs` -> `tests/skill_lint.rs` -> `tests/skill_canary_harness.rs`

### Phase Assets

- Core surface: `src/prompt/renderer.rs`
- Skill surfaces: `skills/procurement-ingestion/fts/`, `skills/manifest-generation/`
- Support surface: `tests/support/skills.rs`
- Test homes entered: `tests/skill_lint.rs`, `tests/skill_canary_harness.rs`

### Gate Criteria

- `UT-SKL-001` through `UT-SKL-008` pass.
- `E2E-CAN-001` through `E2E-CAN-009` pass.
- `E2E-CON-013` passes.
- `cargo check --all-targets` passes.

### Roll Back If

- Production skill directories cannot satisfy the Layer 0 skill standard without L3 promotion or source-health machinery.
- Prompt rendering cannot supply run-manifest instructions from declared skill/job inputs alone.

## Phase 10: Full Live Loop And Reliability

### Prerequisites

- Phase 8 gate passed.
- Phase 9 gate passed.

### Ordering

- Sequential: expand `src/main.rs` to final CLI trigger + startup reconciliation behavior
- Sequential: activate the full live loop through `tests/agent_procurement_ingestion.rs`
- Parallel late in the phase: finish restart/lifecycle handshake coverage in `tests/ipc_handshake.rs` while the live-loop suite proves full-path behavior

### Phase Assets

- Core surface expansion: `src/main.rs`, `src/ipc/handshake.rs`, `src/ipc/descriptor.rs`
- Test home entered: `tests/agent_procurement_ingestion.rs`
- Final test surface expansion: `tests/ipc_handshake.rs`, `tests/session_reaper.rs`

### Gate Criteria

- `E2E-ING-001` through `E2E-ING-026` pass.
- `IT-IPC-006`, `IT-IPC-007`, `IT-IPC-011`, and `IT-IPC-012` pass.
- `E2E-RPR-012` passes.
- Manual gate `E2E-ING-020` passes on the target host.
- Final gate equals the spec exit criteria: first live end-to-end local loop completes, Experiment 4 reaches `>=90%` success across 10 consecutive daemon-managed batches, and orphan count ends at `0`.

### Roll Back If

- The final live loop still depends on any L2+ surface (desktop UI, sync engine, validation enforcement, scheduler, resource arbiter).
- Restart semantics require pretending PTY streams are resumable after daemon crash.

## Completeness Mapping

Primary-home rule: each spec section maps to the phase where its behavior first becomes gateable. Later phases may extend the same file or section without changing the home phase.

| Spec Section | Phase | Notes |
|--------------|-------|-------|
| §1.1 Layer Narrative | 1 | Boundary guard for a thin Rust-only layer |
| §1.2 Capability Delivered | 10 | Final exit criteria only become true at convergence |
| §1.3 System Context | 1 | Defines the allowed L1 runtime surface |
| §1.4 Relationship to Layer 0 | 1 | Hard prerequisite for sequencing |
| §1.5 Component Overview | 1 | Inventory anchor for all file homes |
| §1.6 Ingestion Flow | 10 | Full 14-step loop is the final convergence surface |
| §1.7 Adapter-Mediated Hybrid Execution | 5 | Adapter trait and vendor implementations |
| §1.8 Process-Group Supervision & Session Semantics | 4 | Session + reaper closure gate |
| §1.9 Error Classification | 8 | Error-manifest layer owns the active taxonomy surface |
| §1.10 What’s Stubbed In This Layer | 1 | Scope guard from the first phase |
| §1.11 Things NOT Implemented In Layer 1 | 1 | Scope guard from the first phase |
| §2.1 Rust Crate Structure | 1 | Bootstrap sequencing anchor |
| §2.2 Rust Crate Dependencies | 1 | Manifest/toolchain bootstrap |
| §2.3 Agent Adapter Trait | 5 | Primary adapter contract phase |
| §2.4 Session Lifecycle | 4 | Seven canonical phases become mechanically testable here |
| §2.5 Artifact Store | 6 | Chunk/index/run-manifest contract home |
| §2.6 Job Runner | 7 | Claim/execute/persist/ack home |
| §2.7 SQLite Configuration | 1 | DB startup and commit fence begin here |
| §2.8 Daemon Discovery & IPC Protocol | 2 | Walking-skeleton startup and handshake home |
| §3.1 Tables Introduced In Layer 1 | 1 | Migration-created tables home |
| §3.2 Tables Activated In Layer 1 | 10 | First live writes happen only in final integration |
| §3.3 Stage Tracking Model | 7 | Runner-owned cohort and head semantics |
| §3.4 Source Configuration | 9 | Real skills/canaries activate the seeded source surface |
| §3.5 Job Runner Integration | 7 | Runner-owned lifecycle integration |
| §3.6 Index Design | 1 | Migration/bootstrap surface |
| §3.7 Forward-Compatibility Provisions | 1 | Phase-1 scope guard for dormant fields |
| §4.1 New Skills | 9 | Production skill directories arrive after runner stability |
| §4.2 Run Manifest System | 9 | Prompt + skill + manifest contract convergence |
| §4.3 Error Taxonomy And Error Manifest System | 8 | Error-manifest phase owns this section |
| §4.4 Skill Linting CI Gate | 9 | Real skill directories + lint gate converge here |
| §5.1 Definition Of Done | 10 | Final cumulative gate |
| §5.2 Test File Listing | 1 | Test inventory is sequenced from bootstrap onward |
| §5.3 Test Scenarios | 10 | Edge-case matrix closes only at full-system integration |
| §5.4 Risks | 1 | Planning/risk guard from the first phase |
| §6.1 Open Questions | 1 | Planning guardrail only |
| §6.2 Stubs | 1 | Planning guardrail only |
| §6.3 Forward-Compatibility Provisions | 1 | Planning guardrail only |
| §6.4 Dependencies | 1 | Planning guardrail only |
| §6.5 ADRs Addressed | 1 | Planning guardrail only |

**Coverage:** 39/39 section homes assigned.

## Definition-Of-Done Accumulation

| DoD Item | First phase where it becomes gateable | Notes |
|----------|---------------------------------------|-------|
| 1 | 1 | Static Rust gates start in the bootstrap phase |
| 2 | 6 | Both PTY and contract execution surfaces are present by then |
| 3 | 10 | Manual FTS trigger is part of the final live-loop convergence |
| 4 | 10 | Full live end-to-end local loop is the final integration proof |
| 5 | 10 | CLI summary + local manifest artifact are final-loop outputs |
| 6 | 4 | Process-group supervision and cleanup verdicts are closed by reaper phase |
| 7 | 3 | Artifact capture becomes real with PTY capture |
| 8 | 9 | Prompt + manifest-generation skill + fallback manifest path are all present |
| 9 | 8 | Error manifest system becomes active before final integration |
| 10 | 9 | Skill-lint CI gate begins with real skill directories |
| 11 | 9 | Canary harness begins with real skills and prompt contract |
| 12 | 10 | Experiment 4 is the final release gate |
| 13 | 7 | Runner/stage-cohort rules become gateable before the live-source final phase |

## Dependency Diagram

```text
Phase 1  Workspace + SQLite
   |
   v
Phase 2  Walking skeleton startup
   |
   v
Phase 3  PTY spawn -> capture
   | \
   |  \
   |   +--> Phase 4  Health + reaper
   |
   +------> Phase 5  Adapter trait -> {Codex, Claude}
                  |
                  v
             Phase 6  {Artifacts, Contracts}
                  \      /
                   \    /
                    v  v
            Phase 7  Runner commit/stage tracking
               | \
               |  \
               |   +--> Phase 8  Error manifests
               |
               +------> Phase 9  Prompt + skills + lint + canary
                 \        /
                  \      /
                   v    v
             Phase 10  Full live loop + restart semantics + Experiment 4
```

## Gates Summary

| Phase | Gate | Roll Back If |
|-------|------|--------------|
| 1 | Static Rust gates pass and `IT-IPC-013` proves one-database startup | Phase 1 needs a second metadata DB or cannot chain `0001` then `0003` |
| 2 | Walking skeleton starts, binds, writes descriptor, and handshakes cleanly | Startup readiness depends on later runtime features |
| 3 | PTY spawn/capture works with real helper processes and chunked artifacts | PTY semantics or crash-safe chunking cannot be honored from day 1 |
| 4 | Seven-phase lifecycle and cleanup verdicts are mechanically testable | Cleanup completeness still depends on later layers |
| 5 | Adapter trait and both vendor adapters satisfy probe/mode/parser contracts | Vendor behavior leaks past the adapter boundary |
| 6 | Artifact store and contract validation work independently of live skills | Result validation still belongs inside vendor adapters |
| 7 | Runner owns durable claim/execute/persist/ack and stage cohort semantics | `commit_seq` or `stage_run_id` needs later phases to become testable |
| 8 | Runtime failures create persistent Markdown error manifests | Error context requires L3 validator state or mutates original failures |
| 9 | Real skills lint cleanly, render prompts correctly, and pass canary lanes | Production skills cannot satisfy the L0 directory standard on the L1 runtime |
| 10 | Full live loop, restart rules, and Experiment 4 all pass | Any L2+ dependency is still required for the advertised L1 capability |
