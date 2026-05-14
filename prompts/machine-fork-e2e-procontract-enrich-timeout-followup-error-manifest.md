# ProContract Enrichment Timeout Follow-Up - QA Error Manifest

ULTRATHINK and be meticulous. This is a follow-up forensic debugging document for the Stage 4b ProContract enrichment blocker after the first fix.

## DEFECT FINGERPRINT

| Field | Value |
|-------|-------|
| **Type** | live_enrichment_timeout / missing_result_contract |
| **Severity** | high |
| **Reproducibility** | Reproduced on the preserved live E2E state after commit `413c5a7` and after Claude quota recovered |
| **Affected Component** | `skills/procurement-ingestion/procontract-enrich-tender/SKILL.md`; live ProContract RFx/activity navigation |
| **Improved Behavior** | The daemon now marks the `procontract-enrich` job failed and writes an internal `error_manifests` row instead of leaving it `running`. |
| **Remaining Blocker** | The live enrichment skill still times out after 1200 seconds and produces no `result.json`, no run manifest, no `tender_enrichment` rows, and no downloaded document rows. |
| **Environment** | macOS Darwin 25.3.0 arm64; Cargo 1.96.0-nightly; Bun 1.3.13; `agent-browser` 0.27.0; Chrome CDP 148.0.7778.97; Claude probe passed before retry |
| **Git State** | `main` at `a20cd98 docs: record claude quota pause for e2e`; Stage 4b fix commit `413c5a7` present |

---

## REPRODUCTION PROTOCOL

Run from repo root with the preserved E2E context:

```bash
cd /Users/zacfarrell/Desktop/Testproject1
source state-e2e-machine-fork-20260512T195251Z/e2e-env.sh
```

### Step 1: Verify Claude and ProContract are usable

```bash
run_logged resume-claude-quota-probe-after-reset \
  claude -p "Return exactly OK." --dangerously-skip-permissions
run_logged resume-cdp-version-after-claude-quota-reset \
  curl -sfS -m 3 http://localhost:9222/json/version
run_logged stage-04b-procontract-login-open-after-claude-quota-reset \
  agent-browser --cdp 9222 open "https://procontract.due-north.com/SupplierPostLoginHome#"
run_logged stage-04b-procontract-login-title-after-claude-quota-reset-2 \
  agent-browser --cdp 9222 get title
run_logged stage-04b-procontract-login-snapshot-after-claude-quota-reset \
  agent-browser --cdp 9222 snapshot
```

Observed:

```text
claude probe: OK.
CDP: Chrome/148.0.7778.97, Protocol-Version 1.3
ProContract open: Supplier Post-Login Home
ProContract title: Supplier Post-Login Home
snapshot: SORELLON LIMITED supplier home with Vendor profile and Workgroups
```

### Step 2: Retry Stage 4b after the timeout fix

```bash
run_logged stage-04b-procontract-enrich-retry-after-timeout-fix \
  env AGENT_BROWSER_CDP_PORT=9222 "$E2E_DAEMON" \
    --state-root "$E2E_STATE_ROOT" \
    --runtime-dir "$E2E_RUNTIME_DIR" \
    --artifact-root "$E2E_ARTIFACT_ROOT" \
    --ingest procontract-enrich
```

Actual:

```text
2026-05-14T13:50:55.101556Z  INFO daemon bootstrap ready socket_path=/Users/zacfarrell/Desktop/Testproject1/runtime-e2e-machine-fork-20260512T195251Z/daemon.sock state_db=/Users/zacfarrell/Desktop/Testproject1/state-e2e-machine-fork-20260512T195251Z/state.db artifact_root=/Users/zacfarrell/Desktop/Testproject1/artifacts-e2e-machine-fork-20260512T195251Z
2026-05-14T13:50:55.102624Z  WARN L4 startup sync degraded before cross-authority dispatch; local operation continues failure=remote-owned pull unavailable for shared_source_configs
2026-05-14T14:52:23.769586Z  WARN PTY skill exceeded wall-clock timeout; dispatching SIGTERM runtime_ids_pid=2016 runtime_ids_pgid=2016 timeout_seconds=1200
FTS live loop failed: job_id=job-33fb160456947771375daba10501662ef301e46b280bcc6dfcc55c1f217405bf reason=procontract enrichment skill did not produce a result contract
Error: FTS live loop failed: job_id=job-33fb160456947771375daba10501662ef301e46b280bcc6dfcc55c1f217405bf reason=procontract enrichment skill did not produce a result contract
```

### Step 3: Confirm the browser state was not auth/captcha

```bash
run_logged stage-04b-procontract-post-retry-timeout-title agent-browser --cdp 9222 get title
run_logged stage-04b-procontract-post-retry-timeout-snapshot agent-browser --cdp 9222 snapshot
```

Observed:

```text
The title command returned an empty title, but the snapshot showed an authenticated ProContract activity dashboard.
Visible content included:
- Deadline & time remaining
- 5th June 2026 at 12:00 PM
- Messages & clarifications (0)
- Your response
- Start my response
- Opt out
- Indicate your intent to respond
```

