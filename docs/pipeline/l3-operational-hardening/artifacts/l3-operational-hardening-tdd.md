# L3 Operational Hardening — TDD Specification

## 1. Scope

**Primary source of truth:** `docs/strategy/layers/layer-3-operational-hardening.md`

**Supporting context:**
- `docs/strategy/layer-strategy.md` §5, §9, §11
- `docs/strategy/cross-reference-index.md`
- `docs/pipeline/l1-daemon-core/artifacts/l1-daemon-core-tdd.md`
- `docs/pipeline/l1-daemon-core/artifacts/l1-daemon-core-bootstrap.md`
- `docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-tdd.md`
- `AGENTS.md` §Toolchain, §Compiler Checks, §UBS, §Beads workflow
- `crates/daemon/src/jobs/runner.rs`
- `crates/daemon/src/agents/artifacts.rs`
- `crates/daemon/src/agents/contracts.rs`
- `crates/daemon/src/errors/manifest.rs`
- `crates/daemon/src/sessions/reaper.rs`
- `crates/daemon/src/pty/session.rs`
- `crates/daemon/src/pty/capture.rs`
- `crates/daemon/src/pty/health.rs`
- `crates/daemon/src/agents/adapter.rs`
- `crates/daemon/src/db/sqlite.rs`
- `crates/daemon/src/ipc/command_router.rs`
- `packages/intelligence/src/qualification/goods-qualification.test.ts`

**Purpose of this document:** define exactly what tests must exist before Layer 3 implementation begins.

**Coverage target:** all `98` unique `L3-FR-*` requirement IDs in the validated Layer 3 spec map to at least one stable test ID in this document.

**Planned inventory:** `191` total checks
- `8` static CI gates
- `183` runnable tests
- `18` spec-predeclared new files from Layer 3 §5.2
- `3` additive auxiliary files required to close schema, shared-library, and operational-status coverage gaps without hiding them inside unrelated containers

**Important mixed-stack constraint:** Layer 3 spans Rust daemon runtime, SQLite schema activation, skill filesystem contracts, and TypeScript package logic. This TDD therefore covers:
- Cargo-driven Rust unit/integration/E2E tests under `crates/daemon/tests/`
- Bun-driven Vitest tests under `packages/intelligence/src/relations/` and `packages/manifests/tests/`
- Real PTY/process-group tests using helper binaries and OS primitives
- Real SQLite writes with in-memory or temp-file databases
- Dry-run validator sub-job execution through the real Job Runner boundary
- Manual operational signoff for the five-day unattended target

## 2. Non-Negotiable Test Principles

- Use Cargo only for Rust workspaces and Bun only for TypeScript workspaces.
- Do not mock SQLite. Use `rusqlite::Connection::open_in_memory()` only for pure unit-policy tests and temp-file databases for integration/E2E tests that assert migrations, WAL semantics, or transactional linkage.
- Do not mock process supervision. PTY, reaper, cleanup, and host-event tests must use real helper binaries, real process groups, real signals, and real `waitpid`/`killpg` behavior.
- Any test claiming PTY hardening, session semantics, or cleanup correctness must prove it with a real PTY plus helper binaries. Transcript-only success criteria are not acceptable.
- Dry-run validation tests must assert concrete database fields and artifact paths: `workflow_jobs.validation_required`, `workflow_jobs.validation_verdict_id`, `validation_verdicts`, `pipeline_assessments.validation_state`, scratch artifact roots, and backlog rows.
- Relation-resolution tests are TypeScript/Vitest tests. They must not be translated into Rust unit tests or validated only through prose.
- Shared-library coverage is mandatory: the relation engine and error-manifest renderer each require direct package-level tests, not only downstream daemon smoke tests.
- Keep `agent_claimed_*` and `daemon_observed_*` assertions separate in manifest tests. Tests that collapse them into one field are invalid.
- No `.skip`, `#[ignore]`, `test.todo`, empty test bodies, existence-only assertions, vague “works correctly” checks, or false-positive-prone snapshot-only approvals.
- Per-commit CI means cumulative `T1 + T2` must stay under five minutes on a standard hosted runner.
- `#![forbid(unsafe_code)]` remains mandatory for all Rust production and test code.
- Run `ubs` on the changed files before landing the implementation that satisfies this TDD.

## 3. Test Pyramid

| Layer | Purpose | Allowed Medium | Disallowed Shortcuts | Primary Evidence |
|------|---------|----------------|----------------------|------------------|
| Static Gates | build, lint, format, inventory parity, no-unsafe policy | `cargo check`, `cargo clippy`, `cargo fmt`, `bun test`, inventory assertions | manual eyeballing | command exit status, generated inventory reports |
| Unit | pure policy, relation scoring, trigger evaluation, manifest field logic, telemetry math | in-memory SQLite, in-process fixtures, Vitest pure functions | process mocks, fake DB APIs, transcript-only checks | exact values, structured decision reasons |
| Integration | temp-file SQLite, real PTY/process, real artifacts, command-router dispatch, schema activation | helper binaries, temp roots, `waitpid`, `killpg`, real migrations, Bun package tests | mocked cleanup, mocked WAL, fake sub-job linkage | DB rows, artifact paths, lifecycle traces, typed responses |
| E2E / Soak | full daemon path, real skill directories, validator sub-jobs, canary cycles, catchup sweeps | daemon runtime + real helper binaries + real skill filesystem + retained artifacts | transcript scraping, fake scheduler clocks without persisted state | session artifacts, `validation_verdicts`, `source_runtime_state`, manifests, telemetry rows |
| Manual Gate | unattended operations target, operator reviewability, desktop/web trace drill-down | target host, live cron, live canaries, real operator dashboard | replay-only signoff | run diary, blackout incidents, zero-orphan verification, trace drill-down |

## 4. CI Tiering And Cadence

| Tier | Budget | Cadence | Includes |
|------|--------|---------|----------|
| `T1` | `< 90 sec` | every commit | static gates, pure policy tests, relation scoring, manifest field parity, error-code inventory, TypeScript package tests |
| `T2` | `< 4 min` | every commit | temp-file SQLite integration, cron scheduling, lease/arbiter integration, schema activation smoke, dry-run isolation, selected PTY hardening smoke |
| `T3` | `< 20 min` | nightly / pre-merge as needed | replay suites, validator executor, backlog recovery, canary health, source breaker transitions, skill lifecycle journeys |
| `T4` | `< 30 min` | scheduled dedicated runner / pre-release | live cron-triggered ingestion smoke, live source-health canaries, abbreviated unattended rehearsal |
| `Manual Gate` | operator time | milestone signoff | five consecutive unattended days, `>=95%` skill success, blackout review, zero orphaned descendants, verdict-to-bytes trace drill-down |

**Tier allocation rule:** any test that spawns real validator sub-jobs, waits for canary cadence, or exercises crash-recovery/replay beyond smoke scope belongs in `T3` or `T4`, never `T1`.

## 5. Shared Test Infrastructure

| Harness / Fixture | Scope | Purpose |
|------------------|-------|---------|
| `TestStateDb` | `T1` / `T2` / `T3` | Reuses the existing L1 temp-file SQLite bootstrap with L0/L1/L2 migrations, then applies the L3 migration under test |
| `ArtifactSandbox` | `T2` / `T3` | Creates per-test artifact roots, scratch roots, chunk stores, run-manifest paths, and recovery probes |
| `PtySessionHarness` | `T2` / `T3` | Reuses the L1 PTY harness for real PTY spawn/drain/phase capture and extends it with L3 hardening assertions |
| `ContractSessionHarness` | `T2` / `T3` | Reuses the L1 contract harness for validator/result-contract sessions and bounded prompt payload assertions |
| `DaemonHarness` | `T2` / `T3` / `T4` | Boots the daemon with temp runtime roots, artifact roots, UDS descriptors, skill roots, and seeded source/runtime rows |
| `TestSkillDir` | `T1` / `T2` / `T3` | Materializes valid and invalid Layer 3 skill directory variants, including validator/base and source-health/auth-refresh folders |
| `ProcessTreeHelper` | `T2` / `T3` | Runs helper modes for sleep, stubborn children, split stdout/stderr, auth challenge, forked descendants, crash-after-checkpoint, and PTY semantics |
| `ProcessIsolationGuard` | `T2` / `T3` | Ensures unique temp roots and zero surviving descendants after every real-process test; serializes only when shared signal surfaces require it |
| `ManifestAssert` | `T1` / `T2` / `T3` | Parses YAML frontmatter, appendix blocks, disagreement sections, and manifest file-size limits using the existing L1 helper |
| `CronSchedulerHarness` | `T1` / `T2` | Seeds cron definitions, frozen skill bindings, source runtime state, and explicit clock instants; verifies enqueued `workflow_jobs` rows and `next_run_at` transitions |
| `ResourceArbiterFixture` | `T1` / `T2` | Seeds `browser_slots`, `agent_slots`, and `source_lanes`; tracks structured grant/deny/release events and effective ceilings after skill-local lowering |
| `SourceHealthFixture` | `T2` / `T3` | Produces passed/failed/timed-out canary outcomes, challenge windows, cooldown timestamps, and logical canary classifications mapped to existing `ErrorCode` values |
| `ValidationPolicyFixture` | `T1` | Feeds policy mode, promotion stage, trigger inventory, risk signals, behavior versions, and recent verdict history into the policy engine |
| `ValidatorHarness` | `T2` / `T3` | Spawns real validator sub-jobs through the Job Runner with bounded manifests, rubric hashes, drift deltas, and synthetic verdict payloads |
| `CircuitBreakerFixture` | `T1` / `T2` | Drives explicit `closed -> half_open -> open` state transitions with persisted timestamps and stale-write conflict checks |
| `RelationScoringFixture` | `T1` | Produces deterministic notice-pair feature vectors, hard blockers, heuristic scores, and canonical-root tie-breakers for Vitest |
| `TelemetryProbe` | `T1` / `T2` / `T3` | Captures skill-level and validation-level metric emissions plus health-state transitions at their declared update points |
| `OperationalStatusHarness` | `T2` | Exercises typed IPC status commands for cron, source health, validation, and resource arbiter visibility without mutating runtime state |

## 6. Planned File Inventory

