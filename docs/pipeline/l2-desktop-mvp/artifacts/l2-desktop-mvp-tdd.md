# L2 Desktop MVP — TDD Specification

## 1. Scope

**Primary source of truth:** `docs/strategy/layers/layer-2-single-operator-desktop.md`

**Supporting context:**
- `docs/strategy/layer-strategy.md` §9-§11
- `docs/strategy/cross-reference-index.md`
- `docs/strategy/layers/layer-0-gold-set-evaluation.md` §2.5.8
- `docs/pipeline/l1-daemon-core/artifacts/l1-daemon-core-tdd.md`
- `docs/pipeline/l1-daemon-core/artifacts/l1-daemon-core-bootstrap.md`
- `AGENTS.md` §Toolchain, §Compiler Checks

**Purpose of this document:** define exactly what tests must exist before Layer 2 implementation begins.

**Coverage target:** all `118` unique `L2-FR-*` requirement IDs in the validated Layer 2 spec map to at least one stable test ID in this document.

**Planned inventory:** `158` total checks
- `7` static CI gates
- `61` Tier 1 Playwright browser E2E checks
- `12` Tier 2 Tauri verification checks
- `78` Tier 3 mixed backend/package/agent checks

**Important constraint:** Layer 2 is a mixed-stack layer. The TDD must cover:
- Browser E2E against `localhost:5173` through the dev bridge
- Full Tauri verification for desktop-only lifecycle behaviors
- Rust integration against real SQLite/IPC surfaces
- TypeScript package tests for shared UI, manifest rendering, coverage math, and desktop stores
- Real agent/skill E2E where the spec requires live runtime behavior

## 2. Non-Negotiable Test Principles

- Use Bun only for TypeScript workspaces and Cargo only for Rust workspaces.
- Keep the Layer 2 runtime split intact in tests: QueryService is read-only, DaemonBridge is a thin RPC proxy, and live session transport is a separate stream surface.
- Treat the commit fence as a release-gate invariant. Every mutation-flow test must prove both pre-fence old state and post-fence visible state.
- Do not mock SQLite for QueryService or bridge contract tests. Use temp-file SQLite with `0001_sqlite_state_initial.sql`, `0003_sqlite_runtime.sql`, and the L2 overlay migration that adds `pending_operator_intents`.
- Do not test session streaming by scraping final transcript text alone. Streaming tests must assert typed `SessionFrame` / `StructuredSessionEvent` shape, sequence monotonicity, gap semantics, and replay behavior.
- Do not let Playwright tests bypass QueryService or DaemonBridge. Browser E2E must call the same app-level commands and payloads that the Tauri webview uses.
- Playwright tests must use human-readable selectors (`getByRole`, accessible names, visible text, stable route params). CSS-class selectors are not acceptable for normative assertions.
- Playwright tests must wait on observable data conditions (`snapshotSeq`, `visibleAfterSeq`, role/text appearance, WebSocket event completion), not raw `waitForTimeout`, except when explicitly simulating backpressure windows inside the harness.
- Keep agent-authored content untrusted in every test surface. Manifest, error, and terminal tests must use adversarial fixtures, not only benign prose.
- Do not use `.skip`, `test.fixme`, empty bodies, existence-only assertions, or vague “renders correctly” checks.
- Any test claiming Tauri-specific behavior must run through the native shell, not only through the browser dev bridge.
- `#![forbid(unsafe_code)]` remains mandatory for all Rust production and test code introduced by Layer 2.

## 3. CI Gates And Runtime Tiers

Static gates are not counted as Tier 1/2/3 runtime tiers. They are preconditions for any runnable suite.

| Lane | Budget | Cadence | Includes |
|------|--------|---------|----------|
| Static Gates | `< 3 min` | every commit | Rust build/lint/format, Bun lint/build/test entrypoints, no-unsafe policy, artifact inventory |
| `T1` | `< 10 min` | every commit / every implementation iteration | Playwright browser E2E on `localhost:5173` via dev bridge |
| `T2` | `< 20 min` | pre-merge / milestone gate | Native Tauri verification, desktop-shell lifecycle, dev-bridge parity checks |
| `T3` | `< 30 min` | CI + nightly as needed | Rust integration, package tests, real agent E2E, skill canaries, replay/error/cleanup contracts |
| Manual Gate | operator time | milestone signoff | five-to-seven-day single-operator desktop run diary |

## 4. Shared Test Infrastructure

| Harness / Fixture | Scope | Purpose |
|------------------|-------|---------|
| `L2StateDbFixture` | `T1` / `T2` / `T3` | Temp SQLite seeded with L0 + L1 migrations plus the L2 `pending_operator_intents` overlay; exposes `commit_clock (id = 1, commit_seq)`, `pipeline_assessments`, `agent_session_logs`, exact `pending_operator_intents` column contract, and artifact-path helpers |
| `L2DaemonHarness` | `T1` / `T2` / `T3` | Boots the real daemon with temp runtime paths, UDS socket, descriptor file, artifact root, and log capture; can force disconnects, restarts, busy states, and degraded health |
| `L2QueryServiceHarness` | `T3` | Opens read-only connections with `PRAGMA query_only = ON`, configurable busy readers, and artifact-availability toggles |
| `L2BridgeHarness` | `T2` / `T3` | Sends typed `hello`, `command`, `subscribe`, and `unsubscribe` envelopes over the authenticated local IPC channel; can drop or duplicate acks deterministically |
| `L2StreamHarness` | `T1` / `T2` / `T3` | Publishes `SessionFrame` sequences, enforces bounded live buffers, injects gaps, and simulates missing chunk replay ranges |
| `PlaywrightSeedKit` | `T1` | Seeds deterministic process/session/error datasets into SQLite and exposes test-only daemon hooks for rerun, gap, restart, and refusal scenarios |
| `TauriLaunchHarness` | `T2` | Launches `bun tauri dev` or a built Tauri binary, controls main-window close vs full app quit, and tracks daemon PID / descriptor reuse |
| `SecurityFixtureFactory` | `T1` / `T3` | Produces hostile Markdown, malicious links, hostile error messages, and dangerous ANSI/OSC terminal payloads |
| `CoverageMathFixture` | `T1` / `T3` | Mixed-contract lot/candidate fixtures with addressable-value splits, mandatory lots, and value-share variations |
| `ManifestFixture` | `T1` / `T3` | Deterministic manifest inputs plus expected heading order, empty-state prose, and formatting snapshots |
| `SessionStoreFixture` | `T1` / `T3` | Deterministic registry/buffer state mutations for replay, gap, and restart-aware `lastSeq` persistence |
| `ContractsFinderSkillFixture` | `T3` | Canonical `skills/procurement-ingestion/contracts-finder/` skeleton with `SKILL.md`, schemas, `canary.inputs.jsonl`, and `golden.assertions.yaml` |
| `InheritedL1GateContext` | Static / `T3` | Re-runs the relevant existing L1 daemon tests (`agent_procurement_ingestion.rs`, `ipc_handshake.rs`, `agent_pty_session.rs`) that Layer 2 depends on but does not replace |

## 5. Planned File Inventory

| Surface | Count | Notes |
|--------|-------|-------|
| Static gates | 7 | repository-level commands and policies |
| Playwright browser E2E | 8 files / 61 tests | primary UI lane at `localhost:5173` |
| Tauri verification | 1 file / 12 tests | native-shell-only behaviors |
| Rust integration | 2 files / 22 tests | QueryService + DaemonBridge contract coverage |
| TypeScript unit/package tests | 7 files / 37 tests | shared UI, manifests, coverage, stores, transport |
| Agent E2E (Rust) | 4 files / 19 tests | Contracts Finder, enrichment, health detection, error manifests |
| Manual operator gate | 1 checklist | milestone signoff, not part of automated count |

## 6. Static CI Gates

| Test ID | Tier | Fixtures | Exact Assertions | Requirements |
|--------|------|----------|------------------|-------------|
| `CI-L2-001` | Static | repo workspace | `cargo check --all-targets` exits `0` for the root workspace and the future desktop-side Rust crate; no Layer 2 Rust target breaks the existing daemon build | `L2-FR-BOOT.1`, `L2-FR-DB.15` |
| `CI-L2-002` | Static | repo workspace | `cargo clippy --all-targets -- -D warnings` exits `0`; no desktop bridge or QueryService code ships with warnings | `L2-FR-BOOT.1`, `L2-FR-SEC.2` |
| `CI-L2-003` | Static | repo workspace | `cargo fmt --check` exits `0`; Rust bootstrap, QueryService, bridge, and tests remain formatted | `L2-FR-BOOT.1` |
| `CI-L2-004` | Static | Bun workspace root | `bun install` resolves all declared workspaces; `bun run lint` exits `0`; no TypeScript surface imports desktop-only code into shared packages | `L2-FR-DEV.3`, `L2-FR-SEC.2`, `L2-FR-UI.1` |
| `CI-L2-005` | Static | Bun workspace root | `bun run build` and `bun run test` exit `0`; package tests and desktop unit tests are runnable from the same workspace root | `L2-FR-PAGE.7`, `L2-FR-STATE.4`, `L2-FR-UI.8` |
| `CI-L2-006` | Static | desktop e2e config | `bunx playwright test --list` resolves the Layer 2 browser specs, uses the configured dev server, and exposes no production-only transport assumptions | `L2-FR-DEV.1`, `L2-FR-DEV.4` |
| `CI-L2-007` | Static | desktop Rust crate + tests | every `apps/desktop/src-tauri/src/*.rs` and `apps/desktop/src-tauri/tests/*.rs` file starts from a `#![forbid(unsafe_code)]` policy and contains no `unsafe` blocks | `L2-FR-BOOT.1`, `L2-FR-SEC.2` |

