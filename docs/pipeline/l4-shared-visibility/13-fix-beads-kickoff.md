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
14. **Bead claim discipline via wrappers (AGENTS.md Rule 2 — constitutional).** Use `bin/br-claim <fix-bead-id>` to start and `bin/br-complete <fix-bead-id>` to finish. The wrappers are atomic: claim reserves files, sends the `[<id>] Start` mail to peers, and runs `br update --status=in_progress` in the correct order; complete sends `[<id>] Complete`, runs `br close`, syncs `.beads/`, and releases reservations. Raw `br update --status=in_progress` and `br close` are FORBIDDEN — they bypass Agent Mail and the file-reservation collision detector. The pre-commit hook at `.git/hooks/hooks.d/pre-commit/50-agent-mail.py` enforces this; future Phase B extension will additionally block any `<bead-id>:`-prefixed commit with no corresponding Start mail.
15. **Agent Mail is a HARD GATE at both ends of every fix bead** — `bin/br-claim` and `bin/br-complete` send the Start/Complete mails for you, so the discipline is "always run the wrappers, never bypass." A fix bead with no `[<id>] Start` mail in the archive was never claimed (sibling agents will collide); a fix bead with no `[<id>] Complete` mail is not closed in the swarm's view regardless of `br` state. `fetch_inbox(...)` between every fix bead — surface any blocker notices another agent left for you before you pick the next one.
16. **AGENTS.md is the constitutional source.** §"Source-of-Truth Loading" item #1 says READ FIRST — that means full read, top to bottom, before your first non-AgentMail tool call. Re-read it (or at minimum re-skim Rule 1, Rule 2, Stage Validation Test Gate, and the `br`/`bin/br-sync`/`bin/br-claim`/`bin/br-complete` conventions) at the start of every fresh session and after any compaction. AGENTS.md wins over this kickoff if they ever disagree.

If code conflicts with these contracts, the contract wins.

---

## Source-of-Truth Loading (MANDATORY — Before Starting)

**STOP. Read item #1 fully before any other tool call.** This is constitutional — AGENTS.md governs all the rules below. Skipping it leads to the same Agent Mail discipline collapse that was observed during the L4 main execution (4 Start mails sent in the first 3 minutes, then 0 mails for the next 10 hours of work). Re-read AGENTS.md (or at minimum re-skim Rule 1, Stage Validation Test Gate, and the `br`/`bin/br-sync` conventions) at the start of every fresh session and after any compaction.

Read in this order:

1. **AGENTS.md** — **READ FULLY** — agent guidelines, Stage Validation Test Gate, file deletion rules, branching, beads CLI invariants, Agent Mail conventions
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

## AgentMail Setup (DO THIS FIRST — before reading anything else)

**STOP.** Register with AgentMail NOW, before reading any other source-of-truth document below. This makes you visible to the swarm and prevents the silent-claim collisions that contaminated the L4 main run. Everything else waits until you have your `agent_name` AND you have exported it as `AGENT_MAIL_NAME` (the wrappers in `bin/br-claim` / `bin/br-complete` require that env var).

```
# 1. Register with the project
ensure_project(human_key="/Users/alinaaitova/Desktop/Testproject1")
register_agent(
  project_key="/Users/alinaaitova/Desktop/Testproject1",
  program="l4-fix-beads-executor",
  model="<your-model>",
  task_description="L4 Shared Visibility fix-bead execution"
)

# 2. Export your agent name for the wrappers (REQUIRED — bin/br-claim / bin/br-complete read this)
#    Run this in the same shell you'll run the wrappers in:
export AGENT_MAIL_NAME="<your-name>"

# 3. Set contact policy
set_contact_policy(
  project_key="/Users/alinaaitova/Desktop/Testproject1",
  agent_name="<your-name>",
  policy="open"
)

# 4. Discover peers (refresh between beads)
list_contacts(
  project_key="/Users/alinaaitova/Desktop/Testproject1",
  agent_name="<your-name>"
)  # → e.g. ["BlueLake", "RedPeak"]

# 5. Check inbox for messages from prior sessions or other agents
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

A fix bead is CLAIMED only when `bin/br-claim <fix-bead-id>` succeeded. The wrapper is atomic — it (a) runs `br show` to confirm the bead is open, (b) checks for prior `[<id>] Start` mail (idempotency), (c) reserves the files listed in the bead's `### Files to Create/Modify` section (or `--paths` overrides), (d) sends `[<id>] Start: <title>` to peers from `list_contacts`, (e) sets the bead `in_progress` — in that order. If `br-claim` exits non-zero, the bead is NOT claimed; do not start coding.

A fix bead is COMPLETE only when ALL of these are true:
1. All code changes described in the fix bead are implemented
2. The specific TDD tests referenced in the bead pass (run `cargo test --test <test_file>` for the affected file)
3. All build/lint/test verification commands pass with zero warnings (the FULL Stage Validation Test Gate from AGENTS.md)
4. Code is committed with fix bead ID prefix: `git commit -m "<fix-bead-id>: <description>"`
5. `git push` succeeded (no push = no progress; `bin/br-complete` refuses to run otherwise)
6. `bin/br-complete <fix-bead-id>` succeeded — the wrapper atomically: (a) verifies status==in_progress and that you sent the Start mail, (b) finds the `<bead-id>:`-prefixed commit and verifies HEAD == upstream, (c) sends `[<id>] Complete: <title>`, (d) runs `br close --reason "Completed"`, (e) runs `bin/br-sync` and pushes the `.beads/` JSONL update, (f) releases your file reservations.

