# L2 Desktop MVP — Bead Bulk Import

## l2-01a: Bun/TS Workspace Root Files

- type: task
- priority: P0
- labels: phase-1,typescript,workspace
- parent: testproject1-dr6

### Description

## SOURCE OF TRUTH WARNING

This bead is a quick-reference derived from:
- **Spec**: `docs/strategy/layers/layer-2-single-operator-desktop.md` — §1.9 (PRIMARY — read this first)
- **TDD**: `docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-tdd.md` — CI-L2-004, CI-L2-005 (PRIMARY — test definitions here)
- **Strategy**: `docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-strategy.md` — Phase 1 (ordering context only)

Before implementing, you MUST read the referenced spec and TDD sections directly.

## Context

Create the Bun workspace root that all TypeScript packages and apps depend on. This is the absolute first bead — nothing else can start until the workspace resolves.

## Language & Quality Gates

- **Language:** `typescript`
- **Quality gate commands:**
  - `bun install`

## Files to Create

- `package.json` — Bun workspace root with `workspaces` array including `packages/*` and `apps/*`
- `bunfig.toml` — Workspace-level Bun behavior configuration
- `tsconfig.json` — Root TS path aliases and compiler baseline for all packages/apps

## Spec Requirements (from §1.9, §2.3)

The root workspace must enable all downstream package and app lanes to resolve. Strategy §Phase 1 specifies:
- Root `package.json` must exist before any package or app lane can resolve
- `bunfig.toml` defines workspace-level Bun behavior
- Root `tsconfig.json` sets TS path and compiler baseline before all package/app work

## Tests to Pass

- **CI-L2-004**: `bun install` resolves all declared workspaces; `bun run lint` exits 0; no TypeScript surface imports desktop-only code into shared packages
- **CI-L2-005**: `bun run build` and `bun run test` exit 0; package tests and desktop unit tests are runnable from the same workspace root

## Acceptance Criteria

1. `bun install` resolves the Bun workspace with all declared workspace members present
2. Root `tsconfig.json` provides path aliases usable by downstream packages
3. No workspace member requires a second package manager
4. No dependencies on any other bead — this is the root

## Dependencies

- **Blocked by:** none (root bead)
- **Blocks:** l2-01b, l2-02a, l2-03a, l2-05a, l2-05b, l2-06a, l2-06b
- **Phase:** 1 — Workspace Roots And Desktop Package Shells

---

## l2-01b: Desktop Package Shell And Native Crate Shell

- type: task
- priority: P0
- labels: phase-1,mixed,workspace
- parent: testproject1-dr6

### Description

## SOURCE OF TRUTH WARNING

This bead is a quick-reference derived from:
- **Spec**: `docs/strategy/layers/layer-2-single-operator-desktop.md` — §1.3, §1.9 (PRIMARY — read this first)
- **TDD**: `docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-tdd.md` — CI-L2-004 (PRIMARY)
- **Strategy**: `docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-strategy.md` — Phase 1 (ordering context only)

## Context

Create the desktop app package shell and the native Tauri Rust crate shell. These are empty scaffolds that make `bun install` truthful with `apps/desktop` present, and provide the `Cargo.toml` for the native crate (not yet adopted into the root workspace).

## Language & Quality Gates

- **Language:** `mixed` (TypeScript + Rust shell)
- **Quality gate commands:**
  - `bun install`
  - `cargo check --all-targets`
  - `cargo fmt --check`

## Files to Create

- `apps/desktop/package.json` — Desktop package shell needed for Bun workspace install
- `apps/desktop/src-tauri/Cargo.toml` — Native crate shell; NOT adopted into root Cargo workspace yet

## Spec Requirements (from §1.3)

> Tauri v2 with a **separate background daemon** — the desktop app is NOT the daemon. The daemon must survive UI close, restart independently, and have its own lifecycle.

**L2-FR-BOOT.1 — The Desktop Shell Uses a Separate Background Daemon**
- AC1: The Tauri shell remains a client of a separately managed daemon process rather than embedding daemon execution inside the window lifecycle.
- AC2: Closing the desktop UI does not implicitly terminate a healthy daemon instance.
- AC3: The daemon can restart independently of the desktop and present a new `instanceId` on the next handshake.

## Tests to Pass

- **CI-L2-004**: `bun install` resolves all declared workspaces including `apps/desktop`

## Acceptance Criteria

1. `bun install` resolves the Bun workspace with `apps/desktop` present
2. `cargo check --all-targets` and `cargo fmt --check` remain green for the daemon-only workspace
3. No root Cargo workspace membership update happens yet — `src-tauri` is a shell, not a live member
4. Desktop package shell does not embed daemon code

## Dependencies

- **Blocked by:** l2-01a
- **Blocks:** l2-02a, l2-02b
- **Phase:** 1 — Workspace Roots And Desktop Package Shells

---

## l2-02a: Frontend Tooling Configs

- type: task
- priority: P0
- labels: phase-2,typescript,tooling
- parent: testproject1-dr6

### Description

## SOURCE OF TRUTH WARNING

This bead is a quick-reference derived from:
- **Spec**: `docs/strategy/layers/layer-2-single-operator-desktop.md` — §1.8, §4.1 (PRIMARY — read this first)
- **TDD**: `docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-tdd.md` — CI-L2-006 (PRIMARY)
- **Strategy**: `docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-strategy.md` — Phase 2 (ordering context only)

## Context

Create the frontend tooling configurations: Vite bundler, Vitest unit testing, and Playwright browser E2E configs. These are scaffolding only — they must parse and resolve without app-level contract assumptions.

## Language & Quality Gates

- **Language:** `typescript`
- **Quality gate commands:**
  - `bunx vitest --config apps/desktop/vitest.config.ts --run --passWithNoTests`
  - `bunx playwright test --config apps/desktop/playwright.config.ts --list`

## Files to Create

- `apps/desktop/vite.config.ts` — Frontend bundler config
- `apps/desktop/vitest.config.ts` — Unit-test config
- `apps/desktop/playwright.config.ts` — Browser-first E2E config

## Spec Requirements (from §1.8, §4.1)

> Tauri's `invoke()` and `Channel<T>` only work inside the native webview. This makes browser-based testing tools unable to interact with the real app.
> A dev-only Vite middleware that bridges the gap so the React SPA is fully functional at `http://localhost:5173` in any browser. This is the **primary E2E testing surface** for all UI functionality.

Playwright config must target `localhost:5173` as the primary E2E surface (spec §4.1).

## Tests to Pass

- **CI-L2-006**: `bunx playwright test --list` resolves the Layer 2 browser specs, uses the configured dev server, and exposes no production-only transport assumptions

## Acceptance Criteria

1. Vite config parses and resolves without app-level contract assumptions
2. Vitest config runs with `--passWithNoTests`
3. Playwright config lists specs without requiring a running Tauri shell
4. No tooling config assumes page-level contracts or native runtime behaviors

## Dependencies

- **Blocked by:** l2-01b
- **Blocks:** l2-07a
- **Phase:** 2 — Desktop Tooling Configs And Native Baselines

---

## l2-02b: Native Tauri Baseline Config

- type: task
- priority: P0
- labels: phase-2,mixed,tooling
- parent: testproject1-dr6

### Description

## SOURCE OF TRUTH WARNING

This bead is a quick-reference derived from:
- **Spec**: `docs/strategy/layers/layer-2-single-operator-desktop.md` — §1.9, §5.3 (PRIMARY — read this first)
- **TDD**: `docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-tdd.md` — CI-L2-007 (PRIMARY)
- **Strategy**: `docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-strategy.md` — Phase 2 (ordering context only)

## Context

Create the native Tauri baseline configuration: `tauri.conf.json` for sidecar and desktop build settings, and `capabilities/default.json` for least-privilege shell/sidecar access.

## Language & Quality Gates

- **Language:** `mixed` (JSON config + Rust context)
- **Quality gate commands:**
  - `cargo check --all-targets`

## Files to Create

- `apps/desktop/src-tauri/tauri.conf.json` — Sidecar and desktop build configuration
- `apps/desktop/src-tauri/capabilities/default.json` — Allowed shell/sidecar commands (least-privilege)

## Spec Requirements

**L2-FR-SEC.2 — Desktop Capability Exposure Is Least-Privilege**
- AC1: Desktop capability configuration admits only the shell/sidecar access required by the L2 desktop surface.
- AC2: Dev-only transport exposure remains loopback-only and excluded from production builds.
- AC3: Shared packages do not acquire elevated access indirectly by importing desktop-only bridge or daemon modules.

## Tests to Pass

- **CI-L2-007**: Every `apps/desktop/src-tauri/src/*.rs` and `apps/desktop/src-tauri/tests/*.rs` file starts from a `#![forbid(unsafe_code)]` policy and contains no `unsafe` blocks

## Acceptance Criteria

1. `tauri.conf.json` parses without assuming Layer 3+ functionality
2. `capabilities/default.json` provides least-privilege baseline
3. Native baseline config does not require page-level contracts or native runtime behaviors to parse

## Dependencies

- **Blocked by:** l2-01b
- **Blocks:** l2-07a
- **Phase:** 2 — Desktop Tooling Configs And Native Baselines

---

## l2-03a: Persistent Daemon Hosting Path

- type: task
- priority: P0
- labels: phase-3,rust,daemon
- parent: testproject1-dr6

### Description

## SOURCE OF TRUTH WARNING

This bead is a quick-reference derived from:
- **Spec**: `docs/strategy/layers/layer-2-single-operator-desktop.md` — §1.3, §1.5 (PRIMARY — read this first)
- **TDD**: `docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-tdd.md` — CI-L2-001 (PRIMARY)
- **Strategy**: `docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-strategy.md` — Phase 3 (ordering context only)

## Context

Extend `crates/daemon/src/main.rs` to support a long-lived service mode that can accept repeated IPC connections from the desktop. Currently the daemon is hard-coded for single-run `--ingest fts`. This bead adds a persistent hosting path so the daemon can serve as a background service.

## Language & Quality Gates

- **Language:** `rust`
- **Quality gate commands:**
  - `cargo check --all-targets`
  - `cargo clippy --all-targets -- -D warnings`
  - `cargo fmt --check`

## Files to Modify

- `crates/daemon/src/main.rs` — Add persistent service loop for long-lived daemon hosting

## Spec Requirements (from §1.3)

> The daemon must survive UI close, restart independently, and have its own lifecycle.

**L2-FR-BOOT.1 — The Desktop Shell Uses a Separate Background Daemon**
- AC1: The Tauri shell remains a client of a separately managed daemon process rather than embedding daemon execution inside the window lifecycle.
- AC2: Closing the desktop UI does not implicitly terminate a healthy daemon instance.
- AC3: The daemon can restart independently of the desktop and present a new `instanceId` on the next handshake.

## Tests to Pass

- **CI-L2-001**: `cargo check --all-targets` exits 0 for the root workspace and the future desktop-side Rust crate; no Layer 2 Rust target breaks the existing daemon build

## Acceptance Criteria

1. Daemon can run as a persistent service accepting repeated IPC connections
2. Existing bootstrap, descriptor, and signal handling behavior remains green
3. The daemon is no longer limited to single-run `--ingest fts` mode

## Dependencies

- **Blocked by:** l2-01a
- **Blocks:** l2-03b
- **Phase:** 3 — Daemon Service Mode And Handshake Wire

---

## l2-03b: Additive Handshake And Descriptor Alignment

- type: task
- priority: P0
- labels: phase-3,rust,daemon,ipc
- parent: testproject1-dr6

### Description

## SOURCE OF TRUTH WARNING

This bead is a quick-reference derived from:
- **Spec**: `docs/strategy/layers/layer-2-single-operator-desktop.md` — §1.5 (PRIMARY — read this first)
- **TDD**: `docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-tdd.md` — RS-DBR-001, RS-DBR-009 (PRIMARY)
- **Strategy**: `docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-strategy.md` — Phase 3 (ordering context only)

## Context

