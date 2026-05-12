# Machine Fork E2E QA - Layers 0-3 + Stages 1-14

ULTRATHINK and be meticulous.

## MISSION

Validate that this newly duplicated machine can run the procurement intelligence platform end to end for everything built through Layer 3 and the post-Layer-3 QA stages 1-14: daemon startup, source ingestion, contract retrieval, document extraction, supplier price comparison, logistics/landed-cost comparison, manifest generation, operator decision capture, desktop control panel, and the localhost dashboard surfaces. The run is deliberately interruption-friendly: if CDP/browser login is missing, pause and ask the operator to log in, then retry; if a real product bug appears, create a forensic error manifest, stop, notify the operator, and resume only after the operator says the bug has been fixed. Do not validate Layer 4 sync/RLS/shared-visibility semantics, but do include a bounded local smoke of `apps/web` if the operator means the localhost website dashboard in addition to the Layer 2 desktop dev dashboard.

The output of this handoff is a validation manifest at:

```text
docs/pipeline/machine-fork-l0-l3-stages-1-14-e2e-validation-manifest.md
```

Keep this manifest updated as a resume ledger after every completed phase and before every pause.

Concrete deliverables:

1. `docs/pipeline/machine-fork-l0-l3-stages-1-14-e2e-validation-manifest.md` with a command-by-command ledger, evidence, pause history, and final pass/fail/deferred status.
2. Screenshots for desktop, desktop Vite, and web smoke checks under `docs/pipeline/machine-fork-l0-l3-stages-1-14-e2e-screenshots/`.
3. Error manifests under `prompts/machine-fork-e2e-<short-failure>-error-manifest.md` for every real product blocker.
4. A final session handoff that says exactly where to resume if the run is paused, or exactly what was proven on this machine if it completes.

---

## Non-Negotiable Operating Protocol

1. Read `AGENTS.md` first and obey it.
2. Do not delete files, state roots, artifact roots, downloaded documents, or temporary forensic data. If cleanup seems needed, leave it and document it.
3. Work on `main`.
4. Use Bun for TypeScript layers and Cargo for Rust layers. Do not use npm, yarn, or pnpm.
5. For browser testing, invoke `/agent-browser` first if the session has that skill available. Use the Codex Browser/in-app browser for localhost visual checks and the `agent-browser --cdp` CLI for daemon-driven persistent-CDP skills.
6. Never run `agent-browser close`, `agent-browser close --all`, close Chrome, or replace the operator's CDP profile. Some older prompts in this repository mention closing agent-browser sessions; those instructions are stale for this machine-fork QA run.
7. Do not fix product bugs during this QA run. This run is a validator, not a repair session. The only writing expected is the validation manifest, screenshots/evidence, and error manifests. If a bug blocks progress, create the error manifest and pause.
8. Do not create an error manifest for the first occurrence of an authentication/CDP/session issue. Pause, prompt the operator to log in or repair the browser session, then retry exactly the failed step. If the same issue repeats after operator confirmation, create an error manifest because it is now likely source drift or code/tooling breakage.
9. Use subagents freely for read-only research or focused log/database analysis. Do not let subagents edit code. If a subagent finds a blocker, fold its evidence into the validation manifest or an error manifest.

---

## Pause Rules

### Pause For Operator Login / CDP Repair, Then Retry

Pause and ask the operator to repair the persistent browser session when any stage reports or visibly shows:

- `CDP_CONNECT_FAILED`
- `AGENT_AUTH_CHALLENGE`
- `PROCONTRACT_SESSION_EXPIRED`
- `SOURCE_CAPTCHA_BLOCKED`
- `AUTH_STATE_STALE`
- `SOURCE_AUTH_STATE_INVALID`
- SeaRates `cdp_state` of `captcha_blocked`, `session_expired`, or `unreachable`
- Alibaba, 1688, eBay, ProContract, or SeaRates showing a sign-in, expired session, account challenge, or captcha page in the persistent CDP browser

Prompt text should be direct:

```text
QA is paused because the persistent CDP browser is not authenticated for <source>. Please log in or clear the challenge in the Chrome window attached to localhost:9222, then tell me to continue. I will retry the same step.
```

Do not ask for credentials in chat. Do not type credentials for the operator. Do not automate captcha solving. The operator completes the login/challenge directly in the already-open Chrome profile.

After the operator says to continue:

1. Re-run the CDP preflight.
2. Re-open the affected site through `agent-browser --cdp 9222`.
3. Retry the exact failed daemon command.
4. Append before/after evidence to the validation manifest.

### Create Error Manifest And Pause For Real Bugs

For anything other than first-pass login/CDP/session repair, create a QA-grade error manifest and stop:

- build/check/test failure
- daemon panic or hang
- invalid result contract
- missing `result.json` or missing run manifest
- schema epoch mismatch
- migration failure
- database FK/invariant failure
- wrong persisted rows
- a stage completes but its expected downstream rows are missing
- desktop or web UI crash
- Playwright failure with valid environment
- Supabase query failure when valid env is present
- source skill returns structurally invalid data after CDP/auth is confirmed healthy

Use the error-manifest skill instructions at:

```text
/Users/zacfarrell/.codex/skills/error-manifest/SKILL.md
```

Name manifests like:

```text
prompts/machine-fork-e2e-<short-failure>-error-manifest.md
```

The error manifest must include exact commands, exact output snippets, the state root, runtime dir, artifact root, relevant session IDs, database queries, and reproduction steps. Then tell the operator:

