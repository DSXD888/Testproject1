# Desktop Native Shell Pipeline Rerun Stop Hook Flake - QA Error Manifest

ULTRATHINK and be meticulous. This is a forensic debugging document.

## DEFECT FINGERPRINT

| Field | Value |
|-------|-------|
| **Type** | race_condition / flaky_test |
| **Severity** | high |
| **Reproducibility** | intermittent: failed 1/3 observed native-shell runs; filtered rerun and second full rerun passed |
| **Affected Component** | `apps/desktop/src-tauri/src/main.rs` test hook state around `stop_daemon_for_exclusive_pipeline_rerun` |
| **Working Analogue** | The same test passes when run alone, and the full `native_shell` target passed on immediate rerun. |
| **Scope** | Required machine-fork baseline gate failed because `bug_pipeline_rerun_regression_releases_desktop_daemon_lock_before_ingest` observed no stopped PID where the test expected `[4242]`. |
| **Environment** | macOS Darwin 25.3.0 arm64, Bun 1.3.13, Cargo 1.96.0-nightly, `agent-browser` 0.27.0, Chrome CDP 148.0.7778.97 |
| **Git State** | `146d575 docs: verify machine fork preflight repair` on `main`; shared checkout also has unrelated staged `.DS_Store` and `bin/__pycache__` entries left untouched |

---

## REPRODUCTION PROTOCOL

Follow these steps exactly from the repo root.

### Prerequisites

- Repository: `/Users/zacfarrell/Desktop/Testproject1`
- Branch: `main`
- Current HEAD observed by QA: `146d575 docs: verify machine fork preflight repair`
- E2E run context: source `/Users/zacfarrell/Desktop/Testproject1/state-e2e-machine-fork-20260512T195251Z/e2e-env.sh`

### Step 1: Run the desktop native baseline command

```bash
cd /Users/zacfarrell/Desktop/Testproject1
source state-e2e-machine-fork-20260512T195251Z/e2e-env.sh
run_logged baseline-desktop-native-shell cargo test -p procintel-desktop --test native_shell
```

Expected: all 22 native shell tests pass.

Actual: 21 passed, 1 failed.

```text
test support::native_shell::desktop_main::tests::bug_pipeline_rerun_regression_releases_desktop_daemon_lock_before_ingest ... FAILED

---- support::native_shell::desktop_main::tests::bug_pipeline_rerun_regression_releases_desktop_daemon_lock_before_ingest stdout ----
Error: Condition failed: `*stopped_pids.lock().map_err(|_| anyhow!("test stopped-pids lock poisoned"))? == vec![4242]` ([] vs [4242])

failures:
    support::native_shell::desktop_main::tests::bug_pipeline_rerun_regression_releases_desktop_daemon_lock_before_ingest

test result: FAILED. 21 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.45s
```

Log:

```text
/Users/zacfarrell/Desktop/Testproject1/artifacts-e2e-machine-fork-20260512T195251Z/qa-logs/baseline-desktop-native-shell.log
```

### Step 2: Run the same test by filter

```bash
cd /Users/zacfarrell/Desktop/Testproject1
source state-e2e-machine-fork-20260512T195251Z/e2e-env.sh
run_logged repro-desktop-native-shell-rerun-lock-filter \
  cargo test -p procintel-desktop --test native_shell \
  bug_pipeline_rerun_regression_releases_desktop_daemon_lock_before_ingest
```

Expected: if deterministic, the same test fails again.

Actual: the filtered test passed.

```text
running 1 test
test support::native_shell::desktop_main::tests::bug_pipeline_rerun_regression_releases_desktop_daemon_lock_before_ingest ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 21 filtered out; finished in 0.00s
```

### Step 3: Rerun the full native shell target

```bash
cd /Users/zacfarrell/Desktop/Testproject1
source state-e2e-machine-fork-20260512T195251Z/e2e-env.sh
run_logged repro-desktop-native-shell-full-rerun \
  cargo test -p procintel-desktop --test native_shell
```

Expected: if deterministic, full target fails again.

Actual: the full target passed on rerun.

```text
running 22 tests
test support::native_shell::desktop_main::tests::bug_pipeline_rerun_regression_releases_desktop_daemon_lock_before_ingest ... ok

test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.43s
```

### Verification - Confirm Defect Present

The defect is the failed required baseline gate. The failure is preserved in the original log:

```bash
sed -n '1,220p' \
  /Users/zacfarrell/Desktop/Testproject1/artifacts-e2e-machine-fork-20260512T195251Z/qa-logs/baseline-desktop-native-shell.log
```

Output contains:

```text
Error: Condition failed: `*stopped_pids.lock().map_err(|_| anyhow!("test stopped-pids lock poisoned"))? == vec![4242]` ([] vs [4242])
```

### Control - Passing Reruns

```bash
sed -n '1,120p' \
  /Users/zacfarrell/Desktop/Testproject1/artifacts-e2e-machine-fork-20260512T195251Z/qa-logs/repro-desktop-native-shell-rerun-lock-filter.log
sed -n '1,140p' \
  /Users/zacfarrell/Desktop/Testproject1/artifacts-e2e-machine-fork-20260512T195251Z/qa-logs/repro-desktop-native-shell-full-rerun.log
```

Both logs show the previously failing test passing.

---

## EVIDENCE BUNDLE

### E0 - Environment Evidence

**E0.1: Git state**

```bash
git status --short --branch
git log --oneline -5
```

```text
## main...origin/main
M  .DS_Store
A  bin/__pycache__/br-claimcpython-313.pyc
A  bin/__pycache__/br-completecpython-313.pyc
146d575 docs: verify machine fork preflight repair
c479d1a docs: update machine fork schema verification
9323712 docs: record agent-browser cdp setup repair
6d113ad fix desktop schema epoch alignment
3251f19 docs: add schema epoch mismatch fix handoff
```

**E0.2: E2E context**

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

**E0.3: E2E state database was not implicated**

```bash
source state-e2e-machine-fork-20260512T195251Z/e2e-env.sh
sqlite3 -header "$E2E_STATE_ROOT/state.db" "
PRAGMA integrity_check;
PRAGMA foreign_key_check;
SELECT COUNT(*) AS workflow_jobs FROM workflow_jobs;
SELECT COUNT(*) AS agent_session_logs FROM agent_session_logs;
SELECT COUNT(*) AS error_manifests FROM error_manifests;
"
```

```text
integrity_check
ok
workflow_jobs
0
agent_session_logs
0
error_manifests
0
```

### E2 - Reproduction Evidence

**E2.1: Required native shell baseline failed**

```bash
source state-e2e-machine-fork-20260512T195251Z/e2e-env.sh
sed -n '1,220p' "$E2E_LOG_ROOT/baseline-desktop-native-shell.log"
```

```text
test support::native_shell::desktop_main::tests::bug_pipeline_rerun_regression_releases_desktop_daemon_lock_before_ingest ... FAILED
Error: Condition failed: `*stopped_pids.lock().map_err(|_| anyhow!("test stopped-pids lock poisoned"))? == vec![4242]` ([] vs [4242])
test result: FAILED. 21 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.45s
```

**E2.2: Filtered rerun passed**

```bash
source state-e2e-machine-fork-20260512T195251Z/e2e-env.sh
sed -n '1,120p' "$E2E_LOG_ROOT/repro-desktop-native-shell-rerun-lock-filter.log"
```

```text
running 1 test
test support::native_shell::desktop_main::tests::bug_pipeline_rerun_regression_releases_desktop_daemon_lock_before_ingest ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 21 filtered out; finished in 0.00s
```

**E2.3: Full rerun passed**

```bash
source state-e2e-machine-fork-20260512T195251Z/e2e-env.sh
sed -n '1,140p' "$E2E_LOG_ROOT/repro-desktop-native-shell-full-rerun.log"
```

```text
running 22 tests
test support::native_shell::desktop_main::tests::bug_pipeline_rerun_regression_releases_desktop_daemon_lock_before_ingest ... ok
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.43s
```

**E2.4: The failing assertion belongs to a test-only stop hook**

```bash
nl -ba apps/desktop/src-tauri/src/main.rs | sed -n '2950,3000p'
```

