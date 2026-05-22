# Layer 6 — Request Pipeline Automation

> **Authority:** This document is the execution-grade specification for Layer 6. An implementation agent should be able to build this layer from this document alone without reopening the research document.
>
> **Milestone:** M6
> **Dependencies:** Layers 0-5 (full pipeline, desktop, web, sync, decision-grade intelligence)
> **Exit criteria:** System can prepare and submit one request on a target portal end-to-end with full audit trail.

> **Implementation Status (as of 2026-05-21):** Layer 6 is **VALIDATED FOR IMPLEMENTATION
> PIPELINE**. Requirement IDs (`L6-FR-X.Y.Z`) added; 5 operator decisions transcribed in
> §1.0; naming reconciliation Proactis=Procontract confirmed; L5→L6 readiness gate
> preserved verbatim. Pipeline at `docs/pipeline/l6-request-pipeline-automation/`.
> Stage 15 (`bid_workflow`) deferred handoff at `prompts/stage-15-bid-workflow-handoff-DEFERRED.md`
> will be reopened or superseded once L6 implementation completes.

---

## 1. Overview & Bid Workflow System

### 1.0 Recorded Decisions (2026-05-21)

**Naming note (2026-05-21):** This spec was authored referring to "Proactis." In the codebase, the same portal is called "Procontract" (URL: `procontract.due-north.com`). The two names are interchangeable; new files use `procontract` to match the existing `skills/auth-signin/procontract/` and `skills/procurement-ingestion/procontract-*` convention. The capability delivered is identical.

Decision 1: Proactis === Procontract naming reconciliation
- AC1: New codebase artifacts use `procontract` naming: `packages/bidding/procontract-workflow.ts`, `skills/bid-portal-procontract/`, and `bidPortalType: 'procontract'`.
- AC2: Existing prose references to Proactis are treated as aliases for the same Procontract portal and do not reduce scope.
- Choice: "Proactis === Procontract — same portal, same website (procontract.due-north.com)"
- Rationale: "Operator confirmed 2026-05-21: 'Proactis is the same thing as Procontract. It's the same website. There's a CDP Agent Browser skill that lets you input the username and password saved in Google Chrome.' The L6 spec was authored with 'Proactis' naming but the codebase uses 'procontract' (skills/auth-signin/procontract/, skills/procurement-ingestion/procontract-*). This is a NAMING reconciliation, not scope drift — same capability."
- Implementation:
  - naming_in_codebase: "procontract"
  - naming_in_spec_text: "either accepted; cross-reference alias note added"
  - file_renames_vs_spec:
    - "spec §1.6 lists `packages/bidding/proactis-workflow.ts` → use `procontract-workflow.ts` instead"
    - "spec §1.7 lists `skills/bid-portal-proactis/` → use `skills/bid-portal-procontract/` instead"
    - "spec §1.7 lists `bidPortalType: 'proactis'` in BidScopePayload — use string literal `'procontract'`"
    - "spec §2.5 lists `pdf-extraction/bid-template` overlay — preserve naming as `bid-template` (portal-agnostic)"
- Recorded by: operator
- Recorded at: "2026-05-21"

Decision 2: reuse existing `skills/auth-signin/procontract/`
- AC1: L6 does not create `skills/auth-signin/proactis/`.
- AC2: `skills/bid-portal-procontract/` detects stale auth and returns `AUTH_STATE_STALE` cleanly; the daemon orchestrates the `auth-signin/procontract` retry and then reruns the original bid job.
- Choice: "REUSE existing skills/auth-signin/procontract/ — do NOT create skills/auth-signin/proactis/"
- Rationale: "Operator confirmed 2026-05-21: 'There's a CDP Agent Browser skill that lets you input the username and password saved in Google Chrome. We recently removed the Touch ID necessity, so you should have no problem with that.' The existing auth-signin/procontract/SKILL.md (~339 lines, v1.0.0) handles login via macOS Passwords autofill against procontract.due-north.com. The new bid-portal skill attaches to the same persistent CDP browser at /Users/alinaaitova/.chrome-automation (port 9222) and delegates auth to the existing skill when session is detected as expired (URL contains /Login or text matches 'Session expired|Please log in')."
- Implementation:
- Existing skills reused: `skills/auth-signin/procontract/` (login via Chrome saved credentials, v1.0.0), `skills/procurement-ingestion/procontract-register-interest/` (external-mutation CDP reference pattern, v1.1.0), `skills/procurement-ingestion/procontract-discover/` (navigation and tender lookup), and `skills/procurement-ingestion/procontract-enrich-tender/` (document download and extraction).
  1. `bid-portal-procontract` attaches to the persistent CDP browser via `bin/ensure-cdp-9222`.
  2. If URL contains `/Login` OR text matches `Session expired|Please log in`, the skill returns `AUTH_STATE_STALE` and exits cleanly; it does not attempt login.
  3. The daemon's existing `AUTH_STATE_STALE` handler runs `auth-signin/procontract` as a sub-job.
  4. On auth-signin success, the daemon reruns the original `bid-portal-procontract` job.
- Recorded by: operator
- Recorded at: "2026-05-21"

Decision 3: validator overlay included as L6 deliverable
- AC1: L6 includes `skills/validation/overlay-bid-portal-procontract/` as part of implementation scope.
- AC2: The overlay checks portal-specific anomalies rather than relying only on the base validator.
- Choice: "YES — include skills/validation/overlay-bid-portal-procontract/ as L6 deliverable"
- Rationale: "Spec §1.7 says bid skills use validation_policy: always (fail-closed). Without a domain-specific overlay, validation falls back to base validator, which won't catch portal-specific anomalies (wrong opportunity ref, side-effect class drift, screenshot completeness, draft vs final submission state divergence). Matches L5's overlay-customs-classification recommendation."
- Implementation: create `skills/validation/overlay-bid-portal-procontract/` during Phase 5 following `skills/validation/overlay-supplier-enrichment/`; frontmatter declares `worker_behavior_version: bid-portal-procontract/*@VERSION+1.0` and `validator_behavior_version: validation/overlay-bid-portal-procontract@VERSION+1.0`.
- Assertion strawman for TDD finalization: opportunity ref matches `BidScopePayload.portalOpportunityRef`; screenshot count matches run shape; side-effect progression is monotonic; side-effect class never exceeds `BidScopePayload.sideEffectCap`; submission confirmation reference is well-formed when present; agent narrative cites lot/candidate IDs verbatim; no PII leaks into the narrative.
- Recorded by: operator
- Recorded at: "2026-05-21"

Decision 4: sister implementation epic
- AC1: Step 07 closes `testproject1-rc0` as the doc-only layer epic after confirming the doc-only scope.
- AC2: Step 07 creates a sister implementation epic titled `Epic: L6 Implementation — Request Pipeline Automation` and threads all implementation beads under it.
- Choice: "Sister epic — close rc0 as doc-only, create 'Epic: L6 Implementation — Request Pipeline Automation'"
- Rationale: "Matches L4 (testproject1-4zq) + L5 sister-epic pattern. Clean separation between doc-authoring (rc0 — already complete) and implementation. Recorded in project memory as preferred."
- Implementation:
  Step 07 procedure:
  1. Verify `testproject1-rc0` doc-only status with `br show testproject1-rc0 --json`.
  2. Close rc0 with reason `Layer doc produced; implementation tracked in sister epic`.
  3. Create the sister epic with `br create --type=epic --priority=P0 --title="Epic: L6 Implementation — Request Pipeline Automation" --labels l6`.
  4. Capture the resulting epic ID under `existing_implementation_epic`.
  5. Thread all Phase 1-N implementation beads under the new epic via `--parent`.
- Recorded by: operator
- Recorded at: "2026-05-21"

Decision 5: Procontract access is already solved
- AC1: DoD #8 live-demo feasibility is not blocked on creating a new auth skill.
- AC2: First live demo uses `sideEffectCap = REMOTE_DRAFT`; `FORMAL_SUBMISSION` requires separate explicit operator authorization for a chosen genuine opportunity.
- Choice: "DoD #8 live-demo is NOT a hard prerequisite blocker — operator already has Procontract access"
- Rationale: "Operator confirmed 2026-05-21: existing auth-signin/procontract/ + persistent CDP Chrome profile with Touch-ID-removed saved credentials means the live demo is feasible from day one. No operator-built skill prerequisite needed (unlike L5's freightos/taric). The live demo can run as part of step 12 completion audit against a real Procontract opportunity."
- Implementation: bootstrap documents the persistent CDP profile at `/Users/alinaaitova/.chrome-automation`, verifies Chrome saved credentials for `sales@sorelon.com` with Touch ID disabled, and records an operator-selected Procontract opportunity reference for the live-demo run.
- Live demo constraint: "First live demo runs with sideEffectCap = REMOTE_DRAFT (draft_saved terminal); FORMAL_SUBMISSION class only after operator manually authorizes a separate run with a chosen genuine opportunity."
- Recorded by: operator
- Recorded at: "2026-05-21"

