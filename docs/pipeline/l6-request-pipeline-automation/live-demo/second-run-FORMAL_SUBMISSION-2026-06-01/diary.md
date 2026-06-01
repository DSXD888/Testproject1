# FORMAL_SUBMISSION Live-Run Diary — 2026-06-01 (SMOKE TEST)

> **SMOKE TEST / PRACTICE MODE.** Synthetic/stubbed submission only. **No real bid submitted. No buyer notified.**
> Bead `testproject1-3yqf.7.3` (l6-p7-live-demo-formal-submission) · Owner TealHarbor · Test `L6-T-live_demo-003`.
> Authorization: operator standing authorization 2026-06-01 (see `operator-authorization.md`) — synthetic close only; real submission deferred.

## Mandate reframe (handoff §3, §10-7.3)
The operator's standing authorization covers closing 7.3 on a synthetic basis. I did **not** pause for a real-submission go-ahead and did **not** submit a real bid. Goal: prove the submission mechanism + fail-closed validator are possible + correct via synthetic/stubbed data.

## Timeline

### 14:2xZ — Authorization gate
`operator-authorization.md` recorded BEFORE any system action (hard gate satisfied on a smoke-test basis). It explicitly is NOT authorization for a real submission.

### 14:2xZ — Submission mechanism (synthetic, stubbed)
Proven that the state machine advances `draft_saved → bid_in_progress → bid_submitted` under `sideEffectCap = FORMAL_SUBMISSION` with the portal submission step **stubbed** (synthetic `SubmissionResult` carrying confirmation ref `SMOKE-TEST-CONF-FD405AAE-…`). Proven by the real daemon binary + state machine (no mocks):
| Test | Proves |
|---|---|
| `bid_workflow_state_machine::l6_t_bid_workflow_states_016_draft_saved_to_submitted_rechecks_readiness_and_validation` | `draft_saved→bid_submitted` rechecks readiness + validation |
| `…017_draft_saved_to_submitted_rejects_final_gate_failures` | final-gate failures block submit |
| `…010_in_progress_to_submitted_requires_validation_and_cap` | submit requires validation + FORMAL_SUBMISSION cap |
| `…024 / …025` | confirmation ref required / missing ref rejected |
| `side_effect_cap::l6_t_portal_side_effects_006_authorized_formal_submission_plan_reaches_confirmation` | authorized FORMAL_SUBMISSION plan reaches confirmation |
| `…006_formal_submission_cap_blocks_funds_committed` | cap still blocks the next class up (`FundsCommitted`) |

### 14:2xZ — Fail-closed validator behavior (spec §1.10 #7) — BOTH directions
`cargo test --test validator_overlay_bid` → **18/18**. Decisive cases:
- `bid_portal_overlay_routes_through_existing_validator_bundle_and_rejection_blocks_submit` — **validator rejection BLOCKS `bid_submitted`** (fail-closed).
- `validator_outage_blocks_final_submit_but_draft_save_remains_reachable` — **validator outage BLOCKS final submit** (fail-closed), while draft save stays reachable.
- `structured_override_allows_final_submit_and_renders_manifest_audit` — **with a structured operator override, submit proceeds** and the override is rendered into the manifest audit.
This proves both required directions: no override ⇒ rejection/outage blocks submission; structured override ⇒ submission proceeds with audit.

### Constitutional check
- `agent-browser` portal session is logged out (`/Login`); no real submit control touched.
- Confirmation reference is visibly synthetic (`SMOKE-TEST-CONF-…`).
- Production `bid_workflow_states` left empty (no synthetic row written to the operator's real DB).

## Decision: synthetic/stubbed mechanism proof (not a real submission)
Per the operator's standing authorization and bright line §2.1, a real `FormalSubmission` on the live account is forbidden. The mechanism is proven via the real daemon binary + state machine + validator overlay with synthetic stubbed data — the sanctioned smoke-test path (handoff §10-7.3).

## Evidence index
- `operator-authorization.md` — standing authorization (synthetic close; real submission deferred).
- `portal-confirmation.md` — synthetic confirmation reference + how a real ref is captured.
- This diary — execution log + test evidence.

## Verdict
**GREEN (smoke test).** FORMAL_SUBMISSION mechanism proven possible + correct via synthetic/stubbed submission: state reaches `bid_submitted` (synthetic), confirmation ref required + captured (synthetic), and the validator is **fail-closed** (rejection/outage block submit; structured override permits it with audit). Real legally-binding submission deferred per operator instruction 2026-06-01. `L6-T-live_demo-003` satisfied on a smoke-test basis. DoD #8 → PARTIAL.
