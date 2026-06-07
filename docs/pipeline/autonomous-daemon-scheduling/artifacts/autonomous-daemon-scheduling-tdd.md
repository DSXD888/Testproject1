# Autonomous Daemon Scheduling & ProContract Resumption - TDD Specification

> Document Type: Source of Truth #2 (TDD)
> Feature: Autonomous Daemon Scheduling & ProContract Resumption
> Created: 2026-06-07
> Spec: `docs/pipeline/autonomous-daemon-scheduling/artifacts/autonomous-daemon-scheduling-spec.md`
> Bootstrap: `docs/pipeline/autonomous-daemon-scheduling/artifacts/autonomous-daemon-scheduling-bootstrap.md`

This document defines the tests that must exist before autonomous daemon scheduling can be declared implemented. The bootstrap document defines how those tests are implemented and run.

## 1. Scope

Primary inputs:

- `docs/pipeline/autonomous-daemon-scheduling/artifacts/autonomous-daemon-scheduling-spec.md`
- `docs/research/2026-06-04-autonomous-daemon-scheduling-research.md` section 7a through 7e
- `docs/pipeline/autonomous-daemon-scheduling/manifest.yaml`
- `docs/pipeline/autonomous-daemon-scheduling/artifacts/autonomous-daemon-scheduling-architecture.md`

Reference test quality bars:

- `docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-tdd.md`
- `docs/pipeline/goods-led-install-maintenance/artifacts/goods-led-install-maintenance-tdd.md`

Spec inventory:

- 38 requirement IDs.
- 33 functional/runtime requirement IDs: `FR-G1.1` through `FR-G6.4` plus `FR-RT.1` through `FR-RT.4`.
- 5 guardrail non-goal IDs: `NFR-1` through `NFR-5`.
- 6 committed goals, G1 through G6.
- 3 net-new Rust integration suites: `scheduler_tick.rs`, `worker_pump.rs`, and `procontract_resumption.rs`.
- 5 E2E manifests: M1 through M5.

## 2. Non-Negotiable Test Principles

1. Real harness only. The only permitted fake is the deterministic `fake-claude` CLI, and it stands in for the agent/PTY only.
2. Every test uses real SQLite in a tempdir, real cron parsing, and the real `daemon` binary subprocess where the flow requires a binary.
3. Every requirement ID in the spec has at least one stable test ID in section 10.
4. Empty handlers fail review. `status='completed'` alone is never sufficient for a cron-kind handler.
5. M1 and M2 drive `--run-scheduler-once --tick-now <rfc3339>` because `RunHarness.run()` uses blocking `Command::output()`.
6. Environment-dependent tests auto-skip via runtime probes. They must not use `#[ignore]`.
7. The full per-file integration loop is the only green Rust stage gate for this feature.

## 3. CI And Local Gate Inventory

The gate is intentionally written verbatim. A passing `cargo test -p daemon --bin daemon` alone silently skips every integration target, currently about 61 files before this feature and about 64 after the 3 net-new suites land. The new suites are only exercised by the loop.

```bash
cargo fmt --check
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test -p daemon --bin daemon
for t in crates/daemon/tests/*.rs; do
  cargo test --test "$(basename "$t" .rs)" || exit 1
done
```

If Q7 selects desktop UI rendering, also run the relevant `apps/desktop` Bun build/test/lint commands.

| Gate ID | Gate | Proves |
|---|---|---|
| `CI-ADS-001` | `cargo fmt --check` | Rust formatting is stable. |
| `CI-ADS-002` | `cargo check --all-targets` | Daemon code and all test targets compile. |
| `CI-ADS-003` | `cargo clippy --all-targets -- -D warnings` | Match arms, route tables, schema constants, and concurrency helpers satisfy lint policy. |
| `CI-ADS-004` | `cargo test -p daemon --bin daemon` | Daemon binary unit tests pass. |
| `CI-ADS-005` | `for t in crates/daemon/tests/*.rs; do cargo test --test "$(basename "$t" .rs)" || exit 1; done` | Every daemon integration target runs, including `scheduler_tick`, `worker_pump`, and `procontract_resumption`. |
| `CI-ADS-006` | Q7-selected Bun gate | Desktop status surfacing only, when selected by operator decision. |

