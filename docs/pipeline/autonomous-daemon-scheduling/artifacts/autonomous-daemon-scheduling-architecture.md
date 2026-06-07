# Autonomous Daemon Scheduling & ProContract Resumption - Architecture

> **Document Type:** Supplementary Architecture
> **Primary Source Of Truth:** `docs/pipeline/autonomous-daemon-scheduling/artifacts/autonomous-daemon-scheduling-spec.md`
> **Created:** 2026-06-07

This document explains how to implement the specification in the existing codebase. If this document conflicts with the specification, the specification wins.

## 1. System Overview

The daemon already contains schedules, a queue, skill-backed live loops, a resource arbiter, WAL-enabled local SQLite, and a blocking serve loop. The autonomy change adds the missing runtime machinery:

1. A recurring scheduler tick that keeps asking "what is due now?"
2. Two continuous worker lanes that drain queue jobs by kind class.
3. A shared source execution body so cron `sync_source` jobs and manual `--ingest` jobs execute the same scraper logic.
4. A durable ProContract awaiting-access queue injected into enrich.
5. A scheduled, coverage-driven `commercial_sweep` for `landed-cost` -> `net-gate` -> `manifest`.

```text
daemon --serve
  |
  +-- serve / IPC loop
  |     owns RuntimeBootstrap.state_db
  |     handles desktop commands and snapshots
  |
  +-- SyncLeaseHeartbeat
  |     owns StateDb connection A
  |     refreshes writer lease
  |
  +-- SchedulerTickThread
  |     owns StateDb connection B
  |     SourceRegistry + CronScheduler in memory
  |     dispatch_due_jobs -> JobRunner::enqueue_job
  |
  +-- WorkerLane: PTY/browser
  |     owns StateDb connection C, pending Q1
  |     handles sync_source, source_health, auth_refresh
  |     uses ResourceArbiter for browser/agent/source lanes
  |
  +-- WorkerLane: coverage/deterministic
        owns StateDb connection D, pending Q1
        handles validation_* and commercial_sweep
        uses ResourceArbiter for agent/skill-backed sub-passes
```

The recommended Q1 default is own `StateDb` connection per worker lane. The shared-services alternative is documented in section 8 but should not be implemented unless the operator records that answer.

## 2. Thread Model

### 2.1 Steady-State Threads And Connections

| Thread | Existing/New | DB Handle | Writes | Notes |
|---|---|---|---|---|
| Serve / IPC loop | Existing | `RuntimeBootstrap.state_db` | IPC job enqueue, source mutations, status queries with possible writes elsewhere | Blocking accept loop in `serve_handshake_service`; never shares this handle across threads. |
| Lease heartbeat | Existing | Own `StateDb` connection | Writer lease heartbeat | `SyncLeaseHeartbeat::spawn` precedent. |
| Scheduler tick | New | Own `StateDb` connection | Enqueue due jobs; record status telemetry | Own in-memory scheduler; reloads registry only on Q9 trigger. |
| PTY/browser lane | New | Own connection recommended | Claims jobs, updates leases, persists skill/live-loop results | Runs browser-backed jobs under arbiter. |
| Coverage/deterministic lane | New | Own connection recommended | Claims jobs, persists validation and commercial sweep results | Does not consume procurement-source lane, but skill-backed sub-passes still consume agent/skill slots. |

### 2.2 Spawn Order

```text
bootstrap_runtime(cli)
  open main StateDb
  reconcile interrupted sessions
  ensure local executor registered
  write descriptor
  return RuntimeBootstrap

initialize_runtime_services(runtime)
  spawn SyncLeaseHeartbeat (existing)
  spawn SchedulerTickThread (new)
  spawn WorkerLane::pty_browser (new)
  spawn WorkerLane::coverage_deterministic (new)

serve_handshake_service(runtime)
  blocking IPC loop remains focused on desktop commands
```

`enqueue_startup_cron_jobs` is folded into `SchedulerTickThread`: the thread runs one tick immediately using `Utc::now()` from the loop and then waits for the interval.

### 2.3 Shutdown

Each new background service mirrors heartbeat ownership:

```rust
struct SchedulerTickHandle {
    stop_tx: Option<mpsc::Sender<()>>,
    join_handle: Option<JoinHandle<()>>,
}

struct WorkerLaneHandle {
    stop_tx: Option<mpsc::Sender<()>>,
    join_handle: Option<JoinHandle<()>>,
}
```

