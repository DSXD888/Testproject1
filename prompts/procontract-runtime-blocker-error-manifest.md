# ProContract E2E Runtime Blockers — QA Error Manifest

ULTRATHINK and be meticulous. This is a forensic debugging document.

## DEFECT FINGERPRINT

| Field | Value |
|-------|-------|
| **Type** | integration_mismatch |
| **Severity** | high |
| **Reproducibility** | mixed: deterministic for `codex` adapter mismatch, environment-specific for nested `claude` API failure, deterministic silent-success behavior in manual `--skill-path` path |
| **Affected Component** | [crates/daemon/src/main.rs](/Users/alinaaitova/Desktop/Testproject1/crates/daemon/src/main.rs:3515), [crates/daemon/src/agents/codex.rs](/Users/alinaaitova/Desktop/Testproject1/crates/daemon/src/agents/codex.rs:197), nested agent runtime |
| **Working Analogue** | Direct `agent-browser` navigation/login/interest-registration/RFx drilldown works; targeted SQLite persistence tests pass; daemon `--dry-run` bootstraps and seeds ProContract sources |
| **Scope** | ProContract pipeline code is implemented and validated at compile/persistence level, but end-to-end daemon-driven browser execution is blocked by agent runtime failures and one daemon manual-run path that reports success even when no `result.json` exists |
| **Environment** | Darwin 24.5.0 arm64, `claude` 2.1.104, `codex-cli` 0.120.0, `agent-browser` 0.25.3, repo commit `ae5047c61564125481469f4ce6d74e4b0b227a28` |
| **Git State** | branch `main`, dirty due unrelated concurrent doc/runtime artifacts plus this manifest file |

---

## REPRODUCTION PROTOCOL

> Follow these steps EXACTLY. You should be able to reproduce the blockers in <5 minutes.

### Prerequisites

- Repo at `/Users/alinaaitova/Desktop/Testproject1`
- `claude`, `codex`, and `agent-browser` installed on PATH
- Current code at commit `ae5047c61564125481469f4ce6d74e4b0b227a28`
- Use local project state paths: `state/`, `runtime/`, `artifacts/`

### Step 1: Verify daemon bootstrap and source seeding still work

```bash
cargo run --bin daemon -- --dry-run --state-root state --runtime-dir runtime --artifact-root artifacts
sqlite3 state/state.db "SELECT id, kind, enabled FROM sources WHERE id IN ('procontract','procontract-interest','procontract-enrich') ORDER BY id;"
```

**EXPECTED**: daemon boots cleanly and all three ProContract source rows exist.

**ACTUAL**:

```text
2026-04-13T15:39:20.262342Z  INFO daemon bootstrap ready socket_path=runtime/daemon.sock state_db=state/state.db artifact_root=artifacts
2026-04-13T15:39:20.262398Z  INFO dry-run bootstrap completed
procontract|procurement|1
procontract-enrich|procurement|1
procontract-interest|procurement|1
```

### Step 2: Reproduce the deterministic `codex` adapter failure

```bash
cargo run --bin daemon -- --skill-path skills/procurement-ingestion/procontract-discover --job-parameters-json '{"source_id":"procontract","limit":1}' --agent-cli codex --state-root state --runtime-dir runtime --artifact-root artifacts
```

**EXPECTED**: daemon probes the selected agent CLI, runs the discovery skill, and either persists at least one notice or fails with a procurement-skill/result-contract error.

**ACTUAL**:

```text
2026-04-13T15:41:21.570566Z  INFO daemon bootstrap ready socket_path=runtime/daemon.sock state_db=state/state.db artifact_root=artifacts
Error: AGENT_INTERNAL_ERROR: `codex` ["probe", "--json"] exited unsuccessfully: error: unexpected argument '--json' found

  tip: to pass '--json' as a value, use '-- --json'

Usage: codex [OPTIONS] [PROMPT]
       codex [OPTIONS] <COMMAND> [ARGS]
```

### Step 3: Reproduce the nested `claude` runtime failure plus silent-success manual-run bug