## 4. Shared Test Infrastructure

| Harness or fixture | Owner surface | Requirements |
|---|---|---|
| `RunHarness` | `agent_procurement_ingestion.rs`; copied by M1, M2, M3, M4 | Uses tempdir state/runtime/artifact roots, `Command::cargo_bin("daemon")?...output()`, and deterministic fake-claude env vars. It is blocking, so live `--serve` is not used with `RunHarness.run()`. |
| `fake-claude` CLI | `agent_procurement_ingestion.rs` | Must answer `--version`/`-v`, `probe`, write `DAEMON_TEST_CLAUDE_RESULT_PATH`, and write `DAEMON_TEST_CLAUDE_CHECKPOINT_PATH`. |
| `TestStateDb` | `crates/daemon/tests/support/db.rs` | All new SQLite integration tests use `TestStateDb::new()` and the real `open_state_db` path. |
| `support/db.rs` seed helpers | `support/db.rs` | Reuse `seed_process`, `seed_raw_document`, and `logical_schema_for_tables` where those shapes apply. |
| `CronScheduler` helpers | `cron_scheduler.rs` | Copy the fixed `ts()` helper, `CronScheduler::from_registry`, `refresh_next_run_at`, `dispatch_due_jobs`, and `seed_source`/registry idioms. |
| `JobRunner` claim helpers | `job_claim.rs` | Copy `seed_job`, `JobRunner::claim_next_job`, and the `Arc<Barrier>` multi-thread pattern. |
| `ResourceArbiter` helpers | `resource_arbiter.rs` | Copy `admit`, `release`, `ArbiterDecision`, `DenialReason`, and resource event assertions. |
| `sqlite_schema.rs` schema helpers | `sqlite_schema.rs` and `support/db.rs` | Copy `PRAGMA user_version`, `schema_migrations` row checks, `table_exists`/column inspection, and `logical_schema_for_tables` idioms. |

## 5. Planned Test File Inventory

| File | Status | Primary IDs |
|---|---|---|
| `crates/daemon/tests/scheduler_tick.rs` | new | `tick_001` through `tick_006` |
| `crates/daemon/tests/worker_pump.rs` | new | `worker_001` through `worker_005` |
| `crates/daemon/tests/procontract_resumption.rs` | new | `M3_step_001` through `M3_step_004` plus skill-contract assertions |
| `crates/daemon/tests/cron_scheduler.rs` | extend | `impact_cron_scheduler_001`, `impact_cron_scheduler_006` |
| `crates/daemon/tests/job_claim.rs` | extend | `impact_job_claim_new_kinds_claimable`, existing ordering and lease assertions |
| `crates/daemon/tests/agent_procurement_ingestion.rs` | preserve | `impact_ingestion_regression_001`, M1 query idiom |
| `crates/daemon/tests/agent_contracts_finder_ingestion.rs` | preserve | `impact_contracts_finder_regression_001` |
| `crates/daemon/tests/sqlite_schema.rs` | conditional extension | `schema_0015_procontract_pending_enrichment` or `schema_state_string_refire_guard` |
| `skills/procurement-ingestion/procontract-enrich-tender/` canary/lint tests | extend | `skill_contract_001_awaiting_access_targets` |
| desktop/IPC tests modeled on `ipc_trigger_bid_workflow.rs` | conditional extension | `M4_trigger_ingestion_executes`, Q7/E1 selected assertions |

## 6. Existing-Test Impact Assertions

