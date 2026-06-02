# L6 Request Pipeline Automation — Step 04: Optimize TDD (Fresh-Eyes Audit)

> **Different agent than step 03.** Fresh-eyes review of the L6 TDD + Bootstrap.
> No skill to invoke. Manual reading + targeted edits.

## Your Mission

Audit the L6 TDD + Bootstrap produced by step 03. Find what was missed. Tighten test
assertions. Verify every `L6-FR-X.Y.Z` has a non-vague test. Verify the bootstrap is
runnable end-to-end on a fresh checkout.

**You are reading and improving existing documents — not writing code.**

---

## What To Read (Do This First)

### The Documents to Audit (TARGET)
```
docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-tdd.md       # primary audit target
docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-bootstrap.md # secondary audit target
```

### Verify Against (SOURCE OF TRUTH)
```
docs/strategy/layers/layer-6-request-pipeline-automation.md           # SOT #1 — spec must match
docs/pipeline/l6-request-pipeline-automation/manifest.yaml            # recorded_decisions + existing_files + new_modules
```

### Codebase (GROUND TRUTH)
```
crates/daemon/src/errors/manifest.rs:19-114                           # ErrorCode enum — verify TDD's variant additions are correct
crates/daemon/src/validation/policy.rs:17-25                          # ValidationPolicyMode — verify TDD's fail-closed tests target the right layer
crates/daemon/src/validation/manifest_writer.rs:23-54                 # write_hybrid_run_manifest — verify TDD's manifest tests align
crates/daemon/src/main.rs:1513-1517                                   # **Job-kind dispatch match arms — THIS is where TDD's run_bid_workflow test must point; NOT jobs/runner.rs**
crates/daemon/src/jobs/runner.rs                                      # Shared types only (NO match dispatch); verify TDD doesn't send agents to the wrong file
crates/daemon/src/ipc/handlers/jobs.rs:45,95                          # trigger_ingestion + trigger_enrichment reference patterns
crates/daemon/src/ipc/handlers/jobs.rs                                # verify TDD's trigger_bid_workflow handler test pattern
crates/daemon/src/main.rs:5256-5260,5436,5498,5510-5527,5537-5542,5781-5787   # verify TDD's readiness gate tests reuse existing checks
crates/daemon/tests/support/db.rs                                     # verify TDD's tests use TestStateDb pattern, not bespoke setup
crates/daemon/tests/skill_canary_harness.rs                           # verify TDD extends e2e_can_NNN pattern (next ID after existing)
crates/daemon/tests/skill_golden_journeys.rs                          # verify TDD extends l3_t_skill_golden_journeys_NNN pattern
crates/daemon/tests/redaction_boundary.rs                             # verify TDD's PII redaction tests target the right boundary
skills/auth-signin/procontract/SKILL.md                               # verify TDD's CDP discipline tests match existing
skills/procurement-ingestion/procontract-register-interest/SKILL.md   # verify TDD's reference-pattern claim is accurate
skills/validation/overlay-supplier-enrichment/SKILL.md                # verify TDD's overlay frontmatter expectations match
```

### Quality Bar (MATCH OR EXCEED)
```
docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-tdd.md       # 873 lines, 98 IDs
docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-tdd.md                                     # 623 lines, 158 checks
docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-bootstrap.md                               # 978 lines — bootstrap reference
docs/pipeline/l5-expanded-commercial-intelligence/archived/2026-05-21-l5-expanded-commercial-intelligence/04-optimize-tdd.md  # 218 lines
```

---

## Think Independently — 11 Audit Dimensions

### 1. Coverage Matrix Completeness
Walk every `L6-FR-X.Y.Z` from the spec. Does each appear in the TDD coverage matrix
with at least one mapped `L6-T-*` test ID? Specifically check:
- All **14** state-machine transitions (spec §1.3 has 14 distinct From→To rows — NOT 11; verify by counting the transition table rows yourself)
- All 5 L5 readiness blockers (CLASSIFICATION_AMBIGUOUS / TARIFF_BRANCH_DIVERGENCE / SUPPLEMENTARY_UNIT_MISSING / decision_grade=broker_required / classification freshness)
- Quote-role quality gate (`domestic_wholesale_floor` / `optimistic_floor` evidence)
- Upstream pipeline trust gate (`pipeline_assessments.validation_state = validated`)
- Company profile document expiry check
- All 6 SideEffectClass values + monotonic progression + cap-exceeded guard
- All 11 DoD criteria
- All 8 invariants
- All 19 bid workflow flow steps
- All 11 skill playbook steps
- All 6 audit-trail artifact types

### 2. Test Specificity
Bad test descriptions read like:
- "tests bid workflow" — replace with what specifically
- "verifies validation works" — replace with which assertion fails when
- "checks the gate" — replace with which condition triggers what return value