**Verbatim `manifest.yaml#recorded_decisions` transcription:**

```yaml
recorded_decisions:
  decision_1_proactis_is_procontract:
    choice: "Proactis === Procontract — same portal, same website (procontract.due-north.com)"
    rationale: "Operator confirmed 2026-05-21: 'Proactis is the same thing as Procontract. It's the same website. There's a CDP Agent Browser skill that lets you input the username and password saved in Google Chrome.' The L6 spec was authored with 'Proactis' naming but the codebase uses 'procontract' (skills/auth-signin/procontract/, skills/procurement-ingestion/procontract-*). This is a NAMING reconciliation, not scope drift — same capability."
    implementation:
      # The spec text remains intelligible referring to "Proactis"; codebase artifacts
      # use "procontract" to match existing convention. Step 01 adds the alias note.
      naming_in_codebase: "procontract"             # NEW files use this
      naming_in_spec_text: "either accepted; cross-reference alias note added"
      file_renames_vs_spec:
        - "spec §1.6 lists `packages/bidding/proactis-workflow.ts` → use `procontract-workflow.ts` instead"
        - "spec §1.7 lists `skills/bid-portal-proactis/` → use `skills/bid-portal-procontract/` instead"
        - "spec §1.7 lists `bidPortalType: 'proactis'` in BidScopePayload — use string literal `'procontract'`"
        - "spec §2.5 lists `pdf-extraction/bid-template` overlay — preserve naming as `bid-template` (portal-agnostic)"
    recorded_by: operator
    recorded_at: "2026-05-21"

  decision_2_existing_procontract_infrastructure_reuse:
    choice: "REUSE existing skills/auth-signin/procontract/ — do NOT create skills/auth-signin/proactis/"
    rationale: |
      Operator confirmed 2026-05-21: 'There's a CDP Agent Browser skill that lets
      you input the username and password saved in Google Chrome. We recently removed
      the Touch ID necessity, so you should have no problem with that.' The existing
      auth-signin/procontract/SKILL.md (~339 lines, v1.0.0) handles login via macOS
      Passwords autofill against procontract.due-north.com. The new bid-portal skill
      attaches to the same persistent CDP browser at /Users/alinaaitova/.chrome-automation
      (port 9222) and delegates auth to the existing skill when session is detected
      as expired (URL contains /Login or text matches 'Session expired|Please log in').
    implementation:
      existing_skills_reused:
        - skill: skills/auth-signin/procontract/
          purpose: "Login via Chrome saved credentials (no Touch ID required after recent removal)"
          version: "1.0.0"
          interaction_profile: pty_interactive
          requires_cdp: true
          notes: "Operator account sales@sorelon.com — REAL procurement account; the bid skill MUST inherit auth-signin's sensitivity rules (no profile changes outside the bid workflow, no sign-out, etc.)"
        - skill: skills/procurement-ingestion/procontract-register-interest/
          purpose: "Reference pattern for portal-mutation skills with side_effect_class: external_mutation, CDP persistent browser, ProContract URL structure"
          version: "1.1.0"
          notes: "Existing skill that clicks 'Register interest' — L6 bid-portal-procontract is a more elaborate multi-step variant of this same pattern."
        - skill: skills/procurement-ingestion/procontract-discover/
          purpose: "Reference pattern for ProContract navigation; tender reference lookup; CDP attach"
        - skill: skills/procurement-ingestion/procontract-enrich-tender/
          purpose: "Reference pattern for ProContract document download + extraction"
    spec_action: |
      Step 01 validate-spec adds a callout under §1.7 noting: 'bid-portal-procontract
      delegates portal login to skills/auth-signin/procontract/ (existing) via the
      DAEMON-ORCHESTRATED RETRY pattern (the established Procontract skill convention):

        1. bid-portal-procontract attaches to the persistent CDP browser via bin/ensure-cdp-9222
        2. If URL contains /Login OR text matches "Session expired|Please log in",
           the skill returns AUTH_STATE_STALE and exits cleanly (does NOT attempt login)
        3. The daemon's existing AUTH_STATE_STALE handler runs auth-signin/procontract as a sub-job
        4. On auth-signin success, the daemon re-runs the original bid-portal-procontract job

      The bid skill MUST NOT invoke auth-signin/procontract directly as a sub-skill — that
      would bypass the daemon's session/lifecycle bookkeeping. This pattern is identical
      to procurement-ingestion/procontract-register-interest/SKILL.md which the bid skill
      uses as its reference pattern.'
    recorded_by: operator
    recorded_at: "2026-05-21"

  decision_3_validator_overlay_included:
    choice: "YES — include skills/validation/overlay-bid-portal-procontract/ as L6 deliverable"
    rationale: "Spec §1.7 says bid skills use validation_policy: always (fail-closed). Without a domain-specific overlay, validation falls back to base validator, which won't catch portal-specific anomalies (wrong opportunity ref, side-effect class drift, screenshot completeness, draft vs final submission state divergence). Matches L5's overlay-customs-classification recommendation."
    implementation: |
      Create skills/validation/overlay-bid-portal-procontract/ during Phase 5.
      Structure follows skills/validation/overlay-supplier-enrichment/ precedent
      (~84 lines for the customs overlay; ~80-150 lines expected here).
      Frontmatter declares worker_behavior_version: bid-portal-procontract/*@VERSION+1.0
      and validator_behavior_version: validation/overlay-bid-portal-procontract@VERSION+1.0.

      Assertion strawman (TDD step 03 finalizes exact assertions per workflow shape):
        - opportunity ref matches BidScopePayload.portalOpportunityRef
        - screenshot count appropriate to run shape: first run reaching draft_saved
          via 10a path → ~11 screenshots (steps 1-9 + 10a + 11); first run reaching
          bid_submitted via 10b path → ~11 (steps 1-9 + 10b + 11); continuation
          run from draft_saved → variable (TDD defines the rubric)
        - side_effect_class progression is monotonic
        - side_effect_class never exceeds BidScopePayload.sideEffectCap
        - submission confirmation reference (if present) is well-formed
        - agent narrative cites BidScopePayload lot/candidate IDs verbatim
        - no PII leakage in narrative (cross-check redaction_boundary tests)
    bead_impact: "Adds 1 skill directory + ~2-3 beads to Phase 5"
    recorded_by: operator
    recorded_at: "2026-05-21"

  decision_4_implementation_epic_structure:
    choice: "Sister epic — close rc0 as doc-only, create 'Epic: L6 Implementation — Request Pipeline Automation'"
    rationale: "Matches L4 (testproject1-4zq) + L5 sister-epic pattern. Clean separation between doc-authoring (rc0 — already complete) and implementation. Recorded in project memory as preferred."
    implementation: |
      Step 07 actions (in order):
        1. Verify testproject1-rc0 doc-only status: `br show testproject1-rc0 --json`
           confirms description targets the layer doc only.
        2. Close rc0: `br close testproject1-rc0 --reason "Layer doc produced; implementation tracked in sister epic"`
        3. Create sister epic:
           `br create --type=epic --priority=P0 --title="Epic: L6 Implementation — Request Pipeline Automation" --labels l6`
        4. Capture the resulting epic ID and write to status file under
           `existing_implementation_epic: <new-id>`
        5. All Phase 1-N implementation beads thread under this new epic via --parent.
    recorded_by: operator
    recorded_at: "2026-05-21"

  decision_5_proactis_access_already_solved:
    choice: "DoD #8 live-demo is NOT a hard prerequisite blocker — operator already has Procontract access"
    rationale: "Operator confirmed 2026-05-21: existing auth-signin/procontract/ + persistent CDP Chrome profile with Touch-ID-removed saved credentials means the live demo is feasible from day one. No operator-built skill prerequisite needed (unlike L5's freightos/taric). The live demo can run as part of step 12 completion audit against a real Procontract opportunity."
    implementation: |
      Bootstrap doc documents: (a) ensure persistent CDP profile at
      /Users/alinaaitova/.chrome-automation has saved sales@sorelon.com credential,
      (b) verify Touch ID requirement is disabled in Chrome's password autofill,
      (c) identify at least one Procontract opportunity reference for live-demo testing
      (operator selects ANY currently-open opportunity that won't generate buyer-visible
      legal commitment — REMOTE_DRAFT side-effect class cap during live demo).
    live_demo_constraint: "First live demo runs with sideEffectCap = REMOTE_DRAFT (draft_saved terminal); FORMAL_SUBMISSION class only after operator manually authorizes a separate run with a chosen genuine opportunity."
    recorded_by: operator
    recorded_at: "2026-05-21"
```

