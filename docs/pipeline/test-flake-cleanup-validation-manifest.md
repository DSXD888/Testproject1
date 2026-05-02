# Test Flake Cleanup — Validation Manifest

> Generated: 2026-05-02T13:30:47Z
> Status: GREEN

## Summary

Both requested failure clusters are green on the current checkout. The handoff expected HEAD `4ba771d`; pre-flight found actual HEAD `e71f84e`. The initial `native_shell` baseline reproduced the requested 4 failures. The `fault_process_cleanup` file passed once during pre-flight on this host, but the three timing-sensitive tests were still refactored and then proved stable with 50/50 single-threaded loops.

Cargo validation used `CARGO_TARGET_DIR=/tmp/procintel-flake-target` to avoid a shared workspace target lock held by concurrent builds.

Agent Mail writes were unavailable: repeated `register_agent` / session macro attempts returned `Resource is temporarily busy`, though the health endpoint was reachable.

## Cluster A: native_shell

The desktop production expected schema epoch changed during the session from `2` to `6` due concurrent work in `apps/desktop/src-tauri/src/main.rs`. No production behavior change was introduced by this flake cleanup; the final test fixtures align to the production constant present during validation.

| Test | Fix approach | Before | After | Stability |
|---|---|---|---|---|
| `tau_int_003` | Fixture schema epoch aligned to current expected epoch `6` | FAIL | PASS | 10/10 full-file loop |
| `tau_int_007` | Fixture schema epoch aligned to current expected epoch `6` | FAIL | PASS | 10/10 full-file loop |
| `tau_int_008` | Fixture schema epoch aligned to current expected epoch `6` | FAIL | PASS | 10/10 full-file loop |
| `tau_int_010` | Reconnect welcome fixture schema epoch aligned to current expected epoch `6` | FAIL | PASS | 10/10 full-file loop |

Evidence:

| Command | Result |
|---|---|
| `cargo test -p procintel-desktop --test native_shell -- --test-threads=1` | `16 passed; 0 failed` |
| 10x `native_shell` loop | `10/10` |

## Cluster B: fault_process_cleanup

| Test | Fix approach | Before | After | Stability |
|---|---|---|---|---|
| `_003 clean verdict` | D2 option 3: capture PGID while child is alive, explicitly terminate and wait for exit before cleanup | Pre-existing flake; pre-flight pass on this host | PASS | 50/50 |
| `_004 forced verdict` | D2 option 3: deterministic signal-ignoring Python fixture prints `ready` after installing handlers; test asserts process group is visible before cleanup | FAIL during refactor as `Clean`; fixed | PASS | 50/50 |
| `_008 reliability batch` | D2 option 3: reuse exited-child fixture instead of racing `sleep 0` against `getpgid` | Pre-existing flake; pre-flight pass on this host | PASS | 50/50 |

No D2 option 2 tolerance widening was used for the three requested fault tests, so R.9 is not applicable.

## R.1–R.10 Status

| # | Criterion | Status | Evidence |
|---|---|---|---|
| R.1 | `native_shell` green | PASS | `16 passed; 0 failed` on current HEAD; current file includes one extra support unit test versus handoff's 15 |
| R.2 | Native fixes stable | PASS | 10/10 full-file rerun loop |
| R.3 | `fault_process_cleanup` green | PASS | `28 passed; 0 failed` |
| R.4 | Fault fixes stable | PASS | `_003`, `_004`, `_008` each 50/50 with `--test-threads=1` |
| R.5 | No requested regressions | PASS | daemon bin 169/169, query_service 21/21, daemon_bridge 11/11, native_shell 16/16, Vitest 35/35, integration files 39/39 |
| R.6 | Production code rationale | PASS | Concurrent production changes existed; only formatting/clippy-preserving edits were applied to make required gates pass |
| R.7 | Per-file integration loop | PASS | `SUMMARY pass=39 fail=0` |
| R.8 | Per-test fix approach documented | PASS | See Cluster B table |
| R.9 | Tolerance widening justified | PASS | N/A for requested fault tests; process-state fixtures used instead |
| R.10 | Quality gates | PASS | See Quality Gates |

## Quality Gates

| Gate | Result |
|---|---|
| `cargo fmt --check` | PASS |
| `cargo check --all-targets` | PASS |
| `cargo clippy --all-targets -- -D warnings` | PASS |
| `cargo test -p daemon --bin daemon -- --test-threads=1` | PASS, 169/169 |
| `cargo test -p procintel-desktop --test query_service -- --test-threads=1` | PASS, 21/21 |
| `cargo test -p procintel-desktop --test daemon_bridge -- --test-threads=1` | PASS, 11/11 |
| `cargo test -p procintel-desktop --test native_shell -- --test-threads=1` | PASS, 16/16 |
| `cd apps/desktop && bunx vitest run` | PASS, 35/35 |
| `for t in crates/daemon/tests/*.rs; do cargo test --test "$(basename "$t" .rs)"; done` | PASS, 39/39 |

## Out-of-scope Items

No new beads filed. A production-side clock abstraction would still make broader reaper timing tests cleaner, but the requested fixes were completed with test-only process-state fixtures.