Extend `handshake.rs` and `descriptor.rs` for repeated authenticated connections. The L1 `hello`/`welcome` exchange stays intact while the socket becomes reusable. This is additive — no wire shape replacement.

## Language & Quality Gates

- **Language:** `rust`
- **Quality gate commands:**
  - `cargo test -p daemon --test ipc_handshake`
  - `cargo check --all-targets`
  - `cargo clippy --all-targets -- -D warnings`
  - `cargo fmt --check`

## Files to Modify

- `crates/daemon/src/ipc/handshake.rs` — Additive wire truth: repeated connections on same socket
- `crates/daemon/src/ipc/descriptor.rs` — Descriptor and service bootstrap alignment

## Spec Requirements (from §1.5)

**L2-FR-DB.15 — Layer 2 Extends the Handshake-Only IPC Envelope**
- AC1: The Layer 1 `hello`/`welcome` handshake remains the first exchange on the socket, including the existing `messageType` tagging and welcome field names.
- AC2: After a successful handshake, the same authenticated IPC channel accepts `command`, `subscribe`, and `unsubscribe` requests as specified here.
- AC3: The daemon-side protocol extension is additive relative to `crates/daemon/src/ipc/handshake.rs`; incompatible mutation traffic is no longer rejected once the L2 command router is active.

**L2-FR-DB.2 — Handshake Negotiates Compatibility and Restart Semantics**
- AC1: The first daemon-wire message is always `hello`, and the daemon responds with `DaemonWireWelcome` while preserving the Layer 1 welcome field names and meaning.
- AC2: `DaemonWireWelcome` includes protocol version, daemon version, `instanceId`, `schemaEpoch`, current `commitSeq`, daemon health summary, mutation allowance, active-session summary, reconnect outcome, and stream-resume permission.
- AC3: If the desktop-provided `lastSeenInstanceId` differs from the daemon `instanceId`, the desktop invalidates cached state and does not attempt to resume PTY streams from the prior daemon instance.

## Tests to Pass

- **RS-DBR-001**: First wire frame is `hello`; successful response is `welcome` with protocol version, daemon version, `instanceId`, `schemaEpoch`, `commitSeq`, health, `mutationsAllowed`, active sessions, reconnect outcome, and stream-resume flag
- **RS-DBR-009**: The first exchange remains `hello`/`welcome`; raw pre-router mutation traffic is rejected on the current Layer 1 handshake-only socket with `mutations_unavailable`; once the L2 command router is active, command traffic is accepted only after successful handshake

## Acceptance Criteria

1. The additive `hello`/`welcome` exchange stays intact while repeated authenticated connections become possible
2. Existing daemon bootstrap and descriptor behavior remain green
3. Service mode does not require replacing the Layer 1 wire shape

## Dependencies

- **Blocked by:** l2-03a
- **Blocks:** l2-04a
- **Phase:** 3 — Daemon Service Mode And Handshake Wire

---

## l2-04a: Command Router And Control Handler

- type: task
- priority: P0
- labels: phase-4,rust,daemon,ipc
- parent: testproject1-dr6

### Description

## SOURCE OF TRUTH WARNING

This bead is a quick-reference derived from:
- **Spec**: `docs/strategy/layers/layer-2-single-operator-desktop.md` — §1.5 (PRIMARY — read this first)
- **TDD**: `docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-tdd.md` — RS-DBR-005, RS-DBR-006 (PRIMARY)
- **Strategy**: `docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-strategy.md` — Phase 4 (ordering context only)

## Context

Create the first post-handshake dispatch point: `command_router.rs` dispatches typed commands to domain handlers. Also create `handlers/control.rs` for `get_daemon_info` and `restart_daemon`.

## Language & Quality Gates

- **Language:** `rust`
- **Quality gate commands:**
  - `cargo test -p daemon --test agent_contract_session`
  - `cargo check --all-targets`
  - `cargo clippy --all-targets -- -D warnings`
  - `cargo fmt --check`

## Files to Create

- `crates/daemon/src/ipc/command_router.rs` — First post-handshake dispatch point
- `crates/daemon/src/ipc/handlers/control.rs` — `get_daemon_info` and `restart_daemon` handlers

## Spec Requirements (from §1.5)

**L2-FR-DB.17 — Command Dispatch Is Mechanically Mapped to Daemon Handlers**
- AC1: `crates/daemon/src/ipc/command_router.rs` performs method dispatch from the request envelope to the daemon handler functions listed in the command table.
- AC2: Handler ownership is split by domain (`jobs`, `decisions`, `errors`, `sessions`, `control`) rather than by one monolithic mutation function.
- AC3: Unknown methods surface `BridgeError { code: 'not_found' }` or an equivalent unsupported-method rejection without invoking any handler.

**L2-FR-DB.19 — Bridge Request Kinds Form a Closed Wire Contract**
- AC1: The only valid daemon-wire request kinds in Layer 2 are `hello`, `command`, `subscribe`, and `unsubscribe`.
- AC2: Every request carries a client-generated correlation id (`requestId` on post-handshake traffic).
- AC3: Mutation `command` requests require both `method` and `idempotencyKey`; pure diagnostic reads such as `get_daemon_info` omit the idempotency key.

**L2-FR-DB.21 — `get_daemon_info()` Completes the Daemon Diagnostics Surface**
- AC1: `get_daemon_info()` returns the runtime/descriptor data needed by `DaemonStatus.tsx`: socket path, descriptor path, DB path, artifact root, WAL size, and startup-recovery summary.
- AC2: `get_daemon_info()` is read-only and does not perform SQLite mutations.

**L2-FR-DB.12 — `restart_daemon()` Acknowledges Before Disconnection**
- AC1: A successful restart request returns a committed `MutationAck` before the daemon intentionally disconnects.
- AC2: After restart, the next handshake exposes a new `instanceId`.

## Tests to Pass

- **RS-DBR-005**: Post-handshake traffic accepts only `command`, `subscribe`, and `unsubscribe`; every request carries `requestId`; mutation commands require `idempotencyKey` while `get_daemon_info` omits it
- **RS-DBR-006**: Unknown methods return `not_found`; domain handlers are invoked by the declared router ownership only for supported methods

## Acceptance Criteria

1. Command router dispatches by `method` field to domain-specific handlers
2. Unknown methods return typed rejection without invoking any handler
3. `get_daemon_info` returns runtime diagnostics without mutations
4. `restart_daemon` acknowledges before disconnect

## Dependencies

- **Blocked by:** l2-03b
- **Blocks:** l2-04b, l2-04c
- **Phase:** 4 — Daemon Control, Jobs, And Session Command Lane

---

## l2-04b: Jobs Handler And Runner Reuse

- type: task
- priority: P0
- labels: phase-4,rust,daemon,jobs
- parent: testproject1-dr6

### Description

## SOURCE OF TRUTH WARNING

This bead is a quick-reference derived from:
- **Spec**: `docs/strategy/layers/layer-2-single-operator-desktop.md` — §1.5 (PRIMARY — read this first)
- **TDD**: `docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-tdd.md` — RS-DBR-007 (PRIMARY)
- **Strategy**: `docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-strategy.md` — Phase 4 (ordering context only)

## Context

Create `handlers/jobs.rs` for `trigger_ingestion`, `trigger_enrichment`, and `retry_job`. These must reuse `JobRunner` durability and `visible_after_seq` semantics rather than inventing a second mutation path. Also generalize the source-command abstraction so the daemon is no longer "FTS-only" at the routing layer.

## Language & Quality Gates

- **Language:** `rust`
- **Quality gate commands:**
  - `cargo test -p daemon --test agent_contract_session`
  - `cargo check --all-targets`
  - `cargo clippy --all-targets -- -D warnings`
  - `cargo fmt --check`

## Files to Create/Modify

- `crates/daemon/src/ipc/handlers/jobs.rs` — `trigger_ingestion`, `trigger_enrichment`, `retry_job` handlers
- `crates/daemon/src/jobs/runner.rs` — Reuse existing durability/fence semantics; generalize source routing

## Spec Requirements (from §1.5)

**L2-FR-DB.4 — `trigger_ingestion()` Creates an Observable Ingestion Job**
- AC1: Successful `trigger_ingestion(source_id)` returns `MutationAck` with `accepted: true`, `jobId`, and `visibleAfterSeq`.
- AC2: Refused ingestion attempts surface an explicit operator-visible reason when the source is unknown, mutations are limited, or the daemon is unavailable.
- AC3: The desktop does not synthesize a job row before a subsequent query satisfies `snapshotSeq >= visibleAfterSeq`.

**L2-FR-DB.5 — `trigger_enrichment()` Re-Runs One Process**
- AC1: Successful `trigger_enrichment(process_id)` returns a committed `MutationAck` with `jobId`.
- AC2: The command targets one process and does not silently fan out to unrelated work.
- AC3: Unknown or non-rerunnable processes surface an explicit refusal instead of a silent no-op.

**L2-FR-DB.11 — `retry_job()` Creates a New Job Identity**
- AC1: Successful `retry_job(job_id)` returns a committed `MutationAck` with a new `jobId`.
- AC2: The original failed job remains queryable for audit and is not overwritten in place.
- AC3: Non-retryable or unknown jobs surface explicit refusal/failure.

**L2-FR-DB.3 — `MutationAck` Means Durable, Idempotent Progress**
- AC1: `accepted: true` means the mutation has reached durable SQLite state with a `visibleAfterSeq`; it never means "queued in RAM only."
- AC2: Retrying the same logical mutation with the same `idempotencyKey` returns the same durable outcome rather than creating duplicate writes.

## Tests to Pass

- **RS-DBR-007**: `busy`/`transport_unavailable` are retryable; validation and `not_interactive` errors are not; retry hints are stable across equivalent failures

## Acceptance Criteria

1. `trigger_ingestion`, `trigger_enrichment`, and `retry_job` all dispatch through router-owned handlers
2. Lost-ack, `visible_after_seq`, and idempotency behavior reuse `JobRunner` semantics
3. The daemon is no longer "FTS-only" at the command-routing layer
4. Command routing does not bypass `JobRunner` durability

## Dependencies

- **Blocked by:** l2-04a
- **Blocks:** l2-08a
- **Phase:** 4 — Daemon Control, Jobs, And Session Command Lane

---

## l2-04c: Session Control Handler

- type: task
- priority: P0
- labels: phase-4,rust,daemon,sessions
- parent: testproject1-dr6

### Description

## SOURCE OF TRUTH WARNING

This bead is a quick-reference derived from:
- **Spec**: `docs/strategy/layers/layer-2-single-operator-desktop.md` — §1.5 (PRIMARY — read this first)
- **TDD**: `docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-tdd.md` — RS-DBR-007 (PRIMARY)
- **Strategy**: `docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-strategy.md` — Phase 4 (ordering context only)

## Context

Create `handlers/sessions.rs` for `stop_session`, `send_session_input`, and `resize_session`. Without these controls, the operator still needs terminal-level escape hatches — undermining the "control panel, not browser" promise.

## Language & Quality Gates

- **Language:** `rust`
- **Quality gate commands:**
  - `cargo test -p daemon --test agent_contract_session`
  - `cargo check --all-targets`
  - `cargo clippy --all-targets -- -D warnings`
  - `cargo fmt --check`

## Files to Create

- `crates/daemon/src/ipc/handlers/sessions.rs` — `stop_session`, `send_session_input`, `resize_session` handlers

## Spec Requirements (from §1.5)

**L2-FR-DB.8 — `stop_session()` Is an Explicit Operator Interrupt**
- AC1: `stop_session(session_id, mode)` supports both `graceful` and `force` modes for active attached sessions.
- AC2: Successful stop requests return a committed `MutationAck`, and lifecycle updates continue through the streaming surface.
- AC3: Requests against unknown or already-terminal sessions surface an explicit refusal or no-op outcome.

**L2-FR-DB.9 — `send_session_input()` Targets Attached PTY Sessions Only**
- AC1: Successful `send_session_input(session_id, bytes)` returns a committed `MutationAck` for an attached PTY-backed session.
- AC2: Contract-mode sessions, detached sessions, and already-terminal sessions reject input explicitly.
- AC3: The bridge does not silently drop bytes while returning success.

