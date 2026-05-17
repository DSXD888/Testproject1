# Stage 9 Marketplace Coverage Completes With No Durable Attempts After CDP/Auth Repair — QA Error Manifest

ULTRATHINK and be meticulous. This is a forensic debugging document.

## DEFECT FINGERPRINT

| Field | Value |
|-------|-------|
| **Type** | wrong_behavior / silent_failure |
| **Severity** | high |
| **Reproducibility** | deterministic on the preserved machine-fork E2E state |
| **Affected Component** | `crates/daemon/src/main.rs` Stage 8/9 candidate selection and marketplace live-loop dispatch |
| **Working Analogue** | CDP/browser/auth preflight now works; Alibaba is authenticated; selected Claude CLI works |
| **Scope** | Stage 9 `--ingest marketplace-coverage` exits 0 with no marketplace jobs, no `search_attempts`, and no `supplier_quotes` even though the resume ledger expected Stage 9 to search 5 generated profiles |
| **Environment** | macOS, Chrome/148.0.7778.168 on CDP port 9222, `agent-browser 0.27.0`, SQLite schema epoch 9 |
| **Git State** | `main` at `169fb2d docs: record stage 9 alibaba auth pause` during investigation |

## REPRODUCTION PROTOCOL

Follow these steps exactly from `/Users/zacfarrell/Desktop/Testproject1`.

### Prerequisites

- Reuse the existing preserved roots. Do not create a new state root.
- Chrome must be listening on `localhost:9222`.
- Alibaba must be authenticated in the CDP browser.

```bash
source state-e2e-machine-fork-20260512T195251Z/e2e-env.sh
```

### Step 1: Confirm CDP and Alibaba auth are healthy

```bash
run_logged stage-09-cdp-version-after-alibaba-auth-repair \
  curl -sfS -m 3 http://localhost:9222/json/version

run_logged stage-09-alibaba-snapshot-after-auth-repair \
  agent-browser --cdp 9222 snapshot
```

Expected: CDP returns Chrome metadata and Alibaba shows authenticated account state.

Actual: PASS. Snapshot contains `Welcome to Alibaba.com, zac`.

### Step 2: Run the exact Stage 9 command

```bash
run_logged stage-09-marketplace-coverage-after-alibaba-auth-repair \
  env AGENT_BROWSER_CDP_PORT=9222 "$E2E_DAEMON" \
    --state-root "$E2E_STATE_ROOT" \
    --runtime-dir "$E2E_RUNTIME_DIR" \
    --artifact-root "$E2E_ARTIFACT_ROOT" \
    --ingest marketplace-coverage
```

Expected: durable `search_attempts` rows for attempted sources, `supplier_quotes` when usable prices exist, and `workflow_jobs.kind='marketplace_search'` rows.

Actual:

```text
marketplace-coverage live loop completed: marketplace-search `amazon-uk` found no candidates needing search | marketplace-search `ebay-uk` found no candidates needing search | marketplace-search `gem-imports` found no candidates needing search | marketplace-search `mx-wholesale` found no candidates needing search | marketplace-search `alibaba-search` found no candidates needing search
```

No marketplace jobs were inserted, because `run_marketplace_search_live_loop` returns before `workflow_jobs` insertion when `load_candidates_for_marketplace_search` returns an empty batch.

### Step 3: Prove missing durability

```bash
run_logged stage-09-marketplace-db-validation-after-alibaba-auth-repair \
  sqlite3 -header "$E2E_STATE_ROOT/state.db" "
SELECT source, COUNT(*) AS attempts, SUM(result_count) AS results, SUM(shortlisted_count) AS shortlisted,
       GROUP_CONCAT(DISTINCT search_failure_mode) AS failure_modes
FROM search_attempts GROUP BY source ORDER BY source;

SELECT source_name, price_signal_role, COUNT(*) AS quotes,
       MIN(unit_price_gbp_pence) AS min_gbp_pence,
       MAX(unit_price_gbp_pence) AS max_gbp_pence
FROM supplier_quotes GROUP BY source_name, price_signal_role ORDER BY source_name;

SELECT required_role, coverage_state, latest_failure_mode, COUNT(*)
FROM candidate_quote_coverage GROUP BY required_role, coverage_state, latest_failure_mode
ORDER BY required_role, coverage_state;

SELECT kind, status, COUNT(*)
FROM workflow_jobs
WHERE kind='marketplace_search'
GROUP BY kind, status
ORDER BY kind, status;
"
```

