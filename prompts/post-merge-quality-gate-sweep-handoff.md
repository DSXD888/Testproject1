# Post-Merge Quality Gate Sweep on Main

ULTRATHINK and be meticulous.

## MISSION

Four parallel handoffs (Stage 14 Phase 2, Stage 11 order-quantity hardening, Stage 11 CDP resilience, multi-source ingestion) landed on `main` between 2026-05-02 13:18 and 14:11. Each was independently validated against a different baseline (some on main, one in a worktree, one cross-applied via `git apply --3way`). The merged state at HEAD `4ba771d` has **never been formally re-gated as a single unit**. This handoff is the cross-handoff coherence check — confirm that running every gate against the unified main produces clean results, and explain any delta vs the four individual manifests.

You will **run → diagnose → fix → re-run in a loop** until every numbered acceptance criterion (R.1–R.10) is GREEN. The expected steady-state is "all gates pass cleanly" — but if any fail, the fix is part of this handoff (not deferred), provided it's small and clearly attributable to the merge.

```
┌────────────────────────────────────────────────────────────┐
│                          THE LOOP                            │
│   1. Pre-flight (§0)                                         │
│   2. Run gates in order (§2)                                │
│   3. For each failure: diagnose via §7 playbook              │
│   4. Fix or document as out-of-scope; re-run                │
│   5. All green → write the validation manifest               │
└────────────────────────────────────────────────────────────┘
```

Read `AGENTS.md` first.

---

## §0 — PRE-FLIGHT

| Claim | Verify |
|---|---|
| `main` HEAD = `4ba771d` ("Stage 11 hardening — CDP captcha resilience for SeaRates skill"); 4 new commits since `91a1484` (Stage 14 Phase 1) | `git log --oneline 91a1484..HEAD` |
| Daemon binary unit count expected: **169** (147 baseline + 10 OQ.* + 12 CDP.*) | `cargo test -p daemon --bin daemon 2>&1 \| grep "test result:"` |
| Tauri test counts expected: vitest 35 / 35; query_service 21 / 21; daemon_bridge 11 / 11; native_shell 11 pass + 4 pre-existing FAIL (schema-epoch fixtures) | run individually below |
| Per-file integration loop expected: 38 / 39 PASS, 1 PRE-EXISTING FAIL (`fault_process_cleanup` — 3 timing-sensitive sub-tests). See bead `MS-FLAKY-PROCESS-CLEANUP` | run loop below |
| Working tree must be clean (no stray uncommitted state from prior agent runs) | `git status --short` |
| `agent-mail` healthy; commits (if any) need `AGENT_NAME=claude-code git commit ...` | inline at commit time |

**Parallel-safety**: this handoff is read-only on production code (the only writes are if a bug needs fixing). Safe to run alongside the documentation-update handoff (`strategy-docs-l1-l3-implementation-status-handoff.md`) but should NOT run alongside any handoff that modifies daemon/desktop sources (CF walkthrough, CDP live demos, test flake cleanup) because the test outputs would shift.

---

## §1 — DECISIONS ALREADY MADE

### D1 — Pre-existing failures are NOT blockers

Two known failures exist at HEAD baseline (predate all four recent handoffs):

1. `crates/daemon/tests/fault_process_cleanup.rs::l3_t_fault_process_cleanup_003/_004/_008` — timing-sensitive PTY child cleanup verdicts. Tracked as `MS-FLAKY-PROCESS-CLEANUP`.
2. `apps/desktop/src-tauri/tests/native_shell.rs::tau_int_003/_007/_008/_010` — schema-epoch mismatch (test fixtures use `schemaEpoch: 0` or `1`; prod constant `EXPECTED_SCHEMA_EPOCH = 2` at `apps/desktop/src-tauri/src/main.rs:51`).

**Both are addressed by the parallel `test-flake-cleanup-handoff.md`** — DO NOT fix them in this handoff. Document them as known-and-isolated failures (R.5 + R.6 carry an "expected_failures" status), and continue.

### D2 — Failure attribution rule

For any failure that is NOT one of the two pre-existing categories above, attempt attribution:

| Symptom | Probable owner |
|---|---|
| Compilation error mentioning `BasisUndetermined` / `OrderQuantityBasis` | Stage 11 order-quantity hardening (`be38085`) |
| Compilation error mentioning `captcha_intercepted` / `SeaRatesCdpState` / `lookup_cached_freight` | Stage 11 CDP resilience (`4ba771d`) |
| Compilation error mentioning `ManifestInbox` / `OperatorDecisionPanel` / `clarifications.rs` | Stage 14 Phase 2 (`be38085`) |
| Runtime test failure mentioning Contracts Finder / process_relations / canonical_ocid | Multi-source (`f769b23` / `2a52fc2`) |

