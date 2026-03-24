# Handoff: L3+L4 Validation Pipeline & Sync Engine

## Expert Framing

You are a **distributed systems engineer and ML operations specialist** with deep experience building data validation pipelines, eventually-consistent sync protocols, and production quality gates for AI-generated output. You've operated systems where "the model said it was right" isn't good enough — you've built semantic validators, calibration harnesses, and fail-open enforcement policies. You understand the engineering cost of correctness and know when to invest in validation vs. when to let things through.

Draw from: Transactional outbox pattern (Debezium, CDC), CAS (compare-and-swap) for optimistic concurrency, ML model monitoring (drift detection, calibration curves, precision/recall at threshold), RLS (Row-Level Security) in Postgres, eventually-consistent replication protocols, and fail-open vs fail-closed safety systems (circuit breakers, feature flags).

---

## Mission

ULTRATHINK and be meticulous. You are reviewing **Layers 3 and 4** of a 7-layer strategy for a UK public procurement intelligence platform.

**Layer 3 (Operational Hardening)** introduces:
- Cron scheduler with explicit skill binding
- Resource arbiter (typed quotas: browser_slots, agent_slots, source_lanes)
- PTY hardening (fd hygiene, EIO handling, terminal geometry)
- Relation resolution engine (4 classifications: merged_same_process, related_not_merged, possible_duplicate, separate)
- Source health monitoring with circuit breaker (CLOSED → HALF_OPEN → OPEN)
- **Worker-validator system in dry-run mode** — validation policy engine, validation executor, base validator skill, initial calibration with 3 corpora
- Skill promotion flow (6 stages: draft → candidate → canary → shadow → stable → rollback)
- 12 new Rust modules, 10 new skills, 5 new tables

**Layer 4 (Shared Visibility)** introduces:
- **Full sync engine** — transactional outbox push, cursor-based pull, CAS for operator decisions
- Active-writer lease (one daemon per org)
- Schema epoch guard (incompatible epoch → sync pauses)
- Supabase/Postgres schema (18 projection tables + 5 summary tables + 3 remote-owned tables)
- RLS policy matrix (4 roles: viewer, operator, admin, daemon)
- Web viewing portal (Next.js + Supabase, read-only)
- **Validation enforcement mode** — transition from dry-run to production blocking
- 4 domain-specific validator overlays
- Formal calibration harness (3 corpora, Wilson-bound precision gates)

Don't just answer my questions — **the questions I DON'T KNOW to ask are more important than the ones I do.**

---

## User's Strong Beliefs (Engage With These)

1. **"Fail-open validation is constitutional."** Validator failures never block the pipeline. If the validator crashes, times out, or produces garbage, the pipeline continues and the result is flagged as unverified. The user believes the cost of blocking good data is always higher than the cost of letting some bad data through. **Challenge this. When is fail-open dangerous?**

2. **"Local SQLite is the operational authority."** The sync engine replicates TO Supabase (push) and pulls config FROM Supabase (pull). If sync breaks, the desktop continues full local operation. The web portal degrades gracefully. The user believes this makes the system resilient. **What failure modes does this miss?**

3. **"Per-slice enforcement readiness."** Validation enforcement is not a global switch — it's per worker-skill × source × worker-version × validator-overlay-version. Each "slice" must independently prove calibration (minimum adjudicated cases, Wilson-bound precision) before enforcement activates for that slice. **Is this practical? Is the granularity right?**

---

## Essential Context

### What This Project Is

The daemon orchestrates AI agent "skills" that scrape procurement portals and marketplaces. Skills produce structured output (product candidates, supplier quotes, landed-cost estimates). The worker-validator system adds a second AI agent that reviews what the first agent produced — checking the run manifest (what the agent says it did) against machine-observed evidence (what the daemon saw it actually do).

### Key Terms

- **Worker-validator:** Two-agent pattern. Worker executes the skill. Validator reviews the worker's output. Validator produces a verdict (approved/rejected/flagged).
- **Run manifest:** Hybrid-authored artifact. Agent writes narrative sections. Daemon appends machine-observed evidence appendix with `agent_claimed_*` vs `daemon_observed_*` fields.
- **Validation policy engine:** Decides whether to validate a given job: always, sample (risk-stratified rate), trigger (condition-based), skip.
- **Risk-stratified sampling:** Not uniform random. Jobs with risk factors (new skill version, source health amber/red, unknown fingerprint, high-value opportunity) are sampled at higher rates.
- **Per-slice enforcement readiness:** Enforcement activated independently for each (worker_skill, source, worker_version, validator_overlay_version) tuple. Requires minimum calibration targets met.
- **Basis sufficiency:** Validator can say "I don't have enough information to judge" — a third category beyond approve/reject.
- **Fail-open:** Validator failure → pipeline continues. Result marked as `VALIDATION_SKIPPED_VALIDATOR_ERROR`.
- **Transactional outbox:** Every push-owned mutation creates an outbox entry in the same SQLite transaction. Push dispatcher sends batches to Supabase.
- **CAS (Compare-and-Swap):** Operator decisions use optimistic concurrency. `update_operator_decision(process_id, disposition, base_revision, command_id)` — detects conflicts, idempotent retries.
- **Active-writer lease:** Only one daemon instance per org can push to Supabase at a time.
- **Schema epoch:** Integer version. If local and remote epochs disagree, sync pauses. Local operation continues uninterrupted.
- **RLS:** Row-Level Security in Postgres. Different roles see different data.

### Technical Constraints

- SQLite local, Supabase/Postgres remote
- Push-only for pipeline data (local → remote)
- Pull-only for config/decisions (remote → local)
- No mixed-write tables
- Validation sub-jobs use same adapter/lifecycle/cleanup as worker jobs
- Fail-open is non-negotiable
- One daemon per org for MVP (active-writer lease)

---

## Current Architecture

### Validation System (L3 Dry-Run → L4 Enforcement)

**Policy engine (L3):**
```
For each completed worker job:
  1. Look up validation_mode for this skill: always | sample | trigger | skip
  2. If sample: compute risk score from 8 factors:
     - source_health amber/red (force validate)
     - unknown page fingerprint (force validate)
     - first run after skill version change (force validate)
     - continuation chain (force validate at chain close)
     - high fallback depth in results (boost rate)
     - high-value opportunity (boost rate)
     - suspect output in recent runs for this skill+source (force validate)
     - pre-stable skill stage (force validate)
  3. If risk score triggers validation: spawn validator sub-job
```

**Validator execution (L3):**
- Spawn validator skill as daemon sub-job (same lifecycle, same cleanup)
- Inject: run manifest, OQG anomaly signals, distilled rubric (validator-rubric.yaml), anomaly-stratified result sample, structured drift deltas
- Validator produces: verdict (approved/rejected/flagged), confidence_bps, summary, basis_sufficiency (high/medium/low), anchor_conflict_count, incident_signature, enforcement_eligible

**Dry-run mode (L3):**
- Verdicts logged to `validation_verdicts` table
- No pipeline rows blocked
- No outbox entries from verdicts
- No stage heads moved based on verdicts
- Purpose: calibrate validator accuracy before enforcement

**Enforcement mode (L4):**
- Transition requires: minimum adjudicated cases per slice, Wilson-bound precision ≥70%, false-pass rate <5%
- Rejected results quarantined as `AGENT_SUSPECT_OUTPUT`
- Per-slice: enforcement can be active for eBay-product-extraction-v2.1-validator-v1.0 but dry-run for alibaba-product-extraction-v1.0-validator-v1.0
- Automatic demotion: override rate spike or performance regression → revert to dry-run
- Override feedback loop: every operator override becomes calibration case

**4 domain-specific validator overlays (L4):**
1. Procurement-ingestion: OCDS compliance, date validity, locale, CPV consistency
2. Supplier-enrichment: price plausibility, currency match, seller diversity, compliance evidence
3. Document-acquisition: type identification, download completeness, content verification
4. Product-extraction: candidate type, archetype, slot completeness, bundle decomposition

### Sync Engine (L4)

**Push protocol:**
```
On every push-owned INSERT/UPDATE in SQLite:
  1. In same transaction: write data row + create local_outbox entry
  2. Push dispatcher (background): read outbox → batch → send to Supabase
  3. Supabase RPC: apply_push_batch(batch_json) — single transactional RPC
  4. On success: mark outbox entries as sent
  5. On conflict: reject batch (row_version mismatch or epoch mismatch)
```

**Push dispatcher contract:**
- Single in-flight batch at a time
- Identical-envelope retries (if network fails, resend same batch)
- Batch boundaries: never cross stage_run_id, never cross schema_epoch
- Monotonic local_seq ordering within batch

**Pull protocol:**
```
On daemon startup and periodically:
  1. Read remote_change_seq cursor for each pull domain
  2. Query Supabase for rows with change_seq > cursor
  3. Apply to local SQLite (upsert based on primary key)
  4. Advance cursor
```

**Pull-before-push ordering:** On startup: pull remote-owned data FIRST, then coalesce pending operator intents, then resume push. This prevents stale local state from overriding remote decisions.

**CAS for operator decisions:**
```
update_operator_decision(process_id, disposition, base_revision, command_id)
  IF base_revision matches current → apply, increment revision
  ELSE → return conflict (operator must refresh and re-decide)
  command_id makes retries idempotent
```

**Active-writer lease:** One daemon per org. Enforced at push protocol level. If two daemons try to push, second gets rejected. MVP simplification — multi-daemon not needed yet.

**Schema epoch guard:**
- Both SQLite and Supabase store schema_epoch (integer)
- If they disagree: sync pauses, local operation continues
- Resolution: daemon upgrade → new epoch → sync resumes
- Outbox entries with old epoch are never sent

**Degraded-mode behavior:**
- **Healthy (Green):** <90s since last sync. Full real-time updates.
- **Degraded (Amber):** 90s–5min. Data is behind. Broadcast still connected.
- **Offline (Red):** >5min or daemon offline. Cached data only.

### Supabase Schema (L4)

**18 push-owned projection tables** (mirror SQLite pipeline tables)
**5 push-owned summary tables** (redacted projections for web: workflow_job_summaries, error_manifest_summaries, validation_verdict_summaries, viewer_sync_status, executors)
**3 remote-owned tables** (daemon pulls, admins/operators write):
- shared_company_profile
- shared_source_configs
- operator_decision_heads (CAS-protected)

**RLS policy matrix:**
| Role | Push-Owned | Remote-Owned | Summary |
|------|-----------|-------------|---------|
| viewer | SELECT | — | SELECT |
| operator | SELECT | SELECT + UPDATE (CAS) | SELECT |
| admin | SELECT | Full CRUD | SELECT |
| daemon | INSERT/UPDATE (batch RPC) | SELECT | INSERT/UPDATE |

---

## Tensions to Resolve

### Tension 1: Validation Calibration Bootstrap Problem

Enforcement requires calibration data: >=30 adjudicated rejects, >=60 audited approves, 70% precision lower bound, <5% false-pass upper bound. But this data comes from dry-run mode (L3), which requires production traffic (L2 operating successfully).

- **Chicken-and-egg:** You need production data to calibrate validators. You need calibrated validators to enforce quality. But the data you're calibrating on was produced WITHOUT enforcement — so it includes bad data. How does the calibration account for this?
- **Corpus bootstrapping:** The "mutated-bad" corpus (30-50 cases) must be realistic. But mutations require understanding what "bad" looks like in production. At L3, you haven't seen enough production to know.
- **Per-slice granularity:** Each (skill × source × version × overlay) tuple needs independent calibration. With 5 initial skills × 1-4 sources × versions, the number of slices can be large. Most will have insufficient data for calibration.
- **Stress test:** L3 dry-run runs for 2 weeks. Skill `product-extraction` on eBay has 200 runs, 15 validations (sample rate ~7.5%). Of those 15, 3 are rejected. Is that enough to compute Wilson-bound precision? (Answer: probably not.) When can enforcement realistically activate?

### Tension 2: Sync Protocol Correctness Under Adversity

The push outbox + pull cursor + CAS protocol is a well-known pattern. But the devil is in the details:

- **Outbox ordering vs. stage_run_id:** Outbox entries are ordered by local_seq. But pipeline stages may complete out of order (lot extraction finishes before qualification re-run). If the push dispatcher sends lot-extraction results before the re-qualification result, the remote view is temporarily inconsistent. Is this acceptable?
- **Pull-before-push race:** On startup, daemon pulls remote state, then resumes push. But what if remote state changed DURING the pull? (New operator decision arrived while daemon was processing the pull.) Is the pull atomic? If not, what's the consistency window?
- **CAS command_id uniqueness:** The CAS RPC uses command_id for idempotency. But who generates the command_id? The desktop (user action)? The daemon (automatic)? If the desktop crashes between generating command_id and receiving ACK, is the decision applied?
- **Batch boundary correctness:** Batches never cross stage_run_id. But a single pipeline run may update multiple tables (product_candidates + pipeline_assessments + process_stage_heads). Do all of these go in the same batch? If the batch is too large for a single RPC, how is it split without violating the stage_run_id boundary?

### Tension 3: Fail-Open Cascade Risk

Fail-open validation means validator failures don't block the pipeline. But:

- **Systematic validator failure:** If the validator skill itself is broken (bad prompt, API key expired, model downgraded), ALL validations fail silently. The pipeline runs without any quality gate for potentially hours/days until someone notices.
- **Validator health monitoring:** The doc mentions "health monitored separately" but doesn't specify how. What checks? What alerting? What's the SLA for noticing a broken validator?
- **Trust-state propagation:** Unvalidated results carry `validation_state = 'unverified'`. But downstream stages process unverified results identically to verified ones. The unverified flag is only informational (for the operator). Is this sufficient?
- **Stress test:** Validator skill breaks at 2 AM. Overnight batch produces 500 results, all unverified. Morning operator sees 500 results with 'unverified' badges. How do they decide which to trust? Is there a mechanism to retroactively validate once the validator is fixed?

### Tension 4: Web Portal Read-Only Guarantee

The web portal is explicitly read-only (ADR-008). But:

- **Operator decisions via web?** L4 introduces `operator_decision_heads` as remote-owned with CAS. The operator updates this from the desktop via DaemonBridge → daemon → push to Supabase. But what about an operator who wants to review and decide from the web portal? The web has Supabase access. Can't the operator call the CAS RPC directly from the web?
- **Current answer:** The doc says no — web is read-only. Operator decisions only via desktop.
- **But the architecture enables it:** The RLS role `operator` has UPDATE permission on `operator_decision_heads` via CAS RPC. If a web user has operator role credentials, the web COULD call the RPC. Is the "read-only web" a UI constraint or a security constraint?
- **Stress test:** Operator is traveling, only has web access. Sees a time-sensitive bid opportunity. Can't approve from web. Must wait to get to desktop. Is this the right tradeoff?

### Tension 5: Relation Resolution Conservatism vs. Accuracy

L3 introduces relation resolution (merging notices into processes). The scoring is weighted: 0.35 ref match + 0.20 buyer + 0.20 title + 0.10 CPV + 0.10 date + 0.05 value. Conservative thresholds prevent false merges.

- **But false non-merges are also costly:** Two notices for the same procurement, not merged, get enriched independently. Operator sees duplicate opportunities with different data. Confusion, wasted effort, inconsistent decisions.
- **Heuristic weights:** Are the weights right? Why 0.35 for ref match and only 0.05 for value? A framework call-off might share a buyer and CPV but have different references. Would this fail to merge?
- **Late merge problem:** If relation resolution runs AFTER enrichment (re-run on updated data), merging two previously-separate processes requires reconciling their enrichment state (different product candidates, different quotes). This is identified as an open question ("processClusterId"). But it's not just open — it's architecturally significant.

---

## What I Don't Want (Anti-Patterns)

- **Don't propose fail-closed validation.** Fail-open is constitutional. The question is how to make fail-open safe, not whether to switch to fail-closed.
- **Don't suggest replacing the transactional outbox with CDC/WAL streaming.** The outbox pattern is chosen for its simplicity and SQLite compatibility.
- **Don't propose multi-master replication.** One daemon per org, one direction of push, is the design.
- **Don't recommend Kafka/RabbitMQ for the sync protocol.** The system uses direct RPC.
- **Don't suggest removing the per-slice enforcement granularity.** The question is whether it's practical, not whether to simplify to global enforcement.

---

## What I Do Want

1. **Validation calibration timeline:** Concrete math on how long calibration takes. Given realistic traffic volumes, when can enforcement activate? What's the minimum viable calibration?

2. **Sync protocol correctness analysis:** Walk through the outbox + pull + CAS protocol and identify correctness gaps. Propose fixes for any ordering, atomicity, or idempotency issues.

3. **Fail-open safety mechanisms:** How to make fail-open validation safe. What monitoring, alerting, and retroactive validation mechanisms are needed?

4. **Web portal decision-making story:** Should the web portal remain strictly read-only? Or should operator decisions be possible from web with appropriate authentication? What are the security implications?

5. **Relation resolution improvement:** Better heuristics? Different weights? Late-merge reconciliation strategy?

6. **Ideas I haven't asked about.** What distributed systems concerns am I missing?

Output your review as structured analysis with specific, actionable recommendations.

---

## Context Docs to Attach

1. **`docs/strategy/layers/layer-3-operational-hardening.md`** (~900 lines) — Validation system, relation resolution, source health, skill promotion. Read cover to cover.
2. **`docs/strategy/layers/layer-4-shared-visibility.md`** (~800 lines) — Sync engine, Supabase schema, web portal, validation enforcement. Read cover to cover.
3. **`docs/strategy/layer-strategy.md`** (~2100 lines) — Cross-layer contracts, testing evolution. Read §§10-11.

# Layer Strategy — UK Public Procurement Intelligence Platform

> **Authority:** This document is the authoritative cross-layer strategy for the sequentially-shippable layer decomposition of the procurement intelligence platform. It supersedes ad-hoc milestone references elsewhere.
>
> **Source:** Derived from the 8,400-line research document (`docs/research/2026-03-08-public-procurement-intelligence-platform-research.md`) through structured investigation of all major subsystems, dependency mapping, and synthesis.
>
> **Layers:** 7 layers (0–6), each independently deployable, testable, and incrementally valuable.

---

## Glossary

Terms used throughout this document:

| Term | Definition |
|------|-----------|
| **PTY** | Pseudo-terminal — a Unix facility that emulates a terminal, enabling the daemon to run browser-automation agents as if they were typing in a real terminal. Required for interactive flows (login, CAPTCHAs, multi-step forms). |
| **Contract mode** | Non-interactive structured execution (JSON/JSONL I/O) — used for deterministic tasks that don't need a browser. Lighter than PTY. |
| **Push-owned** | A table whose authoritative copy lives in local SQLite, written by the daemon. A projection is pushed to Supabase for web viewing. The daemon is the authority; Supabase is a read-only mirror. |
| **Remote-owned** | A table whose authoritative copy lives in Supabase (e.g., operator decisions, shared config). The daemon pulls a local cache. |
| **CAS** | Compare-And-Swap — a concurrency pattern where an update includes the expected current version. If the version doesn't match (someone else changed it), the update fails rather than silently overwriting. Used for operator decisions. |
| **OQG** | Output Quality Gate — code-based checks (schema validation, statistical anomaly detection, drift detection) that run on every skill result before the semantic validator. |
| **RLS** | Row-Level Security — Postgres/Supabase feature that restricts which rows each database role can read/write. |
| **UDS** | Unix Domain Socket — local inter-process communication transport used between the desktop app and the daemon. |
| **FTS** | Find a Tender Service — UK government procurement notice portal. |
| **CF** | Contracts Finder — UK government procurement notice portal for lower-value contracts. |
| **PCS** | Public Contracts Scotland — Scottish procurement portal. |
| **OCDS** | Open Contracting Data Standard — structured format for procurement data used by UK government portals. |
| **`stage_run_id`** | An identifier for one atomic commit cohort within a pipeline stage. All outputs from one execution of one stage at one scope grain share a `stage_run_id`. Used for sync batch boundaries and rollback. |
| **`matchArchetype`** | Classification of how a product candidate's identity is recovered on marketplaces (e.g., `standard_class_led` for products found via standards, `dimension_material_led` for products found via physical specs, `model_exact` for branded products found by model number). Drives query strategy in marketplace search. |
| **Skill** | A versioned directory containing a Markdown playbook (SKILL.md), schemas, and test fixtures that tells an AI agent how to perform a specific task (e.g., scrape a procurement portal, extract products from tender documents). Skills are passive assets — the daemon orchestrates when and how they run. |
| **QueryService** | A Tauri-side Rust component that reads local SQLite directly (read-only) and returns typed `QueryEnvelope<T>` responses to the desktop frontend. No network dependency. |
| **DaemonBridge** | A Tauri-side Rust component that forwards desktop commands (mutations, triggers) to the background daemon process over UDS/named-pipe. Returns `MutationAck` responses. |
| **Channel\<T\>** | Tauri's typed streaming mechanism — used for live session output from the daemon to the desktop frontend (e.g., real-time terminal output from a running agent). |
| **Run manifest** | A structured Markdown document produced after every skill execution. Contains an agent-written narrative ("what I did") and a daemon-appended evidence appendix ("what the machine observed"). Used for debugging, operator review, and semantic validation. |

