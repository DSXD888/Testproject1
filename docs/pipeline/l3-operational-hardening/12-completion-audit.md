# L3 Operational Hardening — Completion Audit

## First Action

Invoke the `/bead-completion-audit` skill.

---

## Your Mission

Deep-read the spec, TDD, and codebase for L3 Operational Hardening. Find EVERY gap between what the spec requires and what's actually implemented. Create fix beads for gaps. Do NOT execute them — just create and scope.

**This is the verification loop.** It runs after all beads are executed. It checks whether the implementation matches the spec at 100%. If gaps exist, fix beads are created and executed (step 13), then this audit runs again.

---

## Source Documents

```
docs/strategy/layers/layer-3-operational-hardening.md    — Spec (SOT #1) — Walk EVERY requirement
docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-tdd.md — TDD (SOT #2) — Verify EVERY test exists with correct assertions
docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-bootstrap.md — Bootstrap — Verify environment matches
```

### Codebase (THE IMPLEMENTATION — read deeply)

**New Rust Modules (verify ALL exist and match spec):**
```
crates/daemon/src/scheduler/cron.rs                      — Cron scheduler
crates/daemon/src/sources/registry.rs                    — Source registry
crates/daemon/src/jobs/claim.rs                          — Concurrent job claiming
crates/daemon/src/pty/hardening.rs                       — PTY hardening
crates/daemon/src/agents/arbiter.rs                      — Resource arbiter
crates/daemon/src/db/checkpoints.rs                      — Checkpoint store
crates/daemon/src/tracing/structured.rs                  — Structured tracing
crates/daemon/src/tracing/forensic.rs                    — Forensic capture
crates/daemon/src/validation/policy.rs                   — Validation policy engine
crates/daemon/src/validation/executor.rs                 — Validation executor
crates/daemon/src/validation/manifest_writer.rs          — Run manifest writer
```

**Extended Existing Modules:**
```
crates/daemon/src/main.rs                                — Module declarations, bootstrap, cron startup
crates/daemon/src/jobs/runner.rs                         — Validation sub-job spawning, verdict handling
crates/daemon/src/sessions/reaper.rs                     — Extended reaper (pty/reaper.rs)
crates/daemon/src/agents/artifacts.rs                    — Extended manifest types
crates/daemon/src/agents/contracts.rs                    — Extended validation integration
crates/daemon/src/errors/manifest.rs                     — New error codes
crates/daemon/src/pty/session.rs                         — PTY hardening integration
crates/daemon/src/pty/health.rs                          — Extended health checks
crates/daemon/src/db/sqlite.rs                           — Migration for 5 new tables
crates/daemon/src/ipc/command_router.rs                  — New IPC commands
```

**New TS Modules:**
```
packages/intelligence/src/relations/resolve-process-relations.ts
packages/intelligence/src/relations/resolve-process-relations.test.ts
packages/manifests/render-error-manifest.ts
packages/manifests/error.md.ts
```

**New Skills:**
```
skills/auth-refresh/alibaba/
skills/auth-refresh/1688/
skills/source-health/fts/
skills/source-health/alibaba/
skills/validation/base/
```

**Test Files (verify ALL 18 exist):**
```
crates/daemon/tests/fault_process_cleanup.rs
crates/daemon/tests/fault_pty_edge_cases.rs
crates/daemon/tests/fault_host_events.rs
crates/daemon/tests/fault_partial_failure.rs
crates/daemon/tests/replay_pty_sessions.rs
crates/daemon/tests/replay_contract_sessions.rs
crates/daemon/tests/cron_scheduler.rs
crates/daemon/tests/job_claim.rs
crates/daemon/tests/skill_golden_journeys.rs
crates/daemon/tests/skill_shadow_compare.rs
crates/daemon/tests/skill_drift_monitor.rs
crates/daemon/tests/validation_policy.rs
crates/daemon/tests/validation_executor.rs
crates/daemon/tests/validation_dry_run.rs
crates/daemon/tests/validation_golden_verdicts.rs
crates/daemon/tests/validation_canary_health.rs
crates/daemon/tests/validation_retro_backlog.rs
packages/intelligence/src/relations/resolve-process-relations.test.ts
```

---

## Audit Methodology

### Phase 1: Requirement Walk
For EVERY L3-FR-x.y.z requirement in the spec:
1. Find the implementing code
2. Verify the implementation matches the spec
3. Find the test(s)
4. Verify the test assertions match the TDD

### Phase 2: Test Verification
For EVERY L3-T-* test ID in the TDD:
1. Find the test function in the codebase
2. Verify it has real assertions (not empty body, not `#[ignore]`)
3. Run `cargo test <test_name>` — does it pass?
4. Does the assertion match what the TDD specifies?

### Phase 3: Quality Gates
```bash
cargo check --all-targets
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
bun run build
bun run test
bun run lint
```

### Phase 4: Integration Verification
- [ ] Cron scheduler starts with daemon and fires on schedule
- [ ] Resource arbiter enforces quotas (browser_slots=2, agent_slots=4)
- [ ] Source health canaries are scheduled and fire
- [ ] Circuit breaker transitions work (closed → half_open → open)
- [ ] Validation policy engine makes correct decisions for all modes
- [ ] Validation executor spawns sub-jobs correctly
- [ ] Dry-run verdicts are logged but NOT enforced
- [ ] Fail-open: validator crash does NOT block pipeline
- [ ] Relation resolution classifies correctly (4 types)
- [ ] Skill promotion stages transition correctly
- [ ] Run manifests are generated with all required fields
- [ ] Telemetry metrics are populated
- [ ] Error manifests render correctly

---

## Follow-Up Pass

> After creating all fix beads:

ULTRATHINK.

1. Re-read the fix beads you created. Are they specific enough?
2. Does each fix bead reference the right spec section and TDD test IDs?
3. Could an agent implement each fix from the bead description alone?
4. Is the acceptance criteria concrete — not "fix this" but "add field X of type Y to struct Z"?
5. Are fix bead dependencies correct? (Some fixes may depend on other fixes.)
6. Run `bin/br-sync` to export.

Report: gaps found, fix beads created, estimated effort, confidence level.
