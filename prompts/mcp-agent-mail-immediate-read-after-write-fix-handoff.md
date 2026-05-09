# MCP Agent Mail Immediate Read-After-Write Regression — Implementation Handoff

ULTRATHINK and be meticulous. This is a complete fix handoff for an external bug whose surface manifestations are documented in `prompts/mcp-agent-mail-immediate-read-after-write-regression-error-manifest.md`. Live reproduction on `2026-05-06` proved the defect is **deeper and more persistent** than that manifest claims — read this entire document before opening the source.

---

## MISSION

Fix the MCP Agent Mail bug in which `send_message` returns success while every immediate read path (`search_messages`, `fetch_inbox`, `resource://inbox`, `resource://thread`) fails with a generic `"A database error occurred. This may be a transient issue - try again."`, and the resulting archive/SQLite drift is **not** automatically repaired within any bounded time window. The fix lives in the external `mcp_agent_mail_rust` Rust workspace at `/Users/zacfarrell/Desktop/mcp_agent_mail_rust`. It must ship a new build of the `am` binary, not just edit source.

The defect affects the entire host: a single `send_message` from any pane, in any project, on `127.0.0.1:8765` is sufficient to break read-after-write for every project on the same Mac until reconcile fires (which may not happen automatically). Treat this as a host-global outage class, not a per-project bug.

Out of scope: deleting smoke-test mailbox data, rewriting Agent Mail's archive layout, redesigning the commit coalescer, or modifying the `Testproject1` codebase.

---

## CRITICAL CONTEXT BEFORE YOU START

Five things that change how you should approach this. They are not in the original manifest:

1. **The patch in `/Users/zacfarrell/Desktop/mcp_agent_mail_rust` is uncommitted and was never built or installed.** `git status` shows seven dirty files on top of commit `dc586439`; `am --version` still reports `0.2.51` and the live binary at `/Users/zacfarrell/.local/bin/am` (PID 69379, LaunchAgent `com.agent-mail`) was last touched before the patch existed. Every "ACTUAL" line in the manifest reflects the unpatched binary, not the in-flight fix. Your first verification step is to confirm the live binary's build hash diverges from the patched source — and after your fix, to ship a new `0.2.x` and restart the daemon.

2. **The defect is not "intermittent" in the way the manifest implies.** The manifest reports 1 failing immediate-read cycle and 1 passing immediate-read cycle. Live reproduction on `2026-05-06T08:16:46Z` failed on the next write *and stayed failing for 2+ minutes* — health remained red, search remained broken, and message id 8 stayed permanently archive-only. The original "passing" cycle for id 7 likely caught a window where the readiness drift detector or integrity guard had recently fired; without that, the drift is **unbounded**. Plan for the worst case, not the lucky case.

3. **Two distinct read-side failure modes share one verdict surface.** The manifest captures both, but conflates them:
   - **Mode A** (`search_messages` / `fetch_inbox`): the read pool's setup error is collapsed to `DbError::Sqlite(…)` and surfaces as the generic `"A database error occurred"`. No structured diagnostic.
   - **Mode B** (`resource://inbox` / `resource://thread`): the snapshot reconstruction path *is* attempted, lands on `/var/folders/.../mailbox.sqlite3`, and `validate_sqlite_target_path` rejects it because `/var → /private/var` is a symlink on every macOS host.
   - Which mode you observe depends on whether the verdict engine routes the read to the snapshot path before failing. Both modes indicate the same upstream problem (the read pool cannot fall back gracefully) but their fixes are separable.

4. **The daemon's "fresh durability probe" is not just observation — it is an active suppressor of the bug.** This is the single most important correction to the manifest. Read `crates/mcp-agent-mail-db/src/queries.rs::verify_message_recipients_visible_after_commit` (line 1769) and the sister function `verify_agent_visible_after_commit` (line 1506). When the **fresh** SQLite handle cannot see the just-committed row, the function deliberately retries on the **pooled** handle (which sees the writer's own state including its in-flight transaction), logs `WARN fresh durability probe missed committed message visibility; pooled runtime handle confirmed rows`, and **returns `Outcome::Ok(())`** anyway. So `send_message` returns success based on a handle that is structurally guaranteed to see the row, even when every other connection on the host cannot. The fix lives at this exact site, not at the response builder.

5. **`cargo test` for `mcp-agent-mail-tools` currently fails on this Mac with a linker error.** Building or even running unit tests requires either skipping the ONNX/`libort_sys` dependency that pulls `_OBJC_CLASS_$_MLComputePlan` and `_OBJC_CLASS_$_MLOptimizationHints`, or adding the missing `CoreML` framework link. You will need to fix this before you can validate any change with `cargo test`. The exact failing command from the manifest:

```bash
cargo test -p mcp-agent-mail-tools read_pool_setup_error_reports_archive_sqlite_degraded_state -- --exact
# error: linking with `cc` failed: exit status: 1
# Undefined symbols for architecture arm64:
#   "_OBJC_CLASS_$_MLComputePlan", "_OBJC_CLASS_$_MLOptimizationHints"
```

6. **The bug is conditional on a custom-SQLite-fork feature, not vanilla SQLite.** The project ships its own SQLite fork ("FrankenSQLite") with optional MVCC concurrent transactions. The flag is `FSQLITE_CONCURRENT_MODE=true` (see `crates/mcp-agent-mail-db/src/queries.rs:1309-1346`). When enabled, `create_message_with_recipients_tx` runs `BEGIN CONCURRENT → INSERTs → COMMIT` (queries.rs:5478-5482) which uses snapshot isolation — and snapshot isolation is what makes the writer-pool view diverge from fresh-handle views during the visibility window. Confirm the live binary has MVCC enabled (`launchctl print gui/$UID/com.agent-mail | grep FSQLITE_CONCURRENT_MODE`, or check the LaunchAgent plist at `~/Library/LaunchAgents/com.agent-mail.plist`); if the bug doesn't reproduce on a host without MVCC, the fix is still correct, but the urgency is conditional.

7. **Patch `br-nw23y` is marked CLOSED but never deployed.** The bead ticket `br-nw23y` ("Fix archive-ahead read/search/resource recovery diagnostics") was closed by the patch author at `2026-05-05T23:27:36Z` with `close_reason="Completed archive-ahead read/resource diagnostics and macOS snapshot canonicalization"`. The working tree changes ARE the implementation of that bead — but they were never committed or built. Reopen or supersede the bead if you take this work on; do not assume "closed" means "shipped."

---

## FIRST 5 MINUTES — EMPIRICAL SIGNAL CAPTURE (DO THIS BEFORE ANY SOURCE WORK)

Before opening Cargo or grep, gather the live signals so you know what *actually* fails on the target host:

```bash
# Confirm the running binary, version, and PID
am --version && which am && lsof -nP -iTCP:8765 -sTCP:LISTEN | tail -1

# Confirm the live SQLite is in DELETE mode (MVCC + DELETE is the bug-prone combo)
sqlite3 -readonly ~/.mcp_agent_mail_git_mailbox_repo/storage.sqlite3 'PRAGMA journal_mode;'
# expect: delete

# Confirm whether MVCC is enabled in the running daemon
ps -ww $(pgrep -f 'am serve-http') | head
launchctl print gui/$UID/com.agent-mail | grep -i FSQLITE 2>/dev/null

# Reproduce by running steps in REPRODUCTION RECIPE (below) and capture:
#   - the message id returned by send_message
#   - the timestamps of the WARN line in stderr.log
#   - the archive-vs-db delta in health_check
date -u '+%Y-%m-%dT%H:%M:%SZ'
# ... do reproduction ...
grep -E "(fresh durability|archive inventory is ahead|reconstructed)" \
     ~/Library/Logs/agent-mail/stderr.log | tail -10

# Confirm the patch state
git -C /Users/zacfarrell/Desktop/mcp_agent_mail_rust status --short
git -C /Users/zacfarrell/Desktop/mcp_agent_mail_rust diff --stat
```

Outcomes you might see:

- `journal_mode=delete` + WARN appears + archive>db: bug confirmed, proceed to Slice D.
- `journal_mode=wal` + WARN still appears: the bug is NOT just a DELETE-mode artifact, MVCC alone is enough — Slice D is still correct.
- `journal_mode=wal` + WARN gone + archive==db: the bug may have already been worked around at deploy time. Document the deploy state and verify Slices A/B/C/E are still desirable.
- WARN absent on multiple back-to-back send_message attempts: you may not be hitting the MVCC code path. Confirm `FSQLITE_CONCURRENT_MODE` and re-run under load.

---

## REPRODUCTION RECIPE (PROVEN ON THIS HOST)

This recipe was executed on `2026-05-06T08:16Z` against the live `am 0.2.51` server. It is reliable — it failed on the very next write after a green health check, and the resulting drift persisted indefinitely.

### Prerequisites (already true on this Mac)

- LaunchAgent `com.agent-mail` running on `127.0.0.1:8765`, PID visible via `lsof -nP -iTCP:8765 -sTCP:LISTEN`.
- `am --version` returns `am 0.2.51`.
- Storage root `/Users/zacfarrell/.mcp_agent_mail_git_mailbox_repo/` exists and contains projects.
- Agent `MistyHorse` is registered in project `/Users/zacfarrell/Desktop/Testproject1` (verify with `list_agents`).

### Step 1 — confirm baseline green

```json
{"tool":"mcp__mcp_agent_mail__.health_check","arguments":{}}
```

Expect `health_level:"green"` and `archive == db` counts. If not green, wait for or trigger reconcile before proceeding.

### Step 2 — send a self-addressed message with a unique marker

```json
{"tool":"mcp__mcp_agent_mail__.send_message",
 "arguments":{
   "project_key":"/Users/zacfarrell/Desktop/Testproject1",
   "sender_name":"MistyHorse",
   "to":["MistyHorse"],
   "subject":"[am-fix-verify] cycle <N>",
   "body_md":"Marker: am-fix-verify-<unique-utc-stamp>",
   "thread_id":"am-fix-verify-<unique-utc-stamp>",
   "importance":"normal",
   "ack_required":false}}
```

Expect `payload.id` in the response and a new file under `~/.mcp_agent_mail_git_mailbox_repo/projects/users-zacfarrell-desktop-testproject1/messages/2026/05/`.

### Step 3 — three immediate reads, in parallel

```json
{"tool":"mcp__mcp_agent_mail__.search_messages","arguments":{"project_key":"/Users/zacfarrell/Desktop/Testproject1","query":"<marker>","limit":10,"ranking":"recency"}}
{"tool":"mcp__mcp_agent_mail__.fetch_inbox","arguments":{"project_key":"/Users/zacfarrell/Desktop/Testproject1","agent_name":"MistyHorse","include_bodies":true,"limit":10}}
ReadMcpResource: server="mcp-agent-mail" uri="resource://inbox/MistyHorse?project=/Users/zacfarrell/Desktop/Testproject1&include_bodies=true&limit=10"
{"tool":"mcp__mcp_agent_mail__.health_check","arguments":{}}
```

On the unpatched binary, expect:
- All three reads return `"A database error occurred. This may be a transient issue - try again."` (or, for the resource path under some routing decisions, `"refusing reconstruct sqlite target /var/folders/.../mailbox.sqlite3 because it traverses symlinked path /var"`).
- `health_check` returns `health_level:"red"`, `semantic_readiness.detail` reads `archive inventory is ahead of the sqlite index (archive ... messages=N+1, db ... messages=N)`, `recovery.next_action` reads `"Run \`am doctor repair\` to attempt automatic recovery"`.

