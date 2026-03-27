# L2 Desktop MVP — Implementation Strategy

## Source Documents

| Document | Path | Role |
|----------|------|------|
| Spec | `docs/strategy/layers/layer-2-single-operator-desktop.md` | SOT #1 — scope, contracts, file inventory |
| TDD | `docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-tdd.md` | SOT #2 — verification inventory and gate IDs |
| Bootstrap | `docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-bootstrap.md` | setup and harness prerequisites |
| Cross-layer sequencing | `docs/strategy/layer-strategy.md` §10-§11 | authority boundaries and layer ordering |
| Quality-bar reference | `docs/pipeline/l1-daemon-core/artifacts/l1-daemon-core-strategy.md` | structure and gate-specificity benchmark |

## Strategy Guardrails

- This document defines order, dependency edges, bead grouping, and gate ownership only. The Layer 2 spec and TDD remain the content authorities.
- Primary-home rule: each production file and each TDD test file has exactly one home phase. Later phases may extend the same files without moving their home.
- No phase may home more than `5` production files. If a lane exceeds that, it must split.
- The strategy intentionally stays at `12` phases. That matches the requested `8-12` granularity target while preserving the `<= 5` production-file cap.
- Browser-first UI gates must not land before the QueryService and DaemonBridge contracts are real. The dev bridge exists to expose the same app contract in a browser, not to justify a second read/mutation implementation.
- The current daemon is still Layer-1-shaped:
  - `crates/daemon/src/ipc/handshake.rs` only accepts `hello` / `welcome` plus mutation rejection.
  - `crates/daemon/src/main.rs` still hard-codes the `fts` ingest path.
  These are early L2 risks, not endgame polish.
- `crates/daemon/src/jobs/runner.rs` already owns durable `visible_after_seq` semantics. L2 mutation commands must route through that truth rather than inventing a second ack/fence path.
- Contracts Finder is an independent daemon/skill lane. It must stay off the desktop critical path until the final milestone gate.
- Native Tauri closure is the last convergence point. `tauri.conf.json`, capability allowlists, daemon auto-launch, and native lifecycle verification should not block earlier browser-first UI work.
- Target bead envelope: `25-28` beads.

## Overview

| Phase | Name | Prerequisites | Gate Type | Target Beads |
|------|------|---------------|-----------|--------------|
| 1 | Workspace Roots And Desktop Package Shells | none | mixed static | 2 |
| 2 | Desktop Tooling Configs And Native Baselines | Phase 1 | Bun/tooling | 2 |
| 3 | Daemon Service Mode And Handshake Wire | Phase 1 | Rust-only | 2 |
| 4 | Daemon Control, Jobs, And Session Command Lane | Phase 3 | Rust-only | 2-3 |
| 5 | Shared Data Packages | Phase 1 | Bun package | 2 |
| 6 | Shared UI Surfaces | Phase 1 | Bun package | 2 |
| 7 | Frontend Transport And App Shell Foundation | Phase 2 + Phase 6 | Bun app | 2 |
| 8 | Desktop Rust Bridge And Native Bootstrap | Phase 4 + Phase 7 | mixed Rust/Tauri | 2-3 |
| 9 | Read-Only Operator Pages | Phase 5 + Phase 6 + Phase 7 + Phase 8 | browser-first | 2-3 |
| 10 | Streaming And Session UX Lane | Phase 4 + Phase 7 + Phase 8 | mixed TS/browser | 2-3 |
| 11 | Decisions, Coverage, Staleness, And Operator Validation Package | Phase 5 + Phase 8 + Phase 9 | mixed Rust/browser | 2-3 |
| 12 | Contracts Finder Skill Lane, Hardening, And Milestone Gate | Phase 4 + Phase 9 + Phase 10 + Phase 11 | browser + native + skill + manual | 2 |

## Critical Path And Parallelism

- Longest sequential chain: `1 -> 2 -> 7 -> 8 -> 9 -> 11 -> 12` or `1 -> 3 -> 4 -> 8 -> 9 -> 11 -> 12`.
- Critical-path length: `7` phases, which is under the `> 8` follow-up threshold.
- Three-agent parallel model:
  - Agent A: daemon lane (`3 -> 4`)
  - Agent B: shared package lane (`5 -> 6`)
  - Agent C: workspace/app-shell lane (`1 -> 2 -> 7`)
- After those converge:
  - Agent A can take Phase `8` native bridge work.
  - Agent B can take Phase `9` read-only pages.
  - Agent C can take Phase `10` session UX.
- Contracts Finder remains parallel and off the critical path:
  - once Phase `4` lands, its implementation can proceed independently inside Phase `12` while browser/native hardening closes.
- High-risk items are deliberately early:
  - IPC/service-mode truthfulness: Phases `3` and `4`
  - QueryService commit fence correctness: Phase `8`
  - Live streaming and replay semantics: Phase `10`
  - Decision overlay correctness: Phase `11`

## Expected Bead Envelope

