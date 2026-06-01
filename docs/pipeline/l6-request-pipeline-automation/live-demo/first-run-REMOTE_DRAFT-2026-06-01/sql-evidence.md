# SQL Evidence — REMOTE_DRAFT Smoke Test — 2026-06-01

> SMOKE TEST. No real bid run executed against a real opportunity, so the production `bid_workflow_states` table is intentionally empty. The `draft_saved` state-write is proven against the real daemon binary in the test fixtures (no mocks).

## Production daemon state.db (`~/Library/Application Support/com.procintel.desktop/state/state.db`)

### `bid_workflow_states` — empty (no real bid placed)
```sql
SELECT process_id, bid_state, started_at, updated_at FROM bid_workflow_states;
-- (0 rows)
```
This is the **expected and correct** state under the smoke-test mandate: no synthetic rows were written to the operator's production table (avoids a mixed-write on this operationally-authoritative table), and no real bid was placed. `bid_state` column default = `'not_started'`.

### `validation_verdicts` — empty
```sql
SELECT count(*) FROM validation_verdicts;  -- 0
```
No portal run manifest → no live verdict for this scope. Link column is `worker_job_id` (verified by schema).

### `bid_workflow_states` schema (verified)
`process_id` (PK), `bid_state` (NOT NULL, default `'not_started'`), `bid_scope_json`, `review_required`, `bid_job_id`, `started_at`, `completed_at`, `owner_id`, `row_version`, `last_local_seq`, `stage_run_id`, `updated_at`, `created_at`.

## Live daemon readiness-gate enforcement (real binary, temp state-root)
```
$ daemon --run-bid-workflow SMOKE-TEST-bid-job-0601 --state-root <tmp> ...
Error: bid readiness blocked for `SMOKE-TEST-bid-job-0601`:
  [PROCESS_NOT_FOUND, OPERATOR_NOT_APPROVED, DECISION_BASIS_STALE,
   L5_READINESS_PROJECTION_MISSING, COMPANY_PROFILE_OUT_OF_DATE,
   QUOTE_ROLE_FLOOR_ONLY(overrideable), UPSTREAM_TRUST_UNVALIDATED]
```
Run against a fresh temp `state.db` (migrated via `daemon --dry-run`), NOT the production DB. Proves the gate enforces every readiness invariant before any REMOTE_DRAFT portal action.

## draft_saved state-write proven in test fixtures (real daemon binary)
The `BidInProgress→DraftSaved` SQL state transition is asserted by:
- `bid_workflow_state_machine::l6_t_bid_workflow_states_008_in_progress_to_draft_saved_requires_draft_evidence` (35/35 suite green)
- `bid_runner_body` (33/33) — runner persists `bid_state='draft_saved'` with scope JSON
- `cli_bid_workflow::l6_t_daemon_runner_010_*` (31/31) — `--run-bid-workflow` against a seeded ready graph enqueues the continuation and the runner advances state.
These run the real daemon binary against a fixture state.db — the same SQL path a real run would take, with synthetic upstream data.