### Step 4 — confirm persistence

Wait 30 seconds, repeat `health_check` and `search_messages`. Expect both still failing. Wait 2 minutes — same. The drift will not auto-clear unless the periodic integrity guard or another readiness check happens to fire and detect the ahead state. The auto-recovery message you should *not* see in `~/Library/Logs/agent-mail/stderr.log` is `INFO database reconstruction from archive complete stats=reconstructed ... messages=N+1`. If you do see it, you got lucky — re-run.

### Step 5 — confirm the daemon already detected it

```bash
grep -E "(fresh durability probe missed|message_id=<N+1>|messages=<N+1>)" ~/Library/Logs/agent-mail/stderr.log | tail -5
```

Expect a WARN within ~50 ms of your `send_message` of the form:

```text
WARN fresh durability probe missed committed message visibility;
     pooled runtime handle confirmed rows project_id=1 message_id=<N+1>
     fresh_error=Internal error: message row not visible after commit for message_id=<N+1> project_id=1
```

This is the daemon admitting the bug in real time.

### Step 6 — force recovery and reset for the next cycle

```bash
am doctor reconstruct --dry-run --json
am doctor reconstruct --yes --json
# OR: stop the daemon, run reconstruct, restart
```

After reconstruct, repeat from Step 1.

---

## LIVE EVIDENCE BUNDLE (CAPTURED 2026-05-06)

You can corroborate every claim below against files already on this host.

### Environment

```bash
$ am --version
am 0.2.51
$ which am
/Users/zacfarrell/.local/bin/am
$ lsof -nP -iTCP:8765 -sTCP:LISTEN | tail -1
am      69379 zacfarrell    9u  IPv4  ...        0t0  TCP 127.0.0.1:8765 (LISTEN)
$ uname -a
Darwin Mac.fritz.box 24.5.0 Darwin Kernel Version 24.5.0: Tue Apr 22 19:54:26 PDT 2025;
                                root:xnu-11417.121.6~2/RELEASE_ARM64_T8112 arm64
```

### External source state (the patch is uncommitted)

```bash
$ git -C /Users/zacfarrell/Desktop/mcp_agent_mail_rust rev-parse HEAD
dc5864396c0144163ecb51e74b74805cb7a0c6ab
$ git -C /Users/zacfarrell/Desktop/mcp_agent_mail_rust status --short
 M .beads/issues.jsonl
 M crates/mcp-agent-mail-db/src/pool.rs
 M crates/mcp-agent-mail-db/src/reconstruct.rs
 M crates/mcp-agent-mail-server/src/startup_checks.rs
 M crates/mcp-agent-mail-tools/src/identity.rs
 M crates/mcp-agent-mail-tools/src/lib.rs
 M crates/mcp-agent-mail-tools/src/resources.rs
$ git -C /Users/zacfarrell/Desktop/mcp_agent_mail_rust diff --stat
 .beads/issues.jsonl                                |   1 +
 crates/mcp-agent-mail-db/src/pool.rs               |  15 +-
 crates/mcp-agent-mail-db/src/reconstruct.rs        |  14 +-
 crates/mcp-agent-mail-server/src/startup_checks.rs |   2 +-
 crates/mcp-agent-mail-tools/src/identity.rs        |   2 +-
 crates/mcp-agent-mail-tools/src/lib.rs             | 250 ++++++++++++++++++++-
 crates/mcp-agent-mail-tools/src/resources.rs       | 131 +++++++++--
 7 files changed, 380 insertions(+), 35 deletions(-)
```

### Storage layout (one SQLite, three projects — proves global blast radius)

```bash
$ ls /Users/zacfarrell/.mcp_agent_mail_git_mailbox_repo/projects/
ahead-project
users-zacfarrell-desktop-mcp-agent-mail-rust
users-zacfarrell-desktop-testproject1

$ ls /Users/zacfarrell/.mcp_agent_mail_git_mailbox_repo/projects/users-zacfarrell-desktop-testproject1/messages/2026/05/
2026-05-05T21-04-24Z__agent-mail-smoke-self-test-message__1.md
2026-05-05T21-04-27Z__contact-request-from-bronzegoose__2.md
2026-05-05T21-04-27Z__smoke-test-hello-from-bronzegoose__3.md
2026-05-06T07-56-45Z__agent-mail-patch-check-fresh-read-search-verification__6.md
2026-05-06T08-05-13Z__agent-mail-manifest-repro-immediate-read-failure__7.md
2026-05-06T08-16-46Z__am-raw-race-repro-cycle-1-immediate-read__8.md
```

Note `__8.md` — the message I sent during reproduction. It exists in the archive but was permanently invisible to SQLite reads at the time of writing this document.

### Live failure capture (msg id=8)

```text
2026-05-06T08:16:46.518466Z  send_message returned id=8, count=1, verified_sender=false
2026-05-06T08:16:46.560069Z  WARN fresh durability probe missed committed message visibility;
                              pooled runtime handle confirmed rows project_id=1 message_id=8
                              fresh_error=Internal error: message row not visible after commit
                              for message_id=8 project_id=1
2026-05-06T08:16:50          search_messages → "A database error occurred. ..."
2026-05-06T08:16:50          fetch_inbox     → "A database error occurred. ..."
2026-05-06T08:16:50          resource://inbox/MistyHorse → MCP error -32000: same generic error
2026-05-06T08:16:50          health_check    → red, archive=8/db=7, next_action=`am doctor repair`
2026-05-06T08:17:14          health_check    → still red, same drift (no reconcile fired)
2026-05-06T08:18:53          health_check    → still red, integrity guard ran (checks_total=134),
                                                file passed integrity, no reconcile fired
```

### Health snapshot during the failure

```json
{
  "status":"error","health_level":"red",
  "semantic_readiness":{"status":"fail","detail":"archive inventory is ahead of the sqlite index (archive projects=3, agents=7, messages=8, latest_id=8, db projects=3, agents=7, messages=7, max_id=7)"},
  "queues":{
    "wbq":{"depth":1,"capacity":8192,"latency_p99_ms":155,"enqueued_total":39,"drained_total":39},
    "commit_coalescer":{"pending_requests":0,"latency_p50_ms":1289,"latency_p95_ms":1289,"latency_p99_ms":1289,"enqueued_total":41,"drained_total":41,"sync_fallbacks_total":0}
  },
  "recovery":{"mode":"degraded_read_only","owner":"none","next_action":"Run `am doctor repair` to attempt automatic recovery","bundle_path":"/Users/zacfarrell/.mcp_agent_mail_git_mailbox_repo/doctor/forensics/storage.sqlite3/reconstruct-20260506_075704_372","recovery_lock_active":false}
}
```

Note `commit_coalescer.latency_p50_ms = 1289`. Average commit lag is over a second. That's the size of the read-after-write race window.

### Forensic bundle (manifest-window failure)

```bash
$ ls /Users/zacfarrell/.mcp_agent_mail_git_mailbox_repo/doctor/forensics/storage.sqlite3/
reconstruct-20260505_210708_464   # original manifest incident
reconstruct-20260506_075622_932   # 07:56 reconcile (msg 6)
reconstruct-20260506_075704_372   # 07:57 reconcile (msg 6 retry)

$ cat /Users/zacfarrell/.mcp_agent_mail_git_mailbox_repo/doctor/forensics/storage.sqlite3/reconstruct-20260506_075704_372/references/archive-drift-report.json
{"archive_message_count":7,"db_message_count":6,"shared_message_count":6,
 "archive_only_ids":[6],"db_only_ids":[],"identity_mismatches":[],
 "archive_projects":3,"db_projects":3,"archive_agents":7,"db_agents":6,
 "archive_latest_message_id":7,"db_max_message_id":7}
```

---

## ARCHITECTURE OVERVIEW (READ THIS BEFORE OPENING SOURCE)

You will save hours by understanding the layering before grepping. Agent Mail is a single LaunchAgent serving HTTP/MCP on `127.0.0.1:8765`. One daemon per host, one storage root per host, multiple projects share the same SQLite. The Rust workspace splits responsibilities across these crates:

```
mcp-agent-mail-server/   ← HTTP/MCP transport, JSON-RPC dispatch, startup checks
mcp-agent-mail-tools/    ← MCP tool handlers (send_message, fetch_inbox, search_messages)
                            and resource renderers (resource://inbox, resource://thread)
mcp-agent-mail-db/       ← SQLite pool, schema, queries, durability probe, reconstruct,
                            mailbox verdict engine, validate_sqlite_target_path
mcp-agent-mail-storage/  ← Git archive write path, write-back queue (wbq), CommitCoalescer
mcp-agent-mail-cli/      ← `am doctor`, `am service`, repair/reconstruct/fix flows
mcp-agent-mail-core/     ← shared config, disk pressure, identity primitives
```

### Storage layers

```
~/.mcp_agent_mail_git_mailbox_repo/
├── projects/<slug>/messages/YYYY/MM/*.md     ← canonical Git archive (durable, append-only)
├── projects/<slug>/agents/<name>/profile.json
├── storage.sqlite3                            ← derived index (online query state)
├── search_index/                              ← Tantivy index (separate derived view)
├── doctor/forensics/storage.sqlite3/...       ← capture bundles for every reconstruct
└── .git/                                      ← archive is a Git repo for audit/sync
```

### Write path (what `send_message` does — verified anchors)

```
JSON-RPC send_message
  └── messaging.rs:1427  pub async fn send_message(...)   [tools crate]
        ├── canonical Markdown rendered + WRITTEN to projects/.../messages/YYYY/MM/N.md
        ├── recipient inbox copies WRITTEN to per-agent inbox folders
        ├── INSERT into messages + message_recipients via queries.rs paths
        │     ├── storage::wbq_enqueue → drained by writer task             [storage/lib.rs:401]
        │     │   (WBQ_FLUSH_INTERVAL_MS = 100ms, see storage/lib.rs:243)
        │     └── storage::CommitCoalescer batches → flush_sync drives COMMIT
        │         (CommitCoalescer at storage/lib.rs:1724,
        │          singleton via get_commit_coalescer at storage/lib.rs:3245)
        ├── verify_message_recipients_visible_after_commit                    [queries.rs:1769]
        │     ├── try fresh handle (durability_probe_query)                   [queries.rs:1426]
        │     ├── if fresh handle MISSES the row, retry on POOLED handle     [queries.rs:1786]
        │     ├── if pooled handle CONFIRMS the row, log WARN and RETURN OK  [queries.rs:1808]
        │     │   ← THIS IS THE FALSE-SUCCESS GATE; FIX HERE
        │     └── only return Err if BOTH fresh and pooled probes miss
        └── return {id, count:1, verified_sender}
```

The probe is *intended* to gate the response. It fails to gate because the pooled fallback sees the writer's own pre-publication state. The `WARN ... pooled runtime handle confirmed rows` log line is the daemon admitting it just suppressed the failure.

### Read path (what `search_messages` / `fetch_inbox` / resources do — verified anchors)