| Phase | Target |
|------|--------|
| 1 | `2` beads |
| 2 | `2` beads |
| 3 | `2` beads |
| 4 | `2-3` beads |
| 5 | `2` beads |
| 6 | `2` beads |
| 7 | `2` beads |
| 8 | `2-3` beads |
| 9 | `2-3` beads |
| 10 | `2-3` beads |
| 11 | `2-3` beads |
| 12 | `2` beads |

**Expected total:** `25-28` beads.

## Implementation Unit Allocation

### Workspace And Native Baseline Surfaces

| Surface | Home Phase | Why it lives there |
|---------|------------|--------------------|
| Root `package.json` | 1 | Bun workspace root must exist before any package or app lane can resolve |
| `bunfig.toml` | 1 | workspace-level Bun behavior belongs in the root bootstrap |
| Root `tsconfig.json` | 1 | TS path and compiler baseline must precede all package/app work |
| `apps/desktop/package.json` | 1 | desktop package shell is needed before Bun workspace install becomes truthful |
| `apps/desktop/src-tauri/Cargo.toml` | 1 | native crate shell exists early, but the root workspace does not adopt it yet |
| `apps/desktop/vite.config.ts` | 2 | frontend bundler config belongs in the tooling phase |
| `apps/desktop/vitest.config.ts` | 2 | unit-test config is tooling, not app-contract logic |
| `apps/desktop/playwright.config.ts` | 2 | browser-first E2E config must exist before any Playwright lane is honest |
| `apps/desktop/src-tauri/tauri.conf.json` | 2 | native-shell baseline config belongs in the tooling/native baseline phase |
| `apps/desktop/src-tauri/capabilities/default.json` | 2 | least-privilege baseline belongs with native scaffolding, even though later phases tighten it |

### Daemon-Side L2 Surfaces

| Surface | Home Phase | Why it lives there |
|---------|------------|--------------------|
| `crates/daemon/src/main.rs` | 3 | service mode and long-lived daemon hosting must land before any desktop contract is believable |
| `crates/daemon/src/ipc/handshake.rs` | 3 | additive wire truth starts here |
| `crates/daemon/src/ipc/descriptor.rs` | 3 | descriptor and service bootstrap stay coupled |
| `crates/daemon/src/ipc/command_router.rs` | 4 | first post-handshake dispatch point |
| `crates/daemon/src/ipc/handlers/control.rs` | 4 | `get_daemon_info` and restart control belong to the early daemon command lane |
| `crates/daemon/src/ipc/handlers/jobs.rs` | 4 | `trigger_ingestion`, `trigger_enrichment`, and `retry_job` land here |
| `crates/daemon/src/ipc/handlers/sessions.rs` | 4 | stop/input/resize must exist before session UX closes |
| `crates/daemon/src/jobs/runner.rs` | 4 | routed daemon commands must reuse the existing durability/fence semantics |
| `crates/daemon/src/ipc/handlers/decisions.rs` | 11 | decision persistence depends on the overlay migration and page action wiring |
| `crates/daemon/src/ipc/handlers/errors.rs` | 11 | resolve/rerun-from-error belongs with operator error actions |
| `migrations/0004_l2_pending_operator_intents.sql` | 11 | decision overlay first becomes active with operator capture |

### Shared Package Surfaces

| Surface | Home Phase | Why it lives there |
|---------|------------|--------------------|
| `packages/manifests/render-opportunity.ts` | 5 | deterministic manifest rendering is a pure package lane |
| `packages/manifests/opportunity.md.ts` | 5 | canonical manifest section layout belongs beside the renderer |
| `packages/intelligence/src/margins/coverage-rollup.ts` | 5 | coverage math is pure TS and should gate before page wiring |
| `packages/intelligence/src/qualification/*` | 5 | goods/service qualification is pure TS and package-local |
| `packages/ui/contract-card-frame.tsx` | 6 | list/detail shell cards should exist before page work consumes them |
| `packages/ui/pipeline-badge.tsx` | 6 | stable pipeline-state display belongs in the shared UI lane |
| `packages/ui/freshness-badge.tsx` | 6 | surface-neutral freshness presentation belongs in shared UI |
| `packages/ui/manifest-viewer.tsx` | 6 | sanitized manifest rendering is a shared UI concern, not a page concern |
| `packages/ui/error-summary-list.tsx` | 6 | sanitized typed error summaries belong with the shared UI surface |
| `packages/ui/index.ts` | 7 | barrel export becomes truthful only after the shared UI files exist |

### Frontend App Surfaces

