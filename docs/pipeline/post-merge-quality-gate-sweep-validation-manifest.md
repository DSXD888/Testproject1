# Post-Merge Quality Gate Sweep - Validation Manifest

> Generated: 2026-05-03T08:46:17Z
> HEAD: 2317fdf
> Status: GREEN

## Summary

The unified main quality gate is green after restoring the expected validation fixture. The blocker was live SQLite fixture drift, not a code defect: `state/state.db` no longer contained the required Contracts Finder rows and expected landed-cost basis state. Per explicit user authorization, the prior DB was backed up to `state/state.db.pre-restore-20260503T084322Z`, `state/state.db.pre-cdp-demos` was restored into `state/state.db`, and migration `0010_operator_override_audit_rows` was applied so the restored fixture matches current schema epoch 7. Compiler, lint, Rust, Vitest, per-file integration, and Section 3 DB invariant gates now all pass. Test counts are higher than the original handoff because main has advanced beyond `4ba771d`.

## Gate-by-gate results

| Gate | Expected | Observed | Status | Notes |
|---|---:|---:|---|---|
| git status --short | clean | clean | PASS | `main...origin/main`; no tracked changes before this manifest refresh. |
| cargo fmt --check | clean | clean | PASS | Silent. |
| cargo check --all-targets | clean | clean | PASS | Ran with `CARGO_TARGET_DIR=/tmp/procintel-green-target`. |
| cargo clippy --all-targets -- -D warnings | 0 errors | 0 errors | PASS | Ran with `CARGO_TARGET_DIR=/tmp/procintel-green-target`. |
| cargo test -p daemon --bin daemon | 169 / 169 | 175 / 175 | PASS | Positive delta from later CDP/skill work on current main. |
| cargo test -p procintel-desktop --test query_service | 21 / 21 | 23 / 23 | PASS | Positive delta from later desktop regression coverage. |
| cargo test -p procintel-desktop --test daemon_bridge | 11 / 11 | 12 / 12 | PASS | Positive delta from later bridge regression coverage. |
| cargo test -p procintel-desktop --test native_shell | 11 / 15, 4 expected_failures | 17 / 17 | PASS | Former schema-epoch expected failures are fixed on current main. |
| bunx vitest run (apps/desktop) | 35 / 35 | 39 / 39 | PASS | Positive delta from later frontend regression coverage. |
| Per-file integration loop | 38 / 39, 1 expected_failure | 39 / 39 | PASS | `fault_process_cleanup` passed on current main. |
| Section 3 state-DB invariants | match expected | match expected | PASS | Restored fixture has DGU rows, 17 review verdicts, 7 clarify manifests, Contracts Finder 100, ProContract 140, and 0 process relations. |

## Findings

1. State DB invariant mismatch was fixture drift, not a code bug.
   - Reproduction before restore: Section 3 query showed only `procontract=133`, missing `contracts-finder=100`, with incomplete landed-cost basis rows.
   - Fix: backed up current DB to `state/state.db.pre-restore-20260503T084322Z`, restored `state/state.db.pre-cdp-demos` into `state/state.db`, then applied `migrations/0010_operator_override_audit_rows.sql` and set `PRAGMA user_version = 7`.
   - Disposition: GREEN after restore. The restored fixture matches the original post-merge data expectations.

2. Current main has expected positive test-count deltas versus the original handoff.
   - Original handoff targeted HEAD `4ba771d`; current HEAD is `2317fdf`.
   - Deltas: daemon bin `175` vs `169`, query_service `23` vs `21`, daemon_bridge `12` vs `11`, native_shell `17` all passing instead of `11 pass + 4 expected failures`, Vitest `39` vs `35`, and per-file integration `39/39` instead of `38/39`.
   - Disposition: Documented as expected current-main drift.

## Fixes applied

| File | Change | Justification |
|---|---|---|
| `state/state.db` | Restored from `state/state.db.pre-cdp-demos`, then migrated to schema epoch 7. | Required to restore the expected post-merge validation fixture. |
| `state/state.db.pre-restore-20260503T084322Z` | Created backup of the previous live DB before overwrite. | Preserves the pre-restore local state per user authorization. |

## Out-of-scope items deferred

| Bead ID | Title | Why deferred |
|---|---|---|
| N/A | None | All requested gates are green. |
