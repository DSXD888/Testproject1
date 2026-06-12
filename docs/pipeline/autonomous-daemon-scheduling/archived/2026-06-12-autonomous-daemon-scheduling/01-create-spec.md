# Autonomous Daemon Scheduling — Step 01: Create Spec

## First Action

Invoke the `/research-to-spec` skill.

---

## Your Mission

Turn the research doc `docs/research/2026-06-04-autonomous-daemon-scheduling-research.md` into a production-ready **specification** (SOT #1) + **architecture** doc for a **runtime-autonomy** change to the Rust daemon. The research doc is unusually mature — it is codebase-verified, has resolved 3 design points inline (Q2/Q3/Q8), and carries concrete test manifests (§7). Your job is to convert it into a parseable, requirement-ID'd spec WITHOUT losing a single feature, and to escalate the 7 genuinely-open questions (§8) to the operator.

This feature does **not** change *what* the skills scrape. It makes the *existing* schedules fire and drain autonomously while the daemon sits open, hardens ProContract resumption, and gives the commercial stages an autonomous trigger.

---

## Source Documents (Read In This Order)

### Primary Input (CREATE FROM THIS)
```
docs/research/2026-06-04-autonomous-daemon-scheduling-research.md   — READ FULLY (364 lines, the contract)
```

### Pipeline Context
```
docs/pipeline/autonomous-daemon-scheduling/manifest.yaml            — READ FULLY (scope contract, verified file:lines, open questions)
AGENTS.md                                                           — SKIM (Rule 2 wrappers, Stage Validation Test Gate, toolchain, backwards-compat policy)
```

### Codebase Context (STUDY FOR PATTERNS — verified file:lines in manifest `key_files`)
```
crates/daemon/src/main.rs:3260       — SyncLeaseHeartbeat::spawn (the background-thread + own-connection precedent the tick/worker copy)
crates/daemon/src/main.rs:1703-1727  — run_next_workflow_job (the 3-kind match + `other => fail_job` that this feature extends)
crates/daemon/src/scheduler/cron.rs:120,341 — dispatch_due_jobs + build_enqueue_request (cron kinds + idempotency key)
crates/daemon/src/jobs/runner.rs:580,1626   — enqueue_job + find_existing_enqueue_ack (the dedup that makes the fold safe)
crates/daemon/src/agents/arbiter.rs:14,82-107 — ResourceBudgets::default + DenialReason (the defer-on-denial + lane model)
```

### Quality Bar (MATCH OR EXCEED)
```
docs/pipeline/goods-led-install-maintenance/artifacts/goods-led-install-maintenance-spec.md  — Best existing spec (633 lines)
docs/pipeline/goods-led-install-maintenance/artifacts/goods-led-install-maintenance-architecture.md — Best existing architecture (689 lines)
```

---

## Cardinal Rule: ZERO SCOPE DRIFT

**If it's in the research doc, it gets implemented. No exceptions.**

You may improve HOW (cleaner architecture, sharper ACs), resequence WHEN, and strengthen vague language. You may NEVER remove features, defer them to a non-existent "Phase 2+/post-MVP," label them "nice to have," reduce ACs below the research doc, or silently drop anything.

**The one nuance** — the research doc itself leaves SEVEN questions genuinely open (§8: Q1, Q4, Q5, Q6, Q7, Q8-residual, Q9) and marks four items as operator-optional (Q6 shadow flag, Q7 desktop UX, Q8 chained nudge, `enrich_process` button routing). These are NOT scope cuts. Handle them exactly like goods-led handled `future_tier_b`: **NAME each one, define its scope boundary and the extension point it hangs off, present the recommended default, and escalate to the operator for a decision.** Never silently drop them; never fabricate a decision the research doc deferred.

---

## Key Context From Codebase Research

- **Two stacked gaps + one related gap** (research §2.2/§2.5): (1) no recurring schedule tick — `dispatch_due_jobs` has exactly one caller, `enqueue_startup_cron_jobs` (`main.rs:3697`), invoked once at bootstrap; (2) no always-on worker — `run_next_workflow_job` (`main.rs:1703`) matches only 3 kinds and `other => fail_job`s everything else (`:1722`), and is only reachable via `--run-next-job`; (3) the commercial stages (`landed-cost`→`net-gate`→`manifest`) have no autonomous trigger at all.
- **The concurrency foundation already exists** — WAL + `busy_timeout=5000` (`sqlite.rs:215-218`), and `SyncLeaseHeartbeat::spawn` (`main.rs:3260`) is a proven background-thread-with-own-connection pattern the tick and worker copy.
- **Three resolved design points** (transcribe verbatim, do NOT re-open): Q2 fold-startup-into-tick, Q3 two-lane worker, Q8 commercial-trigger model (scheduled `commercial_sweep` cron kind + coverage body, in-process, NOT `trigger_pipeline_rerun`).
- **The §4.4 re-registration landmine** is a correctness requirement, not advice: awaiting-access must NOT flip `interest_registration.state` away from `'completed'`, or register-interest re-fires every run.
- **Schema epoch is 10** (`sqlite.rs:89`), all four desktop/daemon constants confirmed at 10, highest migration is `0014`. A G4 dedicated table (Q4) ships as `0015_*` and bumps every epoch constant 10→11 in one commit (R6).

---

## Step-by-Step Workflow

### Step 0: Read Source Documents (MANDATORY — before doing anything)
1. `docs/research/2026-06-04-autonomous-daemon-scheduling-research.md` — **READ FULLY**
2. `docs/pipeline/autonomous-daemon-scheduling/manifest.yaml` — **READ FULLY** (the `feature_surface`, `recorded_decisions`, `open_questions_for_spec`, `key_files`)
3. `AGENTS.md` — **SKIM** (Rule 2, Stage Validation Test Gate §"Stage Validation Test Gate", toolchain, no-backwards-compat policy)
4. `crates/daemon/src/main.rs:3260` and `:1703-1727` — **STUDY** (the two patterns the feature extends)
5. `docs/pipeline/goods-led-install-maintenance/artifacts/goods-led-install-maintenance-spec.md` — **MATCH OR EXCEED THIS QUALITY**

### Step 1: Invoke the Skill
Run `/research-to-spec`.

**Inputs:**
- Source: `docs/research/2026-06-04-autonomous-daemon-scheduling-research.md`
- Spec output: `docs/pipeline/autonomous-daemon-scheduling/artifacts/autonomous-daemon-scheduling-spec.md`
- Architecture output: `docs/pipeline/autonomous-daemon-scheduling/artifacts/autonomous-daemon-scheduling-architecture.md`

**Key context to provide the skill:**
- Requirement IDs must be **goal-anchored** so coverage is mechanically checkable: organize the spec around G1–G6 (research §3) plus the cross-cutting concurrency model (§6). Suggested ID scheme: `FR-G1.x` (scheduler tick), `FR-G2.x` (worker pump + new handlers), `FR-G3.x` (manual-path preservation + live buttons), `FR-G4.x` (ProContract resumption), `FR-G5.x` (concurrency correctness), `FR-G6.x` (commercial sweep), `FR-RT.x` (live-loop refactor, the §4.3 cross-cutting work), `NFR-x` (non-goals N1–N4 as guardrails).
- The §4.3 per-source parameter-derivation table and the §4.4 lifecycle table must survive into the spec **as tables** — they are the implementable core, not prose.
- Transcribe the 3 resolved decisions (Q2/Q3/Q8) into the spec as DECIDED design, with the research's rationale.

### Step 2: Feature-Specific Guidance
While the skill runs, ensure the spec pins these precisely:
- **The clock-injection seam is a design requirement** (`run_scheduler_tick(scheduler, registry, db, now)` + `--run-scheduler-once --tick-now <rfc3339>`), not a test hook. Without it the §7c M1/M2 manifests are unwritable. Give it its own requirement under FR-G1.
- **Defer-on-denial** must be specified precisely: denied admission → `status='pending'`, NO `attempt_count` bump, NO held lease, denial backoff (prefer in-memory per-lane cool-down to avoid a schema change). A denial is NOT a failed attempt — make the spec say so.
- **The two-lane model** has a falsifiable claim (worker_004 head-of-line): the spec must STATE it and define the fallback (kind-aware pool) if it fails.
- **R7** (`procurement_source_lanes=1` serializes same-source runs) is an invariant the spec asserts and a test (worker_003/M5) proves.
- **Bound the idempotency guarantee correctly (verified against `runner.rs:1626`).** `find_existing_enqueue_ack` dedups on `status IN ('pending','running','retry','completed')` — it deliberately **excludes `'failed'`**. So the spec's tick/enqueue idempotency requirement must be stated as "at most one *live-or-succeeded* job per slot" — a slot whose prior job FAILED is legitimately re-enqueueable by a later tick (desirable retry behavior, NOT a dedup bug). Do NOT write the spec to promise "exactly-once forever" or to suppress a failed slot. The Q2 fold-safety (bootstrap vs first tick) is unaffected because that race occurs while the job is still `'pending'`.
- **The commercial sweep must NOT reuse `trigger_pipeline_rerun`** — call this out as a hard contract with the reason (it's a desktop PID-stop affordance; an in-process trigger can't reuse it). Research §9 says the autonomous trigger must **reconcile (not reuse)** that path — so the spec must also address the INTERACTION: `trigger_pipeline_rerun` stays a manual desktop affordance, but it kills the daemon by PID. If an operator clicks "Re-run pipeline" while the autonomous `commercial_sweep` is mid-run, the desktop SIGTERMs the daemon mid-sweep. The spec must state how this is handled (e.g. the sweep's per-process-scoped transactions (R8) mean a mid-sweep kill leaves committed-per-process progress and the next 07:00 sweep is self-healing/coverage-idempotent) — i.e. the two paths coexist safely, they don't corrupt each other.
- **Coverage-lane arbiter nuance** — within `commercial_sweep`, `net-gate` is a **pure DB sweep** (`run_net_margin_gate_once`, no skill), but `landed-cost` and `manifest` **may invoke their skill runners** for eligible rows and therefore **consume agent/skill slots** — so the coverage/deterministic lane jobs take NO procurement-source browser slot, but the `ResourceArbiter` is still in force for the skill-backed sub-passes. The spec must not imply the coverage lane is arbiter-free.

### Step 2c: Architecture Doc — Required Content (do not leave it thin)
The architecture doc is the companion to the spec. For THIS feature it must contain, concretely:
- **Thread model diagram** — the steady-state daemon: serve/IPC loop (owned `state_db` handle), lease heartbeat (own connection, `main.rs:3260` precedent), the NEW scheduler-tick thread (own connection), and the NEW worker lane(s) — PTY/browser + coverage/deterministic — each with its own `StateDb` connection (or the shared-services alternative, pending Q1). Show which thread owns which connection.
- **WAL single-writer story** — up to 4 concurrent writers to one `state.db`; how short transactions + `busy_timeout=5000` keep them safe; the per-process-scoped sweep-transaction rule (R8); the escalation path to a single-writer actor (R1).
- **Data-flow** — cron schedule → tick `dispatch_due_jobs` → `enqueue_job` (dedup by idempotency key) → `claim_next_job` (overdue-first) → lane routing by kind → execute (refactored shared body / new handler / coverage sweep) → complete/fail → loop.
- **The awaiting-access state machine** (the §4.4 lifecycle: create/keep/clear/backfill/expire) as a diagram, with the landmine annotation (`interest_registration.state` stays `'completed'`).
- **The Q1 decision record** (own-connection-per-lane vs shared services) with the rationale, since everything in §4.2/§4.3 hangs off it.

### Step 3: Escalate the Open Questions (MANDATORY)
The research §8 leaves Q1, Q4, Q5, Q6, Q7, Q8-residual, Q9 open. For EACH, the spec must: (a) name it, (b) state the options + the research's recommended default, (c) define the extension point it hangs off, and (d) mark it as an **operator decision needed**. Then **ask the operator** to resolve them — at minimum Q1 (worker exec context — the spec's first decision per §11) and Q4 (awaiting-access modeling — drives whether there's a schema-epoch bump). Record answers verbatim. Do NOT fabricate decisions; do NOT drop the underlying feature if the operator defers.

### Step 4: Verify Output
- [ ] `autonomous-daemon-scheduling-spec.md` exists with systematic, greppable requirement IDs
- [ ] Every item in manifest `feature_surface` (scheduler_tick, worker_pump, live_loop_refactor, new_job_kind_handlers, commercial_sweep, procontract_resumption, concurrency_correctness, desktop_status_surfacing, tests) maps to ≥1 requirement
- [ ] G1–G6 each have a §7d-style "correct proof + regression shape" success criterion
- [ ] The §4.3 derivation table and §4.4 lifecycle table are present as tables
- [ ] All 7 open questions are named + escalated, not silently resolved or dropped
- [ ] The architecture doc documents the thread model (serve/IPC, lease heartbeat, scheduler tick, ≤2 worker lanes) and the WAL single-writer correctness story (R1–R8)
- [ ] No "post-MVP / Phase 2+ / stretch goal / nice to have / deferred" language anywhere

### Step 5: Follow-Up Pass (Sub-Optimization)

> Send this as a follow-up message in the same session after the spec is drafted. ULTRATHINK.

Re-read the spec you just wrote with completely fresh eyes, against the codebase ground truth:

1. Open `crates/daemon/src/main.rs:1703-1727` (`run_next_workflow_job`). The current match has exactly 3 arms (`extract_document_structure`, `extract_document_semantics`, `run_bid_workflow`) and `other => fail_job`. Does your spec's FR-G2 enumerate ALL the new arms the worker must add — `sync_source`, `source_health`, `auth_refresh`, `validation_cross_model_audit`, `validation_calibration_readiness_sweep`, `commercial_sweep`? Does it note that `source_health`/`auth_refresh`/both `validation_*` have **no executor today** (new handlers, more than wiring), while `sync_source` reuses refactored live-loop logic? If any kind is missing from the spec, add it.

2. Open `crates/daemon/src/scheduler/cron.rs:341-397` (`build_enqueue_request` + idempotency key). The cron-enqueued `sync_source` `input_payload` carries SCHEDULE metadata (`source_id`, `skill_id`, `skill_version`, `cron_expression`, `scheduled_for`) — NOT the per-run execution params the `--ingest` wrappers compute today. Does your spec's FR-RT (live-loop refactor) require the shared body to DERIVE exec params at execution time per-source, and does it reproduce the §4.3 derivation table keyed on `(source_id, skill_id)` (not `source_id` alone)? If it collapses discover/interest/enrich into one `source_id=procontract` mapping, that's a bug — fix it.

3. Open `crates/daemon/src/main.rs:15371` (`load_procontract_interest_candidates`) — it excludes `state IN ('completed','succeeded','skipped')`. Does your spec's FR-G4 explicitly forbid flipping `interest_registration.state` to `awaiting_access` (the §4.4 landmine), and does the Q4 table option keep that row `'completed'`? Does the Q4 state-string option REQUIRE widening this guard's state-set? If the landmine isn't called out as a blocking correctness rule, add it.

4. Open `crates/daemon/src/agents/arbiter.rs:82-107` (`DenialReason` + `ArbiterDecision`) and `crates/daemon/src/jobs/claim.rs:72,176` (`claim_next_job` + the `COALESCE(scheduled_for, created_at)` ordering). Does your spec's FR-G2 defer-on-denial requirement distinguish "denied admission → defer (return to `pending`, NO `attempt_count` bump, NO lease, backoff)" from "ran → failed (attempt bump)", and does it name the exact `DenialReason` variants the worker must handle (`BrowserSlotsExhausted`, `SourceLaneExhausted{source_id,limit,in_use}`)? Does FR-G1 rely on the overdue-first claim ordering so a missed slot is picked before newer manual work? If either is hand-waved, sharpen it against these signatures.
5. Count the requirements. The research has 6 goals + a cross-cutting refactor + a concurrency model + 7 open questions. A spec with < ~25–30 requirements has almost certainly under-specified something (compare goods-led's depth). Are the per-source derivations, the 5 lifecycle transitions, and the defer-on-denial mechanics each their own AC, or were they hand-waved into one line?

6. Re-read every success criterion. Is each one PROVABLE by a query/command (like research §7d), with a named regression shape? "Scheduler works" is not a criterion; "`scheduler_tick.rs` tick_001 enqueues exactly one `sync_source` row at injected `now=04:15`; regression = due slot never enqueues without restart" is.

Then re-run **Step 4's checklist** and the scope-drift scan once more before declaring the spec done.

### Step 6: Scope-Preservation Check (MANDATORY — BLOCKING)

Do this last, as a gate. Do not declare the spec done until all four pass:

1. **Enumerate** every item in manifest `feature_surface` (scheduler_tick ×8, worker_pump ×6, live_loop_refactor ×5, new_job_kind_handlers ×4, commercial_sweep ×5, procontract_resumption ×7, concurrency_correctness, desktop_status_surfacing, tests) AND every goal G1–G6 + non-goal N1–N4.
2. **Verify** each maps to ≥1 spec requirement with equivalent depth — nothing silently dropped, no later goal thinner than an early one.
3. **Scan** the entire spec for red-flag language: "post-MVP," "Phase 2+," "stretch goal," "nice to have," "deferred," "out of scope" (except the explicit non-goals N1–N4), "future iteration," "simplified for initial implementation," "can be added later." Any hit OUTSIDE the explicit operator-escalated open-question blocks (Q6/Q7/Q8/enrich_process) is a BLOCKING failure.
4. **Confirm** the 7 open questions are NAMED + escalated (not silently resolved, not dropped). If any feature is missing or any red flag is found, fix it before proceeding.
