# L6 Request Pipeline Automation — Step 01: Validate Existing Spec

> Pattern B — the L6 layer doc IS already an execution-grade spec. This step VALIDATES
> structure (requirement IDs, ACs, completeness, scope-drift cleanliness) and
> TRANSCRIBES **five** pre-resolved operator decisions (recorded 2026-05-21 in
> `manifest.yaml#recorded_decisions`). It does NOT rewrite content.
>
> **There is NO skill to invoke.** This is manual editing in-place. Bump the spec's
> Implementation Status callout when done.

## §0 — PRE-FLIGHT (CRITICAL — DO NOT SKIP)

Before any spec editing, ALL of these must be GREEN. If any fails, STOP and report:

| Claim | Verify | Action if fails |
|-------|--------|-----------------|
| L5 is COMPLETE | `grep "verdict.*COMPLETE" docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-completion-audit-*.md` | If absent → L5 is not built → STOP; L6 cannot start |
| L5 status `ready_for_l6: true` | `grep "ready_for_l6: true" docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-status.yaml` | If false/absent → STOP; revisit L5 |
| L5→L6 readiness gate producible | `grep "l5_to_l6_gate_producible: true" docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-expanded-commercial-intelligence-status.yaml` | If false → STOP; L5 didn't fully deliver the gate |
| Spec source exists | `test -f docs/strategy/layers/layer-6-request-pipeline-automation.md` | If absent → STOP; pipemaxxer was misinvoked |
| Manifest exists | `test -f docs/pipeline/l6-request-pipeline-automation/manifest.yaml` | If absent → STOP; re-run pipemaxxer |
| AgentMail server healthy | `curl -s http://127.0.0.1:8765/health/liveness` returns `{"status":"alive"}` | If down → start AgentMail; bin/br-claim will fail otherwise (but step 01 has no bead-claim work, so this is informational) |
| `bid_workflow_states` table exists in L0 schema | `grep -n "CREATE TABLE.*bid_workflow_states" migrations/0001_sqlite_state_initial.sql` returns line 437 | If different line → spec's data-changes section may be stale; verify columns match spec §1.4 |
| `bid_workflow_states` already in push set | `grep -n "bid_workflow_states" crates/daemon/src/db/sync.rs` returns line 39 inside `PUSH_OWNED_RESEED_TABLES` | If absent → sync engine doesn't push bid state; need an L4 fix-up before L6 |
| Existing `auth-signin/procontract/` exists (decision_2) | `test -f skills/auth-signin/procontract/SKILL.md` | If absent → operator confirmation of decision_2 was wrong; STOP and re-clarify |
| Existing `procurement-ingestion/procontract-register-interest/` exists (reference pattern) | `test -f skills/procurement-ingestion/procontract-register-interest/SKILL.md` | If absent → reference pattern unavailable; the bid skill bead body needs alternative pattern reference |
| Existing layer-doc epic `testproject1-rc0` exists | `br show testproject1-rc0 --json` returns title containing "Layer 6" | If absent → decision_4 sister-epic step 07 needs revisiting |
| `prompts/stage-15-bid-workflow-handoff-DEFERRED.md` exists | `test -f prompts/stage-15-bid-workflow-handoff-DEFERRED.md` | If absent → can't supersede in step 15 archive; informational |

If ANY pre-flight check fails, STOP and report which one. Do not begin spec editing
with broken assumptions about the surrounding state.

---

## Your Mission

The L6 spec (`docs/strategy/layers/layer-6-request-pipeline-automation.md`, 383 lines)
is structurally good — 2 sections, clear state machine, explicit DoD criteria, named
test files. Before it enters the TDD pipeline, you must:

1. Add stable `L6-FR-X.Y.Z` requirement IDs (current spec uses prose; downstream TDD
   coverage matrices need greppable IDs).
2. Verify every requirement has explicit, bullet-listed acceptance criteria.
3. Transcribe **5 operator decisions pre-resolved 2026-05-21** verbatim into the spec.
4. Scan for scope-drift language.
5. Verify the L5→L6 readiness gate (5 blocker conditions) is preserved verbatim.

**Cardinal rule (ZERO SCOPE DRIFT):** every feature in the spec ships. Phases are
execution order, not importance tiers. The five recorded decisions in
`manifest.yaml#recorded_decisions` are NAMING + ARCHITECTURE clarifications; they do
NOT reduce scope. If you find yourself deleting features, stop.

