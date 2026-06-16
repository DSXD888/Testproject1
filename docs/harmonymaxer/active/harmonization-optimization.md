# Harmonization Optimization — Autonomous Daemon Scheduling & ProContract Resumption (RETROACTIVE)

> **Run 5** · Final architecture-QA gate. **Run this AFTER `harmonization-handoff.md` (and its Follow-Up Pass) is complete.** A built-in fresh-eyes Follow-Up Pass is at the bottom — send it as a second message in the same session.

You are the **final reviewer** verifying that the autonomous-daemon-scheduling capability was harmonized into `docs/strategy/` flawlessly. The standard is **boringly consistent**: no doc tells a different story about any concept this feature touches. This is **retroactive mode** — the **code is ground truth**; a doc that contradicts the code is wrong even if it reads cleanly.

---

## 1. Deep Re-Read (mandatory before any audit)

- Re-read the research doc: `docs/research/2026-06-04-autonomous-daemon-scheduling-research.md` (esp. §4, §4.4 lifecycle table, §4.5, §5, §6 R1–R8, §8 open questions).
- Re-read **every** doc in `docs/strategy/` — not only the ones changed: all 7 layer docs + `layer-strategy.md` + `cross-reference-index.md` + `README.md`. Cross-document contradictions only surface when you read the whole set fresh.
- Spot-check 4–6 of the code surfaces cited in the handoff §4 to confirm the docs match the code (e.g. `DEFAULT_SCHEMA_EPOCH = 11` in `db/sqlite.rs:96`; the two-lane enum at `main.rs:746-749`; `COMMERCIAL_SWEEP_CRON = "0 0 7 * * *"` at `scheduler/cron.rs:27`).

---

## 2. Contradiction Hunt (specific terms — grep the whole `docs/strategy/` tree)

For each concept below, find **every** statement across all docs and verify they agree:

