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
  -> Tauri Channel<T>        for live streams (sessions, jobs, daemon health)
  -> Desktop Rust bridge
       -> QueryService       (read-only SQLite, page-specific read models)
       -> DaemonBridge       (UDS/named-pipe RPC to daemon)
            -> Rust daemon   (writer, jobs, PTY, L1 runtime)
```

**Three communication channels:**

| Channel | Transport | Direction | Purpose |
|---------|-----------|-----------|---------|
| **Snapshot reads** | `invoke()` -> QueryService | Frontend -> SQLite (read-only) | List views, detail views, metrics — fast, no network dependency |
| **Mutations/triggers** | `invoke()` -> DaemonBridge -> UDS/pipe -> daemon | Frontend -> Daemon | Trigger ingestion, trigger enrichment, capture decisions |
| **Live streams** | Daemon -> DaemonBridge -> `Channel<T>` -> React | Daemon -> Frontend | Session terminal output, job progress, daemon health |

**Why not pure HTTP on localhost:** Port discovery/conflicts, auth/token management, extra attack surface, network semantics for a non-networked relationship.

**Why the Tauri backend is not the daemon:** The daemon is a background scheduler/executor. It must survive UI close, restart independently, and have its own lifecycle. Embedding it makes shutdown, crash recovery, and "app closed but cron still runs" (Layer 3) architecturally unsound.

### 1.4 QueryService Interface

The QueryService opens **read-only** SQLite connections and returns page-specific read models. It enforces: read-only/query-only connections, page-limited queries, no streaming cursors held open, no generic SQL from the frontend, no transcript/blob reads in hot queries.

```typescript
type QueryEnvelope<T> = {
  data: T
  snapshotSeq: number
  sourceMeta: {
    mode: 'healthy' | 'browse_only' | 'recovery_required'
    daemonConnected: boolean
    syncHealth: 'healthy' | 'degraded' | 'offline'  // always 'offline' in L2 (no sync yet)
  }
}
```

**Read model examples:**

| Query | Returns | Page |
|-------|---------|------|
| `list_processes(cursor, limit, filters)` | Keyset-paginated process list with pipeline state, coverage metrics, decision status | List view |
| `get_process_detail(process_id)` | Full process detail: notices, lots, candidates, quotes, costs, manifest, errors | Detail view |
| `list_sessions(cursor, limit, filters)` | Active and recent agent sessions with metadata | Sessions view |
| `list_errors(cursor, limit, filters)` | Unresolved error manifests with context | Errors view |

**SQLite concurrency rule:** SQLite is single-writer (daemon only) but supports many short readers. The QueryService opens read-only connections with short-lived transactions. Long-lived readers are prohibited because they prevent WAL checkpoints from draining. No transcript blobs or large artifacts in hot queries — those load on demand from the artifact store.

### 1.5 DaemonBridge Interface

The DaemonBridge is a thin RPC proxy — it does not hold state or perform writes itself. It forwards commands to the daemon over UDS (macOS/Linux) or named pipe (Windows).

```typescript
type MutationAck = {
  accepted: boolean
  idempotencyKey: string
  jobId?: string
  remoteState?: 'committed' | 'queued' | 'conflicted' | 'not_applicable'
  // L2: remoteState is always 'not_applicable' (no sync yet)
}
```

**Bridge RPC types:**

```typescript
type BridgeRequest =
  | { id: string; kind: 'command'; method: string; params: Record<string, unknown>; idempotencyKey: string }
  | { id: string; kind: 'subscribe'; topic: string; fromSeq?: number }
  | { id: string; kind: 'unsubscribe'; topic: string }

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

### 1.6 Channel\<T\> Live Streaming

Tauri's typed streaming mechanism for live session output from daemon to frontend.

**Session frame types:**

