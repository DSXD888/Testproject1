# L1 Daemon Core — Optimize TDD (Fresh-Eyes Audit)

## Your Mission

You are performing a **comprehensive fresh-eyes audit** of the TDD specification for Layer 1. The TDD was created in Step 03 from the validated L1 spec. Your job is to find coverage gaps, unrealistic test strategies, and drift from the spec.

**You are reading, analyzing, and improving an existing document — not writing code.**

---

## What To Read (Do This First)

### The Document to Audit (TARGET)

```
docs/pipeline/l1-daemon-core/artifacts/l1-daemon-core-tdd.md       — TDD (created in previous step) — READ FULLY
docs/pipeline/l1-daemon-core/artifacts/l1-daemon-core-bootstrap.md  — Bootstrap (created in previous step) — READ FULLY
```

### Verify Against (SOURCE OF TRUTH)

```
docs/strategy/layers/layer-1-thin-local-runner.md   — Spec (SOT #1) — The source of truth for WHAT
docs/strategy/layer-strategy.md                      — §11 Testing Evolution — testing targets for L1
```

### Codebase (GROUND TRUTH)

```
AGENTS.md                                            — Toolchain conventions, compiler check requirements
docs/strategy/layers/layer-0-gold-set-evaluation.md  — §5 Evaluation Harness — test infrastructure pattern
```

---

## Think Independently

### 1. Coverage Matrix Completeness

Walk every `L1-FR-*` requirement ID in the spec. For each one, verify:
- At least one test ID is mapped to it in the TDD
- The test tier (unit/integration/E2E) is appropriate for the requirement
- The test actually verifies the requirement (not just touching the same code path)

Flag any requirements with ZERO test coverage.

### 2. Test Realism for Process Supervision

The spec requires real process supervision — no mocking. Verify:
- PTY tests actually create PTY pairs and spawn real child processes
- Reaper tests actually use `SIGCHLD` and `waitpid`, not simulated signals
- Process-group tests actually use `setpgid`/`killpg`, not mocked process trees
- Are test helper binaries specified? (e.g., a `test-agent` that sleeps, one that forks, one that ignores signals)
- Is there a strategy for test isolation? (Process tests can interfere with each other)

### 3. SQLite Test Strategy

Verify the SQLite testing approach:
- Are tests using real SQLite (in-memory or temp-file), not a mock layer?
- Is WAL mode tested? (WAL has different locking behavior than default journal)
- Is the migration runner tested? (Does `0003_sqlite_runtime.sql` apply cleanly on top of L0's `0001_sqlite_state_initial.sql`?)
- Is `commit_seq` monotonicity tested under concurrent readers?
- Is the single-writer enforcement tested? (What happens if two threads try to write?)

### 4. Adapter Test Coverage

The spec defines two adapters (Codex, Claude) with different parsing strategies. Verify:
- Both adapters have independent test suites (not just one adapter tested)
- Capability probing is tested (CLI present, CLI absent, unknown version → canary mode)
- Mode selection matrix is tested (all 6 rows from §2.3)
- JSONL parsing (Codex) and JSON/stream-json parsing (Claude) have separate parser tests
- Error cases: malformed output, unexpected exit, no output, partial output

### 5. E2E Test Feasibility

The spec requires "10 consecutive batch runs, ≥90% success rate" (Experiment 4). Verify:
- Is this E2E test feasible in CI? (It requires live FTS access)
- Is there a fallback for CI without network access? (Captured evidence bundles?)
- Is the test deterministic or flaky-prone? (Network-dependent tests need retry strategy)
- What's the expected test duration? (10 batch runs could take 30+ minutes)
- Is there a CI tier assignment? (This should be nightly, not per-commit)

### 6. Bootstrap Document Quality

Verify the bootstrap document:
- Does it include workspace `Cargo.toml` with the daemon crate and test crate?
- Does it include `rust-toolchain.toml` for Rust 2024 nightly?
- Does it include dev-dependencies: `tokio-test`, `tempfile`, `assert_cmd`, etc.?
- Does it include a test helper binary specification?
- Does it include CI configuration (GitHub Actions or equivalent)?
- Does it specify the `PRAGMA` initialization for test SQLite instances?

### 7. Error Manifest Test Depth

Verify error manifest testing:
- Are all 12 error types tested with dedicated negative-path tests?
- Do error manifest tests verify the Markdown output structure (not just "an error occurred")?
- Is the error manifest resolution workflow tested? (`resolved = false` → `resolved = true`)
- Is the error manifest Markdown rendering engine tested with all context fields?

### 8. Run Manifest Test Coverage

The run manifest system is critical for downstream validation (L3+). Verify:
- Is the agent_claimed vs daemon_observed naming convention tested?
- Is the fallback machine-authored manifest tested? (Agent fails to narrate → daemon produces minimum manifest)
- Is the YAML frontmatter parsing tested?
- Is the daemon evidence appendix generation tested?
- Is the SHA-256 hash verification tested? (`run_manifest_hash` in `agent_session_logs`)

---

## Drift Checklist

- [ ] Every `L1-FR-*` requirement has at least one test ID
- [ ] No test IDs reference requirements that don't exist in the spec
- [ ] Test tiers are appropriate (unit tests don't need real network; E2E tests do)
- [ ] Test file names match §5.2 (the spec prescribes exact names)
- [ ] Bootstrap document is consistent with AGENTS.md toolchain (Cargo, not npm)
- [ ] No tests assume L3+ features (cron scheduler, resource arbiter, validation enforcement)
- [ ] CI tier assignments are realistic (per-commit tests < 5 min; nightly tests < 30 min)

---

## Follow-Up Pass (Run After Fixes)

> After making your fixes, send this as a follow-up message in the same session.

1. Count test IDs. Is the total between 100-150? If significantly outside this range, recalibrate.
2. Re-read the bootstrap document — does it compile? (Mentally trace `cargo test` from fresh clone)
3. Check for test isolation — can all tests run in parallel with `cargo test`? (SQLite file conflicts, port conflicts, PID conflicts)
4. Verify no "TODO" or "placeholder" markers remain in either document.