---

## Source Documents (Read In This Order)

### The Spec to Validate (TARGET — read fully)
```
docs/strategy/layers/layer-6-request-pipeline-automation.md         (383 lines)
```

### Manifest (recorded decisions + spec callouts — READ FULLY before editing)
```
docs/pipeline/l6-request-pipeline-automation/manifest.yaml          # recorded_decisions + spec_callouts blocks
```

### Cross-Layer Context (SKIM)
```
docs/strategy/layer-strategy.md                                     # §8 Layer 6 + §10 Interface Contracts
docs/strategy/cross-reference-index.md                              # rows 224-232 + 252 + 280 (L6 references)
docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-completion-audit-2026-05-20.md
                                                                    # confirms L5 ready_for_l6: true
```

### Codebase (STUDY — verify file paths the spec mentions actually exist)
```
migrations/0001_sqlite_state_initial.sql:437-451                    # bid_workflow_states schema (already exists from L0)
crates/daemon/src/db/sync.rs:24-50                                  # PUSH_OWNED_RESEED_TABLES (bid_workflow_states already at line 39)
crates/daemon/src/errors/manifest.rs:19-114                         # ErrorCode enum (~46 variants after L5)
crates/daemon/src/validation/policy.rs:17-25                        # ValidationPolicyMode enum (Always/Sample/Trigger/Skip)
skills/auth-signin/procontract/SKILL.md                             # existing — to be reused by bid skill
skills/procurement-ingestion/procontract-register-interest/SKILL.md # reference pattern (v1.1.0, external_mutation)
skills/validation/overlay-supplier-enrichment/SKILL.md              # frontmatter precedent for L6 validator overlay
```

### Quality Bar (MATCH OR EXCEED)
```
docs/strategy/layers/layer-5-expanded-commercial-intelligence.md    # L5 spec — same structural conventions
docs/strategy/layers/layer-4-shared-visibility.md                   # L4 spec — RLS/web-portal references
docs/pipeline/l5-expanded-commercial-intelligence/archived/2026-05-21-l5-expanded-commercial-intelligence/01-validate-spec.md
                                                                    # 398 lines — L5's validate-spec for structural reference
```

---

## Cardinal Rule: ZERO SCOPE DRIFT

**If it's in the spec, it gets implemented. No exceptions.**

Your job is to IMPROVE structure (IDs, ACs, callouts) and transcribe pre-resolved
decisions. It is NEVER to reduce scope.

You may NOT:
- Remove DoD criteria (11 of them)
- Defer features to "post-L6," "Phase 7+," or any non-existent future iteration
- Label any of the 11 invariants as "nice to have" or "stretch goal"
- Silently drop any of the 19 bid-workflow flow steps from §1.5
- Reduce the 11-step skill playbook in §1.7
- Weaken the L5→L6 readiness gate's 5 blocker conditions

You MAY:
- Add `L6-FR-X.Y.Z` IDs (purely additive)
- Extract acceptance criteria from prose into bullet lists (clarity, not reduction)
- Insert operator-decision callouts (decisions 1-5 from `manifest.yaml#recorded_decisions`)
- Insert naming alias notes (Proactis = Procontract)
- Bump the spec's Implementation Status callout to "VALIDATED FOR IMPLEMENTATION PIPELINE (2026-05-21)"

---

## Key Context From Codebase Research

