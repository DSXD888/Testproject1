# L5 Expanded Commercial Intelligence — 13 · Fix Beads Execution Kickoff

> For fresh agents executing `audit-fix` beads from the L5 completion audit. Read this FIRST.

---

## Mission

You are executing **fix beads** for L5 Expanded Commercial Intelligence — gaps identified by the completion audit (step 12) between the spec and the actual L5 implementation. Each fix bead is a targeted correction: a missing field, a wrong assertion, an unimplemented edge case, an under-populated `landed_cost_estimates` column, an unproduced ErrorCode emission, a stale freshness vector, a missing measure-interaction case, etc.

Work from the fix-bead graph (`br ready --label l5 --label audit-fix --json`). Each fix bead is **self-contained** — implement from the bead alone. The L5 spec and TDD are tiebreakers if a fix bead is ambiguous.

**Organization: FLAT.** No hierarchy. Pick the next ready fix bead, implement it fully, move on.

**Coordination: AGENTMAIL-FIRST.** Register, check inbox, announce claims BEFORE touching code.

---

## Implementation Boundary

**EXISTS (already implemented during main L5 execution):**

- L5 customs engine at `packages/intelligence/src/customs/` (6 files: description-quality-gate, multi-source-recall, ten-digit-resolution, measure-scanner, exporter-resolution, classification-freshness).
- L5 FX engine at `packages/intelligence/src/fx/` (4 files: hmrc-exchange-rates, xe-commercial-rates, fx-engine, fx-types).
- L5 freight engine at `packages/intelligence/src/logistics/` (5 files: mode-selector, freightos-client, searates-client, freight-engine, freight-types).
- L5 comparison adapters at `packages/intelligence/src/validation/` (3 files: dutify, zonos, avalara).
- L5 evidence-tiered accuracy at `packages/intelligence/src/customs/accuracy.ts`.
- `migrations/NNNN_l5_product_commodity_codes.sql` — local SQLite junction table.
- `crates/daemon/src/errors/manifest.rs` — 10 new ErrorCode variants integrated (enum + as_str() matcher).
- `crates/daemon/src/sources/registry.rs` — 4-5 new `RegisteredSource` entries for marketplace skills.
- `crates/daemon/src/scheduler/cron.rs` — 4 new `CronJobDefinition` entries (HMRC tariff news watcher, TRA case register watcher, FX monthly rollover, marketplace source-health canary).
- `crates/daemon/src/db/sync.rs` — `PUSH_OWNED_RESEED_TABLES` includes `product_commodity_codes` (new L5 table).
- `crates/daemon/src/main.rs` — landed-cost paths refactored to delegate to `packages/intelligence/src/customs/`.
- `skills/supplier-price-enrichment/{alibaba,amazon,1688}/` — matured to `promotion_stage: stable` with their L5 deltas.
- `skills/procurement-ingestion/pcs/` — operational (Scotland IN-scope per operator decision 2026-05-08).
- `skills/validation/overlay-customs-classification/` (if added per step 03) — calibrated.
- `tests/fixtures/calibration/customs-classification/{category}/` — ≥3 categories with canary inputs + golden assertions.
- New L5 daemon tests at `crates/daemon/tests/customs_*.rs` (or co-located TS tests).

**FIX BEADS ADDRESS:** Gaps, missing edge cases, incorrect behavior, thin test coverage, spec requirements not fully met, populated columns missing values, unproduced ErrorCode emissions, stale freshness logic, broken measure-interaction goldens. These are targeted surgical fixes, NOT new features.

Specific gap categories the fix beads may cover:
- Missing customs error emission (e.g., `CLASSIFICATION_AMBIGUOUS` not triggered when sources diverge).
- Missing `landed_cost_estimates` column populator (e.g., `customs_cost_spread_bps` always NULL).
- Missing measure-interaction golden case (e.g., aluminium named-exporter scenario absent).
- Missing conservative-default trigger (e.g., engine emits non-residual rate without exporter proof).
- Missing freshness-vector logic (e.g., `tradeRemedyWatchState` not transitioning on TRA investigation open).
- Missing CBAM forward-exposure flag emission for tender windows crossing 2027-01-01.
- Missing marketplace skill canary (e.g., Chinese retry on Alibaba not exercised).
- Missing redaction-boundary test for new sensitive `landed_cost_estimates` field.
- Missing L5→L6 gate condition emission verifier.
- Missing gold-set category corpus (e.g., 2 categories instead of required ≥3).

