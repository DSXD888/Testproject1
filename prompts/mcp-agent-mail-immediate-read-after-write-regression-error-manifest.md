# MCP Agent Mail Immediate Read-After-Write Regression - QA Error Manifest

ULTRATHINK and be meticulous. This is a forensic debugging document.

## DEFECT FINGERPRINT

| Field | Value |
|-------|-------|
| **Type** | race_condition / integration_mismatch / wrong_behavior |
| **Severity** | high |
| **Reproducibility** | intermittent, observed 1 failing immediate read cycle and 1 passing immediate read cycle during 2026-05-06 verification |
| **Affected Component** | MCP Agent Mail live server and external source checkout `/Users/alinaaitova/Desktop/mcp_agent_mail_rust` |
| **Working Analogue** | delayed reads after automatic recovery and a later immediate read cycle both returned the written message |
| **Scope** | `send_message` can return success while immediate `search_messages`, `fetch_inbox`, `list_agents`, and resource reads see archive/SQLite drift and fail generically |
| **Environment** | macOS 15.5 arm64, `am 0.2.51`, server PID 69379 on `127.0.0.1:8765`, storage `/Users/alinaaitova/.mcp_agent_mail_git_mailbox_repo` |
| **Git State** | Testproject1 `main` at `96baf479f744f01fb7b39460c5597ae41db34372`, untracked `docs/session-log-2026-05-05.md`; Agent Mail source `main` at `dc5864396c0144163ecb51e74b74805cb7a0c6ab`, dirty |

---

## REPRODUCTION PROTOCOL

> Follow these steps EXACTLY. The defect is timing-dependent; if the immediate read succeeds, repeat the send-and-read cycle and preserve both pass and fail evidence.

### Prerequisites

- MCP Agent Mail server is running at `127.0.0.1:8765`.
- `am --version` reports `am 0.2.51`.
- Project key is `/Users/alinaaitova/Desktop/Testproject1`.
- Agent `MistyHorse` exists in the project.
- The live server is using `/Users/alinaaitova/.mcp_agent_mail_git_mailbox_repo/storage.sqlite3`.
- External source checkout exists at `/Users/alinaaitova/Desktop/mcp_agent_mail_rust`.

### Steps

**STEP 1**: Confirm live health is green before writing.
```json
{"tool":"mcp__mcp_agent_mail__.health_check","arguments":{}}
```
**EXPECTED**: Health is green and archive/SQLite counts are aligned.
**ACTUAL**: Before the failing cycle, health was green with `archive projects=3, agents=6, messages=6, db projects=3, agents=6, messages=6`.

**STEP 2**: Send a self-addressed message.
```json
{"tool":"mcp__mcp_agent_mail__.send_message","arguments":{"project_key":"/Users/alinaaitova/Desktop/Testproject1","sender_name":"MistyHorse","to":["MistyHorse"],"subject":"[agent-mail-patch-check] Fresh read/search verification","thread_id":"agent-mail-patch-check-2026-05-06-075640","importance":"normal","ack_required":true}}
```
**EXPECTED**: The message is durable and immediately visible through read/search/resource paths.
**ACTUAL**:
```json
{"payload":{"id":6,"thread_id":"agent-mail-patch-check-2026-05-06-075640","subject":"[agent-mail-patch-check] Fresh read/search verification","from":"MistyHorse","to":["MistyHorse"]},"count":1,"verified_sender":true}
```

**STEP 3**: Immediately search for the unique marker.
```json
{"tool":"mcp__mcp_agent_mail__.search_messages","arguments":{"project_key":"/Users/alinaaitova/Desktop/Testproject1","query":"agent-mail-patch-check-2026-05-06-075640","limit":10,"ranking":"recency"}}
```
**EXPECTED**: Message id `6` is returned.
**ACTUAL**:
```text
A database error occurred. This may be a transient issue - try again.
```

**STEP 4**: Immediately fetch the recipient inbox.
```json
{"tool":"mcp__mcp_agent_mail__.fetch_inbox","arguments":{"project_key":"/Users/alinaaitova/Desktop/Testproject1","agent_name":"MistyHorse","include_bodies":true,"limit":10}}
```
**EXPECTED**: Message id `6` is returned.
**ACTUAL**:
```text
A database error occurred. This may be a transient issue - try again.
```