```
JSON-RPC search_messages / fetch_inbox
  └── search.rs:713 pub async fn search_messages(...)        [tools]
      messaging.rs:3020 pub async fn fetch_inbox(...)        [tools]
        └── tool_util::get_read_db_pool()                    [lib.rs:869]
              └── tool_util::open_read_db_pool()             [lib.rs:774]
                    ├── compute mailbox verdict via mailbox_verdict.rs
                    │     ├── scan_archive_message_inventory(storage_root)
                    │     └── compare against SQLite inventory
                    ├── if Healthy → return Ok(None) → fall back to live get_db_pool
                    ├── if ArchiveAhead → ReadSnapshotDirGuard::new(...)    [lib.rs:843]
                    │     ├── creates dir under std::env::temp_dir() = /var/folders/.../T/
                    │     ├── join("mailbox.sqlite3") → snapshot target
                    │     ├── reconstruct_from_archive_with_salvage(...)
                    │     │     └── validate_sqlite_target_path(snapshot)   [pool.rs:3718]
                    │     │           └── walks every component, rejects any symlink
                    │     │             ← /var IS a symlink to /private/var → FAILS HERE
                    │     └── create_pool against snapshot
                    └── on any setup error → read_pool_setup_error_to_mcp_error(msg)  [lib.rs:375]
                          ├── (patched) try archive_sqlite_degraded_error(msg)        [lib.rs:387]
                          │   → returns DATABASE_DEGRADED with archive/db counts
                          └── else: DbError::Sqlite(msg) → "A database error occurred..."

JSON-RPC resource://inbox/{agent} | resource://thread/{thread}
  └── resources.rs::open_resource_read_pool                  [resources.rs:335]
        ├── ResourceSnapshotDirGuard::new(...)               [resources.rs:403]
        ├── reconstruct_from_archive[_with_salvage]
        ├── create_pool against snapshot
        └── on error → resource_sync_db_error_to_mcp_error(msg)  [resources.rs:85]
              └── (patched) try archive_sqlite_degraded_error(msg)
```

Both error mappers exist; both currently strip the diagnostic. The patch wires `archive_sqlite_degraded_error` into both at lib.rs:379 and resources.rs:89, but neither is deployed. The patch does **not** wire it into `db_error_to_mcp_error` (lib.rs:190), which is the mapper used by *non-snapshot* DB errors — so even with the patch deployed, errors arising during normal SQLite operation under archive-ahead conditions still surface as the generic message.

### Recovery triggers

Reconstruction from the Git archive happens on three triggers, in this order of how reliably they fire:

1. **Daemon startup** — `reconcile_archive_state_before_init` (`pool.rs:3606`) runs before opening the pool. Always reconciles archive-ahead state. Fires reliably *only* on restart.
2. **Periodic integrity guard** — runs `PRAGMA integrity_check` every ~hour. If the *file* is corrupt, defers reconstruct to the next "corrupt-verdict read." Does NOT detect "I dropped a row" drift, because the file is structurally healthy.
3. **Readiness archive-drift detector** — runs opportunistically inside the verdict computation when read pools are opened. Fires sometimes, doesn't fire reliably for short-window drift. This is what the manifest's "5-second wait recovers" relied on.

Critically: **none of these is gated on the fresh-durability probe failure**. The probe knows the SQL row is missing, but no code path turns that knowledge into a reconstruct trigger.

### Test-deterministic helpers (use these in your new tests)

These are public APIs you can call from tests to remove timing flake:

```rust
mcp_agent_mail_storage::wbq_flush();   // storage/lib.rs:430 — sync-drain the write-back queue
mcp_agent_mail_storage::wbq_stats();   // storage/lib.rs:488 — inspect queue depth
mcp_agent_mail_storage::get_commit_coalescer().flush_sync();  // storage/lib.rs:3274
```

`tests/conformance.rs:1487` and `:2077` already use `mcp_agent_mail_storage::wbq_flush()` to deterministically wait for queue drain in conformance tests — copy that pattern.

---

## THE FIVE FAILURE MODES IN DETAIL

Address them in priority order. Items 1–2 are the actual root causes; items 3–5 are surface manifestations the existing patch tries to address.

### Mode 1 — the durability probe gate is broken: it returns Ok when only the pooled handle sees the row

**Verified code anchor**: `crates/mcp-agent-mail-db/src/queries.rs`, function `verify_message_recipients_visible_after_commit` at line 1769 (and sister `verify_agent_visible_after_commit` at line 1506). The exact broken pattern, copied from the source:

```rust
// queries.rs:1769 — verify_message_recipients_visible_after_commit
let fresh_result = verify_message_recipients_visible_with_probe_mode(
    cx, pool, project_id, message_id, expected_recipients,
    MessageVisibilityProbeMode::FreshHandle,                // ← opens NEW connection
).await;
let Outcome::Err(fresh_error) = &fresh_result else {
    return fresh_result;                                    // probe saw row → return Ok
};
if pool.sqlite_path() == ":memory:"
    || !is_message_visibility_probe_consistency_error(fresh_error)
{
    return fresh_result;                                    // not a "not visible" error → return as-is
}

match verify_message_recipients_visible_with_probe_mode(
    cx, pool, project_id, message_id, expected_recipients,
    MessageVisibilityProbeMode::PooledHandle,               // ← reuses POOLED connection
).await {
    Outcome::Ok(()) => {
        tracing::warn!(
            project_id, message_id, fresh_error = %fresh_error,
            "fresh durability probe missed committed message visibility; \
             pooled runtime handle confirmed rows"
        );
        Outcome::Ok(())                                     // ← BUG: returns Ok despite fresh miss
    }
    Outcome::Err(pooled_error) => {
        tracing::warn!(/*both failed*/);
        fresh_result                                         // returns the fresh error
    }
    ...
}
```

**Why this is the root cause**: the comment two lines above the function explicitly says the function exists to "guard against ghost success where the API returns success but `message_recipients` rows are missing." But the guard's fallback is structurally unsound:

- `MessageVisibilityProbeMode::FreshHandle` opens a brand-new SQLite connection — same isolation level a future `search_messages` / `fetch_inbox` / `resource://inbox` call will see when it opens its own connection.
- `MessageVisibilityProbeMode::PooledHandle` reuses an existing pooled handle — the same handle that performed the INSERT, which still sees its own writes via SQLite's read-your-own-writes guarantee.
- When `Fresh fails ∧ Pooled succeeds`, that means **the row exists in the writer's view but not yet in any other connection's view**. Returning `Ok(())` here is exactly the false-success contract.

**Live evidence from this host**: at `2026-05-06T08:16:46.560Z` the daemon logged this exact WARN for `message_id=8` 42 ms after `send_message` returned success. The row never became visible to fresh connections in the next 2+ minutes.

**Architectural cause beneath the gate**: the project uses a custom SQLite fork ("FrankenSQLite") with optional MVCC concurrent transactions. `create_message_with_recipients_tx` at `queries.rs:5478` runs `BEGIN CONCURRENT → INSERT message → INSERT recipients → COMMIT` (gated by env var `FSQLITE_CONCURRENT_MODE=true`, see `queries.rs:1309-1346`). Under MVCC snapshot isolation, the writer's connection sees its own committed transaction immediately, but propagation to other readers' snapshot windows is not strictly synchronous. The write-back queue at `storage/lib.rs:401 wbq_enqueue` (polled at `WBQ_FLUSH_INTERVAL_MS = 100ms`) and `CommitCoalescer` at `storage/lib.rs:1724` (observed `latency_p50 = 1289ms`) compound the visibility window. The probe falls into that window and incorrectly accepts the pooled view as proof.

> **Why WAL is not "the fix" for this bug.** `am doctor check` flags `[WARN] wal_mode` and `am doctor fix --yes` will run `PRAGMA journal_mode = WAL`. This is tempting as a quick deployment, but read `crates/mcp-agent-mail-db/src/schema.rs:240-248` first: WAL is **intentionally avoided** for FrankenConnection-backed databases because of "mixed-runtime corruption and malformed-image scenarios when the server process is terminated abruptly." The DELETE journal mode is a deliberate design choice, not a missed configuration. Slice D (probe-gate fix) is the right fix; do not flip WAL as a shortcut.

**Verified call chain (top to bottom)**:

```
JSON-RPC tools/call send_message
  └── messaging.rs:1427  pub async fn send_message(...)
        ├── ... validation, normalization, archive write ...
        └── messaging.rs:2073  queries::create_message_with_recipients(...)
              └── queries.rs:~5427 inside pub async fn create_message_with_recipients
                    ├── queries.rs:5478  create_message_with_recipients_tx(...)
                    │     └── BEGIN CONCURRENT → INSERTs → COMMIT
                    ├── queries.rs:5427  verify_message_recipients_visible_after_commit(...)
                    │     ├── FreshHandle probe via durability_probe_query (queries.rs:1426)
                    │     │   └── DbConn::open_file(...) — completely fresh connection,
                    │     │      no recovery, no fallback (queries.rs:1448 comment)
                    │     ├── if Fresh.Err && is_message_visibility_probe_consistency_error:
                    │     │   PooledHandle probe via acquire_conn(cx, pool) (queries.rs:1635)
                    │     └── if Pooled.Ok → log WARN, return Outcome::Ok(())  ← BUG
                    ├── if probe error: annotate_message_visibility_error_with_writer_counts
                    │   then cleanup_message_after_post_commit_probe_failure (queries.rs:3210)
                    └── invalidate cached inbox stats, return MessageRow
```

The verified test at `queries.rs:18614` named `verify_message_recipients_visible_with_pooled_handle_sees_committed_rows` is itself proof the pooled handle sees committed rows — exactly what makes the fallback unsafe as a gate.

**Fix direction**: stop returning `Ok(())` at queries.rs:1808 (and the sister at queries.rs:1551 in the agent function). The right shape is a bounded retry on the **fresh** handle, matching the existing `durability_probe_backoff` style. Full code in Slice D below.

**Sanity rules for the fix**:

- Do **not** delete the pooled-handle fallback code without a replacement — it discriminates the "not visible after commit" error from other DB errors via `is_message_visibility_probe_consistency_error` at queries.rs:1673 / `is_agent_visibility_probe_consistency_error` at queries.rs:1563. Keep the discrimination; drop the silent acceptance.
- Do **not** add a synchronous wait that reuses the writer's pool — read-your-own-writes guarantees it succeeds and you'd ship a fix that passes its own test and fails in the wild. The gating decision must always use a fresh handle.
- The post-commit-probe contract is enforced by `is_hard_post_commit_probe_error` at queries.rs:3063, which always returns `true` ("Any probe error means that proof failed, so none of these errors are advisory"). The unit test at queries.rs:18263 (`post_commit_probe_errors_are_never_advisory`) locks this contract — your fix must preserve it.
- When the probe correctly returns Err, `cleanup_committed_message_after_consistency_failure` (called from `cleanup_message_after_post_commit_probe_failure` at queries.rs:3210) deletes the SQLite rows. **Verify this cleanup also handles the canonical archive `.md` file** (likely written upstream in messaging.rs before queries::create_message_with_recipients is invoked). If the cleanup only deletes from SQLite, the fix would create exactly the archive-ahead drift the manifest documents — *every* error case becomes a persistent inconsistency. This is a critical scope-of-fix question; resolve it before landing Slice D.
- The same shape of fix must apply to: `verify_agent_visible_after_commit` (queries.rs:1506), and the additional `durability_probe_query` callers at queries.rs:1632, :1850, :1882, :4037. Grep `durability_probe_query` to find every production caller (8 sites in queries.rs, plus 18 in tests). Each non-test site needs the same audit.

### Mode 2 — drift is unbounded, not eventually consistent

**Code anchor**: search for callers of `reconcile_archive_state_before_init` (`pool.rs`) and `reconstruct_from_archive` (`reconstruct.rs`). Then trace what triggers them post-startup.

