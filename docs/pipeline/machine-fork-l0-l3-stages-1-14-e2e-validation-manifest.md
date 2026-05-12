# Machine Fork L0-L3 Stages 1-14 E2E Validation Manifest

Generated: 2026-05-12T18:46:53Z
Last verified: 2026-05-12T21:43:13Z
Status: PAUSED - Stage 4a ProContract register-interest produced unsupported `pending_registration` actions after a real portal registration
Git HEAD at verification: e4080be docs: record contracts finder claude auth blocker
State root: `/Users/zacfarrell/Desktop/Testproject1/state-e2e-machine-fork-20260512T195251Z`
Runtime dir: `/Users/zacfarrell/Desktop/Testproject1/runtime-e2e-machine-fork-20260512T195251Z`
Artifact root: `/Users/zacfarrell/Desktop/Testproject1/artifacts-e2e-machine-fork-20260512T195251Z`
QA log root: `/Users/zacfarrell/Desktop/Testproject1/artifacts-e2e-machine-fork-20260512T195251Z/qa-logs`
Daemon binary: `/tmp/testproject1-target-machine-fork-20260512T195251Z/debug/daemon`
CDP browser: available; Chrome/148.0.7778.97, Protocol 1.3, listening on 127.0.0.1:9222, launched with `$HOME/.chrome-automation`

Status meanings:
- GREEN: all required L0-L3 and requested localhost smoke checks passed.
- PAUSED: QA stopped intentionally for operator login/permission or an error manifest.
- RED: an unrepaired required L0-L3 blocker remains.
- PARTIAL: L0-L3 passed but optional/bounded smoke items are deferred or inconclusive.

## Pause Ledger

| Time | Stage | Cause | Action | Resume Point |
|---|---|---|---|---|
| 2026-05-12T18:46:53Z | Critical current-state checks | Daemon schema epoch is 9 while desktop/Tauri constants still expect 8. Desktop cannot reach full control against daemon-created state. | Created `prompts/machine-fork-e2e-schema-epoch-mismatch-error-manifest.md`. | After the schema epoch mismatch is fixed, re-run the critical schema epoch `rg` command, then continue with Environment Preflight. |
| 2026-05-12T18:46:53Z | Environment preflight | Required `agent-browser` CLI is not on `PATH`. | Created `prompts/machine-fork-e2e-agent-browser-missing-error-manifest.md`. | After `agent-browser` is installed/exposed, run `which agent-browser`, `agent-browser --cdp 9222 open about:blank`, and continue CDP preflight. |
| 2026-05-12T18:46:53Z | Persistent CDP browser preflight | First observed CDP setup failure: `localhost:9222` refused connection. | Recorded as operator CDP setup pause, not a product error manifest. | Launch the persistent Chrome CDP profile on port 9222, then re-run `/json/version` and CDP login checks. |
| 2026-05-12T19:29:53Z | Browser/CDP setup repair | `agent-browser` missing and CDP unavailable from the original preflight. | Installed `agent-browser` with Homebrew, ran `agent-browser install`, launched persistent Chrome CDP with `$HOME/.chrome-automation`, and validated `agent-browser --cdp 9222`. | Browser/CDP preflight is green. Resume the E2E handoff after the independent schema epoch blocker is fixed, starting at Environment Preflight -> Toolchain, then Persistent CDP Browser, Login Checks, and Isolated Run Setup. |
| 2026-05-12T19:43:18Z | Schema epoch repair verification | Desktop/Tauri constants and fixtures previously expected epoch 8 while daemon schema epoch was 9. | Commit `6d113ad` aligned desktop epoch expectations to 9 and added a daemon-authority regression guard; validation commands passed. | Schema epoch blocker is resolved. Resume from Environment Preflight -> Toolchain, then Persistent CDP Browser, Login Checks, and Isolated Run Setup. |
| 2026-05-12T19:52:51Z | Resume after operator clearance | Operator reported error manifests and fixes were complete. | Re-ran schema epoch, toolchain, CDP, and source login checks; created fresh isolated E2E roots; built daemon; ran daemon dry-run. | Continue with static/unit baseline gates before live stage execution. |
| 2026-05-12T20:01:43Z | Static/unit baseline - desktop native shell | `cargo test -p procintel-desktop --test native_shell` failed once in the required baseline gate; filtered and full reruns passed, making this an intermittent/order-dependent gate failure. | Created `prompts/machine-fork-e2e-desktop-native-shell-rerun-lock-error-manifest.md` and paused before live Stage 1 ingestion. | After a fixer resolves or formally classifies the flake, rerun `cargo test -p procintel-desktop --test native_shell`, then continue from the static/unit baseline gate. |
| 2026-05-12T20:18:40Z | Resume after native-shell fix | Commit `9d15b9c` fixed native-shell test hook serialization. | Exact failing test passed, full `native_shell` target passed, and 20 full native-shell stress iterations passed. | Continue to live Stage 1 source ingestion. |
| 2026-05-12T20:39:49Z | Stage 1 - Contracts Finder live ingestion | The real live Contracts Finder daemon command launched a PTY skill session, but the selected Claude CLI exited with `Failed to authenticate. API Error: 401 Invalid authentication credentials`; no `result.json` or run manifest was produced. | Created `prompts/machine-fork-e2e-contracts-finder-claude-auth-error-manifest.md` and paused before ProContract ingestion. | Repair or verify Claude CLI auth, then retry the exact failed `stage-01-contracts-finder` command against the existing E2E roots. |
| 2026-05-12T21:05:37Z | Resume after Claude auth repair | The direct selected-agent probe `claude -p "Return exactly OK." --dangerously-skip-permissions` returned `OK.` | Retried Contracts Finder against the existing E2E roots. | Continue Stage 1 source ingestion validation. |
| 2026-05-12T21:27:25Z | Stage 1 - source ingestion complete | Real Contracts Finder and ProContract ingestion both completed with valid result contracts and persisted rows. | Contracts Finder total now 104 notices; ProContract added 100 notices; source circuits remain closed. FTS is deferred to avoid synthetic fixture data because no current live FTS daemon path/fixture file was found. | Continue with deterministic pipeline for `procontract` and `contracts-finder`. |
| 2026-05-12T21:31:10Z | Deterministic Stages 2-3 and 5-8 baseline | `--pipeline procontract` and `--pipeline contracts-finder` both completed. | 204 lots, 408 qualification rows, 204 raw document references, 118 goods-qualified rows, no pending extraction jobs. Product candidates/search profiles remain pending document enrichment. | Continue to Stage 4a ProContract register-interest. |
| 2026-05-12T21:43:13Z | Stage 4a - ProContract register interest | Valid result contract contained eight unsupported `pending_registration` actions; daemon rejected persistence with `unsupported ProContract registration action` and left the source job `running`. One real portal registration had already occurred. | Created `prompts/machine-fork-e2e-procontract-pending-registration-error-manifest.md` and paused without retrying or fixing product code. | After the fixer updates contract/persistence semantics, retry the exact Stage 4a command against the existing E2E roots, accounting for the already registered `BEST START FAMILY HUBS OUTREACH SERVICES` opportunity. |