- **`bid_workflow_states` already exists** in L0 schema with all spec'd columns (`process_id`, `bid_state`, `bid_scope_json`, `review_required`, `bid_job_id`, etc.). No DDL changes needed for the table.
- **`bid_workflow_states` is already in the sync push set** (`crates/daemon/src/db/sync.rs:39`). L6 mutations auto-flow to Supabase via L4's sync engine — no sync-engine changes.
- **`auth-signin/procontract/` already exists** (v1.0.0, CDP login via Chrome saved credentials for `sales@sorelon.com`, Touch ID requirement removed 2026-05). bid-portal-procontract DELEGATES to this skill.
- **`procurement-ingestion/procontract-register-interest/` already exists** (v1.1.0, `side_effect_class: external_mutation`, ~446-line SKILL.md). This is the REFERENCE PATTERN for the new bid-portal skill — same CDP attach pattern, same portal, more elaborate playbook.
- **All 5 L5 readiness-gate blocker codes are already raised by daemon code**: `CLASSIFICATION_AMBIGUOUS`/`TARIFF_BRANCH_DIVERGENCE`/`SUPPLEMENTARY_UNIT_MISSING` at `crates/daemon/src/main.rs:5510-5527`; `decision_grade=broker_required` check at line 5781; `classification_fresh_until_expired()` at lines 5537-5542. L6 only adds a `blocks_ready_to_bid` aggregator above them.
- **ErrorCode enum live boundaries** (verified via direct file read 2026-05-21): `pub enum ErrorCode` at line 20 (NOT 19 — line 19 is the `#[derive(...)]` attribute); enum body closes at line 64 (~45 variants); `impl ErrorCode { fn as_str() ... }` at lines 65-115; `parse_error_code()` function at line ~365. L6 adds NEW variants inside the enum body (before line 64) AND new arms inside the as_str() impl (lines 65-115) AND new arms inside parse_error_code() — all three must update together for the round-trip test to pass.
- **`bid_workflow_states` schema is at line 437 of `migrations/0001_sqlite_state_initial.sql`** (verified via direct grep). Columns: process_id (PK FK), bid_state (TEXT default 'not_started'), bid_scope_json (TEXT nullable), review_required (INTEGER), bid_job_id, started_at, completed_at, owner_id, row_version, last_local_seq, stage_run_id, updated_at, created_at. **NO CHECK constraint on bid_state** — Rust enforces.
- **`bid_workflow_states` is already at index 14 (line 39) of `PUSH_OWNED_RESEED_TABLES` in `crates/daemon/src/db/sync.rs`** (verified). L6 sync is no-op.
- **STAGE-15 SUPERSESSION ALERT**: `prompts/stage-15-bid-workflow-handoff-DEFERRED.md` was authored 2026-05-02 with a DIFFERENT 5-state state machine (`not_started → drafting → reviewing → submitted → tracking → won|lost|withdrawn`). This is OBSOLETE — L6 spec §1.3's 7-state machine (`not_started → ready_to_bid → bid_in_progress → draft_saved → bid_submitted | bid_failed | portal_state_unknown`) is AUTHORITATIVE and supersedes it. The deferred handoff is flagged for cleanup in step 15 archive. Do NOT use the stage-15 state machine as a reference.
- **3-6 new ErrorCode variants** (final count decided in TDD step 03 after redundancy audit against existing codes — see `manifest.yaml#new_modules_rust` for the full strawman + likely-redundant analysis). Likely-new: `BID_SUBMISSION_FAILED`, `BID_DUPLICATE_DETECTED`, `BID_SIDE_EFFECT_CAP_EXCEEDED`, `BID_READINESS_GATE_FAILED`. Likely-redundant (TDD verifies): `BID_PORTAL_CHANGED` (existing `PAGE_SHAPE_DRIFT` may cover), `BID_PORTAL_AUTH_EXPIRED` (existing `AUTH_STATE_STALE` may cover; auth refresh is daemon-orchestrated retry, NOT bid-skill direct invocation of auth-signin). Each NEW variant requires enum variant + `as_str()` arm + clippy-surfaced exhaustive matches in 5-15 sites (per L5 precedent).
- **No new cron entries needed** — bid workflow is operator-triggered, not scheduled.
- **The web portal will gain READ-ONLY bid_state visibility** (RLS viewer-role enforcement per L4 pattern); no web mutation surface.

---

## Step-by-Step Workflow

### Step 0: Read Source Documents (MANDATORY — before editing)

1. `docs/strategy/layers/layer-6-request-pipeline-automation.md` — **READ FULLY**
2. `docs/pipeline/l6-request-pipeline-automation/manifest.yaml` — **READ FULLY** (`recorded_decisions` + `spec_callouts` blocks)
3. `docs/strategy/layer-strategy.md` — **SKIM** §8 + §10
4. `docs/strategy/cross-reference-index.md` — **REFERENCE** rows 224-232 for L6 entries
5. `docs/pipeline/l5-expanded-commercial-intelligence/artifacts/l5-completion-audit-2026-05-20.md` — **SKIM** to confirm L5 ready_for_l6: true
6. `skills/auth-signin/procontract/SKILL.md` — **STUDY** (existing auth pattern L6 delegates to)
7. `skills/procurement-ingestion/procontract-register-interest/SKILL.md` — **STUDY** (reference pattern for L6 bid skill)
8. `docs/pipeline/l5-expanded-commercial-intelligence/archived/2026-05-21-l5-expanded-commercial-intelligence/01-validate-spec.md` — **MATCH OR EXCEED** structural quality

