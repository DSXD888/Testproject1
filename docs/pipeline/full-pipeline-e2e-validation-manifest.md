# Full Pipeline End-to-End Validation Manifest

> Generated: 2026-05-04T15:15:35Z  
> Status: YELLOW — L1/L3 pipeline reached Stage 14 CLI on real data; desktop UI reached full-control mode and the native bridge behind Quick-Decide captured a second decision, but click-level UI interaction remains blocked by missing assistive access.
> HEAD at validation start: 4c31791 `docs(prompts): add full-pipeline end-to-end validation handoff`  
> E2E state root: `state-e2e-20260504T113157Z`  
> Spotlight process: `proc-pc-956d1aaa-8841-f111-813c-005056b64545` ("4) Woodhouses Voluntary Controlled Primary School - Boilers and BMS Panel Replacement")

## Summary

This run drove a fresh isolated L1-L3 E2E state from live ProContract and Contracts Finder discovery through deterministic normalization/relating, real ProContract interest registration, tender enrichment, PDF extraction, marketplace pricing, landed cost, net margin gating, manifest generation, and CLI operator decision capture. The final spotlight process was a ProContract Oldham Council boiler/BMS replacement opportunity with two extracted boiler SKU candidates and one real GBP supplier quote from eBay UK.

The run surfaced and fixed several daemon issues that only appeared under continuous real-data flow: ProContract interest over-selection, ProContract enrich result identifier drift, stale extraction job blocking, semantic extraction jobs lacking prepared Docling artifacts, and contract child timeout/supervision behavior for contract-mode skills. Two larger gaps were filed as follow-up beads: large mechanical-spec semantic extraction timeout and a PTY marketplace child that wrote `result.json` but hung before exit.

The desktop app launched via `bunx tauri dev`, was pointed at the E2E state root, and was relaunched with `PROCINTEL_DAEMON_PROGRAM` so the app reached `full control` mode. Because assistive access for automated UI clicks is not available to `osascript`/Computer Use in this session, I exercised the same local daemon bridge command path that the Tauri Quick-Decide and clarification controls call. This produced the second decision row and a clarification override, but the manifest stays YELLOW because the native window was not manually clicked through.

## Setup

| Item | Value |
|---|---|
| Branch | `main` |
| HEAD at start | `4c31791` |
| Final commit | Commit containing this manifest and daemon fixes |
| Isolated state root | `state-e2e-20260504T113157Z` |
| Isolated runtime dir | `runtime-e2e-20260504T113157Z` |
| Isolated artifact root | `artifacts-e2e-20260504T113157Z` |
| Patched E2E daemon | `/tmp/testproject1-target-e2e/debug/daemon` |
| CDP browser | Chrome 147 on `http://localhost:9222/json/version` |
| Operator portal sign-in | Confirmed by successful ProContract interest/enrich runs |
| Wait strategy used | D3.1 auto-grant path: interest registration was followed immediately by document access/enrichment for registered RFx processes |
| Live `state/state.db` sha256 before | `2d82a96dfafa58795dddc440ba0b3f38079ab28ae30c3eafc0c8364f96e5816a` |
| Live `state/state.db` sha256 after restore | `2d82a96dfafa58795dddc440ba0b3f38079ab28ae30c3eafc0c8364f96e5816a` |

## Spotlight Selection (D2)

| Field | Value |
|---|---|
| process_id | `proc-pc-956d1aaa-8841-f111-813c-005056b64545` |
| Notice ID | `pc-956d1aaa-8841-f111-813c-005056b64545` |
| Notice title | `4) Woodhouses Voluntary Controlled Primary School - Boilers and BMS Panel Replacement (Four of five school summer shutdown projects)` |
| Buyer | Oldham Council |
| Source | ProContract |
| Published | 2026-04-26T21:00:00Z |
| Deadline | 2026-05-22T12:00:00Z |
| Estimated value | £130,000.00 |
| Category | `399999 - Not Elsewhere Classified` |
| Reason chosen | One of the safely capped registered-interest opportunities; enrichment produced a concrete boiler schedule with physical SKU-like items and marketplace-priceable products. |

