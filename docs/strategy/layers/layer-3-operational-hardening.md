# Layer 3 — Operational Hardening

> **Authority:** This document is the execution-grade specification for Layer 3. An implementation agent should be able to build this layer from this document alone without reopening the research document.
>
> **Milestone:** M3
> **Dependencies:** Layer 0 (intelligence pipeline, gold set for validator calibration), Layer 1 (daemon runtime, agent adapter), Layer 2 (desktop shell, pipeline data model)
> **Exit criteria:** System runs unattended for 5 consecutive days with >=95% skill success rate and no manual intervention.

> **Implementation Status (as of 2026-05-02):** Layer 3 is **BUILT** around Stages 1-14 of the daemon pipeline, multi-source ingestion, Stage 11 landed-cost hardening, and Stage 14 operator decision/clarification flow. See `docs/pipeline/stage-11-hardening-order-quantity-validation-manifest.md`, `docs/pipeline/stage-11-hardening-cdp-resilience-validation-manifest.md`, `docs/pipeline/stage-14-validation-manifest.md`, `docs/pipeline/stage-14-phase-2-validation-manifest.md`, and `docs/pipeline/multi-source-ingestion-validation-manifest.md` for per-stage evidence. This doc has been annotated to reflect actual implementation choices — search for `[IMPL]` markers. Where actual implementation diverged from the pre-build spec, the rationale is in a `> **[IMPL] ⚠ Deviation:**` callout.

---

## 1. Overview & Daemon Additions

### 1.1 Layer Narrative

Daily automation without babysitting. The daemon gains cron scheduling, batched ingestion, relation resolution, PTY hardening, resource management, and the worker-validator system in dry-run mode. The system runs unattended for 5 consecutive days. This layer transforms the single-operator tool from Layer 2 into a production-grade pipeline.

**Why this layer exists:** Layer 2 proved the value proposition — a real operator finds genuine opportunities. Layer 3 removes the need for the operator to manually trigger every run. It also introduces the worker-validator system (in dry-run mode) to calibrate semantic quality checking before enforcement in Layer 4.

**If we never build Layer 4:** A single operator has a fully automated, self-monitoring pipeline that runs daily, handles relation resolution, manages resources, and validates output quality via dry-run semantic checking. No multi-user visibility, but the core automation works.

### 1.2 Capability Delivered

System runs unattended for 5 consecutive days with >=95% skill success rate:

- **Cron-scheduled ingestion** — FTS + Contracts Finder runs on schedule without manual triggers
- **Resource management** — typed quotas prevent resource exhaustion (browser slots, agent slots, source lanes)
- **PTY hardening** — fd hygiene, EIO handling, terminal geometry stability for reliable sessions
- **Relation resolution** — notices classified as merged/related/possible_duplicate/separate
- **Source health monitoring** — canary probes detect source changes before production jobs; circuit breaker on repeated failures
- **Skill promotion flow** — draft -> candidate -> canary -> shadow -> stable -> rollback
- **Worker-validator dry-run** — run manifests generated, base validator renders verdicts, verdicts logged without pipeline enforcement

> **[IMPL] ⚠ Deviation:** Actual implementation (2026-05-02): L3 is built as a staged production pipeline and hardening layer, but the evidence is per-stage validation rather than a single documented five-day unattended run. Current hardening includes scheduler/resource/source/validation modules plus Stages 1-14 in `crates/daemon/src/main.rs:1259`, Stage 11 landed-cost hardening, and Stage 14 operator decision/clarification flow; use the Stage 11/14 and multi-source validation manifests as the implementation snapshot.

> **[IMPL] ⚠ Deviation:** Actual implementation (2026-05-02): Stage 11 added landed-cost order-quantity basis hardening that is not reflected in this pre-build L3 narrative. `OrderQuantityBasis` now includes `BasisUndetermined` and operator-override paths in `crates/daemon/src/main.rs:3473`, `crates/daemon/src/main.rs:4058`, and `crates/daemon/src/main.rs:4197`; `docs/pipeline/stage-11-hardening-order-quantity-validation-manifest.md` documents the zero-margin calibration and warning behavior.

> **[IMPL] ✓ As designed (2026-06-12 — autonomous daemon scheduling):** The "cron-scheduled ingestion … without manual triggers" claim is now literally true while the daemon stays open. Two runtime additions close the gap that previously made schedules fire only at process start: (1) a continuous **`autonomous-scheduler-tick`** background thread (~30s, injected-clock seam) re-evaluates due schedules while serving, and (2) an always-on **two-lane worker pump** (PTY/browser lane + coverage/deterministic lane) drains `workflow_jobs` and routes every cron kind. A new **`commercial_sweep`** cron kind (cron `0 0 7 * * *`, 07:00 UTC) advances the commercial stages autonomously, and ProContract awaiting-access items are tracked durably in a dedicated table while `interest_registration.state` stays `'completed'` (§1.11). This shipped at **schema epoch 11** (`migrations/0015_procontract_pending_enrichment.sql`). See §1.5 (rewritten trigger model) and §1.11 (ProContract resumption). The daemon no longer needs a quit-and-reopen to pick up schedules.

**Traceability rule:** Every testable Layer 3 behavior below is identified by an inline `L3-FR-*` marker using the stable pattern `L3-FR-{section}.{subsection}.{item}`. Later verification artifacts may reference these requirement IDs without restating the full contract.

### 1.3 Layer 3 Modules (12 surfaces)

| File | Module | Responsibility |
|------|--------|---------------|
| `scheduler/cron.rs` | Cron Scheduler | Schedule and enqueue agent work based on cron definitions. Runs locally in daemon. Binds explicit skill id + version at enqueue time (no late-bound resolution). Catchup sweeps on startup for overdue jobs. |
| `sources/registry.rs` | Source Registry | Typed source definitions with version tracking, skill binding, schedule, and health/cooldown status. Static definitions in local SQLite `sources`; mutable state in `source_runtime_state`. |
| `jobs/claim.rs` | Job Claimer | Concurrent job claiming with lease management. Multiple worker threads can claim jobs atomically. Lease expiry prevents stuck claims. |
| `sessions/reaper.rs` | Session Reaper | Existing Layer 1 reaper extended for L3 descendant process-tree cleanup after session completion/failure. Kill escalation: in-band interrupt -> SIGTERM group -> grace period -> SIGKILL group -> orphan sweep. Records cleanup verdict. On daemon startup, sweeps for orphaned sessions from prior runs. |
| `pty/hardening.rs` | PTY Hardening | fd hygiene (aggressively close inherited fds before exec), EIO handling (treat EIO on PTY master read after slave closes as normal termination), terminal geometry (fix PTY size for headless runs, e.g., 120x40). |
| `agents/arbiter.rs` | Resource Arbiter | Admit agent jobs using typed resource budgets. Three quota types with admission priority ordering. |
| `db/checkpoints.rs` | Checkpoint Store | Checkpoint persistence for partial progress. Validates checkpoint schema, stores to artifact directory, updates `workflow_jobs` with checkpoint reference. |
| `tracing/structured.rs` | Structured Tracing | Control events (spawn, signal, exit, reap, cleanup verdict) + protocol events (vendor JSONL/JSON parsed into daemon events, validated checkpoints). These drive workflow state. |
| `tracing/forensic.rs` | Forensic Capture | Raw byte capture (PTY bytes or stdout/stderr) as compressed immutable artifacts. These are for debugging and replay, not control flow. |
| `validation/policy.rs` | Validation Policy Engine | Determine whether a completed job requires semantic validation based on policy mode, promotion stage, trigger conditions, and risk factors. |
| `validation/executor.rs` | Validation Executor | Spawn validator sub-jobs via Job Runner. Compose validator kickoff prompt with run manifest, result contract sample, runtime rubric, OQG signals, and drift deltas. |
| `validation/manifest_writer.rs` | Run Manifest Writer | Produce hybrid-authored run manifest after every skill run. Agent narrative + daemon evidence appendix. Fallback machine-authored manifest if agent fails to narrate. |

Existing repo surfaces intentionally reused rather than forked: `jobs/runner.rs` remains the authoritative enqueue/order/idempotency boundary; `agents/artifacts.rs` remains the authoritative run-manifest struct/size-budget surface; `agents/contracts.rs` remains the authoritative OQG/checkpoint/`ValidationDisposition` surface; `pty/health.rs` remains the host/session health-code surface; `db/sqlite.rs` remains the seeded source/runtime-state entry point; and `ipc/command_router.rs` remains the manual-trigger command boundary. Layer 3 adds modules where needed, but it does not silently create duplicate contracts for those existing types.

> **[IMPL] ✓ As designed** — All 12 named L3 module surfaces exist in the daemon tree: `crates/daemon/src/scheduler/cron.rs`, `crates/daemon/src/sources/registry.rs`, `crates/daemon/src/jobs/claim.rs`, `crates/daemon/src/sessions/reaper.rs`, `crates/daemon/src/pty/hardening.rs`, `crates/daemon/src/agents/arbiter.rs`, `crates/daemon/src/db/checkpoints.rs`, `crates/daemon/src/tracing/structured.rs`, `crates/daemon/src/tracing/forensic.rs`, `crates/daemon/src/validation/policy.rs`, `crates/daemon/src/validation/executor.rs`, and `crates/daemon/src/validation/manifest_writer.rs`.

> **[IMPL] ✓ As designed (2026-06-12 — autonomous daemon scheduling):** The serve loop is no longer a passive IPC-only listener. Three runtime surfaces were added on top of the 12 above to make scheduling autonomous (no new L3-FR contracts — these realize the §1.5 "runs locally in daemon" intent):
> - `scheduler/tick.rs` — the pure scheduler-tick body `run_scheduler_tick(scheduler, registry, state_db, now)` (injected-clock seam, no inline `Utc::now()`), driven on interval by a `SchedulerTickThread` (thread `autonomous-scheduler-tick`, ~30s, env `DAEMON_SCHEDULER_TICK_INTERVAL_SECS`) with its own `StateDb` connection.
> - Two `AutonomousWorkerThread` lanes (PTY/browser + coverage/deterministic) spawned in the serve loop, each with its own `StateDb` connection, draining `workflow_jobs` and routing every cron kind (see §1.5). Defer-on-denial is an **in-memory** `DenialCooldowns` (no schema column) that returns a resource-denied job to `pending` without costing an attempt.
> - `scheduler/cron.rs` now also defines the synthetic **`commercial_sweep`** slot (source id `commercial-sweep`, skill `pipeline/commercial-sweep`, `0 0 7 * * *`) alongside the existing source schedules. The startup-only `enqueue_startup_cron_jobs` path was **deleted** — cold-start catch-up now runs the tick body once at thread start.

**L3-FR-1.3.1 — `scheduler/cron.rs` owns daemon-local cron scheduling**
- AC1: Layer 3 includes `scheduler/cron.rs`.
- AC2: The module schedules and enqueues agent work from cron definitions inside the daemon rather than via external schedulers.
- AC3: The module preserves explicit skill binding and startup catchup behavior as defined in §1.5.

**L3-FR-1.3.2 — `sources/registry.rs` owns typed source definitions and mutable runtime binding**
- AC1: Layer 3 includes `sources/registry.rs`.
- AC2: The module stores typed source definitions with version tracking, skill binding, schedules, and health/cooldown status.
- AC3: Static source definitions remain rooted in `sources`, while mutable runtime state is read from and written to `source_runtime_state`.

**L3-FR-1.3.3 — `jobs/claim.rs` provides atomic concurrent job claiming**
- AC1: Layer 3 includes `jobs/claim.rs`.
- AC2: Concurrent worker threads can claim jobs without double-claiming the same runnable work item.
- AC3: Lease expiry returns stuck claims to the runnable pool without inventing duplicate authoritative work.
- AC4: The concurrent claimer preserves the current `JobRunner::claim_next_job` eligibility/order surface: only jobs in `pending` or `retry` status are claimable, ordered by `COALESCE(scheduled_for, created_at), created_at`.
- AC5: A successful claim records runner-owned attempt metadata before execution begins, including `lease_owner`, `started_at`, and `attempt_count = attempt_count + 1`; Layer 3 adds atomicity/lease-expiry without changing those semantics.
- AC6: `jobs/claim.rs` becomes the single SQL claim/lease surface used by `JobRunner`; Layer 3 does not keep a competing ad hoc claimant inside `jobs/runner.rs`.

**L3-FR-1.3.4 — `sessions/reaper.rs` owns post-session cleanup and orphan recovery**
- AC1: Layer 3 extends the existing `sessions/reaper.rs` module rather than introducing a second cleanup-verdict implementation elsewhere.
- AC2: The reaper applies group-based kill escalation in the order in-band interrupt -> `SIGTERM` -> grace period -> `SIGKILL` -> orphan sweep.
- AC3: Daemon startup sweeps for orphaned sessions left by prior runs and records a cleanup verdict.

**L3-FR-1.3.5 — `pty/hardening.rs` owns PTY-specific reliability protections**
- AC1: Layer 3 includes `pty/hardening.rs`.
- AC2: The module implements fd hygiene, PTY `EIO` handling, fixed geometry for headless runs, and signal-as-wakeup wait draining.
- AC3: PTY hardening changes terminal/runtime reliability only and does not redefine business pipeline semantics.

**L3-FR-1.3.6 — `agents/arbiter.rs` admits work through typed resource budgets**
- AC1: Layer 3 includes `agents/arbiter.rs`.
- AC2: Admission decisions consume typed quota budgets rather than one global concurrency counter.
- AC3: Admission outcomes are compatible with ordered priorities and source-level suppression from §§1.4-1.5.

**L3-FR-1.3.7 — `db/checkpoints.rs` persists validated checkpoint state**
- AC1: Layer 3 includes `db/checkpoints.rs`.
- AC2: The module validates checkpoint payloads before persisting them.
- AC3: Persisted checkpoints update `workflow_jobs` with a checkpoint reference and keep artifact-store linkage explicit.
- AC4: Checkpoint persistence preserves the existing `persist_validated_checkpoint` sequence rules in `agents/contracts.rs`; Layer 3 moves storage/path handling into `db/checkpoints.rs` without redefining checkpoint validity.

**L3-FR-1.3.8 — `tracing/structured.rs` records authoritative control and protocol events**
- AC1: Layer 3 includes `tracing/structured.rs`.
- AC2: The module records control events such as spawn, signal, exit, reap, and cleanup verdict.
- AC3: The module records protocol events such as parsed vendor events and validated checkpoints, and these events remain the workflow-driving surface.

**L3-FR-1.3.9 — `tracing/forensic.rs` records immutable raw session evidence**
- AC1: Layer 3 includes `tracing/forensic.rs`.
- AC2: The module captures raw PTY bytes or contract-mode stdout/stderr as compressed immutable artifacts.
- AC3: Forensic capture exists for debugging and replay, not as an authoritative control-flow input.

**L3-FR-1.3.10 — `validation/policy.rs` decides validation eligibility from declared policy and runtime risk**
- AC1: Layer 3 includes `validation/policy.rs`.
- AC2: The module evaluates policy mode, promotion stage, trigger conditions, and risk factors to decide whether validation is required.
- AC3: Policy changes come from metadata/configuration rather than code changes for each skill.

**L3-FR-1.3.11 — `validation/executor.rs` runs validator sub-jobs through the existing job runner**
- AC1: Layer 3 includes `validation/executor.rs`.
- AC2: The module spawns validator sub-jobs via the Job Runner and links them to worker jobs.
- AC3: The module composes validator inputs from the run manifest, result sample, rubric, OQG signals, and drift deltas.

**L3-FR-1.3.12 — `validation/manifest_writer.rs` produces hybrid-authored run manifests for every skill run**
- AC1: Layer 3 includes `validation/manifest_writer.rs`.
- AC2: The module writes agent narrative content plus a daemon-authored evidence appendix after every skill run.
- AC3: If the agent does not narrate, the module still emits a machine-authored fallback manifest.
- AC4: The writer extends the current `RunManifestFrontmatter` and `DaemonEvidenceAppendix` contract in `agents/artifacts.rs`; Layer 3 additive fields layer onto that surface instead of forking a second manifest format.

### 1.4 Resource Arbiter — Typed Quota Model

Three quota types with ordered admission priority:

| Quota Type | What It Limits | MVP Default |
|------------|---------------|-------------|
| `browser_slots` | Hard cap — each browser-backed session consumes a slot | 2 |
| `agent_slots` | Soft cap — total concurrent adapter-mediated sessions | 4 |
| `source_lanes` | Rate cap — per-source concurrency (prevent overwhelming a single target) | Procurement: 1/source, Supplier: 2/source |

**Admission priority order:**
1. Procurement ingestion (highest — data freshness)
2. Auth refresh / health probes (keeps sources accessible)
3. Qualification (pipeline progression)
3b. Document semantic extraction (pipeline progression — feeds lot/product extraction)
4. Supplier enrichment (most sessions, most resource-intensive)
5. Manifest generation (lightweight)
6. Bid automation (lowest — rare, operator-initiated)

**Note on extraction job resources:** `extract_document_structure` jobs do NOT consume `browser_slots` or `agent_slots` — they run as daemon-internal subprocesses (Docling). Only `extract_document_semantics` jobs consume `agent_slots` (1 per semantic overlay, `headless_contract` mode). Structural extraction is therefore not subject to arbiter admission.

Skill-level `max_concurrency` can lower these ceilings but never raise them.

> **[IMPL] ✓ As designed** — Typed resource admission is implemented by `ResourceArbiter` in `crates/daemon/src/agents/arbiter.rs:152`, initialized in daemon runtime state at `crates/daemon/src/main.rs:2172`, and covered by `crates/daemon/tests/resource_arbiter.rs`.

> **[IMPL] ✓ As designed (2026-06-12 — autonomous daemon scheduling):** The always-on autonomous worker honors these exact quotas — the arbiter is the single `Arc<Mutex<…>>` shared across the serve thread and both worker lanes, so concurrency caps hold whether work is operator-triggered or scheduler-triggered. The `source_lanes` rate cap is load-bearing for autonomy correctness: `procurement_source_lanes = 1` **serializes same-source runs**, so a cron slot that fires while the previous run for that source is still in flight is denied admission (`DenialReason::SourceLaneExhausted { source_id, limit: 1, in_use: 1 }`) rather than running two concurrent scrapes against one authenticated browser. A resource-denied job **defers** rather than failing: it is returned to `pending` with **no `attempt_count` bump and no held lease** (an in-memory `DenialCooldowns` backoff, base 500ms + per-job jitter, prevents hot-looping). "Denied admission → defer" is kept distinct from "ran → failed".

**Admission and release semantics:**
- Resource units are acquired when a runnable job is admitted and are released on pre-spawn failure or once the session reaches a terminal cleanup point (`reaped`/`archived`).
- Validator sub-jobs consume `agent_slots` only. In Layer 3 they never consume `browser_slots` or `source_lanes` because `headless_contract` validation does not touch source websites.

**L3-FR-1.4.1 — `browser_slots` is a hard concurrency cap for browser-backed sessions**
- AC1: Every browser-backed session consumes one `browser_slots` unit.
- AC2: The MVP default cap is `2`.
- AC3: If no browser slot is available when work becomes runnable, the arbiter delays admission rather than silently oversubscribing the host.

**L3-FR-1.4.2 — `agent_slots` is a soft cap for adapter-mediated session concurrency**
- AC1: All adapter-mediated sessions count against `agent_slots`.
- AC2: The MVP default cap is `4`.
- AC3: The arbiter can queue runnable work when the soft cap is reached, but it must not admit more concurrent sessions than the configured ceiling.

**L3-FR-1.4.3 — `source_lanes` is a per-source rate cap**
- AC1: Procurement sources default to `1/source` and supplier sources default to `2/source`.
- AC2: The same source cannot exceed its configured concurrent lane count even if global capacity remains available.
- AC3: Source-lane enforcement is source-specific and does not suppress unrelated sources.

**L3-FR-1.4.4 — Resource admission uses ordered priorities and skill-local lowering only**
- AC1: Admission priority remains, in order, procurement ingestion, auth refresh / health probes, qualification, supplier enrichment, manifest generation, and bid automation.
- AC2: `max_concurrency` declared by a skill may lower the effective ceiling below the arbiter default.
- AC3: No skill declaration can raise any typed quota above the daemon-configured budget.

**L3-FR-1.4.5 — Resource acquisition/release and validator admission remain explicit**
- AC1: Admitted work holds its resource units until pre-spawn failure or terminal cleanup releases them; slots are never leaked until manual restart.
- AC2: Validator sub-jobs consume `agent_slots` but not `browser_slots` or `source_lanes`.
- AC3: The arbiter emits structured grant/deny/release events keyed by job/session identity so resource behaviour is auditable across observability layers.