## Error Manifests Created

| Path | Stage | Blocking Symptom | Status |
|---|---|---|---|
| `prompts/machine-fork-e2e-schema-epoch-mismatch-error-manifest.md` | Critical current-state checks / Desktop validation precondition | Daemon `DEFAULT_SCHEMA_EPOCH=9`; desktop expected epoch 8 and rejected mismatched state/handshake. | Resolved by commit `6d113ad`; verified 2026-05-12T19:43:18Z. |
| `prompts/machine-fork-e2e-agent-browser-missing-error-manifest.md` | Environment preflight / Browser testing precondition | `agent-browser` command not found, so CDP preflight and daemon-driven browser skills cannot be validated. | Resolved by machine setup repair on 2026-05-12T19:29:53Z. |
| `prompts/machine-fork-e2e-desktop-native-shell-rerun-lock-error-manifest.md` | Static/unit baseline / Desktop native shell | Intermittent failure: rerun-lock test expected stopped PID `[4242]` but observed `[]` in the required full `native_shell` target. | Resolved by commit `9d15b9c`; verified with exact test, full target, and 20x stress loop. |
| `prompts/machine-fork-e2e-contracts-finder-claude-auth-error-manifest.md` | Stage 1 / Contracts Finder live ingestion | Selected Claude CLI returned `401 Invalid authentication credentials`, causing the PTY skill to exit before writing a result contract. | Resolved by Claude auth repair; direct probe returned `OK` and Contracts Finder retry completed. |
| `prompts/machine-fork-e2e-procontract-pending-registration-error-manifest.md` | Stage 4a / ProContract register interest | Skill result contract included eight `pending_registration` final actions after one real registration; daemon persistence rejects that action and leaves the workflow job `running`. | Open; QA paused at Stage 4a. |

## Preflight

Commands run:

```bash
pwd
git status --short --branch
git log --oneline -1
sed -n '1,260p' AGENTS.md
sed -n '261,620p' AGENTS.md
sed -n '621,980p' AGENTS.md
```

Evidence:

```text
/Users/zacfarrell/Desktop/Testproject1
## main...origin/main
 M .DS_Store
?? bin/__pycache__/
4d1d1dd docs: persist machine fork e2e helpers
```

AGENTS.md was read first. The repo is on `main`. Existing unrelated local artifacts were not stashed, reverted, deleted, or staged.

Toolchain preflight:

```bash
which cargo
which bun
which claude || true
which codex || true
which agent-browser || true
ls "$HOME/.docling-venv/bin/docling" 2>/dev/null || true
bun --version
cargo --version
rustup show active-toolchain
```

Output:

```text
/Users/zacfarrell/.cargo/bin/cargo
/Users/zacfarrell/.bun/bin/bun
/Users/zacfarrell/.local/bin/claude
/opt/homebrew/bin/codex
1.3.13
cargo 1.96.0-nightly (cbb9bb8bd 2026-03-13)
nightly-aarch64-apple-darwin (overridden by '/Users/zacfarrell/Desktop/Testproject1/rust-toolchain.toml')
```

`agent-browser` did not resolve.

Schema epoch critical check:

```bash
rg -n "pub const DEFAULT_SCHEMA_EPOCH|EXPECTED_SCHEMA_EPOCH|DEFAULT_SCHEMA_EPOCH" \
  crates/daemon/src/db/sqlite.rs \
  apps/desktop/src-tauri/src/main.rs \
  apps/desktop/src-tauri/src/query_service.rs \
  apps/desktop/src-tauri/src/daemon_bridge.rs \
  apps/desktop/src-tauri/tests -g '*.rs'
```

Key output:

```text
crates/daemon/src/db/sqlite.rs:75:pub const DEFAULT_SCHEMA_EPOCH: i64 = 9;
apps/desktop/src-tauri/src/main.rs:51:const EXPECTED_SCHEMA_EPOCH: i64 = 8;
apps/desktop/src-tauri/src/query_service.rs:14:const EXPECTED_SCHEMA_EPOCH: i64 = 8;
apps/desktop/src-tauri/src/daemon_bridge.rs:16:pub const DEFAULT_SCHEMA_EPOCH: i64 = 8;
apps/desktop/src-tauri/tests/native_shell.rs:21:const EXPECTED_SCHEMA_EPOCH_FOR_TEST: i64 = 8;
```

Origin freshness check:

```bash
git pull --rebase
git fetch origin
git log --oneline -1 origin/main
git show origin/main:crates/daemon/src/db/sqlite.rs | rg -n "DEFAULT_SCHEMA_EPOCH|0012_l4_sync_state"
git show origin/main:apps/desktop/src-tauri/src/main.rs | rg -n "EXPECTED_SCHEMA_EPOCH" | sed -n '1,6p'
git show origin/main:apps/desktop/src-tauri/src/query_service.rs | rg -n "EXPECTED_SCHEMA_EPOCH" | sed -n '1,5p'
git show origin/main:apps/desktop/src-tauri/src/daemon_bridge.rs | rg -n "DEFAULT_SCHEMA_EPOCH" | sed -n '1,5p'
```

Output:

```text
error: cannot pull with rebase: You have unstaged changes.
error: Please commit or stash them.

4d1d1dd docs: persist machine fork e2e helpers
26:const L4_SYNC_STATE_MIGRATION_PATH: &str = "migrations/0012_l4_sync_state.sql";
72:    ("0012_l4_sync_state", 9, L4_SYNC_STATE_MIGRATION_PATH),
75:pub const DEFAULT_SCHEMA_EPOCH: i64 = 9;
51:const EXPECTED_SCHEMA_EPOCH: i64 = 8;
14:const EXPECTED_SCHEMA_EPOCH: i64 = 8;
16:pub const DEFAULT_SCHEMA_EPOCH: i64 = 8;
```

`git pull --rebase` was blocked by pre-existing unstaged work. No stash or revert was performed. `git fetch origin` succeeded and showed `origin/main` still has the same epoch mismatch.

CLI dispatcher check:

```bash
rg -n '"fts" =>|"contracts-finder" =>|"procontract" =>|"procontract-interest" =>|"procontract-enrich" =>|"marketplace-coverage" =>|"landed-cost" =>|"net-gate" =>|"manifest" =>' crates/daemon/src/main.rs
rg -n "run_deterministic_pipeline|run_marketplace_search_live_loop|run_marketplace_coverage_live_loop|run_landed_cost_live_loop|run_net_margin_gate_live_loop|run_manifest_generation_live_loop" crates/daemon/src/main.rs
```

Key output:

```text
1348:            "fts" => {
1352:            "contracts-finder" => run_contracts_finder_live_loop(&mut runtime, &cli).await?,
1353:            "procontract" => run_procontract_discover_live_loop(&mut runtime, &cli).await?,
1354:            "procontract-interest" => {
1357:            "procontract-enrich" => {
1360:            "landed-cost" => run_landed_cost_live_loop(&mut runtime, &cli).await?,
1361:            "net-gate" => run_net_margin_gate_live_loop(&mut runtime, &cli)?,
1362:            "manifest" => run_manifest_generation_live_loop(&mut runtime, &cli).await?,
1363:            "marketplace-coverage" => {
1364:                run_marketplace_coverage_live_loop(&mut runtime, &cli).await?
1367:                run_marketplace_search_live_loop(&mut runtime, source_id, &cli).await?
1380:        let report = run_deterministic_pipeline(&mut runtime, source_id)?;
```

Persistent CDP preflight:

```bash
curl -sfS -m 3 http://localhost:9222/json/version
agent-browser --cdp 9222 open "about:blank"
```

Output:

```text
curl: (7) Failed to connect to localhost port 9222 after 0 ms: Couldn't connect to server
zsh:1: command not found: agent-browser
```

Browser/CDP setup repair:

```bash
find "$HOME" "/opt/homebrew" -maxdepth 4 \( -type f -o -type l \) -name 'agent-browser*' 2>/dev/null | sed -n '1,40p'
brew install agent-browser
agent-browser install
command -v agent-browser
agent-browser --version
agent-browser doctor --offline --quick
open -na "Google Chrome" --args \
  "--remote-debugging-port=9222" \
  "--remote-allow-origins=*" \
  "--user-data-dir=$HOME/.chrome-automation" \
  "--profile-directory=Default" \
  "--no-first-run" \
  "--no-default-browser-check"
curl -sfS -m 3 http://localhost:9222/json/version
lsof -nP -iTCP:9222 -sTCP:LISTEN
agent-browser --cdp 9222 open "about:blank"
agent-browser --cdp 9222 get title
agent-browser --cdp 9222 snapshot
```

Repair evidence:

```text
No existing `agent-browser` binary was found under the checked `$HOME` or `/opt/homebrew` roots before installation.
Install method: `brew install agent-browser`
Installed CLI path: /opt/homebrew/bin/agent-browser
CLI version: agent-browser 0.27.0
Browser payload install: `agent-browser install`
Browser payload: Chrome 148.0.7778.97 at /Users/zacfarrell/.agent-browser/browsers/chrome-148.0.7778.97
Doctor: `agent-browser doctor --offline --quick` reported 6 pass, 0 warn, 0 fail.
CDP listener: Google Chrome PID 26502 listening on 127.0.0.1:9222.
CDP metadata: Browser Chrome/148.0.7778.97, Protocol-Version 1.3, V8 14.8.178.14, WebKit 537.36 (@6b3fa66a923a9442c8ab0bc71b4b41ff24528d3b).
CDP browser launch profile: $HOME/.chrome-automation.
agent-browser control: `open about:blank`, `get title`, and `snapshot` completed against `--cdp 9222`.
```

