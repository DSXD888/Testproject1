# L1 Daemon Core — Test Bootstrap

## 1. Scope

This document defines how the Layer 1 daemon tests are bootstrapped and run. It is the executable companion to:

- `docs/pipeline/l1-daemon-core/artifacts/l1-daemon-core-tdd.md`
- `docs/strategy/layers/layer-1-thin-local-runner.md`

The repository is greenfield for Rust daemon code. The goal here is to provide drop-in scaffolding for the daemon test harness, not to create mock-heavy non-executable tests.

## 2. Success Criteria Coverage Summary

| DoD Item | Bootstrap Surface | Primary Tests / Commands |
|---------|-------------------|--------------------------|
| 1 | compiler/lint/format gates | `CI-CORE-001`..`CI-CORE-003`; `cargo check --all-targets`; `cargo clippy --all-targets -- -D warnings`; `cargo fmt --check` |
| 2 | PTY + contract skill launch | `agent_pty_session.rs`, `agent_contract_session.rs`, `adapter_codex.rs`, `adapter_claude.rs` |
| 3 | manual FTS ingestion trigger | `agent_procurement_ingestion.rs` live/captured-evidence lanes |
| 4 | end-to-end local loop | `agent_procurement_ingestion.rs` + manifest generation path |
| 5 | verifiable output artifact | `agent_procurement_ingestion.rs`, `adapter_result_contract.rs`, `ipc_handshake.rs` |
| 6 | process-group supervision | `agent_pty_session.rs`, `session_reaper.rs` |
| 7 | artifact capture | `agent_pty_session.rs` |
| 8 | run manifests | `adapter_result_contract.rs`, `agent_contract_session.rs`, `agent_procurement_ingestion.rs` |
| 9 | error manifests | `adapter_result_contract.rs`, `session_reaper.rs`, `ipc_handshake.rs` |
| 10 | skill linting CI gate | `skill_lint.rs` |
| 11 | smoke canary harness | `skill_canary_harness.rs` |
| 12 | Experiment 4 stress | `agent_procurement_ingestion.rs`, `session_reaper.rs` |
| 13 | `stage_run_id` scoping | `agent_procurement_ingestion.rs` |

## 3. Environment Prerequisites

| Requirement | Why it is needed | Validation |
|------------|------------------|------------|
| Rust nightly with 2024 edition support | daemon crate and tests target nightly Rust 2024 | `rustup toolchain list | rg nightly` |
| Cargo | build/test/lint entrypoint | `cargo --version` |
| Bun | Layer 0/1 skills still execute through Bun-backed skill directories | `bun --version` |
| `rg` | fast fixture discovery / debugging | `rg --version` |
| local filesystem write access | artifact store, temp SQLite DBs, UDS sockets, lock files | write to temp dir |
| Unix-like OS (macOS/Linux) | PTY, process groups, `waitpid`, pathname UDS peer credentials | `uname -s` in `{Darwin,Linux}` |
| optional `codex` CLI | live Codex adapter smoke lane | `command -v codex` |
| optional `claude` CLI | live Claude adapter smoke lane | `command -v claude` |
| optional internet access | live FTS canary / live loop gates | DNS + HTTPS probe |

## 4. Validation Script

Use the following bootstrap validation script before writing or running daemon tests:

```bash
#!/usr/bin/env bash
set -euo pipefail

pass() { printf '[PASS] %s\n' "$1"; }
warn() { printf '[WARN] %s\n' "$1"; }
fail() { printf '[FAIL] %s\n' "$1"; exit 1; }

rustup toolchain list | rg -q 'nightly' || fail 'Rust nightly toolchain missing'
cargo --version >/dev/null 2>&1 || fail 'cargo missing'
bun --version >/dev/null 2>&1 || fail 'bun missing'
rg --version >/dev/null 2>&1 || fail 'ripgrep missing'

case "$(uname -s)" in
  Darwin|Linux) pass 'Unix-like host detected' ;;
  *) fail 'Layer 1 PTY/process tests are Unix-only in this bootstrap' ;;
esac

test -f docs/strategy/layers/layer-1-thin-local-runner.md || fail 'L1 spec missing'
test -f docs/strategy/layers/layer-0-gold-set-evaluation.md || fail 'L0 source doc missing'
test -f docs/pipeline/l1-daemon-core/artifacts/l1-daemon-core-tdd.md || fail 'TDD artifact missing'

if command -v codex >/dev/null 2>&1; then
  pass 'codex CLI present'
else
  warn 'codex CLI absent: Codex live-smoke T4 lane will be skipped'
fi

if command -v claude >/dev/null 2>&1; then
  pass 'claude CLI present'
else
  warn 'claude CLI absent: Claude live-smoke T4 lane will be skipped'
fi

tmp_root="$(mktemp -d)"
trap 'rm -rf "$tmp_root"' EXIT
touch "$tmp_root/probe" || fail 'temp filesystem is not writable'
pass 'temp filesystem writable'

if curl -IsSf https://www.find-tender.service.gov.uk >/dev/null 2>&1; then
  pass 'live FTS endpoint reachable'
else
  warn 'live FTS endpoint unreachable: use captured evidence bundles for T3'
fi
```

