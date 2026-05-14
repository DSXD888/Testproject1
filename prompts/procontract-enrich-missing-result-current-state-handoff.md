# ProContract Enrich Missing Result Current-State Handoff

ULTRATHINK and be meticulous. This is a forensic debugging and validation handoff for the Stage 4b ProContract enrichment missing-result failure.

## Mission

Handle the `--ingest procontract-enrich` failure mode where the PTY skill session exits, times out, or is killed without a valid `result.json`.

Important current-state finding: on the current checkout, the missing-result trigger is still reproducible, but the original stale `workflow_jobs.status='running'` defect did not reproduce in synthetic validation. The daemon now marks the job `failed`, records `completed_at`, persists session context in `output_payload`, and creates an `error_manifests` row. Treat this as a candidate fix that still needs full quality-gate and live Stage 4b validation.

Do not delete or recreate preserved E2E state, runtime, artifact, or session directories. Do not close or replace the persistent CDP browser.

## Defect Fingerprint

| Field | Value |
|-------|-------|
| Type | missing_result_contract / workflow_job_durability / live_skill_timeout |
| Severity | high |
| Current reproducibility | Deterministic for synthetic timeout and fast-fail no-result triggers |
| Original defect | Missing result caused raw error return after session log insertion, leaving `workflow_jobs` row `running` |
| Current observed behavior | Missing result returns CLI error text, but database row is `failed` with durable context |
| Affected daemon path | `crates/daemon/src/main.rs::run_procontract_enrich_tender_live_loop` |
| Affected skill | `skills/procurement-ingestion/procontract-enrich-tender/SKILL.md` |
| Required remaining validation | Full cargo gate, synthetic timeout/fast-fail/block replay, then live Stage 4b resume |

## Environment Snapshot

Commands run from `/Users/zacfarrell/Desktop/Testproject1` on 2026-05-14.

```bash
git status --short --branch
```

```text
## main...origin/main
M  .DS_Store
A  bin/__pycache__/br-claimcpython-313.pyc
A  bin/__pycache__/br-completecpython-313.pyc
```

```bash
git rev-parse HEAD
git log -1 --oneline
uname -a
cargo --version
rustc --version
bun --version
sqlite3 --version
agent-browser --version
claude --version
sqlite3 -header -column state-e2e-machine-fork-20260512T195251Z/state.db "PRAGMA user_version;"
```

```text
9731b160eddcc32471aaded327efe1d7e4a35901
9731b16 docs: record procontract enrichment followup timeout
Darwin Mac 25.3.0 Darwin Kernel Version 25.3.0: Wed Jan 28 20:56:34 PST 2026; root:xnu-12377.91.3~2/RELEASE_ARM64_T8122 arm64
cargo 1.96.0-nightly (cbb9bb8bd 2026-03-13)
rustc 1.96.0-nightly (bcf3d36c9 2026-03-19)
1.3.13
3.51.0 2025-06-12 13:14:41 f0ca7bba1c5e232e5d279fad6338121ab55af0c8c68c84cdfb18ba5114dcaapl (64-bit)
agent-browser 0.27.0
2.1.139 (Claude Code)
user_version
------------
9
```

The unrelated dirty files listed above were pre-existing. Leave them untouched unless explicitly told otherwise.

## Current Code Context

Read these areas before changing anything:

| File | Role | Current lines |
|------|------|---------------|
| `crates/daemon/src/main.rs` | Completion payload preserves blocked status and blockers | around 4381-4436 |
| `crates/daemon/src/main.rs` | Missing-result error context builder | around 4451-4470 |
| `crates/daemon/src/main.rs` | Missing-result failure helper | around 4473-4505 |
| `crates/daemon/src/main.rs` | PTY result-path detection and cleanup verdict | around 15742-15865 |
| `crates/daemon/src/main.rs` | ProContract enrich live loop calls missing-result helper | around 16864-16876 |
| `crates/daemon/src/main.rs` | Focused missing-result and blocked-result tests | around 28384-28632 |
| `crates/daemon/src/main.rs` | PTY timeout/result supervisor tests | around 30207-30320 |
| `skills/procurement-ingestion/procontract-enrich-tender/SKILL.md` | Bounded-stall contract and blocked result requirements | around 92-120, 277-329 |

