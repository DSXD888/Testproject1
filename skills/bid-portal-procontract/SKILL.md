---
skill_id: bid-portal/procontract
skill_version: 0.1.0
interaction_profile: pty_interactive
result_schema_version: 1.0.0
checkpoint_schema_version: 1.0.0
model_profile: portal_form_high_reliability
agent_cli: claude
side_effect_class: external_mutation
validation_policy: always
requires_cdp: true
cdp_default_port: 9222
source_structure_version: procontract-bid-portal-2026-05-22
promotion_stage: draft
worker_behavior_version: bid-portal/procontract@0.1.0+1.0
version_change_affects_behavior: true
---

# bid-portal/procontract

## Use when
Use this skill when the daemon has already approved a scoped L6 bid workflow for a Procontract opportunity and needs a PTY browser agent to perform the portal-specific bid response flow.

## Do not use when
Do not use this skill to discover procurement opportunities, refresh portal credentials, change account settings, withdraw bids, or perform any portal action outside the injected bid scope.

## Inputs
- `BidScopePayload`: scoped process, lot, candidate, quote, opportunity, decision-basis, review, and `sideEffectCap` contract already approved by the daemon readiness gate.
- `CompanyProfile`: daemon-injected profile version from `shared_company_profile`; treat all profile fields as PII.
- `BidDocument[]`: daemon-staged upload files with content hashes and safe local paths.
- `stage_run_id`, `bid_job_id`, and optional structured operator override records.

## Outputs
- `result.json` matching `result.schema.json`.
- `checkpoint.json` matching `checkpoint.schema.json` after every completed playbook step.
- One screenshot per playbook step, written only to the daemon-provided artifact directory.
- No company-profile values in logs, screenshots filenames, checkpoint text, or narrative fields.

## Side Effects
This skill performs externally visible portal work on the operator's real Procontract account. The daemon must enforce `BidScopePayload.sideEffectCap` before each step, and the skill must still self-check before any step that can reach `REMOTE_DRAFT` or `FORMAL_SUBMISSION`.

## Failure Codes
- AUTH_STATE_STALE
- BID_SUBMISSION_FAILED
- BID_DUPLICATE_DETECTED
- BID_SIDE_EFFECT_CAP_EXCEEDED
- BID_PORTAL_CHANGED
- BID_PORTAL_AUTH_EXPIRED
- PAGE_SHAPE_DRIFT
- CDP_CONNECT_FAILED

---

## Purpose
Navigate the Procontract eSourcing portal, fill bid forms, upload documents, and either save a draft or submit an expression of interest with a full audit trail: screenshots per step, PTY capture, run manifest evidence, and an error manifest on failure.

## Pre-Flight
1. Resolve the CDP port:

   ```bash
   CDP_PORT="${AGENT_BROWSER_CDP_PORT:-9222}"
   PROJECT_ROOT="/Users/alinaaitova/Desktop/Testproject1"
   ```

2. Run the idempotent CDP helper before any portal navigation. It attaches to the operator's persistent Chrome profile at `/Users/alinaaitova/.chrome-automation` on port 9222 and must not create a fresh profile.

   ```bash
   "${PROJECT_ROOT}/bin/ensure-cdp-9222"
   curl -sfS -m 3 "http://localhost:${CDP_PORT}/json/version"
   ```

3. If CDP is unreachable, write `result.json` with `status: "failed"`, `failure_code: "CDP_CONNECT_FAILED"`, no portal mutation fields, and stop.

4. Read only daemon-injected input files. Do not read credential stores, browser password settings, or unrelated operator data.

5. Create an initial checkpoint with `step: 0`, `side_effect_class_reached: "READ_ONLY"`, and an empty `screenshot_paths` array.

## Browser Command Contract
Every browser command must include the CDP attachment flag:

```bash
agent-browser --cdp ${CDP_PORT} open "https://procontract.due-north.com/"
agent-browser --cdp ${CDP_PORT} snapshot
agent-browser --cdp ${CDP_PORT} eval '<read-only JavaScript probe>'
```

Do not run bare `agent-browser` commands. Do not run browser setup commands. The operator browser is shared state.

## Checkpoint Contract
After each step:
- Save a screenshot to the daemon artifact directory.
- Append the relative screenshot path to `screenshot_paths`.
- Update `step`, `step_name`, `side_effect_class_reached`, `portal_state`, and `updated_at`.
- Record only references and booleans in `portal_state`; do not write company-profile values.
- If the process stops after a mutating step, the next run must call `verifyPortalState()` before any further mutation.

## Playbook
You are a portal action agent. The daemon has already decided that the bid scope is eligible. Your job is to perform exactly the scoped Procontract response workflow and report what happened.

### Step 1 - Navigate to Procontract portal login page
- Side-effect class: `READ_ONLY`.
- Screenshot: required.
- Action: open `https://procontract.due-north.com/` through the CDP browser.
- Evidence: current URL, page title, and screenshot path.

```bash
agent-browser --cdp ${CDP_PORT} open "https://procontract.due-north.com/"
agent-browser --cdp ${CDP_PORT} wait 1500
agent-browser --cdp ${CDP_PORT} snapshot
```

### Step 2 - Verify authenticated state
- Side-effect class: `READ_ONLY`.
- Screenshot: required.
- Action: read the current URL and visible page text using CDP-attached read-only probes.
- If the URL contains `/Login` or the page text matches `Session expired|Please log in|Login or register for free`, return `AUTH_STATE_STALE` immediately.
- Do not attempt portal sign-in.
- Do not navigate to the login page after detecting stale auth.
- Do not type credentials, click sign-in controls, or interact with Chrome credential prompts.
- Do not call `auth-signin/procontract`.
- The daemon owns the retry: it runs `auth-signin/procontract` as a sub-job and then reruns this bid job.

Step 2 stale-auth result envelope:

```json
{
  "kind": "failure",
  "error_code": "AUTH_STATE_STALE",
  "step_reached": 2,
  "side_effect_class_reached": "READ_ONLY",
  "screenshot_paths": ["sessions/<session_id>/step-02-auth-state.webp"],
  "error_message": "Procontract session expired or missing; daemon must run auth-signin/procontract and retry",
  "portal_state": {
    "opportunity_found": false,
    "existing_draft_detected": false,
    "existing_submission_detected": false,
    "portal_status": "auth_state_stale"
  }
}
```

### Step 3 - Search or navigate to the target opportunity
- Side-effect class: `READ_ONLY`.
- Screenshot: required.
- Action: locate the opportunity using only `BidScopePayload.portalOpportunityRef`.
- Prefer a direct opportunity URL if supplied by the daemon. Otherwise use the portal search UI and the exact reference.
- Stop with `BID_PORTAL_CHANGED` if the search surface has changed enough that the reference cannot be entered or results cannot be interpreted.

### Step 4 - Verify opportunity and duplicate state
- Side-effect class: `READ_ONLY`.
- Screenshot: required.
- Action: compare the visible tender reference, title, deadline, and portal status against `BidScopePayload.portalOpportunityRef` and daemon-provided context.
- Detect existing draft, existing response, submitted expression of interest, withdrawn response, closed opportunity, or portal status that differs from the expected open-response state.
- If an existing submission is detected, return `BID_DUPLICATE_DETECTED` and do not continue.
- If an existing draft is detected, record `existing_draft_detected: true` and continue only if the daemon scope allows draft reconciliation.
- If the page fingerprint or opportunity reference diverges, return `PAGE_SHAPE_DRIFT` or `BID_PORTAL_CHANGED`.

### Step 5 - Open bid response form
- Side-effect class: `READ_ONLY`.
- Screenshot: required.
- Action: open the response form for the verified opportunity.
- Guard: confirm the form belongs to the same opportunity reference before touching any form field.
- Stop with `BID_PORTAL_CHANGED` if the portal redirects to a different tender, profile page, account page, or notification settings page.

