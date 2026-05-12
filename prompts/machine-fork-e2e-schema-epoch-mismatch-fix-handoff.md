# Desktop Schema Epoch Mismatch - Investigation + Fix Handoff

ULTRATHINK and be meticulous.

## MISSION

Fix the desktop/Tauri schema epoch mismatch that blocks machine-fork E2E full-control validation. The daemon SQLite schema is at epoch 9 after `0012_l4_sync_state`, but desktop bootstrap, query service, daemon bridge, and native fixtures still expect epoch 8, so a daemon-created database or daemon welcome at epoch 9 is treated as incompatible. Preserve rejection of genuinely incompatible schema epochs; do not lower the daemon schema epoch or disable migration `0012_l4_sync_state`.

Out of scope: validating Layer 4 sync/RLS/shared visibility, weakening schema mismatch handling, deleting local dirty files, or changing non-desktop pipeline behavior.

---

## REPRODUCED STATE

Verified in `/Users/zacfarrell/Desktop/Testproject1` on `2026-05-12`.

```bash
git status --short --branch
git log --oneline -1
```

Observed:

```text
## main...origin/main
M  .DS_Store
A  bin/__pycache__/br-claimcpython-313.pyc
A  bin/__pycache__/br-completecpython-313.pyc
2a7d8ec docs: record machine fork e2e pause
```

The dirty `.DS_Store` / `bin/__pycache__` entries are shared-checkout noise. Do not delete, stash, revert, or commit them unless explicitly instructed.

Environment:

```text
Darwin Mac 25.3.0 Darwin Kernel Version 25.3.0: Wed Jan 28 20:56:34 PST 2026; root:xnu-12377.91.3~2/RELEASE_ARM64_T8122 arm64
bun 1.3.13
cargo 1.96.0-nightly (cbb9bb8bd 2026-03-13)
nightly-aarch64-apple-darwin (overridden by '/Users/zacfarrell/Desktop/Testproject1/rust-toolchain.toml')
```

Critical mismatch check:

```bash
rg -n "pub const DEFAULT_SCHEMA_EPOCH|EXPECTED_SCHEMA_EPOCH|DEFAULT_SCHEMA_EPOCH" \
  crates/daemon/src/db/sqlite.rs \
  apps/desktop/src-tauri/src/main.rs \
  apps/desktop/src-tauri/src/query_service.rs \
  apps/desktop/src-tauri/src/daemon_bridge.rs \
  apps/desktop/src-tauri/tests -g '*.rs'
```

Observed:

```text
crates/daemon/src/db/sqlite.rs:75:pub const DEFAULT_SCHEMA_EPOCH: i64 = 9;
apps/desktop/src-tauri/src/daemon_bridge.rs:16:pub const DEFAULT_SCHEMA_EPOCH: i64 = 8;
apps/desktop/src-tauri/src/query_service.rs:14:const EXPECTED_SCHEMA_EPOCH: i64 = 8;
apps/desktop/src-tauri/src/main.rs:51:const EXPECTED_SCHEMA_EPOCH: i64 = 8;
apps/desktop/src-tauri/tests/native_shell.rs:21:const EXPECTED_SCHEMA_EPOCH_FOR_TEST: i64 = 8;
apps/desktop/src-tauri/tests/support/native_shell.rs:20:const EXPECTED_SCHEMA_EPOCH_FOR_TEST: i64 = 8;
```

`origin/main` currently shows the same mismatch at `2a7d8ec`.

Important coverage finding: the current desktop native tests pass despite the mismatch because fixtures also encode epoch 8.

```text
cargo test -p procintel-desktop --test daemon_bridge  # 14 passed
cargo test -p procintel-desktop --test query_service  # 25 passed
cargo test -p procintel-desktop --test native_shell   # 22 passed
```

Do not treat these green tests alone as proof of the fix. Add a regression guard that fails before the fix.

---

## RESEARCH PHASE

### 1. Daemon Schema Authority

