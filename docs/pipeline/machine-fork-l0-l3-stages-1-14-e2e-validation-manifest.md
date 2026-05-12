# Machine Fork L0-L3 Stages 1-14 E2E Validation Manifest

Generated: 2026-05-12T18:46:53Z
Status: PAUSED
Git HEAD: 4d1d1dd docs: persist machine fork e2e helpers
State root: not created; QA paused during critical current-state preflight before isolated run setup
Runtime dir: not created; QA paused during critical current-state preflight before isolated run setup
Artifact root: not created; QA paused during critical current-state preflight before isolated run setup
QA log root: not created; QA paused during critical current-state preflight before isolated run setup
Daemon binary: not built; QA paused before isolated daemon build
CDP browser: unavailable; `curl -sfS -m 3 http://localhost:9222/json/version` failed to connect

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

## Error Manifests Created

| Path | Stage | Blocking Symptom | Status |
|---|---|---|---|
| `prompts/machine-fork-e2e-schema-epoch-mismatch-error-manifest.md` | Critical current-state checks / Desktop validation precondition | Daemon `DEFAULT_SCHEMA_EPOCH=9`; desktop expects epoch 8 and rejects mismatched state/handshake. | Open |
| `prompts/machine-fork-e2e-agent-browser-missing-error-manifest.md` | Environment preflight / Browser testing precondition | `agent-browser` command not found, so CDP preflight and daemon-driven browser skills cannot be validated. | Open |

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

## Stage Results

| Stage | Command | Result | Evidence |
|---|---|---|---|
| AGENTS.md read | `sed -n '1,260p' AGENTS.md`; `sed -n '261,620p' AGENTS.md`; `sed -n '621,980p' AGENTS.md` | PASS | Repo rules read before validation. |
| Git state | `git status --short --branch`; `git log --oneline -1` | PASS with dirty shared checkout | On `main` at `4d1d1dd`; unrelated `.DS_Store` and `bin/__pycache__/` present and untouched. |
| Schema epoch current-state check | `rg -n "...SCHEMA_EPOCH..." ...` | BLOCKED | Daemon epoch 9, desktop epoch 8. Error manifest created. |
| Origin freshness | `git fetch origin`; `git show origin/main:...` | PASS | `origin/main` is also `4d1d1dd` and has the same epoch mismatch. |
| Toolchain preflight | `which cargo`; `which bun`; `which agent-browser`; version commands | BLOCKED | Cargo and Bun present; `agent-browser` missing. Error manifest created. |
| CDP preflight | `curl -sfS -m 3 http://localhost:9222/json/version` | PAUSED | First CDP setup failure; no listener on port 9222. |
| Static/unit baseline | Not run | DEFERRED | Real blocker found before baseline gates. Resume after blocker fix. |
| Isolated daemon dry-run | Not run | DEFERRED | Paused before isolated run setup. |
| Stages 1-14 | Not run | DEFERRED | Paused before live pipeline. |
| Desktop UI | Not run | BLOCKED | Desktop schema epoch mismatch prevents full-control validation. |
| Desktop Vite localhost | Not run | DEFERRED | Browser tooling/CDP preflight blocked. |
| apps/web localhost smoke | Not run | DEFERRED | Browser tooling/CDP preflight blocked. |

## Spotlight Process

Not selected. The run paused before source ingestion and deterministic pipeline execution.

## CDP/Auth Events

No source login checks were attempted because `agent-browser` is missing and no browser is listening on `localhost:9222`.

First setup failure evidence:

```text
curl: (7) Failed to connect to localhost port 9222 after 0 ms: Couldn't connect to server
zsh:1: command not found: agent-browser
```

Resume CDP setup with:

```bash
open -na "Google Chrome" --args \
  "--remote-debugging-port=9222" \
  "--remote-allow-origins=*" \
  "--user-data-dir=$HOME/.chrome-automation" \
  "--profile-directory=Default" \
  "--no-first-run" \
  "--no-default-browser-check"
curl -sfS -m 3 http://localhost:9222/json/version
agent-browser --cdp 9222 open "about:blank"
agent-browser --cdp 9222 get title
```

## Desktop And Localhost UI

Not executed.

Desktop UI validation is blocked by the schema epoch mismatch:

```text
Daemon schema epoch: 9
Desktop expected schema epoch: 8
```

Desktop full-control validation must resume only after the constants and tests are aligned with the daemon epoch and the desktop gates pass.

Screenshot directory prepared:

```text
docs/pipeline/machine-fork-l0-l3-stages-1-14-e2e-screenshots/
```

No screenshots were captured before the pause.

## Operational Subsystems

Not executed. No isolated `state.db` was created before the pause, so no SQLite integrity, source runtime, workflow job, session log, error manifest table, validation verdict, IPC, scheduler, job claim, resource arbiter, or replay checks were run.

## Quality Gates

Not executed. The handoff requires a pause for real blockers before continuing the E2E run.

Minimum resume gates after the schema/tooling fix:

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

QA is PAUSED before live E2E execution.

Proven on this machine:

- The checkout is on `main` at `4d1d1dd`.
- Cargo and Bun are present.
- `origin/main` does not contain a schema epoch fix.
- The daemon schema epoch is 9 because migration `0012_l4_sync_state` is present.
- Desktop/Tauri still expects schema epoch 8 in native bootstrap, query service, daemon bridge, and tests.
- `agent-browser` is missing from `PATH`.
- No persistent Chrome CDP listener is available on `localhost:9222`.

Exact resume point:

1. Fix `prompts/machine-fork-e2e-schema-epoch-mismatch-error-manifest.md`.
2. Fix `prompts/machine-fork-e2e-agent-browser-missing-error-manifest.md`.
3. Launch or repair Chrome CDP on port 9222.
4. Re-run the schema epoch `rg` command and CDP/toolchain preflight.
5. If green, continue from Environment Preflight and Isolated Run Setup in the original handoff.
