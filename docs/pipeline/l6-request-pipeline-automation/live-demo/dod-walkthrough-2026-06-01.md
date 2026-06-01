# L6 Definition-of-Done Walkthrough — 2026-06-01

> Bead `testproject1-3yqf.7.6` (l6-p7-dod-walkthrough) · Owner TealHarbor.
> 11 DoD criteria (spec §2.1, `layer-6-request-pipeline-automation.md:682-693`). Test verdicts from the 7.5 sweep (all 60 integration targets `fail=0`; daemon binary 231/231; TS green; e2e desktop 77/77, web 29/29 prod).
> **DoD #8 = PARTIAL** — prepare+submit proven via smoke test; real legally-binding submission deferred per operator instruction 2026-06-01.

| # | FR | Criterion | Proving tests / evidence | Status |
|---|----|-----------|--------------------------|--------|
| 1 | L6-FR-2.1.1 | Bid-portal skill navigates target portal via browser automation | Skill present (CI-L6-008: `skills/bid-portal-procontract/`); `skill_canary_harness` `e2e_can_011/012` (auth-stale replay, opportunity lookup); live read-only nav 2026-05-25 (DN815582) + portal reachable 2026-06-01 | ✅ PASS |
| 2 | L6-FR-2.1.2 | Company profile injected from `shared_company_profile` | `bid_readiness_gate` (profile mapping `readiness_gate.rs:609-617`); seeded profile id `5f87c23d-…`; readiness fixture `bid-readiness-gate.test.ts` 16/16 | ✅ PASS |
| 3 | L6-FR-2.1.3 | Request prep automated (form fill, upload) with side-effect class tracking | `side_effect_cap` 11/11 (`…004/005/006`); `e2e_can_013` (form-fill/upload remote-draft hash checks); `procontract-workflow` TS 4/4 | ✅ PASS |
| 4 | L6-FR-2.1.4 | Submission step automated with screenshot capture at each stage | `bid_runner_body` 33/33 (screenshot trail + manifest); `e2e_can_014` (draft & submit terminal side-effects); mechanism proven, real portal submit deferred (→ #8) | ✅ PASS (mechanism) |
| 5 | L6-FR-2.1.5 | Full terminal output tracing for bid workflow | `observability_telemetry` (incl. `test_error_code_roundtrip`); integration target green | ✅ PASS |
| 6 | L6-FR-2.1.6 | Run manifest generated for submitted request with audit trail | `bid_run_manifest` 2/2 (hybrid narrative + daemon evidence appendix); 7.2 run-manifest.md | ✅ PASS |
| 7 | L6-FR-2.1.7 | Bid failure produces error manifest with portal-specific context | `agent_error_manifest` integration target green (step/screenshot/form-state/side-effect-class) | ✅ PASS |
| 8 | L6-FR-2.1.8 | System can prepare and submit one request on Procontract end-to-end | **Mechanism** proven via smoke test: 7.2 `draft_saved` (state machine + runner + cap-respect) + 7.3 `bid_submitted` synthetic (state machine `…010/016/024/025`, validator fail-closed `validator_overlay_bid` 18/18, synthetic confirmation ref). **Real legally-binding submission NOT performed** — deferred per operator instruction 2026-06-01 (constitutional bright line; portal session logged out; no buyer notified). | ⚠️ **PARTIAL** (smoke-proven; real submission deferred) |
| 9 | L6-FR-2.1.9 | `draft_saved` path works when `reviewRequired = true` | `bid_workflow_state_machine` `…008_in_progress_to_draft_saved_requires_draft_evidence`, `…032_defaults_to_operator_review_and_remote_draft_cap`; 7.2 evidence | ✅ PASS |
| 10 | L6-FR-2.1.10 | L5 readiness gate blocks `ready_to_bid` when active L5 blockers present | `bid_readiness_gate` target green; live `daemon --run-bid-workflow` emitted `L5_READINESS_PROJECTION_MISSING` + 6 other blockers (7.2 run-manifest §A) | ✅ PASS |
| 11 | L6-FR-2.1.11 | Validator rejection or outage blocks `bid_submitted` (fail-closed) | `validator_overlay_bid` 18/18 — `…rejection_blocks_submit`, `…validator_outage_blocks_final_submit_but_draft_save_remains_reachable`, `…structured_override_allows_final_submit` | ✅ PASS |

## Summary
- **10 PASS, 1 PARTIAL (DoD #8), 0 FAIL.**
- DoD #8 is honestly PARTIAL: every mechanism required to prepare and submit end-to-end is proven correct (draft_saved + bid_submitted state machine, fail-closed validator, cap-respect, run manifest, error manifest, screenshot trail), but the final real legally-binding submission on Sorellon's live Procontract account is deferred to a future, separately-authorized session per the operator's 2026-06-01 instruction. This is the only criterion not fully PASS, and it is partial by deliberate operator policy, not by defect.

## Discrepancies flagged (for follow-up)
1. **Desktop e2e script name:** kickoff §8 / bootstrap §11 reference `bun run test:e2e` for `apps/desktop`, but the shipped `apps/desktop/package.json` exposes the Playwright run as **`e2e`** (`bunx playwright test`). Web correctly uses `test:e2e`. Reconcile docs or add a `test:e2e` alias to desktop.
2. **Web e2e webServer:** the web Playwright `webServer` is `bun run dev` (Next dev), which makes page-render e2e tests flaky on slower hosts (lazy per-route compile vs 60s timeout). The identical suite passes 29/29 in 2.9s against a production build. Recommend switching the e2e `webServer` to `next build && next start` (or raising `expect.timeout`).
3. **"sorelon" vs "sorellon" typo** across L6 docs/fixtures (single-L) vs the canonical double-L `Sorellon Limited` (Companies House #16575318). Known systemic typo; cleanup is a deferred doc task.
4. **agent-mail FTS search degraded** server-side this session (search_messages returns empty for all queries; writes healthy) — caused `bin/br-complete`'s Start-mail check to false-negative; worked around by completing Agent-Mail steps manually with the Start mail verified present in the git archive. Recommend `am doctor reconstruct` (with the server stopped) to rebuild the index.
