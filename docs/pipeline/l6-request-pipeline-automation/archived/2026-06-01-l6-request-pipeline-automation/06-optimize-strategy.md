# L6 Request Pipeline Automation — Step 06: Optimize Strategy (Fresh-Eyes Audit)

> **Different agent than step 05.** Fresh-eyes review of the L6 Strategy.
> No skill to invoke. Manual reading + targeted edits.

## Your Mission

Audit the L6 Strategy produced by step 05. Verify phase ordering respects critical-path
dependencies. Verify every spec feature has a phase home. Verify the strategy stays
ordering-only (no spec/TDD content duplication).

**You are reading and improving an existing document — not writing code.**

---

## What To Read (Do This First)

### The Document to Audit (TARGET)
```
docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-strategy.md
```

### Verify Against (SOURCE OF TRUTH)
```
docs/strategy/layers/layer-6-request-pipeline-automation.md           # SOT #1 — every feature has a phase home
docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-tdd.md       # SOT #2 — every test file has a phase home
docs/pipeline/l6-request-pipeline-automation/manifest.yaml            # recorded_decisions + existing_files + new_modules
```

### Codebase (GROUND TRUTH — verify dependency claims)
```
crates/daemon/src/errors/manifest.rs:19-114                           # ErrorCode propagation chain
crates/daemon/src/main.rs:1513-1517                                   # **Actual job-kind dispatch match arms** — Phase 2 adds here (NOT in jobs/runner.rs)
crates/daemon/src/jobs/runner.rs (1656 lines)                         # Shared types; NO match dispatch here
crates/daemon/src/ipc/handlers/jobs.rs                                # IPC handler — depends on job dispatch
crates/daemon/src/validation/policy.rs:17-25                          # validation policy
crates/daemon/src/validation/executor.rs                              # validator dispatch
crates/daemon/src/validation/manifest_writer.rs:23-54                 # manifest writer
crates/daemon/src/agents/adapter.rs:104-108                           # adapter interaction profile
skills/auth-signin/procontract/SKILL.md                               # auth delegation surface
skills/procurement-ingestion/procontract-register-interest/SKILL.md   # reference bid pattern
packages/contracts-schema/src/index.ts                                # type definitions location
packages/ui/                                                           # existing components for L6 UI extensions
apps/desktop/                                                          # desktop app for Phase 6 integration
apps/web/                                                              # web read-only viewer for Phase 6
```

### Quality Bar
```
docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-strategy.md  # L5 strategy
docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-strategy.md                                # L4 strategy
docs/pipeline/l5-expanded-commercial-intelligence/archived/2026-05-21-l5-expanded-commercial-intelligence/06-optimize-strategy.md  # 216 lines
```

---

## Think Independently — 10 Audit Dimensions

### 1. Phase Count & Granularity
6-8 phases is the target. If the strategy has 4 phases, it's likely conflating multiple
independent work units. If 10+, it's likely over-splitting. Verify each phase has a
distinct deliverable + distinct gate criterion.

### 2. Critical-Path Correctness
Walk the dependency DAG:
- Phase 1 (ErrorCode + types) MUST be root — Rust exhaustive-match propagation blocks all downstream compile
- Phase 2 (daemon runtime: jobs/runner + ipc/handler) depends on Phase 1
- Phase 3 (bid workflow state machine) depends on Phase 2
- Phase 4 (portal skill) can be parallel with Phase 3 if portal skill is built against Rust contracts that don't yet exist — but typically depends on Phase 1's BidScopePayload type at minimum
- Phase 5 (validator overlay + packages/bidding) can be parallel with Phase 4 if `worker_behavior_version` contract is locked in Phase 1
- Phase 6 (desktop + web UI) depends on Phase 2 (IPC handler exists) + Phase 5 (validator overlay verdicts to display)
- Phase 7 (live demo + integration) depends on ALL prior phases

Verify the strategy correctly identifies these dependencies. If Phase 4 is listed as parallel with Phase 3 but actually consumes Phase 3's state-machine API, the parallelism is illusory.