| Surface | Count | Notes |
|--------|-------|-------|
| Static gates | 8 | repository-level commands, inventory, and drift guards |
| Fault injection | 4 files / 33 tests | exactly the 4 files predeclared in Layer 3 §5.2 |
| Deterministic replay | 2 files / 12 tests | exactly the 2 files predeclared in Layer 3 §5.2 |
| Scheduling & jobs | 2 files / 19 tests | exactly the 2 files predeclared in Layer 3 §5.2 |
| Skill lifecycle | 3 files / 21 tests | exactly the 3 files predeclared in Layer 3 §5.2 |
| Validation | 6 files / 59 tests | exactly the 6 files predeclared in Layer 3 §5.2 |
| Relations | 1 file / 12 tests | exactly the 1 file predeclared in Layer 3 §5.2 |
| Additive auxiliary coverage | 3 files / 27 tests | required to cover schema activation, shared error-manifest renderer, manifest-key parity, enum inventories, and operational-status surfaces not named directly in §5.2 |

## 7. Static CI Gates

| Test ID | Tier | Fixtures | Exact Assertions | Requirements |
|--------|------|----------|------------------|-------------|
| `CI-L3-001` | `T1` | repo workspace | `cargo check --all-targets` exits `0` for the daemon workspace after Layer 3 modules, migrations, and tests are added | `L3-FR-1.3.1`-`L3-FR-1.3.12`, `L3-FR-4.1.1`-`L3-FR-4.3.1` |
| `CI-L3-002` | `T1` | repo workspace | `cargo clippy --all-targets -- -D warnings` exits `0`; no Layer 3 runtime or test code ships with warnings | `L3-FR-1.3.1`-`L3-FR-4.6.2` |
| `CI-L3-003` | `T1` | repo workspace | `cargo fmt --check` exits `0` for all Rust production and test code | supporting invariant |
| `CI-L3-004` | `T1` | Bun workspace | `bun run test` at the workspace root plus `cd packages/intelligence && bun run test` and `cd packages/manifests && bun test` all exit `0`; Vitest package coverage remains runnable under Bun only | `L3-FR-1.8.1`, `L3-FR-3.6.1`-`L3-FR-3.6.8` |
| `CI-L3-005` | `T1` | repository inventory | module inventory matches the 12 new Layer 3 daemon surfaces, `migrations/0005_l3_operational_hardening.sql` exists, the 18 predeclared test files exist, additive files are present, and the Layer 3 skill filesystem inventory matches the spec | `L3-FR-1.3.1`-`L3-FR-1.3.12`, `L3-FR-1.9.1`, `L3-FR-4.1.1`-`L3-FR-4.3.1` |
| `CI-L3-006` | `T1` | repository policy | all new Rust files keep `#![forbid(unsafe_code)]`, no `#[ignore]`/`.skip`/`test.todo` markers exist in Layer 3 Rust or TS test surfaces | supporting invariant |
| `CI-L3-007` | `T1` | inventory report | manifest-field parity, `ErrorCode` inventory, and `HealthIssue` inventory each map to explicit TDD tests; no core field or enum variant is left uncovered by name | supporting invariant |
| `CI-L3-008` | `T1` | coverage report | every `L3-FR-*` requirement has at least one test ID, every test ID is unique, and the declared `T1 + T2` budget stays under five minutes | supporting invariant |

## 8. Detailed Test Catalog

### 8.1 `fault_process_cleanup.rs`

| Test ID | Tier | Fixtures | Exact Assertions | Requirements |
|--------|------|----------|------------------|-------------|
| `L3-T-fault_process_cleanup-001` | `T2` | `DaemonHarness`, `ProcessTreeHelper`, `ProcessIsolationGuard` | orphaned grandchild cleanup escalates through the declared signal order and ends with `cleanup_verdict = forced` only if descendants survive past graceful budgets; no leaked PID remains in the final survivor scan | `L3-FR-1.3.4`, `L3-FR-1.6.1` |
| `L3-T-fault_process_cleanup-002` | `T2` | `DaemonHarness`, `ProcessTreeHelper` | losing the root child handle does not skip group cleanup; the reaper still sweeps by `process_group_id`, writes a cleanup report, and records zero leaked descendants when the group exits cleanly | `L3-FR-1.3.4` |
| `L3-T-fault_process_cleanup-003` | `T2` | `DaemonHarness`, `ProcessTreeHelper` | a helper that ignores `SIGINT` and `SIGTERM` forces escalation to `SIGKILL`; the action log preserves `sigint -> sigterm -> sigkill -> orphan-sweep` order and does not report `clean` | `L3-FR-1.3.4`, `L3-FR-1.6.1` |
| `L3-T-fault_process_cleanup-004` | `T3` | `DaemonHarness`, `ProcessTreeHelper`, `ArtifactSandbox` | daemon-startup orphan recovery finds descendants from a prior session, writes a crash-recovery cleanup verdict, and links the recovery manifest/artifact path to the recovered session identity | `L3-FR-1.3.4`, `L3-FR-4.4.1` |
| `L3-T-fault_process_cleanup-005` | `T2` | `ProcessTreeHelper`, `ProcessIsolationGuard` | if the root process already exited, the reaper still sweeps recorded descendants before returning `clean`; it never treats `root_pid = None` as permission to skip descendant cleanup | `L3-FR-1.3.4` |
| `L3-T-fault_process_cleanup-006` | `T3` | `ProcessTreeHelper`, `ProcessIsolationGuard` | a double-fork escape attempt is caught by the orphan sweep and leaves zero surviving descendants after teardown; the final verdict is never silently upgraded to `clean` if survivors were detected mid-cleanup | `L3-FR-1.3.4`, `L3-FR-1.6.1` |
| `L3-T-fault_process_cleanup-007` | `T2` | `ArtifactSandbox`, `DaemonHarness` | cleanup reports persist `cleanup_verdict`, `action_log`, `leaked_pids`, and `error_code` fields with monotonic action order and a stable artifact path under the session root | `L3-FR-1.3.4`, `L3-FR-4.4.1` |
| `L3-T-fault_process_cleanup-008` | `T2` | `DaemonHarness` | archive finalization is blocked until `CompletionState` proves `child_exit_observed`, `pty_drained`, `descendants_clean`, and `verdict_recorded`; no session reaches `archived` while any completion prerequisite is false | `L3-FR-1.3.4`, `L3-FR-1.6.1` |

### 8.2 `fault_pty_edge_cases.rs`

| Test ID | Tier | Fixtures | Exact Assertions | Requirements |
|--------|------|----------|------------------|-------------|
| `L3-T-fault_pty_edge_cases-001` | `T2` | `PtySessionHarness`, `ProcessTreeHelper` | inherited non-stdio file descriptors are closed or marked `CLOEXEC` before child `exec`; the helper reports only the expected terminal and stdio descriptors | `L3-FR-1.3.5`, `L3-FR-1.6.1` |
| `L3-T-fault_pty_edge_cases-002` | `T2` | `PtySessionHarness`, `ArtifactSandbox` | `EIO` on PTY master read after slave closure is treated as expected hangup and transitions through `terminating -> exited -> reaped` without a standalone internal-error classification | `L3-FR-1.3.5`, `L3-FR-1.6.1` |
| `L3-T-fault_pty_edge_cases-003` | `T2` | `PtySessionHarness` | an unexpected PTY read failure while the session is still active follows the explicit cleanup/error path and produces a classified failure with runtime IDs, not a silent hangup success | `L3-FR-1.3.5`, `L3-FR-1.6.1` |
| `L3-T-fault_pty_edge_cases-004` | `T2` | `PtySessionHarness`, `ProcessTreeHelper` | headless PTY sessions start at `120x40`; the helper sees exactly those dimensions until an explicit operator resize occurs | `L3-FR-1.3.5`, `L3-FR-1.6.1` |
| `L3-T-fault_pty_edge_cases-005` | `T2` | `PtySessionHarness` | terminal geometry changes occur only on explicit operator attach/resize commands and never from capture, replay, or session bootstrap side effects | `L3-FR-1.6.1` |
| `L3-T-fault_pty_edge_cases-006` | `T2` | `PtySessionHarness`, `ProcessTreeHelper` | coalesced child-exit signals act as wakeups only; the reaper drains `waitpid` until no children remain instead of assuming one signal equals one reap | `L3-FR-1.3.5`, `L3-FR-1.6.1` |
| `L3-T-fault_pty_edge_cases-007` | `T2` | `PtySessionHarness` | all hardening paths preserve the canonical phase ordering `created -> spawned -> active -> terminating -> exited -> reaped -> archived`; no hardening-specific phase is introduced | `L3-FR-1.6.1` |
| `L3-T-fault_pty_edge_cases-008` | `T1` | `PtySessionHarness` | expected hangup paths do not require a new `PtySessionError` variant; unexpected PTY I/O remains classified through the existing `AgentRunFailed` error surface with explicit context | `L3-FR-1.6.1` |

### 8.3 `fault_host_events.rs`

| Test ID | Tier | Fixtures | Exact Assertions | Requirements |
|--------|------|----------|------------------|-------------|
| `L3-T-fault_host_events-001` | `T2` | `DaemonHarness`, `TelemetryProbe` | a monotonic suspend gap above the configured threshold emits `HOST_SLEEP_INTERRUPTED` and taints any active browser-driven PTY session | `L3-FR-1.7.1` |
| `L3-T-fault_host_events-002` | `T1` | `TelemetryProbe` | a wall-clock change without a monotonic gap does not falsely emit `HOST_SLEEP_INTERRUPTED`; stall detection remains monotonic-time-based | `L3-FR-1.7.1` |
| `L3-T-fault_host_events-003` | `T2` | `ResourceArbiterFixture`, `TelemetryProbe` | CPU/memory/disk pressure blocks admission of new work while allowing already-running safe work to complete; the pressure event is surfaced explicitly in host-health telemetry | `L3-FR-1.7.1`, `L3-FR-1.4.1`-`L3-FR-1.4.4` |
| `L3-T-fault_host_events-004` | `T2` | `ResourceArbiterFixture` | saturating one source lane during resource pressure still permits unrelated-source work when quotas allow; denials remain source-specific and auditable | `L3-FR-1.4.1`-`L3-FR-1.4.4`, `L3-FR-1.7.1` |
| `L3-T-fault_host_events-005` | `T2` | `CronSchedulerHarness`, `ResourceArbiterFixture` | startup catchup after downtime orders overdue work by admission priority rather than flooding runnable jobs all at once | `L3-FR-1.5.2`, `L3-FR-1.7.1` |
| `L3-T-fault_host_events-006` | `T2` | `CronSchedulerHarness`, `SourceHealthFixture` | a source with `circuit_state = open` or active cooldown suppresses normal catchup/cron jobs and admits only `source-health/*` or `auth-refresh/*` jobs | `L3-FR-1.5.2`, `L3-FR-3.5.2`, `L3-FR-3.5.3` |
| `L3-T-fault_host_events-007` | `T1` | `TelemetryProbe` | sleep and resource pressure reuse the existing `HealthIssue` surface and do not introduce a second host-health enum | `L3-FR-1.7.1` |
| `L3-T-fault_host_events-008` | `T1` | `TelemetryProbe` | when sleep/resource classifications are present, `StallTracker` output does not misclassify the same event as `AGENT_STALLED`; health precedence remains explicit and deterministic | `L3-FR-1.7.1` |
| `L3-T-fault_host_events-009` | `T1` | `TelemetryProbe` | the complete `HealthIssue` inventory remains `AgentStalled`, `AgentAuthChallenge`, `HostSleepInterrupted`, and `HostResourceExhausted`, and each variant’s `.code()` value matches the daemon contract exactly | `L3-FR-1.7.1` |
| `L3-T-fault_host_events-010` | `T1` | `TelemetryProbe` | `detect_auth_challenge`, `detect_host_sleep`, and `detect_resource_pressure` can all emit their own `HealthIssue` values without being collapsed into `AgentStalled`; `StallTracker` stays a separate no-output/no-progress surface | `L3-FR-1.7.1` |

