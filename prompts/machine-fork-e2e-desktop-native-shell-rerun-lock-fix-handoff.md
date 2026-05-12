# Desktop Native Shell Rerun Lock Flake - Fix Handoff

ULTRATHINK and be meticulous. This handoff is for a fixer agent. Do not weaken or remove the regression coverage. The goal is to make the native shell test target stable under parallel Rust test execution while preserving production behavior.

## Mission

Fix the intermittent desktop native shell flake where test-only daemon hook state is overwritten or reset while another native-shell test is using it.

Primary symptom preserved by QA:

```text
test support::native_shell::desktop_main::tests::bug_pipeline_rerun_regression_releases_desktop_daemon_lock_before_ingest ... FAILED
Error: Condition failed: `*stopped_pids.lock().map_err(|_| anyhow!("test stopped-pids lock poisoned"))? == vec![4242]` ([] vs [4242])
test result: FAILED. 21 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.45s
```

Fresh handoff stress reproduced the same underlying class under forced parallelism, with a sibling symptom:

```text
ITERATION 001 test-threads=22
test tau_int_003_startup_autolaunch_reaches_full_control_from_native_bootstrap ... FAILED
Error: Condition failed: `json_mode(&snapshot) == Some("full_control")` (Some("browse_only") vs Some("full_control"))
test result: FAILED. 21 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.62s
```

The exact stopped-pids failure did not re-fire in current 1x + 20x default runs, but its original log remains present and the 22-thread run confirms native-shell test hook state is not isolated.

## Source Error Manifest

Read first:

```text
prompts/machine-fork-e2e-desktop-native-shell-rerun-lock-error-manifest.md
```

It contains the original QA failure from `146d575 docs: verify machine fork preflight repair`. Current checkout during this handoff was one docs commit later:

```text
be41325 docs: pause machine fork e2e on native shell flake
146d575 docs: verify machine fork preflight repair
```

No production code change had landed between those commits for this component.

## Current Environment Evidence

Commands run from `/Users/zacfarrell/Desktop/Testproject1`.

```bash
git status --short --branch
```

```text
## main...origin/main
M  .DS_Store
A  bin/__pycache__/br-claimcpython-313.pyc
A  bin/__pycache__/br-completecpython-313.pyc
```

These dirty entries pre-existed and must not be reverted or deleted.

```bash
uname -a
rustc --version
cargo --version
bun --version
```

```text
Darwin Mac 25.3.0 Darwin Kernel Version 25.3.0: Wed Jan 28 20:56:34 PST 2026; root:xnu-12377.91.3~2/RELEASE_ARM64_T8122 arm64
rustc 1.96.0-nightly (bcf3d36c9 2026-03-19)
cargo 1.96.0-nightly (cbb9bb8bd 2026-03-13)
1.3.13
```

Machine-fork environment:

```bash
source state-e2e-machine-fork-20260512T195251Z/e2e-env.sh
printf '%s\n' "$E2E_STATE_ROOT" "$E2E_RUNTIME_DIR" "$E2E_ARTIFACT_ROOT" "$E2E_LOG_ROOT" "$E2E_DAEMON"
```

```text
/Users/zacfarrell/Desktop/Testproject1/state-e2e-machine-fork-20260512T195251Z
/Users/zacfarrell/Desktop/Testproject1/runtime-e2e-machine-fork-20260512T195251Z
/Users/zacfarrell/Desktop/Testproject1/artifacts-e2e-machine-fork-20260512T195251Z
/Users/zacfarrell/Desktop/Testproject1/artifacts-e2e-machine-fork-20260512T195251Z/qa-logs
/tmp/testproject1-target-machine-fork-20260512T195251Z/debug/daemon
```

State database is not implicated. A handoff-time `sqlite3` check returned `PRAGMA integrity_check = ok` and zero rows in `workflow_jobs`, `agent_session_logs`, and `error_manifests`.

## Reproduction Evidence

Do not overwrite the original `baseline-desktop-native-shell.log`; it is the preserved failing gate. Use new labels when running `run_logged`.

Original exact failure:

```bash
source state-e2e-machine-fork-20260512T195251Z/e2e-env.sh
sed -n '1,220p' "$E2E_LOG_ROOT/baseline-desktop-native-shell.log"
```

Key output:

```text
test support::native_shell::desktop_main::tests::bug_pipeline_rerun_regression_releases_desktop_daemon_lock_before_ingest ... FAILED
Error: Condition failed: `*stopped_pids.lock().map_err(|_| anyhow!("test stopped-pids lock poisoned"))? == vec![4242]` ([] vs [4242])
test result: FAILED. 21 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.45s
```

