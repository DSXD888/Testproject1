# L1 Daemon Core — TDD Specification

## 1. Scope

**Primary source of truth:** `docs/strategy/layers/layer-1-thin-local-runner.md`

**Supporting context:**
- `docs/strategy/layer-strategy.md` §11
- `docs/strategy/layers/layer-0-gold-set-evaluation.md` §2, §5, §6
- `docs/strategy/cross-reference-index.md` §Testing
- `AGENTS.md` §Toolchain, §Compiler Checks

**Target output of this document:** define exactly what tests must exist before implementation of the Layer 1 Rust daemon core begins.

**Coverage target:** all `138` unique `L1-FR-*` requirement IDs in the validated L1 spec map to at least one test ID in this document.

**Planned test inventory:** `150` test IDs total
- `6` static gates
- `47` unit tests
- `57` integration tests
- `40` E2E / scheduled-manual tests

## 2. Non-Negotiable Test Principles

- Use Cargo and Rust 2024 nightly only for daemon tests.
- Use `#[test]` for sync logic and `#[tokio::test]` for async/runtime paths.
- Do not mock SQLite. Use `rusqlite::Connection::open_in_memory()` for unit scope and temp-file databases for integration/E2E scope.
- Do not mock process supervision. Use real helper binaries and real process groups.
- Any test that claims PTY/session/reaper behavior must prove it with real helper binaries plus OS primitives (`openpty`, `setsid`, `waitpid`, `killpg`, real signals). Pure policy tests must say they are policy-only.
- Do not scrape transcript text for authoritative assertions when the spec says control lane / contracts are authoritative.
- Do not use `.skip`, `#[ignore]`, empty test bodies, existence-only assertions, or vague “works” checks.
- Treat PTY capture, lifecycle transitions, run manifests, and error manifests as first-class release-gate surfaces.
- Keep all test code `#![forbid(unsafe_code)]`.
- Per-commit CI means cumulative `T1 + T2` under five minutes on a standard hosted runner; deterministic nightly CI stays under 30 minutes; the live 10-batch Experiment 4 gate runs only on a dedicated networked runner or target workstation.

## 3. Test Pyramid

| Layer | Purpose | Allowed Medium | Disallowed Shortcuts | Primary Evidence |
|------|---------|----------------|----------------------|------------------|
| Static Gates | Build, lint, formatting, inventory, unsafe policy | `cargo check`, `cargo clippy`, `cargo fmt`, manifest/inventory assertions | Manual eyeballing | command exit status, generated reports |
| Unit | Pure policy, parsers, renderer rules, schema/manifest validation | in-memory SQLite, in-process parser fixtures, temp dirs | process mocks, fake DB APIs | assertion logs, exact values |
| Integration | Real PTY/process, temp-file SQLite, local UDS, artifact files | helper binaries, real file system, real `waitpid`, real zstd chunks | mocked `killpg`, mocked `waitpid`, mocked WAL | artifact paths, DB rows, lifecycle events |
| E2E | Full daemon path, real skill directory, captured evidence or live source, real agent CLI where required | daemon binary + helper binaries + live/captured skill inputs | transcript-only success criteria | session artifacts, DB state, manifests, CLI summaries |
| Manual Release Gates | Live FTS run, operator-visible summary, Experiment 4 confirmation on target workstation | actual local daemon and agent CLIs | replay-only signoff | manifest artifact, zero-orphan report |

## 4. CI Tiering And Cadence

| Tier | Budget | Cadence | Includes |
|------|--------|---------|----------|
| `T1` | `< 90 sec` | every commit | static gates, pure parser tests, mode-resolution matrix tests, skill lint, manifest/error-manifest renderers |
| `T2` | `< 4 min` | every commit | temp-file SQLite integration, L0→L1 migration bootstrap, IPC handshake, result-contract integration, PTY/session semantics smoke with real helper binaries |
| `T3` | `< 20 min` | nightly | crash recovery, chunk recovery, stubborn-process cleanup, captured-evidence canary, captured 10-run reliability soak |
| `T4` | `< 30 min` | scheduled dedicated networked runner + pre-release | live FTS single-batch smoke, live CLI canary, live adapter availability checks |
| `Manual Gate` | operator time | milestone signoff / target-host confirmation | 10 consecutive live daemon-managed batches, live manifest inspection, zero-orphan confirmation on the intended workstation |

## 5. Shared Test Infrastructure

| Harness / Fixture | Scope | Purpose |
|------------------|-------|---------|
| `TestStateDb` | UT / IT / E2E | Creates in-memory or temp-file `state.db` with Layer 0 DDL + L1 migration applied, WAL enabled, `commit_clock` seeded |
| `ArtifactSandbox` | IT / E2E | Creates per-test artifact root, verifies chunk files, hashes, manifests, cleanup reports |
| `PtySessionHarness` | IT / E2E | Spawns helper binaries under PTY, records `sid/pid/pgid`, drains output, exposes lifecycle frames |
| `ContractSessionHarness` | IT / E2E | Runs helper contract binary, feeds `Stdout/Stderr` frames, validates checkpoint and final-result paths |
| `DaemonHarness` | IT / E2E | Boots daemon runtime with temp state root, lock path, descriptor path, UDS path, and injected skill roots |
| `TestSkillDir` | UT / IT / E2E | Materializes valid and invalid skill directory variants for `skill_lint.rs` and runner binding tests |
| `EvidenceBundleFixture` | E2E | Replays captured FTS responses for deterministic nightly E2E lanes without pretending replay equals live signoff |
| `LiveFtsCanaryContext` | E2E / Manual | Holds live source config, timeout budget, network prerequisites, and source cursor seeds |
| `ProcessTreeHelper` | IT / E2E | Helper binary launcher for child, grandchild, stubborn child, auth challenge, stall, stdout/stderr, and crash scenarios |
| `ProcessIsolationGuard` | IT / E2E | Enforces unique temp roots, serial-only tests for shared signal/lock surfaces, bounded teardown, and zero-descendant assertions after every real-process test |
| `ManifestAssert` | UT / IT / E2E | Validates YAML frontmatter, disagreement summary, appendix fields, Markdown sections, hash/size contract |

`ProcessTreeHelper` must support, at minimum, helper modes for: session-semantics reporting (`isatty`, controlling-tty path, `sid/pid/pgid`), silent stall/no-output, ignoring both `SIGINT` and `SIGTERM`, well-formed JSONL, malformed JSONL, partial/truncated JSONL, well-formed stream-JSON, malformed/truncated stream-JSON, checkpoint-then-crash, forked grandchild, double-fork escape, stdout/stderr split output, explicit exit codes, and auth-challenge emission. PTY/reaper tests that touch real signal handlers, OS locks, or shared CLI homes must run under `ProcessIsolationGuard` and assert zero surviving descendants during teardown.

## 6. File Inventory

| Surface | Count | Notes |
|--------|-------|-------|
| Static gates | 6 | not tied to a single Rust test file |
| `agent_pty_session.rs` | 22 | PTY + lifecycle + chunk recovery |
| `agent_contract_session.rs` | 18 | two-lane contract execution and runner persistence |
| `agent_procurement_ingestion.rs` | 26 | live loop, stage tracking, pipeline/error behavior |
| `adapter_codex.rs` | 11 | Codex adapter trait contract |
| `adapter_claude.rs` | 11 | Claude adapter trait contract |
| `adapter_result_contract.rs` | 13 | contract validation + manifest semantics |
| `skill_lint.rs` | 8 | CI gate for skill directory correctness |
| `skill_canary_harness.rs` | 9 | source/browser/contract drift smoke coverage |
| `session_reaper.rs` | 13 | explicit reap, cleanup verdicts, daemon-crash recovery |
| `ipc_handshake.rs` | 13 | descriptor, lock, handshake, restart rules, UDS auth |

## 7. Static Gates

| Test ID | Tier | Tag | What it proves | Requirements |
|--------|------|-----|----------------|-------------|
| `CI-CORE-001` | `T1` | `bootstrap/daemon/P0` | `cargo check --all-targets` succeeds for the Layer 1 workspace surface | `L1-FR-DAEMON.2`, `L1-FR-DAEMON.3` |
| `CI-CORE-002` | `T1` | `bootstrap/daemon/P0` | `cargo clippy --all-targets -- -D warnings` succeeds with no warnings | `L1-FR-DAEMON.2`, `L1-FR-DAEMON.3` |
| `CI-CORE-003` | `T1` | `bootstrap/daemon/P0` | `cargo fmt --check` passes for production and test code | `L1-FR-DAEMON.2` |
| `CI-CORE-004` | `T1` | `bootstrap/daemon/P0` | runtime component inventory matches the 17 daemon files, 2 skills, and `0003_sqlite_runtime.sql` declared in the spec/manifest | `L1-FR-DAEMON.2` |
| `CI-CORE-005` | `T1` | `bootstrap/db/P0` | Cargo manifests declare the active Layer 1 dependency set and exclude later-layer-only runtime dependencies unless needed by active behavior | `L1-FR-DAEMON.3` |
| `CI-CORE-006` | `T1` | `bootstrap/daemon/P0` | repository-wide Rust production/test targets keep `#![forbid(unsafe_code)]` and contain no `unsafe` blocks | supporting invariant for all L1 tests |

## 8. Detailed Test Catalog

### 8.1 `agent_pty_session.rs`

