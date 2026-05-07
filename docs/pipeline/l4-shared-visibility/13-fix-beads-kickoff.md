# L4 Shared Visibility — 13 · Fix Beads Execution Kickoff

> For fresh agents executing `audit-fix` beads from the L4 completion audit. Read this FIRST.

---

## Mission

You are executing **fix beads** for L4 Shared Visibility — gaps identified by the completion audit (step 12) between the spec and the actual implementation. Each fix bead is a targeted correction: a missing field, a wrong assertion, an unimplemented edge case, a missed cross-domain note, an unimplemented §5.2 stub minimum path, etc.

Work from the fix-bead graph (`br ready --label layer-4 --label audit-fix`). Each fix bead is **self-contained** — implement from the bead alone. The spec and TDD are tiebreakers if a fix bead is ambiguous.

**Organization: FLAT.** No hierarchy. Pick the next ready fix bead, implement it fully, move on.

**Coordination: AGENTMAIL-FIRST.** Register, check inbox, announce claims BEFORE touching code.

---

## Implementation Boundary

**EXISTS (already implemented during main execution):**
- L4 sync engine (push + pull + CAS) at `crates/daemon/src/db/sync.rs`
- 8 Supabase migrations applied
- Web portal at `apps/web/` (7 source files + tests)
- 4 validator overlays at `skills/validation/{procurement-ingestion,supplier-enrichment,document-acquisition,product-extraction}/`
- Validation enforcement mode (per-slice readiness, AGENT_SUSPECT_OUTPUT, provisional promotion)
- Calibration harness with 3 corpora + 9 mutation families + Wilson/rule-of-three math
- All test files in `crates/daemon/tests/{sqlite_schema,sqlite_supabase_sync,redaction_boundary}.rs` and `apps/web/e2e/contracts.spec.ts`

**FIX BEADS ADDRESS:** Gaps, missing edge cases, incorrect behavior, thin test coverage, spec requirements not fully met. These are targeted surgical fixes, NOT new features.

Specific gap categories the fix beads cover:
- Missing equal-version retry case (e.g., 3 of 4 cases handled, 1 missing)
- Missing redaction test for one summary projection
- Missing daemon-pull tombstone visibility (RLS policy missing the daemon-role exception)
- Missing minimum epoch-repair reproject path (§5.2 stub left as TODO)
- Missing one mutation family in calibration harness
- Missing 3-state FreshnessBadge prop type extension
- Missing CAS conflict IPC event from `decisions.rs`
- Missing schema hash CI gate workflow file

---

## Core Contracts / Non-Negotiables

