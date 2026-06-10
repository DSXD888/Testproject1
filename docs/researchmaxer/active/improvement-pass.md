# Improvement Pass — Autonomous Daemon Scheduling & ProContract Resumption

> **You are improving a research document, not implementing it.** One session: research → analyze → improve the target doc → fresh-eyes verify. You edit exactly ONE file. Everything else is read-only context.

**Target document (the ONLY file you may edit):**
`docs/research/2026-06-04-autonomous-daemon-scheduling-research.md` (215 lines, STRUCTURED, already codebase-verified)

---

## 1. Mission

This research doc is the input to a `pipemaxxer` run (→ spec → TDD → strategy → beads). It is already unusually strong: codebase-verified with specific `file:line` references, clear goals/non-goals, an honest gap analysis, and explicit open questions. Your job is **not** to rewrite it — it is to make it *implementation-airtight* by closing the specific gaps found during a fresh deep-research pass, correcting two stale facts, and strengthening the under-scoped surfaces so the spec author has zero ambiguity.

The feature itself: leave the Tauri desktop app (and the detached `--serve` daemon it spawns) open perpetually, and have the daemon (1) fire its cron schedules autonomously while running (no quit/reopen, no manual button clicks), (2) drain the resulting jobs end-to-end through an always-on worker, (3) advance the whole pipeline including the commercial stages, and (4) never silently drop a ProContract notice whose access hasn't been granted yet.

**Absorb the embedded findings below, then think independently.** The surface issues are already flagged in the doc and here. What earns this pass its keep is the *non-obvious* insight — the design tension the doc resolves too glibly, the failure mode nobody named, the simpler architecture hiding in the existing code.

---

## 2. Source-of-truth hierarchy

1. **The codebase is ground truth** for what exists today. Where the doc and the code disagree, the code wins and the doc must be corrected.
2. **The target doc** is what you improve.
3. **Project docs** (`AGENTS.md`, `docs/research/2026-03-08-public-procurement-intelligence-platform-research.md`, the strategy set) are READ-ONLY context. Do **not** edit them. Align terminology to them.
4. **Memory facts** (schema-epoch coupling; L6 archived) are background — verify against code before relying on them; one of them is now stale (see §4 finding A).

---

## 3. What to read (with reading notes)

**The target doc** — read it twice. First for what it says (§1 intent, §2 current-state, §3 goals, §4 architecture, §5 routing, §6 concurrency, §7 testing, §8 questions). Second for what it *implies but doesn't pin down* — especially §4.3 (the refactor), §4.5 (commercial-stage trigger), and §6 (concurrency).

**Codebase — the integration surfaces (all verified to exist at these locations):**