| Test ID | Tier | Tag | What it proves | Requirements |
|--------|------|-----|----------------|-------------|
| `UT-PTY-001` | `T1` | `session/pty/P0` | PTY master/slave pair is created before child spawn begins | `L1-FR-PTY.1` |
| `UT-PTY-002` | `T1` | `session/pty/P0` | child spawn policy requires `setsid` -> controlling-terminal setup -> stdio redirection ordering before `exec` | `L1-FR-PTY.1` |
| `UT-PTY-003` | `T1` | `session/daemon/P0` | `sid`, `pid`, and `pgid` are durably recorded before the session is marked `spawned` | `L1-FR-PTY.1`, `L1-FR-DAEMON.5` |
| `UT-PTY-004` | `T1` | `session/pty/P0` | PTY master and daemon control fds are marked `CLOEXEC` | `L1-FR-PTY.2` |
| `UT-PTY-005` | `T1` | `session/pty/P0` | spawn path chooses helper-binary pre-exec route when Tokio fork-safety rules would be violated | `L1-FR-PTY.3` |
| `UT-PTY-006` | `T1` | `session/pty/P1` | default PTY geometry is `120x40` and is stored separately from browser viewport config | `L1-FR-PTY.4` |
| `UT-PTY-007` | `T1` | `session/pty/P1` | `resize_pty(session_id, cols, rows)` mutates PTY geometry only when invoked | `L1-FR-PTY.4` |
| `IT-PTY-008` | `T2` | `capture/artifacts/P0` | PTY output frames are sequenced losslessly into capture order before semantic events are consumed | `L1-FR-ADAPTER.6`, `L1-FR-ADAPTER.14`, `L1-FR-PIPELINE.8` |
| `IT-PTY-009` | `T3` | `capture/artifacts/P0` | exact `32KB` compressed boundary rotates chunks correctly with contiguous sequence ranges | `L1-FR-ARTIFACTS.2`, `L1-FR-ARTIFACTS.3`, `L1-FR-ARTIFACTS.4` |
| `IT-PTY-010` | `T3` | `capture/artifacts/P1` | zero-byte / near-zero-byte PTY sessions still finalize index state without exposing partial chunks | `L1-FR-ARTIFACTS.2`, `L1-FR-ARTIFACTS.3`, `L1-FR-ARTIFACTS.4` |
| `IT-PTY-011` | `T3` | `recovery/artifacts/P0` | crash recovery truncates torn trailing JSONL lines and rebuilds longest contiguous committed prefix | `L1-FR-ARTIFACTS.3`, `L1-FR-ARTIFACTS.4` |
| `IT-PTY-020` | `T3` | `capture/artifacts/P0` | zstd-compressed chunks round-trip the original bytes and the recorded SHA-256 matches the durable chunk payload | `L1-FR-ARTIFACTS.3`, `L1-FR-ARTIFACTS.4` |
| `IT-PTY-021` | `T3` | `recovery/artifacts/P0` | chunk writes use temp-file then atomic-rename semantics; crash mid-write never exposes a partial final-name chunk and recovery ignores stray temp files | `L1-FR-ARTIFACTS.4` |
| `IT-PTY-012` | `T2` | `lifecycle/daemon/P0` | nominal PTY session emits `created -> spawned -> active -> terminating -> exited -> reaped -> archived` in order | `L1-FR-PIPELINE.14`, `L1-FR-DAEMON.4`-`L1-FR-DAEMON.12` |
| `IT-PTY-013` | `T2` | `lifecycle/daemon/P0` | pre-spawn failure normalizes through terminal phases with null runtime IDs instead of leaving session stranded in `created` | `L1-FR-DAEMON.4`, `L1-FR-DAEMON.12` |
| `IT-PTY-014` | `T2` | `lifecycle/daemon/P0` | child exit immediately after spawn still records zero-progress `active` before exit handling | `L1-FR-DAEMON.6`, `L1-FR-DAEMON.12` |
| `IT-PTY-015` | `T2` | `lifecycle/reaper/P0` | unexpected PTY EOF triggers `terminating` and cleanup rather than silent success | `L1-FR-DAEMON.7`, `L1-FR-REAPER.2` |
| `IT-PTY-016` | `T2` | `lifecycle/reaper/P0` | PTY `EIO` during drain is treated as expected hangup, not standalone session failure | `L1-FR-DAEMON.8`, `L1-FR-REAPER.2` |
| `E2E-PTY-017` | `T3` | `capture/artifacts/P0` | real helper child produces compressed artifact files, metadata rows, byte counts, and hashes | `L1-FR-ARTIFACTS.1`-`L1-FR-ARTIFACTS.5`, `L1-FR-DB.5` |
| `E2E-PTY-018` | `T3` | `capture/pty/P1` | transcript remains reproducible when no resize occurs and geometry stays constant | `L1-FR-PTY.4`, `L1-FR-ARTIFACTS.1` |
| `IT-PTY-019` | `T2` | `session/pty/P0` | controlling-terminal setup or `exec` failure aborts spawn with no surviving process group and a classified failure path | `L1-FR-PTY.1`, `L1-FR-PIPELINE.5`, `L1-FR-ERRORS.28` |
| `IT-PTY-022` | `T2` | `session/pty/P0` | real helper confirms `/dev/tty` access, `isatty(stdin/stdout/stderr)=true`, and session/pgid separation so PTY mode preserves interactive terminal semantics without contract-mode shortcuts | `L1-FR-ADAPTER.2`, `L1-FR-PTY.1`, `L1-FR-PTY.4` |

### 8.2 `agent_contract_session.rs`

| Test ID | Tier | Tag | What it proves | Requirements |
|--------|------|-----|----------------|-------------|
| `UT-CON-001` | `T1` | `adapter/contract/P0` | contract mode emits `Stdout(Bytes)` and `Stderr(Bytes)` losslessly and separately | `L1-FR-ADAPTER.3`, `L1-FR-ADAPTER.6` |
| `UT-CON-002` | `T1` | `adapter/contract/P0` | parser-derived control events are emitted only after the source frame is sequenced for capture | `L1-FR-ADAPTER.7`, `L1-FR-ADAPTER.14` |
| `UT-CON-003` | `T1` | `adapter/contracts/P0` | `FinalResultReady` exposes a path only and never certifies payload validity | `L1-FR-ADAPTER.7`, `L1-FR-ADAPTER.8` |
| `UT-CON-004` | `T1` | `adapter/policy/P0` | `ContractRequired` never degrades to PTY | `L1-FR-ADAPTER.1`, `L1-FR-ADAPTER.5`, `L1-FR-ADAPTER.12` |
| `UT-CON-005` | `T1` | `adapter/policy/P1` | `ContractPreferred` fallback behavior follows `allow_pty_fallback` exactly | `L1-FR-ADAPTER.5`, `L1-FR-ADAPTER.15` |
| `IT-CON-006` | `T2` | `runner/contracts/P0` | validated checkpoints update `last_checkpoint_seq` and persist in order | `L1-FR-PIPELINE.9`, `L1-FR-RUNNER.3`, `L1-FR-DB.5` |
| `IT-CON-007` | `T2` | `runner/contracts/P0` | out-of-order checkpoint sequencing is rejected without advancing authoritative state | `L1-FR-PIPELINE.9`, `L1-FR-ERRORS.26` |
| `IT-CON-008` | `T2` | `runner/contracts/P0` | schema-valid final result advances the runner success path only after validator success | `L1-FR-PIPELINE.11`, `L1-FR-RUNNER.3` |
| `IT-CON-009` | `T2` | `runner/errors/P0` | schema-invalid final result fails the job and writes error context | `L1-FR-PIPELINE.11`, `L1-FR-ERRORS.24`, `L1-FR-ERRORS.39` |
| `IT-CON-010` | `T2` | `runner/errors/P0` | OQG failure quarantines outputs and blocks pipeline-head advancement | `L1-FR-PIPELINE.11`, `L1-FR-ERRORS.25` |
| `IT-CON-011` | `T2` | `runner/errors/P0` | partial failure with valid checkpoint never upgrades to success without final contract + clean reap | `L1-FR-ERRORS.26`, `L1-FR-REAPER.1` |
| `IT-CON-012` | `T2` | `runner/db/P0` | SQLite write failure emits no new `visibleAfterSeq` and advances no stage head | `L1-FR-RUNNER.1`, `L1-FR-RUNNER.2`, `L1-FR-ERRORS.41` |
| `E2E-CON-013` | `T3` | `runner/contracts/P0` | manifest-generation skill completes in contract mode and persists result artifact + run manifest | `L1-FR-ADAPTER.3`, `L1-FR-SKILLS.3`, `L1-FR-ARTIFACTS.9` |
| `E2E-CON-014` | `T3` | `runner/errors/P0` | non-zero exit without a more specific pattern is classified `AGENT_RUN_FAILED` | `L1-FR-ERRORS.28`, `L1-FR-ERRORS.39` |
| `IT-CON-015` | `T2` | `runner/contracts/P1` | `continuation_required` preserves committed work and enqueues a follow-on segment with resume token | `L1-FR-RUNNER.6`, `L1-FR-PIPELINE.7` |
| `IT-CON-016` | `T2` | `adapter/contract/P1` | stderr-only contract sessions are preserved on the output lane without semantic loss | `L1-FR-ADAPTER.3`, `L1-FR-ADAPTER.6`, `L1-FR-ADAPTER.14` |
| `IT-CON-017` | `T2` | `runner/db/P0` | retrying a lost `MutationAck` with the same idempotency context returns the original `visibleAfterSeq` and writes no duplicate stage rows | `L1-FR-RUNNER.2`, `L1-FR-DB.2` |
| `IT-CON-018` | `T2` | `runner/db/P0` | concurrent WAL readers hold a stable snapshot during a write transaction, and a competing second writer that exhausts `busy_timeout` surfaces `SQLITE_WRITE_FAILED` with no new `visibleAfterSeq`, no `commit_seq` advance, and no stage-head movement | `L1-FR-RUNNER.1`, `L1-FR-RUNNER.2`, `L1-FR-ERRORS.41`, `L1-FR-DB.2`, `L1-FR-DB.3` |