### 8.4 `fault_partial_failure.rs`

| Test ID | Tier | Fixtures | Exact Assertions | Requirements |
|--------|------|----------|------------------|-------------|
| `L3-T-fault_partial_failure-001` | `T2` | `TestStateDb`, `ValidatorHarness` | a valid checkpoint followed by crash preserves `last_checkpoint_seq`, records partial-failure state, and never upgrades the job to success without a final validated result | `L3-FR-1.3.7`, `L3-FR-2.5.1` |
| `L3-T-fault_partial_failure-002` | `T2` | `TestStateDb` | out-of-order checkpoint replay is rejected without advancing checkpoint state or stage-head state; the persisted checkpoint reference remains the last durable valid one | `L3-FR-1.3.7` |
| `L3-T-fault_partial_failure-003` | `T2` | `TestStateDb` | a continuation segment that crashes leaves the continuation chain open and prevents final stage-head closure until the chain is completed or explicitly failed | `L3-FR-2.5.2` |
| `L3-T-fault_partial_failure-004` | `T2` | `TestStateDb` | additive partial commits preserve successful sibling scope rows while failed siblings remain explicit failures; no set-replacement head is advanced from a mixed-success additive run | `L3-FR-2.5.1`, `L3-FR-2.5.2` |
| `L3-T-fault_partial_failure-005` | `T2` | `ContractSessionHarness`, `TestStateDb` | a schema-invalid final result mid-stream fails the worker path, emits explicit contract-invalid context, and spawns no semantic validator sub-job | `L3-FR-1.3.7`, `L3-FR-2.4.4` |
| `L3-T-fault_partial_failure-006` | `T2` | `ValidatorHarness`, `TestStateDb` | a rendered-but-unpersisted semantic verdict is treated as validator failure: the worker advances fail-open, receives no `validation_verdict_id`, and the backlog records the missed validation | `L3-FR-2.4.4`, `L3-FR-2.7.1` |
| `L3-T-fault_partial_failure-007` | `T2` | `ArtifactSandbox`, `TestStateDb` | `db/checkpoints.rs` validates checkpoint schema before persisting the checkpoint artifact and refuses malformed checkpoint payloads without writing a broken reference into `workflow_jobs` | `L3-FR-1.3.7` |

### 8.5 `replay_pty_sessions.rs`

| Test ID | Tier | Fixtures | Exact Assertions | Requirements |
|--------|------|----------|------------------|-------------|
| `L3-T-replay_pty_sessions-001` | `T3` | `ArtifactSandbox`, `PtySessionHarness` | replaying captured PTY bytes with split reads reproduces the same typed control-event sequence after raw bytes are reassembled in capture order | `L3-FR-1.3.8`, `L3-FR-1.3.9`, `L3-FR-4.4.1`, `L3-FR-4.4.2` |
| `L3-T-replay_pty_sessions-002` | `T3` | `ArtifactSandbox` | ANSI/escape-heavy PTY payloads do not corrupt stream-index sequencing or structured trace boundaries during deterministic replay | `L3-FR-1.3.8`, `L3-FR-1.3.9` |
| `L3-T-replay_pty_sessions-003` | `T3` | `ArtifactSandbox`, `ManifestAssert` | a stored verdict can be traced to the manifest and then to PTY artifact refs without transcript scraping or operator memory; the join path uses explicit IDs and hashes only | `L3-FR-4.4.1`, `L3-FR-4.4.3` |
| `L3-T-replay_pty_sessions-004` | `T3` | `ArtifactSandbox` | replay recovery truncates torn trailing index lines and exposes only the longest contiguous committed PTY byte prefix for replay | `L3-FR-1.3.9`, `L3-FR-4.4.3` |
| `L3-T-replay_pty_sessions-005` | `T3` | `ArtifactSandbox`, `PtySessionHarness` | replaying a captured auth challenge yields a structured `AuthChallenge` event while preserving the original raw PTY bytes as immutable forensic evidence | `L3-FR-1.3.8`, `L3-FR-4.4.2` |
| `L3-T-replay_pty_sessions-006` | `T2` | `ArtifactSandbox`, `ManifestAssert` | manifest appendix artifact refs and stored SHA-256 values resolve to the same session stream files replayed in the forensic layer | `L3-FR-2.1.4`, `L3-FR-4.4.3` |

### 8.6 `replay_contract_sessions.rs`

| Test ID | Tier | Fixtures | Exact Assertions | Requirements |
|--------|------|----------|------------------|-------------|
| `L3-T-replay_contract_sessions-001` | `T3` | `ArtifactSandbox`, `ContractSessionHarness` | replaying captured contract sessions preserves separate stdout/stderr byte lanes while control events derive only from the structured parser path | `L3-FR-1.3.8`, `L3-FR-1.3.9`, `L3-FR-4.4.2` |
| `L3-T-replay_contract_sessions-002` | `T3` | `ContractSessionHarness` | malformed and truncated structured payloads replay to explicit parser/protocol errors; no fabricated checkpoints or final-result events appear | `L3-FR-1.3.8`, `L3-FR-4.4.1` |
| `L3-T-replay_contract_sessions-003` | `T2` | `ContractSessionHarness`, `TestStateDb` | `FinalResultReady` remains non-authoritative on replay until the result contract is validated and the worker transaction commits successfully | `L3-FR-1.3.8`, `L3-FR-2.4.1` |
| `L3-T-replay_contract_sessions-004` | `T3` | `ValidatorHarness`, `ArtifactSandbox` | replaying an archived contract session rebuilds the same bounded validator input packet: manifest, rubric hash, drift delta, OQG signals, and anomaly-stratified sample | `L3-FR-2.4.2`, `L3-FR-4.4.3` |
| `L3-T-replay_contract_sessions-005` | `T3` | `ValidatorHarness`, `ManifestAssert` | a fail-open incident remains traceable from `validation_verdicts` to the stored manifest and then to contract-mode forensic bytes without manual archaeology | `L3-FR-2.7.1`, `L3-FR-4.4.3` |
| `L3-T-replay_contract_sessions-006` | `T2` | `TelemetryProbe` | retention policy metadata for protocol traces and contract forensic bytes matches the declared minima and never expires forensic bytes before linked manifest/verdict surfaces | `L3-FR-4.4.3` |

### 8.7 `cron_scheduler.rs`

| Test ID | Tier | Fixtures | Exact Assertions | Requirements |
|--------|------|----------|------------------|-------------|
| `L3-T-cron_scheduler-001` | `T1` | `CronSchedulerHarness` | a valid cron expression parses and computes the expected `next_run_at` from a seeded UTC baseline without mutating unrelated sources | `L3-FR-1.3.1`, `L3-FR-1.5.1` |
| `L3-T-cron_scheduler-002` | `T1` | `CronSchedulerHarness` | an invalid cron expression is rejected at definition load time with an explicit configuration error and creates no runnable job rows | `L3-FR-1.3.1`, `L3-FR-1.3.2` |
| `L3-T-cron_scheduler-003` | `T2` | `CronSchedulerHarness`, `TestStateDb` | a due root cron run enqueues through `EnqueueJobRequest` with frozen `source_id`, `skill_id`, `skill_version`, `cron_expression`, and `parent_job_id = NULL` | `L3-FR-1.5.1` |
| `L3-T-cron_scheduler-004` | `T2` | `CronSchedulerHarness`, `TestSkillDir` | after a newer skill version exists on disk, an already-enqueued due job still executes the frozen original binding and is not late-rebound at claim time | `L3-FR-1.5.1` |
| `L3-T-cron_scheduler-005` | `T1` | `CronSchedulerHarness` | disabled definitions do not enqueue work and preserve explicit enablement/next-run state for later reactivation | `L3-FR-1.3.2`, `L3-FR-1.5.1` |
| `L3-T-cron_scheduler-006` | `T2` | `CronSchedulerHarness`, `ResourceArbiterFixture` | a startup catchup sweep enqueues overdue jobs in admission-priority order and never floods the system beyond arbiter capacity | `L3-FR-1.3.1`, `L3-FR-1.5.2` |
| `L3-T-cron_scheduler-007` | `T2` | `CronSchedulerHarness` | Europe/London spring-forward (`UTC+0 -> UTC+1`) skips the nonexistent local hour without fabricating a double-run; the first post-transition enqueue occurs at the correct next matching instant | `L3-FR-1.5.2` |
| `L3-T-cron_scheduler-008` | `T2` | `CronSchedulerHarness` | Europe/London fall-back repeated local hour does not double-enqueue a cron definition for the same logical instant; repeated wall-clock labels stay deduplicated by instant | `L3-FR-1.5.2` |
| `L3-T-cron_scheduler-009` | `T2` | `CronSchedulerHarness`, `SourceHealthFixture` | `source_runtime_state` with `open` or cooling-down circuit suppresses normal ingestion while still scheduling `source-health/*` and `auth-refresh/*` jobs for that source | `L3-FR-1.5.2`, `L3-FR-3.5.2`, `L3-FR-3.5.3` |
| `L3-T-cron_scheduler-010` | `T2` | `CronSchedulerHarness`, `TestStateDb` | source definitions remain rooted in `sources` while mutable health/cooldown changes write only to `source_runtime_state`; cron evaluation reads both surfaces without collapsing them into one table | `L3-FR-1.3.2` |

### 8.8 `job_claim.rs`

