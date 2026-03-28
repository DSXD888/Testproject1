# L2 Desktop MVP — Optimize Spec (Fresh-Eyes Audit)

## Your Mission

You are performing a **comprehensive fresh-eyes audit** of the Layer 2 spec — the execution-grade specification for the Single-Operator Desktop MVP. This is the first user-facing layer in a UK procurement intelligence platform, covering a Tauri v2 desktop app, shared UI components, manifest rendering, coverage metrics, decision capture, and a new Contracts Finder ingestion skill.

**You are reading, analyzing, and improving an existing document — not writing code.**

---

## What To Read (Do This First)

### The Document to Audit (TARGET)
```
docs/strategy/layers/layer-2-single-operator-desktop.md    — Spec (validated in previous step, now has L2-FR-* IDs)
```
Read fully. This is the primary audit target.

### Verify Against (SOURCE OF TRUTH)
```
docs/strategy/layer-strategy.md                            — Cross-layer contracts, L2 summary (lines 490-609)
docs/strategy/cross-reference-index.md                     — Topic lookup for consistency checks
```

### Codebase (GROUND TRUTH)
```
crates/daemon/src/ipc/handshake.rs                         — L1 IPC protocol that L2 DaemonBridge extends
crates/daemon/src/ipc/descriptor.rs                        — daemon.info.json format L2 bootstrap reads
crates/daemon/src/agents/adapter.rs                        — AgentAdapter trait, ControlEvent, OutputFrame
crates/daemon/src/jobs/runner.rs                           — Job Runner that L2 triggers via DaemonBridge
crates/daemon/src/pty/session.rs                           — SessionPhase enum that maps to Channel<T> lifecycle
crates/daemon/src/sessions/reaper.rs                       — CleanupVerdict that appears in lifecycle events
crates/daemon/src/db/sqlite.rs                             — commit_seq / commit_clock that drives commit fence
migrations/0001_sqlite_state_initial.sql                   — L0 schema: all 47 tables, all columns
migrations/0003_sqlite_runtime.sql                         — L1 additions: agent_session_logs, etc.
skills/procurement-ingestion/fts/SKILL.md                  — Existing skill standard (quality bar for CF skill)
```

### Quality Bar (MATCH OR EXCEED)
```
docs/strategy/layers/layer-1-thin-local-runner.md          — L1 spec (138 L1-FR-* IDs, well-structured)
```

---

## Think Independently

Don't just check for obvious errors. Think critically about these 10 areas:

### 1. IPC Protocol Completeness
The spec defines 10 DaemonBridge commands (§1.5). Cross-reference against `crates/daemon/src/ipc/handshake.rs`:
- Does the L1 IPC handler have a command dispatch mechanism that can accommodate L2's 10 new commands?
- Is the `BridgeRequest` type forward-compatible with L1's existing `hello/welcome` protocol?
- Are there missing commands? (e.g., `get_daemon_info` for DaemonStatus page, `list_active_sessions` for session view initial state)

### 2. Query/Mutation Boundary Integrity
The spec separates QueryService (read-only SQLite) from DaemonBridge (mutations via daemon). Verify:
- No QueryService query requires a daemon connection (pure SQLite reads)
- No DaemonBridge command does a direct SQLite read (it forwards to daemon)
- The commit fence model (§1.4) is consistent with L1's `commit_clock` table
- `snapshotSeq` sourcing: spec says "from commit_clock table" — verify this matches L1's `StateDb` implementation

### 3. Streaming Architecture Consistency
Channel<T> streaming (§1.6) must be consistent with L1's session model:
- Do the `SessionFrame` variants map 1:1 to L1's `ControlEvent` + `OutputFrame` enums?
- Is the `seq` field on `SessionFrame` the same as L1's event sequencing?
- The spec says "batch frames every 33-50ms" — is this a frontend concern or a daemon-side concern? Who enforces the batching?

