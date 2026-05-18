# Agent Mail SQLite Drift — D2 + D4 Follow-Up Handoff

> Pickup point for the next agent. D1 (write rejection) and D3 (`recovery.write_policy` field) landed in this session; D2 (auto-reconstruct on detection) and D4 (suspect-aware backup rotation) are still open. This document is self-contained: read it cold and you can act.

## Context (what already shipped in the parent handoff)

- **Defect manifest**: `prompts/agent-mail-sqlite-index-drift-error-manifest.md`
- **Parent fix handoff**: `prompts/agent-mail-sqlite-index-drift-fix-handoff.md`
- **D1**: `tool_util::reject_write_if_degraded()` (new) is called from the top of all 10 mutating MCP tool bodies in `crates/mcp-agent-mail-tools/src/{identity,messaging,reservations}.rs`. When the durability verdict says writes are unsafe (any `DurabilityState` except `Healthy`), the gate returns `DATABASE_DEGRADED` instead of letting the write coalescer silently drain a non-persisting write.
- **D3**: added `recovery.write_policy: "full" | "reject_all_mutations"` to `health_check.recovery` (via `RecoveryStatusResponse::write_policy`) and to the rejection error JSON (via `build_degraded_write_rejection_error` + the existing `archive_sqlite_degraded_error`).
- **D5**: `bin/br-claim` and `bin/br-complete` got a `preflight_health()` that aborts before any mutation when the server is red, plus a `~/.config/mcp-agent-mail/config.env` bearer-token fallback for Codex installs. Skill copies in `~/.claude/skills/install-bead-wrappers/` and `~/.codex/skills/install-bead-wrappers/` are byte-identical and updated; `SKILL.md` Step 1 Pre-flight now also greps `am doctor check` for drift signals.
- **State repair**: operator approved `am doctor reconstruct --yes`; SQLite was rebuilt from archive, `health_check` is now green.

The operator's binary at `/Users/alinaaitova/.local/bin/am` is **still the pre-fix `am 0.2.54`** — D1/D3 are in the source tree but not yet built/installed. The `~/Library/LaunchAgents/com.agent-mail.plist` LaunchAgent auto-respawns the server, so any binary swap MUST `launchctl unload` the agent first.

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