| Test ID | Tier | Fixtures | Exact Assertions | Requirements |
|--------|------|----------|------------------|-------------|
| `L3-T-job_claim-001` | `T2` | `TestStateDb` | concurrent claimers cannot double-claim the same runnable job; at most one claimer receives a given `workflow_jobs.id` | `L3-FR-1.3.3` |
| `L3-T-job_claim-002` | `T2` | `TestStateDb` | claim eligibility and ordering remain `status IN ('pending','retry')` ordered by `COALESCE(scheduled_for, created_at), created_at`; running/completed jobs are never returned | `L3-FR-1.3.3` |
| `L3-T-job_claim-003` | `T2` | `TestStateDb` | a successful claim atomically sets `lease_owner`, `started_at`, and `attempt_count = attempt_count + 1` before execution begins | `L3-FR-1.3.3` |
| `L3-T-job_claim-004` | `T2` | `TestStateDb` | lease expiry returns a stuck claim to the runnable pool exactly once and does not duplicate authoritative work rows | `L3-FR-1.3.3` |
| `L3-T-job_claim-005` | `T1` | repository inventory | `jobs/claim.rs` is the single SQL claim/lease surface used by `JobRunner`; no competing ad hoc claimant remains in `jobs/runner.rs` | `L3-FR-1.3.3` |
| `L3-T-job_claim-006` | `T2` | `ResourceArbiterFixture` | `browser_slots` is a hard cap: when two browser-backed sessions are active, a third browser job is denied admission and remains queued | `L3-FR-1.3.6`, `L3-FR-1.4.1` |
| `L3-T-job_claim-007` | `T2` | `ResourceArbiterFixture` | `agent_slots` is a hard ceiling for concurrent adapter-mediated work; validator sub-jobs consume only `agent_slots` and never consume `browser_slots` or `source_lanes` | `L3-FR-1.4.2`, `L3-FR-1.4.5` |
| `L3-T-job_claim-008` | `T2` | `ResourceArbiterFixture` | `source_lanes` enforce per-source concurrency while allowing unrelated sources to proceed when global capacity remains available | `L3-FR-1.4.3` |
| `L3-T-job_claim-009` | `T2` | `ResourceArbiterFixture`, `TelemetryProbe` | skill-level `max_concurrency` can lower effective ceilings but can never raise daemon-configured quotas; grant/deny/release events remain keyed by job/session identity | `L3-FR-1.4.4`, `L3-FR-1.4.5` |

### 8.9 `skill_golden_journeys.rs`

| Test ID | Tier | Fixtures | Exact Assertions | Requirements |
|--------|------|----------|------------------|-------------|
| `L3-T-skill_golden_journeys-001` | `T3` | `TestSkillDir`, `DaemonHarness` | `draft` skills run in scratch/manual mode only, bind to no scheduler roots, and create no authoritative pipeline-state commits | `L3-FR-3.1.1` |
| `L3-T-skill_golden_journeys-002` | `T3` | `TestSkillDir`, `ValidatorHarness` | `candidate` runs always write scratch manifests and always spawn validator sub-jobs; no authoritative production state is mutated | `L3-FR-3.1.2` |
| `L3-T-skill_golden_journeys-003` | `T3` | `TestSkillDir`, `DaemonHarness` | `canary` routing sends only the configured bounded traffic share or shard to the candidate version and retains stable-authoritative comparison data | `L3-FR-3.1.3` |
| `L3-T-skill_golden_journeys-004` | `T3` | `TestSkillDir`, `DaemonHarness` | `shadow` mode runs stable and candidate on full traffic while only stable outputs remain authoritative | `L3-FR-3.1.4` |
| `L3-T-skill_golden_journeys-005` | `T3` | `TestSkillDir`, `DaemonHarness` | `stable` is the scheduler default and `rollback` rebinds to the prior stable version without skipping intermediate promotion stages | `L3-FR-3.1.5`, `L3-FR-3.1.6`, `L3-FR-3.1.7` |
| `L3-T-skill_golden_journeys-006` | `T2` | `TestSkillDir` | promotion attempts that skip stages are rejected deterministically; validator approval thresholds remain part of the promotion gate inputs | `L3-FR-3.1.7` |
| `L3-T-skill_golden_journeys-007` | `T1` | `TestSkillDir` | the Layer 3 skill inventory contains the four operational directories plus `skills/validation/base/` with its required file set and declared interaction profiles | `L3-FR-1.9.1`, `L3-FR-2.8.1`, `L3-FR-3.3.1` |
| `L3-T-skill_golden_journeys-008` | `T3` | `TestSkillDir`, `SourceHealthFixture` | auth-refresh skills only establish/publish auth-state handles and source-health skills only execute the six-check contract; neither family silently takes on unrelated extraction responsibilities | `L3-FR-3.3.1`, `L3-FR-3.4.1`, `L3-FR-3.8.1`, `L3-FR-3.8.2` |

### 8.10 `skill_shadow_compare.rs`

| Test ID | Tier | Fixtures | Exact Assertions | Requirements |
|--------|------|----------|------------------|-------------|
| `L3-T-skill_shadow_compare-001` | `T3` | `TestSkillDir`, `TelemetryProbe` | canary exit requires at least `20` bounded-production runs with quality metrics within tolerance of stable and no precision/recall regression versus the gold set | `L3-FR-3.1.3` |
| `L3-T-skill_shadow_compare-002` | `T3` | `TestSkillDir`, `TelemetryProbe` | shadow exit requires at least `50` paired runs with shadow quality at least equal to stable or explicitly within tolerance | `L3-FR-3.1.4` |
| `L3-T-skill_shadow_compare-003` | `T3` | `TestSkillDir`, `ValidatorHarness` | promotion from candidate/canary is blocked when validator approval rates miss the declared threshold even if fill-rate metrics look acceptable | `L3-FR-3.1.3`, `L3-FR-3.1.7` |
| `L3-T-skill_shadow_compare-004` | `T1` | `ValidationPolicyFixture` | cosmetic prompt edits marked non-behavior-affecting do not bump `worker_behavior_version` and do not reset slice readiness | `L3-FR-3.2.2` |
| `L3-T-skill_shadow_compare-005` | `T1` | `ValidationPolicyFixture` | result-schema or behavior-affecting playbook changes bump `worker_behavior_version` and clear prior slice readiness evidence | `L3-FR-3.2.1`, `L3-FR-3.2.2` |
| `L3-T-skill_shadow_compare-006` | `T2` | `TestSkillDir`, `TelemetryProbe` | rollback demotes the failed stable version and rebinds the source to the prior stable version without mutating historical quality evidence | `L3-FR-3.1.6` |
| `L3-T-skill_shadow_compare-007` | `T2` | `TelemetryProbe` | shadow comparison records fill-rate, duplicate-rate, cost, and validator approval metrics for both stable and candidate variants and exposes them to promotion decisions | `L3-FR-3.1.3`-`L3-FR-3.1.7`, `L3-FR-4.5.1` |

### 8.11 `skill_drift_monitor.rs`

| Test ID | Tier | Fixtures | Exact Assertions | Requirements |
|--------|------|----------|------------------|-------------|
| `L3-T-skill_drift_monitor-001` | `T2` | `SourceHealthFixture` | source-health canaries execute all six checks and map failures to logical canary classes plus existing top-level `ErrorCode` values with structured context | `L3-FR-3.4.1` |
| `L3-T-skill_drift_monitor-002` | `T2` | `SourceHealthFixture`, `TestStateDb` | active-source canaries default to a 30-minute cadence and persist passed, failed, and timed-out outcomes explicitly in `source_runtime_state` | `L3-FR-3.4.2` |
| `L3-T-skill_drift_monitor-003` | `T2` | `CircuitBreakerFixture` | after `N=3` consecutive challenge-class failures within the time window, the circuit enters `open`, respects the default two-hour cooldown, and returns through `half_open` only | `L3-FR-3.5.1`-`L3-FR-3.5.4` |
| `L3-T-skill_drift_monitor-004` | `T2` | `CircuitBreakerFixture` | stale canary successes cannot close a newer `open` or `half_open` circuit because the fresher timestamp wins transactionally per `source_id` | `L3-FR-3.5.4` |
| `L3-T-skill_drift_monitor-005` | `T1` | `SourceHealthFixture` | anti-bot difficulty ratings and mitigation strategies remain source-specific inputs to pacing and expectations and are not replaced by generic retry loops | `L3-FR-3.7.1` |
| `L3-T-skill_drift_monitor-006` | `T1` | `TelemetryProbe` | non-applicable skill telemetry metrics emit `null`/`not_applicable` and are excluded from alert math rather than being recorded as synthetic zeroes | `L3-FR-4.5.1` |

### 8.12 `validation_policy.rs`

| Test ID | Tier | Fixtures | Exact Assertions | Requirements |
|--------|------|----------|------------------|-------------|
| `L3-T-validation_policy-001` | `T1` | `ValidationPolicyFixture` | schema-invalid worker outputs never spawn semantic validator jobs and stay quarantined as `AGENT_OUTPUT_CONTRACT_INVALID` | `L3-FR-2.3.1` |
| `L3-T-validation_policy-002` | `T1` | `ValidationPolicyFixture` | `draft`, `candidate`, `canary`, and `shadow` stages force validation before mode-specific branches are evaluated | `L3-FR-2.3.2` |
| `L3-T-validation_policy-003` | `T1` | `ValidationPolicyFixture` | `validation_policy: always` validates every eligible run without downsampling based on load or cost | `L3-FR-2.3.3` |
| `L3-T-validation_policy-004` | `T1` | `ValidationPolicyFixture` | `validation_policy: sample` uses `max(validation_sample_rate, risk_multiplier)` and records an explainable decision reason | `L3-FR-2.3.4` |
| `L3-T-validation_policy-005` | `T1` | `ValidationPolicyFixture` | source health amber/red forces effective sample probability `1.0` | `L3-FR-2.3.4` |
| `L3-T-validation_policy-006` | `T1` | `ValidationPolicyFixture` | fingerprint unknown/no baseline forces effective sample probability `1.0` | `L3-FR-2.3.4` |
| `L3-T-validation_policy-007` | `T1` | `ValidationPolicyFixture` | first run after a worker behavior-version change forces validation regardless of the nominal sample rate | `L3-FR-2.3.4`, `L3-FR-3.2.2` |
| `L3-T-validation_policy-008` | `T1` | `ValidationPolicyFixture` | continuation chain length greater than `1` or high locator fallback depth forces validation at `1.0` probability | `L3-FR-2.3.4` |
| `L3-T-validation_policy-009` | `T1` | `ValidationPolicyFixture` | low recent `basis_sufficiency` or same-slice validator error spikes force validation in sample mode | `L3-FR-2.3.4` |
| `L3-T-validation_policy-010` | `T1` | `ValidationPolicyFixture` | high-value opportunity or `suspect_output_in_last_5` forces validation in sample mode | `L3-FR-2.3.4` |
| `L3-T-validation_policy-011` | `T1` | `ValidationPolicyFixture` | `validation_policy: trigger` validates only when one of the declared trigger conditions fires | `L3-FR-2.3.5` |
| `L3-T-validation_policy-012` | `T1` | `ValidationPolicyFixture` | `validation_policy: skip` suppresses semantic validation but still runs OQG, while global override triggers can still force validation | `L3-FR-2.3.6` |
| `L3-T-validation_policy-013` | `T1` | `ValidationPolicyFixture` | trigger evaluation persists enough structured reason data to prove exactly why validation did or did not run | `L3-FR-2.3.7` |
| `L3-T-validation_policy-014` | `T1` | `ValidationPolicyFixture` | the enforcement-readiness slice key is exactly `worker skill x source x worker behavior version x validator behavior version`; cosmetic edits do not reset readiness | `L3-FR-2.3.8`, `L3-FR-3.2.2` |
| `L3-T-validation_policy-015` | `T1` | `ValidationPolicyFixture` | demotion triggers immediately return a slice to dry-run/flag-only and block re-promotion until calibration gates are re-met | `L3-FR-2.3.9` |
| `L3-T-validation_policy-016` | `T1` | `ValidationPolicyFixture` | bootstrap, calibrating, signature-limited, and full-slice enforcement ladder phases use the exact validation rates and gate semantics declared in the spec | `L3-FR-2.3.10` |