## 7. Detailed Test Catalog

### 7.1 `apps/desktop/e2e/contracts.spec.ts`

| Test ID | Tier | Fixtures | Exact Assertions | Requirements |
|--------|------|----------|------------------|-------------|
| `PW-CON-001` | `T1` | `PlaywrightSeedKit.processes.multiPage` | list page renders 25 rows from one `list_processes()` envelope; first row shows title, buyer, deadline, badges, and decision summary; no per-row fallback fetch occurs | `L2-FR-PAGE.1`, `L2-FR-QS.4`, `L2-FR-UI.2` |
| `PW-CON-002` | `T1` | `PlaywrightSeedKit.processes.empty` | empty query renders explicit empty-state copy; UI keeps `hasMore = false` and `nextCursor = null`; no synthetic warning banner is shown | `L2-FR-PAGE.1`, `L2-FR-QS.4`, `L2-FR-PAG.2` |
| `PW-CON-003` | `T1` | `PlaywrightSeedKit.processes.multiPage` | next-page navigation uses cursor, not offset; page 2 contains no duplicate `processId`; last page exposes `nextCursor = null` without an extra probe | `L2-FR-PAG.1`, `L2-FR-PAG.2`, `L2-FR-PAGE.8` |
| `PW-CON-004` | `T1` | `PlaywrightSeedKit.processes.liveInsertStableSort` | inserting a new qualifying process while page 1 is open does not reorder already-visible rows; focus remains on the previously selected row; the new row appears only after a deliberate refresh window | `L2-FR-PAGE.1`, `L2-FR-PAG.1`, `L2-FR-A11Y.2` |
| `PW-CON-005` | `T1` | `PlaywrightSeedKit.processes.filterMatrix` | source, pipeline-state, stale-only, and decision filters normalize into the allowed filter shape only; invalid filter values are rejected before partial rendering; filtered row count matches seed expectations | `L2-FR-QS.11`, `L2-FR-PAGE.8` |
| `PW-CON-006` | `T1` | `PlaywrightSeedKit.processes.expiryAndDps` | expired non-DPS notice is absent from the opportunity queue; expired DPS notice remains visible; no active-row counter includes the filtered notice | `L2-FR-PAGE.1`, `L2-FR-QS.4` |
| `PW-CON-007` | `T1` | `PlaywrightSeedKit.detail.fullReviewPacket` | detail page renders notices, lots, candidates, quotes, landed costs, coverage, manifest, and unresolved errors from one detail fetch; key values are shown in integer-pence / bps-derived display formats | `L2-FR-PAGE.2`, `L2-FR-QS.5`, `L2-FR-MAN.1`, `L2-FR-MAN.3`, `L2-FR-COV.1` |
| `PW-CON-008` | `T1` | `PlaywrightSeedKit.detail.zeroLots` | zero-lot process shows explicit empty-state copy for lot analysis; decision controls and manifest sections remain present; payload keys are empty arrays rather than omitted | `L2-FR-PAGE.2`, `L2-FR-QS.5`, `L2-FR-MAN.4` |
| `PW-CON-009` | `T1` | `PlaywrightSeedKit.detail.noCandidates` | empty candidates/quotes render explicit placeholders; coverage shows zero/empty values rather than blanks; the page does not surface a transport failure | `L2-FR-PAGE.2`, `L2-FR-QS.5`, `L2-FR-COV.1`, `L2-FR-MAN.4` |
| `PW-CON-010` | `T1` | `CoverageMathFixture.valueWeightedBelowThreshold` | UI shows both lot-count coverage and value-weighted gate result; strong lot-count coverage does not suppress the failed value-weighted warning; operator sees “below threshold” copy | `L2-FR-COV.1`, `L2-FR-COV.3`, `L2-FR-COV.5`, `L2-FR-PAGE.2` |

### 7.2 `apps/desktop/e2e/decisions.spec.ts`

| Test ID | Tier | Fixtures | Exact Assertions | Requirements |
|--------|------|----------|------------------|-------------|
| `PW-DEC-001` | `T1` | `PlaywrightSeedKit.decisions.approveableProcess` | clicking approve returns committed ack metadata; `pending_operator_intents` row stores `process_id`, `desired_disposition = approve`, `reviewed_basis_hash`, `reviewed_manifest_hash`, `reviewed_at_stage_run_id`, `reviewed_at`, `base_remote_revision = 0`, `idempotency_key`, `status = queued`, and `queued_at`; when no note or retry metadata is supplied, `decision_note`, `last_attempt_at`, and `last_error` remain `NULL` on initial capture | `L2-FR-DB.6`, `L2-FR-DEC.1`, `L2-FR-DEC.4` |
| `PW-DEC-002` | `T1` | `PlaywrightSeedKit.decisions.watchableProcess` | watch action persists `desired_disposition = watch` and leaves prior history intact; UI shows watch as the current decision only after the commit fence clears | `L2-FR-DB.6`, `L2-FR-DEC.1`, `L2-FR-QS.2` |
| `PW-DEC-003` | `T1` | `PlaywrightSeedKit.decisions.coalescing` | approve then reject creates two distinct intent rows; first row’s `superseded_by_intent_id` points to the second; UI shows only reject as current | `L2-FR-DEC.2`, `L2-FR-DEC.4` |
| `PW-DEC-004` | `T1` | `PlaywrightSeedKit.decisions.sameDecisionReaffirm` | re-affirming the same disposition writes a new intent with a new `idempotency_key` and fresh reviewed-basis metadata; old row is not mutated in place | `L2-FR-DEC.2`, `L2-FR-DEC.3` |
| `PW-DEC-005` | `T1` | `PlaywrightSeedKit.decisions.staleAfterRerun` | decision captured against hash A becomes stale after rerun produces hash B; detail view shows `isStale = true`; stale warning blocks the “still current” affordance | `L2-FR-DEC.3`, `L2-FR-PAGE.2` |
| `PW-DEC-006` | `T1` | `PlaywrightSeedKit.decisions.sameHashRerun` | rerun with unchanged `decisionBasisHash` does not mark the decision stale; no warning badge is shown; reviewed basis remains equal to current basis | `L2-FR-DEC.3` |
| `PW-DEC-007` | `T1` | `PlaywrightSeedKit.decisions.lostAckRetry` | dropping the first response after DB commit and retrying with the same `idempotency_key` returns the same durable ack; only one new `pending_operator_intents` row exists | `L2-FR-DB.3`, `L2-FR-DEC.1` |
| `PW-DEC-008` | `T1` | `PlaywrightSeedKit.decisions.commitFence` | immediate post-click query with `snapshotSeq < visibleAfterSeq` still shows old decision state; post-fence snapshot shows the new decision and reviewed timestamp exactly once | `L2-FR-QS.2`, `L2-FR-QS.3`, `L2-FR-DB.3`, `L2-FR-DB.6` |

### 7.3 `apps/desktop/e2e/sessions.spec.ts`

| Test ID | Tier | Fixtures | Exact Assertions | Requirements |
|--------|------|----------|------------------|-------------|
| `PW-SES-001` | `T1` | `PlaywrightSeedKit.sessions.mixedList` | sessions page renders active and recent sessions with lifecycle phase, execution mode, duration, cleanup verdict, checkpoint count, and failure metadata from `list_sessions()` | `L2-FR-PAGE.3`, `L2-FR-QS.6` |
| `PW-SES-002` | `T1` | `PlaywrightSeedKit.sessions.preActiveFailure` | session that never reaches `active` still appears with canonical phase, error type, and duration; row drill-down remains enabled | `L2-FR-PAGE.3`, `L2-FR-QS.6` |
| `PW-SES-003` | `T1` | `PlaywrightSeedKit.sessions.livePty` | PTY session opens in live-terminal mode; bytes stream in sequence order; lifecycle state changes are visible without transcript corruption | `L2-FR-PAGE.4`, `L2-FR-CH.1`, `L2-FR-CH.7` |
| `PW-SES-004` | `T1` | `PlaywrightSeedKit.sessions.liveContract` | contract-mode session opens in transcript/history mode; input controls are disabled; structured event list remains searchable | `L2-FR-PAGE.4`, `L2-FR-CH.7` |
| `PW-SES-005` | `T1` | `PlaywrightSeedKit.sessions.selectedTopicOnly` | selecting one session attaches exactly one full-session stream while the list continues to receive metadata-only updates; opening a second viewer does not attach every session row | `L2-FR-CH.2`, `L2-FR-PAGE.3`, `L2-FR-PAGE.4` |
| `PW-SES-006` | `T1` | `PlaywrightSeedKit.sessions.gracefulStop` | `stop_session(graceful)` returns accepted ack; viewer shows `terminating` then terminal phase; session row remains in the list for audit | `L2-FR-DB.8`, `L2-FR-PAGE.4` |
| `PW-SES-007` | `T1` | `PlaywrightSeedKit.sessions.forceStop` | forced stop yields `cleanupVerdict = forced`; viewer surfaces the final lifecycle frame; no fake “clean exit” label is shown | `L2-FR-DB.8`, `L2-FR-CH.1`, `L2-FR-PAGE.4` |
| `PW-SES-008` | `T1` | `PlaywrightSeedKit.sessions.authChallengePty` | typing into the SessionViewer sends bytes to the attached PTY and advances the session beyond the auth prompt; duplicate bytes are not injected | `L2-FR-DB.9`, `L2-FR-CH.7`, `L2-FR-CH.8` |
| `PW-SES-009` | `T1` | `PlaywrightSeedKit.sessions.nonInteractive` | detached, contract, or already-terminal session disables input and resize controls with explicit rationale text | `L2-FR-DB.9`, `L2-FR-DB.10`, `L2-FR-CH.7` |
| `PW-SES-010` | `T1` | `PlaywrightSeedKit.sessions.resizeablePty` | `resize_session()` returns accepted ack only for attached PTY sessions; terminal geometry updates and stream ordering stays intact | `L2-FR-DB.10`, `L2-FR-PAGE.4` |
| `PW-SES-011` | `T1` | `PlaywrightSeedKit.sessions.retryableFailure` | retry action creates a new job/session identity while the original failed job remains queryable; the new session appears in the sessions page after fence visibility | `L2-FR-DB.11`, `L2-FR-PAGE.3`, `L2-FR-PAGE.4` |
| `PW-SES-012` | `T1` | `PlaywrightSeedKit.sessions.gapAndReconnect` | slow consumer or dropped socket emits an explicit `gap`; viewer requests replay from `lastSeq`; recovered frames merge without duplicate `seq` values and unresolved loss stays visible if replay is partial | `L2-FR-CH.3`, `L2-FR-CH.5`, `L2-FR-CH.9`, `L2-FR-CH.10`, `L2-FR-CH.11`, `L2-FR-CH.13`, `L2-FR-PAGE.11` |