Dropping the handle sends stop and joins. A restart request from the serve loop returns from `serve_handshake_service`, then runtime drop stops the services.

## 3. WAL Single-Writer Story

The current SQLite foundation is:

- `journal_mode = WAL`
- `synchronous = NORMAL`
- `foreign_keys = ON`
- `busy_timeout = 5000`
- `StateDb::with_write_transaction` uses an in-process write mutex before `TransactionBehavior::Immediate`

This feature may create up to four writer classes:

```text
serve/IPC writer
lease heartbeat writer
scheduler enqueue writer
worker lane writer(s)
```

Correctness rules:

1. Background threads open independent connections; they do not share the serve-loop connection.
2. Write transactions stay short.
3. Job claim uses existing immediate transaction semantics.
4. Commercial sweep writes are per-process scoped, never one giant transaction around the entire sweep.
5. A skill-backed sub-pass may run outside a write transaction; only persistence is wrapped.
6. IPC mutations are allowed to wait under `busy_timeout`, but M5 must prove no permanent `SQLITE_BUSY` and no lost writes.
7. If M5 exposes writer starvation or lost writes, the escalation path is a single-writer actor that serializes write intents while preserving the same logical handlers.

## 4. Data Flow

### 4.1 Scheduled Source Flow

```text
Cron definition
  |
  v
SchedulerTickThread
  run_scheduler_tick(scheduler, registry, db, now)
  |
  v
CronScheduler::dispatch_due_jobs(&registry, now)
  |
  v
JobRunner::enqueue_job
  - input_payload.request = schedule metadata
  - input_payload.runner_meta.idempotency_key = cron:{source}:{skill}:{slot}
  - dedup checks pending/running/retry/completed
  |
  v
PTY/browser worker lane
  claim next sync_source/source_health/auth_refresh by lane filter
  |
  v
ResourceArbiter admission
  granted -> run handler
  denied -> release to pending, no attempt bump, no lease, denial backoff
  |
  v
Handler
  sync_source -> shared source body
  source_health -> canary skill
  auth_refresh -> auth-signin skill
  |
  v
complete_job / fail_job
```

### 4.2 Validation And Commercial Flow

```text
SchedulerTickThread
  dispatch validation_* and commercial_sweep
  |
  v
coverage/deterministic worker lane
  |
  +-- validation_cross_model_audit
  |     real handler persists validation audit verdict/summary
  |
  +-- validation_calibration_readiness_sweep
  |     real handler persists readiness summary
  |
  +-- commercial_sweep
        landed-cost coverage pass
          may invoke landed-cost skills under arbiter
        run_net_margin_gate_once
          pure DB pass
        manifest-generation coverage pass
          may invoke manifest skill under arbiter
```

The coverage/deterministic lane does not take a procurement-source browser lane. It is not arbiter-free: landed-cost and manifest sub-passes may consume agent/skill slots.

### 4.3 Manual Paths

```text
--ingest <source>
  create_source_sync_job_from_parameters
  claim_job_by_id
  shared source body

--run-next-job
  claim_next_job
  extended route table

desktop trigger_ingestion
  IPC enqueue sync_source
  worker or --run-next-job executes it

desktop trigger_pipeline_rerun
  stops daemon by PID
  shells daemon --ingest landed-cost/net-gate/manifest
  restarts daemon
```

The autonomous sweep never calls the desktop rerun path.

## 5. Scheduler Architecture

### 5.1 Core Types

```rust
pub struct SchedulerTickConfig {
    pub state_db_path: PathBuf,
    pub schema_epoch: i64,
    pub tick_interval: Duration,
    pub registry_generation: Arc<AtomicU64>,
    pub drain_after_tick: bool,
}

pub struct SchedulerTickState {
    source_registry: SourceRegistry,
    cron_scheduler: CronScheduler,
    observed_generation: u64,
}
```

### 5.2 Tick Body

```rust
fn run_scheduler_tick(
    scheduler: &mut CronScheduler,
    registry: &SourceRegistry,
    state_db: &mut StateDb,
    now: DateTime<Utc>,
) -> Result<usize> {
    let dispatches = scheduler.dispatch_due_jobs(registry, now)?;
    let runner = JobRunner::new(LOCAL_EXECUTOR_ID);
    let mut enqueued = 0;
    for dispatch in dispatches {
        let scheduled_for = dispatch
            .enqueue_request
            .scheduled_for
            .ok_or_else(|| anyhow!("cron dispatch missing scheduled_for"))?;
        runner.enqueue_job(state_db, &dispatch.enqueue_request)?;
        scheduler.record_enqueued(&dispatch.definition_key, scheduled_for)?;
        enqueued += 1;
    }
    Ok(enqueued)
}
```