### 1.5 Cron Scheduler — Explicit Skill Binding

**Constitutional invariant:** At enqueue time, the scheduler freezes skill id + version. No late-bound skill resolution for cron jobs.

```rust
struct CronJobDefinition {
    source_id: String,
    skill_id: String,
    skill_version: String,    // Frozen at enqueue time
    cron_expression: String,
    enabled: bool,
    last_run_at: Option<DateTime<Utc>>,
    next_run_at: Option<DateTime<Utc>>,
}
```

**Continuous tick (trigger model).** Schedules are re-evaluated **continuously while the daemon serves**, not only at process start. A background `autonomous-scheduler-tick` thread builds the `CronScheduler` and seeds `last_run_at` from history once at thread start, then on each interval (~30s, `DEFAULT_SCHEDULER_TICK_INTERVAL_SECS`, env-overridable) reads `now` from an injected clock and calls `dispatch_due_jobs(&registry, now)`, enqueuing due slots through the ordinary idempotent enqueue path (`cron:{source}:{skill}:{scheduled_for}` dedup). The tick body is `run_scheduler_tick(scheduler, registry, state_db, now)` with **no inline `Utc::now()`** — production supplies the wall clock, tests supply a fixed instant, and a one-shot autonomous entrypoint `--run-scheduler-once --tick-now <rfc3339>` evaluates schedules at an injected instant, drains the worker, and exits. The registry is reloaded **only when it changes**, signalled by a monotonic `SOURCE_REGISTRY_GENERATION` counter bumped by IPC source-mutation handlers (not a per-tick DB poll); on reload, `last_run_at` is re-seeded for newly enabled sources so a just-enabled source does not fire a spurious catch-up burst.

**Catchup sweeps (folded into the tick).** Cold-start catch-up runs the tick body **once at thread start** — the same `schedule.after(last_run_at)` math that, on any later tick, turns a long monotonic gap (e.g. after host sleep/wake or downtime) into an overdue slot the next tick enqueues. Overdue work is processed in priority order through the Resource Arbiter, which enforces concurrency limits during catch-up exactly as in steady state. The previous startup-only path (`enqueue_startup_cron_jobs`) was deleted in favour of this single tick path; the bootstrap-vs-first-tick race is closed by the shared idempotency key.

**Cron-kind routing (target state, now realized).** The always-on worker (§1.4 lanes) routes every enqueued cron kind to a real executor — replacing the earlier reality where only three document/bid kinds were handled and cron-enqueued rows were orphaned:

| `kind` | Lane | Executor |
|--------|------|----------|
| `sync_source` | PTY/browser | Shared source live-loop body, routed by `(source_id, skill_id)` (FTS, Contracts Finder, ProContract discover/interest/enrich) |
| `source_health` | PTY/browser | `source-health/*` canary skill handler — **shadow/record-only** (§1.9) |
| `auth_refresh` | PTY/browser | `auth-refresh/*` (auth-signin) skill handler — **shadow/record-only** (§1.9) |
| `validation_cross_model_audit` | coverage/deterministic | Validation cross-model audit handler (not shadow-gated; see §2.3) |
| `validation_calibration_readiness_sweep` | coverage/deterministic | Calibration readiness sweep handler (not shadow-gated; see §2.3) |
| `commercial_sweep` *(new at epoch 11)* | coverage/deterministic | In-process `landed-cost` → `run_net_margin_gate_once` → manifest-generation coverage pass (see L5 §1.2 and §2.4 for the commercial-stage pointer); **does not** stop the daemon, **does not** call `trigger_pipeline_rerun` |
| `enrich_process` | — | **✗ Deferred — still orphaned** (no executor); the desktop `trigger_enrichment` button remains non-functional |

**Source circuit breaker interaction:** If `source_runtime_state` shows an open circuit or active cooldown for a source, the scheduler suppresses normal ingestion and only allows `source-health/*` or `auth-refresh/*` jobs for that source.

> **[IMPL] ✓ As designed** — Cron scheduling freezes source/skill/version definitions in `crates/daemon/src/scheduler/cron.rs:15` and applies source-circuit suppression in `crates/daemon/src/scheduler/cron.rs:140`.

> **[IMPL] ↻ Superseded (2026-06-12 — autonomous daemon scheduling):** The original "schedules dispatched once at startup via `enqueue_startup_cron_jobs`" model is **gone**. The pure tick body is `crates/daemon/src/scheduler/tick.rs:16`; the `SchedulerTickThread` is spawned in `serve_handshake_service` (thread `autonomous-scheduler-tick`, own `StateDb` connection); the one-shot entrypoint is `--run-scheduler-once --tick-now <rfc3339>`; the registry-reload signal is `SOURCE_REGISTRY_GENERATION` (`crates/daemon/src/sources/registry.rs`); the `commercial_sweep` synthetic slot (`0 0 7 * * *`) is defined in `crates/daemon/src/scheduler/cron.rs`. `enqueue_startup_cron_jobs` is deleted; cold-start catch-up runs the tick body once at thread start.

**L3-FR-1.5.1 — Cron scheduling freezes the executable skill binding at enqueue time**
- AC1: Every cron-enqueued job stores `source_id`, `skill_id`, `skill_version`, `cron_expression`, and enablement/next-run state as part of its scheduling definition.
- AC2: The scheduler freezes `skill_id + skill_version` when the job is enqueued and does not resolve a newer skill version later at execution time.
- AC3: If the scheduler cannot admit a due job immediately, the bound job remains queued/runnable with its original frozen binding.
- AC4: Cron roots enqueue through the ordinary `EnqueueJobRequest` surface, leaving `parent_job_id = NULL` for root jobs and preserving the same job-claim/visibility-fence semantics as manually triggered work.

**L3-FR-1.5.2 — Catchup sweeps and source suppression are explicit scheduler behaviors**
- AC1: On daemon start after downtime, the scheduler detects overdue runs and processes them in priority order rather than flooding the system.
- AC2: Catchup execution still passes through the Resource Arbiter and therefore respects typed quotas during recovery.
- AC3: If a source circuit is open or cooling down, normal ingestion for that source is suppressed and only `source-health/*` or `auth-refresh/*` jobs may be scheduled.

### 1.6 PTY Hardening

| Hardening | What It Does | Why It Matters |
|-----------|-------------|----------------|
| **fd hygiene** | Aggressively close inherited fds before exec | macOS Big Sur/Cocoa and GNOME/Mutter can leak fds into child processes, causing intermittent failures |
| **EIO handling** | Treat EIO on PTY master read (after slave closes) as normal termination | Without this, EIO after slave close looks like an internal error |
| **Terminal geometry** | Fix PTY size for headless runs (e.g., 120x40) | CLI formatting and browser/tool behavior must be stable. Only resize when operator attaches for live viewing |
| **Signal handling** | Use signals only as wakeups, then drain waitpid/wait loops until nothing left to reap | Signals may be coalesced by Tokio's Unix signal streams |

> **[IMPL] ✓ As designed** — PTY hardening has a dedicated module in `crates/daemon/src/pty/hardening.rs`, while default terminal geometry and phase names are implemented in `crates/daemon/src/pty/session.rs:19`.

**L3-FR-1.6.1 — PTY sessions apply the full hardening bundle before being treated as reliable**
- AC1: Child exec paths aggressively close inherited fds before exec so unrelated parent fds are not leaked into browser/CLI descendants.
- AC2: `EIO` on PTY master read after slave closure is treated as expected hangup, while unexpected active-session PTY read failure still follows an explicit cleanup/error path.
- AC3: Headless PTY runs default to a stable fixed geometry of `120x40` and only resize on explicit operator attachment.
- AC4: Unix signals act as wakeups only; the daemon drains `waitpid`/wait loops until no more children remain to reap.
- AC5: PTY hardening preserves the existing session lifecycle ordering `Created -> Spawned -> Active -> Terminating -> Exited -> Reaped -> Archived` rather than inventing a separate hardening-specific state machine.
- AC6: Expected post-slave-close hangup does not require a dedicated new `PtySessionError` variant; unexpected PTY I/O still flows through the existing `PtySessionError` classification surface with explicit context.

### 1.7 Host Environment Awareness

- **Sleep/wake:** If host suspends during active session, assume browser-driven PTY sessions are tainted. Use monotonic time for stall detection. Emit `HOST_SLEEP_INTERRUPTED` if suspend gap exceeds configurable threshold.
- **Missed runs:** Catch-up no longer requires a daemon restart. While the daemon stays open, the next `autonomous-scheduler-tick` (§1.5) after a sleep/wake or idle gap sees the long `schedule.after(last_run_at)` interval as an overdue slot and enqueues it; the cold-start run of the same tick body covers the restart-after-downtime case. Recovered work is executed in priority order via the resource arbiter either way.

> **[IMPL] ✓ As designed** — Host and operational snapshots are surfaced through the daemon status model: cron status at `crates/daemon/src/main.rs:2370`, source health at `crates/daemon/src/main.rs:2397`, validation status at `crates/daemon/src/main.rs:2430`, and resource status at `crates/daemon/src/main.rs:2516`.

**L3-FR-1.7.1 — Host sleep/wake and downtime are first-class runtime conditions**
- AC1: Active browser-driven PTY sessions are treated as tainted if the host suspends during execution.
- AC2: Stall detection uses monotonic time rather than wall-clock time.
- AC3: A suspend gap above the configured threshold emits `HOST_SLEEP_INTERRUPTED`.
- AC4: Extended downtime triggers a catchup sweep for overdue jobs, and recovered work still respects resource-arbiter priority ordering.
- AC5: Host resource-pressure signals may block new job admission while allowing already-running work to complete when safe, and the pressure event is surfaced explicitly rather than inferred from secondary failures.
- AC6: Sleep/resource classifications extend the existing `pty/health.rs::HealthIssue` surface rather than creating a parallel host-health enum.

### 1.8 Shared Library Additions

| Package | File | Purpose |
|---------|------|---------|
| `packages/intelligence` | `src/relations/resolve-process-relations.ts` | Relation resolution engine (see §3) |
| `packages/manifests` | `render-error-manifest.ts` | Error manifest Markdown rendering from error context |
| `packages/manifests` | `error.md.ts` | Error manifest Markdown template: error chain, job context, skill info, suggested resolution |

> **[IMPL] ⚠ Deviation:** Actual implementation (2026-05-02): relation computation is currently daemon-owned rather than implemented as the full TypeScript package algorithm described here. The runtime relation stage lives in `crates/daemon/src/main.rs:19175` and writes OCID-exact `merged_same_process` edges; richer heuristic/scored relation logic remains a future extension.

**L3-FR-1.8.1 — Layer 3 activates the shared-library modules needed for relations and error rendering**
- AC1: `packages/intelligence/src/relations/resolve-process-relations.ts` exists as the Layer 3 relation-resolution engine surface referenced by §3.6.
- AC2: `packages/manifests/render-error-manifest.ts` and `packages/manifests/error.md.ts` exist as the shared error-manifest rendering and template surfaces used by the daemon/desktop path.
- AC3: These shared-library additions provide reusable package-level functionality and do not move authoritative workflow state out of the daemon.

### 1.9 New Skills (Layer 3)

**4 operational skills:**

| Skill | Purpose | Interaction Profile |
|-------|---------|---------------------|
| `skills/auth-refresh/alibaba/` | Re-authenticate Alibaba session when auth expires | `pty_interactive` |
| `skills/auth-refresh/1688/` | Re-authenticate 1688 session when auth expires | `pty_interactive` |
| `skills/source-health/fts/` | Canary probe: verify FTS page fingerprint + basic extraction | `pty_interactive` |
| `skills/source-health/alibaba/` | Canary probe: verify Alibaba page fingerprint + auth + basic search | `pty_interactive` |

**7 validator-base files** (6 standard skill files + 1 reference index; base semantic validator skill):

| File | Purpose |
|------|---------|
| `skills/validation/base/SKILL.md` | Base validator playbook — generic semantic quality checks applicable to any skill |
| `skills/validation/base/result.schema.json` | Validator verdict schema |
| `skills/validation/base/checkpoint.schema.json` | No-op checkpoint schema; validators do not emit checkpoints in Layer 3 |
| `skills/validation/base/canary.inputs.jsonl` | Known-good and known-bad run manifests for canary testing |
| `skills/validation/base/golden.assertions.yaml` | Expected verdicts for canary inputs |
| `skills/validation/base/validator-rubric.yaml` | Distilled runtime rubric — skill semantic invariants for live prompts (not raw golden assertions) |
| `skills/validation/base/references/manifest-corpus-index.md` | Corpus index used for calibration provenance and canary inventory |

**L3-FR-1.9.1 — Layer 3 skill inventory is explicitly specified and interaction-profile bound**
- AC1: The operational skill inventory includes `skills/auth-refresh/alibaba/`, `skills/auth-refresh/1688/`, `skills/source-health/fts/`, and `skills/source-health/alibaba/`.
- AC2: Each operational skill declares a single purpose and uses the `pty_interactive` interaction profile.
- AC3: The base validator skill inventory includes the six standard skill files plus `references/manifest-corpus-index.md` under `skills/validation/base/`.
- AC4: The Layer 3 skill inventory is sufficient to implement auth refresh, source health, and generic semantic validation without introducing undocumented directories or files.

> **[IMPL] ✓ As designed** — The operational skill families and validator base directory exist under `skills/auth-refresh/`, `skills/source-health/`, and `skills/validation/base/`; validation execution code consumes the base validator contract through `crates/daemon/src/validation/executor.rs:25`.

> **[IMPL] ✓ As designed (2026-06-12 — autonomous daemon scheduling):** The `source-health/*` and `auth-refresh/*` skills now have **daemon job-handlers** (previously skills with no executor reachable from cron — see §1.5 routing). Both run in **shadow / record-only** mode: the handlers run for real and **persist their side-effect rows** (`source_runtime_state`, `auth_refresh_results`), but completion is stamped `autonomy_mode=shadow enforcement_action=recorded_only production_gate_applied=false`, so the recorded outcome does not yet gate production. This is a **compile-time constant** (`HEALTH_AUTH_SHADOW_MODE = "shadow"`), **not** a runtime/rollout config flag. Shadow mode is applied **only** to `source_health` + `auth_refresh`; the `validation_*` and `commercial_sweep` cron kinds are **not** shadow-gated.

### 1.10 What's Stubbed in This Layer

| Feature | Layer 3 Stub | Full Implementation |
|---------|-------------|-------------------|
| Validation enforcement | Dry-run only — verdicts logged, not enforced | Layer 4: enforcement mode |
| Domain-specific validators | Base validator only (generic semantic quality) | Layer 4: 4 domain overlays |
| Validator calibration | Initial calibration against gold-set manifests | Layer 4: formal calibration harness with 3 corpora |
| Sync | Tracking columns populated, no outbox/push/pull | Layer 4: full sync engine |
| Multi-user | Single operator only | Layer 4: multi-user via web portal |

> **[IMPL] ↻ Superseded** — The “L3 stub” rows remain accurate only for L4/L5/L6 dependencies. L3 itself has since landed Stage 14 operator clarification/override data in `migrations/0009_operator_input_overrides.sql:1` and desktop IPC handlers in `crates/daemon/src/ipc/handlers/clarifications.rs:90`; validation enforcement, sync, multi-user web, and domain validator overlays remain planned L4 work.

### 1.11 ProContract Awaiting-Access Resumption

> **[IMPL] ✓ As designed (2026-06-12 — autonomous daemon scheduling, schema epoch 11).** This subsection documents an operational-hardening capability added alongside the autonomous scheduler/worker. It is the *ingestion-side* resumption of ProContract tenders awaiting portal access — distinct from the L6 bid-workflow `AUTH_STATE_STALE` resumption pattern.

**Problem.** ProContract register-interest fires once and correctly never re-expresses interest, but when the portal has not yet granted document access the tender sits "awaiting access". Previously the only safety net was the enrich skill re-scanning the portal's "Recently added" list (`maxPages` ≈ 50 rows) each run; a tender stuck awaiting access for many days while newer activities pile in could age out of that window and be silently dropped, because the daemon kept no durable record.

**Durable tracking.** Awaiting-access tenders are now tracked in the local-only `procontract_pending_enrichment` table (full DDL in §4.1) while `interest_registration.state` remains `'completed'`. Enrich derives a **work-list** from `procontract_pending_enrichment WHERE state='awaiting_access'` and navigates directly to each tracked advert by its stored `advert_id`, so resumption is **independent of the "Recently added" ordering and `maxPages`**.

**Correctness invariant (the landmine).** The create transition does **NOT** touch `interest_registration.state` — it stays `'completed'`, so the register-interest re-fire guard (which excludes `state IN ('completed','succeeded','skipped')`) still excludes the tender. A second `NOT EXISTS` anti-join against `procontract_pending_enrichment` was added to the interest-candidate query. If awaiting-access were instead modeled by flipping the `interest_registration` row's state, register-interest would re-express interest every run — explicitly avoided.

**Five-transition lifecycle:**

| Transition | Trigger | Row effect |
|------------|---------|-----------|
| **create** | register-interest succeeds | insert `state='awaiting_access'`, `observation_count=0`; `interest_registration.state` stays `'completed'` |
| **keep** | enrich runs, portal still shows no live tender event | same row, bump `observation_count` / `last_checked_at` |
| **clear** | enrich finds the live tender event ("Not started (Respond by …)") and downloads docs | `state='enriched'`, `enriched_at` recorded |
| **backfill** | enrich observes an untracked portal item as `not_yet_responded` | insert a new tracking row (e.g. interest registered before this feature) |
| **expire** | staleness sweep fires | `state='expired'`, stop sweeping it |

**Staleness rule (Q5).** Expiry keys off the notice-level `deadline_at` captured at discover (reason `notice_deadline`) else a `PROCONTRACT_AWAITING_ACCESS_MAX_AGE_DAYS = 90` fallback (reason `max_age`). It **must never** key off the RFx *response* deadline — that is only learned from the live tender event, which an awaiting-access item by definition has not reached.

**Skill contract.** `skills/procurement-ingestion/procontract-enrich-tender/SKILL.md` is at **v1.3.0**, gaining an optional `awaitingAccessTargets: [{ advertId, sourceNoticeId, processId }]` input. The skill runs its normal "Recently added" scan and, for each work-list target not already located, navigates directly by `advertId`, then **unions** both result sets deduping by `advertId`. When the work-list is empty the skill behaves exactly as before (pure scan) — backward compatible. The daemon builds the work-list at execution time (it is not in the cron payload), so resumption depends only on the autonomous enrich run happening on schedule (G1/G2).

**L3-FR-1.11.1 — Awaiting-access resumption is durable and ordering-independent**
- AC1: A successful register-interest creates a `procontract_pending_enrichment` row in `state='awaiting_access'` without mutating `interest_registration.state`.
- AC2: Enrich sweeps the daemon-tracked work-list by `advert_id`, independent of the portal's "Recently added" ordering and `maxPages`.
- AC3: The five transitions (create/keep/clear/backfill/expire) are each driven by their stated trigger.
- AC4: Expiry uses notice-level `deadline_at` else the 90-day max-age fallback, never the RFx response deadline.
- AC5: The enrich skill (`v1.3.0`) is backward compatible — an empty work-list reproduces the prior pure-scan behavior.

---

## 2. Worker-Validator System (Dry-Run)

> **Ownership:** This section owns the dry-run validation system: policy engine, executor, base validator skill, outcome classes, and fail-open semantics. Validation **enforcement** (transition from dry-run to production blocking) is owned by Layer 4 §3. The base **run manifest contract** is defined in Layer 1 §4; this section recaps what the validator needs and adds Layer-3-specific fields.

### 2.1 Run Manifest Recap (Self-Contained for Layer 3)

Every skill execution produces a **hybrid-authored run manifest** — agent narrative + daemon evidence appendix. This recap provides what a Layer 3 implementer needs without opening Layer 1.

**Structure:**

```
YAML frontmatter (structured metadata)
---
Agent narrative sections (human-readable MD)
---
Daemon evidence appendix (machine-authored, immutable)
```

**YAML frontmatter — mostly agent-claimed fields plus a small daemon-derived status subset:**

