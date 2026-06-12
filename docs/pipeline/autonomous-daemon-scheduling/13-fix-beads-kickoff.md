# Autonomous Daemon Scheduling — Fix Beads Execution Kickoff

> For fresh agents executing fix beads from the completion audit (step 12). Read this FIRST.

---

## Mission

Execute fix beads for autonomous daemon scheduling — gaps identified by the completion audit between the spec and the actual `crates/daemon/` implementation. Each fix bead is a targeted correction: a missing worker match arm, a hollow handler, a wrong assertion, an unimplemented lifecycle transition, a missed epoch-constant bump, etc.

Work from the fix bead graph. Each fix bead is self-contained — implement from the bead alone. The spec and TDD are tiebreakers if ambiguous.

**Organization: FLAT.** No hierarchy. Pick the next ready fix bead, implement it fully, move on.
**Coordination: AGENTMAIL-FIRST.** Register, check inbox, announce claims BEFORE touching code.

---

## Implementation Boundary

**EXISTS (already implemented — the main feature):** the recurring scheduler tick, the two-lane worker, the new cron-kind handlers, the queue-driven shared source bodies, the `--run-scheduler-once` entrypoint, the commercial_sweep, the ProContract awaiting-access tracking + work-list sweep. These were built in steps 10–11.

**FIX BEADS ADDRESS:** gaps, hollow handlers, missing lifecycle transitions, thin test coverage, spec requirements not fully met, missed epoch-constant updates. Targeted surgical fixes, not new features.

---

## Core Contracts / Non-Negotiables (the contract wins over code)
- Tick and worker on SEPARATE threads; tick body takes injected `now` (no inline `Utc::now()`).
- Two lanes by job class; defer-on-denial (no attempt_count bump, no lease, backoff).
- `procurement_source_lanes=1` serializes same-source runs (R7).
- §4.4 landmine: NEVER overwrite `interest_registration.state` (stays `'completed'`).
- `commercial_sweep` in-process on the coverage lane — NEVER `trigger_pipeline_rerun`.
- Schema-epoch coupling: all constants move together (10→11 if Q4=table).
- `#![forbid(unsafe_code)]`; no backwards-compat shims; no new files unless genuinely new.

---

## Source of Truth Loading (MANDATORY — before starting)
1. **AGENTS.md** — Rule 2 (wrappers), Stage Validation Test Gate
2. **Spec** — `docs/pipeline/autonomous-daemon-scheduling/artifacts/autonomous-daemon-scheduling-spec.md`
3. **TDD** — `docs/pipeline/autonomous-daemon-scheduling/artifacts/autonomous-daemon-scheduling-tdd.md`
4. **Fix bead graph:**
   ```bash
   br list --label audit-fix --json     # fix beads (also carry label autonomous-scheduling)
   br ready --label audit-fix --json    # ready fix beads
   ```
5. **Integration surfaces** — manifest `key_files` (verified file:lines)

---

## AgentMail Setup (Do This ONCE At Session Start)
```
ensure_project(project_key="/Users/alinaaitova/Desktop/Testproject1")
register_agent(project_key, program="claude-code", model="opus")   # export AGENT_MAIL_NAME
set_contact_policy(project_key, agent_name, policy="open")
# Bootstrap contact handshakes before any bin/br-claim:
#   list_agents → macro_contact_handshake(project_key, requester=agent_name, target=peer["name"])
#   verify via list_contacts(project_key, agent_name)
fetch_inbox(project_key, agent_name, limit=20)
```
Read and acknowledge pending messages before starting. Address any blocker notice first.

---

## Bead Completion Policy (CRITICAL)
Use the wrappers ONLY: `bin/br-claim <fix-bead-id>` / `bin/br-complete <fix-bead-id>`. NEVER raw `br update`/`br close` (AGENTS.md Rule 2 constitutional; precommit hook enforces).

**Fully complete every fix bead you claim.** No partial completion, no "finish later," no mid-way commits.

A fix bead is COMPLETE when:
1. All code changes in the bead are implemented
2. The specific TDD tests referenced pass
3. The FULL Stage Validation Test Gate passes (see below) — zero warnings
4. Committed with `<fix-bead-id>:` prefix AND pushed
5. `bin/br-complete <fix-bead-id>` ran clean (mail + close + sync + release)

On blocker: diagnose + fix yourself. Only escalate via AgentMail (reply in the fix bead's thread, file:line) if the blocker is in another agent's code. Do NOT release reservations or close while blocked.

---

## Work Loop
```
ONE-TIME: AgentMail register (export AGENT_MAIL_NAME) → read AGENTS.md + spec + TDD → br ready --label audit-fix

MAIN LOOP:
  PICK     br ready --label audit-fix → highest-priority ready fix bead; read it FULLY
  CLAIM    bin/br-claim <fix-bead-id>   (reservation FIRST, then Start mail, then in_progress)
           # FILE_RESERVATION_CONFLICT → pick a different bead; override with --paths if no Files section
  IMPLEMENT  study target files → read the spec section in the bead → fix/write tests FIRST → implement → verify ACs
  VERIFY     run the FULL gate (below); fix ALL failures
  COMMIT     git add <specific files> → git commit -m "<fix-bead-id>: <desc>" → git pull --rebase && git push
  COMPLETE   bin/br-complete <fix-bead-id>   (verifies pushed, Complete mail, close, br-sync, release)
  LOOP → PICK
```
"Use debug builds for fast iteration."

---

## Verification Commands (the FULL Stage Validation Test Gate)
```bash
cargo fmt --check
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test -p daemon --bin daemon
for t in crates/daemon/tests/*.rs; do cargo test --test "$(basename "$t" .rs)" || exit 1; done
# (only if Q7 desktop surfacing was taken:)  bun run build && bun run test && bun run lint
```
`--bin daemon` ALONE is NOT green — it skips the net-new scheduler_tick.rs / worker_pump.rs / procontract_resumption.rs. The `for` loop is mandatory.

---

## Completion Gate
When ALL fix beads are complete: run the full gate one final time. If gaps remain, go back to step 12 and re-audit. Do NOT stop to ask what's next — pick the next ready fix bead. If fully blocked: push all work, release reservations, announce handoff via AgentMail with done/blocked status.

---

## AgentMail Quick Reference
NO broadcast. `bin/br-claim`/`bin/br-complete` send Start/Complete mail + manage reservations. Discover peers via `list_contacts(project_key, agent_name)`; refresh between beads. Use `send_message` only for blockers/questions:
```
list_contacts(project_key, agent_name)   # → <peers>
fetch_inbox(project_key, agent_name, limit=10)
send_message(project_key, sender_name=<you>, to=["<peers>"], thread_id="<fix-bead-id>",
  subject="[<fix-bead-id>] BLOCKED: <issue>", body_md="File:<path> Line:<N>. Issue:<desc>. Needed:<what>")
release_file_reservations(project_key, agent_name)
```