---

## Core Contracts / Non-Negotiables

(L5 contracts are constitutional. Fix beads must respect them. If code conflicts with a contract, the contract wins.)

1. **Conservative defaults are constitutional** — never relax. When resolution is incomplete, apply the spec §1.12 defaults (MFN, residual_default, out-of-quota, standard VAT, boundary scenarios, abstain on supplementary unit). Surface the gap via `unlockable_margin_bps` + `evidence_leverage` keys on `confidence_breakdown`.
2. **FPO is one source among five — never primary classifier.** FPO alone is NOT evidence. Independent agreement requires slot-level compatibility checking (spec §1.4 critical rule).
3. **UK-specificity rule.** Taric Support and HS Ping are useful cross-checks but MUST NEVER overrule HMRC on UK national measures, VAT additional codes (DE 6/17), UK quotas, or UK trade-remedy implementations (spec §1.4).
4. **Never greedy descent.** Multi-family expansion to UK 10-digit considers ALL plausible HS6/CN8 families (spec §1.5). Top-candidate poisoning is a primary silent failure mode.
5. **Compound/specific duty expressions preserved.** Never flatten to single `dutyPct`. HMRC commodity API duty expressions (ad valorem, specific per unit, compound) are first-class (spec §1.8).
6. **Customs branch divergence is a hard blocker** — NOT a scenario band. When plausible UK10 branches produce materially different outcomes, emit `CLASSIFICATION_AMBIGUOUS` / `TARIFF_BRANCH_DIVERGENCE`. Do not average (spec §1.13).
7. **Supplementary units are first-class requirements.** When tariff line requires supp units and data is missing, emit `SUPPLEMENTARY_UNIT_MISSING` and abstain on that duty component (spec §1.8). Do not estimate.
8. **L5→L6 readiness gate is constitutional.** All 5 blocker conditions MUST remain producible: `CLASSIFICATION_AMBIGUOUS`, `TARIFF_BRANCH_DIVERGENCE`, `SUPPLEMENTARY_UNIT_MISSING`, `decision_grade = broker_required`, classification freshness expired (cross-reference-index.md row 229; layer-strategy.md §10).
9. **CBAM is placeholder-only in L5.** Rate-bearing logic awaits secondary legislation. Future-regulatory-exposure flag emission ships; rate computation does NOT (spec §1.14). 2027-01-01 boundary applies; in-scope families: aluminium, cement, fertiliser, hydrogen, iron and steel.
10. **Skill-first architecture (operator decision 2026-05-08).** Zero paid commercial API procurements. Integrations are: HMRC public APIs (Trade Tariff, monthly FX publication), Frankfurter (free, ECB-backed), and CDP browser skills (existing SeaRates/HMRC/simple-fx-lookup; operator-built Freightos + EU TARIC). No mocks for any external integration — fixture cassettes for unit tests, real HTTP / real CDP for integration tests. Do not introduce a new mocking framework. Xe / Freightos paid / SeaRates paid / Taric Support paid / HS Ping / Dutify-Zonos-Avalara are DECLINED — do not add them back to fix code or fix beads.
11. **NO file deletion** (AGENTS.md Rule 1 — constitutional).
12. **Bead claim discipline via wrappers (AGENTS.md Rule 2 — constitutional).** Use `bin/br-claim <fix-bead-id>` to start and `bin/br-complete <fix-bead-id>` to finish. The wrappers are atomic: claim reserves files (collision detector FIRST), sends `[<id>] Start` mail to peers (via `list_contacts`), and runs `br update --status=in_progress` in the correct order; complete sends `[<id>] Complete`, runs `br close`, syncs `.beads/`, and releases reservations. Raw `br update --status=in_progress` and `br close` are FORBIDDEN — they bypass Agent Mail and the file-reservation collision detector. The pre-commit hook at `.git/hooks/hooks.d/pre-commit/50-agent-mail.py` enforces this.
13. **Agent Mail is a HARD GATE at both ends of every fix bead** — `bin/br-claim` and `bin/br-complete` send the Start/Complete mails for you, so the discipline is "always run the wrappers, never bypass." A fix bead with no `[<id>] Start` mail in the archive was never claimed (sibling agents will collide); a fix bead with no `[<id>] Complete` mail is not closed in the swarm's view regardless of `br` state. `fetch_inbox(...)` between every fix bead — surface any blocker notices another agent left for you before you pick the next one. NO `to="broadcast"` — `"broadcast"` is NOT a valid recipient; always discover peers via `list_contacts(...)` and use `to=["<peers>"]`.
14. **Stage Validation Test Gate** (AGENTS.md:150-166) is the minimum gate. `cargo test -p daemon --bin daemon` alone is INSUFFICIENT — you MUST iterate every integration test in `crates/daemon/tests/*.rs`. If any test fails, fix or properly auto-skip it (runtime probe, not `#[ignore]`).
15. **Single-edit chokepoints** (one bead at a time):
    - `crates/daemon/src/errors/manifest.rs` — ErrorCode enum + `as_str()` matcher.
    - `crates/daemon/src/sources/registry.rs` — RegisteredSource entries.
    - `crates/daemon/src/scheduler/cron.rs` — CronJobDefinition entries.
    - `crates/daemon/src/main.rs` — Phase 6 landed-cost refactor target.
    - `crates/daemon/src/db/sync.rs` — `PUSH_OWNED_RESEED_TABLES` list.
    - daemon `Cargo.toml`, root `package.json` — any new dependency.
