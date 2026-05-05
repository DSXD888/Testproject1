# MCP Agent Mail Read/Search Database Errors - Investigation + Fix Handoff

ULTRATHINK and be meticulous.

## MISSION

Fix the MCP Agent Mail failure mode where write-side operations can succeed while read/search/resource paths fail because the Git archive is ahead of the SQLite index or the SQLite index is corrupt/stale. The fix belongs in the external MCP Agent Mail Rust codebase, not in this Procurement Intelligence Platform repository; this repository contains the captured error manifest, post-incident evidence, and a tracked patch with useful Agent Mail file-path clues. The receiving session must make read/search behavior deterministic: either rebuild/reconcile from the authoritative archive before serving reads, or return a structured degraded-state diagnostic that explains the archive/SQLite mismatch instead of a generic transient database error.

Out of scope: destructive cleanup of live Agent Mail storage, deleting smoke-test mailbox records, or changing this project's Rust/TypeScript procurement platform code.

---

## SYMPTOMS

- **Observed behavior**: `register_agent`, `send_message`, file reservations, release, and acknowledgement returned success, but `search_messages`, `fetch_inbox`, and `resource://inbox` / `resource://thread` failed with database errors in the same session.
- **Expected behavior**: after a successful `send_message`, the message is visible through `search_messages`, `fetch_inbox`, and resource reads, or the server returns an actionable degraded-state error that names the archive/SQLite mismatch and recovery action.
- **Reproducibility**: environment-specific. Captured 2/2 during a degraded state on `2026-05-05T21:04Z`; later stopped reproducing after archive/SQLite alignment.
- **Runtime**: macOS 15.5 arm64, `am 0.2.51`, MCP Agent Mail HTTP server at `127.0.0.1:8765`, storage root `~/.mcp_agent_mail_git_mailbox_repo`.
- **Captured storage path**: `/Users/alinaaitova/.mcp_agent_mail_git_mailbox_repo/storage.sqlite3`.
- **Project key in evidence**: `/Users/alinaaitova/Desktop/Testproject1`.

Key degraded health payload:

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

Generic tool failure:

```text
A database error occurred. This may be a transient issue - try again.
```

Resource-specific macOS temp-path failure:

```text
SQLite error: reconstruct: refusing reconstruct sqlite target /var/folders/z3/784rpxcj23x3_0d8d88c_27h0000gn/T/agent-mail-resource-snapshot-69379_1778015106574028000_0/mailbox.sqlite3 because it traverses symlinked path /var
```

Post-recovery success payloads prove the archive had the data and reconstruction/alignment fixed the reads:

```json
{
  "result": [
    {
      "id": 1,
      "subject": "[agent-mail-smoke] Self-test message",
      "thread_id": "agent-mail-smoke-2026-05-05",
      "from": "CrimsonChapel",
      "to": ["CrimsonChapel"]
    }
  ]
}
```

---

## LOCAL EVIDENCE TO READ FIRST

Read these files in `/Users/alinaaitova/Desktop/Testproject1` before touching Agent Mail source:

1. `prompts/mcp-agent-mail-read-search-error-manifest.md`
   - Primary forensic manifest supplied for this task.
   - Focus on E2 broken read/search evidence, E3 recovery evidence, H1/H2/H3 hypotheses, and acceptance criteria.

2. `docs/handoff-2026-05-05-mcp-agent-mail-db-recovery.md`
   - Post-incident handoff with later daemon-log evidence.
   - Important refinement: the outage likely came from pre-existing SQLite page corruption detected at startup, not from the smoke test itself.
   - It records that automatic `am doctor reconstruct` rebuilt `1 projects, 3 agents, 3 messages` with zero parse errors.

3. `agent-mail-search-fixes.patch`
   - Tracked patch against the external Agent Mail codebase.
   - Use it only as source-surface evidence. It touches `crates/mcp-agent-mail-db/src/queries.rs`, `search_planner.rs`, `search_service.rs`, `search_v3.rs`, and `crates/mcp-agent-mail-tools/src/search.rs`.
   - It is not sufficient for this bug by itself; it mainly covers path canonicalization, search pagination, recency ordering, and sender-filter direction.