Current daemon branch for missing result:

```rust
insert_executed_session_log(runtime.state_db.connection(), &skill, &job_id, &executed)?;
let Some(result_path) = executed.result_path.as_ref() else {
    return fail_procontract_enrich_missing_result_contract(
        runtime, &job_id, &skill, &executed,
    );
};
```

Current helper records the durable context and then calls `fail_job`:

```rust
fn fail_procontract_enrich_missing_result_contract(
    runtime: &mut RuntimeBootstrap,
    job_id: &str,
    skill: &LoadedSkill,
    executed: &ExecutedSession,
) -> Result<LiveLoopReport> {
    let message = "procontract enrichment skill did not produce a result contract";
    let run_manifest_path =
        session_run_manifest_path(runtime.state_db.connection(), &executed.session_id)?;
    let error_context =
        procontract_enrich_missing_result_context(skill, executed, run_manifest_path.as_deref());
    let error_manifest_id = create_error_manifest(
        runtime,
        ErrorCode::AgentRunFailed,
        job_id,
        None,
        Some(&executed.session_id),
        Some(&skill.skill_path),
        "tender_enrichment",
        "agent_execute",
        message,
        error_context.clone(),
    )?;
    let mut output_payload = error_context;
    if let Some(payload) = output_payload.as_object_mut() {
        payload.insert("job_id".to_owned(), json!(job_id));
        payload.insert(
            "error_code".to_owned(),
            json!(ErrorCode::AgentRunFailed.as_str()),
        );
        payload.insert("error_manifest_id".to_owned(), json!(error_manifest_id));
    }
    fail_job(runtime, job_id, message, output_payload)
}
```

Current blocked-result payload preserves `status` and `blockers`:

```rust
payload.insert(
    "status".to_owned(),
    result_json
        .get("status")
        .cloned()
        .unwrap_or_else(|| json!("completed")),
);
payload.insert(
    "blockers".to_owned(),
    result_json
        .get("blockers")
        .cloned()
        .unwrap_or_else(|| json!([])),
);
```

## Reproduction Protocol

### Step 1 - Source the preserved E2E environment

```bash
cd /Users/zacfarrell/Desktop/Testproject1
source state-e2e-machine-fork-20260512T195251Z/e2e-env.sh
```

### Step 2 - Recreate timeout/no-result with a fake sleeping CLI

This writes only under `/tmp`.

```bash
REPRO_DIR="/tmp/testproject1-procontract-enrich-timeout-validation-$(date -u +%Y%m%dT%H%M%SZ)"
mkdir -p "$REPRO_DIR/state" "$REPRO_DIR/runtime" "$REPRO_DIR/artifacts" "$REPRO_DIR/logs"
FAKE_CLAUDE="$REPRO_DIR/fake-claude-sleep.sh"
printf '%s\n' '#!/bin/sh' \
'if [ "${1:-}" = "--version" ]; then' \
'  printf "1.2.3 (fake claude)\n"' \
'  exit 0' \
'fi' \
'if [ "${1:-}" = "probe" ]; then' \
'  printf "probe unsupported in fake cli\n" >&2' \
'  exit 1' \
'fi' \
'sleep 30' > "$FAKE_CLAUDE"
chmod 700 "$FAKE_CLAUDE"

set +e
PTY_SKILL_TIMEOUT_SECONDS=2 "$E2E_DAEMON" \
  --agent-cli-program "$FAKE_CLAUDE" \
  --state-root "$REPRO_DIR/state" \
  --runtime-dir "$REPRO_DIR/runtime" \
  --artifact-root "$REPRO_DIR/artifacts" \
  --ingest procontract-enrich > "$REPRO_DIR/logs/synthetic-timeout.log" 2>&1
daemon_status=$?
set -e

printf 'REPRO_DIR=%s\n' "$REPRO_DIR"
printf 'DAEMON_STATUS=%s\n' "$daemon_status"
tail -80 "$REPRO_DIR/logs/synthetic-timeout.log"
sqlite3 -header -column "$REPRO_DIR/state/state.db" "
SELECT id, status, last_error, completed_at,
       json_extract(output_payload, '$.session_id') AS session_id,
       json_extract(output_payload, '$.cleanup_verdict') AS cleanup_verdict,
       json_extract(output_payload, '$.terminal_output_path') AS terminal_output_path,
       json_extract(output_payload, '$.error_manifest_id') AS error_manifest_id
FROM workflow_jobs WHERE scope_id='procontract-enrich' ORDER BY created_at DESC LIMIT 1;
SELECT id, job_id, exit_code, cleanup_verdict, terminal_output_path, run_manifest_path
FROM agent_session_logs ORDER BY created_at DESC LIMIT 1;
SELECT id, job_id, pipeline_stage, workflow_step, error_type, error_message,
       skill_path, agent_session_id, markdown_path, resolved, created_at
FROM error_manifests ORDER BY created_at DESC LIMIT 1;
"
```

