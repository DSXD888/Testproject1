# L4 Shared Visibility — 14 · Fix Beads Push (Continuation Nudge)

> For continuing agents working on L4 fix beads. Compact reminder of contracts, work loop, and verification.

---

## Re-Orient

1. Re-read `AGENTS.md` — agent rules, Stage Validation Test Gate, beads CLI invariants
2. Feature: L4 Shared Visibility — executing **fix beads** from completion audit
3. Spec: `docs/strategy/layers/layer-4-shared-visibility.md` — SOT #1 (tiebreaker if fix bead is ambiguous)
4. TDD: `docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-tdd.md` — SOT #2 (test definitions)
5. Completion audit report: `docs/pipeline/l4-shared-visibility/completion-audit-report-{date}.md`
6. Fix bead graph: `br ready --label layer-4 --label audit-fix --json`

**AgentMail registration (compact):**
```
macro_start_session(
  project_key="/Users/alinaaitova/Desktop/Testproject1",
  program="l4-fix-beads-executor",
  model="<your-model>",
  task_description="L4 Shared Visibility fix-bead execution"
)
```

---

## Core Contracts (compact, unchanged from main execution)

1. Push protocol transactional + atomic (single RPC, partial-apply forbidden)
2. Equal-version retry — all 4 cases
3. Pull-before-push at startup
4. CAS for operator decisions with command ledger
5. Writer fencing token (`writer_epoch`)
6. Schema epoch guard
7. Provisional promotion buffer for enforcement-active slices
8. Per-slice enforcement readiness (Wilson-bound, rule-of-three, all 7 prerequisites)
9. Web read-only is a SECURITY BOUNDARY (viewer-scoped credentials only)
10. `packages/ui` lint invariant: NO imports of `@tauri-apps/api`/`apps/desktop`/`crates/daemon`/`supabase`/`transport`
11. Single-edit chokepoints: `main.rs`, `runner.rs`, `policy.rs`, `executor.rs`, daemon `Cargo.toml`, root `package.json`, `packages/ui/freshness-badge.tsx`

If code conflicts with these contracts, the contract wins.

---

## Fix-Bead Completion Policy

**Every fix bead you claim MUST be fully completed.** No partial work. No TODOs. No "finish later."

Complete = all code + all tests + all checks + committed + pushed + announced + reservations released + bead closed + .beads/ pushed.

**If blocked:** Diagnose it. Fix it yourself. Only escalate via AgentMail if the blocker is in another agent's code. Do NOT abandon the fix bead.

---

## Work Loop (compact)

```
1. Check inbox: fetch_inbox(project_key="/Users/alinaaitova/Desktop/Testproject1", agent_name=<your-name>, limit=10) → acknowledge messages
2. br ready --label layer-4 --label audit-fix --json → pick highest-priority ready fix bead
3. Read fix bead FULLY → study target files referenced
4. CLAIM:
   br update <fix-bead-id> --status=in_progress
   file_reservation_paths(project_key="/Users/alinaaitova/Desktop/Testproject1", agent_name=<your-name>,
     paths=[<files>], ttl_seconds=3600, exclusive=true, reason="<fix-bead-id>")
   send_message(project_key="/Users/alinaaitova/Desktop/Testproject1", sender_name=<your-name>, to=["<peers>"],
     thread_id="<fix-bead-id>", subject="[<fix-bead-id>] Start: <title>")
   # <peers> from list_contacts — NEVER "broadcast"
5. IMPLEMENT: Read spec L4-FR + TDD L4-T tests referenced. Fix/write tests first, then code. Implement ALL of it. No mid-way commits. No partial work.
6. VERIFY: Stage Validation Test Gate (full)
7. COMMIT:
   ubs $(git diff --name-only --cached)
   git add <specific-files>
   git commit -m "<fix-bead-id>: <short description>"
   git pull --rebase && git push
8. COMPLETE:
   br close <fix-bead-id> --reason "Completed"
   bin/br-sync
   git add .beads/
   git commit -m "beads: close <fix-bead-id>"
   git pull --rebase && git push
   send_message(project_key="/Users/alinaaitova/Desktop/Testproject1", sender_name=<your-name>, to=["<peers>"],
     thread_id="<fix-bead-id>", subject="[<fix-bead-id>] Complete", body_md="Commit: <hash>")
   release_file_reservations(project_key="/Users/alinaaitova/Desktop/Testproject1", agent_name=<your-name>)
9. LOOP → step 1

Do not stop to ask what to work on next. Pick the next ready fix bead.
```

---

## Verification (Stage Validation Test Gate — full)

Use debug builds for fast iteration. Fix ALL failures before committing.

```bash
# Rust full gate (NEVER skip the test-file iteration)
cargo fmt --check
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test -p daemon --bin daemon
for t in crates/daemon/tests/*.rs; do
  cargo test --test "$(basename "$t" .rs)" || exit 1
done

# TS
bun run test
bun run lint
bun run build

# Web e2e (if your fix bead touches apps/web)
(cd apps/web && bun run test:e2e)

# Pre-commit
ubs $(git diff --name-only --cached)
```