**L2-FR-DB.10 — `resize_session()` Resizes Attached PTYs Only**
- AC1: Successful `resize_session(session_id, cols, rows)` returns a committed `MutationAck` for an attached PTY-backed session.
- AC2: Non-PTY or detached sessions reject resize requests explicitly.
- AC3: Invalid geometry parameters are rejected before they can taint session rendering state.

## Tests to Pass

- **RS-DBR-007**: Retry matrix validation for session commands — `not_interactive` errors are not retryable

## Acceptance Criteria

1. `stop_session` supports `graceful` and `force` modes
2. `send_session_input` targets only attached PTY sessions; contract/detached/terminal sessions reject
3. `resize_session` validates geometry before applying
4. All three handlers dispatch through the router

## Dependencies

- **Blocked by:** l2-04a
- **Blocks:** l2-08a, l2-10a
- **Phase:** 4 — Daemon Control, Jobs, And Session Command Lane

---

## l2-05a: Manifest Renderer And Template

- type: task
- priority: P0
- labels: phase-5,typescript,packages
- parent: testproject1-dr6

### Description

## SOURCE OF TRUTH WARNING

This bead is a quick-reference derived from:
- **Spec**: `docs/strategy/layers/layer-2-single-operator-desktop.md` — §1.12, §2.2 (PRIMARY — read this first)
- **TDD**: `docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-tdd.md` — TS-MAN-001 through TS-MAN-005 (PRIMARY)
- **Strategy**: `docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-strategy.md` — Phase 5 (ordering context only)

## Context

Create the deterministic opportunity manifest renderer in `packages/manifests`. This is the canonical rendering — the `manifest-generation` skill may contribute narrative synthesis, but this package is authoritative. `decisionBasisHash` is daemon-authored and passed through, NOT computed here.

## Language & Quality Gates

- **Language:** `typescript`
- **Quality gate commands:**
  - `bun run build`
  - `bun run lint`
  - `bun run test`

## Files to Create

- `packages/manifests/render-opportunity.ts` — Deterministic opportunity manifest renderer
- `packages/manifests/opportunity.md.ts` — Canonical manifest section layout / template
- `packages/manifests/tests/render-opportunity.test.ts` — Manifest package contract tests

## Spec Requirements (from §1.12, §2.2)

**L2-FR-MAN.1 — `packages/manifests` Owns Deterministic Opportunity Rendering**
- AC1: The canonical opportunity manifest renderer lives in `packages/manifests`, not in a skill directory or UI surface.
- AC2: Skills may contribute optional narrative content, but they do not replace the deterministic renderer contract.
- AC3: The renderer consumes daemon-provided hashes and pipeline inputs without re-authoring authority-owned values.

**L2-FR-MAN.2 — `renderOpportunityManifest()` Has a Stable Typed Contract**
- AC1: The renderer accepts the `ManifestInput` fields listed in §2.2 and returns `ManifestOutput` with `markdown`, `basisHash`, `sections`, `blockerCount`, and `generatedAt`.
- AC2: `basisHash` is passed through from daemon-provided input and is not recomputed inside the renderer.
- AC3: Missing optional pipeline data degrades to explicit missing sections or empty tables.

**L2-FR-MAN.3 — Opportunity Markdown Uses the Canonical Section Layout**
- AC1: Generated manifests contain the eight sections: Summary, Lot Analysis, Product Candidates, Pricing Evidence, Margin Estimate, Coverage Metrics, Risk Flags, Recommendation.
- AC2: Monetary and percentage formatting follow the rules: `£X,XXX.XX` from integer pence, percentages from basis points.
- AC3: Blockers and confidence bands are rendered in operator-facing formats.

**L2-FR-MAN.4 — Empty Data Still Produces Explicit Manifest Sections**
- AC1: Missing candidates, quotes, or landed-cost data produce explicit empty-state prose or empty tables.

**L2-FR-MAN.5 — Manifest Navigation Uses Stable Parsed Sections**
- AC1: `sections` in `ManifestOutput` correspond to the generated heading structure.

## Key Types (from spec §2.2 — verbatim)

```typescript
interface ManifestInput {
  process: ProcurementProcess
  notices: ContractNotice[]
  lots: NoticeLot[]
  candidates: ProductCandidate[]
  quotes: SupplierQuote[]
  pricingAssessments: CandidatePricingAssessment[]
  landedCosts: LandedCostEstimate[]
  pipelineAssessment: PipelineAssessment
  coverageMetrics: {
    pricedCoverageBps: number
    landedCostCoverageBps: number
    netQualifiedCoverageBps: number
    unresolvedMandatoryLots: number
  }
}

interface ManifestOutput {
  markdown: string
  basisHash: string
  sections: string[]
  blockerCount: number
  generatedAt: string
}

function renderOpportunityManifest(input: ManifestInput): ManifestOutput
```

## Tests to Pass

- **TS-MAN-001**: Renderer returns `markdown`, `basisHash`, `sections`, `blockerCount`, and `generatedAt`; no output field omitted
- **TS-MAN-002**: Renderer passes through daemon-provided `basisHash` without recomputing; output hash matches input exactly
- **TS-MAN-003**: Output contains eight canonical section headings in declared order with stable parsed section ids
- **TS-MAN-004**: Missing candidates/quotes/landed-cost still produce explicit sections and empty-state prose
- **TS-MAN-005**: Money renders from integer pence, percentages from bps, blockers prominent, confidence bands avoid decimal dumps

## Acceptance Criteria

1. `renderOpportunityManifest()` accepts `ManifestInput` and returns `ManifestOutput`
2. `basisHash` is passed through, NOT recomputed
3. All eight canonical sections are present in output
4. Empty data produces explicit empty-state sections, not omissions
5. All tests TS-MAN-001 through TS-MAN-005 pass

## Dependencies

- **Blocked by:** l2-01a
- **Blocks:** l2-09a
- **Phase:** 5 — Shared Data Packages

---

## l2-05b: Coverage Rollup And Qualification Rules

- type: task
- priority: P0
- labels: phase-5,typescript,packages
- parent: testproject1-dr6

### Description

## SOURCE OF TRUTH WARNING

This bead is a quick-reference derived from:
- **Spec**: `docs/strategy/layers/layer-2-single-operator-desktop.md` — §3.1, §3.4 (PRIMARY — read this first)
- **TDD**: `docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-tdd.md` — TS-COV-001 through TS-COV-006, TS-QUAL-001 through TS-QUAL-005 (PRIMARY)
- **Strategy**: `docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-strategy.md` — Phase 5 (ordering context only)

## Context

Create the coverage rollup module and goods/services qualification rules in `packages/intelligence`. These are pure TS packages with no daemon, desktop, or UI dependencies.

## Language & Quality Gates

- **Language:** `typescript`
- **Quality gate commands:**
  - `bun run build`
  - `bun run lint`
  - `bun run test`

## Files to Create

- `packages/intelligence/src/margins/coverage-rollup.ts` — Runtime implementation of L0 §3.14 coverage formulas
- `packages/intelligence/src/margins/coverage-rollup.test.ts` — Coverage math tests
- `packages/intelligence/src/qualification/goods-qualification.ts` — Goods/services classifier
- `packages/intelligence/src/qualification/goods-qualification.test.ts` — Qualification tests

## Spec Requirements (from §3.1, §3.4)

**L2-FR-COV.2 — Coverage Rollup Implements the L0 Formula Contract**
- AC1: `computeCoverageRollup()` returns both lot-count and value-weighted coverage outputs.
- AC2: Coverage output is derived from qualifying-lot, lot-value, and candidate-level inputs.

**L2-FR-COV.5 — Auto-Qualification Uses Value-Weighted Coverage**
- AC1: The auto-qualification gate uses `netQualifiedValueCoverageBps >= 5000`, not `netQualifiedCoverageBps`.
- AC2: Lot-count coverage badges do not override a failed value-weighted gate.
- AC3: Missing lot values or candidate value shares keep the gate conservative.

## Key Types (from spec §3.4 — verbatim)

```typescript
interface CoverageInput {
  lots: Array<{
    lotId: string
    qualificationResult: 'pass' | 'review' | 'reject'
    isMandatory: boolean
    lotAddressableValuePence: number | null
    candidates: Array<{
      candidateId: string
      hasPricing: boolean
      hasLandedCost: boolean
      passesNetGate: boolean
      candidateValueShareBps: number | null
    }>
  }>
}

interface CoverageOutput {
  pricedCoverageBps: number
  landedCostCoverageBps: number
  netQualifiedCoverageBps: number
  pricedValueCoverageBps: number
  landedCostValueCoverageBps: number
  netQualifiedValueCoverageBps: number
  unresolvedMandatoryLots: number
  decisionBasisHash: string
}

function computeCoverageRollup(input: CoverageInput): CoverageOutput
```

## Tests to Pass

- **TS-COV-001**: Lot-count metrics follow qualifying-lot denominators exactly
- **TS-COV-002**: Value-weighted metrics use qualifying lot value, not lot count
- **TS-COV-003**: 7000/3000 threshold bands map to stable labels and tones
- **TS-COV-004**: `unresolvedMandatoryLots` returned as absolute count
- **TS-COV-005**: Missing lot values keep gate conservative, never inflate coverage
- **TS-COV-006**: `decisionBasisHash` stable for equivalent inputs, changes when inputs change
- **TS-QUAL-001**: CPV fixtures classify into goods/services/works/mixed/unknown deterministically
- **TS-QUAL-002**: Expired notices filtered; DPS/Dynamic Market remain eligible
- **TS-QUAL-003**: Mixed contract with separable goods computes goods-addressable value
- **TS-QUAL-004**: Low-confidence cases stay reviewable, not silently qualifying
- **TS-QUAL-005**: Access-route edge cases preserved in qualification output

## Acceptance Criteria

1. `computeCoverageRollup()` returns all lot-count and value-weighted metrics
2. Auto-qualification gate uses value-weighted coverage (5000 bps threshold)
3. Goods/services classifier handles CPV families correctly
4. All tests TS-COV-001-006 and TS-QUAL-001-005 pass

## Dependencies

- **Blocked by:** l2-01a
- **Blocks:** l2-09a, l2-11a
- **Phase:** 5 — Shared Data Packages

---

## l2-06a: List And Detail Shared Cards And Badges

- type: task
- priority: P0
- labels: phase-6,typescript,ui
- parent: testproject1-dr6

### Description

## SOURCE OF TRUTH WARNING

This bead is a quick-reference derived from:
- **Spec**: `docs/strategy/layers/layer-2-single-operator-desktop.md` — §1.11, §2.1 (PRIMARY — read this first)
- **TDD**: `docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-tdd.md` — TS-UI-002, TS-UI-004, TS-UI-005 (PRIMARY)
- **Strategy**: `docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-strategy.md` — Phase 6 (ordering context only)

## Context

Create three shared UI components: `ContractCardFrame` (frame + slot composition), `PipelineBadge` (accessible pipeline state), and `FreshnessBadge` (surface-neutral freshness). These are pure presentational — no backend imports.

## Language & Quality Gates

- **Language:** `typescript`
- **Quality gate commands:**
  - `bun run build`
  - `bun run test`

## Files to Create

- `packages/ui/contract-card-frame.tsx` — Process summary card with slot composition
- `packages/ui/pipeline-badge.tsx` — Pipeline state badge with color + text + icon
- `packages/ui/freshness-badge.tsx` — Surface-neutral freshness indicator

## Spec Requirements (from §1.11, §2.1)

**L2-FR-UI.1 — `packages/ui` Remains Purely Presentational**
- AC1: Shared UI components accept typed props and render UI without importing QueryService, Supabase clients, Tauri bridge code, or mutation orchestration.