## 5. `Cargo.toml` Test Additions

The repo currently has no Rust workspace, so the bootstrap must define the root workspace files as well as the daemon crate manifest. Layer 1 should keep tests inside `crates/daemon/tests/` rather than introducing a second standalone workspace member unless that becomes absolutely necessary later.

### 5.1 Workspace Root

Create a root `Cargo.toml`:

```toml
[workspace]
members = ["crates/daemon"]
resolver = "2"

[workspace.package]
edition = "2024"

[workspace.lints.clippy]
pedantic = "deny"
nursery = "deny"
```

Create `rust-toolchain.toml`:

```toml
[toolchain]
channel = "nightly"
components = ["rustfmt", "clippy"]
profile = "minimal"
```

### 5.2 `crates/daemon/Cargo.toml`

Add or merge the following into `crates/daemon/Cargo.toml`:

```toml
[package]
name = "daemon"
version = "0.1.0"
edition = "2024"

[features]
default = []
test-support = []

[dependencies]
anyhow = "1"
async-trait = "0.1"
bytes = "1"
chrono = { version = "0.4", default-features = false, features = ["clock", "std"] }
clap = { version = "4", features = ["derive"] }
fs4 = "0.8"
nix = { version = "0.29", features = ["signal", "process", "term"] }
portable-pty = "0.8"
rusqlite = { version = "0.32", features = ["bundled"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
serde_yaml = "0.9"
sha2 = "0.10"
signal-hook = "0.3"
tokio = { version = "1", features = ["rt-multi-thread", "macros", "process", "signal", "sync", "time", "test-util"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["fmt", "env-filter"] }
uuid = { version = "1", features = ["serde", "v4"] }
zstd = "0.13"

[dev-dependencies]
assert_cmd = "2"
assert_fs = "1"
predicates = "3"
serial_test = "3"
tempfile = "3"
tokio-test = "0.4"
wait-timeout = "0.2"

[[bin]]
name = "test_agent"
path = "src/bin/test_agent.rs"
required-features = ["test-support"]
```

Rationale:
- The helper binary lives behind the `test-support` feature, but it still needs normal crate dependencies such as `clap`, `signal-hook`, `nix`, and `rusqlite`.
- `serial_test` is required for lock-file, shared-signal, UDS, and shared CLI-home tests that must not race.
- The spec’s file inventory prescribes integration tests under `crates/daemon/tests/`; that is the Layer 1 “test crate” surface, so a separate `daemon-tests` workspace member would be unnecessary file proliferation.

## 6. Test Workspace Layout

```text
crates/daemon/
  src/
    bin/
      test_agent.rs
  tests/
    support/
      mod.rs
      db.rs
      artifacts.rs
      process.rs
      skills.rs
      assertions.rs
    agent_pty_session.rs
    agent_contract_session.rs
    agent_procurement_ingestion.rs
    adapter_codex.rs
    adapter_claude.rs
    adapter_result_contract.rs
    skill_lint.rs
    skill_canary_harness.rs
    session_reaper.rs
    ipc_handshake.rs
```

## 7. Helper Binary Scaffold

Use one reusable helper binary with explicit subcommands instead of many tiny one-off binaries. The current helper sketch is too weak for the TDD because it does not exercise real signal handling, malformed adapter output, or controlling-terminal proof points.

### 7.1 Required Helper Modes

