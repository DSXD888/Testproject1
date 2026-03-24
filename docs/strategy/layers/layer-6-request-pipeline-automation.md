# Layer 6 — Request Pipeline Automation

> **Authority:** This document is the execution-grade specification for Layer 6. An implementation agent should be able to build this layer from this document alone without reopening the research document.
>
> **Milestone:** M6
> **Dependencies:** Layers 0-5 (full pipeline, desktop, web, sync, decision-grade intelligence)
> **Exit criteria:** System can prepare and submit one request on a target portal end-to-end with full audit trail.

---

## 1. Overview & Bid Workflow System

### 1.1 Layer Narrative

Portal-specific bid skills automate the **request pipeline** — preparation and submission steps for tender portal applications. This does NOT place real-money bids; actual negotiations and fulfillment are handled manually by humans. The system produces full audit trails (screenshots, logs, manifests) for every submission. The cost of pipeline failure is low — no real money changes hands automatically.

**Important distinction:** The notice **discovery source** (Find a Tender, Contracts Finder) and the **submission portal** are often different. Actual bidder interaction happens on downstream eSourcing portals such as Jaggaer, Atamis, Proactis, PCS-Tender, or authority-specific procurement systems. Bid workflows are modeled around the **underlying submission portal**, not the notice discovery source.

**Why this layer exists:** Layers 0-5 provide complete procurement intelligence with decision-grade landed cost. Layer 6 automates the most repetitive part of the downstream workflow — filling portal forms and submitting expressions of interest. This is the last layer because the intelligence stack must be trusted before automating submissions.

**If we never build this layer:** The system is fully functional for intelligence and decision-making. Operators manually prepare and submit portal applications based on system recommendations.

### 1.2 Capability Delivered

System can prepare and submit a request on one portal (Proactis as the initial target) with full audit trail:

- Agent navigates to downstream portal via browser automation
- Company profile data injected from `shared_company_profile`
- Portal forms filled using skill playbook
- Documents uploaded to portal
- Submission captured with screenshots at each stage
- Full terminal output tracing for debugging
- Manifest generated for every submission
- Error manifests on failure with portal-specific context

### 1.3 Bid Workflow State Machine

The `bid_workflow_states` table (created in Layer 0, dormant until now) becomes fully active.

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

| From | To | Trigger | Requirements |
|------|-----|---------|-------------|
| `not_started` | `ready_to_bid` | Operator approves opportunity AND disposition is not stale AND L5 readiness gate passes | `operator_decision_heads.disposition = 'approved'` AND `reviewedBasisHash == current decisionBasisHash` AND no active L5 blockers (see §1.11) |
| `ready_to_bid` | `bid_in_progress` | Operator clicks "Start Bid" in desktop | Prerequisites re-verified at this moment (not cached from `ready_to_bid` entry). Bid scope payload defined. Daemon creates adapter-mediated agent session. |
| `ready_to_bid` | `not_started` | Decision becomes stale, L5 blocker activates, bid window expires, or operator withdraws | Daemon monitors prerequisite validity while in `ready_to_bid`; regression is automatic on staleness. Operator can also manually withdraw. |
| `bid_in_progress` | `draft_saved` | Agent saves draft on portal (when `reviewRequired = true` OR side-effect class reached) | Draft confirmed saved on portal. Screenshots captured. Portal state verified. Operator reviews before final submission. |
| `bid_in_progress` | `bid_submitted` | Agent completes portal submission successfully AND semantic validation clears | All form steps completed. Screenshots captured. Submission confirmation received. Run manifest generated. Validator verdict is `approved` or `validation_policy: skip`. Daemon compares stored `decisionBasisHash` against current hash before final submit step; mismatch pauses to `draft_saved`. |
| `bid_in_progress` | `bid_failed` | Agent fails during portal submission OR operator requests cancellation | Error manifest created with step context, screenshots at failure point, terminal output. If operator cancels and side-effect class >= `REMOTE_DRAFT`, transition to `portal_state_unknown` instead. |
| `bid_in_progress` | `portal_state_unknown` | Agent crashes or disconnects after `REMOTE_DRAFT`+ side-effect class, OR session timeout, OR operator cancels after `REMOTE_DRAFT`+ | Portal state may have been mutated. Manual reconciliation required before retry. |
| `draft_saved` | `bid_submitted` | Operator confirms and triggers final submission | Prerequisite re-check: `reviewedBasisHash == current decisionBasisHash`, no new L5 blockers, classification freshness still valid. Validator `approved` verdict or operator override of validation gate. |
| `draft_saved` | `bid_in_progress` | Operator approves draft, agent resumes workflow from saved draft | For portals requiring additional steps after draft (not just final submit). Prerequisites re-verified. |
| `draft_saved` | `bid_failed` | Operator abandons or final submission fails | Operator decides not to proceed, or final submission attempt fails. |
| `portal_state_unknown` | `bid_in_progress` | Operator verifies portal state and authorizes retry | Human confirms portal state (no duplicate, draft recoverable). All prerequisites re-verified. |
| `portal_state_unknown` | `bid_submitted` | Operator confirms submission actually completed on portal | Manual verification: operator provides portal confirmation reference. Handles crash-after-submit-but-before-record. |
| `portal_state_unknown` | `bid_failed` | Operator determines portal state is unrecoverable | Manual review shows duplicate submission or irrecoverable state. |
| `bid_failed` | `ready_to_bid` | Operator authorizes retry after reviewing error manifest | All prerequisites re-verified. Previous error manifest reviewed. Retry creates new workflow job linked to same process. |