---

## Table of Contents

1. [Design Rationale](#1-design-rationale)
2. [Layer 0 — Gold Set + Evaluation Harness](#2-layer-0--gold-set--evaluation-harness)
3. [Layer 1 — Thin Local Runner](#3-layer-1--thin-local-runner)
4. [Layer 2 — Single-Operator Desktop MVP](#4-layer-2--single-operator-desktop-mvp)
5. [Layer 3 — Operational Hardening](#5-layer-3--operational-hardening)
6. [Layer 4 — Shared Visibility](#6-layer-4--shared-visibility)
7. [Layer 5 — Expanded Commercial Intelligence](#7-layer-5--expanded-commercial-intelligence)
8. [Layer 6 — Request Pipeline Automation](#8-layer-6--request-pipeline-automation)
9. [Cross-Layer: Schema Stability Matrix](#9-cross-layer-schema-stability-matrix)
10. [Cross-Layer: Interface Contracts](#10-cross-layer-interface-contracts)
11. [Cross-Layer: Testing Evolution](#11-cross-layer-testing-evolution)
12. [Cross-Layer: Milestone Mapping](#12-cross-layer-milestone-mapping)
13. [Cross-Layer: Stack Sequencing](#13-cross-layer-stack-sequencing)

---

## 1. Design Rationale

### Why These Boundaries

The layer decomposition follows three ordering principles from the research document's validation-first philosophy:

1. **Risk retirement, not architectural completeness.** The riskiest assumptions live in the intelligence pipeline — can the system extract product candidates, match them to listings, and produce margin estimates? Infrastructure is well-understood engineering that follows, not leads.

2. **Vertical capability slices, not horizontal stack layers.** Each layer delivers an end-to-end flow, not "all Rust components" or "all TypeScript." The first pipeline layer completes the full operational cycle from notice ingestion through verifiable output.

3. **Schema from day 1, features incrementally.** Sync tracking columns (`owner_id`, `row_version`, `last_local_seq`, `stage_run_id`) are present on all push-owned tables from their first creation in Layer 0. Validation fields (`parent_job_id`, `validation_required`, `validation_verdict_id`) exist as nullable columns on `workflow_jobs` from Layer 0. All columns needed in any future layer exist as nullable from table creation. The sync ENGINE and validation ENFORCEMENT are deferred — the schema provisions are not.

### Relationship to Research Document Milestones

The 7 layers map directly to the research document's M0–M6 milestones (§11). Deviations are minimal and justified inline. The research doc's milestone structure was validated through 3 planmaxer expert review iterations and 2 harmonymaxer feedback integrations, making it a reliable starting hypothesis that required no major reordering.

### Implementation Order Snapshot

If an implementation team starts tomorrow, the build order is:

1. **Layer 0 first**: establish the gold set, evaluation harness, screening-grade pipeline primitives, and schema skeleton.
2. **Layer 1 second**: make the first live FTS loop real under the thin runner and prove `ingest → process → verifiable local output`.
3. **Layer 2 third**: add the single-operator desktop surface and Contracts Finder so a real operator can use the system daily.
4. **Layer 3 fourth**: automate and harden what Layer 2 already proved useful.
5. **Layer 4 fifth**: activate sync, remote-owned shared state, and the read-only viewing portal.
6. **Layer 5 sixth**: deepen commercial intelligence quality, not platform breadth.
7. **Layer 6 last**: automate the request pipeline only after the intelligence stack is already trusted.

Stop/go rule between layers:

- Do not start Layer 1 until Layer 0 passes the gold-set experiments.
- Do not start Layer 2 until Layer 1 proves the first live loop and runtime reliability.
- Do not start Layer 3 until Layer 2 proves single-operator value on live data.
- Do not start Layer 4 until Layer 3 proves unattended local operation.
- Do not start Layer 6 until Layer 5 proves decision-grade commercial intelligence on validated categories.

### Non-Negotiable Constraints Respected in Every Layer

These constraints from the research document apply to ALL layers without exception:

| Constraint | Source | Enforcement |
|-----------|--------|-------------|
| Authority matrix — every table classified, no mixed writes | ADR-002, §4.7 | Schema DDL enforces classification at table creation |
| Integer pence for money (`_pence` suffix) | §5.4 | Type system + linting |
| Integer basis points for confidence/percentage (`_bps` suffix, 0–10000) | §5.4 | Type system + linting |
| Process-group supervision with cleanup verdicts | ADR-004, §4.9 | From Layer 1 (first agent execution) |
| Run manifests for every skill execution | §4.10 | From Layer 1 (fallback machine-authored if agent fails) |
| Error manifests for every pipeline failure | §5.11 | From Layer 1 |
| Skill metadata declarations (SKILL.md + schemas) | §4.10, ADR-005 | From Layer 0 (first skill authored) |
| Conservative customs defaults (MFN, residual, standard VAT) | ADR-006, §4.5 | From Layer 0 (screening-grade) |
| No real-money bids without human approval | ADR-007 | All layers |
| Sync tracking columns on push-owned tables from creation | §5.4 | Schema DDL |
| `pipeline_assessments.validation_state = 'unverified'` from introduction | §5.4 | Schema DDL |

### Maintained Doc Set

The maintained planning set is exactly **8 files**. No companion docs, ADR addenda, or side decision files outside this set are authoritative for layer decomposition.

| File | Authority |
|------|-----------|
| `docs/strategy/layer-strategy.md` | Cross-layer source: layer ordering, schema stability matrix, interface contracts, stage-scope matrix, testing evolution, maintained-doc-set policy |
| `docs/strategy/layers/layer-0-gold-set-evaluation.md` | Layer-0-local narrative, scope, acceptance criteria, risks, and open questions |
| `docs/strategy/layers/layer-1-thin-local-runner.md` | Layer-1-local narrative, scope, acceptance criteria, risks, and open questions |
| `docs/strategy/layers/layer-2-single-operator-desktop.md` | Layer-2-local narrative, scope, acceptance criteria, risks, and open questions |
| `docs/strategy/layers/layer-3-operational-hardening.md` | Layer-3-local narrative, scope, acceptance criteria, risks, and open questions |
| `docs/strategy/layers/layer-4-shared-visibility.md` | Layer-4-local narrative, scope, acceptance criteria, risks, and open questions |
| `docs/strategy/layers/layer-5-expanded-commercial-intelligence.md` | Layer-5-local narrative, scope, acceptance criteria, risks, and open questions |
| `docs/strategy/layers/layer-6-request-pipeline-automation.md` | Layer-6-local narrative, scope, acceptance criteria, risks, and open questions |

Authority split inside the maintained set is fixed:

- `docs/strategy/layer-strategy.md` owns all cross-layer material: schema stability matrix, interface contracts, stage-scope matrix, testing evolution, stack sequencing, and doc-set policy.
- Each layer doc owns only layer-local narrative, capability, components, data first activated in that layer, stubs, definition of done, risks, and open questions.
- Open questions must live in the owning layer doc, not in a separate scratch file or review artifact.
- Temporary prompt artifacts, review handoffs, and iteration notes are explicitly non-authoritative and must never become de facto companion docs.
- `docs/strategy/README.md` and `docs/strategy/cross-reference-index.md` are **supporting navigation aids**, not part of the 8 authoritative planning files. They provide reading order and topic lookup but carry no planning authority.
- All 7 per-layer docs have been authored and reviewed. This strategy file now serves as the cross-layer reference; layer-local detail lives in the layer docs.

### Performance Targets

| Metric | Target | Owning Layer | Notes |
|--------|--------|-------------|-------|
| Daemon startup | <500ms | L1 | SQLite open + schema check + cron init |
| Incremental ingestion | <5min/source | L1 | Per-source agent session |
| Desktop detail page | <2s | L2 | From warm local SQLite cache via QueryService |
| Binary size (release) | <30MB | L1 | LTO + strip enabled (see Cargo release profile) |
| Idle memory (daemon) | <100MB | L1 | No leaks on long sessions |

### Per-Layer Documentation

All layer docs are complete. Each is self-contained for implementation.

| Layer | File | Lines | Key Content |
|-------|------|-------|-------------|
| 0 | `layers/layer-0-gold-set-evaluation.md` | ~3,900 | Full DDL (43 tables), 15-stage pipeline logic, skill standard, evaluation harness |
| 1 | `layers/layer-1-thin-local-runner.md` | ~1,300 | Daemon core, PTY/contract adapters, stage tracking model, run manifests |
| 2 | `layers/layer-2-single-operator-desktop.md` | ~700 | Desktop architecture, QueryService/DaemonBridge/Channel interfaces, coverage UI |
| 3 | `layers/layer-3-operational-hardening.md` | ~900 | Worker-validator dry-run, skill promotion, source health, relation resolution |
| 4 | `layers/layer-4-shared-visibility.md` | ~800 | Sync engine, Supabase schema/RLS, web portal, validation enforcement |
| 5 | `layers/layer-5-expanded-commercial-intelligence.md` | ~600 | Full customs engine, FX/freight, multi-source marketplace |
| 6 | `layers/layer-6-request-pipeline-automation.md` | ~400 | Bid workflow, portal-specific skills, audit trail |

### Atomic Operation Pairs (Never Split Across Layers)

| Pair | First Appears | Rationale |
|------|---------------|-----------|
| Skill execution + run manifest | Layer 1 | Losing forensic data from any skill run is permanent |
| Job execution + stage status update | Layer 1 | Pipeline state machine becomes inconsistent without stage tracking |
| Local data mutation + outbox entry | Layer 4 | From sync activation, every push-owned write must create outbox entry in same SQLite transaction |
| Stage output write + `stage_run_id` + stage head advancement | Layer 1 | All outputs of a pipeline stage share a `stage_run_id`; stage head advances atomically |

---

## 2. Layer 0 — Gold Set + Evaluation Harness

### Narrative

Prove the intelligence thesis on a curated lot corpus before building any live-source runtime. This layer is evaluation infrastructure first: gold-set data, harness logic, skill authoring standards, and the screening-grade TypeScript pipeline exercised against curated inputs. No daemon, no desktop, no sync, and no remote authority. Skills run from the command line for benchmark execution only. Results land in a harness-local SQLite dataset and are verified by the automated evaluation harness and manual inspection. Layer 0 deliberately does **not** count as the first live pipeline layer; it is the measurement system that Layer 1 must satisfy.

**If we never build Layer 1:** The gold set and evaluation harness remain valuable as a benchmarking tool and intelligence pipeline prototype. The skills and TypeScript intelligence code can run standalone indefinitely.

### Capability Delivered

A curated gold-set lot corpus (50–100 lots) runs through extraction → matching → screening-grade cost estimation, producing per-lot precision/recall metrics against human-labeled ground truth. Three experiments are validated:

1. **Extraction quality** — ≥80% of lots yield usable product candidates
2. **Matching quality** — ≥70% of candidates with `matchArchetype` produce genuine matches
3. **Margin accuracy** — screening-grade margin bands agree with human expert on ≥75% of lots (±5pp)

### Components Touched

| Component | Action | Files |
|-----------|--------|-------|
| Workspace bootstrap | Create | `package.json`, `bunfig.toml`, `.env.example` |
| `packages/contracts-schema` | Create | `contracts.ts`, `jobs.ts`, `confidence.ts`, `zod.ts` |
| `packages/intelligence` | Create | 13 files (qualification, extraction, search, customs screening, margins, evaluation harness) |
| Skills | Create | 5 skills (document-acquisition, lot-extraction, product-extraction, preliminary-qualification, supplier-price-enrichment/ebay) |
| Database | Create | `0001_sqlite_state_initial.sql` — full pipeline schema including dormant tables for future layers (all columns present as nullable from creation) |

### Data Introduced

**SQLite pipeline tables (full schema — dormant tables included for forward compatibility):**

| Table | Authority | Purpose | Sync Tracking Columns |
|-------|-----------|---------|----------------------|
| `procurement_processes` | Push-owned (future) | Canonical internal record | `owner_id`, `row_version`, `last_local_seq`, `stage_run_id` present from creation |
| `contract_notices` | Push-owned (future) | Raw notice data | Same sync columns |
| `notice_lots` | Push-owned (future) | Per-lot entities | Same sync columns |
| `product_candidates` | Push-owned (future) | Extracted candidates | Same sync columns |
| `search_profiles` | Push-owned (future) | Structured search definitions | Same sync columns |
| `search_attempts` | Push-owned (future) | Per-wave search results | Same sync columns |
| `supplier_quotes` | Push-owned (future) | Normalized pricing | Same sync columns |
| `candidate_pricing_assessments` | Push-owned (future) | Gross margin results | Same sync columns |
| `pipeline_assessments` | Push-owned (future) | Process-level rollup | Same sync columns |
| `qualification_runs` | Push-owned (future) | Qualification decisions | Same sync columns |
| `landed_cost_estimates` | Push-owned (future) | Landed-cost results (screening-grade initially; all decision-grade columns present as nullable from creation) | Same sync columns |
| `bid_workflow_states` | Push-owned (future) | Bid workflow progression (dormant until Layer 6; schema present from creation) | Same sync columns |
| `manifests` | Push-owned (future) | Opportunity manifests (dormant until Layer 1 first live end-to-end local loop) | Same sync columns |
| `process_stage_status` | Push-owned (future) | Per-stage execution status records (dormant until Layer 1 activates stage tracking) | Same sync columns |
| `process_stage_heads` | Push-owned (future) | Latest successful stage run per scope (dormant until Layer 1) | Same sync columns |
| `tender_document_sets` | Push-owned (future) | Document acquisition output (dormant until Layer 1 first live-source document acquisition) | Same sync columns |

**Local-only orchestration tables (also in initial schema, dormant until later layers):**

| Table | Authority | Purpose |
|-------|-----------|---------|
| `workflow_jobs` | Local-only | Job lifecycle tracking (dormant until Layer 1). Forward-compatibility columns exist from creation: `parent_job_id` (nullable), `validation_required` (NOT NULL default `false`), `validation_verdict_id` (nullable). |
| `pending_operator_intents` | Local-only | Local decision-intent ledger/queue. Introduced early so Layer 2 can persist approve/reject/watch actions without violating the remote ownership of `operator_decision_heads`. Carries the future CAS payload (`process_id`, disposition, reviewed-basis metadata, idempotency key, status). |

**Junction tables (introduced with parent tables):**

| Table | Authority | Purpose |
|-------|-----------|---------|
| `notice_cpv_codes` | Push-owned (future) | Notice ↔ CPV code mapping |
| `product_commodity_codes` | Push-owned (future) | Product ↔ commodity code mapping |
| `lot_cpv_codes` | Push-owned (future) | Lot ↔ CPV code mapping |

**Critical schema decisions enforced at creation:**
- All monetary fields use integer pence (`_pence` suffix)
- All confidence/percentage fields use integer basis points (`_bps` suffix)
- All push-owned tables include `owner_id`, `row_version`, `last_local_seq`, `stage_run_id`, `created_at`, `updated_at`
- **All columns needed in later layers exist as nullable from table creation** — later layers populate existing columns, they never add new columns to tables introduced in earlier layers
- `pipeline_assessments.validation_state` exists from creation with default `'unverified'`
- `workflow_jobs.validation_required` exists from creation with default `false`; `parent_job_id` and `validation_verdict_id` are present and nullable from creation
- `product_candidates` requires `lotId` (not optional)
- `landed_cost_estimates` includes all decision-grade fields (duty breakdown, measure interaction results, exporter rate basis, supplementary units, valuation mode) as nullable — populated when Layer 5 activates the full customs engine
- `pipeline_assessments` includes all overlay and coverage fields (`reviewRequired`, `availabilityStatus`, `pricedCoverageBps`, `landedCostCoverageBps`, `netQualifiedCoverageBps`, `unresolvedMandatoryLots`, `decisionBasisHash`) as nullable — populated when Layer 2 activates coverage tracking

### Authority Classification

Layer 0 tables have three authority classifications, all permanent from creation:

- **Push-owned** (15 pipeline data tables + 3 junction tables): Daemon-authoritative, projected to Supabase in Layer 4.
- **Local-only** (`workflow_jobs`, `pending_operator_intents`): Never authoritative in Supabase. A redacted summary may be projected later, but the local tables themselves remain SQLite-only.
- **Remote-owned** tables are not introduced in Layer 0. Pre-sync operator decisions are captured only as local intents; the remote authoritative `operator_decision_heads` table arrives in Layer 4 and is remote-owned from introduction.

### Shared Library Additions

- `packages/contracts-schema/src/contracts.ts` — All TypeScript interfaces for pipeline entities
- `packages/contracts-schema/src/jobs.ts` — Job payload types
- `packages/contracts-schema/src/confidence.ts` — Confidence scoring types
- `packages/contracts-schema/src/zod.ts` — Runtime validation schemas

### Invariants Enforced

| Invariant | Type | Enforcement |
|-----------|------|-------------|
| Integer pence for all monetary values | Constitutional | DDL + type system |
| Integer basis points for confidence/percentage | Constitutional | DDL + type system |
| Authority matrix classification on all tables | Constitutional | DDL |
| Sync tracking columns on push-owned tables | Constitutional | DDL |
| Skill metadata declarations (SKILL.md + schemas) | Constitutional | Skill authoring guide + linting |
| Conservative customs defaults | Constitutional | Screening-grade implementation |
| `lotId` required on product candidates | Constitutional | DDL NOT NULL |
| Forward-compatibility nullable columns on `workflow_jobs` | Constitutional | DDL |

### What's Stubbed

| Feature | Stub | Full Implementation |
|---------|------|-------------------|
| Landed-cost engine | Screening-grade: HMRC Tariff API + flat freight + single FX | Layer 5: multi-source recall, 10-digit resolution, measure scanner |
| Marketplace search | Single source (eBay) | Layer 5: Alibaba/1688, Amazon |
| Skills execution | CLI invocation (`bun run skill-name`) | Layer 1: daemon-managed PTY/contract execution |
| Relation resolution | Gold set is pre-curated (no deduplication) | Layer 3: full heuristic resolution |
| Procurement sources | Gold set is pre-selected | Layer 1: first live FTS loop; Layer 2: adds Contracts Finder + desktop operator surface |
| Stage tracking | Implicit (no daemon) | Layer 1: explicit `stage_run_id` + stage heads |
| Sync | No sync — local SQLite only | Layer 4: push/pull sync engine |

### Definition of Done

1. Gold-set lot corpus contains ≥50 curated lots across whitelist categories (office supplies, cleaning products, basic PPE, hand tools, basic furniture, packaging materials) with human labels for product candidates, expected matches, and margin bands
2. Bilingual lexicon seeded for top 30–50 product families (EN/ZH noun aliases, market framing terms)
3. All 5 skills have complete SKILL.md + result.schema.json + checkpoint.schema.json + canary.inputs.jsonl + golden.assertions.yaml
4. Evaluation harness runs full gold-set corpus through extraction → matching → screening-grade cost, produces per-lot precision/recall report
5. Experiment 1 passes: ≥80% of gold-set lots yield at least one usable product candidate with correct unit, quantity, and specification
6. Experiment 2 passes: ≥70% of candidates with `matchArchetype` produce at least one genuine match (not false positive)
7. Experiment 3 passes: margin bands agree with human expert on ≥75% of gold-set lots (±5pp for screening-grade)
8. All 4 unit test suites pass (goods qualification, quoteability, tariff parsing, margin rules)
9. SQLite schema validates: all push-owned tables have sync tracking columns, all monetary fields are integer pence, all confidence fields are integer basis points
10. Each skill's SKILL.md includes instructions for the agent to write a structured narrative of what it did (a "run manifest") as part of its output — this prepares for the formal daemon-managed manifest system in Layer 1

### Test Scenarios

- **Fast inner loop:** Extraction parsers, margin math, scoring functions, data model validation — per-save, near-zero cost
- **Benchmark batch:** Full gold-set corpus through pipeline, compare to human labels — per-milestone, medium cost (~$2–10)
- Gold-set lot with 12 lots where 11 are irrelevant → system correctly identifies the 1 qualifying lot
- Mixed contract with separable goods → `addressableValuePence` reflects goods portion only
- Candidate with `matchArchetype = unknown` → system does not fabricate an archetype
- Screening-grade landed cost uses conservative defaults (MFN rate, residual duty, standard VAT)

### Dependencies

- None (greenfield)

### Forward-Compatibility Provisions

- Sync tracking columns on all tables (for Layer 4)
- Nullable validation columns on `workflow_jobs` (for Layer 3)
- `stage_run_id` column on pipeline tables (for Layer 1 stage tracking)
- Skill directory structure follows production standard (for Layer 1 skill linting)
- `SearchProfile` created for ALL candidates including below-threshold (for Layer 2 RFQ re-entry)
- `pending_operator_intents` exists from the start so local-only operator decisions can precede remote CAS without any table reclassification

### Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Gold-set too small for statistical significance | Medium | Start with 50 lots, expand per category; lot-level measurement (not notice-level) |
| Screening-grade landed cost too inaccurate for margin estimation | Medium | Conservative defaults ensure overestimation, not underestimation |
| eBay listings don't represent real supplier economics | High | eBay is first validation source only; Alibaba/1688 added in Layer 5 |

### Decisions Addressed

- ADR-001: Skills as primary data retrieval (first 5 skills authored)
- ADR-005: Skill directory standard established
- ADR-006: Internal landed-cost engine (screening-grade subset)

---

## 3. Layer 1 — Thin Local Runner

### Narrative

Minimal Rust runtime — just enough to launch skills, capture artifacts, supervise processes, and run the first live-source end-to-end local pipeline loop. PTY is available as a first-class capability, with contract mode for deterministic tasks. This remains a thin daemon layer, but it is also the first live pipeline layer: a manual CLI trigger ingests live FTS notices, runs the full local qualification/pricing/manifest loop, and emits verifiable local output without any UI or sync dependency.

**If we never build Layer 2:** The thin runner provides a reliable way to execute skills from the command line with full process supervision, artifact capture, and error manifests. It operates as a headless pipeline executor.

### Capability Delivered

10 consecutive batch runs of skills complete with ≥90% skill success rate and no orphaned processes. Skills are now daemon-managed with proper lifecycle: spawn → monitor → capture → validate → persist → reap. This layer also completes the first live operational loop:

`notice ingestion → pipeline processing → verifiable local output`

The verifiable output is a CLI-visible summary plus a local markdown opportunity manifest/file artifact for at least one live-source batch.

### Components Touched

| Component | Action | Files |
|-----------|--------|-------|
| Skills | Create | 2 (`procurement-ingestion/fts`, `manifest-generation`) |
| Rust workspace | Create | `Cargo.toml` (workspace root) |
| `crates/daemon` | Create | 14 core files (main, db/sqlite, pty/session+capture+health, jobs/runner, agents/adapter+codex+claude+artifacts+contracts, prompt/renderer, errors/manifest+markdown) |
| Database | Create | `0003_sqlite_artifacts.sql` (agent_session_logs, raw_documents, error_manifests) |

### Data Introduced

**SQLite tables (artifacts DB):**

| Table | Authority | Purpose |
|-------|-----------|---------|
| `agent_session_logs` | Local-only | Full session transcript/trace with `run_manifest_path`, `run_manifest_hash` |
| `raw_documents` | Local-only | Downloaded tender documents |
| `error_manifests` | Local-only | Full error context for pipeline failures |
| `executors` | Push-owned | Daemon instance identity (`owner_id`, `executor_id`, heartbeat) |
| `source_cursors` | Local-only | Per-source ingestion watermarks for incremental retrieval |

**Activated tables (created in Layer 0, now actively used):**

| Table | Change |
|-------|--------|
| `qualification_runs`, `product_candidates`, `search_profiles`, `search_attempts`, `supplier_quotes`, `candidate_pricing_assessments`, `landed_cost_estimates` | First **live-source** writes now active via the daemon-managed local pipeline loop |
| `tender_document_sets` | First live-source document acquisition writes now active |
| `manifests` | First live-source verifiable output now active (local markdown/file artifact + row) |
| `pipeline_assessments` | First live-source process rollups now active; `validation_state` starts as `unverified` from first write |
| `workflow_jobs` | Now actively used by Job Runner for job lifecycle; `parent_job_id`, `validation_required`, `validation_verdict_id` remain nullable |
| `process_stage_status` | Now actively written by Job Runner — records execution status per stage per scope |
| `process_stage_heads` | Now actively written — advances atomically when a stage succeeds for its full scope |

### Authority Classification

New tables are mostly **local-only** (no sync): `agent_session_logs` contains full PTY bytes and structured traces — too large and sensitive for sync; `error_manifests` contains full error context; `source_cursors` and `raw_documents` are per-daemon operational state. The exception is `executors` (push-owned) — the daemon's identity record that will be synced to Supabase in Layer 4. Redacted summaries of `agent_session_logs` and `error_manifests` will be pushed as separate projection tables in Layer 4. Source configuration is still bootstrapped locally in this layer (hardcoded/seeded runtime copy), not yet remote-owned shared config.

### Shared Library Additions

None (Rust-only layer).

### Invariants Enforced

| Invariant | Type | Enforcement |
|-----------|------|-------------|
| Process-group supervision | Constitutional | Every agent session in own pgid; cleanup verdict (clean/forced/leaked) mandatory |
| Run manifests for every skill run | Constitutional | Daemon fallback if agent fails to narrate |
| Error manifests for every failure | Constitutional | Error manifest system active |
| Session artifact capture | Constitutional | Raw bytes stored as compressed immutable artifacts |
| Adapter-mediated execution | Constitutional | PTY or contract mode per skill's `interaction_profile` |
| Stage tracking with `stage_run_id` | Constitutional | Job Runner scopes commits to correct grain |
| Scope-aware commit semantics | Constitutional | Set-replacement stages (e.g., lot extraction) must commit atomically for full scope or fail entirely; additive stages (e.g., per-candidate pricing) may commit independently per unit |

### What's Stubbed

| Feature | Stub | Full Implementation |
|---------|------|-------------------|
| Scheduler | CLI manual trigger only | Layer 3: cron scheduler |
| Resource arbiter | No concurrency limits | Layer 3: typed quotas |
| PTY hardening | Basic session creation | Layer 3: fd hygiene, EIO handling, terminal geometry |
| Job claiming/leases | Single-threaded execution | Layer 3: concurrent job claiming |
| Source registry | Bootstrapped local seed / hardcoded source definitions | Layer 3: typed registry; Layer 4: `shared_source_configs` pull |
| Procurement coverage | First live source only (FTS) | Layer 2: adds Contracts Finder; Layer 5: PCS if needed |
| Session reaper | Basic cleanup | Layer 3: full reaper with escalation |
| Tracing | Basic log capture | Layer 3: structured + forensic tracing |
| Validation | Fields exist, always `validation_required = false` | Layer 3: dry-run; Layer 4: enforcement |
| Operator-facing manifest rendering | Minimal local markdown/file artifact emitted directly by the thin runner + skill | Layer 2: shared `packages/manifests` renderer and desktop presentation |
| Desktop/operator surface | CLI summary + local file artifact only | Layer 2: desktop control panel |
| Sync | Tracking columns populated, no outbox entries | Layer 4: full sync engine |

### Definition of Done

1. `cargo check --all-targets` and `cargo clippy --all-targets -- -D warnings` pass
2. Rust binary launches skills via CLI (both PTY and contract mode)
3. Manual CLI trigger ingests a live FTS batch using source-batched skill execution
4. The first live end-to-end local loop completes for at least one notice: normalize → qualify → acquire docs → extract lots → extract products → price-signal match → gross gate → landed cost → manifest
5. Verifiable output exists without later layers: CLI summary and/or local markdown manifest artifact proves the loop completed
6. Process-group supervision verified: every session gets pgid, cleanup verdict recorded (clean/forced/leaked)
7. Artifact capture works: PTY bytes compressed and stored, session metadata in `agent_session_logs`
8. Run manifests generated: agent narrative + daemon evidence appendix; fallback machine-authored if agent fails
9. Error manifest system operational: pipeline failures produce Markdown error manifests with full context
10. Skill linting CI gate validates directory structure, metadata, schemas, trigger boundaries
11. Smoke canary harness executes: page fingerprint check + auth verification + minimal extraction per source
12. Experiment 4 passes: 10 consecutive batch runs, ≥90% success rate, 0 orphaned processes
13. `stage_run_id` correctly scoped per business unit grain for all live-pipeline stage commits

### Test Scenarios

- `agent_pty_session.rs` — PTY session creation, output capture, full lifecycle including reap and cleanup verdict
- `agent_contract_session.rs` — Contract-mode session, structured event capture, result contract validation
- `agent_procurement_ingestion.rs` — Live FTS batch produces a local end-to-end opportunity artifact/manifest without desktop or sync
- `adapter_codex.rs` — Codex adapter capability probe, JSONL parsing, mode selection
- `adapter_claude.rs` — Claude adapter capability probe, JSON parsing, mode selection
- `adapter_result_contract.rs` — Result contract validation, checkpoint sequencing, partial failure classification
- `skill_lint.rs` — Skill directory structure validation
- `skill_canary_harness.rs` — Live smoke canary execution
- Agent crashes mid-session → cleanup verdict is `forced`, run manifest has machine-authored fallback
- Orphaned grandchild process → detected and reaped by process supervisor

### Dependencies

- Layer 0 (skills, intelligence pipeline, contracts-schema, SQLite schema)

### Forward-Compatibility Provisions

- Adapter trait designed for extension (new agent CLIs)
- Error manifest format includes fields for validator reasoning (Layer 3)
- Agent session logs include `run_manifest_path` and `run_manifest_hash` (Layer 3 validation)
- Job Runner aware of `parent_job_id` (for Layer 3 validation sub-jobs)

### Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| PTY session reliability across macOS/Linux | Medium | Focus on macOS first (development platform); PTY hardening in Layer 3 |
| Agent CLI API changes break adapter | Low | Adapter abstraction isolates CLI changes; capability probe detects incompatibility |
| Process-group cleanup misses edge cases | Medium | Conservative kill escalation; leaked process detection |

### Decisions Addressed

- ADR-001: First live skill-based procurement ingestion loop
- ADR-002: Local-operational-authority half becomes live on real writes; sync engine remains deferred to Layer 4
- ADR-003: Tauri + separate Rust daemon (daemon starts here)
- ADR-004: PTY-based agent execution from day 1 (hybrid PTY + contract)
- ADR-005: Skills + explicit binding + daemon orchestration

---

## 4. Layer 2 — Single-Operator Desktop MVP

### Narrative

Prove repeatable operator value with a manual-trigger desktop sitting on top of the Layer-1 live local pipeline. This is the first user-facing layer — a single operator runs the system daily, reviews pipeline output in a desktop control panel, and makes accept/reject/watch decisions. This layer expands live procurement coverage to Contracts Finder, renders operator-facing manifests and coverage metrics, and persists single-operator decisions locally as queued intents without violating remote authority ownership.

**If we never build Layer 3:** A single operator can use the system with manual triggering for daily procurement intelligence. No cron, no automation, but the core value loop works.

### Capability Delivered

Single operator runs system daily for one week, making real accept/reject decisions. The pipeline surfaces genuine opportunities from live procurement data with screening-grade margin estimates. The desktop control panel provides list/detail views, agent session monitoring, and decision capture.

### Components Touched

| Component | Action | Files |
|-----------|--------|-------|
| Skills | Create | 1 (Contracts Finder ingestion) |
| `packages/intelligence` | Create | 1 (coverage-rollup.ts) |
| `packages/ui` | Create | 5 (index, contract-card-frame, manifest-viewer, pipeline-badge, sync-age-badge) |
| `packages/manifests` | Create | 2 (render-opportunity.ts, opportunity.md.ts template) |
| `apps/desktop` | Create | 14 files (full desktop shell) |

### Data Introduced

**Activated tables (created in prior layers, now actively used by the desktop/operator surface):**

| Table | Change |
|-------|--------|
| `tender_document_sets` | Existing live document sets are now rendered in desktop detail views with completeness surfaced to the operator |
| `manifests` | Existing Layer-1 local manifests are now rendered through the operator-facing manifest package |
| `pipeline_assessments` | Coverage metrics (`pricedCoverageBps`, `landedCostCoverageBps`, `netQualifiedCoverageBps`, `unresolvedMandatoryLots`, `decisionBasisHash`) and overlay fields (`reviewRequired`, `availabilityStatus`, `validationState`) are now surfaced to the operator |
| `product_candidates` | `candidateValueShareBps` is now surfaced for top-value selection and coverage rollups |
| `pending_operator_intents` | Now actively written as the local single-operator decision ledger/overlay queue for `approve` / `reject` / `watch`, including `reviewedBasisHash`, `reviewedManifestHash`, and `reviewedAtStageRunId` for later CAS flush |

### Authority Classification

No new remote-owned tables are introduced in this layer. The desktop writes only local data (`pending_operator_intents`) while the authoritative shared decision table remains deferred to Layer 4. This preserves the authority matrix: single-operator local mode first, remote CAS later.

### Shared Library Additions

- `packages/ui` — Frame + slot presentational components (contract cards, manifest viewer, pipeline badges, sync-age badges)
- `packages/manifests` — Opportunity manifest renderer

### Invariants Enforced

| Invariant | Type | Enforcement |
|-----------|------|-------------|
| Stage coherence | Constitutional | All stage outputs share `stage_run_id`; `process_stage_status` updated and `process_stage_heads` advanced atomically |
| Coverage metrics accuracy | Constitutional | Process-level state is rollup from candidate-level, not arbitrary |
| Decision staleness tracking | Constitutional | `reviewedBasisHash`, `reviewedManifestHash`, `reviewedAtStageRunId` captured on local decision intents from first operator use |
| Feature flags for new capabilities | Operational | Progressive enablement via flags |

### What's Stubbed

| Feature | Stub | Full Implementation |
|---------|------|-------------------|
| Scheduling | Manual trigger from desktop | Layer 3: cron scheduler |
| Relation resolution | Notices processed independently | Layer 3: merge/relate heuristics |
| Landed cost | Screening-grade (flat freight, single FX) | Layer 5: decision-grade |
| Search sources | eBay only (from Layer 0) | Layer 5: Alibaba/1688, Amazon |
| Sync to Supabase | None — local-only operation | Layer 4: full sync |
| Operator decisions | Local intent queue only — single-operator desktop overlay, no shared authority yet | Layer 4: remote-owned `operator_decision_heads` + Supabase CAS |
| Validation | Disabled (fields exist) | Layer 3: dry-run |
| Degraded-mode UX | Basic daemon-unavailable detection | Layer 4: three-state (healthy/degraded/offline) |

### Definition of Done

1. FTS ingestion skill scrapes live notices from Find a Tender
2. Contracts Finder ingestion skill scrapes live notices
3. Notice normalization produces canonical internal records
4. Goods/services classifier correctly categorizes notices (goods/services/works/mixed)
5. Desktop control panel launches with QueryService + DaemonBridge + bootstrap state machine
6. Notice list view with keyset pagination displays live pipeline output
7. Opportunity detail view shows margin bands, product candidates, supplier quotes
8. Decision capture (`reject` / `watch` / `approve`) persists locally in `pending_operator_intents` with reviewed-basis metadata and correct local overlay behavior
9. Agent session monitoring page shows live terminal output via Channel<T> streaming
10. Operator-facing coverage metrics displayed: `pricedCoverageBps`, `netQualifiedCoverageBps`
11. Decision-basis staleness indicators flag stale decisions
12. Single operator runs daily for 1 week and reports genuine opportunities surfaced

### Test Scenarios

- `agent_procurement_ingestion.rs` — Real FTS skill session, output verified
- `agent_supplier_enrichment.rs` — Real enrichment skill session, output verified
- `agent_health_detection.rs` — Stall detection, auth challenge detection
- `agent_error_manifest.rs` — Error manifest creation on failures
- `apps/desktop/e2e/contracts.spec.ts` — Desktop browsing and decision flow
- Notice with 12 lots → system processes each lot independently, displays per-lot results
- Mixed contract → goods-addressable value used for margin computation, not whole contract
- Expired notice → correctly filtered, not displayed as active opportunity
- Agent session crashes → error manifest created, operator can see failure context in desktop

### Dependencies

- Layer 0 (intelligence pipeline, skills, contracts-schema)
- Layer 1 (daemon runtime, artifact capture, process supervision)

### Forward-Compatibility Provisions

- Desktop bootstrap state machine detects daemon health modes (prep for Layer 4 degraded-mode UX)
- `packages/ui` frame + slot pattern designed for both desktop and web shells
- Coverage metrics on `pipeline_assessments` feed into Layer 4 sync projections
- `stage_run_id` on all stage outputs enables Layer 4 sync batch boundaries
- Local decision intents already carry the future CAS payload (`process_id`, disposition, reviewed-basis metadata, idempotency key), so Layer 4 can activate remote-owned decisions without any table reclassification

### Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Source website changes break FTS/CF skills | High | Error manifest flags failures; skill update cycle |
| Desktop UX reveals pipeline quality issues | Medium | Expected — this is why desktop comes before hardening |
| Single-source eBay pricing not representative | High | Known limitation; Layer 5 adds Alibaba/1688 |
| QueryService read performance on large datasets | Low | SQLite reads are fast; keyset pagination prevents full scans |

### Decisions Addressed

- ADR-003: Tauri v2 desktop shell (first desktop UI)
- ADR-008: Desktop = control panel with full read/write/trigger
- ADR-001: Live procurement source skills (FTS, Contracts Finder)

---

## 5. Layer 3 — Operational Hardening

### Narrative

Daily automation without babysitting. The daemon gains cron scheduling, batched ingestion, relation resolution, PTY hardening, resource management, and the worker-validator system in dry-run mode. The system runs unattended for 5 consecutive days. This layer transforms the single-operator tool into a production-grade pipeline.

**If we never build Layer 4:** A single operator has a fully automated, self-monitoring pipeline that runs daily, handles relation resolution, manages resources, and validates output quality via dry-run semantic checking. No multi-user visibility, but the core automation works.

### Capability Delivered

System runs unattended for 5 consecutive days with ≥95% skill success rate. Cron-scheduled ingestion, resource management, PTY hardening, source health monitoring, and dry-run worker-validator semantic validation are all operational. The skill promotion flow ensures quality gates for skill updates.

### Components Touched

| Component | Action | Files |
|-----------|--------|-------|
| `crates/daemon` | Create | 12 additions (scheduler/cron, sources/registry, jobs/claim, pty/reaper+hardening, agents/arbiter, db/checkpoints, tracing/structured+forensic, validation/policy+executor+manifest_writer) |
| Skills | Create | 10 (4 operational: auth-refresh/alibaba+1688, source-health/fts+alibaba; 6 validator-base files: SKILL.md, result.schema.json, checkpoint.schema.json, canary.inputs.jsonl, golden.assertions.yaml, validator-rubric.yaml) |
| `packages/intelligence` | Create | 1 (relations/resolve-process-relations.ts) |
| `packages/manifests` | Create | 2 (render-error-manifest.ts, error.md.ts template) |

### Data Introduced

**New tables:**

| Table | Authority | Purpose |
|-------|-----------|---------|
| `validation_verdicts` | Local-only | Full validator reasoning and evidence |
| `source_runtime_state` | Local-only | Circuit breaker state, cooldown tracking |
| `listing_evidence` | Local-only | Two-tier cache for search results + listing evidence |
| `process_relations` | Push-owned | Relation edges between processes |
| `process_aliases` | Push-owned | Redirect from retired process IDs |

**Modified tables:**

| Table | Change |
|-------|--------|
| `workflow_jobs` | `parent_job_id`, `validation_required`, `validation_verdict_id` now actively used for validation sub-jobs (still nullable — `validation_required` defaults to `false` except when policy triggers) |
| `agent_session_logs` | `run_manifest_path` and `run_manifest_hash` now actively populated by manifest writer |

### Authority Classification

| Table | Classification | Rationale |
|-------|---------------|-----------|
| `validation_verdicts` | Local-only | Full validator reasoning too large/sensitive for sync |
| `source_runtime_state` | Local-only | Per-daemon runtime state |
| `listing_evidence` | Local-only | Cache layer, not authoritative |
| `process_relations` | Push-owned | Shared visibility in Layer 4 |
| `process_aliases` | Push-owned | Stable deep links across desktop/web |

### Shared Library Additions

- `packages/intelligence/src/relations/resolve-process-relations.ts` — Relation resolution engine

### Invariants Enforced

| Invariant | Type | Enforcement |
|-----------|------|-------------|
| Explicit skill binding for cron | Constitutional | Scheduler binds skill id + version at enqueue time |
| Skill promotion flow gates | Constitutional | Draft → candidate → canary → shadow → stable → rollback |
| Fail-open validation | Constitutional | Validator failures never block pipeline; worker results advance |
| Dry-run isolation | Constitutional | Dry-run verdicts logged to scratch, no authoritative rows/outbox/stage heads advanced |
| Source circuit breaker | Operational | Repeated challenge-class failures → cooldown → only canary/auth jobs |
| Resource arbiter quotas | Operational | Browser slots, agent slots, source lanes |

### What's Stubbed

| Feature | Stub | Full Implementation |
|---------|------|-------------------|
| Validation enforcement | Dry-run only — verdicts logged, not enforced | Layer 4: enforcement mode |
| Domain-specific validators | Base validator only (generic semantic quality) | Layer 4: 4 domain overlays |
| Validator calibration | Initial calibration against gold-set manifests | Layer 4: formal calibration harness |
| Sync | Tracking columns populated, no outbox/push/pull | Layer 4: full sync |
| Multi-user | Single operator only | Layer 4: multi-user via web portal |

### Definition of Done

1. Cron scheduler runs on configured schedule with explicit skill binding
2. Batched ingestion: scheduled FTS + CF runs complete without manual intervention
3. Relation resolution engine correctly classifies notices (merged/related/possible_duplicate/separate)
4. PTY hardening: fd hygiene, EIO handling, terminal geometry stability verified
5. Host environment awareness: sleep/wake detection, catchup sweeps after resume
6. Resource arbiter enforces typed quotas (browser slots, agent slots, source lanes)
7. Skill promotion flow operational: draft → candidate → canary → shadow → stable
8. Source-health canaries and auth-refresh skills operational
9. Worker-validator dry-run: run manifests generated, base validator renders verdicts, verdicts logged without enforcement
10. Error manifest control-panel view displays failures with resolution context
11. System runs unattended for 5 consecutive days, ≥95% success rate, no manual intervention

### Test Scenarios

- `fault_process_cleanup.rs` — Orphaned grandchildren, dropped handle, cleanup timeout, leaked detection
- `fault_pty_edge_cases.rs` — PTY EOF/EIO handling, fd leak scenarios, terminal geometry
- `fault_host_events.rs` — Host sleep/monotonic gap, resource exhaustion, concurrent pressure
- `fault_partial_failure.rs` — Partial checkpoint then crash, result contract failures
- `replay_pty_sessions.rs` — Replay captured PTY byte streams against adapter parser
- `replay_contract_sessions.rs` — Replay captured sessions against adapter parser
- `cron_scheduler.rs` — Cron-based scheduling
- `job_claim.rs` — Concurrent job claiming and lease management
- `skill_golden_journeys.rs` — Invariant-based E2E per skill family
- `skill_shadow_compare.rs` — Quality metric diff between stable and candidate versions
- `skill_drift_monitor.rs` — Page fingerprint, fallback depth, fill-rate trending
- `validation_policy.rs` — All policy mode × trigger condition combinations
- `validation_executor.rs` — Validator sub-job spawning, kickoff prompt composition
- `validation_dry_run.rs` — Dry-run verdicts logged, pipeline not blocked
- `validation_golden_verdicts.rs` — Known-good/bad manifests → correct verdicts
- `resolve-process-relations.test.ts` — Relation scoring and hard blockers

### Dependencies

- Layer 0 (intelligence pipeline, gold set for validator calibration)
- Layer 1 (daemon runtime, agent adapter)
- Layer 2 (desktop shell for error manifest view, pipeline data model)

### Forward-Compatibility Provisions

- Validation policy engine ready for enforcement mode toggle (Layer 4)
- Run manifest corpus accumulates for validator calibration (Layer 4)
- `validation_verdicts` table schema includes all verdict metadata fields for Layer 4 projections
- Source circuit breaker state feeds into validation risk-stratified sampling

### Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Validator false positive rate too high for enforcement | Medium | Dry-run mode allows calibration before enforcement |
| Cron scheduling misses edge cases (timezone, DST) | Low | Catchup sweeps handle missed schedules |
| Relation resolution produces false merges | Medium | Conservative thresholds; operator can unmerge |
| Source changes break multiple skills simultaneously | Medium | Source-health canaries detect before production jobs |

### Decisions Addressed

- ADR-004: PTY hardening (full reliability)
- ADR-005: Explicit skill binding for cron (production reliability)

---

## 6. Layer 4 — Shared Visibility

### Narrative

Collaboration infrastructure. The sync engine activates, projecting pipeline data to Supabase for the read-only web viewing portal. Operator decisions flow through Supabase CAS. Worker-validator enforcement mode is enabled after dry-run calibration. Two users can now view the same opportunity pipeline — one via desktop (read-write), one via web (read-only).

**If we never build Layer 5:** Multi-user visibility works with screening-grade accuracy and single-marketplace pricing. The system is production-ready for the initial category whitelist.

### Capability Delivered

Two users view the same pipeline: desktop operator (read-write) and web viewer (read-only). Sync engine pushes pipeline data to Supabase. Operator decisions use compare-and-swap via Supabase. Validator rejections quarantine results in enforcement mode. Domain-specific validator overlays for procurement, supplier enrichment, document acquisition, and product extraction.

### Components Touched

| Component | Action | Files |
|-----------|--------|-------|
| Database | Create | 8 files (SQLite sync migration, 6 Supabase migrations, Supabase config) |
| `crates/daemon` | Create | 1 (db/sync.rs) |
| Skills | Create | 4 domain validator overlays (procurement-ingestion, supplier-enrichment, document-acquisition, product-extraction) |
| `apps/web` | Create | 7 files (full web shell) |

### Data Introduced

**SQLite sync infrastructure tables:**

| Table | Authority | Purpose |
|-------|-----------|---------|
| `local_outbox` | Local-only (SQLite sync infrastructure) | Transactional outbox entries with `content_hash` and `schema_epoch` from introduction |
| `sync_batches` | Local-only (SQLite sync infrastructure) | Batch metadata with `schema_epoch` |
| `pull_cursors` | Local-only (SQLite sync infrastructure) | Per-domain pull cursor position |
| `pending_operator_intents` | Local-only | Existing Layer-2 operator intent queue, now reconciled/flushed through remote CAS |

**Supabase projection tables (push-owned, mirroring local):**

All push-owned tables from Layers 0–3 get Supabase counterparts: `procurement_processes`, `contract_notices`, `notice_lots`, `tender_document_sets`, `qualification_runs`, `product_candidates`, `search_profiles`, `search_attempts`, `supplier_quotes`, `candidate_pricing_assessments`, `landed_cost_estimates`, `manifests`, `process_relations`, `process_aliases`, `pipeline_assessments`, `bid_workflow_states`, `process_stage_status`, `process_stage_heads`.

**Supabase summary tables (push-owned projections):**

| Table | Purpose |
|-------|---------|
| `workflow_job_summaries` | Redacted job status for web |
| `error_manifest_summaries` | Redacted error context for web |
| `validation_verdict_summaries` | Verdict status for web |
| `viewer_sync_status` | Sync freshness indicator |
| `executors` | Heartbeat summary |

**Supabase remote-owned tables:**

| Table | Purpose |
|-------|---------|
| `shared_company_profile` | Org-wide bidder profile |
| `shared_source_configs` | Source enablement and configuration |
| `operator_decision_heads` | Operator dispositions with CAS |

**Junction tables (Supabase):**

| Table | Purpose |
|-------|---------|
| `notice_cpv_codes` | Notice ↔ CPV code mapping |
| `product_commodity_codes` | Product ↔ commodity code mapping |
| `lot_cpv_codes` | Lot ↔ CPV code mapping |

### Authority Classification

Formalized and enforced at the Supabase database level via RLS:

| Role | Push projections | Push summaries | Remote config | Remote decisions |
|------|-----------------|----------------|---------------|-----------------|
| `viewer` | SELECT | SELECT | SELECT | SELECT |
| `operator` | SELECT | SELECT | SELECT | SELECT + UPDATE (CAS RPC) |
| `admin` | SELECT | SELECT | Full CRUD | Full CRUD |
| `daemon` | INSERT/UPDATE (batch RPC) | INSERT/UPDATE (batch RPC) | SELECT (pull) | SELECT (pull) |

### Shared Library Additions

- `apps/web` uses existing `packages/ui` components with web-specific data fetching hooks
- `packages/contracts-schema` types used for Supabase query typing

### Invariants Enforced

| Invariant | Type | Enforcement |
|-----------|------|-------------|
| Push batch atomicity | Constitutional | Single transactional RPC, not individual upserts |
| Outbox + data in same SQLite transaction | Constitutional | From activation, every push-owned mutation creates outbox entry atomically |
| CAS for operator decisions | Constitutional | `base_remote_revision` checked; mismatch → conflict |
| Active-writer lease | Constitutional | One daemon per org enforced at push protocol level |
| `schema_epoch` on every batch | Constitutional | Incompatible epoch → sync pauses, local operation continues |
| Batch boundaries never cross `stage_run_id` | Constitutional | Push dispatcher enforces |
| Single-in-flight batch | Constitutional | One batch at a time per push stream |
| Validation enforcement | Operational | Rejections quarantine results (after dry-run calibration) |
| RLS on all Supabase tables | Constitutional | Policy matrix enforced at database boundary |

### What's Stubbed

| Feature | Stub | Full Implementation |
|---------|------|-------------------|
| Schema epoch transitions | Initial epoch only (no transitions needed yet) | Future: expand/drain/upgrade/contract |
| Multi-org | Single org hardcoded | Future: org_id scoping |
| Full rebuild | Not implemented | Future: generation-based remote rebuild |
| Logical consistency auditor | Not implemented | Future: domain invariant checking |

### Definition of Done

1. Push protocol: transactional outbox entries created for all push-owned mutations in same SQLite transaction
2. Push batches applied to Supabase via single transactional RPC with `schema_epoch`, `row_version` ordering
3. Pull protocol: cursor-based pull for `shared_source_configs`, `shared_company_profile`, `operator_decision_heads`
4. Operator decisions: queued local intents flush through Supabase CAS RPC with `base_remote_revision`; conflicts returned (not silently overwritten) and local queue state updated
5. Active-writer lease: second daemon instance detected and rejected
6. RLS policies enforced for all 4 roles (viewer, operator, admin, daemon)
7. Web viewing portal displays opportunities, manifests, pipeline status (read-only)
8. Supabase Broadcast invalidation delivers live updates to web portal
9. `viewer_sync_status` shows sync freshness (green/amber/red)
10. Validation enforcement mode: validator rejections quarantine results as `AGENT_SUSPECT_OUTPUT`
11. 4 domain-specific validator overlays deployed and producing verdicts
12. Two users can view the same pipeline: desktop (read-write) + web (read-only)

### Test Scenarios

- `sqlite_schema.rs` — Full schema creation and migration
- `sqlite_supabase_sync.rs` — Push-pull round-trip sync
- `apps/web/e2e/contracts.spec.ts` — Web portal browsing
- Push batch with stale `row_version` → remote rejects (idempotent)
- Push batch with incompatible `schema_epoch` → sync pauses, local operation continues
- Two desktop operators make concurrent decisions → CAS detects conflict, second operator retries
- Validator rejects output in enforcement mode → results quarantined, error manifest created
- Validator itself crashes → fail-open, worker results advance with `validation_skipped_validator_error`
- Sync degraded → desktop shows browse-only mode, web shows staleness indicator
- Full push replay after sync recovery → idempotent, no duplicates

### Dependencies

- Layer 0–3 (all pipeline data, all push-owned tables with sync tracking columns)

### Forward-Compatibility Provisions

- `schema_epoch` exists for future schema evolution protocol
- RLS roles designed for future multi-org (`org_id` column planned)
- Projection tables designed for future materialized views

### Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Sync engine bugs corrupt remote data | Medium | Local copy is authoritative; remote can be republished |
| Validator enforcement blocks good data | Medium | Dry-run calibration in Layer 3; operator override available |
| Push batch size exceeds Supabase RPC limits | Low | Batch size bounded by `stage_run_id` scope |
| Network interruptions during push | Medium | Identical-envelope retries; degraded mode continues local operation |

### Decisions Addressed

- ADR-002: Sync / remote-projection half of the authority matrix fully operational
- ADR-008: Web viewing portal created (read-only)

---

## 7. Layer 5 — Expanded Commercial Intelligence

### Narrative

Broader categories, more supplier sources, decision-grade accuracy. The customs classification engine expands from screening-grade to full multi-source recall with 10-digit resolution, measure interaction, and trade remedy monitoring. Additional marketplace skills (Alibaba/1688, Amazon) provide multi-source pricing. FX engine gains dual-rate capability (HMRC customs + commercial spot). Freight engine gains real API integrations. Each new category requires its own gold-set validation before entering production.

**If we never build Layer 6:** The system is a mature procurement intelligence platform covering ≥3 goods categories with decision-grade landed cost and ≥2 supplier sources per category. Only request pipeline automation is missing.

### Capability Delivered

System handles ≥3 goods categories with decision-grade landed cost and ≥2 supplier sources per category. Full customs classification pipeline: multi-source recall (FPO + CPV priors + rule-based hints + tariff keyword search), multi-family expansion to UK 10-digit, exporter/manufacturer resolution, measure interaction engine, and trade remedy monitoring.

### Components Touched

| Component | Action | Files |
|-----------|--------|-------|
| Skills | Create | 4 (Amazon, Alibaba, 1688, PCS ingestion) |
| `packages/intelligence` | Create | 19 files (full customs, FX, freight, validation comparison) |

### Data Introduced

No new tables and no new columns — all decision-grade columns exist as nullable from Layer 0. Layer 5 populates previously-empty fields:

| Table | Columns Now Populated |
|-------|----------------------|
| `landed_cost_estimates` | Decision-grade fields: full duty breakdown, measure interaction results, exporter rate basis, supplementary unit requirements, valuation mode (all existed as nullable since L0) |
| `product_candidates` | Commodity code families from multi-source recall (via `product_commodity_codes` junction table, existed since L0) |
| `supplier_quotes` | Multi-source quotes (Alibaba, 1688, Amazon rows alongside existing eBay rows — same schema, more rows) |

### Authority Classification

No changes — all existing classifications hold.

### Shared Library Additions

- `packages/intelligence/src/customs/*` — Full customs classification engine (6 files)
- `packages/intelligence/src/fx/*` — Full FX engine (4 files)
- `packages/intelligence/src/logistics/*` — Full freight engine (5 files)
- `packages/intelligence/src/validation/*` — Landed-cost comparison adapters (3 files)

### Invariants Enforced

| Invariant | Type | Enforcement |
|-----------|------|-------------|
| Conservative customs defaults | Constitutional | MFN rates, residual duty, standard VAT when resolution is incomplete |
| Customs branch divergence as hard blocker | Constitutional | Materially different tariff branches → `CLASSIFICATION_AMBIGUOUS`, not averaged |
| Integer pence for all new cost fields | Constitutional | Type system |
| Per-category gold-set validation | Operational | New category requires gold-set expansion before production |

### What's Stubbed

Nothing significant — this is the full commercial intelligence implementation.

### Definition of Done

1. Alibaba/1688 search skills operational with Chinese keyword clusters
2. Amazon search skill operational
3. PCS (Scotland) ingestion skill operational
4. HMRC FPO API integration for HS code classification candidates
5. Multi-source recall produces candidates from ≥2 independent sources (FPO + CPV priors)
6. 10-digit resolution via HMRC Trade Tariff API produces UK-specific commodity codes
7. Measure scanner detects AD/CVD/safeguard/quota measures and models interactions
8. HMRC Exchange Rate Service provides customs valuation FX (monthly rolling)
9. Xe commercial FX provides spot rates
10. Freightos + SeaRates provide freight market data
11. Decision-grade landed cost per validated category (with full audit trail)
12. Category-specific gold-set expansion completed for ≥3 categories

### Test Scenarios

- `hmrc-fpo.test.ts` — FPO HS code classification
- Multi-family expansion: product with 3 plausible HS6 families → all UK10 descendants evaluated
- Trade remedy detection: steel product → AD/CVD measures detected, residual rate applied
- Compound duty: product with specific duty (£X/kg) → not flattened to percentage
- FX monthly rollover → forced recalculation of affected estimates
- Alibaba search with Chinese retry after English failure
- Category whitelist enforcement: medical device → rejected before search

### Dependencies

- Layer 0–4 (full pipeline, sync, web portal)

### Forward-Compatibility Provisions

- Classification freshness system (`tariffCheckedAt`, `classificationFreshUntil`, `tradeRemedyWatchState`) enables future auto-refresh
- CBAM forward placeholder for iron/steel/aluminium ready for future regulation

### Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| HMRC FPO API access restricted | Medium | FPO is not hard dependency; fallback to CPV-to-HS mapping |
| Anti-bot detection on Alibaba/1688 | High | Anti-bot classification table; source-health canaries; auth-refresh skills |
| Customs classification wrong for complex products | Medium | Multi-source recall + conservative defaults + abstention |
| Freight API rate variability | Medium | Dual-source cross-check (Freightos + SeaRates) |

### Decisions Addressed

- ADR-006: Internal landed-cost engine at full scope (decision-grade)
- ADR-001: Additional marketplace skills (Alibaba, 1688, Amazon)

---

## 8. Layer 6 — Request Pipeline Automation

### Narrative

Portal-specific bid skills automate the request pipeline — preparation and submission steps for tender portal applications. This does NOT place real-money bids; actual negotiations and fulfillment are manual. The system produces full audit trails (screenshots, logs, manifests) for every submission.

**If we never build this layer:** The system is fully functional for intelligence and decision-making. Operators manually prepare and submit portal applications based on system recommendations.

### Capability Delivered

System can prepare and submit a request on one portal (Find a Tender or Contracts Finder) with full audit trail. Includes agent-browser workflow, screenshot capture, and manifest generation for submitted requests.

### Components Touched

| Component | Action | Files |
|-----------|--------|-------|
| Skills | Create | 1 (bid-portal-proactis) |
| `packages/bidding` | Create | 3 (workflow registry, proactis workflow, company profile schema) |

### Data Introduced

**Modified tables:**

| Table | Enhancement |
|-------|-------------|
| `bid_workflow_states` | Fully active: `not_started` → `ready_to_bid` → `bid_in_progress` → `bid_submitted` / `bid_failed` |

### Authority Classification

`bid_workflow_states` is push-owned — daemon tracks bid progress, pushes to web portal.

### Shared Library Additions

- `packages/bidding` — Workflow registry and portal-specific workflow definitions

### Invariants Enforced

| Invariant | Type | Enforcement |
|-----------|------|-------------|
| No real-money bids without human approval | Constitutional | Bid workflow requires operator `approved` disposition |
| Bid workflow carries explicit scope | Constitutional | Which lots and candidates included |
| Full audit trail for submissions | Constitutional | Screenshots, logs, manifests captured |

### What's Stubbed

Nothing — this is the final layer.

### Definition of Done

1. Bid-portal skill navigates target portal via browser automation
2. Company profile data injected from `shared_company_profile`
3. Request preparation steps automated (form filling, document upload)
4. Submission step automated with screenshot capture at each stage
5. Full terminal output tracing for bid workflow
6. Manifest generated for submitted request with audit trail
7. Bid failure produces error manifest with portal-specific context
8. System can prepare and submit one request on the target portal end-to-end

### Test Scenarios

- Full bid workflow: approve → prepare → submit → manifest
- Bid failure mid-workflow: error manifest with screenshots at failure point
- Stale decision blocks bid: operator decision marked stale → bid workflow blocked until re-affirm
- Portal website changed: error manifest flags skill failure with step context

### Dependencies

- Layer 0–5 (full pipeline, desktop, web, sync, decision-grade intelligence)

### Forward-Compatibility Provisions

- Workflow registry pattern supports additional portal-specific skills
- Company profile schema designed for multiple portal requirements

### Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Portal UI changes break bid skill | High | Low impact — not real money; error manifest flags failures |
| Form structure varies by procurement type | Medium | Portal-specific workflow handles variance |

### Decisions Addressed

- ADR-007: Request pipeline automation (not real-money bids)

---

## 9. Cross-Layer: Schema Stability Matrix

### Table Introduction and Modification Across Layers

| Table | Created | First Active Use | Sync Activated | Authority |
|-------|---------|-----------------|----------------|-----------|
| `procurement_processes` | L0 | L0 | L4 | Push-owned |
| `contract_notices` | L0 | L0 | L4 | Push-owned |
| `notice_lots` | L0 | L0 | L4 | Push-owned |
| `product_candidates` | L0 | L0 | L4 | Push-owned |
| `search_profiles` | L0 | L0 | L4 | Push-owned |
| `search_attempts` | L0 | L0 | L4 | Push-owned |
| `supplier_quotes` | L0 | L0 | L4 | Push-owned |
| `candidate_pricing_assessments` | L0 | L0 | L4 | Push-owned |
| `pipeline_assessments` | L0 | L0 (gold-set harness), L1 (first live-source rollups), L2 (desktop coverage surface) | L4 | Push-owned |
| `qualification_runs` | L0 | L0 | L4 | Push-owned |
| `landed_cost_estimates` | L0 | L0 (screening-grade), L5 (decision-grade fields populated) | L4 | Push-owned |
| `tender_document_sets` | L0 | L1 (first live-source document acquisition) | L4 | Push-owned |
| `manifests` | L0 | L1 (first live-source verifiable output), L2 (desktop rendering) | L4 | Push-owned |
| `process_stage_status` | L0 | L1 (stage tracking) | L4 | Push-owned |
| `process_stage_heads` | L0 | L1 (stage tracking) | L4 | Push-owned |
| `bid_workflow_states` | L0 | L6 (bid automation) | L4 | Push-owned |
| `workflow_jobs` | L0 | L1 (job runner), L3 (validation fields) | Never (local-only) | Local-only |
| `notice_cpv_codes` | L0 | L0 | L4 | Push-owned (junction) |
| `product_commodity_codes` | L0 | L0 | L4 | Push-owned (junction) |
| `lot_cpv_codes` | L0 | L0 | L4 | Push-owned (junction) |
| `agent_session_logs` | L1 | L1 (run-manifest fields populated from introduction) | Never (local-only) | Local-only |
| `raw_documents` | L1 | L1 | Never (local-only) | Local-only |
| `error_manifests` | L1 | L1 | Never (local-only) | Local-only |
| `executors` | L1 | L1 | L4 | Push-owned |
| `source_cursors` | L1 | L1 | Never (local-only) | Local-only |
| `sources` | L1 | L1, L4 (pull from `shared_source_configs`) | Never (local-only) | Local-only (runtime copy) |
| `validation_verdicts` | L3 | L3 | Never (local-only) | Local-only |
| `source_runtime_state` | L3 | L3 | Never (local-only) | Local-only |
| `listing_evidence` | L3 | L3 | Never (local-only) | Local-only |
| `process_relations` | L3 | L3 | L4 | Push-owned |
| `process_aliases` | L3 | L3 | L4 | Push-owned |
| `local_outbox` | L4 | L4 | N/A (SQLite sync infra) | Local-only |
| `sync_batches` | L4 | L4 | N/A (SQLite sync infra) | Local-only |
| `pull_cursors` | L4 | L4 | N/A (SQLite sync infra) | Local-only |
| `pending_operator_intents` | L0 | L2 (desktop decision capture), L4 (CAS flush + conflict tracking) | Never (local-only) | Local-only |
| `operator_decision_heads` | L4 (Supabase + pulled local cache) | L4 | N/A (Supabase-native) | Remote-owned |
| `shared_company_profile` | L4 (Supabase) | L4, L6 (bid workflow fields populated) | N/A (Supabase-native) | Remote-owned |
| `shared_source_configs` | L4 (Supabase) | L4, L5 (new source entries) | N/A (Supabase-native) | Remote-owned |
| Summary tables (`workflow_job_summaries`, `error_manifest_summaries`, `validation_verdict_summaries`, `viewer_sync_status`) | L4 (Supabase) | L4 | N/A (push-only projections) | Push-owned |

### Schema Evolution Rules

1. **No table reclassification.** A table's authority classification is permanent from its introduction layer. `operator_decision_heads` is remote-owned from introduction in Layer 4; before that, local single-operator decisions exist only as `pending_operator_intents`.
2. **All columns exist from table creation.** Later layers populate existing nullable columns — they never add new columns to tables introduced in earlier layers. This eliminates schema migrations between layers.
3. **Sync tracking columns from day 1.** All push-owned tables include `owner_id`, `row_version`, `last_local_seq`, `stage_run_id`, `created_at`, `updated_at` from their creation in Layer 0, and those fields are populated on first write even before the outbox engine exists.
4. **No column renames.** Column names are stable across layers (important for sync field mapping).
5. **Junction tables introduced with parent tables.** `notice_cpv_codes` created alongside `contract_notices` in L0.
6. **Job kinds.** Each pipeline stage maps to a specific `job_kind` value in `workflow_jobs` (e.g., `sync_source`, `normalize_notice`, `qualify_notice`, `extract_products`, `enrich_supplier_prices`, `estimate_landed_cost`). The full mapping of 16 job kinds to pipeline stages is defined in the research document Appendix C.
7. **Validation forward-compatibility exists from job-table introduction.** `workflow_jobs.parent_job_id` is nullable from Layer 0, `workflow_jobs.validation_required` exists from Layer 0 with default `false`, and `workflow_jobs.validation_verdict_id` is nullable from Layer 0.
8. **Run-manifest columns exist from session-log introduction.** `agent_session_logs.run_manifest_path` and `agent_session_logs.run_manifest_hash` exist when `agent_session_logs` is introduced in Layer 1 and are populated from the first skill run.
9. **Validation state exists from assessment-table introduction.** `pipeline_assessments.validation_state` exists from Layer 0 with default `'unverified'`.

---

## 10. Cross-Layer: Interface Contracts

### Boundaries That Later Layers MUST NOT Break

| Interface | Introduced | Consumers | Contract |
|-----------|-----------|-----------|----------|
| Agent Adapter trait | L1 | All skill execution | Unified event stream regardless of PTY/contract mode; mode selection via `interaction_profile` |
| QueryService envelope | L2 | Desktop UI | `QueryEnvelope<T>` response format for all SQLite reads |
| DaemonBridge commands | L2 | Desktop UI | Mutation acknowledgment format; UDS/named-pipe transport |
| Channel<T> streaming | L2 | Desktop session viewer | Session streaming protocol |
| Skill directory standard | L0 | All skills, skill linting | SKILL.md + result.schema.json + checkpoint.schema.json + canary.inputs.jsonl + golden.assertions.yaml |
| Result contract format | L1 | Job Runner, OQG, validators | Schema-validated output format per skill |
| Push batch RPC | L4 | Sync engine → Supabase | `apply_push_batch(batch_json)` with `schema_epoch`, `row_version` ordering |
| CAS decision RPC | L4 | Desktop → Supabase | `update_operator_decision(process_id, disposition, base_revision, command_id)` — idempotent via `command_id` to handle lost-response retries |
| Pull-before-push ordering | L4 | Sync startup | Pull remote-owned state FIRST, then classify/coalesce pending intents, then resume push — ensures cross-authority consistency |
| `packages/contracts-schema` types | L0 | All TypeScript code | Canonical type definitions; changes must be backward-compatible |
| `packages/ui` frame + slot pattern | L2 | Desktop + web shells | Presentational components; data-fetching hooks are shell-specific |
| Evaluation harness API | L0 | Gold-set benchmarking | Pipeline runner + metrics computation interface |

### Stage Scope Matrix

`ProcessStageStatus.scopeKind` is constrained to exactly six values: `process`, `notice`, `lot`, `candidate`, `candidate_source_pair`, and `candidate_quote_scenario`. The strategy must keep every stage on one of these grains once a process/notice identity exists.

| Stage | Runtime Grain | `scopeKind` | Required Entities Already Introduced |
|-------|---------------|-------------|--------------------------------------|
| Ingest | source-batched precursor job | Not recorded in `process_stage_status` until notice/process identity exists | `sources`, `source_cursors`, procurement ingestion skill |
| Normalize | per notice | `notice` | `contract_notices` |
| Relate | per process cluster | `process` | `procurement_processes`, `contract_notices` |
| Qualify | per lot | `lot` | `notice_lots`, `qualification_runs` |
| Acquire docs | per notice | `notice` | `contract_notices`, `tender_document_sets` |
| Extract lots | per notice | `notice` | `contract_notices`, `notice_lots` |
| Extract products | per lot | `lot` | `notice_lots`, `product_candidates` |
| Score quoteability | per candidate | `candidate` | `product_candidates`, `search_profiles` |
| Price-signal match | per candidate-source pair | `candidate_source_pair` | `product_candidates`, `search_profiles`, `search_attempts`, `supplier_quotes` |
| Gross margin gate | per candidate | `candidate` | `candidate_pricing_assessments`, `supplier_quotes` |
| Landed cost | per candidate-quote scenario | `candidate_quote_scenario` | `candidate_pricing_assessments`, `landed_cost_estimates` |
| Net margin gate | per candidate | `candidate` | `landed_cost_estimates`, `pipeline_assessments` rollup inputs |
| Manifest | per process | `process` | `pipeline_assessments`, `manifests` |
| Operator decision | per process | `process` | `pending_operator_intents` first, `operator_decision_heads` once Layer 4 activates remote authority |
| Bid workflow | per process with explicit lot/candidate scope | `process` | `bid_workflow_states`, explicit bid scope payload |

Implementation note for the apparent ingest-grain mismatch:

- Source-batched ingestion is tracked at the `workflow_jobs` / `source_cursors` / `executors` level.
- `process_stage_status` begins once a notice or process identity exists, so persisted stage-head coherence still stays inside the six canonical `scopeKind` values from the research model.

Scope dependency rule:

- Lot-grain stages require `notice_lots` to exist in the same or an earlier layer.
- Candidate-grain stages require `product_candidates` to exist in the same or an earlier layer.
- Candidate-source-pair stages require `search_profiles` and `supplier_quotes` machinery to exist in the same or an earlier layer.
- Candidate-quote-scenario stages require a persisted candidate-scoped pricing assessment and selected quote before landed cost runs.

### Stage Boundary Contracts

These input/output contracts between pipeline stages are established in Layer 0 and must hold through all layers:

| Boundary | Required Input | Required Output | Established |
|----------|---------------|-----------------|-------------|
| qualify → acquire_docs | `QualificationResult` with lotId, expiryQualified, goodsQualified, accessRoute | Per-lot pass/review/reject | L0 |
| acquire_docs → extract_lots | Document set completeness, document roles | `TenderDocumentSet` with `documentSetHash` | L1 |
| extract_lots → extract_products | `NoticeLot` with title/description, lotNumber | At least one lot per qualified notice | L0 |
| extract_products → score_quoteability | `ProductCandidate` with lotId, headNoun, identityConfidence, candidateType, matchArchetype | At least one candidate per qualified lot | L0 |
| score_quoteability → price_match | quoteabilityScore, recallRiskScore, SearchProfile with queryWaves | quoteabilityScore ≥ 0.45 for auto-search | L0 |
| price_match → gross_gate | `SupplierQuote` with matchConfidence, quoteUsabilityConfidence, priceSignalRole | At least one quote per searched pair | L0 |
| gross_gate → landed_cost | Candidate-scoped pricing assessment, revenue basis, confidence gates | grossMarginBps, selected quote | L0 |
| landed_cost → net_gate | `LandedCostEstimate` anchored to candidate + quote | netMarginLowPct/BasePct/HighPct, blockers | L0 |
| net_gate → manifest | Process-level rollup with coverage metrics | Coverage-aware qualification | L1 |

---

## 11. Cross-Layer: Testing Evolution

### Testing Strategy Per Layer

| Layer | Fast Inner Loop | Live Smoke | Benchmark Batch | Release Gate |
|-------|----------------|------------|-----------------|--------------|
| L0 | Extraction parsers, margin math, scoring, data model | One skill + one lot through pipeline | Full gold-set corpus vs human labels | All skills, full pipeline, extended corpus |
| L1 | + Rust unit tests (adapter, PTY) | + Real skill session via daemon | + 10 consecutive batch runs | + Process supervision stress test |
| L2 | + Desktop component tests | + Desktop E2E flow | + Operator runs daily for 1 week | + Full desktop + pipeline E2E |
| L3 | + Validation policy logic, relation scoring | + Cron-triggered skill session | + 5 consecutive days unattended | + All fault injection + replay tests |
| L4 | + Sync protocol unit tests | + Push-pull round trip | + Multi-user concurrent access | + Full sync + web portal E2E |
| L5 | + Customs classification, FX, freight | + Multi-source search session | + Decision-grade accuracy vs gold set | + ≥3 categories, ≥2 sources each |
| L6 | + Bid workflow logic | + Portal navigation | + End-to-end bid submission | + Full audit trail verification |

### Test Count Growth

| Layer | New Test Files | Cumulative | Focus |
|-------|---------------|------------|-------|
| L0 | 4 | 4 | Intelligence pipeline unit tests |
| L1 | 7 | 11 | Adapter, PTY, skill linting |
| L2 | 5 | 16 | Agent E2E, desktop E2E |
| L3 | 16 | 32 | Fault injection, replay, validation, drift |
| L4 | 3 | 35 | Sync, schema, web E2E |
| L5 | 1 | 36 | Customs FPO |
| L6 | 0 | 36 | Covered by skill E2E tests |

---

## 12. Cross-Layer: Milestone Mapping

### Correspondence to Research Document Milestones (§11)

| Layer | Research Milestone | Deviation | Justification |
|-------|-------------------|-----------|---------------|
| L0 | M0: Gold Set + Evaluation Harness | None | Direct mapping |
| L1 | M1: Thin Local Runner | None | Direct mapping |
| L2 | M2: Single-Operator Desktop MVP | None | Direct mapping |
| L3 | M3: Operational Hardening | None | Direct mapping — includes dry-run validation |
| L4 | M4: Shared Visibility | None | Direct mapping — includes validation enforcement |
| L5 | M5: Expanded Commercial Intelligence | None | Direct mapping |
| L6 | M6: Request Pipeline Automation | None | Direct mapping |

**No deviations from M0–M6.** The research document's milestone structure was validated through 3 planmaxer expert review iterations and 2 harmonymaxer feedback integrations. The dependency analysis confirms that the milestone ordering correctly respects component dependencies, atomic operation pairs, and the validation-first delivery philosophy.

### Why No Merging or Splitting

- **M3/M4 remain separate:** M3 (dry-run validation) must accumulate a run-manifest corpus and calibrate false positive rates before M4 enables enforcement. Merging them would force simultaneous deployment of enforcement and calibration — a known anti-pattern.
- **M5/M6 remain separate:** Decision-grade landed cost (M5) is independently valuable without bid automation (M6). Merging forces the highest-complexity commercial intelligence and portal-specific bid skills into one layer.
- **M0 remains standalone:** The evaluation harness has ZERO runtime dependencies on the daemon. It must exist before pipeline code to anchor all quality metrics.

---

## 13. Cross-Layer: Stack Sequencing

### Technology Introduction Order

The research document's recommended stack sequencing (§11) is followed without deviation:

```
Layer 0: Skills (SKILL.md playbooks) + TypeScript intelligence
  ↓
Layer 1: + Thin Rust (daemon binary, PTY, process supervision)
  ↓
Layer 2: + Desktop UI (Tauri shell) + Shared UI (packages/ui)
  ↓
Layer 3: + Operational Rust (cron, resource arbiter, validation)
  ↓
Layer 4: + Supabase (sync engine, projections) + Web (Next.js portal)
  ↓
Layer 5: + Extended intelligence (customs, FX, freight APIs)
  ↓
Layer 6: + Bidding (portal-specific workflow skills)
```

### Rationale

1. **Skills first** — Cheapest to write, fastest to iterate, directly validate intelligence thesis. Run from CLI.
2. **TypeScript intelligence** — Pipeline logic lives here. This is where the product thesis is proven or disproven.
3. **Thin Rust** — Only enough daemon to validate runtime reliability. Not a full platform.
4. **Desktop UI** — Only after the pipeline produces results worth displaying.
5. **Operational Rust** — Hardening for daily unattended operation.
6. **Supabase + Web** — Only after a single operator has validated daily use.
7. **Extended intelligence** — Decision-grade accuracy for broader categories.
8. **Bidding** — Portal automation after the full intelligence stack is proven.

### Package Dependency Graph

```
packages/contracts-schema (L0)
  ← packages/intelligence (L0)
  ← packages/manifests (L2)
  ← packages/ui (L2)
  ← packages/bidding (L6)
  ← apps/desktop (L2)
  ← apps/web (L4)
  ← crates/daemon (L1, via SQLite schema alignment)
```

---

## Appendix: Component Activation Sequence

For quick reference, when each major component becomes active:

| Component | Layer | Mode |
|-----------|-------|------|
| Skill directory standard | L0 | Full |
| TypeScript intelligence pipeline | L0 | Screening-grade |
| Evaluation harness | L0 | Full |
| Gold-set lot corpus | L0 | Initial (50–100 lots) |
| Rust daemon binary | L1 | Thin (launch + capture + supervise) |
| Agent adapter (PTY + contract) | L1 | Full |
| Process-group supervision | L1 | Full |
| Run manifest writer | L1 | Full (with machine-authored fallback) |
| Error manifest system | L1 | Full |
| Stage tracking (`stage_run_id` + heads) | L1 | Full |
| Tauri desktop shell | L2 | Full control panel |
| QueryService (SQLite reads) | L2 | Full |
| DaemonBridge (daemon commands) | L2 | Full |
| Live session streaming | L2 | Full |
| Shared UI components | L2 | Full |
| Cron scheduler | L3 | Full |
| Resource arbiter | L3 | Full |
| PTY hardening | L3 | Full |
| Relation resolution | L3 | Full |
| Skill promotion flow | L3 | Full |
| Source health monitoring | L3 | Full |
| Validation policy engine | L3 | Dry-run |
| Validation executor | L3 | Dry-run |
| Base validator skill | L3 | Dry-run |
| Sync engine (push) | L4 | Full |
| Sync engine (pull) | L4 | Full |
| Supabase projections | L4 | Full |
| Web viewing portal | L4 | Full |
| Validation enforcement | L4 | Full |
| Domain validator overlays | L4 | Full |
| Full customs engine | L5 | Decision-grade |
| Full FX engine | L5 | Full |
| Full freight engine | L5 | Full |
| Multi-source marketplace search | L5 | Full |
| Bid workflow automation | L6 | Full |

# Layer 3 — Operational Hardening

> **Authority:** This document is the execution-grade specification for Layer 3. An implementation agent should be able to build this layer from this document alone without reopening the research document.
>
> **Milestone:** M3
> **Dependencies:** Layer 0 (intelligence pipeline, gold set for validator calibration), Layer 1 (daemon runtime, agent adapter), Layer 2 (desktop shell, pipeline data model)
> **Exit criteria:** System runs unattended for 5 consecutive days with >=95% skill success rate and no manual intervention.

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

### 1.3 New Daemon Components (12 files)

| File | Module | Responsibility |
|------|--------|---------------|
| `scheduler/cron.rs` | Cron Scheduler | Schedule and enqueue agent work based on cron definitions. Runs locally in daemon. Binds explicit skill id + version at enqueue time (no late-bound resolution). Catchup sweeps on startup for overdue jobs. |
| `sources/registry.rs` | Source Registry | Typed source definitions with version tracking, skill binding, schedule, and health/cooldown status. Static definitions in local SQLite `sources`; mutable state in `source_runtime_state`. |
| `jobs/claim.rs` | Job Claimer | Concurrent job claiming with lease management. Multiple worker threads can claim jobs atomically. Lease expiry prevents stuck claims. |
| `pty/reaper.rs` | Session Reaper | Descendant process-tree cleanup after session completion/failure. Kill escalation: in-band interrupt -> SIGTERM group -> grace period -> SIGKILL group -> orphan sweep. Records cleanup verdict. On daemon startup, sweeps for orphaned sessions from prior runs. |
| `pty/hardening.rs` | PTY Hardening | fd hygiene (aggressively close inherited fds before exec), EIO handling (treat EIO on PTY master read after slave closes as normal termination), terminal geometry (fix PTY size for headless runs, e.g., 120x40). |
| `agents/arbiter.rs` | Resource Arbiter | Admit agent jobs using typed resource budgets. Three quota types with admission priority ordering. |
| `db/checkpoints.rs` | Checkpoint Store | Checkpoint persistence for partial progress. Validates checkpoint schema, stores to artifact directory, updates `workflow_jobs` with checkpoint reference. |
| `tracing/structured.rs` | Structured Tracing | Control events (spawn, signal, exit, reap, cleanup verdict) + protocol events (vendor JSONL/JSON parsed into daemon events, validated checkpoints). These drive workflow state. |
| `tracing/forensic.rs` | Forensic Capture | Raw byte capture (PTY bytes or stdout/stderr) as compressed immutable artifacts. These are for debugging and replay, not control flow. |
| `validation/policy.rs` | Validation Policy Engine | Determine whether a completed job requires semantic validation based on policy mode, promotion stage, trigger conditions, and risk factors. |
| `validation/executor.rs` | Validation Executor | Spawn validator sub-jobs via Job Runner. Compose validator kickoff prompt with run manifest, result contract sample, runtime rubric, OQG signals, and drift deltas. |
| `validation/manifest_writer.rs` | Run Manifest Writer | Produce hybrid-authored run manifest after every skill run. Agent narrative + daemon evidence appendix. Fallback machine-authored manifest if agent fails to narrate. |

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
4. Supplier enrichment (most sessions, most resource-intensive)
5. Manifest generation (lightweight)
6. Bid automation (lowest — rare, operator-initiated)

Skill-level `max_concurrency` can lower these ceilings but never raise them.

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

**Catchup sweeps:** When daemon starts after extended downtime, it checks for overdue scheduled jobs and runs them in priority order, not all at once. Resource arbiter enforces concurrency limits during catchup.

**Source circuit breaker interaction:** If `source_runtime_state` shows an open circuit or active cooldown for a source, the scheduler suppresses normal ingestion and only allows `source-health/*` or `auth-refresh/*` jobs for that source.

### 1.6 PTY Hardening

| Hardening | What It Does | Why It Matters |
|-----------|-------------|----------------|
| **fd hygiene** | Aggressively close inherited fds before exec | macOS Big Sur/Cocoa and GNOME/Mutter can leak fds into child processes, causing intermittent failures |
| **EIO handling** | Treat EIO on PTY master read (after slave closes) as normal termination | Without this, EIO after slave close looks like an internal error |
| **Terminal geometry** | Fix PTY size for headless runs (e.g., 120x40) | CLI formatting and browser/tool behavior must be stable. Only resize when operator attaches for live viewing |
| **Signal handling** | Use signals only as wakeups, then drain waitpid/wait loops until nothing left to reap | Signals may be coalesced by Tokio's Unix signal streams |

### 1.7 Host Environment Awareness

- **Sleep/wake:** If host suspends during active session, assume browser-driven PTY sessions are tainted. Use monotonic time for stall detection. Emit `HOST_SLEEP_INTERRUPTED` if suspend gap exceeds configurable threshold.
- **Missed runs:** On daemon start after extended downtime, catchup sweep checks for overdue jobs. Jobs executed in priority order via resource arbiter.

### 1.8 Shared Library Additions

| Package | File | Purpose |
|---------|------|---------|
| `packages/intelligence` | `src/relations/resolve-process-relations.ts` | Relation resolution engine (see §3) |
| `packages/manifests` | `render-error-manifest.ts` | Error manifest Markdown rendering from error context |
| `packages/manifests` | `error.md.ts` | Error manifest Markdown template: error chain, job context, skill info, suggested resolution |

### 1.9 New Skills (Layer 3)

**4 operational skills:**

| Skill | Purpose | Profile |
|-------|---------|---------|
| `skills/auth-refresh/alibaba/` | Re-authenticate Alibaba session when auth expires | `pty_browser` |
| `skills/auth-refresh/1688/` | Re-authenticate 1688 session when auth expires | `pty_browser` |
| `skills/source-health/fts/` | Canary probe: verify FTS page fingerprint + basic extraction | `pty_browser` |
| `skills/source-health/alibaba/` | Canary probe: verify Alibaba page fingerprint + auth + basic search | `pty_browser` |

**6 validator-base files** (base semantic validator skill):

| File | Purpose |
|------|---------|
| `skills/validation/base/SKILL.md` | Base validator playbook — generic semantic quality checks applicable to any skill |
| `skills/validation/base/result.schema.json` | Validator verdict schema |
| `skills/validation/base/checkpoint.schema.json` | N/A — validators don't checkpoint |
| `skills/validation/base/canary.inputs.jsonl` | Known-good and known-bad run manifests for canary testing |
| `skills/validation/base/golden.assertions.yaml` | Expected verdicts for canary inputs |
| `skills/validation/base/validator-rubric.yaml` | Distilled runtime rubric — skill semantic invariants for live prompts (not raw golden assertions) |

### 1.10 What's Stubbed in This Layer

| Feature | Layer 3 Stub | Full Implementation |
|---------|-------------|-------------------|
| Validation enforcement | Dry-run only — verdicts logged, not enforced | Layer 4: enforcement mode |
| Domain-specific validators | Base validator only (generic semantic quality) | Layer 4: 4 domain overlays |
| Validator calibration | Initial calibration against gold-set manifests | Layer 4: formal calibration harness with 3 corpora |
| Sync | Tracking columns populated, no outbox/push/pull | Layer 4: full sync engine |
| Multi-user | Single operator only | Layer 4: multi-user via web portal |

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

**YAML frontmatter — agent-claimed fields** (self-reported, `agent_claimed_*` prefix):

| Field | Purpose |
|-------|---------|
| `manifest_version` | Format version |
| `skill_id`, `skill_version` | Which skill ran |
| `job_id`, `session_id` | Execution identity |
| `execution_mode` | `pty` or `contract` |
| `started_at`, `completed_at` | Timing |
| `outcome_class` | One of 6 outcome classes (see §2.5) |
| `items_processed`, `items_skipped`, `items_failed` | Self-reported counts |
| `agent_claimed_pages_visited` | Navigation count |
| `agent_claimed_source_url` | Target URL |
| `agent_claimed_fingerprint_state` | `matched` / `mismatched` / `unknown` |
| `agent_claimed_locale_state` | `matched` / `mismatched` / `unknown` |
| `agent_claimed_currency_state` | `matched` / `mismatched` / `unknown` |

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

**Target size:** < 50 KB per manifest. Summary patterns and anomalies, don't enumerate every item.

### 2.2 Layer 3 Validation Additions to Run Manifests

Beyond the base contract above, Layer 3 adds these validator-relevant fields to the daemon evidence appendix:

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

The unit of enforcement readiness is **worker skill x source x worker version x validator overlay version** — not the whole system. A supplier-enrichment validator can be enforcement-ready while a document-acquisition validator stays dry-run.

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

### 2.4 Validation Executor

Spawns validator sub-jobs via the Job Runner. Links validator job to worker job via `parent_job_id`.

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
- Base validator (`skills/validation/base/`) checks generic quality
- Domain overlay (e.g., `skills/validation/procurement-ingestion/`) adds domain-specific criteria
- Source-specific overrides via co-located `validator-overlay.md` when shared overlay insufficient

**Verdict output:**

```json
{
  "verdict": "approved | rejected | flagged",
  "basis_sufficiency": "sufficient | partial | insufficient",
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
| `basis_sufficiency: insufficient` | Log as `VALIDATION_FLAGGED_INSUFFICIENT_BASIS` | Advance with `unverified` trust state |
| Validator crash/timeout | Fail-open: advance as if skipped | Fail-open: advance with `validation_skipped_validator_error` |

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

### 2.7 Fail-Open Semantics (Constitutional)

**Validator failures never block the pipeline.** This is a constitutional invariant — not a temporary expedient.

If the validator crashes, times out, or produces an invalid verdict:
- Worker results advance as if validation were skipped
- `validation_skipped_validator_error` flag set on the job
- Error manifest created for the validator failure itself
- Validation system self-monitors its own failure rate

The rationale: a broken validator is worse than no validator. The pipeline must never stall because the quality-checking subsystem fails. The validator adds confidence; it does not gate availability.

### 2.8 Base Validator Skill

**Directory:** `skills/validation/base/`

**Files:** `SKILL.md`, `result.schema.json`, `checkpoint.schema.json`, `canary.inputs.jsonl`, `golden.assertions.yaml`, `validator-rubric.yaml`, `references/manifest-corpus-index.md`

**What the base validator checks:**

1. **Manifest completeness:** All required frontmatter fields present, narrative sections non-empty, daemon appendix present
2. **Agent-vs-daemon agreement:** Cross-check `agent_claimed_*` against `daemon_observed_*` — discrepancies in fingerprint state, locale, currency, item count are high-signal
3. **Result contract consistency:** Manifest claims consistent with result contract sample (item counts, value ranges, source URLs)
4. **Narrative plausibility:** Steps described match the observed URL chain and page-kind sequence
5. **`basis_sufficiency` assessment:** Can the validator render a confident verdict? If the daemon appendix is thin (e.g., `no_baseline` fingerprint, missing OQG signals), the validator declares `basis_sufficiency: insufficient` rather than guessing
6. **Drift detection:** Current run's fill rate, fallback depth, and error patterns compared to recent runs via structured drift deltas

**What the base validator does NOT check:**
- Domain-specific business rules (those come from overlay packages)
- Result contract schema compliance (already checked by OQG)
- Raw forensic bytes (too large, too noisy)

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

**Override feedback loop:** Every operator override of a validator verdict (operator approves what validator rejected, or vice versa) becomes a calibration case for the next calibration cycle.

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

### 3.3 New Skills Introduced in Layer 3

**Validator base skill (6 files):**

| File | Purpose |
|------|---------|
| `skills/validation/base/SKILL.md` | Generic run manifest quality checks — narrative consistency, agent-claimed vs daemon-observed cross-check, result contract plausibility, `basis_sufficiency` assessment |
| `skills/validation/base/result.schema.json` | Verdict output schema (`approved`/`rejected`/`flagged` with metadata) |
| `skills/validation/base/checkpoint.schema.json` | Checkpoint schema for multi-segment validation |
| `skills/validation/base/canary.inputs.jsonl` | Known-good and known-bad manifests for smoke testing |
| `skills/validation/base/golden.assertions.yaml` | Invariant assertions (mutated-bad must be caught, clean-real must pass) |
| `skills/validation/base/validator-rubric.yaml` | Distilled runtime rubric — skill semantic invariants consumed by the Validation Executor at runtime (NOT raw golden assertions injected into live prompts) |

**Operational skills (4):**

| Skill | Path | Purpose |
|-------|------|---------|
| Auth refresh (Alibaba) | `skills/auth-refresh/alibaba/` | Establish/refresh authenticated session state for Alibaba.com. Publishes auth-state handle for enrichment skills. |
| Auth refresh (1688) | `skills/auth-refresh/1688/` | Establish/refresh authenticated session state for 1688.com. Publishes auth-state handle. |
| Source health (FTS) | `skills/source-health/fts/` | Pre-flight canary: verify FTS reachability, page fingerprint, no challenge markers, OCDS endpoint responsive. |
| Source health (Alibaba) | `skills/source-health/alibaba/` | Pre-flight canary: verify Alibaba reachability, geo-correctness, auth state, page fingerprint, challenge rate. |

Auth-refresh skills do one thing: establish authenticated state and publish an auth-state handle. Source-health canaries run before expensive scheduled jobs to detect whether a source is reachable and not actively challenging.

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

**Trigger criteria:** Circuit opens after N consecutive challenge-class failures (default N=3) within a time window. Challenge-class failures include: `AGENT_AUTH_CHALLENGE`, `SOURCE_BLOCKED`, `SOURCE_RATE_LIMITED`.

**State persisted in:** `source_runtime_state` table (local-only — see §4 for DDL).

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

4. **Edge creation** — `related_not_merged` pairs get a `ProcessRelation` edge enabling cache sharing of search results and listing evidence across the cluster.

5. **Alias management** — when a previously published process ID is retired by a merge, write a `process_aliases` redirect so desktop/web deep links remain stable.

**New tables:** `process_relations` (push-owned) and `process_aliases` (push-owned). See §4 for full DDL.

**Conservative thresholds:** Default thresholds are deliberately conservative. False merges are harder to undo than false separations. The operator can manually merge `possible_duplicate` pairs.

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

---

## 4. Schema, Telemetry & Observability

### 4.1 New Tables (Layer 3)

All DDL in Postgres syntax (SQLite mirrors with type substitutions per L0 §2.1).

#### validation_verdicts (local-only)

Full validator verdict reasoning — never synced. Redacted summary pushed as `validation_verdict_summaries` in Layer 4.

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
  last_health_state text not null default 'green',
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

#### process_relations (push-owned)

```sql
create table process_relations (
  id uuid primary key default gen_random_uuid(),
  process_id_a uuid not null references procurement_processes(id),
  process_id_b uuid not null references procurement_processes(id),
  relation_type text not null,                  -- merged_same_process, related_not_merged, possible_duplicate, separate
  confidence_bps integer not null,              -- basis points (0-10000)
  evidence jsonb,
  owner_id text not null,
  row_version integer not null default 1,
  last_local_seq integer,
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
  row_version integer not null default 1,
  last_local_seq integer,
  updated_at timestamptz not null default now(),
  created_at timestamptz not null default now()
);
```

### 4.2 Modified Tables

| Table | Field | Change in L3 |
|-------|-------|-------------|
| `workflow_jobs` | `parent_job_id` | Now actively used — links validator sub-jobs to worker jobs |
| `workflow_jobs` | `validation_required` | Now actively set by Validation Policy Engine (still defaults false) |
| `workflow_jobs` | `validation_verdict_id` | Now populated after validation completes |
| `agent_session_logs` | `run_manifest_path` | Now actively populated by Run Manifest Writer |
| `agent_session_logs` | `run_manifest_hash` | Now actively populated (SHA-256 of manifest) |

### 4.3 Authority Classification (L3 Additions)

| Table | Authority | Rationale |
|-------|-----------|-----------|
| `validation_verdicts` | Local-only | Full verdict reasoning too large/sensitive for sync |
| `source_runtime_state` | Local-only | Per-daemon operational state |
| `listing_evidence` | Local-only | Cache layer, not authoritative data |
| `process_relations` | Push-owned | Shared visibility in Layer 4 |
| `process_aliases` | Push-owned | Stable deep links across desktop/web |

### 4.4 Four-Layer Observability Model

| Layer | Content | Storage | Purpose |
|-------|---------|---------|---------|
| **Control events** | Job lifecycle, stage transitions, decision points, resource grants/denials, circuit breaker state changes | Structured JSON logs (tracing crate) | Operational monitoring, alerting |
| **Protocol events** | Agent communication, skill loading, checkpoint progression, result contract events, adapter events | Structured JSON in `agent_session_logs.structured_trace` | Debugging, replay, performance |
| **Forensic bytes** | Raw PTY output OR contract-mode stdout/stderr | Compressed immutable artifacts at `terminal_output_path` | Post-incident analysis, deterministic replay |
| **Narrative** | Run manifests — agent-written narrative + daemon evidence appendix | Immutable artifacts at `run_manifest_path` | Semantic validation, operator review |

All four layers capture data from both PTY and contract-mode sessions.

### 4.5 Skill-Level Telemetry (13 Metrics)

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

### 4.6 Validation Telemetry (Dry-Run)

5 validation metrics for Layer 3 (expanded to 13 in Layer 4 enforcement):

| Metric | Definition | Unit |
|--------|-----------|------|
| `validator_approval_rate_bps` | Approved / total validated | bps |
| `validator_rejection_rate_bps` | Rejected / total validated | bps |
| `validator_flag_rate_bps` | Flagged / total validated | bps |
| `validator_cost_per_run_pence` | Model cost per validation | integer pence |
| `operator_override_rate_bps` | Overrides / total rejections (false positive proxy) | bps |

---

## 5. Testing & Acceptance

### 5.1 Definition of Done

| # | Criterion | Verification |
|---|----------|-------------|
| 1 | Cron scheduler runs on configured schedule with explicit skill binding | `cron_scheduler.rs` |
| 2 | Batched ingestion: scheduled FTS + CF runs complete without manual intervention | 24h unattended cycle |
| 3 | Relation resolution correctly classifies (merged/related/possible_duplicate/separate) | `resolve-process-relations.test.ts` |
| 4 | PTY hardening: fd hygiene, EIO handling, terminal geometry stability | `fault_pty_edge_cases.rs` |
| 5 | Host environment awareness: sleep/wake detection, catchup sweeps | `fault_host_events.rs` |
| 6 | Resource arbiter enforces typed quotas (browser slots, agent slots, source lanes) | Load test |
| 7 | Skill promotion flow: draft -> candidate -> canary -> shadow -> stable | `skill_golden_journeys.rs` + `skill_shadow_compare.rs` |
| 8 | Source-health canaries and auth-refresh skills operational | Canary test per source |
| 9 | Worker-validator dry-run: manifests generated, base validator renders verdicts, logged without enforcement | `validation_dry_run.rs` + `validation_golden_verdicts.rs` |
| 10 | Error manifest control-panel view displays failures with resolution context | Manual desktop verification |
| 11 | System runs unattended for 5 consecutive days, >=95% success rate | Monitoring + operator sign-off |

### 5.2 Test File Listing (16 New, Cumulative 32)

#### Fault Injection (4)

| File | Coverage |
|------|---------|
| `fault_process_cleanup.rs` | Orphaned grandchildren, dropped handle, cleanup timeout, leaked detection, pgid kill escalation |
| `fault_pty_edge_cases.rs` | PTY EOF/EIO, fd leak scenarios, terminal geometry changes mid-session |
| `fault_host_events.rs` | Host sleep/monotonic gap, resource exhaustion, concurrent pressure, catchup sweep |
| `fault_partial_failure.rs` | Partial checkpoint then crash, result contract failure mid-stream, additive partial commit |

#### Deterministic Replay (2)

| File | Coverage |
|------|---------|
| `replay_pty_sessions.rs` | Replay captured PTY byte streams against adapter parser (escape sequences, split reads) |
| `replay_contract_sessions.rs` | Replay captured contract-mode sessions (structured event parsing, checkpoint extraction) |

#### Scheduling & Jobs (2)

| File | Coverage |
|------|---------|
| `cron_scheduler.rs` | Cron expression parsing, schedule evaluation, skill binding, catchup sweep, timezone/DST |
| `job_claim.rs` | Concurrent claiming, lease expiry, double-claim prevention, crash recovery |

#### Skill Lifecycle (3)

| File | Coverage |
|------|---------|
| `skill_golden_journeys.rs` | Invariant-based E2E per skill family against known inputs/golden assertions |
| `skill_shadow_compare.rs` | Quality diff between stable and candidate: fill-rate, duplicate-rate, cost, validator approval |
| `skill_drift_monitor.rs` | Page fingerprint trending, fallback depth, fill-rate trending |

#### Validation (4)

| File | Coverage |
|------|---------|
| `validation_policy.rs` | All policy mode x trigger combinations: always/sample/trigger/skip, anomaly triggers |
| `validation_executor.rs` | Sub-job spawning, kickoff prompt composition, verdict parsing, basis_sufficiency |
| `validation_dry_run.rs` | Dry-run verdicts logged, pipeline NOT blocked, no outbox, no stage head advancement |
| `validation_golden_verdicts.rs` | Known-good -> approved, known-bad -> rejected, ambiguous -> flagged |

#### Relations (1)

| File | Coverage |
|------|---------|
| `resolve-process-relations.test.ts` | Relation scoring, hard blockers, four-way classification, confidence thresholds |

### 5.3 Test Scenarios

| Scenario | Expected Behavior |
|----------|------------------|
| Cron fires after daemon slept through scheduled time | Catchup sweep detects, runs overdue jobs |
| Two FTS jobs scheduled concurrently | Resource arbiter grants one browser slot, second queues |
| Candidate skill has worse fill-rate than stable | Shadow comparison fails, candidate NOT promoted |
| Source-health canary detects FTS layout change | Source enters cooldown (circuit breaker), error manifest created |
| Validator rejects supplier enrichment manifest | Dry-run: logged, NOT enforced, worker results advance |
| Validator crashes mid-validation | Fail-open: worker results advance, `VALIDATION_SKIPPED_VALIDATOR_ERROR` |
| Relation resolution: same OCID but different buyers | `possible_duplicate` (NOT merged) — buyer mismatch is hard blocker |
| Host enters low-disk state during session | Resource arbiter blocks new starts, existing complete, alert logged |
| PTY receives EIO (terminal disconnected) | Graceful handling, session transitions to terminating, verdict recorded |
| Auth-refresh detects expired Alibaba session | Re-authenticates, stores fresh state, subsequent jobs use new auth |

### 5.4 Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Validator false positive rate too high for future enforcement | Medium | High | Dry-run calibration in L3; golden verdicts regression; track rejection rate per skill |
| Cron edge cases (timezone, DST, clock drift) | Low | Medium | Catchup sweeps; monotonic clock for internal timers |
| Relation resolution false merges | Medium | Medium | Conservative thresholds; hard blockers; operator can unmerge; `possible_duplicate` for borderline |
| Source changes break multiple skills simultaneously | Medium | High | Source-health canaries detect before production; per-source circuit breaker isolates failures |

---

## 6. Open Questions, Forward Compatibility & Dependencies

### 6.1 Open Questions

**§10 Further Research #4: Expiry behavior for in-flight jobs**
- Should expiry (`availabilityStatus = expired`) cancel in-flight jobs or let them complete?
- Current design: let complete, block downstream progression.
- **L3 impact:** Cron scheduler may trigger jobs for notices expiring mid-pipeline.

**§10 Further Research #6: Late process merge reconciliation**
- How should post-enrichment merges reconcile downstream data?
- `processClusterId` proposed but not designed.
- **L3 impact:** Relation engine creates edges, but post-enrichment merges need cluster-aware rollups.

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

| Provision | What Exists in L3 | Target Layer |
|-----------|-------------------|-------------|
| Validation policy engine ready for enforcement toggle | Evaluates but always dry-run=true | Layer 4 |
| Run manifest corpus accumulates | Every skill run produces manifest | Layer 4 calibration |
| `validation_verdicts` full schema from creation | Includes enforcement_eligible, basis_sufficiency | Layer 4 projections |
| Source circuit breaker state accessible | Feeds into risk-stratified sampling | Layer 4 |
| Skill promotion includes validator approval rate | Shadow comparison metric | Layer 4 promotion gate |

### 6.4 Dependencies

**Layer deps:** L0 (intelligence, gold set), L1 (daemon, adapter), L2 (desktop, pipeline model)

#### Additional Rust Crates

| Crate | Purpose |
|-------|---------|
| `cron` | Cron expression parsing |
| `sysinfo` | Host resource pressure detection |

#### New Skills (10)

| Skill | Purpose |
|-------|---------|
| `auth-refresh/alibaba/` | Alibaba auth state refresh |
| `auth-refresh/1688/` | 1688 auth state refresh |
| `source-health/fts/` | FTS source health canary |
| `source-health/alibaba/` | Alibaba source health canary |
| `validation/base/` (6 files) | Base validator skill with rubric |

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

# Layer 4 — Shared Visibility

> **Authority:** This document is the execution-grade specification for Layer 4. An implementation agent should be able to build this layer from this document alone without reopening the research document.
>
> **Milestone:** M4
> **Dependencies:** Layers 0-3 (all pipeline data, all push-owned tables with sync tracking columns)
> **Exit criteria:** Two users view the same pipeline: desktop operator (read-write) + web viewer (read-only). Sync engine operational. Validation enforcement active.

---

## 1. Overview & Sync Engine

### 1.1 Layer Narrative

Collaboration infrastructure. The sync engine activates, projecting pipeline data to Supabase for the read-only web viewing portal. Operator decisions flow through Supabase CAS. Worker-validator enforcement mode is enabled after dry-run calibration. Two users can now view the same opportunity pipeline — one via desktop (read-write), one via web (read-only).

**Why this layer exists:** Layers 1-3 proved the pipeline works for a single operator on a single machine. Layer 4 makes the pipeline output visible to others and enables shared decision-making. The sync engine is the bridge between local operational authority and shared visibility.

**If we never build Layer 5:** Multi-user visibility works with screening-grade accuracy and single-marketplace pricing. The system is production-ready for the initial category whitelist.

### 1.2 Capability Delivered

Two users view the same pipeline: desktop operator (read-write) and web viewer (read-only). Sync engine pushes pipeline data to Supabase. Operator decisions use compare-and-swap via Supabase. Validator rejections quarantine results in enforcement mode. Domain-specific validator overlays for procurement, supplier enrichment, document acquisition, and product extraction.

### 1.3 Push Protocol (Local -> Supabase)

**Transactional outbox pattern:** From Layer 4 activation, every push-owned mutation creates a `local_outbox` entry in the same SQLite transaction as the data write. No data mutation without outbox entry.

**Push dispatcher contract:**
- Single-in-flight batch (one batch at a time per push stream)
- Identical-envelope retries on failure (same content, same order)
- Batch boundaries never cross `schema_epoch` or `stage_run_id`
- `row_version` ordering within batch (monotonic per row)
- `content_hash` for idempotent deduplication

**Push batch RPC:** `apply_push_batch(batch_json)` — single transactional Supabase RPC. Batch either fully applies or fully rejects. No partial application.

**Outbox entry structure:**

```sql
-- Each outbox entry (from L0 §2 DDL):
-- local_seq (autoincrement), executor_id, entity_kind, entity_id,
-- op (insert/update), row_version, stage_run_id, schema_epoch,
-- payload_json, content_hash, created_at
```

### 1.4 Pull Protocol (Supabase -> Local)

**Cursor-based pull:** Per-domain pull cursors track `remote_change_seq`. Three pull domains:
- `shared_source_configs` — source enablement and configuration
- `shared_company_profile` — org-wide bidder profile
- `operator_decision_heads` — operator dispositions with CAS

**Pull-before-push ordering:** On sync startup, pull remote-owned state FIRST, then classify/coalesce pending intents, then resume push. This ensures cross-authority consistency.

**Pending operator intent reconciliation:** After pull, reconcile `pending_operator_intents` queue:
1. Compare each queued intent against freshly pulled `operator_decision_heads`
2. If intent's `base_remote_revision` matches current remote revision → flush via CAS RPC
3. If revision mismatch → conflict, update local queue status, surface to operator
4. Coalesce multiple intents for same process (latest-per-process)

### 1.5 Compare-and-Swap (CAS) for Operator Decisions

**CAS RPC:** `update_operator_decision(process_id, disposition, base_revision, command_id)`
- `base_remote_revision` checked — mismatch means someone else changed it
- `command_id` for idempotent retry (lost-response handling)
- Conflict returned (not silently overwritten)
- Desktop shows conflict resolution UI

### 1.6 Active-Writer Lease

One daemon per org in MVP. Enforced at push protocol level:
- Daemon registers with `executor_id` on first push
- Second daemon attempting to push → rejected with lease conflict
- Lease heartbeat via `viewer_sync_status` updates

### 1.7 Schema Epoch Guard

`schema_epoch` (integer) on every push batch and in `PRAGMA user_version`:
- If batch epoch != remote epoch → sync pauses
- Local operation continues uninterrupted
- Operator/admin must resolve epoch mismatch before sync resumes
- Prevents schema-mismatched data from corrupting the remote projection

### 1.8 Degraded Mode Behavior

| Sync State | Desktop | Web |
|-----------|---------|-----|
| Healthy | Full read-write-trigger | Full read-only with live Broadcast updates |
| Degraded (push failing) | Full local operation; push queues in outbox | Stale data (amber indicator); Broadcast stopped |
| Offline (Supabase unreachable) | Full local operation; browse-only for decisions | Red staleness indicator; last-known data |

Desktop continues operating on local SQLite regardless of sync state. Web reflects last-pushed data with staleness indicator.

### 1.9 Invariants Enforced

| Invariant | Type | Enforcement |
|-----------|------|-------------|
| Push batch atomicity | Constitutional | Single transactional RPC, not individual upserts |
| Outbox + data in same SQLite transaction | Constitutional | Every push-owned mutation creates outbox entry atomically |
| CAS for operator decisions | Constitutional | `base_remote_revision` checked; mismatch -> conflict |
| Active-writer lease | Constitutional | One daemon per org at push protocol level |
| `schema_epoch` on every batch | Constitutional | Incompatible epoch -> sync pauses |
| Batch boundaries never cross `stage_run_id` | Constitutional | Push dispatcher enforces |
| Single-in-flight batch | Constitutional | One batch at a time per push stream |
| Validation enforcement | Operational | Rejections quarantine results after dry-run calibration |
| RLS on all Supabase tables | Constitutional | Policy matrix at database boundary |
| Pull-before-push ordering | Constitutional | Pull remote state FIRST on startup |

### 1.10 Components Touched

| Component | Action | Files |
|-----------|--------|-------|
| Database | Create | `0002_sqlite_state_sync.sql` (SQLite sync tables), 6 Supabase migrations, `infra/supabase/config.toml` |
| `crates/daemon` | Create | `src/db/sync.rs` (push-pull sync engine) |
| Skills | Create | 4 domain validator overlays |
| `apps/web` | Create | 7 files (full Next.js viewing portal) |

---

## 2. Supabase Schema, RLS & Remote-Owned Tables

> **Ownership:** This section owns the Supabase-side schema: projection tables, summary tables, remote-owned tables, junction tables, RLS policies, and migration listing. Push/pull protocol mechanics are owned by §1 (Sync Engine). SQLite schema is owned by L0 §2 and L1 §3.

### 2.1 Supabase Projection Tables (Push-Owned)

All 18 push-owned SQLite tables get Supabase counterparts. These are **push-only projections** — the daemon pushes via `apply_push_batch(batch_json)` RPC. The local SQLite copy is authoritative; Supabase is a read-only mirror for the web portal.

| # | Table | Scope | Notes |
|---|-------|-------|-------|
| 1 | `procurement_processes` | Process | Canonical process entity |
| 2 | `contract_notices` | Notice | Source-specific notice records |
| 3 | `notice_lots` | Lot | Per-notice lot decomposition |
| 4 | `tender_document_sets` | Process | Acquired document sets |
| 5 | `qualification_runs` | Lot | Per-lot qualification results |
| 6 | `product_candidates` | Lot | Extracted product candidates |
| 7 | `search_profiles` | Candidate | Structured search profiles |
| 8 | `search_attempts` | Candidate | Per-wave search execution records |
| 9 | `supplier_quotes` | Candidate | Marketplace/RFQ quotes |
| 10 | `candidate_pricing_assessments` | Candidate | Gross margin gate results |
| 11 | `landed_cost_estimates` | Candidate-quote | Per-candidate-quote cost estimates |
| 12 | `manifests` | Process | Generated opportunity manifests |
| 13 | `pipeline_assessments` | Process | Process-level pipeline rollup |
| 14 | `bid_workflow_states` | Process | Bid automation state (dormant until L6) |
| 15 | `process_stage_status` | Scoped | Per-stage execution records |
| 16 | `process_stage_heads` | Scoped | Current stage head per scope |
| 17 | `process_relations` | Process | Inter-process relationship edges |
| 18 | `process_aliases` | Process | Merged-process alias redirects |

Schema for these tables mirrors the SQLite DDL from L0 §2 with Postgres type substitutions (TEXT → text, INTEGER → integer/bigint, etc.). All push-owned columns (`owner_id`, `row_version`, `last_local_seq`, `stage_run_id`, `created_at`, `updated_at`) are present.

### 2.2 Summary Tables (Push-Owned Projections)

Redacted/summarized views of local-only tables, pushed to Supabase for web portal visibility.

| Table | Source | Purpose | Key Fields |
|-------|--------|---------|-----------|
| `workflow_job_summaries` | `workflow_jobs` (local-only) | Redacted job status for web | `job_id`, `job_kind`, `status`, `skill_path`, `started_at`, `completed_at`, `error_type` (no full context) |
| `error_manifest_summaries` | `error_manifests` (local-only) | Redacted error context for web | `manifest_id`, `job_id`, `error_type`, `error_code`, `incident_signature`, `created_at` (no full context_json or markdown) |
| `validation_verdict_summaries` | `validation_verdicts` (local-only) | Verdict status for web | `verdict_id`, `worker_job_id`, `verdict`, `basis_sufficiency`, `enforcement_eligible`, `created_at` (no full reasoning) |
| `viewer_sync_status` | Daemon sync state | Sync freshness indicator | `sync_health` (healthy/degraded/offline), `last_push_success_at`, `pending_outbox_count`, `schema_epoch` |
| `executors` | `executors` (push-owned) | Daemon heartbeat summary | `executor_id`, `heartbeat_at`, `daemon_version`, `started_at` |

### 2.3 Remote-Owned Tables

These tables originate in Supabase and are pulled to local. The daemon does NOT write to these — it only reads (pulls).

#### shared_company_profile

Org-wide bidder profile used by the bid workflow (Layer 6).

```sql
CREATE TABLE shared_company_profile (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  org_id text NOT NULL,
  company_name text NOT NULL,
  registration_number text,
  vat_number text,
  address_json jsonb,
  contact_json jsonb,
  bid_profile_json jsonb,       -- portal-specific form field defaults
  remote_change_seq bigint NOT NULL DEFAULT 1,
  remote_revision bigint NOT NULL DEFAULT 1,
  deleted_at timestamptz,       -- tombstone for soft-delete
  created_at timestamptz NOT NULL DEFAULT now(),
  updated_at timestamptz NOT NULL DEFAULT now()
);
```

#### shared_source_configs

Source enablement and configuration. Controls which procurement/supplier sources are active and their scheduling parameters.

```sql
CREATE TABLE shared_source_configs (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  org_id text NOT NULL,
  source_id text NOT NULL UNIQUE,
  source_type text NOT NULL CHECK (source_type IN ('procurement', 'supplier')),
  display_name text NOT NULL,
  enabled boolean NOT NULL DEFAULT true,
  skill_path text NOT NULL,                       -- e.g., 'procurement-ingestion/fts'
  schedule_cron text,                              -- cron expression for scheduled runs
  config_json jsonb,                               -- source-specific configuration
  remote_change_seq bigint NOT NULL DEFAULT 1,
  remote_revision bigint NOT NULL DEFAULT 1,
  deleted_at timestamptz,
  created_at timestamptz NOT NULL DEFAULT now(),
  updated_at timestamptz NOT NULL DEFAULT now()
);
```

#### operator_decision_heads

Operator dispositions with compare-and-swap semantics. This is the **remote-authoritative** decision table — before Layer 4, operator decisions exist only as local intents in `pending_operator_intents`.

```sql
CREATE TABLE operator_decision_heads (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  org_id text NOT NULL,
  process_id text NOT NULL UNIQUE,
  disposition text NOT NULL CHECK (disposition IN ('pending', 'watching', 'approved', 'rejected')),
  reviewed_basis_hash text,                        -- hash of candidates/quotes/estimates reviewed
  reviewed_manifest_hash text,                     -- hash of manifest read
  reviewed_at_stage_run_id text,                   -- stage run decision was based on
  decision_stale boolean NOT NULL DEFAULT false,    -- set true when upstream changes basis
  operator_id text,                                 -- who made the decision
  command_id text,                                  -- idempotency key for CAS (handles lost-response retries)
  remote_change_seq bigint NOT NULL DEFAULT 1,
  remote_revision bigint NOT NULL DEFAULT 1,
  deleted_at timestamptz,
  created_at timestamptz NOT NULL DEFAULT now(),
  updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX idx_decisions_process ON operator_decision_heads(process_id);
CREATE INDEX idx_decisions_disposition ON operator_decision_heads(disposition);
```

**CAS semantics:** Updates via `update_operator_decision(process_id, disposition, base_remote_revision, command_id)` RPC. Mismatch on `base_remote_revision` → conflict returned (not silently overwritten). `command_id` provides idempotency for lost-response retries. Direct table UPDATE is blocked — only the RPC path is exposed.

### 2.4 Junction Tables (Supabase)

Mirrored from SQLite for web portal querying:

| Table | Purpose |
|-------|---------|
| `notice_cpv_codes` | Notice ↔ CPV code mapping |
| `product_commodity_codes` | Product ↔ commodity code mapping |
| `lot_cpv_codes` | Lot ↔ CPV code mapping |

### 2.5 RLS Policy Matrix

Row-Level Security policies enforce the authority matrix at the Supabase database boundary. All tables scoped by `org_id` (implicit single-org in MVP).

| Table Category | Tables | `viewer` | `operator` | `admin` | `daemon` |
|---|---|---|---|---|---|
| Push-owned projections | All 18 projection tables | SELECT | SELECT | SELECT | INSERT, UPDATE (via `apply_push_batch` RPC) |
| Push-owned summaries | 5 summary tables + executors | SELECT | SELECT | SELECT | INSERT, UPDATE (via `apply_push_batch` RPC) |
| Remote-owned config | `shared_company_profile`, `shared_source_configs` | SELECT | SELECT | Full CRUD | SELECT (pull) |
| Remote-owned decisions | `operator_decision_heads` | SELECT | SELECT, UPDATE (via CAS RPC) | Full CRUD | SELECT (pull) |
| Sync infrastructure | `schema_metadata` | — | — | SELECT, UPDATE | SELECT, UPDATE |

**Implementation rules:**
- Push-owned tables: daemon writes through `apply_push_batch(batch_json)` RPC that validates `schema_epoch`, checks `row_version` ordering, applies atomically. RLS grants INSERT/UPDATE only to `daemon` role.
- Operator decisions: `update_operator_decision()` RPC enforces CAS. Direct UPDATE blocked.
- All SELECT policies include `WHERE deleted_at IS NULL` for tombstoned tables (excludes soft-deleted rows from portal views).
- **RLS tombstone visibility fix:** The daemon pull query must see tombstoned rows (to propagate deletions locally). Daemon role SELECT policy does NOT filter on `deleted_at IS NULL` for remote-owned tables.
- `viewer` and `operator` roles cannot see sync infrastructure tables.

### 2.6 Supabase Migrations

Six migration files create the full Supabase schema:

| # | Migration | Purpose |
|---|-----------|---------|
| 1 | `supabase/migrations/0001_projection_tables.sql` | CREATE TABLE for all 18 push-owned projection tables + 3 junction tables. Schema mirrors SQLite DDL with Postgres types. |
| 2 | `supabase/migrations/0002_remote_owned_tables.sql` | CREATE TABLE for `shared_company_profile`, `shared_source_configs`, `operator_decision_heads`. |
| 3 | `supabase/migrations/0003_summary_tables.sql` | CREATE TABLE for 5 summary tables: `workflow_job_summaries`, `error_manifest_summaries`, `validation_verdict_summaries`, `viewer_sync_status`, `executors`. |
| 4 | `supabase/migrations/0004_rls_policies.sql` | RLS policies for all 4 roles (viewer, operator, admin, daemon) across all table categories. |
| 5 | `supabase/migrations/0005_sync_rpcs.sql` | `apply_push_batch(batch_json)` transactional RPC + `update_operator_decision(process_id, disposition, base_revision, command_id)` CAS RPC. |
| 6 | `supabase/migrations/0006_schema_metadata.sql` | `schema_metadata` table with `schema_epoch` for compatibility checking. |

**Schema coordination rule:** Both SQLite and Postgres derive from the same canonical logical schema. CI must apply both migrations from scratch, introspect both, and fail if logical schema hash differs.

**SQLite sync migration:** `supabase/migrations/0007_sqlite_sync.sql` — adds `local_outbox`, `sync_batches`, `pull_cursors` tables to SQLite (see §1 for DDL).

### 2.7 Cross-Authority Consistency

Pipeline data crosses authority boundaries at specific points. Each crossing has a classification:

| Crossing Point | Authority Domains | Classification | Provenance Tracking |
|---|---|---|---|
| Pipeline output → operator review | Push-owned → display | **Display** — no authority conflict | `decisionBasisHash` links decision to pipeline snapshot |
| Operator decision → bid workflow | Remote-owned → push-owned automation | **Automation** — must track decision provenance | `reviewedAtStageRunId`, `reviewedBasisHash` carried through |
| Coverage metrics → qualification gate | Push-owned → push-owned control | **Control** — no authority crossing | Internal to pipeline |
| Sync health → UX mode | Push-owned infra → display | **Display** — no authority conflict | `viewer_sync_status` drives 3-state UX |

---

## 3. Web Portal & Validation Enforcement

### 3.1 Web Viewing Portal (`apps/web/` -- 7 files)

The web viewing portal is a **read-only** hosted client. It reads Supabase projection tables and receives live invalidation hints via Supabase Broadcast. It does NOT support mutations, decision capture, agent management, or configuration changes.

**Architecture:**

```text
Next.js / React
  -> Supabase snapshot queries (projection tables)
  -> Supabase Broadcast private channels (invalidation hints)
  -> refetch / patch read-only projections
```

**7 files:**

| File | Responsibility |
|------|---------------|
| `src/app/layout.tsx` | Root layout with global sync status badge (`viewer_sync_status`) |
| `src/app/page.tsx` | Redirect to contracts list |
| `src/app/contracts/page.tsx` | Keyset-paginated opportunity list using Supabase client + `packages/ui` `ContractCardFrame`. Shows "N new/updated" pill on live updates (not live reorder). |
| `src/app/contracts/[id]/page.tsx` | Opportunity detail: candidates, quotes, margins, manifest, errors. Auto-refresh for minor changes; non-destructive refresh chip for major changes. |
| `src/app/status/page.tsx` | Pipeline status, job summaries, error summaries, sync health |
| `src/lib/supabase.ts` | Supabase client initialization, typed query helpers using `packages/contracts-schema` types |
| `src/lib/broadcast.ts` | Supabase Broadcast subscription manager: topic-based invalidation, per-page subscription lifecycle |

**Web-specific data-fetching:** The web uses Supabase client for all data access. It reuses `packages/ui` presentational components (ContractCardFrame, ManifestViewer, PipelineBadge, SyncAgeBadge, ErrorSummaryList) but has its own data-fetching hooks. No SQLite, no daemon bridge, no `invoke()`.

### 3.2 Supabase Broadcast Invalidation

The portal uses **Supabase Broadcast as an invalidation plane**, not as a database subscription. Broadcast payloads are small invalidation hints, not full rows.

**Why Broadcast over Postgres Changes:** Postgres Changes is simpler but scales worse (single-threaded for ordering). Broadcast supports private channels, custom payloads via `realtime.send`, and better security scoping.

**Topic scheme (page-level, not row-level):**

| Page | Topics |
|------|--------|
| `/contracts` | `org:{orgId}:contracts`, `org:{orgId}:sync` |
| `/contracts/[id]` | `process:{processId}:updates`, `org:{orgId}:sync` |
| `/status` | `org:{orgId}:status`, `org:{orgId}:errors`, `org:{orgId}:sync` |

**Invalidation payload types:**

```typescript
type PortalUpdate =
  | {
      event: 'contracts.changed'
      processIds: string[]
      changed: Array<'assessment' | 'decision' | 'manifest' | 'error'>
      maxSeq: number
    }
  | {
      event: 'process.updated'
      processId: string
      changed: Array<'quotes' | 'landed_cost' | 'manifest' | 'jobs' | 'errors'>
      major: boolean
      maxSeq: number
    }
  | {
      event: 'sync.updated'
      syncHealth: 'healthy' | 'degraded' | 'offline'
      lastAckedSeq: number
      lastSuccessAt: string
    }
```

**Emission point:** Daemon emits Broadcast notifications **after** the remote projection batch is successfully applied. Coalesces affected topics so 20 quote upserts produce one notification per topic.

**Per-page behavior:**
- **List page:** Patch badges/counts if in payload. Otherwise invalidate list query. Show "N new/updated opportunities" pill -- do NOT reorder the visible list while user is reading.
- **Detail page (minor):** Auto-refresh the affected section.
- **Detail page (major -- manifest, landed cost, canonical process):** Show non-destructive refresh chip. Do NOT silently swap content under the user.

### 3.3 Degraded-Mode UX -- Three Distinct States

The portal is honest about staleness. Three states, not one badge:

| State | Indicator | UX |
|-------|-----------|-----|
| **Healthy** | Green badge, "Data current" | Full real-time updates via Broadcast. All data fresh. |
| **Degraded** | Amber badge, "Updates delayed -- data may be behind desktop" | Broadcast connected but daemon sync is behind. Data is stale by 90s-5min. All features work but data is not current. |
| **Offline** | Red badge, "Sync offline -- showing last known data" | No sync for >5min or daemon offline. Browse cached projection data only. Broadcast disconnected. |

**Thresholds (from `viewer_sync_status` projection table):**
- Green: < 90 seconds since last successful sync
- Amber: 90s -- 5 min
- Red: > 5 min or daemon offline

The `viewer_sync_status` table (push-owned projection) is updated after each successful remote apply:

```typescript
interface ViewerSyncStatus {
  orgId: string
  syncHealth: 'healthy' | 'degraded' | 'offline'
  lastPushSuccessAt: string       // ISO-8601
  lastPushAttemptAt: string       // ISO-8601
  lastAckedLocalSeq: number       // Latest local_seq successfully pushed
  pendingOutboxCount: number      // Entries waiting to be pushed
  schemaEpoch: number             // Current schema epoch
  daemonVersion: string           // Daemon version string
}
```

**Desktop also shows three states** (upgraded from L2's basic detection):
- **Healthy:** Daemon connected, sync healthy -- full desktop
- **Remote degraded:** Daemon connected, sync behind/offline -- full local ops, sync warning banner
- **Browse-only:** Daemon unavailable -- read-only local browsing, clear banner

### 3.4 Validation Enforcement Mode

Transition from Layer 3 dry-run to production enforcement. In enforcement mode, validator rejections **quarantine** results instead of just logging.

**What changes from dry-run (L3) to enforcement (L4):**

| Aspect | Dry-Run (L3) | Enforcement (L4) |
|--------|-------------|-----------------|
| Rejected results | Advance normally, verdict logged | **Quarantined** as `AGENT_SUSPECT_OUTPUT` |
| Error manifests | Created for logging | Created with operator alert |
| Stage head advancement | Normal (verdict doesn't block) | **Blocked** for rejected results |
| Outbox entries | Normal | **Not created** for quarantined results |
| Pipeline state | Unchanged by verdicts | Quarantined results excluded from rollups |

**Per-slice enforcement readiness:**

The unit of enforcement is **worker skill x source x worker version x validator overlay version** -- not the whole system. Enforcement is enabled independently per slice.

**Prerequisites for enabling enforcement on a slice:**
1. Validator overlay `validator_overlay_compatible_versions` matches current worker behavior version
2. Minimum adjudicated cases threshold met (>=30 rejected/flagged with known ground truth)
3. Minimum audited approvals threshold met (>=60 checked for silent misses)
4. Wilson-bound reject precision lower bound >= 70%
5. Rule-of-three false-pass upper bound <= 5%
6. Coverage of dominant failure families in the challenge bank
7. No active demotion trigger

**Automatic demotion** from enforcement back to dry-run:
- Operator override rate spikes (operators regularly reversing rejections)
- Challenge-bank performance regresses (validator stops catching known-bad)
- Anomaly-conditioned validator error rate rises
- Worker behavior version moves outside overlay's compatible range

Demotion is automatic and immediate. Re-promotion requires meeting calibration thresholds again.

### 3.5 Domain-Specific Validator Overlays (4 skills)

Layer 3 deploys the base validator (generic semantic quality). Layer 4 adds domain-specific overlays that check business rules unique to each skill family.

| Overlay | Directory | Domain Checks |
|---------|-----------|--------------|
| **Procurement ingestion** | `skills/validation/overlay-procurement-ingestion/` | Notice completeness, OCDS field presence, correct source locale, notice-type consistency, expiry date plausibility, CPV code validity |
| **Supplier enrichment** | `skills/validation/overlay-supplier-enrichment/` | Price plausibility (too high/low for category), currency correctness, pack basis consistency, seller diversity, listing freshness, compliance evidence completeness for regulated goods |
| **Document acquisition** | `skills/validation/overlay-document-acquisition/` | Document type identification correctness, download completeness, content-type verification, file size plausibility, document set hash consistency |
| **Product extraction** | `skills/validation/overlay-product-extraction/` | Candidate type plausibility for CPV family, archetype assignment consistency, slot completeness for archetype, quantity/unit plausibility, bundle decomposition correctness |

Each overlay follows the skill directory standard: `SKILL.md` + `result.schema.json` + `checkpoint.schema.json` + `canary.inputs.jsonl` + `golden.assertions.yaml` + `validator-rubric.yaml`.

Overlays are loaded alongside the base validator. The base checks generic quality; the overlay adds domain-specific rules. A skill can declare which overlay to use in its metadata; the Validation Executor loads both.

### 3.6 Formal Calibration Harness

Before enforcement, each slice must pass calibration. The harness uses **three corpora**:

**1. Clean real runs (25-30 per skill family):**
Real successful runs from gold lots and live dry-run traffic. Establish baseline: what does healthy variability look like?

**2. Weird-but-valid runs (15-20 per skill family):**
Unusual but acceptable runs: source layout changed but extraction correct, localized content correctly normalized, partial but honestly narrated data, low-result-count but legitimate.

**3. Mutated bad runs (30-50 per skill family):**
Real good runs mutated into realistic failures:

| Mutation Family | What It Tests |
|----------------|--------------|
| Wrong source/domain in URL chain | Validator catches navigation to wrong site |
| Wrong page kind (search vs category vs detail) | Validator catches page-type confusion |
| Locale/currency swap (claims GBP, evidence EUR) | Validator catches cross-checking failure |
| Pack/unit distortion (per-unit vs per-box vs per-pallet) | Validator catches unit normalization errors |
| Accessory-vs-primary confusion | Validator catches product identity errors |
| Selective warning omission (says 2, daemon shows 5) | Validator catches narrative honesty |
| Manifest/result count mismatch (claims 12, contract has 8) | Validator catches count reconciliation |
| Fabricated evidence references | Validator catches fake anchors |
| Cross-segment duplicates/gaps (continuation chains) | Validator catches chain-level coherence |

**Pairwise challenge sets:** In addition to absolute verdicts, build pairs: run A = clean, run B = one corruption. Measure whether the validator ranks B less trustworthy.

**Minimum calibration targets per slice before enforcement:**
- >=30 adjudicated rejected/flagged cases with known ground truth
- >=60 audited approved cases checked for silent misses
- Wilson-bound reject precision lower bound >= 70% (~26 correct out of 30)
- Rule-of-three false-pass upper bound <= 5% (0 misses in 60 audited)
- Coverage of dominant failure families in challenge bank

**Cross-model audit sampling:** Periodically run a sample through a different model family to detect correlated blindness.

**Override feedback loop:** Every operator override becomes: (1) labeled calibration case, (2) incident-cluster member, (3) candidate mutation template for challenge bank.

### 3.7 `viewer_sync_status` Table

Push-owned projection table in Supabase. Updated after each successful remote apply.

```sql
CREATE TABLE viewer_sync_status (
  org_id TEXT PRIMARY KEY,
  sync_health TEXT NOT NULL DEFAULT 'offline'
    CHECK (sync_health IN ('healthy', 'degraded', 'offline')),
  last_push_success_at TEXT,           -- ISO-8601
  last_push_attempt_at TEXT,           -- ISO-8601
  last_acked_local_seq INTEGER,        -- Latest local_seq pushed
  pending_outbox_count INTEGER DEFAULT 0,
  schema_epoch INTEGER NOT NULL,
  daemon_version TEXT,
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
```

---

## 4. Testing & Acceptance

### 4.1 Definition of Done

| # | Criterion | Verification |
|---|----------|-------------|
| 1 | Push protocol: transactional outbox entries created for all push-owned mutations in same SQLite transaction | `sqlite_supabase_sync.rs` |
| 2 | Push batches applied to Supabase via single transactional RPC with schema_epoch, row_version ordering | `sqlite_supabase_sync.rs` |
| 3 | Pull protocol: cursor-based pull for shared_source_configs, shared_company_profile, operator_decision_heads | `sqlite_supabase_sync.rs` |
| 4 | Operator decisions: queued local intents flush through Supabase CAS RPC with base_remote_revision; conflicts returned (not silently overwritten) | E2E test |
| 5 | Active-writer lease: second daemon instance detected and rejected | `sqlite_supabase_sync.rs` |
| 6 | RLS policies enforced for all 4 roles (viewer, operator, admin, daemon) | Supabase migration test |
| 7 | Web viewing portal displays opportunities, manifests, pipeline status (read-only) | `contracts.spec.ts` |
| 8 | Supabase Broadcast invalidation delivers live updates to web portal | Manual + E2E |
| 9 | viewer_sync_status shows sync freshness (green/amber/red) | Manual verification |
| 10 | Validation enforcement mode: validator rejections quarantine results as AGENT_SUSPECT_OUTPUT | `sqlite_supabase_sync.rs` |
| 11 | 4 domain-specific validator overlays deployed and producing verdicts | Validator canary tests |
| 12 | Two users can view the same pipeline: desktop (read-write) + web (read-only) | Manual: two-browser test |

### 4.2 Test File Listing (3 New, Cumulative 35)

| File | Coverage |
|------|---------|
| `sqlite_schema.rs` | Full schema creation and migration: all tables created, all constraints enforced, all indices present, sync tracking columns populated, integer pence/bps verified |
| `sqlite_supabase_sync.rs` | Push-pull round-trip: outbox entry creation, batch assembly, transactional RPC apply, row_version ordering, pull cursor advancement, CAS conflict detection, active-writer lease, schema_epoch guard, idempotent replay |
| `apps/web/e2e/contracts.spec.ts` | Web portal E2E: navigate to contracts list, verify read-only (no decision controls), verify keyset pagination, verify sync freshness indicator, verify Broadcast live updates |

### 4.3 Test Scenarios

| Scenario | Expected Behavior |
|----------|------------------|
| Push batch with stale row_version | Remote rejects batch (idempotent); daemon retries with current version |
| Push batch with incompatible schema_epoch | Sync pauses; local operation continues; operator alerted |
| Two desktop operators make concurrent decisions | CAS detects conflict on base_remote_revision; second operator retries with refreshed state |
| Validator rejects output in enforcement mode | Results quarantined as AGENT_SUSPECT_OUTPUT; error manifest created; operator can override |
| Validator itself crashes during validation | Fail-open: worker results advance with VALIDATION_SKIPPED_VALIDATOR_ERROR; operator can review manually |
| Sync degraded (Supabase unreachable) | Desktop shows browse-only mode (local SQLite reads); web shows staleness indicator (amber/red); local operations continue uninterrupted |
| Full push replay after sync recovery | Idempotent: no duplicates; row_version ordering prevents stale overwrites |
| Batch boundary crosses stage_run_id | Push dispatcher rejects: batch boundaries never cross stage_run_id (enforced) |
| Pull-before-push ordering violation | Sync startup enforces pull first, then classify/coalesce pending intents, then resume push |
| Network interruption during push batch | Identical-envelope retry; batch either fully applied or fully rejected (transactional RPC) |

### 4.4 Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Sync engine bugs corrupt remote data | Medium | Medium | Local copy is authoritative; remote can be fully republished from local outbox replay |
| Validator enforcement blocks good data | Medium | Medium | Dry-run calibration in L3; operator override available; false positive rate monitored via operator_override_rate_bps |
| Push batch size exceeds Supabase RPC limits | Low | Low | Batch size bounded by stage_run_id scope; single-in-flight batch constraint |
| Network interruptions during push | Medium | Low | Identical-envelope retries; degraded mode continues local operation; no data loss |

---

## 5. Open Questions, Forward Compatibility & Dependencies

### 5.1 Open Questions

**§10 Further Research #7: Validator calibration corpus bootstrapping**
- Formal calibration harness requires three corpora per skill family: clean real (25-30), weird-but-valid (15-20), mutated bad (30-50).
- L3 dry-run accumulated initial corpus. L4 enforcement decision depends on calibration quality.
- **L4 impact:** Must have sufficient calibration before enabling enforcement; per-slice readiness prevents premature enforcement.

**§10 Assumption #5: Company profile data can be centrally stored with appropriate protections**
- shared_company_profile in Supabase stores org-wide bidder data.
- Impact if wrong: bid workflows need stronger local-only secret partitioning.
- **L4 impact:** RLS policies must protect company profile; confirm data residency comfort level.

### 5.2 Stubs

| Feature | L4 Stub | Full Implementation |
|---------|---------|-------------------|
| Schema epoch transitions | Initial epoch only (no transitions needed) | Future: expand/drain/upgrade/contract protocol |
| Multi-org | Single org hardcoded | Future: org_id scoping on all tables |
| Full remote rebuild | Not implemented | Future: generation-based projection_generation |
| Logical consistency auditor | Not implemented | Future: domain invariant checking with conditional auto-repair |

### 5.3 Forward-Compatibility

| Provision | What Exists in L4 | Target Layer |
|-----------|-------------------|-------------|
| schema_epoch on every batch | Initial epoch; sync pauses on mismatch | Future schema evolution protocol |
| RLS roles designed for multi-org | viewer/operator/admin/daemon roles | Future: org_id column for multi-tenancy |
| Projection tables | Denormalized push-owned mirrors | Future: materialized views for complex queries |

### 5.4 Dependencies

**Layer deps:** L0-L3 (all pipeline data, all push-owned tables with sync tracking columns)

#### External Accounts (L4)

| Account | Purpose |
|---------|---------|
| Supabase project | Shared sync target: Postgres database, Auth, Realtime (Broadcast), Storage |

#### Supabase Migrations (from §9.4)

| Migration | Purpose |
|-----------|---------|
| `0001_initial.sql` | All push-owned projection tables + remote-owned tables |
| `0002_rls.sql` | RLS policies per role (viewer, operator, admin, daemon) |
| `0003_sync_apply.sql` | Transactional sync apply RPC + operator decision CAS RPC |
| `0004_schema_metadata.sql` | Schema metadata table + initial schema_epoch |
| `0005_stage_tracking.sql` | Scope-aware process_stage_status + process_stage_heads projections |
| `0006_junction_tables.sql` | notice_cpv_codes, product_commodity_codes, lot_cpv_codes |

#### New Daemon Module

| Module | Purpose |
|--------|---------|
| `crates/daemon/src/db/sync.rs` | SQLite-to-Supabase push-pull sync engine |

#### New Skills (4 Domain Validator Overlays)

| Skill | Purpose |
|-------|---------|
| `skills/validation/procurement-ingestion/` | OCID structure, date ranges, source verification |
| `skills/validation/supplier-enrichment/` | Pricing plausibility, currency, locale checks |
| `skills/validation/document-acquisition/` | Artifact type, content, completeness |
| `skills/validation/product-extraction/` | Candidate plausibility, slot consistency |

#### Web App

| Path | Files | Purpose |
|------|-------|---------|
| `apps/web` | 7 files | Next.js viewing portal: contracts list, contract detail, status page, realtime subscriptions, staleness store, topic map |

### 5.5 ADRs Addressed

| ADR | Decision | L4 Action |
|-----|----------|----------|
| **ADR-002** | Authority-matrix database | Sync/remote-projection half fully operational |
| **ADR-008** | Desktop = control, web = view | Web viewing portal created (read-only) |