Output: no rows for all four result sets.

### Smallest retry command after a fix

```bash
source state-e2e-machine-fork-20260512T195251Z/e2e-env.sh
run_logged stage-09-marketplace-coverage-after-no-candidates-fix \
  env AGENT_BROWSER_CDP_PORT=9222 "$E2E_DAEMON" \
    --state-root "$E2E_STATE_ROOT" \
    --runtime-dir "$E2E_RUNTIME_DIR" \
    --artifact-root "$E2E_ARTIFACT_ROOT" \
    --ingest marketplace-coverage
```

Then rerun `stage-09-marketplace-db-validation-after-alibaba-auth-repair` or the same SQL under a new log label.

## EVIDENCE BUNDLE

### E0 — Environment Evidence

**E0.1: Preserved roots**

```text
REPO_ROOT=/Users/zacfarrell/Desktop/Testproject1
E2E_STATE_ROOT=/Users/zacfarrell/Desktop/Testproject1/state-e2e-machine-fork-20260512T195251Z
E2E_RUNTIME_DIR=/Users/zacfarrell/Desktop/Testproject1/runtime-e2e-machine-fork-20260512T195251Z
E2E_ARTIFACT_ROOT=/Users/zacfarrell/Desktop/Testproject1/artifacts-e2e-machine-fork-20260512T195251Z
E2E_LOG_ROOT=/Users/zacfarrell/Desktop/Testproject1/artifacts-e2e-machine-fork-20260512T195251Z/qa-logs
E2E_DAEMON=/tmp/testproject1-target-machine-fork-20260512T195251Z/debug/daemon
AGENT_BROWSER_CDP_PORT=9222
```

**E0.2: CDP repaired**

Log: `artifacts-e2e-machine-fork-20260512T195251Z/qa-logs/stage-09-cdp-version-after-alibaba-auth-repair.log`

```text
Browser Chrome/148.0.7778.168
Protocol-Version 1.3
webSocketDebuggerUrl ws://localhost:9222/devtools/browser/b74db0c3-a500-47c0-bb34-5bd7005e8729
```

Log: `artifacts-e2e-machine-fork-20260512T195251Z/qa-logs/stage-09-cdp-lsof-after-alibaba-auth-repair.log`

```text
Google PID 10039 listening on 127.0.0.1:9222
```

**E0.3: agent-browser and Claude CLI available**

Logs:

- `stage-09-agent-browser-cli-after-alibaba-auth-repair.log`
- `stage-09-claude-probe-after-alibaba-auth-repair.log`

```text
/opt/homebrew/bin/agent-browser
agent-browser 0.27.0
OK.
```

**E0.4: Alibaba authenticated after operator repair**

Log: `stage-09-alibaba-snapshot-after-auth-repair.log`

```text
heading "Welcome to Alibaba.com, zac"
Deliver to: GB
English-GBP
```

### E2 — Reproduction Evidence

**E2.1: Stage 9 command exits 0 but finds no candidates for every source**

Log: `stage-09-marketplace-coverage-after-alibaba-auth-repair.log`

```text
marketplace-coverage live loop completed: marketplace-search `amazon-uk` found no candidates needing search | marketplace-search `ebay-uk` found no candidates needing search | marketplace-search `gem-imports` found no candidates needing search | marketplace-search `mx-wholesale` found no candidates needing search | marketplace-search `alibaba-search` found no candidates needing search
```

**E2.2: Marketplace durability tables are empty**

Log: `stage-09-marketplace-db-validation-after-alibaba-auth-repair.log`

The validation query printed only the command wrapper and no result rows for:

- `search_attempts GROUP BY source`
- `supplier_quotes GROUP BY source_name, price_signal_role`
- `candidate_quote_coverage GROUP BY required_role, coverage_state, latest_failure_mode`
- `workflow_jobs WHERE kind='marketplace_search'`

**E2.3: Search profiles exist, but the Stage 9 picker equivalent returns zero**

Log: `stage-09-candidate-selection-stepwise-after-no-candidates.log`

```text
step|count
profiles_total|5
step|count
eligible_or_marginal|5
step|goods_qualified|expiry_qualified|count
notice_only_qualification_rows_for_target|0|1|1
step|count
stage9_full_picker_equivalent|0
```

