# ProContract Enrichment Timeout / Missing Result Fix Handoff

ULTRATHINK and be meticulous.

## MISSION

Fix the Stage 4b ProContract enrichment failure where `--ingest procontract-enrich` can execute a PTY skill session that produces no valid `result.json`, then returns `Error: procontract enrichment skill did not produce a result contract` while the `workflow_jobs` row remains `running`.

You must fix both sides of the defect:

1. The daemon must durably fail or durably block the `procontract-enrich` workflow job whenever the enrichment skill exits, times out, or is killed without a valid result contract. It must not leave the job `running`.
2. The enrichment skill must have bounded navigation/stall checkpoints and must emit a valid structured blocked `result.json` for auth, page-shape, source-blocked, or stalled portal states before the daemon wall-clock timeout.

Do not weaken result-contract validation. Do not delete or recreate preserved E2E state, runtime, artifact, or session directories. Do not close or replace the persistent CDP browser.

## CURRENT REPRODUCTION STATUS

The original user-supplied manifest captured a live portal timeout:

- Command log: `artifacts-e2e-machine-fork-20260512T195251Z/qa-logs/stage-04b-procontract-enrich.log`
- Job: `job-cf642ec3d9e68b1fe4e28e94eb9eaca37126e59f8dc7c300b091263ac0109755`
- Session: `session-pty-33fd5d63d698179f0b393ea6f8a3481a2c00f2191e3773017ee8e836aa4b7757`
- Session cleanup: `terminated_after_timeout`
- Daemon error: `procontract enrichment skill did not produce a result contract`
- No `run_manifest_path`, no `workdir/result.json`, no tender document rows.

I then recreated the defect on the same preserved E2E roots on 2026-05-12 at `22:44:40Z`. The live retry did not spend 1200 seconds because the nested Claude CLI hit its quota guard immediately, but it reproduced the same missing-result/stale-running daemon contract:

```bash
cd /Users/zacfarrell/Desktop/Testproject1
source state-e2e-machine-fork-20260512T195251Z/e2e-env.sh

run_logged stage-04b-procontract-login-open-recreate-20260512T224440Z \
  agent-browser --cdp 9222 open "https://procontract.due-north.com/SupplierPostLoginHome#"
run_logged stage-04b-procontract-login-title-recreate-20260512T224440Z \
  agent-browser --cdp 9222 get title
run_logged stage-04b-procontract-login-snapshot-recreate-20260512T224440Z \
  agent-browser --cdp 9222 snapshot

run_logged stage-04b-procontract-enrich-recreate-20260512T224440Z \
  env AGENT_BROWSER_CDP_PORT=9222 "$E2E_DAEMON" \
    --state-root "$E2E_STATE_ROOT" \
    --runtime-dir "$E2E_RUNTIME_DIR" \
    --artifact-root "$E2E_ARTIFACT_ROOT" \
    --ingest procontract-enrich
```

Observed preflight:

```text
Supplier Post-Login Home
snapshot contained SORELLON LIMITED, Opportunities, Company details summary, Vendor profile, Workgroups
```

Observed daemon output:

```text
2026-05-12T22:44:59.042239Z  INFO daemon bootstrap ready socket_path=/Users/zacfarrell/Desktop/Testproject1/runtime-e2e-machine-fork-20260512T195251Z/daemon.sock state_db=/Users/zacfarrell/Desktop/Testproject1/state-e2e-machine-fork-20260512T195251Z/state.db artifact_root=/Users/zacfarrell/Desktop/Testproject1/artifacts-e2e-machine-fork-20260512T195251Z
2026-05-12T22:44:59.044725Z  WARN L4 startup sync degraded before cross-authority dispatch; local operation continues failure=remote-owned pull unavailable for shared_source_configs
Error: procontract enrichment skill did not produce a result contract
```

Fresh live retry rows:

```text
workflow_jobs:
job-a855cfe11399a490f377a6bc3c756aa1f35bce922e9b53197c286a81b3a59990
kind=sync_source scope_id=procontract-enrich status=running attempt_count=1
started_at=2026-05-12T22:44:59.049606+00:00 completed_at=NULL last_error=NULL

agent_session_logs:
session-pty-5dfb38a0afd70ebc29d256727129ce36edae9ffd427d66c5b135bce3dd8027b1
job_id=job-a855cfe11399a490f377a6bc3c756aa1f35bce922e9b53197c286a81b3a59990
skill_path=skills/procurement-ingestion/procontract-enrich-tender/SKILL.md
execution_mode=pty cli_vendor=claude status=completed exit_code=1 cleanup_verdict=clean
terminal_output_bytes=133 run_manifest_path=NULL
```

