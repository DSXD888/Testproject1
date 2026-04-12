# L2 Desktop MVP — Execute Fix Beads

## Your Mission

Execute the fix beads created by the completion audit. Multiple agents may work on fix beads in parallel — use AgentMail coordination to avoid conflicts.

**Organization: FLAT.** Pick next ready fix bead, implement, verify, move on.
**Coordination: AGENTMAIL-FIRST.** Check inbox, claim via mail, reserve files before touching code.

---

## Source Documents

```
docs/strategy/layers/layer-2-single-operator-desktop.md                — Spec (SOT #1) — Tiebreaker if fix bead is ambiguous
docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-tdd.md          — TDD (SOT #2) — Test definitions for verification
```

### Fix Beads
```bash
br list --label audit-fix     # All fix beads from completion audit
br ready --label audit-fix    # Ready fix beads
```

---

## AgentMail Coordination

**Project key:** `/Users/alinaaitova/Desktop/Testproject1`

### Before picking a fix bead:
```
fetch_inbox(
  project_key="/Users/alinaaitova/Desktop/Testproject1",
  agent_name="<your-name>",
  include_bodies=true
)
search_messages(
  project_key="/Users/alinaaitova/Desktop/Testproject1",
  query="audit-fix OR fix-bead"
)
```

### For each fix bead:
1. **Claim:** `br update <id> --status=in_progress`
2. **Reserve files:**
   ```
   file_reservation_paths(
     project_key="/Users/alinaaitova/Desktop/Testproject1",
     agent_name="<your-name>",
     paths=[<files-from-fix-bead>],
     ttl_seconds=3600,
     exclusive=true,
     reason="<fix-bead-id>"
   )
   ```
3. **Announce:**
   ```
   send_message(
     project_key="/Users/alinaaitova/Desktop/Testproject1",
     sender_name="<your-name>",
     to=["<peers>"],  # from list_contacts — NEVER "broadcast"
     thread_id="<fix-bead-id>",
     subject="[<fix-bead-id>] Claimed: <fix-description>",
     ack_required=true
   )
   ```
4. **Read the bead:** `br show <id>` — self-contained with spec refs and TDD test IDs
5. **Implement the fix**
6. **Run the specific TDD tests** referenced in the bead
7. **Run full quality gates:**
   ```bash
   # Rust (if Rust files changed)
   cargo check --all-targets
   cargo clippy --all-targets -- -D warnings
   cargo fmt --check
   cargo test

   # TypeScript (if TS files changed)
   bun run build
   bun run lint
   bun run test

   # Playwright (if UI files changed)
   bunx playwright test
   ```
8. **Verify acceptance criteria** from the fix bead
9. **Commit** with fix bead ID as prefix:
   ```bash
   br close <id> --reason "Fixed: <summary>"
   br sync --flush-only
   git pull --rebase
   git add <changed-files> .beads/
   git commit -m "fix(desktop): <description>

   <fix-bead-id>"
   git push
   ```
10. **Release reservations:**
    ```
    release_file_reservations(
      project_key="/Users/alinaaitova/Desktop/Testproject1",
      agent_name="<your-name>"
    )
    ```
11. **Announce completion:**
    ```
    send_message(
      project_key="/Users/alinaaitova/Desktop/Testproject1",
      sender_name="<your-name>",
      to=["<peers>"],  # from list_contacts — NEVER "broadcast"
      thread_id="<fix-bead-id>",
      subject="[<fix-bead-id>] Complete: <fix-description>",
      ack_required=false
    )
    ```
12. **Loop** to next ready fix bead

---

## Quality Gates

```bash
# Rust
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo fmt --check
cargo test

# TypeScript
bun run build
bun run lint
bun run test

# Playwright E2E
bunx playwright test
```

---

## Mini Push (After Each Fix)

If gaps remain after this pass, go back to the completion audit (step 12) and re-audit.
Check inbox for coordination messages. Check next fix bead via `br ready --label audit-fix`.
Keep going until ALL fix beads are complete or you are BLOCKED on a SPEC_QUESTION.
Do NOT stop to ask what to work on next. Do NOT wait for permission.

If fully blocked: push all work, release all reservations, announce handoff via mail.
