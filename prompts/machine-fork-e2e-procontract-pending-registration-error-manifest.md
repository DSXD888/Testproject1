# ProContract Register-Interest Pending Registration Action - QA Error Manifest

ULTRATHINK and be meticulous. This is a forensic debugging document.

## DEFECT FINGERPRINT

| Field | Value |
|-------|-------|
| **Type** | invalid_result_contract / persistence_failure |
| **Severity** | high |
| **Reproducibility** | deterministic for the captured result contract |
| **Affected Component** | `crates/daemon/src/main.rs::run_procontract_register_interest_live_loop` and `skills/procurement-ingestion/procontract-register-interest/SKILL.md` |
| **Working Analogue** | Contracts Finder and ProContract discovery wrote valid result contracts and persisted notices. |
| **Scope** | Stage 4a ProContract register-interest performs at least one portal action, writes a result contract containing `pending_registration`, then the daemon rejects that action and leaves the workflow job `running`. |
| **Environment** | macOS Darwin 25.3.0 arm64; Bun 1.3.13; Cargo 1.96.0-nightly; `claude` 2.1.80; `agent-browser` 0.27.0; Chrome CDP 148.0.7778.97 |
| **Git State** | `main` at `e4080be docs: record contracts finder claude auth blocker` with QA ledger/fixer-handoff edits pending; unrelated staged `.DS_Store` and `bin/__pycache__` entries left untouched |

---

## REPRODUCTION PROTOCOL

Follow these steps exactly from the repo root.

### Prerequisites

- Repository: `/Users/zacfarrell/Desktop/Testproject1`
- E2E context: `/Users/zacfarrell/Desktop/Testproject1/state-e2e-machine-fork-20260512T195251Z/e2e-env.sh`
- Persistent CDP browser on `localhost:9222`
- ProContract authenticated session in the persistent browser
- Existing real Stage 1 and deterministic rows from this QA run:
  - `contract_notices`: 104 `contracts-finder`, 100 `procontract`
  - `procurement_processes`: 202
  - `notice_lots`: 204
  - `qualification_runs`: 408

### Step 1: Run Stage 4a register-interest

```bash
cd /Users/zacfarrell/Desktop/Testproject1
source state-e2e-machine-fork-20260512T195251Z/e2e-env.sh
run_logged stage-04a-procontract-interest \
  env AGENT_BROWSER_CDP_PORT=9222 "$E2E_DAEMON" \
    --state-root "$E2E_STATE_ROOT" \
    --runtime-dir "$E2E_RUNTIME_DIR" \
    --artifact-root "$E2E_ARTIFACT_ROOT" \
    --ingest procontract-interest
```

Expected: the command completes, persists `interest_registration` stage rows for every evaluated registration item, and marks the source job completed or failed with a durable error.

Actual:

```text
2026-05-12T21:38:17.600712Z  INFO PTY skill produced a valid result contract; dispatching SIGTERM after stable period runtime_ids_pid=15595 runtime_ids_pgid=15595
Error: unsupported ProContract registration action `pending_registration`
```

### Verification - Confirm Defect Present

```bash
source state-e2e-machine-fork-20260512T195251Z/e2e-env.sh
sed -n '1,220p' "$E2E_LOG_ROOT/stage-04a-procontract-interest.log"
sqlite3 -header -column "$E2E_STATE_ROOT/state.db" "
SELECT id, kind, status, scope_kind, scope_id, attempt_count, last_error, created_at, started_at, completed_at
FROM workflow_jobs WHERE scope_id='procontract-interest' ORDER BY created_at DESC LIMIT 5;
SELECT stage, state, COUNT(*) AS rows FROM process_stage_status WHERE stage='interest_registration' GROUP BY stage, state;
"
jq '{summary, action_counts: (.registrations | group_by(.action) | map({action: .[0].action, count: length}))}' \
  "$E2E_ARTIFACT_ROOT/sessions/2026/05/session-pty-5e1fb98f68771c12cb090dee2e0055d1e5f73d69345ab17faac4ed322981e8d9/workdir/result.json"
```

Output:

```text
job-974dda4de782bc61ad5d5a91fa09d48152978ef9d0ccdbd01e95bd3f2dabb256  sync_source  running  source  procontract-interest  1  created_at=2026-05-12 21:31:14  started_at=2026-05-12T21:31:14.277324+00:00

No interest_registration rows were returned.

summary:
total_evaluated=100, registered=1, already_registered=2, skipped_service=60, skipped_expiry=29, failed=0
action_counts:
already_registered=2, pending_registration=8, registered=1, skipped=89
```

### Control - Valid Action Set

```bash
sed -n '100,160p' skills/procurement-ingestion/procontract-register-interest/SKILL.md
nl -ba crates/daemon/src/main.rs | sed -n '12851,12864p'
```

The skill contract says per-item `action` is one of `registered | already_registered | skipped | failed`; the daemon persists only those same states and bails on anything else.

---

## EVIDENCE BUNDLE

### E0 - Environment Evidence

**E0.1: E2E context**

```bash
source state-e2e-machine-fork-20260512T195251Z/e2e-env.sh
printf '%s\n' "$E2E_STATE_ROOT" "$E2E_RUNTIME_DIR" "$E2E_ARTIFACT_ROOT" "$E2E_LOG_ROOT" "$E2E_DAEMON"
```

```text
/Users/zacfarrell/Desktop/Testproject1/state-e2e-machine-fork-20260512T195251Z
/Users/zacfarrell/Desktop/Testproject1/runtime-e2e-machine-fork-20260512T195251Z
/Users/zacfarrell/Desktop/Testproject1/artifacts-e2e-machine-fork-20260512T195251Z
/Users/zacfarrell/Desktop/Testproject1/artifacts-e2e-machine-fork-20260512T195251Z/qa-logs
/tmp/testproject1-target-machine-fork-20260512T195251Z/debug/daemon
```

**E0.2: Stage 1 and deterministic preconditions were satisfied**

```bash
source state-e2e-machine-fork-20260512T195251Z/e2e-env.sh
sed -n '1,220p' "$E2E_LOG_ROOT/stage-01-source-ingestion-validation.log"
sed -n '1,220p' "$E2E_LOG_ROOT/stages-02-08-validation.log"
```

```text
contract_notices: contracts-finder=104, procontract=100
procurement_processes=202
notice_lots=204
qualification_runs=408
raw_documents=204
product_candidates=0
search_profiles=0
```

### E2 - Reproduction Evidence

**E2.1: Stage 4a command failed after result contract was produced**

```bash
source state-e2e-machine-fork-20260512T195251Z/e2e-env.sh
sed -n '1,220p' "$E2E_LOG_ROOT/stage-04a-procontract-interest.log"
```

```text
$ env AGENT_BROWSER_CDP_PORT=9222 /tmp/testproject1-target-machine-fork-20260512T195251Z/debug/daemon --state-root /Users/zacfarrell/Desktop/Testproject1/state-e2e-machine-fork-20260512T195251Z --runtime-dir /Users/zacfarrell/Desktop/Testproject1/runtime-e2e-machine-fork-20260512T195251Z --artifact-root /Users/zacfarrell/Desktop/Testproject1/artifacts-e2e-machine-fork-20260512T195251Z --ingest procontract-interest
2026-05-12T21:31:14.264783Z  INFO daemon bootstrap ready socket_path=/Users/zacfarrell/Desktop/Testproject1/runtime-e2e-machine-fork-20260512T195251Z/daemon.sock state_db=/Users/zacfarrell/Desktop/Testproject1/state-e2e-machine-fork-20260512T195251Z/state.db artifact_root=/Users/zacfarrell/Desktop/Testproject1/artifacts-e2e-machine-fork-20260512T195251Z
2026-05-12T21:31:14.267451Z  WARN L4 startup sync degraded before cross-authority dispatch; local operation continues failure=remote-owned pull unavailable for shared_source_configs
2026-05-12T21:38:17.600712Z  INFO PTY skill produced a valid result contract; dispatching SIGTERM after stable period runtime_ids_pid=15595 runtime_ids_pgid=15595
Error: unsupported ProContract registration action `pending_registration`
```

