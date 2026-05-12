# ProContract Enrichment Timeout Missing Result - QA Error Manifest

ULTRATHINK and be meticulous. This is a forensic debugging document.

## DEFECT FINGERPRINT

| Field | Value |
|-------|-------|
| **Type** | daemon_timeout / missing_result_contract / stale_running_job |
| **Severity** | high |
| **Reproducibility** | Reproduced once in live Stage 4b after authenticated ProContract preflight. Retry only after a fix because this is a live portal stage. |
| **Affected Component** | `crates/daemon/src/main.rs::run_procontract_enrich_tender_live_loop` and `skills/procurement-ingestion/procontract-enrich-tender/SKILL.md` |
| **Working Analogue** | Stage 1 Contracts Finder and ProContract discovery completed with result contracts; Stage 4a register-interest fix persisted rows after result contract handling was repaired. |
| **Scope** | Stage 4b ProContract enrichment runs for 1200 seconds, times out, writes no result contract or run manifest, leaves the workflow job `running`, and persists no tender document rows. |
| **Environment** | macOS Darwin 25.3.0 arm64; Bun 1.3.13; Cargo 1.96.0-nightly; `agent-browser` 0.27.0; Chrome CDP 148.0.7778.97 |
| **Git State** | `main` at `66420a0 docs: record procontract enrichment auth pause` before this QA artifact; Stage 4a product fix commit `45546c2` present |

---

## REPRODUCTION PROTOCOL

Run from repo root after sourcing the preserved E2E context:

```bash
cd /Users/zacfarrell/Desktop/Testproject1
source state-e2e-machine-fork-20260512T195251Z/e2e-env.sh
```

### Step 1: Verify ProContract CDP auth

```bash
run_logged stage-04b-procontract-login-open-after-operator-login \
  agent-browser --cdp 9222 open "https://procontract.due-north.com/SupplierPostLoginHome#"
run_logged stage-04b-procontract-login-title-after-operator-login \
  agent-browser --cdp 9222 get title
run_logged stage-04b-procontract-login-snapshot-after-operator-login \
  agent-browser --cdp 9222 snapshot
```

Observed before running enrichment:

```text
✓ Supplier Post-Login Home
https://procontract.due-north.com/SupplierPostLoginHome#
Supplier Post-Login Home
snapshot showed SORELLON LIMITED, Opportunities, Company details summary, Vendor profile
```

### Step 2: Run Stage 4b enrichment

```bash
run_logged stage-04b-procontract-enrich \
  env AGENT_BROWSER_CDP_PORT=9222 "$E2E_DAEMON" \
    --state-root "$E2E_STATE_ROOT" \
    --runtime-dir "$E2E_RUNTIME_DIR" \
    --artifact-root "$E2E_ARTIFACT_ROOT" \
    --ingest procontract-enrich
```

Expected:

- The skill writes a valid `result.json`, including either `enriched_tenders[]` or a structured blocked result.
- The daemon persists tender enrichment/document rows or a durable blocked state.
- The source workflow job is completed or failed, not left `running`.

Actual:

```text
$ env AGENT_BROWSER_CDP_PORT=9222 /tmp/testproject1-target-machine-fork-20260512T195251Z/debug/daemon --state-root /Users/zacfarrell/Desktop/Testproject1/state-e2e-machine-fork-20260512T195251Z --runtime-dir /Users/zacfarrell/Desktop/Testproject1/runtime-e2e-machine-fork-20260512T195251Z --artifact-root /Users/zacfarrell/Desktop/Testproject1/artifacts-e2e-machine-fork-20260512T195251Z --ingest procontract-enrich
2026-05-12T22:12:12.219875Z  INFO daemon bootstrap ready socket_path=/Users/zacfarrell/Desktop/Testproject1/runtime-e2e-machine-fork-20260512T195251Z/daemon.sock state_db=/Users/zacfarrell/Desktop/Testproject1/state-e2e-machine-fork-20260512T195251Z/state.db artifact_root=/Users/zacfarrell/Desktop/Testproject1/artifacts-e2e-machine-fork-20260512T195251Z
2026-05-12T22:12:12.220709Z  WARN L4 startup sync degraded before cross-authority dispatch; local operation continues failure=remote-owned pull unavailable for shared_source_configs
2026-05-12T22:32:12.821566Z  WARN PTY skill exceeded wall-clock timeout; dispatching SIGTERM runtime_ids_pid=42416 runtime_ids_pgid=42416 timeout_seconds=1200
Error: procontract enrichment skill did not produce a result contract
```

