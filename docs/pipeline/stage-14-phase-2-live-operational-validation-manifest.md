# Stage 14 Phase 2 - Live Operational Validation Manifest

> Generated: 2026-05-02T15:17:51Z
> Status: RED

## Summary

The live desktop app launched under `bun run tauri:dev`, rendered the Manifest Inbox and Process Detail views from `state/state.db`, and the DGU cross-session stale-decision path was proven end to end: a queued decision became stale after an out-of-band DGU quantity change and Stage 11/12/13 re-run, the Inbox showed the stale indicator, the detail view showed the stale banner, and withdrawal cleared the banner. Three live-demo regressions were fixed with regression tests. The validation cannot be marked GREEN because the shared live database no longer matches the handoff fixture: it now has 8 manifests instead of 7, net-margin verdicts are `reject=1, review=17` instead of 17 review-only rows, DGU and Aggregates each expose one clarification prompt instead of the requested prompt shapes, and the DGU override path updates the active row in place rather than retaining an active 150 row plus superseded 250 row.

## Step-by-step Screenshots

| Step | Screenshot | Result |
|---|---|---|
| 1 - Inbox loads | docs/pipeline/stage-14-phase-2-screenshots/step-01-inbox.png | FAIL - rendered from live DB, but live DB has 8 process cards, not the required 7. |
| 2 - DGU detail | docs/pipeline/stage-14-phase-2-screenshots/step-02-detail-DGU.png | PASS - DGU detail opened and rendered detail content plus operator panel. |
| 3 - DGU clarification form | docs/pipeline/stage-14-phase-2-screenshots/step-03-clarification-form.png | FAIL - form rendered, but live DGU manifest exposed only `awaiting_rfq_quote`, not the required two prompts. |
| 4 - Manifest body | docs/pipeline/stage-14-phase-2-screenshots/step-04-manifest-body.png | PARTIAL - manifest/detail content rendered; A-F markdown section completeness was not fully proven by screenshot. |
| 5 - Decision history | docs/pipeline/stage-14-phase-2-screenshots/step-05-decision-history.png | PASS - history and active overrides rendered after IPC parameter-wrapping fix. |
| 6 - DGU decision captured | docs/pipeline/stage-14-phase-2-screenshots/step-06-decision-captured.png | PASS - decision appeared in the detail header after capture. |
| 6b - DGU queued history | docs/pipeline/stage-14-phase-2-screenshots/step-06-decision-history-queued.png | PASS - history showed the new queued decision with note. |
| 7 - Aggregates detail | docs/pipeline/stage-14-phase-2-screenshots/step-07-aggregates-detail.png | FAIL - Aggregates opened, but live prompt shape no longer matched the requested `quantity_basis_undetermined` / `qty=500 kg` workflow. |
| Staleness S.5 - Inbox stale indicator | docs/pipeline/stage-14-phase-2-screenshots/step-10-stale-inbox.png | PASS - DGU card showed `Approve (stale)` after external basis-hash change. |
| Staleness S.6 - Detail stale banner | docs/pipeline/stage-14-phase-2-screenshots/step-11-stale-detail.png | PASS - detail banner appeared: "This decision was made against an earlier manifest version. Re-confirm or withdraw." |
| Staleness S.8 - Post-withdrawal | docs/pipeline/stage-14-phase-2-screenshots/step-12-post-withdrawal.png | PASS - stale DGU decision was withdrawn and the banner disappeared. |

## Bugs Surfaced + Fixed

| File | Bug | Fix | Regression test |
|---|---|---|---|
| `apps/desktop/src/lib/transport.ts` | Stage 14 Tauri commands such as `get_decision_history` and `capture_clarification` were invoked without the required `{ params: ... }` wrapper, so live history/override calls failed. | Added all Stage 14 parameter-object commands to `TAURI_PARAMS_WRAPPED_COMMANDS`. | `apps/desktop/src/lib/transport.test.ts` asserts wrapping for `get_decision_history` and `capture_clarification`. |
| `apps/desktop/src-tauri/src/query_service.rs` | Manifest Inbox compared intent `reviewed_basis_hash` against `manifests.manifest_basis_hash`, making fresh queued decisions look stale after manifest re-render even when the decision basis was current. | Joined `pipeline_assessments` and compare against `decision_basis_hash` for inbox decision status. | `bug_manifest_inbox_regression_compares_decision_basis_not_manifest_basis`. |
| `apps/desktop/src/pages/OperatorDecisionPanel.tsx` | Stale decisions showed the stale banner but had no withdraw button; only queued decisions could be withdrawn in history. | Render withdraw action for both `queued` and `stale` history entries. | `bug_stale_decision_withdraw_regression_renders_withdraw_button`. |

## R.1-R.12 Status