**L2-FR-UI.2 — `ContractCardFrame` Renders the Stable Summary Card Contract**
- AC1: The component accepts summary, metrics, decision, `rightRail`, and `actions` props.
- AC2: Monetary and percentage formatting follow display rules: `£X,XXX` from integer pence, percentages from bps.
- AC3: Stale decisions and keyset-stable ordering remain visible without boolean-prop variants.

**L2-FR-UI.4 — `PipelineBadge` Presents Pipeline State Accessibly**
- AC1: Color + text label + icon (never color alone — accessibility).
- AC2: `validationState` badge shown only when not `unverified`.

**L2-FR-UI.5 — `FreshnessBadge` Consumes a Surface-Neutral View Model**
- AC1: Accepts only `FreshnessBadgeVM` (`label`, `tone`, `tooltip`) and does not compute freshness internally.

**L2-FR-A11Y.2 — Operator Signals Are Never Color-Only**
- AC1: Margin, risk, coverage, and health indicators present text and/or icon semantics alongside color.

## Tests to Pass

- **TS-UI-002**: `ContractCardFrame` renders summary, metrics, decision, `rightRail`, and `actions` slots without boolean-prop branching; error state renderable
- **TS-UI-004**: `PipelineBadge` always shows text + icon + color; `validationState` hidden when `unverified`
- **TS-UI-005**: `FreshnessBadge` consumes only `FreshnessBadgeVM` shape; no internal timestamp inspection

## Acceptance Criteria

1. All three components are pure presentational with no backend imports
2. `ContractCardFrame` uses frame + slot composition pattern
3. `PipelineBadge` is accessible: color + text + icon
4. `FreshnessBadge` accepts surface-neutral VM only
5. Tests TS-UI-002, TS-UI-004, TS-UI-005 pass

## Dependencies

- **Blocked by:** l2-01a
- **Blocks:** l2-07a
- **Phase:** 6 — Shared UI Surfaces

---

## l2-06b: Sanitized Manifest And Error Renderers

- type: task
- priority: P0
- labels: phase-6,typescript,ui,security
- parent: testproject1-dr6

### Description

## SOURCE OF TRUTH WARNING

This bead is a quick-reference derived from:
- **Spec**: `docs/strategy/layers/layer-2-single-operator-desktop.md` — §1.11, §2.1, §2.3 (PRIMARY — read this first)
- **TDD**: `docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-tdd.md` — TS-UI-003, TS-UI-006 (PRIMARY)
- **Strategy**: `docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-strategy.md` — Phase 6 (ordering context only)

## Context

Create two shared UI components with critical security boundaries: `ManifestViewer` (sanitized Markdown renderer) and `ErrorSummaryList` (typed error list). Agent-authored content is UNTRUSTED INPUT.

## Language & Quality Gates

- **Language:** `typescript`
- **Quality gate commands:**
  - `bun run build`
  - `bun run test`

## Files to Create

- `packages/ui/manifest-viewer.tsx` — Sanitized Markdown manifest renderer with section navigation
- `packages/ui/error-summary-list.tsx` — Compact error manifest list with severity/resolution status
- `packages/ui/tests/components.test.tsx` — Shared UI component tests (home for all TS-UI-* tests)

## Spec Requirements (from §1.11, §2.3)

**L2-FR-UI.3 — `ManifestViewer` Renders Sanitized, Navigable Opportunity Manifests**
- AC1: Renders manifest Markdown with section navigation, blocker highlighting, and print-friendly layout.
- AC2: Raw HTML passthrough is disabled and dangerous link schemes are stripped.
- AC3: Section copy and navigation affordances operate on parsed section structure.

**L2-FR-UI.6 — `ErrorSummaryList` Surfaces Sanitized, Typed Error Summaries**
- AC1: Accepts typed `ErrorSummaryItem` rows and resolve/click callbacks.
- AC2: Severity derivation follows the rule set in the spec.
- AC3: Error messages and context are sanitized before rendering.

**L2-FR-SEC.1 — Agent-Authored Content Is Sanitized Before Rendering**
- AC1: `ManifestViewer`, `ErrorSummaryList`, and `SessionViewer` treat agent-authored text as untrusted input.
- AC2: Markdown renderers disable raw HTML and strip `javascript:` and unsafe `file://` links.

**L2-FR-UI.11 — Read-Only Surfaces Omit Mutating Error Actions**
- AC1: `onResolveClick` is optional and may be `null` for read-only consumers.

## Tests to Pass

- **TS-UI-003**: `ManifestViewer` renders navigable sections, preserves copyable section ids, uses sanitized markdown output only
- **TS-UI-006**: `ErrorSummaryList` renders typed rows, honors optional `onResolveClick = null`, and functions without mutable surface

## Acceptance Criteria

1. `ManifestViewer` sanitizes Markdown: no raw HTML, no dangerous links
2. `ErrorSummaryList` accepts typed rows with optional resolve callback
3. Both components are pure presentational with no backend imports
4. Tests TS-UI-003 and TS-UI-006 pass

## Dependencies

- **Blocked by:** l2-01a
- **Blocks:** l2-07a
- **Phase:** 6 — Shared UI Surfaces

---

## l2-07a: Transport Shim, Dev Bridge, And UI Barrel

- type: task
- priority: P0
- labels: phase-7,typescript,transport
- parent: testproject1-dr6

### Description

## SOURCE OF TRUTH WARNING

This bead is a quick-reference derived from:
- **Spec**: `docs/strategy/layers/layer-2-single-operator-desktop.md` — §1.8, §1.11 (PRIMARY — read this first)
- **TDD**: `docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-tdd.md` — TS-TRN-001 through TS-TRN-004, TS-UI-001, TS-UI-007, TS-UI-008, CI-L2-004 (PRIMARY)
- **Strategy**: `docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-strategy.md` — Phase 7 (ordering context only)

## Context

Create the centralized transport shim, Vite dev bridge plugin, and shared UI barrel export. This establishes the single frontend access path: all hooks/stores call through `transport.ts`, never importing `@tauri-apps/api` directly.

## Language & Quality Gates

- **Language:** `typescript`
- **Quality gate commands:**
  - `bun run build`
  - `bun run lint`
  - `bun run test`

## Files to Create

- `packages/ui/index.ts` — Barrel export for all shared components
- `apps/desktop/src/lib/transport.ts` — Runtime shim: Tauri native bridge or HTTP fallback
- `apps/desktop/src/lib/transport.test.ts` — Transport contract tests
- `apps/desktop/vite-plugin-tauri-dev-bridge.ts` — Vite plugin: HTTP proxy for `/__tauri_invoke__` + WebSocket relay at `/__ws__`

## Spec Requirements (from §1.8)

**L2-FR-DEV.1 — Dev Browser Invokes the Same Query and Command Surface**
- AC1: In browser dev mode, `/__tauri_invoke__` proxies the same query and mutation commands that the production Tauri bridge exposes.

**L2-FR-DEV.2 — Dev Browser Streaming Matches the Session Frame Contract**
- AC1: The WebSocket relay at `/__ws__` forwards `SessionFrame` and `BridgeEvent` payloads using the same typed contracts.

**L2-FR-DEV.3 — Transport Selection Is Centralized in One Runtime Shim**
- AC1: Frontend hooks and stores call through `src/lib/transport.ts` instead of importing Tauri APIs directly.
- AC2: Production webview execution chooses the native bridge when `window.__TAURI_INTERNALS__` is present.
- AC3: Browser dev execution chooses the HTTP/WebSocket fallback only when the Tauri bridge is absent.

**L2-FR-DEV.4 — The Dev Bridge Is Local-Only and Never Shipped**
- AC1: The middleware binds to `127.0.0.1` only.
- AC2: Production builds exclude the dev bridge code path.

**L2-FR-UI.1 — `packages/ui` Remains Purely Presentational**
**L2-FR-UI.8 — `index.ts` Re-Exports Only the Shared Presentational Surface**

## Tests to Pass

- **TS-TRN-001**: Transport shim selects native invoke/channel path when Tauri internals exist; no HTTP fallback request issued
- **TS-TRN-002**: Transport shim uses `POST /__tauri_invoke__` plus `/__ws__` for streaming in browser dev mode
- **TS-TRN-003**: Hooks and stores import the shim instead of `@tauri-apps/api` directly
- **TS-TRN-004**: Production bundle omits dev bridge entry and loopback middleware code path
- **TS-UI-001**: `packages/ui` exports presentational components and view-model types only; no desktop bridge, QueryService, daemon, or Supabase imports
- **TS-UI-007**: Barrel export re-exports only shared UI surface from one entry point; no backend side effects
- **TS-UI-008**: All shared-component tests are fixture-based and backend-agnostic
- **CI-L2-004**: `bun install` resolves; `bun run lint` exits 0; no TS surface imports desktop-only code into shared packages

## Acceptance Criteria

1. `transport.ts` centralizes all frontend transport — ~10 lines runtime shim
2. Dev bridge plugin provides HTTP + WebSocket proxy matching the Tauri bridge contract
3. `packages/ui/index.ts` barrel export exposes only presentational surface
4. No frontend code imports `@tauri-apps/api` directly
5. All tests pass: TS-TRN-001-004, TS-UI-001, TS-UI-007, TS-UI-008, CI-L2-004

## Dependencies

- **Blocked by:** l2-02a, l2-02b, l2-06a, l2-06b
- **Blocks:** l2-07b
- **Phase:** 7 — Frontend Transport And App Shell Foundation

---

## l2-07b: Control-Plane Hook And App Shell

- type: task
- priority: P0
- labels: phase-7,typescript,app
- parent: testproject1-dr6

### Description

## SOURCE OF TRUTH WARNING

This bead is a quick-reference derived from:
- **Spec**: `docs/strategy/layers/layer-2-single-operator-desktop.md` — §1.7, §1.9, §1.10 (PRIMARY — read this first)
- **TDD**: `docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-tdd.md` — CI-L2-004 (PRIMARY)
- **Strategy**: `docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-strategy.md` — Phase 7 (ordering context only)

## Context

Create the control-plane hook and root App component. `useControlPlaneState()` derives one of five operator modes from daemon health. `App.tsx` owns the six route shapes and bootstrap gating.

## Language & Quality Gates

- **Language:** `typescript`
- **Quality gate commands:**
  - `bun run build`
  - `bun run lint`
  - `bun run test`

## Files to Create

- `apps/desktop/src/hooks/use-control-plane.ts` — `useControlPlaneState()` via `useSyncExternalStore`
- `apps/desktop/src/App.tsx` — Root React component with routing and bootstrap state

## Spec Requirements (from §1.7, §1.9, §1.10)

**L2-FR-BOOT.3 — Bootstrap Derives One of Five Operator Modes**
- AC1: Launch evaluation considers DB readability/schema state and daemon handshake results.
- AC2: Exactly one of: full control, control limited, browse-only, incompatible, or recovery required.
- AC3: Each mode presents the UX contract listed in the spec.

**L2-FR-BOOT.7 — `ControlPlaneState` Is the Canonical Runtime Health Model**

**L2-FR-BOOT.11 — Runtime-Mode Derivation Is Total and Non-Overlapping**
- AC1: The precedence rules map every reachable `ControlPlaneState` combination to exactly one UX mode.

**L2-FR-PAGE.7 — `src/App.tsx` Owns the Route Table and Bootstrap Gating**
- AC1: `src/App.tsx` exposes the six route shapes: `/`, `/process/:id`, `/sessions`, `/sessions/:id`, `/errors`, `/daemon`.
- AC2: Route availability and action enablement are gated by bootstrap/runtime mode.
- AC3: Unknown routes resolve to an operator-visible not-found state.

**L2-FR-STATE.1 — Snapshot, Streaming, and Control-Plane State Stay Separated**
**L2-FR-STATE.6 — Control-Plane Store Has a Minimal Public API**

## Key Types (from spec §1.4 — verbatim)