Final browser preflight validation block:

```bash
set -euo pipefail
cd /Users/zacfarrell/Desktop/Testproject1

echo "== cli =="
which agent-browser
agent-browser --version

echo "== cdp =="
curl -sfS -m 3 http://localhost:9222/json/version

echo "== control =="
agent-browser --cdp 9222 open "about:blank"
agent-browser --cdp 9222 get title
agent-browser --cdp 9222 snapshot

echo "AGENT_BROWSER_CDP_PREFLIGHT_GREEN"
```

Validation output summary:

```text
== cli ==
/opt/homebrew/bin/agent-browser
agent-browser 0.27.0
== cdp ==
Browser Chrome/148.0.7778.97, Protocol-Version 1.3
== control ==
about:blank opened, title command succeeded, snapshot returned `(empty page)`.
AGENT_BROWSER_CDP_PREFLIGHT_GREEN
```

Resume preflight after operator clearance:

```bash
git status --short --branch
git log --oneline -5
rg -n "pub const DEFAULT_SCHEMA_EPOCH|EXPECTED_SCHEMA_EPOCH|DEFAULT_SCHEMA_EPOCH" \
  crates/daemon/src/db/sqlite.rs \
  apps/desktop/src-tauri/src/main.rs \
  apps/desktop/src-tauri/src/query_service.rs \
  apps/desktop/src-tauri/src/daemon_bridge.rs \
  apps/desktop/src-tauri/tests -g '*.rs'
which cargo
which bun
which claude || true
which codex || true
which agent-browser || true
curl -sfS -m 3 http://localhost:9222/json/version
agent-browser --cdp 9222 open "about:blank"
agent-browser --cdp 9222 get title
```

Resume output summary:

```text
HEAD includes 9d15b9c fix: serialize native shell test hooks.
Schema epoch check: daemon, desktop bootstrap, query service, daemon bridge, and native fixtures all use epoch 9.
agent-browser path: /opt/homebrew/bin/agent-browser
CDP browser: Chrome/148.0.7778.97, Protocol 1.3.
agent-browser CDP control: opened about:blank and returned title `about:blank`.
```

Source login checks after resume:

```text
ProContract: Supplier Post-Login Home loaded; account and logout links present.
Alibaba: homepage loaded and showed "Welcome to Alibaba.com, zac".
SeaRates: Logistics Explorer loaded with freight search controls; no blocking sign-in page.
eBay UK: homepage loaded and showed "Hi zac!".
```

Isolated E2E setup after resume:

```text
E2E_TS=20260512T195251Z
RUN_START_AT=2026-05-12T19:52:51+00:00
E2E_STATE_ROOT=/Users/zacfarrell/Desktop/Testproject1/state-e2e-machine-fork-20260512T195251Z
E2E_RUNTIME_DIR=/Users/zacfarrell/Desktop/Testproject1/runtime-e2e-machine-fork-20260512T195251Z
E2E_ARTIFACT_ROOT=/Users/zacfarrell/Desktop/Testproject1/artifacts-e2e-machine-fork-20260512T195251Z
E2E_LOG_ROOT=/Users/zacfarrell/Desktop/Testproject1/artifacts-e2e-machine-fork-20260512T195251Z/qa-logs
E2E_TARGET_DIR=/tmp/testproject1-target-machine-fork-20260512T195251Z
E2E_DAEMON=/tmp/testproject1-target-machine-fork-20260512T195251Z/debug/daemon
```

Daemon setup evidence:

```text
CARGO_TARGET_DIR=/tmp/testproject1-target-machine-fork-20260512T195251Z cargo build -p daemon --bin daemon
Finished `dev` profile [unoptimized + debuginfo] target(s) in 25.06s

run_logged daemon-dry-run "$E2E_DAEMON" --state-root "$E2E_STATE_ROOT" --runtime-dir "$E2E_RUNTIME_DIR" --artifact-root "$E2E_ARTIFACT_ROOT" --dry-run
dry-run bootstrap completed
PRAGMA user_version: 9
```

Dry-run log: `/Users/zacfarrell/Desktop/Testproject1/artifacts-e2e-machine-fork-20260512T195251Z/qa-logs/daemon-dry-run.log`.

## Stage Results