```text
2953	    fn bug_pipeline_rerun_regression_releases_desktop_daemon_lock_before_ingest() -> Result<()> {
2954	        let _guard = lock_test_runtime()?;
2955	        reset_test_runtime_state()?;
...
2981	        let stopped_pids = Arc::new(Mutex::new(Vec::<u32>::new()));
2982	        let stopped_pids_for_hook = Arc::clone(&stopped_pids);
2983	        let running_for_stop = Arc::clone(&running);
2984	        set_test_daemon_stop_hook(move |pid| {
2985	            stopped_pids_for_hook
2986	                .lock()
2987	                .map_err(|_| anyhow!("test stopped-pids lock poisoned"))?
2988	                .push(pid);
2989	            running_for_stop.store(false, Ordering::Relaxed);
2990	            Ok(())
2991	        })?;
2992	
2993	        anyhow::ensure!(stop_daemon_for_exclusive_pipeline_rerun(&paths)?);
2994	        anyhow::ensure!(
2995	            *stopped_pids
2996	                .lock()
2997	                .map_err(|_| anyhow!("test stopped-pids lock poisoned"))?
2998	                == vec![4242]
2999	        );
```

### E3 - Forensic Evidence

**E3.1: `stop_daemon_for_exclusive_pipeline_rerun` should call `signal_daemon_stop` after process-running hook says true**

```bash
nl -ba apps/desktop/src-tauri/src/main.rs | sed -n '1215,1240p'
```

```text
1215	fn stop_daemon_for_exclusive_pipeline_rerun(paths: &DesktopPaths) -> Result<bool> {
1216	    let (descriptor, _) = read_descriptor(paths.descriptor_path().as_path());
1217	    let Some(descriptor) = descriptor else {
1218	        return Ok(false);
1219	    };
1220	
1221	    if !process_is_running(descriptor.pid)? {
1222	        reset_daemon_launch_guard()?;
1223	        return Ok(false);
1224	    }
1225	
1226	    signal_daemon_stop(descriptor.pid).with_context(|| {
...
1238	    reset_daemon_launch_guard()?;
1239	    Ok(true)
```

**E3.2: Process-running and stop hooks are global `OnceLock<Mutex<Option<...>>>` state**

```bash
nl -ba apps/desktop/src-tauri/src/main.rs | sed -n '60,76p;280,304p;335,352p'
```

```text
70	type TestDaemonStopHook = Arc<dyn Fn(u32) -> Result<()> + Send + Sync>;
72	static TEST_DAEMON_STOP_HOOK: OnceLock<Mutex<Option<TestDaemonStopHook>>> = OnceLock::new();
74	type TestProcessRunningHook = Arc<dyn Fn(u32) -> Result<bool> + Send + Sync>;
76	static TEST_PROCESS_RUNNING_HOOK: OnceLock<Mutex<Option<TestProcessRunningHook>>> = OnceLock::new();
...
296	    let stop_hook = TEST_DAEMON_STOP_HOOK.get_or_init(|| Mutex::new(None));
297	    *stop_hook
298	        .lock()
299	        .map_err(|_| anyhow!("test daemon stop hook lock poisoned"))? = None;
301	    let running_hook = TEST_PROCESS_RUNNING_HOOK.get_or_init(|| Mutex::new(None));
302	    *running_hook
303	        .lock()
304	        .map_err(|_| anyhow!("test process-running hook lock poisoned"))? = None;
...
335	fn set_test_daemon_stop_hook<F>(hook: F) -> Result<()>
...
347	fn set_test_process_running_hook<F>(hook: F) -> Result<bool>
```

**E3.3: Real command path calls the stop-before-rerun helper**

```bash
nl -ba apps/desktop/src-tauri/src/main.rs | sed -n '2001,2020p'
```

```text
2005	fn trigger_pipeline_rerun<R: tauri::Runtime>(
...
2009	    let paths = DesktopPaths::from_handle(&app).map_err(|error| error.to_string())?;
2010	    stop_daemon_for_exclusive_pipeline_rerun(&paths).map_err(|error| error.to_string())?;
2011	    let rerun_result = trigger_pipeline_rerun_impl(&paths, &params);
2012	    let restart_result = ensure_daemon_ready_after_pipeline_rerun(&paths);
```

**E3.4: A separate native-shell test verifies a similar stop hook in an app harness**

```bash
nl -ba apps/desktop/src-tauri/tests/native_shell.rs | sed -n '170,186p'
```

```text
170	#[test]
171	fn tau_int_002_stop_background_daemon_and_quit_records_native_stop_then_exit() -> Result<()> {
...
182	    let response: String = app.invoke_typed("stop_background_daemon_and_quit", Value::Null)?;
184	    anyhow::ensure!(response == "Stop background daemon and quit requested.");
185	    anyhow::ensure!(runtime.stopped_pids()? == vec![4242]);
```

