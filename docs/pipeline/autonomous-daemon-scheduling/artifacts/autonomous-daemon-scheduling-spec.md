# Autonomous Daemon Scheduling & ProContract Resumption - Specification

> **Document Type:** Source of Truth #1 (Specification)
> **Feature:** Autonomous Daemon Scheduling & ProContract Resumption
> **Created:** 2026-06-07
> **Primary Source:** `docs/research/2026-06-04-autonomous-daemon-scheduling-research.md`
> **Supplementary Architecture:** `docs/pipeline/autonomous-daemon-scheduling/artifacts/autonomous-daemon-scheduling-architecture.md`

This specification is authoritative for implementation. If this document and the architecture document conflict, this specification wins.

## 1. Scope And Fixed Decisions

### 1.1 Mission

This feature makes the existing Rust daemon autonomous at runtime. A daemon left alive must keep evaluating configured schedules, enqueue due work, drain the queue, resume ProContract awaiting-access tenders, and advance commercial coverage without requiring a daemon restart or operator button click.

This is not a change to what the skills scrape. The current skill-first PTY/browser model remains the data-retrieval model.

### 1.2 Committed Goals

| Goal | Committed Outcome |
|---|---|
| G1 | The daemon fires due schedules autonomously while running continuously, including catch-up after sleep or downtime. |
| G2 | Cron-enqueued jobs execute end-to-end via an always-on worker, including `sync_source`, `source_health`, `auth_refresh`, `validation_cross_model_audit`, `validation_calibration_readiness_sweep`, and new `commercial_sweep`. |
| G3 | Manual CLI paths keep working, `trigger_ingestion` becomes executable through the worker, and manual `trigger_pipeline_rerun` remains a separate desktop affordance. |
| G4 | ProContract awaiting-access items are tracked durably and swept by enrich independently of the portal's Recently-added ordering. |
| G5 | Runtime autonomy is concurrency-correct under local SQLite WAL, the existing serve loop, the writer-lease heartbeat, the new scheduler thread, and the worker lanes. |
| G6 | The commercial stages `landed-cost` -> `net-gate` -> `manifest` advance autonomously through a scheduled coverage sweep. |

### 1.3 Recorded Decisions

The research document resolved these decisions. Implementation must not re-open them.

| Decision | Fixed Constraint |
|---|---|
| Q2 scheduler startup fold | Startup enqueue is folded into the scheduler tick. The tick body runs once immediately at thread start for cold-start catch-up, then on interval. The prior `enqueue_startup_cron_jobs` path is removed when the tick lands. Duplicate bootstrap-vs-first-tick dispatch is already dedup-closed by the same idempotency key. |
| Q3 worker lanes | Worker execution uses two lanes by job class: a PTY/browser lane for `sync_source`, `source_health`, and `auth_refresh`; and a coverage/deterministic lane for `commercial_sweep` and `validation_*`. This is falsified by `worker_004_head_of_line`; fallback is a kind-aware pool only if the test cannot pass. |
| Q8 principal commercial trigger | Commercial autonomy uses a new scheduled `commercial_sweep` cron kind with a coverage-driven in-process body on the worker coverage/deterministic lane. It must not call `trigger_pipeline_rerun`. |
| Clock seam | `run_scheduler_tick(scheduler, registry, db, now)` is a design requirement. Production supplies `Utc::now()` from the loop. Tests and the one-shot autonomous entrypoint supply a fixed RFC3339 instant. |
| Schedule retuning | Existing source schedule values stay as currently defined. This feature adds exactly one new commercial sweep slot around 07:00 UTC. |

### 1.4 Operator Decisions Needed

These are not scope cuts. The feature still ships the underlying capability. Each decision has a recommended default and a concrete extension point. Operator answers must be recorded verbatim before implementation beads that depend on the decision are started.

| ID | Decision Needed | Options | Recommended Default | Extension Point |
|---|---|---|---|---|
| Q1 | Worker execution context | Own `StateDb` connection per worker lane; or a shared services handle. | Own connection per lane, matching `SyncLeaseHeartbeat::spawn` and avoiding shared mutable runtime handles. | `AutonomousWorkerRuntime` construction and live-loop body refactor. |
| Q4 | Awaiting-access model | Dedicated `procontract_pending_enrichment` table; or an additional state value in existing process-stage status. | Dedicated table with migration `0015_procontract_pending_enrichment.sql` and schema epoch 10 -> 11 across daemon and desktop constants. | G4 persistence and schema migration. |
| Q5 | Awaiting-access expiry policy | Notice-level `deadline_at`, max-age since interest, or both. | Both: expire when notice-level `deadline_at` is past when known, otherwise when max age is exceeded. | G4 staleness sweep. |
| Q6 | Shadow flag for health/auth handlers | Gate `source_health` and `auth_refresh` autonomous execution behind a shadow flag first; or fully enforce immediately. | Shadow flag first, while still routing the jobs and recording side effects. | G2 handlers and worker routing. |
| Q7 | Desktop scheduler/status UX | Add read-only status fields in daemon snapshot, desktop Rust bridge, and React UI; or keep daemon-only docs/tests for this batch. | Add minimal read-only fields: scheduler running, last tick, next run, awaiting-access count. | `CronStatusSnapshot`, desktop daemon bridge/query service, control panel rendering. |
| Q8-residual | Chained commercial nudge | Also enqueue a deduped `commercial_sweep` after successful ingestion; or rely only on the scheduled 07:00 coverage sweep. | Scheduled sweep only unless the operator wants lower latency. | Successful `sync_source` completion path. |
| Q9 | Registry reload trigger | Monotonic registry generation counter bumped by source-mutation IPC handlers; or poll `MAX(sources.updated_at)` each tick. | Generation counter, with newly enabled sources re-seeding last-run history. | Scheduler tick thread and source mutation handlers. |
| E1 | Desktop `trigger_enrichment` routing | Route `enrich_process` through a process-enrichment worker handler; or leave the button as a named separate work item while `trigger_ingestion` ships. | Route only if the operator wants this button fixed with G3. It is not cron-core. | Worker kind routing for `enrich_process`. |

