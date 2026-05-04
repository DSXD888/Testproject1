# Stage 14 Phase 2 — Live Operational Validation Manifest

> Generated: 2026-05-04T00:33:50Z
> Status: GREEN

## Summary

All 9 live desktop demo steps and the cross-session staleness E2E were walked against `state/state.db` with the Tauri desktop app and daemon. The run used real Stage 11/12/13 reruns, captured mandatory screenshots, verified DB ground truth after each mutation, fixed live bugs surfaced during the walk, and finished with 7 manifests, 17 `review` net-margin verdicts, and the original Phase 1 DGU demo intents unchanged (`dec-2026-05-01-dgu-001=superseded`, `dec-2026-05-01-dgu-002=withdrawn`). Final demo-created DGU and Aggregates decisions were withdrawn through the UI; final active overrides are DGU `qty=300 square meter` and Aggregates `qty=500 kg`.

## Step-by-step screenshots

| Step | Screenshot | Result |
|---|---|---|
| 1 — Inbox loads | docs/pipeline/stage-14-phase-2-screenshots/step-01-inbox.png | PASS |
| 2 — DGU detail | docs/pipeline/stage-14-phase-2-screenshots/step-02-detail-DGU.png | PASS |
| 3 — DGU clarification form | docs/pipeline/stage-14-phase-2-screenshots/step-03-clarification-DGU.png | PASS |
| 4 — DGU clarification submitted | docs/pipeline/stage-14-phase-2-screenshots/step-04-clarification-submit.png | PASS |
| 5 — DGU rerun progress | docs/pipeline/stage-14-phase-2-screenshots/step-05-rerun-progress.png | PASS |
| 6 — DGU decision captured | docs/pipeline/stage-14-phase-2-screenshots/step-06-decision-captured.png | PASS |
| 7 — Aggregates detail | docs/pipeline/stage-14-phase-2-screenshots/step-07-aggregates-detail.png | PASS |
| 7 — Aggregates clarification | docs/pipeline/stage-14-phase-2-screenshots/step-07-aggregates-clarification.png | PASS |
| 8 — Aggregates watch captured | docs/pipeline/stage-14-phase-2-screenshots/step-08-aggregates-watch-captured.png | PASS |
| 9 — Aggregates watch withdrawn | docs/pipeline/stage-14-phase-2-screenshots/step-09-aggregates-watch-withdrawn.png | PASS |
| Staleness S.5 — Inbox stale indicator | docs/pipeline/stage-14-phase-2-screenshots/step-10-stale-inbox.png | PASS |
| Staleness S.6 — Detail stale banner | docs/pipeline/stage-14-phase-2-screenshots/step-11-stale-detail.png | PASS |
| Staleness S.8 — Post-withdrawal | docs/pipeline/stage-14-phase-2-screenshots/step-12-post-withdrawal.png | PASS |

## Bugs surfaced + fixed

| File | Bug | Fix | Regression test |
|---|---|---|---|
| `crates/daemon/src/main.rs` | Operator quantity overrides against generic `expected_value` revenue could distort Stage 12 verdicts. | Treat operator override + generic expected-value revenue as an unresolved revenue-basis clarification unless RFQ revenue has been synthesized. | `bug_operator_quantity_override_expected_value_regression_remains_review_when_revenue_basis_unaligned`; `operator_rfq_override_expected_value_revenue_can_pass_when_basis_is_synthesized` |
| `crates/daemon/src/main.rs` | Low-confidence high-reject candidates could reject instead of staying review. | Require confidence floors before high-reject can become reject. | `bug_low_confidence_high_reject_regression_remains_review_until_confidence_clear` |
| `crates/daemon/src/main.rs` | Stage 13 skill renderer mode could fail hard on malformed skill JSON. | Default renderer mode is `auto`; invalid skill output falls back to daemon renderer. | `bug_stage13_invalid_json_regression_default_renderer_mode_uses_auto_fallback` |
| `apps/desktop/src-tauri/src/query_service.rs` | `net_qualified` / `manifest_ready` pipeline states rendered as UI error. | Map both states to `PipelineState::Qualified`. | `bug_pipeline_state_regression_net_qualified_is_not_rendered_as_error` |
| `apps/desktop/src-tauri/src/main.rs` | `trigger_pipeline_rerun` used an unsupported queue path instead of running Stage 11/12/13. | Implement live `landed-cost -> net-gate -> manifest` rerun chain with restart around the app-owned daemon lock. | `bug_pipeline_rerun_regression_runs_live_ingest_chain_instead_of_unsupported_queue_job`; `bug_pipeline_rerun_regression_releases_desktop_daemon_lock_before_ingest` |
| `apps/desktop/src-tauri/src/main.rs` | Stale descriptors and zombie daemon PIDs blocked rerun recovery. | Reset launch guard, detect defunct PIDs, prefer sibling `target/debug/daemon`, and restart after stale descriptor. | `bug_pipeline_rerun_regression_defunct_daemon_pid_is_not_running`; `bug_pipeline_rerun_regression_restarts_after_stale_descriptor` |
| `apps/desktop/src-tauri/src/main.rs` | After the required external daemon run, UI mutation commands could hit a stale socket and fail with connection refused. | General daemon bridge calls now restart the daemon once on `transport_unavailable` before retrying the command. | `bug_bridge_restart_regression_recovers_from_stale_daemon_socket` |
| `apps/desktop/src/pages/OperatorDecisionPanel.tsx` | Rerun status did not reliably paint before native invoke started. | Defer invoke briefly and keep the progress surface visible with fixed overlay styling. | `OperatorDecisionPanel.test.tsx` FT.9 |

