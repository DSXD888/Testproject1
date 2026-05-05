# MCP Agent Mail Read/Search Database Errors - QA Error Manifest

ULTRATHINK and be meticulous. This is a forensic debugging document.

## DEFECT FINGERPRINT

| Field | Value |
|-------|-------|
| **Type** | integration_mismatch / persistence_index_failure |
| **Severity** | high |
| **Reproducibility** | environment-specific; captured 2/2 during degraded state on 2026-05-05T21:04Z, no longer reproducing after later archive/SQLite alignment |
| **Affected Component** | MCP Agent Mail server storage/index/read paths |
| **Working Analogue** | `register_agent`, `send_message`, `file_reservation_paths`, `release_file_reservations`, `acknowledge_message` succeeded in the same session |
| **Scope** | Agent Mail write-side operations succeeded, but `search_messages`, `fetch_inbox`, and resource reads failed with database errors while health reported archive inventory ahead of SQLite index |
| **Environment** | macOS 15.5 arm64, MCP Agent Mail development server on `127.0.0.1:8765`, storage at `/Users/alinaaitova/.mcp_agent_mail_git_mailbox_repo/storage.sqlite3` |
| **Git State** | branch `main`, commit `c5a7c24f3f5960573e6cede05dd9c47c363fd82e`, dirty with unrelated untracked docs at time of manifest creation |

---

## REPRODUCTION PROTOCOL

> Follow these steps EXACTLY against an Agent Mail instance in the captured degraded state. Do not corrupt or rewrite live Agent Mail storage to force the state; if the live instance is already healthy, reproduce in an isolated copy or use this manifest as the captured evidence package.

### Prerequisites

- MCP Agent Mail server reachable through MCP tools.
- Project key: `/Users/alinaaitova/Desktop/Testproject1`.
- Agent identity registered as `CrimsonChapel`.
- A self-addressed message exists:
  - id `1`
  - thread `agent-mail-smoke-2026-05-05`
  - subject `[agent-mail-smoke] Self-test message`
- Health check shows degraded archive/SQLite mismatch:
  - archive has project/agent/message rows
  - SQLite index reports zero project/agent/message rows

### Step 1: Confirm degraded health state

MCP tool call:
```json
{"tool":"mcp__mcp_agent_mail__.health_check","arguments":{}}
```

**EXPECTED**: `status: "ok"` and archive/SQLite inventory aligned.

**ACTUAL**:
```json
{
  "status": "error",
  "health_level": "red",
  "semantic_readiness": {
    "status": "fail",
    "detail": "archive inventory is ahead of the sqlite index (archive projects=1, agents=3, messages=3, latest_id=3, db projects=0, agents=0, messages=0, max_id=0, missing archive project(s) in db: users-alinaaitova-desktop-testproject1 (/Users/alinaaitova/Desktop/Testproject1))"
  },
  "recovery": {
    "mode": "degraded_read_only",
    "owner": "none",
    "next_action": "Run `am doctor repair` to attempt automatic recovery",
    "recovery_lock_active": false
  }
}
```

### Step 2: Search for the known self-test message

MCP tool call:
```json
{
  "tool": "mcp__mcp_agent_mail__.search_messages",
  "arguments": {
    "project_key": "/Users/alinaaitova/Desktop/Testproject1",
    "query": "agent-mail-smoke",
    "limit": 10,
    "ranking": "recency"
  }
}
```

**EXPECTED**: one result with message id `1`.

**ACTUAL**:
```text
A database error occurred. This may be a transient issue - try again.
```

### Step 3: Fetch the recipient inbox

MCP tool call:
```json
{
  "tool": "mcp__mcp_agent_mail__.fetch_inbox",
  "arguments": {
    "project_key": "/Users/alinaaitova/Desktop/Testproject1",
    "agent_name": "CrimsonChapel",
    "include_bodies": true,
    "limit": 10
  }
}
```

**EXPECTED**: one inbox message with id `1`.

**ACTUAL**:
```text
A database error occurred. This may be a transient issue - try again.
```

### Step 4: Read inbox through the resource interface

MCP resource read:
```text
resource://inbox/CrimsonChapel?project=/Users/alinaaitova/Desktop/Testproject1&include_bodies=true&limit=10
```

