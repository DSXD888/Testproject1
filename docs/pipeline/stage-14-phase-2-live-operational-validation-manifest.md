# Stage 14 Phase 2 - Live Operational Validation Manifest

> Generated: 2026-05-03T19:37:18Z
> Status: RED

## Summary

The live desktop workflow was driven against `state/state.db` with screenshots for the Manifest Inbox, DGU detail/clarification/decision flow, cross-session stale decision E2E, and Aggregates clarification/decision flow. Multiple live regressions were fixed with regression tests, and all required quality gates now pass. This validation is still RED because R.11 is false in the final live database: the handoff requires 17 net-margin verdicts to remain `review`, but the Aggregates 500 kg override leaves `pass=1, review=16`. The Aggregates path was also only partially native to the fixture because the live manifest exposed one asphalt `no_pricing_signal` prompt; the required `quantity_basis_undetermined` 500 kg prompt was seeded through the daemon CLI before completing the UI evidence.

## Step-by-step Screenshots

| Step | Screenshot | Result |
|---|---|---|
| 1 - Inbox loads | `docs/pipeline/stage-14-phase-2-screenshots/step-01-inbox.png` | PASS - live Inbox rendered 7 process cards. |
| 2 - DGU detail | `docs/pipeline/stage-14-phase-2-screenshots/step-02-detail-DGU.png` | PASS - DGU detail opened with operator controls and history. |
| 3 - DGU clarification form | `docs/pipeline/stage-14-phase-2-screenshots/step-03-clarification-form.png` | PASS - clarification form rendered; live fixture had 1 prompt instead of the handoff's 2-prompt DGU shape. |
| 4 - DGU clarification submitted | `docs/pipeline/stage-14-phase-2-screenshots/step-04-clarification-submitted.png` | PASS - DGU 150 m2 override was recorded and later superseded by the out-of-band 300 m2 staleness input. |
| 5 - DGU rerun progress | `docs/pipeline/stage-14-phase-2-screenshots/step-05-rerun-progress.png` | PARTIAL - UI rerun feedback was captured; the verified Stage 11/12/13 hash refresh required the external CLI chain. |
| 6 - DGU decision captured | `docs/pipeline/stage-14-phase-2-screenshots/step-06-decision-captured.png` | PASS - approve decision captured with note. |
| 6b - DGU queued history | `docs/pipeline/stage-14-phase-2-screenshots/step-06-decision-history-queued.png` | PASS - queued decision history rendered without a false stale banner after the query-service fix. |
| 7 - Aggregates detail | `docs/pipeline/stage-14-phase-2-screenshots/step-07-aggregates-detail.png` | PASS - CPU 8309 / Aggregates process opened. |
| 7b - Aggregates clarification | `docs/pipeline/stage-14-phase-2-screenshots/step-07-aggregates-clarification.png` | PARTIAL - UI showed the active 500 kg override after CLI seeding; the native manifest prompt shape did not match the handoff. |
| 8 - Aggregates rerun | `docs/pipeline/stage-14-phase-2-screenshots/step-08-aggregates-rerun-progress.png` | PARTIAL - desktop reported "Pipeline rerun queued"; follow-up filed for visible stage execution/progress. |
| 8b - Aggregates watch | `docs/pipeline/stage-14-phase-2-screenshots/step-08-aggregates-watch-captured.png` | PASS - `watch` decision captured and visible in history/DB. |
| 9 - Withdraw decision | `docs/pipeline/stage-14-phase-2-screenshots/step-12-post-withdrawal.png` | PASS - stale DGU approval was withdrawn through the UI; accidental Aggregates reject was also withdrawn through UI evidence in `step-09-aggregates-reject-withdrawn.png`. |
| Staleness S.5 - Inbox stale indicator | `docs/pipeline/stage-14-phase-2-screenshots/step-10-stale-inbox.png` | PASS - DGU card showed `Approve (stale)` after out-of-band basis change. |
| Staleness S.6 - Detail stale banner | `docs/pipeline/stage-14-phase-2-screenshots/step-11-stale-detail.png` | PASS - detail view showed the stale decision banner. |
| Staleness S.8 - Post-withdrawal | `docs/pipeline/stage-14-phase-2-screenshots/step-12-post-withdrawal.png` | PASS - stale DGU decision became `withdrawn` and banner cleared. |

