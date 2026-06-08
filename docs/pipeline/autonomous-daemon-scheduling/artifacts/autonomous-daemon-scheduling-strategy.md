# Autonomous Daemon Scheduling & ProContract Resumption - Implementation Strategy

## Source Documents

| Document | Path | Role |
|----------|------|------|
| Spec | `docs/pipeline/autonomous-daemon-scheduling/artifacts/autonomous-daemon-scheduling-spec.md` | SOT #1 - WHAT to build |
| TDD | `docs/pipeline/autonomous-daemon-scheduling/artifacts/autonomous-daemon-scheduling-tdd.md` | SOT #2 - HOW to verify |
| Manifest | `docs/pipeline/autonomous-daemon-scheduling/manifest.yaml` | Feature surface, recorded decisions, open questions, and gates |
| Research | `docs/research/2026-06-04-autonomous-daemon-scheduling-research.md` | Sequencing reference, especially sections 4.4 and 11 |
| Quality-bar reference | `docs/pipeline/goods-led-install-maintenance/artifacts/goods-led-install-maintenance-strategy.md` | Structure, gate specificity, and coverage-map benchmark |

## Strategy Guardrails

- Phases are execution order, not importance tiers. Every committed feature surface ships.
- The strategy routes work only. The spec owns feature content; the TDD owns test assertions.
- Q1 is the first blocking decision because it shapes worker runtime context, live-loop extraction, and WAL pressure.
- The clock seam and `--run-scheduler-once --tick-now` entrypoint land before downstream E2E work.
- Phase gates are runnable TDD IDs or explicit quality-gate IDs. A phase-review statement is never a substitute gate.
- The live-loop refactor is a foundation dependency for both `sync_source` worker routing and manual CLI preservation.
- G4 ProContract resumption lands after G1/G2 because scheduled enrich must exist before the awaiting-access work-list can drain autonomously.
- G6 commercial sweep is a full phase with its own cron slot, worker body, and M2 gate.
- Existing source schedule values are frozen. This feature adds exactly one schedule slot: 07:00 UTC `commercial_sweep`.
- G5 concurrency hardening runs after all writer surfaces exist, so M5 exercises the real contention shape.
- Q7 desktop status surfacing is last because it reads scheduler, worker, and awaiting-access state produced earlier.
- Operator-gated items are named in their owning phase and gate branch. None are silently dropped.
- Bead content comes from the spec and TDD. This strategy supplies routing only.
- Bead-tracked execution uses `bin/br-claim <bead-id>` and `bin/br-complete <bead-id>`, never raw status updates.

## Pre-Flight

| Check | Required Outcome |
|-------|------------------|
| Branch | Work is on `main` |
| Agent Mail | Session identity, peer handshakes, and narrow file reservations exist before bead-tracked edits |
| Existing docs | Spec, TDD, bootstrap, architecture, manifest, and this strategy are present |
| Operator decisions | Q1, Q4, Q5, Q6, Q7, Q8-residual, Q9, and E1 are recorded before dependent implementation beads start |
| Q1 first | Worker execution context is recorded before Phase 0 live-loop and worker-runtime work begins |
| Q4 branch | Dedicated-table branch starts Phase 4 with migration `0015_*` and epoch `10 -> 11`; state-string branch starts with the widened re-fire guard |
| Full Rust gate | The per-file daemon integration loop is the Rust stage gate, not only `cargo test -p daemon --bin daemon` |
| Desktop gate | Q7-selected UI work adds the relevant `apps/desktop` Bun build/test/lint gate |

## Overview

| Phase | Name | Prerequisites | Gate Type | Target Beads |
|------|------|---------------|-----------|--------------|
| 0 | Foundations, Q1, Clock Seam, And Live-Loop Refactor | Spec/TDD accepted, decisions ready to record | decision, harness, refactor regression | 12 |
| 1 | Scheduler Tick And Registry Reload | Phase 0 harness and route surface | `tick_001..007`, M1 enqueue path | 10 |
| 2 | Worker Pump And Cron Kind Routing | Phase 0 refactor, Phase 1 tick body | `worker_001..005`, M1 drain path, M4 ingestion path | 14 |
| 3 | Commercial Sweep Autonomy | Phases 1-2 | 07:00 cron slot, M2, commercial side effects | 11 |
| 4 | ProContract Awaiting-Access Resumption | Phases 1-2; Phase 0 ProContract body extraction | M3 lifecycle, Q4 schema branch, skill contract | 13 |
| 5 | Concurrency Hardening And E2E Soak | Phases 1-4 writer surfaces | M5, R1-R8, full Rust gate | 10 |
| 6 | Desktop Status Surfacing Decision Branch | Phases 1-5 state surfaces | `desktop_status_001` when selected, recorded branch otherwise | 6 |

## Critical Path And Parallelism

- Root critical path: `0 -> 1 -> 2 -> 3 -> 4 -> 5 -> 6`.
- Phase 0 is root because Q1, the clock seam, the one-shot entrypoint, and extracted source bodies unblock every E2E route.
- Phase 1 depends on Phase 0 because the tick body and one-shot harness must exist before scheduler tests can drive fixed time.
- Phase 2 depends on Phase 0 and Phase 1 because worker draining must use the same route table as `--run-scheduler-once`.
- Phase 3 depends on Phases 1-2 because `commercial_sweep` is a scheduled worker kind, not a desktop rerun path.
- Phase 4 depends on Phases 1-2 because ProContract enrich must run on schedule before durable awaiting-access resumption has autonomous effect.
- Phase 5 depends on Phases 1-4 because the soak must include scheduler, worker lanes, commercial sweep, ProContract persistence, IPC, and heartbeat writers.
- Phase 6 depends on Phases 1-5 because Q7 status reads scheduler, lane, and awaiting-access state.
- Phase 0 has two parallel lanes after Q1 is recorded: clock/one-shot harness and live-loop extraction.
- Phase 2 has two parallel lanes after route-table shape is stable: lane pump/admission and net-new handler bodies.
- Phase 3 can update scheduler anchors in parallel with commercial body routing after the worker route table exists.
- Phase 4 can run schema branch work in parallel with skill-contract schema work after Q4 is recorded.
- Merge-hot surfaces are serialized:
  - Q1 runtime context and live-loop body extraction: Phase 0 only.
  - Tick body, startup fold, and Q9 registry reload: Phase 1 only.
  - Worker claim loop, resource denial, and cron-kind route table: Phase 2 only.
  - `commercial_sweep` schedule and coverage body: Phase 3 only.
  - `procontract_pending_enrichment` or state-string guard: Phase 4 only.
  - Cross-writer soak and single-writer escalation decision: Phase 5 only.
  - Q7 daemon/desktop/React surfacing: Phase 6 only.

## Follow-Up Dependency Audit