**Evidence**: 2+ minutes after `send_message` returned id=8, health was still red and `search_messages` still failing. Periodic integrity guard ran during that window (`checks_total` advanced `133 → 134`, `last_ok_ts` updated) and reported the file healthy — so it did not trigger reconstruct. The readiness archive-drift detector did not fire either.

**Why this matters**: the manifest's "control" cycle that recovered after a 5-second wait was lucky. Without an integrity-guard-or-corruption signal, archive-ahead-of-SQLite drift is a **stable state** that survives across hours, daemon restarts (until startup reconcile fires), and any number of read attempts. From the user's perspective, `send_message` returns a message id that no future read will surface — effective data loss for that read surface, even though the archive holds the row.

**Fix direction**: the fresh-durability-probe failure is already a known signal. Wire it into the recovery scheduler so that any probe miss enqueues a bounded local catch-up (not a full reconstruct) for the missing IDs. A row-level catch-up that re-runs the SQL inserts from the canonical archive for `archive_only_ids` should complete in milliseconds — far cheaper than a full reconstruct, and it can run synchronously inside the `send_message` response budget if Mode 1 is fixed by gating.

### Mode 3 — `DATABASE_DEGRADED` exists in the patch but is not deployed

**Code anchor**: `crates/mcp-agent-mail-tools/src/lib.rs::tool_util::archive_sqlite_degraded_error` (added in the uncommitted patch, around line 376 onwards). Wired into `read_pool_setup_error_to_mcp_error` as an early-return wrapper, and re-exported into `resources.rs::resource_sync_db_error_to_mcp_error`.

**What the patch produces** (good shape, just not deployed):

```json
{
  "type": "DATABASE_DEGRADED",
  "message": "Agent Mail SQLite index is behind or unavailable relative to the Git archive; reads are temporarily degraded.",
  "recoverable": true,
  "data": {
    "error_detail": "<original setup error>",
    "semantic_readiness": {
      "status": "fail",
      "detail": "...",
      "archive_projects": 3, "archive_agents": 7, "archive_messages": 8, "archive_latest_id": 8,
      "db_projects": 3, "db_agents": 7, "db_messages": 7, "db_max_id": 7,
      "missing_archive_projects": []
    },
    "recovery": {
      "mode": "degraded_read_only",
      "next_action": "Run `am doctor reconstruct --dry-run` ... or wait for automatic recovery to finish.",
      "recovery_lock_active": false,
      "recovery_lock_pid": null
    }
  }
}
```

**Why the patch alone is insufficient**: it only fires from `read_pool_setup_error_to_mcp_error`, which only runs when the snapshot pool setup itself errors. In Mode A failures (`search_messages`/`fetch_inbox` returning generic error without any snapshot attempt), the error path is different — likely a pre-snapshot guard or a different DB error mapper. You must trace where the generic error originates in the live code (it is *not* always `read_pool_setup_error_to_mcp_error`) and ensure `archive_sqlite_degraded_error` is invoked there too. Greppable strings: `"This may be a transient issue"`, `DbError::Sqlite`, `is_lock_error`, `db_error_to_mcp_error`.

### Mode 4 — macOS `/var → /private/var` symlink rejection

**Code anchor**: `crates/mcp-agent-mail-db/src/pool.rs::validate_sqlite_target_path` and its caller `crates/mcp-agent-mail-db/src/reconstruct.rs::reconstruct_from_archive`.

**What happens**: `ResourceSnapshotDirGuard::new()` and `ReadSnapshotDirGuard::new()` create directories under `std::env::temp_dir()`, which on macOS is `/var/folders/<hash>/T/`. When reconstruct tries to write a SQLite into that directory, `validate_sqlite_target_path` walks every component with `symlink_metadata` and rejects `/var` because it's a symlink to `/private/var`.

**Patch state**: the patch canonicalizes the snapshot directory immediately after `create_dir`:

```rust
match std::fs::create_dir(&path) {
    Ok(()) => {
        let path = path.canonicalize()?;   // /var/folders/.../X → /private/var/folders/.../X
        return Ok(Self { path });
    }
    ...
}
```

**Why this might still be wrong**: the canonicalized path is stored in the guard, but `reconstruct_from_archive` may receive the path through a different parameter or rebuild the target by joining the (now-canonical) snapshot root with `mailbox.sqlite3` — verify that the path passed to `validate_sqlite_target_path` is the canonical one. If `validate_sqlite_target_path` itself canonicalizes its input, the bug fix shifts to it; otherwise you need to ensure every caller canonicalizes before calling.

**Containment requirement (do not weaken security)**: the live `validate_sqlite_target_path` correctly rejects symlinked components for *caller-controlled* SQLite targets, because allowing those would let a malicious project key escape the storage root. The fix must distinguish snapshot targets (internally created, contained under a canonical temp root) from live database targets (caller-influenced, must reject symlinked traversal). Add a snapshot-specific containment check:

1. Canonicalize the snapshot root after creation.
2. Build the SQLite target as `<canonical_snapshot_root>/mailbox.sqlite3`.
3. Canonicalize the parent directory of the target.
4. Assert the canonical parent starts with the canonical snapshot root.
5. Allow the write if and only if the assertion holds.
6. Keep the existing strict `validate_sqlite_target_path` for any path the caller supplies.

There is a unit test in the patch (`resource_snapshot_dir_canonicalizes_symlinked_temp_base`) that constructs a symlinked temp base and asserts the snapshot path is canonical. Make sure your fix passes that test *and* an integration test that actually invokes `reconstruct_from_archive` against the canonicalized snapshot.

### Mode 5 — diagnostic text mismatch

**Code anchors**: `crates/mcp-agent-mail-server/src/startup_checks.rs::format_recovery_context` (line ~1749) and `crates/mcp-agent-mail-tools/src/identity.rs::build_recovery_status` (line ~174). Both currently print `"Run \`am doctor repair\` to attempt automatic recovery"`.

**Reality**: the `am doctor repair` next-action does not exist as a reconstruct path; the working primitive is `am doctor reconstruct --yes` (or `am doctor fix --yes` for the broader hardening pass). `am doctor check` simultaneously reports `[FAIL] archive_db_parity` with `Next action: No database repair action is currently recommended`.

**Patch text** (already correct, just undeployed):

```text
Run `am doctor reconstruct --dry-run` to inspect archive-backed recovery, then
`am doctor reconstruct --yes` if needed
```

**Additional surface to fix**: `am doctor check`'s top-line "Next action" logic. Find the function that prints the primary issue and next-action; ensure that `[FAIL] archive_db_parity` does not coexist with `"No database repair action is currently recommended"`. The CLI surface, the MCP `health_check.recovery.next_action`, and the resource error payloads must all agree.

---

## WHAT THE EXISTING PATCH ACTUALLY DOES (BE HONEST ABOUT IT)

The dirty working tree at `/Users/zacfarrell/Desktop/mcp_agent_mail_rust` is the in-progress implementation of bead ticket **`br-nw23y`** ("Fix archive-ahead read/search/resource recovery diagnostics"). The patch author marked the bead **closed** at `2026-05-05T23:27:36Z` with `close_reason="Completed archive-ahead read/resource diagnostics and macOS snapshot canonicalization"` — but the working tree was never committed and the patch was never built or installed. So the bead reads as "done" while the binary on disk still reflects the pre-patch behavior. The receiving agent should:

1. Decide whether `br-nw23y` should be reopened or marked as "in-progress" once the actual fix lands.
2. Treat the patch as a partial implementation that addresses Modes 3, 4, and 5 — and **does not touch** Modes 1 or 2 (which are the actual root causes documented above).

```bash
git -C /Users/zacfarrell/Desktop/mcp_agent_mail_rust diff --stat
# .beads/issues.jsonl                                |   1 +
# crates/mcp-agent-mail-db/src/pool.rs               |  15 +-     # tests only — canonicalize tempdir in three test fns
# crates/mcp-agent-mail-db/src/reconstruct.rs        |  14 +-     # tests only — canonicalize tempdir in two symlink tests
# crates/mcp-agent-mail-server/src/startup_checks.rs |   2 +-     # next_action text fix
# crates/mcp-agent-mail-tools/src/identity.rs        |   2 +-     # next_action text fix
# crates/mcp-agent-mail-tools/src/lib.rs             | 250 ++++   # NEW archive_sqlite_degraded_error + ReadSnapshotDirGuard::new_in
# crates/mcp-agent-mail-tools/src/resources.rs       | 131 +++    # ResourceSnapshotDirGuard::new_in + wire-up + tests
```

What the patch's `archive_sqlite_degraded_error` does:

1. Looks at the current `Config`, resolves the SQLite path.
2. Refuses to act on `:memory:` databases.
3. Calls `mcp_agent_mail_db::scan_archive_message_inventory(&config.storage_root)`.
4. Opens a fresh `DbConn` to the SQLite, queries inventory.
5. Calls `archive_metadata_advantage_is_decisive` to decide whether the archive is decisively ahead.
6. If yes (or if the message contains hints like "reconstruct", "snapshot", "database disk image is malformed"), wraps the error with the `DATABASE_DEGRADED` payload shown earlier.
7. Otherwise returns `None` and falls through to the existing `DbError::Sqlite(msg)` path.

The patch wires this in at exactly two sites:
- `tool_util::read_pool_setup_error_to_mcp_error` (`lib.rs`, after the lock-error early-return)
- `resource_sync_db_error_to_mcp_error` (`resources.rs`, mirror)

What the patch does **not** do:

- It does **not** modify `db_error_to_mcp_error` (the generic mapper). So errors that flow through that path during normal SQLite operation (not snapshot setup) still surface as the unstructured `"A database error occurred"`.
- It does **not** modify `send_message` or the durability probe handling. The race window is unchanged.
- It does **not** add a new recovery trigger. Drift is still unbounded.
- It does **not** unify the `am doctor check` next-action text with the new MCP next-action text.
- The new test `read_pool_setup_error_reports_archive_sqlite_degraded_state` cannot run because of the linker failure (see Toolchain Issues).

You are inheriting a partial fix. Treat it as a starting point for Modes 3–5, then add the deeper work for Modes 1–2.

---

## FIX SLICES IN PRIORITY ORDER

The ordering below is the **fix-the-defect** order, not the "lowest-risk-first" order. **Slice D is the actual bug fix; Slices A/B/C are improved diagnostics and defense in depth**. Land Slice D first if you can demonstrably write a failing test for the probe gate before you start; otherwise land Slice A first to get visibility into what's failing in production, then Slice D.

### Slice D summary (skim this first; full body is below)

The smallest, most important code change in this whole handoff. Two functions in `crates/mcp-agent-mail-db/src/queries.rs`:

- `verify_message_recipients_visible_after_commit` at line 1769 — remove the `Outcome::Ok(())` at line 1808
- `verify_agent_visible_after_commit` at line 1506 — remove the `Outcome::Ok(())` at line 1551

Replace with a bounded poll on the **fresh** handle, capped at ~1500ms (covers observed `commit_coalescer.latency_p99 = 1289ms`). After the budget, return the original `fresh_error` so the caller surfaces a structured `WRITE_VISIBILITY_DEFERRED` MCP error. Full code shape is in the Slice D detail below.

### Slice A — make read tools surface `DATABASE_DEGRADED` everywhere drift is known (deploy the patch correctly)

This is the lowest-risk, highest-immediate-value slice. It does not change behavior; it improves the operator's ability to diagnose.

