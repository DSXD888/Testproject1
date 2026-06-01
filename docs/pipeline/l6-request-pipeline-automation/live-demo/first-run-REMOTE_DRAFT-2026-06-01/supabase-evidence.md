# Supabase Projection Evidence — REMOTE_DRAFT Smoke Test — 2026-06-01

> SMOKE TEST. No real REMOTE_DRAFT run executed, so no `draft_saved` projection row was pushed to Supabase and the web viewer badge was not updated for a real bid. This is correct under the smoke-test mandate.

## Projection mechanism (how it would work for a real run)
1. The daemon writes `bid_workflow_states.bid_state = 'draft_saved'` locally (operational authority).
2. The L4 transactional-outbox push projects the row to Supabase (`bid_workflow_states` projection, shared authority).
3. The web viewing portal reads the viewer-role projection and renders the **"draft saved" badge + timestamp** (`bid-workflow-status-badge.tsx`).

## Smoke-test status
- Local `bid_workflow_states`: empty (no real run) — see `sql-evidence.md`.
- Supabase projection for a Sorellon bid: **none** (nothing to project).
- Web viewer badge: not exercised for a real bid this session.

## Supabase reachability (confirmed live)
- `curl -s -o /dev/null -w "%{http_code}" http://localhost:54321/rest/v1/` → `200` (stack up from the 2026-05-25 bootstrap).
- The placeholder company profile lives in Supabase `shared_company_profile` (id `5f87c23d-8b23-4b28-85a0-9548c1a02787`, org_id `sorellon`) and is mirrored locally — see preflight.

## Why no real projection
Pushing a synthetic `draft_saved` row to the shared Supabase projection would publish a fabricated bid state to the viewer portal — misleading and a mixed-write on remote-owned/projected state. The projection + badge path is proven by the Phase-3/5 test suite and L4 sync tests; exercising it end-to-end for a real Sorellon draft is deferred to the future real-bid session.