**STEP 5**: Immediately read the inbox resource.
```text
resource://inbox/MistyHorse?project=/Users/alinaaitova/Desktop/Testproject1&include_bodies=true&limit=10
```
**EXPECTED**: Message id `6` is returned.
**ACTUAL**:
```text
SQLite error: reconstruct: refusing reconstruct sqlite target /var/folders/z3/784rpxcj23x3_0d8d88c_27h0000gn/T/agent-mail-resource-snapshot-69379_1778054212903181000_3/mailbox.sqlite3 because it traverses symlinked path /var
```

**STEP 6**: Confirm degraded health during the failure window.
```json
{"tool":"mcp__mcp_agent_mail__.health_check","arguments":{}}
```
**EXPECTED**: Health remains green or returns a structured, actionable degraded-read error.
**ACTUAL**:
```json
{"status":"error","health_level":"red","semantic_readiness":{"status":"fail","detail":"archive inventory is ahead of the sqlite index (archive projects=3, agents=7, messages=7, latest_id=7, db projects=3, agents=6, messages=6, max_id=7)"},"recovery":{"mode":"degraded_read_only","next_action":"Run `am doctor repair` to attempt automatic recovery"}}
```

### Verification - Confirm Defect Present

```bash
sed -n '1,220p' /Users/alinaaitova/.mcp_agent_mail_git_mailbox_repo/doctor/forensics/storage.sqlite3/reconstruct-20260506_075704_372/references/archive-drift-report.json
```
**OUTPUT**:
```json
{"archive_message_count":7,"db_message_count":6,"shared_message_count":6,"archive_only_ids":[6],"db_only_ids":[],"archive_projects":3,"db_projects":3,"archive_agents":7,"db_agents":6,"archive_latest_message_id":7,"db_max_message_id":7}
```

### Control - Working Analogue

After a 5 second wait, the same message became readable:
```json
{"tool":"mcp__mcp_agent_mail__.search_messages","arguments":{"project_key":"/Users/alinaaitova/Desktop/Testproject1","query":"agent-mail-patch-check-2026-05-06-075640","limit":10,"ranking":"recency"}}
```
**OUTPUT**:
```json
{"result":[{"id":6,"subject":"[agent-mail-patch-check] Fresh read/search verification","thread_id":"agent-mail-patch-check-2026-05-06-075640","from":"MistyHorse","to":["MistyHorse"]}]}
```

A later immediate cycle also passed for message id `7`:
```json
{"tool":"mcp__mcp_agent_mail__.search_messages","arguments":{"project_key":"/Users/alinaaitova/Desktop/Testproject1","query":"agent-mail-manifest-repro-2026-05-06-080506","limit":10,"ranking":"recency"}}
```
**OUTPUT**:
```json
{"result":[{"id":7,"subject":"[agent-mail-manifest-repro] Immediate read failure","thread_id":"agent-mail-manifest-repro-2026-05-06-080506","from":"MistyHorse","to":["MistyHorse"]}]}
```

---

## EVIDENCE BUNDLE

### E0 - Environment Evidence

**E0.1**: Timestamp.
```bash
date -u '+%Y-%m-%dT%H:%M:%SZ'
```
```text
2026-05-06T08:05:06Z
```

**E0.2**: OS.
```bash
sw_vers && uname -a
```
```text
ProductName: macOS
ProductVersion: 15.5
BuildVersion: 24F74
Darwin Mac.fritz.box 24.5.0 Darwin Kernel Version 24.5.0: Tue Apr 22 19:54:26 PDT 2025; root:xnu-11417.121.6~2/RELEASE_ARM64_T8112 arm64
```

**E0.3**: Project git state.
```bash
git rev-parse --abbrev-ref HEAD && git rev-parse HEAD && git status --short
```
```text
main
96baf479f744f01fb7b39460c5597ae41db34372
?? docs/session-log-2026-05-05.md
```

**E0.4**: Installed Agent Mail binary and process.
```bash
am --version
lsof -nP -iTCP:8765 -sTCP:LISTEN
```
```text
am 0.2.51
COMMAND   PID        USER   FD   TYPE  DEVICE SIZE/OFF NODE NAME
am      69379 alinaaitova    9u  IPv4  ...    0t0      TCP 127.0.0.1:8765 (LISTEN)
```

