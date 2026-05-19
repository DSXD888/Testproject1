# L5 Expanded Commercial Intelligence — 14 · Fix Beads Push (Continuation Nudge)

> For continuing agents working on L5 fix beads. Compact reminder of contracts, work loop, and verification.

---

## Re-Orient

**STOP. Two mandatory blocking steps before you touch any other tool:**

1. **Re-read `AGENTS.md` in full.** Top to bottom. It is constitutional — Rule 1 (no file deletion), Rule 2 (`bin/br-claim` / `bin/br-complete` wrappers — raw `br update --status=in_progress` / `br close` are FORBIDDEN), branching (main only), toolchain matrix (Cargo + Bun split), Stage Validation Test Gate at lines 150-166 (`cargo test -p daemon --bin daemon` is INSUFFICIENT — must iterate every integration test in `crates/daemon/tests/*.rs`), AgentMail conventions, beads workflow (`br` not `bd`; `bin/br-sync` not raw `br sync`), UBS bug scanner. AGENTS.md wins over this push prompt if anything disagrees.
2. **Confirm AgentMail registration.** If you don't have an `agent_name`, run the `macro_start_session` block below FIRST, then `export AGENT_MAIL_NAME="<your-name>"` (the wrappers require that env var). Then `fetch_inbox(...)` to read any pending blocker notices left by sibling agents — address those before claiming new fix work.

After those two:

1. Feature: L5 Expanded Commercial Intelligence — executing **fix beads** from completion audit.
2. Spec: `docs/strategy/layers/layer-5-expanded-commercial-intelligence.md` — SOT #1 (tiebreaker if fix bead is ambiguous).
3. TDD: `docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-tdd.md` — SOT #2 (test definitions).
4. Strategy: `docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-strategy.md` — SOT #3 (phase ordering).
5. Completion audit report: `docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-completion-audit-{YYYY-MM-DD}.md`.
6. Fix bead graph: `br ready --label l5 --label audit-fix --json`.

**AgentMail registration (compact):**
```
macro_start_session(
  project_key="/Users/alinaaitova/Desktop/Testproject1",
  program="l5-fix-beads-executor",
  model="<your-model>",
  task_description="L5 Expanded Commercial Intelligence fix-bead execution"
)

# REQUIRED for the wrappers — set in the same shell where you'll run bin/br-claim / bin/br-complete
export AGENT_MAIL_NAME="<your-name>"
```

---

## Core Contracts (compact, unchanged from main execution)

1. **Conservative defaults are constitutional** — never relax. Surface gap via `unlockable_margin_bps` + `evidence_leverage`.
2. **FPO is one source among five — never primary.** FPO alone is NOT evidence (spec §1.4 critical rule).
3. **UK-specificity rule** — HMRC overrules Taric Support / HS Ping on UK national measures (spec §1.4).
4. **Never greedy descent** — multi-family expansion considers ALL plausible HS6/CN8 families (spec §1.5).
5. **Compound/specific duty expressions preserved** — never flatten to single `dutyPct` (spec §1.8).
6. **Customs branch divergence is a hard blocker** — emit `CLASSIFICATION_AMBIGUOUS` / `TARIFF_BRANCH_DIVERGENCE`; do not average (spec §1.13).
7. **Supplementary units are first-class** — emit `SUPPLEMENTARY_UNIT_MISSING` and abstain when missing (spec §1.8).
8. **L5→L6 readiness gate (constitutional)** — these MUST remain producible: `CLASSIFICATION_AMBIGUOUS`, `TARIFF_BRANCH_DIVERGENCE`, `SUPPLEMENTARY_UNIT_MISSING`, `decision_grade = broker_required`, classification freshness expired.
9. **CBAM is placeholder-only in L5** — rate-bearing logic awaits secondary legislation (spec §1.14).
10. **Skill-first (operator decision 2026-05-08) — no paid API procurement.** Integrations: HMRC public APIs + Frankfurter + CDP browser skills (existing SeaRates/HMRC/simple-fx-lookup + operator-built Freightos + EU TARIC). No mocks — fixture cassettes for unit, real HTTP / real CDP for integration. Xe / Freightos paid / SeaRates paid / Taric Support paid / HS Ping / Dutify-Zonos-Avalara are DECLINED.
11. **Single-edit chokepoints** (one bead at a time): `crates/daemon/src/errors/manifest.rs` (ErrorCode), `crates/daemon/src/sources/registry.rs`, `crates/daemon/src/scheduler/cron.rs`, `crates/daemon/src/main.rs` Phase 6 refactor target, `crates/daemon/src/db/sync.rs` `PUSH_OWNED_RESEED_TABLES`, daemon `Cargo.toml`, root `package.json`.
12. **NO file deletion** (AGENTS.md Rule 1).

If code conflicts with these contracts, the contract wins.

---

## Fix-Bead Completion Policy

**Every fix bead you claim MUST be fully completed.** No partial work. No TODOs. No "finish later."

Complete = all code + all tests + all checks + committed + pushed + `bin/br-complete` succeeded.