| Dependency Edge | Created In | First Consumer | Blocking Gate |
|-----------------|------------|----------------|---------------|
| Q1 runtime context decision | Phase 0 | Worker lanes, source bodies, commercial sweep | `decision_001_operator_decisions_recorded`, M5 |
| Clock-injected tick body | Phase 0 | Scheduler thread, one-shot entrypoint, M1, M2 | `tick_004_clock_seam` |
| `--run-scheduler-once --tick-now` harness | Phase 0 | M1, M2, downstream phase gates | M1/M2 harness checks |
| Shared source execution bodies | Phase 0 | `sync_source` route, `--ingest`, M3, M4 | live-loop regression IDs |
| Scheduler startup fold | Phase 1 | Live `--serve` scheduler and cold-start catch-up | `tick_002`, `tick_007` |
| Q9 registry reload trigger | Phase 1 | Continuous scheduler tick | `tick_005` |
| Worker lane route table | Phase 2 | All cron kind execution, M1/M2/M4 | `worker_001`, `impact_job_claim_new_kinds_claimable` |
| Resource denial semantics | Phase 2 | Same-source and arbiter pressure | `worker_002`, `worker_003`, `worker_005` |
| `commercial_sweep` cron definition | Phase 3 | M2 and M5 writer pressure | `impact_cron_scheduler_001`, `impact_cron_scheduler_006` |
| Commercial coverage body | Phase 3 | Q8 nudge branch and M5 R8 | `worker_001`, M2 |
| Q4 awaiting-access store | Phase 4 | ProContract enrich work-list, Q7 counts | Q4 schema branch, M3 |
| ProContract enrich skill input | Phase 4 | M3 direct advert visit proof | `skill_contract_001_awaiting_access_targets` |
| All local writer surfaces | Phase 5 | Full gate and concurrency closure | M5 |
| Q7 status fields | Phase 6 | Desktop control panel read-only rendering | `desktop_status_001` |

No consumer above depends on a surface created in a later phase.

## Code Foothold Audit

| Phase | First Task Anchor | Current Code Foothold |
|-------|-------------------|-----------------------|
| 0 | Q1 runtime and live-loop extraction | `crates/daemon/src/main.rs:20096`, `19171`, `19382`, `19481`, `19627`; `SyncLeaseHeartbeat::spawn` precedent at `main.rs:3260` |
| 1 | scheduler thread and startup fold | `SyncLeaseHeartbeat::spawn` at `main.rs:3260`; `build_cron_status_snapshot` at `main.rs:3890`; cron scheduler tests already exist |
| 2 | lane-filtered claim and route table | `claim_next_job` at `crates/daemon/src/jobs/claim.rs:72`; ordering at `claim.rs:176`; `run_next_workflow_job` at `main.rs:1703`; arbiter denial variants in `agents/arbiter.rs:82` |
| 3 | in-process commercial body | `run_deterministic_pipeline` at `main.rs:23323`; commercial live loops at `main.rs:10532`, `12345`, and `15043` |
| 4 | ProContract lifecycle and work-list | ProContract live loops at `main.rs:19382`, `19481`, `19627`; candidate guard at `main.rs:15371`; schema epoch at `db/sqlite.rs:89` |
| 5 | writer and lease soak | WAL/busy timeout at `db/sqlite.rs:215` and `218`; claim leases in `jobs/claim.rs`; arbiter in `agents/arbiter.rs:152` |
| 6 | read-only status surfacing | `build_cron_status_snapshot` at `main.rs:3890`; desktop schema/bridge constants in `apps/desktop/src-tauri` |

## Suggested Bead Fan-Out

| Phase | Task Beads | Epic / Review Beads |
|------|------------|---------------------|
| 0 | 12 | 1 |
| 1 | 10 | 1 |
| 2 | 14 | 1 |
| 3 | 11 | 1 |
| 4 | 13 | 1 |
| 5 | 10 | 1 |
| 6 | 6 | 1 |

Expected total: `76` task beads plus `7` phase sub-epic/review beads.

## Phase Balance Audit

| Phase | Primary Requirements | Route Atoms | Gate Families | Target Beads | Balance Check |
|-------|----------------------|-------------|---------------|--------------|---------------|
| 0 | 6 primary, 4 support | 9 | decision, clock seam, one-shot harness, live-loop regressions, M4 foundation, guard, CI | 12 | Foundation carries Q1, clock seam, one-shot harness, and source refactor |
| 1 | 5 primary, 1 support | 8 | `tick_001..003`, `tick_005..007`, overdue-first claim, M1 enqueue, CI | 10 | Scheduler tick is full-depth but narrower than worker and G4 |
| 2 | 6 primary, 2 support | 14 | `worker_001..005`, new-kind claimability, Q6, E1, M1 drain, M4 trigger ingestion, CI | 14 | Worker owns lanes, admission, all cron-kind executors, Q6, and E1 |
| 3 | 5 primary, 2 support | 10 | cron 001/006 impacts, commercial route/body, M2, M4/M5 coexistence, Q8, guard, CI | 11 | G6 is not thin: cron slot, route, body, rerun coexistence, Q8, M2 |
| 4 | 5 primary, 1 support | 10 | Q4 branch, `M3_step_001..004`, full M3, skill contract, Q5, guard, skill canaries, CI | 13 | G4 is not thin: Q4/Q5, 5-state lifecycle, net-new M3 suite, skill contract |
| 5 | 5 primary, 2 guardrails | 5 | M5, `worker_003`, lease tests, R1-R8 proof map, guard scopes, full Rust gate | 10 | G5 waits for all writers and then owns R1-R8 plus full gate |
| 6 | 1 primary, 1 support | 1 | Q7 decision, `desktop_status_001`, daemon/desktop/React read-only proof, Bun gate branch | 6 | Q7 is a bounded read-only surfacing branch with daemon/Rust/React gates |

Later phases are equal shipping phases. Phase 3 and Phase 4 both carry implementation gates and bead fan-out at or above the scheduler-tick phase, while Phase 5 owns the full contention proof after all writer surfaces exist.

## Manifest Feature Surface Count Audit

Counts below come from the current `manifest.yaml` `feature_surface`. Compound manifest bullets are split into route atoms only when a single bullet names multiple concrete deliverables; each atom has one primary phase home in the Feature Surface Routing table.

| Manifest Surface | Manifest Bullets | Route Atoms | Primary Phase Home |
|------------------|------------------|-------------|--------------------|
| `scheduler_tick` | 9 | `S1..S9` | Phase 0 for `S4..S5`; Phase 1 for `S1..S3`, `S6..S9` |
| `worker_pump` | 6 | `W1..W6` | Phase 2 |
| `new_job_kind_handlers` | 5 | `H1..H5` | Phase 2 |
| `live_loop_refactor` | 5 | `R1..R5` | Phase 0 |
| `commercial_sweep` | 7 | `C1..C7` | Phase 3 |
| `procontract_resumption` | 7 | `P1..P7` | Phase 4 |
| `concurrency_correctness` | 5 | `K1..K5` | Phase 0 for `K1`; Phase 5 for `K2..K5` |
| `desktop_status_surfacing` | 1 | `U1` | Phase 6 |
| `tests` | 4 | `T1..T12` | Phases 0 through 5 by first closure gate |

No manifest bullet is routed to more than one primary owner. Supporting gates may appear in earlier phases, but implementation ownership remains singular.

## Phase Bead Routing Batches

Rows are routing labels, not miniature specifications. Beads in the same parallel group may run concurrently after the listed predecessor or phase gate is green.