| Stage | Command | Result | Evidence |
|---|---|---|---|
| AGENTS.md read | `sed -n '1,260p' AGENTS.md`; `sed -n '261,620p' AGENTS.md`; `sed -n '621,980p' AGENTS.md` | PASS | Repo rules read before validation. |
| Git state | `git status --short --branch`; `git log --oneline -1` | PASS with dirty shared checkout | Original pause was on `main` at `4d1d1dd`; current live-stage verification ran on `main` at `9d15b9c`. Shared dirty artifacts were not stashed, reverted, deleted, or staged. |
| Schema epoch current-state check | `rg -n "...SCHEMA_EPOCH..." ...` | PASS | Daemon epoch 9; desktop bootstrap, query service, daemon bridge, and native fixtures now expect epoch 9. Regression guard `schema_epoch_defaults_match_daemon_authority_and_hello_payload` passed. |
| Origin freshness | `git fetch origin`; `git show origin/main:...` | PASS | Original pause saw `origin/main` at `4d1d1dd` with the epoch mismatch. Current verification ran on `main` at `9d15b9c`, with schema fix commit `6d113ad` present locally and on `origin/main`. |
| Toolchain preflight | `which cargo`; `which bun`; `which agent-browser`; version commands | PASS for browser setup | Cargo and Bun present; `agent-browser` now resolves at `/opt/homebrew/bin/agent-browser` and reports `agent-browser 0.27.0`. |
| CDP preflight | `curl -sfS -m 3 http://localhost:9222/json/version`; `agent-browser --cdp 9222 open "about:blank"`; `agent-browser --cdp 9222 get title`; `agent-browser --cdp 9222 snapshot` | PASS | Chrome/148.0.7778.97, Protocol 1.3, launched with `$HOME/.chrome-automation`; validation block printed `AGENT_BROWSER_CDP_PREFLIGHT_GREEN`. |
| Source login checks | `agent-browser --cdp 9222 open <source>` and `snapshot` for ProContract, Alibaba, SeaRates, eBay | PASS | All four source pages loaded without sign-in or captcha wall. |
| Isolated run setup | E2E env creation, isolated daemon build, `run_logged daemon-dry-run ... --dry-run` | PASS | Fresh roots created; daemon built in `/tmp/testproject1-target-machine-fork-20260512T195251Z`; dry-run completed; `PRAGMA user_version` returned 9. |
| Static/unit baseline | Required cargo/Bun gates | PASS | Rust fmt/check/clippy, daemon binary tests, all daemon integration test targets, package gates, desktop Bun test/build, `query_service`, `daemon_bridge`, and repaired `native_shell` passed. |
| Static/unit baseline | Targeted desktop gates plus Rust gates | PASS | `cargo test -p procintel-desktop --test query_service`; `cargo test -p procintel-desktop --test daemon_bridge`; `cargo test -p procintel-desktop --test native_shell`; daemon schema probe; `cargo fmt --check`; `cargo check --all-targets`; `cargo clippy --all-targets -- -D warnings` all passed. |
| Static/unit baseline | Current resumed desktop native shell gate | PASS | `resume-native-shell-specific.log`, `resume-native-shell-full.log`, and `resume-native-shell-stress.log` passed after commit `9d15b9c`. |
| Stage 1 - Contracts Finder initial live ingestion | `run_logged stage-01-contracts-finder ... --ingest contracts-finder` | BLOCKED THEN RESOLVED | Initial jobs `job-c3d5...` and fixer repro `job-2c44...` failed before result contract because Claude auth returned 401. Error manifest: `prompts/machine-fork-e2e-contracts-finder-claude-auth-error-manifest.md`. |
| Stage 1 - Contracts Finder auth repair probe | `run_logged resume-claude-auth-probe-2 claude -p "Return exactly OK." --dangerously-skip-permissions` | PASS | Probe returned `OK.` |
| Stage 1 - Contracts Finder retry | `run_logged stage-01-contracts-finder-retry-after-claude-auth env AGENT_BROWSER_CDP_PORT=9222 "$E2E_DAEMON" --state-root "$E2E_STATE_ROOT" --runtime-dir "$E2E_RUNTIME_DIR" --artifact-root "$E2E_ARTIFACT_ROOT" --ingest contracts-finder` | PASS | Job `job-fe439a72a132f4b43211dc71d6f544418880bd809d008511476461bdcb73e59b` completed with `notices=100 continuation=true`; `contract_notices` contains 104 `contracts-finder` rows after preserving previous retry history. |
| Stage 1 - ProContract CDP preflight | `agent-browser --cdp 9222 open "https://procontract.due-north.com/SupplierPostLoginHome#"` plus title/snapshot checks | PASS | Snapshot showed authenticated supplier home content, `SORELLON LIMITED`, opportunities, company summary, and vendor profile. |
| Stage 1 - ProContract live ingestion | `run_logged stage-01-procontract env AGENT_BROWSER_CDP_PORT=9222 "$E2E_DAEMON" --state-root "$E2E_STATE_ROOT" --runtime-dir "$E2E_RUNTIME_DIR" --artifact-root "$E2E_ARTIFACT_ROOT" --ingest procontract` | PASS | Job `job-7761e9efc2107ce4f9da40261177bfc4f3fb0e794f75df254a457c77e8a3cd92` completed with `notices=100 continuation=true`; `contract_notices` contains 100 `procontract` rows. |
| Stage 1 - FTS | Not run | DEFERRED | `--ingest fts` remains fixture-backed in current daemon dispatch and no current FTS fixture file/live daemon path was found. Deferred to keep this run real-data only. |
| Stage 1 validation | `run_logged stage-01-source-ingestion-validation sqlite3 ...` | PASS | `contracts-finder=104`, `procontract=100`, `procurement_processes=202`; source circuits closed; latest successful sessions have `terminated_after_result_contract`. Earlier failed Claude-auth jobs remain as documented forensic history. |
| Stages 2-3 and 5-8 deterministic pipeline - ProContract | `run_logged stages-02-08-pipeline-procontract "$E2E_DAEMON" ... --pipeline procontract` | PASS | `lots_created=100 prequalified=100 lots_qualified=100 reclassified=0 quoteability_scored=0 margins_assessed=0 relations=0`. |
| Stages 2-3 and 5-8 deterministic pipeline - Contracts Finder | `run_logged stages-02-08-pipeline-contracts-finder "$E2E_DAEMON" ... --pipeline contracts-finder` | PASS | `lots_created=104 prequalified=104 lots_qualified=104 reclassified=0 quoteability_scored=0 margins_assessed=0 relations=0`. |
| Stages 2-3 and 5-8 validation | `run_logged stages-02-08-validation sqlite3 ...` | PASS / PRODUCTS PENDING | Stage state shows `Normalize=300`, `extract lots=204`, `prequalify notice=204`, `qualify lot=204`, `relate=202`; `notice_lots=204`, `qualification_runs=408`, `raw_documents=204`, no pending extraction jobs. Product candidates/search profiles remain `0` until ProContract document enrichment/acquisition runs. |
| Stage 4a - ProContract register interest | `run_logged stage-04a-procontract-interest env AGENT_BROWSER_CDP_PORT=9222 "$E2E_DAEMON" --state-root "$E2E_STATE_ROOT" --runtime-dir "$E2E_RUNTIME_DIR" --artifact-root "$E2E_ARTIFACT_ROOT" --ingest procontract-interest` | BLOCKED / ERROR MANIFEST | The command produced a valid result contract then exited with `Error: unsupported ProContract registration action \`pending_registration\``. Result action counts: `registered=1`, `already_registered=2`, `pending_registration=8`, `skipped=89`. Workflow job `job-974dda4de782bc61ad5d5a91fa09d48152978ef9d0ccdbd01e95bd3f2dabb256` remains `running`; session `session-pty-5e1fb98f68771c12cb090dee2e0055d1e5f73d69345ab17faac4ed322981e8d9` completed with `terminated_after_result_contract`. Error manifest: `prompts/machine-fork-e2e-procontract-pending-registration-error-manifest.md`. |
| Stages 9-14 | Not run | PENDING | Awaiting document enrichment, product extraction, and spotlight selection. |
| Desktop UI | Not run | PENDING | Schema epoch blocker is repaired; UI validation waits until deterministic/downstream pipeline state is available. |
| Desktop Vite localhost | Not run | PENDING | Browser tooling/CDP preflight is green; localhost smoke remains for later in the run. |
| apps/web localhost smoke | Not run | DEFERRED | Browser tooling/CDP preflight is now green; localhost smoke remains for the resumed E2E run. |

