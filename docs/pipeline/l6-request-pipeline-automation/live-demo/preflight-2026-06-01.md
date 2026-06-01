# L6 Live Demo Preflight — 2026-06-01

> **SMOKE TEST / PRACTICE MODE.** Synthetic data + placeholder profile. No real bid placed.
> Operator standing authorization (2026-06-01): close 7.2/7.3 on a smoke-test basis; real legally-binding submission deferred.
> Bead: `testproject1-3yqf.7.1` (l6-p7-preflight-evidence) · Owner: TealHarbor · Test owned: `L6-T-live_demo-001`.

This preflight captures the environment + prerequisites for the Phase-7 smoke-test closure. Unlike the original `preflight-2026-05-23.md`, this run executes under the **smoke-test reframe**: the target is to prove each bid mechanism is *possible and correct* with clearly-labeled synthetic data, not to place a real bid on a real opportunity.

## 1. CDP Health — GREEN
- Command: `bin/ensure-cdp-9222`
- Output: `{"helper":"ensure-cdp-9222","status":"launched","port":9222,"message":"Chrome launched with pid 59164; CDP reachable after 8s","ts":"2026-06-01T14:08:01Z"}`
- Verification: `curl -sfS http://localhost:9222/json/version` → `"Browser": "Chrome/148.0.7778.179"`, `"Protocol-Version": "1.3"`.
- Profile path: `/Users/alinaaitova/.chrome-automation` (operator's persistent profile).
- Saved credential `sales@sorellon.com` (double-L) and Touch-ID-disabled state: operator-confirmed baseline 2026-05-21, re-confirmed by successful sign-in during the 2026-05-25 session (`2026-05-25-smoke-test.md`). Not re-screenshotted this session; the live cap-respect proof in 7.2 will re-exercise the signed-in portal read path.

## 2. AgentMail Health — GREEN
- Endpoint: `http://127.0.0.1:8765/health`
- Response: `{"status":"ready","version":"0.2.51","database_path":"storage.sqlite3","project_count":77,"message_count":993,"durability_state":"healthy"}`
- Session registered as `TealHarbor` (program `l6-request-pipeline-automation-executor`, model `claude-opus-4-8`). All other registered agents last active 2026-05-22/23 → effectively solo; `bin/br-claim --solo` used (sanctioned per AGENTS.md Rule 2 / handoff §8).

## 3. Supabase / PostgREST — GREEN
- `colima` + local Supabase stack already running from the 2026-05-25 bootstrap (persists across this session).
- `curl -s -o /dev/null -w "%{http_code}" http://localhost:54321/rest/v1/` → `200`.

## 4. Daemon + state.db — GREEN
- `cargo check -p daemon --all-targets` → `Finished` (exit 0), no errors.
- Daemon HEAD: `4a4accc` (docs(l6): make Phase 7 handoff fully autonomous + self-healing).
- Real daemon DB: `~/Library/Application Support/com.procintel.desktop/state/state.db` (NOT the `./state/state.db` stub).
- Schema current — present tables include `bid_workflow_states`, `validation_verdicts`, `shared_company_profile`, `operator_decision_heads`, `shared_source_configs`.
- **No real bid state exists:** `SELECT bid_state,bid_job_id FROM bid_workflow_states;` → 0 rows. Clean slate (`not_started`).

## 5. L5 Readiness Fixture — GREEN (gate is producible)
- `bun test packages/bidding/tests/bid-readiness-gate.test.ts` → **16 pass, 0 fail** (30 expect() calls, 129 ms).
- Confirms the L5→L6 readiness gate is producible and enforces its blocker semantics; the bid-workflow runner's readiness check (`readiness_gate.rs`) is the live counterpart.

## 6. Company Profile — GREEN (smoke-test placeholder, clearly labeled)
- `shared_company_profile`: id `5f87c23d-8b23-4b28-85a0-9548c1a02787`, org_id `sorellon`, company_name `Sorellon Limited`, registration_number `16575318`, `remote_revision = 1`.
- `bid_profile_json` (verified):
  - certifications: `SMOKE_TEST_PLACEHOLDER_Cyber Essentials` / `PLACEHOLDER-NOT-REAL-2026` / issuer "IASME (PLACEHOLDER - not a real certificate)" / expiry 2027-12-31.
  - insurancePolicies: `SMOKE_TEST_PLACEHOLDER_public_liability` / provider "PLACEHOLDER - no real insurer yet" / £1,000,000 cover / expiry 2027-12-31.
  - smeStatus: `micro`.
- `companyProfileVersion` will be supplied as the UUID `5f87c23d-8b23-4b28-85a0-9548c1a02787` (or the stringified `remote_revision` `"1"`).
- VAT genuinely NULL (Sorellon not yet VAT-registered per `sorellon.md`) — consistent with a pre-revenue org; placeholder is acceptable for the smoke test per operator feedback.

## Synthetic Scope (fabricated for the smoke test — clearly labeled)
Verified `BidScopePayload` shape (`crates/daemon/src/bid_workflow/types.rs:11-23`): `processId`, `lotIds[]`, `candidateIds[]`, `selectedQuoteIds[]`, `companyProfileVersion`, `bidPortalType`, `portalOpportunityRef`, `reviewRequired`, `decisionBasisHash`, `sideEffectCap`. `sideEffectCap` is serialized SCREAMING_SNAKE_CASE; `DEFAULT_SIDE_EFFECT_CAP = RemoteDraft`, `DEFAULT_REVIEW_REQUIRED = true`, `DECISION_BASIS_HASH_LENGTH = 64`.

```json
{
  "bidPortalType": "procontract",
  "processId": "SMOKE-TEST-<uuid>",
  "lotIds": ["SMOKE-TEST-lot-1"],
  "candidateIds": ["SMOKE-TEST-cand-1"],
  "selectedQuoteIds": ["SMOKE-TEST-quote-1"],
  "portalOpportunityRef": "SMOKE-TEST-DN000000",
  "decisionBasisHash": "<64-hex; daemon computes/validates at run time>",
  "sideEffectCap": "REMOTE_DRAFT",
  "companyProfileVersion": "5f87c23d-8b23-4b28-85a0-9548c1a02787",
  "reviewRequired": true
}
```

## Operator-Selected Read-Only Reference Opportunity
- For the *live read-only* cap-respect proof (7.2 Proof A) the reference opportunity is **DN815582 "PPE and Consumables", South Hams District Council** (advertId `7080510a-e94a-f111-813c-005056b64545`), selected and viewed read-only in the 2026-05-25 session. CPV `18140000-2 / 35113000-9 / 39830000-9` — exact fit for Sorellon's category.
- Duplicate-state check (2026-05-25): no existing submission/draft for `sales@sorellon.com`; only "Register interest" available.
- Under `sideEffectCap = REMOTE_DRAFT`, "Register interest" (= `NotificationOrLock`) **exceeds** the cap, so the skill correctly refuses → no buyer-visible legal commitment. Operator confirmed (standing authorization 2026-06-01).
- The bid *mechanism* proofs (draft_saved, bid_submitted) run against fully **synthetic** scope, NOT this real opportunity.

## Authorization Scope
- First live-portal run authorization scope: exactly `sideEffectCap = REMOTE_DRAFT`.
- FORMAL_SUBMISSION: operator standing authorization (2026-06-01) permits closing 7.3 on a **synthetic/stubbed** basis. A *real* legally-binding `FormalSubmission`/`FundsCommitted` on the live account remains a constitutional bright line and is explicitly deferred to a future, separately-authorized session.

## Verdict
**GREEN** — all 6 preflight sections pass. Environment is live (CDP, AgentMail, Supabase, daemon), the readiness gate is producible, the placeholder profile is seeded and clearly labeled, and no real bid state exists. Safe to proceed to the REMOTE_DRAFT smoke test (7.2). `L6-T-live_demo-001` satisfied: opportunity reference recorded, REMOTE_DRAFT confirmed to create no buyer-visible legal commitment, profile/version prerequisites captured.
