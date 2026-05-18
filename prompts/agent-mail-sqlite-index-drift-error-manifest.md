# Agent Mail SQLite Index Drift - QA Error Manifest

ULTRATHINK and be meticulous. This is a forensic debugging document.

## DEFECT FINGERPRINT

| Field | Value |
|-------|-------|
| **Type** | integration_mismatch / silent_failure |
| **Severity** | high |
| **Reproducibility** | deterministic |
| **Affected Component** | Agent Mail MCP server storage/index recovery |
| **Working Analogue** | Archive-backed Agent Mail operations: register agent, list agents, send message, fetch inbox, search messages, file reservations |
| **Scope** | Agent Mail MCP functional calls succeed, but `health_check` reports red because archive inventory is ahead of the SQLite index. |
| **Environment** | macOS Darwin 24.5.0 arm64, `am 0.2.54`, SQLite 3.43.2 |
| **Git State** | Repo branch `main`, HEAD `0caa0ce10701fb266151605c41919fd0f53ce4bd`, dirty working tree with unrelated untracked files |

---

## SKILL INVENTORY FINDING

The wrapper-related skill exists and is named `install-bead-wrappers`.

That skill is not the repair path for this defect. Its job is to bootstrap or refresh project-local bead/Agent Mail discipline infrastructure:

- `bin/br-claim`
- `bin/br-complete`
- AGENTS.md Rule 2
- Agent Mail guard hooks via `am guard install`

The current project already has the relevant infrastructure installed:

```bash
ls -la bin
```

```text
br-claim
br-complete
br-sync
```

```bash
am guard status /Users/alinaaitova/Desktop/Testproject1
```

```text
Guard Status:
  Hooks dir            /Users/alinaaitova/Desktop/Testproject1/.git/hooks
  Mode                 Block
  Worktrees            true
  Pre-commit           installed
  Pre-push             installed
```

The local wrappers do not match the skill asset hashes:

```bash
cmp -s bin/br-claim /Users/alinaaitova/.codex/skills/install-bead-wrappers/assets/br-claim; echo br-claim-cmp:$?
cmp -s bin/br-complete /Users/alinaaitova/.codex/skills/install-bead-wrappers/assets/br-complete; echo br-complete-cmp:$?
```

```text
br-claim-cmp:1
br-complete-cmp:1
```

Per the skill itself, rerunning it in a project with customized wrappers can overwrite local customizations. It also does not reconstruct Agent Mail's SQLite database or repair an archive/index drift. Use this skill later only if the desired action is explicitly to refresh wrapper assets and the local customization risk is accepted.

---

## REPRODUCTION PROTOCOL

Follow these steps exactly. They reproduce the defect without applying any repair.

### Prerequisites

- Agent Mail HTTP server is running on `127.0.0.1:8765`.
- Project path is `/Users/alinaaitova/Desktop/Testproject1`.
- Storage root is `/Users/alinaaitova/.mcp_agent_mail_git_mailbox_repo`.

### Step 1: Verify the lightweight HTTP health endpoint is alive

```bash
curl -s --max-time 3 http://127.0.0.1:8765/healthz
```

**Actual**:

```text
{"status":"alive"}
```

### Step 2: Call the Agent Mail MCP health check

MCP tool call:

```text
health_check({})
```

**Expected**: green/ready semantic health when Agent Mail can register agents, write messages, and manage reservations.

**Actual**: status is red and semantic readiness fails:

```json
{
  "status": "error",
  "health_level": "red",
  "database_url": "sqlite:////Users/alinaaitova/.mcp_agent_mail_git_mailbox_repo/storage.sqlite3",
  "semantic_readiness": {
    "status": "fail",
    "detail": "archive inventory is ahead of the sqlite index (archive projects=1, agents=2, messages=1, latest_id=1, db projects=1, agents=0, messages=0, max_id=0)"
  },
  "recovery": {
    "mode": "degraded_read_only",
    "next_action": "Run `am doctor reconstruct --dry-run` to inspect archive-backed recovery, then `am doctor reconstruct --yes` if needed"
  }
}
```

