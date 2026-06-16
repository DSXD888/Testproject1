# Harmonization Handoff — Autonomous Daemon Scheduling & ProContract Resumption (RETROACTIVE)

> **Run 5** · Mode: **RETROACTIVE** (feature already implemented + completion-audited 2026-06-12) · Source mode: RESEARCH · Integration type: ARCHITECTURE EVOLUTION + CROSS-CUTTING
>
> **Run this FIRST.** It integrates the new capability into the strategy/layer docs. A built-in fresh-eyes Follow-Up Pass is at the very bottom — send it as a second message in the same session. Then run `harmonization-optimization.md`.

---

## 1. Mission

You are the **chief systems integrator** for the UK public procurement intelligence platform. The **autonomous daemon scheduling & ProContract resumption** feature (epic `testproject1-0c1g`, ~86 beads, completion-audited green on 2026-06-12) is **already implemented and shipped** in the codebase. The strategy/layer docs in `docs/strategy/` still describe the **pre-feature** world — they are stale for this capability. Your job is to make those docs describe the system **as it now actually exists**.

**You are editing documentation only. Do NOT change code.** The deliverable is an updated, internally-consistent doc set that fully absorbs this capability so the research doc becomes redundant.

**Primary goal:** one coherent model of the daemon's autonomous runtime — no contradictions, no stale "schedules only fire at startup" language surviving anywhere.

**Secondary goal:** implementation-faithful docs. Because this is **retroactive mode, the CODE is ground truth.** Where the research doc left a question OPEN, the implementation already answered it — the docs must follow the **code's** answer, not the research doc's menu of options.

---

## 2. Retroactive Mode — Source-of-Truth Hierarchy (READ THIS FIRST)

```
CODE (what was actually built)  >  RESEARCH DOC (intent + rationale)  >  LAYER DOCS (current, stale)
```

- The **CODE** tells you what exists and how it works. Cite it.
- The **RESEARCH DOC** (`docs/research/2026-06-04-autonomous-daemon-scheduling-research.md`, 363 lines) tells you *why* — the design rationale, the hostile-reviewer challenges, the concurrency model. Use it for narrative context, NOT as the spec of record.
- The **LAYER DOCS** describe the pre-feature system. You are fixing them.

The research doc's §8 lists nine open questions (Q1–Q9). **Every one was resolved in code.** You MUST document the *implemented* resolution, not present the open menu. The resolutions (verified against the codebase) are the table in §5 below. The implementation also recorded these as **8 RECORDED operator decisions** in an `AUTONOMOUS_SCHEDULING_STATUS` artifact, asserted by `crates/daemon/tests/ipc_trigger_bid_workflow.rs::decision_001_operator_decisions_recorded` — treat that test as the canonical decision ledger.

**This feature is NOT a new layer.** The platform is L0–L6 complete; there is no L7. This capability was woven retroactively into the **existing** layers. **Do not create a new layer doc.** It lands primarily in **L3 (Operational Hardening)** and **L1 (Thin Local Runner)**, with secondary touches in L0/L2/L4/L5 and the three cross-cutting docs.

---

## 3. MANDATORY Pre-Read (do NOT start editing until this is done)

This is not optional. Do not begin editing after reading only the research doc — cross-document contradictions are only visible after reading everything and comparing it to the code.

Required steps, in order:

1. **Read the research doc in full** — `docs/research/2026-06-04-autonomous-daemon-scheduling-research.md`. Focus on §2 (current-state / before), §4 (architecture), §4.4 (ProContract lifecycle table), §4.5 (commercial sweep), §5 (job-kind routing), §6 (concurrency/risks R1–R8), §8 (open questions). The §2 "current-state findings" describe the BEFORE state — they become historical.
2. **Read the code surfaces in §4 below** for drift awareness (you are NOT changing code — read for factual accuracy). The implementation diverges from the research doc's *plan* in three places (§5 DRIFT rows) — follow the code.
3. **Read every target doc in full** (the primary + secondary docs in §6, plus the three cross-cutting docs). Do not skip docs that "probably aren't affected."
4. **Build a private comparison matrix** with three columns for each concept: *intended design (research doc)* → *as implemented (code)* → *as currently documented (layer docs)*. The deltas between column 2 and column 3 are your edit list.
5. **Only THEN begin harmonizing.**

