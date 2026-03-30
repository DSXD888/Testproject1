# L3 Operational Hardening — Bead Bulk Import

## l3-p1-deps-migration: Cargo Dependencies And L3 Schema Activation

- type: task
- priority: P0
- labels: phase-1,rust,schema,foundation
- parent: testproject1-hmw

### Description

## SOURCE OF TRUTH WARNING

This bead is a quick-reference derived from:
- **Spec**: `docs/strategy/layers/layer-3-operational-hardening.md` — §1.3, §4.1–4.3 (PRIMARY — read this first)
- **TDD**: `docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-tdd.md` — CI-L3-001, CI-L3-005, L3-T-schema_activation-001..008 (PRIMARY — test definitions here)
- **Strategy**: `docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-strategy.md` — Phase 1 (ordering context only)

Before implementing, you MUST read the referenced spec and TDD sections directly.

## Context

Root bead for all Layer 3 work. Adds `cron` and `sysinfo` crate dependencies, creates the L3 migration that materializes all 5 new tables, and updates `db/sqlite.rs` to apply the migration. Everything else depends on this.

## Language & Quality Gates

- **Language:** `rust`
- **Quality gate commands:**
  - `cargo check --all-targets`
  - `cargo clippy --all-targets -- -D warnings`
  - `cargo fmt --check`

## Files to Create/Modify

- **Modify:** `crates/daemon/Cargo.toml` — add `cron` and `sysinfo` dependencies
- **Create:** `migrations/0005_l3_operational_hardening.sql` — all 5 L3 tables + ALTER statements for modified columns
- **Modify:** `crates/daemon/src/db/sqlite.rs` — apply migration 0005 in open path
- **Create:** `crates/daemon/tests/schema_activation.rs` — schema activation tests

## Spec Requirements (verbatim from §4.1–4.3)

**L3-FR-4.1.1 — `validation_verdicts` stores full local-only validator reasoning**
- AC1: `validation_verdicts` links worker and validator jobs/sessions and stores verdict identity, confidence, summary, `basis_sufficiency`, incident signature, `enforcement_eligible`, issue JSON, and run-manifest hash.
- AC2: Operator override fields (`override_verdict`, `override_reason`, `override_by`, `override_at`) exist from first activation.
- AC3: The table is local-only; Layer 4 later derives redacted `validation_verdict_summaries` from it without requiring a duplicate local summary table in Layer 3.

**L3-FR-4.1.2 — `source_runtime_state` persists breaker/health state per source**
- AC1: `source_runtime_state` is keyed by `source_id` and records health color, circuit state, challenge counters/window, cooldown timestamps, latest canary outcome, and last error code.
- AC2: The table is the local operational authority for source-health and circuit-breaker transitions.
- AC3: Persisted fields are sufficient to audit why a source is `closed`, `half_open`, or `open`.

**L3-FR-4.1.3 — `listing_evidence` activates as a trust-aware local cache**
- AC1: `listing_evidence` stores source identity, listing identity, normalized title, structured attributes, parsed standards/units, seller/commercial fields, freshness timestamps, and detail-fetch flags.
- AC2: The table carries `trust_state` with default `unverified` plus `source_job_id` for provenance/trust tracing.
- AC3: Uniqueness is enforced on `(source, source_listing_id)`, and source-specific freshness TTLs match the table in this section.

**L3-FR-4.1.4 — `process_relations` stores push-owned relation edges**
- AC1: `process_relations` stores both process IDs, `relation_type`, `confidence_bps`, supporting evidence, and the standard sync-tracking fields.
- AC2: Stored `relation_type` values remain constrained to `merged_same_process`, `related_not_merged`, and `possible_duplicate`; `separate` writes no row.
- AC3: The table is push-owned so relation state can become shared visibility in Layer 4 without changing authority.

**L3-FR-4.1.5 — `process_aliases` stores push-owned redirects for retired process IDs**
- AC1: `process_aliases` maps `alias_process_id` to `canonical_process_id` with an `alias_reason` and the full standard push-owned sync columns including `stage_run_id`.
- AC2: Alias rows preserve deep-link stability after relation-driven merges.
- AC3: The alias table is push-owned and remains separate from the full cluster model carried by `canonical_process_id`.

**L3-FR-4.2.1 — Layer 3 activates its forward-compatible columns on existing tables**
- AC1: `workflow_jobs.parent_job_id` links validator sub-jobs to worker jobs in live Layer 3 behavior.
- AC2: `workflow_jobs.validation_required` and `workflow_jobs.validation_verdict_id` are actively populated by validation policy/execution.
- AC3: `agent_session_logs.run_manifest_path` and `agent_session_logs.run_manifest_hash` are populated for every manifest-producing session.

**L3-FR-4.3.1 — Layer 3 table authority classes are fixed at activation**
- AC1: `validation_verdicts`, `source_runtime_state`, and `listing_evidence` are local-only.
- AC2: `process_relations` and `process_aliases` are push-owned.
- AC3: No Layer 3 table introduced in this section mixes local and remote authority.

## DDL (from spec §4.1 — use SQLite type substitutions per L0 §2.1)

See spec §4.1 for the full DDL of: `validation_verdicts`, `source_runtime_state`, `listing_evidence`, `process_relations`, `process_aliases`.

## Tests to Pass

- **L3-T-schema_activation-001** (T2): applying the Layer 3 migration creates `validation_verdicts`, `source_runtime_state`, `listing_evidence`, `process_relations`, and `process_aliases` with declared columns and key constraints
- **L3-T-schema_activation-002** (T2): `source_runtime_state` includes `circuit_state` and `last_canary_result` at migration time; no runtime `ALTER TABLE` drift
- **L3-T-schema_activation-003** (T2): `validation_verdicts` insert and `workflow_jobs.validation_verdict_id` linkage commit atomically
- **L3-T-schema_activation-004** (T2): `listing_evidence` enforces `(source, source_listing_id)` uniqueness, defaults `trust_state = 'unverified'`
- **L3-T-schema_activation-005** (T2): `process_relations` stores only non-`separate` outcomes
- **L3-T-schema_activation-006** (T2): `process_aliases` preserves alias redirects with push-owned sync columns
- **L3-T-schema_activation-007** (T2): live L3 paths populate forward-compatible columns on existing tables
- **L3-T-schema_activation-008** (T2): local-only tables excluded from sync; push-owned tables retain sync columns
- **CI-L3-001**: `cargo check --all-targets` exits 0
- **CI-L3-005**: module inventory matches expected surfaces

## Acceptance Criteria

1. `migrations/0005_l3_operational_hardening.sql` creates all 5 tables with correct columns/constraints
2. `crates/daemon/Cargo.toml` includes `cron` and `sysinfo` dependencies
3. `db/sqlite.rs` applies migration 0005 in the open path
4. All L3-T-schema_activation tests pass
5. No runtime ALTER TABLE drift

## Dependencies

- **Blocked by:** none (root bead for L3)
- **Blocks:** all other L3 beads
- **Phase:** 1 — Schema, Claim, And Admission Foundation

---

## l3-p1-job-claim: Concurrent Job Claiming With Lease Management

- type: task
- priority: P0
- labels: phase-1,rust,jobs
- parent: testproject1-hmw

### Description

## SOURCE OF TRUTH WARNING

This bead is a quick-reference derived from:
- **Spec**: `docs/strategy/layers/layer-3-operational-hardening.md` — §1.3.3 (PRIMARY)
- **TDD**: `docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-tdd.md` — L3-T-job_claim-001..005 (PRIMARY)
- **Strategy**: `docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-strategy.md` — Phase 1, parallel after schema

## Context

Extract claim SQL and lease management from `jobs/runner.rs` into a dedicated `jobs/claim.rs` module. Concurrent worker threads can claim jobs atomically. Lease expiry returns stuck claims.

## Language & Quality Gates

- **Language:** `rust`
- **Quality gate commands:**
  - `cargo check --all-targets`
  - `cargo clippy --all-targets -- -D warnings`

## Files to Create

- **Create:** `crates/daemon/src/jobs/claim.rs` — concurrent claim + lease management

## Spec Requirements (verbatim)

**L3-FR-1.3.3 — `jobs/claim.rs` provides atomic concurrent job claiming**
- AC1: Layer 3 includes `jobs/claim.rs`.
- AC2: Concurrent worker threads can claim jobs without double-claiming the same runnable work item.
- AC3: Lease expiry returns stuck claims to the runnable pool without inventing duplicate authoritative work.
- AC4: The concurrent claimer preserves the current `JobRunner::claim_next_job` eligibility/order surface: only jobs in `pending` or `retry` status are claimable, ordered by `COALESCE(scheduled_for, created_at), created_at`.
- AC5: A successful claim records runner-owned attempt metadata before execution begins, including `lease_owner`, `started_at`, and `attempt_count = attempt_count + 1`.
- AC6: `jobs/claim.rs` becomes the single SQL claim/lease surface used by `JobRunner`; Layer 3 does not keep a competing ad hoc claimant inside `jobs/runner.rs`.

## Tests to Pass