### Step 6 - Fill company details section
- Side-effect class: `REMOTE_DRAFT`.
- Screenshot: required.
- Pre-step cap check: do not start if `sideEffectCap` is below `REMOTE_DRAFT`.
- Action: fill company details from `CompanyProfile` at the daemon-specified `companyProfileVersion`.
- Do not print filled values, do not echo them in command output, and do not serialize them into checkpoint or result narrative fields.
- After filling, checkpoint `side_effect_class_reached: "REMOTE_DRAFT"` because portal auto-save may have committed a remote draft.

### Step 7 - Fill lot selection and pricing sections
- Side-effect class: `REMOTE_DRAFT`.
- Screenshot: required.
- Pre-step cap check: do not start if `sideEffectCap` is below `REMOTE_DRAFT`.
- Action: select only `BidScopePayload.lotIds`; fill pricing and quote evidence only for selected candidates and selected quotes.
- Do not add unscoped lots, optional alternatives, or whole-process blanket responses.
- If a required scoped lot is missing, return `BID_PORTAL_CHANGED`.

### Step 8 - Upload required documents
- Side-effect class: `REMOTE_DRAFT`.
- Screenshot: required.
- Pre-step cap check: do not start if `sideEffectCap` is below `REMOTE_DRAFT`.
- Action: upload only daemon-staged `BidDocument[]` entries.
- Verify each document content hash before upload.
- Never browse outside daemon-staged document paths.
- If a required upload control is absent or a hash does not match, return `BID_SUBMISSION_FAILED`.

### Step 9 - Review submission summary
- Side-effect class: `READ_ONLY`.
- Screenshot: required.
- Action: open the review or summary page and compare visible lot, price, document, and tender-reference summaries against the injected scope.
- This is a read-only verification step. Do not click final submission controls.
- Stop with `BID_PORTAL_CHANGED` if the summary does not match the scope.

### Step 10a - Save draft
- Side-effect class: `REMOTE_DRAFT -> state transitions to draft_saved`.
- Screenshot: required.
- Choose this branch when `reviewRequired=true`, when the next step would exceed `sideEffectCap`, or when the daemon requested draft-only execution.
- Action: click the portal save-draft control and capture the portal draft reference or visible saved state.
- Return `DraftSaveResult` with `draft_saved: true`, `portal_draft_ref`, final side-effect class `REMOTE_DRAFT`, and the complete screenshot path list.

### Step 10b - Submit expression of interest
- Side-effect class: `FORMAL_SUBMISSION -> state transitions to bid_submitted`.
- Screenshot: required.
- Choose this branch only when `reviewRequired=false`, validator clearance is present or a structured validation override is injected, and `sideEffectCap` permits `FORMAL_SUBMISSION`.
- Pre-step cap check: if `sideEffectCap` is below `FORMAL_SUBMISSION`, do not click submit. Save draft if possible, then return `BID_SIDE_EFFECT_CAP_EXCEEDED`.
- Action: click the final submit or expression-of-interest control exactly once.
- Return `SubmissionResult` with `submitted: true`, confirmation reference, final side-effect class `FORMAL_SUBMISSION`, and the complete screenshot path list.

### Step 11 - Capture confirmation page/reference
- Side-effect class: `READ_ONLY`.
- Screenshot: required.
- Action: read the confirmation page after Step 10b or the saved-draft state after Step 10a.
- Capture portal confirmation reference, draft reference, portal status, URL, and page fingerprint.
- If the portal response is ambiguous after a final submission click, stop with `portal_state_unknown` semantics in the result and do not retry blindly.

## Side-Effect Cap Guard
Before every step, compare the next step's side-effect class with `BidScopePayload.sideEffectCap`.

