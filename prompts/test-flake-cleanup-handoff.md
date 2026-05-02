# Pre-Existing Test Flake Cleanup (fault_process_cleanup + native_shell)

ULTRATHINK and be meticulous.

## MISSION

Two pre-existing test failure clusters at HEAD baseline have been carried forward through every recent validation manifest, blocking a clean "100% green" status:

1. **`crates/daemon/tests/fault_process_cleanup.rs`** — 3 sub-tests fail under PTY child-cleanup timing pressure: `l3_t_fault_process_cleanup_003_clean_verdict_when_descendants_exit_before_deadline` (line 110), `_004_forced_verdict_when_kill_is_required` (line 131), `_008_reliability_batch_leaves_no_known_orphans` (line 266). Confirmed pre-existing by stashing all parallel-agent WIP and re-running. Likely sensitive to OS scheduling jitter on the host.

2. **`apps/desktop/src-tauri/tests/native_shell.rs`** — 4 sub-tests fail because their welcome-message fixtures encode `schemaEpoch: 0` or `1`, but the production constant is `EXPECTED_SCHEMA_EPOCH = 2` (`apps/desktop/src-tauri/src/main.rs:51`). Affected: `tau_int_003`, `tau_int_007`, `tau_int_008`, `tau_int_010`.

This handoff fixes both clusters with surgical changes. The fault_process_cleanup work needs careful test refactoring (timing assertions → synthetic clocks OR widened tolerance with justification). The native_shell work is a fixture bump.

You will **reproduce → diagnose → fix → run 50× to verify stability → repeat for next test** until R.1–R.10 are GREEN.

```
┌────────────────────────────────────────────────────────────┐
│                          THE LOOP                            │
│   1. Pre-flight (§0): reproduce both failure clusters       │
│   2. Cluster A (native_shell): bump fixture schemaEpoch    │
│   3. Validate: 4 tests now pass; no other tests regress    │
│   4. Cluster B (fault_process_cleanup): per-test refactor  │
│   5. Loop test 50× to verify stability                     │
│   6. Both clusters green → write validation manifest        │
└────────────────────────────────────────────────────────────┘
```

Read `AGENTS.md` first.

---

## §0 — PRE-FLIGHT

| Claim | Verify |
|---|---|
| HEAD = `4ba771d`. Both test files exist | `ls -la crates/daemon/tests/fault_process_cleanup.rs apps/desktop/src-tauri/tests/native_shell.rs` |
| Baseline reproduces: native_shell fails 4/15, fault_process_cleanup fails 3/28 | `cargo test -p procintel-desktop --test native_shell 2>&1 \| tail -10; cargo test --test fault_process_cleanup 2>&1 \| tail -10` |
| `EXPECTED_SCHEMA_EPOCH = 2` at `apps/desktop/src-tauri/src/main.rs:51` | `grep -n EXPECTED_SCHEMA_EPOCH apps/desktop/src-tauri/src/main.rs \| head -2` |
| native_shell test fixtures encode `schemaEpoch: 0` or `1` somewhere; find the welcome-message construction | `grep -nE "schemaEpoch|schema_epoch" apps/desktop/src-tauri/tests/native_shell.rs \| head -10` |
| fault_process_cleanup uses real-time deadlines (not synthetic clocks); look at the failing sub-tests for `Duration::from_secs(*N*)` or similar timing patterns | `sed -n '105,170p' crates/daemon/tests/fault_process_cleanup.rs` |
| 169 daemon binary tests + 38/39 integration files baseline (the failing one is `fault_process_cleanup`) | `cargo test -p daemon --bin daemon 2>&1 \| grep "test result:"` |
| `agent-mail` healthy; commits need `AGENT_NAME=claude-code git commit ...` | inline at commit time |

**Parallel-safety**: this handoff modifies test files exclusively (`crates/daemon/tests/fault_process_cleanup.rs`, `apps/desktop/src-tauri/tests/native_shell.rs`). Safe to run alongside any handoff that doesn't touch these files. Use `git worktree add ../testproject1-flake main` for clean isolation if running concurrently with other test work.