### 7.4 `apps/desktop/e2e/commit-fence.spec.ts`

| Test ID | Tier | Fixtures | Exact Assertions | Requirements |
|--------|------|----------|------------------|-------------|
| `PW-FEN-001` | `T1` | `PlaywrightSeedKit.fence.ingestion` | `trigger_ingestion()` returns `accepted = true` with `jobId` and `visibleAfterSeq`; list query with older `snapshotSeq` does not show the new job row; post-fence snapshot does | `L2-FR-QS.2`, `L2-FR-DB.3`, `L2-FR-DB.4` |
| `PW-FEN-002` | `T1` | `PlaywrightSeedKit.fence.enrichment` | `trigger_enrichment()` targets exactly one process; no unrelated process rerun row appears before or after fence; acknowledged job appears only after `snapshotSeq >= visibleAfterSeq` | `L2-FR-QS.2`, `L2-FR-DB.5` |
| `PW-FEN-003` | `T1` | `PlaywrightSeedKit.fence.decision` | capture-decision flow preserves old decision until the fence clears; new decision becomes visible exactly once; `snapshotSeq` never uses stream progress as a shortcut | `L2-FR-QS.2`, `L2-FR-QS.3`, `L2-FR-BOOT.2`, `L2-FR-DB.6` |
| `PW-FEN-004` | `T1` | `PlaywrightSeedKit.fence.resolveError` | resolving an error does not remove the row optimistically; error queue still shows the row pre-fence and removes it only on the first satisfying snapshot | `L2-FR-QS.2`, `L2-FR-DB.7`, `L2-FR-PAGE.5` |
| `PW-FEN-005` | `T1` | `PlaywrightSeedKit.fence.retryJob` | retrying a failed job yields a new `jobId`; old failed record stays visible; new record is hidden until its `visibleAfterSeq` is satisfied | `L2-FR-DB.3`, `L2-FR-DB.11` |
| `PW-FEN-006` | `T1` | `PlaywrightSeedKit.fence.rollbackAndArchive` | refused or rolled-back mutations advance no user-visible `snapshotSeq`; session archive handoff switches to historical mode only after `snapshotSeq >= archiveSeq` | `L2-FR-QS.3`, `L2-FR-CH.4`, `L2-FR-DB.14` |
| `PW-FEN-007` | `T1` | `PlaywrightSeedKit.fence.concurrentMutations` | two committed mutations return ordered `visibleAfterSeq` values `A < B`; snapshot before `A` shows neither mutation, snapshot between `A` and `B` shows only the first, and snapshot `>= B` shows both; UI does not substitute `data_version` for the user-visible fence | `L2-FR-QS.2`, `L2-FR-QS.3`, `L2-FR-DB.3` |

### 7.5 `apps/desktop/e2e/bridge-failures.spec.ts`

| Test ID | Tier | Fixtures | Exact Assertions | Requirements |
|--------|------|----------|------------------|-------------|
| `PW-BRD-001` | `T1` | `PlaywrightSeedKit.bridge.transportUnavailable` | command issued while daemon socket is unreachable yields explicit transport failure; UI enters reconnect or browse-only handling; no optimistic success toast or ghost row appears | `L2-FR-DB.14`, `L2-FR-BOOT.4` |
| `PW-BRD-002` | `T1` | `PlaywrightSeedKit.bridge.conflictRefusal` | refusal with `conflictReason` is shown verbatim enough to distinguish busy/conflict/not-rerunnable states; retry CTA respects `retryable` flag | `L2-FR-DB.3`, `L2-FR-DB.18` |
| `PW-BRD-003` | `T1` | `PlaywrightSeedKit.bridge.invalidParams` | invalid command params surface `invalid_params`; no partial UI mutation occurs; field-level validation message matches the rejected enum or missing field | `L2-FR-DB.16`, `L2-FR-DB.18` |
| `PW-BRD-004` | `T1` | `PlaywrightSeedKit.bridge.alreadyTerminal` | stop/input/resize against an already-terminal session surfaces `already_terminal` or equivalent no-op outcome; viewer controls remain disabled afterwards | `L2-FR-DB.8`, `L2-FR-DB.9`, `L2-FR-DB.10` |
| `PW-BRD-005` | `T1` | `PlaywrightSeedKit.bridge.lostAck` | post-commit lost ack retried with same `idempotency_key` returns the original durable outcome; UI shows one success path and one durable row only | `L2-FR-DB.3`, `L2-FR-DB.14` |
| `PW-BRD-006` | `T1` | `PlaywrightSeedKit.bridge.reservationConflict` | downstream Agent Mail reservation conflict surfaces through session/error surfaces rather than a silent spinner; operator sees rerun/resolve path instead of implicit success | `L2-FR-DB.14`, `L2-FR-PAGE.5` |

### 7.6 `apps/desktop/e2e/errors.spec.ts`

| Test ID | Tier | Fixtures | Exact Assertions | Requirements |
|--------|------|----------|------------------|-------------|
| `PW-ERR-001` | `T1` | `PlaywrightSeedKit.errors.unresolvedSet` | errors page lists unresolved manifests with severity, stage, summary, and session linkage; row count matches unresolved seed rows only | `L2-FR-PAGE.5`, `L2-FR-QS.7` |
| `PW-ERR-002` | `T1` | `PlaywrightSeedKit.errors.empty` | empty unresolved set renders explicit empty-state copy; page does not synthesize warnings or placeholder rows | `L2-FR-QS.7`, `L2-FR-PAGE.12` |
| `PW-ERR-003` | `T1` | `PlaywrightSeedKit.errors.severityMatrix` | leaked cleanup or daemon-crash errors render as critical; pipeline-quality and source-drift errors render as warning; no L2 row is mislabeled as info | `L2-FR-QS.7`, `L2-FR-UI.6`, `L2-FR-UI.9` |
| `PW-ERR-004` | `T1` | `SecurityFixtureFactory.errorMarkdownAndJson` | drill-down renders manifest-style markdown through the sanitized pipeline and JSON/error context in escaped blocks; raw HTML never executes | `L2-FR-PAGE.5`, `L2-FR-SEC.1` |
| `PW-ERR-005` | `T1` | `PlaywrightSeedKit.errors.resolveable` | resolve action returns committed ack and removes the row only after commit-fence visibility; repeated resolve is idempotent and does not create duplicate state | `L2-FR-DB.7`, `L2-FR-PAGE.12` |
| `PW-ERR-006` | `T1` | `PlaywrightSeedKit.errors.rerunnable` | rerun-from-error creates a new job from the failed stage; original error remains part of history; row action enablement reflects whether rerun is valid | `L2-FR-DB.13`, `L2-FR-PAGE.12` |

### 7.7 `apps/desktop/e2e/security.spec.ts`

