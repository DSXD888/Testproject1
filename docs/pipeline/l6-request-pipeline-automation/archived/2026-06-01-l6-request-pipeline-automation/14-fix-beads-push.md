# L6 Request Pipeline Automation — Step 14: Fix Beads Push (Continuation Nudge)

> For continuing agents executing L6 fix beads. Compact reminder of contracts, work
> loop, and verification. If you haven't read the fix-beads kickoff (step 13), START
> THERE.

---

## Re-Orient

1. Re-read `AGENTS.md` — agent rules and coordination (especially Rule 2 + authoritative `br` block lines 279-330)
2. Feature: **L6 Request Pipeline Automation** — executing fix beads from completion audit
3. Spec (tiebreaker if fix bead ambiguous): `docs/strategy/layers/layer-6-request-pipeline-automation.md` (SOT #1)
4. TDD (test definitions): `docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-tdd.md` (SOT #2)
5. Completion audit (gap analysis): `docs/pipeline/l6-request-pipeline-automation/artifacts/l6-completion-audit-YYYY-MM-DD.md`
6. Fix bead graph: `br ready --label l6-audit-fix --json`

---

## Core Contracts (compact)

8 invariants from spec §1.10:
1. No real-money bids without human approval
2. No mutation beyond approved side-effect class
3. Bid workflow carries explicit scope
4. Full audit trail
5. Stale decisions block bid
6. L5 readiness gate
7. Validation fail-closed for portal submission
8. Quote-role quality gate

5 recorded decisions:
1. Proactis === Procontract (naming)
2. Reuse `skills/auth-signin/procontract/`
3. Validator overlay is L6 deliverable
4. Sister implementation epic
5. Procontract access already solved

---

## Bead Completion Policy

**Every fix bead you claim MUST be fully completed.** No partial work. No TODOs. No "finish later."

Complete means: all code + all tests + all checks + committed AND pushed + `bin/br-complete` ran clean.

**Use `bin/br-claim <id>` and `bin/br-complete <id>` — never raw `br update`/`br close`.**
AGENTS.md Rule 2 (constitutional, lines 21-34) makes the wrappers the only authorized path.

**If blocked:** Diagnose it. Fix it yourself. Only escalate via AgentMail (reply in the
fix bead's thread) if the blocker is in another agent's code. Do NOT abandon the bead.
Do NOT release reservations while blocked.

---

## Work Loop

```
1. Check inbox (fetch_inbox) → acknowledge any messages
2. br ready --label l6-audit-fix --json → pick highest-priority ready fix bead
3. Read fix bead FULLY → study target files → read referenced spec section
4. CLAIM:
   bin/br-claim <fix-bead-id>
   # Wrapper handles file_reservation + Start mail + br update atomically.
   # Override file list with --paths if needed.
5. IMPLEMENT: Fix per AC bullets. Tests first, then code. Implement ALL of it. No mid-way commits.
6. VERIFY:
   cargo fmt --check && cargo check --all-targets && cargo clippy --all-targets -- -D warnings
   cargo test --test <relevant-test>
   bun run build && bun run lint && bun run test
   # Phase 6 fixes:
   (cd apps/desktop && bun run test:e2e)
   (cd apps/web && bun run test:e2e)
   # ALL must pass
7. COMMIT + PUSH:
   git add <specific-files>
   git commit -m "<fix-bead-id>: <short description>"
   git pull --rebase && git push
   # br-complete refuses until HEAD == @{u}.
8. COMPLETE:
   bin/br-complete <fix-bead-id>
   # Wrapper verifies pushed + Complete mail + close + sync + release reservations.
9. LOOP → back to step 1

Do not stop to ask what to work on next. Pick the next ready fix bead.
```

---

## Verification (Every Fix Bead)

Use debug builds for fast iteration. Fix ALL failures before committing.

```bash
# Rust
cargo fmt --check
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test --test <relevant-test>

# TypeScript
bun run build
bun run lint
bun run test

# E2E (only if Phase 6 fix)
(cd apps/desktop && bun run test:e2e)
(cd apps/web && bun run test:e2e)

# UBS
ubs $(git diff --name-only --cached)
```

---

## CDP Browser Discipline (Phase 4/7 fixes)

ALL Procontract skills attach to operator's persistent Chrome at `/Users/alinaaitova/.chrome-automation:9222`.

```bash
/Users/alinaaitova/Desktop/Testproject1/bin/ensure-cdp-9222
curl -sfS -m 3 "http://localhost:9222/json/version"
```

**MUST NOT:** launch fresh browser, log in outside auth-signin skill, solve captchas, log out, close operator's browser.

---

## Live Demo Constraints (if fix bead re-runs demo)

Per decision_5:
- First run: sideEffectCap = REMOTE_DRAFT → draft_saved
- Second run: sideEffectCap allows FORMAL_SUBMISSION → bid_submitted, requires EXPLICIT operator authorization

---

## Completion Gate

When all fix beads are done:
- Run the full test suite one final time
- If gaps remain, go back to completion audit (step 12) and re-audit (audit_iterations += 1)
- If fully blocked: push work, release reservations, announce via AgentMail

When all green + step 12 re-audit verdict = COMPLETE → proceed to step 15 (archive).

---

## AgentMail Quick Reference

**CRITICAL:** Agent Mail has NO broadcast. Discover peers via `list_contacts(project_key, agent_name)` first; store as `<peers>`.

```
list_contacts(project_key, agent_name)  # → discover <peers>
fetch_inbox(project_key, agent_name, limit=10)

# Claim/Complete — wrappers handle Start/Complete mail:
bin/br-claim <fix-bead-id>
bin/br-complete <fix-bead-id>

# Ad-hoc only:
send_message(project_key, sender_name=<your-name>, to=["<peers>"],
  thread_id="<fix-bead-id>", subject="[<fix-bead-id>] BLOCKED: <issue>",
  body_md="File: <path>, Line: <N>. Issue: <desc>. Needed: <what>")
```

---

## Key Reminders (compact)

- Fix beads are SURGICAL — implement exactly what the bead says
- ZERO SCOPE DRIFT — no deferring to "later cycle"
- 8 invariants + 5 recorded decisions NON-NEGOTIABLE
- Use `bin/br-claim` / `bin/br-complete` / `bin/br-sync` (NEVER raw)
- CDP discipline — never fresh, never logout, never escape scope
- Live demo gated on operator action per decision_5
- After all fix beads close → re-audit (step 12) → only then archive (step 15)
