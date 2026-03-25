# Layer 2 — Single-Operator Desktop MVP

> **Authority:** This document is the execution-grade specification for Layer 2. An implementation agent should be able to build this layer from this document alone without reopening the research document.
>
> **Milestone:** M2
> **Dependencies:** Layer 0 (intelligence pipeline, skills, contracts-schema), Layer 1 (daemon runtime, artifact capture, process supervision)
> **Exit criteria:** Single operator runs system daily for 1 week, making real accept/reject decisions on genuine opportunities.

---

## 1. Overview, Capability & Desktop Architecture

### 1.1 Layer Narrative

Prove repeatable operator value with a manual-trigger desktop sitting on top of the Layer 1 live local pipeline. This is the **first user-facing layer** — a single operator runs the system daily, reviews pipeline output in a desktop control panel, and makes accept/reject/watch decisions. This layer expands live procurement coverage to Contracts Finder, renders operator-facing manifests and coverage metrics, and persists single-operator decisions locally as queued intents without violating remote authority ownership.

**Why this layer exists:** Layer 1 proved runtime reliability. Layer 2 proves the value proposition: can a real operator use this system daily and find genuine commercial opportunities? If the answer is no, later layers (sync, validation, expanded intelligence) are wasted investment.

**If we never build Layer 3:** A single operator can use the system with manual triggering for daily procurement intelligence. No cron, no automation, but the core value loop works.

### 1.2 Capability Delivered

Single operator runs system daily for one week, making real accept/reject decisions. The pipeline surfaces genuine opportunities from live procurement data with screening-grade margin estimates. The desktop control panel provides:

- Notice list view with keyset pagination
- Opportunity detail view with margin bands, product candidates, supplier quotes
- Agent session monitoring with live terminal output
- Decision capture (approve/reject/watch) with staleness tracking
- Coverage metrics display
- Error manifest view

### 1.3 Desktop Architecture

Tauri v2 with a **separate background daemon** — the desktop app is NOT the daemon. The daemon must survive UI close, restart independently, and have its own lifecycle.

```text
React SPA (Vite)
  -> Tauri invoke()          for snapshot queries + mutations
  -> Tauri Channel<T>        for live streams (sessions, jobs)
  -> Desktop Rust bridge
       -> QueryService       (read-only SQLite, page-specific read models, commit fence)
       -> DaemonBridge       (UDS/named-pipe RPC to daemon, handshake, peer auth)
            -> Rust daemon   (writer, jobs, PTY, L1 runtime, commit_seq authority)
```

**Three communication channels:**

| Channel | Transport | Direction | Purpose |
|---------|-----------|-----------|---------|
| **Snapshot reads** | `invoke()` -> QueryService | Frontend -> SQLite (read-only) | List views, detail views, metrics — fast, no network dependency |
| **Mutations/triggers** | `invoke()` -> DaemonBridge -> UDS/pipe -> daemon | Frontend -> Daemon | Trigger ingestion, trigger enrichment, capture decisions |
| **Live streams** | Daemon -> DaemonBridge -> `Channel<T>` -> React | Daemon -> Frontend | Session terminal output, job progress, daemon health |

**Why not pure HTTP on localhost (production):** Port discovery/conflicts, auth/token management, extra attack surface, network semantics for a non-networked relationship. In production, `invoke()` + `Channel<T>` through the Tauri webview is the only transport. A dev-only HTTP fallback exists for browser-based testing — see §1.8.

**Why the Tauri backend is not the daemon:** The daemon is a background scheduler/executor. It must survive UI close, restart independently, and have its own lifecycle. Embedding it makes shutdown, crash recovery, and "app closed but cron still runs" (Layer 3) architecturally unsound.

### 1.4 QueryService Interface

The QueryService opens **read-only** SQLite connections and returns page-specific read models. It enforces: read-only/query-only connections, page-limited queries, no streaming cursors held open, no generic SQL from the frontend, no transcript/blob reads in hot queries.