| Test ID | Tier | Fixtures | Exact Assertions | Requirements |
|--------|------|----------|------------------|-------------|
| `PW-SEC-001` | `T1` | `SecurityFixtureFactory.htmlAttackMatrix` | hostile payloads such as `<img src=x onerror=alert(1)>` and `<svg onload=alert(1)>` render as inert sanitized content; no injected node or handler survives in the DOM | `L2-FR-SEC.1`, `L2-FR-UI.3` |
| `PW-SEC-002` | `T1` | `SecurityFixtureFactory.dangerousLinks` | link payloads such as `[click](javascript:void(0))` and `[click](data:text/html,<script>alert(1)</script>)` are stripped or neutralized in manifest and error drill-down views | `L2-FR-SEC.1`, `L2-FR-UI.3`, `L2-FR-UI.6` |
| `PW-SEC-003` | `T1` | `SecurityFixtureFactory.errorMessageHtml` | agent-authored error text renders as escaped text content rather than raw HTML fragments; shared error summary view remains readable | `L2-FR-SEC.1`, `L2-FR-UI.6` |
| `PW-SEC-004` | `T1` | `SecurityFixtureFactory.osc8Hyperlink` | SessionViewer strips dangerous OSC 8 hyperlink escape sequences before paint; visible terminal text excludes the hidden link target | `L2-FR-SEC.1`, `L2-FR-PAGE.4` |
| `PW-SEC-005` | `T1` | `SecurityFixtureFactory.osc52Clipboard` | SessionViewer strips clipboard-related OSC 52 escapes; no clipboard write side effect occurs during render | `L2-FR-SEC.1`, `L2-FR-PAGE.4` |
| `PW-SEC-006` | `T1` | `SecurityFixtureFactory.crossSurfaceStrings` | list/detail/session surfaces render untrusted strings as text nodes across all affected surfaces; no `dangerouslySetInnerHTML`-style path is observable | `L2-FR-SEC.1`, `L2-FR-PAGE.2`, `L2-FR-PAGE.4`, `L2-FR-PAGE.5` |
| `PW-SEC-007` | `T1` | `SecurityFixtureFactory.largePayloads` | a `10MB` manifest and `100K`-line session output render through bounded truncation/virtualization paths with an explicit truncation or lazy-load marker; the page remains interactive enough for Playwright to resolve a named heading/control after load; sanitization markers remain in effect on the retained visible content | `L2-FR-SEC.1`, `L2-FR-UI.3`, `L2-FR-PAGE.4` |
| `PW-SEC-008` | `T1` | `SecurityFixtureFactory.unicodeSpoofing` | RTL override characters and zero-width joiners in agent output remain inert text and do not spoof filenames, links, or status labels in manifest, error, or session views | `L2-FR-SEC.1`, `L2-FR-PAGE.4`, `L2-FR-PAGE.5` |

### 7.8 `apps/desktop/e2e/daemon-status.spec.ts`

| Test ID | Tier | Fixtures | Exact Assertions | Requirements |
|--------|------|----------|------------------|-------------|
| `PW-DGN-001` | `T1` | `PlaywrightSeedKit.daemonStatus.live` | Daemon status page shows instance id, daemon version, socket path, DB path, artifact root, WAL bytes, and startup-recovery summary from `get_daemon_info()` | `L2-FR-PAGE.6`, `L2-FR-DB.21` |
| `PW-DGN-002` | `T1` | `PlaywrightSeedKit.daemonStatus.descriptorFallback` | when daemon is unavailable, page renders descriptor-backed stale diagnostics and explicit unavailable markers for live-only fields; no SQLite mutation occurs | `L2-FR-PAGE.6`, `L2-FR-DB.21` |
| `PW-DGN-003` | `T1` | `PlaywrightSeedKit.daemonStatus.restartable` | restart CTA is visible only in modes that allow it; accepted restart returns ack before disconnect and reconnects with a new `instanceId` | `L2-FR-PAGE.13`, `L2-FR-DB.12` |
| `PW-DGN-004` | `T1` | `PlaywrightSeedKit.daemonStatus.readOnly` | browse-only / limited contexts still show readable status diagnostics while mutating controls remain disabled or hidden | `L2-FR-PAGE.6`, `L2-FR-BOOT.10`, `L2-FR-PAGE.13` |

### 7.9 `apps/desktop/e2e/tauri-integration.spec.ts`

| Test ID | Tier | Fixtures | Exact Assertions | Requirements |
|--------|------|----------|------------------|-------------|
| `TAU-INT-001` | `T2` | `TauriLaunchHarness.separateDaemon` | Tauri window acts as a client of a separately managed daemon; app close does not implicitly kill a healthy daemon; daemon PID remains alive after UI close | `L2-FR-BOOT.1`, `L2-FR-BOOT.5` |
| `TAU-INT-002` | `T2` | `TauriLaunchHarness.stopDaemonAndQuit` | “Stop background daemon and quit” performs explicit daemon shutdown before app exit; daemon PID disappears and descriptor is cleaned up deterministically | `L2-FR-BOOT.5` |
| `TAU-INT-003` | `T2` | `TauriLaunchHarness.startupAutolaunch` | startup sequence reads descriptor before launching; absent daemon triggers one detached launch attempt; successful retry reaches full control without manual operator startup | `L2-FR-BOOT.6`, `L2-FR-BOOT.8`, `L2-FR-BOOT.9` |
| `TAU-INT-004` | `T2` | `TauriLaunchHarness.storageReadyDaemonUnavailable` | readable DB plus unavailable daemon yields browse-only mode with explicit banner and mutating controls disabled | `L2-FR-BOOT.3`, `L2-FR-BOOT.10` |
| `TAU-INT-005` | `T2` | `TauriLaunchHarness.schemaMismatch` | schema mismatch or protocol incompatibility enters incompatible mode rather than generic transport failure; operator sees restart/update/browse-only affordances | `L2-FR-BOOT.3`, `L2-FR-DB.2` |
| `TAU-INT-006` | `T2` | `TauriLaunchHarness.unreadableStorage` | unreadable or repairing storage enters recovery-required mode and blocks ordinary browsing until repair succeeds | `L2-FR-BOOT.3`, `L2-FR-BOOT.7` |
| `TAU-INT-007` | `T2` | `TauriLaunchHarness.degradedDaemon` | `daemon = starting|degraded` or `control = limited|read_only` maps to the limited mode, not browse-only or crash handling | `L2-FR-BOOT.3`, `L2-FR-BOOT.11` |
| `TAU-INT-008` | `T2` | `TauriLaunchHarness.totalModeMatrix` | reachable `storage × daemon × control × load` combinations map to exactly one UX mode; no overlapping or fall-through branch exists | `L2-FR-BOOT.11`, `L2-FR-BOOT.7` |
| `TAU-INT-009` | `T2` | `TauriLaunchHarness.sameDaemonReconnect` | closing and reopening the UI while daemon stays alive performs cheap reconnect; `instanceId` stays the same; `lastSeq` anchors remain valid | `L2-FR-BOOT.4`, `L2-FR-CH.5`, `L2-FR-STATE.8` |
| `TAU-INT-010` | `T2` | `TauriLaunchHarness.daemonCrashMidSession` | daemon crash during live PTY session triggers one restart attempt, then a fresh handshake; changed `instanceId` invalidates PTY resumption and leaves historical artifacts only | `L2-FR-BOOT.4`, `L2-FR-CH.6`, `L2-FR-DB.2` |
| `TAU-INT-011` | `T2` | `TauriLaunchHarness.nativeVsDevParity` | a native Tauri invoke/query/stream triplet returns the same app-level payload shape as the browser dev bridge for one seeded scenario; no second business-logic path exists | `L2-FR-DEV.1`, `L2-FR-DEV.2`, `L2-FR-DB.15` |
| `TAU-INT-012` | `T2` | `TauriLaunchHarness.productionBuild` | production desktop build excludes the dev bridge path and loopback HTTP server; shipped artifact still uses native invoke/channel successfully | `L2-FR-DEV.4`, `L2-FR-SEC.2` |

### 7.10 `apps/desktop/src-tauri/tests/query_service.rs`

| Test ID | Tier | Fixtures | Exact Assertions | Requirements |
|--------|------|----------|------------------|-------------|
| `RS-QS-001` | `T3` | `L2QueryServiceHarness.readOnlyDb` | read connection enables `PRAGMA query_only = ON`; attempted write or schema change fails; no frontend-provided generic SQL path exists | `L2-FR-QS.1` |
| `RS-QS-002` | `T3` | `L2QueryServiceHarness.processList` | `list_processes()` returns `QueryEnvelope<KeysetPage<ProcessListItem>>` with `data`, `snapshotSeq`, and `snapshotAt`; row fields are sufficient for list rendering with no per-row joins | `L2-FR-QS.2`, `L2-FR-QS.4`, `L2-FR-QS.9` |
| `RS-QS-003` | `T3` | `L2QueryServiceHarness.commitClock` | `snapshotSeq` is sourced from `commit_clock.commit_seq` at `id = 1`, not `PRAGMA data_version`; rolled-back writes advance neither user-visible clock nor fence | `L2-FR-QS.3` |
| `RS-QS-004` | `T3` | `L2QueryServiceHarness.processDetailVariants` | `get_process_detail()` returns explicit empty arrays for zero lots / zero candidates; missing large artifacts degrade to explicit error state while the rest of the payload remains intact | `L2-FR-QS.5`, `L2-FR-QS.13` |
| `RS-QS-005` | `T3` | `L2QueryServiceHarness.sessionList` | `list_sessions()` returns canonical phase names, execution mode, timing, cleanup verdict, exit code, checkpoint count, byte count, and pre-active failure metadata | `L2-FR-QS.6`, `L2-FR-QS.10` |
| `RS-QS-006` | `T3` | `L2QueryServiceHarness.errorList` | `list_errors()` returns derived severity, linkage fields, and empty-page semantics; no unresolved-error set becomes a transport error | `L2-FR-QS.7`, `L2-FR-QS.10` |
| `RS-QS-007` | `T3` | `L2QueryServiceHarness.invalidCursorAndFilters` | invalid cursor returns `invalid_cursor`; unsupported filter value returns `invalid_filter`; validation happens before partial payload emission | `L2-FR-QS.11` |
| `RS-QS-008` | `T3` | `L2QueryServiceHarness.historyReplay` | `get_session_history()` returns `QueryEnvelope<SessionHistoryChunk>` with `availability = complete|partial|missing`; `artifact_unavailable` is used only when metadata exists but payload cannot be recovered | `L2-FR-QS.12`, `L2-FR-QS.13` |
| `RS-QS-009` | `T3` | `L2QueryServiceHarness.busyReader` | busy-timeout exhaustion or unreadable storage returns typed read failure and mutates no state; no optimistic row or fence movement occurs | `L2-FR-QS.8` |
| `RS-QS-010` | `T3` | `L2QueryServiceHarness.shortReadTransactions` | list/detail queries finish in short-lived read transactions; WAL checkpoint can proceed after query completion; transaction duration limit is observable in the harness | `L2-FR-QS.1`, `L2-FR-PAG.2` |
| `RS-QS-011` | `T3` | `L2QueryServiceHarness.commitIncrement` | successful write transaction increments `commit_clock.commit_seq` exactly once from the `id = 1` row and returns the same value as the mutation visibility fence; explicit rollback leaves `commit_seq` unchanged | `L2-FR-QS.3`, `L2-FR-DB.3` |
| `RS-QS-012` | `T3` | `L2QueryServiceHarness.dataVersionVsCommitSeq` | QueryService may observe `PRAGMA data_version` changes for internal cache invalidation, but returned `snapshotSeq` remains the committed `commit_seq`; cache refresh without committed write never advances the user-visible visibility fence | `L2-FR-QS.3` |

