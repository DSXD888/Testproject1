# Autonomous Daemon Scheduling & ProContract Resumption — Research

- **Date:** 2026-06-04
- **Status:** Research (input to pipemaxxer → spec → TDD → strategy → beads)
- **Author:** Owner + Claude (codebase-verified)
- **Scope area:** `crates/daemon/` (Rust runtime); skill-side touch in `skills/procurement-ingestion/procontract-enrich-tender/`; minor `apps/desktop/` touch for optional status surfacing (§9, Q7)
- **Related memory:** Desktop/daemon schema-epoch coupling; L6 archived/synthetic smoke test

---

## 1. Intent (what the owner actually wants)

Leave the desktop app (and therefore the daemon) open **perpetually** and have it:

1. **Fire its scheduled scraping runs autonomously** at their configured times (e.g. FTS ingest 04:15 UTC, Contracts Finder 04:45, ProContract discover/interest/enrich 05:00/05:30/06:00) — with **no quit-and-reopen and no manual button clicks**.
2. **Run the whole pipeline through** once a schedule fires, end to end, using the operator's already-authenticated browser sessions (the existing skill-first / PTY model — no API connectors).
3. **Not silently drop a contract** that interest was registered for but where the portal hasn't granted access yet by the end of a run (the "no reply in 2–3 hours / overnight" case).

This is explicitly a **runtime-autonomy** change, not a change to *what* the skills scrape. The schedules, skills, sources, job queue, resource arbiter, and live-loop scrapers already exist; the scrapers execute today via the `--ingest <source>` CLI (and the desktop pipeline-rerun, which shells out to `--ingest`). Note the desktop *trigger* buttons do **not** currently execute anything — see §2.2.

> Precision on "leave the app open": the daemon is spawned **detached** with `--serve` (`apps/desktop/src-tauri/src/main.rs`) and survives the desktop app closing — so the practical requirement for autonomy is the **daemon process alive + the host awake + the browser sessions the scheduled skills need being available and authenticated**. For the authenticated portals (ProContract, and the bid portals) that specifically means the operator's persistent Chrome/CDP session reachable on `:9222` (those PTY scrapers attach to that browser; see the enrich skill's CDP discipline); public sources may not need a logged-in session but still run as browser automation. The desktop app being open is the convenient way to keep the daemon alive, but it is not, by itself, what makes runs fire. Anything that quits the host, sleeps it, or kills the CDP browser breaks the portal runs regardless of the app.

---

## 2. Current-state findings (codebase-verified)

### 2.1 The schedule is evaluated only once, at process start

- `CronScheduler::dispatch_due_jobs()` (`crates/daemon/src/scheduler/cron.rs:120`) has **exactly one caller**: `enqueue_startup_cron_jobs()` (`crates/daemon/src/main.rs:3686`), which is called **once** from `bootstrap_runtime()` (`main.rs:2924`).
- The serve loop `serve_handshake_service()` (`main.rs:3808`) is a **blocking** `loop { runtime.listener.accept() … }` that only handles desktop IPC handshake/command connections. It has **no timer, no tick, and never dispatches due jobs or executes them**. It *does* construct a `CronScheduler` per handshake — but only to build a read-only `CronStatusSnapshot` (`build_cron_status_snapshot`, `main.rs:3879`) for displaying next-run times to the desktop; that path never enqueues or runs anything.
- **Consequence:** while the daemon sits open, "is it 04:15 yet?" is never re-asked. Schedules only fire (with catch-up for missed slots, via `seed_cron_scheduler_from_history`, `main.rs:3725`) at the next **bootstrap** — i.e. on app/daemon (re)start. This is the "quit and reopen each morning" caveat.

### 2.2 Nothing drains the job queue autonomously while serving

- The cron scheduler, when it does dispatch, enqueues rows into `workflow_jobs` with `kind ∈ { sync_source, source_health, auth_refresh, validation_cross_model_audit, validation_calibration_readiness_sweep }` (`scheduler/cron.rs:341` `build_enqueue_request`).
- The only queue worker, `run_next_workflow_job()` (`main.rs:1703`), matches **only three kinds** — `extract_document_structure`, `extract_document_semantics`, `run_bid_workflow` — and sends **everything else** to `other => fail_job(...)` (`main.rs:1722`). So a cron-enqueued `sync_source` job is **not executable** by today's worker; it would fail as "unsupported queued job kind".
- `run_next_workflow_job` is only reachable via the **`--run-next-job` CLI flag** (`main.rs:1424`), which claims and runs **one** job and exits. It is never called from the serve loop.
- The actual source scrapers (`run_fts_live_loop`, `run_contracts_finder_live_loop`, `run_procontract_discover_live_loop`, `run_procontract_register_interest_live_loop`, `run_procontract_enrich_tender_live_loop`, and the marketplace search loops) are reached **only via the `--ingest <source>` CLI flag** (`main.rs:1379`). Each **mints its own `sync_source` job and claims it by ID** (e.g. `create_source_sync_job_from_parameters` + `claim_job_by_id`, `main.rs:19658-19668`) rather than consuming a queued job. So a cron-enqueued `sync_source` row is effectively **orphaned** by the live-loop path.
- The downstream commercial stages — `landed-cost`, `net-gate`, `manifest` — are a **different shape**: they are *not* source scrapers, *not* `sync_source` jobs, and **not cron-bound**. Each allocates a `stage_run_id` and runs a coverage pass over eligible DB rows (verified: `run_landed_cost_live_loop`, `main.rs:10532`; `run_net_margin_gate_live_loop` → `run_net_margin_gate_once`, `main.rs:12403`; `run_manifest_generation_live_loop`, `main.rs:15043`). Today they fire only via the desktop **pipeline-rerun** (`trigger_pipeline_rerun`) or `--ingest <stage>` — there is no autonomous trigger and no automatic chaining from ingestion. See **§2.5**.
- The desktop *trigger buttons* hit the same dead-end: `trigger_ingestion` (IPC) enqueues a `sync_source` job and `trigger_enrichment` enqueues an `enrich_process` job (`ipc/handlers/jobs.rs:222`+), each returning an ack — but **neither kind is consumed by any worker today** (`enrich_process` appears in no handler; `run_next_workflow_job` rejects both). So clicking "Run ingestion" / "Rerun enrichment" currently *enqueues an orphaned job and runs nothing*. (The worker pump in §4.2 is also what would make these buttons actually execute — see G3.)
- The only long-lived background thread today is the **L4 writer-lease heartbeat** (`SyncLeaseHeartbeat::spawn`, `main.rs:2978`), which opens its **own** `StateDb` connection on an interval (`recv_timeout`) and refreshes the writer lease. It does **not** run jobs.

**Net:** two stacked gaps in the cron→execution path. (1) No recurring schedule tick. (2) No always-on worker that can execute the cron job kinds. Both must be closed for autonomy — and a third, related gap (the post-ingestion commercial stages have no autonomous trigger at all) is covered in §2.5.

### 2.3 Concurrency foundation already exists (de-risks the change)

- `open_state_db` sets `PRAGMA journal_mode = WAL`, `synchronous = NORMAL`, `busy_timeout = 5000`, `foreign_keys = ON` (`crates/daemon/src/db/sqlite.rs:215-218`).
- The lease-heartbeat thread already demonstrates the supported pattern: a background thread **opens its own connection to the same `state.db` file** and writes safely under WAL + busy-timeout.
- `RuntimeBootstrap.state_db` is owned (not `Arc`/`Mutex`), so background threads must **not** share that handle — they open their own connection (same precedent). `resource_arbiter` is already `Mutex<ResourceArbiter>`.

### 2.4 ProContract resumption today is portal-mediated, not daemon-tracked

- Register-interest correctly fires **once**: candidate selection excludes notices that already have an `interest_registration` stage in state `completed`/`succeeded`/`skipped` (`load_procontract_interest_candidates`, `main.rs:15371`; `persist_custom_process_stage_state`, `main.rs:15600`). It will not re-express interest. ✅ correct.
- The **enrich** stage is what resumes awaiting-access items, and it reads **no daemon state** — it re-scrapes the portal's "My Activities → Recently added" list from scratch each run (`skills/procurement-ingestion/procontract-enrich-tender/SKILL.md`, Step 2; paginates all pages up to `maxPages: 5`). For each activity it looks for a **live tender event** ("Not started (Respond by: …)") and skips "Expression of interest accepted".
- When interest is registered but no live tender event exists yet (access not granted), the skill counts it in `summary.not_yet_responded` and **leaves it in place**; it is **not persisted** anywhere durable. The `cursor` (`{"last_checked_at":…,"pages_checked":…}`) is a soft progress marker, **not** a hard exclusion filter.