```bash
cargo run --bin daemon -- --skill-path skills/procurement-ingestion/procontract-discover --job-parameters-json '{"source_id":"procontract","limit":1}' --state-root state --runtime-dir runtime --artifact-root artifacts
sqlite3 -readonly state/state.db "SELECT id, status, completed_at FROM workflow_jobs ORDER BY created_at DESC LIMIT 1;"
sqlite3 -readonly state/state.db "SELECT COUNT(*) FROM contract_notices WHERE source_id = 'procontract';"
sqlite3 -readonly state/state.db "SELECT id, job_id, terminal_output_path, run_manifest_path, status, exit_code FROM agent_session_logs ORDER BY created_at DESC LIMIT 1;"
```

**EXPECTED**: if the nested agent fails before writing `result.json`, the manual run should be marked failed; otherwise at least one notice should persist.

**ACTUAL**:

```text
Skill run completed: job_id=job-02f7f661b2149f54f12e37518b0a2b8714eba60b3f7f1aba5f02b135bb0f7743 skill_id=procurement-ingestion/procontract-discover session_id=session-manual-0baf04372549cb253576f677b7526a82c2a0b000440273c051be686895486b9f adapter=claude
job-02f7f661b2149f54f12e37518b0a2b8714eba60b3f7f1aba5f02b135bb0f7743|completed|2026-04-13T15:40:18.063444+00:00
0
session-manual-0baf04372549cb253576f677b7526a82c2a0b000440273c051be686895486b9f|job-02f7f661b2149f54f12e37518b0a2b8714eba60b3f7f1aba5f02b135bb0f7743|artifacts/sessions/2026/04/session-manual-0baf04372549cb253576f677b7526a82c2a0b000440273c051be686895486b9f/stream.index.jsonl||completed|1
```

### Step 4: Verify the nested `claude` PTY actually failed upstream

```bash
zstd -dc artifacts/sessions/2026/04/session-manual-0baf04372549cb253576f677b7526a82c2a0b000440273c051be686895486b9f/stream/000001.pty.zst
find artifacts/sessions/2026/04/session-manual-0baf04372549cb253576f677b7526a82c2a0b000440273c051be686895486b9f -maxdepth 3 -type f | sort
```

**EXPECTED**: either a valid `result.json` exists in the session workdir or the daemon explicitly fails the run.

**ACTUAL**:

```text
API Error: 500 {"type":"error","error":{"type":"api_error","message":"Internal server error"},"request_id":"req_011Ca22FxfAW7Cz72cmS1fvL"}
artifacts/sessions/2026/04/session-manual-0baf04372549cb253576f677b7526a82c2a0b000440273c051be686895486b9f/stream.index.jsonl
artifacts/sessions/2026/04/session-manual-0baf04372549cb253576f677b7526a82c2a0b000440273c051be686895486b9f/stream/000001.pty.zst
artifacts/sessions/2026/04/session-manual-0baf04372549cb253576f677b7526a82c2a0b000440273c051be686895486b9f/workdir/storage.sqlite3
artifacts/sessions/2026/04/session-manual-0baf04372549cb253576f677b7526a82c2a0b000440273c051be686895486b9f/workdir/storage.sqlite3-shm
artifacts/sessions/2026/04/session-manual-0baf04372549cb253576f677b7526a82c2a0b000440273c051be686895486b9f/workdir/storage.sqlite3-wal
artifacts/sessions/2026/04/session-manual-0baf04372549cb253576f677b7526a82c2a0b000440273c051be686895486b9f/workdir/storage.sqlite3.bak
```

### Verification — Confirm Defect Present

```bash
sqlite3 -readonly state/state.db "SELECT COUNT(*) FROM contract_notices WHERE source_id = 'procontract';"
```

**OUTPUT**:

```text
0
```

### Control — Working Analogues

```bash
cargo test --bin daemon tests::persist_interest_registrations_writes_stage_tracking_rows -- --exact
cargo test --bin daemon tests::persist_tender_enrichments_updates_notice_and_documents -- --exact
```

**OUTPUT**:

```text
test tests::persist_interest_registrations_writes_stage_tracking_rows ... ok
test tests::persist_tender_enrichments_updates_notice_and_documents ... ok
```

