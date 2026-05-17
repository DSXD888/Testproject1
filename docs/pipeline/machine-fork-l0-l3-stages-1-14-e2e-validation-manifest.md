# Machine Fork L0-L3 Stages 1-14 E2E Validation Manifest

Generated: 2026-05-12T18:46:53Z
Last verified: 2026-05-17T14:46:35Z
Status: PAUSED - Stage 9 marketplace preflight is blocked on Alibaba authentication in the repaired persistent CDP browser
Git HEAD at verification: 7957146 docs: add stage 9 cdp repair handoff
State root: `/Users/zacfarrell/Desktop/Testproject1/state-e2e-machine-fork-20260512T195251Z`
Runtime dir: `/Users/zacfarrell/Desktop/Testproject1/runtime-e2e-machine-fork-20260512T195251Z`
Artifact root: `/Users/zacfarrell/Desktop/Testproject1/artifacts-e2e-machine-fork-20260512T195251Z`
QA log root: `/Users/zacfarrell/Desktop/Testproject1/artifacts-e2e-machine-fork-20260512T195251Z/qa-logs`
Daemon binary: `/tmp/testproject1-target-machine-fork-20260512T195251Z/debug/daemon`
CDP browser: repaired during Stage 9 preflight; Chrome/148.0.7778.168, Protocol 1.3, listening on `127.0.0.1:9222`, launched with `$HOME/.chrome-automation`

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
| 2026-05-12T22:08:47Z | Resume after Stage 4a pending-registration fix | Commit `45546c2` updated ProContract register-interest persistence and skill/result schema semantics. | Rebuilt the pinned daemon binary, inspected the fix retry, validated `interest_registration` rows, and ran focused regression tests. No additional portal mutation was triggered by this QA resume. | Continue to Stage 4b ProContract enrichment/get-documents. |
| 2026-05-12T22:10:15Z | Stage 4b - ProContract enrichment preflight | Persistent CDP browser is reachable, but `https://procontract.due-north.com/SupplierPostLoginHome#` redirected to `Login/Index?ReturnUrl=%2fSupplierPostLoginHome`; page title is `Log In`. | Recorded as first-pass operator ProContract auth repair pause. No enrichment command was run. | Operator logs in or clears the challenge in the Chrome window attached to `localhost:9222`, then QA retries the same ProContract CDP preflight and runs Stage 4b. |
| 2026-05-12T22:33:04Z | Stage 4b - ProContract enrichment | After operator login, CDP preflight reached `Supplier Post-Login Home`, but `--ingest procontract-enrich` timed out after 1200 seconds without a result contract. Post-timeout browser snapshot was still inside an authenticated activity dashboard, not a login/captcha page. | Created `prompts/machine-fork-e2e-procontract-enrich-timeout-error-manifest.md` and paused. | After fixer repairs Stage 4b timeout/missing-result handling, retry `stage-04b-procontract-enrich-retry-after-timeout-fix` against the existing E2E roots. |
| 2026-05-12T23:38:14Z | Resume after Stage 4b timeout fix | Commit `413c5a7` updated ProContract enrichment missing-result handling and skill bounded-stall instructions; pinned daemon rebuild and focused regressions passed. | Direct selected-agent probe failed with `You've hit your limit - resets 2:50am (Europe/London)`. Created `prompts/machine-fork-e2e-claude-quota-blocker-error-manifest.md`; no live portal retry was run. | After Claude quota resets/restores, rerun the direct Claude probe, ProContract CDP preflight, then retry Stage 4b with `stage-04b-procontract-enrich-retry-after-timeout-fix`. |
| 2026-05-14T14:53:03Z | Stage 4b - ProContract enrichment follow-up retry | Claude probe returned `OK`, CDP and ProContract supplier-home preflight passed, and the exact Stage 4b retry was run. It still timed out after 1200 seconds without `result.json`; post-timeout snapshot remained on an authenticated ProContract activity dashboard. | Created `prompts/machine-fork-e2e-procontract-enrich-timeout-followup-error-manifest.md`. The daemon durability fix worked: latest job is `failed` and internal `error_manifests` row `error-948338ee-8651-4aa2-a900-a139b70d7b87` exists. | After fixer updates the live enrichment skill to emit a valid enrichment or blocked result for this activity-dashboard state, retry with `stage-04b-procontract-enrich-retry-after-followup-fix`. |
| 2026-05-14T15:42:46Z | Stage 4b - Document extraction after ProContract enrichment | The current fix worked for the live enrichment timeout: `stage-04b-procontract-enrich-retry-after-followup-fix` produced a valid result contract, persisted one tender document set, and downloaded three real ProContract DOCX files. The immediate extraction drain then failed all three `extract_document_structure` jobs with `Docling runtime unavailable for wrapper /Users/zacfarrell/Desktop/Testproject1/scripts/docling-extract.py`; Python cannot import `docling`, `$HOME/.docling-venv/bin/docling` is absent, and `$HOME/.cache/docling/models` is absent. | Created `prompts/machine-fork-e2e-docling-runtime-unavailable-error-manifest.md`. No product code was changed. | After Docling runtime/model artifacts are installed or otherwise configured, source the existing E2E env, verify `scripts/docling-extract.py --version`, then retry the approved Stage 4b/extraction path and validate `document_extractions`, semantic extraction, product candidates, and search profiles. |
| 2026-05-16T15:57:20Z | Stage 4b - Docling setup and extraction retry | Installed Docling 2.93.0 into `/Users/zacfarrell/.docling-venv/bin/python`, prefetched models into `/Users/zacfarrell/.cache/docling/models`, retried the preserved extraction path, and the latest structural extraction jobs completed. The pipeline now has `document_extractions=3`, `semantic_extractions=3`, and `product_candidates=5`; `search_profiles=0`. One new semantic job failed with `missing string lots[].lot_number` because the preserved `pdf-extraction/tender-specification` `result.json` has numeric `lot_number=1`. | Created `prompts/machine-fork-e2e-semantic-lot-number-type-error-manifest.md`. No product code was changed for the Docling setup fix. | Fix the semantic lot-number contract/type mismatch, then retry only the failed semantic job or the smallest supported extraction continuation against the existing E2E roots. |
| 2026-05-16T16:41:16Z | Resume check after semantic lot-number blocker | Operator said continue. Fetched `origin/main` and verified the checkout is still at `1e380fb docs: record docling setup validation` with no product-code fix for the semantic lot-number blocker. The failed semantic job remains `job-479bed51-0fea-46ee-a207-51e94c641c0b` with `missing string lots[].lot_number`; there are no pending extraction jobs and `search_profiles=0`. | Updated this validation ledger. No daemon command was retried because the recorded product blocker is still present and no fix commit is available. | Fix the semantic lot-number contract/type mismatch, then retry only `job-479bed51-0fea-46ee-a207-51e94c641c0b` or the smallest supported extraction continuation against the existing E2E roots. |
| 2026-05-17T13:40:56Z | Stage 4b - Semantic lot-number fix and continuation | The daemon now canonicalizes string-or-integer lot identifiers at the semantic domain-write boundary while preserving the tender-spec schema as string-only. Rebuilt the pinned daemon, requeued only semantic job `job-479bed51-0fea-46ee-a207-51e94c641c0b`, and replayed the preserved numeric-lot `result.json` through the daemon contract session path. | The failed semantic job is now `completed`; semantic extraction rows increased to 4; `product_candidates=5`. Re-running `--pipeline procontract` scored 5 candidates and produced `search_profiles=5` for the target process. | Continue to Stage 9 supplier price comparison / marketplace search against the existing E2E roots. |
| 2026-05-17T14:00:17Z | Stage 9 - Marketplace CDP preflight | `stage-09-cdp-version` failed before any marketplace source command: `curl -sfS -m 3 http://localhost:9222/json/version` returned connection refused. The selected agent probe returned `OK`, so this is a persistent browser/CDP setup issue, not a model CLI issue. | Recorded as first Stage 9 CDP repair pause. No Stage 9 daemon command was run, and no error manifest was created on this first CDP occurrence. | Operator launches or repairs the Chrome profile attached to localhost:9222, then QA re-runs `/json/version`, marketplace login checks, and Stage 9 marketplace coverage. |
| 2026-05-17T14:46:35Z | Stage 9 - CDP repair and marketplace source preflight | Relaunched the canonical Chrome automation profile on port 9222. `/json/version` and `lsof` passed, and `agent-browser --cdp 9222` controlled `about:blank`. Initial marketplace opens failed with `net::ERR_BLOCKED_BY_CLIENT` because `agent-browser` had targeted Chrome's Gemini side panel; selecting stable tab `t1` restored normal navigation. Amazon UK, eBay UK, Gem Imports, and MX Wholesale loaded through CDP. Alibaba loaded but the snapshot showed `Sign in` and generic `Welcome to Alibaba.com`, not the authenticated account state required by `marketplace-search/alibaba`. | Recorded as first-pass operator Alibaba auth repair pause. No `--ingest marketplace-coverage` or source-specific Stage 9 daemon command was run after this auth finding. Logs include `stage-09-cdp-version-after-repair`, `stage-09-agent-browser-select-t1-after-blocked-client`, `stage-09-agent-browser-example-open-after-t1-select`, and `stage-09-alibaba-snapshot-after-cdp-target-repair`. | Operator logs in or clears the Alibaba challenge in the Chrome window attached to `localhost:9222`, then QA reruns CDP preflight, selects real page tab `t1` if needed, retries the Alibaba source check, and only then runs Stage 9 marketplace coverage. |