**Therefore:** the resumption *mechanism* is sound — whenever enrich runs, it re-scans the portal and picks the item up once access appears, so a 2–3 hour or overnight delay is handled and the portal acts as the durable queue. (Caveat that connects to §2.2: the cron enrich job is currently **orphaned**, so today "the next enrich run" happens only when the operator manually invokes `--ingest procontract-enrich` — the autonomy work in §4 is what makes it run daily on its own.) **But** even with enrich running, the safety net is bounded by the enrich skill's scan window ("Recently added", `maxPages: 5`, ~50 rows). At higher volume, an item stuck awaiting access for many days while newer activities pile in could **age out past page 5** and be missed, because the daemon keeps no "awaiting access" record and runs no sweep over such items.

> Volume note: at current pre-revenue deal flow this gap is effectively inert (~50-row window >> live awaiting-access set). The owner has chosen to harden it now anyway, folded into this work.

### 2.5 "Run the whole pipeline through" is broader than firing cron

The owner's intent #2 ("run the whole thing") is **not satisfied by the scheduler-tick + worker-pump alone.** Cron only schedules **source ingestion** (FTS, Contracts Finder, ProContract discover/interest/enrich), plus canaries, auth-refresh, L5 watchers, and validation sweeps. The **commercial stages that turn raw notices into a bid decision — `landed-cost` → `net-gate` → `manifest` — are not cron-bound and have no autonomous trigger** (per §2.2). So even after autonomous ingestion populates new processes, nothing today advances them through the commercial pipeline without an operator pressing "Re-run pipeline".

There is one useful precedent already in the daemon: `run_deterministic_pipeline()` (`main.rs:23323`, invoked by `--pipeline <source_id>` at `main.rs:1423`) runs extract-lots → prequalify → qualify-lot → reclassify-candidates → score-quoteability → gross-margin-gate → relate **synchronously in-process** against `&mut runtime.state_db`, without stopping the daemon. It covers qualification through gross-margin/relate, **not** the commercial `landed-cost` → `net-gate` → `manifest` chain. The design question is therefore not "can the daemon run deterministic chains in-process?" — it already can — but whether the commercial chain should be queued through the worker or folded into a broader in-process "advance-pipeline" sweep.

Achieving true end-to-end autonomy therefore requires a **third mechanism**: how the commercial stages get triggered. The candidate models — (a) chain after a successful ingestion run; (b) own daily schedule slot after the ingestion window; (c) coverage-driven — are **resolved in §4.5** to a single design that collapses (b) and (c): a scheduled `commercial_sweep` cron kind (~07:00) whose body is the existing coverage-driven pass, executed in-process by the worker's coverage/deterministic lane (no daemon stop). This was the part most likely to be under-scoped if left implicit; it is now pinned (see §4.5, Q8).

---

## 3. Goals / Non-goals

**Goals**
- G1. Daemon fires due schedules autonomously while running continuously (no restart, no manual trigger), including catch-up after sleep/wake.
- G2. Cron-enqueued jobs (`sync_source`, `source_health`, `auth_refresh`, `validation_*`) execute end-to-end via an always-on worker.
- G3. The CLI manual paths (`--ingest`, `--run-next-job`) keep working unchanged. The desktop **ingestion** button (`trigger_ingestion` → `sync_source`) becomes **functional for free**, because the worker already routes `sync_source` (today it just enqueues an orphaned job — §2.2). The desktop **enrichment** button (`trigger_enrichment` → `enrich_process`) is a *separate* orphaned kind that cron never enqueues; making it work needs the worker to also route `enrich_process` to a process-enrichment handler — optional, fold in only if wanted (§5).
- G4. ProContract awaiting-access items are tracked in daemon state and swept by enrich regardless of the portal's "Recently added" ordering, so resumption can't age out.
- G5. Concurrency-correct under WAL single-writer with the existing serve thread + lease heartbeat + new scheduler/worker threads.
- G6. After autonomous ingestion, the commercial pipeline (`landed-cost`→`net-gate`→`manifest`) advances without an operator pressing "Re-run pipeline" — satisfying intent #2 (§2.5/§4.5).

**Non-goals**
- N1. No change to scraping logic / page selectors beyond what G4 requires.
- N2. No move to API connectors; PTY/browser-session model stays.
- N3. No multi-org / multi-daemon concurrency (one daemon per org, per existing MVP decision).
- N4. No new remote/HTTP trigger surface.

---

## 4. Proposed architecture (for spec to refine)