Direct portal analogue outside the daemon also works:

```bash
agent-browser open "https://procontract.due-north.com/Advert?advertId=3ba4e6b8-9cef-f011-813c-005056b64545"
```

**OUTPUT excerpt**:

```text
✓ 107_26 Inspection and Maintenance of Indoor Gymnasium, Fitness and Sport Equipment, and Outdoor Scho...
  https://procontract.due-north.com/Supplier/Advert/View?advertId=3ba4e6b8-9cef-f011-813c-005056b64545&...
```

---

## EVIDENCE BUNDLE

### E0 — Environment Evidence

**E0.1**: Git and OS snapshot

```bash
git rev-parse HEAD
git status -sb
date -u +'%Y-%m-%dT%H:%M:%SZ'
uname -a
```

```text
ae5047c61564125481469f4ce6d74e4b0b227a28
## main...origin/main
 M docs/harmonymaxer/.run
 ...
2026-04-13T16:36:07Z
Darwin Mac.fritz.box 24.5.0 ... arm64
```

**E0.2**: CLI versions

```bash
which claude && claude --version
which codex && codex --version
agent-browser --version
```

```text
/Users/alinaaitova/.local/bin/claude
2.1.104 (Claude Code)
/Users/alinaaitova/.nvm/versions/node/v20.19.5/bin/codex
codex-cli 0.120.0
agent-browser 0.25.3
```

**E0.3**: Source rows present in the real SQLite DB

```bash
sqlite3 state/state.db "SELECT id, kind, enabled FROM sources WHERE id IN ('procontract','procontract-interest','procontract-enrich') ORDER BY id;"
```

```text
procontract|procurement|1
procontract-enrich|procurement|1
procontract-interest|procurement|1
```

### E2 — Reproduction Evidence

**E2.1**: `codex` adapter mismatch is deterministic

```bash
cargo run --bin daemon -- --skill-path skills/procurement-ingestion/procontract-discover --job-parameters-json '{"source_id":"procontract","limit":1}' --agent-cli codex --state-root state --runtime-dir runtime --artifact-root artifacts
```

```text
Error: AGENT_INTERNAL_ERROR: `codex` ["probe", "--json"] exited unsuccessfully: error: unexpected argument '--json' found
```

**E2.2**: Manual `claude` run prints success even though nothing persisted

```bash
cargo run --bin daemon -- --skill-path skills/procurement-ingestion/procontract-discover --job-parameters-json '{"source_id":"procontract","limit":1}' --state-root state --runtime-dir runtime --artifact-root artifacts
sqlite3 -readonly state/state.db "SELECT COUNT(*) FROM contract_notices WHERE source_id = 'procontract';"
```

```text
Skill run completed: job_id=job-02f7f661... adapter=claude
0
```

**E2.3**: Latest workflow row is marked completed, not failed

```bash
sqlite3 -readonly state/state.db "SELECT id, status, output_payload, last_error FROM workflow_jobs ORDER BY created_at DESC LIMIT 1;"
```

```text
job-02f7f661b2149f54f12e37518b0a2b8714eba60b3f7f1aba5f02b135bb0f7743|completed|{"adapter_id":"claude","execution_mode":"pty","job_id":"job-02f7f661...","session_id":"session-manual-0baf...","skill_id":"procurement-ingestion/procontract-discover"}|
```

**E2.4**: PTY capture shows upstream nested-agent failure

```bash
zstd -dc artifacts/sessions/2026/04/session-manual-0baf04372549cb253576f677b7526a82c2a0b000440273c051be686895486b9f/stream/000001.pty.zst
```

```text
API Error: 500 {"type":"error","error":{"type":"api_error","message":"Internal server error"},"request_id":"req_011Ca22FxfAW7Cz72cmS1fvL"}
```

**E2.5**: No result contract artifact exists in the session workdir

```bash
find artifacts/sessions/2026/04/session-manual-0baf04372549cb253576f677b7526a82c2a0b000440273c051be686895486b9f -maxdepth 3 -type f | sort
```

