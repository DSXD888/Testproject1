# L6 Request Pipeline Automation — Step 12: Completion Audit

## First Action

Invoke the `/bead-completion-audit` skill.

---

## Your Mission

Deep-read the spec, TDD, and codebase for L6. Find EVERY gap between what the spec
requires and what's actually implemented. Create `audit-fix` beads for gaps. Do NOT
execute them — just create and scope.

**CRITICAL: The spec is the source of truth.** Every L6-FR-X.Y.Z in the spec must be
implemented in the codebase. Every DoD criterion must be met. The live demo (DoD #8)
must pass. If something is missing, it's a gap — not a "nice to have" that was
intentionally skipped. Create a fix bead for it.

This is a full-codebase audit, NOT a sampling exercise. Walk EVERY requirement.

---

## Source Documents

### Three Sources of Truth (READ FULLY — verify against codebase)
```
docs/strategy/layers/layer-6-request-pipeline-automation.md                                       # SOT #1 — walk EVERY L6-FR-X.Y.Z
docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-tdd.md      # SOT #2 — verify EVERY L6-T-* test exists with correct assertions
docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-bootstrap.md # Environment + quality gate sequence
docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-strategy.md  # SOT #3 — verify phase deliverables landed
```

### Manifest
```
docs/pipeline/l6-request-pipeline-automation/manifest.yaml            # recorded_decisions + existing_files + new_modules
```

### Bead Inventory
```
docs/pipeline/l6-request-pipeline-automation/artifacts/l6-bead-alias-map.json
docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-bulk-import.md
br list --label l6 --json
```

### Codebase (THE IMPLEMENTATION — read deeply)
```
# NEW deliverables (must exist as designed)
packages/bidding/workflow-registry.ts
packages/bidding/procontract-workflow.ts
packages/bidding/company-profile-schema.ts
packages/bidding/bid-scope-types.ts                       # or wherever types landed
packages/bidding/tests/bid-workflow-states.test.ts
packages/bidding/tests/portal-side-effects.test.ts
packages/bidding/tests/bid-readiness-gate.test.ts
packages/bidding/package.json
packages/bidding/tsconfig.json
packages/bidding/vitest.config.ts

skills/bid-portal-procontract/SKILL.md
skills/bid-portal-procontract/result.schema.json
skills/bid-portal-procontract/checkpoint.schema.json
skills/bid-portal-procontract/canary.inputs.jsonl
skills/bid-portal-procontract/golden.assertions.yaml

skills/validation/overlay-bid-portal-procontract/SKILL.md

tests/fixtures/calibration/bid-portal-procontract/{simple,medium,complex}/{canary.inputs.jsonl,golden.assertions.yaml,metadata.json,manifest.md}

# EXTENDED daemon (verify changes landed)
crates/daemon/src/errors/manifest.rs:19-114               # NEW ErrorCode variants present? Count per TDD (typically 3-6)
crates/daemon/src/main.rs:1513+                           # **run_bid_workflow dispatch arm present here?** (next to existing extract_document_* arms — NOT in jobs/runner.rs)
crates/daemon/src/jobs/runner.rs                          # Verify Phase 2 did NOT misroute the dispatch into runner.rs (which holds only shared types)
crates/daemon/src/ipc/handlers/jobs.rs                    # trigger_bid_workflow handler present?
crates/daemon/src/main.rs                                 # --run-bid-workflow CLI subcommand present?
crates/daemon/src/sources/registry.rs                     # RegisteredSource entries for bid-portal + overlay present?
crates/daemon/src/bid_workflow/ (or inline equivalent)    # state-machine logic present?

# EXTENDED tests (verify additions landed)
crates/daemon/tests/skill_canary_harness.rs               # e2e_can_010+ present?
crates/daemon/tests/skill_golden_journeys.rs              # l3_t_skill_golden_journeys_009+ present?
crates/daemon/tests/redaction_boundary.rs                 # bid-related PII redaction tests present?
crates/daemon/tests/sqlite_schema.rs                      # state-machine value enforcement tests present?

# UI deliverables
packages/ui/src/BidWorkflowStatusBadge.tsx
packages/ui/src/BidScopeSummary.tsx
packages/ui/src/DraftSavedReviewPanel.tsx
packages/ui/src/BidWorkflowStartButton.tsx
apps/desktop/src/                                          # BidWorkflow tab integration
apps/desktop/e2e/bid-workflow.spec.ts
apps/web/src/                                              # read-only bid_state badge
apps/web/e2e/bid-workflow-viewer.spec.ts
```

### Quality Bar
```
docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-completion-audit-2026-05-20.md     # 270 lines, COMPLETE verdict, walked every L5-FR
docs/pipeline/l4-shared-visibility/artifacts/l4-completion-audit-2026-05-08.md                    # 172 lines, COMPLETE verdict, walked every L4-FR
docs/pipeline/l5-expanded-commercial-intelligence/archived/2026-05-21-l5-expanded-commercial-intelligence/12-completion-audit.md  # 262 lines
```

---

## Audit Methodology

Walk the spec **requirement by requirement**. For each `L6-FR-X.Y.Z`:

1. **Find the corresponding implementation** in the codebase (file + function/component)
2. **Verify it matches the spec's acceptance criteria** (read the actual code)
3. **Find the corresponding L6-T-* test(s)** (per `l6-bead-alias-map.json`)
4. **Verify tests exist with correct assertions** (open the test file, read the assertions)
5. **Run the tests** to confirm they pass
6. **If gap found** → create a fix bead with clear scope

**Do NOT accept "this was intentionally deferred" as an excuse.** If it's in the spec
and not in the codebase, it's a gap. Create a fix bead.

### Audit Dimensions (Walk Each)

#### Dimension 1: Spec FR Coverage (Implementation)
For every `L6-FR-X.Y.Z`, name the file + function/component that implements it. Build
a coverage table. If any FR has no implementation, gap.

#### Dimension 2: Spec FR Coverage (Tests)
For every `L6-FR-X.Y.Z`, name the test ID(s) that verify it. Build a test-coverage
table. If any FR has no test, gap.

#### Dimension 3: DoD Verification (11 criteria from spec §2.1)
For each:
- DoD #1: Bid-portal skill navigates Procontract via browser automation → `skills/bid-portal-procontract/SKILL.md` playbook step 1-2; verified by `skill_canary_harness.rs::e2e_can_010_bid_portal_procontract_canary`
- DoD #2: Company profile injected → `crates/daemon/src/main.rs` (line 1513+ dispatch arm + the `run_bid_workflow_job(...)` handler function that reads `shared_company_profile` and injects into kickoff prompt); verified by E2E test
- DoD #3: Form filling + document upload with side-effect class tracking → playbook step 6-8; verified by `portal-side-effects.test.ts`
- DoD #4: Submission with screenshot at each stage → all 11 playbook steps emit screenshots; verified by canary test asserting screenshot file count == 11
- DoD #5: Full terminal output tracing → daemon captures PTY bytes per existing infrastructure; verified by run manifest assertion
- DoD #6: Run manifest generated → `manifest_writer.rs::write_hybrid_run_manifest` invoked; verified by integration test
- DoD #7: Bid failure produces error manifest with portal-specific context → `crates/daemon/src/errors/manifest.rs` writes error_manifests row; verified by integration test
- DoD #8: System can prepare and submit one request on Procontract end-to-end → **LIVE DEMO** (manual smoke); verified by operator running first demo (sideEffectCap=REMOTE_DRAFT → draft_saved) AND operator-authorized second demo (FORMAL_SUBMISSION → bid_submitted)
- DoD #9: draft_saved path works when reviewRequired=true → state machine test + E2E desktop test
- DoD #10: L5 readiness gate blocks ready_to_bid when active L5 blockers present → `bid-readiness-gate.test.ts` with 5 blocker scenarios + quote-role + freshness + company-profile + upstream-trust
- DoD #11: Validator rejection or outage blocks bid_submitted (fail-closed) → validator overlay tests + integration test simulating outage

For each DoD criterion, the verdict must be PASS (implementation present + tests
present + tests pass) or GAP (fix bead created).

#### Dimension 4: Invariant Enforcement (8 invariants from spec §1.10)
For each invariant, name the enforcement mechanism in code:
- "No real-money bids without human approval" → readiness gate checks `operator_decision_heads.disposition == 'approved'`
- "No externally visible portal mutation beyond approved side-effect class" → bid-portal skill respects `BidScopePayload.sideEffectCap`; raises `BID_SIDE_EFFECT_CAP_EXCEEDED` on attempt to escalate
- "Bid workflow carries explicit scope" → BidScopePayload required at job enqueue time
- "Full audit trail for submissions" → manifest_writer emits hybrid manifest with screenshot artifact refs
- "Stale decisions block bid" → readiness gate computes current decisionBasisHash and compares to operator_decision_heads.reviewed_basis_hash
- "L5 readiness gate" → readiness gate wraps L5 checks at crates/daemon/src/main.rs:5256-5260,5510-5527,5537-5542,5781-5787
- "Validation fail-closed for portal submission" → bid-workflow state machine refuses bid_submitted transition without validator verdict approved OR operator override
- "Quote-role quality gate" → readiness gate checks supplier_quotes.price_signal_role for selected quotes

If any invariant lacks an enforcement mechanism in code, gap.

#### Dimension 5: State Machine Completeness (**14** transitions from spec §1.3)
Walk each of the **14 transition rows** in the spec §1.3 table (NOT 11 — re-count yourself; the table has 14 distinct From→To pairs across the 7 lifecycle states). Verify:
- Code path exists for the transition
- Preconditions are checked
- Test exists for happy + sad path
- Recovery transitions work (`bid_failed → ready_to_bid`, `portal_state_unknown → bid_in_progress`)
- Self-loops do not exist (e.g., no `bid_in_progress → bid_in_progress`)

#### Dimension 6: Bid Portal Skill Playbook (11 steps from spec §1.7)
For each step, verify:
- Skill SKILL.md describes the step
- Side-effect class declaration matches spec
- Test (canary or unit) covers the step

#### Dimension 7: Validator Overlay (decision_3)
Verify `skills/validation/overlay-bid-portal-procontract/SKILL.md` exists with:
- Frontmatter matching overlay-supplier-enrichment shape
- Assertions: opportunity ref match, screenshot count, side-effect monotonicity, submission confirmation, narrative cites lots/candidates, no PII

#### Dimension 8: PII Redaction (BidScopePayload + CompanyProfile fields)
Verify `crates/daemon/tests/redaction_boundary.rs` extensions cover all 13+ sensitive fields and pass.

#### Dimension 9: Sync Engine No-Op (bid_workflow_states already in push set)
Verify (test or manual): mutate bid_workflow_states locally → outbox → sync → Supabase row matches. Should be no-op for L6 (L4's sync engine already handles).

#### Dimension 10: ErrorCode Propagation (TDD-determined count — typically 3-6 new variants)
Open the TDD (`l6-request-pipeline-automation-tdd.md`) and find the final ErrorCode-extension test plan. Verify `crates/daemon/src/errors/manifest.rs:19-114` has every NEW variant the TDD specified — neither more, nor fewer.

Likely-new variants (verify each landed): `BID_SUBMISSION_FAILED`, `BID_DUPLICATE_DETECTED`, `BID_SIDE_EFFECT_CAP_EXCEEDED`, `BID_READINESS_GATE_FAILED`.

Likely-redundant variants (TDD may have dropped these — verify):
- `BID_PORTAL_CHANGED` → if dropped, verify L6 reuses `PAGE_SHAPE_DRIFT` with portal-specific `error_context`
- `BID_PORTAL_AUTH_EXPIRED` → if dropped, verify L6 reuses `AUTH_STATE_STALE` (daemon-orchestrated retry pattern: bid skill returns AUTH_STATE_STALE → daemon runs auth-signin/procontract → daemon re-runs bid skill)

Each NEW variant has an `as_str()` arm. Round-trip test passes. `cargo clippy --all-targets -- -D warnings` passes (proves exhaustive-match propagation completed across all 5-15 sites).

#### Dimension 11: Live Demo (DoD #8 — M6 capstone, decision_5)
This is the FINAL gate. Verify:
- First live demo ran with `sideEffectCap=REMOTE_DRAFT` against an operator-chosen Procontract opportunity
- Reached `draft_saved` state in `bid_workflow_states`
- 11 screenshots present in artifact store
- Run manifest written and includes BidScopePayload + portal interaction trace
- Validator verdict recorded
- Supabase row visible to web viewer
- Operator-authorized second demo ran with `sideEffectCap` allowing `FORMAL_SUBMISSION`
- Reached `bid_submitted` state
- Portal confirmation reference captured in run manifest
- All other DoD criteria still green

If live demo did NOT run, gap (the M6 capstone is unmet). Create a fix bead for live demo execution.

#### Dimension 12: Quality Gate (Full Sweep)
Run the complete quality gate sequence from bootstrap. Every command must exit zero:
```bash
cargo fmt --check
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test -p daemon --bin daemon
for t in crates/daemon/tests/*.rs; do cargo test --test "$(basename "$t" .rs)" || exit 1; done
bun run build
bun run lint
bun run test
(cd apps/desktop && bun run test:e2e)
(cd apps/web && bun run test:e2e)
```

If any fails, gap.

---

## Audit Verdict Format

After walking all 12 dimensions, write a verdict document:

**Path:** `docs/pipeline/l6-request-pipeline-automation/artifacts/l6-completion-audit-YYYY-MM-DD.md` (use today's date)

**Structure:**
```markdown
# L6 Request Pipeline Automation — Completion Audit

**Date:** YYYY-MM-DD
**Auditor:** <agent-name>
**Verdict:** COMPLETE | INCOMPLETE | BLOCKED

## Executive Summary
[1 paragraph: total L6-FR walked, gaps found, fix beads created]

## Coverage Matrix
| L6-FR-X.Y.Z | Implementation File | Test ID | Status |
|-------------|---------------------|---------|--------|
| L6-FR-1.1.1 | skills/bid-portal-procontract/SKILL.md | L6-T-bid_portal_procontract_skill-001 | PASS |
| ... (every FR) | ... | ... | ... |

## DoD Walkthrough
| # | DoD Criterion | Implementation | Tests | Verdict |
|---|---------------|----------------|-------|---------|
| 1 | Bid-portal skill navigates Procontract | ... | ... | PASS / GAP |
| ... |

## Invariant Enforcement Walkthrough
| # | Invariant | Enforcement Mechanism | Test | Verdict |
| 1 | No real-money bids without human approval | readiness gate | L6-T-... | PASS |
| ... |

## Live Demo Verification
[Detailed walkthrough of first + second live demo: screenshots, run manifest paths, bid_workflow_states row, Supabase visibility]

## Quality Gate Sweep
[Full output of the bootstrap quality gate sequence]

## Gaps Found + Fix Beads Created
| Gap | Fix Bead ID | Priority | Scope |
| ... |

## Verdict
COMPLETE | INCOMPLETE (N fix beads) | BLOCKED (description)
```

---

## Fix Bead Creation Guidelines

For each gap:
1. Create a bead via `br create` with `--labels "l6-audit-fix,l6"` (NOTE: `br create` takes ONE `--labels` flag with comma-separated values; verified `--label` singular is rejected by clap. `br list`/`br ready` use `--label` repeatable — different from create.)
2. Title: `l6-audit-fix-<short-slug>` (e.g., `l6-audit-fix-validator-overlay-missing-opportunity-ref-assertion`)
3. Body:
   - **Gap:** what's missing
   - **Spec Reference:** L6-FR-X.Y.Z + spec text
   - **Files to Create/Modify:** exact paths
   - **Acceptance Criteria:** testable bullets
   - **Dependencies:** which previously-completed L6 beads this builds on
   - **Verification:** how to confirm the fix works
4. Parent: the L6 implementation epic (NOT the original task bead — original is closed)

After all fix beads created:
- `bin/br-sync`
- `git add .beads/issues.jsonl`
- `git add docs/pipeline/l6-request-pipeline-automation/artifacts/l6-completion-audit-YYYY-MM-DD.md`
- `git commit -m "l6-pipeline: completion audit N gaps found; fix beads created"`
- `git push`

---

## Follow-Up Pass (After Creating Fix Beads)

> ULTRATHINK. Re-read the fix beads you created with fresh eyes.

1. **Are they specific enough?** A fresh agent must be able to implement each fix from the bead alone.
2. **Does each one reference the right spec section and TDD test IDs?**
3. **Could an agent implement each fix from the bead description alone?** Same self-containment standard as step 07 beads.
4. **Is the acceptance criteria concrete?** Not "fix this" but "add field X of type Y to struct Z at file path P line N".
5. **Did you cover EVERY spec requirement, or did you stop early?** Re-walk the spec.
6. **Run a final count: spec requirements vs (closed implementation beads + fix beads) = 100%?**

---

## Output

| File | Path |
|------|------|
| Audit verdict | `docs/pipeline/l6-request-pipeline-automation/artifacts/l6-completion-audit-YYYY-MM-DD.md` |
| Fix beads (if any) | `.beads/issues.jsonl` (via `br create`) |

---

## ⛔ Conditional Stop

- **If verdict = COMPLETE** → proceed to step 15 (archive). NO fix beads needed.
- **If verdict = INCOMPLETE (N fix beads)** → proceed to step 13 (fix beads kickoff). DO NOT skip to step 15.
- **If verdict = BLOCKED** → escalate to human. Stop.

---

## Key Reminders

- This is a FULL audit, NOT a sample — walk EVERY L6-FR-X.Y.Z
- The live demo (DoD #8) is the M6 capstone — its verification is the most critical dimension
- ZERO TOLERANCE for "intentionally deferred" — if spec'd and not built, gap
- Fix beads must be as self-contained as task beads (step 07 standard)
- Use `bin/br-sync` (NEVER raw `br sync`)
- Match L4/L5 completion audit rigor (walk every FR, no shortcuts)
- The verdict is binary (COMPLETE / INCOMPLETE / BLOCKED) — no middle ground