**Prerequisite check (constitutional):** Bid workflow CANNOT start unless:
1. Operator `approved` disposition exists on `operator_decision_heads` (not just `pending_operator_intents`)
2. The decision is NOT stale (`reviewedBasisHash == current decisionBasisHash`)
3. If stale, operator must re-affirm before bid can proceed
4. No active L5 blockers: `CLASSIFICATION_AMBIGUOUS`, `TARIFF_BRANCH_DIVERGENCE`, `SUPPLEMENTARY_UNIT_MISSING` on required duty components, `decision_grade = broker_required`
5. Classification freshness still valid for the expected import window (`classificationFreshUntil` not expired)
6. Quote-role quality gate: opportunities backed only by `domestic_wholesale_floor` or `optimistic_floor` evidence are not bid-ready unless operator explicitly overrides with acknowledgment
7. Company profile documents and certificates current (not expired)
8. Upstream pipeline trust gate satisfied for the selected bid scope: no selected lot/candidate depends on `suspect` inputs, and the process-level rollup surfaces `pipeline_assessments.validation_state = validated` unless the operator explicitly accepts flagged/unverified trust debt for that scope

**Validation policy — fail-closed for portal submission:** Bid automation skills use `validation_policy: always`. Unlike ordinary pipeline stages where fail-open is constitutional (Layer 3), portal submission is **fail-closed**: validator rejection or validator outage blocks transition to `bid_submitted`. This bid-run validation gate is separate from the upstream pipeline trust gate above. The workflow may reach `draft_saved` without validation clearance, but final submission requires either validator `approved` verdict or explicit operator override of the validation gate. This exception to fail-open is justified because portal submission creates externally visible mutations that cannot be undone.

### 1.4 Bid Workflow Scope

Every bid workflow carries an explicit scope payload — NOT whole-process blanket bids.

```typescript
interface BidScopePayload {
  processId: string
  lotIds: string[]                    // Which lots are included in this bid
  candidateIds: string[]              // Which product candidates back this bid
  selectedQuoteIds: string[]          // Which supplier quotes informed the pricing
  companyProfileVersion: string       // Version of shared_company_profile used
  bidPortalType: string               // e.g., 'proactis', 'jaggaer', 'atamis'
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

The scope payload is persisted on `bid_workflow_states.bidScopeJson` (JSON column, existed as nullable from Layer 0).

### 1.5 Bid Workflow Flow

```text
1. Operator clicks "Start Bid" in desktop control panel
2. Desktop sends trigger command to daemon via DaemonBridge
3. Daemon creates workflow_jobs entry with job_kind = 'run_bid_workflow'
4. Daemon creates bid_workflow_states entry with status = 'bid_in_progress'
5. Agent Adapter selects PTY mode (browser automation required)
6. Portal-specific skill loaded (e.g., skills/bid-portal-proactis/)
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

### 1.6 `packages/bidding` — 3 Files