## Stage-by-stage Outcomes

| Stage | Result | Evidence |
|---|---|---|
| 1 — Discover (ProContract) | PASS | 100 ProContract notices ingested into isolated DB. |
| 1 — Discover (Contracts Finder) | PASS | 100 Contracts Finder notices ingested into isolated DB. |
| 1 — Discover (FTS) | DEFERRED | No local FTS fixture was found; this remains covered by bead `MS-FTS-LIVE-1`. |
| 2 — Normalize | PASS | 200 notices produced 200 procurement processes; 200 `Normalize` succeeded rows. |
| 3 — Relate | PASS | Deterministic pipelines ran for both live sources; `process_relations` count was 0, expected for disjoint ProContract/CF OCID namespaces. |
| 4a — Register interest | PASS | Real portal mutation capped at 3 registrations; Woodhouses spotlight registered. |
| 4-wait | PASS | D3.1: document access was available after registration, so the run proceeded directly to enrich. |
| 4b — Enrich | PASS | Woodhouses document pack downloaded under `artifacts-e2e-20260504T113157Z/downloads/procontract/785205/`; two spotlight PDFs persisted for extraction. |
| 5-7 — Lots/qualify/reclassify | PASS | Pipeline rerun created the implicit lot, qualification, reclassification, quoteability, and pricing state needed downstream. |
| 8 — Product extraction | PASS WITH CAVEAT | Boiler schedule semantic extraction produced two SKU candidates; large mechanical spec semantic job timed out/pending and was filed as `testproject1-qnl3`. |
| 9 — Marketplace search | PASS WITH CAVEAT | Amazon produced 0 usable quotes due locale/currency drift; eBay UK produced 1 GBP quote after manual child termination of a hung PTY skill, filed as `testproject1-hf42`. |
| 10 — Gross margin gate | PASS | Pipeline selected the best available quote despite strict retail usability rejection; 1 candidate pricing assessment persisted. |
| 11 — Landed cost | PASS | 1 domestic landed-cost estimate persisted with decision grade `review`; import skills correctly skipped for GB ship-from quote. |
| 12 — Net margin gate | PASS | 2 net-margin assessments: B01 `review` for `no_pricing_signal`, B02 `reject` with base/low/high net margin -10000 bps. |
| 13 — Manifest | PASS | Manifest row persisted with stoplight `clarify`, basis hash `fad1066031083dcbfaf2af1f537e8d08cfe627ceda6e23d409fbbd907797def2`, and rendered `manifest.md`. |
| 14a — CLI decision | PASS | `pending_operator_intents` row queued with idempotency key `e2e-cli-20260504T145047Z`, disposition `watch`. |
| 14b — UI decision | PASS WITH CAVEAT | Tauri reached full-control mode; native daemon bridge accepted a Quick-Decide-shaped `watch` decision with idempotency key `decision-watch-e2e-ui-bridge-20260504T1616Z`. Literal click-level UI automation remains blocked. |

## Stage Evidence Details

| Query / Artifact | Result |
|---|---|
| `contract_notices` by source | `contracts-finder=100`, `procontract=100` |
| `procurement_processes` | 200 |
| `process_relations` | 0 |
| Spotlight raw docs | 2 PDFs persisted through `document_extractions` |
| Spotlight structural docs | Boiler schedule: 2 pages, 2 tables; mechanical spec: 41 pages, 224 sections, 21 tables |
| Spotlight semantic extractions | 1 persisted semantic extraction |
| Product candidates | B01 Ideal Evomax 2 80kW boiler, B02 Ideal Evomax 2 60kW boiler |
| Supplier quote | `sq-5f0522a9d4dd45b9ba7b44348af86413-0`, eBay UK, GBP 306,670 pence, listing `227226093138` |
| Landed cost | `landed-...boiler-b02...`, economic landed cost 13,000,000 pence, basis `from_lot_quantity`, confidence 5500 bps |
| Manifest artifact | `artifacts-e2e-20260504T113157Z/manifests/proc-pc-956d1aaa-8841-f111-813c-005056b64545/manifest.md` |
| UI bridge clarification | `no_pricing_signal` override for B01, payload `{"variant":"defer","reason":"E2E UI bridge validation: defer B01 manual pricing until a follow-up supplier quote is available."}` |
| UI bridge decision | `decision-watch-e2e-ui-bridge-20260504T1616Z`, status `queued`, disposition `watch`, visible after commit seq 76 |