### 1.5 Guardrail Non-Goals

| ID | Guardrail |
|---|---|
| NFR-1 | Do not change procurement or supplier scraping selectors except the ProContract enrich input/work-list changes required by G4. |
| NFR-2 | Do not replace skill-first PTY/browser scraping with API connectors. |
| NFR-3 | Do not add multi-org or multi-daemon execution semantics. The current model remains one daemon per org. |
| NFR-4 | Do not add a new remote HTTP trigger surface. |
| NFR-5 | Do not create compatibility shims around deprecated APIs. Refactor the direct live-loop bodies and queue handlers. |

## 2. Functional Requirements

### 2.1 G1 - Autonomous Scheduler Tick

#### FR-G1.1 Background Scheduler Thread

**Requirement:** The `--serve` daemon must spawn a recurring scheduler-tick thread that opens its own `StateDb` connection and dispatches due cron jobs while the daemon remains alive.

**Acceptance Criteria:**

- `AC-FR-G1.1-a`: The scheduler thread follows the `SyncLeaseHeartbeat::spawn` precedent: it owns a stop channel, loops on `recv_timeout`, and opens its own connection to the same `state.db`.
- `AC-FR-G1.1-b`: The thread builds `SourceRegistry` and `CronScheduler`, refreshes `next_run_at`, and seeds `last_run_at` from history once at thread start.
- `AC-FR-G1.1-c`: The tick does not rebuild/re-seed the scheduler every interval unless the registry reload trigger fires.
- `AC-FR-G1.1-d`: The scheduler tick is separate from the worker lanes. A long PTY scrape cannot block due-slot evaluation.
- `AC-FR-G1.1-e`: The existing one-shot bootstrap enqueue path is removed or reduced to the same tick body; there is no second independent schedule-dispatch implementation.

**Core Tests:** `scheduler_tick.rs` `tick_001_due_slot_enqueues_exactly_one_row`, `tick_003_long_gap_catchup`; E2E M1.

#### FR-G1.2 Clock-Injection Seam And One-Shot Entrypoint

**Requirement:** The scheduler tick body must accept an injected `DateTime<Utc>` and expose a one-shot autonomous CLI entrypoint for deterministic end-to-end tests.

**Acceptance Criteria:**

- `AC-FR-G1.2-a`: The tick body signature is equivalent to `run_scheduler_tick(scheduler, registry, db, now: DateTime<Utc>)`.
- `AC-FR-G1.2-b`: The tick body contains no inline `Utc::now()` call. The loop supplies production time; tests supply fixed instants.
- `AC-FR-G1.2-c`: Add `--run-scheduler-once --tick-now <rfc3339>`.
- `AC-FR-G1.2-d`: The one-shot entrypoint evaluates schedules at the injected instant, enqueues due jobs, drains worker lanes to quiescence, and exits 0.
- `AC-FR-G1.2-e`: A tick before a source's slot enqueues nothing; the same tick at or after the slot enqueues the expected row.

**Core Tests:** `scheduler_tick.rs` `tick_004_clock_seam`; E2E M1/M2.

#### FR-G1.3 Cron Enqueue Idempotency Bound

**Requirement:** Repeated scheduler ticks for the same slot must enqueue at most one live-or-succeeded job for that slot, while allowing a slot whose prior job failed to be re-enqueued later.

**Acceptance Criteria:**

- `AC-FR-G1.3-a`: Cron enqueue uses the existing idempotency key shape `cron:{source_id}:{skill_id}:{scheduled_for.to_rfc3339()}`.
- `AC-FR-G1.3-b`: The implementation relies on `find_existing_enqueue_ack` status filtering for `pending`, `running`, `retry`, and `completed`.
- `AC-FR-G1.3-c`: The spec and implementation do not promise exactly-once forever. A previous `failed` job for the slot is legitimately re-enqueueable.
- `AC-FR-G1.3-d`: Startup catch-up and first interval tick are dedup-safe because the race occurs while the job is still live, not failed.

**Core Tests:** `scheduler_tick.rs` `tick_002_repeated_ticks_same_slot_idempotent`; a failed-slot reenqueue assertion in `scheduler_tick.rs`.

#### FR-G1.4 Registry Reload Trigger

**Requirement:** The scheduler must reload source definitions only when the source registry changes and must avoid spurious catch-up bursts for newly enabled sources.

**Acceptance Criteria:**

- `AC-FR-G1.4-a`: Q9 operator decision is recorded before implementation starts for this requirement.
- `AC-FR-G1.4-b`: Recommended path: an `AtomicU64` registry generation counter is bumped by every IPC handler that mutates source enablement or schedule-bearing source configuration.
- `AC-FR-G1.4-c`: If the polling option is selected, each tick polls only a cheap maximum-updated timestamp or equivalent generation value, not the full registry.
- `AC-FR-G1.4-d`: On reload, the scheduler re-seeds last-run history for newly enabled sources before dispatching, so enablement does not cause an unbounded catch-up burst.

**Core Tests:** `scheduler_tick.rs` `tick_005_registry_reload_on_generation_bump`.

#### FR-G1.5 Sleep/Wake Catch-Up

**Requirement:** A daemon that survives host sleep or time gaps must enqueue due overdue slots at the next tick without manual restart.

**Acceptance Criteria:**

