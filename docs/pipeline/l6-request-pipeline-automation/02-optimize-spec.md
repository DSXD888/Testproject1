# L6 Request Pipeline Automation — Step 02: Optimize Spec (Fresh-Eyes Audit)

> **Different agent than step 01.** Independent fresh-eyes review of the validated spec.
> No skill to invoke. Manual reading + targeted edits.

## Your Mission

You are auditing the L6 spec **AFTER step 01 validated it** (added `L6-FR-X.Y.Z` IDs,
transcribed 5 operator decisions, reconciled Proactis=Procontract naming, bumped the
Implementation Status callout). Your job is to find what step 01 missed and to harden
the spec against scope drift before it enters the TDD pipeline.

**You are reading and improving an existing document — not writing code.**

---

## What To Read (Do This First)

### The Document to Audit (TARGET)
```
docs/strategy/layers/layer-6-request-pipeline-automation.md  # post step-01 — should have L6-FR-X.Y.Z IDs + 5 decisions in §1.0
```

### Verify Against (SOURCE OF TRUTH for what L6 must deliver)
```
docs/pipeline/l6-request-pipeline-automation/manifest.yaml          # spec_callouts + recorded_decisions
docs/strategy/layer-strategy.md                                     # §8 + §10 Interface Contracts
docs/strategy/cross-reference-index.md                              # rows 224-232 + 252 + 280
```

### Codebase (GROUND TRUTH — verify everything the spec claims about existing infra)
```
migrations/0001_sqlite_state_initial.sql:437-451                    # bid_workflow_states schema
crates/daemon/src/db/sync.rs:24-50                                  # bid_workflow_states at line 39 in push set
crates/daemon/src/errors/manifest.rs:19-114                         # ErrorCode enum (~46 variants)
crates/daemon/src/main.rs:5256-5260,5436,5498,5510-5527,5537-5542,5781-5787   # L5 readiness gate raw checks
crates/daemon/src/validation/policy.rs:17-25                        # ValidationPolicyMode
skills/auth-signin/procontract/SKILL.md                             # existing auth pattern
skills/procurement-ingestion/procontract-register-interest/SKILL.md # reference bid pattern
```

### Quality Bar
```
docs/strategy/layers/layer-5-expanded-commercial-intelligence.md    # L5 spec for structural reference
docs/pipeline/l5-expanded-commercial-intelligence/archived/2026-05-21-l5-expanded-commercial-intelligence/02-optimize-spec.md
                                                                    # L5's optimize-spec — 219 lines
```

---

## Think Independently — 10 Audit Dimensions

### 1. Requirement ID Coverage
Walk every § subsection. Does every load-bearing capability have an `L6-FR-X.Y.Z` ID?
Use a fresh count — don't trust step 01's reported total. Look for IDs that conflate
multiple distinct requirements (these need splitting). Look for prose without IDs
(these need adding).

### 2. Acceptance Criteria Specificity
For every `L6-FR-X.Y.Z`, check the AC. Bad ACs read like:
- "handles appropriately" — replace with concrete behavior
- "validates" — replace with what it validates AGAINST
- "logs" — replace with what it logs, where, at what level

Each AC must be testable. If a TDD author can't write a test from the AC alone, the AC is too vague.

### 3. State Machine Completeness (§1.3)
The **14-row** transition table (spec §1.3 has 14 distinct From→To pairs — re-verify yourself; this is a place where step 01 may have undercounted as 11) has explicit "Requirements" — verify each is concrete and
testable. Specifically:
- Every transition trigger is unambiguous (operator action vs daemon detection)
- Every prerequisite is a checkable predicate
- No transition is missing a sad-path (timeout, crash, operator cancel)
- The 7 states form a connected graph (no orphan state)
- The constitutional prerequisite check (lines 71-80) covers ALL 5 L5 blockers + quote-role + company-profile + upstream-trust

Cross-reference against `migrations/0001_sqlite_state_initial.sql:437-451` (no CHECK
constraint — Rust enforces). Confirm the spec acknowledges this.

### 4. BidScopePayload Field Completeness (§1.4)
Walk every field. Verify it's typed, required-vs-optional is explicit, validation rules are stated:
- `processId` — string, required, FK to procurement_processes.id
- `lotIds: string[]` — required, each FK to notice_lots
- `candidateIds: string[]` — required, each FK to product_candidates
- `selectedQuoteIds: string[]` — required, each FK to supplier_quotes
- `companyProfileVersion: string` — required, snapshot version
- `bidPortalType: string` — required, currently only `'procontract'` (per decision_1; not `'proactis'`)
- `portalOpportunityRef: string` — required, deterministic target for mutating workflow
- `reviewRequired: boolean` — required, default `true` per decision_5 live-demo policy
- `decisionBasisHash: string` — required, SHA-256 of pipeline assessment + selected quotes + company profile version + document hashes + classification freshness + L5 blocker state
- `sideEffectCap: SideEffectClass` — required, default `REMOTE_DRAFT` per decision_5