---

## CODEPATH COMPARISON

### Working Path: Filtered Test Rerun

Entry point: `apps/desktop/src-tauri/src/main.rs:2953`

Flow:
1. Acquire `lock_test_runtime()`.
2. `reset_test_runtime_state()` clears global hooks.
3. Test installs process-running hook returning true for PID `4242`.
4. Test installs stop hook that pushes PID `4242` and flips running to false.
5. `stop_daemon_for_exclusive_pipeline_rerun()` returns true.
6. `stopped_pids == [4242]`.

Observed control log: `repro-desktop-native-shell-rerun-lock-filter.log` passed.

### Broken Path: Required Full Native Shell Baseline

Entry point: same test, run as part of `cargo test -p procintel-desktop --test native_shell`

Flow:
1. Same test target starts with 22 tests.
2. The pipeline rerun stop-hook test fails once with `stopped_pids == []`.
3. The failure means either `signal_daemon_stop` was not reached, a different/no stop hook was active when signal was invoked, or the hook side effect was lost before assertion.

Observed failure log: `baseline-desktop-native-shell.log`.

### Divergence Points

| # | Passing Filtered Path | Failing Full Baseline Path | Potential Impact |
|---|---|---|---|
| 1 | Single test owns global hook state. | Full target runs many native-shell tests concurrently unless all share the same lock. | Test-only hook state may be reset or overwritten by another test. |
| 2 | Stop hook side effect records `[4242]`. | Stop hook side effect records `[]`. | Desktop rerun lock-release behavior is not proven by the baseline. |
| 3 | Full rerun later passes. | First full baseline fails. | Intermittent baseline creates unreliable release gate. |

---

## ROOT CAUSE HYPOTHESES

### H1: Test-only global hook state is shared across parallel tests without one common lock - LIKELIHOOD: HIGH

Description: `TEST_DAEMON_STOP_HOOK` and `TEST_PROCESS_RUNNING_HOOK` are global. The failing test uses `lock_test_runtime()`, while native harness tests in `apps/desktop/src-tauri/tests/native_shell.rs` use their own `TEST_MUTEX`. If Rust runs tests in parallel, another test can reset or replace global hooks while this test is between hook installation and assertion.

Supporting evidence: E2.1, E2.2, E2.3, E3.2, E3.4.

Contradicting evidence: no direct interleaving trace was captured in the failure output.

Verification steps:
1. Run the full target repeatedly with default parallelism.
2. Run the full target repeatedly with `-- --test-threads=1`.
3. If default parallelism flakes and single-threaded runs pass, H1 is confirmed.

Fix scope: unify native-shell test serialization around one lock, or isolate test hooks per harness/test instead of global `OnceLock` slots.

### H2: `stop_daemon_for_exclusive_pipeline_rerun` can return true without invoking the installed stop hook under a stale descriptor/process-running race - LIKELIHOOD: MEDIUM

Description: The helper returns true after `signal_daemon_stop` and `wait_for_daemon_stop`. The failing assertion suggests the hook did not run, but the helper still returned true. A stale hook, descriptor, or process-running check could allow a false-positive path.

Supporting evidence: E2.1, E3.1.

Contradicting evidence: filtered test and full rerun both passed.

Verification steps:
1. Instrument or log inside `process_is_running` and `signal_daemon_stop` hook branches for this test.
2. Confirm whether the failing path calls `signal_daemon_stop(4242)`.
3. Confirm whether `process_is_running(4242)` used the intended hook.

Fix scope: `stop_daemon_for_exclusive_pipeline_rerun`, `process_is_running`, `signal_daemon_stop`, and test hook lifecycle.

### H3: The failure is a one-off test scheduling artifact from a previous process/test binary state - LIKELIHOOD: LOW

Description: The immediate filtered and full reruns passed, so the first failure may depend on nondeterministic test ordering or previous test binary process state.

Supporting evidence: E2.2 and E2.3.

Contradicting evidence: Rust test process should start fresh per `cargo test` invocation; global state sharing inside the same target remains more likely.

Verification steps:
1. Run 20 iterations of `cargo test -p procintel-desktop --test native_shell`.
2. Compare failures with and without `-- --test-threads=1`.

Fix scope: test isolation and deterministic serialization.