| Phase | Routing Batches | Must Follow | Primary Gate IDs |
|-------|-----------------|-------------|------------------|
| 0 | operator-decision capture; Q1 runtime context; clock seam; one-shot entrypoint; drain-to-quiescence harness; source live-loop extraction; `(source_id, skill_id)` route keys; runtime parameter derivation; manual `--ingest` preservation | accepted spec/TDD | `decision_001`, `tick_004`, M1/M2 harness shape, `impact_ingestion_regression_001`, `impact_contracts_finder_live_loop_regression_001`, M4 |
| 1 | scheduler thread; startup fold; idempotent slot enqueue; failed-slot reenqueue; catch-up; overdue-first scheduled drain; Q9 registry reload; live-serve spawn/poll/terminate | Phase 0 | `tick_001..007`, existing job-claim overdue-first assertion, M1 enqueue path |
| 2 | two worker lanes; idle backoff; claim filters; defer-on-denial; denial backoff; all `DenialReason` variants; route-table extension; source health; auth refresh; validation audit; validation readiness; `trigger_ingestion`; E1 branch | Phases 0-1 | `worker_001..005`, `impact_job_claim_new_kinds_claimable`, `shadow_health_auth_001`, M1, M4, `trigger_enrichment_001` |
| 3 | 07:00 schedule key; cron scheduler anchors; commercial body; arbiter use for skill sub-passes; desktop rerun coexistence; Q8-residual branch | Phases 1-2 | `impact_cron_scheduler_001`, `impact_cron_scheduler_006`, `worker_001`, M2, M4, `commercial_nudge_001` |
| 4 | Q4 schema branch; Q5 expiry branch; lifecycle transitions; work-list sweep; direct advert visit; skill contract extension; staleness sweep; Q7 awaiting count source | Phases 1-2 plus Phase 0 ProContract extraction | `M3_step_001..004`, M3, `skill_contract_001`, Q4 schema tests |
| 5 | connection ownership audit; WAL short-write proof; same-source serialization soak; lease/sleep interaction; R1-R8 map; full Rust gate; guardrail scans | Phases 1-4 | M5, `worker_003`, existing lease tests, guard scope IDs, `CI-ADS-001..005` |
| 6 | Q7 daemon snapshot; desktop Rust bridge; desktop React rendering; read-only behavior; Bun gate branch; named non-rendering branch | Phases 1-5 | `desktop_status_001`, `decision_001`, `CI-ADS-006` when selected |

## Implementation Unit Allocation

| Surface Group | Home Phase | Ordering Reason |
|---------------|------------|-----------------|
| Q1 worker execution context | 0 | Worker runtime and live-loop body signatures depend on it |
| Clock seam and one-shot entrypoint | 0 | M1/M2 cannot be written against a blocking live `--serve` harness |
| Shared source bodies and manual wrappers | 0 | Worker `sync_source` must reuse the same bodies as `--ingest` |
| Scheduler tick thread and startup fold | 1 | Due-slot evaluation must be autonomous before workers drain scheduled jobs |
| Q9 registry reload trigger | 1 | Tick rebuild/reseed behavior is scheduler-owned |
| Worker lanes and admission semantics | 2 | All scheduled kinds depend on claim, route, execute, and completion behavior |
| Health/auth/validation handlers | 2 | These are cron-kind executors and belong with route-table closure |
| Desktop `trigger_ingestion` execution | 2 | It becomes live when `sync_source` is routable |
| E1 `enrich_process` branch | 2 | It is a worker route-table decision, not cron-core scheduler work |
| Commercial 07:00 slot and coverage body | 3 | It consumes the scheduler and worker but does not depend on G4 |
| Q8-residual chained nudge | 3 | It is a commercial-sweep enqueue branch after the principal sweep model exists |
| ProContract awaiting-access store and lifecycle | 4 | It depends on scheduled enrich and the extracted ProContract bodies |
| ProContract enrich skill contract | 4 | It supplies the work-list execution contract for G4 |
| WAL and full contention soak | 5 | It must exercise all concurrent writer classes after they exist |
| Q7 status surfacing | 6 | It reads scheduler, worker, and awaiting-access state produced earlier |

## Phase 0: Foundations, Q1, Clock Seam, And Live-Loop Refactor

**Target beads:** `12`

### Prerequisites

- Spec, TDD, manifest, and bootstrap are present.
- Operator decisions can be recorded before dependent work starts.
- No overlapping reservation exists on the live-loop and CLI entrypoint surfaces.

### Requirement IDs Covered

- Primary: `FR-G1.2`, `FR-RT.1`, `FR-RT.2`, `FR-RT.3`, `FR-RT.4`, `FR-G3.1`.
- Guardrail home: `NFR-5`.
- Foundation support for `FR-G2.3`, `FR-G3.2`, `FR-G4.3`, and `FR-G6.2`.

### Test IDs Covered

- `decision_001_operator_decisions_recorded`.
- `tick_004_clock_seam`.
- M1/M2 one-shot harness shape.
- `impact_ingestion_regression_001_fts_unchanged`.
- `impact_contracts_finder_existing_contracts_unchanged`.
- `impact_contracts_finder_live_loop_regression_001`.
- M4 manual `--ingest` coverage.
- `guard_scope_005_no_compat_shims`.

### Ordering

- Sequential: record Q1 -> choose runtime context -> freeze worker runtime construction path -> allow live-loop refactor beads.
- Sequential: clock seam -> one-shot entrypoint -> drain-to-quiescence route use -> M1/M2 harness readiness.
- Sequential: source body extraction -> thin manual wrappers -> FTS regression -> Contracts Finder regression -> ProContract body readiness.
- Sequential: `(source_id, skill_id)` route-key surface -> execution-time parameter derivation -> worker route-table consumption.
- Parallel after Q1: clock/one-shot lane and live-loop extraction lane.
- No worker-only alternate executor is created in this phase.

### Gate Criteria

- `decision_001_operator_decisions_recorded` includes Q1 before any worker-runtime or live-loop implementation bead starts.
- `tick_004_clock_seam` proves injected time and the before-slot negative control.
- M1/M2 can drive `--run-scheduler-once --tick-now` without using a wall-clock `--serve` wait.
- `impact_ingestion_regression_001_fts_unchanged` stays green.
- `impact_contracts_finder_existing_contracts_unchanged` stays green.
- `impact_contracts_finder_live_loop_regression_001` adds the missing real daemon run.
- M4 still owns the broader manual-path regression, but its `--ingest` foundation is available here.
- `guard_scope_005_no_compat_shims` is green for the refactor surface.
- `CI-ADS-001..005` pass for touched Rust surfaces.

### Roll Back If

- Q1 cannot be recorded before worker/runtime signatures are changed.
- The one-shot entrypoint needs a separate executor that bypasses worker route/admission/completion paths.
- Shared source bodies cannot preserve manual `--ingest` behavior.
- Route lookup cannot distinguish ProContract bindings by `(source_id, skill_id)`.

## Phase 1: Scheduler Tick And Registry Reload

**Target beads:** `10`

### Prerequisites

- Phase 0 gate passed.
- One-shot harness and injected-clock tick body are available.

### Requirement IDs Covered

- Primary: `FR-G1.1`, `FR-G1.3`, `FR-G1.4`, `FR-G1.5`, `FR-G1.6`.
- Supporting proof for `FR-G5.5`.

### Test IDs Covered

- `tick_001_due_slot_enqueues_exactly_one_row`.
- `tick_002_repeated_ticks_same_slot_idempotent`.
- `tick_003_long_gap_catchup`.
- `tick_005_registry_reload_on_generation_bump`.
- `tick_006_failed_slot_reenqueueable`.
- `tick_007_serve_scheduler_thread_spawn_poll_terminate`.
- Existing `job_claim.rs` overdue-first ordering assertion.
- M1 enqueue path.

### Ordering

- Sequential: scheduler thread -> startup fold -> cold-start first tick -> interval tick.
- Sequential: due-slot enqueue -> idempotent repeat -> failed-slot reenqueue -> long-gap catch-up.
- Sequential: Q9 decision record -> registry reload trigger -> newly-enabled source reseed.
- Sequential: scheduled `scheduled_for` preservation -> overdue-first claim order proof.
- Parallel after thread shape: idempotency tests and registry reload tests.
- Live `--serve` proof stays bounded to spawn/poll/terminate; M1 remains one-shot.