Understanding checkpoints:

1. Why did writes return concrete IDs while reads/search failed?
2. What did health know that `search_messages` and `fetch_inbox` failed to report?
3. Why is the `/var/folders` error likely resource-specific and macOS-specific?
4. Which evidence proves that the Git archive was intact and SQLite was the rebuildable broken index?

---

## EXTERNAL AGENT MAIL CODEBASE ORIENTATION

Locate the Agent Mail Rust source checkout. If the checkout is not already open, find or clone the upstream source that builds the installed `am 0.2.51` binary. The path names below are inferred from the tracked patch and local Agent Mail bead metadata; validate them with `rg --files` in the Agent Mail repo.

Likely crate boundaries:

```text
crates/mcp-agent-mail-server/src/lib.rs        # HTTP transport, MCP tool/resource dispatch
crates/mcp-agent-mail-tools/src/*.rs           # tool handlers and resource renderers
crates/mcp-agent-mail-db/src/*.rs              # SQLite pool, schema, queries, search, errors
crates/mcp-agent-mail-storage/src/lib.rs       # Git archive, file locks, commit coalescer
crates/mcp-agent-mail-core/src/*.rs            # config, metrics, diagnostics, lock ordering
crates/mcp-agent-mail-cli/src/*.rs             # am doctor, service, CLI repair/reconstruct flows
```

Start with these discovery commands in the Agent Mail source repo:

```bash
rg -n "pub async fn search_messages|search_messages\\(" crates/
rg -n "pub async fn fetch_inbox|fetch_inbox\\(" crates/
rg -n "resource://inbox|resource://thread|inbox resource|thread resource" crates/
rg -n "semantic_readiness|archive inventory is ahead|archive_db_parity|degraded_read_only" crates/
rg -n "doctor repair|doctor reconstruct|automatic-recovery|reconstruct" crates/
rg -n "refusing reconstruct sqlite target|traverses symlinked path|symlinked path" crates/
rg -n "integrity_check|quick_check|database disk image is malformed|page .* is never used" crates/
rg -n "ionice|journal_mode|WAL|wal_mode" crates/
```

Likely files to inspect:

- `crates/mcp-agent-mail-db/src/pool.rs` - pool acquisition, connection setup, startup integrity behavior.
- `crates/mcp-agent-mail-db/src/schema.rs` - SQLite PRAGMAs, journal mode, migrations.
- `crates/mcp-agent-mail-db/src/queries.rs` - project/agent/message visibility and any read/query helpers.
- `crates/mcp-agent-mail-db/src/error.rs` - corruption classification and DB error typing.
- `crates/mcp-agent-mail-db/src/search_service.rs` and `search_v3.rs` - search execution, Tantivy bridge, recency/cursor behavior.
- `crates/mcp-agent-mail-tools/src/search.rs` - `search_messages` MCP tool error mapping and result formatting.
- `crates/mcp-agent-mail-tools/src/resources.rs` or equivalent - `resource://inbox` and `resource://thread` rendering.
- `crates/mcp-agent-mail-tools/src/lib.rs` or equivalent - shared `db_error_to_mcp_error` mapping.
- `crates/mcp-agent-mail-tools/src/health.rs` or equivalent - `health_check` and `semantic_readiness` payloads.
- `crates/mcp-agent-mail-cli/src/doctor*.rs` or equivalent - `am doctor check`, `repair`, `reconstruct`, and next-action text.
- `crates/mcp-agent-mail-storage/src/lib.rs` - archive inventory scan, reconstruction source, maintenance shell-out using `ionice`.

Understanding checkpoints:

1. Which layer treats the Git archive as canonical and SQLite as a rebuildable index?
2. Where does `health_check` compute archive-vs-DB inventory drift?
3. Which code path triggers automatic reconstruction, and what admission lock prevents duplicate reconstructs?
4. Do `search_messages`, `fetch_inbox`, and resource reads share the same degraded-readiness guard?
5. Where is a resource snapshot SQLite target created and validated?

---

## REFINED INCIDENT MODEL