---

## 4. What To Read — Code Surfaces (ground truth; read, don't edit)

Reading order: research doc (intent) → code (truth) → layer docs (stale state to fix).

**Scheduler tick (§4.1 of research):**
- `crates/daemon/src/scheduler/tick.rs:16-48` — pure tick body `run_scheduler_tick(scheduler, registry, state_db, now: DateTime<Utc>)` — the injected-clock seam (no inline `Utc::now()`).
- `crates/daemon/src/main.rs:862` — `SchedulerTickThread`; spawned in `serve_handshake_service` at `main.rs:5719-5726`; thread name `autonomous-scheduler-tick`. Own `StateDb` connection (`initialize_scheduler_tick_thread`, `main.rs:5468-5492`).
- `main.rs:1006-1007` — `DEFAULT_SCHEDULER_TICK_INTERVAL_SECS = 30`; env override `DAEMON_SCHEDULER_TICK_INTERVAL_SECS`.
- `main.rs:556,565,1736-1738,3121` — one-shot entrypoint `--run-scheduler-once --tick-now <rfc3339>` (`run_scheduler_one_shot` → enqueue → `drain_one_shot_worker_queue` → exit).
- `enqueue_startup_cron_jobs` is **deleted** (Q2 fold); cold-start catch-up runs the tick body once at thread start (`main.rs:4611-4618`, phase `"cold_start"`).
- `sources/registry.rs:35,43,517,536` — `SOURCE_REGISTRY_GENERATION: AtomicU64` + `bump_source_registry_generation()` (Q9); tick reloads only on change via `refresh_scheduler_registry_if_generation_changed` (`main.rs:5581-5610`), re-seeding `last_run_at` for newly enabled sources.

**Worker pump (§4.2):**
- `main.rs:746-749` — `enum AutonomousWorkerLane { PtyBrowser, CoverageDeterministic }` (Q3 = two lanes).
- `main.rs:728-733` — `PTY_BROWSER_WORKFLOW_KINDS = [sync_source, source_health, auth_refresh]`; `COVERAGE_DETERMINISTIC_WORKFLOW_KINDS = [commercial_sweep, validation_cross_model_audit, validation_calibration_readiness_sweep]`.
- `main.rs:4786-4841` — `run_autonomous_worker_loop`; idle backoff 250ms (`AUTONOMOUS_WORKER_IDLE_BACKOFF_MS`, `main.rs:734`).
- `main.rs:978-998,2124-2134,2140-2166` — defer-on-denial = **in-memory** `DenialCooldowns` (no schema column), 500ms base + per-job jitter; `defer_claimed_job_without_attempt` returns job to `pending`, nulls lease, and **compensating-decrements `attempt_count`** to net-zero (so a denial costs no attempt).
- `main.rs:4754-4768,816-840` — **Q1 resolution: each lane opens its OWN `StateDb` connection** (lease-heartbeat precedent); only `ResourceArbiter` is shared (`Arc<Mutex<…>>`).

**Job-kind routing (§5):**
- `main.rs:2169-2202` — `execute_claimed_workflow_job` (replaces the old 3-kind match + `other => fail_job`). Real handlers: `run_sync_source_workflow_job` (`:2205`), `run_source_health_workflow_job`, `run_auth_refresh_workflow_job`, `run_validation_cross_model_audit_workflow_job`, `run_validation_calibration_readiness_sweep_workflow_job`, `run_commercial_sweep_workflow_job` (`:3051`). Retained: `extract_document_structure`, `extract_document_semantics`, `run_bid_workflow`.
- **`enrich_process` remains orphaned** (no arm, in no lane) — desktop `trigger_enrichment` button is still non-functional (anticipated-optional G3 fold-in, not done).