Check that ALL 6 SideEffectClass values (READ_ONLY / REMOTE_DRAFT / NOTIFICATION_OR_LOCK / LEGAL_DECLARATION / FORMAL_SUBMISSION / FUNDS_COMMITTED) are present.

### 5. Skill Playbook Step Completeness (§1.7)
The 11-step table (1-11 including 10a/10b alternates). For each step:
- Action is concrete (not "do the thing")
- Screenshot column is `Yes` for every step (audit-trail constitutional)
- Side-effect class assignment is correct (Step 1-5 = READ_ONLY; Step 6-8 + 10a = REMOTE_DRAFT; Step 9, 11 = READ_ONLY; Step 10b = FORMAL_SUBMISSION)

Cross-reference: step 2 (Authenticate) should explicitly reference delegation to
`skills/auth-signin/procontract/` per decision_2. If not, add the callout.

### 6. Invariant Enforceability (§1.10)
8 invariants. For each, the "Enforcement" column must name a concrete mechanism:
- A code path
- A schema constraint
- A pre-condition check
- A test gate

If the enforcement is "operator must" or "developer should," it's not an enforced
invariant — it's a hope. Either upgrade to a real enforcement or relabel as a guideline.

### 7. Validation Policy Fail-Closed Distinction (§1.3 last paragraph)
The spec correctly distinguishes the upstream-pipeline-trust gate (constitutional
fail-open per L3) from the bid-submission validation gate (constitutional fail-closed
for portal mutations). Verify this distinction is preserved post step-01 edits.
Specifically:
- Validator outage → `bid_submitted` BLOCKED (fail-closed) — but `draft_saved` permitted
- Operator override of the validation gate at final submission is allowed (and logged)
- This is consistent with `ValidationPolicyMode::Always` semantics in `crates/daemon/src/validation/policy.rs:17-25`

### 8. Test File Inventory (§2.2)
3 files named:
- `bid-workflow-states.test.ts` — state machine + L5 readiness gate + staleness + decisionBasisHash
- `portal-side-effects.test.ts` — side-effect class + crash recovery + duplicate-submission + cap enforcement
- `bid-readiness-gate.test.ts` — L5 blocker checks + quote-role + freshness + company profile docs

Per decision_3, the spec should also acknowledge `skills/validation/overlay-bid-portal-procontract/`
as an L6 deliverable. The TDD will own concrete test counts; the spec must own the
file inventory. Verify this is added (or add it).

### 9. DoD Coverage (§2.1, 11 criteria)
For each DoD criterion, name the `L6-FR-X.Y.Z` IDs that satisfy it. Build a quick
mental matrix:

| DoD # | L6-FR IDs that satisfy it |
|-------|---------------------------|
| 1 | L6-FR-1.2.1, L6-FR-1.7.1...11, ... |
| 2 | L6-FR-1.2.2, L6-FR-1.5.7, ... |
| ... | ... |

If a DoD criterion has ZERO mapped IDs, step 01 missed coverage — fix it.

### 10. Cross-Layer Spec Echoes
The spec has two cross-domain notes at the bottom (L0 CHECK constraint suggestion + L4
staleness vector inclusion). Both are NON-BLOCKING (forward-compatibility guidance);
verify step 01 didn't delete them.

---

## Scope Drift Detection (MANDATORY — BLOCKING)

### Red-Flag Language Scan
Search the ENTIRE spec for these phrases. **If ANY found (outside §2.8 which states
"Nothing — this is the final layer"), this is a BLOCKING failure.**

- "post-MVP", "post-L6", "Phase 7+"
- "stretch goal", "nice to have"
- "out of scope", "deferred"
- "future iteration", "later"
- "simplified for initial implementation"
- "can be added later"
- "if time permits"
- "as a stretch"
- "future enhancement"

### Coverage Verification
Re-walk every:
- DoD criterion (11) → must have ≥1 mapped L6-FR
- Invariant (8) → must have an explicit enforcement
- State machine transition (11) → must have all 4 columns (From/To/Trigger/Requirements) populated
- Skill playbook step (11) → must have all 3 columns (Step/Action/Screenshot/Side-Effect-Class) populated
- BidScopePayload field → must have type + required/optional + validation rule
- SideEffectClass value (6) → must appear in the union AND in at least one skill step's "Side-Effect Class" column