The manifest initially frames the bug as an integration mismatch: write-side success but read-side SQLite/search failure. The later post-incident handoff adds stronger evidence:

1. At `2026-05-05T20:54:13.519Z`, the daemon startup integrity guard detected SQLite corruption:

```text
database disk image is malformed: page 72 is never used
```

2. The guard intentionally did not mutate the live DB in the background. It logged that query-path recovery would trigger on the next corrupt-verdict read.

3. During the smoke workload, writes landed in the archive and appeared to succeed, but fresh durability probes missed committed visibility for agents and messages.

4. Readiness later detected archive drift:

```text
archive inventory is ahead of the sqlite index (archive=1/3/3, db=0/0/0)
```

5. The daemon auto-ran archive-backed reconstruction at `2026-05-05T21:07:08Z`, captured a forensic bundle, removed a truncated 32-byte WAL sidecar, and rebuilt SQLite from the archive in about 50 ms.

6. After reconstruction, `health_check`, `search_messages`, `fetch_inbox`, and resource reads returned correct results.

Implication: the implementation problem is not simply "search did not index a message." It is a recovery/readiness/error-surface problem around a rebuildable SQLite index that can fall behind the canonical archive.

---

## HYPOTHESIS SPACE

Ranked by implementation value:

1. **Read/search tools do not consistently gate on archive-vs-SQLite readiness.**
   - Evidence: health already reported `degraded_read_only` and exact inventory drift, while `search_messages` and `fetch_inbox` returned only a generic database error.
   - Fix direction: all read/search/resource paths should share a readiness/reconcile gate before querying SQLite/search indexes.

2. **Automatic recovery is deferred too late or only on some read paths.**
   - Evidence: startup detected corruption around 20:54Z but reconstruction did not happen until 21:07Z after read/search failures.
   - Fix direction: when startup integrity detects corruption or health sees archive ahead of DB, trigger an admission-controlled archive reconstruct before accepting normal read traffic, or return a deterministic degraded error until the reconstruct completes.

3. **Resource snapshot reconstruction rejects valid macOS temp paths.**
   - Evidence: resource reads tried to reconstruct below `/var/folders/...` and rejected `/var` because it is symlinked to `/private/var`, which is normal on macOS.
   - Fix direction: canonicalize the temp root and target before validation, then enforce containment against the canonical root. Do not blindly allow arbitrary symlink traversal.

4. **Error mapping strips actionable diagnostics from read/search tools.**
   - Evidence: resource errors carried `error_detail`; tool errors did not.
   - Fix direction: tool MCP errors should include a stable error type plus bounded diagnostic data such as `semantic_readiness.status`, archive/db counts, recovery mode, and recommended action.

5. **Human-facing recovery guidance is inconsistent.**
   - Evidence: MCP health suggested `am doctor repair`; the CLI reportedly said no database repair was recommended while `archive_db_parity` failed; the daemon actually ran `am doctor reconstruct`.
   - Fix direction: align `health_check`, `am doctor check`, and CLI help text around `reconstruct` or `fix` for archive-full / DB-empty drift.

6. **WAL/default durability and macOS archive maintenance are adjacent hardening issues.**
   - Evidence: `journal_mode=delete` was flagged, a truncated WAL sidecar was swept, and archive maintenance attempted `nice ionice` on macOS where `ionice` does not exist.
   - Fix direction: consider default WAL for new DBs/migrations and Darwin-specific archive maintenance fallback, but do not let these distract from read/search recovery correctness.

---

## IMPLEMENTATION STRATEGY

### 1. Establish the authoritative consistency contract

Document or codify this invariant in tests:

```text
The Git archive is canonical for persisted mailbox entities.
SQLite and Tantivy are query indexes.
If SQLite/Tantivy are missing archive-visible entities, read/search paths must not silently act healthy.
They must either repair/rebuild from archive or return a structured degraded-state error.
```

Do not weaken write-side behavior that already worked: successful write tools should still return IDs only after their success contract is satisfied.

### 2. Add a shared read-readiness gate

Find the function that computes `semantic_readiness` / `archive_db_parity`. Extract or reuse it from a shared module so every read/search/resource entry point can call the same logic.

