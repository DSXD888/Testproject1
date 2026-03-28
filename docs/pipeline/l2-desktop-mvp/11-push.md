# L2 Desktop MVP — Push Prompt (Continuation Nudge)

> This file IS the push prompt. Paste it into a session to re-orient an agent that paused or completed beads.

---

## L2 Desktop MVP — Execution Push

> **Role:** You are a continuing implementation agent building the Layer 2 desktop MVP for a UK public procurement intelligence platform. Re-orient and keep working.

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
  program="l2-desktop-executor",
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
bun install          # in case package.json changed
```

If other agents modified files you're working on, re-read those files before continuing.

---

## 6. Find Ready Work

```bash
br ready --json
```

Pick the highest-priority bead with zero blockers. Bead prefix: `l2`. Do NOT pick beads outside the L2 scope.

---

## 7. Claim + Reserve + Announce

```bash
br update <id> --status=in_progress
```

```
file_reservation_paths(
  project_key="/Users/alinaaitova/Desktop/Testproject1",
  agent_name="<your-name>",
  paths=["<files-from-bead-description>"],
  ttl_seconds=3600,
  exclusive=true,
  reason="<bead-id>"
)
```

If `FILE_RESERVATION_CONFLICT` → pick a different bead.

```
send_message(
  project_key="/Users/alinaaitova/Desktop/Testproject1",
  sender_name="<your-name>",
  to=["broadcast"],
  thread_id="<bead-id>",
  subject="[<bead-id>] Claimed: <bead-title>",
  body="Starting implementation. Files reserved: <list>",
  ack_required=true
)
```

---

## 8. Implement

The bead description is self-contained — it has the spec requirements, TDD test IDs, file listing, and acceptance criteria inline.

1. `br show <id>` — read the bead
2. Read relevant spec sections in `docs/strategy/layers/layer-2-single-operator-desktop.md` (tiebreaker only)
3. Read test specs in `docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-tdd.md`
4. Write production code + test code

---

## 9. Quality Gates

**Rust (when Rust files changed):**
```bash
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo fmt --check
cargo test
```

**TypeScript (when TS/TSX files changed):**
```bash
bun install
bun run build
bun run lint
bun run test
```

**Playwright (when UI files changed):**
```bash
bunx playwright test
```

No `unsafe` Rust code. No `unwrap()` in production Rust code. Integer pence for money, integer basis points for percentages.

---

## 10. Close + Sync + Push

```bash
br close <id> --reason "Completed: <brief summary>"
br sync --flush-only
git pull --rebase
git add apps/ packages/ crates/daemon/ migrations/ skills/ .beads/
git commit -m "feat(desktop): <description>

<bead-id>"
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
  thread_id="<bead-id>",
  subject="[<bead-id>] Completed: <bead-title>",
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
- Spec ambiguity → check `docs/strategy/layers/layer-2-single-operator-desktop.md` for the authoritative answer. If genuinely ambiguous, create a `br create --type=question` bead and move on.

---

## Constitutional Constraints

- `#![forbid(unsafe_code)]` — no exceptions (Rust)
- No `unwrap()` in production Rust code
- Integer pence for money (`_pence` suffix)
- Integer basis points for confidence/percentages (`_bps` suffix, 0-10000)
- Agent-authored content is untrusted input — sanitize all Markdown/ANSI rendering
- `packages/ui` components must NOT import data-fetching hooks, daemon modules, or Supabase clients
- `decisionBasisHash` is daemon-authored — never compute in TypeScript
