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
not_started --> ready_to_bid --> bid_in_progress --> bid_submitted
                                                 \-> bid_failed
```

**State transitions:**

| From | To | Trigger | Requirements |
|------|-----|---------|-------------|
| `not_started` | `ready_to_bid` | Operator approves opportunity AND disposition is not stale | `operator_decision_heads.disposition = 'approve'` AND `reviewedBasisHash == current decisionBasisHash` (not stale) |
| `ready_to_bid` | `bid_in_progress` | Operator clicks "Start Bid" in desktop | Bid scope payload defined (which lots, which candidates). Daemon creates adapter-mediated agent session. |
| `bid_in_progress` | `bid_submitted` | Agent completes portal submission successfully | All form steps completed. Screenshots captured. Submission confirmation received. Run manifest generated. |
| `bid_in_progress` | `bid_failed` | Agent fails during portal submission | Error manifest created with step context, screenshots at failure point, terminal output. Operator can retry or abandon. |

**Prerequisite check (constitutional):** Bid workflow CANNOT start unless:
1. Operator `approved` disposition exists on `operator_decision_heads` (not just `pending_operator_intents`)
2. The decision is NOT stale (`reviewedBasisHash == current decisionBasisHash`)
3. If stale, operator must re-affirm before bid can proceed

**Validation policy:** Bid automation skills default to `validation_policy: always` despite low financial risk, given operational consequence (portal form filling, document submission). Validator catches: wrong tender, wrong opportunity data, incorrect documents.

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
  portalOpportunityRef?: string       // Portal-specific opportunity reference
  reviewRequired: boolean             // Does the operator need to sign off before final submission?
}
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
  fillForms(session: AgentSession, scope: BidScopePayload, data: PipelineData): Promise<void>
  uploadDocuments(session: AgentSession, documents: BidDocument[]): Promise<void>
  submit(session: AgentSession, reviewRequired: boolean): Promise<SubmissionResult>
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
| **Interaction profile** | `pty_browser` — browser automation required |
| **Model profile** | `reasoning` — needs to interpret portal structure and handle form variations |
| **Validation policy** | `always` — every bid run gets semantic validation |
| **Auth requirements** | Portal credentials (stored in session-isolated config) |
| **Resume strategy** | `RESUME_UNSAFE_AFTER_SUBMIT` — cannot safely resume after form submission boundary |

**Skill playbook steps:**

| Step | Action | Screenshot | Side-Effect Boundary |
|------|--------|-----------|---------------------|
| 1 | Navigate to Proactis portal login page | Yes | No |
| 2 | Authenticate with portal credentials | Yes | No |
| 3 | Search/navigate to the target opportunity | Yes | No |
| 4 | Verify opportunity matches expected tender reference | Yes | No |
| 5 | Open bid response form | Yes | No |
| 6 | Fill company details section from `shared_company_profile` | Yes | No (draft auto-save may occur) |
| 7 | Fill lot selection and pricing sections from pipeline data | Yes | **Yes — form save boundary** |
| 8 | Upload required documents (certificates, declarations) | Yes | **Yes — document upload** |
| 9 | Review submission summary | Yes | No |
| 10 | Submit expression of interest (if `reviewRequired = false`) OR save draft (if `reviewRequired = true`) | Yes | **Yes — submission boundary** |
| 11 | Capture confirmation page/reference | Yes | No |

**Side-effect boundary rule:** After step 7+ (form save), the skill declares `RESUME_UNSAFE_AFTER_SUBMIT`. If the skill fails after this point, it switches to verify-only mode or requires human confirmation before retrying. This prevents duplicate submissions.

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
| `bid_workflow_states` | Push-owned | Now fully active: lifecycle states `not_started` -> `ready_to_bid` -> `bid_in_progress` -> `bid_submitted` / `bid_failed`. `bidScopeJson` populated with explicit lot/candidate scope. Synced to Supabase for web visibility. |

### 1.10 Invariants Enforced (Constitutional)

| Invariant | Enforcement |
|-----------|-------------|
| No real-money bids without human approval | Bid workflow requires operator `approved` disposition on `operator_decision_heads` |
| Bid workflow carries explicit scope | `bidScopeJson` payload with lot/candidate IDs — no blanket bids |
| Full audit trail for submissions | Screenshots, terminal output, run manifest captured for every submission |
| Stale decisions block bid | `reviewedBasisHash != decisionBasisHash` -> bid blocked until re-affirm |
| Validation policy: always | Every bid run gets semantic validation regardless of promotion stage |

---

## 2. Testing, DoD, Risks & Open Questions

### 2.1 Definition of Done

| # | Criterion |
|---|----------|
| 1 | Bid-portal skill navigates target portal (Proactis) via browser automation |
| 2 | Company profile data injected from `shared_company_profile` |
| 3 | Request preparation steps automated (form filling, document upload) |
| 4 | Submission step automated with screenshot capture at each stage |
| 5 | Full terminal output tracing for bid workflow |
| 6 | Run manifest generated for submitted request with audit trail |
| 7 | Bid failure produces error manifest with portal-specific context (step, screenshot, form state) |
| 8 | System can prepare and submit one request on Proactis end-to-end |

### 2.2 Test Scenarios

| Scenario | Expected Behavior |
|----------|------------------|
| Full bid workflow: approve -> prepare -> submit -> manifest | Complete lifecycle: `not_started` -> `ready_to_bid` -> `bid_in_progress` -> `bid_submitted`. Screenshots at each step. Manifest generated. |
| Bid failure mid-workflow (step 8: document upload fails) | Error manifest with screenshots at failure point. `bid_failed` state. Operator can retry or abandon. |
| Stale decision blocks bid | Operator decision marked stale (`reviewedBasisHash` mismatch) -> bid workflow blocked. Cannot proceed until operator re-affirms with updated basis. |
| Portal website changed (form structure different) | Skill fails with `AGENT_SKILL_FAILED`. Error manifest flags the specific step where portal structure diverged. Page fingerprint drift detected. |
| `reviewRequired = true` | Skill saves draft instead of submitting. Operator reviews in desktop before final submission trigger. |

### 2.3 Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Portal UI changes break bid skill | High | Low | Not real money — error manifest flags failures; skill update cycle; page fingerprint drift detection |
| Form structure varies by procurement type | Medium | Medium | Portal-specific workflow handles variance; fallback to verify-only mode |
| Duplicate submission on retry after side-effect boundary | Medium | Medium | `RESUME_UNSAFE_AFTER_SUBMIT` flag; verify-only mode after submission boundary; human confirmation required |
| Portal credentials expire during bid | Low | Low | Auth challenge detection; `auth-refresh` skill for portal (future) |

### 2.4 Forward-Compatibility Provisions

| Provision | What Exists in L6 | Future Extension |
|-----------|-------------------|-----------------|
| Workflow registry pattern | Maps portal type to workflow implementation | Add new portal support = new workflow file + registry entry |
| Company profile schema | Structured profile for Proactis | Extensible for additional portal-specific fields |
| `bidScopeJson` | Explicit lot/candidate scope payload | Supports multi-lot bids, partial bids, amended bids |

### 2.5 Dependencies

- **Layer 0-5:** Full pipeline, desktop, web, sync, decision-grade intelligence
- **Specifically required:**
  - `operator_decision_heads` (Layer 4) — operator approval prerequisite
  - `shared_company_profile` (Layer 4) — company data for form filling
  - `bid_workflow_states` (Layer 0 schema, Layer 6 activation) — bid lifecycle
  - Semantic validation system (Layer 3-4) — `validation_policy: always` for bid skills

### 2.6 ADRs Addressed

| ADR | Decision | Layer 6 Action |
|-----|----------|---------------|
| **ADR-007** | Request pipeline automation (not real-money bids) | Portal-specific bid skill with full audit trail; operator approval required |

### 2.7 What's NOT Stubbed

Nothing — this is the final layer. No features are deferred beyond this point.
