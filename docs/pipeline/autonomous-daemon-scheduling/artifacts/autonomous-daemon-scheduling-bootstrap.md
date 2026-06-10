# Autonomous Daemon Scheduling & ProContract Resumption - Bootstrap

> Document Type: Test Bootstrap For Source of Truth #2
> Feature: Autonomous Daemon Scheduling & ProContract Resumption
> Created: 2026-06-07
> TDD: `docs/pipeline/autonomous-daemon-scheduling/artifacts/autonomous-daemon-scheduling-tdd.md`
> Spec: `docs/pipeline/autonomous-daemon-scheduling/artifacts/autonomous-daemon-scheduling-spec.md`

This document is the environment and implementation bootstrap an executing agent reads before writing the autonomous scheduling tests. It defines the required toolchain, harnesses, database setup, and commands.

## 1. Toolchain

| Surface | Requirement |
|---|---|
| Rust | Rust 2024 nightly per `rust-toolchain.toml`. |
| Rust package manager | Cargo only. Do not use npm, yarn, pnpm, or Bun for Rust code. |
| Unsafe policy | `#![forbid(unsafe_code)]` remains required in new Rust test files. |
| TypeScript | Bun only if Q7 selects desktop status surfacing. |
| Browser/PTY fake | The deterministic fake-claude CLI is allowed only as the agent/PTY stand-in. The daemon under test is real. |
| SQLite | Real SQLite opened through `open_state_db` in a tempdir. |

## 2. Required Stage Gate

Use the full gate every time a stage validation manifest claims green. `cargo test -p daemon --bin daemon` alone silently skips every integration file, currently about 61 files before this feature and about 64 after `scheduler_tick.rs`, `worker_pump.rs`, and `procontract_resumption.rs` land. Those three new suites only run in the per-file loop.

```bash
cargo fmt --check
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test -p daemon --bin daemon
for t in crates/daemon/tests/*.rs; do
  cargo test --test "$(basename "$t" .rs)" || exit 1
done
```

If Q7 selects desktop UI rendering, add the matching Bun build/test/lint commands for `apps/desktop` after the Rust gate.

## 3. Fast Local Iteration Commands

Run one suite at a time while developing tests:

```bash
cargo test --test scheduler_tick
cargo test --test worker_pump
cargo test --test procontract_resumption
```

Run targeted existing exemplars when copying idioms:

```bash
cargo test --test cron_scheduler
cargo test --test job_claim
cargo test --test resource_arbiter
cargo test --test agent_procurement_ingestion
cargo test --test sqlite_schema
```

Use debug builds for iteration. The full gate above is still required before any green claim.

## 4. Fake-Claude Harness

Copy the `RunHarness` shape from `crates/daemon/tests/agent_procurement_ingestion.rs`.

The harness creates:

- tempdir state root
- runtime root
- artifact root
- fixture file
- fake-claude program under the temp root

The fake CLI must support:

- `claude --version` and `claude -v`, returning a version string.
- `claude probe`, returning probe JSON with `auth_healthy`, `structured_output`, `pty_viable`, and `browser_tools`.
- contract mode detection through `--output-format`, copying a deterministic result to `DAEMON_TEST_CLAUDE_RESULT_PATH`.
- PTY mode checkpoint emission to `DAEMON_TEST_CLAUDE_CHECKPOINT_PATH`.

Required environment variables:

```text
DAEMON_TEST_CLAUDE_RESULT_PATH
DAEMON_TEST_CLAUDE_CHECKPOINT_PATH
```

Critical harness fact: `RunHarness.run()` uses `Command::cargo_bin("daemon")?...output()`. That call blocks until the child process exits. A live `--serve` daemon never exits, so M1 and M2 must invoke:

```bash
daemon --run-scheduler-once --tick-now <rfc3339>
```

The one-shot entrypoint evaluates schedules at the injected instant, enqueues due jobs, drains worker lanes to quiescence, and exits 0. A live serve harness may be implemented with `spawn()`, SQLite polling, timeout, and terminate only for tests that specifically exercise the interval loop. M1/M2 correctness does not depend on wall-clock waiting against `--serve`.

## 5. Database Setup

All tests use a real SQLite database in a tempdir. Do not open bespoke schema fragments unless the test is explicitly a migration-fragment test.

Use:

```rust
let db = TestStateDb::new()?;
let conn = db.connection();
```

`TestStateDb::new()` calls `open_state_db(&db_path, DEFAULT_SCHEMA_EPOCH)`, so the real schema, migrations, and PRAGMAs are applied.

Current constants and PRAGMAs:

- `DEFAULT_SCHEMA_EPOCH == 10` in `crates/daemon/src/db/sqlite.rs`.
- `journal_mode = WAL`.
- `synchronous = NORMAL`.
- `foreign_keys = ON`.
- `busy_timeout = 5000`.

If Q4 selects the dedicated `procontract_pending_enrichment` table:

- add `migrations/0015_procontract_pending_enrichment.sql`;
- bump `DEFAULT_SCHEMA_EPOCH` 10 to 11 in daemon;
- bump desktop epoch constants 10 to 11 in the same change;
- add `schema_0015_procontract_pending_enrichment` to `sqlite_schema.rs`.

The Q4 table test copies these idioms:

```rust
let db = TestStateDb::new()?;
let conn = db.connection();

let user_version: i64 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;
ensure!(user_version == 11, "expected PRAGMA user_version 11, got {user_version}");

let applied: Option<String> = conn
    .query_row(
        "SELECT id FROM schema_migrations WHERE id = '0015_procontract_pending_enrichment'",
        [],
        |row| row.get(0),
    )
    .optional()?;
ensure!(applied.as_deref() == Some("0015_procontract_pending_enrichment"));

let schema = logical_schema_for_tables(conn, &["procontract_pending_enrichment"])?;
let columns = schema
    .get("procontract_pending_enrichment")
    .context("missing procontract_pending_enrichment logical schema")?;
for required in [
    "process_id",
    "notice_id",
    "advert_id",
    "state",
    "interest_registered_at",
    "deadline_at",
    "observation_count",
    "last_checked_at",
    "enriched_at",
] {
    ensure!(columns.iter().any(|column| column.name == required), "missing column {required}");
}
```

If Q4 selects the state-string model, do not bump the epoch. Instead add `schema_state_string_refire_guard`, which asserts `PRAGMA user_version == DEFAULT_SCHEMA_EPOCH`, confirms `schema_migrations` does not contain `0015_procontract_pending_enrichment`, seeds an awaiting-access marker, and asserts `load_procontract_interest_candidates` does not pick the same notice again.

## 6. Helper Reuse Map

| Need | Copy from | Notes |
|---|---|---|
| Fixed instants | `cron_scheduler.rs` `ts()` | Use fixed UTC instants in all scheduler tests. |
| Real scheduler setup | `cron_scheduler.rs` | `CronScheduler::from_registry`, `refresh_next_run_at`, `dispatch_due_jobs`, `record_enqueued`. |
| Temp SQLite | `support/db.rs` | `TestStateDb::new()`, `seed_process`, `seed_raw_document`, `logical_schema_for_tables`. |
| Job seed/claim | `job_claim.rs` | `seed_job`, `JobRunner::claim_next_job`, lease and attempt assertions. |
| Concurrency | `job_claim.rs` test 001 | `Arc<Barrier>`, one `StateDb::open` per thread. |
| Resource admission | `resource_arbiter.rs` | `ResourceArbiter::new(ResourceBudgets::default())`, `admit`, `release`, concrete `DenialReason` matching. |
| E2E binary run | `agent_procurement_ingestion.rs` | `RunHarness`, fake-claude, post-run SQLite query idioms. |
| Schema inspection | `sqlite_schema.rs` and `support/db.rs` | `PRAGMA user_version`, `schema_migrations`, `logical_schema_for_tables`, column assertions. |

## 7. Scheduler Tick Test Skeleton

Create `crates/daemon/tests/scheduler_tick.rs` with `#![forbid(unsafe_code)]`.