### Step 3: Confirm Agent Mail operations still work through MCP tools

MCP tool calls:

```text
list_agents(project_key="/Users/alinaaitova/Desktop/Testproject1")
fetch_inbox(project_key="/Users/alinaaitova/Desktop/Testproject1", agent_name="MaroonMountain", include_bodies=true, limit=10)
search_messages(project_key="/Users/alinaaitova/Desktop/Testproject1", query="agentmail-smoke-20260518T1020Z", include_body_md=true, ranking="recency", limit=10)
```

**Expected**: functional paths should return data.

**Actual**:

```json
[
  {
    "name": "MaroonMountain",
    "program": "codex-cli",
    "model": "gpt-5",
    "task_description": "Agent Mail MCP smoke test: registration, inbox, messaging, reservations",
    "contact_policy": "auto"
  },
  {
    "name": "JadeMink",
    "program": "codex-desktop",
    "model": "gpt-5",
    "task_description": "Commit and sync current workspace docs changes",
    "contact_policy": "auto"
  }
]
```

```json
[
  {
    "id": 1,
    "thread_id": "agentmail-smoke-20260518T1020Z",
    "subject": "[agentmail-smoke-20260518T1020Z] Self-test message",
    "ack_required": true,
    "from": "MaroonMountain",
    "body_md": "Agent Mail MCP smoke test self-message. Marker: agentmail-smoke-20260518T1020Z. This verifies send_message delivery, sender verification, inbox retrieval, and searchability."
  }
]
```

```json
{
  "result": [
    {
      "id": 1,
      "subject": "[agentmail-smoke-20260518T1020Z] Self-test message",
      "thread_id": "agentmail-smoke-20260518T1020Z",
      "from": "MaroonMountain"
    }
  ]
}
```

### Step 4: Confirm raw SQLite counts disagree with archive-backed operations

```bash
sqlite3 /Users/alinaaitova/.mcp_agent_mail_git_mailbox_repo/storage.sqlite3 "SELECT 'projects', count(*) FROM projects UNION ALL SELECT 'agents', count(*) FROM agents UNION ALL SELECT 'messages', count(*) FROM messages UNION ALL SELECT 'message_recipients', count(*) FROM message_recipients UNION ALL SELECT 'file_reservations', count(*) FROM file_reservations UNION ALL SELECT 'file_reservation_releases', count(*) FROM file_reservation_releases;"
```

**Expected**: SQLite should contain the same agents/messages visible through Agent Mail operations.

**Actual**:

```text
projects|1
agents|0
messages|0
message_recipients|0
file_reservations|0
file_reservation_releases|0
```

### Step 5: Confirm archive files exist for the missing SQLite rows

```bash
find /Users/alinaaitova/.mcp_agent_mail_git_mailbox_repo/projects/users-alinaaitova-desktop-testproject1 -maxdepth 5 -type f | sed 's#^/Users/alinaaitova/.mcp_agent_mail_git_mailbox_repo/##' | sort | head -120
```

**Expected**: archive has the records health_check claims are missing from SQLite.

**Actual**:

```text
projects/users-alinaaitova-desktop-testproject1/agents/JadeMink/profile.json
projects/users-alinaaitova-desktop-testproject1/agents/MaroonMountain/profile.json
projects/users-alinaaitova-desktop-testproject1/file_reservations/5aa7ad26df08e82dc7c19119714d656b24684631.json
projects/users-alinaaitova-desktop-testproject1/file_reservations/c567beea3aa2ef2f2d7b2749efe51d361ee90df5.json
projects/users-alinaaitova-desktop-testproject1/file_reservations/id-1.json
projects/users-alinaaitova-desktop-testproject1/messages/2026/05/2026-05-18T10-20-26Z__agentmail-smoke-20260518t1020z-self-test-message__1.md
projects/users-alinaaitova-desktop-testproject1/messages/threads/agentmail-smoke-20260518t1020z.md
projects/users-alinaaitova-desktop-testproject1/project.json
```