- `AC-FR-G1.5-a`: Catch-up uses the scheduler's existing `schedule.after(last_run_at)` behavior.
- `AC-FR-G1.5-b`: The tick records enqueued slots in memory through `record_enqueued`.
- `AC-FR-G1.5-c`: Host sleep tainting for in-flight PTY sessions remains governed by the existing host-sleep detection and does not suppress the scheduler tick.

**Core Tests:** `scheduler_tick.rs` `tick_003_long_gap_catchup`; E2E M1.

### 2.2 G2 - Continuous Worker Pump And Cron Kind Routing

#### FR-G2.1 Two-Lane Worker Pump

**Requirement:** The daemon must spawn continuous worker lanes that claim, route, execute, and complete queued jobs without operator action.

**Acceptance Criteria:**

- `AC-FR-G2.1-a`: The PTY/browser lane filters for `sync_source`, `source_health`, and `auth_refresh`.
- `AC-FR-G2.1-b`: The coverage/deterministic lane filters for `commercial_sweep`, `validation_cross_model_audit`, and `validation_calibration_readiness_sweep`.
- `AC-FR-G2.1-c`: Each lane has an idle backoff when no job is claimable and never busy-spins.
- `AC-FR-G2.1-d`: Claims preserve overdue-first ordering within each lane, based on `COALESCE(scheduled_for, created_at), created_at`.
- `AC-FR-G2.1-e`: The head-of-line claim is falsifiable: a slow PTY `sync_source` does not delay a ready coverage/deterministic job.
- `AC-FR-G2.1-f`: If lane separation cannot satisfy `worker_004_head_of_line`, implementation switches to a kind-aware pool that preserves the same resource caps.

**Core Tests:** `worker_pump.rs` `worker_004_head_of_line`, `worker_005_arbiter_quota_respected`.

#### FR-G2.2 Resource Arbiter Admission And Defer-On-Denial

**Requirement:** Autonomous workers must honor `ResourceArbiter` admission exactly and treat denial as a defer, not a failed attempt.

**Acceptance Criteria:**

- `AC-FR-G2.2-a`: Worker admission uses `browser_slots=2`, `agent_slots=4`, `procurement_source_lanes=1`, and `supplier_source_lanes=2` by default.
- `AC-FR-G2.2-b`: Denied admission sets the job back to `status='pending'`.
- `AC-FR-G2.2-c`: Denied admission does not increment `attempt_count`.
- `AC-FR-G2.2-d`: Denied admission clears any job lease and holds no resource lease.
- `AC-FR-G2.2-e`: Denied admission applies denial backoff. Recommended model: in-memory per-lane cool-down keyed by denial class and source/kind to avoid a schema change.
- `AC-FR-G2.2-f`: Denial handling does not reuse failure retry machinery. A denial is not an attempt.

**Core Tests:** `worker_pump.rs` `worker_002_defer_on_denial_no_attempt_bump`; `worker_003_source_lane_serializes_same_source`.

#### FR-G2.3 Cron Kind Handler Coverage

**Requirement:** Every cron-enqueued kind must have a real executor and must no longer hit the current `other => fail_job` unsupported-kind branch.

**Acceptance Criteria:**

- `AC-FR-G2.3-a`: `sync_source` routes to the refactored live-loop body described by FR-RT.1 through FR-RT.4.
- `AC-FR-G2.3-b`: `source_health` invokes the existing `source-health/*` canary skill path and persists a canary/source-health result row.
- `AC-FR-G2.3-c`: `auth_refresh` invokes the existing `auth-signin/*` skill path and persists an auth/refresh result row.
- `AC-FR-G2.3-d`: `validation_cross_model_audit` has a new handler that persists validation verdict/summary evidence and is distinct from semantic per-result validation jobs.
- `AC-FR-G2.3-e`: `validation_calibration_readiness_sweep` has a new handler that persists readiness summary evidence.
- `AC-FR-G2.3-f`: `commercial_sweep` has the G6 handler and side effects.
- `AC-FR-G2.3-g`: Empty handlers that only mark jobs completed do not satisfy this requirement. Each handler test must assert a real side-effect row.

**Core Tests:** `worker_pump.rs` `worker_001_routes_each_cron_kind`.

#### FR-G2.4 Health/Auth Shadow Flag Decision

**Requirement:** If Q6 selects a shadow flag, `source_health` and `auth_refresh` execution must be observable before it becomes enforcement-grade.

**Acceptance Criteria:**

- `AC-FR-G2.4-a`: Q6 operator decision is recorded before handlers are wired to production schedules.
- `AC-FR-G2.4-b`: Recommended shadow mode still claims, executes, persists side effects, and completes jobs; it only gates enforcement outcomes that could disable sources or block production runs.
- `AC-FR-G2.4-c`: Logs and status snapshots identify shadow-mode handler outcomes.
- `AC-FR-G2.4-d`: Validation jobs and `commercial_sweep` are not accidentally gated by the health/auth shadow flag.

**Core Tests:** `worker_pump.rs` handler side-effect assertions plus status snapshot assertions when Q6 selects shadow mode.

### 2.3 Cross-Cutting Live-Loop Refactor

#### FR-RT.1 Shared Source Execution Body

**Requirement:** Source scraping bodies must be executable both from a claimed queued job and from the existing `--ingest` CLI path.

**Acceptance Criteria:**

- `AC-FR-RT.1-a`: Extract each source live-loop body so it accepts a claimed job, runtime/services handle, and CLI/config inputs.
- `AC-FR-RT.1-b`: The shared body does not mint or claim its own job.
- `AC-FR-RT.1-c`: Thin `--ingest` wrappers keep enqueueing and `claim_job_by_id`-ing the new job, then call the shared body.
- `AC-FR-RT.1-d`: Existing `--ingest fts`, `--ingest contracts-finder`, `--ingest procontract-discover`, `--ingest procontract-register-interest`, and `--ingest procontract-enrich` regression tests continue to pass.