### 8.3 `agent_procurement_ingestion.rs`

| Test ID | Tier | Tag | What it proves | Requirements |
|--------|------|-----|----------------|-------------|
| `E2E-ING-001` | `T4` | `pipeline/trigger/P0` | manual CLI trigger creates a `sync_source` job under daemon control | `L1-FR-PIPELINE.2`, `L1-FR-PIPELINE.3` |
| `E2E-ING-002` | `T4` | `pipeline/trigger/P0` | job creation persists explicit skill binding and resolved skill version before execution | `L1-FR-PIPELINE.3`, `L1-FR-SKILLS.1` |
| `E2E-ING-003` | `T4` | `pipeline/adapter/P0` | FTS skill resolves to PTY execution and fails explicitly if PTY requirements are unavailable | `L1-FR-PIPELINE.4`, `L1-FR-SKILLS.2` |
| `E2E-ING-004` | `T4` | `pipeline/session/P0` | live-loop spawn records runtime identifiers and classified failure paths when spawn fails | `L1-FR-PIPELINE.5`, `L1-FR-DB.5` |
| `E2E-ING-005` | `T4` | `pipeline/source/P0` | page fingerprint must be proven before extraction begins | `L1-FR-PIPELINE.6`, `L1-FR-ERRORS.35` |
| `E2E-ING-006` | `T4` | `pipeline/source/P0` | cursor backtrack window and durable batch-boundary advancement are enforced | `L1-FR-PIPELINE.7`, `L1-FR-DB.8`, `L1-FR-SKILLS.2` |
| `E2E-ING-007` | `T4` | `pipeline/source/P1` | empty FTS batch completes successfully and does not advance cursor past unseen data | `L1-FR-PIPELINE.7`, `L1-FR-DB.8` |
| `E2E-ING-008` | `T4` | `pipeline/capture/P0` | capture-write failure becomes a job failure and cannot yield false-success completion | `L1-FR-PIPELINE.8`, `L1-FR-ERRORS.41` |
| `E2E-ING-009` | `T4` | `pipeline/contracts/P0` | workflow progression uses validated checkpoints/results, not transcript scraping | `L1-FR-PIPELINE.9`, `L1-FR-ADAPTER.7`, `L1-FR-ADAPTER.8` |
| `E2E-ING-010` | `T4` | `pipeline/manifests/P0` | CLI-visible summary and local run/opportunity manifest artifacts are emitted from persisted pipeline state | `L1-FR-PIPELINE.1`, `L1-FR-PIPELINE.10`, `L1-FR-PIPELINE.15`, `L1-FR-SKILLS.3` |
| `E2E-ING-011` | `T4` | `pipeline/validation/P0` | schema validation and OQG jointly gate authoritative pipeline advancement | `L1-FR-PIPELINE.11`, `L1-FR-ERRORS.24`, `L1-FR-ERRORS.25` |
| `E2E-ING-012` | `T4` | `pipeline/stages/P0` | the live loop executes normalization through manifest using Layer 0 computation authority and writes the expected tables | `L1-FR-PIPELINE.12`, `L1-FR-DB.6`, `L1-FR-DB.9` |
| `E2E-ING-013` | `T4` | `pipeline/stages/P0` | stage outputs, `process_stage_status`, `process_stage_heads`, and `commit_clock` commit atomically | `L1-FR-PIPELINE.13`, `L1-FR-RUNNER.1`, `L1-FR-RUNNER.4`, `L1-FR-DB.2` |
| `E2E-ING-014` | `T4` | `pipeline/scopes/P0` | stage-to-scope mapping matches the six canonical scope kinds and Layer 1 matrix | `L1-FR-PIPELINE.17`, `L1-FR-PIPELINE.18` |
| `E2E-ING-015` | `T4` | `pipeline/scopes/P0` | set-replacement stages never leak partial authoritative writes or head movement | `L1-FR-PIPELINE.19`, `L1-FR-RUNNER.9` |
| `E2E-ING-016` | `T4` | `pipeline/scopes/P0` | additive stages can commit multiple scoped units from one session while failed siblings remain explicit failures | `L1-FR-PIPELINE.16`, `L1-FR-PIPELINE.19`, `L1-FR-RUNNER.5` |
| `E2E-ING-017` | `T4` | `pipeline/freshness/P0` | `expected_input_basis_hash` mismatch marks a job obsolete and enqueues rerun from the correct stage boundary | `L1-FR-PIPELINE.21`, `L1-FR-RUNNER.8` |
| `E2E-ING-018` | `T4` | `pipeline/freshness/P1` | soft-stale vs hard-invalid outputs follow the staleness cascade rules and head freshness states | `L1-FR-PIPELINE.20`, `L1-FR-PIPELINE.21` |
| `E2E-ING-019` | `T3` | `pipeline/schema/P1` | reserved forward-compatibility columns stay reserved in Layer 1 live writes | `L1-FR-DB.11` |
| `E2E-ING-020` | `Manual Gate` | `pipeline/reliability/P0` | Experiment 4 runs 10 consecutive live daemon-managed batches on the target host at `>=90%` success and `0` orphaned descendants | `L1-FR-DAEMON.1`, `L1-FR-PIPELINE.14` |
| `E2E-ING-021` | `T4` | `pipeline/errors/P1` | `SOURCE_CHANGED` and `NOTICE_PARSE_FAILED` preserve raw evidence and review context | `L1-FR-ERRORS.14`, `L1-FR-ERRORS.15`, `L1-FR-ERRORS.39` |
| `E2E-ING-022` | `T4` | `pipeline/errors/P1` | low-confidence classification, supplier match, and commodity-code outcomes route to review instead of silent auto-qualification | `L1-FR-ERRORS.16`, `L1-FR-ERRORS.17`, `L1-FR-ERRORS.18` |
| `E2E-ING-023` | `T4` | `pipeline/errors/P1` | uncertain revenue basis and FPO unavailability reduce confidence / continue conservatively instead of fabricating certainty | `L1-FR-ERRORS.19`, `L1-FR-ERRORS.20` |
| `E2E-ING-024` | `T3` | `pipeline/durability/P1` | each of the 14 live-loop steps writes only the durable surfaces declared by the authoritative step-to-write map | `L1-FR-PIPELINE.22` |
| `E2E-ING-025` | `T4` | `pipeline/output/P0` | `manifests` row references a local markdown opportunity artifact backed by persisted pipeline data | `L1-FR-PIPELINE.15`, `L1-FR-SKILLS.3` |
| `E2E-ING-026` | `T4` | `pipeline/scopes/P0` | one session exercising both notice-scope set-replacement and candidate-source additive stages proves set-replacement cohorts stay singular per full scope while additive stages emit multiple fresh `stage_run_id` values without reusing failed cohorts | `L1-FR-PIPELINE.16`, `L1-FR-PIPELINE.19`, `L1-FR-PIPELINE.20`, `L1-FR-RUNNER.5`, `L1-FR-RUNNER.9` |

### 8.4 `adapter_codex.rs`

| Test ID | Tier | Tag | What it proves | Requirements |
|--------|------|-----|----------------|-------------|
| `UT-CDX-001` | `T1` | `adapter/codex/P0` | `probe()` returns all normative capability fields | `L1-FR-ADAPTER.4`, `L1-FR-ADAPTER.11`, `L1-FR-ADAPTER.15` |
| `UT-CDX-002` | `T1` | `adapter/codex/P0` | unknown CLI versions enter canary mode | `L1-FR-ADAPTER.4`, `L1-FR-ADAPTER.9` |
| `UT-CDX-003` | `T1` | `adapter/codex/P0` | missing CLI / probe failure yields explicit failure rather than crash | `L1-FR-ADAPTER.4`, `L1-FR-ERRORS.27` |
| `UT-CDX-004` | `T1` | `adapter/codex/P0` | `resolve_mode()` honors `PtyRequired` deterministically | `L1-FR-ADAPTER.1`, `L1-FR-ADAPTER.12` |
| `UT-CDX-005` | `T1` | `adapter/codex/P0` | `resolve_mode()` honors `ContractRequired` deterministically | `L1-FR-ADAPTER.1`, `L1-FR-ADAPTER.12` |
| `UT-CDX-006` | `T1` | `adapter/codex/P1` | `resolve_mode()` runs the full six-row fallback matrix, including both explicit error rows, with deterministic outcomes and reasons | `L1-FR-ADAPTER.5`, `L1-FR-ADAPTER.12`, `L1-FR-ADAPTER.15` |
| `UT-CDX-007` | `T1` | `adapter/codex/P0` | `build_launch_spec()` returns spawn inputs only and isolates config/cwd per session | `L1-FR-ADAPTER.10`, `L1-FR-ADAPTER.13` |
| `UT-CDX-008` | `T1` | `adapter/codex/P0` | JSONL parser translates vendor output into canonical `Started/Checkpoint/Progress/FinalResultReady/Exited` events | `L1-FR-ADAPTER.7`, `L1-FR-ADAPTER.14` |
| `UT-CDX-009` | `T1` | `adapter/codex/P1` | `parser().finish()` runs once and produces terminal classification on abrupt stream end | `L1-FR-ADAPTER.18` |
| `UT-CDX-010` | `T1` | `adapter/codex/P1` | malformed JSONL frames and truncated tail-at-EOF cases yield explicit protocol-mismatch / parse failure with no fabricated control events or silent drops | `L1-FR-ADAPTER.14`, `L1-FR-ADAPTER.18`, `L1-FR-ERRORS.31` |
| `IT-CDX-010` | `T2` | `adapter/codex/P1` | CLI version changes invalidate cached probe results and force re-probe before next launch | `L1-FR-ADAPTER.11`, `L1-FR-ADAPTER.16`, `L1-FR-ADAPTER.17` |