### 1.0A Operator Decisions Required Before Step 07

The following Tier 1 questions from `manifest.yaml#open_questions` are not pre-resolved in `recorded_decisions`. Step 07 bead creation cannot proceed until each is either recorded as a new operator decision or explicitly converted into a strategy-owned constraint.

⚠ OPERATOR DECISION REQUIRED — q1 module placement
- Options: (A) Phase 1 creates `crates/daemon/src/bid_workflow/` module skeleton; Phase 2 dispatch wires to stub; Phase 3 fills bodies. (B) Inline `run_bid_workflow_job(...)` in `main.rs`. (C) Create a separate `crates/bidding-runner/` sibling crate.
- Recommended default: A.
- Blocks: step 05 strategy phase ordering and step 07 Phase 1 file list.

⚠ OPERATOR DECISION REQUIRED — q2 model profile
- Options: (A) introduce `portal_form_high_reliability` as an L6 deliverable. (B) reuse `browse_stable_low_cost`. (C) introduce the profile as an alias of the existing browsing profile initially.
- Recommended default: A.
- Blocks: Phase 1 daemon model-profile registration and Phase 4 skill frontmatter.

⚠ OPERATOR DECISION REQUIRED — q3 ErrorCode set
- Options: (A) add only `BID_SUBMISSION_FAILED`, `BID_DUPLICATE_DETECTED`, `BID_SIDE_EFFECT_CAP_EXCEEDED`, and `BID_READINESS_GATE_FAILED`; reuse `PAGE_SHAPE_DRIFT` and `AUTH_STATE_STALE`. (B) add all six strawman bid codes including `BID_PORTAL_CHANGED` and `BID_PORTAL_AUTH_EXPIRED`. (C) let TDD step 03 decide after auditing existing variants.
- Recommended default: A.
- Blocks: step 03 TDD and Phase 1 daemon code.

⚠ OPERATOR DECISION REQUIRED — q4 default `reviewRequired`
- Options: (A) default true, requiring operator review for every draft before final submission. (B) default false when validation is clear. (C) make it a per-opportunity desktop choice with no global default.
- Recommended default: A.
- Blocks: Phase 6 desktop UI default and Phase 7 live-demo policy.

⚠ OPERATOR DECISION REQUIRED — q5 default `sideEffectCap`
- Options: (A) default `REMOTE_DRAFT`; operator explicitly raises cap for `FORMAL_SUBMISSION`. (B) default `FORMAL_SUBMISSION` when validation passes. (C) make it a per-opportunity desktop choice with no global default.
- Recommended default: A.
- Blocks: Phase 6 desktop UI and Phase 7 live-demo policy.

⚠ OPERATOR DECISION REQUIRED — q6 SQL `bid_state` CHECK constraint
- Options: (A) add a new migration with a SQL CHECK constraint for the seven L6 states. (B) rely on Rust enum enforcement only and avoid schema churn.
- Recommended default: B.
- Blocks: Phase 1 schema work.

**TDD-decided implementation annotations (Tier 2):**
- q7 `BidDocument[]` source: TDD/strategy decides the exact source model; recommended shape is hybrid auto-pull from tender documents with operator override.
- q8 optional cron entries: TDD/strategy keeps L6 operator-triggered unless an explicit health-check requirement is added; no bid-submission cron is implied by this spec.
- q9 validator calibration depth: TDD defines calibration corpus size; recommended starting point is a light Procontract corpus with expansion based on live usage.
- q10 Stage-15 handoff file handling: archive step decides whether the old handoff file becomes a superseded historical note or an operational runbook; the §1.3 seven-state machine remains authoritative.

**Phase 7 live-demo prerequisite checklist (Tier 3):**
- q11 Operator selects a real Procontract opportunity reference before step 12 live-demo execution.
- q12 `shared_company_profile` is populated before the first live bid workflow run.
- q13 Certificate expiry warnings are not a readiness blocker unless TDD adds a warning requirement; expiry itself remains a hard gate.
- q14 CDP contention must be verified against existing daemon resource arbitration before live demo; otherwise the operator runs Procontract skills serially during the demo.

**Strategy-owned cosmetic item (Tier 4):**
- q15 Phase count is finalized by step 05 strategy; recommended default remains seven phases.

### 1.1 Layer Narrative

L6-FR-1.1.1 — Layer 6 automates request-pipeline preparation and submission on downstream portals
- AC1: Portal-specific bid skills automate form preparation, document upload, draft save, and expression-of-interest submission steps.
- AC2: Actual negotiations, bid pricing commitments, and fulfillment remain manual human responsibilities.

L6-FR-1.1.2 — Discovery source and submission portal are modeled separately
- AC1: Notice sources such as Find a Tender or Contracts Finder are not treated as bid-submission surfaces.
- AC2: Bid workflows target the downstream eSourcing portal, initially Procontract/Proactis.

L6-FR-1.1.3 — Automation is gated by trusted Layers 0-5 intelligence
- AC1: Layer 6 starts only after L5 has supplied decision-grade landed-cost intelligence and readiness blockers.
- AC2: If L6 is not built, the system remains complete for intelligence and operator decision-making.

L6-FR-1.1.4 — Every portal submission attempt produces a full audit trail
- AC1: Screenshots, raw terminal output, run manifests, and error manifests are captured for every bid run.
- AC2: Audit artifacts distinguish draft-saving from final external submission.

Portal-specific bid skills automate the **request pipeline** — preparation and submission steps for tender portal applications. This does NOT place real-money bids; actual negotiations and fulfillment are handled manually by humans. The system produces full audit trails (screenshots, logs, manifests) for every submission. The cost of pipeline failure is low — no real money changes hands automatically.

**Important distinction:** The notice **discovery source** (Find a Tender, Contracts Finder) and the **submission portal** are often different. Actual bidder interaction happens on downstream eSourcing portals such as Jaggaer, Atamis, Proactis, PCS-Tender, or authority-specific procurement systems. Bid workflows are modeled around the **underlying submission portal**, not the notice discovery source.

**Why this layer exists:** Layers 0-5 provide complete procurement intelligence with decision-grade landed cost. Layer 6 automates the most repetitive part of the downstream workflow — filling portal forms and submitting expressions of interest. This is the last layer because the intelligence stack must be trusted before automating submissions.

**If we never build this layer:** The system is fully functional for intelligence and decision-making. Operators manually prepare and submit portal applications based on system recommendations.

### 1.2 Capability Delivered

System can prepare and submit a request on one portal (Procontract/Proactis as the initial target) with full audit trail:

| ID | Capability | Acceptance criteria |
|----|------------|---------------------|
| L6-FR-1.2.1 | Agent navigates to downstream portal via browser automation | PTY/browser execution opens the target Procontract opportunity in the persistent CDP browser and verifies authenticated portal state before any mutation. |
| L6-FR-1.2.2 | Company profile data injected from `shared_company_profile` | Bid job input includes the resolved `CompanyProfile` at `companyProfileVersion`; missing or expired required profile documents block readiness. |
| L6-FR-1.2.3 | Portal forms filled using skill playbook | The bid skill follows the §1.7 playbook and records the step reached plus highest side-effect class. |
| L6-FR-1.2.4 | Documents uploaded to portal | `BidDocument[]` entries are content-hash checked before upload; upload failures generate an error manifest with step context. |
| L6-FR-1.2.5 | Submission captured with screenshots at each stage | Each major form step has a screenshot artifact; validator overlay checks screenshot completeness against the run shape. |
| L6-FR-1.2.6 | Full terminal output tracing for debugging | Raw PTY output is captured as a compressed forensic artifact linked to the run manifest. |
| L6-FR-1.2.7 | Manifest generated for every submission attempt | Success and draft-save paths produce run manifests with daemon evidence appendix and side-effect-class progression. |
| L6-FR-1.2.8 | Error manifests on failure with portal-specific context | Failure paths persist step, screenshot, portal state, error code, and next debugging action. |

### 1.3 Bid Workflow State Machine

The `bid_workflow_states` table (created in Layer 0, dormant until now) becomes fully active.

**State value requirements:**