| Test ID | File | Exact required change |
|---|---|---|
| `impact_cron_scheduler_001_anchor_set_includes_0700_commercial_sweep` | `cron_scheduler.rs` test 001 | The anchor set gains one new definition key `commercial-sweep::pipeline/commercial-sweep`; the definition count moves from 13 to 14; with `refresh_next_run_at(ts(2026,3,25,4,10,0))`, the new key has `next_run_at == ts(2026,3,25,7,0,0)`. Existing source schedule anchors stay unchanged. |
| `impact_cron_scheduler_006_catchup_order_includes_commercial_sweep_key` | `cron_scheduler.rs` test 006 | The catch-up priority-order scenario must include the new key `commercial-sweep::pipeline/commercial-sweep`. If the test tick remains before 07:00, move the tick to after the 07:00 slot so the key is due, then assert the complete ordered key list including the commercial key rather than a vague definition-count update. |
| `impact_job_claim_new_kinds_claimable` | `job_claim.rs` | Add claimability coverage for `sync_source`, `source_health`, `auth_refresh`, `validation_cross_model_audit`, `validation_calibration_readiness_sweep`, and `commercial_sweep`. Do not mutate the existing stage-job ordering tests. |
| `impact_ingestion_regression_001_fts_unchanged` | `agent_procurement_ingestion.rs` | Existing test 001 must pass unchanged after the live-loop body refactor. Its post-run query is the idiom M1 must copy: `SELECT kind, status FROM workflow_jobs WHERE parent_job_id IS NULL ORDER BY created_at DESC LIMIT 1` returns `sync_source`, `completed`. |
| `impact_contracts_finder_regression_001` | `agent_contracts_finder_ingestion.rs` | Existing Contracts Finder ingestion regression must pass unchanged after the shared body refactor. |
| `impact_sqlite_schema_q4_table` | `sqlite_schema.rs` | Only when Q4 selects the dedicated table, assert epoch 11, migration `0015_procontract_pending_enrichment`, and table/column shape. |

## 7. Detailed Test Catalog

### 7.1 Scheduler Tick

All scheduler tick tests live in new file `crates/daemon/tests/scheduler_tick.rs` and copy the `cron_scheduler.rs` idioms: fixed `ts()`, `TestStateDb::new()`, seeded registry/source rows, `CronScheduler::from_registry`, `refresh_next_run_at`, `dispatch_due_jobs`, and real `JobRunner::enqueue_job`.

| Test ID | Tier | Test |
|---|---|---|
| `tick_001_due_slot_enqueues_exactly_one_row` | T2 | Seed enabled FTS; build registry/scheduler; call `run_scheduler_tick(..., ts(2026,6,5,4,15,0))`; assert exactly one parent `workflow_jobs` row for `kind='sync_source'`, `scheduled_for='2026-06-05T04:15:00+00:00'`, source `fts`, skill `procurement-ingestion/fts`, and idempotency key `cron:fts:procurement-ingestion/fts:2026-06-05T04:15:00+00:00`. |
| `tick_002_repeated_ticks_same_slot_idempotent` | T2 | Call the tick body 3 times inside the same due slot. Assert `COUNT(*) FROM workflow_jobs WHERE kind='sync_source'` remains 1 through the pending/completed path. This pins the existing `find_existing_enqueue_ack` status filter: `pending`, `running`, `retry`, `completed` are suppressing states; `failed` is not. |
| `tick_003_long_gap_catchup` | T2 | First tick at the normal FTS slot, then advance fixed `now` several days and tick again. Assert overdue slot enqueue follows `schedule.after(last_run_at)` and `record_enqueued` advances in-memory state. Model the ordered due-key check on `cron_scheduler.rs` test 006. |
| `tick_004_clock_seam` | T2 | Prove the public tick body accepts injected `now`. A tick at `2026-06-05T03:00:00Z` enqueues nothing; the same setup at `2026-06-05T04:15:00Z` enqueues the FTS row. Add a static source assertion that the tick body contains no inline `Utc::now()` call. |
| `tick_005_registry_reload_on_generation_bump` | T2 | Seed source disabled; tick enqueues nothing. Mutate source enablement through the same path as IPC source mutation and bump the selected Q9 generation/timestamp trigger. Next tick reloads registry, re-seeds last-run history, and enqueues exactly the due row without a burst. |
| `tick_006_failed_slot_reenqueueable` | T2 | Seed a failed job with the same cron idempotency key. Tick the same slot again and assert a new row is inserted. This is a positive retry-behavior test, not part of `tick_002`. |

### 7.2 Worker Pump

All worker tests live in new file `crates/daemon/tests/worker_pump.rs` and copy `job_claim.rs` plus `resource_arbiter.rs`. They run one worker iteration or a bounded lane pump against real SQLite, never a fake daemon handler.