### 8.5 `adapter_claude.rs`

| Test ID | Tier | Tag | What it proves | Requirements |
|--------|------|-----|----------------|-------------|
| `UT-CLD-001` | `T1` | `adapter/claude/P0` | `probe()` reports all normative capability fields for Claude | `L1-FR-ADAPTER.4`, `L1-FR-ADAPTER.11`, `L1-FR-ADAPTER.15` |
| `UT-CLD-002` | `T1` | `adapter/claude/P0` | unknown Claude CLI versions enter canary mode | `L1-FR-ADAPTER.4`, `L1-FR-ADAPTER.9` |
| `UT-CLD-003` | `T1` | `adapter/claude/P0` | probe failure classifies explicit adapter internal error | `L1-FR-ADAPTER.4`, `L1-FR-ERRORS.27` |
| `UT-CLD-004` | `T1` | `adapter/claude/P0` | `resolve_mode()` honors `PtyRequired` deterministically | `L1-FR-ADAPTER.1`, `L1-FR-ADAPTER.12` |
| `UT-CLD-005` | `T1` | `adapter/claude/P0` | `resolve_mode()` honors `ContractRequired` deterministically | `L1-FR-ADAPTER.1`, `L1-FR-ADAPTER.12` |
| `UT-CLD-006` | `T1` | `adapter/claude/P1` | `resolve_mode()` runs the full six-row fallback matrix, including both explicit error rows, with deterministic outcomes and reasons | `L1-FR-ADAPTER.5`, `L1-FR-ADAPTER.12`, `L1-FR-ADAPTER.15` |
| `UT-CLD-007` | `T1` | `adapter/claude/P0` | `build_launch_spec()` returns isolated launch inputs without process side effects | `L1-FR-ADAPTER.10`, `L1-FR-ADAPTER.13` |
| `UT-CLD-008` | `T1` | `adapter/claude/P0` | JSON/stream-JSON parser emits canonical control events while preserving raw output lane bytes | `L1-FR-ADAPTER.6`, `L1-FR-ADAPTER.7`, `L1-FR-ADAPTER.14` |
| `UT-CLD-009` | `T1` | `adapter/claude/P1` | `parser().finish()` runs exactly once on forced termination | `L1-FR-ADAPTER.18` |
| `UT-CLD-010` | `T1` | `adapter/claude/P1` | malformed JSON / invalid stream envelopes and truncated stream tails yield explicit protocol-mismatch / parse failure while complete multi-chunk objects still parse losslessly | `L1-FR-ADAPTER.14`, `L1-FR-ADAPTER.18`, `L1-FR-ERRORS.31` |
| `IT-CLD-010` | `T2` | `adapter/claude/P1` | CLI version changes invalidate cached probe results while adapter identity stays stable across sessions and shared instances keep state out of globals | `L1-FR-ADAPTER.11`, `L1-FR-ADAPTER.16`, `L1-FR-ADAPTER.17` |

### 8.6 `adapter_result_contract.rs`

| Test ID | Tier | Tag | What it proves | Requirements |
|--------|------|-----|----------------|-------------|
| `UT-RSL-001` | `T1` | `contracts/results/P0` | valid `result.schema.json` payloads pass structural validation | `L1-FR-PIPELINE.11`, `L1-FR-ADAPTER.8` |
| `UT-RSL-002` | `T1` | `contracts/manifests/P0` | required frontmatter fields round-trip through YAML parsing and remain mandatory for agent-authored run manifests | `L1-FR-ARTIFACTS.10` |
| `UT-RSL-003` | `T1` | `contracts/manifests/P0` | Summary / Steps Taken / Decisions Made / Warnings / Evidence sections must be present and non-empty | `L1-FR-ARTIFACTS.11` |
| `UT-RSL-004` | `T1` | `contracts/manifests/P0` | `agent_claimed_*` and `daemon_observed_*` naming never collide or collapse into ambiguous keys | `L1-FR-ARTIFACTS.7`, `L1-FR-ARTIFACTS.13` |
| `UT-RSL-005` | `T1` | `contracts/manifests/P1` | claim/observation disagreements append explicit summary instead of overwriting values | `L1-FR-ARTIFACTS.15` |
| `UT-RSL-006` | `T1` | `contracts/results/P1` | `continuation_required` is distinguished from `AGENT_PARTIAL_FAILURE` | `L1-FR-RUNNER.6`, `L1-FR-ERRORS.26` |
| `UT-RSL-007` | `T1` | `contracts/results/P1` | `degraded_completed` remains distinct from failure/quarantine outcomes | `L1-FR-PIPELINE.11`, `L1-FR-ARTIFACTS.9` |
| `IT-RSL-008` | `T2` | `contracts/manifests/P0` | daemon synthesizes fallback run manifest when agent narrative is missing | `L1-FR-PIPELINE.10`, `L1-FR-ARTIFACTS.8`, `L1-FR-ARTIFACTS.14` |
| `IT-RSL-009` | `T2` | `contracts/manifests/P0` | manifest path, finalized-byte SHA-256, and size budget are verified and persisted for finalized sessions | `L1-FR-ARTIFACTS.6`, `L1-FR-DB.5` |
| `IT-RSL-010` | `T2` | `errors/manifests/P0` | error-manifest Markdown renders header, pipeline summary, runtime context table, linked artifacts, next debugging action, and resolution fields | `L1-FR-ERRORS.13`, `L1-FR-ERRORS.39` |
| `IT-RSL-011` | `T2` | `contracts/manifests/P1` | daemon appendix contains checkpoint count, result count, OQG summary, and artifact refs from machine sources only | `L1-FR-ARTIFACTS.12` |
| `IT-RSL-012` | `T2` | `recovery/artifacts/P1` | manifest/index readers expose only the longest contiguous committed prefix after recovery | `L1-FR-ARTIFACTS.3`, `L1-FR-ARTIFACTS.4` |
| `IT-RSL-014` | `T2` | `errors/manifests/P1` | resolving an error manifest flips `resolved = false` to `true` with `resolved_at/resolved_by` while preserving original failure context and markdown path | `L1-FR-ERRORS.13`, `L1-FR-ERRORS.39` |

### 8.7 `skill_lint.rs`

| Test ID | Tier | Tag | What it proves | Requirements |
|--------|------|-----|----------------|-------------|
| `UT-SKL-001` | `T1` | `skills/lint/P0` | required files exist in every production skill directory | `L1-FR-SKILLS.1`, `L1-FR-SKILLS.4` |
| `UT-SKL-002` | `T1` | `skills/lint/P0` | missing `SKILL.md` fails lint immediately | `L1-FR-ERRORS.40`, `L1-FR-SKILLS.4` |
| `UT-SKL-003` | `T1` | `skills/lint/P0` | required metadata fields (`interaction_profile`, `result_schema_version`, `model_profile`) are mandatory | `L1-FR-SKILLS.1`, `L1-FR-SKILLS.4` |
| `UT-SKL-004` | `T1` | `skills/lint/P0` | trigger-boundary sections (`Use when`, `Do not use when`, `Inputs`, `Outputs`, `Non-goals`) are mandatory | `L1-FR-SKILLS.4` |
| `UT-SKL-005` | `T1` | `skills/lint/P0` | result schema, checkpoint schema, canary inputs, and golden assertions must parse and be non-empty | `L1-FR-SKILLS.4` |
| `UT-SKL-006` | `T1` | `skills/lint/P1` | failure-code enumeration and resume strategy must be declared | `L1-FR-SKILLS.4` |
| `UT-SKL-007` | `T1` | `skills/lint/P1` | bounded pagination / side-effect declarations are required where applicable | `L1-FR-SKILLS.4` |
| `UT-SKL-008` | `T1` | `skills/prompt/P0` | run-manifest instruction block is required so prompt rendering can satisfy the manifest contract | `L1-FR-PROMPT.1`, `L1-FR-SKILLS.4` |

### 8.8 `skill_canary_harness.rs`

| Test ID | Tier | Tag | What it proves | Requirements |
|--------|------|-----|----------------|-------------|
| `E2E-CAN-001` | `T3` | `skills/canary/P0` | FTS canary fetches minimum viable notice set and validates basic OCDS structure | `L1-FR-SKILLS.2` |
| `E2E-CAN-002` | `T3` | `skills/canary/P0` | page fingerprint match is checked before broader traffic depends on the skill | `L1-FR-SKILLS.2`, `L1-FR-ERRORS.35` |
| `E2E-CAN-003` | `T3` | `skills/canary/P1` | auth-state verification is enforced when a skill declares it, even if FTS itself is public | `L1-FR-ADAPTER.4`, `L1-FR-ERRORS.34` |
| `E2E-CAN-004` | `T3` | `skills/canary/P0` | protocol/version drift enters canary handling instead of production traffic | `L1-FR-ADAPTER.4`, `L1-FR-ERRORS.31` |
| `E2E-CAN-005` | `T3` | `skills/canary/P0` | page-shape drift yields classified failure with artifact capture | `L1-FR-ERRORS.35`, `L1-FR-ERRORS.39` |
| `E2E-CAN-006` | `T3` | `skills/canary/P1` | locale/currency drift fails fast and requires config review | `L1-FR-ERRORS.36` |
| `E2E-CAN-007` | `T3` | `skills/canary/P1` | source-soft-blocked responses preserve artifacts and fail explicitly | `L1-FR-ERRORS.32` |
| `E2E-CAN-008` | `T3` | `skills/canary/P1` | hard CAPTCHA block produces `blocked_human_action` outcome and stops autonomous progress | `L1-FR-ERRORS.33` |
| `E2E-CAN-009` | `T3` | `skills/canary/P1` | declared canary step failure emits `AGENT_SKILL_FAILED` with `skill_path` and `workflow_step` context instead of collapsing into a generic source failure | `L1-FR-ERRORS.23`, `L1-FR-ERRORS.39` |