| Field | Purpose |
|-------|---------|
| `manifest_version` | Format version |
| `skill_id`, `skill_version` | Which skill ran |
| `job_id`, `session_id` | Execution identity |
| `execution_mode` | `pty` or `contract` |
| `started_at`, `completed_at` | Timing |
| `outcome_class` | One of 6 outcome classes (see §2.5) |
| `items_processed`, `items_skipped`, `items_failed` | Self-reported counts |
| `warnings_count` | Count of surfaced warnings/degradations |
| `agent_claimed_pages_visited` | Navigation count |
| `agent_claimed_source_url` | Target URL |
| `agent_claimed_fingerprint_state` | `matched` / `mismatched` / `unknown` |
| `agent_claimed_locale_state` | `matched` / `mismatched` / `unknown` |
| `agent_claimed_currency_state` | `matched` / `mismatched` / `unknown` |
| `agent_narrative_missing` | Explicit daemon-set flag for fallback manifests |

**Agent narrative sections:**

- **Summary:** What the agent did and found
- **Steps Taken:** Navigation and extraction sequence
- **Decisions Made:** Choices with reasoning
- **Warnings:** Anomalies or degradations encountered
- **Evidence:** Key evidence with claim/evidence references to result contract fields or daemon appendix anchors

**Daemon evidence appendix** (machine-authored, immutable, `daemon_observed_*` prefix):

| Field | Purpose |
|-------|---------|
| `observed_url_chain` | Actual URL sequence from protocol events |
| `observed_page_kind_sequence` | Page-kind classifications from adapter |
| `daemon_observed_fingerprint_state` | `matched` / `mismatched` / `unknown` / `no_baseline` |
| `daemon_observed_locale` | Detected locale from protocol events |
| `daemon_observed_currency` | Detected currency from protocol events |
| `auth_challenge_events` | Count and types of auth/challenge events |
| `locator_fallback_depth` | Maximum fallback tier reached during navigation |
| `checkpoint_count` | Validated checkpoint count |
| `result_item_count` | Item count from validated result contract (independent of agent claim) |
| `oqg_anomaly_summary` | OQG semantic/drift signals (fill-rate delta, duplicate-rate delta, outliers) |
| `representative_item_sample` | IDs of median items + OQG-flagged outlier items |
| `artifact_refs` | Paths and hashes to forensic artifacts |

**Critical naming rule:** `agent_claimed_*` = self-reported by agent. `daemon_observed_*` = independently observed by daemon. Discrepancies between these are high-signal issues for validators.

**Fallback:** If the agent fails to emit a narrative (crash, timeout), the daemon synthesizes a minimum machine-authored manifest with `agent_narrative_missing: true`. The worst runs must not leave the thinnest review surface.

**Target size:** < 50 KB per manifest. Summary patterns and anomalies, don't enumerate every item. If a full appendix would exceed budget, the daemon first elides bulky representative samples/drift blobs behind artifact refs and explicit "evidence elided for size" markers; only if that bounded form still fails does it fall back to the minimum machine-authored manifest.

**L3-FR-2.1.1 — Every Layer 3 skill run produces a hybrid-authored run manifest**
- AC1: Every skill execution emits one manifest containing YAML frontmatter, agent narrative sections, and a daemon evidence appendix.
- AC2: The manifest structure is stable enough that a Layer 3 implementer can implement validator inputs without reopening Layer 1.
- AC3: Run manifests remain immutable execution artifacts rather than editable operator documents.

**L3-FR-2.1.2 — Agent-claimed frontmatter uses a stable claimed-field contract**
- AC1: Frontmatter includes `manifest_version`, skill identity/version, job/session identity, execution mode, timing, `outcome_class`, item-count claims, `warnings_count`, and claimed page/source/locale/currency state.
- AC2: Agent-self-reported fields use the `agent_claimed_*` prefix whenever the value is not independently observed by the daemon.
- AC3: `agent_narrative_missing` is an explicit frontmatter field, not an inferred renderer-only convenience.
- AC4: `outcome_class` values align to the operational semantics in §2.5, even where the current `artifacts.rs` enum serializes only a subset directly in frontmatter.

**L3-FR-2.1.3 — Agent narrative sections are present and evidentiary**
- AC1: Narrated manifests include `Summary`, `Steps Taken`, `Decisions Made`, `Warnings`, and `Evidence` sections.
- AC2: `Evidence` entries reference either validated result-contract fields or daemon appendix anchors rather than free-floating unsupported claims.
- AC3: Successful narrated runs must not leave required narrative sections empty.

**L3-FR-2.1.4 — The daemon appendix is machine-authored, discrepancy-aware, and fallback-safe**
- AC1: The appendix records observed URL/page-kind sequences, daemon-observed fingerprint/locale/currency state, auth/challenge events, fallback depth, checkpoint/result counts, OQG anomaly summary, representative samples, and artifact references.
- AC2: `agent_claimed_*` and `daemon_observed_*` names remain distinct so validator disagreement can be detected mechanically.
- AC3: If the agent fails to narrate, the daemon still emits a fallback manifest with `agent_narrative_missing: true`.
- AC4: Target manifest size remains under `50 KB` by summarizing patterns/anomalies rather than enumerating every extracted item.
- AC5: Size-budget overflow is handled by bounded evidence elision before fallback rather than by failing the worker run for manifest verbosity alone.

> **[IMPL] ✓ As designed** — Run-manifest frontmatter, six outcome classes, daemon appendix fields, and the 50 KB target are implemented in `crates/daemon/src/agents/artifacts.rs:15` and written through `crates/daemon/src/validation/manifest_writer.rs:9`.

**L3-FR-2.1.5 — Claim/observation disagreements are explicit review artifacts**
- AC1: Disagreements between agent-claimed and daemon-observed fields render as an explicit claim/observation disagreement section rather than being hidden in free text.
- AC2: Stored-manifest metadata may derive `warning_review_required = true` when disagreements exist, but that storage flag is derived metadata rather than additional frontmatter.
- AC3: The current `RunManifestFrontmatter` and `DaemonEvidenceAppendix` field sets in `artifacts.rs` are the minimum core contract; additive Layer 3 appendix fields from §2.2 must extend that surface rather than being silently omitted.

### 2.2 Layer 3 Validation Additions to Run Manifests

Layer 3 treats the following appendix fields as validator-required. Most are newly added here; `checkpoint_count` is repeated explicitly because validators depend on it as part of the bounded-evidence contract:

| Field | Purpose | Source |
|-------|---------|--------|
| `basis_sufficiency` | Whether the manifest provides enough evidence for the validator to render a confident verdict | Computed by daemon from appendix completeness |
| `page_fingerprint_hash` | Cryptographic hash of the page fingerprint at extraction time | Page fingerprint system |
| `auth_challenge_detected` | Boolean — whether any auth challenge was observed | Session health monitor |
| `extraction_fill_rate_bps` | Fill rate as integer basis points (0-10000) | OQG behavioral checks |
| `checkpoint_count` | Number of validated checkpoints | Checkpoint validator |
| `byte_count_compressed` | Compressed artifact size in bytes | Artifact store |
| `validator_rubric_hash` | Hash of the `validator-rubric.yaml` used | Skill directory |
| `drift_delta_json` | Structured drift deltas vs prior runs (not narrative history) | OQG behavioral tracking |

**L3-FR-2.2.1 — Layer 3 appends validator-specific evidence fields to the manifest appendix**
- AC1: The daemon evidence appendix includes `basis_sufficiency`, `page_fingerprint_hash`, `auth_challenge_detected`, `extraction_fill_rate_bps`, `checkpoint_count`, `byte_count_compressed`, `validator_rubric_hash`, and `drift_delta_json`.
- AC2: These fields are machine-derived from daemon/runtime systems rather than agent-authored narrative text.
- AC3: The added fields are available to validators without reopening raw forensic bytes.

**L3-FR-2.2.2 — Basis sufficiency, rubric identity, and drift deltas are explicit validation inputs**
- AC1: `basis_sufficiency` is computed from appendix completeness rather than guessed by the validator from prose alone.
- AC2: `validator_rubric_hash` binds the rendered verdict to the exact `validator-rubric.yaml` in effect for that run.
- AC3: `drift_delta_json` stays structured and machine-comparable rather than narrative-only history.
- AC4: If OQG signals or drift deltas are unavailable, the appendix records explicit absence and the validator lowers `basis_sufficiency` instead of inferring a clean run from missing inputs.

> **[IMPL] ✓ As designed** — The validator-specific appendix fields are part of `DaemonEvidenceAppendix` in `crates/daemon/src/agents/artifacts.rs:48`, so validators can consume bounded evidence without reading raw forensic bytes.

### 2.3 Validation Policy Engine

Determines whether a completed job requires semantic validation. Reads policy from skill metadata — no daemon code changes needed.

**Decision flow:**

1. Schema validation failed? -> No validator (already quarantined as `AGENT_OUTPUT_CONTRACT_INVALID`)
2. Skill in draft/candidate/canary/shadow stage? -> **ALWAYS VALIDATE** regardless of declared policy
3. `validation_policy: always`? -> **VALIDATE**
4. `validation_policy: sample`? -> **Risk-stratified sampling** (see below)
5. `validation_policy: trigger`? -> Check trigger conditions; validate if any fires
6. `validation_policy: skip`? -> **SKIP** (OQG code checks still run)
7. Any trigger condition met regardless of policy? -> **VALIDATE**

**Risk-stratified sampling** (replaces pure Bernoulli random):

When `validation_policy: sample`, the effective validation probability = `max(validation_sample_rate, risk_multiplier)`. Risk factors that force probability to 1.0 (always validate):

| Risk Factor | Source | Rationale |
|-------------|--------|-----------|
| Source health amber or red | `source_runtime_state` | Degraded sources produce the outputs most likely to be wrong |
| Page fingerprint unknown / no baseline | Daemon evidence appendix | No machine-verifiable page identity |
| First run after worker version change | Skill version metadata | Version transitions are highest-risk for regressions |
| Validator error spike on same slice | Recent validation history | Fail-open is opening on the hard cases |
| Continuation chain length > 1 | Job metadata | Multi-segment runs have cross-segment blind spots |
| High locator fallback depth | Daemon evidence appendix | Deep fallback signals DOM instability |
| Low basis sufficiency in recent runs | Recent verdict metadata | Validators couldn't assess this slice |
| High-value opportunity | Pipeline assessment | Downstream damage scales with value |
| `suspect_output_in_last_5` | Recent job history | Ongoing quality issue pattern |

**Extended trigger conditions** (force validation regardless of declared policy):

`fill_rate_drop > threshold`, `first_run_after_version_change`, `suspect_output_in_last_5`, `operator_requested`, `source_health_amber_or_red`, `page_fingerprint_unknown`, `continuation_chain_length > 1`, `high_value_opportunity`

**Per-slice enforcement scope:**

The unit of enforcement readiness is **worker skill x source x worker behavior version x validator behavior version** — not the whole system. A supplier-enrichment validator can be enforcement-ready while a document-acquisition validator stays dry-run.

**Behavior version vs raw version:** Only bump `worker_behavior_version` or `validator_behavior_version` when output semantics or decision boundaries materially change. Cosmetic prompt edits, observability-only changes, or cost-optimisation tuning do NOT reset slice readiness. Without this distinction, most slices never reach enforcement because the evidence window is shorter than the version half-life.

Enforcement eligibility per slice requires:
- Validator overlay `validator_overlay_compatible_versions` matches current worker behavior version
- Calibration thresholds met for that slice (minimum adjudicated cases, reject precision bounds)
- No active demotion trigger

**Automatic demotion** from enforcement to dry-run/flag-only:
- Operator override rate spikes above threshold (operators regularly reversing rejections)
- Challenge-bank performance regresses (validator stops catching known-bad cases)
- Anomaly-conditioned validator error rate rises (validators crash on risky runs)
- Worker behavior version moves outside overlay's compatible range

Demotion is automatic and immediate. Re-promotion requires meeting calibration thresholds again.

**Dry-run bootstrap validation ladder:**

Validation rates during dry-run should be staged to accelerate calibration data acquisition:

| Phase | Validation Rate | Duration / Trigger |
|-------|----------------|--------------------|
| **Bootstrap** | 100% of first 20 live runs for a new slice | New slice introduction |
| **Calibrating** | 50% of all runs; 100% of high-risk runs | Until slice is clearly maturing or clearly long-tail |
| **Signature-limited enforcement** | Quarantine only on calibrated high-precision incident signatures; everything else flag-only | After overlay/family prequalification passes |
| **Full slice enforcement** | Per current 30/60 + Wilson + false-pass criteria | After all calibration gates met |

This ladder fits the existing design because `incident_signature` and `enforcement_eligible` are already first-class verdict fields. Calibrated incident families can accelerate enforcement without collapsing the per-slice model.

**L3-FR-2.3.1 — Validation policy is metadata-driven and starts after contract validity is known**
- AC1: The policy engine reads validation behavior from skill metadata rather than requiring daemon code edits for each skill.
- AC2: Outputs that already failed schema validation do not spawn validator runs and remain quarantined as `AGENT_OUTPUT_CONTRACT_INVALID`.
- AC3: Trigger conditions can still force validation regardless of the declared steady-state policy mode.
- AC4: When policy selects validation for a worker job, `workflow_jobs.validation_required` is set on the worker job; validator sub-jobs are then linked through `parent_job_id` rather than inferred from transcript scraping or ad hoc session metadata.

**L3-FR-2.3.2 — Pre-stable promotion stages always validate**
- AC1: Skills in `draft`, `candidate`, `canary`, or `shadow` stage always validate regardless of declared `validation_policy`.
- AC2: This always-validate rule applies before the mode-specific branches for `always`, `sample`, `trigger`, or `skip`.
- AC3: Stable/rollback behavior may use declared policy, but pre-stable stages may not opt out.

**L3-FR-2.3.3 — `validation_policy: always` validates every eligible completed run**
- AC1: In `always` mode, every completed run that is not already schema-invalid proceeds to semantic validation.
- AC2: The daemon does not downsample `always` mode based on cost or current load.
- AC3: `always` mode still shares the same executor, verdict schema, and fail-open handling as other policies.

**L3-FR-2.3.4 — `validation_policy: sample` uses risk-stratified sampling rather than pure Bernoulli sampling**
- AC1: Effective validation probability in `sample` mode is `max(validation_sample_rate, risk_multiplier)`.
- AC2: The following risk factors force probability to `1.0`: source health amber/red, fingerprint unknown/no baseline, first run after worker version change, validator error spike on same slice, continuation chain length > `1`, high locator fallback depth, low basis sufficiency in recent runs, high-value opportunity, and `suspect_output_in_last_5`.
- AC3: Sampling decisions are explainable from explicit runtime metadata, not opaque random-only heuristics.

**L3-FR-2.3.5 — `validation_policy: trigger` validates only when a defined trigger fires**
- AC1: In `trigger` mode, validation runs when one or more configured trigger conditions are satisfied.
- AC2: If no trigger fires, the run skips semantic validation while still passing through the non-semantic OQG path.
- AC3: Trigger evaluation is deterministic from the declared trigger inventory in this section.

**L3-FR-2.3.6 — `validation_policy: skip` suppresses semantic validation without suppressing OQG**
- AC1: In `skip` mode, the daemon omits semantic validator execution for normal runs.
- AC2: OQG code checks still run even when semantic validation is skipped.
- AC3: Global override triggers may still force validation for a `skip` skill when the trigger inventory says they must.

**L3-FR-2.3.7 — Trigger conditions are explicit and mechanically testable**
- AC1: The extended trigger inventory includes `fill_rate_drop > threshold`, `first_run_after_version_change`, `suspect_output_in_last_5`, `operator_requested`, `source_health_amber_or_red`, `page_fingerprint_unknown`, `continuation_chain_length > 1`, and `high_value_opportunity`.
- AC2: A run that satisfies any listed trigger validates even if its declared mode would otherwise skip or sample.
- AC3: Trigger evaluation records enough structured context that tests can assert why validation did or did not run.

**L3-FR-2.3.8 — Enforcement readiness is tracked per behavioral slice, not globally**
- AC1: The slice key is `worker skill x source x worker behavior version x validator behavior version`.
- AC2: `worker_behavior_version` and `validator_behavior_version` change only for semantic/decision-boundary changes, not cosmetic prompt or observability edits.
- AC3: Enforcement eligibility requires compatible overlay versions, calibration thresholds for that slice, and no active demotion trigger.

**L3-FR-2.3.9 — Demotion from enforcement readiness is automatic and immediate**
- AC1: Automatic demotion triggers include operator override spikes, challenge-bank performance regression, anomaly-conditioned validator error-rate increase, and worker-version incompatibility with the overlay.
- AC2: When a demotion trigger fires, the slice returns to dry-run/flag-only immediately without waiting for operator approval.
- AC3: Re-promotion requires satisfying calibration thresholds again for the affected slice.

**L3-FR-2.3.10 — Dry-run validation rates follow the staged bootstrap ladder**
- AC1: New slices validate `100%` of the first `20` live runs during `Bootstrap`.
- AC2: `Calibrating` mode validates `50%` of all runs and `100%` of high-risk runs until the slice is clearly maturing or clearly long-tail.
- AC3: `Signature-limited enforcement` allows quarantine only for calibrated high-precision incident signatures while all other cases remain flag-only.
- AC4: `Full slice enforcement` remains gated by the existing `30/60 + Wilson + false-pass` criteria.

> **[IMPL] ✓ As designed** — Validation policy modes, risk triggers, dry-run ladder phase derivation, and blind demotion logic are implemented in `crates/daemon/src/validation/policy.rs:14`, `crates/daemon/src/validation/policy.rs:238`, and `crates/daemon/src/validation/policy.rs:352`, with coverage in `crates/daemon/tests/validation_policy.rs` and `crates/daemon/tests/validation_canary_health.rs`.

> **[IMPL] ✓ As designed (2026-06-12 — autonomous daemon scheduling):** The two scheduled validation cron kinds — `validation_cross_model_audit` and `validation_calibration_readiness_sweep` — now have real worker executors on the daemon's **coverage/deterministic lane** (§1.5 routing). They are **not** shadow-gated (shadow/record-only applies only to `source_health` + `auth_refresh`, §1.9). This is the *scheduling* path that drives these sweeps autonomously; it is distinct from the per-result `semantic_validate` sub-job path that this section's policy engine governs.

### 2.4 Validation Executor

Spawns validator sub-jobs via the Job Runner. Links validator job to worker job via `parent_job_id`.