| Test ID | Tier | Test |
|---|---|---|
| `worker_001_routes_each_cron_kind` | T2 | For each cron kind, seed a pending parent job, run the real worker route once, assert it does not hit unsupported-kind failure, and assert a real side-effect row listed in section 7.2.1. |
| `worker_002_defer_on_denial_no_attempt_bump` | T2 | Saturate `browser_slots=2` by admitting two browser-backed leases. Seed a third `sync_source`. One worker iteration receives `DenialReason::BrowserSlotsExhausted { limit: 2, in_use: 2 }`, writes the job back to `status='pending'`, leaves `attempt_count` unchanged, clears `lease_owner`, clears `lease_expires_at`, releases no resource lease, and the immediately-next iteration does not re-claim the same job because denial backoff/cooldown is active. |
| `worker_003_source_lane_serializes_same_source` | T2 | Admit one FTS `sync_source` so `procurement_source_lanes=1` is consumed. Seed a second FTS `sync_source`. Assert admission returns `DenialReason::SourceLaneExhausted { source_id: "fts", limit: 1, in_use: 1 }`, the second job stays pending with no attempt bump, no second PTY run starts, and it runs only after the first lease is released. |
| `worker_004_head_of_line` | T2/E2E | Enqueue a slow PTY `sync_source` whose fake-claude path sleeps and a ready `commercial_sweep`. Run both lanes. Assert `commercial_sweep` completes before the slow `sync_source`, proving the PTY lane cannot starve the coverage/deterministic lane. |
| `worker_005_arbiter_quota_respected` | T2 | Start enough PTY/browser jobs to pressure the arbiter. Assert the maximum concurrent browser-backed admissions observed by events/snapshot is exactly 2, never 3, and source-lane and skill-concurrency caps are released after completion/failure. |

#### 7.2.1 `worker_001` Anti-Hollow Side-Effect Matrix

| Cron kind | Required side-effect assertion |
|---|---|
| `sync_source` | After real FTS execution via fake-claude, assert a source-produced row exists, for example `procurement_processes.id='process-fts-001'` and at least one associated `contract_notices` or `raw_documents` row from the fixture. Also assert the parent job is `completed`. |
| `source_health` | After the `source-health/fts` canary handler runs, assert `source_runtime_state` for `source_id='fts'` has a fresh `last_canary_at`, `last_canary_result` containing the six-check result contract, and the expected health state. |
| `auth_refresh` | After the `auth-signin/procontract` or chosen auth-refresh handler runs, assert a durable auth-state row is written for the source/portal. The row must capture source or portal ID, `signin_status` or refresh status, `cdp_state`, result timestamp, and the workflow job ID. A job status flip without this auth-state row fails the test. |
| `validation_cross_model_audit` | Assert a validation evidence row is written, using the existing validation tables where possible: `validation_verdicts` for the worker job and/or `validation_incident_clusters` summary evidence for the audit. |
| `validation_calibration_readiness_sweep` | Assert readiness sweep evidence in `validation_calibration_cases`, `enforcement_readiness`, or the chosen calibration-readiness summary table. It must include slice/corpus identity, readiness verdict, and the worker job ID or run ID. |
| `commercial_sweep` | Seed eligible rows and assert all three commercial side effects: at least one current `landed_cost_estimates` row, at least one `candidate_net_margin_assessments` row from `run_net_margin_gate_once`, and one current `manifests` plus `pipeline_assessments.current_stage='manifest_ready'` or equivalent stage row. |

### 7.3 ProContract Resumption

`crates/daemon/tests/procontract_resumption.rs` is a net-new suite. There is no existing ProContract integration test to extend. It is built from scratch on the `RunHarness` plus fake-claude pattern.