```text
.../stream.index.jsonl
.../stream/000001.pty.zst
.../workdir/storage.sqlite3
.../workdir/storage.sqlite3-shm
.../workdir/storage.sqlite3-wal
.../workdir/storage.sqlite3.bak
```

**E2.6**: Direct portal/browser flow itself is viable

```bash
agent-browser open "https://procontract.due-north.com/Advert?advertId=3ba4e6b8-9cef-f011-813c-005056b64545"
agent-browser snapshot -c
```

```text
Heading: 107_26 Inspection and Maintenance of Indoor Gymnasium...
Expression of interest window: From 12/01/2026 09:30 to 15/05/2026 12:00
Register interest in this opportunity
Attachments: Brief Guide to Access ITT Events...pdf
```

### E3 — Forensic Evidence

**E3.1**: `CodexAdapter::probe()` hardcodes `probe --json`

[crates/daemon/src/agents/codex.rs](/Users/alinaaitova/Desktop/Testproject1/crates/daemon/src/agents/codex.rs:197)

```rust
let version_stdout = self.run_probe_command(ctx, &["--version"])?;
let cli_version = Self::parse_version(&version_stdout)?;
let probe_stdout = self.run_probe_command(ctx, &["probe", "--json"])?;
```

**E3.2**: `ClaudeAdapter::probe()` already has a fallback when `probe --json` is unavailable

[crates/daemon/src/agents/claude.rs](/Users/alinaaitova/Desktop/Testproject1/crates/daemon/src/agents/claude.rs:215)

```rust
let envelope = match self.run_probe_command(ctx, &["probe", "--json"]) {
    Ok(probe_stdout) => serde_json::from_slice::<ProbeEnvelope>(&probe_stdout)?,
    Err(_) => ProbeEnvelope {
        auth_healthy: true,
        structured_output: true,
        pty_viable: true,
        browser_tools: true,
    },
};
```

**E3.3**: `run_bound_skill()` only persists procurement output if `result_path` exists, but still completes the job unconditionally

[crates/daemon/src/main.rs](/Users/alinaaitova/Desktop/Testproject1/crates/daemon/src/main.rs:3515)

```rust
if skill.metadata.skill_id.starts_with("procurement-ingestion/")
    && let Some(ref result_path) = executed.result_path
{
    ...
}

runtime.state_db.with_write_transaction(|tx| {
    tx.execute("UPDATE workflow_jobs SET status = 'completed', ...")
})?;
```

**E3.4**: PTY result discovery depends on either a `FinalResultReady` event or `workdir/result.json`

[crates/daemon/src/main.rs](/Users/alinaaitova/Desktop/Testproject1/crates/daemon/src/main.rs:4836)

```rust
let result_path = events
    .iter()
    .find_map(|event| match event {
        ControlEvent::FinalResultReady { path } => Some(path.clone()),
        _ => None,
    })
    .or_else(|| {
        let candidate = launch_spec.cwd.join("result.json");
        if candidate.exists() { Some(candidate) } else { None }
    });
```

**E3.5**: The stricter ProContract live loop path already requires `result_path`

[crates/daemon/src/main.rs](/Users/alinaaitova/Desktop/Testproject1/crates/daemon/src/main.rs:5377)

```rust
let result_path = executed
    .result_path
    .as_ref()
    .ok_or_else(|| anyhow!("procontract discovery skill did not produce a result contract"))?;
```

**E3.6**: ProContract code and skill surfaces are already implemented

- Discovery loop: [main.rs](/Users/alinaaitova/Desktop/Testproject1/crates/daemon/src/main.rs:5377)
- Interest loop: [main.rs](/Users/alinaaitova/Desktop/Testproject1/crates/daemon/src/main.rs:5477)
- Enrichment loop: [main.rs](/Users/alinaaitova/Desktop/Testproject1/crates/daemon/src/main.rs:5628)
- Custom persistence: [main.rs](/Users/alinaaitova/Desktop/Testproject1/crates/daemon/src/main.rs:3225), [main.rs](/Users/alinaaitova/Desktop/Testproject1/crates/daemon/src/main.rs:3268)
- Source registry entries: [registry.rs](/Users/alinaaitova/Desktop/Testproject1/crates/daemon/src/sources/registry.rs:613)
- Source seeding: [sqlite.rs](/Users/alinaaitova/Desktop/Testproject1/crates/daemon/src/db/sqlite.rs:432)