### Gate Criteria

- `tick_001`, `tick_002`, `tick_003`, `tick_005`, `tick_006`, and `tick_007` pass.
- Existing `job_claim.rs` overdue-first ordering remains green.
- Startup enqueue is folded into the tick body or delegates to the same body and idempotency tuple.
- Q9 selected reload mechanism is recorded before implementation of `FR-G1.4`.
- Registry reload does not create a newly-enabled catch-up burst.
- M1 can enqueue the FTS due slot through the one-shot path.
- `CI-ADS-001..005` pass for scheduler surfaces.

### Roll Back If

- The scheduler thread needs to share the serve-loop state handle.
- Startup and steady-state dispatch require divergent enqueue paths.
- Failed slots cannot be re-enqueued without adding an auxiliary cron dedup surface.
- Registry reload must rebuild the full registry every tick.

## Phase 2: Worker Pump And Cron Kind Routing

**Target beads:** `14`

### Prerequisites

- Phase 0 route/body foundation passed.
- Phase 1 scheduler enqueue semantics passed.
- Q6 and E1 branch decisions are recorded before dependent handlers start.

### Requirement IDs Covered

- Primary: `FR-G2.1`, `FR-G2.2`, `FR-G2.3`, `FR-G2.4`, `FR-G3.2`, `FR-G3.5`.
- Supporting proof for `FR-G5.3` and `FR-G5.5`.

### Test IDs Covered

- `worker_001_routes_each_cron_kind`.
- `worker_002_defer_on_denial_no_attempt_bump`.
- `worker_003_source_lane_serializes_same_source`.
- `worker_004_head_of_line`.
- `worker_005_arbiter_quota_respected`.
- `impact_job_claim_new_kinds_claimable`.
- `shadow_health_auth_001`.
- `trigger_enrichment_001`.
- M1 drain path and M4 `trigger_ingestion`.

### Ordering

- Sequential: lane claim filters -> idle backoff -> route table -> completion/failure paths.
- Sequential: arbiter admission -> defer-on-denial -> denial backoff -> all denial variants.
- Sequential: `sync_source` matrix -> source health -> auth refresh -> validation audit -> validation readiness.
- Sequential: `trigger_ingestion` enqueue -> worker drain -> M4 desktop path proof.
- Sequential: record Q6 -> health/auth shadow branch -> status/snapshot evidence when selected.
- Sequential: record E1 -> `enrich_process` handler branch or named extension branch.
- Parallel after claim loop: handler bodies and denial/admission tests.

### Gate Criteria

- `worker_001..005` pass, including anti-hollow side-effect assertions for every cron kind.
- `worker_001` proves the `sync_source` route-key matrix for FTS, Contracts Finder, and the three ProContract bindings.
- `worker_002` proves denial is defer, not an attempt.
- `worker_003` proves same-source procurement serialization.
- `worker_004` proves lane separation prevents PTY head-of-line blocking.
- `worker_005` forces every current `DenialReason` variant and proves releases.
- `impact_job_claim_new_kinds_claimable` is green without mutating existing ordering tests.
- `shadow_health_auth_001` is green when Q6 selects shadow mode.
- M1 drains a scheduled `sync_source` through the worker path.
- M4 proves desktop `trigger_ingestion` no longer leaves an unsupported pending job.
- `trigger_enrichment_001` follows the recorded E1 branch.
- `CI-ADS-001..005` pass for worker surfaces.

### Roll Back If

- Any cron kind still reaches the unsupported-kind failure branch.
- Any handler can satisfy review by status completion alone.
- Resource denial increments `attempt_count` or leaves a held lease.
- Lane separation cannot satisfy `worker_004` and a kind-aware pool fallback is not explicitly selected.

## Phase 3: Commercial Sweep Autonomy

**Target beads:** `11`

### Prerequisites

- Phase 1 scheduler tick gate passed.
- Phase 2 worker route/admission gate passed.
- Q8-residual decision is recorded before ingestion completion hooks are touched.

### Requirement IDs Covered

- Primary: `FR-G3.3`, `FR-G6.1`, `FR-G6.2`, `FR-G6.3`, `FR-G6.4`.
- Guardrail home: `NFR-4`.
- Supporting proof for `FR-G5.2` and `FR-G5.5`.

### Test IDs Covered

- `impact_cron_scheduler_001_anchor_set_includes_0700_commercial_sweep`.
- `impact_cron_scheduler_006_catchup_order_includes_commercial_sweep_key`.
- `worker_001_routes_each_cron_kind` commercial subcase.
- M2.
- M4 manual-rerun coexistence assertions.
- `commercial_nudge_001`.
- `guard_scope_004_no_remote_http_trigger`.

### Ordering

- Sequential: scheduler-owned definition -> anchor count update -> catch-up ordering update -> per-slot dedup proof.
- Sequential: worker route arm -> commercial body -> landed-cost/net-gate/manifest side-effect assertions -> M2.
- Sequential: manual rerun coexistence -> no desktop PID-stop reuse -> M4/M5 coexistence proof.
- Sequential: record Q8-residual -> nudge branch or scheduled-only branch.
- Parallel after route arm: cron scheduler test updates and commercial body side-effect work.
- The commercial body does not route through `sync_source`.

### Gate Criteria

- `impact_cron_scheduler_001` proves the new 07:00 schedule key and exact definition count.
- `impact_cron_scheduler_006` proves catch-up ordering includes the commercial key.
- `worker_001` proves the commercial side-effect row family.
- M2 proves the commercial stages advance from `commercial_sweep` with no daemon stop/restart path.
- M4 or M5 proves manual `trigger_pipeline_rerun` coexists with the autonomous sweep.
- `commercial_nudge_001` is green when Q8-residual selects the chained nudge.
- `guard_scope_004_no_remote_http_trigger` is green.
- Existing source schedule anchors remain unchanged except for the added commercial key.
- `CI-ADS-001..005` pass for scheduler, worker, and commercial surfaces.

### Roll Back If

- Commercial autonomy requires `trigger_pipeline_rerun` or desktop PID-stop/relaunch behavior.
- The 07:00 slot cannot use the same cron enqueue/idempotency family as other cron jobs.
- Coverage sweep side effects cannot be proven without manual pipeline rerun.
- Q8 nudge blurs source scraper execution with commercial stage execution.

## Phase 4: ProContract Awaiting-Access Resumption

**Target beads:** `13`

### Prerequisites

- Phase 0 ProContract shared bodies are available.
- Phase 1 scheduler gate passed.
- Phase 2 worker/enrich routing gate passed.
- Q4 and Q5 are recorded before dependent persistence and expiry work starts.

### Requirement IDs Covered

- Primary: `FR-G4.1`, `FR-G4.2`, `FR-G4.3`, `FR-G4.4`, `FR-G4.5`.
- Guardrail home: `NFR-1`.
- Supporting proof for `FR-G5.5`.

### Test IDs Covered

- `M3_step_001_register_creates_awaiting_access_without_refire`.
- `M3_step_002_enrich_keep_and_backfill`.
- `M3_step_003_enrich_clear_via_worklist`.
- `M3_step_004_staleness_expire`.
- `skill_contract_001_awaiting_access_targets`.
- `M3_procontract_resumption_full_lifecycle_g4`.
- `schema_0015_procontract_pending_enrichment`.
- `schema_state_string_refire_guard`.
- `impact_sqlite_schema_q4_table`.
- `impact_sqlite_schema_q4_state_string`.
- `guard_scope_001_skill_selector_boundary`.

### Ordering