```rust
#[test]
fn tick_001_due_slot_enqueues_exactly_one_row() -> Result<()> {
    let db = TestStateDb::new()?;
    seed_builtin_sources(&db)?;
    let registry = reload_registry(&db)?;
    let mut scheduler = CronScheduler::from_registry(&registry);
    scheduler.refresh_next_run_at(ts(2026, 6, 5, 4, 10, 0)?)?;

    let enqueued = run_scheduler_tick(
        &mut scheduler,
        &registry,
        &mut open_worker_state_db(db.db_path())?,
        ts(2026, 6, 5, 4, 15, 0)?,
    )?;

    ensure!(enqueued == 1);
    let count: i64 = db.connection().query_row(
        "SELECT COUNT(*) FROM workflow_jobs WHERE kind='sync_source' AND scheduled_for='2026-06-05T04:15:00+00:00'",
        [],
        |row| row.get(0),
    )?;
    ensure!(count == 1, "expected exactly one FTS sync_source job, got {count}");
    Ok(())
}
```

`tick_002` must call the tick body three times and assert the count remains 1 through pending/completed states. It must not assert failed jobs are suppressed. It also includes a source-static guard: any remaining startup enqueue helper must delegate to `run_scheduler_tick`, and no auxiliary cron dedup table, flag, or bootstrap-only guard may appear.

`tick_006_failed_slot_reenqueueable` seeds a `failed` row with the same idempotency key, ticks again, and asserts a second row exists. This pins the retry behavior verified against `find_existing_enqueue_ack`.

`tick_007_serve_scheduler_thread_spawn_poll_terminate` is the bounded live-serve assertion for FR-G1.1. It must use `Command::spawn()`, not `.output()`: start `daemon --serve` in a tempdir, poll real SQLite/status with a timeout until the scheduler thread's cold-start tick is observed, then terminate the process and wait for clean exit. M1/M2 still use `--run-scheduler-once --tick-now`.

`tick_004` includes both behavioral and source-static checks:

- before-slot injected `now` produces zero rows;
- due-slot injected `now` produces one row;
- the tick body has no inline `Utc::now()` call.

## 8. Worker Pump Test Skeleton

Create `crates/daemon/tests/worker_pump.rs` with `#![forbid(unsafe_code)]`.

### 8.1 Denial Test

```rust
#[test]
fn worker_002_defer_on_denial_no_attempt_bump() -> Result<()> {
    let db = TestStateDb::new()?;
    seed_sync_source_job(db.connection(), "job-denied-001", "fts")?;

    let mut arbiter = ResourceArbiter::new(ResourceBudgets::default());
    let first = arbiter.admit(&browser_request("held-1", "fts-a"));
    let second = arbiter.admit(&browser_request("held-2", "fts-b"));
    ensure!(matches!(first, ArbiterDecision::Granted(_)));
    ensure!(matches!(second, ArbiterDecision::Granted(_)));

    let before_attempts = attempt_count(db.connection(), "job-denied-001")?;
    let outcome = run_one_worker_iteration_with_arbiter(db.connection(), &mut arbiter)?;
    ensure!(matches!(
        outcome.denial_reason(),
        Some(DenialReason::BrowserSlotsExhausted { limit: 2, in_use: 2 })
    ));

    let row = job_state(db.connection(), "job-denied-001")?;
    ensure!(row.status == "pending");
    ensure!(row.attempt_count == before_attempts);
    ensure!(row.lease_owner.is_none());
    ensure!(row.lease_expires_at.is_none());

    let immediate = run_one_worker_iteration_with_arbiter(db.connection(), &mut arbiter)?;
    ensure!(!immediate.claimed_job_id_is("job-denied-001"));
    Ok(())
}
```

The final no-immediate-reclaim assertion is required. A denial is not a failed run and must not reuse retry attempt accounting.

### 8.2 Handler Side-Effect Assertions

`worker_001_routes_each_cron_kind` must implement one subcase per kind. The `sync_source` subcase is a route-key matrix covering FTS, Contracts Finder, ProContract discover, ProContract register-interest, and ProContract enrich by `(source_id, skill_id)`. Each subcase asserts both job result and durable side effect:

```text
sync_source -> route-key matrix plus procurement_processes and notice/raw-document rows
source_health -> source_runtime_state last_canary_at, last_canary_result, last_health_state/circuit_state, updated_at
auth_refresh -> auth_refresh_results row keyed by workflow_job_id, or exact source_runtime_state auth columns schema-tested by name
validation_cross_model_audit -> validation_incident_clusters row, plus validation_verdicts.worker_job_id when verdicts are written
validation_calibration_readiness_sweep -> enforcement_readiness row plus validation_calibration_cases row for the same slice
commercial_sweep -> landed_cost_estimates.stage_run_id plus candidate_net_margin_assessments.basis_hash plus manifests and pipeline_assessments.pipeline_state='manifest_ready'
```