---

## Completion Gate

When all fix beads are done:
- Run the FULL Stage Validation Test Gate one final time
- Re-run completion audit (step 12); must return COMPLETE
- If gaps remain, go back to step 12 and re-audit (loop)

If fully blocked: push all work, release reservations, announce via AgentMail with status of what's done + what's blocked.

---

## Common Fix Bead Categories (you'll see one of these)

- **Equal-version retry case missed** — add the missing case to `crates/daemon/src/db/sync.rs` push dispatcher; extend `crates/daemon/tests/sqlite_supabase_sync.rs`
- **RLS policy gap** — extend `infra/supabase/migrations/0004_rls_policies.sql`; extend `crates/daemon/tests/redaction_boundary.rs`
- **Daemon-pull tombstone visibility** — RLS policy must NOT filter `deleted_at IS NULL` for daemon role on remote-owned tables; test in `redaction_boundary.rs`
- **Validator overlay missing field** — extend `skills/validation/{slug}/SKILL.md` frontmatter; verify canary test passes
- **Mutation family missing** — extend `tests/fixtures/calibration/...` and the mutation library code; verify Wilson-bound test catches it
- **3-state FreshnessBadge prop** — extend `packages/ui/freshness-badge.tsx`; verify desktop + web both compile
- **CAS conflict IPC event** — extend `crates/daemon/src/ipc/handlers/decisions.rs`; verify e2e test catches event emission
- **Schema hash CI gate** — add `.github/workflows/schema-hash.yml` (or extend existing CI); verify it fails on intentional schema mismatch
- **Epoch-repair reproject minimum path** — extend `crates/daemon/src/db/sync.rs` push dispatcher; integration test that simulates epoch repair flow
- **Logical consistency auditor stub** — minimum stub at `crates/daemon/src/db/sync.rs` (or new module); domain invariant check + report (no auto-repair)

Each fix bead body specifies the exact category + file paths + test IDs + ACs.

---

## AgentMail Quick Reference (compact)

**CRITICAL:** Agent Mail has NO broadcast. `"broadcast"` is NOT a valid recipient. Discover peers via `list_contacts` first; store as `<peers>`.

```
list_contacts(project_key="/Users/alinaaitova/Desktop/Testproject1", agent_name=<your-name>)  # → discover <peers>
fetch_inbox(project_key="/Users/alinaaitova/Desktop/Testproject1", agent_name=<your-name>, limit=10)
send_message(project_key="/Users/alinaaitova/Desktop/Testproject1", sender_name=<your-name>, to=["<peers>"],
  thread_id="<fix-bead-id>", subject="[<fix-bead-id>] Start: <title>")
send_message(project_key="/Users/alinaaitova/Desktop/Testproject1", sender_name=<your-name>, to=["<peers>"],
  thread_id="<fix-bead-id>", subject="[<fix-bead-id>] Complete: <title>", body_md="Commit: <hash>. All pass.")
send_message(project_key="/Users/alinaaitova/Desktop/Testproject1", sender_name=<your-name>, to=["<peers>"],
  thread_id="<fix-bead-id>", subject="[<fix-bead-id>] BLOCKED: <issue>",
  body_md="File: <path>, Line: <N>. Issue: <desc>. Needed: <what>")
release_file_reservations(project_key="/Users/alinaaitova/Desktop/Testproject1", agent_name=<your-name>)
```

---

## Critical Rules (numbered, compact)

1. Fix beads = targeted corrections, NOT new features
2. Implement from fix bead body; spec is tiebreaker only
3. NEVER delete files (AGENTS.md Rule 1)
4. NEVER stash other agents' work
5. Use `br` (NOT `bd`); use `bin/br-sync` (NOT raw `br sync`)
6. Stage Validation Test Gate is minimum gate — iterate every `crates/daemon/tests/*.rs`
7. `packages/ui` lint invariant must hold
8. Web is read-only — security boundary, not UI convention
9. Do NOT ask what to work on — `br ready --label layer-4 --label audit-fix` answers
10. Work is NOT done until `git push` succeeds
11. After every fix bead: `bin/br-sync && git add .beads/ && git commit && git push`
12. After ALL fix beads done: re-run completion audit (step 12)

---

## Quick Reference

| | |
|---|---|
| Fix bead graph | `br ready --label layer-4 --label audit-fix --json` |
| Bead CLI | `br` |
| Bead sync | `bin/br-sync` |
| AgentMail key | `/Users/alinaaitova/Desktop/Testproject1` |
| Spec | `docs/strategy/layers/layer-4-shared-visibility.md` |
| TDD | `docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-tdd.md` |
| Audit report | `docs/pipeline/l4-shared-visibility/completion-audit-report-{date}.md` |
| Branch | `main` |