**Live-loop refactor (§4.3):**
- `main.rs:21908-21989` — `SyncSourceRoute::from_key(source_id, skill_id)` (keys off the tuple, since discover/interest/enrich share `source_id`).
- `main.rs:22623-22665` — `run_procontract_enrich_tender_live_loop` (thin `--ingest` wrapper, mints+claims) calls shared `run_procontract_enrich_tender_claimed_live_loop` — the same body the worker invokes. Per-source params derived at execution time (cursor, download_root, awaitingAccessTargets), not from the cron payload.
- `main.rs:1746-1790` — `--ingest` CLI path preserved unchanged.

**ProContract resumption / G4 (§4.4):**
- `migrations/0015_procontract_pending_enrichment.sql` (registered `db/sqlite.rs:31-32,90-92`) — **Q4 = dedicated table.** Columns: `id, process_id, notice_id, advert_id, source_notice_id, state CHECK IN ('awaiting_access','enriched','expired'), interest_registered_at, deadline_at, observation_count, last_checked_at, enriched_at, expiry_reason, owner_id, row_version, last_local_seq, stage_run_id, created_at, updated_at`; `UNIQUE(process_id, notice_id, advert_id)`; indexes on state + advert_id.
- Landmine avoided: `interest_registration.state` stays `'completed'`; re-fire guard gets a **second `NOT EXISTS` anti-join** on the new table (`load_procontract_interest_candidates`, `main.rs:17473-17482`).
- Lifecycle handlers: create `persist_procontract_pending_enrichment` (`:17770`), keep/backfill `keep_or_backfill_procontract_pending_enrichment` (`:18077`), clear `mark_procontract_pending_enrichment_enriched` (`:18157`), expire `expire_stale_procontract_pending_enrichment` (`:18235`).
- **Q5 staleness:** `procontract_pending_expiry_reason` (`:18213-18233`) → notice-level `deadline_at` (reason `"notice_deadline"`) else `PROCONTRACT_AWAITING_ACCESS_MAX_AGE_DAYS = 90` fallback (reason `"max_age"`, `main.rs:1483`). **Never** the RFx response deadline.
- `skills/procurement-ingestion/procontract-enrich-tender/SKILL.md` **v1.3.0** — optional `awaitingAccessTargets: [{advertId, sourceNoticeId, processId}]` input (lines 29-38), union-with-scan + dedupe-by-advertId merge (lines 298-311), backward-compatible when empty. Daemon builds the work-list from `procontract_awaiting_access_targets` (`main.rs:22143-22170`).

**Commercial sweep / G6 (§4.5):**
- `scheduler/cron.rs:24-27,226-232,321-331` — `commercial_sweep` kind; synthetic source `COMMERCIAL_SWEEP_SOURCE_ID = "commercial-sweep"`, skill `pipeline/commercial-sweep`, `COMMERCIAL_SWEEP_CRON = "0 0 7 * * *"` (07:00 UTC daily); scheduler-owned (not in `sources/registry.rs`).
- `main.rs:3051-3111` — `run_commercial_sweep_workflow_job`: in-process `landed-cost` (coverage-current-only) → `run_net_margin_gate_once` → manifest-generation on the **coverage/deterministic lane**. Does **not** call `trigger_pipeline_rerun` (referenced only in a negative test assertion, `main.rs:31064`). Q8 residual chained nudge **not** added.

**Shadow mode (Q6, bead h6s6):**
- `main.rs:2502-2525,2686,2718,2844,2907` — `HEALTH_AUTH_SHADOW_MODE = "shadow"`, `enforcement_action = "recorded_only"`. `source_health` + `auth_refresh` handlers **run and persist their side-effect rows** (`source_runtime_state`, `auth_refresh_results`) but completion is stamped `autonomy_mode=shadow enforcement_action=recorded_only production_gate_applied=false`. Validation + commercial_sweep are **not** gated. **It is a compile-time const, not a runtime config flag.**