### 7.11 `apps/desktop/src-tauri/tests/daemon_bridge.rs`

| Test ID | Tier | Fixtures | Exact Assertions | Requirements |
|--------|------|----------|------------------|-------------|
| `RS-DBR-001` | `T3` | `L2BridgeHarness.handshakeOk` | first wire frame is `hello`; successful response is `welcome` with protocol version, daemon version, `instanceId`, `schemaEpoch`, `commitSeq`, health, `mutationsAllowed`, active sessions, reconnect outcome, and stream-resume flag | `L2-FR-DB.1`, `L2-FR-DB.2`, `L2-FR-DB.20` |
| `RS-DBR-002` | `T3` | `L2BridgeHarness.protocolMismatch` | unsupported protocol range returns `protocol_range_mismatch` with server-supported range; no handler executes | `L2-FR-DB.2`, `L2-FR-DB.18` |
| `RS-DBR-003` | `T3` | `L2BridgeHarness.schemaMismatch` | schema epoch mismatch returns typed rejection with expected vs actual epoch; incompatible state is distinguishable from transport failure | `L2-FR-DB.2`, `L2-FR-BOOT.3` |
| `RS-DBR-004` | `T3` | `L2BridgeHarness.reconnectOutcomes` | `lastSeenInstanceId` equal to current id yields `same_daemon_reconnect`; differing id yields `daemon_restart_invalidated`; stream-resume permission matches reconnect outcome | `L2-FR-DB.2`, `L2-FR-CH.6` |
| `RS-DBR-005` | `T3` | `L2BridgeHarness.requestKinds` | post-handshake traffic accepts only `command`, `subscribe`, and `unsubscribe`; every request carries `requestId`; mutation commands require an `idempotencyKey` while `get_daemon_info` omits it | `L2-FR-DB.19`, `L2-FR-DB.21` |
| `RS-DBR-006` | `T3` | `L2BridgeHarness.commandRouter` | unknown methods return `not_found`; domain handlers are invoked by the declared router ownership (`jobs`, `decisions`, `errors`, `sessions`, `control`) only for supported methods | `L2-FR-DB.17`, `L2-FR-DB.18` |
| `RS-DBR-007` | `T3` | `L2BridgeHarness.errorRetryMatrix` | `busy`/`transport_unavailable` are retryable; validation and `not_interactive` errors are not; retry hints are stable across equivalent failures | `L2-FR-DB.14`, `L2-FR-DB.18` |
| `RS-DBR-008` | `T3` | `L2BridgeHarness.streamProjection` | `OutputFrame::Pty/Stdout/Stderr` project to `SessionFrame.kind = pty/stdout/stderr`; `ControlEvent::Started/Checkpoint/Progress/AuthChallenge/Warning/FinalResultReady/Exited` project to the declared `StructuredSessionEvent` variants without renaming or dropping any event; lifecycle frames stay separate from semantic event frames; topic naming is stable across reconnects | `L2-FR-CH.1`, `L2-FR-CH.12`, `L2-FR-CH.14`, `L2-FR-DB.15` |
| `RS-DBR-009` | `T3` | `L2BridgeHarness.legacyHandshakeCompatibility` | the first exchange remains `hello`/`welcome`; raw pre-router mutation traffic is rejected on the current Layer 1 handshake-only socket with `mutations_unavailable`; once the L2 command router is active, command traffic is accepted only after successful handshake, preserving additive compatibility with `crates/daemon/src/ipc/handshake.rs` | `L2-FR-DB.15`, `L2-FR-DB.19` |
| `RS-DBR-010` | `T3` | `L2BridgeHarness.lifecycleFrames` | the public lifecycle stream emits `created -> spawned -> active -> terminating -> exited -> reaped -> archived` in order; `cleanupVerdict` appears only on `reaped`; `archiveSeq` appears only on `archived`; no phase is skipped or duplicated in the nominal path | `L2-FR-CH.1`, `L2-FR-CH.4` |

### 7.12 `packages/intelligence/src/qualification/goods-qualification.test.ts`

| Test ID | Tier | Fixtures | Exact Assertions | Requirements |
|--------|------|----------|------------------|-------------|
| `TS-QUAL-001` | `T3` | `CoverageMathFixture.cpvFamilies` | CPV fixtures classify into goods, services, works, mixed, and unknown exactly as expected; result is deterministic across runs | `L2-FR-PAGE.1`, `L2-FR-PAGE.2` |
| `TS-QUAL-002` | `T3` | `CoverageMathFixture.expiredVsDps` | ordinary expired notices are filtered out while DPS / Dynamic Market cases remain eligible; rule does not leak into non-DPS paths | `L2-FR-PAGE.1` |
| `TS-QUAL-003` | `T3` | `CoverageMathFixture.separableGoods` | mixed contract with separable goods computes goods-addressable value rather than whole-contract value; downstream manifest input uses the goods-addressable value | `L2-FR-PAGE.2`, `L2-FR-MAN.3` |
| `TS-QUAL-004` | `T3` | `CoverageMathFixture.lowConfidence` | low-confidence or review-required cases stay reviewable instead of silently qualifying; classification output preserves reason text for the operator surface | `L2-FR-PAGE.1`, `L2-FR-PAGE.2` |
| `TS-QUAL-005` | `T3` | `CoverageMathFixture.accessRouteRules` | access-route edge cases used by the list/detail display are preserved in the qualification output contract without UI-side heuristics | `L2-FR-PAGE.1`, `L2-FR-PAGE.2` |

### 7.13 `packages/intelligence/src/margins/coverage-rollup.test.ts`

| Test ID | Tier | Fixtures | Exact Assertions | Requirements |
|--------|------|----------|------------------|-------------|
| `TS-COV-001` | `T3` | `CoverageMathFixture.lotCountHappyPath` | `pricedCoverageBps`, `landedCostCoverageBps`, and `netQualifiedCoverageBps` follow the qualifying-lot denominators exactly | `L2-FR-COV.2` |
| `TS-COV-002` | `T3` | `CoverageMathFixture.valueWeightedHappyPath` | value-weighted metrics use qualifying lot value, not lot count; expected bps outputs match the seeded addressable values exactly | `L2-FR-COV.2`, `L2-FR-COV.5` |
| `TS-COV-003` | `T3` | `CoverageMathFixture.thresholdBands` | 7000/3000 threshold bands map to stable labels and tones; displayed label and numeric percentage are both derivable from the same bps input | `L2-FR-COV.1`, `L2-FR-COV.3` |
| `TS-COV-004` | `T3` | `CoverageMathFixture.mandatoryLots` | `unresolvedMandatoryLots` is returned as an absolute count, not percentage; non-zero unresolved mandatory lots remain a blocker | `L2-FR-COV.4` |
| `TS-COV-005` | `T3` | `CoverageMathFixture.missingValuesConservative` | missing lot values or candidate value shares keep the gate conservative and never inflate `netQualifiedValueCoverageBps` | `L2-FR-COV.5` |
| `TS-COV-006` | `T3` | `CoverageMathFixture.basisHash` | `decisionBasisHash` is stable for canonically equivalent inputs and changes when qualifying operator-relevant inputs change | `L2-FR-COV.2`, `L2-FR-DEC.3` |

### 7.14 `packages/manifests/tests/render-opportunity.test.ts`

| Test ID | Tier | Fixtures | Exact Assertions | Requirements |
|--------|------|----------|------------------|-------------|
| `TS-MAN-001` | `T3` | `ManifestFixture.full` | renderer returns `markdown`, `basisHash`, `sections`, `blockerCount`, and `generatedAt`; no output field is omitted in the happy path | `L2-FR-MAN.1`, `L2-FR-MAN.2` |
| `TS-MAN-002` | `T3` | `ManifestFixture.passThroughHash` | renderer passes through daemon-provided `basisHash` without recomputing it; output hash matches input exactly | `L2-FR-MAN.2` |
| `TS-MAN-003` | `T3` | `ManifestFixture.sectionOrder` | output contains the eight canonical section headings in the declared order and with stable parsed section ids | `L2-FR-MAN.3`, `L2-FR-MAN.5` |
| `TS-MAN-004` | `T3` | `ManifestFixture.emptyData` | missing candidates, quotes, or landed-cost data still produce explicit sections and empty-state prose instead of silently removing sections | `L2-FR-MAN.4` |
| `TS-MAN-005` | `T3` | `ManifestFixture.formatting` | money renders from integer pence, percentages render from bps, blockers remain prominent, and confidence bands avoid raw decimal dumps | `L2-FR-MAN.3` |

