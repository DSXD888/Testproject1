# L1 Daemon Core — Create TDD

## First Action

Invoke the `/spec-to-tdd` skill.

---

## Your Mission

Create a comprehensive **Test-Driven Development specification** and **bootstrap document** for Layer 1 — the Rust daemon core. The TDD must map every requirement in the validated spec to concrete test IDs, define the testing pyramid (unit / integration / E2E), specify test infrastructure, and produce a bootstrap document with actual test file scaffolding.

This is a **Rust-first** TDD. The daemon is written in Rust 2024 (nightly) with Tokio async runtime, `rusqlite` for SQLite, `nix` for POSIX process management, and `portable-pty` for PTY sessions. Tests use `cargo test` with `#[test]` and `#[tokio::test]`.

---

## Source Documents (Read In This Order)

### Primary Input (CREATE FROM THIS)

```
docs/strategy/layers/layer-1-thin-local-runner.md   — Validated L1 spec (SOT #1) — READ FULLY
```

### Source of Truth Context

```
docs/strategy/layer-strategy.md                      — §11 Testing Evolution targets for L1 — SKIM
docs/strategy/layers/layer-0-gold-set-evaluation.md  — §5 Evaluation Harness (test infrastructure pattern) — SKIM
docs/strategy/layers/layer-0-gold-set-evaluation.md  — §2 Full DDL (table schemas for assertions) — REFERENCE
```

### Codebase Context (STUDY FOR PATTERNS)

```
AGENTS.md                                            — Toolchain (Cargo, Bun, Rust 2024), compiler checks, coding discipline
docs/strategy/cross-reference-index.md               — Topic → file → section lookup
```

### Quality Bar (MATCH OR EXCEED)

```
docs/strategy/layers/layer-0-gold-set-evaluation.md  — L0 has 4 test suites defined in §6.2 — match this specificity
```

---

## Key Context From Codebase Research

- **Greenfield codebase** — no existing code, no existing test infrastructure. The TDD bootstraps everything.
- **17 daemon source files** — each maps to specific subsystem requirements (PTY, ADAPTER, RUNNER, etc.)
- **10 test files specified** in the L1 spec §5.2 with coverage descriptions
- **30+ edge case scenarios** in §5.3 — each needs a test ID and tier assignment
- **12 error types** in §1.9 / §4.3 — each needs negative-path test coverage
- **7 session lifecycle phases** — state machine needs transition coverage
- **Rust testing conventions:** `#[test]` for sync, `#[tokio::test]` for async, test modules in separate files under `tests/`, integration tests need real SQLite

---

## Step-by-Step Workflow

### Step 0: Read Source Documents (MANDATORY — before doing anything)

1. `docs/strategy/layers/layer-1-thin-local-runner.md` — **READ FULLY** (~1,590+ lines with requirement IDs from Step 01)
2. `docs/strategy/layer-strategy.md` §11 — **SKIM** (testing evolution: what L1 must add)
3. `docs/strategy/layers/layer-0-gold-set-evaluation.md` §5, §6 — **SKIM** (L0 test harness for pattern reference)
4. `AGENTS.md` — **SKIM** (Rust toolchain conventions, compiler check requirements)

### Step 1: Invoke the Skill

Run `/spec-to-tdd`.

**Inputs:**
- Source: `docs/strategy/layers/layer-1-thin-local-runner.md`
- Output spec: `docs/pipeline/l1-daemon-core/artifacts/l1-daemon-core-tdd.md`
- Output bootstrap: `docs/pipeline/l1-daemon-core/artifacts/l1-daemon-core-bootstrap.md`

**Key arguments/context to provide the skill:**
- Language: Rust 2024 (nightly), async via Tokio
- Test framework: `cargo test` with `#[test]` and `#[tokio::test]`
- Unsafe policy: `#![forbid(unsafe_code)]`
- DB: SQLite via `rusqlite` (bundled), WAL mode
- Process management: `nix` crate for POSIX (setpgid, killpg, waitpid)
- PTY: `portable-pty` or `pty-process` crate
- No mocks for database — use real SQLite in-memory or temp files
- No mocks for process supervision — use real fork/exec with test helper binaries
- The spec already lists 10 test file names in §5.2 — use these as the file structure

### Step 2: Feature-Specific Guidance

While the skill runs, pay attention to:

- **Process supervision tests need real processes.** The spec explicitly requires "real E2E agent tests, no mocks/stubs." For PTY and reaper tests, use small test helper binaries (e.g., a script that sleeps, a script that forks grandchildren, a script that ignores SIGTERM) rather than mocking `nix::sys::signal`.

- **SQLite tests need real databases.** Use `rusqlite::Connection::open_in_memory()` for unit tests, temp-file databases for integration tests. Never mock the database layer.

- **The two-lane adapter model needs both lanes tested independently.** `OutputFrame` capture tests should verify raw bytes are captured correctly. `ControlEvent` parsing tests should verify semantic events are extracted correctly. And integration tests should verify both lanes work together.

- **The 7-phase session lifecycle is a state machine.** Tests should cover every valid transition AND every invalid transition (e.g., `created` → `active` skipping `spawned` should error).

- **Chunked capture tests need crash-safety verification.** Test that a kill during capture leaves the stream in a recoverable state (no partial chunks, index consistent with chunk files).

- **Error manifest tests should verify Markdown output.** The error manifests are human-readable Markdown — test that they render correctly with all context fields populated.

### Step 3: Verify Output

After the skill completes, verify:

- [ ] TDD output file exists at `docs/pipeline/l1-daemon-core/artifacts/l1-daemon-core-tdd.md`
- [ ] Bootstrap output file exists at `docs/pipeline/l1-daemon-core/artifacts/l1-daemon-core-bootstrap.md`
- [ ] Every `L1-FR-*` requirement ID from the spec has at least one test ID mapped to it
- [ ] Test pyramid is defined: unit tests (fast, isolated) vs integration tests (real SQLite, real processes) vs E2E tests (full daemon + real skills)
- [ ] All 10 test files from §5.2 are present in the TDD with full test case listings
- [ ] All 30+ edge cases from §5.3 have assigned test IDs
- [ ] All 12 error types have negative-path test coverage
- [ ] Bootstrap document includes actual Rust test scaffolding (`#[test]`, `#[tokio::test]`)
- [ ] CI tier assignments: which tests run on every commit vs nightly vs manual

### Step 4: Follow-Up Pass (Sub-Optimization)

> Send this as a follow-up message in the same session after the main work is complete.

ULTRATHINK and verify with fresh eyes:

1. Open the TDD and count test IDs. For a 17-file daemon with 12 error types, 7 session phases, and 15 pipeline stages, expect **100-150 test IDs**. If significantly less, test coverage has gaps.

2. Open `docs/strategy/layers/layer-1-thin-local-runner.md` §2.3 — the `AgentAdapter` trait has `probe()`, `resolve_mode()`, `build_launch_spec()`, `parser()`. Does the TDD test each method for BOTH the Codex and Claude adapters? That's 8 test groups minimum.

3. Open §2.5 Artifact Store — chunked capture has specific rules (32KB chunks, zstd compression, SHA-256 hashing, atomic rename). Does the TDD test chunk boundary behavior? What happens at exactly 32KB? At 0 bytes?

4. Open §2.6 Job Runner — `stage_run_id` scoping has set-replacement vs additive semantics. Does the TDD test the boundary between them? (e.g., a stage that produces partial results — what happens to the `stage_run_id`?)

5. Open §2.8 IPC — the handshake has `lastSeenInstanceId` for reconnect detection. Does the TDD test daemon restart detection? Does it test the "daemon crashed while PTY session was active" recovery path?

6. Verify the bootstrap document includes a `Cargo.toml` for the test crate with all necessary dev-dependencies (`tokio-test`, `tempfile`, `assert_cmd`, etc.)

---

## Key Reminders

- This is Rust — test infrastructure means real `#[test]` functions, not Jest/Vitest
- No mocking the database — use real SQLite (in-memory for unit, temp-file for integration)
- No mocking process supervision — use real fork/exec with test helper binaries
- The spec's Definition of Done (§5.1) requires `cargo check --all-targets` and `cargo clippy -- -D warnings` — the bootstrap must set this up
- Test file names are specified in §5.2 — use these exact names
- The L1 spec says "Release gates use real systems or captured evidence bundles, not mock-only substitutes" — enforce this in the TDD
- `#![forbid(unsafe_code)]` is a project-wide invariant — no `unsafe` in test code either