### Step 1: Add Requirement IDs

Scheme: `L6-FR-{section}.{subsection}.{ordinal}` — by spec section/subsection.

Examples (illustrative; you assign the final values):

```
L6-FR-1.1.1   Layer narrative: bid skills automate request pipeline (preparation + submission)
L6-FR-1.1.2   Discovery source ≠ submission portal — bid workflows modeled around submission portal
L6-FR-1.2.1   Capability: agent navigates portal via browser automation
L6-FR-1.2.2   Capability: company profile data injected from shared_company_profile
... (continue through §1.10 + §2)
L6-FR-1.3.1   bid_workflow_states.bid_state value: 'not_started' (default)
L6-FR-1.3.2   bid_workflow_states.bid_state value: 'ready_to_bid'
L6-FR-1.3.3   bid_workflow_states.bid_state value: 'bid_in_progress'
L6-FR-1.3.4   bid_workflow_states.bid_state value: 'draft_saved'
L6-FR-1.3.5   bid_workflow_states.bid_state value: 'bid_submitted'
L6-FR-1.3.6   bid_workflow_states.bid_state value: 'bid_failed'
L6-FR-1.3.7   bid_workflow_states.bid_state value: 'portal_state_unknown'
L6-FR-1.3.8   Transition: not_started → ready_to_bid (operator approved + not stale + L5 gate)
L6-FR-1.3.9   Transition: ready_to_bid → bid_in_progress (operator click + prerequisites re-verified)
L6-FR-1.3.10  Transition: ready_to_bid → not_started (decision stale / blocker activates / window expires / operator withdraws)
L6-FR-1.3.11  Transition: bid_in_progress → draft_saved
L6-FR-1.3.12  Transition: bid_in_progress → bid_submitted
L6-FR-1.3.13  Transition: bid_in_progress → bid_failed
L6-FR-1.3.14  Transition: bid_in_progress → portal_state_unknown
L6-FR-1.3.15  Transition: draft_saved → bid_submitted
L6-FR-1.3.16  Transition: draft_saved → bid_in_progress
L6-FR-1.3.17  Transition: draft_saved → bid_failed
L6-FR-1.3.18  Transition: portal_state_unknown → bid_in_progress
L6-FR-1.3.19  Transition: portal_state_unknown → bid_submitted
L6-FR-1.3.20  Transition: portal_state_unknown → bid_failed
L6-FR-1.3.21  Transition: bid_failed → ready_to_bid
# ^ The spec §1.3 transition table has 14 distinct From→To rows (re-verified 2026-05-21). Do not undercount.
L6-FR-1.3.30  Prerequisite check (constitutional): operator approved disposition required
L6-FR-1.3.21  Prerequisite check: decision NOT stale (reviewedBasisHash == current)
L6-FR-1.3.22  Prerequisite check: no active L5 blockers (5 enumerated codes)
L6-FR-1.3.23  Prerequisite check: classification freshness valid
L6-FR-1.3.24  Prerequisite check: quote-role quality gate
L6-FR-1.3.25  Prerequisite check: company profile documents current
L6-FR-1.3.26  Prerequisite check: upstream pipeline trust gate
L6-FR-1.3.27  Validation policy fail-closed for portal submission
L6-FR-1.4.1   BidScopePayload: processId required
L6-FR-1.4.2   BidScopePayload: lotIds[] (which lots in scope)
... continue for all BidScopePayload fields + SideEffectClass union
L6-FR-1.5.1...19  Each of the 19 numbered bid workflow flow steps
L6-FR-1.6.1   packages/bidding/workflow-registry.ts (file exists)
L6-FR-1.6.2   packages/bidding/procontract-workflow.ts (per decision_1 naming reconciliation)
L6-FR-1.6.3   packages/bidding/company-profile-schema.ts
L6-FR-1.7.1...11  Each of the 11 skill playbook steps
L6-FR-1.8.1...6   Each of the 6 audit-trail artifact types
L6-FR-1.10.1...8  Each of the 8 invariants
L6-FR-2.1.1...11  Each of the 11 DoD criteria
```

