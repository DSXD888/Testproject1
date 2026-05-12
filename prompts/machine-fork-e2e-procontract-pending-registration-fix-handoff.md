# ProContract Pending Registration Persistence Fix Handoff

ULTRATHINK and be meticulous.

## MISSION

Fix the Stage 4a ProContract register-interest failure where the skill writes `pending_registration` actions in `registrations[]`, the daemon rejects that action during persistence, and the `procontract-interest` workflow job remains stuck in `running`.

The fix must make the result contract semantics explicit and durable. Do not silently accept arbitrary unknown actions. Do not rerun live portal mutation until the idempotency and retry behavior is clear.

Primary error manifest:

- `prompts/machine-fork-e2e-procontract-pending-registration-error-manifest.md`

Affected files:

- `skills/procurement-ingestion/procontract-register-interest/SKILL.md`
- `crates/daemon/src/main.rs`
- likely tests in `crates/daemon/src/main.rs` unit module
- likely integration coverage in `crates/daemon/tests/agent_procurement_ingestion.rs` or a focused daemon test target

Out of scope:

- deleting or recreating E2E state, runtime, artifacts, or session directories
- closing or resetting the persistent CDP browser
- weakening result validation into a catch-all fallback
- disturbing unrelated dirty/staged files such as `.DS_Store` or `bin/__pycache__`

## REPRODUCED STATE

Verified from `/Users/zacfarrell/Desktop/Testproject1` on `2026-05-12`.

Current repo head when this handoff was created:

```bash
git log --oneline -1
```

Observed:

```text
1a7c9fa docs: pause machine fork e2e on procontract action contract
```

Dirty state at verification time:

```text
## main...origin/main
M  .DS_Store
A  bin/__pycache__/br-claimcpython-313.pyc
A  bin/__pycache__/br-completecpython-313.pyc
```

Treat these as shared-checkout noise. Do not delete, stash, revert, or overwrite them.

E2E context exists:

```bash
source state-e2e-machine-fork-20260512T195251Z/e2e-env.sh
printf '%s\n' "$E2E_STATE_ROOT" "$E2E_RUNTIME_DIR" "$E2E_ARTIFACT_ROOT" "$E2E_LOG_ROOT" "$E2E_DAEMON"
```

Observed:

```text
/Users/zacfarrell/Desktop/Testproject1/state-e2e-machine-fork-20260512T195251Z
/Users/zacfarrell/Desktop/Testproject1/runtime-e2e-machine-fork-20260512T195251Z
/Users/zacfarrell/Desktop/Testproject1/artifacts-e2e-machine-fork-20260512T195251Z
/Users/zacfarrell/Desktop/Testproject1/artifacts-e2e-machine-fork-20260512T195251Z/qa-logs
/tmp/testproject1-target-machine-fork-20260512T195251Z/debug/daemon
```

## NON-MUTATING REPRODUCTION

Do not start by rerunning the live Stage 4a command. The preserved run already performed at least one real portal mutation. Recreate the defect from the captured log, result contract, and DB state first.

```bash
source state-e2e-machine-fork-20260512T195251Z/e2e-env.sh
sed -n '1,220p' "$E2E_LOG_ROOT/stage-04a-procontract-interest.log"
```

Observed:

```text
$ env AGENT_BROWSER_CDP_PORT=9222 /tmp/testproject1-target-machine-fork-20260512T195251Z/debug/daemon --state-root /Users/zacfarrell/Desktop/Testproject1/state-e2e-machine-fork-20260512T195251Z --runtime-dir /Users/zacfarrell/Desktop/Testproject1/runtime-e2e-machine-fork-20260512T195251Z --artifact-root /Users/zacfarrell/Desktop/Testproject1/artifacts-e2e-machine-fork-20260512T195251Z --ingest procontract-interest
2026-05-12T21:31:14.264783Z  INFO daemon bootstrap ready socket_path=/Users/zacfarrell/Desktop/Testproject1/runtime-e2e-machine-fork-20260512T195251Z/daemon.sock state_db=/Users/zacfarrell/Desktop/Testproject1/state-e2e-machine-fork-20260512T195251Z/state.db artifact_root=/Users/zacfarrell/Desktop/Testproject1/artifacts-e2e-machine-fork-20260512T195251Z
2026-05-12T21:31:14.267451Z  WARN L4 startup sync degraded before cross-authority dispatch; local operation continues failure=remote-owned pull unavailable for shared_source_configs
2026-05-12T21:38:17.600712Z  INFO PTY skill produced a valid result contract; dispatching SIGTERM after stable period runtime_ids_pid=15595 runtime_ids_pgid=15595
Error: unsupported ProContract registration action `pending_registration`
```

