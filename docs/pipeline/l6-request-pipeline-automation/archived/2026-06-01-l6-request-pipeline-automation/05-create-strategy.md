# L6 Request Pipeline Automation — Step 05: Create Strategy

## First Action

Invoke the `/spec-to-strategy` skill.

---

## Your Mission

Produce the **L6 Strategy doc** (SOT #3) — the phase ordering + dependency DAG +
gate criteria for L6 implementation. This is the ROUTING TABLE that the bead-creation
step uses. It does NOT duplicate spec or TDD content.

**Quality bar:** L5 strategy + L4 strategy (lightweight, ordering-only). The strategy
is the THIRD SOT — it never overrides spec (WHAT) or TDD (HOW TO VERIFY), only
declares WHEN.

---

## Source Documents (Read In This Order)

### Primary Inputs (CREATE FROM THESE)
```
docs/strategy/layers/layer-6-request-pipeline-automation.md                                       # SOT #1 (WHAT)
docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-tdd.md      # SOT #2 (HOW TO VERIFY)
docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-bootstrap.md # Environment + quality-gate runbook
```

### Manifest Context
```
docs/pipeline/l6-request-pipeline-automation/manifest.yaml                                        # recorded_decisions + existing_files + new_modules
```

### Codebase (STUDY FOR DEPENDENCY ORDER)
```
crates/daemon/src/errors/manifest.rs:19-114                           # ErrorCode enum — Phase 1 foundation (compile-time)
crates/daemon/src/sources/registry.rs (1338 lines total)              # registry entries — Phase 1 adds bid-portal-procontract + overlay-bid-portal-procontract
crates/daemon/src/main.rs:1513-1517                                   # **Job-kind dispatch match arms** ("extract_document_structure", "extract_document_semantics") — Phase 2 adds "run_bid_workflow" here; handler functions at lines 1529 + 1836 are reference patterns. NOT in jobs/runner.rs.
crates/daemon/src/jobs/runner.rs (1656 lines)                         # Shared job-runner types — Phase 2 may add EnqueueJobRequest builder, but the match dispatch is in main.rs
crates/daemon/src/ipc/handlers/jobs.rs:45,95                          # trigger_ingestion + trigger_enrichment IPC handlers — Phase 2 adds trigger_bid_workflow following these patterns
crates/daemon/src/validation/policy.rs:17-25                          # validation policy — Phase 3 (after runner)
crates/daemon/src/validation/executor.rs                              # validator dispatch — Phase 3
skills/auth-signin/procontract/SKILL.md                               # existing — reused by Phase 4 skill
skills/procurement-ingestion/procontract-register-interest/SKILL.md   # reference pattern — Phase 4
skills/validation/overlay-supplier-enrichment/SKILL.md                # frontmatter shape — Phase 5
packages/contracts-schema/src/index.ts:463                            # existing types — Phase 2 TS types extend here OR in packages/bidding/
```

### Quality Bar (MATCH OR EXCEED)
```
docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-strategy.md  # L5 strategy
docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-strategy.md                                # L4 strategy
docs/pipeline/l5-expanded-commercial-intelligence/archived/2026-05-21-l5-expanded-commercial-intelligence/05-create-strategy.md  # L5 step 05 — 235 lines
```

---

## Cardinal Rule: ZERO SCOPE DRIFT

**Phases are EXECUTION ORDER, not importance tiers.**

Every spec feature appears in some phase. Every phase ships. Later phases get the
SAME depth as early phases. There is no "Phase 6 = nice to have." There is no "Phase
7" — L6 is the final layer (no L7 exists).

You may NOT:
- Defer features to a non-existent later phase
- Label any phase as "stretch" or "optional"
- Make later phases thinner than earlier ones
- Drop the validator overlay (decision_3 made it a deliverable)
- Skip the live demo (DoD #8 is M6 capstone)

---

## Key Context From Codebase Research

- **Foundation order is dictated by Rust compile-time dependencies:**
  - ErrorCode enum extension MUST land first (clippy-surfaced exhaustive matches across 5-15 sites)
  - Then job_runner dispatch arm (depends on ErrorCode)
  - Then IPC handler `trigger_bid_workflow` (depends on job_runner)
  - Then validator job + manifest extensions (parallel with portal skill)
- **Portal skill is the largest single deliverable** — `skills/bid-portal-procontract/` with 11-step playbook, schemas, canary inputs, golden assertions. This is THE feature; everything else supports it.
- **Validator overlay is parallel with portal skill** — depends on portal skill's `worker_behavior_version` declaration but development can proceed in parallel using the spec's defined contracts.
- **Desktop + web UI surfaces are late** — depend on the IPC handler being stable (Phase 2 done) and the validator overlay being authored (Phase 5 done).
- **Live demo (DoD #8) is M6 capstone** — runs at end of execution, BEFORE step 12 completion audit.
- **No cron entries** — L6 is operator-triggered.
- **No schema migrations** — `bid_workflow_states` already exists from L0.
- **Sync engine is unchanged** — bid_workflow_states already in `PUSH_OWNED_RESEED_TABLES` line 39.

---

## Step-by-Step Workflow

### Step 0: Read Source Documents (MANDATORY)
1. `docs/strategy/layers/layer-6-request-pipeline-automation.md` — **READ FULLY**
2. `docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-tdd.md` — **READ FULLY**
3. `docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-bootstrap.md` — **STUDY**
4. `docs/pipeline/l6-request-pipeline-automation/manifest.yaml` — **READ FULLY**
5. `docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-strategy.md` — **STUDY** quality bar
6. `docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-strategy.md` — **STUDY** alt quality bar

### Step 1: Invoke the Skill
Run `/spec-to-strategy`.

**Inputs:**
- Spec: `docs/strategy/layers/layer-6-request-pipeline-automation.md`
- TDD: `docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-tdd.md`
- Output: `docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-strategy.md`

**Key arguments / context to provide the skill:**
- Phase shape SUGGESTION (the skill validates against critical-path analysis):
  1. **Foundation** — ErrorCode +NEW variants (3-6 TDD-determined) + propagation; **`crates/daemon/src/bid_workflow/` module SKELETON** with mod.rs + types.rs + runner.rs (stub `todo!("Phase 3")`) + readiness_gate.rs (stub) + state_machine.rs (stub) — Phase 1 wires `mod bid_workflow;` into main.rs so Phase 2 dispatch + IPC handler can `use crate::bid_workflow::...` and Phase 3 fills the stubs; **`packages/bidding/bid-scope-types.ts` (TypeScript exports of BidScopePayload + SideEffectClass + BidWorkflow + BidDocument + SubmissionResult + DraftSaveResult + PortalStateVerification)** — these types are Phase 1 because Phase 4's `skills/bid-portal-procontract/result.schema.json` references SubmissionResult/DraftSaveResult shapes AND Phase 2's IPC handler accepts BidScopePayload at its public boundary; `packages/bidding/package.json` + `tsconfig.json` (minimal scaffolding); contracts-schema re-exports if any; PUSH_OWNED_RESEED_TABLES check (no-op, already present)
  2. **Daemon Runtime** — main.rs `"run_bid_workflow" => bid_workflow::runner::run_bid_workflow_job(...).await` dispatch arm (at line 1513+, next to existing `extract_document_structure` / `extract_document_semantics` arms) — wires to the Phase 1 stub which panics with `todo!()` until Phase 3 fills the body; `crates/daemon/src/ipc/handlers/jobs.rs` `trigger_bid_workflow` handler at the same level as `trigger_ingestion` (line 45) / `trigger_enrichment` (line 95) with readiness-gate pre-check that calls `bid_workflow::readiness_gate::evaluate(...)`; main.rs `--run-bid-workflow <bid-job-id>` CLI subcommand; `crates/daemon/src/jobs/runner.rs` may gain an EnqueueJobRequest builder helper but the dispatch + module wiring is in main.rs
  3. **Bid Workflow State Machine + Readiness Gate** — Fills out the Phase 1 stubs in `crates/daemon/src/bid_workflow/` (runner.rs / state_machine.rs / readiness_gate.rs / decision_basis_hash.rs). Rust state-machine enforcement (no SQL CHECK); all **14** transition implementations; staleness detection; decisionBasisHash computation with explicit L4+L5+L0 input vectors; 8 invariants enforcement; readiness-gate wraps existing L5 checks at `crates/daemon/src/main.rs:5256-5260,5510-5527,5537-5542,5781-5787` + quote-role + freshness + company-profile + upstream-trust; operator-override read path
  4. **Portal Skill** — `skills/bid-portal-procontract/` SKILL.md + schemas + canary inputs + golden assertions; CDP attach pattern + auth-signin delegation; 11-step playbook with per-step side-effect class declarations; calibration corpus (simple/medium/complex variants)
  5. **Validator Overlay + packages/bidding (workflow logic)** — `skills/validation/overlay-bid-portal-procontract/`; `packages/bidding/` workflow files (workflow-registry.ts + procontract-workflow.ts + company-profile-schema.ts — bid-scope-types.ts ALREADY landed in Phase 1); package.json + tsconfig + vitest.config; 3 TS test files (bid-workflow-states.test.ts + portal-side-effects.test.ts + bid-readiness-gate.test.ts)
  6. **Desktop + Web UI** — BidWorkflowStatusBadge, BidScopeSummary, DraftSavedReviewPanel, BidWorkflowStartButton in packages/ui; desktop tab integration; web read-only bid_state badge + RLS verification; desktop E2E + web E2E test files
  7. **Live Demo + Integration + Calibration Validation** — first live demo (sideEffectCap=REMOTE_DRAFT → draft_saved); operator-authorized second demo (FORMAL_SUBMISSION); calibration corpus runs against bid-portal-procontract skill; validator overlay calibration; final quality gate sweep

- Phase ordering rationale:
  - Phase 1 root of DAG (ErrorCode propagation blocks everything that compiles)
  - Phase 2 depends on Phase 1
  - Phase 3 depends on Phase 2 (state machine consumes daemon runtime)
  - Phase 4 parallel with Phase 5 (skill development independent of validator overlay) — BUT both depend on Phase 1 for `packages/bidding/bid-scope-types.ts`; portal skill's result.schema.json references SubmissionResult / DraftSaveResult defined there
  - **Phase 6 HARD-DEPENDS on Phase 2** (UI consumes IPC handler `trigger_bid_workflow` — Start Bid button is non-functional without it) + Phase 5 (UI displays validator verdicts from bid runs). Phase 6 CANNOT start until Phase 2 closes.
  - Phase 7 depends on ALL prior phases (live demo proves the integration)

### Step 2: Feature-Specific Guidance for the Skill

Ensure the strategy:

1. **Owns ONLY phase ordering** — does not duplicate spec text, TDD test descriptions, or bootstrap commands. Strategy = WHEN. Spec = WHAT. TDD = HOW TO VERIFY.
2. **Names every L6-FR-X.Y.Z's phase home** — a coverage table mapping FR → phase
3. **Names every L6-T-* test's phase home** — phase ownership of each test file (which phase makes the tests pass)
4. **Has gate criteria per phase** — what must be green before next phase starts
5. **Has critical-path identification** — which beads block downstream beads (Phase 1 ErrorCode propagation is the root)
6. **Has parallel-execution lanes** — Phases 4 + 5 are parallel; Phase 6 has internal parallel tracks (desktop + web)
7. **Has explicit "operator-action gates"** — e.g., before Phase 7 live demo, operator must:
   - Confirm at least one Procontract opportunity ref for testing
   - Confirm sideEffectCap=REMOTE_DRAFT for first run
   - Authorize FORMAL_SUBMISSION for second run (separate decision)
8. **Has milestone alignment** — L6 = M6 (final). Exit criterion = DoD §2.1 all green + L5→L6 readiness gate producible from L5 (already verified, but L6 doesn't break it).
9. **NO content duplication from spec or TDD** — if a phase description repeats spec text, replace with FR reference.
10. **NO scope-drift language** — same grep as steps 02/04.

### Step 3: Verify Output

After `/spec-to-strategy` completes, verify:
- [ ] Strategy file exists at `docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-strategy.md`
- [ ] Phase count is 6-8 (mirrors L4/L5 phase shape)
- [ ] Every L6-FR-X.Y.Z appears in the phase-coverage table
- [ ] Every L6-T-* test file is owned by some phase
- [ ] Each phase has explicit gate criteria
- [ ] Critical path identified (Phase 1 = root)
- [ ] Parallel lanes identified (Phase 4 ∥ Phase 5; Phase 6 desktop ∥ web)
- [ ] Operator-action gates explicit (Phase 7 live demo prerequisites)
- [ ] No spec text duplicated
- [ ] No TDD assertions duplicated
- [ ] No scope-drift language

### Step 4: Follow-Up Pass (Sub-Optimization)

> ULTRATHINK. After `/spec-to-strategy` finishes, re-read with fresh eyes.

1. **Is the critical path actually critical?** Walk the DAG. If you remove Phase 1's ErrorCode propagation bead, does it really block Phase 2's job_runner dispatch arm? (Yes — exhaustive match propagation requires the new variants to exist.) If you remove Phase 5's validator overlay, does it block Phase 7's live demo? (Yes — `validation_policy: always` will fall back to base validator, missing portal-specific assertions; live demo's fail-closed gate would block on insufficient validation depth.)

2. **Are parallel lanes truly independent?** Phase 4 (portal skill) and Phase 5 (validator overlay + packages/bidding) — do they share any files? Both touch the `worker_behavior_version` contract; verify the spec contract is stable BEFORE Phase 4/5 start (this is Phase 3's responsibility).

3. **Open `crates/daemon/src/errors/manifest.rs` lines 19-114.** Count existing variants. Confirm Phase 1's ErrorCode bead specifies the TDD-determined NEW variant set (typically 3-6 after auditing existing codes for redundancy — see `manifest.yaml#new_modules_rust` for the strawman + likely-redundant analysis). Strawman likely-new: `BID_SUBMISSION_FAILED`, `BID_DUPLICATE_DETECTED`, `BID_SIDE_EFFECT_CAP_EXCEEDED`, `BID_READINESS_GATE_FAILED`. Strawman likely-redundant (verify): `BID_PORTAL_CHANGED` (vs PAGE_SHAPE_DRIFT), `BID_PORTAL_AUTH_EXPIRED` (vs AUTH_STATE_STALE in daemon-orchestrated retry). Propagation site count target: 5-15 sites.

4. **Open `crates/daemon/src/main.rs` around lines 1513-1517 and find the existing job-kind dispatch** (`"extract_document_structure"` and `"extract_document_semantics"` arms). The dispatch is in **main.rs**, NOT in `jobs/runner.rs` (runner.rs holds shared types only). Confirm Phase 2's dispatch-arm bead references the exact insertion point in main.rs and the existing arms it mirrors. The handler function should follow `run_extract_document_structure_job` at line 1529 as a structural template.

5. **Open `crates/daemon/src/ipc/handlers/jobs.rs` and read `trigger_ingestion` at line 45 + `trigger_enrichment` at line 95.** These are the exact reference patterns. Confirm Phase 2's `trigger_bid_workflow` bead references both as the pattern and explicitly notes the readiness-gate pre-check happens BEFORE job enqueue (rejection returns structured blocker info).

6. **Open `skills/procurement-ingestion/procontract-register-interest/SKILL.md`.** Confirm Phase 4's bid-portal-procontract bead references this as the REFERENCE PATTERN (CDP discipline, side_effect_class, schema layout).

7. **Open `skills/validation/overlay-supplier-enrichment/SKILL.md` frontmatter.** Confirm Phase 5's validator overlay bead references this as the frontmatter shape precedent (20-line YAML with `worker_behavior_version`, `validator_behavior_version`, `validator_overlay_compatible_versions`, etc.).

8. **Open `packages/contracts-schema/src/index.ts`.** Decide (or confirm strategy's decision): do BidScopePayload + SideEffectClass + BidWorkflow types live in `packages/contracts-schema` or `packages/bidding`? Strategy must lock this. **Recommendation:** live in `packages/bidding/bid-scope-types.ts`, re-export from contracts-schema only if a non-bidding package needs them. CRITICAL: this file MUST land in Phase 1 (not Phase 5) because Phase 4's portal skill `result.schema.json` references SubmissionResult/DraftSaveResult, and Phase 2's IPC handler accepts BidScopePayload at the trigger_bid_workflow boundary. Strategy phase ordering MUST reflect this dependency.

9. **Phase 6 UI bead scope:** which React components in `packages/ui`? Strategy names them (BidWorkflowStatusBadge, BidScopeSummary, DraftSavedReviewPanel, BidWorkflowStartButton). Confirm the strategy specifies their location (`packages/ui/src/`) and the desktop tab they integrate into (`apps/desktop/src/...`).

10. **Phase 7 live demo prerequisites:** strategy must enumerate them as a pre-condition:
    - Persistent CDP profile at `/Users/alinaaitova/.chrome-automation` has saved `sales@sorelon.com` credential (Touch ID disabled)
    - Operator has identified at least one Procontract opportunity ref (won't generate buyer-visible legal commitment for first run)
    - `bin/ensure-cdp-9222` returns success
    - AgentMail healthy
    - All Phase 1-6 beads closed
    - Quality gate (full cargo + bun + e2e) green
    - L5 readiness gate condition: no active L5 blockers on the chosen opportunity's selected candidates

11. **Re-walk every spec invariant (§1.10, 8 of them).** Each invariant's enforcement mechanism lands in SOME phase. Verify which:
    - "No real-money bids without human approval" → Phase 3 state-machine enforcement
    - "No externally visible portal mutation beyond approved side-effect class" → Phase 3 + Phase 4 (skill respects cap)
    - "Bid workflow carries explicit scope" → Phase 1 + Phase 3
    - "Full audit trail for submissions" → Phase 2 (job_runner persists run manifest with screenshots)
    - "Stale decisions block bid" → Phase 3 (state-machine prerequisite check)
    - "L5 readiness gate" → Phase 3 + Phase 2 (IPC handler pre-check)
    - "Validation fail-closed for portal submission" → Phase 3 + Phase 5 (validator overlay) + Phase 2 (state-machine respects verdict)
    - "Quote-role quality gate" → Phase 3 (state-machine prerequisite check)

12. **Verify zero scope-drift language** in strategy.

13. **Count phases.** 6-8 is healthy. Less than 6 = phases too coarse; more than 8 = phases too fine.

### Step 5: Scope-Preservation Check (MANDATORY — BLOCKING)

- [ ] Every L6-FR-X.Y.Z appears in phase coverage matrix
- [ ] Every L6-T-* test file is owned by some phase
- [ ] Every DoD criterion is addressed by some phase (often the final phase)
- [ ] Every invariant (8) has its enforcement-mechanism phase named
- [ ] Phases are labeled execution order, NOT importance tiers
- [ ] No "stretch" / "optional" / "post-MVP" phase labels
- [ ] Phase 7 (live demo) has explicit operator-action gates
- [ ] Critical path identified
- [ ] Parallel lanes identified
- [ ] No spec/TDD content duplicated
- [ ] Strategy stays ordering-only (lightweight)

---

## Output

| File | Path |
|------|------|
| Strategy | `docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-strategy.md` |

Expected size: 400-700 lines (lighter than spec/TDD — strategy is ordering, not content).

---

## Key Reminders

- Strategy is the THIRD SOT — never overrides spec (WHAT) or TDD (HOW TO VERIFY), only WHEN
- 6-8 phases is the target shape
- Phase 1 (ErrorCode + types foundation) is the root of the DAG
- Phase 4 + Phase 5 are parallel
- Phase 7 (live demo) gates on Phase 1-6 + operator-action preconditions
- ZERO SCOPE DRIFT — every spec feature has a phase home
- The validator overlay (decision_3) is L6 deliverable — owns Phase 5
- The Proactis=Procontract reconciliation (decision_1) means all artifacts use procontract naming
- Match L5 strategy quality (lightweight ordering-only doc)