| Surface | Home Phase | Why it lives there |
|---------|------------|--------------------|
| `apps/desktop/src/lib/transport.ts` | 7 | every frontend transport call must centralize here before page work begins |
| `apps/desktop/vite-plugin-tauri-dev-bridge.ts` | 7 | browser-first transport parity starts in the app-foundation phase |
| `apps/desktop/src/hooks/use-control-plane.ts` | 7 | control-plane state belongs with transport/bootstrap, not with session buffering |
| `apps/desktop/src/App.tsx` | 7 | route ownership and bootstrap gating start with the app shell |
| Root `Cargo.toml` workspace membership update | 8 | the root workspace should not adopt `src-tauri` until real Rust sources compile |
| `apps/desktop/src-tauri/src/main.rs` | 8 | native bootstrap converges only once daemon/service and app contracts exist |
| `apps/desktop/src-tauri/src/query_service.rs` | 8 | read-model truth belongs in the native bridge phase |
| `apps/desktop/src-tauri/src/daemon_bridge.rs` | 8 | daemon RPC proxy belongs beside QueryService |
| `apps/desktop/src/pages/ContractList.tsx` | 9 | first read-only operator browse surface |
| `apps/desktop/src/pages/ContractDetail.tsx` | 9 | first detail read-model surface |
| `apps/desktop/src/pages/Errors.tsx` | 9 | browse-only error queue lands before mutation actions |
| `apps/desktop/src/pages/DaemonStatus.tsx` | 9 | read-only diagnostics land before restart control closure |
| `apps/desktop/src/stores/session-registry.ts` | 10 | live metadata and replay anchors belong to the session lane |
| `apps/desktop/src/stores/session-buffer.ts` | 10 | live/history buffering belongs to the session lane |
| `apps/desktop/src/hooks/use-daemon.ts` | 10 | session hooks are only meaningful once the session stores exist |
| `apps/desktop/src/pages/Sessions.tsx` | 10 | list-level session UX belongs with session stores and controls |
| `apps/desktop/src/pages/SessionViewer.tsx` | 10 | live/history viewer belongs with replay and control semantics |

### Skill And Milestone Surfaces

| Surface | Home Phase | Why it lives there |
|---------|------------|--------------------|
| `docs/testing/l2-operator-weekly-validation.md` | 11 | manual operator signoff package is prepared alongside decision/staleness closure |
| `skills/procurement-ingestion/contracts-finder/SKILL.md` | 12 | Contracts Finder is an independent source skill lane |
| `skills/procurement-ingestion/contracts-finder/checkpoint.schema.json` | 12 | CF skill packaging must mirror FTS structure exactly |
| `skills/procurement-ingestion/contracts-finder/result.schema.json` | 12 | skill result contract belongs with the CF lane |
| `skills/procurement-ingestion/contracts-finder/canary.inputs.jsonl` | 12 | canary inputs are part of the skill packaging standard |
| `skills/procurement-ingestion/contracts-finder/golden.assertions.yaml` | 12 | golden assertions belong with the CF lane |

### Test Home Mapping

| Test Surface | Home Phase | Notes |
|--------------|------------|-------|
| `CI-L2-001`, `CI-L2-002`, `CI-L2-003`, `CI-L2-007` | 8 | first truthful desktop-Rust workspace gates |
| `CI-L2-004` | 7 | first truthful shared-package purity gate once package and app lanes both exist |
| `CI-L2-005` | 10 | first truthful combined package + desktop unit-test gate |
| `CI-L2-006` | 12 | first truthful full-browser-spec discovery gate |
| `packages/manifests/tests/render-opportunity.test.ts` | 5 | manifest package contract home |
| `packages/intelligence/src/qualification/goods-qualification.test.ts` | 5 | qualification package contract home |
| `packages/intelligence/src/margins/coverage-rollup.test.ts` | 5 | coverage math contract home |
| `packages/ui/tests/components.test.tsx` | 6 | home test file for the shared UI lane; later phases extend the same file |
| `apps/desktop/src/lib/transport.test.ts` | 7 | transport shim and dev-bridge contract home |
| `apps/desktop/src-tauri/tests/query_service.rs` | 8 | QueryService contract home |
| `apps/desktop/src-tauri/tests/daemon_bridge.rs` | 8 | DaemonBridge contract home; Phase 10 extends the same file for streaming IDs |
| `apps/desktop/e2e/contracts.spec.ts` | 9 | list/detail browsing home |
| `apps/desktop/e2e/errors.spec.ts` | 9 | browse-only errors home; Phase 11 extends it for actions |
| `apps/desktop/e2e/daemon-status.spec.ts` | 9 | read-only diagnostics home; Phase 12 extends it for restart |
| `apps/desktop/src/stores/session-registry.test.ts` | 10 | session registry home |
| `apps/desktop/src/stores/session-buffer.test.ts` | 10 | session buffer home |
| `apps/desktop/e2e/sessions.spec.ts` | 10 | session UX home |
| `crates/daemon/tests/agent_health_detection.rs` | 10 | live session/error health lane home |
| `apps/desktop/e2e/decisions.spec.ts` | 11 | decision intent and staleness home |
| `apps/desktop/e2e/commit-fence.spec.ts` | 11 | commit-fence mutation visibility home |
| `crates/daemon/tests/agent_supplier_enrichment.rs` | 11 | enrichment and decision-basis-change home |
| `crates/daemon/tests/agent_error_manifest.rs` | 11 | error action and severity derivation home |
| `docs/testing/l2-operator-weekly-validation.md` manual checklist | 11 | milestone-prep home |
| `crates/daemon/tests/agent_contracts_finder_ingestion.rs` | 12 | Contracts Finder source lane home |
| `apps/desktop/e2e/bridge-failures.spec.ts` | 12 | unhappy-path browser hardening home |
| `apps/desktop/e2e/security.spec.ts` | 12 | cross-surface security hardening home |
| `apps/desktop/e2e/tauri-integration.spec.ts` | 12 | native Tauri lifecycle and parity home |