Required behavior:

- If archive and DB are aligned, proceed normally.
- If the DB is corrupt or archive is ahead of DB, attempt exactly one admission-controlled reconstruct/reconcile for the storage root.
- If reconstruct succeeds, retry the read/search operation using the rebuilt index.
- If reconstruct is already in progress, either wait within a bounded budget or return `DATABASE_DEGRADED` / equivalent with `recovery_lock_active=true`.
- If reconstruct fails or is not safe, return structured diagnostics with archive/db counts and next action.

Do not let one path return success from stale SQLite while another path reports degraded state for the same storage root.

### 3. Fix `search_messages` and `fetch_inbox` diagnostics

Patch the MCP tool error mapping so read/search callers receive enough data to act:

```json
{
  "type": "DATABASE_DEGRADED",
  "message": "Agent Mail SQLite index is behind the Git archive; reads are temporarily degraded.",
  "recoverable": true,
  "data": {
    "semantic_readiness": {
      "status": "fail",
      "archive_projects": 1,
      "archive_agents": 3,
      "archive_messages": 3,
      "db_projects": 0,
      "db_agents": 0,
      "db_messages": 0,
      "missing_archive_projects": ["users-alinaaitova-desktop-testproject1"]
    },
    "recovery": {
      "mode": "degraded_read_only",
      "next_action": "Run `am doctor reconstruct` or wait for automatic recovery to finish."
    }
  }
}
```

Field names can follow existing Agent Mail conventions, but the resulting payload must make the archive/SQLite mismatch discoverable without an extra health call.

### 4. Fix macOS resource snapshot path validation

Search for the guard that produced:

```text
refusing reconstruct sqlite target /var/folders/... because it traverses symlinked path /var
```

The likely bug is validating lexical path components before canonicalizing a normal macOS temp root. Implement a containment-safe algorithm:

1. Create the snapshot temp directory using the existing tempfile/tempdir mechanism.
2. Canonicalize the snapshot root after creation.
3. Build the SQLite target below that root.
4. Canonicalize the parent directory of the target, not the nonexistent target file.
5. Verify the canonical parent starts with the canonical snapshot root.
6. Reject if any caller-controlled component can escape the root.
7. Permit `/var/folders/...` when canonicalized to `/private/var/folders/...` and still contained within the temp root.

Do not replace this with a blanket "allow `/var`" exception. The security invariant is containment in the snapshot root, not trust in a path prefix.

### 5. Align `health_check` and `am doctor` recovery guidance

For the specific condition:

```text
archive projects/messages > db projects/messages
db messages == 0
archive messages > 0
```

The recommended action should not be generic `repair` or "no repair recommended." It should point at the actual supported recovery path:

```text
automatic archive-backed reconstruct is pending/in progress; to force it manually, stop the daemon and run `am doctor reconstruct --yes`
```

Use the exact CLI wording already accepted by the project, but make the MCP and CLI surfaces agree.

### 6. Treat WAL and `ionice` as optional adjacent fixes

If the same Agent Mail issue includes hardening scope, implement:

- WAL as the default journal mode for new or migrated SQLite stores, preserving safe upgrade behavior.
- Darwin archive maintenance fallback: do not shell out to `ionice` on macOS. Use plain `nice`, skip I/O priority, or use a Darwin-specific equivalent if the codebase already has one.

If these are too broad, file follow-up issues and keep this fix focused on read/search/resource recovery.

---

## REGRESSION TESTS TO ADD

### Test 1: archive-ahead-of-SQLite read/search behavior

Goal: prove `search_messages` and `fetch_inbox` no longer return generic database errors when archive has data and SQLite is empty/stale.

Suggested setup:

1. Build an isolated storage root with a valid project, agent, and message in the Git archive.
2. Create an empty or stale SQLite index for that same storage root.
3. Call the same code path used by `search_messages`.
4. Call the same code path used by `fetch_inbox`.

Accept either of these outcomes, depending on the design decision:

- Preferred: the read path auto-reconstructs SQLite from archive and returns the message.
- Acceptable: the read path returns a structured degraded-state error with archive/db counts and a correct reconstruct next action.