(Same as main kickoff §5 — the contracts haven't changed. Fix beads must respect them.)

1. Push protocol transactional + atomic (single RPC, partial-apply forbidden)
2. Equal-version retry semantics — all 4 cases
3. Pull-before-push at startup
4. CAS for operator decisions with `base_remote_revision` + durable command ledger
5. Writer fencing token (`writer_epoch`) on every push batch
6. Schema epoch guard
7. Provisional promotion buffer for enforcement-active slices
8. Per-slice enforcement readiness with all 7 prerequisites
9. Web read-only is a SECURITY BOUNDARY (viewer-scoped credentials only)
10. `packages/ui` lint invariant: NO imports of `@tauri-apps/api`/`apps/desktop`/`crates/daemon`/`supabase`/`transport`
11. NO file deletion (AGENTS.md Rule 1)
12. Single-edit chokepoints: at most one bead per chokepoint at a time
13. Stage Validation Test Gate from AGENTS.md is the minimum gate

If code conflicts with these contracts, the contract wins.

---

## Source-of-Truth Loading (MANDATORY — Before Starting)

Read in this order:

1. **AGENTS.md** — agent guidelines, Stage Validation Test Gate, file deletion rules, branching, beads CLI invariants
   ```text
   AGENTS.md
   ```

2. **Spec — SOT #1** — the authority for WHAT the feature should do
   ```text
   docs/strategy/layers/layer-4-shared-visibility.md
   ```

3. **TDD — SOT #2** — test definitions for verification
   ```text
   docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-tdd.md
   ```

4. **Completion audit report** — what gaps exist and why
   ```text
   docs/pipeline/l4-shared-visibility/completion-audit-report-{date}.md
   ```

5. **Fix bead graph — your ACTUAL work orders**
   ```bash
   br list --label layer-4 --label audit-fix --json     # All fix beads
   br ready --label layer-4 --label audit-fix --json    # Ready fix beads
   ```

6. **Integration surfaces** — study for context on each fix
   ```text
   crates/daemon/src/db/sync.rs                                — sync engine main module
   crates/daemon/src/validation/policy.rs                      — per-slice readiness gate
   crates/daemon/src/validation/executor.rs                    — provisional promotion + quarantine
   crates/daemon/src/jobs/runner.rs                            — provisional write path
   crates/daemon/src/ipc/handlers/decisions.rs                 — CAS conflict event path
   crates/daemon/src/errors/manifest.rs                        — AGENT_SUSPECT_OUTPUT
   crates/daemon/src/main.rs                                   — startup wiring (Phase 6 area)
   migrations/0012_l4_sync_state.sql (or chosen NNNN)         — local_outbox/sync_batches/pull_cursors
   infra/supabase/migrations/0001-0008_*.sql                   — 8 Supabase migrations
   apps/web/src/{app,lib}/*                                    — 7 web files
   skills/validation/{4 overlays}/                             — domain validator overlays
   packages/ui/freshness-badge.tsx                             — 3-state extension
   crates/daemon/tests/{sqlite_schema,sqlite_supabase_sync,redaction_boundary}.rs  — test files
   apps/web/e2e/contracts.spec.ts                              — Playwright e2e
   tests/fixtures/calibration/...                              — calibration corpora
   ```

---

## AgentMail Setup (Do This ONCE At Session Start)

```
# 1. Register with the project
ensure_project(human_key="/Users/alinaaitova/Desktop/Testproject1")
register_agent(
  project_key="/Users/alinaaitova/Desktop/Testproject1",
  program="l4-fix-beads-executor",
  model="<your-model>",
  task_description="L4 Shared Visibility fix-bead execution"
)

# 2. Set contact policy
set_contact_policy(
  project_key="/Users/alinaaitova/Desktop/Testproject1",
  agent_name="<your-name>",
  policy="open"
)

# 3. Discover peers (refresh between beads)
list_contacts(
  project_key="/Users/alinaaitova/Desktop/Testproject1",
  agent_name="<your-name>"
)  # → e.g. ["BlueLake", "RedPeak"]

# 4. Check inbox for messages from prior sessions or other agents
fetch_inbox(
  project_key="/Users/alinaaitova/Desktop/Testproject1",
  agent_name="<your-name>",
  limit=20
)
```

Read and acknowledge any pending messages before starting work. If another agent left a blocker notice on a fix bead, address it first.

---

## Fix-Bead Completion Policy (CRITICAL)

**You MUST fully complete every fix bead you claim.** No partial completion. No "I'll finish this later." No moving on to another fix bead while one is in progress.

A fix bead is COMPLETE when:
1. All code changes described in the fix bead are implemented
2. The specific TDD tests referenced in the bead pass (run `cargo test --test <test_file>` for the affected file)
3. All build/lint/test verification commands pass with zero warnings (the FULL Stage Validation Test Gate from AGENTS.md)
4. Code is committed with fix bead ID prefix: `git commit -m "<fix-bead-id>: <description>"`
5. Fix bead is marked complete: `br close <fix-bead-id> --reason "Completed"`
6. Beads synced: `bin/br-sync && git add .beads/ && git commit -m "beads: close <id>" && git push`
7. Completion announced via AgentMail
8. File reservations released

**If you hit a blocker:**
- DO NOT abandon the fix bead and move to another
- DO NOT mark it complete with missing pieces
- DO NOT do mid-way commits of incomplete work
- Diagnose the blocker. Fix it yourself.
- Only escalate via AgentMail if the blocker is in another agent's code (announce with file path, line number, what's wrong, what's needed)

---

## Work Loop

```
ONE-TIME SETUP:
  1. AgentMail registration (above)
  2. Read AGENTS.md + spec + TDD + completion audit report
  3. br ready --label layer-4 --label audit-fix → understand the fix graph

MAIN LOOP (repeat until all fix beads complete):

  ┌─ PICK ──────────────────────────────────────────────┐
  │ br ready --label layer-4 --label audit-fix --json   │
  │ Pick the highest-priority ready fix bead.            │
  │ Read the fix bead description FULLY.                 │
  └──────────────────────────────────────────────────────┘
           │
  ┌─ VERIFY AVAILABLE ─────────────────────────────────┐
  │ # GUARD: prevent duplicate claims                  │
  │ br show <fix-bead-id> --json                       │
  │ # → if status=in_progress, SKIP this bead          │
  │ search_messages(                                   │
  │   project_key="/Users/alinaaitova/Desktop/Testproject1",│
  │   agent_name=<your-name>,                          │
  │   query="[<fix-bead-id>] Start"                    │
  │ )                                                  │
  │ # → if claim message found, SKIP this bead         │
  └────────────────────────────────────────────────────┘
           │
  ┌─ CLAIM + ANNOUNCE ──────────────────────────────────┐
  │ br update <fix-bead-id> --status=in_progress         │
  │                                                      │
  │ file_reservation_paths(                              │
  │   project_key="/Users/alinaaitova/Desktop/Testproject1",│
  │   agent_name=<your-name>,                            │
  │   paths=[<files this fix bead touches>],             │
  │   ttl_seconds=3600, exclusive=true,                  │
  │   reason="<fix-bead-id>"                             │
  │ )                                                    │
  │                                                      │
  │ send_message(                                        │
  │   project_key="/Users/alinaaitova/Desktop/Testproject1",│
  │   sender_name=<your-name>,                           │
  │   to=["<peers>"],   # NEVER "broadcast"              │
  │   thread_id="<fix-bead-id>",                         │
  │   subject="[<fix-bead-id>] Start: <title>",          │
  │   body_md="Claiming fix bead. Files: <list>."        │
  │ )                                                    │
  └──────────────────────────────────────────────────────┘
           │
  ┌─ IMPLEMENT ─────────────────────────────────────────┐
  │ 1. Study the target files — understand current state │
  │ 2. Read the spec L4-FR section referenced in bead    │
  │ 3. Write/fix tests FIRST (from TDD test IDs)         │
  │ 4. Implement the fix to make tests pass              │
  │ 5. Verify ACs from the fix bead                      │
  │                                                      │
  │ DO NOT cut corners. DO NOT skip tests.               │
  │ DO NOT leave TODOs. DO NOT do mid-way commits.       │
  └──────────────────────────────────────────────────────┘
           │
  ┌─ VERIFY ────────────────────────────────────────────┐
  │ # Stage Validation Test Gate (full)                  │
  │ cargo fmt --check                                    │
  │ cargo check --all-targets                            │
  │ cargo clippy --all-targets -- -D warnings            │
  │ cargo test -p daemon --bin daemon                    │
  │ for t in crates/daemon/tests/*.rs; do                │
  │   cargo test --test "$(basename "$t" .rs)" || exit 1 │
  │ done                                                 │
  │ bun run test                                         │
  │ bun run lint                                         │
  │ # If your fix bead touches apps/web:                 │
  │ (cd apps/web && bun run test:e2e)                    │
  │                                                      │
  │ Fix ALL failures before proceeding.                  │
  │ Never proceed with failing checks.                   │
  └──────────────────────────────────────────────────────┘
           │
  ┌─ COMMIT ────────────────────────────────────────────┐
  │ ubs $(git diff --name-only --cached)                 │
  │ git add <specific-files-only>                        │
  │ git commit -m "<fix-bead-id>: <short description>"   │
  │ git pull --rebase                                    │
  │ git push                                             │
  │ # Work is NOT done until push succeeds.              │
  └──────────────────────────────────────────────────────┘
           │
  ┌─ COMPLETE + RELEASE ────────────────────────────────┐
  │ br close <fix-bead-id> --reason "Completed"          │
  │ bin/br-sync                                          │
  │ git add .beads/                                      │
  │ git commit -m "beads: close <fix-bead-id>"           │
  │ git pull --rebase && git push                        │
  │                                                      │
  │ send_message(                                        │
  │   project_key="/Users/alinaaitova/Desktop/Testproject1",│
  │   sender_name=<your-name>,                           │
  │   to=["<peers>"],   # NEVER "broadcast"              │
  │   thread_id="<fix-bead-id>",                         │
  │   subject="[<fix-bead-id>] Complete: <title>",       │
  │   body_md="Fix complete. Commit: <hash>. All pass."  │
  │ )                                                    │
  │                                                      │
  │ release_file_reservations(                           │
  │   project_key="/Users/alinaaitova/Desktop/Testproject1",│
  │   agent_name=<your-name>                             │
  │ )                                                    │
  └──────────────────────────────────────────────────────┘
           │
        LOOP → back to PICK
```

"Use debug builds for fast iteration."

---

## Verification Commands

```bash
# Rust workspace (full Stage Validation Test Gate)
cargo fmt --check
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test -p daemon --bin daemon
for t in crates/daemon/tests/*.rs; do
  cargo test --test "$(basename "$t" .rs)" || exit 1
done

# TypeScript packages
bun run test
bun run lint
bun run build

# apps/web (if your fix bead touches it)
(cd apps/web && bun run test:e2e)

# Pre-commit
ubs $(git diff --name-only --cached)
```

Constitutional constraints (unchanged from main execution):
- `#![forbid(unsafe_code)]`
- No `unwrap()` in production code paths
- Rust 2024 nightly
- Bun only (no npm/yarn/pnpm) for TypeScript
- `packages/ui` lint rule: no imports of `@tauri-apps/api`/`apps/desktop`/`crates/daemon`/`supabase`/`transport`

---

## Completion Gate

When ALL fix beads are complete:
- Run the FULL Stage Validation Test Gate one final time
- Verify build is green with zero warnings
- Verify all 18 DoD criteria from the spec are met (manual walkthrough)
- Re-run the completion audit (step 12) — must return COMPLETE
- If any gaps remain, go back to step 12 and re-audit (loop)
- Do NOT stop to ask what to work on next — pick the next ready fix bead
- Do NOT wait for permission between fix beads

If fully blocked on all remaining fix beads: push all work, release all reservations, announce handoff via AgentMail with status of what's done and what's blocked.

---

## AgentMail Quick Reference

```
**CRITICAL:** Agent Mail has NO broadcast. `"broadcast"` is NOT a valid recipient. Discover peers via `list_contacts(project_key, agent_name)` after registration; store as `<peers>`.

# Discover peers (after registration; refresh between beads)
list_contacts(project_key="/Users/alinaaitova/Desktop/Testproject1", agent_name=<your-name>)

# Check inbox (between fix beads)
fetch_inbox(project_key="/Users/alinaaitova/Desktop/Testproject1", agent_name=<your-name>, limit=10)

# Claim fix bead — to=<peers> is MANDATORY (never use "broadcast")
send_message(project_key="/Users/alinaaitova/Desktop/Testproject1", sender_name=<your-name>, to=["<peers>"],
  thread_id="<fix-bead-id>", subject="[<fix-bead-id>] Start: <title>")

# Complete fix bead
send_message(project_key="/Users/alinaaitova/Desktop/Testproject1", sender_name=<your-name>, to=["<peers>"],
  thread_id="<fix-bead-id>", subject="[<fix-bead-id>] Complete: <title>",
  body_md="Commit: <hash>. All checks pass.")

# Report blocker
send_message(project_key="/Users/alinaaitova/Desktop/Testproject1", sender_name=<your-name>, to=["<peers>"],
  thread_id="<fix-bead-id>", subject="[<fix-bead-id>] BLOCKED: <issue>",
  body_md="File: <path>, Line: <N>. Issue: <desc>. Needed: <what>")

# Release files
release_file_reservations(project_key="/Users/alinaaitova/Desktop/Testproject1", agent_name=<your-name>)
```

---

## Quick Reference

| | |
|---|---|
| Spec | `docs/strategy/layers/layer-4-shared-visibility.md` |
| TDD | `docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-tdd.md` |
| Completion audit report | `docs/pipeline/l4-shared-visibility/completion-audit-report-{date}.md` |
| Fix bead graph | `br ready --label layer-4 --label audit-fix --json` |
| Bead CLI | `br` |
| Bead sync | `bin/br-sync` |
| AgentMail key | `/Users/alinaaitova/Desktop/Testproject1` |
| Branch | `main` |

---

## Critical Rules (numbered)

1. Fix beads are NOT new features — they're targeted corrections of gaps from the completion audit
2. Implement from the fix bead body alone; spec is tiebreaker only
3. NEVER delete files (AGENTS.md Rule 1)
4. NEVER stash other agents' work
5. Use `br` (NOT `bd`); use `bin/br-sync` (NOT raw `br sync`)
6. Stage Validation Test Gate from AGENTS.md is the minimum gate — iterate every `crates/daemon/tests/*.rs`
7. `packages/ui` lint invariant: no imports of `@tauri-apps/api`/`apps/desktop`/`crates/daemon`/`supabase`/`transport`
8. Web is read-only — viewer-scoped credentials only — security boundary, not UI convention
9. Do NOT ask what to work on — `br ready --label layer-4 --label audit-fix` answers
10. Work is NOT done until `git push` succeeds
11. After every fix bead: `bin/br-sync && git add .beads/ && git commit && git push`
12. After ALL fix beads done, re-run completion audit (step 12) to verify COMPLETE verdict