**Core Tests:** existing `agent_procurement_ingestion.rs`, `agent_contracts_finder_ingestion.rs`; new ProContract regression cases in `procontract_resumption.rs`.

#### FR-RT.2 Source Body Mapping

**Requirement:** `sync_source` routing must map by `(source_id, skill_id)`, not by `source_id` alone.

**Acceptance Criteria:**

- `AC-FR-RT.2-a`: FTS maps to the FTS body.
- `AC-FR-RT.2-b`: Contracts Finder maps to the Contracts Finder body.
- `AC-FR-RT.2-c`: ProContract discover, register-interest, and enrich are differentiated by skill binding even though their logical portal family is shared.
- `AC-FR-RT.2-d`: Marketplace search loops that are `--ingest`-only remain executable manually but are not incorrectly treated as source cron production slots unless a schedule exists.
- `AC-FR-RT.2-e`: Commercial stages are not routed through `sync_source`.

**Core Tests:** `worker_pump.rs` `worker_001_routes_each_cron_kind`; live-loop regression tests.

#### FR-RT.3 Execution-Time Parameter Derivation

**Requirement:** The shared source body must derive execution parameters at run time from daemon state because cron payloads carry schedule metadata, not CLI execution parameters.

| `source_id` / binding | Body to invoke | Parameters derived at execution time |
|---|---|---|
| `fts` + `procurement-ingestion/fts` | FTS shared body | `download_root` from artifact root; page/window defaults from registry and current source state. |
| `contracts-finder` + `procurement-ingestion/contracts-finder` | Contracts Finder shared body | Same derivation class as FTS: artifact-root download path plus registry defaults. |
| `procontract` + `procurement-ingestion/procontract-discover` | ProContract discover shared body | `download_root` from artifact root; discover window defaults from registry/state. |
| `procontract-interest` + `procurement-ingestion/procontract-register-interest` | ProContract register-interest shared body | Candidate set from `load_procontract_interest_candidates`; not a payload-provided list. |
| `procontract-enrich` + `procurement-ingestion/procontract-enrich-tender` | ProContract enrich shared body | Cursor from `source_cursors`; `download_root` from artifact root; awaiting-access work-list from G4 tracking model. |

**Acceptance Criteria:**

- `AC-FR-RT.3-a`: Cron payload fields such as `source_id`, `skill_id`, `skill_version`, `cron_expression`, `scheduled_for`, and `schedule_key` are treated as scheduling metadata.
- `AC-FR-RT.3-b`: No handler assumes the cron payload already contains source-specific execution parameters.
- `AC-FR-RT.3-c`: The ProContract enrich body injects the G4 awaiting-access work-list when present.

**Core Tests:** `worker_pump.rs` `worker_001_routes_each_cron_kind`; `procontract_resumption.rs`.

#### FR-RT.4 Worker Runtime Context Decision

**Requirement:** Worker lane runtime context must be chosen through Q1 and then applied consistently across source bodies and commercial sweep.

**Acceptance Criteria:**

- `AC-FR-RT.4-a`: Q1 operator decision is recorded before live-loop refactor implementation starts.
- `AC-FR-RT.4-b`: Recommended own-connection-per-lane path opens a fresh `StateDb` for each lane and constructs a minimal runtime/services handle around that connection.
- `AC-FR-RT.4-c`: If shared services is selected, the implementation must prove it does not share `RuntimeBootstrap.state_db` unsafely across threads.
- `AC-FR-RT.4-d`: The chosen runtime context allows skill execution, artifact-root access, state writes, job completion/failure, resource-arbiter admission, and status logging.

**Core Tests:** `worker_pump.rs`, M5 concurrency soak.

### 2.4 G3 - Manual Paths And Desktop Trigger Interaction

#### FR-G3.1 CLI Manual Path Preservation

**Requirement:** Existing CLI manual entrypoints must keep working through the refactored bodies.

**Acceptance Criteria:**

- `AC-FR-G3.1-a`: `--ingest <source>` still enqueues, claims by ID, executes, completes, and exits as today.
- `AC-FR-G3.1-b`: `--run-next-job` uses the extended job-kind routing and can drain any cron kind it is allowed to execute.
- `AC-FR-G3.1-c`: `--pipeline <source_id>` and `run_deterministic_pipeline` remain callable and do not become the commercial sweep trigger.
- `AC-FR-G3.1-d`: `--run-scheduler-once --tick-now` is a new deterministic entrypoint and does not replace existing manual entrypoints.

**Core Tests:** M4 manual-path regression.

#### FR-G3.2 Desktop `trigger_ingestion` Becomes Live

**Requirement:** The existing desktop `trigger_ingestion` IPC command must become executable because its `sync_source` job is now routable.

**Acceptance Criteria:**

- `AC-FR-G3.2-a`: `trigger_ingestion` continues to enqueue `kind='sync_source'`.
- `AC-FR-G3.2-b`: The worker or `--run-next-job` can claim that job and execute the correct source body.
- `AC-FR-G3.2-c`: The job reaches `completed` or a legitimate handler failure; it does not remain pending solely because `sync_source` is unsupported.
- `AC-FR-G3.2-d`: No new remote trigger surface is introduced.

**Core Tests:** M4 desktop trigger-ingestion path, modeled on `ipc_trigger_bid_workflow.rs`.

#### FR-G3.3 Manual Pipeline Rerun Coexists With Autonomous Sweep

**Requirement:** The manual desktop `trigger_pipeline_rerun` path stays a PID-stop one-shot affordance and must coexist safely with an in-process autonomous commercial sweep.

**Acceptance Criteria:**

