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

| Skill | Purpose | Interaction Profile |
|-------|---------|---------------------|
| `skills/auth-refresh/alibaba/` | Re-authenticate Alibaba session when auth expires | `pty_interactive` |
| `skills/auth-refresh/1688/` | Re-authenticate 1688 session when auth expires | `pty_interactive` |
| `skills/source-health/fts/` | Canary probe: verify FTS page fingerprint + basic extraction | `pty_interactive` |
| `skills/source-health/alibaba/` | Canary probe: verify Alibaba page fingerprint + auth + basic search | `pty_interactive` |

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
- Base validator (`skills/validation/base/`) checks generic quality
- Domain overlay (e.g., `skills/validation/procurement-ingestion/`) adds domain-specific criteria
- Source-specific overrides via co-located `validator-overlay.md` when shared overlay insufficient

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
| Validator crash/timeout | Fail-open: advance as if skipped | Fail-open: advance with `VALIDATION_SKIPPED_VALIDATOR_ERROR` |

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

**Framing:** Fail-open for availability, not fail-open for trust equivalence. Unvalidated results flow through the pipeline but carry lower trust. Downstream behaviour should depend on trust state.

If the validator crashes, times out, or produces an invalid verdict:
- Worker results advance as if validation were skipped
- `VALIDATION_SKIPPED_VALIDATOR_ERROR` flag set on the job
- Error manifest created for the validator failure itself
- Validation system self-monitors its own failure rate
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

**Behavior version derivation:** For enforcement slice tracking, `worker_behavior_version` is derived from `skill_version` + `result_schema_version` (changes that affect output semantics). `validator_behavior_version` is derived from `validator_overlay_version`. Cosmetic-only skill edits (observability improvements, prompt formatting, cost tuning) do NOT bump behavior version and therefore do NOT reset slice enforcement readiness. The skill metadata must declare whether a version bump is behavior-affecting.

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

6. **Cluster identity assignment** — every `ProcurementProcess` carries a `canonical_process_id` (defaults to self). On merge, the surviving process becomes the cluster root; the retired process's `canonical_process_id` points to the root. Rollups, manifests, and deep links resolve through `canonical_process_id`, not raw `process_id`. The `process_aliases` table handles redirects, but `canonical_process_id` is the full merge model.

7. **Late merge as decision-staleness event** — if two already-enriched processes merge after operator review:
   - All relevant `pending_operator_intents` / `operator_decision_heads` entries are marked stale (`decision_stale = true`)
   - Manifest is regenerated for the merged cluster
   - `pipeline_assessments` recomputed at cluster level
   - Operator sees explicit "identity changed after review" notification
   - This prevents technically correct sync with semantically wrong decisions

**New tables:** `process_relations` (push-owned) and `process_aliases` (push-owned). See §4 for full DDL.

**Conservative thresholds:** Default thresholds are deliberately conservative. False merges are harder to undo than false separations. The operator can manually merge `possible_duplicate` pairs.

**Weight variation by context:** The global weights above are a bootstrap default. Weights should eventually vary by source-pair and notice-stage-pair:
- Planning → tender → award: values drift significantly across lifecycle stages
- Call-offs share buyer/CPV/title but may differ on reference semantics
- FTS vs Contracts Finder duplicates may normalise references differently

For MVP, the global weights are acceptable. A simple interpretable logistic model per source-pair / stage-pair fitted from adjudicated pairs can replace them later — keep the rule engine and hard blockers, fit only the soft weights.

**Additional features for future improvement:** Normalised buyer registration/authority identifier, contact email/domain similarity, lot-count similarity, postcode/region, procurement method / notice type compatibility, reference edit distance after stripping amendment/lot suffixes, document hash overlap if available.

**UI grouping before perfect merge:** To reduce false non-merge cost without destructive auto-merge, group `related_not_merged` and `possible_duplicate` in the desktop UI as a cluster card with relation confidence. Share caches cautiously within the cluster. Let the operator see one cluster view with constituent processes.

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

### 4.1 Tables First Activated in Layer 3

All DDL in Postgres syntax (SQLite mirrors with type substitutions per L0 §2.1). The authoritative table shapes are reserved in L0 §2; this section restates the tables that become operationally active in Layer 3.

#### validation_verdicts (local-only)

Full validator verdict reasoning — never synced. Redacted summary pushed as `validation_verdict_summaries` in Layer 4.

From the first verdict in Layer 3, the daemon also materializes the redacted local `validation_verdict_summaries` projection rows that Layer 4 later syncs to Supabase. This keeps the forward-compatibility chain explicit: L0 reserves the projection schema, L3 starts producing projection content, L4 exports it remotely.

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
| 11 | Validator health canaries running every 15-30 min; known-good/bad/insufficient-basis correctly classified | `validation_canary_health.rs` |
| 12 | Validator health state transitions (healthy → degraded → blind) fire on failure rate spikes | `validation_canary_health.rs` |
| 13 | Retroactive validation backlog accumulates during validator outage and drains on recovery | `validation_retro_backlog.rs` |
| 14 | System runs unattended for 5 consecutive days, >=95% success rate | Monitoring + operator sign-off |