```text
QA is paused. I created <path>. Please hand that error manifest to a fixer. After it is fixed, tell me to continue and I will resume from <phase/stage>.
```

---

## Scope

### In Scope

- Layer 0: contracts schema, intelligence package, gold/evaluation/calibration fixtures, skill schema discipline.
- Layer 1: daemon binary, SQLite bootstrap, PTY/contract skill execution, cron/job queue, run manifests, error manifests, IPC readiness.
- Layer 2: desktop Tauri app, Vite localhost dev dashboard at `http://127.0.0.1:5173`, contract list/detail, sessions, errors, manifest inbox, operator decisions.
- Layer 3: operational hardening, source registry, resource arbiter, validation dry-run surfaces, relation resolution, Stage 1-14 pipeline behavior.
- Post-L3 QA stages:
  - Stage 1 source ingestion: Contracts Finder, ProContract, FTS if fixture or live path is available.
  - Stages 2-8 deterministic normalization, relating, qualification, document acquisition/extraction, product extraction, quoteability.
  - Stage 9 marketplace price comparison: Amazon UK, eBay UK, Gem Imports, MX Wholesale, Alibaba.
  - Stage 10 gross margin/commercial basis.
  - Stage 11 logistics landed cost: HMRC, shipment sizing, SeaRates, FX.
  - Stage 12 net margin.
  - Stage 13 manifest generation.
  - Stage 14 operator decision/clarification through CLI and desktop UI.
- Explicitly requested local website smoke: `apps/web` at `http://127.0.0.1:3000`, but only as a local dashboard fork-health check, not as Layer 4 sync/RLS acceptance.

### Out Of Scope

- Layer 4 sync correctness, Supabase RLS, two-user shared visibility, command ledger behavior, cross-authority conflict resolution.
- Layer 5 decision-grade commercial intelligence beyond the commercial-basis state machine already present in Stages 8-13.
- Layer 6 request pipeline automation.
- Stage 15 bid workflow.
- Any real bid, purchase, booking, supplier message, checkout, or commitment.

The only allowed external mutation is ProContract register-interest, capped by the daemon and used only if the run reaches that stage with an authenticated browser session.

---

## Authoritative Reading

Read these before executing:

- `AGENTS.md`
- `docs/strategy/layer-strategy.md`
- `docs/strategy/README.md`
- `docs/strategy/layers/layer-0-gold-set-evaluation.md`
- `docs/strategy/layers/layer-1-thin-local-runner.md`
- `docs/strategy/layers/layer-2-single-operator-desktop.md`
- `docs/strategy/layers/layer-3-operational-hardening.md`
- `docs/session-log-2026-04-01-to-04-11-qa-validation.md`
- `docs/session-log-2026-04-17.md`
- `docs/pipeline/full-pipeline-e2e-validation-manifest.md`
- `docs/pipeline/full-pipeline-e2e-revalidation-validation-manifest.md`
- `docs/pipeline/full-pipeline-e2e-commercial-basis-followup-manifest.md`
- `docs/pipeline/persistent-cdp-agent-browser-standardization-validation-manifest.md`
- `docs/pipeline/stage-11-cdp-live-demos-validation-manifest.md`
- `docs/pipeline/stage-14-phase-2-live-operational-validation-manifest.md`

Use these as reference, not as commands to copy blindly. Current code and skill files win over stale prompt text.

---

## Reading Pass And Understanding Checkpoints

Do this reading pass before running the pipeline. The goal is to avoid treating old validation manifests as gospel when the current code has moved.

1. Layer strategy and scope:
   - Read `docs/strategy/layer-strategy.md` for the authority matrix, Layer 0-3 boundaries, and the explicit deferral of Layer 4 shared visibility.
   - Read `docs/strategy/layers/layer-0-gold-set-evaluation.md` through `docs/strategy/layers/layer-3-operational-hardening.md` for the layer-specific acceptance model.
   - Checkpoint questions:
     - Which tables are local-only before Layer 4, especially for operator decisions and workflow jobs?
     - Which behavior is allowed in this run even though Layer 4 schema/migrations may exist in the codebase?

2. Daemon and stage dispatch:
   - Read `crates/daemon/src/main.rs`, especially the `Cli` struct, the `if let Some(source_id) = &cli.ingest` dispatch, `run_deterministic_pipeline`, Stage 9 marketplace live loops, Stage 11 landed-cost loop, Stage 12 net-gate loop, Stage 13 manifest loop, and Stage 14 decision/clarification handlers.
   - Read `crates/daemon/src/db/sqlite.rs` for migration and `PRAGMA user_version` behavior.
   - Checkpoint questions:
     - Which stage commands write authoritative SQLite rows, and which commands only read or enqueue?
     - What exact source IDs does the daemon accept for `--ingest`, and which ones fan out into multiple skills?

3. Browser/CDP skill contracts:
   - Read `crates/daemon/src/prompt/renderer.rs` for the daemon-injected Persistent CDP Browser Contract.
   - Read the in-scope `SKILL.md` files under `skills/procurement-ingestion/`, `skills/marketplace-search/`, and `skills/landed-cost/`.
   - Checkpoint questions:
     - Which skills require the operator's persistent CDP browser, and how do they report `CDP_CONNECT_FAILED`, captcha, or expired sessions?
     - Which failures should pause for operator login versus produce an error manifest?