- Sequential: record Q4 -> schema branch first task -> lifecycle persistence.
- Dedicated-table branch: migration `0015_*` -> all four daemon/desktop epoch `10 -> 11` constants -> schema tests -> lifecycle work.
- State-string branch: no epoch bump proof -> widened re-fire guard -> lifecycle work.
- Sequential: record Q5 -> expiry policy -> staleness sweep -> M3 step 4.
- Sequential: create -> keep/backfill -> clear via work-list -> expire.
- Sequential: skill input schema -> canary/lint -> direct advert visit proof -> full M3.
- Parallel after Q4 branch foundation: skill contract work and lifecycle row-transition work.
- Q7 awaiting-access count source is prepared here but rendered in Phase 6.

### Gate Criteria

- The selected Q4 branch has its corresponding schema/refire guard test green before lifecycle beads depend on it.
- Dedicated-table branch starts this phase with migration `0015_*` and all four daemon/desktop epoch `10 -> 11` constants in the same change.
- State-string branch proves no migration and a widened re-registration guard.
- `M3_step_001..004` pass individually.
- `M3_procontract_resumption_full_lifecycle_g4` passes as the ordered lifecycle scenario.
- `skill_contract_001_awaiting_access_targets` proves the work-list contract and CDP discipline.
- `guard_scope_001_skill_selector_boundary` is green.
- `desktop_status_001` can consume active/terminal counts later when Q7 selects status surfacing.
- `CI-ADS-001..005` pass, plus skill lint/canary gates for the ProContract enrich skill.

### Roll Back If

- Awaiting-access modeling makes register-interest eligible to re-fire for the same item.
- The Q4 table path cannot bump every daemon and desktop epoch constant atomically.
- Enrich cannot visit a tracked advert independent of Recently-added ordering.
- Expiry requires the unknown RFx response deadline.
- Skill contract changes affect unrelated scraping selectors.

## Phase 5: Concurrency Hardening And E2E Soak

**Target beads:** `10`

### Prerequisites

- Phase 1 scheduler writers exist.
- Phase 2 worker lanes and route table exist.
- Phase 3 commercial sweep writer path exists.
- Phase 4 ProContract persistence path exists.

### Requirement IDs Covered

- Primary: `FR-G5.1`, `FR-G5.2`, `FR-G5.3`, `FR-G5.4`, `FR-G5.5`.
- Guardrail homes: `NFR-2`, `NFR-3`.

### Test IDs Covered

- `M5_concurrency_soak_g5`.
- `worker_003_source_lane_serializes_same_source`.
- Existing job-claim lease tests.
- `guard_scope_002_skill_first_boundary`.
- `guard_scope_003_single_daemon_boundary`.
- Full gate IDs `CI-ADS-001` through `CI-ADS-005`.

### Ordering

- Sequential: connection ownership audit -> WAL contention soak -> same-source overlap proof -> lease/sleep interaction.
- Sequential: commercial per-process transaction proof -> IPC mutation pressure -> no lost writes proof.
- Sequential: R1-R8 map verification -> single-writer actor escalation decision only if M5 falsifies WAL assumptions.
- Sequential: guardrail scans -> full per-file integration loop -> phase review.
- M5 runs after all writer classes are present, not as a substitute for earlier focused tests.

### R1-R8 Closure Map

| Research Risk | Phase 5 Proof Home |
|---------------|--------------------|
| R1 write contention | M5 no lost writes / no permanent `SQLITE_BUSY`, with single-writer actor escalation recorded only if falsified |
| R2 PTY starvation of tick | Scheduler/worker separation remains green through M1, `tick_007`, and M5 |
| R3 double dispatch | `tick_002` remains green under final scheduler startup fold |
| R4 arbiter accounting | `worker_005` remains green with every current `DenialReason` variant and release path |
| R5 sleep/wake and leases | Existing lease tests plus M5 prove catch-up, retry recycle, and admission ordering |
| R6 schema epoch coupling | Q4 branch proof remains green before soak closure |
| R7 same-source overlap | `worker_003` and M5 prove the second same-source procurement job defers and never runs concurrently |
| R8 heavy sweep writer pressure | M5 proves commercial sweep transactions are per-process scoped and do not starve IPC beyond `busy_timeout` |

### Gate Criteria

- M5 proves no lost writes, no permanent `SQLITE_BUSY`, and no concurrent same-source procurement scrape.
- Existing job-claim lease assertions stay green.
- `worker_003` remains green under the final worker implementation.
- R1-R8 all have named proof from subsystem tests or M5.
- `guard_scope_002_skill_first_boundary` and `guard_scope_003_single_daemon_boundary` are green.
- `CI-ADS-001..005` pass exactly as listed in the TDD and manifest.

### Roll Back If

- Workers share the serve-loop `RuntimeBootstrap.state_db` unsafely across threads.
- Commercial sweep requires one giant write transaction.
- IPC mutations starve beyond `busy_timeout`.
- Same-source procurement jobs run concurrently.
- The full per-file integration loop exposes failures hidden by the daemon bin-only test.

## Phase 6: Desktop Status Surfacing Decision Branch

**Target beads:** `6`

### Prerequisites

- Phase 1 scheduler state exists.
- Phase 2 worker lane status exists.
- Phase 4 awaiting-access active/terminal counts exist.
- Phase 5 concurrency gate passed.
- Q7 is recorded before desktop files are touched.

### Requirement IDs Covered

- Primary: `FR-G3.4`.
- Supporting proof for `FR-G4.5`.

### Test IDs Covered

- `decision_001_operator_decisions_recorded`.
- `desktop_status_001`.
- `CI-ADS-006` when Q7 selects rendered desktop UI.

### Ordering

- Sequential: record Q7 -> daemon snapshot branch -> desktop Rust bridge branch -> React rendering branch -> read-only proof.
- If Q7 selects daemon-only documentation for this batch, record the branch and keep the named extension point in artifacts.
- If Q7 selects rendered UI, run daemon Rust, desktop Rust, and Bun gates in one phase.
- Awaiting-access status reads consume Phase 4 state; this phase does not change G4 lifecycle semantics.

### Gate Criteria

- `decision_001_operator_decisions_recorded` includes Q7 before any desktop surface changes.
- `desktop_status_001` is green when Q7 selects status surfacing.
- Daemon, desktop Rust, and React surfaces stay read-only when Q7 selects rendering.
- `CI-ADS-006` passes when Q7 selects rendered desktop UI.
- `CI-ADS-001..005` remain green for Rust surfaces.

### Roll Back If

- Status surfacing requires mutating daemon state from the desktop view.
- Desktop rendering needs scheduler or awaiting-access fields not produced by earlier phases.
- Q7 is not recorded before desktop files are touched.

## Invariant Enforcement Phase Homes

| Invariant | Enforcement Phase | Mechanism Home |
|-----------|-------------------|----------------|
| Q1 is first | 0 | Decision record before runtime/context changes |
| One-shot harness exists before E2E | 0 | Clock seam and `--run-scheduler-once --tick-now` |
| No compatibility shims | 0 | Direct live-loop body refactor and `guard_scope_005` |
| Startup enqueue folds into tick | 1 | Shared tick body and `tick_002` |
| Failed slot remains reenqueueable | 1 | `tick_006` |
| Registry reload is change-triggered | 1 | Q9 branch and `tick_005` |
| Cron kinds have real handlers | 2 | `worker_001` side-effect matrix |
| Denial is not failure | 2 | `worker_002`, `worker_003`, `worker_005` |
| Commercial sweep is not desktop rerun | 3 | M2, M4, `guard_scope_004` |
| Q4 schema branch is first in G4 | 4 | `0015_*`/epoch branch or state-string guard branch |
| Awaiting-access does not refire interest | 4 | M3 step 1 |
| Skill-first scraping remains | 5 | `guard_scope_002` |
| Single-daemon scope remains | 5 | `guard_scope_003` and M5 |
| Q7 branch is recorded | 6 | `decision_001` and `desktop_status_001` branch |

