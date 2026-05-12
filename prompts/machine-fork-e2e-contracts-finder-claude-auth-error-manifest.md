# Contracts Finder Live Ingestion Claude Auth Failure - QA Error Manifest

ULTRATHINK and be meticulous. This is a forensic debugging document.

## DEFECT FINGERPRINT

| Field | Value |
|-------|-------|
| **Type** | machine_setup_blocker / integration_mismatch |
| **Severity** | high |
| **Reproducibility** | deterministic on this machine at time of QA |
| **Affected Component** | Contracts Finder live ingestion PTY skill path: `crates/daemon/src/main.rs::run_contracts_finder_live_loop` using `skills/procurement-ingestion/contracts-finder/SKILL.md` |
| **Working Analogue** | Static daemon and desktop gates passed with fake/test adapters; CDP browser and `agent-browser` control are healthy. |
| **Scope** | Live Stage 1 cannot start because the Claude CLI selected by the daemon exits with `401 Invalid authentication credentials` before writing `result.json`. |
| **Environment** | macOS Darwin 25.3.0 arm64; Bun 1.3.13; Cargo 1.96.0-nightly; `claude` 2.1.80; `codex-cli` 0.130.0; `agent-browser` 0.27.0; Chrome CDP 148.0.7778.97 |
| **Git State** | `main` at `9d15b9c fix: serialize native shell test hooks`; shared checkout also has unrelated staged `.DS_Store` and `bin/__pycache__` entries left untouched |

---

## REPRODUCTION PROTOCOL

Follow these steps exactly from the repo root.

### Prerequisites

- Repository: `/Users/zacfarrell/Desktop/Testproject1`
- Branch: `main`
- E2E context: `/Users/zacfarrell/Desktop/Testproject1/state-e2e-machine-fork-20260512T195251Z/e2e-env.sh`
- The daemon binary has been rebuilt at `/tmp/testproject1-target-machine-fork-20260512T195251Z/debug/daemon`.
- Persistent CDP is already healthy on `localhost:9222`; this defect reproduces before Contracts Finder page/API access.

### Step 1: Source the E2E context

```bash
cd /Users/zacfarrell/Desktop/Testproject1
source state-e2e-machine-fork-20260512T195251Z/e2e-env.sh
```

Expected: shell exports the isolated state, runtime, artifact, log, and daemon paths.

Actual: context loads successfully.

### Step 2: Confirm the selected live skill adapter cannot authenticate

```bash
run_logged stage-01-contracts-finder-forensics-claude-auth \
  claude -p "Return exactly OK." --dangerously-skip-permissions
```

Expected: the CLI returns a short successful model response.

Actual:

```text
$ claude -p Return\ exactly\ OK. --dangerously-skip-permissions
Failed to authenticate. API Error: 401 Invalid authentication credentials
```

### Step 3: Run real Contracts Finder Stage 1 ingestion

```bash
run_logged stage-01-contracts-finder \
  env AGENT_BROWSER_CDP_PORT=9222 "$E2E_DAEMON" \
    --state-root "$E2E_STATE_ROOT" \
    --runtime-dir "$E2E_RUNTIME_DIR" \
    --artifact-root "$E2E_ARTIFACT_ROOT" \
    --ingest contracts-finder
```

Expected: the daemon launches the Contracts Finder skill, the skill writes a valid `result.json`, and `contract_notices` receives live source rows.

Actual:

```text
FTS live loop failed: job_id=job-c3d5b06af6f38699d94beb2d5a9458e7038d457edb0a291526d0e28108d29807 reason=contracts-finder skill did not produce a result contract
Error: FTS live loop failed: job_id=job-c3d5b06af6f38699d94beb2d5a9458e7038d457edb0a291526d0e28108d29807 reason=contracts-finder skill did not produce a result contract
```

### Verification - Confirm Defect Present

```bash
source state-e2e-machine-fork-20260512T195251Z/e2e-env.sh
sed -n '1,220p' "$E2E_LOG_ROOT/stage-01-contracts-finder.log"
sed -n '1,120p' "$E2E_LOG_ROOT/stage-01-contracts-finder-forensics-session-stream.log"
```

Output contains:

```text
contracts-finder skill did not produce a result contract
Failed to authenticate. API Error: 401 Invalid authentication credentials
```

### Control - CDP Is Not The Failing Layer

```bash
source state-e2e-machine-fork-20260512T195251Z/e2e-env.sh
sed -n '1,120p' "$E2E_LOG_ROOT/stage-01-contracts-finder-forensics-cli-tools.log"
```

Output:

```text
/opt/homebrew/bin/claude
2.1.80 (Claude Code)
/opt/homebrew/bin/codex
codex-cli 0.130.0
/opt/homebrew/bin/agent-browser
agent-browser 0.27.0
{
   "Browser": "Chrome/148.0.7778.97",
   "Protocol-Version": "1.3",
   "webSocketDebuggerUrl": "ws://localhost:9222/devtools/browser/82e48ff8-b761-4922-873a-4b34ff03cc57"
}
```

---

## EVIDENCE BUNDLE

### E0 - Environment Evidence

**E0.1: Git state**

```bash
git status --short --branch
git log --oneline -1
```

```text
## main...origin/main
M  .DS_Store
A  bin/__pycache__/br-claimcpython-313.pyc
A  bin/__pycache__/br-completecpython-313.pyc
 M docs/pipeline/machine-fork-l0-l3-stages-1-14-e2e-validation-manifest.md
 M prompts/machine-fork-e2e-desktop-native-shell-rerun-lock-error-manifest.md
9d15b9c fix: serialize native shell test hooks
```

**E0.2: E2E roots**

```bash
source state-e2e-machine-fork-20260512T195251Z/e2e-env.sh
printf '%s\n' "$E2E_STATE_ROOT" "$E2E_RUNTIME_DIR" "$E2E_ARTIFACT_ROOT" "$E2E_LOG_ROOT" "$E2E_DAEMON"
```

```text
/Users/zacfarrell/Desktop/Testproject1/state-e2e-machine-fork-20260512T195251Z
/Users/zacfarrell/Desktop/Testproject1/runtime-e2e-machine-fork-20260512T195251Z
/Users/zacfarrell/Desktop/Testproject1/artifacts-e2e-machine-fork-20260512T195251Z
/Users/zacfarrell/Desktop/Testproject1/artifacts-e2e-machine-fork-20260512T195251Z/qa-logs
/tmp/testproject1-target-machine-fork-20260512T195251Z/debug/daemon
```

**E0.3: Tool auth and CDP evidence**

```bash
source state-e2e-machine-fork-20260512T195251Z/e2e-env.sh
run_logged stage-01-contracts-finder-forensics-cli-tools zsh -lc \
  'which claude || true; claude --version || true; which codex || true; codex --version || true; which agent-browser || true; agent-browser --version || true; curl -sfS -m 3 http://localhost:9222/json/version'
```

```text
/opt/homebrew/bin/claude
2.1.80 (Claude Code)
/opt/homebrew/bin/codex
codex-cli 0.130.0
/opt/homebrew/bin/agent-browser
agent-browser 0.27.0
Browser Chrome/148.0.7778.97, Protocol-Version 1.3
```

**E0.4: Direct Claude auth probe fails**

```bash
source state-e2e-machine-fork-20260512T195251Z/e2e-env.sh
run_logged stage-01-contracts-finder-forensics-claude-auth \
  claude -p "Return exactly OK." --dangerously-skip-permissions
```

```text
$ claude -p Return\ exactly\ OK. --dangerously-skip-permissions
Failed to authenticate. API Error: 401 Invalid authentication credentials
```

### E2 - Reproduction Evidence

**E2.1: Stage 1 daemon command fails**

```bash
source state-e2e-machine-fork-20260512T195251Z/e2e-env.sh
sed -n '1,220p' "$E2E_LOG_ROOT/stage-01-contracts-finder.log"
```