Result action counts:

```bash
source state-e2e-machine-fork-20260512T195251Z/e2e-env.sh
jq '{summary, action_counts: (.registrations | group_by(.action) | map({action: .[0].action, count: length})), pending_count: ([.registrations[] | select(.action=="pending_registration")] | length)}' \
  "$E2E_ARTIFACT_ROOT/sessions/2026/05/session-pty-5e1fb98f68771c12cb090dee2e0055d1e5f73d69345ab17faac4ed322981e8d9/workdir/result.json"
```

Observed:

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

Database state after the failure:

```bash
source state-e2e-machine-fork-20260512T195251Z/e2e-env.sh
sqlite3 -header -column "$E2E_STATE_ROOT/state.db" "
SELECT id, kind, status, scope_kind, scope_id, attempt_count, last_error, created_at, started_at, completed_at
FROM workflow_jobs WHERE scope_id='procontract-interest' ORDER BY created_at DESC LIMIT 5;
SELECT stage, state, COUNT(*) AS rows FROM process_stage_status WHERE stage='interest_registration' GROUP BY stage, state;
"
```

Observed:

```text
id                                                                    kind         status   scope_kind  scope_id              attempt_count  last_error  created_at           started_at                        completed_at
--------------------------------------------------------------------  -----------  -------  ----------  --------------------  -------------  ----------  -------------------  --------------------------------  ------------
job-974dda4de782bc61ad5d5a91fa09d48152978ef9d0ccdbd01e95bd3f2dabb256  sync_source  running  source      procontract-interest  1                          2026-05-12 21:31:14  2026-05-12T21:31:14.277324+00:00
```

No `interest_registration` stage rows were returned.

## ROOT CAUSE CONTEXT

The daemon labels register-eligible pre-filter rows as `pending_registration` before sending them to the skill:

```rust
fn evaluation_report_entry(evaluation: &ProcontractInterestEvaluation) -> Value {
    json!({
        "notice_id": evaluation.notice_id,
        "process_id": evaluation.process_id,
        "source_url": evaluation.source_url,
        "title": evaluation.title,
        "action": if evaluation.action == "register" {
            "pending_registration"
        } else {
            "skipped"
        },
        "skip_reason": evaluation.skip_reason,
        "pre_filter_result": evaluation.pre_filter_result,
    })
}
```

The daemon only sends a capped subset of eligible register candidates for live mutation:

```rust
const MAX_PROCONTRACT_INTEREST_REGISTRATIONS_PER_RUN: usize = 3;

fn procontract_interest_registration_targets(
    evaluations: &[ProcontractInterestEvaluation],
) -> Vec<Value> {
    evaluations
        .iter()
        .filter(|evaluation| evaluation.action == "register")
        .take(MAX_PROCONTRACT_INTEREST_REGISTRATIONS_PER_RUN)
        .map(registration_job_entry)
        .collect()
}
```

The skill currently has conflicting contract language:

```text
Outputs:
- One result item per evaluated contract when `pre_filter_evaluated` is supplied

If `pre_filter_evaluated` is present:
- Any item already marked `action: "skipped"` should be copied through to the final `registrations[]` unchanged.
- Only perform portal actions for the items in `contracts_to_register`.

Per-item fields:
action: one of registered | already_registered | skipped | failed
```