Read:

- `crates/daemon/src/db/sqlite.rs` - migration list and authoritative `DEFAULT_SCHEMA_EPOCH`.
- `crates/daemon/src/main.rs` - daemon runtime uses `DEFAULT_SCHEMA_EPOCH` for state DB open and handshake context.
- `crates/daemon/tests/ipc_handshake.rs` - daemon-side contract for successful and rejected schema handshakes.

Key code:

```rust
// crates/daemon/src/db/sqlite.rs
("0012_l4_sync_state", 9, L4_SYNC_STATE_MIGRATION_PATH),
pub const DEFAULT_SCHEMA_EPOCH: i64 = 9;
```

```rust
// crates/daemon/src/main.rs
Ok(HandshakeContext {
    protocol_version: runtime.descriptor.protocol_version,
    daemon_version: env!("CARGO_PKG_VERSION").to_owned(),
    instance_id: runtime.descriptor.instance_id.clone(),
    schema_epoch: DEFAULT_SCHEMA_EPOCH,
    commit_seq: runtime.state_db.current_commit_seq()?,
    // ...
})
```

You must be able to answer:

1. Which migration advanced the schema to epoch 9?
2. Where does the daemon report epoch 9 during IPC handshake?
3. How does the daemon reject a desktop hello whose expected epoch differs from `DEFAULT_SCHEMA_EPOCH`?

### 2. Desktop Bootstrap and Handshake

Read:

- `apps/desktop/src-tauri/src/main.rs` - `EXPECTED_SCHEMA_EPOCH`, storage inspection, native hello payload, and welcome parsing.
- `apps/desktop/src-tauri/src/daemon_bridge.rs` - standalone bridge default epoch and hello payload.
- `apps/desktop/src-tauri/src/query_service.rs` - read snapshot schema validation.

Key code:

```rust
// apps/desktop/src-tauri/src/main.rs
const EXPECTED_SCHEMA_EPOCH: i64 = 8;

if schema_epoch != 0 && schema_epoch != EXPECTED_SCHEMA_EPOCH {
    return StorageInspection {
        state: StorageState::SchemaMismatch,
        reasons: vec![format!(
            "Desktop expects schema epoch {EXPECTED_SCHEMA_EPOCH}, but {} is at {schema_epoch}.",
            db_path.display()
        )],
    };
}

if schema_epoch != EXPECTED_SCHEMA_EPOCH {
    return Err(HandshakeFailure::Incompatible(format!(
        "Daemon reported schema epoch {schema_epoch}, expected {EXPECTED_SCHEMA_EPOCH}."
    )));
}
```

```rust
// apps/desktop/src-tauri/src/daemon_bridge.rs
pub const DEFAULT_PROTOCOL_VERSION: u32 = 1;
pub const DEFAULT_SCHEMA_EPOCH: i64 = 8;

DaemonWireRequest::Hello {
    desktop_version: self.config.desktop_version.clone(),
    supported_protocol_range: self.config.supported_protocol_range.clone(),
    expected_schema_epoch: self.config.expected_schema_epoch,
    last_seen_instance_id: self.config.last_seen_instance_id.clone(),
}
```

You must be able to answer:

1. Which desktop paths reject a database or daemon welcome at epoch 9?
2. Why does `schema_epoch == 0` remain a special case for empty/fresh storage?
3. Why does `DaemonBridgeConfig::new` need to align with the daemon's current epoch?

### 3. Desktop Tests and Fixture Drift

Read:

- `apps/desktop/src-tauri/tests/daemon_bridge.rs` - fake descriptor/welcome payloads currently use literal `8`.
- `apps/desktop/src-tauri/tests/native_shell.rs` - native full-control path and fixture epoch constant.
- `apps/desktop/src-tauri/tests/support/native_shell.rs` - shared native shell descriptor/welcome helpers.
- `apps/desktop/src-tauri/tests/query_service.rs` - already includes daemon SQLite with `#[path = "../../../../crates/daemon/src/db/sqlite.rs"]`.

