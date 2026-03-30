# L3 Operational Hardening — Optimize Spec (Fresh-Eyes Audit)

## Your Mission

You are performing a **comprehensive fresh-eyes audit** of the L3 Operational Hardening spec — the execution-grade specification for adding cron scheduling, resource management, PTY hardening, relation resolution, source health monitoring, skill promotion, and the worker-validator system (dry-run) to the Rust daemon.

**You are reading, analyzing, and improving an existing document — not writing code.**

---

## What To Read (Do This First)

### The Document to Audit (TARGET)
```
docs/strategy/layers/layer-3-operational-hardening.md    — Spec (SOT #1, validated in step 01) — READ FULLY
```

### Verify Against (SOURCE OF TRUTH)
```
docs/strategy/layer-strategy.md                          — Cross-layer strategy — READ §5 (L3), §9-§13
docs/strategy/cross-reference-index.md                   — Topic lookup — REFERENCE
```

### Codebase (GROUND TRUTH)
```
crates/daemon/src/main.rs                                — Daemon entry point, module declarations (131K)
crates/daemon/src/jobs/runner.rs                         — Job lifecycle, EnqueueJobRequest, ClaimedJob (25K)
crates/daemon/src/sessions/reaper.rs                     — Process cleanup, CleanupVerdict (17K)
crates/daemon/src/agents/artifacts.rs                    — RunManifestFrontmatter, DaemonEvidenceAppendix (15K)
crates/daemon/src/agents/contracts.rs                    — ValidationOutcome, StageValidationWrite (9.2K)
crates/daemon/src/errors/manifest.rs                     — ErrorCode enum (29 variants), ErrorRuntimeContext (14K)
crates/daemon/src/pty/session.rs                         — SessionPhase, PtyGeometry, SpawnOptions (15K)
crates/daemon/src/pty/health.rs                          — HealthIssue, StallTracker (3.3K)
crates/daemon/src/agents/adapter.rs                      — AgentAdapter trait, InteractionProfile (3.7K)
crates/daemon/src/db/sqlite.rs                           — StateDb, StateDbConfig (9.3K)
crates/daemon/src/ipc/command_router.rs                  — CommandRouter dispatch (9.2K)
packages/intelligence/src/qualification/goods-qualification.ts — Existing TS intelligence module
packages/manifests/render-opportunity.ts                 — Existing TS manifest rendering
```

### Quality Bar (MATCH OR EXCEED)
```
docs/pipeline/l1-daemon-core/artifacts/l1-operational-hardening-tdd.md — Best existing TDD (study structure)
docs/strategy/layers/layer-1-thin-local-runner.md         — Best existing layer spec
```

---

## Think Independently

Don't just check for obvious errors. Think critically about these 8 areas:

### 1. Codebase Alignment — Do the Types Match?
Open the actual Rust types. Does the spec's description of existing types match reality?
- `OutcomeClass` in `agents/artifacts.rs` has 4 variants currently. The spec (§2.5) defines 6. Are the 2 new ones (`suspect_output`, `failed`) explicitly called out as additions?
- `ErrorCode` has 29 variants. §3.4 mentions new error codes. Are all new codes listed explicitly?
- `SessionPhase` has 7 phases. Does §1.6 PTY hardening integrate with the correct phases?
- `ValidationOutcome` and `ValidationDisposition` exist in `contracts.rs`. Does §2.4 verdict handling build on these or replace them?

### 2. Schema Consistency — Does the DDL Hold Up?
- §4.1 shows 5 new tables with full DDL. Cross-check `validation_verdicts.worker_job_id` references `workflow_jobs(id)` — is this FK valid given the L0 DDL?
- `listing_evidence` has `unique (source, source_listing_id)` — is this the right uniqueness constraint?
- `process_relations` references `procurement_processes(id)` — does this table exist in L0?
- `source_runtime_state` references `sources(id)` — does the `sources` table exist in L0?
- §4.2 modified tables: are the column additions backward-compatible with L0's DDL?

### 3. Integration Flow Completeness — Is the Pipeline Connected?
Trace the full flow for a cron-triggered ingestion:
1. Cron fires → scheduler creates job → resource arbiter admits → PTY session spawns → skill runs → result contract validates → run manifest writes → validation policy evaluates → validator sub-job spawns (if needed) → verdict renders → job advances
- Is every handoff between components explicitly defined?
- What happens at each failure point?
- Where do the 13 skill-level telemetry metrics (§4.5) get updated?