Execution reuses the Layer 1 agent-runtime contracts rather than inventing a validator-only path: validator sessions run through the same Agent Adapter boundary defined in [layer-1-thin-local-runner.md](/Users/alinaaitova/Desktop/Testproject1/docs/strategy/layers/layer-1-thin-local-runner.md#L475), the same result-contract validation boundary in [layer-1-thin-local-runner.md](/Users/alinaaitova/Desktop/Testproject1/docs/strategy/layers/layer-1-thin-local-runner.md#L1302), and the same cross-layer interface contract summarized in [layer-strategy.md](/Users/alinaaitova/Desktop/Testproject1/docs/strategy/layer-strategy.md#L1176).

**Kickoff prompt composition:**

| Input | Source | Purpose |
|-------|--------|---------|
| Run manifest (full) | Artifact store | Primary evidence — agent narrative + daemon appendix |
| `validator-rubric.yaml` | Skill directory (distilled, not raw `golden.assertions.yaml`) | Evaluation criteria — compact, runtime-optimized |
| Structured drift deltas | OQG behavioral tracking | How this run differs from recent runs (not narrative history) |
| OQG anomaly signals | Result contract validator | Fill-rate, duplicate-rate, value-range anomalies |
| Anomaly-stratified result contract sample | Result contract | Representative items + OQG-flagged outliers (not uniform random) |

**What the validator does NOT see:**
- Raw forensic bytes (too large, too noisy)
- Worker's internal reasoning (prevents anchoring bias)
- Full result contract (only anomaly-stratified sample)

**Validator skill execution:**
- Uses `headless_contract` mode (no browser needed)
- Uses `structured_extract_low_cost` model profile
- Layer 3 loads only the base validator (`skills/validation/base/`) for generic quality checks
- Overlay wiring is reserved for Layer 4; Layer 3 does not yet load domain overlays or source-specific validator overrides

**Verdict output:**

```json
{
  "verdict": "approved | rejected | flagged",
  "basis_sufficiency": "high | medium | low",
  "enforcement_eligible": true,
  "confidence_bps": 8500,
  "issues": [
    {
      "severity": "critical | major | minor | observation",
      "category": "identity | completeness | plausibility | consistency | freshness",
      "description": "...",
      "evidence_refs": ["daemon_appendix.observed_url_chain", "result_contract.item_42"]
    }
  ],
  "summary": "..."
}
```

**Verdict handling by Job Runner:**

| Verdict | Dry-Run (L3) | Enforcement (L4) |
|---------|-------------|-----------------|
| `approved` | Log verdict, advance stage normally | Advance stage normally |
| `rejected` + `enforcement_eligible` | Log verdict, advance stage anyway (dry-run) | Quarantine as `AGENT_SUSPECT_OUTPUT`, create error manifest |
| `rejected` + not enforcement-eligible | Log verdict, advance with flag | Advance with flag for operator review |
| `flagged` | Log verdict, advance with flag | Advance with flag for operator review |
| `basis_sufficiency: low` | Log as `VALIDATION_FLAGGED` with a `coverage_gap` issue | Advance with `unverified` trust state |
| Validator crash/timeout/invalid verdict payload | Fail-open: advance as if skipped | Fail-open: advance with `VALIDATION_SKIPPED_VALIDATOR_ERROR` |

**Persistence rule:** A validator verdict becomes authoritative only when the `validation_verdicts` insert and the worker-job `validation_verdict_id` linkage commit in the same SQLite transaction. A rendered-but-unpersisted verdict is treated as validator failure and routed through fail-open + retroactive backlog handling.

**L3-FR-2.4.1 — Validator execution reuses the Layer 1 job/session boundary**
- AC1: Validator runs are spawned as Job Runner sub-jobs linked to worker jobs through `parent_job_id`.
- AC2: Validator sessions reuse the existing Layer 1 agent adapter and result-contract boundaries instead of defining a validator-only execution path.
- AC3: Validator execution remains visible as its own job/session pair for auditing and failure handling.
- AC4: Validation sub-jobs enqueue through the same `EnqueueJobRequest` surface as other jobs, preserving `parent_job_id` linkage and not bypassing the runner's claim/order/idempotency path.
- AC5: If the worker run belongs to a continuation chain, validator bookkeeping preserves that chain identity rather than treating continuation segments as unrelated runs.

**L3-FR-2.4.2 — The executor builds validator prompts from bounded, structured evidence**
- AC1: Kickoff composition includes the full run manifest, `validator-rubric.yaml`, structured drift deltas, OQG anomaly signals, and an anomaly-stratified result sample.
- AC2: The validator does not receive raw forensic bytes, the worker's internal reasoning, or the full result contract.
- AC3: Prompt inputs are runtime-bounded so validators review representative evidence rather than arbitrarily large payloads.

**L3-FR-2.4.3 — Validator runs use a low-cost contract-mode execution profile in Layer 3**
- AC1: Validator jobs use `headless_contract` execution and do not require a browser session.
- AC2: Validator jobs use the `structured_extract_low_cost` model profile.
- AC3: Layer 3 validation runs use the base validator skill only; domain overlays and source-specific overrides are reserved for Layer 4 activation.

**L3-FR-2.4.4 — Verdict output and verdict handling are explicit job-runner contracts**
- AC1: Validator results emit `verdict`, `basis_sufficiency`, `enforcement_eligible`, `confidence_bps`, `issues`, and `summary` using the JSON contract shown in this section.
- AC2: Dry-run handling follows the matrix in this section: verdicts are logged, rejected-but-enforcement-eligible results still advance in Layer 3, low-basis verdicts log `coverage_gap`, and crash/timeout/invalid-verdict paths fail open.
- AC3: Layer 4 enforcement behavior remains traceable from the same verdict contract rather than a separate output format.
- AC4: Integration with the existing `agents/contracts.rs` surface is explicit: semantic-validator `approved` aligns with the accepted worker-output path; enforcement-active semantic `rejected` maps to `ValidationDisposition::Quarantined` / `AGENT_SUSPECT_OUTPUT`; `flagged` and `basis_sufficiency: low` remain accepted worker outputs plus trust-state/validation metadata rather than a new worker `ValidationDisposition`.
- AC5: `VALIDATION_FLAGGED` and `VALIDATION_SKIPPED_VALIDATOR_ERROR` are validation-system flags/markers on jobs/verdicts, not new `ErrorCode` enum variants unless the underlying Layer 1 taxonomy is explicitly expanded.
- AC6: Unknown verdict strings, schema-invalid validator outputs, and rendered-but-unpersisted verdicts are treated as validator failure, never silently coerced to `approved`/`flagged`.
- AC7: Verdict persistence commits the `validation_verdicts` row and the worker-job linkage atomically so later audits never observe a half-written semantic-validation state.

> **[IMPL] ✓ As designed** — Validator execution, fail-open marker handling, and atomic verdict persistence are implemented in `crates/daemon/src/validation/executor.rs:25`, `crates/daemon/src/validation/executor.rs:637`, and `crates/daemon/src/validation/executor.rs:1046`.

### 2.5 Outcome Classes

Skills return one of six standard outcome classes (replacing binary success/failure):

| Status | Meaning | Daemon Response |
|--------|---------|----------------|
| `completed` | Full scope finished, all validation passed | Accept results, advance stage |
| `degraded_completed` | Partial but usable results with quality warnings | Accept with degradation flag, surface warnings |
| `continuation_required` | Clean exit at safe boundary with remaining work | Accept committed results, enqueue continuation segment |
| `blocked_human_action` | Human intervention required (CAPTCHA, MFA) | Emit error code, alert operator |
| `suspect_output` | Schema-valid but failed semantic/drift validation | Quarantine, do not promote, surface for review |
| `failed` | Unrecoverable error with failure code | Create error manifest, apply retry/cooldown |

`continuation_required` is distinct from failure — a long-running skill that hits its time budget after 45 notices emits a valid continuation contract. The Job Runner enqueues the next segment. Crashes with only prior checkpoints remain `AGENT_PARTIAL_FAILURE`.

**Continuation chains:** For multi-segment runs, per-segment validation runs independently. The Job Runner does not mark the chain closed or advance the final stage head until chain-close aggregate validation passes (all segments in the chain validated or dry-run cleared).

**L3-FR-2.5.1 — Layer 3 standardizes six outcome classes with explicit daemon responses**
- AC1: Skills return exactly one of `completed`, `degraded_completed`, `continuation_required`, `blocked_human_action`, `suspect_output`, or `failed`.
- AC2: Each outcome class maps to the daemon response stated in the table in this section rather than ad hoc caller-specific behavior.
- AC3: `suspect_output` remains semantically distinct from unrecoverable execution failure.
- AC4: The current `RunManifestFrontmatter::OutcomeClass` enum in `artifacts.rs` directly serializes `completed`, `degraded_completed`, `continuation_required`, and `blocked_human_action`; until that enum expands, `suspect_output` and terminal `failed` remain distinguishable through job/error/validator state rather than being silently collapsed.

**L3-FR-2.5.2 — Continuation chains are validated per segment but closed as one chain**
- AC1: `continuation_required` is treated as a clean boundary with committed work, not as failure.
- AC2: The Job Runner enqueues the follow-on segment when a continuation contract is emitted.
- AC3: The final stage head does not advance until chain-close aggregate validation has passed or been dry-run-cleared for all segments in the chain.

> **[IMPL] ✓ As designed** — The six `OutcomeClass` variants are implemented in `crates/daemon/src/agents/artifacts.rs:19`, including `SuspectOutput` and `Failed`, so the current code now matches the full L3 outcome-class set.

### 2.6 Dry-Run Mode

Dry-run is the default calibration path before enforcement is enabled. Isolation guarantees:

| Guarantee | Enforcement |
|-----------|-------------|
| No authoritative pipeline rows advanced based on verdict | Dry-run verdicts are informational only |
| No outbox entries created from dry-run verdicts | Sync engine never sees dry-run data |
| No stage heads moved based on verdict | Stage advancement follows normal (non-validation) path |
| Artifacts written to `artifacts/scratch/<run_id>/` | Separate from production artifact paths |
| Verdicts logged to `validation_verdicts` table | For calibration analysis only |
| Pipeline continues regardless of verdict | Worker results always advance in dry-run |

**Purpose:** Accumulate a run-manifest corpus for calibration. Track false positive/negative rates. Must reach acceptable rates on gold-set manifests and live manifests before Layer 4 enables enforcement.

**L3-FR-2.6.1 — Dry-run verdicts are isolated from authoritative pipeline advancement**
- AC1: No authoritative pipeline row, outbox entry, or stage head advances because of a dry-run verdict.
- AC2: Dry-run artifacts are written under `artifacts/scratch/<run_id>/`.
- AC3: Verdicts are logged to `validation_verdicts`, and the worker pipeline continues regardless of approval/rejection/flagging.

**L3-FR-2.6.2 — Dry-run exists to build calibration evidence before enforcement**
- AC1: Dry-run accumulates a run-manifest corpus for calibration analysis.
- AC2: The system tracks false-positive and false-negative behavior on both gold-set and live manifests.
- AC3: Layer 4 enforcement remains blocked until the dry-run system reaches acceptable rates on those corpora.

> **[IMPL] ✓ As designed** — Dry-run validation verdict storage is materialized in `migrations/0005_l3_operational_hardening.sql:14`, and schema activation is covered by `crates/daemon/tests/schema_activation.rs:214`.

### 2.7 Fail-Open Semantics (Constitutional)

**Validator failures never block the pipeline.** This is a constitutional invariant — not a temporary expedient.

**Framing:** Fail-open for availability, not fail-open for trust equivalence. Unvalidated results flow through the pipeline but carry lower trust. Downstream behaviour should depend on trust state.

If the validator crashes, times out, or produces an invalid verdict:
- Worker results advance as if validation were skipped
- `VALIDATION_SKIPPED_VALIDATOR_ERROR` flag set on the job
- Error manifest created for the validator failure itself
- Validation system self-monitors its own failure rate
- Unknown verdict enums, schema-invalid validator outputs, and rendered-but-unpersisted verdicts are all classified as validator failure for this rule
- If OQG anomaly inputs were unavailable, the worker result is treated as `unverified` with explicit missing-signal context rather than silently upgraded
- Result queued in **retroactive validation backlog** for later backfill (see below)

The rationale: a broken validator is worse than no validator. The pipeline must never stall because the quality-checking subsystem fails. The validator adds confidence; it does not gate availability.

**Trust-state lattice:**

Every pipeline result carries a trust state that propagates downstream:

| State | Meaning | Downstream Behaviour |
|-------|---------|---------------------|
| `verified` | Validator approved (or enforcement not active and OQG clean) | Full trust — all downstream actions allowed |
| `flagged` | Validator flagged for human review | Manifests generated; rollups allowed but surfaced as flagged; operator review recommended |
| `unverified` | Validation skipped (fail-open, dry-run, or no policy) | Manifests generated; rollups surfaced as unverified; bid workflow requires explicit human review |
| `suspect` | Validator rejected (or suspect output from OQG) | Quarantined in enforcement mode; in dry-run, surfaced for review with high-priority trust debt |

**Process-level surface mapping:** The trust-state lattice is finer-grained than the operator-facing `pipeline_assessments.validation_state` field. `pipeline_assessments.validation_state` remains the coarse process-level rollup exposed to desktop/web and therefore stays constrained to `validated` / `flagged` / `unverified`. Mapping rule:
- selected-scope outputs all `verified` → process rollup `validated`
- any selected-scope `flagged` or `suspect` output participating in the current rollup → process rollup `flagged`
- validation skipped / insufficient basis / fail-open path → process rollup `unverified`

Downstream behaviour by trust state:
- **Manifest generation:** allowed at all trust states
- **Internal rollups:** allowed, but surfaced externally through the coarse `pipeline_assessments.validation_state` mapping above
- **Decision recommendations:** lowered weight for `unverified`/`suspect`
- **Bid workflow / external actions:** require a clean upstream trust gate for the selected scope plus explicit process-level `validation_state = validated`, or an explicit operator override
- **Reusable caches** (`listing_evidence`, relation cache sharing): do not allow `suspect` outputs to seed caches at full trust; carry trust metadata

**Validator health states (subsystem-level):**

Beyond per-run failure flags, the validation subsystem maintains an aggregate health state:

| State | Meaning | Trigger |
|-------|---------|---------|
| `healthy` | Validator operating normally | Default; canary checks passing |
| `degraded` | Elevated failure rate or latency | Validator execution failure rate >20% over 1h, or timeout spike, or basis-insufficiency spike |
| `blind` | Validator effectively non-functional | Validator execution failure rate >50% over 1h, canary misclassification, model/provider unavailable |

When `blind`:
- Pipeline still runs (fail-open)
- Enforcement auto-demotes on all affected slices
- Desktop and web show explicit **validation blackout banner** with affected time window
- Retroactive validation backlog starts accumulating
- Operator morning queue shows one prominent incident with sorted trust-debt queue

**Synthetic validator canaries:**

Run the validator path every 15-30 minutes with:
- One known-good manifest (must approve)
- One known-bad manifest (must reject)
- One known-insufficient-basis manifest (must declare insufficient)

These use the same executor, same model family, same overlay loading, same schema validation path as production. A misclassification triggers transition from `healthy` to `degraded` or `blind`. A broken validator should be noticed in minutes, not after 500 unverified rows at 9 AM.

**Retroactive validation backlog:**

Every result that passes through due to validator failure is queued for later backfill validation. Backlog priority:
1. High-value opportunities
2. Operator-reviewed processes (decisions may be based on bad data)
3. Slices currently under enforcement (trust regression)
4. Recent outputs (freshest data, most actionable)

When the validator recovers:
- Validate backlog items
- If later rejected: mark affected underlying outputs as `suspect`, surface affected process rollups as `flagged`, and flag decisions as stale
- Push updated status to web portal
- Surface "validated retroactively: N issues found" in operator dashboard

Backlog items do **not** silently expire in Layer 3. They remain queued until successfully revalidated or explicitly marked expired by a later audited retention policy; operator-reviewed and high-value items are non-expiring in Layer 3 semantics.

**L3-FR-2.7.1 — Validator failures never block worker availability**
- AC1: If validation crashes, times out, or produces an invalid verdict, worker results advance as if validation were skipped.
- AC2: The daemon records `VALIDATION_SKIPPED_VALIDATOR_ERROR` and creates an error manifest for the validator failure itself.
- AC3: Every fail-open pass-through is queued for retroactive validation rather than being silently forgotten.
- AC4: Host-sleep-interrupted validator sub-jobs are treated as validator failures for fail-open purposes rather than blocking or silently certifying the worker result.
- AC5: Unknown verdict values, schema-invalid validator outputs, and rendered-but-unpersisted verdicts all count as validator failures for fail-open handling.

**L3-FR-2.7.2 — Every output carries a trust state with downstream consequences**
- AC1: Trust states are `verified`, `flagged`, `unverified`, and `suspect`.
- AC2: Manifest generation is allowed at all trust states, but bid/external actions require a clean trust gate or explicit operator override as stated in this section.
- AC3: `suspect` outputs may not seed reusable caches at full trust, and `listing_evidence`/relation-cache reuse carries trust metadata forward.

**L3-FR-2.7.3 — Process-level validation rollups remain coarser than per-output trust**
- AC1: `pipeline_assessments.validation_state` is constrained to `validated`, `flagged`, or `unverified`.
- AC2: All selected-scope outputs verified maps to `validated`; any contributing `flagged` or `suspect` output maps to `flagged`; skipped/insufficient-basis/fail-open paths map to `unverified`.
- AC3: Process rollup state must not be treated as identical to the finer-grained trust-state lattice.

**L3-FR-2.7.4 — Validator subsystem health is tracked as `healthy`, `degraded`, or `blind`**
- AC1: `degraded` triggers at validator execution failure rate > `20%` over `1h`, timeout spike, or basis-insufficiency spike.
- AC2: `blind` triggers at validator execution failure rate > `50%` over `1h`, canary misclassification, or model/provider unavailability.
- AC3: When `blind`, enforcement auto-demotes, a validation blackout banner is surfaced, retroactive backlog accumulates, and the operator morning queue shows one prominent blackout incident.

**L3-FR-2.7.5 — Synthetic canaries and retroactive backlog recovery keep fail-open auditable**
- AC1: The validation path runs synthetic canaries every `15-30` minutes using one known-good, one known-bad, and one known-insufficient-basis manifest.
- AC2: Canary misclassification transitions health from `healthy` toward `degraded` or `blind` quickly enough that overnight blind spots are detected within minutes.
- AC3: Retroactive validation backlog prioritizes high-value opportunities, operator-reviewed processes, slices under enforcement, then recent outputs.
- AC4: On validator recovery, backlog revalidation can mark underlying outputs `suspect`, re-flag process rollups, stale affected decisions, push updated portal status, and surface a recovery summary in the operator dashboard.
- AC5: Backlog items do not silently expire in Layer 3; any future expiry/pruning action must be explicit and auditable.

> **[IMPL] ⚠ Deviation:** Actual implementation (2026-05-02): the validation health and fail-open code paths are implemented, but L4-facing portal push behavior in the retroactive recovery bullets is not built because L4 sync/web is still planned. Current evidence is local daemon behavior in `crates/daemon/src/validation/executor.rs`, `crates/daemon/src/validation/policy.rs`, and `crates/daemon/tests/validation_canary_health.rs`.

### 2.8 Base Validator Skill

**Directory:** `skills/validation/base/`

**Files:** `SKILL.md`, `result.schema.json`, `checkpoint.schema.json`, `canary.inputs.jsonl`, `golden.assertions.yaml`, `validator-rubric.yaml`, `references/manifest-corpus-index.md`

**What the base validator checks:**

1. **Manifest completeness:** All required frontmatter fields present, narrative sections non-empty, daemon appendix present
2. **Agent-vs-daemon agreement:** Cross-check `agent_claimed_*` against `daemon_observed_*` — discrepancies in fingerprint state, locale, currency, item count are high-signal
3. **Result contract consistency:** Manifest claims consistent with result contract sample (item counts, value ranges, source URLs)
4. **Narrative plausibility:** Steps described match the observed URL chain and page-kind sequence
5. **`basis_sufficiency` assessment:** Can the validator render a confident verdict? If the daemon appendix is thin (e.g., `no_baseline` fingerprint, missing OQG signals), the validator declares `basis_sufficiency: low` rather than guessing
6. **Drift detection:** Current run's fill rate, fallback depth, and error patterns compared to recent runs via structured drift deltas

**What the base validator does NOT check:**
- Domain-specific business rules (those come from overlay packages)
- Result contract schema compliance (already checked by OQG)
- Raw forensic bytes (too large, too noisy)

**L3-FR-2.8.1 — The base validator skill directory is explicitly specified**
- AC1: `skills/validation/base/` includes `SKILL.md`, `result.schema.json`, `checkpoint.schema.json`, `canary.inputs.jsonl`, `golden.assertions.yaml`, `validator-rubric.yaml`, and `references/manifest-corpus-index.md`.
- AC2: The directory contract is sufficient to execute runtime validation plus synthetic canaries without inventing undocumented validator assets.
- AC3: Validator skills remain standard skill directories rather than bespoke daemon-only plugins.

**L3-FR-2.8.2 — The base validator checks generic semantic quality and explicitly excludes non-generic concerns**
- AC1: The base validator checks manifest completeness, agent-vs-daemon agreement, result-sample consistency, narrative plausibility, `basis_sufficiency`, and drift deltas.
- AC2: When appendix evidence is thin, the validator returns `basis_sufficiency: low` rather than guessing.
- AC3: Domain-specific business rules, schema compliance already enforced by OQG, and raw forensic bytes are out of scope for the base validator.

### 2.9 Initial Calibration (Layer 3)

Before enforcement can be enabled in Layer 4, the dry-run system must demonstrate adequate precision:

**Calibration data sources:**
- Gold-set manifests from Layer 0 (known-good extractions)
- Live manifests from Layer 1-2 operation
- Mutated manifests (synthetically corrupted for known-bad testing)

**Calibration corpus structure (3 corpora):**

| Corpus | Purpose | Source |
|--------|---------|--------|
| **Clean real** | Establish baseline approval rates | Live manifests from successful runs |
| **Weird-but-valid** | Test that unusual-but-correct runs aren't rejected | Edge cases from gold set |
| **Mutated bad** | Test that corrupted data is caught | Synthetically mutated manifests |

**Calibration targets (must-reach before L4 enforcement):**
- Validator must correctly identify ≥90% of mutated-bad manifests
- Validator must correctly approve ≥95% of clean-real manifests
- False positive rate (rejecting good data) must be <5%
- Wilson-bound precision gates must hold at 95% confidence

**Cross-model audit:** A subset of calibration runs should use a different model than the primary validator to detect model-specific blind spots.

**Override feedback loop:** Every operator override of a validator verdict (operator approves what validator rejected, or vice versa) becomes a calibration case for the next calibration cycle. Note: overrides are selection-biased toward visible/contested cases — use as demotion signal, incident source, and mutation template source, but not as sole precision estimator.

**Realistic calibration timeline:**

For a given slice, the bottleneck is almost always the 30 confirmed reject cases, not the approvals.

Formula: `weeks_to_30_rejects = 30 / (runs_per_week × validation_rate × reject_yield × adjudication_rate)`

| Slice Profile | Runs/Week | Validation Rate | Reject Yield | Weeks to 30 Rejects | Weeks to 60 Audited Approvals | Earliest Enforcement |
|---------------|-----------|-----------------|--------------|---------------------|-------------------------------|---------------------|
| Hot slice | 100 | 50% | 15% | ~4 | ~1.4 | ~4–6 weeks |
| Warm slice | 40 | 50% | 10% | ~15 | ~3.3 | ~15–18 weeks |
| Long-tail slice | 10 | 100% | 10% | ~30 | ~6.7 | ~30+ weeks |

**Explicit product truth:** Many slices will remain permanently dry-run / flag-only because their traffic volume is too low to accumulate enforcement-grade evidence before the worker or overlay changes. This is not a flaw — it is an expected operational steady state. The dry-run bootstrap validation ladder (§2.3) and signature-limited enforcement allow partial enforcement value even for slices that never reach full gate criteria.

**Two calibration gates (not one):**

| Gate | Purpose | Data Required |
|------|---------|---------------|
| **A. Overlay/family prequalification** | Prove the validator architecture is sane for this skill family | 20–30 clean-real + 15–20 weird-but-valid + 30–50 mutated-bad (offline corpus) |
| **B. Exact-slice enforcement readiness** | Prove this exact worker/source/version combination is trustworthy | 30 adjudicated rejects + 60 audited approvals from live traffic (current gate) |

Gate A is reusable across versions within a skill family. Gate B is per-slice. Gate A must pass before Gate B evidence acquisition begins.

**L3-FR-2.9.1 — Calibration uses three explicit corpora**
- AC1: Calibration data includes `clean real`, `weird-but-valid`, and `mutated bad` corpora.
- AC2: Clean-real cases come from successful live manifests, weird-but-valid cases come from gold-set edge cases, and mutated-bad cases come from synthetically corrupted manifests.
- AC3: Calibration reporting preserves which corpus produced each judgment so false positives and false negatives can be attributed correctly.

**L3-FR-2.9.2 — Calibration targets and audit inputs are numerically defined**
- AC1: Before Layer 4 enforcement, the validator must catch at least `90%` of mutated-bad manifests, approve at least `95%` of clean-real manifests, keep false positives below `5%`, and satisfy Wilson-bound precision gates at `95%` confidence.
- AC2: A subset of calibration runs uses a different model than the primary validator to detect model-specific blind spots.
- AC3: Every operator override becomes an input to the next calibration cycle, but override data is treated as selection-biased rather than the sole precision estimator.

**L3-FR-2.9.3 — Enforcement readiness requires both reusable family proof and exact-slice proof**
- AC1: Gate A (overlay/family prequalification) requires `20-30` clean-real, `15-20` weird-but-valid, and `30-50` mutated-bad offline cases.
- AC2: Gate B (exact-slice enforcement readiness) requires `30` adjudicated rejects and `60` audited approvals from live traffic for the exact slice.
- AC3: Gate A must pass before Gate B evidence acquisition begins.
- AC4: Long-tail slices may remain permanently dry-run or flag-only, and that steady state is explicitly acceptable rather than treated as a spec failure.

> **[IMPL] ✗ Deferred** — Formal three-corpus enforcement calibration remains L4-blocked. L3 has validator policy/executor/schema surfaces and canary fixtures, but the full adjudicated enforcement-readiness corpus and cross-model audit workflow described here are not production gates as of 2026-05-02.

---

## 3. Skill Promotion, Source Health & Relations

> **Ownership:** This section owns the skill promotion flow, source health monitoring, source circuit breaker, relation resolution logic, and browser automation reliability patterns. Skill directory standard and metadata declarations are owned by L0 §4. Worker-validator system is owned by §2 above. Schema (new tables for relations/source state) is owned by §4 below.

### 3.1 Six-Stage Skill Promotion Flow

Every skill follows a linear promotion path. No stage can be skipped. Skills in pre-stable stages always validate regardless of declared `validation_policy`.

| Stage | Entry Criteria | What Happens | Exit Criteria |
|-------|---------------|-------------|---------------|
| **`draft`** | Skill directory created with SKILL.md | Manual exploration via Kickoff Prompt Renderer. No scheduler binding, no pipeline state commits. Dry-run mode only. | CI lint passes (directory structure, metadata fields, schema validity). Smoke canary passes on at least one input. |
| **`candidate`** | CI lint + smoke canaries pass | Skill runs in dry-run mode against scratch workspace. Run manifests generated but not authoritative. Validator sub-jobs always spawned. | All canary inputs produce valid result contracts. Golden assertions pass. Validator approves >=90% of runs. |
| **`canary`** | Candidate exit criteria met | Small percentage of production traffic (or single source shard) routed to canary version. Results committed to pipeline state alongside stable results. Quality metrics compared against stable baseline. | Quality metrics within acceptable variance of stable baseline for >=20 runs. No regression on precision/recall vs gold set. Validator approval rate >=95%. |
| **`shadow`** | Canary exit criteria met | Runs in parallel with stable version on full production traffic. Both versions produce results; only stable results are authoritative. Quality metric diff computed. | Shadow quality >= stable quality (or within configured tolerance). Fill-rate, precision, recall stable over >=50 parallel runs. |
| **`stable`** | Shadow exit criteria met | Scheduler default. All production jobs bind this version. Results are authoritative. | Remains stable until replaced by a newer version completing the promotion path. |
| **`rollback`** | Operator-initiated or automated quality regression detected | Scheduler rebinds the source to the prior stable version. Current stable demoted. | Rollback version verified operational. Failed version returned to candidate for investigation. |

**Promotion gates include validation pass rates.** Validator approval rate is a first-class promotion gate alongside quality metrics. A skill cannot promote from canary to shadow unless the worker-validator system approves at threshold rates.

**L3-FR-3.1.1 — `draft` is manual-only, non-authoritative exploration**
- AC1: `draft` begins when a skill directory exists with `SKILL.md`.
- AC2: `draft` work runs via manual exploration only, with no scheduler binding and no authoritative pipeline-state commits.
- AC3: Exit from `draft` requires CI lint to pass plus at least one smoke-canary success.

**L3-FR-3.1.2 — `candidate` runs in dry-run with mandatory validation**
- AC1: `candidate` runs use scratch/dry-run workflow surfaces rather than authoritative production state.
- AC2: Candidate runs always generate run manifests and always spawn validator sub-jobs.
- AC3: Exit from `candidate` requires valid result contracts for all canary inputs, passing golden assertions, and validator approval on at least `90%` of runs.

**L3-FR-3.1.3 — `canary` routes a bounded portion of production traffic to the candidate version**
- AC1: `canary` traffic is a small percentage of production load or one source shard, not full production.
- AC2: Canary results are committed alongside stable results so quality comparison is grounded in production reality.
- AC3: Exit from `canary` requires acceptable variance versus stable for at least `20` runs, no precision/recall regression versus the gold set, and validator approval rate at or above `95%`.

**L3-FR-3.1.4 — `shadow` compares full-traffic behavior while keeping stable authoritative**
- AC1: In `shadow`, the candidate and stable versions run in parallel on full production traffic.
- AC2: Only the stable version remains authoritative during the shadow period.
- AC3: Exit from `shadow` requires shadow quality at least matching stable quality or staying within configured tolerance across at least `50` parallel runs.

**L3-FR-3.1.5 — `stable` is the scheduler default and authoritative production binding**
- AC1: `stable` is the default version bound by the scheduler for that skill/source.
- AC2: Stable results are authoritative production outputs until another version completes the full promotion path.
- AC3: Stable status does not exempt the skill from source health, telemetry, or validation-policy rules elsewhere in this document.

**L3-FR-3.1.6 — `rollback` rebinds traffic to the prior stable version**
- AC1: `rollback` is entered by operator action or automated quality-regression detection.
- AC2: Entering `rollback` rebinds the scheduler to the prior stable version and demotes the failed version.
- AC3: Exit from `rollback` requires the rollback version to be verified operational and the failed version to return to `candidate` for investigation.

**L3-FR-3.1.7 — Promotion is linear and validator approval is a first-class gate**
- AC1: No stage in the promotion flow may be skipped.
- AC2: Promotion gates include validator approval rate alongside fill-rate/precision/recall quality metrics.
- AC3: A skill may not promote from `canary` to `shadow` unless validator approval meets the declared threshold.

> **[IMPL] ⚠ Deviation:** Actual implementation (2026-05-02): promotion-stage and version metadata structures exist in the source registry (`crates/daemon/src/sources/registry.rs:103`), but a complete automated promotion controller is not a production workflow yet. Current scheduling binds stable/default source definitions; future promotion work should extend the registry surfaces rather than assuming this six-stage process is already fully automated.

### 3.2 Five Independent Version Dimensions

Skill versioning tracks five dimensions independently. Only `skill_version` is manually bumped.

| Dimension | What It Tracks | Bumped By | Example |
|-----------|---------------|-----------|---------|
| `skill_version` | SKILL.md playbook content, navigation logic, extraction rules | Manual (author edits SKILL.md) | `2026.03.15-1` |
| `result_schema_version` | Shape of the result contract (JSON Schema) | Manual (when output fields change) | `1.2` |
| `checkpoint_schema_version` | Shape of checkpoint payloads | Manual (when checkpoint format changes) | `1.0` |
| `validator_overlay_version` | Domain-specific validation criteria | Manual (when rubric changes) | `2026.03.12-1` |
| `source_structure_version` | Source website layout fingerprint | Automated (drift detection) | `fts-2026-03-19` |

**Key rule:** A playbook update does NOT force a schema version bump unless the result contract shape changes. Schema migrations use dual-accept windows during transition.

**Behavior version derivation:** For enforcement slice tracking, `worker_behavior_version` is derived from `skill_version` + `result_schema_version` (changes that affect output semantics). `validator_behavior_version` is derived from `validator_overlay_version`. Cosmetic-only skill edits (observability improvements, prompt formatting, cost tuning) do NOT bump behavior version and therefore do NOT reset slice enforcement readiness. The skill metadata must declare whether a version bump is behavior-affecting.

**L3-FR-3.2.1 — Layer 3 tracks five version dimensions independently**
- AC1: Version tracking covers `skill_version`, `result_schema_version`, `checkpoint_schema_version`, `validator_overlay_version`, and `source_structure_version`.
- AC2: Only `skill_version` is manually bumped for playbook-content changes; schema and overlay versions change only when their specific contracts change.
- AC3: A playbook-only edit does not force schema-version churn unless the result or checkpoint shape changes.

**L3-FR-3.2.2 — Behavior-version tracking protects calibration evidence from cosmetic churn**
- AC1: `worker_behavior_version` derives from semantic worker changes (`skill_version` + `result_schema_version`), and `validator_behavior_version` derives from `validator_overlay_version`.
- AC2: Cosmetic prompt edits, observability-only edits, and cost-tuning edits do not reset behavior-version-based readiness.
- AC3: Skill metadata must declare whether a version change is behavior-affecting so slice enforcement readiness can be evaluated mechanically.

> **[IMPL] ✓ As designed** — The source registry models independent version dimensions and behavior-version fields in `crates/daemon/src/sources/registry.rs:103`, with runtime source loading in `crates/daemon/src/sources/registry.rs:210`.

### 3.3 New Skills Introduced in Layer 3

**Validator base skill files (7 total: 6 standard + 1 reference index):**

| File | Purpose |
|------|---------|
| `skills/validation/base/SKILL.md` | Generic run manifest quality checks — narrative consistency, agent-claimed vs daemon-observed cross-check, result contract plausibility, `basis_sufficiency` assessment |
| `skills/validation/base/result.schema.json` | Verdict output schema (`approved`/`rejected`/`flagged` with metadata) |
| `skills/validation/base/checkpoint.schema.json` | No-op checkpoint schema; validators do not emit checkpoints in Layer 3 |
| `skills/validation/base/canary.inputs.jsonl` | Known-good and known-bad manifests for smoke testing |
| `skills/validation/base/golden.assertions.yaml` | Invariant assertions (mutated-bad must be caught, clean-real must pass) |
| `skills/validation/base/validator-rubric.yaml` | Distilled runtime rubric — skill semantic invariants consumed by the Validation Executor at runtime (NOT raw golden assertions injected into live prompts) |
| `skills/validation/base/references/manifest-corpus-index.md` | Calibration/corpus provenance index |

**Operational skills (4):**

| Skill | Path | Purpose |
|-------|------|---------|
| Auth refresh (Alibaba) | `skills/auth-refresh/alibaba/` | Establish/refresh authenticated session state for Alibaba.com. Publishes auth-state handle for enrichment skills. |
| Auth refresh (1688) | `skills/auth-refresh/1688/` | Establish/refresh authenticated session state for 1688.com. Publishes auth-state handle. |
| Source health (FTS) | `skills/source-health/fts/` | Pre-flight canary: verify FTS reachability, page fingerprint, no challenge markers, OCDS endpoint responsive. |
| Source health (Alibaba) | `skills/source-health/alibaba/` | Pre-flight canary: verify Alibaba reachability, geo-correctness, auth state, page fingerprint, challenge rate. |

Auth-refresh skills do one thing: establish authenticated state and publish an auth-state handle. Source-health canaries run before expensive scheduled jobs to detect whether a source is reachable and not actively challenging.

**L3-FR-3.3.1 — Layer 3 skill families have explicit operational roles**
- AC1: The base validator skill performs generic run-manifest quality checks and emits verdicts through the validator contract.
- AC2: Auth-refresh skills establish and publish authenticated session state without taking on unrelated extraction responsibilities.
- AC3: Source-health canaries prove reachability, fingerprint/auth/locale sanity, and minimal extraction before expensive scheduled work is allowed to proceed.

### 3.4 Source Health Monitoring

Source-health canary skills run at higher frequency than production jobs. Each canary checks:

| Check | Purpose | Failure Classification |
|-------|---------|----------------------|
| **Reachability** | Source responds at all | `SOURCE_UNREACHABLE` |
| **Page fingerprint** | Expected page structure present (required anchors, no forbidden anchors) | `SOURCE_LAYOUT_CHANGED` |
| **Auth state** | Authenticated session still valid (where applicable) | `AGENT_AUTH_CHALLENGE` |
| **Geo/locale** | Correct locale and currency (e.g., `en-GB`, `GBP` for eBay UK) | `SOURCE_LOCALE_DRIFT` |
| **Challenge rate** | No CAPTCHA, rate-limit, or bot-detection triggers | `SOURCE_BLOCKED` |
| **Minimal extraction** | Can extract at least one valid item from a known query | `SOURCE_EXTRACTION_FAILED` |

**Scheduling:** Source-health canaries run every 30 minutes for active sources. Production ingestion/enrichment jobs are suppressed if the most recent canary failed.

**Error-code taxonomy integration:** Layer 3 does not create a second authoritative error-code enum. The current Layer 1 / `errors/manifest.rs` surface carries **28** concrete `ErrorCode` variants in the checked-in daemon code as of **March 29, 2026**. The canary classifications in this section are logical source-health classes that map onto those existing error codes plus structured `error_context`, rather than silently inventing unimplemented enum variants.

| Logical Canary Classification | Existing `ErrorCode` Surface | Mapping Rule |
|------------------------------|------------------------------|--------------|
| `SOURCE_LAYOUT_CHANGED` | `PAGE_SHAPE_DRIFT` | Preferred top-level code for fingerprint/layout drift |
| `SOURCE_LOCALE_DRIFT` | `LOCALE_OR_CURRENCY_DRIFT` | Preferred top-level code for wrong market/locale/currency |
| Hard challenge / human gate | `SOURCE_CAPTCHA_BLOCKED` or `AGENT_AUTH_CHALLENGE` | Use the existing challenge/auth code that best matches the observed blocker |
| Soft block / rate limit / no usable data | `SOURCE_SOFT_BLOCKED` | Carry finer-grained class such as `source_blocked` or `source_rate_limited` in structured context |
| Extraction reached but minimal contract failed | `AGENT_SKILL_FAILED` | Use when the source is reachable but the canary cannot satisfy its extraction contract |
| Transport / reachability failure with no better source-specific code | `AGENT_RUN_FAILED` | Carry `source_unreachable` in structured context until the enum grows a dedicated reachability code |

**L3-FR-3.4.1 — Source-health canaries execute a fixed six-check health contract**
- AC1: Each canary checks reachability, page fingerprint, auth state where applicable, geo/locale correctness, challenge rate, and minimal extraction.
- AC2: Each failed check maps to an explicit failure classification: `SOURCE_UNREACHABLE`, `SOURCE_LAYOUT_CHANGED`, `AGENT_AUTH_CHALLENGE`, `SOURCE_LOCALE_DRIFT`, `SOURCE_BLOCKED`, or `SOURCE_EXTRACTION_FAILED`.
- AC3: The canary surface is lightweight enough to run ahead of scheduled production work rather than as a substitute for full ingestion.
- AC4: The named failure classifications in this section are logical source-health classes; they map onto the existing 28-variant `ErrorCode` surface plus structured context rather than introducing hidden enum additions.

**L3-FR-3.4.2 — Source-health runs on a higher-frequency cadence and suppresses unhealthy production work**
- AC1: Active-source canaries run every `30` minutes by default.
- AC2: If the most recent canary failed or timed out, production ingestion/enrichment jobs for that source are suppressed until health recovers.
- AC3: Canary success, failure, and timeout outcomes are all persisted as explicit source-runtime signals rather than inferred later from missing work.

> **[IMPL] ✓ As designed** — Source runtime health state and canary/circuit fields are implemented by the migration in `migrations/0005_l3_operational_hardening.sql:3`, loaded through `crates/daemon/src/sources/registry.rs:210`, and exposed as status snapshots by `crates/daemon/src/main.rs:2397`.

### 3.5 Source Circuit Breaker

Repeated challenge-class failures trigger a circuit breaker that protects the source account/IP.

**Three states:**

```text
CLOSED (healthy)
  → repeated failures → HALF_OPEN (canary-only)
  → canary fails → OPEN (paused)
  → canary succeeds → CLOSED

HALF_OPEN
  → canary succeeds → CLOSED
  → canary fails → OPEN

OPEN
  → cooldown expires → HALF_OPEN
  → manual reset → HALF_OPEN
```

| State | Allowed Jobs | Duration |
|-------|-------------|----------|
| `closed` | All jobs for this source | Normal operation |
| `half_open` | Only `source-health/*` and `auth-refresh/*` canary/auth jobs | Until canary succeeds |
| `open` | No jobs for this source | Configurable cooldown (default 2 hours), then auto-transitions to `half_open` |

**Trigger criteria:** Circuit opens after N consecutive challenge-class failures (default N=3) within a time window. Challenge-class failures include `AGENT_AUTH_CHALLENGE`, `SOURCE_CAPTCHA_BLOCKED`, and `SOURCE_SOFT_BLOCKED` when structured context classifies the blocker as `source_blocked` or `source_rate_limited`.

**State persisted in:** `source_runtime_state` table (local-only — see §4 for DDL).

**L3-FR-3.5.1 — `closed` source circuits permit normal scheduling**
- AC1: In `closed`, all jobs for the source remain eligible for normal scheduling.
- AC2: `closed` represents healthy operation until challenge-class failures accumulate past the configured threshold.
- AC3: Transition out of `closed` is driven by the explicit breaker rules in this section rather than ad hoc operator judgment.

**L3-FR-3.5.2 — `half_open` source circuits are canary-only recovery mode**
- AC1: In `half_open`, only `source-health/*` and `auth-refresh/*` jobs are allowed for that source.
- AC2: A successful canary from `half_open` transitions the source back to `closed`.
- AC3: A failed or timed-out canary from `half_open` transitions the source to `open`.

**L3-FR-3.5.3 — `open` source circuits pause normal work until cooldown expiry or manual reset**
- AC1: In `open`, no normal jobs for the source are admitted.
- AC2: `open` lasts for a configurable cooldown with default duration `2 hours`.
- AC3: Cooldown expiry or manual reset transitions the source to `half_open`, not directly to `closed`.

**L3-FR-3.5.4 — Breaker trigger criteria and state persistence are explicit**
- AC1: The circuit opens after `N=3` consecutive challenge-class failures within the configured time window unless configuration overrides the default.
- AC2: Challenge-class failures include `AGENT_AUTH_CHALLENGE`, `SOURCE_CAPTCHA_BLOCKED`, and `SOURCE_SOFT_BLOCKED` with structured blocker class `source_blocked` or `source_rate_limited`.
- AC3: `source_runtime_state` persists the last health state, circuit state, challenge-window counters, cooldown timestamps, latest canary outcome, and last error code so state transitions are auditable.
- AC4: Circuit transitions execute transactionally per `source_id`; newer canary timestamps win over stale writes, and a stale success may not silently close a more recent `open`/`half_open` state.

> **[IMPL] ✓ As designed** — Circuit state semantics are implemented in `crates/daemon/src/sources/registry.rs:20`, effective-source suppression in `crates/daemon/src/sources/registry.rs:264`, and scheduler suppression in `crates/daemon/src/scheduler/cron.rs:140`.

### 3.6 Relation Resolution Engine

**Module:** `packages/intelligence/src/relations/resolve-process-relations.ts`

**Purpose:** Classify notice-to-process relations after normalization. Runs as stage 3 of the pipeline.

**Four classifications:**

| Classification | Meaning | Action |
|---------------|---------|--------|
| `merged_same_process` | Same OCID or explicit linked notice chain | Merge into single `ProcurementProcess`; retired ID gets `process_aliases` redirect |
| `related_not_merged` | Framework/call-off, planning/tender, same buyer + subject but different OCIDs | Create `ProcessRelation` edge for cache sharing; keep separate processes |
| `possible_duplicate` | High-similarity pair but not confident enough for auto-merge | Hold for operator review; create `ProcessRelation` edge |
| `separate` | No meaningful relationship | No action |

**Resolution algorithm:**

1. **Hard blockers check** — contradictions that prevent merging regardless of similarity:
   - Conflicting exact reference numbers
   - Framework establishment vs call-off/award mismatch
   - Explicit different OCIDs with separate-process semantics (e.g., direct award after unsuccessful competition)
   - Incompatible stage chronology
   - Same buyer but clearly different lot/value/reference structure

2. **Exact-match rules** — deterministic classification:
   - Same OCID → `merged_same_process`
   - Explicit related-notice reference with plausible stage sequence → `merged_same_process`
   - Framework/call-off relation detected → `related_not_merged`

3. **Heuristic scoring** (non-blocked, non-exact pairs only):

   The MVP weight vector is deterministic and versioned as `relation_model_version = 1`. The evaluated feature vector, blockers hit, model version, and threshold bucket are persisted in `process_relations.evidence` (or merge-audit metadata) so the same input pair yields the same replayable decision.

   ```
   merge_score =
     0.35 * reference_match
   + 0.20 * buyer_match
   + 0.20 * title_similarity
   + 0.10 * cpv_overlap
   + 0.10 * date_proximity
   + 0.05 * value_proximity
   ```

   - >= 0.95 with no blocker → `merged_same_process`
   - 0.75–0.94 → `related_not_merged` or `possible_duplicate` (for operator review)
   - < 0.75 → `separate`

4. **Edge creation** — `related_not_merged` and `possible_duplicate` pairs always get a `ProcessRelation` edge enabling cache sharing of search results and listing evidence across the cluster. `merged_same_process` may persist an audit edge, but `separate` writes no `process_relations` row.

5. **Alias management** — when a previously published process ID is retired by a merge, write a `process_aliases` redirect so desktop/web deep links remain stable.

6. **Cluster identity assignment** — every `ProcurementProcess` carries a `canonical_process_id` (defaults to self). On merge, resolve both inputs to their current canonical roots first, then choose the surviving root deterministically (`earliest created_at`, UUID tie-breaker). The retired process's `canonical_process_id` points to the root. Rollups, manifests, and deep links resolve through `canonical_process_id`, not raw `process_id`. The `process_aliases` table handles redirects, but `canonical_process_id` is the full merge model.

7. **Late merge as decision-staleness event** — if two already-enriched processes merge after operator review:
   - All relevant `pending_operator_intents` / `operator_decision_heads` entries are marked stale (`decision_stale = true`)
   - Manifest is regenerated for the merged cluster
   - `pipeline_assessments` recomputed at cluster level
   - Operator sees explicit "identity changed after review" notification
   - This prevents technically correct sync with semantically wrong decisions

   If no operator decision or pending intent exists yet, the engine still regenerates the cluster rollup/manifest but does **not** emit a spurious stale-decision incident.

**New tables:** `process_relations` (push-owned) and `process_aliases` (push-owned). See §4 for full DDL.

**Conservative thresholds:** Default thresholds are deliberately conservative. False merges are harder to undo than false separations. The operator can manually merge `possible_duplicate` pairs.

> **[IMPL] ⚠ Deviation:** Actual implementation (2026-05-02): the relation stage is much narrower than this full four-classifier design. `crates/daemon/src/main.rs:19175` only creates `merged_same_process` relations for processes sharing the same canonical OCID, and `docs/pipeline/multi-source-ingestion-validation-manifest.md` records that Contracts Finder and ProContract produced zero relations because their OCID namespaces were disjoint. Do not assume title/CPV/value heuristic merging exists yet.

**Weight variation by context:** The global weights above are a bootstrap default. Weights should eventually vary by source-pair and notice-stage-pair:
- Planning → tender → award: values drift significantly across lifecycle stages
- Call-offs share buyer/CPV/title but may differ on reference semantics
- FTS vs Contracts Finder duplicates may normalise references differently

For MVP, the global weights are acceptable. A simple interpretable logistic model per source-pair / stage-pair fitted from adjudicated pairs can replace them later — keep the rule engine and hard blockers, fit only the soft weights.

**Additional features for future improvement:** Normalised buyer registration/authority identifier, contact email/domain similarity, lot-count similarity, postcode/region, procurement method / notice type compatibility, reference edit distance after stripping amendment/lot suffixes, document hash overlap if available.

**UI grouping before perfect merge:** To reduce false non-merge cost without destructive auto-merge, group `related_not_merged` and `possible_duplicate` in the desktop UI as a cluster card with relation confidence. Share caches cautiously within the cluster. Let the operator see one cluster view with constituent processes.

**L3-FR-3.6.1 — `merged_same_process` is reserved for true single-process identity**
- AC1: `merged_same_process` applies when notices share the same OCID or an explicit related-notice chain with plausible stage sequence proves one process.
- AC2: Auto-merge is allowed only when no hard blocker contradicts the merge.
- AC3: A merge retires the losing process identity through alias/canonical-process handling rather than silently deleting history.

**L3-FR-3.6.2 — `related_not_merged` preserves relational context without destructive merge**
- AC1: `related_not_merged` applies to framework/call-off, planning/tender, or same-buyer/same-subject relationships that should share context but remain separate processes.
- AC2: `related_not_merged` pairs create `ProcessRelation` edges for cache/context sharing.
- AC3: `related_not_merged` processes keep separate authoritative process identities.

**L3-FR-3.6.3 — `possible_duplicate` holds high-similarity uncertainty for operator review**
- AC1: `possible_duplicate` applies to high-similarity pairs that are not confident enough for auto-merge.
- AC2: `possible_duplicate` creates a relation edge and explicitly queues operator review rather than forcing a merge.
- AC3: Borderline similarity is biased toward `possible_duplicate` instead of false merge.

**L3-FR-3.6.4 — `separate` is the explicit no-relationship classification**
- AC1: `separate` applies when no meaningful relationship exists.
- AC2: `separate` performs no merge and creates no cache-sharing relation edge.
- AC3: Low similarity or unresolved evidence without a qualifying relation does not imply silent merge/default grouping.

**L3-FR-3.6.5 — Relation resolution uses blockers first, deterministic rules second, heuristics last**
- AC1: Hard blockers such as conflicting reference numbers, framework/call-off mismatch, explicit separate-process semantics, incompatible chronology, or clearly different lot/value/reference structure prevent merge regardless of similarity score.
- AC2: Deterministic exact rules then classify same-OCID and explicit linked-notice cases as `merged_same_process`, and framework/call-off relations as `related_not_merged`.
- AC3: Non-blocked, non-exact pairs use the declared heuristic weights and thresholds: `>=0.95` with no blocker -> `merged_same_process`, `0.75-0.94` -> `related_not_merged` or `possible_duplicate`, `<0.75` -> `separate`.
- AC4: Contradictory evidence resolves conservatively toward non-merge (`related_not_merged` or `possible_duplicate`) rather than overriding a hard blocker.
- AC5: Heuristic decisions are replayable because blocker hits, feature weights, and `relation_model_version` are persisted with the decision evidence rather than left implicit in transient code state.

**L3-FR-3.6.6 — Merges preserve stable identity through aliases and `canonical_process_id`**
- AC1: When a merge retires a previously published process ID, `process_aliases` stores a redirect from the alias to the surviving canonical process.
- AC2: Every `ProcurementProcess` carries a `canonical_process_id` that defaults to self and points to the cluster root after merge.
- AC3: Rollups, manifests, and deep links resolve through `canonical_process_id` rather than raw historical `process_id`.
- AC4: Transitive merges resolve both sides to their current canonical roots first and choose the surviving root deterministically, so overlapping merges cannot leave split-brain cluster identity.

**L3-FR-3.6.7 — Late merges are treated as decision-staleness events**
- AC1: When already-reviewed or in-flight process clusters merge, affected `pending_operator_intents` and `operator_decision_heads` entries are marked stale instead of being silently preserved as still-valid decisions.
- AC2: Late merge handling regenerates the manifest and recomputes `pipeline_assessments` at cluster scope.
- AC3: The operator sees an explicit identity-changed notification so contradictory evidence is surfaced rather than hidden.
- AC4: If no recorded/pending operator decision exists, the cluster still recomputes but does not emit a stale-decision incident.

**L3-FR-3.6.8 — Relation clusters remain visible before perfect merge certainty exists**
- AC1: Desktop/UI grouping clusters `related_not_merged` and `possible_duplicate` processes together with relation confidence.
- AC2: Cache sharing inside a cluster remains cautious and trust-aware rather than equivalent to merged identity.
- AC3: Cluster grouping reduces duplicate operator effort without forcing destructive auto-merge.

> **[IMPL] ✗ Deferred** — Operator-facing relation cluster grouping for `related_not_merged` and `possible_duplicate` is not built because those relation classes are not yet produced by the daemon relation stage. The implemented relation surface is the OCID-exact `process_relations` writer plus schema support.

### 3.7 Anti-Bot Classification Table

Per-source anti-bot difficulty rating and mitigation strategy:

| Source | Anti-Bot Difficulty | Primary Challenges | Mitigation |
|--------|--------------------|--------------------|------------|
| **FTS** | Low | Rate limiting only | Respect rate limits; cursor-based incremental retrieval |
| **Contracts Finder** | Low | Rate limiting, daily CSV limits | Rate-limit-aware polling; batch via daily export |
| **PCS** | Low | Standard rate limiting | Month-based query model; respectful polling |
| **eBay UK** | Medium | Bot detection on search, CAPTCHA on rapid access | Category-prior search (not raw keyword spam); fixed-price filter; moderate request pacing |
| **Amazon UK** | Medium-High | Sophisticated bot detection, request fingerprinting | Alias-family queries (not specification paragraphs); session rotation; PA-API deprecated 2026-04-30 reinforces browser-first |
| **Alibaba** | High | Auth walls, geo-detection, session fingerprinting, CAPTCHA | Auth-refresh skills; geo-correct egress; session state management; "Ready to Ship" filter reduces page depth |
| **1688** | High | Chinese-only, heavy auth, aggressive bot detection | Auth-refresh skills; Chinese keyword clusters only; minimal page depth; primarily existence signals |

**L3-FR-3.7.1 — Source anti-bot difficulty and mitigation strategy are explicit inputs to operations**
- AC1: Each listed source has a declared anti-bot difficulty, primary challenge set, and mitigation approach.
- AC2: Operational planning for each source follows the source-specific mitigation strategy in this table rather than generic browser retries.
- AC3: Anti-bot classification informs health monitoring, pacing, and skill-design expectations without replacing live source-health signals.

### 3.8 Browser Automation Reliability Patterns

**Auth persistence:** Auth-refresh skills maintain session state independently from extraction skills. Session handles are stored and reused. Extraction skills consume auth-state handles rather than owning login logic.

**Readiness algorithm:** Wait for specific DOM state, not arbitrary timeouts. Every navigation step specifies:
- Required anchors that must be present
- Forbidden anchors that indicate error/block state
- Maximum wait time before classifying as failure

**Geo/locale drift detection:** Every browser skill checks locale and currency before extraction:
- Expected locale (e.g., `en-GB`) verified via page language attributes
- Expected currency (e.g., `GBP`) verified via price format or currency selector
- Mismatch triggers `SOURCE_LOCALE_DRIFT` and skill abort (wrong-locale data is worse than no data)

**Session contamination prevention:**
- Each skill session starts with a clean browser context (no shared cookies/state from other skills)
- Auth-refresh skills write to a dedicated auth-state store; extraction skills read from it
- No implicit session sharing between different skill types or different sources

**Page fingerprint verification:** Before extracting, prove the page is correct:
- Required anchors present (search box, results container, expected headings)
- Forbidden anchors absent (login form, CAPTCHA, "access denied", wrong-locale indicators)
- This prevents "clean run, wrong page" — the most insidious failure mode because output looks schema-valid

**L3-FR-3.8.1 — Browser automation isolates auth state and prevents session contamination**
- AC1: Auth-refresh skills own session establishment and publish reusable auth-state handles.
- AC2: Extraction skills consume auth-state handles rather than embedding login logic.
- AC3: Each extraction session starts with a clean browser context and does not implicitly reuse unrelated cookies/state from other skills or sources.

**L3-FR-3.8.2 — Browser readiness, locale checks, and fingerprint proof are explicit preconditions to extraction**
- AC1: Every navigation step waits for required anchors, rejects forbidden anchors, and uses explicit maximum wait times rather than arbitrary sleeps.
- AC2: Browser skills verify expected locale and currency before extraction and abort with `SOURCE_LOCALE_DRIFT` on mismatch.
- AC3: Extraction begins only after page fingerprint proof confirms the correct page and rejects login/CAPTCHA/access-denied/wrong-locale states.

> **[IMPL] ✓ As designed** — Browser-source resilience is reflected in Stage 11 CDP/captcha hardening, including fallback and operator-action blocker behavior in `crates/daemon/src/main.rs:6757` and validated by `docs/pipeline/stage-11-hardening-cdp-resilience-validation-manifest.md`.

---

## 4. Schema, Telemetry & Observability

### 4.1 Tables First Activated in Layer 3

All DDL in Postgres syntax (SQLite mirrors with type substitutions per L0 §2.1). The authoritative table shapes are reserved in L0 §2; this section restates the tables that become operationally active in Layer 3.

**Repo-alignment note:** The checked-in SQLite migration surface does not yet fully materialize every L3-ready shape described in the constitutional docs. Before the first Layer 3 write, implementation must reconcile the migration surface so `source_runtime_state` includes `circuit_state` + `last_canary_result`, `process_relations` and `listing_evidence` are materialized, and `process_aliases` keeps the standard push-owned sync columns. Do not rely on opportunistic per-daemon `ALTER TABLE` drift to "discover" the Layer 3 schema at runtime.

> **[IMPL] ↻ Superseded** — The migration surface has been reconciled. `migrations/0005_l3_operational_hardening.sql:3` materializes `source_runtime_state`, `validation_verdicts`, `listing_evidence`, `process_relations`, and `process_aliases`; schema coverage is tested in `crates/daemon/tests/schema_activation.rs`.

#### validation_verdicts (local-only)

Full validator verdict reasoning — never synced. Layer 4 later projects the redacted summary from this local-only source; Layer 3 does **not** require a second local `validation_verdict_summaries` table.

```sql
create table validation_verdicts (
  id uuid primary key default gen_random_uuid(),
  worker_job_id uuid not null references workflow_jobs(id),
  worker_session_id uuid references agent_session_logs(id),
  validator_job_id uuid not null references workflow_jobs(id),
  validator_session_id uuid references agent_session_logs(id),
  validator_skill_path text not null,
  validator_skill_version text,
  verdict text not null,                        -- approved, rejected, flagged
  confidence_bps integer,                       -- basis points (0-10000)
  summary text not null,                        -- one-paragraph human-readable (max 2000 chars)
  basis_sufficiency text not null default 'medium', -- high, medium, low
  anchor_conflict_count integer not null default 0,
  incident_signature text,
  enforcement_eligible boolean not null default false,
  issues_json jsonb not null default '[]',      -- JSON array of ValidationIssue objects
  quality_assessment_json jsonb,
  skill_improvement_suggestions_json jsonb,
  run_manifest_hash text not null,              -- SHA-256 of validated manifest
  oqg_signals_summary text,                     -- OQG inputs provided to validator
  override_verdict text,                        -- operator override: approved, rejected
  override_reason text,
  override_by text,
  override_at timestamptz,
  verdict_rendered_at timestamptz not null,
  created_at timestamptz not null default now()
);
```

#### source_runtime_state (local-only)

Per-source circuit breaker state.

```sql
create table source_runtime_state (
  source_id text primary key references sources(id),
  last_health_state text not null default 'unknown', -- unknown, green, amber, red
  circuit_state text not null default 'closed', -- closed, half_open, open
  challenge_event_count integer not null default 0,
  challenge_window_started_at timestamptz,
  circuit_opened_at timestamptz,
  cooldown_until timestamptz,
  last_canary_at timestamptz,
  last_canary_result text,                      -- passed, failed
  last_error_code text,
  updated_at timestamptz not null default now(),
  created_at timestamptz not null default now()
);
```

#### listing_evidence (local-only)

Two-tier cache for marketplace listing evidence. Reusable across candidates/processes via `ProcessRelation` edges.

```sql
create table listing_evidence (
  id uuid primary key default gen_random_uuid(),
  source text not null,                         -- ebay, amazon, alibaba, 1688
  source_listing_id text not null,
  normalized_title text not null,
  structured_attributes_json jsonb,
  parsed_standards_json jsonb,
  parsed_units_json jsonb,
  seller_identifier text,
  minimum_order_quantity numeric,
  shipping_terms text,
  condition text,                               -- new, used, refurbished, unknown
  compliance_evidence_tier text not null,        -- structured_attribute, title_body_mention, absent
  intended_use_language text,
  is_ready_to_ship boolean,
  fetched_at timestamptz not null,
  detail_page_fetched boolean not null default false,
  trust_state text not null default 'unverified', -- verified, flagged, unverified, suspect
  source_job_id uuid,                             -- job that produced this evidence (for trust tracing)
  created_at timestamptz not null default now(),
  unique (source, source_listing_id)
);
```

Source-specific freshness TTLs:

| Source | Search-Result TTL | Listing-Evidence TTL |
|--------|------------------|---------------------|
| eBay UK | 24h | 48h |
| Amazon UK | 36h | 48h |
| Alibaba | 48h | 72h |
| 1688 | 72h | 96h |
| Ready-to-Ship (Alibaba) | 36h | 48h |

#### procontract_pending_enrichment (local-only)

> **[IMPL] ✓ As designed (2026-06-12 — autonomous daemon scheduling, schema epoch 11):** Durable per-daemon tracking of ProContract tenders where interest was registered but portal access is not yet granted, so enrich can resume them independently of the portal's "Recently added" ordering (see §1.11 for the lifecycle). Added by `migrations/0015_procontract_pending_enrichment.sql` (the epoch 10→11 bump). This is a dedicated awaiting-access table, **not** an `interest_registration.state` flip; `interest_registration.state` remains `'completed'`. **Local-only / not projected to Supabase** — there is no Supabase mirror migration for this table; the `owner_id`/`row_version`/`last_local_seq`/`stage_run_id` columns are present for shape consistency only.

```sql
create table if not exists procontract_pending_enrichment (
  id text primary key,
  process_id text not null,
  notice_id text not null,
  advert_id text not null,
  source_notice_id text,
  state text not null check (state in ('awaiting_access', 'enriched', 'expired')),
  interest_registered_at text not null,
  deadline_at text,                              -- notice-level deadline captured at discover (may be null)
  observation_count integer not null default 0 check (observation_count >= 0), -- enrich-sweep counter; NOT job retry attempt_count
  last_checked_at text,
  enriched_at text,
  expiry_reason text,                           -- 'notice_deadline' | 'max_age'
  owner_id text not null default 'local-owner',
  row_version integer not null default 1 check (row_version > 0),
  last_local_seq integer,
  stage_run_id text,
  created_at text not null default current_timestamp,
  updated_at text not null default current_timestamp,
  unique (process_id, notice_id, advert_id),
  check (length(trim(id)) > 0),
  check (length(trim(process_id)) > 0),
  check (length(trim(notice_id)) > 0),
  check (length(trim(advert_id)) > 0),
  check (source_notice_id is null or length(trim(source_notice_id)) > 0),
  check (length(trim(interest_registered_at)) > 0),
  check (deadline_at is null or length(trim(deadline_at)) > 0),
  check (last_checked_at is null or length(trim(last_checked_at)) > 0),
  check (enriched_at is null or length(trim(enriched_at)) > 0),
  check (expiry_reason is null or length(trim(expiry_reason)) > 0),
  check (length(trim(owner_id)) > 0),
  check (stage_run_id is null or length(trim(stage_run_id)) > 0)
);
create index if not exists idx_procontract_pending_enrichment_state
  on procontract_pending_enrichment(state, updated_at);
create index if not exists idx_procontract_pending_enrichment_advert
  on procontract_pending_enrichment(advert_id);
```

**L3-FR-4.1.6 — `procontract_pending_enrichment` durably tracks awaiting-access tenders (local-only)**
- AC1: A row is created with `state='awaiting_access'` when ProContract register-interest succeeds, **without** mutating `interest_registration.state` (it stays `'completed'` so the re-fire guard still excludes it — §1.11).
- AC2: `observation_count` is the enrich-sweep counter and is distinct from the job-retry `attempt_count`.
- AC3: Expiry (`state='expired'`) keys off the notice-level `deadline_at` (`expiry_reason='notice_deadline'`) else a max-age-since-interest fallback (`expiry_reason='max_age'`) — **never** the RFx response deadline, which an awaiting-access item by definition has not reached.
- AC4: The table is local-only; it is not projected to Supabase.

#### process_relations (push-owned)

```sql
create table process_relations (
  id uuid primary key default gen_random_uuid(),
  process_id_a uuid not null references procurement_processes(id),
  process_id_b uuid not null references procurement_processes(id),
  relation_type text not null,                  -- merged_same_process, related_not_merged, possible_duplicate (no row for separate)
  confidence_bps integer not null,              -- basis points (0-10000)
  evidence jsonb,
  owner_id text not null,
  row_version bigint not null default 1,
  last_local_seq bigint,
  stage_run_id text,
  updated_at timestamptz not null default now(),
  created_at timestamptz not null default now()
);
```

#### process_aliases (push-owned)

```sql
create table process_aliases (
  alias_process_id uuid primary key,
  canonical_process_id uuid not null references procurement_processes(id),
  alias_reason text not null default 'merged_same_process',
  owner_id text not null,
  row_version bigint not null default 1,
  last_local_seq bigint,
  stage_run_id text,
  updated_at timestamptz not null default now(),
  created_at timestamptz not null default now()
);
```

**L3-FR-4.1.1 — `validation_verdicts` stores full local-only validator reasoning**
- AC1: `validation_verdicts` links worker and validator jobs/sessions and stores verdict identity, confidence, summary, `basis_sufficiency`, incident signature, `enforcement_eligible`, issue JSON, and run-manifest hash.
- AC2: Operator override fields (`override_verdict`, `override_reason`, `override_by`, `override_at`) exist from first activation.
- AC3: The table is local-only; Layer 4 later derives redacted `validation_verdict_summaries` from it without requiring a duplicate local summary table in Layer 3.

**L3-FR-4.1.2 — `source_runtime_state` persists breaker/health state per source**
- AC1: `source_runtime_state` is keyed by `source_id` and records health color, circuit state, challenge counters/window, cooldown timestamps, latest canary outcome, and last error code.
- AC2: The table is the local operational authority for source-health and circuit-breaker transitions.
- AC3: Persisted fields are sufficient to audit why a source is `closed`, `half_open`, or `open`.

**L3-FR-4.1.3 — `listing_evidence` activates as a trust-aware local cache**
- AC1: `listing_evidence` stores source identity, listing identity, normalized title, structured attributes, parsed standards/units, seller/commercial fields, freshness timestamps, and detail-fetch flags.
- AC2: The table carries `trust_state` with default `unverified` plus `source_job_id` for provenance/trust tracing.
- AC3: Uniqueness is enforced on `(source, source_listing_id)`, and source-specific freshness TTLs match the table in this section.

**L3-FR-4.1.4 — `process_relations` stores push-owned relation edges**
- AC1: `process_relations` stores both process IDs, `relation_type`, `confidence_bps`, supporting evidence, and the standard sync-tracking fields.
- AC2: Stored `relation_type` values remain constrained to evidence-bearing non-`separate` outcomes: `merged_same_process`, `related_not_merged`, and `possible_duplicate`; `separate` stays an evaluated outcome but writes no row.
- AC3: The table is push-owned so relation state can become shared visibility in Layer 4 without changing authority.

**L3-FR-4.1.5 — `process_aliases` stores push-owned redirects for retired process IDs**
- AC1: `process_aliases` maps `alias_process_id` to `canonical_process_id` with an `alias_reason` and the full standard push-owned sync columns including `stage_run_id`.
- AC2: Alias rows preserve deep-link stability after relation-driven merges.
- AC3: The alias table is push-owned and remains separate from the full cluster model carried by `canonical_process_id`.

> **[IMPL] ✓ As designed** — The L3 schema activation tables exist in `migrations/0005_l3_operational_hardening.sql`; `process_relations` and `process_aliases` include sync tracking columns and `stage_run_id` at `migrations/0005_l3_operational_hardening.sql:75` and `migrations/0005_l3_operational_hardening.sql:97`.

### 4.2 Modified Tables

| Table | Field | Change in L3 |
|-------|-------|-------------|
| `workflow_jobs` | `parent_job_id` | Now actively used — links validator sub-jobs to worker jobs |
| `workflow_jobs` | `validation_required` | Now actively set by Validation Policy Engine (still defaults false) |
| `workflow_jobs` | `validation_verdict_id` | Now populated after validation completes |
| `agent_session_logs` | `run_manifest_path` | Now actively populated by Run Manifest Writer |
| `agent_session_logs` | `run_manifest_hash` | Now actively populated (SHA-256 of manifest) |

**L3-FR-4.2.1 — Layer 3 activates its forward-compatible columns on existing tables**
- AC1: `workflow_jobs.parent_job_id` links validator sub-jobs to worker jobs in live Layer 3 behavior.
- AC2: `workflow_jobs.validation_required` and `workflow_jobs.validation_verdict_id` are actively populated by validation policy/execution.
- AC3: `agent_session_logs.run_manifest_path` and `agent_session_logs.run_manifest_hash` are populated for every manifest-producing session.

> **[IMPL] ✓ As designed** — Job claiming and validator linkage are active in `crates/daemon/src/jobs/claim.rs:72`, `crates/daemon/src/jobs/runner.rs:244`, and `crates/daemon/src/validation/executor.rs:1046`; run-manifest path/hash columns are in `migrations/0003_sqlite_runtime.sql:29`.

### 4.3 Authority Classification (L3 Additions)

| Table | Authority | Rationale |
|-------|-----------|-----------|
| `validation_verdicts` | Local-only | Full verdict reasoning too large/sensitive for sync |
| `source_runtime_state` | Local-only | Per-daemon operational state |
| `listing_evidence` | Local-only | Cache layer, not authoritative data |
| `procontract_pending_enrichment` *(epoch 11)* | Local-only | Per-daemon awaiting-access resumption state; dedicated table so `interest_registration.state` stays `'completed'`; not projected to Supabase |
| `process_relations` | Push-owned | Shared visibility in Layer 4 |
| `process_aliases` | Push-owned | Stable deep links across desktop/web |

**L3-FR-4.3.1 — Layer 3 table authority classes are fixed at activation**
- AC1: `validation_verdicts`, `source_runtime_state`, `listing_evidence`, and `procontract_pending_enrichment` are local-only.
- AC2: `process_relations` and `process_aliases` are push-owned.
- AC3: No Layer 3 table introduced in this section mixes local and remote authority.

### 4.4 Four-Layer Observability Model

| Layer | Content | Storage | Purpose |
|-------|---------|---------|---------|
| **Control events** | Job lifecycle, stage transitions, decision points, resource grants/denials, circuit breaker state changes | Structured JSON logs (tracing crate) | Operational monitoring, alerting |
| **Protocol events** | Agent communication, skill loading, checkpoint progression, result contract events, adapter events | Structured JSON in `agent_session_logs.structured_trace` | Debugging, replay, performance |
| **Forensic bytes** | Raw PTY output OR contract-mode stdout/stderr | Compressed immutable artifacts at `terminal_output_path` | Post-incident analysis, deterministic replay |
| **Narrative** | Run manifests — agent-written narrative + daemon evidence appendix | Immutable artifacts at `run_manifest_path` | Semantic validation, operator review |

All four layers capture data from both PTY and contract-mode sessions.

**Correlation and retention rules:**
- Every layer is keyed by `job_id`; session-scoped artifacts additionally carry `session_id`; process-facing surfaces carry `process_id`; validator records carry both worker and validator job/session identities plus `run_manifest_hash`.
- The minimum audit trail for a 3 AM failure is verdict -> `validation_verdicts.worker_session_id` / `run_manifest_hash` -> `agent_session_logs.run_manifest_path` -> appendix `artifact_refs` / `terminal_output_path` -> structured control/protocol traces.
- Minimum L3 retention: control/protocol traces `30` days, raw forensic bytes `14` days or until linked error manifests are resolved (whichever is longer), and run manifests plus `validation_verdicts` `180` days and never shorter than the lifetime of linked workflow/error/backlog records.

**L3-FR-4.4.1 — Observability remains explicitly split across four evidence layers**
- AC1: Control events, protocol events, forensic bytes, and narrative manifests are stored as separate observability layers with the storage surfaces named in this section.
- AC2: Each layer has a distinct purpose and is not treated as a substitute for another layer's evidence.
- AC3: The observability model supports both operational monitoring and post-incident replay/review.

**L3-FR-4.4.2 — PTY and contract sessions both populate the same four observability layers**
- AC1: PTY sessions and contract-mode sessions both emit control and protocol events.
- AC2: PTY raw bytes and contract stdout/stderr both flow into immutable forensic capture.
- AC3: Both execution modes produce narrative run manifests for semantic validation/review.

**L3-FR-4.4.3 — Observability layers are correlated and retained long enough for delayed investigation**
- AC1: Verdicts, manifests, forensic bytes, and structured traces remain joinable by explicit job/session/process identifiers plus `run_manifest_hash`.
- AC2: A validator rejection or fail-open incident can be traced from verdict to manifest to forensic artifact without transcript scraping or operator memory.
- AC3: Retention minima follow the policy in this section and do not prune raw bytes before the linked manifest/verdict/error surfaces remain reviewable.

> **[IMPL] ✓ As designed** — The four-layer observability model is represented by structured trace/artifact/session/manifest surfaces in `migrations/0003_sqlite_runtime.sql:3`, `crates/daemon/src/tracing/structured.rs`, `crates/daemon/src/tracing/forensic.rs`, and `crates/daemon/src/agents/artifacts.rs`.

### 4.5 Skill-Level Telemetry (13 Metrics)

**Applicability and emission rules:**
- Metrics that do not apply to a skill family emit `null` / `not_applicable` and are excluded from alert math rather than being recorded as synthetic zeroes.
- Update points are fixed: lifecycle metrics (`success_rate`, durations, cost, token, retry) update on terminal job completion; `checkpoint_count` updates on validated checkpoint write; fill/fingerprint/auth/drift metrics update when the daemon appendix is finalized; `listing_match_rate_bps` updates on supplier-match commit; `validation_rejection_rate_bps` updates when a verdict is persisted.

| # | Metric | Definition | Unit | Alert Threshold |
|---|--------|-----------|------|----------------|
| 1 | `success_rate` | Successful completions / total runs | ratio | < 0.90 over 24h |
| 2 | `median_duration_ms` | Median wall-clock time per run | ms | > 2x baseline |
| 3 | `p95_duration_ms` | 95th percentile duration | ms | > 3x baseline |
| 4 | `cost_per_run_pence` | Model token cost per run | integer pence | > 2x baseline |
| 5 | `token_count` | Total tokens consumed per run | count | Informational |
| 6 | `checkpoint_count` | Checkpoints emitted per run | count | Informational |
| 7 | `retry_count` | Retries triggered per run | count | > 2 average |
| 8 | `extraction_fill_rate_bps` | Fields populated / total expected | bps (0-10000) | < 7000 (70%) |
| 9 | `page_fingerprint_delta_bps` | Structure deviation from baseline | bps (0-10000) | > 2000 (20%) |
| 10 | `auth_challenge_rate_bps` | Runs encountering auth challenge | bps (0-10000) | > 1000 (10%) |
| 11 | `listing_match_rate_bps` | Candidates with >= 1 genuine match | bps (0-10000) | < 5000 (50%) |
| 12 | `validation_rejection_rate_bps` | Runs rejected by validator | bps (0-10000) | > 2000 (20%) |
| 13 | `drift_score_bps` | Composite page/content drift | bps (0-10000) | > 3000 (30%) |

**L3-FR-4.5.1 — Layer 3 records the 13 skill-level metrics with concrete alert thresholds**
- AC1: Skill-level telemetry records all 13 metrics listed in this table, including success, duration, cost, checkpoint, retry, fill-rate, fingerprint, auth, listing-match, validation-rejection, and drift signals.
- AC2: Numeric alert thresholds match the table values, including success rate `<0.90`, fill rate `<7000`, fingerprint delta `>2000`, auth challenge rate `>1000`, validation rejection rate `>2000`, and drift score `>3000`.
- AC3: Metrics marked informational remain threshold-free by design; all others use explicit numeric thresholds or baseline-relative thresholds as stated here.
- AC4: Non-applicable metrics are represented as `null` / `not_applicable` and excluded from alert thresholds rather than silently counting as zero.

### 4.6 Validation Telemetry (Dry-Run)

8 validation metrics for Layer 3 (expanded to 16 in Layer 4 enforcement):

| Metric | Definition | Unit |
|--------|-----------|------|
| `validator_approval_rate_bps` | Approved / total validated | bps |
| `validator_rejection_rate_bps` | Rejected / total validated | bps |
| `validator_flag_rate_bps` | Flagged / total validated | bps |
| `validator_cost_per_run_pence` | Model cost per validation | integer pence |
| `operator_override_rate_bps` | Overrides / total rejections (false positive proxy) | bps |
| `validator_execution_failure_rate_bps` | Validator crashes/timeouts / total validation attempts | bps |
| `validator_canary_pass_rate_bps` | Synthetic canary correct classifications / total canary runs | bps |
| `validator_health_state` | Current subsystem health: healthy/degraded/blind | enum |

Validation telemetry updates at three fixed points: verdict persistence, synthetic-canary completion, and validator-health state transition. Missing verdict persistence means the run counts as validator execution failure, not as a silent success.

**L3-FR-4.6.1 — Layer 3 records the eight dry-run validation telemetry metrics**
- AC1: Validation telemetry records approval, rejection, flag, cost, operator override, execution failure, canary pass, and subsystem health metrics.
- AC2: `validator_health_state` is constrained to `healthy`, `degraded`, or `blind`.
- AC3: Validation telemetry is sufficient to audit both verdict quality and validator availability during dry-run.

**L3-FR-4.6.2 — Validation telemetry drives health-state transitions with concrete thresholds**
- AC1: `validator_execution_failure_rate_bps` above `2000` over `1h` can drive `degraded`, and above `5000` over `1h` can drive `blind`, as defined in §2.7.
- AC2: `validator_canary_pass_rate_bps` reflects the required known-good/known-bad/known-insufficient-basis synthetic canaries from §2.7, and any canary misclassification can force `degraded` or `blind`.
- AC3: Operator override rate and execution failure telemetry remain available as demotion/calibration inputs rather than only as dashboards.

> **[IMPL] ✓ As designed** — Validation status snapshots are built in `crates/daemon/src/main.rs:2430`, policy health/demotion logic is in `crates/daemon/src/validation/policy.rs:352`, and telemetry behavior is covered by `crates/daemon/tests/observability_telemetry.rs` plus validation tests.

---

## 5. Testing & Acceptance

Coverage planning note: this validated revision defines **98** `L3-FR-*` requirements across §§1-4. The 18 files in §5.2 are file containers, not total test-case count, so a test engineer does **not** yet have an exact final case count from this spec alone. Downstream TDD must assign at least one test ID per requirement and, after policy-mode matrices and edge-case expansions, should expect roughly **120-150** concrete tests.

### 5.1 Definition of Done

All items must pass before Layer 3 is complete, and each item must trace to one or more `L3-FR-*` requirements:

| # | Criterion | Verification | Related Requirements |
|---|----------|-------------|----------------------|
| 1 | Cron scheduler runs on configured schedule with explicit skill binding | `cron_scheduler.rs` | `L3-FR-1.3.1`, `L3-FR-1.5.1`, `L3-FR-1.5.2` |
| 2 | Batched ingestion: scheduled FTS + CF runs complete without manual intervention | 24h unattended cycle | `L3-FR-1.4.1`-`L3-FR-1.4.4`, `L3-FR-1.5.2`, `L3-FR-1.7.1`, `L3-FR-3.4.2` |
| 3 | Relation resolution correctly classifies (merged/related/possible_duplicate/separate) | `resolve-process-relations.test.ts` | `L3-FR-3.6.1`-`L3-FR-3.6.8`, `L3-FR-4.1.4`, `L3-FR-4.1.5` |
| 4 | PTY hardening: fd hygiene, EIO handling, terminal geometry stability | `fault_pty_edge_cases.rs` | `L3-FR-1.3.4`, `L3-FR-1.3.5`, `L3-FR-1.6.1` |
| 5 | Host environment awareness: sleep/wake detection, catchup sweeps | `fault_host_events.rs` | `L3-FR-1.5.2`, `L3-FR-1.7.1` |
| 6 | Resource arbiter enforces typed quotas (browser slots, agent slots, source lanes) | Load test | `L3-FR-1.3.6`, `L3-FR-1.4.1`-`L3-FR-1.4.4` |
| 7 | Skill promotion flow: draft -> candidate -> canary -> shadow -> stable | `skill_golden_journeys.rs` + `skill_shadow_compare.rs` | `L3-FR-3.1.1`-`L3-FR-3.1.7`, `L3-FR-3.2.1`, `L3-FR-3.2.2` |
| 8 | Source-health canaries and auth-refresh skills operational | Canary test per source | `L3-FR-1.9.1`, `L3-FR-3.3.1`, `L3-FR-3.4.1`, `L3-FR-3.4.2`, `L3-FR-3.5.1`-`L3-FR-3.5.4`, `L3-FR-3.8.1`, `L3-FR-3.8.2` |
| 9 | Worker-validator dry-run: manifests generated, base validator renders verdicts, logged without enforcement | `validation_dry_run.rs` + `validation_golden_verdicts.rs` | `L3-FR-1.3.10`-`L3-FR-1.3.12`, `L3-FR-2.1.1`-`L3-FR-2.9.3`, `L3-FR-4.1.1` |
| 10 | Error manifest control-panel view displays failures with resolution context | Manual desktop verification | `L3-FR-1.3.8`, `L3-FR-1.8.1`, `L3-FR-4.4.1`, `L3-FR-4.4.2` |
| 11 | Validator health canaries running every 15-30 min; known-good/bad/insufficient-basis correctly classified | `validation_canary_health.rs` | `L3-FR-2.7.4`, `L3-FR-2.7.5`, `L3-FR-2.8.1`, `L3-FR-2.8.2`, `L3-FR-4.6.1`, `L3-FR-4.6.2` |
| 12 | Validator health state transitions (healthy → degraded → blind) fire on failure rate spikes | `validation_canary_health.rs` | `L3-FR-2.7.4`, `L3-FR-2.7.5`, `L3-FR-4.6.2` |
| 13 | Retroactive validation backlog accumulates during validator outage and drains on recovery | `validation_retro_backlog.rs` | `L3-FR-2.7.1`, `L3-FR-2.7.5`, `L3-FR-4.1.1`, `L3-FR-4.6.1` |
| 14 | System runs unattended for 5 consecutive days, >=95% success rate | Monitoring + operator sign-off | `L3-FR-1.4.1`-`L3-FR-1.5.2`, `L3-FR-1.7.1`, `L3-FR-3.4.2`, `L3-FR-4.5.1`, `L3-FR-4.6.2` |
| 15 | L3 schema activation matches the declared DDL and forward-compatible columns are populated without runtime table-shape drift | Migration smoke + DB inspection | `L3-FR-4.1.1`-`L3-FR-4.3.1` |
| 16 | Operator can trace verdict → run manifest → forensic bytes → originating session/job without manual transcript archaeology | Replay/manual drill-down | `L3-FR-2.4.4`, `L3-FR-4.4.1`-`L3-FR-4.4.3` |
| 17 | Telemetry emits at the declared update points and non-applicable metrics do not generate false alerts | Metrics smoke + fault injection | `L3-FR-1.4.5`, `L3-FR-4.5.1`, `L3-FR-4.6.1`, `L3-FR-4.6.2` |

### 5.2 Test File Listing (18 New, Cumulative 48)

#### Fault Injection (4)

| File | Coverage | Primary Requirements | DoD Coverage |
|------|---------|----------------------|--------------|
| `fault_process_cleanup.rs` | Orphaned grandchildren, dropped handle, cleanup timeout, leaked detection, pgid kill escalation | `L3-FR-1.3.4`, `L3-FR-1.6.1` | 4 |
| `fault_pty_edge_cases.rs` | PTY EOF/EIO, fd leak scenarios, terminal geometry changes mid-session | `L3-FR-1.3.4`, `L3-FR-1.3.5`, `L3-FR-1.6.1` | 4 |
| `fault_host_events.rs` | Host sleep/monotonic gap, resource exhaustion, concurrent pressure, catchup sweep | `L3-FR-1.5.2`, `L3-FR-1.7.1`, `L3-FR-1.4.1`-`L3-FR-1.4.4` | 2, 5, 6, 14 |
| `fault_partial_failure.rs` | Partial checkpoint then crash, result contract failure mid-stream, additive partial commit | `L3-FR-1.3.7`, `L3-FR-2.5.1`, `L3-FR-2.5.2`, `L3-FR-2.7.1` | 9 |

#### Deterministic Replay (2)

| File | Coverage | Primary Requirements | DoD Coverage |
|------|---------|----------------------|--------------|
| `replay_pty_sessions.rs` | Replay captured PTY byte streams against adapter parser (escape sequences, split reads) | `L3-FR-1.3.8`, `L3-FR-1.3.9`, `L3-FR-4.4.1`, `L3-FR-4.4.2` | 4, 9 |
| `replay_contract_sessions.rs` | Replay captured contract-mode sessions (structured event parsing, checkpoint extraction) | `L3-FR-1.3.8`, `L3-FR-1.3.9`, `L3-FR-4.4.1`, `L3-FR-4.4.2` | 9 |

#### Scheduling & Jobs (2)

| File | Coverage | Primary Requirements | DoD Coverage |
|------|---------|----------------------|--------------|
| `cron_scheduler.rs` | Cron expression parsing, schedule evaluation, skill binding, catchup sweep, timezone/DST | `L3-FR-1.3.1`, `L3-FR-1.5.1`, `L3-FR-1.5.2` | 1, 2, 5, 14 |
| `job_claim.rs` | Concurrent claiming, lease expiry, double-claim prevention, crash recovery | `L3-FR-1.3.3`, `L3-FR-1.3.6`, `L3-FR-1.4.1`-`L3-FR-1.4.4` | 2, 6, 14 |

#### Skill Lifecycle (3)

| File | Coverage | Primary Requirements | DoD Coverage |
|------|---------|----------------------|--------------|
| `skill_golden_journeys.rs` | Invariant-based E2E per skill family against known inputs/golden assertions | `L3-FR-1.9.1`, `L3-FR-3.1.1`-`L3-FR-3.1.7`, `L3-FR-3.3.1` | 7, 8 |
| `skill_shadow_compare.rs` | Quality diff between stable and candidate: fill-rate, duplicate-rate, cost, validator approval | `L3-FR-3.1.3`-`L3-FR-3.1.7`, `L3-FR-3.2.2`, `L3-FR-4.5.1` | 7, 14 |
| `skill_drift_monitor.rs` | Page fingerprint trending, fallback depth, fill-rate trending | `L3-FR-2.2.2`, `L3-FR-3.4.1`, `L3-FR-4.5.1` | 8, 14 |

#### Validation (6)

| File | Coverage | Primary Requirements | DoD Coverage |
|------|---------|----------------------|--------------|
| `validation_policy.rs` | All policy mode x trigger combinations: always/sample/trigger/skip, anomaly triggers | `L3-FR-1.3.10`, `L3-FR-2.3.1`-`L3-FR-2.3.10` | 9 |
| `validation_executor.rs` | Sub-job spawning, kickoff prompt composition, verdict parsing, basis_sufficiency, unknown-verdict fail-open, transactional verdict persistence | `L3-FR-1.3.11`, `L3-FR-2.4.1`-`L3-FR-2.4.4`, `L3-FR-2.5.2` | 9, 16 |
| `validation_dry_run.rs` | Dry-run verdicts logged, pipeline NOT blocked, no outbox, no stage head advancement, missing-OQG/unavailable-signal handling | `L3-FR-2.6.1`, `L3-FR-2.6.2`, `L3-FR-2.7.1`, `L3-FR-4.1.1` | 9, 13, 17 |
| `validation_golden_verdicts.rs` | Known-good -> approved, known-bad -> rejected, ambiguous -> flagged | `L3-FR-2.8.1`, `L3-FR-2.8.2`, `L3-FR-2.9.1`, `L3-FR-2.9.2` | 9, 11 |
| `validation_canary_health.rs` | Synthetic canary scheduling, known-good/bad/insufficient-basis classification, health state transitions (healthy → degraded → blind), auto-demotion on blind | `L3-FR-2.7.4`, `L3-FR-2.7.5`, `L3-FR-4.6.1`, `L3-FR-4.6.2` | 11, 12 |
| `validation_retro_backlog.rs` | Backlog accumulation during validator outage, priority-ordered drain on recovery, stale-marking of affected decisions | `L3-FR-2.7.1`, `L3-FR-2.7.5`, `L3-FR-4.1.1` | 13 |

#### Relations (1)

| File | Coverage | Primary Requirements | DoD Coverage |
|------|---------|----------------------|--------------|
| `resolve-process-relations.test.ts` | Relation scoring, hard blockers, four-way classification, confidence thresholds, transitive-root assignment, overlapping-pair determinism | `L3-FR-3.6.1`-`L3-FR-3.6.8`, `L3-FR-4.1.4`, `L3-FR-4.1.5` | 3, 15 |

### 5.3 Test Scenarios

| Scenario | Expected Behavior | Related Requirements |
|----------|------------------|----------------------|
| Cron fires after daemon slept through scheduled time | Catchup sweep detects, runs overdue jobs | `L3-FR-1.5.2`, `L3-FR-1.7.1` |
| Two FTS jobs scheduled concurrently | Resource arbiter grants one browser slot, second queues | `L3-FR-1.3.6`, `L3-FR-1.4.1`, `L3-FR-1.4.3`, `L3-FR-1.4.4` |
| Candidate skill has worse fill-rate than stable | Shadow comparison fails, candidate NOT promoted | `L3-FR-3.1.3`, `L3-FR-3.1.4`, `L3-FR-4.5.1` |
| Source-health canary detects FTS layout change | Source enters cooldown (circuit breaker), error manifest created | `L3-FR-3.4.1`, `L3-FR-3.4.2`, `L3-FR-3.5.2`-`L3-FR-3.5.4` |
| Validator rejects supplier enrichment manifest | Dry-run: logged, NOT enforced, worker results advance | `L3-FR-2.4.4`, `L3-FR-2.6.1`, `L3-FR-2.7.2`, `L3-FR-2.7.3` |
| Validator crashes mid-validation | Fail-open: worker results advance, `VALIDATION_SKIPPED_VALIDATOR_ERROR` | `L3-FR-2.4.4`, `L3-FR-2.7.1`, `L3-FR-2.7.5` |
| Relation resolution: same OCID but different buyers | `possible_duplicate` (NOT merged) — buyer mismatch is hard blocker | `L3-FR-3.6.3`, `L3-FR-3.6.5` |
| Host enters low-disk state during session | Resource arbiter blocks new starts, existing complete, alert logged | `L3-FR-1.4.1`-`L3-FR-1.4.4`, `L3-FR-1.7.1` |
| PTY receives EIO (terminal disconnected) | Graceful handling, session transitions to terminating, verdict recorded | `L3-FR-1.3.4`, `L3-FR-1.3.5`, `L3-FR-1.6.1` |
| Auth-refresh detects expired Alibaba session | Re-authenticates, stores fresh state, subsequent jobs use new auth | `L3-FR-1.9.1`, `L3-FR-3.3.1`, `L3-FR-3.4.1`, `L3-FR-3.8.1` |
| Validator breaks at 2 AM, 200 overnight runs | All 200 results carry `unverified` trust state; validator health transitions to `blind`; retroactive validation backlog queued; operator morning dashboard shows single validation blackout incident | `L3-FR-2.7.1`-`L3-FR-2.7.5`, `L3-FR-4.6.2` |
| Validator canary misclassifies known-bad as approved | Health transitions to `degraded`; enforcement auto-demotes on affected slices | `L3-FR-2.7.4`, `L3-FR-2.7.5`, `L3-FR-4.6.2` |
| Two processes merge after operator reviewed one | Both operator decisions marked stale; merged `canonical_process_id` assigned; manifest regenerated | `L3-FR-3.6.6`, `L3-FR-3.6.7`, `L3-FR-4.1.5` |
| Cosmetic prompt edit to skill (no behavior change) | Behavior version unchanged; slice enforcement readiness preserved | `L3-FR-2.3.8`, `L3-FR-3.2.2` |
| Full cron-triggered ingestion reaches dry-run validation | Scheduler enqueues due job, arbiter admits it, worker manifest is written, validator sub-job runs, verdict persists, worker job completes | `L3-FR-1.3.1`, `L3-FR-1.3.6`, `L3-FR-1.4.5`, `L3-FR-1.5.1`, `L3-FR-2.4.1`-`L3-FR-2.4.4`, `L3-FR-2.6.1` |
| Validator returns unknown verdict value | Run fails open, validator error manifest created, worker result marked `unverified`, backlog entry queued | `L3-FR-2.4.4`, `L3-FR-2.7.1`, `L3-FR-2.7.5` |
| Run manifest would exceed 50 KB | Daemon elides oversized appendix sections, emits bounded manifest, and keeps validator input reviewable | `L3-FR-2.1.4`, `L3-FR-2.4.2` |
| Two concurrent canaries update the same source breaker row | Newer canary timestamp wins; stale success does not close a newer `open` circuit | `L3-FR-3.5.4`, `L3-FR-4.1.2` |
| Overlapping relation jobs resolve `(A,B)` and `(B,C)` concurrently | Both decisions converge on one deterministic canonical root; no split-brain `canonical_process_id` remains | `L3-FR-3.6.5`, `L3-FR-3.6.6`, `L3-FR-4.1.4`, `L3-FR-4.1.5` |
| OQG signals unavailable during validation | Verdict may still render with lowered `basis_sufficiency`, or run remains `unverified`; pipeline does not silently treat missing OQG as clean | `L3-FR-2.2.2`, `L3-FR-2.7.1`, `L3-FR-4.6.1` |

### 5.4 Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Validator false positive rate too high for future enforcement | Medium | High | Dry-run calibration in L3; golden verdicts regression; track rejection rate per skill |
| Cron edge cases (timezone, DST, clock drift) | Low | Medium | Catchup sweeps; monotonic clock for internal timers |
| Relation resolution false merges | Medium | Medium | Conservative thresholds; hard blockers; operator can unmerge; `possible_duplicate` for borderline |
| Source changes break multiple skills simultaneously | Medium | High | Source-health canaries detect before production; per-source circuit breaker isolates failures |
| Systematic validator failure goes undetected overnight | Medium | High | Synthetic validator canaries every 15-30 min; health state transitions; retroactive validation backlog on recovery |
| Late merge invalidates operator decisions silently | Medium | High | Late merge treated as decision-staleness event; affected decisions marked stale; manifest regenerated |
| Most slices never reach enforcement-grade calibration | High | Medium | Expected — explicit product truth; signature-limited enforcement provides partial value; dry-run still catches issues |
| Unverified results seed reusable caches at full trust | Low | High | Trust metadata on `listing_evidence` and relation cache sharing; `suspect` outputs excluded from cache seeding |

---

## 6. Open Questions, Forward Compatibility & Dependencies

### 6.1 Open Questions

The questions below are non-blocking for Layer 3 implementation and do not defer any `L3-FR-*` requirement in §§1-4.

**§10 Further Research #4: Expiry behavior for in-flight jobs**
- Should expiry (`availabilityStatus = expired`) cancel in-flight jobs or let them complete?
- Current design: let complete, block downstream progression.
- **L3 impact:** Cron scheduler may trigger jobs for notices expiring mid-pipeline.

**§10 Further Research #6: Late process merge reconciliation**
- How should post-enrichment merges reconcile downstream data?
- **Partially resolved:** `canonical_process_id` introduced (§3.6) as the cluster identity root. Late merges now treated as decision-staleness events with manifest regeneration, assessment recomputation, and decision stale-marking.
- **Remaining open:** Full cluster-aware rollup logic (reconciling divergent product candidates, quotes, and coverage metrics from the two previously-separate processes). How to handle conflicting quotes for the same product from different processes? How to merge coverage metrics when one process has enrichment the other lacks?
- **L3 impact:** Relation engine creates edges and assigns `canonical_process_id`. Full rollup reconciliation deferred until post-enrichment merges actually occur in practice.

**§10 Further Research #7: Validator calibration corpus bootstrapping**
- Three corpora needed: clean real (25-30), weird-but-valid (15-20), mutated bad (30-50).
- How to bootstrap weird-but-valid before production traffic?
- **L3 impact:** Gold-set from L0 may provide initial mutation material.

### 6.2 Stubs

| Feature | L3 Stub | Full Implementation |
|---------|---------|-------------------|
| Validation enforcement | Dry-run only — logged, not enforced | Layer 4: enforcement + quarantine |
| Domain-specific validators | Base validator only | Layer 4: 4 overlays (procurement, supplier, document, product) |
| Formal calibration harness | Initial calibration against gold-set | Layer 4: three-corpus with mutation bank |
| Sync | Tracking columns populated, no outbox | Layer 4: full sync engine |
| Multi-user | Single operator only | Layer 4: web portal + RLS |

### 6.3 Forward-Compatibility

| Provision | What Exists in L3 | L4 Consumer / Expectation | Target Layer |
|-----------|-------------------|--------------------------|-------------|
| Validation policy engine ready for enforcement toggle | Evaluates and sets `validation_required`, but always dry-run in L3 | L4 §1.3 provisional promotion buffer and §3.4 per-slice enforcement consume the same policy decision surface | Layer 4 |
| Run manifest corpus accumulates | Every skill run produces manifest | L4 §3.6 formal calibration harness consumes the accumulated corpus | Layer 4 calibration |
| `validation_verdicts` full schema from creation | Includes `enforcement_eligible`, `basis_sufficiency`, verdict reasoning | L4 §2.2 derives `validation_verdict_summaries` from this local-only source for shared visibility; no duplicate local summary table is required in L3 | Layer 4 projections |
| Source circuit breaker state accessible | Daemon-local source-health and breaker state already persisted | L4 keeps consuming the same local risk signal inside daemon-local validation/enforcement logic; no new remote table dependency is required in L4 §§1-3 | Layer 4 daemon-local enforcement |
| Skill promotion includes validator approval rate | Promotion bookkeeping already treats validator approval as a gate | No new sync schema in L4 §§1-3 depends on this directly; it remains an operational rollout input across L3/L4 | Layer 4 operations |
| Validator health states operational | `healthy` / `degraded` / `blind` with synthetic canaries | L4 §3.4 automatic demotion from enforcement depends on these states already existing | Layer 4 enforcement auto-demotion |
| Retroactive validation backlog | Backlog accumulates during outages and can stale/flag affected outputs on replay | L4 §1.3 enforcement/quarantine semantics consume the replayed outputs after recovery; no separate remote table is required first | Layer 4 enforcement recovery |
| Trust-state lattice on pipeline results | `verified` / `flagged` / `unverified` / `suspect` tracked locally | L4 §1.3 provisional promotion/quarantine logic and shared visibility of coarse `validation_state` depend on this trust separation | Layer 4 downstream behaviour gating |
| `canonical_process_id` on processes | Defaults to self; updated on merge | L4 §2.1 projection tables plus `process_relations` / `process_aliases` expect stable canonical identity for cluster-aware sync | Layer 4 cluster-aware sync |

### 6.4 Dependencies

**Layer deps:** L0 (intelligence, gold set), L1 (daemon, adapter), L2 (desktop, pipeline model)

#### Additional Rust Crates

| Crate | Purpose |
|-------|---------|
| `cron` | Cron expression parsing |
| `sysinfo` | Host CPU/memory/disk pressure detection for admission gating and observability |

No new PTY/process-supervision crate is required for Layer 3 hardening: the existing `nix` + `portable_pty` stack remains the execution substrate for fd hygiene, wait draining, and process-group cleanup.

#### New Skill Surfaces

| Skill | Purpose |
|-------|---------|
| `auth-refresh/alibaba/` | Alibaba auth state refresh |
| `auth-refresh/1688/` | 1688 auth state refresh |
| `source-health/fts/` | FTS source health canary |
| `source-health/alibaba/` | Alibaba source health canary |
| `validation/base/` (7 files: 6 standard + 1 reference index) | Base validator skill with rubric |

#### New TS Module

| Module | Purpose |
|--------|---------|
| `packages/intelligence/src/relations/resolve-process-relations.ts` | Relation resolution engine |
| `packages/manifests/src/render-error-manifest.ts` + template | Error manifest renderer |

### 6.5 ADRs Addressed

| ADR | Decision | L3 Action |
|-----|----------|----------|
| **ADR-004** | PTY-based execution | Full PTY hardening |
| **ADR-005** | Explicit skill binding | Scheduler binds skill id+version at enqueue |

<!-- CROSS-DOMAIN NOTE FOR L0: `canonical_process_id` (uuid, nullable, defaults to self) must exist on `procurement_processes` from creation. This column is introduced conceptually in L3 §3.6 but per the all-columns-from-creation rule, it must be present as nullable in the L0 DDL. On first write, it defaults to the process's own `id`. L3 relation resolution populates it on merge. -->

<!-- CROSS-DOMAIN NOTE FOR L0: `trust_state` (text, not null, default 'unverified') and `source_job_id` (uuid, nullable) belong to the Layer 3 operational activation of `listing_evidence`. The constitutional table shape is reserved in L0, so these fields must be present when that table is first materialized for runtime use. -->

<!-- CROSS-DOMAIN NOTE FOR L2: Trust-state lattice (verified/flagged/unverified/suspect) from L3 §2.7 is finer-grained than the operator-facing `pipeline_assessments.validation_state` field. L2's primary surface remains the coarse process-level states `validated` / `flagged` / `unverified`, but desktop views should still expose finer-grained trust debt where relevant (for example, highlighting `suspect` supporting artifacts or caches behind a `flagged` process rollup). -->

<!-- CROSS-DOMAIN NOTE FOR L2: `canonical_process_id` from L3 §3.6 affects desktop deep links and cluster UI grouping. L2's opportunity detail page should resolve through `canonical_process_id` when navigating. L2's opportunity list should group `related_not_merged` and `possible_duplicate` processes as cluster cards with relation confidence. -->

<!-- CROSS-DOMAIN NOTE FOR L6: Trust-state lattice from L3 §2.7 gates bid workflow. At the bid-readiness surface this maps to two checks: the selected bid scope must contain no `suspect` upstream inputs, and the process-level rollup should surface `pipeline_assessments.validation_state = validated` unless the operator explicitly accepts flagged/unverified trust debt. -->

<!-- CROSS-DOMAIN NOTE FOR L0: Current repo copies of the L0 DDL/migration surface do not yet fully reflect every L3-ready field described here (notably `source_runtime_state.circuit_state`, `source_runtime_state.last_canary_result`, and the full activation surface for `process_relations` / `listing_evidence`). Layer 3 implementation must reconcile the constitutional schema before first write. -->

<!-- CROSS-DOMAIN NOTE FOR L5: Cumulative test count in L5 §3.2 header should be 57 (L4 cumulative 52 + L5 new 5 = 57). The authoritative count is in layer-strategy.md §11. -->

<!-- CROSS-DOMAIN NOTE FOR L6: Cumulative test count in L6 §2.2 header should be 60 (L5 cumulative 57 + L6 new 3 = 60). The authoritative count is in layer-strategy.md §11. -->