Fail the test if the error is only:

```text
A database error occurred. This may be a transient issue - try again.
```

### Test 2: resource snapshot accepts canonicalized macOS temp symlink

Goal: prove resource reads do not reject normal `/var/folders` style temp roots on macOS.

Portable unit-test shape:

1. Create a temp directory.
2. Create a symlink path that points to a real temp root.
3. Set or inject the snapshot temp root through the symlink path.
4. Run the snapshot target validation/reconstruct helper.
5. Assert validation succeeds when the canonical target remains inside the canonical temp root.

macOS integration variant:

```bash
TMPDIR=/var/folders/... cargo test -p <agent-mail-crate> resource_snapshot
```

Fail the test if the error contains:

```text
traverses symlinked path /var
```

### Test 3: resource snapshot still blocks real traversal

Goal: prevent the macOS fix from weakening path traversal protection.

Cases to include:

- `../outside/mailbox.sqlite3` style target escapes.
- A symlink inside the temp root points outside the temp root.
- A caller-controlled absolute path is supplied where only temp-root-relative paths are allowed.

Each must be rejected with a security/path-validation error.

### Test 4: degraded error mapping for tools

Goal: `search_messages` and `fetch_inbox` preserve actionable diagnostics.

Inject or mock a readiness failure equivalent to:

```text
archive projects=1, agents=3, messages=3
db projects=0, agents=0, messages=0
```

Assert the MCP error payload includes:

- stable error type such as `DATABASE_DEGRADED`, `DATABASE_CORRUPTION`, or the codebase's existing equivalent,
- `recoverable: true`,
- semantic readiness detail,
- recovery mode,
- correct next action.

### Test 5: full smoke cycle remains green

Run an isolated end-to-end smoke test:

1. `ensure_project`
2. `register_agent`
3. `send_message` to self or second agent
4. `search_messages` for a subject/body token
5. `fetch_inbox` for the recipient
6. `resource://inbox/{agent}`
7. `resource://thread/{thread_id}`
8. `file_reservation_paths`
9. reservation resource read
10. `release_file_reservations`
11. `acknowledge_message` twice to prove idempotency
12. `health_check`

Expected result: all reads agree on the same message and health is green after the cycle.

---

## MANUAL REPRODUCTION / VALIDATION COMMANDS

Never corrupt or rewrite live `~/.mcp_agent_mail_git_mailbox_repo` to reproduce this. Use an isolated copy or test fixture storage root.

Evidence commands from the incident:

```json
{"tool":"mcp__mcp_agent_mail__.health_check","arguments":{}}
```

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

Resource reads to validate:

```text
resource://inbox/CrimsonChapel?project=/Users/alinaaitova/Desktop/Testproject1&include_bodies=true&limit=10
resource://thread/agent-mail-smoke-2026-05-05?project=/Users/alinaaitova/Desktop/Testproject1&include_bodies=true&limit=10
```

Post-fix expected outcomes:

- If storage is healthy, these return the known message or equivalent smoke message.
- If archive is ahead of SQLite, automatic reconstruct runs and the read succeeds, or the tool returns a structured degraded diagnostic.
- Resource reads on macOS never fail solely because `/var` is a symlink to `/private/var`.

---

## FIX CONSTRAINTS

1. Do not require users or agents to run destructive cleanup against Agent Mail storage.
2. Do not lose, rewrite, or delete archive messages, agents, projects, reservations, or forensic bundles.
3. Preserve working write-side behavior: registration, send, reservations, release, and acknowledgement.
4. Treat the Git archive as the canonical recovery source and SQLite/Tantivy as rebuildable indexes.
5. Resource snapshot path validation must remain traversal-safe after macOS canonicalization.
6. Read/search paths must not report success from known-stale SQLite state.
7. Error payloads should be actionable, but should not leak more filesystem detail than existing health/resource diagnostics already expose.
8. Recovery must be admission-controlled so concurrent reads do not trigger competing reconstruct jobs.
9. Tests must use isolated storage roots and must not mutate the operator's live mailbox store.
10. Do not apply `agent-mail-search-fixes.patch` blindly as the complete fix; audit whether its path canonicalization helps project identity, then separately fix recovery/readiness/snapshot behavior.