```typescript
type SessionFrame =
  | { seq: number; kind: 'lifecycle'; state: string; ts: string }
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
- User scrolls up → switch from live-tail to history mode (lazy-loaded from artifact chunks)
- UI falls behind stream → emit `gap` frame and force resync from last known artifact position

**Dual viewer pattern:**

| Viewer | Default For | Description |
|--------|------------|-------------|
| Live terminal | PTY mode sessions | ANSI-faithful, auto-scroll, last few thousand lines |
| Transcript/history | Contract mode sessions | Virtualized line list, searchable, lazy-loaded from artifact chunks |

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

| Mode | Local DB | Daemon | Remote Sync | UX |
|------|----------|--------|-------------|-----|
| **Healthy** | OK | Connected | N/A (L2) | Full desktop: list, detail, trigger, decide, monitor |
| **Browse-only** | OK | Unavailable | N/A | Read-only local browsing: list, detail, manifests, historical transcripts. No reruns, no triggers, no live sessions. Clear banner: "Daemon unavailable — read-only local snapshot." |
| **Recovery required** | Missing/mismatch | Any | N/A | Repair/update screen. DB schema_epoch mismatch or IPC protocol version mismatch. |

**Daemon crash mid-session:**
1. Stream/heartbeat drops
2. Desktop shows reconnect overlay
3. Bootstrap attempts one automatic daemon restart
4. On reconnect: invalidate all snapshot queries, reattach session streams from last `seq`, daemon reconciles orphan sessions
5. If restart fails: fall back to browse-only mode

**Shutdown behavior:** Closing the main window does not silently kill the daemon.
- **Quit UI** — daemon keeps running (important for Layer 3 cron)
- **Stop background daemon and quit** — clean daemon shutdown

### 1.8 Desktop File Listing

**`apps/desktop/` — 14 files:**

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
| `src/stores/session-registry.ts` | External store: tracks all active/recent sessions with metadata |
| `src/stores/session-buffer.ts` | External store: per-session ring buffer for live frames |
| `src/hooks/use-daemon.ts` | `useSessionMeta()`, `useSessionTail()`, `useSessionTimeline()` — slice subscriptions via `useSyncExternalStore` |

### 1.9 React State Management

**Pattern:** Purpose-built external stores with `useSyncExternalStore()` for slice subscriptions. **Not** TanStack Query for streaming state.

| State Class | Approach |
|-------------|----------|
| Snapshot server-state | TanStack Query for snapshot reads only — no offline persistence, no normalized cache |
| Live streaming state | Custom external stores (`SessionRegistry`, `SessionBuffer`) + `useSyncExternalStore` |
| Ephemeral UI state | Local component state / tiny persisted store |

**Rule: never put raw transcript arrays in React state.** Session output is a log system problem, not a React state problem.

**Hooks:**
- `useSessionMeta(sessionId)` — metadata slice (status, elapsed, checkpoints)
- `useSessionTail(sessionId)` — last N lines for terminal view
- `useSessionTimeline(sessionId)` — structured event timeline for contract mode

### 1.10 Shared UI Components (`packages/ui`)

**Frame + slot composition pattern** (not boolean props):

```tsx
<ContractCardFrame
  summary={vm.summary}
  metrics={vm.metrics}
  rightRail={<SyncAgeBadge meta={vm.meta} />}
  actions={desktop ? <DesktopContractActions ... /> : null}