---

## CODEPATH COMPARISON

### Working Path: Strict live-loop ingestion

**Entry point**: [run_procontract_discover_live_loop()](/Users/alinaaitova/Desktop/Testproject1/crates/daemon/src/main.rs:5377)

**Flow**:
1. Load skill and assemble `job_parameters`
2. Run `execute_skill_session()`
3. Require `executed.result_path`
4. Parse result JSON
5. Persist through SQLite

**Impact**: missing `result.json` is treated as an error, not as success.

### Broken Path: Manual `--skill-path` validation path

**Entry point**: [run_bound_skill()](/Users/alinaaitova/Desktop/Testproject1/crates/daemon/src/main.rs:3515)

**Flow**:
1. Load skill and run `execute_skill_session()`
2. Only persist if `executed.result_path` exists
3. No explicit failure branch when `result_path` is absent
4. Always mark `workflow_jobs.status = 'completed'`

**Impact**: nested agent failure with no result contract becomes silent success.

### Divergence Points

| # | Working Path | Broken Path | Potential Impact |
|---|-------------|-------------|-----------------|
| 1 | Requires `result_path` and errors if absent | Ignores absent `result_path` for procurement manual runs | False-positive success state |
| 2 | Designed for `sync_source` ingest validation | Used for phase-1 manual skill validation in the handoff | Masks real runtime failure during developer testing |
| 3 | ProContract live loop would fail loudly | Manual `--skill-path` path returned success with `0` notices | Misleads operator and next agent |
| 4 | Claude probe has compatibility fallback | Codex probe has no fallback and hard-fails | Deterministic inability to test with local `codex-cli` |

---

## ROOT CAUSE HYPOTHESES

### H1: `run_bound_skill()` has a silent-success bug for missing procurement result contracts — LIKELIHOOD: HIGH

**Description**: manual `--skill-path` runs do not fail when a procurement skill exits without `result.json`. This masked the upstream nested-`claude` failure and made the run appear successful.

**Supporting evidence**: E2.2, E2.3, E3.3, E3.4  
**Contradicting evidence**: none

**Verification steps**:
1. Patch `run_bound_skill()` to fail when `skill_id.starts_with("procurement-ingestion/") && executed.result_path.is_none()`
2. Re-run the same `claude` command from Step 3 and confirm the daemon returns a failed job, not a success string

**Fix scope**: [crates/daemon/src/main.rs](/Users/alinaaitova/Desktop/Testproject1/crates/daemon/src/main.rs:3515)

### H2: `CodexAdapter::probe()` is incompatible with installed `codex-cli 0.120.0` — LIKELIHOOD: HIGH

**Description**: the adapter assumes `codex probe --json` exists, but the installed CLI rejects `--json`. This blocks the fallback agent runtime for all ProContract skill validation.

**Supporting evidence**: E2.1, E3.1, E3.2  
**Contradicting evidence**: none

**Verification steps**:
1. Run `codex --help` and inspect supported subcommands/options
2. Replace the probe implementation with either:
   - a supported machine-readable command, or
   - a compatibility fallback like the Claude adapter
3. Re-run Step 2

**Fix scope**: [crates/daemon/src/agents/codex.rs](/Users/alinaaitova/Desktop/Testproject1/crates/daemon/src/agents/codex.rs:197)

### H3: Nested `claude` PTY runs are failing upstream before emitting the daemon contract — LIKELIHOOD: MEDIUM

**Description**: the PTY capture shows an Anthropic-side/internal server error before any `result.json` was written. Even after fixing the manual-run semantics, the actual nested `claude` runtime may still need retry/fallback behavior.

**Supporting evidence**: E2.2, E2.4, E2.5, E3.4  
**Contradicting evidence**: the same environment can run direct `agent-browser` flows, so the portal itself is not the blocker