| Surface | Location | What it is |
|---|---|---|
| Blocking serve loop (no tick) | `crates/daemon/src/main.rs:3819` `serve_handshake_service` | `loop { listener.accept() … }`, pure IPC, no timer |
| Cron status snapshot (read-only) | `main.rs:3890` `build_cron_status_snapshot` | builds `CronScheduler` per handshake only to display next-run times; never enqueues |
| Bootstrap | `main.rs:2921` `bootstrap_runtime` → `:3697` `enqueue_startup_cron_jobs` | the ONLY caller of `dispatch_due_jobs` (single call site at `:3708`) |
| History seed | `main.rs:3736` `seed_cron_scheduler_from_history` | rebuilds `last_run_at` from `workflow_jobs.scheduled_for` |
| Cron core | `crates/daemon/src/scheduler/cron.rs:120` `dispatch_due_jobs(registry, now)`; `:341` `build_enqueue_request`; `:204` `record_enqueued` | `now` is **already a parameter** (clock seam exists); catch-up via `schedule.after(&anchor).next()` |
| Current worker | `main.rs:1703` `run_next_workflow_job` | matches only `extract_document_structure` / `extract_document_semantics` / `run_bid_workflow`; `other => fail_job` at `:1722` |
| `--run-next-job` | `main.rs:1424`/`1435` | claims ONE job, runs, exits |
| Background-thread precedent | `main.rs:682` `struct SyncLeaseHeartbeat` + `:2991` `spawn` | own `StateDb` connection via `open_state_db`, `recv_timeout(interval)` loop, `mpsc` stop channel, named thread |
| Runtime struct | `main.rs:657` `RuntimeBootstrap` | `state_db: StateDb` (owned, NOT Arc), `state_db_path: PathBuf` (cloned to threads), `resource_arbiter: Mutex<ResourceArbiter>` |
| Lease gates push only | `main.rs:3225` | lease-conflict path `return Ok(())` disables push dispatch; local execution unaffected |
| Job claim/runner | `crates/daemon/src/jobs/claim.rs:174` ordering `COALESCE(scheduled_for, created_at), created_at`; `:15-16` lease 300s/hb 60s; `:97` `claim_job_by_id`; `crates/daemon/src/jobs/runner.rs:580` `enqueue_job`; `:1626` `find_existing_enqueue_ack` (dedup on `idempotency_key`, matches kind+scope across statuses `pending/running/retry/completed`) |
| Live-loop bodies | `main.rs` `run_fts_live_loop` (:20096), `run_contracts_finder_live_loop` (:19171), `run_procontract_discover_live_loop` (:19382), `run_procontract_register_interest_live_loop` (:19481), `run_procontract_enrich_tender_live_loop` (:19627), marketplace (:19883), `create_source_sync_job_from_parameters` (:4511) | each derives per-run params (cursor from `source_cursors`, `download_root` from artifact root), mints a `sync_source` job, then `claim_job_by_id`, then executes |
| `--ingest` dispatch | `main.rs:1390-1414` | maps source string → live-loop fn |
| Commercial stages | `main.rs:12345` `run_net_margin_gate_once` (wrapper `run_net_margin_gate_live_loop` :12403), `:15043` `run_manifest_generation_live_loop`, `:10532` `run_landed_cost_live_loop` | `stage_run_id`-keyed whole-DB batch passes; not `sync_source`, not cron-bound |
| **In-process sequential stage runner** | `main.rs:23323` `run_deterministic_pipeline` (via `--pipeline`, dispatch at `:1423`) | **see §4 finding C — the doc omits this entirely** |
| Resource arbiter | `crates/daemon/src/agents/arbiter.rs:13` `ResourceBudgets::default` (browser 2 / agent 4 / proc-lane 1 / supplier-lane 2); `:194` `admit → ArbiterDecision::Granted(ResourceLease) | Denied(DenialReason)`; `:231` `release(lease, ReleaseCause)`; `DenialReason::{BrowserSlotsExhausted, AgentSlotsExhausted, SourceLaneExhausted, SkillConcurrencyCapReached}` |
| Register-interest guard | `main.rs:15371` `load_procontract_interest_candidates` — excludes `process_stage_status` rows with `stage='interest_registration' AND state IN ('completed','succeeded','skipped')`; `:15600` `persist_custom_process_stage_state` |
| Stage-state table | `migrations/0001_sqlite_state_initial.sql:453` `process_stage_status` — keyed `(process_id, stage, scope_kind, scope_id, stage_run_id)`, `state` is a free string, has `process_stage_heads` companion; **no `notice_id` column** |
| Source cursors | `migrations/0003_sqlite_runtime.sql:79` `source_cursors (source_id, cursor_key, cursor_value, updated_at)` |
| ProContract IDs | `contract_notices.source_notice_id` = advertId (held from discover); `rfx_id`/RFx URL only exist after access granted (extracted from live tender event page) |
| Desktop triggers | `apps/desktop/src-tauri/src/main.rs:2005` `trigger_pipeline_rerun`, `:1755` `trigger_ingestion`, `:1114` `run_daemon_ingest_stage`, `:78` `PIPELINE_RERUN_INGEST_STAGES = ["landed-cost","net-gate","manifest"]` |
| Daemon IPC enqueue-only | `crates/daemon/src/ipc/handlers/jobs.rs:222` `trigger_ingestion` (enqueues `sync_source`), `trigger_enrichment` (enqueues `enrich_process`) — both enqueue-only, no consumer |
| Schema epoch | `crates/daemon/src/db/sqlite.rs:89` `DEFAULT_SCHEMA_EPOCH = 10`; `apps/desktop/src-tauri/src/main.rs:51` `EXPECTED_SCHEMA_EPOCH = 10`; `MIGRATIONS` array at `sqlite.rs:33`, last entry `0014_installation_labour_fields` (epoch 10) |
| Enrich skill | `skills/procurement-ingestion/procontract-enrich-tender/SKILL.md` — Step 2 "My Activities → Recently added", `maxPages: 5` (~50 rows), `summary.not_yet_responded`, cursor `{"last_checked_at":…,"pages_checked":…}`, live-event detection "Not started (Respond by: …)" vs "Expression of interest accepted", CDP `:9222` |