---

## §1 — DECISIONS ALREADY MADE

### D1 — native_shell fixture bump approach

The simplest fix for native_shell is: update the fixture welcome-message to encode `schemaEpoch: 2` (matching the prod constant). The test currently asserts the daemon-handshake response carries the right schema_epoch — the failing tests construct a fake daemon response with old epoch, then assert the desktop side handles it correctly. If the desktop's behavior is "accept any epoch the daemon claims" or "reject mismatched epoch", that semantic might matter. Read the test bodies before bumping.

If a test EXPLICITLY tests the mismatch path (e.g., "desktop should error gracefully when daemon claims wrong epoch"), DON'T bump that test's fixture — instead, keep it at the older epoch and verify the test still asserts the expected error behavior. Only bump fixtures for tests that intend "happy path with current schema."

### D2 — fault_process_cleanup refactor approach

Three options per failing sub-test, in order of preference:

1. **Synthetic clock** — replace `std::time::Instant::now()` with a mockable abstraction. The existing daemon code may already have a clock abstraction (`grep -n "trait Clock\|fn now()" crates/daemon/src/`); use it. If not, this is a larger refactor — file as bead instead.

2. **Widen tolerance with justification** — if the test asserts "child exits within 100ms of signal", widen to "within 500ms" with a code comment explaining: *"Tolerance widened to absorb OS scheduling jitter on shared CI hosts. The functional contract is 'cleanup eventually succeeds', not 'cleanup completes in N ms'."* This is the pragmatic option for sub-tests where the timing IS the test (e.g., kill-on-deadline).

3. **Restructure to use process-state assertions instead of timing** — rather than "child exited within X ms", assert "child's process state transitions from Running → Exited" + use a polling loop with reasonable max wait. This is more robust than fixed deadlines but requires test refactoring.

Pick per sub-test based on what the test is actually trying to prove. Document the choice in the validation manifest.

### D3 — Stability bar: 50 consecutive passes

After fixing each test, run it in a tight loop:

```bash
for i in {1..50}; do
  cargo test --test fault_process_cleanup l3_t_fault_process_cleanup_003 -- --test-threads=1 || { echo "FAIL on iteration $i"; break; }
done
```

A test that passes 50× with `--test-threads=1` is accepted as stable. If it fails any iteration, the fix is insufficient — iterate. (50 is arbitrary but high enough to catch jitter; raise to 200 if the test is particularly suspicious.)

### D4 — Don't regress other tests

After both clusters fixed, run the FULL daemon binary suite + per-file integration loop + Tauri Rust + vitest. If any other test regresses, the fix introduced a side effect — investigate.

### D5 — No production code changes

This handoff is test code only. If a fix requires production code change (e.g., adding a Clock trait), file as bead and use the "widen tolerance" workaround instead. Production refactors for testability are a separate concern.

---

## §2 — EXECUTION ORDER

1. **Pre-flight reproduction** (§0).
2. **Cluster A: native_shell fixture bump**:
   - Read each failing test (`tau_int_003/007/008/010`) to understand intent.
   - For each: identify the welcome-message fixture, decide whether to bump epoch or preserve mismatch test.
   - Apply fix.
   - Run `cargo test -p procintel-desktop --test native_shell` until 15/15 pass.
   - Run 10 more times to verify stability.
3. **Cluster B: fault_process_cleanup**:
   - Read each failing sub-test (`_003/_004/_008`) to understand its timing assertions.
   - For each: pick fix approach per D2.
   - Apply fix.
   - Run that specific sub-test 50× per D3.
   - Once stable: move to the next.
4. **Full regression check**:
   - `cargo test -p daemon --bin daemon` → 169 / 169
   - `cargo test -p procintel-desktop --test native_shell` → 15 / 15 (was 11 / 15)
   - Per-file integration loop → all 39 PASS (was 38 / 39)
   - vitest, query_service, daemon_bridge → unchanged from pre-handoff
