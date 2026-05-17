# Stage 9 Marketplace No-Candidates Silent Success - Fix Handoff

ULTRATHINK and be meticulous.

## Mission

Fix the machine-fork E2E blocker where Stage 9 `--ingest marketplace-coverage` exits 0 after CDP and Alibaba auth are healthy, but writes no durable marketplace evidence: no `workflow_jobs.kind='marketplace_search'`, no `search_attempts`, no `supplier_quotes`, and no `candidate_quote_coverage`. The preserved state has five `search_profiles`, but Stage 9 selects zero candidates because the target process is a social-care services notice with `notice_only goods_qualified=0` and mostly `candidate_type='service'`. The fix must align Stage 8 search-profile generation and Stage 9 durable no-op semantics so a completed Stage 9 run is never indistinguishable from a skipped/non-applicable run.

Primary forensic manifest:

```text
prompts/machine-fork-e2e-stage-9-no-candidates-after-cdp-auth-error-manifest.md
```

Do not delete, recreate, or manually edit the preserved E2E roots. Do not rerun Stage 4a portal registration. Do not force marketplace searches for service/non-goods candidates just to satisfy the ledger.

## Current State

Run from:

```bash
cd /Users/zacfarrell/Desktop/Testproject1
source state-e2e-machine-fork-20260512T195251Z/e2e-env.sh
```

Preserved roots:

```text
State root:    /Users/zacfarrell/Desktop/Testproject1/state-e2e-machine-fork-20260512T195251Z
Runtime dir:   /Users/zacfarrell/Desktop/Testproject1/runtime-e2e-machine-fork-20260512T195251Z
Artifact root: /Users/zacfarrell/Desktop/Testproject1/artifacts-e2e-machine-fork-20260512T195251Z
Daemon binary: /tmp/testproject1-target-machine-fork-20260512T195251Z/debug/daemon
Target dir:    /tmp/testproject1-target-machine-fork-20260512T195251Z
```

Current repo/environment evidence captured on 2026-05-17:

```text
HEAD: 2e9fe68 docs: record stage 9 no-candidates blocker
Chrome: 148.0.7778.168 on 127.0.0.1:9222
agent-browser: 0.27.0
rustc: 1.96.0-nightly (bcf3d36c9 2026-03-19)
cargo: 1.96.0-nightly (cbb9bb8bd 2026-03-13)
bun: 1.3.13
SQLite user_version: 9
```

Treat unrelated local changes as concurrent work. Do not stash, revert, delete, clean, or overwrite them.

## Recreate The Error

First prove the blocker is no longer CDP/auth:

```bash
source state-e2e-machine-fork-20260512T195251Z/e2e-env.sh

run_logged stage-09-cdp-version-handoff-repro-20260517 \
  curl -sfS -m 3 http://localhost:9222/json/version

run_logged stage-09-cdp-lsof-handoff-repro-20260517 \
  lsof -nP -iTCP:9222 -sTCP:LISTEN

run_logged stage-09-agent-browser-version-handoff-repro-20260517 \
  agent-browser --version

run_logged stage-09-agent-browser-open-alibaba-handoff-repro-20260517 \
  agent-browser --cdp 9222 open https://www.alibaba.com

run_logged stage-09-agent-browser-get-title-handoff-repro-20260517 \
  agent-browser --cdp 9222 get title

run_logged stage-09-alibaba-snapshot-after-open-handoff-repro-20260517 \
  agent-browser --cdp 9222 snapshot
```

Current pass evidence:

```text
Browser Chrome/148.0.7778.168
Google PID 10039 listening on 127.0.0.1:9222
agent-browser 0.27.0
Alibaba.com: Manufacturers, Suppliers, Exporters & Importers from the world's largest online B2B marketplace
Snapshot contains: heading "Welcome to Alibaba.com, zac"
Snapshot contains: Deliver to / English-GBP
```

Then run the smallest failing Stage 9 command:

```bash
source state-e2e-machine-fork-20260512T195251Z/e2e-env.sh

run_logged stage-09-marketplace-coverage-handoff-repro-20260517 \
  env AGENT_BROWSER_CDP_PORT=9222 "$E2E_DAEMON" \
    --state-root "$E2E_STATE_ROOT" \
    --runtime-dir "$E2E_RUNTIME_DIR" \
    --artifact-root "$E2E_ARTIFACT_ROOT" \
    --ingest marketplace-coverage
```

Current output:

```text
marketplace-coverage live loop completed: marketplace-search `amazon-uk` found no candidates needing search | marketplace-search `ebay-uk` found no candidates needing search | marketplace-search `gem-imports` found no candidates needing search | marketplace-search `mx-wholesale` found no candidates needing search | marketplace-search `alibaba-search` found no candidates needing search
```

Prove the silent durability failure:

```bash
source state-e2e-machine-fork-20260512T195251Z/e2e-env.sh

run_logged stage-09-marketplace-db-counts-handoff-repro-20260517 \
  sqlite3 -header "$E2E_STATE_ROOT/state.db" "
SELECT 'search_attempts' AS table_name, COUNT(*) AS rows FROM search_attempts;
SELECT 'supplier_quotes' AS table_name, COUNT(*) AS rows FROM supplier_quotes;
SELECT 'candidate_quote_coverage' AS table_name, COUNT(*) AS rows FROM candidate_quote_coverage;
SELECT 'marketplace_search_workflow_jobs' AS table_name, COUNT(*) AS rows
FROM workflow_jobs WHERE kind='marketplace_search';
"
```

Current output:

```text
table_name|rows
search_attempts|0
table_name|rows
supplier_quotes|0
table_name|rows
candidate_quote_coverage|0
table_name|rows
marketplace_search_workflow_jobs|0
```

Run the grouped validation too; it currently prints no result rows:

```bash
source state-e2e-machine-fork-20260512T195251Z/e2e-env.sh

run_logged stage-09-marketplace-db-validation-handoff-repro-20260517-retry \
  sqlite3 -header -cmd ".timeout 5000" "$E2E_STATE_ROOT/state.db" "
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

SQLite integrity is not the blocker:

```bash
source state-e2e-machine-fork-20260512T195251Z/e2e-env.sh

run_logged stage-09-sqlite-integrity-handoff-repro-20260517-retry \
  sqlite3 -header -cmd ".timeout 5000" "$E2E_STATE_ROOT/state.db" \
  "PRAGMA integrity_check; PRAGMA foreign_key_check;"
```

Current output:

```text
integrity_check
ok
```

## Selector Evidence

The preserved state has profiles, but the Stage 9 picker equivalent returns zero:

```bash
source state-e2e-machine-fork-20260512T195251Z/e2e-env.sh

run_logged stage-09-candidate-selection-stepwise-handoff-repro-20260517 \
  sqlite3 -header -cmd ".timeout 5000" "$E2E_STATE_ROOT/state.db" "
SELECT 'profiles_total' AS step, COUNT(*) AS count
FROM search_profiles;
SELECT 'eligible_or_marginal' AS step, COUNT(*) AS count
FROM search_profiles
WHERE search_eligibility_state IN ('eligible','marginal');
SELECT 'notice_only_qualification_rows_for_target' AS step, goods_qualified, expiry_qualified, COUNT(*) AS count
FROM qualification_runs
WHERE process_id='proc-pc-ecd07d9f-5249-f111-813c-005056b64545'
  AND qualification_basis='notice_only'
GROUP BY goods_qualified, expiry_qualified;
SELECT 'stage9_full_picker_equivalent' AS step, COUNT(*) AS count
FROM search_profiles sp
JOIN product_candidates pc ON pc.id = sp.product_candidate_id
JOIN qualification_runs qr ON qr.process_id = pc.process_id
JOIN contract_notices cn ON cn.id = pc.notice_id
WHERE sp.search_eligibility_state IN ('eligible', 'marginal')
  AND qr.qualification_basis = 'notice_only'
  AND qr.goods_qualified = 1
  AND qr.expiry_qualified = 1
  AND pc.candidate_type != 'service'
  AND cn.title NOT LIKE '%DPS%'
  AND cn.title NOT LIKE '%Dynamic Purchasing System%'
  AND cn.title NOT LIKE '%Flexible Procurement System%'
  AND cn.title NOT LIKE '%Acceptance Round%'
  AND cn.title NOT LIKE '%Approved Provider List%'
  AND cn.title NOT LIKE '%Approved Supplier List%';
