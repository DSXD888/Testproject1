# Stage 4b Semantic Lot Number Type Mismatch - QA Error Manifest

ULTRATHINK and be meticulous. This is a forensic debugging document.

## DEFECT FINGERPRINT

| Field | Value |
|-------|-------|
| Type | integration_mismatch / contract_validation |
| Severity | medium |
| Reproducibility | deterministic from preserved Stage 4b retry artifacts |
| Affected Component | `skills/pdf-extraction/tender-specification/result.schema.json`, `crates/daemon/src/main.rs::persist_tender_spec_domain_writes`, `crates/daemon/src/main.rs::required_string_at` |
| Working Analogue | Two pricing semantic extraction jobs completed; one tender-specification job against Part B completed and wrote product candidates. |
| Scope | One `pdf-extraction/tender-specification` semantic extraction job failed after Docling was repaired because the agent output used numeric `lots[].lot_number` while the daemon/schema contract requires a string. |
| Environment | macOS 26.3.1 arm64; Cargo 1.96 nightly; Bun 1.3.13; Python 3.13.12; Docling 2.93.0 in `/Users/zacfarrell/.docling-venv/bin/python` |
| Git State | `main` at `8279ad6 docs: record docling extraction blocker`; shared checkout has unrelated staged `.DS_Store` and `bin/__pycache__/*` artifacts |

---

## REPRODUCTION PROTOCOL

Follow these steps exactly from `/Users/zacfarrell/Desktop/Testproject1`. Do not delete or recreate the preserved E2E roots.

### Prerequisites

- Existing E2E state root: `/Users/zacfarrell/Desktop/Testproject1/state-e2e-machine-fork-20260512T195251Z`
- Existing E2E artifact root: `/Users/zacfarrell/Desktop/Testproject1/artifacts-e2e-machine-fork-20260512T195251Z`
- The Docling setup retry has already run and created semantic job `job-479bed51-0fea-46ee-a207-51e94c641c0b`.

### Step 1: Confirm the semantic job failure

```bash
source state-e2e-machine-fork-20260512T195251Z/e2e-env.sh
sqlite3 -header -column "$E2E_STATE_ROOT/state.db" "
SELECT id, kind, status, scope_kind, scope_id, attempt_count, last_error, created_at, started_at, completed_at
FROM workflow_jobs
WHERE kind='extract_document_semantics'
ORDER BY created_at DESC, id;
"
```

Expected: every semantic extraction job for the retried documents should complete or produce a structured non-contract blocker.

Actual:

```text
id                                        kind                        status     scope_kind  scope_id                                 attempt_count  last_error                          created_at           started_at                        completed_at
----------------------------------------  --------------------------  ---------  ----------  ---------------------------------------  -------------  ----------------------------------  -------------------  --------------------------------  --------------------------------
job-3924181c-a842-4769-9014-c55ae0d1cfbd  extract_document_semantics  completed  notice      pc-ecd07d9f-5249-f111-813c-005056b64545  1                                                  2026-05-16 15:31:54  2026-05-16T15:40:48.614711+00:00  2026-05-16T15:47:22.106474+00:00
job-7eb6d5d0-4506-4bbf-a0be-59c3ef29d1f5  extract_document_semantics  completed  notice      pc-ecd07d9f-5249-f111-813c-005056b64545  1                                                  2026-05-16 15:31:54  2026-05-16T15:47:22.106969+00:00  2026-05-16T15:51:56.708826+00:00
job-27375307-f3d7-46f9-bb67-ed71f4ea406a  extract_document_semantics  completed  notice      pc-ecd07d9f-5249-f111-813c-005056b64545  1                                                  2026-05-16 15:31:50  2026-05-16T15:37:45.612138+00:00  2026-05-16T15:40:48.613299+00:00
job-479bed51-0fea-46ee-a207-51e94c641c0b  extract_document_semantics  failed     notice      pc-ecd07d9f-5249-f111-813c-005056b64545  1              missing string `lots[].lot_number`  2026-05-16 15:31:50  2026-05-16T15:31:58.248733+00:00  2026-05-16T15:37:45.609782+00:00
```

### Step 2: Inspect the failed job input/session

```bash
source state-e2e-machine-fork-20260512T195251Z/e2e-env.sh
sqlite3 -header -column "$E2E_STATE_ROOT/state.db" "
SELECT id, status, last_error, substr(input_payload,1,1200) AS input_payload_prefix, substr(output_payload,1,1200) AS output_payload_prefix
FROM workflow_jobs
WHERE id='job-479bed51-0fea-46ee-a207-51e94c641c0b';

SELECT id, job_id, skill_path, execution_mode, cli_vendor, status, exit_code, cleanup_verdict, terminal_output_bytes, terminal_output_path, run_manifest_path, created_at
FROM agent_session_logs
WHERE job_id='job-479bed51-0fea-46ee-a207-51e94c641c0b';
"
```