**Verification steps**:
1. After H1 is fixed, re-run the `claude` command and confirm the failure is surfaced cleanly
2. Inspect whether the error is transient or deterministic across 2-3 attempts
3. Decide whether to add retry/backoff, or switch phase-1 validation to a repaired `codex` path

**Fix scope**: runtime policy around [run_bound_skill()](/Users/alinaaitova/Desktop/Testproject1/crates/daemon/src/main.rs:3515) and possibly [execute_pty_skill_session()](/Users/alinaaitova/Desktop/Testproject1/crates/daemon/src/main.rs:4836)

---

## INVESTIGATION ROADMAP

### Step 1: Fix the silent-success manual-run path

Make procurement manual runs fail if `executed.result_path` is absent or if `executed.exit_code != 0`.

**If confirmed** → re-run Step 3 and ensure the `claude` failure becomes explicit  
**If rejected** → inspect whether `result_path` actually exists but parsing/persistence is being skipped elsewhere

### Step 2: Make the `codex` adapter compatible with the installed CLI

Patch [crates/daemon/src/agents/codex.rs](/Users/alinaaitova/Desktop/Testproject1/crates/daemon/src/agents/codex.rs:197) to stop assuming `probe --json`.

**If confirmed** → re-run Step 2 and use `codex` as the local validation runner  
**If rejected** → inspect whether `codex-cli 0.120.0` needs a different command family entirely

### Step 3: Re-run discovery through the daemon with a healthy agent runtime

Use either repaired `codex` or recovered `claude`.

**Target command**:

```bash
cargo run --bin daemon -- --skill-path skills/procurement-ingestion/procontract-discover --job-parameters-json '{"source_id":"procontract","limit":1}' --agent-cli codex --state-root state --runtime-dir runtime --artifact-root artifacts
```

**Success condition** → at least one ProContract notice persists or a genuine skill-level error is surfaced  
**Failure condition** → inspect result artifact path and PTY capture again

### Step 4: Move from manual validation to real ingest loops

After manual discovery succeeds:

```bash
cargo run --bin daemon -- --ingest procontract --state-root state --runtime-dir runtime --artifact-root artifacts
cargo run --bin daemon -- --ingest procontract-interest --state-root state --runtime-dir runtime --artifact-root artifacts
cargo run --bin daemon -- --ingest procontract-enrich --state-root state --runtime-dir runtime --artifact-root artifacts
```

**If confirmed** → validate DB state across notices, stage tracking, raw documents, and tender document sets  
**If rejected** → create a second manifest scoped to the first ingest-stage failure

---

## FIX CONSTRAINTS

1. Do not revert or disturb unrelated concurrent doc/runtime changes already present in the worktree.
2. Do not remove the ProContract source IDs or skill packages that were just added.
3. Preserve the stricter live-loop behavior that requires a valid procurement `result.json`.
4. Keep `interest_registration` and `tender_enrichment` SQLite persistence semantics intact unless evidence shows a schema mismatch.
5. Do not weaken failure visibility: missing result contracts must become more explicit, not less.

---

## ACCEPTANCE CRITERIA

### Functional

- [ ] `cargo run --bin daemon -- --skill-path skills/procurement-ingestion/procontract-discover ...` no longer reports success when no `result.json` exists
- [ ] `cargo run --bin daemon -- --skill-path ... --agent-cli codex ...` no longer fails at the adapter probe step
- [ ] At least one ProContract notice persists into `contract_notices` via the daemon path, or a real procurement error is surfaced

### Evidence-Based

- [ ] Re-running `sqlite3 -readonly state/state.db "SELECT COUNT(*) FROM contract_notices WHERE source_id = 'procontract';"` returns `> 0` after a successful run
- [ ] If the nested agent fails, the workflow row is `failed`, not `completed`
- [ ] The session workdir contains `result.json` on success, or the daemon records a clear failure on absence

### Regression