## Error Manifests Created

| Path | Stage | Blocking Symptom | Status |
|---|---|---|---|
| `prompts/machine-fork-e2e-schema-epoch-mismatch-error-manifest.md` | Critical current-state checks / Desktop validation precondition | Daemon `DEFAULT_SCHEMA_EPOCH=9`; desktop expected epoch 8 and rejected mismatched state/handshake. | Resolved by commit `6d113ad`; verified 2026-05-12T19:43:18Z. |
| `prompts/machine-fork-e2e-agent-browser-missing-error-manifest.md` | Environment preflight / Browser testing precondition | `agent-browser` command not found, so CDP preflight and daemon-driven browser skills cannot be validated. | Resolved by machine setup repair on 2026-05-12T19:29:53Z. |
| `prompts/machine-fork-e2e-desktop-native-shell-rerun-lock-error-manifest.md` | Static/unit baseline / Desktop native shell | Intermittent failure: rerun-lock test expected stopped PID `[4242]` but observed `[]` in the required full `native_shell` target. | Resolved by commit `9d15b9c`; verified with exact test, full target, and 20x stress loop. |
| `prompts/machine-fork-e2e-contracts-finder-claude-auth-error-manifest.md` | Stage 1 / Contracts Finder live ingestion | Selected Claude CLI returned `401 Invalid authentication credentials`, causing the PTY skill to exit before writing a result contract. | Resolved by Claude auth repair; direct probe returned `OK` and Contracts Finder retry completed. |
| `prompts/machine-fork-e2e-procontract-pending-registration-error-manifest.md` | Stage 4a / ProContract register interest | Skill result contract included eight `pending_registration` final actions after one real registration; daemon persistence rejects that action and leaves the workflow job `running`. | Resolved by commit `45546c2`; validated 2026-05-12T22:08:47Z. |
| `prompts/machine-fork-e2e-procontract-enrich-timeout-error-manifest.md` | Stage 4b / ProContract enrichment | PTY skill exceeded 1200-second timeout and produced no `result.json` or run manifest; job remained `running` and no tender enrichment/document rows were persisted. | Resolved by commit `413c5a7`; focused regressions passed 2026-05-12T23:38:14Z, live retry blocked by Claude quota. |
| `prompts/machine-fork-e2e-claude-quota-blocker-error-manifest.md` | Stage 4b / Selected agent CLI preflight | `claude -p "Return exactly OK." --dangerously-skip-permissions` returns `You've hit your limit - resets 2:50am (Europe/London)`. | Resolved; `resume-claude-quota-probe-after-reset` returned `OK` on 2026-05-14. |
| `prompts/machine-fork-e2e-procontract-enrich-timeout-followup-error-manifest.md` | Stage 4b / ProContract enrichment follow-up retry | After Claude quota recovered and ProContract auth preflight passed, the fixed Stage 4b retry still timed out without `result.json`; no tender document rows were produced. | Resolved by current skill fix; verified 2026-05-14T15:42:46Z with a valid result contract and persisted documents. |
| `prompts/machine-fork-e2e-docling-runtime-unavailable-error-manifest.md` | Stage 4b / Document extraction after ProContract enrichment | Three real ProContract DOCX files were downloaded, but all three `extract_document_structure` jobs failed because Docling package/model artifacts are absent on this machine. | Resolved by machine setup on 2026-05-16: Docling 2.93.0 installed in the default venv and models prefetched to `/Users/zacfarrell/.cache/docling/models`; latest structural extraction retries completed. |
| `prompts/machine-fork-e2e-semantic-lot-number-type-error-manifest.md` | Stage 4b / Semantic extraction after Docling setup | `pdf-extraction/tender-specification` result for one document emitted numeric `lots[].lot_number`; daemon persistence requires a string and failed the job with `missing string lots[].lot_number`. | Resolved by the semantic lot-number canonicalization fix; verified 2026-05-17 by replaying the preserved result through job `job-479bed51-0fea-46ee-a207-51e94c641c0b` and generating 5 search profiles. |

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
| Stage 4a - ProContract register interest fix verification | `run_logged stage-04a-procontract-interest-retry-after-pending-action-fix ... --ingest procontract-interest`; `run_logged resume-daemon-rebuild-after-procontract-fix env CARGO_TARGET_DIR="$E2E_TARGET_DIR" cargo build -p daemon --bin daemon`; focused cargo tests | PASS | Fix retry job `job-2abb48c0309b63a8aea49b89d1461183719d8c5857cf0f06fc85d570a25c0943` completed with `evaluations=8 persisted=8`; result contained only `pending_registration=8` and no new registered/already-registered actions. Stage state now has `completed=3`, `pending=16`, `skipped=89`; latest retryable-candidate check excluding completed/succeeded/skipped/pending returns 0. Regression logs: `regression-procontract-interest-agent-ingestion.log`, `regression-procontract-interest-pending-unit.log`, and `regression-procontract-interest-unknown-action-unit.log`. |
| Stage 4b - ProContract enrichment CDP preflight | `run_logged stage-04b-procontract-cdp-version curl -sfS -m 3 http://localhost:9222/json/version`; `run_logged stage-04b-procontract-login-open agent-browser --cdp 9222 open "https://procontract.due-north.com/SupplierPostLoginHome#"`; `run_logged stage-04b-procontract-login-title agent-browser --cdp 9222 get title` | PAUSED FOR OPERATOR AUTH | CDP is healthy with Chrome/148.0.7778.97, but ProContract redirected to `https://procontract.due-north.com/Login/Index?ReturnUrl=%2fSupplierPostLoginHome` and page title `Log In`. This is the first Stage 4b auth/session pause, so no error manifest was created and `--ingest procontract-enrich` was not run. |
| Stage 4b - ProContract enrichment after operator login | `run_logged stage-04b-procontract-login-open-after-operator-login agent-browser --cdp 9222 open "https://procontract.due-north.com/SupplierPostLoginHome#"`; `run_logged stage-04b-procontract-enrich env AGENT_BROWSER_CDP_PORT=9222 "$E2E_DAEMON" ... --ingest procontract-enrich` | BLOCKED / ERROR MANIFEST | Login preflight passed with `Supplier Post-Login Home`. The enrichment command ran until `timeout_seconds=1200`, then exited with `Error: procontract enrichment skill did not produce a result contract`. Latest job `job-cf642ec3d9e68b1fe4e28e94eb9eaca37126e59f8dc7c300b091263ac0109755` remains `running`; session `session-pty-33fd5d63d698179f0b393ea6f8a3481a2c00f2191e3773017ee8e836aa4b7757` has `cleanup_verdict=terminated_after_timeout`, `terminal_output_bytes=77`, no `run_manifest_path`, and no `result.json`. No tender enrichment rows or downloaded raw document files were persisted. Error manifest: `prompts/machine-fork-e2e-procontract-enrich-timeout-error-manifest.md`. |
| Stage 4b - ProContract enrichment timeout fix verification | `run_logged resume-daemon-rebuild-after-procontract-enrich-fix env CARGO_TARGET_DIR="$E2E_TARGET_DIR" cargo build -p daemon --bin daemon`; focused daemon tests; `run_logged resume-claude-quota-probe-after-procontract-enrich-fix claude -p "Return exactly OK." --dangerously-skip-permissions` | FIX VERIFIED LOCALLY / LIVE RETRY PAUSED | Commit `413c5a7` was present and the pinned daemon rebuilt. Tests `procontract_enrich_missing_result_marks_job_failed`, `procontract_enrich_timeout_missing_result_marks_job_failed`, `procontract_enrich_blocked_result_is_durable`, and `persist_tender_enrichments_updates_notice_and_documents` passed. Live Stage 4b retry was not run because the selected Claude CLI returned `You've hit your limit - resets 2:50am (Europe/London)`. Error manifest: `prompts/machine-fork-e2e-claude-quota-blocker-error-manifest.md`. |
| Stage 4b - ProContract enrichment follow-up retry | `run_logged resume-claude-quota-probe-after-reset claude -p "Return exactly OK." --dangerously-skip-permissions`; `run_logged stage-04b-procontract-login-open-after-claude-quota-reset agent-browser --cdp 9222 open "https://procontract.due-north.com/SupplierPostLoginHome#"`; `run_logged stage-04b-procontract-enrich-retry-after-timeout-fix env AGENT_BROWSER_CDP_PORT=9222 "$E2E_DAEMON" ... --ingest procontract-enrich` | BLOCKED / ERROR MANIFEST | Claude returned `OK`; ProContract supplier home loaded and title was `Supplier Post-Login Home`. The enrichment retry still timed out after 1200 seconds with `procontract enrichment skill did not produce a result contract`. Latest job `job-33fb160456947771375daba10501662ef301e46b280bcc6dfcc55c1f217405bf` is now correctly `failed`, and internal error manifest row `error-948338ee-8651-4aa2-a900-a139b70d7b87` was created. No `tender_enrichment`, `tender_document_sets`, downloaded `raw_documents`, extraction, product candidate, or search profile rows were produced. Error manifest: `prompts/machine-fork-e2e-procontract-enrich-timeout-followup-error-manifest.md`. |
| Stage 4b - ProContract enrichment after current follow-up fix | `run_logged resume-daemon-rebuild-after-procontract-enrich-followup-fix env CARGO_TARGET_DIR="$E2E_TARGET_DIR" cargo build -p daemon --bin daemon`; `run_logged regression-procontract-enrich-missing-result-unit-after-followup-fix cargo test -p daemon --bin daemon procontract_enrich_missing_result_marks_job_failed`; `run_logged regression-procontract-enrich-blocked-result-unit-after-followup-fix cargo test -p daemon --bin daemon procontract_enrich_blocked_result_is_durable`; `run_logged resume-claude-probe-before-procontract-enrich-followup claude -p "Return exactly OK." --dangerously-skip-permissions`; `run_logged stage-04b-procontract-login-open-after-enrich-followup-fix agent-browser --cdp 9222 open "https://procontract.due-north.com/SupplierPostLoginHome#"`; `run_logged stage-04b-procontract-enrich-retry-after-followup-fix env AGENT_BROWSER_CDP_PORT=9222 "$E2E_DAEMON" ... --ingest procontract-enrich` | ENRICHMENT PASS / EXTRACTION BLOCKED | The pinned daemon rebuild passed, focused regressions passed, Claude returned `OK`, and ProContract CDP preflight reached `Supplier Post-Login Home`. Live Stage 4b produced a valid result contract and exited 0: `procontract tender enrichment completed: job_id=job-109da01929119faeea0cebfb7e4785c12d3153038521766e980900aa5e0c3ae6 enriched=1 extractions=3`. SQLite now has `tender_enrichment completed=1`, `tender_document_sets=1`, and `raw_docs_with_files=3`. However all three extraction jobs failed with `Docling runtime unavailable for wrapper /Users/zacfarrell/Desktop/Testproject1/scripts/docling-extract.py`; `document_extractions=0`, `semantic_extractions=0`, `product_candidates=0`, `search_profiles=0`. Error manifest: `prompts/machine-fork-e2e-docling-runtime-unavailable-error-manifest.md`. |
| Stage 4b - Docling setup and extraction retry | `run_logged stage-04b-docling-model-download "$HOME/.docling-venv/bin/docling-tools" models download`; focused daemon regressions; `run_logged stage-04b-procontract-enrich-retry-after-docling-setup env AGENT_BROWSER_CDP_PORT=9222 "$E2E_DAEMON" ... --ingest procontract-enrich`; post-retry SQLite validation | DOCLING FIXED / SEMANTIC BLOCKED | Docling 2.93.0 is available at `/Users/zacfarrell/.docling-venv/bin/python`; model artifacts exist at `/Users/zacfarrell/.cache/docling/models`; no daemon env overrides are required. Latest structural extraction retries completed and created `document_extractions=3`; downstream reached `semantic_extractions=3` and `product_candidates=5`. One semantic job failed with `missing string lots[].lot_number`; preserved result JSON contains numeric `lot_number=1` despite the tender-spec schema requiring a string. Error manifest: `prompts/machine-fork-e2e-semantic-lot-number-type-error-manifest.md`. |
| Resume check after semantic lot-number blocker | `git fetch origin`; `git log --oneline -8 --decorate --all --max-count=12`; `sqlite3 ... workflow_jobs WHERE kind='extract_document_semantics'`; Docling venv version probe | STILL BLOCKED | `origin/main` and local `main` are at `1e380fb docs: record docling setup validation`; no code fix for the numeric lot-number contract mismatch is present. `/Users/zacfarrell/.docling-venv/bin/python scripts/docling-extract.py --version` returns `2.93.0`, so Docling remains healthy. SQLite still shows `extract_document_semantics completed=3 failed=1`, failed job `job-479bed51-0fea-46ee-a207-51e94c641c0b`, `pending_extraction_jobs=0`, and `search_profiles=0`. |
| Stage 4b - Semantic lot-number fix verification | `run_logged stage-04b-semantic-lot-number-single-job-retry-after-fix env AGENT_BROWSER_CDP_PORT=9222 SEMANTIC_REPLAY_RESULT_JSON="$E2E_ARTIFACT_ROOT/sessions/2026/05/session-semantic-80e2a70dc3f7de84a29da4d65e5660042ea33eb7b2a2124d17ac92408ff0da0c/workdir/result.json" DAEMON_CLAUDE_PROGRAM=/tmp/testproject1-semantic-lot-replay-claude-20260517 "$E2E_DAEMON" ... --retry-workflow-job job-479bed51-0fea-46ee-a207-51e94c641c0b`; `run_logged stage-05-08-pipeline-procontract-after-semantic-lot-number-fix "$E2E_DAEMON" ... --pipeline procontract`; SQLite validation | PASS / READY FOR STAGE 9 | The pinned daemon rebuilt with the fix and retried only failed semantic job `job-479bed51-0fea-46ee-a207-51e94c641c0b` using the preserved result JSON. The job is now `completed`; extract semantics summary is `completed=4`, and historical structural failures remain preserved. Counts after deterministic continuation: `document_extractions=3`, `semantic_extractions=4`, `product_candidates=5`, `search_profiles=5`; all 5 target-process candidates now have search profiles and marginal search eligibility. |
| Stage 9 - Marketplace preflight | `run_logged stage-09-cdp-version curl -sfS -m 3 http://localhost:9222/json/version`; `run_logged stage-09-claude-probe claude -p "Return exactly OK." --dangerously-skip-permissions` | PAUSED FOR OPERATOR CDP REPAIR | CDP preflight failed with `curl: (7) Failed to connect to localhost port 9222`; selected-agent probe returned `OK`. No marketplace source or daemon Stage 9 command was run. |
| Stages 9-14 | Not run | PENDING | Stage 9 is ready from pipeline data, but marketplace CDP preflight is paused until the persistent browser on localhost:9222 is restored. |
| Desktop UI | Not run | PENDING | Schema epoch blocker is repaired; UI validation waits until deterministic/downstream pipeline state is available. |
| Desktop Vite localhost | Not run | PENDING | Browser tooling/CDP preflight is green; localhost smoke remains for later in the run. |
| apps/web localhost smoke | Not run | DEFERRED | Browser tooling/CDP preflight is now green; localhost smoke remains for the resumed E2E run. |