**Desktop (Q7, G3):**
- `ipc/handshake.rs:87-93` — `CronStatusSnapshot.{scheduler_running, last_tick_at, next_run_at, worker_lanes, awaiting_access{active_count, expired_count}, definitions}`, built by `build_cron_status_snapshot` (`main.rs:5870`).
- `apps/desktop/src-tauri/src/main.rs:96,1756` — `get_cron_status` Tauri command; rendered in `apps/desktop/src/pages/DaemonStatus.tsx` `AutonomyStatusPanel` (`:271-316`).
- `ipc/handlers/jobs.rs:272` — `trigger_ingestion` (`sync_source`) is now **functional** (claimed by the PtyBrowser lane). `trigger_enrichment` (`enrich_process`) **stays orphaned**.

**Schema epoch / lease:**
- `db/sqlite.rs:96` — `DEFAULT_SCHEMA_EPOCH = 11`. Desktop `EXPECTED_SCHEMA_EPOCH = 11` (`apps/desktop/src-tauri/src/main.rs:51`, `query_service.rs:14`). Moved 10→11 together with migration 0015.
- Research §6 R8: the L4 **writer lease gates remote push to Supabase, NOT local execution** — the worker runs regardless of lease state (`main.rs:3225` disables only push dispatch on lease loss).

**Tests (net-new):** `crates/daemon/tests/scheduler_tick.rs`, `worker_pump.rs` (incl. `shadow_health_auth_001`), `procontract_resumption.rs`, `scheduler_registry_reload.rs`. Updated: `cron_scheduler.rs` (now 14 definitions incl. 07:00 commercial-sweep anchor; catch-up order ends with `commercial-sweep`), `sqlite_schema.rs` (`user_version == 11`, migration `0015_*`).

---

## 5. Open-Question Resolutions — the canonical table (document the CODE answer, not the menu)

| # | Research-doc question | **Implemented resolution (document THIS)** | Notes / DRIFT |
|---|---|---|---|
| Q1 | Worker execution context | **Own `StateDb` connection per lane** (lease-heartbeat precedent); arbiter shared via `Arc<Mutex>` | resolved as research favored |
| Q2 | Tick subsumes startup-enqueue? | **Folded** — `enqueue_startup_cron_jobs` deleted; tick runs once at thread start for cold-start catch-up | — |
| Q3 | Worker lane model | **Two lanes** (PTY/browser + coverage/deterministic) | head-of-line test green |
| Q4 | Awaiting-access modeling | **Dedicated `procontract_pending_enrichment` table**, migration `0015`, epoch **10→11** | landmine avoided via NOT EXISTS anti-join |
| Q5 | Staleness/expiry policy | Notice-level `deadline_at` **else** 90-day `max_age`; **never** RFx response deadline | `PROCONTRACT_AWAITING_ACCESS_MAX_AGE_DAYS=90` (const) |
| Q6 | Shadow-gate health/auth? | **Yes — shadow/record-only**: handlers run + persist side effects but enforcement not applied | **DRIFT: a compile-time const, not a runtime config flag; record-only, not skip-side-effects** |
| Q7 | Desktop status surfacing | **Fully built + rendered** (AutonomyStatusPanel) | **DRIFT (positive): research called this "optional"; it shipped** |
| Q8 | Chained post-ingestion nudge? | **Not added** — scheduled `commercial_sweep` + coverage body only | deferred as research preferred |
| Q9 | Registry-reload trigger | **Generation counter (`AtomicU64`)**, not `MAX(updated_at)` polling | re-seeds newly enabled sources |
| — | `enrich_process` routing (G3 optional) | **Not done** — `enrich_process` stays orphaned; `trigger_enrichment` button still non-functional | **DRIFT: do NOT document this button as working** |