- [ ] `cargo check --all-targets`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test --bin daemon tests::persist_interest_registrations_writes_stage_tracking_rows -- --exact`
- [ ] `cargo test --bin daemon tests::persist_tender_enrichments_updates_notice_and_documents -- --exact`

### Test

- [ ] Add a regression test for the missing-`result.json` manual procurement path in `run_bound_skill()`
- [ ] Add a regression test or probe compatibility shim test for the current `codex-cli` behavior

---

## FILE MAP

| File | Role | Lines Referenced |
|------|------|-----------------|
| [crates/daemon/src/main.rs](/Users/alinaaitova/Desktop/Testproject1/crates/daemon/src/main.rs:3515) | Manual skill execution path with silent-success bug | 3515-3605 |
| [crates/daemon/src/main.rs](/Users/alinaaitova/Desktop/Testproject1/crates/daemon/src/main.rs:4836) | PTY result contract discovery | 4836-4850 |
| [crates/daemon/src/main.rs](/Users/alinaaitova/Desktop/Testproject1/crates/daemon/src/main.rs:5377) | ProContract discovery live loop | 5377-5472 |
| [crates/daemon/src/main.rs](/Users/alinaaitova/Desktop/Testproject1/crates/daemon/src/main.rs:5477) | ProContract interest-registration live loop | 5477-5624 |
| [crates/daemon/src/main.rs](/Users/alinaaitova/Desktop/Testproject1/crates/daemon/src/main.rs:5628) | ProContract enrichment live loop | 5628-5696 |
| [crates/daemon/src/main.rs](/Users/alinaaitova/Desktop/Testproject1/crates/daemon/src/main.rs:3225) | Custom registration persistence | 3225-3265 |
| [crates/daemon/src/main.rs](/Users/alinaaitova/Desktop/Testproject1/crates/daemon/src/main.rs:3268) | Custom enrichment persistence | 3268-3466 |
| [crates/daemon/src/agents/codex.rs](/Users/alinaaitova/Desktop/Testproject1/crates/daemon/src/agents/codex.rs:197) | Incompatible probe command | 197-202 |
| [crates/daemon/src/agents/claude.rs](/Users/alinaaitova/Desktop/Testproject1/crates/daemon/src/agents/claude.rs:215) | Probe fallback analogue | 215-230 |
| [crates/daemon/src/sources/registry.rs](/Users/alinaaitova/Desktop/Testproject1/crates/daemon/src/sources/registry.rs:613) | ProContract source registry entries | 613-675 |
| [crates/daemon/src/db/sqlite.rs](/Users/alinaaitova/Desktop/Testproject1/crates/daemon/src/db/sqlite.rs:432) | Source seeding for real SQLite DB | 432-453 |
| [skills/procurement-ingestion/procontract-discover/SKILL.md](/Users/alinaaitova/Desktop/Testproject1/skills/procurement-ingestion/procontract-discover/SKILL.md:1) | Discovery playbook | 1-220 |
| [skills/procurement-ingestion/procontract-register-interest/SKILL.md](/Users/alinaaitova/Desktop/Testproject1/skills/procurement-ingestion/procontract-register-interest/SKILL.md:1) | Side-effecting registration playbook | 1-211 |
| [skills/procurement-ingestion/procontract-enrich-tender/SKILL.md](/Users/alinaaitova/Desktop/Testproject1/skills/procurement-ingestion/procontract-enrich-tender/SKILL.md:1) | RFx enrichment playbook | 1-241 |
| `artifacts/sessions/2026/04/session-manual-0baf04372549cb253576f677b7526a82c2a0b000440273c051be686895486b9f/stream/000001.pty.zst` | Forensic PTY capture of nested `claude` failure | whole file |

---

## CURRENT SITUATION SUMMARY

- ProContract discovery, interest registration, and enrichment skills exist and are committed on `main`.
- Registry/seeding/ingest arms/custom persistence are implemented.
- Compile, clippy, and targeted persistence tests pass.
- Direct portal exploration proved that listing, login, interest registration, dashboard drilldown, and RFx document listing are viable.
- Full daemon-driven browser execution is currently blocked by:
  1. deterministic `codex` probe incompatibility
  2. upstream nested `claude` API failure
  3. manual procurement runs being marked successful even when the nested agent never produces `result.json`