Observed current run:

```text
REPRO_DIR=/tmp/testproject1-procontract-enrich-timeout-handoff-20260514T150438Z
DAEMON_STATUS=1
2026-05-14T15:04:38.854892Z  INFO daemon bootstrap ready socket_path=/tmp/testproject1-procontract-enrich-timeout-handoff-20260514T150438Z/runtime/daemon.sock state_db=/tmp/testproject1-procontract-enrich-timeout-handoff-20260514T150438Z/state/state.db artifact_root=/tmp/testproject1-procontract-enrich-timeout-handoff-20260514T150438Z/artifacts
2026-05-14T15:04:38.858087Z  WARN L4 startup sync degraded before cross-authority dispatch; local operation continues failure=remote-owned pull unavailable for shared_source_configs
2026-05-14T15:04:41.252718Z  WARN PTY skill exceeded wall-clock timeout; dispatching SIGTERM runtime_ids_pid=23322 runtime_ids_pgid=23322 timeout_seconds=2
FTS live loop failed: job_id=job-30d20fa9ee33ab55766214aefb4850a22d1780bc999924d0624c4a4d9a794e00 reason=procontract enrichment skill did not produce a result contract
Error: FTS live loop failed: job_id=job-30d20fa9ee33ab55766214aefb4850a22d1780bc999924d0624c4a4d9a794e00 reason=procontract enrichment skill did not produce a result contract

workflow_jobs latest:
id=job-30d20fa9ee33ab55766214aefb4850a22d1780bc999924d0624c4a4d9a794e00
status=failed
last_error=procontract enrichment skill did not produce a result contract
completed_at=2026-05-14T15:04:41.318724+00:00
session_id=session-pty-354e7f9af073a13ba95151d73d8cad7ad05a0c32222bd58c191b9f83c61c489d
cleanup_verdict=terminated_after_timeout
terminal_output_path=/tmp/testproject1-procontract-enrich-timeout-handoff-20260514T150438Z/artifacts/sessions/2026/05/session-pty-354e7f9af073a13ba95151d73d8cad7ad05a0c32222bd58c191b9f83c61c489d/stream.index.jsonl
error_manifest_id=error-bfe16dc0-0762-4d0d-a5a4-5143d1a13a85

agent_session_logs latest:
id=session-pty-354e7f9af073a13ba95151d73d8cad7ad05a0c32222bd58c191b9f83c61c489d
exit_code=1
cleanup_verdict=terminated_after_timeout
run_manifest_path=NULL

error_manifests latest:
id=error-bfe16dc0-0762-4d0d-a5a4-5143d1a13a85
pipeline_stage=tender_enrichment
workflow_step=agent_execute
error_type=AGENT_RUN_FAILED
error_message=procontract enrichment skill did not produce a result contract
agent_session_id=session-pty-354e7f9af073a13ba95151d73d8cad7ad05a0c32222bd58c191b9f83c61c489d
```

Expected on a correct build: the command may return nonzero because the run failed, but no latest `procontract-enrich` job may remain `running`.

### Step 3 - Recreate fast-fail/no-result with a fake quota CLI