"
```

Current output:

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

Candidate classification confirms the process is not a Stage 9 physical-goods target:

```bash
source state-e2e-machine-fork-20260512T195251Z/e2e-env.sh

run_logged stage-09-service-classification-validation-handoff-repro-20260517 \
  sqlite3 -header -cmd ".timeout 5000" "$E2E_STATE_ROOT/state.db" "
SELECT pc.candidate_type, COUNT(*) AS candidates
FROM product_candidates pc
GROUP BY pc.candidate_type
ORDER BY pc.candidate_type;

SELECT pc.id, pc.name, pc.head_noun, pc.candidate_type, pc.quoteability_score_bps,
       sp.search_eligibility_state, cn.title AS notice_title, cn.main_category
FROM product_candidates pc
JOIN search_profiles sp ON sp.product_candidate_id = pc.id
JOIN contract_notices cn ON cn.id = pc.notice_id
ORDER BY pc.id;

SELECT qr.qualification_basis, qr.goods_qualified, qr.expiry_qualified, COUNT(*)
FROM qualification_runs qr
JOIN product_candidates pc ON pc.process_id = qr.process_id
GROUP BY qr.qualification_basis, qr.goods_qualified, qr.expiry_qualified;
"
```

Current key output:

```text
candidate_type|candidates
service|4
sku|1

notice_title: BEST START FAMILY HUBS OUTREACH SERVICES
main_category: 321100 - Social Community Care Supplies & Services - Children

qualification_basis|goods_qualified|expiry_qualified|COUNT(*)
lot_level|0|1|5
notice_only|0|1|5
```

## Code Context

Read these files before editing:

```text
crates/daemon/src/main.rs
crates/daemon/src/jobs/claim.rs
docs/pipeline/machine-fork-l0-l3-stages-1-14-e2e-validation-manifest.md
prompts/machine-fork-e2e-stage-9-no-candidates-after-cdp-auth-error-manifest.md
skills/marketplace-search/amazon-uk/SKILL.md
skills/marketplace-search/ebay-uk/SKILL.md
skills/marketplace-search/gem-imports/SKILL.md
skills/marketplace-search/mx-wholesale/SKILL.md
skills/marketplace-search/alibaba/SKILL.md
```

Important current types and signatures:

```rust
#[derive(Debug, Clone)]
struct LiveLoopReport {
    summary_line: String,
    status: String,
}

fn load_candidates_for_marketplace_search(
    conn: &Connection,
    source_id: &str,
    limit: i64,
) -> Result<Vec<Value>>;

async fn run_marketplace_search_live_loop(
    runtime: &mut RuntimeBootstrap,
    source_id: &str,
    cli: &Cli,
) -> Result<LiveLoopReport>;

async fn run_marketplace_coverage_live_loop(
    runtime: &mut RuntimeBootstrap,
    cli: &Cli,
) -> Result<LiveLoopReport>;

fn run_score_quoteability_stage(state_db: &mut StateDb) -> Result<usize>;
```

Current broken path in `crates/daemon/src/main.rs`:

```text
run_marketplace_coverage_live_loop
  -> for amazon-uk, ebay-uk, gem-imports, mx-wholesale, alibaba-search
  -> run_marketplace_search_live_loop
  -> load_candidates_for_marketplace_search
  -> candidates.is_empty()
  -> returns LiveLoopReport { status: "completed" }
  -> workflow_jobs insert is never reached
