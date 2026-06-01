# L6 Request Pipeline Automation — Step 13: Fix Beads Execution Kickoff

> **For fresh agents executing fix beads from the completion audit.** Read this FIRST
> before claiming any `l6-audit-fix` bead.
>
> If step 12 verdict = COMPLETE (zero fix beads), SKIP this step and go to step 15.

---

## Mission

You are executing **fix beads** for L6 (Request Pipeline Automation) — gaps identified
by the completion audit between the spec and the actual implementation. Each fix bead
is a targeted correction: a missing field, a wrong assertion, an unimplemented edge
case, an incomplete invariant enforcement, etc.

Work from the fix bead graph. Each fix bead is self-contained — implement from the
bead alone. The spec and TDD are tiebreakers if a fix bead is ambiguous.

**Organization: FLAT.** No hierarchy. Pick the next ready fix bead, implement it
fully, move on.

**Coordination: AGENTMAIL-FIRST.** Register, check inbox, announce claims BEFORE
touching code.

---

## Implementation Boundary

**EXISTS (the main L6 implementation — fix beads correct gaps in this):**

- L6 implementation epic with all task beads closed (per step 12 verdict)
- All 7 strategy phases delivered:
  - Phase 1: ErrorCode + types
  - Phase 2: Daemon runtime (`run_bid_workflow` dispatch + `trigger_bid_workflow` IPC handler + CLI)
  - Phase 3: State machine + readiness gate
  - Phase 4: `skills/bid-portal-procontract/`
  - Phase 5: `skills/validation/overlay-bid-portal-procontract/` + `packages/bidding/`
  - Phase 6: Desktop + Web UI extensions
  - Phase 7: Live demo (first run REMOTE_DRAFT → draft_saved; second run FORMAL_SUBMISSION → bid_submitted)
- All 3 spec TS test files + Rust integration test extensions
- PII redaction extended for BidScopePayload + CompanyProfile fields
- Validator overlay assertions
- Calibration corpus at `tests/fixtures/calibration/bid-portal-procontract/`

**FIX BEADS ADDRESS:** Gaps, missing edge cases, incorrect behavior, thin test
coverage, spec requirements not fully met. These are targeted surgical fixes, not
new features.

---

## Core Contracts / Non-Negotiables (same as main kickoff)

8 constitutional invariants from spec §1.10 (verbatim):
1. No real-money bids without human approval
2. No externally visible portal mutation beyond approved side-effect class
3. Bid workflow carries explicit scope
4. Full audit trail for submissions
5. Stale decisions block bid
6. L5 readiness gate
7. Validation fail-closed for portal submission
8. Quote-role quality gate

5 recorded decisions (from manifest):
1. Proactis === Procontract (naming)
2. Reuse `skills/auth-signin/procontract/`
3. Validator overlay is L6 deliverable
4. Sister implementation epic structure
5. Procontract access already solved (live demo feasible)

"If code conflicts with this contract, the contract wins."

---

## Source of Truth Loading (MANDATORY — Read Before Starting)

1. **AGENTS.md** — agent guidelines, Rule 2 (constitutional), authoritative `br` block (lines 279-330)
   ```text
   /Users/alinaaitova/Desktop/Testproject1/AGENTS.md
   ```
2. **Spec** — SOT #1 — the authority for WHAT the feature should do
   ```text
   /Users/alinaaitova/Desktop/Testproject1/docs/strategy/layers/layer-6-request-pipeline-automation.md
   ```
3. **TDD** — SOT #2 — test definitions for verification
   ```text
   /Users/alinaaitova/Desktop/Testproject1/docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-tdd.md
   ```
4. **Completion Audit** — the gap analysis you're fixing
   ```text
   /Users/alinaaitova/Desktop/Testproject1/docs/pipeline/l6-request-pipeline-automation/artifacts/l6-completion-audit-YYYY-MM-DD.md
   ```