```typescript
type ControlPlaneState = {
  daemonInstanceId: string
  daemonVersion: string
  protocolVersion: number
  schemaEpoch: number
  storage: 'ready' | 'repairing' | 'unreadable' | 'schema_mismatch'
  daemon: 'starting' | 'ready' | 'degraded' | 'unavailable' | 'incompatible'
  control: 'full' | 'limited' | 'read_only'
  load: 'normal' | 'busy' | 'saturated'
  reasons: string[]
}

type AppRoutes =
  | '/'
  | `/process/${string}`
  | '/sessions'
  | `/sessions/${string}`
  | '/errors'
  | '/daemon'
```

## Tests to Pass

- **CI-L2-004**: `bun run lint` exits 0; no TS surface imports desktop-only code into shared packages

## Acceptance Criteria

1. `useControlPlaneState()` derives exactly one of five operator modes
2. `App.tsx` owns six route shapes with bootstrap gating
3. Unknown routes render not-found state
4. Control-plane state is separate from snapshot query state

## Dependencies

- **Blocked by:** l2-07a
- **Blocks:** l2-08a, l2-09a, l2-10a
- **Phase:** 7 — Frontend Transport And App Shell Foundation

---

## l2-08a: Workspace Update And Native Bootstrap

- type: task
- priority: P0
- labels: phase-8,mixed,tauri
- parent: testproject1-dr6

### Description

## SOURCE OF TRUTH WARNING

This bead is a quick-reference derived from:
- **Spec**: `docs/strategy/layers/layer-2-single-operator-desktop.md` — §1.3, §1.7, §1.9 (PRIMARY — read this first)
- **TDD**: `docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-tdd.md` — CI-L2-001, CI-L2-002, CI-L2-003, CI-L2-007 (PRIMARY)
- **Strategy**: `docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-strategy.md` — Phase 8 (ordering context only)

## Context

Adopt `src-tauri` into the root Cargo workspace and create the native Tauri bootstrap `main.rs`. This converges only once daemon/service and app contracts exist.

## Language & Quality Gates

- **Language:** `mixed` (Rust + workspace config)
- **Quality gate commands:**
  - `cargo check --all-targets`
  - `cargo clippy --all-targets -- -D warnings`
  - `cargo fmt --check`

## Files to Modify/Create

- `Cargo.toml` — Root workspace membership update to adopt `apps/desktop/src-tauri`
- `apps/desktop/src-tauri/src/main.rs` — Tauri Rust bootstrap: DB check, daemon handshake, QueryService + DaemonBridge init

## Spec Requirements (from §1.7, §1.9)

**L2-FR-BOOT.6 — Desktop Startup Auto-Launches the Daemon When Possible**
- AC1: Bootstrap performs the startup sequence: DB checks, descriptor read, handshake attempt, detached daemon launch if needed, retry, then mode fallback.
- AC2: The operator is not required to start the daemon manually.

**L2-FR-BOOT.8 — Descriptor Read Precedes Detached Launch**
**L2-FR-BOOT.9 — Auto-Launch Is Single-Flight**

## Tests to Pass

- **CI-L2-001**: `cargo check --all-targets` exits 0 for root workspace including desktop Rust crate
- **CI-L2-002**: `cargo clippy --all-targets -- -D warnings` exits 0
- **CI-L2-003**: `cargo fmt --check` exits 0
- **CI-L2-007**: Every `apps/desktop/src-tauri/src/*.rs` file starts with `#![forbid(unsafe_code)]`

## Acceptance Criteria

1. Root Cargo workspace includes `apps/desktop/src-tauri`
2. `src-tauri/src/main.rs` performs startup sequence from spec §1.7
3. `#![forbid(unsafe_code)]` on all desktop Rust sources
4. All CI gates CI-L2-001, CI-L2-002, CI-L2-003, CI-L2-007 pass

## Dependencies

- **Blocked by:** l2-04b, l2-04c, l2-07b
- **Blocks:** l2-08b, l2-08c
- **Phase:** 8 — Desktop Rust Bridge And Native Bootstrap

---

## l2-08b: QueryService Read-Only Bridge

- type: task
- priority: P0
- labels: phase-8,rust,queryservice
- parent: testproject1-dr6

### Description

## SOURCE OF TRUTH WARNING

This bead is a quick-reference derived from:
- **Spec**: `docs/strategy/layers/layer-2-single-operator-desktop.md` — §1.4 (PRIMARY — read this first)
- **TDD**: `docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-tdd.md` — RS-QS-001 through RS-QS-012 (PRIMARY)
- **Strategy**: `docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-strategy.md` — Phase 8 (ordering context only)

## Context

Implement the QueryService: read-only SQLite queries returning `QueryEnvelope<T>` with commit-fence semantics. This is the desktop-side read model for all list/detail views.

## Language & Quality Gates

- **Language:** `rust`
- **Quality gate commands:**
  - `cargo test --test query_service`
  - `cargo check --all-targets`
  - `cargo clippy --all-targets -- -D warnings`
  - `cargo fmt --check`

## Files to Create

- `apps/desktop/src-tauri/src/query_service.rs` — Read-only SQLite queries returning `QueryEnvelope<T>`
- `apps/desktop/src-tauri/tests/query_service.rs` — QueryService contract tests

## Spec Requirements (from §1.4)

**L2-FR-QS.1 — QueryService Is Read-Only, Page-Scoped, and Non-Streaming**
- AC1: QueryService connections enable `PRAGMA query_only = ON` and never perform writes.
- AC2: Each desktop query returns a page-specific read model.
- AC3: Hot queries exclude transcript blobs, artifact payloads, and long-lived streaming cursors.
- AC4: Query execution uses short-lived read transactions so WAL checkpoints are not starved.

**L2-FR-QS.2 — Snapshot Envelopes and the Commit Fence Are Normative**
- AC1: Every QueryService response returns `QueryEnvelope<T>` with `data`, `snapshotSeq`, and `snapshotAt`.
- AC2: `QueryEnvelope<T>` contains snapshot data only; live daemon health is delivered separately.
- AC3: The desktop treats a mutation as visible only when `snapshotSeq >= visibleAfterSeq`.

**L2-FR-QS.3 — The User-Visible Snapshot Clock Is `commit_seq`**
- AC1: `snapshotSeq` is sourced from `commit_clock.commit_seq`, not `PRAGMA data_version`.

**L2-FR-QS.4 through L2-FR-QS.13** — See spec §1.4 for all 13 QueryService requirements.

## Key Read Models (from spec §1.4 — verbatim)

```typescript
type QueryEnvelope<T> = {
  data: T
  snapshotSeq: number
  snapshotAt: string
}

type KeysetPage<T> = {
  items: T[]
  nextCursor: string | null
  hasMore: boolean
}
```

**Queries:** `list_processes()`, `get_process_detail()`, `list_sessions()`, `list_errors()`, `get_session_history()`

## Tests to Pass

- **RS-QS-001**: Read connection enables `PRAGMA query_only = ON`; no write/schema/generic SQL path
- **RS-QS-002**: `list_processes()` returns `QueryEnvelope<KeysetPage<ProcessListItem>>` with proper fields
- **RS-QS-003**: `snapshotSeq` sourced from `commit_clock.commit_seq`, not `PRAGMA data_version`
- **RS-QS-004**: `get_process_detail()` explicit empty arrays for zero lots/candidates; missing artifacts degrade
- **RS-QS-005**: `list_sessions()` returns canonical phase names, timing, cleanup verdicts
- **RS-QS-006**: `list_errors()` returns derived severity and linkage fields
- **RS-QS-007**: Invalid cursor returns `invalid_cursor`; unsupported filter returns `invalid_filter`
- **RS-QS-008**: `get_session_history()` returns availability `complete|partial|missing`
- **RS-QS-009**: Busy-timeout returns typed read failure; no state mutation
- **RS-QS-010**: Short-lived read transactions; WAL checkpoints can proceed
- **RS-QS-011**: `commit_clock.commit_seq` increments exactly once per write transaction
- **RS-QS-012**: `PRAGMA data_version` for cache invalidation only; `snapshotSeq` is `commit_seq`

## Acceptance Criteria

1. QueryService opens read-only SQLite connections with `PRAGMA query_only = ON`
2. All queries return `QueryEnvelope<T>` with `snapshotSeq` from `commit_clock`
3. Short-lived read transactions that don't starve WAL checkpoints
4. All tests RS-QS-001 through RS-QS-012 pass

## Dependencies

- **Blocked by:** l2-08a
- **Blocks:** l2-09a
- **Phase:** 8 — Desktop Rust Bridge And Native Bootstrap

---

## l2-08c: DaemonBridge RPC Proxy

- type: task
- priority: P0
- labels: phase-8,rust,daemonbridge
- parent: testproject1-dr6

### Description

## SOURCE OF TRUTH WARNING

This bead is a quick-reference derived from:
- **Spec**: `docs/strategy/layers/layer-2-single-operator-desktop.md` — §1.5 (PRIMARY — read this first)
- **TDD**: `docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-tdd.md` — RS-DBR-001 through RS-DBR-007 (PRIMARY)
- **Strategy**: `docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-strategy.md` — Phase 8 (ordering context only)

## Context

Implement the DaemonBridge: thin RPC proxy forwarding 10 commands to the daemon over UDS/named pipe. Does not hold state or perform writes itself. Handles handshake, wire adaptation, and `Channel<T>` relay.

## Language & Quality Gates

- **Language:** `rust`
- **Quality gate commands:**
  - `cargo test --test daemon_bridge`
  - `cargo check --all-targets`
  - `cargo clippy --all-targets -- -D warnings`
  - `cargo fmt --check`

## Files to Create

- `apps/desktop/src-tauri/src/daemon_bridge.rs` — RPC proxy to daemon over UDS/named-pipe
- `apps/desktop/src-tauri/tests/daemon_bridge.rs` — DaemonBridge contract tests

## Spec Requirements (from §1.5)

**L2-FR-DB.1 — DaemonBridge Is a Thin RPC Proxy**
- AC1: Forwards commands to daemon over UDS/named pipe; does not perform authoritative writes.
- AC2: Bridge state derived from daemon handshakes, heartbeats, and command responses.
- AC3: Transport errors surfaced distinctly from daemon refusals.

**10 Commands (from spec §1.5):**
1. `get_daemon_info` — Pure read command
2. `trigger_ingestion` — `TriggerIngestionParams { sourceId }` → `MutationAck` with `jobId`
3. `trigger_enrichment` — `TriggerEnrichmentParams { processId }` → `MutationAck` with `jobId`
4. `capture_decision` — `CaptureDecisionParams` → `MutationAck`
5. `resolve_error` — `ResolveErrorParams { errorManifestId }` → `MutationAck`
6. `stop_session` — `StopSessionParams { sessionId, mode }` → `MutationAck`
7. `send_session_input` — `SendSessionInputParams { sessionId, inputUtf8 }` → `MutationAck`
8. `resize_session` — `ResizeSessionParams { sessionId, cols, rows }` → `MutationAck`
9. `retry_job` — `RetryJobParams { jobId }` → `MutationAck` with new `jobId`
10. `restart_daemon` — `RestartDaemonParams` → `MutationAck` (ack before restart)
11. `rerun_from_error` — `RerunFromErrorParams { errorManifestId }` → `MutationAck` with new `jobId`

**L2-FR-DB.14 through L2-FR-DB.20** — See spec §1.5 for all DaemonBridge requirements.

## Tests to Pass

- **RS-DBR-001**: Handshake `hello`/`welcome` with all required fields
- **RS-DBR-002**: Protocol mismatch returns `protocol_range_mismatch`
- **RS-DBR-003**: Schema epoch mismatch returns typed rejection
- **RS-DBR-004**: Reconnect outcomes: `same_daemon_reconnect` vs `daemon_restart_invalidated`
- **RS-DBR-005**: Post-handshake accepts `command`/`subscribe`/`unsubscribe` only; `requestId` required
- **RS-DBR-006**: Unknown methods return `not_found`; domain routing correct
- **RS-DBR-007**: Retry matrix: `busy`/`transport_unavailable` retryable; validation errors not

## Acceptance Criteria

1. DaemonBridge is a thin proxy — no authoritative writes
2. All 10 commands routed through bridge to daemon
3. Transport errors distinct from daemon refusals
4. Handshake compatibility and restart semantics correct
5. All tests RS-DBR-001 through RS-DBR-007 pass

