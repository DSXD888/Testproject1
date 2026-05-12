# Machine Fork L0-L3 Stages 1-14 E2E Validation Manifest

Generated: 2026-05-12T18:46:53Z
Last verified: 2026-05-12T20:39:49Z
Status: PAUSED - Stage 1 Contracts Finder live ingestion blocked by Claude CLI 401 before result contract
Git HEAD at verification: 9d15b9c fix: serialize native shell test hooks
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

## Error Manifests Created

| Path | Stage | Blocking Symptom | Status |
|---|---|---|---|
| `prompts/machine-fork-e2e-schema-epoch-mismatch-error-manifest.md` | Critical current-state checks / Desktop validation precondition | Daemon `DEFAULT_SCHEMA_EPOCH=9`; desktop expected epoch 8 and rejected mismatched state/handshake. | Resolved by commit `6d113ad`; verified 2026-05-12T19:43:18Z. |
| `prompts/machine-fork-e2e-agent-browser-missing-error-manifest.md` | Environment preflight / Browser testing precondition | `agent-browser` command not found, so CDP preflight and daemon-driven browser skills cannot be validated. | Resolved by machine setup repair on 2026-05-12T19:29:53Z. |
| `prompts/machine-fork-e2e-desktop-native-shell-rerun-lock-error-manifest.md` | Static/unit baseline / Desktop native shell | Intermittent failure: rerun-lock test expected stopped PID `[4242]` but observed `[]` in the required full `native_shell` target. | Resolved by commit `9d15b9c`; verified with exact test, full target, and 20x stress loop. |
| `prompts/machine-fork-e2e-contracts-finder-claude-auth-error-manifest.md` | Stage 1 / Contracts Finder live ingestion | Selected Claude CLI returns `401 Invalid authentication credentials`, causing the PTY skill to exit before writing a result contract. | Open; QA paused before ProContract ingestion. |

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
| Stage 1 - Contracts Finder live ingestion | `run_logged stage-01-contracts-finder env AGENT_BROWSER_CDP_PORT=9222 "$E2E_DAEMON" --state-root "$E2E_STATE_ROOT" --runtime-dir "$E2E_RUNTIME_DIR" --artifact-root "$E2E_ARTIFACT_ROOT" --ingest contracts-finder` | BLOCKED | Job `job-c3d5b06af6f38699d94beb2d5a9458e7038d457edb0a291526d0e28108d29807` failed with `contracts-finder skill did not produce a result contract`; PTY stream for `session-pty-99e3c444b8b8f752709101a88a2ec48c6fdeacdc090af0c9a58ff69a2d29d110` contains `Failed to authenticate. API Error: 401 Invalid authentication credentials`. Error manifest: `prompts/machine-fork-e2e-contracts-finder-claude-auth-error-manifest.md`. |
| Stage 1 - ProContract live ingestion | Not run | PAUSED | Not attempted because the run must stop on the Contracts Finder real blocker. |
| Stages 2-14 | Not run | DEFERRED | Paused before deterministic pipeline and downstream live stages. |
| Desktop UI | Not run | BLOCKED BEHIND STAGE 1 | Schema epoch blocker is repaired, but UI validation waits until the live pipeline has data or a documented structured no-result path. |
| Desktop Vite localhost | Not run | BLOCKED BEHIND STAGE 1 | Browser tooling/CDP preflight is green and schema epoch mismatch is repaired; localhost smoke remains for the resumed E2E run. |
| apps/web localhost smoke | Not run | DEFERRED | Browser tooling/CDP preflight is now green; localhost smoke remains for the resumed E2E run. |

## Spotlight Process

Not selected. The run paused before source ingestion and deterministic pipeline execution.

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

Partially executed through the static baseline, isolated dry-run, and first live Stage 1 attempt.

```text
workflow_jobs:
job-c3d5b06af6f38699d94beb2d5a9458e7038d457edb0a291526d0e28108d29807  sync_source  failed  source  contracts-finder  attempt_count=1  last_error="contracts-finder skill did not produce a result contract"

agent_session_logs:
session-pty-99e3c444b8b8f752709101a88a2ec48c6fdeacdc090af0c9a58ff69a2d29d110  skill=skills/procurement-ingestion/contracts-finder/SKILL.md  mode=pty  status=completed  exit_code=1  cleanup_verdict=clean  terminal_output_bytes=131

source_runtime_state:
contracts-finder/procontract/fts circuits remained closed with health `unknown`; no source captcha or browser-auth state was recorded.

contract_notices: 0
procurement_processes: 0
error_manifests table rows: 0
```

Full post-pipeline operational subsystem checks remain pending because QA paused at live Stage 1 before any notices were ingested.

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

Live Stage 1 started after this verification and is now paused on the Contracts Finder Claude auth blocker.

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

QA is PAUSED. Static/unit baseline blockers are resolved and verified, and the first real live Stage 1 Contracts Finder ingestion was attempted. It did not reach Contracts Finder data retrieval because the selected Claude CLI failed authentication before the skill could write `result.json`.

Proven on this machine:

- The checkout is on `main` at `9d15b9c`.
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
- The live Contracts Finder command created failed workflow job `job-c3d5b06af6f38699d94beb2d5a9458e7038d457edb0a291526d0e28108d29807`.
- The associated PTY session was `session-pty-99e3c444b8b8f752709101a88a2ec48c6fdeacdc090af0c9a58ff69a2d29d110`.
- The PTY stream contains `Failed to authenticate. API Error: 401 Invalid authentication credentials`.
- A direct `claude -p "Return exactly OK." --dangerously-skip-permissions` auth probe fails with the same 401.
- No live notices, procurement processes, result contract, or run manifest were produced for Contracts Finder.

Current next point:

1. Repair or verify the Claude CLI/API authentication state used by `/opt/homebrew/bin/claude`, or make an explicit product decision to use a different authenticated adapter.
2. Source `/Users/zacfarrell/Desktop/Testproject1/state-e2e-machine-fork-20260512T195251Z/e2e-env.sh`.
3. Re-run `run_logged stage-01-contracts-finder-retry-after-claude-auth env AGENT_BROWSER_CDP_PORT=9222 "$E2E_DAEMON" --state-root "$E2E_STATE_ROOT" --runtime-dir "$E2E_RUNTIME_DIR" --artifact-root "$E2E_ARTIFACT_ROOT" --ingest contracts-finder`.
4. Run the Stage 1 validation query.
5. Continue to ProContract ingestion only if Contracts Finder now writes a valid result contract or a valid structured no-result/blocker contract.