This is the desktop-side implementation of the snapshot-envelope and commit-fence contracts summarized in [layer-strategy.md](/Users/alinaaitova/Desktop/Testproject1/docs/strategy/layer-strategy.md#L1177) and first defined operationally in [layer-1-thin-local-runner.md](/Users/alinaaitova/Desktop/Testproject1/docs/strategy/layers/layer-1-thin-local-runner.md#L717).

```typescript
type QueryEnvelope<T> = {
  data: T
  snapshotSeq: number    // daemon-authored commit_seq at snapshot time
  snapshotAt: string     // ISO-8601 timestamp of snapshot
}
```

**`QueryEnvelope<T>` carries snapshot data only.** Live daemon health is a separate concern — see `ControlPlaneState` below. Mixing two clock domains (DB snapshot data and live daemon status) in one envelope is an abstraction leak that causes confusing UI behavior.

**Commit fence model (read-your-writes consistency):**

The daemon increments a monotonic `commit_seq` on every successful write transaction. The desktop uses this as a global visibility fence:

1. Operator clicks "Start Ingestion" → DaemonBridge sends command
2. Daemon commits job row, increments `commit_seq`, returns `MutationAck` with `visibleAfterSeq`
3. QueryService reads SQLite directly — each page query runs inside one short read transaction
4. Each `QueryEnvelope<T>` returns the DB snapshot's `snapshotSeq`
5. The UI only treats a mutation as visible when `snapshotSeq >= visibleAfterSeq`

Without this fence, the desktop will show ghost states: mutation succeeded, live stream shows progress, but list/detail page still shows an older DB snapshot.

**`snapshotSeq` source:** Do NOT use `PRAGMA data_version` as the user-visible snapshot clock. SQLite documents `data_version` as meaningful only when comparing values from the same connection at two different points in time. Use `data_version` for QueryService cache invalidation; use `commit_seq` (from the daemon-authored `commit_clock` table) as the global visibility fence.

**SQLite configuration for QueryService:**
```sql
PRAGMA query_only = ON;   -- read-only connection, cannot accidentally write
PRAGMA busy_timeout = 1000; -- short timeout for read contention
```

**`ControlPlaneState` (separate from QueryEnvelope):**

```typescript
type ControlPlaneState = {
  daemonInstanceId: string
  daemonVersion: string
  protocolVersion: number
  schemaEpoch: number
  storage: 'ready' | 'repairing' | 'unreadable' | 'schema_mismatch'
  daemon: 'starting' | 'ready' | 'degraded' | 'unavailable' | 'incompatible'
  control: 'full' | 'limited' | 'read_only'
  load: 'normal' | 'busy' | 'saturated'
  reasons: string[]   // human-readable explanation of current state
}
```

This is populated from the daemon handshake and heartbeat, not from SQLite queries.

**Read model examples:**

| Query | Returns | Page |
|-------|---------|------|
| `list_processes(cursor, limit, filters)` | Keyset-paginated process list with pipeline state, coverage metrics, decision status | List view |
| `get_process_detail(process_id)` | Full process detail: notices, lots, candidates, quotes, costs, manifest, errors | Detail view |
| `list_sessions(cursor, limit, filters)` | Active and recent agent sessions: session_id, skill_path, skill_version, execution_mode, status (lifecycle phase), started_at, duration, cleanup_verdict, error_type, exit_code, checkpoint_count, byte_count | Sessions view |
| `list_errors(cursor, limit, filters)` | Unresolved error manifests: error_manifest_id, process_id, pipeline_stage, error_type, error_message, severity (derived from error_type category: `critical`/`warning`/`info`), skill_path, created_at, resolved, agent_session_id | Errors view |

**SQLite concurrency rule:** SQLite is single-writer (daemon only) but supports many short readers. The QueryService opens read-only connections with short-lived transactions. Long-lived readers are prohibited because they prevent WAL checkpoints from draining. No transcript blobs or large artifacts in hot queries — those load on demand from the artifact store.

### 1.5 DaemonBridge Interface

The DaemonBridge is a thin RPC proxy — it does not hold state or perform writes itself. It forwards commands to the daemon over UDS (macOS/Linux) or named pipe (Windows).

This is the desktop-side implementation of the daemon descriptor, handshake, and command-boundary contracts first defined in [layer-1-thin-local-runner.md](/Users/alinaaitova/Desktop/Testproject1/docs/strategy/layers/layer-1-thin-local-runner.md#L798) and summarized in [layer-strategy.md](/Users/alinaaitova/Desktop/Testproject1/docs/strategy/layer-strategy.md#L1178).

```typescript
type MutationAck = {
  accepted: boolean
  idempotencyKey: string
  durability: 'committed' | 'queued'   // 'committed' = durably in SQLite
  visibleAfterSeq: number              // commit_seq at which mutation is visible
  jobId?: string
  sessionId?: string
  remoteState?: 'committed' | 'queued' | 'conflicted' | 'not_applicable'
  conflictReason?: string
  // L2: remoteState is always 'not_applicable' (no sync yet)
}
```

**Durability rule:** `accepted: true` means "durably committed in SQLite," never "I have it in RAM." The ack is sent only after the write transaction commits.

**Bridge RPC types:**

```typescript
type BridgeRequest =
  | { id: string; kind: 'hello'; desktopVersion: string; supportedProtocolRange: [number, number]; expectedSchemaEpoch: number; lastSeenInstanceId?: string }
  | { id: string; kind: 'command'; method: string; params: Record<string, unknown>; idempotencyKey: string }
  | { id: string; kind: 'subscribe'; topic: string; fromSeq?: number }
  | { id: string; kind: 'unsubscribe'; topic: string }

type BridgeWelcome = {
  selectedProtocolVersion: number
  daemonVersion: string
  instanceId: string
  schemaEpoch: number
  commitSeq: number
  healthState: ControlPlaneState
  mutationsAllowed: boolean
  activeSessions: { count: number; ids: string[]; states: Record<string, string> }
}
```

**Handshake:** First message from desktop is always `hello`. Daemon responds with `BridgeWelcome`. If `lastSeenInstanceId` differs from `instanceId`, the desktop knows the daemon restarted — it must invalidate all cached state and not attempt to resume PTY streams from previous sessions.

```typescript
type BridgeEvent =
  | { topic: 'daemon.health'; payload: DaemonHealthPayload }
  | { topic: string; payload: JobProgressPayload }
  | { topic: string; seq: number; kind: 'pty' | 'stdout' | 'stderr' | 'event' | 'lifecycle' | 'gap'; payload: unknown }
```

**Commands available in Layer 2:**

| Command | Action | Returns |
|---------|--------|---------|
| `trigger_ingestion(source_id)` | Start ingestion job for a source | `MutationAck` with `jobId` |
| `trigger_enrichment(process_id)` | Re-run enrichment for a process | `MutationAck` with `jobId` |
| `capture_decision(process_id, disposition, basis)` | Record operator decision | `MutationAck` |
| `resolve_error(error_manifest_id)` | Mark error as resolved | `MutationAck` |
| `stop_session(session_id, mode)` | Stop active session (`graceful` or `force`) | `MutationAck` |
| `send_session_input(session_id, bytes)` | Send keyboard input to attached PTY session | `MutationAck` |
| `resize_session(session_id, cols, rows)` | Resize attached PTY session terminal | `MutationAck` |
| `retry_job(job_id)` | Retry a failed job | `MutationAck` with new `jobId` |
| `restart_daemon()` | Request clean daemon restart | `MutationAck` (ack sent before restart) |
| `rerun_from_error(error_manifest_id)` | Re-run pipeline from the failed stage | `MutationAck` with new `jobId` |

Without at least stop/input/retry/restart, the operator still needs terminal-level escape hatches — undermining the "control panel, not browser" promise.

### 1.6 Channel\<T\> Live Streaming

Tauri's typed streaming mechanism for live session output from daemon to frontend.

**Session frame types:**

```typescript
type SessionFrame =
  | { seq: number; kind: 'lifecycle'; state: string; cleanupVerdict?: 'clean' | 'forced' | 'leaked'; archiveSeq?: number; ts: string }
  | { seq: number; kind: 'pty'; chunk: Uint8Array; ts: string }
  | { seq: number; kind: 'stdout'; chunk: string; ts: string }
  | { seq: number; kind: 'stderr'; chunk: string; ts: string }
  | { seq: number; kind: 'event'; event: StructuredSessionEvent; ts: string }
  | { seq: number; kind: 'gap'; fromSeq: number; toSeq: number }
```

**Two subscription levels:**

| Level | What | Cost | Used By |
|-------|------|------|---------|
| All running sessions (metadata only) | Status, checkpoint count, last line excerpt, elapsed time | Cheap — session list can show 20+ concurrent sessions | Session list page |
| Selected session (full stream) | Full PTY/stdout/stderr via `Channel<T>` | Expensive — at most 2-3 sessions attached simultaneously | Session detail viewer |

**Buffering strategy:**
- Batch frames every 33-50ms before writing to terminal component
- Keep only last N lines / M MB hot in live buffer
- User scrolls up → switch from live-tail to history mode (lazy-loaded from chunked artifact store)
- UI falls behind stream → emit `gap` frame; `SessionViewer` recovers by fetching artifact chunks covering `fromSeq..toSeq` from the chunk index (`stream.index.jsonl`)

**Live → historical handoff:**

When a session completes:
1. Daemon emits `Archived { archiveSeq }` lifecycle event (where `archiveSeq` is the `commit_seq` at session archival)
2. `SessionRegistry` marks session as archived with `archiveSeq`
3. `SessionViewer` waits until a query returns `snapshotSeq >= archiveSeq`
4. Only then does `SessionViewer` switch from live stream to historical artifact mode

This removes the "session finished but missing from Sessions page" seam — the UI waits for the DB snapshot to catch up before switching modes.

**Chunked replay for desktop reconnect:**

When desktop reconnects after close/reopen (daemon stayed alive):
1. Handshake returns active session summary
2. `SessionViewer` subscribes from `lastSeq` (persisted in `SessionRegistry`)
3. If `lastSeq` is behind current stream, daemon sends `gap` frame
4. `SessionViewer` fetches missing chunks from artifact store to fill the gap
5. Live streaming resumes from current position

When daemon restarted (detected by `instanceId` mismatch): PTY streams are NOT resumable. `SessionViewer` shows historical artifacts only for previous sessions.

**Dual viewer pattern:**

| Viewer | Default For | Description |
|--------|------------|-------------|
| Live terminal | PTY mode sessions | ANSI-faithful, auto-scroll, last few thousand lines. **Optionally interactive:** keyboard input/paste sent to PTY for auth prompts and stuck interactive flows. Terminal escape sanitization (strip dangerous OSC sequences). Resize support for attached sessions. |
| Transcript/history | Contract mode sessions | Virtualized line list, searchable, lazy-loaded from chunked artifact store. Input disabled (clearly indicated). |

**Browser-auth challenge handling:** Some MFA/CAPTCHA flows are browser-only and cannot be resolved via PTY input. For L2, add a minimal intervention path: `open_managed_browser_session(session_id)` or "reveal profile path" + clear instructions + reattach button. Otherwise `AGENT_AUTH_CHALLENGE` is visible but not actionable from the desktop.

### 1.7 Bootstrap State Machine

On launch, the desktop determines its runtime mode:

```text
Start
  -> Check local DB readability + schema_epoch (PRAGMA user_version)
  -> Attempt daemon handshake over socket/pipe
  -> Handshake returns: daemon version, IPC protocol version, schema_epoch,
     sync health, active sessions summary
```

**Runtime modes:**

Three flattened states are insufficient — "daemon reachable but unhealthy" is not the same as "daemon unreachable." The bootstrap derives UX mode from `ControlPlaneState`:

| Mode | Condition | UX |
|------|-----------|-----|
| **Full control** | `storage=ready`, `daemon=ready`, `control=full` | Full desktop: list, detail, trigger, decide, monitor, stop, input, retry |
| **Control limited** | `storage=ready`, `daemon=ready\|degraded`, `control=limited` | Read/write but new job triggers may be refused. Banner: "Daemon busy — some operations limited." |
| **Browse-only** | `storage=ready`, `daemon=unavailable` | Read-only local browsing: list, detail, manifests, historical transcripts. No triggers, no live sessions. Banner: "Daemon unavailable — read-only local snapshot." |
| **Incompatible** | `daemon=incompatible` OR `storage=schema_mismatch` | Protocol/schema mismatch. Options: restart daemon, update desktop, or enter browse-only. |
| **Recovery required** | `storage=unreadable\|repairing` | DB inaccessible or corrupt. Repair/update screen. |

**Daemon crash mid-session:**
1. Stream/heartbeat drops
2. Desktop shows reconnect overlay
3. Bootstrap attempts one automatic daemon restart
4. On reconnect: desktop performs new handshake — if `instanceId` changed, the daemon restarted
5. Invalidate all cached state and snapshot queries
6. Do NOT attempt to resume PTY streams from previous sessions (PTY master fds died with the daemon)
7. Daemon reconciles orphan sessions on its side (kill surviving descendants, synthesize error manifests)
8. If restart fails: fall back to browse-only mode

**Critical distinction:** Desktop reconnect (daemon stayed alive) and daemon restart are fundamentally different scenarios. Desktop reconnect is cheap — streams are resumable from `lastSeq`. Daemon restart is lossy — PTY sessions from the previous daemon instance are dead. The desktop detects which scenario occurred via `instanceId` comparison in the handshake.

**Shutdown behavior:** Closing the main window does not silently kill the daemon.
- **Quit UI** — daemon keeps running (important for Layer 3 cron)
- **Stop background daemon and quit** — clean daemon shutdown

**Startup sequence (L2):**

1. Verify DB readability (`PRAGMA quick_check`)
2. Compare `schema_epoch` (`PRAGMA user_version`)
3. Read `daemon.info.json` descriptor file
4. Attempt daemon handshake over UDS
5. If no daemon: launch detached daemon process once
6. Retry handshake (with backoff)
7. If still unavailable: enter browse-only mode
8. If protocol/schema incompatible: enter incompatible mode (offer restart or browse-only)
9. If daemon reports `degraded` or `saturated`: enter control-limited mode

The desktop does NOT require the user to manually start the daemon. Auto-launch on first open is essential for the control-panel promise.

### 1.8 Dev-Mode Browser Transport (Test-Only)

Tauri's `invoke()` and `Channel<T>` only work inside the native webview — `window.__TAURI_INTERNALS__` is injected by the Tauri runtime and absent in a plain browser. On macOS (WKWebView), Playwright cannot connect to the webview via CDP either. This makes browser-based testing tools (Playwright MCP, Vercel agent browser skills) unable to interact with the real app.

**Solution:** A dev-only Vite middleware that bridges the gap so the React SPA is fully functional at `http://localhost:5173` in any browser. This is the **primary E2E testing surface** for all UI functionality (see §4).

```text
Browser (Playwright / any browser)
  -> http://localhost:5173          Vite dev server serves React SPA
  -> POST /__tauri_invoke__         Vite middleware proxies to daemon
       -> QueryService logic        (read-only SQLite, same queries)
       -> DaemonBridge logic        (UDS/pipe forward, same commands)
  -> ws://localhost:5173/__ws__     WebSocket relay for live streaming
       -> daemon event stream       (same SessionFrame / BridgeEvent types)
```

**Runtime shim (~10 lines, in `src/lib/transport.ts`):**

```typescript
// Production: Tauri bridge exists, use it directly.
// Dev browser: bridge absent, fall back to HTTP/WebSocket via Vite middleware.
export const invoke = window.__TAURI_INTERNALS__?.invoke
  ?? ((cmd: string, args?: Record<string, unknown>) =>
    fetch('/__tauri_invoke__', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ cmd, args }),
    }).then(r => r.json()))
```

All existing hooks (`use-daemon.ts`, `use-control-plane.ts`) and stores (`session-registry.ts`, `session-buffer.ts`) call through this shim instead of importing `@tauri-apps/api` directly. In the Tauri webview, the native bridge is always present and the HTTP path is dead code.

**Channel\<T\> equivalent:** The Vite dev plugin opens a WebSocket at `/__ws__` that relays `SessionFrame` and `BridgeEvent` payloads from the daemon. The `session-registry.ts` store connects via WebSocket when `window.__TAURI_INTERNALS__` is absent, using the same frame types and gap-recovery logic.

**What this enables:**
- `npm run dev` starts Vite with the middleware plugin active
- Playwright navigates to `localhost:5173` in a standard Chromium browser
- Full daemon access: real data, real mutations, real live streaming
- All UI E2E tests run in a browser — no Tauri build required for iteration

**What this does NOT change:**
- Production transport remains `invoke()` + `Channel<T>` through the Tauri webview
- The daemon does not know or care whether commands arrive via Tauri bridge or HTTP middleware — it receives the same UDS messages
- The dev middleware is not shipped in production builds
- This is NOT the Layer 4 web viewing portal (which is a separate Next.js app reading from Supabase)

**Security:** The middleware binds to `127.0.0.1` only and is excluded from production builds via Vite's `mode` configuration. It exists solely for local development and testing.

**Dev-mode files (not shipped):**

| File | Purpose |
|------|---------|
| `vite-plugin-tauri-dev-bridge.ts` | Vite plugin: HTTP handler for `/__tauri_invoke__` (proxies to daemon via UDS) + WebSocket relay at `/__ws__` |
| `src/lib/transport.ts` | Runtime shim: selects Tauri native bridge or HTTP fallback based on `window.__TAURI_INTERNALS__` presence |

### 1.9 Desktop File Listing

**`apps/desktop/` — 18 files (16 shipped + 2 dev-only):**

| File | Responsibility |
|------|---------------|
| `src-tauri/src/main.rs` | Tauri Rust bootstrap: DB check, daemon handshake, QueryService + DaemonBridge init |
| `src-tauri/src/query_service.rs` | Read-only SQLite queries returning `QueryEnvelope<T>` |
| `src-tauri/src/daemon_bridge.rs` | RPC proxy to daemon over UDS/named-pipe; `Channel<T>` relay |
| `src-tauri/capabilities/default.json` | Allowed shell/sidecar commands |
| `src-tauri/tauri.conf.json` | Sidecar and desktop build configuration |
| `src/App.tsx` | Root React component with routing and bootstrap state |
| `src/pages/ContractList.tsx` | Keyset-paginated process list with pipeline badges and decision status |
| `src/pages/ContractDetail.tsx` | Full opportunity detail: candidates, quotes, margins, manifest, errors |
| `src/pages/Sessions.tsx` | Active/recent agent session list with metadata |
| `src/pages/SessionViewer.tsx` | Live terminal/transcript viewer with dual mode (PTY/contract) |
| `src/pages/Errors.tsx` | Unresolved error manifest list with drill-down |
| `src/stores/session-registry.ts` | External store: tracks all active/recent sessions with metadata; keyed by `daemonInstanceId + sessionId`; tracks `lastSeq`, `gapState`, `archiveSeq`, `reattachable` |
| `src/stores/session-buffer.ts` | External store: per-session buffer — PTY mode stores bytes (not lines), contract mode stores text frames; explicit gap markers |
| `src/hooks/use-daemon.ts` | `useSessionMeta()`, `useSessionTail()`, `useSessionTimeline()` — slice subscriptions via `useSyncExternalStore` |
| `src/pages/DaemonStatus.tsx` | Daemon operations: instance id/version, socket path, DB path, artifact root, WAL size, startup/recovery info, restart button, open logs/artifacts |
| `src/hooks/use-control-plane.ts` | `useControlPlaneState()` — daemon health separate from query snapshots; consumed via `useSyncExternalStore` |
| `src/lib/transport.ts` | **Dev + shipped:** Runtime shim — selects Tauri native bridge or HTTP fallback (see §1.8) |
| `vite-plugin-tauri-dev-bridge.ts` | **Dev-only (not shipped):** Vite plugin — HTTP proxy for `invoke()` + WebSocket relay for streaming (see §1.8) |

### 1.10 React State Management

**Pattern:** Purpose-built external stores with `useSyncExternalStore()` for slice subscriptions. **Not** TanStack Query for streaming state.

| State Class | Approach |
|-------------|----------|
| Snapshot server-state | TanStack Query for snapshot reads only — no offline persistence, no normalized cache |
| Live streaming state | Custom external stores (`SessionRegistry`, `SessionBuffer`) + `useSyncExternalStore` |
| Control-plane state | Custom external store updated from daemon handshake/heartbeat — separate from query snapshots |
| Ephemeral UI state | Local component state / tiny persisted store |

**Rule: never put raw transcript arrays in React state.** Session output is a log system problem, not a React state problem.

**Hooks:**
- `useSessionMeta(sessionId)` — metadata slice (status, elapsed, checkpoints)
- `useSessionTail(sessionId)` — last N lines for terminal view
- `useSessionTimeline(sessionId)` — structured event timeline for contract mode

### 1.11 Shared UI Components (`packages/ui`)

**Frame + slot composition pattern** (not boolean props):

```tsx
<ContractCardFrame
  summary={vm.summary}
  metrics={vm.metrics}
  rightRail={<FreshnessBadge vm={vm.freshness} />}
  actions={desktop ? <DesktopContractActions ... /> : null}
/>
```

**5 shared components:**

| Component | Purpose | Used By |
|-----------|---------|---------|
| `ContractCardFrame` | Process summary card with slot composition | Desktop list, web list (L4) |
| `ManifestViewer` | Markdown manifest renderer with section navigation and HTML sanitization | Desktop detail, web detail (L4) |
| `PipelineBadge` | Pipeline state badge (qualified/review/rejected/pending) | Both surfaces |
| `FreshnessBadge` | Data freshness indicator — accepts surface-neutral `FreshnessBadgeVM` (label, tone, tooltip); desktop and web compute the VM separately | Both surfaces |
| `ErrorSummaryList` | Compact error manifest list with severity/resolution status | Both surfaces |

#### `error-summary-list.tsx` — Error Manifest List

```tsx
interface ErrorSummaryItem {
  errorManifestId: string
  processId: string
  pipelineStage: string
  errorType: string
  errorMessage: string
  severity: 'critical' | 'warning' | 'info'   // derived from error_type category
  createdAt: string
  resolved: boolean
  resolvedAt?: string
}

interface ErrorSummaryListProps {
  errors: ErrorSummaryItem[]
  onErrorClick?: (errorManifestId: string) => void
  onResolveClick?: (errorManifestId: string) => void  // null on web (read-only)
}
```

**Severity derivation rule:** `AGENT_*` and `HOST_*` errors with cleanup verdict `leaked` → `critical`. Other `AGENT_*` errors → `warning`. `SOURCE_*` and pipeline errors → `warning`. Informational outcomes (`continuation_required`, `degraded_completed`) → `info`.

**Sharing boundary:** Share down to view-models, presentational components, formatting/markdown/tables/badges. Stop sharing at repositories, hooks, realtime subscriptions, mutation flows.

**Security boundary:** Agent-authored content (manifests, error messages, session transcripts) is **untrusted input**:
- `ManifestViewer`: sanitize Markdown, disable raw HTML, strip `javascript:` and unsafe `file://` links
- `ErrorSummaryList`: sanitize error messages and context
- Never render arbitrary embedded HTML from agent text

### 1.12 Manifest Rendering (`packages/manifests`)

**2 files:**

| File | Purpose |
|------|---------|
| `render-opportunity.ts` | Deterministic opportunity manifest renderer — takes pipeline data, produces structured Markdown |
| `opportunity.md.ts` | Markdown template for opportunity manifests |

The canonical rendering lives here, not in the `manifest-generation` skill. The skill may contribute optional narrative synthesis, but the deterministic renderer is authoritative.

**`decisionBasisHash` ownership:** The hash is daemon-authored from a canonical serialization of operator-relevant fields, NOT computed by the manifest renderer or by a skill. The renderer consumes the already-computed hash. This prevents hash drift across Rust/TS/agent implementations.

### 1.13 New Skill: Contracts Finder Ingestion

| Attribute | Value |
|-----------|-------|
| **Path** | `skills/procurement-ingestion/contracts-finder/` |
| **Purpose** | Ingest live notices from Contracts Finder |
| **Interaction profile** | `pty_interactive` |
| **API** | OCDS/Search API: `publishedFrom`, `publishedTo`, `stages`, `limit`, `cursor`. Rate limiting documented. |
| **Licensing** | Aligned with open contracting/publication standards |
| **Cursor strategy** | Date-based filtering + cursor; rate-limit-aware polling |

### 1.14 What's Stubbed in This Layer

| Feature | Layer 2 Stub | Full Implementation |
|---------|-------------|-------------------|
| Scheduling | Manual trigger from desktop | Layer 3: cron scheduler |
| Relation resolution | Notices processed independently | Layer 3: merge/relate heuristics |
| Landed cost | Screening-grade (flat freight, single FX) | Layer 5: decision-grade |
| Search sources | eBay only (from Layer 0) | Layer 5: Alibaba/1688, Amazon |
| Sync to Supabase | None — local-only operation | Layer 4: full sync |
| Operator decisions | Local intent queue only — single-operator overlay, no shared authority | Layer 4: remote-owned `operator_decision_heads` + CAS |
| Validation | Disabled (fields exist) | Layer 3: dry-run |
| Degraded-mode UX | Five-state structured health (full/limited/browse-only/incompatible/recovery) | Layer 4: adds sync-specific states (sync healthy/degraded/offline as distinct dimension) |
| Web portal | Not present | Layer 4: read-only viewing portal |

### 1.15 Things NOT Implemented in Layer 2

- Cron scheduling (Layer 3)
- Resource arbiter (Layer 3)
- Worker-validator system (Layer 3/4)
- Sync engine (Layer 4)
- Web viewing portal (Layer 4)
- Multi-source marketplace search (Layer 5)
- Decision-grade landed cost (Layer 5)
- Bid workflow automation (Layer 6)
- Multi-user / RBAC (not in MVP)

### 1.16 Accessibility

- Do NOT `aria-live` every transcript line — announce only state transitions ("session stalled", "new opportunities available")
- Do NOT use color as the only signal for margin/risk or sync health — provide text labels and icons alongside
- Provide plain-text transcript mode for screen readers alongside ANSI terminal view
- Keyset pagination is accessible by default (stable focus targets, no shifting content)
- Deep links resolve through `process_aliases` so bookmarked IDs survive later merges

---

## 2. Shared UI Components & Manifest Rendering

### 2.1 `packages/ui` — Shared Presentational Components

**5 files.** These components are shared between the desktop control panel (Layer 2) and the web viewing portal (Layer 4). They are **pure presentational** — they receive typed props and render UI. They must NOT import data-fetching hooks, Supabase clients, Tauri invoke wrappers, daemon bridge code, or mutation orchestration.

**Boundary rule:** Share down to view-models, presentational components, and formatting helpers. Stop sharing at repositories, hooks, realtime subscriptions, and mutation flows. Data-fetching is surface-specific: desktop uses QueryService (SQLite), web uses Supabase client.

#### `index.ts` — Barrel Export

Re-exports all shared components. Consumers import from `@project/ui`.

#### `contract-card-frame.tsx` — Process Summary Card

Frame + slot composition for process/opportunity cards in list views.

```tsx
interface ContractCardFrameProps {
  summary: {
    processId: string
    title: string
    buyerName: string
    publishedDate: string
    deadlineDate: string | null
    estimatedValuePence: number | null
    lotCount: number
    accessRoute: string
    legalCategory: 'goods' | 'services' | 'works' | 'mixed' | 'unknown'
  }
  metrics: {
    pipelineState: 'pending' | 'qualifying' | 'enriching' | 'qualified' | 'review' | 'rejected'
    pricedCoverageBps: number | null
    netQualifiedCoverageBps: number | null
    grossMarginBps: number | null     // best candidate
    netMarginBaseBps: number | null   // best candidate
    candidateCount: number
    quotedCandidateCount: number
  }
  decision?: {
    disposition: 'approve' | 'reject' | 'watch' | null
    isStale: boolean   // decisionBasisHash changed since decision
    decidedAt: string | null
  }
  rightRail?: React.ReactNode   // Slot for FreshnessBadge, custom badges
  actions?: React.ReactNode     // Slot for desktop-only action buttons (null on web)
}
```

**Rendering rules:**
- All monetary values displayed in `£X,XXX` format (from integer pence)
- All percentages displayed from basis points (`bps / 100` for display)
- Stale decisions show warning indicator
- Keyset pagination: card order is stable (no shifting on live updates)

#### `manifest-viewer.tsx` — Opportunity Manifest Renderer

Renders Markdown opportunity manifests with section navigation sidebar.

```tsx
interface ManifestViewerProps {
  manifestMarkdown: string
  manifestVersion: string
  basisHash: string
  generatedAt: string
  sections?: string[]           // Parsed section headings for sidebar nav
  highlightBlockers?: boolean   // Visually highlight blocker sections
  onSectionClick?: (sectionId: string) => void
}
```

**Features:**
- Markdown rendering (via `marked` or equivalent) with **HTML sanitization** — agent-authored Markdown is untrusted input
- Disable raw HTML passthrough in Markdown parser
- Strip `javascript:`, `data:`, and unsafe `file://` links
- Table formatting for pricing/margin data
- Blocker highlighting (red badge on sections with critical blockers)
- Section navigation sidebar
- Copy-to-clipboard for individual sections
- Print-friendly layout

#### `pipeline-badge.tsx` — Pipeline State Indicator

```tsx
interface PipelineBadgeProps {
  state: 'pending' | 'qualifying' | 'enriching' | 'qualified' | 'review' | 'rejected' | 'error'
  validationState?: 'unverified' | 'validated' | 'flagged'  // process-level validation state from pipeline_assessments (Layer 3+)
  size?: 'sm' | 'md' | 'lg'
}
```

**Display rules:**
- Color + text label + icon (never color alone — accessibility)
- `validationState` badge shown only when not `unverified` (Layer 3+)
- Tooltip with timestamp of last state change

#### `freshness-badge.tsx` — Data Freshness Indicator

```tsx
interface FreshnessBadgeVM {
  label: string                   // e.g., "Fresh", "2 min ago", "Stale"
  tone: 'good' | 'warn' | 'bad' | 'muted'
  tooltip: string                 // e.g., "Last sync: 2026-03-19T10:05:23Z"
}

interface FreshnessBadgeProps {
  vm: FreshnessBadgeVM
}
```

**Surface-neutral design:** Desktop and web compute the `FreshnessBadgeVM` separately from their own state. Desktop derives it from daemon heartbeat and local `snapshotSeq` freshness. Web (Layer 4) derives it from Supabase sync freshness. The component itself is purely presentational — it does not know which surface it runs on.

**Thresholds (Layer 4 — for web portal sync freshness):**
- Green: < 90 seconds since last sync
- Amber: 90s–5min
- Red: > 5min or daemon offline
- Text: "Data may be behind desktop local state."

**Layer 2 behavior:** Shows daemon connection status and local data freshness only. Sync fields are null.

### 2.2 `packages/manifests` — Opportunity Manifest Renderer

**2 files.** Deterministic opportunity manifest rendering from pipeline data. The canonical rendering lives here — the `manifest-generation` skill may contribute narrative synthesis, but this package is authoritative.

#### `render-opportunity.ts` — Manifest Renderer

```typescript
interface ManifestInput {
  process: ProcurementProcess
  notices: ContractNotice[]
  lots: NoticeLot[]
  candidates: ProductCandidate[]
  quotes: SupplierQuote[]
  pricingAssessments: CandidatePricingAssessment[]
  landedCosts: LandedCostEstimate[]
  pipelineAssessment: PipelineAssessment
  coverageMetrics: {
    pricedCoverageBps: number
    landedCostCoverageBps: number
    netQualifiedCoverageBps: number
    unresolvedMandatoryLots: number
  }
}

interface ManifestOutput {
  markdown: string
  basisHash: string           // daemon-provided SHA-256 of operator-relevant pipeline data (passed through, NOT computed here)
  sections: string[]          // Parsed section headings
  blockerCount: number
  generatedAt: string
}

function renderOpportunityManifest(input: ManifestInput): ManifestOutput
```

#### `opportunity.md.ts` — Markdown Template

Sections in the generated manifest:

| Section | Content |
|---------|---------|
| **Summary** | Process title, buyer, value, lot count, deadline, access route |
| **Lot Analysis** | Per-lot qualification result, addressable value, goods classification |
| **Product Candidates** | Per-lot candidate list with archetype, identity confidence, quoteability score |
| **Pricing Evidence** | Per-candidate quotes with source, match confidence, usability confidence, price-signal role |
| **Margin Estimate** | Gross margin gate result, landed cost breakdown, net margin band (low/base/high) |
| **Coverage Metrics** | `pricedCoverageBps`, `netQualifiedCoverageBps`, `unresolvedMandatoryLots` |
| **Risk Flags** | Critical blockers, low-confidence components, stale data warnings |
| **Recommendation** | Auto-qualified / manual review required / not qualified — with reasoning |

**Formatting rules:**
- All monetary values in `£X,XXX.XX` (from integer pence)
- All percentages from basis points
- Blockers shown prominently with red indicators
- Confidence shown as bands (high/usable/manual), not decimal numbers
- Component scores shown in panel format, not as a single composite number

### 2.3 Component Design Rules (Constitutional)

1. **`packages/ui` components must NOT import** `packages/db`, desktop bridge code, daemon modules, Supabase clients, or data-fetching hooks
2. **`packages/manifests` must NOT import** daemon modules or Supabase clients — it operates on typed inputs only
3. **Data-fetching is surface-specific:** Desktop uses `QueryService` (SQLite via `invoke()`); Web (Layer 4) uses Supabase client
4. **No boolean prop explosion:** Use frame + slot composition, not `<Card canReject canBid readOnly showSync ... />`
5. **View-model types shared; data-access types not shared:** `ContractCardFrameProps` is shared; `useContractsList()` is not
6. **All monetary display from integer pence;** all percentage display from integer basis points — no floating-point formatting paths
7. **Agent-authored content is untrusted input.** All renderers that display agent or skill output (ManifestViewer, ErrorSummaryList, SessionViewer) must sanitize their input: disable raw HTML in Markdown, strip dangerous links, sanitize dangerous OSC terminal escape sequences (OSC 8 hyperlinks, clipboard-related escapes). This is a real desktop/webview security boundary.
8. **Testing shared components:** Use fixture-based view-model tests at the `packages/ui` level (`packages/ui/fixtures/*.ts`), snapshot/visual tests at the package level, surface-specific integration tests in `apps/desktop` and later `apps/web`. Never couple shared component tests to specific data-fetching backends.

---

## 3. Pipeline Surface, Coverage & Decision Capture

### 3.1 Coverage Metrics — Operator-Facing Presentation

> **Ownership:** L0 §3 owns the full pipeline-side derivation and production logic. Strategy §10 owns the cross-layer interface summary. This section owns **Layer 2 presentation**: thresholds, display format, and operator-facing behavior.

**Coverage metric interface (compact recap for self-containment):**

| Field | Type | Meaning | Denominator |
|-------|------|---------|-------------|
| `pricedCoverageBps` | `INTEGER` (0-10000) | Fraction of qualifying lots with >=1 priced candidate | Total qualifying lots |
| `landedCostCoverageBps` | `INTEGER` (0-10000) | Fraction of qualifying lots with a landed cost estimate | Total qualifying lots |
| `netQualifiedCoverageBps` | `INTEGER` (0-10000) | Fraction of qualifying lots passing the net margin gate | Total qualifying lots |
| `unresolvedMandatoryLots` | `INTEGER` | Count of mandatory lots without resolution | N/A (absolute count) |
| `decisionBasisHash` | `TEXT` | SHA-256 of the pipeline data the operator would review | N/A (staleness key) |

Display: `bps / 100` for percentage (e.g., 7500 bps = "75.0%").

**Color thresholds for `netQualifiedCoverageBps`:**

| Range | Color | Label |
|-------|-------|-------|
| >= 7000 bps (70%+) | Green | Strong coverage |
| 3000-6999 bps (30-69%) | Amber | Partial coverage -- review recommended |
| < 3000 bps (< 30%) | Red | Low coverage -- manual review required |

Always show both color AND text label (accessibility). Auto-qualification guard: `netQualifiedCoverageBps >= 5000` (50%) required. Desktop surfaces "Coverage below threshold" indicator when not met.

### 3.2 Decision Capture — `pending_operator_intents`

Local single-operator overlay. NOT the authoritative shared decision table (that is `operator_decision_heads` in Layer 4 via CAS).

**`decisionBasisHash` is daemon-authored:** The hash is computed by the daemon from a canonical serialization of operator-relevant pipeline fields. It is NOT computed by the manifest renderer, by a skill, or by the desktop. The desktop captures the daemon-provided hash at decision time. This prevents hash drift across Rust/TS/agent implementations.

**Local dispositions:** `approve` | `reject` | `watch`

**Remote canonical states (Layer 4):** `approved` | `rejected` | `watching`. L2 persists local intent verbs; the L4 CAS flush normalizes them into the remote-owned `operator_decision_heads` state model.

**Decision metadata captured on every intent:**

| Field | Purpose |
|-------|---------|
| `process_id` | Target process |
| `disposition` | approve / reject / watch |
| `reviewedBasisHash` | SHA-256 of pipeline data at decision time |
| `reviewedManifestHash` | SHA-256 of the manifest reviewed |
| `reviewedAtStageRunId` | Current `stage_run_id` at decision time |
| `idempotencyKey` | UUID for future CAS deduplication (Layer 4) |
| `decidedAt` | Timestamp |
| `status` | `queued` (always in L2 -- no remote flush yet) |

**Latest-per-process coalescing:** Only the latest intent per process matters. If operator changes `approve` to `reject`, the `reject` supersedes. Desktop immediately reflects the queued intent as the current decision. In L2, the local intent IS the decision. In L4, it becomes a queue entry flushed to Supabase via CAS.

### 3.3 Decision Staleness Detection

1. Operator decides -> `reviewedBasisHash` captured from `pipeline_assessments.decisionBasisHash`
2. Pipeline re-runs -> `decisionBasisHash` changes
3. Desktop compares: mismatch = **stale**

**Stale decision UX:**
- Warning indicator: "Decision may be outdated -- pipeline data changed since review"
- Operator can re-affirm (new intent with updated hash) or change disposition
- **Stale decisions block bid workflow** (Layer 6) -- must re-affirm first

**Staleness triggers:** New/updated quotes, re-run landed cost, new documents changing candidates, relation resolution changes, any upstream change modifying `decisionBasisHash`.

### 3.4 Coverage Rollup Module

`packages/intelligence/src/margins/coverage-rollup.ts` -- runtime implementation of L0 §3.14 formulas.

```typescript
interface CoverageInput {
  lots: Array<{
    lotId: string
    qualificationResult: 'pass' | 'review' | 'reject'
    isMandatory: boolean
    candidates: Array<{
      candidateId: string
      hasPricing: boolean
      hasLandedCost: boolean
      passesNetGate: boolean
      candidateValueShareBps: number
    }>
  }>
}

interface CoverageOutput {
  pricedCoverageBps: number
  landedCostCoverageBps: number
  netQualifiedCoverageBps: number
  unresolvedMandatoryLots: number
  decisionBasisHash: string
}

function computeCoverageRollup(input: CoverageInput): CoverageOutput
```

### 3.5 Keyset Pagination

All list views use **keyset/cursor pagination** (not offset). Offset causes row jumping on live updates.

- Client sends: `{ after: lastSeenCursor, limit: 25, filters: {...} }`
- Server returns: `{ items: [...], nextCursor: string | null, hasMore: boolean }`
- Cursor encodes sort position (e.g., `created_at + id`)
- Stable focus targets -- items don't shift while user reads

---

## 4. Testing & Acceptance

### 4.1 Testing Tiers

**Browser-on-localhost via Playwright is the primary UI testing surface.** Testing desktop apps natively requires OS-level interaction (screen capture, mouse simulation — "computer use"), which is slow and fragile. Since Tauri's frontend is a standard React SPA served by Vite on localhost, and the dev-mode transport bridge (§1.8) provides full daemon access in a plain browser, we test UI functionality in a real browser using Playwright. This enables use of Playwright MCP and Vercel agent browser skills for interactive verification that simulates real human usage.

| Tier | Surface | What It Covers | When It Runs |
|------|---------|---------------|--------------|
| **1. Browser E2E (primary)** | Playwright against `localhost:5173` via dev bridge | All UI rendering, navigation, pagination, data display, decision capture, session monitoring, coverage metrics, error views, staleness indicators, commit-fence consistency, live streaming via WebSocket, stop/input/retry controls | Every iteration — fast feedback loop, no Tauri build required |
| **2. Tauri verification** | Full Tauri build, manual + scripted | Desktop-only behaviors: daemon auto-launch on app open, window lifecycle (close UI ≠ kill daemon), browse-only fallback on daemon unavailability, `instanceId` mismatch detection after daemon crash | Before milestone gate — verifies Tauri-specific integration |
| **3. Agent E2E** | Rust test harness, real daemon + real skills | Skill runtime: PTY spawn, notice extraction, result contract validation, cleanup verdicts, error manifests | CI gate — validates pipeline backend independently of UI |

**Why browser-first:** Criteria 6-11 and 14-15 in the Definition of Done (§4.2) test UI behavior that has no Tauri dependency — list pagination, detail views, decision capture, coverage badges, session controls, commit-fence consistency. These run faster, more reliably, and with better tooling (Playwright selectors, assertions, screenshots, trace viewer) in a browser than through Tauri's webview. Only criteria 5, 12, 13, and 16 genuinely require the Tauri desktop context.

**Browser E2E workflow:**

```text
1. Start daemon (background)
2. npm run dev                   → Vite + dev bridge plugin on localhost:5173
3. npx playwright test           → Chromium navigates to localhost:5173
                                   Full daemon access via HTTP/WebSocket bridge
                                   Real data, real mutations, real streaming
```

Playwright tests interact with the app exactly as a human operator would — navigating pages, clicking buttons, filling forms, observing live updates. The dev bridge (§1.8) makes this indistinguishable from the Tauri webview experience at the application level.

### 4.2 Definition of Done

| # | Criterion | Verification | Tier |
|---|----------|-------------|------|
| 1 | **FTS ingestion skill** scrapes live notices from Find a Tender | Automated: agent E2E test produces notices | 3 |
| 2 | **Contracts Finder ingestion skill** scrapes live notices | Automated: agent E2E test produces notices | 3 |
| 3 | **Notice normalization** produces canonical internal records | Automated: normalized records match expected schema | 3 |
| 4 | **Goods/services classifier** correctly categorizes notices | Automated: unit tests for CPV-based classification | 3 |
| 5 | **Desktop control panel** launches with QueryService + DaemonBridge + bootstrap state machine | Tauri verification: app launches, state machine detects daemon | 2 |
| 6 | **Notice list view** with keyset pagination displays live pipeline output | Playwright: navigate list, verify pagination, verify data from real daemon | 1 |
| 7 | **Opportunity detail view** shows margin bands, product candidates, supplier quotes | Playwright: open detail, verify all data sections render with real pipeline data | 1 |
| 8 | **Decision capture** (reject/watch/approve) persists locally in `pending_operator_intents` with reviewed-basis metadata | Playwright: capture decision, verify DB state via daemon query | 1 |
| 9 | **Agent session monitoring** page shows live terminal output via streaming | Playwright: trigger ingestion, navigate to session viewer, verify live output appears via WebSocket stream | 1 |
| 10 | **Coverage metrics** displayed: `pricedCoverageBps`, `netQualifiedCoverageBps` with color thresholds | Playwright: verify badges render with correct colors and text labels | 1 |
| 11 | **Decision-basis staleness** indicators flag stale decisions | Playwright: capture decision, trigger re-run, verify staleness indicator appears | 1 |
| 12 | **Single operator** runs daily for 1 week and reports genuine opportunities surfaced | Manual: operator validation over 5-7 working days (Tauri desktop) | 2 |
| 13 | **Desktop auto-launches daemon** if not running; operator never needs manual daemon startup | Tauri verification: close daemon, open desktop, verify daemon starts automatically | 2 |
| 14 | **Stop/input/retry controls** work: stop active session, send PTY input for auth challenges, retry failed jobs | Playwright: exercise each control during live session, verify daemon responds | 1 |
| 15 | **Commit fence consistency:** mutation → `visibleAfterSeq` → subsequent query returns updated data | Playwright: trigger mutation, poll until `snapshotSeq >= visibleAfterSeq`, verify data | 1 |
| 16 | **Daemon restart detection:** desktop detects daemon restart via `instanceId` mismatch, invalidates state, does not attempt PTY stream resume | Tauri verification: kill daemon during session, verify desktop recovery | 2 |

### 4.3 Test Files

| File | Type | Tier | What It Tests |
|------|------|------|--------------|
| `apps/desktop/e2e/contracts.spec.ts` | Playwright (browser) | 1 | List browsing with keyset pagination, detail view with all data sections, coverage metrics display, pipeline badges, freshness indicators |
| `apps/desktop/e2e/decisions.spec.ts` | Playwright (browser) | 1 | Decision capture (approve/reject/watch), staleness detection after re-run, reviewed-basis metadata verification |
| `apps/desktop/e2e/sessions.spec.ts` | Playwright (browser) | 1 | Session list, live terminal output via WebSocket, session stop, PTY input for auth challenges, retry failed jobs |
| `apps/desktop/e2e/commit-fence.spec.ts` | Playwright (browser) | 1 | Mutation → `visibleAfterSeq` → query reflects change; verify no ghost states |
| `apps/desktop/e2e/errors.spec.ts` | Playwright (browser) | 1 | Error manifest list, drill-down, resolve action, severity indicators |
| `apps/desktop/e2e/security.spec.ts` | Playwright (browser) | 1 | Agent-authored Markdown sanitization (ManifestViewer), dangerous link stripping, ANSI escape sanitization (SessionViewer) |
| `apps/desktop/e2e/tauri-integration.spec.ts` | Tauri-specific | 2 | Daemon auto-launch, window lifecycle (close ≠ kill daemon), browse-only fallback, daemon restart detection via `instanceId` mismatch |
| `agent_procurement_ingestion.rs` | Agent E2E (Rust) | 3 | Real FTS skill session: PTY spawn, notice extraction, result contract validation, cleanup verdict |
| `agent_supplier_enrichment.rs` | Agent E2E (Rust) | 3 | Real eBay enrichment session: search execution, quote extraction, dual confidence scoring |
| `agent_health_detection.rs` | Agent E2E (Rust) | 3 | Stall detection (no output + no progress), auth challenge detection, cleanup escalation |
| `agent_error_manifest.rs` | Agent E2E (Rust) | 3 | Error manifest creation on various failure types (skill failed, contract invalid, suspect output) |

### 4.4 Test Scenarios

| Scenario | Expected Behavior | Tier |
|----------|------------------|------|
| Notice with 12 lots | System processes each lot independently; detail view shows per-lot results | 1 |
| Mixed contract with separable goods | Goods-addressable value used for margin computation, not whole contract value | 1 |
| Expired notice | Correctly filtered; not displayed as active opportunity | 1 |
| Agent session crashes mid-run | Error manifest created; operator sees failure context in error view; cleanup verdict is `forced` | 1 |
| Decision on process, then pipeline re-runs | Decision marked stale; warning indicator shown; bid workflow blocked | 1 |
| DPS/Dynamic Market notice | 30-day expiry filter NOT applied; notice remains active | 1 |
| Agent auth challenge during ingestion | `AGENT_AUTH_CHALLENGE` classified; operator alerted via session view | 1 |
| Stop active session from UI | Session stop command reaches daemon, session enters terminating state, cleanup verdict rendered | 1 |
| PTY input for auth challenge | Operator types into SessionViewer, input sent to PTY via `send_session_input`, agent proceeds | 1 |
| Retry failed job from UI | Retry command creates new job, original job marked, new session appears in session list | 1 |
| Commit fence: mutation then immediate query | Mutation returns `visibleAfterSeq`; query polls until `snapshotSeq >= visibleAfterSeq`; no ghost state | 1 |
| Keyset pagination under live updates | Immutable sort key (created_at + id) ensures stable pagination; no row jumping | 1 |
| Agent-authored Markdown containing raw HTML / link abuse | ManifestViewer sanitizes HTML, strips dangerous links | 1 |
| ANSI output containing hyperlink/clipboard escape sequences | SessionViewer sanitizes dangerous OSC sequences | 1 |
| Slow SessionViewer consumer causing gap | `gap` frame emitted, viewer recovers by fetching artifact chunks for missing `seq` range | 1 |
| Long-running QueryService read starving WAL checkpoints | Short-lived read transactions prevent WAL growth; test verifies transaction duration limits | 1 |
| Daemon unavailable at desktop launch | Bootstrap state machine falls back to browse-only mode with clear banner | 2 |
| Desktop closes and reopens while daemon stays alive and PTY session is active | Desktop reconnects, subscribes from `lastSeq`, stream resumes, no data loss | 2 |
| Daemon crashes while PTY session is active | Desktop detects `instanceId` mismatch, does NOT attempt PTY resume, shows historical artifacts only | 2 |
| Lost `MutationAck` response after DB commit, retried with same idempotency key | Idempotent: returns same ack, no duplicate writes | 1 |

### 4.5 Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Source website changes break FTS/CF skills | High | Medium | Error manifest flags failures; skill update cycle; page fingerprint drift detection |
| Desktop UX reveals pipeline quality issues | Medium | Medium | Expected -- this is why desktop comes before hardening (Layer 3). Operator feedback drives pipeline improvement. |
| Single-source eBay pricing not representative | High | Medium | Known limitation; Layer 5 adds Alibaba/1688 for wholesale pricing |
| QueryService read performance on large datasets | Low | Low | SQLite reads are fast; keyset pagination prevents full scans; short-lived read transactions |
| Desktop-daemon IPC failure | Medium | Medium | Bootstrap state machine detects failure; browse-only fallback; reconnect overlay |
| Dev bridge transport divergence from Tauri transport | Low | Low | Transport shim is thin (~10 lines); Tier 2 Tauri verification catches any divergence before milestone gate. Both paths produce identical daemon UDS messages. |

---

## 5. Open Questions, Forward Compatibility & Dependencies

### 5.1 Open Questions

**§10 Further Research #5: Auto-qualification coverage floor calibration**
- Default `process_coverage_floor_bps` is 5000 (50%). May be too aggressive for mixed contracts with many service lots.
- **Layer 2 impact:** Affects coverage rollup thresholds and auto-qualification guard in the desktop UI.

**§10 Further Research #1: Exact procurement source list by commercial importance**
- Which portals matter for daily operator value beyond FTS and Contracts Finder?
- **Layer 2 impact:** Determines whether PCS or other sources should be added before Layer 5.

### 5.2 Forward-Compatibility Provisions

| Provision | What Exists in L2 | Target Layer |
|-----------|-------------------|-------------|
| Bootstrap state machine health modes | Five-state structured health from `ControlPlaneState` (full/limited/browse-only/incompatible/recovery) | Layer 4: adds sync-specific dimension (sync healthy/degraded/offline) to `ControlPlaneState` |
| `packages/ui` frame + slot pattern | 5 components shared between desktop and web | Layer 4: web viewing portal uses same components |
| Coverage metrics on `pipeline_assessments` | Populated and displayed in desktop | Layer 4: sync projections push these to Supabase |
| `stage_run_id` on all stage outputs | Populated from Layer 1 | Layer 4: sync batch boundaries use `stage_run_id` |
| CAS payload on `pending_operator_intents` | `process_id`, disposition, reviewed-basis metadata, idempotency key | Layer 4: remote-owned `operator_decision_heads` with CAS flush |

### 5.3 Dependencies

- **Layer 0:** Intelligence pipeline, skills (5 initial), contracts-schema, SQLite schema
- **Layer 1:** Daemon runtime, artifact capture, process-group supervision, run manifests, error manifests

**Desktop configuration (from §9.2):**

| Config File | Purpose |
|-------------|---------|
| `apps/desktop/src-tauri/capabilities/default.json` | Allowed shell/sidecar commands only |
| `apps/desktop/src-tauri/tauri.conf.json` | Sidecar and desktop build configuration |

### 5.4 ADRs Addressed

| ADR | Decision | Layer 2 Action |
|-----|----------|---------------|
| **ADR-003** | Tauri v2 + separate daemon (not embedded) | Desktop shell created with QueryService + DaemonBridge architecture |
| **ADR-008** | Desktop = control panel (full read/write/trigger) | Full operator surface: list, detail, decide, trigger, monitor |
| **ADR-001** | Skills as primary data retrieval | FTS + Contracts Finder ingestion skills live |

<!-- CROSS-DOMAIN NOTE FOR L4: (1) ControlPlaneState gains a sync dimension in L4 — add `sync: 'healthy' | 'degraded' | 'offline'` alongside existing storage/daemon/control/load dimensions. The 5-state bootstrap model (full/limited/browse-only/incompatible/recovery) extends to incorporate sync state without replacing the structure. (2) The shared freshness indicator is `FreshnessBadge` with surface-neutral `FreshnessBadgeVM` (label, tone, tooltip). Web portal computes the VM from Supabase sync freshness (green <90s, amber 90s-5min, red >5min). (3) ErrorSummaryList has typed `ErrorSummaryItem` props with `severity` derived from error_type category (critical/warning/info). Web portal passes the same props from Supabase query results. The `onResolveClick` callback is null on web (read-only surface). (4) SessionFrame lifecycle frames carry `cleanupVerdict` on `reaped` and `archiveSeq` on `archived` — not applicable to web portal (web uses Supabase Broadcast, not Channel<T> streaming). (5) `QueryEnvelope<T>` is snapshot-only (snapshotSeq, snapshotAt) — web portal uses Supabase client, not QueryService, so the envelope type is desktop-specific. -->

<!-- CROSS-DOMAIN NOTE FOR L3: (1) L3 adds cron-triggered jobs to the DaemonBridge command set — the daemon's IPC command router (active from L1, extended in L2) accommodates new commands without protocol changes. (2) L3's validation dry-run adds a `validationState` badge to PipelineBadge; this badge reflects the process-level `pipeline_assessments.validation_state` contract (`validated` / `flagged` / `unverified`), not raw validator verdict enums. (3) L3's error manifest rendering (`packages/manifests/render-error-manifest.ts`) uses the same security boundary (sanitize agent-authored Markdown, strip dangerous links) as ManifestViewer. -->