## Dependencies

- **Blocked by:** l2-08a
- **Blocks:** l2-09a, l2-10a
- **Phase:** 8 — Desktop Rust Bridge And Native Bootstrap

---

## l2-09a: Contract List And Detail Pages

- type: task
- priority: P0
- labels: phase-9,typescript,pages
- parent: testproject1-dr6

### Description

## SOURCE OF TRUTH WARNING

This bead is a quick-reference derived from:
- **Spec**: `docs/strategy/layers/layer-2-single-operator-desktop.md` — §1.9 (PRIMARY — read this first)
- **TDD**: `docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-tdd.md` — PW-CON-001 through PW-CON-010 (PRIMARY)
- **Strategy**: `docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-strategy.md` — Phase 9 (ordering context only)

## Context

Create the first read-only operator browse surfaces: `ContractList.tsx` (keyset-paginated process list) and `ContractDetail.tsx` (full opportunity detail with manifest, coverage, and errors).

## Language & Quality Gates

- **Language:** `typescript`
- **Quality gate commands:**
  - `bunx playwright test e2e/contracts.spec.ts`

## Files to Create

- `apps/desktop/src/pages/ContractList.tsx` — Keyset-paginated process list
- `apps/desktop/src/pages/ContractDetail.tsx` — Full opportunity detail view
- `apps/desktop/e2e/contracts.spec.ts` — List/detail browsing E2E tests

## Spec Requirements (from §1.9)

**L2-FR-PAGE.1 — Contract List Surfaces the Daily Opportunity Queue**
- AC1: List page renders `list_processes()` results with keyset pagination, pipeline badges, coverage summaries, and decision status.
- AC2: Empty-result states render explicitly with filter-aware empty messaging.
- AC3: Live updates do not cause row jumping or focus loss.

**L2-FR-PAGE.2 — Contract Detail Surfaces the Full Review Packet**
- AC1: Detail page renders notices, lots, candidates, quotes, margin outputs, manifest, and errors.
- AC2: Zero-lot processes render explicit empty states.
- AC3: Decision status and staleness are visible on the detail surface.

**L2-FR-PAGE.8, L2-FR-PAGE.9** — Query and navigation contracts.
**L2-FR-PAG.1, L2-FR-PAG.2** — Keyset pagination and empty/terminal page semantics.
**L2-FR-COV.1, L2-FR-COV.3** — Coverage metrics presentation and threshold labels.

## Tests to Pass

- **PW-CON-001**: List page renders 25 rows from one envelope; first row shows title, buyer, deadline, badges, decision summary
- **PW-CON-002**: Empty query renders explicit empty-state copy; `hasMore = false`
- **PW-CON-003**: Next-page uses cursor not offset; no duplicate `processId`; last page `nextCursor = null`
- **PW-CON-004**: Live insert does not reorder visible rows; focus stays on selected row
- **PW-CON-005**: Filter normalization; invalid filter values rejected before partial rendering
- **PW-CON-006**: Expired non-DPS absent; expired DPS visible
- **PW-CON-007**: Detail page renders full review packet in integer-pence/bps-derived formats
- **PW-CON-008**: Zero-lot process shows explicit empty-state copy
- **PW-CON-009**: Empty candidates/quotes render explicit placeholders; coverage shows zero values
- **PW-CON-010**: Both lot-count and value-weighted gate results visible; failed value-weighted warning shown

## Acceptance Criteria

1. ContractList uses keyset pagination, not offset
2. ContractDetail renders full review packet from one detail read model
3. Empty states are explicit, not errors
4. All tests PW-CON-001 through PW-CON-010 pass

## Dependencies

- **Blocked by:** l2-05a, l2-05b, l2-07b, l2-08b, l2-08c
- **Blocks:** l2-11a
- **Phase:** 9 — Read-Only Operator Pages

---

## l2-09b: Errors And Daemon Status Pages

- type: task
- priority: P0
- labels: phase-9,typescript,pages
- parent: testproject1-dr6

### Description

## SOURCE OF TRUTH WARNING

This bead is a quick-reference derived from:
- **Spec**: `docs/strategy/layers/layer-2-single-operator-desktop.md` — §1.9 (PRIMARY — read this first)
- **TDD**: `docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-tdd.md` — PW-ERR-001 through PW-ERR-004, PW-DGN-001, PW-DGN-002, PW-DGN-004 (PRIMARY)
- **Strategy**: `docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-strategy.md` — Phase 9 (ordering context only)

## Context

Create the read-only error browse surface and daemon diagnostics page. Errors browse-only in Phase 9; action closure (resolve/rerun) lands in Phase 11.

## Language & Quality Gates

- **Language:** `typescript`
- **Quality gate commands:**
  - `bunx playwright test e2e/errors.spec.ts e2e/daemon-status.spec.ts`

## Files to Create

- `apps/desktop/src/pages/Errors.tsx` — Unresolved error manifest list with drill-down
- `apps/desktop/src/pages/DaemonStatus.tsx` — Daemon diagnostics with restart control
- `apps/desktop/e2e/errors.spec.ts` — Error queue E2E tests (browse-only subset)
- `apps/desktop/e2e/daemon-status.spec.ts` — Daemon status E2E tests

## Spec Requirements (from §1.9)

**L2-FR-PAGE.5 — Errors Page Is the Operator Error Work Queue**
**L2-FR-PAGE.6 — Daemon Status Exposes Operator Diagnostics**
**L2-FR-PAGE.12 — `Errors.tsx` Has an Explicit Queue and Resolution Contract**
**L2-FR-PAGE.13 — `DaemonStatus.tsx` Has an Explicit Diagnostic Action Contract**

## Tests to Pass

- **PW-ERR-001**: Errors page lists unresolved manifests with severity, stage, summary, session linkage
- **PW-ERR-002**: Empty unresolved set renders explicit empty-state copy
- **PW-ERR-003**: Severity matrix: leaked/crash = critical; pipeline/source = warning
- **PW-ERR-004**: Drill-down renders sanitized markdown + escaped JSON context
- **PW-DGN-001**: Daemon status shows instance id, version, socket path, DB path, WAL bytes
- **PW-DGN-002**: Unavailable daemon shows descriptor-backed fallback + unavailable markers
- **PW-DGN-004**: Browse-only/limited contexts show readable diagnostics; mutating controls disabled

## Acceptance Criteria

1. Errors page renders unresolved manifests with severity derivation
2. DaemonStatus shows runtime diagnostics from `get_daemon_info()`
3. Descriptor-backed fallback when daemon unavailable
4. All tests pass: PW-ERR-001-004, PW-DGN-001, PW-DGN-002, PW-DGN-004

## Dependencies

- **Blocked by:** l2-05a, l2-05b, l2-07b, l2-08b, l2-08c
- **Blocks:** l2-11a
- **Phase:** 9 — Read-Only Operator Pages

---

## l2-10a: Session Registry And Buffer Stores

- type: task
- priority: P0
- labels: phase-10,typescript,stores
- parent: testproject1-dr6

### Description

## SOURCE OF TRUTH WARNING

This bead is a quick-reference derived from:
- **Spec**: `docs/strategy/layers/layer-2-single-operator-desktop.md` — §1.10 (PRIMARY — read this first)
- **TDD**: `docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-tdd.md` — TS-REG-001 through TS-REG-005, TS-BUF-001 through TS-BUF-004 (PRIMARY)
- **Strategy**: `docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-strategy.md` — Phase 10 (ordering context only)

## Context

Create the purpose-built external stores for live session state: `SessionRegistry` (metadata, `lastSeq`, gap state, archive fence) and `SessionBuffer` (PTY bytes or contract-mode frames with explicit gap markers). NOT TanStack Query.

## Language & Quality Gates

- **Language:** `typescript`
- **Quality gate commands:**
  - `bun run test`

## Files to Create

- `apps/desktop/src/stores/session-registry.ts` — External store for session metadata and replay anchors
- `apps/desktop/src/stores/session-registry.test.ts` — Session registry tests
- `apps/desktop/src/stores/session-buffer.ts` — Per-session buffer (PTY bytes or text frames)
- `apps/desktop/src/stores/session-buffer.test.ts` — Session buffer tests

## Spec Requirements (from §1.10)

**L2-FR-STATE.1 — Snapshot, Streaming, and Control-Plane State Stay Separated**
**L2-FR-STATE.2 — Session Output Never Lives as Raw React Component State**
- AC1: PTY buffers store bytes rather than pre-expanded React-friendly line arrays.
- AC2: Contract-mode history stores structured text/event frames with explicit gap markers.

**L2-FR-STATE.5 — Replay State Is First-Class in the Registry**
**L2-FR-STATE.7 — Session Buffer Has an Explicit Frame API**
**L2-FR-STATE.8 — `lastSeq` Persistence Is Restart-Aware**

## Key Types (from spec §1.10 — verbatim)

```typescript
type SessionRegistryApi = {
  getSnapshot(): SessionRegistryState
  subscribe(onStoreChange: () => void): () => void
  resetForDaemon(instanceId: string): void
  applySessionMeta(session: SessionListItem): void
  applyStreamFrame(sessionId: string, frame: SessionFrame): void
  markReplayResolved(sessionId: string, fromSeq: number, toSeq: number): void
}
```

## Tests to Pass

- **TS-REG-001**: Registry stores sessions by `daemonInstanceId + sessionId`; metadata upserts do not clobber unrelated sessions
- **TS-REG-002**: Stream frames advance `lastSeq` monotonically; out-of-order rejected/buffered
- **TS-REG-003**: Gap state records exact `fromSeq..toSeq`; replay resolution clears matching range only
- **TS-REG-004**: `archiveSeq` tracked; prevents historical handoff until snapshot fence satisfied
- **TS-REG-005**: Same-daemon reopen preserves `lastSeq`; daemon instance change clears anchors
- **TS-BUF-001**: PTY buffer stores bytes not lines; tail extraction derived without mutating authoritative storage
- **TS-BUF-002**: Contract buffer stores text/event frames with gap markers; frame kind preserved through replay
- **TS-BUF-003**: Replay merge inserts by original `seq` without duplicating present frames
- **TS-BUF-004**: Bounded pruning retains tail context and visible unresolved-gap markers

## Acceptance Criteria

1. `SessionRegistry` uses `useSyncExternalStore` pattern
2. PTY buffers store bytes, not lines
3. Gap state is first-class with explicit `fromSeq..toSeq`
4. `lastSeq` persistence is restart-aware
5. All tests TS-REG-001-005 and TS-BUF-001-004 pass

## Dependencies

- **Blocked by:** l2-04c, l2-07b, l2-08c
- **Blocks:** l2-10b
- **Phase:** 10 — Streaming And Session UX Lane

---

## l2-10b: Session Hooks And List Page

- type: task
- priority: P0
- labels: phase-10,typescript,pages
- parent: testproject1-dr6

### Description

## SOURCE OF TRUTH WARNING

This bead is a quick-reference derived from:
- **Spec**: `docs/strategy/layers/layer-2-single-operator-desktop.md` — §1.9, §1.10 (PRIMARY — read this first)
- **TDD**: `docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-tdd.md` — PW-SES-001, PW-SES-002, PW-SES-005, PW-SES-011, CI-L2-005 (PRIMARY)
- **Strategy**: `docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-strategy.md` — Phase 10 (ordering context only)

## Context

Create session hooks (`useSessionMeta`, `useSessionTail`, `useSessionTimeline`) and the Sessions list page. Hooks subscribe by slice via `useSyncExternalStore`.

## Language & Quality Gates

- **Language:** `typescript`
- **Quality gate commands:**
  - `bun run test`
  - `bunx playwright test e2e/sessions.spec.ts`

## Files to Create

- `apps/desktop/src/hooks/use-daemon.ts` — Session hooks via `useSyncExternalStore`
- `apps/desktop/src/pages/Sessions.tsx` — Active/recent session list with metadata
- `apps/desktop/e2e/sessions.spec.ts` — Session UX E2E tests

