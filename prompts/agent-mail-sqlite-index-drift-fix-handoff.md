# Agent Mail SQLite Index Drift — Fix Handoff

ULTRATHINK and be meticulous. This is a fix handoff. Read this in full before doing anything. The accompanying defect document is `prompts/agent-mail-sqlite-index-drift-error-manifest.md` — keep it open as you work; this handoff cites it and extends it.

You are taking over a forensic investigation that has been *reproduced and partially pinpointed*. Your job is hypothesis-driven, not patch-first. Order:

0. Verify §0.5 Prerequisites — confirm you have the access, tokens, and toolchain to do the rest.
1. Re-confirm the live state (one minute) and the inferred timeline (§1.5).
2. Read §3.0 in the exact order it specifies — *especially* `mailbox_verdict.rs` and `live_db_is_suspect` — until you can answer the two gating questions in §6.5 (verdict cached vs. recomputed; whether a write gate already exists somewhere). Do not patch before answering.
3. Get explicit operator approval to run `am doctor reconstruct --yes` (the immediate state repair). Bounded scope: one project.
4. Land **wrapper preflight** in `bin/br-claim` and `bin/br-complete` (defense-in-depth that protects every project on the machine while you work on the server). Mirror byte-for-byte into both `~/.claude/skills/install-bead-wrappers/` and `~/.codex/skills/install-bead-wrappers/`.
5. Land **upstream Rust fixes** in `/Users/alinaaitova/Desktop/mcp_agent_mail_rust/` (the source for the `am` binary) for the defect classes you confirmed in Step 2, plus the adjacent classes D2–D4. Get operator approval before the binary swap.

The defect is architectural, not project-local. The same bug bites every project on this machine — Codex CLI agents and Claude Code agents alike — because every CLI talks to the **same** Agent Mail HTTP MCP server at `127.0.0.1:8765` and the **same** storage root at `~/.mcp_agent_mail_git_mailbox_repo/`.

---

## 0 · Quick Summary (3 minutes — do read all of it)

- **Symptom:** `health_check` is `red` with `semantic_readiness.detail: archive inventory is ahead of the sqlite index (archive projects=1, agents=2, messages=1, latest_id=1, db projects=1, agents=0, messages=0, max_id=0)`. Reads (`list_agents`, `fetch_inbox`, `search_messages`) return data anyway because there is an explicit archive-snapshot fallback for the read path — `live_db_is_suspect` at `crates/mcp-agent-mail-tools/src/lib.rs:737-777` is that fallback, and it is *intended* (see the source `tracing::info!` it emits).
- **What we observed but did not yet *prove* in code:** mutating MCP calls (`register_agent`, `send_message`, `file_reservation_paths`) keep succeeding under red health. The 2026-05-09 registration of agent `JadeMink` and the 2026-05-18 smoke test's `MaroonMountain`/message/reservations were all written into the archive while SQLite remained empty (§1.5 reconstructs this timeline). **The question for the next agent is not "implement a gate" but "trace whether a write gate exists and why it isn't firing".** D1 in §4 is hypothesis-driven; do not start patching until you have read the actual entry points (`register_agent` at `identity.rs:1103`, `send_message` at `messaging.rs:1427`, `file_reservation_paths` at `reservations.rs:378`) and confirmed where the write actually lands.
- **Immediate state-level fix:** `am doctor reconstruct --yes`. Operator approval required. Bounded scope: only one project (`users-alinaaitova-desktop-testproject1`) exists under the storage root, so this is a small, surgical operation. Idempotent re-applies are safe. **Do not substitute `am doctor fix --yes`** — that command also rewrites shell rcs, MCP configs, hook installs, and can restart the server (see `am doctor fix --help`). Use the single-purpose `reconstruct`.
- **Upstream fix (server-side, Rust):** the cleanest framing is "make the write path symmetric with the existing read path". The read path consults `live_db_is_suspect` and redirects to archive snapshots; the write path needs an equivalent consult that *rejects* with a `DATABASE_DEGRADED` MCP error (already plumbed by commit `fc9b74dc` — verify the plumbing is reachable from the three tool entry points). Plus: the "best-effort warning" detection at `crates/mcp-agent-mail-server/src/lib.rs:13601-13640` should become more than a log line — see D2 in §4.
- **Wrapper fix (agent-side, Python):** `bin/br-claim` and `bin/br-complete` must perform a `health_check` preflight and refuse to mutate if `health_level=red` and the drift signature is present. Skill assets at `~/.claude/skills/install-bead-wrappers/assets/` AND `~/.codex/skills/install-bead-wrappers/assets/` must mirror byte-for-byte after the edit (METHODOLOGY.md already requires this — verified byte-identical at handoff time).
- **What's important about the dual-CLI angle:** the server is CLI-agnostic, so D1–D4 (Rust fixes) automatically protect both Claude and Codex agents. The Claude-vs-Codex split *only* matters for D5 (wrappers + skill), where you must edit both `~/.claude/skills/install-bead-wrappers/` and `~/.codex/skills/install-bead-wrappers/` in lockstep.

---

## 0.5 · Prerequisites (5 minutes)

Before you touch anything, confirm you can execute. The next agent inherits a half-broken system; verify you have what you need to work in it.