```text
$ env AGENT_BROWSER_CDP_PORT=9222 /tmp/testproject1-target-machine-fork-20260512T195251Z/debug/daemon --state-root /Users/zacfarrell/Desktop/Testproject1/state-e2e-machine-fork-20260512T195251Z --runtime-dir /Users/zacfarrell/Desktop/Testproject1/runtime-e2e-machine-fork-20260512T195251Z --artifact-root /Users/zacfarrell/Desktop/Testproject1/artifacts-e2e-machine-fork-20260512T195251Z --ingest contracts-finder
2026-05-12T20:34:18.281813Z  INFO daemon bootstrap ready socket_path=/Users/zacfarrell/Desktop/Testproject1/runtime-e2e-machine-fork-20260512T195251Z/daemon.sock state_db=/Users/zacfarrell/Desktop/Testproject1/state-e2e-machine-fork-20260512T195251Z/state.db artifact_root=/Users/zacfarrell/Desktop/Testproject1/artifacts-e2e-machine-fork-20260512T195251Z
2026-05-12T20:34:18.282988Z  WARN L4 startup sync degraded before cross-authority dispatch; local operation continues failure=remote-owned pull unavailable for shared_source_configs
FTS live loop failed: job_id=job-c3d5b06af6f38699d94beb2d5a9458e7038d457edb0a291526d0e28108d29807 reason=contracts-finder skill did not produce a result contract
Error: FTS live loop failed: job_id=job-c3d5b06af6f38699d94beb2d5a9458e7038d457edb0a291526d0e28108d29807 reason=contracts-finder skill did not produce a result contract
```

**E2.2: Workflow job and session rows**

```bash
source state-e2e-machine-fork-20260512T195251Z/e2e-env.sh
sed -n '1,220p' "$E2E_LOG_ROOT/stage-01-contracts-finder-forensics-db.log"
```

```text
job-c3d5b06af6f38699d94beb2d5a9458e7038d457edb0a291526d0e28108d29807  sync_source  failed  source  contracts-finder  1  contracts-finder skill did not produce a result contract  2026-05-12T20:34:18.284574+00:00  2026-05-12T20:34:23.364034+00:00
session-pty-99e3c444b8b8f752709101a88a2ec48c6fdeacdc090af0c9a58ff69a2d29d110  job-c3d5b06af6f38699d94beb2d5a9458e7038d457edb0a291526d0e28108d29807  skills/procurement-ingestion/contracts-finder/SKILL.md  pty  completed  exit_code=1  cleanup_verdict=clean  terminal_output_bytes=131
contract_notice_count
0
process_count
0
error_manifest_count
0
```

**E2.3: PTY stream shows CLI auth failure**

```bash
source state-e2e-machine-fork-20260512T195251Z/e2e-env.sh
sed -n '1,120p' "$E2E_LOG_ROOT/stage-01-contracts-finder-forensics-session-stream.log"
```

```text
{"chunk":"stream/000001.pty.zst","first_seq":1,"last_seq":2,"byte_length":131,"wall_ts_start":"2026-05-12T20:34:22.869530+00:00","wall_ts_end":"2026-05-12T20:34:23.310795+00:00","sha256":"c879ffefaf0ea8abf41ddad295f132d3ae346a3ef8c7a48d217e9bf305644d76"}
Failed to authenticate. API Error: 401 Invalid authentication credentials
```

**E2.4: No result contract or run manifest was produced for the session**

```bash
source state-e2e-machine-fork-20260512T195251Z/e2e-env.sh
sed -n '1,220p' "$E2E_LOG_ROOT/stage-01-contracts-finder-forensics-artifacts.log"
```

```text
/Users/zacfarrell/Desktop/Testproject1/artifacts-e2e-machine-fork-20260512T195251Z/qa-logs/stage-01-contracts-finder.log
/Users/zacfarrell/Desktop/Testproject1/artifacts-e2e-machine-fork-20260512T195251Z/sessions/2026/05/session-pty-99e3c444b8b8f752709101a88a2ec48c6fdeacdc090af0c9a58ff69a2d29d110/stream.index.jsonl
/Users/zacfarrell/Desktop/Testproject1/artifacts-e2e-machine-fork-20260512T195251Z/sessions/2026/05/session-pty-99e3c444b8b8f752709101a88a2ec48c6fdeacdc090af0c9a58ff69a2d29d110/stream/000001.pty.zst
```

### E3 - Codepath Evidence

**E3.1: Contracts Finder skill explicitly selects Claude and requires CDP**

```bash
sed -n '1,32p' skills/procurement-ingestion/contracts-finder/SKILL.md
```