- **L3-T-job_claim-001** (T2): concurrent claimers cannot double-claim the same runnable job
- **L3-T-job_claim-002** (T2): claim eligibility remains `status IN ('pending','retry')` ordered by `COALESCE(scheduled_for, created_at), created_at`
- **L3-T-job_claim-003** (T2): successful claim atomically sets `lease_owner`, `started_at`, `attempt_count + 1`
- **L3-T-job_claim-004** (T2): lease expiry returns stuck claim exactly once without duplicating work
- **L3-T-job_claim-005** (T1): `jobs/claim.rs` is the single SQL claim surface; no competing claimant in `jobs/runner.rs`

## Acceptance Criteria

1. `jobs/claim.rs` exists and is the sole claim/lease SQL surface
2. Concurrent threads cannot double-claim
3. Lease expiry works correctly
4. Eligibility/ordering preserved from existing `JobRunner::claim_next_job`

## Dependencies

- **Blocked by:** l3-p1-deps-migration
- **Blocks:** l3-p1-runner-main-integration
- **Phase:** 1

---

## l3-p1-arbiter: Resource Arbiter Typed Quota Model

- type: task
- priority: P0
- labels: phase-1,rust,admission
- parent: testproject1-hmw

### Description

## SOURCE OF TRUTH WARNING

This bead is a quick-reference derived from:
- **Spec**: `docs/strategy/layers/layer-3-operational-hardening.md` — §1.3.6, §1.4 (PRIMARY)
- **TDD**: `docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-tdd.md` — L3-T-job_claim-006..009 (PRIMARY)
- **Strategy**: `docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-strategy.md` — Phase 1, parallel with job-claim

## Context

Create the Resource Arbiter that admits agent jobs using typed resource budgets. Three quota types with admission priority ordering. Runs parallel with job-claim after schema activation.

## Language & Quality Gates

- **Language:** `rust`
- **Quality gate commands:**
  - `cargo check --all-targets`
  - `cargo clippy --all-targets -- -D warnings`

## Files to Create

- **Create:** `crates/daemon/src/agents/arbiter.rs` — typed quota admission

## Spec Requirements (verbatim)

**L3-FR-1.3.6 — `agents/arbiter.rs` admits work through typed resource budgets**
- AC1: Layer 3 includes `agents/arbiter.rs`.
- AC2: Admission decisions consume typed quota budgets rather than one global concurrency counter.
- AC3: Admission outcomes are compatible with ordered priorities and source-level suppression.

**L3-FR-1.4.1 — `browser_slots` is a hard concurrency cap for browser-backed sessions**
- AC1: Every browser-backed session consumes one `browser_slots` unit.
- AC2: The MVP default cap is `2`.
- AC3: If no browser slot is available, the arbiter delays admission rather than silently oversubscribing.

**L3-FR-1.4.2 — `agent_slots` is a soft cap for adapter-mediated session concurrency**
- AC1: All adapter-mediated sessions count against `agent_slots`.
- AC2: The MVP default cap is `4`.
- AC3: The arbiter can queue runnable work when the soft cap is reached.

**L3-FR-1.4.3 — `source_lanes` is a per-source rate cap**
- AC1: Procurement sources default to `1/source` and supplier sources default to `2/source`.
- AC2: The same source cannot exceed its configured concurrent lane count.
- AC3: Source-lane enforcement is source-specific and does not suppress unrelated sources.

**L3-FR-1.4.4 — Resource admission uses ordered priorities and skill-local lowering only**
- AC1: Admission priority: procurement ingestion, auth refresh/health probes, qualification, supplier enrichment, manifest generation, bid automation.
- AC2: `max_concurrency` declared by a skill may lower the effective ceiling below the arbiter default.
- AC3: No skill declaration can raise any typed quota above the daemon-configured budget.

**L3-FR-1.4.5 — Resource acquisition/release and validator admission remain explicit**
- AC1: Admitted work holds resource units until pre-spawn failure or terminal cleanup releases them.
- AC2: Validator sub-jobs consume `agent_slots` but not `browser_slots` or `source_lanes`.
- AC3: The arbiter emits structured grant/deny/release events keyed by job/session identity.

## Tests to Pass

- **L3-T-job_claim-006** (T2): `browser_slots` hard cap — third browser job denied when two active
- **L3-T-job_claim-007** (T2): `agent_slots` ceiling; validator sub-jobs consume only `agent_slots`
- **L3-T-job_claim-008** (T2): `source_lanes` per-source concurrency with unrelated sources unaffected
- **L3-T-job_claim-009** (T2): skill `max_concurrency` lowers but cannot raise quotas; grant/deny/release events keyed by identity

## Acceptance Criteria

1. `agents/arbiter.rs` implements three quota types: `browser_slots`, `agent_slots`, `source_lanes`
2. Admission priority ordering enforced
3. Validator sub-jobs consume only `agent_slots`
4. Grant/deny/release events emitted for auditability

## Dependencies

- **Blocked by:** l3-p1-deps-migration
- **Blocks:** l3-p1-runner-main-integration
- **Phase:** 1

---

## l3-p1-runner-main-integration: Phase 1 Runner And Bootstrap Integration

- type: task
- priority: P0
- labels: phase-1,rust,integration
- parent: testproject1-hmw

### Description

## SOURCE OF TRUTH WARNING

This bead is a quick-reference derived from:
- **Spec**: `docs/strategy/layers/layer-3-operational-hardening.md` — §1.3.3, §1.3.6, §1.4 (PRIMARY)
- **TDD**: `docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-tdd.md` — L3-T-job_claim-001..009 (PRIMARY)
- **Strategy**: `docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-strategy.md` — Phase 1 convergence

## Context

Wire `jobs/claim.rs` and `agents/arbiter.rs` into the existing `jobs/runner.rs` and `main.rs`. The runner delegates claim to `jobs/claim.rs` and admission to `agents/arbiter.rs`. Bootstrap registers admission state.

## Language & Quality Gates

- **Language:** `rust`
- **Quality gate commands:**
  - `cargo check --all-targets`
  - `cargo clippy --all-targets -- -D warnings`

## Files to Modify

- **Modify:** `crates/daemon/src/jobs/runner.rs` — delegate `claim_next_job` to `jobs/claim.rs`, add arbiter admission chokepoint
- **Modify:** `crates/daemon/src/main.rs` — add `mod jobs::claim`, `mod agents::arbiter`, `bootstrap_runtime()` admission-state initialization

## Spec Requirements

Per L3-FR-1.3.3 AC4-AC6: `jobs/claim.rs` becomes the single SQL claim/lease surface. Per L3-FR-1.3.6 AC3: admission outcomes compatible with runner-owned job visibility.

Strategy guardrail: `main.rs` is a merge-hot choke point. One terminal integration bead per phase after module contracts are frozen.

## Tests to Pass

- **L3-T-job_claim-001..009** — all claim and arbiter tests pass end-to-end through runner integration
- **CI-L3-001, CI-L3-005** — compile gate + inventory parity

## Acceptance Criteria

1. `jobs/runner.rs` delegates claim to `jobs/claim.rs` — no competing ad hoc claim SQL remains
2. `main.rs` declares `jobs::claim` and `agents::arbiter` modules
3. `bootstrap_runtime()` initializes admission state
4. All Phase 1 gate criteria pass

## Dependencies

- **Blocked by:** l3-p1-job-claim, l3-p1-arbiter
- **Blocks:** l3-p1-tests
- **Phase:** 1

---

## l3-p1-tests: Phase 1 Test Files

- type: task
- priority: P0
- labels: phase-1,rust,tests
- parent: testproject1-hmw

### Description

## SOURCE OF TRUTH WARNING

This bead is a quick-reference derived from:
- **Spec**: `docs/strategy/layers/layer-3-operational-hardening.md` — §4.1–4.3, §1.3.3, §1.4 (PRIMARY)
- **TDD**: `docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-tdd.md` — §8.8, §8.19 (PRIMARY)
- **Strategy**: `docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-strategy.md` — Phase 1

## Context

Create both Phase 1 test files. `schema_activation.rs` validates the migration and table shapes. `job_claim.rs` validates concurrent claiming, lease management, and typed quota admission.

## Language & Quality Gates

- **Language:** `rust`
- **Quality gate commands:**
  - `cargo test --test schema_activation --test job_claim`

## Files to Create

- **Create:** `crates/daemon/tests/schema_activation.rs` — 8 tests (L3-T-schema_activation-001..008)
- **Create:** `crates/daemon/tests/job_claim.rs` — 9 tests (L3-T-job_claim-001..009)

## Tests to Implement

See TDD §8.8 and §8.19 for full assertion specifications. All 17 tests must use stable test IDs verbatim in the test function names.

## Phase 1 Gate Criteria (from strategy)