## Coverage Summary

- Desktop files from spec §1.9: `18/18` assigned.
- Shared UI files from spec §2.1: `6/6` assigned.
- TDD test files: `22/22` assigned to a home phase.
- Manual milestone checklist from spec §4.3: `1/1` assigned.

## Phase 1: Workspace Roots And Desktop Package Shells

### Prerequisites

- None.

### Ordering

- Sequential: root `package.json` -> `bunfig.toml` / root `tsconfig.json` -> `apps/desktop/package.json` -> `apps/desktop/src-tauri/Cargo.toml`.

### Phase Assets

- Root `package.json`
- `bunfig.toml`
- Root `tsconfig.json`
- `apps/desktop/package.json`
- `apps/desktop/src-tauri/Cargo.toml`

### Bead Grouping Guidance

- Target: `2` beads.
- Batch A: root Bun/TS workspace files.
- Batch B: desktop package shell plus native crate shell.

### Gate Criteria

- Gate type: `mixed static`.
- `bun install` resolves the Bun workspace with `apps/desktop` present.
- `cargo check --all-targets` and `cargo fmt --check` remain green for the daemon-only workspace.
- No root Cargo workspace membership update happens yet; `src-tauri` is present as a shell, not a live member.

### Roll Back If

- The workspace requires a second repo or a non-Bun package manager for the desktop shell.
- Desktop package shells cannot be introduced without breaking root install resolution.

## Phase 2: Desktop Tooling Configs And Native Baselines

### Prerequisites

- Phase 1 gate passed.

### Ordering

- Sequential: `vite.config.ts` -> `vitest.config.ts` -> `playwright.config.ts` -> `tauri.conf.json` -> `capabilities/default.json`.

### Phase Assets

- `apps/desktop/vite.config.ts`
- `apps/desktop/vitest.config.ts`
- `apps/desktop/playwright.config.ts`
- `apps/desktop/src-tauri/tauri.conf.json`
- `apps/desktop/src-tauri/capabilities/default.json`

### Bead Grouping Guidance

- Target: `2` beads.
- Batch A: Vite + Vitest + Playwright configs.
- Batch B: native Tauri baseline config and capability baseline.

### Gate Criteria

- Gate type: `Bun/tooling`.
- Tooling and native baseline configs parse and resolve without app-level contract assumptions.
- Relevant command lane:
  - `bunx vitest --config apps/desktop/vitest.config.ts --run --passWithNoTests`
  - `bunx playwright test --config apps/desktop/playwright.config.ts --list`
- This phase is scaffolding only; the numbered CI gates do not become truthful until later convergence phases.

### Roll Back If

- Tooling config requires page-level contracts or native runtime behaviors to parse.
- Native baseline config already assumes Layer 3+ functionality.

## Phase 3: Daemon Service Mode And Handshake Wire

### Prerequisites

- Phase 1 gate passed.

### Ordering

- Sequential: `crates/daemon/src/main.rs` service loop -> additive wire changes in `crates/daemon/src/ipc/handshake.rs` -> descriptor/bootstrap alignment in `crates/daemon/src/ipc/descriptor.rs`.

### Phase Assets

- `crates/daemon/src/main.rs`
- `crates/daemon/src/ipc/handshake.rs`
- `crates/daemon/src/ipc/descriptor.rs`

### Bead Grouping Guidance

- Target: `2` beads.
- Batch A: persistent daemon hosting path in `main.rs`.
- Batch B: additive handshake and descriptor alignment.

### Gate Criteria

- Gate type: `Rust-only`.
- The additive `hello` / `welcome` exchange stays intact while repeated authenticated connections become possible.
- Existing daemon bootstrap and descriptor behavior remain green.
- Relevant command lane:
  - `cargo test -p daemon --test ipc_handshake`
  - `cargo check --all-targets`
  - `cargo clippy --all-targets -- -D warnings`
  - `cargo fmt --check`

### Roll Back If

- Service mode requires replacing the Layer 1 wire shape rather than extending it.
- Descriptor/startup behavior becomes coupled to desktop-only code.

## Phase 4: Daemon Control, Jobs, And Session Command Lane

### Prerequisites

- Phase 3 gate passed.

### Ordering

- Sequential: router -> control/jobs handlers -> session handler -> runner reuse and source-command generalization.

### Phase Assets

- `crates/daemon/src/ipc/command_router.rs`
- `crates/daemon/src/ipc/handlers/control.rs`
- `crates/daemon/src/ipc/handlers/jobs.rs`
- `crates/daemon/src/ipc/handlers/sessions.rs`
- `crates/daemon/src/jobs/runner.rs`

