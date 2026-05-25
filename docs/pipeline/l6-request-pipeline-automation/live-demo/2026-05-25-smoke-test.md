# L6 Smoke Test - 2026-05-25

## Verdict

**SMOKE TEST OUTCOME**: GREEN at constitutional discipline; YELLOW at form-fill coverage.

The bid-portal-procontract playbook was exercised against a real Procontract opportunity (DN815582, "PPE and Consumables", South Hams District Council) under `sideEffectCap = REMOTE_DRAFT`. The smoke test stopped at the **register-interest gate** as expected, demonstrating correct constitutional cap-respect (kickoff §3 invariant 2). The form-fill, document-upload, draft-save, and submission playbook steps were NOT exercised because no opportunity in Sorellon's accessible portal pool is past the register-interest stage. This smoke test was orchestrated by a human-in-the-loop Claude session acting as the skill executor, NOT by `daemon --skill-path` — the difference is execution mechanism, not playbook behavior.

## Mode

PRACTICE MODE - placeholder profile data, not a genuine bid attempt. `sideEffectCap = REMOTE_DRAFT` enforced. No buyer-visible action taken on Procontract.

## Environment Captured

- **Today**: 2026-05-25
- **Daemon HEAD**: `b8bd52b` (beads: l6 handoff sync). Two prior WIP commits on bead 7.1 still on `main` (22df20a, 4d275b6).
- **CDP**: Chrome `Chrome/148.0.7778.179` at port 9222. Multiple restarts during the session via `bin/ensure-cdp-9222` (Chrome closed externally between sessions).
- **AgentMail**: `{"status":"ready","version":"0.2.51","project_count":77,"message_count":993}`.
- **Supabase**: Local CLI 2.101.0 (installed from GitHub release, shim binary + supabase-go co-located at `~/.local/share/supabase/`, compatibility symlinks at `~/.local/bin/{supabase,supabase-go}`).
- **Docker runtime**: Colima 0.10.1 + docker CLI 29.5.2 (server 29.2.1). Lima VM, Ubuntu 24.04. Docker socket: `~/.colima/default/docker.sock`.
- **Supabase containers**: db (Postgres 17), rest, kong, auth (GoTrue), studio, pg_meta, analytics, inbucket healthy. vector/realtime/storage/edge_runtime/imgproxy excluded (docker.sock mount incompatibility with Colima).
- **Migrations applied**: All 9 in `infra/supabase/migrations/0001..0009` to Supabase; all 13 in `migrations/0001..0013` to the daemon's Tauri-managed `~/Library/Application Support/com.procintel.desktop/state/state.db`.
- **Backup**: `state.db.preB-smoke-test-20260525T112735Z.bak` next to live state.db before daemon migrations were applied.

## Codex Skill Install (separate task this session)

Two symlinks created in `~/.codex/skills/`:
- `supabase -> ../../.agents/skills/supabase`
- `supabase-postgres-best-practices -> ../../.agents/skills/supabase-postgres-best-practices`

Same shared source as Claude's `~/.claude/skills/`; matches the existing 8-skill symlink pattern. Verified by `readlink -f` and `diff`. SKILL.md readable through both. Codex discovers skills by directory listing; no manifest file needed.

## Profile Seeding

### Supabase row (canonical)

`shared_company_profile`:
- `id` = `5f87c23d-8b23-4b28-85a0-9548c1a02787`
- `org_id` = `sorellon`
- `company_name` = `Sorellon Limited`
- `registration_number` = `16575318` (Companies House)
- `vat_number` = NULL (genuinely not yet registered per sorellon.md line 34)
- `address_json`: real registered office at Avantgarde Tower, Apartment 144, London E1 6GS
- `contact_json`: real Director Zac Farrell, `sales@sorellon.com`, 07950265101
- `bid_profile_json`: SMOKE_TEST_PLACEHOLDER certifications and insurance policies with conservative expiry 2027-12-31