## TDD Phase Homes

| TDD Surface | Phase Home | Notes |
|-------------|------------|-------|
| `decision_001_operator_decisions_recorded` | 0 + branch phases | Q1 in Phase 0; Q4/Q5/Q6/Q7/Q8/Q9/E1 before their dependent phases |
| `tick_001`, `tick_002`, `tick_003`, `tick_005`, `tick_006`, `tick_007` | 1 | Scheduler tick, startup fold, catch-up, reload, live serve |
| `tick_004` | 0 | Clock seam foundation |
| `worker_001..005` | 2 | Worker route table, admission, lanes, and arbiter |
| `M1_end_to_end_autonomy_g1_g2` | 1 + 2 | Scheduler enqueue in Phase 1; worker drain in Phase 2 |
| `M2_commercial_stage_autonomy_g6` | 3 | Commercial sweep autonomy |
| `M3_step_001..004`, M3 | 4 | ProContract lifecycle |
| `M4_manual_path_regression_and_trigger_ingestion` | 0 + 2 + 3 | Manual `--ingest` foundation, trigger ingestion, rerun coexistence |
| `M5_concurrency_soak_g5` | 5 | Full writer and lane contention |
| Q4 schema branch IDs | 4 | Dedicated table or state-string branch |
| `shadow_health_auth_001` | 2 | Q6 branch |
| `desktop_status_001` | 6 | Q7 branch |
| `commercial_nudge_001` | 3 | Q8-residual branch |
| `trigger_enrichment_001` | 2 | E1 branch |
| Guard scope IDs | 0, 3, 4, 5 | Enforcement homes by guardrail |
| `CI-ADS-001..005` | 0 through 5 | Rust quality gate for touched Rust phases |
| `CI-ADS-006` | 6 | Q7-selected desktop gate |

## Test File Status

Verified against `crates/daemon/tests/`: the three phase-owned suites below are net-new; the existing impact files named below are present and must be preserved or extended in place.

| Test File Or Evidence | Status | Phase Home |
|-----------------------|--------|------------|
| `crates/daemon/tests/scheduler_tick.rs` | net-new | 1 |
| `crates/daemon/tests/worker_pump.rs` | net-new | 2 |
| `crates/daemon/tests/procontract_resumption.rs` | net-new | 4 |
| `crates/daemon/tests/cron_scheduler.rs` impacts | existing, extend | 3 |
| `crates/daemon/tests/job_claim.rs` impacts | existing, extend | 2 |
| `crates/daemon/tests/agent_procurement_ingestion.rs` | existing, preserve | 0 |
| `crates/daemon/tests/agent_contracts_finder_ingestion.rs` | existing, preserve/extend | 0 |
| `crates/daemon/tests/sqlite_schema.rs` Q4 branch | existing, conditional extend | 4 |
| `skills/procurement-ingestion/procontract-enrich-tender/` contract | existing, extend | 4 |
| Desktop/IPC `trigger_ingestion` modeled tests | modeled from existing IPC tests | 2 |
| Desktop status surfacing tests | Q7-gated | 6 |

## Literal TDD ID Routing

| Phase | Literal Test IDs |
|-------|------------------|
| 0 | `decision_001_operator_decisions_recorded`, `tick_004_clock_seam`, `impact_ingestion_regression_001_fts_unchanged`, `impact_contracts_finder_existing_contracts_unchanged`, `impact_contracts_finder_live_loop_regression_001`, `guard_scope_005_no_compat_shims` |
| 1 | `tick_001_due_slot_enqueues_exactly_one_row`, `tick_002_repeated_ticks_same_slot_idempotent`, `tick_003_long_gap_catchup`, `tick_005_registry_reload_on_generation_bump`, `tick_006_failed_slot_reenqueueable`, `tick_007_serve_scheduler_thread_spawn_poll_terminate`, M1 enqueue assertions |
| 2 | `worker_001_routes_each_cron_kind`, `worker_002_defer_on_denial_no_attempt_bump`, `worker_003_source_lane_serializes_same_source`, `worker_004_head_of_line`, `worker_005_arbiter_quota_respected`, `impact_job_claim_new_kinds_claimable`, `shadow_health_auth_001`, `trigger_enrichment_001`, M1 drain assertions, M4 `trigger_ingestion` assertions |
| 3 | `impact_cron_scheduler_001_anchor_set_includes_0700_commercial_sweep`, `impact_cron_scheduler_006_catchup_order_includes_commercial_sweep_key`, M2, M4 rerun coexistence assertions, `commercial_nudge_001`, `guard_scope_004_no_remote_http_trigger` |
| 4 | `M3_step_001_register_creates_awaiting_access_without_refire`, `M3_step_002_enrich_keep_and_backfill`, `M3_step_003_enrich_clear_via_worklist`, `M3_step_004_staleness_expire`, `M3_procontract_resumption_full_lifecycle_g4`, `skill_contract_001_awaiting_access_targets`, `schema_0015_procontract_pending_enrichment`, `schema_state_string_refire_guard`, `impact_sqlite_schema_q4_table`, `impact_sqlite_schema_q4_state_string`, `guard_scope_001_skill_selector_boundary` |
| 5 | `M5_concurrency_soak_g5`, existing job-claim lease tests, `guard_scope_002_skill_first_boundary`, `guard_scope_003_single_daemon_boundary`, `CI-ADS-001`, `CI-ADS-002`, `CI-ADS-003`, `CI-ADS-004`, `CI-ADS-005` |
| 6 | `desktop_status_001`, `CI-ADS-006` when Q7 selects rendered desktop UI |

## Requirement Routing

Each functional requirement below has exactly one primary phase home. Supporting prerequisites may appear earlier, but feature ownership lives in this table.

| Requirement IDs | Primary Phase |
|-----------------|---------------|
| `FR-G1.2`, `FR-RT.1`, `FR-RT.2`, `FR-RT.3`, `FR-RT.4`, `FR-G3.1` | 0 |
| `FR-G1.1`, `FR-G1.3`, `FR-G1.4`, `FR-G1.5`, `FR-G1.6` | 1 |
| `FR-G2.1`, `FR-G2.2`, `FR-G2.3`, `FR-G2.4`, `FR-G3.2`, `FR-G3.5` | 2 |
| `FR-G3.3`, `FR-G6.1`, `FR-G6.2`, `FR-G6.3`, `FR-G6.4` | 3 |
| `FR-G4.1`, `FR-G4.2`, `FR-G4.3`, `FR-G4.4`, `FR-G4.5` | 4 |
| `FR-G5.1`, `FR-G5.2`, `FR-G5.3`, `FR-G5.4`, `FR-G5.5` | 5 |
| `FR-G3.4` | 6 |

## Literal Requirement ID Routing