## Spec Requirements (from §1.9, §1.10)

**L2-FR-STATE.3 — Session Hooks Subscribe by Slice**
**L2-FR-PAGE.3 — Sessions Page Surfaces Active and Recent Sessions**
**L2-FR-PAGE.10 — `Sessions.tsx` Has an Explicit Filter and Drill-Down Contract**
**L2-FR-CH.2 — Subscription Cost Is Bounded by View Intent**

## Tests to Pass

- **PW-SES-001**: Sessions page renders active/recent sessions with lifecycle phase, mode, duration, cleanup, checkpoints
- **PW-SES-002**: Pre-active failure still appears with canonical phase, error type, duration
- **PW-SES-005**: Selecting one session attaches one full stream; list continues metadata-only
- **PW-SES-011**: Retry creates new job/session; original remains queryable; new appears after fence
- **CI-L2-005**: `bun run build` and `bun run test` exit 0

## Acceptance Criteria

1. `useSessionMeta`, `useSessionTail`, `useSessionTimeline` subscribe by slice
2. Sessions page renders from `list_sessions()` with proper metadata
3. Pre-active failures appear with canonical state
4. Subscription cost bounded by view intent

## Dependencies

- **Blocked by:** l2-10a
- **Blocks:** l2-10c
- **Phase:** 10 — Streaming And Session UX Lane

---

## l2-10c: Session Viewer And Replay

- type: task
- priority: P0
- labels: phase-10,typescript,pages,streaming
- parent: testproject1-dr6

### Description

## SOURCE OF TRUTH WARNING

This bead is a quick-reference derived from:
- **Spec**: `docs/strategy/layers/layer-2-single-operator-desktop.md` — §1.6, §1.9 (PRIMARY — read this first)
- **TDD**: `docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-tdd.md` — PW-SES-003, PW-SES-004, PW-SES-006 through PW-SES-010, PW-SES-012, RS-DBR-008 through RS-DBR-010, AG-HLT-001 through AG-HLT-005 (PRIMARY)
- **Strategy**: `docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-strategy.md` — Phase 10 (ordering context only)

## Context

Create the SessionViewer with dual mode (live terminal for PTY, transcript for contract), stop/input/resize controls, gap recovery, and live-to-historical handoff. Also the daemon-side streaming bridge extensions.

## Language & Quality Gates

- **Language:** `mixed` (TypeScript + Rust streaming bridge)
- **Quality gate commands:**
  - `bun run test`
  - `bunx playwright test e2e/sessions.spec.ts`
  - `cargo test -p daemon --test agent_health_detection`
  - `cargo test --test daemon_bridge`

## Files to Create

- `apps/desktop/src/pages/SessionViewer.tsx` — Live terminal/transcript viewer with dual mode
- `crates/daemon/tests/agent_health_detection.rs` — Health detection agent E2E tests

## Spec Requirements (from §1.6)

**L2-FR-CH.1 through L2-FR-CH.15** — Full session streaming contract. See spec §1.6 for all 15 Channel requirements.

**L2-FR-PAGE.4 — Session Viewer Supports Monitoring and Intervention**
- AC1: Viewer renders live PTY or transcript history according to execution mode.
- AC2: Interactive controls available only when session is attached and mode supports them.
- AC3: Gap markers, reconnect state, and browser-only auth interventions surfaced explicitly.

**L2-FR-PAGE.11 — `SessionViewer.tsx` Has an Explicit Replay and Intervention Contract**

## Tests to Pass

- **PW-SES-003**: PTY session opens in live-terminal mode; bytes stream in sequence order
- **PW-SES-004**: Contract-mode opens in transcript mode; input controls disabled
- **PW-SES-006**: `stop_session(graceful)` returns accepted ack; viewer shows terminating/terminal
- **PW-SES-007**: Forced stop yields `cleanupVerdict = forced`; no fake "clean exit"
- **PW-SES-008**: Typing into SessionViewer sends bytes to PTY; no duplicate bytes
- **PW-SES-009**: Detached/contract/terminal session disables input/resize
- **PW-SES-010**: `resize_session()` returns accepted ack for attached PTY only
- **PW-SES-012**: Gap → replay → recovery; unresolved loss visible if partial
- **RS-DBR-008**: `OutputFrame` → `SessionFrame` projection; `ControlEvent` → `StructuredSessionEvent` projection
- **RS-DBR-009**: Legacy handshake compatibility
- **RS-DBR-010**: Lifecycle stream: created → spawned → active → terminating → exited → reaped → archived
- **AG-HLT-001**: Stall detection yields `AGENT_STALLED`
- **AG-HLT-002**: PTY auth challenge yields structured event
- **AG-HLT-003**: Browser-only challenge emits actionable reveal-profile-path
- **AG-HLT-004**: Cleanup escalation records verdicts correctly
- **AG-HLT-005**: Pre-active crash persists canonical phase history

## Acceptance Criteria

1. Dual viewer: live terminal for PTY, transcript for contract mode
2. Stop/input/resize controls gated by session state
3. Gap recovery via `get_session_history()` with explicit unresolved-gap markers
4. Live-to-historical handoff obeys `archiveSeq` fence
5. All tests pass: PW-SES-003-012, RS-DBR-008-010, AG-HLT-001-005

## Dependencies

- **Blocked by:** l2-10b
- **Blocks:** l2-12b
- **Phase:** 10 — Streaming And Session UX Lane

---

## l2-11a: Decision Migration And Handler

- type: task
- priority: P0
- labels: phase-11,mixed,decisions
- parent: testproject1-dr6

### Description

## SOURCE OF TRUTH WARNING

This bead is a quick-reference derived from:
- **Spec**: `docs/strategy/layers/layer-2-single-operator-desktop.md` — §3.2, §3.3 (PRIMARY — read this first)
- **TDD**: `docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-tdd.md` — PW-DEC-001 through PW-DEC-008, PW-FEN-001 through PW-FEN-007 (PRIMARY)
- **Strategy**: `docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-strategy.md` — Phase 11 (ordering context only)

## Context

Create the `pending_operator_intents` SQLite migration and the `capture_decision` daemon handler. This is the first operator mutation surface: local intent queue for approve/reject/watch decisions with reviewed-basis metadata and staleness detection.

## Language & Quality Gates

- **Language:** `mixed` (Rust handler + SQL migration + TypeScript page wiring)
- **Quality gate commands:**
  - `bunx playwright test e2e/decisions.spec.ts e2e/commit-fence.spec.ts`
  - `cargo test -p daemon --test agent_supplier_enrichment`

## Files to Create

- `migrations/0004_l2_pending_operator_intents.sql` — Decision overlay table from L0 §2.5.8 DDL
- `crates/daemon/src/ipc/handlers/decisions.rs` — `capture_decision` handler
- `apps/desktop/e2e/decisions.spec.ts` — Decision capture E2E tests
- `apps/desktop/e2e/commit-fence.spec.ts` — Commit-fence mutation visibility tests
- `crates/daemon/tests/agent_supplier_enrichment.rs` — Enrichment agent E2E tests

## Spec Requirements (from §3.2)

**L2-FR-DEC.1 — Each Local Decision Captures the Reviewed Basis**
- AC1: Every decision intent records: `process_id`, `desired_disposition`, `reviewed_basis_hash`, `reviewed_manifest_hash`, `reviewed_at_stage_run_id`, `reviewed_at`, `base_remote_revision`, `idempotency_key`, `status`, `superseded_by_intent_id`, `queued_at`.
- AC2: `reviewedBasisHash` copied from daemon-authored pipeline assessment, NOT recomputed in UI.

**L2-FR-DEC.2 — Latest-Per-Process Coalescing Is Explicit**
- AC1: Only the latest queued intent per process is the current local decision.
- AC2: Newer decision supersedes via `superseded_by_intent_id`.

**L2-FR-DEC.3 — Decision Staleness Is Hash-Based and Operator-Visible**
- AC1: Desktop compares `reviewedBasisHash` against `pipeline_assessments.decisionBasisHash`; mismatch = stale.

**L2-FR-DEC.4 — `pending_operator_intents` Reuses the Layer 0 DDL Exactly**
**L2-FR-DB.6 — `capture_decision()` Persists the Operator Intent**

## Tests to Pass

- **PW-DEC-001**: Approve returns committed ack; `pending_operator_intents` row stores all required fields
- **PW-DEC-002**: Watch action persists and shows after commit fence clears
- **PW-DEC-003**: Approve then reject creates two rows; first superseded; UI shows reject
- **PW-DEC-004**: Re-affirming same disposition writes new intent with new key
- **PW-DEC-005**: Decision on hash A becomes stale after rerun produces hash B
- **PW-DEC-006**: Rerun with same hash does NOT mark stale
- **PW-DEC-007**: Lost ack retry with same idempotency key returns same ack; one row
- **PW-DEC-008**: Pre-fence shows old; post-fence shows new; no ghost state
- **PW-FEN-001**: `trigger_ingestion()` fence semantics
- **PW-FEN-002**: `trigger_enrichment()` fence semantics
- **PW-FEN-003**: Capture-decision fence preserves old until clears
- **PW-FEN-004**: Resolve error fence
- **PW-FEN-005**: Retry job fence
- **PW-FEN-006**: Rollback advances no fence; archive handoff obeys archiveSeq
- **PW-FEN-007**: Concurrent mutations ordered `visibleAfterSeq` values
- **AG-ENR-001 through AG-ENR-004**: Enrichment happy path, confidence, scoped rerun, partial data

## Acceptance Criteria

1. `pending_operator_intents` table matches L0 §2.5.8 DDL exactly
2. `capture_decision` handler writes reviewed-basis metadata
3. Latest-per-process coalescing via `superseded_by_intent_id`
4. Decision staleness detected via `decisionBasisHash` comparison
5. All commit-fence tests verify pre/post visibility semantics
6. All tests pass: PW-DEC-001-008, PW-FEN-001-007, AG-ENR-001-004

## Dependencies

- **Blocked by:** l2-05b, l2-08b, l2-09a, l2-09b
- **Blocks:** l2-11b
- **Phase:** 11 — Decisions, Coverage, Staleness, And Operator Validation Package

---

## l2-11b: Error Actions And Page Wiring

- type: task
- priority: P0
- labels: phase-11,mixed,errors
- parent: testproject1-dr6

### Description

## SOURCE OF TRUTH WARNING

This bead is a quick-reference derived from:
- **Spec**: `docs/strategy/layers/layer-2-single-operator-desktop.md` — §1.5 (PRIMARY — read this first)
- **TDD**: `docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-tdd.md` — PW-ERR-005, PW-ERR-006, AG-ERR-001 through AG-ERR-004 (PRIMARY)
- **Strategy**: `docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-strategy.md` — Phase 11 (ordering context only)

## Context

Create `handlers/errors.rs` for `resolve_error` and `rerun_from_error` daemon commands. Wire the resolve/rerun actions into the Errors page. This closes the error action surface.

## Language & Quality Gates

- **Language:** `mixed` (Rust handlers + TypeScript page action wiring)
- **Quality gate commands:**
  - `bunx playwright test e2e/errors.spec.ts`
  - `cargo test -p daemon --test agent_error_manifest`

## Files to Create

- `crates/daemon/src/ipc/handlers/errors.rs` — `resolve_error` and `rerun_from_error` handlers
- `crates/daemon/tests/agent_error_manifest.rs` — Error manifest agent E2E tests

## Spec Requirements (from §1.5)

**L2-FR-DB.7 — `resolve_error()` Closes One Error Manifest**
- AC1: Successful `resolve_error(error_manifest_id)` returns committed `MutationAck`.
- AC2: Repeating resolve against already-resolved error is idempotent.

**L2-FR-DB.13 — `rerun_from_error()` Restarts Work From a Failed Stage**
- AC1: Successful `rerun_from_error(error_manifest_id)` returns committed `MutationAck` with new `jobId`.
- AC2: Rerun starts from the failed stage, not arbitrary earlier stage.
- AC3: Unknown or non-rerunnable error manifests surface explicit refusal.