**E2.2: Result contract action counts**

```bash
source state-e2e-machine-fork-20260512T195251Z/e2e-env.sh
jq '{summary, action_counts: (.registrations | group_by(.action) | map({action: .[0].action, count: length})), pending_count: ([.registrations[] | select(.action=="pending_registration")] | length)}' \
  "$E2E_ARTIFACT_ROOT/sessions/2026/05/session-pty-5e1fb98f68771c12cb090dee2e0055d1e5f73d69345ab17faac4ed322981e8d9/workdir/result.json"
```

```json
{
  "summary": {
    "total_evaluated": 100,
    "registered": 1,
    "already_registered": 2,
    "skipped_service": 60,
    "skipped_expiry": 29,
    "failed": 0
  },
  "action_counts": [
    { "action": "already_registered", "count": 2 },
    { "action": "pending_registration", "count": 8 },
    { "action": "registered", "count": 1 },
    { "action": "skipped", "count": 89 }
  ],
  "pending_count": 8
}
```

**E2.3: Portal-visible actions in the result**

```bash
source state-e2e-machine-fork-20260512T195251Z/e2e-env.sh
jq -r '.registrations[] | select(.action=="registered" or .action=="already_registered") | [.notice_id,.process_id,.action,.registered_at,.title] | @tsv' \
  "$E2E_ARTIFACT_ROOT/sessions/2026/05/session-pty-5e1fb98f68771c12cb090dee2e0055d1e5f73d69345ab17faac4ed322981e8d9/workdir/result.json"
```

```text
pc-ecd07d9f-5249-f111-813c-005056b64545  proc-pc-ecd07d9f-5249-f111-813c-005056b64545  registered          2026-05-12T22:32:10Z  BEST START FAMILY HUBS OUTREACH SERVICES
pc-82498b3f-d31b-f111-813c-005056b64545  proc-pc-82498b3f-d31b-f111-813c-005056b64545  already_registered  2026-05-04T14:01:54Z  26023 Traffic & Flood Monitoring CCTV Camera Network
pc-ffb5deb6-a626-f111-813c-005056b64545  proc-pc-ffb5deb6-a626-f111-813c-005056b64545  already_registered  2026-04-13T18:33:39Z  509_27 Parking Management Solutions
```

**E2.4: `pending_registration` entries that triggered persistence failure**

```bash
source state-e2e-machine-fork-20260512T195251Z/e2e-env.sh
jq -r '.registrations[] | select(.action=="pending_registration") | [.notice_id,.process_id,.title,.source_url] | @tsv' \
  "$E2E_ARTIFACT_ROOT/sessions/2026/05/session-pty-5e1fb98f68771c12cb090dee2e0055d1e5f73d69345ab17faac4ed322981e8d9/workdir/result.json"
```

```text
pc-7ceec675-8223-f111-813c-005056b64545  proc-pc-7ceec675-8223-f111-813c-005056b64545  274_26 Grounds Maintenance Machinery  https://procontract.due-north.com/Advert?advertId=7ceec675-8223-f111-813c-005056b64545
pc-f25bb10d-131e-f111-813c-005056b64545  proc-pc-f25bb10d-131e-f111-813c-005056b64545  301_26 The supply of Liquid Fuels  https://procontract.due-north.com/Advert?advertId=f25bb10d-131e-f111-813c-005056b64545
pc-a0a0b3eb-ebd5-f011-813a-005056b64545  proc-pc-a0a0b3eb-ebd5-f011-813a-005056b64545  16_26 Flooring (Supply, Fit and Refurbishment)  https://procontract.due-north.com/Advert?advertId=a0a0b3eb-ebd5-f011-813a-005056b64545
pc-7e2a6fb2-67a3-f011-813a-005056b64545  proc-pc-7e2a6fb2-67a3-f011-813a-005056b64545  BSH/HRA/Metal Works - Dynamic Purchasing System (DPS)  https://procontract.due-north.com/Advert?advertId=7e2a6fb2-67a3-f011-813a-005056b64545
pc-7046e8ec-2734-ed11-8119-005056b64545  proc-pc-7046e8ec-2734-ed11-8119-005056b64545  BSH/HRA/Supply of Double Glazed Units and Sheet Glass - DPS  https://procontract.due-north.com/Advert?advertId=7046e8ec-2734-ed11-8119-005056b64545
pc-b7aaab27-1f34-ed11-8119-005056b64545  proc-pc-b7aaab27-1f34-ed11-8119-005056b64545  BSH/HRA/Joinery Timber Supplies - DPS  https://procontract.due-north.com/Advert?advertId=b7aaab27-1f34-ed11-8119-005056b64545
pc-9a5cf5e3-36d0-eb11-810c-005056b64545  proc-pc-9a5cf5e3-36d0-eb11-810c-005056b64545  Provision and maintenance of Mobile Plant  https://procontract.due-north.com/Advert?advertId=9a5cf5e3-36d0-eb11-810c-005056b64545
pc-45c1e490-2ad6-e911-80fa-005056b64545  proc-pc-45c1e490-2ad6-e911-80fa-005056b64545  902-19 Supply and Application of Bituminous Based Binder Products in Highways Maintenance  https://procontract.due-north.com/Advert?advertId=45c1e490-2ad6-e911-80fa-005056b64545
```