`crates/daemon/src/bin/test_agent.rs` must expose, at minimum, the following scenarios:

| Subcommand | Purpose | Used By |
|-----------|---------|---------|
| `pty-echo` | Deterministic single-line PTY output | PTY lifecycle smoke / capture ordering |
| `report-session-semantics` | Emit JSON with `pid`, `pgid`, `sid`, `isatty(stdin/stdout/stderr)`, and controlling-tty path | PTY / controlling-terminal tests |
| `sleep` | Quiet child with deterministic runtime | PTY lifecycle / cleanup timing |
| `silent-stall` | No output for timeout window | stall detection / `AGENT_STALLED` |
| `ignore-int-term` | Install real handlers that ignore both `SIGINT` and `SIGTERM` | forced / leaked cleanup verdicts |
| `emit-jsonl-good` | Good JSONL checkpoint/result stream | Codex / contract happy path |
| `emit-jsonl-malformed` | Malformed JSONL frame | Codex parser negative path |
| `emit-jsonl-partial` | Truncated JSONL tail with no newline | Codex `finish()` negative path |
| `emit-stream-json-good` | Good multi-chunk JSON/stream-JSON | Claude happy path |
| `emit-stream-json-malformed` | Malformed stream frame | Claude parser negative path |
| `emit-stream-json-partial` | Truncated stream-json tail | Claude `finish()` negative path |
| `checkpoint-then-crash` | Emit valid checkpoint then terminate abnormally | partial-failure classification |
| `fork-grandchild` | Spawn descendant in same session/pgid | orphan sweep / `killpg` coverage |
| `double-fork-escape` | Attempt browser-helper style escape from main process group | leak detection / escape-case coverage |
| `stdout-stderr` | Deterministic split output | contract raw-lane assertions |
| `emit-auth-challenge` | PTY-visible auth prompt text | `AGENT_AUTH_CHALLENGE` |
| `exit-code` | Explicit exit status | generic failure classification |

### 7.2 Enum Skeleton

`crates/daemon/src/bin/test_agent.rs`

```rust
#![forbid(unsafe_code)]

use clap::{Parser, Subcommand};

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    cmd: Command,
}

#[derive(Subcommand)]
enum Command {
    PtyEcho { message: String },
    ReportSessionSemantics,
    Sleep { seconds: u64 },
    SilentStall { seconds: u64 },
    IgnoreIntTerm { seconds: u64 },
    EmitJsonlGood,
    EmitJsonlMalformed,
    EmitJsonlPartial,
    EmitStreamJsonGood,
    EmitStreamJsonMalformed,
    EmitStreamJsonPartial,
    CheckpointThenCrash,
    ForkGrandchild { seconds: u64 },
    DoubleForkEscape { seconds: u64 },
    StdoutStderr,
    EmitAuthChallenge,
    ExitCode { code: i32 },
}
```

Implementation notes:
- `IgnoreIntTerm` must install real handlers that ignore both `SIGINT` and `SIGTERM`; otherwise the test cannot prove escalation to `SIGKILL` or `leaked`.
- `ReportSessionSemantics` is the authoritative helper for PTY tests that claim true controlling-terminal semantics.
- The malformed / partial output helpers are mandatory. Parser tests are not credible if they only exercise happy-path fixtures.
- `DoubleForkEscape` does not need to defeat the OS perfectly; it only needs to simulate the browser-helper escape case strongly enough for descendant-sweep assertions.

## 8. Shared Support Module Scaffold

`crates/daemon/tests/support/mod.rs`

```rust
#![forbid(unsafe_code)]

pub mod artifacts;
pub mod assertions;
pub mod db;
pub mod process;
pub mod skills;

use std::path::PathBuf;

use tempfile::TempDir;

pub struct TestContext {
    pub temp: TempDir,
    pub state_root: PathBuf,
    pub runtime_root: PathBuf,
    pub artifact_root: PathBuf,
    pub db_path: PathBuf,
}

impl TestContext {
    pub fn new() -> anyhow::Result<Self> {
        let temp = tempfile::tempdir()?;
        let root = temp.path().to_path_buf();
        Ok(Self {
            temp,
            state_root: root.join("state"),
            runtime_root: root.join("runtime"),
            artifact_root: root.join("artifacts"),
            db_path: root.join("state").join("state.db"),
        })
    }
}
```