| Test ID | Tier | Test |
|---|---|---|
| `M3_step_001_register_creates_awaiting_access_without_refire` | T2/E2E | Run `--ingest procontract-register-interest` with canned success. Assert `procontract_pending_enrichment` row `state='awaiting_access'` exists, `observation_count=0`, and `process_stage_status` for `stage='interest_registration'` remains `state='completed'` or an existing terminal success state. Re-running register-interest must not re-express interest for the same notice. |
| `M3_step_002_enrich_keep_and_backfill` | T2/E2E | Run `--ingest procontract-enrich` with canned no-live-event result for the tracked advert plus an untracked portal item observed as `not_yet_responded`. Assert tracked row stays `awaiting_access`, `observation_count` increments, `last_checked_at` is set, and a second row is backfilled. |
| `M3_step_003_enrich_clear_via_worklist` | T2/E2E | Run enrich again with canned live tender event. Assert the daemon injects `awaitingAccessTargets`, fake-claude evidence proves direct visit by `advertId` rather than Recently-added discovery, documents persist, and the tracked row becomes `state='enriched'` with `enriched_at` set. |
| `M3_step_004_staleness_expire` | T2 | Seed an old awaiting-access row using the Q5 policy. Run the sweep and assert `state='expired'`, `last_checked_at` is set, `expiry_reason` is `notice_deadline` or `max_age`, expired rows are excluded from future `awaitingAccessTargets`, and no RFx response deadline is used. |
| `skill_contract_001_awaiting_access_targets` | T2 | Extend `procontract-enrich-tender` schemas/canaries so optional `awaitingAccessTargets: [{ advertId, sourceNoticeId, processId }]` is accepted, normal scan results and direct-target results are unioned/deduped by `advertId`, empty work-list behavior matches the current scan-only path, and CDP discipline remains mandatory. |

### 7.4 E2E Manifests

| Manifest | Tier | Scenario and assertions |
|---|---|---|
| `M1_end_to_end_autonomy_g1_g2` | T3 | Using `RunHarness`, invoke `daemon --run-scheduler-once --tick-now 2026-06-05T04:15:00Z` with FTS enabled and fake-claude wired. Do not call `--ingest` or IPC. Verify with the exact post-run SQLite idiom: `SELECT kind,status FROM workflow_jobs WHERE kind='sync_source' AND parent_job_id IS NULL ORDER BY created_at DESC LIMIT 1` returns `('sync_source','completed')`. Also assert FTS-produced rows exist in `procurement_processes` and associated notice/raw-document tables. Negative control: `--tick-now 2026-06-05T03:00:00Z` enqueues and persists no FTS run. This manifest confirms `RunHarness.run()` uses `Command::output()` and therefore M1 cannot point at `--serve`. |
| `M2_commercial_stage_autonomy_g6` | T3 | Seed or reuse a process needing pricing. Invoke `daemon --run-scheduler-once --tick-now 2026-06-05T07:00:00Z`. Assert `commercial_sweep` fires and drains in-process, current landed-cost/net-margin/manifest rows exist, the daemon does not stop/restart, and no `trigger_pipeline_rerun` or exclusive desktop PID-stop path is used. |
| `M3_procontract_resumption_full_lifecycle_g4` | T3 | Execute `M3_step_001` through `M3_step_004` as one ordered scenario, using real SQLite and fake-claude for agent/PTY result contracts. |
| `M4_manual_path_regression_and_trigger_ingestion` | T3 | Assert `--ingest fts`, `--run-next-job`, and manual `trigger_pipeline_rerun` still work. Enqueue through the desktop `trigger_ingestion` IPC/button path modeled on `ipc_trigger_bid_workflow.rs`, then drain with `--run-next-job`; assert the `sync_source` job reaches `completed`, not `pending` orphan. If E1 selects `enrich_process` routing, assert `trigger_enrichment` completes through the same route table. |
| `M5_concurrency_soak_g5` | T3 | Run worker lanes, simulated IPC mutations, lease heartbeat, and same-source overlap under WAL with each thread opening its own `StateDb::open`. Use `Arc<Barrier>`. Prove no lost writes concretely: N threads each perform M unique inserts or increments and final `COUNT(*)` or counter value equals `N*M`. Assert no surfaced permanent `SQLITE_BUSY` after `busy_timeout` handling. Assert R7: a second same-source `sync_source` is denied with `SourceLaneExhausted`, does not run concurrently, then runs after release. |

### 7.5 Conditional And Guardrail Tests