**E2.5: Database state after failure**

```bash
source state-e2e-machine-fork-20260512T195251Z/e2e-env.sh
sed -n '1,220p' "$E2E_LOG_ROOT/stage-04a-procontract-interest-forensics-db.log"
```

```text
job-974dda4de782bc61ad5d5a91fa09d48152978ef9d0ccdbd01e95bd3f2dabb256  sync_source  running  source  procontract-interest  attempt_count=1
session-pty-5e1fb98f68771c12cb090dee2e0055d1e5f73d69345ab17faac4ed322981e8d9  procontract-register-interest  completed  exit_code=143  cleanup_verdict=terminated_after_result_contract
pending_intents=0
```

### E3 - Codepath Evidence

**E3.1: Daemon pre-filter serializes register candidates as `pending_registration` in the evaluation report**

```bash
nl -ba crates/daemon/src/main.rs | sed -n '12490,12504p'
```

```rust
 12490 fn evaluation_report_entry(evaluation: &ProcontractInterestEvaluation) -> Value {
 12491     json!({
 12492         "notice_id": evaluation.notice_id,
 12493         "process_id": evaluation.process_id,
 12494         "source_url": evaluation.source_url,
 12495         "title": evaluation.title,
 12496         "action": if evaluation.action == "register" {
 12497             "pending_registration"
 12498         } else {
 12499             "skipped"
 12500         },
```

**E3.2: Skill instructions require copied `pre_filter_evaluated` skipped rows but only portal-action results for `contracts_to_register`**

```bash
sed -n '80,145p' skills/procurement-ingestion/procontract-register-interest/SKILL.md
```

```text
If `pre_filter_evaluated` is present:
- Any item already marked `action: "skipped"` should be copied through to the final `registrations[]` unchanged.
- Only perform portal actions for the items in `contracts_to_register`.

Per-item fields:
action:                one of registered | already_registered | skipped | failed
```

**E3.3: Persistence rejects `pending_registration`**

```bash
nl -ba crates/daemon/src/main.rs | sed -n '12851,12864p'
```

```rust
 12858             let state = match registration.action.as_str() {
 12859                 "registered" | "already_registered" => "completed",
 12860                 "skipped" => "skipped",
 12861                 "failed" => "failed",
 12862                 other => bail!("unsupported ProContract registration action `{other}`"),
 12863             };
```

---

## CODEPATH COMPARISON

### Working Path: Discovery And Deterministic Pipeline

Entry points:

- `--ingest contracts-finder`
- `--ingest procontract`
- `--pipeline procontract`
- `--pipeline contracts-finder`

Observed result: source notices and deterministic qualification rows persist successfully.

### Broken Path: ProContract Register Interest

Entry point: `run_procontract_register_interest_live_loop`

Flow:

1. Daemon builds `pre_filter_evaluated` with `pending_registration` for qualifying register candidates.
2. Daemon sends capped `contracts_to_register` plus full `pre_filter_evaluated` to the skill.
3. Skill registers one opportunity, reports two already registered opportunities, skips 89, and copies eight unacted qualifying rows as `pending_registration`.
4. Daemon reads the valid JSON result contract.
5. `persist_interest_registrations` rejects `pending_registration`.
6. CLI exits non-zero and leaves the source job `running` rather than completed/failed.

### Divergence Points

| # | Expected Contract | Actual Contract | Impact |
|---|---|---|---|
| 1 | Final skill output actions are `registered`, `already_registered`, `skipped`, or `failed`. | Eight rows have `pending_registration`. | Daemon rejects the result after portal mutation. |
| 2 | Every evaluated row should get a terminal persisted stage state. | Persistence aborts before writing `interest_registration` rows. | Downstream enrichment cannot rely on stage state. |
| 3 | Failing live-loop command should leave a failed job row. | Job remains `running` after CLI exits with error. | Scheduler/operator UI may see stale running work. |

---

## ROOT CAUSE HYPOTHESES

### H1: Skill copied daemon-only `pending_registration` evaluation rows into final output - LIKELIHOOD: HIGH

**Description**: `pending_registration` appears to be a daemon planning label in `pre_filter_evaluated`, not a terminal result action. The skill copied unacted register candidates into `registrations[]` without converting them to `skipped`, `failed`, or another daemon-supported terminal action.

**Supporting evidence**: E2.2, E2.4, E3.1, E3.2, E3.3.

**Contradicting evidence**: None.

**Verification steps**:

1. Inspect `contracts_to_register` count in the result-session workdir or reconstructed prompt.
2. Compare capped targets to the eight `pending_registration` rows.
3. Confirm that those rows were not acted because of cap/time/selection behavior.

**Fix scope**: skill instructions and/or daemon result normalization before persistence.

### H2: Daemon should accept `pending_registration` as a non-terminal planning state - LIKELIHOOD: MEDIUM

**Description**: If `pending_registration` is intentionally part of the final result schema, `persist_interest_registrations` needs an explicit mapping and downstream semantics.

**Supporting evidence**: E3.1 shows daemon itself emits `pending_registration` in `pre_filter_evaluated`.

**Contradicting evidence**: The skill contract says final per-item action is only `registered | already_registered | skipped | failed`; persistence rejects anything else.

**Verification steps**:

1. Inspect result schema for `procontract-register-interest`.
2. Decide whether final output should be terminal only or may include pending rows.
3. If pending is allowed, add schema, persistence, UI, and retry semantics.

**Fix scope**: result schema, Rust contract type, persistence, tests, UI expectations.

### H3: Error handling after persistence failure fails to mark the job failed - LIKELIHOOD: HIGH

**Description**: The daemon inserts the session log and then errors in `persist_interest_registrations`; the error propagates out of the CLI without calling `fail_job`, leaving `workflow_jobs.status='running'`.

**Supporting evidence**: E2.1 and E2.5.

**Contradicting evidence**: None observed.

**Verification steps**:

1. Reproduce with the captured result JSON or a unit test.
2. Confirm job status stays `running` after `persist_interest_registrations` bails.
3. Add test coverage for result-persistence errors in source live loops.

**Fix scope**: `run_procontract_register_interest_live_loop` error handling around result persistence and `complete_source_job`/`fail_job` calls.

---

## INVESTIGATION ROADMAP

### Step 1: Decide the final result contract semantics

Read:

- `skills/procurement-ingestion/procontract-register-interest/SKILL.md`
- `crates/daemon/src/main.rs::evaluation_report_entry`
- `crates/daemon/src/main.rs::persist_interest_registrations`

If final outputs must be terminal, fix the skill prompt/normalization to never emit `pending_registration` in final `registrations[]`. If pending is valid, update schema and persistence deliberately.

### Step 2: Preserve the portal action evidence

Before retrying Stage 4a, account for the already-performed registration:

```text
registered: BEST START FAMILY HUBS OUTREACH SERVICES
notice_id: pc-ecd07d9f-5249-f111-813c-005056b64545
registered_at: 2026-05-12T22:32:10Z
```

