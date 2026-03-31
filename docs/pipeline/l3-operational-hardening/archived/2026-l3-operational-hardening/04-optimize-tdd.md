# L3 Operational Hardening — Optimize TDD (Fresh-Eyes Audit)

## Your Mission

You are performing a **comprehensive fresh-eyes audit** of the L3 Operational Hardening TDD specification — the document that defines exactly what tests must exist before implementation of Layer 3 begins.

**You are reading, analyzing, and improving an existing document — not writing code.**

---

## What To Read (Do This First)

### The Document to Audit (TARGET)
```
docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-tdd.md — TDD (created in step 03)
docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-bootstrap.md — Bootstrap (created in step 03)
```
Read fully. These are the primary audit targets.

### Verify Against (SOURCE OF TRUTH)
```
docs/strategy/layers/layer-3-operational-hardening.md    — Spec (SOT #1) — The source of truth for WHAT to build
```

### Codebase (GROUND TRUTH)
```
crates/daemon/src/main.rs                                — Module structure, existing tests pattern
crates/daemon/src/jobs/runner.rs                         — JobRunner types (verify TDD tests reference correct types)
crates/daemon/src/agents/artifacts.rs                    — RunManifestFrontmatter fields (verify manifest test coverage)
crates/daemon/src/agents/contracts.rs                    — ValidationOutcome types (verify validation test coverage)
crates/daemon/src/errors/manifest.rs                     — ErrorCode variants (verify error code test coverage)
crates/daemon/src/pty/session.rs                         — SessionPhase, PtyGeometry (verify PTY test coverage)
crates/daemon/src/sessions/reaper.rs                     — CleanupVerdict, ReaperConfig (verify reaper test coverage)
crates/daemon/src/pty/health.rs                          — HealthIssue variants (verify health test coverage)
crates/daemon/src/db/sqlite.rs                           — StateDb API (verify migration/DB test patterns)
crates/daemon/tests/                                     — Existing test files (verify naming/pattern consistency)
packages/intelligence/src/qualification/goods-qualification.test.ts — Existing TS test pattern
```

### Quality Bar (MATCH OR EXCEED)
```
docs/pipeline/l1-daemon-core/artifacts/l1-daemon-core-tdd.md — L1 TDD (best existing TDD)
```

---

## Think Independently

### 1. Coverage Matrix Completeness
Run the coverage matrix manually:
- Extract ALL L3-FR-x.y.z requirement IDs from the spec
- Check each one has at least one L3-T-* test ID in the TDD
- Report: N requirements covered, M requirements missing, K tests orphaned
- **Zero tolerance:** every requirement MUST have a test. If there's a gap, it's a real gap — fix it.

### 2. Test Quality — Are Assertions Concrete?
For every test definition in the TDD:
- Does it have a SPECIFIC expected outcome (not "works correctly")?
- Does it reference the EXACT type names from the codebase (`JobRunner`, `SessionReaper`, `CronJobDefinition`, etc.)?
- Could you write a `#[test]` function from the description alone?
- Does it specify what table/field to assert on (not "verify database state")?

Spot-check at least 10 test definitions. Rewrite any that are vague.

### 3. CI Tiering — Is the Budget Realistic?
- Count tests per tier. T1+T2 combined must be <5 min.
- Are PTY/process tests correctly in T2 or T3? (They need real process spawning)
- Are validation sub-job tests in T3? (They spawn real agent sessions)
- Are cron tests with timezone edge cases in T2 or T3?
- Is the relation resolution TS test in the TS test pipeline, not the Rust pipeline?
- Total T3 budget should be <20 min. Count the heavy tests and estimate.

### 4. Test Infrastructure — Are New Harnesses Defined?
L3 needs new test fixtures beyond L1's. Verify:
- `CronSchedulerHarness` — feeds test cron expressions, mocks system clock, verifies enqueue
- `ResourceArbiterFixture` — pre-seeds quota configuration, tracks grant/deny decisions
- `SourceHealthFixture` — simulates canary responses (pass/fail/timeout), circuit breaker state
- `ValidationPolicyFixture` — configures policy modes, feeds risk factors, verifies decisions
- `ValidatorHarness` — simulates validator sub-job execution, feeds verdicts, verifies handling
- `CircuitBreakerFixture` — state machine test helper (closed → half_open → open transitions)
- `RelationScoringFixture` — feeds notice pairs with known scores, verifies classifications

