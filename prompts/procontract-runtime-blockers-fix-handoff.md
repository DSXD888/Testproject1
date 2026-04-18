# Fix ProContract Runtime Blockers + E2E Validation

ULTRATHINK and be meticulous.

## MISSION

Fix the one remaining runtime bug that prevents the ProContract pipeline from executing through the daemon, commit the already-written Codex adapter fix, then run the full three-stage ProContract pipeline end-to-end through the daemon and verify data lands correctly in SQLite. After all three `--ingest` stages complete successfully, the ProContract source is production-ready.

**Out of scope:** The nested Claude API 500 error was transient (Anthropic server-side) and does not need a fix. NHS Supply Chain source is not being pursued.

---

## PHASE 1: DEEP RESEARCH (mandatory before any code)

### 1A. The Bug — `run_bound_skill()` silent-success on missing result contract

Read `crates/daemon/src/main.rs` lines 3696-3794 — the complete `run_bound_skill()` function. Here is the bug:

```rust
// Line 3715-3720: conditional persist — only runs if result_path exists
if skill.metadata.skill_id.starts_with("procurement-ingestion/")
    && let Some(ref result_path) = executed.result_path
{
    let result_json = read_result_contract(result_path)?;
    let (source_id, persisted_count) = persist_procurement_result_for_skill(...)?;
    eprintln!("Persisted {persisted_count} ...");
}

// Line 3762-3785: UNCONDITIONALLY marks job as completed
runtime.state_db.with_write_transaction(|tx| {
    tx.execute("UPDATE workflow_jobs SET status = 'completed', ...", ...)?;
    Ok(())
})?;

Ok(LiveLoopReport {
    summary_line: format!("Skill run completed: ..."),
    status: "completed".to_owned(),
})
```

**The problem:** When a procurement skill fails upstream (agent crash, API error, timeout) and never writes `result.json`, `executed.result_path` is `None`. The if-block is skipped, zero data is persisted, but the job is marked `completed` and the function returns `status: "completed"`. The operator sees "Skill run completed" with no error. Zero notices persisted, no error manifest, no indication anything went wrong.

**Compare with the stricter live-loop path** at `crates/daemon/src/main.rs` line 5377:

```rust
// In run_procontract_discover_live_loop():
let result_path = executed
    .result_path
    .as_ref()
    .ok_or_else(|| anyhow!("procontract discovery skill did not produce a result contract"))?;
```

This path correctly errors when `result_path` is absent. The manual `--skill-path` path should do the same.

You must be able to answer:
1. What happens in the current code when a procurement skill exits without writing result.json?
2. Why is the `--ingest` live-loop path NOT affected by this bug?
3. What status should the workflow job have when no result.json exists?

### 1B. The Already-Written Fix — Codex Adapter Probe Fallback

The Codex adapter probe incompatibility has ALREADY BEEN FIXED on disk but is **uncommitted**. Check `crates/daemon/src/agents/codex.rs` — the following functions exist in the working tree but not in the last commit:

- `probe_envelope()` (line 112) — tries `codex probe --json`, catches unsupported-argument errors, falls back to `assumed_capabilities()`
- `probe_command_is_unsupported()` (line 145) — checks for `"unexpected argument '--json' found"` and related error strings
- `assumed_capabilities()` (line 152) — returns all-true capabilities like the Claude adapter's fallback
- `probe()` (line 256) — now calls `self.probe_envelope(ctx)?` instead of the old hard-failing `self.run_probe_command(ctx, &["probe", "--json"])?`

Additional improvements already on disk:
- `parse_frame_value()` — gracefully handles unknown event types and missing event/type fields (returns empty vec instead of erroring)
- `process_buffer()` — skips non-JSON lines (lines not starting with `{`)
- `build_launch_spec()` — always uses `exec --json --skip-git-repo-check`, conditional `CODEX_HOME` based on auth.json existence
- New vendor-event parsers: `parse_vendor_item()`, `parse_vendor_error()`, `classify_auth_challenge()`

Verify this by running: `git diff HEAD -- crates/daemon/src/agents/codex.rs`

You must be able to answer:
1. Is the Codex probe fallback already written and on disk?
2. Does it need any modification, or just a commit?