Do not merge those subcases into a single status-only loop. The test must fail if any handler only writes `workflow_jobs.status='completed'`.

`worker_005_arbiter_quota_respected` must force all current `DenialReason` variants: `BrowserSlotsExhausted`, `AgentSlotsExhausted`, `SourceLaneExhausted`, and `SkillConcurrencyCapReached`. It then asserts every held cap is released after completion/failure.

### 8.3 Head-Of-Line Test

`worker_004_head_of_line` uses fake-claude sleep for the PTY lane:

```text
1. Enqueue slow `sync_source` with fake-claude sleep.
2. Enqueue ready `commercial_sweep`.
3. Start PTY/browser lane and coverage/deterministic lane.
4. Poll real SQLite.
5. Assert commercial completion timestamp is earlier than sync_source completion timestamp.
```

The test proves lane separation. It must not pass by making the PTY job fail before the commercial job can run.

## 9. ProContract Resumption Test Skeleton

Create `crates/daemon/tests/procontract_resumption.rs` with `#![forbid(unsafe_code)]`.

This file is net-new. Build it on `RunHarness` and fake-claude, not on an imaginary existing ProContract integration suite.

Ordered scenario:

```text
M3_step_001:
  run --ingest procontract-register-interest with canned success
  assert pending enrichment row state='awaiting_access'
  assert interest_registration stage remains completed
  assert rerun does not create a second interest expression

M3_step_002:
  run --ingest procontract-enrich with no live event for tracked advert
  include one untracked not_yet_responded item
  assert tracked row kept and observation_count increments
  assert untracked row backfilled

M3_step_003:
  run --ingest procontract-enrich with live event for tracked advert
  assert fake-claude evidence contains direct advertId visit
  assert docs persist
  assert row state='enriched' and enriched_at set

M3_step_004:
  seed old awaiting row
  run staleness sweep
  assert row state='expired', last_checked_at set, expiry_reason set
  assert no RFx response deadline drives expiry
```

Minimum SQL shape for each lifecycle checkpoint:

```sql
SELECT state, observation_count, last_checked_at, enriched_at
FROM procontract_pending_enrichment
WHERE process_id = ?1 AND advert_id = ?2;
```

If Q4 selects state-string, adapt the row checks to the selected storage, but keep all four lifecycle transitions and the re-fire guard assertion.

Contracts Finder is a special regression guard. Existing `agent_contracts_finder_ingestion.rs` tests `ag_cf_001` through `ag_cf_006` are skill-contract assertions and must pass unchanged, but they are not a daemon `--ingest contracts-finder` run. Add `impact_contracts_finder_live_loop_regression_001` in that file or a sibling daemon E2E suite using the `RunHarness` pattern: run `--ingest contracts-finder`, assert the latest parent job is `kind='sync_source'`, `status='completed'`, and assert Contracts Finder-produced process/notice/raw-document rows exist.

## 10. E2E Harness Commands

M1 command shape:

```bash
daemon \
  --run-scheduler-once \
  --tick-now 2026-06-05T04:15:00Z \
  --agent-cli claude \
  --agent-cli-program "$FAKE_CLAUDE" \
  --state-root "$STATE_ROOT" \
  --runtime-dir "$RUNTIME_ROOT" \
  --artifact-root "$ARTIFACT_ROOT" \
  --fixture-path "$FIXTURE_PATH"
```

M1 verification query:

```sql
SELECT kind, status
FROM workflow_jobs
WHERE parent_job_id IS NULL
ORDER BY created_at DESC
LIMIT 1;
```

Expected result:

```text
sync_source | completed
```

Then assert FTS-produced rows:

```sql
SELECT COUNT(*) FROM procurement_processes WHERE id='process-fts-001';
SELECT COUNT(*) FROM contract_notices WHERE process_id='process-fts-001';
SELECT COUNT(*) FROM raw_documents;
```

M1 negative control:

```bash
daemon --run-scheduler-once --tick-now 2026-06-05T03:00:00Z ...
```

Expected database result:

```sql
SELECT COUNT(*) FROM workflow_jobs WHERE kind='sync_source';
-- 0
```

M2 command shape:

```bash
daemon --run-scheduler-once --tick-now 2026-06-05T07:00:00Z ...
```

M2 verification:

```sql
SELECT COUNT(*) FROM landed_cost_estimates WHERE process_id = ?1;
SELECT COUNT(*) FROM candidate_net_margin_assessments WHERE process_id = ?1;
SELECT COUNT(*) FROM manifests WHERE process_id = ?1;
SELECT COUNT(*) FROM pipeline_assessments WHERE process_id = ?1;
```

Also assert the autonomous path did not call the desktop PID-stop path. In one-shot, assert no restart/exclusive-rerun marker, subprocess spawn, or `trigger_pipeline_rerun` call was observed. In a live serve harness, assert the daemon PID is unchanged until the harness terminates it.

M4 first proves the manual source paths:

```bash
daemon --ingest fts ...
daemon --ingest contracts-finder ...
```

Then M4 drains desktop `trigger_ingestion` with:

```bash
daemon --run-next-job --state-root "$STATE_ROOT" --runtime-dir "$RUNTIME_ROOT" --artifact-root "$ARTIFACT_ROOT" ...
```

The drain must use the same extended route table as the always-on worker.

## 11. Concurrency Soak Skeleton

M5 uses `job_claim.rs` test 001's `Arc<Barrier>` pattern.

```rust
let db_path = db.db_path().to_path_buf();
let barrier = Arc::new(Barrier::new(N));
for worker_index in 0..N {
    let db_path = db_path.clone();
    let barrier = Arc::clone(&barrier);
    handles.push(thread::spawn(move || -> Result<()> {
        let state_db = StateDb::open(&db_path)?;
        barrier.wait();
        for item_index in 0..M {
            insert_unique_counter_row(
                state_db.connection(),
                worker_index,
                item_index,
            )?;
        }
        Ok(())
    }));
}
```

After joining:

```rust
let count: i64 = conn.query_row("SELECT COUNT(*) FROM concurrency_counter", [], |row| row.get(0))?;
ensure!(count == N * M, "lost writes: expected {}, got {count}", N * M);
```

The same run also starts:

- a scheduler tick path;
- worker lanes;
- simulated IPC source mutation writes;
- lease heartbeat writes;
- a same-source overlap case.
- a commercial-sweep writer-pressure case with per-process-scoped transactions.

Assertions:

- no surfaced permanent `SQLITE_BUSY` error;
- final counter or unique-row count equals `N*M`;
- same-source FTS overlap returns `SourceLaneExhausted { source_id: "fts", limit: 1, in_use: 1 }`;
- the second same-source job runs after the first release;
- commercial-sweep writes do not hold one giant transaction that starves IPC mutations past `busy_timeout`.

## 12. Env Auto-Skip Discipline

Tests that depend on external environment state must probe at runtime and return `Ok(())` with a clear diagnostic when the dependency is unavailable. They must not use `#[ignore]`.

Examples:

- CDP-dependent skill canaries probe `http://localhost:9222/json/version`.
- Live portal smoke probes auth/CDP readiness before running.
- Tests that only need fake-claude should not probe CDP; they should use deterministic fixture paths.

The autonomous scheduling core tests are designed to avoid live external services by using fake-claude for agent/PTY result contracts and real daemon/SQLite for the runtime.

## 13. Completion Checklist

- `autonomous-daemon-scheduling-tdd.md` exists and maps every spec requirement.
- `scheduler_tick.rs` defines `tick_001` through `tick_007`.
- `worker_pump.rs` defines `worker_001` through `worker_005`.
- `procontract_resumption.rs` implements M3 as one ordered scenario.
- M1 and M2 use `--run-scheduler-once --tick-now`, not `--serve` with blocking `.output()`.
- M1 copies the `agent_procurement_ingestion.rs` latest-parent-job query and adds FTS-produced row assertions.
- `worker_001` contains one real side-effect row assertion per cron kind.
- `worker_001` proves `sync_source` routing by `(source_id, skill_id)`.
- `worker_002` copies resource arbiter denial idioms and asserts unchanged attempts plus cleared lease.
- `worker_005` forces every current `DenialReason` variant.
- Q4 table path copies `sqlite_schema.rs` schema-migration idioms.
- Q4 state-string path asserts no epoch bump and widened awaiting-access re-fire guard.
- The full stage gate in section 2 passes.
