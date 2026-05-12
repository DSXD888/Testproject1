# Desktop Schema Epoch Mismatch Blocks Machine Fork E2E - QA Error Manifest

ULTRATHINK and be meticulous. This is a forensic debugging document.

## DEFECT FINGERPRINT

| Field | Value |
|-------|-------|
| **Type** | integration_mismatch |
| **Severity** | high |
| **Reproducibility** | deterministic |
| **Affected Component** | `crates/daemon/src/db/sqlite.rs`, `apps/desktop/src-tauri/src/main.rs`, `apps/desktop/src-tauri/src/query_service.rs`, `apps/desktop/src-tauri/src/daemon_bridge.rs`, `apps/desktop/src-tauri/tests/*` |
| **Working Analogue** | Daemon schema epoch is internally self-consistent at 9. |
| **Scope** | Desktop/Tauri still expects SQLite schema epoch 8 while the daemon creates and reports epoch 9. |
| **Environment** | macOS Darwin 25.3.0 arm64, Bun 1.3.13, Cargo 1.96.0-nightly, repo branch `main` |
| **Git State** | `4d1d1dd docs: persist machine fork e2e helpers`; dirty shared checkout contains unrelated `.DS_Store` and `bin/__pycache__/` |

---

## REPRODUCTION PROTOCOL

Follow these steps exactly. No daemon build is required to reproduce the mismatch.

### Prerequisites

- Repository path: `/Users/zacfarrell/Desktop/Testproject1`
- Branch: `main`
- No product-code edits are needed.

### Step 1: Confirm checkout and HEAD

```bash
cd /Users/zacfarrell/Desktop/Testproject1
git status --short --branch
git log --oneline -1
```

Expected: checkout is on `main`; unrelated dirty files may exist and should not be touched.

Actual:

```text
## main...origin/main
 M .DS_Store
?? bin/__pycache__/
4d1d1dd docs: persist machine fork e2e helpers
```

### Step 2: Run the schema epoch critical check

```bash
rg -n "pub const DEFAULT_SCHEMA_EPOCH|EXPECTED_SCHEMA_EPOCH|DEFAULT_SCHEMA_EPOCH" \
  crates/daemon/src/db/sqlite.rs \
  apps/desktop/src-tauri/src/main.rs \
  apps/desktop/src-tauri/src/query_service.rs \
  apps/desktop/src-tauri/src/daemon_bridge.rs \
  apps/desktop/src-tauri/tests -g '*.rs'
```

Expected: daemon and desktop/Tauri agree on the active schema epoch.

Actual:

```text
crates/daemon/src/db/sqlite.rs:75:pub const DEFAULT_SCHEMA_EPOCH: i64 = 9;
apps/desktop/src-tauri/src/main.rs:51:const EXPECTED_SCHEMA_EPOCH: i64 = 8;
apps/desktop/src-tauri/src/query_service.rs:14:const EXPECTED_SCHEMA_EPOCH: i64 = 8;
apps/desktop/src-tauri/src/daemon_bridge.rs:16:pub const DEFAULT_SCHEMA_EPOCH: i64 = 8;
apps/desktop/src-tauri/tests/native_shell.rs:21:const EXPECTED_SCHEMA_EPOCH_FOR_TEST: i64 = 8;
apps/desktop/src-tauri/tests/support/native_shell.rs:20:const EXPECTED_SCHEMA_EPOCH_FOR_TEST: i64 = 8;
```

### Verification - Confirm Defect Present

```bash
nl -ba crates/daemon/src/db/sqlite.rs | sed -n '68,98p'
nl -ba apps/desktop/src-tauri/src/main.rs | sed -n '45,58p;876,904p;1488,1502p'
nl -ba apps/desktop/src-tauri/src/query_service.rs | sed -n '8,20p;244,256p'
nl -ba apps/desktop/src-tauri/src/daemon_bridge.rs | sed -n '10,20p;436,448p'
```

Output:

```text
crates/daemon/src/db/sqlite.rs:
72	    ("0012_l4_sync_state", 9, L4_SYNC_STATE_MIGRATION_PATH),
75	pub const DEFAULT_SCHEMA_EPOCH: i64 = 9;

apps/desktop/src-tauri/src/main.rs:
51	const EXPECTED_SCHEMA_EPOCH: i64 = 8;
884	    if schema_epoch != 0 && schema_epoch != EXPECTED_SCHEMA_EPOCH {
888	                "Desktop expects schema epoch {EXPECTED_SCHEMA_EPOCH}, but {} is at {schema_epoch}.",
1494	            if schema_epoch != EXPECTED_SCHEMA_EPOCH {
1496	                    "Daemon reported schema epoch {schema_epoch}, expected {EXPECTED_SCHEMA_EPOCH}."

apps/desktop/src-tauri/src/query_service.rs:
14	const EXPECTED_SCHEMA_EPOCH: i64 = 8;
251	        Self::open_read_connection(&self.db_path, EXPECTED_SCHEMA_EPOCH)

apps/desktop/src-tauri/src/daemon_bridge.rs:
16	pub const DEFAULT_SCHEMA_EPOCH: i64 = 8;
444	            expected_schema_epoch: DEFAULT_SCHEMA_EPOCH,
```

### Control - Origin Main

```bash
git fetch origin
git log --oneline -1 origin/main
git show origin/main:crates/daemon/src/db/sqlite.rs | rg -n "DEFAULT_SCHEMA_EPOCH|0012_l4_sync_state"
git show origin/main:apps/desktop/src-tauri/src/main.rs | rg -n "EXPECTED_SCHEMA_EPOCH" | sed -n '1,6p'
git show origin/main:apps/desktop/src-tauri/src/query_service.rs | rg -n "EXPECTED_SCHEMA_EPOCH" | sed -n '1,5p'
git show origin/main:apps/desktop/src-tauri/src/daemon_bridge.rs | rg -n "DEFAULT_SCHEMA_EPOCH" | sed -n '1,5p'
```

Output:

```text
4d1d1dd docs: persist machine fork e2e helpers
26:const L4_SYNC_STATE_MIGRATION_PATH: &str = "migrations/0012_l4_sync_state.sql";
72:    ("0012_l4_sync_state", 9, L4_SYNC_STATE_MIGRATION_PATH),
75:pub const DEFAULT_SCHEMA_EPOCH: i64 = 9;
51:const EXPECTED_SCHEMA_EPOCH: i64 = 8;
14:const EXPECTED_SCHEMA_EPOCH: i64 = 8;
16:pub const DEFAULT_SCHEMA_EPOCH: i64 = 8;
```

The mismatch is also present on `origin/main`.

---

## EVIDENCE BUNDLE

### E0 - Environment Evidence

**E0.1: OS and toolchain**

```bash
uname -a
bun --version
cargo --version
rustup show active-toolchain
```

```text
Darwin Mac 25.3.0 Darwin Kernel Version 25.3.0: Wed Jan 28 20:56:34 PST 2026; root:xnu-12377.91.3~2/RELEASE_ARM64_T8122 arm64
1.3.13
cargo 1.96.0-nightly (cbb9bb8bd 2026-03-13)
nightly-aarch64-apple-darwin (overridden by '/Users/zacfarrell/Desktop/Testproject1/rust-toolchain.toml')
```

**E0.2: Git state**

```bash
git status --short --branch
git log --oneline -1
```

```text
## main...origin/main
 M .DS_Store
?? bin/__pycache__/
4d1d1dd docs: persist machine fork e2e helpers
```

### E2 - Reproduction Evidence

**E2.1: Daemon epoch includes migration 0012 and reports 9**

```bash
nl -ba crates/daemon/src/db/sqlite.rs | sed -n '68,78p'
```

```text
68	        "0011_commercial_basis_coverage",
69	        8,
70	        L_COMMERCIAL_BASIS_COVERAGE_MIGRATION_PATH,
71	    ),
72	    ("0012_l4_sync_state", 9, L4_SYNC_STATE_MIGRATION_PATH),
73	];
74	
75	pub const DEFAULT_SCHEMA_EPOCH: i64 = 9;
```

**E2.2: Desktop bootstrap expects 8 and returns schema mismatch**

```bash
nl -ba apps/desktop/src-tauri/src/main.rs | sed -n '45,58p;884,891p'
```

