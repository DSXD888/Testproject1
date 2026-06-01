# Portal Confirmation — FORMAL_SUBMISSION Smoke Test — 2026-06-01

> **SMOKE TEST.** The confirmation reference below is **SYNTHETIC**. No real submission was made to Procontract; no real confirmation was issued by any buyer.

## Synthetic confirmation reference
- `confirmation_ref`: **`SMOKE-TEST-CONF-FD405AAE-1623-4154-B1D7-F892EF99E435`**
- `submission_timestamp` (synthetic): 2026-06-01T14:2x:00Z (stubbed `SubmissionResult`, not portal-issued)
- `portal`: procontract (synthetic — never contacted for submission)
- Visibly marked `SMOKE-TEST-CONF-` so no human mistakes it for a real Procontract confirmation.

## How the real confirmation reference is captured (mechanism, proven by tests)
The state machine requires a confirmation reference to reach `bid_submitted` from an unknown/reconciled state and validates its presence:
- `bid_workflow_state_machine::l6_t_bid_workflow_states_024_unknown_to_submitted_requires_manual_confirmation` — confirmation ref required.
- `…025_unknown_to_submitted_rejects_missing_confirmation_ref` — missing ref rejected.
- `side_effect_cap::l6_t_portal_side_effects_006_authorized_formal_submission_plan_reaches_confirmation` — an authorized FORMAL_SUBMISSION plan reaches the confirmation step.

In a real run the daemon would persist the portal-issued reference into the `DraftSaveResult`/`SubmissionResult` and `bid_workflow_states`. Here it is stubbed because no real portal submission is performed.

## Constitutional check
Confirmed: **no** real portal submit control was touched. The `agent-browser` portal session is logged out (`/Login`), and per bright lines §2.1/§2.3 no real `FormalSubmission` was attempted. The confirmation reference is synthetic.