| File | Purpose |
|------|---------|
| `workflow-registry.ts` | Maps portal type string to workflow implementation. Extensible — new portal support = new workflow file + registry entry. |
| `proactis-workflow.ts` | Proactis portal-specific workflow: authentication, opportunity lookup, form field mapping, document upload sequences, submission confirmation detection |
| `company-profile-schema.ts` | TypeScript schema for `shared_company_profile` data used in bid form filling: company name, registration number, address, contact details, certifications, insurance, financial statements references |

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
  ['proactis', new ProactisWorkflow()],
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

### 1.7 Bid Portal Skill — `skills/bid-portal-proactis/`

| Attribute | Value |
|-----------|-------|
| **Purpose** | Navigate Proactis eSourcing portal, fill bid forms, upload documents, submit expression of interest |
| **Interaction profile** | `pty_interactive` — browser automation required |
| **Model profile** | `portal_form_high_reliability` — premium profile for externally visible portal mutations |
| **Validation policy** | `always` — every bid run gets semantic validation |
| **Auth requirements** | Portal credentials (stored in session-isolated config) |
| **Resume strategy** | Graded by side-effect class: `READ_ONLY` steps are safe to retry; `REMOTE_DRAFT` steps require `verifyPortalState()` before retry; `FORMAL_SUBMISSION` requires human verification |

**Skill playbook steps:**

| Step | Action | Screenshot | Side-Effect Class |
|------|--------|-----------|-------------------|
| 1 | Navigate to Proactis portal login page | Yes | `READ_ONLY` |
| 2 | Authenticate with portal credentials | Yes | `READ_ONLY` |
| 3 | Search/navigate to the target opportunity | Yes | `READ_ONLY` |
| 4 | Verify opportunity matches expected tender reference + check for existing draft/submission | Yes | `READ_ONLY` |
| 5 | Open bid response form | Yes | `READ_ONLY` |
| 6 | Fill company details section from `shared_company_profile` | Yes | `REMOTE_DRAFT` (portal auto-save may commit) |
| 7 | Fill lot selection and pricing sections from pipeline data | Yes | `REMOTE_DRAFT` |
| 8 | Upload required documents (certificates, declarations) | Yes | `REMOTE_DRAFT` |
| 9 | Review submission summary | Yes | `READ_ONLY` |
| 10a | Save draft (if `reviewRequired = true` OR `sideEffectCap` reached) | Yes | `REMOTE_DRAFT` → state transitions to `draft_saved` |
| 10b | Submit expression of interest (if `reviewRequired = false` AND validation cleared) | Yes | `FORMAL_SUBMISSION` → state transitions to `bid_submitted` |
| 11 | Capture confirmation page/reference | Yes | `READ_ONLY` |

**Side-effect class rules:**
- Each step declares its side-effect class. The skill tracks the highest class reached.
- If the workflow's `sideEffectCap` (from `BidScopePayload`) is exceeded, the skill saves draft and stops.
- **Crash recovery by class:**
  - `READ_ONLY`: Safe to retry from scratch.
  - `REMOTE_DRAFT`: Must `verifyPortalState()` before retry. If draft exists, resume from draft. If not, retry from scratch.
  - `NOTIFICATION_OR_LOCK` / `LEGAL_DECLARATION`: Human must reconfirm. State transitions to `portal_state_unknown`.
  - `FORMAL_SUBMISSION`: Never blind retry. State transitions to `portal_state_unknown`. Human verifies.

### 1.8 Audit Trail (Constitutional)

Every bid submission produces a complete, traceable audit trail:

| Artifact | Storage | Purpose |
|----------|---------|---------|
| Screenshots (per step) | Artifact store | Visual proof of portal state at each form step |
| Raw PTY terminal output | Artifact store (compressed) | Full forensic record of agent actions |
| Run manifest | Artifact store | Agent narrative + daemon evidence appendix |
| Bid workflow state | `bid_workflow_states` table | Persistent status record with scope payload |
| Error manifest (on failure) | `error_manifests` table + artifact store | Portal-specific failure context for debugging |
| Submission confirmation ref | `bid_workflow_states` metadata | Portal-generated reference number/ID |

### 1.9 Data Changes

**No new tables. No new columns.** `bid_workflow_states` was created in Layer 0 with all columns present.

**Activated table:**