| ID | State | Acceptance criteria |
|----|-------|---------------------|
| L6-FR-1.3.24 | `not_started` | Default state for a process with no approved, current, L5-ready bid workflow. |
| L6-FR-1.3.25 | `ready_to_bid` | Entered only when operator approval, freshness, L5 readiness, quote-role, company-profile, and upstream-trust gates pass. |
| L6-FR-1.3.26 | `bid_in_progress` | Entered only from an eligible start/resume action with a defined `BidScopePayload` and adapter-mediated PTY session. |
| L6-FR-1.3.27 | `draft_saved` | Indicates a remote draft was saved and operator review or final-submit action remains. |
| L6-FR-1.3.28 | `bid_submitted` | Indicates portal confirmation has been received or manually reconciled, with audit manifest and confirmation reference where available. |
| L6-FR-1.3.29 | `bid_failed` | Indicates an unrecoverable or operator-abandoned run with error manifest evidence. |
| L6-FR-1.3.30 | `portal_state_unknown` | Indicates possible remote mutation after `REMOTE_DRAFT` or higher, requiring human reconciliation before retry or closure. |

L6-FR-1.3.1 — Bid lifecycle has seven Rust-enforced values.
- AC1: The seven lifecycle values are `not_started`, `ready_to_bid`, `bid_in_progress`, `draft_saved`, `bid_submitted`, `bid_failed`, and `portal_state_unknown`.
- AC2: `migrations/0001_sqlite_state_initial.sql:437-451` defines `bid_state TEXT NOT NULL DEFAULT 'not_started'` with no SQL CHECK constraint; Rust state-machine validation enforces the seven values.

```text
not_started <--> ready_to_bid --> bid_in_progress --> draft_saved ---------> bid_submitted
                                  |   |              |  \-> bid_in_progress (resume)
                                  |   |              \---> bid_failed
                                  |   \-> bid_failed ---> ready_to_bid (retry)
                                  \-----> portal_state_unknown --> bid_in_progress (retry)
                                                               \-> bid_submitted (manual confirm)
                                                               \-> bid_failed
```

**State transitions:**

| ID | From | To | Trigger | Requirements |
|----|------|-----|---------|-------------|
| L6-FR-1.3.2 | `not_started` | `ready_to_bid` | Operator approves opportunity AND disposition is not stale AND L5 readiness gate passes | `operator_decision_heads.disposition = 'approved'` AND `reviewedBasisHash == current decisionBasisHash` AND no active L5 blockers (see the prerequisite check below) |
| L6-FR-1.3.3 | `ready_to_bid` | `bid_in_progress` | Operator clicks "Start Bid" in desktop | Prerequisites re-verified at this moment (not cached from `ready_to_bid` entry). Bid scope payload defined. Daemon creates adapter-mediated agent session. |
| L6-FR-1.3.4 | `ready_to_bid` | `not_started` | Decision becomes stale, L5 blocker activates, bid window expires, or operator withdraws | Daemon monitors prerequisite validity while in `ready_to_bid`; regression is automatic on staleness. Operator can also manually withdraw. |
| L6-FR-1.3.5 | `bid_in_progress` | `draft_saved` | Agent saves draft on portal (when `reviewRequired = true` OR side-effect class reached) | Draft confirmed saved on portal. Screenshots captured. Portal state verified. Operator reviews before final submission. |
| L6-FR-1.3.6 | `bid_in_progress` | `bid_submitted` | Agent completes portal submission successfully AND semantic validation clears | All form steps completed. Screenshots captured. Submission confirmation received. Run manifest generated. Validator verdict is `approved` or an explicit operator validation-gate override is logged. Daemon compares stored `decisionBasisHash` against current hash before final submit step; mismatch pauses to `draft_saved`. |
| L6-FR-1.3.7 | `bid_in_progress` | `bid_failed` | Agent fails during portal submission OR operator requests cancellation | Error manifest created with step context, screenshots at failure point, terminal output. If operator cancels and side-effect class >= `REMOTE_DRAFT`, transition to `portal_state_unknown` instead. |
| L6-FR-1.3.8 | `bid_in_progress` | `portal_state_unknown` | Agent crashes or disconnects after `REMOTE_DRAFT`+ side-effect class, OR session timeout, OR operator cancels after `REMOTE_DRAFT`+ | Portal state may have been mutated. Manual reconciliation required before retry. |
| L6-FR-1.3.9 | `draft_saved` | `bid_submitted` | Operator confirms and triggers final submission | Prerequisite re-check: `reviewedBasisHash == current decisionBasisHash`, no new L5 blockers, classification freshness still valid. Validator `approved` verdict or operator override of validation gate. |
| L6-FR-1.3.10 | `draft_saved` | `bid_in_progress` | Operator approves draft, agent resumes workflow from saved draft | For portals requiring additional steps after draft (not just final submit). Prerequisites re-verified. |
| L6-FR-1.3.11 | `draft_saved` | `bid_failed` | Operator abandons or final submission fails | Operator decides not to proceed, or final submission attempt fails. |
| L6-FR-1.3.12 | `portal_state_unknown` | `bid_in_progress` | Operator verifies portal state and authorizes retry | Human confirms portal state (no duplicate, draft recoverable). All prerequisites re-verified. |
| L6-FR-1.3.13 | `portal_state_unknown` | `bid_submitted` | Operator confirms submission actually completed on portal | Manual verification: operator provides portal confirmation reference. Handles crash-after-submit-but-before-record. |
| L6-FR-1.3.14 | `portal_state_unknown` | `bid_failed` | Operator determines portal state is unrecoverable | Manual review shows duplicate submission or irrecoverable state. |
| L6-FR-1.3.15 | `bid_failed` | `ready_to_bid` | Operator authorizes retry after reviewing error manifest | All prerequisites re-verified. Previous error manifest reviewed. Retry creates new workflow job linked to same process. |

**Prerequisite check (constitutional):** Bid workflow CANNOT start unless all five L5→L6 readiness blockers are clear:
1. Operator `approved` disposition on `operator_decision_heads` (not just `pending_operator_intents`)
2. Decision is NOT stale (`reviewedBasisHash == current decisionBasisHash`)
3. No active L5 blockers: `CLASSIFICATION_AMBIGUOUS`, `TARIFF_BRANCH_DIVERGENCE`, `SUPPLEMENTARY_UNIT_MISSING`, `decision_grade = broker_required`
4. Classification freshness still valid (`classificationFreshUntil` not expired)
5. Company profile documents and certificates current

Plus the two quality gates:
- Quote-role quality gate (`domestic_wholesale_floor` / `optimistic_floor` evidence requires explicit operator override)
- Upstream pipeline trust gate (`pipeline_assessments.validation_state = validated` for the scope)

**Prerequisite requirement mapping:**

- L6-FR-1.3.16 — Operator-approved disposition required.
  - AC1: `pending_operator_intents` alone never satisfies the gate.
  - AC2: `operator_decision_heads.disposition = 'approved'` is required.
- L6-FR-1.3.17 — Decision freshness required.
  - AC1: `reviewedBasisHash == current decisionBasisHash` is required.
  - AC2: Stale decisions block start until the operator re-affirms.
- L6-FR-1.3.18 — L5 hard blockers prevent bid readiness.
  - AC1: Active `CLASSIFICATION_AMBIGUOUS`, `TARIFF_BRANCH_DIVERGENCE`, or required-component `SUPPLEMENTARY_UNIT_MISSING` blocks `ready_to_bid`.
  - AC2: `decision_grade = broker_required` blocks `ready_to_bid`.
- L6-FR-1.3.19 — Classification freshness must cover the expected import window.
  - AC1: Expired `classificationFreshUntil` blocks `ready_to_bid`.
- L6-FR-1.3.20 — Quote-role quality gate is enforced.
  - AC1: `domestic_wholesale_floor`-only or `optimistic_floor`-only evidence blocks bid readiness unless the operator explicitly overrides with acknowledgment.
- L6-FR-1.3.21 — Company profile documents and certificates must be current.
  - AC1: Missing or expired required company-profile documents block bid readiness.
- L6-FR-1.3.22 — Upstream pipeline trust gate is enforced.
  - AC1: Selected lots/candidates cannot depend on `suspect` inputs.
  - AC2: Process rollup must surface `pipeline_assessments.validation_state = validated` unless the operator explicitly accepts flagged/unverified trust debt for the selected scope.

L6-FR-1.3.23 — Validation policy is fail-closed for portal submission.
- AC1: Validator rejection blocks transition to `bid_submitted`.
- AC2: Validator outage blocks transition to `bid_submitted`.
- AC3: Workflow may reach `draft_saved` without validation clearance.
- AC4: Operator may override the validation gate at final submission; the override is logged in bid state metadata and the run manifest.

**Validation policy — fail-closed for portal submission:** Bid automation skills use `validation_policy: always`. Unlike ordinary pipeline stages where fail-open is constitutional (Layer 3), portal submission is **fail-closed**: validator rejection or validator outage blocks transition to `bid_submitted`. This bid-run validation gate is separate from the upstream pipeline trust gate above. The workflow may reach `draft_saved` without validation clearance, but final submission requires either validator `approved` verdict or explicit operator override of the validation gate. This exception to fail-open is justified because portal submission creates externally visible mutations that cannot be undone.