5. Quality gates.
6. Validation manifest.
7. Commit.

---

## §3 — REQUIRED TESTS

This handoff modifies tests. The success criterion IS the tests (3 + 4 = 7 sub-tests transitioning from FAIL → PASS).

If the fix for fault_process_cleanup adds a new helper (e.g., a `wait_for_exit_with_max(duration)` function), that helper should have its own unit test. Add as needed.

---

## §4 — ACCEPTANCE CRITERIA — R.1 to R.10

| # | Criterion |
|---|---|
| R.1 | `cargo test -p procintel-desktop --test native_shell` reports 15 passed / 0 failed (was 11 / 4) |
| R.2 | All 4 native_shell fixes pass 10/10 in a re-run loop |
| R.3 | `cargo test --test fault_process_cleanup` reports 28 passed / 0 failed (was 25 / 3) |
| R.4 | All 3 fault_process_cleanup fixes pass 50/50 in a `--test-threads=1` loop |
| R.5 | No other test regresses: 169 daemon binary, 21 query_service, 11 daemon_bridge, 35 vitest, all 39 integration files PASS |
| R.6 | If any production code was modified (should be NONE per D5), the rationale is documented |
| R.7 | Per-file integration loop now reports **39 / 39 PASS** (the previously-failing 1 is now green) |
| R.8 | The validation manifest documents per-test the fix approach used (D2 option 1, 2, or 3) and rationale |
| R.9 | If a fix used D2 option 2 (widen tolerance), the widened value is justified by data (e.g., "p99 of 50 timed runs was 312ms; widening to 500ms gives 60% headroom") |
| R.10 | Quality gates: `cargo fmt --check && cargo check --all-targets && cargo clippy --all-targets -- -D warnings` pass; if any new helper was added, it has zero clippy warnings |

---

## §5 — VALIDATION QUERIES + COMMANDS

```bash
# Reproduce baseline (pre-fix)
cargo test -p procintel-desktop --test native_shell 2>&1 | grep -E "test (tau_int_|result)"
cargo test --test fault_process_cleanup 2>&1 | grep -E "test (l3_t_fault|result)"

# After native_shell fix
cargo test -p procintel-desktop --test native_shell 2>&1 | tail -10

# After fault_process_cleanup fix (per test, 50x loop)
for i in {1..50}; do
  cargo test --test fault_process_cleanup l3_t_fault_process_cleanup_003 -- --test-threads=1 2>&1 | grep -E "test result:" || { echo "FAIL on iteration $i"; break; }
done | tail -5

# Final full sweep
cargo test -p daemon --bin daemon 2>&1 | grep "test result:"
cargo test -p procintel-desktop --test query_service 2>&1 | grep "test result:"
cargo test -p procintel-desktop --test daemon_bridge 2>&1 | grep "test result:"
cargo test -p procintel-desktop --test native_shell 2>&1 | grep "test result:"
cd apps/desktop && bunx vitest run 2>&1 | tail -5; cd ../..

# Per-file integration loop (39 / 39 expected)
for t in crates/daemon/tests/*.rs; do
  echo -n "$(basename $t .rs): "
  cargo test --test "$(basename $t .rs)" 2>&1 | grep "test result:" | head -1
done
```

---

## §6 — FAILURE-MODE PLAYBOOK

| Symptom | Cause | Fix |
|---|---|---|
| native_shell test still fails after fixture bump | The test ALSO asserts on the mismatch error path (D1 caveat) | Re-read the test. If it's an explicit "mismatch handling" test, DON'T bump — preserve the old epoch and verify the test correctly asserts the expected error behavior. Document why. |
| native_shell test passes once but fails on re-run | Some shared-state pollution between tests (TempDir not cleaned, port collision) | Add `--test-threads=1` to serialize. If still flaky, investigate fixture isolation. |
| fault_process_cleanup test passes 49/50 times | Tolerance still too narrow OR underlying issue is real (not just jitter) | Widen further OR investigate the test setup (e.g., is the child process actually being signaled in a deterministic way?) |
| Refactor to use synthetic clock requires production-side trait | Per D5, that's out of scope here | Use D2 option 2 (widen tolerance) instead, document the deferral as a follow-up bead |
| Other tests regress after fix | Side effect of test refactor (e.g., shared test-helper modified) | Localize the change. If a shared helper changed semantics, revert and inline-duplicate the fix |
| `cargo test` shows compile warnings on test files | New helpers introduced unused imports or dead code | `cargo clippy --all-targets` will catch these — fix in place |