The retry must be idempotent and should see that item as already registered.

### Step 3: Fix stale running job behavior

Add a regression path that feeds a malformed/unsupported ProContract registration action and asserts the job is failed with a durable error rather than left `running`.

### Step 4: Retry exact command

After the fix:

```bash
source /Users/zacfarrell/Desktop/Testproject1/state-e2e-machine-fork-20260512T195251Z/e2e-env.sh
run_logged stage-04a-procontract-interest-retry-after-pending-action-fix \
  env AGENT_BROWSER_CDP_PORT=9222 "$E2E_DAEMON" \
    --state-root "$E2E_STATE_ROOT" \
    --runtime-dir "$E2E_RUNTIME_DIR" \
    --artifact-root "$E2E_ARTIFACT_ROOT" \
    --ingest procontract-interest
```

Then validate:

```bash
sqlite3 -header -column "$E2E_STATE_ROOT/state.db" "
SELECT stage, state, COUNT(*) FROM process_stage_status WHERE stage='interest_registration' GROUP BY stage, state;
SELECT id, kind, status, scope_id, last_error FROM workflow_jobs WHERE scope_id='procontract-interest' ORDER BY created_at DESC LIMIT 5;
"
```

---

## FIX CONSTRAINTS

1. Do not delete or recreate the E2E state, runtime, artifact, or session directories.
2. Do not rerun portal mutation before deciding idempotency for the already registered `BEST START FAMILY HUBS OUTREACH SERVICES` opportunity.
3. Do not weaken result-contract validation into silently accepting unknown actions without explicit semantics.
4. Preserve the allowed action set or update schema, skill, daemon, and tests together.
5. Do not close the persistent CDP browser.
6. Do not disturb unrelated staged `.DS_Store` or `bin/__pycache__` entries.

---

## ACCEPTANCE CRITERIA

### Functional

- [ ] Stage 4a exits 0 or writes a structured blocked/no-action contract under a valid CDP/auth environment.
- [ ] Final `registrations[]` contains only daemon-supported terminal actions, or the daemon deliberately supports `pending_registration` with schema and persistence semantics.
- [ ] `process_stage_status` records `interest_registration` states for evaluated rows.
- [ ] The latest `procontract-interest` workflow job is not left `running` after a CLI error.

### Evidence-Based

- [ ] The retry log no longer contains `unsupported ProContract registration action`.
- [ ] `jq` action counts for the retry result contain no unsupported action names.
- [ ] SQLite validation shows completed/skipped/failed `interest_registration` rows.

### Regression

- [ ] Add or update tests for `persist_interest_registrations` action handling.
- [ ] Add or update a test proving unsupported registration actions fail the job durably rather than leaving it `running`.
- [ ] Re-run `cargo test -p daemon --test agent_procurement_ingestion` or the smallest relevant daemon test target.

### Test

- [ ] If prompt/schema changes are made, run the skill lint/golden tests that cover `procontract-register-interest`.
- [ ] Re-run the exact Stage 4a command against this preserved E2E state after the fix.

---

## FILE MAP

| File | Role | Lines Referenced |
|------|------|------------------|
| `skills/procurement-ingestion/procontract-register-interest/SKILL.md` | Final result contract instructions and allowed actions. | 1-160 |
| `crates/daemon/src/main.rs` | Interest candidate planning, result persistence, live-loop error handling. | 12368-12515, 12840-12864, 16434-16575 |
| `artifacts-e2e-machine-fork-20260512T195251Z/qa-logs/stage-04a-procontract-interest.log` | Primary failed Stage 4a daemon command log. | whole file |
| `artifacts-e2e-machine-fork-20260512T195251Z/qa-logs/stage-04a-procontract-interest-forensics-db.log` | Workflow/session/stage state after failure. | whole file |
| `artifacts-e2e-machine-fork-20260512T195251Z/sessions/2026/05/session-pty-5e1fb98f68771c12cb090dee2e0055d1e5f73d69345ab17faac4ed322981e8d9/workdir/result.json` | Result contract containing unsupported `pending_registration` actions. | whole file |