The captured result followed the "one item per evaluated contract" part and copied eight unacted eligible rows as `pending_registration`, but the daemon persistence only supports terminal actions:

```rust
let state = match registration.action.as_str() {
    "registered" | "already_registered" => "completed",
    "skipped" => "skipped",
    "failed" => "failed",
    other => bail!("unsupported ProContract registration action `{other}`"),
};
```

The stale job bug is separate but triggered by the same failure. In `run_procontract_register_interest_live_loop`, the code inserts the executed session log, reads `result.json`, then uses `?` on `persist_interest_registrations`. That propagates out of the CLI without calling `fail_job`, so the row remains `running`.

## RECOMMENDED FIX DIRECTION

Recommended primary fix: support `pending_registration` deliberately as a known retryable non-terminal action, and make the skill contract say that explicitly.

Reason: the daemon caps portal mutations at 3 per run. In the captured run, the skill terminally acted on 3 opportunities and left 8 qualified opportunities unacted. Converting those 8 to `skipped` would suppress future retries because `load_procontract_interest_candidates` excludes `completed`, `succeeded`, and `skipped` rows:

```sql
AND NOT EXISTS (
  SELECT 1
  FROM process_stage_status pss
  WHERE pss.process_id = cn.process_id
    AND pss.stage = 'interest_registration'
    AND pss.state IN ('completed', 'succeeded', 'skipped')
)
```

Concrete semantics if using the recommended direction:

- `registered` -> stage state `completed`
- `already_registered` -> stage state `completed`
- `skipped` -> stage state `skipped`
- `failed` -> stage state `failed`
- `pending_registration` -> stage state `pending`
- unknown action -> error, job failed durably, no partial writes

Also update the skill `summary` schema so counts add up. Add a `pending_registration` count or equivalent explicit pending count. The captured summary has `total_evaluated=100`, but the named counters sum to only 92 because pending is not represented.

If you decide instead that final result contracts must be terminal-only, do not mark unacted qualified rows as `skipped` unless product explicitly wants to abandon them. Prefer omitting unacted eligible rows from `registrations[]`, setting `continuation_required=true`, and keeping them retryable. That path requires updating the "one result item per evaluated contract" acceptance language and tests.

## IMPLEMENTATION NOTES

1. Update `skills/procurement-ingestion/procontract-register-interest/SKILL.md`.

Clarify that `pending_registration` from `pre_filter_evaluated` is a daemon planning label and either:

- remains a supported final action only for qualified items not in `contracts_to_register`, with explicit retryable semantics, or
- must not appear in final output if you choose the terminal-only design.

For the recommended retryable-pending design, update the per-item field list:

```text
action: one of registered | already_registered | skipped | failed | pending_registration
pending_registration: use only for daemon-approved register candidates that were not in `contracts_to_register` or were intentionally not acted in this capped run; do not click anything for these rows; they remain retryable.
```

Update the example summary to include a pending count.

2. Update `ProcontractRegistrationSummary`.

Current struct:

```rust
struct ProcontractRegistrationSummary {
    total_evaluated: u32,
    registered: u32,
    already_registered: u32,
    skipped_service: u32,
    skipped_expiry: u32,
    failed: u32,
}
```

Add a field such as:

```rust
#[serde(default)]
pending_registration: u32,
```

Use `#[serde(default)]` if older replay artifacts without the field should still parse while this branch is in flight. This is not a compatibility shim for users; it is a practical replay safeguard for existing QA artifacts.

3. Update `persist_interest_registrations`.

Recommended mapping:

```rust
let state = match registration.action.as_str() {
    "registered" | "already_registered" => "completed",
    "pending_registration" => "pending",
    "skipped" => "skipped",
    "failed" => "failed",
    other => bail!("unsupported ProContract registration action `{other}`"),
};
```

Keep the current transaction boundary so an unsupported action rolls back all rows in that result.

4. Fix durable job failure.

Do not let result-persistence errors escape after a job has been claimed. Wrap `persist_interest_registrations` in `run_procontract_register_interest_live_loop` and call `fail_job` with a clear error payload on failure.

