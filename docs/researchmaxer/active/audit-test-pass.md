# Audit & Test Pass — Autonomous Daemon Scheduling & ProContract Resumption

> **You are the last quality gate before this research doc becomes a spec.** One session: audit for gaps/gotchas/contradictions → define a complete REAL-E2E testing strategy → edit the target doc with both → fresh-eyes verify. You edit exactly ONE file.

**Run this AFTER `improvement-pass.md` has been executed.**

**Target document (the ONLY file you may edit):**
`docs/research/2026-06-04-autonomous-daemon-scheduling-research.md`

---

## 1. Mission

The improvement pass strengthened the design. Your job is twofold:

1. **Audit** the (now-improved) doc for implementation-readiness gaps, contradictions, undocumented failure modes, and gotchas an implementer will hit.
2. **Define the testing strategy** to the standard this codebase actually enforces: **real end-to-end, no mocks/stubs** (`AGENTS.md`). The doc's §7 is a good skeleton (clock-injection seam, worker defer, ProContract lifecycle, concurrency soak) — your job is to turn it into manifests concrete enough that a fresh implementation agent could build *and verify* the feature from the doc + your test definitions alone, matching existing test conventions exactly.

A fresh deep-research pass mapped the test infrastructure precisely (§4 below). Use it — don't re-discover it.

---

## 2. What to read

- **The target doc**, fully — paying special attention to §7 (testing strategy), §6 (concurrency risks R1–R6), §8 (open questions), and the §4.5 commercial-trigger material the improvement pass expanded.
- **The improvement-pass output** (the edited target doc) — your audit is against the *improved* version, not the original.
- **`AGENTS.md`** — the quality gate is explicit and non-negotiable (reproduced in §5d below). Real E2E, no mocks. Integration tests are per-file targets.
- **Codebase test surfaces** — the exemplar files in §4. Read at least `crates/daemon/tests/cron_scheduler.rs`, `crates/daemon/tests/job_claim.rs`, `crates/daemon/tests/agent_procurement_ingestion.rs`, and the `crates/daemon/tests/support/` helpers before writing any manifest, so your manifests match real patterns (the `ts()` helper, `TestStateDb`, `CronSchedulerHarness::advance_to`, the `assert_cmd` + fake-claude subprocess harness).

---

## 3. Audit directives

Go subsystem by subsystem. For each, decide: *is the design complete enough to implement, are failure modes documented, what will an implementer trip on?*

**Scheduler tick (§4.1)**
- Is "build the scheduler once at thread start, don't re-seed every tick" airtight against a source being enabled/disabled mid-run? The doc says "reload registry only when it changes" — but *how* does the tick learn it changed (IPC sets a flag? version counter? poll-on-change)? Pin the mechanism or flag it as a Q for the spec.
- The "fold startup-enqueue into the tick (run once immediately, then on interval)" lean — confirm the idempotency key genuinely makes this safe even when bootstrap's `enqueue_startup_cron_jobs` and the first tick race. Trace the dedup (`find_existing_enqueue_ack`).

**Worker pump (§4.2)**
- Single worker vs pool (Q3): the doc leans pool. Audit the *head-of-line blocking* claim — a long PTY `sync_source` run blocking quick deterministic commercial stages. Is the pool the right fix, or does separating PTY-source jobs from deterministic-stage jobs onto different lanes/workers solve it more cleanly? Make the doc commit to a falsifiable position or a crisp Q.
- Defer-on-denial: is "back to pending, no attempt bump, no held lease" fully specified, including the *backoff* so a permanently-over-subscribed queue doesn't hot-loop re-deferring?

**Live-loop refactor (§4.3)**
- The cron-vs-manual payload divergence (manual packs `job_parameters`; cron packs only schedule metadata): is the "derive at execution time from `source_id`+state" path specified for EACH cron-bound source (FTS, contracts-finder, procontract discover/interest/enrich)? Enrich specifically needs cursor (from `source_cursors`) + `download_root` (from artifact root) derived — confirm the doc says so per-source, not just generically.

**Commercial-stage trigger (§4.5)** — the riskiest area
- After the improvement pass, does the doc clearly pick (or clearly defer with criteria) between worker-queued jobs and an in-process sweep modeled on `run_deterministic_pipeline`? A vague "scheduled or coverage-driven" without resolving the execution-context question (worker thread vs in-process) is still under-specified for the spec.
- Dedup: "several ingestion runs in one window shouldn't trigger redundant commercial sweeps" — is the dedup key/condition defined (coverage query? stage_run cohort? time window?)?