### 8.13 `validation_executor.rs`

| Test ID | Tier | Fixtures | Exact Assertions | Requirements |
|--------|------|----------|------------------|-------------|
| `L3-T-validation_executor-001` | `T2` | `ValidatorHarness`, `TestStateDb` | validator sub-jobs enqueue through `EnqueueJobRequest` with `parent_job_id` linked to the worker job and continuation-chain identity preserved when present | `L3-FR-2.4.1`, `L3-FR-2.5.2` |
| `L3-T-validation_executor-002` | `T2` | `ValidatorHarness` | validator jobs use `headless_contract`, `structured_extract_low_cost`, and the base validator skill only; they never request browser slots or source lanes | `L3-FR-1.4.5`, `L3-FR-2.4.3` |
| `L3-T-validation_executor-003` | `T2` | `ValidatorHarness`, `ManifestAssert` | kickoff prompt composition includes manifest, rubric hash, structured drift deltas, OQG signals, and anomaly-stratified sample while excluding raw forensic bytes, internal reasoning, and the full result contract | `L3-FR-2.4.2` |
| `L3-T-validation_executor-004` | `T2` | `ValidatorHarness` | the validator result contract must contain `verdict`, `basis_sufficiency`, `enforcement_eligible`, `confidence_bps`, `issues`, and `summary`; missing any required field is a schema-invalid validator failure | `L3-FR-2.4.4` |
| `L3-T-validation_executor-005` | `T2` | `ValidatorHarness`, `TestStateDb` | an `approved` verdict persists one `validation_verdicts` row plus the worker-job linkage atomically in the same SQLite transaction | `L3-FR-2.4.4`, `L3-FR-4.1.1`, `L3-FR-4.2.1` |
| `L3-T-validation_executor-006` | `T2` | `ValidatorHarness`, `TestStateDb` | `flagged` and `basis_sufficiency: low` do not create a new worker `ValidationDisposition`; they persist trust-state/validation metadata while the worker output remains accepted | `L3-FR-2.4.4` |
| `L3-T-validation_executor-007` | `T2` | `ValidatorHarness`, `TestStateDb` | in Layer 3 dry-run, `rejected + enforcement_eligible` persists the verdict but still advances the worker path and does not quarantine authoritative rows | `L3-FR-2.4.4`, `L3-FR-2.6.1` |
| `L3-T-validation_executor-008` | `T2` | `ValidatorHarness`, `TestStateDb` | an unknown verdict string triggers fail-open handling, produces a validator error manifest, and leaves the worker result `unverified` | `L3-FR-2.4.4`, `L3-FR-2.7.1` |
| `L3-T-validation_executor-009` | `T2` | `ValidatorHarness`, `TestStateDb` | a schema-invalid validator payload triggers fail-open handling and never silently coerces to `approved`, `rejected`, or `flagged` | `L3-FR-2.4.4`, `L3-FR-2.7.1` |
| `L3-T-validation_executor-010` | `T2` | `ValidatorHarness`, `TestStateDb` | a rendered-but-unpersisted verdict is treated as validator failure, not as a silent success; the worker row receives no `validation_verdict_id` | `L3-FR-2.4.4`, `L3-FR-2.7.1` |
| `L3-T-validation_executor-011` | `T2` | `ValidatorHarness`, `TestStateDb` | continuation segments are validated independently but the final chain-close stage head remains blocked until all segments are validated or dry-run-cleared | `L3-FR-2.5.2` |
| `L3-T-validation_executor-012` | `T2` | `ManifestAssert`, `ArtifactSandbox` | the run-manifest writer extends `RunManifestFrontmatter` and `DaemonEvidenceAppendix` with the L3 appendix additions and still emits a fallback machine-authored manifest when the agent narrative is missing | `L3-FR-1.3.12`, `L3-FR-2.1.1`-`L3-FR-2.2.2` |

### 8.14 `validation_dry_run.rs`

| Test ID | Tier | Fixtures | Exact Assertions | Requirements |
|--------|------|----------|------------------|-------------|
| `L3-T-validation_dry_run-001` | `T2` | `ValidatorHarness`, `TestStateDb` | a dry-run rejected verdict does not advance authoritative stage heads; the worker stage head matches the non-validation success path exactly | `L3-FR-2.6.1` |
| `L3-T-validation_dry_run-002` | `T2` | `ValidatorHarness`, `TestStateDb` | dry-run verdicts create no outbox rows or sync-visible side effects | `L3-FR-2.6.1` |
| `L3-T-validation_dry_run-003` | `T2` | `ArtifactSandbox`, `ValidatorHarness` | dry-run artifacts are written under `artifacts/scratch/<run_id>/` and never under production session artifact paths | `L3-FR-2.6.1` |
| `L3-T-validation_dry_run-004` | `T2` | `ValidatorHarness`, `TestStateDb` | approval, rejection, and flagging all insert `validation_verdicts` rows while the worker pipeline continues regardless of verdict class | `L3-FR-2.6.1` |
| `L3-T-validation_dry_run-005` | `T2` | `ValidatorHarness`, `TestStateDb` | worker `visibleAfterSeq` and authoritative stage progression remain identical whether dry-run validation approves, rejects, flags, or is skipped | `L3-FR-2.6.1` |
| `L3-T-validation_dry_run-006` | `T2` | `ValidatorHarness`, `ManifestAssert` | missing OQG signals or drift deltas explicitly lower `basis_sufficiency` or keep the result `unverified`; missing inputs never imply a clean semantic run | `L3-FR-2.2.2`, `L3-FR-2.7.1` |
| `L3-T-validation_dry_run-007` | `T2` | `ValidatorHarness`, `TestStateDb` | validator crash/timeout sets `VALIDATION_SKIPPED_VALIDATOR_ERROR`, emits a validator error manifest, and queues a retroactive backlog item while the worker result advances | `L3-FR-2.7.1` |
| `L3-T-validation_dry_run-008` | `T2` | `ValidatorHarness`, `TestStateDb` | a host-sleep-interrupted validator sub-job is handled as validator failure and never blocks or silently certifies the worker result | `L3-FR-2.7.1` |
| `L3-T-validation_dry_run-009` | `T2` | `ValidatorHarness`, `TestStateDb` | per-output trust states map to coarse `pipeline_assessments.validation_state` exactly as specified: all verified -> `validated`, any flagged/suspect -> `flagged`, skipped/low-basis/fail-open -> `unverified` | `L3-FR-2.7.2`, `L3-FR-2.7.3` |
| `L3-T-validation_dry_run-010` | `T2` | `ValidatorHarness`, `TestStateDb` | dry-run corpus accumulation preserves provenance tags for `clean real`, `weird-but-valid`, and `mutated bad` cases used by later calibration | `L3-FR-2.6.2`, `L3-FR-2.9.1` |

### 8.15 `validation_golden_verdicts.rs`

| Test ID | Tier | Fixtures | Exact Assertions | Requirements |
|--------|------|----------|------------------|-------------|
| `L3-T-validation_golden_verdicts-001` | `T1` | `TestSkillDir` | `skills/validation/base/` contains `SKILL.md`, `result.schema.json`, `checkpoint.schema.json`, `canary.inputs.jsonl`, `golden.assertions.yaml`, `validator-rubric.yaml`, and `references/manifest-corpus-index.md` | `L3-FR-2.8.1` |
| `L3-T-validation_golden_verdicts-002` | `T2` | `ValidatorHarness` | a clean-real manifest with complete appendix evidence is approved with non-low `basis_sufficiency` | `L3-FR-2.8.2`, `L3-FR-2.9.1` |
| `L3-T-validation_golden_verdicts-003` | `T2` | `ValidatorHarness` | a mutated-bad manifest is rejected and includes explicit issue categories plus evidence refs from manifest or daemon appendix anchors | `L3-FR-2.8.2`, `L3-FR-2.9.1` |
| `L3-T-validation_golden_verdicts-004` | `T2` | `ValidatorHarness` | a weird-but-valid or ambiguous manifest is flagged or low-basis rather than rejected as a false positive | `L3-FR-2.8.2`, `L3-FR-2.9.1` |
| `L3-T-validation_golden_verdicts-005` | `T2` | `ValidatorHarness` | thin appendix evidence yields `basis_sufficiency: low`; the base validator does not guess domain semantics from missing data | `L3-FR-2.8.2` |
| `L3-T-validation_golden_verdicts-006` | `T3` | `ValidatorHarness` | the cross-model audit subset records comparison outcomes against a different model family and surfaces divergences explicitly rather than overwriting the primary calibration judgment | `L3-FR-2.9.2` |
| `L3-T-validation_golden_verdicts-007` | `T1` | calibration report fixture | calibration reports compute and assert the numerical targets: `>=90%` mutated-bad catch rate, `>=95%` clean-real approval, false positives `<5%`, and Wilson-bound precision at `95%` confidence | `L3-FR-2.9.2` |

### 8.16 `validation_canary_health.rs`