**Project context:** `AGENTS.md` (Rust 2024 nightly, `#![forbid(unsafe_code)]`, clippy pedantic+nursery `-D warnings`, **real E2E no mocks**, no file proliferation, no backwards-compat shims, schema-epoch coupling, CDP `:9222` discipline). The doc already respects these; keep it that way.

---

## 4. Embedded research findings — what the fresh pass discovered

The doc's `file:line` references are accurate (most within ~11 lines of actual, a few exact; none structurally wrong). The architecture analysis in §2 is sound and verified. The following are the **deltas** — corrections and gaps. Fold each into the doc precisely.

### A. CORRECTION — schema epoch is 10, not 9 (and the memory is stale)
§6 R6 says the desktop/daemon epochs are "(both 9)". **They are both 10 today** (`DEFAULT_SCHEMA_EPOCH = 10`, `EXPECTED_SCHEMA_EPOCH = 10`); migration `0014_installation_labour_fields` bumped to epoch 10 on ~2026-06-02 (the goods-led install/maintenance expansion). Correct §6 R6 to say "both currently **10**", and make §4.4 / §8 Q4 say a new G4 **table** bumps the pair to **11** (next free migration is `0015_*`). Keep the principle identical (bump both sides together; a new state-string is not a bump) — only the numbers are wrong.

### B. SHARPEN — `trigger_pipeline_rerun` is a *desktop* command that kills+restarts the daemon
The doc's §4.5/§7/§10 are correct that it "stops the daemon for an exclusive run," and §10 correctly attributes it to `apps/desktop/src-tauri/src/main.rs`. Make the mechanism explicit so the spec doesn't mis-model it: `trigger_pipeline_rerun` (desktop, `:2005`) (1) **signals the detached `--serve` daemon to stop by PID and waits for it to release the runtime lock** (`stop_daemon_for_exclusive_pipeline_rerun`), (2) shells `daemon --ingest landed-cost`, `--ingest net-gate`, `--ingest manifest` as **one-shot subprocesses** (`PIPELINE_RERUN_INGEST_STAGES`), (3) **relaunches the detached daemon** and waits for handshake. So the exclusivity comes from the *desktop killing the daemon*, not the daemon yielding — which is exactly why an in-process autonomous trigger cannot reuse it. State this as a one-liner in §4.5.

### C. GAP — the doc omits an existing in-process sequential stage runner (`run_deterministic_pipeline`)
There is a **third** execution surface the doc never mentions, and it directly informs §4.5: `run_deterministic_pipeline` (`main.rs:23323`, invoked by `--pipeline <source_id>` at `:1423`) runs a **synchronous, in-process, daemon-resident** sequence — extract-lots → prequalify → qualify-lot → reclassify-candidates → score-quoteability → **gross-margin-gate** → relate — taking `&mut runtime.state_db`, **without stopping the daemon**. It covers the *qualification* stages (up through the gross-margin gate), **not** the *commercial* stages (`landed-cost`→`net-gate`→`manifest`) that §4.5 targets — but it is the existing precedent for "run a deterministic stage chain in-process under the daemon." §4.5 should (a) acknowledge this runner exists, (b) frame the commercial-stage trigger as either *extending the same in-process sequential pattern* or *routing the commercial stages through the worker/arbiter as queued jobs*, and (c) note the open design question this raises: do the deterministic commercial sweeps run on the worker thread (subject to head-of-line blocking behind a long PTY job — see Q3) or as a separate in-process sweep like `run_deterministic_pipeline`? They consume **no** browser slots, so arbiter pressure is near-zero; the real question is scheduling/ordering, not resource contention. This is the single most under-scoped part of the doc — give it the depth it deserves.