Fresh one-shot rerun on current HEAD passed:

```bash
source state-e2e-machine-fork-20260512T195251Z/e2e-env.sh
run_logged handoff-repro-desktop-native-shell-baseline cargo test -p procintel-desktop --test native_shell
```

```text
running 22 tests
test support::native_shell::desktop_main::tests::bug_pipeline_rerun_regression_releases_desktop_daemon_lock_before_ingest ... ok
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.44s
```

Fresh filtered rerun passed:

```bash
source state-e2e-machine-fork-20260512T195251Z/e2e-env.sh
run_logged handoff-repro-rerun-lock-filter cargo test -p procintel-desktop --test native_shell bug_pipeline_rerun_regression_releases_desktop_daemon_lock_before_ingest
```

```text
running 1 test
test support::native_shell::desktop_main::tests::bug_pipeline_rerun_regression_releases_desktop_daemon_lock_before_ingest ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 21 filtered out; finished in 0.00s
```

Default parallel stress passed 20/20:

```bash
source state-e2e-machine-fork-20260512T195251Z/e2e-env.sh
run_logged handoff-stress-native-shell-default-20 zsh -lc 'set -o pipefail
for i in {1..20}; do
  printf "ITERATION %02d default-parallel\n" "$i"
  cargo test -p procintel-desktop --test native_shell || exit $?
done'
```

Log evidence:

```bash
source state-e2e-machine-fork-20260512T195251Z/e2e-env.sh
rg -n "ITERATION 01|ITERATION 20|test result:" "$E2E_LOG_ROOT/handoff-stress-native-shell-default-20.log"
```

All 20 iterations report:

```text
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

Serial stress passed 20/20:

```bash
source state-e2e-machine-fork-20260512T195251Z/e2e-env.sh
run_logged handoff-stress-native-shell-serial-20 zsh -lc 'set -o pipefail
for i in {1..20}; do
  printf "ITERATION %02d serial\n" "$i"
  cargo test -p procintel-desktop --test native_shell -- --test-threads=1 || exit $?
done'
```

All 20 iterations report:

```text
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

Forced high-parallel stress failed immediately:

```bash
source state-e2e-machine-fork-20260512T195251Z/e2e-env.sh
run_logged handoff-stress-native-shell-parallel22-100 zsh -lc 'set -o pipefail
for i in {1..100}; do
  printf "ITERATION %03d test-threads=22\n" "$i"
  cargo test -p procintel-desktop --test native_shell -- --test-threads=22 || exit $?
done'
```

```text
ITERATION 001 test-threads=22
test tau_int_003_startup_autolaunch_reaches_full_control_from_native_bootstrap ... FAILED
Error: Condition failed: `json_mode(&snapshot) == Some("full_control")` (Some("browse_only") vs Some("full_control"))
test result: FAILED. 21 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.62s
```

Important log paths:

```text
artifacts-e2e-machine-fork-20260512T195251Z/qa-logs/baseline-desktop-native-shell.log
artifacts-e2e-machine-fork-20260512T195251Z/qa-logs/handoff-repro-desktop-native-shell-baseline.log
artifacts-e2e-machine-fork-20260512T195251Z/qa-logs/handoff-repro-rerun-lock-filter.log
artifacts-e2e-machine-fork-20260512T195251Z/qa-logs/handoff-stress-native-shell-default-20.log
artifacts-e2e-machine-fork-20260512T195251Z/qa-logs/handoff-stress-native-shell-serial-20.log
artifacts-e2e-machine-fork-20260512T195251Z/qa-logs/handoff-stress-native-shell-parallel22-100.log
```

## Code Context

The integration test target includes production desktop code through:

```rust
// apps/desktop/src-tauri/tests/support/native_shell.rs
#[path = "../../src/main.rs"]
#[allow(dead_code)]
pub mod desktop_main;
```

The included `desktop_main` module has global test hook slots:

```rust
// apps/desktop/src-tauri/src/main.rs
#[cfg(test)]
type TestDaemonLaunchHook = Arc<dyn Fn(&Path, &Path, &Path) -> Result<()> + Send + Sync>;
#[cfg(test)]
static TEST_DAEMON_LAUNCH_HOOK: OnceLock<Mutex<Option<TestDaemonLaunchHook>>> = OnceLock::new();
#[cfg(test)]
type TestPipelineRerunHook = Arc<dyn Fn(&str) -> Result<()> + Send + Sync>;
#[cfg(test)]
static TEST_PIPELINE_RERUN_HOOK: OnceLock<Mutex<Option<TestPipelineRerunHook>>> = OnceLock::new();
#[cfg(test)]
type TestDaemonStopHook = Arc<dyn Fn(u32) -> Result<()> + Send + Sync>;
#[cfg(test)]
static TEST_DAEMON_STOP_HOOK: OnceLock<Mutex<Option<TestDaemonStopHook>>> = OnceLock::new();
#[cfg(test)]
type TestProcessRunningHook = Arc<dyn Fn(u32) -> Result<bool> + Send + Sync>;
#[cfg(test)]
static TEST_PROCESS_RUNNING_HOOK: OnceLock<Mutex<Option<TestProcessRunningHook>>> = OnceLock::new();
```

`reset_test_runtime_state()` clears every slot:

```rust
// apps/desktop/src-tauri/src/main.rs
pub fn reset_test_runtime_state() -> Result<()> {
    // Clears LAUNCH_GUARD plus TEST_DAEMON_LAUNCH_HOOK,
    // TEST_PIPELINE_RERUN_HOOK, TEST_DAEMON_STOP_HOOK,
    // and TEST_PROCESS_RUNNING_HOOK.
    Ok(())
}
```

The failing stopped-pids regression installs the process-running hook and stop hook, then expects `stop_daemon_for_exclusive_pipeline_rerun()` to record PID `4242`. The descriptor setup immediately above this excerpt writes PID `4242` into `paths.descriptor_path()`.

```rust
// apps/desktop/src-tauri/src/main.rs
#[test]
fn bug_pipeline_rerun_regression_releases_desktop_daemon_lock_before_ingest() -> Result<()> {
    let _guard = lock_test_runtime()?;
    reset_test_runtime_state()?;
    let running = Arc::new(AtomicBool::new(true));
    let running_for_check = Arc::clone(&running);
    set_test_process_running_hook(move |pid| {
        anyhow::ensure!(pid == 4242);
        Ok(running_for_check.load(Ordering::Relaxed))
    })?;

    let stopped_pids = Arc::new(Mutex::new(Vec::<u32>::new()));
    let stopped_pids_for_hook = Arc::clone(&stopped_pids);
    let running_for_stop = Arc::clone(&running);
    set_test_daemon_stop_hook(move |pid| {
        stopped_pids_for_hook
            .lock()
            .map_err(|_| anyhow!("test stopped-pids lock poisoned"))?
            .push(pid);
        running_for_stop.store(false, Ordering::Relaxed);
        Ok(())
    })?;

    anyhow::ensure!(stop_daemon_for_exclusive_pipeline_rerun(&paths)?);
    anyhow::ensure!(
        *stopped_pids
            .lock()
            .map_err(|_| anyhow!("test stopped-pids lock poisoned"))?
            == vec![4242]
    );
    anyhow::ensure!(!running.load(Ordering::Relaxed));

    reset_test_runtime_state()?;
    Ok(())
}
```

The helper routes through the hooks in test builds:

```rust
// apps/desktop/src-tauri/src/main.rs
fn process_is_running(pid: u32) -> Result<bool> {
    #[cfg(test)]
    {
        let hook = TEST_PROCESS_RUNNING_HOOK
            .get_or_init(|| Mutex::new(None))
            .lock()
            .map_err(|_| anyhow!("test process-running hook lock poisoned"))?
            .clone();
        if let Some(hook) = hook {
            return hook(pid);
        }
    }
    // Otherwise falls back to kill -0 plus zombie check.
}

fn signal_daemon_stop(pid: u32) -> Result<()> {
    #[cfg(test)]
    {
        let hook = TEST_DAEMON_STOP_HOOK
            .get_or_init(|| Mutex::new(None))
            .lock()
            .map_err(|_| anyhow!("test daemon stop hook lock poisoned"))?
            .clone();
        if let Some(hook) = hook {
            return hook(pid);
        }
    }
    // Otherwise falls back to kill -TERM.
}
```

The rerun path under test:

```rust
// apps/desktop/src-tauri/src/main.rs
fn stop_daemon_for_exclusive_pipeline_rerun(paths: &DesktopPaths) -> Result<bool> {
    let (descriptor, _) = read_descriptor(paths.descriptor_path().as_path());
    let Some(descriptor) = descriptor else {
        return Ok(false);
    };

    if !process_is_running(descriptor.pid)? {
        reset_daemon_launch_guard()?;
        return Ok(false);
    }

    signal_daemon_stop(descriptor.pid).with_context(|| {
        format!(
            "stopping daemon pid {} before pipeline rerun",
            descriptor.pid
        )
    })?;
    wait_for_daemon_stop(descriptor.pid).with_context(|| {
        format!(
            "waiting for daemon pid {} to release runtime lock",
            descriptor.pid
        )
    })?;
    reset_daemon_launch_guard()?;
    Ok(true)
}
```

