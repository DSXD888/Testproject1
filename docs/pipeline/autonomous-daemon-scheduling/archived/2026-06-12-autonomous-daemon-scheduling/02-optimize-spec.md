# Autonomous Daemon Scheduling — Step 02: Optimize Spec (Fresh-Eyes Audit)

## First Action

**No skill to invoke.** This is a manual fresh-eyes audit — you read, analyze, and edit the existing spec/architecture directly. Do NOT run `/research-to-spec` or any other skill. Use ULTRATHINK throughout.

---

## Your Mission

Perform a **comprehensive fresh-eyes audit** of the spec at `docs/pipeline/autonomous-daemon-scheduling/artifacts/autonomous-daemon-scheduling-spec.md` — the SOT #1 for a runtime-autonomy change to the Rust daemon (autonomous schedule firing + always-on worker + ProContract resumption hardening + autonomous commercial sweep).

The research doc has been deeply iterated and codebase-verified. Your job is to verify the spec preserves 100% of its feature surface, sharpen every weak requirement, and confirm the design is implementable against the actual codebase.

**You are reading, analyzing, and improving an existing document — not writing code.**

---

## What To Read (Do This First)

### The Document to Audit (TARGET)
```
docs/pipeline/autonomous-daemon-scheduling/artifacts/autonomous-daemon-scheduling-spec.md   — READ FULLY
docs/pipeline/autonomous-daemon-scheduling/artifacts/autonomous-daemon-scheduling-architecture.md — READ FULLY
```

### Verify Against (SOURCE OF TRUTH)
```
docs/research/2026-06-04-autonomous-daemon-scheduling-research.md   — the contract (enumerate every feature)
docs/pipeline/autonomous-daemon-scheduling/manifest.yaml            — feature_surface + open_questions + verified file:lines
```

### Codebase (GROUND TRUTH — verify the spec's claims are real)
```
crates/daemon/src/main.rs:1703-1727  — run_next_workflow_job 3-kind match (the thing FR-G2 extends)
crates/daemon/src/scheduler/cron.rs:120-397 — dispatch_due_jobs, build_enqueue_request, idempotency key
crates/daemon/src/jobs/claim.rs:72,176 — claim_next_job + COALESCE(scheduled_for, created_at) ordering
crates/daemon/src/agents/arbiter.rs:14,82-107 — budgets + DenialReason
crates/daemon/src/main.rs:15371,15600 — interest candidate selection + stage-state persistence (the §4.4 landmine site)
```

### Quality Bar (MATCH OR EXCEED)
```
docs/pipeline/goods-led-install-maintenance/artifacts/goods-led-install-maintenance-spec.md   (633 lines)
```

---

## Think Independently — Audit These Dimensions

### 1. Goal coverage equality (G1–G6)
Each of the 6 goals must have requirements of EQUAL depth. A common failure: G1 (scheduler tick) and G2 (worker) get rich treatment while G4 (ProContract resumption) or G6 (commercial sweep) get a paragraph. G4 is a NET-NEW test suite and a 5-state lifecycle — it deserves the most ACs, not the fewest. Count requirements per goal; flag any imbalance.

### 2. The live-loop refactor (FR-RT) is the meaty part
Research §4.3 calls the refactor "the meaty part." Verify the spec captures: (a) extract the per-source body to accept a *claimed* job it didn't create; (b) thin `--ingest` wrappers preserved; (c) `(source_id, skill_id)` keying; (d) execution-time param derivation per source (the full table); (e) the payload-mismatch handling (schedule metadata vs run params). If any sub-point is missing, the spec under-scopes the highest-effort work.

### 3. Concurrency correctness (FR-G5 / §6)
Verify R1–R8 are each addressed. Especially: R7 (`procurement_source_lanes=1` serializes same-source — denied second run defers), R8 (per-process-scoped sweep transactions, not one giant txn), and the Q1 worker-exec-context decision (own connection per lane vs shared services). If Q1 is still unresolved in the spec, that's acceptable (it's an escalated open question) — but it must be NAMED as the spec's first implementation decision, not omitted.

### 4. Defer-on-denial precision
Verify the spec distinguishes "denied admission → defer (no attempt bump, no lease)" from "ran → failed (attempt bump)". This is subtle and easy to conflate. Verify the backoff mechanism is specified (in-memory cool-down preferred over a `next_eligible_at` column to avoid an epoch bump).