4. Desktop and localhost surfaces:
   - Read `apps/desktop/src-tauri/src/main.rs`, `apps/desktop/src-tauri/src/query_service.rs`, `apps/desktop/src-tauri/src/daemon_bridge.rs`, and the desktop pages listed in the Desktop App Validation section.
   - Read `apps/web/package.json`, `apps/web/playwright.config.ts`, and the `apps/web` routes only for a bounded local dashboard smoke.
   - Checkpoint questions:
     - What does "full control" mean in the desktop bootstrap state?
     - How can `apps/web` fail in a way that is only a Layer 4/local-dashboard smoke issue, not an L0-L3 pipeline failure?

5. Prior QA evidence:
   - Read the pipeline manifests listed in Authoritative Reading for known caveats and known-good command shapes.
   - Treat older prompts as historical evidence. If an older prompt contradicts this handoff or current code, current code and this handoff win.
   - Checkpoint questions:
     - Which previous failures were auth/CDP/session setup problems rather than product bugs?
     - Which previous failures became permanent acceptance criteria for stages 9-14?

---

## Critical Current-State Checks

Run these before any live pipeline work:

```bash
git status --short --branch
git log --oneline -1

rg -n "pub const DEFAULT_SCHEMA_EPOCH|EXPECTED_SCHEMA_EPOCH|DEFAULT_SCHEMA_EPOCH" \
  crates/daemon/src/db/sqlite.rs \
  apps/desktop/src-tauri/src/main.rs \
  apps/desktop/src-tauri/src/query_service.rs \
  apps/desktop/src-tauri/src/daemon_bridge.rs \
  apps/desktop/src-tauri/tests -g '*.rs'
```

At the time this handoff was written, daemon schema epoch was `9` while desktop/Tauri constants were still `8`. If that mismatch remains, create an error manifest and pause before doing UI validation. Do not paper over it with `PRAGMA user_version` edits. The daemon may include the `0012_l4_sync_state.sql` migration because the current schema epoch includes it; that does not make Layer 4 sync validation in scope.

Also verify the current CLI dispatchers:

```bash
rg -n '"fts" =>|"contracts-finder" =>|"procontract" =>|"procontract-interest" =>|"procontract-enrich" =>|"marketplace-coverage" =>|"landed-cost" =>|"net-gate" =>|"manifest" =>' crates/daemon/src/main.rs
rg -n "run_deterministic_pipeline|run_marketplace_search_live_loop|run_marketplace_coverage_live_loop|run_landed_cost_live_loop|run_net_margin_gate_live_loop|run_manifest_generation_live_loop" crates/daemon/src/main.rs
```

---

## Current Execution Contract Reference

The current daemon CLI contract in `crates/daemon/src/main.rs` has these key flags:

```text
--state-root <PATH>           SQLite state root; default is state
--runtime-dir <PATH>          lock/socket/descriptor directory; default is runtime
--artifact-root <PATH>        captured bytes, run manifests, opportunity manifests; default is artifacts
--dry-run                     bootstrap, write readiness artifacts, exit
--serve                       long-running daemon mode
--serve-once                  accept one IPC handshake
--ingest <SOURCE_ID>          run a live-loop stage/source
--pipeline <SOURCE_ID>        run deterministic stages for already-ingested data
--run-next-job                claim and execute one pending workflow job
--capture-decision <PROCESS>  Stage 14 operator decision capture
--disposition <approve|reject|watch>
--idempotency-key <KEY>
--reasoning <TEXT>
--capture-clarification <PROCESS>
--candidate-id <CANDIDATE>
--reason-code <CODE>
--payload <JSON>
--note <TEXT>
--withdraw-clarification <PROCESS>
--withdraw-decision <IDEMPOTENCY_KEY>
--list-decisions
```

Accepted `--ingest` source IDs visible in current dispatch:

```text
fts
contracts-finder
procontract
procontract-interest
procontract-enrich
marketplace-coverage
gem-imports
mx-wholesale
amazon-uk
ebay-uk
alibaba-search
landed-cost
net-gate
manifest
```

Skill/CDP map to keep in mind:

| Pipeline Area | Daemon Source ID | Skill ID | CDP Policy |
|---|---|---|---|
| Contracts Finder ingestion | `contracts-finder` | `procurement-ingestion/contracts-finder` | `requires_cdp: true`, port 9222 |
| FTS ingestion | `fts` | `procurement-ingestion/fts` | `requires_cdp: true`, port 9222 |
| ProContract discover | `procontract` | `procurement-ingestion/procontract-discover` | `requires_cdp: true`, port 9222 |
| ProContract interest | `procontract-interest` | `procurement-ingestion/procontract-register-interest` | `requires_cdp: true`, port 9222 |
| ProContract enrich | `procontract-enrich` | `procurement-ingestion/procontract-enrich-tender` | `requires_cdp: true`, port 9222 |
| Stage 9 marketplace fanout | `marketplace-coverage` | all marketplace search skills | CDP required for all current browser marketplace skills |
| Stage 9 source isolation | `gem-imports`, `mx-wholesale`, `amazon-uk`, `ebay-uk`, `alibaba-search` | matching `marketplace-search/*` skill | CDP required |
| Stage 11 customs | internal Stage 11 sub-skill | `landed-cost/hmrc-customs-classification` | no browser login expected |
| Stage 11 sizing | internal Stage 11 sub-skill | `landed-cost/shipment-sizing` | CDP required when listing inspection is needed |
| Stage 11 freight | internal Stage 11 sub-skill | `landed-cost/searates-freight-estimate` | CDP required |
| Stage 11 FX | internal Stage 11 sub-skill | `landed-cost/simple-fx-lookup` | `requires_cdp: false` |

The daemon prompt renderer injects this contract for CDP-required skills:

```text
This skill requires the operator persistent CDP browser.
The daemon sets AGENT_BROWSER_CDP_PORT=<port>.
Use agent-browser --cdp ${AGENT_BROWSER_CDP_PORT} for every browser command.
Do not close the operator browser.
If CDP is unreachable, write a blocked result.json; do not launch a browser.
```

If observed behavior contradicts this contract, that is a product/tooling blocker after the first operator-login retry.

---

## Environment Preflight

### Toolchain

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

If `bun install` is needed because this is a fresh fork, run it from repo root. Do not use npm/yarn/pnpm.

```bash
bun install
```

### Persistent CDP Browser

Chrome/Chromium must be running on `localhost:9222` with a non-default automation profile. The canonical launch is:

```bash
open -na "Google Chrome" --args \
  "--remote-debugging-port=9222" \
  "--remote-allow-origins=*" \
  "--user-data-dir=$HOME/.chrome-automation" \
  "--profile-directory=Default" \
  "--no-first-run" \
  "--no-default-browser-check"
```

Verify:

```bash
curl -sfS -m 3 http://localhost:9222/json/version
agent-browser --cdp 9222 open "about:blank"
agent-browser --cdp 9222 get title
```

If this fails, pause for operator browser setup. Do not continue into source skills.

### Login Checks

Use `agent-browser --cdp 9222` so the check uses the same browser state as the daemon skills.

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

Pause for the operator if any page requires sign-in or captcha. For SeaRates, the important state is a usable Logistics Explorer session/token. For Alibaba, the header should show an authenticated account/session or at minimum should not be a sign-in/captcha wall. For ProContract, the supplier home page must not redirect to login.

---

## Isolated Run Setup

Use a fresh state root and a pinned daemon binary. Do not run against `state/state.db` unless the operator explicitly asks.

```bash
export E2E_TS="$(date -u +%Y%m%dT%H%M%SZ)"
export E2E_STATE_ROOT="state-e2e-machine-fork-$E2E_TS"
export E2E_RUNTIME_DIR="runtime-e2e-machine-fork-$E2E_TS"
export E2E_ARTIFACT_ROOT="artifacts-e2e-machine-fork-$E2E_TS"
export E2E_TARGET_DIR="/tmp/testproject1-target-machine-fork-$E2E_TS"
export RUN_START_AT="$(date -u -Iseconds)"

mkdir "$E2E_STATE_ROOT" "$E2E_RUNTIME_DIR" "$E2E_ARTIFACT_ROOT"
CARGO_TARGET_DIR="$E2E_TARGET_DIR" cargo build -p daemon --bin daemon
export E2E_DAEMON="$E2E_TARGET_DIR/debug/daemon"

"$E2E_DAEMON" \
  --state-root "$E2E_STATE_ROOT" \
  --runtime-dir "$E2E_RUNTIME_DIR" \
  --artifact-root "$E2E_ARTIFACT_ROOT" \
  --dry-run
```

Every daemon command in this handoff must use those three root flags.

Record in the validation manifest:

- `E2E_TS`
- `RUN_START_AT`
- daemon binary path
- CDP browser version from `/json/version`
- `git log --oneline -1`
- schema epoch values
- whether any CDP/login pauses occurred

---

## Static And Unit Baseline

Run the Layer 0-3 required gates before the live E2E. If any fail, create an error manifest and pause.

```bash
cargo fmt --check
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test -p daemon --bin daemon
for t in crates/daemon/tests/*.rs; do
  cargo test --test "$(basename "$t" .rs)" || exit 1
done

cd packages/contracts-schema && bun run test && bun run build && cd ../..
cd packages/ui && bun run test && bun run build && cd ../..
cd packages/intelligence && bun run test && bun run build && cd ../..
cd packages/manifests && bun run test && bun run build && cd ../..
cd apps/desktop && bun run test && bun run build && cd ../..
```

Do not use root `bun run test` or root `bun run build` as the only preflight signal for this handoff. The root scripts include `apps/web`, and this run intentionally keeps `apps/web` as a bounded localhost smoke rather than a hard Layer 0-3 acceptance gate.

Optional consolidated checks after the required gates pass:

```bash
bun run test
bun run build
```

If these consolidated checks fail only inside `apps/web`, continue the L0-L3 pipeline after documenting that the `apps/web` smoke is pending. Create an error manifest labelled `apps/web local dashboard smoke` when you reach the web section and can capture focused evidence.

---

## Stage Drive / Validate Loop

For each stage:

1. Run the command.
2. Inspect terminal output and the latest `agent_session_logs`, `workflow_jobs`, `process_stage_status`, and stage-specific tables.
3. If CDP/auth/captcha/session issue: pause for operator and retry.
4. If real bug: create error manifest and pause.
5. If pass: append evidence to the validation manifest and continue.

Use this helper prefix mentally:

```bash
"$E2E_DAEMON" \
  --state-root "$E2E_STATE_ROOT" \
  --runtime-dir "$E2E_RUNTIME_DIR" \
  --artifact-root "$E2E_ARTIFACT_ROOT"
```

### Stage 1 - Source Ingestion

Run live Contracts Finder and ProContract ingestion:

```bash
"$E2E_DAEMON" --state-root "$E2E_STATE_ROOT" --runtime-dir "$E2E_RUNTIME_DIR" --artifact-root "$E2E_ARTIFACT_ROOT" --ingest contracts-finder
"$E2E_DAEMON" --state-root "$E2E_STATE_ROOT" --runtime-dir "$E2E_RUNTIME_DIR" --artifact-root "$E2E_ARTIFACT_ROOT" --ingest procontract
```