**If blocked:** Diagnose it. Fix it yourself. Only escalate via AgentMail if the blocker is in another agent's code. Do NOT abandon the fix bead.

**Wrappers are the hard gate (AGENTS.md Rule 2 — constitutional):**

- **Claim with `bin/br-claim <fix-bead-id>`** — atomic: reserves files (collision detector), sends `[<id>] Start: <title>` mail to peers from `list_contacts`, sets `br` status `in_progress`. If it exits non-zero, the bead is NOT claimed — fix the underlying issue (other-agent claim, file reservation conflict, agent-mail server down, `AGENT_MAIL_NAME` unset) and retry. Do NOT use raw `br update --status=in_progress` to bypass.
- **Complete with `bin/br-complete <fix-bead-id>`** — atomic: verifies your `[<id>] Start` exists, finds the `<bead-id>:`-prefixed commit, verifies `HEAD == upstream` (refuses if you haven't pushed), sends `[<id>] Complete: <title>`, runs `br close`, syncs `.beads/`, pushes, releases reservations. Do NOT use raw `br close <id>` to bypass.
- **Between every fix bead:** `fetch_inbox(...)` — wrappers do NOT do this for you. Surface blocker notices another agent left before you pick the next bead.

If a sibling agent runs `search_messages(query="[<fix-bead-id>]")` and finds no Start or no Complete from you, the fix bead is treated as un-done. Wrapper success IS Start/Complete success — they're inseparable.

---

## Work Loop (compact)

```
1. Check inbox: fetch_inbox(project_key="/Users/alinaaitova/Desktop/Testproject1", agent_name=<your-name>, limit=10) → acknowledge messages
2. br ready --label l5 --label audit-fix --json → pick highest-priority ready fix bead
3. Read fix bead FULLY → study target files referenced
4. CLAIM (atomic — AGENTS.md Rule 2):
   bin/br-claim <fix-bead-id>
   # Files parsed from the bead's `### Files to Create/Modify` section.
   # If absent: bin/br-claim <fix-bead-id> --paths <p1> <p2> ...
   # If exit ≠ 0: bead NOT claimed; fix root cause and retry.
   # NEVER bypass with raw `br update --status=in_progress`.
5. IMPLEMENT: Read spec L5-FR + TDD L5-T tests referenced. Fix/write tests first, then code. Implement ALL of it. No mid-way commits. No partial work. Conservative defaults are constitutional.
6. VERIFY: Stage Validation Test Gate (full)
7. COMMIT:
   ubs $(git diff --name-only --cached)
   git add <specific-files>
   git commit -m "<fix-bead-id>: <short description>"
   git pull --rebase && git push
8. COMPLETE (atomic — AGENTS.md Rule 2):
   bin/br-complete <fix-bead-id>
   # Wrapper verifies your Start mail + that HEAD == upstream, sends
   # [<id>] Complete, runs br close, syncs .beads/, pushes, releases
   # reservations. If your commit subject doesn't start with the bead id,
   # pass --commit <sha>. NEVER bypass with raw `br close <id>`.
9. LOOP → step 1

Do not stop to ask what to work on next. Pick the next ready fix bead.
```

---

## Verification (Stage Validation Test Gate — full)

Use debug builds for fast iteration. Fix ALL failures before committing.

```bash
# Rust full gate (NEVER skip the test-file iteration — AGENTS.md:150-166)
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

# Pre-commit
ubs $(git diff --name-only --cached)
```

---

## Completion Gate

When all fix beads are done:
- Run the FULL Stage Validation Test Gate one final time.
- Re-run completion audit (step 12); must return COMPLETE verdict.
- If gaps remain, go back to step 12 and re-audit (loop).
- Verify the L5→L6 readiness gate (5 conditions) is still producible end-to-end.

If fully blocked: push all work, release reservations, announce via AgentMail with status of what's done + what's blocked.

---

## Common Fix Bead Categories (you'll see one of these)

- **Customs error code emission missed** — extend `packages/intelligence/src/customs/{multi-source-recall,measure-scanner,exporter-resolution,classification-freshness}.ts`; verify with test in `crates/daemon/tests/customs_classification.rs` (or co-located TS test).
- **`landed_cost_estimates` column populator missing** — extend `packages/intelligence/src/customs/`, `fx/`, or `logistics/`; verify SQL fixture in `crates/daemon/tests/sqlite_supabase_sync.rs`.
- **Measure-interaction golden case missed** — extend `packages/intelligence/src/customs/measure-scanner.ts`; add golden case to `crates/daemon/tests/customs_measure_interaction.rs` or `tests/fixtures/calibration/customs-classification/{category}/`.
- **Conservative-default trigger missed** — extend `packages/intelligence/src/customs/exporter-resolution.ts` or `customs/measure-scanner.ts`; verify with parametric test.
- **Freshness-vector logic missed** — extend `packages/intelligence/src/customs/classification-freshness.ts`; cron entry in `crates/daemon/src/scheduler/cron.rs`; integration test for state transition.
- **CBAM forward-exposure flag missing** — extend customs engine; test for tender window crossing 2027-01-01 against in-scope families (aluminium, cement, fertiliser, hydrogen, iron and steel).
- **Marketplace skill canary missing** — extend `tests/fixtures/calibration/marketplace-search/{source}/canary.inputs.jsonl` + `golden.assertions.yaml`; verify with `crates/daemon/tests/skill_canary_harness.rs`.
- **Redaction-boundary test missing** — extend `crates/daemon/tests/redaction_boundary.rs`; ensure new sensitive `landed_cost_estimates` field is RLS-tested.
- **Gold-set category missing** — extend `tests/fixtures/calibration/customs-classification/{new-category}/`.
- **L5→L6 gate emission missing** — locate engine code path; emit gate condition; integration test for emission.
- **Skill registry entry missing** — extend `crates/daemon/src/sources/registry.rs:570-680`; verify daemon registry test.
- **Cron entry missing** — extend `crates/daemon/src/scheduler/cron.rs:16-34` (constants 16-23, struct 26-34); verify cron registration test.
- **ErrorCode variant missing or `as_str()` matcher mismatch** — extend `crates/daemon/src/errors/manifest.rs:20-55+` (NOT validation/policy.rs); clippy will surface propagation sites across `validation/policy.rs`, `validation/executor.rs`, `errors/markdown.rs`, `tracing/structured.rs`, `ipc/handlers/*`.

Each fix bead body specifies the exact category + file paths + test IDs + ACs.

---

## AgentMail Quick Reference (compact)

**Wrappers handle Start/Complete + reservations atomically.** Use raw mail tools only for inbox checks and BLOCKED escalations. Agent Mail has NO broadcast — `"broadcast"` is NOT a valid recipient. The wrappers discover peers via `list_contacts` for you.

```
# Atomic claim/complete (AGENTS.md Rule 2 — REQUIRED)
bin/br-claim <fix-bead-id>                  # reserves files + Start mail + br update
bin/br-complete <fix-bead-id>               # Complete mail + br close + .beads/ sync + release

# Raw tools — only between beads or for blocker escalations
fetch_inbox(project_key="/Users/alinaaitova/Desktop/Testproject1", agent_name=<your-name>, limit=10)
send_message(project_key="/Users/alinaaitova/Desktop/Testproject1", sender_name=<your-name>, to=["<peers>"],
  thread_id="<fix-bead-id>", subject="[<fix-bead-id>] BLOCKED: <issue>",
  body_md="File: <path>, Line: <N>. Issue: <desc>. Needed: <what>")
list_contacts(project_key="/Users/alinaaitova/Desktop/Testproject1", agent_name=<your-name>)  # rarely needed manually

# Forbidden (bypasses Rule 2):
#   br update <id> --status=in_progress
#   br close <id> --reason "Completed"
# If a wrapper fails, fix the underlying issue (server down, env var unset,
# reservation conflict). Never bypass.
```

---

## Critical Rules (numbered, compact)

1. Fix beads = targeted corrections, NOT new features.
2. Implement from fix bead body; spec is tiebreaker only.
3. NEVER delete files (AGENTS.md Rule 1).
4. **ALWAYS use `bin/br-claim` and `bin/br-complete` (AGENTS.md Rule 2).** Raw `br update --status=in_progress` and `br close <id>` are forbidden.
5. NEVER stash other agents' work.
6. Use `br` (NOT `bd`); `bin/br-sync` (NOT raw `br sync`); `bin/br-complete` runs sync inside its flow.
7. Stage Validation Test Gate is minimum gate — iterate every `crates/daemon/tests/*.rs`.
8. Conservative defaults are constitutional — never relax to bypass a fix.
9. FPO is never primary; UK-specificity rule.
10. Never greedy descent in multi-family expansion.
11. Compound/specific duty preserved; never flatten.
12. L5→L6 readiness gate (5 conditions) must remain producible after every fix.
13. ErrorCode lives in `crates/daemon/src/errors/manifest.rs:20+`, NOT `validation/policy.rs`.
14. Do NOT ask what to work on — `br ready --label l5 --label audit-fix` answers.
15. Work is NOT done until `git push` succeeds AND `bin/br-complete` succeeds.
16. After ALL fix beads done: re-run completion audit (step 12).

---

## Quick Reference

| | |
|---|---|
| Fix bead graph | `br ready --label l5 --label audit-fix --json` |
| Bead CLI | `br` |
| Bead sync | `bin/br-sync` |
| Bead claim | `bin/br-claim <id>` |
| Bead complete | `bin/br-complete <id>` |
| AgentMail key | `/Users/alinaaitova/Desktop/Testproject1` |
| Spec | `docs/strategy/layers/layer-5-expanded-commercial-intelligence.md` |
| TDD | `docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-tdd.md` |
| Audit report | `docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-completion-audit-{YYYY-MM-DD}.md` |
| ErrorCode location | `crates/daemon/src/errors/manifest.rs:20-55+` (NOT validation/policy.rs) |
| Branch | `main` |