### Step 6: Inspect reconstruction without mutating the database

```bash
am doctor reconstruct --dry-run
```

**Expected**: dry run should identify whether archive-backed reconstruction can repair the index.

**Actual**:

```text
Dry run - scanning archive at /Users/alinaaitova/.mcp_agent_mail_git_mailbox_repo
Would recover from archive:
  Projects:      1
  Agents:        2
  Messages:      1
  Thread digests: 1
  Database path: /Users/alinaaitova/.mcp_agent_mail_git_mailbox_repo/storage.sqlite3
No changes made.
```

---

## EVIDENCE BUNDLE

### E0 - Environment Evidence

**E0.1: Runtime and tool versions**

```bash
date -u '+%Y-%m-%dT%H:%M:%SZ' && uname -a && am --version || true && sqlite3 --version || true
```

```text
2026-05-18T10:27:29Z
Darwin Mac.fritz.box 24.5.0 Darwin Kernel Version 24.5.0: Tue Apr 22 19:54:26 PDT 2025; root:xnu-11417.121.6~2/RELEASE_ARM64_T8112 arm64
am 0.2.54
3.43.2 2023-10-10 13:08:14 1b37c146ee9ebb7acd0160c0ab1fd11017a419fa8a3187386ed8cb32b709aapl (64-bit)
```

**E0.2: Git state**

```bash
git rev-parse --abbrev-ref HEAD && git rev-parse HEAD && git status --short | wc -l
```

```text
main
0caa0ce10701fb266151605c41919fd0f53ce4bd
13
```

**E0.3: Lightweight HTTP service health is alive**

```bash
curl -s --max-time 3 http://127.0.0.1:8765/healthz
```

```text
{"status":"alive"}
```

### E1 - Defect Characterization Evidence

**E1.1: Semantic health is red**

MCP tool:

```text
health_check({})
```

```text
status=error
health_level=red
semantic_readiness.status=fail
semantic_readiness.detail=archive inventory is ahead of the sqlite index (archive projects=1, agents=2, messages=1, latest_id=1, db projects=1, agents=0, messages=0, max_id=0)
recovery.mode=degraded_read_only
```

**E1.2: Dry-run reconstruction agrees with archive inventory**

```bash
am doctor reconstruct --dry-run
```

```text
Would recover from archive:
  Projects:      1
  Agents:        2
  Messages:      1
  Thread digests: 1
```

### E2 - Reproduction Evidence

**E2.1: Functional Agent Mail list/fetch/search paths see archive records**

MCP tools:

```text
list_agents(...)
fetch_inbox(...)
search_messages(...)
```

```text
list_agents returns MaroonMountain and JadeMink.
fetch_inbox returns message id=1, thread_id=agentmail-smoke-20260518T1020Z.
search_messages returns message id=1 for query agentmail-smoke-20260518T1020Z.
```

**E2.2: SQLite index tables do not contain those records**

```bash
sqlite3 /Users/alinaaitova/.mcp_agent_mail_git_mailbox_repo/storage.sqlite3 "SELECT 'projects', count(*) FROM projects UNION ALL SELECT 'agents', count(*) FROM agents UNION ALL SELECT 'messages', count(*) FROM messages UNION ALL SELECT 'message_recipients', count(*) FROM message_recipients UNION ALL SELECT 'file_reservations', count(*) FROM file_reservations UNION ALL SELECT 'file_reservation_releases', count(*) FROM file_reservation_releases;"
```

```text
projects|1
agents|0
messages|0
message_recipients|0
file_reservations|0
file_reservation_releases|0
```

**E2.3: Archive contains records missing from SQLite**