1. Decide whether to commit the existing patch or rebuild it cleanly. Recommendation: commit it as-is on a feature branch, then iterate on top.
2. Audit every call site that currently returns `DbError::Sqlite(msg)` for a known archive-ahead state. Specifically:
   - `tool_util::db_error_to_mcp_error` (the generic mapper used by most tools, not just snapshot setup)
   - `resource_sync_db_error_to_mcp_error`
   - Any direct `DbError::Sqlite` constructions in `messaging.rs`, `search.rs`, `resources.rs`
3. At each site, call `archive_sqlite_degraded_error(&message)` first; only fall through to `DbError::Sqlite(message)` if it returns `None`.
4. Ensure the verdict-evaluation cost inside `archive_sqlite_degraded_error` is acceptable per error path (it opens a DB connection, scans archive). Cache the verdict if the per-request cost is noticeable.
5. Add an integration test that:
   - Sets up an archive-ahead fixture (one project, one agent, one message in archive; empty SQLite).
   - Calls `search_messages`, `fetch_inbox`, `resource://inbox`.
   - Asserts each returns `error.type == "DATABASE_DEGRADED"` with non-zero `archive_messages` and zero `db_messages`.
   - Asserts the next-action contains `"am doctor reconstruct"`.

**Deployment effect**: the operator now sees archive/db counts in tool errors. The race itself is unchanged.

### Slice B — fix the macOS `/var` symlink rejection (deploy the rest of the patch)

1. Confirm the patch's `ResourceSnapshotDirGuard::new_in` and `ReadSnapshotDirGuard::new_in` correctly canonicalize after `create_dir`.
2. Trace `reconstruct_from_archive`'s path-validation site. Confirm the path passed to `validate_sqlite_target_path` is the canonical snapshot path. If not, either canonicalize at the call site or add a snapshot-specific validator.
3. Add a snapshot-specific containment validator (`validate_snapshot_sqlite_target` or similar) that:
   - Accepts the canonical snapshot root.
   - Canonicalizes the parent of the proposed target.
   - Verifies the canonical parent starts with the canonical snapshot root.
   - Rejects if not.
4. Keep `validate_sqlite_target_path` strict for caller-controlled paths.
5. Add the patch's `resources_use_archive_snapshot_when_temp_root_is_symlinked` test plus an integration test that sets `TMPDIR=/var/folders/.../linked-temp-base` (a symlinked tempdir) and verifies `resource://inbox` succeeds against an archive-ahead fixture.

**Deployment effect**: resource reads can now actually fall back to archive snapshots on macOS. They will surface real data instead of `traverses symlinked path /var`.

### Slice C — make read paths fall back to the canonical archive for archive-only IDs (Mode 2 fix, defense in depth)

This is *defense in depth* for cases where the Slice D probe gate is bypassed (e.g., schema changes, future regressions, or a recovery already in flight). It does **not** depend on Slice D landing first.

