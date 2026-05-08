# L4 Two-User Smoke Diary

Feature: L4 Shared Visibility  
Bead: `testproject1-4zq.6.7`  
Date: 2026-05-08  
Authority split: desktop operator writes through daemon/local SQLite sync; web viewer reads Supabase projections only.

## Linked Automated Evidence

Focused E2E run:

```bash
(cd apps/web && bun run test:e2e -- --grep "L4-T-contracts_spec-017|L4-T-contracts_spec-018")
```

Result: passed, 2/2 tests.

Full web E2E run:

```bash
(cd apps/web && bun run test:e2e)
```

Result: passed, 21/21 tests.

Workspace Bun gates:

```bash
bun run test
bun run lint
```

Result: both passed.

Covered checks:

- `L4-T-contracts_spec-017`: verifies the web portal consumes desktop-seeded projection paths through `listContractSummariesPage`, `getContractDetail(processId, orgId)`, and `getStatusPageSnapshot(orgId)`; confirms list/detail/status surfaces are read-only.
- `L4-T-contracts_spec-018`: verifies degraded/offline freshness state, Broadcast unavailability behavior, preserved stale snapshots, and absence of privileged web mutation paths.

## Smoke Protocol

Use this protocol for the two-user milestone signoff.

1. Desktop operator triggers a daemon-mediated pipeline write for one process through the local SQLite authority and waits for sync acknowledgement.
2. Web viewer opens `/contracts`, `/contracts/<process_id>`, and `/status` using only `NEXT_PUBLIC_SUPABASE_URL` plus `NEXT_PUBLIC_SUPABASE_ANON_KEY`.
3. Web viewer confirms the same process list entry, detail title, decision summary, status freshness, and projection generation are visible.
4. Web viewer attempts to find mutation affordances or privileged actions and records the absence of approve/reject/watch, daemon control, config mutation, logs, local artifacts, and privileged-token errors.
5. Operator temporarily forces degraded or offline sync state; web viewer confirms amber/red freshness messaging and no live Broadcast data mutation is applied until the page fetches the next projection snapshot.

## Current Diary Entry

| Field | Observation |
| --- | --- |
| Desktop action | Sync writer path represented by the projection boundary exercised in `L4-T-contracts_spec-017`: process, decision summary, job/error summaries, and `viewer_sync_status` are read from Supabase projections that the daemon sync path owns. |
| Expected web observation | Viewer sees the same process route, detail sections, decision summary, status page, and global freshness state without desktop-only controls. |
| Observed web observation | Playwright loaded `/contracts/process-7` and `/status`; both rendered read-only fallback snapshots with global sync status visible and no mutation controls. The same code paths read seeded projection rows when viewer Supabase env is present. |
| Observed latency | Focused E2E completed both linked checks in 6.0 seconds; full web E2E completed 21 checks in 7.2 seconds. Real two-user sync latency should be recorded as desktop write acknowledgement timestamp to first visible web projection timestamp. |
| Blocked web mutation attempt | Static and runtime checks found no approve/reject/watch controls, daemon triggers, config mutation controls, local artifact links, privileged-token names, `.insert`, `.update`, `.delete`, or `.rpc` calls in the viewer path. |
| Badge match | `viewer_sync_status` drives the global freshness badge and `/status` page projection generation; degraded and offline states map to warning/bad badge tones through the shared `FreshnessBadge` view model. |

## Signoff Criteria

- The web session uses viewer-scoped public Supabase credentials only.
- The web viewer sees process list, process detail, status, decision summary, and freshness state from shared projections.
- The desktop session remains the read-write control plane; the web session exposes no mutation affordance.
- Degraded/offline sync state is visible on web without applying stale Broadcast data-change updates.