| # | Criterion | Status | Evidence |
|---|---|---|---|
| R.1 | `bunx tauri dev` launches and app window opens within 3 minutes; subsequent launches within 30s. | PASS | `bun run tauri:dev` launched the desktop window repeatedly; last launch compiled and opened the Aggregates detail route within the allowed first-launch window. |
| R.2 | Inbox renders all 7 process cards from live `state.db`; stoplight colors correct. | FAIL | Screenshot `step-01-inbox.png`; DB query returned `manifests=8`. |
| R.3 | DGU detail opens with markdown sections A-F, Quick-Decide, Clarification form, and two prior intents. | PARTIAL | DGU detail, operator panel, and prior intents rendered; live DGU prompt data no longer included the required two-prompt form. |
| R.4 | Clarification submit creates active qty=150 override and supersedes prior qty=250. | FAIL | DGU override was changed through daemon CLI because desktop click automation was blocked; final active DGU override is qty=300 after staleness E2E, and the row is updated in place rather than preserving a visible superseded 250 row. |
| R.5 | Re-run pipeline button shows progress modal and bumps DGU `manifest_basis_hash`. | PARTIAL | Stage 11/12/13 re-run was driven externally and DGU basis hash bumped; the UI progress modal was not validated because click automation was blocked. |
| R.6 | Decision capture writes queued row with entered `decision_note`. | PASS | `dec-2026-05-02-dgu-live-001` was created with disposition `approve`, note text, and `queued` status before staleness; later withdrawn in S.8. |
| R.7 | Aggregates opens, qty=500 kg clarification, re-run, decide `watch`, visible in DB. | FAIL | Aggregates opened, but live data exposed a `no_pricing_signal` choice prompt for asphalt rather than the requested quantity-basis prompt. |
| R.8 | Stale banner appears within 2s of out-of-band basis-hash change. | PASS | DB moved DGU live decision to `stale`; `step-10-stale-inbox.png` and `step-11-stale-detail.png` show UI indicators. |
| R.9 | Withdraw via UI flips intent to `withdrawn`. | PARTIAL | The UI now renders the withdraw button for stale decisions; macOS click automation could not activate it, so final withdrawal was completed through daemon CLI and verified in UI/DB. |
| R.10 | All 9 demo screenshots plus at least 3 staleness screenshots committed. | FAIL | Screenshots exist for steps 1-7 and S.5/S.6/S.8; exact step 8 and step 9 UI-action screenshots were not captured because the Aggregates workflow and UI clicking were blocked. |
| R.11 | Stage 11/12/13/14 invariants intact. | FAIL | `manifests=8`; net-margin verdicts are `reject=1, review=17`. Seeded demo intents `dec-2026-05-01-dgu-001` and `dec-2026-05-01-dgu-002` remained `superseded` and `withdrawn`. |
| R.12 | Quality gates and required test suites pass. | PASS | All listed gates passed; counts below. |

## Quality Gates

| Gate | Result |
|---|---|
| `cargo fmt --check` | PASS |
| `cargo check --all-targets` | PASS |
| `cargo clippy --all-targets -- -D warnings` | PASS |
| `cargo test -p daemon --bin daemon` | PASS - 169/169 |
| `cargo test -p procintel-desktop --test query_service` | PASS - 23/23, includes 2 regression tests |
| `cargo test -p procintel-desktop --test daemon_bridge` | PASS - 11/11 |
| `cd apps/desktop && bunx vitest run` | PASS - 37/37 |
| `for t in crates/daemon/tests/*.rs; do cargo test --test "$(basename "$t" .rs)" || exit 1; done` | PASS - all 39 integration test files |
| `ubs <changed files>` | PASS exit 0; warnings were limited to pre-existing/test panic-surface heuristics and generic switch-case heuristics. |

## Live DB Evidence

| Query | Observed |
|---|---|
| `SELECT COUNT(*) FROM manifests` | 8 |
| `SELECT COUNT(*) FROM landed_cost_estimates` | 4 |
| `SELECT verdict, COUNT(*) FROM candidate_net_margin_assessments GROUP BY verdict` | `reject=1`, `review=17` |
| Seeded DGU intents | `dec-2026-05-01-dgu-001=superseded`; `dec-2026-05-01-dgu-002=withdrawn` |
| Live DGU intent | `dec-2026-05-02-dgu-live-001=withdrawn` after stale E2E withdrawal |
| DGU active override | qty=300 after the out-of-band staleness E2E override |

## Coordination Notes

Agent Mail was unavailable for reservations: `health_check` returned `health_level=red` with degraded read-only recovery advice. No file reservations were acquired.

## Out-of-scope Items Filed as Beads

| Bead | Status | Reason |
|---|---|---|
| `testproject1-q63q` | OPEN | Restore or rebuild a deterministic Stage 14 Phase 2 live-validation fixture so the all-green demo can be re-run against the expected 7 manifests, 17 review verdicts, DGU/Aggregates prompt shapes, and override audit chain. |