### 5.2 Test File Listing (18 New, Cumulative 38)

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

#### Validation (6)

| File | Coverage |
|------|---------|
| `validation_policy.rs` | All policy mode x trigger combinations: always/sample/trigger/skip, anomaly triggers |
| `validation_executor.rs` | Sub-job spawning, kickoff prompt composition, verdict parsing, basis_sufficiency |
| `validation_dry_run.rs` | Dry-run verdicts logged, pipeline NOT blocked, no outbox, no stage head advancement |
| `validation_golden_verdicts.rs` | Known-good -> approved, known-bad -> rejected, ambiguous -> flagged |
| `validation_canary_health.rs` | Synthetic canary scheduling, known-good/bad/insufficient-basis classification, health state transitions (healthy → degraded → blind), auto-demotion on blind |
| `validation_retro_backlog.rs` | Backlog accumulation during validator outage, priority-ordered drain on recovery, stale-marking of affected decisions |

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
| Validator breaks at 2 AM, 200 overnight runs | All 200 results carry `unverified` trust state; validator health transitions to `blind`; retroactive validation backlog queued; operator morning dashboard shows single validation blackout incident |
| Validator canary misclassifies known-bad as approved | Health transitions to `degraded`; enforcement auto-demotes on affected slices |
| Two processes merge after operator reviewed one | Both operator decisions marked stale; merged `canonical_process_id` assigned; manifest regenerated |
| Cosmetic prompt edit to skill (no behavior change) | Behavior version unchanged; slice enforcement readiness preserved |

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

| Provision | What Exists in L3 | Target Layer |
|-----------|-------------------|-------------|
| Validation policy engine ready for enforcement toggle | Evaluates but always dry-run=true | Layer 4 |
| Run manifest corpus accumulates | Every skill run produces manifest | Layer 4 calibration |
| `validation_verdicts` full schema from creation | Includes enforcement_eligible, basis_sufficiency | Layer 4 projections |
| Source circuit breaker state accessible | Feeds into risk-stratified sampling | Layer 4 |
| Skill promotion includes validator approval rate | Shadow comparison metric | Layer 4 promotion gate |
| Validator health states operational | healthy/degraded/blind with canary checks | Layer 4 enforcement auto-demotion |
| Retroactive validation backlog | Accumulates during outages | Layer 4 trust-state-aware enforcement |
| Trust-state lattice on pipeline results | verified/flagged/unverified/suspect tracked | Layer 4 downstream behaviour gating |
| `canonical_process_id` on processes | Defaults to self; updated on merge | Layer 4 cluster-aware sync |

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

<!-- CROSS-DOMAIN NOTE FOR L0: `canonical_process_id` (uuid, nullable, defaults to self) must exist on `procurement_processes` from creation. This column is introduced conceptually in L3 §3.6 but per the all-columns-from-creation rule, it must be present as nullable in the L0 DDL. On first write, it defaults to the process's own `id`. L3 relation resolution populates it on merge. -->

<!-- CROSS-DOMAIN NOTE FOR L0: `trust_state` (text, not null, default 'unverified') and `source_job_id` (uuid, nullable) belong to the Layer 3 operational activation of `listing_evidence`. The constitutional table shape is reserved in L0, so these fields must be present when that table is first materialized for runtime use. -->

<!-- CROSS-DOMAIN NOTE FOR L2: Trust-state lattice (verified/flagged/unverified/suspect) from L3 §2.7 is finer-grained than the operator-facing `pipeline_assessments.validation_state` field. L2's primary surface remains the coarse process-level states `validated` / `flagged` / `unverified`, but desktop views should still expose finer-grained trust debt where relevant (for example, highlighting `suspect` supporting artifacts or caches behind a `flagged` process rollup). -->

<!-- CROSS-DOMAIN NOTE FOR L2: `canonical_process_id` from L3 §3.6 affects desktop deep links and cluster UI grouping. L2's opportunity detail page should resolve through `canonical_process_id` when navigating. L2's opportunity list should group `related_not_merged` and `possible_duplicate` processes as cluster cards with relation confidence. -->

<!-- CROSS-DOMAIN NOTE FOR L6: Trust-state lattice from L3 §2.7 gates bid workflow. At the bid-readiness surface this maps to two checks: the selected bid scope must contain no `suspect` upstream inputs, and the process-level rollup should surface `pipeline_assessments.validation_state = validated` unless the operator explicitly accepts flagged/unverified trust debt. -->

<!-- CROSS-DOMAIN NOTE FOR L5: Cumulative test count in L5 §3.2 header shows 44 but should be 47 (L4 cumulative 42 + L5 new 5 = 47). The authoritative count is in layer-strategy.md §11. -->

<!-- CROSS-DOMAIN NOTE FOR L6: Cumulative test count in L6 §2.2 header shows 47 but should be 50 (L5 cumulative 47 + L6 new 3 = 50). The authoritative count is in layer-strategy.md §11. -->