**EXPECTED**: resource payload containing message id `1`.

**ACTUAL**:
```text
resources/read failed for `mcp_agent_mail` (resource://inbox/CrimsonChapel?project=/Users/alinaaitova/Desktop/Testproject1&include_bodies=true&limit=10): Mcp error: -32000: A database error occurred. This may be a transient issue - try again.
```

Detailed error payload:
```json
{
  "error": {
    "type": "DATABASE_ERROR",
    "message": "A database error occurred. This may be a transient issue - try again.",
    "recoverable": true,
    "data": {
      "error_detail": "SQLite error: reconstruct: refusing reconstruct sqlite target /var/folders/z3/784rpxcj23x3_0d8d88c_27h0000gn/T/agent-mail-resource-snapshot-69379_1778015106574028000_0/mailbox.sqlite3 because it traverses symlinked path /var"
    }
  }
}
```

### Step 5: Control - verify write-side operations in the same session

MCP tool call:
```json
{
  "tool": "mcp__mcp_agent_mail__.acknowledge_message",
  "arguments": {
    "project_key": "/Users/alinaaitova/Desktop/Testproject1",
    "agent_name": "CrimsonChapel",
    "message_id": 1
  }
}
```

**EXPECTED**: acknowledgement succeeds or idempotently reports an existing acknowledgement.

**ACTUAL**:
```json
{
  "message_id": 1,
  "acknowledged": true,
  "acknowledged_at": "2026-05-05T21:05:16.344952Z",
  "read_at": "2026-05-05T21:05:16.344952Z"
}
```

---

## EVIDENCE BUNDLE

### E0 - Environment Evidence

**E0.1**: Repository path.
```bash
pwd
```
```text
/Users/alinaaitova/Desktop/Testproject1
```

**E0.2**: Branch.
```bash
git rev-parse --abbrev-ref HEAD
```
```text
main
```

**E0.3**: Commit.
```bash
git rev-parse HEAD
```
```text
c5a7c24f3f5960573e6cede05dd9c47c363fd82e
```

**E0.4**: Worktree state at manifest creation.
```bash
git status --short
```
```text
?? docs/handoff-2026-05-05-mcp-agent-mail-db-recovery.md
?? docs/session-log-2026-05-05.md
```

**E0.5**: Operating system.
```bash
sw_vers
```
```text
ProductName:		macOS
ProductVersion:		15.5
BuildVersion:		24F74
```

**E0.6**: Kernel and architecture.
```bash
uname -a
```
```text
Darwin Mac.fritz.box 24.5.0 Darwin Kernel Version 24.5.0: Tue Apr 22 19:54:26 PDT 2025; root:xnu-11417.121.6~2/RELEASE_ARM64_T8112 arm64
```

**E0.7**: Agent Mail CLI location.
```bash
command -v am
```
```text
/Users/alinaaitova/.local/bin/am
```

### E1 - Working Write-Side Evidence

**E1.1**: Agent registration succeeded.
```json
{"tool":"mcp__mcp_agent_mail__.register_agent","arguments":{"project_key":"/Users/alinaaitova/Desktop/Testproject1","program":"codex-cli","model":"gpt-5","task_description":"Testing MCP Agent Mail registration, messages, reservations, and search"}}
```
```json
{"id":2,"name":"CrimsonChapel","program":"codex-cli","model":"gpt-5","project_id":1,"capabilities":["send_message","fetch_inbox","file_reservation_paths","acknowledge_message"]}
```

**E1.2**: Sending a self-addressed message succeeded.
```json
{"tool":"mcp__mcp_agent_mail__.send_message","arguments":{"project_key":"/Users/alinaaitova/Desktop/Testproject1","sender_name":"CrimsonChapel","to":["CrimsonChapel"],"subject":"[agent-mail-smoke] Self-test message","thread_id":"agent-mail-smoke-2026-05-05","importance":"normal","ack_required":true}}
```
```json
{"deliveries":[{"project":"/Users/alinaaitova/Desktop/Testproject1","payload":{"id":1,"thread_id":"agent-mail-smoke-2026-05-05","subject":"[agent-mail-smoke] Self-test message","from":"CrimsonChapel","to":["CrimsonChapel"]}}],"count":1,"verified_sender":true}
```