### Bead Grouping Guidance

- Target: `2-3` beads.
- Batch A: router + control handler.
- Batch B: jobs handler plus `JobRunner` reuse.
- Batch C: session control handler.

### Gate Criteria

- Gate type: `Rust-only`.
- `get_daemon_info`, `trigger_ingestion`, `trigger_enrichment`, `retry_job`, `restart_daemon`, `stop_session`, `send_session_input`, and `resize_session` all dispatch through router-owned handlers.
- Lost-ack, `visible_after_seq`, and idempotency behavior continue to reuse `JobRunner` semantics rather than a second mutation path.
- The daemon is no longer truthfully “FTS-only” at the command-routing layer even though the Contracts Finder skill itself is still deferred.
- Relevant command lane:
  - `cargo test -p daemon --test agent_contract_session`
  - `cargo check --all-targets`
  - `cargo clippy --all-targets -- -D warnings`
  - `cargo fmt --check`

### Roll Back If

- Command routing bypasses `JobRunner` durability and invents a separate ack/fence mechanism.
- The jobs path still depends on the old `--ingest fts` special case instead of a command/source abstraction.

## Phase 5: Shared Data Packages

### Prerequisites

- Phase 1 gate passed.

### Ordering

- Parallel: `packages/manifests` and `packages/intelligence`.
- Sequential late in phase: package-local fixtures and tests after both package roots are stable.

### Phase Assets

- `packages/manifests/render-opportunity.ts`
- `packages/manifests/opportunity.md.ts`
- `packages/intelligence/src/margins/coverage-rollup.ts`
- `packages/intelligence/src/qualification/*`

### Bead Grouping Guidance

- Target: `2` beads.
- Batch A: manifest renderer and template.
- Batch B: coverage rollup and qualification rules.

### Gate Criteria

- Gate type: `Bun package`.
- `TS-MAN-001` through `TS-MAN-005` pass.
- `TS-COV-001` through `TS-COV-006` pass.
- `TS-QUAL-001` through `TS-QUAL-005` pass.
- Relevant command lane:
  - `bun run build`
  - `bun run lint`
  - `bun run test`

### Roll Back If

- Shared data packages need desktop transport, daemon imports, or page hooks to become testable.
- Coverage or qualification logic depends on UI-local heuristics instead of typed package contracts.

## Phase 6: Shared UI Surfaces

### Prerequisites

- Phase 1 gate passed.

### Ordering

- Sequential: `ContractCardFrame` -> `PipelineBadge` / `FreshnessBadge` -> `ManifestViewer` / `ErrorSummaryList`.

### Phase Assets

- `packages/ui/contract-card-frame.tsx`
- `packages/ui/pipeline-badge.tsx`
- `packages/ui/freshness-badge.tsx`
- `packages/ui/manifest-viewer.tsx`
- `packages/ui/error-summary-list.tsx`

### Bead Grouping Guidance

- Target: `2` beads.
- Batch A: list/detail shared cards and badges.
- Batch B: sanitized manifest/error renderers.

### Gate Criteria

- Gate type: `Bun package`.
- `packages/ui/tests/components.test.tsx` passes:
  - `TS-UI-002`
  - `TS-UI-003`
  - `TS-UI-004`
  - `TS-UI-005`
  - `TS-UI-006`
- Relevant command lane:
  - `bun run build`
  - `bun run test`

### Roll Back If

- Shared UI needs mutation hooks, QueryService types, or daemon imports to render.
- Security or formatting rules leak page-specific logic into the components.

## Phase 7: Frontend Transport And App Shell Foundation

### Prerequisites

- Phase 2 gate passed.
- Phase 6 gate passed.

### Ordering

- Sequential: shared barrel closure -> transport shim -> Vite dev bridge -> control-plane hook -> route shell.

### Phase Assets

- `packages/ui/index.ts`
- `apps/desktop/src/lib/transport.ts`
- `apps/desktop/vite-plugin-tauri-dev-bridge.ts`
- `apps/desktop/src/hooks/use-control-plane.ts`
- `apps/desktop/src/App.tsx`

### Bead Grouping Guidance

- Target: `2` beads.
- Batch A: `packages/ui/index.ts` plus transport shim/dev bridge.
- Batch B: control-plane hook plus `App.tsx`.

### Gate Criteria

- Gate type: `Bun app`.
- `TS-TRN-001` through `TS-TRN-004` pass.
- `packages/ui/tests/components.test.tsx` passes:
  - `TS-UI-001`
  - `TS-UI-007`
  - `TS-UI-008`
- `CI-L2-004` is now meaningful and passes.
- Relevant command lane:
  - `bun run build`
  - `bun run lint`
  - `bun run test`

### Roll Back If

- Frontend callers import Tauri APIs directly instead of going through `src/lib/transport.ts`.
- The dev bridge creates a second app-level read/mutation contract instead of exposing the same contract in browser form.

## Phase 8: Desktop Rust Bridge And Native Bootstrap

### Prerequisites

- Phase 4 gate passed.
- Phase 7 gate passed.