- `AC-FR-G3.3-a`: Autonomous commercial sweep does not call `trigger_pipeline_rerun`, `stop_daemon_for_exclusive_pipeline_rerun`, or the desktop one-shot subprocess loop.
- `AC-FR-G3.3-b`: If an operator clicks "Re-run pipeline" while `commercial_sweep` is running, the daemon may receive SIGTERM mid-sweep through the existing desktop path.
- `AC-FR-G3.3-c`: Mid-sweep termination is safe because commercial sweep writes are per-process scoped and committed incrementally.
- `AC-FR-G3.3-d`: The next scheduled `commercial_sweep` is coverage-idempotent and picks up any process not completed before termination.
- `AC-FR-G3.3-e`: The two paths must not corrupt each other's stage rows, manifests, or decision basis state.

**Core Tests:** M2 plus a manual-rerun coexistence assertion in M4 or M5.

#### FR-G3.4 Desktop Status Surfacing Decision

**Requirement:** Q7 determines whether this batch adds read-only status surfacing for scheduler/autonomy health.

**Acceptance Criteria:**

- `AC-FR-G3.4-a`: Q7 operator decision is recorded before desktop files are touched.
- `AC-FR-G3.4-b`: If selected, daemon `CronStatusSnapshot` adds scheduler-running state, last tick instant, worker lane health, and awaiting-access count.
- `AC-FR-G3.4-c`: If selected, desktop Rust bridge/query service carries the new fields without mutating daemon state.
- `AC-FR-G3.4-d`: If selected, desktop React renders the fields in the control panel with read-only behavior.
- `AC-FR-G3.4-e`: If not selected for this batch, the extension point remains named in the artifacts and no silent deletion of Q7 occurs.

**Core Tests:** Cargo gate for daemon/desktop Rust; Bun test/build/lint only when Q7 selects UI rendering.

#### FR-G3.5 Desktop `trigger_enrichment` Routing Decision

**Requirement:** The `enrich_process` job kind remains explicitly named as a separate operator decision and is not silently conflated with cron autonomy.

**Acceptance Criteria:**

- `AC-FR-G3.5-a`: `enrich_process` is not a cron kind and is not required for G1/G2/G6.
- `AC-FR-G3.5-b`: If E1 selects routing, the worker gains a real process-enrichment handler and M4 asserts the desktop enrichment button completes.
- `AC-FR-G3.5-c`: If E1 is not selected for this batch, `trigger_ingestion` still ships as live and the `enrich_process` extension point remains documented.

**Core Tests:** M4.

### 2.5 G4 - ProContract Awaiting-Access Resumption

#### FR-G4.1 Awaiting-Access Persistence Model

**Requirement:** The daemon must persist a durable awaiting-access record when ProContract register-interest succeeds, without changing the completed interest-registration stage state.

**Acceptance Criteria:**

- `AC-FR-G4.1-a`: Q4 operator decision is recorded before implementation starts for this requirement.
- `AC-FR-G4.1-b`: Recommended dedicated-table path creates `procontract_pending_enrichment` through migration `0015_procontract_pending_enrichment.sql`.
- `AC-FR-G4.1-c`: Dedicated-table path bumps SQLite schema epoch 10 -> 11 and updates daemon and all desktop epoch constants in the same commit.
- `AC-FR-G4.1-d`: State-string path does not bump the schema epoch but must widen the re-registration guard so `awaiting_access` counts as already registered.
- `AC-FR-G4.1-e`: In all paths, `process_stage_status` for `stage='interest_registration'` remains `state='completed'`, `state='succeeded'`, or another existing completed terminal state after successful register-interest.
- `AC-FR-G4.1-f`: Register-interest must not re-express interest on an item already tracked as awaiting access.

**Core Tests:** `procontract_resumption.rs` M3 step 1; `sqlite_schema.rs` conditional migration test.

#### FR-G4.2 Awaiting-Access Lifecycle

**Requirement:** Every transition in the ProContract awaiting-access lifecycle must have an explicit trigger and row mutation.

| Transition | Trigger | State / row written |
|---|---|---|
| create | Register-interest succeeds. | Insert `procontract_pending_enrichment(process_id, notice_id, advert_id, state='awaiting_access', interest_registered_at, deadline_at, observation_count=0, last_checked_at=NULL, enriched_at=NULL)`. Do not touch `interest_registration.state`; it stays `completed`. |
| keep | Enrich pass runs and portal still shows no live tender event. | Keep same row with `state='awaiting_access'`; bump `observation_count` and `last_checked_at` only. |
| clear | Enrich finds live tender event and downloads documents. | Set `state='enriched'`; record `enriched_at` and final `last_checked_at`. Recommended path keeps the terminal row for auditability. |
| backfill | Enrich observes a portal item as `not_yet_responded` with no tracking row. | Insert a row as in create, with best-known `interest_registered_at` from daemon state or observed portal evidence. |
| expire | Staleness sweep detects past notice deadline or max-age since interest. | Set `state='expired'`; record `last_checked_at`; stop sweeping it. Must not use RFx response deadline because it is unknown before access is granted. |

**Acceptance Criteria:**

- `AC-FR-G4.2-a`: `observation_count` is separate from workflow job `attempt_count`.
- `AC-FR-G4.2-b`: Terminal states are auditable and do not cause register-interest to fire again.
- `AC-FR-G4.2-c`: Expiry policy is governed by Q5 and must not key off the unknown RFx response deadline.

**Core Tests:** `procontract_resumption.rs` M3 steps 1-4.

#### FR-G4.3 Enrich Work-List Sweep

**Requirement:** ProContract enrich must sweep daemon-tracked awaiting-access records independent of the portal Recently-added window.

**Acceptance Criteria:**