**E1.3**: File reservation and release succeeded.
```json
{"tool":"mcp__mcp_agent_mail__.file_reservation_paths","arguments":{"project_key":"/Users/alinaaitova/Desktop/Testproject1","agent_name":"CrimsonChapel","paths":["AGENTS.md"],"ttl_seconds":600,"exclusive":true,"reason":"agent-mail-smoke-2026-05-05"}}
```
```json
{"granted":[{"id":1,"path_pattern":"AGENTS.md","exclusive":true,"reason":"agent-mail-smoke-2026-05-05","expires_ts":"2026-05-05T21:14:29.868664Z"}],"conflicts":[]}
```

Release call:
```json
{"tool":"mcp__mcp_agent_mail__.release_file_reservations","arguments":{"project_key":"/Users/alinaaitova/Desktop/Testproject1","agent_name":"CrimsonChapel","paths":["AGENTS.md"]}}
```
```json
{"released":1,"released_at":"2026-05-05T21:04:32.314686Z"}
```

### E2 - Broken Read/Search Evidence

**E2.1**: Health check showed archive ahead of SQLite index.
```json
{"tool":"mcp__mcp_agent_mail__.health_check","arguments":{}}
```
```json
{
  "status": "error",
  "health_level": "red",
  "semantic_readiness": {
    "status": "fail",
    "detail": "archive inventory is ahead of the sqlite index (archive projects=1, agents=3, messages=3, latest_id=3, db projects=0, agents=0, messages=0, max_id=0, missing archive project(s) in db: users-alinaaitova-desktop-testproject1 (/Users/alinaaitova/Desktop/Testproject1))"
  },
  "recovery": {
    "mode": "degraded_read_only",
    "owner": "none",
    "next_action": "Run `am doctor repair` to attempt automatic recovery",
    "recovery_lock_active": false
  }
}
```

**E2.2**: `search_messages` failed for the known message.
```json
{
  "tool": "mcp__mcp_agent_mail__.search_messages",
  "arguments": {
    "project_key": "/Users/alinaaitova/Desktop/Testproject1",
    "query": "agent-mail-smoke",
    "limit": 10,
    "ranking": "recency"
  }
}
```
```text
A database error occurred. This may be a transient issue - try again.
```

**E2.3**: `fetch_inbox` failed for the known recipient.
```json
{
  "tool": "mcp__mcp_agent_mail__.fetch_inbox",
  "arguments": {
    "project_key": "/Users/alinaaitova/Desktop/Testproject1",
    "agent_name": "CrimsonChapel",
    "include_bodies": true,
    "limit": 10
  }
}
```
```text
A database error occurred. This may be a transient issue - try again.
```

**E2.4**: Resource inbox read failed with a symlink traversal detail.
```text
resource://inbox/CrimsonChapel?project=/Users/alinaaitova/Desktop/Testproject1&include_bodies=true&limit=10
```
```json
{
  "error": {
    "type": "DATABASE_ERROR",
    "message": "A database error occurred. This may be a transient issue - try again.",
    "recoverable": true,
    "data": {
      "error_detail": "SQLite error: reconstruct: refusing reconstruct sqlite target /var/folders/z3/784rpxcj23x3_0d8d88c_27h0000gn/T/agent-mail-resource-snapshot-69379_1778015106574028000_0/mailbox.sqlite3 because it traverses symlinked path /var"
    }
  }
}
```

**E2.5**: Resource thread read failed with the same symlink traversal detail.
```text
resource://thread/agent-mail-smoke-2026-05-05?project=/Users/alinaaitova/Desktop/Testproject1&include_bodies=true&limit=10
```
```json
{
  "error": {
    "type": "DATABASE_ERROR",
    "message": "A database error occurred. This may be a transient issue - try again.",
    "recoverable": true,
    "data": {
      "error_detail": "SQLite error: reconstruct: refusing reconstruct sqlite target /var/folders/z3/784rpxcj23x3_0d8d88c_27h0000gn/T/agent-mail-resource-snapshot-69379_1778015106589886000_1/mailbox.sqlite3 because it traverses symlinked path /var"
    }
  }
}
```

### E3 - Recovery Evidence