## Real Portal Mutations (D4)

| Notice ID | Buyer | Title | Action | Result |
|---|---|---|---|---|
| `pc-82498b3f-d31b-f111-813c-005056b64545` | Leicester City Council | Leicester City Council CCTV Opportunity | register_interest | registered |
| `pc-2a824562-8941-f111-813c-005056b64545` | Oldham Council | Yew Tree Community School - Boilers and BMS Panel Replacement | register_interest | registered |
| `pc-956d1aaa-8841-f111-813c-005056b64545` | Oldham Council | Woodhouses VC Primary School - Boilers and BMS Panel Replacement | register_interest | registered |

## Bugs Surfaced + Fixed

| File | Bug | Fix | Regression test |
|---|---|---|---|
| `crates/daemon/src/main.rs` | `procontract-interest` could attempt far more than the D4 cap because source-loop targeting was not capped fresh-first. | Added `MAX_PROCONTRACT_INTEREST_REGISTRATIONS_PER_RUN=3` and target selection helper. | `procontract_interest_registration_targets_are_capped_to_mutation_limit`, `failed_interest_registration_remains_retryable` |
| `crates/daemon/src/main.rs` | ProContract enrich result IDs could be portal numeric/RFx IDs rather than local process IDs, causing FK/persistence failures. | Added process resolution from hinted process ID, notice ID, title, buyer, and artifact context. | `enrichment_notice_resolution_ignores_invalid_hinted_process_id` |
| `crates/daemon/src/main.rs` | Pending/stale unrelated extraction jobs could block enrich replay and Stage 8 drains. | Extraction drain now claims extraction jobs only, retries expired running extraction leases, and filters to enriched process IDs. | `extraction_drain_selector_ignores_older_source_jobs`, `extraction_drain_selector_retries_expired_running_extraction_jobs`, `extraction_drain_selector_respects_process_filter` |
| `crates/daemon/src/main.rs` | Semantic PDF jobs were launched without prepared Docling markdown/sections/tables artifacts. | Added semantic input preparation and context loading under `pdf-extraction-inputs`. | `semantic_skill_parameters_include_prepared_extraction_artifacts` |
| `crates/daemon/src/main.rs` | Contract-mode child timeout could leave output drains/processes hanging. | Added `CONTRACT_SKILL_TIMEOUT_SECONDS`, process-group launch on Unix, stdout/stderr drain threads, and timeout termination. | `contract_child_output_timeout_reaps_child` |
| `crates/daemon/src/main.rs` | Live ProContract enrich reruns needed a safe replay path to diagnose parser/persistence bugs without re-driving the portal. | Added hidden `PROCONTRACT_ENRICH_REPLAY_RESULT_JSON` replay hook. | Covered by enrich resolution/drain replay regressions. |

## Bugs Surfaced + Filed

| Bead ID | Title | Reproduction | Why deferred |
|---|---|---|---|
| `testproject1-qnl3` | Stage 8 semantic extraction times out on large mechanical specifications | Re-run semantic extraction for document extraction `02b4ad86-2b7a-4262-8488-26d703cd937b` in `state-e2e-20260504T113157Z`. | Larger than the E2E bug-fix window; likely needs chunking/summarization or prompt/runtime redesign. |
| `testproject1-hf42` | Marketplace PTY skill can write result contract but hang before exit | Run eBay UK skill with `/tmp/e2e_woodhouses_ebay_job.json`; valid `result.json` is written but child may not exit. | Needs PTY adapter/result-contract supervision work beyond the small inline contract-mode timeout fix. |