Good test descriptions:
- "Given operator decision is approved AND classification_fresh_until is in the past, when readiness gate is evaluated, then BID_READINESS_GATE_FAILED is returned with blocker_code=CLASSIFICATION_FRESHNESS_EXPIRED."
- "Given bid_in_progress + side_effect_cap=REMOTE_DRAFT, when skill attempts FORMAL_SUBMISSION step, then state transitions to bid_failed and ErrorCode is BID_SIDE_EFFECT_CAP_EXCEEDED."

### 3. ErrorCode Test Density
6 new variants × at least 3 test angles each (enum-creation, as_str() round-trip,
clippy-exhaustive-match-propagation site count) = ~18 tests minimum. Verify the TDD
specifies this density.

### 4. State Machine Tests — Happy + Sad + Recovery
For each of the **14** transitions (spec §1.3 has 14 From→To rows — not 11):
- Happy path test (preconditions met, transition succeeds)
- Sad path test (at least one precondition fails, transition rejected)
- Recovery test (where applicable — e.g., `bid_failed → ready_to_bid`, `portal_state_unknown → bid_in_progress`)

Expected ~30-50 tests in `bid-workflow-states.test.ts` alone.

### 5. Validator Overlay Tests (per decision_3)
The TDD should specify tests for `skills/validation/overlay-bid-portal-procontract/`:
- Approves a clean bid run manifest
- Rejects a manifest with wrong opportunity ref
- Rejects a manifest with fewer than 11 step screenshots
- Rejects a manifest with non-monotonic side-effect class progression
- Rejects a manifest where final submission confirmation reference is malformed
- Rejects a manifest where agent narrative doesn't cite lot/candidate IDs
- Rejects a manifest with PII leakage in narrative
- The fail-closed gate at bid_submitted handles validator outage (separate from rejection)

### 6. Calibration Corpus Spec
Per L5 precedent: `tests/fixtures/calibration/bid-portal-procontract/{simple,medium,complex}/{canary.inputs.jsonl,golden.assertions.yaml,validator-rubric.yaml,metadata.json,manifest.md}`. Verify the TDD specifies this structure with minimum canary input counts per variant (e.g., ≥3 simple, ≥3 medium, ≥2 complex).

### 7. Live Demo Test (DoD #8 verification)
DoD #8 — "System can prepare and submit one request on Procontract end-to-end" — needs:
- Manual smoke test (T4) documented in TDD
- First run: sideEffectCap = REMOTE_DRAFT, reaches `draft_saved`, screenshot trail present, run manifest written, Supabase visible
- Operator-authorized second run: sideEffectCap allows FORMAL_SUBMISSION, reaches `bid_submitted`, portal confirmation ref captured
- Both runs verifiable via SQL query on `bid_workflow_states`

### 8. Sync Engine Propagation Test
`bid_workflow_states` is already in `PUSH_OWNED_RESEED_TABLES`. The TDD should
specify a test that mutates a `bid_workflow_states` row locally → triggers outbox →
sync engine pushes → Supabase row matches. This may already be covered by L4's
generic sync tests; if so, the TDD should reference those tests + add a focused
bid-workflow-specific smoke.

### 9. PII Redaction Tests
`BidScopePayload` contains `companyProfileVersion` but the embedded company profile
itself (used in the run manifest narrative) has:
- `companyName`, `registrationNumber`, `vatNumber`
- `address.{line1,line2,city,county,postcode,country}`
- `primaryContact.{name,email,phone,role}`
- `certifications[].number`
- `insurancePolicies[].policyNumber`, `coverAmountPence`
- `financialStatements.companiesHouseRef`

The TDD must specify tests that verify these are scrubbed from:
- Error manifests (`error_manifests.error_context`, `error_manifests.markdown_path`)
- Run manifest narrative (`agent_narrative` and `daemon_evidence_appendix`)
- Supabase projections (`bid_workflow_states.bid_scope_json` may carry hashes but NOT raw PII)
- Log output (structured logs)

### 10. Bootstrap Runnability
Walk the bootstrap as if you're a fresh agent on a new checkout. Can you execute every step? Specifically:
- Are all prerequisite checks accompanied by an exact verification command?
- Is the local quality gate sequence complete (no missing commands)?
- Is the Procontract opportunity-ref selection guidance concrete (what to look for, what to avoid)?
- Is the AgentMail registration walkthrough copy-pasteable?
- Does the bootstrap correctly reference `bin/br-claim` / `bin/br-complete` / `bin/br-sync` (NEVER raw `br update`)?
- Does the bootstrap acknowledge AGENTS.md Rule 2 + the authoritative `br` block (lines 279-330)?

