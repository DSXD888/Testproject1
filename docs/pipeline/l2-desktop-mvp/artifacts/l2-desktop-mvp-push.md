# L2 Desktop MVP — Push Prompt (Continuation)

> **Paste this into an existing agent session to keep it moving.**

---

Re-read `AGENTS.md` so it's fresh in your mind. Continue the great work.

**Project key:** `/Users/alinaaitova/Desktop/Testproject1`

If you're not registered yet:
```
macro_start_session(project_key="/Users/alinaaitova/Desktop/Testproject1", program="l2-desktop-executor", model="<your-model>")
```

If you have a bead currently in-progress that you haven't finished, **finish it first** — implement, verify, commit, push, announce, release.

Then:

1. **Check inbox** — `fetch_inbox(project_key="/Users/alinaaitova/Desktop/Testproject1", agent_name="<your-name>", include_bodies=true)` — see what other agents completed or flagged.

2. **Pull latest** — `git pull --rebase` to sync other agents' changes.

3. **Find ready work** — `br ready --json` — only pick beads with `l2-` prefix in their title (bead IDs are `testproject1-dr6.*`). Ignore all other prefixes.

4. **Claim it:**
   - `br update <id> --status=in_progress`
   - `file_reservation_paths(project_key="/Users/alinaaitova/Desktop/Testproject1", agent_name="<your-name>", paths=[<files from bead>], ttl_seconds=3600, exclusive=true, reason="<bead-id>")`
   - `send_message(project_key="/Users/alinaaitova/Desktop/Testproject1", sender_name="<your-name>", to=["broadcast"], thread_id="<bead-id>", subject="[<bead-id>] Claimed: <title>", ack_required=true)`

5. **Read the bead** — `br show <id>`. It's self-contained. Implement from the bead alone. Each bead has full file paths, type definitions, test specs, and acceptance criteria. Only consult the spec (`docs/strategy/layers/layer-2-single-operator-desktop.md`) as a tiebreaker.

6. **Study the codebase** — before implementing, read existing code your bead integrates with:
   - **Daemon IPC:** `crates/daemon/src/ipc/handshake.rs` for handshake protocol
   - **Daemon DB:** `crates/daemon/src/db/sqlite.rs` for commit_clock + write transactions
   - **Job Runner:** `crates/daemon/src/jobs/runner.rs` for MutationAck + visibleAfterSeq
   - **Sessions:** `crates/daemon/src/pty/session.rs` for SessionPhase lifecycle
   - **Artifacts:** `crates/daemon/src/agents/artifacts.rs` for ArtifactStore
   - **FTS skill:** `skills/procurement-ingestion/fts/` for CF skill reference

7. **Implement > verify > commit > push > announce > release > loop.**

Quality gates before every commit:
```bash
# Rust (always)
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo fmt --check
cargo test

# TypeScript (when TS files changed)
bun install && bun run build && bun run lint && bun run test

# Playwright (when UI files changed)
bunx playwright test
```

Do NOT stop to ask what to work on. Do NOT wait for permission. Do NOT ask if you should continue. Just keep going until every `l2-*` bead is done or you are fully blocked.

If fully blocked (no ready `l2-*` beads and nothing in-progress): run QA checks on completed code, write missing tests, review recent commits. If truly nothing to do, execute the Session Landing Protocol — push all work, release all reservations, announce handoff:

```
release_file_reservations(project_key="/Users/alinaaitova/Desktop/Testproject1", agent_name="<your-name>")
send_message(project_key="/Users/alinaaitova/Desktop/Testproject1", sender_name="<your-name>", to=["broadcast"], subject="Session handoff: <your-name> signing off", body_md="## Completed\n- ...\n## Notes\n- ...", importance="normal")
```