- **Scheduling trigger** — search: `at startup`, `at process start`, `at bootstrap`, `quit and reopen`, `restart`, `manual CLI trigger only`. PASS = all docs say the running daemon re-evaluates schedules continuously via the `autonomous-scheduler-tick` thread (~30s). The only legitimate survivor: L1 stating cron *itself* is an L3 capability (L1 doesn't own cron) — but no doc may imply a *running* daemon can't tick.
- **Worker** — search: `run_next_workflow_job`, `3 kinds`/`three kinds`, `--run-next-job`, `orphaned`, `unsupported queued job kind`. PASS = always-on two-lane worker routes all cron kinds; `--run-next-job` is a one-shot drainer, not the only worker; `enrich_process` is the *only* kind still orphaned.
- **Schema epoch** — search: `epoch 10`, `epoch is 10`, `EXPECTED_SCHEMA_EPOCH`, `DEFAULT_SCHEMA_EPOCH`, `0014`. PASS = epoch **11**, migration **0015**, daemon and desktop together. No "highest migration 0014" survivor.
- **Commercial stages** — search: `re-run pipeline`, `trigger_pipeline_rerun`, `commercial_sweep`. PASS = `commercial_sweep` (07:00, in-process, coverage/deterministic lane) is the autonomous trigger; `trigger_pipeline_rerun` is manual-only and never the autonomous mechanism.
- **Desktop buttons** — search: `trigger_ingestion`, `trigger_enrichment`. PASS = `trigger_ingestion` functional; `trigger_enrichment` orphaned. No doc claims the latter works.
- **Shadow mode** — search: `shadow`, `recorded_only`, `enforcement`. PASS = `source_health` + `auth_refresh` run shadow/record-only via a **compile-time const** (not a config flag); validation + commercial_sweep are **not** gated.
- **ProContract resumption** — search: `procontract_pending_enrichment`, `awaiting_access`, `Recently added`, `maxPages`, `interest_registration`. PASS = dedicated table; five lifecycle transitions; `interest_registration.state='completed'` invariant stated; staleness keys off notice `deadline_at` else 90-day max-age, **never** the RFx response deadline; enrich work-list sweep makes resumption independent of "Recently added"/`maxPages`.

---

## 3. Doc Boundary Sanity

- **Concept ownership** is correct: continuous tick + two-lane worker + job-kind routing + shadow mode + ProContract awaiting-access live in **L3**; live-loop refactor + serve-loop-hosts-threads in **L1**; `workflow_jobs` enum + schema matrix + epoch doctrine in **L0**; desktop buttons + autonomy panel in **L2**; lease + epoch guard in **L4**; commercial-stage *pointer* (not mechanism) in **L5**.
- **No new layer** was created. The capability is woven into existing L0–L6; there is no L7.
- **Cross-references** are correct and bidirectional (L5 → L3 commercial_sweep mechanism; cross-ref-index rows point at real sections; L6 ingestion-side resumption distinguished from bid-side `AUTH_STATE_STALE`).
- **Authority split** preserved: layer-strategy.md routes/summarizes, per-layer docs hold detail, cross-reference-index.md indexes. No detail dumped into the wrong tier.

---

## 4. Implementation Readiness

Could an engineer implement (or correctly modify) the system from only the docs?
- Are the tick interval/seam, the two lanes' kind-partition, defer-on-denial semantics, and the `commercial_sweep` schedule unambiguous?
- Is the `procontract_pending_enrichment` schema (columns, CHECK states, UNIQUE, lifecycle triggers) specified well enough to rebuild it, including the `interest_registration.state='completed'` invariant?
- Could two engineers reading different docs derive **compatible** models of: when schedules fire, which lane runs which kind, what advances the commercial stages, and what epoch the schema is at? If any doc could mislead, fix it.

---

## 5. Overreach Check

Verify the harmonization did **not**:
- Add implementation sequencing / bead IDs / "first implement X then Y" / dev-tool instructions.
- Append "Autonomous Scheduling" sections instead of integrating into existing sections.
- Over-specify still-open design choices as settled (there are none left open — but don't invent precision the code doesn't have, e.g. don't document a config flag for shadow mode that doesn't exist).
- Churn unaffected commercial/customs/validation-internals content.

---

## 6. Absorption Completeness (research doc → redundant?)

Verify **every** concept from the research doc has a home in the strategy docs and nothing was silently dropped:
- §4.1 scheduler tick (interval, seam, generation-counter reload, one-shot entrypoint, Q2 fold) ✓ in L3 / layer-strategy?
- §4.2 two-lane worker (lanes, idle backoff, defer-on-denial, Q1 own-conn-per-lane) ✓?
- §4.3 live-loop refactor (`SyncSourceRoute::from_key`, per-source param derivation) ✓ in L1?
- §4.4 ProContract lifecycle (all 5 transitions, Q4 table, Q5 staleness, skill `awaitingAccessTargets`) ✓ in L3?
- §4.5 commercial_sweep (07:00, in-process, not `trigger_pipeline_rerun`, Q8 no-nudge) ✓ in L3 + L5 pointer?
- §5 job-kind routing table (incl. `enrich_process` still-orphaned) ✓ in L0/layer-strategy/cross-ref?
- §6 concurrency model (R7 source-lane serialization, R8 lease-gates-push-only, epoch 11/R6) ✓ in L3/L4?
- Shadow mode (Q6) ✓? Desktop surfacing (Q7) ✓ in L2?

If an engineer would still need to open the research doc to understand any of the above, that's an absorption gap — fix it.

---

## 7. Terminology Consistency

All docs use identical names: `autonomous-scheduler-tick` thread, `AutonomousWorkerLane` (PtyBrowser / CoverageDeterministic — or the docs' chosen prose for "PTY/browser lane" vs "coverage/deterministic lane", used uniformly), `commercial_sweep`, `procontract_pending_enrichment`, `awaiting_access`/`enriched`/`expired` states, `CronStatusSnapshot`, `--run-scheduler-once`. No doc uses an older term (e.g. "scheduler loop", "poll loop", "pending enrichment queue") for these.

---

## 8. Fix and Final Fresh-Eyes Pass

Fix all findings with **targeted** edits (not wholesale rewrites). Then re-read everything ONE more time and apply this test:

> Imagine you are the engineer maintaining this daemon and the strategy docs are the only thing you get. Could you still misread anything about: **when schedules fire, which lane runs which job kind, what advances the commercial stages, how ProContract resumption avoids re-expressing interest, or what schema epoch the system is at?**

Fix anything that fails this test.

**Required output:** a short report listing (a) docs changed in this pass, (b) contradictions removed, (c) absorption gaps closed, (d) any residual ambiguity you could not resolve and why.

---

## 9. Follow-Up Pass (built-in fresh-eyes — send as a SECOND message in the same session)

> Send this after the optimization above is complete.

One more focused sweep on the **three drift points** most likely to be papered over wrong, plus the highest-value invariants:

1. **`enrich_process` / `trigger_enrichment`** — confirm no doc anywhere claims this button/kind works. It is the single intentionally-orphaned kind.
2. **Shadow mode** — confirm every mention says code-level record-only (handlers run + persist side effects, enforcement not applied), never a runtime/config rollout flag, and never applied to validation or `commercial_sweep`.
3. **`trigger_pipeline_rerun`** — confirm it is described only as a manual-only desktop daemon-stop affordance, never as the autonomous commercial trigger.
4. **Epoch 11** consistency on **both** daemon (`DEFAULT_SCHEMA_EPOCH`) and desktop (`EXPECTED_SCHEMA_EPOCH`) sides, everywhere.
5. **ProContract landmine** — confirm the `interest_registration.state='completed'` invariant (why awaiting-access uses a dedicated table / anti-join, not a state flip) is stated wherever the table appears.

Fix any survivor. Confirm the research doc is now genuinely redundant — an engineer never needs it to understand the implemented system.