Fresh session artifact:

```bash
zstdcat "$E2E_ARTIFACT_ROOT/sessions/2026/05/session-pty-5dfb38a0afd70ebc29d256727129ce36edae9ffd427d66c5b135bce3dd8027b1/stream/000001.pty.zst"
```

```text
You've hit your limit - resets 2:50am (Europe/London)
```

Downstream rows after the live retry:

```text
tender_document_sets=0
raw_documents_total=204
raw_docs_with_files=0
document_extractions=0
semantic_extractions=0
product_candidates=0
search_profiles=0
```

I also created an isolated deterministic timeout reproduction under `/tmp` so you can exercise the original timeout branch without touching the live portal:

```bash
REPRO_DIR=/tmp/testproject1-procontract-enrich-timeout-repro-20260512T224440Z
FAKE_CLAUDE="$REPRO_DIR/fake-claude-sleep.sh"

PTY_SKILL_TIMEOUT_SECONDS=2 "$E2E_DAEMON" \
  --agent-cli-program "$FAKE_CLAUDE" \
  --state-root "$REPRO_DIR/state" \
  --runtime-dir "$REPRO_DIR/runtime" \
  --artifact-root "$REPRO_DIR/artifacts" \
  --ingest procontract-enrich
```

Observed synthetic timeout:

```text
2026-05-12T22:48:16.831003Z  WARN PTY skill exceeded wall-clock timeout; dispatching SIGTERM runtime_ids_pid=63962 runtime_ids_pgid=63962 timeout_seconds=2
Error: procontract enrichment skill did not produce a result contract
```

Synthetic timeout rows:

```text
workflow_jobs:
job-de39076ae5a25d6f4f9dbb1b103a72d13d70bd9f43bf395f54985c39ef90d4c3
status=running scope_id=procontract-enrich started_at=2026-05-12T22:48:14.489033+00:00 completed_at=NULL last_error=NULL

agent_session_logs:
session-pty-67ae8123bf7789630280a9d1db35b6e1d073a2333e7c747fa936b87ccc8822c6
exit_code=1 cleanup_verdict=terminated_after_timeout terminal_output_bytes=0 run_manifest_path=NULL
```

This proves the daemon failure path is independent of the portal: if `executed.result_path` is `None`, the run returns an error after inserting the session log and before calling `fail_job`.

## ENVIRONMENT SNAPSHOT

```text
Repo: /Users/zacfarrell/Desktop/Testproject1
HEAD: d1d6ba89a566b17f647fb0f0e365bf10b87a6c9a
Last commit: d1d6ba8 docs: record procontract enrichment timeout blocker
Branch: main...origin/main
Existing unrelated worktree changes at handoff time:
  M .DS_Store
  A bin/__pycache__/br-claimcpython-313.pyc
  A bin/__pycache__/br-completecpython-313.pyc
OS: Darwin 25.3.0 arm64
Cargo: cargo 1.96.0-nightly
Bun: 1.3.13
agent-browser: 0.27.0
Claude Code: 2.1.139
E2E PRAGMA user_version: 9
```

## CODE CONTEXT

Read these before changing code:

- `crates/daemon/src/main.rs:16662` - `run_procontract_enrich_tender_live_loop`
- `crates/daemon/src/main.rs:16741` - live PTY session execution and session-log insertion
- `crates/daemon/src/main.rs:16749` - current bug: `executed.result_path.as_ref().ok_or_else(...)` returns an error without failing the job
- `crates/daemon/src/main.rs:15689` - PTY supervisor result-path detection
- `crates/daemon/src/main.rs:15706` - `cleanup_verdict` becomes `terminated_after_timeout` on timeout
- `crates/daemon/src/main.rs:19778` - `fail_job`
- `crates/daemon/src/main.rs:19714` - `fail_with_manifest`
- `crates/daemon/src/main.rs:19847` - `create_error_manifest`
- `skills/procurement-ingestion/procontract-enrich-tender/SKILL.md:34` - declared failure codes include `AGENT_STALLED`
- `skills/procurement-ingestion/procontract-enrich-tender/SKILL.md:58` and `:88` - blocked result contract requirements
- `skills/procurement-ingestion/procontract-enrich-tender/SKILL.md:139` - risky live-event navigation step
- `skills/procurement-ingestion/procontract-enrich-tender/SKILL.md:235` - final result contract section

Current daemon bug:

```rust
insert_executed_session_log(runtime.state_db.connection(), &skill, &job_id, &executed)?;
let result_path = executed
    .result_path
    .as_ref()
    .ok_or_else(|| anyhow!("procontract enrichment skill did not produce a result contract"))?;
```