Actual:

```text
id                                        status  last_error                          input_payload_prefix                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    output_payload_prefix
----------------------------------------  ------  ----------------------------------  --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------  -------------------------------------------------------------------------
job-479bed51-0fea-46ee-a207-51e94c641c0b  failed  missing string `lots[].lot_number`  {"request":{"request":{"document_extraction_id":"35c697a6-6970-4391-9f83-9767e9115ae2","document_set_id":"tds-pc-ecd07d9f-5249-f111-813c-005056b64545","extraction_scope":"full_document","notice_id":"pc-ecd07d9f-5249-f111-813c-005056b64545","process_id":"proc-pc-ecd07d9f-5249-f111-813c-005056b64545","raw_document_id":"raw-pc-ecd07d9f-5249-f111-813c-005056b64545-0e7e8e1d05fb47625925cbdc743cefcd692954904ca9b9b0257d47c4fb27302b","skill_id":"pdf-extraction/tender-specification","stage_run_id":"stage-run-c497cbd5-d294-4b91-a504-f43a480df8f5-proc-pc-ecd07d9f-5249-f111-813c-005056b64545","target_domain":"lots"}},"runner_meta":{"executor_id":"local-daemon","idempotency_key":"extract_document_structure::job-dc189357-13ee-4869-bb15-039bfda14e78::pdf-extraction/tender-specification","visible_after_seq":53}}  {"error_code":"AGENT_RUN_FAILED","job_kind":"extract_document_semantics"}

id                                                                                 job_id                                    skill_path                           execution_mode  cli_vendor  status     exit_code  cleanup_verdict  terminal_output_bytes  terminal_output_path                                                                                                                                                                                              run_manifest_path  created_at
---------------------------------------------------------------------------------  ----------------------------------------  -----------------------------------  --------------  ----------  ---------  ---------  ---------------  ---------------------  ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------  -----------------  -------------------
session-semantic-80e2a70dc3f7de84a29da4d65e5660042ea33eb7b2a2124d17ac92408ff0da0c  job-479bed51-0fea-46ee-a207-51e94c641c0b  pdf-extraction/tender-specification  contract        claude      completed  0          clean            95809                  /Users/zacfarrell/Desktop/Testproject1/artifacts-e2e-machine-fork-20260512T195251Z/sessions/2026/05/session-semantic-80e2a70dc3f7de84a29da4d65e5660042ea33eb7b2a2124d17ac92408ff0da0c/contract-output.index.json                     2026-05-16 15:37:45
```

### Step 3: Prove the result shape violates daemon expectations

```bash
source state-e2e-machine-fork-20260512T195251Z/e2e-env.sh
jq -r '"keys=" + ((keys|sort)|join(",")), "lots_len=" + ((.lots|length)|tostring), (.lots[:5][]? | "lot_keys=" + ((keys|sort)|join(",")) + " lot_number=" + ((.lot_number // "<missing>")|tostring) + " title=" + (.title // "<missing>")), "seeds_len=" + ((.product_requirement_seeds|length)|tostring)' \
  "$E2E_ARTIFACT_ROOT/sessions/2026/05/session-semantic-80e2a70dc3f7de84a29da4d65e5660042ea33eb7b2a2124d17ac92408ff0da0c/workdir/result.json"
```

Actual:

```text
keys=evidence_anchors,extraction_confidence_bps,extraction_metadata,field_confidences,lots,product_requirement_seeds,warnings
lots_len=1
lot_keys=contract_duration,cpv_codes,description,estimated_value,evaluation_criteria,lot_id,lot_number,lot_ref,primary_evidence_anchor_id,product_requirements,requirements,technical_standards,title lot_number=1 title=Best Start Family Hubs Outreach Services
seeds_len=0
```

The display above confirms `lot_number` is present, but it is JSON numeric `1`, not string `"1"`. The failure string comes from the daemon's string accessor.

---

## EVIDENCE BUNDLE

### E0 - Environment Evidence

**E0.1 - Docling runtime is now healthy**

```bash
/Users/zacfarrell/.docling-venv/bin/python scripts/docling-extract.py --version
/Users/zacfarrell/.docling-venv/bin/python -c "from importlib.metadata import version; print(version('docling'))"
ls -ld /Users/zacfarrell/.cache/docling/models
```