5. **Fix bead graph** — your ACTUAL work orders
   ```bash
   br list --label l6-audit-fix --json    # All fix beads
   br ready --label l6-audit-fix --json   # Ready fix beads
   ```
6. **Original L6 strategy** (for phase context)
   ```text
   /Users/alinaaitova/Desktop/Testproject1/docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-strategy.md
   ```
7. **Integration surfaces** — study for context on each fix
   ```text
   # Whatever files the specific fix bead targets — referenced in the fix bead's Files section
   ```

---

## AgentMail Setup (Do This ONCE At Session Start)

```
# 1. Register with the project
ensure_project(project_key="/Users/alinaaitova/Desktop/Testproject1")
register_agent(project_key, program="claude-code", model="opus")

# 2. Set contact policy
set_contact_policy(project_key, agent_name, policy="open")

# 3. Bootstrap Contact Handshakes (MANDATORY before any bin/br-claim)
peers = list_agents(project_key)
for peer in peers:
    if peer['program'] == 'claude-code' and peer['active_in_last_hour'] and peer['name'] != agent_name:
        macro_contact_handshake(project_key, requester=agent_name, target=peer['name'])

# 4. Verify handshake landed
list_contacts(project_key, agent_name)  # Should contain peers you handshook

# 5. Check inbox for messages from prior sessions or other agents
fetch_inbox(project_key, agent_name, limit=20)
```

Read and acknowledge any pending messages before starting work. If another agent left
a blocker notice for a fix bead, address it first.

---

## Bead Completion Policy (CRITICAL)

**Use the project wrappers — they are the only authorized path:**

- `bin/br-claim <fix-bead-id>` — claims atomically (file reservation FIRST, then Start mail to peers, then `br update --status=in_progress`)
- `bin/br-complete <fix-bead-id>` — completes atomically (verifies commit on remote, sends Complete mail, closes bead, syncs `.beads/`, releases reservations)

Do NOT call raw `br update --status=in_progress` or `br close` — they bypass mail
and reservations. AGENTS.md Rule 2 (constitutional, lines 21-34) forbids it; the
Phase B precommit hook blocks any commit with a `<fix-bead-id>:` subject that has
no Start mail in the archive.

**You MUST fully complete every fix bead you claim.** No partial completion. No "I'll
finish this later." No moving on to another fix bead while one is in progress.

A fix bead is COMPLETE when:
1. All code changes described in the fix bead are implemented
2. The specific L6-T-* tests referenced in the bead pass
3. All build/lint/test verification commands pass (zero warnings on `cargo clippy --all-targets -- -D warnings`)
4. Code is committed with `<fix-bead-id>:` subject prefix and pushed to the remote
5. `bin/br-complete <fix-bead-id>` has run successfully (mail + close + sync + release)

