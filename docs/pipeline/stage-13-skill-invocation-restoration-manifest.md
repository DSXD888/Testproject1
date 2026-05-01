# Stage 13 Skill Invocation Restoration (Option B) Validation Manifest

> Generated: 2026-05-01T18:08:30Z
> Status: GREEN

## Summary
Option B succeeded. The original stall was a combination of model/profile latency and the child process continuing after `result.json` had already been written; the first diagnostic used the CLI default Opus-class model and spent extra time producing cleanup/run-manifest work after the contract was complete. The fix restores real `manifest-generation` skill invocation, pins Stage 13 to Haiku by default, terminates the child after a stable `result.json`, recovers valid result contracts after SIGTERM exit 143, validates the skill output through the existing drift suite, and invokes up to 3 skill workers concurrently. The final full skill-mode run rendered all 7 manifests in 199.45s, and all persisted manifest frontmatter now has `renderer_mode: skill`.

## Investigation Findings
- Stall category: model-latency plus post-result overrun/protocol lifecycle cleanup. CLI/auth was healthy; payload size was not the primary blocker.
- Smallest-process T0/T1/T2/T3: T0 `2026-05-01T17:12:45.426372+00:00`; T1 `2026-05-01T17:12:47.958975+00:00`; T2 `2026-05-01T17:13:36.219569+00:00`; T3 `2026-05-01T17:13:37.740852+00:00`.
- Largest-process T0/T1/T2/T3: T0 `2026-05-01T17:19:22.781074+00:00`; T1 `2026-05-01T17:19:25.318076+00:00`; T2 `2026-05-01T17:20:27.231730+00:00`; T3 `2026-05-01T17:20:28.754025+00:00`.
- Total wall time for full `--ingest manifest` (skill mode): 199.45s with `STAGE13_RENDERER_MODE=skill`, `STAGE13_SKILL_TIMEOUT_SECONDS=900`, default concurrency 3.
- Logs path: `/tmp/stage13_diag_smallest.log`, `/tmp/stage13_diag_smallest_after_fixes.log`, `/tmp/stage13_diag_largest_after_frontmatter_fix.log`, `/tmp/stage13_full_skill_run.log`, `/tmp/stage13_r12_rerun.log`, `/tmp/stage13_r13_auto.log`.

## Fixes Applied
| Fix | Applied? | Effect |
|---|---|---|
| F.1 raise timeout | YES | Added `STAGE13_SKILL_TIMEOUT_SECONDS`, used 900s for diagnostics; no final run hit timeout. |
| F.2 concurrency | YES | Added default concurrency 3 for pure skill mode; reduced full 7-process run from 351.96s serial to 199.45s. |
| F.3 model pin | YES | Pinned Stage 13 Claude model to Haiku by default via `STAGE13_CLAUDE_MODEL`; Opus default was slow, Sonnet was reliable but too slow for R.14. |
| F.4 bundle trim | NO | Not needed; largest 5-candidate bundle succeeded without trimming. |
| F.5 protocol parser hardening | YES | Added bounded Stage 13 runner with result recovery after non-zero exit, result-stability detection, child termination after contract write, and malformed-frontmatter normalization only when YAML parsing fails. |
| F.6 CLI/auth | YES | Verified `claude` on PATH and authenticated; smoke and session traces showed `auth_healthy=true`. |

## Per-Process Skill Invocation Result
| Process | session_id | wall_clock_s | outcome |
|---|---|---:|---|
| proc-pc-19574d11-0929-f111-813c-005056b64545 | session-pty-1b7e07f1adb0cec16f979586cb8ad90c1814dd217512a170659bf5784dcc9607 | 64.5 | succeeded |
| proc-pc-218726bd-9dda-f011-813c-005056b64545 | session-pty-7cf46e3bb7eaf973c036372ae1a9670ec643202feb7e7b47b7208f80821ec3bf | 37.6 | succeeded |
| proc-pc-3016d0da-ef3c-f111-813c-005056b64545 | session-pty-2c8cf37ae40006c92fd97d0958ff8ff6c5fce9725dbc0add479a31ac7ebf7ab2 | 71.8 | succeeded |
| proc-pc-3ba4e6b8-9cef-f011-813c-005056b64545 | session-pty-1c81524e45b0088adecc78f34c41b36d931c66f978527fd1e7910e24ffd2b56a | 39.3 | succeeded |
| proc-pc-7046e8ec-2734-ed11-8119-005056b64545 | session-pty-06292d31ffcea895262beb5fb10cb5df47709a2d684ee027d6f836e84dab7d65 | 57.2 | succeeded |
| proc-pc-7e2a6fb2-67a3-f011-813a-005056b64545 | session-pty-545107261b35cd156b812dd779ab2d2e5f9af24b00d39917855ac30f3f1d5e36 | 33.0 | succeeded |
| proc-pc-d23508ec-853d-f111-813c-005056b64545 | session-pty-60bf8d4598b9a89af09ca898b1d8a890a1950dafffcf6483523696de02b9c777 | 56.2 | succeeded |