## Bugs Surfaced + Fixed

| File | Bug | Fix | Regression test |
|---|---|---|---|
| `crates/daemon/src/main.rs` | Manifest stoplight could prefer `pursue` when a pass candidate coexisted with a re-evaluation-eligible candidate. | Prioritized any re-evaluation-eligible candidate before `pass`/`pursue`; bumped manifest template version. | `bug_mixed_pass_and_re_eval_regression_clarify_precedes_pursue` |
| `apps/desktop/src/pages/OperatorDecisionPanel.tsx` | Active quantity overrides were not merged back into editable prompts, mutation bridge error payloads could look successful, and post-mutation data went stale. | Merged active overrides into prompts, asserted bridge error payloads, and refreshed history/overrides after mutations. | `bug_active_quantity_override_regression_renders_editable_number_prompt`, `bug_panel_mutation_error_regression_throws_bridge_error_payload`, `bug_active_override_refresh_regression_renders_latest_refetched_payload` |
| `apps/desktop/src/hooks/use-control-plane.ts` | An unavailable/read-only bootstrap snapshot could be preserved and hide a later full-control snapshot. | Always apply fresh bootstrap snapshots and call `refresh_bootstrap_snapshot` during refresh. | `bug_unavailable_bootstrap_regression_accepts_later_full_control_snapshot` |
| `apps/desktop/src-tauri/src/daemon_bridge.rs` | Subscription frames could poison command responses, and one-shot subscription replay could be lost by a non-persistent reader. | Kept a persistent buffered reader and cloned writer stream for command/subscription lifecycles. | `bug_buffered_lifecycle_response_regression_preserves_command_response`, `bug_subscription_replay_regression_handles_one_shot_topic_replay` |
| `crates/daemon/src/ipc/handshake.rs` | Long-lived replay subscription sockets starved later desktop commands because the daemon accepts bridge sockets serially. | Added a 1 second idle timeout after subscription replay while preserving immediate unsubscribe on the same socket. | `it_ipc_014_router_active_accepts_subscribe_and_emits_initial_health_payload`, `it_ipc_014_router_accepts_supported_subscriptions` |
| `crates/daemon/tests/ipc_handshake.rs` | Peer UID mismatch test could race into `Broken pipe` by starting the rejecting server before the client hello was flushed. | Added a pre-written client request helper for that rejection path. | `it_ipc_009_peer_uid_mismatch_is_rejected` |
| `apps/desktop/src-tauri/src/query_service.rs` | Fresh decisions could be marked stale because current basis was derived from pipeline assessment basis instead of the manifest basis reviewed by Stage 14. | Detail and Inbox decision status now compare against `manifests.manifest_basis_hash`, with fallback to pipeline basis only when needed. | `bug_stage14_stale_regression_uses_manifest_basis_not_pipeline_basis`, `bug_stage14_stale_regression_marks_manifest_basis_drift_stale` |

## R.1-R.12 Status