**E0.5**: External Agent Mail source state.
```bash
git -C /Users/alinaaitova/Desktop/mcp_agent_mail_rust rev-parse --abbrev-ref HEAD
git -C /Users/alinaaitova/Desktop/mcp_agent_mail_rust rev-parse HEAD
git -C /Users/alinaaitova/Desktop/mcp_agent_mail_rust status --short
```
```text
main
dc5864396c0144163ecb51e74b74805cb7a0c6ab
 M .beads/issues.jsonl
 M crates/mcp-agent-mail-db/src/pool.rs
 M crates/mcp-agent-mail-db/src/reconstruct.rs
 M crates/mcp-agent-mail-server/src/startup_checks.rs
 M crates/mcp-agent-mail-tools/src/identity.rs
 M crates/mcp-agent-mail-tools/src/lib.rs
 M crates/mcp-agent-mail-tools/src/resources.rs
```

### E1 - Patch Evidence

**E1.1**: Tracked patch file in Testproject1 is search-focused, not read-pool/resource focused.
```bash
git apply --stat agent-mail-search-fixes.patch
```
```text
crates/mcp-agent-mail-db/src/queries.rs        |  7 ++++++
crates/mcp-agent-mail-db/src/search_planner.rs |  8 ++++++
crates/mcp-agent-mail-db/src/search_service.rs | 30 ++++++++++++++++++++++--
crates/mcp-agent-mail-db/src/search_v3.rs      |  2 +-
crates/mcp-agent-mail-tools/src/products.rs    | 10 +++++++-
crates/mcp-agent-mail-tools/src/search.rs      | 24 ++++++++++++++-----
6 files changed, 70 insertions(+), 11 deletions(-)
```

**E1.2**: Actual external working-tree patch touches recovery/resource files.
```bash
git -C /Users/alinaaitova/Desktop/mcp_agent_mail_rust diff --stat
```
```text
.beads/issues.jsonl                                |   1 +
crates/mcp-agent-mail-db/src/pool.rs               |  15 +-
crates/mcp-agent-mail-db/src/reconstruct.rs        |  14 +-
crates/mcp-agent-mail-server/src/startup_checks.rs |   2 +-
crates/mcp-agent-mail-tools/src/identity.rs        |   2 +-
crates/mcp-agent-mail-tools/src/lib.rs             | 250 ++++++++++++++++++++-
crates/mcp-agent-mail-tools/src/resources.rs       | 131 +++++++++--
7 files changed, 380 insertions(+), 35 deletions(-)
```

**E1.3**: Patch adds structured degraded diagnostics, but live tools still returned generic errors during the failure window.
```bash
git -C /Users/alinaaitova/Desktop/mcp_agent_mail_rust diff -- crates/mcp-agent-mail-tools/src/lib.rs
```
```diff
+    pub(crate) fn archive_sqlite_degraded_error(message: &str) -> Option<McpError> {
+        Some(legacy_tool_error(
+            "DATABASE_DEGRADED",
+            "Agent Mail SQLite index is behind or unavailable relative to the Git archive; reads are temporarily degraded.",
+            true,
+            json!({"semantic_readiness":{"status":"fail"},"recovery":{"mode":"degraded_read_only"}}),
+        ))
+    }
```

**E1.4**: Patch canonicalizes snapshot directories but live resource reads still rejected `/var`.
```bash
git -C /Users/alinaaitova/Desktop/mcp_agent_mail_rust diff -- crates/mcp-agent-mail-tools/src/resources.rs
```
```diff
+    fn new_in(prefix: &str, base: &Path) -> std::io::Result<Self> {
+        let path = base.join(format!("{prefix}{pid}_{nanos}_{counter}"));
+        match std::fs::create_dir(&path) {
+            Ok(()) => {
+                let path = path.canonicalize()?;
+                return Ok(Self { path });
+            }
+            /* ... */
+        }
+    }
```

**E1.5**: Targeted test command for the new degraded diagnostic test did not complete because linking failed.
```bash
cargo test -p mcp-agent-mail-tools read_pool_setup_error_reports_archive_sqlite_degraded_state -- --exact
```
```text
error: linking with `cc` failed: exit status: 1
Undefined symbols for architecture arm64:
  "_OBJC_CLASS_$_MLComputePlan", referenced from:
      objc-class-ref in libort_sys-...rlib(model.mm.o)
  "_OBJC_CLASS_$_MLOptimizationHints", referenced from:
      objc-class-ref in libort_sys-...rlib(model.mm.o)
error: could not compile `mcp-agent-mail-tools` (lib test) due to 1 previous error
```