| Test ID | Tier | Fixtures | Exact Assertions | Requirements |
|--------|------|----------|------------------|-------------|
| `L3-T-validation_canary_health-001` | `T3` | `ValidatorHarness`, `TestSkillDir` | synthetic canaries run every `15-30` minutes through the same executor, model family, schema path, and base validator skill used in production | `L3-FR-2.7.5` |
| `L3-T-validation_canary_health-002` | `T2` | `TelemetryProbe`, `ValidatorHarness` | canary pass-rate metrics update only when canary verdicts are actually persisted; missing persistence counts as execution failure | `L3-FR-4.6.1` |
| `L3-T-validation_canary_health-003` | `T2` | `TelemetryProbe` | validator execution failure rate above `2000` bps over one hour transitions subsystem health to `degraded` | `L3-FR-2.7.4`, `L3-FR-4.6.2` |
| `L3-T-validation_canary_health-004` | `T2` | `TelemetryProbe` | validator execution failure rate above `5000` bps over one hour transitions subsystem health to `blind` | `L3-FR-2.7.4`, `L3-FR-4.6.2` |
| `L3-T-validation_canary_health-005` | `T2` | `ValidatorHarness`, `TelemetryProbe` | any canary misclassification can force `degraded` or `blind` immediately and creates a blackout incident record with affected time window | `L3-FR-2.7.4`, `L3-FR-2.7.5`, `L3-FR-4.6.2` |
| `L3-T-validation_canary_health-006` | `T2` | `ValidationPolicyFixture`, `TelemetryProbe` | entering `blind` auto-demotes every affected enforcement-ready slice back to dry-run/flag-only status | `L3-FR-2.7.4`, `L3-FR-2.7.5` |
| `L3-T-validation_canary_health-007` | `T1` | `TelemetryProbe` | validation telemetry records approval, rejection, flag, cost, operator override, execution failure, canary pass, and health-state metrics with the declared thresholds and enum values | `L3-FR-4.6.1`, `L3-FR-4.6.2` |
| `L3-T-validation_canary_health-008` | `T3` | `TelemetryProbe`, `DaemonHarness` | the operator morning queue aggregates a single validation blackout incident with trust-debt counts instead of one alert per missed validator run | `L3-FR-2.7.4`, `L3-FR-4.6.2` |

### 8.17 `validation_retro_backlog.rs`

| Test ID | Tier | Fixtures | Exact Assertions | Requirements |
|--------|------|----------|------------------|-------------|
| `L3-T-validation_retro_backlog-001` | `T2` | `ValidatorHarness`, `TestStateDb` | every fail-open pass-through creates one retroactive backlog item with worker identity, trust state, and priority inputs | `L3-FR-2.7.1` |
| `L3-T-validation_retro_backlog-002` | `T2` | `ValidatorHarness`, `TestStateDb` | backlog priority ordering is exactly `high-value -> operator-reviewed -> enforcement slices -> recent outputs` | `L3-FR-2.7.5` |
| `L3-T-validation_retro_backlog-003` | `T3` | `ValidatorHarness`, `TestStateDb` | validator recovery drains backlog items in priority order and persists retro verdicts without skipping earlier higher-priority items | `L3-FR-2.7.5` |
| `L3-T-validation_retro_backlog-004` | `T3` | `ValidatorHarness`, `TestStateDb` | a retroactive rejection marks underlying outputs `suspect`, flags process rollups, and marks affected decisions stale | `L3-FR-2.7.5` |
| `L3-T-validation_retro_backlog-005` | `T2` | `TestStateDb` | backlog items do not silently expire; high-value and operator-reviewed items remain queued until explicit audited resolution | `L3-FR-2.7.5` |
| `L3-T-validation_retro_backlog-006` | `T2` | `TelemetryProbe`, `DaemonHarness` | recovery surfaces a single summary event with “validated retroactively: N issues found” and updated status propagation | `L3-FR-2.7.5`, `L3-FR-4.6.1` |

### 8.18 `resolve-process-relations.test.ts`

| Test ID | Tier | Fixtures | Exact Assertions | Requirements |
|--------|------|----------|------------------|-------------|
| `L3-T-resolve_process_relations-001` | `T1` | `RelationScoringFixture.sameOcid` | same OCID pairs classify as `merged_same_process` with no heuristic fallback required | `L3-FR-3.6.1` |
| `L3-T-resolve_process_relations-002` | `T1` | `RelationScoringFixture.linkedNoticeChain` | explicit related-notice chains with plausible chronology classify as `merged_same_process` | `L3-FR-3.6.1` |
| `L3-T-resolve_process_relations-003` | `T1` | `RelationScoringFixture.frameworkCallOff` | framework-establishment versus call-off pairs classify as `related_not_merged` and keep separate process identities | `L3-FR-3.6.2` |
| `L3-T-resolve_process_relations-004` | `T1` | `RelationScoringFixture.conflictingReferences` | conflicting exact reference numbers hard-block merge even when soft similarity is otherwise high | `L3-FR-3.6.3`, `L3-FR-3.6.5` |
| `L3-T-resolve_process_relations-005` | `T1` | `RelationScoringFixture.differentLotStructure` | clearly different lot/value/reference structures with the same buyer classify as non-merge (`possible_duplicate` or `separate`), never auto-merge | `L3-FR-3.6.3`, `L3-FR-3.6.5` |
| `L3-T-resolve_process_relations-006` | `T1` | `RelationScoringFixture.highScoreNoBlocker` | heuristic score `>= 0.95` with no blocker classifies as `merged_same_process` | `L3-FR-3.6.5` |
| `L3-T-resolve_process_relations-007` | `T1` | `RelationScoringFixture.midScore` | heuristic scores from `0.75` to `0.94` classify as `related_not_merged` or `possible_duplicate` and create an operator-review-worthy relation edge | `L3-FR-3.6.2`, `L3-FR-3.6.3`, `L3-FR-3.6.5` |
| `L3-T-resolve_process_relations-008` | `T1` | `RelationScoringFixture.lowScore` | heuristic scores below `0.75` classify as `separate` and write no relation row | `L3-FR-3.6.4`, `L3-FR-4.1.4` |
| `L3-T-resolve_process_relations-009` | `T1` | `RelationScoringFixture.evidenceReplay` | decision evidence persists blocker hits, feature vector, `relation_model_version = 1`, and threshold bucket so the same pair replays to the same classification | `L3-FR-3.6.5`, `L3-FR-4.1.4` |
| `L3-T-resolve_process_relations-010` | `T1` | `RelationScoringFixture.transitiveRoots` | overlapping merges resolve both inputs to their canonical roots first and pick the survivor deterministically by earliest `created_at` then UUID | `L3-FR-3.6.6`, `L3-FR-4.1.5` |
| `L3-T-resolve_process_relations-011` | `T1` | `RelationScoringFixture.lateMerge` | a late merge marks existing operator intents/decisions stale only when such decisions exist and recomputes manifest/assessment at cluster scope | `L3-FR-3.6.7`, `L3-FR-4.1.5` |
| `L3-T-resolve_process_relations-012` | `T1` | `RelationScoringFixture.clusterVisibility` | `related_not_merged` and `possible_duplicate` clusters remain visible with trust-aware cache-sharing hints and without destructive auto-merge | `L3-FR-3.6.8`, `L3-FR-4.1.4` |

### 8.19 `schema_activation.rs`

| Test ID | Tier | Fixtures | Exact Assertions | Requirements |
|--------|------|----------|------------------|-------------|
| `L3-T-schema_activation-001` | `T2` | `TestStateDb` | applying the Layer 3 migration creates `validation_verdicts`, `source_runtime_state`, `listing_evidence`, `process_relations`, and `process_aliases` with the declared columns and key constraints | `L3-FR-4.1.1`-`L3-FR-4.1.5` |
| `L3-T-schema_activation-002` | `T2` | `TestStateDb` | `source_runtime_state` includes `circuit_state` and `last_canary_result` at migration time; runtime does not rely on opportunistic `ALTER TABLE` drift to discover L3 shape | `L3-FR-4.1.2` |
| `L3-T-schema_activation-003` | `T2` | `TestStateDb` | `validation_verdicts` insert and `workflow_jobs.validation_verdict_id` linkage commit atomically or not at all | `L3-FR-4.1.1`, `L3-FR-4.2.1` |
| `L3-T-schema_activation-004` | `T2` | `TestStateDb` | `listing_evidence` enforces `(source, source_listing_id)` uniqueness, defaults `trust_state = 'unverified'`, and records `source_job_id` provenance when present | `L3-FR-4.1.3` |
| `L3-T-schema_activation-005` | `T2` | `TestStateDb` | `process_relations` stores only `merged_same_process`, `related_not_merged`, and `possible_duplicate`; `separate` remains a computed outcome that writes no relation row | `L3-FR-4.1.4` |
| `L3-T-schema_activation-006` | `T2` | `TestStateDb` | `process_aliases` preserves `alias_process_id -> canonical_process_id` redirects with push-owned sync columns including `stage_run_id` | `L3-FR-4.1.5` |
| `L3-T-schema_activation-007` | `T2` | `TestStateDb` | live Layer 3 paths populate `workflow_jobs.parent_job_id`, `validation_required`, `validation_verdict_id`, and `agent_session_logs.run_manifest_path/run_manifest_hash` | `L3-FR-4.2.1` |
| `L3-T-schema_activation-008` | `T2` | `TestStateDb` | local-only tables stay excluded from push-owned sync/outbox behavior and push-owned tables retain standard sync columns; no mixed-write authority violation occurs | `L3-FR-4.3.1` |

### 8.20 `observability_telemetry.rs`