| Test ID | Tier | Test |
|---|---|---|
| `decision_001_operator_decisions_recorded` | T1/T2 | Before dependent implementation, assert the status artifact records verbatim answers for Q1, Q4, Q5, Q6, Q7, Q8-residual, Q9, and E1. This is a blocking precondition for decision-dependent tests. |
| `schema_0015_procontract_pending_enrichment` | T2 | If Q4 selects the dedicated table, copy `sqlite_schema.rs` idioms: `PRAGMA user_version == 11`, `schema_migrations` contains `0015_procontract_pending_enrichment`, `logical_schema_for_tables` sees `process_id`, `notice_id`, `advert_id`, `state`, `interest_registered_at`, `deadline_at`, `observation_count`, `last_checked_at`, and `enriched_at`, and daemon plus desktop epoch constants move 10 to 11 in one change. |
| `schema_state_string_refire_guard` | T2 | If Q4 selects the state-string model, assert no epoch bump, and assert register-interest candidate selection excludes a notice marked awaiting access so the same item is not picked again. |
| `shadow_health_auth_001` | T2 | If Q6 selects shadow mode, assert `source_health` and `auth_refresh` still claim, execute, write side-effect rows, complete, and surface shadow outcome fields, while validation jobs and `commercial_sweep` are not gated by the flag. |
| `desktop_status_001` | T2/T3 | If Q7 selects status surfacing, assert daemon `CronStatusSnapshot` includes scheduler running, last tick, next run, worker lane health, and awaiting-access counts; desktop Rust passes the fields read-only; React renders them read-only. |
| `commercial_nudge_001` | T2/T3 | If Q8-residual selects the chained nudge, assert successful ingestion enqueues a deduped `commercial_sweep` using the same idempotency family and never runs commercial stage execution inside the source scraper. |
| `trigger_enrichment_001` | T2/T3 | If E1 selects `enrich_process` routing, assert the worker has a real handler and desktop `trigger_enrichment` completes through `--run-next-job`; otherwise assert the extension point remains named and `trigger_ingestion` still ships. |
| `guard_scope_001_skill_selector_boundary` | T1/T2 | Assert this feature changes only the ProContract enrich input/work-list contract for scraping behavior. Procurement and supplier scraping selectors are otherwise untouched. |
| `guard_scope_002_skill_first_boundary` | T1/T2 | Assert no replacement API connector path is introduced for procurement or supplier scraping. Agent/PTY skill execution remains the data retrieval path. |
| `guard_scope_003_single_daemon_boundary` | T1/T2 | Assert no multi-daemon or multi-org scheduler semantics are introduced. |
| `guard_scope_004_no_remote_http_trigger` | T1/T2 | Assert no new remote HTTP trigger surface is added. |
| `guard_scope_005_no_compat_shims` | T1/T2 | Assert the live-loop refactor edits direct bodies/handlers and does not add compatibility shim wrappers around deprecated APIs. |

## 8. Per-Goal Verification Matrix

| Goal | Correct proof | Regression shape | Gate |
|---|---|---|---|
| G1 | M1 plus `tick_001`, `tick_003`, `tick_004`, and job-claim overdue-first ordering. | Due slot never enqueues without restart; no scheduler thread; inline wall-clock-only path; overdue scheduled work waits behind newer manual work. | Full Rust stage gate with per-file integration loop. |
| G2 | `worker_001` proves every cron kind has a real side-effect row and never hits unsupported-kind failure. | Any cron kind reaches `other => fail_job`; any handler only flips status. | Full Rust stage gate. |
| G3 | M4 proves `--ingest`, `--run-next-job`, manual `trigger_pipeline_rerun`, and live `trigger_ingestion` behavior. | Desktop ingestion job remains pending; manual CLI paths break after refactor. | Rust integration loop; desktop gates if Q7/E1 touches desktop. |
| G4 | M3 steps 1 through 4 plus Q4 schema branch. | Enrich misses aged awaiting-access item; register-interest fires again; expiry uses unknown RFx response deadline. | Full Rust integration loop plus skill lint/schema checks. |
| G5 | M5 plus `worker_003` prove no lost writes, no permanent `SQLITE_BUSY`, and no same-source overlap. | Lost counter rows, surfaced busy errors, or concurrent FTS scrape. | Full Rust integration loop. |
| G6 | M2 proves current manifest/decision from `commercial_sweep` with no daemon stop/restart. | Manifest appears only after manual rerun; sweep invokes desktop PID-stop path. | Full Rust integration loop. |

