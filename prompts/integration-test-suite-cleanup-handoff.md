# Integration Test Suite Cleanup — Pre-existing Failures Hidden by Narrow `--bin` Test Runs

ULTRATHINK and be meticulous.

## CONTEXT

Two consecutive validation manifests (Stage 11 real-data validation, then Stage 11 hardening) declared `cargo test ... PASS - 81 tests`. That count is correct **only for the binary unit tests** (`cargo test -p daemon --bin daemon`). The 39 integration test files in `crates/daemon/tests/*.rs` were never run during validation.

A workspace-wide tally per-test-file finds **13 failing tests across 7 files, none of which are caused by Stage 11 work** but all of which block a true "fully green" claim and reduce confidence in any future validation that uses the same narrow command.

```
files_run=39 pass=1009 fail=13
  adapter_claude            :: 11 passed; 1 failed
  cron_scheduler            :: 16 passed; 1 failed
  pdf_structural_extraction :: 29 passed; 7 failed
  session_reaper            :: 32 passed; 1 failed
  skill_canary_harness      :: 10 passed; 1 failed
  skill_drift_monitor       :: 28 passed; 1 failed
  skill_lint                ::  9 passed; 1 failed
```

Your job is to fix or properly skip every one of these so that `cargo test --workspace --no-fail-fast` is green from this point on. Each issue is a small, targeted task; none requires Stage 11 knowledge.

