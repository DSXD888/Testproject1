# Autonomous Daemon Scheduling — Step 11: Push (Continuation Nudge)

> This file is used DURING execution. The actual push prompt agents consume is generated at step 10
> (`artifacts/autonomous-daemon-scheduling-push.md`). This handoff explains WHEN and HOW to use it.

---

## When To Use the Push Prompt

Send the generated push prompt (`docs/pipeline/autonomous-daemon-scheduling/artifacts/autonomous-daemon-scheduling-push.md`) when:
- An agent's context is compacting and it needs a compact re-orientation, OR
- A fresh continuation agent joins the swarm mid-implementation, OR
- An agent finishes a bead and needs the work loop reinforced ("don't stop to ask what's next — pick the next ready bead").

The push is the COMPACT form of the kickoff. The kickoff is for cold start; the push is for keeping momentum.

---

## What the Push Must Reinforce (verify these are in the generated push)

1. **Re-read AGENTS.md** — Rule 2 (wrappers), the full Stage Validation Test Gate.
2. **Work orders are beads:** `br ready --label autonomous-scheduling --json`.
3. **Core contracts (compact):** separate tick/worker threads; clock seam (injected `now`); two lanes by job class; defer-on-denial (no attempt_count bump, no lease); `procurement_source_lanes=1` serializes same-source (R7); §4.4 landmine (never overwrite `interest_registration.state`); `commercial_sweep` in-process, NEVER `trigger_pipeline_rerun`; schema-epoch coupling (if Q4=table, bump all constants 10→11 together).
4. **Bead completion policy (compact):** `bin/br-claim`/`bin/br-complete` only; fully complete each bead; full per-file gate must pass; push BEFORE `bin/br-complete`.
5. **The full per-file test gate**, not `--bin daemon` alone — the net-new suites (scheduler_tick.rs, worker_pump.rs, procontract_resumption.rs) are only exercised by the `for t in crates/daemon/tests/*.rs` loop.
6. **Work loop (compact):** inbox → `br ready` → read bead → `bin/br-claim` → TDD implement → verify (full gate) → commit `<bead-id>:` + push → `bin/br-complete` → loop. Don't stop to ask what's next.

---

## Reminders Specific To This Feature

- **The clock-seam/one-shot entrypoint (`--run-scheduler-once --tick-now`) is the test harness** — if E2E manifests M1/M2 are failing to even run (hanging), check the agent isn't pointing `RunHarness.run()` at `--serve` (it blocks on `Command::output()`). Use the one-shot entrypoint.
- **Foundations beads gate everything** — if `br ready` looks empty but beads remain, the clock-seam/refactor/Q1 beads probably aren't done; check the dependency tree.
- **Don't let the live-loop refactor break the manual path** — `agent_procurement_ingestion.rs` test 001 + `agent_contracts_finder_ingestion.rs` must still pass unchanged.
- **If a worker handler test passes with only `status='completed'`** — it's hollow. worker_001 requires a real-side-effect row assertion per kind.

---

## Mandatory Stop

⛔ When all beads under `autonomous-scheduling` are closed and the full Stage Validation Test Gate is green, STOP. Do not self-authorize the completion audit. The human triggers step 12 (`/bead-completion-audit`).