### 8.9 `session_reaper.rs`

| Test ID | Tier | Tag | What it proves | Requirements |
|--------|------|-----|----------------|-------------|
| `IT-RPR-001` | `T2` | `reaper/core/P0` | real `SIGCHLD` delivery plus `waitpid(-1, WNOHANG)` drain loop associates exited pids with the right session | `L1-FR-REAPER.3` |
| `IT-RPR-002` | `T2` | `reaper/core/P0` | cleanup ladder obeys in-band interrupt -> PTY `^C` -> `killpg(SIGINT)` -> `killpg(SIGTERM)` -> `killpg(SIGKILL)` against a real process group | `L1-FR-REAPER.2`, `L1-FR-DAEMON.7` |
| `IT-RPR-003` | `T2` | `reaper/core/P0` | `clean` verdict is recorded when all descendants exit before deadline | `L1-FR-REAPER.1`, `L1-FR-REAPER.4` |
| `IT-RPR-004` | `T2` | `reaper/core/P0` | `forced` verdict is recorded when `SIGTERM` or `SIGKILL` is required | `L1-FR-REAPER.2`, `L1-FR-REAPER.4` |
| `IT-RPR-005` | `T2` | `reaper/core/P0` | `leaked` verdict is recorded when descendants survive group kill | `L1-FR-REAPER.4`, `L1-FR-ERRORS.29` |
| `IT-RPR-006` | `T2` | `reaper/core/P0` | orphaned grandchild is detected by descendant sweep after primary child exit | `L1-FR-REAPER.3`, `L1-FR-REAPER.5` |
| `IT-RPR-007` | `T2` | `reaper/artifacts/P0` | `cleanup_report.json` is emitted for every session with verdict and surviving pid details | `L1-FR-REAPER.3`, `L1-FR-DAEMON.9` |
| `IT-RPR-008` | `T3` | `reaper/recovery/P0` | daemon restart reconciles `spawned|active|terminating` sessions by killing survivors and synthesizing crash manifests | `L1-FR-IPC.4`, `L1-FR-ERRORS.30` |
| `IT-RPR-009` | `T2` | `reaper/drain/P0` | child exit before PTY hangup still waits for drain completion before `reaped` | `L1-FR-DAEMON.8`, `L1-FR-REAPER.1` |
| `IT-RPR-010` | `T3` | `reaper/platform/P1` | browser-helper escape case uses `kill(-pgid, ...)` first and descendant sweep second | `L1-FR-REAPER.5` |
| `IT-RPR-011` | `T3` | `reaper/health/P1` | host sleep interruptions and host resource pressure classify correctly during active sessions | `L1-FR-ERRORS.37`, `L1-FR-ERRORS.38` |
| `E2E-RPR-012` | `T3` | `reaper/reliability/P0` | captured/session-helper process-supervision stress run ends with zero orphaned descendants across ten sessions | `L1-FR-DAEMON.1`, `L1-FR-PIPELINE.14`, `L1-FR-REAPER.1`-`L1-FR-REAPER.5` |
| `IT-RPR-013` | `T2` | `reaper/core/P1` | reap of an unknown/unmapped pid is logged safely without corrupting session state | `L1-FR-REAPER.3`, `L1-FR-DAEMON.9` |

### 8.10 `ipc_handshake.rs`

| Test ID | Tier | Tag | What it proves | Requirements |
|--------|------|-----|----------------|-------------|
| `IT-IPC-001` | `T2` | `ipc/descriptor/P0` | `daemon.info.json` is written atomically only after lock, DB readiness, and socket bind succeed | `L1-FR-IPC.1`, `L1-FR-DB.4` |
| `IT-IPC-002` | `T2` | `ipc/lock/P0` | `daemon.lock` prevents a second writable daemon instance from starting | `L1-FR-IPC.2` |
| `IT-IPC-003` | `T2` | `ipc/handshake/P0` | successful `hello/welcome` negotiation returns protocol, schema, commit fence, health, and active sessions | `L1-FR-IPC.3` |
| `IT-IPC-004` | `T2` | `ipc/handshake/P0` | protocol-range mismatch is rejected explicitly and blocks further commands | `L1-FR-IPC.3` |
| `IT-IPC-005` | `T2` | `ipc/handshake/P0` | schema-epoch mismatch returns both expected and actual values and blocks stream/command use | `L1-FR-IPC.3`, `L1-FR-DB.4` |
| `IT-IPC-006` | `T2` | `ipc/restart/P1` | desktop restart while daemon stays alive allows resume from last seen sequence | `L1-FR-IPC.4` |
| `IT-IPC-007` | `T2` | `ipc/restart/P0` | daemon restart changes `instanceId` and invalidates PTY-stream resumability for pre-crash sessions | `L1-FR-IPC.4` |
| `IT-IPC-008` | `T2` | `ipc/surface/P0` | Layer 1 socket accepts handshake only and rejects mutation commands | `L1-FR-IPC.5` |
| `IT-IPC-009` | `T2` | `ipc/security/P0` | peer UID mismatch on pathname UDS is rejected | `L1-FR-IPC.7` |
| `IT-IPC-010` | `T2` | `ipc/startup/P0` | startup failures never advertise `ready: true` | `L1-FR-DB.3`, `L1-FR-DB.4`, `L1-FR-ERRORS.42` |
| `IT-IPC-011` | `T2` | `ipc/lifecycle/P1` | `archived` lifecycle frame carries `archiveSeq`, and `reaped` frame carries `cleanupVerdict` | `L1-FR-IPC.6`, `L1-FR-DAEMON.10` |
| `IT-IPC-012` | `T2` | `ipc/restart/P0` | handshake compares client `lastSeenInstanceId` to current `instanceId` and explicitly distinguishes resumable same-daemon reconnect from daemon-restart invalidation | `L1-FR-IPC.3`, `L1-FR-IPC.4` |
| `IT-IPC-013` | `T2` | `db/startup/P0` | startup applies `0001_sqlite_state_initial.sql` then `0003_sqlite_runtime.sql` inside the same `state.db`, seeds `commit_clock`, verifies WAL/`quick_check`/`user_version`, and creates no secondary metadata DB | `L1-FR-DB.1`, `L1-FR-DB.2`, `L1-FR-DB.3`, `L1-FR-DB.4`, `L1-FR-ERRORS.42` |

## 9. Edge-Case Scenario Matrix