Run FTS only if a current fixture or live path is available and documented:

```bash
"$E2E_DAEMON" --state-root "$E2E_STATE_ROOT" --runtime-dir "$E2E_RUNTIME_DIR" --artifact-root "$E2E_ARTIFACT_ROOT" --ingest fts
```

Validate:

```bash
sqlite3 -header "$E2E_STATE_ROOT/state.db" "
SELECT source_id, COUNT(*) AS notices FROM contract_notices GROUP BY source_id;
SELECT COUNT(*) AS processes FROM procurement_processes;
SELECT source_id, circuit_state, health_state FROM source_runtime_state ORDER BY source_id;
SELECT kind, status, COUNT(*) FROM workflow_jobs GROUP BY kind, status ORDER BY kind, status;
"
```

Expected: at least one of ProContract or Contracts Finder produces notices; both should work if network/CDP/API conditions are healthy. ProContract auth failures follow pause rules.

### Stages 2-3 And 5-8 - Deterministic Pipeline

Run for each ingested source:

```bash
"$E2E_DAEMON" --state-root "$E2E_STATE_ROOT" --runtime-dir "$E2E_RUNTIME_DIR" --artifact-root "$E2E_ARTIFACT_ROOT" --pipeline procontract
"$E2E_DAEMON" --state-root "$E2E_STATE_ROOT" --runtime-dir "$E2E_RUNTIME_DIR" --artifact-root "$E2E_ARTIFACT_ROOT" --pipeline contracts-finder
```

Then drain any extraction jobs if the command output or DB shows pending extraction work:

```bash
while sqlite3 "$E2E_STATE_ROOT/state.db" "SELECT COUNT(*) FROM workflow_jobs WHERE status='pending' AND kind LIKE 'extract%';" | rg -q '^[1-9]'; do
  "$E2E_DAEMON" --state-root "$E2E_STATE_ROOT" --runtime-dir "$E2E_RUNTIME_DIR" --artifact-root "$E2E_ARTIFACT_ROOT" --run-next-job
done
```

Validate:

```bash
sqlite3 -header "$E2E_STATE_ROOT/state.db" "
SELECT stage, state, COUNT(*) FROM process_stage_status GROUP BY stage, state ORDER BY stage, state;
SELECT COUNT(*) AS lots FROM notice_lots;
SELECT COUNT(*) AS qualification_runs FROM qualification_runs;
SELECT COUNT(*) AS raw_documents FROM raw_documents;
SELECT COUNT(*) AS document_extractions FROM document_extractions;
SELECT COUNT(*) AS semantic_extractions FROM document_semantic_extractions;
SELECT COUNT(*) AS product_candidates FROM product_candidates;
SELECT COUNT(*) AS search_profiles FROM search_profiles;
"
```

Pick a spotlight process after this phase. Prefer a ProContract process with:

- physical goods or goods-like specification
- documents available or auto-granted after interest
- product candidates/search profiles
- a reasonable deadline
- likely quoteability on marketplaces

Document the selection query and reasoning.

### Stage 4a - ProContract Register Interest

Only run after ProContract login is verified. This is a real external portal mutation but is the allowed one for this QA run.

```bash
"$E2E_DAEMON" --state-root "$E2E_STATE_ROOT" --runtime-dir "$E2E_RUNTIME_DIR" --artifact-root "$E2E_ARTIFACT_ROOT" --ingest procontract-interest
```

Validate:

```bash
sqlite3 -header "$E2E_STATE_ROOT/state.db" "
SELECT stage, state, COUNT(*) FROM process_stage_status WHERE stage='interest_registration' GROUP BY stage, state;
SELECT id, kind, status, last_error FROM workflow_jobs WHERE kind LIKE '%interest%' ORDER BY created_at DESC LIMIT 10;
"
```

Record every portal-visible action: notice ID, title, action, and result. If the browser is signed out or a captcha appears, pause for operator login and retry.

### Stage 4b - ProContract Enrich / Get Documents

```bash
"$E2E_DAEMON" --state-root "$E2E_STATE_ROOT" --runtime-dir "$E2E_RUNTIME_DIR" --artifact-root "$E2E_ARTIFACT_ROOT" --ingest procontract-enrich
```

Validate:

```bash
sqlite3 -header "$E2E_STATE_ROOT/state.db" "
SELECT stage, state, COUNT(*) FROM process_stage_status WHERE stage='tender_enrichment' GROUP BY stage, state;
SELECT COUNT(*) AS tender_document_sets FROM tender_document_sets;
SELECT COUNT(*) AS raw_docs_with_files FROM raw_documents WHERE storage_path IS NOT NULL;
SELECT id, storage_path, mime_type FROM raw_documents WHERE storage_path IS NOT NULL ORDER BY created_at DESC LIMIT 20;
"
```

If enrichment reports not-yet-available buyer response, document the wait strategy and either pause for operator timing or select a previously registered process with available documents.

### Stage 9 - Supplier Price Comparison

Preferred orchestration:

```bash
"$E2E_DAEMON" --state-root "$E2E_STATE_ROOT" --runtime-dir "$E2E_RUNTIME_DIR" --artifact-root "$E2E_ARTIFACT_ROOT" --ingest marketplace-coverage
```

If you need source-specific isolation:

```bash
for s in amazon-uk ebay-uk gem-imports mx-wholesale alibaba-search; do
  "$E2E_DAEMON" --state-root "$E2E_STATE_ROOT" --runtime-dir "$E2E_RUNTIME_DIR" --artifact-root "$E2E_ARTIFACT_ROOT" --ingest "$s"
done
```