Because the `ok_or_else` returns `Err`, the function never calls `complete_source_job`, `fail_job`, or `fail_with_manifest`. The job lease has already been claimed, so the row stays `running`.

## IMPLEMENTATION REQUIREMENTS

### R1 - Daemon missing-result failure path

Patch `run_procontract_enrich_tender_live_loop` so a missing `executed.result_path` after session execution:

- Creates an error manifest or otherwise records equivalent structured failure context.
- Marks the claimed `workflow_jobs` row `failed`.
- Sets `last_error` to a useful message that includes `procontract enrichment skill did not produce a result contract`.
- Includes at least these fields in durable context, either in `output_payload` and/or the `error_manifests` row: `source_id`, `skill_id`, `session_id`, `terminal_output_path`, `cleanup_verdict`, `exit_code`, `run_manifest_path` if present, and `durable_step_surfaces`.
- Returns a `LiveLoopReport` with `status="failed"` instead of bubbling a raw anyhow error.

Prefer extracting a small helper so the regression test can exercise the branch without launching a real agent. Keep the successful result path unchanged.

### R2 - Timeout and fast-fail both covered

The fix must handle both observed variants:

- Timeout/no-result: `cleanup_verdict='terminated_after_timeout'`, exit code non-zero, no terminal bytes.
- Fast-fail/no-result: `cleanup_verdict='clean'`, exit code non-zero, terminal output contains an upstream CLI failure such as quota exhaustion.

Both must mark the job failed durably and must not remain `running`.

### R3 - Structured blocked result handling

If the skill writes a valid blocked result such as:

```json
{
  "cursor": "{\"blocked_at\":\"2026-05-12T22:00:00Z\"}",
  "continuation_required": false,
  "enriched_tenders": [],
  "summary": {
    "total_checked": 1,
    "enriched": 0,
    "not_yet_responded": 0,
    "failed": 1
  },
  "status": "blocked",
  "blockers": [
    {
      "code": "AGENT_STALLED",
      "message": "Timed out opening the live RFx event from the activity dashboard.",
      "severity": "blocked"
    }
  ]
}
```

the daemon must persist a durable non-running outcome. Choose the project-consistent status (`failed` with a blocker last_error, or completed with `output_payload.status='blocked'`), but make it queryable and do not drop `status`/`blockers` from the job output. The current completion payload only includes `summary`, so extend it if needed.

### R4 - Skill bounded-stall contract

Update `skills/procurement-ingestion/procontract-enrich-tender/SKILL.md` so the agent cannot silently navigate until daemon timeout. Add explicit instructions:

- Before portal work, record `started_at` and an internal deadline comfortably below the daemon timeout.
- Every page transition has a bounded retry budget.
- Each activity has a bounded processing budget.
- If a required page shape, live tender event, RFx summary, or document section is not reachable within budget, write a valid blocked `result.json` with `status='blocked'`, a `blockers[]` entry, current URL/title when available, and summary counts.
- If some tenders were enriched before a blocker, include them in `enriched_tenders[]` and set `summary.failed` plus blockers. Do not leave silence as the failure mode.
- Leave the persistent CDP browser open.

Do not tell the skill to write partial `result.json` checkpoints during normal successful processing: the daemon supervisor terminates the PTY child shortly after it sees a valid `result.json`. Only write `result.json` when the run is complete or blocked.

### R5 - No live retry until local gates pass

Do not rerun the live Stage 4b portal flow until daemon unit tests, synthetic timeout reproduction, and blocked-contract replay/fake tests are green. After that, use the preserved resume point below.

## REQUIRED TESTS

Add focused regression coverage. Suggested test names:

```bash
cargo test --bin daemon tests::procontract_enrich_missing_result_marks_job_failed -- --exact
cargo test --bin daemon tests::procontract_enrich_timeout_missing_result_marks_job_failed -- --exact
cargo test --bin daemon tests::procontract_enrich_blocked_result_is_durable -- --exact
cargo test --bin daemon tests::persist_tender_enrichments_updates_notice_and_documents -- --exact
cargo test --bin daemon tests::pty_skill_timeout_without_result_records_failure -- --exact
cargo test --bin daemon tests::pty_skill_valid_result_contract_finalizes_hung_child -- --exact
```

If full-loop tests are hard to wire directly, extract a helper for the missing-result branch and test it with an `ExecutedSession` fixture containing:

```text
session_id=session-pty-test
execution_mode=pty
cli_vendor=claude
exit_code=143
cleanup_verdict=terminated_after_timeout
terminal_output_path=/tmp/session/stream.index.jsonl
result_path=None
```

