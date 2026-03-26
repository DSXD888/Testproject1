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

**Traceability rule:** Every testable Layer 2 behavior in this document is identified by an inline `L2-FR-*` marker. Later sections may reference an existing requirement ID instead of restating the full contract. File, command, component, Definition of Done, and test-scenario tables must all map back to these requirement IDs mechanically.

**L2-FR-BOOT.1 — The Desktop Shell Uses a Separate Background Daemon**
- AC1: The Tauri shell remains a client of a separately managed daemon process rather than embedding daemon execution inside the window lifecycle.
- AC2: Closing the desktop UI does not implicitly terminate a healthy daemon instance.
- AC3: The daemon can restart independently of the desktop and present a new `instanceId` on the next handshake.

**L2-FR-BOOT.2 — Desktop Transport Is Split by Responsibility**
- AC1: Snapshot reads use `invoke()` into `QueryService`; mutations use `invoke()` into `DaemonBridge`; live session and health updates use `Channel<T>` or the dev-mode equivalent from §1.8.
- AC2: Production builds do not expose a general localhost HTTP control surface as the primary transport.
- AC3: The frontend does not infer mutation visibility from stream activity alone; it combines transport responsibilities with the commit-fence contract in `L2-FR-QS.2`.

### 1.4 QueryService Interface

The QueryService opens **read-only** SQLite connections and returns page-specific read models. It enforces: read-only/query-only connections, page-limited queries, no streaming cursors held open, no generic SQL from the frontend, no transcript/blob reads in hot queries.

