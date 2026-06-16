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
   PROJECT_ROOT="~/Desktop/testproject1"
   ```

2. Run the idempotent CDP helper before any portal navigation. It attaches to the operator's persistent Chrome profile at `~/.chrome-automation` on port 9222 and must not create a fresh profile.

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
- Action: locate the opportunity using only `BidScopePayload.portalOpportunityRef`; do not browse adjacent tenders or infer from title alone.
- Prefer a direct opportunity URL if supplied by the daemon. Otherwise use the portal search UI and enter the exact reference into the opportunity or tender-reference search field.
- Read the search result row with CDP-attached read-only probes before clicking it. Capture the observed reference, tender title, buyer name when visible, deadline, and visible portal status.
- If no row has an exact `portalOpportunityRef` match, stop with `BID_PORTAL_CHANGED` and include the expected reference plus the visible result references in the failure context.
- Stop with `BID_PORTAL_CHANGED` if the search surface has changed enough that the reference cannot be entered or results cannot be interpreted.

### Step 4 - Verify opportunity and duplicate state
- Side-effect class: `READ_ONLY`.
- Screenshot: required.
- Action: compare the visible tender reference, title, deadline, and portal status against `BidScopePayload.portalOpportunityRef` and daemon-provided context.
- Read portal metadata with CDP-attached read-only probes. Do not type into the form or click mutation controls during this step.
- Opportunity-reference check: the observed tender or opportunity reference must exactly equal `BidScopePayload.portalOpportunityRef`. A mismatch means the browser is on the wrong tender; return `PAGE_SHAPE_DRIFT` when the page structure is unrecognizable, otherwise return `BID_PORTAL_CHANGED` with expected and observed references.
- Title check: compare the visible title against the daemon-provided title or decision context when present. Use a conservative substring or normalized-token match only as corroborating evidence; a severe mismatch returns `BID_PORTAL_CHANGED`.
- Portal-status check: read the visible status text. `Open` or an equivalent active-response state may continue. `Closed`, `Withdrawn`, `Awarded`, `Cancelled`, or any status that prevents a response returns `BID_READINESS_GATE_FAILED` because the readiness gate allowed a stale portal state.
- Existing-submission detection: search the current page and response summary for indicators such as `Already submitted`, `Submitted`, `View your response`, `Response submitted`, `Expression of interest submitted`, a portal confirmation reference, or a final-response timestamp. If any are present, return `BID_DUPLICATE_DETECTED` and do not continue.
- Existing-draft detection: search for indicators such as `Resume draft`, `Saved draft`, `Continue response`, `Draft response`, or `Last saved`. Record `existing_draft_detected: true`; continue only when the daemon scope includes a continuation or draft-reconciliation chain. Otherwise stop before mutation with `BID_DUPLICATE_DETECTED` if the draft cannot be safely reconciled.
- Page-fingerprint check: compute a read-only DOM-structure fingerprint for the source-structure version using these anchors: opportunity summary panel, opportunity reference element, response action bar, response form root, lot selection grid, document upload widget, and declaration/submit control group. Compare the anchor set against `source_structure_version: procontract-bid-portal-2026-05-22`. Major divergence returns `PAGE_SHAPE_DRIFT`; landing on a different Procontract tender, profile page, account page, or settings page returns `BID_PORTAL_CHANGED`.
- If the page fingerprint or opportunity reference diverges, return `PAGE_SHAPE_DRIFT` or `BID_PORTAL_CHANGED` before any portal mutation.
- Emit the Step 4 checkpoint with `PortalStateVerification`: `opportunity_found`, `existing_draft_detected`, `existing_submission_detected`, and `portal_status`. The checkpoint may include the opportunity reference and status, but must not include company-profile values.

### Step 5 - Open bid response form
- Side-effect class: `READ_ONLY`.
- Screenshot: required.
- Action: open the response form for the verified opportunity.
- Guard: confirm the form belongs to the same opportunity reference before touching any form field.
- Re-run the opportunity-reference and page-fingerprint checks after the form opens. The response form root, lot selection grid, and upload widget anchors must all belong to the verified `portalOpportunityRef`.
- Emit a Step 5 checkpoint that carries forward the Step 4 `PortalStateVerification` and records that no mutating side-effect class has been reached.
- Stop with `BID_PORTAL_CHANGED` if the portal redirects to a different tender, profile page, account page, or notification settings page.

### Step 6 - Fill company details section
- Side-effect class: `REMOTE_DRAFT`.
- Screenshot: required.
- Pre-step cap check: do not start if `sideEffectCap` is below `REMOTE_DRAFT`.
- Action: fill company details from `CompanyProfile` at the daemon-specified `companyProfileVersion`.
- Field mapping (from `CompanyProfile`, NEVER from any other source):
  - `companyName` -> portal "Company Name" input.
  - `registrationNumber` -> portal "Company Registration Number" input.
  - `vatNumber` -> portal "VAT Number" input, omitting the portal field when absent.
  - `address.line1` -> portal "Address Line 1" input.
  - `address.line2` -> portal "Address Line 2" input, omitting the portal field when absent.
  - `address.city` -> portal "City" input.
  - `address.county` -> portal "County" input.
  - `address.postcode` -> portal "Postcode" input.
  - `address.country` -> portal "Country" input.
  - `primaryContact.name` -> portal "Contact Name" input.
  - `primaryContact.email` -> portal "Contact Email" input.
  - `primaryContact.phone` -> portal "Contact Phone" input.
  - `primaryContact.role` -> portal "Contact Role/Title" input.
- DO NOT auto-fill from previously-saved portal data.
- DO NOT log filled values to stdout/stderr.
- DO NOT cache filled values to disk outside the daemon-managed artifact store.
- Do not print filled values, do not echo them in command output, and do not serialize them into checkpoint or result narrative fields.
- Use deterministic field selectors only after confirming the form still belongs to `BidScopePayload.portalOpportunityRef`.
- After all fields are filled, click the portal save control if it is present; otherwise wait up to 5 seconds for the portal auto-save indicator.
- The Step 6 checkpoint records `company_profile_version`, field names, omitted optional field names, and screenshot path only; every actual profile value must be redacted.
- After filling, checkpoint `side_effect_class_reached: "REMOTE_DRAFT"` because portal auto-save may have committed a remote draft.

### Step 7 - Fill lot selection and pricing sections
- Side-effect class: `REMOTE_DRAFT`.
- Screenshot: required.
- Pre-step cap check: do not start if `sideEffectCap` is below `REMOTE_DRAFT`.
- Action: select only `BidScopePayload.lotIds`; fill pricing and quote evidence only for selected candidates and selected quotes.
- Use only `BidScopePayload.lotIds`, `candidateIds`, `selectedQuoteIds` and daemon-provided `selected_quote_summaries`.
- Never infer lots, candidates, quantities, prices, or quote roles from cached portal values, adjacent tender rows, previous drafts, or visible account history.
- For each selected lot:
  1. Confirm the portal lot identifier exactly matches one entry in `BidScopePayload.lotIds`.
  2. Select the lot only once.
  3. Fill pricing fields from the matching selected quote summary for the scoped candidate and selected quote.
  4. Record only lot IDs, candidate IDs, quote IDs, pricing field names, and screenshot paths in the checkpoint; redact monetary values when the daemon marks them sensitive.
- For qualification questions, answer only when the daemon input carries an explicit scoped answer or document reference. Stop with `BID_SUBMISSION_FAILED` rather than inventing narrative answers.
- Do not add unscoped lots, optional alternatives, or whole-process blanket responses.
- If a required scoped lot is missing, return `BID_PORTAL_CHANGED`.
- If the portal preselects an unscoped lot or optional alternative, clear it before saving; if it cannot be cleared, stop with `BID_PORTAL_CHANGED`.
- The Step 7 checkpoint must include the selected `lotIds`, `candidateIds`, `selectedQuoteIds`, selected field names, and screenshot path.

### Step 8 - Upload required documents
- Side-effect class: `REMOTE_DRAFT`.
- Screenshot: required.
- Pre-step cap check: do not start if `sideEffectCap` is below `REMOTE_DRAFT`.
- Action: upload only daemon-staged `BidDocument[]` entries.
- For each `BidDocument` in `BidDocument[]`:
  1. Verify `BidDocument.filePath` exists and points inside the daemon-staged artifact area.
  2. Compute SHA-256 over the file contents.
  3. Compare the observed hash against `BidDocument.contentHash`.
  4. If the hashes differ, return `BID_SUBMISSION_FAILED` before selecting the file. Include structured context with `document_type`, `expected_hash`, and `observed_hash`.
  5. Locate the upload control for the matching document type. If no matching control exists, return `BID_SUBMISSION_FAILED`.
  6. Click "Upload" or "Add document" and select only `BidDocument.filePath`.
  7. Wait for portal acknowledgement such as "Upload complete", the filename appearing in the attachment list, or a portal-generated upload receipt.
  8. Capture a screenshot after each upload and at the end of Step 8.
  9. Record `{ document_type, file_name, content_hash, portal_acknowledgement_text, screenshot_path }` in the Step 8 checkpoint.
- The checkpoint key for acknowledgement text is `portal_acknowledgement_text`.
- Never browse outside daemon-staged document paths.
- Do not upload files that are not listed in `BidDocument[]`.
- If a required upload control is absent, a hash does not match, the portal rejects file type or size, or acknowledgement text never appears, return `BID_SUBMISSION_FAILED`.

### Step 9 - Review submission summary
- Side-effect class: `READ_ONLY`.
- Screenshot: required.
- Action: open the review or summary page and compare visible lot, price, document, and tender-reference summaries against the injected scope.
- This is a read-only verification step. Do not click final submission controls.
- Stop with `BID_PORTAL_CHANGED` if the summary does not match the scope.

### Step 10a - Save draft
- Side-effect class: `REMOTE_DRAFT -> state transitions to draft_saved`.
- Screenshot: required.
- Branch decision:
  - If `reviewRequired = true` -> branch 10a.
  - If `sideEffectCap` is `READ_ONLY`, `REMOTE_DRAFT`, `NOTIFICATION_OR_LOCK`, or `LEGAL_DECLARATION` -> branch 10a.
  - If the daemon requested draft-only execution -> branch 10a.
- Action:
  1. Click only the portal save-draft control.
  2. Wait for a visible saved state, toast, or draft reference.
  3. Capture the portal draft reference as `portal_draft_ref`; if the portal only exposes saved-state text, record that text as the draft reference evidence.
  4. Capture `step-10-save-draft.webp`.
  5. Emit a Step 10a checkpoint with `side_effect_class_reached: "REMOTE_DRAFT"`, `portal_draft_ref`, and the screenshot path list.
- Return `DraftSaveResult`:
  ```json
  {
    "kind": "draft_save_result",
    "draft_saved": true,
    "portal_draft_ref": "<draft-id-or-visible-saved-state>",
    "side_effect_class": "REMOTE_DRAFT",
    "screenshot_paths": ["sessions/<session_id>/step-01-login-page.webp", "...", "sessions/<session_id>/step-10-save-draft.webp"],
    "portal_timestamp": "<iso8601-from-portal-or-null>",
    "portal_state_verified": true
  }
  ```
- Do not include any final-submission confirmation fields in the draft result.

### Step 10b - Submit expression of interest
- Side-effect class: `FORMAL_SUBMISSION -> state transitions to bid_submitted`.
- Screenshot: required.
- Branch decision:
  - If `reviewRequired = false` AND `sideEffectCap` permits `FORMAL_SUBMISSION` -> branch 10b.
  - `validation_approved = true` OR `validation_override_logged = true` must also be present before clicking submit.
- Pre-step cap check: if `sideEffectCap` is below `FORMAL_SUBMISSION`, do not click submit. Save draft if possible, then return `BID_SIDE_EFFECT_CAP_EXCEEDED` with the blocked next step and attempted class.
- Pre-submit checks immediately before the click:
  1. `reviewRequired = false`.
  2. `sideEffectCap` is `FORMAL_SUBMISSION` or `FUNDS_COMMITTED`.
  3. `validation_approved = true` OR `validation_override_logged = true`.
  4. The review summary still matches `BidScopePayload.portalOpportunityRef`, `lotIds`, `candidateIds`, and `selectedQuoteIds`.
  5. No existing submission is visible from `verifyPortalState()`.
- Action:
  1. Click the final submit or expression-of-interest control exactly once.
  2. Do not click submit again on timeout or ambiguous portal state.
  3. Wait for the confirmation page or a portal confirmation panel.
  4. Capture `step-10-submit.webp`.
  5. Proceed to Step 11 before returning.
- Return `SubmissionResult` only after Step 11 has captured confirmation evidence:
  ```json
  {
    "kind": "submission_result",
    "submitted": true,
    "confirmation_ref": "<portal-confirmation-reference>",
    "final_side_effect_class": "FORMAL_SUBMISSION",
    "screenshot_paths": ["sessions/<session_id>/step-01-login-page.webp", "...", "sessions/<session_id>/step-11-confirmation.webp"],
    "portal_timestamp": "<iso8601-from-portal-or-null>",
    "portal_state_verified": true,
    "validation_approved": true,
    "validation_override_logged": false
  }
  ```

### Step 11 - Capture confirmation page/reference
- Side-effect class: `READ_ONLY`.
- Screenshot: required.
- Action: read the confirmation page after Step 10b or the saved-draft state after Step 10a.
- Capture portal confirmation reference, draft reference, portal status, URL, and page fingerprint.
- Step 11 always runs after Step 10b.
- For the final-submission path, capture `confirmation_ref`, portal-reported submission timestamp, confirmation page URL, and `step-11-confirmation.webp`.
- Keep `side_effect_class_reached` at `FORMAL_SUBMISSION` during Step 11; Step 11 is read-only, but the run's highest side-effect class must not decrease.
- For the draft path, Step 11 may read saved-draft state only when the daemon explicitly asks for post-draft verification; do not turn a saved draft into a submission.
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
- DO NOT launch a fresh browser profile — attach to operator's persistent CDP at `~/.chrome-automation:9222`.
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