**ProContract resumption (§4.4)**
- Lifecycle completeness: create-on-success → keep-while-no-event → clear-on-enrich → backfill-untracked → staleness-expiry. Is EACH transition's trigger and the exact state/row it writes specified? The expiry (Q5) must NOT key off the RFx response deadline (unknown for awaiting-access items) — confirm the doc uses notice-level `deadline_at` or max-age. 
- Skill contract: the enrich skill must accept an explicit work-list of advertIds in addition to its "Recently added" scan. Is the input shape and the merge-with-Recently-added behavior specified well enough to write the SKILL.md change?

**Cross-cutting**
- Contradictions: scan for any place §2.5/§4.5/§7/§8/§9 disagree on the commercial-trigger model or on the `run_deterministic_pipeline`/`trigger_pipeline_rerun` relationship.
- Schema: every epoch reference reads "10" (and "11" for the new-table case)? Q4's table-vs-state-string decision stated with its consequence (widen the guard if state-string; bump epoch if table)?
- The weakest remaining point: name it explicitly in your audit summary.
- Hostile-reviewer challenge: what would a skeptical senior engineer attack first? (Likely: "two concurrent same-source `sync_source` runs across a tick boundary," or "the worker pool + WAL single-writer under IPC mutation pressure.") Make sure the doc has an answer or an owned open question.

---

## 4. Embedded testing landscape (verified — use this, don't re-discover)

**The gate (from `AGENTS.md`, enforced):**
```bash
cargo fmt --check
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test -p daemon --bin daemon                      # binary unit tests
for t in crates/daemon/tests/*.rs; do                  # EACH integration file is its own target
  cargo test --test "$(basename "$t" .rs)" || exit 1
done
```
`cargo test -p daemon --bin daemon` alone is **insufficient** — it silently skips the integration files (this caused two false-GREEN Stage 11 manifests). Environment-dependent tests must auto-skip via runtime probes, never `#[ignore]`.

**Time control — there is NO global clock mock.** Time is threaded explicitly:
- `dispatch_due_jobs(registry, now)` already takes `now` — the clock-injection seam the doc's §7 calls for **already exists at the scheduler layer**. The doc's real test requirement is threading that same `now` through the *tick thread* so a test can drive "it is 04:15" without advancing a live `--serve` wall clock.
- `crates/daemon/tests/cron_scheduler.rs:77` `fn ts(y,mo,d,h,mi,s) -> DateTime<Utc>` — the fixed-instant helper used everywhere.
- `crates/daemon/tests/support/cron.rs` `CronSchedulerHarness` with `.advance_to(instant)` (multi-step time progression), `.seed_definition()`, `.run_due_sweep()`.

**Test DB construction (real SQLite, tempdir):**
- `crates/daemon/tests/support/db.rs` — `TestStateDb::new()` / `L4TestStateDb::new()` open a real SQLite DB in a tempdir at the current `DEFAULT_SCHEMA_EPOCH`; seed helpers `seed_process`, `seed_raw_document`, etc.; schema-introspection helpers (`logical_schema_for_tables`, table/column assertions).

**Subprocess E2E harness (whole-daemon, real binary):**
- `crates/daemon/tests/agent_procurement_ingestion.rs` `RunHarness` — tempdir state/runtime/artifact roots; runs `Command::cargo_bin("daemon").arg("--ingest").arg("fts")…`; a **fake-claude shell script** (mode 0o755) responds to `--version`/`probe`/result-path env vars so the PTY/agent side is deterministic without mocking the daemon. `DAEMON_TEST_CLAUDE_RESULT_PATH` / `…_CHECKPOINT_PATH` env vars feed canned skill results. Post-run, opens the real SQLite and asserts rows. This is the model for any "full autonomous run" E2E.

**Exemplars to copy (verified):**