```bash
find /Users/alinaaitova/.mcp_agent_mail_git_mailbox_repo/projects/users-alinaaitova-desktop-testproject1 -maxdepth 5 -type f | sed 's#^/Users/alinaaitova/.mcp_agent_mail_git_mailbox_repo/##' | sort | head -120
```

```text
projects/users-alinaaitova-desktop-testproject1/agents/JadeMink/profile.json
projects/users-alinaaitova-desktop-testproject1/agents/MaroonMountain/profile.json
projects/users-alinaaitova-desktop-testproject1/messages/2026/05/2026-05-18T10-20-26Z__agentmail-smoke-20260518t1020z-self-test-message__1.md
projects/users-alinaaitova-desktop-testproject1/messages/threads/agentmail-smoke-20260518t1020z.md
```

### E3 - Wrapper Skill Evidence

**E3.1: `install-bead-wrappers` is present but does not target this defect**

```bash
sed -n '1,40p' /Users/alinaaitova/.codex/skills/install-bead-wrappers/SKILL.md
```

```text
name: install-bead-wrappers
description: Bootstrap mail-discipline infrastructure in any git project. Copies bin/br-claim and bin/br-complete wrappers, appends AGENTS.md Rule 2, installs the agent-mail precommit hook via `am guard install`. Idempotent - safe to re-run. Run once per new project.
```

**E3.2: Project guard infrastructure is already installed**

```bash
am guard status /Users/alinaaitova/Desktop/Testproject1
```

```text
Pre-commit           installed
Pre-push             installed
```

**E3.3: Local wrappers differ from skill assets**

```bash
cmp -s bin/br-claim /Users/alinaaitova/.codex/skills/install-bead-wrappers/assets/br-claim; echo br-claim-cmp:$?
cmp -s bin/br-complete /Users/alinaaitova/.codex/skills/install-bead-wrappers/assets/br-complete; echo br-complete-cmp:$?
```

```text
br-claim-cmp:1
br-complete-cmp:1
```

---

## CODEPATH COMPARISON

### Working Path: Archive-backed MCP operations

**Entry points**: Agent Mail MCP tools.

**Flow**:

1. `register_agent` creates or updates an agent identity.
2. `send_message` writes a canonical archive message and delivers it to inbox views.
3. `fetch_inbox` returns the new message.
4. `search_messages` returns the new message by marker.
5. `file_reservation_paths` creates a reservation and `release_file_reservations` releases it.

**Observed result**: These operations returned successful payloads during the smoke test.

### Broken Path: SQLite semantic readiness/index state

**Entry points**:

- MCP `health_check`
- Direct SQLite reads of `/Users/alinaaitova/.mcp_agent_mail_git_mailbox_repo/storage.sqlite3`
- `am doctor reconstruct --dry-run`

**Flow**:

1. `health_check` compares archive inventory with SQLite index state.
2. Archive inventory reports agents and messages.
3. SQLite index reports zero agents and zero messages.
4. Health enters `red` with `recovery.mode=degraded_read_only`.

**Observed result**: The database index is stale relative to archive files and functional MCP reads.

### Divergence Points

| # | Working Path | Broken Path | Potential Impact |
|---|--------------|-------------|------------------|
| 1 | Archive contains `MaroonMountain`, `JadeMink`, and message id 1. | SQLite `agents` and `messages` tables contain zero rows. | Any tool depending strictly on SQLite can see stale or empty state. |
| 2 | MCP reads/search can return the test records. | `health_check` reports semantic readiness failure. | System is partially usable but not healthy for coordinated multi-agent work. |
| 3 | `am doctor reconstruct --dry-run` can enumerate recoverable archive records. | Reconstruction has not been applied. | The system remains in degraded recovery mode until repaired. |

---

## ROOT CAUSE HYPOTHESES

### H1: SQLite index reconstruction is pending after prior database corruption or replacement - LIKELIHOOD: HIGH

**Description**: The archive is the durable source for agents/messages, but `storage.sqlite3` is missing rows. Existing `storage.sqlite3.corrupt-*` files and `am doctor reconstruct --dry-run` output suggest the database was replaced or recovered without replaying archive records into SQLite.