**E3.1**: Later health check showed archive and SQLite aligned.
```json
{"tool":"mcp__mcp_agent_mail__.health_check","arguments":{}}
```
```json
{
  "status": "ok",
  "health_level": "green",
  "semantic_readiness": {
    "status": "ok",
    "detail": "Archive and sqlite inventory are aligned enough for health_check: archive projects=1, agents=3, messages=3, db projects=1, agents=3, messages=3"
  }
}
```

**E3.2**: `search_messages` succeeded after alignment.
```json
{"tool":"mcp__mcp_agent_mail__.search_messages","arguments":{"project_key":"/Users/alinaaitova/Desktop/Testproject1","query":"agent-mail-smoke","limit":10,"ranking":"recency"}}
```
```json
{"result":[{"id":1,"subject":"[agent-mail-smoke] Self-test message","importance":"normal","ack_required":1,"created_ts":"2026-05-05T21:04:24.536214Z","thread_id":"agent-mail-smoke-2026-05-05","from":"CrimsonChapel","to":["CrimsonChapel"],"cc":[]}]}
```

**E3.3**: `fetch_inbox` succeeded after alignment.
```json
{"tool":"mcp__mcp_agent_mail__.fetch_inbox","arguments":{"project_key":"/Users/alinaaitova/Desktop/Testproject1","agent_name":"CrimsonChapel","include_bodies":true,"limit":10}}
```
```json
[{"id":1,"project_id":1,"sender_id":1,"thread_id":"agent-mail-smoke-2026-05-05","subject":"[agent-mail-smoke] Self-test message","importance":"normal","ack_required":true,"from":"CrimsonChapel","created_ts":"2026-05-05T21:04:24.536214Z","kind":"to","attachments":[],"body_md":"Smoke test from Codex on 2026-05-05. This verifies send_message, inbox delivery, and search indexing for the current project."}]
```

**E3.4**: Resource inbox read succeeded after alignment.
```text
resource://inbox/CrimsonChapel?project=/Users/alinaaitova/Desktop/Testproject1&include_bodies=true&limit=10
```
```json
{"agent":"CrimsonChapel","count":1,"messages":[{"id":1,"thread_id":"agent-mail-smoke-2026-05-05","subject":"[agent-mail-smoke] Self-test message","importance":"normal","ack_required":true,"from":"CrimsonChapel","kind":"to"}],"project":"/Users/alinaaitova/Desktop/Testproject1"}
```

---

## CODEPATH COMPARISON

The affected implementation is the external MCP Agent Mail server, not this repository's Rust/TypeScript source tree. No local source files were inspected or modified for the defect. The comparison below is therefore an operation-path comparison grounded in MCP behavior.

### Working Path: Write-Side Agent Mail Operations

**Entry points**:
- `register_agent`
- `send_message`
- `file_reservation_paths`
- `release_file_reservations`
- `acknowledge_message`

**Flow**:
1. Register project/agent identity.
2. Write message or reservation to Agent Mail storage.
3. Return concrete ids and timestamps.

**Observed behavior**:
- Agent `CrimsonChapel` registered successfully (E1.1).
- Message id `1` was delivered with `verified_sender: true` (E1.2).
- File reservation id `1` was granted and released (E1.3).
- Message id `1` was acknowledged (Step 5).

### Broken Path: Read/Search/Resource Operations During Degraded State

**Entry points**:
- `search_messages`
- `fetch_inbox`
- `resource://inbox/...`
- `resource://thread/...`

**Flow inferred from errors**:
1. Read operation depends on SQLite/search snapshot state.
2. Health reports archive inventory ahead of SQLite index.
3. Read operation returns a generic database error instead of serving from archive, waiting for repair, or surfacing the degraded-state diagnostic.
4. Resource reads additionally attempt to reconstruct a SQLite snapshot under macOS `/var/folders/...` and reject the target because `/var` is symlinked.

### Divergence Points

| # | Working Path | Broken Path | Potential Impact |
|---|--------------|-------------|------------------|
| 1 | Writes create visible ids and timestamps | Reads/search depend on SQLite index that can lag archive | Freshly written messages can exist but be unreadable/search-invisible |
| 2 | Write tools returned precise operation results | Read tools returned generic transient database error | Debugger lacks actionable repair/root-cause detail unless checking health/resource payload |
| 3 | File reservation tool worked through degraded state | Resource read for file reservations initially failed through snapshot reconstruction | Tool and resource surfaces can disagree for the same domain |
| 4 | Normalized storage path under user home worked | Resource snapshot path under `/var/folders/...` was rejected due to `/var` symlink traversal | macOS temp directory handling can break resource reads independently of archive/SQLite alignment |