`crates/daemon/tests/support/db.rs`

```rust
#![forbid(unsafe_code)]

use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::Connection;

pub fn open_state_db(path: &Path, schema_epoch: i64) -> anyhow::Result<Connection> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let conn = Connection::open(path)?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    conn.pragma_update(None, "busy_timeout", 5_000_i64)?;
    conn.pragma_update(None, "cache_size", -64_000_i64)?;
    conn.pragma_update(None, "user_version", schema_epoch)?;
    apply_sql_file(&conn, "migrations/0001_sqlite_state_initial.sql")?;
    apply_sql_file(&conn, "migrations/0003_sqlite_runtime.sql")?;
    conn.execute(
        "INSERT OR IGNORE INTO commit_clock (id, commit_seq) VALUES (1, 0)",
        [],
    )?;
    let quick_check: String = conn.query_row("PRAGMA quick_check", [], |row| row.get(0))?;
    anyhow::ensure!(quick_check == "ok", "quick_check failed: {quick_check}");
    Ok(conn)
}

fn apply_sql_file(conn: &Connection, path: &str) -> anyhow::Result<()> {
    let ddl = fs::read_to_string(repo_path(path))?;
    conn.execute_batch(&ddl)?;
    Ok(())
}

fn repo_path(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..").join(rel)
}
```

`crates/daemon/tests/support/process.rs`

Bootstrap this support surface even if the implementation stays small at first:

- `ProcessIsolationGuard::new(test_id)` allocates unique temp roots and session labels per test.
- `register_pid/register_pgid` records spawned helpers for teardown.
- `assert_no_survivors()` runs after every PTY/reaper/integration test, even on failure paths.
- Tests that touch shared signal handlers, `daemon.lock`, shared CLI homes, or global environment use `#[serial]`.
- Descendant cleanup during teardown must use the same real `killpg` / sweep logic as production code, not ad hoc test-only shortcuts.

Parallelism rule:
- Default `cargo test` parallelism should work for the majority of tests because each test must allocate a unique temp root, unique SQLite path, unique UDS path, and unique artifact root.
- Tests that intentionally exercise shared signal handlers, daemon lock exclusivity, or shared CLI-home/global-env state are not parallel-safe by design and must be marked `#[serial]`.
- No test may depend on a fixed TCP/UDS port, a fixed filesystem path, or a pre-existing PID/PGID.

`crates/daemon/tests/support/assertions.rs`

```rust
#![forbid(unsafe_code)]

pub fn assert_phase_history(actual: &[&str], expected: &[&str]) {
    assert_eq!(actual, expected, "phase history mismatch");
}

pub fn assert_contains_all(haystack: &str, needles: &[&str]) {
    for needle in needles {
        assert!(
            haystack.contains(needle),
            "missing required fragment: {needle}"
        );
    }
}
```

## 9. Test File Scaffolding

The following code blocks are representative, non-empty starting points. They use the real harness interfaces, real assertions, and the 4-part structure mandated by the TDD.

### 9.1 `agent_pty_session.rs`

```rust
#![forbid(unsafe_code)]

mod support;

#[cfg(unix)]
mod tests {
    use daemon::pty::session::SessionManager;

    use crate::support::{assertions::assert_phase_history, TestContext};

    #[tokio::test]
    async fn spawn_records_runtime_ids_before_spawned() -> anyhow::Result<()> {
        let ctx = TestContext::new()?;
        let mut manager = SessionManager::for_test(&ctx.state_root, &ctx.artifact_root).await?;

        let session = manager.spawn_test_agent(["pty-echo", "hello"]).await?;

        assert!(session.sid > 0);
        assert!(session.pid > 0);
        assert!(session.pgid > 0);
        assert_phase_history(
            &session.phase_history,
            &["created", "spawned", "active", "terminating", "exited", "reaped", "archived"],
        );
        Ok(())
    }

    #[tokio::test]
    async fn crash_recovery_rebuilds_stream_index_from_durable_chunks() -> anyhow::Result<()> {
        let ctx = TestContext::new()?;
        let mut manager = SessionManager::for_test(&ctx.state_root, &ctx.artifact_root).await?;

        let session = manager.spawn_chunky_test_agent().await?;
        manager.simulate_torn_index_line(&session.session_id).await?;
        manager.recover_artifacts(&session.session_id).await?;

        let index = manager.read_stream_index(&session.session_id).await?;
        assert!(index.is_contiguous());
        assert!(index.last_seq() >= 1);
        Ok(())
    }
}
```