**E2.4: Candidate/classification state is inconsistent with Stage 9 expectations**

Log: `stage-09-service-classification-validation-after-no-candidates.log`

```text
candidate_type|candidates
service|4
sku|1
```

The five profiles are for `BEST START FAMILY HUBS OUTREACH SERVICES`, category `321100 - Social Community Care Supplies & Services - Children`. Four candidates are classified `service`; one candidate, `Strengthening Parental Confidence through Play Based Learning`, is classified `sku`, but the process still has `notice_only goods_qualified=0`.

**E2.5: SQLite integrity is clean**

Log: `stage-09-sqlite-integrity-after-no-candidates.log`

```text
integrity_check
ok
```

`PRAGMA foreign_key_check` returned no rows.

### E3 — Codepath Evidence

**E3.1: Stage 9 source order**

File: `crates/daemon/src/main.rs:17307`

```rust
for source_id in [
    "amazon-uk",
    "ebay-uk",
    "gem-imports",
    "mx-wholesale",
    "alibaba-search",
] {
    let report = run_marketplace_search_live_loop(runtime, source_id, cli).await?;
    summaries.push(report.summary_line);
    if report.status != "completed" {
        return Ok(LiveLoopReport {
            summary_line: format!(
                "marketplace-coverage stopped at source={source_id}: {}",
                summaries.join(" | ")
            ),
            status: report.status,
        });
    }
}
```

**E3.2: Empty candidate batch returns completed before durable job insertion**

File: `crates/daemon/src/main.rs:17150`

```rust
let candidates = load_candidates_for_marketplace_search(
    runtime.state_db.connection(),
    source_id,
    MARKETPLACE_SEARCH_BATCH_SIZE,
)?;
if candidates.is_empty() {
    return Ok(LiveLoopReport {
        summary_line: format!(
            "marketplace-search `{source_id}` found no candidates needing search"
        ),
        status: "completed".to_owned(),
    });
}
```

The `workflow_jobs` insertion happens later at `crates/daemon/src/main.rs:17209`, so this path leaves no durable Stage 9 job row.

**E3.3: Candidate picker filters out this preserved state**

File: `crates/daemon/src/main.rs:14068`

```sql
WHERE sp.search_eligibility_state IN ('eligible', 'marginal')
  AND qr.qualification_basis = 'notice_only'
  AND qr.goods_qualified = 1
  AND qr.expiry_qualified = 1
  AND pc.candidate_type != 'service'
```

The preserved state has five marginal profiles, but the target process has `notice_only goods_qualified=0`, and four of five candidates are `service`.

## ROOT CAUSE HYPOTHESES

### H1: Stage 8 creates search profiles for candidates that Stage 9 can never select — LIKELIHOOD: HIGH

Description: The pipeline produced five `search_profiles` and the resume ledger treated Stage 9 as ready, but the Stage 9 picker requires a goods-qualified notice and non-service candidate. The target notice is a social-care services opportunity with `goods_qualified=0`, so Stage 9 deterministically no-ops.

Supporting evidence: E2.3, E2.4, E3.3.

Verification steps:

1. Inspect the Stage 8 search-profile generation path and confirm whether it gates on `qualification_runs.goods_qualified = 1` and `product_candidates.candidate_type != 'service'`.
2. Check whether `search_profiles=5` should have been generated for this process at all.

Fix scope: Stage 7/8 candidate classification/search-profile generation and/or deterministic pipeline continuation semantics in `crates/daemon/src/main.rs`.

### H2: Stage 9 silently treats "no selectable candidates" as success when it should persist a structured no-op/blocker — LIKELIHOOD: MEDIUM

Description: Even if the target process is legitimately not searchable, the E2E stage should not appear green after generating search profiles but writing no Stage 9 durability rows. It should either create a durable workflow/no-op record or fail with a clear structured blocker.

Supporting evidence: E2.1, E2.2, E3.2.

Verification steps:

1. Decide the intended contract for `--ingest marketplace-coverage` when `search_profiles` exist but all are filtered out.
2. Add a regression test around `run_marketplace_search_live_loop` or its candidate picker to prevent "completed with zero durable evidence" for this mismatch.

Fix scope: `run_marketplace_search_live_loop`, Stage 9 validation/reporting, and tests.