| Requirement | Probe | Expected |
|---|---|---|
| `am` binary on `PATH` | `which am && am --version` | `/Users/alinaaitova/.local/bin/am` and `am 0.2.54` |
| Agent Mail HTTP server reachable | `curl -s --max-time 3 http://127.0.0.1:8765/healthz` | `{"status":"alive"}` |
| MCP bearer token accessible | `cat ~/.config/mcp-agent-mail/config.env` (contains `HTTP_BEARER_TOKEN=...`) or `~/.claude.json` (in `mcpServers` entry's `headers.Authorization: Bearer ...`) | non-empty token |
| Source repo present + clean | `cd /Users/alinaaitova/Desktop/mcp_agent_mail_rust && git status --short` | empty (or only intentional changes) |
| `cargo` toolchain available | `cargo --version` | recent stable + the workspace's nightly per `rust-toolchain.toml` |
| Operator availability | (out-of-band) | yes — you will need explicit approval for §6 Step 1 and §6 Step 4 |
| `AGENT_MAIL_NAME` env var | `echo "$AGENT_MAIL_NAME"` (or `$AGENT_NAME`) | non-empty — required for wrapper smoke tests in §6 Step 2 |
| Pre-commit hook installed | `am guard status /Users/alinaaitova/Desktop/Testproject1` | `Pre-commit: installed`, `Pre-push: installed` |

If any probe fails, stop and report to the operator. Do not paper over a missing prerequisite — most of them are AGENTS.md Rule 2 requirements.

**Bearer token resolution order (in the wrappers today, see `bin/br-claim:50-70` and `bin/br-complete:50-66`):**

1. `AGENT_MAIL_BEARER_TOKEN` env var.
2. `~/.claude.json` → `mcpServers["mcp-agent-mail"].headers.Authorization` (`Bearer <token>` form).
3. Empty (the wrappers will then send unauthenticated requests, which the local-loopback server may or may not accept depending on its config).

There is a third canonical location not currently consulted: `~/.config/mcp-agent-mail/config.env` (which contains `HTTP_BEARER_TOKEN=<token>` as a single line). Codex agents using this machine do not necessarily have `~/.claude.json` — consider adding a fallback to read `config.env` when you touch the wrappers in §6 Step 2. This makes the wrappers portable to a Codex-only setup without an `~/.claude.json`.

---

## 1 · Current Confirmed State (2026-05-18T10:34:39Z)

Environment matches the error manifest exactly. Re-verified live before this handoff was written:

| Probe | Result |
|---|---|
| `am --version` | `am 0.2.54` |
| `sqlite3 --version` | `3.43.2` |
| `curl http://127.0.0.1:8765/healthz` | `{"status":"alive"}` |
| `health_check({})` | `status=error, health_level=red, recovery.mode=degraded_read_only, semantic_readiness.detail=archive inventory is ahead of the sqlite index (archive projects=1, agents=2, messages=1, latest_id=1, db projects=1, agents=0, messages=0, max_id=0)` |
| `list_agents` | returns `MaroonMountain`, `JadeMink` (archive-backed) |
| `fetch_inbox(MaroonMountain)` | returns message id=1 (archive-backed) |
| `search_messages("agentmail-smoke-20260518T1020Z")` | returns message id=1 (archive-backed) |
| Raw SQLite `agents` count | `0` |
| Raw SQLite `messages` count | `0` |
| Raw SQLite `projects` count | `1` (skeleton row from a partial reconstruct — see forensics below) |
| `am doctor reconstruct --dry-run` | would recover: projects=1, agents=2, messages=1, thread digests=1 |
| `storage.sqlite3.bak` (2026-05-18 11:41) | also has agents=0, messages=0 → **backup is poisoned**, cannot restore from it |
| Forensic bundles | `~/.mcp_agent_mail_git_mailbox_repo/doctor/forensics/storage.sqlite3/reconstruct-20260508_202737_002/` and `reconstruct-20260508_202805_335/` — from the original corruption events on 2026-05-08, both contain `manifest.json` + `summary.json` + a `sqlite/` and `references/` directory |
| `storage.sqlite3.corrupt-20260508_202737_018` and `…_202805_341` | preserved corruption copies, do not delete |
| Running server | PID **29404**, command `/Users/alinaaitova/.local/bin/am serve-http --host 127.0.0.1 --port 8765 --no-tui`, **continuously up since 2026-05-08 22:58** (~10 days) — the same process handled the May 8 corruption, the May 9 `JadeMink` write, and the May 18 smoke test |
| Installed binary mtime | 2026-05-06 17:58 (`ls -la /Users/alinaaitova/.local/bin/am`). Source HEAD `2862eebe` is dated ~6 minutes later (2026-05-06 18:04). The running binary is essentially current modulo HEAD's last commit. **`target/release/am` does not exist** in the source tree — operator installed via `install-local.sh` or an out-of-tree build. |
| Project scope under storage root | **Only one project:** `users-alinaaitova-desktop-testproject1`. `am doctor reconstruct` will touch only this project; no risk to unrelated workspaces. |

**What this tells you:**
- The DB was corrupted on 2026-05-08, a reconstruct bundle was written, but the SQLite index was never replayed — the file at `storage.sqlite3` is a fresh, empty (except for the `projects` skeleton row) schema. Two clean-up attempts left two `reconstruct-*` bundles.
- Every subsequent write since 2026-05-08 has gone *only* into the archive. The 2026-05-09 `JadeMink` registration and the 2026-05-18 smoke test agents/message/reservations all exist in the archive and not in SQLite.
- The drift is **growing**, not stable. Each agent that uses this machine adds to it.

---

## 1.5 · Inferred Timeline (evidence-backed)

Building a mental model of how the system reached this state is essential before you reason about *why* writes bypass SQLite. The clearest evidence the next agent has:

| When (UTC) | Source of evidence | Event |
|---|---|---|
| 2026-05-08 20:27:37 | `storage.sqlite3.corrupt-20260508_202737_018`, `doctor/forensics/.../reconstruct-20260508_202737_002/` | First detected corruption event. Server (presumably) crashed or hit an integrity error; an automatic reconstruct attempt was initiated. |
| 2026-05-08 20:28:05 | `storage.sqlite3.corrupt-20260508_202805_341`, `doctor/forensics/.../reconstruct-20260508_202805_335/` | Second corruption event ~28 seconds later. A second reconstruct attempt followed. |
| 2026-05-08 22:58 | `ps -ef` shows server PID 29404 started `8May26 22:58` | Server restarted (current long-running process). |
| 2026-05-08 ≈ 22:58 | SQLite `projects` row exists; `agents`/`messages` empty | Recovery on restart only re-created the `projects` skeleton row; agents/messages were *not* replayed from archive. |
| 2026-05-09 12:59:41 | `agents/JadeMink/profile.json` `inception_ts` | First post-corruption agent registration. **Wrote to archive only** — SQLite `agents` count remained 0. |
| 2026-05-18 10:19:52 | `agents/MaroonMountain/profile.json` `inception_ts` | Codex smoke test registered an agent. Wrote to archive only. |
| 2026-05-18 10:20:26 | `messages/2026/05/2026-05-18T10-20-26Z…__1.md` | Smoke-test self-message. Wrote to archive only. |
| 2026-05-18 10:20:49 | `file_reservations/id-1.json` | Smoke-test reservation. Wrote to archive only. |
| 2026-05-18 10:20:52 | `file_reservations/id-1.json` `released_ts` | Same reservation released. Release recorded in archive only. |
| 2026-05-18 11:41 | `storage.sqlite3.bak` mtime | Backup written — and the backup already contains the drifted empty index (agents=0, messages=0). |
| 2026-05-18 10:34:39 | `health_check` JSON in §1 | First confirmed reproduction of `health_level=red` for this manifest. |

**Two implications for tracing:**

1. The window between the May 8 restart and the May 9 `JadeMink` registration (≈14 hours) is the first proof point that writes bypass SQLite. If the next agent assumes the bug appeared with the May 18 smoke test, they will mis-target. The bug is at least 9 days old.
2. The running server is the **same process** that lived through every event above. If the durability state machine has any "state once latched, never lowered" semantics, the May 8 corruption may have left the process in a permanent-degraded mode that survives subsequent recoveries. If the state machine is *recomputed on every request* (more likely, given `compute_mailbox_verdict` is called on demand), then the durability state is recomputed cheaply and the bug is in the writer not consulting that verdict — not in the state machine itself. **This is the first thing to verify in §3 reading order.**

---

## 2 · The Architecture Mental Model (read this before touching code)

Agent Mail has **two stores** that must agree:

| Store | Path | Role |
|---|---|---|
| **Archive** (git-backed, plain files) | `~/.mcp_agent_mail_git_mailbox_repo/projects/<slug>/{agents,messages,file_reservations,project.json}` | **Canonical durability** — agent profiles, message Markdown bodies with JSON frontmatter, reservation JSONs. Survives DB corruption. |
| **SQLite index** | `~/.mcp_agent_mail_git_mailbox_repo/storage.sqlite3` | **Query/index layer.** Tables: `projects`, `agents`, `messages`, `message_recipients`, `file_reservations`, `file_reservation_releases`, `agent_links`, plus search/ATC/tooling tables. Plus a separate `search_index/` Tantivy-ish directory next to it for full-text. |

A healthy write hits **both** atomically (archive first, then SQLite index). The SQLite index is the *fast path*; the archive is the *source of truth*. When the SQLite index is suspect, the system enters one of the durability states defined in `crates/mcp-agent-mail-core/src/mailbox_durability.rs`:

```
//! Healthy → Suspect → Broken → DegradedReadOnly → Recovering → Healthy
//!         └─ safe snapshot available ──────────► DegradedReadOnly
//!         └─ evidence worsens ─────────────────► Suspect | Broken | DegradedReadOnly
```

`DegradedReadOnly` is the contract this defect tests. Per the source's own documentation:

- `crates/mcp-agent-mail-tools/src/lib.rs:737-758`:

  > `Check whether the live SQLite database is suspect (DegradedReadOnly or worse). … DegradedReadOnly — reads should come from archive snapshots.`

- `crates/mcp-agent-mail-tools/src/messaging.rs:3060`:

  > `// snapshots when the live SQLite is suspect (DegradedReadOnly).`

- `crates/mcp-agent-mail-tools/src/resources.rs:300`:

  > `DegradedReadOnly (or worse), resource reads should fall back to archive snapshots.`

So **reads** falling back to the archive is *intended*. That fully explains why `list_agents`, `fetch_inbox`, `search_messages` keep working under red health.

The **write contract** of `DegradedReadOnly` is, per its name and the state-machine doc-comment, **read-only**. The 2026-05-18 smoke test produced new archive entries (agent, message, reservations) while the server was already in `degraded_read_only`, and those entries are absent from SQLite — so *something* in the write path is bypassing whatever gate exists for this state. Whether that "something" is (a) an entirely missing write gate, (b) a verdict cache that latched stale-healthy, or (c) a per-tool short-circuit that skips the verdict for already-known entities is the **unresolved question for the next agent** — §4 D1 enumerates the hypotheses and §3.0 the reading order to discriminate between them. Do **not** assume (a) and patch on that basis; trace first.

---

## 3 · Exact Source Map for the Upstream Fix

Repository: `/Users/alinaaitova/Desktop/mcp_agent_mail_rust/` (workspace branch `fix/immediate-read-after-write`; HEAD `2862eebe`; workspace version `0.2.54`). Several recent commits already touch adjacent issues — read those before touching the same hot path:

```
2862eebe fix(verdict): tolerate FrankenSQLite snapshot-advance visibility window
db14eb65 fix(db-pool): use canonical SQLite for inventory probes (FrankenSQLite drift)
59f52e05 fix(tools): wire archive_sqlite_degraded_error into db_error_to_mcp_error
fc9b74dc fix(tools): surface DATABASE_DEGRADED + canonicalize macOS snapshot dir
```

`fc9b74dc` and `59f52e05` already plumb a `DATABASE_DEGRADED` MCP error (per their commit *subjects* — verify by reading the diffs, since this handoff did not). **The hypothesis is that the plumbing is reachable from some paths but not from the three mutating MCP tools that participated in the smoke test.** Confirm by reading the tool entry points listed below before changing anything.

### 3.0 Reading order (do this in sequence — do not skim)

The single highest-leverage piece of context is whether the durability state is **computed per-request** or **latched at startup**. The fast way to answer:

1. **`crates/mcp-agent-mail-db/src/mailbox_verdict.rs`** — read top-to-bottom. This is the verdict subsystem (`compute_mailbox_verdict`, `VerdictOptions::fast()`) the rest of the codebase calls into for fresh assessments. Read it until you can answer: *is the verdict cached, or recomputed on each call?*
2. **`crates/mcp-agent-mail-core/src/mailbox_durability.rs`** — the state enum + transition table. Note `DEGRADED_READ_ONLY_CONTRACT` (≈line 111). Look for any field that documents allowed *write* operations; the absence of such a field is itself a finding.
3. **`crates/mcp-agent-mail-tools/src/lib.rs:737-777`** — `live_db_is_suspect` (read-side gate). Quoted partially in §0. This is the *template* the write gate should mirror. Read the full function and the two callers (search for `live_db_is_suspect(` in the same file). Note that it currently has no write-side counterpart called `live_db_blocks_writes` or similar — *that is the most likely site of the bug*.
4. **`crates/mcp-agent-mail-tools/src/identity.rs:1103`** — `pub async fn register_agent`. Walk the body. Does it call `live_db_is_suspect` (or anything analogous)? Does it return `DATABASE_DEGRADED` for any branch? Does it write to archive *then* to SQLite, or write to both in one transaction? **Answering this localizes the bug.**
5. **`crates/mcp-agent-mail-tools/src/messaging.rs:1427`** — `pub async fn send_message`. Same questions.
6. **`crates/mcp-agent-mail-tools/src/reservations.rs:378`** — `pub async fn file_reservation_paths`. Same questions.
7. **`crates/mcp-agent-mail-server/src/lib.rs:13601-13640`** — the "best-effort warning" detection (one of the few places in the codebase that *actively* reacts to drift rather than just describing it). Understand what triggers it and what it currently does.

Time-box this reading to ~30 minutes. If you can't answer the verdict-cached-or-recomputed question in 30 minutes, escalate to the operator — that fact gates the entire fix shape.

### 3.1 Drift detection (server-side)

- **`crates/mcp-agent-mail-server/src/lib.rs:2034`** — primary `health_check` builder. The literal string `"archive inventory is ahead of the sqlite index (archive projects={}, agents={}, messages={}, latest_id={}, db projects={}, agents={}, messages={}, max_id={}{})"` is formatted here. This is the human-readable diagnostic the manifest reproduced.
- **`crates/mcp-agent-mail-server/src/lib.rs:13506`** — error branch matching the drift string at runtime.
- **`crates/mcp-agent-mail-server/src/lib.rs:13601-13640`** — a **best-effort warning** path (the comment literally says "Best-effort warning: archive inventory is ahead of the live SQLite index"). This is the place that *detects* the drift but only *logs* it. It is the most obvious place to either (a) escalate to a hard failure on mutating paths, or (b) trigger an auto-reconstruct attempt with a backoff lock.
- **`crates/mcp-agent-mail-server/src/lib.rs:15285, 15393`** — tests that already assert the drift detection fires; do not break them, but extend them to also assert the new write-rejection behavior.
- **`crates/mcp-agent-mail-tools/src/lib.rs:496`** — tools-side variant of the same message (matches an "or the sqlite index is unavailable" condition).
- **`crates/mcp-agent-mail-tools/src/lib.rs:526`** — emits the `"mode": "degraded_read_only"` JSON key returned through the MCP envelope.
- **`crates/mcp-agent-mail-tools/src/identity.rs:527, 2548, 2675`** — identity-tool copies of the same diagnostic; keep them in lockstep.
- **`crates/mcp-agent-mail-tools/src/identity.rs:170-187`** — **not a write gate**, despite the `DurabilityState::DegradedReadOnly` arm at line 172. This block builds the `recovery.next_action` *string* (the message returned in `health_check.recovery.next_action` and the same message the manifest captured). When the recovery lock is inactive it returns `"Run \`am doctor reconstruct --dry-run\` to inspect archive-backed recovery, then \`am doctor reconstruct --yes\` if needed"`. If you want to redirect operators to `am doctor fix --dry-run` first, this is the line to edit. Cosmetic, not the bug.

### 3.1a Read-side gate (the *template* for the missing write-side gate)

- **`crates/mcp-agent-mail-tools/src/lib.rs:737-777`** — `live_db_is_suspect(database_url, storage_root, sqlite_path) -> bool`. This is the existing read-path gate. The body:
  1. Calls `mcp_agent_mail_db::compute_mailbox_verdict(...)` with `VerdictOptions::fast()`.
  2. Derives `DurabilityState::from_mailbox_state(verdict.state)`.
  3. Calls `verdict_prefers_archive_snapshot_reads_for_primary_read_surface(&verdict, sqlite_path)`.
  4. If `prefer_archive && durability.allows_reads()` → returns `true` (route reads to archive). Emits a `tracing::info!` with `"live SQLite is suspect; read surfaces will prefer archive snapshots"`.
  5. If `prefer_archive && !durability.allows_reads()` → returns `true` with a `tracing::warn!` for `"live SQLite is corrupt/recovering; read surfaces will attempt archive snapshot fallback"`.
- The fix for D1 is to add a peer function — call it `live_db_blocks_writes(...) -> bool` — that consults the *same* verdict and `DurabilityState` but inverts the disposition: if the live DB is suspect, return true so the mutating tool can return `DATABASE_DEGRADED`. Mirror the same `compute_mailbox_verdict` call so write-side and read-side stay coherent.

### 3.2 Durability state machine

- **`crates/mcp-agent-mail-core/src/mailbox_durability.rs`** — single source of truth for the state enum and contracts. Note `DEGRADED_READ_ONLY_CONTRACT` (line 111). Look for the field that lists *allowed* operations under each contract; the write-allow flag for `DegradedReadOnly` is the leak.

### 3.3 Mutating tool entry points (where the rejection must be added)

These are the **actual function bodies** to walk; not the prior `DegradedReadOnly` lines at 172/644 in identity.rs (those build *strings*, see §3.1).

- **`crates/mcp-agent-mail-tools/src/identity.rs:1103`** — `pub async fn register_agent(...)`. Primary culprit of the smoke-test write. Adjacent entry points in the same file: `create_agent_identity`, `resolve_pane_identity` (search for `pub async fn`).
- **`crates/mcp-agent-mail-tools/src/messaging.rs:1427`** — `pub async fn send_message(...)`. Also covers `reply_message`, `acknowledge_message`, `mark_message_read` — confirm each gates writes. The comment at line 3060 ("snapshots when the live SQLite is suspect (DegradedReadOnly)") is in the *read* branch — verify the write branch makes a symmetric decision.
- **`crates/mcp-agent-mail-tools/src/reservations.rs:378`** — `pub async fn file_reservation_paths(...)`. Also `release_file_reservations`, `force_release_file_reservation`, `renew_file_reservations`. **Highest blast radius:** reservations are the collision detector that AGENTS.md Rule 2 relies on. A silent archive-only write here corrupts collision detection for every project on the machine. Walk this function first.
- **`crates/mcp-agent-mail-tools/src/resources.rs`** — line 300 documents the read-fallback rule. Audit any write paths (e.g. `ensure_project`, `acquire_build_slot`).

For each entry point, the call sequence to confirm or add is:

```text
1. Resolve database URL, storage root, sqlite path from config (existing).
2. Consult live_db_blocks_writes(...)  ← NEW peer of live_db_is_suspect.
3. If blocked → return DATABASE_DEGRADED MCP error with recovery payload
   that mirrors health_check.recovery (so callers like bin/br-claim can
   parse the same shape).
4. Otherwise → existing archive write + SQLite write path.
```

Keep step 2 a single function so the four tools stay in sync; pasting the same `compute_mailbox_verdict` call into each entry point is a pattern this codebase has already learned to centralize (compare the read-side `live_db_is_suspect`).

### 3.4 Reconstruction code path

- **`crates/mcp-agent-mail-db/src/reconstruct.rs`** — the function backing `am doctor reconstruct --yes`. Notable missing fields the reconstruct cannot recover (documented at the top of the file):
  - `read_ts` / `ack_ts` on `message_recipients` (no archive artifact)
  - `agent_links` / contacts (handshake state not archived)
  - `products` / `product_project_links` (not archived)
- Recovery flows merge those DB-only rows back from a salvageable DB if available — read the `salvage_*` static atomics for test hooks. **Consequence for this case:** running `am doctor reconstruct --yes` will rebuild agents and messages but will *not* re-create any contact handshakes that existed only in SQLite. That's an acceptable loss for this project today (the only handshake state visible was the smoke-test agents which were self-only) but mention it to the operator before they approve the repair.

### 3.5 CLI / robot

- **`crates/mcp-agent-mail-cli/src/robot.rs:780`** — the CLI's documented mode label list (`"healthy" | "degraded_read_only" | "recovering" | "corrupt"`).
- **`crates/mcp-agent-mail-cli/src/robot.rs:2715, 2741, 3110, 16051`** — emission points for the mode through the CLI. Keep wording consistent with whatever the MCP tools return.

### 3.6 Startup checks

- **`crates/mcp-agent-mail-server/src/startup_checks.rs:1731, 1748`** — startup-time durability arms. If you elect "auto-reconstruct on detection", this is where you'd trigger it (single-shot, behind a recovery lock — see `recovery.recovery_lock_active` already in `health_check`'s payload).