## Desktop UI Walkthrough

| Step | Screenshot | Result |
|---|---|---|
| Default launch | `docs/pipeline/full-pipeline-e2e-screenshots/step-00-default-path-schema-mismatch.png` | PASS for diagnosis: app launched but read app-support DB and reported schema mismatch. |
| Relaunch with E2E state root | `docs/pipeline/full-pipeline-e2e-screenshots/step-01-e2e-state-inbox-blocked-by-privacy-prompt.png` | PARTIAL: app read `state-e2e-20260504T113157Z`, showed browse-only snapshot, but privacy prompt blocked interaction. |
| Final prompt state | `docs/pipeline/full-pipeline-e2e-screenshots/step-02-final-privacy-prompt-state.png` | BLOCKED: macOS screen/audio privacy prompt required explicit approval; no click was performed. |
| Relaunch with daemon program | `docs/pipeline/full-pipeline-e2e-screenshots/step-03-full-control-desktop-ready.png` | PASS: app reached `full control` mode and displayed "All desktop actions are available." |
| Native bridge continuation | no screenshot | PASS WITH CAVEAT: bridge accepted clarification, rerun chain completed (`landed-cost`, `net-gate`, `manifest`), and bridge accepted Quick-Decide-shaped `watch`; this did not use click-level UI automation. |

Notes:

- Path A is supported by desktop env overrides: `PROCINTEL_DESKTOP_STATE_ROOT`, `PROCINTEL_DESKTOP_RUNTIME_DIR`, `PROCINTEL_DESKTOP_ARTIFACT_ROOT`.
- The app initially launched in browse-only mode because no daemon program override was supplied for full control in the first UI pass; relaunching with `PROCINTEL_DAEMON_PROGRAM=/tmp/testproject1-target-e2e/debug/daemon` reached full control.
- UI R.18/R.21 remain blocked pending explicit user approval for assistive UI automation or user-driven clicks in the native window.

## Operational Subsystem Checks

| Subsystem | Status | Evidence |
|---|---|---|
| Cron scheduler | PASS (code/test-backed) | Cron definitions are built from source registry at runtime; required native-shell/daemon tests passed. No persisted `cron_definitions` table exists in schema. |
| Source health | PASS | `source_runtime_state` had 5 registered sources (`contracts-finder`, `fts`, `procontract`, `procontract-enrich`, `procontract-interest`), all circuit state `closed`. |
| Resource arbiter | PASS | Multi-marketplace/source skill runs completed without arbiter panic; required daemon tests passed. |
| Validation pipeline | PASS EXPECTED EMPTY | `validation_verdicts=0`; L4 validator skills are not configured. |
| Workflow jobs | PASS WITH CAVEAT | `workflow_jobs`: 18 completed, 1 failed, 9 retry; extraction jobs transitioned through completed/failed/retry during Stage 8. |
| IPC bridge | PASS WITH CAVEAT | Tauri daemon bridge 14/14 and native shell 22/22 passed; live UI bootstrap reached full-control mode, and the native bridge accepted clarification/decision mutations. Click-level UI automation remained blocked. |

## Live state.db Integrity Check (R.25)

| Check | Result |
|---|---|
| sha256 before | `2d82a96dfafa58795dddc440ba0b3f38079ab28ae30c3eafc0c8364f96e5816a` |
| sha256 after | `2d82a96dfafa58795dddc440ba0b3f38079ab28ae30c3eafc0c8364f96e5816a` |
| Match | YES |

## R.1–R.30 Status

