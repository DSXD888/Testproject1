# Post-Merge Quality Gate Sweep - Validation Manifest

> Generated: 2026-05-02T13:47:23Z
> HEAD: 5a8f4fd
> Status: RED

## Summary

All compiler, lint, Rust test, and Vitest gates passed on the final stable worktree snapshot. The requested baseline drifted during execution: the handoff expected HEAD `4ba771d`, but `main` advanced to `5a8f4fd` while gates were running, adding the flake-cleanup, documentation, and Contracts Finder walkthrough commits. That explains the positive count deltas: `native_shell` now passes `16/16`, `fault_process_cleanup` passes in the per-file loop, `query_service` now has `22` tests, and Vitest now has `36` tests. Status is RED only because the live ignored `state/state.db` contains an extra Contracts Finder manifest/assessment row beyond the requested invariant counts; the expected Stage 14 DGU rows and multi-source source counts are intact. No destructive DB cleanup was performed.

## Gate-by-gate results

| Gate | Expected | Observed | Status | Notes |
|---|---:|---:|---|---|
| git status --short | clean | clean at initial pre-flight; later concurrent edits appeared | PASS with drift | Initial R.1 was empty. Current repo changes are from this sweep plus concurrent handoffs and are included in this handoff state. |
| cargo fmt --check | clean | clean | PASS | Final stable snapshot passed silently. |
| cargo check --all-targets | clean | clean | PASS | Final stable snapshot passed. |
| cargo clippy --all-targets -- -D warnings | 0 errors | 0 errors | PASS | Initially failed on `map(...).unwrap_or_else(...)`; fixed and re-ran clean. |
| cargo test -p daemon --bin daemon | 169 / 169 | 169 / 169 | PASS | `169 passed; 0 failed`. |
| cargo test -p procintel-desktop --test query_service | 21 / 21 | 22 / 22 | PASS | Extra withdrawn-decision regression test landed during sweep. |
| cargo test -p procintel-desktop --test daemon_bridge | 11 / 11 | 11 / 11 | PASS | Clean. |
| cargo test -p procintel-desktop --test native_shell | 11 / 15, 4 expected_failures | 16 / 16 | PASS | Positive delta from `f2fc0ec` flake-cleanup commit. |
| bunx vitest run (apps/desktop) | 35 / 35 | 36 / 36 | PASS | Extra native event-name regression test landed during sweep. |
| Per-file integration loop | 38 / 39, 1 expected_failure | 39 / 39 | PASS | `fault_process_cleanup` passed after `f2fc0ec`. |
| Section 3 state-DB invariants | match expected | expected rows intact, plus 1 extra reject and 1 extra clarify manifest | RED | Extra row is `proc-ocds-b5fd17-5399000a-8bde-4a4b-aa59-b025c4f5881b`; no DB deletion performed. |

## Findings

1. Requested HEAD did not match current `main`.
   - Reproduction: `git log --oneline 91a1484..HEAD`
   - Expected by handoff: HEAD `4ba771d`, 4 commits since `91a1484`.
   - Observed: HEAD `5a8f4fd`, 10 commits since `91a1484`.
   - Disposition: Documented. Later commits explain the native-shell and integration-loop positive deltas.

2. Clippy failure in Stage 14 query-service test helper.
   - Reproduction: `cargo clippy --all-targets -- -D warnings`
   - Error: `clippy::map_unwrap_or` at `apps/desktop/src-tauri/tests/query_service.rs`.
   - Attribution: helper introduced by `be38085` Stage 14 Phase 2 / order-quantity handoff.
   - Disposition: Fixed in place with `Option::map_or_else`; final clippy passed.

3. Decision-status read-model partial merge produced compile failures during the moving-tree pass.
   - Reproduction: `cargo test -p procintel-desktop --test query_service`
   - Error: missing `status` in `DecisionStatus` and `DecisionRecord` initializers.
   - Attribution: concurrent desktop status plumbing landed in the worktree during this sweep.
   - Disposition: Fixed in place by selecting `pending_operator_intents.status`, carrying it through `DecisionRecord`, projecting it in process-list and manifest-inbox VMs, and adding query-service regression assertions.

4. State DB invariant count mismatch.
   - Reproduction: run the Section 3 SQLite query.
   - Expected: candidate net-margin verdicts all `review` with count 17; manifests all `clarify` with count 7.
   - Observed: `review=17`, `reject=1`; `clarify=8`.
   - Extra process: `proc-ocds-b5fd17-5399000a-8bde-4a4b-aa59-b025c4f5881b`.
   - Disposition: Documented as live local state drift. Removing rows from `state/state.db` would be destructive data mutation, so it was not performed without explicit authorization.

5. Agent Mail pre-flight health was red.
   - Reproduction: `health_check`
   - Observed: degraded read-only mode because the archive inventory is ahead of the SQLite index.
   - Disposition: Documented. It did not affect local quality gates.

## Fixes applied

| File | Change | Justification |
|---|---|---|
| `apps/desktop/src-tauri/tests/query_service.rs` | Replaced `map(...).unwrap_or_else(...)` with `map_or_else(...)`. | Required for `cargo clippy --all-targets -- -D warnings`. |
| `apps/desktop/src-tauri/src/query_service.rs` | Completed `DecisionStatus.status` projection from `pending_operator_intents.status`; made stale calculation explicit. | Required to compile and to expose decision lifecycle state consistently. Landed in `5a8f4fd`. |
| `apps/desktop/src-tauri/tests/query_service.rs` | Added regression assertions for process-list and manifest-inbox decision status. | Covers the read-model fix. |
| `.gitignore` | Ignored `runtime-*/` directories. | Keeps generated runtime walkthrough state out of Git without deleting local files. |

## Out-of-scope items deferred

| Bead ID | Title | Why deferred |
|---|---|---|
| N/A | Live `state/state.db` extra Contracts Finder manifest/assessment row | Fixing the count would require destructive DB row deletion or a DB reset, which is outside this handoff without explicit authorization. |