```bash
REPRO_DIR="/tmp/testproject1-procontract-enrich-fastfail-validation-$(date -u +%Y%m%dT%H%M%SZ)"
mkdir -p "$REPRO_DIR/state" "$REPRO_DIR/runtime" "$REPRO_DIR/artifacts" "$REPRO_DIR/logs"
FAKE_CLAUDE="$REPRO_DIR/fake-claude-fastfail.sh"
printf '%s\n' '#!/bin/sh' \
'if [ "${1:-}" = "--version" ]; then' \
'  printf "1.2.3 (fake claude)\n"' \
'  exit 0' \
'fi' \
'if [ "${1:-}" = "probe" ]; then' \
'  printf "probe unsupported in fake cli\n" >&2' \
'  exit 1' \
'fi' \
'printf "You have hit your limit - resets 2:50am (Europe/London)\n" >&2' \
'exit 1' > "$FAKE_CLAUDE"
chmod 700 "$FAKE_CLAUDE"

set +e
PTY_SKILL_TIMEOUT_SECONDS=20 "$E2E_DAEMON" \
  --agent-cli-program "$FAKE_CLAUDE" \
  --state-root "$REPRO_DIR/state" \
  --runtime-dir "$REPRO_DIR/runtime" \
  --artifact-root "$REPRO_DIR/artifacts" \
  --ingest procontract-enrich > "$REPRO_DIR/logs/fastfail.log" 2>&1
daemon_status=$?
set -e

printf 'REPRO_DIR=%s\n' "$REPRO_DIR"
printf 'DAEMON_STATUS=%s\n' "$daemon_status"
tail -80 "$REPRO_DIR/logs/fastfail.log"
sqlite3 -header -column "$REPRO_DIR/state/state.db" "
SELECT id, status, last_error, completed_at,
       json_extract(output_payload, '$.session_id') AS session_id,
       json_extract(output_payload, '$.cleanup_verdict') AS cleanup_verdict,
       json_extract(output_payload, '$.terminal_output_bytes') AS terminal_output_bytes,
       json_extract(output_payload, '$.terminal_output_tail') AS terminal_output_tail
FROM workflow_jobs WHERE scope_id='procontract-enrich' ORDER BY created_at DESC LIMIT 1;
SELECT id, job_id, exit_code, cleanup_verdict, terminal_output_bytes, run_manifest_path
FROM agent_session_logs ORDER BY created_at DESC LIMIT 1;
"
```

Observed current run:

```text
REPRO_DIR=/tmp/testproject1-procontract-enrich-fastfail-handoff-20260514T150508Z
DAEMON_STATUS=1
FTS live loop failed: job_id=job-9915bc554625334172d24ea84ae3a62d7d37b8841cc2811ea9c49a74466ed52b reason=procontract enrichment skill did not produce a result contract
Error: FTS live loop failed: job_id=job-9915bc554625334172d24ea84ae3a62d7d37b8841cc2811ea9c49a74466ed52b reason=procontract enrichment skill did not produce a result contract

workflow_jobs latest:
id=job-9915bc554625334172d24ea84ae3a62d7d37b8841cc2811ea9c49a74466ed52b
status=failed
last_error=procontract enrichment skill did not produce a result contract
completed_at=2026-05-14T15:05:08.630275+00:00
cleanup_verdict=clean
terminal_output_bytes=65
terminal_output_tail=You 've hit your limit - resets 2:50am (Europe/London)

agent_session_logs latest:
id=session-pty-3538b63860a5406c9095fbb11dff3fb74b53730fea286672151b6594fa814c2a
exit_code=1
cleanup_verdict=clean
terminal_output_bytes=65
run_manifest_path=NULL
```

Expected on a correct build: `status=failed`, `completed_at IS NOT NULL`, `cleanup_verdict=clean`, and terminal-output context is queryable.

### Step 4 - Replay a structured blocked result