### Ordering

- Sequential: root `Cargo.toml` workspace update -> `src-tauri/src/main.rs` -> `query_service.rs` -> `daemon_bridge.rs`.

### Phase Assets

- Root `Cargo.toml` workspace membership update
- `apps/desktop/src-tauri/src/main.rs`
- `apps/desktop/src-tauri/src/query_service.rs`
- `apps/desktop/src-tauri/src/daemon_bridge.rs`

### Bead Grouping Guidance

- Target: `2-3` beads.
- Batch A: workspace update + native bootstrap.
- Batch B: `query_service.rs`.
- Batch C: `daemon_bridge.rs`.

### Gate Criteria

- Gate type: `mixed Rust/Tauri`.
- `RS-QS-001` through `RS-QS-012` pass.
- `RS-DBR-001` through `RS-DBR-007` pass.
- `CI-L2-001`, `CI-L2-002`, `CI-L2-003`, and `CI-L2-007` pass.
- Relevant command lane:
  - `cargo test --test query_service`
  - `cargo test --test daemon_bridge`
  - `cargo check --all-targets`
  - `cargo clippy --all-targets -- -D warnings`
  - `cargo fmt --check`

### Roll Back If

- QueryService performs writes, relies on `PRAGMA data_version` as the user-visible fence, or exposes generic SQL.
- DaemonBridge starts absorbing daemon authority instead of remaining a thin RPC proxy.

## Phase 9: Read-Only Operator Pages

### Prerequisites

- Phase 5 gate passed.
- Phase 6 gate passed.
- Phase 7 gate passed.
- Phase 8 gate passed.

### Ordering

- Sequential: contract list -> contract detail -> errors browse surface -> daemon diagnostics browse surface.

### Phase Assets

- `apps/desktop/src/pages/ContractList.tsx`
- `apps/desktop/src/pages/ContractDetail.tsx`
- `apps/desktop/src/pages/Errors.tsx`
- `apps/desktop/src/pages/DaemonStatus.tsx`

### Bead Grouping Guidance

- Target: `2-3` beads.
- Batch A: list + detail.
- Batch B: errors + daemon status.

### Gate Criteria

- Gate type: `browser-first`.
- `PW-CON-001` through `PW-CON-010` pass.
- `PW-ERR-001` through `PW-ERR-004` pass.
- `PW-DGN-001`, `PW-DGN-002`, and `PW-DGN-004` pass.
- Relevant command lane:
  - `bunx playwright test e2e/contracts.spec.ts e2e/errors.spec.ts e2e/daemon-status.spec.ts`

### Roll Back If

- Read-only pages still depend on fake query paths or page-local fetch fallbacks.
- The diagnostics page is only useful through native-only affordances rather than the real app contract.

## Phase 10: Streaming And Session UX Lane

### Prerequisites

- Phase 4 gate passed.
- Phase 7 gate passed.
- Phase 8 gate passed.

### Ordering

- Sequential: session registry -> session buffer -> `use-daemon.ts` -> `Sessions.tsx` -> `SessionViewer.tsx`.

### Phase Assets

- `apps/desktop/src/stores/session-registry.ts`
- `apps/desktop/src/stores/session-buffer.ts`
- `apps/desktop/src/hooks/use-daemon.ts`
- `apps/desktop/src/pages/Sessions.tsx`
- `apps/desktop/src/pages/SessionViewer.tsx`

### Bead Grouping Guidance

- Target: `2-3` beads.
- Batch A: registry + buffer.
- Batch B: hook + list page.
- Batch C: viewer + replay/control closure.

### Gate Criteria

- Gate type: `mixed TS/browser`.
- `TS-REG-001` through `TS-REG-005` pass.
- `TS-BUF-001` through `TS-BUF-004` pass.
- `PW-SES-001` through `PW-SES-012` pass.
- `RS-DBR-008` through `RS-DBR-010` pass.
- `AG-HLT-001` through `AG-HLT-005` pass.
- `CI-L2-005` is now meaningful and passes.
- Relevant command lane:
  - `bun run test`
  - `bunx playwright test e2e/sessions.spec.ts`
  - `cargo test -p daemon --test agent_health_detection`
  - `cargo test --test daemon_bridge`

### Roll Back If

- Replay or gap handling requires abandoning the typed `SessionFrame` / `BridgeEvent` contract.
- Session controls bypass the routed daemon command path.

## Phase 11: Decisions, Coverage, Staleness, And Operator Validation Package

### Prerequisites

- Phase 5 gate passed.
- Phase 8 gate passed.
- Phase 9 gate passed.

### Ordering

- Sequential: `pending_operator_intents` migration -> decision handler -> error-action handler -> page/action wiring -> operator validation package.

### Phase Assets

- `crates/daemon/src/ipc/handlers/decisions.rs`
- `crates/daemon/src/ipc/handlers/errors.rs`
- `migrations/0004_l2_pending_operator_intents.sql`
- `docs/testing/l2-operator-weekly-validation.md`

### Bead Grouping Guidance

