# REMOTE_DRAFT Live-Run Diary — 2026-06-01 (SMOKE TEST)

> **SMOKE TEST / PRACTICE MODE.** Synthetic scope + placeholder profile. **No real draft saved on Procontract. No buyer notified.**
> Bead `testproject1-3yqf.7.2` (l6-p7-live-demo-remote-draft) · Owner TealHarbor · Test `L6-T-live_demo-002`.
> Operator standing authorization 2026-06-01: prove REMOTE_DRAFT mechanism possible+correct via smoke test, then close.

## Mandate reframe (handoff §10-7.2)
Goal = prove the REMOTE_DRAFT mechanism is **possible and correct**, NOT to save a real draft on a real opportunity. Two proofs:
- **Proof A** — live cap-respect (safety) on the real portal, read-only.
- **Proof B** — `draft_saved` mechanism correctness (synthetic + authoritative test suite).

## Timeline

### 14:08Z — Environment (see preflight-2026-06-01.md)
CDP up (Chrome/148), AgentMail healthy, Supabase REST 200, daemon `cargo check` exit 0, placeholder profile seeded, `bid_workflow_states` empty.

### 16:17Z — Proof A: live portal cap-respect (read-only)
- `agent-browser connect 9222` → ✓; `agent-browser open https://procontract.due-north.com/` → redirected to `/Login` (title "Log In").
- **Finding:** the operator's persistent Chrome session has expired since 2026-05-25. The portal correctly requires authentication (read-only observation).
- Per bright lines §2.3/§2.4 I did **not** manually sign in or click — the daemon orchestrates auth on `AUTH_STATE_STALE`; an agent must not "help" by clicking. No mutating portal action attempted.
- Screenshot: `screenshots/step-01-portal-auth-gate.webp` (portal reachable + auth-gated).
- **Live cap-respect at the register-interest gate** was fully captured on **2026-05-25** (`../2026-05-25-smoke-test.md`): opportunity DN815582 "PPE and Consumables" (South Hams DC, advertId `7080510a-…`) viewed read-only; the only available action was "Register interest" = `NotificationOrLock`, which **exceeds** the `RemoteDraft` cap, so the skill stopped with `BID_SIDE_EFFECT_CAP_EXCEEDED` and no buyer-visible action. That is the canonical Proof A result; today's session confirms the portal is still live and auth-gated.
- Cap enforcement is independently proven by the test suite (`side_effect_cap` 11/11, incl. `…005_remote_draft_cap_allows_draft_but_blocks_submit`, `…004_read_only_cap_blocks_remote_form_mutation`).

### 14:19Z — Proof B: draft_saved mechanism (live daemon + test suite)
- **Live daemon readiness-gate enforcement:** migrated a fresh temp `state.db` (`daemon --dry-run --state-root <tmp>`), inserted a SMOKE-TEST `run_bid_workflow` job (process `SMOKE-TEST-process-0601`, cap `REMOTE_DRAFT`, `reviewRequired=true`), ran the real `daemon --run-bid-workflow`. The gate refused with 7 structured blockers (PROCESS_NOT_FOUND, OPERATOR_NOT_APPROVED, DECISION_BASIS_STALE, L5_READINESS_PROJECTION_MISSING, COMPANY_PROFILE_OUT_OF_DATE, QUOTE_ROLE_FLOOR_ONLY[overrideable], UPSTREAM_TRUST_UNVALIDATED). See `run-manifest.md` §A / `sql-evidence.md`. Per §11 a blocked manual trigger printing blockers is a valid invariant-enforcement proof. (Run against a temp DB — production state.db untouched.)
- **Authoritative test suite (real daemon binary, no mocks):** state_machine 35/35, side_effect_cap 11/11, bid_runner_body 33/33, bid_run_manifest 2/2, cli_bid_workflow 31/31, ipc_trigger_bid_workflow 36/36, TS bid-workflow-states + portal-side-effects 46/46. These prove `BidInProgress→DraftSaved` with evidence guards, the RemoteDraft cap (allows draft / blocks submit), the runner's `draft_saved` state-write + run manifest, and the desktop Start Bid → IPC trigger path.

### Decision: do NOT fabricate ~80 columns of synthetic upstream state on the production remote-owned tables
The 2026-05-25 session judged the full synthetic-state path too heavy and a mixed-write violation. The test fixtures already drive the identical SQL path through the real daemon binary with synthetic data, so that is the chosen, honest Proof B mechanism (handoff §10-7.2 Proof B(ii) — explicitly sanctioned).

## Evidence index
- `run-manifest.md` — hybrid run manifest + daemon evidence appendix.
- `sql-evidence.md` — production `bid_workflow_states` (empty) + live blocked-gate output + fixture state-write proof.
- `supabase-evidence.md` — projection mechanism + smoke-test status.
- `screenshots/step-01-portal-auth-gate.webp` — live portal reachable + auth-gated.

## Verdict
**GREEN (smoke test).** REMOTE_DRAFT mechanism proven possible+correct: cap-respect proven live (2026-05-25 register-interest stop + today's auth-gate) and by test suite; `draft_saved` mechanism proven by the real daemon binary across state machine, runner, CLI, and IPC paths; readiness gate proven to enforce its invariants live. No real draft on Procontract — deferred per operator instruction 2026-06-01. `L6-T-live_demo-002` satisfied on a smoke-test basis (mechanism + evidence captured; real portal draft_saved row deferred).