```bash
REPRO_DIR="/tmp/testproject1-procontract-enrich-blocked-replay-validation-$(date -u +%Y%m%dT%H%M%SZ)"
mkdir -p "$REPRO_DIR/state" "$REPRO_DIR/runtime" "$REPRO_DIR/artifacts" "$REPRO_DIR/logs"
RESULT_JSON="$REPRO_DIR/result-blocked.json"
printf '%s\n' \
'{' \
'  "cursor": "{\"blocked_at\":\"2026-05-14T15:05:30Z\",\"started_at\":\"2026-05-14T15:04:30Z\",\"last_activity\":\"synthetic blocked replay\"}",' \
'  "continuation_required": false,' \
'  "enriched_tenders": [],' \
'  "summary": {' \
'    "total_checked": 1,' \
'    "enriched": 0,' \
'    "not_yet_responded": 0,' \
'    "failed": 1' \
'  },' \
'  "status": "blocked",' \
'  "blockers": [' \
'    {' \
'      "code": "AGENT_STALLED",' \
'      "message": "Synthetic blocked replay for handoff validation.",' \
'      "severity": "blocked",' \
'      "url": "https://procontract.due-north.com/RfxResponse/Index?rfxId=synthetic",' \
'      "title": "Synthetic RFx"' \
'    }' \
'  ]' \
'}' > "$RESULT_JSON"

set +e
PROCONTRACT_ENRICH_REPLAY_RESULT_JSON="$RESULT_JSON" "$E2E_DAEMON" \
  --state-root "$REPRO_DIR/state" \
  --runtime-dir "$REPRO_DIR/runtime" \
  --artifact-root "$REPRO_DIR/artifacts" \
  --ingest procontract-enrich > "$REPRO_DIR/logs/blocked-replay.log" 2>&1
daemon_status=$?
set -e

printf 'REPRO_DIR=%s\n' "$REPRO_DIR"
printf 'DAEMON_STATUS=%s\n' "$daemon_status"
tail -80 "$REPRO_DIR/logs/blocked-replay.log"
sqlite3 -header -column "$REPRO_DIR/state/state.db" "
SELECT id, status, last_error, completed_at,
       json_extract(output_payload, '$.status') AS payload_status,
       json_extract(output_payload, '$.blockers[0].code') AS blocker_code,
       json_extract(output_payload, '$.summary.failed') AS failed_count
FROM workflow_jobs WHERE scope_id='procontract-enrich' ORDER BY created_at DESC LIMIT 1;
"
```

Observed current run:

```text
REPRO_DIR=/tmp/testproject1-procontract-enrich-blocked-replay-handoff-20260514T150526Z
DAEMON_STATUS=0
procontract tender enrichment replay completed: job_id=job-d5b263251c4a3737b5d5f2c0f7529ac4fbb31a68f830bd5e9422b1eea76dbfa1 enriched=0 extractions=0

workflow_jobs latest:
id=job-d5b263251c4a3737b5d5f2c0f7529ac4fbb31a68f830bd5e9422b1eea76dbfa1
status=completed
completed_at=2026-05-14T15:05:26.272442+00:00
payload_status=blocked
blocker_code=AGENT_STALLED
failed_count=1
```

Expected on a correct build: blocked status and blockers remain in `workflow_jobs.output_payload`; the job must not remain `running`.

## Focused Tests Already Run

All commands below passed on current checkout:

```bash
cargo test -p daemon --bin daemon tests::procontract_enrich_missing_result_marks_job_failed -- --exact
cargo test -p daemon --bin daemon tests::procontract_enrich_timeout_missing_result_marks_job_failed -- --exact
cargo test -p daemon --bin daemon tests::procontract_enrich_blocked_result_is_durable -- --exact
cargo test -p daemon --bin daemon tests::persist_tender_enrichments_updates_notice_and_documents -- --exact
cargo test -p daemon --bin daemon tests::pty_skill_timeout_without_result_records_failure -- --exact
cargo test -p daemon --bin daemon tests::pty_skill_valid_result_contract_finalizes_hung_child -- --exact
cargo fmt --check
```

Representative output for the first three:

```text
test tests::procontract_enrich_missing_result_marks_job_failed ... ok
test tests::procontract_enrich_timeout_missing_result_marks_job_failed ... ok
test tests::procontract_enrich_blocked_result_is_durable ... ok
```

## Root Cause Hypotheses

### H1 - Original daemon stale-running defect is already fixed on current checkout - likelihood high

Supporting evidence:

- `run_procontract_enrich_tender_live_loop` now calls `fail_procontract_enrich_missing_result_contract` when `executed.result_path` is `None`.
- Focused daemon tests for fast-fail/no-result and timeout/no-result pass.
- Synthetic timeout no-result run produced `workflow_jobs.status=failed`, non-null `completed_at`, durable `output_payload`, and an `error_manifests` row.
- Synthetic fast-fail no-result run produced `workflow_jobs.status=failed`, non-null `completed_at`, `cleanup_verdict=clean`, and terminal output context.

Verification:

1. Re-run the focused tests and synthetic timeout/fast-fail reproduction above.
2. Query latest `workflow_jobs` rows and confirm none remain `running`.
3. Confirm `error_manifests.error_context` includes `source_id`, `skill_id`, `session_id`, `terminal_output_path`, `cleanup_verdict`, `exit_code`, `run_manifest_path`, and `durable_step_surfaces`.

Fix scope if rejected: `crates/daemon/src/main.rs` around `run_procontract_enrich_tender_live_loop`, `fail_procontract_enrich_missing_result_contract`, and missing-result tests.

### H2 - Remaining live failure, if reproduced, is skill navigation silence despite bounded-stall instructions - likelihood medium

Supporting evidence:

- Historical follow-up manifest `prompts/machine-fork-e2e-procontract-enrich-timeout-followup-error-manifest.md` records a live run after the first daemon fix that still timed out after 1200 seconds with no `result.json`.
- Current `SKILL.md` has bounded-stall instructions, but live Stage 4b was not rerun in this handoff session.
- Synthetic checks cannot prove the nested live agent will obey the skill contract inside the authenticated ProContract portal.

Verification:

1. Do not run live Stage 4b until local focused tests, synthetic timeout, fast-fail, and blocked replay are green.
2. Reopen the preserved CDP browser to the authenticated ProContract home page.
3. Run the live resume command in the "Live Resume" section.
4. If it times out with no `result.json`, inspect the session stream and tighten `skills/procurement-ingestion/procontract-enrich-tender/SKILL.md` around the exact stall page or page-shape failure.

Fix scope if confirmed: `skills/procurement-ingestion/procontract-enrich-tender/SKILL.md`; possibly daemon prompt/launch parameters only if the skill is not receiving the updated instructions.

### H3 - Full quality gate or integration tests may reveal regressions outside the focused path - likelihood medium

Supporting evidence:

- Only focused tests and `cargo fmt --check` were run in this handoff session.
- The full daemon integration loop was not run here.
- The repo has concurrent unrelated dirty files, so the next agent must treat the tree as shared and avoid disturbing unrelated changes.

Verification:

1. Run the full quality gate from the "Full Gate" section.
2. If failures are unrelated to ProContract enrichment, record them separately and avoid broad refactors.
3. If failures implicate ProContract enrichment, fix them before live retry.

Fix scope if confirmed: depends on failing test target.

## Fix Constraints

1. Do not weaken result-contract validation. Invalid or missing `result.json` must not be treated as success.
2. Do not leave a claimed `procontract-enrich` workflow job `running` after skill exit, timeout, or kill.
3. Preserve session artifact paths and error-manifest context; the operator must be able to query what happened.
4. Preserve `status='blocked'` and `blockers[]` from valid blocked result contracts.
5. Do not write partial `result.json` checkpoints during normal successful skill processing; the PTY supervisor terminates soon after seeing a valid result contract.
6. Do not close, reset, or replace the persistent CDP browser.
7. Do not delete or recreate preserved E2E roots.
8. Use Cargo for Rust and Bun for TypeScript layers.

## Live Resume

Only run this after focused tests, synthetic timeout, fast-fail, and blocked replay are green.

```bash
cd /Users/zacfarrell/Desktop/Testproject1
source state-e2e-machine-fork-20260512T195251Z/e2e-env.sh

run_logged stage-04b-procontract-login-open-after-enrich-missing-result-current-state \
  agent-browser --cdp 9222 open "https://procontract.due-north.com/SupplierPostLoginHome#"

run_logged stage-04b-procontract-enrich-retry-after-missing-result-current-state \
  env AGENT_BROWSER_CDP_PORT=9222 "$E2E_DAEMON" \
    --state-root "$E2E_STATE_ROOT" \
    --runtime-dir "$E2E_RUNTIME_DIR" \
    --artifact-root "$E2E_ARTIFACT_ROOT" \
    --ingest procontract-enrich
```