**Supporting evidence**: E1.1, E1.2, E2.2, E2.3.

**Contradicting evidence**: None observed.

**Verification steps**:

1. Inspect the Agent Mail doctor/recovery code that produces `reconstruct-*` bundles.
2. Run `am doctor reconstruct --dry-run` again immediately before repair.
3. If approved, run `am doctor reconstruct --yes`.
4. Re-run `health_check` and direct SQLite counts.

**Fix scope**: Agent Mail recovery/doctor path, not this repo's `bin/br-claim` or `bin/br-complete` wrappers.

### H2: MCP write path allows archive writes while recovery mode prevents SQLite indexing - LIKELIHOOD: MEDIUM

**Description**: The service reports `recovery.mode=degraded_read_only`, but smoke-test operations successfully created archive-backed agent/message/reservation artifacts. If the service permits writes in this mode without updating SQLite, every write can increase archive/index drift.

**Supporting evidence**: E1.1, E2.1, E2.2, E2.3.

**Contradicting evidence**: The exact server write implementation was not inspected in this manifest.

**Verification steps**:

1. Find the Agent Mail write path for `register_agent`, `send_message`, and `file_reservation_paths`.
2. Check whether degraded recovery mode intentionally routes writes to archive only.
3. Confirm whether SQLite indexing failures are logged, swallowed, queued, or intentionally skipped.
4. Add or run a regression test that forces recovery mode and attempts a write.

**Fix scope**: Agent Mail server write/index synchronization and degraded-mode policy.

### H3: Health check and MCP tools use different state sources by design, and the defect is only missing operator guidance - LIKELIHOOD: LOW

**Description**: The system may intentionally support archive-backed reads during degraded mode while marking SQLite readiness red. If so, the defect is not data loss but insufficiently clear operational gating: functional tools appear healthy even though coordinated multi-agent usage should be blocked until reconstruction.

**Supporting evidence**: E1.1, E2.1.

**Contradicting evidence**: `health_check` labels the state `error/red`, and `am doctor reconstruct --dry-run` says records need recovery.

**Verification steps**:

1. Read Agent Mail operational docs for degraded mode guarantees.
2. Confirm whether mutating MCP tools should be blocked in `degraded_read_only`.
3. If the behavior is intentional, improve health messaging and wrapper preflight checks so agents do not treat partial success as full health.

**Fix scope**: Agent Mail degraded-mode documentation, health status semantics, and possibly `br-claim`/`br-complete` preflight checks if wrappers should hard-fail on semantic red.

---

## INVESTIGATION ROADMAP

### Step 1: Verify H1

Run:

```bash
am doctor reconstruct --dry-run
sqlite3 /Users/alinaaitova/.mcp_agent_mail_git_mailbox_repo/storage.sqlite3 "SELECT count(*) FROM agents; SELECT count(*) FROM messages;"
```

If dry-run still reports recoverable agents/messages and SQLite counts remain zero, H1 is confirmed.

If confirmed, get explicit operator approval before running:

```bash
am doctor reconstruct --yes
```

Then re-run `health_check`, `list_agents`, `fetch_inbox`, `search_messages`, and SQLite counts.

### Step 2: Verify H2

If H1 repair succeeds but future writes recreate drift, inspect the Agent Mail server write path and recovery-mode gate. Specifically verify whether the server should reject `register_agent`, `send_message`, and `file_reservation_paths` while `recovery.mode=degraded_read_only`.

If confirmed, fix write/index synchronization or hard-block mutating operations during degraded mode.

### Step 3: Verify H3

If H1 and H2 are rejected, treat this as an operational semantics issue. Make the system clearly distinguish:

- HTTP process liveness (`/healthz` is alive)
- semantic storage readiness (`health_check` is green)
- archive-only degraded read behavior
- safe/unsafe mutating operations

---