1. In `tool_util::open_read_db_pool` (lib.rs:774) and `resources::open_resource_read_pool` (resources.rs:335), when the verdict reports `ArchiveAhead` and the snapshot reconstruction has *succeeded*, the snapshot pool already serves archive-only rows. Verify this is the case by inspecting the snapshot DB's `messages` count after `reconstruct_from_archive_with_salvage` returns.
2. When the verdict reports `ArchiveAhead` and the snapshot reconstruction *fails* (e.g., disk full, snapshot validator rejection without Slice B's macOS fix), the read tool should still consult the canonical archive directly for the requested message id / thread / inbox query, instead of returning a generic error. This requires a `mcp_agent_mail_storage` helper that reads `.md` files from the archive when SQLite is unavailable.
3. As a smaller alternative if (2) is too invasive: have `read_pool_setup_error_to_mcp_error` (lib.rs:375) return a structured `DATABASE_DEGRADED` (which Slice A already does via the patch) with a richer `archive_only_ids` field listing the specific message ids the verdict thinks are archive-only. Callers can then re-issue the read against a forced reconcile or call `am doctor reconstruct`.
4. Add tests:
   - Archive-ahead with healthy snapshot pool → read returns the archive-only message.
   - Archive-ahead with broken snapshot pool → read returns `DATABASE_DEGRADED` with the missing id list.
   - Concurrent read-pool setup failures → only one snapshot/reconcile fires.

**Deployment effect**: even if a future regression bypasses the probe gate, callers get specific actionable diagnostics instead of generic transient errors.

### Slice D — fix the durability probe gate (Mode 1 fix — primary contract)

The smallest and most important code change in this whole handoff. Both files, two functions:

| File:line | Function | What to change |
|---|---|---|
| `crates/mcp-agent-mail-db/src/queries.rs:1769` | `verify_message_recipients_visible_after_commit` | Stop returning `Outcome::Ok(())` from the pooled-handle fallback at queries.rs:1808 |
| `crates/mcp-agent-mail-db/src/queries.rs:1506` | `verify_agent_visible_after_commit` | Same shape: stop returning `Outcome::Ok(())` from the pooled-handle fallback at queries.rs:1551 |

Recommended replacement (apply to both functions; uses the project's existing primitives):

```rust
// imports already present at queries.rs:18 — `use asupersync::time::{sleep, wall_now};`
// imports already present in queries.rs near the other probe code — `use crate::retry::RetryConfig;`
// (these are the same primitives durability_probe_backoff at queries.rs:1491 uses)

// instead of:
//     match verify_..._with_probe_mode(..., PooledHandle).await { Ok(()) => warn + return Ok, ... }
// do:

// Bounded poll on a FRESH handle. The pooled handle is unsafe as a gate
// because it sees the writer's own pre-publication MVCC snapshot under BEGIN CONCURRENT
// (see queries.rs:1346 `begin_concurrent_tx`).
const PROBE_VISIBILITY_MAX_ATTEMPTS: u32 = 30;   // ~1.5s budget — covers observed commit_coalescer p99 of 1289ms
let backoff = RetryConfig {
    base_delay: Duration::from_millis(25),       // matches durability_probe_backoff (queries.rs:1491)
    max_delay: Duration::from_millis(200),
    use_circuit_breaker: false,
    ..Default::default()
};

let mut last_fresh_error = fresh_error.clone();
for attempt in 0..PROBE_VISIBILITY_MAX_ATTEMPTS {
    let () = sleep(wall_now(), backoff.delay_for_attempt(attempt)).await;
    let retry = verify_message_recipients_visible_with_probe_mode(
        cx, pool, project_id, message_id, expected_recipients,
        MessageVisibilityProbeMode::FreshHandle,
    ).await;
    match retry {
        Outcome::Ok(()) => {
            if attempt > 0 {
                tracing::warn!(
                    project_id, message_id, attempts = attempt + 1,
                    "fresh durability probe required retry after commit \
                     (write-visibility window observed)"
                );
            }
            return Outcome::Ok(());
        }
        Outcome::Err(e) if is_message_visibility_probe_consistency_error(&e) => {
            last_fresh_error = e;   // keep looping; this is the visibility window
        }
        Outcome::Err(other) => return Outcome::Err(other),
        Outcome::Cancelled(r) => return Outcome::Cancelled(r),
        Outcome::Panicked(p) => return Outcome::Panicked(p),
    }
}

// Budget exhausted — write was acked at writer pool but never published to fresh handles.
// Cross-check the pooled handle one final time so the error message can include writer-side
// confirmation; preserves the existing observability behavior without using it as a gate.
let pooled_outcome = verify_message_recipients_visible_with_probe_mode(
    cx, pool, project_id, message_id, expected_recipients,
    MessageVisibilityProbeMode::PooledHandle,
).await;
match pooled_outcome {
    Outcome::Ok(()) => {
        tracing::error!(
            project_id, message_id, fresh_error = %last_fresh_error,
            "post-commit visibility budget exhausted: pooled handle confirms row but \
             fresh handle still cannot see it after {}ms; rejecting send to avoid ghost success",
            PROBE_VISIBILITY_MAX_ATTEMPTS * 100   // approximate upper bound
        );
    }
    Outcome::Err(both) => {
        tracing::error!(
            project_id, message_id, fresh_error = %last_fresh_error, pooled_error = %both,
            "post-commit visibility budget exhausted: both fresh and pooled probes fail"
        );
    }
    other => return other,
}
return Outcome::Err(last_fresh_error);
```

Notes:

- **Use the project's `asupersync::time::sleep`, not `tokio::time::sleep`.** `queries.rs:18` already imports `use asupersync::time::{sleep, wall_now};` — the codebase runs on a custom executor. Direct tokio calls may not work or may bypass test-time control.
- **Use `crate::retry::RetryConfig` with the same `base_delay=25ms / max_delay=200ms`** as `durability_probe_backoff` at queries.rs:1491-1499. This keeps the project's retry conventions consistent and inherits the existing exponential backoff curve.
- **Budget**: `PROBE_VISIBILITY_MAX_ATTEMPTS=30` with the 25-200ms backoff curve gives roughly 1.5s of poll wall time, which covers the observed `commit_coalescer.latency_p99 = 1289ms`. Tune in profiling, but do not go below `commit_coalescer.latency_p95` or you will routinely cause false-failure send_message responses.
- Optional pre-poll call: `mcp_agent_mail_storage::get_commit_coalescer().flush_sync()` (storage/lib.rs:3274) right before entering the loop pushes any pending COMMIT through immediately. This trades ~5-50ms of work for a faster typical-case propagation. Recommended for the message gate; probably overkill for the agent gate (registers are rare).
- The same shape of fix must apply at `verify_agent_visible_after_commit` (queries.rs:1506) — substitute `is_agent_visibility_probe_consistency_error` (queries.rs:1563) and `verify_agent_visible_on_pooled_handle` (queries.rs:1567).
- The wider call surface — `durability_probe_query` callers at queries.rs:1632, :1850, :1882, :4037 — invoke `durability_probe_query` directly without the pooled-handle fallback. They are not affected by Mode 1's specific fallback bug, but you should still audit each one to confirm it does not swallow probe errors at a higher layer. The probe-query itself already handles transient busy errors via `durability_probe_backoff`.
- The MCP error mapper layer needs a translation when the post-budget error bubbles up. At `messaging.rs:1427` (`send_message`), map the post-commit probe error to a structured MCP error such as `WRITE_VISIBILITY_DEFERRED`. Keep `verified_sender` and the existing schema; add fields, do not break them. Slice A's `archive_sqlite_degraded_error` is the template for new structured error shapes.

**Critical compatibility verification (do this BEFORE merging Slice D)**:

When the probe returns Err, `cleanup_message_after_post_commit_probe_failure` (queries.rs:3210) → `cleanup_committed_message_after_consistency_failure` runs. **You must verify this cleanup also removes the canonical archive `.md` file** (likely written upstream in `messaging.rs` before `create_message_with_recipients` is invoked). If cleanup only handles SQLite rows, fixing Slice D will trade one bug (false success + invisible row) for another (every visibility-window failure becomes a persistent archive-only orphan — exactly the drift the manifest documents).

Audit by reading the message-creation flow in messaging.rs:1427+ and confirming whether canonical-write happens (a) inside `create_message_with_recipients` (and is therefore covered by cleanup), or (b) before/around it (in which case cleanup needs an analogous archive removal). If (b), extend cleanup or document the cleanup-scope expansion as part of Slice D.

5. Add tests:
   - **Unit (db crate)**: synthetic pool where the writer connection sees a row but no other connection does (force this by holding a write transaction open from a stub). The probe must return `Err`, not `Ok`.
   - **Integration (tools crate)**: 20-iteration smoke loop on a real isolated storage root: `send_message` → immediate `search_messages` / `fetch_inbox` / `resource://inbox/{agent}` / `resource://thread/{thread}`. Zero generic `DATABASE_ERROR`s, every id visible immediately. Use `mcp_agent_mail_storage::wbq_flush()` between iterations to keep the test deterministic.
   - **Concurrency (tools crate)**: 4 senders × 50 messages each, immediate read after every send. Zero generic errors. Verify `commit_coalescer.flush_sync()` is not deadlocked by the probe wait.
   - **Latency regression (criterion bench)**: measure p50/p95/p99 latency of `send_message` before and after the fix on a hot loop. Expect a measurable but bounded increase (target: p99 ≤ 200ms increase under default config).

**Deployment effect**: `send_message` returning success becomes a true read-after-write contract for fresh connections. The race in the manifest goes away by construction.

### Slice E — align the three diagnostic surfaces

Already partially in the patch. Finish it:

1. Patch `am doctor check`'s top-line next-action so `[FAIL] archive_db_parity` never coexists with `"No database repair action is currently recommended"`.
2. Make `am doctor` help text recommend `reconstruct` for the archive-ahead-DB-empty condition.
3. Add a CLI-output snapshot test that asserts the three surfaces (`health_check`, `am doctor check`, the patch's MCP `next_action` text) point to the same recovery primitive for the same condition.

---

## CONSTRAINTS

These are non-negotiable. A fix that violates any of them is not acceptable.

1. **Do not mutate or delete data in `~/.mcp_agent_mail_git_mailbox_repo/`** during development. Use `tempfile::tempdir()` fixtures (canonicalized after creation — see Mode 4 above) for all tests. The live store contains real cross-project mailbox state.
2. **Do not weaken `validate_sqlite_target_path` for caller-controlled paths.** Any path derived from a `project_key`, an MCP request parameter, or env var must remain symlink-rejecting. Only internally-created snapshot roots may use the relaxed canonical-containment check.
3. **Do not skip the fresh-durability probe** even after fixing Mode 1. The probe is also useful as a long-term canary for SQLite isolation regressions. Promote it from observability to enforcement; do not delete it.
4. **Do not relax write durability guarantees**. The canonical archive write must remain the first-class durable surface. SQLite is the index, not the truth.
5. **Do not collapse the `verified_sender` field** or any existing response shape. Add fields, do not change or remove. Other clients (Claude Desktop, the CLI, codex-cli) consume the existing schema.
6. **Do not bypass `cargo fmt` / `clippy --all-targets`** to land the patch. The existing test additions in the patch already need clippy review.
7. **Do not skip building a new `am` binary and restarting the LaunchAgent** as part of "deployment." Source-only fixes do not change live behavior. See Deployment section.
8. **Do not file or rely on `am doctor repair` as a recovery primitive** in any new code. Use `am doctor reconstruct` and/or `am doctor fix`.

---

## TOOLCHAIN ISSUES YOU MUST FIX FIRST

Before any test can run, fix the linker failure for `mcp-agent-mail-tools`:

```text
error: linking with `cc` failed: exit status: 1
Undefined symbols for architecture arm64:
  "_OBJC_CLASS_$_MLComputePlan", referenced from:
      objc-class-ref in libort_sys-...rlib(model.mm.o)
  "_OBJC_CLASS_$_MLOptimizationHints", referenced from:
      objc-class-ref in libort_sys-...rlib(model.mm.o)
```

Likely paths (try in order):

1. **Disable the ONNX/CoreML feature flag** if `mcp-agent-mail-tools` enables it via a default Cargo feature. The classes belong to Apple's CoreML framework; `libort_sys` is the ONNX Runtime sys crate. Probably an embedding/search-AI feature that is irrelevant for the read-pool unit test.

```bash
rg -n "ort|onnx|ortruntime|CoreML|libort_sys" /Users/zacfarrell/Desktop/mcp_agent_mail_rust/crates/mcp-agent-mail-tools/Cargo.toml
rg -n "ort|onnx" /Users/zacfarrell/Desktop/mcp_agent_mail_rust/crates/*/Cargo.toml
cargo test -p mcp-agent-mail-tools --no-default-features read_pool_setup_error_reports_archive_sqlite_degraded_state -- --exact
```

2. **If ONNX is genuinely required for the test harness** (e.g., search relevance), add the missing CoreML link directive in a `build.rs` or via `RUSTFLAGS`:

```bash
RUSTFLAGS="-C link-arg=-framework -C link-arg=CoreML" \
  cargo test -p mcp-agent-mail-tools read_pool_setup_error_reports_archive_sqlite_degraded_state -- --exact
```

3. **Last resort**: pin to an older `ort`/`libort_sys` that doesn't reference these symbols, or compile against a different macOS SDK.

The receiving agent should add a CI guard that asserts `cargo test -p mcp-agent-mail-tools` succeeds on macOS arm64 with the default feature set used in production.

Other commands you'll want available:

```bash
cargo build --release -p mcp-agent-mail   # or whichever crate produces the `am` binary
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
```

---

## DEPLOYMENT — HOW TO ACTUALLY SHIP THIS

A source change is not a deployment. The live `am` is at `/Users/zacfarrell/.local/bin/am` and the LaunchAgent is `com.agent-mail` running as PID 69379 today. Every fix must culminate in:

1. **Build a new release**:

```bash
cd /Users/zacfarrell/Desktop/mcp_agent_mail_rust
git status                                    # confirm clean working tree on your branch
cargo build --release --bin am                # or the workspace bin equivalent
ls -la target/release/am                      # confirm new binary exists
target/release/am --version                   # bump version, e.g. 0.2.52
```

2. **Stop the daemon cleanly**:

```bash
launchctl bootout gui/$UID/com.agent-mail
pgrep -f 'am serve-http' || echo "stopped"
```

3. **Install the new binary**:

```bash
cp target/release/am /Users/zacfarrell/.local/bin/am
codesign --force --sign - /Users/zacfarrell/.local/bin/am  # ad-hoc sign if needed for Gatekeeper
```

4. **Restart the LaunchAgent**:

```bash
launchctl bootstrap gui/$UID ~/Library/LaunchAgents/com.agent-mail.plist
am service status
am --version    # verify the new version
```

5. **Verify the deploy with the live reproduction recipe**:

   - Run `health_check`; expect green.
   - Run the Step 2 + Step 3 reproduction in this document; expect every read to succeed and `health_check` to stay green.
   - If the snapshot path is exercised, expect no `traverses symlinked path /var` error.
   - Loop the reproduction 20 times in quick succession; expect zero generic `DATABASE_ERROR`s.

6. **Inspect the daemon log** for the absence of `fresh durability probe missed committed message visibility`:

```bash
date -u '+%Y-%m-%dT%H:%M:%SZ'
# ... run reproduction loop ...
grep "fresh durability probe missed" ~/Library/Logs/agent-mail/stderr.log | awk '$1 > "<your-start-timestamp>"'
# expect: empty output
```

If you see new probe-miss WARNs after deploy, Slice C/D didn't take. Iterate.

---

## ROLLBACK / SAFETY

If your new build introduces a regression:

```bash
launchctl bootout gui/$UID/com.agent-mail
cp /Users/zacfarrell/.local/bin/am.previous /Users/zacfarrell/.local/bin/am   # if you saved it
launchctl bootstrap gui/$UID ~/Library/LaunchAgents/com.agent-mail.plist
am --version    # verify rollback
```

Always save a copy of the existing binary before installing the new one. Do not mass-delete the storage root. Do not edit `storage.sqlite3` directly. If post-rollback the SQLite is in an inconsistent state:

```bash
am doctor reconstruct --dry-run --json   # preview
am doctor reconstruct --yes --json       # apply if safe
```

The forensic bundles under `~/.mcp_agent_mail_git_mailbox_repo/doctor/forensics/` are the audit trail. Do not delete them; future debugging needs them.

---

## ACCEPTANCE CRITERIA

A complete fix satisfies all of the following.

### Functional

- [ ] `send_message` returning success implies the message id is visible to the next `search_messages`, `fetch_inbox`, `resource://inbox/{agent}`, and `resource://thread/{thread}` call from any client without sleep, retry, or recovery action.
- [ ] If immediate visibility cannot be guaranteed for an exceptional reason (e.g., known concurrent reconstruct), all four read paths return a structured `DATABASE_DEGRADED` error containing archive/db counts, recovery mode, and a next-action that points to a primitive that exists (`am doctor reconstruct` or `am doctor fix`).
- [ ] `resource://inbox/{agent}` and `resource://thread/{thread}` never fail with `traverses symlinked path /var` on macOS hosts where `std::env::temp_dir()` is `/var/folders/...`.
- [ ] `health_check.recovery.next_action` and `am doctor check`'s top-line "Next action" agree, and both point to a primitive that actually fires for the failing condition.
- [ ] `am doctor check` does not simultaneously report `[FAIL] archive_db_parity` and `Next action: No database repair action is currently recommended.`

### Evidence-based

- [ ] Run the reproduction recipe in this document with 20 fresh markers in a tight loop. Zero generic `DATABASE_ERROR`s. Zero red `health_check`s after `send_message` returns. Zero `fresh durability probe missed committed message visibility` log lines for the new binary.
- [ ] Run the same loop concurrently from two MCP clients (Claude Code + codex-cli, or two Claude Code panes). Zero failures.
- [ ] Run from a project key other than `Testproject1` (e.g., the `mcp_agent_mail_rust` project). Same zero-failure expectation.
- [ ] If you deliberately inject an archive-ahead state (e.g., copy a `.md` into `projects/.../messages/...` while the daemon is stopped, then restart), the read paths should either auto-recover within a bounded window OR return `DATABASE_DEGRADED` with correct counts.
- [ ] Forensic bundles under `~/.mcp_agent_mail_git_mailbox_repo/doctor/forensics/` continue to be captured before any reconstruct.

### Regression — existing tests that must keep passing

These exist in the source today and lock in the contract Slice D must preserve. If your fix breaks any of them, the fix is wrong; rework, don't update the test.

```bash
# Symlink-rejection tests for caller-controlled SQLite paths (Mode 4 must not weaken these)
cargo test -p mcp-agent-mail-db reconstruct_rejects_symlinked_destination_path
cargo test -p mcp-agent-mail-db reconstruct_rejects_symlinked_destination_parent

# Probe contract: any post-commit probe error is fatal (Mode 1 fix must preserve this)
cargo test -p mcp-agent-mail-db post_commit_probe_errors_are_never_advisory                  # queries.rs:18263
cargo test -p mcp-agent-mail-db post_commit_probe_cancelled_and_panicked_are_deterministic   # queries.rs:18284

# Probe behavior (Mode 1 fix is in this family)
cargo test -p mcp-agent-mail-db durability_probe_for_agent_visibility_ignores_uncommitted_writer_state    # queries.rs:17834
cargo test -p mcp-agent-mail-db durability_probe_for_message_recipients_ignores_uncommitted_writer_state  # queries.rs:18465
cargo test -p mcp-agent-mail-db durability_probe_rejects_never_inserted_agent                              # queries.rs:17927
cargo test -p mcp-agent-mail-db durability_probe_succeeds_for_committed_agent                              # queries.rs:17988
cargo test -p mcp-agent-mail-db durability_probe_succeeds_under_reserved_lock_for_committed_agent          # queries.rs:18058
cargo test -p mcp-agent-mail-db durability_probe_retries_transient_busy_after_committed_write              # queries.rs:18146
cargo test -p mcp-agent-mail-db verify_message_recipients_visible_after_commit_rejects_committed_message_without_recipients  # queries.rs:17180
cargo test -p mcp-agent-mail-db message_visibility_probe_consistency_error_classifier_is_narrow            # queries.rs:18592

# Tool-side probe behavior
cargo test -p mcp-agent-mail-tools tool_send_and_reply_success_are_immediately_visible_via_inbox_reads
```

⚠️ **Existing test that the fix WILL break (intentionally, and must be rewritten as part of Slice D)**:

```bash
cargo test -p mcp-agent-mail-db verify_message_recipients_visible_with_pooled_handle_sees_committed_rows  # queries.rs:18614
```

This test asserts the current (buggy) behavior — that the pooled handle confirms committed rows. Slice D removes the *use* of pooled-handle confirmation as a gate, but the pooled handle still sees the rows. Rewrite the test to assert the correct contract: when fresh probe fails and pooled probe succeeds, the function must return Err (after the budget) — not Ok. Add a new positive test asserting that when the bounded poll on the fresh handle eventually succeeds, the function returns Ok and emits the "probe required retry" warning.

### Regression — broader checks

- [ ] All existing tests pass with the listed exception above. Use `cargo test --workspace` after the linker fix.
- [ ] `register_agent`, `acknowledge_message`, `file_reservation_paths`, `release_file_reservations`, `create_agent_identity`, `whois`, `list_agents`, `list_contacts` still pass their existing tests.
- [ ] `agent-mail-search-fixes.patch` (the prior tracked patch in `Testproject1/agent-mail-search-fixes.patch`) still applies cleanly or has been merged. Do not regress search pagination, recency ordering, or sender-filter direction.
- [ ] `git diff --check` is clean.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` is clean.

### New tests required

- [ ] **Read-after-write integration test**: sends N=20 self-addressed messages, immediately searches/fetches/resource-reads each; fails on any miss or generic error.
- [ ] **Archive-ahead degraded-state test**: builds an archive-ahead fixture (real archive, empty SQLite); asserts read tools return structured `DATABASE_DEGRADED` with counts.
- [ ] **macOS symlinked tempdir test**: builds an isolated symlinked temp base, points `TMPDIR` at it, runs resource reads under archive-ahead state; fails if any error contains `traverses symlinked path`.
- [ ] **Containment guard test (negative)**: caller-controlled path with a symlinked parent must still be rejected. Snapshot relaxation must not weaken caller-path validation.
- [ ] **Diagnostic consistency test**: snapshot-tests for `health_check.recovery.next_action`, `am doctor check`'s next-action line, and `archive_sqlite_degraded_error`'s payload — all three must agree for the same fixture state.
- [ ] **Probe→catch-up integration test** (Slice C): forces a probe miss; asserts the row becomes visible to a fresh connection within a bounded budget.
- [ ] **Concurrent-write stress test**: multiple senders, immediate reads, no failures.

---

## INVESTIGATION ROADMAP (FIRST 60 MINUTES)

Do these in order before editing source. They prevent the next session from chasing the wrong layer.

### 1. Verify the bug, the binary, and the patch state

```bash
am --version                                          # expect 0.2.51 today
ls -la /Users/zacfarrell/.local/bin/am
git -C /Users/zacfarrell/Desktop/mcp_agent_mail_rust status --short
git -C /Users/zacfarrell/Desktop/mcp_agent_mail_rust log --oneline -5
git -C /Users/zacfarrell/Desktop/mcp_agent_mail_rust diff --stat
```

If the patch is gone or has been committed, adjust the plan. If `am --version` is ≥ 0.2.52 with the patch applied, the situation has changed since this handoff — re-run the reproduction recipe and re-baseline.

### 2. Re-run the reproduction recipe

Send a marker, do three immediate reads, capture the failure. If it does not fail, repeat 5 times. If it still doesn't fail, look at recent log entries for `fresh durability probe missed` — if absent, the bug may already be fixed.

### 3. Map the source

```bash
cd /Users/zacfarrell/Desktop/mcp_agent_mail_rust

# Read-side (where errors originate)
rg -n "open_read_db_pool|get_read_db_pool|open_resource_read_pool|get_resource_db_pool|read_pool_setup_error_to_mcp_error|resource_sync_db_error_to_mcp_error|archive_sqlite_degraded_error|db_error_to_mcp_error" crates/

# Write-side (where the false success happens)
rg -n "fresh.durability.probe|durability.probe|message row not visible|recipient rows not visible|verified_sender|wbq|commit_coalescer" crates/

# Verdict / health
rg -n "compute_mailbox_verdict|MailboxArchiveDriftState|verdict_prefers_archive_snapshot|archive_metadata_advantage_is_decisive|scan_archive_message_inventory" crates/

# Recovery triggers
rg -n "reconcile_archive_state_before_init|reconstruct_from_archive|integrity guard|readiness archive drift" crates/

# Diagnostic text
rg -n "Run \`am doctor repair\`|Run \`am doctor reconstruct\`|No database repair action|degraded_read_only" crates/

# Search/inbox/resource entry points
rg -n "pub async fn search_messages|pub async fn fetch_inbox|resource://inbox|resource://thread" crates/

# Path validators
rg -n "validate_sqlite_target_path|traverses symlinked path|symlink_metadata|canonicalize" crates/
```

### 4. Run existing anchor tests (after fixing the linker)

```bash
cargo test -p mcp-agent-mail-tools resources_use_archive_snapshot_when_live_db_is_stale
cargo test -p mcp-agent-mail-tools tool_send_and_reply_success_are_immediately_visible_via_inbox_reads
cargo test -p mcp-agent-mail-tools resource_sync_generic_sqlite_error_stays_database_error
cargo test -p mcp-agent-mail-tools read_pool_setup_error_reports_archive_sqlite_degraded_state
cargo test -p mcp-agent-mail-db reconcile_archive_state_before_init_rebuilds_healthy_db_when_archive_is_ahead
cargo test -p mcp-agent-mail-db reconcile_archive_state_before_init_reconstructs_missing_db
cargo test -p mcp-agent-mail-db archive_recovery_reconciles_healthy_db_when_archive_is_ahead
cargo test -p mcp-agent-mail-db reconstruct_rejects_symlinked_destination_path
cargo test -p mcp-agent-mail-db reconstruct_rejects_symlinked_destination_parent
```

If `tool_send_and_reply_success_are_immediately_visible_via_inbox_reads` exists and **passes**, that is suspicious — the live behavior says it should fail. Inspect the test fixture: it may use the same connection for write and read (which always succeeds), or it may artificially drain the commit coalescer before the read.

### 5. Decide your slice ordering

Based on what you find, commit to Slice A first (lowest risk), then B, then C+D in tandem (they are interrelated), then E. Skip a slice only if your investigation shows it is already done correctly in the live source.

---

## VERIFIED SOURCE ANCHORS (file:line — confirmed against `dc586439` working tree)

This table was generated by greppping the live source. Use it as the single source of truth — the older fix handoff (`mcp-agent-mail-read-search-database-errors-fix-handoff.md`) cited some functions that have since moved.

### Probe and write-side gates (where the false-success bug lives)

| File:line | Symbol | Notes |
|---|---|---|
| `crates/mcp-agent-mail-db/src/queries.rs:1426` | `async fn durability_probe_query(...)` | Opens a fresh handle for the visibility probe |
| `crates/mcp-agent-mail-db/src/queries.rs:1491` | `async fn durability_probe_backoff(attempt: u32)` | Backoff helper for transient busy errors |
| `crates/mcp-agent-mail-db/src/queries.rs:1506` | `async fn verify_agent_visible_after_commit(...)` | **Mode 1 fix site (agent)** |
| `crates/mcp-agent-mail-db/src/queries.rs:1551` | (inside above) | The `Outcome::Ok(())` to remove (agent path) |
| `crates/mcp-agent-mail-db/src/queries.rs:1563` | `fn is_agent_visibility_probe_consistency_error(error: &DbError) -> bool` | Discriminator — keep |
| `crates/mcp-agent-mail-db/src/queries.rs:1673` | `fn is_message_visibility_probe_consistency_error(error: &DbError) -> bool` | Discriminator — keep |
| `crates/mcp-agent-mail-db/src/queries.rs:1769` | `async fn verify_message_recipients_visible_after_commit(...)` | **Mode 1 fix site (message)** |
| `crates/mcp-agent-mail-db/src/queries.rs:1808` | (inside above) | The `Outcome::Ok(())` to remove (message path) |
| `crates/mcp-agent-mail-db/src/queries.rs:1850, :1882` | additional `durability_probe_query` callers | Audit; same shape if they swallow errors |
| `crates/mcp-agent-mail-db/src/queries.rs:4037` | `existing_before` probe | Audit |

### Write-back queue and commit coalescer (the latency window)

| File:line | Symbol | Notes |
|---|---|---|
| `crates/mcp-agent-mail-storage/src/lib.rs:243` | `const WBQ_FLUSH_INTERVAL_MS: u64 = 100` | Floor of the race window |
| `crates/mcp-agent-mail-storage/src/lib.rs:304` | `pub fn wbq_start()` | Bootstrap |
| `crates/mcp-agent-mail-storage/src/lib.rs:401` | `pub fn wbq_enqueue(op: WriteOp) -> WbqEnqueueResult` | Where send_message hands off |
| `crates/mcp-agent-mail-storage/src/lib.rs:430` | `pub fn wbq_flush()` | **Test helper — synchronous drain** |
| `crates/mcp-agent-mail-storage/src/lib.rs:488` | `pub fn wbq_stats() -> WbqStats` | Inspect from tests |
| `crates/mcp-agent-mail-storage/src/lib.rs:1724` | `pub struct CommitCoalescer` | Batches multiple SQL inserts into one COMMIT |
| `crates/mcp-agent-mail-storage/src/lib.rs:3245` | `pub fn get_commit_coalescer() -> &'static CommitCoalescer` | Singleton accessor |
| `crates/mcp-agent-mail-storage/src/lib.rs:3274` | (`get_commit_coalescer().flush_sync()`) | **Test helper — force coalescer drain** |

### Read-side error mapping and snapshot setup

| File:line | Symbol | Notes |
|---|---|---|
| `crates/mcp-agent-mail-tools/src/lib.rs:190` | `pub fn db_error_to_mcp_error(e: DbError) -> McpError` | Generic mapper. **Patch does NOT wire `archive_sqlite_degraded_error` here — Slice A must add this** |
| `crates/mcp-agent-mail-tools/src/lib.rs:375` | `fn read_pool_setup_error_to_mcp_error(message: String) -> McpError` | Snapshot-setup mapper. Patch wires `archive_sqlite_degraded_error` here at lib.rs:379 |
| `crates/mcp-agent-mail-tools/src/lib.rs:387` | `pub(crate) fn archive_sqlite_degraded_error(message: &str) -> Option<McpError>` | NEW in patch |
| `crates/mcp-agent-mail-tools/src/lib.rs:687` | `struct ReadSnapshotDirGuard { path: PathBuf }` | Tool-side snapshot dir |
| `crates/mcp-agent-mail-tools/src/lib.rs:691` | `impl ReadSnapshotDirGuard { fn new(prefix); fn new_in(prefix, base) }` | `new_in` is NEW in patch (canonicalizes after `create_dir`) |
| `crates/mcp-agent-mail-tools/src/lib.rs:774` | `fn open_read_db_pool() -> Result<Option<ToolReadPool>, String>` | Verdict + snapshot setup |
| `crates/mcp-agent-mail-tools/src/lib.rs:843` | call site of `ReadSnapshotDirGuard::new("agent-mail-tool-snapshot-")` | |
| `crates/mcp-agent-mail-tools/src/lib.rs:869` | `pub fn get_read_db_pool() -> McpResult<ToolReadPool>` | Public entry |
| `crates/mcp-agent-mail-tools/src/resources.rs:85` | `fn resource_sync_db_error_to_mcp_error(message: String) -> McpError` | Patch wires `archive_sqlite_degraded_error` here at resources.rs:89 |
| `crates/mcp-agent-mail-tools/src/resources.rs:186` | `fn resource_archive_is_ahead(...)` | Triggers snapshot path |
| `crates/mcp-agent-mail-tools/src/resources.rs:230` | `struct ResourceSnapshotDirGuard { path: PathBuf }` | Resource-side snapshot dir |
| `crates/mcp-agent-mail-tools/src/resources.rs:234` | `impl ResourceSnapshotDirGuard { fn new(prefix); fn new_in(prefix, base) }` | `new_in` is NEW in patch |
| `crates/mcp-agent-mail-tools/src/resources.rs:335` | `fn open_resource_read_pool() -> Result<Option<ResourceReadPool>, String>` | |
| `crates/mcp-agent-mail-tools/src/resources.rs:403` | call site of `ResourceSnapshotDirGuard::new("agent-mail-resource-snapshot-")` | |
| `crates/mcp-agent-mail-tools/src/resources.rs:429` | `fn get_resource_db_pool() -> McpResult<ResourceReadPool>` | Public entry |

### Tool/resource entry points

| File:line | Symbol |
|---|---|
| `crates/mcp-agent-mail-tools/src/messaging.rs:1427` | `pub async fn send_message(...)` |
| `crates/mcp-agent-mail-tools/src/messaging.rs:3020` | `pub async fn fetch_inbox(...)` |
| `crates/mcp-agent-mail-tools/src/messaging.rs:3061` | `let read_pool = get_read_db_pool()?` (use site) |
| `crates/mcp-agent-mail-tools/src/search.rs:713` | `pub async fn search_messages(...)` |
| `crates/mcp-agent-mail-tools/src/search.rs:791, :1017` | `let pool = get_read_db_pool()?` (use sites) |

### Pool, validator, reconstruct

| File:line | Symbol | Notes |
|---|---|---|
| `crates/mcp-agent-mail-db/src/pool.rs:3467` | `pub fn archive_metadata_advantage_is_decisive(...)` | Verdict input; signature: `(archive_projects: usize, archive_agents: usize, archive_messages: usize, archive_latest_id: i64, db_projects: usize, db_agents: usize, db_messages: usize, db_max_id: i64, missing_archive_projects: &[String]) -> bool` |
| `crates/mcp-agent-mail-db/src/pool.rs:3606` | `fn reconcile_archive_state_before_init(primary_path: &Path, storage_root: &Path) -> Result<bool, SqlError>` | Startup-only reconcile |
| `crates/mcp-agent-mail-db/src/pool.rs:3700` | `fn sqlite_validation_anchor(path: &Path) -> Result<PathBuf, SqlError>` | Anchors relative paths to CWD before validation |
| `crates/mcp-agent-mail-db/src/pool.rs:3718` | `pub(crate) fn validate_sqlite_target_path(path: &Path, label: &str) -> Result<(), SqlError>` | **Mode 4 site — walks components, rejects any symlink** |
| `crates/mcp-agent-mail-db/src/pool.rs:4890` | `pub fn resolve_mailbox_sqlite_path(database_url: &str) -> DbResult<ResolvedMailboxSqlitePath>` | Patch reads `.canonical_path` |
| `crates/mcp-agent-mail-db/src/reconstruct.rs` | `pub fn reconstruct_from_archive(db_path, storage_root) -> DbResult<ReconstructStats>` | Calls `validate_sqlite_target_path` |
| `crates/mcp-agent-mail-db/src/reconstruct.rs` | `pub fn reconstruct_from_archive_with_salvage(db_path, storage_root, salvage_db_path: Option<&Path>) -> DbResult<ReconstructStats>` | Used by snapshot path when live DB exists |
| `crates/mcp-agent-mail-db/src/mailbox_verdict.rs` | `compute_mailbox_verdict(...)`, `MailboxArchiveDriftState`, `verdict_prefers_archive_snapshot_reads(...)`, `verdict_prefers_archive_snapshot_reads_for_primary_read_surface(...)` | Verdict engine |

### Diagnostic text surfaces

| File:line | Symbol | Notes |
|---|---|---|
| `crates/mcp-agent-mail-server/src/startup_checks.rs:1749` | `fn format_recovery_context() -> Option<String>` | Patch updates next-action text from `repair` → `reconstruct` |
| `crates/mcp-agent-mail-tools/src/identity.rs:174` | `fn build_recovery_status(config: &Config) -> Option<RecoveryStatusResponse>` | Same text fix |
| `crates/mcp-agent-mail-cli/src/doctor*.rs` | `am doctor` CLI surfaces | Slice E target — find the next-action printer |

---

## RELATED CONTEXT (DO NOT DISCOVER COLD)

These artifacts already exist in `Testproject1` and contain useful background. Read in this order if you need more context:

1. **`prompts/mcp-agent-mail-immediate-read-after-write-regression-error-manifest.md`** — original QA manifest for the visible failure. Reproduction protocol still works; the "intermittent" framing understates the real failure mode.
2. **`prompts/mcp-agent-mail-read-search-database-errors-fix-handoff.md`** — prior implementation handoff for the earlier (2026-05-05) version of the same family of bugs. Contains good source orientation, hypothesis ranking, and the upstream-vs-target comparison strategy. Some claims are now outdated: the partial fix it asks for is what's currently uncommitted at `mcp_agent_mail_rust`. Use it for source map; do not retread its hypotheses without checking against this document.
3. **`docs/handoff-2026-05-05-mcp-agent-mail-db-recovery.md`** — operator-facing post-incident handoff for the original 2026-05-05 SQLite corruption that started this whole investigation. Useful for the reconstruct/forensic bundle layout.
4. **`prompts/mcp-agent-mail-read-search-error-manifest.md`** — older error manifest from 2026-05-05. Same family.
5. **`agent-mail-search-fixes.patch`** (in `Testproject1` root) — adjacent search/pagination patch. Out of scope for this fix but should remain applied/merged.
6. Forensic bundles in `~/.mcp_agent_mail_git_mailbox_repo/doctor/forensics/storage.sqlite3/` — pre-recovery captures with full archive-drift reports. Consult `archive-drift-report.json` and `summary.json` in each bundle.
7. Daemon log `~/Library/Logs/agent-mail/stderr.log` — load-bearing. Note that this log is heavily polluted by an unrelated `ATC operator tick exceeded configured budget` process; filter with `grep -E "(fresh durability|reconstruct|archive inventory|integrity guard|message_id=|drop_close.*storage)"`.
8. Upstream Agent Mail repo: <https://github.com/Dicklesworthstone/mcp_agent_mail_rust>. Check `main` head and any tags/releases newer than `dc586439`.

---

## FILE MAP

| File | Role |
|---|---|
| `prompts/mcp-agent-mail-immediate-read-after-write-fix-handoff.md` | This document |
| `prompts/mcp-agent-mail-immediate-read-after-write-regression-error-manifest.md` | Original QA manifest for the visible failure |
| `prompts/mcp-agent-mail-read-search-database-errors-fix-handoff.md` | Prior fix handoff for the related 2026-05-05 incident |
| `prompts/mcp-agent-mail-read-search-error-manifest.md` | Earlier QA manifest (2026-05-05) for the same family |
| `docs/handoff-2026-05-05-mcp-agent-mail-db-recovery.md` | Operator post-incident handoff |
| `agent-mail-search-fixes.patch` | Tracked search-fixes patch (out of scope; keep applied/merged) |
| `/Users/zacfarrell/.local/bin/am` | Live binary, currently `0.2.51` |
| `/Users/zacfarrell/.mcp_agent_mail_git_mailbox_repo/` | Live storage root (DO NOT MUTATE) |
| `/Users/zacfarrell/.mcp_agent_mail_git_mailbox_repo/storage.sqlite3` | Live SQLite index |
| `/Users/zacfarrell/.mcp_agent_mail_git_mailbox_repo/doctor/forensics/` | Recovery bundles, audit trail |
| `~/Library/Logs/agent-mail/stderr.log` | Daemon log (load-bearing, polluted, grep before reading) |
| `~/Library/LaunchAgents/com.agent-mail.plist` | LaunchAgent definition (KeepAlive, RunAtLoad, env) |
| `/Users/zacfarrell/Desktop/mcp_agent_mail_rust/` | External Rust workspace, dirty working tree on `dc586439` |
| `crates/mcp-agent-mail-tools/src/lib.rs` | Tool-side read pool + `archive_sqlite_degraded_error` patch |
| `crates/mcp-agent-mail-tools/src/resources.rs` | Resource snapshot guard + error mapping patch |
| `crates/mcp-agent-mail-tools/src/messaging.rs` | `send_message`, `fetch_inbox`, fresh-durability-probe call site |
| `crates/mcp-agent-mail-tools/src/search.rs` | `search_messages` |
| `crates/mcp-agent-mail-db/src/pool.rs` | `validate_sqlite_target_path`, `reconcile_archive_state_before_init`, write-back queue / coalescer hooks |
| `crates/mcp-agent-mail-db/src/reconstruct.rs` | `reconstruct_from_archive` and symlink-rejection tests |
| `crates/mcp-agent-mail-db/src/mailbox_verdict.rs` | Verdict engine (drives degraded routing) |
| `crates/mcp-agent-mail-server/src/startup_checks.rs` | Recovery next-action text in startup readiness |
| `crates/mcp-agent-mail-cli/src/doctor*.rs` | `am doctor` CLI surfaces |

---

## QUICK SANITY CHECKLIST BEFORE YOU CALL THE FIX DONE

- [ ] `am --version` reports a new version (e.g., `0.2.52`).
- [ ] LaunchAgent has been bootout/bootstrap-restarted with the new binary.
- [ ] 20-iteration `send → search → fetch → resource://inbox → resource://thread → health_check` loop on `MistyHorse → MistyHorse` returns zero generic errors and `health_check` stays green throughout.
- [ ] Same loop run from a second project (e.g., `mcp_agent_mail_rust`) returns zero errors.
- [ ] Daemon log shows no new `fresh durability probe missed committed message visibility` entries since the new binary started.
- [ ] `am doctor check` does not report `[FAIL] archive_db_parity` after the loop.
- [ ] `health_check.recovery.next_action`, `am doctor check`'s next-action line, and any read-tool `DATABASE_DEGRADED` payload all reference `am doctor reconstruct` (not `repair`) for archive-ahead conditions.
- [ ] Caller-controlled symlinked SQLite paths still get rejected (regression check).
- [ ] All tests added by Slices A–E pass.
- [ ] `cargo clippy --all-targets -- -D warnings` clean.
- [ ] `git diff --check` clean.

When all boxes are checked, write a short post-fix update under `docs/` and reference this handoff.
