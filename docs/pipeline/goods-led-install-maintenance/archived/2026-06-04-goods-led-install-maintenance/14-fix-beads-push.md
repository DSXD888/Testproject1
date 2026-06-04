# Goods-Led Install/Maintenance — Fix Beads Push

> For continuing agents executing fix beads. Compact reminder of contracts, work loop, and verification.

---

## Re-Orient

1. Re-read `AGENTS.md` — agent rules + coordination (Rule 2 constitutional).
2. Feature: goods-led install/maintenance — executing fix beads from the completion audit.
3. Spec: `docs/pipeline/goods-led-install-maintenance/artifacts/goods-led-install-maintenance-spec.md` (SOT #1, tiebreaker).
4. TDD: `docs/pipeline/goods-led-install-maintenance/artifacts/goods-led-install-maintenance-tdd.md` (SOT #2).
5. Fix bead graph: `br ready --label audit-fix --json`

---

## Core Contracts (compact)

Integer pence / bps (10000=100%). Screening-grade ±20%, never a firm quote. Fail-soft (`blocked` labour → `review`, no crash). Labour summed at BOTH rollup sites (main.rs:8114 + 8548); columns nullable/default-0; labour only when an `InstallationRequirement` exists. Near-floor value-share → `review`. Labour skill `http_public` (no CDP).

---

## Bead Completion Policy

`bin/br-claim <id>` / `bin/br-complete <id>` only — never raw `br update`/`br close` (Rule 2). Every fix bead fully complete: all code + referenced TDD tests + full AGENTS.md stage gate + committed AND pushed + `br-complete` clean. No partial work, no TODOs, no "finish later." If blocked: diagnose, fix yourself; escalate via AgentMail only if the blocker is in another agent's code. Do NOT abandon the fix bead.

---

## Work Loop

```
1. fetch_inbox → acknowledge messages
2. br ready --label audit-fix --json → pick highest-priority ready fix bead
3. Read fix bead FULLY → study target files
4. CLAIM: bin/br-claim <fix-bead-id>   (--paths <files> if no Files section)
5. IMPLEMENT: read the spec section + TDD test IDs in the bead → fix/write tests first → code. No mid-way commits.
6. VERIFY (full stage gate): cargo fmt --check && cargo check --all-targets && cargo clippy --all-targets -- -D warnings && cargo test -p daemon --bin daemon && (every crates/daemon/tests/*.rs target) && bun test
7. COMMIT + PUSH: git add <specific-files> && git commit -m "<fix-bead-id>: <desc>" && git pull --rebase && git push
8. COMPLETE: bin/br-complete <fix-bead-id>
9. LOOP → step 1
Do not stop to ask what to work on next. Pick the next ready fix bead.
```

---

## Verification (Every Fix Bead)

Use debug builds for fast iteration. The full AGENTS.md stage gate must pass — a passing `cargo test -p daemon --bin daemon` alone is NOT green; every `crates/daemon/tests/*.rs` integration target (incl. `installation_labour_engine.rs`) + `bun test` must run. Fix ALL failures before committing.

---

## Completion Gate

When all fix beads are done, run the full stage gate one final time. If gaps remain, go back to the completion audit (step 12) and re-audit. If fully blocked: push work, release reservations, announce via AgentMail.

---

## AgentMail Quick Reference

NO broadcast. Discover peers via `list_contacts(project_key, agent_name)` first. Wrappers send Start/Complete mail; use `send_message` directly only for blockers/questions (`thread_id="<fix-bead-id>"`). Release with `release_file_reservations(project_key, agent_name)`.
