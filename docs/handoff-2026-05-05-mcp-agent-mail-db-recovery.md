---
title: MCP Agent Mail SQLite corruption — post-incident handoff
date: 2026-05-05
status: resolved
severity: medium  # read-API outage ~3 min, zero data loss, auto-recovered
component: mcp-agent-mail
version: 0.2.51
platform: macOS Darwin 24.5.0 (arm64)
audience: on-call operator OR fresh debugging agent
forensic_bundle: ~/.mcp_agent_mail_git_mailbox_repo/doctor/forensics/storage.sqlite3/reconstruct-20260505_210708_464/
daemon_log: ~/Library/Logs/agent-mail/stderr.log
---

# Post-Incident Handoff: MCP Agent Mail SQLite corruption auto-recovered from Git archive

| Field | Value |
| --- | --- |
| **Status** | RESOLVED — daemon self-recovered via `am doctor reconstruct` at 2026-05-05T21:07:08Z |
| **Operator action required** | None for the outage. Two preventative actions recommended (§13). |
| **Data loss** | Zero. All 3 messages and 3 agents intact in DB and archive (proof in §7). |
| **Audience** | On-call operator OR a fresh debugging agent picking this up cold for upstream bug filing. |
| **Prerequisites to execute the playbooks** | `am` ≥ 0.2.51 on PATH, `sqlite3` CLI, `launchctl` access (macOS LaunchAgent lifecycle). |

## Operator One-Liner

> Outage already auto-recovered. To clear the two standing WARNs, run `am doctor fix --dry-run` then `am doctor fix --yes` (preview shown in §13.1). To file upstream, attach the forensic bundle (path in frontmatter) and the trimmed log per §13.3.

**Companion implementation handoff:** use `prompts/mcp-agent-mail-read-search-database-errors-fix-handoff.md` for the upstream fix session. This document is the operator/evidence record; it should not be used alone as the implementation plan.

**Fresh-source caveat for implementation agents:** before filing or fixing upstream, compare the failing incident binary (`am 0.2.51`) with current `mcp_agent_mail_rust` source. A fresh source pass found that upstream `main` already contains archive-aware read-pool/snapshot paths for `search_messages`, `fetch_inbox`, and resources, plus SQLite WAL initialization. Treat this document as evidence for the incident runtime; use the companion implementation handoff to decide whether the real fix is a backport/release gap or a still-present bug in the current code.

---

## Contents