### 4. Bootstrap State Machine Correctness
§1.7 defines 5 runtime modes derived from `ControlPlaneState`. Verify:
- Every possible combination of `storage` × `daemon` × `control` × `load` maps to exactly one UX mode
- No dead states (unreachable combinations)
- The daemon crash recovery sequence (§1.7 steps 1-8) is consistent with L1's session reaper behavior

### 5. Shared Component API Stability
§2.1 defines shared UI components for desktop AND web (Layer 4). Verify:
- Component props use only primitive types and view-model interfaces (no Tauri-specific types)
- `FreshnessBadgeVM` is truly surface-neutral (no daemon/sync assumptions leaked)
- `ErrorSummaryItem.severity` derivation rules (§1.11) are unambiguous for every `ErrorCode` in L1

### 6. Coverage Metric Accuracy
§3.1 and §3.4 define coverage metrics. Verify:
- The `CoverageInput` interface (§3.4) has all fields needed to compute every metric
- `decisionBasisHash` ownership is consistent: spec says daemon-authored (§1.12, §3.2) — is this stated once or does it contradict itself?
- The auto-qualification guard threshold (50% `netQualifiedCoverageBps`) is the same in §3.1 and §4.2

### 7. Decision System Integrity
§3.2-§3.3 define the decision capture system. Verify:
- The `pending_operator_intents` schema in the spec matches the DDL in `migrations/0001_sqlite_state_initial.sql`
- Latest-per-process coalescing (§3.2) doesn't conflict with the CAS payload preparation (§5.2)
- Staleness trigger list (§3.3) is exhaustive — are there pipeline events that change `decisionBasisHash` but aren't listed?

### 8. Test Coverage Completeness
§4.2 has 16 DoD criteria, §4.3 has 11 test files, §4.4 has 15 scenarios. Verify:
- Every DoD criterion maps to at least one test file
- Every test scenario maps to at least one DoD criterion
- No DoD criterion is untested
- The testing tier assignments (1/2/3) are consistent with §4.1's tier definitions
- Playwright tests against `localhost:5173` cover everything claimed (the dev bridge must support all tested operations)

### 9. Security Boundary Enforcement
§2.3 rule 7 says agent-authored content is untrusted input. Verify:
- Every component that renders agent content has explicit sanitization requirements
- The spec covers: ManifestViewer (Markdown), ErrorSummaryList (error messages), SessionViewer (ANSI escapes)
- Are there other surfaces where agent content appears? (e.g., session metadata, error manifest drill-down)

### 10. Cross-Layer Contract Consistency
Verify the spec is consistent with the layer-strategy.md L2 section (lines 490-609):
- Definition of Done matches (layer-strategy has 12 criteria, spec §4.2 has 16 — the spec's 16 should be a superset)
- Components Touched matches §1.9 file listing
- Risks match §4.5

---

## Drift Checklist

- [ ] Every `L2-FR-*` requirement added in the validation step has a testable acceptance criterion
- [ ] No content was silently dropped or weakened from the original spec
- [ ] No assumptions made that aren't in the spec (especially about L0/L1 behavior)
- [ ] The spec is self-contained — an agent can build Layer 2 without reopening the research doc
- [ ] The DaemonBridge command set is complete for all 6 desktop pages
- [ ] The Channel<T> streaming model is complete for all session-related views
- [ ] The coverage metrics are fully derivable from the pipeline data (no hidden dependencies)
- [ ] All 18 desktop files have clear, non-overlapping responsibilities

---

## Follow-Up Pass (Run After Fixes)

After making your fixes, send this as a follow-up message:

> Re-read the fixes you just made. Did they introduce new issues? Specifically:
>
> 1. If you added requirement IDs, are they unique? Run a mental grep for duplicates.
> 2. If you extracted acceptance criteria from prose, did you lose nuance? Compare the new bullet list against the original paragraph.
> 3. If you added error path descriptions, are they consistent with L1's `ErrorCode` enum (29 codes in `crates/daemon/src/errors/manifest.rs`)?
> 4. Count the total `L2-FR-*` IDs. Is the count reasonable for the scope? (Target: 100-180)
> 5. Check the Definition of Done (§4.2) one more time — can each criterion be verified by a test in §4.3?