## R.1–R.12 status

| # | Criterion | Status | Evidence |
|---|---|---|---|
| R.1 | `bunx tauri dev` launches and window opens | PASS | Live app launched; subsequent hot reload after Rust fixes completed in about 10-11s. |
| R.2 | Inbox renders all 7 live manifests | PASS | `step-01-inbox.png`; DB `SELECT COUNT(*) FROM manifests` = 7. |
| R.3 | DGU detail renders markdown, Quick-Decide, clarification, history | PASS | `step-02-detail-DGU.png`, `step-03-clarification-DGU.png`; DGU history showed prior `superseded` and `withdrawn` intents. |
| R.4 | DGU clarification writes override | PASS | UI submitted `qty=150`; later external stale test moved active override to `qty=300` while `qty=150` was superseded. |
| R.5 | Re-run pipeline runs progress and refreshes basis | PASS | `step-05-rerun-progress.png`; DGU basis moved through `e485156...` and then `19c354...` after live reruns. |
| R.6 | DGU decision writes queued row with note | PASS | `decision-approve-453b7c21-097b-4c1c-8c00-53a2618aee15` initially queued with note `DGU at 150 m²...`; later withdrawn after stale validation. |
| R.7 | Aggregates can be clarified, rerun, decided watch | PASS | Active Aggregates override `qty=500 kg`; `decision-watch-c2a24988-92e3-42b1-966f-15bdeeae485f` captured with watch note and later withdrawn. |
| R.8 | Stale indicator/banner appears after out-of-band basis change | PASS | DB moved DGU decision to `stale`; `step-10-stale-inbox.png` and `step-11-stale-detail.png` show UI stale state/banner. |
| R.9 | Withdraw via UI flips status to withdrawn | PASS | DGU and Aggregates withdrawals completed through UI; DB statuses are `withdrawn`. |
| R.10 | Required screenshots committed | PASS | 13 step/staleness screenshots present under `docs/pipeline/stage-14-phase-2-screenshots/`. |
| R.11 | Stage 11/12/13/14 invariants intact | PASS | 17 net-margin verdicts still `review`; 7 manifests present; original DGU demo intents untouched. |
| R.12 | Quality gates pass | PASS | See Quality Gates. |

## Quality Gates

| Gate | Result |
|---|---|
| `cargo fmt --check` | PASS |
| `cargo check --all-targets` | PASS |
| `cargo clippy --all-targets -- -D warnings` | PASS |
| `cargo test -p daemon --bin daemon` | PASS — 180 / 180 |
| `cargo test -p procintel-desktop --test query_service` | PASS — 25 / 25 |
| `cargo test -p procintel-desktop --test daemon_bridge` | PASS — 14 / 14 |
| `cargo test -p procintel-desktop --test native_shell` | PASS — 22 / 22 |
| `for t in crates/daemon/tests/*.rs; do cargo test --test "$(basename "$t" .rs)" || exit 1; done` | PASS — all daemon integration targets |
| `cd apps/desktop && bunx vitest run` | PASS — 43 / 43 |

## Final DB Evidence

| Check | Result |
|---|---|
| Manifest count | 7 |
| Net-margin verdicts | `review=17` |
| DGU final manifest hash | `19c35450955f15c2158bf9444da2cec1a01891bb1b9bfd011e0e9c6956ed66fb` |
| Aggregates final manifest hash | `517d6d35153d7d3a8ebaaeccdaa7b60f49a8dc78f3651b23b624d85c6345bb30` |
| Original DGU demo intents | `dec-2026-05-01-dgu-001=superseded`, `dec-2026-05-01-dgu-002=withdrawn` |

## Out-of-scope items (filed as beads)

None.
