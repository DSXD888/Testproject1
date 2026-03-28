# L1 Daemon Core — Bead Audit (Spec Drift Detection)

## First Action

Invoke the `/bead-plan-audit` skill.

---

## Your Mission

You are performing a **dedicated, systematic audit** of all beads for Layer 1 Daemon Core. This is the LAST GATE before execution. Your job is to verify **zero spec drift** — every spec requirement is covered, every TDD test is assigned, no silent scope cuts, and no content was weakened during the create/optimize cycle.

This is the most critical quality gate in the pipeline. If a requirement is missing from a bead, it will NOT be implemented.

---

## Source Documents

### Beads (TARGET — audit EVERY bead)

```bash
br list --json           # Full inventory — read ALL beads
br show <id> --json      # Individual bead detail
bv --robot-triage        # Dependency health check
bv --robot-insights      # Graph analysis (cycles, critical path)
```

Read EVERY bead. Not a sample — every single one.

### Verify Against

```
docs/strategy/layers/layer-1-thin-local-runner.md                   — Spec (SOT #1) — Every L1-FR-* MUST have a bead
docs/pipeline/l1-daemon-core/artifacts/l1-daemon-core-tdd.md        — TDD (SOT #2) — Every test ID MUST be assigned
docs/pipeline/l1-daemon-core/artifacts/l1-daemon-core-strategy.md   — Strategy (SOT #3) — Phase ordering MUST match
```

### Codebase

```
docs/pipeline/l1-daemon-core/manifest.yaml                          — All expected file paths
AGENTS.md                                                            — Constraints (forbid unsafe, no file deletion, etc.)
```

---

## Audit Methodology

The `/bead-plan-audit` skill handles the systematic methodology. Add these Layer-1-specific checks:

### Feature-Specific Checks

1. **Process supervision coverage:** Every one of the 7 session lifecycle phases (created → spawned → active → terminating → exited → reaped → archived) must appear in at least one bead's acceptance criteria.

2. **Error type coverage:** All 12 error types from §1.9 + 5 source/skill errors from §4.3 + 2 host errors = 19 total. Each must have:
   - A bead that implements the detection logic
   - A bead that implements the error manifest generation
   - A TDD test ID that verifies the error path

3. **Two-lane adapter coverage:** Both `OutputFrame` and `ControlEvent` lanes must be tested for BOTH adapters (Codex and Claude). That's 4 combinations — each must appear in bead acceptance criteria.

4. **Chunked capture completeness:** The artifact store beads must include:
   - Chunk boundary behavior (exactly 32KB, less than 32KB, empty session)
   - `stream.index.jsonl` generation
   - SHA-256 hash verification
   - EIO handling as expected hangup
   - Crash recovery (partial chunk cleanup)

5. **IPC completeness:** The IPC beads must include:
   - `daemon.info.json` atomic write
   - `daemon.lock` exclusive acquisition
   - Stale socket detection
   - `hello`/`welcome` handshake with all fields from §2.8
   - `lastSeenInstanceId` mismatch detection (daemon restart)

6. **Stage tracking correctness:** Beads that involve the Job Runner must correctly handle:
   - Set-replacement commit semantics (atomic all-or-nothing per scope)
   - Additive commit semantics (independent per scope unit)
   - `stage_run_id` generation and scoping
   - `process_stage_status` and `process_stage_heads` atomic update

7. **Run manifest dual-authorship:** The run manifest beads must cover:
   - Agent narrative sections (Summary, Steps Taken, Decisions Made, Warnings, Evidence)
   - Daemon evidence appendix (all fields from §4.2)
   - `agent_claimed_*` vs `daemon_observed_*` naming
   - Fallback machine-authored manifest when agent fails
   - SHA-256 hash stored in `agent_session_logs`

---

## Follow-Up Pass

> After the audit is complete and all fixes applied:

1. Re-run `bv --robot-insights | jq '.Cycles'` — MUST be empty
2. Re-run the coverage matrix. Count `L1-FR-*` IDs in the spec, count beads. Ratio should be > 1.0 (every requirement in at least one bead, some in multiple).
3. Verify bead count is still in the 15-25 range (audit fixes shouldn't create bead explosion)
4. Run `bv --robot-triage` one final time — the top recommendation should be the foundation bead with zero blockers