### 1.4 Bid Workflow Scope

Every bid workflow carries an explicit scope payload — NOT whole-process blanket bids.

**Bid scope field requirements:**

| ID | Field / type | Acceptance criteria |
|----|--------------|---------------------|
| L6-FR-1.4.1 | `processId` | Required; identifies the process whose approved operator decision and pipeline rollup are being used. |
| L6-FR-1.4.2 | `lotIds[]` | Required; records exactly which lots are in bid scope. |
| L6-FR-1.4.3 | `candidateIds[]` | Required; records exactly which product candidates back the bid. |
| L6-FR-1.4.4 | `selectedQuoteIds[]` | Required; records exactly which supplier quotes informed pricing and qualification responses. |
| L6-FR-1.4.5 | `companyProfileVersion` | Required; pins the `shared_company_profile` version used for form filling and document validation. |
| L6-FR-1.4.6 | `bidPortalType` | Required; uses `procontract` for the initial Proactis/Procontract portal implementation. |
| L6-FR-1.4.7 | `portalOpportunityRef` | Required; deterministic target for a mutating workflow. Missing or ambiguous refs block start. |
| L6-FR-1.4.8 | `reviewRequired` | Required boolean; default remains an operator decision before step 07 (recommended: true). |
| L6-FR-1.4.9 | `decisionBasisHash` | Required SHA-256 over pipeline assessment, selected quotes, company profile version, document hashes, classification freshness, and L5 blocker state. |
| L6-FR-1.4.10 | `sideEffectCap` | Required maximum side-effect class for the run; default remains an operator decision before step 07 (recommended: `REMOTE_DRAFT`). |
| L6-FR-1.4.11 | `SideEffectClass` values | All six side-effect classes are supported: `READ_ONLY`, `REMOTE_DRAFT`, `NOTIFICATION_OR_LOCK`, `LEGAL_DECLARATION`, `FORMAL_SUBMISSION`, and `FUNDS_COMMITTED`. Retry/reconciliation rules match the inline comments in the type union below. |
| L6-FR-1.4.12 | Persistence | The full payload is persisted in `bid_workflow_states.bid_scope_json` and synced with bid state metadata only through the daemon-owned push path. |

**Decision basis hash L5 freshness vectors:** `decisionBasisHash` covers the process-level `pipeline_assessments` basis, selected supplier quote versions, `shared_company_profile` version, per-candidate `landed_cost_estimates.classification_fresh_until`, the `classification_fresh_until_expired()` result, and L5 tariff/measures/remedies/FX vector hashes. Any change makes `reviewedBasisHash != current decisionBasisHash` and blocks bid start until operator re-affirmation.

```typescript
interface BidScopePayload {
  processId: string
  lotIds: string[]                    // Which lots are included in this bid
  candidateIds: string[]              // Which product candidates back this bid
  selectedQuoteIds: string[]          // Which supplier quotes informed the pricing
  companyProfileVersion: string       // Version of shared_company_profile used
  bidPortalType: string               // e.g., 'procontract', 'jaggaer', 'atamis'
  portalOpportunityRef: string        // Portal-specific opportunity reference (REQUIRED — deterministic target for mutating workflow)
  reviewRequired: boolean             // Does the operator need to sign off before final submission?
  decisionBasisHash: string           // SHA-256 covering: pipeline assessment, selected quotes, company profile version, document hashes, classification freshness, L5 blocker state
  sideEffectCap: SideEffectClass      // Maximum side-effect class authorized for this workflow run
}

type SideEffectClass =
  | 'READ_ONLY'              // Login, navigate, verify tender — safe to retry
  | 'REMOTE_DRAFT'           // Open response, autosave, save draft, document upload — verify remote state before retry
  | 'NOTIFICATION_OR_LOCK'   // Buyer-visible status change, slot/response lock — human reconfirm before retry
  | 'LEGAL_DECLARATION'      // Accepting portal declarations/attestations — human reconfirm before retry
  | 'FORMAL_SUBMISSION'      // Final submit — never blind retry
  | 'FUNDS_COMMITTED'        // Deposits/fees (future portals) — never blind retry
```

The scope payload is persisted on `bid_workflow_states.bid_scope_json` (JSON column, existed as nullable from Layer 0).

**Side-effect default callout:** The manifest recommends `sideEffectCap = REMOTE_DRAFT` when `reviewRequired = true`, and `FORMAL_SUBMISSION` only when the operator explicitly toggles `reviewRequired = false` and confirms a known opportunity ref. Because q4/q5 are Tier 1 open questions, step 07 must capture operator confirmation before bead creation. The first live demo uses `sideEffectCap = REMOTE_DRAFT`.

### 1.5 Bid Workflow Flow

**Flow step requirements:**

| ID | Step | Acceptance criteria |
|----|------|---------------------|
| L6-FR-1.5.1 | Operator clicks "Start Bid" in desktop control panel | Desktop action is available only when readiness pre-check returns eligible or a structured override path is explicitly available. |
| L6-FR-1.5.2 | Desktop sends trigger command to daemon via DaemonBridge | Command carries `process_id` and explicit bid scope; mutation is not executed by the web portal. |
| L6-FR-1.5.3 | Daemon creates `workflow_jobs` entry with `job_kind = 'run_bid_workflow'` | Job is traceable through existing job/session viewer infrastructure. |
| L6-FR-1.5.4 | Daemon creates `bid_workflow_states` entry with status `bid_in_progress` | State write includes `bid_scope_json`, `bid_job_id`, owner, and stage/run metadata. |
| L6-FR-1.5.5 | Agent Adapter selects PTY mode | `pty_interactive` resolves to PTY execution because browser automation is required. |
| L6-FR-1.5.6 | Portal-specific skill loaded | Initial skill path is `skills/bid-portal-procontract/`; Proactis references are aliases only. |
| L6-FR-1.5.7 | Company profile injected | Profile version matches `BidScopePayload.companyProfileVersion`. |
| L6-FR-1.5.8 | Agent navigates to downstream eSourcing portal | Navigation targets the portal opportunity ref, not the notice discovery source. |
| L6-FR-1.5.9 | Agent authenticates on the portal | Auth follows the daemon-orchestrated `AUTH_STATE_STALE` retry pattern using existing `auth-signin/procontract`. |
| L6-FR-1.5.10 | Agent locates the correct opportunity/tender | Opportunity ref, title, and portal status are verified before form mutation. |
| L6-FR-1.5.11 | Agent fills portal forms using skill playbook | Company details, lot selection, pricing, and qualification responses are sourced from scope/profile/pipeline data. |
| L6-FR-1.5.12 | Agent uploads required documents | Upload uses content-hash-checked local artifact paths and records document type. |
| L6-FR-1.5.13 | Agent captures screenshot at each major form step | Screenshot paths are linked in run manifest and validator overlay checks completeness. |
| L6-FR-1.5.14 | Agent submits or saves draft | Branch respects `reviewRequired`, `sideEffectCap`, and validation gate status. |
| L6-FR-1.5.15 | Raw session output captured | PTY stream is stored as forensic artifact. |
| L6-FR-1.5.16 | Run manifest generated | Manifest includes agent narrative and daemon evidence appendix. |
| L6-FR-1.5.17 | Success updates `bid_workflow_states` to `bid_submitted` | Update happens only after final validation or logged operator validation override plus portal confirmation. |
| L6-FR-1.5.18 | Failure creates error manifest and sets `bid_failed` | Error context includes portal step, screenshot, terminal output, and side-effect class reached. |
| L6-FR-1.5.19 | Results sync to Supabase for web portal visibility | Sync uses existing push-owned `bid_workflow_states` propagation; web remains read-only. |

```text
1. Operator clicks "Start Bid" in desktop control panel
2. Desktop sends trigger command to daemon via DaemonBridge
3. Daemon creates workflow_jobs entry with job_kind = 'run_bid_workflow'
4. Daemon creates bid_workflow_states entry with status = 'bid_in_progress'
5. Agent Adapter selects PTY mode (browser automation required)
6. Portal-specific skill loaded (e.g., skills/bid-portal-procontract/)
7. Company profile injected into kickoff prompt from shared_company_profile
8. Agent navigates to downstream eSourcing portal via browser automation
9. Agent authenticates on the portal
10. Agent locates the correct opportunity/tender
11. Agent fills portal forms using skill playbook:
    - Company details from shared_company_profile
    - Lot selection from bidScopeJson.lotIds
    - Pricing/qualification responses from pipeline data
12. Agent uploads required documents (certificates, declarations, etc.)
13. Agent captures screenshot at each major form step
14. Agent submits or saves draft (depending on reviewRequired flag)
15. Full raw session output captured as forensic artifact
16. Run manifest generated (agent narrative + daemon evidence appendix)
17. On success: bid_workflow_states updated to 'bid_submitted'
18. On failure: error manifest created with portal-specific context, bid_workflow_states updated to 'bid_failed'
19. Results synced to Supabase for web portal visibility
```

