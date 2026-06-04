# Goods-Led Install/Maintenance — Fix Beads Execution Kickoff

> For fresh agents executing fix beads from the completion audit. Read this FIRST.

---

## Mission

You are executing fix beads for the goods-led install/maintenance feature — gaps the completion audit found between the spec and the actual implementation. Each fix bead is a targeted correction: a missing field, a wrong assertion, an unimplemented edge case, a missing test row.

Work from the fix bead graph. Each fix bead is self-contained — implement from the bead alone. The spec and TDD are tiebreakers if a fix bead is ambiguous.

**Organization: FLAT.** No hierarchy. Pick the next ready fix bead, implement it fully, move on.
**Coordination: AGENTMAIL-FIRST.** Register, check inbox, announce claims BEFORE touching code.

---

## Implementation Boundary

**EXISTS (already implemented — the main feature):** the `uk-installation-labour-estimate` skill + reference tables; labour/permit fields in the rollup at both sites; the two-phase qualification (Phase-1 branch + Phase-2 dominance gate); the `goods_led_mixed` service-gate split; `MaintenancePeriodEstimate`; the gold-set + canary corpora; the schema-epoch bump.

**FIX BEADS ADDRESS:** gaps, missing edge cases, incorrect behaviour, thin test coverage, spec requirements not fully met. Targeted surgical fixes, not new features.

---

## Core Contracts / Non-Negotiables

- Money = integer pence; ratios/confidence = bps (10000=100%); saturating arithmetic.
- Estimates screening-grade ±20%, never firm quotes; fail-soft (`blocked` → `review`, no crash).
- Labour summed at BOTH `main.rs:8114` + `:8548`; columns nullable/default-0; labour only when an `InstallationRequirement` exists (Scenario 8 stays labour=0).
- Labour skill is `http_public` — no CDP membership.
- Near-floor value-share → `review`, never auto-reject; prefer contract-value basis when available.
- "If code conflicts with this contract, the contract wins."

---

## Source of Truth Loading (MANDATORY — Read Before Starting)