## 9. Follow-Up Audit Findings Incorporated

| Check | Finding | TDD consequence |
|---|---|---|
| `agent_procurement_ingestion.rs` test 001 | Post-run assertion queries the latest parent job and expects `kind='sync_source'`, `status='completed'`. | M1 uses `SELECT kind,status FROM workflow_jobs WHERE kind='sync_source' AND parent_job_id IS NULL ORDER BY created_at DESC LIMIT 1` and adds FTS-produced row assertions. |
| `RunHarness.run()` | Uses `Command::cargo_bin("daemon")?...output()`, which blocks until child exit. | M1/M2 use `--run-scheduler-once --tick-now`; live `--serve` is only a spawn/poll/terminate harness shape when a test specifically targets the interval loop. |
| `cron_scheduler.rs` tests 001 and 006 | Adding the 07:00 `commercial_sweep` slot affects exact anchors and catch-up ordering. | Section 6 names the exact new key, 07:00 anchor, count change, and catch-up ordered-key update. |
| `worker_001` | Status-only completion is hollow. | Section 7.2.1 lists a side-effect row assertion for each of the six cron kinds. |
| `resource_arbiter.rs` | Existing tests assert `admit`, `release`, and concrete `DenialReason` variants. | `worker_002` and `worker_003` copy those idioms and assert no attempt bump, cleared lease, and no immediate re-claim. |
| `sqlite_schema.rs` | Existing tests use `PRAGMA user_version`, `schema_migrations`, and table/column helpers. | `schema_0015_procontract_pending_enrichment` copies those idioms for Q4 table path. |

## 10. Spec To Test Coverage Matrix

| Spec requirement | Test ID(s) | Type | Status |
|---|---|---|---|
| `FR-G1.1` | `tick_001`, `tick_003`, M1, M5 | T2/T3 | Covered |
| `FR-G1.2` | `tick_004`, M1, M2 | T2/T3 | Covered |
| `FR-G1.3` | `tick_002`, `tick_006`, `impact_cron_scheduler_001` | T2 | Covered |
| `FR-G1.4` | `tick_005`, `decision_001_operator_decisions_recorded` | T2 | Covered |
| `FR-G1.5` | `tick_003`, M1, M5 | T2/T3 | Covered |
| `FR-G1.6` | existing `job_claim.rs` overdue-first ordering, `tick_003`, M1 | T2/T3 | Covered |
| `FR-G2.1` | `worker_004`, `worker_005`, M1, M2 | T2/T3 | Covered |
| `FR-G2.2` | `worker_002`, `worker_003`, `worker_005` | T2 | Covered |
| `FR-G2.3` | `worker_001` | T2 | Covered |
| `FR-G2.4` | `decision_001_operator_decisions_recorded`, `worker_001`, `shadow_health_auth_001` | T2 | Covered |
| `FR-RT.1` | `impact_ingestion_regression_001`, `impact_contracts_finder_regression_001`, M4, M3 | T2/T3 | Covered |
| `FR-RT.2` | `worker_001`, M3, M4 | T2/T3 | Covered |
| `FR-RT.3` | `worker_001`, M3 | T2/T3 | Covered |
| `FR-RT.4` | `decision_001_operator_decisions_recorded`, `worker_002`, M5 | T2/T3 | Covered |
| `FR-G3.1` | M4 | T3 | Covered |
| `FR-G3.2` | M4 | T3 | Covered |
| `FR-G3.3` | M2, M4, M5 | T3 | Covered |
| `FR-G3.4` | `decision_001_operator_decisions_recorded`, `desktop_status_001` | T2/T3 | Covered |
| `FR-G3.5` | `decision_001_operator_decisions_recorded`, M4, `trigger_enrichment_001` | T2/T3 | Covered |
| `FR-G4.1` | `M3_step_001`, `schema_0015_procontract_pending_enrichment`, `schema_state_string_refire_guard` | T2/T3 | Covered |
| `FR-G4.2` | `M3_step_001`, `M3_step_002`, `M3_step_003`, `M3_step_004` | T2/T3 | Covered |
| `FR-G4.3` | `M3_step_002`, `M3_step_003` | T2/T3 | Covered |
| `FR-G4.4` | `skill_contract_001_awaiting_access_targets`, M3 | T2/T3 | Covered |
| `FR-G4.5` | `M3_step_004`, `desktop_status_001` | T2/T3 | Covered |
| `FR-G5.1` | M5 | T3 | Covered |
| `FR-G5.2` | M5 | T3 | Covered |
| `FR-G5.3` | `worker_003`, M5 | T2/T3 | Covered |
| `FR-G5.4` | existing job-claim lease tests, M5 | T2/T3 | Covered |
| `FR-G5.5` | `tick_002`, `tick_003`, `worker_002`, `worker_003`, `worker_005`, M5, Q4 schema branch | T2/T3 | Covered |
| `FR-G6.1` | `impact_cron_scheduler_001`, `impact_cron_scheduler_006`, `tick_001`, `tick_002`, M2 | T2/T3 | Covered |
| `FR-G6.2` | `worker_001`, M2 | T2/T3 | Covered |
| `FR-G6.3` | M2, M4 | T3 | Covered |
| `FR-G6.4` | `decision_001_operator_decisions_recorded`, M2, `commercial_nudge_001` | T2/T3 | Covered |
| `NFR-1` | `guard_scope_001_skill_selector_boundary`, M3 | T1/T2 | Covered |
| `NFR-2` | `guard_scope_002_skill_first_boundary`, M1, M2, M3, M4, M5 | T1/T3 | Covered |
| `NFR-3` | `guard_scope_003_single_daemon_boundary`, M5 | T1/T3 | Covered |
| `NFR-4` | `guard_scope_004_no_remote_http_trigger`, M4 | T1/T3 | Covered |
| `NFR-5` | `guard_scope_005_no_compat_shims`, `impact_ingestion_regression_001`, M4 | T1/T3 | Covered |