### 1.6 `packages/bidding` — 3 Files + Validator Overlay

| ID | File / family | Purpose | Acceptance criteria |
|----|---------------|---------|---------------------|
| L6-FR-1.6.1 | `workflow-registry.ts` | Maps portal type string to workflow implementation. Extensible — new portal support = new workflow file + registry entry. | Registry maps `'procontract'` to `ProcontractWorkflow` and rejects unknown portal types with structured error context. |
| L6-FR-1.6.2 | `procontract-workflow.ts` | Procontract portal-specific workflow: auth-state detection, opportunity lookup, form field mapping, document upload sequences, draft save, submission confirmation detection | File name uses `procontract` per decision 1; Proactis is accepted only as alias text. |
| L6-FR-1.6.3 | `company-profile-schema.ts` | TypeScript schema for `shared_company_profile` data used in bid form filling: company name, registration number, address, contact details, certifications, insurance, financial statements references | Schema validates required profile fields and certificate/insurance expiry before bid readiness. |
| L6-FR-1.6.4 | `skills/validation/overlay-bid-portal-procontract/` | Domain validator overlay for bid-portal runs | Overlay is included in L6 deliverables and checks opportunity ref, screenshots, side-effect progression, side-effect cap, confirmation ref, lot/candidate citations, and PII redaction. |
| L6-FR-1.6.5 | Daemon bid-workflow module placement | Runner, state machine, readiness gate, and side-effect-cap enforcement live in the operator-confirmed location | Step 07 cannot create beads until q1 module placement is resolved; recommended default is `crates/daemon/src/bid_workflow/` skeleton in Phase 1. |
| L6-FR-1.6.6 | Bid workflow error-code extension | New bid failure modes are represented in `ErrorCode` only when existing variants are insufficient | Step 03 audits existing `ErrorCode` variants and updates enum body, `as_str()`, `parse_error_code()`, and all exhaustive matches for each new variant. |

```typescript
// workflow-registry.ts
interface BidWorkflow {
  portalType: string
  authenticate(session: AgentSession, profile: CompanyProfile): Promise<void>
  locateOpportunity(session: AgentSession, ref: string): Promise<void>
  verifyPortalState(session: AgentSession, ref: string): Promise<PortalStateVerification>
  fillForms(session: AgentSession, scope: BidScopePayload, data: PipelineData): Promise<void>
  uploadDocuments(session: AgentSession, documents: BidDocument[]): Promise<void>
  saveDraft(session: AgentSession): Promise<DraftSaveResult>
  submitFinal(session: AgentSession): Promise<SubmissionResult>
}

interface BidDocument {
  documentType: string               // e.g., 'certificate', 'declaration', 'financial_statement', 'insurance'
  fileName: string
  filePath: string                   // Path in local artifact store
  contentHash: string                // SHA-256 for integrity verification
  expiryDate?: string                // ISO-8601 — certificates/insurance must not be expired
  issuingBody?: string
}

interface SubmissionResult {
  submitted: boolean
  confirmationRef?: string           // Portal-generated reference number/ID
  finalSideEffectClass: SideEffectClass  // Actual highest side-effect reached
  screenshotPaths: string[]          // Screenshots at each step
  portalTimestamp?: string           // Portal-reported submission time
}

interface DraftSaveResult {
  draftSaved: boolean
  portalDraftRef?: string            // Portal reference for saved draft
  sideEffectClass: SideEffectClass   // Side-effect class reached during save
}

interface PortalStateVerification {
  opportunityFound: boolean
  existingDraftDetected: boolean     // Previous draft exists for this opportunity
  existingSubmissionDetected: boolean // Already submitted — prevents duplicate
  portalStatus: string               // Portal-reported opportunity status
}

const workflowRegistry: Map<string, BidWorkflow> = new Map([
  ['procontract', new ProcontractWorkflow()],
  // Future: ['jaggaer', new JaggaerWorkflow()], etc.
])
```

```typescript
// company-profile-schema.ts
interface CompanyProfile {
  companyName: string
  registrationNumber: string
  vatNumber?: string
  address: {
    line1: string; line2?: string; city: string;
    county?: string; postcode: string; country: string
  }
  primaryContact: {
    name: string; email: string; phone: string; role: string
  }
  certifications: Array<{
    type: string; number: string; expiryDate: string; issuingBody: string
  }>
  insurancePolicies: Array<{
    type: string; provider: string; policyNumber: string;
    coverAmountPence: number; expiryDate: string
  }>
  financialStatements?: {
    lastFiledYear: number; companiesHouseRef: string
  }
  smeStatus?: 'micro' | 'small' | 'medium' | 'large'
  equalityStatement?: string
  environmentalPolicy?: string
}
```

**Daemon placement callout:** The manifest recommends a Phase 1 module skeleton at `crates/daemon/src/bid_workflow/{mod.rs,types.rs,runner.rs,readiness_gate.rs,state_machine.rs}`, with `mod bid_workflow;` wired in `main.rs`; Phase 2 adds the `"run_bid_workflow"` dispatch arm and IPC handler; Phase 3 fills the runner/state/readiness logic. This remains a q1 operator decision before step 07.

**ErrorCode callout:** Current `ErrorCode` lives in `crates/daemon/src/errors/manifest.rs`: enum starts at line 20, `as_str()` at lines 65-115, and `parse_error_code()` around line 365. Lines 20-63 currently contain no `Bid*`/`BID_*` variants, so none of the six L6 strawman codes collide by name. Step 03 performs the redundancy audit before finalizing which variants are added.

| Candidate code | Distinct L6 semantic |
|----------------|----------------------|
| `BID_PORTAL_CHANGED` | Procontract page fingerprint or expected form shape changed in a bid-specific way; TDD may reuse `PAGE_SHAPE_DRIFT` if its context schema is sufficient. |
| `BID_SUBMISSION_FAILED` | Final submit step failed after form preparation, distinct from generic `AGENT_SKILL_FAILED` because side-effect class reached and confirmation absence must be explicit. |
| `BID_DUPLICATE_DETECTED` | `verifyPortalState()` found an existing submission for the same opportunity ref, preventing blind duplicate submission. |
| `BID_PORTAL_AUTH_EXPIRED` | Procontract session expired mid-workflow; TDD may reuse `AUTH_STATE_STALE` if daemon-orchestrated retry context is sufficient. |
| `BID_SIDE_EFFECT_CAP_EXCEEDED` | Skill attempted to escalate beyond `BidScopePayload.sideEffectCap`; safety stop before unauthorized mutation. |
| `BID_READINESS_GATE_FAILED` | One or more §1.3 prerequisite checks blocked a runtime start/resume attempt; error context names the failed gate(s). |

### 1.7 Bid Portal Skill — `skills/bid-portal-procontract/`

| ID | Attribute | Value | Acceptance criteria |
|----|-----------|-------|---------------------|
| L6-FR-1.7.1 | **Purpose** | Navigate Procontract eSourcing portal, fill bid forms, upload documents, save draft or submit expression of interest | Skill uses the same portal as Proactis references and never mutates unrelated account/profile surfaces. |
| L6-FR-1.7.2 | **Interaction profile** | `pty_interactive` — browser automation required | Adapter selects PTY mode; CDP preflight must pass before navigation. |
| L6-FR-1.7.3 | **Model profile** | `portal_form_high_reliability` — premium profile for externally visible portal mutations | Step 07 is blocked until q2 confirms this new profile or selects a reuse/alias option. |
| L6-FR-1.7.4 | **Validation policy** | `always` — every bid run gets semantic validation | Final `bid_submitted` transition is fail-closed unless validator approves or operator logs a validation-gate override. |
| L6-FR-1.7.5 | **Auth requirements** | Existing `skills/auth-signin/procontract/` via daemon-orchestrated retry | Bid skill returns `AUTH_STATE_STALE` on expired session and does not invoke auth-signin directly. |
| L6-FR-1.7.6 | **Resume strategy** | Graded by side-effect class: `READ_ONLY` steps are safe to retry; `REMOTE_DRAFT` steps require `verifyPortalState()` before retry; `FORMAL_SUBMISSION` requires human verification | Resume path must verify portal state before retrying any prior remote mutation. |