### Step 3: Confirm ProContract auth was still live after timeout

```bash
run_logged stage-04b-procontract-post-timeout-title agent-browser --cdp 9222 get title
run_logged stage-04b-procontract-post-timeout-snapshot agent-browser --cdp 9222 snapshot
```

Observed after timeout:

```text
The snapshot showed an authenticated ProContract activity dashboard, not a login or captcha page.
Visible page content included:
- Messages & clarifications (0)
- Your response
- Start my response
- Opt out
- Indicate your intent to respond
```

This was therefore not classified as a first-pass auth/CDP pause.

---

## EVIDENCE BUNDLE

### E0 - E2E Context

```text
E2E_STATE_ROOT=/Users/zacfarrell/Desktop/Testproject1/state-e2e-machine-fork-20260512T195251Z
E2E_RUNTIME_DIR=/Users/zacfarrell/Desktop/Testproject1/runtime-e2e-machine-fork-20260512T195251Z
E2E_ARTIFACT_ROOT=/Users/zacfarrell/Desktop/Testproject1/artifacts-e2e-machine-fork-20260512T195251Z
E2E_LOG_ROOT=/Users/zacfarrell/Desktop/Testproject1/artifacts-e2e-machine-fork-20260512T195251Z/qa-logs
E2E_DAEMON=/tmp/testproject1-target-machine-fork-20260512T195251Z/debug/daemon
PRAGMA user_version=9
```

### E1 - Primary Failed Command Log

Primary log:

```text
/Users/zacfarrell/Desktop/Testproject1/artifacts-e2e-machine-fork-20260512T195251Z/qa-logs/stage-04b-procontract-enrich.log
```

Relevant output:

```text
2026-05-12T22:32:12.821566Z  WARN PTY skill exceeded wall-clock timeout; dispatching SIGTERM runtime_ids_pid=42416 runtime_ids_pgid=42416 timeout_seconds=1200
Error: procontract enrichment skill did not produce a result contract
```

### E2 - Workflow And Session Rows

```bash
sqlite3 -header -column "$E2E_STATE_ROOT/state.db" "
SELECT id, kind, status, scope_kind, scope_id, attempt_count, last_error, created_at, started_at, completed_at
FROM workflow_jobs WHERE scope_id='procontract-enrich' ORDER BY created_at DESC LIMIT 5;
SELECT id, job_id, skill_path, execution_mode, cli_vendor, status, exit_code, cleanup_verdict,
       terminal_output_bytes, terminal_output_path, run_manifest_path, created_at
FROM agent_session_logs
WHERE job_id IN (SELECT id FROM workflow_jobs WHERE scope_id='procontract-enrich')
ORDER BY created_at DESC LIMIT 5;
"
```

Observed:

```text
job-cf642ec3d9e68b1fe4e28e94eb9eaca37126e59f8dc7c300b091263ac0109755  sync_source  running  source  procontract-enrich  attempt_count=1  created_at=2026-05-12 22:12:12  started_at=2026-05-12T22:12:12.222379+00:00

session-pty-33fd5d63d698179f0b393ea6f8a3481a2c00f2191e3773017ee8e836aa4b7757
job_id=job-cf642ec3d9e68b1fe4e28e94eb9eaca37126e59f8dc7c300b091263ac0109755
skill_path=skills/procurement-ingestion/procontract-enrich-tender/SKILL.md
execution_mode=pty
cli_vendor=claude
status=completed
exit_code=143
cleanup_verdict=terminated_after_timeout
terminal_output_bytes=77
run_manifest_path=NULL
terminal_output_path=/Users/zacfarrell/Desktop/Testproject1/artifacts-e2e-machine-fork-20260512T195251Z/sessions/2026/05/session-pty-33fd5d63d698179f0b393ea6f8a3481a2c00f2191e3773017ee8e836aa4b7757/stream.index.jsonl
```