## Tests to Pass

- **PW-ERR-005**: Resolve returns committed ack; row removed only after commit fence; repeated resolve idempotent
- **PW-ERR-006**: Rerun-from-error creates new job from failed stage; original stays in history
- **AG-ERR-001**: Skill failure creates error manifest with process/job/session linkage
- **AG-ERR-002**: Contract-invalid path produces error manifest with typed `error_type`
- **AG-ERR-003**: Resolve and rerun keeps original queryable; new job from failed stage
- **AG-ERR-004**: Leaked cleanup = critical; pipeline/source = warning; severity stable for UI

## Acceptance Criteria

1. `resolve_error` closes error manifests idempotently
2. `rerun_from_error` creates new job from failed stage
3. Error manifest creation covers skill failure, contract-invalid, suspect output
4. Severity derivation stable for UI consumers
5. All tests pass: PW-ERR-005-006, AG-ERR-001-004

## Dependencies

- **Blocked by:** l2-11a
- **Blocks:** l2-11c
- **Phase:** 11 — Decisions, Coverage, Staleness, And Operator Validation Package

---

## l2-11c: Operator Validation Checklist

- type: task
- priority: P1
- labels: phase-11,docs,milestone
- parent: testproject1-dr6

### Description

## SOURCE OF TRUTH WARNING

This bead is a quick-reference derived from:
- **Spec**: `docs/strategy/layers/layer-2-single-operator-desktop.md` — §4.3 (PRIMARY — read this first)
- **TDD**: `docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-tdd.md` — Manual Gate (PRIMARY)
- **Strategy**: `docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-strategy.md` — Phase 11 (ordering context only)

## Context

Create the manual operator weekly validation checklist for the 5-7 day milestone run. This is the "single operator runs system daily for 1 week" acceptance gate.

## Language & Quality Gates

- **Language:** `markdown` (documentation only)
- **Quality gate commands:** none (manual checklist)

## Files to Create

- `docs/testing/l2-operator-weekly-validation.md` — Manual operator validation checklist

## Spec Requirements (from §4.2, §4.3)

> **DoD #12:** Single operator runs daily for 1 week and reports genuine opportunities surfaced. Manual: operator validation over 5-7 working days (Tauri desktop).

The checklist must cover: daily triggers, surfaced opportunities, accept/reject/watch actions, and observed value.

## Acceptance Criteria

1. Checklist exists and is aligned with the current phase graph
2. Covers: daily ingestion triggers, opportunity review, decision capture, error handling
3. Ready for the 5-7 day milestone run

## Dependencies

- **Blocked by:** l2-11b
- **Blocks:** l2-12b
- **Phase:** 11 — Decisions, Coverage, Staleness, And Operator Validation Package

---

## l2-12a: Contracts Finder Skill Package

- type: task
- priority: P0
- labels: phase-12,skill,contracts-finder
- parent: testproject1-dr6

### Description

## SOURCE OF TRUTH WARNING

This bead is a quick-reference derived from:
- **Spec**: `docs/strategy/layers/layer-2-single-operator-desktop.md` — §1.13 (PRIMARY — read this first)
- **TDD**: `docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-tdd.md` — AG-CF-001 through AG-CF-006 (PRIMARY)
- **Strategy**: `docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-strategy.md` — Phase 12 (ordering context only)

## Context

Create the Contracts Finder ingestion skill package, mirroring the FTS skill structure exactly. This is an independent daemon/skill lane — NO dependencies on desktop beads. Only depends on the daemon command lane (Phase 4).

## Language & Quality Gates

- **Language:** `mixed` (skill playbook + Rust daemon test)
- **Quality gate commands:**
  - `cargo test -p daemon --test agent_contracts_finder_ingestion`

## Files to Create

- `skills/procurement-ingestion/contracts-finder/SKILL.md` — CF ingestion skill playbook
- `skills/procurement-ingestion/contracts-finder/checkpoint.schema.json` — Checkpoint schema
- `skills/procurement-ingestion/contracts-finder/result.schema.json` — Result schema
- `skills/procurement-ingestion/contracts-finder/canary.inputs.jsonl` — Canary test inputs
- `skills/procurement-ingestion/contracts-finder/golden.assertions.yaml` — Golden assertions
- `crates/daemon/tests/agent_contracts_finder_ingestion.rs` — CF agent E2E tests

## Spec Requirements (from §1.13)

**L2-FR-CF.1 — Contracts Finder Ingestion Is a First-Class Layer 2 Source Skill**
- AC1: Layer 2 adds a skill at `skills/procurement-ingestion/contracts-finder/`.
- AC2: The skill declares a `pty_interactive` interaction profile.
- AC3: Cursor handling is rate-limit-aware and stable for repeated daily ingestion.

**L2-FR-CF.2 — Contracts Finder Skill Packaging Matches the FTS Standard**
- AC1: Same structural sections as FTS: use/do-not-use, inputs, outputs, failure codes, resume strategy, canary, run-manifest instructions.
- AC2: Checkpoint-safe continuation semantics.
- AC3: Cursor overlap/backtrack behavior documented explicitly.

**L2-FR-CF.3 — Contracts Finder Canary and Manifest Rules Are Mandatory**
- AC1: Lightweight canary before full retrieval; fail fast on drift.
- AC2: Same run-manifest frontmatter and narrative sections as FTS.
- AC3: Source drift and rate-limit issues surface as explicit failure codes.

## Tests to Pass

- **AG-CF-001**: Skill directory contains SKILL.md, checkpoint.schema.json, result.schema.json, canary.inputs.jsonl, golden.assertions.yaml
- **AG-CF-002**: Canary validates recent response shape, locale, currency, rate-limit expectations
- **AG-CF-003**: Incremental run emits notices + checkpoint; `continuation_required` when time budget expires
- **AG-CF-004**: Cursor backtrack prevents skipped notices; no duplicate durable notice ids
- **AG-CF-005**: Run manifest includes required YAML frontmatter and narrative sections
- **AG-CF-006**: Source drift, locale drift, soft block, CAPTCHA, stall emit explicit failure codes

## Acceptance Criteria

1. CF skill mirrors FTS five-file skeleton exactly
2. Canary validates before full retrieval
3. Incremental cursor with backtrack window
4. Run manifest matches FTS standard
5. All tests AG-CF-001 through AG-CF-006 pass

## Dependencies

- **Blocked by:** l2-04b (daemon command lane only — independent of desktop beads)
- **Blocks:** l2-12b
- **Phase:** 12 — Contracts Finder Skill Lane, Hardening, And Milestone Gate

---

## l2-12b: Bridge-Failure Hardening, Native Parity, And Milestone Gate

- type: task
- priority: P0
- labels: phase-12,mixed,hardening,milestone
- parent: testproject1-dr6

### Description

## SOURCE OF TRUTH WARNING

This bead is a quick-reference derived from:
- **Spec**: `docs/strategy/layers/layer-2-single-operator-desktop.md` — §1.5, §1.7, §1.8, §1.16, §2.3, §4.2 (PRIMARY — read this first)
- **TDD**: `docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-tdd.md` — PW-BRD-001 through PW-BRD-006, PW-SEC-001 through PW-SEC-008, PW-DGN-003, TAU-INT-001 through TAU-INT-012, CI-L2-006 (PRIMARY)
- **Strategy**: `docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-strategy.md` — Phase 12 (ordering context only)

## Context

Final convergence: bridge-failure hardening (unhappy paths), security boundary verification, native Tauri parity checks, and cumulative milestone gate. This closes Layer 2.

## Language & Quality Gates

- **Language:** `mixed` (TypeScript E2E + Tauri verification + Rust)
- **Quality gate commands (cumulative):**
  - `cargo check --all-targets`
  - `cargo clippy --all-targets -- -D warnings`
  - `cargo fmt --check`
  - `bun run build`
  - `bun run lint`
  - `bun run test`
  - `bunx playwright test`
  - `cargo test -p daemon --test agent_contracts_finder_ingestion`

## Files to Create

- `apps/desktop/e2e/bridge-failures.spec.ts` — Unhappy-path browser hardening
- `apps/desktop/e2e/security.spec.ts` — Cross-surface security hardening
- `apps/desktop/e2e/tauri-integration.spec.ts` — Native Tauri lifecycle and parity

## Spec Requirements

**L2-FR-DB.14 — Bridge Failure Paths Are Explicit and Operator-Visible**
**L2-FR-SEC.1 — Agent-Authored Content Is Sanitized Before Rendering** (5 ACs)
**L2-FR-SEC.2 — Desktop Capability Exposure Is Least-Privilege**
**L2-FR-BOOT.3 through L2-FR-BOOT.11** — All bootstrap modes and recovery paths
**L2-FR-A11Y.1 — Session Accessibility Prioritizes State Changes Over Raw Stream Noise**

## Tests to Pass

**Bridge Failures:**
- **PW-BRD-001**: Transport unavailable → explicit failure; browse-only handling; no ghost row
- **PW-BRD-002**: Conflict refusal shown verbatim; retry CTA respects `retryable`
- **PW-BRD-003**: Invalid params surface `invalid_params`; no partial mutation
- **PW-BRD-004**: Stop/input/resize on terminal session → `already_terminal` no-op
- **PW-BRD-005**: Lost ack retried with same key → same durable outcome
- **PW-BRD-006**: Reservation conflict surfaces through session/error surfaces

**Security:**
- **PW-SEC-001**: Hostile HTML payloads render as inert sanitized content
- **PW-SEC-002**: Dangerous link schemes stripped in manifest and error views
- **PW-SEC-003**: Agent error text renders as escaped text, not raw HTML
- **PW-SEC-004**: OSC 8 hyperlink escapes stripped from SessionViewer
- **PW-SEC-005**: OSC 52 clipboard escapes stripped; no clipboard side effect
- **PW-SEC-006**: Untrusted strings render as text nodes across all surfaces
- **PW-SEC-007**: 10MB manifest and 100K-line session output bounded
- **PW-SEC-008**: RTL override and zero-width joiners remain inert

**Daemon Status:**
- **PW-DGN-003**: Restart CTA visible only in allowed modes; ack before disconnect

**Tauri Integration:**
- **TAU-INT-001**: App close does not kill daemon; daemon PID alive after UI close
- **TAU-INT-002**: "Stop daemon and quit" performs clean shutdown
- **TAU-INT-003**: Startup auto-launches daemon; one detached launch attempt
- **TAU-INT-004**: Readable DB + unavailable daemon → browse-only
- **TAU-INT-005**: Schema/protocol mismatch → incompatible mode
- **TAU-INT-006**: Unreadable storage → recovery-required
- **TAU-INT-007**: Degraded/starting → limited mode, not crash
- **TAU-INT-008**: Total mode matrix covers all reachable combinations
- **TAU-INT-009**: Same-daemon reconnect preserves `lastSeq` anchors
- **TAU-INT-010**: Daemon crash invalidates PTY resumption
- **TAU-INT-011**: Native invoke/query/stream parity with dev bridge
- **TAU-INT-012**: Production build excludes dev bridge; native transport works

**CI Gate:**
- **CI-L2-006**: `bunx playwright test --list` resolves all L2 browser specs

## Acceptance Criteria

1. All bridge failure paths are explicit and operator-visible
2. All security boundary tests pass (HTML sanitization, link stripping, OSC escapes)
3. All 12 Tauri integration tests pass including native lifecycle
4. Full cumulative quality gate passes
5. CF skill mirrors FTS skeleton exactly
6. Weekly operator checklist is ready
7. All tests pass: PW-BRD-001-006, PW-SEC-001-008, PW-DGN-003, TAU-INT-001-012, CI-L2-006

## Dependencies

- **Blocked by:** l2-09a, l2-09b, l2-10c, l2-11c, l2-12a
- **Blocks:** none (terminal bead — Layer 2 milestone gate)
- **Phase:** 12 — Contracts Finder Skill Lane, Hardening, And Milestone Gate