## Spotlight Process

Spotlight process: `proc-pc-ecd07d9f-5249-f111-813c-005056b64545` / notice `pc-ecd07d9f-5249-f111-813c-005056b64545`, `BEST START FAMILY HUBS OUTREACH SERVICES`, buyer `Oldham Council`.

Selection basis: this is the real ProContract opportunity for which Stage 4a registered interest, Stage 4b downloaded real DOCX tender documents, Docling structural extraction completed, semantic extraction completed, and the deterministic pipeline produced 5 search profiles. It is service-heavy rather than an ideal physical-goods tender, but it is the only fully enriched/document-extracted real ProContract process in this E2E state and is therefore the current end-to-end validation target.

Current candidate/search state:

```text
Community Development outreach service                         quoteability_score_bps=6000
Strengthening Parental Confidence through Play Based Learning  quoteability_score_bps=5800
Support for Dads/Male Carers                                   quoteability_score_bps=5300
Mental Health Peer Support                                     quoteability_score_bps=5300
Parenting Peer Support Service                                 quoteability_score_bps=5300
search_profiles=5, search_eligibility_state=marginal
```

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

Stage 4b ProContract auth pause:

```text
CDP: reachable, Chrome/148.0.7778.97
open: https://procontract.due-north.com/SupplierPostLoginHome#
actual URL: https://procontract.due-north.com/Login/Index?ReturnUrl=%2fSupplierPostLoginHome
title: Log In
```