```

Current Stage 9 selector filters in `load_candidates_for_marketplace_search`:

```sql
WHERE sp.search_eligibility_state IN ('eligible', 'marginal')
  AND qr.qualification_basis = 'notice_only'
  AND qr.goods_qualified = 1
  AND qr.expiry_qualified = 1
  AND pc.candidate_type != 'service'
  AND cn.title NOT LIKE '%DPS%'
  AND cn.title NOT LIKE '%Dynamic Purchasing System%'
  AND cn.title NOT LIKE '%Flexible Procurement System%'
  AND cn.title NOT LIKE '%Acceptance Round%'
  AND cn.title NOT LIKE '%Approved Provider List%'
  AND cn.title NOT LIKE '%Approved Supplier List%'
  AND NOT EXISTS (
    SELECT 1 FROM search_attempts sa
    WHERE sa.product_candidate_id = pc.id
      AND sa.source = ?1
      AND sa.executed_at > datetime('now', '-7 days')
  )
```

Current Stage 8 profile emission does not apply the same goods/service gate:

```sql
SELECT pc.id, pc.process_id, pc.name, pc.head_noun,
       pc.candidate_type, pc.match_archetype,
       pc.specification_text, pc.quantity, pc.quantity_unit,
       pc.material, pc.standards_json,
       pc.critical_attributes_json, pc.numeric_specs_json
FROM product_candidates pc
WHERE EXISTS (
  SELECT 1 FROM process_stage_heads h
  WHERE h.process_id = pc.process_id
    AND h.stage = 'qualify lot'
    AND h.scope_kind = 'lot'
    AND h.scope_id = pc.lot_id
)
AND NOT EXISTS (
  SELECT 1 FROM process_stage_heads h
  WHERE h.process_id = pc.process_id
    AND h.stage = 'Score quoteability'
    AND h.scope_kind = 'candidate'
    AND h.scope_id = pc.id
)
```

`workflow_jobs.status` is free text in the schema, so a durable Stage 9 skip can use `status='skipped'` if that is the clearest contract:

```sql
CREATE TABLE workflow_jobs (
  id TEXT PRIMARY KEY,
  kind TEXT NOT NULL,
  ...
  status TEXT NOT NULL,
  ...
  output_payload TEXT,
  last_error TEXT,
  completed_at TEXT
);
```

## Recommended Fix Strategy

Implement both halves of the contract:

1. **Upstream alignment:** Stage 8 should not create or refresh marketplace `search_profiles` for candidates that Stage 9 can never select. Add the same durable eligibility boundary to `run_score_quoteability_stage`: parent qualification must be goods-qualified and expiry-qualified, and `pc.candidate_type != 'service'`. Keep this scoped to marketplace search-profile emission; do not break product-candidate extraction or qualification evidence for non-goods notices.
2. **Stage 9 durable skip:** If `load_candidates_for_marketplace_search` returns empty while eligible/marginal `search_profiles` exist but are filtered out by goods/service/direct-order/recent-attempt predicates, insert a durable `workflow_jobs` row for that source with an explicit skip payload. The row should make the no-op visible in Stage 9 evidence without pretending a marketplace search occurred.
3. **Coverage loop semantics:** Preserve full source iteration for `--ingest marketplace-coverage`. If each source records a durable skip, the coverage command may still return `LiveLoopReport.status="completed"` so all five sources are checked, but each source-level `workflow_jobs` row should have a distinct skipped/non-applicable status or output payload. Avoid making a skipped source indistinguishable from a completed searched source.

Suggested source-level skip payload:

```json
{
  "source_id": "alibaba-search",
  "status": "skipped",
  "reason_code": "NO_SELECTABLE_MARKETPLACE_CANDIDATES",
  "reason": "search_profiles exist, but none satisfy Stage 9 goods/service/direct-order selector",
  "diagnostics": {
    "search_profiles_total": 5,
    "eligible_or_marginal_profiles": 5,
    "selectable_candidates": 0,
    "excluded_non_goods": 5,
    "excluded_service": 4
  },
  "durable_step_surfaces": ["workflow_jobs", "agent_session_logs", "search_attempts", "supplier_quotes", "candidate_quote_coverage"]
}
```

Do not write `search_attempts` for a source that was never searched. Do not mark real public-source no-results as failures; if a marketplace skill actually runs and returns structured no-result attempts, those should continue to persist normally through `persist_marketplace_search_results`.

## Red Tests First

Add focused daemon unit tests in `crates/daemon/src/main.rs` under the existing `#[cfg(test)] mod tests`. These private functions are easiest to test from the binary unit test module.