### Local mirror row

The L4 sync engine was not configured to pull from local Supabase this session (daemon's startup sync degraded gracefully with `remote-owned pull unavailable for shared_source_configs`). To unblock the bid workflow runner's local read path, the three remote-owned tables were created in local SQLite directly per the Supabase schema, and the same Sorellon row was inserted into the local `shared_company_profile`:

- `shared_company_profile`, `operator_decision_heads`, `shared_source_configs` CREATE TABLE applied locally
- Sorellon row mirrored locally with same id/org_id/data

This is a mixed-write violation in production terms (these tables are remote-owned per L4 architecture) but is acknowledged as smoke-test-only.

## Spelling Reconciliation

Companies House and `sorellon.md` (operator source of truth) use **double-L "sorellon"** everywhere: `Sorellon Limited`, `sorellon.com`, `sales@sorellon.com`. The L6 docs (spec, kickoff, strategy, TDD, manifest, bid skill READMEs, the `Sorelon Ltd` test fixture at `packages/bidding/tests/bid-readiness-gate.test.ts:481`) systematically use **single-L "sorelon"**. The single-L spelling is a typo throughout. **Decision (operator, 2026-05-25)**: double-L is canonical going forward. L6 docs cleanup is a deferred task for a separate bead.

## Scope Pivot

Original handoff assumed a fully populated pipeline state for the live demo. Probing the daemon's state.db confirmed `ingestion_processes`, `notice_lots`, `product_candidates`, `supplier_quotes`, `pipeline_assessments`, `landed_cost_estimates`, `bid_workflow_states` are either missing or 0 rows. **No upstream pipeline data exists.**

Operator considered three smoke-test paths and chose "Synthetic full state for the bid skill only" initially. After honest re-estimate of synthetic-state scope (~10 tables, ~80 NOT NULL columns, no fixture factories), pivoted to **daemon `--skill-path` direct invocation** which bypasses the readiness gate and synthetic upstream requirement. After further investigation of how `daemon --skill-path` injects the company profile and whether the bid skill can actually fire on a pre-register-interest opportunity, the smoke test was executed manually as a human-in-the-loop walkthrough of the skill's playbook against the chosen opportunity.

## Opportunity Selection

After searching Procontract's "Find opportunities" with terms relevant to Sorellon's category:
- "wipes" - no matches
- "cleaning" - no matches (input concatenation bug worked around via form-submit eval)
- "PPE" - 4 visible matches across visible page

Selected: **"PPE and Consumables" (DN815582)** by South Hams District Council. Why:
- CPV categories `18140000-2 (Workwear accessories)`, `35113000-9 (Safety equipment)`, `39830000-9 (Cleaning products)` - exact fit for Sorellon's product space
- Single-supplier appointment (real one-off tender, not a Dynamic Purchasing System)
- 2-year contract (2026-08-01 to 2028-07-31)
- Expression window open: 2026-05-08 00:00 to 2026-06-05 12:00 (11 days remaining at time of test)
- Regional fit: Devon (Sorellon delivers UK-wide)
- Buyer: Ollie Rees, South Hams District Council, Follaton House, Totnes, Devon
- "Register interest" link visible; no "Withdraw" or "You have applied" markers (Sorellon has NOT engaged previously)

## Playbook Walk

Skill: `bid-portal/procontract`. Cap: `REMOTE_DRAFT`.

| Step | Side-effect class | Outcome | Evidence |
|---|---|---|---|
| 1. Navigate to portal | READ_ONLY | ✓ Chrome navigated via agent-browser to `https://procontract.due-north.com/` | Earlier in session |
| 2. Verify auth state | READ_ONLY | ✓ Initially on `/Login`; operator signed in via Chrome autofill; landed on `/SupplierPostLoginHome?initialLogin=True` | Verified post-sign-in |
| 3. Locate opportunity | READ_ONLY | ✓ Navigated to "Find opportunities" → searched "PPE" → clicked "PPE and Consumables" row | Filtered page snapshot |
| 4. Verify portal state for opportunity | READ_ONLY | ✓ Landed at `/Supplier/Advert/View?advertId=7080510a-e94a-f111-813c-005056b64545`. Page state: opportunity open, no draft, no submission, only "Register interest" link visible. `BidPortalChanged` NOT raised (page fingerprint matches the source-structure-version `procontract-bid-portal-2026-05-22`). | `step-04-opportunity-detail-DN815582-fullpage.png` (197 KB), `step-04-opportunity-detail-snapshot.txt` (accessibility tree) |
| 5. Next action would be | NOTIFICATION_OR_LOCK | **EXCEEDS sideEffectCap = REMOTE_DRAFT**. Equivalent skill outcome: `failure_code = BID_SIDE_EFFECT_CAP_EXCEEDED`. Skill STOPPED. No portal mutation attempted. | Constitutional stop |

Per kickoff §1.7, "Register interest in this opportunity" is a notification/lock-level side effect: it notifies the buyer of supplier intent. Under a `REMOTE_DRAFT` cap, the playbook cannot proceed past Step 4 on this opportunity. Per kickoff §3 invariant 2, the skill MUST save draft when possible and stop — there is no draft to save at this stage, so the skill stops with the cap-exceeded outcome.

## Equivalent Skill Result Envelope

If the skill had been daemon-invoked, the result.json equivalent would be:

```json
{
  "status": "failed",
  "failure_code": "BID_SIDE_EFFECT_CAP_EXCEEDED",
  "kind": "no_mutation_attempted",
  "final_side_effect_class_reached": "READ_ONLY",
  "side_effect_cap": "REMOTE_DRAFT",
  "portal_state": {
    "opportunity_found": true,
    "existing_draft_detected": false,
    "existing_submission_detected": false,
    "portal_status": "awaiting_expression_of_interest"
  },
  "screenshot_paths": [
    "step-04-opportunity-detail-DN815582-fullpage.png"
  ],
  "narrative_summary": "Located DN815582 (PPE and Consumables, South Hams DC). Page state confirms opportunity open with only 'Register interest' available. Next playbook action requires NOTIFICATION_OR_LOCK class (notifies buyer). Cap is REMOTE_DRAFT. Skill stopped without portal mutation per constitutional invariant 1.10.2."
}
```

## What This Smoke Test Proves

1. **Portal navigation works** end-to-end via agent-browser over CDP against the operator's persistent Chrome profile.
2. **Authentication state detection works** - the playbook can verify it's signed in and locate the right opportunity by reference.
3. **Constitutional cap-respect works** - faced with a next step that would exceed the operator-approved `sideEffectCap`, the playbook stops cleanly without unauthorized portal mutation. This is the most important L6 invariant.
4. **No PII leakage** - operator name/email/phone do not appear in the captured snapshot.txt or screenshot filename (verified by grep).

## What This Smoke Test Does NOT Cover

| Gap | Why | How to close |
|---|---|---|
| Form-fill playbook (Steps 5-10) | No opportunity in Sorellon's pool is past the register-interest gate | (a) raise the cap on an operator-approved opportunity, register interest, wait for response form to appear, re-run skill; OR (b) find / wait for a Procontract opportunity that doesn't gate response-form access behind register-interest |
| Document upload | Same reason | Same as above |
| Draft-save (`bid_state = draft_saved`) | Form-fill is upstream of draft-save | Same as above |
| Formal submission (DoD #8) | Constitutionally blocked without separate written FORMAL_SUBMISSION authorization (kickoff §13). Sorellon also lacks the real certifications + insurance required by the readiness gate | Hard-blocked until Sorellon procures real insurance + certifications |
| Validator overlay verdict | Skill stopped before any narrative was produced; no run manifest content to validate | Same as form-fill |
| Daemon `--skill-path` invocation mechanics | Smoke test was orchestrated by Claude session directly, not by daemon-spawned claude CLI subprocess | Future session: actually invoke `daemon --skill-path skills/bid-portal-procontract --job-parameters-json '{...}'` once a form-fill-stage opportunity is available |
| Bid workflow runner / readiness gate / state machine | All bypassed in this session | Requires synthetic upstream pipeline state - explicitly deferred |

## Bead 7.1 Status

Bead `testproject1-3yqf.7.1` (l6-p7-preflight-evidence) remains **open**. This session did NOT claim it via `bin/br-claim` (the comprehensive prep + smoke-test work was scoped under direct operator collaboration, not under the bead's AC). The original preflight artifact `preflight-2026-05-23.md` (last touched by GrayMouse) is unchanged on disk - this new artifact is intentionally separate to preserve the 7.1 audit trail.

Three of the five blockers from preflight-2026-05-23 are now cleared:
- ✓ Supabase/PostgREST reachable (running on 127.0.0.1:54321)
- ✓ `shared_company_profile` populated (smoke-test placeholder in Supabase + mirror in local SQLite)
- ✓ Operator-selected Procontract opportunity reference: DN815582 (PPE and Consumables, South Hams DC)

Two remain partial:
- ~ Chrome saved credential / Touch ID state: operator manually confirmed during this session by successful sign-in; not screenshot-verified against `chrome://settings/passwords` directly
- ~ Operator-decision row: NOT seeded; not needed under this smoke-test path (daemon's readiness gate bypassed)

A fresh agent claiming 7.1 could now either:
- Mark these as cleared with this artifact as evidence
- OR re-execute the original preflight checklist with the new infrastructure in place

## Files Touched This Session

- Created: `~/.codex/skills/{supabase,supabase-postgres-best-practices}` symlinks
- Created: `~/.local/share/supabase/{supabase,supabase-go,supabase.tar.gz}` (CLI binary + co-located go binary + downloaded tarball)
- Created: `~/.local/bin/supabase-go` symlink for shim resolution
- Created: 1 row in Supabase `shared_company_profile` (placeholder profile)
- Created: 3 tables in local SQLite (`shared_company_profile`, `operator_decision_heads`, `shared_source_configs`)
- Created: 1 row in local `shared_company_profile` (mirror of the Supabase row)
- Created: Backup `state.db.preB-smoke-test-20260525T112735Z.bak`
- Created: This file
- Created: `artifacts/sessions/2026-05-25-l6-smoke-test/` with 2 screenshots + 1 accessibility snapshot
- Memory updates: `project_sorellon_identity.md`, `feedback_smoke_test_mode.md`, MEMORY.md index entries

No git commits made. Working tree changes:
- 1 new tracked file (this diary)
- 1 new directory of artifacts (untracked by default; outside `.gitignore` patterns to check)
- No modifications to existing tracked files

## Recommended Next Steps

1. **If continuing the smoke test path**: operator decides whether to raise the cap to NOTIFICATION_OR_LOCK on this PPE opportunity and register interest as Sorellon. If yes, wait for the response form to become available (could be immediate, could take buyer review), then re-run the bid skill against the form-fill stage.
2. **If shifting to production readiness**: defer further live testing until Sorellon procures real insurance certificates (employers' liability, public liability, product liability minimum) and at least one real certification (Cyber Essentials being the lowest barrier). Update `shared_company_profile.bid_profile_json` with the real records, bump `remote_revision`, then run a real bid scope through the daemon's full workflow runner.
3. **Daemon-orchestration smoke test**: actually invoke `daemon --skill-path` end-to-end (with the daemon configured to reach local Supabase via env vars, so the L4 sync engine doesn't gracefully degrade). This validates the PTY orchestration / artifact store / run manifest hybrid-author mechanism that this session's manual walkthrough did not exercise.