16. **AGENTS.md is the constitutional source.** Re-read it (or at minimum re-skim Rule 1, Rule 2, Stage Validation Test Gate, and the `br`/`bin/br-sync`/`bin/br-claim`/`bin/br-complete` conventions) at the start of every fresh session and after any compaction. AGENTS.md wins over this kickoff if they ever disagree.

---

## Source-of-Truth Loading (MANDATORY — Before Starting)

**STOP. Read item #1 fully before any other tool call.** This is constitutional — AGENTS.md governs all the rules below. Skipping it leads to Agent Mail discipline collapse. Re-read AGENTS.md (or at minimum re-skim Rule 1, Rule 2, Stage Validation Test Gate, and the `br`/`bin/br-sync` conventions) at the start of every fresh session and after any compaction.

Read in this order:

1. **AGENTS.md** — **READ FULLY** — agent guidelines, Stage Validation Test Gate (lines 150-166), file deletion rules (Rule 1, lines 13-18), bead claim discipline (Rule 2, lines 21-33), branching, toolchain matrix, beads CLI invariants (`br` not `bd`, `bin/br-sync` not raw `br sync`), Agent Mail conventions.
   ```text
   /Users/alinaaitova/Desktop/Testproject1/AGENTS.md
   ```

2. **L5 Spec — SOT #1** — the authority for WHAT L5 should do
   ```text
   docs/strategy/layers/layer-5-expanded-commercial-intelligence.md
   ```

3. **L5 TDD — SOT #2** — test definitions for verification
   ```text
   docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-tdd.md
   ```

4. **Completion audit report** — context on what gaps were found
   ```text
   docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-completion-audit-{YYYY-MM-DD}.md
   ```

5. **Strategy** — phase ordering + dependency DAG
   ```text
   docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-strategy.md
   ```

6. **Fix bead graph — your ACTUAL work orders**
   ```bash
   br list --label l5 --label audit-fix --json     # All fix beads
   br ready --label l5 --label audit-fix --json    # Ready fix beads
   ```

7. **Integration surfaces** — study for context on each fix
   ```text
   packages/intelligence/src/customs/        — most fixes will land here
   packages/intelligence/src/fx/             — FX-specific fixes
   packages/intelligence/src/logistics/      — freight-specific fixes
   packages/intelligence/src/customs/accuracy.ts  — evidence-tiered accuracy
   crates/daemon/src/errors/manifest.rs      — ErrorCode enum + as_str() matcher (NOT validation/policy.rs — that holds ValidationPolicyMode)
   crates/daemon/src/main.rs                 — Phase 6 landed-cost refactor target
   tests/fixtures/calibration/customs-classification/  — gold-set fixes
   skills/supplier-price-enrichment/         — marketplace skill maturation fixes
   ```

---