| Scenario | Test ID(s) | Tier | Requirements |
|---------|------------|------|--------------|
| Agent crashes mid-session (`SIGKILL`) | `IT-CON-011`, `IT-RPR-008` | `T2/T3` | `L1-FR-ERRORS.26`, `L1-FR-ERRORS.30` |
| Orphaned grandchild process | `IT-RPR-006`, `IT-RPR-005` | `T2` | `L1-FR-REAPER.3`, `L1-FR-ERRORS.29` |
| PTY EOF received unexpectedly | `IT-PTY-015` | `T2` | `L1-FR-DAEMON.7`, `L1-FR-REAPER.2` |
| Agent CLI not found on PATH | `UT-CDX-003`, `UT-CLD-003` | `T1` | `L1-FR-ERRORS.27` |
| Agent exits immediately after spawn | `IT-PTY-014` | `T2` | `L1-FR-DAEMON.12` |
| Agent produces partial output then stalls | `IT-CON-011`, `IT-RPR-002` | `T2` | `L1-FR-ERRORS.21`, `L1-FR-ERRORS.26` |
| FTS API returns empty results | `E2E-ING-007` | `T4` | `L1-FR-PIPELINE.7` |
| FTS API rate limit / soft block | `E2E-CAN-007` | `T3` | `L1-FR-ERRORS.32` |
| Result schema validation fails | `IT-CON-009`, `UT-RSL-001` | `T1/T2` | `L1-FR-ERRORS.24` |
| Multiple candidates in one notice | `E2E-ING-014`, `E2E-ING-016` | `T4` | `L1-FR-PIPELINE.18`, `L1-FR-PIPELINE.19` |
| `stage_run_id` collision / uniqueness under parallel scopes | `E2E-ING-016` | `T4` | `L1-FR-PIPELINE.16`, `L1-FR-RUNNER.5` |
| Child exits before PTY hangup | `IT-RPR-009` | `T2` | `L1-FR-DAEMON.8`, `L1-FR-REAPER.1` |
| PTY master reports `EIO` before child reap | `IT-PTY-016` | `T2` | `L1-FR-DAEMON.8`, `L1-FR-REAPER.2` |
| Master fd inherited by grandchild | `UT-PTY-004`, `IT-RPR-006` | `T1/T2` | `L1-FR-PTY.2` |
| Daemon crashes during active PTY session | `IT-RPR-008`, `IT-IPC-007` | `T3/T2` | `L1-FR-IPC.4`, `L1-FR-ERRORS.30` |
| Desktop handshake sees schema epoch mismatch | `IT-IPC-005` | `T2` | `L1-FR-IPC.3`, `L1-FR-DB.4` |
| Skill directory missing required files | `UT-SKL-001`, `UT-SKL-002` | `T1` | `L1-FR-ERRORS.40` |
| SQLite commit fails during stage persistence | `IT-CON-012`, `E2E-ING-013` | `T2/T4` | `L1-FR-ERRORS.41` |
| Lock/migration/readiness fails at startup | `IT-IPC-001`, `IT-IPC-010` | `T2` | `L1-FR-ERRORS.42` |
| Lost `MutationAck` retried with same idempotency context | `IT-CON-017` | `T2` | `L1-FR-RUNNER.2` |
| L0 base schema plus `0003_sqlite_runtime.sql` migration bootstrap inside one `state.db` | `IT-IPC-013` | `T2` | `L1-FR-DB.1`-`L1-FR-DB.4`, `L1-FR-ERRORS.42` |
| WAL reader observes pre-commit snapshot while writer transaction is open | `IT-CON-018` | `T2` | `L1-FR-RUNNER.1`, `L1-FR-DB.2`, `L1-FR-DB.3` |
| Busy-timeout exhaustion on competing write connection | `IT-CON-018` | `T2` | `L1-FR-ERRORS.41`, `L1-FR-RUNNER.1`, `L1-FR-RUNNER.2` |
| Auth challenge requiring PTY input | `E2E-CAN-003`, `IT-RPR-011` | `T3` | `L1-FR-ERRORS.22` |
| Unknown CLI version enters canary | `UT-CDX-002`, `UT-CLD-002` | `T1` | `L1-FR-ADAPTER.9`, `L1-FR-ERRORS.31` |
| Malformed JSONL / stream-JSON output from adapter CLI | `UT-CDX-010`, `UT-CLD-010` | `T1` | `L1-FR-ADAPTER.14`, `L1-FR-ERRORS.31` |
| Invalid lifecycle transition `created -> active` | `IT-PTY-013`, `IT-PTY-014` | `T2` | `L1-FR-DAEMON.12` |
| Exact 32KB chunk boundary | `IT-PTY-009` | `T3` | `L1-FR-ARTIFACTS.2`-`L1-FR-ARTIFACTS.4` |
| Crash during temp-chunk write before rename | `IT-PTY-021` | `T3` | `L1-FR-ARTIFACTS.4` |
| SHA-256 mismatch on durable chunk | `IT-PTY-020` | `T3` | `L1-FR-ARTIFACTS.3`, `L1-FR-ARTIFACTS.4` |
| Zero-byte capture | `IT-PTY-010` | `T3` | `L1-FR-ARTIFACTS.2`-`L1-FR-ARTIFACTS.4` |
| Torn trailing `stream.index.jsonl` line | `IT-PTY-011`, `IT-RSL-012` | `T3/T2` | `L1-FR-ARTIFACTS.3`, `L1-FR-ARTIFACTS.4` |
| Controlling-terminal setup failure | `IT-PTY-019` | `T2` | `L1-FR-PTY.1` |
| Contract-required skill with unavailable structured output | `UT-CON-004`, `UT-CDX-005`, `UT-CLD-005` | `T1` | `L1-FR-ADAPTER.5` |
| Stderr-only contract run | `IT-CON-016` | `T2` | `L1-FR-ADAPTER.6` |
| Missing run-manifest frontmatter field | `UT-RSL-002` | `T1` | `L1-FR-ARTIFACTS.10` |
| Missing narrative section | `UT-RSL-003` | `T1` | `L1-FR-ARTIFACTS.11` |
| Run-manifest hash does not match finalized bytes | `IT-RSL-009` | `T2` | `L1-FR-ARTIFACTS.6`, `L1-FR-DB.5` |
| Agent/daemon disagreement on observed facts | `UT-RSL-005` | `T1` | `L1-FR-ARTIFACTS.15` |
| Error manifest resolution workflow updates status without mutating the original failure record | `IT-RSL-014` | `T2` | `L1-FR-ERRORS.13`, `L1-FR-ERRORS.39` |
| `continuation_required` vs `partial_failure` confusion | `UT-RSL-006`, `IT-CON-015` | `T1/T2` | `L1-FR-RUNNER.6` |
| Soft stale downstream inputs | `E2E-ING-018` | `T4` | `L1-FR-PIPELINE.20`, `L1-FR-PIPELINE.21` |
| Hard invalid downstream inputs | `E2E-ING-017`, `E2E-ING-018` | `T4` | `L1-FR-PIPELINE.21` |
| `SOURCE_CHANGED` from live source drift | `E2E-ING-021` | `T4` | `L1-FR-ERRORS.14` |
| Low-confidence classification / supplier match / commodity code | `E2E-ING-022` | `T4` | `L1-FR-ERRORS.16`-`L1-FR-ERRORS.18` |
| `FPO_ACCESS_UNAVAILABLE` fallback | `E2E-ING-023` | `T4` | `L1-FR-ERRORS.20` |
| Peer UID mismatch on UDS | `IT-IPC-009` | `T2` | `L1-FR-IPC.7` |
| `lastSeenInstanceId` matches active daemon instance | `IT-IPC-006`, `IT-IPC-012` | `T2` | `L1-FR-IPC.3`, `L1-FR-IPC.4` |
| `lastSeenInstanceId` differs after daemon restart | `IT-IPC-007`, `IT-IPC-012` | `T2` | `L1-FR-IPC.3`, `L1-FR-IPC.4` |
| Archived lifecycle frame missing archive sequence | `IT-IPC-011` | `T2` | `L1-FR-IPC.6` |

## 10. Negative-Path Error Coverage

| Requirement | Error Code | Planned Tests |
|------------|------------|---------------|
| `L1-FR-ERRORS.13` | error manifest row storage | `IT-RSL-010`, `IT-RSL-014`, `IT-CON-009` |
| `L1-FR-ERRORS.14` | `SOURCE_CHANGED` | `E2E-ING-021` |
| `L1-FR-ERRORS.15` | `NOTICE_PARSE_FAILED` | `E2E-ING-021` |
| `L1-FR-ERRORS.16` | `GOODS_CLASSIFICATION_LOW_CONFIDENCE` | `E2E-ING-022` |
| `L1-FR-ERRORS.17` | `SUPPLIER_MATCH_LOW_CONFIDENCE` | `E2E-ING-022` |
| `L1-FR-ERRORS.18` | `COMMODITY_CODE_LOW_CONFIDENCE` | `E2E-ING-022` |
| `L1-FR-ERRORS.19` | `REVENUE_BASIS_UNCERTAIN` | `E2E-ING-023` |
| `L1-FR-ERRORS.20` | `FPO_ACCESS_UNAVAILABLE` | `E2E-ING-023` |
| `L1-FR-ERRORS.21` | `AGENT_STALLED` | `IT-CON-011`, `IT-RPR-002` |
| `L1-FR-ERRORS.22` | `AGENT_AUTH_CHALLENGE` | `E2E-CAN-003`, `IT-RPR-011` |
| `L1-FR-ERRORS.23` | `AGENT_SKILL_FAILED` | `E2E-CAN-009` |
| `L1-FR-ERRORS.24` | `AGENT_OUTPUT_CONTRACT_INVALID` | `IT-CON-009`, `UT-RSL-001` |
| `L1-FR-ERRORS.25` | `AGENT_SUSPECT_OUTPUT` | `IT-CON-010` |
| `L1-FR-ERRORS.26` | `AGENT_PARTIAL_FAILURE` | `IT-CON-011`, `UT-RSL-006` |
| `L1-FR-ERRORS.27` | `AGENT_INTERNAL_ERROR` | `UT-CDX-003`, `UT-CLD-003` |
| `L1-FR-ERRORS.28` | `AGENT_RUN_FAILED` | `E2E-CON-014`, `IT-PTY-019` |
| `L1-FR-ERRORS.29` | `AGENT_CLEANUP_INCOMPLETE` | `IT-RPR-005` |
| `L1-FR-ERRORS.30` | `AGENT_DAEMON_CRASH_DURING_SESSION` | `IT-RPR-008`, `IT-IPC-007` |
| `L1-FR-ERRORS.31` | `AGENT_PROTOCOL_MISMATCH` | `E2E-CAN-004`, `UT-CDX-002`, `UT-CDX-010`, `UT-CLD-002`, `UT-CLD-010` |
| `L1-FR-ERRORS.32` | `SOURCE_SOFT_BLOCKED` | `E2E-CAN-007` |
| `L1-FR-ERRORS.33` | `SOURCE_CAPTCHA_BLOCKED` | `E2E-CAN-008` |
| `L1-FR-ERRORS.34` | `AUTH_STATE_STALE` | `E2E-CAN-003` |
| `L1-FR-ERRORS.35` | `PAGE_SHAPE_DRIFT` | `E2E-ING-005`, `E2E-CAN-005` |
| `L1-FR-ERRORS.36` | `LOCALE_OR_CURRENCY_DRIFT` | `E2E-CAN-006` |
| `L1-FR-ERRORS.37` | `HOST_SLEEP_INTERRUPTED` | `IT-RPR-011` |
| `L1-FR-ERRORS.38` | `HOST_RESOURCE_EXHAUSTED` | `IT-RPR-011` |
| `L1-FR-ERRORS.39` | error-manifest runtime/resolution context | `IT-RSL-010`, `IT-RSL-014`, `IT-CON-009`, `E2E-CAN-009`, `E2E-ING-021` |
| `L1-FR-ERRORS.40` | `SKILL_DIRECTORY_INVALID` | `UT-SKL-001`, `UT-SKL-002` |
| `L1-FR-ERRORS.41` | `SQLITE_WRITE_FAILED` | `IT-CON-012`, `IT-CON-018`, `E2E-ING-008` |
| `L1-FR-ERRORS.42` | `DAEMON_STARTUP_FAILED` | `IT-IPC-001`, `IT-IPC-010`, `IT-IPC-013` |