### Test 1: Stage 9 persists a durable skip when profiles exist but selector filters all

Current code should fail this test because no `workflow_jobs` row is inserted on the empty candidate path.

Suggested test shape:

```rust
#[tokio::test]
async fn marketplace_search_records_skip_when_profiles_exist_but_selector_filters_all() -> Result<()> {
    let (temp, mut runtime) = bootstrap_validation_runtime()?;
    let source_id = "alibaba-search";
    let process_id = "proc-stage9-non-goods";
    let notice_id = "notice-stage9-non-goods";
    let candidate_id = "cand-stage9-service";
    let lot_id = seed_commercial_basis_fixture(&runtime.state_db, process_id, notice_id, candidate_id)?;

    runtime.state_db.connection().execute(
        "UPDATE product_candidates
         SET name = 'Community Development outreach service',
             head_noun = 'community development outreach service',
             candidate_type = 'service',
             quoteability_score_bps = 6000,
             recall_risk_score_bps = 4000
         WHERE id = ?1",
        params![candidate_id],
    )?;
    runtime.state_db.connection().execute(
        "INSERT INTO qualification_runs (
           id, process_id, notice_id, lot_id, expiry_qualified, goods_qualified,
           goods_confidence_bps, legal_category, qualification_basis, owner_id
         ) VALUES (
           'qual-stage9-non-goods', ?1, ?2, ?3, 1, 0, 1500, 'services', 'notice_only', 'local-owner'
         )",
        params![process_id, notice_id, lot_id],
    )?;
    runtime.state_db.connection().execute(
        "INSERT INTO search_profiles (
           id, product_candidate_id, head_noun, candidate_type, match_archetype,
           query_waves_json, recall_risk_score_bps, search_eligibility_state, owner_id
         ) VALUES (
           'sp-stage9-service', ?1, 'community development outreach service', 'service', 'direct',
           '[{\"wave\":1,\"query\":\"community development outreach service Alibaba\"}]',
           4000, 'marginal', 'local-owner'
         )",
        params![candidate_id],
    )?;

    let cli = Cli::parse_from([
        "daemon",
        "--serve",
        "--state-root",
        temp.path().join("state").to_str().expect("utf8 state root"),
        "--runtime-dir",
        temp.path().join("runtime").to_str().expect("utf8 runtime dir"),
        "--artifact-root",
        temp.path().join("artifacts").to_str().expect("utf8 artifact root"),
    ]);

    let report = super::run_marketplace_search_live_loop(&mut runtime, source_id, &cli).await?;
    assert_eq!(report.status, "completed");
    assert!(report.summary_line.contains("skipped") || report.summary_line.contains("no selectable"));

    let (status, output_payload): (String, String) = runtime.state_db.connection().query_row(
        "SELECT status, output_payload
         FROM workflow_jobs
         WHERE kind = 'marketplace_search' AND scope_id = ?1
         ORDER BY created_at DESC LIMIT 1",
        params![source_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    assert_eq!(status, "skipped");
    let output: Value = serde_json::from_str(&output_payload)?;
    assert_eq!(output["reason_code"], "NO_SELECTABLE_MARKETPLACE_CANDIDATES");
    assert_eq!(output["diagnostics"]["eligible_or_marginal_profiles"], json!(1));
    assert_eq!(output["diagnostics"]["selectable_candidates"], json!(0));

    let attempts: i64 = runtime.state_db.connection().query_row(
        "SELECT COUNT(*) FROM search_attempts",
        [],
        |row| row.get(0),
    )?;
    assert_eq!(attempts, 0);
    Ok(())
}
```

Adjust the fixture setup to match actual helper constraints if needed, but preserve the assertion contract: existing profile + selector mismatch must produce durable skip evidence and zero fake attempts.

Run it before fixing:

```bash
cargo test -p daemon --bin daemon marketplace_search_records_skip_when_profiles_exist_but_selector_filters_all -- --nocapture
```

Expected red result before the fix: query for `workflow_jobs.kind='marketplace_search'` returns no row or the row lacks an explicit skipped payload.

### Test 2: Stage 8 does not emit marketplace profiles for non-goods qualification

Current code should fail this test if it still creates `search_profiles` solely because a `qualify lot` stage head exists.

Suggested test shape:

```rust
#[test]
fn score_quoteability_does_not_emit_search_profile_for_non_goods_notice() -> Result<()> {
    let temp = tempdir()?;
    let db_path = temp.path().join("state.db");
    let mut state_db = open_stage14_state_db(&db_path)?;
    let process_id = "proc-stage8-non-goods";
    let notice_id = "notice-stage8-non-goods";
    let candidate_id = "cand-stage8-service";
    let lot_id = seed_commercial_basis_fixture(&state_db, process_id, notice_id, candidate_id)?;

    state_db.connection().execute(
        "UPDATE product_candidates
         SET candidate_type = 'service',
             head_noun = 'community development outreach service'
         WHERE id = ?1",
        params![candidate_id],
    )?;
    state_db.connection().execute(
        "INSERT INTO qualification_runs (
           id, process_id, notice_id, lot_id, expiry_qualified, goods_qualified,
           goods_confidence_bps, legal_category, qualification_basis, owner_id
         ) VALUES (
           'qual-stage8-non-goods', ?1, ?2, ?3, 1, 0, 1500, 'services', 'notice_only', 'local-owner'
         )",
        params![process_id, notice_id, lot_id],
    )?;

    let _scored = super::run_score_quoteability_stage(&mut state_db)?;

    let profiles: i64 = state_db.connection().query_row(
        "SELECT COUNT(*) FROM search_profiles WHERE product_candidate_id = ?1",
        params![candidate_id],
        |row| row.get(0),
    )?;
    assert_eq!(profiles, 0);
    Ok(())
}
```

Run it before fixing:

```bash
cargo test -p daemon --bin daemon score_quoteability_does_not_emit_search_profile_for_non_goods_notice -- --nocapture
```

Expected red result before the fix: one `search_profiles` row exists for the non-goods candidate.

## Fix Loop

Use this loop exactly:

1. Add the red tests in this section or equivalent focused tests with the same assertions.
2. Run both focused tests and capture the failures.
3. Implement the smallest fix in `crates/daemon/src/main.rs`.
4. Re-run the focused tests until they pass:

```bash
cargo test -p daemon --bin daemon marketplace_search_records_skip_when_profiles_exist_but_selector_filters_all -- --nocapture
cargo test -p daemon --bin daemon score_quoteability_does_not_emit_search_profile_for_non_goods_notice -- --nocapture
```

5. Run related marketplace persistence tests:

```bash
cargo test -p daemon --bin daemon marketplace_agent_failure_persists_import_coverage_attempt -- --nocapture
cargo test -p daemon --bin daemon gross_margin_single_uk_quote_is_fallback_after_import_attempt_failure -- --nocapture
cargo test -p daemon --bin daemon gross_margin_missing_import_attempt_blocks_two_sided_basis -- --nocapture
```

6. Run `ubs` on changed files before committing:

```bash
ubs crates/daemon/src/main.rs
```

7. Run the required Rust gates for code changes:

```bash
cargo fmt --check
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test -p daemon --bin daemon
for t in crates/daemon/tests/*.rs; do
  cargo test --test "$(basename "$t" .rs)" || exit 1
done
```

8. Rebuild the pinned daemon for the preserved E2E roots:

```bash
source state-e2e-machine-fork-20260512T195251Z/e2e-env.sh
env CARGO_TARGET_DIR="$E2E_TARGET_DIR" cargo build -p daemon --bin daemon
test -x "$E2E_DAEMON"
```

9. Retry Stage 9 against the same preserved roots:

```bash
source state-e2e-machine-fork-20260512T195251Z/e2e-env.sh
run_logged stage-09-marketplace-coverage-after-no-candidates-fix \
  env AGENT_BROWSER_CDP_PORT=9222 "$E2E_DAEMON" \
    --state-root "$E2E_STATE_ROOT" \
    --runtime-dir "$E2E_RUNTIME_DIR" \
    --artifact-root "$E2E_ARTIFACT_ROOT" \
    --ingest marketplace-coverage
```

10. Validate that Stage 9 is no longer silent:

```bash
source state-e2e-machine-fork-20260512T195251Z/e2e-env.sh
run_logged stage-09-marketplace-db-validation-after-no-candidates-fix \
  sqlite3 -header -cmd ".timeout 5000" "$E2E_STATE_ROOT/state.db" "
SELECT kind, scope_id, status, COUNT(*)
FROM workflow_jobs
WHERE kind='marketplace_search'
GROUP BY kind, scope_id, status
ORDER BY scope_id, status;

SELECT scope_id, status, output_payload
FROM workflow_jobs
WHERE kind='marketplace_search'
ORDER BY created_at DESC
LIMIT 10;

SELECT source, COUNT(*) AS attempts, SUM(result_count) AS results,
       GROUP_CONCAT(DISTINCT search_failure_mode) AS failure_modes
FROM search_attempts GROUP BY source ORDER BY source;

SELECT source_name, price_signal_role, COUNT(*) AS quotes
FROM supplier_quotes GROUP BY source_name, price_signal_role ORDER BY source_name;

SELECT required_role, coverage_state, latest_failure_mode, COUNT(*)
FROM candidate_quote_coverage
GROUP BY required_role, coverage_state, latest_failure_mode
ORDER BY required_role, coverage_state;
"
```

Pass condition for the preserved service-notice state:

```text
workflow_jobs contains durable marketplace_search rows for skipped/non-applicable sources, with explicit reason_code/diagnostics explaining why no candidate was selectable.
search_attempts and supplier_quotes may remain 0 because no marketplace source was actually searched.
The Stage 9 command summary mentions skipped/no-selectable state rather than implying a searched success.
```

Pass condition for a future goods-qualified state:

```text
Stage 9 still runs marketplace skills for selectable physical-goods candidates.
Real source no-result attempts persist through search_attempts.
Usable quotes persist through supplier_quotes and candidate_quote_coverage.
```

11. Update the validation ledger with the retry command, DB evidence, and next resume point:

```text
docs/pipeline/machine-fork-l0-l3-stages-1-14-e2e-validation-manifest.md
```

## Acceptance Criteria

- Stage 8 no longer emits marketplace `search_profiles` for non-goods or service-only candidates that Stage 9 can never select, unless the code intentionally documents and tests a different durable skip contract.
- Stage 9 no longer returns a completed no-candidates run with zero durable evidence when `search_profiles` exist but all are filtered out.
- `workflow_jobs.kind='marketplace_search'` distinguishes skipped/non-applicable source checks from completed searched source checks.
- No fake `search_attempts` or `supplier_quotes` are written for sources that were not actually searched.
- Existing marketplace result persistence and agent-failure persistence tests still pass.
- CDP preflight still passes on port 9222, and Alibaba still shows authenticated account state.
- The preserved roots, runtime dir, artifacts, logs, downloads, screenshots, and existing manifests are not deleted or recreated.

## Files Likely To Change

```text
crates/daemon/src/main.rs
docs/pipeline/machine-fork-l0-l3-stages-1-14-e2e-validation-manifest.md
```

Avoid creating new source files unless the local test/module structure genuinely requires it. Keep the fix in the existing daemon flow unless code review proves a small helper belongs elsewhere.

## Final Handoff Back To QA

When fixed, report:

1. The commit hash containing the fix.
2. The red test names and before/after results.
3. The focused marketplace regression commands run.
4. The full Rust gate commands run.
5. The exact Stage 9 retry command run against the preserved roots.
6. The SQLite evidence showing durable Stage 9 skip/attempt evidence.
7. Whether the next QA resume point is Stage 10 gross-margin gate or a new documented blocker.
