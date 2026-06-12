# Autonomous Daemon Scheduling — Fix Beads Push

> For continuing agents executing fix beads. Compact reminder of contracts, work loop, and verification.

---

## Re-Orient
1. Re-read `AGENTS.md` — Rule 2 (wrappers), Stage Validation Test Gate.
2. Feature: autonomous daemon scheduling — executing fix beads from the completion audit (step 12).
3. Spec: `docs/pipeline/autonomous-daemon-scheduling/artifacts/autonomous-daemon-scheduling-spec.md` (SOT #1, tiebreaker).
4. TDD: `docs/pipeline/autonomous-daemon-scheduling/artifacts/autonomous-daemon-scheduling-tdd.md` (SOT #2).
5. Fix bead graph: `br ready --label audit-fix --json`.

---

## Core Contracts (compact)
Separate tick/worker threads · clock seam (injected `now`, no inline `Utc::now()`) · two lanes by job class · defer-on-denial (no attempt_count bump, no lease) · `procurement_source_lanes=1` serializes same-source (R7) · §4.4 landmine: never overwrite `interest_registration.state` · `commercial_sweep` in-process, NEVER `trigger_pipeline_rerun` · epoch coupling (all constants move together) · `#![forbid(unsafe_code)]`, no compat shims.

---

## Bead Completion Policy
`bin/br-claim`/`bin/br-complete` only — never raw `br update`/`br close` (AGENTS.md Rule 2 constitutional). Every fix bead fully completed: all code + referenced TDD tests + FULL per-file gate + committed AND pushed + `bin/br-complete` clean. No partial work. If blocked: diagnose, fix yourself; escalate via AgentMail only if the blocker is in another agent's code. Do NOT abandon the fix bead.

---

## Work Loop
```
1. fetch_inbox → acknowledge messages
2. br ready --label audit-fix --json → pick highest-priority ready fix bead
3. Read fix bead FULLY → study target files + the spec section it cites
4. CLAIM: bin/br-claim <fix-bead-id>   (--paths override if no Files section)
5. IMPLEMENT: fix/write referenced tests FIRST → implement → verify ACs. No mid-way commits.
6. VERIFY: full gate (fmt + check + clippy + --bin daemon + for-loop over tests/*.rs)
7. COMMIT: git add <files> → git commit -m "<fix-bead-id>: <desc>" → git pull --rebase && git push
8. COMPLETE: bin/br-complete <fix-bead-id>
9. LOOP → step 1
Do not stop to ask what's next. Pick the next ready fix bead.
```

---

## Verification (Every Fix Bead)
```bash
cargo fmt --check && cargo check --all-targets && cargo clippy --all-targets -- -D warnings
cargo test -p daemon --bin daemon
for t in crates/daemon/tests/*.rs; do cargo test --test "$(basename "$t" .rs)" || exit 1; done
```
`--bin daemon` alone is NOT green (skips scheduler_tick.rs / worker_pump.rs / procontract_resumption.rs). Use debug builds for iteration.

---

## Completion Gate
When all fix beads are done, run the full gate one final time. If gaps remain, go back to step 12 and re-audit. If fully blocked: push work, release reservations, announce via AgentMail.

---

## AgentMail Quick Reference
NO broadcast. Discover peers via `list_contacts(project_key, agent_name)` first.
```
list_contacts(project_key, agent_name)   # → <peers>
fetch_inbox(project_key, agent_name, limit=10)
send_message(project_key, sender_name=<you>, to=["<peers>"], thread_id="<id>",
  subject="[<id>] BLOCKED: <issue>")
release_file_reservations(project_key, agent_name)
```