This is the desktop-side implementation of the snapshot-envelope and commit-fence contracts summarized in [layer-strategy.md](/Users/alinaaitova/Desktop/Testproject1/docs/strategy/layer-strategy.md#L1177) and first defined operationally in [layer-1-thin-local-runner.md](/Users/alinaaitova/Desktop/Testproject1/docs/strategy/layers/layer-1-thin-local-runner.md#L717).

```typescript
type QueryEnvelope<T> = {
  data: T
  snapshotSeq: number    // daemon-authored commit_seq at snapshot time
  snapshotAt: string     // ISO-8601 timestamp of snapshot
}

type KeysetPage<T> = {
  items: T[]
  nextCursor: string | null
  hasMore: boolean
}

type QueryServiceError = {
  code:
    | 'not_found'
    | 'invalid_cursor'
    | 'invalid_filter'
    | 'busy'
    | 'storage_unreadable'
    | 'schema_mismatch'
    | 'artifact_unavailable'
  message: string
  retryable: boolean
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

| Query | Return Type | Returns | Page |
|-------|-------------|---------|------|
| `list_processes(cursor, limit, filters)` | `QueryEnvelope<KeysetPage<ProcessListItem>>` | Keyset-paginated process list with pipeline state, coverage metrics, decision status | List view |
| `get_process_detail(process_id)` | `QueryEnvelope<ProcessDetailVm>` | Full process detail: notices, lots, candidates, quotes, costs, manifest, errors | Detail view |
| `list_sessions(cursor, limit, filters)` | `QueryEnvelope<KeysetPage<SessionListItem>>` | Active and recent agent sessions: session_id, skill_path, skill_version, execution_mode, status (lifecycle phase), started_at, duration, cleanup_verdict, error_type, exit_code, checkpoint_count, byte_count | Sessions view |
| `list_errors(cursor, limit, filters)` | `QueryEnvelope<KeysetPage<ErrorListItem>>` | Unresolved error manifests: error_manifest_id, process_id, pipeline_stage, error_type, error_message, severity (derived from error_type category: `critical`/`warning`/`info`), skill_path, created_at, resolved, agent_session_id | Errors view |

**Public API for `src-tauri/src/query_service.rs`:**

```typescript
type ProcessListFilters = {
  sourceIds?: string[]
  pipelineStates?: Array<'pending' | 'qualifying' | 'enriching' | 'qualified' | 'review' | 'rejected' | 'error'>
  decisionDispositions?: Array<'approve' | 'reject' | 'watch'>
  staleOnly?: boolean
  search?: string
}

type ProcessListItem = {
  processId: string
  canonicalProcessId: string
  title: string
  buyerName: string
  publishedAt: string
  deadlineAt: string | null
  pipelineState: 'pending' | 'qualifying' | 'enriching' | 'qualified' | 'review' | 'rejected' | 'error'
  pricedCoverageBps: number | null
  netQualifiedCoverageBps: number | null
  netQualifiedValueCoverageBps: number | null
  grossMarginBps: number | null
  netMarginBaseBps: number | null
  candidateCount: number
  quotedCandidateCount: number
  decision: { disposition: 'approve' | 'reject' | 'watch' | null; isStale: boolean; decidedAt: string | null }
}

type ProcessDetailVm = {
  processId: string
  canonicalProcessId: string
  notices: ContractNoticeVm[]
  lots: NoticeLotVm[]
  candidates: ProductCandidateVm[]
  quotes: SupplierQuoteVm[]
  landedCosts: LandedCostVm[]
  manifest: { markdown: string; basisHash: string; generatedAt: string; sections: string[] } | null
  coverage: {
    pricedCoverageBps: number
    landedCostCoverageBps: number
    netQualifiedCoverageBps: number
    pricedValueCoverageBps: number
    landedCostValueCoverageBps: number
    netQualifiedValueCoverageBps: number
    unresolvedMandatoryLots: number
  }
  decision: { disposition: 'approve' | 'reject' | 'watch' | null; isStale: boolean; decidedAt: string | null }
  errors: ErrorListItem[]
}

type SessionListFilters = {
  phases?: Array<'created' | 'spawned' | 'active' | 'terminating' | 'exited' | 'reaped' | 'archived'>
  executionModes?: Array<'pty' | 'contract'>
  activeOnly?: boolean
  search?: string
}

type SessionListItem = {
  sessionId: string
  daemonInstanceId: string
  skillPath: string
  skillVersion: string | null
  executionMode: 'pty' | 'contract'
  phase: 'created' | 'spawned' | 'active' | 'terminating' | 'exited' | 'reaped' | 'archived'
  startedAt: string | null
  endedAt: string | null
  durationMs: number | null
  cleanupVerdict: 'clean' | 'forced' | 'leaked' | null
  errorType: string | null
  exitCode: number | null
  checkpointCount: number
  byteCount: number
  reattachable: boolean
}

type ErrorListFilters = {
  severities?: Array<'critical' | 'warning' | 'info'>
  resolved?: boolean
  pipelineStages?: string[]
  search?: string
}

type ErrorListItem = {
  errorManifestId: string
  processId: string | null
  pipelineStage: string | null
  errorType: string
  errorMessage: string
  cleanupVerdict: 'clean' | 'forced' | 'leaked' | null
  severity: 'critical' | 'warning' | 'info'
  skillPath: string | null
  createdAt: string
  resolved: boolean
  agentSessionId: string | null
}

type SessionHistoryRequest = {
  sessionId: string
  fromSeq: number
  toSeq: number
  maxBytes?: number
}

type SessionHistoryChunk = {
  sessionId: string
  firstSeq: number
  lastSeq: number
  frames: SessionFrame[]
  availability: 'complete' | 'partial' | 'missing'
}
```

**SQLite concurrency rule:** SQLite is single-writer (daemon only) but supports many short readers. The QueryService opens read-only connections with short-lived transactions. Long-lived readers are prohibited because they prevent WAL checkpoints from draining. No transcript blobs or large artifacts in hot queries — those load on demand from the artifact store.

**L2-FR-QS.1 — QueryService Is Read-Only, Page-Scoped, and Non-Streaming**
- AC1: QueryService connections enable `PRAGMA query_only = ON` and never perform writes, schema changes, or generic SQL execution from frontend input.
- AC2: Each desktop query returns a page-specific read model rather than an unrestricted repository object graph.
- AC3: Hot queries exclude transcript blobs, artifact payloads, and long-lived streaming cursors.
- AC4: Query execution uses short-lived read transactions so WAL checkpoints are not starved by the desktop.

**L2-FR-QS.2 — Snapshot Envelopes and the Commit Fence Are Normative**
- AC1: Every QueryService response returns `QueryEnvelope<T>` with `data`, `snapshotSeq`, and `snapshotAt`.
- AC2: `QueryEnvelope<T>` contains snapshot data only; live daemon health is delivered separately through `ControlPlaneState`.
- AC3: The desktop treats a mutation as visible only when a subsequent snapshot satisfies `snapshotSeq >= visibleAfterSeq`.
- AC4: The UI does not claim a successful mutation has appeared in list or detail views before the commit-fence condition is satisfied.

**L2-FR-QS.3 — The User-Visible Snapshot Clock Is `commit_seq`**
- AC1: `snapshotSeq` is sourced from the daemon-authored `commit_clock.commit_seq`, not from `PRAGMA data_version`.
- AC2: `PRAGMA data_version` may be used only for local cache invalidation within QueryService.
- AC3: Failed or rolled-back writes advance no user-visible snapshot clock and therefore produce no new visibility fence for the UI.

**L2-FR-QS.4 — `list_processes()` Returns a Stable Keyset Page**
- AC1: `list_processes(cursor, limit, filters)` returns `QueryEnvelope<KeysetPage<ProcessListItem>>`.
- AC2: Each item includes enough data for the list surface to render pipeline state, coverage metrics, and decision status without additional per-row queries.
- AC3: Empty-result queries return `items: []`, `nextCursor: null`, and `hasMore: false` rather than surfacing an error solely because no processes match the filter.
- AC4: Pagination semantics align with `L2-FR-PAG.1` and remain stable under live updates.

**L2-FR-QS.5 — `get_process_detail()` Returns a Complete Detail Read Model**
- AC1: `get_process_detail(process_id)` returns `QueryEnvelope<ProcessDetailVm>` for one process.
- AC2: The detail read model includes notices, lots, candidates, quotes, costs, manifest, and unresolved errors needed by the opportunity detail page.
- AC3: Zero-lot processes and processes with no product candidates return explicit empty arrays plus derived empty-state fields, not omitted keys or transport-level failures.
- AC4: Missing or unreadable large artifacts degrade to explicit operator-visible error/loading states without blocking the rest of the detail payload.

**L2-FR-QS.6 — `list_sessions()` Returns Canonical Session Metadata**
- AC1: `list_sessions(cursor, limit, filters)` returns `QueryEnvelope<KeysetPage<SessionListItem>>`.
- AC2: Each session item exposes the lifecycle phase, execution mode, skill identity, timing fields, cleanup verdict, error type, exit code, checkpoint count, and byte count listed in this section.
- AC3: Sessions that never reach `active` still appear with their canonical phase, failure context, and duration fields populated consistently enough for operator review.
- AC4: An empty session result set is represented as an empty page rather than as a transport or parsing error.

**L2-FR-QS.7 — `list_errors()` Returns the Operator Error Queue**
- AC1: `list_errors(cursor, limit, filters)` returns `QueryEnvelope<KeysetPage<ErrorListItem>>`.
- AC2: Each error item includes process linkage, pipeline stage, error type, error message, cleanup verdict (when available from linked session metadata), derived severity, skill path, creation time, resolution status, and session linkage.
- AC3: Severity derivation matches the rules defined for shared error rendering in `L2-FR-UI.6`.
- AC4: An empty unresolved-error set is represented as `items: []` with no synthetic warning state.

**L2-FR-QS.8 — QueryService Read Failures Are Explicit and Non-Mutating**
- AC1: SQLite read errors, schema-read failures, and busy-timeout exhaustion surface as explicit read failures rather than partial envelopes.
- AC2: QueryService failures do not mutate desktop state, synthesize optimistic rows, or advance any visibility fence.
- AC3: When storage remains readable but the daemon is unavailable, QueryService continues to serve browse-only snapshots under the bootstrap rules in `L2-FR-BOOT.3`.

**L2-FR-QS.9 — Process and Detail Read Models Are Fully Typed**
- AC1: `ProcessListItem` and `ProcessDetailVm` expose the fields listed in this section so list and detail pages do not need out-of-band joins.
- AC2: `pipelineState` includes the full desktop badge set, including `error`.
- AC3: `canonicalProcessId` is returned anywhere deep linking must survive later aliasing.

**L2-FR-QS.10 — Session and Error Read Models Are Fully Typed**
- AC1: `SessionListItem` and `ErrorListItem` expose the exact fields listed in this section.
- AC2: Session lifecycle phases use the seven canonical public phase names from Layer 1.
- AC3: Error rows include enough context for the errors page and shared error component to render without additional lookups.

**L2-FR-QS.11 — Query Filters Are Explicit and Validated**
- AC1: `ProcessListFilters`, `SessionListFilters`, and `ErrorListFilters` are the only supported frontend filter shapes in Layer 2.
- AC2: Invalid cursors or unsupported filter values surface `QueryServiceError` with `invalid_cursor` or `invalid_filter`.
- AC3: Filter validation happens before any partial result is returned.

**L2-FR-QS.12 — Session History Replay Uses an On-Demand Read Contract**
- AC1: `src-tauri/src/query_service.rs` exposes `get_session_history(request: SessionHistoryRequest) -> QueryEnvelope<SessionHistoryChunk>` for historical replay and gap recovery.
- AC2: History replay is on-demand and excluded from hot list/detail queries.
- AC3: `availability` reports `complete`, `partial`, or `missing` for the requested `fromSeq..toSeq` range so the viewer can distinguish full recovery from unresolved gaps.

**L2-FR-QS.13 — QueryService Errors Are Typed**
- AC1: QueryService failures surface `QueryServiceError` with the codes declared in this section.
- AC2: `not_found` is used for missing single-resource lookups such as unknown `processId` or `sessionId`.
- AC3: `artifact_unavailable` is used when replay/history payloads are no longer recoverable even though the metadata row exists.

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
type DaemonCommandMethod =
  | 'get_daemon_info'
  | 'trigger_ingestion'
  | 'trigger_enrichment'
  | 'capture_decision'
  | 'resolve_error'
  | 'stop_session'
  | 'send_session_input'
  | 'resize_session'
  | 'retry_job'
  | 'restart_daemon'
  | 'rerun_from_error'

type DaemonWireRequest =
  | {
      messageType: 'hello'
      desktopVersion: string
      supportedProtocolRange: { min: number; max: number }
      expectedSchemaEpoch: number
      lastSeenInstanceId?: string
    }
  | {
      messageType: 'command'
      requestId: string
      method: DaemonCommandMethod
      params: Record<string, unknown>
      idempotencyKey?: string   // required for mutations, omitted for pure diagnostic reads
    }
  | { messageType: 'subscribe'; requestId: string; topic: string; fromSeq?: number }
  | { messageType: 'unsubscribe'; requestId: string; topic: string }

type ActiveSessionSummary = {
  count: number
  ids: string[]
  states: string[]   // index-aligned with ids; preserves the Layer 1 handshake shape
}

type DaemonWireWelcome = {
  selectedProtocolVersion: number
  daemonVersion: string
  instanceId: string
  schemaEpoch: number
  commitSeq: number
  daemonHealth: { status: string }   // preserves Layer 1 `HealthState`
  mutationsAllowed: boolean
  activeSessions: ActiveSessionSummary
  reconnectOutcome: 'fresh_connection' | 'same_daemon_reconnect' | 'daemon_restart_invalidated'
  streamResumePermitted: boolean
}

type BridgeError = {
  code:
    | 'transport_unavailable'
    | 'protocol_range_mismatch'
    | 'schema_epoch_mismatch'
    | 'mutations_unavailable'
    | 'invalid_params'
    | 'not_found'
    | 'already_terminal'
    | 'not_interactive'
    | 'conflict'
    | 'busy'
    | 'artifact_unavailable'
  message: string
  retryable: boolean
  details?: Record<string, unknown>
}

type DaemonInfoSnapshot = {
  instanceId: string
  daemonVersion: string
  protocolVersion: number
  schemaEpoch: number
  socketPath: string
  descriptorPath: string
  stateRoot: string
  stateDbPath: string
  artifactRoot: string
  walBytes: number | null
  startedAt: string
  startupRecovery: {
    recoveredInterruptedSessions: number
    lastRecoveryAt: string | null
  }
}

type GetDaemonInfoParams = Record<string, never>
type TriggerIngestionParams = { sourceId: string }
type TriggerEnrichmentParams = { processId: string }
type CaptureDecisionParams = {
  processId: string
  disposition: 'approve' | 'reject' | 'watch'
  reviewedBasisHash: string
  reviewedManifestHash: string
  reviewedAtStageRunId: string
}
type ResolveErrorParams = { errorManifestId: string }
type StopSessionParams = { sessionId: string; mode: 'graceful' | 'force' }
type SendSessionInputParams = { sessionId: string; inputUtf8: string }
type ResizeSessionParams = { sessionId: string; cols: number; rows: number }
type RetryJobParams = { jobId: string }
type RestartDaemonParams = Record<string, never>
type RerunFromErrorParams = { errorManifestId: string }
```

**Wire compatibility rule:** The daemon-facing wire contract stays additive relative to Layer 1. The first frame on the authenticated socket remains the L1 `hello`/`welcome` exchange with the original field names (`messageType`, `selectedProtocolVersion`, `daemonVersion`, `instanceId`, `schemaEpoch`, `commitSeq`, `mutationsAllowed`, `activeSessions`, `reconnectOutcome`, `streamResumePermitted`). `ControlPlaneState` is assembled in the desktop bridge from local storage checks plus the welcome payload and subsequent `daemon.health` events; it is not the raw Layer 1 welcome payload.

**Handshake:** First wire message from desktop is always `hello`. Daemon responds with `DaemonWireWelcome`. If `lastSeenInstanceId` differs from `instanceId`, the desktop knows the daemon restarted — it must invalidate all cached state and not attempt to resume PTY streams from previous sessions.

**Layer-1 cross-check:** `crates/daemon/src/ipc/handshake.rs` currently accepts `Hello` and rejects mutation traffic with `mutations_unavailable`. Layer 2 extends that handshake-only socket into the typed request model in this section by adding command dispatch and stream subscription on the same authenticated local IPC channel.

```typescript
type DaemonHealthPayload = {
  status: string
  mutationsAllowed: boolean
  activeSessions: ActiveSessionSummary
  reasons?: string[]
}

type JobProgressPayload = {
  jobId: string
  processId?: string
  state: 'pending' | 'running' | 'completed' | 'failed' | 'retry'
  stage?: string
  visibleAfterSeq?: number
}

type BridgeEvent =
  | { topic: 'daemon.health'; payload: DaemonHealthPayload }
  | { topic: string; payload: JobProgressPayload }
  | { topic: string; seq: number; kind: 'pty' | 'stdout' | 'stderr' | 'event' | 'lifecycle' | 'gap'; payload: unknown }
```

**Commands available in Layer 2:**

| Command | Requirement ID | Params | Returns | Idempotency | Daemon Handler | Failure Path |
|---------|----------------|--------|---------|-------------|----------------|--------------|
| `get_daemon_info` | `L2-FR-DB.21` | `GetDaemonInfoParams` | `DaemonInfoSnapshot` | Pure read command; no idempotency key required | `ipc/handlers/control.rs::get_daemon_info` | If the daemon is unavailable, `DaemonStatus` falls back to descriptor-backed stale diagnostics and marks live-only fields unavailable |
| `trigger_ingestion` | `L2-FR-DB.4` | `TriggerIngestionParams` | `MutationAck` with `jobId` | Required `idempotencyKey` dedupes repeated start requests | `ipc/handlers/jobs.rs::trigger_ingestion` via `ipc/command_router.rs` | Unknown source, disabled control mode, or daemon transport failure surfaces explicit refusal/failure; no ghost job row |
| `trigger_enrichment` | `L2-FR-DB.5` | `TriggerEnrichmentParams` | `MutationAck` with `jobId` | Required `idempotencyKey` dedupes repeated reruns | `ipc/handlers/jobs.rs::trigger_enrichment` | Unknown process or non-rerunnable state surfaces explicit refusal/failure |
| `capture_decision` | `L2-FR-DB.6` | `CaptureDecisionParams` | `MutationAck` | Required `idempotencyKey` dedupes repeated local-intent writes | `ipc/handlers/decisions.rs::capture_decision` | Invalid disposition, unreadable process detail, or local write failure surfaces explicit refusal/failure |
| `resolve_error` | `L2-FR-DB.7` | `ResolveErrorParams` | `MutationAck` | Required `idempotencyKey` dedupes repeated resolve requests | `ipc/handlers/errors.rs::resolve_error` | Unknown error or storage failure surfaces explicit refusal/failure |
| `stop_session` | `L2-FR-DB.8` | `StopSessionParams` | `MutationAck` | Idempotent per terminal session state and request key | `ipc/handlers/sessions.rs::stop_session` | Unknown or already-terminal session surfaces explicit no-op/refusal semantics |
| `send_session_input` | `L2-FR-DB.9` | `SendSessionInputParams` | `MutationAck` | Not semantically replay-safe; duplicate requests are deduped only by explicit request key | `ipc/handlers/sessions.rs::send_session_input` | Non-PTY, detached, or terminal session surfaces explicit refusal |
| `resize_session` | `L2-FR-DB.10` | `ResizeSessionParams` | `MutationAck` | Latest duplicate wins for identical key/geometry pair | `ipc/handlers/sessions.rs::resize_session` | Non-PTY or detached session surfaces explicit refusal |
| `retry_job` | `L2-FR-DB.11` | `RetryJobParams` | `MutationAck` with new `jobId` | Required `idempotencyKey` dedupes repeated retry creation | `ipc/handlers/jobs.rs::retry_job` | Non-retryable or unknown job surfaces explicit refusal/failure |
| `restart_daemon` | `L2-FR-DB.12` | `RestartDaemonParams` | `MutationAck` (ack sent before restart) | Duplicate request keys collapse to one accepted restart intent | `ipc/handlers/control.rs::restart_daemon` | If restart cannot be accepted, the operator receives an explicit refusal before any disconnect |
| `rerun_from_error` | `L2-FR-DB.13` | `RerunFromErrorParams` | `MutationAck` with new `jobId` | Required `idempotencyKey` dedupes repeated rerun creation | `ipc/handlers/errors.rs::rerun_from_error` | Unknown or non-rerunnable error manifest surfaces explicit refusal/failure |

Without at least stop/input/retry/restart, the operator still needs terminal-level escape hatches — undermining the "control panel, not browser" promise.

**L2-FR-DB.1 — DaemonBridge Is a Thin RPC Proxy**
- AC1: DaemonBridge forwards commands to the daemon over UDS or named pipe and does not perform authoritative writes itself.
- AC2: Bridge state is derived from daemon handshakes, heartbeats, and command responses rather than from bridge-local mutation caches.
- AC3: Transport errors are surfaced distinctly from daemon refusals so the desktop can differentiate reconnect behavior from business-rule failures.

**L2-FR-DB.2 — Handshake Negotiates Compatibility and Restart Semantics**
- AC1: The first daemon-wire message is always `hello`, and the daemon responds with `DaemonWireWelcome` while preserving the Layer 1 welcome field names and meaning.
- AC2: `DaemonWireWelcome` includes protocol version, daemon version, `instanceId`, `schemaEpoch`, current `commitSeq`, daemon health summary, mutation allowance, active-session summary, reconnect outcome, and stream-resume permission.
- AC3: If the desktop-provided `lastSeenInstanceId` differs from the daemon `instanceId`, the desktop invalidates cached state and does not attempt to resume PTY streams from the prior daemon instance.
- AC4: Schema or protocol incompatibility is surfaced through the bootstrap modes in `L2-FR-BOOT.3`, not hidden behind generic connection failure text.

**L2-FR-DB.3 — `MutationAck` Means Durable, Idempotent Progress**
- AC1: `accepted: true` means the mutation has reached durable SQLite state with a `visibleAfterSeq`; it never means "queued in RAM only."
- AC2: Retrying the same logical mutation with the same `idempotencyKey` returns the same durable outcome rather than creating duplicate writes.
- AC3: `accepted: false`, `remoteState: conflicted`, or a populated `conflictReason` prevent the UI from claiming the mutation succeeded.
- AC4: A lost response after commit is recoverable by retry with the same `idempotencyKey`.

**L2-FR-DB.4 — `trigger_ingestion()` Creates an Observable Ingestion Job**
- AC1: Successful `trigger_ingestion(source_id)` returns `MutationAck` with `accepted: true`, `jobId`, and `visibleAfterSeq`.
- AC2: Refused ingestion attempts surface an explicit operator-visible reason when the source is unknown, mutations are limited, or the daemon is unavailable.
- AC3: The desktop does not synthesize a job row before a subsequent query satisfies `snapshotSeq >= visibleAfterSeq`.

**L2-FR-DB.5 — `trigger_enrichment()` Re-Runs One Process**
- AC1: Successful `trigger_enrichment(process_id)` returns a committed `MutationAck` with `jobId`.
- AC2: The command targets one process and does not silently fan out to unrelated work.
- AC3: Unknown or non-rerunnable processes surface an explicit refusal instead of a silent no-op.

**L2-FR-DB.6 — `capture_decision()` Persists the Operator Intent**
- AC1: Successful `capture_decision(process_id, disposition, basis)` returns a committed `MutationAck` tied to the local `pending_operator_intents` write.
- AC2: The command writes the reviewed-basis metadata defined in `L2-FR-DEC.1` and participates in the coalescing rules in `L2-FR-DEC.2`.
- AC3: Invalid disposition values or storage failures surface explicit refusal/failure without mutating the local overlay.

**L2-FR-DB.7 — `resolve_error()` Closes One Error Manifest**
- AC1: Successful `resolve_error(error_manifest_id)` returns a committed `MutationAck`.
- AC2: Repeating the same resolve action against an already-resolved error is idempotent and does not create duplicate state.
- AC3: Unknown error IDs or storage failures surface explicit refusal/failure.

**L2-FR-DB.8 — `stop_session()` Is an Explicit Operator Interrupt**
- AC1: `stop_session(session_id, mode)` supports both `graceful` and `force` modes for active attached sessions.
- AC2: Successful stop requests return a committed `MutationAck`, and lifecycle updates continue through the streaming surface.
- AC3: Requests against unknown or already-terminal sessions surface an explicit refusal or no-op outcome instead of pretending the stop was meaningful.

**L2-FR-DB.9 — `send_session_input()` Targets Attached PTY Sessions Only**
- AC1: Successful `send_session_input(session_id, bytes)` returns a committed `MutationAck` for an attached PTY-backed session.
- AC2: Contract-mode sessions, detached sessions, and already-terminal sessions reject input explicitly.
- AC3: The bridge does not silently drop bytes while returning success.

**L2-FR-DB.10 — `resize_session()` Resizes Attached PTYs Only**
- AC1: Successful `resize_session(session_id, cols, rows)` returns a committed `MutationAck` for an attached PTY-backed session.
- AC2: Non-PTY or detached sessions reject resize requests explicitly.
- AC3: Invalid geometry parameters are rejected before they can taint session rendering state.

**L2-FR-DB.11 — `retry_job()` Creates a New Job Identity**
- AC1: Successful `retry_job(job_id)` returns a committed `MutationAck` with a new `jobId`.
- AC2: The original failed job remains queryable for audit and is not overwritten in place.
- AC3: Non-retryable or unknown jobs surface explicit refusal/failure.

**L2-FR-DB.12 — `restart_daemon()` Acknowledges Before Disconnection**
- AC1: A successful restart request returns a committed `MutationAck` before the daemon intentionally disconnects.
- AC2: After restart, the next handshake exposes a new `instanceId`, and the desktop follows the recovery rules in `L2-FR-BOOT.4`.
- AC3: If the daemon cannot accept the restart, the operator receives an explicit refusal without an ambiguous disconnect.

**L2-FR-DB.13 — `rerun_from_error()` Restarts Work From a Failed Stage**
- AC1: Successful `rerun_from_error(error_manifest_id)` returns a committed `MutationAck` with a new `jobId`.
- AC2: The rerun starts from the failed stage represented by the error manifest rather than from an arbitrary earlier stage.
- AC3: Unknown or non-rerunnable error manifests surface explicit refusal/failure.

**L2-FR-DB.14 — Bridge Failure Paths Are Explicit and Operator-Visible**
- AC1: If the daemon is unreachable during a command, the bridge surfaces a transport failure and the desktop transitions into reconnect or browse-only handling instead of optimistic success.
- AC2: If a downstream job later fails because of an execution conflict, including an Agent Mail `file_reservation_paths` conflict, the operator sees the failure through the session/error surfaces rather than through a silent stall.
- AC3: Command refusal reasons are preserved verbatim enough for the operator to distinguish busy/unavailable/conflicted/not-rerunnable states.

**L2-FR-DB.15 — Layer 2 Extends the Handshake-Only IPC Envelope**
- AC1: The Layer 1 `hello`/`welcome` handshake remains the first exchange on the socket, including the existing `messageType` tagging and welcome field names.
- AC2: After a successful handshake, the same authenticated IPC channel accepts `command`, `subscribe`, and `unsubscribe` requests as specified here.
- AC3: The daemon-side protocol extension is additive relative to `crates/daemon/src/ipc/handshake.rs`; incompatible mutation traffic is no longer rejected once the L2 command router is active.

**L2-FR-DB.16 — Every Command Has a Typed Input Contract**
- AC1: The only valid parameter shapes are the TypeScript parameter objects declared in this section.
- AC2: Missing required fields or invalid enum values surface `BridgeError { code: 'invalid_params' }`.
- AC3: `capture_decision` requires reviewed-basis metadata so the UI cannot write underspecified intents.

**L2-FR-DB.17 — Command Dispatch Is Mechanically Mapped to Daemon Handlers**
- AC1: `crates/daemon/src/ipc/command_router.rs` performs method dispatch from the request envelope to the daemon handler functions listed in the command table.
- AC2: Handler ownership is split by domain (`jobs`, `decisions`, `errors`, `sessions`, `control`) rather than by one monolithic mutation function.
- AC3: Unknown methods surface `BridgeError { code: 'not_found' }` or an equivalent unsupported-method rejection without invoking any handler.

**L2-FR-DB.18 — Bridge Errors Are Typed and Retry-Aware**
- AC1: Command failures surface `BridgeError` with one of the codes declared in this section.
- AC2: `retryable` is true only when the operator or caller can safely retry without changing parameters.
- AC3: Conflict and busy conditions are distinguished from terminal validation errors so the desktop can present the right CTA.

**L2-FR-DB.19 — Bridge Request Kinds Form a Closed Wire Contract**
- AC1: The only valid daemon-wire request kinds in Layer 2 are `hello`, `command`, `subscribe`, and `unsubscribe`.
- AC2: Every request carries a client-generated correlation id (`requestId` on post-handshake traffic) suitable for logs and tests.
- AC3: Mutation `command` requests require both `method` and `idempotencyKey`; pure diagnostic reads such as `get_daemon_info` omit the idempotency key; `subscribe` may include `fromSeq`; `unsubscribe` targets one previously subscribed topic.

**L2-FR-DB.20 — Welcome Frames Carry the Full Control-Plane Snapshot**
- AC1: `DaemonWireWelcome` includes `selectedProtocolVersion`, `daemonVersion`, `instanceId`, `schemaEpoch`, `commitSeq`, `daemonHealth`, `mutationsAllowed`, `activeSessions`, `reconnectOutcome`, and `streamResumePermitted`.
- AC2: `activeSessions.count` matches both the `ids` cardinality and the parallel `states` cardinality on the wire.
- AC3: `mutationsAllowed: false` forces the desktop into a non-full-control mode even if the daemon is reachable.

**L2-FR-DB.21 — `get_daemon_info()` Completes the Daemon Diagnostics Surface**
- AC1: `get_daemon_info()` returns the runtime/descriptor data needed by `DaemonStatus.tsx` that is not present in `ControlPlaneState` alone: socket path, descriptor path, DB path, artifact root, WAL size, and startup-recovery summary.
- AC2: `get_daemon_info()` is read-only and does not perform SQLite mutations.
- AC3: If the daemon is unavailable, `DaemonStatus.tsx` degrades to descriptor-backed stale diagnostics plus explicit unavailable markers for live-only fields rather than querying SQLite or inventing defaults.

### 1.6 Channel\<T\> Live Streaming

Tauri's typed streaming mechanism for live session output from daemon to frontend.

**Session frame types:**

```typescript
type StructuredSessionEvent =
  | { type: 'started'; pid: number; pgid: number; sid: number; mode: 'pty' | 'contract' }
  | { type: 'checkpoint'; checkpointSeq: number; path: string }
  | { type: 'progress'; completed: number; total: number | null }
  | {
      type: 'auth_challenge'
      challengeKind: 'relogin' | 'mfa' | 'sso' | 'permission_prompt' | 'unknown'
      detail: string
      intervention?: { type: 'reveal_profile_path'; profilePath: string; reattachable: boolean }
    }
  | { type: 'warning'; code: string; detail: string }
  | { type: 'final_result_ready'; path: string }
  | { type: 'exited'; exitCode: number | null }

type SessionFrame =
  | { seq: number; kind: 'lifecycle'; state: string; cleanupVerdict?: 'clean' | 'forced' | 'leaked'; archiveSeq?: number; ts: string }
  | { seq: number; kind: 'pty'; chunk: Uint8Array; ts: string }
  | { seq: number; kind: 'stdout'; chunk: string; ts: string }
  | { seq: number; kind: 'stderr'; chunk: string; ts: string }
  | { seq: number; kind: 'event'; event: StructuredSessionEvent; ts: string }
  | { seq: number; kind: 'gap'; fromSeq: number; toSeq: number }

type SessionReplayRequest = {
  sessionId: string
  fromSeq: number
  toSeq: number
}
```

**Event projection rule:** `StructuredSessionEvent` is the JSON-serializable projection of the Layer 1 `ControlEvent` union (`Started`, `Checkpoint`, `Progress`, `AuthChallenge`, `Warning`, `FinalResultReady`, `Exited`). No additional semantic event kinds are introduced in Layer 2 without a same-edit update to both this public union and the daemon adapter boundary.

**Sequence rule:** `SessionFrame.seq` is the per-session transport sequence assigned by the daemon/bridge capture pipeline. It is **not** the same field as `checkpointSeq` inside `StructuredSessionEvent { type: 'checkpoint' }`. `checkpointSeq` remains the skill/parser checkpoint number from Layer 1; `SessionFrame.seq` is the delivery/replay cursor.

**Two subscription levels:**

| Level | What | Cost | Used By |
|-------|------|------|---------|
| All running sessions (metadata only) | Status, checkpoint count, last line excerpt, elapsed time | Cheap — session list can show 20+ concurrent sessions | Session list page |
| Selected session (full stream) | Full PTY/stdout/stderr via `Channel<T>` | Expensive — at most 2-3 sessions attached simultaneously | Session detail viewer |

**Buffering strategy:**
- Viewer batches received frames every 33-50ms before writing to the terminal component; this is a client-side render policy, not a daemon durability or ordering contract
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

**Browser-auth challenge handling:** Some MFA/CAPTCHA flows are browser-only and cannot be resolved via PTY input. For Layer 2, the **minimum required** intervention path is reveal-profile-path plus clear reattach guidance surfaced from session metadata or the structured auth-challenge event. `open_managed_browser_session(session_id)` is forward-compatible but optional in Layer 2; the operator must never be left with an un-actionable auth-challenge banner.

**L2-FR-CH.1 — Session Streams Use a Typed, Sequenced Frame Contract**
- AC1: Every session frame carries a monotonic `seq` and a stable `kind` drawn from the `SessionFrame` union defined in this section.
- AC2: `gap` frames explicitly identify the missing inclusive sequence range as `fromSeq..toSeq`.
- AC3: The frontend does not invent or infer missing stream content outside the typed replay contract.

**L2-FR-CH.2 — Subscription Cost Is Bounded by View Intent**
- AC1: The session list surface subscribes only to metadata-level updates for all running sessions.
- AC2: Full PTY/stdout/stderr attachment is reserved for the selected session viewer, with the expected concurrency bounded to a small number of attached sessions.
- AC3: Metadata subscriptions remain cheap enough that session-list monitoring does not require transcript attachment for every session.

**L2-FR-CH.3 — Gap Recovery and Backpressure Handling Are Testable**
- AC1: Live terminal writes are batched every 33-50ms before terminal rendering.
- AC2: The hot buffer is bounded by line or byte limits, and scrolling up transitions to lazy-loaded historical mode instead of unbounded live accumulation.
- AC3: If the UI falls behind, a subscription drops, or a consumer misses frames, the stream emits a `gap` frame and `SessionViewer` attempts replay from the chunk index (`stream.index.jsonl`).
- AC4: If replay cannot fill the gap, the viewer surfaces an explicit gap/unavailable state instead of silently pretending the transcript is complete.

**L2-FR-CH.4 — Live-to-Historical Handoff Obeys the Archive Visibility Fence**
- AC1: On archive, the daemon emits a lifecycle frame carrying `archiveSeq`.
- AC2: `SessionRegistry` records `archiveSeq`, and `SessionViewer` waits for a snapshot with `snapshotSeq >= archiveSeq` before switching to historical mode.
- AC3: The viewer never switches to historical mode early enough to create a "finished but missing from sessions page" seam.

**L2-FR-CH.5 — Desktop Reconnect Replays From the Last Observed Sequence**
- AC1: If the desktop reconnects while the daemon instance is unchanged, the handshake returns the active-session summary and the viewer resubscribes from persisted `lastSeq`.
- AC2: Missing frames are recovered through the same `gap` and chunk-replay contract before live streaming resumes.
- AC3: Desktop reconnect does not discard viable live PTY sessions solely because the UI was closed and reopened.

**L2-FR-CH.6 — Daemon Restart Invalidates PTY Resumption**
- AC1: If handshake `instanceId` differs from the prior daemon instance, PTY streams from the previous instance are treated as non-resumable.
- AC2: After daemon restart, `SessionViewer` shows historical artifacts only for sessions owned by the prior daemon instance.
- AC3: The desktop invalidates cached attachability state on daemon restart instead of presenting stale live controls.

**L2-FR-CH.7 — Session Viewing Matches the Execution Mode**
- AC1: PTY-mode sessions default to the live terminal viewer with ANSI-faithful rendering, resize support, and optional interactive input.
- AC2: Contract-mode sessions default to the transcript/history viewer with search and lazy-loaded chunks.
- AC3: Input controls are clearly disabled for sessions that are non-interactive, detached, or no longer attached to a live PTY.

**L2-FR-CH.8 — Browser-Only Auth Challenges Have an Actionable Operator Path**
- AC1: When a session hits an auth or CAPTCHA flow that cannot be satisfied through PTY input, the desktop exposes a concrete intervention path. Reveal-profile-path plus reattach guidance is the minimum required Layer 2 contract; `open_managed_browser_session(session_id)` remains optional.
- AC2: The session surface never leaves the operator with an unbounded spinner or ambiguous stalled state for a browser-only challenge.
- AC3: The operator-visible instructions distinguish PTY-resolvable auth prompts from browser-only intervention requirements.

**L2-FR-CH.9 — Gap Frames Have Deterministic Trigger Conditions**
- AC1: The daemon or bridge emits `gap { fromSeq, toSeq }` when a subscriber requests `fromSeq` older than the live buffer can satisfy, when discontinuity is detected in delivered frame sequence, or when backpressure forces frame dropping.
- AC2: `fromSeq` is the first missing frame and `toSeq` is the last missing frame in the unresolved contiguous range.
- AC3: A test can induce a missing range such as `100..150` and expect exactly one unresolved gap range covering that interval before replay.

**L2-FR-CH.10 — Gap Recovery Uses an Explicit Replay Request**
- AC1: On `gap`, `SessionViewer` issues `get_session_history(SessionReplayRequest)` using the same `sessionId`, `fromSeq`, and `toSeq` range.
- AC2: Replay merges recovered frames back into the live session buffer in sequence order before live tailing resumes.
- AC3: Successful replay does not duplicate already-present frames before or after the requested range.

**L2-FR-CH.11 — Missing Replay Chunks Produce an Explicit Unresolved-Gap State**
- AC1: If the chunk index or artifact store cannot satisfy the full requested replay range, QueryService returns `availability: 'partial' | 'missing'`.
- AC2: The session viewer renders an explicit unresolved-gap marker covering the missing sequence range rather than pretending the transcript is continuous.
- AC3: Live streaming may continue after the unresolved gap marker, but the historical loss remains visible to the operator.

**L2-FR-CH.12 — Stream Topics Are Stable and Testable**
- AC1: `daemon.health` is the canonical control-plane topic.
- AC2: Session metadata and full-session streaming use stable topic families that are derivable from session identity rather than anonymous ad hoc topic strings.
- AC3: Subscription tests can bind deterministically to the same topic names across reconnects.

**L2-FR-CH.13 — Session Sequence Numbers Are Monotonic Per Session**
- AC1: For a given session stream, `seq` values are strictly increasing across live and replayed frames.
- AC2: Replay frames preserve the original sequence numbers rather than being renumbered client-side.
- AC3: Sequence monotonicity is scoped per session rather than globally across unrelated sessions.

**L2-FR-CH.14 — Structured Session Events Stay Aligned With the Layer 1 Adapter Boundary**
- AC1: `StructuredSessionEvent` is the public JSON projection of the Layer 1 `ControlEvent` variants and remains a closed union in Layer 2.
- AC2: `checkpointSeq` inside `StructuredSessionEvent` is distinct from `SessionFrame.seq` and is never reused as the replay cursor.
- AC3: Lifecycle frames (`created`, `terminating`, `reaped`, `archived`) remain separate from adapter-semantic event frames in the public stream contract.

**L2-FR-CH.15 — Render Batching Is Client-Side and Order-Preserving**
- AC1: The 33-50ms batching rule applies at the viewer/render layer, not at the daemon durability boundary.
- AC2: Batching never reorders frames or changes gap detection semantics.
- AC3: Tests may tighten or disable viewer batching without changing the public stream payload shape.

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
| **Control limited** | `storage=ready`, `daemon=starting\|ready\|degraded`, and (`control=limited\|read_only` OR `load=busy\|saturated` OR `daemon=starting\|degraded`) | Live status remains available, but some or all mutations may be refused. Banner: "Daemon limited — some operations unavailable." |
| **Browse-only** | `storage=ready`, `daemon=unavailable` | Read-only local browsing: list, detail, manifests, historical transcripts. No triggers, no live sessions. Banner: "Daemon unavailable — read-only local snapshot." |
| **Incompatible** | `daemon=incompatible` OR `storage=schema_mismatch` | Protocol/schema mismatch. Options: restart daemon, update desktop, or enter browse-only. |
| **Recovery required** | `storage=unreadable\|repairing` | DB inaccessible or corrupt. Repair/update screen. |

**Mode-derivation precedence (total mapping):**
1. `storage in {unreadable, repairing}` → **Recovery required**
2. else if `storage = schema_mismatch` OR `daemon = incompatible` → **Incompatible**
3. else if `daemon = unavailable` → **Browse-only**
4. else if `daemon in {starting, degraded}` OR `control in {limited, read_only}` OR `load in {busy, saturated}` → **Control limited**
5. else → **Full control**

This precedence makes every reachable combination of `storage × daemon × control × load` map to exactly one mode. In particular, `daemon=starting` is a transient limited mode, not a sixth bootstrap state.

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

**L2-FR-BOOT.3 — Bootstrap Derives One of Five Operator Modes**
- AC1: Launch evaluation considers DB readability/schema state and daemon handshake results before declaring a runtime mode.
- AC2: The desktop derives exactly one of the five modes defined in this section: full control, control limited, browse-only, incompatible, or recovery required.
- AC3: Each mode presents the UX contract listed in this section rather than leaving controls ambiguously enabled.
- AC4: Browse-only mode remains usable for local snapshot browsing when storage is readable but the daemon is unavailable.

**L2-FR-BOOT.4 — Mid-Session Daemon Failure Has a Defined Recovery Path**
- AC1: If heartbeats or streams drop mid-session, the desktop shows reconnect state and attempts one automatic daemon restart.
- AC2: On reconnect, the desktop performs a fresh handshake and applies `instanceId` restart rules before resuming any session UI.
- AC3: The desktop never attempts PTY resumption across a daemon restart; orphan reconciliation remains daemon-owned.
- AC4: If restart fails, the desktop falls back to browse-only mode with clear operator messaging.

**L2-FR-BOOT.5 — Window Close and Daemon Stop Are Separate Operator Actions**
- AC1: "Quit UI" leaves the daemon running.
- AC2: "Stop background daemon and quit" performs an explicit clean daemon shutdown before exiting the desktop shell.
- AC3: The operator can distinguish these two actions clearly in the UI.

**L2-FR-BOOT.6 — Desktop Startup Auto-Launches the Daemon When Possible**
- AC1: Bootstrap performs the startup sequence in the order listed here: DB checks, descriptor read, handshake attempt, detached daemon launch if needed, retry, then mode fallback.
- AC2: The operator is not required to start the daemon manually for normal desktop use.
- AC3: Protocol or schema incompatibility enters incompatible mode explicitly rather than masquerading as a generic transport failure.
- AC4: If no daemon becomes available after the documented retry path, the desktop enters browse-only mode rather than crashing or hanging indefinitely.

**L2-FR-BOOT.7 — `ControlPlaneState` Is the Canonical Runtime Health Model**
- AC1: `ControlPlaneState` includes the fields and enum domains defined in this section.
- AC2: `reasons` contains human-readable explanations suitable for banners and status panes.
- AC3: UI state decisions are derived from `ControlPlaneState`, not from ad hoc combinations of unrelated booleans.

**L2-FR-BOOT.8 — Descriptor Read Precedes Detached Launch**
- AC1: Bootstrap reads `daemon.info.json` before attempting detached daemon launch.
- AC2: A stale descriptor alone does not count as a healthy daemon; a successful handshake is still required.
- AC3: Descriptor-read failure is recoverable when the daemon can still be launched and handshaken successfully.

**L2-FR-BOOT.9 — Auto-Launch Is Single-Flight**
- AC1: The desktop attempts detached daemon launch at most once per startup sequence before falling back to browse-only.
- AC2: Retry handshake uses bounded backoff rather than a busy loop.
- AC3: Multiple windows or repeated bootstrap transitions do not fan out into duplicate daemon launches.

**L2-FR-BOOT.10 — Browse-Only Mode Has a Narrow, Explicit Capability Set**
- AC1: Browse-only mode permits snapshot list/detail/error browsing and historical transcript access only.
- AC2: Trigger, decide, stop, input, resize, retry, and restart controls are either hidden or visibly disabled.
- AC3: The operator sees a banner explaining that the snapshot is local and read-only because the daemon is unavailable.

**L2-FR-BOOT.11 — Runtime-Mode Derivation Is Total and Non-Overlapping**
- AC1: The precedence rules in this section map every reachable `ControlPlaneState` combination to exactly one UX mode.
- AC2: `daemon=starting` is treated as a limited mode rather than as an implicit crash or unavailable state.
- AC3: No UI branch infers mode from ad hoc booleans outside the documented precedence rules.

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
- `bun run dev` starts Vite with the middleware plugin active
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

**L2-FR-DEV.1 — Dev Browser Invokes the Same Query and Command Surface**
- AC1: In browser dev mode, `/__tauri_invoke__` proxies the same query and mutation commands that the production Tauri bridge exposes.
- AC2: Query semantics, mutation semantics, and payload shapes match the production bridge closely enough that Playwright tests exercise the same application contract.
- AC3: The dev bridge does not create a second business-logic path that bypasses QueryService or DaemonBridge rules.

**L2-FR-DEV.2 — Dev Browser Streaming Matches the Session Frame Contract**
- AC1: The WebSocket relay at `/__ws__` forwards `SessionFrame` and `BridgeEvent` payloads using the same typed contracts as the Tauri bridge.
- AC2: Dev-browser session stores reuse the same gap-recovery and replay semantics defined for production live streams.
- AC3: A dropped dev WebSocket is recoverable through reconnect plus `lastSeq` replay rather than requiring a full page reset.

**L2-FR-DEV.3 — Transport Selection Is Centralized in One Runtime Shim**
- AC1: Frontend hooks and stores call through `src/lib/transport.ts` instead of importing Tauri APIs directly.
- AC2: Production webview execution chooses the native bridge when `window.__TAURI_INTERNALS__` is present.
- AC3: Browser dev execution chooses the HTTP/WebSocket fallback only when the Tauri bridge is absent.

**L2-FR-DEV.4 — The Dev Bridge Is Local-Only and Never Shipped**
- AC1: The middleware binds to `127.0.0.1` only.
- AC2: Production builds exclude the dev bridge code path from shipped desktop artifacts.
- AC3: The dev bridge exists solely for local development and testing and is not presented as a production or remote-access transport.

### 1.9 Desktop File Listing

**`apps/desktop/` — 18 files (16 shipped + 2 dev-only):**

| File | Responsibility | Requirement IDs |
|------|---------------|-----------------|
| `src-tauri/src/main.rs` | Tauri Rust bootstrap: DB check, daemon handshake, QueryService + DaemonBridge init | `L2-FR-BOOT.1`, `L2-FR-BOOT.3`, `L2-FR-BOOT.6` |
| `src-tauri/src/query_service.rs` | Read-only SQLite queries returning `QueryEnvelope<T>` | `L2-FR-QS.1`-`L2-FR-QS.13`, `L2-FR-PAG.1`, `L2-FR-PAG.2` |
| `src-tauri/src/daemon_bridge.rs` | RPC proxy to daemon over UDS/named-pipe; handshake/wire adaptation; `Channel<T>` relay | `L2-FR-DB.1`-`L2-FR-DB.21`, `L2-FR-CH.1`-`L2-FR-CH.15` |
| `src-tauri/capabilities/default.json` | Allowed shell/sidecar commands | `L2-FR-SEC.2` |
| `src-tauri/tauri.conf.json` | Sidecar and desktop build configuration | `L2-FR-BOOT.1`, `L2-FR-SEC.2`, `L2-FR-DEV.4` |
| `src/App.tsx` | Root React component with routing and bootstrap state | `L2-FR-BOOT.3`, `L2-FR-BOOT.5`, `L2-FR-STATE.1`, `L2-FR-PAGE.7` |
| `src/pages/ContractList.tsx` | Keyset-paginated process list with pipeline badges and decision status | `L2-FR-PAGE.1`, `L2-FR-PAGE.8`, `L2-FR-QS.4`, `L2-FR-COV.1`, `L2-FR-PAG.1` |
| `src/pages/ContractDetail.tsx` | Full opportunity detail: candidates, quotes, margins, manifest, errors | `L2-FR-PAGE.2`, `L2-FR-PAGE.9`, `L2-FR-QS.5`, `L2-FR-COV.1`, `L2-FR-DEC.3` |
| `src/pages/Sessions.tsx` | Active/recent agent session list with metadata | `L2-FR-PAGE.3`, `L2-FR-PAGE.10`, `L2-FR-QS.6`, `L2-FR-CH.2` |
| `src/pages/SessionViewer.tsx` | Live terminal/transcript viewer with dual mode (PTY/contract) | `L2-FR-PAGE.4`, `L2-FR-PAGE.11`, `L2-FR-CH.3`-`L2-FR-CH.15`, `L2-FR-SEC.1`, `L2-FR-A11Y.1` |
| `src/pages/Errors.tsx` | Unresolved error manifest list with drill-down | `L2-FR-PAGE.5`, `L2-FR-PAGE.12`, `L2-FR-QS.7`, `L2-FR-DB.7`, `L2-FR-UI.6` |
| `src/stores/session-registry.ts` | External store: tracks all active/recent sessions with metadata; keyed by `daemonInstanceId + sessionId`; tracks `lastSeq`, `gapState`, `archiveSeq`, `reattachable` | `L2-FR-STATE.1`, `L2-FR-STATE.3`-`L2-FR-STATE.8`, `L2-FR-CH.4`-`L2-FR-CH.6`, `L2-FR-CH.10`, `L2-FR-CH.11` |
| `src/stores/session-buffer.ts` | External store: per-session buffer — PTY mode stores bytes (not lines), contract mode stores text frames; explicit gap markers | `L2-FR-STATE.2`, `L2-FR-STATE.7`, `L2-FR-CH.3`, `L2-FR-CH.7` |
| `src/hooks/use-daemon.ts` | `useSessionMeta()`, `useSessionTail()`, `useSessionTimeline()` — slice subscriptions via `useSyncExternalStore` | `L2-FR-STATE.3`, `L2-FR-CH.2`, `L2-FR-CH.7` |
| `src/pages/DaemonStatus.tsx` | Daemon operations: instance id/version, socket path, DB path, artifact root, WAL size, startup/recovery info, restart button, open logs/artifacts | `L2-FR-PAGE.6`, `L2-FR-PAGE.13`, `L2-FR-BOOT.4`, `L2-FR-DB.12`, `L2-FR-DB.21` |
| `src/hooks/use-control-plane.ts` | `useControlPlaneState()` — daemon health separate from query snapshots; consumed via `useSyncExternalStore` | `L2-FR-STATE.1`, `L2-FR-STATE.6`, `L2-FR-BOOT.3` |
| `src/lib/transport.ts` | **Dev + shipped:** Runtime shim — selects Tauri native bridge or HTTP fallback (see §1.8) | `L2-FR-DEV.1`-`L2-FR-DEV.4`, `L2-FR-STATE.4` |
| `vite-plugin-tauri-dev-bridge.ts` | **Dev-only (not shipped):** Vite plugin — HTTP proxy for `invoke()` + WebSocket relay for streaming (see §1.8) | `L2-FR-DEV.1`, `L2-FR-DEV.2`, `L2-FR-DEV.4` |

**L2-FR-PAGE.1 — Contract List Surfaces the Daily Opportunity Queue**
- AC1: The list page renders `list_processes()` results with keyset pagination, pipeline badges, coverage summaries, and current decision status.
- AC2: Empty-result states render explicitly with filter-aware empty messaging rather than a blank screen.
- AC3: Live updates do not cause row jumping or focus loss while the operator is reading.

**L2-FR-PAGE.2 — Contract Detail Surfaces the Full Review Packet**
- AC1: The detail page renders notices, lots, candidates, quotes, margin outputs, manifest, and unresolved errors from one detail read model.
- AC2: Zero-lot processes and processes with no candidates render explicit empty states without breaking the rest of the page.
- AC3: Decision status and staleness are visible on the detail surface without requiring a separate page hop.

**L2-FR-PAGE.3 — Sessions Page Surfaces Active and Recent Sessions**
- AC1: The sessions page renders active and recent sessions from `list_sessions()` with lifecycle phase, execution mode, timing, cleanup, and failure metadata.
- AC2: Sessions that fail before reaching `active` still appear in the list with canonical state and error context.
- AC3: The page links cleanly into `SessionViewer` without requiring raw transcript attachment for every row.

**L2-FR-PAGE.4 — Session Viewer Supports Monitoring and Intervention**
- AC1: The viewer renders live PTY or transcript history according to the session execution mode.
- AC2: Interactive controls are available only when the session is attached and the underlying mode supports them.
- AC3: Gap markers, reconnect state, and browser-only auth interventions are surfaced explicitly to the operator.

**L2-FR-PAGE.5 — Errors Page Is the Operator Error Work Queue**
- AC1: The errors page renders unresolved manifests with severity, stage, summary, and session linkage.
- AC2: The page supports drill-down and resolve actions for desktop-authorized operators.
- AC3: Resolution results follow the `MutationAck` visibility fence instead of optimistic UI removal.

**L2-FR-PAGE.6 — Daemon Status Exposes Operator Diagnostics**
- AC1: The daemon status page shows instance id, daemon version, socket path, DB path, artifact root, WAL size, and startup/recovery context.
- AC2: The page exposes an explicit restart action and affordances to open logs or artifacts.
- AC3: Status data remains readable in limited/browse-only contexts whenever either the live daemon or the descriptor-backed fallback source is available.

**Public API for key page files from `apps/desktop/src/pages/*`:**

```typescript
type AppRoutes =
  | '/'
  | `/process/${string}`
  | '/sessions'
  | `/sessions/${string}`
  | '/errors'
  | '/daemon'

type ContractListPageQuery = {
  after?: string
  limit?: 25 | 50
  search?: string
  sourceId?: string
  pipelineState?: ProcessListFilters['pipelineStates'][number]
  decision?: ProcessListFilters['decisionDispositions'][number]
  staleOnly?: 'true' | 'false'
}

type SessionViewerRoute = {
  sessionId: string
  initialMode?: 'live' | 'history'
}
```

**L2-FR-PAGE.7 — `src/App.tsx` Owns the Route Table and Bootstrap Gating**
- AC1: `src/App.tsx` exposes the six route shapes listed in this section.
- AC2: Route availability and action enablement are gated by bootstrap/runtime mode.
- AC3: Unknown routes resolve to an operator-visible not-found state rather than a blank shell.

**L2-FR-PAGE.8 — `ContractList.tsx` Has an Explicit Query and Navigation Contract**
- AC1: The list page consumes `ContractListPageQuery`, normalizes it into `ProcessListFilters`, and calls `list_processes()`.
- AC2: Row selection navigates by canonical process identity into `/process/:processId`.
- AC3: Pagination controls operate solely through keyset cursors and never via offset indices.

**L2-FR-PAGE.9 — `ContractDetail.tsx` Has an Explicit Detail and Action Contract**
- AC1: The detail page reads one `processId` route param and calls `get_process_detail(processId)`.
- AC2: The page is the desktop surface that exposes decision capture and per-process rerun/re-enrichment actions.
- AC3: Unknown `processId` values render a not-found state distinct from storage failure.

**L2-FR-PAGE.10 — `Sessions.tsx` Has an Explicit Filter and Drill-Down Contract**
- AC1: The sessions page normalizes its filter UI into `SessionListFilters` and calls `list_sessions()`.
- AC2: Selecting a row navigates into `/sessions/:sessionId`.
- AC3: The page can render active-only, phase-filtered, and execution-mode-filtered views without transcript attachment for every row.

**L2-FR-PAGE.11 — `SessionViewer.tsx` Has an Explicit Replay and Intervention Contract**
- AC1: The viewer consumes `SessionViewerRoute`, session meta hooks, and on-demand replay via `get_session_history()`.
- AC2: The viewer invokes only the session-scoped commands relevant to a live attached session: stop, input, and resize.
- AC3: Unknown `sessionId` values and unrecoverable artifact loss render explicit viewer error states.

**L2-FR-PAGE.12 — `Errors.tsx` Has an Explicit Queue and Resolution Contract**
- AC1: The errors page normalizes its filter UI into `ErrorListFilters` and calls `list_errors()`.
- AC2: The page exposes resolve and rerun actions for rows where those actions are valid.
- AC3: Unknown or already-resolved rows remain distinguishable in UI state after refetch.

**L2-FR-PAGE.13 — `DaemonStatus.tsx` Has an Explicit Diagnostic Action Contract**
- AC1: The page consumes `useControlPlaneState()` for live health and `get_daemon_info()` for descriptor/runtime diagnostics that are not present in `ControlPlaneState`.
- AC2: The page may invoke `restart_daemon()` but does not directly mutate unrelated runtime state.
- AC3: Desktop-only affordances such as opening logs or artifacts are absent from shared UI packages.

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

**Public API for `src/lib/transport.ts` and `src/stores/session-registry.ts`:**

```typescript
type TransportApi = {
  invoke<T>(method: string, args?: Record<string, unknown>): Promise<T>
  subscribe(topic: string, fromSeq: number | undefined, onEvent: (event: BridgeEvent) => void): () => void
}

type SessionRegistryState = {
  daemonInstanceId: string | null
  sessions: Record<string, SessionListItem>
  lastSeqBySession: Record<string, number>
  gapStateBySession: Record<string, { fromSeq: number; toSeq: number } | null>
}

type SessionRegistryApi = {
  getSnapshot(): SessionRegistryState
  subscribe(onStoreChange: () => void): () => void
  resetForDaemon(instanceId: string): void
  applySessionMeta(session: SessionListItem): void
  applyStreamFrame(sessionId: string, frame: SessionFrame): void
  markReplayResolved(sessionId: string, fromSeq: number, toSeq: number): void
}
```

**L2-FR-STATE.1 — Snapshot, Streaming, and Control-Plane State Stay Separated**
- AC1: Snapshot server-state uses TanStack Query only for QueryService reads.
- AC2: Live session state and control-plane state live in purpose-built external stores rather than inside the snapshot-query cache.
- AC3: The UI does not collapse daemon health into snapshot envelopes or transcript buffers into page-level query state.

**L2-FR-STATE.2 — Session Output Never Lives as Raw React Component State**
- AC1: PTY buffers store bytes rather than pre-expanded React-friendly line arrays.
- AC2: Contract-mode session history stores structured text/event frames with explicit gap markers.
- AC3: Transcript growth is bounded by external-store retention rules rather than by unbounded component-state accumulation.

**L2-FR-STATE.3 — Session Hooks Subscribe by Slice**
- AC1: `useSessionMeta()`, `useSessionTail()`, and `useSessionTimeline()` read from external stores through `useSyncExternalStore`.
- AC2: Slice subscriptions avoid rerendering unrelated session views on every frame.
- AC3: Reconnect and replay update the same external-store contract without requiring hook callers to manage transport details directly.

**L2-FR-STATE.4 — Transport and Session Registry APIs Are Stable**
- AC1: `src/lib/transport.ts` exports `invoke()` and `subscribe()` as the only frontend transport entry points.
- AC2: `src/stores/session-registry.ts` exports the registry methods listed in this section instead of exposing mutable store internals directly.
- AC3: Registry reset is keyed by daemon instance identity so reconnect and restart behavior remain distinct.

**L2-FR-STATE.5 — Replay State Is First-Class in the Registry**
- AC1: The registry persists `lastSeq` and per-session gap state.
- AC2: Applying replayed frames updates the same session record used by live streaming.
- AC3: Resolved and unresolved gap states are mechanically observable for tests and UI rendering.

**L2-FR-STATE.6 — Control-Plane Store Has a Minimal Public API**
- AC1: The control-plane store exposes `getSnapshot()`, `subscribe()`, `applyWelcome()`, and `applyHealthEvent()` or an equivalent minimal API.
- AC2: The store is the sole owner of current `ControlPlaneState`.
- AC3: Health updates do not require invalidating unrelated snapshot query caches.

**L2-FR-STATE.7 — Session Buffer Has an Explicit Frame API**
- AC1: The session buffer exposes append, replay-merge, gap-mark, and prune operations rather than leaking raw mutable arrays.
- AC2: PTY buffers operate on bytes while contract-mode buffers operate on structured text/event frames.
- AC3: Buffer pruning preserves enough recent context for tail views without violating the bounded-retention rule.

**L2-FR-STATE.8 — `lastSeq` Persistence Is Restart-Aware**
- AC1: The registry persists `lastSeq` keyed by `daemonInstanceId + sessionId`.
- AC2: A daemon instance change invalidates persisted replay anchors from the previous daemon.
- AC3: Desktop close/reopen with the same daemon retains replay anchors so reconnect is resumable.

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

| Component | Requirement ID | Purpose | Used By |
|-----------|----------------|---------|---------|
| `ContractCardFrame` | `L2-FR-UI.2`, `L2-FR-UI.7` | Process summary card with slot composition | Desktop list, web list (L4) |
| `ManifestViewer` | `L2-FR-UI.3`, `L2-FR-SEC.1` | Markdown manifest renderer with section navigation and HTML sanitization | Desktop detail, web detail (L4) |
| `PipelineBadge` | `L2-FR-UI.4`, `L2-FR-A11Y.2` | Pipeline state badge (qualified/review/rejected/pending) | Both surfaces |
| `FreshnessBadge` | `L2-FR-UI.5` | Data freshness indicator — accepts surface-neutral `FreshnessBadgeVM` (label, tone, tooltip); desktop and web compute the VM separately | Both surfaces |
| `ErrorSummaryList` | `L2-FR-UI.6`, `L2-FR-SEC.1` | Compact error manifest list with severity/resolution status | Both surfaces |

#### `error-summary-list.tsx` — Error Manifest List

```tsx
interface ErrorSummaryItem {
  errorManifestId: string
  processId: string | null
  pipelineStage: string | null
  errorType: string
  errorMessage: string
  cleanupVerdict?: 'clean' | 'forced' | 'leaked' | null
  severity: 'critical' | 'warning' | 'info'   // derived upstream from errorType + cleanupVerdict
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

**Severity derivation ownership:** QueryService or the page-level mapper derives `severity` before the shared component renders. `ErrorSummaryList` does not inspect raw error codes on its own.

**Severity derivation rule (explicit):**
- `critical`: `AGENT_CLEANUP_INCOMPLETE`, `AGENT_DAEMON_CRASH_DURING_SESSION`, `HOST_RESOURCE_EXHAUSTED`, `DAEMON_STARTUP_FAILED`, or any error row whose linked session records `cleanupVerdict = leaked`
- `warning`: all remaining `AGENT_*`, `HOST_SLEEP_INTERRUPTED`, all `SOURCE_*`, `AUTH_STATE_STALE`, `PAGE_SHAPE_DRIFT`, `LOCALE_OR_CURRENCY_DRIFT`, pipeline-quality errors such as `NOTICE_PARSE_FAILED` / `GOODS_CLASSIFICATION_LOW_CONFIDENCE` / `SUPPLIER_MATCH_LOW_CONFIDENCE` / `COMMODITY_CODE_LOW_CONFIDENCE` / `REVENUE_BASIS_UNCERTAIN` / `FPO_ACCESS_UNAVAILABLE`, and `SQLITE_WRITE_FAILED`
- `info`: reserved for future non-error advisories; L2 `error_manifests` rows do not currently emit informational severities

**Sharing boundary:** Share down to view-models, presentational components, formatting/markdown/tables/badges. Stop sharing at repositories, hooks, realtime subscriptions, mutation flows.

**Security boundary:** Agent-authored content (manifests, error messages, session transcripts) is **untrusted input**:
- `ManifestViewer`: sanitize Markdown, disable raw HTML, strip `javascript:` and unsafe `file://` links
- `ErrorSummaryList`: sanitize error messages and context
- Error-manifest drill-down: render Markdown through the same sanitized pipeline as manifests; render JSON/error context in escaped text blocks, not raw HTML
- `SessionViewer`: never render transcript bytes via `dangerouslySetInnerHTML`; strip dangerous OSC 8/52 escape sequences before terminal paint
- Never render arbitrary embedded HTML from agent text

### 1.12 Manifest Rendering (`packages/manifests`)

**2 files:**

| File | Requirement ID | Purpose |
|------|----------------|---------|
| `render-opportunity.ts` | `L2-FR-MAN.1`, `L2-FR-MAN.2` | Deterministic opportunity manifest renderer — takes pipeline data, produces structured Markdown |
| `opportunity.md.ts` | `L2-FR-MAN.3` | Markdown template for opportunity manifests |

The canonical rendering lives here, not in the `manifest-generation` skill. The skill may contribute optional narrative synthesis, but the deterministic renderer is authoritative.

**`decisionBasisHash` ownership:** The hash is daemon-authored from a canonical serialization of operator-relevant fields, NOT computed by the manifest renderer or by a skill. The renderer consumes the already-computed hash. This prevents hash drift across Rust/TS/agent implementations.

**L2-FR-MAN.1 — `packages/manifests` Owns Deterministic Opportunity Rendering**
- AC1: The canonical opportunity manifest renderer lives in `packages/manifests`, not in a skill directory or UI surface.
- AC2: Skills may contribute optional narrative content, but they do not replace the deterministic renderer contract.
- AC3: The renderer consumes daemon-provided hashes and pipeline inputs without re-authoring authority-owned values.

**L2-FR-MAN.2 — `renderOpportunityManifest()` Has a Stable Typed Contract**
- AC1: The renderer accepts the `ManifestInput` fields listed in §2.2 and returns `ManifestOutput` with `markdown`, `basisHash`, `sections`, `blockerCount`, and `generatedAt`.
- AC2: `basisHash` is passed through from daemon-provided input and is not recomputed inside the renderer.
- AC3: Missing optional pipeline data degrades to explicit missing sections or empty tables inside the manifest output rather than hidden omission.

**L2-FR-MAN.3 — Opportunity Markdown Uses the Canonical Section Layout**
- AC1: Generated manifests contain the eight sections listed in `opportunity.md.ts`.
- AC2: Monetary and percentage formatting follow the rules defined in this section.
- AC3: Blockers and confidence bands are rendered in the operator-facing formats specified here rather than as raw internal numeric dumps.

**L2-FR-MAN.4 — Empty Data Still Produces Explicit Manifest Sections**
- AC1: Missing candidates, quotes, or landed-cost data produce explicit empty-state prose or empty tables in the relevant sections rather than silently removing the section.
- AC2: Zero-lot processes still render a summary plus explicit lot-analysis empty state.
- AC3: The manifest remains structurally navigable even when some sections contain no commercial evidence.

**L2-FR-MAN.5 — Manifest Navigation Uses Stable Parsed Sections**
- AC1: `sections` in `ManifestOutput` correspond to the generated heading structure.
- AC2: Sidebar navigation and section-copy affordances operate on those parsed sections.
- AC3: Print-friendly output preserves the same section ordering used for in-app navigation.

### 1.13 New Skill: Contracts Finder Ingestion

| Attribute | Value |
|-----------|-------|
| **Path** | `skills/procurement-ingestion/contracts-finder/` |
| **Purpose** | Ingest live notices from Contracts Finder |
| **Interaction profile** | `pty_interactive` |
| **API** | OCDS/Search API: `publishedFrom`, `publishedTo`, `stages`, `limit`, `cursor`. Rate limiting documented. |
| **Licensing** | Aligned with open contracting/publication standards |
| **Cursor strategy** | Date-based filtering + cursor; rate-limit-aware polling |
| **Packaging standard** | Must match the FTS skill structure and manifest instructions in `skills/procurement-ingestion/fts/SKILL.md` |

**Minimum skill contract (must meet the FTS quality bar):**
- **Use when:** The daemon needs incremental Contracts Finder notice retrieval for the daily source-batched ingestion loop.
- **Inputs:** `source_cursors` watermark plus optional `publishedFrom`, `publishedTo`, `stages`, `cursor`, and `limit` job parameters.
- **Outputs:** Structured notice batch, checkpoint-friendly partial progress, and `continuation_required` when safe remaining work exceeds the time budget.
- **Failure codes:** `SOURCE_CHANGED`, `AGENT_PROTOCOL_MISMATCH`, `SOURCE_SOFT_BLOCKED`, `SOURCE_CAPTCHA_BLOCKED`, `PAGE_SHAPE_DRIFT`, `LOCALE_OR_CURRENCY_DRIFT`, `AGENT_STALLED`.
- **Resume strategy:** Cursor backtrack window with small overlap so incremental runs never skip notices if Contracts Finder mutates during retrieval.
- **Canary:** Fetch the most recent notices, verify response shape/locale/currency/rate-limit expectations, and fail fast on drift before a full ingest run.
- **Run manifest instructions:** Same required YAML frontmatter and narrative sections as the FTS skill, so the daemon can finalize a durable run-manifest artifact consistently across procurement sources.

**Acquisition rule:** Layer 2 continues the project-wide skill-based retrieval rule. The Contracts Finder skill may drive the official OCDS/search endpoints, but the retrieval contract still lives in the skill package rather than in a long-lived bespoke client library.

**L2-FR-CF.1 — Contracts Finder Ingestion Is a First-Class Layer 2 Source Skill**
- AC1: Layer 2 adds a skill at `skills/procurement-ingestion/contracts-finder/`.
- AC2: The skill declares a `pty_interactive` interaction profile and targets the documented Contracts Finder search/API parameters in this section.
- AC3: Cursor handling is rate-limit-aware and stable enough for repeated daily ingestion windows.

**L2-FR-CF.2 — Contracts Finder Skill Packaging Matches the FTS Standard**
- AC1: The Contracts Finder skill includes the same structural sections as the FTS skill: use/do-not-use guidance, inputs, outputs, failure codes, resume strategy, canary, and run-manifest instructions.
- AC2: The skill supports checkpoint-safe continuation semantics rather than assuming one monolithic batch run.
- AC3: The skill documents its cursor overlap/backtrack behavior explicitly enough for the daemon to resume without source-level skips.

**L2-FR-CF.3 — Contracts Finder Canary and Manifest Rules Are Mandatory**
- AC1: The skill performs a lightweight canary before full retrieval and fails fast on response-shape or locale/currency drift.
- AC2: The skill emits the same run-manifest frontmatter and narrative sections required of the FTS ingestion skill.
- AC3: Source drift and rate-limit issues surface as explicit failure codes rather than as silent partial success.

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
- Relation resolution / notice merge heuristics (Layer 3)
- Worker-validator system (Layer 3/4)
- Validation dry-run using existing validation fields (Layer 3)
- Sync engine (Layer 4)
- Remote-owned shared operator decisions via CAS flush (Layer 4)
- Sync-specific health dimension beyond local daemon/storage state (Layer 4)
- Web viewing portal (Layer 4)
- Multi-source marketplace search (Layer 5)
- Decision-grade landed cost (Layer 5)
- Bid workflow automation (Layer 6)
- Multi-user / RBAC (not in MVP)

This list is the operator-facing shorthand for the stub table in §1.14. If the two diverge, §1.14 must be updated and this summary must be brought back into sync in the same edit.

### 1.16 Accessibility

- Do NOT `aria-live` every transcript line — announce only state transitions ("session stalled", "new opportunities available")
- Do NOT use color as the only signal for margin/risk or sync health — provide text labels and icons alongside
- Provide plain-text transcript mode for screen readers alongside ANSI terminal view
- Keyset pagination is accessible by default (stable focus targets, no shifting content)
- Deep links resolve through `process_aliases` so bookmarked IDs survive later merges

**L2-FR-A11Y.1 — Session Accessibility Prioritizes State Changes Over Raw Stream Noise**
- AC1: Transcript views do not `aria-live` every incoming line.
- AC2: Screen-reader users have access to a plain-text transcript path alongside the ANSI terminal presentation.
- AC3: Session-state transitions that matter operationally are surfaced through concise accessible announcements.

**L2-FR-A11Y.2 — Operator Signals Are Never Color-Only**
- AC1: Margin, risk, coverage, and health indicators present text and/or icon semantics alongside color.
- AC2: Keyset pagination keeps focus targets stable while content updates.
- AC3: Deep-link identities remain resolvable through canonical process ids and later alias support rather than brittle ephemeral row positions.

---

## 2. Shared UI Components & Manifest Rendering

### 2.1 `packages/ui` — Shared Presentational Components

**6 files (5 components + 1 barrel export).** These components are shared between the desktop control panel (Layer 2) and the web viewing portal (Layer 4). They are **pure presentational** — they receive typed props and render UI. They must NOT import data-fetching hooks, Supabase clients, Tauri invoke wrappers, daemon bridge code, or mutation orchestration.

**Boundary rule:** Share down to view-models, presentational components, and formatting helpers. Stop sharing at repositories, hooks, realtime subscriptions, and mutation flows. Data-fetching is surface-specific: desktop uses QueryService (SQLite), web uses Supabase client.

#### `index.ts` — Barrel Export

Re-exports all shared components. Consumers import from `@project/ui`.

Requirement mapping: `index.ts` participates in `L2-FR-UI.1` by exposing only the presentational surface of the shared package.

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
    pipelineState: 'pending' | 'qualifying' | 'enriching' | 'qualified' | 'review' | 'rejected' | 'error'
    pricedCoverageBps: number | null
    netQualifiedCoverageBps: number | null
    netQualifiedValueCoverageBps: number | null
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
- Auto-qualification affordances use `netQualifiedValueCoverageBps` for gating; lot-count `netQualifiedCoverageBps` is informative but not authoritative for the gate
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

**L2-FR-UI.1 — `packages/ui` Remains Purely Presentational**
- AC1: Shared UI components accept typed props and render UI without importing QueryService, Supabase clients, Tauri bridge code, or mutation orchestration.
- AC2: Surface-specific data loading remains outside `packages/ui`.
- AC3: Shared components stay reusable across desktop and web without embedding backend-specific assumptions.

**L2-FR-UI.2 — `ContractCardFrame` Renders the Stable Summary Card Contract**
- AC1: The component accepts the summary, metrics, decision, `rightRail`, and `actions` props defined in this section.
- AC2: Monetary and percentage formatting follow the display rules defined here.
- AC3: Stale decisions and keyset-stable ordering remain visible without relying on boolean-prop variants.

**L2-FR-UI.3 — `ManifestViewer` Renders Sanitized, Navigable Opportunity Manifests**
- AC1: The component renders manifest Markdown with section navigation, blocker highlighting, and print-friendly layout.
- AC2: Raw HTML passthrough is disabled and dangerous link schemes are stripped per the security rules in `L2-FR-SEC.1`.
- AC3: Section copy and navigation affordances operate on the parsed section structure rather than on brittle raw DOM heuristics.

**L2-FR-UI.4 — `PipelineBadge` Presents Pipeline State Accessibly**
- AC1: The component renders pipeline state with color, text, and icon semantics together.
- AC2: `validationState` appears only when not `unverified`.
- AC3: Tooltip output reflects the last state change rather than requiring the operator to infer freshness from color alone.

**L2-FR-UI.5 — `FreshnessBadge` Consumes a Surface-Neutral View Model**
- AC1: The component accepts only `FreshnessBadgeVM` (`label`, `tone`, `tooltip`) and does not compute freshness from raw backend state itself.
- AC2: Desktop and web compute the view model independently from their own freshness sources.
- AC3: Layer 2 desktop behavior reflects daemon/local freshness only; sync-specific freshness remains a later-layer concern.

**L2-FR-UI.6 — `ErrorSummaryList` Surfaces Sanitized, Typed Error Summaries**
- AC1: The component accepts typed `ErrorSummaryItem` rows and resolve/click callbacks as defined in this section.
- AC2: Severity derivation follows the rule set in this document rather than arbitrary caller-provided prose.
- AC3: Error messages and context are sanitized before rendering to the operator.

**L2-FR-UI.7 — Shared Component Prop Unions Match Surface Usage**
- AC1: `ContractCardFrameProps.metrics.pipelineState` covers every pipeline-state badge the list/detail surfaces may render, including `error`.
- AC2: Shared prop unions do not force page components to coerce or drop valid desktop states before rendering.
- AC3: Any later-added surface state requires a same-edit update to the relevant shared prop union and traceability row.

**L2-FR-UI.8 — `index.ts` Re-Exports Only the Shared Presentational Surface**
- AC1: The barrel export exposes the shared components and view-model types intended for cross-surface use.
- AC2: The barrel export does not pull in desktop-only transport, hooks, or mutation helpers as side effects.
- AC3: Consumers can import the shared surface from one stable package entry point.

**L2-FR-UI.9 — Severity Derivation Is Stable Across Surfaces**
- AC1: `ErrorSummaryList` severity derivation follows the exact rule set declared in this section.
- AC2: Desktop and web pass the same derived severity categories to the shared component.
- AC3: Severity labels do not vary by surface-specific wording or ad hoc local logic.

**L2-FR-UI.10 — Freshness View Models Preserve Surface-Neutral Semantics**
- AC1: `FreshnessBadgeVM.label`, `tone`, and `tooltip` are sufficient for rendering without the component inspecting raw timestamps.
- AC2: Desktop and web may compute different freshness thresholds, but they emit the same view-model shape.
- AC3: The component does not special-case sync-vs-local freshness internally.

**L2-FR-UI.11 — Read-Only Surfaces Omit Mutating Error Actions**
- AC1: `onResolveClick` is optional and may be `null` for read-only consumers such as the future web portal.
- AC2: The shared error component renders correctly whether or not resolve actions are available.
- AC3: Read-only consumers do not need to fork the component simply to remove mutation affordances.

**L2-FR-UI.12 — Shared Component Tests Are Fixture-Based**
- AC1: Shared component tests operate on fixture view models in `packages/ui/fixtures/*`.
- AC2: Shared component tests do not depend on SQLite, Supabase, or daemon transport.
- AC3: Surface-specific behavior remains tested in the app packages rather than inside `packages/ui`.

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

**L2-FR-SEC.1 — Agent-Authored Content Is Sanitized Before Rendering**
- AC1: `ManifestViewer`, `ErrorSummaryList`, and `SessionViewer` treat agent-authored text as untrusted input.
- AC2: Markdown renderers disable raw HTML and strip dangerous link schemes such as `javascript:` and unsafe `file://`.
- AC3: Terminal viewers sanitize dangerous OSC sequences, including hyperlink and clipboard-related escapes, before rendering.
- AC4: Error-manifest drill-down uses the same sanitized-markdown path as opportunity manifests, while JSON/error-context payloads render only as escaped text.
- AC5: List/detail/session surfaces render agent-authored strings as text content rather than as raw HTML fragments.

**L2-FR-SEC.2 — Desktop Capability Exposure Is Least-Privilege**
- AC1: Desktop capability configuration admits only the shell/sidecar access required by the L2 desktop surface.
- AC2: Dev-only transport exposure remains loopback-only and excluded from production builds.
- AC3: Shared packages do not acquire elevated access indirectly by importing desktop-only bridge or daemon modules.

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
| `pricedValueCoverageBps` | `INTEGER` (0-10000) | Fraction of qualifying value with >=1 priced candidate | Total qualifying lot value |
| `landedCostValueCoverageBps` | `INTEGER` (0-10000) | Fraction of qualifying value with landed-cost estimates | Total qualifying lot value |
| `netQualifiedValueCoverageBps` | `INTEGER` (0-10000) | Fraction of qualifying value passing the net margin gate | Total qualifying lot value |
| `unresolvedMandatoryLots` | `INTEGER` | Count of mandatory lots without resolution | N/A (absolute count) |
| `decisionBasisHash` | `TEXT` | SHA-256 of the pipeline data the operator would review | N/A (staleness key) |

Display: `bps / 100` for percentage (e.g., 7500 bps = "75.0%").

**Color thresholds for `netQualifiedCoverageBps`:**

| Range | Color | Label |
|-------|-------|-------|
| >= 7000 bps (70%+) | Green | Strong coverage |
| 3000-6999 bps (30-69%) | Amber | Partial coverage -- review recommended |
| < 3000 bps (< 30%) | Red | Low coverage -- manual review required |

Always show both color AND text label (accessibility). **Auto-qualification guard uses value-weighted coverage, not lot-count coverage:** `netQualifiedValueCoverageBps >= 5000` (50% of qualifying value) is required. Desktop surfaces a "Coverage below threshold" indicator whenever the value-weighted gate fails, even if lot-count coverage appears strong.

**L2-FR-COV.1 — Coverage Metrics Are Presented as an Operator Decision Aid**
- AC1: The desktop displays the lot-count coverage metrics (`pricedCoverageBps`, `landedCostCoverageBps`, `netQualifiedCoverageBps`), the value-weighted gate metrics (`pricedValueCoverageBps`, `landedCostValueCoverageBps`, `netQualifiedValueCoverageBps`) where the operator needs gate context, and `unresolvedMandatoryLots` using the interface summarized in this section.
- AC2: `netQualifiedCoverageBps` uses the color-and-text threshold bands defined here.
- AC3: Auto-qualification guidance reflects the 5000 bps **value-weighted** guard and explicitly calls out when coverage remains below threshold.
- AC4: Processes with no qualifying lots or no priced candidates degrade to explicit zero/empty coverage states rather than undefined percentages.

### 3.2 Decision Capture — `pending_operator_intents`

Local single-operator overlay. NOT the authoritative shared decision table (that is `operator_decision_heads` in Layer 4 via CAS).

**`decisionBasisHash` is daemon-authored:** The hash is computed by the daemon from a canonical serialization of operator-relevant pipeline fields. It is NOT computed by the manifest renderer, by a skill, or by the desktop. The desktop captures the daemon-provided hash at decision time. This prevents hash drift across Rust/TS/agent implementations.

**Local dispositions:** `approve` | `reject` | `watch`

**Remote canonical states (Layer 4):** `approved` | `rejected` | `watching`. L2 persists local intent verbs; the L4 CAS flush normalizes them into the remote-owned `operator_decision_heads` state model.

**Decision metadata captured on every intent:**

| Field | Purpose |
|-------|---------|
| `process_id` | Target process |
| `desired_disposition` | approve / reject / watch |
| `reviewed_basis_hash` | SHA-256 of pipeline data at decision time |
| `reviewed_manifest_hash` | SHA-256 of the manifest reviewed |
| `reviewed_at_stage_run_id` | Current `stage_run_id` at decision time |
| `reviewed_at` | Timestamp of the operator review being affirmed |
| `base_remote_revision` | Defaults to `0` in L2 because no remote decision head exists yet |
| `idempotency_key` | UUID for future CAS deduplication (Layer 4) |
| `status` | `queued` (always in L2 -- no remote flush yet) |
| `superseded_by_intent_id` | Links older intents to the newest intent for the same process |
| `queued_at` | Local enqueue timestamp |

**Latest-per-process coalescing:** Only the latest intent per process matters. If operator changes `approve` to `reject`, the `reject` supersedes. Desktop immediately reflects the queued intent as the current decision. In L2, the local intent IS the decision. In L4, it becomes a queue entry flushed to Supabase via CAS.

**Normative schema alignment:** Layer 0 §2.5.8 owns the SQLite contract for `pending_operator_intents`. The current checked-in `migrations/0001_sqlite_state_initial.sql` does **not** yet define this table, so Layer 2 implementation must add it using the Layer 0 column contract above before decision capture is considered shippable.

**L2-FR-DEC.1 — Each Local Decision Captures the Reviewed Basis**
- AC1: Every decision intent persisted in `pending_operator_intents` records the reviewed-basis payload from this section, including `process_id`, `desired_disposition`, `reviewed_basis_hash`, `reviewed_manifest_hash`, `reviewed_at_stage_run_id`, `reviewed_at`, `base_remote_revision`, `idempotency_key`, `status`, `superseded_by_intent_id`, and `queued_at`.
- AC2: `reviewedBasisHash` is copied from the daemon-authored pipeline assessment rather than recomputed in the UI.
- AC3: L2 persists local intent verbs (`approve`, `reject`, `watch`) while leaving remote canonical-state normalization to Layer 4.
- AC4: Decision capture failures do not leave the UI claiming a new current decision without a committed `MutationAck`.

**L2-FR-DEC.2 — Latest-Per-Process Coalescing Is Explicit**
- AC1: Only the latest queued intent for a process is treated as the current local decision.
- AC2: A newer decision supersedes the older one via `superseded_by_intent_id` without losing the audit trail of prior intents.
- AC3: Reaffirming or changing a decision produces a new intent with a new reviewed basis instead of mutating historical intent rows in place.

**L2-FR-DEC.4 — `pending_operator_intents` Reuses the Layer 0 DDL Exactly**
- AC1: Layer 2 does not invent a parallel decision-intent table shape; it reuses the Layer 0 column contract and naming (`desired_disposition`, `reviewed_basis_hash`, `reviewed_manifest_hash`, `reviewed_at_stage_run_id`, `reviewed_at`, `base_remote_revision`, `idempotency_key`, `status`, `superseded_by_intent_id`, `queued_at`, `last_attempt_at`, `last_error`).
- AC2: `base_remote_revision` defaults to `0` for L2 local-only operation.
- AC3: The implementation is not considered done until the checked-in SQLite migration defines this table.

### 3.3 Decision Staleness Detection

1. Operator decides -> `reviewedBasisHash` captured from `pipeline_assessments.decisionBasisHash`
2. Pipeline re-runs -> `decisionBasisHash` changes
3. Desktop compares: mismatch = **stale**

**Stale decision UX:**
- Warning indicator: "Decision may be outdated -- pipeline data changed since review"
- Operator can re-affirm (new intent with updated hash) or change disposition
- **Stale decisions block bid workflow** (Layer 6) -- must re-affirm first

**Staleness triggers:** The authoritative trigger is **any** change to `pipeline_assessments.decisionBasisHash`. Typical causes include new/updated quotes, landed-cost recalculation, document-set changes, lot extraction changes, product-candidate changes, relation-resolution changes, manifest regeneration inputs, or any upstream rerun that changes the reviewed basis.

**L2-FR-DEC.3 — Decision Staleness Is Hash-Based and Operator-Visible**
- AC1: The desktop compares `reviewedBasisHash` against the current `pipeline_assessments.decisionBasisHash`; mismatch means stale.
- AC2: Staleness warning UI appears whenever `decisionBasisHash` changes, regardless of which upstream stage caused it.
- AC3: Operators can re-affirm or replace a stale decision, creating a new local intent with the updated basis hash.
- AC4: Stale decisions are treated as blockers for later bid workflow layers rather than as silently still-current approvals.

### 3.4 Coverage Rollup Module

`packages/intelligence/src/margins/coverage-rollup.ts` -- runtime implementation of L0 §3.14 formulas.

```typescript
interface CoverageInput {
  lots: Array<{
    lotId: string
    qualificationResult: 'pass' | 'review' | 'reject'
    isMandatory: boolean
    lotAddressableValuePence: number | null
    candidates: Array<{
      candidateId: string
      hasPricing: boolean
      hasLandedCost: boolean
      passesNetGate: boolean
      candidateValueShareBps: number | null
    }>
  }>
}

interface CoverageOutput {
  pricedCoverageBps: number
  landedCostCoverageBps: number
  netQualifiedCoverageBps: number
  pricedValueCoverageBps: number
  landedCostValueCoverageBps: number
  netQualifiedValueCoverageBps: number
  unresolvedMandatoryLots: number
  decisionBasisHash: string
}

function computeCoverageRollup(input: CoverageInput): CoverageOutput
```

**L2-FR-COV.2 — Coverage Rollup Implements the L0 Formula Contract**
- AC1: `computeCoverageRollup()` returns both lot-count and value-weighted coverage outputs: `pricedCoverageBps`, `landedCostCoverageBps`, `netQualifiedCoverageBps`, `pricedValueCoverageBps`, `landedCostValueCoverageBps`, `netQualifiedValueCoverageBps`, `unresolvedMandatoryLots`, and `decisionBasisHash`.
- AC2: Coverage output is derived from qualifying-lot, lot-value, and candidate-level inputs, not from arbitrary UI-side heuristics.
- AC3: The module remains the runtime implementation of the L0 formulas while Layer 2 owns only the presentation behavior in `L2-FR-COV.1`.

**L2-FR-COV.5 — Auto-Qualification Uses Value-Weighted Coverage**
- AC1: The auto-qualification gate uses `netQualifiedValueCoverageBps >= 5000`, not `netQualifiedCoverageBps`.
- AC2: Lot-count coverage badges do not override a failed value-weighted gate.
- AC3: Missing lot values or candidate value shares keep the gate conservative rather than treating unresolved value as implicitly covered.

**L2-FR-COV.3 — Coverage Threshold Labels Are Stable UI Contract**
- AC1: The green/amber/red threshold bands in this section are the only Layer 2 labels for `netQualifiedCoverageBps`.
- AC2: The UI shows both the numeric percentage and the textual label for the current band.
- AC3: Threshold-band wording is stable enough for test assertions.

**L2-FR-COV.4 — Unresolved Mandatory Lots Are Presented as an Absolute Risk Count**
- AC1: `unresolvedMandatoryLots` is displayed as a count, not coerced into a percentage.
- AC2: Non-zero unresolved mandatory lots are surfaced as a risk or blocker signal in detail and manifest surfaces.
- AC3: Zero unresolved mandatory lots remain renderable as an explicit non-blocking state.

### 3.5 Keyset Pagination

All list views use **keyset/cursor pagination** (not offset). Offset causes row jumping on live updates.

- Client sends: `{ after: lastSeenCursor, limit: 25, filters: {...} }`
- Server returns: `{ items: [...], nextCursor: string | null, hasMore: boolean }`
- Cursor encodes sort position (e.g., `created_at + id`)
- Stable focus targets -- items don't shift while user reads

**L2-FR-PAG.1 — All Desktop Lists Use Cursor Pagination**
- AC1: List surfaces send `after`, `limit`, and filters and receive `items`, `nextCursor`, and `hasMore`.
- AC2: Cursors encode stable sort position rather than offset counts.
- AC3: Live writes do not reorder already-seen rows within the operator's current page window.

**L2-FR-PAG.2 — Empty and Terminal Pages Are Explicit States**
- AC1: The first empty page returns `items: []`, `nextCursor: null`, and `hasMore: false`.
- AC2: The last non-empty page returns `nextCursor: null` and `hasMore: false` without requiring the UI to probe an extra page to discover the end.
- AC3: Pagination state remains mechanically testable for process, session, and error lists.

---

## 4. Testing & Acceptance

### 4.1 Testing Tiers

**Browser-on-localhost via Playwright is the primary UI testing surface.** Testing desktop apps natively requires OS-level interaction (screen capture, mouse simulation — "computer use"), which is slow and fragile. Since Tauri's frontend is a standard React SPA served by Vite on localhost, and the dev-mode transport bridge (§1.8) provides full daemon access in a plain browser, we test UI functionality in a real browser using Playwright. This enables use of Playwright MCP and Vercel agent browser skills for interactive verification that simulates real human usage.

| Tier | Surface | What It Covers | When It Runs |
|------|---------|---------------|--------------|
| **1. Browser E2E (primary)** | Playwright against `localhost:5173` via dev bridge | All UI rendering, navigation, pagination, data display, decision capture, session monitoring, coverage metrics, error views, staleness indicators, commit-fence consistency, live streaming via WebSocket, stop/input/retry controls | Every iteration — fast feedback loop, no Tauri build required |
| **2. Tauri verification** | Full Tauri build, manual + scripted | Desktop-only behaviors: daemon auto-launch on app open, window lifecycle (close UI ≠ kill daemon), browse-only fallback on daemon unavailability, `instanceId` mismatch detection after daemon crash | Before milestone gate — verifies Tauri-specific integration |
| **3. Backend verification** | Rust + TypeScript backend tests plus real daemon/skill E2E where needed | Skill runtime, normalization/classification contracts, QueryService/bridge invariants, cleanup verdicts, error manifests | CI gate — validates backend and contract behavior independently of UI |

**Why browser-first:** Criteria 6-11, 14-15, 17, 19, and 20 in the Definition of Done (§4.2) test UI behavior that has no Tauri dependency — list pagination, detail views, decision capture, coverage badges, session controls, error surfaces, sanitization, reconnect/gap handling, and commit-fence consistency. These run faster, more reliably, and with better tooling (Playwright selectors, assertions, screenshots, trace viewer) in a browser than through Tauri's webview. Only criteria 5, 12, 13, 16, and the desktop-shell-specific parts of 18 genuinely require the Tauri desktop context.

**Browser E2E workflow:**

```text
1. Start daemon (background)
2. bun run dev                   → Vite + dev bridge plugin on localhost:5173
3. bunx playwright test          → Chromium navigates to localhost:5173
                                   Full daemon access via HTTP/WebSocket bridge
                                   Real data, real mutations, real streaming
```

Playwright tests interact with the app exactly as a human operator would — navigating pages, clicking buttons, filling forms, observing live updates. The dev bridge (§1.8) makes this indistinguishable from the Tauri webview experience at the application level.

### 4.2 Definition of Done

| # | Criterion | Verification | Tier | Requirement IDs |
|---|----------|-------------|------|-----------------|
| 1 | **FTS ingestion skill** scrapes live notices from Find a Tender | Automated: agent E2E test produces notices | 3 | `L1-FR-SKILLS.2`, `L2-FR-DB.4` |
| 2 | **Contracts Finder ingestion skill** scrapes live notices | Automated: agent E2E test produces notices and a compliant run manifest | 3 | `L2-FR-CF.1`-`L2-FR-CF.3`, `L2-FR-DB.4` |
| 3 | **Notice normalization** produces canonical internal records | Automated: normalized records match expected schema | 3 | `L1-FR-PIPELINE.12`-`L1-FR-PIPELINE.22`, `L2-FR-QS.5` |
| 4 | **Goods/services classifier** correctly categorizes notices | Automated: unit tests for CPV-based classification | 3 | `L2-FR-PAGE.1`, `L2-FR-PAGE.2`, `L2-FR-MAN.3` |
| 5 | **Desktop control panel** launches with QueryService + DaemonBridge + bootstrap state machine | Tauri verification: app launches, state machine detects daemon, and runtime mode is derived deterministically | 2 | `L2-FR-BOOT.1`-`L2-FR-BOOT.11`, `L2-FR-QS.1`, `L2-FR-DB.1` |
| 6 | **Notice list view** with keyset pagination displays live pipeline output | Playwright: navigate list, verify pagination, verify data from real daemon | 1 | `L2-FR-PAGE.1`, `L2-FR-QS.4`, `L2-FR-PAG.1`, `L2-FR-COV.1` |
| 7 | **Opportunity detail view** shows margin bands, product candidates, supplier quotes | Playwright: open detail, verify all data sections render with real pipeline data | 1 | `L2-FR-PAGE.2`, `L2-FR-QS.5`, `L2-FR-MAN.1`-`L2-FR-MAN.3`, `L2-FR-COV.1` |
| 8 | **Decision capture** (reject/watch/approve) persists locally in `pending_operator_intents` with reviewed-basis metadata | Playwright: capture decision, verify DB state via daemon query and the Layer 0 DDL column contract | 1 | `L2-FR-DB.6`, `L2-FR-DEC.1`, `L2-FR-DEC.2`, `L2-FR-DEC.4` |
| 9 | **Agent session monitoring** page shows live terminal output via streaming | Playwright: trigger ingestion, navigate to session viewer, verify live output appears via WebSocket stream | 1 | `L2-FR-PAGE.3`, `L2-FR-PAGE.4`, `L2-FR-CH.1`-`L2-FR-CH.7` |
| 10 | **Coverage metrics** displayed: lot-count coverage badges plus value-weighted auto-qualification gate indicator | Playwright: verify badges render with correct colors/text and that the value-weighted gate is surfaced distinctly | 1 | `L2-FR-COV.1`-`L2-FR-COV.5`, `L2-FR-UI.4`, `L2-FR-A11Y.2` |
| 11 | **Decision-basis staleness** indicators flag stale decisions | Playwright: capture decision, trigger re-run, verify staleness indicator appears | 1 | `L2-FR-DEC.1`-`L2-FR-DEC.3`, `L2-FR-PAGE.2` |
| 12 | **Single operator** runs daily for 1 week and reports genuine opportunities surfaced | Manual: operator validation over 5-7 working days (Tauri desktop) | 2 | `L2-FR-PAGE.1`-`L2-FR-PAGE.6`, `L2-FR-DEC.1`-`L2-FR-DEC.3`, `L2-FR-COV.1` |
| 13 | **Desktop auto-launches daemon** if not running; operator never needs manual daemon startup | Tauri verification: close daemon, open desktop, verify daemon starts automatically | 2 | `L2-FR-BOOT.6` |
| 14 | **Stop/input/retry controls** work: stop active session, send PTY input for auth challenges, retry failed jobs | Playwright: exercise each control during live session, verify daemon responds | 1 | `L2-FR-DB.8`-`L2-FR-DB.11`, `L2-FR-PAGE.4`, `L2-FR-CH.7`, `L2-FR-CH.8` |
| 15 | **Commit fence consistency:** mutation → `visibleAfterSeq` → subsequent query returns updated data | Playwright: trigger mutation, poll until `snapshotSeq >= visibleAfterSeq`, verify data | 1 | `L2-FR-QS.2`, `L2-FR-QS.3`, `L2-FR-DB.3` |
| 16 | **Daemon restart detection:** desktop detects daemon restart via `instanceId` mismatch, invalidates state, does not attempt PTY stream resume | Tauri verification: kill daemon during session, verify desktop recovery | 2 | `L2-FR-DB.2`, `L2-FR-BOOT.4`, `L2-FR-CH.6` |
| 17 | **Errors page** surfaces unresolved manifests, sanitized drill-down, and resolve flow | Playwright: inspect error queue, open drill-down, resolve one error, and verify commit-fence removal | 1 | `L2-FR-PAGE.5`, `L2-FR-QS.7`, `L2-FR-DB.7`, `L2-FR-UI.6`, `L2-FR-SEC.1` |
| 18 | **Daemon status page** surfaces runtime diagnostics and restart control | Browser/Tauri verification: inspect diagnostics, verify descriptor-backed fallback, and exercise restart control | 2 | `L2-FR-PAGE.6`, `L2-FR-PAGE.13`, `L2-FR-DB.12`, `L2-FR-DB.21` |
| 19 | **Security boundary** sanitizes manifest Markdown, error drill-down/context, and ANSI terminal output | Playwright: inject hostile content and verify sanitization/escaping on every affected surface | 1 | `L2-FR-SEC.1`, `L2-FR-UI.3`, `L2-FR-UI.6`, `L2-FR-PAGE.4` |
| 20 | **Stream reconnect and gap recovery** preserve order and expose unresolved loss explicitly | Playwright: drop subscription, induce a gap, replay from `lastSeq`, and verify no silent duplication or silent loss | 1 | `L2-FR-CH.3`, `L2-FR-CH.5`, `L2-FR-CH.9`-`L2-FR-CH.15`, `L2-FR-PAGE.4` |
| 21 | **Query and mutation contracts** remain explicit under load and retry: short-lived reads, empty states, transport failures, idempotent lost-ack retry | Rust integration + Playwright: exercise read limits, invalid filters/cursors, explicit bridge failures, and retry-after-commit behavior | 3 | `L2-FR-QS.1`, `L2-FR-QS.8`, `L2-FR-PAG.2`, `L2-FR-DB.3`, `L2-FR-DB.14` |

### 4.3 Verification Assets

| File | Type | Tier | What It Tests | Requirement IDs | DoD # |
|------|------|------|--------------|-----------------|-------|
| `apps/desktop/e2e/contracts.spec.ts` | Playwright (browser) | 1 | List browsing with keyset pagination, detail view with all data sections, coverage metrics display, pipeline badges, freshness indicators | `L2-FR-PAGE.1`, `L2-FR-PAGE.2`, `L2-FR-QS.4`, `L2-FR-QS.5`, `L2-FR-COV.1`-`L2-FR-COV.5`, `L2-FR-PAG.1`, `L2-FR-UI.2`, `L2-FR-UI.4`, `L2-FR-UI.5` | 6, 7, 10 |
| `apps/desktop/e2e/decisions.spec.ts` | Playwright (browser) | 1 | Decision capture (approve/reject/watch), staleness detection after re-run, reviewed-basis metadata verification | `L2-FR-DB.6`, `L2-FR-DEC.1`-`L2-FR-DEC.4` | 8, 11 |
| `apps/desktop/e2e/sessions.spec.ts` | Playwright (browser) | 1 | Session list, live terminal output via WebSocket, session stop, PTY input for auth challenges, retry failed jobs, gap recovery, reconnect behavior | `L2-FR-PAGE.3`, `L2-FR-PAGE.4`, `L2-FR-DB.8`-`L2-FR-DB.11`, `L2-FR-CH.1`-`L2-FR-CH.15` | 9, 14, 20 |
| `apps/desktop/e2e/commit-fence.spec.ts` | Playwright (browser) | 1 | Mutation → `visibleAfterSeq` → query reflects change; verify no ghost states | `L2-FR-QS.2`, `L2-FR-QS.3`, `L2-FR-DB.3` | 15 |
| `apps/desktop/e2e/bridge-failures.spec.ts` | Playwright (browser) | 1 | Mutation transport failure, explicit refusal paths, lost-ack idempotent retry, conflict surfacing | `L2-FR-DB.3`, `L2-FR-DB.14`, `L2-FR-PAG.2` | 21 |
| `apps/desktop/e2e/errors.spec.ts` | Playwright (browser) | 1 | Error manifest list, sanitized drill-down, resolve action, severity indicators | `L2-FR-PAGE.5`, `L2-FR-QS.7`, `L2-FR-DB.7`, `L2-FR-UI.6`, `L2-FR-SEC.1` | 17 |
| `apps/desktop/e2e/security.spec.ts` | Playwright (browser) | 1 | Agent-authored Markdown sanitization (ManifestViewer + error drill-down), dangerous link stripping, ANSI escape sanitization (SessionViewer) | `L2-FR-UI.3`, `L2-FR-UI.6`, `L2-FR-SEC.1` | 19 |
| `apps/desktop/e2e/daemon-status.spec.ts` | Playwright (browser) | 1 | Daemon diagnostics rendering via `get_daemon_info()`, fallback state markers, restart CTA gating | `L2-FR-PAGE.6`, `L2-FR-PAGE.13`, `L2-FR-DB.21` | 18 |
| `apps/desktop/e2e/tauri-integration.spec.ts` | Tauri-specific | 2 | Daemon auto-launch, window lifecycle (close ≠ kill daemon), browse-only fallback, daemon restart detection via `instanceId` mismatch, desktop-shell restart/open-path affordances | `L2-FR-BOOT.3`-`L2-FR-BOOT.11`, `L2-FR-DB.2`, `L2-FR-DB.12`, `L2-FR-DEV.4` | 5, 13, 16, 18 |
| `apps/desktop/src-tauri/tests/query_service.rs` | Rust integration | 3 | QueryService short-lived reads, invalid cursor/filter handling, artifact-unavailable history reads, empty-page contract | `L2-FR-QS.1`, `L2-FR-QS.8`, `L2-FR-QS.11`-`L2-FR-QS.13`, `L2-FR-PAG.2` | 21 |
| `agent_procurement_ingestion.rs` | Agent E2E (Rust) | 3 | Real FTS skill session: PTY spawn, notice extraction, normalization, result contract validation, cleanup verdict | `L1-FR-SKILLS.2`, `L1-FR-PIPELINE.12`-`L1-FR-PIPELINE.22` | 1, 3 |
| `agent_contracts_finder_ingestion.rs` | Agent E2E (Rust) | 3 | Real Contracts Finder skill session: retrieval, checkpointing, continuation, run-manifest compliance | `L2-FR-CF.1`-`L2-FR-CF.3`, `L2-FR-DB.4` | 2 |
| `packages/intelligence/src/qualification/goods-qualification.test.ts` | TypeScript unit | 3 | CPV-based goods/services/works/mixed classification, DPS expiry exception, access-route rules | `L2-FR-PAGE.1`, `L2-FR-PAGE.2`, `L2-FR-MAN.3` | 4 |
| `agent_supplier_enrichment.rs` | Agent E2E (Rust) | 3 | Real eBay enrichment session: search execution, quote extraction, dual confidence scoring, coverage-rollup inputs | `L1-FR-PIPELINE.12`-`L1-FR-PIPELINE.22`, `L2-FR-COV.2` | 7, 10 |
| `agent_health_detection.rs` | Agent E2E (Rust) | 3 | Stall detection (no output + no progress), auth challenge detection, cleanup escalation | `L1-FR-ERRORS.21`, `L1-FR-ERRORS.22`, `L2-FR-CH.8` | 9, 14, 17 |
| `agent_error_manifest.rs` | Agent E2E (Rust) | 3 | Error manifest creation on various failure types (skill failed, contract invalid, suspect output) | `L1-FR-ERRORS.23`-`L1-FR-ERRORS.39`, `L2-FR-PAGE.5` | 17 |
| `docs/testing/l2-operator-weekly-validation.md` | Manual checklist | 2 | Five-to-seven-day operator diary: daily triggers, surfaced opportunities, accept/reject/watch actions, and observed value | `L2-FR-PAGE.1`-`L2-FR-PAGE.6`, `L2-FR-DEC.1`-`L2-FR-DEC.3`, `L2-FR-COV.1` | 12 |

### 4.4 Test Scenarios

| Scenario | Expected Behavior | Tier | Requirement IDs | DoD # |
|----------|------------------|------|-----------------|-------|
| Notice with 12 lots | System processes each lot independently; detail view shows per-lot results | 1 | `L2-FR-PAGE.2`, `L2-FR-QS.5`, `L2-FR-MAN.3` | 7 |
| Mixed contract with separable goods | Goods-addressable value used for margin computation, not whole contract value | 1 | `L2-FR-COV.1`, `L2-FR-COV.2`, `L2-FR-PAGE.2` | 4, 10 |
| Expired notice | Correctly filtered; not displayed as active opportunity | 1 | `L2-FR-QS.4`, `L2-FR-PAGE.1`, `L2-FR-PAG.1` | 6 |
| Agent session crashes mid-run | Error manifest created; operator sees failure context in error view; cleanup verdict is `forced` | 1 | `L2-FR-PAGE.5`, `L2-FR-QS.7`, `L2-FR-PAGE.3` | 17 |
| Decision on process, then pipeline re-runs | Decision marked stale; warning indicator shown; bid workflow blocked | 1 | `L2-FR-DEC.1`-`L2-FR-DEC.3` | 11 |
| DPS/Dynamic Market notice | 30-day expiry filter NOT applied; notice remains active | 1 | `L2-FR-QS.4`, `L2-FR-PAGE.1` | 4, 6 |
| Agent auth challenge during ingestion | `AGENT_AUTH_CHALLENGE` classified; operator alerted via session view | 1 | `L2-FR-PAGE.4`, `L2-FR-CH.8` | 9, 14, 17 |
| Browser-only auth challenge with no PTY input path | SessionViewer exposes reveal/reattach intervention instructions instead of a dead-end prompt | 1 | `L2-FR-PAGE.4`, `L2-FR-CH.8` | 14 |
| Stop active session from UI | Session stop command reaches daemon, session enters terminating state, cleanup verdict rendered | 1 | `L2-FR-DB.8`, `L2-FR-PAGE.4` | 14 |
| PTY input for auth challenge | Operator types into SessionViewer, input sent to PTY via `send_session_input`, agent proceeds | 1 | `L2-FR-DB.9`, `L2-FR-CH.7` | 14 |
| Retry failed job from UI | Retry command creates new job, original job marked, new session appears in session list | 1 | `L2-FR-DB.11`, `L2-FR-PAGE.3` | 14 |
| Commit fence: mutation then immediate query | Mutation returns `visibleAfterSeq`; query polls until `snapshotSeq >= visibleAfterSeq`; no ghost state | 1 | `L2-FR-QS.2`, `L2-FR-QS.3`, `L2-FR-DB.3` | 15 |
| Keyset pagination under live updates | Immutable sort key (created_at + id) ensures stable pagination; no row jumping | 1 | `L2-FR-PAG.1`, `L2-FR-PAGE.1`, `L2-FR-A11Y.2` | 6 |
| QueryService returns no matching processes | List page renders a stable empty state with `hasMore: false` and no synthetic error | 1 | `L2-FR-QS.4`, `L2-FR-PAG.2`, `L2-FR-PAGE.1` | 6, 21 |
| Process detail has zero lots | Detail page renders explicit zero-lot messaging and still shows manifest/decision surfaces | 1 | `L2-FR-QS.5`, `L2-FR-PAGE.2` | 7 |
| Process detail has no product candidates | Detail page renders empty candidate/quote states and coverage remains explicit rather than blank | 1 | `L2-FR-QS.5`, `L2-FR-PAGE.2`, `L2-FR-COV.1` | 7, 10 |
| Session fails before reaching `active` | Sessions list still shows the session with canonical phase, failure metadata, and drill-down | 1 | `L2-FR-QS.6`, `L2-FR-PAGE.3` | 9, 17 |
| Agent-authored Markdown containing raw HTML / link abuse | ManifestViewer sanitizes HTML, strips dangerous links | 1 | `L2-FR-UI.3`, `L2-FR-SEC.1` | 19 |
| ANSI output containing hyperlink/clipboard escape sequences | SessionViewer sanitizes dangerous OSC sequences | 1 | `L2-FR-SEC.1`, `L2-FR-PAGE.4` | 19 |
| Slow SessionViewer consumer causing gap | `gap` frame emitted, viewer issues replay for the missing range, and recovery is explicit if chunks are unavailable | 1 | `L2-FR-CH.3`, `L2-FR-CH.9`, `L2-FR-CH.10`, `L2-FR-CH.11` | 20 |
| Channel/WebSocket subscription drops during a live session | Viewer reconnects, replays from `lastSeq`, and either recovers or shows an explicit unresolved-gap state | 1 | `L2-FR-CH.3`, `L2-FR-CH.5`, `L2-FR-CH.10`, `L2-FR-CH.11` | 20 |
| Long-running QueryService read starving WAL checkpoints | Short-lived read transactions prevent WAL growth; test verifies transaction duration limits | 1 | `L2-FR-QS.1`, `L2-FR-QS.8` | 21 |
| Daemon unavailable at desktop launch | Bootstrap state machine falls back to browse-only mode with clear banner | 2 | `L2-FR-BOOT.3`, `L2-FR-BOOT.6` | 5, 13 |
| Daemon becomes unreachable during a mutation command | Bridge surfaces transport failure explicitly and UI enters reconnect or browse-only handling without false success | 1 | `L2-FR-DB.14`, `L2-FR-BOOT.4` | 21 |
| Downstream job hits an Agent Mail `file_reservation_paths` conflict | Operator sees the conflict through the session/error surfaces rather than a silent stall | 1 | `L2-FR-DB.14`, `L2-FR-PAGE.5` | 17, 21 |
| Desktop closes and reopens while daemon stays alive and PTY session is active | Desktop reconnects, subscribes from `lastSeq`, stream resumes, no data loss | 2 | `L2-FR-CH.5`, `L2-FR-BOOT.5` | 20 |
| Daemon crashes while PTY session is active | Desktop detects `instanceId` mismatch, does NOT attempt PTY resume, shows historical artifacts only | 2 | `L2-FR-DB.2`, `L2-FR-BOOT.4`, `L2-FR-CH.6` | 16 |
| Lost `MutationAck` response after DB commit, retried with same idempotency key | Idempotent: returns same ack, no duplicate writes | 1 | `L2-FR-DB.3` | 21 |

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
