# L1 Daemon Core — Execution Push

> **Role:** You are a continuing Rust implementation agent building the Layer 1 daemon core. Re-orient and keep working.

---

## 1. Check Inbox

```
fetch_inbox(project_key="/Users/alinaaitova/Desktop/Testproject1", agent_name="your-name")
```

Acknowledge messages. Note conflicts or coordination requests from other agents.

## 2. Pull Latest

```bash
git pull --rebase
```

If other agents modified files you're working on, re-read those files before continuing.

## 3. Pick Next Bead

```bash
br ready --json
```

Choose the highest-priority non-epic bead with ID prefix `testproject1-63g.*`. If your current bead is stuck, check its dependencies with `br show <id>` and check file reservations. Do NOT pick beads outside the L1 scope.

## 4. Claim + Reserve + Announce

```bash
br update <id> --status=in_progress
```

```
file_reservation_paths(
  project_key="/Users/alinaaitova/Desktop/Testproject1",
  agent_name="your-name",
  paths=["crates/daemon/src/<target-files>"],
  ttl_seconds=3600, exclusive=true,
  reason="br-testproject1-63g.N"
)
```

If you get `FILE_RESERVATION_CONFLICT`, pick a different bead.

Announce start via `send_message` with `thread_id="br-testproject1-63g.N"`.

## 5. Implement

1. Read the bead description (`br show <id>`) — it contains spec sections, TDD test IDs, and file targets
2. Read the relevant spec sections in `docs/strategy/layers/layer-1-thin-local-runner.md`
3. Read the test specs in `docs/pipeline/l1-daemon-core/artifacts/l1-daemon-core-tdd.md`
4. Write production code + test code as specified by the bead
5. Verify: `cargo check --all-targets && cargo clippy --all-targets -- -D warnings && cargo test`

## 6. Build Gate

```bash
cargo check --all-targets && cargo clippy --all-targets -- -D warnings && cargo fmt --check
```

Fix all errors before proceeding. No `unsafe` code. No `unwrap()` in production code.

## 7. Test Gate

```bash
cargo test
```

Run bead-specific tests if the full suite is too slow. All tests in the bead's TDD spec must pass.

## 8. Close + Sync + Push

```bash
br close <id> --reason "Completed: <summary>"
br sync --flush-only
git pull --rebase
git add crates/daemon/ migrations/ skills/ .beads/
git commit -m "feat(daemon): <description>

br-testproject1-63g.N"
git push
```

## 9. Release + Announce

```
release_file_reservations(project_key="/Users/alinaaitova/Desktop/Testproject1", agent_name="your-name", paths=["crates/daemon/src/<target-files>"])
send_message(project_key="/Users/alinaaitova/Desktop/Testproject1", from_agent="your-name", to_agent="broadcast", thread_id="br-testproject1-63g.N", subject="[br-testproject1-63g.N] Completed", body="<summary>. File reservation released.")
```

## 10. Loop

Pick the next ready bead. Do NOT stop to ask what to work on. Do NOT wait for permission. Run `br ready` and continue from step 3.