Validate:

```bash
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
"
```

Expected: durable attempts for UK-side and China/import-side roles, with quotes when the source cooperates. Alibaba captcha/auth follows pause rules. Public marketplace no-results can be a documented operational outcome if result contracts are valid.

### Stage 10 - Gross Margin / Commercial Basis

Run deterministic pipeline again after Stage 9:

```bash
"$E2E_DAEMON" --state-root "$E2E_STATE_ROOT" --runtime-dir "$E2E_RUNTIME_DIR" --artifact-root "$E2E_ARTIFACT_ROOT" --pipeline procontract
"$E2E_DAEMON" --state-root "$E2E_STATE_ROOT" --runtime-dir "$E2E_RUNTIME_DIR" --artifact-root "$E2E_ARTIFACT_ROOT" --pipeline contracts-finder
```

Validate:

```bash
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

Expected: no unsafe `legacy_single_quote` for fresh commercial-basis paths unless the code explicitly uses it for legacy rows only. Missing import roles should be labelled, not hidden.

### Stage 11 - Logistics / Landed Cost Comparison

Preflight CDP again before this stage:

```bash
curl -sfS -m 3 http://localhost:9222/json/version
```

Run:

```bash
"$E2E_DAEMON" --state-root "$E2E_STATE_ROOT" --runtime-dir "$E2E_RUNTIME_DIR" --artifact-root "$E2E_ARTIFACT_ROOT" --ingest landed-cost
```

Validate:

```bash
sqlite3 -header "$E2E_STATE_ROOT/state.db" "
SELECT decision_grade, COUNT(*) FROM landed_cost_estimates GROUP BY decision_grade;
SELECT product_candidate_id, supplier_quote_id, ship_from_country, decision_grade,
       supplier_cost_gbp_pence, freight_base_gbp_pence, customs_duty_gbp_pence,
       import_vat_gbp_pence, economic_landed_cost_gbp_pence, blockers
FROM landed_cost_estimates ORDER BY updated_at DESC LIMIT 20;
"
```

For import rows, verify the Stage 11 skill chain:

- `skills/landed-cost/hmrc-customs-classification/SKILL.md`
- `skills/landed-cost/shipment-sizing/SKILL.md`
- `skills/landed-cost/searates-freight-estimate/SKILL.md`
- `skills/landed-cost/simple-fx-lookup/SKILL.md`

SeaRates/Alibaba CDP blockers follow pause rules. If retry/cache behavior looks wrong after CDP is healthy, create an error manifest.

### Stage 12 - Net Margin

```bash
"$E2E_DAEMON" --state-root "$E2E_STATE_ROOT" --runtime-dir "$E2E_RUNTIME_DIR" --artifact-root "$E2E_ARTIFACT_ROOT" --ingest net-gate
```

Validate:

```bash
sqlite3 -header "$E2E_STATE_ROOT/state.db" "
SELECT verdict, COUNT(*) FROM candidate_net_margin_assessments GROUP BY verdict;
SELECT product_candidate_id, verdict, net_margin_base_bps, reasons_json
FROM candidate_net_margin_assessments ORDER BY updated_at DESC LIMIT 20;
SELECT pipeline_state, review_required, COUNT(*) FROM pipeline_assessments GROUP BY pipeline_state, review_required;
"
```

Expected: review/reject/pursue outcomes should be backed by reasons, especially for missing China/import quote, fallback screening basis, no pricing signal, or quantity ambiguity.

### Stage 13 - Manifest Generation

Prefer daemon renderer first to decouple QA from model variance. Then, if stable, run normal mode if required.

```bash
STAGE13_RENDERER_MODE=daemon "$E2E_DAEMON" --state-root "$E2E_STATE_ROOT" --runtime-dir "$E2E_RUNTIME_DIR" --artifact-root "$E2E_ARTIFACT_ROOT" --ingest manifest
```

Validate:

```bash
sqlite3 -header "$E2E_STATE_ROOT/state.db" "
SELECT process_id, stoplight, suggested_action, manifest_basis_hash, artifact_path
FROM manifests ORDER BY generated_at DESC LIMIT 20;
"
if test -d "$E2E_ARTIFACT_ROOT/manifests"; then
  find "$E2E_ARTIFACT_ROOT/manifests" -type f -name 'manifest.md' | sort | tail -20
else
  echo "No manifest artifact directory found"
fi
```

Expected: at least one manifest for the spotlight process, with stoplight, suggested action, basis hash, and rendered artifact.

### Stage 14 - Operator Decision And Clarification

Use CLI first:

```bash
export SPOTLIGHT_PROCESS_ID="<set-this-from-selection>"

"$E2E_DAEMON" \
  --state-root "$E2E_STATE_ROOT" \
  --runtime-dir "$E2E_RUNTIME_DIR" \
  --artifact-root "$E2E_ARTIFACT_ROOT" \
  --capture-decision "$SPOTLIGHT_PROCESS_ID" \
  --disposition watch \
  --idempotency-key "machine-fork-e2e-cli-$E2E_TS" \
  --reasoning "Machine-fork E2E validation decision capture."