### Drift Checklist
- [ ] Every L6-FR ID has a non-vague AC
- [ ] No prose features lack an ID
- [ ] No silently dropped invariants, transitions, flow steps, playbook steps
- [ ] The 5 L5→L6 readiness blockers are present verbatim
- [ ] §2.8 still says "Nothing — final layer. No features deferred beyond this point."
- [ ] Operator decisions §1.0 has all 5 entries with required fields
- [ ] Proactis=Procontract banner appears at §1 and is reflected in §1.5/§1.6/§1.7
- [ ] Validator overlay (overlay-bid-portal-procontract) is acknowledged as L6 deliverable
- [ ] No "Proactis" references remain in callouts about new files (only in narrative explaining the alias)
- [ ] Constitutional prerequisite check (8 conditions) is intact and reachable from §1.3

---

## Codebase Cross-References (verify spec accurately describes existing infra)

Open each file and confirm the spec's claim matches reality:

1. **`migrations/0001_sqlite_state_initial.sql:437-451`** — `bid_workflow_states` schema. Spec §1.9 says "No new tables. No new columns." Confirm by reading the columns. Confirm `bid_scope_json` is nullable TEXT.

2. **`crates/daemon/src/db/sync.rs:24-50`** — `PUSH_OWNED_RESEED_TABLES`. Spec §1.9 says the table is synced to Supabase. Confirm at line 39.

3. **`crates/daemon/src/main.rs:5256-5260`** — L5 blocker code constants. Spec §1.3 lists 4 of them as readiness-gate blockers. Confirm names match.

4. **`crates/daemon/src/main.rs:5781-5787`** — `decision_grade` check. Spec §1.3 line 75 says "decision_grade = broker_required". Confirm the code path.

5. **`crates/daemon/src/main.rs:5537-5542`** — `classification_fresh_until_expired()`. Spec §1.3 line 77 references this freshness check. Confirm.

6. **`skills/auth-signin/procontract/SKILL.md`** — existing auth. Spec §1.7 step 2 should delegate to this (per decision_2). Confirm the callout was added by step 01.

7. **`skills/procurement-ingestion/procontract-register-interest/SKILL.md`** — reference pattern. Spec should note this as the "REFERENCE PATTERN for bid-portal-procontract" (per decision_2 implementation note). Confirm.

8. **`skills/validation/overlay-supplier-enrichment/SKILL.md`** — frontmatter precedent. Per decision_3, the new overlay-bid-portal-procontract follows this format. Confirm the spec acknowledges it.

If any of these turn up a mismatch, fix the spec. Mismatches between spec and ground
truth are the most expensive bugs to surface in step 03+.

---

## Follow-Up Pass (Run After Fixes)

> ULTRATHINK. Re-read YOUR fixes with fresh eyes. Did they introduce new issues?

1. **Did you split an ID and forget to update DoD/test-scenario references?**
   If you split `L6-FR-1.3.8` into `L6-FR-1.3.8a` and `L6-FR-1.3.8b`, every downstream
   document that referenced `L6-FR-1.3.8` must be updated.

2. **Did you tighten an AC and unintentionally narrow the feature?**
   E.g., changing "validates" to "validates against schema X" — verify schema X is
   actually the right validation target. If you guessed, mark it as needing TDD
   confirmation.

3. **Did you add a callout that contradicts another callout?**
   Read the spec end-to-end one more time. Specifically check §1.0 (recorded
   decisions) against §1.5/§1.6/§1.7/§1.10 — do they all agree on naming, delegation,
   and overlay inclusion?

4. **Open `docs/strategy/cross-reference-index.md` and `docs/strategy/layer-strategy.md`.**
   If your spec edits added new vocabulary or renamed anything (other than
   Proactis→Procontract), the cross-reference index may also need updating to point
   to the new spec subsections. Update if needed.

5. **Re-run the scope-drift grep.** Confirm zero hits in the updated spec.

6. **Count L6-FR IDs.** Aim 80-120. Report the final count in the spec's top-of-file
   callout or in this handoff's completion note.

7. **Re-walk DoD #8 specifically:** "System can prepare and submit one request on
   Proactis end-to-end." Per decision_5, the first live demo runs with
   `sideEffectCap = REMOTE_DRAFT` (draft only). The spec should NOT weaken DoD #8 to
   "draft only" — it remains "prepare and submit." The first run reaches `draft_saved`;
   a separate operator-authorized run reaches `bid_submitted` (this is the FULL DoD #8
   completion). Confirm the spec preserves this nuance.

---

## Output

- **Same spec file**, audited, IDs tightened, ACs sharpened, cross-references verified
- **No new files** produced
- **No skill invocation**
- Bead activity: NONE

---

## Key Reminders

- Fresh eyes — assume step 01 missed things; you are not rubber-stamping
- Cross-reference everything against the codebase — spec claims about existing infra MUST be ground-truth
- ZERO SCOPE DRIFT — if you delete a feature, you've failed
- Match L5's `02-optimize-spec.md` structural rigor (219 lines, 10 dimensions covered)