**If you hit a blocker:**
- DO NOT abandon the fix bead and move to another
- DO NOT mark it complete with missing pieces
- DO NOT do mid-way commits of incomplete work
- Diagnose the blocker. Fix it yourself.
- Only escalate via AgentMail (reply in the fix bead's thread) if the blocker is in another agent's code (file path, line number, what's wrong). Do NOT release reservations or close the bead while blocked.

---

## Work Loop

```
ONE-TIME SETUP:
  1. AgentMail registration (see above; export AGENT_MAIL_NAME)
  2. Read AGENTS.md + spec + TDD + completion audit
  3. br ready --label l6-audit-fix → understand the fix graph

MAIN LOOP (repeat until all fix beads complete):

  ┌─ PICK ──────────────────────────────────────────────┐
  │ br ready --label l6-audit-fix --json                 │
  │ Pick the highest-priority ready fix bead.            │
  │ Read the fix bead description FULLY.                 │
  │ Read the completion audit section that created it.   │
  └──────────────────────────────────────────────────────┘
           │
  ┌─ CLAIM (atomic via wrapper) ────────────────────────┐
  │ bin/br-claim <fix-bead-id>                           │
  │ # Wrapper does, in order:                            │
  │ #   1. duplicate-claim guard (status + Start search) │
  │ #   2. file reservation (collision detector — FIRST) │
  │ #   3. [<fix-bead-id>] Start mail to peers           │
  │ #   4. br update --status=in_progress                │
  │ # If FILE_RESERVATION_CONFLICT: pick a different bead│
  │ # Override file list with --paths if needed.         │
  └──────────────────────────────────────────────────────┘
           │
  ┌─ IMPLEMENT ─────────────────────────────────────────┐
  │ 1. Study the target files — understand current state │
  │ 2. Read the spec section referenced in the fix bead  │
  │ 3. Write/fix tests FIRST (from L6-T-* test IDs)      │
  │ 4. Implement the fix to make tests pass              │
  │ 5. Verify acceptance criteria from the fix bead      │
  │                                                      │
  │ DO NOT cut corners. DO NOT skip tests.               │
  │ DO NOT leave TODOs. DO NOT do mid-way commits.       │
  │                                                      │
  │ CDP-related fixes: invoke bin/ensure-cdp-9222 before │
  │ any portal navigation. NEVER launch a fresh browser. │
  │ ATTACH to operator's persistent profile at           │
  │ /Users/alinaaitova/.chrome-automation.               │
  └──────────────────────────────────────────────────────┘
           │
  ┌─ VERIFY ────────────────────────────────────────────┐
  │ cargo fmt --check                                    │
  │ cargo check --all-targets                            │
  │ cargo clippy --all-targets -- -D warnings            │
  │ cargo test --test <relevant-test>                    │
  │ bun run build                                        │
  │ bun run lint                                         │
  │ bun run test                                         │
  │ # Phase 6 fixes also need E2E:                       │
  │ (cd apps/desktop && bun run test:e2e)                │
  │ (cd apps/web && bun run test:e2e)                    │
  │                                                      │
  │ Fix ALL failures before proceeding.                  │
  │ Never proceed with failing checks.                   │
  └──────────────────────────────────────────────────────┘
           │
  ┌─ COMMIT + PUSH (must precede br-complete) ──────────┐
  │ git add <specific-files-only>                        │
  │ git commit -m "<fix-bead-id>: <short description>"   │
  │ git pull --rebase && git push                        │
  │ # br-complete refuses to run until HEAD == @{u}.     │
  │ One commit per fix bead.                             │
  └──────────────────────────────────────────────────────┘
           │
  ┌─ COMPLETE (atomic via wrapper) ─────────────────────┐
  │ bin/br-complete <fix-bead-id>                        │
  │ # Wrapper does, in order:                            │
  │ #   1. verify status=in_progress + Start mail by you │
  │ #   2. resolve commit SHA + verify pushed            │
  │ #   3. [<fix-bead-id>] Complete mail to peers        │
  │ #   4. br close <fix-bead-id>                        │
  │ #   5. bin/br-sync + git add .beads/ + commit + push │
  │ #   6. release_file_reservations(agent_name=self)    │
  └──────────────────────────────────────────────────────┘
           │
        LOOP → back to PICK
```

"Use debug builds for fast iteration."

---

## Verification Commands

```bash
# Rust (daemon)
cargo fmt --check
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test -p daemon --bin daemon
for t in crates/daemon/tests/*.rs; do cargo test --test "$(basename "$t" .rs)" || exit 1; done

# TypeScript (packages + apps)
bun run build
bun run lint
bun run test

# E2E (only if fix bead touches Phase 6 UI)
(cd apps/desktop && bun run test:e2e)
(cd apps/web && bun run test:e2e)

# UBS (on staged files)
ubs $(git diff --name-only --cached)
```

---

## CDP Browser Discipline (for any fix bead that touches Procontract)

ALL Procontract skills attach to operator's persistent Chrome at `/Users/alinaaitova/.chrome-automation` on port 9222.

```bash
/Users/alinaaitova/Desktop/Testproject1/bin/ensure-cdp-9222
curl -sfS -m 3 "http://localhost:9222/json/version"
```

**MUST NOT:**
- Launch a fresh browser profile
- Attempt portal login outside auth-signin/procontract skill
- Solve captchas
- Log out
- Close the operator's browser

**Sensitivity:** `sales@sorelon.com` is the operator's REAL Procontract account.
Fix beads MUST NOT mutate anything outside their declared scope.

---

## Live Demo Constraints (if a fix bead re-runs the live demo)

Per decision_5:
- **First live demo:** sideEffectCap = REMOTE_DRAFT. Reaches `draft_saved`.
- **Operator-authorized second live demo:** sideEffectCap allows FORMAL_SUBMISSION. Reaches `bid_submitted`.

The second run requires EXPLICIT operator authorization (separate operator interaction).
DO NOT escalate to FORMAL_SUBMISSION unless the fix bead's AC explicitly says so AND
operator has confirmed via desktop UI.

---

## Completion Gate

When ALL fix beads are complete:

1. Run the full quality gate sequence one final time:
   ```bash
   cargo fmt --check
   cargo check --all-targets
   cargo clippy --all-targets -- -D warnings
   cargo test -p daemon --bin daemon
   for t in crates/daemon/tests/*.rs; do cargo test --test "$(basename "$t" .rs)" || exit 1; done
   bun run build
   bun run lint
   bun run test
   (cd apps/desktop && bun run test:e2e)
   (cd apps/web && bun run test:e2e)
   ```
2. Verify all fix beads closed: `br list --label l6-audit-fix --status open --json` returns 0
3. Verify no warnings in any quality gate output
4. If any gaps remain, re-trigger step 12 completion audit (audit_iterations += 1)
5. Do NOT stop to ask what to work on next — pick the next ready fix bead
6. Do NOT wait for permission between fix beads

If fully blocked on all remaining fix beads: push all work, release all reservations,
announce handoff via AgentMail with status of what's done and what's blocked.

When all fix beads green and step 12 re-audit verdict = COMPLETE → proceed to step 15 (archive).

---

## AgentMail Quick Reference

**CRITICAL:** Agent Mail has NO broadcast. `"broadcast"` is NOT a valid recipient.
Discover peers via `list_contacts(project_key, agent_name)` after registration;
store result as `<peers>`. Refresh between beads.

```
# Discover peers (after registration; refresh between beads)
list_contacts(project_key, agent_name)  # → e.g. ["BlueLake", "RedPeak"]
# Check inbox (between fix beads)
fetch_inbox(project_key, agent_name, limit=10)
# Claim/Complete — use wrappers (handles mail + reservations atomically):
bin/br-claim <fix-bead-id>
bin/br-complete <fix-bead-id>
# Ad-hoc — report blocker (NOT for Start/Complete; wrappers send those):
send_message(project_key, sender_name=<your-name>, to=["<peers>"],
  thread_id="<fix-bead-id>", subject="[<fix-bead-id>] BLOCKED: <issue>",
  body_md="File: <path>, Line: <N>. Issue: <desc>. Needed: <what>")
```

---

## Key Reminders

- Fix beads are SURGICAL — implement exactly what the bead says, no more
- ALL completion audits + live demo gates from main kickoff still apply
- ZERO SCOPE DRIFT — no fix bead may defer to "post-fix-cycle"
- 8 constitutional invariants + 5 recorded decisions are NON-NEGOTIABLE
- Use `bin/br-claim` / `bin/br-complete` (NEVER raw `br update`/`br close`)
- Use `bin/br-sync` (NEVER raw `br sync`)
- CDP discipline applies — never fresh browser, never log out, never mutate outside scope
- Live demo fixes require operator gating per decision_5
- After all fix beads close → re-trigger step 12 audit (loop) → only then proceed to step 15