| Test scenario | Copy this file | Key patterns |
|---|---|---|
| **Scheduler-tick** (time-injected dispatch, idempotency, catch-up) | `crates/daemon/tests/cron_scheduler.rs` (esp. tests 003 dispatch shape, 005 persist `scheduled_for`, 006 startup catch-up priority order) | `ts()`, `TestStateDb::new()`, `CronScheduler::from_registry` → `refresh_next_run_at(now)` → `dispatch_due_jobs(registry, now)`; assert exactly one `workflow_jobs` row per slot under repeated ticks |
| **Worker-pump** (claim → route → complete; defer-on-denial; arbiter caps) | `crates/daemon/tests/job_claim.rs` + `crates/daemon/tests/resource_arbiter.rs` | `seed_job()`/`seed_running_job()`, concurrent-claim barrier, lease expiry/recycle; arbiter `admit`/`release` with `DenialReason` |
| **ProContract awaiting-access E2E** (greenfield) | `crates/daemon/tests/agent_procurement_ingestion.rs` (`RunHarness` + fake-claude) | subprocess `--ingest procontract-interest` then `--ingest procontract-enrich`; seed/assert the awaiting-access row across runs |
| **Schema migration** (if G4 adds a table) | `crates/daemon/tests/sqlite_schema.rs` | `PRAGMA user_version == DEFAULT_SCHEMA_EPOCH`; `schema_migrations` row present; table/columns exist |

