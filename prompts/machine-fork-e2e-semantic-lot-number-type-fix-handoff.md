# Stage 4b Semantic Lot Number Type Mismatch - Fix Handoff

ULTRATHINK and be meticulous.

## Mission

Fix the machine-fork E2E blocker where one `pdf-extraction/tender-specification` semantic extraction session completed and wrote `result.json`, but the daemon failed the workflow job with:

```text
missing string `lots[].lot_number`
```

The preserved result contains numeric `lots[0].lot_number = 1`; the tender-spec skill schema requires `lot_number` to be a string, and daemon persistence currently reads it through `required_string_at`. The fix should let the Stage 4b extraction path advance without weakening the persisted domain model.

Primary forensic manifest:

```text
prompts/machine-fork-e2e-semantic-lot-number-type-error-manifest.md
```

Do not delete, recreate, or reset the preserved E2E roots. Do not rerun Stage 4a portal interest registration.

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
```

Current repo evidence captured on 2026-05-17:

```text
## main...origin/main
M  .DS_Store
A  bin/__pycache__/br-claimcpython-313.pyc
A  bin/__pycache__/br-completecpython-313.pyc
?? prompts/machine-fork-e2e-docling-runtime-unavailable-fix-handoff.md
4d45d17 docs: record semantic blocker resume check
Darwin Mac 25.3.0 arm64
cargo 1.96.0-nightly (cbb9bb8bd 2026-03-13)
bun 1.3.13
nightly-aarch64-apple-darwin
```

Treat unrelated local changes as concurrent work. Do not stash, revert, delete, or clean them.

## Recreate The Error

Use the preserved failed job, not a fresh portal run:

```bash
source state-e2e-machine-fork-20260512T195251Z/e2e-env.sh
sqlite3 -header -column "$E2E_STATE_ROOT/state.db" '
SELECT id, kind, status, last_error, input_payload, output_payload
FROM workflow_jobs
WHERE id="job-479bed51-0fea-46ee-a207-51e94c641c0b";
'
```

Expected current output:

```text
id                                        kind                        status  last_error
----------------------------------------  --------------------------  ------  ----------------------------------
job-479bed51-0fea-46ee-a207-51e94c641c0b  extract_document_semantics  failed  missing string `lots[].lot_number`
```

Confirm this is the current Stage 4b blocker, not the old Docling runtime blocker:

```bash
source state-e2e-machine-fork-20260512T195251Z/e2e-env.sh
sqlite3 -header -column "$E2E_STATE_ROOT/state.db" '
SELECT kind, status, COUNT(*) AS count
FROM workflow_jobs
WHERE kind IN ("extract_document_structure", "extract_document_semantics")
GROUP BY kind, status
ORDER BY kind, status;

SELECT COUNT(*) AS document_extractions FROM document_extractions;
SELECT COUNT(*) AS semantic_extractions FROM document_semantic_extractions;
SELECT COUNT(*) AS product_candidates FROM product_candidates;
SELECT COUNT(*) AS search_profiles FROM search_profiles;
'
```

Current output:

```text
extract_document_semantics  completed  3
extract_document_semantics  failed     1
extract_document_structure  completed  3
extract_document_structure  failed     3
document_extractions        3
semantic_extractions        3
product_candidates          5
search_profiles             0
```

The three failed structural rows are historical rows from before Docling setup. The latest structural retries completed; the open blocker is semantic persistence.

Prove the contract mismatch:

```bash
source state-e2e-machine-fork-20260512T195251Z/e2e-env.sh
jq -n -r \
  --slurpfile result "$E2E_ARTIFACT_ROOT/sessions/2026/05/session-semantic-80e2a70dc3f7de84a29da4d65e5660042ea33eb7b2a2124d17ac92408ff0da0c/workdir/result.json" \
  --slurpfile schema skills/pdf-extraction/tender-specification/result.schema.json '
    "lot_number_type=" + ($result[0].lots[0].lot_number | type),
    "lot_number_value=" + ($result[0].lots[0].lot_number | tostring),
    "schema_lots_lot_number_type=" + $schema[0].properties.lots.items.properties.lot_number.type,
    "schema_product_requirement_seeds_lot_number_type=" + $schema[0].properties.product_requirement_seeds.items.properties.lot_number.type
  '
