# Agent Mail SQLite Drift — D2 + D4 Follow-Up Handoff

> Pickup point for the next agent. D1 (write rejection) and D3 (`recovery.write_policy` field) landed in this session; D2 (auto-reconstruct on detection) and D4 (suspect-aware backup rotation) are still open. This document is self-contained: read it cold and you can act.

## Context (what already shipped in the parent handoff)

- **Defect manifest**: `prompts/agent-mail-sqlite-index-drift-error-manifest.md`
- **Parent fix handoff**: `prompts/agent-mail-sqlite-index-drift-fix-handoff.md`
- **D1** (commits `bfd286d7` and bugfix `6d94fed3` on `fix/immediate-read-after-write`): `tool_util::reject_write_if_degraded()` is called from the top of all 10 mutating MCP tool bodies in `crates/mcp-agent-mail-tools/src/{identity,messaging,reservations}.rs`. When the archive-vs-SQLite drift probe trips (using non-fast `VerdictOptions` + an explicit `read_archive_is_ahead` second probe), the gate returns `DATABASE_DEGRADED`. **Verified live in production**: under degraded state, `register_agent` / `send_message` / `file_reservation_paths` all refuse with verbatim error message; reads continue via archive snapshot fallback; no drift growth from refused calls.
- **D3** (commit `bfd286d7`): added `recovery.write_policy: "full" | "reject_all_mutations"` to `health_check.recovery` (via `RecoveryStatusResponse::write_policy`) and to the rejection error JSON.
- **D5** (commit `e64d8de` in `Testproject1`): `bin/br-claim` and `bin/br-complete` got a `preflight_health()` that aborts before any mutation when the server is red, plus a `~/.config/mcp-agent-mail/config.env` bearer-token fallback for Codex installs. Skill copies in `~/.claude/skills/install-bead-wrappers/` and `~/.codex/skills/install-bead-wrappers/` are byte-identical and updated (`METHODOLOGY.md` snapshot stamped to `e64d8de`); `SKILL.md` Step 1 Pre-flight now also greps `am doctor check` for drift signals.
- **State repair**: operator approved `am doctor reconstruct --yes`; SQLite was rebuilt from archive multiple times across the session as needed.
- **Binary swap**: the running `am 0.2.54` binary at `~/.local/bin/am` has been replaced with the D1+D3 build (mtime 2026-05-18 14:34). Old binary preserved at `~/.local/bin/am.prefix-20260518`. `~/Library/LaunchAgents/com.agent-mail.plist` was unloaded/reloaded around the swap.

### ⚠️ Class B bug NOT fixed: FrankenSQLite snapshot-advance phantom writes (GH#65 `fcw_base_drift`)

**What it is, in one paragraph.** Agent Mail's `am` binary is built on `sqlmodel-frankensqlite` (workspace dep `sqlmodel-frankensqlite = "0.2.1"`, source at `~/Desktop/frankensqlite/`), a custom MVCC-augmented SQLite fork. Under sustained write load with `BEGIN CONCURRENT` transactions (`FSQLITE_CONCURRENT_MODE=true`, see `crates/mcp-agent-mail-core/src/config.rs:126`), the writer's connection sees its own COMMIT immediately, *but propagation to other connections' MVCC snapshots is not strictly synchronous and visibility to a freshly-opened connection is not the same as durability to the on-disk page file*. The result: `send_message` returns success with `id=N`, the canonical Markdown archive lands on disk (because archive writes are a separate, durable path through the Write-Behind Queue), but the SQLite row may never become visible to subsequent reads and may not be durably persisted at all. After such a "phantom" write, the archive-vs-DB inventory probe trips and D1 refuses every subsequent mutating call — so the *first* lost write is silent, every *subsequent* call is loud.

**Why this is distinct from D1.** D1 is a *pre-write* gate that refuses calls when drift is already detected. The phantom write *creates* the drift; D1 cannot run before it because the system is still nominally green at that point. The two failures compose: D1 successfully bounds the blast radius (one lost write per drift incident instead of unbounded silent compounding), but does not eliminate the single lost write. Empirically verified during the D1 fix-fix session (commit `6d94fed3` log): a `send_message` succeeded with `id=4` against a green system, drift appeared seconds later (archive=4, db=3), and every subsequent mutating call refused with `DATABASE_DEGRADED`. The drift incident itself was silent.