**Critical gap to flag in the doc:** there is **no existing ProContract integration test file** — register-interest/enrich are only referenced inside `cron_scheduler.rs` schedule assertions. So the G4 E2E suite is **net-new**, built on the `agent_procurement_ingestion.rs` harness pattern. The testing strategy must say this explicitly (it's more work than "extend an existing test").

**Cron schedules already asserted (so your tick tests have known anchors):** FTS `04:15`, FTS canary every 30 min, ProContract discover `05:00`, register-interest `05:30`, enrich `06:00` (see `cron_scheduler.rs` definition assertions). A new commercial-sweep slot (if §4.5 picks "scheduled") would slot after the ingestion window (doc suggests ~07:00).

---

## 5. Testing strategy requirements (the doc must end up with these)

Produce a §7 (or expanded §7 + a new testing-manifest subsection) covering ALL of the following. Every test must be REAL — real SQLite, real cron parsing, real binary subprocess where the flow demands it; the only permitted "fake" is the deterministic fake-claude CLI already used by `agent_procurement_ingestion.rs` (it stands in for the *agent/PTY*, not for the daemon under test).

**a. Existing-test impact**
- Which existing tests change because the worker now routes more kinds / the live-loop body is refactored? At minimum `cron_scheduler.rs` (new dispatch assertions for any new commercial-sweep kind), `job_claim.rs` (new kinds claimable), and any `--ingest`-driven test whose body moved behind the shared execution function. List them with the expected change.

**b. New unit/integration tests** (per subsystem, naming the exemplar to follow)
- **Scheduler tick:** a due schedule produces exactly ONE `workflow_jobs` row per slot; repeated ticks within the same slot enqueue nothing new (idempotency); a simulated long gap (advance `now` past several slots) produces overdue slots that the next tick enqueues (catch-up). Follow `cron_scheduler.rs`.
- **Worker pump:** enqueue each cron `kind` (`sync_source`, `source_health`, `auth_refresh`, `validation_cross_model_audit`, `validation_calibration_readiness_sweep`) and assert it is claimed → routed → completed (NOT `fail_job "unsupported"`); a slot-denied job defers back to `pending` with no attempt-count bump and no held lease; arbiter quotas honored (browser_slots=2 caps concurrent PTY — note this presumes the pool choice from Q3; with a single serial worker there's no concurrency to cap, so state which model the test assumes). Follow `job_claim.rs` + `resource_arbiter.rs`.
- **Live-loop refactor regression:** `--ingest fts`, `--ingest procontract-enrich`, `--run-next-job` still work (these execute today). Follow `agent_procurement_ingestion.rs`.

**c. E2E manifests (real, not stubs)** — each with: scenario name, preconditions, steps, expected outcomes, verification method (exact query/command).
- **End-to-end autonomy:** drive "it is now 04:15" via the tick's clock seam (thread `now` through the tick thread; the seam is itself a small design requirement, per the doc) OR register a temporary test schedule a few seconds in the future; then assert a full source run executes and persists with NO manual trigger.
- **Commercial-stage autonomy (G6):** after an autonomous ingestion run yields a process needing pricing, assert `landed-cost`→`net-gate`→`manifest` advance to a current `manifest`/decision with no operator action — via whichever §4.5 model is chosen, and **without** the daemon-stop behavior of `trigger_pipeline_rerun`. (If the chosen model is an in-process sweep modeled on `run_deterministic_pipeline`, the manifest should assert it ran on the serve daemon without a stop/restart.)
- **ProContract resumption (G4) full lifecycle:** after register-interest **succeeds**, assert an awaiting-access record persists (and that the `interest_registration` row stays `state='completed'` so the re-fire guard still excludes it — the §4.4 landmine); first enrich pass with no live tender event → record **kept**, and an untracked portal `not_yet_responded` item gets **backfilled**; later enrich pass where the live event now exists → enrich finds it via the daemon work-list (NOT dependent on "Recently added"/`maxPages`), downloads docs, and **clears** the record; staleness expiry fires per the chosen policy (notice-level `deadline_at` or max-age — Q5; explicitly NOT the RFx response deadline). Build on the `RunHarness` subprocess pattern; this is net-new (no ProContract test exists).
- **Manual-path regression vs new capability:** `--ingest fts`, `--run-next-job`, and desktop `trigger_pipeline_rerun` still work (regression). Separately, the desktop `trigger_ingestion` button (`sync_source`) now **executes** via the worker (NEW capability — today it enqueues an orphan). If `enrich_process` routing is folded in (Q/§5/G3), `trigger_enrichment` executes too.
- **Concurrency soak:** worker(s) + IPC mutations + lease heartbeat in parallel under WAL; assert no lost writes, no permanent `SQLITE_BUSY`. **Add the case the improvement pass surfaced:** two slots for the same source firing across a tick boundary while the first run is still in-flight — assert the `procurement_source_lanes=1` budget (or whatever serialization the doc chose) prevents two concurrent same-source runs, OR that concurrent runs are provably safe.

**d. Verification criteria** — for each of G1–G6, state what "correctly implemented" looks like and the exact command/query that proves it, plus what a regression looks like. Map each criterion to the gate commands in §4. Make clear that the stage-validation gate requires the per-file integration loop, not just `--bin daemon`.

**e. Schema-migration test (conditional on Q4)** — if G4 adds `procontract_pending_enrichment` (or any table/column): a `sqlite_schema.rs`-style test asserting `user_version == 11`, the `0015_*` migration recorded in `schema_migrations`, and the table/columns present — AND a note that desktop `EXPECTED_SCHEMA_EPOCH` must move 10→11 in the same change (schema-epoch coupling). If Q4 picks the state-string option instead, no migration test, but a test that the widened re-fire guard still excludes `awaiting_access`.

---

## 6. What a great outcome looks like

- Every audit gap is either fixed in the doc or converted into a precise, owned open question (not a vague "TBD").
- §7 becomes a testing strategy a fresh agent could execute: real-E2E manifests with exact preconditions/steps/queries, each pointing at the exemplar test file to copy.
- The "no ProContract test exists yet → G4 suite is net-new" reality is stated, so the strategy/beads downstream size it correctly.
- The clock-injection seam is described as a *design requirement on the tick thread* (thread `now` through), not just a test wish.
- Verification criteria tie each goal to the actual gate commands, including the per-file integration loop.
- No contradiction survives between §2.5/§4.5/§7/§8/§9 on the commercial-trigger model.

---

## 7. Follow-up fresh-eyes pass (run in THIS session, after the edits)

Re-read the doc + the testing strategy cold. Verify:
- **Coverage:** every subsystem (tick, worker, live-loop refactor, commercial trigger, ProContract G4, concurrency, schema) has at least one real-E2E test; G1–G6 each have a verification criterion.
- **No mock leakage:** nothing in the strategy relies on stubbing the daemon or faking SQLite; the only fake is the agent/PTY (fake-claude), consistent with `agent_procurement_ingestion.rs`.
- **Manifests are executable:** preconditions, steps, and verification queries are concrete enough to run without further design.
- **Gate correctness:** the strategy invokes the full gate (fmt/check/clippy/`--bin daemon`/per-file integration loop), not the insufficient narrow command.
- **Internal consistency:** epoch numbers (10, and 11 for the table case) consistent; commercial-trigger model identical across sections; no leftover references to a daemon-side `trigger_pipeline_rerun` (it's desktop-only).
- **Honest sizing:** anything net-new (G4 ProContract suite, new cron-kind handlers with no executor today) is flagged as new work, not "extend existing."

Fix what fails. Then report: the gaps you found and how each was resolved, the weakest remaining point, the full test inventory (by subsystem, with exemplar file), and any open questions the spec must still answer.