### 4.1 Recurring scheduler tick
A background thread modelled on `SyncLeaseHeartbeat`: opens its own `StateDb` connection and loops on `recv_timeout(tick_interval)` (default ~30–60s, configurable). **Build the `CronScheduler` and seed `last_run_at` from history once, at thread start** — do *not* rebuild/re-seed every tick (wasteful, and re-deriving state from the DB each pass invites drift). Then each tick:
1. Reads `now` (see clock seam below), then calls `dispatch_due_jobs(&registry, now)` against the in-memory scheduler and enqueues due jobs. Enqueue dedups via `enqueue_job → find_existing_enqueue_ack` (read-before-write on idempotency key `cron:{source}:{skill}:{scheduled_for}` — `jobs/runner.rs:585`), so a duplicate slot returns the existing ack instead of inserting a row.
2. Calls `record_enqueued` in-memory so `next_run_at` advances.
3. Reloads the source registry **only when it changes** — see the reload-trigger mechanism below.
- **Clock-injection seam (design requirement, not just a test hook).** The tick thread must take its `now` from an injected clock source, not call `Utc::now()` inline. Concretely: the tick body is `fn run_scheduler_tick(scheduler, registry, db, now: DateTime<Utc>)` and the thread's loop supplies `now`; production supplies `Utc::now()`, tests supply a fixed `ts(2026,…)` instant (the seam `dispatch_due_jobs(&registry, now)` already takes `now` at the scheduler layer — this requirement extends the same threading up through the tick thread so a test can assert "it is 04:15" without advancing a live `--serve` wall clock). Without this seam the end-to-end autonomy test (§7) is unwritable; it is therefore in scope, not optional. Expose it as a **one-shot autonomous entrypoint** (`--run-scheduler-once --tick-now <rfc3339>`: evaluate schedules at the injected instant → enqueue → drain worker to quiescence → exit) — this is what the §7c M1/M2 manifests drive (a live `--serve` daemon can't be `.output()`-ed without hanging — see the §7c harness note).
- **Registry-reload trigger (pin the mechanism, don't poll).** "Reload only when it changes" needs a concrete signal. Recommended: a monotonic **registry generation counter** (`AtomicU64`) bumped by every IPC source-mutation handler (enable/disable/add source — `ipc/handlers/`); the tick reads the counter cheaply each pass and reloads `SourceRegistry::load` + rebuilds the in-memory schedules **only when the observed generation differs** from the last reload. This avoids both per-tick DB reloads and missed mutations. *Residual Q for spec (Q9):* if a simpler poll-the-`sources`-table-`updated_at`-max each tick is preferred over wiring a counter through every mutation site, that is acceptable but state which. Either way, on reload the new schedules must re-seed `last_run_at` from history for *newly enabled* sources so a just-enabled source doesn't fire a spurious catch-up burst.
- Sleep/wake: rely on the existing catch-up math (`schedule.after(last_run_at)`); a long monotonic gap on wake naturally produces an overdue slot that the next tick enqueues. Consider the existing host-sleep detection (`pty/health.rs`) for tainting in-flight PTY only.
- **Decision (resolved — fold, see Q2).** Fold startup-enqueue and steady-state dispatch into the **one** tick path: run the tick body once immediately at thread start (covers cold-start catch-up, replacing `enqueue_startup_cron_jobs`'s sole purpose), then on interval. This is safe because the **bootstrap-vs-first-tick race is dedup-closed**: both compute the *identical* idempotency key `cron:{source}:{skill}:{scheduled_for}` for a given slot, and `find_existing_enqueue_ack` (read-before-write) makes the second insert a no-op returning the first ack. (If `enqueue_startup_cron_jobs` is retained at bootstrap for a transition period, the same key still makes the double-dispatch a no-op — so retaining it is a simplicity regression, not a correctness risk.) Lean: delete the separate startup path once the tick lands.

### 4.2 Continuous worker pump
A background worker that, in a loop: claims the next ready job (`JobRunner::claim_next_job`, ordered by `COALESCE(scheduled_for, created_at), created_at` — so an overdue cron slot is picked before newer manual work), routes by `kind`, executes, completes/fails, then continues. When `claim_next_job` returns `None` (empty queue), **poll with a short idle backoff rather than busy-spinning**. Respects the existing `ResourceArbiter` (browser slots default 2, agent slots 4, source lanes) so concurrent PTY runs don't exceed quotas. (The worker writes local state directly — local SQLite is operational authority. The L4 **writer lease** gates *remote push to Supabase*, **not** local execution — `main.rs:3225` disables only "push dispatch" on lease loss — so the worker runs regardless of lease state; under the one-daemon-per-org model (N3) the lease is normally held anyway.)
- **Worker-lane model (resolved — replaces the bare "single vs pool" framing of Q3).** Do **not** run a single serial worker (a long PTY `sync_source` run would head-of-line-block cheap deterministic stages queued behind it) and do **not** run a homogeneous pool that ignores job character. Instead split into **two lanes by job class**, because the arbiter *already* distinguishes resource classes:
  - a **PTY/browser lane** for `sync_source`, `source_health`, `auth_refresh` (browser-backed; sized to `browser_slots=2`), and
  - a **coverage/deterministic lane** for the commercial-sweep kind (§4.5) and the `validation_*` kinds (not source scrapers and no procurement-source browser slot; `landed-cost`/`manifest` may still consume agent/skill capacity, so the arbiter remains in force).
  Each lane is its own claim loop, filtering `claim_next_job` by kind-class. **Falsifiable claim the spec must hold or refute:** with lane separation, a long `sync_source` PTY run never delays a ready `commercial_sweep`/`validation_*` job — provable by the §7 head-of-line test (enqueue a slow PTY job + a coverage/deterministic job, assert the latter completes before the PTY one). If that test can't be made to pass with lane separation, fall back to a kind-aware pool. The arbiter caps *real* concurrency either way, so over-claiming is bounded.
- Job-kind routing must cover the cron kinds (see §5), extending today's 3-kind match. Note the asymmetry: `sync_source` reuses refactored live-loop logic (§4.3), but `source_health`, `auth_refresh`, and both `validation_*` kinds have **no executor today** and need new handlers — so this is more than wiring, it's new execution paths (the underlying skills exist for the first two).
- A long PTY run must **not** block the scheduler tick — keep tick and worker(s) on **separate** threads/tasks.
- **Defer-on-denial with backoff (fully specified).** A claimed job that can't get its resource lease (browser/agent/lane slot exhausted — `ResourceArbiter` returns `ArbiterDecision::Denied(DenialReason::…)`) must be **released back to `pending`** with **no `attempt_count` bump and no held lease** (distinguish "denied admission → defer" from "ran → failed"). To stop a permanently over-subscribed queue from hot-looping the same denied job, the deferral applies a **denial backoff**: stamp a `next_eligible_at = now + denial_backoff` (short, e.g. the idle-poll interval, jittered) and have `claim_next_job` skip jobs whose `next_eligible_at` is in the future — or, minimally, the lane sleeps the backoff interval before re-claiming after any denial. This must **not** reuse the failure/retry `attempt_count` machinery (a denial is not an attempt). *Spec note:* if `next_eligible_at` is modeled as a column it is a schema change (§6 R6); if modeled as an in-memory per-lane cool-down it is not — prefer in-memory to avoid an epoch bump (Q4-adjacent).

### 4.3 Live-loop refactor (the meaty part)
Today `run_*_live_loop` functions take `&mut RuntimeBootstrap` and mint+claim their own job. Refactor so the same scraping logic can be driven by a **claimed queued job** handed in by the worker:
- Extract the per-source execution body into a function that accepts (claimed job, a runtime/services handle, cli/config) and does **not** assume it created the job.
- Keep thin `--ingest` CLI wrappers that enqueue+claim then call the shared body (preserves the `--ingest` CLI path, G3).
- Map `sync_source` job `input_payload.source_id` → the right source body (FTS, contracts-finder, procontract discover/interest/enrich, and the marketplace search loops). Note: only FTS, contracts-finder, and the three procontract stages are cron-bound today; marketplace search is `--ingest`-only. The commercial stages (landed-cost/net-gate/manifest) are **not** in this mapping — they are `stage_run_id`-keyed passes handled in §4.5, not `sync_source`.
- **Payload mismatch to handle:** the cron-enqueued `sync_source` `input_payload` carries *schedule metadata* (`source_id`, `skill_id`, `skill_version`, `cron_expression`, `scheduled_for`) — **not** the per-run execution parameters the CLI wrappers compute today. The shared body must **derive those at execution time** from `source_id`/state, not assume the job payload already contains them. This is why §4.3 is a refactor of the *body*, not just a new `match` arm. The derivation is **per-source and must be specified for each cron-bound source** (a generic "derive from state" is under-scoped):

  | `source_id` | Body to invoke | Parameters derived at execution time (from state, not payload) |
  |---|---|---|
  | `fts` | `run_fts_live_loop` body | artifact `download_root` from artifact root; page/window defaults from registry |
  | `contracts-finder` | `run_contracts_finder_live_loop` body | as FTS |
  | `procontract` (discover) | `run_procontract_discover_live_loop` body | `download_root`; discover window defaults |
  | `procontract` (register-interest) | `run_procontract_register_interest_live_loop` body | candidate set from `load_procontract_interest_candidates` (state query) — **not** a payload list |
  | `procontract` (enrich) | `run_procontract_enrich_tender_live_loop` body | **`cursor` from `source_cursors`**, **`download_root`** from artifact root, **and the awaiting-access work-list** from the new G4 tracking table (§4.4) |

  Note discover/interest/enrich share `source_id=procontract` but are distinct skill bindings (different `skill_id`); the body mapping must key off `(source_id, skill_id)`, not `source_id` alone — confirm against how `dispatch_due_jobs` freezes `skill_id` into the enqueue payload (`cron_scheduler.rs` test 003 freezes skill binding).
- Decide the runtime/state handle a worker thread uses: own `StateDb` connection (lease-heartbeat precedent) vs a shared services struct. **This is the highest-risk design decision** — see §6.

### 4.4 ProContract resumption hardening (G4)
- **Create** a durable **awaiting-access** record when register-interest *succeeds* (interest is in; tender access/documents pending). Register-interest does **not** itself check for a live tender event — that's enrich's job — so the trigger is simply successful registration, not "no live event yet."
  - **Correctness landmine — do not overwrite `interest_registration` state.** The re-registration guard (§2.4) excludes only `state IN ('completed','succeeded','skipped')`. If awaiting-access is modeled by flipping the `interest_registration` row's state to `awaiting_access`, that row stops matching the guard and **register-interest will re-express interest every run.** Therefore prefer a **dedicated `procontract_pending_enrichment` row** (keyed by `process_id`/`notice_id`), leaving `interest_registration.state = 'completed'` intact. If a shared `process_stage_status` row *is* reused instead, the guard's state-set must be widened to also treat `awaiting_access` as "already registered."
  - **Schema-cost of "dedicated" depends on shape (Q4, R6).** A dedicated tracking *row in an existing table* = no migration. A dedicated **new table `procontract_pending_enrichment`** (the lean option, cleanest for the landmine) **is** a schema change → ships as `0015_*.sql`, `user_version 10 → 11`, desktop `EXPECTED_SCHEMA_EPOCH 10 → 11` in the same commit (§6 R6, §7e). The state-string-on-existing-column alternative needs no epoch bump but reopens the re-registration-guard landmine above. Q4 picks between them; both are tested (§7e).
- Have enrich **sweep the daemon-tracked awaiting-access set** (not just "Recently added"), locating each tracked item on the portal by its stored ProContract identifier (`source_notice_id`/advertId), so resumption is independent of the "Recently added" ordering and `maxPages`. (We hold the advert/activity identifier from interest time; the RFx URL/`rfxId` does not exist until access is granted, so the sweep keys off the identifier, not a stored RFx URL.)
- **Clear** the record (awaiting-access → enriched) when enrich finds the live tender event and downloads documents; **keep** it (bumping an `observation_count`) while the portal still shows no live event; add an optional **staleness expiry** so the set doesn't grow unbounded. Note the expiry **cannot** key off the RFx *response* deadline — that is only learned from the live tender event, which an awaiting-access item by definition hasn't reached — so use the **notice-level `deadline_at`** captured at discover (which may itself be null), or a **max-age since interest** as the robust fallback (see Q5).
- Belt-and-suspenders: enrich should also **backfill** awaiting-access for any portal item it observes as `not_yet_responded` that isn't already tracked (e.g. interest registered before this feature, or directly on the portal).
- **Lifecycle (every transition's trigger + the exact row written) — for the spec:**

  | Transition | Trigger | State / row written |
  |---|---|---|
  | **create** | register-interest succeeds | insert `procontract_pending_enrichment(process_id, notice_id, advert_id, state='awaiting_access', interest_registered_at, deadline_at?, observation_count=0)`. **Do NOT touch `interest_registration.state`** — it stays `'completed'` so the re-fire guard still excludes it (§2.4 landmine). Note `observation_count` is the *enrich-sweep* counter and is deliberately **separate** from the job-retry `attempt_count` (§4.2). |
  | **keep** | enrich pass runs, portal still shows no live tender event | same row, `state='awaiting_access'`, bump `observation_count`/`last_checked_at` only |
  | **clear** | enrich finds the live tender event ("Not started (Respond by …)") and downloads docs | set `state='enriched'` (or delete the row — spec's call; prefer terminal state for auditability), record `enriched_at` |
  | **backfill** | enrich observes a portal item as `not_yet_responded` with no tracking row | insert a row as in **create**, `interest_registered_at` = best-known/observed |
  | **expire** | staleness sweep: `now > deadline_at` (notice-level, captured at discover, may be null) **OR** `now - interest_registered_at > max_age` (robust fallback) | set `state='expired'`, stop sweeping it. **Must NOT key off the RFx response deadline** — unknown for awaiting-access items (Q5). |

- **Skill contract (`procontract-enrich-tender`) — input shape + merge.** The skill gains an optional input array, e.g. `awaitingAccessTargets: [{ advertId, sourceNoticeId, processId }]`, in addition to its existing "Recently added" scan. **Merge behavior:** the skill (a) runs its normal Recently-added scan, and (b) for each work-list target *not already located by the scan*, navigates directly to that advert by `advertId` to check for a live tender event — then **unions** both result sets, deduping by `advertId`. This makes resumption independent of Recently-added ordering and `maxPages`. The work-list is derived by the daemon from `procontract_pending_enrichment WHERE state='awaiting_access'` and injected at execution time (§4.3 enrich row). When the work-list is empty the skill behaves exactly as today (pure Recently-added scan) — backward compatible.
- **Sequencing:** this hardening only pays off once enrich actually runs on schedule — i.e. it depends on G1/G2 (autonomous scheduling + worker). Until then the awaiting-access set accumulates but is only drained by a manual `--ingest procontract-enrich`. Strategy should land G1/G2 before, or together with, G4.

### 4.5 Autonomous trigger for the commercial stages (`landed-cost` → `net-gate` → `manifest`)
Needed so "run the whole thing" (§2.5) actually holds, not just ingestion. These stages are `stage_run_id` passes, not source scrapers and not `sync_source`; `net-gate` is a pure DB sweep, while `landed-cost` and `manifest` may invoke their existing skill runners for eligible rows. They still do not require the ProContract/FTS browser session used by source scraping, so the hard axis is scheduling/order and worker head-of-line behavior, not procurement-source browser contention.

**Existing precedent:** `run_deterministic_pipeline()` (`main.rs:23323`) already proves the daemon can run a sequential in-process pipeline chain under `&mut runtime.state_db` without stopping `--serve`, but it stops at qualification/gross-margin/relate. The commercial autonomy decision is therefore between (1) **worker-queued commercial jobs** on the coverage/deterministic lane, and (2) a broader in-process `advance-pipeline` sweep that unifies qualification and commercial coverage stages behind the same pattern as `run_deterministic_pipeline`. Strongest option for this feature: keep PTY/source scrapers on the worker/arbiter, and add a single coverage-driven `commercial_sweep` job on the coverage/deterministic lane now; leave full `advance-pipeline` unification to the spec only if it can reuse `run_deterministic_pipeline` without dragging source-scraper concerns into it.

**Resolved model (the execution-context question §2.5 raised, now decided): scheduled cron kind + coverage-driven body, executed in-process by the worker's coverage/deterministic lane — NOT a daemon-stop rerun.** Concretely:
- **Trigger = a new cron slot + new job kind.** Add one schedule slot *after* the ingestion window (~07:00 UTC) that enqueues a new kind `commercial_sweep` (name TBD), via the exact §4.1 tick + §4.2 worker machinery — no bespoke completion-hook plumbing, no new trigger surface (respects N4).
- **Body = coverage-driven, in-process.** The `commercial_sweep` handler runs the **existing** `landed-cost` → `run_net_margin_gate_once` → manifest-generation sequence over eligible rows. These are coverage operations (they advance every eligible process/candidate lacking a current downstream decision, not a single `process_id`). So the "scheduled" trigger and the "coverage-driven" body are the *same* design, not competing options: the schedule decides *when*, the coverage query decides *what*. This collapses the three §2.5 candidates into one.
- **Execution context = worker coverage/deterministic lane (§4.2), same-process.** The handler invokes the stage passes directly on the serve daemon under the arbiter — it does **NOT** call `trigger_pipeline_rerun`, which is a **desktop command** (`apps/desktop/src-tauri/src/main.rs:2005`) that first signals the detached `--serve` daemon to stop by PID and waits for it to release the runtime lock (`stop_daemon_for_exclusive_pipeline_rerun`, `:1215`), then shells `daemon --ingest landed-cost`, `--ingest net-gate`, `--ingest manifest` as one-shot subprocesses (`PIPELINE_RERUN_INGEST_STAGES`, `:78`), then relaunches the detached daemon and waits for handshake (`ensure_daemon_ready_after_pipeline_rerun`). That exclusivity comes from the desktop killing the daemon, not from the daemon yielding; an autonomous in-process trigger cannot reuse it. The coverage/deterministic lane prevents a long PTY/source scrape from head-of-line blocking the commercial sweep, while the arbiter still controls any skill sessions the sweep launches.
- **Dedup key (defined, not vague).** Two layers: (1) the **per-slot cron idempotency key** `cron:commercial-sweep:{skill}:{scheduled_for}` makes repeated ticks within one slot enqueue at most one sweep job (same `find_existing_enqueue_ack` path as all cron kinds); (2) the sweep **body is idempotent by coverage** — it only advances processes that *lack* a current decision, so even if two sweeps ran in a window the second is a near-no-op. Together these mean "several ingestion runs in one window" cannot produce redundant commercial work. *Residual (Q8):* whether to *also* add a lightweight chained nudge (enqueue a `commercial_sweep` immediately after a successful ingestion run, deduped by the same key) as an optimization so a new process doesn't wait until 07:00 — defer; the scheduled+coverage model is correct without it.

**Why not "chained-only":** a pure completion-hook is fragile (a missed/partial ingestion run leaves processes stranded with no later pass) and needs new plumbing. Scheduled+coverage is self-healing: the next 07:00 sweep picks up anything earlier runs missed.

---

## 5. Job-kind routing (target state)

| `kind` | Source today | Target executor |
|---|---|---|
| `sync_source` | cron + `--ingest` live-loops | Refactor live-loop bodies into a shared source body, route by `source_id` (§4.3) |
| `source_health` | cron (FTS canary every 30 min; L5 marketplace canary daily) | **New handler** invoking the `source-health/*` canary skill — skill exists, no daemon job-handler today |
| `auth_refresh` | cron (bindings at `registry.rs:796,827`) | **New handler** invoking the `auth-signin/*` skill — skill exists, no daemon job-handler today |
| `validation_cross_model_audit` | cron | **New handler** — *no executor exists today* (kind appears only in `cron.rs`); distinct from the existing `semantic_validate` per-result sub-job path |
| `validation_calibration_readiness_sweep` | cron | **New handler** — *no executor exists today* (same as above) |
| `commercial_sweep` *(new)* | **new** cron slot (~07:00, §4.5) | **New handler** on the coverage/deterministic lane — runs existing `landed-cost` → `run_net_margin_gate_once` → manifest-generation coverage passes in-process (no daemon stop) |
| `extract_document_structure` | enqueued by enrich | already handled by `run_next_workflow_job` |
| `extract_document_semantics` | enqueued by extract | already handled by `run_next_workflow_job` |
| `run_bid_workflow` | manual/IPC | already handled (keep) |
| `enrich_process` | desktop `trigger_enrichment` only (**not** cron) | **Orphaned today** (no handler). *Optional:* route to a process-enrichment handler so the "Rerun enrichment" button works — outside the cron-core scope |

---

## 6. Concurrency & correctness model (primary risk area)

- **Threads touching `state.db`:** serve/IPC (owned handle), lease heartbeat (own conn), **new** scheduler tick (own conn), **new** worker (own conn or shared services). WAL + `busy_timeout=5000` already configured; still single-writer at any instant.
- **Risks:**
  - R1. Write contention / `SQLITE_BUSY` if worker does heavy writes concurrently with IPC mutations. Mitigation: short write transactions; rely on busy_timeout; consider serializing writes through a single writer actor if contention shows in E2E.
  - R2. A long PTY job starving the scheduler tick. Mitigation: separate threads; tick never blocks on the worker.
  - R3. Double-dispatch of a schedule across startup-enqueue + tick. Mitigation: existing idempotency key.
  - R4. Resource-arbiter accounting must be honored by the autonomous worker exactly as the manual path does (browser/agent slots, source lanes) to avoid launching more browser sessions than slots.
  - R5. Sleep/wake: in-flight PTY tainting on host sleep (`pty/health.rs` `detect_host_sleep`, 5s threshold) must interact cleanly with the worker's claim/lease (300s lease, 60s heartbeat).
  - R6. Schema-epoch coupling (memory): desktop `EXPECTED_SCHEMA_EPOCH` must always equal daemon `DEFAULT_SCHEMA_EPOCH`. A new **table or column** for G4 is a schema change → bump the epoch and move *both* sides together; a new **state-string value** in an existing column (e.g. `awaiting_access`) is *not* a schema change and needs no epoch bump (but still see the §4.4 re-registration-guard landmine). **Concrete:** highest existing migration is `0014_installation_labour_fields.sql` and `DEFAULT_SCHEMA_EPOCH = 10`; so a G4 `procontract_pending_enrichment` table ships as `0015_*.sql` with `user_version 10 → 11` and desktop `EXPECTED_SCHEMA_EPOCH 10 → 11` in the *same* change (§7e migration test).
  - **R7 (hostile-reviewer #1 — two concurrent same-source `sync_source` runs across a tick boundary).** A slot can fire while the previous run for the same source is still in-flight (long overnight scrape + next slot, or a manual `--ingest` overlapping a cron slot). Two concurrent same-source scrapes against one authenticated CDP browser would corrupt cursors/sessions. **Invariant the spec must hold:** `procurement_source_lanes = 1` (arbiter default) **serializes same-source runs** — the second `sync_source` for that source is denied at admission (`DenialReason::SourceLaneExhausted { source_id, limit: 1, in_use: 1 }`) and **defers** (§4.2 backoff), it does not run concurrently. This must be asserted by the §7 concurrency soak, not assumed. (`supplier_source_lanes=2` permits 2 concurrent supplier scrapes — fine, those are distinct browsers/sources.)
  - **R8 (hostile-reviewer #2 — worker lanes + WAL single-writer under IPC mutation pressure).** Two worker lanes + IPC handlers + lease heartbeat are up to 4 writers to one `state.db`; WAL allows only one writer at an instant. Mitigation as R1 (short write transactions, `busy_timeout=5000`) plus: the coverage/deterministic lane's commercial sweep does the *heaviest* writes — keep its transactions per-process-scoped, not one giant transaction, so it yields the writer lock between processes and IPC mutations aren't starved past `busy_timeout`. Falsifiable via the §7 soak (no permanent `SQLITE_BUSY`, no lost writes). If contention surfaces, escalate to a single-writer actor (R1).
- **Foundations that help:** WAL already on; lease-heartbeat precedent for background-thread DB access; `ResourceArbiter` already `Mutex`-guarded; idempotency keys on cron enqueue; claim ordered by `COALESCE(scheduled_for, created_at)` (overdue-first); `procurement_source_lanes=1` already serializes per-source admission.

---

## 7. Testing strategy (project rule: real E2E, no mocks/stubs)

**Gate (from `AGENTS.md` §"Stage Validation Test Gate", non-negotiable).** Every claim of green must run the *full* loop, because `cargo test -p daemon --bin daemon` **silently skips all integration files** (this caused two false-GREEN manifests):
```bash
cargo fmt --check
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test -p daemon --bin daemon                      # binary unit tests
for t in crates/daemon/tests/*.rs; do                  # EACH integration file is its own target
  cargo test --test "$(basename "$t" .rs)" || exit 1
done
```
The only permitted "fake" is the **deterministic fake-claude CLI** already used by `agent_procurement_ingestion.rs` — it stands in for the *agent/PTY*, never for the daemon under test. Everything else is real: real SQLite (tempdir), real cron parsing, real `daemon` binary subprocess where the flow demands it. Env-dependent tests **auto-skip via runtime probes, never `#[ignore]`**.

### 7a. Existing-test impact (what changes because the worker routes more kinds / the live-loop body moves)
| File | Change | Expected |
|---|---|---|
| `cron_scheduler.rs` | The `commercial_sweep` slot (§4.5) is added to the registry/schedule set | Test 001's schedule-anchor assertions must add the new ~07:00 slot; test 006's catch-up priority-order list gains the new key. **If you add the slot you MUST update these or they break.** |
| `job_claim.rs` | New kinds are now claimable (`sync_source`, `source_health`, `auth_refresh`, `validation_*`, `commercial_sweep`) | Existing claim-ordering tests unaffected (they use `kind='stage-job'`); add new tests in 7b, don't mutate the ordering tests. |
| `agent_procurement_ingestion.rs` | `--ingest fts` body now goes through the refactored shared body (§4.3) | Test 001 (`kind='sync_source'`, `status='completed'`) must **still pass unchanged** — this is the live-loop-refactor regression guard. |
| `agent_contracts_finder_ingestion.rs` | same refactor | must still pass unchanged. |
| `resource_arbiter.rs` | no source change; reused as the model for the new defer-on-denial + source-lane tests | unaffected. |
| `sqlite_schema.rs` | **only if Q4 picks a table** → `user_version` moves 10→11 | the existing `user_version == DEFAULT_SCHEMA_EPOCH` assertion auto-tracks the constant; add the new-table assertions (7e). |

### 7b. New unit/integration tests (per subsystem; exemplar to copy named)

**Scheduler tick** — *new file `scheduler_tick.rs`, copy `cron_scheduler.rs` idioms (`ts()`, `TestStateDb::new()`, `seed_source`, `CronScheduler::from_registry`).*
- `tick_001_due_slot_enqueues_exactly_one_row`: seed FTS; build scheduler; call the tick body with `now = ts(2026,6,5,4,15,0)`; assert exactly one `workflow_jobs` row `WHERE kind='sync_source'` for that slot.
- `tick_002_repeated_ticks_same_slot_idempotent`: call the tick body 3× with `now` inside the same slot window; assert `SELECT COUNT(*) FROM workflow_jobs` is still 1 (dedup via `find_existing_enqueue_ack`).
- `tick_003_long_gap_catchup`: first tick at `ts(…04:15)`, then advance `now` past several days (`ts(…+3d 04:20)`); assert the next tick enqueues the overdue slot(s) per `schedule.after(last_run_at)` — model on `cron_scheduler.rs` test 006.
- `tick_004_clock_seam`: prove the tick body is `fn(…, now)` and contains no inline `Utc::now()` — a due slot at an injected `now` enqueues; the same call at a `now` before the slot enqueues nothing. (This *is* the §4.1 design-requirement test.)
- `tick_005_registry_reload_on_generation_bump`: seed source disabled → tick enqueues nothing; bump the registry generation (enable via the same path IPC uses) → next tick reloads and enqueues. (Pins §4.1 reload mechanism.)

**Worker pump** — *new file `worker_pump.rs`, copy `job_claim.rs` (`seed_job`, `JobRunner::claim_next_job`) + `resource_arbiter.rs` (`admit`/`release`, `DenialReason`).*
- `worker_001_routes_each_cron_kind`: for each `kind ∈ {sync_source, source_health, auth_refresh, validation_cross_model_audit, validation_calibration_readiness_sweep, commercial_sweep}` seed a pending job, run one worker iteration, assert it ends `status='completed'` (or a legitimate run-failure) and **never** `fail_job` "unsupported queued job kind". (Routing-coverage guard against today's `other => fail_job` at `main.rs:1722`.) **Anti-hollow-handler requirement:** `status='completed'` alone is insufficient — an empty handler that flips status would pass falsely. For each kind the test must also assert the handler's **real side-effect row**, produced through the real handler + fake-claude where a skill runner is involved or the real DB pass where it is not: `sync_source`→a source-produced row (as M1); `source_health`→a `source_health`/canary result row; `auth_refresh`→the auth-state/refresh row the `auth-signin` skill writes; `validation_*`→the validation verdict/summary row; `commercial_sweep`→landed-cost estimate/net-margin/manifest rows as applicable. Skill-backed portions run via the fake-claude harness (real PTY/agent path), not a stubbed handler.
- `worker_002_defer_on_denial_no_attempt_bump`: saturate `browser_slots=2` (admit two), enqueue a third `sync_source`; run a worker iteration; assert the third returns to `status='pending'`, `attempt_count` **unchanged**, `lease_owner IS NULL`, and a denial backoff defers re-claim (assert it is not re-claimed on the immediately-next iteration).
- `worker_003_source_lane_serializes_same_source` (R7): admit one `sync_source` for `fts` (consumes `procurement_source_lanes=1`); enqueue a second `fts` `sync_source`; assert `DenialReason::SourceLaneExhausted { source_id:"fts", limit:1, in_use:1 }` → defer, not concurrent run.
- `worker_004_head_of_line` (the §4.2 falsifiable claim): enqueue a slow PTY `sync_source` (fake-claude sleeps) on the PTY lane and a `commercial_sweep` on the coverage/deterministic lane; assert the `commercial_sweep` reaches `completed` **before** the `sync_source` does. If unpassable with lane separation, that falsifies the lane model (fall back to kind-aware pool).
- `worker_005_arbiter_quota_respected`: assert never more than `browser_slots=2` concurrent PTY admissions (model `resource_arbiter.rs` test 006). *Assumes the two-lane model (§4.2); with a hypothetical single serial worker there'd be no concurrency to cap — state the assumption in the test header.*

**Live-loop refactor regression** — *extend `agent_procurement_ingestion.rs` `RunHarness` pattern.*
- `--ingest fts`, `--ingest procontract-enrich`, `--run-next-job` still execute and persist (these run today; they must keep running after the body moves behind the shared execution function).

### 7c. E2E manifests (real binary subprocess; build on `RunHarness` + fake-claude)

> **Harness-shape requirement (read before M1/M2).** The one-shot `RunHarness.run()` calls `Command::…output()`, which **blocks until the child exits** — a live `--serve` daemon never exits, so M1/M2 **cannot** use `RunHarness.run()` as-is against `--serve` (the test would hang). Two supported shapes:
> - **(preferred — deterministic) one-shot autonomous entrypoint.** Make the §4.1 clock seam *also* a test entrypoint: a flag like `--run-scheduler-once --tick-now <rfc3339>` that, in-process, evaluates schedules at the injected instant, enqueues due jobs, **drains the worker lane(s) to quiescence, then exits 0**. This fits `RunHarness.run()` exactly (one-shot, `.output()` returns) and removes wall-clock race flakiness. It is a small, owned design requirement — the same seam that makes G1 testable. *This is the shape M1/M2 below assume.*
> - **(higher-fidelity, flakier) spawn-poll-terminate serve harness.** A separate harness that `spawn()`s a real `--serve` daemon (not `.output()`), polls the SQLite for the expected row with a timeout, then SIGTERM/kills it. Use only if exercising the live tick-interval loop is specifically required; prefer the one-shot entrypoint otherwise.

**M1 — End-to-end autonomy (G1+G2).**
- *Preconditions:* `RunHarness` tempdir; FTS source seeded enabled; fake-claude wired (`--version`/`probe`/result-path env vars).
- *Steps:* invoke the one-shot autonomous entrypoint `--run-scheduler-once --tick-now 2026-06-05T04:15:00Z` (per the harness note above); do **nothing** else (no `--ingest`, no IPC trigger). The entrypoint evaluates the schedule at 04:15, enqueues the due FTS slot, drains the worker, and exits.
- *Expected:* a full FTS run executes and persists with **no manual trigger** — driven only by the injected clock.
- *Verify:* open the real SQLite; `SELECT kind,status FROM workflow_jobs WHERE kind='sync_source' AND parent_job_id IS NULL ORDER BY created_at DESC LIMIT 1` → `('sync_source','completed')` (same assertion idiom as `agent_procurement_ingestion.rs` test 001), and the FTS-produced rows (e.g. `procurement_processes`/`raw_documents`) exist. *(Negative control: a run at `--tick-now 2026-06-05T03:00:00Z` — before the slot — must enqueue and persist nothing.)*

**M2 — Commercial-stage autonomy (G6).**
- *Preconditions:* M1's autonomous run has produced a process needing pricing (or seed one via `seed_process` at `current_stage='qualification'` with the landed-cost inputs present).
- *Steps:* invoke the same one-shot autonomous entrypoint at `--tick-now 2026-06-05T07:00:00Z` (the harness note above) so the `commercial_sweep` slot fires and drains; no operator action, no `trigger_pipeline_rerun`.
- *Expected:* `landed-cost`→`net-gate`→`manifest` advance to a **current** `manifest`/decision, executed **in-process through the same daemon runtime path** as §4.5 (not through the desktop exclusive-rerun path).
- *Verify:* (1) decision/manifest rows current for the process; (2) **no daemon stop/restart occurred** — in the live-`--serve` harness assert the daemon PID/process is the same throughout; in the one-shot harness assert no stop/restart path or `trigger_pipeline_rerun` call was invoked. The exclusive-restart path must NOT be the mechanism. A regression here looks like the manifest only appearing after an operator `trigger_pipeline_rerun`, or the daemon process restarting.

**M3 — ProContract resumption full lifecycle (G4) — NET-NEW SUITE.**
> **There is no existing ProContract integration test file** — register-interest/enrich are referenced only inside `cron_scheduler.rs` schedule assertions. This suite (`procontract_resumption.rs`) is **built from scratch** on the `RunHarness` + fake-claude pattern; it is materially more work than "extend an existing test," and strategy/beads must size it as net-new.
- *Preconditions:* `RunHarness`; ProContract source seeded; fake-claude returns canned interest/enrich skill results via `DAEMON_TEST_CLAUDE_RESULT_PATH`.
- *Steps & expected (one ordered scenario):*
  1. Run `--ingest procontract-register-interest` with a canned success result. **Assert:** a `procontract_pending_enrichment` row exists `state='awaiting_access'` **AND** the `interest_registration` stage row is still `state='completed'` (the §4.4 landmine — proves the re-fire guard still excludes it, so a re-run won't re-express interest).
  2. Run `--ingest procontract-enrich` with a canned result showing **no live tender event** for that advert, plus an *untracked* portal item seen as `not_yet_responded`. **Assert:** the tracked row is **kept** (`state='awaiting_access'`, `observation_count` bumped) **AND** the untracked item is **backfilled** (new row inserted).
  3. Run `--ingest procontract-enrich` again with a canned result where the live tender event **now exists** for the tracked advert. **Assert:** enrich located it via the **daemon work-list** (inject the work-list and make the fake-claude result prove the advert was visited by `advertId`, not found via "Recently added"/`maxPages`), downloaded docs, and the row is **cleared** (`state='enriched'`).
  4. Staleness: seed a row with `interest_registered_at` older than `max_age` (and/or `deadline_at` in the past); run the sweep. **Assert:** row → `state='expired'`; and **assert the expiry did NOT key off an RFx response deadline** (none is stored for these items — Q5).
- *Verify:* `SELECT state, observation_count FROM procontract_pending_enrichment WHERE …` at each step.

**M4 — Manual-path regression vs new capability.**
- *Regression:* `--ingest fts`, `--run-next-job`, and desktop `trigger_pipeline_rerun` still work.
- *New capability:* the desktop `trigger_ingestion` button (`sync_source` via IPC) now **executes** via the worker (today it enqueues an orphan that nothing consumes — §2.2). *Steps:* enqueue via the IPC handler path (model `ipc_trigger_bid_workflow.rs`), then **drain with the one-shot `--run-next-job`** (the existing one-shot drainer — *not* a live `--serve` poll, which would hang per the §7c harness note); `--run-next-job` must share the same extended kind-routing as the always-on worker (§4.2), so this proves the button's job is now routable. *Verify:* the `sync_source` job reaches `completed`, not stuck `pending`. If `enrich_process` routing is folded in (Q/§5/G3), assert `trigger_enrichment` executes the same way.

**M5 — Concurrency soak.**
- *Steps:* run the worker lane(s) + simulated IPC mutations + lease heartbeat in parallel under WAL (model the `job_claim.rs` `Arc<Barrier>` multi-thread pattern, each thread opening its own `StateDb::open`).
- *Expected/Verify:* **no lost writes — proven concretely, not asserted hand-wavily:** N threads each perform M increments of a known counter row (or insert M uniquely-keyed rows); after the barrier, assert the final value `== N*M` (or `COUNT(*) == N*M`) — any lost update fails the count. And **no permanent `SQLITE_BUSY`** (transient + `busy_timeout` retry is fine; assert the run completes without a surfaced `SQLITE_BUSY` error after retry). **Plus the R7 case:** two slots for the *same* source firing across a tick boundary while the first run is in-flight → assert `procurement_source_lanes=1` admission denies the second (no two concurrent same-source runs), it defers and runs after.

### 7d. Verification criteria (per goal → exact proof → regression shape → gate)
| Goal | "Correct" proof (query/command) | Regression looks like |
|---|---|---|
| **G1** scheduler fires autonomously | M1 + `scheduler_tick.rs` tick_001/003/004 green | due slot never enqueues without restart; no tick thread |
| **G2** cron kinds execute via worker | `worker_pump.rs` worker_001 all kinds `completed` | any cron kind hits `other => fail_job` "unsupported" |
| **G3** manual paths intact + buttons live | M4: `--ingest`/`--run-next-job`/`trigger_pipeline_rerun` pass; `trigger_ingestion` `sync_source` reaches `completed` | button job stuck `pending` (orphan) |
| **G4** awaiting-access tracked + swept | M3 steps 1–4 green; `procontract_resumption.rs` | enrich misses an aged-out item; or interest re-expressed (landmine) |
| **G5** concurrency-correct under WAL | M5 + worker_003 green | lost write / permanent `SQLITE_BUSY` / two same-source runs |
| **G6** commercial stages autonomous | M2: current manifest, no daemon stop | manifest only after operator rerun; daemon restarted |

Every row above is proven only under the **full per-file integration loop** (§7 gate top) — `--bin daemon` alone would skip `scheduler_tick.rs`/`worker_pump.rs`/`procontract_resumption.rs` and falsely report green. Stage-validation manifests for this feature MUST run the `for t in …` loop.

### 7e. Schema-migration test (conditional on Q4)
- **If Q4 picks the dedicated `procontract_pending_enrichment` table** → add a `sqlite_schema.rs`-style test (copy its `user_version`/`schema_migrations`/`table_exists` idioms): assert `PRAGMA user_version == 11`, assert `schema_migrations` contains `'0015_procontract_pending_enrichment'` (next after the current highest `0014_installation_labour_fields`), assert the table + its columns (`process_id, notice_id, advert_id, state, interest_registered_at, deadline_at, observation_count, last_checked_at, enriched_at`) exist via `logical_schema_for_tables`. **AND** a non-test note enforced by the change: desktop `EXPECTED_SCHEMA_EPOCH` must move **10 → 11** in the same commit (schema-epoch coupling, R6) — otherwise desktop reads fail wholesale.
- **If Q4 picks the state-string option instead** → no migration test (no epoch bump); instead add a test that the **widened re-fire guard still excludes `awaiting_access`** (register-interest candidate selection does not re-pick an item already marked awaiting-access), guarding the §4.4 landmine.

---

## 8. Open questions (for spec)

*Status tags added by the audit pass: **OPEN** = spec must decide; **RESOLVED (this doc)** = decided here, listed for traceability; **NARROWED** = principal decision made, only a residual remains.*

1. **OPEN.** Worker execution context: own `StateDb` connection per worker/lane vs a shared services handle? (Drives the §4.3 refactor shape; the lease-heartbeat precedent favors own-connection-per-lane.)
2. **RESOLVED (this doc) → fold.** Tick subsumes startup-enqueue (§4.1); the shared idempotency key makes the bootstrap-vs-first-tick race a dedup no-op. Retaining `enqueue_startup_cron_jobs` is a simplicity regression, not a correctness risk.
3. **RESOLVED (this doc) → two lanes by job class**, not single-serial and not homogeneous pool (§4.2): a PTY/browser lane (sized to `browser_slots=2`) + a coverage/deterministic lane (commercial-sweep + `validation_*`). Falsifiable via the head-of-line test (worker_004); fall back to a kind-aware pool only if that test can't pass.
4. **OPEN.** Awaiting-access modeling: dedicated `procontract_pending_enrichment` table (lean — keeps `interest_registration.state='completed'`, avoids the landmine; ships as `0015_*`, epoch 10→11) vs an extra `process_stage_status` state value (no epoch bump but **must** widen the re-registration guard — §4.4 landmine). Pick one; both have a §7e test.
5. **OPEN.** Staleness/expiry policy for awaiting-access: notice-level `deadline_at` (may be null) and/or `max_age` fallback; operator-visible? **Hard constraint:** must NOT key off the RFx response deadline (unknown for these items).
6. **OPEN.** Should `source_health`/`auth_refresh` autonomous execution be gated behind a flag initially (shadow) before full enforcement, mirroring the validator rollout philosophy?
7. **OPEN.** Desktop UX: surface "scheduler running / last tick / next run / awaiting-access count" in the control panel? (Read-only snapshot already exists to extend.)
8. **NARROWED.** Commercial-stage trigger model is **resolved to scheduled cron kind + coverage-driven body, worker-executed in-process** (§4.5); `trigger_pipeline_rerun`'s PID-stop/one-shot-subprocess/relaunch path stays a manual-only desktop affordance and is **not** the autonomous mechanism. *Residual:* whether to *additionally* enqueue a deduped `commercial_sweep` immediately after a successful ingestion run (a chained nudge so new processes don't wait for 07:00), or fold the commercial chain into a broader `advance-pipeline` sweep that reuses the `run_deterministic_pipeline` precedent — optional optimization, defer unless the spec can keep the source-scraper boundary clean.
9. **OPEN (new, from audit).** Registry-reload trigger for the tick: wire a registry **generation counter** bumped by every IPC source-mutation handler (lean), vs poll `MAX(sources.updated_at)` each tick. Either avoids per-tick reloads; pick one and ensure newly enabled sources re-seed `last_run_at` so they don't fire a spurious catch-up burst (§4.1).

---

## 9. Scope summary

**In scope:** scheduler tick (with the clock-injection seam as a design requirement, not just a test hook — §4.1); two-lane continuous worker (§4.2); live-loop refactor to be queue-driven (`sync_source`, with per-source param derivation — §4.3); **new handlers for the `source_health`, `auth_refresh`, and `validation_*` cron kinds** (no executor today); **an autonomous trigger for the commercial stages** `landed-cost`→`net-gate`→`manifest` via the resolved **scheduled `commercial_sweep` cron kind + coverage body** (§4.5) — which includes adding *one new* ~07:00 commercial-sweep schedule slot and reconciling (not reusing) `trigger_pipeline_rerun`'s daemon-stop; ProContract awaiting-access persistence (table or state-string, Q4) + enrich work-list sweep + skill input change; defer-on-denial backoff; tests (net-new `scheduler_tick.rs`, `worker_pump.rs`, `procontract_resumption.rs`); *optionally* minimal desktop status surfacing (scope per Q7).

**Out of scope:** new sources/skills; API connectors; multi-daemon; remote trigger; **retuning the existing source schedule values** (FTS 04:15 etc. stay code-defined in `sources/registry.rs` + `scheduler/cron.rs`).

---

## 10. Key file references (verified)

- `crates/daemon/src/main.rs:3808` — `serve_handshake_service` (blocking IPC loop, no tick)
- `crates/daemon/src/main.rs:2924`, `:3686` — `enqueue_startup_cron_jobs` (only dispatch caller)
- `crates/daemon/src/main.rs:1703-1728` — `run_next_workflow_job` (3-kind match; `other => fail_job`)
- `crates/daemon/src/main.rs:1379` — `--ingest` source dispatch
- `crates/daemon/src/main.rs:10532`, `:12403`, `:15043` — `run_landed_cost_live_loop` / `run_net_margin_gate_live_loop` (→ `run_net_margin_gate_once`) / `run_manifest_generation_live_loop` (commercial stages; `stage_run_id`, **not** `sync_source`, **not** cron-bound)
- `crates/daemon/src/main.rs:1423`, `:23323` — `--pipeline <source_id>` dispatch to `run_deterministic_pipeline` (existing in-process sequential stage runner through gross-margin/relate; no daemon stop)
- `crates/daemon/src/main.rs:19627` — `run_procontract_enrich_tender_live_loop` (mints+claims own job; cursor)
- `crates/daemon/src/main.rs:15371`, `:15600`, `:15885` — ProContract interest candidate selection / stage-state persistence
- `crates/daemon/src/ipc/handlers/jobs.rs:222`+ — `trigger_ingestion` (enqueues `sync_source`) / `trigger_enrichment` (enqueues `enrich_process`); both enqueue-only, no consumer today
- `apps/desktop/src-tauri/src/main.rs:1755`, `:2035` — desktop `trigger_ingestion` command (enqueue-only) vs pipeline-rerun's `run_daemon_ingest_stage` (`--ingest`, actually executes)
- `crates/daemon/src/scheduler/cron.rs:120`, `:341` — `dispatch_due_jobs`, `build_enqueue_request` (cron kinds + idempotency key)
- `crates/daemon/src/jobs/runner.rs:585` — `enqueue_job` dedups via `find_existing_enqueue_ack` (read-before-write on `idempotency_key`; returns the existing ack rather than inserting a duplicate) — this is what enforces the "prevents dupes" claim
- `crates/daemon/src/sources/registry.rs:580+` — per-source cron bindings (FTS 04:15, etc.)
- `crates/daemon/src/sources/registry.rs:796`, `:827` — `auth_refresh_binding: Some(...)` (auth_refresh kind is actually configured)
- `crates/daemon/src/main.rs:2978` — `SyncLeaseHeartbeat::spawn` (background-thread + own-connection precedent)
- `crates/daemon/src/db/sqlite.rs:215-218` — WAL / busy_timeout PRAGMAs
- `crates/daemon/src/agents/arbiter.rs:13-20` — `ResourceBudgets::default` (browser_slots=2, agent_slots=4, procurement_source_lanes=1, supplier_source_lanes=2)
- `crates/daemon/src/jobs/claim.rs:15-16` — lease 300s / heartbeat 60s; `:174-176` claim ordering
- `crates/daemon/src/main.rs:25527` — `default_host_sleep_threshold_ms` = 5000ms
- `apps/desktop/src-tauri/src/main.rs:78`, `:1215`, `:2005`, `:2023` — `PIPELINE_RERUN_INGEST_STAGES`; `stop_daemon_for_exclusive_pipeline_rerun`; `trigger_pipeline_rerun`; `trigger_pipeline_rerun_impl` (desktop PID-stop → one-shot `--ingest` subprocesses → daemon relaunch/handshake)
- `skills/procurement-ingestion/procontract-enrich-tender/SKILL.md` — Activities scan, `not_yet_responded`, `maxPages: 5`, CDP `:9222` discipline
- `crates/daemon/tests/cron_scheduler.rs` — `ts()` fixed-instant helper; `CronScheduler::from_registry` → `refresh_next_run_at` → `dispatch_due_jobs`; schedule anchors (FTS 04:15, canary, contracts-finder, discover 05:00, interest 05:30, enrich 06:00)
- `crates/daemon/tests/job_claim.rs` / `resource_arbiter.rs` — `seed_job`/`claim_next_job`; `admit`/`release`, `DenialReason::{BrowserSlotsExhausted, SourceLaneExhausted, SkillConcurrencyCapReached}`
- `crates/daemon/tests/agent_procurement_ingestion.rs` — `RunHarness` + fake-claude (`DAEMON_TEST_CLAUDE_RESULT_PATH`/`_CHECKPOINT_PATH`); post-run SQLite assertion idiom
- `crates/daemon/tests/support/db.rs` — `TestStateDb::new()`/`L4TestStateDb::new()`, `seed_process`/`seed_raw_document`, `logical_schema_for_tables`; `DEFAULT_SCHEMA_EPOCH = 10`
- `crates/daemon/tests/sqlite_schema.rs` — `PRAGMA user_version == DEFAULT_SCHEMA_EPOCH`, `schema_migrations` row check; **highest migration today = `0014_installation_labour_fields.sql`** → G4 table = `0015_*`
- **No ProContract integration test exists** — G4's `procontract_resumption.rs` is net-new (§7c M3)

---

## 11. Audit summary (this pass)

**Verdict:** implementation-ready for a spec, with the open questions in §8 the only remaining decisions. The audit pass resolved 3 previously-open design points inline (Q2 fold, Q3 two-lane worker, Q8 commercial-trigger model) and added concrete test manifests (§7) buildable from the named exemplars.

**Weakest remaining point (named explicitly):** the **worker execution-context choice (Q1)** — own `StateDb` connection per lane vs a shared services handle — is the highest-risk *unresolved* decision, because it shapes the §4.3 refactor surface and the §6 R8 WAL-contention behavior. The lease-heartbeat precedent points to own-connection-per-lane, but this should be the spec's first decision since everything in §4.2/§4.3 hangs off it. (The commercial-trigger execution context — previously the weakest point — is now resolved in §4.5.)

**Hostile-reviewer challenges and where they're answered:**
- *"Two concurrent same-source `sync_source` runs across a tick boundary."* → §6 R7: `procurement_source_lanes=1` denies the second at admission (`SourceLaneExhausted`) → defer; asserted by §7 worker_003 + M5.
- *"Worker lanes + WAL single-writer collapse under IPC mutation pressure."* → §6 R8: per-process-scoped sweep transactions + `busy_timeout`; falsified by the §7 M5 soak; escalation path = single-writer actor (R1).
- *"The startup-enqueue and first tick double-dispatch a slot."* → §4.1: identical idempotency key + `find_existing_enqueue_ack` read-before-write → no-op; asserted by tick_002.
- *"Awaiting-access modeling silently re-triggers register-interest."* → §4.4 landmine + lifecycle table: `interest_registration.state` stays `'completed'`; asserted by M3 step 1.
- *"`--bin daemon` says green."* → §7 gate: insufficient by `AGENTS.md`; the per-file integration loop is mandatory and the new suites (`scheduler_tick.rs`, `worker_pump.rs`, `procontract_resumption.rs`) are only exercised by it.

**Net-new work flagged for sizing:** the entire G4 ProContract test suite (`procontract_resumption.rs`) is built from scratch — no ProContract integration test exists today. Strategy/beads must size it accordingly, not as "extend an existing test."