```

Current output:

```text
lot_number_type=number
lot_number_value=1
schema_lots_lot_number_type=string
schema_product_requirement_seeds_lot_number_type=string
```

Session evidence:

```bash
source state-e2e-machine-fork-20260512T195251Z/e2e-env.sh
sqlite3 -header -column "$E2E_STATE_ROOT/state.db" '
SELECT id, job_id, skill_path, execution_mode, cli_vendor, status, exit_code, cleanup_verdict, terminal_output_bytes, terminal_output_path, run_manifest_path, created_at
FROM agent_session_logs
WHERE job_id="job-479bed51-0fea-46ee-a207-51e94c641c0b";
'
```

Key result:

```text
skill_path: pdf-extraction/tender-specification
execution_mode: contract
cli_vendor: claude
status: completed
exit_code: 0
cleanup_verdict: clean
terminal_output_bytes: 95809
```

The agent run itself completed; the daemon failed later while persisting semantic domain writes.

## Code Context

Read these files before editing:

```text
skills/pdf-extraction/tender-specification/SKILL.md
skills/pdf-extraction/tender-specification/result.schema.json
crates/daemon/src/main.rs
crates/daemon/tests/pdf_semantic_extraction.rs
crates/daemon/tests/support/pdf.rs
```

Relevant daemon path:

```text
run_extract_document_semantics_job
  -> execute_skill_session
  -> parse result.json
  -> persist_extraction_from_agent_output
  -> persist_semantic_domain_writes
  -> persist_tender_spec_domain_writes
  -> required_string_at(lot, ["lot_number"], "lots[].lot_number")
```

Current persistence code in `crates/daemon/src/main.rs`:

```rust
let lots = required_array_at(result_json, &["lots"], "lots")?;
for lot in lots {
    let lot_number_text =
        required_string_at(lot, &["lot_number"], "lots[].lot_number")?.to_owned();
    let lot_id = resolve_notice_lot_id(tx, notice_id, &lot_number_text)?;
    // ...
}
```

Current schema requires strings for both lot-number locations:

```json
"lots": {
  "items": {
    "properties": {
      "lot_number": {
        "type": "string"
      }
    }
  }
},
"product_requirement_seeds": {
  "items": {
    "properties": {
      "lot_number": {
        "type": "string"
      }
    }
  }
}
```

## Recommended Fix Strategy

Prefer a targeted canonicalization boundary for lot identifiers before semantic domain persistence:

1. Keep the public skill schema canonical: `lot_number` remains a string.
2. Add a daemon helper that accepts string lot numbers and canonicalizes JSON integer lot numbers to decimal strings for identifier fields only.
3. Reject booleans, objects, arrays, nulls, non-finite values, and fractional numbers with a clear error.
4. Use the helper in tender-spec `lots[].lot_number`.
5. Use the same helper where tender-spec product requirement seeds are filtered by `lot_number`, so numeric seed lot numbers do not get silently dropped if a future model emits them.
6. Consider the same helper for pricing-schedule and bill-of-quantities `lot_number` fields, because those overlays already default/match through lot-number text.

This keeps the persisted data model string-based while making the daemon robust to a common structured-output model slip. Do not broadly coerce every string field.

If the project owner instead wants strict schema rejection, the fix must still add schema validation before domain persistence and must produce a clearer `AGENT_OUTPUT_CONTRACT_INVALID` failure. That will not advance this preserved E2E job unless the semantic result is regenerated or corrected, so the canonicalization path is the pragmatic fix for the current blocker.

## Red Test First

Add a focused regression in `crates/daemon/tests/pdf_semantic_extraction.rs`.

Suggested test shape:

```rust
#[test]
fn test_tender_spec_numeric_lot_number_is_canonicalized_before_domain_writes() -> Result<()> {
    let harness = SemanticHarness::new()?;
    harness.seed_process("proc-numeric-lot")?;
    harness.seed_notice("proc-numeric-lot", "notice-numeric-lot", "raw-numeric-lot")?;
    harness.seed_document_extraction(
        "extract-numeric-lot",
        "raw-numeric-lot",
        "proc-numeric-lot",
        "notice-numeric-lot",
        &["pdf-extraction/tender-specification"],
    )?;
    harness.seed_semantic_job(
        "job-numeric-lot",
        "proc-numeric-lot",
        "extract-numeric-lot",
        "notice-numeric-lot",
        "pdf-extraction/tender-specification",
    )?;

    let mut payload = sample_semantic_result("pdf-extraction/tender-specification");
    payload["lots"][0]["lot_number"] = serde_json::json!(1);
    if let Some(seeds) = payload["product_requirement_seeds"].as_array_mut() {
        for seed in seeds {
            seed["lot_number"] = serde_json::json!(1);
        }
    }

    let run = harness.run_next_job(&payload)?;
    ensure!(run.success, "stderr: {}", run.stderr);

    let conn = harness.connection()?;
    ensure!(workflow_job_status(&conn, "job-numeric-lot")? == "completed");
    let lot_number: i64 = conn.query_row(
        "SELECT lot_number FROM notice_lots WHERE notice_id = ?1 LIMIT 1",
        params!["notice-numeric-lot"],
        |row| row.get(0),
    )?;
    ensure!(lot_number == 1);
    let candidate_rows: i64 = conn.query_row(
        "SELECT COUNT(*) FROM product_candidates WHERE process_id = ?1",
        params!["proc-numeric-lot"],
        |row| row.get(0),
    )?;
    ensure!(candidate_rows >= 1);
    Ok(())
}
```

Run it before fixing and confirm it fails with the current `missing string` error:

```bash
cargo test -p daemon --test pdf_semantic_extraction test_tender_spec_numeric_lot_number_is_canonicalized_before_domain_writes -- --nocapture
```

## Fix Loop

Use this loop exactly:

1. Add the red regression above.
2. Run the focused test and capture the failure.
3. Implement the smallest targeted daemon fix.
4. Re-run the focused test until it passes.
5. Run related semantic tests:

```bash
cargo test -p daemon --test pdf_semantic_extraction
cargo test -p daemon --test pdf_lot_product_refresh
cargo test -p daemon --test pdf_extraction_cascade
```

6. Run the required Rust gates for code changes:

```bash
cargo fmt --check
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
```

7. Rebuild the pinned daemon for the preserved E2E roots:

```bash
source state-e2e-machine-fork-20260512T195251Z/e2e-env.sh
env CARGO_TARGET_DIR="$E2E_TARGET_DIR" cargo build -p daemon --bin daemon
test -x "$E2E_DAEMON"
```

8. Retry the smallest preserved E2E continuation. Do not rerun Stage 4a. Prefer a single-job retry/requeue if you add or discover an existing safe helper for failed semantic jobs. If no single-job retry path exists, use the established Stage 4b resume command from the validation ledger:

```bash
source state-e2e-machine-fork-20260512T195251Z/e2e-env.sh
run_logged stage-04b-procontract-enrich-retry-after-semantic-lot-number-fix \
  env AGENT_BROWSER_CDP_PORT=9222 \
  "$E2E_DAEMON" \
  --state-root "$E2E_STATE_ROOT" \
  --runtime-dir "$E2E_RUNTIME_DIR" \
  --artifact-root "$E2E_ARTIFACT_ROOT" \
  --ingest procontract-enrich