### 9.2 `agent_contract_session.rs`

```rust
#![forbid(unsafe_code)]

mod support;

mod tests {
    use daemon::jobs::runner::JobRunner;

    use crate::support::TestContext;

    #[tokio::test]
    async fn contract_mode_preserves_stdout_and_stderr() -> anyhow::Result<()> {
        let ctx = TestContext::new()?;
        let runner = JobRunner::for_test(&ctx.state_root, &ctx.artifact_root).await?;

        let run = runner.run_test_contract(["stdout-stderr"]).await?;

        assert_eq!(run.output_frames.stdout.len(), 1);
        assert_eq!(run.output_frames.stderr.len(), 1);
        assert!(run.control_events.is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn schema_invalid_output_fails_without_advancing_commit_fence() -> anyhow::Result<()> {
        let ctx = TestContext::new()?;
        let runner = JobRunner::for_test(&ctx.state_root, &ctx.artifact_root).await?;

        let run = runner.run_invalid_result_contract().await?;

        assert_eq!(run.error_code.as_deref(), Some("AGENT_OUTPUT_CONTRACT_INVALID"));
        assert!(run.visible_after_seq.is_none());
        assert!(!run.stage_head_advanced);
        Ok(())
    }
}
```

### 9.3 `agent_procurement_ingestion.rs`

```rust
#![forbid(unsafe_code)]

mod support;

mod tests {
    use daemon::jobs::runner::JobRunner;

    use crate::support::TestContext;

    #[tokio::test]
    async fn captured_evidence_loop_writes_stage_outputs_atomically() -> anyhow::Result<()> {
        let ctx = TestContext::new()?;
        let runner = JobRunner::for_test(&ctx.state_root, &ctx.artifact_root).await?;

        let outcome = runner.run_fts_bundle_fixture("fixtures/fts/minimal-batch").await?;

        assert!(outcome.stage_head_advanced);
        assert!(outcome.commit_seq > 0);
        assert_eq!(outcome.stage_status, "succeeded");
        assert!(outcome.manifest_path.ends_with("run-manifest.md"));
        Ok(())
    }

    #[tokio::test]
    async fn experiment4_batch_leaves_no_orphans() -> anyhow::Result<()> {
        let ctx = TestContext::new()?;
        let runner = JobRunner::for_test(&ctx.state_root, &ctx.artifact_root).await?;

        let summary = runner.run_reliability_batch(10).await?;

        assert!(summary.success_rate >= 0.90);
        assert_eq!(summary.orphan_count, 0);
        Ok(())
    }
}
```

### 9.4 `adapter_codex.rs`

```rust
#![forbid(unsafe_code)]

mod support;

mod tests {
    use daemon::agents::codex::CodexAdapter;

    #[tokio::test]
    async fn probe_reports_required_capability_fields() -> anyhow::Result<()> {
        let adapter = CodexAdapter::for_test("0.0.0-test");
        let caps = adapter.probe(&Default::default()).await?;

        assert!(!caps.cli_version.is_empty());
        assert!(matches!(caps.structured_output, true | false));
        assert!(matches!(caps.pty_viable, true | false));
        Ok(())
    }

    #[test]
    fn jsonl_parser_emits_canonical_events() -> anyhow::Result<()> {
        let mut parser = CodexAdapter::for_test("0.0.0-test").parser_for_contract();
        let events = parser.consume_line(r#"{"event":"checkpoint","seq":1,"path":"checkpoint-001.json"}"#)?;

        assert_eq!(events.len(), 1);
        Ok(())
    }
}
```

Add an explicit negative fixture that covers both malformed JSONL and truncated tail-at-EOF behavior. The adapter parser surface is not sufficiently specified unless those failure modes are tested separately from the happy path.

### 9.5 `adapter_claude.rs`