### 11. CI Tier Distribution
T1 (unit, <30s) / T2 (integration, <2m) / T3 (live external, <5m, requires CDP) / T4 (manual smoke). Verify the distribution is sane:
- T1 majority (state machine, readiness gate, ErrorCode round-trip, redaction)
- T2 for daemon dispatch + IPC handler + sync propagation
- T3 for bid-portal-procontract canary (CDP browser smoke)
- T4 for the live Procontract end-to-end demo

---

## Scope Drift Detection (MANDATORY — BLOCKING)

### Red-Flag Language Scan
Grep both TDD and Bootstrap for:
- "post-MVP" / "post-L6" / "Phase 7+"
- "stretch goal" / "nice to have"
- "out of scope" / "deferred"
- "future iteration" / "later"
- "simplified for initial implementation"
- "can be added later"
- "skip for now"
- "TODO"
- "placeholder"

**Zero hits expected.** Any hit = BLOCKING failure.

### Coverage Verification
- [ ] Every L6-FR-X.Y.Z has ≥1 mapped L6-T-* test ID
- [ ] Every DoD criterion (11) has ≥1 satisfying test
- [ ] All **14** state transitions tested (spec §1.3 has 14 distinct From→To rows; happy + sad + recovery where applicable)
- [ ] All NEW ErrorCode variants (TDD-determined count, typically 3-6 after redundancy audit) tested — verify the TDD audited existing codes (`PAGE_SHAPE_DRIFT`, `AUTH_STATE_STALE`, `AGENT_SKILL_FAILED`) for coverage of L6 failure modes before adding new variants
- [ ] All 6 SideEffectClass values tested
- [ ] All 5 L5 readiness blockers tested independently
- [ ] Quote-role gate tested with both `domestic_wholesale_floor` and `optimistic_floor` cases
- [ ] Company profile document expiry tested
- [ ] Upstream pipeline trust gate tested
- [ ] Validator overlay has ≥8 tests covering both approval and rejection paths
- [ ] Calibration corpus structure spec'd with per-variant canary counts
- [ ] PII redaction tests cover all 13+ sensitive fields
- [ ] Live demo (DoD #8) is explicitly documented as T4 manual smoke
- [ ] Sync propagation test for bid_workflow_states present

### Drift Checklist
- [ ] No tests dropped under "manual smoke handles this"
- [ ] No DoD criteria marked as "post-implementation verification"
- [ ] No invariants reduced to guidelines
- [ ] No requirement IDs missing from coverage matrix
- [ ] Bootstrap quality gate sequence is complete (cargo fmt/check/clippy/test + bun build/lint/test + desktop e2e + web e2e)
- [ ] Bootstrap acknowledges AGENTS.md Rule 2 + uses bin/br-* wrappers
- [ ] Bootstrap references `bin/br-sync` (NOT raw `br sync`)
- [ ] Bootstrap does NOT require any paid commercial accounts (decision_5)
- [ ] Bootstrap does NOT require auth-signin/proactis (decision_2 — reuses procontract)

---

## Follow-Up Pass (Run After Fixes)

> ULTRATHINK. Re-read YOUR fixes with completely fresh eyes.

1. **If you split a test ID, did you update the coverage matrix?**
2. **If you added a test, did you keep the bootstrap's test command sequence still able to run it?** (E.g., new TS test file must be picked up by `bun run test` in the right package.)
3. **Open the L4 TDD's RLS section. Did you mirror its rigor for L6's web RLS boundary?** (Viewer can see bid_state badge; viewer CANNOT see bid_scope_json or run manifest paths.)
4. **Open the L5 TDD's calibration test section. Did you mirror its per-category corpus structure?**
5. **Walk every test scenario in spec §2.3 (11 scenarios). For each, name the L6-T-* IDs that cover it.** If any scenario has zero coverage, fix.
6. **Walk every risk in spec §2.4 (4 risks). For each, identify the test that proves the mitigation works.**
7. **Re-run the scope-drift grep. Zero hits.**
8. **Count L6-T-* IDs.** Target 80-120 (matching L5's 98). Report final count.

---

## Output

- Same TDD + Bootstrap files, audited and tightened
- No new files produced
- No skill invocation
- Bead activity: NONE

---

## Key Reminders

- Fresh eyes — assume step 03 missed things
- Cross-reference everything against codebase ground truth
- ZERO SCOPE DRIFT — if you delete a test, you must replace with stronger coverage
- Validator overlay is L6 deliverable (decision_3) — its tests are MANDATORY
- 3-6 NEW ErrorCode variants (TDD-determined, after auditing existing codes for redundancy) need propagation coverage
- Live demo (DoD #8) is T4 manual smoke; verified in step 12 completion audit
- Match L5 TDD's 873-line + 98-ID density (or exceed)