Then query:

```bash
sqlite3 -header -column "$E2E_STATE_ROOT/state.db" "
SELECT stage, state, COUNT(*) FROM process_stage_status WHERE stage='tender_enrichment' GROUP BY stage, state;
SELECT COUNT(*) AS tender_document_sets FROM tender_document_sets;
SELECT COUNT(*) AS raw_docs_with_files FROM raw_documents WHERE storage_path IS NOT NULL;
SELECT id, kind, status, scope_id, last_error, completed_at, output_payload
FROM workflow_jobs WHERE scope_id='procontract-enrich' ORDER BY created_at DESC LIMIT 8;
SELECT id, job_id, exit_code, cleanup_verdict, terminal_output_path, run_manifest_path
FROM agent_session_logs
WHERE job_id IN (SELECT id FROM workflow_jobs WHERE scope_id='procontract-enrich')
ORDER BY created_at DESC LIMIT 8;
SELECT id, job_id, pipeline_stage, workflow_step, error_type, error_message,
       skill_path, agent_session_id, markdown_path, resolved, created_at
FROM error_manifests ORDER BY created_at DESC LIMIT 8;
"
```

Passing outcomes:

- Attachments reachable: `tender_document_sets > 0`, `raw_docs_with_files > 0`, and tender-enrichment stage rows exist.
- Portal blocked, drifted, or stalled: latest job is not `running`; `output_payload.status='blocked'` and `blockers[]` are queryable, or latest job is `failed` with explicit `last_error` and error context.
- Nested CLI fails before skill starts: latest job is `failed`, `last_error` includes `procontract enrichment skill did not produce a result contract`, and session context is durable.

## Full Gate

Run before declaring the issue fixed:

```bash
cargo fmt --check
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test -p daemon --bin daemon
for t in crates/daemon/tests/*.rs; do
  cargo test --test "$(basename "$t" .rs)" || exit 1
done
ubs crates/daemon/src/main.rs skills/procurement-ingestion/procontract-enrich-tender/SKILL.md
```

## Validate / Fix Loop

Repeat until all acceptance criteria are met:

1. Run the six focused tests listed above.
2. Run the synthetic timeout reproduction. If the latest job is `running`, fix the daemon missing-result branch.
3. Run the fast-fail reproduction. If the latest job is `running` or lacks terminal context, fix missing-result context persistence.
4. Run the blocked-result replay. If `status` or `blockers` are dropped from `output_payload`, fix `procontract_enrich_completion_payload`.
5. Run the live Stage 4b resume only after steps 1-4 pass.
6. Query `workflow_jobs`, `agent_session_logs`, `error_manifests`, `process_stage_status`, `tender_document_sets`, and `raw_documents` after every live run.
7. If the live skill produces no result before daemon timeout, inspect the current URL/title and PTY stream, then tighten `SKILL.md` with a narrower bounded blocker for that exact stall/page-shape/auth/source-blocked state.
8. Re-run the focused tests and synthetic checks after every daemon or skill edit.
9. Run the full gate.
10. Stop only when no latest `procontract-enrich` job is `running`, the outcome is durable, and either documents are persisted or a structured blocked/failure state is persisted.

## Acceptance Criteria

- Missing result contract marks the job failed durably with non-null `completed_at`.
- Timeout/no-result records `cleanup_verdict='terminated_after_timeout'`.
- Fast-fail/no-result records `cleanup_verdict='clean'`, nonzero exit, and terminal artifact context.
- Durable context includes `source_id`, `skill_id`, `session_id`, `terminal_output_path`, `cleanup_verdict`, `exit_code`, `run_manifest_path`, and `durable_step_surfaces`.
- Valid blocked `result.json` remains queryable through `workflow_jobs.output_payload.status` and `workflow_jobs.output_payload.blockers`.
- Valid enrichment `result.json` still persists tender document sets and raw document storage paths.
- Live Stage 4b either persists documents or produces a durable blocked/failure outcome before daemon timeout.
- `cargo fmt --check`, `cargo check --all-targets`, `cargo clippy --all-targets -- -D warnings`, daemon binary tests, every daemon integration target, and scoped `ubs` pass.