This is the first ProContract session/auth pause for Stage 4b after the Stage 4a fix, so it is classified as operator CDP/auth repair rather than a product blocker.

After the operator logged in, the same ProContract preflight passed:

```text
open: https://procontract.due-north.com/SupplierPostLoginHome#
actual URL: https://procontract.due-north.com/SupplierPostLoginHome#
title: Supplier Post-Login Home
snapshot: SORELLON LIMITED supplier home, Opportunities, Company details summary, Vendor profile
```

The subsequent Stage 4b timeout was not classified as auth/CDP. Post-timeout snapshot still showed an authenticated ProContract activity dashboard with `Messages & clarifications (0)`, `Your response`, `Start my response`, `Opt out`, and `Indicate your intent to respond`.

Stage 4b timeout fix was present before the next retry, but the selected agent CLI was unavailable:

```text
claude -p "Return exactly OK." --dangerously-skip-permissions
You've hit your limit - resets 2:50am (Europe/London)
```

No live portal retry was run after that probe.

On the next resume, Claude and ProContract were both available:

```text
resume-claude-quota-probe-after-reset: OK.
stage-04b-procontract-login-open-after-claude-quota-reset: Supplier Post-Login Home
stage-04b-procontract-login-title-after-claude-quota-reset-2: Supplier Post-Login Home
```