1. **AGENTS.md** — agent rules + coordination (Rule 2 constitutional; `br` block lines 279-330).
2. **Spec** — `docs/pipeline/goods-led-install-maintenance/artifacts/goods-led-install-maintenance-spec.md` (SOT #1).
3. **TDD** — `docs/pipeline/goods-led-install-maintenance/artifacts/goods-led-install-maintenance-tdd.md` (SOT #2).
4. **Completion audit report** — `docs/pipeline/goods-led-install-maintenance/artifacts/goods-led-install-maintenance-completion-audit-YYYY-MM-DD.md` (why each fix bead exists).
5. **Fix bead graph** — your ACTUAL work orders:
   ```bash
   br list --label audit-fix --json     # all fix beads (also tagged goods-led-install)
   br ready --label audit-fix --json    # ready fix beads
   ```
6. **Integration surfaces** — the key files from `10-generate-kickoff.md` §9 File-Surface Map.

---

## AgentMail Setup (Do This ONCE At Session Start)
```
ensure_project(project_key="/Users/alinaaitova/Desktop/Testproject1")
register_agent(project_key, program="claude-code", model="opus")   # export AGENT_MAIL_NAME
set_contact_policy(project_key, agent_name, policy="open")
# Bootstrap contact handshakes BEFORE any bin/br-claim:
#   list_agents → same-program live peers → macro_contact_handshake(...) → verify in list_contacts
fetch_inbox(project_key, agent_name, limit=20)
```
Read and acknowledge pending messages before starting. Address any prior-agent blocker notice first.

---

## Bead Completion Policy (CRITICAL)

Use the project wrappers — the ONLY authorized path (AGENTS.md Rule 2, constitutional):
- `bin/br-claim <fix-bead-id>` — claims atomically (file reservation FIRST, then Start mail to peers, then `br update --status=in_progress`).
- `bin/br-complete <fix-bead-id>` — completes atomically (verifies the commit is pushed, Complete mail, `br close`, `bin/br-sync`, releases reservations).

Do NOT use raw `br update`/`br close`. The Phase B precommit hook blocks any commit with a `<fix-bead-id>:` subject lacking a Start mail.

**You MUST fully complete every fix bead you claim.** A fix bead is COMPLETE when:
1. All code changes in the bead are implemented.
2. The specific TDD tests referenced pass.
3. The full AGENTS.md stage gate passes (zero clippy warnings).
4. Committed with `<fix-bead-id>:` prefix and pushed.
5. `bin/br-complete <fix-bead-id>` ran clean.

**If blocked:** diagnose, fix yourself. Escalate via AgentMail (reply in the fix bead's thread) only if the blocker is in another agent's code. Do NOT release reservations or close the bead while blocked.

---

## Work Loop

```
ONE-TIME SETUP:
  1. AgentMail registration (export AGENT_MAIL_NAME)
  2. Read AGENTS.md + spec + TDD + completion-audit report
  3. br ready --label audit-fix --json → understand the fix graph

MAIN LOOP (repeat until all fix beads complete):

  ┌─ PICK ──────────────────────────────────────────────┐
  │ br ready --label audit-fix --json                    │
  │ Pick highest-priority ready fix bead. Read it FULLY. │
  └──────────────────────────────────────────────────────┘
           │
  ┌─ CLAIM (atomic) ────────────────────────────────────┐
  │ bin/br-claim <fix-bead-id>                           │
  │ # reservation FIRST → Start mail → status=in_progress│
  │ # FILE_RESERVATION_CONFLICT → pick a different bead  │
  │ # --paths <files> if the bead lacks a Files section  │
  └──────────────────────────────────────────────────────┘
           │
  ┌─ IMPLEMENT ─────────────────────────────────────────┐
  │ Study target files → read the spec section + TDD     │
  │ test IDs in the bead → fix/write tests FIRST →       │
  │ implement → verify the bead's acceptance criteria.   │
  │ NO corners. NO skipped tests. NO TODOs. NO mid-way   │
  │ commits.                                             │
  └──────────────────────────────────────────────────────┘
           │
  ┌─ VERIFY (full AGENTS.md stage gate) ────────────────┐
  │ cargo fmt --check                                    │
  │ cargo check --all-targets                            │
  │ cargo clippy --all-targets -- -D warnings            │
  │ cargo test -p daemon --bin daemon                    │
  │ for t in crates/daemon/tests/*.rs; do \              │
  │   cargo test --test "$(basename "$t" .rs)" || exit 1; done │
  │ bun test                                             │
  └──────────────────────────────────────────────────────┘
           │
  ┌─ COMMIT + PUSH (must precede br-complete) ──────────┐
  │ git add <specific-files-only>                        │
  │ git commit -m "<fix-bead-id>: <short description>"   │
  │ git pull --rebase && git push                        │
  └──────────────────────────────────────────────────────┘
           │
  ┌─ COMPLETE (atomic) ─────────────────────────────────┐
  │ bin/br-complete <fix-bead-id>                        │
  └──────────────────────────────────────────────────────┘
           │
        LOOP → back to PICK
```
"Use debug builds for fast iteration."

---

## Verification Commands

The FULL AGENTS.md stage gate (above) — a passing `cargo test -p daemon --bin daemon` alone is NOT green. Every `crates/daemon/tests/*.rs` integration target (including `installation_labour_engine.rs`) must run, plus `bun test`.

---

## Completion Gate

When ALL fix beads are complete: run the full stage gate one final time; verify build green with zero warnings. If any gaps remain, go back to the completion audit (step 12) and re-audit. Do NOT stop to ask what's next — pick the next ready fix bead. If fully blocked on all remaining: push all work, release all reservations, announce handoff via AgentMail with done/blocked status.

---

## AgentMail Quick Reference

NO broadcast — `"broadcast"` is not a valid recipient. Discover peers via `list_contacts(project_key, agent_name)`; refresh between beads. The wrappers send Start/Complete mail; use `send_message` directly only for blockers/questions/hand-offs (always `thread_id="<fix-bead-id>"`).