**Auth delegation callout:** `bid-portal-procontract` delegates portal login to the existing `skills/auth-signin/procontract/` through daemon-orchestrated retry. The bid skill attaches to the persistent CDP browser via `bin/ensure-cdp-9222`; if URL contains `/Login` or text matches `Session expired|Please log in`, it returns `AUTH_STATE_STALE` and exits. The daemon runs `auth-signin/procontract` as a sub-job and reruns the original bid job after successful sign-in. The bid skill must not call auth-signin directly because that bypasses daemon lifecycle bookkeeping.

**Skill playbook steps:**

| ID | Step | Action | Screenshot | Side-Effect Class |
|----|------|--------|------------|-------------------|
| L6-FR-1.7.7 | 1 | Navigate to Procontract portal login page | Yes | `READ_ONLY` |
| L6-FR-1.7.8 | 2 | Verify authenticated state or return `AUTH_STATE_STALE` for daemon retry | Yes | `READ_ONLY` |
| L6-FR-1.7.9 | 3 | Search/navigate to the target opportunity | Yes | `READ_ONLY` |
| L6-FR-1.7.10 | 4 | Verify opportunity matches expected tender reference + check for existing draft/submission | Yes | `READ_ONLY` |
| L6-FR-1.7.11 | 5 | Open bid response form | Yes | `READ_ONLY` |
| L6-FR-1.7.12 | 6 | Fill company details section from `shared_company_profile` | Yes | `REMOTE_DRAFT` (portal auto-save may commit) |
| L6-FR-1.7.13 | 7 | Fill lot selection and pricing sections from pipeline data | Yes | `REMOTE_DRAFT` |
| L6-FR-1.7.14 | 8 | Upload required documents (certificates, declarations) | Yes | `REMOTE_DRAFT` |
| L6-FR-1.7.15 | 9 | Review submission summary | Yes | `READ_ONLY` |
| L6-FR-1.7.16 | 10a | Save draft (if `reviewRequired = true` OR `sideEffectCap` reached) | Yes | `REMOTE_DRAFT` → state transitions to `draft_saved` |
| L6-FR-1.7.17 | 10b | Submit expression of interest (if `reviewRequired = false` AND validation cleared) | Yes | `FORMAL_SUBMISSION` → state transitions to `bid_submitted` |
| L6-FR-1.7.18 | 11 | Capture confirmation page/reference | Yes | `READ_ONLY` |

**Side-effect class rules:**
- L6-FR-1.7.19 — Each step declares its side-effect class.
  - AC1: The skill tracks the highest class reached and includes it in result/checkpoint/manifest output.
- L6-FR-1.7.20 — `sideEffectCap` is enforced at runtime.
  - AC1: If the workflow's `sideEffectCap` (from `BidScopePayload`) is exceeded, the skill saves draft and stops.
  - AC2: Attempted escalation past the cap emits the selected bid side-effect error code from step 03.
- **Crash recovery by class:**
  - `READ_ONLY`: Safe to retry from scratch.
  - `REMOTE_DRAFT`: Must `verifyPortalState()` before retry. If draft exists, resume from draft. If not, retry from scratch.
  - `NOTIFICATION_OR_LOCK` / `LEGAL_DECLARATION`: Human must reconfirm. State transitions to `portal_state_unknown`.
  - `FORMAL_SUBMISSION`: Never blind retry. State transitions to `portal_state_unknown`. Human verifies.

### 1.8 Audit Trail (Constitutional)

Every bid submission produces a complete, traceable audit trail:

| ID | Artifact | Storage | Purpose / acceptance criteria |
|----|----------|---------|-------------------------------|
| L6-FR-1.8.1 | Screenshots (per step) | Artifact store | Visual proof of portal state at each form step; missing screenshots fail validator overlay unless the run shape explicitly skips the step. |
| L6-FR-1.8.2 | Raw PTY terminal output | Artifact store (compressed) | Full forensic record of agent actions; linked from run manifest. |
| L6-FR-1.8.3 | Run manifest | Artifact store | Agent narrative + daemon evidence appendix, including bid scope and side-effect progression. |
| L6-FR-1.8.4 | Bid workflow state | `bid_workflow_states` table | Persistent status record with scope payload and job/stage metadata. |
| L6-FR-1.8.5 | Error manifest (on failure) | `error_manifests` table + artifact store | Portal-specific failure context for debugging, including step, screenshot, form state, side-effect class reached, and next action. |
| L6-FR-1.8.6 | Submission confirmation ref | `bid_workflow_states` metadata | Portal-generated reference number/ID or operator-provided manual confirmation when reconciling `portal_state_unknown`. |

### 1.9 Data Changes

**No new tables. No new columns.** `bid_workflow_states` was created in Layer 0 with all columns present.

**Activated table:**

| ID | Table | Authority | Change / acceptance criteria |
|----|-------|-----------|------------------------------|
| L6-FR-1.9.1 | `bid_workflow_states` | Push-owned | Now fully active: lifecycle states `not_started` → `ready_to_bid` → `bid_in_progress` → `draft_saved` → `bid_submitted` / `bid_failed` / `portal_state_unknown`. `bid_scope_json` populated with explicit lot/candidate scope including `sideEffectCap` and expanded `decisionBasisHash`. Synced to Supabase for web visibility. |
| L6-FR-1.9.2 | SQL `bid_state` constraint | Rust-enforced | L6 does not add a SQL CHECK constraint unless q6 is resolved differently before step 07; Rust state-machine validation enforces the seven lifecycle states. |
| L6-FR-1.9.3 | Web visibility | Read-only Supabase projection | Web viewer shows a read-only bid-state badge and review-required/error links, with no mutation surface and no exposure of sensitive `BidScopePayload` JSON or manifest content to viewer-role users. |

**Sync callout:** `crates/daemon/src/db/sync.rs:24-50` already includes `bid_workflow_states` in `PUSH_OWNED_RESEED_TABLES` at line 39 (zero-based index 14). L6 requires no sync-engine table-list change for bid state propagation; daemon mutations to `bid_workflow_states` already flow through the L4 push-owned sync engine.

### 1.10 Invariants Enforced (Constitutional)

| ID | Invariant | Enforcement |
|----|-----------|-------------|
| L6-FR-1.10.1 | No real-money bids without human approval | Bid workflow requires operator `approved` disposition on `operator_decision_heads` |
| L6-FR-1.10.2 | No externally visible portal mutation beyond approved side-effect class | `sideEffectCap` on `BidScopePayload` limits maximum side-effect class; exceeded cap → save draft and stop |
| L6-FR-1.10.3 | Bid workflow carries explicit scope | `bid_scope_json` payload with lot/candidate IDs — no blanket bids |
| L6-FR-1.10.4 | Full audit trail for submissions | Screenshots, terminal output, run manifest captured for every step including side-effect class reached |
| L6-FR-1.10.5 | Stale decisions block bid | `reviewedBasisHash != decisionBasisHash` → bid blocked until re-affirm |
| L6-FR-1.10.6 | L5 readiness gate | Active L5 blockers (`CLASSIFICATION_AMBIGUOUS`, `TARIFF_BRANCH_DIVERGENCE`, `SUPPLEMENTARY_UNIT_MISSING`, `broker_required`) block `ready_to_bid`; see `docs/strategy/layer-strategy.md` §10 Interface Contracts and `docs/strategy/cross-reference-index.md` rows 230 and 232 |
| L6-FR-1.10.7 | Validation fail-closed for portal submission | Validator rejection or outage blocks `bid_submitted`; `draft_saved` permitted without validation clearance |
| L6-FR-1.10.8 | Quote-role quality gate | `domestic_wholesale_floor`-only or `optimistic_floor`-only evidence requires explicit operator override for bid readiness |

---

## 2. Testing, DoD, Risks & Dependencies

### 2.1 Definition of Done

| ID | # | Criterion |
|----|---|----------|
| L6-FR-2.1.1 | 1 | Bid-portal skill navigates target portal (Procontract/Proactis) via browser automation |
| L6-FR-2.1.2 | 2 | Company profile data injected from `shared_company_profile` |
| L6-FR-2.1.3 | 3 | Request preparation steps automated (form filling, document upload) with side-effect class tracking |
| L6-FR-2.1.4 | 4 | Submission step automated with screenshot capture at each stage |
| L6-FR-2.1.5 | 5 | Full terminal output tracing for bid workflow |
| L6-FR-2.1.6 | 6 | Run manifest generated for submitted request with audit trail |
| L6-FR-2.1.7 | 7 | Bid failure produces error manifest with portal-specific context (step, screenshot, form state, side-effect class reached) |
| L6-FR-2.1.8 | 8 | System can prepare and submit one request on Procontract end-to-end |
| L6-FR-2.1.9 | 9 | `draft_saved` path works when `reviewRequired = true` (skill saves draft, operator reviews, then triggers final submission) |
| L6-FR-2.1.10 | 10 | L5 readiness gate blocks `ready_to_bid` when active L5 blockers present |
| L6-FR-2.1.11 | 11 | Validator rejection or outage blocks `bid_submitted` (fail-closed for portal submission) |