## 11. Coverage Matrix

### 11.1 DAEMON / PTY / REAPER

| Requirement | Test ID(s) | Type | Status |
|------------|------------|------|--------|
| `L1-FR-DAEMON.1` | `E2E-ING-020`, `E2E-RPR-012` | E2E + Manual Gate | Covered |
| `L1-FR-DAEMON.2` | `CI-CORE-001`, `CI-CORE-002`, `CI-CORE-003`, `CI-CORE-004` | Static | Covered |
| `L1-FR-DAEMON.3` | `CI-CORE-001`, `CI-CORE-002`, `CI-CORE-005` | Static | Covered |
| `L1-FR-DAEMON.4` | `IT-PTY-012`, `IT-PTY-013` | IT | Covered |
| `L1-FR-DAEMON.5` | `UT-PTY-003`, `IT-PTY-012`, `E2E-ING-004` | UT + IT + E2E | Covered |
| `L1-FR-DAEMON.6` | `IT-PTY-014`, `IT-RPR-011` | IT | Covered |
| `L1-FR-DAEMON.7` | `IT-PTY-015`, `IT-RPR-002` | IT | Covered |
| `L1-FR-DAEMON.8` | `IT-PTY-016`, `IT-RPR-009` | IT | Covered |
| `L1-FR-DAEMON.9` | `IT-RPR-007`, `IT-RPR-013` | IT | Covered |
| `L1-FR-DAEMON.10` | `IT-IPC-011`, `IT-PTY-012` | IT | Covered |
| `L1-FR-DAEMON.11` | `IT-PTY-012` | IT | Covered |
| `L1-FR-DAEMON.12` | `IT-PTY-013`, `IT-PTY-014`, `IT-PTY-012` | IT | Covered |
| `L1-FR-PTY.1` | `UT-PTY-001`, `UT-PTY-002`, `IT-PTY-019`, `IT-PTY-022` | UT + IT | Covered |
| `L1-FR-PTY.2` | `UT-PTY-004`, `IT-RPR-006` | UT + IT | Covered |
| `L1-FR-PTY.3` | `UT-PTY-005` | UT | Covered |
| `L1-FR-PTY.4` | `UT-PTY-006`, `UT-PTY-007`, `E2E-PTY-018` | UT + E2E | Covered |
| `L1-FR-REAPER.1` | `IT-RPR-003`, `IT-RPR-009`, `E2E-RPR-012` | IT + E2E | Covered |
| `L1-FR-REAPER.2` | `IT-RPR-002`, `IT-PTY-016` | IT | Covered |
| `L1-FR-REAPER.3` | `IT-RPR-001`, `IT-RPR-006`, `IT-RPR-007`, `IT-RPR-013` | IT | Covered |
| `L1-FR-REAPER.4` | `IT-RPR-003`, `IT-RPR-004`, `IT-RPR-005` | IT | Covered |
| `L1-FR-REAPER.5` | `IT-RPR-006`, `IT-RPR-010` | IT | Covered |

### 11.2 ADAPTER

| Requirement | Test ID(s) | Type | Status |
|------------|------------|------|--------|
| `L1-FR-ADAPTER.1` | `UT-CON-004`, `UT-CDX-004`, `UT-CLD-004`, `E2E-ING-003` | UT + E2E | Covered |
| `L1-FR-ADAPTER.2` | `IT-PTY-022`, `E2E-ING-003`, `E2E-CAN-003` | IT + E2E | Covered |
| `L1-FR-ADAPTER.3` | `UT-CON-001`, `E2E-CON-013`, `IT-CON-016` | UT + IT + E2E | Covered |
| `L1-FR-ADAPTER.4` | `UT-CDX-001`, `UT-CLD-001`, `E2E-CAN-004` | UT + E2E | Covered |
| `L1-FR-ADAPTER.5` | `UT-CON-005`, `UT-CDX-006`, `UT-CLD-006` | UT | Covered |
| `L1-FR-ADAPTER.6` | `IT-PTY-008`, `UT-CON-001`, `IT-CON-016`, `UT-CLD-008` | UT + IT | Covered |
| `L1-FR-ADAPTER.7` | `UT-CON-002`, `UT-CDX-008`, `UT-CLD-008`, `E2E-ING-009` | UT + E2E | Covered |
| `L1-FR-ADAPTER.8` | `UT-CON-003`, `UT-RSL-001`, `E2E-ING-009` | UT + E2E | Covered |
| `L1-FR-ADAPTER.9` | `UT-CDX-002`, `UT-CLD-002`, `E2E-CAN-004` | UT + E2E | Covered |
| `L1-FR-ADAPTER.10` | `UT-CDX-007`, `UT-CLD-007` | UT | Covered |
| `L1-FR-ADAPTER.11` | `UT-CDX-001`, `UT-CLD-001`, `IT-CDX-010`, `IT-CLD-010` | UT + IT | Covered |
| `L1-FR-ADAPTER.12` | `UT-CDX-004`, `UT-CDX-005`, `UT-CDX-006`, `UT-CLD-004`, `UT-CLD-005`, `UT-CLD-006` | UT | Covered |
| `L1-FR-ADAPTER.13` | `UT-CDX-007`, `UT-CLD-007` | UT | Covered |
| `L1-FR-ADAPTER.14` | `UT-CON-002`, `UT-CDX-008`, `UT-CDX-010`, `UT-CLD-008`, `UT-CLD-010`, `IT-CON-016` | UT + IT | Covered |
| `L1-FR-ADAPTER.15` | `UT-CDX-001`, `UT-CDX-006`, `UT-CLD-001`, `UT-CLD-006` | UT | Covered |
| `L1-FR-ADAPTER.16` | `IT-CDX-010`, `IT-CLD-010` | IT | Covered |
| `L1-FR-ADAPTER.17` | `IT-CDX-010`, `IT-CLD-010` | IT | Covered |
| `L1-FR-ADAPTER.18` | `UT-CDX-009`, `UT-CDX-010`, `UT-CLD-009`, `UT-CLD-010`, `IT-RPR-002` | UT + IT | Covered |

### 11.3 ARTIFACTS / PROMPT

| Requirement | Test ID(s) | Type | Status |
|------------|------------|------|--------|
| `L1-FR-ARTIFACTS.1` | `E2E-PTY-017` | E2E | Covered |
| `L1-FR-ARTIFACTS.2` | `IT-PTY-009`, `IT-PTY-010` | IT | Covered |
| `L1-FR-ARTIFACTS.3` | `IT-PTY-009`, `IT-PTY-011`, `IT-PTY-020`, `IT-RSL-012` | IT | Covered |
| `L1-FR-ARTIFACTS.4` | `IT-PTY-009`, `IT-PTY-010`, `IT-PTY-011`, `IT-PTY-020`, `IT-PTY-021` | IT | Covered |
| `L1-FR-ARTIFACTS.5` | `E2E-PTY-017`, `E2E-ING-010` | E2E | Covered |
| `L1-FR-ARTIFACTS.6` | `IT-RSL-009`, `E2E-ING-010` | IT + E2E | Covered |
| `L1-FR-ARTIFACTS.7` | `UT-RSL-004` | UT | Covered |
| `L1-FR-ARTIFACTS.8` | `IT-RSL-008` | IT | Covered |
| `L1-FR-ARTIFACTS.9` | `E2E-CON-013`, `UT-RSL-007` | E2E + UT | Covered |
| `L1-FR-ARTIFACTS.10` | `UT-RSL-002` | UT | Covered |
| `L1-FR-ARTIFACTS.11` | `UT-RSL-003` | UT | Covered |
| `L1-FR-ARTIFACTS.12` | `IT-RSL-011` | IT | Covered |
| `L1-FR-ARTIFACTS.13` | `UT-RSL-004` | UT | Covered |
| `L1-FR-ARTIFACTS.14` | `IT-RSL-008` | IT | Covered |
| `L1-FR-ARTIFACTS.15` | `UT-RSL-005` | UT | Covered |
| `L1-FR-PROMPT.1` | `UT-SKL-008`, `IT-RSL-008` | UT + IT | Covered |

### 11.4 DB / IPC

| Requirement | Test ID(s) | Type | Status |
|------------|------------|------|--------|
| `L1-FR-DB.1` | `IT-IPC-013`, `IT-IPC-001`, `E2E-ING-013` | IT + E2E | Covered |
| `L1-FR-DB.2` | `IT-CON-017`, `IT-CON-018`, `IT-IPC-003`, `IT-IPC-013`, `E2E-ING-013`, `IT-CON-012` | IT + E2E | Covered |
| `L1-FR-DB.3` | `IT-CON-018`, `IT-IPC-010`, `IT-IPC-013` | IT | Covered |
| `L1-FR-DB.4` | `IT-IPC-001`, `IT-IPC-005`, `IT-IPC-010`, `IT-IPC-013` | IT | Covered |
| `L1-FR-DB.5` | `E2E-PTY-017`, `IT-CON-006`, `IT-RSL-009` | E2E + IT | Covered |
| `L1-FR-DB.6` | `E2E-ING-012` | E2E | Covered |
| `L1-FR-DB.7` | `IT-IPC-001`, `IT-IPC-002` | IT | Covered |
| `L1-FR-DB.8` | `E2E-ING-006`, `E2E-ING-007` | E2E | Covered |
| `L1-FR-DB.9` | `E2E-ING-012` | E2E | Covered |
| `L1-FR-DB.10` | `E2E-ING-001`, `E2E-CAN-001` | E2E | Covered |
| `L1-FR-DB.11` | `E2E-ING-019` | E2E | Covered |
| `L1-FR-IPC.1` | `IT-IPC-001`, `IT-IPC-010` | IT | Covered |
| `L1-FR-IPC.2` | `IT-IPC-002` | IT | Covered |
| `L1-FR-IPC.3` | `IT-IPC-003`, `IT-IPC-004`, `IT-IPC-005`, `IT-IPC-012` | IT | Covered |
| `L1-FR-IPC.4` | `IT-IPC-006`, `IT-IPC-007`, `IT-IPC-012`, `IT-RPR-008` | IT + T3 | Covered |
| `L1-FR-IPC.5` | `IT-IPC-008`, `E2E-CAN-003` | IT + E2E | Covered |
| `L1-FR-IPC.6` | `IT-IPC-011` | IT | Covered |
| `L1-FR-IPC.7` | `IT-IPC-009` | IT | Covered |

