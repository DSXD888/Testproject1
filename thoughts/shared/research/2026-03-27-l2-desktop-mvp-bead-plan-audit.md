# L2 Desktop MVP Bead Plan Audit

Date: 2026-03-27
Auditor: `JadeGoose`
Scope: all Layer 2 beads in `br`, including the closed merged bead `l2-11c`

## Source Of Truth

- Spec: `docs/strategy/layers/layer-2-single-operator-desktop.md`
- TDD: `docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-tdd.md`
- Strategy: `docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-strategy.md`
- Code anchors:
  - `crates/daemon/src/ipc/handshake.rs`
  - `crates/daemon/src/agents/adapter.rs`
  - `crates/daemon/src/db/sqlite.rs`
  - `crates/daemon/src/jobs/runner.rs`
  - `migrations/0001_sqlite_state_initial.sql`
  - `skills/procurement-ingestion/fts/`

## Final Coverage Result

- Spec requirements: `118 / 118` assigned to one or more L2 beads.
- TDD IDs: `145 / 145` assigned to one or more L2 beads.
- Shared UI files from spec §2.1: assigned.
- Desktop file list from spec §1.9: assigned.
- Closed bead note: `l2-11c` is intentionally superseded by `l2-11b`, not a live gap.

## Findings Fixed During Audit

### 1. Bootstrap state-machine fidelity was too implicit

Severity: High

Problem:
- The bead set referenced `L2-FR-BOOT.*`, but the concrete five-mode matrix, precedence, startup sequence, single-flight launch rule, crash-recovery path, and explicit `Quit UI` vs `Stop background daemon and quit` split were not spelled out clearly enough for a fresh execution pass.

Fix:
- Updated `l2-07b`, `l2-08a`, and `l2-12c` so the bead text now explicitly names:
  - `full control`
  - `control limited`
  - `browse-only`
  - `incompatible`
  - `recovery required`
- Added explicit startup-sequence ownership, single-flight launch semantics, crash-recovery / `instanceId` invalidation behavior, and native shell action split.

### 2. Deep-link alias resolution had no explicit bead owner

Severity: High

Problem:
- Spec §1.16 requires deep links to resolve through `process_aliases`, but no L2 bead named that responsibility explicitly.

Fix:
- Updated `l2-08b` to own canonical-process resolution through `process_aliases`.
- Updated `l2-09a` to own the page-level deep-link stability contract for `/process/:processId`.

### 3. Session replay and accessibility anchors were missing from the SessionViewer bead

Severity: High

Problem:
- The bead text for `l2-10c` covered streaming broadly, but omitted explicit ownership for:
  - `plain-text transcript` accessibility path
  - no per-line `aria-live`
  - browser-only auth `reveal_profile_path`
  - replay anchored to chunked artifact history / `stream.index.jsonl`

Fix:
- Updated `l2-10c` to add `L2-FR-A11Y.1` explicitly and to call out the missing replay/auth/accessibility anchors in both context and acceptance criteria.

### 4. Shared UI implementation bead did not name its shared test home

Severity: Medium

Problem:
- `l2-06a` owned `TS-UI-002`, `TS-UI-004`, and `TS-UI-005`, but did not explicitly name `packages/ui/tests/components.test.tsx` in its file list.

Fix:
- Updated `l2-06a` to include `packages/ui/tests/components.test.tsx` as the shared test home for those assertions.

### 5. Native diagnostics closure did not explicitly own logs/artifacts affordances

Severity: Medium

Problem:
- `DaemonStatus` restart parity was assigned, but the spec’s desktop-only open-logs/open-artifacts affordances were not stated clearly in the closing native bead.

Fix:
- Updated `l2-12c` to assign those diagnostics affordances explicitly.

## Beads Updated

- `l2-06a`
- `l2-07b`
- `l2-08a`
- `l2-08b`
- `l2-09a`
- `l2-10c`
- `l2-12c`

## High-Risk Check Outcome

- DaemonBridge command coverage: pass. The closed command set is explicitly represented, including `restart_daemon` and `rerun_from_error`.
- Channel streaming completeness: pass after bead clarification. Frame kinds, bounded subscription levels, replay, gap handling, live-to-historical handoff, and browser-only auth intervention are assigned.
- Bootstrap state machine coverage: pass after bead clarification. Five modes, precedence, startup sequence, crash recovery, and quit/stop split are assigned.
- Shared UI completeness: pass. `5 + 1` package/UI surfaces are assigned.
- Security sanitization ownership: pass. Markdown sanitization, dangerous-link stripping, and ANSI/OSC sanitization are assigned explicitly.
- Coverage rollup module: pass. `packages/intelligence/src/margins/coverage-rollup.ts` has its own bead.
- Decision metadata completeness: pass. `pending_operator_intents` metadata is explicitly assigned.
- Accessibility assignment: pass after bead clarification. Session accessibility and non-color-only signaling now have named bead owners.
- Test pairing: pass after bead clarification. No orphan L2 test homes remain.
- Dev bridge bead: pass. `vite-plugin-tauri-dev-bridge.ts` and `src/lib/transport.ts` are assigned.

## Residual Notes

- `l2-11c` remains closed with close reason `Merged into l2-11b during fresh-eyes bead audit`. This is intentional historical state, not live drift.
- The audit found no uncovered `L2-FR-*` or TDD IDs after repairs.