---

## ROOT CAUSE HYPOTHESES

### H1: SQLite index reconstruction lag or failure left archive and DB out of sync - LIKELIHOOD: HIGH

**Description**: The archive contained project, agent, and message records, but `storage.sqlite3` was empty from the server's perspective. Read/search tools depend on SQLite and failed until the archive/SQLite inventory became aligned.

**Supporting evidence**: E2.1, E2.2, E2.3, E3.1, E3.2, E3.3.

**Contradicting evidence**: Write-side tools succeeded during the degraded state (E1.1-E1.3), which means the entire server was not down.

**Verification steps**:
1. Inspect Agent Mail server logs around `2026-05-05T21:04Z` for archive reconcile, SQLite open, migration, or write-back queue events.
2. Inspect the code that computes `semantic_readiness` and the code that serves `search_messages` / `fetch_inbox`.
3. Confirm whether read/search paths have an archive fallback when SQLite is behind.
4. Reproduce in an isolated Agent Mail storage copy with archive records present and an empty/stale SQLite index.

**Fix scope**: Agent Mail server archive reconciliation, SQLite index rebuild, and read/search fallback/error handling.

### H2: Resource snapshot reconstruction rejects normal macOS temp paths because `/var` is symlinked - LIKELIHOOD: HIGH

**Description**: Resource reads attempted to reconstruct a SQLite snapshot below `/var/folders/...` and refused the target because `/var` traverses a symlink. On macOS, `/var` commonly points to `/private/var`; rejecting this path can break resource reads even when the path is a normal system temp location.

**Supporting evidence**: E2.4, E2.5. Later resource reads succeeded after recovery (E3.4), but the earlier detailed error explicitly named the symlink traversal check.

**Contradicting evidence**: Tool-based `search_messages` and `fetch_inbox` failed too, without the symlink detail (E2.2, E2.3). This means the symlink issue may be resource-specific or secondary.

**Verification steps**:
1. Inspect resource snapshot reconstruction code for symlink traversal checks.
2. Confirm the temp directory source on macOS, likely `TMPDIR=/var/folders/...`.
3. Test resource reads with a temp directory under `/private/var/folders/...` or a canonicalized path.
4. Verify the safety check still prevents traversal outside the intended snapshot root after canonicalization.

**Fix scope**: Agent Mail resource snapshot reconstruction path validation and macOS temp directory canonicalization.

### H3: Error reporting masks actionable health diagnostics - LIKELIHOOD: MEDIUM

**Description**: `search_messages` and `fetch_inbox` returned only "A database error occurred. This may be a transient issue - try again." The health endpoint already knew the specific issue and the next action. Read/search callers should receive enough structured detail to distinguish transient query failure from degraded archive/SQLite mismatch.

**Supporting evidence**: E2.1, E2.2, E2.3, E2.4.

**Contradicting evidence**: Resource reads did include a detailed `error_detail` under `data` (E2.4, E2.5), so some paths already preserve low-level detail.

**Verification steps**:
1. Trace error mapping for `search_messages` and `fetch_inbox`.
2. Compare it with resource error mapping that returned `DATABASE_ERROR` plus `error_detail`.
3. Confirm whether degraded health diagnostics can be attached safely without exposing sensitive local paths beyond what health already exposes.

**Fix scope**: MCP tool error mapping for read/search operations.

---

## INVESTIGATION ROADMAP

### Step 1: Verify H1

Inspect Agent Mail server logs and code for archive reconciliation and SQLite index rebuild. Look for the transition from:

```text
archive projects=1, agents=3, messages=3; db projects=0, agents=0, messages=0
```

to:

```text
archive projects=1, agents=3, messages=3; db projects=1, agents=3, messages=3
```

**If confirmed** -> add regression coverage for read/search behavior when archive is ahead of SQLite, then implement repair/fallback/error handling.

**If rejected** -> proceed to Step 2.

### Step 2: Verify H2