```rust
#![forbid(unsafe_code)]

mod support;

mod tests {
    use daemon::agents::claude::ClaudeAdapter;

    #[tokio::test]
    async fn unknown_version_enters_canary() -> anyhow::Result<()> {
        let adapter = ClaudeAdapter::for_test("9999.0.0");
        let caps = adapter.probe(&Default::default()).await?;
        assert!(caps.is_canary);
        Ok(())
    }

    #[test]
    fn stream_json_parser_preserves_output_lane() -> anyhow::Result<()> {
        let mut parser = ClaudeAdapter::for_test("0.0.0-test").parser_for_contract();
        let (frames, events) = parser.consume_stream_chunk(br#"{"event":"started"}"#)?;

        assert_eq!(frames.len(), 1);
        assert_eq!(events.len(), 1);
        Ok(())
    }
}
```

Add an explicit negative fixture that covers malformed-stream and partial-tail behavior for Claude. The stream-json parser must prove both multi-chunk happy-path assembly and finish-time failure classification.

### 9.6 `adapter_result_contract.rs`

```rust
#![forbid(unsafe_code)]

mod support;

mod tests {
    use daemon::agents::contracts::ResultContractValidator;

    #[test]
    fn fallback_manifest_marks_agent_narrative_missing() -> anyhow::Result<()> {
        let validator = ResultContractValidator::for_test();
        let manifest = validator.make_fallback_manifest("session-1")?;

        assert!(manifest.contains("agent_narrative_missing: true"));
        assert!(manifest.contains("Daemon Evidence Appendix"));
        Ok(())
    }

    #[test]
    fn disagreement_summary_preserves_both_values() -> anyhow::Result<()> {
        let validator = ResultContractValidator::for_test();
        let manifest = validator.append_disagreement_summary(
            "agent_claimed_fingerprint_state",
            "matched",
            "mismatched",
        )?;

        assert!(manifest.contains("matched"));
        assert!(manifest.contains("mismatched"));
        Ok(())
    }
}
```

### 9.7 `skill_lint.rs`

```rust
#![forbid(unsafe_code)]

mod support;

mod tests {
    use daemon::skills::lint::SkillLint;

    use crate::support::skills::make_skill_dir;

    #[test]
    fn valid_skill_dir_passes() -> anyhow::Result<()> {
        let dir = make_skill_dir(true)?;
        SkillLint::new().check(dir.path())?;
        Ok(())
    }

    #[test]
    fn missing_schema_fails_with_skill_directory_invalid() {
        let dir = make_skill_dir(false).expect("skill dir");
        let err = SkillLint::new().check(dir.path()).expect_err("lint should fail");
        assert!(err.to_string().contains("SKILL_DIRECTORY_INVALID"));
    }
}
```

### 9.8 `skill_canary_harness.rs`

```rust
#![forbid(unsafe_code)]

mod support;

mod tests {
    use daemon::skills::canary::CanaryHarness;

    use crate::support::TestContext;

    #[tokio::test]
    async fn page_shape_drift_is_classified() -> anyhow::Result<()> {
        let ctx = TestContext::new()?;
        let harness = CanaryHarness::for_test(&ctx.state_root).await?;

        let result = harness.run_fixture("fixtures/fts/page-shape-drift").await?;

        assert_eq!(result.error_code.as_deref(), Some("PAGE_SHAPE_DRIFT"));
        Ok(())
    }

    #[tokio::test]
    async fn captcha_blocked_yields_blocked_human_action() -> anyhow::Result<()> {
        let ctx = TestContext::new()?;
        let harness = CanaryHarness::for_test(&ctx.state_root).await?;

        let result = harness.run_fixture("fixtures/fts/captcha").await?;

        assert_eq!(result.outcome_class, "blocked_human_action");
        Ok(())
    }
}
```

### 9.9 `session_reaper.rs`

```rust
#![forbid(unsafe_code)]

mod support;

#[cfg(unix)]
mod tests {
    use daemon::sessions::reaper::SessionReaper;

    use crate::support::TestContext;

    #[tokio::test]
    async fn leaked_descendants_record_leaked_verdict() -> anyhow::Result<()> {
        let ctx = TestContext::new()?;
        let reaper = SessionReaper::for_test(&ctx.state_root, &ctx.artifact_root).await?;

        let result = reaper.run_stubborn_process_case().await?;

        assert_eq!(result.cleanup_verdict, "leaked");
        assert!(!result.leaked_pids.is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn daemon_crash_reconciliation_synthesizes_error_manifest() -> anyhow::Result<()> {
        let ctx = TestContext::new()?;
        let reaper = SessionReaper::for_test(&ctx.state_root, &ctx.artifact_root).await?;

        let result = reaper.reconcile_after_crash().await?;

        assert_eq!(result.error_code, "AGENT_DAEMON_CRASH_DURING_SESSION");
        assert!(result.manifest_path.ends_with(".md"));
        Ok(())
    }
}
```