The core lock mismatch:

```rust
// apps/desktop/src-tauri/src/main.rs, inside mod tests
static TEST_RUNTIME_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn lock_test_runtime() -> Result<std::sync::MutexGuard<'static, ()>> {
    TEST_RUNTIME_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .map_err(|_| anyhow!("test runtime lock poisoned"))
}
```

```rust
// apps/desktop/src-tauri/tests/native_shell.rs
struct TestStateReset;
static TEST_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();

impl TestStateReset {
    fn new() -> Result<Self> {
        desktop_main::reset_test_runtime_state()?;
        Ok(Self)
    }
}

impl Drop for TestStateReset {
    fn drop(&mut self) {
        let _ = desktop_main::reset_test_runtime_state();
    }
}
```

The `support::native_shell::desktop_main::tests::*` tests and the `tau_int_*` tests run in the same integration-test binary and share `desktop_main` hook slots, but they do not share one runtime lock.

## Root Cause Hypothesis

H1 is the working root cause: test-only global hook state is shared, but the tests that mutate it are serialized by two different locks.

Supporting evidence:

- The exact QA failure shows the stop hook side effect did not record PID `4242`.
- A filtered run of the same test passes because no sibling test can mutate the hook slots.
- `reset_test_runtime_state()` clears all global hook slots.
- `tau_int_*` tests call `desktop_main::reset_test_runtime_state()` under `TEST_MUTEX`.
- `desktop_main::tests::*` call `reset_test_runtime_state()` under `TEST_RUNTIME_LOCK`.
- Forced `--test-threads=22` produced an immediate sibling failure where `tau_int_003` installed a launch hook but observed `browse_only`, consistent with another test resetting or replacing launch hook state.

H2 remains possible but lower likelihood: `stop_daemon_for_exclusive_pipeline_rerun()` might have a false-positive path where it returns true without invoking the current stop hook. Only pursue H2 if a single shared lock does not stabilize high-parallel stress.

## Fix Strategy

Use one lock for every test in this integration target that mutates or depends on `desktop_main` test runtime state.

Recommended implementation:

1. Move `TEST_RUNTIME_LOCK` and `lock_test_runtime()` out of `mod tests` in `apps/desktop/src-tauri/src/main.rs` to top-level `#[cfg(test)]`.
2. Make the helper public within test builds:

```rust
#[cfg(test)]
#[allow(clippy::missing_errors_doc)]
pub fn lock_test_runtime() -> Result<std::sync::MutexGuard<'static, ()>> {
    TEST_RUNTIME_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .map_err(|_| anyhow!("test runtime lock poisoned"))
}
```

3. Keep the existing `desktop_main::tests::*` calls as `lock_test_runtime()?`; because `mod tests` imports `super::*`, they should resolve to the top-level helper after the local copy is removed.
4. In `apps/desktop/src-tauri/tests/native_shell.rs`, remove use of the local `TEST_MUTEX` and change every guard from:

```rust
let _guard = TEST_MUTEX
    .get_or_init(|| Mutex::new(()))
    .lock()
    .map_err(|_| anyhow!("native-shell test lock poisoned"))?;
```

to:

```rust
let _guard = desktop_main::lock_test_runtime()?;
```

5. Keep `TestStateReset` and its drop reset, but ensure every call to `TestStateReset::new()` occurs after the shared lock is acquired.
6. Do not serialize by changing global test runner config or requiring `--test-threads=1`; that hides the race instead of fixing the target.

Potential follow-up if H1 fix is insufficient:

- Add test-only tracing around `set_test_*_hook`, `process_is_running`, `signal_daemon_stop`, and `reset_test_runtime_state()` with thread IDs and test names while reproducing locally.
- If trace proves `stop_daemon_for_exclusive_pipeline_rerun()` returns true without the stop hook, inspect `process_is_running()`, `wait_for_daemon_stop()`, and stale PID/descriptor behavior.

## Validate/Fix Loop

Use this loop exactly. Do not declare fixed after one green run.

### Loop Step 1 - Reproduce before changing code

Run:

```bash
cargo test -p procintel-desktop --test native_shell -- --test-threads=22
```

If it fails, save the failure output. If it passes once, run:

```bash
for i in {1..20}; do
  cargo test -p procintel-desktop --test native_shell -- --test-threads=22 || exit 1
done
```