### D. CONFIRM + embed — the refactor entanglement (§4.3) is exactly as described, with one concrete detail to add
Verified: live-loop bodies derive per-run params *then* mint+claim. `create_source_sync_job_from_parameters` (`main.rs:4511`) writes a `workflow_jobs` row whose `input_payload` carries `{source_id, trigger:"manual_cli", skill_binding, agent_cli, job_parameters, durable_step_surfaces}` — i.e. the *manual* path packs `job_parameters` (cursor, download_root) **into the job at mint time**. The *cron* path (`build_enqueue_request`, `cron.rs:341`) packs only schedule metadata `{source_id, skill_id, skill_version, scheduled_for, schedule_key}` — **no `job_parameters`**. So the doc's §4.3 "payload mismatch" is real and precisely this: the shared body must branch on "did the payload already carry `job_parameters` (manual) or must I derive them from `source_id`+state (cron)?" Add this concrete payload contrast to §4.3 so the spec knows the exact shape divergence, not just that one exists.

### E. CONFIRM — the §4.4 re-registration landmine is exactly right, with the precise guard string
The guard is literally `state IN ('completed','succeeded','skipped')` (`main.rs:15391`). `process_stage_status` has **no `notice_id` column** (keyed on `process_id`+stage+scope), and `state` is a free string (not a DB-enum), so an `awaiting_access` value is schema-legal without migration — but it breaks the guard exactly as the doc warns. The doc's recommendation (dedicated `procontract_pending_enrichment` row, leaving `interest_registration.state='completed'` intact) is the safer call; the table-vs-state-string tradeoff is correctly captured. One addition: because `process_stage_status` is keyed by `process_id` (not `notice_id`) and the sweep must key off `source_notice_id`/advertId, a dedicated row that stores the advertId is *also* a cleaner data fit than overloading a stage row that lacks that identifier. Note this as additional support for the dedicated-row option in §4.4/Q4.