---

## §7 — CONSTRAINTS

1. **Test code only.** No production code changes (D5).
2. **No new migrations.**
3. **No new dependencies.** Use what's already in `Cargo.toml`.
4. **Fix in place** — don't relocate tests or restructure files unless a fix demands it.
5. **Don't `#[ignore]` failing tests.** That hides the failure rather than fixing it. The whole point of this handoff is to FIX, not silence.
6. **Stability bar: 50 passes for fault_process_cleanup, 10 for native_shell.** Lower bar for native_shell because it's a deterministic fixture bump; higher bar for fault_process_cleanup because timing is inherently flaky.
7. **No silent test exclusion.** Quality gates per R.10 mandatory.

---

## §8 — DELIVERABLE

### MANDATORY: Validation manifest

Path: `docs/pipeline/test-flake-cleanup-validation-manifest.md`. Include:

```markdown
# Test Flake Cleanup — Validation Manifest

> Generated: <ISO timestamp>
> Status: GREEN / RED

## Summary
<Both clusters fixed? Stability bar met? Any other tests regressed?>

## Cluster A: native_shell (4 tests)

| Test | Fix approach | Before | After | Stability |
|---|---|---|---|---|
| tau_int_003 | D1 fixture bump epoch 1→2 | FAIL | PASS | 10/10 |
| tau_int_007 | D1 ... | | | |
| tau_int_008 | D1 ... | | | |
| tau_int_010 | D1 ... | | | |

## Cluster B: fault_process_cleanup (3 tests)

| Test | Fix approach | Before | After | Stability (50x loop) |
|---|---|---|---|---|
| _003 clean verdict | D2 option N (description) | FAIL | PASS | 50/50 |
| _004 forced verdict | D2 option N | FAIL | PASS | 50/50 |
| _008 reliability batch | D2 option N | FAIL | PASS | 50/50 |

## R.1–R.10 status
| # | Criterion | Status | Evidence |

## Quality Gates
| Gate | Result |

## Out-of-scope items (deferred beads)
<e.g., "Production-side Clock trait would enable cleaner synthetic-clock testing — filed as TC-CLOCK-1">
```

### Quality gates

```
cargo fmt --check
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test -p daemon --bin daemon              # 169 / 169
cargo test -p procintel-desktop --test query_service     # 21 / 21
cargo test -p procintel-desktop --test daemon_bridge     # 11 / 11
cargo test -p procintel-desktop --test native_shell      # 15 / 15 ← fixed
cd apps/desktop && bunx vitest run               # 35 / 35
for t in crates/daemon/tests/*.rs; do cargo test --test "$(basename "$t" .rs)" || break; done   # 39 / 39 ← fixed
```

---

## §9 — REMINDERS

- ULTRATHINK and be meticulous.
- These are PRE-EXISTING failures, not regressions from any recent handoff. The fix is overdue tech debt, not crisis response.
- For native_shell: the fix is fast IF you correctly identify which tests intend mismatch-path coverage vs happy-path coverage. Read each test before changing.
- For fault_process_cleanup: timing tests are notoriously flaky. The most defensible fix is widening tolerance with data justification, NOT trying to eliminate all jitter.
- Stability matters more than first-pass success. A test that "always passes" beats a test that "passes the first time but fails 1 in 20 runs".
- After this handoff lands, the per-file integration loop should report 39/39 and the post-merge gate sweep can finally claim "no expected_failures".