### E3 - Artifact Tree

```bash
find "$E2E_ARTIFACT_ROOT/sessions/2026/05/session-pty-33fd5d63d698179f0b393ea6f8a3481a2c00f2191e3773017ee8e836aa4b7757" -maxdepth 3 -type f -print | sort
sed -n '1,80p' "$E2E_ARTIFACT_ROOT/sessions/2026/05/session-pty-33fd5d63d698179f0b393ea6f8a3481a2c00f2191e3773017ee8e836aa4b7757/stream.index.jsonl"
zstdcat "$E2E_ARTIFACT_ROOT/sessions/2026/05/session-pty-33fd5d63d698179f0b393ea6f8a3481a2c00f2191e3773017ee8e836aa4b7757/stream/000001.pty.zst"
```

Observed:

```text
.../stream.index.jsonl
.../stream/000001.pty.zst

{"chunk":"stream/000001.pty.zst","first_seq":1,"last_seq":1,"byte_length":77,"wall_ts_start":"2026-05-12T22:32:13.236190+00:00","wall_ts_end":"2026-05-12T22:32:13.236190+00:00","sha256":"aa9e5e713c8b076acc8d3f4899fd126315119d18a5ff3a64e0195879c1c51c7e"}
```

The decompressed PTY bytes were only terminal-control sequences. There is no `workdir/result.json` and no `run_manifest_path`.

### E4 - Downstream Rows Missing

```bash
sqlite3 -header -column "$E2E_STATE_ROOT/state.db" "
SELECT stage, state, COUNT(*) AS rows FROM process_stage_status WHERE stage='tender_enrichment' GROUP BY stage, state;
SELECT COUNT(*) AS tender_document_sets FROM tender_document_sets;
SELECT COUNT(*) AS raw_documents_total FROM raw_documents;
SELECT COUNT(*) AS raw_docs_with_files FROM raw_documents WHERE storage_path IS NOT NULL;
SELECT COUNT(*) AS document_extractions FROM document_extractions;
SELECT COUNT(*) AS semantic_extractions FROM document_semantic_extractions;
SELECT COUNT(*) AS product_candidates FROM product_candidates;
SELECT COUNT(*) AS search_profiles FROM search_profiles;
"
```

Observed:

```text
tender_document_sets=0
raw_documents_total=204
raw_docs_with_files=0
document_extractions=0
semantic_extractions=0
product_candidates=0
search_profiles=0
```

No `tender_enrichment` rows were returned.

### E5 - Codepath Evidence

Relevant daemon path:

```text
crates/daemon/src/main.rs:16662-16786
```

Key behavior:

```rust
let executed =
    execute_skill_session(runtime, &skill, &job_id, &session_id, &job_parameters, cli).await?;
insert_executed_session_log(runtime.state_db.connection(), &skill, &job_id, &executed)?;
let result_path = executed
    .result_path
    .as_ref()
    .ok_or_else(|| anyhow!("procontract enrichment skill did not produce a result contract"))?;
```

Because the timeout produced no `result_path`, the function returns an error after session-log insertion and before `complete_source_job` or `fail_job`, leaving the job `running`.

Relevant skill contract:

```text
skills/procurement-ingestion/procontract-enrich-tender/SKILL.md:34-40 failure codes include AGENT_AUTH_CHALLENGE, PAGE_SHAPE_DRIFT, SOURCE_SOFT_BLOCKED, AGENT_PARTIAL_FAILURE, AGENT_STALLED.
skills/procurement-ingestion/procontract-enrich-tender/SKILL.md:88 says if portal shows Log In, Session expired, captcha, or another auth challenge, emit a valid blocked result.
```