- `cargo check --all-targets`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check` pass
- CI-L3-001, CI-L3-002, CI-L3-003, CI-L3-005 green
- L3-T-schema_activation-001..008 pass
- L3-T-job_claim-001..009 pass
- Runner still owns the only enqueue/order/idempotency lane
- `bootstrap_runtime()` remains the only startup insertion point

## Dependencies

- **Blocked by:** l3-p1-runner-main-integration
- **Blocks:** Phase 2 beads, Phase 3 beads, Phase 5 beads
- **Phase:** 1

---

## l3-p2-source-registry: Source Registry And Runtime State

- type: task
- priority: P1
- labels: phase-2,rust,sources
- parent: testproject1-hmw

### Description

## SOURCE OF TRUTH WARNING

This bead is a quick-reference derived from:
- **Spec**: `docs/strategy/layers/layer-3-operational-hardening.md` — §1.3.2, §3.4, §3.5 (PRIMARY)
- **TDD**: `docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-tdd.md` — L3-T-skill_drift_monitor-001..004 (PRIMARY)
- **Strategy**: `docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-strategy.md` — Phase 2, sequential Rust trunk root

## Context

Create `sources/registry.rs` with typed source definitions, version tracking, skill binding, schedule, and health/cooldown status. Integrate with `source_runtime_state` table for circuit breaker state. This module precedes source-health skills and scheduler.

## Language & Quality Gates

- **Language:** `rust`
- **Quality gate commands:**
  - `cargo check --all-targets`
  - `cargo clippy --all-targets -- -D warnings`

## Files to Create

- **Create:** `crates/daemon/src/sources/registry.rs` — source definitions, version tracking, skill binding, circuit breaker transitions

## Spec Requirements (verbatim)

**L3-FR-1.3.2 — `sources/registry.rs` owns typed source definitions and mutable runtime binding**
- AC1: Layer 3 includes `sources/registry.rs`.
- AC2: The module stores typed source definitions with version tracking, skill binding, schedules, and health/cooldown status.
- AC3: Static source definitions remain rooted in `sources`, while mutable runtime state is read from and written to `source_runtime_state`.

**L3-FR-3.5.1 — `closed` source circuits permit normal scheduling**
- AC1: In `closed`, all jobs for the source remain eligible for normal scheduling.

**L3-FR-3.5.2 — `half_open` source circuits are canary-only recovery mode**
- AC1: In `half_open`, only `source-health/*` and `auth-refresh/*` jobs are allowed.
- AC2: A successful canary transitions back to `closed`.
- AC3: A failed canary transitions to `open`.

**L3-FR-3.5.3 — `open` source circuits pause normal work until cooldown**
- AC1: In `open`, no normal jobs admitted.
- AC2: Default cooldown `2 hours`.
- AC3: Cooldown expiry transitions to `half_open`, not directly to `closed`.

**L3-FR-3.5.4 — Breaker trigger criteria and state persistence are explicit**
- AC1: Circuit opens after `N=3` consecutive challenge-class failures.
- AC2: Challenge-class: `AGENT_AUTH_CHALLENGE`, `SOURCE_CAPTCHA_BLOCKED`, `SOURCE_SOFT_BLOCKED` with blocker class.
- AC3: State persisted in `source_runtime_state`.
- AC4: Circuit transitions execute transactionally; newer canary timestamps win.

## Tests to Pass

- **L3-T-skill_drift_monitor-001** (T2): canaries execute six checks and map failures to existing `ErrorCode` values
- **L3-T-skill_drift_monitor-002** (T2): 30-minute cadence, persist outcomes in `source_runtime_state`
- **L3-T-skill_drift_monitor-003** (T2): N=3 failures -> `open`, 2h cooldown, return through `half_open`
- **L3-T-skill_drift_monitor-004** (T2): stale canary successes cannot close newer `open`/`half_open`

## Dependencies

- **Blocked by:** l3-p1-tests (Phase 1 gate)
- **Blocks:** l3-p2-cron-scheduler
- **Phase:** 2

---

## l3-p2-cron-scheduler: Cron Scheduler With Explicit Skill Binding

- type: task
- priority: P1
- labels: phase-2,rust,scheduler
- parent: testproject1-hmw

### Description

## SOURCE OF TRUTH WARNING

This bead is a quick-reference derived from:
- **Spec**: `docs/strategy/layers/layer-3-operational-hardening.md` — §1.3.1, §1.5 (PRIMARY)
- **TDD**: `docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-tdd.md` — L3-T-cron_scheduler-001..010 (PRIMARY)
- **Strategy**: `docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-strategy.md` — Phase 2, after registry

## Context

Create `scheduler/cron.rs` with cron expression parsing, schedule evaluation, explicit skill binding at enqueue time, catchup sweeps, and source suppression integration.

## Language & Quality Gates

- **Language:** `rust`
- **Quality gate commands:**
  - `cargo check --all-targets`
  - `cargo clippy --all-targets -- -D warnings`

## Files to Create

- **Create:** `crates/daemon/src/scheduler/cron.rs` — cron scheduling with frozen skill binding

## Spec Requirements (verbatim)

**L3-FR-1.3.1 — `scheduler/cron.rs` owns daemon-local cron scheduling**
- AC1: Layer 3 includes `scheduler/cron.rs`.
- AC2: Schedules and enqueues agent work from cron definitions inside the daemon.
- AC3: Preserves explicit skill binding and startup catchup behavior.

**L3-FR-1.5.1 — Cron scheduling freezes the executable skill binding at enqueue time**
- AC1: Every cron-enqueued job stores `source_id`, `skill_id`, `skill_version`, `cron_expression`, and enablement/next-run state.
- AC2: Freezes `skill_id + skill_version` at enqueue time — no late-bound resolution.
- AC3: If admission delayed, bound job remains queued with original binding.
- AC4: Cron roots enqueue through ordinary `EnqueueJobRequest` with `parent_job_id = NULL`.

**L3-FR-1.5.2 — Catchup sweeps and source suppression**
- AC1: On daemon start after downtime, detects overdue runs and processes in priority order.
- AC2: Catchup respects typed quotas.
- AC3: Open/cooling-down source circuit suppresses normal ingestion; only `source-health/*` or `auth-refresh/*` allowed.

## Tests to Pass

- **L3-T-cron_scheduler-001** (T1): valid cron expression parses and computes `next_run_at`
- **L3-T-cron_scheduler-002** (T1): invalid cron expression rejected at load time
- **L3-T-cron_scheduler-003** (T2): due root cron enqueues with frozen skill binding via `EnqueueJobRequest`
- **L3-T-cron_scheduler-004** (T2): already-enqueued job uses frozen binding, not newer version
- **L3-T-cron_scheduler-005** (T1): disabled definitions do not enqueue work
- **L3-T-cron_scheduler-006** (T2): catchup sweep enqueues overdue in priority order within arbiter capacity
- **L3-T-cron_scheduler-007** (T2): Europe/London spring-forward skips nonexistent hour
- **L3-T-cron_scheduler-008** (T2): Europe/London fall-back does not double-enqueue
- **L3-T-cron_scheduler-009** (T2): open/cooling circuit suppresses normal ingestion, allows health/auth
- **L3-T-cron_scheduler-010** (T2): source definitions in `sources`, mutable state in `source_runtime_state`

## Dependencies

- **Blocked by:** l3-p2-source-registry
- **Blocks:** l3-p2-runner-main-integration
- **Phase:** 2

---

## l3-p2-operational-skills: Auth-Refresh And Source-Health Skill Directories

- type: task
- priority: P1
- labels: phase-2,skills,filesystem
- parent: testproject1-hmw

### Description

## SOURCE OF TRUTH WARNING

This bead is a quick-reference derived from:
- **Spec**: `docs/strategy/layers/layer-3-operational-hardening.md` — §1.9, §3.3, §3.4, §3.7, §3.8 (PRIMARY)
- **TDD**: `docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-tdd.md` — L3-T-skill_golden_journeys-007, 008, L3-T-skill_drift_monitor-005 (PRIMARY)
- **Strategy**: `docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-strategy.md` — Phase 2, parallel filesystem lane

## Context

Create the 4 operational skill directories. Each follows the standard 6-file skill directory format: SKILL.md, result.schema.json, checkpoint.schema.json, canary.inputs.jsonl, golden.assertions.yaml + any source-specific files. Parallel with Rust work after source IDs and interaction profiles are frozen.

## Language & Quality Gates

- **Language:** `filesystem` (no compilation — skill directory structure)
- **Quality gate commands:**
  - Directory structure lint (CI-L3-005)

## Files to Create

- **Create:** `skills/auth-refresh/alibaba/SKILL.md` + standard skill files
- **Create:** `skills/auth-refresh/1688/SKILL.md` + standard skill files
- **Create:** `skills/source-health/fts/SKILL.md` + standard skill files
- **Create:** `skills/source-health/alibaba/SKILL.md` + standard skill files

## Spec Requirements (verbatim)

**L3-FR-1.9.1 — Layer 3 skill inventory is explicitly specified and interaction-profile bound**
- AC1: Inventory includes `skills/auth-refresh/alibaba/`, `skills/auth-refresh/1688/`, `skills/source-health/fts/`, `skills/source-health/alibaba/`.
- AC2: Each operational skill declares a single purpose and uses the `pty_interactive` interaction profile.
- AC4: Sufficient for auth refresh, source health, and generic semantic validation.

**L3-FR-3.3.1 — Layer 3 skill families have explicit operational roles**
- AC2: Auth-refresh skills establish and publish authenticated session state.
- AC3: Source-health canaries prove reachability, fingerprint/auth/locale sanity, and minimal extraction.

**L3-FR-3.4.1 — Source-health canaries execute a fixed six-check health contract**
- AC1: Each canary checks reachability, page fingerprint, auth state, geo/locale, challenge rate, minimal extraction.
- AC2: Failed checks map to explicit classification (existing `ErrorCode` surface + structured context).

**L3-FR-3.7.1 — Source anti-bot difficulty and mitigation strategy are explicit**
- AC1: Each source has declared anti-bot difficulty, challenges, and mitigation.

**L3-FR-3.8.1 — Browser automation isolates auth state**
- AC1: Auth-refresh skills own session establishment and publish reusable auth-state handles.
- AC2: Extraction skills consume handles rather than embedding login logic.
- AC3: Each extraction session starts with clean browser context.

**L3-FR-3.8.2 — Browser readiness, locale checks, and fingerprint proof**
- AC1: Every navigation step waits for required anchors, rejects forbidden anchors.
- AC2: Verify expected locale and currency before extraction.
- AC3: Extraction begins only after page fingerprint proof.

## Tests to Pass

- **L3-T-skill_golden_journeys-007** (T1): L3 skill inventory present with required file set and interaction profiles
- **L3-T-skill_golden_journeys-008** (T3): auth-refresh and source-health skills execute their specific contracts only
- **L3-T-skill_drift_monitor-005** (T1): anti-bot ratings are source-specific inputs, not generic retry loops

## Dependencies

- **Blocked by:** l3-p2-source-registry (source IDs and interaction profiles frozen)
- **Blocks:** l3-p2-tests
- **Phase:** 2

---

## l3-p2-runner-main-integration: Phase 2 Scheduler Bootstrap Integration

- type: task
- priority: P1
- labels: phase-2,rust,integration
- parent: testproject1-hmw

### Description

## SOURCE OF TRUTH WARNING

- **Spec**: `docs/strategy/layers/layer-3-operational-hardening.md` — §1.3.1, §1.3.2, §1.5
- **TDD**: `docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-tdd.md` — L3-T-cron_scheduler tests
- **Strategy**: `docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-strategy.md` — Phase 2 convergence

## Context

Wire `sources/registry.rs` and `scheduler/cron.rs` into `jobs/runner.rs` (enqueue_job cron-root use) and `main.rs` (mod tree + bootstrap_runtime scheduler startup/catchup/suppression wiring).

## Files to Modify

- **Modify:** `crates/daemon/src/jobs/runner.rs` — `enqueue_job` cron-root support
- **Modify:** `crates/daemon/src/main.rs` — `sources` / `scheduler` mod tree + `bootstrap_runtime()` scheduler startup and catchup/suppression wiring

## Dependencies

- **Blocked by:** l3-p2-cron-scheduler
- **Blocks:** l3-p2-tests
- **Phase:** 2

---

## l3-p2-tests: Phase 2 Test Files

- type: task
- priority: P1
- labels: phase-2,rust,tests
- parent: testproject1-hmw

### Description

## SOURCE OF TRUTH WARNING

- **TDD**: `docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-tdd.md` — §8.7, §8.11, §8.20 (partial)

## Context

Create Phase 2 test files: `cron_scheduler.rs` (10 tests), `skill_drift_monitor.rs` (6 tests), and the Phase 2 portion of `observability_telemetry.rs` (L3-T-observability_telemetry-012).

## Files to Create

- **Create:** `crates/daemon/tests/cron_scheduler.rs` — 10 tests (L3-T-cron_scheduler-001..010)
- **Create:** `crates/daemon/tests/skill_drift_monitor.rs` — 6 tests (L3-T-skill_drift_monitor-001..006)
- **Create:** `crates/daemon/tests/observability_telemetry.rs` — starts with L3-T-observability_telemetry-012 (ErrorCode inventory parity)

## Phase 2 Gate Criteria

- L3-T-cron_scheduler-001..010 pass
- L3-T-skill_drift_monitor-001..006 pass
- L3-T-observability_telemetry-012 passes
- Frozen skill binding, catchup ordering, source suppression all route through persisted state

## Dependencies

- **Blocked by:** l3-p2-runner-main-integration, l3-p2-operational-skills
- **Blocks:** Phase 4 beads
- **Phase:** 2

---

## l3-p3-pty-hardening: PTY Hardening Module

- type: task
- priority: P1
- labels: phase-3,rust,pty
- parent: testproject1-hmw

### Description

## SOURCE OF TRUTH WARNING

- **Spec**: `docs/strategy/layers/layer-3-operational-hardening.md` — §1.3.5, §1.6 (PRIMARY)
- **TDD**: `docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-tdd.md` — L3-T-fault_pty_edge_cases-001..008 (PRIMARY)
- **Strategy**: `docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-strategy.md` — Phase 3, sequential root

## Context

Create `pty/hardening.rs` with fd hygiene, EIO handling, terminal geometry, and signal handling. This is the Phase 3 sequential root — all other Phase 3 work depends on this.

## Files to Create

- **Create:** `crates/daemon/src/pty/hardening.rs` — PTY hardening bundle

## Spec Requirements (verbatim)

**L3-FR-1.3.5 — `pty/hardening.rs` owns PTY-specific reliability protections**
- AC1: Layer 3 includes `pty/hardening.rs`.
- AC2: Implements fd hygiene, PTY `EIO` handling, fixed geometry for headless runs, and signal-as-wakeup wait draining.
- AC3: PTY hardening changes terminal/runtime reliability only, not business pipeline semantics.

**L3-FR-1.6.1 — PTY sessions apply the full hardening bundle**
- AC1: Close inherited fds before exec.
- AC2: `EIO` on PTY master read after slave closure treated as expected hangup.
- AC3: Headless PTY runs default to `120x40`.
- AC4: Unix signals act as wakeups only; drain `waitpid` loops.
- AC5: Preserve existing session lifecycle ordering.
- AC6: Expected post-slave-close hangup does not require new `PtySessionError` variant.

## Tests to Pass

- **L3-T-fault_pty_edge_cases-001** (T2): inherited fds closed/CLOEXEC before exec
- **L3-T-fault_pty_edge_cases-002** (T2): EIO treated as hangup, transitions through `terminating -> exited -> reaped`
- **L3-T-fault_pty_edge_cases-003** (T2): unexpected PTY read failure follows cleanup/error path
- **L3-T-fault_pty_edge_cases-004** (T2): headless PTY starts at `120x40`
- **L3-T-fault_pty_edge_cases-005** (T2): geometry changes only on explicit operator resize
- **L3-T-fault_pty_edge_cases-006** (T2): coalesced signals act as wakeups; drain waitpid
- **L3-T-fault_pty_edge_cases-007** (T2): preserve canonical lifecycle phase ordering
- **L3-T-fault_pty_edge_cases-008** (T1): expected hangup uses existing error surface

## Dependencies

- **Blocked by:** l3-p1-tests (Phase 1 gate)
- **Blocks:** l3-p3-reaper-health
- **Phase:** 3

---

## l3-p3-reaper-health: Session Reaper And Health Extensions

- type: task
- priority: P1
- labels: phase-3,rust,pty,health
- parent: testproject1-hmw

### Description

## SOURCE OF TRUTH WARNING

- **Spec**: `docs/strategy/layers/layer-3-operational-hardening.md` — §1.3.4, §1.7 (PRIMARY)
- **TDD**: `docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-tdd.md` — L3-T-fault_process_cleanup-001..008, L3-T-fault_host_events-007..010 (PRIMARY)

## Context

Extend existing `sessions/reaper.rs` with L3 group cleanup (kill escalation, orphan sweep). Extend `pty/health.rs` with `HostSleepInterrupted` and `HostResourceExhausted` health issues.

## Files to Modify

- **Modify:** `crates/daemon/src/sessions/reaper.rs` — group-based kill escalation, orphan recovery on startup
- **Modify:** `crates/daemon/src/pty/health.rs` — add `HostSleepInterrupted`, `HostResourceExhausted` to `HealthIssue`

## Spec Requirements (verbatim)

**L3-FR-1.3.4 — `sessions/reaper.rs` owns post-session cleanup and orphan recovery**
- AC1: Extends existing module rather than introducing second cleanup-verdict implementation.
- AC2: Group-based kill escalation: in-band interrupt -> SIGTERM -> grace period -> SIGKILL -> orphan sweep.
- AC3: Daemon startup sweeps for orphaned sessions from prior runs.

**L3-FR-1.7.1 — Host sleep/wake and downtime are first-class runtime conditions**
- AC1: Active browser PTY sessions treated as tainted on host suspend.
- AC2: Stall detection uses monotonic time.
- AC3: Suspend gap above threshold emits `HOST_SLEEP_INTERRUPTED`.
- AC5: Host pressure blocks new admission while allowing running work.
- AC6: Sleep/resource classifications extend existing `HealthIssue` surface.

## Tests to Pass

- **L3-T-fault_process_cleanup-001..008** — orphan cleanup, kill escalation, startup recovery, archive blocking
- **L3-T-fault_host_events-007** (T1): sleep/resource reuse existing `HealthIssue` surface
- **L3-T-fault_host_events-008** (T1): `StallTracker` does not misclassify sleep/resource as `AGENT_STALLED`
- **L3-T-fault_host_events-009** (T1): complete `HealthIssue` inventory verified
- **L3-T-fault_host_events-010** (T1): each detector emits its own `HealthIssue`

## Dependencies

- **Blocked by:** l3-p3-pty-hardening
- **Blocks:** l3-p3-structured-tracing
- **Phase:** 3

---

## l3-p3-structured-tracing: Structured Tracing Module

- type: task
- priority: P1
- labels: phase-3,rust,tracing
- parent: testproject1-hmw

### Description

## SOURCE OF TRUTH WARNING

- **Spec**: `docs/strategy/layers/layer-3-operational-hardening.md` — §1.3.8, §4.4 (PRIMARY)
- **TDD**: `docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-tdd.md` — L3-T-observability_telemetry-001, 002 (PRIMARY)

## Context

Create `tracing/structured.rs` for control events (spawn, signal, exit, reap, cleanup verdict) and protocol events (vendor JSONL/JSON parsed into daemon events, validated checkpoints).

## Files to Create

- **Create:** `crates/daemon/src/tracing/structured.rs` — control + protocol events

## Spec Requirements (verbatim)

**L3-FR-1.3.8 — `tracing/structured.rs` records authoritative control and protocol events**
- AC1: Records control events: spawn, signal, exit, reap, cleanup verdict.
- AC2: Records protocol events: parsed vendor events and validated checkpoints.
- AC3: Protocol events remain the workflow-driving surface.

## Dependencies

- **Blocked by:** l3-p3-reaper-health
- **Blocks:** l3-p3-forensic-capture
- **Phase:** 3

---

## l3-p3-forensic-capture: Forensic Byte Capture Module

- type: task
- priority: P1
- labels: phase-3,rust,tracing
- parent: testproject1-hmw

### Description

## SOURCE OF TRUTH WARNING

- **Spec**: `docs/strategy/layers/layer-3-operational-hardening.md` — §1.3.9, §4.4 (PRIMARY)
- **TDD**: `docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-tdd.md` — L3-T-replay_pty_sessions, L3-T-replay_contract_sessions (PRIMARY)

## Context

Create `tracing/forensic.rs` for raw byte capture (PTY bytes or contract stdout/stderr) as compressed immutable artifacts.

## Files to Create

- **Create:** `crates/daemon/src/tracing/forensic.rs` — compressed immutable artifact capture

## Spec Requirements (verbatim)

**L3-FR-1.3.9 — `tracing/forensic.rs` records immutable raw session evidence**
- AC1: Captures raw PTY bytes or contract-mode stdout/stderr as compressed immutable artifacts.
- AC2: Forensic capture exists for debugging and replay, not control flow.

## Dependencies

- **Blocked by:** l3-p3-structured-tracing
- **Blocks:** l3-p3-main-tests
- **Phase:** 3

---

## l3-p3-main-tests: Phase 3 Integration And Test Files

- type: task
- priority: P1
- labels: phase-3,rust,integration,tests
- parent: testproject1-hmw

### Description

## SOURCE OF TRUTH WARNING

- **TDD**: `docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-tdd.md` — §8.1, §8.2, §8.5, §8.6

## Context

Wire Phase 3 modules into `main.rs` (mod tree + crash-recovery/evidence wiring). Create all Phase 3 test files.

## Files to Modify/Create

- **Modify:** `crates/daemon/src/main.rs` — `pty::hardening`, `tracing::structured`, `tracing::forensic` mod tree + evidence wiring
- **Create:** `crates/daemon/tests/fault_process_cleanup.rs` — 8 tests (L3-T-fault_process_cleanup-001..008)
- **Create:** `crates/daemon/tests/fault_pty_edge_cases.rs` — 8 tests (L3-T-fault_pty_edge_cases-001..008)
- **Create:** `crates/daemon/tests/replay_pty_sessions.rs` — 6 tests (L3-T-replay_pty_sessions-001..006)
- **Create:** `crates/daemon/tests/replay_contract_sessions.rs` — 6 tests (L3-T-replay_contract_sessions-001..006)

## Phase 3 Gate Criteria

- L3-T-fault_process_cleanup-001..008 pass
- L3-T-fault_pty_edge_cases-001..008 pass
- L3-T-replay_pty_sessions-001..006 pass
- L3-T-replay_contract_sessions-001..003,006 pass
- PTY sessions follow existing seven-phase lifecycle

## Dependencies

- **Blocked by:** l3-p3-forensic-capture
- **Blocks:** Phase 4 beads
- **Phase:** 3

---

## l3-p4-artifacts-extensions: Run Manifest Struct Extensions

- type: task
- priority: P1
- labels: phase-4,rust,validation,manifest
- parent: testproject1-hmw

### Description

## SOURCE OF TRUTH WARNING

- **Spec**: `docs/strategy/layers/layer-3-operational-hardening.md` — §2.1, §2.2 (PRIMARY)
- **TDD**: `docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-tdd.md` — L3-T-observability_telemetry-008..011, L3-T-validation_executor-012 (PRIMARY)

## Context

Extend `agents/artifacts.rs` with Layer 3 fields: `RunManifestFrontmatter` additions (19 fields), `DaemonEvidenceAppendix` additions (12 core + 8 L3 validator fields). This must freeze before manifest_writer and executor.

## Files to Modify

- **Modify:** `crates/daemon/src/agents/artifacts.rs` — L3 frontmatter + appendix field additions

## Spec Requirements (verbatim, from §2.1–2.2)

**L3-FR-2.1.2 — Agent-claimed frontmatter uses a stable claimed-field contract**
- AC1: Frontmatter includes `manifest_version`, skill identity/version, job/session identity, execution mode, timing, `outcome_class`, item-count claims, `warnings_count`, and claimed page/source/locale/currency state.
- AC2: Agent-self-reported fields use the `agent_claimed_*` prefix.
- AC3: `agent_narrative_missing` is explicit frontmatter field.

**L3-FR-2.1.4 — The daemon appendix is machine-authored, discrepancy-aware, and fallback-safe**
- AC1: Appendix records observed URL/page-kind sequences, daemon-observed states, auth events, fallback depth, counts, OQG summary, samples, artifact refs.
- AC4: Target size < 50 KB.
- AC5: Size-budget overflow handled by bounded evidence elision before fallback.

**L3-FR-2.2.1 — Layer 3 appends validator-specific evidence fields**
- AC1: `basis_sufficiency`, `page_fingerprint_hash`, `auth_challenge_detected`, `extraction_fill_rate_bps`, `checkpoint_count`, `byte_count_compressed`, `validator_rubric_hash`, `drift_delta_json`.

## Tests to Pass

- **L3-T-observability_telemetry-008** (T1): every frontmatter field has assertion-bearing test
- **L3-T-observability_telemetry-009** (T1): frontmatter key parity asserted by exact field name
- **L3-T-observability_telemetry-010** (T1): core appendix key parity asserted by exact field name
- **L3-T-observability_telemetry-011** (T1): L3 appendix additions asserted by field name and absence-handling

## Dependencies

- **Blocked by:** l3-p2-tests (Phase 2 gate), l3-p3-main-tests (Phase 3 gate)
- **Blocks:** l3-p4-manifest-writer
- **Phase:** 4

---

## l3-p4-manifest-writer: Run Manifest Writer

- type: task
- priority: P1
- labels: phase-4,rust,validation
- parent: testproject1-hmw

### Description

## SOURCE OF TRUTH WARNING

- **Spec**: `docs/strategy/layers/layer-3-operational-hardening.md` — §1.3.12, §2.1 (PRIMARY)
- **TDD**: `docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-tdd.md` — L3-T-validation_executor-012 (PRIMARY)

## Context

Create `validation/manifest_writer.rs` that produces hybrid-authored run manifests after every skill run. Agent narrative + daemon evidence appendix. Fallback machine-authored manifest if agent fails to narrate.

## Files to Create

- **Create:** `crates/daemon/src/validation/manifest_writer.rs`

## Spec Requirements (verbatim)

**L3-FR-1.3.12 — `validation/manifest_writer.rs` produces hybrid-authored run manifests**
- AC1: Writes agent narrative content plus daemon-authored evidence appendix.
- AC2: If agent does not narrate, emits machine-authored fallback with `agent_narrative_missing: true`.
- AC3: Target size < 50 KB.
- AC4: Extends current `RunManifestFrontmatter` and `DaemonEvidenceAppendix` in `artifacts.rs`.

## Dependencies

- **Blocked by:** l3-p4-artifacts-extensions
- **Blocks:** l3-p4-validation-policy, l3-p4-contracts-checkpoints, l3-p4-validator-skill
- **Phase:** 4

---

## l3-p4-contracts-checkpoints: Contracts Extensions And Checkpoint Store

- type: task
- priority: P1
- labels: phase-4,rust,validation
- parent: testproject1-hmw

### Description

## SOURCE OF TRUTH WARNING

- **Spec**: `docs/strategy/layers/layer-3-operational-hardening.md` — §1.3.7, §2.4, §2.5 (PRIMARY)
- **TDD**: `docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-tdd.md` — L3-T-fault_partial_failure-001..007 (PRIMARY)

## Context

Extend `agents/contracts.rs` with L3 validation dispositions and checkpoint sequencing. Create `db/checkpoints.rs` for checkpoint persistence with validation. Parallel with policy and skill lanes after manifest contract frozen.

## Files to Modify/Create

- **Modify:** `crates/daemon/src/agents/contracts.rs` — L3 validation disposition additions, outcome class alignment
- **Create:** `crates/daemon/src/db/checkpoints.rs` — checkpoint validation + persistence

## Spec Requirements (verbatim)

**L3-FR-1.3.7 — `db/checkpoints.rs` persists validated checkpoint state**
- AC1: Layer 3 includes `db/checkpoints.rs`.
- AC2: Validates checkpoint payloads before persisting.
- AC3: Updates `workflow_jobs` with checkpoint reference, keeps artifact-store linkage explicit.
- AC4: Preserves existing `persist_validated_checkpoint` sequence rules in `agents/contracts.rs`.

**L3-FR-2.5.1 — Layer 3 standardizes six outcome classes**
- AC1: Skills return exactly one of `completed`, `degraded_completed`, `continuation_required`, `blocked_human_action`, `suspect_output`, or `failed`.
- AC2: Each outcome maps to daemon response from spec table.

**L3-FR-2.5.2 — Continuation chains validated per segment, closed as one chain**
- AC1: `continuation_required` is clean boundary with committed work.
- AC2: Job Runner enqueues follow-on segment.
- AC3: Final stage head does not advance until chain-close aggregate validation passed.

## Tests to Pass

- **L3-T-fault_partial_failure-001** (T3): valid checkpoint then crash preserves state
- **L3-T-fault_partial_failure-002** (T2): out-of-order checkpoint replay rejected
- **L3-T-fault_partial_failure-003** (T2): continuation crash leaves chain open
- **L3-T-fault_partial_failure-004** (T2): additive partial commits preserve successful siblings
- **L3-T-fault_partial_failure-005** (T2): schema-invalid result spawns no validator
- **L3-T-fault_partial_failure-006** (T3): rendered-but-unpersisted verdict is validator failure
- **L3-T-fault_partial_failure-007** (T2): `db/checkpoints.rs` validates before persisting

## Dependencies

- **Blocked by:** l3-p4-manifest-writer (appendix contract frozen)
- **Blocks:** l3-p4-validation-executor
- **Phase:** 4

---

## l3-p4-validation-policy: Validation Policy Engine

- type: task
- priority: P1
- labels: phase-4,rust,validation
- parent: testproject1-hmw

### Description

## SOURCE OF TRUTH WARNING

- **Spec**: `docs/strategy/layers/layer-3-operational-hardening.md` — §1.3.10, §2.3 (PRIMARY)
- **TDD**: `docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-tdd.md` — L3-T-validation_policy-001..018 (PRIMARY)

## Context

Create `validation/policy.rs` that determines whether a completed job requires semantic validation. Reads policy from skill metadata. Four modes: always/sample/trigger/skip. Risk-stratified sampling. Per-slice enforcement tracking.

## Files to Create

- **Create:** `crates/daemon/src/validation/policy.rs`

## Spec Requirements (verbatim from §2.3)

**L3-FR-1.3.10 — `validation/policy.rs` decides validation eligibility**
- AC1: Evaluates policy mode, promotion stage, trigger conditions, risk factors.
- AC2: Policy changes from metadata/configuration, not code changes per skill.

**L3-FR-2.3.1** through **L3-FR-2.3.10** — Full policy engine specification including:
- Schema-invalid outputs never spawn validators (AC for 2.3.1)
- Pre-stable stages always validate (2.3.2)
- Four policy modes: always/sample/trigger/skip (2.3.3–2.3.6)
- 9 risk factors forcing sample probability to 1.0 (2.3.4)
- 8 extended trigger conditions (2.3.7)
- Per-slice enforcement key: worker skill x source x worker behavior version x validator behavior version (2.3.8)
- Automatic demotion triggers (2.3.9)
- Dry-run bootstrap validation ladder (2.3.10)

## Tests to Pass

- **L3-T-validation_policy-001..018** — all 18 policy tests covering every mode, trigger, risk factor, enforcement slice, demotion, and bootstrap ladder behavior

## Dependencies

- **Blocked by:** l3-p4-manifest-writer (appendix contract frozen)
- **Blocks:** l3-p4-validation-executor
- **Phase:** 4

---

## l3-p4-validator-skill: Base Validator Skill Directory

- type: task
- priority: P1
- labels: phase-4,skills,filesystem,validation
- parent: testproject1-hmw

### Description

## SOURCE OF TRUTH WARNING

- **Spec**: `docs/strategy/layers/layer-3-operational-hardening.md` — §1.9, §2.8, §3.3 (PRIMARY)
- **TDD**: `docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-tdd.md` — L3-T-validation_golden_verdicts-001 (PRIMARY)

## Context

Create the base validator skill directory with 7 files (6 standard + 1 reference index). Generic semantic quality checks applicable to any skill. Parallel with policy and contracts lanes.

## Files to Create

- **Create:** `skills/validation/base/SKILL.md` — generic quality check playbook
- **Create:** `skills/validation/base/result.schema.json` — verdict output schema
- **Create:** `skills/validation/base/checkpoint.schema.json` — no-op checkpoint
- **Create:** `skills/validation/base/canary.inputs.jsonl` — known-good/bad manifests
- **Create:** `skills/validation/base/golden.assertions.yaml` — expected verdicts
- **Create:** `skills/validation/base/validator-rubric.yaml` — distilled runtime rubric
- **Create:** `skills/validation/base/references/manifest-corpus-index.md` — corpus index

## Spec Requirements (verbatim)

**L3-FR-2.8.1 — The base validator skill directory is explicitly specified**
- AC1: Includes all 7 files listed above.
- AC2: Sufficient for runtime validation plus synthetic canaries.
- AC3: Validator skills remain standard skill directories.

**L3-FR-2.8.2 — The base validator checks generic semantic quality**
- AC1: Checks manifest completeness, agent-vs-daemon agreement, result-sample consistency, narrative plausibility, `basis_sufficiency`, drift deltas.
- AC2: Thin evidence → `basis_sufficiency: low` rather than guessing.
- AC3: Domain-specific rules, schema compliance, raw forensic bytes out of scope.

## Tests to Pass

- **L3-T-validation_golden_verdicts-001** (T1): `skills/validation/base/` contains all 7 required files

## Dependencies

- **Blocked by:** l3-p4-manifest-writer (appendix/rubric contract frozen)
- **Blocks:** l3-p4-validation-executor
- **Phase:** 4

---

## l3-p4-validation-executor: Validation Executor

- type: task
- priority: P1
- labels: phase-4,rust,validation
- parent: testproject1-hmw

### Description

## SOURCE OF TRUTH WARNING

- **Spec**: `docs/strategy/layers/layer-3-operational-hardening.md` — §1.3.11, §2.4 (PRIMARY)
- **TDD**: `docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-tdd.md` — L3-T-validation_executor-001..012 (PRIMARY)

## Context

Create `validation/executor.rs` that spawns validator sub-jobs via Job Runner. Composes kickoff prompt from run manifest, rubric, drift deltas, OQG signals, and anomaly-stratified sample. Convergence point — depends on all parallel Phase 4 lanes.

## Files to Create

- **Create:** `crates/daemon/src/validation/executor.rs`

## Spec Requirements (verbatim)

**L3-FR-1.3.11 — `validation/executor.rs` runs validator sub-jobs through the existing job runner**
- AC1: Spawns validator sub-jobs via Job Runner linked through `parent_job_id`.
- AC2: Composes validator inputs from run manifest, result sample, rubric, OQG signals, drift deltas.

**L3-FR-2.4.1** through **L3-FR-2.4.4** — Full executor specification including:
- Layer 1 job/session boundary reuse (2.4.1)
- Bounded structured evidence kickoff (2.4.2)
- Low-cost contract-mode execution, base validator only (2.4.3)
- Verdict output contract and handling matrix (2.4.4)

## Tests to Pass

- **L3-T-validation_executor-001..012** — all 12 executor tests covering sub-job spawning, kickoff composition, verdict parsing, fail-open handling, continuation chain support, manifest writer integration

## Dependencies

- **Blocked by:** l3-p4-contracts-checkpoints, l3-p4-validation-policy, l3-p4-validator-skill
- **Blocks:** l3-p4-runner-main-integration
- **Phase:** 4

---

## l3-p4-runner-main-integration: Phase 4 Validation Runner And Bootstrap Integration

- type: task
- priority: P1
- labels: phase-4,rust,integration
- parent: testproject1-hmw

### Description

## SOURCE OF TRUTH WARNING

- **Spec**: `docs/strategy/layers/layer-3-operational-hardening.md` — §2.4, §2.5, §2.6
- **Strategy**: Phase 4 convergence — runner.rs and main.rs

## Context

Wire validation into runner completion paths: `persist_success` and `persist_continuation` gain validation-aware handling. `main.rs` gains `db::checkpoints` and `validation::*` mod tree plus validator/session-completion integration.

## Files to Modify

- **Modify:** `crates/daemon/src/jobs/runner.rs` — `persist_success` / `persist_continuation` validation-aware completion + validator sub-job enqueue
- **Modify:** `crates/daemon/src/main.rs` — `db::checkpoints`, `validation::*` mod tree + validator runtime wiring

## Dependencies

- **Blocked by:** l3-p4-validation-executor
- **Blocks:** l3-p4-tests
- **Phase:** 4

---

## l3-p4-policy-tests: Validation Policy Test File

- type: task
- priority: P1
- labels: phase-4,rust,tests
- parent: testproject1-hmw

### Description

## SOURCE OF TRUTH WARNING

- **TDD**: `docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-tdd.md` — §8.12

## Context

Create `validation_policy.rs` test file with all 18 policy tests covering every mode/trigger/risk combination.

## Files to Create

- **Create:** `crates/daemon/tests/validation_policy.rs` — 18 tests (L3-T-validation_policy-001..018)

## Dependencies

- **Blocked by:** l3-p4-validation-policy (policy decision keys frozen)
- **Blocks:** Phase 6 beads
- **Phase:** 4

---

## l3-p4-executor-dry-run-tests: Validation Executor And Dry-Run Test Files

- type: task
- priority: P1
- labels: phase-4,rust,tests
- parent: testproject1-hmw

### Description

## SOURCE OF TRUTH WARNING

- **TDD**: `docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-tdd.md` — §8.13, §8.14

## Context

Create test files for validation executor (12 tests) and dry-run isolation (10 tests).

## Files to Create

- **Create:** `crates/daemon/tests/validation_executor.rs` — 12 tests (L3-T-validation_executor-001..012)
- **Create:** `crates/daemon/tests/validation_dry_run.rs` — 10 tests (L3-T-validation_dry_run-001..010)

## Dependencies

- **Blocked by:** l3-p4-runner-main-integration
- **Blocks:** Phase 6 beads
- **Phase:** 4

---

## l3-p4-golden-partial-tests: Golden Verdicts And Partial Failure Test Files

- type: task
- priority: P1
- labels: phase-4,rust,tests
- parent: testproject1-hmw

### Description

## SOURCE OF TRUTH WARNING

- **TDD**: `docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-tdd.md` — §8.4, §8.15, §8.20 (partial)

## Context

Create test files for golden verdicts (7 tests), partial failure scenarios (7 tests), and expand observability_telemetry.rs with Phase 4 manifest parity tests.

## Files to Create

- **Create:** `crates/daemon/tests/validation_golden_verdicts.rs` — 7 tests (L3-T-validation_golden_verdicts-001..007)
- **Create:** `crates/daemon/tests/fault_partial_failure.rs` — 7 tests (L3-T-fault_partial_failure-001..007)
- **Modify:** `crates/daemon/tests/observability_telemetry.rs` — add L3-T-observability_telemetry-008..011

## Phase 4 Gate Criteria

- L3-T-validation_policy-001..018 pass
- L3-T-validation_executor-001..012 pass
- L3-T-validation_dry_run-001..010 pass
- L3-T-validation_golden_verdicts-001..007 pass
- L3-T-fault_partial_failure-001..007 pass
- L3-T-observability_telemetry-008..011 pass
- CI-L3-007 green

## Dependencies

- **Blocked by:** l3-p4-runner-main-integration
- **Blocks:** Phase 6 beads
- **Phase:** 4

---

## l3-p5-relation-engine: Process Relation Resolution Engine

- type: task
- priority: P1
- labels: phase-5,typescript,relations
- parent: testproject1-hmw

### Description

## SOURCE OF TRUTH WARNING

- **Spec**: `docs/strategy/layers/layer-3-operational-hardening.md` — §3.6 (PRIMARY)
- **TDD**: `docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-tdd.md` — L3-T-resolve_process_relations-001..015 (PRIMARY)
- **Strategy**: `docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-strategy.md` — Phase 5, parallel side lane

## Context

Create the TypeScript relation resolution engine. Classifies notice-to-process relations using hard blockers, deterministic rules, then heuristic scoring. This is the Phase 5 TS side lane — depends only on Phase 1 schema.

## Language & Quality Gates

- **Language:** `typescript`
- **Quality gate commands:**
  - `cd packages/intelligence && bun run test`

## Files to Create

- **Create:** `packages/intelligence/src/relations/resolve-process-relations.ts`

## Spec Requirements (verbatim from §3.6)

**L3-FR-3.6.1** through **L3-FR-3.6.8** — Full relation engine specification including:
- `merged_same_process` for true single-process identity (3.6.1)
- `related_not_merged` preserves relational context (3.6.2)
- `possible_duplicate` for operator review (3.6.3)
- `separate` — explicit no-relationship (3.6.4)
- Blockers first, deterministic rules second, heuristics last (3.6.5)
- Stable identity through aliases and `canonical_process_id` (3.6.6)
- Late merges as decision-staleness events (3.6.7)
- Cluster visibility before perfect merge certainty (3.6.8)

Resolution algorithm: hard blockers → exact-match rules → heuristic scoring (6-weight formula) → edge creation → alias management → cluster identity → late merge handling.

## Tests to Pass

- **L3-T-resolve_process_relations-001..015** — all 15 relation tests covering OCID merge, linked chains, framework call-off, blockers, threshold boundaries, transitive roots, late merge, cluster visibility

## Dependencies

- **Blocked by:** l3-p1-tests (Phase 1 gate — schema must exist)
- **Blocks:** l3-p5-relation-tests
- **Phase:** 5

---

## l3-p5-relation-tests: Process Relation Test File

- type: task
- priority: P1
- labels: phase-5,typescript,tests
- parent: testproject1-hmw

### Description

## SOURCE OF TRUTH WARNING

- **TDD**: `docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-tdd.md` — §8.18

## Context

Create the Vitest test file for the relation resolution engine with all 15 tests.

## Files to Create

- **Create:** `packages/intelligence/src/relations/resolve-process-relations.test.ts` — 15 tests

## Dependencies

- **Blocked by:** l3-p5-relation-engine
- **Blocks:** l3-p5-package-config
- **Phase:** 5

---

## l3-p5-error-renderer: Error Manifest Renderer And Template

- type: task
- priority: P1
- labels: phase-5,typescript,manifests
- parent: testproject1-hmw

### Description

## SOURCE OF TRUTH WARNING

- **Spec**: `docs/strategy/layers/layer-3-operational-hardening.md` — §1.8 (PRIMARY)
- **TDD**: `docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-tdd.md` — L3-T-render_error_manifest-001..004 (PRIMARY)

## Context

Create the shared error manifest renderer and Markdown template in the manifests package.

## Language & Quality Gates

- **Language:** `typescript`
- **Quality gate commands:**
  - `cd packages/manifests && bun test`

## Files to Create

- **Create:** `packages/manifests/render-error-manifest.ts` — error manifest renderer
- **Create:** `packages/manifests/error.md.ts` — error manifest Markdown template
- **Create:** `packages/manifests/tests/render-error-manifest.test.ts` — 4 tests

## Spec Requirements (verbatim)

**L3-FR-1.8.1 — Layer 3 activates shared-library modules for relations and error rendering**
- AC2: `packages/manifests/render-error-manifest.ts` and `packages/manifests/error.md.ts` exist as shared surfaces.
- AC3: Package-level functionality; does not move workflow state out of daemon.

## Tests to Pass

- **L3-T-render_error_manifest-001** (T1): rendered markdown includes error chain, context, debugging action, artifacts
- **L3-T-render_error_manifest-002** (T1): logical source-health classes render using mapped `ErrorCode` strings
- **L3-T-render_error_manifest-003** (T1): resolving error manifest flips metadata, preserves original
- **L3-T-render_error_manifest-004** (T1): hostile error context escaped/fenced

## Dependencies

- **Blocked by:** l3-p1-tests (Phase 1 gate — schema/contracts frozen)
- **Blocks:** l3-p5-package-config
- **Phase:** 5

---

## l3-p5-package-config: Package Configuration Updates

- type: task
- priority: P1
- labels: phase-5,typescript,config
- parent: testproject1-hmw

### Description

## SOURCE OF TRUTH WARNING

- **Strategy**: Phase 5 — package.json updates for both intelligence and manifests packages

## Context

Update both `package.json` files to include new test scripts and any needed dependencies for the relation engine and error renderer.

## Files to Modify

- **Modify:** `packages/intelligence/package.json` — test script for relations
- **Modify:** `packages/manifests/package.json` — test script for error renderer

## Phase 5 Gate Criteria

- `bun run test`, `(cd packages/intelligence && bun run test)`, `(cd packages/manifests && bun test)` pass
- CI-L3-004 green
- L3-T-resolve_process_relations-001..015 pass
- L3-T-render_error_manifest-001..004 pass

## Dependencies

- **Blocked by:** l3-p5-relation-tests, l3-p5-error-renderer
- **Blocks:** Phase 6 beads
- **Phase:** 5

---

## l3-p6-promotion-bookkeeping: Skill Promotion And Behavior-Version Tracking

- type: task
- priority: P2
- labels: phase-6,rust,capstone,promotion
- parent: testproject1-hmw

### Description

## SOURCE OF TRUTH WARNING

- **Spec**: `docs/strategy/layers/layer-3-operational-hardening.md` — §3.1, §3.2 (PRIMARY)
- **TDD**: `docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-tdd.md` — L3-T-skill_golden_journeys-001..008, L3-T-skill_shadow_compare-001..007 (PRIMARY)

## Context

Implement the six-stage promotion flow (draft → candidate → canary → shadow → stable → rollback) and five independent version dimensions. Promotion gates include validator approval rate.

## Spec Requirements (verbatim from §3.1–3.2)

**L3-FR-3.1.1** through **L3-FR-3.1.7** — Full promotion flow: draft manual-only, candidate dry-run, canary bounded traffic, shadow parallel comparison, stable default, rollback rebinding, linear promotion with validator gates.

**L3-FR-3.2.1 — Five version dimensions tracked independently**
- `skill_version`, `result_schema_version`, `checkpoint_schema_version`, `validator_overlay_version`, `source_structure_version`

**L3-FR-3.2.2 — Behavior-version tracking protects calibration from cosmetic churn**
- `worker_behavior_version` from semantic changes only; cosmetic edits don't reset readiness.

## Dependencies

- **Blocked by:** l3-p4-policy-tests, l3-p4-executor-dry-run-tests, l3-p4-golden-partial-tests, l3-p5-package-config
- **Blocks:** l3-p6-canary-blackout
- **Phase:** 6

---

## l3-p6-canary-blackout: Validator Canary Health And Blackout Orchestration

- type: task
- priority: P2
- labels: phase-6,rust,capstone,validation
- parent: testproject1-hmw

### Description

## SOURCE OF TRUTH WARNING

- **Spec**: `docs/strategy/layers/layer-3-operational-hardening.md` — §2.7 (PRIMARY)
- **TDD**: `docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-tdd.md` — L3-T-validation_canary_health-001..008, L3-T-validation_retro_backlog-001..006 (PRIMARY)

## Context

Implement synthetic validator canaries (15-30 min), health state transitions (healthy → degraded → blind), auto-demotion, validation blackout banner, and retroactive validation backlog with priority-ordered recovery.

## Spec Requirements (verbatim from §2.7)

**L3-FR-2.7.1** — Validator failures never block worker availability. Fail-open + retroactive backlog.
**L3-FR-2.7.2** — Trust-state lattice: verified/flagged/unverified/suspect.
**L3-FR-2.7.3** — Process-level validation rollups coarser than per-output trust.
**L3-FR-2.7.4** — Validator health tracked as healthy/degraded/blind with concrete thresholds.
**L3-FR-2.7.5** — Synthetic canaries + retroactive backlog recovery keep fail-open auditable. Backlog items don't silently expire.

## Dependencies

- **Blocked by:** l3-p6-promotion-bookkeeping
- **Blocks:** l3-p6-status-surfaces
- **Phase:** 6

---

## l3-p6-status-surfaces: Read-Only Operational Status Builders

- type: task
- priority: P2
- labels: phase-6,rust,capstone,status
- parent: testproject1-hmw

### Description

## SOURCE OF TRUTH WARNING

- **Spec**: `docs/strategy/layers/layer-3-operational-hardening.md` — §4.4 (PRIMARY)
- **TDD**: `docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-tdd.md` — L3-T-observability_telemetry-013..015 (PRIMARY)

## Context

Build read-only operational snapshot builders in `main.rs` / `build_handshake_context()` for cron status, source health, validation state, and arbiter state. These feed the command router.

## Files to Modify

- **Modify:** `crates/daemon/src/main.rs` — `serve_handshake_service()` / `build_handshake_context()` read-only status exposure

## Dependencies

- **Blocked by:** l3-p6-canary-blackout
- **Blocks:** l3-p6-command-router
- **Phase:** 6

---

## l3-p6-command-router: IPC Operational Status Commands

- type: task
- priority: P2
- labels: phase-6,rust,capstone,ipc
- parent: testproject1-hmw

### Description

## SOURCE OF TRUTH WARNING

- **TDD**: `docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-tdd.md` — L3-T-observability_telemetry-013..015

## Context

Add read-only L3 operational status methods to `ipc/command_router.rs`: `get_cron_status`, `get_source_health_status`, `get_validation_status`, `get_resource_arbiter_status`.

## Files to Modify

- **Modify:** `crates/daemon/src/ipc/command_router.rs` — 4 new read-only status methods

## Tests to Pass

- **L3-T-observability_telemetry-013** (T2): `get_cron_status` read-only snapshot
- **L3-T-observability_telemetry-014** (T2): `get_source_health_status` read-only
- **L3-T-observability_telemetry-015** (T2): `get_validation_status` and `get_resource_arbiter_status` read-only

## Dependencies

- **Blocked by:** l3-p6-status-surfaces
- **Blocks:** l3-p6-runner-main-capstone
- **Phase:** 6

---

## l3-p6-runner-main-capstone: Capstone Runner And Main Integration

- type: task
- priority: P2
- labels: phase-6,rust,capstone,integration
- parent: testproject1-hmw

### Description

## SOURCE OF TRUTH WARNING

- **Strategy**: Phase 6 — final telemetry/observability drill-down and unattended rehearsal wiring

## Context

Final `jobs/runner.rs` touch (`mark_obsolete_and_enqueue_rerun`) and `main.rs` capstone wiring for unattended operation. This is the last integration bead.

## Files to Modify

- **Modify:** `crates/daemon/src/jobs/runner.rs` — `mark_obsolete_and_enqueue_rerun` stale/recovery
- **Modify:** `crates/daemon/src/main.rs` — final unattended rehearsal wiring

## Dependencies

- **Blocked by:** l3-p6-command-router
- **Blocks:** l3-p6-lifecycle-tests, l3-p6-capstone-tests
- **Phase:** 6

---

## l3-p6-lifecycle-tests: Skill Lifecycle Test Files

- type: task
- priority: P2
- labels: phase-6,rust,tests,capstone
- parent: testproject1-hmw

### Description

## SOURCE OF TRUTH WARNING

- **TDD**: `docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-tdd.md` — §8.9, §8.10

## Context

Create test files for skill golden journeys (8 tests) and shadow comparison (7 tests).

## Files to Create

- **Create:** `crates/daemon/tests/skill_golden_journeys.rs` — 8 tests (L3-T-skill_golden_journeys-001..008)
- **Create:** `crates/daemon/tests/skill_shadow_compare.rs` — 7 tests (L3-T-skill_shadow_compare-001..007)

## Dependencies

- **Blocked by:** l3-p6-runner-main-capstone
- **Blocks:** none (end of chain)
- **Phase:** 6

---

## l3-p6-capstone-tests: Capstone Validation And Observability Test Files

- type: task
- priority: P2
- labels: phase-6,rust,tests,capstone
- parent: testproject1-hmw

### Description

## SOURCE OF TRUTH WARNING

- **TDD**: `docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-tdd.md` — §8.3, §8.16, §8.17, §8.20

## Context

Create capstone test files and finalize observability_telemetry.rs with all 15 tests.

## Files to Create

- **Create:** `crates/daemon/tests/validation_canary_health.rs` — 8 tests (L3-T-validation_canary_health-001..008)
- **Create:** `crates/daemon/tests/validation_retro_backlog.rs` — 6 tests (L3-T-validation_retro_backlog-001..006)
- **Create:** `crates/daemon/tests/fault_host_events.rs` — 10 tests (L3-T-fault_host_events-001..010)
- **Modify:** `crates/daemon/tests/observability_telemetry.rs` — finalize all 15 tests (L3-T-observability_telemetry-001..015)

## Phase 6 Gate Criteria

- All Phase 6 tests pass: skill_golden_journeys, skill_shadow_compare, validation_canary_health, validation_retro_backlog, observability_telemetry, fault_host_events
- CI-L3-001 through CI-L3-008 all green
- Capstone milestone hold: Manual Gate remains open for 5-day unattended rehearsal

## Dependencies

- **Blocked by:** l3-p6-runner-main-capstone
- **Blocks:** none (end of chain)
- **Phase:** 6