Expected behavior after the fix:

- CLI can still exit non-zero for an invalid contract.
- The corresponding `workflow_jobs` row must be `failed`, not `running`.
- `last_error` must contain the persistence failure reason.
- The output payload should include a machine-readable code such as `PROCONTRACT_INTEREST_PERSISTENCE_FAILED`, plus `session_id` and `source_id` when available.

Consider applying the same wrapper to the no-portal synthetic persistence branch, because it creates and claims a job before persistence.

5. Consider adding a replay path for testability.

`run_procontract_enrich_tender_live_loop` already supports:

```rust
PROCONTRACT_ENRICH_REPLAY_RESULT_JSON
```

A matching `PROCONTRACT_INTEREST_REPLAY_RESULT_JSON` would let tests and operators replay a saved `result.json` without using CDP or mutating the portal. If added, it should still create/claim a normal `procontract-interest` job and exercise the same persistence/fail-job path.

## RED TESTS TO ADD

Add focused unit tests in the `#[cfg(test)]` module in `crates/daemon/src/main.rs`, where private helpers are already imported.

Suggested tests:

```text
persist_interest_registrations_maps_pending_registration_to_retryable_pending
```

Test shape:

- seed a minimal ProContract notice with `seed_minimal_procontract_notice`
- call `persist_interest_registrations` with one registration whose action is `pending_registration`
- assert persisted count is 1
- assert latest `process_stage_status.state` is `pending`
- assert `load_procontract_interest_candidates` still includes that process ID, proving the pending row is retryable under the current candidate query

```text
persist_interest_registrations_rejects_unknown_action_without_partial_writes
```

Test shape:

- seed two notices
- create a result contract with one valid `registered` row and one invalid `bogus_action` row
- assert the function returns an error containing `unsupported ProContract registration action`
- assert `process_stage_status` has zero `interest_registration` rows, proving transaction rollback still works

Add one job-level regression test for the stale running job. Choose the smallest maintainable route:

- Preferred: add `PROCONTRACT_INTEREST_REPLAY_RESULT_JSON`, then an integration test that runs `daemon --ingest procontract-interest` against a temp DB and a replay result with `bogus_action`. Assert CLI failure and latest job `status='failed'` with `last_error` set.
- Alternate: factor the persistence/error wrapper into a small private helper and test it directly in `main.rs` with a `RuntimeBootstrap`.

The important assertion is not only that the CLI fails; it must prove the job row is no longer `running`.

## VALIDATE/FIX LOOP

Run this loop until every step is green.

1. Recreate the preserved failure without portal mutation.

```bash
source state-e2e-machine-fork-20260512T195251Z/e2e-env.sh
jq '{summary, action_counts: (.registrations | group_by(.action) | map({action: .[0].action, count: length}))}' \
  "$E2E_ARTIFACT_ROOT/sessions/2026/05/session-pty-5e1fb98f68771c12cb090dee2e0055d1e5f73d69345ab17faac4ed322981e8d9/workdir/result.json"
sqlite3 -header -column "$E2E_STATE_ROOT/state.db" "
SELECT id, status, scope_id, last_error FROM workflow_jobs WHERE scope_id='procontract-interest' ORDER BY created_at DESC LIMIT 5;
SELECT stage, state, COUNT(*) AS rows FROM process_stage_status WHERE stage='interest_registration' GROUP BY stage, state;
"
```

2. Add the red tests above and run the narrowest targets.

```bash
cargo test -p daemon --bin daemon persist_interest_registrations
cargo test -p daemon --bin daemon procontract_interest
```

If you add a CLI replay integration test, run it directly:

```bash
cargo test -p daemon --test agent_procurement_ingestion procontract_interest
```

3. Implement the Rust fix and skill contract update.

Keep edits scoped to `crates/daemon/src/main.rs`, `skills/procurement-ingestion/procontract-register-interest/SKILL.md`, and the focused test file(s).

4. Re-run focused Rust tests.