### 1C. The Live Loop Functions (already working)

Read these to understand how the `--ingest` path works (these are correct and don't need changes):

- `run_procontract_discover_live_loop()` — main.rs ~line 5558
- `run_procontract_register_interest_live_loop()` — main.rs ~line 5658
- `run_procontract_enrich_tender_live_loop()` — main.rs ~line 5809

Note how each of these REQUIRES `result_path` and errors if absent — unlike `run_bound_skill()`.

### 1D. Existing Tests to Preserve

Run these before and after your changes to confirm no regressions:

```bash
cargo test --bin daemon tests::persist_interest_registrations_writes_stage_tracking_rows -- --exact
cargo test --bin daemon tests::persist_tender_enrichments_updates_notice_and_documents -- --exact
```

### 1E. Error Manifest for Context

Read `prompts/procontract-runtime-blocker-error-manifest.md` — the forensic document that originally identified these blockers. It has reproduction steps, evidence bundles, and root cause analysis. Use it as reference but note that H2 (Codex probe) is already fixed on disk.

---

## PHASE 2: IMPLEMENTATION

### Fix 1: Patch `run_bound_skill()` to fail on missing procurement result contract

**File:** `crates/daemon/src/main.rs`
**Location:** `run_bound_skill()` function (~line 3696)

The fix: after the session executes, check if the skill is a procurement skill AND the result_path is None. If so, fail the job instead of silently succeeding.

**Approach:** Insert a failure branch between the session execution (line 3713) and the conditional persist block (line 3715). The logic should be:

```
if skill is a procurement-ingestion/ skill AND result_path is None:
    1. Create an error manifest (AGENT_RUN_FAILED)
    2. Mark the workflow_job as 'failed' (not 'completed')
    3. Return a LiveLoopReport with status: "failed"
```

**Key patterns to follow** (from the existing codebase):

The `fail_job` helper and `create_error_manifest` function are used throughout `main.rs`. Search for `fail_job(` to see examples. Also look at `fail_with_manifest(` for the combined error-manifest + job-failure pattern.

**What NOT to do:**
- Don't change the behavior for non-procurement skills (pdf-extraction, validation, etc.) — those may legitimately have no result.json
- Don't change the `--ingest` live-loop paths — they already handle this correctly
- Don't remove the successful-persist path — when result_path IS present, the existing persist logic is correct

### Fix 2: Commit the Codex adapter changes

The Codex adapter fix is already written. Just verify it compiles and passes tests, then include it in your commit.

### Fix 3: Add a regression test for the silent-success bug

Add a test that verifies: when a procurement skill is run via `run_bound_skill()` and the executed session has `result_path: None`, the workflow job is marked `failed` (not `completed`).

Look at existing test patterns in `main.rs` — search for `#[test]` or `#[tokio::test]` blocks that test skill execution paths.

---

## PHASE 3: VALIDATE + E2E

### Step 1: Compile and check

```bash
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
```

### Step 2: Run existing tests

```bash
cargo test --bin daemon tests::persist_interest_registrations_writes_stage_tracking_rows -- --exact
cargo test --bin daemon tests::persist_tender_enrichments_updates_notice_and_documents -- --exact
```

### Step 3: Verify the fix works — `--skill-path` manual run

```bash
# This should now fail cleanly if the nested agent doesn't produce result.json
# (instead of the old silent-success behavior)
cargo run --release --bin daemon -- --skill-path skills/procurement-ingestion/procontract-discover \
  --job-parameters-json '{"source_id":"procontract","limit":1}' \
  --state-root state --runtime-dir runtime --artifact-root artifacts
```

If the nested Claude CLI fails upstream again (transient API error), you should now see an explicit failure message, not "Skill run completed." That's the correct behavior.

If it succeeds, you should see notices persisted. Verify:
```bash
sqlite3 state/state.db "SELECT COUNT(*) FROM contract_notices WHERE source_id = 'procontract';"
```

### Step 4: Verify Codex adapter works

```bash
cargo run --release --bin daemon -- --skill-path skills/procurement-ingestion/procontract-discover \
  --job-parameters-json '{"source_id":"procontract","limit":1}' \
  --agent-cli codex \
  --state-root state --runtime-dir runtime --artifact-root artifacts
```

This should no longer fail at the probe step. It should either succeed (Codex runs the skill) or fail at the skill execution level (legitimate failure, not a probe crash).

### Step 5: Full E2E — Run all three `--ingest` stages sequentially

```bash
# Stage 1: Discovery
cargo run --release --bin daemon -- --ingest procontract \
  --state-root state --runtime-dir runtime --artifact-root artifacts

# Stage 2: Interest registration (depends on Stage 1 data)
cargo run --release --bin daemon -- --ingest procontract-interest \
  --state-root state --runtime-dir runtime --artifact-root artifacts

# Stage 3: Tender enrichment (depends on Stage 2 actions)
cargo run --release --bin daemon -- --ingest procontract-enrich \
  --state-root state --runtime-dir runtime --artifact-root artifacts
```

### Step 6: Verify data across all tables

```bash
# Discovery data
sqlite3 state/state.db "SELECT COUNT(*), source_id FROM contract_notices WHERE source_id = 'procontract' GROUP BY source_id;"

# Interest registration tracking
sqlite3 state/state.db "SELECT process_id, stage, state FROM process_stage_status WHERE stage = 'interest_registration' ORDER BY created_at DESC LIMIT 10;"

# Enrichment data + documents
sqlite3 state/state.db "SELECT id, content_type, storage_path FROM raw_documents WHERE source_id IN ('procontract', 'procontract-enrich') AND storage_path IS NOT NULL LIMIT 10;"
sqlite3 state/state.db "SELECT id, process_id, extraction_state FROM tender_document_sets WHERE process_id LIKE 'proc-pc-%' LIMIT 10;"

# Cursors persisted for all 3 sources
sqlite3 state/state.db "SELECT source_id, cursor_key, cursor_value FROM source_cursors WHERE source_id LIKE 'procontract%';"

# No orphaned records
sqlite3 state/state.db "SELECT cn.id FROM contract_notices cn LEFT JOIN procurement_processes pp ON cn.process_id = pp.id WHERE cn.source_id = 'procontract' AND pp.id IS NULL;"
```

---

## CONSTRAINTS

1. **Only fix `run_bound_skill()` for procurement skills.** Non-procurement skills (pdf-extraction, validation) may legitimately produce no result.json. Don't break those.
2. **Don't change the `--ingest` live-loop paths.** They already handle missing result contracts correctly.
3. **Don't revert the Codex adapter changes.** They're already written and correct — just verify and commit.
4. **Preserve existing test behavior.** Both persist tests must continue passing.
5. **Rust conventions:** `cargo check --all-targets && cargo clippy --all-targets -- -D warnings` must pass with zero warnings.
6. **Don't weaken failure visibility.** The whole point is making failures louder, not quieter.

---

## DELIVERABLES

1. **Patched `run_bound_skill()`** — procurement skills with no result.json are marked `failed` with an error manifest, not `completed`
2. **Committed Codex adapter fix** — the already-written `probe_envelope()` fallback and parser improvements
3. **Regression test** — verifies the silent-success bug cannot recur
4. **Clean compile + clippy** — zero warnings
5. **Successful E2E run** — all three `--ingest` stages complete with data verified in SQLite

---

## VERIFICATION

1. `cargo check --all-targets` passes
2. `cargo clippy --all-targets -- -D warnings` passes
3. Both existing persist tests pass
4. New regression test passes
5. `--skill-path` manual run with no result.json shows explicit failure (not "completed")
6. At least one `--ingest procontract` stage completes with notices in the database
7. Source cursors exist for all three procontract sources

---

## REMINDERS

- ULTRATHINK and be meticulous. Read `run_bound_skill()` completely before editing.
- The Codex adapter changes are ALREADY ON DISK — don't rewrite them, just verify and commit.
- The API 500 error was transient. If it recurs during E2E testing, just retry. Don't build retry logic.
- The blocker error manifest at `prompts/procontract-runtime-blocker-error-manifest.md` has full forensic evidence — reference it if you need reproduction steps or PTY capture locations.
- Search for `fail_job(` and `fail_with_manifest(` in main.rs to see the existing patterns for failing jobs with error manifests.
