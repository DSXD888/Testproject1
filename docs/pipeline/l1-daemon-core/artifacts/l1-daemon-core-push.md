# L1 Daemon Core — Execution Push

> **Role:** You are a continuing Rust implementation agent building the Layer 1 daemon core. Re-orient and keep working.

---

## 1. Re-read AGENTS.md

Re-read `AGENTS.md` before doing anything else. Refresh on coding conventions, toolchain, `br` workflow, AgentMail coordination, and session protocol.

---

## 2. Project Key

**AgentMail project key:** `/Users/alinaaitova/Desktop/Testproject1`

If you are NOT registered with AgentMail in this session:
```
macro_start_session(
  project_key="/Users/alinaaitova/Desktop/Testproject1",
  program="l1-daemon-executor",
  model="<your-model>"
)
```

---

## 3. Finish In-Progress Work First

```bash
br list --status=in_progress --json
```

If you have a bead in progress, finish it before picking new work.

---

## 4. Check Inbox

```
fetch_inbox(
  project_key="/Users/alinaaitova/Desktop/Testproject1",
  agent_name="<your-name>",
  include_bodies=true
)
```

Act on any messages before picking new work. Acknowledge what you've read.

---

## 5. Pull Latest

```bash
git pull --rebase
```

If other agents modified files you're working on, re-read those files before continuing.

---

## 6. Find Ready Work

```bash
br ready --json
```

Pick the highest-priority bead with zero blockers. Bead prefix: `testproject1-63g.*`. Do NOT pick beads outside the L1 scope.

---

## 7. Claim + Reserve + Announce

```bash
br update <id> --status=in_progress
```

```
file_reservation_paths(
  project_key="/Users/alinaaitova/Desktop/Testproject1",
  agent_name="<your-name>",
  paths=["crates/daemon/src/<target-files>"],
  ttl_seconds=3600,
  exclusive=true,
  reason="br-testproject1-63g.N"
)
```

If `FILE_RESERVATION_CONFLICT` → pick a different bead.

```
send_message(
  project_key="/Users/alinaaitova/Desktop/Testproject1",
  sender_name="<your-name>",
  to=["broadcast"],
  thread_id="br-testproject1-63g.N",
  subject="[br-testproject1-63g.N] Claimed: <bead-title>",
  body="Starting implementation. Files reserved: <list>",
  ack_required=true
)
```

---

## 8. Implement

The bead description is self-contained — it has the spec requirements, TDD test IDs, file listing, and acceptance criteria inline.

1. `br show <id>` — read the bead
2. Read relevant spec sections in `docs/strategy/layers/layer-1-thin-local-runner.md` (tiebreaker only)
3. Read test specs in `docs/pipeline/l1-daemon-core/artifacts/l1-daemon-core-tdd.md`
4. Write production code + test code

---

## 9. Quality Gates

```bash
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo fmt --check
cargo test
```

No `unsafe` code. No `unwrap()` in production code.

---

## 10. Close + Sync + Push

```bash
br close <id> --reason "Completed: <brief summary>"
br sync --flush-only
git pull --rebase
git add crates/daemon/ migrations/ skills/ .beads/
git commit -m "feat(daemon): <description>

br-testproject1-63g.N"
git push
```

---

## 11. Release + Announce Completion

```
release_file_reservations(
  project_key="/Users/alinaaitova/Desktop/Testproject1",
  agent_name="<your-name>"
)
```

```
send_message(
  project_key="/Users/alinaaitova/Desktop/Testproject1",
  sender_name="<your-name>",
  to=["broadcast"],
  thread_id="br-testproject1-63g.N",
  subject="[br-testproject1-63g.N] Completed: <bead-title>",
  body="Implementation complete. Files released. Tests passing.",
  ack_required=false
)
```

---

## 12. Loop

Go back to step 4. Pick the next ready bead. Do NOT stop to ask what to work on next. Do NOT wait for permission.

---

## 13. Session Landing Protocol

When fully blocked (no ready beads, all remaining depend on other agents):

1. Push all work: `git pull --rebase && git push`
2. Release ALL reservations:
   ```
   release_file_reservations(
     project_key="/Users/alinaaitova/Desktop/Testproject1",
     agent_name="<your-name>"
   )
   ```
3. Announce handoff:
   ```
   send_message(
     project_key="/Users/alinaaitova/Desktop/Testproject1",
     sender_name="<your-name>",
     to=["broadcast"],
     subject="Session landing — blocked on <bead-ids>",
     body="Completed: <list>. Blocked: <list and why>. All reservations released.",
     ack_required=false
   )
   ```
4. Do NOT leave file reservations active — they block other agents.

---

## If Blocked on a Single Bead

- Dependency not met → check if the blocker bead is in progress by another agent. Pick a different ready bead.
- Compilation error from another agent's code → `git pull --rebase` first. If still broken, create an error manifest and pick a different bead.
- Spec ambiguity → check `docs/strategy/layers/layer-1-thin-local-runner.md` for the authoritative answer. If genuinely ambiguous, create a `br create --type=question` bead and move on.

---

## Constitutional Constraints

- `#![forbid(unsafe_code)]` — no exceptions
- No `unwrap()` in production code
- Process-group cleanup verdict mandatory for every session
- Run manifest mandatory for every skill execution
- Integer pence for money (`_pence` suffix)
- Integer basis points for confidence (`_bps` suffix, 0-10000)
- Single-writer SQLite enforcement