Current fixture drift:

```text
apps/desktop/src-tauri/tests/daemon_bridge.rs:59:                "schemaEpoch": 8,
apps/desktop/src-tauri/tests/daemon_bridge.rs:120:        "schemaEpoch": 8,
apps/desktop/src-tauri/tests/daemon_bridge.rs:209:                        "schemaEpoch": 8,
apps/desktop/src-tauri/tests/native_shell.rs:21:const EXPECTED_SCHEMA_EPOCH_FOR_TEST: i64 = 8;
apps/desktop/src-tauri/tests/support/native_shell.rs:20:const EXPECTED_SCHEMA_EPOCH_FOR_TEST: i64 = 8;
apps/desktop/src-tauri/tests/query_service.rs:42:... "schemaEpoch":8 ...
```

You must be able to answer:

1. Which tests currently pass only because their fake daemon also says epoch 8?
2. Which test can be converted into a real regression guard that fails before the fix?
3. How can tests compare desktop defaults against the daemon `DEFAULT_SCHEMA_EPOCH` without adding a production dependency?

---

## HYPOTHESIS SPACE

1. **Stale duplicated constants in desktop/Tauri** - high confidence. Daemon epoch moved to 9; desktop constants and fixtures stayed at 8.
2. **Missing regression guard for cross-crate epoch drift** - high confidence. The three desktop native gates pass even with the mismatch because fixtures are internally self-consistent at 8.
3. **A shared production constant may be desirable but is not required for this fix** - medium confidence. The desktop crate currently has no daemon dependency; tests can import daemon SQLite with `#[path]`, but production sharing may require crate-boundary changes that are larger than this bug.

---

## FIX PLAN

1. Reproduce the mismatch with the critical `rg` command in this handoff.
2. Add or update a regression guard that fails on the current checkout and passes after the fix. Recommended minimal guard:
   - In `apps/desktop/src-tauri/tests/daemon_bridge.rs`, import daemon SQLite like `query_service.rs` does:

```rust
#[allow(dead_code)]
#[path = "../../../../crates/daemon/src/db/sqlite.rs"]
mod sqlite_impl;
```

   - Add a test asserting `daemon_bridge_impl::DEFAULT_SCHEMA_EPOCH == sqlite_impl::DEFAULT_SCHEMA_EPOCH` and `DaemonBridgeConfig::new(runtime.dir.clone()).expected_schema_epoch == sqlite_impl::DEFAULT_SCHEMA_EPOCH`.
   - Also assert the hello sent by `DaemonBridge` carries `expectedSchemaEpoch: 9` after the fix.
3. Update desktop epoch expectations from 8 to 9 in:
   - `apps/desktop/src-tauri/src/main.rs`
   - `apps/desktop/src-tauri/src/query_service.rs`
   - `apps/desktop/src-tauri/src/daemon_bridge.rs`
   - `apps/desktop/src-tauri/tests/native_shell.rs`
   - `apps/desktop/src-tauri/tests/support/native_shell.rs`
   - Literal test payloads in `apps/desktop/src-tauri/tests/daemon_bridge.rs`
   - Literal descriptor payload in `apps/desktop/src-tauri/tests/query_service.rs`
4. Preserve the existing behavior that epoch 99, epoch 10, or any other nonzero unexpected epoch maps to schema mismatch/incompatible.
5. Do not lower `crates/daemon/src/db/sqlite.rs::DEFAULT_SCHEMA_EPOCH`; it is the authority and must stay 9.

---

## FIX CONSTRAINTS

1. Do not delete files. Do not touch `.DS_Store` or `bin/__pycache__/`.
2. Use Bun for TypeScript layers and Cargo for Rust layers. Do not use npm/yarn/pnpm.
3. Do not add a compatibility shim for epoch 8. Early development policy says fix the code directly.
4. Do not bypass schema validation by accepting both 8 and 9. The desktop must reject genuinely incompatible epochs.
5. Do not validate Layer 4 shared visibility/RLS as part of this fix.
6. If this is bead-tracked work, use `bin/br-claim <bead-id>` before edits and `bin/br-complete <bead-id>` after verification. Do not use raw `br update --status=in_progress` or `br close`.