```bash
cargo test -p daemon --bin daemon persist_interest_registrations
cargo test -p daemon --bin daemon procontract_interest
cargo test -p daemon --test agent_procurement_ingestion procontract_interest
```

5. Run skill lint if the skill prompt changed.

```bash
cargo test -p daemon --test skill_lint
```

6. Run required quality gates for substantive Rust changes.

```bash
cargo fmt --check
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
```

7. Replay the captured result without live portal mutation if you added `PROCONTRACT_INTEREST_REPLAY_RESULT_JSON`.

```bash
source state-e2e-machine-fork-20260512T195251Z/e2e-env.sh
PROCONTRACT_INTEREST_REPLAY_RESULT_JSON="$E2E_ARTIFACT_ROOT/sessions/2026/05/session-pty-5e1fb98f68771c12cb090dee2e0055d1e5f73d69345ab17faac4ed322981e8d9/workdir/result.json" \
  "$E2E_DAEMON" \
    --state-root "$E2E_STATE_ROOT" \
    --runtime-dir "$E2E_RUNTIME_DIR" \
    --artifact-root "$E2E_ARTIFACT_ROOT" \
    --ingest procontract-interest
```

Then validate:

```bash
sqlite3 -header -column "$E2E_STATE_ROOT/state.db" "
SELECT stage, state, COUNT(*) AS rows FROM process_stage_status WHERE stage='interest_registration' GROUP BY stage, state;
SELECT id, kind, status, scope_id, last_error FROM workflow_jobs WHERE scope_id='procontract-interest' ORDER BY created_at DESC LIMIT 5;
"
```

Expected with recommended pending semantics:

- no `unsupported ProContract registration action` error
- `interest_registration` rows include `completed`, `skipped`, and `pending`
- latest job is `completed` for a valid replay result
- invalid-action replay test leaves latest job `failed`, not `running`

8. Only after the replay and unit/integration tests are green, rerun the live Stage 4a command against the preserved E2E state.

Before doing this, account for the already-performed mutation:

```text
registered: BEST START FAMILY HUBS OUTREACH SERVICES
notice_id: pc-ecd07d9f-5249-f111-813c-005056b64545
registered_at: 2026-05-12T22:32:10Z
```

Live command:

```bash
source /Users/zacfarrell/Desktop/Testproject1/state-e2e-machine-fork-20260512T195251Z/e2e-env.sh
run_logged stage-04a-procontract-interest-retry-after-pending-action-fix \
  env AGENT_BROWSER_CDP_PORT=9222 "$E2E_DAEMON" \
    --state-root "$E2E_STATE_ROOT" \
    --runtime-dir "$E2E_RUNTIME_DIR" \
    --artifact-root "$E2E_ARTIFACT_ROOT" \
    --ingest procontract-interest
```

Post-live validation:

```bash
sqlite3 -header -column "$E2E_STATE_ROOT/state.db" "
SELECT stage, state, COUNT(*) FROM process_stage_status WHERE stage='interest_registration' GROUP BY stage, state;
SELECT id, kind, status, scope_id, last_error FROM workflow_jobs WHERE scope_id='procontract-interest' ORDER BY created_at DESC LIMIT 5;
"
```

If any step fails, read the exact error, fix the root cause, and restart the loop at the narrowest failing command. Do not skip ahead to live portal mutation.

## ACCEPTANCE CRITERIA

- The ProContract register-interest result contract has explicit semantics for unacted qualified rows.
- `pending_registration`, if retained, is an explicitly supported known action with retryable persistence semantics.
- Unknown registration actions still fail validation and roll back partial writes.
- A persistence error after a claimed `procontract-interest` job marks the job `failed` with a durable `last_error`; it never leaves the job `running`.
- Focused unit tests cover `pending_registration` and unknown-action rollback.
- A job-level regression proves unsupported actions fail the job durably.
- Skill lint passes after prompt/schema edits.
- Full Rust gates pass: `cargo fmt --check`, `cargo check --all-targets`, `cargo clippy --all-targets -- -D warnings`.
- Preserved replay or live retry no longer logs `unsupported ProContract registration action`.