- Target: `2-3` beads.
- Batch A: migration + decision handler.
- Batch B: error-action handler + page action wiring.
- Batch C: operator validation checklist package.

### Gate Criteria

- Gate type: `mixed Rust/browser`.
- `PW-DEC-001` through `PW-DEC-008` pass.
- `PW-FEN-001` through `PW-FEN-007` pass.
- `PW-ERR-005` and `PW-ERR-006` pass.
- `AG-ENR-001` through `AG-ENR-004` pass.
- `AG-ERR-001` through `AG-ERR-004` pass.
- `docs/testing/l2-operator-weekly-validation.md` exists and is aligned with the current phase graph and milestone gate.
- Relevant command lane:
  - `bunx playwright test e2e/decisions.spec.ts e2e/commit-fence.spec.ts e2e/errors.spec.ts`
  - `cargo test -p daemon --test agent_supplier_enrichment`
  - `cargo test -p daemon --test agent_error_manifest`

### Roll Back If

- Decision capture still needs Layer 4 CAS or remote authority to be meaningful.
- Coverage/staleness rules depend on UI-local hashes or heuristics instead of daemon-authored values plus package math.

## Phase 12: Contracts Finder Skill Lane, Hardening, And Milestone Gate

### Prerequisites

- Phase 4 gate passed.
- Phase 9 gate passed.
- Phase 10 gate passed.
- Phase 11 gate passed.

### Ordering

- Parallel early batch: Contracts Finder skill package can advance as soon as the Phase `4` daemon command lane is stable.
- Sequential hardening batch: bridge-failure flows -> security hardening -> native Tauri parity -> cumulative milestone gate.

### Phase Assets

- `skills/procurement-ingestion/contracts-finder/SKILL.md`
- `skills/procurement-ingestion/contracts-finder/checkpoint.schema.json`
- `skills/procurement-ingestion/contracts-finder/result.schema.json`
- `skills/procurement-ingestion/contracts-finder/canary.inputs.jsonl`
- `skills/procurement-ingestion/contracts-finder/golden.assertions.yaml`

### Bead Grouping Guidance

- Target: `3` beads.
- Batch A: Contracts Finder skill package and daemon test lane.
- Batch B: bridge-failure and browser/security hardening.
- Batch C: native Tauri parity and cumulative milestone closeout.

### Gate Criteria

- Gate type: `browser + native + skill + manual`.
- `AG-CF-001` through `AG-CF-006` pass.
- `PW-BRD-001` through `PW-BRD-006` pass.
- `PW-SEC-001` through `PW-SEC-008` pass.
- `PW-DGN-003` passes.
- `TAU-INT-001` through `TAU-INT-012` pass.
- `CI-L2-006` is now meaningful and passes.
- Full cumulative quality gate passes:
  - `cargo check --all-targets`
  - `cargo clippy --all-targets -- -D warnings`
  - `cargo fmt --check`
  - `bun run build`
  - `bun run lint`
  - `bun run test`
  - `bunx playwright test`
- The Contracts Finder skill mirrors the current `skills/procurement-ingestion/fts/` five-file skeleton exactly.
- The weekly operator checklist is ready for the 5-7 day milestone run.
- Relevant command lane:
  - `cargo test -p daemon --test agent_contracts_finder_ingestion`
  - `bunx playwright test e2e/bridge-failures.spec.ts e2e/security.spec.ts e2e/tauri-integration.spec.ts`
  - `bunx playwright test`

### Roll Back If

- Contracts Finder requires any desktop, Tauri, or web code to run.
- Native desktop closure still depends on the dev bridge, loopback HTTP, or any Layer 3 scheduler/resource-arbiter behavior.
- The milestone proof still needs a missing later-layer capability to make the layer honest.

## Completeness Mapping

Primary-home rule: each key spec/TDD anchor maps to the phase where it first becomes truthfully gateable.

| Anchor | Home Phase | Notes |
|--------|------------|-------|
| Spec §1.5 DaemonBridge Interface | 3, 4, 8, 10, 11 | wire first, routing next, native bridge next, streaming next, decision/error actions last |
| Spec §1.8 Dev-Mode Browser Transport | 7 | dev bridge and transport shim start here; later phases consume the contract |
| Spec §1.9 Desktop File Listing | 1, 2, 7, 8, 9, 10 | all `18` files routed with no duplicates |
| Spec §1.10 React State Management | 7, 10 | control-plane first, session stores later |
| Spec §2.1 Shared UI Components | 6, 7 | shared components first, barrel export when package surface closes |
| Spec §2.2 Manifest Rendering | 5 | deterministic renderer and template are a pure package lane |
| Spec §3.2 `pending_operator_intents` | 11 | first truthful activation point |
| Spec §3.4 Coverage Rollup Module | 5, 11 | pure math first, UI/mutation convergence later |
| TDD §6 Static CI Gates | 7, 8, 10, 12 | distributed by first truthful execution point |
| TDD §7 Browser E2E catalog | 9, 10, 11, 12 | read-only first, sessions next, mutation/fence next, hardening last |
| TDD §7 Rust integration catalog | 8, 10 | QueryService/bridge first, streaming bridge IDs later |
| TDD §7 Agent E2E catalog | 10, 11, 12 | health first, enrichment/error actions next, CF last |
| Spec §4.3 manual weekly validation asset | 11 | prepared before final milestone gate |