| Test ID | Tier | Fixtures | Exact Assertions | Requirements |
|--------|------|----------|------------------|-------------|
| `L3-T-observability_telemetry-001` | `T2` | `TelemetryProbe`, `DaemonHarness` | control events record resource grants/denials/releases, circuit transitions, spawn, signal, exit, reap, and cleanup verdicts with explicit job/session keys | `L3-FR-1.3.8`, `L3-FR-4.4.1` |
| `L3-T-observability_telemetry-002` | `T2` | `TelemetryProbe`, `PtySessionHarness`, `ContractSessionHarness` | protocol events record adapter events, checkpoint progression, and result-contract events for both PTY and contract sessions in the same structured trace surface | `L3-FR-1.3.8`, `L3-FR-4.4.2` |
| `L3-T-observability_telemetry-003` | `T2` | `TelemetryProbe`, `ManifestAssert`, `ArtifactSandbox` | verdicts, manifests, structured traces, and forensic bytes remain joinable by `job_id`, `session_id`, `process_id`, and `run_manifest_hash` without transcript scraping | `L3-FR-4.4.3` |
| `L3-T-observability_telemetry-004` | `T1` | retention config fixture | retention policy configuration matches the declared minima: control/protocol `30` days, forensic bytes `14` days or longer if linked, manifests/verdicts `180` days | `L3-FR-4.4.3` |
| `L3-T-observability_telemetry-005` | `T1` | `TelemetryProbe` | all 13 skill-level metrics emit at the declared update points and baseline/threshold semantics match the spec table | `L3-FR-4.5.1` |
| `L3-T-observability_telemetry-006` | `T1` | `TelemetryProbe` | all 8 validation telemetry metrics emit only on verdict persistence, canary completion, or health-state transition; missing persistence counts as failure | `L3-FR-4.6.1` |
| `L3-T-observability_telemetry-007` | `T2` | `ManifestAssert`, `ArtifactSandbox`, `TelemetryProbe` | a 3 AM failure drill-down can traverse verdict -> manifest -> forensic bytes -> structured trace entirely through explicit IDs and refs | `L3-FR-4.4.1`-`L3-FR-4.4.3` |
| `L3-T-observability_telemetry-008` | `T1` | `ManifestAssert` | every `RunManifestFrontmatter` field, every core `DaemonEvidenceAppendix` field, and every L3 appendix addition has at least one direct assertion-bearing test in the suite | `L3-FR-2.1.2`, `L3-FR-2.1.4`, `L3-FR-2.2.1` |
| `L3-T-observability_telemetry-009` | `T1` | `ManifestAssert` | `RunManifestFrontmatter` key parity is asserted by exact field name against the current struct: `manifest_version`, `skill_id`, `skill_version`, `job_id`, `session_id`, `execution_mode`, `started_at`, `completed_at`, `outcome_class`, `items_processed`, `items_skipped`, `items_failed`, `warnings_count`, `agent_claimed_pages_visited`, `agent_claimed_source_url`, `agent_claimed_fingerprint_state`, `agent_claimed_locale_state`, `agent_claimed_currency_state`, and `agent_narrative_missing` | `L3-FR-2.1.2` |
| `L3-T-observability_telemetry-010` | `T1` | `ManifestAssert` | core `DaemonEvidenceAppendix` key parity is asserted by exact field name against the current struct: `observed_url_chain`, `observed_page_kind_sequence`, `daemon_observed_fingerprint_state`, `daemon_observed_locale`, `daemon_observed_currency`, `auth_challenge_events`, `locator_fallback_depth`, `checkpoint_count`, `result_item_count`, `oqg_anomaly_summary`, `representative_item_sample`, and `artifact_refs` | `L3-FR-2.1.4` |
| `L3-T-observability_telemetry-011` | `T1` | `ManifestAssert` | Layer 3 appendix additions are asserted by exact field name and absence-handling contract: `basis_sufficiency`, `page_fingerprint_hash`, `auth_challenge_detected`, `extraction_fill_rate_bps`, `checkpoint_count`, `byte_count_compressed`, `validator_rubric_hash`, and `drift_delta_json` | `L3-FR-2.2.1`, `L3-FR-2.2.2` |
| `L3-T-observability_telemetry-012` | `T1` | enum inventory fixture | the current checked-in `ErrorCode` inventory contains exactly `28` variants, their `as_str()` values are asserted by name, and Layer 3 logical canary classes are verified to map onto that existing enum rather than inventing hidden top-level codes | `L3-FR-3.4.1`, `L3-FR-1.8.1` |
| `L3-T-observability_telemetry-013` | `T2` | `OperationalStatusHarness` | `get_cron_status` dispatches through `CommandRouter`, rejects invalid params, and returns a read-only snapshot of cron definitions / next-run state without mutating `commit_seq` | `L3-FR-1.3.1`, `L3-FR-4.4.1` |
| `L3-T-observability_telemetry-014` | `T2` | `OperationalStatusHarness` | `get_source_health_status` returns source health, cooldown, and breaker state from `source_runtime_state` as a read-only status surface and rejects malformed params | `L3-FR-3.4.2`, `L3-FR-4.1.2`, `L3-FR-4.4.1` |
| `L3-T-observability_telemetry-015` | `T2` | `OperationalStatusHarness` | `get_validation_status` and `get_resource_arbiter_status` each dispatch through `CommandRouter`, expose health/telemetry or quota snapshots without mutating runtime state, and reject invalid params with `invalid_params` rather than silent defaults | `L3-FR-1.4.5`, `L3-FR-2.7.4`, `L3-FR-4.5.1`, `L3-FR-4.6.1` |

### 8.21 `render-error-manifest.test.ts`

| Test ID | Tier | Fixtures | Exact Assertions | Requirements |
|--------|------|----------|------------------|-------------|
| `L3-T-render_error_manifest-001` | `T1` | error-manifest fixture | rendered error-manifest markdown includes error chain, job/process/session context, next debugging action, linked artifacts, and resolution fields in stable section order | `L3-FR-1.8.1` |
| `L3-T-render_error_manifest-002` | `T1` | error-manifest fixture | logical source-health classes render using the mapped existing `ErrorCode` strings plus structured context rather than inventing undocumented top-level codes | `L3-FR-1.8.1`, `L3-FR-3.4.1` |
| `L3-T-render_error_manifest-003` | `T1` | resolved-manifest fixture | resolving an error manifest flips `resolved` metadata while preserving the original failure message, context, and linked markdown path | `L3-FR-1.8.1` |
| `L3-T-render_error_manifest-004` | `T1` | hostile-markdown fixture | hostile error context is escaped or fenced so raw HTML, scripts, and unsafe links do not execute in rendered output | `L3-FR-1.8.1` |

## 9. Coverage Matrix