---

## 4 · The Five Defect Classes to Fix

### D1 — Mutating tools succeed despite drift (highest severity, **hypothesis-driven**)

**Hypothesis:** `register_agent`, `send_message`, `file_reservation_paths` complete (returning archive-side success) when `live_db_is_suspect == true`; SQLite is never updated. Each call therefore widens the archive-vs-SQLite drift silently.

**Evidence we have:**
- Archive contains rows that SQLite does not (§1, §1.5).
- The most recent of these (2026-05-18 smoke test) was created while `health_check` was already reporting red.
- There is a documented read-side gate (`live_db_is_suspect` at `lib.rs:737-777`) but no obvious write-side peer.

**Evidence we do NOT yet have, and that the next agent must collect before patching:**
1. Walk `register_agent` at `identity.rs:1103` and the matching `send_message`/`file_reservation_paths` entries. Do they call any function whose name suggests "is degraded?" / "is suspect?" / "is locked?". If yes — does it return early?
2. Reproduce: with the server still red, call `register_agent(name="DiagnosticAgent", ...)`. If it returns `DATABASE_DEGRADED` you are *wrong about D1* — the gate exists but is firing inconsistently with the smoke test's path (look for an LRU cache of verdicts, a stale durability state in a long-running task, or a different code branch when the project already exists).
3. Repeat for `send_message` (with a thread_id distinct from the smoke test) and `file_reservation_paths` (with a fresh path pattern).

