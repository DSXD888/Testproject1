# L2 Desktop MVP — Completion Audit

## First Action

Invoke the `/bead-completion-audit` skill.

---

## Your Mission

Deep-read the spec, TDD, and codebase for L2 Desktop MVP. Find EVERY gap between what the spec requires and what's actually implemented. Create fix beads for gaps. Do NOT execute them — just create and scope.

This is a mixed-stack audit — you must verify BOTH the Rust code (Tauri backend, daemon IPC extensions) AND the TypeScript code (React pages, shared packages, Playwright tests).

---

## Source Documents

```
docs/strategy/layers/layer-2-single-operator-desktop.md                — Spec (SOT #1) — Walk EVERY L2-FR-* requirement
docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-tdd.md          — TDD (SOT #2) — Verify EVERY test exists with correct assertions
docs/pipeline/l2-desktop-mvp/artifacts/l2-desktop-mvp-bootstrap.md     — Bootstrap — Verify environment matches
```

### Codebase (THE IMPLEMENTATION — read deeply)

**Rust (Tauri backend + daemon extensions):**
```
apps/desktop/src-tauri/src/main.rs                — Tauri bootstrap
apps/desktop/src-tauri/src/query_service.rs       — Read-only SQLite queries
apps/desktop/src-tauri/src/daemon_bridge.rs       — RPC proxy + Channel<T> relay
crates/daemon/src/ipc/handshake.rs                — IPC protocol (L2 extensions)
crates/daemon/src/main.rs                         — Daemon entry point
```

**TypeScript (React frontend + packages):**
```
apps/desktop/src/App.tsx                          — Root component + routing
apps/desktop/src/pages/ContractList.tsx            — Process list
apps/desktop/src/pages/ContractDetail.tsx          — Opportunity detail
apps/desktop/src/pages/Sessions.tsx                — Session list
apps/desktop/src/pages/SessionViewer.tsx           — Live terminal
apps/desktop/src/pages/Errors.tsx                  — Error manifests
apps/desktop/src/pages/DaemonStatus.tsx            — Daemon operations
apps/desktop/src/stores/session-registry.ts        — Session store
apps/desktop/src/stores/session-buffer.ts          — Per-session buffer
apps/desktop/src/hooks/use-daemon.ts               — Session hooks
apps/desktop/src/hooks/use-control-plane.ts        — Daemon health hook
apps/desktop/src/lib/transport.ts                  — Runtime shim
apps/desktop/vite-plugin-tauri-dev-bridge.ts       — Dev bridge
packages/ui/*.tsx                                  — All 5+1 shared components
packages/manifests/*.ts                            — Manifest renderer + template
packages/intelligence/src/margins/coverage-rollup.ts — Coverage rollup
```

**Skills:**
```
skills/procurement-ingestion/contracts-finder/     — CF skill directory
```

**Tests:**
```
apps/desktop/e2e/*.spec.ts                         — All 7 Playwright specs
crates/daemon/tests/agent_*.rs                     — Agent E2E tests
```

---

## Audit Dimensions

### 1. Requirement Coverage
Walk EVERY `L2-FR-*` requirement from the spec. For each:
- Does implementation code exist that satisfies it?
- Does at least one test exercise it?
- Are the acceptance criteria met in the current code?

### 2. DaemonBridge Command Completeness
Verify all 10 commands from §1.5 are implemented:
- [ ] `trigger_ingestion(source_id)` → returns `MutationAck`
- [ ] `trigger_enrichment(process_id)` → returns `MutationAck`
- [ ] `capture_decision(process_id, disposition, basis)` → returns `MutationAck`
- [ ] `resolve_error(error_manifest_id)` → returns `MutationAck`
- [ ] `stop_session(session_id, mode)` → returns `MutationAck`
- [ ] `send_session_input(session_id, bytes)` → returns `MutationAck`
- [ ] `resize_session(session_id, cols, rows)` → returns `MutationAck`
- [ ] `retry_job(job_id)` → returns `MutationAck`
- [ ] `restart_daemon()` → returns `MutationAck`
- [ ] `rerun_from_error(error_manifest_id)` → returns `MutationAck`

### 3. QueryService Read Model Completeness
Verify all 4 read models from §1.4 are implemented:
- [ ] `list_processes(cursor, limit, filters)` → keyset-paginated with pipeline state
- [ ] `get_process_detail(process_id)` → full detail with notices, lots, candidates, etc.
- [ ] `list_sessions(cursor, limit, filters)` → active and recent sessions
- [ ] `list_errors(cursor, limit, filters)` → unresolved error manifests

### 4. Channel<T> Streaming Completeness
- [ ] All 6 frame types implemented (lifecycle, pty, stdout, stderr, event, gap)
- [ ] Two subscription levels (metadata-only, full stream)
- [ ] Gap recovery (slow consumer → gap frame → artifact fetch → resume)
- [ ] Live→historical handoff (archiveSeq-based)
- [ ] Chunked replay on desktop reconnect

### 5. Bootstrap State Machine Completeness
- [ ] All 5 runtime modes (full control, control limited, browse-only, incompatible, recovery required)
- [ ] Daemon crash recovery (8 steps from §1.7)
- [ ] Startup sequence (9 steps from §1.7)
- [ ] Shutdown behavior (quit UI ≠ kill daemon)

### 6. Shared UI Component Correctness
- [ ] All 5+1 components exist in `packages/ui/`
- [ ] Components use frame + slot composition (not boolean props)
- [ ] Components do NOT import data-fetching hooks or daemon modules
- [ ] Security sanitization implemented (ManifestViewer, ErrorSummaryList, SessionViewer)
- [ ] All monetary values from integer pence, all percentages from basis points

### 7. Decision System Correctness
- [ ] `pending_operator_intents` table written with all 8 metadata fields
- [ ] Latest-per-process coalescing works
- [ ] Staleness detection works (hash comparison)
- [ ] CAS payload fields present for future Layer 4 flush

### 8. Coverage Metrics Correctness
- [ ] `coverage-rollup.ts` computes all 4 metrics correctly
- [ ] Color thresholds match spec (§3.1)
- [ ] Auto-qualification guard at 50% `netQualifiedCoverageBps`

### 9. Contracts Finder Skill Correctness
- [ ] Skill directory matches FTS structure (SKILL.md + schemas + canary + golden)
- [ ] Canary runs produce expected results
- [ ] Rate limiting respected
- [ ] Cursor-based pagination works

### 10. Test Coverage
- [ ] All 7 Playwright specs exist and pass
- [ ] All Tier 2 Tauri tests pass
- [ ] All Tier 3 Agent E2E tests pass
- [ ] Security tests include adversarial fixtures (XSS payloads, ANSI escapes)

---

## Follow-Up Pass

> After creating all fix beads:

1. Re-read the fix beads you created. Are they specific enough?
2. Does each fix bead reference the right spec section and TDD test IDs?
3. Could an agent implement each fix from the bead description alone?
4. Is the acceptance criteria concrete — not "fix this" but "add field X of type Y to struct Z"?
5. How many fix beads did you create? If > 10, were the original beads too coarse?