### 3. Parallel Lane Validation
For each claimed parallel lane:
- Is there ANY shared file/module/type?
- If yes, has the shared interface been locked in an earlier phase?
- If no, the parallelism is real

Common false-parallel claims:
- "Phase 4 ∥ Phase 5" — both touch `worker_behavior_version` contract; verify Phase 1 or Phase 3 locks the contract first
- "Desktop ∥ Web" within Phase 6 — both touch `bid_workflow_states` projection; verify the projection schema is locked before Phase 6 starts

### 4. Gate Criteria Specificity
Each phase has gate criteria. Bad gates:
- "Tests pass" — which tests? all? subset?
- "Build is green" — which build? cargo? bun? both?
- "Implementation complete" — vacuous

Good gates:
- "cargo clippy --all-targets -- -D warnings passes; all TDD-specified NEW ErrorCode variants (typically 3-6 after redundancy audit) present in `crates/daemon/src/errors/manifest.rs` as_str() arm; clippy reports zero exhaustive-match errors across the daemon"
- "Test `bid-workflow-states.test.ts` all 30+ assertions green; test `crates/daemon/tests/skill_canary_harness.rs::e2e_can_010_bid_portal_procontract_locates_opportunity` green"

### 5. Coverage Verification (Spec → Strategy)
Every `L6-FR-X.Y.Z` must appear in some phase's coverage table. Walk the spec list and
search for each ID in the strategy. If any FR is orphaned, the strategy is incomplete.

### 6. Coverage Verification (TDD → Strategy)
Every L6 test file (3 named TS files + Rust extensions to canary harness + golden journeys + redaction boundary + sqlite_schema + new IPC test + new validator-overlay test) must be owned by some phase.

### 7. Invariant Enforcement Phase Assignment
Each of the 8 invariants (spec §1.10) has an enforcement mechanism. Walk each:
- "No real-money bids without human approval" → Phase 3 state machine
- "No externally visible portal mutation beyond approved side-effect class" → Phase 3 + Phase 4
- "Bid workflow carries explicit scope" → Phase 1 (types) + Phase 3 (state machine pre-conditions)
- "Full audit trail for submissions" → Phase 2 (run manifest persistence in job_runner) + Phase 4 (skill captures screenshots)
- "Stale decisions block bid" → Phase 3 (state machine prerequisite)
- "L5 readiness gate" → Phase 3 + Phase 2 (IPC handler pre-check before job enqueue)
- "Validation fail-closed for portal submission" → Phase 3 + Phase 5 + Phase 2
- "Quote-role quality gate" → Phase 3

Verify the strategy assigns each enforcement to a specific phase.

