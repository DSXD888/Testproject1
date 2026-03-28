# L1 Daemon Core — Execute Fix Beads

## Your Mission

Execute the fix beads created by the completion audit. Multiple agents may work on fix beads in parallel — use AgentMail coordination to avoid conflicts.

**Organization: FLAT.** Pick next ready fix bead, implement, verify, move on.
**Coordination: AGENTMAIL-FIRST.** Check inbox, claim via mail, reserve files before touching code.

---

## Source Documents

```
docs/strategy/layers/layer-1-thin-local-runner.md                   — Spec (SOT #1) — Tiebreaker if fix bead is ambiguous
docs/pipeline/l1-daemon-core/artifacts/l1-daemon-core-tdd.md        — TDD (SOT #2) — Test definitions for verification
```

### Fix Beads

```bash
br list --label audit-fix --json    # All fix beads from completion audit
br ready --label audit-fix          # Ready fix beads (no blockers)
```

---

## AgentMail Registration

**Project key:** `/Users/alinaaitova/Desktop/Testproject1`

If not already registered in this session:

```
macro_start_session(
  project_key="/Users/alinaaitova/Desktop/Testproject1",
  program="l1-daemon-fix-executor",
  model="<your-model>"
)
```

---

## AgentMail Coordination

### Before picking a fix bead:

```
fetch_inbox(
  project_key="/Users/alinaaitova/Desktop/Testproject1",
  agent_name="<your-name>",
  include_bodies=true
)
```

```
search_messages(
  project_key="/Users/alinaaitova/Desktop/Testproject1",
  query="audit-fix OR fix-bead OR in_progress"
)
```

Act on any messages before picking work. If another agent claimed a fix bead, DO NOT take it.

---

## Workflow Per Fix Bead

### 1. Pick ready fix bead

```bash
br ready --label audit-fix --json
```

Choose the highest-priority fix bead with zero blockers.

### 2. Claim

```bash
br update <id> --status=in_progress
```

### 3. Reserve files

```
file_reservation_paths(
  project_key="/Users/alinaaitova/Desktop/Testproject1",
  agent_name="<your-name>",
  paths=["crates/daemon/src/<target-files>"],
  ttl_seconds=3600,
  exclusive=true,
  reason="br-<fix-bead-id>"
)
```

If `FILE_RESERVATION_CONFLICT` → pick a different fix bead.

### 4. Announce start

```
send_message(
  project_key="/Users/alinaaitova/Desktop/Testproject1",
  sender_name="<your-name>",
  to=["broadcast"],
  thread_id="br-<fix-bead-id>",
  subject="[br-<fix-bead-id>] Claimed: <fix-title>",
  body="Starting fix implementation. Files reserved: <list>",
  ack_required=true
)
```

### 5. Read the bead

```bash
br show <id>
```

The bead is self-contained with spec refs and TDD test IDs.

### 6. Implement the fix

Write the code change specified by the fix bead.

### 7. Run the specific TDD tests

```bash
cargo test <bead_specific_tests>
```

Run the tests referenced in the bead's TDD test IDs.

### 8. Run full quality gates

```bash
cargo check --all-targets && cargo clippy --all-targets -- -D warnings && cargo fmt --check && cargo test
```

ALL must pass before proceeding.

### 9. Close + sync + push

```bash
br close <id> --reason "Fix: <description>"
br sync --flush-only
git pull --rebase
git add crates/daemon/ migrations/ skills/ .beads/
git commit -m "l1: fix — <bead-title>

br-<fix-bead-id>"
git push
```

### 10. Release reservations

```
release_file_reservations(
  project_key="/Users/alinaaitova/Desktop/Testproject1",
  agent_name="<your-name>"
)
```

### 11. Announce completion

```
send_message(
  project_key="/Users/alinaaitova/Desktop/Testproject1",
  sender_name="<your-name>",
  to=["broadcast"],
  thread_id="br-<fix-bead-id>",
  subject="[br-<fix-bead-id>] Completed: <fix-title>",
  body="Fix implemented. Files released. Tests passing.",
  ack_required=false
)
```

### 12. Loop

Go back to the inbox check. Pick the next ready fix bead.

---

## Mini Push (After Each Fix)

If gaps remain after this pass, go back to the completion audit (step 12 in the pipeline) and re-audit.

Check inbox for coordination messages. Check next fix bead:
```bash
br ready --label audit-fix --json
```

Keep going until ALL fix beads are complete or you are BLOCKED on a SPEC_QUESTION.
Do NOT stop to ask what to work on next. Do NOT wait for permission.

---

## Session Landing Protocol

When fully blocked (no ready fix beads, all remaining depend on other agents):

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
     subject="Fix session landing — blocked on <bead-ids>",
     body="Completed fixes: <list>. Blocked: <list and why>. All reservations released.",
     ack_required=false
   )
   ```
4. Do NOT leave file reservations active — they block other agents.

---

## Constitutional Constraints

- `#![forbid(unsafe_code)]` — no exceptions, even in fixes
- No `unwrap()` in production code
- Every session must have a cleanup verdict
- Every skill run must produce a run manifest
- Build + lint + format + test must pass after every fix