## Definition-Of-Done Accumulation

| DoD Item | First Phase Where It Becomes Gateable | Notes |
|---------|---------------------------------------|-------|
| 1. FTS ingestion skill still works | 4 | daemon command lane must not regress existing source execution |
| 2. Contracts Finder ingestion skill | 12 | independent skill lane |
| 3. Notice normalization visible to desktop | 9 | read-only pages consume real QueryService data |
| 4. Goods/services classifier | 5 | pure package lane |
| 5. Desktop control panel bootstrap | 8 | first truthful native bootstrap closure |
| 6. Notice list view | 9 | first browser-read-only page lane |
| 7. Opportunity detail view | 9 | first browser-read-only detail lane |
| 8. Decision capture | 11 | overlay migration + handler + page wiring converge here |
| 9. Session monitoring | 10 | stores, bridge streaming, and session UI converge here |
| 10. Coverage metrics display | 11 | pure math plus page wiring converge here |
| 11. Decision staleness | 11 | depends on decision overlay plus basis-hash change semantics |
| 12. Single operator daily value loop | 12 | milestone/manual gate only |
| 13. Desktop auto-launches daemon | 12 | native-shell-only |
| 14. Stop/input/retry controls | 10 | session control lane |
| 15. Commit fence consistency | 11 | mutation + fence + page semantics converge here |
| 16. Daemon restart detection | 12 | native lifecycle only |
| 17. Errors page + resolve flow | 11 | browse surface starts in 9, action closure lands in 11 |
| 18. Daemon status + restart control | 12 | browse-only diagnostics land in 9; full item closes natively in 12 |
| 19. Security boundary | 12 | adversarial browser hardening gate |
| 20. Stream reconnect and gap recovery | 10 | core behavior closes in the session lane; hardening happens in 12 |
| 21. Query/mutation contract robustness | 8 -> 12 | Rust contract first, browser/native failure closure last |

## Dependency Diagram

```text
Phase 1  Workspace roots + desktop package shells
   | \  \
   |  \  +--> Phase 5  Shared data packages
   |   +----> Phase 6  Shared UI surfaces
   |
   +------> Phase 2  Desktop tooling + native baselines
                 \          /
                  \        /
                   v      v
                Phase 7  Frontend transport + app shell foundation

Phase 1
   |
   v
Phase 3  Daemon service mode + handshake wire
   |
   v
Phase 4  Daemon control/jobs/session command lane
   | \
   |  +--> Phase 10  Streaming + session UX lane
   |
   +------> Phase 8  Desktop Rust bridge + native bootstrap
                 | \
                 |  \
                 |   +--> Phase 9   Read-only operator pages
                 |                  |
                 |                  v
                 |             Phase 11  Decisions + coverage + staleness + operator validation package
                 |
                 +--> Phase 10  Streaming + session UX lane

Phase 4 + Phase 9 + Phase 10 + Phase 11
                       |
                       v
              Phase 12  Contracts Finder skill lane + hardening + milestone gate
```

## Gates Summary

| Phase | Gate | Roll Back If |
|------|------|--------------|
| 1 | Bun workspace install resolves and daemon-only cargo checks still pass | workspace bootstrap requires another repo or another package manager |
| 2 | desktop tooling and native baseline configs parse without app-contract assumptions | Vite/Vitest/Playwright/Tauri config already depends on unbuilt app/runtime behavior |
| 3 | daemon can host repeated additive `hello` / `welcome` connections without descriptor regression | service mode breaks Layer 1 wire or descriptor semantics |
| 4 | routed daemon commands reuse `JobRunner` durability/fence behavior | command lane invents a second mutation or source-routing path |
| 5 | manifest + coverage + qualification packages build and test as pure TS packages | package logic leaks desktop/runtime concerns |
| 6 | shared UI components render as pure presentation and stay sanitized | components need backend imports or page-local security logic |
| 7 | transport shim, dev bridge, shared UI barrel, and app shell form the only frontend access path | app code imports Tauri directly or the dev bridge forks the app contract |
| 8 | native bootstrap, QueryService, and DaemonBridge satisfy Rust contract tests | desktop Rust absorbs daemon authority or violates read-only/fence rules |
| 9 | list/detail/errors/status pages are browser-testable against real app contracts | pages still rely on fake data paths or native-only affordances |
| 10 | session stores, replay, gap handling, and controls are end-to-end testable | typed streaming contract cannot survive real replay/gap/control flows |
| 11 | decisions/errors/coverage/staleness close against real fences and overlay DDL | overlay still depends on Layer 4 CAS or UI-local heuristics |
| 12 | CF skill, browser hardening, native parity, and milestone package all pass | the layer still depends on dev-only transport or Layer 3+ capabilities to be real |