| Table | Authority | Change |
|-------|-----------|--------|
| `bid_workflow_states` | Push-owned | Now fully active: lifecycle states `not_started` → `ready_to_bid` → `bid_in_progress` → `draft_saved` → `bid_submitted` / `bid_failed` / `portal_state_unknown`. `bidScopeJson` populated with explicit lot/candidate scope including `sideEffectCap` and expanded `decisionBasisHash`. Synced to Supabase for web visibility. |

### 1.10 Invariants Enforced (Constitutional)

| Invariant | Enforcement |
|-----------|-------------|
| No real-money bids without human approval | Bid workflow requires operator `approved` disposition on `operator_decision_heads` |
| No externally visible portal mutation beyond approved side-effect class | `sideEffectCap` on `BidScopePayload` limits maximum side-effect class; exceeded cap → save draft and stop |
| Bid workflow carries explicit scope | `bidScopeJson` payload with lot/candidate IDs — no blanket bids |
| Full audit trail for submissions | Screenshots, terminal output, run manifest captured for every step including side-effect class reached |
| Stale decisions block bid | `reviewedBasisHash != decisionBasisHash` → bid blocked until re-affirm |
| L5 readiness gate | Active L5 blockers (`CLASSIFICATION_AMBIGUOUS`, `TARIFF_BRANCH_DIVERGENCE`, `SUPPLEMENTARY_UNIT_MISSING`, `broker_required`) block `ready_to_bid` |
| Validation fail-closed for portal submission | Validator rejection or outage blocks `bid_submitted`; `draft_saved` permitted without validation clearance |
| Quote-role quality gate | `domestic_wholesale_floor`-only or `optimistic_floor`-only evidence requires explicit operator override for bid readiness |

---

## 2. Testing, DoD, Risks & Dependencies

### 2.1 Definition of Done

| # | Criterion |
|---|----------|
| 1 | Bid-portal skill navigates target portal (Proactis) via browser automation |
| 2 | Company profile data injected from `shared_company_profile` |
| 3 | Request preparation steps automated (form filling, document upload) with side-effect class tracking |
| 4 | Submission step automated with screenshot capture at each stage |
| 5 | Full terminal output tracing for bid workflow |
| 6 | Run manifest generated for submitted request with audit trail |
| 7 | Bid failure produces error manifest with portal-specific context (step, screenshot, form state, side-effect class reached) |
| 8 | System can prepare and submit one request on Proactis end-to-end |
| 9 | `draft_saved` path works when `reviewRequired = true` (skill saves draft, operator reviews, then triggers final submission) |
| 10 | L5 readiness gate blocks `ready_to_bid` when active L5 blockers present |
| 11 | Validator rejection or outage blocks `bid_submitted` (fail-closed for portal submission) |

### 2.2 Test Files (3 New, Cumulative 50)

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
| Company profile schema | Structured profile for Proactis | Extensible for additional portal-specific fields |
| `bidScopeJson` | Explicit lot/candidate scope payload | Supports multi-lot bids, partial bids, amended bids |

### 2.6 Dependencies

- **Layer 0-5:** Full pipeline, desktop, web, sync, decision-grade intelligence
- **Specifically required:**
  - `operator_decision_heads` (Layer 4) — operator approval prerequisite
  - `shared_company_profile` (Layer 4) — company data for form filling
  - `bid_workflow_states` (Layer 0 schema, Layer 6 activation) — bid lifecycle
  - Semantic validation system (Layer 3-4) — `validation_policy: always` for bid skills

### 2.7 ADRs Addressed

| ADR | Decision | Layer 6 Action |
|-----|----------|---------------|
| **ADR-007** | Request pipeline automation (not real-money bids) | Portal-specific bid skill with full audit trail; operator approval required |

### 2.8 What's NOT Stubbed

Nothing — this is the final layer. No features are deferred beyond this point.

---

<!-- CROSS-DOMAIN NOTE FOR L0: `bid_workflow_states` DDL bid_state CHECK constraint should be expanded to include `draft_saved` and `portal_state_unknown` values added by L6. The current DDL allows free-text (no CHECK), so this is a recommendation, not a blocker. -->

<!-- CROSS-DOMAIN NOTE FOR L4: `operator_decision_heads` staleness detection must include L5 freshness vectors (tariff structure, measures, remedies, FX) in addition to pipeline assessment hash. When any L5 freshness vector expires for classifications backing an approved decision, the decision should be marked stale. -->