---

## INVESTIGATION ROADMAP

### Step 1: Establish flake rate

Run:

```bash
for i in {1..20}; do
  cargo test -p procintel-desktop --test native_shell || exit 1
done
```

If it fails intermittently, record the iteration number and proceed to Step 2.

### Step 2: Test the parallelism hypothesis

Run:

```bash
for i in {1..20}; do
  cargo test -p procintel-desktop --test native_shell -- --test-threads=1 || exit 1
done
```

If single-threaded runs pass while default parallel runs fail, make test hook state serialized by one shared lock or isolate hook state per test.

### Step 3: Trace hook calls

Add temporary local logging or assertions in the failing test only:

- when `set_test_process_running_hook` is installed
- when `set_test_daemon_stop_hook` is installed
- when `process_is_running` uses the test hook
- when `signal_daemon_stop` uses the test hook

If the stop hook is not invoked in a failing run, inspect which global hook value is present at that point.

---

## FIX CONSTRAINTS

1. Do not weaken or remove the regression test; it protects desktop rerun lock release before daemon ingest.
2. Do not run product E2E validation past this baseline failure until a fixer resolves or formally classifies the flake.
3. Preserve the production behavior where desktop stops the background daemon before the exclusive rerun chain and restarts it afterward.
4. Keep test hooks unavailable in non-test builds.
5. Do not delete the E2E logs or state roots used by this manifest.

---

## ACCEPTANCE CRITERIA

1. The failing baseline command passes:

```bash
cargo test -p procintel-desktop --test native_shell
```

2. Flake stress passes under default parallelism:

```bash
for i in {1..20}; do
  cargo test -p procintel-desktop --test native_shell || exit 1
done
```

3. Desktop native baseline trio passes:

```bash
cargo test -p procintel-desktop --test query_service
cargo test -p procintel-desktop --test daemon_bridge
cargo test -p procintel-desktop --test native_shell
```

4. The machine-fork QA can resume from `Static/unit baseline: desktop native shell gate`, then continue to Stage 1 source ingestion.

---

## FILE MAP

| File | Why It Matters |
|---|---|
| `apps/desktop/src-tauri/src/main.rs` | Contains desktop rerun stop helper, test hooks, and the failing regression test. |
| `apps/desktop/src-tauri/tests/native_shell.rs` | Contains native app harness tests that also interact with daemon stop behavior. |
| `artifacts-e2e-machine-fork-20260512T195251Z/qa-logs/baseline-desktop-native-shell.log` | Failing required baseline evidence. |
| `artifacts-e2e-machine-fork-20260512T195251Z/qa-logs/repro-desktop-native-shell-rerun-lock-filter.log` | Passing filtered rerun control. |
| `artifacts-e2e-machine-fork-20260512T195251Z/qa-logs/repro-desktop-native-shell-full-rerun.log` | Passing full rerun control. |

---

## SELF-VERIFICATION

- Reproduction protocol includes exact commands and exact outputs.
- Evidence covers the failing path and passing control reruns.
- File paths and line references were read from the current checkout.
- Hypotheses are ranked and linked to evidence.
- Acceptance criteria use the same commands that failed during QA.

---

## RESOLUTION VERIFICATION

Status: resolved for QA resume by commit `9d15b9c fix: serialize native shell test hooks`.

Commands:

```bash
source state-e2e-machine-fork-20260512T195251Z/e2e-env.sh
run_logged resume-native-shell-specific \
  cargo test -p procintel-desktop --test native_shell \
  bug_pipeline_rerun_regression_releases_desktop_daemon_lock_before_ingest
run_logged resume-native-shell-full \
  cargo test -p procintel-desktop --test native_shell
run_logged resume-native-shell-stress \
  zsh -lc 'for i in {1..20}; do echo "native_shell iteration $i"; cargo test -p procintel-desktop --test native_shell >/tmp/native_shell_stress_$i.log 2>&1 || { cat /tmp/native_shell_stress_$i.log; exit 1; }; tail -5 /tmp/native_shell_stress_$i.log; done'
```

Result:

```text
resume-native-shell-specific: PASS, 1 test.
resume-native-shell-full: PASS, 22 tests.
resume-native-shell-stress: PASS, 20 full native_shell iterations.
```

QA may resume from the static/unit baseline gate and continue to live Stage 1 source ingestion.