## Spotlight Process

Not selected yet. Source ingestion has completed, but spotlight selection waits until the deterministic pipeline creates qualification, document, candidate, and search-profile state.

## CDP/Auth Events

Initial source login checks were not attempted during the original pause because `agent-browser` was missing and no browser was listening on `localhost:9222`.

First setup failure evidence:

```text
curl: (7) Failed to connect to localhost port 9222 after 0 ms: Couldn't connect to server
zsh:1: command not found: agent-browser
```

Repair result: source login-check commands were attempted after the browser/CDP preflight passed.

```bash
agent-browser --cdp 9222 open "https://procontract.due-north.com/SupplierPostLoginHome#"
agent-browser --cdp 9222 snapshot

agent-browser --cdp 9222 open "https://www.alibaba.com/"
agent-browser --cdp 9222 snapshot

agent-browser --cdp 9222 open "https://www.searates.com/logistics-explorer/"
agent-browser --cdp 9222 snapshot

agent-browser --cdp 9222 open "https://www.ebay.co.uk/"
agent-browser --cdp 9222 snapshot
```

Login-check observations, re-verified 2026-05-12T19:45:30Z:

```text
ProContract: reachable through CDP and authenticated; `/SupplierPostLoginHome#` loaded the supplier home page with account controls and company summary.
Alibaba: reachable through CDP and authenticated; home page showed `Welcome to Alibaba.com, zac` with search and product content. No captcha challenge was visible in the snapshot.
SeaRates: reachable through CDP; Logistics Explorer page loaded. No interactive captcha challenge was surfaced during this check.
eBay UK: reachable through CDP and authenticated; home page showed `Hi zac!`, Watchlist/My eBay controls, and search UI. No captcha challenge was visible in the snapshot.
```

Stage 1 blocker is not a source-browser authentication pause. The persistent CDP browser was still reachable during forensics, but the selected model CLI failed before browser/API work:

```text
claude -p "Return exactly OK." --dangerously-skip-permissions
Failed to authenticate. API Error: 401 Invalid authentication credentials
```

This produced no Contracts Finder result contract, no live notices, and no portal/source mutation.

Claude auth repair evidence:

```text
claude -p "Return exactly OK." --dangerously-skip-permissions
OK.
```

Contracts Finder retry evidence:

```text
contracts-finder live loop completed: job_id=job-fe439a72a132f4b43211dc71d6f544418880bd809d008511476461bdcb73e59b notices=100 continuation=true
```

ProContract authenticated-session evidence:

```text
Supplier home snapshot showed `SORELLON LIMITED`, opportunities, company details summary, and vendor profile controls.
```

ProContract live discovery evidence:

```text
procontract discovery completed: job_id=job-7761e9efc2107ce4f9da40261177bfc4f3fb0e794f75df254a457c77e8a3cd92 notices=100 continuation=true
```

Stage 4a was not an auth/CDP pause. The ProContract session was authenticated and the skill performed one real portal registration before the daemon persistence failure:

```text
registered: BEST START FAMILY HUBS OUTREACH SERVICES
notice_id: pc-ecd07d9f-5249-f111-813c-005056b64545
process_id: proc-pc-ecd07d9f-5249-f111-813c-005056b64545
registered_at: 2026-05-12T22:32:10Z
```

The result also reported two already-registered opportunities:

```text
26023 Traffic & Flood Monitoring CCTV Camera Network
509_27 Parking Management Solutions
```

The blocker is product/contract semantics: the final `registrations[]` contained eight `pending_registration` actions, which the daemon does not accept as terminal interest-registration states.

## Desktop And Localhost UI

Not executed.

Original desktop UI validation was blocked by the schema epoch mismatch:

```text
Daemon schema epoch: 9
Desktop expected schema epoch: 8
```

Schema epoch blocker is now repaired and verified:

```text
Daemon schema epoch: 9
Desktop expected schema epoch: 9
Regression guard: schema_epoch_defaults_match_daemon_authority_and_hello_payload passed
Desktop native gates: query_service, daemon_bridge, and native_shell passed
```

Desktop full-control validation can resume after Environment Preflight and source login checks.

Screenshot directory prepared:

```text
docs/pipeline/machine-fork-l0-l3-stages-1-14-e2e-screenshots/
```

No screenshots were captured before the pause.

## Operational Subsystems

Partially executed through the static baseline, isolated dry-run, and live Stage 1 source ingestion.

```text
workflow_jobs:
completed sync_source jobs: 3
failed sync_source jobs: 2, both documented Claude-auth missing-result attempts for Contracts Finder
pending sync_source jobs: 2, Contracts Finder continuation jobs