- `AC-FR-G4.3-a`: The daemon derives `awaitingAccessTargets` from rows with `state='awaiting_access'`.
- `AC-FR-G4.3-b`: Each target includes `advertId`, `sourceNoticeId`, and `processId`.
- `AC-FR-G4.3-c`: The skill still performs the normal Recently-added scan.
- `AC-FR-G4.3-d`: For targets not located by the scan, the skill navigates directly by `advertId` or equivalent stored ProContract identifier.
- `AC-FR-G4.3-e`: Scan results and direct-target results are unioned and deduped by `advertId`.
- `AC-FR-G4.3-f`: Empty work-list behavior is identical to the current scan-only path.
- `AC-FR-G4.3-g`: Fake-claude test evidence proves the direct target was visited by `advertId`, not found only through Recently-added.

**Core Tests:** `procontract_resumption.rs` M3 steps 2-3.

#### FR-G4.4 Skill Contract Extension

**Requirement:** `skills/procurement-ingestion/procontract-enrich-tender` must accept the optional daemon work-list and report enough evidence for persistence transitions.

**Acceptance Criteria:**

- `AC-FR-G4.4-a`: Skill inputs add optional `awaitingAccessTargets: [{ advertId, sourceNoticeId, processId }]`.
- `AC-FR-G4.4-b`: Result schema or run manifest carries enough evidence to distinguish enriched, not-yet-responded, failed, and directly visited targets.
- `AC-FR-G4.4-c`: The skill keeps mandatory CDP discipline on `localhost:9222` and never launches an isolated browser profile.
- `AC-FR-G4.4-d`: A blocked CDP/auth result keeps awaiting-access rows in `awaiting_access` and records observation/failure evidence without expiring them.

**Core Tests:** skill schema lint/canary; `procontract_resumption.rs`.

### 2.6 G5 - Concurrency Correctness

#### FR-G5.1 StateDb Ownership And Thread Model

**Requirement:** Background autonomy must not share the serve-loop `RuntimeBootstrap.state_db` handle across threads.

**Acceptance Criteria:**

- `AC-FR-G5.1-a`: Serve/IPC keeps owning its existing `state_db` handle.
- `AC-FR-G5.1-b`: Lease heartbeat keeps its own connection.
- `AC-FR-G5.1-c`: Scheduler tick owns its own connection.
- `AC-FR-G5.1-d`: Worker lanes follow the Q1 decision, with recommended own connection per lane.
- `AC-FR-G5.1-e`: `ResourceArbiter` remains synchronized for cross-lane admission and release.

**Core Tests:** M5 concurrency soak.

#### FR-G5.2 WAL Single-Writer Discipline

**Requirement:** The implementation must remain correct with up to four concurrent local SQLite writers.

**Acceptance Criteria:**

- `AC-FR-G5.2-a`: Implementation relies on the existing WAL, `busy_timeout=5000`, and in-process write mutex behavior.
- `AC-FR-G5.2-b`: Write transactions are short.
- `AC-FR-G5.2-c`: Commercial sweep uses per-process-scoped transactions, not one giant transaction.
- `AC-FR-G5.2-d`: IPC mutations are not starved past `busy_timeout` by a commercial sweep.
- `AC-FR-G5.2-e`: If M5 exposes permanent `SQLITE_BUSY` or lost writes, implementation escalates to a single-writer actor.

**Core Tests:** M5 counter/unique-row concurrency soak.

#### FR-G5.3 Same-Source Serialization

**Requirement:** Two same-source procurement `sync_source` runs must not execute concurrently.

**Acceptance Criteria:**

- `AC-FR-G5.3-a`: `procurement_source_lanes=1` remains the default invariant.
- `AC-FR-G5.3-b`: A second same-source procurement `sync_source` admission receives `DenialReason::SourceLaneExhausted { source_id, limit: 1, in_use: 1 }`.
- `AC-FR-G5.3-c`: The denied second job defers through FR-G2.2 and later runs after the first source lane releases.
- `AC-FR-G5.3-d`: Supplier source lanes retain their existing limit of 2.

**Core Tests:** `worker_pump.rs` `worker_003_source_lane_serializes_same_source`; M5.

#### FR-G5.4 Lease And Sleep/Wake Interaction

**Requirement:** Worker job leases, host-sleep detection, and scheduler catch-up must interact without duplicated live work.

**Acceptance Criteria:**

- `AC-FR-G5.4-a`: Worker claims use the existing 300-second lease and 60-second heartbeat model or a direct extension of it.
- `AC-FR-G5.4-b`: Expired running leases recycle to `retry` through the existing claim path.
- `AC-FR-G5.4-c`: Host sleep detection may taint in-flight PTY sessions but does not disable schedule catch-up.
- `AC-FR-G5.4-d`: A retry caused by lease expiry still goes through resource admission before execution.

**Core Tests:** M5 plus existing job-claim lease tests.

### 2.7 G6 - Autonomous Commercial Sweep

#### FR-G6.1 Commercial Sweep Cron Slot

**Requirement:** Add one scheduler-owned `commercial_sweep` cron definition around 07:00 UTC after the ingestion window.

**Acceptance Criteria:**

- `AC-FR-G6.1-a`: The scheduler includes a new `commercial_sweep` definition in addition to existing source-backed, validation, and L5 scheduler-owned definitions.
- `AC-FR-G6.1-b`: Existing source schedule values are not retuned.
- `AC-FR-G6.1-c`: `cron_scheduler.rs` schedule-anchor tests are updated for the new definition count and ordering.
- `AC-FR-G6.1-d`: The idempotency key uses the same per-slot cron dedup family and is bounded by FR-G1.3.

**Core Tests:** `cron_scheduler.rs` updated anchor/catch-up tests; `scheduler_tick.rs`.