### E2 - Failure Evidence

**E2.1**: Immediate search after message id `6` failed generically.
```json
{"tool":"mcp__mcp_agent_mail__.search_messages","arguments":{"project_key":"/Users/alinaaitova/Desktop/Testproject1","query":"agent-mail-patch-check-2026-05-06-075640","limit":10,"ranking":"recency"}}
```
```text
A database error occurred. This may be a transient issue - try again.
```

**E2.2**: Immediate inbox fetch after message id `6` failed generically.
```json
{"tool":"mcp__mcp_agent_mail__.fetch_inbox","arguments":{"project_key":"/Users/alinaaitova/Desktop/Testproject1","agent_name":"MistyHorse","include_bodies":true,"limit":10}}
```
```text
A database error occurred. This may be a transient issue - try again.
```

**E2.3**: Immediate resource reads still hit the macOS `/var` symlink rejection.
```text
resource://inbox/MistyHorse?project=/Users/alinaaitova/Desktop/Testproject1&include_bodies=true&limit=10
```
```text
SQLite error: reconstruct: refusing reconstruct sqlite target /var/folders/.../mailbox.sqlite3 because it traverses symlinked path /var
```

**E2.4**: Health moved from green to archive-ahead red immediately after the successful write.
```json
{"tool":"mcp__mcp_agent_mail__.health_check","arguments":{}}
```
```json
{"status":"error","health_level":"red","semantic_readiness":{"status":"fail","detail":"archive inventory is ahead of the sqlite index (archive projects=3, agents=7, messages=7, latest_id=7, db projects=3, agents=6, messages=6, max_id=7)"}}
```

**E2.5**: `am doctor check` simultaneously reported parity failure but no repair action.
```bash
am doctor check
```
```text
Primary issue: [FAIL] Archive canonical data is not fully represented in the SQLite DB (agents archive=7 db=6, messages archive=7 db=6)
Next action: No database repair action is currently recommended.
```

**E2.6**: Automatic reconstruction forensic bundle captured archive-only message id `6`.
```bash
sed -n '1,220p' /Users/alinaaitova/.mcp_agent_mail_git_mailbox_repo/doctor/forensics/storage.sqlite3/reconstruct-20260506_075704_372/references/archive-drift-report.json
```
```json
{"archive_message_count":7,"db_message_count":6,"archive_only_ids":[6],"archive_agents":7,"db_agents":6,"archive_latest_message_id":7,"db_max_message_id":7}
```

### E3 - Control and Recovery Evidence

**E3.1**: Automatic recovery bundle existed for the failing window.
```bash
find /Users/alinaaitova/.mcp_agent_mail_git_mailbox_repo/doctor/forensics/storage.sqlite3/reconstruct-20260506_075704_372 -maxdepth 2 -type f -print
```
```text
.../summary.json
.../references/live-db-state.json
.../references/archive-drift-report.json
.../references/archive-drift.json
.../references/environment.json
.../sqlite/storage.sqlite3
.../manifest.json
```

**E3.2**: After a short delay, health returned green.
```json
{"tool":"mcp__mcp_agent_mail__.health_check","arguments":{}}
```
```json
{"status":"ok","health_level":"green","semantic_readiness":{"status":"ok","detail":"Archive and sqlite inventory are aligned enough for health_check: archive projects=3, agents=7, messages=7, db projects=3, agents=7, messages=7"}}
```

**E3.3**: After recovery, `fetch_inbox` returned message id `6`.
```json
{"tool":"mcp__mcp_agent_mail__.fetch_inbox","arguments":{"project_key":"/Users/alinaaitova/Desktop/Testproject1","agent_name":"MistyHorse","include_bodies":true,"limit":10}}
```
```json
[{"id":6,"thread_id":"agent-mail-patch-check-2026-05-06-075640","subject":"[agent-mail-patch-check] Fresh read/search verification","from":"MistyHorse","kind":"to"}]
```

