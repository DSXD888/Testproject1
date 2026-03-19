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