### 7.15 `packages/ui/tests/components.test.tsx`

| Test ID | Tier | Fixtures | Exact Assertions | Requirements |
|--------|------|----------|------------------|-------------|
| `TS-UI-001` | `T3` | static import graph | `packages/ui` exports presentational components and view-model types only; package tests import no desktop bridge, QueryService, daemon, or Supabase modules | `L2-FR-UI.1`, `L2-FR-SEC.2` |
| `TS-UI-002` | `T3` | `packages/ui/fixtures/contract-card-frame.ts` | `ContractCardFrame` renders summary, metrics, decision, `rightRail`, and `actions` slots without boolean-prop branching; error state remains renderable | `L2-FR-UI.2`, `L2-FR-UI.7` |
| `TS-UI-003` | `T3` | `packages/ui/fixtures/manifest-viewer.ts` | `ManifestViewer` renders navigable sections, preserves copyable section ids, and uses sanitized markdown output only | `L2-FR-UI.3` |
| `TS-UI-004` | `T3` | `packages/ui/fixtures/pipeline-badge.ts` | `PipelineBadge` always shows text plus icon plus color; `validationState` is hidden when `unverified` and rendered otherwise | `L2-FR-UI.4`, `L2-FR-A11Y.2` |
| `TS-UI-005` | `T3` | `packages/ui/fixtures/freshness-badge.ts` | `FreshnessBadge` consumes only the `FreshnessBadgeVM` shape and does not inspect raw timestamps or sync state internally | `L2-FR-UI.5`, `L2-FR-UI.10` |
| `TS-UI-006` | `T3` | `packages/ui/fixtures/error-summary-list.ts` | `ErrorSummaryList` renders typed rows, honors optional `onResolveClick = null`, and does not require a mutable surface to function | `L2-FR-UI.6`, `L2-FR-UI.11` |
| `TS-UI-007` | `T3` | `packages/ui/fixtures/index.ts` | barrel export re-exports only the shared UI surface from one entry point and introduces no backend side effects | `L2-FR-UI.8` |
| `TS-UI-008` | `T3` | `packages/ui/fixtures/*` | all shared-component tests are fixture-based and backend-agnostic; no test opens SQLite, UDS, or network transport to render shared UI | `L2-FR-UI.12` |

### 7.16 `apps/desktop/src/stores/session-registry.test.ts`

| Test ID | Tier | Fixtures | Exact Assertions | Requirements |
|--------|------|----------|------------------|-------------|
| `TS-REG-001` | `T3` | `SessionStoreFixture.metaUpsert` | registry stores sessions by `daemonInstanceId + sessionId`; metadata upserts do not clobber unrelated sessions | `L2-FR-STATE.1`, `L2-FR-STATE.4` |
| `TS-REG-002` | `T3` | `SessionStoreFixture.seqTracking` | applying stream frames advances `lastSeq` monotonically per session only; out-of-order frame is rejected or buffered according to the store contract | `L2-FR-STATE.5`, `L2-FR-CH.13` |
| `TS-REG-003` | `T3` | `SessionStoreFixture.gapLifecycle` | gap state records exact `fromSeq..toSeq`; replay resolution clears the gap only for the matching range | `L2-FR-STATE.5`, `L2-FR-CH.10`, `L2-FR-CH.11` |
| `TS-REG-004` | `T3` | `SessionStoreFixture.archiveFence` | `archiveSeq` is tracked and prevents historical handoff until snapshot fence is satisfied; early mode switch is impossible | `L2-FR-CH.4`, `L2-FR-STATE.4` |
| `TS-REG-005` | `T3` | `SessionStoreFixture.restartAwareAnchors` | same-daemon reopen preserves `lastSeq`; daemon instance change clears persisted anchors and attachability | `L2-FR-STATE.8`, `L2-FR-CH.5`, `L2-FR-CH.6` |

### 7.17 `apps/desktop/src/stores/session-buffer.test.ts`

| Test ID | Tier | Fixtures | Exact Assertions | Requirements |
|--------|------|----------|------------------|-------------|
| `TS-BUF-001` | `T3` | `SessionStoreFixture.ptyBytes` | PTY buffer stores bytes, not pre-split lines; tail extraction is derived without mutating authoritative byte storage | `L2-FR-STATE.2`, `L2-FR-STATE.7` |
| `TS-BUF-002` | `T3` | `SessionStoreFixture.contractFrames` | contract buffer stores text/event frames and explicit gap markers; frame kind is preserved through replay merge | `L2-FR-STATE.2`, `L2-FR-STATE.7` |
| `TS-BUF-003` | `T3` | `SessionStoreFixture.replayMerge` | replay merge inserts recovered frames by original `seq` without duplicating already-present frames before or after the requested range | `L2-FR-CH.10`, `L2-FR-CH.13`, `L2-FR-CH.15` |
| `TS-BUF-004` | `T3` | `SessionStoreFixture.pruneAndMissing` | bounded pruning retains tail context and visible unresolved-gap markers; partial/missing replay does not silently collapse the marker | `L2-FR-CH.3`, `L2-FR-CH.11`, `L2-FR-STATE.7` |

### 7.18 `apps/desktop/src/lib/transport.test.ts`

| Test ID | Tier | Fixtures | Exact Assertions | Requirements |
|--------|------|----------|------------------|-------------|
| `TS-TRN-001` | `T3` | `window.__TAURI_INTERNALS__` present | transport shim selects native invoke/channel path when Tauri internals exist; no HTTP fallback request is issued | `L2-FR-DEV.3`, `L2-FR-STATE.4` |
| `TS-TRN-002` | `T3` | `window.__TAURI_INTERNALS__` absent | transport shim uses `POST /__tauri_invoke__` plus `/__ws__` for streaming in browser dev mode; payload shape matches app contract | `L2-FR-DEV.1`, `L2-FR-DEV.2`, `L2-FR-DEV.3` |
| `TS-TRN-003` | `T3` | static import graph | hooks and stores import the shim instead of importing `@tauri-apps/api` directly; runtime bridge selection stays centralized in one file | `L2-FR-DEV.3`, `L2-FR-STATE.4` |
| `TS-TRN-004` | `T3` | production bundle manifest | production bundle omits the dev bridge entry and loopback middleware code path; shipped desktop build still resolves native transport | `L2-FR-DEV.4`, `L2-FR-SEC.2` |

### 7.19 `crates/daemon/tests/agent_contracts_finder_ingestion.rs`

| Test ID | Tier | Fixtures | Exact Assertions | Requirements |
|--------|------|----------|------------------|-------------|
| `AG-CF-001` | `T3` | `ContractsFinderSkillFixture.structure` | skill directory contains `SKILL.md`, `checkpoint.schema.json`, `result.schema.json`, `canary.inputs.jsonl`, and `golden.assertions.yaml` in the expected path | `L2-FR-CF.1`, `L2-FR-CF.2`, `L2-FR-CF.3` |
| `AG-CF-002` | `T3` | `ContractsFinderSkillFixture.canaryHappyPath` | canary fetch validates recent response shape, locale, currency, and rate-limit expectations before full ingestion begins | `L2-FR-CF.3` |
| `AG-CF-003` | `T3` | `ContractsFinderSkillFixture.incrementalCursor` | incremental run emits structured notices plus checkpoint-safe continuation metadata; `continuation_required` is used when time budget expires with safe remaining work | `L2-FR-CF.1`, `L2-FR-CF.2` |
| `AG-CF-004` | `T3` | `ContractsFinderSkillFixture.backtrackWindow` | cursor overlap/backtrack prevents skipped notices when the source mutates between runs; overlap does not create duplicate durable notice ids | `L2-FR-CF.1`, `L2-FR-CF.2` |
| `AG-CF-005` | `T3` | `ContractsFinderSkillFixture.runManifest` | run manifest includes the required YAML frontmatter and narrative sections matching the FTS standard; daemon finalization can consume it without ad hoc patching | `L2-FR-CF.2`, `L2-FR-CF.3` |
| `AG-CF-006` | `T3` | `ContractsFinderSkillFixture.failureMatrix` | source drift, locale/currency drift, soft block, CAPTCHA block, and stall conditions emit explicit failure codes rather than silent partial success | `L2-FR-CF.3`, `L2-FR-DB.4` |

### 7.20 `crates/daemon/tests/agent_supplier_enrichment.rs`

| Test ID | Tier | Fixtures | Exact Assertions | Requirements |
|--------|------|----------|------------------|-------------|
| `AG-ENR-001` | `T3` | `L2DaemonHarness.enrichmentHappyPath` | enrichment session produces supplier quotes and the candidate-scoped pricing inputs needed by the detail page and coverage rollup | `L2-FR-PAGE.2`, `L2-FR-COV.2` |
| `AG-ENR-002` | `T3` | `L2DaemonHarness.enrichmentConfidence` | quote extraction preserves dual confidence or usability fields expected by the opportunity detail and manifest renderer | `L2-FR-PAGE.2`, `L2-FR-MAN.3` |
| `AG-ENR-003` | `T3` | `L2DaemonHarness.enrichmentScopedRerun` | rerunning enrichment for one process yields a fresh `stage_run_id` for affected scope and changes `decisionBasisHash` only when operator-relevant data changes | `L2-FR-DB.5`, `L2-FR-DEC.3` |
| `AG-ENR-004` | `T3` | `L2DaemonHarness.enrichmentPartialData` | missing or partial quote data degrades to explicit empty/partial outputs without corrupting the remaining process detail payload | `L2-FR-QS.5`, `L2-FR-COV.1` |