**DoD coverage map:**

| DoD | Satisfied by FRs |
|-----|------------------|
| 1 | L6-FR-1.2.1, L6-FR-1.5.5, L6-FR-1.5.8, L6-FR-1.7.2, L6-FR-1.7.7 |
| 2 | L6-FR-1.2.2, L6-FR-1.4.5, L6-FR-1.5.7, L6-FR-1.6.3 |
| 3 | L6-FR-1.2.3, L6-FR-1.2.4, L6-FR-1.5.11, L6-FR-1.5.12, L6-FR-1.7.12, L6-FR-1.7.13, L6-FR-1.7.14, L6-FR-1.7.19 |
| 4 | L6-FR-1.2.5, L6-FR-1.5.13, L6-FR-1.7.17, L6-FR-1.8.1 |
| 5 | L6-FR-1.2.6, L6-FR-1.5.15, L6-FR-1.8.2 |
| 6 | L6-FR-1.2.7, L6-FR-1.5.16, L6-FR-1.8.3 |
| 7 | L6-FR-1.2.8, L6-FR-1.5.18, L6-FR-1.8.5, L6-FR-1.6.6 |
| 8 | L6-FR-1.1.1, L6-FR-1.2.1, L6-FR-1.2.8, L6-FR-1.5.1 through L6-FR-1.5.19 |
| 9 | L6-FR-1.3.5, L6-FR-1.3.9, L6-FR-1.3.27, L6-FR-1.3.28, L6-FR-1.5.14, L6-FR-1.7.16 |
| 10 | L6-FR-1.3.18, L6-FR-1.3.19, L6-FR-1.3.25, L6-FR-1.10.6 |
| 11 | L6-FR-1.3.23, L6-FR-1.7.4, L6-FR-1.10.7 |

**Local quality gate callout:** This repo currently has no `.github/workflows/` directory. L6 validation uses the local quality gate sequence from the pipeline manifest (`cargo check --all-targets`, full daemon stage-validation tests when stage manifests are involved, `bun run build`, `bun run test`, E2E where relevant, clippy/fmt, and scoped `ubs`). Adding CI is not silently in scope.

### 2.2 Test Files (3 New, Cumulative 60)

| File | Coverage |
|------|---------|
| `bid-workflow-states.test.ts` | State machine transitions including `draft_saved`, `portal_state_unknown`; side-effect class tracking; L5 readiness gate; staleness checks; `decisionBasisHash` completeness |
| `portal-side-effects.test.ts` | Side-effect class assignment per portal step; crash-after-`REMOTE_DRAFT` recovery with `verifyPortalState()`; duplicate-submission detection; `sideEffectCap` enforcement |
| `bid-readiness-gate.test.ts` | L5 blocker checks (`CLASSIFICATION_AMBIGUOUS`, `TARIFF_BRANCH_DIVERGENCE`, `SUPPLEMENTARY_UNIT_MISSING`, `broker_required`); quote-role quality gate; classification freshness for import window; company profile document expiry |

### 2.3 Test Scenarios

| Scenario | Expected Behavior |
|----------|------------------|
| Full bid workflow: approve → prepare → submit → manifest | Complete lifecycle: `not_started` → `ready_to_bid` → `bid_in_progress` → `bid_submitted`. Screenshots at each step. Manifest generated. Side-effect class tracked per step. |
| `reviewRequired = true`: save draft path | Skill saves draft at step 10a. State transitions to `draft_saved`. Operator reviews in desktop. Triggers final submission → `bid_submitted`. |
| Bid failure mid-workflow (step 8: document upload fails) | Error manifest with screenshots at failure point. `bid_failed` state. Side-effect class `REMOTE_DRAFT` recorded. Operator can retry after `verifyPortalState()`. |
| Crash after `REMOTE_DRAFT` (step 7) | State transitions to `portal_state_unknown`. Human must verify portal state before retry. `verifyPortalState()` checks for existing draft. |
| Stale decision blocks bid | Operator decision marked stale (`reviewedBasisHash` mismatch) → bid workflow blocked. Cannot proceed until operator re-affirms with updated basis. |
| L5 blocker blocks bid | `CLASSIFICATION_AMBIGUOUS` active on selected candidate → `ready_to_bid` transition blocked. Operator cannot start bid until classification resolved. |
| Validator rejection before final submit | Validator rejects run manifest → `bid_submitted` blocked. Workflow stays at `draft_saved` or `bid_in_progress`. Operator can override validation gate. |
| Validator outage before final submit | Validator unavailable → `bid_submitted` blocked (fail-closed). Workflow can reach `draft_saved`. Operator must wait for validator or override. |
| Portal website changed (form structure different) | Skill fails with `AGENT_SKILL_FAILED`. Error manifest flags the specific step where portal structure diverged. Page fingerprint drift detected. |
| Duplicate submission detected | `verifyPortalState()` detects existing submission → `portal_state_unknown`. Operator reviews. |
| Quote-role quality gate | Opportunity backed only by `domestic_wholesale_floor` quotes → `ready_to_bid` blocked unless operator explicitly overrides. |

### 2.4 Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Portal UI changes break bid skill | High | Low | Not real money — error manifest flags failures; skill update cycle; page fingerprint drift detection |
| Form structure varies by procurement type | Medium | Medium | Portal-specific workflow handles variance; fallback to verify-only mode |
| Duplicate submission on retry after side-effect boundary | Medium | Medium | Side-effect class tracking; `verifyPortalState()` before retry after `REMOTE_DRAFT`; human confirmation required after `NOTIFICATION_OR_LOCK` or higher |
| Portal credentials expire during bid | Low | Low | Auth challenge detection; `auth-refresh` skill for portal (future) |

### 2.5 Forward-Compatibility Provisions

| Provision | What Exists in L6 | Future Extension |
|-----------|-------------------|-----------------|
| Workflow registry pattern | Maps portal type to workflow implementation | Add new portal support = new workflow file + registry entry |
| Company profile schema | Structured profile for Procontract/Proactis | Extensible for additional portal-specific fields |
| `bid_scope_json` | Explicit lot/candidate scope payload | Supports multi-lot bids, partial bids, amended bids |
| PDF extraction toolkit extension | L1 toolkit supports future `pdf-extraction/bid-template` overlay skill for extracting portal-specific bid form structure. Note: document **generation** (letters of intent, cover letters) is a separate module, not an extraction overlay. | Add new overlay skill without schema/daemon changes |

### 2.6 Dependencies

- **Layer 0-5:** Full pipeline, desktop, web, sync, decision-grade intelligence
- **Specifically required:**
  - `operator_decision_heads` (Layer 4) — operator approval prerequisite
  - `shared_company_profile` (Layer 4) — company data for form filling
  - `bid_workflow_states` (Layer 0 schema, Layer 6 activation) — bid lifecycle
  - Semantic validation system (Layer 3-4) — `validation_policy: always` for bid skills

**Dependency acceptance criteria:**
- `operator_decision_heads` is the sole source of constitutional operator approval.
  - AC1: Bid workflow never proceeds from pending intents alone.
- `shared_company_profile` is the bidder profile authority.
  - AC1: Company profile version is pinned in `BidScopePayload`.
- `bid_workflow_states` is activated without adding new tables or columns.
  - AC1: Existing L0 schema and L4 sync push set carry bid lifecycle state.
- Semantic validation remains the bid-run validation authority.
  - AC1: `validation_policy: always` plus the bid overlay enforces fail-closed final submission.

### 2.7 ADRs Addressed

| ADR | Decision | Layer 6 Action |
|-----|----------|---------------|
| **ADR-007** | Request pipeline automation (not real-money bids) | Portal-specific bid skill with full audit trail; operator approval required |

### 2.8 What's NOT Stubbed

Nothing — this is the final layer. No features are deferred beyond this point.

---

<!-- CROSS-DOMAIN NOTE FOR L0: `bid_workflow_states` DDL bid_state CHECK constraint should be expanded to include `draft_saved` and `portal_state_unknown` values added by L6. The current DDL allows free-text (no CHECK), so this is a recommendation, not a blocker. -->

<!-- CROSS-DOMAIN NOTE FOR L4: `operator_decision_heads` staleness detection must include L5 freshness vectors (tariff structure, measures, remedies, FX) in addition to pipeline assessment hash. When any L5 freshness vector expires for classifications backing an approved decision, the decision should be marked stale. -->