### 5. The §4.4 landmine and Q4 coupling
Verify the spec forbids overwriting `interest_registration.state` (re-registration landmine) and correctly couples Q4: table option → migration `0015_*` + epoch 10→11 across all constants; state-string option → no epoch bump BUT widened re-fire guard. Both must have a §7e test path named.

### 6. Commercial sweep must not be `trigger_pipeline_rerun`
Verify the spec states, as a hard contract, that `commercial_sweep` runs in-process on the coverage/deterministic lane and does NOT invoke `trigger_pipeline_rerun` (the desktop PID-stop affordance). M2's regression shape (manifest only after operator rerun; daemon restarted) should appear as a success criterion.

### 7. The clock seam as a first-class requirement
Verify the clock-injection seam + one-shot `--run-scheduler-once --tick-now` entrypoint is a NAMED requirement under G1, justified as a design requirement (the §7c harness note: `RunHarness.run()` blocks on `Command::output()`, so a live `--serve` daemon can't be driven by the E2E tests).

### 8. Testability — every requirement provable
Each requirement should map cleanly to a named test from research §7 (scheduler_tick.rs tick_001..005, worker_pump.rs worker_001..005, procontract_resumption.rs M3 steps 1–4, M1/M2/M4/M5). If a requirement has no conceivable test, it's vague — sharpen it.

---

## Scope Drift Detection (MANDATORY — BLOCKING)

### Red-Flag Language Scan
Search the ENTIRE spec for these phrases. If ANY are found (other than inside the explicit, operator-escalated open-question blocks for Q6/Q7/Q8/enrich_process), it is a BLOCKING failure — remove and replace with proper content:
- "post-MVP," "Phase 2+," "stretch goal," "nice to have"
- "future iteration," "out of scope" (except the explicit non-goals N1–N4), "can be added later"
- "simplified for initial implementation," "deferred to"

### Coverage Verification
Enumerate every feature in manifest `feature_surface`. For each, confirm a spec requirement covers it with equivalent depth:
- [ ] scheduler_tick (8 items incl. clock seam, one-shot entrypoint, registry-reload, fold, sleep/wake)
- [ ] worker_pump (6 items incl. two lanes, idle backoff, arbiter respect, defer-on-denial, lease-independence)
- [ ] live_loop_refactor (5 items incl. per-source derivation table, (source_id, skill_id) keying)
- [ ] new_job_kind_handlers (4 new handlers — source_health, auth_refresh, both validation_*)
- [ ] commercial_sweep (5 items incl. 07:00 slot, coverage body, in-process, two-layer dedup)
- [ ] procontract_resumption (7 items incl. Q4 modeling, lifecycle table, work-list sweep, skill contract, staleness)
- [ ] concurrency_correctness (R1–R8, Q1)
- [ ] desktop_status_surfacing (Q7 — named + bounded, escalated)
- [ ] tests (3 new suites + M1–M5 + 7a impacts + 7e migration test)

### Drift Checklist
- [ ] Every research §3 goal G1–G6 is represented with equal depth
- [ ] The §4.3 derivation table and §4.4 lifecycle table survived as tables
- [ ] No content silently dropped or weakened vs the research doc
- [ ] The 7 open questions are named + escalated, NOT silently resolved or dropped
- [ ] Non-goals N1–N4 preserved as guardrails (no new sources, no API connectors, no multi-daemon, no remote trigger)
- [ ] The one new slot (07:00 commercial_sweep) is the ONLY schedule change — existing values (FTS 04:15 etc.) untouched

---

## Follow-Up Pass (Run After Fixes)

> After making your fixes, send this as a follow-up message in the same session. ULTRATHINK.

Re-read the fixes you just made. Did they introduce new issues?
1. Re-open `crates/daemon/src/jobs/runner.rs:1626` (`find_existing_enqueue_ack`). Your spec relies on this for the Q2 fold-safety (bootstrap-vs-first-tick no double-dispatch) AND the commercial-sweep per-slot dedup. Does the spec correctly state that BOTH compute the *identical* idempotency key for a slot, making the second insert a no-op? Verify you didn't accidentally describe two different dedup mechanisms.
2. Re-count requirements per goal after your edits — is the depth now balanced across G1–G6, or did fixing one section make another look thin?
3. Re-verify: did any fix introduce a scope-drift phrase? Re-run the red-flag scan on your own edits.
4. Final coverage matrix: enumerate manifest `feature_surface` once more and confirm 100% mapping. If any item still has no requirement, the spec is not done.