Inspect the resource snapshot reconstruction code for a symlink traversal guard. Run resource reads on macOS with `TMPDIR` resolving through `/var/folders` and again with a canonical `/private/var/folders` path.

**If confirmed** -> canonicalize or relocate the temp snapshot root while preserving traversal safety.

**If rejected** -> proceed to Step 3.

### Step 3: Verify H3

Inspect how tool errors are converted into MCP responses for `search_messages` and `fetch_inbox`. Compare with the resource read path that included `error_detail`.

**If confirmed** -> preserve structured diagnostics in read/search tool errors, including semantic readiness state when available.

**If rejected** -> re-examine database connection acquisition and query execution around degraded recovery mode.

---

## FIX CONSTRAINTS

1. Do not require users or agents to run destructive cleanup against Agent Mail storage.
2. Do not lose or rewrite archive messages, agents, projects, or reservations during repair/reconciliation.
3. Preserve write-side behavior that already works: registration, send, reservations, release, and acknowledgement.
4. Resource snapshot path validation must remain traversal-safe after any macOS symlink/canonicalization fix.
5. Search and inbox reads should not report success unless returned data is backed by the authoritative archive/SQLite state.
6. Error messages should be actionable but must avoid leaking more local filesystem detail than existing health/resource diagnostics already expose.

---

## ACCEPTANCE CRITERIA

### Functional

- [ ] When a message is successfully sent, `search_messages` can find it by a subject/body token.
- [ ] When a message is successfully sent to an agent, `fetch_inbox` returns it for that agent.
- [ ] `resource://inbox/{agent}` and `resource://thread/{thread_id}` return the same message data as the tools.
- [ ] File reservation resources and tools agree after reservation and release operations.
- [ ] If archive is ahead of SQLite, the server either repairs/rebuilds automatically or returns a structured degraded-state error with the specific archive/SQLite inventory mismatch.

### Evidence-Based

- [ ] The health check reports aligned counts:
  ```text
  archive projects=1, agents=3, messages=3, db projects=1, agents=3, messages=3
  ```
- [ ] This command returns message id `1` or the equivalent newly created smoke-test message:
  ```json
  {"tool":"mcp__mcp_agent_mail__.search_messages","arguments":{"project_key":"/Users/alinaaitova/Desktop/Testproject1","query":"agent-mail-smoke","limit":10,"ranking":"recency"}}
  ```
- [ ] This command returns the same message in the recipient inbox:
  ```json
  {"tool":"mcp__mcp_agent_mail__.fetch_inbox","arguments":{"project_key":"/Users/alinaaitova/Desktop/Testproject1","agent_name":"CrimsonChapel","include_bodies":true,"limit":10}}
  ```
- [ ] Resource reads on macOS do not fail with:
  ```text
  refusing reconstruct sqlite target /var/folders/... because it traverses symlinked path /var
  ```

### Regression

- [ ] `register_agent` continues to succeed.
- [ ] `send_message` continues to return a concrete message id and verified sender status when a valid token is supplied.
- [ ] `file_reservation_paths` and `release_file_reservations` continue to succeed.
- [ ] `acknowledge_message` remains idempotent.
- [ ] Health check remains green after a full smoke cycle.

### Test

- [ ] Add a regression test for read/search behavior when archive inventory is ahead of the SQLite index.
- [ ] Add a macOS-path-focused test for resource snapshot reconstruction with a temp path that enters through `/var/folders`.
- [ ] Add a tool error-mapping test ensuring `search_messages` and `fetch_inbox` include actionable diagnostics for degraded read/search failures.

---

## FILE MAP

| File | Role | Lines Referenced |
|------|------|------------------|
| `prompts/mcp-agent-mail-read-search-error-manifest.md` | This forensic handoff document | all |
| `/Users/alinaaitova/.mcp_agent_mail_git_mailbox_repo/storage.sqlite3` | Agent Mail SQLite index mentioned in health output | external runtime file |
| `/Users/alinaaitova/.mcp_agent_mail_git_mailbox_repo/projects` | Agent Mail archive project storage listed in environment evidence | external runtime directory |
| `/Users/alinaaitova/.mcp_agent_mail_git_mailbox_repo/search_index` | Agent Mail search index directory listed in environment evidence | external runtime directory |