```text
51	const EXPECTED_SCHEMA_EPOCH: i64 = 8;
884	    if schema_epoch != 0 && schema_epoch != EXPECTED_SCHEMA_EPOCH {
885	        return StorageInspection {
886	            state: StorageState::SchemaMismatch,
887	            reasons: vec![format!(
888	                "Desktop expects schema epoch {EXPECTED_SCHEMA_EPOCH}, but {} is at {schema_epoch}.",
```

**E2.3: Desktop daemon handshake expects 8 and rejects daemon epoch 9**

```bash
nl -ba apps/desktop/src-tauri/src/main.rs | sed -n '1488,1498p'
```

```text
1494	            if schema_epoch != EXPECTED_SCHEMA_EPOCH {
1495	                return Err(HandshakeFailure::Incompatible(format!(
1496	                    "Daemon reported schema epoch {schema_epoch}, expected {EXPECTED_SCHEMA_EPOCH}."
1497	                )));
```

**E2.4: QueryService read path expects 8**

```bash
nl -ba apps/desktop/src-tauri/src/query_service.rs | sed -n '11,15p;249,252p'
```

```text
14	const EXPECTED_SCHEMA_EPOCH: i64 = 8;
251	        Self::open_read_connection(&self.db_path, EXPECTED_SCHEMA_EPOCH)
```

**E2.5: DaemonBridge default expects 8**

```bash
nl -ba apps/desktop/src-tauri/src/daemon_bridge.rs | sed -n '15,17p;436,445p'
```

```text
15	pub const DEFAULT_PROTOCOL_VERSION: u32 = 1;
16	pub const DEFAULT_SCHEMA_EPOCH: i64 = 8;
444	            expected_schema_epoch: DEFAULT_SCHEMA_EPOCH,
```

---

## CODEPATH COMPARISON

### Working Path: Daemon Schema Bootstrap

Entry point: `crates/daemon/src/db/sqlite.rs`

Flow:
1. Migration list includes `0012_l4_sync_state` at epoch 9.
2. `StateDbConfig::default()` uses `DEFAULT_SCHEMA_EPOCH`.
3. Fresh daemon state is created at epoch 9.

Key code:

```rust
("0012_l4_sync_state", 9, L4_SYNC_STATE_MIGRATION_PATH),

pub const DEFAULT_SCHEMA_EPOCH: i64 = 9;
```

### Broken Path: Desktop State Inspection And Handshake

Entry point: `apps/desktop/src-tauri/src/main.rs`

Flow:
1. Desktop constant remains `EXPECTED_SCHEMA_EPOCH: i64 = 8`.
2. Existing state DB with epoch 9 returns `StorageState::SchemaMismatch`.
3. Daemon handshake reporting epoch 9 returns `HandshakeFailure::Incompatible`.
4. Full-control desktop validation cannot proceed.

Key code:

```rust
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

### Divergence Points

| # | Daemon Path | Desktop Path | Potential Impact |
|---|---|---|---|
| 1 | Epoch is 9 after `0012_l4_sync_state`. | Epoch constants remain 8. | Fresh daemon DBs are rejected by desktop. |
| 2 | L4 migration exists but Layer 4 behavior is out of scope for this QA. | Desktop treats schema epoch mismatch as incompatible. | Desktop cannot reach full control for L0-L3 QA. |
| 3 | Daemon bridge should negotiate against current daemon epoch. | `DaemonBridge` sends expected epoch 8. | Handshake fails before UI actions are available. |

---

## ROOT CAUSE HYPOTHESES

### H1: Desktop epoch constants were not advanced with daemon migration 0012 - LIKELIHOOD: HIGH

Description: The daemon schema epoch was advanced to 9 when `0012_l4_sync_state` was added, but desktop native bootstrap, query service, bridge, and native tests still encode epoch 8.

Supporting evidence: E2.1, E2.2, E2.3, E2.4, E2.5.

Contradicting evidence: none.

Verification steps:
1. Inspect the files listed in the affected component field.
2. Confirm every desktop/native expected epoch aligns with daemon `DEFAULT_SCHEMA_EPOCH`.
3. Run desktop native tests after the fix.

Fix scope: `apps/desktop/src-tauri/src/main.rs`, `apps/desktop/src-tauri/src/query_service.rs`, `apps/desktop/src-tauri/src/daemon_bridge.rs`, `apps/desktop/src-tauri/tests/*`.

### H2: Desktop should import or share the daemon schema epoch instead of duplicating constants - LIKELIHOOD: MEDIUM

Description: The immediate mismatch may be a symptom of duplicated constants across crates. A shared constant or fixture could prevent recurrence.

Supporting evidence: E2.1 through E2.5 show four separate desktop/native epoch definitions.

Contradicting evidence: Project conventions may intentionally duplicate constants across crate boundaries.

Verification steps:
1. Check workspace crate dependencies to see whether desktop can safely depend on the daemon/common schema constant.
2. Check whether tests already import daemon SQLite support, as `apps/desktop/src-tauri/tests/query_service.rs` references daemon SQLite test support.

Fix scope: same files as H1, possibly workspace crate exports if a shared constant is accepted.

---

## INVESTIGATION ROADMAP

### Step 1: Verify H1

Run:

```bash
rg -n "EXPECTED_SCHEMA_EPOCH|DEFAULT_SCHEMA_EPOCH_FOR_TEST|DEFAULT_SCHEMA_EPOCH" apps/desktop/src-tauri crates/daemon/src/db/sqlite.rs -g '*.rs'
```

If desktop constants still say 8 while daemon says 9, H1 is confirmed. Fix all desktop/native epoch expectations together.

### Step 2: Verify H2

Search for existing shared schema constant patterns:

```bash
rg -n "schema_epoch|DEFAULT_SCHEMA_EPOCH|EXPECTED_SCHEMA_EPOCH" crates apps/desktop -g '*.rs'
```

If duplication is broader than the affected files, consider a shared constant or test guard. If duplication is intentional, keep the fix minimal and update constants/tests in place.

---

## FIX CONSTRAINTS

1. Do not lower the daemon schema epoch from 9.
2. Do not edit `PRAGMA user_version` manually in QA state roots to bypass the mismatch.
3. Do not remove or disable `0012_l4_sync_state` just to make desktop pass.
4. Do not validate Layer 4 sync/RLS/shared visibility as part of the fix acceptance.
5. Preserve desktop rejection of genuinely incompatible schema epochs.

---

## ACCEPTANCE CRITERIA

1. The critical check reports daemon and desktop/Tauri epoch expectations aligned:

```bash
rg -n "pub const DEFAULT_SCHEMA_EPOCH|EXPECTED_SCHEMA_EPOCH|DEFAULT_SCHEMA_EPOCH" \
  crates/daemon/src/db/sqlite.rs \
  apps/desktop/src-tauri/src/main.rs \
  apps/desktop/src-tauri/src/query_service.rs \
  apps/desktop/src-tauri/src/daemon_bridge.rs \
  apps/desktop/src-tauri/tests -g '*.rs'
```

2. Desktop native gates pass:

```bash
cargo test -p procintel-desktop --test query_service
cargo test -p procintel-desktop --test daemon_bridge
cargo test -p procintel-desktop --test native_shell
```

3. Required Rust gates pass:

```bash
cargo fmt --check
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
```

4. A fresh daemon dry-run creates a state DB whose `PRAGMA user_version` matches the desktop expected epoch.

5. Machine-fork QA can resume at Environment Preflight and then Desktop App Validation without schema-mismatch blocking full control.

---

## FILE MAP

| File | Why It Matters |
|---|---|
| `crates/daemon/src/db/sqlite.rs` | Defines migration epochs and daemon `DEFAULT_SCHEMA_EPOCH`. |
| `apps/desktop/src-tauri/src/main.rs` | Defines desktop expected epoch, storage inspection, and daemon handshake rejection. |
| `apps/desktop/src-tauri/src/query_service.rs` | Opens read snapshots against an expected epoch. |
| `apps/desktop/src-tauri/src/daemon_bridge.rs` | Sends desktop expected schema epoch in daemon bridge handshake. |
| `apps/desktop/src-tauri/tests/native_shell.rs` | Native shell test expectation still uses epoch 8. |
| `apps/desktop/src-tauri/tests/support/native_shell.rs` | Support fixture expectation still uses epoch 8. |

---

## SELF-VERIFICATION

- Reproduction protocol uses exact commands and captured outputs.
- Evidence items E0.1 through E2.5 back the defect claim.
- File paths were read from the current checkout.
- Hypotheses are ranked and tied to evidence.
- Acceptance criteria use the same critical check that proved the defect.