### 11.5 RUNNER / PIPELINE

| Requirement | Test ID(s) | Type | Status |
|------------|------------|------|--------|
| `L1-FR-RUNNER.1` | `IT-CON-012`, `IT-CON-018`, `E2E-ING-013` | IT + E2E | Covered |
| `L1-FR-RUNNER.2` | `IT-CON-012`, `IT-CON-017`, `IT-CON-018` | IT | Covered |
| `L1-FR-RUNNER.3` | `IT-CON-006`, `IT-CON-008`, `E2E-CON-013` | IT + E2E | Covered |
| `L1-FR-RUNNER.4` | `E2E-ING-013` | E2E | Covered |
| `L1-FR-RUNNER.5` | `E2E-ING-016` | E2E | Covered |
| `L1-FR-RUNNER.6` | `IT-CON-015`, `UT-RSL-006` | IT + UT | Covered |
| `L1-FR-RUNNER.7` | `E2E-ING-012`, `E2E-ING-019` | E2E | Covered |
| `L1-FR-RUNNER.8` | `E2E-ING-017` | E2E | Covered |
| `L1-FR-RUNNER.9` | `E2E-ING-015` | E2E | Covered |
| `L1-FR-PIPELINE.1` | `E2E-ING-010` | E2E | Covered |
| `L1-FR-PIPELINE.2` | `E2E-ING-001` | E2E | Covered |
| `L1-FR-PIPELINE.3` | `E2E-ING-001`, `E2E-ING-002` | E2E | Covered |
| `L1-FR-PIPELINE.4` | `E2E-ING-003` | E2E | Covered |
| `L1-FR-PIPELINE.5` | `E2E-ING-004`, `IT-PTY-019` | E2E + IT | Covered |
| `L1-FR-PIPELINE.6` | `E2E-ING-005` | E2E | Covered |
| `L1-FR-PIPELINE.7` | `E2E-ING-006`, `E2E-ING-007`, `IT-CON-015` | E2E + IT | Covered |
| `L1-FR-PIPELINE.8` | `IT-PTY-008`, `E2E-ING-008` | IT + E2E | Covered |
| `L1-FR-PIPELINE.9` | `IT-CON-006`, `IT-CON-007`, `E2E-ING-009` | IT + E2E | Covered |
| `L1-FR-PIPELINE.10` | `E2E-ING-010`, `IT-RSL-008` | E2E + IT | Covered |
| `L1-FR-PIPELINE.11` | `IT-CON-008`, `IT-CON-009`, `IT-CON-010`, `UT-RSL-001` | UT + IT | Covered |
| `L1-FR-PIPELINE.12` | `E2E-ING-012` | E2E | Covered |
| `L1-FR-PIPELINE.13` | `E2E-ING-013` | E2E | Covered |
| `L1-FR-PIPELINE.14` | `IT-PTY-012`, `E2E-ING-020`, `E2E-RPR-012` | IT + E2E + Manual Gate | Covered |
| `L1-FR-PIPELINE.15` | `E2E-ING-010`, `E2E-ING-025` | E2E | Covered |
| `L1-FR-PIPELINE.16` | `E2E-ING-016`, `E2E-ING-026` | E2E | Covered |
| `L1-FR-PIPELINE.17` | `E2E-ING-014` | E2E | Covered |
| `L1-FR-PIPELINE.18` | `E2E-ING-014` | E2E | Covered |
| `L1-FR-PIPELINE.19` | `E2E-ING-015`, `E2E-ING-016`, `E2E-ING-026` | E2E | Covered |
| `L1-FR-PIPELINE.20` | `E2E-ING-018`, `E2E-ING-026` | E2E | Covered |
| `L1-FR-PIPELINE.21` | `E2E-ING-017`, `E2E-ING-018` | E2E | Covered |
| `L1-FR-PIPELINE.22` | `E2E-ING-024` | E2E | Covered |

### 11.6 SKILLS / ERRORS

| Requirement | Test ID(s) | Type | Status |
|------------|------------|------|--------|
| `L1-FR-SKILLS.1` | `UT-SKL-001`, `UT-SKL-003` | UT | Covered |
| `L1-FR-SKILLS.2` | `E2E-ING-003`, `E2E-ING-006`, `E2E-CAN-001`, `E2E-CAN-002` | E2E | Covered |
| `L1-FR-SKILLS.3` | `E2E-CON-013`, `E2E-ING-025` | E2E | Covered |
| `L1-FR-SKILLS.4` | `UT-SKL-001`-`UT-SKL-008` | UT | Covered |
| `L1-FR-ERRORS.13` | `IT-RSL-010`, `IT-RSL-014` | IT | Covered |
| `L1-FR-ERRORS.14` | `E2E-ING-021` | E2E | Covered |
| `L1-FR-ERRORS.15` | `E2E-ING-021` | E2E | Covered |
| `L1-FR-ERRORS.16` | `E2E-ING-022` | E2E | Covered |
| `L1-FR-ERRORS.17` | `E2E-ING-022` | E2E | Covered |
| `L1-FR-ERRORS.18` | `E2E-ING-022` | E2E | Covered |
| `L1-FR-ERRORS.19` | `E2E-ING-023` | E2E | Covered |
| `L1-FR-ERRORS.20` | `E2E-ING-023` | E2E | Covered |
| `L1-FR-ERRORS.21` | `IT-CON-011` | IT | Covered |
| `L1-FR-ERRORS.22` | `E2E-CAN-003`, `IT-RPR-011` | E2E + IT | Covered |
| `L1-FR-ERRORS.23` | `E2E-CAN-009` | E2E | Covered |
| `L1-FR-ERRORS.24` | `IT-CON-009` | IT | Covered |
| `L1-FR-ERRORS.25` | `IT-CON-010` | IT | Covered |
| `L1-FR-ERRORS.26` | `IT-CON-011`, `UT-RSL-006` | IT + UT | Covered |
| `L1-FR-ERRORS.27` | `UT-CDX-003`, `UT-CLD-003` | UT | Covered |
| `L1-FR-ERRORS.28` | `E2E-CON-014`, `IT-PTY-019` | E2E + IT | Covered |
| `L1-FR-ERRORS.29` | `IT-RPR-005` | IT | Covered |
| `L1-FR-ERRORS.30` | `IT-RPR-008`, `IT-IPC-007` | IT + T3 | Covered |
| `L1-FR-ERRORS.31` | `UT-CDX-002`, `UT-CDX-010`, `UT-CLD-002`, `UT-CLD-010`, `E2E-CAN-004` | UT + E2E | Covered |
| `L1-FR-ERRORS.32` | `E2E-CAN-007` | E2E | Covered |
| `L1-FR-ERRORS.33` | `E2E-CAN-008` | E2E | Covered |
| `L1-FR-ERRORS.34` | `E2E-CAN-003` | E2E | Covered |
| `L1-FR-ERRORS.35` | `E2E-ING-005`, `E2E-CAN-005` | E2E | Covered |
| `L1-FR-ERRORS.36` | `E2E-CAN-006` | E2E | Covered |
| `L1-FR-ERRORS.37` | `IT-RPR-011` | IT | Covered |
| `L1-FR-ERRORS.38` | `IT-RPR-011` | IT | Covered |
| `L1-FR-ERRORS.39` | `IT-RSL-010`, `IT-RSL-014`, `IT-CON-009`, `E2E-CAN-009`, `E2E-ING-021` | IT + E2E | Covered |
| `L1-FR-ERRORS.40` | `UT-SKL-002` | UT | Covered |
| `L1-FR-ERRORS.41` | `IT-CON-012`, `IT-CON-018`, `E2E-ING-008` | IT + E2E | Covered |
| `L1-FR-ERRORS.42` | `IT-IPC-001`, `IT-IPC-010`, `IT-IPC-013` | IT | Covered |

## 12. Anti-Patterns Blocked By This TDD

- No fake DB adapter in place of `rusqlite`.
- No fake process supervisor in place of `setsid` / `killpg` / `waitpid`.
- No success assertions that only inspect transcripts while ignoring contracts, DB rows, or artifacts.
- No run-manifest assertions that stop at file existence; every manifest test checks frontmatter, sections, appendix, and disagreement handling.
- No stage-tracking assertions that check only row counts; they must check head advancement, `stage_run_id`, and `commit_seq`.
- No IPC test that handwaves restart behavior; same-instance resume and cross-instance invalidation are separate tests.
- No live-source release gate that relies only on captured evidence; live and captured lanes are distinct.

## 13. Downstream Consumer Notes

- Strategy generation must treat `T1/T2` as the cumulative per-commit gate, `T3` as deterministic nightly stability coverage, `T4` as scheduled live-network runner coverage, and `Manual Gate` as the target-host Experiment 4 confirmation.
- Beads should reference concrete test IDs from this file, not only requirement IDs.
- Completion audit should mechanically diff the `138` requirement IDs from the spec against the coverage matrix in §11 and fail if any row is missing.