### 4. Concurrency Model — Are Race Conditions Addressed?
- §1.3 specifies `jobs/claim.rs` for concurrent job claiming. But the current `JobRunner::claim_next_job` in `jobs/runner.rs` is single-threaded. How does the new claim.rs interact with the existing runner?
- Resource arbiter admits jobs — but who releases slots? Is release explicit or on session completion?
- Source circuit breaker state changes — what happens if two concurrent health canaries both detect failure?
- Validation executor spawns sub-jobs via Job Runner — can a validator job claim resources from the arbiter?

### 5. Fail-Open Semantics — Are Edge Cases Truly Covered?
§2.7 declares fail-open as "constitutional." Verify:
- What if the validator produces a VALID verdict but with an UNKNOWN verdict value (not approved/rejected/flagged)?
- What if the run manifest is too large (>50KB target) for the validator prompt context?
- What if the validator crashes AFTER rendering a verdict but BEFORE it's persisted?
- What if the OQG signals the validator relies on are themselves unavailable?
- The retroactive validation backlog (§2.7): what's the maximum backlog size? Is there a TTL after which backlog items expire?

### 6. Relation Resolution — Is the Algorithm Deterministic?
§3.6 defines a scoring algorithm. Check:
- Are the weights (0.35 reference_match + 0.20 buyer_match + ...) stable across runs?
- What happens when a new notice arrives that changes a prior `separate` classification to `related_not_merged`?
- The `canonical_process_id` assignment — what if process A and B merge, then B and C merge independently?
- Late merge as decision-staleness event — what if the operator hasn't reviewed the process yet?

### 7. Observability Gaps — Can You Debug a Failure at 3 AM?
- §4.4 defines four observability layers. Is there a clear way to correlate across layers?
- If a validator rejects a run at 3 AM and the operator checks at 9 AM, can they trace from verdict → manifest → forensic bytes → original session?
- Are there structured log event types for circuit breaker state changes?
- What's the retention policy for forensic bytes vs run manifests vs validation verdicts?

### 8. Test Coverage — Does §5 Match the Spec?
- §5.2 claims 18 new test files, cumulative 38. Count the actual test files in the table — does it add up?
- §5.3 has 14 scenarios. Do they cover all 12 new daemon components?
- Are there scenarios for the telemetry metrics (§4.5, §4.6)?
- Is there a test scenario for the full cron → ingestion → validation → dry-run verdict flow?
- Missing scenario: what about concurrent relation resolution where two agents process overlapping notice pairs?

---

## Drift Checklist

- [ ] Every spec requirement is represented in this document
- [ ] No content was silently dropped or weakened from the research doc
- [ ] No assumptions made that aren't in the spec
- [ ] DDL matches the table descriptions in prose
- [ ] Error codes referenced in prose exist in the spec's error taxonomy
- [ ] Telemetry metrics in §4.5/4.6 match the descriptions in §2/§3
- [ ] Cross-domain notes (HTML comments) at EOF are still accurate
- [ ] Forward-compatibility provisions (§6.3) match Layer 4's expectations
- [ ] Dependencies (§6.4) include all crates actually needed (is `sysinfo` sufficient for host resource detection?)
- [ ] The 14 DoD criteria in §5.1 cover all major spec sections

---

## Follow-Up Pass (Run After Fixes)

> After making your fixes, send this as a follow-up message in the same session.

ULTRATHINK. Re-read the fixes you just made. Did they introduce new issues?

1. **If you added requirement IDs:** verify the numbering is sequential with no gaps. Count total IDs and compare to the L1 spec's 138 — is the ratio reasonable given L3's scope?

2. **If you modified DDL:** run the CREATE TABLE statements mentally. Do all foreign keys reference tables that exist? Do all column types have SQLite equivalents?

3. **If you added acceptance criteria:** are they truly testable? Could you write a test function name for each one? Try naming 5 test functions from the ACs you added.

4. **Open `docs/strategy/layer-strategy.md` §11** — does the cross-layer test count still add up? L3 claims 18 new (cumulative 38). L4 should build on 38. Verify the chain is unbroken.

5. **Check `docs/strategy/cross-reference-index.md`** — do any of your changes require new entries in the cross-reference index? (New topics, renamed concepts, moved sections.)

Report the total requirement count, any remaining gaps, and confidence level (high/medium/low) that this spec is ready for TDD generation.