```
┌──────────────────────────────────────────────────────────────────┐
│                          THE LOOP                                  │
│                                                                    │
│   For each failing file:                                          │
│     a. Reproduce the failure with                                 │
│        `cargo test --test <name> -- --nocapture`                  │
│     b. Identify root cause (see classifications below)            │
│     c. Apply the surgical fix                                     │
│     d. Confirm `cargo test --test <name>` is green                │
│                                                                    │
│   Final gate: run the per-file loop                               │
│   `for t in crates/daemon/tests/*.rs; do                          │
│       cargo test --test "$(basename "$t" .rs)" || break           │
│    done`                                                           │
│   and confirm pass=1022 fail=0.                                   │
└──────────────────────────────────────────────────────────────────┘
```

Read `AGENTS.md` first.

---

## ISSUE A — `adapter_claude::ut_cld_007_build_launch_spec_returns_isolated_launch_inputs` (1 failure)

### Symptom

```
Error: unexpected contract launch arguments
```

### Root cause

`crates/daemon/src/agents/claude.rs:289–295` builds contract args as:

```rust
args.push(OsString::from("--output-format"));
args.push(OsString::from("stream-json"));
args.push(OsString::from("--verbose"));
```

`crates/daemon/tests/adapter_claude.rs:354–362` asserts:

```rust
spec.args == vec![
    OsString::from("--output-format"),
    OsString::from("stream-json"),
    OsString::from("--headless"),
]
```

The `--verbose` flag was added in commit `3887e82 chore: contracts-finder skill + claude adapter improvements` (or `a81af5f Sync workspace changes`); the test was never updated. This is **pre-existing**, predates Stage 11.

### Fix

Update the test fixture to expect `["--output-format", "stream-json", "--verbose", "--headless"]`. Confirm the same flag should remain in production code (re-read the adapter's docstring or the contracts-finder commit message — `--verbose` is the right thing to do for stream-json, since it ensures every event is emitted).

### Test

`cargo test --test adapter_claude` → 12 passed, 0 failed.

---

## ISSUE B — `cron_scheduler::l3_t_cron_scheduler_001_valid_cron_expression_computes_expected_next_run_at` (1 failure)

### Symptom

```
Error: L3-T-cron_scheduler-001 expected four cron-backed definitions
```

### Root cause

The test enforces exactly 4 cron-backed definitions. The source registry now contains 5 procurement source definitions (`fts`, `contracts-finder`, `procontract`, `procontract-interest`, `procontract-enrich`) plus 7 supplier definitions. Some of the procontract-* sources also have cron schedules.

### Fix

Two options:
1. **Update the test** to count exactly the cron-backed definitions present today (likely 6: fts production + canary, contracts-finder production + canary, plus any procontract-* that have cron). Inspect `crates/daemon/src/sources/registry.rs` for `cron_expression: Some(...)` entries and re-derive the expected count and the per-key `next_run_at` map.
2. **Filter the test** to only assert on FTS + Contracts Finder schedules (its original intent). Keep the `definitions["fts::..."]` and `definitions["contracts-finder::..."]` checks; drop the bare `definitions.len() == 4` assertion or replace it with `>=4`.

Prefer (1) if procontract-* schedules are stable; (2) if they are still in flux. Document the choice in the test comment.

### Test

`cargo test --test cron_scheduler` → 17 passed, 0 failed.

---

## ISSUE C — `pdf_structural_extraction::test_*` (7 failures, environment-dependent)

### Symptom

```
Error: EXTRACTION_ENGINE_UNAVAILABLE: Docling runtime probe failed or docling is not installed
```

The 7 failing tests are all `test_corrupt_pdf_error`, `test_extract_batch`, `test_extract_success`, `test_extract_timeout`, `test_health_probe_available`, `test_oom_detection`, `test_timeout_handling`. They require a working **Docling** Python runtime which is not installed on every developer machine.

### Fix

These tests must auto-skip (not fail) when the Docling runtime probe returns `EXTRACTION_ENGINE_UNAVAILABLE`. Two acceptable patterns:

1. **Test guard**: at the top of each test, `if !docling_available() { return Ok(()); }`. Add a helper `fn docling_available() -> bool` that runs the probe once and caches the result.
2. **`#[ignore]` annotation** with a separate CI lane that opts in via `cargo test --test pdf_structural_extraction -- --include-ignored`. This is heavier-handed and risks the tests rotting; prefer (1).

In both cases, log `"PDF structural extraction tests skipped: Docling runtime not available"` once so the silent skip is visible.

### Test

`cargo test --test pdf_structural_extraction` → 36 passed, 0 failed (or 29 passed, 7 skipped if Docling isn't installed).

---

## ISSUE D — `session_reaper::it_rpr_008_crash_reconciliation_synthesizes_manifest` (calendar rollover)

### Symptom

```
assertion failed: result.manifest_path.to_string_lossy().contains("/error-manifests/2026/03/")
```

### Root cause

The test hardcodes the year-month directory `/error-manifests/2026/03/`. Today is `2026-04-30`. The error-manifest renderer derives the path from the **current** timestamp (presumably `Utc::now()` at the time of the synthesized crash), so when April rolls in, the generated path is `/error-manifests/2026/04/` and the assertion never matches.

This is a **calendar-rollover bug**. It will fail every month until the test is fixed.

### Fix

Either:
1. **Use a regex** that matches any `/error-manifests/{YYYY}/{MM}/` segment.
2. **Use today's UTC year-month** dynamically: build the expected substring from `chrono::Utc::now().format("/error-manifests/%Y/%m/")`.
3. **Inject a clock** into the reaper's manifest renderer so the test can pin the date to a known value, then assert against that.

(2) is the smallest fix; (3) is the most robust but touches production code.

### Test

`cargo test --test session_reaper` → 33 passed, 0 failed.

---

## ISSUE E — `skill_lint::ut_skl_001_required_files_exist_in_every_production_skill_directory` (1 failure)

### Symptom

```
Error: FTS and Contracts Finder must be the active procurement source skills in Layer 2
```

### Root cause

`crates/daemon/tests/skill_lint.rs:43–51` asserts:

```rust
procurement_skill_dirs == BTreeSet::from(["contracts-finder".to_owned(), "fts".to_owned()])
```

But `skills/procurement-ingestion/` already contains `procontract-discover`, `procontract-enrich-tender`, `procontract-register-interest` (added Apr 8/13/17, well before today). The test was never updated to allow those.

### Fix

Update the assertion to allow the procontract skills, OR rename the variable / split the test into separate assertions per skill that confirm each lints cleanly without enforcing the **set** is exactly two. A reasonable shape:

```rust
let expected: BTreeSet<&str> = ["contracts-finder", "fts", "procontract-discover", "procontract-enrich-tender", "procontract-register-interest"].into_iter().collect();
ensure!(procurement_skill_dirs == expected.iter().map(|s| s.to_string()).collect());
```

Then run `lint_skill_directory(...)` against each procontract skill and confirm interaction profile + required files. Add the assertion that none of them is missing critical fields.

### Test

`cargo test --test skill_lint` → 10 passed, 0 failed.

---

## ISSUE F — `skill_canary_harness::e2e_can_001_fts_canary_fetches_minimum_viable_notice_set_and_validates_basic_ocds_structure` (1 failure)

### Symptom

```
Error: FTS must be the only seeded procurement source
```

### Root cause

The canary harness seeds the source registry as part of its e2e fixture and asserts that FTS is the **only** procurement source. Once the procontract-* sources were added to the registry (and possibly contracts-finder), this assertion broke.

### Fix

Update the assertion to allow the canonical set of procurement sources (FTS, Contracts Finder, plus the three procontract-* skills if they're still seeded as default). Or scope the canary to seed **only** FTS for this test by explicitly filtering the registry. Prefer the explicit-filter approach so the canary is hermetic.

### Test

`cargo test --test skill_canary_harness` → 11 passed, 0 failed.

---

## ISSUE G — `skill_drift_monitor::l3_t_skill_drift_monitor_002_canary_cadence_defaults_and_outcomes_persist_explicitly` (1 failure)

### Symptom

```
Error: L3-T-skill_drift_monitor-002 expected explicit persisted canary outcomes
```

### Root cause

Expected persisted canary outcomes is missing rows for the procontract skills (or any skill added after the test was written). The test seeds canary outcomes for the original 4 procurement sources and asserts a specific count or set — but the registry now has more.

### Fix

Read the test source (`crates/daemon/tests/skill_drift_monitor.rs`) and either widen the expected outcome set, or filter the source list in the test setup the same way as Issue F. The same-shape fix as F — reference it.

### Test

`cargo test --test skill_drift_monitor` → 29 passed, 0 failed.

---

## DELIVERABLE

### MANDATORY: Cleanup manifest

Path: `docs/pipeline/integration-test-cleanup-manifest.md`. For each issue A–G:

| # | Issue | Status | Tests fixed | Lines touched |
|---|---|---|---|---|

Append a "Final per-file tally" section showing the output of:

```bash
for t in crates/daemon/tests/*.rs; do
  name=$(basename "$t" .rs)
  result=$(cargo test --test "$name" 2>&1 | grep -E "^test result:" | head -1)
  echo "$name :: $result"
done
```

Expect `pass=1022 fail=0` (or higher if new tests landed during this work).

### Quality gates

```
cargo fmt --check
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test -p daemon --bin daemon                       # binary unit tests
for t in crates/daemon/tests/*.rs; do                   # all integration tests
  cargo test --test "$(basename "$t" .rs)" || exit 1
done
```

The two test commands above are the **new minimum** for any future Stage validation manifest. Do not declare green based on `--bin` alone again.

---

## CONSTRAINTS

1. **Do not modify Stage 11 code or the four hardening fixes.** This is purely test-suite cleanup.
2. **Do not change skill production behavior to satisfy a test.** Always update the test, not the production behavior, unless the production behavior is genuinely broken.
3. **No `#[ignore]` for tests that *should* run on a clean machine.** Only Issue C (Docling) qualifies — every other failure must be properly fixed.
4. **Issues B, E, F, G all share the same shape**: a hardcoded list of skills/sources that doesn't match the current registry. Solve them with the same idiomatic pattern; don't reinvent the wheel three times.

---

## REMINDERS

- ULTRATHINK and be meticulous.
- The Stage 11 hardening manifest's GREEN status is genuinely accurate for what it tested. The problem is **scope**: integration tests were silently excluded. After this work, future manifests must run both the binary and per-file integration tests before claiming green.
- Once this is done, the chain Stage 1 → Stage 11 is fully green by every meaningful gate, and Stage 12 can be started without any test-suite debt rolling forward.