---

## DELIVERABLES

1. Root-cause note in the final response explaining the stale desktop epoch constants and fixture drift.
2. Production code updated so desktop sends/expects schema epoch 9 everywhere it previously used epoch 8.
3. Desktop tests updated so fake descriptors/welcomes match daemon epoch 9.
4. A regression guard that compares at least one desktop default against daemon `DEFAULT_SCHEMA_EPOCH`, so a future daemon epoch bump cannot silently leave desktop at the old value.
5. Verification transcript for all commands in the validation loop below.

---

## REQUIRED VALIDATE / FIX LOOP

Run this loop until every command is green. Do not stop at the first green targeted test because the current suite already passes while the defect exists.

### Loop A: Reproduce Red

Before editing, run:

```bash
rg -n "pub const DEFAULT_SCHEMA_EPOCH|EXPECTED_SCHEMA_EPOCH|DEFAULT_SCHEMA_EPOCH" \
  crates/daemon/src/db/sqlite.rs \
  apps/desktop/src-tauri/src/main.rs \
  apps/desktop/src-tauri/src/query_service.rs \
  apps/desktop/src-tauri/src/daemon_bridge.rs \
  apps/desktop/src-tauri/tests -g '*.rs'
```

Expected before fix: daemon reports 9; desktop reports 8.

Add the regression guard, then confirm it fails before the constant update:

```bash
cargo test -p procintel-desktop --test daemon_bridge schema_epoch
```

Expected before fix: failure showing desktop default 8 differs from daemon 9.

### Loop B: Fix

Apply the minimal epoch alignment and fixture updates. Then rerun:

```bash
rg -n "pub const DEFAULT_SCHEMA_EPOCH|EXPECTED_SCHEMA_EPOCH|DEFAULT_SCHEMA_EPOCH" \
  crates/daemon/src/db/sqlite.rs \
  apps/desktop/src-tauri/src/main.rs \
  apps/desktop/src-tauri/src/query_service.rs \
  apps/desktop/src-tauri/src/daemon_bridge.rs \
  apps/desktop/src-tauri/tests -g '*.rs'
```

Expected after fix: every active desktop/Tauri epoch expectation is 9. Tests may still contain deliberate mismatch values like 99 or `DEFAULT_SCHEMA_EPOCH + 1`; those are valid.

### Loop C: Targeted Desktop Native Gates

```bash
cargo test -p procintel-desktop --test query_service
cargo test -p procintel-desktop --test daemon_bridge
cargo test -p procintel-desktop --test native_shell
```

Expected after fix: all pass.

### Loop D: Required Rust Gates

```bash
cargo fmt --check
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
```

Expected after fix: all pass.

### Loop E: Fresh DB Epoch Probe

Run a focused daemon schema probe:

```bash
cargo test -p daemon --test sqlite_schema rs_sqlite_schema_001_full_schema_activation_sets_user_version
```

If that exact test name changed, use:

```bash
cargo test -p daemon --test sqlite_schema user_version
```

Expected after fix: fresh state DB `PRAGMA user_version` equals daemon `DEFAULT_SCHEMA_EPOCH` and desktop expected epoch is also 9.

---

## FINAL CHECKS

Before handing back:

1. Re-run the critical `rg` check and quote the relevant epoch lines in the final response.
2. Confirm the regression guard failed before the fix or explain why it could not be run red-first.
3. Confirm all targeted desktop native gates passed.
4. Confirm `cargo fmt --check`, `cargo check --all-targets`, and `cargo clippy --all-targets -- -D warnings` passed, or document exact blockers.
5. Run `git status --short --branch` and identify only the files intentionally changed for this fix.