**Upstream tracker.** Issue GH#65 in the FrankenSQLite project ("fcw_base_drift" — `fcw` is the FrankenConnection window). The bug pattern is referenced verbatim in our source at six sites: `crates/mcp-agent-mail-tools/src/identity.rs:494`, `crates/mcp-agent-mail-core/src/config.rs:126`, `crates/mcp-agent-mail-db/src/mailbox_verdict.rs:1189`, `crates/mcp-agent-mail-db/src/queries.rs:{1316,1420,3553}`. Upstream FrankenSQLite source for the MVCC layer: `~/Desktop/frankensqlite/crates/fsqlite-mvcc/src/{lib.rs,observability.rs}` plus the VDBE at `crates/fsqlite-vdbe/src/engine.rs`.

**What's already been done about it (history; read commits before re-doing):**

| Commit | Date | What it does | Why it doesn't close Class B |
|---|---|---|---|
| `fe52967d` | 2026-05-06 | Bounded fresh-handle retry in `verify_message_recipients_visible_after_commit` and the agent-side sister (`queries.rs:1769`, `:1506`). 25–200 ms backoff, ~2 s budget over 12 attempts. | Retries fix the *transient* visibility window (most cases). Phantom writes survive when (a) the snapshot the probe sees is the writer's still-uncommitted-to-disk view, or (b) the budget exhausts and cleanup runs but only on SQLite, leaving the canonical archive `.md` orphaned (see "smoking gun" below). |
| `2862eebe` | 2026-05-06 | Verdict tolerance: don't fire `archive_latest_id_ahead` standalone, gate it on message-count drift. | Reduces false positives in the drift detector. Does NOT prevent real lost writes — those manifest as message-count drift, so the detector still fires (correctly) once a phantom write happens. |
| `db14eb65` | 2026-05-06 | Use canonical SQLite for inventory probes (avoid FrankenSQLite snapshot drift in the probe path itself). | Same scope — fixes the probe's view, not the underlying write durability. |
| `fc9b74dc`, `59f52e05` | 2026-05-06 | Plumb `DATABASE_DEGRADED` MCP error + canonicalize macOS snapshot dir. | Read-side error surfacing. |
| `bfd286d7`, `6d94fed3` | 2026-05-18 | D1 write gate: refuse mutations when drift detected. | Catches *subsequent* writes, not the one that creates the drift. |