The Stage 4b follow-up retry is not an auth/CDP pause. After timeout, the persistent browser was still on an authenticated ProContract activity dashboard with `Messages & clarifications (0)`, `Your response`, `Start my response`, `Opt out`, and `Indicate your intent to respond`.

The Stage 4b retry after commit `39a7935` was also not an auth/CDP pause. Claude returned `OK`, ProContract loaded `Supplier Post-Login Home`, and the skill emitted a valid result contract. It downloaded three real DOCX files for `BEST START FAMILY HUBS OUTREACH SERVICES`. The immediate pause after that retry was local extraction setup: `python3 scripts/docling-extract.py --version` returned `docling import unavailable: No module named 'docling'`, `$HOME/.docling-venv/bin/docling` was absent, and `$HOME/.cache/docling/models` was absent. That Docling setup blocker is now resolved.

The semantic `lots[].lot_number` type mismatch is also resolved. The Stage 9 preflight pause is CDP/browser setup only:

```text
stage-09-cdp-version:
curl -sfS -m 3 http://localhost:9222/json/version
curl: (7) Failed to connect to localhost port 9222 after 0 ms: Couldn't connect to server

stage-09-claude-probe:
claude -p "Return exactly OK." --dangerously-skip-permissions
OK.
```

No marketplace source was opened and no Stage 9 daemon command was run after this first Stage 9 CDP failure.

Stage 9 CDP repair evidence from 2026-05-17T14:46:35Z:

```text
stage-09-cdp-version-before-repair:
curl: (7) Failed to connect to localhost port 9222 after 0 ms: Couldn't connect to server

stage-09-cdp-version-after-repair:
Browser Chrome/148.0.7778.168
Protocol-Version 1.3
webSocketDebuggerUrl ws://localhost:9222/devtools/browser/b74db0c3-a500-47c0-bb34-5bd7005e8729

stage-09-cdp-lsof-after-repair:
Google PID 10039 listening on 127.0.0.1:9222

stage-09-agent-browser-cli-after-repair:
/opt/homebrew/bin/agent-browser
agent-browser 0.27.0

stage-09-agent-browser-aboutblank-after-repair:
opened about:blank
```

The first marketplace preflight after CDP repair found an `agent-browser` target-selection issue, not a marketplace/source issue:

```text
stage-09-amazon-open-after-cdp-repair:
Navigation failed: net::ERR_BLOCKED_BY_CLIENT

stage-09-agent-browser-tab-list-after-blocked-client:
[t1] about:blank - about:blank
-> [t2] Google Gemini - https://gemini.google.com/glic?hl=en

stage-09-agent-browser-select-t1-after-blocked-client:
selected t1

stage-09-agent-browser-example-open-after-t1-select:
opened https://example.com/
title Example Domain
```

Marketplace source checks after selecting the real page target:

```text
Amazon UK: loaded `https://www.amazon.co.uk/`; snapshot showed search UI and no captcha/robot interstitial. Optional account header showed `Hello, sign in`, which is not a blocker for the Amazon retail-benchmark skill because the skill explicitly says not to log in.
eBay UK: loaded `https://www.ebay.co.uk/`; snapshot showed authenticated state `Hi zac!` and no captcha/challenge.
Gem Imports: loaded `https://www.gemimports.co.uk/`; snapshot showed catalogue/navigation content and optional `Sign in`, with no captcha/challenge.
MX Wholesale: loaded `https://www.mxwholesale.co.uk/`; snapshot showed catalogue/navigation content and optional `Log in`, with no captcha/challenge.
Alibaba: loaded `https://www.alibaba.com/`, but snapshot showed `Sign in` and generic `Welcome to Alibaba.com`, not the previously verified authenticated state `Welcome to Alibaba.com, zac`. Because `marketplace-search/alibaba` requires a logged-in operator Chrome session, Stage 9 is paused for first-pass Alibaba auth repair before any daemon marketplace command is run.
```

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

Stage 4a fix-validation state:

```text
fix commit: 45546c2 fix procontract interest pending persistence
pinned daemon rebuild: resume-daemon-rebuild-after-procontract-fix.log PASS
fix retry log: stage-04a-procontract-interest-retry-after-pending-action-fix.log
latest completed job: job-2abb48c0309b63a8aea49b89d1461183719d8c5857cf0f06fc85d570a25c0943
latest session: session-pty-a019859b7bf30032e7ff25f08f579fd5deeeb858919004ee412537215a1264d0
latest result summary: total_evaluated=8, pending_registration=8, registered=0, already_registered=0, failed=0
interest_registration stage rows: completed=3, pending=16, skipped=89
```

The second Stage 4a retry did not report new portal-visible registrations. The one real registration remains the earlier `BEST START FAMILY HUBS OUTREACH SERVICES` action from the original blocked run.

Stage 4b timeout blocker state:

```text
workflow_jobs:
job-cf642ec3d9e68b1fe4e28e94eb9eaca37126e59f8dc7c300b091263ac0109755  sync_source  running  source  procontract-enrich  attempt_count=1

agent_session_logs:
session-pty-33fd5d63d698179f0b393ea6f8a3481a2c00f2191e3773017ee8e836aa4b7757  skills/procurement-ingestion/procontract-enrich-tender/SKILL.md  status=completed  exit_code=143  cleanup_verdict=terminated_after_timeout  terminal_output_bytes=77  run_manifest_path=NULL

stage/data rows:
no `tender_enrichment` rows
tender_document_sets=0
raw_documents_total=204
raw_docs_with_files=0
document_extractions=0
semantic_extractions=0
product_candidates=0
search_profiles=0
```

Stage 4b follow-up retry state after `413c5a7`:

```text
workflow_jobs:
job-33fb160456947771375daba10501662ef301e46b280bcc6dfcc55c1f217405bf  sync_source  failed  source  procontract-enrich  last_error="procontract enrichment skill did not produce a result contract"

agent_session_logs:
session-pty-3a29a437adfcce4b7cf3da0420cadc1b84970e978a4e9eeb31d148152a2c6afd  skills/procurement-ingestion/procontract-enrich-tender/SKILL.md  status=completed  exit_code=143  cleanup_verdict=terminated_after_timeout  terminal_output_bytes=77  run_manifest_path=NULL

internal error manifest:
error-948338ee-8651-4aa2-a900-a139b70d7b87  tender_enrichment  agent_execute  AGENT_RUN_FAILED  markdown_path=/Users/zacfarrell/Desktop/Testproject1/artifacts-e2e-machine-fork-20260512T195251Z/error-manifests/2026/05/error-948338ee-8651-4aa2-a900-a139b70d7b87.md

stage/data rows:
no `tender_enrichment` rows
tender_document_sets=0
raw_docs_with_files=0
document_extractions=0
semantic_extractions=0
product_candidates=0
search_profiles=0
```

Full post-pipeline operational subsystem checks remain pending until Stages 2-14 and UI smoke checks complete.

Stage 4b previous Docling blocker state (preserved pre-setup evidence):

```text
workflow_jobs:
job-109da01929119faeea0cebfb7e4785c12d3153038521766e980900aa5e0c3ae6          sync_source                 completed  source  procontract-enrich
job-extract-ca6b80ef3ce373a7c1778c03b5a0dcf41f32ca1cf461c2c2bd2e1e44a39f04c1  extract_document_structure  failed     notice  pc-ecd07d9f-5249-f111-813c-005056b64545  Docling runtime unavailable for wrapper /Users/zacfarrell/Desktop/Testproject1/scripts/docling-extract.py
job-extract-1cb941ce7e90472ce26cdc084fe33f844796cd69eedd365f31b34209122cf593  extract_document_structure  failed     notice  pc-ecd07d9f-5249-f111-813c-005056b64545  Docling runtime unavailable for wrapper /Users/zacfarrell/Desktop/Testproject1/scripts/docling-extract.py
job-extract-47b82860ecd39dacf4d1e54377080b2cc94f5f7b8f34db94bf111023f132600e  extract_document_structure  failed     notice  pc-ecd07d9f-5249-f111-813c-005056b64545  Docling runtime unavailable for wrapper /Users/zacfarrell/Desktop/Testproject1/scripts/docling-extract.py