## AgentMail Setup (Do This ONCE At Session Start)

```text
ensure_project(project_key="/Users/alinaaitova/Desktop/Testproject1")
register_agent(project_key, program="claude-code", model="opus")
set_contact_policy(project_key, agent_name, policy="open")
fetch_inbox(project_key, agent_name, limit=20)

# REQUIRED for the wrappers — set in the same shell where you'll run bin/br-claim / bin/br-complete
export AGENT_MAIL_NAME="<your-name>"
```

Read and acknowledge any pending messages before starting work. If another agent left a blocker notice, address it first.

---

## Bead Completion Policy (CRITICAL)

**Use the project wrappers — they are the only authorized path** (AGENTS.md Rule 2, constitutional):

- `bin/br-claim <fix-bead-id>` — claims atomically (file reservation FIRST as collision detector, then `[<id>] Start` mail to peers via `list_contacts`, then `br update --status=in_progress`).
- `bin/br-complete <fix-bead-id>` — completes atomically (verifies your `[<id>] Start` mail exists; finds the `<bead-id>:`-prefixed commit; verifies `HEAD == @{u}` and refuses if you haven't pushed; sends `[<id>] Complete` mail; runs `br close`; syncs `.beads/`; pushes; releases reservations).

Do NOT call raw `br update --status=in_progress` or `br close` — they bypass mail and reservations. The Phase B precommit hook (when active) blocks any commit with a `<fix-bead-id>:` subject that has no Start mail in the archive.

**You MUST fully complete every fix bead you claim.** No partial completion. No "I'll finish this later." No moving on to another fix bead while one is in progress.

A fix bead is COMPLETE when:
1. All code changes described in the fix bead are implemented.
2. The specific TDD tests referenced in the bead pass.
3. All build/lint/test verification commands pass (zero warnings).
4. Code is committed with `<fix-bead-id>:` subject prefix and pushed to the remote.
5. `bin/br-complete <fix-bead-id>` has run successfully (mail + close + sync + release).

**If you hit a blocker:**
- DO NOT abandon the fix bead and move to another.
- DO NOT mark it complete with missing pieces.
- DO NOT do mid-way commits of incomplete work.
- Diagnose the blocker. Read the error. Understand the root cause.
- Fix it yourself.
- Only escalate via AgentMail (reply in the fix bead's thread) if the blocker is in another agent's code (file path, line number, what's wrong).
- Do NOT release reservations or close the bead while blocked.

---

## Work Loop

```text
ONE-TIME SETUP:
  1. AgentMail registration (see above; export AGENT_MAIL_NAME)
  2. Read AGENTS.md + spec + TDD + completion audit report
  3. br ready --label l5 --label audit-fix → understand the fix graph

MAIN LOOP (repeat until all fix beads complete):

  ┌─ PICK ──────────────────────────────────────────────┐
  │ br ready --label l5 --label audit-fix --json        │
  │ Pick the highest-priority ready fix bead.            │
  │ Read the fix bead description FULLY.                 │
  └──────────────────────────────────────────────────────┘
           │
  ┌─ CLAIM (atomic via wrapper — AGENTS.md Rule 2) ─────┐
  │ bin/br-claim <fix-bead-id>                           │
  │ # Wrapper does, in order:                            │
  │ #   1. duplicate-claim guard (status + Start search) │
  │ #   2. file reservation (collision detector — FIRST) │
  │ #   3. [<fix-bead-id>] Start mail to peers           │
  │ #   4. br update --status=in_progress                │
  │ # If FILE_RESERVATION_CONFLICT: pick a different bead│
  │ # If exit ≠ 0: bead NOT claimed; fix root cause      │
  │ # (other-agent claim, reservation conflict, mail     │
  │ # server down, AGENT_MAIL_NAME unset) and retry.     │
  │ # NEVER bypass with raw br update --status=in_progress.│
  │ # Override file list with --paths if bead lacks      │
  │ # ### Files to Create/Modify section.                │
  └──────────────────────────────────────────────────────┘
           │
  ┌─ IMPLEMENT ─────────────────────────────────────────┐
  │ 1. Study the target files — understand current state │
  │ 2. Read the spec L5-FR section referenced in the fix │
  │ 3. Write/fix tests FIRST (from TDD test IDs)         │
  │ 4. Implement the fix to make tests pass              │
  │ 5. Verify acceptance criteria from the fix bead      │
  │                                                      │
  │ DO NOT cut corners. DO NOT skip tests.               │
  │ DO NOT leave TODOs. DO NOT do mid-way commits.       │
  │ DO NOT relax conservative defaults.                  │
  └──────────────────────────────────────────────────────┘
           │
  ┌─ VERIFY (Stage Validation Test Gate — full) ────────┐
  │ cargo fmt --check                                    │
  │ cargo check --all-targets                            │
  │ cargo clippy --all-targets -- -D warnings            │
  │ cargo test -p daemon --bin daemon                    │
  │ for t in crates/daemon/tests/*.rs; do                │
  │   cargo test --test "$(basename "$t" .rs)" || exit 1 │
  │ done                                                 │
  │ bun run build && bun run lint && bun run test        │
  │ ubs $(git diff --name-only --cached)                 │
  │                                                      │
  │ Fix ALL failures before proceeding.                  │
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
  ┌─ COMPLETE (atomic via wrapper — AGENTS.md Rule 2) ──┐
  │ bin/br-complete <fix-bead-id>                        │
  │ # Wrapper does, in order:                            │
  │ #   1. verify status=in_progress + Start mail by you │
  │ #   2. resolve commit SHA + verify pushed            │
  │ #   3. [<fix-bead-id>] Complete mail to peers        │
  │ #   4. br close <fix-bead-id>                        │
  │ #   5. bin/br-sync + git add .beads/ + commit + push │
  │ #   6. release_file_reservations(agent_name=self)    │
  │ # NEVER bypass with raw br close.                    │
  └──────────────────────────────────────────────────────┘
           │
        LOOP → back to PICK
```

"Use debug builds for fast iteration."

---

## Verification Commands (per stack surface)

```bash
# Rust full gate (NEVER skip the test-file iteration)
cargo fmt --check
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test -p daemon --bin daemon
for t in crates/daemon/tests/*.rs; do
  cargo test --test "$(basename "$t" .rs)" || exit 1
done

# TypeScript
bun run build
bun run lint
bun run test

# UBS (Ultimate Bug Scanner — pre-commit)
ubs $(git diff --name-only --cached)
```

---

## Common Fix Bead Categories (you'll see one of these)

- **Customs error code emission missed** — extend `packages/intelligence/src/customs/{multi-source-recall,measure-scanner,exporter-resolution,classification-freshness}.ts`; verify with test in `crates/daemon/tests/customs_classification.rs` or co-located TS test.
- **`landed_cost_estimates` column populator missing** — extend `packages/intelligence/src/customs/` or `fx/` or `logistics/`; verify SQL fixture in `crates/daemon/tests/sqlite_supabase_sync.rs` or co-located test.
- **Measure-interaction golden case missed** — extend `packages/intelligence/src/customs/measure-scanner.ts`; add golden case to `crates/daemon/tests/customs_measure_interaction.rs` or `tests/fixtures/calibration/customs-classification/{category}/`.
- **Conservative-default trigger missed** — extend `packages/intelligence/src/customs/exporter-resolution.ts` or `customs/measure-scanner.ts`; verify with parametric test.
- **Freshness-vector logic missed** — extend `packages/intelligence/src/customs/classification-freshness.ts`; cron entry in `crates/daemon/src/scheduler/cron.rs`; integration test for state transition.
- **CBAM forward-exposure flag missing** — extend `packages/intelligence/src/customs/` (likely a new file or accuracy.ts); test for tender window crossing 2027-01-01.
- **Marketplace skill canary missing** — extend `tests/fixtures/calibration/marketplace-search/{source}/canary.inputs.jsonl` + `golden.assertions.yaml`; verify with `crates/daemon/tests/skill_canary_harness.rs`.
- **Redaction-boundary test missing** — extend `crates/daemon/tests/redaction_boundary.rs`; ensure new sensitive `landed_cost_estimates` field is RLS-tested.
- **Gold-set category missing** — extend `tests/fixtures/calibration/customs-classification/{new-category}/`; canary inputs + golden assertions; verify Wilson-bound precision gate.
- **L5→L6 gate emission missing** — locate the engine code path that should produce the gate condition; emit; integration test for emission.
- **Skill registry entry missing** — extend `crates/daemon/src/sources/registry.rs:570-680`; verify daemon registry test.
- **Cron entry missing** — extend `crates/daemon/src/scheduler/cron.rs:16-34` (constants 16-23, struct 26-34); verify cron registration test.
- **ErrorCode variant missing or `as_str()` matcher mismatch** — extend `crates/daemon/src/errors/manifest.rs:20-55+`; clippy will surface propagation sites.

Each fix bead body specifies the exact category + file paths + test IDs + ACs.

---

## Completion Gate

When ALL fix beads are complete:
- Run the full Stage Validation Test Gate one final time.
- Re-run completion audit (step 12); must return COMPLETE verdict.
- If gaps remain, go back to step 12 and re-audit (loop).
- Do NOT stop to ask what to work on next — pick the next ready fix bead.
- Do NOT wait for permission between fix beads.

If fully blocked on all remaining fix beads: push all work, release all reservations, announce handoff via AgentMail with status of what's done and what's blocked.

---

## AgentMail Quick Reference

**CRITICAL:** Agent Mail has NO broadcast. `"broadcast"` is NOT a valid recipient. Discover peers via `list_contacts(project_key, agent_name)` after registration; store result as `<peers>`. Refresh between beads.

```text
# Discover peers (after registration; refresh between beads)
list_contacts(project_key="/Users/alinaaitova/Desktop/Testproject1", agent_name=<your-name>)
# → e.g. ["BlueLake", "RedPeak"]

# Check inbox (between fix beads)
fetch_inbox(project_key="/Users/alinaaitova/Desktop/Testproject1", agent_name=<your-name>, limit=10)

# Claim/Complete via wrappers (handles Start/Complete mail + reservations atomically):
bin/br-claim <fix-bead-id>
bin/br-complete <fix-bead-id>

# Ad-hoc messages (NOT for Start/Complete — wrappers send those):
# Report blocker
send_message(project_key="/Users/alinaaitova/Desktop/Testproject1", sender_name=<your-name>, to=["<peers>"],
  thread_id="<fix-bead-id>", subject="[<fix-bead-id>] BLOCKED: <issue>",
  body_md="File: <path>, Line: <N>. Issue: <desc>. Needed: <what>")

# Mid-implementation question
send_message(project_key="/Users/alinaaitova/Desktop/Testproject1", sender_name=<your-name>, to=["<peers>"],
  thread_id="<fix-bead-id>", subject="[<fix-bead-id>] Question: <topic>")

# Release files (only if voluntarily releasing — wrappers handle normal completion)
release_file_reservations(project_key="/Users/alinaaitova/Desktop/Testproject1", agent_name=<your-name>)

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
13. Do NOT ask what to work on — `br ready --label l5 --label audit-fix` answers.
14. Work is NOT done until `git push` succeeds AND `bin/br-complete` succeeds.
15. After ALL fix beads done: re-run completion audit (step 12).

---

## Quick Reference

| | |
|---|---|
| Fix bead graph | `br ready --label l5 --label audit-fix --json` |
| Bead CLI | `br` (NEVER `bd`) |
| Bead sync | `bin/br-sync` (NEVER raw `br sync`) |
| Bead claim | `bin/br-claim <id>` (NEVER raw `br update --status=in_progress`) |
| Bead complete | `bin/br-complete <id>` (NEVER raw `br close`) |
| AgentMail key | `/Users/alinaaitova/Desktop/Testproject1` |
| Spec | `docs/strategy/layers/layer-5-expanded-commercial-intelligence.md` |
| TDD | `docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-tdd.md` |
| Audit report | `docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-completion-audit-{YYYY-MM-DD}.md` |
| ErrorCode location | `crates/daemon/src/errors/manifest.rs:20-55+` (NOT validation/policy.rs) |
| CronJobDefinition | `crates/daemon/src/scheduler/cron.rs:26-34` (struct); existing constants 16-23 |
| Branch | `main` |
| L5→L6 gate conditions | `CLASSIFICATION_AMBIGUOUS`, `TARIFF_BRANCH_DIVERGENCE`, `SUPPLEMENTARY_UNIT_MISSING`, `decision_grade = broker_required`, classification freshness expired |