### 9.10 `ipc_handshake.rs`

```rust
#![forbid(unsafe_code)]

mod support;

#[cfg(unix)]
mod tests {
    use serial_test::serial;

    use daemon::ipc::handshake::HandshakeServer;

    use crate::support::TestContext;

    #[tokio::test]
    #[serial]
    async fn schema_epoch_mismatch_is_explicit() -> anyhow::Result<()> {
        let ctx = TestContext::new()?;
        let server = HandshakeServer::for_test(&ctx.runtime_root).await?;

        let reply = server.hello_for_test(7, Some("old-instance")).await?;

        assert_eq!(reply.kind, "schema_epoch_mismatch");
        assert!(reply.expected_schema_epoch > 0);
        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn lock_prevents_second_writer() -> anyhow::Result<()> {
        let ctx = TestContext::new()?;
        let first = HandshakeServer::for_test(&ctx.runtime_root).await?;
        let second = HandshakeServer::for_test(&ctx.runtime_root).await;

        assert!(first.is_ready());
        assert!(second.is_err());
        Ok(())
    }
}
```

## 10. Command Matrix

Use these commands once the daemon crate exists:

```bash
# Static gates (per-commit)
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo fmt --check

# Fast unit suites (per-commit)
cargo test -p daemon --features test-support --test adapter_codex
cargo test -p daemon --features test-support --test adapter_claude
cargo test -p daemon --features test-support --test adapter_result_contract
cargo test -p daemon --features test-support --test skill_lint

# Integration suites (per-commit)
cargo test -p daemon --features test-support --test agent_pty_session -- --nocapture
cargo test -p daemon --features test-support --test agent_contract_session -- --nocapture
cargo test -p daemon --features test-support --test session_reaper -- --nocapture
cargo test -p daemon --features test-support --test ipc_handshake -- --nocapture

# Nightly deterministic lanes (captured evidence only)
RUN_CAPTURED_FTS=1 cargo test -p daemon --features test-support --test skill_canary_harness -- --nocapture
RUN_CAPTURED_FTS=1 cargo test -p daemon --features test-support --test agent_procurement_ingestion -- --nocapture

# Scheduled dedicated networked runner
RUN_LIVE_FTS=1 RUN_AGENT_CLIS=1 cargo test -p daemon --features test-support --test skill_canary_harness -- --nocapture
RUN_LIVE_FTS=1 RUN_AGENT_CLIS=1 cargo test -p daemon --features test-support --test agent_procurement_ingestion -- --nocapture

# Manual target-host Experiment 4 confirmation
RUN_LIVE_FTS=1 RUN_AGENT_CLIS=1 cargo test -p daemon --features test-support --test agent_procurement_ingestion -- --exact experiment4_live_batch_confirmation --nocapture
```

Do not use `#[ignore]` as the primary live-gate mechanism. Use environment-variable gating and explicit test filters so skipped live lanes are visible and intentional.

### 10.1 CI Workflow Skeleton

The bootstrap should include a CI file such as `.github/workflows/l1-daemon-core-tests.yml`:

```yaml
name: l1-daemon-core-tests

on:
  pull_request:
  push:
    branches: [main]
  schedule:
    - cron: "0 3 * * *"

jobs:
  fast:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@nightly
        with:
          components: rustfmt, clippy
      - run: cargo check --all-targets
      - run: cargo clippy --all-targets -- -D warnings
      - run: cargo fmt --check
      - run: cargo test -p daemon --features test-support --test adapter_codex
      - run: cargo test -p daemon --features test-support --test adapter_claude
      - run: cargo test -p daemon --features test-support --test adapter_result_contract
      - run: cargo test -p daemon --features test-support --test skill_lint
      - run: cargo test -p daemon --features test-support --test agent_pty_session -- --nocapture
      - run: cargo test -p daemon --features test-support --test agent_contract_session -- --nocapture
      - run: cargo test -p daemon --features test-support --test session_reaper -- --nocapture
      - run: cargo test -p daemon --features test-support --test ipc_handshake -- --nocapture

  captured-nightly:
    if: github.event_name == 'schedule'
    runs-on: ubuntu-latest
    env:
      RUN_CAPTURED_FTS: "1"
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@nightly
      - run: cargo test -p daemon --features test-support --test skill_canary_harness -- --nocapture
      - run: cargo test -p daemon --features test-support --test agent_procurement_ingestion -- --nocapture
```