- If the next step is allowed, continue.
- If the next step would exceed the cap and the current state can be saved as a draft, take Step 10a and stop.
- If the next step would exceed the cap and no draft can be saved, return `BID_SIDE_EFFECT_CAP_EXCEEDED` without escalating.
- Never rely only on daemon enforcement; the skill must self-police the cap.
- Treat missing or malformed `sideEffectCap` as a hard failure before any mutating step.
- Record the blocked next step and attempted side-effect class in the failed result.
- Do not downgrade the declared step class to make a blocked action appear allowed.

## Failure Modes by Side-Effect Class
- `READ_ONLY`: Safe to retry from the beginning after CDP and auth state are rechecked.
- `REMOTE_DRAFT`: Call `verifyPortalState()` before retry. If a matching draft exists, resume from the draft rather than creating a second one.
- `NOTIFICATION_OR_LOCK`: Stop at `portal_state_unknown`; human must verify whether a buyer-visible notification or lock occurred.
- `LEGAL_DECLARATION`: Stop at `portal_state_unknown`; human must reconfirm before any later legal-declaration action.
- `FORMAL_SUBMISSION`: Never blind retry. Stop at `portal_state_unknown`; human verifies portal confirmation or duplicate state.
- `FUNDS_COMMITTED`: Procontract bid workflow must not intentionally reach this class. If observed, stop at `portal_state_unknown` and report a critical failure.

## Crash Recovery
- Last reached `READ_ONLY`: rerun from Step 1.
- Last reached `REMOTE_DRAFT`: navigate to the opportunity, call `verifyPortalState()`, and reconcile with the existing draft before editing.
- Last reached `FORMAL_SUBMISSION`: do not click submit again. Inspect portal state read-only and return duplicate or confirmation evidence.
- Any unknown portal state after mutation: return a failed result with `portal_state_unknown: true` and all screenshots captured so far.

## CDP Discipline - Do NOT
- DO NOT launch a fresh browser profile — attach to operator's persistent CDP at `/Users/alinaaitova/.chrome-automation:9222`.
- DO NOT close the operator's browser.
- DO NOT sign out — subsequent skills (or operator) need the session.
- DO NOT solve captchas.
- DO NOT type credentials directly — auth-signin/procontract uses Chrome autofill.
- DO NOT invoke auth-signin/procontract as a sub-skill — return AUTH_STATE_STALE on session expiry; the DAEMON orchestrates the retry.
- DO NOT escalate side-effect class past BidScopePayload.sideEffectCap.
- DO NOT submit any form OTHER than the bid response form (no profile changes, no account settings, no withdraws of other bids).
- DO NOT cache or log filled values to disk — company profile fields are PII.
- DO NOT proceed past Step 4 if verifyPortalState detects an existing submission — raise BID_DUPLICATE_DETECTED.
- DO NOT continue if portal state diverged from the expected opportunity — raise PAGE_SHAPE_DRIFT (or BID_PORTAL_CHANGED).

## Result Writing
Write exactly one terminal `result.json`.

- `status: "draft_saved"` for Step 10a success.
- `status: "submitted"` for Step 10b plus Step 11 confirmation success.
- `status: "failed"` with a structured failure code for all failures.
- Include `screenshot_paths`, `side_effect_class_reached`, `portal_state_verified`, and either draft or submission reference fields.
- Do not include raw company-profile values, operator credentials, cookies, local browser profile paths beyond the fixed CDP profile path, or unscoped portal data.

## Audit Trail
- Screenshot every step, including failed steps.
- Keep screenshot filenames generic: `step-01-login-page.webp`, `step-02-auth-state.webp`, and so on.
- The daemon writes PTY output and the run manifest. The skill must keep stdout concise and avoid PII.
- Mention lot IDs, candidate IDs, selected quote IDs, opportunity reference, and side-effect class progression in result metadata only when those values came from the daemon scope.
- Cite structured operator override identifiers when the daemon injects them; do not invent override reasons.
