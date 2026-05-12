# Contracts Finder Claude Auth Failure - Fix Handoff

ULTRATHINK and be meticulous. This is a standalone fix handoff for a fresh debugging agent.

## Mission

Fix the Contracts Finder live Stage 1 ingestion blocker by making the selected Claude CLI capable of one minimal authenticated model call, then prove the daemon can run `--ingest contracts-finder` against the preserved E2E roots without failing for a missing result contract. The failure is currently a machine/auth setup blocker, not a CDP/browser failure. Do not weaken result-contract validation, do not silently switch the Contracts Finder skill to Codex, and do not delete or reset the preserved E2E evidence.

## Defect Summary

Live Contracts Finder ingestion uses:

- Skill: `skills/procurement-ingestion/contracts-finder/SKILL.md`
- Daemon path: `crates/daemon/src/main.rs::run_contracts_finder_live_loop`
- Selected agent CLI: `claude`
- E2E context: `state-e2e-machine-fork-20260512T195251Z/e2e-env.sh`

The selected Claude CLI exits before the skill can run:

```text
Failed to authenticate. API Error: 401 Invalid authentication credentials
```

The daemon then correctly fails the job because no valid `result.json` or parser `FinalResultReady` path exists:

```text
contracts-finder skill did not produce a result contract
```

## Current Reproduction Evidence

From repo root:

```bash
cd /Users/zacfarrell/Desktop/Testproject1
source state-e2e-machine-fork-20260512T195251Z/e2e-env.sh
run_logged stage-01-contracts-finder-forensics-claude-auth-recreate-codex \
  claude -p "Return exactly OK." --dangerously-skip-permissions
```

Observed:

```text
$ claude -p Return\ exactly\ OK. --dangerously-skip-permissions
Failed to authenticate. API Error: 401 {"type":"error","error":{"type":"authentication_error","message":"Invalid authentication credentials"},"request_id":"req_011CayKzxdKRaLZ98cNVAqys"}
```

Then:

```bash
run_logged stage-01-contracts-finder-recreate-codex \
  env AGENT_BROWSER_CDP_PORT=9222 "$E2E_DAEMON" \
    --state-root "$E2E_STATE_ROOT" \
    --runtime-dir "$E2E_RUNTIME_DIR" \
    --artifact-root "$E2E_ARTIFACT_ROOT" \
    --ingest contracts-finder
```

Observed:

```text
FTS live loop failed: job_id=job-2c44c70bbaf925cea4a4d7bab48f6ff02333ce1cfb7a222fd1702d7b986d21a8 reason=contracts-finder skill did not produce a result contract
Error: FTS live loop failed: job_id=job-2c44c70bbaf925cea4a4d7bab48f6ff02333ce1cfb7a222fd1702d7b986d21a8 reason=contracts-finder skill did not produce a result contract
```

Fresh PTY evidence:

```bash
zstdcat artifacts-e2e-machine-fork-20260512T195251Z/sessions/2026/05/session-pty-e0e643e05cc0ce79d5401266ec98f7d7841743b8a8df3bfaea03da58cf01bee8/stream/000001.pty.zst
```

Observed:

```text
Failed to authenticate. API Error: 401 {"type":"error","error":{"type":"authentication_error","message":"Invalid authentication credentials"},"request_id":"req_011CayL1YifjkUKQeP4yoUWB"}
```

CDP control is healthy:

```bash
curl -sfS -m 3 http://localhost:9222/json/version
```

Observed browser version in the captured logs:

```text
Chrome/148.0.7778.97
```

Both known Claude binaries currently fail auth:

```bash
command -v claude
ls -l /opt/homebrew/bin/claude /Users/zacfarrell/.local/bin/claude 2>/dev/null || true
/Users/zacfarrell/.local/bin/claude --version 2>&1 || true
/Users/zacfarrell/.local/bin/claude -p "Return exactly OK." --dangerously-skip-permissions 2>&1 || true
/opt/homebrew/bin/claude -p "Return exactly OK." --dangerously-skip-permissions 2>&1 || true
```

Observed:

```text
/opt/homebrew/bin/claude
lrwxr-xr-x@ 1 zacfarrell  staff  54 May 12 21:44 /Users/zacfarrell/.local/bin/claude -> /Users/zacfarrell/.local/share/claude/versions/2.1.139
lrwxr-xr-x  1 zacfarrell  admin  48 Mar 20 12:10 /opt/homebrew/bin/claude -> /opt/homebrew/Caskroom/claude-code/2.1.80/claude
2.1.139 (Claude Code)
Failed to authenticate. API Error: 401 Invalid authentication credentials
Failed to authenticate. API Error: 401 {"type":"error","error":{"type":"authentication_error","message":"Invalid authentication credentials"},"request_id":"req_011CayL69zKtctHgDRxbUrPS"}
```