**Three DRIFT points the docs must honor (do not paper over them):**
1. `enrich_process` / desktop `trigger_enrichment` button **remains orphaned** — `trigger_ingestion` works, `trigger_enrichment` does not.
2. Shadow mode is a **hardcoded compile-time const with record-only semantics** (runs + persists side effects, just doesn't enforce) — describe it as a code-level shadow stamp, **not** a configurable rollout flag.
3. Desktop autonomy surfacing was **fully implemented and rendered**, not left optional.

---

## 6. Core Harmonization Goals (specific — what must become consistent everywhere)

1. **Continuous scheduling.** Every doc must consistently state that the daemon re-evaluates due schedules **continuously while serving** via a background `autonomous-scheduler-tick` thread (~30s, injected-clock seam, generation-counter registry reload). The old model — "schedules evaluated only once at process start / quit-and-reopen each morning / restart to pick up schedules" — must be **eliminated or reframed as historical** everywhere it appears (notably the L3 cron section, layer-strategy §5, and any "catchup sweeps on startup" phrasing that implies startup is the *only* trigger).
2. **Always-on two-lane worker.** All docs must consistently describe the always-on **two-lane worker pump** (PTY/browser + coverage/deterministic) that drains `workflow_jobs` and routes **all** cron kinds — replacing the old "only worker matched 3 kinds, reachable only via `--run-next-job`" reality. Include defer-on-denial (in-memory cooldown, no attempt cost) and that the arbiter caps real concurrency.
3. **Job-kind routing is complete except `enrich_process`.** The enumerated job kinds (in L0 `workflow_jobs` DDL/enum, layer-strategy job-kinds list, cross-ref index) must include `commercial_sweep` and reflect that `source_health`/`auth_refresh`/`validation_*`/`sync_source` now have real executors — while stating `enrich_process` is the one kind still unrouted.
4. **Shadow mode** for `source_health` + `auth_refresh` must be documented uniformly as **code-level shadow / record-only** (not a config flag), and explicitly **not** applied to validation or commercial_sweep.
5. **ProContract awaiting-access durable tracking.** All docs touching ProContract ingestion/resumption must consistently describe the dedicated `procontract_pending_enrichment` table, the five lifecycle transitions, the `interest_registration.state='completed'` invariant (the landmine), the Q5 staleness rule (notice-deadline else 90-day max-age, never RFx), and the enrich work-list sweep that makes resumption independent of "Recently added"/`maxPages`.
6. **Autonomous commercial stages.** All docs must consistently state that `landed-cost → net-gate → manifest` now advance autonomously via the scheduled `commercial_sweep` cron kind (~07:00 UTC) executed **in-process on the coverage/deterministic lane** — and that the desktop `trigger_pipeline_rerun` daemon-stop path remains a **manual-only** affordance, explicitly **not** the autonomous mechanism.
7. **Schema epoch is 11.** Every "epoch 10 / current epoch is 10 / EXPECTED_SCHEMA_EPOCH 10 / highest migration 0014" statement must move to **11 / migration 0015**, on both daemon and desktop, consistently across L0, L4, layer-strategy §9, cross-ref index, README.
8. **Desktop autonomy status.** L2 (and the desktop entries in the cross-cutting docs) must describe the `CronStatusSnapshot` → `get_cron_status` → `AutonomyStatusPanel` surface (scheduler running / last tick / next run / awaiting-access counts / per-lane health) and that `trigger_ingestion` is now functional (but `trigger_enrichment` is not).
9. **Lease vs local execution.** L4 must state the writer lease gates **remote push only**, so the autonomous worker runs regardless of lease state.
10. **No new layer.** The capability is woven into existing L0–L6; no L7. Primary homes L1 + L3.

---

## 7. Expected Document Changes (by priority — exact docs + sections)

### PRIMARY targets (substantial changes)

**`docs/strategy/layers/layer-3-operational-hardening.md`** — the flagship home for autonomy.
- **§1.2 Capability Delivered** — the existing "cron-scheduled ingestion … without manual triggers" claim was aspirational; make it true and concrete: continuous tick + always-on worker now deliver it.
- **§1.3 Layer 3 Modules** — add the new runtime surfaces: `scheduler/tick.rs` (tick body), the `SchedulerTickThread` + two `AutonomousWorkerThread` lanes in the serve loop, `DenialCooldowns`. Note `scheduler/cron.rs` now defines the `commercial_sweep` synthetic slot.
- **§1.4 Resource Arbiter** — the autonomous worker honors the same typed quotas; `procurement_source_lanes=1` serializes same-source runs (R7); defer-on-denial backoff.
- **§1.5 Cron Scheduler** — **rewrite the trigger model**: from "catchup sweeps on startup" (startup-only) to **continuous ~30s tick** with cold-start catch-up folded in, injected-clock seam, generation-counter registry reload, the one-shot `--run-scheduler-once --tick-now` entrypoint. Add the `commercial_sweep` 07:00 slot and the full cron-kind→executor routing.
- **§1.7 Host Environment Awareness** — catch-up after sleep/wake now happens on the next tick (not only at restart).
- **§1.9 New Skills** — source-health/auth-refresh now have **daemon job-handlers** (previously skills with no executor), running in **shadow/record-only** mode.
- **§2.3 Validation Policy** — `validation_cross_model_audit` + `validation_calibration_readiness_sweep` now have worker executors on the coverage/deterministic lane (not shadow-gated).
- **NEW operational subsection (place under §1, near cron/arbiter): ProContract awaiting-access resumption** — the `procontract_pending_enrichment` table, five-transition lifecycle, the `interest_registration.state='completed'` landmine, Q5 staleness, the enrich work-list sweep. (This is an operational-hardening concern; it belongs in L3, cross-referenced from wherever ProContract ingestion is described.)

**`docs/strategy/layers/layer-1-thin-local-runner.md`** — runtime/live-loop surface.
- **§1.3 Layer Narrative / §1.1 "What's NOT in Layer 1"** — L1's "no cron scheduler (manual CLI trigger only)" boundary is **still correct for L1's own scope** (cron is L3). Do **not** claim L1 gained cron. But add a forward-pointer that the L1 live-loop bodies were **refactored** so a worker-claimed queued job can drive them (shared `*_claimed_live_loop` body + thin `--ingest` wrapper, `SyncSourceRoute::from_key(source_id, skill_id)`), and that the L3 serve loop now hosts the scheduler-tick + worker threads.
- **§1.6 Ingestion Flow** — note the live loop is now drivable by a claimed queued job (per-source param derivation at execution time), with `--ingest` preserved as the manual path.
- **§2.1 Daemon Core Components** — reflect that the serve loop is no longer IPC-only; it spawns the tick + worker lanes (the IMPL deviation note already acknowledges later-layer modules exist).

### SECONDARY targets (consistency alignment)

**`docs/strategy/layers/layer-0-gold-set-evaluation.md`**
- **§2.5.2 `workflow_jobs` DDL + job-kind enum** — add `commercial_sweep`; ensure the enum/notes reflect the now-routed kinds. (Leave **§1.4 MVE** "no daemon, no cron" alone — accurate for the pre-L1 MVE.)
- **§2.2.1 Schema Evolution** — record the epoch **10→11** advance and migration `0015_procontract_pending_enrichment`; add the table to the constitutional schema inventory.

**`docs/strategy/layers/layer-2-single-operator-desktop.md`**
- **§1.5 DaemonBridge** — `trigger_ingestion` is now functional (worker consumes `sync_source`); `trigger_enrichment` (`enrich_process`) **remains orphaned**; add the read-only **autonomy status** surface (`get_cron_status` → `AutonomyStatusPanel`). Clarify `trigger_pipeline_rerun` stays a **manual-only** daemon-stop affordance, not the autonomous commercial trigger.

**`docs/strategy/layers/layer-4-shared-visibility.md`**
- **§1.7 Schema Epoch Guard** — "current epoch is 10" → **11**, last bumped by migration `0015`.
- **§1.6 Active-Writer Lease** — clarify the lease gates **remote push only**; the autonomous worker executes locally regardless of lease state.

**`docs/strategy/layers/layer-5-expanded-commercial-intelligence.md`**
- **§1.2 / §2.4** — the commercial stages (landed-cost/net-gate/manifest) now **advance autonomously** via the L3 `commercial_sweep` cron kind (~07:00, in-process on the coverage/deterministic lane) — add a pointer to the L3 mechanism; do not duplicate the mechanism here.

### CROSS-CUTTING (must end consistent with the above)

**`docs/strategy/layer-strategy.md`**
- **Header (~line 11)** — the "Layer 1 … cron scheduler" bullet misattributes cron to L1; cron belongs to L3. Fix the attribution and add an "Updated 2026-06-12 — autonomous daemon scheduling harmonization" note (mirroring the existing goods-led update note style).
- **§5 (L3)** — the master narrative for the continuous tick + two-lane worker + new job kinds + ProContract resumption. This is the cross-cutting mirror of the L3 doc changes.
- **§3 (L1)** — the live-loop refactor note.
- **§9 Schema Stability Matrix** — epoch 10→11; add a `procontract_pending_enrichment` row (authority, created/first-active-use); the L4 epoch note (~line 892 "current epoch is 10") → 11.
- **Job-kinds list (~line 1229)** — add `commercial_sweep` (and reconcile the "20 job kinds" count if it changes).
- **§10 Interface Contracts** — add the `CronStatusSnapshot` autonomy fields.
- **Appendix Component Activation** — reflect the tick + worker threads activating in the serve loop.

**`docs/strategy/cross-reference-index.md`**
- **Add new concept→location rows**: `autonomous-scheduler-tick` (L3 §1.5 / `scheduler/tick.rs`), `two-lane worker pump` (L3), `commercial_sweep` cron kind (L3 §1.5 / `scheduler/cron.rs`), `procontract_pending_enrichment` table + `awaiting-access` lifecycle (L3 + L0 schema matrix), shadow/record-only health-auth, desktop autonomy status (L2), one-shot `--run-scheduler-once` entrypoint.
- **Update** existing cron/job-claim/arbiter rows to point at the new surfaces; update the **Implementation Status Snapshot** (epoch 11, autonomy shipped 2026-06-12).

**`docs/strategy/README.md`**
- Update the status line (epoch **11**; autonomous daemon scheduling shipped 2026-06-12) and the **L3 capability** summary; mirror the existing "Update 2026-06-04 …" convention with a 2026-06-12 line.

### LIGHT TOUCH / VERIFY ONLY

**`docs/strategy/layers/layer-6-request-pipeline-automation.md`** — ProContract **resumption** here is *ingestion-side* (awaiting-access enrich), distinct from L6's **bid-workflow** `AUTH_STATE_STALE` resumption pattern. Verify the two are not conflated and add a one-line cross-reference if a reader could confuse them. Only edit if a genuine contradiction exists.

---

## 8. Integration Rules

**After harmonization, these must hold:**
- No doc says or implies schedules fire only at startup / require a restart-or-reopen to pick up. No doc says the worker matches only 3 kinds or that cron kinds are orphaned (except `enrich_process`, which IS still orphaned).
- No doc says "current epoch is 10" / highest migration `0014`. Both daemon and desktop are epoch **11** / migration `0015`.
- No doc claims `trigger_enrichment` works or that `trigger_pipeline_rerun` is the autonomous commercial trigger.
- No doc describes shadow mode as a runtime config flag.
- Terminology is uniform: `autonomous-scheduler-tick` (not "scheduler loop"/"poll loop"), `commercial_sweep` (exact kind name), `procontract_pending_enrichment` (exact table name), "coverage/deterministic lane" vs "PTY/browser lane".

**Must stay OUT of the docs:**
- Implementation task sequencing, bead IDs, "first implement X then Y" language, tool/dev-workflow instructions. (The architecture docs describe WHAT the system is, not HOW it was built. Bead/epic IDs may appear only as a provenance footnote if the doc already uses that convention — do not introduce it.)

**Style:**
- Preserve each doc's local section structure and voice. **Integrate into existing sections** — do not append "Autonomous Scheduling" appendices. The one genuinely-new subsection (L3 ProContract awaiting-access) must be properly placed under L3 §1, not bolted to the end.
- Honor the existing IMPL/deviation annotation convention (✓ As designed, ⚠ Deviation, ✗ Deferred, ↻ Superseded) where docs already use it — e.g. mark `enrich_process` as ✗ Deferred and the desktop surfacing as ⚠/✓ as appropriate.
- Don't churn unaffected commercial/customs/validation-internals content.

---

## 9. What A Great Outcome Looks Like

- An engineer reading **only** the harmonized strategy docs understands the daemon's autonomous runtime end-to-end: it ticks continuously, drains a two-lane queue, fires `commercial_sweep` at 07:00, tracks ProContract awaiting-access durably, runs health/auth in shadow — without ever opening the research doc or the code.
- The docs clearly distinguish: **continuous tick** (new) vs **startup-only** (old/historical); **autonomous `commercial_sweep`** vs **manual `trigger_pipeline_rerun`**; **`trigger_ingestion` works** vs **`trigger_enrichment` orphaned**; **shadow/record-only** vs **enforced**; **ingestion-side awaiting-access resumption** vs **L6 bid-workflow auth resumption**.
- The docs no longer imply the daemon must be restarted/reopened to pick up schedules.
- Epoch **11** is consistent on both sides everywhere.
- Edit until this standard is met — not until you've touched the obvious files.

---

## 10. Deliverable

Updated docs (primary: L3, L1; secondary: L0, L2, L4, L5; cross-cutting: layer-strategy.md, cross-reference-index.md, README.md; verify: L6). Plus a concise change summary: which docs changed, which goals (§6) each satisfied, and any residual ambiguity for the optimization pass.

At minimum expect to modify: **layer-3, layer-1, layer-0, layer-2, layer-4, layer-5, layer-strategy.md, cross-reference-index.md, README.md** (nine files).

---

## 11. Final Verification Checklist (before you stop)

- [ ] Every modified doc re-read after editing.
- [ ] No doc contradicts the **code** (retroactive: code wins over research doc).
- [ ] No doc contradicts another target doc on any of the 10 goals (§6).
- [ ] No "schedules fire only at startup" / "restart to pick up" survives anywhere.
- [ ] No "worker matches 3 kinds" / "orphaned cron kind" survives (except the intentional `enrich_process`).
- [ ] Epoch **11** / migration `0015` consistent across L0, L4, layer-strategy §9, cross-ref index, README — both daemon and desktop.
- [ ] `commercial_sweep` documented as autonomous + in-process; `trigger_pipeline_rerun` documented as manual-only.
- [ ] Shadow mode = code-level record-only, not a config flag; not applied to validation/commercial_sweep.
- [ ] ProContract: dedicated table, `interest_registration.state='completed'` invariant stated, Q5 staleness (never RFx), work-list sweep.
- [ ] `trigger_ingestion` documented working; `trigger_enrichment` documented orphaned.
- [ ] No implementation sequencing / bead IDs leaked into architecture docs.
- [ ] No new layer doc created.

---

## 12. Follow-Up Pass (built-in fresh-eyes — send as a SECOND message in the same session)

> Send the following as a follow-up message after the harmonization above is complete.

**Fresh-eyes contradiction hunt.** Re-read the research doc and every doc you modified with completely fresh eyes. Then hunt specifically for these terms/phrases across the **entire** `docs/strategy/` tree and fix any stale or contradictory survivor:

- "only … at startup", "at process start", "at bootstrap", "quit and reopen", "restart to pick up", "manual CLI trigger only" (re: autonomy — make sure L1's legitimate "cron is L3" boundary is preserved, but no doc implies the *running* daemon can't tick)
- "run_next_workflow_job", "three kinds"/"3 kinds", "--run-next-job" (as the only drainer), "orphaned"/"unsupported queued job kind"
- "epoch 10", "current epoch is 10", "EXPECTED_SCHEMA_EPOCH", "DEFAULT_SCHEMA_EPOCH", "0014" (as the highest migration)
- "re-run pipeline", "trigger_pipeline_rerun", "trigger_enrichment", "trigger_ingestion"
- "Recently added", "maxPages" (as the only enrich resumption mechanism)
- "commercial_sweep", "procontract_pending_enrichment", "awaiting_access", "shadow"

Verify: (1) no doc claims `trigger_enrichment` works; (2) shadow mode is never described as a runtime flag; (3) `commercial_sweep` is never described as using `trigger_pipeline_rerun`; (4) the ProContract `interest_registration.state='completed'` landmine invariant is stated wherever the table is introduced; (5) epoch 11 is consistent on both daemon and desktop sides. Bump any changelog/"Updated" header lines to 2026-06-12 where the doc uses that convention. Fix everything you find.