No login page, captcha, or session-expired page was visible.

---

## EVIDENCE BUNDLE

### E0 - Preserved E2E Context

```text
E2E_STATE_ROOT=/Users/zacfarrell/Desktop/Testproject1/state-e2e-machine-fork-20260512T195251Z
E2E_RUNTIME_DIR=/Users/zacfarrell/Desktop/Testproject1/runtime-e2e-machine-fork-20260512T195251Z
E2E_ARTIFACT_ROOT=/Users/zacfarrell/Desktop/Testproject1/artifacts-e2e-machine-fork-20260512T195251Z
E2E_LOG_ROOT=/Users/zacfarrell/Desktop/Testproject1/artifacts-e2e-machine-fork-20260512T195251Z/qa-logs
E2E_DAEMON=/tmp/testproject1-target-machine-fork-20260512T195251Z/debug/daemon
PRAGMA user_version=9
```

### E1 - Primary Log

```text
/Users/zacfarrell/Desktop/Testproject1/artifacts-e2e-machine-fork-20260512T195251Z/qa-logs/stage-04b-procontract-enrich-retry-after-timeout-fix.log
```

The log contains repeated L4 writer lease heartbeat warnings during the long run and ends with the timeout/missing-result failure shown above. The L4 warnings are out of scope for Layer 0-3 acceptance but should be noted because they were emitted during the live run:

```text
WARN failed to heartbeat local L4 writer lease error=writer lease heartbeat requires an unexpired lease
```

### E2 - Workflow And Session Rows

```bash
run_logged stage-04b-procontract-enrich-retry-after-timeout-fix-forensics-db \
  sqlite3 -header -column "$E2E_STATE_ROOT/state.db" "
SELECT id, kind, status, scope_kind, scope_id, attempt_count, last_error, created_at, started_at, completed_at
FROM workflow_jobs WHERE scope_id='procontract-enrich' ORDER BY created_at DESC LIMIT 8;
SELECT id, job_id, skill_path, execution_mode, cli_vendor, status, exit_code, cleanup_verdict,
       terminal_output_bytes, terminal_output_path, run_manifest_path, created_at
FROM agent_session_logs
WHERE job_id IN (SELECT id FROM workflow_jobs WHERE scope_id='procontract-enrich')
ORDER BY created_at DESC LIMIT 8;
"
```

Observed latest rows:

```text
job-33fb160456947771375daba10501662ef301e46b280bcc6dfcc55c1f217405bf
kind=sync_source
status=failed
scope_id=procontract-enrich
last_error=procontract enrichment skill did not produce a result contract
started_at=2026-05-14T13:50:55.104202+00:00
completed_at=2026-05-14T14:52:24.106260+00:00

session-pty-3a29a437adfcce4b7cf3da0420cadc1b84970e978a4e9eeb31d148152a2c6afd
job_id=job-33fb160456947771375daba10501662ef301e46b280bcc6dfcc55c1f217405bf
skill_path=skills/procurement-ingestion/procontract-enrich-tender/SKILL.md
execution_mode=pty
cli_vendor=claude
status=completed
exit_code=143
cleanup_verdict=terminated_after_timeout
terminal_output_bytes=77
run_manifest_path=NULL
terminal_output_path=/Users/zacfarrell/Desktop/Testproject1/artifacts-e2e-machine-fork-20260512T195251Z/sessions/2026/05/session-pty-3a29a437adfcce4b7cf3da0420cadc1b84970e978a4e9eeb31d148152a2c6afd/stream.index.jsonl
```

This confirms the first fix improved job state from stale `running` to failed.

### E3 - Internal Error Manifest Row

```bash
sqlite3 -header -column "$E2E_STATE_ROOT/state.db" "
SELECT id, job_id, pipeline_stage, workflow_step, error_type, error_message,
       skill_path, agent_session_id, markdown_path, resolved, created_at
FROM error_manifests ORDER BY created_at DESC LIMIT 5;
"
```

Observed latest row:

```text
id=error-948338ee-8651-4aa2-a900-a139b70d7b87
job_id=job-33fb160456947771375daba10501662ef301e46b280bcc6dfcc55c1f217405bf
pipeline_stage=tender_enrichment
workflow_step=agent_execute
error_type=AGENT_RUN_FAILED
error_message=procontract enrichment skill did not produce a result contract
skill_path=skills/procurement-ingestion/procontract-enrich-tender/SKILL.md
agent_session_id=session-pty-3a29a437adfcce4b7cf3da0420cadc1b84970e978a4e9eeb31d148152a2c6afd
markdown_path=/Users/zacfarrell/Desktop/Testproject1/artifacts-e2e-machine-fork-20260512T195251Z/error-manifests/2026/05/error-948338ee-8651-4aa2-a900-a139b70d7b87.md
resolved=0
created_at=2026-05-14T14:52:24.104795+00:00
```

### E4 - Session Artifact