Each harness should be defined with: scope (UT/IT/E2E), purpose, and setup requirements.

### 5. Validation System Test Density
The validation system is the most complex L3 subsystem. Verify adequate coverage:
- Policy engine: 4 modes × N trigger conditions × risk factors = how many test cases?
- Executor: kickoff prompt composition, sub-job spawning, verdict parsing, error handling
- Dry-run mode: 6 isolation guarantees, each needs a dedicated negative test
- Fail-open: validator crash, timeout, invalid verdict, partial verdict — all covered?
- Canary health: known-good/bad/insufficient-basis classification, health state transitions
- Retroactive backlog: accumulation, priority ordering, drain, stale-marking

### 6. Relation Resolution — Is the TS Test Comprehensive?
The single `resolve-process-relations.test.ts` file carries ALL relation resolution testing:
- 4 classifications (merged/related/possible_duplicate/separate)
- Hard blockers (contradictions prevent merge)
- Exact-match rules (same OCID, explicit references)
- Heuristic scoring (weight calculation, threshold boundaries)
- Cluster identity (canonical_process_id assignment)
- Late merge (decision-staleness marking)
- Edge cases: same buyer different references, framework vs call-off, multi-notice chains

Is one test file enough? If not, should it be split?

### 7. PTY Hardening — Are OS-Level Tests Realistic?
§5.2 defines `fault_pty_edge_cases.rs` covering fd leak scenarios, EIO handling, terminal geometry.
- Can fd leak detection actually be tested deterministically? What's the assertion?
- EIO after slave close: does the test harness support PTY master/slave lifecycle testing?
- Terminal geometry: does `PtyGeometry { cols: 120, rows: 40 }` get verified in a real PTY?
- Signal coalescing: can Tokio's Unix signal streams be tested for coalescing behavior?

### 8. Bootstrap Document — Is It Complete?
The bootstrap document should specify:
- [ ] Rust crate additions: `cron` (version), `sysinfo` (version)
- [ ] SQL migration file: `migrations/0005_l3_operational_hardening.sql` with all 5 new tables
- [ ] Modified table ALTER statements (workflow_jobs, agent_session_logs)
- [ ] Skill directory scaffolding for all 10 new skills
- [ ] Test fixture data: gold-set manifests, canary inputs, golden assertions
- [ ] Configuration defaults: cron expressions, resource quotas, circuit breaker thresholds
- [ ] New module declarations in `crates/daemon/src/main.rs`

---

## Drift Checklist

- [ ] Every spec requirement has at least one test ID in the TDD
- [ ] No test describes behavior not in the spec
- [ ] Test file names match §5.2's pre-defined list
- [ ] CI tiering budget adds up (T1+T2 < 5min, T3 < 20min)
- [ ] Coverage matrix has no orphan requirements or orphan tests
- [ ] Bootstrap doc lists all prerequisites (crates, migrations, skills, config)
- [ ] New test harnesses are defined for each major L3 subsystem
- [ ] TS tests (relation resolution) are in Vitest, not Rust
- [ ] Validator test coverage includes the FULL policy × trigger × verdict matrix
- [ ] Fail-open semantics have dedicated negative tests

---

## Follow-Up Pass (Run After Fixes)

> After making your fixes, send this as a follow-up message in the same session.

ULTRATHINK. Re-read the fixes you just made. Did they introduce new issues?

1. **If you added test IDs:** verify the numbering is sequential within each file. No gaps, no duplicates. Count total test IDs.

2. **If you modified the coverage matrix:** re-run the coverage check. Is it still 100%?

3. **If you added new test harnesses:** do they depend on any L3 code that doesn't exist yet? Bootstrap must set up harness prerequisites.

4. **Open `docs/strategy/layer-strategy.md` §11** — verify the cumulative test count. L3 should add to L2's cumulative total. Does the chain still hold?

5. **Estimate total test runtime** per tier. Are the budgets realistic given the test types?

Report: total test IDs, coverage percentage, any remaining gaps, confidence level.