agent_session_logs:
session-pty-fd96d69bec88f438f2e6846e7efc79af93533de8639f0ce519944952de9081ad  procontract-discover  status=completed  exit_code=143  cleanup_verdict=terminated_after_result_contract
session-pty-97aca0564b676549924deac9e2ab604553200b9454fc0cde047be2e2c70c450a  contracts-finder      status=completed  exit_code=143  cleanup_verdict=terminated_after_result_contract

source_runtime_state:
contracts-finder/procontract/fts circuits remained closed with health `unknown`; no source captcha or browser-auth state was recorded.

contract_notices: contracts-finder=104, procontract=100
procurement_processes: 202
notice_lots: 204
qualification_runs: 408, including 118 goods-qualified rows
raw_documents: 204
document_extractions/product_candidates/search_profiles: 0, pending ProContract document enrichment/acquisition
error_manifests table rows: 0
```

Stage 4a blocker state:

```text
workflow_jobs:
job-974dda4de782bc61ad5d5a91fa09d48152978ef9d0ccdbd01e95bd3f2dabb256  sync_source  running  source  procontract-interest  attempt_count=1

agent_session_logs:
session-pty-5e1fb98f68771c12cb090dee2e0055d1e5f73d69345ab17faac4ed322981e8d9  skills/procurement-ingestion/procontract-register-interest/SKILL.md  status=completed  exit_code=143  cleanup_verdict=terminated_after_result_contract

