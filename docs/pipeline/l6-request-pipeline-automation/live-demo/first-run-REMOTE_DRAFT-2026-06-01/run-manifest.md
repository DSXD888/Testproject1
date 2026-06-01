# Bid Workflow Run Manifest — REMOTE_DRAFT Smoke Test — 2026-06-01

> **SMOKE TEST / PRACTICE MODE.** Synthetic scope + placeholder profile. **No real draft saved on Procontract.**
> Bead `testproject1-3yqf.7.2` · Owner TealHarbor · Test `L6-T-live_demo-002`.
> Operator standing authorization 2026-06-01: prove the REMOTE_DRAFT mechanism *possible + correct* via smoke test, then close. Real portal draft deferred.

## Agent Narrative (worker-authored)
The REMOTE_DRAFT bid mechanism was exercised under the smoke-test mandate. Because (a) the operator's Procontract session is currently logged out and bright lines §2.3/§2.4 forbid me from manually signing in or clicking on the live portal, and (b) no real opportunity in Sorellon's pool is past the register-interest gate, the *real portal draft* is correctly NOT attempted. Instead the mechanism's correctness is proven via (1) a **live daemon execution** of the real `daemon --run-bid-workflow` binary, and (2) the **authoritative test suite** that drives the real daemon binary + state machine (no mocks). Live portal cap-respect is proven by the 2026-05-25 register-interest cap-stop plus today's reachability/auth-gate observation.

## Side-Effect Progression (verified)
SideEffectClass ladder (`crates/daemon/src/bid_workflow/types.rs:27`, ascending):
`ReadOnly < RemoteDraft < NotificationOrLock < LegalDeclaration < FormalSubmission < FundsCommitted`. Default cap `RemoteDraft` (`DEFAULT_SIDE_EFFECT_CAP`). On Procontract, "Register interest" = `NotificationOrLock` → **exceeds** `RemoteDraft` → skill correctly refuses (no buyer notification). Under `RemoteDraft` the highest permitted terminal is `draft_saved`; `bid_submitted` (FormalSubmission) is blocked.

## Daemon Evidence Appendix (machine-observed)

### A. Live readiness-gate enforcement (real `daemon --run-bid-workflow`, 2026-06-01)
A synthetic SMOKE-TEST bid job (process `SMOKE-TEST-process-0601`, cap `REMOTE_DRAFT`, `reviewRequired=true`) was enqueued into a fresh temp state.db and run through the real daemon binary. The gate refused to proceed to any portal side-effect and emitted structured, non-overrideable blockers — proving the gate that guards the bid mechanism enforces its invariants live:
```
Error: bid readiness blocked for `SMOKE-TEST-bid-job-0601`: [
  {"code":"PROCESS_NOT_FOUND","overrideable":false,"source":"procurement_processes"},
  {"code":"OPERATOR_NOT_APPROVED","overrideable":false,"source":"operator_decision_heads"},
  {"code":"DECISION_BASIS_STALE","overrideable":false,"source":"operator_decision_heads.reviewed_basis_hash"},
  {"code":"L5_READINESS_PROJECTION_MISSING","overrideable":false,"source":"landed_cost_estimates"},
  {"code":"COMPANY_PROFILE_OUT_OF_DATE","overrideable":false,"source":"shared_company_profile"},
  {"code":"QUOTE_ROLE_FLOOR_ONLY","overrideable":true,"source":"supplier_quotes.price_signal_role"},
  {"code":"UPSTREAM_TRUST_UNVALIDATED","overrideable":false,"source":"pipeline_assessments.validation_state"}
]
```
Per handoff §11, a blocked manual trigger that prints its blockers JSON is itself a valid proof the gate enforces its invariants. The gate refuses any REMOTE_DRAFT portal action until readiness is genuinely satisfied — exactly the protection that makes a real draft safe.

### B. draft_saved mechanism — authoritative test suite (real daemon binary + state machine, no mocks)
| Suite | Result | Proves |
|---|---|---|
| `bid_workflow_state_machine` | 35/35 | `BidInProgress→DraftSaved` transition w/ evidence guards (`l6_t_bid_workflow_states_008_in_progress_to_draft_saved_requires_draft_evidence`); rejects missing evidence (`…009`) |
| `side_effect_cap` | 11/11 | RemoteDraft cap allows draft but **blocks submit** (`l6_t_portal_side_effects_005_remote_draft_cap_allows_draft_but_blocks_submit`); ReadOnly blocks form mutation (`…004`) |
| `bid_runner_body` | 33/33 | runner body writes `draft_saved` state + run manifest |
| `bid_run_manifest` | 2/2 | hybrid run-manifest (narrative + daemon evidence appendix) |
| `cli_bid_workflow` | 31/31 | `--run-bid-workflow` blocks-before-enqueue / enqueues-after-readiness (`l6_t_daemon_runner_010_*`) |
| `ipc_trigger_bid_workflow` | 36/36 | desktop **Start Bid** → `trigger_bid_workflow` IPC path |

### C. Validator verdict
No portal run manifest was produced (no real draft), so no live validator verdict row exists for this scope. The validator-overlay mechanism is proven separately by `validator_overlay_bid` (run under bead 7.3 / 7.4). `validation_verdicts` link column is `worker_job_id`.

### D. Readiness evidence
L5→L6 readiness gate producibility confirmed in preflight: `bid-readiness-gate.test.ts` 16/16. Live gate counterpart = `readiness_gate.rs` (exercised in §A above).

## Conclusion
The REMOTE_DRAFT mechanism is **possible and correct**: the state machine reaches `draft_saved` only with valid draft evidence; the side-effect cap permits draft and blocks submit; the readiness gate refuses portal action until upstream readiness holds; the desktop/IPC/CLI trigger paths all funnel through that gate. Real portal draft deferred per operator instruction 2026-06-01 — no buyer-visible action taken.