#### FR-G6.2 Coverage-Driven Commercial Body

**Requirement:** The `commercial_sweep` handler must run `landed-cost` -> `run_net_margin_gate_once` -> manifest-generation over eligible rows in-process.

**Acceptance Criteria:**

- `AC-FR-G6.2-a`: The sweep advances every eligible process/candidate lacking a current downstream decision.
- `AC-FR-G6.2-b`: `net-gate` runs through the pure DB `run_net_margin_gate_once` path.
- `AC-FR-G6.2-c`: `landed-cost` and manifest-generation may invoke their existing skill runners for eligible rows.
- `AC-FR-G6.2-d`: Coverage lane jobs do not consume a procurement-source browser lane.
- `AC-FR-G6.2-e`: Skill-backed sub-passes still consume agent/skill capacity and remain under `ResourceArbiter`.
- `AC-FR-G6.2-f`: The sweep body is idempotent by coverage: a second sweep over the same window is a near-no-op for rows already current.

**Core Tests:** `worker_pump.rs` `worker_001_routes_each_cron_kind`; E2E M2.

#### FR-G6.3 No Reuse Of Desktop Pipeline Rerun

**Requirement:** Autonomous commercial sweep must reconcile with, but not reuse, the desktop PID-stop pipeline rerun path.

**Acceptance Criteria:**

- `AC-FR-G6.3-a`: The sweep does not call Tauri `trigger_pipeline_rerun`.
- `AC-FR-G6.3-b`: The sweep does not stop the daemon, wait for runtime-lock release, spawn one-shot `daemon --ingest landed-cost`, `--ingest net-gate`, or `--ingest manifest`, or relaunch the daemon.
- `AC-FR-G6.3-c`: Manual pipeline rerun remains available to the desktop operator.
- `AC-FR-G6.3-d`: Mid-run manual daemon stop is safe through per-process transactions and coverage idempotency.

**Core Tests:** E2E M2 and M4.

#### FR-G6.4 Chained Nudge Decision

**Requirement:** Q8-residual determines whether successful ingestion also nudges a deduped `commercial_sweep`.

**Acceptance Criteria:**

- `AC-FR-G6.4-a`: Q8-residual operator decision is recorded before touching ingestion completion hooks.
- `AC-FR-G6.4-b`: If selected, the nudge uses the same dedup family and cannot enqueue redundant commercial work for already-current rows.
- `AC-FR-G6.4-c`: If not selected, the scheduled 07:00 coverage sweep remains sufficient for correctness.
- `AC-FR-G6.4-d`: The nudge must not blur source-scraper `sync_source` concerns with commercial stage execution.

**Core Tests:** E2E M2 plus a nudge-specific test only if Q8-residual selects it.

## 3. User Stories

| ID | Story | Mapped Requirements |
|---|---|---|
| US-1 | As the operator, I want the daemon to fire schedules while it stays open so morning procurement runs do not depend on restart habits. | FR-G1.1-FR-G1.5 |
| US-2 | As the operator, I want scheduled jobs to execute automatically so queued source and validation work does not become an orphaned backlog. | FR-G2.1-FR-G2.4, FR-RT.1-FR-RT.4 |
| US-3 | As the operator, I want existing manual commands and desktop controls to keep working while autonomy is added. | FR-G3.1-FR-G3.5 |
| US-4 | As the operator, I want ProContract awaiting-access tenders to resume even after several days of portal delay. | FR-G4.1-FR-G4.4 |
| US-5 | As a maintainer, I want the runtime design to be testable under fixed clocks, WAL contention, and resource saturation. | FR-G1.2, FR-G2.2, FR-G5.1-FR-G5.4 |
| US-6 | As the operator, I want commercial decisions to advance after ingestion without clicking "Re-run pipeline". | FR-G6.1-FR-G6.4 |

## 4. E2E Test Scenarios

All tests use real SQLite state and the real daemon binary where the flow demands it. The deterministic fake-claude CLI may stand in for agent/PTY output only; it must not replace daemon runtime behavior.

| Manifest | Scenario | Expected Outcome |
|---|---|---|
| M1 | Invoke `--run-scheduler-once --tick-now 2026-06-05T04:15:00Z` with FTS enabled. | A `sync_source` FTS job is enqueued, drained, completed, and persists FTS-produced rows with no `--ingest` or IPC trigger. Negative control at 03:00 enqueues nothing. |
| M2 | Invoke one-shot scheduler at the 07:00 commercial sweep slot after an eligible process exists. | `landed-cost` -> `net-gate` -> `manifest` rows become current in-process, with no daemon stop/restart path. |
| M3 | Run ProContract register-interest and enrich canned outcomes through fake-claude. | Awaiting-access row is created, kept, backfilled, cleared, and expired correctly; interest registration remains completed. |
| M4 | Exercise `--ingest`, `--run-next-job`, desktop `trigger_ingestion`, and manual `trigger_pipeline_rerun`. | Manual paths still work; desktop `sync_source` job becomes executable; rerun remains manual PID-stop affordance. |
| M5 | Run worker lanes, IPC mutations, lease heartbeat, and same-source overlap under WAL. | No lost writes, no permanent `SQLITE_BUSY`, and no concurrent same-source procurement scrape. |

## 5. Dependencies

- Existing Rust daemon runtime in `crates/daemon/src/main.rs`.
- Existing scheduler in `crates/daemon/src/scheduler/cron.rs`.
- Existing job runner, claim, and idempotency code in `crates/daemon/src/jobs/`.
- Existing `ResourceArbiter` admission model in `crates/daemon/src/agents/arbiter.rs`.
- Existing SQLite WAL/busy-timeout configuration in `crates/daemon/src/db/sqlite.rs`.
- Existing procurement ingestion and commercial live-loop functions.
- Existing ProContract enrich skill and result schema.
- Optional Q7 desktop status path in `apps/desktop`.