The run emitted no valid blocked result.

---

## ROOT CAUSE HYPOTHESES

### H1: Skill got stuck in the ProContract activity/RFx navigation path - LIKELIHOOD HIGH

The post-timeout snapshot shows the browser on an authenticated ProContract activity dashboard with response actions. The skill likely navigated into an activity but did not complete Step 4/5, emit a partial result, or emit `AGENT_STALLED` before the daemon timeout.

Evidence: E1, E2, E3, post-timeout snapshot in reproduction Step 3.

### H2: Missing-result timeout error path leaves jobs stale-running - LIKELIHOOD HIGH

The daemon records the session log then bails when `executed.result_path` is `None`; unlike the fixed Stage 4a persistence path, this code path does not call `fail_job`.

Evidence: E2, E5.

### H3: Skill instructions lack a bounded stall checkpoint for per-activity navigation - LIKELIHOOD MEDIUM

The skill has failure code `AGENT_STALLED`, but the live run did not write a blocked contract within 1200 seconds. The playbook may need explicit per-page/per-activity time budgets and mandatory partial-result emission.

Evidence: E1, E3, E5.

---

## FIX CONSTRAINTS

1. Do not delete or recreate the preserved E2E state, runtime, artifact, or session directories.
2. Do not close or replace the persistent CDP browser.
3. Do not weaken result-contract validation.
4. Do not rerun live Stage 4b until the skill/daemon timeout behavior is fixed or the operator explicitly accepts a focused retry.
5. Preserve the existing portal mutation history: Stage 4a already registered `BEST START FAMILY HUBS OUTREACH SERVICES`.
6. Unknown source/page-shape/auth blockers must be emitted as valid blocked `result.json`, not as silence until timeout.

---

## ACCEPTANCE CRITERIA

- Stage 4b exits 0 with a valid enrichment result contract, or exits with a valid structured blocked contract before daemon timeout.
- A timed-out or missing-result `procontract-enrich` session marks the workflow job failed durably; it must not remain `running`.
- `tender_document_sets` and `raw_documents.storage_path` rows are populated when attachments are available, or a structured no-documents/blocked reason is persisted.
- `process_stage_status` includes `tender_enrichment` rows for evaluated tenders.
- Regression coverage proves missing-result/timeout in `run_procontract_enrich_tender_live_loop` marks the job failed and includes the session ID/artifact path.
- Focused replay or fake-adapter tests cover a valid `procontract-enrich` result contract and a structured blocked result.

---

## RESUME POINT AFTER FIX

After the fixer completes the repair, continue this QA run from Stage 4b against the preserved roots:

```bash
cd /Users/zacfarrell/Desktop/Testproject1
source state-e2e-machine-fork-20260512T195251Z/e2e-env.sh
run_logged stage-04b-procontract-login-open-after-enrich-timeout-fix \
  agent-browser --cdp 9222 open "https://procontract.due-north.com/SupplierPostLoginHome#"
run_logged stage-04b-procontract-enrich-retry-after-timeout-fix \
  env AGENT_BROWSER_CDP_PORT=9222 "$E2E_DAEMON" \
    --state-root "$E2E_STATE_ROOT" \
    --runtime-dir "$E2E_RUNTIME_DIR" \
    --artifact-root "$E2E_ARTIFACT_ROOT" \
    --ingest procontract-enrich
```

Then validate:

```bash
sqlite3 -header -column "$E2E_STATE_ROOT/state.db" "
SELECT stage, state, COUNT(*) FROM process_stage_status WHERE stage='tender_enrichment' GROUP BY stage, state;
SELECT COUNT(*) AS tender_document_sets FROM tender_document_sets;
SELECT COUNT(*) AS raw_docs_with_files FROM raw_documents WHERE storage_path IS NOT NULL;
SELECT id, kind, status, scope_id, last_error FROM workflow_jobs WHERE scope_id='procontract-enrich' ORDER BY created_at DESC LIMIT 5;
"
```