```

Validate:

```bash
sqlite3 -header "$E2E_STATE_ROOT/state.db" "
SELECT id, process_id, idempotency_key, status, desired_disposition, decision_note, queued_at
FROM pending_operator_intents
WHERE process_id='$SPOTLIGHT_PROCESS_ID'
ORDER BY queued_at DESC LIMIT 10;
"
```

Then validate the same surface through desktop UI in the next section.

---

## Desktop App Validation

Desktop surfaces:

- `apps/desktop/src/App.tsx`
- `apps/desktop/src/pages/ContractList.tsx`
- `apps/desktop/src/pages/ContractDetail.tsx`
- `apps/desktop/src/pages/ManifestInbox.tsx`
- `apps/desktop/src/pages/OperatorDecisionPanel.tsx`
- `apps/desktop/src/pages/DaemonStatus.tsx`
- `apps/desktop/src/pages/Sessions.tsx`
- `apps/desktop/src/pages/Errors.tsx`
- `apps/desktop/src-tauri/src/main.rs`
- `apps/desktop/src-tauri/src/query_service.rs`
- `apps/desktop/src-tauri/src/daemon_bridge.rs`

Run desktop gates:

```bash
cargo test -p procintel-desktop --test query_service
cargo test -p procintel-desktop --test daemon_bridge
cargo test -p procintel-desktop --test native_shell
cd apps/desktop && bun run test && bun run e2e && cd ../..
```

Launch Tauri against the E2E state:

```bash
cd apps/desktop
PROCINTEL_DESKTOP_STATE_ROOT="$OLDPWD/$E2E_STATE_ROOT" \
PROCINTEL_DESKTOP_RUNTIME_DIR="$OLDPWD/$E2E_RUNTIME_DIR" \
PROCINTEL_DESKTOP_ARTIFACT_ROOT="$OLDPWD/$E2E_ARTIFACT_ROOT" \
PROCINTEL_DAEMON_PROGRAM="$E2E_DAEMON" \
bun run tauri:dev
```

If macOS accessibility permissions block automation, document the blocker and either:

- ask the operator to grant permissions, then continue; or
- build a release app bundle with `bun run tauri:build`, open the bundle, grant permissions, and continue.

UI walkthrough:

1. Confirm the app reaches `full control`.
2. Open `Daemon status`; verify instance ID, schema epoch, socket path, state DB path, logs/artifacts buttons.
3. Open `Manifest Inbox`; verify the spotlight manifest card appears.
4. Click through to Process Detail.
5. Verify candidates, quotes, landed costs, net margin, manifest markdown, and error summaries render.
6. Submit one clarification when a prompt is present, or document that none exists.
7. Click `Re-run pipeline`; verify progress/modal and DB changes.
8. Use Quick-Decide `Watch` (or another safe disposition); verify a new `pending_operator_intents` row distinct from the CLI row.
9. If possible, test stale decision behavior by re-running Stage 13 while detail is open; if the basis hash is stable, document "basis stable" rather than forcing a synthetic stale condition.

Capture screenshots under:

```text
docs/pipeline/machine-fork-l0-l3-stages-1-14-e2e-screenshots/
```

Do not delete failed or intermediate screenshots.

---

## Localhost Dashboard / Web Smoke

There are two localhost surfaces:

1. Layer 2 desktop Vite dev bridge at `http://127.0.0.1:5173` (`apps/desktop`, in scope).
2. `apps/web` Next.js dashboard at `http://127.0.0.1:3000` (Layer 4 read-only projection surface; only smoke it because the operator explicitly requested a website dashboard).

Run desktop dev bridge smoke:

```bash
cd apps/desktop
bun run dev
```

Open `http://127.0.0.1:5173` with the Browser skill and check:

- contract list renders
- process detail route renders
- daemon page route renders
- manifest inbox route renders

Run web smoke:

```bash
cd apps/web
bun run test
bun run test:e2e
bun run dev
```

Open:

- `http://127.0.0.1:3000/contracts`
- `http://127.0.0.1:3000/status`
- `http://127.0.0.1:3000/contracts/<known-or-fallback-id>`

If `NEXT_PUBLIC_SUPABASE_URL` and `NEXT_PUBLIC_SUPABASE_ANON_KEY` are absent, the web app is expected to render fallback/empty snapshots. That is not a Layer 0-3 failure. If valid Supabase env is present and the page fails to query or render, create an error manifest, but label it as apps/web local dashboard smoke, not Layer 4 acceptance.

---

## Operational Subsystem Checks

Run after the main pipeline:

```bash
sqlite3 -header "$E2E_STATE_ROOT/state.db" "
SELECT source_id, health_state, circuit_state, last_success_at, last_failure_at
FROM source_runtime_state ORDER BY source_id;

SELECT kind, status, COUNT(*) FROM workflow_jobs GROUP BY kind, status ORDER BY kind, status;

SELECT execution_mode, status, cleanup_verdict, COUNT(*)
FROM agent_session_logs GROUP BY execution_mode, status, cleanup_verdict
ORDER BY execution_mode, status, cleanup_verdict;

SELECT COUNT(*) AS error_manifest_count FROM error_manifests;
SELECT COUNT(*) AS validation_verdict_count FROM validation_verdicts;
"
```

Also verify IPC and scheduler tests:

```bash
cargo test --test ipc_handshake
cargo test --test cron_scheduler
cargo test --test job_claim
cargo test --test resource_arbiter
cargo test --test replay_contract_sessions
cargo test --test replay_pty_sessions
```

Expected:

- source circuits should be `closed` unless a source had a documented auth/captcha/source failure.
- workflow jobs may include explainable retry/failed extraction rows; unexpected failed source jobs need an error manifest.
- validation verdicts may be empty because L4 validator enforcement is out of scope.

---

## Acceptance Criteria

Mark each item in the validation manifest:

1. Preflight recorded: git HEAD, tool versions, schema epochs, CDP browser version, login status.
2. Static/unit baseline gates run or any failure got an error manifest.
3. Fresh isolated state root created and used for every daemon command.
4. Daemon dry-run succeeds.
5. Contracts Finder ingestion succeeds or has a non-auth error manifest.
6. ProContract ingestion succeeds after CDP login, or pauses correctly for login.
7. FTS is run or explicitly documented as unavailable/deferred.
8. Deterministic pipeline creates process/lots/qualification/product/search state.
9. ProContract register-interest cap is respected; all real portal mutations are listed.
10. Document acquisition/enrichment downloads or records tender docs.
11. Document extraction/semantic extraction produces product candidates or documented blockers.
12. Stage 9 writes durable search attempts for marketplace coverage.
13. Stage 9 writes supplier quotes when sources return usable prices, or records structured no-result/captcha/auth outcomes.
14. Alibaba/China-side coverage is attempted or paused for operator login/captcha.
15. Stage 10 writes candidate pricing assessments with honest commercial basis.
16. Stage 11 writes landed-cost rows or structured blockers; SeaRates CDP behavior is validated.
17. Stage 12 writes net-margin assessments with reasons.
18. Stage 13 writes manifest rows and artifacts.
19. Stage 14 CLI decision writes a queued intent.
20. Desktop app reaches full control against the E2E state.
21. Desktop Manifest Inbox and Process Detail render the spotlight opportunity.
22. Desktop clarification/rerun/quick-decision path is exercised or blocked by an error manifest.
23. Desktop screenshots are saved.
24. `apps/desktop` Vite dev dashboard is smoke-tested at `127.0.0.1:5173`.
25. `apps/web` local dashboard is smoke-tested at `127.0.0.1:3000` with Layer 4 semantics excluded.
26. Operational subsystem checks are documented.
27. Any login/CDP issue paused and retried correctly.
28. Any real bug produced an error manifest and paused.
29. Final quality gates run after all fixes are reported complete.
30. Validation manifest clearly lists pass/fail/deferred items and exact next-resume point if not fully green.

---

## Final Quality Gates

Run the required Layer 0-3 gates at the end, after all pauses/fixes/resumes are complete:

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

cd packages/contracts-schema && bun run test && bun run build && cd ../..
cd packages/ui && bun run test && bun run build && cd ../..
cd packages/intelligence && bun run test && bun run build && cd ../..
cd packages/manifests && bun run test && bun run build && cd ../..
cd apps/desktop && bun run test && bun run build && cd ../..
```

Then run the explicit local website smoke gates:

```bash
cd apps/desktop && bun run e2e && cd ../..
cd apps/web && bun run test && bun run test:e2e && cd ../..
```

If `apps/web` fails, classify it under the bounded localhost website smoke unless the evidence proves it is caused by shared Layer 0-3 code or data. Do not let a missing Supabase env or deferred Layer 4 sync behavior turn a successful L0-L3 pipeline into a false RED.

Run UBS scoped to files changed by this QA session:

```bash
git diff --name-only > /tmp/machine-fork-e2e-changed-files.txt
if test -s /tmp/machine-fork-e2e-changed-files.txt; then
  xargs ubs < /tmp/machine-fork-e2e-changed-files.txt
else
  echo "No changed files for UBS"
fi
```

If only Markdown manifests changed and `ubs` does not support them cleanly, document the result and do not force a full-project scan.

---

## Validation Manifest Template

Use this structure in `docs/pipeline/machine-fork-l0-l3-stages-1-14-e2e-validation-manifest.md`:

```markdown
# Machine Fork L0-L3 Stages 1-14 E2E Validation Manifest

Generated: <timestamp>
Status: <GREEN / PAUSED / RED / PARTIAL>
Git HEAD: <hash subject>
State root: <E2E_STATE_ROOT>
Runtime dir: <E2E_RUNTIME_DIR>
Artifact root: <E2E_ARTIFACT_ROOT>
Daemon binary: <E2E_DAEMON>
CDP browser: <Chrome version from /json/version>

## Pause Ledger

| Time | Stage | Cause | Action | Resume Point |
|---|---|---|---|---|

## Error Manifests Created

| Path | Stage | Blocking Symptom | Status |
|---|---|---|---|

## Preflight

<tool versions, schema epochs, CDP/login checks>

## Stage Results

| Stage | Command | Result | Evidence |
|---|---|---|---|

## Spotlight Process

<selection query, process ID, title, buyer, reason chosen>

## CDP/Auth Events

<every login/captcha pause and retry result>

## Desktop And Localhost UI

<desktop, 5173, and 3000 smoke results + screenshots>

## Operational Subsystems

<cron/job/source/session/error/validation summaries>

## Quality Gates

<commands and results>

## Final Decision

<what works on this machine, what is paused, what remains for fixers>
```

---

## Resume Instruction For The QA Agent

When the operator returns and says the error manifest has been fixed, do not restart from scratch unless the fix changed migrations or core setup. Re-run:

1. the specific failing command,
2. the stage validation query,
3. the smallest relevant regression/unit test,
4. then continue with the next stage.

Append the resume event to the validation manifest. If the same failure recurs after a claimed fix, update the existing error manifest with new evidence or create a follow-up error manifest, then pause again.

## Session Completion

When the QA run is finished or paused on an error manifest, follow `AGENTS.md` landing-plane rules for this repository. Commit and push the validation manifest, screenshots, and any error manifests that were created. Do not commit unrelated code fixes unless the operator explicitly changed the task from QA handoff execution into repair work.