**E3.4**: Later immediate message id `7` read/search/resource paths passed.
```json
{"tool":"mcp__mcp_agent_mail__.send_message","arguments":{"project_key":"/Users/alinaaitova/Desktop/Testproject1","sender_name":"MistyHorse","to":["MistyHorse"],"subject":"[agent-mail-manifest-repro] Immediate read failure","thread_id":"agent-mail-manifest-repro-2026-05-06-080506"}}
```
```json
{"payload":{"id":7,"thread_id":"agent-mail-manifest-repro-2026-05-06-080506","subject":"[agent-mail-manifest-repro] Immediate read failure"},"count":1,"verified_sender":true}
```

Immediate read:
```json
{"tool":"mcp__mcp_agent_mail__.search_messages","arguments":{"project_key":"/Users/alinaaitova/Desktop/Testproject1","query":"agent-mail-manifest-repro-2026-05-06-080506","limit":10,"ranking":"recency"}}
```
```json
{"result":[{"id":7,"subject":"[agent-mail-manifest-repro] Immediate read failure","thread_id":"agent-mail-manifest-repro-2026-05-06-080506","from":"MistyHorse","to":["MistyHorse"]}]}
```

---

## CODEPATH COMPARISON

### Working Path: Eventually Consistent / Recovered Reads

**Entry points**: `health_check`, `search_messages`, `fetch_inbox`, `resource://inbox`, `resource://thread`.

**Flow**:
1. Write lands in the Git archive.
2. Automatic reconstruct/reconcile catches up SQLite from archive.
3. Read/search/resource paths use aligned SQLite/search state and return the message.

**Key evidence**: E3.1, E3.2, E3.3, E3.4.

### Broken Path: Immediate Read During Archive-Ahead Window

**Entry points**: same read/search/resource paths immediately after `send_message`.

**Flow**:
1. `send_message` returns id `6` and `verified_sender=true`.
2. Archive contains id `6`, but SQLite has only 6 messages and 6 agents.
3. `search_messages` and `fetch_inbox` return only `DATABASE_ERROR`.
4. Resource reads try snapshot reconstruction under `/var/folders/...` and reject `/var` as symlinked.
5. Health says `degraded_read_only`; CLI doctor says parity failed but recommends no repair.

**Key evidence**: E2.1, E2.2, E2.3, E2.4, E2.5, E2.6.

### Divergence Points

| # | Working Path | Broken Path | Potential Impact |
|---|--------------|-------------|------------------|
| 1 | Read happens after automatic reconstruct | Read happens before SQLite catches up | Successful writes are temporarily unreadable |
| 2 | Resource snapshot path is usable | Snapshot target under `/var/folders` is rejected | Resource reads fail on normal macOS temp paths |
| 3 | Health and doctor agree after recovery | Health says reconstruct/repair; doctor says no repair | Operators receive conflicting recovery guidance |
| 4 | Source patch adds `DATABASE_DEGRADED` | Live tools emit generic `DATABASE_ERROR` | Patch may be undeployed or insufficiently wired |

---

## ROOT CAUSE HYPOTHESES

### H1: The patched source was not built/deployed into the live `am` server - LIKELIHOOD: HIGH

**Description**: The external checkout contains relevant source edits, but the live server still runs `am 0.2.51`, emitted old generic errors, emitted old `am doctor repair` guidance, and rejected `/var` exactly as before.

**Supporting evidence**: E0.4, E0.5, E1.2, E1.3, E1.4, E1.5, E2.1, E2.3, E2.4.
**Contradicting evidence**: Later reads recovered automatically (E3.2-E3.4), but that behavior also existed before and does not prove this source patch is deployed.

**Verification steps**:
1. Build the external checkout successfully and install/restart the server binary.
2. Confirm the running PID binary path and version/build hash.
3. Repeat the id `6` style immediate write-read test.

**Fix scope**: release/build/deploy path for `/Users/alinaaitova/Desktop/mcp_agent_mail_rust`.

### H2: The patch improves diagnostics/snapshot setup but still allows `send_message` to return before DB/search visibility is satisfied - LIKELIHOOD: HIGH

**Description**: The core user contract is read-after-write visibility. The patch may handle degraded read setup, but it does not obviously change the write success contract or force bounded wait/retry until the SQLite/read index has the new agent/message.

**Supporting evidence**: E2.1, E2.2, E2.4, E2.6, E3.3.
**Contradicting evidence**: Message id `7` passed immediately (E3.4), so the failure is timing-dependent rather than guaranteed.

