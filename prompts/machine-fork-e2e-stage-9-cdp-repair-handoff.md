# Machine Fork E2E Stage 9 CDP Repair Handoff

ULTRATHINK and be meticulous.

## MISSION

Repair the current machine-fork E2E pause at Stage 9, where supplier marketplace validation cannot start because the persistent Chrome CDP endpoint on `localhost:9222` is not reachable. This is a machine/browser setup blocker at the current evidence level, not a product-code bug yet.

After CDP is repaired, resume the existing E2E run from Stage 9 against the preserved roots. Do not restart the E2E run from scratch, do not create a new state root, and do not delete or clean any existing artifacts.

## Current Reproduction Status

Verified from `/Users/zacfarrell/Desktop/Testproject1` on 2026-05-17.

Current git context:

```text
d2e1359 docs: record stage 9 cdp pause
```

Preserved E2E roots:

```text
REPO_ROOT=/Users/zacfarrell/Desktop/Testproject1
E2E_STATE_ROOT=/Users/zacfarrell/Desktop/Testproject1/state-e2e-machine-fork-20260512T195251Z
E2E_RUNTIME_DIR=/Users/zacfarrell/Desktop/Testproject1/runtime-e2e-machine-fork-20260512T195251Z
E2E_ARTIFACT_ROOT=/Users/zacfarrell/Desktop/Testproject1/artifacts-e2e-machine-fork-20260512T195251Z
E2E_LOG_ROOT=/Users/zacfarrell/Desktop/Testproject1/artifacts-e2e-machine-fork-20260512T195251Z/qa-logs
E2E_DAEMON=/tmp/testproject1-target-machine-fork-20260512T195251Z/debug/daemon
AGENT_BROWSER_CDP_PORT=9222
```

Reproduction logs created during this handoff:

```text
artifacts-e2e-machine-fork-20260512T195251Z/qa-logs/stage-09-cdp-version-recreate-20260517T141148Z.log
artifacts-e2e-machine-fork-20260512T195251Z/qa-logs/stage-09-agent-browser-title-recreate-20260517T141148Z.log
artifacts-e2e-machine-fork-20260512T195251Z/qa-logs/stage-09-cdp-lsof-recreate-20260517T141148Z.log
artifacts-e2e-machine-fork-20260512T195251Z/qa-logs/stage-09-ready-state-recreate-20260517T141148Z.log
artifacts-e2e-machine-fork-20260512T195251Z/qa-logs/stage-09-agent-browser-cli-recreate-20260517T141157Z.log
artifacts-e2e-machine-fork-20260512T195251Z/qa-logs/stage-09-git-head-recreate-20260517T141157Z.log
artifacts-e2e-machine-fork-20260512T195251Z/qa-logs/stage-09-schema-epoch-recreate-20260517T141157Z.log
```

Exact reproduced failure:

```bash
source state-e2e-machine-fork-20260512T195251Z/e2e-env.sh
run_logged stage-09-cdp-version-recreate-20260517T141148Z \
  curl -sfS -m 3 http://localhost:9222/json/version
```

Output:

```text
$ curl -sfS -m 3 http://localhost:9222/json/version
curl: (7) Failed to connect to localhost port 9222 after 0 ms: Couldn't connect to server
```

`agent-browser` is installed, but it cannot discover CDP because no listener is present:

```bash
run_logged stage-09-agent-browser-title-recreate-20260517T141148Z \
  agent-browser --cdp 9222 get title
```

Output:

```text
$ agent-browser --cdp 9222 get title
✗ Auto-launch failed: All CDP discovery methods failed for 127.0.0.1:9222: /json/version: Failed to connect to CDP at 127.0.0.1:9222: error sending request for url (http://127.0.0.1:9222/json/version); /json/list: Failed to connect to /json/list at 127.0.0.1:9222: error sending request for url (http://127.0.0.1:9222/json/list); WebSocket: WebSocket connect failed at ws://127.0.0.1:9222/devtools/browser: IO error: Connection refused (os error 61)
```

No process is listening on port 9222:

```bash
run_logged stage-09-cdp-lsof-recreate-20260517T141148Z \
  lsof -nP -iTCP:9222 -sTCP:LISTEN
```

Output:

```text
$ lsof -nP -iTCP:9222 -sTCP:LISTEN
```

The CLI itself is present:

```text
$ command -v agent-browser && agent-browser --version
/opt/homebrew/bin/agent-browser
agent-browser 0.27.0
```

The preserved E2E state is ready for Stage 9 after the prior semantic lot-number fix:

```text
PRAGMA user_version = 9
search_profiles = 5
product_candidates = 5
extract_document_semantics completed = 4
extract_document_structure completed = 3
extract_document_structure failed = 3 historical pre-Docling setup rows
```

## Why This Is The Current Blocker

Stage 9 marketplace skills all require the operator persistent CDP browser. `crates/daemon/src/main.rs` dispatches:

```text
--ingest marketplace-coverage -> run_marketplace_coverage_live_loop
--ingest amazon-uk           -> run_marketplace_search_live_loop
--ingest ebay-uk             -> run_marketplace_search_live_loop
--ingest gem-imports         -> run_marketplace_search_live_loop
--ingest mx-wholesale        -> run_marketplace_search_live_loop
--ingest alibaba-search      -> run_marketplace_search_live_loop using skills/marketplace-search/alibaba
```

`run_marketplace_coverage_live_loop` runs sources in this order:

```text
amazon-uk, ebay-uk, gem-imports, mx-wholesale, alibaba-search
```

The prompt renderer injects this contract into CDP-required skills:

```text
Use `agent-browser --cdp ${AGENT_BROWSER_CDP_PORT}` for every browser command.
Do not close the operator browser. If CDP is unreachable, write a blocked `result.json`; do not launch a browser.
```

The current pause occurred before any marketplace source command was run. No Stage 9 daemon command has been started after the CDP failure.

## Read First

Read these files before changing anything:

- `AGENTS.md`
- `docs/pipeline/machine-fork-l0-l3-stages-1-14-e2e-validation-manifest.md`
- `prompts/machine-fork-l0-l3-stages-1-14-e2e-handoff.md`
- `prompts/machine-fork-e2e-agent-browser-cdp-fix-handoff.md`
- `crates/daemon/src/main.rs` around `run_marketplace_search_live_loop` and `run_marketplace_coverage_live_loop`
- `crates/daemon/src/prompt/renderer.rs`
- `skills/marketplace-search/amazon-uk/SKILL.md`
- `skills/marketplace-search/ebay-uk/SKILL.md`
- `skills/marketplace-search/gem-imports/SKILL.md`
- `skills/marketplace-search/mx-wholesale/SKILL.md`
- `skills/marketplace-search/alibaba/SKILL.md`

Checkpoint questions:

1. Which exact state root, runtime dir, artifact root, and daemon binary must be reused?
2. Which Stage 9 source runs first under `marketplace-coverage`?
3. Which skills require CDP, and what must they do if CDP is unreachable?
4. What distinguishes first-pass CDP/auth repair from a product/tooling blocker in the machine-fork QA protocol?

## Constraints

- Do not delete files, logs, state roots, runtime dirs, artifact roots, downloaded documents, or screenshots.
- Do not run `agent-browser close`, `agent-browser close --all`, close Chrome, or reset the operator's browser profile.
- Do not run marketplace ingestion until CDP preflight and source login checks pass.
- Do not ask for credentials in chat. If a site requires login, MFA, or captcha, ask the operator to complete it in the visible Chrome profile attached to port 9222.
- Do not use npm/yarn/pnpm. This task should not need JS dependency changes.
- Do not patch daemon or skill code unless CDP is confirmed healthy and a product failure appears after retry.
- Do not perform purchases, checkouts, supplier messages, register-interest actions, or any other external write-side action.

## Validate/Fix Loop

Use this loop. After every fix attempt, rerun the validation command at the end of that loop. If the validation still fails, stay in the same loop until you have either fixed it or have enough evidence for a new error manifest.