**Raw `br update --status=in_progress` and `br close <id>` are FORBIDDEN per AGENTS.md Rule 2.** Skipping the wrappers means skipping the AgentMail Start/Complete and the file reservations, which means sibling agents will collide with you. If the next agent in the swarm cannot find your `[<id>] Start` or `[<id>] Complete` message via `search_messages(..., query="[<fix-bead-id>]")`, they will assume the fix bead is still up for grabs.

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
  ┌─ CLAIM + ANNOUNCE (atomic via wrapper) ─────────────┐
  │ # AGENTS.md Rule 2: ONLY use bin/br-claim.           │
  │ # NEVER raw `br update --status=in_progress` —       │
  │ # the wrapper does file-reservation FIRST            │
  │ # (collision detector), then mail, then br update.   │
  │                                                      │
  │ bin/br-claim <fix-bead-id>                           │
  │                                                      │
  │ # Files are parsed from the bead's                   │
  │ # `### Files to Create/Modify` section. If absent,   │
  │ # pass them explicitly:                              │
  │ # bin/br-claim <fix-bead-id> \                       │
  │ #   --paths crates/daemon/src/db/sync.rs \           │
  │ #           crates/daemon/tests/sqlite_supabase_sync.rs │
  │ #                                                    │
  │ # If bin/br-claim exits non-zero, the bead is        │
  │ # NOT claimed. Do not start coding. Read the error,  │
  │ # fix the underlying problem (other agent claimed,   │
  │ # file reservation conflict, agent-mail server       │
  │ # down, AGENT_MAIL_NAME unset). Then retry.          │
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
  │ # bin/br-complete REFUSES to run if HEAD ≠ upstream. │
  └──────────────────────────────────────────────────────┘
           │
  ┌─ COMPLETE + RELEASE (atomic via wrapper) ───────────┐
  │ # AGENTS.md Rule 2: ONLY use bin/br-complete.        │
  │ # NEVER raw `br close <id>` — the wrapper sends      │
  │ # `[<id>] Complete` mail, runs br close, syncs        │
  │ # `.beads/`, pushes, then releases reservations.     │
  │                                                      │
  │ bin/br-complete <fix-bead-id>                        │
  │                                                      │
  │ # Wrapper steps (in order):                          │
  │ #  1. verifies br status == in_progress              │
  │ #  2. confirms YOU sent the [<id>] Start mail        │
  │ #  3. finds the <fix-bead-id>: commit and verifies   │
  │ #     HEAD == upstream (i.e. you actually pushed)    │
  │ #  4. sends [<fix-bead-id>] Complete: <title> mail   │
  │ #  5. br close <fix-bead-id> --reason "Completed"    │
  │ #  6. bin/br-sync && git add .beads/ && commit       │
  │ #     && pull --rebase && push                       │
  │ #  7. release_file_reservations(agent_name=<self>)   │
  │ #                                                    │
  │ # If your commit subject doesn't start with the      │
  │ # bead id, pass --commit <sha> explicitly.           │
  │ # If a step before #7 fails, reservations are NOT    │
  │ # released — fix the underlying issue and re-run.    │
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

**The wrappers do most of this for you.** `bin/br-claim` sends Start + reserves files; `bin/br-complete` sends Complete + releases reservations. Use raw mail tools only for between-bead inbox checks and BLOCKED escalations.

```
**CRITICAL:** Agent Mail has NO broadcast. `"broadcast"` is NOT a valid recipient. Discover peers via `list_contacts(project_key, agent_name)` after registration; store as `<peers>`.

# Claim a fix bead (atomic — reservations + Start mail + br update)
bin/br-claim <fix-bead-id>

# Complete a fix bead (atomic — Complete mail + br close + .beads/ sync + release)
bin/br-complete <fix-bead-id>          # after `git commit -m "<id>: ..." && git push`

# Discover peers (after registration; refresh between beads)
list_contacts(project_key="/Users/alinaaitova/Desktop/Testproject1", agent_name=<your-name>)

# Check inbox (between fix beads — wrappers do NOT do this for you)
fetch_inbox(project_key="/Users/alinaaitova/Desktop/Testproject1", agent_name=<your-name>, limit=10)

# Report blocker (when blocker is in another agent's code)
send_message(project_key="/Users/alinaaitova/Desktop/Testproject1", sender_name=<your-name>, to=["<peers>"],
  thread_id="<fix-bead-id>", subject="[<fix-bead-id>] BLOCKED: <issue>",
  body_md="File: <path>, Line: <N>. Issue: <desc>. Needed: <what>")

# Manual fallback (ONLY if a wrapper is broken AND you've diagnosed the root cause):
#   - Raw `br update --status=in_progress` / `br close` violate AGENTS.md Rule 2.
#   - Fix the wrapper or its dependency (agent-mail server, AGENT_MAIL_NAME env)
#     instead of bypassing.
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
4. **ALWAYS use `bin/br-claim` and `bin/br-complete` (AGENTS.md Rule 2).** Raw `br update --status=in_progress` and `br close <id>` are forbidden — they bypass Agent Mail and the file-reservation collision detector.
5. NEVER stash other agents' work
6. Use `br` (NOT `bd`); use `bin/br-sync` (NOT raw `br sync`); the wrappers handle sync inside `bin/br-complete`
7. Stage Validation Test Gate from AGENTS.md is the minimum gate — iterate every `crates/daemon/tests/*.rs`
8. `packages/ui` lint invariant: no imports of `@tauri-apps/api`/`apps/desktop`/`crates/daemon`/`supabase`/`transport`
9. Web is read-only — viewer-scoped credentials only — security boundary, not UI convention
10. Do NOT ask what to work on — `br ready --label layer-4 --label audit-fix` answers
11. Work is NOT done until `git push` succeeds AND `bin/br-complete` succeeds
12. After ALL fix beads done, re-run completion audit (step 12) to verify COMPLETE verdict