**Verification steps**:
1. Add a test that calls `send_message`, then immediately calls `search_messages` and `fetch_inbox` without sleeping.
2. Run it in a loop under write-back queue pressure.
3. Fail on any generic DB error or missing just-written id.

**Fix scope**: `send_message` durability contract, write-back queue flush/read admission, `get_read_db_pool`, `open_read_db_pool`, `fetch_inbox`, `search_messages`.

### H3: Snapshot canonicalization is only in source or only canonicalizes the temp directory, while the DB reconstruct validator still rejects symlinked parents - LIKELIHOOD: MEDIUM

**Description**: The source patch canonicalizes `ResourceSnapshotDirGuard` and `ReadSnapshotDirGuard`, but live resource reads still rejected `/var`. If deployed, the deeper `validate_sqlite_target_path` / `reconstruct_from_archive` path may still validate the noncanonical target or a different snapshot root.

**Supporting evidence**: E1.4, E2.3.
**Contradicting evidence**: A later resource read for id `7` passed (E3.4), so the rejection may require archive-ahead snapshot reconstruction.

**Verification steps**:
1. Force archive-ahead state in an isolated storage root.
2. Set `TMPDIR=/var/folders/...` or a symlinked temp base.
3. Call `resource://inbox` and assert no `traverses symlinked path /var` error.

**Fix scope**: `ResourceSnapshotDirGuard`, `ReadSnapshotDirGuard`, `validate_sqlite_target_path`, `reconstruct_from_archive`.

### H4: Health and doctor recovery guidance remain inconsistent - LIKELIHOOD: MEDIUM

**Description**: During the failure window, health reported degraded read-only with repair guidance, while `am doctor check` reported archive parity failure and no repair action.

**Supporting evidence**: E2.4, E2.5.
**Contradicting evidence**: Source patch edits `startup_checks.rs` and `identity.rs` guidance text (E1.2), but live output still showed old text.

**Verification steps**:
1. Reproduce archive-ahead in an isolated storage root.
2. Compare `health_check`, `am doctor check`, and resource/tool errors.
3. Assert all point to reconstruct or a bounded automatic recovery state.

**Fix scope**: `startup_checks.rs`, `identity.rs`, CLI doctor archive parity next-action logic.

---

## INVESTIGATION ROADMAP

### Step 1: Verify Deployment State

Run:
```bash
am --version
which am
codesign -dv "$(which am)" 2>&1 || true
git -C /Users/alinaaitova/Desktop/mcp_agent_mail_rust diff --stat
cargo test -p mcp-agent-mail-tools read_pool_setup_error_reports_archive_sqlite_degraded_state -- --exact
```

**If confirmed undeployed** -> fix the linker issue, install/restart the patched binary, then rerun the reproduction protocol.
**If rejected** -> proceed to Step 2.

### Step 2: Verify Write-to-Read Contract

Create an integration test that sends N self-addressed messages and immediately calls `search_messages`, `fetch_inbox`, `resource://inbox`, and `resource://thread` for each id.

**If any iteration fails** -> fix write durability/read admission so successful write responses imply immediate read visibility or return a structured degraded state.
**If all iterations pass** -> proceed to Step 3.

### Step 3: Verify Snapshot Path Handling Under Archive-Ahead

Force archive-ahead and symlinked temp root in a test; call resource reads while the read path must reconstruct from archive.

**If `/var` rejection appears** -> fix canonical containment validation for snapshot targets.
**If rejected** -> proceed to Step 4.

### Step 4: Verify Diagnostics Consistency

Compare tool errors, resource errors, `health_check`, and `am doctor check` in the same degraded state.

**If inconsistent** -> align all outputs around `DATABASE_DEGRADED` and `am doctor reconstruct`.
**If rejected** -> re-examine queue/recovery races and whether the failure is caused by a stale connection pool.

---

## FIX CONSTRAINTS

1. Do not delete or rewrite Agent Mail archive data.
2. Do not weaken symlink/path traversal protection for caller-controlled SQLite targets.
3. Preserve successful write-side behavior and ids.
4. Preserve automatic recovery, but do not rely on users waiting blindly after a successful write.
5. Do not return generic `DATABASE_ERROR` when archive/SQLite drift is known.
6. Keep `am doctor check`, `health_check`, tool errors, and resource errors consistent.
7. Ensure tests can run on macOS without requiring destructive changes to live storage.