### 7.21 `crates/daemon/tests/agent_health_detection.rs`

| Test ID | Tier | Fixtures | Exact Assertions | Requirements |
|--------|------|----------|------------------|-------------|
| `AG-HLT-001` | `T3` | `L2DaemonHarness.stallNoOutput` | no output + no progress within the configured window yields `AGENT_STALLED`; session transitions to failure with explicit classification | `L2-FR-CH.8`, `L2-FR-PAGE.3` |
| `AG-HLT-002` | `T3` | `L2DaemonHarness.authChallengePty` | PTY-resolvable auth challenge yields structured `auth_challenge` event with correct challenge kind and reattachability semantics | `L2-FR-CH.8`, `L2-FR-CH.14` |
| `AG-HLT-003` | `T3` | `L2DaemonHarness.authChallengeBrowserOnly` | browser-only challenge emits actionable reveal-profile-path intervention instead of a dead-end prompt | `L2-FR-CH.8`, `L2-FR-PAGE.4` |
| `AG-HLT-004` | `T3` | `L2DaemonHarness.cleanupEscalation` | stop escalation records `clean`, `forced`, or `leaked` cleanup verdicts correctly and surfaces them to the session/error surfaces | `L2-FR-CH.1`, `L2-FR-QS.7`, `L2-FR-UI.9` |
| `AG-HLT-005` | `T3` | `L2DaemonHarness.preActiveCrash` | session failure before `active` still persists canonical phase history and error context for the sessions page and SessionViewer | `L2-FR-QS.6`, `L2-FR-PAGE.3`, `L2-FR-PAGE.4` |

### 7.22 `crates/daemon/tests/agent_error_manifest.rs`

| Test ID | Tier | Fixtures | Exact Assertions | Requirements |
|--------|------|----------|------------------|-------------|
| `AG-ERR-001` | `T3` | `L2DaemonHarness.skillFailure` | skill execution failure creates an error manifest row plus durable markdown/context artifacts with process/job/session linkage | `L2-FR-PAGE.5`, `L2-FR-QS.7` |
| `AG-ERR-002` | `T3` | `L2DaemonHarness.contractInvalid` | contract-invalid or suspect output path produces an error manifest with typed `error_type`, actionable summary, and rerunnable stage metadata | `L2-FR-PAGE.5`, `L2-FR-DB.13` |
| `AG-ERR-003` | `T3` | `L2DaemonHarness.resolveAndRerun` | resolving and rerunning from error keeps the original manifest queryable for audit while creating a new job from the failed stage | `L2-FR-DB.7`, `L2-FR-DB.13`, `L2-FR-PAGE.12` |
| `AG-ERR-004` | `T3` | `L2DaemonHarness.severityCriticality` | leaked cleanup or daemon-crash cases derive `critical`; remaining pipeline/source/agent failures derive `warning`; severity mapping is stable for UI consumers | `L2-FR-QS.7`, `L2-FR-UI.6`, `L2-FR-UI.9` |

## 8. Coverage Matrix