### H3: The E2E fixture/process selection is unsuitable for Stage 9 and should resume on a direct physical-goods process — LIKELIHOOD: MEDIUM

Description: `BEST START FAMILY HUBS OUTREACH SERVICES` is a service opportunity. The product extraction path generated candidate-like rows from service text, but marketplace supplier pricing is designed for physical goods. The E2E run may need to select a different preserved process or ingest a new direct goods notice instead of forcing Stage 9 on this service process.

Supporting evidence: E2.4.

Verification steps:

1. Query for other `search_profiles` or goods-qualified processes in the preserved state.
2. If none exist, decide whether the E2E must ingest a direct goods opportunity before Stage 9 can be validated.

Fix scope: E2E selection strategy and/or upstream qualification thresholds.

## INVESTIGATION ROADMAP

1. Start with H1. Trace where the five profiles were generated and why a `goods_qualified=0` process reached `search_profiles=5`.
2. Then verify H2. Decide whether Stage 9 should write a durable no-op/blocker row when it filters all existing profiles.
3. Finally verify H3. If the product behavior is correct, update the machine-fork QA flow so it does not claim Stage 9 readiness for a service notice.

## FIX CONSTRAINTS

1. Do not delete the preserved state root, runtime dir, artifact root, logs, downloads, screenshots, or existing error manifests.
2. Do not rerun Stage 4a portal registration.
3. Do not treat CDP/auth as the blocker: CDP and Alibaba auth are repaired in the current evidence.
4. Do not force marketplace searches for service candidates just to satisfy the E2E ledger.
5. Do not turn public-source no-results into failures if a real source search later writes valid structured no-result attempts.

## ACCEPTANCE CRITERIA

### Functional

- [ ] The pipeline no longer reaches "Stage 9 ready" with only non-goods/non-selectable candidates unless it records a clear structured no-op/blocker.
- [ ] Re-running the smallest retry command after the fix either writes durable `search_attempts`/quotes/coverage rows for real selectable goods candidates or records an explicit durable reason why Stage 9 is not applicable.
- [ ] `workflow_jobs` no longer makes a completed Stage 9 run indistinguishable from a skipped run when profiles exist but no candidates are selectable.

### Evidence-Based

- [ ] `stage-09-marketplace-db-validation-*` shows either durable attempted-source rows or durable structured skip/blocker evidence.
- [ ] A selector query equivalent to `load_candidates_for_marketplace_search` matches the intended Stage 8/search-profile state.
- [ ] CDP preflight still passes: `/json/version`, `lsof`, `agent-browser --cdp 9222 open`, `get title`, and `snapshot`.

### Regression

- [ ] Add or update a daemon test covering the mismatch: `search_profiles` exist for a `goods_qualified=0` process and Stage 9 must not silently complete without durable evidence.
- [ ] Existing focused daemon tests for marketplace persistence still pass.
- [ ] If code changes are made, run the repo-required Rust gates: `cargo fmt --check`, `cargo check --all-targets`, `cargo clippy --all-targets -- -D warnings`, daemon bin tests, and daemon integration tests.

## FILE MAP

| File | Role |
|------|------|
| `crates/daemon/src/main.rs` | Stage 9 candidate picker, marketplace live-loop dispatch, and empty-batch behavior |
| `docs/pipeline/machine-fork-l0-l3-stages-1-14-e2e-validation-manifest.md` | Resume ledger updated through the CDP/auth repair and this new blocker |
| `artifacts-e2e-machine-fork-20260512T195251Z/qa-logs/stage-09-marketplace-coverage-after-alibaba-auth-repair.log` | Reproduction log for the completed no-candidates Stage 9 run |
| `artifacts-e2e-machine-fork-20260512T195251Z/qa-logs/stage-09-marketplace-db-validation-after-alibaba-auth-repair.log` | Evidence that no Stage 9 durable rows exist |
| `artifacts-e2e-machine-fork-20260512T195251Z/qa-logs/stage-09-candidate-selection-stepwise-after-no-candidates.log` | Stepwise selector proof |
| `artifacts-e2e-machine-fork-20260512T195251Z/qa-logs/stage-09-service-classification-validation-after-no-candidates.log` | Candidate classification and qualification proof |
| `artifacts-e2e-machine-fork-20260512T195251Z/qa-logs/stage-09-alibaba-snapshot-after-auth-repair.log` | Alibaba authenticated browser evidence |