Production supplies `Utc::now()` outside this function. Tests and `--run-scheduler-once --tick-now` supply fixed time.

### 5.3 Registry Reload

Recommended Q9 path:

```text
Arc<AtomicU64> registry_generation
  |
  +-- IPC source mutation handlers: fetch_add(1)
  |
  +-- Scheduler tick: load generation each interval
        if changed:
          SourceRegistry::load
          CronScheduler::from_registry
          refresh_next_run_at(now)
          seed_cron_scheduler_from_history
          observed_generation = generation
```

The reload process must seed from workflow history before dispatch. That avoids a newly enabled source firing a burst because its in-memory last-run state is empty.

### 5.4 Commercial Sweep Definition

Add a scheduler-owned definition alongside validation and L5 watcher definitions:

```rust
pub const COMMERCIAL_SWEEP_SOURCE_ID: &str = "commercial-sweep";
pub const COMMERCIAL_SWEEP_SKILL_ID: &str = "pipeline/commercial-sweep";
pub const COMMERCIAL_SWEEP_SKILL_VERSION: &str = "2026.06.07-1";
pub const COMMERCIAL_SWEEP_CRON: &str = "0 0 7 * * *";
```

`build_enqueue_request` gains a `CommercialSweep` class or a skill-id special case that emits `kind='commercial_sweep'`, `scope_kind='pipeline'`, and `scope_id='commercial'`.

The idempotency key still uses the cron family and scheduled slot. The body idempotency is coverage-based.

## 6. Worker Architecture

### 6.1 Lane Classification

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WorkerLaneKind {
    PtyBrowser,
    CoverageDeterministic,
}

fn lane_for_kind(kind: &str) -> Option<WorkerLaneKind> {
    match kind {
        "sync_source" | "source_health" | "auth_refresh" => Some(WorkerLaneKind::PtyBrowser),
        "validation_cross_model_audit"
        | "validation_calibration_readiness_sweep"
        | "commercial_sweep" => Some(WorkerLaneKind::CoverageDeterministic),
        "extract_document_structure"
        | "extract_document_semantics"
        | "run_bid_workflow" => Some(WorkerLaneKind::CoverageDeterministic),
        _ => None,
    }
}
```

The existing `run_next_workflow_job` should use the same route table as the always-on worker so CLI and worker behavior do not drift.

### 6.2 Lane Claiming

Current `claim_next_job` has no kind filter. Implementation should add a filtered claim function rather than claiming arbitrary jobs and pushing them back:

```rust
pub(super) fn claim_next_job_for_kinds(
    conn: &Connection,
    executor_id: &str,
    kinds: &[&str],
    not_before: Option<DateTime<Utc>>,
) -> Result<Option<ClaimedJob>>;
```

The SQL keeps the same ordering contract:

```sql
SELECT id, kind, process_id, scope_kind, scope_id, input_payload
FROM workflow_jobs
WHERE status IN ('pending', 'retry')
  AND kind IN (...)
  AND (/* denial cooldown not-before condition, if schema-backed */)
ORDER BY COALESCE(scheduled_for, created_at), created_at
LIMIT 1;
```

Recommended denial backoff is in-memory and lane-local, so no `next_eligible_at` column is needed. In that model, a lane that receives a denial sleeps the denial backoff before claiming again for the affected class/key.

### 6.3 Admission And Defer

```rust
match arbiter.admit(&admission_request) {
    ArbiterDecision::Granted(lease) => {
        let result = execute_claimed_job(...).await;
        arbiter.release(&lease, ReleaseCause::Archived);
        result
    }
    ArbiterDecision::Denied(reason) => {
        runner.defer_claimed_job_without_attempt_bump(
            conn,
            &claimed.id,
            &reason,
        )?;
        lane_cooldown.record(reason, Utc::now());
        Ok(WorkerIteration::Deferred)
    }
}
```

`defer_claimed_job_without_attempt_bump` writes:

```sql
UPDATE workflow_jobs
SET status = 'pending',
    lease_owner = NULL,
    lease_expires_at = NULL,
    started_at = NULL