---

## DELIVERABLES

1. Root-cause writeup in the Agent Mail issue/PR explaining the failure as archive-visible data plus broken/stale SQLite read index, with the macOS resource snapshot issue called out separately.
2. Shared read-readiness/reconcile guard used by `search_messages`, `fetch_inbox`, and resource reads.
3. Automatic or bounded archive-backed reconstruction behavior for DB-corrupt / archive-ahead states, or a structured degraded-state error if auto-reconstruct is intentionally not attempted.
4. macOS-safe resource snapshot target validation that canonicalizes temp roots while preserving traversal protection.
5. Improved MCP tool error mapping for read/search failures, including semantic readiness and recovery detail.
6. Aligned `health_check` and `am doctor` next-action guidance for archive-full / DB-empty parity failures.
7. Regression tests covering archive-ahead SQLite, macOS symlinked temp roots, traversal rejection, degraded error mapping, and the full smoke cycle.
8. Validation evidence from targeted tests plus the Agent Mail workspace quality gates.
9. Optional follow-up issues or fixes for WAL defaulting and Darwin `ionice` archive maintenance if not included in the patch.

---

## VERIFICATION

Run these in the Agent Mail source repo, adjusting package names only if the workspace uses different crate names:

```bash
cargo fmt --check
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test -p mcp-agent-mail-db --all-targets
cargo test -p mcp-agent-mail-tools --all-targets
cargo test --workspace --all-targets
```

Then run an isolated live smoke using a non-production storage root:

```bash
export AM_STORAGE_ROOT="$(mktemp -d)/mailbox"
am serve-http --host 127.0.0.1 --port 0
```

If the CLI uses different env vars or flags for storage-root override, use the documented equivalent. The key requirement is that the smoke test must not touch `~/.mcp_agent_mail_git_mailbox_repo`.

Manual smoke assertions:

- `health_check` starts green on a fresh storage root.
- `send_message` returns a concrete ID.
- `search_messages` finds the subject/body token.
- `fetch_inbox` returns the same ID.
- `resource://inbox` and `resource://thread` return the same ID.
- A forced archive-ahead / DB-empty fixture either reconstructs and returns data or returns a structured degraded diagnostic.
- macOS `/var/folders` temp roots are accepted after canonicalization.
- Real path traversal through snapshot target validation remains rejected.

---

## ACCEPTANCE CRITERIA

- [ ] A successful message send is visible via `search_messages`.
- [ ] A successful message send is visible via `fetch_inbox`.
- [ ] `resource://inbox/{agent}` and `resource://thread/{thread_id}` agree with tool reads.
- [ ] File reservation resources and tools agree after reservation and release.
- [ ] Archive-ahead SQLite state no longer produces a generic transient database error.
- [ ] Archive-ahead SQLite state either auto-reconstructs or returns structured degraded diagnostics with archive/db inventory counts.
- [ ] macOS resource reads do not fail with `refusing reconstruct sqlite target /var/folders/... because it traverses symlinked path /var`.
- [ ] Snapshot validation still rejects real traversal outside the temp root.
- [ ] `health_check` and `am doctor` recommend the same correct recovery action for archive-full / DB-empty parity failures.
- [ ] Existing write-side operations continue to succeed and preserve idempotency.
- [ ] All new tests fail on the pre-fix behavior and pass after the fix.

---

## REMINDERS

- ULTRATHINK and take the time to trace the recovery path before editing.
- The live outage already self-recovered; do not use live storage as a repro harness.
- The smoke test triggered the recovery window; it probably did not cause the original SQLite corruption.
- Keep the fix narrow and correctness-first. The central invariant is simple: archive-visible mailbox data must not become unreadable without a precise degraded-state explanation.
- If a read path cannot safely serve data, it must say why in structured form.
- If path canonicalization is changed, add a negative traversal test in the same patch.
- If touching commits in this project later, use the project conventions from `AGENTS.md`; the actual Agent Mail source repo may have its own coordination and commit rules.