```bash
find "$E2E_ARTIFACT_ROOT/sessions/2026/05/session-pty-3a29a437adfcce4b7cf3da0420cadc1b84970e978a4e9eeb31d148152a2c6afd" -maxdepth 3 -type f -print | sort
sed -n '1,80p' "$E2E_ARTIFACT_ROOT/sessions/2026/05/session-pty-3a29a437adfcce4b7cf3da0420cadc1b84970e978a4e9eeb31d148152a2c6afd/stream.index.jsonl"
zstdcat "$E2E_ARTIFACT_ROOT/sessions/2026/05/session-pty-3a29a437adfcce4b7cf3da0420cadc1b84970e978a4e9eeb31d148152a2c6afd/stream/000001.pty.zst"
```

Observed:

```text
.../stream.index.jsonl
.../stream/000001.pty.zst

{"chunk":"stream/000001.pty.zst","first_seq":1,"last_seq":1,"byte_length":77,"wall_ts_start":"2026-05-14T14:52:24.074877+00:00","wall_ts_end":"2026-05-14T14:52:24.074877+00:00","sha256":"aa9e5e713c8b076acc8d3f4899fd126315119d18a5ff3a64e0195879c1c51c7e"}
```

The decompressed PTY stream contains only terminal-control bytes. There is no `workdir/result.json` and no run manifest.

### E5 - Downstream Rows Still Missing

```bash
sqlite3 -header -column "$E2E_STATE_ROOT/state.db" "
SELECT stage, state, COUNT(*) AS rows FROM process_stage_status WHERE stage='tender_enrichment' GROUP BY stage, state;
SELECT COUNT(*) AS tender_document_sets FROM tender_document_sets;
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
raw_docs_with_files=0
document_extractions=0
semantic_extractions=0
product_candidates=0
search_profiles=0
```

No `tender_enrichment` stage rows were returned.

---

## ROOT CAUSE HYPOTHESES

### H1: The skill still navigates into the activity dashboard and stalls without emitting a blocked result - LIKELIHOOD HIGH

The browser ends on the same kind of authenticated activity dashboard as the original timeout. The updated daemon correctly records failure, but the skill still does not emit a structured blocked `result.json` before daemon timeout.

### H2: The bounded-stall instructions are not operational enough for the child agent - LIKELIHOOD HIGH

Despite the skill changes in `413c5a7`, the child agent produced no textual output, no result contract, and no run manifest before the daemon killed it.

### H3: The ProContract activity page requires a specific click/URL path not captured by the skill - LIKELIHOOD MEDIUM

The activity dashboard showed `Start my response`, `Opt out`, and `Indicate your intent to respond`. The child may be waiting at a portal state that requires an allowed/forbidden decision before RFx documents are visible. The skill should detect that state and emit a blocked result rather than waiting until timeout.

---

## FIX CONSTRAINTS

1. Do not delete or recreate the preserved E2E state, runtime, artifacts, or session directories.
2. Do not close or replace the persistent CDP browser.
3. Do not retry live Stage 4b repeatedly without a targeted fix; this is a live portal path.
4. Do not weaken result-contract validation.
5. Preserve the daemon durability behavior introduced by `413c5a7`; the latest job failed durably and created an internal error manifest, which is progress.
6. If the portal is waiting on an operator/bid-response decision such as `Start my response` or `Indicate your intent to respond`, the skill must classify it as a structured blocked result unless that action is explicitly allowed by the QA scope.

---

## ACCEPTANCE CRITERIA

- Stage 4b no longer waits until daemon timeout on this activity-dashboard state.
- The skill emits either:
  - a valid enrichment `result.json` with downloaded documents, or
  - a valid blocked `result.json` with a precise blocker such as `PROCONTRACT_RESPONSE_ACTION_REQUIRED`, `AGENT_STALLED`, or `PAGE_SHAPE_DRIFT`.
- The daemon keeps the job non-running on failure and preserves the internal error manifest behavior.
- `tender_document_sets`, `raw_documents.storage_path`, `document_extractions`, and downstream product/search rows are populated when documents are available.
- If documents are not available because the portal requires a response action, the blocked result is persisted and surfaced with process/stage context.

---

## RESUME POINT AFTER FIX

After the fixer updates Stage 4b behavior, resume from the preserved roots:

```bash
cd /Users/zacfarrell/Desktop/Testproject1
source state-e2e-machine-fork-20260512T195251Z/e2e-env.sh
run_logged resume-claude-probe-before-procontract-enrich-followup \
  claude -p "Return exactly OK." --dangerously-skip-permissions
run_logged stage-04b-procontract-login-open-after-enrich-followup-fix \
  agent-browser --cdp 9222 open "https://procontract.due-north.com/SupplierPostLoginHome#"
run_logged stage-04b-procontract-enrich-retry-after-followup-fix \
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
SELECT COUNT(*) AS document_extractions FROM document_extractions;
SELECT COUNT(*) AS semantic_extractions FROM document_semantic_extractions;
SELECT COUNT(*) AS product_candidates FROM product_candidates;
SELECT COUNT(*) AS search_profiles FROM search_profiles;
SELECT id, kind, status, scope_id, last_error FROM workflow_jobs WHERE scope_id='procontract-enrich' ORDER BY created_at DESC LIMIT 5;
"
```