## Required Reads

1. `prompts/machine-fork-e2e-contracts-finder-claude-auth-error-manifest.md`
   - Read the current reproduction addendum and evidence bundle.
   - Checkpoint: identify the direct auth probe command and the latest reproduced `job_id` / `session_id`.

2. `skills/procurement-ingestion/contracts-finder/SKILL.md`
   - Read the YAML metadata and Mandatory CDP Discipline section.
   - Checkpoint: confirm `agent_cli: claude`, `requires_cdp: true`, and the valid blocked-result behavior if CDP is unavailable.

3. `crates/daemon/src/main.rs`
   - Read CLI args around `--agent-cli` and `--agent-cli-program`.
   - Read `AgentCliKind::env_key`.
   - Read `adapter_runtime_config` and `build_adapter`.
   - Read `launch_args_for_skill`.
   - Read `run_contracts_finder_live_loop`, especially the missing-result branch.
   - Checkpoint: explain adapter selection order and why missing `result_path` becomes `AGENT_RUN_FAILED`.

4. `crates/daemon/src/agents/claude.rs`
   - Read `probe`, `build_launch_spec`, and parser handling for `result` / `auth_challenge`.
   - Checkpoint: explain why `claude --version` does not prove auth health and why Claude CLI v2 falling back from `probe --json` can mask auth status.

5. Regression surfaces:
   - `crates/daemon/tests/adapter_claude.rs`
   - `crates/daemon/tests/agent_procurement_ingestion.rs`
   - `crates/daemon/src/main.rs` tests around `procurement_skills_require_result_contract_to_complete` and `missing_procurement_result_contract_marks_job_failed`

## Constraints

1. Do not delete or recreate these directories:
   - `state-e2e-machine-fork-20260512T195251Z`
   - `runtime-e2e-machine-fork-20260512T195251Z`
   - `artifacts-e2e-machine-fork-20260512T195251Z`
2. Do not delete QA logs or PTY streams.
3. Do not weaken result-contract validation or make missing `result.json` acceptable.
4. Do not change Contracts Finder from Claude to Codex without explicit product approval and regression coverage.
5. Do not close or replace the persistent CDP browser profile on `localhost:9222`.
6. Preserve unrelated staged/modified work. Current unrelated staged entries may include `.DS_Store` and `bin/__pycache__/*`.

## Validate-Test-Fix Loop

Follow this loop exactly. Keep all retry logs under `$E2E_LOG_ROOT` with new labels.

### Loop 1 - Validate and Repair Claude Auth

Run:

```bash
cd /Users/zacfarrell/Desktop/Testproject1
source state-e2e-machine-fork-20260512T195251Z/e2e-env.sh
run_logged stage-01-contracts-finder-validate-agent-cli-auth \
  claude -p "Return exactly OK." --dangerously-skip-permissions
```

Expected after fix:

```text
OK
```

If it still exits non-zero or prints `401 Invalid authentication credentials`, do not run Stage 1 yet. Repair Claude Code authentication first using the project-approved Claude Code auth flow.

Also compare binaries before assuming this is only PATH:

```bash
command -v claude
/opt/homebrew/bin/claude --version || true
/Users/zacfarrell/.local/bin/claude --version || true
/opt/homebrew/bin/claude -p "Return exactly OK." --dangerously-skip-permissions || true
/Users/zacfarrell/.local/bin/claude -p "Return exactly OK." --dangerously-skip-permissions || true
```

If exactly one binary authenticates, validate with the existing override:

```bash
export DAEMON_CLAUDE_PROGRAM=<authenticated-claude-path>
```

Repeat Loop 1 until the minimal model call exits 0.

### Loop 2 - Validate Live Contracts Finder Ingestion

Only run this after Loop 1 passes.

```bash
source state-e2e-machine-fork-20260512T195251Z/e2e-env.sh
run_logged stage-01-contracts-finder-validate-live-ingest \
  env AGENT_BROWSER_CDP_PORT=9222 "$E2E_DAEMON" \
    --state-root "$E2E_STATE_ROOT" \
    --runtime-dir "$E2E_RUNTIME_DIR" \
    --artifact-root "$E2E_ARTIFACT_ROOT" \
    --ingest contracts-finder
```

Pass condition:

- The command exits 0, or exits with a valid structured skill-level blocked/no-result contract.
- The latest `workflow_jobs` row for `scope_id='contracts-finder'` is not failed for `contracts-finder skill did not produce a result contract`.
- The latest retry log and session stream do not contain `Failed to authenticate` or `Invalid authentication credentials`.

Run:

```bash
sqlite3 -header -column "$E2E_STATE_ROOT/state.db" "
SELECT id, kind, status, scope_kind, scope_id, attempt_count, last_error, created_at, started_at, completed_at
FROM workflow_jobs
WHERE scope_id = 'contracts-finder'
ORDER BY created_at DESC
LIMIT 5;
SELECT id, job_id, skill_path, execution_mode, cli_vendor, status, exit_code, cleanup_verdict, terminal_output_bytes, run_manifest_path, terminal_output_path, created_at
FROM agent_session_logs
ORDER BY created_at DESC
LIMIT 5;
SELECT source_id, COUNT(*) AS notices
FROM contract_notices
GROUP BY source_id;
"

if grep -R "Failed to authenticate\|Invalid authentication credentials" \
  "$E2E_LOG_ROOT/stage-01-contracts-finder-validate-live-ingest.log"; then
  exit 1
fi
```

If Loop 2 still fails with Claude 401, return to Loop 1. If Loop 2 fails without Claude 401, preserve the new log/session artifacts and debug the new adapter/session/result-contract failure as a separate issue.

### Loop 3 - Add a Preflight or Runbook Guard if Needed

If this is solved only by machine reauth, no daemon code change is required. Still record the successful Loop 1 and Loop 2 logs.

If code is changed, prefer a narrow preflight that fails before Stage 1 starts when the selected agent CLI cannot perform a minimal authenticated call. Do not weaken `run_contracts_finder_live_loop` missing-result behavior.

Candidate test targets:

```bash
cargo test -p daemon --test adapter_claude
cargo test -p daemon --test agent_procurement_ingestion
cargo test -p daemon --bin daemon procurement_skills_require_result_contract_to_complete
cargo test -p daemon --bin daemon missing_procurement_result_contract_marks_job_failed
```

Expected preservation:

- Fake adapter success paths still pass.
- Procurement ingestion skills still require a result contract.
- Missing result contracts still fail loudly with `AGENT_RUN_FAILED` context.

### Loop 4 - Full Regression for Code Changes

Run this only if code changed:

```bash
cargo fmt --check
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test -p daemon --test agent_procurement_ingestion
cargo test -p daemon --test adapter_claude
cargo test -p procintel-desktop --test native_shell
ubs <changed-rust-or-ts-files>
```

If any command fails, fix the root cause and restart from the narrowest failing loop.

## Acceptance Criteria

- [ ] `claude -p "Return exactly OK." --dangerously-skip-permissions` exits 0 on this machine.
- [ ] The exact Contracts Finder live ingestion command is rerun against the preserved E2E roots.
- [ ] The latest Contracts Finder job is not failed for missing result contract due to Claude auth.
- [ ] Latest retry logs are written under `$E2E_LOG_ROOT`.
- [ ] Latest retry log/session stream no longer contain `Failed to authenticate` or `Invalid authentication credentials`.
- [ ] If code changed, focused regression tests pass and loud missing-result semantics remain intact.

## File Map

| File | Role |
|------|------|
| `prompts/machine-fork-e2e-contracts-finder-claude-auth-error-manifest.md` | Full forensic error manifest and current reproduction evidence. |
| `skills/procurement-ingestion/contracts-finder/SKILL.md` | Skill metadata and live Contracts Finder playbook. |
| `crates/daemon/src/main.rs` | CLI dispatch, adapter selection, PTY skill execution, missing-result failure path. |
| `crates/daemon/src/agents/claude.rs` | Claude adapter launch/probe/parser behavior. |
| `crates/daemon/tests/adapter_claude.rs` | Claude adapter regression tests. |
| `crates/daemon/tests/agent_procurement_ingestion.rs` | Procurement ingestion fake-adapter tests. |
| `state-e2e-machine-fork-20260512T195251Z/e2e-env.sh` | Isolated E2E root/runtime/artifact/log env and `run_logged`. |
| `artifacts-e2e-machine-fork-20260512T195251Z/qa-logs/stage-01-contracts-finder-forensics-claude-auth-recreate-codex.log` | Fresh direct Claude auth failure log. |
| `artifacts-e2e-machine-fork-20260512T195251Z/qa-logs/stage-01-contracts-finder-recreate-codex.log` | Fresh failed live ingestion log. |
| `artifacts-e2e-machine-fork-20260512T195251Z/qa-logs/stage-01-contracts-finder-recreate-codex-db-v2.log` | Fresh SQLite job/session validation. |
| `artifacts-e2e-machine-fork-20260512T195251Z/sessions/2026/05/session-pty-e0e643e05cc0ce79d5401266ec98f7d7841743b8a8df3bfaea03da58cf01bee8/stream/000001.pty.zst` | Fresh PTY stream containing Claude auth failure. |