WHERE id = ?1;
```

It does not increment `attempt_count`.

### 6.4 Route Table

| Kind | Lane | Executor |
|---|---|---|
| `sync_source` | PTY/browser | Shared source body, keyed by `(source_id, skill_id)`. |
| `source_health` | PTY/browser | `source-health/*` skill handler with persisted canary/source-health row. |
| `auth_refresh` | PTY/browser | `auth-signin/*` skill handler with persisted auth refresh row. |
| `validation_cross_model_audit` | Coverage/deterministic | New validation audit handler with verdict/summary side effects. |
| `validation_calibration_readiness_sweep` | Coverage/deterministic | New calibration readiness handler with summary side effects. |
| `commercial_sweep` | Coverage/deterministic | G6 commercial coverage handler. |
| `extract_document_structure` | Existing | Existing structure extraction handler. |
| `extract_document_semantics` | Existing | Existing semantics extraction handler. |
| `run_bid_workflow` | Existing | Existing bid workflow runner. |
| `enrich_process` | Operator E1 | Only routed if E1 is selected. |

## 7. Shared Source Execution Architecture

### 7.1 Runtime Handle

Recommended Q1 own-connection path introduces a minimal handle:

```rust
struct SourceExecutionRuntime {
    state_db: StateDb,
    state_db_path: PathBuf,
    artifact_root: PathBuf,
    descriptor: DaemonDescriptor,
    resource_arbiter: Arc<Mutex<ResourceArbiter>>,
    restart_controller: RestartController,
}
```

This is not a second `RuntimeBootstrap`. It contains only the services the shared source body actually needs.

### 7.2 Shared Source Function Shape

```rust
async fn execute_sync_source_job(
    runtime: &mut SourceExecutionRuntime,
    cli: &Cli,
    runner: &JobRunner,
    claimed: ClaimedJob,
) -> Result<LiveLoopReport> {
    let request = parse_runner_request(&claimed.input_payload)?;
    match (request.source_id.as_str(), request.skill_id.as_str()) {
        ("fts", "procurement-ingestion/fts") => {
            execute_fts_source_body(runtime, cli, runner, claimed, derive_fts_params(runtime)?).await
        }
        ("contracts-finder", "procurement-ingestion/contracts-finder") => {
            execute_contracts_finder_source_body(runtime, cli, runner, claimed, derive_cf_params(runtime)?).await
        }
        ("procontract", "procurement-ingestion/procontract-discover") => {
            execute_procontract_discover_body(runtime, cli, runner, claimed, derive_discover_params(runtime)?).await
        }
        ("procontract-interest", "procurement-ingestion/procontract-register-interest") => {
            execute_procontract_interest_body(runtime, cli, runner, claimed, derive_interest_params(runtime)?).await
        }
        ("procontract-enrich", "procurement-ingestion/procontract-enrich-tender") => {
            execute_procontract_enrich_body(runtime, cli, runner, claimed, derive_enrich_params(runtime)?).await
        }
        _ => fail_unsupported_source_job(...),
    }
}
```

### 7.3 Parameter Derivation

| Source binding | Derivation details |
|---|---|
| FTS | Artifact-root downloads path; registry defaults; current source state for pagination/window. |
| Contracts Finder | Same pattern as FTS. |
| ProContract discover | Artifact-root downloads path; registry/state discover window. |
| ProContract register-interest | Query candidate set through `load_procontract_interest_candidates`; no payload list. |
| ProContract enrich | Query `source_cursors` for `enrich_cursor`; artifact-root downloads path; query awaiting-access rows for `awaitingAccessTargets`. |

Cron payloads are scheduling facts. They are not trusted as execution parameters beyond source/skill identity and slot metadata.

## 8. Q1 Decision Record

| Option | Shape | Benefits | Costs |
|---|---|---|---|
| Own connection per lane | Each worker lane opens its own `StateDb` and builds a minimal runtime handle. | Matches heartbeat precedent; avoids sharing `RuntimeBootstrap.state_db`; clear thread ownership; testable with M5. | Requires extracting live-loop bodies away from `RuntimeBootstrap` assumptions. |
| Shared services handle | Workers borrow or clone a shared service container. | Can reduce duplicated setup if carefully designed. | Highest risk: accidental cross-thread sharing of mutable runtime state; harder to reason about WAL contention and lifetimes. |

Recommended default: own connection per lane.

Operator decision needed: record Q1 verbatim before implementing FR-RT.4. If the operator selects shared services, the implementation must add explicit synchronization and tests proving the serve-loop handle is not shared unsafely.

## 9. ProContract Awaiting-Access Architecture

### 9.1 Recommended Dedicated Table

If Q4 selects the recommended table path:

```sql
CREATE TABLE IF NOT EXISTS procontract_pending_enrichment (
  id TEXT PRIMARY KEY,
  process_id TEXT NOT NULL,
  notice_id TEXT NOT NULL,
  advert_id TEXT NOT NULL,
  source_notice_id TEXT,
  state TEXT NOT NULL CHECK (
    state IN ('awaiting_access', 'enriched', 'expired')
  ),
  interest_registered_at TEXT NOT NULL,
  deadline_at TEXT,
  observation_count INTEGER NOT NULL DEFAULT 0,
  last_checked_at TEXT,
  enriched_at TEXT,
  expiry_reason TEXT,
  owner_id TEXT NOT NULL DEFAULT 'local-owner',
  row_version INTEGER NOT NULL DEFAULT 1,
  last_local_seq INTEGER,
  stage_run_id TEXT,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  UNIQUE(process_id, notice_id, advert_id)
);

CREATE INDEX IF NOT EXISTS idx_procontract_pending_enrichment_state
  ON procontract_pending_enrichment(state, updated_at);

CREATE INDEX IF NOT EXISTS idx_procontract_pending_enrichment_advert
  ON procontract_pending_enrichment(advert_id);
```

Migration:

- `migrations/0015_procontract_pending_enrichment.sql`
- `DEFAULT_SCHEMA_EPOCH` 10 -> 11 in `crates/daemon/src/db/sqlite.rs`
- `EXPECTED_SCHEMA_EPOCH` / `DEFAULT_SCHEMA_EPOCH` 10 -> 11 in desktop constants

### 9.2 State Machine

```text
register-interest success
  |
  v
awaiting_access
  |
  +-- enrich sees no live tender event
  |     observation_count += 1
  |     last_checked_at = now
  |     state remains awaiting_access
  |
  +-- enrich finds live tender event + downloads docs
  |     state = enriched
  |     enriched_at = now
  |
  +-- staleness sweep passes notice deadline or max age
        state = expired
        expiry_reason = notice_deadline | max_age

enrich observes untracked not_yet_responded
  |
  v
backfill awaiting_access row
```

Landmine annotation: `interest_registration.state` remains `completed`. Do not model awaiting access by changing that row to `awaiting_access`, or register-interest candidate selection will re-pick the same notice.

### 9.3 Daemon-To-Skill Input

ProContract enrich job parameters become:

```json
{
  "source_id": "procontract-enrich",
  "cursor": "{...}",
  "download_root": "/abs/path/downloads/procontract",
  "awaitingAccessTargets": [
    {
      "advertId": "advert-123",
      "sourceNoticeId": "advert-123",
      "processId": "proc-123"
    }
  ]
}
```

Skill merge behavior:

```text
normal Recently-added scan
  collect rows up to maxPages

for each awaitingAccessTarget not already located:
  navigate directly by advertId/sourceNoticeId
  inspect activity dashboard
  find live tender event or classify not_yet_responded

union results by advertId
  enriched -> persist docs and clear row
  not_yet_responded -> keep or backfill row
  failed/blocked -> keep row with observation evidence
```

### 9.4 Persistence Hooks

Register-interest success path:

```rust
persist_interest_registrations(...)?;
for registration in successful_registrations(&result_json) {
    upsert_procontract_pending_enrichment(
        tx,
        registration.process_id,
        registration.notice_id,
        registration.advert_id,
        registration.deadline_at,
        registration.registered_at,
    )?;
}
```

Enrich result path:

```rust
for target in result.not_yet_responded {
    keep_or_backfill_pending_enrichment(tx, target, now)?;
}

for tender in result.enriched_tenders {
    persist_tender_enrichment(tx, tender)?;
    mark_pending_enrichment_enriched(tx, tender.process_id, tender.notice_id, now)?;
}

expire_stale_pending_enrichment(tx, now, expiry_policy)?;
```

The expiry policy is set by Q5. It must never use the RFx response deadline for rows still awaiting access.

## 10. Commercial Sweep Architecture

### 10.1 Handler Flow

```rust
async fn run_commercial_sweep_job(
    runtime: &mut SourceExecutionRuntime,
    runner: &JobRunner,
    claimed: ClaimedJob,
    cli: &Cli,
) -> Result<LiveLoopReport> {
    let sweep_id = allocate_stage_run_id();

    let landed = run_landed_cost_coverage_pass(runtime, cli, &sweep_id).await?;
    let net = run_net_margin_gate_once(&mut runtime.state_db, &scoped_stage_run_id(&sweep_id, "net-gate"))?;
    let manifest = run_manifest_generation_coverage_pass(runtime, cli, &sweep_id).await?;

    complete_job_with_sweep_summary(runner, runtime, claimed, landed, net, manifest)
}
```

`run_landed_cost_coverage_pass` and `run_manifest_generation_coverage_pass` should extract the coverage bodies from the existing live loops. They may invoke skills. `run_net_margin_gate_once` already exists as a pure DB pass.

### 10.2 Per-Process Transaction Rule

The sweep must not hold a transaction across all eligible processes. Recommended shape:

```text
load eligible process ids
for process_id in eligible:
  run required skill work outside write transaction
  write persisted result for that process in one short transaction
  update stage head/status for that process
```

This lets IPC and other writer classes interleave under WAL and `busy_timeout`.

### 10.3 Manual Rerun Coexistence

Manual `trigger_pipeline_rerun` can stop the daemon while a sweep is running. The architecture tolerates that:

- Already committed per-process work remains.
- The current claimed `commercial_sweep` job may be recycled or marked interrupted according to existing lease/session recovery.
- The next scheduled sweep re-queries coverage and advances missing/current-stale rows.
- There is no giant transaction to roll back the whole sweep.

## 11. Desktop Status Architecture

Q7 decides whether this batch carries status through the UI. If selected:

```text
daemon main.rs
  build_operational_status_snapshot
  build_cron_status_snapshot
    definitions[]
    scheduler_running
    last_tick_at
    worker_lanes[]
    awaiting_access_count

desktop Rust
  daemon_bridge.rs / query_service.rs
  pass new fields through snapshot structs

desktop React
  control panel read-only status row
```

The UI is read-only. It does not add a remote trigger surface.

## 12. Testing Architecture

### 12.1 Scheduler Tick Tests

New `crates/daemon/tests/scheduler_tick.rs` copies `cron_scheduler.rs` idioms:

- `ts()`
- `TestStateDb::new()`
- `seed_source`
- `CronScheduler::from_registry`
- `refresh_next_run_at`
- `dispatch_due_jobs`

Required tests:

| Test | Architecture Proof |
|---|---|
| `tick_001_due_slot_enqueues_exactly_one_row` | `run_scheduler_tick` enqueues a due slot. |
| `tick_002_repeated_ticks_same_slot_idempotent` | Repeated slot ticks dedup while row is live/succeeded. |
| Failed-slot reenqueue | A failed slot is legitimately re-enqueueable. |
| `tick_003_long_gap_catchup` | Sleep/wake or downtime gap catches up. |
| `tick_004_clock_seam` | Fixed `now` drives behavior; no inline clock in tick body. |
| `tick_005_registry_reload_on_generation_bump` | Registry reload happens only on trigger and re-seeds history. |

### 12.2 Worker Pump Tests

New `crates/daemon/tests/worker_pump.rs` copies:

- `job_claim.rs` seed/claim patterns
- `resource_arbiter.rs` admission/denial assertions
- fake-claude harness where skill side effects are involved

Each handler assertion must check real side effects, not just `status='completed'`.

### 12.3 ProContract Tests

New `crates/daemon/tests/procontract_resumption.rs` is built from scratch:

1. Register-interest canned success creates awaiting-access and leaves interest stage completed.
2. Enrich no-live-event keeps and backfills rows.
3. Enrich live-event clears row after direct advert visit and document persistence.
4. Staleness sweep expires by Q5 policy without RFx response deadline.

### 12.4 E2E Entrypoint Tests

M1 and M2 use `--run-scheduler-once --tick-now` because `RunHarness.run()` uses `Command::output()` and a live `--serve` daemon would not exit.

## 13. Implementation Sequence Reference

Detailed execution ordering belongs to the strategy document, but the architecture has these dependency edges:

1. Record operator answers for Q1 and Q4 first.
2. Add scheduler tick body and one-shot entrypoint.
3. Add worker lane claim/routing foundation and defer-on-denial.
4. Refactor shared source bodies and preserve `--ingest`.
5. Add real cron-kind handlers.
6. Add ProContract awaiting-access persistence and skill input contract.
7. Add `commercial_sweep` definition and handler.
8. Add Q7 desktop status and E1 `enrich_process` routing only if selected.
9. Run the full quality gate from the spec.