### Loop 0 - Restore The Existing E2E Context

```bash
cd /Users/zacfarrell/Desktop/Testproject1
source state-e2e-machine-fork-20260512T195251Z/e2e-env.sh
test -x "$E2E_DAEMON"
sqlite3 "$E2E_STATE_ROOT/state.db" 'PRAGMA user_version;'
git status --short --branch
git log --oneline -1
```

Expected:

- `PRAGMA user_version` prints `9`.
- The daemon binary exists.
- Worktree may contain unrelated changes from other agents; do not revert or stash them.

### Loop 1 - Restore A CDP Listener On Port 9222

First validate the failure:

```bash
run_logged stage-09-cdp-version-before-repair \
  curl -sfS -m 3 http://localhost:9222/json/version
run_logged stage-09-cdp-lsof-before-repair \
  lsof -nP -iTCP:9222 -sTCP:LISTEN
```

If no listener is present, launch the canonical automation Chrome profile:

```bash
open -na "Google Chrome" --args \
  "--remote-debugging-port=9222" \
  "--remote-allow-origins=*" \
  "--user-data-dir=$HOME/.chrome-automation" \
  "--profile-directory=Default" \
  "--no-first-run" \
  "--no-default-browser-check"
```

Do not close any existing Chrome window. If Chrome refuses to expose remote debugging, inspect process args and profile state without deleting anything:

```bash
ps -axo pid,command | rg 'Google Chrome|remote-debugging-port=9222|chrome-automation' | rg -v 'rg '
lsof -nP -iTCP:9222 -sTCP:LISTEN
```

Validation for Loop 1:

```bash
run_logged stage-09-cdp-version-after-repair \
  curl -sfS -m 3 http://localhost:9222/json/version
run_logged stage-09-cdp-lsof-after-repair \
  lsof -nP -iTCP:9222 -sTCP:LISTEN
```

Expected: `/json/version` returns Chrome metadata and `lsof` shows a listener on `127.0.0.1:9222` or equivalent localhost binding.

### Loop 2 - Validate agent-browser Controls The CDP Session

```bash
run_logged stage-09-agent-browser-cli-after-repair \
  zsh -lc 'command -v agent-browser && agent-browser --version'
run_logged stage-09-agent-browser-aboutblank-after-repair \
  agent-browser --cdp 9222 open "about:blank"
run_logged stage-09-agent-browser-title-after-repair \
  agent-browser --cdp 9222 get title
run_logged stage-09-agent-browser-snapshot-after-repair \
  agent-browser --cdp 9222 snapshot
```

Expected:

- CLI path remains `/opt/homebrew/bin/agent-browser` or another real executable path.
- `agent-browser --version` returns a version.
- `open`, `get title`, and `snapshot` complete without CDP discovery failure.

If `curl` passes but `agent-browser --cdp 9222` fails, capture:

```bash
agent-browser --help | sed -n '1,100p'
agent-browser --cdp 9222 snapshot --debug
```

Then decide whether this is an `agent-browser` CLI compatibility issue. Do not proceed to Stage 9 until this loop passes.

### Loop 3 - Validate Marketplace Source Browser State

Run all checks through the same CDP session the daemon skills will use:

```bash
run_logged stage-09-amazon-open-after-cdp-repair \
  agent-browser --cdp 9222 open "https://www.amazon.co.uk/"
run_logged stage-09-amazon-snapshot-after-cdp-repair \
  agent-browser --cdp 9222 snapshot

run_logged stage-09-ebay-open-after-cdp-repair \
  agent-browser --cdp 9222 open "https://www.ebay.co.uk/"
run_logged stage-09-ebay-snapshot-after-cdp-repair \
  agent-browser --cdp 9222 snapshot

run_logged stage-09-gem-imports-open-after-cdp-repair \
  agent-browser --cdp 9222 open "https://www.gemimports.co.uk/"
run_logged stage-09-gem-imports-snapshot-after-cdp-repair \
  agent-browser --cdp 9222 snapshot

run_logged stage-09-mx-wholesale-open-after-cdp-repair \
  agent-browser --cdp 9222 open "https://www.mxwholesale.co.uk/"
run_logged stage-09-mx-wholesale-snapshot-after-cdp-repair \
  agent-browser --cdp 9222 snapshot

run_logged stage-09-alibaba-open-after-cdp-repair \
  agent-browser --cdp 9222 open "https://www.alibaba.com/"
run_logged stage-09-alibaba-snapshot-after-cdp-repair \
  agent-browser --cdp 9222 snapshot
```