process_stage_status:
no `interest_registration` rows were persisted
```

Result contract evidence:

```text
summary: total_evaluated=100, registered=1, already_registered=2, skipped_service=60, skipped_expiry=29, failed=0
action_counts: already_registered=2, pending_registration=8, registered=1, skipped=89
```

Full post-pipeline operational subsystem checks remain pending until Stages 2-14 and UI smoke checks complete.

## Quality Gates

Executed for schema epoch repair verification. Full live E2E gates remain pending until the resumed isolated run.

Current resumed baseline evidence:

```text
cargo fmt --check: PASS
cargo check --all-targets: PASS
cargo clippy --all-targets -- -D warnings: PASS
cargo test -p daemon --bin daemon: PASS, 202 tests
for t in crates/daemon/tests/*.rs; cargo test --test "$t": PASS for every target
packages/contracts-schema: bun run test PASS; bun run build PASS
packages/ui: bun run test PASS; bun run build PASS
packages/intelligence: bun run test PASS; bun run build PASS
packages/manifests: bun run test PASS; bun run build PASS
apps/desktop: bun run test PASS; bun run build PASS
cargo test -p procintel-desktop --test query_service: PASS
cargo test -p procintel-desktop --test daemon_bridge: PASS
cargo test -p procintel-desktop --test native_shell: initially failed once in required baseline, then passed after fix commit `9d15b9c`
```

The `native_shell` failure is preserved in `/Users/zacfarrell/Desktop/Testproject1/artifacts-e2e-machine-fork-20260512T195251Z/qa-logs/baseline-desktop-native-shell.log`. Follow-up controls passed:

```text
repro-desktop-native-shell-rerun-lock-filter.log: PASS, 1 test
repro-desktop-native-shell-full-rerun.log: PASS, 22 tests
```

Native-shell fix verification:

```text
resume-native-shell-specific.log: PASS, 1 test
resume-native-shell-full.log: PASS, 22 tests
resume-native-shell-stress.log: PASS, 20 full native_shell iterations
```

Live Stage 1 started after this verification. The Contracts Finder Claude auth blocker is now resolved and both real source-ingestion paths completed.

Schema repair validation:

```bash
cargo test -p procintel-desktop --test daemon_bridge schema_epoch
cargo test -p procintel-desktop --test query_service
cargo test -p procintel-desktop --test daemon_bridge
cargo test -p procintel-desktop --test native_shell
cargo test -p daemon --test sqlite_schema l4_t_sqlite_schema_001_open_state_db_applies_l4_migration
cargo fmt --check
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
```

Result: all passed.

Minimum full E2E resume gates:

```bash
cargo fmt --check
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test -p daemon --bin daemon
for t in crates/daemon/tests/*.rs; do
  cargo test --test "$(basename "$t" .rs)" || exit 1
done

cargo test -p procintel-desktop --test query_service
cargo test -p procintel-desktop --test daemon_bridge
cargo test -p procintel-desktop --test native_shell

(cd packages/contracts-schema && bun run test && bun run build)
(cd packages/ui && bun run test && bun run build)
(cd packages/intelligence && bun run test && bun run build)
(cd packages/manifests && bun run test && bun run build)
(cd apps/desktop && bun run test && bun run build)
```

## Final Decision

QA is PAUSED. Static/unit baseline blockers are resolved and verified. Real live Stage 1 source ingestion succeeded for Contracts Finder and ProContract, and deterministic Stages 2-3 and 5-8 baseline processing completed. Stage 4a ProContract register-interest reached the allowed real portal mutation path, then exposed a product/contract blocker after the skill returned unsupported `pending_registration` final actions.

Proven on this machine:

- The checkout is on `main` at `e4080be` plus current QA ledger/fixer-handoff edits.
- Cargo and Bun are present.
- `origin/main` contains schema epoch fix commit `6d113ad`.
- The daemon schema epoch is 9 because migration `0012_l4_sync_state` is present.
- Desktop/Tauri now expects schema epoch 9 in native bootstrap, query service, daemon bridge, and tests.
- The bridge regression guard compares the desktop default against daemon `DEFAULT_SCHEMA_EPOCH` and passed.
- Targeted desktop gates, daemon schema probe, `cargo fmt --check`, `cargo check --all-targets`, clippy, daemon bin tests, daemon integration tests, package gates, and desktop Bun gates passed.
- `agent-browser` is installed and resolves from `/opt/homebrew/bin/agent-browser`.
- `agent-browser --version` returns `agent-browser 0.27.0`.
- `agent-browser doctor --offline --quick` reports 6 pass, 0 warn, 0 fail.
- The persistent Chrome CDP listener is available on `localhost:9222`.
- `/json/version` returns Chrome/148.0.7778.97 and Protocol-Version 1.3.
- The CDP browser was launched with `$HOME/.chrome-automation`.
- `agent-browser --cdp 9222 open "about:blank"`, `get title`, and `snapshot` succeed.
- The browser preflight validation block exits 0 and prints `AGENT_BROWSER_CDP_PREFLIGHT_GREEN`.
- Login checks were attempted. ProContract, Alibaba, SeaRates, and eBay all load through CDP. ProContract, Alibaba, and eBay show authenticated state; SeaRates loads Logistics Explorer without surfacing an interactive captcha challenge.
- The isolated daemon dry-run passed with `PRAGMA user_version=9`.
- The initial Contracts Finder blocker was real: failed workflow jobs `job-c3d5...` and `job-2c44...` had PTY streams containing `Failed to authenticate. API Error: 401 Invalid authentication credentials`.
- The selected Claude CLI auth probe now returns `OK`.
- Contracts Finder retry job `job-fe439a72a132f4b43211dc71d6f544418880bd809d008511476461bdcb73e59b` completed with `notices=100 continuation=true`.
- ProContract job `job-7761e9efc2107ce4f9da40261177bfc4f3fb0e794f75df254a457c77e8a3cd92` completed with `notices=100 continuation=true`.
- SQLite now contains 104 `contracts-finder` notices, 100 `procontract` notices, and 202 procurement processes.
- FTS was deferred because the current daemon `fts` path remains fixture-backed and no current fixture file/live FTS daemon path was found; this run is preserving real-data semantics.
- Deterministic pipeline created 204 lots, 408 qualification rows, 204 raw document references, and 118 goods-qualified rows.
- There are no pending extraction jobs at this point; product candidates and search profiles remain pending until ProContract document enrichment/acquisition.
- Stage 4a used the authenticated ProContract session and performed one real registration: `BEST START FAMILY HUBS OUTREACH SERVICES`, notice `pc-ecd07d9f-5249-f111-813c-005056b64545`, process `proc-pc-ecd07d9f-5249-f111-813c-005056b64545`, registered at `2026-05-12T22:32:10Z`.
- The Stage 4a result contract reported `registered=1`, `already_registered=2`, `pending_registration=8`, and `skipped=89`.
- The daemon rejected the result with `unsupported ProContract registration action \`pending_registration\``.
- Workflow job `job-974dda4de782bc61ad5d5a91fa09d48152978ef9d0ccdbd01e95bd3f2dabb256` is left `running`, and no `interest_registration` stage rows were persisted.
- Error manifest `prompts/machine-fork-e2e-procontract-pending-registration-error-manifest.md` captures the exact commands, logs, session ID, result contract path, database state, codepath evidence, hypotheses, and acceptance criteria.

Current next point:

1. Source `/Users/zacfarrell/Desktop/Testproject1/state-e2e-machine-fork-20260512T195251Z/e2e-env.sh`.
2. Confirm the fixer has resolved the `pending_registration` final-action semantics and stale-running-job behavior.
3. Retry exactly: `run_logged stage-04a-procontract-interest-retry-after-pending-action-fix env AGENT_BROWSER_CDP_PORT=9222 "$E2E_DAEMON" --state-root "$E2E_STATE_ROOT" --runtime-dir "$E2E_RUNTIME_DIR" --artifact-root "$E2E_ARTIFACT_ROOT" --ingest procontract-interest`.
4. Validate `interest_registration` stage state and latest `procontract-interest` workflow rows, explicitly checking that the earlier `BEST START FAMILY HUBS OUTREACH SERVICES` registration is treated idempotently.
5. Continue to `procontract-enrich` if the portal session remains authenticated and Stage 4a persists successfully.