Total expected: ~80-120 unique IDs. The exact count depends on your subdivision —
prefer finer-grain (one ID per state, transition, invariant, DoD criterion) so the
TDD coverage matrix can map test IDs 1:1.

### Step 2: Verify ACs Are Explicit

For every `L6-FR-X.Y.Z`, check the surrounding prose has an explicit, bullet-listed
acceptance criterion. Most of the spec already does this (the §1.3 state transition
table HAS explicit "Requirements" column entries; §1.10 invariants HAS an
"Enforcement" column). Where prose buries the AC, extract it.

Example: `L6-FR-1.3.27` (validation policy fail-closed for portal submission). Current
spec text (§1.3 lines 81-82) explains the rationale. Add explicit ACs:
- Validator rejection → `bid_submitted` transition BLOCKED
- Validator outage → `bid_submitted` transition BLOCKED
- Workflow MAY reach `draft_saved` without validation clearance
- Operator MAY override the validation gate at final submission (logged)

### Step 2.5: Handle Open Questions (NEW — added 2026-05-21 9th-pass audit)

Read `manifest.yaml#open_questions` — 15 items grouped by tier:
- **Tier 1 (q1-q6):** architectural; BLOCKS step 05/07. MUST be resolved before step 07 runs. For each: either operator has pre-resolved (transcribe into spec §1.0 as decisions 6+), OR step 01 creates an explicit "Operator Decision Required" callout in spec that blocks step 07.
- **Tier 2 (q7-q10):** implementation details; TDD/strategy can decide with operator input valuable. For each: surface in spec as "(TDD-decided)" annotation OR pre-resolve.
- **Tier 3 (q11-q14):** live-demo/operational; operator must arrange BEFORE step 12, NOT before step 01. Surface in spec as Phase 7 prerequisite checklist OR live-demo prep note.
- **Tier 4 (q15):** cosmetic; defer to strategy author.

