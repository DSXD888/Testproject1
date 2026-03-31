# L3 Operational Hardening — Execute Fix Beads

## Your Mission

Execute the fix beads created by the completion audit. Multiple agents may work on fix beads in parallel — use AgentMail coordination to avoid conflicts.

**Organization: FLAT.** Pick next ready fix bead, implement, verify, move on.
**Coordination: AGENTMAIL-FIRST.** Check inbox, claim via mail, reserve files before touching code.

---

## Source Documents

```
docs/strategy/layers/layer-3-operational-hardening.md    — Spec (SOT #1) — Tiebreaker if fix bead is ambiguous
docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-tdd.md — TDD (SOT #2) — Test definitions for verification
```

### Fix Beads
```bash
br list --label audit-fix         # All fix beads from completion audit
br ready --label audit-fix        # Ready fix beads
```

---

## AgentMail Coordination

**Project key:** `/Users/alinaaitova/Desktop/Testproject1`

### Before picking a fix bead:
```
fetch_inbox(project_key, agent_name, include_bodies=true)
search_messages(project_key, query="audit-fix OR fix-bead")
```

### For each fix bead:
1. **Claim:** `br update <id> --status=in_progress`
2. **Reserve files:** `file_reservation_paths(project_key, agent_name, paths=[...], ttl_seconds=3600, exclusive=true, reason="<fix-bead-id>")`
3. **Announce:** `send_message(project_key, sender_name, to=["all"], subject="[<fix-bead-id>] Claimed: ...", ack_required=true)`
4. **Read the bead:** `br show <id>` — self-contained with spec refs and TDD test IDs
5. **Implement the fix**
6. **Run the specific TDD tests** referenced in the bead
7. **Run full quality gates:**
   ```bash
   cargo check --all-targets
   cargo test
   cargo clippy --all-targets -- -D warnings
   cargo fmt --check
   bun run build && bun run test && bun run lint  # if TS files changed
   ubs $(git diff --name-only --cached)
   ```
8. **Verify acceptance criteria** from the fix bead
9. **Commit** with fix bead ID as prefix: `git commit -m "<fix-bead-id>: fix description"`
10. **Push:** `git push`
11. **Release reservations:** `release_file_reservations(project_key, agent_name)`
12. **Announce completion:** `send_message(project_key, sender_name, to=["all"], subject="[<fix-bead-id>] Complete")`
13. **Close bead:** `br close <id>`
14. **Loop** to next ready fix bead

---

## Quality Gates

```bash
# Rust
cargo check --all-targets
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check

# TypeScript (if TS files changed)
bun run build
bun run test
bun run lint

# UBS (before commit)
ubs $(git diff --name-only --cached)
```

---

## Mini Push (After Each Fix)

If gaps remain after this pass, go back to the completion audit (step 12) and re-audit.
Check inbox for coordination messages. Check next fix bead via `br ready --label audit-fix`.
Keep going until ALL fix beads are complete or you are BLOCKED on a SPEC_QUESTION.
Do NOT stop to ask what to work on next. Do NOT wait for permission.

If fully blocked:
1. Push all work: `git add . && git commit && git push`
2. Release all reservations: `release_file_reservations(project_key, agent_name)`
3. Sync beads: `bin/br-sync && git add .beads/ && git commit && git push`
4. Announce handoff via mail with context for next agent