## R.1-R.16 Acceptance Criteria
| # | Criterion | Result | Detail |
|---|---|---|---|
| R.1 | All 7 rendered via skill, frontmatter and audit rows agree | PASS | 7 manifest rows have `renderer_mode: skill`; 7 new `agent_session_logs` rows in the validation window for `skills/manifest-generation/SKILL.md`. |
| R.2 | Session rows succeeded with distinct ids | PASS | `distinct_sessions=7`, `succeeded=7`, `total=7`. |
| R.3 | Stoplight distribution unchanged | PASS | `clarify=7`. |
| R.4 | Prompt counts unchanged | PASS | Counts are 5/1/5/2/2/1/2, 18 total. |
| R.5 | Reasoning non-empty, bounded, anchored | PASS | All 7 reasoning fields contain exact reason-code anchors such as `awaiting_rfq_quote` or `no_pricing_signal`. |
| R.6 | Drift detection passes | PASS | Skill output passed schema, typed deserialization, deterministic drift checks, prompt drift checks, and reasoning anchoring. |
| R.7 | Frontmatter consistency | PASS | Ruby YAML/JSON verification confirmed `stoplight` and `clarification_prompts` match DB/bundle values for all 7. |
| R.8 | Artifact files match DB and hashes | PASS | File bytes equal `manifests.markdown`; SHA-256 equals `manifests.artifact_hash` for all 7. |
| R.9 | Pipeline assessments remain manifest-ready and Stage 12 columns unchanged | PASS | `pipeline_state=manifest_ready` for 7; snapshot comparison of Stage 12-owned columns returned 0 changed rows. |
| R.10 | Stage 11/12 invariants untouched | PASS | Landed cost rows `review=4`; DGU row `review\|5160\|17692\|0\|4570\|27422`; net-margin assessments `review=17`. |
| R.11 | Idempotency skip | PASS | Second skill-mode run kept rows 7->7, artifact hash aggregate unchanged, session count unchanged, and logged `skipped_unchanged=7`. |
| R.12 | Delete-and-rerun deterministic fields stable | PASS | CSV diff of deterministic fields before/after rerun was empty; markdown/prose varied as allowed. |
| R.13 | Auto fallback | PASS | Forced `STAGE13_SKILL_TIMEOUT_SECONDS=1` in `auto`; manifest frontmatter showed `renderer_mode: auto_fallback_to_daemon`; process was restored to `skill` afterward. |
| R.14 | Wall time <5 minutes | PASS | Concurrent full skill-mode run completed in 199.45s. |
| R.15 | Default mode is Skill | PASS | `read_stage13_renderer_mode()` defaults to `Stage13RendererMode::Skill`; emergency rollback remains `STAGE13_RENDERER_MODE=daemon`. |
| R.16 | Quality gates pass | PASS | Cargo fmt/check/clippy, 129 binary tests, and 39 integration test files all passed. |

## Quality Gates
- cargo fmt --check: PASS
- cargo check --all-targets: PASS
- cargo clippy --all-targets -- -D warnings: PASS
- cargo test -p daemon --bin daemon: PASS - 129 tests, includes T.22-T.30
- per-file integration loop: PASS - 39 files

## Code Changes
| File | Change |
|---|---|
| `crates/daemon/src/main.rs` | Added Stage 13 renderer mode selection, bounded manifest skill invocation, result recovery, session audit insertion, frontmatter validation/normalization, skill-mode concurrency, env-tunable timeout/model/concurrency, and T.22-T.30. |
| `skills/manifest-generation/SKILL.md` | Tightened pass-through contract, required `renderer_mode: skill`, documented `options`, required exact reasoning anchor, and preserved daemon run-manifest lint tokens while forbidding skill-side run-manifest writes. |
| `docs/pipeline/stage-13-skill-invocation-restoration-manifest.md` | Added this validation record. |

## Default Mode After This Work
- `read_stage13_renderer_mode()` default: Skill
- Reasoning: Option B is green with all 7 manifests skill-rendered, the auto fallback works as a safety net, and `STAGE13_RENDERER_MODE=daemon` remains the emergency rollback override.

## Issues Remaining
None blocking. Haiku can still drift if the prompt is weakened; the daemon now rejects such drift, and the skill prompt explicitly requires exact pass-through fields and exact reasoning anchors.

## Out-of-Scope (Documented, Not Fixed Here)
- Operator override mechanism + UI binding (Stage 14)
- Wiring `--ingest manifest` into `run_deterministic_pipeline`
- Manifest staleness TTL / scheduled re-render