| Spec Requirement | Test ID(s) | Status |
|------------------|-----------|--------|
| `L2-FR-BOOT.1` | `CI-L2-001`, `CI-L2-002`, `TAU-INT-001` | Covered |
| `L2-FR-BOOT.2` | `PW-FEN-003`, `RS-QS-002`, `TS-TRN-001`, `TS-TRN-002` | Covered |
| `L2-FR-BOOT.3` | `TAU-INT-004`, `TAU-INT-005`, `TAU-INT-006`, `TAU-INT-007`, `TAU-INT-008` | Covered |
| `L2-FR-BOOT.4` | `PW-BRD-001`, `TAU-INT-009`, `TAU-INT-010` | Covered |
| `L2-FR-BOOT.5` | `TAU-INT-001`, `TAU-INT-002`, `TAU-INT-009` | Covered |
| `L2-FR-BOOT.6` | `TAU-INT-003`, `TAU-INT-004` | Covered |
| `L2-FR-BOOT.7` | `TAU-INT-006`, `TAU-INT-008` | Covered |
| `L2-FR-BOOT.8` | `TAU-INT-003` | Covered |
| `L2-FR-BOOT.9` | `TAU-INT-003` | Covered |
| `L2-FR-BOOT.10` | `PW-DGN-004`, `TAU-INT-004` | Covered |
| `L2-FR-BOOT.11` | `TAU-INT-007`, `TAU-INT-008` | Covered |
| `L2-FR-QS.1` | `RS-QS-001`, `RS-QS-009`, `RS-QS-010` | Covered |
| `L2-FR-QS.2` | `RS-QS-002`, `PW-DEC-008`, `PW-FEN-001` | Covered |
| `L2-FR-QS.3` | `RS-QS-003`, `RS-QS-011`, `RS-QS-012`, `PW-DEC-008`, `PW-FEN-006`, `PW-FEN-007` | Covered |
| `L2-FR-QS.4` | `PW-CON-001`, `PW-CON-002`, `PW-CON-006`, `RS-QS-002` | Covered |
| `L2-FR-QS.5` | `PW-CON-007`, `PW-CON-008`, `PW-CON-009`, `RS-QS-004` | Covered |
| `L2-FR-QS.6` | `PW-SES-001`, `PW-SES-002`, `RS-QS-005`, `AG-HLT-005` | Covered |
| `L2-FR-QS.7` | `PW-ERR-001`, `PW-ERR-003`, `RS-QS-006`, `AG-ERR-001` | Covered |
| `L2-FR-QS.8` | `RS-QS-009`, `PW-BRD-001` | Covered |
| `L2-FR-QS.9` | `RS-QS-002` | Covered |
| `L2-FR-QS.10` | `RS-QS-005`, `RS-QS-006` | Covered |
| `L2-FR-QS.11` | `PW-CON-005`, `RS-QS-007` | Covered |
| `L2-FR-QS.12` | `PW-SES-012`, `RS-QS-008` | Covered |
| `L2-FR-QS.13` | `RS-QS-004`, `RS-QS-008` | Covered |
| `L2-FR-DB.1` | `RS-DBR-001` | Covered |
| `L2-FR-DB.2` | `RS-DBR-001`, `RS-DBR-002`, `RS-DBR-003`, `RS-DBR-004`, `TAU-INT-010` | Covered |
| `L2-FR-DB.3` | `PW-DEC-007`, `PW-DEC-008`, `PW-FEN-001`, `PW-FEN-007`, `PW-BRD-005`, `RS-QS-011` | Covered |
| `L2-FR-DB.4` | `PW-FEN-001`, `AG-CF-006` | Covered |
| `L2-FR-DB.5` | `PW-FEN-002`, `AG-ENR-003` | Covered |
| `L2-FR-DB.6` | `PW-DEC-001`, `PW-DEC-002`, `PW-DEC-008` | Covered |
| `L2-FR-DB.7` | `PW-ERR-005`, `AG-ERR-003` | Covered |
| `L2-FR-DB.8` | `PW-SES-006`, `PW-SES-007`, `PW-BRD-004` | Covered |
| `L2-FR-DB.9` | `PW-SES-008`, `PW-SES-009`, `PW-BRD-004` | Covered |
| `L2-FR-DB.10` | `PW-SES-009`, `PW-SES-010`, `PW-BRD-004` | Covered |
| `L2-FR-DB.11` | `PW-SES-011`, `PW-FEN-005` | Covered |
| `L2-FR-DB.12` | `PW-DGN-003`, `TAU-INT-012` | Covered |
| `L2-FR-DB.13` | `PW-ERR-006`, `AG-ERR-002`, `AG-ERR-003` | Covered |
| `L2-FR-DB.14` | `PW-BRD-001`, `PW-BRD-002`, `PW-BRD-005`, `PW-BRD-006`, `PW-FEN-006`, `RS-DBR-007` | Covered |
| `L2-FR-DB.15` | `CI-L2-001`, `RS-DBR-008`, `RS-DBR-009`, `TAU-INT-011` | Covered |
| `L2-FR-DB.16` | `PW-BRD-003`, `RS-DBR-005` | Covered |
| `L2-FR-DB.17` | `RS-DBR-006` | Covered |
| `L2-FR-DB.18` | `PW-BRD-002`, `PW-BRD-003`, `RS-DBR-002`, `RS-DBR-006`, `RS-DBR-007` | Covered |
| `L2-FR-DB.19` | `RS-DBR-005`, `RS-DBR-009` | Covered |
| `L2-FR-DB.20` | `RS-DBR-001` | Covered |
| `L2-FR-DB.21` | `PW-DGN-001`, `PW-DGN-002`, `RS-DBR-005` | Covered |
| `L2-FR-CH.1` | `PW-SES-003`, `PW-SES-007`, `RS-DBR-008`, `RS-DBR-010`, `AG-HLT-004` | Covered |
| `L2-FR-CH.2` | `PW-SES-001`, `PW-SES-005` | Covered |
| `L2-FR-CH.3` | `PW-SES-012`, `TS-BUF-004` | Covered |
| `L2-FR-CH.4` | `PW-FEN-006`, `TS-REG-004`, `RS-DBR-010` | Covered |
| `L2-FR-CH.5` | `PW-SES-012`, `TAU-INT-009`, `TS-REG-005` | Covered |
| `L2-FR-CH.6` | `RS-DBR-004`, `TAU-INT-010`, `TS-REG-005` | Covered |
| `L2-FR-CH.7` | `PW-SES-003`, `PW-SES-004`, `PW-SES-008`, `PW-SES-009`, `PW-SES-010` | Covered |
| `L2-FR-CH.8` | `PW-SES-008`, `AG-HLT-002`, `AG-HLT-003` | Covered |
| `L2-FR-CH.9` | `PW-SES-012` | Covered |
| `L2-FR-CH.10` | `PW-SES-012`, `TS-REG-003`, `TS-BUF-003` | Covered |
| `L2-FR-CH.11` | `PW-SES-012`, `TS-REG-003`, `TS-BUF-004` | Covered |
| `L2-FR-CH.12` | `RS-DBR-008` | Covered |
| `L2-FR-CH.13` | `PW-SES-012`, `TS-REG-002`, `TS-BUF-003` | Covered |
| `L2-FR-CH.14` | `RS-DBR-008`, `AG-HLT-002` | Covered |
| `L2-FR-CH.15` | `PW-SES-012`, `TS-BUF-003` | Covered |
| `L2-FR-DEV.1` | `CI-L2-006`, `TS-TRN-002`, `TAU-INT-011` | Covered |
| `L2-FR-DEV.2` | `TS-TRN-002`, `TAU-INT-011` | Covered |
| `L2-FR-DEV.3` | `CI-L2-004`, `TS-TRN-001`, `TS-TRN-002`, `TS-TRN-003` | Covered |
| `L2-FR-DEV.4` | `CI-L2-006`, `TS-TRN-004`, `TAU-INT-012` | Covered |
| `L2-FR-PAGE.1` | `PW-CON-001`, `PW-CON-002`, `PW-CON-006`, `TS-QUAL-001`, `TS-QUAL-004` | Covered |
| `L2-FR-PAGE.2` | `PW-CON-007`, `PW-CON-008`, `PW-CON-009`, `PW-CON-010`, `TS-QUAL-001`, `AG-ENR-001` | Covered |
| `L2-FR-PAGE.3` | `PW-SES-001`, `PW-SES-002`, `AG-HLT-001`, `AG-HLT-005` | Covered |
| `L2-FR-PAGE.4` | `PW-SES-003`, `PW-SES-004`, `PW-SES-006`, `PW-SES-008`, `PW-SEC-004`, `AG-HLT-003` | Covered |
| `L2-FR-PAGE.5` | `PW-ERR-001`, `PW-ERR-004`, `PW-ERR-006`, `AG-ERR-001`, `PW-BRD-006` | Covered |
| `L2-FR-PAGE.6` | `PW-DGN-001`, `PW-DGN-002`, `PW-DGN-004` | Covered |
| `L2-FR-PAGE.7` | `CI-L2-005` | Covered |
| `L2-FR-PAGE.8` | `PW-CON-003`, `PW-CON-005` | Covered |
| `L2-FR-PAGE.9` | `PW-CON-007` | Covered |
| `L2-FR-PAGE.10` | `PW-SES-001` | Covered |
| `L2-FR-PAGE.11` | `PW-SES-012` | Covered |
| `L2-FR-PAGE.12` | `PW-ERR-002`, `PW-ERR-005`, `PW-ERR-006`, `AG-ERR-003` | Covered |
| `L2-FR-PAGE.13` | `PW-DGN-003`, `PW-DGN-004` | Covered |
| `L2-FR-STATE.1` | `TS-REG-001`, `TS-TRN-001` | Covered |
| `L2-FR-STATE.2` | `TS-BUF-001`, `TS-BUF-002` | Covered |
| `L2-FR-STATE.3` | `TS-REG-001` | Covered |
| `L2-FR-STATE.4` | `CI-L2-005`, `TS-REG-001`, `TS-TRN-001`, `TS-TRN-003` | Covered |
| `L2-FR-STATE.5` | `TS-REG-002`, `TS-REG-003` | Covered |
| `L2-FR-STATE.6` | `TAU-INT-008` | Covered |
| `L2-FR-STATE.7` | `TS-BUF-001`, `TS-BUF-002`, `TS-BUF-004` | Covered |
| `L2-FR-STATE.8` | `TAU-INT-009`, `TS-REG-005` | Covered |
| `L2-FR-UI.1` | `CI-L2-004`, `TS-UI-001` | Covered |
| `L2-FR-UI.2` | `PW-CON-001`, `TS-UI-002` | Covered |
| `L2-FR-UI.3` | `PW-SEC-001`, `PW-SEC-002`, `TS-UI-003` | Covered |
| `L2-FR-UI.4` | `PW-CON-010`, `TS-UI-004` | Covered |
| `L2-FR-UI.5` | `TS-UI-005` | Covered |
| `L2-FR-UI.6` | `PW-ERR-003`, `PW-SEC-002`, `TS-UI-006`, `AG-ERR-004` | Covered |
| `L2-FR-UI.7` | `TS-UI-002` | Covered |
| `L2-FR-UI.8` | `CI-L2-005`, `TS-UI-007` | Covered |
| `L2-FR-UI.9` | `PW-ERR-003`, `AG-HLT-004`, `AG-ERR-004` | Covered |
| `L2-FR-UI.10` | `TS-UI-005` | Covered |
| `L2-FR-UI.11` | `TS-UI-006` | Covered |
| `L2-FR-UI.12` | `TS-UI-008` | Covered |
| `L2-FR-MAN.1` | `PW-CON-007`, `TS-MAN-001` | Covered |
| `L2-FR-MAN.2` | `TS-MAN-001`, `TS-MAN-002` | Covered |
| `L2-FR-MAN.3` | `PW-CON-007`, `TS-QUAL-003`, `TS-MAN-003`, `TS-MAN-005` | Covered |
| `L2-FR-MAN.4` | `PW-CON-008`, `PW-CON-009`, `TS-MAN-004` | Covered |
| `L2-FR-MAN.5` | `TS-MAN-003` | Covered |
| `L2-FR-CF.1` | `AG-CF-001`, `AG-CF-003`, `AG-CF-004` | Covered |
| `L2-FR-CF.2` | `AG-CF-001`, `AG-CF-003`, `AG-CF-004`, `AG-CF-005` | Covered |
| `L2-FR-CF.3` | `AG-CF-001`, `AG-CF-002`, `AG-CF-005`, `AG-CF-006` | Covered |
| `L2-FR-A11Y.1` | `PW-SES-003`, `TS-UI-003` | Covered |
| `L2-FR-A11Y.2` | `PW-CON-004`, `TS-UI-004` | Covered |
| `L2-FR-SEC.1` | `PW-ERR-004`, `PW-SEC-001`, `PW-SEC-002`, `PW-SEC-004`, `PW-SEC-005`, `PW-SEC-006`, `PW-SEC-007`, `PW-SEC-008` | Covered |
| `L2-FR-SEC.2` | `CI-L2-002`, `CI-L2-004`, `CI-L2-007`, `TS-TRN-004`, `TAU-INT-012` | Covered |
| `L2-FR-COV.1` | `PW-CON-007`, `PW-CON-009`, `PW-CON-010`, `TS-COV-003` | Covered |
| `L2-FR-COV.2` | `AG-ENR-001`, `TS-COV-001`, `TS-COV-002`, `TS-COV-006` | Covered |
| `L2-FR-COV.3` | `PW-CON-010`, `TS-COV-003` | Covered |
| `L2-FR-COV.4` | `TS-COV-004` | Covered |
| `L2-FR-COV.5` | `PW-CON-010`, `TS-COV-002`, `TS-COV-005` | Covered |
| `L2-FR-DEC.1` | `PW-DEC-001`, `PW-DEC-002`, `PW-DEC-007`, `TS-COV-006` | Covered |
| `L2-FR-DEC.2` | `PW-DEC-003`, `PW-DEC-004` | Covered |
| `L2-FR-DEC.3` | `PW-DEC-004`, `PW-DEC-005`, `PW-DEC-006`, `AG-ENR-003`, `TS-COV-006` | Covered |
| `L2-FR-DEC.4` | `PW-DEC-001`, `PW-DEC-003` | Covered |
| `L2-FR-PAG.1` | `PW-CON-003`, `PW-CON-004` | Covered |
| `L2-FR-PAG.2` | `PW-CON-002`, `PW-CON-003`, `RS-QS-010` | Covered |

## 9. Zero-Gap Declaration

All `118` validated `L2-FR-*` requirement IDs are covered.

There are **no open coverage gaps** in this TDD. The remaining dependency note is operational, not a spec gap:
- Layer 2 implementation must add the `pending_operator_intents` table to checked-in SQLite migrations, because the current repository still lacks the L0-defined DDL even though L2 decision tests assume it exists.

## 10. Downstream Consumer Notes

- Strategy creation should use the test IDs in this document as phase gates; the critical gates are the commit-fence lane (`PW-FEN-*`), bridge/handshake lane (`RS-DBR-*`), and native-shell lifecycle lane (`TAU-INT-*`).
- Beads created from this source should reference the concrete file surface and exact test IDs above; do not create “generic desktop testing” beads.
- Completion audit must compare actual assertions against the exact assertions column here, especially for `PW-DEC-001`, `PW-FEN-001`, `PW-SES-012`, `RS-QS-003`, `RS-DBR-001`, `TS-COV-006`, and `AG-CF-005`.