```

9. Validate the blocker is gone:

```bash
source state-e2e-machine-fork-20260512T195251Z/e2e-env.sh
sqlite3 -header -column "$E2E_STATE_ROOT/state.db" '
SELECT id, kind, status, last_error
FROM workflow_jobs
WHERE id="job-479bed51-0fea-46ee-a207-51e94c641c0b"
   OR kind="extract_document_semantics"
ORDER BY created_at DESC, id;

SELECT COUNT(*) AS semantic_extractions FROM document_semantic_extractions;
SELECT COUNT(*) AS product_candidates FROM product_candidates;
SELECT COUNT(*) AS search_profiles FROM search_profiles;
'
```

Pass condition:

```text
No latest semantic extraction job fails with `missing string lots[].lot_number`.
The retry creates or preserves current tender-spec semantic rows.
Product candidates remain present.
Search-profile generation can advance after running the next deterministic pipeline step.
```

10. Continue the machine-fork E2E from the validation manifest resume point:

```text
docs/pipeline/machine-fork-l0-l3-stages-1-14-e2e-validation-manifest.md
```

The current resume point is Stage 4b / semantic extraction continuation before Stage 9 supplier price comparison.

## Acceptance Criteria

- A regression test fails before the fix and passes after the fix for numeric `lots[].lot_number`.
- The daemon no longer fails the semantic job with `missing string lots[].lot_number` for numeric integer lot identifiers.
- Canonical persisted lot identity remains string/text at the semantic contract boundary and numeric in `notice_lots.lot_number` only through existing `parse_optional_lot_number`.
- Existing happy-path semantic extraction, lot/product refresh, and extraction cascade tests still pass.
- The preserved E2E roots are not deleted, recreated, or manually edited.
- The validation ledger is updated with the retry command, DB evidence, and next stage.

## Files Likely To Change

```text
crates/daemon/src/main.rs
crates/daemon/tests/pdf_semantic_extraction.rs
```

Only edit skill text/schema if the code investigation proves the prompt/schema is the root contract source that must be clarified. Do not change the schema to accept broad number-or-string unions unless the project owner explicitly chooses that weaker contract.

## Final Handoff Back To QA

When fixed, report:

1. The commit hash containing the fix.
2. The red test name and before/after result.
3. The focused regression/gate commands run.
4. The exact E2E retry command run against the preserved roots.
5. The SQLite evidence showing no latest semantic job has `missing string lots[].lot_number`.
6. The next QA resume point, which should be Stage 9 supplier price comparison unless a new blocker appears.