**The smoking gun (most likely actual root cause, per the existing read-after-write handoff's Slice D analysis).** Inside `queries::create_message_with_recipients`, the flow is `BEGIN CONCURRENT → INSERT message → INSERT recipients → COMMIT → verify_message_recipients_visible_after_commit → if probe Err: cleanup_message_after_post_commit_probe_failure → cleanup_committed_message_after_consistency_failure`. The existing 91 KB handoff at `prompts/mcp-agent-mail-immediate-read-after-write-fix-handoff.md` (Mode 1, lines 393–476; Slice D, lines 672–779) flagged an unresolved compatibility question for this cleanup path:

> When the probe correctly returns Err, `cleanup_committed_message_after_consistency_failure` (called from `cleanup_message_after_post_commit_probe_failure` at queries.rs:3210) deletes the SQLite rows. **Verify this cleanup also handles the canonical archive `.md` file** (likely written upstream in messaging.rs before queries::create_message_with_recipients is invoked). If the cleanup only deletes from SQLite, the fix would create exactly the archive-ahead drift the manifest documents — *every* error case becomes a persistent inconsistency.

The empirical pattern we see in production *exactly matches* this hypothesis: archive `.md` present, SQLite row absent, `send_message` returned success because the probe came back Ok within the retry budget (the snapshot transiently saw the row) but the row never durably persisted and the next reader can't see it. The archive write happens at `messaging.rs:2167` via `try_write_message_archive` *before* `create_message_with_recipients` runs (see write-path anchors at `messaging.rs:29-67`). If the post-commit probe fails on a later send and only SQLite cleanup fires, the prior message's archive `.md` is also at risk if any retroactive cleanup logic crosses it.

**Reproduction recipe (deterministic, ~5 minutes).** Use the existing test fixtures, not the live store.

```bash
# 1. Pre-flight: ensure the server is green and on the post-fix binary.
mcp__mcp-agent-mail__health_check    # expect status=ok, health_level=green

# 2. Sustained write load. The visibility window opens under burst writes.
#    Use the storage stress fixture which already exercises this path.
cd /Users/alinaaitova/Desktop/mcp_agent_mail_rust
cargo test -p mcp-agent-mail-storage --test stress_pipeline \
    stress_commit_coalescer_batching_100_writes -- --nocapture

# 3. Or live, against a tempdir storage root (NOT the production store):
export TMP_AM_ROOT="$(mktemp -d)/agent-mail-test"
am amctl env STORAGE_ROOT="$TMP_AM_ROOT" -- am serve-http --host 127.0.0.1 --port 8766 &
# Then loop: register N agents in rapid succession via MCP at :8766,
# then sqlite3 -- compare archive file count vs SQLite agents/messages count.
```

The bug is statistically rare in a single send (the existing retries cover the common case) but reliably triggers under burst writes or when the on-disk page file lags the writer's view. The 2026-05-18 fix session triggered it during D1 verification — see the `6d94fed3` commit message for the exact incident log.

**Three fix shapes (pick a combination, ranked by recommended order):**

1. **Archive-as-authoritative continuous reconciler (highest leverage, recommended primary).** Add a background task that watches `projects/*/{agents,messages,file_reservations}/` for new files and ensures each is indexed in SQLite within N seconds of appearing on disk. Code touchpoints:
   - New file: `crates/mcp-agent-mail-server/src/archive_reconciler.rs` modeled on `cleanup.rs` and `ack_ttl.rs` (both run periodic WBQ-coordinated maintenance).
   - Wire into `crates/mcp-agent-mail-server/src/lib.rs`'s server bootstrap alongside the integrity_guard and ack_ttl tasks.
   - Reuse `crates/mcp-agent-mail-db/src/reconstruct.rs::run_for_project` (or a new row-level analogue) for the actual catch-up SQL. The full reconstruct already knows how to read archive files and write SQLite rows — wrap a per-file or per-project incremental version.
   - Single-flight on `recovery.recovery_lock_active` (the same lock D2's auto-reconstruct would use) so reconciliation never races a full reconstruct.
   - Detection trigger: filesystem watch via `notify` crate (already in the workspace if used by `mcp-agent-mail-storage`, check `Cargo.lock`), OR poll the archive inventory and SQLite counts every 5 seconds, OR hook into the WBQ drain notification.
   - **Why this is the right primary fix**: it makes the actual durability contract (archive on disk) the ground truth and treats SQLite as a derived index, which matches the architecture comments at `crates/mcp-agent-mail-db/src/reconstruct.rs:1-21` ("the archive is the canonical source"). It also subsumes D2 (the reconciler is a continuous auto-reconstruct at row-level granularity instead of database-level).

2. **Fix the cleanup-scope mismatch (highest correctness leverage for the symptom we see).** Audit `cleanup_committed_message_after_consistency_failure` (called from `queries.rs:3210`) and `cleanup_agent_after_post_commit_probe_failure` (the agent-side analogue, search `queries.rs` for sister function). Either:
   - **(2a)** Extend cleanup to delete the canonical archive `.md` file via the same `mcp-agent-mail-storage` API used for the original write — making the failure case symmetric. The downside is data loss: a legitimate message that hit a transient visibility window but was correctly received elsewhere now disappears from the archive too.
   - **(2b, preferred)** Leave the archive intact and instead enqueue a deferred reconciliation job (one row-level INSERT replay) into the WBQ. The reconciler from option 1 handles it. The cleanup becomes a no-op that just logs the deferral; option 1 closes the loop.
   - Code touchpoints: `crates/mcp-agent-mail-db/src/queries.rs:3210` (`cleanup_message_after_post_commit_probe_failure`) and its callee `cleanup_committed_message_after_consistency_failure` (grep for the function name). Also audit the agent-side and reservation-side equivalents.

3. **Read-after-write verification in every mutating tool (defensive belt + suspenders).** At the top of each mutating tool's body — after `reject_write_if_degraded()?;` and the actual write — do a fresh-handle SELECT by id and refuse with a new `WRITE_NOT_VISIBLE` MCP error if the row isn't visible. The bounded retry from `fe52967d` already does most of this inside `verify_*_visible_after_commit`; the extension is to lift the contract from "best effort" to "if visibility not confirmed after budget, never return success." Risk: doubles the perceived latency of every write and may produce false-failure responses under load. Skip if option 1 lands.

4. **Tune the fresh-probe retry budget upward + add `pre_probe_flush_sync` (cheap incremental).** The current budget is ~2 s over 12 attempts (`fe52967d`'s `durability_probe_backoff`). Profile `commit_coalescer.latency_p99` (currently 1289 ms observed in production) — if it routinely exceeds 1.5 s, the budget is too tight. Also add an explicit `mcp_agent_mail_storage::get_commit_coalescer().flush_sync()` call at the top of the probe loop (storage/lib.rs around `wbq_flush`) so the coalescer is forced to publish before we start polling. Trades ~5–50 ms of latency for far higher first-probe success rate.

5. **Switch high-stakes writes to `BEGIN IMMEDIATE` instead of `BEGIN CONCURRENT` (config-level mitigation, last-resort).** The config flag already exists at `crates/mcp-agent-mail-core/src/config.rs:126` — set `FSQLITE_CONCURRENT_MODE=false` to fall back. This serializes writers and avoids the snapshot-advance window entirely, at the cost of write concurrency. Use only if all else fails or as a temporary mitigation under load.

6. **Upstream FrankenSQLite fix (GH#65 fcw_base_drift) — out of scope for this repo.** Track upstream. If upstream releases a fix, the right action here is to bump `sqlmodel-frankensqlite` in `Cargo.toml`, rebuild, and audit whether the workarounds above can be relaxed.

**Recommended ordering for the next session.** Land option 2b (cleanup-scope correction) + option 1 (continuous reconciler) together — they are codependent: 2b enqueues into the reconciler, the reconciler processes the queue and the file-system watch. That combo eliminates the phantom-write *symptom* (lost SQLite row) without touching FrankenSQLite's MVCC semantics. Option 3 becomes optional defense-in-depth. Options 4 and 5 are operational tuning. Option 6 is upstream.

**Tests required.**

- Unit (db crate): `cleanup_committed_message_after_consistency_failure` enqueues a reconciler job and does *not* delete the archive `.md` (option 2b). Use a fake reconciler queue.
- Integration (server crate): start a server with the reconciler enabled, manually drop a `.md` into the archive (simulating an out-of-band write or a phantom-write-recovery scenario), assert SQLite picks it up within the configured budget.
- Stress (storage crate): extend `stress_commit_coalescer_batching_100_writes` (`crates/mcp-agent-mail-storage/tests/stress_pipeline.rs:580`) to assert zero archive-ahead-of-SQLite drift at the end of the 100-write burst.
- Regression: the existing `fe52967d`-era tests at `queries.rs:18614` (`verify_message_recipients_visible_with_pooled_handle_sees_committed_rows`) and `queries.rs:18263` (`post_commit_probe_errors_are_never_advisory`) must still pass. The contract that the pooled handle is unsafe as a gate is locked in by that suite — do not weaken it.
- Live probe: after deployment, run a 1000-message burst against a fresh storage root and assert `health_check.semantic_readiness.status=ok` and SQLite counts match archive counts at the end.

**Risk assessment.**

| Risk | Likelihood | Mitigation |
|---|---|---|
| Reconciler races a full `am doctor reconstruct --yes` | Medium | Single-flight via `recovery.recovery_lock_active` — same lock D2 would use. |
| Reconciler writes a duplicate row when SQLite eventually catches up | Medium | Use `INSERT OR IGNORE` keyed on `(project_id, archive_path)` plus an idempotent dedupe check. The reconstruct path already handles this. |
| Filesystem-watch debounce too aggressive → reconciler misses bursts | Low | Combine watch with a 5 s poll fallback so any file present for >5 s is guaranteed to be picked up. |
| Cleanup change in option 2 deletes a legitimate archive `.md` | Medium-High | Option 2b (no archive delete + enqueue deferred replay) avoids this. Audit option 2a only with a feature flag default-off. |
| Reconciler holds writer locks long enough to interfere with normal writes | Low-Medium | Batch in small chunks (e.g., 10 rows per WBQ enqueue), yield between batches. |

**Code map (Class B work, in addition to D2/D4 below).**

| File | Lines | Role | Class B option |
|---|---|---|---|
| `crates/mcp-agent-mail-server/src/archive_reconciler.rs` (NEW) | — | Continuous archive→SQLite reconciler | option 1 (primary) |
| `crates/mcp-agent-mail-server/src/lib.rs` | bootstrap area near `integrity_guard`, `ack_ttl`, `cleanup` task spawns | Wire the reconciler into the server lifecycle | option 1 |
| `crates/mcp-agent-mail-db/src/queries.rs` | 3210, plus the `cleanup_committed_message_after_consistency_failure` callee | Cleanup-scope audit/fix | option 2 |
| `crates/mcp-agent-mail-db/src/queries.rs` | 1316, 1420, 1562, 1869, 3553 | Existing `fcw_base_drift` references (read for context) | all options |
| `crates/mcp-agent-mail-db/src/queries.rs` | 1769 (message probe), 1506 (agent probe) | Existing bounded-retry probe (from `fe52967d`) | option 4 (budget tuning) |
| `crates/mcp-agent-mail-db/src/reconstruct.rs` | top + `run_for_project` | Reuse for row-level reconciliation | option 1 |
| `crates/mcp-agent-mail-tools/src/messaging.rs` | 1427 (send_message), 2167 (archive write call site) | Trace the archive-then-SQLite ordering; optional read-after-write add | option 3 |
| `crates/mcp-agent-mail-storage/src/lib.rs` | `wbq_*`, `get_commit_coalescer`, `flush_sync` | WBQ + coalescer primitives the reconciler will use | option 1, option 4 |
| `crates/mcp-agent-mail-core/src/config.rs` | 126 | `FSQLITE_CONCURRENT_MODE` flag | option 5 |
| `~/Desktop/frankensqlite/crates/fsqlite-mvcc/src/lib.rs` + `observability.rs` | — | Upstream MVCC implementation; check for `fcw_base_drift` instrumentation | option 6 reference |

**Related parent context (DO read these before starting Class B work):**

- `prompts/mcp-agent-mail-immediate-read-after-write-fix-handoff.md` — 91 KB, the parent track for FrankenSQLite write-visibility work. Especially Mode 1 (lines 393–476) and Slice D (lines 672–779). Many of the analytical observations here are summarised from that document; do not reinvent them.
- `prompts/mcp-agent-mail-immediate-read-after-write-regression-error-manifest.md` — 27 KB, the original error manifest for the same bug class with reproduction evidence and 4 root-cause hypotheses.
- Commit `fe52967d`'s full body (`git show fe52967d`) — explicit description of the snapshot-advance window symptom set and why the bounded-retry mitigates but does not eliminate it.

**Out of scope for the next session.**

- Modifying FrankenSQLite itself (option 6) unless upstream releases a fix that we want to bump to.
- Removing the bounded-retry probe (`fe52967d`) — it's still load-bearing for the common case.
- Touching `mailbox_durability` state-machine transitions — they're golden-snapshot-tested and orthogonal.
- Renaming `degraded_read_only` — it's part of the public CLI surface (see `crates/mcp-agent-mail-cli/src/robot.rs:780`).

**Stop condition.** Class B is "fixed" when:
- A burst-write reproduction (1000 writes via `stress_pipeline.rs`) ends with `archive count == SQLite count` and `health_check.semantic_readiness.status=ok`.
- A simulated phantom write (manually drop a `.md` into the archive while server is running) is picked up by the reconciler within 5 s and SQLite count catches up.
- `cleanup_committed_message_after_consistency_failure` no longer creates archive-only orphans (verify by injecting a probe-failure in tests).
- The 91 KB parent handoff's Slice D acceptance criteria (lines 929–1003) still pass.

This section IS the standalone Class B handoff. Don't split it out.

## What's in this handoff: D2 + D4

### D2 — Detection without action

**Symptom.** When the SQLite index drifts behind the archive, `health_check` correctly classifies it as `red`/`degraded_read_only` and the "best-effort warning" path at `crates/mcp-agent-mail-server/src/lib.rs:13601-13643` (`log_active_database`) emits `tracing::warn!` once at server startup. **Nothing else reacts.** The 2026-05-08 corruption sat in `degraded_read_only` for 10 days because no operator was tailing the log. With D1 now in place, writes refuse loudly on every attempt — but the server still can't self-heal, and operators may not even know to look until the next push fails.

**Two fix shapes (pick one with operator):**

1. **Auto-reconstruct on detection (preferred but invasive).** When `inspect_archive_db_drift` returns `Some(drift)` *and* `recovery.recovery_lock_active == false` *and* a single-flight reconstruct lock is acquirable, schedule one in-process `reconstruct::run(storage_root)` attempt. On success, the next `compute_mailbox_verdict` probe naturally flips back to `Healthy`. On failure, surface `recovery.last_attempt_error` in `health_check` and stop retrying until next restart (avoid retry-storm loops).
   - **Hardest part**: the reconstruct subsystem assumes exclusive ownership of the storage root. Today `am doctor reconstruct --yes` only works when no `am serve-http` is running (per `.mailbox.activity.lock`). Making it work in-process requires either (a) lifting that lock for the duration of the reconstruct (the runtime already owns it), or (b) coordinating with the supervisor primitives in `mcp_agent_mail_core::mailbox_durability` to enter `Recovering` state first.
   - Code touchpoints: `crates/mcp-agent-mail-server/src/lib.rs:13601-13643` (trigger), `crates/mcp-agent-mail-db/src/reconstruct.rs::run()` (action), `crates/mcp-agent-mail-server/src/startup_checks.rs:1731-1748` (alternative trigger site at startup).

2. **Louder periodic warning + structured `recovery.last_detection_ts` (minimum).** If auto-reconstruct is too invasive for this branch:
   - Promote the one-shot `tracing::warn!` at `lib.rs:13630-13642` to a periodic emitter (every 60s while degraded) gated by a `tokio::time::interval` or `tokio::time::sleep_until`.
   - Add a `recovery.last_detection_ts` field to `health_check.recovery` so an external monitor can alert on drift detection without parsing log lines.
   - Add a `recovery.detection_count` counter that increments on each fresh drift detection across the process lifetime (so a single restart-loop doesn't accumulate noise).

**Tests.** Extend `crates/mcp-agent-mail-server/src/lib.rs` tests near line 15285 and 15393 (the existing "drift detection fires" assertions). Add at least:
- "auto-reconstruct on startup detection succeeds and flips state to Healthy" (if shape 1).
- "periodic warning re-emits at least N times across M seconds while degraded" (if shape 2).
- "auto-reconstruct does NOT fire when `recovery_lock_active=true`" (single-flight invariant).

### D4 — Backup discipline doesn't validate before snapshotting

**Symptom.** `storage.sqlite3.bak` (rotated every ~1h via `crates/mcp-agent-mail-server/src/backup_rotation.rs::rotate_storage_backups`) was written on 2026-05-18 11:41 — *after* the 2026-05-08 corruption — and inherited the empty-index state. The backup is therefore useless for the very case it should rescue from: a future operator running `am doctor restore <BACKUP_PATH>` would restore the same drifted SQLite they tried to escape.

**Fix shape.**

1. Before `rotate_storage_backups` writes the rotated copy, call the same probe used by `health_check`:
   ```rust
   let verdict = mcp_agent_mail_db::compute_mailbox_verdict(
       &database_url,
       storage_root,
       &mcp_agent_mail_db::VerdictOptions::fast(),
   );
   let durability = mcp_agent_mail_db::DurabilityState::from_mailbox_state(verdict.state);
   if !durability.allows_writes() {
       // Don't rotate a poisoned DB into the canonical backup slot.
       // Move it to a sibling `suspect/` directory and emit a tracing::warn.
       let suspect_dir = storage_root.join("backups").join("suspect");
       std::fs::create_dir_all(&suspect_dir)?;
       let suspect_path = suspect_dir.join(format!("storage.sqlite3.bak-{ts}"));
       // ... write to suspect_path instead of the canonical .bak
       return Ok(BackupRotationOutcome::SkippedDegraded { written_to: suspect_path });
   }
   ```
2. Teach `am doctor restore` to refuse files inside `backups/suspect/` unless `--force` is passed. Surface a clear "this backup was taken while the DB was already degraded; restoring will reproduce the degraded state" warning.
3. Add a `recovery.last_known_good_backup_ts` field to `health_check.recovery` so operators can see at a glance whether *any* current backup is trustworthy.

**Tests.** Extend `crates/mcp-agent-mail-server/src/backup_rotation.rs::tests` (existing tests at lines 382, 425, 441, 450 are the template). Add:
- "rotate_storage_backups_skips_when_degraded_and_writes_to_suspect_dir".
- "rotate_storage_backups_writes_canonical_when_healthy".
- `am doctor restore` integration test that refuses `backups/suspect/*` without `--force`.

## Suggested ordering for the next session

1. **Pre-work** — verify nothing in the live server regressed since this session: run `am doctor check` and `mcp__mcp-agent-mail__health_check` and confirm `health_level=green`, then `cargo test -p mcp-agent-mail-tools --test system_error_parity register_agent_under_degraded_sqlite_returns_database_degraded` should pass (this is the test added in this session).
   - **Pre-existing env note**: in the session that landed D1/D3, this test could not be linked on the operator's machine due to a CoreML/ONNX symbol mismatch (`ort_sys` was built for macOS 13.4 but the link target resolves against an older SDK; `_OBJC_CLASS_$_MLComputePlan` / `_OBJC_CLASS_$_MLOptimizationHints` undefined). This is **NOT a regression** — `cargo check -p mcp-agent-mail-tools` passes cleanly with the D1/D3 changes. If the same link failure recurs, either rebuild the ML deps against the current SDK (`cargo clean -p ort-sys && MACOSX_DEPLOYMENT_TARGET=14.0 cargo test ...`) or run the test in a CI environment where the ML toolchain is fresh.
2. **D2 shape decision** — ask the operator which shape (auto-reconstruct vs louder signaling). The auto-reconstruct version is materially more code but also materially more correct; the periodic-warning version is a 30-line patch that buys time.
3. **D4** — implement before D2 if the operator wants the safer-shipping path. D4 is fully local to `backup_rotation.rs` and doesn't touch the durability state machine.
4. **Workspace verification** — `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`.
5. **Binary swap (operator-gated)** — after all four defects (D1, D2, D3, D4) are landed and tests pass:
   - `cd /Users/alinaaitova/Desktop/mcp_agent_mail_rust && cargo build --release -p mcp-agent-mail-cli`
   - **Stop the LaunchAgent first**: `launchctl unload ~/Library/LaunchAgents/com.agent-mail.plist`
   - Swap binary: `cp target/release/am ~/.local/bin/am`
   - Reload: `launchctl load ~/Library/LaunchAgents/com.agent-mail.plist`
   - Verify with `curl http://127.0.0.1:8765/healthz` + `mcp__mcp-agent-mail__health_check`.
   - Run the smoke-test regression: register a throwaway agent, send a self-message, create + release a reservation; verify both archive AND SQLite get the writes.

## Code map (where D2/D4 land)

| File | Lines | Role | D2 | D4 |
|---|---|---|---|---|
| `crates/mcp-agent-mail-server/src/lib.rs` | 13601-13643 | `log_active_database` — the one-shot drift warning | yes | — |
| `crates/mcp-agent-mail-server/src/startup_checks.rs` | 1731-1748 | startup durability arms | yes (alt site) | — |
| `crates/mcp-agent-mail-server/src/backup_rotation.rs` | 164 | `rotate_storage_backups` | — | yes |
| `crates/mcp-agent-mail-db/src/reconstruct.rs` | top-of-file | `reconstruct::run()` callable | yes | — |
| `crates/mcp-agent-mail-tools/src/identity.rs` | 642-664 | `RecoveryStatusResponse` (extend with `last_attempt_error`, `last_detection_ts`, `last_known_good_backup_ts`) | yes | yes |
| `crates/mcp-agent-mail-tools/src/lib.rs` | new function | `tool_util::write_policy_label` (added in D3) — pattern reuse for D2/D4 status mapping | yes | yes |

## Open question (still unresolved)

- **§6.5 #3 from the parent handoff**: does `am doctor reconstruct` touch the Tantivy `search_index/` directory? Empirical answer (this session): NO — grep of `reconstruct.rs` for `search_index|tantivy|rebuild_search` returns zero matches. After reconstruct, full-text search may still be stale. Currently `search_messages` serves from archive snapshots when degraded, so user-visible impact is minimal post-repair. If D2 ships auto-reconstruct, decide whether to also auto-rebuild the search index (or keep that as a separate `am search reindex` operator command).

## Out of scope for D2/D4

- Anything touching `mcp_agent_mail_core::mailbox_durability` state-machine transitions — those are validated by an extensive golden-snapshot test suite and should not be rebalanced just to accommodate auto-reconstruct.
- Changing the public name `degraded_read_only` (`crates/mcp-agent-mail-cli/src/robot.rs:780` documents it as part of the CLI surface). The D3 `write_policy` field is the new honest signal.
- Any change to `bin/br-claim` / `bin/br-complete` / `install-bead-wrappers` skill — D5 already shipped and verified in this session.