If any page shows login, MFA, expired session, account challenge, or captcha for the first time, pause with exactly this message:

```text
QA is paused because the persistent CDP browser is not authenticated for <source>. Please log in or clear the challenge in the Chrome window attached to localhost:9222, then tell me to continue. I will retry the same step.
```

After the operator says to continue, rerun Loop 1, Loop 2, then retry the exact failed source check. If the same source auth/captcha issue repeats after operator confirmation, create a new QA error manifest and pause.

### Loop 4 - Retry The Exact Stage 9 Command

Only run this after Loops 1-3 pass:

```bash
run_logged stage-09-claude-probe-after-cdp-repair \
  claude -p "Return exactly OK." --dangerously-skip-permissions

run_logged stage-09-marketplace-coverage-after-cdp-repair \
  env AGENT_BROWSER_CDP_PORT=9222 "$E2E_DAEMON" \
    --state-root "$E2E_STATE_ROOT" \
    --runtime-dir "$E2E_RUNTIME_DIR" \
    --artifact-root "$E2E_ARTIFACT_ROOT" \
    --ingest marketplace-coverage
```

If `marketplace-coverage` stops at one source and the failure is source-specific, isolate it:

```bash
for s in amazon-uk ebay-uk gem-imports mx-wholesale alibaba-search; do
  run_logged "stage-09-${s}-after-cdp-repair" \
    env AGENT_BROWSER_CDP_PORT=9222 "$E2E_DAEMON" \
      --state-root "$E2E_STATE_ROOT" \
      --runtime-dir "$E2E_RUNTIME_DIR" \
      --artifact-root "$E2E_ARTIFACT_ROOT" \
      --ingest "$s"
done
```

Do not turn a structured no-result contract into a failure. A source can produce no marketplace results and still pass if it writes durable, valid `search_attempts` rows with honest failure/no-result metadata.

Validation for Loop 4:

```bash
run_logged stage-09-marketplace-db-validation-after-cdp-repair \
  sqlite3 -header "$E2E_STATE_ROOT/state.db" "
SELECT source, COUNT(*) AS attempts, SUM(result_count) AS results, SUM(shortlisted_count) AS shortlisted,
       GROUP_CONCAT(DISTINCT search_failure_mode) AS failure_modes
FROM search_attempts GROUP BY source ORDER BY source;

SELECT source_name, price_signal_role, COUNT(*) AS quotes,
       MIN(unit_price_gbp_pence) AS min_gbp_pence,
       MAX(unit_price_gbp_pence) AS max_gbp_pence
FROM supplier_quotes GROUP BY source_name, price_signal_role ORDER BY source_name;

SELECT required_role, coverage_state, latest_failure_mode, COUNT(*)
FROM candidate_quote_coverage GROUP BY required_role, coverage_state, latest_failure_mode
ORDER BY required_role, coverage_state;

SELECT kind, status, COUNT(*)
FROM workflow_jobs
WHERE kind='marketplace_search'
GROUP BY kind, status
ORDER BY kind, status;
"
```

Expected:

- Durable `search_attempts` rows exist for attempted Stage 9 sources.
- `supplier_quotes` rows exist when sources return usable prices.
- `candidate_quote_coverage` honestly records coverage or failure modes for UK-side and China/import roles.
- `workflow_jobs` for `marketplace_search` are completed or have a documented structured blocker.

### Loop 5 - Continue To Stage 10 Only After Stage 9 Is Durable

After Stage 9 writes durable attempts/quotes, run the deterministic pipeline continuation:

```bash
run_logged stage-10-pipeline-procontract-after-stage-09 \
  "$E2E_DAEMON" \
    --state-root "$E2E_STATE_ROOT" \
    --runtime-dir "$E2E_RUNTIME_DIR" \
    --artifact-root "$E2E_ARTIFACT_ROOT" \
    --pipeline procontract
```

Validate:

```bash
run_logged stage-10-pricing-assessment-validation-after-stage-09 \
  sqlite3 -header "$E2E_STATE_ROOT/state.db" "
SELECT commercial_basis_kind, normal_quote_coverage, COUNT(*)
FROM candidate_pricing_assessments
GROUP BY commercial_basis_kind, normal_quote_coverage
ORDER BY commercial_basis_kind;

SELECT product_candidate_id, selected_quote_id, gross_margin_bps, commercial_basis_kind,
       uk_quote_id, china_import_quote_id, benchmark_quote_id, missing_required_roles_json
FROM candidate_pricing_assessments
ORDER BY updated_at DESC LIMIT 20;
"
```

If this passes, update `docs/pipeline/machine-fork-l0-l3-stages-1-14-e2e-validation-manifest.md` and continue the original QA run at Stage 11 landed cost.

## Failure Branches

Create a new error manifest and pause if any of these happen after CDP/browser setup is confirmed healthy:

- `marketplace-coverage` or a source-specific Stage 9 run fails to produce `result.json`.
- The skill returns structurally invalid marketplace result JSON.
- The daemon accepts a completed Stage 9 command but expected `search_attempts` rows are missing.
- SQLite integrity or foreign key checks fail.
- A source repeatedly shows captcha/auth after the operator says the browser session is repaired.
- `agent-browser --cdp 9222` fails while `curl /json/version` succeeds, suggesting tool/CDP incompatibility.

Suggested manifest path for a repeated CDP/tooling failure:

```text
prompts/machine-fork-e2e-stage-9-cdp-repeat-error-manifest.md
```

Suggested manifest path for a marketplace product/source failure:

```text
prompts/machine-fork-e2e-stage-9-<source-or-short-failure>-error-manifest.md
```

Each manifest must include exact commands, log paths, terminal output, the E2E roots, DB queries, source snapshots, session IDs, job IDs, and a smallest retry command.

## Acceptance Criteria

- `curl -sfS -m 3 http://localhost:9222/json/version` returns Chrome metadata.
- `lsof -nP -iTCP:9222 -sTCP:LISTEN` shows a listener.
- `agent-browser --cdp 9222 open "about:blank"` succeeds.
- `agent-browser --cdp 9222 get title` succeeds.
- `agent-browser --cdp 9222 snapshot` succeeds.
- Amazon UK, eBay UK, Gem Imports, MX Wholesale, and Alibaba load through the same CDP browser without first-pass auth/captcha blockers.
- `--ingest marketplace-coverage` runs, or source-specific isolation explains exactly which source is blocked.
- Stage 9 writes durable `search_attempts` rows for attempted sources.
- Quotes are persisted when sources return usable prices.
- Coverage rows honestly record UK-side and China/import role coverage.
- Stage 10 deterministic continuation writes `candidate_pricing_assessments` after Stage 9.
- The validation manifest is updated with the repair method, log paths, Stage 9 DB evidence, and next resume point.

## Final Notes For The Receiving Agent

This handoff is intentionally scoped to Stage 9 CDP repair and resume. Prior blockers in the machine-fork ledger are resolved: schema epoch mismatch, missing initial `agent-browser`, desktop native-shell flake, Claude auth/quota, ProContract pending-registration persistence, ProContract enrichment timeout, Docling runtime setup, and semantic lot-number canonicalization.

The current resume point is:

```text
Stage 9 supplier price comparison / marketplace coverage, using the existing E2E roots and the 5 generated search profiles for process proc-pc-ecd07d9f-5249-f111-813c-005056b64545.
```

Do not restart earlier stages unless the fix changes migrations or invalidates the preserved E2E state.