## 6. Risks And Mitigations

| Risk | Severity | Mitigation |
|---|---|---|
| Duplicate cron dispatch at startup/tick boundary | High | Fold startup enqueue into tick; existing live-or-succeeded idempotency key dedup; `tick_002`. |
| Failed slot is accidentally suppressed forever | High | State idempotency as live-or-succeeded only; failed rows are re-enqueueable. |
| Same-source PTY scrapes corrupt browser/cursors | High | `procurement_source_lanes=1`; defer-on-denial; `worker_003` and M5. |
| Worker head-of-line blocks commercial/validation coverage | Medium | Two-lane worker; `worker_004`; kind-aware pool fallback if falsified. |
| WAL writer contention starves IPC | High | Short writes, per-process commercial transactions, `busy_timeout=5000`, M5, single-writer actor escalation. |
| Awaiting-access model re-fires register-interest | High | Dedicated tracking row recommended; `interest_registration.state` stays completed; M3 step 1. |
| ProContract activity ages past Recently-added page 5 | High | Daemon-tracked work-list direct by advert ID plus scan/direct union. |
| Autonomous commercial sweep collides with manual rerun | Medium | Per-process transactions and coverage idempotency; manual PID-stop path remains separate. |
| Desktop schema epoch mismatch if Q4 table is chosen | High | Migration `0015_*`, daemon and all desktop constants 10 -> 11 in one commit; schema test. |

## 7. Success-Criteria Coverage Matrix

| Goal | Correct Proof | Regression Shape | Gate |
|---|---|---|---|
| G1 | M1 plus `scheduler_tick.rs` `tick_001`, `tick_003`, `tick_004`. | Due slot never enqueues without restart; no tick thread; inline wall-clock-only test path. | Full Rust stage gate, including per-file integration loop. |
| G2 | `worker_pump.rs` `worker_001` proves every cron kind has side effects and never hits unsupported-kind fail. | Any cron kind reaches `other => fail_job`; empty handler only flips status. | Full Rust stage gate. |
| G3 | M4 proves `--ingest`, `--run-next-job`, `trigger_pipeline_rerun`, and `trigger_ingestion` behavior. | Desktop ingestion job remains pending; manual CLI paths break after refactor. | Rust integration loop; desktop gates if Q7/E1 touches desktop. |
| G4 | M3 full lifecycle and schema test conditional on Q4. | Enrich misses aged-out item; register-interest re-expresses interest; expiry uses RFx response deadline. | Full Rust integration loop plus skill lint/schema checks. |
| G5 | M5 plus `worker_003` proves no lost writes, no permanent `SQLITE_BUSY`, and no same-source overlap. | Lost counter rows, surfaced busy errors, or concurrent FTS scrape. | Full Rust integration loop. |
| G6 | M2 proves current manifest/decision from `commercial_sweep` with no daemon stop/restart. | Manifest appears only after manual rerun; sweep invokes desktop PID-stop path. | Full Rust integration loop. |

## 8. Test Target Inventory

| File | Required Work |
|---|---|
| `crates/daemon/tests/scheduler_tick.rs` | New tests `tick_001` through `tick_005`, plus failed-slot reenqueue assertion. |
| `crates/daemon/tests/worker_pump.rs` | New tests `worker_001` through `worker_005`, including anti-hollow-handler side-effect checks. |
| `crates/daemon/tests/procontract_resumption.rs` | New ProContract lifecycle suite built from scratch on `RunHarness` plus fake-claude. |
| `crates/daemon/tests/cron_scheduler.rs` | Update definition count, schedule anchors, and catch-up ordering for `commercial_sweep`. |
| `crates/daemon/tests/job_claim.rs` | Add new-kind claim/routing tests without mutating existing stage-job ordering assertions. |
| `crates/daemon/tests/agent_procurement_ingestion.rs` | Existing FTS live-loop regression must pass unchanged after refactor. |
| `crates/daemon/tests/agent_contracts_finder_ingestion.rs` | Existing Contracts Finder live-loop regression must pass unchanged after refactor. |
| `crates/daemon/tests/sqlite_schema.rs` | If Q4 selects dedicated table: assert epoch 11, `0015_*` migration, and table columns. If Q4 selects state string: assert widened re-fire guard. |

## 9. Quality Gate

A green stage for this feature requires:

```bash
cargo fmt --check
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test -p daemon --bin daemon
for t in crates/daemon/tests/*.rs; do
  cargo test --test "$(basename "$t" .rs)" || exit 1
done
```

If Q7 adds desktop UI rendering, also run the relevant Bun tests/build/lint for `apps/desktop`.

## 10. Operator Question Prompts

Record answers verbatim in the implementation status artifact before coding begins:

1. Q1: Should autonomous worker lanes use own `StateDb` connections per lane, or a shared services handle?
2. Q4: Should ProContract awaiting-access use a dedicated `procontract_pending_enrichment` table with epoch 10 -> 11, or a state-string model with widened re-registration guard?
3. Q5: What expiry policy should awaiting-access use: notice deadline, max age, or both?
4. Q6: Should `source_health` and `auth_refresh` autonomous handlers begin in shadow mode?
5. Q7: Should the desktop control panel surface scheduler running, last tick, next run, and awaiting-access count in this batch?
6. Q8-residual: Should successful ingestion also enqueue a deduped commercial-sweep nudge, or is the scheduled 07:00 sweep sufficient?
7. Q9: Should registry reload use a generation counter or timestamp polling?
8. E1: Should `trigger_enrichment` / `enrich_process` routing be implemented with this feature?