### F. CONFIRM — clock seam already exists; worker/tick threading is de-risked
`dispatch_due_jobs(registry, now)` already takes `now`. The `SyncLeaseHeartbeat` pattern (own connection + `recv_timeout` + `mpsc` stop) is a complete, working template for both the new tick thread and the worker. `claim_job_by_id` already exists for the `--ingest` path's reuse. The doc's §4.1/§4.2/§4.3 plans are all directly supported by existing primitives — the doc could state more confidently that the *primitives* exist and the work is *composition + one refactor + new handlers*, not net-new infrastructure. (Keep the honest "highest-risk decision" framing on §4.3/§6 — that's correct.)

### G. ADD — defer-vs-fail mechanics for the worker (§4.2) can be made concrete
The arbiter returns `ArbiterDecision::Denied(DenialReason)` vs `Granted(ResourceLease)`; release takes a `ReleaseCause`. The claim path already has `recycle_expired_leases`. So the doc's "release back to `pending`, don't fail" can be pinned to specifics: on `Denied`, the worker must put the job back to `pending` (or `retry`) **without** incrementing the failure/attempt count as a *run* failure, and **without** holding a lease — distinct from a genuine execution error which goes through `fail_job`. Worth a sentence in §4.2 naming the `Denied` vs execution-error fork explicitly.

---

## 5. Improvement directives

Apply these to the target doc. Stay surgical — the doc is already good; do not bloat it.

**CORRECT**
- §6 R6 and §4.4 and §8 Q4: schema epoch `9` → `10`; a new G4 table → epoch `11`, migration `0015_*` (finding A).

**SHARPEN**
- §4.5: make the `trigger_pipeline_rerun` mechanism explicit (PID-stop → one-shot `--ingest` subprocesses → relaunch) and state plainly why an in-process trigger can't reuse it (finding B).
- §4.3: add the concrete cron-vs-manual `input_payload` shape contrast (manual packs `job_parameters`; cron packs only schedule metadata) (finding D).
- §4.2: name the `Denied(DenialReason)` vs execution-error fork and the "no attempt-count bump / no held lease on defer" rule (finding G).

**EXPAND (the one section that needs real depth)**
- §4.5 / §2.5 / §8 Q8: introduce `run_deterministic_pipeline` as the existing in-process sequential-stage precedent, and reframe the commercial-stage trigger decision as "worker-queued jobs vs in-process sweep like `run_deterministic_pipeline`," noting commercial stages consume no browser slots so the real axis is scheduling/HOL-blocking, not resource contention (finding C). This is the highest-leverage improvement in the pass.

**STRENGTHEN (think independently here)**
- The doc lists the three trigger models for commercial stages (chained / scheduled / coverage-driven) and leans "scheduled or coverage-driven." Pressure-test that lean against the existence of `run_deterministic_pipeline` and the dedupe concern. Is there a cleaner unification — e.g. a single "advance-pipeline" sweep (coverage-driven) that handles BOTH qualification and commercial deterministic stages in-process, leaving only the PTY source scrapers on the worker/arbiter? Surface the strongest option, don't just relay the doc's hedge.
- Concurrency (§6): the doc names R1–R6. Add the failure mode that isn't named: **what happens to an in-flight claimed job (lease held, 300s) when the tick thread enqueues the *next* slot for the same source while the previous run is still executing past one tick interval?** The idempotency key is keyed on `scheduled_for`, so two *different* slots are legitimately distinct jobs — does the system tolerate two concurrent `sync_source` runs for the same source, or must per-source serialization (the `procurement_source_lanes=1` budget) prevent it? Trace this and state the resolution.

**ALIGN**
- Terminology: keep "worker pump," "scheduler tick," "live-loop body," "commercial stages," "awaiting-access" consistent with the doc and `AGENTS.md`. Use `process_stage_status` / `process_stage_heads` / `source_cursors` / `workflow_jobs` exactly as the schema names them.

---

## 6. Non-negotiables (do not touch)

- The PTY/browser-session/skill-first model stays (N2). No API connectors.
- One daemon per org (N3). No multi-daemon, no remote/HTTP trigger (N4).
- No retuning of existing schedule values (FTS 04:15 etc. stay code-defined).
- Real E2E, no mocks (project rule) — don't soften this anywhere in the doc.
- The doc's honest framing (e.g. "this is a refactor not a match arm," "highest-risk decision," "at current volume this gap is inert") is a feature. Preserve the candor; don't paper over hard parts.

---

## 7. What I don't want

- Don't rewrite §1, §2, §3 — they're strong and verified. Touch them only for the epoch correction.
- Don't add generic "best practices" filler. Every addition must be grounded in a specific code surface from §3/§4.
- Don't edit any file other than the target doc.
- Don't invent new goals. The scope is fixed at G1–G6 / N1–N4.
- Don't turn the doc into a spec — keep it research-altitude (decisions framed for the spec to make, with the tradeoffs made explicit).

---

## 8. What a great outcome looks like

- The two stale facts (epoch) are corrected everywhere they appear.
- §4.5 is no longer the weakest section: it names `run_deterministic_pipeline`, explains the desktop-kill mechanism of `trigger_pipeline_rerun`, and frames the commercial-trigger decision as a crisp worker-queued-vs-in-process-sweep choice with the resource/scheduling axis made explicit.
- §4.3 and §4.2 carry the concrete payload-shape and defer-vs-fail details an implementer needs.
- §6 gains the unnamed concurrency failure mode (overlapping same-source runs across ticks) with a resolution.
- A spec author reading the improved doc has no "wait, how does X actually work?" moments on the integration surfaces — every one is pinned to a `file:line`.

---

## 9. Follow-up fresh-eyes pass (run in THIS session, after the edits)

Re-read the entire improved doc top to bottom as if you'd never seen it. Verify:
- **Coherence:** every epoch number now reads "10" (and "11" for the new-table case) consistently; no leftover "9".
- **No contradiction:** §2.5, §4.5, §7 (commercial-stage test), §8 Q8, §9 scope all describe the SAME commercial-trigger model and the SAME relationship to `run_deterministic_pipeline` / `trigger_pipeline_rerun`.
- **No bolt-on:** the `run_deterministic_pipeline` material reads as integrated reasoning, not an appended paragraph.
- **Altitude preserved:** still a research doc (decisions + tradeoffs for the spec), not a half-written spec.
- **Nothing weakened:** the honest caveats, the real-E2E rule, and the verified `file:line` references are all intact.
- **Goals still covered:** G1–G6 each still trace to a concrete section.

Fix anything that fails. Then report: what you corrected, what you expanded, the independent insight you added (the overlapping-run concurrency call and/or the unified-sweep proposal), and any residual open question you're handing to the audit pass.