| Requirement | Primary Phase | Gate IDs |
|-------------|---------------|----------|
| `FR-G1.1` | 1 | `tick_001`, `tick_003`, `tick_007`, M1, M5 |
| `FR-G1.2` | 0 | `tick_004`, M1, M2 |
| `FR-G1.3` | 1 | `tick_002`, `tick_006`, `impact_cron_scheduler_001` |
| `FR-G1.4` | 1 | `tick_005`, `decision_001` |
| `FR-G1.5` | 1 | `tick_003`, M1, M5 |
| `FR-G1.6` | 1 | existing job-claim ordering, `tick_003`, M1 |
| `FR-G2.1` | 2 | `worker_004`, `worker_005`, M1, M2 |
| `FR-G2.2` | 2 | `worker_002`, `worker_003`, `worker_005` |
| `FR-G2.3` | 2 | `worker_001` |
| `FR-G2.4` | 2 | `decision_001`, `worker_001`, `shadow_health_auth_001` |
| `FR-RT.1` | 0 | `impact_ingestion_regression_001`, Contracts Finder regressions, M3, M4 |
| `FR-RT.2` | 0 | `worker_001`, M3, M4 |
| `FR-RT.3` | 0 | `worker_001`, M3 |
| `FR-RT.4` | 0 | `decision_001`, `worker_002`, M5 |
| `FR-G3.1` | 0 | M4, ingestion regressions |
| `FR-G3.2` | 2 | M4 |
| `FR-G3.3` | 3 | M2, M4, M5 |
| `FR-G3.4` | 6 | `decision_001`, `desktop_status_001` |
| `FR-G3.5` | 2 | `decision_001`, M4, `trigger_enrichment_001` |
| `FR-G4.1` | 4 | `M3_step_001`, Q4 schema branch |
| `FR-G4.2` | 4 | `M3_step_001..004` |
| `FR-G4.3` | 4 | `M3_step_002`, `M3_step_003`, `skill_contract_001` |
| `FR-G4.4` | 4 | `skill_contract_001`, M3 |
| `FR-G4.5` | 4 | `M3_step_004`, `desktop_status_001` |
| `FR-G5.1` | 5 | M5 |
| `FR-G5.2` | 5 | M5 |
| `FR-G5.3` | 5 | `worker_003`, M5 |
| `FR-G5.4` | 5 | existing lease tests, M5 |
| `FR-G5.5` | 5 | subsystem proofs plus M5 and Q4 branch |
| `FR-G6.1` | 3 | cron scheduler impacts, `tick_001`, `tick_002`, M2 |
| `FR-G6.2` | 3 | `worker_001`, M2 |
| `FR-G6.3` | 3 | M2, M4 |
| `FR-G6.4` | 3 | `decision_001`, M2, `commercial_nudge_001` |
| `NFR-1` | 4 | `guard_scope_001`, M3 |
| `NFR-2` | 5 | `guard_scope_002`, M1-M5 |
| `NFR-3` | 5 | `guard_scope_003`, M5 |
| `NFR-4` | 3 | `guard_scope_004`, M4 |
| `NFR-5` | 0 | `guard_scope_005`, M4 |

## Completeness Mapping

| Spec Section | Phase | Notes |
|-------------|-------|-------|
| Section 1.1 Mission | 0 through 6 | Runtime autonomy closes across scheduler, worker, G4, G5, G6, and Q7 branch |
| Section 1.2 Committed Goals | 1 through 5 | G1-G6 route to owning phases; G3 spans Phase 0/2/3/6 |
| Section 1.3 Recorded Decisions | 0 + 1 + 2 + 3 | Clock seam in Phase 0; Q2 in Phase 1; Q3 in Phase 2; Q8 principal in Phase 3 |
| Section 1.4 Operator Decisions Needed | 0 through 6 | Each decision routes to the first dependent phase |
| Section 1.5 Guardrail Non-Goals | 0 + 3 + 4 + 5 | Guard IDs have explicit enforcement homes |
| Section 2.1 G1 Scheduler Tick | 0 + 1 | Clock seam in Phase 0; recurring tick, dedup, reload, catch-up in Phase 1 |
| Section 2.2 G2 Worker Pump | 2 | Worker lanes, admission, handlers, and shadow branch |
| Section 2.3 Cross-Cutting Live-Loop Refactor | 0 | Q1, body extraction, route keys, runtime parameters |
| Section 2.4 G3 Manual Paths/Desktop Interaction | 0 + 2 + 3 + 6 | Manual CLI, trigger ingestion, rerun coexistence, Q7/E1 branches |
| Section 2.5 G4 ProContract Resumption | 4 | Q4/Q5 branch, lifecycle, work-list, skill contract |
| Section 2.6 G5 Concurrency Correctness | 5 | State ownership, WAL, same-source, lease/sleep, R1-R8 |
| Section 2.7 G6 Commercial Sweep | 3 | Schedule, coverage body, no rerun reuse, Q8-residual branch |
| Section 3 User Stories | 0 through 6 | Routed through owning functional requirements |
| Section 4 E2E Test Scenarios | 1 through 5 | M1/M2/M3/M4/M5 close by owning phase |
| Section 5 Dependencies | 0 through 6 | Consumed by phase prerequisites |
| Section 6 Risks And Mitigations | 0 through 5 | Risk controls route to scheduler, worker, G4, G5, and G6 phases |
| Section 7 Success-Criteria Coverage Matrix | 0 through 5 | Covered by TDD phase homes |
| Section 8 Test Target Inventory | 0 through 6 | Test files route to first owning phase and close by full gate |
| Section 9 Quality Gate | 5 + 6 | Full Rust gate in Phase 5; desktop gate in Phase 6 when selected |
| Section 10 Operator Question Prompts | 0 through 6 | Decision recording precedes dependent implementation |

Coverage: complete. All `33` functional/runtime requirement IDs have exactly one primary phase home, all `5` guardrail IDs have enforcement homes, and every operator decision has a phase branch.

## Dependency Diagram

```text
+--------------------------------------------------+
| Phase 0: Foundations, Q1, Clock, Refactor         |
| Q1 -> clock seam -> one-shot -> source bodies     |
+------------------------+-------------------------+
                         |
                         v
+--------------------------------------------------+
| Phase 1: Scheduler Tick And Registry Reload       |
| recurring tick, startup fold, catch-up, Q9        |
+------------------------+-------------------------+
                         |
                         v
+--------------------------------------------------+
| Phase 2: Worker Pump And Cron Kind Routing        |
| two lanes, admission, handlers, trigger ingestion |
+------------------------+-------------------------+
                         |
          +--------------+--------------+
          |                             |
          v                             v
+-------------------------------+  +--------------------------------+
| Phase 3: Commercial Sweep     |  | Phase 4: ProContract Resumption |
| G6 07:00 coverage autonomy    |  | G4 awaiting-access lifecycle    |
+----------------+--------------+  +----------------+---------------+
                 |                                  |
                 +----------------+-----------------+
                                  |
                                  v
+--------------------------------------------------+
| Phase 5: Concurrency Hardening And E2E Soak       |
| R1-R8, WAL pressure, M5, full Rust gate           |
+------------------------+-------------------------+
                         |
                         v
+--------------------------------------------------+
| Phase 6: Desktop Status Surfacing Decision Branch |
+--------------------------------------------------+
```

Internal dependency highlights:

```text
Q1 -> live-loop body signatures -> worker runtime -> M5
Clock seam -> one-shot entrypoint -> M1/M2 -> phase gates
Scheduler tick -> worker drain -> commercial sweep and ProContract scheduled enrich
Worker lanes -> G6 coverage body and G4 enrich route
Q4 schema branch -> ProContract lifecycle -> Q7 awaiting-access count
All writer surfaces -> M5 -> desktop read-only status branch
```

## Gates Summary