| Spec Requirement | Test ID(s) | Test Type | Status |
|------------------|-----------|-----------|--------|
| `L3-FR-1.3.1` | `CI-L3-001`, `L3-T-cron_scheduler-001`, `L3-T-cron_scheduler-003`, `L3-T-cron_scheduler-006` | Static + UT + IT | Covered |
| `L3-FR-1.3.2` | `L3-T-cron_scheduler-002`, `L3-T-cron_scheduler-005`, `L3-T-cron_scheduler-010` | UT + IT | Covered |
| `L3-FR-1.3.3` | `L3-T-job_claim-001`, `L3-T-job_claim-002`, `L3-T-job_claim-003`, `L3-T-job_claim-004`, `L3-T-job_claim-005` | IT + UT | Covered |
| `L3-FR-1.3.4` | `L3-T-fault_process_cleanup-001`-`008` | IT + T3 | Covered |
| `L3-FR-1.3.5` | `L3-T-fault_pty_edge_cases-001`, `002`, `003`, `006` | IT | Covered |
| `L3-FR-1.3.6` | `L3-T-job_claim-006`, `007`, `008`, `009` | IT | Covered |
| `L3-FR-1.3.7` | `L3-T-fault_partial_failure-001`, `002`, `005`, `007` | IT | Covered |
| `L3-FR-1.3.8` | `L3-T-replay_pty_sessions-001`, `005`, `L3-T-replay_contract_sessions-001`, `002`, `L3-T-observability_telemetry-001`, `002` | T2 + T3 | Covered |
| `L3-FR-1.3.9` | `L3-T-replay_pty_sessions-001`, `002`, `004`, `L3-T-replay_contract_sessions-001`, `L3-T-replay_contract_sessions-006` | T2 + T3 | Covered |
| `L3-FR-1.3.10` | `L3-T-validation_policy-001`-`016` | UT | Covered |
| `L3-FR-1.3.11` | `L3-T-validation_executor-001`-`012` | IT | Covered |
| `L3-FR-1.3.12` | `L3-T-validation_executor-012` | IT | Covered |
| `L3-FR-1.4.1` | `L3-T-job_claim-006`, `L3-T-fault_host_events-003`, `004` | IT | Covered |
| `L3-FR-1.4.2` | `L3-T-job_claim-007` | IT | Covered |
| `L3-FR-1.4.3` | `L3-T-job_claim-008`, `L3-T-fault_host_events-004` | IT | Covered |
| `L3-FR-1.4.4` | `L3-T-job_claim-009`, `L3-T-fault_host_events-003`, `004` | IT | Covered |
| `L3-FR-1.4.5` | `L3-T-job_claim-007`, `009`, `L3-T-validation_executor-002` | IT | Covered |
| `L3-FR-1.5.1` | `L3-T-cron_scheduler-001`, `003`, `004` | UT + IT | Covered |
| `L3-FR-1.5.2` | `L3-T-cron_scheduler-006`, `007`, `008`, `009`, `L3-T-fault_host_events-005`, `006` | IT | Covered |
| `L3-FR-1.6.1` | `L3-T-fault_process_cleanup-001`, `003`, `006`, `008`, `L3-T-fault_pty_edge_cases-001`-`008` | IT | Covered |
| `L3-FR-1.7.1` | `L3-T-fault_host_events-001`-`010` | UT + IT | Covered |
| `L3-FR-1.8.1` | `CI-L3-004`, `L3-T-render_error_manifest-001`-`004`, `L3-T-resolve_process_relations-001`-`012`, `L3-T-observability_telemetry-012` | Static + UT | Covered |
| `L3-FR-1.9.1` | `CI-L3-005`, `L3-T-skill_golden_journeys-007` | Static + UT | Covered |
| `L3-FR-2.1.1` | `L3-T-validation_executor-012` | IT | Covered |
| `L3-FR-2.1.2` | `L3-T-observability_telemetry-008`, `009` | UT | Covered |
| `L3-FR-2.1.3` | `L3-T-validation_executor-012`, `L3-T-observability_telemetry-008` | IT + UT | Covered |
| `L3-FR-2.1.4` | `L3-T-replay_pty_sessions-006`, `L3-T-validation_executor-012`, `L3-T-observability_telemetry-008`, `010` | T2 + T3 | Covered |
| `L3-FR-2.1.5` | `L3-T-validation_executor-012`, `L3-T-observability_telemetry-008` | IT + UT | Covered |
| `L3-FR-2.2.1` | `L3-T-validation_executor-012`, `L3-T-observability_telemetry-008`, `011` | IT + UT | Covered |
| `L3-FR-2.2.2` | `L3-T-validation_executor-003`, `L3-T-validation_dry_run-006`, `L3-T-observability_telemetry-011` | IT | Covered |
| `L3-FR-2.3.1` | `L3-T-validation_policy-001` | UT | Covered |
| `L3-FR-2.3.2` | `L3-T-validation_policy-002` | UT | Covered |
| `L3-FR-2.3.3` | `L3-T-validation_policy-003` | UT | Covered |
| `L3-FR-2.3.4` | `L3-T-validation_policy-004`-`010` | UT | Covered |
| `L3-FR-2.3.5` | `L3-T-validation_policy-011` | UT | Covered |
| `L3-FR-2.3.6` | `L3-T-validation_policy-012` | UT | Covered |
| `L3-FR-2.3.7` | `L3-T-validation_policy-013` | UT | Covered |
| `L3-FR-2.3.8` | `L3-T-validation_policy-014`, `L3-T-skill_shadow_compare-004`, `005` | UT | Covered |
| `L3-FR-2.3.9` | `L3-T-validation_policy-015`, `L3-T-validation_canary_health-006` | UT + IT | Covered |
| `L3-FR-2.3.10` | `L3-T-validation_policy-016` | UT | Covered |
| `L3-FR-2.4.1` | `L3-T-validation_executor-001`, `L3-T-replay_contract_sessions-003` | IT | Covered |
| `L3-FR-2.4.2` | `L3-T-validation_executor-003`, `L3-T-replay_contract_sessions-004` | IT + T3 | Covered |
| `L3-FR-2.4.3` | `L3-T-validation_executor-002` | IT | Covered |
| `L3-FR-2.4.4` | `L3-T-fault_partial_failure-005`, `006`, `L3-T-validation_executor-004`-`010` | IT | Covered |
| `L3-FR-2.5.1` | `L3-T-fault_partial_failure-001`, `004` | IT | Covered |
| `L3-FR-2.5.2` | `L3-T-fault_partial_failure-003`, `004`, `L3-T-validation_executor-001`, `011` | IT | Covered |
| `L3-FR-2.6.1` | `L3-T-validation_dry_run-001`-`005`, `L3-T-validation_executor-007` | IT | Covered |
| `L3-FR-2.6.2` | `L3-T-validation_dry_run-010` | IT | Covered |
| `L3-FR-2.7.1` | `L3-T-fault_partial_failure-006`, `L3-T-validation_executor-008`-`010`, `L3-T-validation_dry_run-006`-`008`, `L3-T-validation_retro_backlog-001` | IT | Covered |
| `L3-FR-2.7.2` | `L3-T-validation_dry_run-009` | IT | Covered |
| `L3-FR-2.7.3` | `L3-T-validation_dry_run-009` | IT | Covered |
| `L3-FR-2.7.4` | `L3-T-validation_canary_health-003`-`008` | T2 + T3 | Covered |
| `L3-FR-2.7.5` | `L3-T-validation_canary_health-001`, `005`, `006`, `008`, `L3-T-validation_retro_backlog-002`-`006` | T2 + T3 | Covered |
| `L3-FR-2.8.1` | `L3-T-skill_golden_journeys-007`, `L3-T-validation_golden_verdicts-001` | UT | Covered |
| `L3-FR-2.8.2` | `L3-T-validation_golden_verdicts-002`-`005` | T2 | Covered |
| `L3-FR-2.9.1` | `L3-T-validation_dry_run-010`, `L3-T-validation_golden_verdicts-002`-`004` | IT + T2 | Covered |
| `L3-FR-2.9.2` | `L3-T-validation_golden_verdicts-006`, `007` | T1 + T3 | Covered |
| `L3-FR-2.9.3` | `L3-T-validation_golden_verdicts-007`, `L3-T-validation_policy-014`, `015` | UT + T1 | Covered |
| `L3-FR-3.1.1` | `L3-T-skill_golden_journeys-001` | T3 | Covered |
| `L3-FR-3.1.2` | `L3-T-skill_golden_journeys-002` | T3 | Covered |
| `L3-FR-3.1.3` | `L3-T-skill_golden_journeys-003`, `L3-T-skill_shadow_compare-001`, `003`, `007` | T3 + T2 | Covered |
| `L3-FR-3.1.4` | `L3-T-skill_golden_journeys-004`, `L3-T-skill_shadow_compare-002` | T3 | Covered |
| `L3-FR-3.1.5` | `L3-T-skill_golden_journeys-005` | T3 | Covered |
| `L3-FR-3.1.6` | `L3-T-skill_golden_journeys-005`, `L3-T-skill_shadow_compare-006` | T3 + T2 | Covered |
| `L3-FR-3.1.7` | `L3-T-skill_golden_journeys-005`, `006`, `L3-T-skill_shadow_compare-003` | T2 + T3 | Covered |
| `L3-FR-3.2.1` | `L3-T-skill_shadow_compare-005` | T1 | Covered |
| `L3-FR-3.2.2` | `L3-T-validation_policy-007`, `014`, `L3-T-skill_shadow_compare-004`, `005` | T1 | Covered |
| `L3-FR-3.3.1` | `L3-T-skill_golden_journeys-007`, `008` | UT + T3 | Covered |
| `L3-FR-3.4.1` | `L3-T-skill_golden_journeys-008`, `L3-T-skill_drift_monitor-001`, `L3-T-render_error_manifest-002`, `L3-T-observability_telemetry-012` | T1 + T2 + T3 | Covered |
| `L3-FR-3.4.2` | `L3-T-skill_drift_monitor-002`, `L3-T-cron_scheduler-009`, `L3-T-observability_telemetry-014` | T2 | Covered |
| `L3-FR-3.5.1` | `L3-T-skill_drift_monitor-003` | T2 | Covered |
| `L3-FR-3.5.2` | `L3-T-cron_scheduler-009`, `L3-T-skill_drift_monitor-003` | T2 | Covered |
| `L3-FR-3.5.3` | `L3-T-cron_scheduler-009`, `L3-T-skill_drift_monitor-003` | T2 | Covered |
| `L3-FR-3.5.4` | `L3-T-skill_drift_monitor-003`, `004` | T2 | Covered |
| `L3-FR-3.6.1` | `L3-T-resolve_process_relations-001`, `002` | T1 | Covered |
| `L3-FR-3.6.2` | `L3-T-resolve_process_relations-003`, `007` | T1 | Covered |
| `L3-FR-3.6.3` | `L3-T-resolve_process_relations-004`, `005`, `007` | T1 | Covered |
| `L3-FR-3.6.4` | `L3-T-resolve_process_relations-008` | T1 | Covered |
| `L3-FR-3.6.5` | `L3-T-resolve_process_relations-004`-`009` | T1 | Covered |
| `L3-FR-3.6.6` | `L3-T-resolve_process_relations-010` | T1 | Covered |
| `L3-FR-3.6.7` | `L3-T-resolve_process_relations-011` | T1 | Covered |
| `L3-FR-3.6.8` | `L3-T-resolve_process_relations-012` | T1 | Covered |
| `L3-FR-3.7.1` | `L3-T-skill_drift_monitor-005` | T1 | Covered |
| `L3-FR-3.8.1` | `L3-T-skill_golden_journeys-008` | T3 | Covered |
| `L3-FR-3.8.2` | `L3-T-skill_golden_journeys-008` | T3 | Covered |
| `L3-FR-4.1.1` | `L3-T-validation_executor-005`, `L3-T-schema_activation-001`, `003` | T2 | Covered |
| `L3-FR-4.1.2` | `L3-T-cron_scheduler-009`, `L3-T-skill_drift_monitor-002`, `004`, `L3-T-schema_activation-002` | T2 | Covered |
| `L3-FR-4.1.3` | `L3-T-schema_activation-004` | T2 | Covered |
| `L3-FR-4.1.4` | `L3-T-resolve_process_relations-008`, `009`, `012`, `L3-T-schema_activation-005` | T1 + T2 | Covered |
| `L3-FR-4.1.5` | `L3-T-resolve_process_relations-010`, `011`, `L3-T-schema_activation-006` | T1 + T2 | Covered |
| `L3-FR-4.2.1` | `L3-T-validation_executor-005`, `L3-T-schema_activation-003`, `007` | T2 | Covered |
| `L3-FR-4.3.1` | `CI-L3-005`, `L3-T-schema_activation-008` | Static + T2 | Covered |
| `L3-FR-4.4.1` | `L3-T-fault_process_cleanup-004`, `007`, `L3-T-replay_pty_sessions-001`, `003`, `L3-T-replay_contract_sessions-002`, `L3-T-observability_telemetry-001`, `007`, `013`, `014` | T2 + T3 | Covered |
| `L3-FR-4.4.2` | `L3-T-replay_pty_sessions-001`, `005`, `L3-T-replay_contract_sessions-001`, `L3-T-observability_telemetry-002` | T2 + T3 | Covered |
| `L3-FR-4.4.3` | `L3-T-replay_pty_sessions-003`, `004`, `006`, `L3-T-replay_contract_sessions-004`, `005`, `006`, `L3-T-observability_telemetry-003`, `004`, `007` | T2 + T3 | Covered |
| `L3-FR-4.5.1` | `L3-T-skill_shadow_compare-007`, `L3-T-skill_drift_monitor-006`, `L3-T-observability_telemetry-005`, `015` | T1 + T2 | Covered |
| `L3-FR-4.6.1` | `L3-T-validation_canary_health-002`, `007`, `L3-T-validation_retro_backlog-006`, `L3-T-observability_telemetry-006`, `015` | T1 + T2 + T3 | Covered |
| `L3-FR-4.6.2` | `L3-T-validation_canary_health-003`-`008` | T2 + T3 | Covered |

## 10. Coverage Summary

| Metric | Value |
|--------|-------|
| Unique requirements in spec | `98` |
| Requirements with at least one test | `98` |
| Coverage percentage | `100%` |
| Static gates | `8` |
| Runnable tests | `183` |
| Total checks | `191` |
| Spec-predeclared new files covered | `18 / 18` |
| Additive auxiliary files | `3` |
| Orphan requirements | `0` |
| Duplicate test IDs | `0` |

## 11. Notes For Strategy / Beads

- The validation subsystem is intentionally over-represented in the inventory because it is the highest-risk Layer 3 surface and the spec explicitly expects a mode/trigger/verdict matrix rather than spot checks.
- If implementation planning insists on the exact cumulative file count from `layer-strategy.md` §11, fold `schema_activation.rs`, `observability_telemetry.rs`, and `render-error-manifest.test.ts` into neighboring files only after preserving every test ID and assertion in this document. They are separate here because hiding these surfaces inside other files would obscure critical coverage.
- The follow-up audit tightened four surfaces that were previously too implicit: exact run-manifest key parity, exact `ErrorCode` inventory parity, `HealthIssue`/`StallTracker` interaction coverage, and concrete `CommandRouter` status methods (`get_cron_status`, `get_source_health_status`, `get_validation_status`, `get_resource_arbiter_status`).