### 8. Operator-Action Gate Completeness
Phase 7 (live demo) has operator preconditions. Verify the strategy lists ALL of:
- Persistent CDP profile at `/Users/alinaaitova/.chrome-automation` with `sales@sorelon.com` saved credential (Touch ID requirement removed per operator)
- At least one Procontract opportunity ref selected for testing (won't generate buyer-visible legal commitment)
- `bin/ensure-cdp-9222` returns success
- AgentMail healthy
- All Phase 1-6 beads closed
- Full quality gate green (cargo + bun + desktop e2e + web e2e)
- No active L5 blockers on the chosen opportunity's selected candidates
- First run constrained to sideEffectCap=REMOTE_DRAFT (draft only)
- Operator EXPLICITLY authorizes a separate FORMAL_SUBMISSION run with a chosen opportunity

### 9. Scope-Drift Detection (BLOCKING)
Grep the strategy for:
- "post-MVP" / "post-L6" / "Phase 8+"
- "stretch goal" / "nice to have"
- "out of scope" / "deferred"
- "future iteration" / "later"
- "simplified for initial implementation"
- "if time permits"
- "could add"
- "optional"
- "skip for now"

Zero hits expected. **Any hit is BLOCKING.**

Specifically check:
- Phase 5 validator overlay is NOT labeled "optional" or "nice to have" (decision_3 made it L6 deliverable)
- Phase 6 web UI is NOT labeled "viewer-only nicety" (it's part of DoD #10's enforcement of L5 readiness gate visibility for operators monitoring on web)
- Phase 7 second live demo (FORMAL_SUBMISSION) is NOT labeled "post-L6" (DoD #8 requires "prepare and submit" — completion includes the submission, not just the draft)

### 10. Content-Duplication Check
Strategy must stay ordering-only. Walk each phase description. If you see:
- Spec text verbatim → replace with FR reference
- TDD test assertion verbatim → replace with test ID reference
- Bootstrap command verbatim → reference the bootstrap section instead

A 600-line strategy that duplicates spec/TDD content is a maintenance burden. A 500-line
strategy that defers to spec/TDD is the right shape.

---

## Scope Drift Detection (MANDATORY — BLOCKING)

### Red-Flag Language Scan
- [ ] No "post-MVP" / "post-L6" / "Phase 8+"
- [ ] No "stretch goal" / "nice to have"
- [ ] No "out of scope" / "deferred"
- [ ] No "future iteration" / "later"
- [ ] No "simplified for initial implementation"
- [ ] No "if time permits" / "could add" / "optional"
- [ ] No "skip for now"

### Coverage Verification
- [ ] Every L6-FR-X.Y.Z appears in some phase
- [ ] Every L6-T-* test file is owned by some phase
- [ ] Every DoD criterion (11) is addressed by some phase
- [ ] Every invariant (8) has its enforcement phase named
- [ ] Validator overlay (decision_3) owned by Phase 5
- [ ] Live demo (DoD #8) owned by Phase 7 with explicit operator-action gates

### Drift Checklist
- [ ] No phase has thinner coverage than its predecessors
- [ ] Phase labels are execution-order ("Foundation", "Portal Skill", "Live Demo"), NOT importance tiers ("Critical", "Important", "Nice")
- [ ] Validator overlay phase exists (Phase 5)
- [ ] Web RLS verification phase exists (Phase 6)
- [ ] Live demo second-run (FORMAL_SUBMISSION) is part of Phase 7 completion, NOT a "follow-up" or "post-archive" item

---

## Follow-Up Pass (Run After Fixes)

> ULTRATHINK. Re-read YOUR fixes with completely fresh eyes.

1. **Did you change phase ordering?** If you moved Phase 4 (portal skill) before Phase 3 (state machine), did you verify the skill can be built against a non-existent state-machine API? (No — the skill's playbook needs to know the state transitions to drive the workflow.)

2. **Did you split a phase?** If you split Phase 4 into 4a (skill scaffold) + 4b (skill playbook + canary), did you update the coverage matrix to reflect both?

3. **Did you add a gate criterion?** Verify the criterion is automatable (test command, build command) — gate criteria that require manual human judgment slow the pipeline.

4. **Open the L5 strategy.** Compare your phase shape to L5's 6-phase structure. Are they comparable in granularity?

5. **Walk every parallel lane claim one more time.** For each "Phase X ∥ Phase Y" claim, verify NO shared file is touched without a locked interface.

6. **Re-walk the operator-action gates for Phase 7.** Are they exhaustive? If you missed (e.g.) "operator confirms BCC list for any portal notification emails," add it.

7. **Re-run the scope-drift grep. Zero hits.**

8. **Count phases.** Within 6-8.

9. **Open `docs/strategy/cross-reference-index.md` and `docs/strategy/layer-strategy.md`.** Verify the strategy doesn't contradict any cross-layer interface contract (especially row 230/232 — the L5→L6 readiness gate definition).

---

## Output

- Same strategy file, audited and tightened
- No new files produced
- No skill invocation
- Bead activity: NONE

---

## Key Reminders

- Strategy is ordering-only — NOT spec, NOT TDD
- Critical path: ErrorCode → daemon runtime → state machine → portal skill ∥ validator overlay → UI → live demo
- Validator overlay (decision_3) is L6 deliverable — explicitly assigned to a phase
- Live demo's second run (FORMAL_SUBMISSION) is part of Phase 7, NOT post-archive
- ZERO SCOPE DRIFT — no phase may be labeled stretch/optional
- Match L5 strategy quality (lightweight, ordering-focused)