| Phase | Gate | Roll Back If |
|------|------|-------------|
| 0 | `decision_001`; `tick_004`; one-shot harness; ingestion regressions; `guard_scope_005`; `CI-ADS-001..005` | Q1 is late, one-shot bypasses worker paths, or source bodies break manual CLI |
| 1 | `tick_001`, `tick_002`, `tick_003`, `tick_005`, `tick_006`, `tick_007`; job-claim ordering; M1 enqueue | scheduler shares serve state, startup and interval diverge, or reload requires full per-tick rebuild |
| 2 | `worker_001..005`; new-kind claimability; Q6/E1 branches; M1 drain; M4 trigger ingestion | any cron kind is hollow/unsupported, denial becomes failure, or lane separation fails without fallback |
| 3 | cron scheduler impacts; `worker_001` commercial subcase; M2; M4/M5 coexistence; Q8 branch; `guard_scope_004` | sweep reuses desktop rerun, 07:00 key cannot dedup, or nudge blurs source/commercial boundaries |
| 4 | Q4 schema branch; M3 steps 1-4; full M3; skill contract; Q5 expiry; `guard_scope_001` | awaiting access refires interest, schema epoch cannot be atomic, or direct advert visits cannot be proven |
| 5 | M5; lease tests; R1-R8 proof; guard scope IDs; `CI-ADS-001..005` full loop | WAL assumptions fail without escalation, same-source overlap occurs, or bin-only tests hide failures |
| 6 | Q7 decision; `desktop_status_001`; `CI-ADS-006` when rendered UI selected | desktop status mutates state, needs missing upstream fields, or Q7 is not recorded first |

## Feature Surface Routing

Every manifest `feature_surface` item has exactly one phase home below. Compound manifest bullets are split into route atoms only where a single bullet names multiple concrete surfaces.

| ID | Manifest Item | Phase Home |
|----|---------------|------------|
| S1 | scheduler background thread | 1 |
| S2 | build scheduler and seed history once | 1 |
| S3 | per-tick dispatch, enqueue, and `record_enqueued` | 1 |
| S4 | clock-injection seam | 0 |
| S5 | one-shot autonomous entrypoint | 0 |
| S6 | Q9 registry reload trigger | 1 |
| S7 | fold startup enqueue into tick path | 1 |
| S8 | sleep/wake catch-up | 1 |
| S9 | tick thread separate from worker | 1 |
| W1 | continuous worker claim/route/execute loop | 2 |
| W2 | two worker lanes by job class | 2 |
| W3 | worker idle backoff | 2 |
| W4 | ResourceArbiter quota respect | 2 |
| W5 | defer-on-denial backoff | 2 |
| W6 | local execution independent of remote writer lease | 2 |
| R1 | source body extraction | 0 |
| R2 | thin `--ingest` wrappers | 0 |
| R3 | `(source_id, skill_id)` source routing | 0 |
| R4 | execution-time parameter derivation | 0 |
| R5 | commercial stages excluded from `sync_source` routing | 0 |
| H1 | `source_health` handler | 2 |
| H2 | `auth_refresh` handler | 2 |
| H3 | `validation_cross_model_audit` handler | 2 |
| H4 | `validation_calibration_readiness_sweep` handler | 2 |
| H5 | route table covers all cron kinds | 2 |
| C1 | `commercial_sweep` cron kind and 07:00 slot | 3 |
| C2 | coverage-driven commercial body | 3 |
| C3 | net-gate pure DB plus skill-backed sub-pass shape | 3 |
| C4 | manual rerun reconciliation | 3 |
| C5 | in-process coverage lane execution | 3 |
| C6 | commercial two-layer dedup | 3 |
| C7 | Q8-residual chained nudge branch | 3 |
| P1 | Q4 awaiting-access model | 4 |
| P2 | create row on register-interest success | 4 |
| P3 | create/keep/clear/backfill/expire lifecycle | 4 |
| P4 | observation count separate from job attempts | 4 |
| P5 | daemon-tracked enrich sweep | 4 |
| P6 | ProContract enrich skill contract change | 4 |
| P7 | Q5 staleness expiry | 4 |
| K1 | Q1 worker execution context | 0 |
| K2 | WAL single-writer discipline | 5 |
| K3 | same-source serialization | 5 |
| K4 | single-writer actor escalation path | 5 |
| K5 | sleep/wake claim and lease interaction | 5 |
| U1 | Q7 desktop status surfacing branch | 6 |
| T1 | new `scheduler_tick.rs` suite | 1 |
| T2 | new `worker_pump.rs` suite | 2 |
| T3 | new `procontract_resumption.rs` suite | 4 |
| T4 | M1 end-to-end autonomy | 2 |
| T5 | M2 commercial autonomy | 3 |
| T6 | M3 ProContract lifecycle | 4 |
| T7 | M4 manual path and live buttons | 3 |
| T8 | M5 concurrency soak | 5 |
| T9 | `cron_scheduler.rs` existing-test impacts | 3 |
| T10 | `job_claim.rs` existing-test impacts | 2 |
| T11 | ingestion regression existing-test impacts | 0 |
| T12 | Q4 schema migration branch tests | 4 |

## Scope-Preservation Check

- Enumerated source set: `33` functional/runtime requirement IDs plus `5` guardrail non-goal IDs.
- Requirement routing contains all `33 / 33` functional/runtime IDs with exactly one primary phase home.
- Guardrail routing contains all `5 / 5` IDs with explicit enforcement homes.
- Manifest `feature_surface` routing contains every route atom from scheduler, worker, refactor, handler, commercial, ProContract, concurrency, desktop, and test surfaces.
- Manifest count check: current bullet counts are `9 / 6 / 5 / 5 / 7 / 7 / 5 / 1 / 4` across scheduler, worker, handlers, refactor, commercial, ProContract, concurrency, desktop, and tests; compound test bullets expand to `T1..T12`.
- Phase balance check: target bead distribution is `12 / 10 / 14 / 11 / 13 / 10 / 6`; Phase 3 and Phase 4 are not thinner than scheduler-tick work.
- Operator-gated items are named in phase homes: Q6 in Phase 2, Q7 in Phase 6, Q8-residual in Phase 3, and E1/enrich-process routing in Phase 2.
- G4 depth check: Phase 4 owns schema branch, lifecycle, work-list sweep, skill contract, expiry, M3, and skill canaries.
- G6 depth check: Phase 3 owns cron definition, scheduler impacts, coverage body, rerun coexistence, Q8 branch, and M2.
- G5 depth check: Phase 5 owns full WAL/lane/lease/IPC writer soak, R1-R8 closure, and full Rust gate.
- Q7 depth check: Phase 6 owns daemon snapshot, desktop Rust bridge, React rendering branch, read-only proof, and Bun gate when selected.
- Clock harness check: Phase 0 creates `--run-scheduler-once --tick-now` before every E2E phase consumes it.
- Q1-first check: no live-loop or worker-runtime implementation starts before Q1 is recorded.
- Research sequencing check: G4 lands after G1/G2, matching research section 4.4.
- Slot check: the strategy adds only the 07:00 `commercial_sweep` slot and keeps existing source schedules unchanged.
- Existing-test impact check: `cron_scheduler.rs` impacts land with G6; `job_claim.rs` impacts land with G1/G2; ingestion regressions land with Phase 0; Q4 schema impacts land with G4.
- Test-file reality check: `scheduler_tick.rs`, `worker_pump.rs`, and `procontract_resumption.rs` are net-new; named existing daemon integration files already exist and remain in-place targets.
- Full-gate check: Phase 5 requires the per-file daemon integration loop; `cargo test -p daemon --bin daemon` alone is never a green claim.
- Scope-cut phrase scan: clean.