The test must assert:

```sql
SELECT status, last_error, completed_at, output_payload
FROM workflow_jobs WHERE id = ?;
```

Expected:

```text
status=failed
last_error contains "procontract enrichment skill did not produce a result contract"
completed_at is not NULL
output_payload or error_manifest contains session_id and terminal_output_path
```

Also add a blocked-contract test that verifies the daemon does not discard `status='blocked'` and `blockers[]` from a valid enrichment result.

## SYNTHETIC VALIDATION REPRO

Use this deterministic timeout harness before touching the live portal. It writes only under `/tmp`.

```bash
cd /Users/zacfarrell/Desktop/Testproject1
source state-e2e-machine-fork-20260512T195251Z/e2e-env.sh

REPRO_DIR=/tmp/testproject1-procontract-enrich-timeout-validation-$(date -u +%Y%m%dT%H%M%SZ)
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

tail -80 "$REPRO_DIR/logs/synthetic-timeout.log"
sqlite3 -header -column "$REPRO_DIR/state/state.db" "
SELECT id, status, last_error, completed_at, output_payload
FROM workflow_jobs WHERE scope_id='procontract-enrich' ORDER BY created_at DESC LIMIT 1;
SELECT id, job_id, exit_code, cleanup_verdict, terminal_output_path, run_manifest_path
FROM agent_session_logs ORDER BY created_at DESC LIMIT 1;
"
```

Before the fix, this leaves the job `running`. After the fix, it must leave the latest `procontract-enrich` job `failed` with non-null `completed_at` and durable session context.

## LIVE RESUME AFTER FIX

Only after local gates and synthetic validation are green:

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
SELECT id, kind, status, scope_id, last_error, completed_at, output_payload
FROM workflow_jobs WHERE scope_id='procontract-enrich' ORDER BY created_at DESC LIMIT 8;
SELECT id, job_id, exit_code, cleanup_verdict, terminal_output_path, run_manifest_path
FROM agent_session_logs
WHERE job_id IN (SELECT id FROM workflow_jobs WHERE scope_id='procontract-enrich')
ORDER BY created_at DESC LIMIT 8;
"
```

Passing outcomes:

- If attachments are reachable: `tender_document_sets > 0`, `raw_docs_with_files > 0`, and `tender_enrichment` rows exist.
- If the portal blocks, drifts, or stalls: a valid structured blocked result is durable and the job is not `running`.
- If the nested CLI fails before the skill starts: job is `failed`, last_error is explicit, and session context is durable.

## FULL QUALITY GATE

After code or skill changes, run:

```bash
cargo fmt --check
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test -p daemon --bin daemon
for t in crates/daemon/tests/*.rs; do
  cargo test --test "$(basename "$t" .rs)" || exit 1
done
```

Also run `ubs` on changed files before committing:

```bash
ubs crates/daemon/src/main.rs skills/procurement-ingestion/procontract-enrich-tender/SKILL.md
```

## VALIDATE / FIX LOOP

Repeat until every acceptance criterion is green:

1. Run the focused unit tests.
2. Run the synthetic timeout validation. If the latest job is still `running`, fix the daemon missing-result branch.
3. Run a blocked-result replay/fake test. If `status`/`blockers` are dropped, fix the daemon completion payload or blocked-state persistence.
4. Run the live Stage 4b resume. If the skill produces no result, patch `SKILL.md` with a narrower stall/page-shape blocker and retry after local validation.
5. Query `workflow_jobs`, `agent_session_logs`, `process_stage_status`, `tender_document_sets`, and `raw_documents` after every run.
6. Run the full quality gate. Fix any format, check, clippy, unit, or integration failures.
7. Stop only when no `procontract-enrich` job from the latest run is `running`, the outcome is durable, and either documents are persisted or a structured blocked state is persisted.

## ACCEPTANCE CRITERIA

- Missing result contract from `run_procontract_enrich_tender_live_loop` marks the job failed durably and includes session/artifact context.
- Timeout without result contract marks the job failed durably and includes `cleanup_verdict='terminated_after_timeout'`.
- Fast upstream CLI failure without result contract marks the job failed durably and includes terminal artifact context.
- Valid blocked `result.json` does not get lost; `status` and `blockers` remain queryable.
- Valid enrichment `result.json` still persists tender document sets and raw document storage paths.
- Skill instructions prevent silent 1200-second portal stalls by requiring a blocked result for bounded stall/page-shape/auth failures.
- `cargo fmt --check`, `cargo check --all-targets`, `cargo clippy --all-targets -- -D warnings`, daemon binary tests, and every daemon integration test target pass.