Keep live-network jobs on a separately labeled runner. Do not put the 10-batch live Experiment 4 confirmation into ordinary PR CI.

## 11. Artifact Checklist

Before the first full test run, the following must exist:

- `Cargo.toml`
- `rust-toolchain.toml`
- `crates/daemon/Cargo.toml`
- `crates/daemon/src/main.rs`
- `crates/daemon/src/bin/test_agent.rs`
- `crates/daemon/src/db/sqlite.rs`
- `crates/daemon/src/pty/session.rs`
- `crates/daemon/src/pty/capture.rs`
- `crates/daemon/src/pty/health.rs`
- `crates/daemon/src/jobs/runner.rs`
- `crates/daemon/src/agents/adapter.rs`
- `crates/daemon/src/agents/codex.rs`
- `crates/daemon/src/agents/claude.rs`
- `crates/daemon/src/agents/artifacts.rs`
- `crates/daemon/src/agents/contracts.rs`
- `crates/daemon/src/prompt/renderer.rs`
- `crates/daemon/src/errors/manifest.rs`
- `crates/daemon/src/errors/markdown.rs`
- `crates/daemon/src/ipc/descriptor.rs`
- `crates/daemon/src/ipc/handshake.rs`
- `crates/daemon/src/sessions/reaper.rs`
- `.github/workflows/l1-daemon-core-tests.yml`
- `migrations/0001_sqlite_state_initial.sql`
- `migrations/0003_sqlite_runtime.sql`
- `skills/procurement-ingestion/fts/`
- `skills/manifest-generation/`
- `crates/daemon/tests/fixtures/adapter/codex/`
- `crates/daemon/tests/fixtures/adapter/claude/`
- `crates/daemon/tests/fixtures/fts/`
- captured FTS evidence bundle for nightly deterministic E2E

## 12. Structured Logging Requirements

Every non-trivial test should log the following markers:

```text
[TEST_START] <test-id>
[SETUP] ...
[ASSERT] ...
[ASSERT] ...
[TEST_END] <test-id> PASS
```

If a test fails after spawning a real process, it must also emit:

```text
[CLEANUP] pid=<pid> pgid=<pgid> verdict=<clean|forced|leaked>
```

## 13. False Positive Prevention

| Risk | Prevention rule |
|------|------------------|
| “Artifact exists” but content is wrong | assert frontmatter keys, section names, appendix fields, hash, and size contract |
| runner updated row counts but not authoritative heads | assert `process_stage_heads`, `process_stage_status`, and `commit_clock` in the same test |
| transcript looked correct but contract was invalid | assert result schema validation and OQG outcome separately from transcript text |
| child exited but descendants still leaked | assert cleanup verdict plus explicit descendant sweep result |
| restart logic conflates desktop restart with daemon restart | run same-instance resume and new-instance invalidation as separate tests |
| source replay masks live failures | keep captured-evidence T3 and live-source T4 as distinct lanes |
| helper binary hides platform-specific behavior | use real `waitpid`, PTY, UDS, and lock files in integration tests |
| 10-batch live reliability gate makes ordinary CI flaky or too slow | keep captured 10-run soak in nightly CI and reserve the live 10-batch gate for dedicated runner / manual target-host confirmation |

## 14. Notes For Implementation

- Keep the helper binary count low. One `test_agent` binary with subcommands is preferred over many tiny binaries.
- Use temp directories for runtime roots so UDS and lock files do not collide between tests.
- Use `serial_test` for tests that exercise lock uniqueness or a shared global CLI home.
- Keep Bun out of Rust unit tests, but do validate Bun in the bootstrap script because L1 still binds Bun-backed skills.
- The TDD enumerates `138` required test IDs. When implementation begins, copy those IDs into test names, log markers, or doc comments so completion audit can compare planned vs implemented coverage mechanically.
