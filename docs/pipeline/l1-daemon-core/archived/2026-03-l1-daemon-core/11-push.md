# L1 Daemon Core — Push Prompt (Execution Continuation)

## When To Use

Use this prompt to continue an implementation session that paused, got stuck, or completed a bead and needs direction for the next one. Paste this into a fresh or continuing session.

---

## 1. Re-read AGENTS.md

Before doing anything else, re-read `AGENTS.md` to refresh on:
- Coding conventions, toolchain, compiler checks
- Beads workflow (`br` CLI, non-invasive — never runs git automatically)
- AgentMail coordination (register, reserve, announce)
- Session protocol (landing the plane — push before ending)

---

## 2. Project Key

**Project key for all AgentMail calls:** `/Users/alinaaitova/Desktop/Testproject1`

If you are NOT yet registered with AgentMail in this session, register first:

```
macro_start_session(
  project_key="/Users/alinaaitova/Desktop/Testproject1",
  program="l1-daemon-executor",
  model="<your-model>"
)
```

---

## 3. Finish In-Progress Work First

Check if you have a bead already in progress:

```bash
br list --status=in_progress --json
```

If you have an in-progress bead, **finish it before picking new work.** Read its description with `br show <id>`, complete the implementation, run quality gates, and close it.

---

## 4. Check Inbox

```
fetch_inbox(
  project_key="/Users/alinaaitova/Desktop/Testproject1",
  agent_name="<your-name>",
  include_bodies=true
)
```

Act on any messages before picking new work. Note conflicts, coordination requests, or completion announcements from other agents. Acknowledge messages you've read.

---

## 5. Pull Latest

```bash
git pull --rebase
```

If other agents modified files you're working on, re-read those files before continuing.

---

## 6. Pick Next Bead

```bash
br ready --json
```

Choose the highest-priority non-epic bead with ID prefix `testproject1-63g.*`. If your current bead is stuck, check its dependencies with `br show <id>` and check file reservations. Do NOT pick beads outside the L1 scope.

---

## 7. Claim + Reserve + Announce

### Update status
```bash
br update <id> --status=in_progress
```

### Reserve files
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

If `FILE_RESERVATION_CONFLICT` → pick a different bead. Do NOT wait.

### Announce start
```
send_message(
  project_key="/Users/alinaaitova/Desktop/Testproject1",
  sender_name="<your-name>",
  to=["<peers>"],  # from list_contacts — NEVER "broadcast"
  thread_id="br-testproject1-63g.N",
  subject="[br-testproject1-63g.N] Claimed: <bead-title>",
  body_md="Starting implementation. Files reserved: <list>",
  ack_required=true
)
```

---

## 8. Implement

1. Read the bead description (`br show <id>`) — it contains spec sections, TDD test IDs, and file targets
2. Read the relevant spec sections in `docs/strategy/layers/layer-1-thin-local-runner.md`
3. Read the test specs in `docs/pipeline/l1-daemon-core/artifacts/l1-daemon-core-tdd.md`
4. Write production code + test code as specified by the bead

---

## 9. Quality Gates

```bash
cargo check --all-targets && cargo clippy --all-targets -- -D warnings && cargo fmt --check && cargo test
```

Fix all errors before proceeding. No `unsafe` code. No `unwrap()` in production code.

---

## 10. Close + Sync + Push

```bash
br close <id> --reason "Completed: <summary>"
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
  to=["<peers>"],  # from list_contacts — NEVER "broadcast"
  thread_id="br-testproject1-63g.N",
  subject="[br-testproject1-63g.N] Completed: <bead-title>",
  body_md="Implementation complete. Files released. Tests passing.",
  ack_required=false
)
```

---

## 12. Loop

Go back to step 4 (Check Inbox). Pick the next ready bead. Do NOT stop to ask what to work on. Do NOT wait for permission.

---

## 13. Session Landing Protocol

When fully blocked (no ready beads, all remaining depend on other agents):

1. **Push all work:**
   ```bash
   git pull --rebase && git push
   ```

2. **Release ALL reservations:**
   ```
   release_file_reservations(
     project_key="/Users/alinaaitova/Desktop/Testproject1",
     agent_name="<your-name>"
   )
   ```

3. **Announce handoff:**
   ```
   send_message(
     project_key="/Users/alinaaitova/Desktop/Testproject1",
     sender_name="<your-name>",
     to=["<peers>"],  # from list_contacts — NEVER "broadcast"
     subject="Session landing — blocked on <bead-ids>",
     body_md="Completed: <list>. Blocked: <list and why>. All reservations released.",
     ack_required=false
   )
   ```

4. Do NOT leave file reservations active — they block other agents.

---

## Quick Reference

| Item | Value |
|------|-------|
| **Project key** | `/Users/alinaaitova/Desktop/Testproject1` |
| **Spec** | `docs/strategy/layers/layer-1-thin-local-runner.md` |
| **TDD** | `docs/pipeline/l1-daemon-core/artifacts/l1-daemon-core-tdd.md` |
| **Strategy** | `docs/pipeline/l1-daemon-core/artifacts/l1-daemon-core-strategy.md` |
| **Bootstrap** | `docs/pipeline/l1-daemon-core/artifacts/l1-daemon-core-bootstrap.md` |
| **Kickoff** | `docs/pipeline/l1-daemon-core/artifacts/l1-daemon-core-kickoff.md` |
| **Bead prefix** | `testproject1-63g.*` |
| **Bead CLI** | `br` |
| **Build gate** | `cargo check --all-targets && cargo clippy --all-targets -- -D warnings && cargo fmt --check && cargo test` |