If attributable to a specific commit, mention the commit in the validation manifest's "Findings" section. If not attributable, document the failure with reproduction steps and decide: fix-here (if small + obvious) or file follow-up bead.

### D3 — No production code refactors

This handoff does NOT refactor merged code, even if cleaner factorings are visible. Bug fixes only, in-place. Opening up cross-handoff refactors here would defeat the point of independent handoff attribution.

---

## §2 — EXECUTION ORDER

Run gates in this order; stop and diagnose at the first unexpected failure:

1. `git status --short` — tree must be clean.
2. `cargo fmt --check` — must be silent.
3. `cargo check --all-targets` — must finish clean.
4. `cargo clippy --all-targets -- -D warnings` — must finish with zero errors.
5. `cargo test -p daemon --bin daemon` — expect 169 passed.
6. `cargo test -p procintel-desktop --test query_service` — expect 21 passed.
7. `cargo test -p procintel-desktop --test daemon_bridge` — expect 11 passed.
8. `cargo test -p procintel-desktop --test native_shell` — expect 11 passed + 4 PRE-EXISTING FAIL.
9. `cd apps/desktop && bunx vitest run` — expect 35 passed.
10. Per-file integration loop (`for t in crates/daemon/tests/*.rs; do cargo test --test "$(basename "$t" .rs)" || break; done`) — expect 38 PASS + 1 PRE-EXISTING FAIL on `fault_process_cleanup`.

After each step that passes, log "✓ R.N PASS". After each step that fails, follow §7 playbook.

---

## §3 — VALIDATION QUERIES