stage/data rows:
tender_enrichment completed=1
tender_document_sets=1
raw_docs_with_files=3
document_extractions=0
semantic_extractions=0
product_candidates=0
search_profiles=0

docling probes:
which docling -> no result
ls $HOME/.docling-venv/bin/docling -> No such file or directory
python3 scripts/docling-extract.py --version -> docling import unavailable: No module named 'docling'
ls $HOME/.cache/docling/models -> No such file or directory
```

Stage 4b Docling setup retry state:

```text
runtime:
/Users/zacfarrell/.docling-venv/bin/python scripts/docling-extract.py --version -> 2.93.0
/Users/zacfarrell/.cache/docling/models -> exists
daemon env overrides -> not required

workflow_jobs:
extract_document_structure  completed  3
extract_document_structure  failed     3  # historical pre-setup failed rows preserved
extract_document_semantics  completed  3
extract_document_semantics  failed     1  # job-479bed51-0fea-46ee-a207-51e94c641c0b

stage/data rows:
raw_docs_with_files=3
document_extractions=3
semantic_extractions=3
product_candidates=5
search_profiles=0

current blocker:
job-479bed51-0fea-46ee-a207-51e94c641c0b failed with `missing string lots[].lot_number`
preserved result JSON has numeric `lot_number=1`
```

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

Stage 4a fix regression evidence:

```text
resume-daemon-rebuild-after-procontract-fix.log: PASS, pinned daemon rebuilt at /tmp/testproject1-target-machine-fork-20260512T195251Z/debug/daemon
regression-procontract-interest-agent-ingestion.log: PASS, e2e_ing_028_procontract_interest_replay_persistence_failure_marks_job_failed
regression-procontract-interest-pending-unit.log: PASS, persist_interest_registrations_maps_pending_registration_to_retryable_pending
regression-procontract-interest-unknown-action-unit.log: PASS, persist_interest_registrations_rejects_unknown_action_without_partial_writes
```

Stage 4b timeout fix regression evidence:

```text
resume-daemon-rebuild-after-procontract-enrich-fix.log: PASS, pinned daemon rebuilt at /tmp/testproject1-target-machine-fork-20260512T195251Z/debug/daemon
regression-procontract-enrich-missing-result-unit.log: PASS, procontract_enrich_missing_result_marks_job_failed
regression-procontract-enrich-timeout-unit.log: PASS, procontract_enrich_timeout_missing_result_marks_job_failed
regression-procontract-enrich-blocked-result-unit.log: PASS, procontract_enrich_blocked_result_is_durable
regression-procontract-enrich-persist-docs-unit.log: PASS, persist_tender_enrichments_updates_notice_and_documents
resume-claude-quota-probe-after-procontract-enrich-fix.log: BLOCKED, selected Claude CLI quota exhausted
```

Stage 4b follow-up fix verification:

```text
resume-daemon-rebuild-after-procontract-enrich-followup-fix.log: PASS, pinned daemon rebuilt at /tmp/testproject1-target-machine-fork-20260512T195251Z/debug/daemon
regression-procontract-enrich-missing-result-unit-after-followup-fix.log: PASS, procontract_enrich_missing_result_marks_job_failed
regression-procontract-enrich-blocked-result-unit-after-followup-fix.log: PASS, procontract_enrich_blocked_result_is_durable
resume-claude-probe-before-procontract-enrich-followup.log: PASS, selected Claude CLI returned OK
stage-04b-procontract-login-open-after-enrich-followup-fix.log: PASS, ProContract supplier home loaded
stage-04b-procontract-enrich-retry-after-followup-fix.log: PASS for enrichment result contract; BLOCKED for extraction because Docling runtime is unavailable
```

Stage 4b Docling setup verification:

```text
stage-04b-docling-after-runtime-probes.log: PASS, /Users/zacfarrell/.docling-venv/bin/python scripts/docling-extract.py --version -> 2.93.0; model artifacts exist at /Users/zacfarrell/.cache/docling/models
regression-procontract-enrich-blocked-result-unit-after-docling-setup.log: PASS, procontract_enrich_blocked_result_is_durable
regression-procontract-enrich-missing-result-unit-after-docling-setup.log: PASS, procontract_enrich_missing_result_marks_job_failed
regression-pdf-structural-engine-unavailable-after-docling-setup.log: PASS, test_engine_unavailable
stage-04b-procontract-enrich-retry-after-docling-setup.log: PASS for command exit; extraction drain processed 7 jobs
stage-04b-docling-after-workflow-counts-v2.log: latest structural extraction retries completed; one semantic job failed with missing string `lots[].lot_number`
stage-04b-docling-after-downstream-counts-v2.log: raw_docs_with_files=3, document_extractions=3, semantic_extractions=3, product_candidates=5, search_profiles=0
```

Stage 4b semantic lot-number fix verification:

```text
regression-pdf-semantic-numeric-lot-before-fix.log: FAIL, missing string `lots[].lot_number`
regression-pdf-semantic-numeric-lot-after-fix.log: PASS, test_tender_spec_numeric_lot_number_is_canonicalized_before_domain_writes
stage-04b-semantic-lot-number-single-job-retry-after-fix.log: PASS, Semantic extraction completed for job-479bed51-0fea-46ee-a207-51e94c641c0b using the preserved numeric-lot result JSON
stage-05-08-pipeline-procontract-after-semantic-lot-number-fix.log: PASS, quoteability_scored=5 and search_profiles=5
post-fix SQLite evidence: extract_document_semantics completed=4 failed=0; document_extractions=3; semantic_extractions=4; product_candidates=5; search_profiles=5
```

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

QA is PAUSED on a Stage 9 Alibaba authentication issue after CDP repair. Static/unit baseline blockers are resolved and verified. Real live Stage 1 source ingestion succeeded for Contracts Finder and ProContract, deterministic Stages 2-3 and 5-8 baseline processing completed, Stage 4a ProContract register-interest was fixed and validated, Stage 4b ProContract enrichment produced real downloaded documents, Docling extraction is repaired, and the semantic lot-number blocker is repaired. The pipeline is ready for Stage 9 from `search_profiles=5`; the persistent CDP listener is now reachable, but Alibaba is not authenticated in the CDP browser.

Proven on this machine:

- The checkout is on `main` at `7957146 docs: add stage 9 cdp repair handoff` plus this Stage 9 CDP/auth pause ledger edit.
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
- `/json/version` returns Chrome/148.0.7778.168 and Protocol-Version 1.3.
- The CDP browser was launched with `$HOME/.chrome-automation`.
- `agent-browser --cdp 9222 open "about:blank"`, `get title`, and `snapshot` succeed.
- `agent-browser` initially targeted Chrome's Gemini side panel during marketplace checks; `agent-browser --cdp 9222 tab t1` selected the real page target and restored normal navigation.
- Amazon UK, eBay UK, Gem Imports, MX Wholesale, and Alibaba all load through CDP after target selection. eBay shows authenticated state. Amazon/Gem/MX show optional sign-in controls but no blocking login wall or captcha. Alibaba shows `Sign in` and generic `Welcome to Alibaba.com`, so it is not authenticated for the Alibaba marketplace skill.
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
- Commit `45546c2` fixed the Stage 4a result contract semantics by making `pending_registration` an explicit retryable state and by failing persistence errors durably.
- The post-fix retry completed job `job-2abb48c0309b63a8aea49b89d1461183719d8c5857cf0f06fc85d570a25c0943` with `evaluations=8 persisted=8`.
- Stage 4a now has `interest_registration` rows: `completed=3`, `pending=16`, `skipped=89`.
- Focused Stage 4a regression tests passed.
- Stage 4b first paused for ProContract login; the operator repaired it, and the retry preflight reached `Supplier Post-Login Home`.
- Stage 4b enrichment then timed out after 1200 seconds without `result.json` or run manifest.
- The post-timeout ProContract browser snapshot was authenticated inside an activity dashboard, so this is not classified as a first-pass auth/CDP issue.
- Error manifest `prompts/machine-fork-e2e-procontract-enrich-timeout-error-manifest.md` captures the exact command, timeout output, session/job IDs, DB state, artifact paths, codepath evidence, and resume instructions.
- Commit `413c5a7` fixed the Stage 4b missing-result/timeout handling and updated the enrichment skill stall instructions.
- The pinned daemon was rebuilt after `413c5a7`, and focused Stage 4b regressions passed.
- The live Stage 4b retry was not run because `claude -p "Return exactly OK." --dangerously-skip-permissions` returned `You've hit your limit - resets 2:50am (Europe/London)`.
- Error manifest `prompts/machine-fork-e2e-claude-quota-blocker-error-manifest.md` captures the selected-agent quota blocker and resume instructions.
- The Claude quota blocker is now resolved; `resume-claude-quota-probe-after-reset` returned `OK`.
- The follow-up Stage 4b timeout error manifest is now resolved by the current skill fix: the live retry produced `result.json`, the daemon saw a valid result contract, and job `job-109da01929119faeea0cebfb7e4785c12d3153038521766e980900aa5e0c3ae6` completed.
- Stage 4b persisted `tender_enrichment completed=1`, `tender_document_sets=1`, and `raw_docs_with_files=3`.
- The persisted files are real ProContract DOCX downloads under `artifacts-e2e-machine-fork-20260512T195251Z/downloads/procontract/786663/`.
- The previous three structural extraction jobs failed with `Docling runtime unavailable for wrapper /Users/zacfarrell/Desktop/Testproject1/scripts/docling-extract.py`; those failed rows are preserved as historical evidence.
- Docling 2.93.0 is now available through `/Users/zacfarrell/.docling-venv/bin/python`, and `/Users/zacfarrell/.cache/docling/models` exists after `docling-tools models download`.
- No daemon env overrides are required because `default_docling_python_bin()` now selects the default venv and `default_docling_artifacts_path()` resolves to the populated model directory.
- Latest structural extraction retries completed and created `document_extractions=3`; downstream processing created `document_semantic_extractions=3` and `product_candidates=5`.
- Semantic job `job-479bed51-0fea-46ee-a207-51e94c641c0b` originally failed with `missing string lots[].lot_number` because its preserved `workdir/result.json` contains numeric `lot_number=1` for the tender-specification lot.
- The semantic lot-number fix requeued only that failed job, replayed the preserved result JSON, and the job is now `completed`.
- Re-running `--pipeline procontract` after the semantic fix scored 5 candidates and produced `search_profiles=5`.
- Error manifest `prompts/machine-fork-e2e-docling-runtime-unavailable-error-manifest.md` is resolved by machine setup.
- Error manifest `prompts/machine-fork-e2e-semantic-lot-number-type-error-manifest.md` is resolved by the daemon semantic lot-number canonicalization fix.
- Stage 9 did not start before CDP repair because `curl -sfS -m 3 http://localhost:9222/json/version` returned connection refused. After repair, no Stage 9 daemon command was run because Alibaba authentication is missing in the persistent CDP browser.

Current next point:

1. Source `/Users/zacfarrell/Desktop/Testproject1/state-e2e-machine-fork-20260512T195251Z/e2e-env.sh`.
2. Keep the existing preserved roots; do not rerun Stage 4a portal registration.
3. Keep the repaired Chrome CDP profile listening on `localhost:9222`.
4. Operator logs in or clears the Alibaba challenge in the Chrome window attached to `localhost:9222`.
5. Re-run `run_logged stage-09-cdp-version-after-alibaba-auth-repair curl -sfS -m 3 http://localhost:9222/json/version`.
6. Run `agent-browser --cdp 9222 tab t1` if `agent-browser tab list` shows the Gemini side panel selected, then re-open `https://www.alibaba.com/` and verify authenticated Alibaba state.
7. If CDP/login checks pass, run Stage 9 supplier price comparison / marketplace search from the 5 generated search profiles for process `proc-pc-ecd07d9f-5249-f111-813c-005056b64545`.
8. After Stage 9 produces supplier quotes, rerun `--pipeline procontract` for Stage 10 gross-margin assessment.