**If hypothesis confirmed (the most likely outcome based on evidence):** add a `live_db_blocks_writes` peer to `live_db_is_suspect` (§3.1a) and call it from each mutating entry point (§3.3). Return the `DATABASE_DEGRADED` MCP error (plumbing landed in commit `fc9b74dc` — verify by reading that commit's diff). The error's response payload should mirror `health_check.recovery` so `bin/br-claim` and `bin/br-complete` (after their preflight in D5) can parse a single shape.

**If hypothesis falsified (some write paths do gate, others don't):** narrow to the path that doesn't gate. The most likely culprit then is a per-tool short-circuit that bypasses the central gate when an entity already exists (e.g., `register_agent` on an already-known agent name might skip the verdict check and go straight to a "touch last_active_ts" path that only updates archive).

**Tests (once you know which it is):** extend `crates/mcp-agent-mail-tools/tests/system_error_parity.rs` and `validation_error_parity.rs` with a "DegradedReadOnly rejects writes" parity case (one per mutating tool). The existing test `register_agent_under_sqlite_lock_maps_to_resource_busy` at `system_error_parity.rs:156` is the *closest* template, but note: it induces a transient SQLite BUSY lock, not a persistent `DegradedReadOnly` durability state. You will need a new fixture that primes the storage root with an archive-without-SQLite drift — see `crates/mcp-agent-mail-test-helpers` for the standard setup primitive and `crates/mcp-agent-mail-db/src/reconstruct.rs`'s tests for a worked example of authoring archive-only state.

### D2 — Detection without action

**Symptom:** `health_check` correctly reports drift; nothing reacts. The "best-effort warning" at `lib.rs:13601-13640` is the highest-leverage detection site and currently only logs.

**Fix shape (preferred):** when drift is detected at startup *and* `recovery.recovery_lock_active=false`, the server should attempt an in-process `reconstruct::run(storage_root)` call (the same code path as `am doctor reconstruct --yes`) behind a single-flight lock. Successful auto-reconstruct should transition the state machine `DegradedReadOnly → Recovering → Healthy` and clear the warning. Failed auto-reconstruct should leave the warning *and* surface a structured `recovery.last_attempt_error` field in `health_check`.

**Fix shape (minimum):** if auto-reconstruct is too invasive for this branch, add a periodic stderr warning (every 60s while degraded) so a running server log is loud about the state. Keep the `recovery.next_action` text pointing at `am doctor reconstruct --yes` (the single-purpose, surgical command) — do **not** rewrite it to `am doctor fix --yes`, because `fix --yes` also rewrites shell rcs / MCP configs / hooks (per `am doctor fix --help`), which is far broader than this defect warrants. If you want to surface the unified path, add a *separate* sentence: "If you also want shell/PATH/hook remediation, see `am doctor fix --dry-run`."

### D3 — `recovery.mode=degraded_read_only` is misleadingly named

**Symptom:** the contract claims read-only; the current implementation allows writes (D1). Even after D1 lands, the name still implies "writes are blocked" with no signal about *what* writes are blocked (mutating MCP tools, archive append, both?).

**Fix shape:** keep the variant name (it's part of the public CLI surface — `robot.rs:780` documents it) but extend the `recovery` block in `health_check` and in each rejected mutating tool's response with a `recovery.write_policy` field (`"reject_all_mutations" | "archive_only" | "full"`). Adopting "`reject_all_mutations`" under D1 makes the contract honest.

### D4 — Backup discipline doesn't validate before snapshotting

**Symptom:** `storage.sqlite3.bak` was written on 2026-05-18 11:41 — *after* the 2026-05-08 corruption — and inherited the empty-index state. The backup is therefore useless for this case. `am doctor restore <BACKUP_PATH>` would restore garbage.

**Fix shape:** before writing a rotated backup, run the same drift probe used by `health_check`. If the source DB is suspect or already drifted vs. archive, *skip* the backup (or write it to a separate `suspect/` directory that `am doctor restore` refuses by default). Source: `crates/mcp-agent-mail-server/src/backup_rotation.rs`.

### D5 — Wrapper and skill preflight gap (this is the cross-CLI part)

**Symptom:** `bin/br-claim` and `bin/br-complete` (Python; project-local) call MCP tools but never call `health_check` first. When the server is red, the wrappers happily call `file_reservation_paths` and `send_message`, treating archive-only success as a real success. Every project on the machine that installed via `install-bead-wrappers` has this same gap.

**Fix shape:**

1. Add a `preflight_health` step at the very top of `main()` in both `bin/br-claim` and `bin/br-complete` — *before* `--dry-run` is honored, so dry-run never paints a misleading "OK" picture on a broken server. The step:
   - Calls MCP `health_check({})`.
   - If `health_level == "red"` AND `semantic_readiness.detail` contains `"archive inventory is ahead"`: `die()` with the exact `recovery.next_action` string echoed verbatim from the response (currently: `Run \`am doctor reconstruct --dry-run\` to inspect archive-backed recovery, then \`am doctor reconstruct --yes\` if needed`). Append an explicit "do not retry until the operator has repaired the index" tail. **Do not hard-code `am doctor fix --yes`** — that command is unified and does too much (see §6 Step 1); always echo whatever the server is currently telling you in `recovery.next_action` so the message stays correct if the server fix in D2 changes it later.
   - Treat any other non-success state (yellow/amber if the server supports it, or `queues.*.warning=True`) as `warn()` and continue. Note: at handoff time the only state observed was `red`; verify the exact set of `health_level` values by grepping `crates/mcp-agent-mail-server/src/lib.rs` for the strings the server emits, rather than assuming.
2. Update both skill copies:
   - `~/.claude/skills/install-bead-wrappers/assets/br-claim`
   - `~/.claude/skills/install-bead-wrappers/assets/br-complete`
   - `~/.codex/skills/install-bead-wrappers/assets/br-claim`
   - `~/.codex/skills/install-bead-wrappers/assets/br-complete`
   …with the same hardened wrapper bytes. METHODOLOGY.md (§"Why dual-CLI symmetry") already requires byte-for-byte mirroring; enforce it with `diff -r ~/.claude/skills/install-bead-wrappers/ ~/.codex/skills/install-bead-wrappers/` printing nothing.
3. Bump the snapshot stamp in `~/.claude/skills/install-bead-wrappers/METHODOLOGY.md` (look for the "Snapshot commit" line — currently `53a6adf` (2026-05-08)) to the new commit SHA that lands these wrapper changes in `Testproject1`. Mirror to Codex.
4. **Bearer token Codex fallback (per §6.5 #6):** while editing `resolve_bearer_token()` to add the preflight, also add `~/.config/mcp-agent-mail/config.env` as a third resolver. Insertion point: after the existing `~/.claude.json` lookup at `br-claim:58-69` and `br-complete:53-66`, fall through to parsing the `HTTP_BEARER_TOKEN=...` line from `config.env`. ~10 lines of Python; makes the wrappers Codex-portable.
5. Add a new server-health probe step to `~/.claude/skills/install-bead-wrappers/SKILL.md` (mirror to Codex). It belongs in Step 1 (Pre-flight) alongside the existing `/healthz` curl, since semantic-red should fail the install just like an unreachable HTTP server:

   ```bash
   # (existing) liveness probe
   curl -s --max-time 3 http://127.0.0.1:8765/healthz | grep -q '"status":"alive"' \
     || { echo "agent-mail HTTP not alive" >&2; exit 1; }

   # (new) semantic-readiness probe — verify the strings against `am doctor check`'s
   # actual output format before relying on these greps; the names may differ.
   DOCTOR_OUT="$(am doctor check 2>&1 | tee /tmp/install-bead-wrappers-doctor.log)"
   if echo "$DOCTOR_OUT" | grep -qE 'archive inventory is ahead|degraded_read_only|health_level.*red'; then
     echo "agent-mail in degraded mode — operator must run 'am doctor reconstruct --yes' (single-purpose, surgical) before installing the wrappers in a new project" >&2
     exit 1
   fi
   ```

   This catches the case where a fresh `install-bead-wrappers` run on a *new* project would otherwise paper over a server-level outage. **Do not** recommend `am doctor fix --yes` in the failure message — it's the unified path with shell/PATH/hook side-effects, broader than this defect warrants (see §6 Step 1).

**Note on local wrappers vs. skill assets:** the manifest already showed that `bin/br-claim` and `bin/br-complete` in *this* project diverge from the skill assets (`cmp` reported non-zero). That's expected — the canonical wrappers live in the project and the skill snapshots them after the fact. Land the preflight in the project wrappers first, smoke-test, then re-snapshot to the skill per METHODOLOGY.md's "Wrapper snapshot stamp" procedure.

---

## 5 · Cross-CLI Considerations (the user's #3 ask)

Both **Claude Code** and **Codex CLI** use the *same* Agent Mail server (HTTP at `127.0.0.1:8765`). The server is CLI-agnostic. So:

- **Server fixes (D1–D4):** apply automatically to both CLIs. There is no "Claude vs Codex" axis in the Rust source — there is only "MCP tool entry point". You do **not** need to ship parallel server fixes.
- **Wrapper fixes (D5):** the wrappers are shell scripts that any CLI invokes (`bin/br-claim`, `bin/br-complete`). Their preflight changes are CLI-agnostic by construction.
- **Skill fixes (D5):** the `install-bead-wrappers` skill is mirrored byte-for-byte between `~/.claude/skills/install-bead-wrappers/` and `~/.codex/skills/install-bead-wrappers/`. Both directories must be updated as a single atomic change. Use `cp -r ~/.claude/skills/install-bead-wrappers/. ~/.codex/skills/install-bead-wrappers/` then verify with `diff -r` printing nothing. METHODOLOGY.md §"Why dual-CLI symmetry" is the authority on this.

Sanity check the dual-presence assumption before you begin: this manifest was generated from a Codex agent and the matching Claude skill copy was confirmed present on 2026-05-18:

```bash
diff -r ~/.codex/skills/install-bead-wrappers/ ~/.claude/skills/install-bead-wrappers/   # must print nothing
```

If they have already drifted, fix that *first* by mirroring `~/.claude/skills/install-bead-wrappers/` to Codex, then proceed.

---

## 6 · Execution Plan (recommended order)

Each step is gated on the previous. Don't combine steps; the verification value comes from re-running `health_check` and the SQLite counts between each one.

### Step 0 — Re-confirm current state (read-only, no approval needed)

```bash
curl -s --max-time 3 http://127.0.0.1:8765/healthz
sqlite3 ~/.mcp_agent_mail_git_mailbox_repo/storage.sqlite3 \
  "SELECT 'agents',count(*) FROM agents UNION ALL SELECT 'messages',count(*) FROM messages;"
am doctor reconstruct --dry-run
```

Plus MCP `health_check({})`. Expected: still red, still archive ahead. If the state changed unexpectedly, STOP and re-read this handoff — the upstream may have already auto-repaired.

### Step 1 — Operator-approved immediate state repair

Tell the operator exactly what will run, what files will change, and what could be lost. Specifically:

- **Use `am doctor reconstruct --yes` — NOT `am doctor fix --yes`.** The `fix` subcommand is a unified remediation that *also* rewrites shell rc PATH order, edits MCP configs, installs guard hooks, removes `.git/index.lock` files, and can stop/restart the local server (see `am doctor fix --help`). With `--yes` it does all of those without prompting. The operator did not ask for any of that. Use the single-purpose `reconstruct`.
- Scope is bounded: only one project (`users-alinaaitova-desktop-testproject1`) exists under the storage root; the dry-run already confirmed projects=1, agents=2, messages=1, thread digests=1. No risk to unrelated projects.
- What can be lost: `agent_links` / contact handshake state, `products` / `product_project_links`, and `read_ts` / `ack_ts` on `message_recipients` cannot be reconstructed from archive (per `crates/mcp-agent-mail-db/src/reconstruct.rs`'s top comment). For *this* project there is no such state currently — the only agents are self-only smoke-test identities — so the loss is empty. Confirm with the operator anyway.

Then, on explicit "go", run:

```bash
am doctor reconstruct --dry-run | tee /tmp/agent-mail-reconstruct-before.log
am doctor reconstruct --yes      | tee /tmp/agent-mail-reconstruct-after.log
```

Then re-verify the full §1 probe table. The acceptance bar:

- `health_check` is `green` (or at minimum `semantic_readiness.status=pass` and not the drift detail).
- SQLite `agents >= 2`, `messages >= 1`, `file_reservations` count matches archive JSONs.
- `list_agents`, `fetch_inbox`, `search_messages` still return the same payloads (now backed by SQLite, not the archive fallback).

Record before/after counts and the `health_check` JSON deltas in `docs/handoff-2026-05-18-cdp-stack-validation-test.md`'s sibling, e.g. `docs/handoff-2026-05-18-agent-mail-index-drift-repair.md`.

### Step 2 — Wrapper preflight (Defect D5)

This is the highest-leverage *defensive* fix and the easiest to land. Do it BEFORE the Rust changes so the wrappers protect every other project on the machine while you work on the server.

**Ordering subtlety:** if you have already completed §6 Step 1 (immediate state repair), the live server is now green and you cannot easily verify the preflight refuses on red. Two options, pick one:

- **(Preferred)** Test the preflight against a *mocked* `health_check` response. Write a tiny Python harness that monkey-patches `urllib.request.urlopen` to return a canned red-state JSON, then invoke `bin/br-claim`'s preflight from a `python3 -c` one-liner. No live server changes required. Document the harness in the wrapper or in a sibling test script.
- **(Fallback)** If you really need a live-red verification, do it *before* §6 Step 1 — i.e., reorder so wrapper Steps 2.1–2.3 happen first against the still-broken server. Be explicit with the operator if you choose this order.

Then:

1. Edit `bin/br-claim` and `bin/br-complete` in this project to add a `preflight_health()` call at the very top of `main()` — before `--dry-run` is honored. Also add `~/.config/mcp-agent-mail/config.env` to `resolve_bearer_token()` as a third resolver (per §4 D5 step 4 and §6.5 #6). See §4 D5 for full fix shape.
2. Smoke test: `bin/br-claim --help`, `bin/br-complete --help` succeed. With your mock harness (or a still-red server), confirm the preflight echoes `recovery.next_action` verbatim and exits non-zero.
3. Smoke test against a green server: confirm the preflight is silent on success and the wrapper proceeds normally.
4. Re-snapshot to both skill copies per METHODOLOGY.md (`cp` to `~/.claude/skills/install-bead-wrappers/assets/`, then `cp -r` mirror to `~/.codex/skills/install-bead-wrappers/`). Update the `METHODOLOGY.md` "Snapshot commit" stamp.
5. Add the new server-health probe to `~/.claude/skills/install-bead-wrappers/SKILL.md` Step 1 Pre-flight (§4 D5 step 5). Mirror to Codex. Verify `diff -r` between the two skill directories prints nothing.

### Step 3 — Upstream Rust fixes (Defects D1, D2, D3, D4)

Branch from `fix/immediate-read-after-write` (current HEAD `2862eebe`). One atomic change per defect class — each in its own commit so a revert is surgical:

1. **D1 (write rejection in `DegradedReadOnly`)** — touch the four files in §3.3 plus the parity tests in `crates/mcp-agent-mail-tools/tests/*.rs`. Verify with `cargo test -p mcp-agent-mail-tools` and the workspace conformance suite `cargo test -p mcp-agent-mail-conformance`.
2. **D2 (auto-reconstruct or louder signaling)** — modify `crates/mcp-agent-mail-server/src/lib.rs:13601-13640` plus `crates/mcp-agent-mail-server/src/startup_checks.rs:1731-1748`. If you go with the auto-reconstruct path, you must take the `recovery_lock_active` lock (already in `health_check`'s payload), and the existing single-flight machinery near `recovery.bundle_path` writes — don't reinvent.
3. **D3 (`recovery.write_policy` field)** — adds a new field; schema migration not needed (it's response-only), but every consumer that pattern-matches on `recovery` mustn't blow up on the new field. The wrappers from Step 2 already tolerate extra keys.
4. **D4 (suspect-aware backup rotation)** — `crates/mcp-agent-mail-server/src/backup_rotation.rs`. Add a probe call before writing the rotated copy; on suspect, write to a `suspect/` subdirectory and update `am doctor backups` to surface "suspect backups need operator review" without offering them in `am doctor restore` without `--force`.

Run `cargo fmt`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, then `cargo build --release -p mcp-agent-mail-cli`. Stage the new binary at `~/.local/bin/am` only after explicit operator approval to upgrade. Note that the operator's current binary is the canonical `am` used by every active server — *don't* swap it under a live MCP session; the operator must restart the HTTP server.

### Step 4 — End-to-end verification (after binary upgrade)

1. Stop the running HTTP server: `pkill -INT -f 'am serve-http'` (operator to confirm no in-flight session) and restart with the new binary.
2. Re-run §1's probe table; expect green.
3. Re-run the 2026-05-18 smoke test as a regression: register a throwaway agent, send a self-message, create + release a reservation, then `fetch_inbox`, `search_messages`, and re-check SQLite counts. The new bytes must be in *both* the archive *and* SQLite.
4. **Validate D1/D2/D3 in the test suite, not by re-corrupting the live system.** Re-corrupting the live `storage.sqlite3` (truncating tables, swapping in a fresh empty DB) is risky and unnecessary — Rust integration tests in `crates/mcp-agent-mail-tools/tests/` can prime the storage root with archive-without-SQLite drift through `crates/mcp-agent-mail-test-helpers/` fixtures (the same pattern the new D1 parity tests use). Run:

   ```bash
   cargo test -p mcp-agent-mail-tools --test system_error_parity rejects_writes_in_degraded_read_only -- --nocapture
   cargo test -p mcp-agent-mail-tools --test validation_error_parity rejects_writes_in_degraded_read_only -- --nocapture
   cargo test -p mcp-agent-mail-conformance -- --nocapture
   ```

   Then confirm at runtime:
   - `health_check` on the live (green) server still returns `recovery.write_policy="full"` (D3 new field).
   - The new `recovery.last_attempt_error` field appears in `health_check` after a drift incident (D2 — only if you shipped auto-reconstruct).

   Only if the operator explicitly approves runtime drift-injection: stop the server, `mv ~/.mcp_agent_mail_git_mailbox_repo/storage.sqlite3 …/storage.sqlite3.test-aside`, restart with an empty index, and confirm the production write-rejection path. **Restore the saved DB or rerun reconstruct afterwards** — never leave the operator's machine in a degraded state.

---

## 6.5 · Open Questions — answer these by reading code, not by patching

These are the unknowns that will steer the fix. **None can be assumed.** Resolve them before §6 Step 3.

1. **Is the durability state recomputed on each request or latched?** `mcp_agent_mail_db::compute_mailbox_verdict(..., VerdictOptions::fast())` looks like an on-demand probe (per `live_db_is_suspect` in `lib.rs:737-777`). But there may be a server-level state cache that latches `DegradedReadOnly` after first detection. If latched, restarting the server *might* clear the state — relevant for D2 and for the §6 Step 4 validation procedure.

2. **Does `register_agent` short-circuit on existing agents?** The smoke-test agent `MaroonMountain` was newly registered, but `JadeMink` could plausibly have been registered before the corruption and then "touched" later. Walk `register_agent` at `identity.rs:1103` and check whether the existing-vs-new branch consults the verdict.

3. **Does `am doctor reconstruct` rebuild the `search_index/` directory or only `storage.sqlite3`?** The storage root has a separate Tantivy-ish `search_index/` directory used by `search_messages`. If reconstruct only touches SQLite, full-text search may still be stale post-repair. Read `crates/mcp-agent-mail-db/src/reconstruct.rs` for what it does — if search isn't covered, surface this to the operator before approving Step 1 and check whether a separate `am search reindex` or similar exists.

4. **Why didn't the May 8 reconstruction bundles get applied?** Two `doctor/forensics/storage.sqlite3/reconstruct-*` directories exist with `manifest.json` and `summary.json` inside. They look like the output of an *attempted* reconstruct that wrote forensic evidence but never wrote the rebuilt SQLite back. Worth understanding before D2 (auto-reconstruct) is designed — the auto-reconstruct must not replicate whatever failure mode left those bundles half-applied.

5. **Is `DegradedReadOnly` recoverable in-process?** Per the state machine in `mcp-agent-mail-core/src/mailbox_durability.rs` (lines around 366, 396), the transitions `Recovering → DegradedReadOnly → Healthy` are documented. But is "transition to Healthy" actually wired anywhere as a *trigger* (e.g., a post-reconstruct hook), or does the state only transition on the next verdict probe? If the latter, the `am doctor reconstruct --yes` in Step 1 will rebuild SQLite but the server's cached state — if any — will not flip until the next `health_check`. The §6 Step 1 verification loop should explicitly re-call `health_check` last.

6. **Bearer token resolution under Codex.** `bin/br-claim:53-70` resolves the bearer token from `AGENT_MAIL_BEARER_TOKEN` or `~/.claude.json`. Codex agents do not necessarily have an `~/.claude.json`. The canonical token lives at `~/.config/mcp-agent-mail/config.env` (`HTTP_BEARER_TOKEN=...`). Decide while doing D5: add a third resolver in `bin/br-claim` and `bin/br-complete` that reads `config.env` as a last-resort fallback before failing. Costs ~10 lines of Python and makes the wrappers Codex-native.

7. **Is `storage.sqlite3.bak` rotation scheduled or event-driven?** It exists from 2026-05-18 11:41, ~1 hour after the smoke test. Find where this rotation is triggered (probably `crates/mcp-agent-mail-server/src/backup_rotation.rs` — but verify). The trigger determines whether D4's "suspect-aware backup" needs a fresh code path or can hook into an existing one.

Park each answer in a short section of your working log (or in the eventual PR description). They are part of the handoff *out* of your session — not just inputs to it.

---

## 7 · Constraints (hard floor)

1. **No file deletion** without explicit operator approval — this is AGENTS.md Rule 1 in this project and is mirrored by the manifest's own constraint. Forensic bundles (`doctor/forensics/storage.sqlite3/reconstruct-*`) and corrupt copies (`storage.sqlite3.corrupt-*`) MUST be preserved.
2. **No destructive git** in the source repo (`/Users/alinaaitova/Desktop/mcp_agent_mail_rust/`) or the project repo. No `git reset --hard`, `git push --force`, `git clean -fd`. Use branches.
3. **Mutating Agent Mail repair (`am doctor reconstruct --yes`, `am doctor fix --yes`, `am doctor restore`, `am migrate --rollback`) requires explicit operator approval each time.** Quote the command verbatim and wait.
4. **Do not skip git hooks** (`--no-verify`) on any commit. The pre-commit hook is precisely the discipline this whole system enforces; bypassing it during the fix would be self-defeating.
5. **Do not edit `~/.claude/skills/install-bead-wrappers/` and `~/.codex/skills/install-bead-wrappers/` independently** — always mirror. Verify with `diff -r` printing nothing.
6. **Do not overwrite `bin/br-claim` / `bin/br-complete` in this project** with stock skill assets without first preserving the project's preflight customization (the very change you're landing in Step 2). The METHODOLOGY's idempotency note already calls this out.
7. **Do not delete `storage.sqlite3.corrupt-*` or `storage.sqlite3.bak`** during any clean-up step. They are forensic evidence. `storage.sqlite3.bak` is *poisoned* per §1 — note it, do not restore from it, but do not delete it either.
8. **Operator's question-cadence preference (persistent across sessions):** *"Don't pause to ask after every checkpoint; decide and proceed."* This means: do not stop after every successful sub-step to confirm — the gates in §6 Step 1 (state-mutating repair) and §6 Step 4 (binary swap) are the only mandatory operator checkpoints. Everything in between (code reads, hypothesis testing in the source tree, wrapper edits, test additions, `cargo build`/`cargo test` cycles) should run end-to-end without re-checking in. Report a punch list at the end of each Step, not a question after each command. If you genuinely don't know which direction to take, *ask* — but don't ask just to confirm "should I proceed?" after a step you already understood.

---

## 8 · Acceptance Criteria

### Functional (after each step, re-verify)

- [ ] `health_check({})` returns non-error health with `semantic_readiness.status=pass`.
- [ ] `list_agents("/Users/alinaaitova/Desktop/Testproject1")` returns `MaroonMountain` and `JadeMink`.
- [ ] `fetch_inbox(MaroonMountain)` returns message id=1 (or an intentional migrated equivalent).
- [ ] `search_messages("agentmail-smoke-20260518T1020Z")` returns message id=1.
- [ ] A fresh `file_reservation_paths` + `release_file_reservations` cycle leaves no orphan reservation.

### State (post-reconstruct)

- [ ] SQLite `agents` count ≥ 2; `messages` count ≥ 1; `file_reservations` count matches archive JSON count for the project.
- [ ] `archive inventory is ahead of the sqlite index` no longer appears in `health_check.semantic_readiness.detail`.
- [ ] `am doctor reconstruct --dry-run` reports a no-op (or an explicitly-understood "would recover N=0" state).

### Behavioral (post-D1 binary upgrade)

- [ ] Under forced `DegradedReadOnly`, `register_agent` returns `DATABASE_DEGRADED` MCP error.
- [ ] Under forced `DegradedReadOnly`, `send_message` returns `DATABASE_DEGRADED` MCP error.
- [ ] Under forced `DegradedReadOnly`, `file_reservation_paths` returns `DATABASE_DEGRADED` MCP error.
- [ ] Under forced `DegradedReadOnly`, *read* tools (`list_agents`, `fetch_inbox`, `search_messages`) continue to return archive-backed data.
- [ ] `recovery.write_policy=="reject_all_mutations"` appears in `health_check.recovery` while degraded.

### Behavioral (post-D2)

- [ ] On server startup with drift present and `recovery_lock_active=false`, the server auto-attempts reconstruction once (or, if auto-repair was deferred, emits a structured `recovery.last_attempt_error=null` field plus a periodic stderr warning loud enough that a tail of the server log shows it within 30 s).

### Defensive (wrappers + skill)

- [ ] `bin/br-claim --help` and `bin/br-complete --help` still succeed.
- [ ] `bin/br-claim <bead-id> --dry-run` against a red server refuses *before* the dry-run banner prints, and the failure message contains the verbatim `recovery.next_action` string from `health_check` (no archive writes occur).
- [ ] `bin/br-claim <bead-id>` against a green server proceeds normally — no preflight regression.
- [ ] `diff -r ~/.codex/skills/install-bead-wrappers/ ~/.claude/skills/install-bead-wrappers/` prints nothing.
- [ ] Updated skill `SKILL.md` Step-1 pre-flight catches a red server and aborts install with a clear remediation.
- [ ] `METHODOLOGY.md` "Snapshot commit" stamp is updated to the SHA that lands the preflight wrappers in `Testproject1`.

### Tests

- [ ] New "rejects writes in DegradedReadOnly" parity tests pass for `register_agent`, `send_message`, `file_reservation_paths` in `crates/mcp-agent-mail-tools/tests/`.
- [ ] Existing FrankenSQLite snapshot-advance tests (the ones added by commit `2862eebe`) still pass.
- [ ] `crates/mcp-agent-mail-conformance` test suite passes.
- [ ] Existing `am guard status /Users/alinaaitova/Desktop/Testproject1` still reports both hooks installed.

### Regression (do not break)

- [ ] Existing `bin/br-claim` and `bin/br-complete` happy paths against a green server still produce the same `Start` / `Complete` mail subjects, the same reservation TTLs, and the same `br update --status=in_progress` / `br close` calls. The preflight is additive, not behavior-changing.
- [ ] No untracked files added outside the planned scope (`bin/`, `prompts/`, `docs/handoff-2026-05-18-*.md`, and edits inside the source repo).

---

## 9 · File Map

### In `/Users/alinaaitova/Desktop/Testproject1/` (this project)

| File | Role | Action |
|---|---|---|
| `prompts/agent-mail-sqlite-index-drift-error-manifest.md` | The forensic manifest produced by the Codex agent that found this bug. Authoritative defect reference. | Read |
| `prompts/agent-mail-sqlite-index-drift-fix-handoff.md` | This document. | Execute |
| `bin/br-claim` (412 lines) | Atomic-claim wrapper invoked by every agent on every bead. | Edit (add preflight) |
| `bin/br-complete` (420 lines) | Atomic-complete counterpart. | Edit (add preflight) |
| `AGENTS.md` (Rule 2, lines 21-33) | Constitutional rule mandating wrapper use. | Reference (do not edit) |
| `docs/handoff-2026-05-18-*.md` | Sibling handoff docs from today's session — read for context cues on the user's working style. | Reference |

### In `~/.mcp_agent_mail_git_mailbox_repo/` (storage root)

| File / Dir | Role | Action |
|---|---|---|
| `storage.sqlite3` | Live index. **Drifted.** Will be rebuilt by `am doctor reconstruct --yes`. | Mutate via official tools only |
| `storage.sqlite3.bak` | **Poisoned backup** from 2026-05-18 11:41 (already drifted). | Preserve; do not restore |
| `storage.sqlite3.corrupt-20260508_*` | Pre-corruption-event copies. Forensic. | Preserve; do not delete |
| `storage.sqlite3.activity.lock`, `.mailbox.activity.lock` | Server activity locks. | Do not touch |
| `doctor/forensics/storage.sqlite3/reconstruct-20260508_*` | Two reconstruct bundles from the original corruption events. `manifest.json` and `summary.json` inside. | Read for context (relevant to Open Question §6.5 #4) |
| `projects/users-alinaaitova-desktop-testproject1/agents/{MaroonMountain,JadeMink}/profile.json` | Archive-backed agent records currently missing from SQLite. | Preserve; will be replayed |
| `projects/users-alinaaitova-desktop-testproject1/messages/2026/05/2026-05-18T10-20-26Z__agentmail-smoke-20260518t1020z-self-test-message__1.md` | Archive-backed smoke-test message. | Preserve; will be replayed |
| `projects/users-alinaaitova-desktop-testproject1/file_reservations/{*.json,id-1.json,id-2.json}` | Archive-backed reservation records. | Preserve; will be replayed |
| `search_index/` | Tantivy search index (separate from SQLite). Open Question §6.5 #3: does `reconstruct` touch this? | Do not touch directly |

### Adjacent system locations (outside the active storage root)

| File / Dir | Role | Action |
|---|---|---|
| `~/.config/mcp-agent-mail/config.env` | `HTTP_BEARER_TOKEN=...` — canonical bearer token location, currently not consulted by `bin/br-claim` / `bin/br-complete` | Reference (consider adding as fallback in wrappers — see §6.5 #6) |
| `~/.local/share/mcp-agent-mail/git_mailbox_repo/` | Empty scaffold storage root from a prior install. **Not in use** — server uses `~/.mcp_agent_mail_git_mailbox_repo/`. | Reference; do not confuse with the active root |

### In `/Users/alinaaitova/Desktop/mcp_agent_mail_rust/` (upstream Rust source)

| File | Role | Action |
|---|---|---|
| `crates/mcp-agent-mail-server/src/lib.rs:2034` | Drift diagnostic format | Reference |
| `crates/mcp-agent-mail-server/src/lib.rs:13506-13640` | Best-effort drift warning path | Edit (D2) |
| `crates/mcp-agent-mail-server/src/startup_checks.rs:1731-1748` | Startup-time durability arms | Edit (D2, optional) |
| `crates/mcp-agent-mail-server/src/backup_rotation.rs` | Backup rotation | Edit (D4) |
| `crates/mcp-agent-mail-core/src/mailbox_durability.rs` | Durability state machine + contracts | Reference (and possibly extend with `write_policy` for D3) |
| `crates/mcp-agent-mail-db/src/mailbox_verdict.rs` | `compute_mailbox_verdict` + `VerdictOptions::fast()`; the on-demand probe used by both read fallback and (proposed) write gate | Read first (Reading order step 1) |
| `crates/mcp-agent-mail-tools/src/lib.rs:737-777` | `live_db_is_suspect` — existing read-side gate; template for the new write-side peer | Reference; add a `live_db_blocks_writes` peer here (D1) |
| `crates/mcp-agent-mail-tools/src/identity.rs:1103` | `pub async fn register_agent` — primary mutating entry point | Edit (D1) |
| `crates/mcp-agent-mail-tools/src/identity.rs:170-187` | `recovery.next_action` string builder (the `DegradedReadOnly` arm at line 172) | Cosmetic only (mention `am doctor fix --dry-run` first if desired) |
| `crates/mcp-agent-mail-tools/src/identity.rs:527, 2548, 2675` | identity-tool copies of the drift diagnostic | Keep in lockstep (D1, D3) |
| `crates/mcp-agent-mail-tools/src/messaging.rs:1427` | `pub async fn send_message` | Edit (D1) |
| `crates/mcp-agent-mail-tools/src/messaging.rs:3060` | Read-fallback comment site (write branch is the bug — adjacent) | Read for context |
| `crates/mcp-agent-mail-tools/src/reservations.rs:378` | `pub async fn file_reservation_paths` — highest blast radius (collision detector) | Edit (D1) |
| `crates/mcp-agent-mail-tools/src/resources.rs:300` | Resource read-fallback rule | Audit write paths (D1) |
| `crates/mcp-agent-mail-tools/src/lib.rs:496-526` | Tools-side drift error & mode JSON | Reference (D1, D3) |
| `crates/mcp-agent-mail-db/src/reconstruct.rs` | Archive→SQLite reconstruction; documents what is NOT recoverable from archive | Reference (decision input for §6 Step 1) |
| `crates/mcp-agent-mail-cli/src/robot.rs:780, 2715-3110, 16051` | CLI mode labels | Reference (keep wording in sync with MCP) |
| `crates/mcp-agent-mail-test-helpers/` | Standard test fixture primitives | Use when authoring new D1 parity tests |
| `crates/mcp-agent-mail-tools/tests/system_error_parity.rs:156` | `register_agent_under_sqlite_lock_maps_to_resource_busy` — closest template for the new parity test (induces transient state, not persistent DegradedReadOnly) | Extend (D1) |
| `crates/mcp-agent-mail-tools/tests/validation_error_parity.rs` | Parity for validation-side errors | Extend (D1) |
| `RECOVERY_RUNBOOK.md` (workspace root) | Operator playbook — covers the git 2.51.0 index-race that originally motivated `recovery.rs` | Read in full before D2 work |

### In `~/.claude/skills/install-bead-wrappers/` and `~/.codex/skills/install-bead-wrappers/`

| File | Role | Action |
|---|---|---|
| `SKILL.md` (both copies) | Skill instructions | Edit (add server-health preflight per §4 D5) |
| `METHODOLOGY.md` (both copies) | Rationale + snapshot stamp | Edit (bump stamp to new SHA) |
| `assets/br-claim`, `assets/br-complete` (both copies) | Byte-for-byte canonical wrapper sources | Replace (with the preflight-enhanced wrappers) |
| `assets/agents-md-rule-2.md` (both copies) | Constitutional snippet appended to AGENTS.md | Do not edit unless Rule 2's text actually needs to change |

---

## 10 · Pitfalls to Avoid (already discovered)

- **Do not assume `am doctor reconstruct --yes` is a one-shot cure.** It rebuilds projects/agents/messages but cannot rebuild `agent_links`, contact handshakes, or product-bus state — those are not archived. For Testproject1 today the loss is empty (the smoke-test agents had no peer handshakes to rebuild), but a different project on the same machine may have non-empty contact state. Tell the operator before running.
- **Do not restore `storage.sqlite3.bak`.** §1 proved it is also drifted.
- **Do not run `am doctor fix --yes` *before* the immediate `am doctor reconstruct --yes`** without reading what other remediations `fix` will trigger. From `am doctor fix --help`: it can also touch shell-rc PATH order, MCP config rewrites, and pre-commit guard installation — none of those are wanted right now, and `--yes` skips the interactive prompt. Either run the single-purpose `reconstruct` first, or pass `am doctor fix --dry-run` and review the plan.
- **Do not edit `bin/br-claim` or `bin/br-complete` while the wrappers' git pre-commit hook is loud about reservations.** Use a `chore:` commit prefix per AGENTS.md Rule 2 (the hook exempts `chore:` for the Start-mail subcheck) and make sure `AGENT_NAME` is set in your shell.
- **Do not push the new `am` binary into `~/.local/bin/am` while the HTTP server is running.** SIGINT the server, swap the binary, restart. The OS will not block the rename, but a running server will keep the old in-memory image, leading to bizarre "code says X, behavior is Y" debugging.
- **Do not silently `cp -r` between the Claude and Codex skill copies without a `diff -r` confirmation first.** If they have already drifted, you would clobber a fix in one with the other.
- **Do not delete the `storage.sqlite3.corrupt-*` files** even after reconstruct succeeds. They are needed if the operator ever wants to reopen the 2026-05-08 corruption forensic.

---

## 11 · Operator-Facing Summary

A single sentence you can paste to the operator before approval gates:

> The local Agent Mail server has been in `degraded_read_only` since 2026-05-08; mutating MCP calls are landing in the archive but not in SQLite (drift is real and growing) — I need approval to run `am doctor reconstruct --yes` (single-purpose, surgical, one project in scope) to repair the current state, then I'll trace the write-path bug in the Rust source and land a defensive preflight in `bin/br-claim`/`bin/br-complete` and the mirrored `install-bead-wrappers` skill copies.

A two-sentence framing for *you*, the agent:

> Read §3.0 in order before patching. The defect class hierarchy in §4 (D1–D5) is hypothesis-driven for D1 specifically — confirm with a live `register_agent` probe against the still-red server before assuming the write path lacks a gate; the bug could equally be a verdict cache, a per-tool short-circuit, or a missing gate, and the source pointers and acceptance criteria differ in each case.

Hand the operator §6 Step 1 first; the rest can land behind that one approval gate plus the §6 Step 4 binary-swap gate.