For every Tier 1 item that is NOT pre-resolved in `recorded_decisions`:
1. Add an explicit "⚠ OPERATOR DECISION REQUIRED" callout in the spec at the relevant subsection
2. List the options + recommended default from `open_questions.qN`
3. Flag step 07 cannot proceed until each is resolved (matches L5's pattern where decisions blocked step 07 bead creation)

This step is the SAFETY NET against accidental "TDD just picks the easy default" outcomes. Operator input on Tier 1 questions matches L5's pre-resolution discipline.

### Step 3: Transcribe Operator Decisions (verbatim from manifest.yaml)

Add a new subsection `§1.0 Recorded Decisions (2026-05-21)` IMMEDIATELY after §1
Overview. Transcribe ALL 5 decisions from `manifest.yaml#recorded_decisions`:

1. **Decision 1: Proactis === Procontract naming reconciliation.** Quote rationale + implementation. Add the alias note in §1.5/§1.6/§1.7 wherever "Proactis" appears.
2. **Decision 2: Reuse existing skills/auth-signin/procontract/.** Quote rationale + implementation. Update §1.7 step 2 to delegate-to-existing-skill.
3. **Decision 3: Validator overlay included as L6 deliverable.** Quote rationale + implementation. Add to §1.6 or §1.7 as a 4th file family.
4. **Decision 4: Sister implementation epic.** Quote rationale; document the step-07 procedure for closing rc0 and creating the sister epic.
5. **Decision 5: Procontract access is already solved.** Quote rationale; the live demo (DoD #8) is feasible from day one via existing CDP browser + saved credentials.

For decisions 1-2, additionally add a banner near the top of §1 explaining:
> **Naming note (2026-05-21):** This spec was authored referring to "Proactis." In the
> codebase, the same portal is called "Procontract" (URL: `procontract.due-north.com`).
> The two names are interchangeable; new files use `procontract` to match the existing
> `skills/auth-signin/procontract/` and `skills/procurement-ingestion/procontract-*`
> convention. The capability delivered is identical.

### Step 4: Scope-Drift Scan

`grep -i` the spec for these phrases. **Any hit (outside the explicit "What's NOT
Stubbed" section, which currently states "Nothing — this is the final layer") is a
BLOCKING failure:**

- "post-MVP", "post-L6", "Phase 7+"
- "stretch goal", "nice to have"
- "out of scope", "deferred"
- "future iteration", "later"
- "simplified for initial implementation"
- "can be added later"

Expected result: **zero hits**. The spec §2.8 currently reads "Nothing — this is the
final layer. No features are deferred beyond this point." Confirm this remains true.

### Step 5: Verify L5→L6 Readiness Gate (CONSTITUTIONAL)

The 5 blocker conditions in §1.3 lines 73-80 MUST remain verbatim:
1. Operator `approved` disposition on `operator_decision_heads` (not just `pending_operator_intents`)
2. Decision is NOT stale (`reviewedBasisHash == current decisionBasisHash`)
3. No active L5 blockers: `CLASSIFICATION_AMBIGUOUS`, `TARIFF_BRANCH_DIVERGENCE`, `SUPPLEMENTARY_UNIT_MISSING`, `decision_grade = broker_required`
4. Classification freshness still valid (`classificationFreshUntil` not expired)
5. Company profile documents/certificates current

Plus the 2 quality gates:
- Quote-role quality gate (`domestic_wholesale_floor` / `optimistic_floor` evidence requires explicit operator override)
- Upstream pipeline trust gate (`pipeline_assessments.validation_state = validated` for the scope)

Cross-reference: `docs/strategy/layer-strategy.md` §10 Interface Contracts row "L5→L6 readiness gate"; `docs/strategy/cross-reference-index.md` rows 230 + 232.

### Step 6: Bump Implementation Status Callout

Replace the spec's current top-of-file callout:

```markdown
> **Implementation Status (as of 2026-05-02):** Layer 6 is **PLANNED — not yet built**.
> No production request pipeline automation or bid-workflow submission code exists for
> this layer. Stage 15 (`bid_workflow`) is L6-blocked — see
> `prompts/stage-15-bid-workflow-handoff-DEFERRED.md` for the planned approach once L6
> is built. This doc remains a planned execution-grade specification and intentionally
> has no inline implementation markers.
```

With:

```markdown
> **Implementation Status (as of 2026-05-21):** Layer 6 is **VALIDATED FOR IMPLEMENTATION
> PIPELINE**. Requirement IDs (`L6-FR-X.Y.Z`) added; 5 operator decisions transcribed in
> §1.0; naming reconciliation Proactis=Procontract confirmed; L5→L6 readiness gate
> preserved verbatim. Pipeline at `docs/pipeline/l6-request-pipeline-automation/`.
> Stage 15 (`bid_workflow`) deferred handoff at `prompts/stage-15-bid-workflow-handoff-DEFERRED.md`
> will be reopened or superseded once L6 implementation completes.
```

### Step 7: Sub-Optimization Pass (Send as a follow-up message AFTER the main work)

> ULTRATHINK. Re-read the edited spec with completely fresh eyes.

Specifically verify:

1. **Open `migrations/0001_sqlite_state_initial.sql` and read lines 437-451**. Does
   §1.3's state-machine list of 7 lifecycle values match what the column allows? (No
   SQL CHECK constraint — Rust enforces. Confirm the spec acknowledges this.)

2. **Open `crates/daemon/src/errors/manifest.rs` and read lines 19-114**. List every
   existing `ErrorCode` variant. For each of the 6 new L6 codes in
   `manifest.yaml#new_modules_rust`, confirm the variant name doesn't collide with an
   existing one. Confirm the spec text (or §3 if you added an Errors subsection) names
   all 6 with crisp, distinct semantics.

3. **Open `skills/auth-signin/procontract/SKILL.md`** (the existing auth-signin skill).
   Does the spec §1.7 step 2 callout you added accurately describe the delegation
   pattern? Specifically: the bid skill detects URL contains `/Login` OR text matches
   `Session expired|Please log in`, invokes `auth-signin/procontract`, then verifies
   signed-in state before continuing? (This is the same pattern used by
   `procontract-register-interest`.)

4. **Open `skills/procurement-ingestion/procontract-register-interest/SKILL.md` and
   read the "Mandatory CDP Discipline" section.** Does the spec's §1.7 imply that
   `bid-portal-procontract` will attach to the SAME persistent CDP browser at
   `/Users/alinaaitova/.chrome-automation` on port 9222, NOT launch a fresh profile?
   If not, add a callout.

5. **Open `crates/daemon/src/db/sync.rs` lines 24-50.** Confirm `bid_workflow_states`
   is at index 14 (line 39) in `PUSH_OWNED_RESEED_TABLES`. The spec §1.9 says "synced
   to Supabase for web visibility" — does it explicitly note no sync-engine changes
   needed? If not, add a callout.

6. **Open `docs/strategy/cross-reference-index.md` rows 224-232 + 252 + 280.** Are all
   L6 capabilities mentioned in the index reflected in the spec? Specifically rows
   230 + 232 (L5→L6 readiness gate) — both reference the SAME 5 blockers but should
   the spec also link to layer-strategy.md §10? Add the cross-reference if missing.

7. **Count the requirement IDs you added.** Aim for 80-120. If under 80, you may have
   conflated multiple requirements into one ID. If over 150, you may have over-divided
   (one ID per sub-bullet is fine; one per word is not).

8. **Walk every §2.1 DoD criterion.** For each (1-11), name the `L6-FR-X.Y.Z` IDs
   that, when implemented, satisfy that DoD criterion. If you find a DoD criterion with
   ZERO mapped FRs, your IDs are incomplete — add the missing FR.

9. **Confirm zero scope-drift language.** Re-run the grep from Step 4. Confirm zero
   hits.

10. **Verify the 5 recorded decisions are transcribed verbatim** (not paraphrased) and
    have all required fields (choice, rationale, implementation, recorded_at).

If any of the above turns up a gap, fix it in the spec and re-run the verification.

### Step 8: Scope-Preservation Gate (MANDATORY — BLOCKING)

Before declaring this step complete:

- [ ] Every feature from §1.1 through §1.10 has an `L6-FR-X.Y.Z` ID
- [ ] All 11 DoD criteria (§2.1) have IDs
- [ ] All 11 test scenarios (§2.3) are still present (they may not need new IDs but they MUST remain)
- [ ] All 4 risks (§2.4) are still present
- [ ] All 4 forward-compatibility provisions (§2.5) are still present
- [ ] All 8 invariants (§1.10) are still present and have IDs
- [ ] All **14** state-machine transitions (§1.3 transition table rows) are still present and have IDs — the table has 14 distinct From→To pairs; do not undercount as 11
- [ ] All 19 bid workflow flow steps (§1.5) are still present and have IDs
- [ ] All 11 skill playbook steps (§1.7) are still present and have IDs
- [ ] All 6 SideEffectClass values (§1.4) are still present
- [ ] All 6 audit-trail artifact types (§1.8) are still present
- [ ] The 5 L5→L6 readiness blockers (§1.3 lines 73-80) are still present verbatim
- [ ] §2.8 still reads "Nothing — this is the final layer. No features are deferred beyond this point."
- [ ] Zero scope-drift language found by Step 4 grep
- [ ] 5 operator decisions transcribed in §1.0
- [ ] Proactis = Procontract alias note present in §1.5/§1.6/§1.7
- [ ] Implementation Status callout bumped to VALIDATED FOR IMPLEMENTATION PIPELINE (2026-05-21)

If ANY checkbox above is `[ ]`, fix it before proceeding to step 02.

---

## Output

- **Same spec file**, validated, structured, IDs added, 5 decisions transcribed, naming reconciliation banner present
- **Implementation Status** callout bumped
- **No new files** produced by this step
- Bead activity: NONE (this step is doc validation, not bead-tracked work)

---

## Key Reminders

- **No skill invocation** — this is manual editing
- **No content removal** — only addition (IDs, callouts, decisions, ACs)
- **Naming reconciliation Proactis=Procontract is the largest semantic edit** — get it right
- The 5 recorded decisions are PRE-RESOLVED. Step 01 transcribes them; it does NOT re-decide.
- ZERO SCOPE DRIFT: every feature in the spec must remain. If you find yourself thinking "this can wait" — it CANNOT.
- Quality bar: L5's `01-validate-spec.md` (398 lines, 9-callout resolution). Match or exceed structural rigor.