## 11. Research Section 7 Preservation Matrix

| Research item | Preserved as |
|---|---|
| 7a `cron_scheduler.rs` impact | `impact_cron_scheduler_001`, `impact_cron_scheduler_006` |
| 7a `job_claim.rs` impact | `impact_job_claim_new_kinds_claimable` |
| 7a ingestion regression impact | `impact_ingestion_regression_001`, `impact_contracts_finder_regression_001` |
| 7a `resource_arbiter.rs` exemplar | `worker_002`, `worker_003`, `worker_005` |
| 7a `sqlite_schema.rs` conditional impact | `schema_0015_procontract_pending_enrichment` |
| 7b scheduler tests | `tick_001`, `tick_002`, `tick_003`, `tick_004`, `tick_005`, plus `tick_006` |
| 7b worker tests | `worker_001`, `worker_002`, `worker_003`, `worker_004`, `worker_005` |
| 7b live-loop regression | `impact_ingestion_regression_001`, M4 |
| 7c M1 | `M1_end_to_end_autonomy_g1_g2` |
| 7c M2 | `M2_commercial_stage_autonomy_g6` |
| 7c M3 | `M3_procontract_resumption_full_lifecycle_g4` |
| 7c M4 | `M4_manual_path_regression_and_trigger_ingestion` |
| 7c M5 | `M5_concurrency_soak_g5` |
| 7d goal verification | Section 8 |
| 7e Q4 table branch | `schema_0015_procontract_pending_enrichment` |
| 7e Q4 state-string branch | `schema_state_string_refire_guard` |

## 12. Completion Audit Checklist

- Every requirement ID in the spec appears in section 10 with at least one test ID.
- Every research section 7 named test appears in section 11.
- M1 uses the `agent_procurement_ingestion.rs` post-run SQLite idiom and adds FTS-produced row assertions.
- M1/M2 use `--run-scheduler-once --tick-now`, with live `--serve` restricted to spawn/poll/terminate harnesses.
- `worker_001` has side-effect row assertions for all six cron kinds.
- `worker_002` asserts unchanged `attempt_count`, cleared lease, and no immediate re-claim.
- Q4 table path copies `sqlite_schema.rs` schema idioms.
- The full per-file integration loop is present in section 3.
