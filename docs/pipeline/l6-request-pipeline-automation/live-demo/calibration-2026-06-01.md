# L6 Calibration Corpus Execution — 2026-06-01

> Bead `testproject1-3yqf.7.4` (l6-p7-calibration-corpus-execution) · Owner TealHarbor.
> Replay/cassette harness only — **no live portal refresh** (AC: live refresh prohibited without separate operator auth; none performed).

## Corpora executed
| Tier | row_count | minimum | replay test | result |
|---|---|---|---|---|
| simple  | 12 | 6  | `e2e_can_016_bid_portal_simple_corpus_replays_every_canary_case`  | ✅ pass |
| medium  | 8  | 8  | `e2e_can_017_bid_portal_medium_corpus_replays_every_canary_case`  | ✅ pass |
| complex | 10 | 10 | `e2e_can_018_bid_portal_complex_corpus_replays_every_canary_case` | ✅ pass |

Total = **30 corpus canary rows** (≥ 24 AC). Provenance: deterministic L6 bid-portal canary shell using Procontract cassette material (`source_structure_version: procontract-bid-portal-2026-05-22`).

## Test execution
- `cargo test --test skill_canary_harness` → **20 passed / 0 failed**. Bid-portal cases:
  - `e2e_can_010_bid_portal_canary_constants_cover_l6_error_codes`
  - `e2e_can_011_bid_portal_auth_stale_canary_replays_session_expired_cassette`
  - `e2e_can_012_bid_portal_opportunity_lookup_canaries_cover_verified_lookup_duplicate_and_drift`
  - `e2e_can_013_bid_portal_form_fill_and_upload_canaries_cover_remote_draft_hash_checks`
  - `e2e_can_014_bid_portal_draft_and_submit_paths_cover_terminal_side_effects`
  - `e2e_can_015_bid_portal_calibration_corpus_contract_covers_required_variants`
  - `e2e_can_016/017/018` corpus replays (above).
- `cargo test --test skill_golden_journeys` → **32 passed / 0 failed**, including the promotion-lifecycle gate:
  - `l3_t_skill_golden_journeys_009_bid_portal_promotion_lifecycle_requires_operator_authorization` ✅ — bid skill stays at `promotion_stage: draft`; promotion to `candidate` requires operator authorization.

## Required scenarios covered (per simple/metadata.json)
`happy_draft, missing_company_profile, stale_auth, duplicate_draft, cap_breach, portal_reference_not_found` + regression scenarios `duplicate_submission, portal_shape_drift, form_fill_upload, upload_hash_mismatch, draft_saved_terminal, formal_submission_terminal`.

## Verdicts vs golden.assertions.yaml
All canary cases produced verdicts matching `golden.assertions.yaml` (the replay tests assert exactly this and pass). No drift detected; no fixture mutation required.

## Regression check
No silent regression vs the Phase-4 baseline — `skill_drift_monitor` + the full canary/golden suites are green. No fixture metadata changes were needed (corpora already meet their contracts), so `tests/fixtures/calibration/bid-portal-procontract/{simple,medium,complex}/metadata.json` are left unmodified.

## Verdict
**GREEN.** 30 corpus canaries replayed, all verdicts match golden assertions, promotion-lifecycle gate enforced, no regression. Calibration uses replay/cassette inputs only — no live portal refresh.