Live state-DB sanity (should match the four manifests' end-state):

```bash
sqlite3 -header -column state/state.db "
SELECT '=== Stage 14 demo state intact ===';
SELECT idempotency_key, status FROM pending_operator_intents WHERE idempotency_key LIKE 'dec-2026-05-01-dgu-%';
SELECT product_candidate_id, json_extract(payload_json, '\$.qty') FROM operator_input_overrides WHERE withdrawn_at IS NULL;

SELECT '=== Stage 11/12/13 invariants ===';
SELECT verdict, COUNT(*) FROM candidate_net_margin_assessments GROUP BY verdict;
SELECT stoplight, COUNT(*) FROM manifests GROUP BY stoplight;
SELECT pc.name, json_extract(lce.confidence_breakdown, '\$.order_quantity_basis.kind') AS basis_kind, cpa.gross_margin_bps, lce.commercial_estimate_confidence_bps
FROM landed_cost_estimates lce
JOIN candidate_pricing_assessments cpa ON cpa.product_candidate_id=lce.product_candidate_id AND cpa.selected_quote_id=lce.supplier_quote_id
JOIN product_candidates pc ON pc.id=lce.product_candidate_id
ORDER BY pc.name;

SELECT '=== Multi-source intact ===';
SELECT source_id, COUNT(*) FROM contract_notices GROUP BY source_id;
SELECT 'process_relations: ' || COUNT(*) FROM process_relations;
"
```

Expected: 1 superseded + 1 withdrawn intent; DGU override at 250; verdicts all `review` (17); stoplights all `clarify` (7); basis_kinds: Aggregates `basis_undetermined`, DGU `from_operator_override`, carpets `rfq_required`; sources: procontract 140 + contracts-finder 100; process_relations 0.

---

## §4 — REQUIRED TESTS

This handoff adds NO new tests (it's a gate sweep, not a code change). All new tests should already exist from the four merged handoffs. If a gate failure is fixed in this handoff, the fix MUST come with a regression test — but the expected steady-state is zero new tests.

---

## §5 — ACCEPTANCE CRITERIA — R.1 to R.10

| # | Criterion |
|---|---|
| R.1 | `git status --short` empty (no stray uncommitted changes) |
| R.2 | `cargo fmt --check` silent |
| R.3 | `cargo check --all-targets` clean |
| R.4 | `cargo clippy --all-targets -- -D warnings` clean (zero errors) |
| R.5 | `cargo test -p daemon --bin daemon` reports 169 passed; expected_failures=0 |
| R.6 | Tauri Rust tests: query_service 21/21; daemon_bridge 11/11; native_shell 11 pass + 4 expected_failures (pre-existing schema-epoch fixtures, see D1) |
| R.7 | `bunx vitest run` in apps/desktop reports 35 passed |
| R.8 | Per-file integration loop: 38 / 39 PASS; 1 expected_failure (`fault_process_cleanup`, see D1) |
| R.9 | §3 validation queries return the expected demo state (Stage 14 intents intact, basis kinds correct, source counts correct, no rogue rows) |
| R.10 | Validation manifest written documenting any deviations from expected gate outputs, including reproduction commands for each |

---

## §6 — FAILURE-MODE PLAYBOOK

| Symptom | Diagnosis | Action |
|---|---|---|
| Daemon test count != 169 | A test was renamed, removed, or added by one of the merges; or a baseline test silently regressed | Compare against the four manifests' claimed counts. Identify the commit that changed the count via `git log -p --since='2026-05-01' -- crates/daemon/src/main.rs \| grep '#\[test\]'`. Document. |
| `cargo clippy` errors on `crates/daemon/src/ipc/handlers/*` | Phase 2 IPC handler module has lint debt (per Phase 2 manifest, some `doc_markdown` warnings were left in parallel-agent territory) | If trivial (missing backticks): fix in place + commit. If structural: file as follow-up bead and skip with `--cap-lints=warn` documented as exception. |
| `bunx vitest run` from repo root sweeps Playwright `*.spec.ts` files | Vitest config requires running from `apps/desktop/` (per Phase 2 manifest §3) | Always `cd apps/desktop` first |
| Per-file integration loop killed by parallel agent's cargo lock | Concurrent build or test contention | Re-run with `CARGO_TARGET_DIR=/tmp/post-merge-gate-target` or wait for parallel agent to finish |
| `fault_process_cleanup` shows MORE than 3 failing sub-tests | New regression — not just pre-existing flake | Investigate via `cargo test --test fault_process_cleanup -- --test-threads=1`. If new failure attributable to a recent merge, fix; otherwise file bead. |
| Compile fails mentioning a parallel-agent symbol that should exist | A merge intermediate state is broken — should not happen if commits are atomic | Inspect the breaking commit via `git bisect`; if confirmed broken commit, escalate; do NOT amend committed history |

---

## §7 — CONSTRAINTS

1. **No production code refactors.** Bug fixes only, in place.
2. **Do not fix `fault_process_cleanup` or `native_shell` failures here.** Those are in `test-flake-cleanup-handoff.md` scope.
3. **No new migrations.**
4. **Do not modify daemon/desktop sources unless a clear merge-attributable bug demands it.** Document instead.
5. **Run every gate even after a failure** — don't bail early. The full failure list is more valuable than a single failure caught.
6. **No silent test exclusion** — if a test must be skipped, document why in the manifest.

---

## §8 — DELIVERABLE

### MANDATORY: Validation manifest

Path: `docs/pipeline/post-merge-quality-gate-sweep-validation-manifest.md`. Template:

```markdown
# Post-Merge Quality Gate Sweep — Validation Manifest

> Generated: <ISO timestamp>
> HEAD: <commit>
> Status: GREEN / RED

## Summary
<1 paragraph: which gates ran, totals, deviations from per-handoff manifests, any fixes applied>

## Gate-by-gate results

| Gate | Expected | Observed | Status | Notes |
|---|---|---|---|---|
| cargo fmt --check | clean | ? | ? | |
| cargo check --all-targets | clean | ? | ? | |
| cargo clippy --all-targets -- -D warnings | 0 errors | ? | ? | |
| cargo test -p daemon --bin daemon | 169 / 169 | ? | ? | |
| cargo test -p procintel-desktop --test query_service | 21 / 21 | ? | ? | |
| cargo test -p procintel-desktop --test daemon_bridge | 11 / 11 | ? | ? | |
| cargo test -p procintel-desktop --test native_shell | 11 / 15 (4 expected_failures) | ? | ? | |
| bunx vitest run (apps/desktop) | 35 / 35 | ? | ? | |
| Per-file integration loop | 38 / 39 (1 expected_failure) | ? | ? | |
| §3 state-DB invariants | match expected | ? | ? | |

## Findings
<Each unexpected failure: full reproduction, attribution, disposition>

## Fixes applied (if any)
| File | Change | Justification |

## Out-of-scope items deferred
| Bead ID | Title | Why deferred |
```

### Quality gates

The handoff IS the quality gate sweep. The manifest IS the deliverable. No additional gates required beyond what's in §2.

---

## §9 — REMINDERS

- ULTRATHINK and be meticulous.
- The expected steady state is **clean across the board, modulo the two known pre-existing failures** (`fault_process_cleanup` 3 sub-tests + `native_shell` 4 sub-tests). Anything else is a regression to investigate.
- This handoff exists because four parallel handoffs touching shared files (especially `crates/daemon/src/main.rs`) landed near-simultaneously. The risk is subtle interaction bugs that didn't appear in any single agent's worktree.
- If you find a real regression, attribute it to a commit and document in the manifest. Don't refactor across handoff boundaries.
- The two pre-existing failure categories are tracked separately and should NOT be fixed here.
