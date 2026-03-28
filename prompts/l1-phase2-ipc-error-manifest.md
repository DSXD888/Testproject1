# L1 Phase 2 IPC Walking Skeleton — Error Manifest

## Defect Fingerprint

- Scope: `testproject1-63g.4` Phase 2 walking-skeleton startup / IPC surface
- Impact: Shared worktree does not satisfy the required per-bead quality gate
- Affected files:
  - `crates/daemon/src/main.rs`
  - `crates/daemon/src/ipc/descriptor.rs`
  - `crates/daemon/src/ipc/handshake.rs`
  - `crates/daemon/tests/ipc_handshake.rs`
  - `crates/daemon/Cargo.toml`

## Environment

- Repo: `/Users/alinaaitova/Desktop/Testproject1`
- Branch: `main`
- Date observed (UTC): `2026-03-25T01:30:47Z`
- Relevant bead: `br-testproject1-63g.4`

## Reproduction Protocol

Run from repo root with the current shared worktree:

```bash
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test --test ipc_handshake -- --nocapture
```

## Evidence

### E1 — `cargo check --all-targets` fails on missing import

```text
error[E0433]: cannot find type `Uid` in this scope
   --> crates/daemon/tests/ipc_handshake.rs:280:44
    |
280 |     setup.context.expected_peer_uid = Some(Uid::current().as_raw().saturating_add(1));
    |                                            ^^^ use of undeclared type `Uid`
```

### E2 — `cargo clippy --all-targets -- -D warnings` fails on active IPC/startup work

Representative failures:

```text
error: method `state_db_path` is never used
  --> crates/daemon/tests/../src/ipc/descriptor.rs:51:12

error: file opened with `create`, but `truncate` behavior not defined
   --> crates/daemon/src/ipc/descriptor.rs:104:14

error: redundant clone
   --> crates/daemon/src/main.rs:79:48

error: item in documentation is missing backticks
  --> crates/daemon/src/main.rs:32:49
```

Additional failures are present in:
- `crates/daemon/src/db/sqlite.rs`
- `crates/daemon/src/ipc/descriptor.rs`
- `crates/daemon/tests/ipc_handshake.rs`

### E3 — `cargo test --test ipc_handshake -- --nocapture` fails 5 tests with missing socket path

```text
Error: connecting to /var/folders/.../runtime/daemon.sock

Caused by:
    No such file or directory (os error 2)
```

Failing tests:
- `it_ipc_003_successful_handshake_returns_runtime_compatibility`
- `it_ipc_004_protocol_range_mismatch_is_rejected`
- `it_ipc_005_schema_epoch_mismatch_reports_expected_and_actual`
- `it_ipc_008_socket_accepts_handshake_only`
- `it_ipc_009_peer_uid_mismatch_is_rejected`

Passing tests in the same file:
- `ci_core_004_runtime_inventory_matches_manifest`
- `it_ipc_001_descriptor_is_written_only_after_runtime_is_ready`
- `it_ipc_002_lock_file_blocks_second_writer`
- `it_ipc_010_startup_failure_never_writes_ready_descriptor`
- `it_ipc_013_state_db_bootstrap_uses_one_database`

### E4 — Likely root cause for E3 is visible in the harness implementation

`HandshakeHarness::new()` creates a temporary context but does not retain it:

```rust
fn new() -> Result<Self> {
    let ctx = support::TestContext::new()?;
    let runtime_paths = RuntimePaths::new(ctx.state_root.clone(), ctx.runtime_root.clone());
    fs::create_dir_all(runtime_paths.runtime_dir())?;
    fs::set_permissions(runtime_paths.runtime_dir(), fs::Permissions::from_mode(0o700))?;
    let listener = bind_listener(&runtime_paths)?;
    // ...
    Ok(Self {
        context,
        listener,
        socket_path: runtime_paths.socket_path().to_path_buf(),
    })
}
```

`TestContext` owns a `TempDir`. Because `ctx` is dropped at the end of `new()`, the temp directory is removed before `request()` tries to connect to `socket_path`.

## Ranked Root Cause Hypotheses

1. `HandshakeHarness` drops the temp runtime directory before the client connects.
   - Evidence: E3 + E4
   - Expected fix: store `TestContext` inside `HandshakeHarness` so the tempdir stays alive for the full test.

2. `ipc_handshake.rs` is missing `use nix::unistd::Uid;`.
   - Evidence: E1
   - Expected fix: import `Uid` in the test file.

3. The active startup/IPC implementation has not been brought up to the workspace lint baseline.
   - Evidence: E2
   - Expected fix: resolve the listed clippy diagnostics on `main.rs`, `descriptor.rs`, `ipc_handshake.rs`, and any now-live `sqlite.rs` APIs compiled through the binary path.

## Fix Constraints

- Do not use `unsafe`.
- Do not disturb unrelated shared doc edits in the worktree.
- Keep fixes confined to the active `63g.4` surface.
- Preserve Layer 1 scope: handshake-only socket, no mutation commands.

## Acceptance Criteria For Resolution

- `cargo check --all-targets` passes
- `cargo clippy --all-targets -- -D warnings` passes
- `cargo test --test ipc_handshake -- --nocapture` passes
- `HandshakeHarness` keeps its temp runtime directory alive for the duration of each test
- The peer-UID mismatch test compiles and runs with an explicit `Uid` import
