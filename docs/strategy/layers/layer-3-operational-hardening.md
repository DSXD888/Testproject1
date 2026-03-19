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