/>
```

**5 shared components:**

| Component | Purpose | Used By |
|-----------|---------|---------|
| `ContractCardFrame` | Process summary card with slot composition | Desktop list, web list (L4) |
| `ManifestViewer` | Markdown manifest renderer with section navigation | Desktop detail, web detail (L4) |
| `PipelineBadge` | Pipeline state badge (qualified/review/rejected/pending) | Both surfaces |
| `SyncAgeBadge` | Data freshness indicator (green/amber/red) | Both surfaces (desktop shows local freshness; web shows sync freshness in L4) |
| `ErrorSummaryList` | Compact error manifest list with severity/resolution status | Both surfaces |

**Sharing boundary:** Share down to view-models, presentational components, formatting/markdown/tables/badges. Stop sharing at repositories, hooks, realtime subscriptions, mutation flows.

### 1.11 Manifest Rendering (`packages/manifests`)

**2 files:**

| File | Purpose |
|------|---------|
| `render-opportunity.ts` | Deterministic opportunity manifest renderer — takes pipeline data, produces structured Markdown |
| `opportunity.md.ts` | Markdown template for opportunity manifests |

The canonical rendering lives here, not in the `manifest-generation` skill. The skill may contribute optional narrative synthesis, but the deterministic renderer is authoritative.

### 1.12 New Skill: Contracts Finder Ingestion

| Attribute | Value |
|-----------|-------|
| **Path** | `skills/procurement-ingestion/contracts-finder/` |
| **Purpose** | Ingest live notices from Contracts Finder |
| **Interaction profile** | `pty_browser` |
| **API** | OCDS/Search API: `publishedFrom`, `publishedTo`, `stages`, `limit`, `cursor`. Rate limiting documented. |
| **Licensing** | Aligned with open contracting/publication standards |
| **Cursor strategy** | Date-based filtering + cursor; rate-limit-aware polling |

### 1.13 What's Stubbed in This Layer

| Feature | Layer 2 Stub | Full Implementation |
|---------|-------------|-------------------|
| Scheduling | Manual trigger from desktop | Layer 3: cron scheduler |
| Relation resolution | Notices processed independently | Layer 3: merge/relate heuristics |
| Landed cost | Screening-grade (flat freight, single FX) | Layer 5: decision-grade |
| Search sources | eBay only (from Layer 0) | Layer 5: Alibaba/1688, Amazon |
| Sync to Supabase | None — local-only operation | Layer 4: full sync |
| Operator decisions | Local intent queue only — single-operator overlay, no shared authority | Layer 4: remote-owned `operator_decision_heads` + CAS |
| Validation | Disabled (fields exist) | Layer 3: dry-run |
| Degraded-mode UX | Basic daemon-unavailable detection | Layer 4: three-state (healthy/degraded/offline) with distinct sync/daemon states |
| Web portal | Not present | Layer 4: read-only viewing portal |

### 1.14 Things NOT Implemented in Layer 2

- Cron scheduling (Layer 3)
- Resource arbiter (Layer 3)
- Worker-validator system (Layer 3/4)
- Sync engine (Layer 4)
- Web viewing portal (Layer 4)
- Multi-source marketplace search (Layer 5)
- Decision-grade landed cost (Layer 5)
- Bid workflow automation (Layer 6)
- Multi-user / RBAC (not in MVP)

### 1.15 Accessibility

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
    netMarginBasePct: number | null   // best candidate
    candidateCount: number
    quotedCandidateCount: number
  }
  decision?: {
    disposition: 'approve' | 'reject' | 'watch' | null
    isStale: boolean   // decisionBasisHash changed since decision
    decidedAt: string | null
  }
  rightRail?: React.ReactNode   // Slot for SyncAgeBadge, custom badges
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
- Markdown rendering (via `marked` or equivalent)
- Table formatting for pricing/margin data
- Blocker highlighting (red badge on sections with critical blockers)
- Section navigation sidebar
- Copy-to-clipboard for individual sections
- Print-friendly layout

#### `pipeline-badge.tsx` — Pipeline State Indicator

```tsx
interface PipelineBadgeProps {
  state: 'pending' | 'qualifying' | 'enriching' | 'qualified' | 'review' | 'rejected' | 'error'
  validationState?: 'unverified' | 'approved' | 'flagged' | 'rejected'  // Layer 3+
  size?: 'sm' | 'md' | 'lg'
}
```

**Display rules:**
- Color + text label + icon (never color alone — accessibility)
- `validationState` badge shown only when not `unverified` (Layer 3+)
- Tooltip with timestamp of last state change

#### `sync-age-badge.tsx` — Data Freshness Indicator

```tsx
interface SyncAgeBadgeProps {
  meta: {
    mode: 'healthy' | 'browse_only' | 'recovery_required'
    daemonConnected: boolean
    syncHealth: 'healthy' | 'degraded' | 'offline'
    lastSnapshotAt?: string
    lastSyncAt?: string           // Layer 4: last successful Supabase push
  }
}
```

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
  basisHash: string           // SHA-256 of inputs — staleness detection
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