Current handoff baseline failed on iteration 1 with `tau_int_003`, so a fixer should expect this to be triggerable.

### Loop Step 2 - Apply the shared-lock fix

Edit only:

```text
apps/desktop/src-tauri/src/main.rs
apps/desktop/src-tauri/tests/native_shell.rs
```

Do not touch product behavior outside `#[cfg(test)]` or native-shell test guards unless H1 is rejected by evidence.

### Loop Step 3 - Fast validate

Run:

```bash
cargo test -p procintel-desktop --test native_shell bug_pipeline_rerun_regression_releases_desktop_daemon_lock_before_ingest
cargo test -p procintel-desktop --test native_shell tau_int_003_startup_autolaunch_reaches_full_control_from_native_bootstrap
cargo test -p procintel-desktop --test native_shell -- --test-threads=22
```

Expected:

```text
test result: ok
```

for each command.

### Loop Step 4 - Stress validate

Run both:

```bash
for i in {1..100}; do
  cargo test -p procintel-desktop --test native_shell -- --test-threads=22 || exit 1
done
```

```bash
for i in {1..100}; do
  cargo test -p procintel-desktop --test native_shell || exit 1
done
```

If either loop fails, do not weaken assertions. Read the failing test, inspect which hook slot or reset path it depends on, fix the isolation issue, and restart this loop from Step 3.

### Loop Step 5 - Desktop native baseline trio

Run:

```bash
cargo test -p procintel-desktop --test query_service
cargo test -p procintel-desktop --test daemon_bridge
cargo test -p procintel-desktop --test native_shell
```

### Loop Step 6 - Required quality gates

Because this is Rust code/test code, run:

```bash
cargo fmt --check
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
ubs apps/desktop/src-tauri/src/main.rs apps/desktop/src-tauri/tests/native_shell.rs
```

Fix every real error or lint and restart from Step 3.

## Acceptance Criteria

The fix is complete only when all items pass:

1. `cargo test -p procintel-desktop --test native_shell bug_pipeline_rerun_regression_releases_desktop_daemon_lock_before_ingest`
2. `cargo test -p procintel-desktop --test native_shell tau_int_003_startup_autolaunch_reaches_full_control_from_native_bootstrap`
3. `cargo test -p procintel-desktop --test native_shell -- --test-threads=22`
4. `for i in {1..100}; do cargo test -p procintel-desktop --test native_shell -- --test-threads=22 || exit 1; done`
5. `for i in {1..100}; do cargo test -p procintel-desktop --test native_shell || exit 1; done`
6. `cargo test -p procintel-desktop --test query_service`
7. `cargo test -p procintel-desktop --test daemon_bridge`
8. `cargo fmt --check`
9. `cargo check --all-targets`
10. `cargo clippy --all-targets -- -D warnings`
11. `ubs apps/desktop/src-tauri/src/main.rs apps/desktop/src-tauri/tests/native_shell.rs`

Evidence to preserve in the final response:

- Whether the exact stopped-pids failure reappeared.
- Whether `--test-threads=22` failed before the fix.
- The before/after status of the two 100x loops.
- Any code path that still required H2 investigation.

## File Map

| File | Role |
|---|---|
| `prompts/machine-fork-e2e-desktop-native-shell-rerun-lock-error-manifest.md` | Original QA forensic error manifest. |
| `apps/desktop/src-tauri/src/main.rs` | Owns test hook globals, reset function, stop helper, process/stop hook dispatch, and included unit tests. |
| `apps/desktop/src-tauri/tests/native_shell.rs` | Integration tests using a separate lock today; should use shared desktop test runtime lock. |
| `apps/desktop/src-tauri/tests/support/native_shell.rs` | Includes `src/main.rs` as `desktop_main` and builds the native Tauri harness. |
| `artifacts-e2e-machine-fork-20260512T195251Z/qa-logs/baseline-desktop-native-shell.log` | Preserved exact stopped-pids failure. |
| `artifacts-e2e-machine-fork-20260512T195251Z/qa-logs/handoff-stress-native-shell-parallel22-100.log` | Fresh high-parallel sibling failure proving hook-state isolation is broken. |

## Constraints

1. Do not delete or overwrite the existing E2E logs or state roots.
2. Do not weaken, ignore, or remove the stopped-pids regression test.
3. Do not make non-test production behavior depend on test locks.
4. Do not use `--test-threads=1` as the fix.
5. Do not revert or disturb unrelated dirty files in the shared checkout.
6. Do not add dependencies for this; `OnceLock<Mutex<()>>` is already sufficient.