| # | Criterion | Status | Evidence |
|---|---|---|---|
| R.1 | `bunx tauri dev` launches and app window opens within 3 minutes; subsequent launches within 30s. | PASS | Desktop launched repeatedly via `bunx tauri dev` with `PROCINTEL_DESKTOP_STATE_ROOT=state`. |
| R.2 | Inbox renders all 7 process cards from live `state.db`; stoplight colors correct. | PASS | `step-01-inbox.png`; final DB query returned 7 manifest rows, all `clarify`. |
| R.3 | DGU detail opens with markdown sections A-F, Quick-Decide, Clarification form, and two prior intents. | PASS | `step-02-detail-DGU.png`, `step-03-clarification-form.png`, `step-06-decision-history-queued.png`; prior DGU intents remain `superseded` and `withdrawn`. |
| R.4 | Clarification submit creates qty=150 override and supersedes prior qty=250. | PASS | DGU override chain contains 250 and 150 superseded rows; latest active is 300 due the required out-of-band staleness step. |
| R.5 | Re-run pipeline button cycles progress and bumps DGU `manifest_basis_hash`. | PARTIAL | DGU basis changed from the 150-run hash to `9579de60...` after external Stage 11/12/13. Desktop rerun feedback did not itself execute the full visible stage chain; `testproject1-50y0` filed. |
| R.6 | Decision capture writes queued row with entered `decision_note`. | PASS | `decision-approve-2c845d2c-851b-41be-9f11-98e271418803` captured with the required reasoning, then became stale and was withdrawn. |
| R.7 | Aggregates opens, qty=500 kg clarification, re-run, decide `watch`, visible in DB. | PARTIAL | Aggregates opened and `decision-watch-77e26ce1-4f1e-434c-b879-f35f55e71773` is `queued`; 500 kg override exists. The quantity override had to be CLI-seeded because the live prompt shape differed. |
| R.8 | Stale banner appears within 2s of out-of-band basis-hash change. | PASS | DGU stale screenshots `step-10-stale-inbox.png` and `step-11-stale-detail.png`; stale DB state was observed before UI withdrawal. |
| R.9 | Withdraw via UI flips intent to `withdrawn`. | PASS | DGU stale approval is now `withdrawn`; accidental Aggregates reject is also `withdrawn` with UI screenshot evidence. |
| R.10 | All 9 demo screenshots plus at least 3 staleness screenshots committed. | PASS | Required screenshots are present under `docs/pipeline/stage-14-phase-2-screenshots/`; additional debug captures are preserved. |
| R.11 | Stage 11/12/13/14 invariants intact. | FAIL | Final DB has 7 manifests and 4 landed-cost rows, but net-margin verdicts are `pass=1, review=16`; expected was 17 review. Seeded DGU demo intents are intact. |
| R.12 | Quality gates and required test suites pass. | PASS | All gates below passed after the final code edits. |

## Quality Gates

| Gate | Result |
|---|---|
| `cargo fmt --check` | PASS |
| `cargo check --all-targets` | PASS |
| `cargo clippy --all-targets -- -D warnings` | PASS |
| `cargo test -p daemon --bin daemon` | PASS - 176/176 |
| `cargo test -p procintel-desktop --test query_service` | PASS - 24/24 |
| `cargo test -p procintel-desktop --test daemon_bridge` | PASS - 14/14 |
| `cd apps/desktop && bunx vitest run` | PASS - 43/43 |
| `for t in crates/daemon/tests/*.rs; do cargo test --test "$(basename "$t" .rs)" || exit 1; done` | PASS - all daemon integration targets |
| `ubs <changed files>` | PASS exit 0; critical issues 0. Remaining warnings are test-pattern and heuristic inventory. |

## Live DB Evidence

| Query | Observed |
|---|---|
| `SELECT COUNT(*) FROM manifests` | 7 |
| `SELECT COUNT(*) FROM landed_cost_estimates` | 4 |
| `SELECT verdict, COUNT(*) FROM candidate_net_margin_assessments GROUP BY verdict` | `pass=1`, `review=16` |
| Seeded DGU intents | `dec-2026-05-01-dgu-001=superseded`; `dec-2026-05-01-dgu-002=withdrawn` |
| Live DGU decision | `decision-approve-2c845d2c-851b-41be-9f11-98e271418803=withdrawn`; note preserved. |
| DGU override chain | active 300 m2; 150 m2 and 250 m2 rows superseded. |
| Aggregates decision chain | `decision-watch-77e26ce1-4f1e-434c-b879-f35f55e71773=queued`; accidental `decision-reject-494a761d-18cb-4f24-a719-0c3ba1251c31=withdrawn`. |
| Aggregates override | active `quantity_basis_undetermined` payload `{"qty":500,"unit":"kg"}`. |

## Out-of-scope Items Filed as Beads

| Bead | Status | Reason |
|---|---|---|
| `testproject1-q63q` | in_progress | Existing fixture-restoration bead remains the blocker for an all-green R.11 run. |
| `testproject1-50y0` | open | Desktop rerun control needs to execute or visibly track the landed-cost -> net-gate -> manifest chain without requiring external CLI commands. |