```text
2.93.0
2.93.0
drwxr-xr-x@ ... /Users/zacfarrell/.cache/docling/models
```

**E0.2 - Stage 4b structural extraction progressed after Docling setup**

```bash
source state-e2e-machine-fork-20260512T195251Z/e2e-env.sh
sqlite3 -header -column "$E2E_STATE_ROOT/state.db" "
SELECT kind, status, COUNT(*) AS count
FROM workflow_jobs
GROUP BY kind,status
ORDER BY kind,status;
"
```

```text
kind                        status     count
--------------------------  ---------  -----
extract_document_semantics  completed  3
extract_document_semantics  failed     1
extract_document_structure  completed  3
extract_document_structure  failed     3
sync_source                 completed  9
sync_source                 failed     4
sync_source                 pending    2
sync_source                 retry      3
```

The three `extract_document_structure failed` rows are preserved historical rows from before Docling setup. The latest retried structural jobs completed.

### E2 - Reproduction Evidence

**E2.1 - Downstream rows now exist, proving this is no longer the Docling runtime blocker**

```bash
source state-e2e-machine-fork-20260512T195251Z/e2e-env.sh
sqlite3 -header -column "$E2E_STATE_ROOT/state.db" "
SELECT COUNT(*) AS raw_docs_with_files FROM raw_documents WHERE storage_path IS NOT NULL;
SELECT COUNT(*) AS document_extractions FROM document_extractions;
SELECT COUNT(*) AS semantic_extractions FROM document_semantic_extractions;
SELECT COUNT(*) AS product_candidates FROM product_candidates;
SELECT COUNT(*) AS search_profiles FROM search_profiles;
"
```

```text
raw_docs_with_files
-------------------
3
document_extractions
--------------------
3
semantic_extractions
--------------------
3
product_candidates
------------------
5
search_profiles
---------------
0
```

**E2.2 - Completed semantic rows came from other overlays/documents**

```bash
source state-e2e-machine-fork-20260512T195251Z/e2e-env.sh
sqlite3 -header -column "$E2E_STATE_ROOT/state.db" "
SELECT id, document_extraction_id, skill_id, target_domain, extraction_scope, extraction_confidence_bps, json_array_length(written_entity_refs_json) AS written_refs, created_at
FROM document_semantic_extractions
ORDER BY created_at DESC, id;
"
```

```text
id                                             document_extraction_id                skill_id                             target_domain  extraction_scope  extraction_confidence_bps  written_refs  created_at
---------------------------------------------  ------------------------------------  -----------------------------------  -------------  ----------------  -------------------------  ------------  -------------------
semantic-a4be91da-0ea1-4d7c-978d-98d1e130c099  e27d7ac8-b6b1-435f-94fd-d605212a8a06  pdf-extraction/pricing-schedule      pricing        full_document     8500                       0             2026-05-16 15:51:56
semantic-7132d06b-90d9-46cc-8bed-aeedcb1b503c  e27d7ac8-b6b1-435f-94fd-d605212a8a06  pdf-extraction/tender-specification  lots           full_document     4500                       6             2026-05-16 15:47:22
semantic-f34caa0d-87ce-4ec6-9c28-2851931e085e  35c697a6-6970-4391-9f83-9767e9115ae2  pdf-extraction/pricing-schedule      pricing        full_document     500                        0             2026-05-16 15:40:48
```

### E3 - Code Evidence

**E3.1 - The skill schema requires `lots[].lot_number` to be a string**

`skills/pdf-extraction/tender-specification/result.schema.json`:

```json
"lots": {
  "type": "array",
  "items": {
    "type": "object",
    "required": [
      "lot_number",
      "title",
      "description",
      "requirements",
      "evaluation_criteria",
      "cpv_codes",
      "product_requirements",
      "technical_standards",
      "evidence_anchor"
    ],
    "properties": {
      "lot_number": {
        "type": "string"
      }
    }
  }
}
```

**E3.2 - The daemon requires `lots[].lot_number` as a string during tender-spec persistence**

`crates/daemon/src/main.rs`:

```rust
let lots = required_array_at(result_json, &["lots"], "lots")?;
for lot in lots {
    let lot_number_text =
        required_string_at(lot, &["lot_number"], "lots[].lot_number")?.to_owned();
    let lot_id = resolve_notice_lot_id(tx, notice_id, &lot_number_text)?;
    // ...
}
```

---

## ROOT CAUSE HYPOTHESES