1. [TL;DR](#tldr)
2. [Executive Summary](#executive-summary)
3. [Timeline](#timeline-utc)
4. [Root Cause](#root-cause-pre-existing-sqlite-page-corruption)
5. [Daemon Log Excerpts](#daemon-log-excerpts)
6. [Forensic Evidence](#forensic-evidence-what-the-daemon-captured)
7. [Data Integrity Proof](#data-integrity-proof)
8. [Known Unknowns](#known-unknowns)
9. [Conflicting Recovery Guidance](#conflicting-recovery-guidance-worth-filing-upstream)
10. [Verification Snapshot](#verification-snapshot)
11. [Reproduction Steps](#reproduction-steps-runnable-mcp-calls)
12. [Recurrence Playbook](#recurrence-playbook-if-the-daemon-fails-to-self-recover)
13. [Recommended Preventative Actions](#recommended-preventative-actions-apply-now)
14. [Upstream Bug Report Template](#upstream-bug-report-template)
15. [Sidebar: Unrelated `ionice` Bug](#sidebar-unrelated-ionice-bug)
16. [Pointers](#pointers)
17. [Out of Scope](#out-of-scope)

---

## TL;DR

The MCP Agent Mail SQLite index lost all rows at daemon startup at 20:54:13Z due to pre-existing page-72 corruption; the daemon detected it, deferred recovery to the query-path, and ran `am doctor reconstruct` itself ~13 minutes later when the smoke-test workload triggered the corrupt-verdict read. **Zero data loss; no operator action required for the outage.** On the incident machine, two preventative WARNs (`wal_mode`, `mcp_config`) remain; upstream bug status must be checked against current source before filing — see §13.

## Executive Summary

The MCP Agent Mail server briefly entered `degraded_read_only` mode while smoke-testing tools from this project. The on-disk Git archive at `~/.mcp_agent_mail_git_mailbox_repo` held everything (1 project / 3 agents / 3 messages), but the SQLite index at `storage.sqlite3` had been emptied, so all read tools (`search_messages`, `fetch_inbox`, `list_agents`, etc.) failed with the generic *"A database error occurred. This may be a transient issue – try again."* Writes (`register_agent`, `send_message`, `file_reservation_paths`) continued to land in the archive and *appeared* to succeed.

**Root cause: SQLite page-level corruption that pre-existed the smoke test by ten minutes.** At daemon startup (`2026-05-05T20:54:13.519Z`) the integrity guard detected `database disk image is malformed: page 72 is never used`, deliberately left the live DB untouched, and queued reconstruct to fire on the next corrupt-verdict read. The smoke test was simply the workload that triggered that read.

Recovery was clean: the daemon ran `am doctor reconstruct` itself at `2026-05-05T21:07:08Z` (`trigger: "automatic-recovery"`), captured a full forensic bundle at the path in the frontmatter, rebuilt the DB from the Git archive in ~50 ms with 0 parse errors. Post-recovery, every MCP read returns correctly; subjects, IDs, and sender names in the DB match the archive filenames 1:1.

Three real follow-ups remained for the incident runtime. A current-source implementation pass must verify whether each is still open or already fixed upstream:

1. **Latent preventative fix for `am 0.2.51`** — SQLite is in `journal_mode = delete`, not WAL. `am doctor check` flags this; `am doctor fix` will set it. WAL would not have prevented the original corruption but would make future write paths more crash-safe. Current upstream source should be checked before filing this as an open code bug, because upstream `main` already contains WAL initialization.
2. **Upstream bugs worth filing** — (a) the SQLite corruption itself; (b) conflicting recovery guidance between the JSON-RPC `health_check` and the Rust `am doctor check` CLI; (c) archive maintenance calls `ionice`, which doesn't exist on macOS, failing every cycle with exit 127.
3. **Recurrence playbook** — for the next time, in case auto-recovery does *not* fire (it might not, if the corrupt-verdict trigger doesn't see a read for some reason).

---

## Timeline (UTC)

| Time | Event | Evidence |
| --- | --- | --- |
| 20:51:43 | Daemon starts on storage root `~/.local/share/mcp-agent-mail/git_mailbox_repo/`. | stderr.log line 1 |
| 20:54:13.296 | Daemon restarts, switches storage root to `~/.mcp_agent_mail_git_mailbox_repo/`. | stderr.log line 2 |
| 20:54:13.426 | Proactive backup of the (already-corrupt-from-prior-shutdown) live DB written to `storage.sqlite3.bak`. Backup happens before integrity_check runs, which is why the backup is also zero-row. | stderr.log line 11 |
| 20:54:13.519 | **Integrity guard detects corruption** at startup: `database disk image is malformed: page 72 is never used`. Live DB intentionally not mutated; recovery deferred to query path. | stderr.log line 14 |
| 21:04:06 | `ensure_project` succeeds; project `users-alinaaitova-desktop-testproject1` created. | archive `git log` `cd0ff55` |
| 21:04:13 | `register_agent BronzeGoose` — fresh-durability probe **misses committed agent visibility** (`agent row not visible after commit for 1:BronzeGoose`); pooled runtime handle confirms the row. First explicit corruption-symptom warning of the session. | stderr.log line 63 |
| 21:04:18 | `register_agent CrimsonChapel` (program `codex-cli`, model `gpt-5`) — same fresh-durability-probe miss. **Real concurrent Codex CLI session writing alongside Claude Code.** | stderr.log line 67 |
| 21:04:20 | `create_agent_identity BronzeFrog` — same probe miss. | stderr.log line 68 |
| 21:04:24 | CrimsonChapel sends self-addressed `[agent-mail-smoke] Self-test message` (id=1); fresh-durability probe misses committed message. | stderr.log lines 69–70 |
| 21:04:27 | `send_message` (BronzeGoose → BronzeFrog) — daemon auto-injects `Contact request from BronzeGoose` (id=2) via the contact-handshake macro, then writes the smoke-test message (id=3). Both miss the fresh-durability probe. | stderr.log lines 71–72 |
| ~21:06 | All MCP read tools (`list_agents`, `search_messages`, `fetch_inbox`) start returning *"A database error occurred. This may be a transient issue – try again."* | observed during smoke test |
| 21:07:08.444 | **Readiness archive-drift detector fires.** "archive inventory is ahead of the sqlite index (archive=1/3/3, db=0/0/0, missing project users-alinaaitova-desktop-testproject1)". | stderr.log line 90 |
| 21:07:08.445 | Daemon removes header-only/truncated WAL sidecar (`storage.sqlite3-wal`, 32 bytes) before reconstruct, to prevent `WAL file too small for header` errors. | stderr.log line 91 |
| 21:07:08.476 | Forensic bundle captured at `doctor/forensics/storage.sqlite3/reconstruct-20260505_210708_464/`, `command=reconstruct`, `trigger=automatic-recovery`. | stderr.log line 97 |
| 21:07:08.495 | **Reconstruct succeeds:** `reconstructed 1 projects, 3 agents, 3 messages (3 recipients), 0 parse errors` (wall time ~50 ms). | stderr.log line 98 |
| ~21:09 | Operator runs `am doctor check` and gets the misleading "Next action: No database repair action is currently recommended" output despite a `[FAIL] archive_db_parity` line in the same CLI output. | CLI output |
| ~21:25 | Re-run of MCP read tools succeeds. `health_check` green. `am doctor check` "All checks passed." | re-run during this handoff |
| 21:27:02 | Unrelated: archive maintenance worker fails with `exit status 127: nice: ionice: No such file or directory` (macOS lacks `ionice`). | stderr.log line 119 |

---

## Root Cause: pre-existing SQLite page corruption

The single line that explains everything (stderr.log line 14, 2026-05-05T20:54:13.519Z, ten minutes before any user activity in this session):

```
WARN integrity guard detected recoverable sqlite corruption; background guard does not mutate
     the live db — the query-path recovery flow will trigger an admission-controlled reconstruct
     on the next corrupt-verdict read
     phase="integrity_check"
     path=/Users/alinaaitova/.mcp_agent_mail_git_mailbox_repo/storage.sqlite3
     error=Integrity check failed: integrity_check detected corruption (3119us):
           database disk image is malformed: page 72 is never used
     storage_root_present=true
```

What the daemon did, by design:

1. **Detected** the corruption (`PRAGMA integrity_check` reports a malformed page).
2. **Did not mutate** the live DB from the background guard. (Correct: a background thread holding a connection is the wrong place to do a destructive rebuild.)
3. **Logged the deferral** explicitly, naming the trigger condition: "the query-path recovery flow will trigger an admission-controlled reconstruct on the next corrupt-verdict read."
4. **Created a proactive backup** of the (already corrupt) live DB at `storage.sqlite3.bak` 100 ms earlier (line 11). This is why `storage.sqlite3.bak` has 0 rows — it was a copy of the corrupted DB at startup.
5. **Continued accepting writes,** which routed to the archive (canonical) and the SQLite index (best-effort). Pooled connections accepted the writes; fresh connections used by durability probes saw a stale snapshot.
6. **Fired reconstruct** at 21:07:08Z when the readiness flow next checked archive-vs-DB parity and saw the divergence.

Why my smoke test triggered it then and not earlier: the daemon had been idle from 20:54:21Z (last startup log) until 21:04:06Z (my `ensure_project`). Once writes started flowing, the readiness/parity check ran and immediately saw that the archive was ahead of the DB.

### Why the DB was already corrupt at startup

The log evidence is **not conclusive on the prior cause.** Two plausible mechanisms, in rough order of likelihood:

1. **Storage root migration without atomic move.** stderr.log lines 1–2 show the daemon was running on `~/.local/share/mcp-agent-mail/git_mailbox_repo/` at 20:51:43Z, then restarted at 20:54:13Z on `~/.mcp_agent_mail_git_mailbox_repo/`. If the new SQLite file was assembled by file-level copy or symlink redirection rather than the supported migration path, a torn page is plausible.
2. **`journal_mode = delete` plus an earlier unclean shutdown.** Without WAL, a shutdown mid-checkpoint can leave the rollback journal in an unsafe state. The 32-byte truncated WAL sidecar the daemon swept on line 91 is consistent with this — though a 32-byte WAL can also appear from any aborted WAL-header write and does not by itself prove the DB was previously running in WAL mode.

Other mechanisms (external processes touching the file mid-transaction, e.g. `cp` / Time Machine / Spotlight) are theoretically possible but **not supported by anything in the log;** treat as third-tier candidates only if the operator has independent evidence.

Whichever it was, **the daemon's recovery design caught and corrected it cleanly.** The forensic bundle preserves the pre-recovery state for upstream investigation.

---

## Daemon Log Excerpts

Path: `~/Library/Logs/agent-mail/stderr.log` (`stdout.log` is empty; all output is on stderr).

### Pre-existing corruption detected at startup (lines 11–14)

```
2026-05-05T20:54:13.426Z  INFO  created proactive database backup
                                primary=…/storage.sqlite3
                                backup=…/storage.sqlite3.bak
2026-05-05T20:54:13.427Z  WARN  event="drop_close" msg="Connection dropped without explicit close()"
2026-05-05T20:54:13.427Z  WARN  sqlite operation hit lock/busy error; retrying
                                operation="sqlite open" error=database is busy retry=1 max_retries=3 delay_ms=25
2026-05-05T20:54:13.519Z  WARN  integrity guard detected recoverable sqlite corruption;
                                background guard does not mutate the live db — the query-path
                                recovery flow will trigger an admission-controlled reconstruct
                                on the next corrupt-verdict read
                                error=Integrity check failed: integrity_check detected corruption
                                      (3119us): database disk image is malformed: page 72 is never used
```

### Symptom side during smoke test — fresh-durability-probe misses (lines 63–72)

For every write during my session and the concurrent codex-cli session, a fresh connection used by durability probes did not see the row that a pooled connection had just committed:

```
21:04:13  WARN  fresh durability probe missed committed agent visibility;
                pooled runtime handle confirmed row project_id=1 agent=BronzeGoose
                fresh_error=agent row not visible after commit for 1:BronzeGoose
21:04:18  WARN  ... agent=CrimsonChapel ...
21:04:20  WARN  ... agent=BronzeFrog ...
21:04:24  WARN  fresh durability probe missed committed message visibility ... message_id=1 ...
21:04:27  WARN  ... message_id=2 ...
21:04:27  WARN  ... message_id=3 ...
```

### Auto-recovery sequence (lines 89–100)

```
21:07:08.444  WARN  readiness archive drift detected; attempting archive-backed SQLite reconcile
                    error=archive inventory is ahead of the sqlite index
                          (archive=1/3/3, db=0/0/0, missing project users-alinaaitova-desktop-testproject1)
21:07:08.445  WARN  removing header-only/truncated WAL sidecar (<=32 bytes; prevents
                    'WAL file too small for header' errors)
                    path=…/storage.sqlite3-wal size=32
21:07:08.476  WARN  captured mailbox forensic bundle before automatic recovery
                    bundle=…/doctor/forensics/storage.sqlite3/reconstruct-20260505_210708_464
21:07:08.495  INFO  database reconstruction from archive complete
                    stats=reconstructed 1 projects, 3 agents, 3 messages (3 recipients), 0 parse errors
21:07:08.505  WARN  reconciled sqlite database from archive before initialization because
                    archive inventory or project identity state was ahead
                    (db=0/0/0/0  archive=1/3/3/3)
```

Wall-clock: trigger to "reconstruction complete" = ~50 ms; no parse errors.

---

## Forensic Evidence: what the daemon captured

Bundle root: `~/.mcp_agent_mail_git_mailbox_repo/doctor/forensics/storage.sqlite3/reconstruct-20260505_210708_464/`

```
summary.json                      # bundle manifest, command="reconstruct", trigger="automatic-recovery"
manifest.json                     # artifact catalog with sha256s
references/
  live-db-state.json              # at capture: db=360448 bytes, wal=32 bytes (truncated header), no shm, no recovery lock
  archive-drift.json
  archive-drift-report.json       # archive_only_ids=[1,2,3], db_only_ids=[], db_message_count=0, archive_message_count=3
  environment.json                # process_id=69379, db_path, storage_root
sqlite/
  storage.sqlite3                 # captured pre-recovery copy (360448 bytes, sha256 ae27e4...)
  storage.sqlite3-wal             # captured pre-recovery wal (32 bytes — truncated header, expected for delete-mode DB)
```

The `archive-drift-report.json` is the load-bearing document for upstream filing. Its `identity_mismatches` field names exactly which project the DB had lost: *"Archive project users-alinaaitova-desktop-testproject1 has no matching DB identity."*

Two stray sqlite snapshots also live alongside the live DB:

```
~/.mcp_agent_mail_git_mailbox_repo/storage.sqlite3.bak                                   (290 KB, 0 rows everywhere)
~/.mcp_agent_mail_git_mailbox_repo/storage.sqlite3.archive-reconcile-20260505_210708_477 (360 KB, 0 rows everywhere)
```

`storage.sqlite3.bak` is the post-corruption proactive backup taken at startup (line 11). The `archive-reconcile-...` snapshot is the pre-reconstruct copy taken just before the rebuild. **Neither file holds recoverable user data** — they are zero-row copies preserved as part of the audit trail. `am doctor backups --json` returns `[]`, confirming the doctor toolkit does not consider these as restorable backups.

---

## Data Integrity Proof

After the auto-recovery, the SQLite index and the on-disk archive are byte-aligned at the row level. Verified directly:

```
$ sqlite3 ~/.mcp_agent_mail_git_mailbox_repo/storage.sqlite3 \
    "SELECT m.id, m.subject, a.name FROM messages m JOIN agents a ON a.id = m.sender_id ORDER BY m.id;"

1 | [agent-mail-smoke] Self-test message  | CrimsonChapel
2 | Contact request from BronzeGoose      | BronzeGoose
3 | Smoke test: hello from BronzeGoose    | BronzeGoose
```

Archive filenames at `…/projects/users-alinaaitova-desktop-testproject1/messages/2026/05/`:

```
2026-05-05T21-04-24Z__agent-mail-smoke-self-test-message__1.md
2026-05-05T21-04-27Z__contact-request-from-bronzegoose__2.md
2026-05-05T21-04-27Z__smoke-test-hello-from-bronzegoose__3.md
```

IDs match (`__1`, `__2`, `__3`); slugs derive directly from the subjects; timestamps match `created_ts` from the MCP responses to the byte. `am doctor archive-scan`:

```
Archive Scan:
  Storage root         /Users/alinaaitova/.mcp_agent_mail_git_mailbox_repo
  Projects:                 1
  Agents:                   3
  Logical messages:         3
  Canonical message files:  3
  Duplicate canonical ids:  0 id(s) / 0 extra file(s)
  Malformed canonical files: 0
  Thread digests:           1
  Missing project.json:     0
  Invalid project.json:     0
  Suspicious temp projects: 0

  No archive hygiene findings detected.
```

Conclusion: zero data loss, zero archive damage, zero hygiene findings.

---

## Known Unknowns

Things this incident did **not** answer. A fresh debugging agent should be aware of them rather than assume they're settled.

| Unknown | Why it matters | How to investigate |
| --- | --- | --- |
| Tantivy search-index post-reconstruct lag. | The Tantivy index lives at `~/.mcp_agent_mail_git_mailbox_repo/search_index/` and is independent of the SQLite reconstruct. Search returned correct results during this handoff, but I don't know whether the reconstruct triggered an automatic backfill, whether there's a measurable lag window, or whether a soft restart is needed for new content to index. | Compare `db_count` and `index_count` in the next `[startup-search]` log line on a daemon restart, or watch for "[backfill]" entries in stderr.log over a few minutes after a reconstruct. |
| What `mcp_config_token` WARN actually flags. | `am doctor check` lists `mcp_config_token` as a separate WARN from `mcp_config`. `am doctor fix --dry-run` does **not** list it among `fixable_checks`, so it isn't auto-fixed. Could be a token-rotation reminder or a config-mismatch flag. | `am doctor check --json` on a clean machine, then diff against this one; or grep the binary for the literal string `mcp_config_token`. |
| LaunchAgent restart policy. | The daemon restarted between 20:51:43Z and 20:54:13Z (storage-root switch). I did not read `~/Library/LaunchAgents/com.agent-mail.plist` to see whether `KeepAlive`, `RunAtLoad`, or env vars triggered the change. Relevant if the corruption was related to the migration. | `cat ~/Library/LaunchAgents/com.agent-mail.plist` (or `plutil -p` on it). |
| Whether the integrity guard's deferred-recovery design has a worst-case latency bound. | If a daemon never sees a "corrupt-verdict read" (e.g., only writes, no reads), reconstruct never fires. This was not the case here, but it's a theoretical edge. | Read upstream code for `integrity guard` / "query-path recovery flow", or run an extended-soak test with writes only. |

---

## Conflicting Recovery Guidance (worth filing upstream)

During the live outage these three sources disagreed:

| Source | What it said |
| --- | --- |
| MCP `health_check` JSON-RPC | `recovery.next_action = "Run \`am doctor repair\` to attempt automatic recovery"` |
| `am doctor check` (CLI) | "Primary issue: [FAIL] archive_db_parity (...). Next action: **No database repair action is currently recommended.**" |
| `am doctor` help text | The dedicated tool for an empty-DB / archive-full scenario is **`am doctor reconstruct`**, not `am doctor repair`. |
| Daemon (in practice) | Ran **`am doctor reconstruct`** itself, which is what actually worked. |

So in production the right answer turned out to be neither of the two human-facing recommendations.

This is a documentation / UX bug worth filing:

- The JSON-RPC `health_check.recovery.next_action` should suggest `reconstruct` (or `am doctor fix`), not `repair`, when the failing check is `archive_db_parity` with `db_message_count == 0` while `archive_message_count > 0`.
- `am doctor check`'s top-line "Next action" should not say "No database repair action is currently recommended" while a `[FAIL] archive_db_parity` is sitting underneath it. A reasonable hint would be "Auto-recovery typically fires on the next read; if you need to force it, run `am doctor reconstruct`."

---

## Verification Snapshot

```jsonc
// health_check
{ "status": "ok", "health_level": "green",
  "semantic_readiness": { "status": "ok",
    "detail": "Archive and sqlite inventory are aligned enough for health_check: archive projects=1, agents=3, messages=3, db projects=1, agents=3, messages=3" }, ... }

// search_messages — query "procurement-smoketest-marker-42"
[{ "id": 3, "subject": "Smoke test: hello from BronzeGoose",
   "from": "BronzeGoose", "to": ["BronzeFrog"], "ack_required": 1, ... }]

// fetch_inbox — BronzeFrog
[{ "id": 3, "subject": "Smoke test: hello from BronzeGoose", "from": "BronzeGoose", ... },
 { "id": 2, "subject": "Contact request from BronzeGoose", "from": "BronzeGoose", ... }]

// list_agents
[{ "name": "BronzeFrog",     "program": "claude-code", "model": "claude-opus-4-7", ... },
 { "name": "CrimsonChapel",  "program": "codex-cli",   "model": "gpt-5",          ... },
 { "name": "BronzeGoose",    "program": "claude-code", "model": "claude-opus-4-7", ... }]

// am doctor check  →  "All checks passed." (4 environment WARNs remain: wal_mode, mcp_config, mcp_config_token, update_available)
```

Note: `BronzeFrog`'s inbox now contains **two** messages because the auto-generated "Contact request from BronzeGoose" (id=2) was dispatched by the daemon's contact-handshake macro when `send_message` was the first delivery between these two agents. This is expected behaviour, not residue from the outage.

`CrimsonChapel` is **not** a daemon-internal artifact. It's a real concurrent Codex CLI session that registered against the same MCP server during the same 7-second window as the Claude Code session — confirmed by the matching `fresh durability probe missed` warning at line 67 (same code path as my own agents) and by `program: codex-cli, model: gpt-5` (the daemon never registers itself with those identifiers; its binary is `am`).

---

## Reproduction Steps (runnable MCP calls)

To reproduce the smoke-test workload that triggered (but did not cause) recovery:

```jsonc
// 1. Ensure project — must be an absolute path matching the working directory
{ "method": "ensure_project",
  "params": { "human_key": "/Users/alinaaitova/Desktop/Testproject1" } }

// 2. Register sender
{ "method": "register_agent",
  "params": { "project_key": "/Users/alinaaitova/Desktop/Testproject1",
              "program": "claude-code",
              "model": "claude-opus-4-7",
              "task_description": "Smoke test" } }
// → response carries the auto-generated agent name; capture it.

// 3. Create a recipient identity (always a fresh adjective+noun)
{ "method": "create_agent_identity",
  "params": { "project_key": "/Users/alinaaitova/Desktop/Testproject1",
              "program": "claude-code",
              "model": "claude-opus-4-7",
              "task_description": "Test recipient" } }

// 4. Send a marker-keyword message so search has something distinctive to find
{ "method": "send_message",
  "params": { "project_key": "/Users/alinaaitova/Desktop/Testproject1",
              "sender_name": "<sender from step 2>",
              "to": ["<recipient from step 3>"],
              "subject": "Smoke test message",
              "body_md": "Marker: procurement-smoketest-marker-42",
              "ack_required": true,
              "importance": "normal" } }

// 5. File reservations
{ "method": "file_reservation_paths",
  "params": { "project_key": "/Users/alinaaitova/Desktop/Testproject1",
              "agent_name": "<sender from step 2>",
              "paths": ["docs/session-log-2026-05-05.md", "docs/research/scratch-*.md"],
              "ttl_seconds": 300, "exclusive": true,
              "reason": "Smoke test" } }

// 6. The reads that failed during the outage and now succeed
{ "method": "search_messages",
  "params": { "project_key": "/Users/alinaaitova/Desktop/Testproject1",
              "query": "procurement-smoketest-marker-42" } }
{ "method": "fetch_inbox",
  "params": { "project_key": "/Users/alinaaitova/Desktop/Testproject1",
              "agent_name": "<recipient from step 3>",
              "include_bodies": true, "limit": 10 } }
{ "method": "list_agents",
  "params": { "project_key": "/Users/alinaaitova/Desktop/Testproject1" } }
```

Note: with the pre-existing corruption gone, this sequence will not re-trigger the desync. To deliberately reproduce the failure mode for upstream debugging you would need to inject the same kind of page corruption directly into a sandbox copy of `storage.sqlite3` — outside the scope of this handoff.

---

## Recurrence Playbook (if the daemon fails to self-recover)

The daemon's design is to fire reconstruct on the next corrupt-verdict read. If, for some reason, that doesn't happen (a future regression, an unusual block on the readiness path, etc.), and `health_check.recovery.mode` stays `degraded_read_only` for more than 2 minutes, this is the safe operator path.

```bash
# 0. Capture state (all read-only)
am --version                         # confirm binary version, e.g., 0.2.51
am service status                    # confirm com.agent-mail launchd LaunchAgent is running
ps -o pid,etime,command -p $(pgrep -f 'am serve-http')
am doctor check 2>&1 | tee /tmp/am-doctor-check-$(date +%Y%m%dT%H%M%S).log

# 1. Confirm there is data to reconstruct FROM. If both archive and DB are empty,
#    stop — there is nothing reconstruct can do.
am doctor reconstruct --dry-run --json
# Expect: {"status":"dry_run", "would_recover":{"projects":N>=1, "agents":..., ...}}
# If `would_recover` is all zeros, the archive is also empty — reconstruct will not help.

# 2. List restorable backups (often empty in dev)
am doctor backups --json

# 3. Stop the daemon BEFORE running reconstruct (it holds the SQLite file open)
am service restart                            # graceful: SIGTERM with force-kill fallback
# OR, for a full stop:
launchctl bootout gui/$UID/com.agent-mail
# Verify it's actually down:
pgrep -f 'am serve-http' || echo "stopped"

# 4. Run reconstruct — captures a forensic bundle, then rebuilds DB from the Git archive
am doctor reconstruct --yes --json

# 5. If reconstruct partially succeeds (status not "ok" or non-zero parse_errors),
#    DO NOT delete artifacts. Capture the forensics bundle path printed in the output
#    and preserve `~/.mcp_agent_mail_git_mailbox_repo/doctor/forensics/` for upstream.
#    Fall back to: `am doctor restore <BACKUP_PATH>` if `am doctor backups` listed any.

# 6. Bring the daemon back up if you fully stopped it
launchctl bootstrap gui/$UID ~/Library/LaunchAgents/com.agent-mail.plist
am service status

# 7. Verify
am doctor check                                # expect "All checks passed."
# From any MCP-aware client:
#   health_check                  → health_level=green, recovery.mode!=degraded_read_only,
#                                   semantic_readiness.status=ok
#   list_agents(project_key=...)  → returns expected agents
#   fetch_inbox(...)              → returns expected messages
#   search_messages(...)          → returns expected matches
```

**Why `am service restart` first:** running `am doctor reconstruct` while the daemon holds the SQLite file open risks the rebuild racing with live writes. The CLI does not refuse to run while the daemon is up, so the operator must stop it explicitly.

**Do NOT** under any circumstance:

- Run `am clear-and-reset-everything` — destructive, wipes the global mailbox store. Only valid if the user has explicitly accepted total data loss.
- `cp` the live `storage.sqlite3` for backup. Even in `delete` journal mode, mid-transaction copies are unreliable. The forensics bundle and `am doctor backups`/`restore` are the supported primitives.
- Edit JSON files under `~/.mcp_agent_mail_git_mailbox_repo/projects/<slug>/` directly. The archive is committed to a Git repo (`.git/` at the storage root) and the daemon detects out-of-band edits.
- Manually delete the two zero-row `.sqlite3.bak` / `.archive-reconcile-...` files. They are part of the forensic audit trail; let `am doctor pack-archive` or `am archive-normalize` clean them.

---

## Recommended Preventative Actions (apply now)

Version caveat: this section is written for the incident machine running `am 0.2.51`. It is still useful operator guidance for that host, but an upstream implementation agent should first verify whether current `main` already includes the WAL/read-pool behavior before treating any item here as unimplemented.

### 1. Run `am doctor fix` to resolve the standing WARNs

`am doctor fix --dry-run --json` (captured during this handoff) shows exactly two non-trivial actions plus six no-op skips:

| check | action | detail |
| --- | --- | --- |
| `legacy_python_alias` | skip | "No legacy aliases found" |
| `path_order` | skip | "/Users/alinaaitova/.local/bin already in PATH" |
| `mcp_config` | dry_run | **"Would convert stdio command mode in `~/Library/Application Support/Claude/claude_desktop_config.json` to `http://127.0.0.1:8765/mcp/`"** |
| `storage_root_git_index_lock` | skip | "No stale index.lock files" |
| `guard_hooks` | skip | "Guard hooks already installed" |
| `database_repair` | skip | "Database … passed file, integrity, and relational consistency probes" |
| `server_runtime` | skip | "Agent Mail server detected on 127.0.0.1:8765; /health responded with HTTP 200" |
| `wal_mode` | dry_run | **"Would enable WAL journal mode"** |

So `am doctor fix --yes` will (a) flip Claude Desktop's MCP config from stdio to HTTP mode and (b) enable WAL on the SQLite file. Both are safe to apply. Note that `database_repair` is a no-op now because the post-recovery DB is healthy — confirming the auto-recovery worked.

```bash
am doctor fix --dry-run         # human-readable preview
am doctor fix --dry-run --json  # machine-readable preview summarized in this section
am doctor fix --yes             # apply the two changes
am doctor check                 # confirm WARNs cleared
```

If you only want WAL and not the MCP-config rewrite, do it manually — the order matters (the daemon must be fully stopped, not just bounced, while you flip the PRAGMA, otherwise it races back up and reopens the DB mid-change):

```bash
# 1. Stop the daemon and confirm it is actually down
launchctl bootout gui/$UID/com.agent-mail
pgrep -f 'am serve-http' || echo "stopped"

# 2. Flip the journal mode (returns "wal" if successful)
sqlite3 ~/.mcp_agent_mail_git_mailbox_repo/storage.sqlite3 'PRAGMA journal_mode = WAL;'

# 3. Bring the daemon back up
launchctl bootstrap gui/$UID ~/Library/LaunchAgents/com.agent-mail.plist
am service status

# 4. Verify
sqlite3 ~/.mcp_agent_mail_git_mailbox_repo/storage.sqlite3 'PRAGMA journal_mode;'  # expect: wal
am doctor check                                                                     # wal_mode WARN should clear
```

### 2. Update the binary

`am 0.2.51` is flagged `[WARN] update_available`. The page-72 corruption may be a known issue fixed in a later release.

```bash
am self-update
am service restart
am doctor check
```

### 3. Capture the daemon log for the upstream bug

```bash
# Whole stderr for context
cp ~/Library/Logs/agent-mail/stderr.log ~/am-stderr-2026-05-05.log

# Just the corruption + recovery window for the bug report
sed -n '/2026-05-05T20:54:13/,/2026-05-05T21:07:09/p' ~/Library/Logs/agent-mail/stderr.log \
  > ~/am-incident-window-2026-05-05.log
```

Attach `~/am-incident-window-2026-05-05.log` and the forensic bundle to the upstream issue.

### 4. File the upstream bug

Use the ready-to-paste template in §14 below. Attach: (a) the trimmed log from the previous step, and (b) the entire forensic bundle directory listed in the frontmatter at the top of this doc.

---

## Upstream Bug Report Template

> **Title:** Page-level SQLite corruption persists across daemon restarts; `health_check.recovery.next_action` and `am doctor check` give conflicting recovery guidance for the same condition; macOS archive maintenance fails on missing `ionice`.
>
> **Version:** `am 0.2.51` LaunchAgent `com.agent-mail` on macOS Darwin 24.5.0 (arm64).
>
> **Environment:** development; storage root `~/.mcp_agent_mail_git_mailbox_repo`; `journal_mode = delete`; daemon was previously running on a different storage root (`~/.local/share/mcp-agent-mail/git_mailbox_repo`) and was restarted ~2.5 minutes earlier.
>
> **Symptoms:**
> 1. At startup, integrity guard logs `database disk image is malformed: page 72 is never used` (stderr.log, attached). DB is left in place; recovery is deferred to the query-path.
> 2. Subsequent agent and message writes log `fresh durability probe missed committed … visibility`. Pooled connections see the row; fresh connections do not.
> 3. ~3 minutes later, all MCP read tools (`search_messages`, `fetch_inbox`, `list_agents`) return *"A database error occurred. This may be a transient issue – try again."* `health_check` returns `health_level: red`, `recovery.mode: degraded_read_only`, `archive=1/3/3, db=0/0/0`, and `recovery.next_action: "Run \`am doctor repair\`"`.
> 4. `am doctor check` simultaneously reports `[FAIL] archive_db_parity` and `Next action: No database repair action is currently recommended.`
> 5. The daemon then runs `am doctor reconstruct` itself (`trigger: automatic-recovery`), recovers cleanly, and sweeps a 32-byte truncated WAL sidecar in the process.
>
> **Suggested fixes:**
> 1. **Page-72 corruption.** Investigate why the SQLite file became corrupt. Storage-root migration, prior unclean shutdown under `journal_mode=delete`, or external-process interference are the candidates. Consider auto-running `PRAGMA integrity_check` on startup *and* failing fast (or running reconstruct synchronously) when corruption is found, rather than deferring to a query-path trigger that may not fire promptly.
> 2. **Default to WAL.** `am doctor check` already flags `journal_mode=delete` as `[WARN] wal_mode`; ship WAL as the default for new installs and migrations if the target release still lacks it. A fresh source pass found WAL initialization on upstream `main`, so verify the branch/tag before filing this as open. Investigate the truncated WAL sidecar's origin (line 91) — it suggests the DB has previously been in WAL mode and was rolled back inconsistently.
> 3. **Conflicting recovery guidance.** When `archive_db_parity` fails with `db_message_count == 0` and `archive_message_count > 0`, both `health_check.recovery.next_action` and `am doctor check`'s "Next action" line should recommend `am doctor reconstruct` (or `am doctor fix`), not `am doctor repair`. They should also acknowledge that auto-recovery may already be in flight.
> 4. **macOS `ionice` regression.** stderr.log line 119: `archive maintenance failed git_dir=… error="exit exit status: 127: nice: ionice: No such file or directory"`. macOS does not ship `ionice`; the archive maintenance task should fall back to plain `nice` (or skip I/O priority on Darwin).
>
> **Forensic bundle:** `~/.mcp_agent_mail_git_mailbox_repo/doctor/forensics/storage.sqlite3/reconstruct-20260505_210708_464/` (attach `summary.json`, `manifest.json`, all four `references/*.json`, and the captured pre-recovery `sqlite/storage.sqlite3` + `sqlite/storage.sqlite3-wal`).
>
> **Daemon log:** trimmed window from `2026-05-05T20:54:13Z` to `2026-05-05T21:07:09Z`, attached as `am-incident-window-2026-05-05.log`.
>
> **Reproducer:** see the "Reproduction Steps" section in the post-incident handoff. Note that the handoff workload triggers but does not cause the failure; reproducing the original corruption requires sandbox-injecting a page-level malformation into `storage.sqlite3` between daemon startups.

---

## Sidebar: Unrelated `ionice` Bug

stderr.log line 119, 2026-05-05T21:27:02.040Z (after the recovery, during routine maintenance):

```
WARN archive maintenance failed
     git_dir=/Users/alinaaitova/.mcp_agent_mail_git_mailbox_repo/.git
     error="exit exit status: 127: nice: ionice: No such file or directory"
```

The archive maintenance worker shells out to `nice ionice <git command>`. macOS does not ship `ionice` — it's a Linux util-linux tool. This means archive maintenance fails every cycle (every 1800 seconds; see line 7) on macOS hosts.

**Impact:** archive maintenance (loose-object repack, gc) does not run automatically on macOS. The user can still manually run `am doctor pack-archive`. There is no immediate data-loss risk; over time the archive `.git/` directory will grow without compaction.

**Workaround:** none from the user side; needs an upstream fix to detect platform and skip `ionice` (or substitute the macOS `taskpolicy -d throughput`) on Darwin. Worth filing as part of the upstream bug template in this handoff (suggested fix #4).

---

## Pointers

| What | Where |
| --- | --- |
| `am` binary | `/Users/alinaaitova/.local/bin/am` (v0.2.51) |
| Storage root | `~/.mcp_agent_mail_git_mailbox_repo/` |
| Live SQLite index | `~/.mcp_agent_mail_git_mailbox_repo/storage.sqlite3` |
| Archive (canonical) | `~/.mcp_agent_mail_git_mailbox_repo/projects/users-alinaaitova-desktop-testproject1/` |
| Forensic bundle (this incident) | `~/.mcp_agent_mail_git_mailbox_repo/doctor/forensics/storage.sqlite3/reconstruct-20260505_210708_464/` |
| Search index (Tantivy) | `~/.mcp_agent_mail_git_mailbox_repo/search_index/` |
| Daemon (LaunchAgent) | `com.agent-mail`, plist at `~/Library/LaunchAgents/com.agent-mail.plist` |
| Daemon process | PID 69379 (at time of writing) |
| Daemon stderr log (load-bearing) | `~/Library/Logs/agent-mail/stderr.log` |
| Daemon stdout log | `~/Library/Logs/agent-mail/stdout.log` (empty in v0.2.51 — all output is on stderr) |
| HTTP server | `127.0.0.1:8765` (development env) |
| Project key (this project) | `/Users/alinaaitova/Desktop/Testproject1` |
| Project slug | `users-alinaaitova-desktop-testproject1` |
| Project UID | `52b365509219e7df692d` |
| Schema version | confirmed at startup, all migrations v15–v21 reported "schema already satisfies" |
| Claude Desktop MCP config (will be rewritten by `am doctor fix`) | `~/Library/Application Support/Claude/claude_desktop_config.json` |

---

## Out of Scope

- Recovering the two zero-row snapshot files (`storage.sqlite3.bak`, `storage.sqlite3.archive-reconcile-20260505_210708_477`) — they have no recoverable user data and are an audit-trail concern, not a recovery one.
- Cleaning up the smoke-test agents (`BronzeGoose`, `BronzeFrog`) and messages (id=2, id=3). They are valid mailbox entities now and should be left in place unless someone explicitly wants the sandbox reset, in which case `am clear-and-reset-everything` is the (destructive) tool.
- The concurrent Codex CLI session that registered `CrimsonChapel`. It is a real third-party agent on the same MCP server; it did not contribute to the corruption (the corruption pre-existed both sessions) and does not need follow-up here.
- Investigating why the daemon's contact-handshake macro auto-injected message id=2 ("Contact request from BronzeGoose") on the first cross-agent send. This is **probably** the documented `send_message` behaviour when an implicit contact handshake fires (the `send_message` schema accepts `auto_contact_if_blocked`); the exact trigger condition was not verified and is not relevant to the desync.
- Tuning the integrity guard's deferred-recovery model. The design is intentional ("background guard does not mutate the live db") and worked correctly. Whether the trigger-on-next-read latency is acceptable is a product question, not a bug.