```yaml
skill_id: procurement-ingestion/contracts-finder
skill_version: 1.0.0
interaction_profile: pty_interactive
result_schema_version: 2.0.0
model_profile: browse_stable_low_cost
agent_cli: claude
side_effect_class: none
requires_cdp: true
cdp_default_port: 9222
```

**E3.2: The daemon default agent CLI is Claude**

```bash
nl -ba crates/daemon/src/main.rs | sed -n '224,232p'
```

```rust
   224 const BACKTRACK_WINDOW_SECONDS: u32 = 60;
   225 const CLI_VENDOR: &str = "fixture-cli";
   226 const CLI_VERSION: &str = "1.0.0";
   227 const DEFAULT_AGENT_CLI: &str = "claude";
   228 const LOCAL_EXECUTOR_ID: &str = "local-daemon";
```

**E3.3: Contracts Finder live loop fails when `ExecutedSession.result_path` is absent**

```bash
nl -ba crates/daemon/src/main.rs | sed -n '16198,16237p'
```

```rust
 16198     // Execute PTY skill session.
 16199     let session_id = format!("session-pty-{}", sha256_hex(job_id.as_bytes()));
 16200     let executed = execute_skill_session(
 16201         runtime,
 16202         &ingest_skill,
 16203         &job_id,
 16204         &session_id,
 16205         &job_parameters,
 16206         cli,
 16207     )
 16208     .await?;
...
 16222     // Read and parse the agent's result contract.
 16223     let result_path = match executed.result_path.as_ref() {
 16224         Some(path) => path.clone(),
 16225         None => {
 16226             return fail_job(
 16227                 runtime,
 16228                 &job_id,
 16229                 "contracts-finder skill did not produce a result contract",
```

**E3.4: PTY supervision only accepts a valid `result.json` or parser `FinalResultReady` path**

```bash
nl -ba crates/daemon/src/main.rs | sed -n '15685,15700p'
```

```rust
 15685     // Look for the result contract written by the agent: prefer the parser's
 15686     // FinalResultReady event, then fall back to the workdir result.json.
 15687     let result_path = events
 15688         .iter()
 15689         .find_map(|event| match event {
 15690             ControlEvent::FinalResultReady { path } => Some(path.clone()),
 15691             _ => None,
 15692         })
 15693         .or_else(|| {
 15694             if workdir_result_path.exists() {
 15695                 Some(workdir_result_path.clone())
 15696             } else {
 15697                 None
 15698             }
 15699         })
 15700         .filter(|path| read_result_contract(path).is_ok());
```

---

## CODEPATH COMPARISON

### Working Path: Static Gates And Fake Adapter Tests

Entry points:

- `cargo fmt --check`
- `cargo check --all-targets`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test -p daemon --bin daemon`
- all `crates/daemon/tests/*.rs`
- desktop `query_service`, `daemon_bridge`, and `native_shell`

Observed result: all required static/unit gates passed after commit `9d15b9c`. These paths do not require the real Claude CLI to authenticate against the API; tests use fake adapter programs where needed.

### Broken Path: Live Contracts Finder PTY Skill

Entry point: `crates/daemon/src/main.rs::run_contracts_finder_live_loop`

Flow:

1. Load `skills/procurement-ingestion/contracts-finder`.
2. Insert and claim a `workflow_jobs` row for `sync_source` / `contracts-finder`.
3. Launch the skill through the selected agent adapter.
4. The selected `claude` binary exits with API 401 before the skill can write `result.json`.
5. `execute_pty_skill_session` returns no valid `result_path`.
6. `run_contracts_finder_live_loop` calls `fail_job` with `contracts-finder skill did not produce a result contract`.

### Divergence Points

| # | Working Path | Broken Path | Potential Impact |
|---|-------------|-------------|------------------|
| 1 | Tests use fake/controlled adapters or pure local compilation. | Live Stage 1 uses real `/opt/homebrew/bin/claude`. | A local CLI auth failure only appears during live ingestion. |
| 2 | CDP preflight uses `agent-browser` directly and succeeds. | Skill never reaches browser/OCDS work because Claude auth fails first. | Retrying CDP login cannot resolve this blocker. |
| 3 | Static gates have no result contract obligation. | Live skill must write `result.json`. | Missing result contract correctly fails the job and blocks source ingestion. |

---

## ROOT CAUSE HYPOTHESES

### H1: Local Claude CLI credentials are invalid or expired - LIKELIHOOD: HIGH

**Description**: The daemon and the Contracts Finder skill select `claude`. A direct `claude -p` command fails with the same `401 Invalid authentication credentials` as the captured PTY stream.

**Supporting evidence**: E0.4, E2.3, E3.1, E3.2.

**Contradicting evidence**: `claude --version` works, but that does not prove API authentication.

**Verification steps**:

1. Run `claude auth` or the project-approved Claude Code auth repair flow outside this QA session.
2. Re-run:
   ```bash
   claude -p "Return exactly OK." --dangerously-skip-permissions
   ```
3. Re-run the exact Stage 1 daemon command and confirm a `result_path` appears in `agent_session_logs`.

**Fix scope**: machine/user Claude Code authentication state, API key helper, or environment used by `/opt/homebrew/bin/claude`.

### H2: The daemon is selecting the wrong Claude binary or credential context - LIKELIHOOD: MEDIUM

**Description**: Earlier preflight documentation showed `claude` resolving under `/Users/zacfarrell/.local/bin/claude`, while the current live failure resolves `/opt/homebrew/bin/claude`. The daemon launches `claude` by program name unless `DAEMON_CLAUDE_PROGRAM` or `--agent-cli-program` overrides it.

**Supporting evidence**: E0.3, E3.2, E3.3.

**Contradicting evidence**: The direct `/opt/homebrew/bin/claude` path is visible and versioned; there is no evidence yet that another installed Claude binary is authenticated.

**Verification steps**:

1. Compare:
   ```bash
   command -v claude
   /Users/zacfarrell/.local/bin/claude --version || true
   /Users/zacfarrell/.local/bin/claude -p "Return exactly OK." --dangerously-skip-permissions || true
   /opt/homebrew/bin/claude -p "Return exactly OK." --dangerously-skip-permissions || true
   ```
2. If exactly one binary is authenticated, re-run the daemon command with the approved override:
   ```bash
   env DAEMON_CLAUDE_PROGRAM=<authenticated-claude-path> "$E2E_DAEMON" ... --ingest contracts-finder
   ```

**Fix scope**: PATH, `DAEMON_CLAUDE_PROGRAM`, daemon adapter config, or machine-local Claude installation.

### H3: Live ingestion should be using the Codex adapter on this machine - LIKELIHOOD: LOW

**Description**: `codex` is available and authenticated for this session, but the Contracts Finder skill metadata and daemon default select Claude. Switching adapters may or may not be a valid product decision; this QA session should not make that change.

**Supporting evidence**: E0.3, E3.1, E3.2.

**Contradicting evidence**: The skill explicitly declares `agent_cli: claude`; changing adapter semantics could invalidate existing skill assumptions and acceptance criteria.

**Verification steps**:

1. Inspect adapter compatibility tests for Contracts Finder PTY skills.
2. Only after product owner/fixer approval, test a bounded retry with `--agent-cli codex`.
3. If Codex succeeds and Claude remains unauthenticated, decide whether the correct fix is machine setup or adapter selection.

**Fix scope**: skill metadata, daemon adapter defaulting, or runbook/machine setup instructions.

---

## INVESTIGATION ROADMAP

### Step 1: Repair or verify Claude CLI auth

Run a minimal model call:

```bash
claude -p "Return exactly OK." --dangerously-skip-permissions
```

If confirmed fixed, continue to Step 2. If it still returns 401, the pipeline cannot run real live source skills that declare `agent_cli: claude`.

### Step 2: Retry the exact failed daemon command

```bash
source /Users/zacfarrell/Desktop/Testproject1/state-e2e-machine-fork-20260512T195251Z/e2e-env.sh
run_logged stage-01-contracts-finder-retry-after-claude-auth \
  env AGENT_BROWSER_CDP_PORT=9222 "$E2E_DAEMON" \
    --state-root "$E2E_STATE_ROOT" \
    --runtime-dir "$E2E_RUNTIME_DIR" \
    --artifact-root "$E2E_ARTIFACT_ROOT" \
    --ingest contracts-finder
```

If the command succeeds, run the Stage 1 validation query and resume with ProContract ingestion. If it fails with the same 401, update this manifest with the retry log and pause again.

### Step 3: Verify result contract persistence

```bash
sqlite3 -header -column "$E2E_STATE_ROOT/state.db" "
SELECT id, kind, status, scope_id, attempt_count, last_error
FROM workflow_jobs
ORDER BY created_at DESC
LIMIT 5;
SELECT id, job_id, skill_path, status, exit_code, run_manifest_path, terminal_output_path
FROM agent_session_logs
ORDER BY created_at DESC
LIMIT 5;
SELECT source_id, COUNT(*) AS notices FROM contract_notices GROUP BY source_id;
"
```

If `contract_notices` remains empty and the session still has no result contract, continue debugging the adapter/session path. If rows exist, resume the original E2E handoff at Stage 1 ProContract ingestion.

---

## FIX CONSTRAINTS

1. Do not delete the E2E state root, runtime dir, artifact root, QA logs, session artifacts, or forensic streams.
2. Do not repair this by editing `PRAGMA user_version` or weakening result-contract validation.
3. Do not make the QA runner silently accept missing `result.json`; live source failures must stay loud.
4. Do not change skill adapter selection from Claude to Codex without an explicit product decision and regression coverage.
5. Do not close or replace the persistent CDP browser profile on `localhost:9222`.
6. Preserve all unrelated staged checkout changes.

---

## ACCEPTANCE CRITERIA

### Functional

- [ ] `claude -p "Return exactly OK." --dangerously-skip-permissions` exits 0 on this machine.
- [ ] The exact Contracts Finder command exits 0 using the isolated E2E roots.
- [ ] `agent_session_logs` for the retried Contracts Finder job has a valid result contract or run manifest path.
- [ ] `contract_notices` contains live `contracts-finder` rows, or the skill writes a valid structured no-result/blocker contract.

### Evidence-Based

- [ ] The retry log is written under `$E2E_LOG_ROOT`.
- [ ] The retry session stream no longer contains `Failed to authenticate. API Error: 401 Invalid authentication credentials`.
- [ ] The validation query shows the latest `workflow_jobs` row for `contracts-finder` is not failed for missing result contract.

### Regression

- [ ] `cargo test -p daemon --test agent_procurement_ingestion` passes.
- [ ] `cargo test -p daemon --test adapter_claude` passes.
- [ ] The previous native shell fix remains green: `cargo test -p procintel-desktop --test native_shell`.

### Test

- [ ] Add or update a machine preflight/runbook check so live E2E blocks before Stage 1 if the selected agent CLI cannot make a minimal authenticated call.
- [ ] If code changes are made, add a regression test that preserves loud failure semantics for missing result contracts.

---

## FILE MAP

| File | Role | Lines Referenced |
|------|------|------------------|
| `skills/procurement-ingestion/contracts-finder/SKILL.md` | Skill metadata and CDP/result contract obligations. | 1-32 |
| `crates/daemon/src/main.rs` | Daemon CLI dispatch, default adapter, PTY execution, missing-result failure handling. | 224-232, 16048-16074, 16124-16237 |
| `crates/daemon/src/agents/claude.rs` | Claude adapter/parser behavior. | 24-112, 125-195 |
| `state-e2e-machine-fork-20260512T195251Z/e2e-env.sh` | Isolated QA run context and `run_logged` helper. | whole file |
| `artifacts-e2e-machine-fork-20260512T195251Z/qa-logs/stage-01-contracts-finder.log` | Primary failed daemon command log. | whole file |
| `artifacts-e2e-machine-fork-20260512T195251Z/qa-logs/stage-01-contracts-finder-forensics-session-stream.log` | Captured PTY stream proving CLI auth failure. | whole file |
| `artifacts-e2e-machine-fork-20260512T195251Z/qa-logs/stage-01-contracts-finder-forensics-db.log` | SQLite workflow/session/source-state evidence. | whole file |
| `artifacts-e2e-machine-fork-20260512T195251Z/qa-logs/stage-01-contracts-finder-forensics-claude-auth.log` | Direct Claude auth probe evidence. | whole file |