### H1: The semantic extraction result violates its schema by emitting numeric `lot_number` - LIKELIHOOD: HIGH

Description: The failed `result.json` contains `lot_number: 1`. The skill schema and daemon persistence both require `"1"` as a string. The daemon reports `missing string lots[].lot_number` because the key exists but is not a JSON string.

Supporting evidence: E2.1, E2.2, E3.1, E3.2.

Verification steps:

1. Inspect `workdir/result.json` for session `session-semantic-80e2a70dc3f7de84a29da4d65e5660042ea33eb7b2a2124d17ac92408ff0da0c`.
2. Confirm `.lots[0].lot_number | type` is `number`.
3. Confirm the corresponding schema says `.lots.items.properties.lot_number.type == "string"`.

Fix scope: `skills/pdf-extraction/tender-specification/SKILL.md`, result schema enforcement, or semantic result normalization/persistence in `crates/daemon/src/main.rs`.

### H2: Contract-mode schema validation did not reject the invalid result before persistence - LIKELIHOOD: MEDIUM

Description: The agent session exited 0 and wrote `result.json`, but the daemon failed later during domain persistence. If contract mode is supposed to validate the skill schema before persistence, it either did not validate this output or did not enforce type errors.

Supporting evidence: session status `completed`, exit_code `0`; daemon job failed in persistence with `missing string lots[].lot_number`.

Verification steps:

1. Trace the contract runner path used by `run_extract_document_semantics_job`.
2. Confirm whether `skills/pdf-extraction/tender-specification/result.schema.json` is loaded and enforced.
3. Add a regression that feeds numeric `lot_number` into the semantic persistence path and expects an early structured validation error or successful normalization, depending on the chosen fix.

Fix scope: contract runner schema validation path and tests near semantic extraction persistence.

### H3: Daemon persistence should coerce numeric lot numbers to strings - LIKELIHOOD: LOW/MEDIUM

Description: Numeric lot numbers are common in model outputs. The project may choose to normalize `1` to `"1"` at the persistence boundary. However, this would weaken strict schema enforcement unless paired with explicit validation/normalization semantics.

Supporting evidence: one valid business lot is present and only the JSON type is wrong.

Contradicting evidence: the existing schema explicitly says string, and early-development project guidance prefers right behavior over compatibility shims.

Fix scope: `required_string_at` call sites or a targeted `required_lot_number_text` helper, plus regressions.

---

## INVESTIGATION ROADMAP

1. Decide whether semantic extraction output must be strictly schema-valid before persistence, or whether daemon persistence should normalize numeric lot numbers.
2. Add a focused regression around `pdf-extraction/tender-specification` semantic persistence using a `lots[].lot_number` numeric value.
3. If strict validation is the chosen behavior, make the failure earlier and clearer than `missing string lots[].lot_number`, and update the skill prompt/schema guidance so agents emit string lot numbers.
4. If normalization is chosen, normalize only lot-number fields that are explicitly schema-backed identifiers, and preserve tests proving product candidate writes remain correct.
5. Retry or requeue only `job-479bed51-0fea-46ee-a207-51e94c641c0b` or the equivalent semantic job after the fix; do not rerun Stage 4a or delete any existing E2E artifacts.

---

## FIX CONSTRAINTS

1. Do not delete or recreate `state-e2e-machine-fork-20260512T195251Z`, `runtime-e2e-machine-fork-20260512T195251Z`, or `artifacts-e2e-machine-fork-20260512T195251Z`.
2. Do not edit `PRAGMA user_version` or bypass schema state.
3. Do not rerun Stage 4a portal interest registration.
4. Preserve the existing downloaded ProContract documents and failed job rows.
5. If this becomes product-code work, use Cargo only and run the required Rust gates.
6. Treat unrelated staged `.DS_Store` and `bin/__pycache__/*` entries as concurrent work.

---

## ACCEPTANCE CRITERIA

1. A focused regression covers numeric `lots[].lot_number` in `pdf-extraction/tender-specification` semantic output.
2. The chosen fix either rejects invalid output before domain persistence with a clear contract-validation error or normalizes the value intentionally and writes the lot/candidate rows.
3. Retrying the failed semantic job no longer ends with `missing string lots[].lot_number`.
4. Existing focused regressions still pass:

```bash
cargo test -p daemon --bin daemon procontract_enrich_blocked_result_is_durable
cargo test -p daemon --bin daemon procontract_enrich_missing_result_marks_job_failed
cargo test -p daemon --test pdf_structural_extraction test_engine_unavailable
```

5. If code changes, run:

```bash
cargo fmt --check
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
```