| # | Criterion | Status | Evidence |
|---|---|---|---|
| R.1 | Pre-flight all green | PASS | CDP, CLIs, skills, dispatcher, baseline tests verified before E2E. |
| R.2 | ProContract >=10 notices | PASS | 100 notices. |
| R.3 | Contracts Finder >=10 notices | PASS | 100 notices. |
| R.4 | FTS ran or deferred | PASS | Deferred, no fixture found. |
| R.5 | Stage 2-3 chains clean | PASS | Both deterministic pipelines ran; 0 expected relations. |
| R.6 | Spotlight selected per D2 | PASS WITH CAVEAT | Strict fresh goods query had no ideal candidate; selected registered physical-goods-like boiler spotlight from fresh ProContract batch. |
| R.7 | Interest registered within cap | PASS | 3 total real registrations, spotlight included. |
| R.8 | Wait strategy documented | PASS | D3.1 immediate document access. |
| R.9 | Enrich downloaded docs | PASS | Woodhouses pack and PDFs present. |
| R.10 | Stages 5-7 reran | PASS | Lot/qualification/quoteability states present. |
| R.11 | Stage 8 product candidates | PASS WITH CAVEAT | 2 boiler candidates; large spec timeout filed. |
| R.12 | Stage 9 supplier quote | PASS WITH CAVEAT | 1 GBP eBay quote; PTY hang filed. |
| R.13 | Landed cost | PASS | 1 LCE row. |
| R.14 | Net margin | PASS | 2 assessments. |
| R.15 | Manifest | PASS | 1 manifest row/artifact. |
| R.16 | CLI decision | PASS | `e2e-cli-20260504T145047Z`, status `queued`. |
| R.17 | UI launched and spotlight visible | PARTIAL | Tauri launched on E2E state and reached full control; interaction blocked before navigating to Manifest Inbox/spotlight. |
| R.18 | Process detail renders manifest | BLOCKED | macOS privacy prompt. |
| R.19 | UI rerun modal/hash refresh | PASS WITH CAVEAT | Native bridge/desktop rerun sequence completed `landed-cost`, `net-gate`, `manifest`; modal animation was not click-tested. |
| R.20 | UI decision captures second row | PASS WITH CAVEAT | Native bridge accepted `decision-watch-e2e-ui-bridge-20260504T1616Z`; DB now has CLI row plus bridge/UI-shaped row. |
| R.21 | Cross-session staleness | BLOCKED | macOS privacy prompt. |
| R.22 | Screenshots committed | PASS | Four UI diagnostic screenshots captured, including full-control desktop readiness. |
| R.23 | Operational subsystems | PASS WITH CAVEAT | Runtime cron source backed by tests, no persisted `cron_definitions` table. |
| R.24 | `validation_verdicts` empty | PASS | Count 0. |
| R.25 | Live DB untouched | PASS | Hash match. |
| R.26 | Bugs fixed/filed | PASS | 6 fixed in daemon, 2 filed beads. |
| R.27 | Tests | PASS | Daemon bin 188/188; integration loop 39/39; vitest 43/43; Tauri Rust tests 25/14/22. |
| R.28 | Quality gates | PASS | fmt, check, clippy clean; `ubs` exit 0. |
| R.29 | Validation manifest | PASS | This document. |
| R.30 | Final commit + push | PASS | Completed by the commit containing this manifest and pushed to `origin/main`. |

## Quality Gates

| Gate | Result |
|---|---|
| `cargo fmt --check` | PASS |
| `cargo check --all-targets` | PASS |
| `cargo clippy --all-targets -- -D warnings` | PASS |
| `cargo test -p daemon --bin daemon` | PASS, 188/188 |
| Per-file daemon integration loop | PASS, 39/39 |
| `cargo test -p procintel-desktop --test query_service` | PASS, 25/25 |
| `cargo test -p procintel-desktop --test daemon_bridge` | PASS, 14/14 |
| `cargo test -p procintel-desktop --test native_shell` | PASS, 22/22 |
| `apps/desktop ./node_modules/.bin/vitest run` | PASS, 43/43 |
| `ubs crates/daemon/src/main.rs` | PASS, exit 0; zero critical issues |

## Stages NOT Exercised

- Stage 15 (`bid_workflow`): L6-blocked and intentionally not run.
- Web viewing portal: L4 not built.
- Sync to Supabase: L4 not built.
- Validator skills: L4 not built.
- Cross-org coordination: L4 not built.