---

## ACCEPTANCE CRITERIA

### Functional

- [ ] A successful `send_message` is immediately visible through `search_messages`.
- [ ] The same message is immediately visible through `fetch_inbox`.
- [ ] `resource://inbox/{agent}` and `resource://thread/{thread}` immediately return the same message.
- [ ] If immediate visibility cannot be provided, all read paths return structured `DATABASE_DEGRADED` with archive/db counts and a correct reconstruct next action.
- [ ] macOS resource snapshots never fail with `traverses symlinked path /var` when the snapshot root is internally created and canonical containment holds.

### Evidence-Based

- [ ] Re-run the exact id `6` style smoke cycle with a new marker 20 times; zero generic database errors.
- [ ] During each cycle, `health_check` does not enter red archive-ahead after `send_message` has returned success.
- [ ] `am doctor check` does not report archive parity failure with `Next action: No database repair action is currently recommended`.
- [ ] If an archive-ahead state is deliberately injected, `archive-drift-report.json` may show archive-only ids, but tool/resource errors must be structured and actionable.

### Regression

- [ ] `register_agent`, `send_message`, `acknowledge_message`, `file_reservation_paths`, and `release_file_reservations` still pass.
- [ ] Existing symlink rejection tests for caller-controlled SQLite destinations still pass.
- [ ] Search pagination/offset/recency behavior from `agent-mail-search-fixes.patch` still passes.
- [ ] `git diff --check` remains clean.

### Test

- [ ] Add a write-read integration test: send, immediate search, immediate inbox, immediate inbox resource, immediate thread resource.
- [ ] Add an archive-ahead degraded-state test for `search_messages` and `fetch_inbox`.
- [ ] Add an archive-ahead resource snapshot test with a symlinked temp base.
- [ ] Add a doctor/health next-action consistency test.
- [ ] Fix the current macOS linker failure or provide a documented test target that avoids `MLComputePlan`/`MLOptimizationHints` linkage for non-ML Agent Mail tests.

---

## FILE MAP

| File | Role | Lines Referenced |
|------|------|------------------|
| `prompts/mcp-agent-mail-immediate-read-after-write-regression-error-manifest.md` | This manifest | all |
| `agent-mail-search-fixes.patch` | Tracked search/pagination patch clue | patch lines 1-211 |
| `prompts/mcp-agent-mail-read-search-database-errors-fix-handoff.md` | Prior implementation handoff and expected fix shape | lines 120-190, 390-470, 515-590 |
| `docs/handoff-2026-05-05-mcp-agent-mail-db-recovery.md` | Post-incident recovery evidence | lines 58-104 |
| `/Users/alinaaitova/Desktop/mcp_agent_mail_rust/crates/mcp-agent-mail-tools/src/lib.rs` | Actual external patch: degraded error mapping and read snapshot changes | diff |
| `/Users/alinaaitova/Desktop/mcp_agent_mail_rust/crates/mcp-agent-mail-tools/src/resources.rs` | Actual external patch: resource snapshot canonicalization and resource error mapping | diff |
| `/Users/alinaaitova/Desktop/mcp_agent_mail_rust/crates/mcp-agent-mail-db/src/pool.rs` | Actual external patch: canonicalized test roots; path validator still central | diff |
| `/Users/alinaaitova/Desktop/mcp_agent_mail_rust/crates/mcp-agent-mail-db/src/reconstruct.rs` | Actual external patch: canonicalized symlink tests; reconstruct still calls validator | diff |
| `/Users/alinaaitova/Desktop/mcp_agent_mail_rust/crates/mcp-agent-mail-server/src/startup_checks.rs` | Actual external patch: health recovery guidance text | diff |
| `/Users/alinaaitova/Desktop/mcp_agent_mail_rust/crates/mcp-agent-mail-tools/src/identity.rs` | Actual external patch: recovery status guidance text | diff |
| `/Users/alinaaitova/.mcp_agent_mail_git_mailbox_repo/doctor/forensics/storage.sqlite3/reconstruct-20260506_075704_372/references/archive-drift-report.json` | Forensic proof of archive-only id `6` during failure | all |