## FIX CONSTRAINTS

1. Do not delete archive files or database backups.
2. Do not run destructive Git or filesystem cleanup commands.
3. Do not overwrite customized `bin/br-claim` or `bin/br-complete` unless explicitly requested.
4. Preserve all existing archive records under `/Users/alinaaitova/.mcp_agent_mail_git_mailbox_repo/projects/users-alinaaitova-desktop-testproject1`.
5. If `am doctor reconstruct --yes` is run, capture before/after health output and SQLite counts.
6. Mutating repair commands require explicit operator approval.

---

## ACCEPTANCE CRITERIA

### Functional

- [ ] `health_check({})` returns non-error health with semantic readiness passing.
- [ ] `list_agents(project_key="/Users/alinaaitova/Desktop/Testproject1")` returns `MaroonMountain` and `JadeMink`.
- [ ] `fetch_inbox` for `MaroonMountain` returns the smoke-test message or an intentional migrated equivalent.
- [ ] `search_messages` finds `agentmail-smoke-20260518T1020Z`.
- [ ] A fresh file reservation can be created and released, with no unreleased reservation left behind.

### Evidence-Based

- [ ] SQLite `agents` count is at least `2`.
- [ ] SQLite `messages` count is at least `1`.
- [ ] Archive inventory and SQLite index no longer disagree in `health_check.semantic_readiness.detail`.
- [ ] `am doctor reconstruct --dry-run` no longer reports pending recovery for the same agents/messages, or it reports an explicitly understood no-op state.

### Regression

- [ ] Existing Agent Mail guard status remains installed:

```bash
am guard status /Users/alinaaitova/Desktop/Testproject1
```

- [ ] Wrapper help still runs:

```bash
bin/br-claim --help
bin/br-complete --help
```

- [ ] Existing customized wrapper files are not overwritten unless that is explicitly part of the approved fix.

### Test

- [ ] Add or run a regression test in the Agent Mail project that covers archive-to-SQLite reconstruction.
- [ ] Add or run a regression test for mutating MCP operations while semantic health is red or recovery mode is degraded.
- [ ] Add or run a smoke test that verifies register, send, fetch/search, reserve, release, and health readiness as one end-to-end path.

---

## FILE MAP

| File | Role | Lines Referenced |
|------|------|------------------|
| `/Users/alinaaitova/.codex/skills/install-bead-wrappers/SKILL.md` | Wrapper skill documentation used to identify why the skill is not the repair path | top-level description and workflow |
| `/Users/alinaaitova/Desktop/Testproject1/bin/br-claim` | Existing project wrapper; should not be overwritten casually | header and MCP call behavior |
| `/Users/alinaaitova/Desktop/Testproject1/bin/br-complete` | Existing project wrapper; should not be overwritten casually | header and MCP call behavior |
| `/Users/alinaaitova/Desktop/Testproject1/AGENTS.md` | Project rule source requiring wrapper discipline | Rule 2 section |
| `/Users/alinaaitova/.mcp_agent_mail_git_mailbox_repo/storage.sqlite3` | SQLite index currently missing archive-backed rows | direct SQLite counts |
| `/Users/alinaaitova/.mcp_agent_mail_git_mailbox_repo/projects/users-alinaaitova-desktop-testproject1/agents/MaroonMountain/profile.json` | Archive-backed agent record missing from SQLite index | archive listing |
| `/Users/alinaaitova/.mcp_agent_mail_git_mailbox_repo/projects/users-alinaaitova-desktop-testproject1/agents/JadeMink/profile.json` | Archive-backed agent record missing from SQLite index | archive listing |
| `/Users/alinaaitova/.mcp_agent_mail_git_mailbox_repo/projects/users-alinaaitova-desktop-testproject1/messages/2026/05/2026-05-18T10-20-26Z__agentmail-smoke-20260518t1020z-self-test-message__1.md` | Archive-backed smoke-test message missing from SQLite index | archive listing |
