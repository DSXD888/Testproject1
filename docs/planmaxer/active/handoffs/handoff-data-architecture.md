# Handoff: Data Architecture & Sync

## Expert Framing

You are a **distributed systems engineer** with deep expertise in offline-first architectures, local-first software, sync protocols, and SQLite. You have built or worked on systems like Replicache, Electric SQL, PowerSync, Turso, or the Figma multiplayer sync engine. You understand the real-world pain of bidirectional sync between local and remote databases — conflict resolution, ordering, convergence, schema migration, and recovery.

Draw from: SQLite WAL mode and its concurrency properties, the CRDT literature on convergent replicated data types, sync protocol patterns (log-based, timestamp-based, version-vector-based), Postgres logical replication concepts, and the operational failure modes of hybrid local/cloud database architectures.

## Mission

ULTRATHINK and be meticulous.

You are reviewing the **data architecture and synchronization layer** of a UK procurement intelligence platform. This layer manages a dual-database architecture: local SQLite as the source of truth for pipeline operations, with Supabase/Postgres as the shared sync target for multi-user visibility.

The core value proposition for the data layer: local-first resilience (the daemon operates independently of network), corruption protection (local SQLite is independent of shared DB state), and multi-user visibility (Supabase provides shared access for web portal viewers and other operators).

**Don't just answer my questions — the questions I DON'T KNOW to ask are more important than the ones I do.**

## User's Strong Beliefs (Engage With These)

1. **"Local SQLite protects against remote DB corruption."** The owner added the dual-database architecture specifically because relying solely on a shared remote Postgres creates corruption risk. Challenge: what corruption scenarios does this actually protect against? Does it introduce new corruption vectors (sync bugs writing bad data to both)?

2. **"Conflict resolution is simple: local wins for pipeline data, remote wins for shared config."** Challenge: is this actually simple? What about data that's both pipeline-generated AND user-editable (e.g., an operator marks something in Supabase, then the daemon writes a conflicting update)? What about clock skew?

3. **"Push-pull sync with clear rules is sufficient."** The owner doesn't want CRDTs or complex sync frameworks — just push after pipeline completion, pull on startup and periodic heartbeat. Challenge: what edge cases make this break? What's the minimum viable sync protocol that actually converges?

## Essential Context

**What the project is:** A procurement intelligence pipeline that runs locally via a Rust daemon, stores data in SQLite, and syncs results to Supabase/Postgres for multi-user access. The desktop app reads local SQLite directly; the web viewing portal reads Supabase.

**Key terms:**
- **Source of truth (local SQLite):** All pipeline data is written here first. The daemon that ran the job is authoritative.
- **Shared sync target (Supabase/Postgres):** Multi-user visibility layer. Web portal reads from here. Also stores shared configuration (company profile, source configs).
- **Push (local → remote):** After pipeline stages complete, daemon pushes results to Supabase.
- **Pull (remote → local):** On startup and periodically, daemon pulls shared config and cross-operator decisions.
- **Tombstone deletes:** Shared records use `deleted_at` rather than hard deletes so pull-sync can converge.

**Technical constraints:**
- SQLite: single-writer model, no native arrays (JSON instead), no `gen_random_uuid()` (app-generated UUIDs), no `timestamptz` (TEXT ISO 8601)
- Supabase/Postgres: RLS-protected, supports realtime subscriptions
- Single daemon per machine for MVP
- Desktop reads local SQLite (fast, no network)
- Web portal reads Supabase (synced data, eventually consistent)

## Current Architecture

### Dual-Database Split
```
Local SQLite (Source of Truth)
  - All pipeline data written here first
  - Full PTY terminal output
  - Full error context
  - Cursor/watermark state
  - Job leasing and execution state

Supabase/Postgres (Shared Sync Target)
  - Multi-user visibility
  - Web portal data source
  - Shared configuration (company profile, source configs)
  - Redacted agent logs (no full terminal output)
  - Redacted error context
```

### Sync Protocol (Current Design)
```
PUSH (local → remote):
  - After each material pipeline stage completion
  - On manual sync trigger
  - Includes: normalized notices, qualification results, supplier quotes,
    landed-cost estimates, manifests, error manifests, job status

PULL (remote → local):
  - On daemon startup
  - Periodic heartbeat
  - Includes: company profile, source configs, operator decisions from other devices

CONFLICT RESOLUTION:
  - Local SQLite wins for pipeline data
  - Supabase wins for shared configuration and cross-operator decisions
```

### Change Tracking (Specified)
```
Mutable synced rows carry:
  - updated_at
  - sync_version
  - updated_by_executor_id
```

### Recovery Model
```
If Supabase is corrupted:
  → Local SQLite republishes pipeline data

If local SQLite is lost:
  → Rebuild shared-state tables from Supabase
  → Rerun ingestion/enrichment from retained raw documents
```

### Schema (Key Tables, Postgres Syntax)
```sql
-- Core tables: sources, source_cursors, executors, procurement_processes,
-- contract_notices, qualification_runs, product_candidates, supplier_quotes,
-- landed_cost_estimates, manifests, opportunity_decisions, workflow_jobs,
-- agent_session_logs, error_manifests, raw_documents
```

### SQLite-Specific Adaptations
- Arrays → TEXT with JSON arrays or junction tables
- UUID → application-generated
- Timestamps → TEXT ISO 8601
- JSONB → TEXT with json1 extension
- WAL mode for concurrent reads

## Tensions to Resolve

### Tension 1: Sync Complexity vs. Actual Multi-User Needs
**The conflict:** The architecture adds significant complexity (sync engine, conflict resolution, change tracking, recovery model, redaction) for multi-user visibility. But how many operators realistically use this system concurrently?

**Current resolution:** Full bidirectional sync engine with push-pull and conflict resolution.

**Why it might be wrong:** If there's one primary operator and 2-3 web viewers, this might be over-engineered. A simpler model (write-only push to Supabase after completion, no pull-back, no conflict resolution) might suffice. The "multiple operators making decisions from different devices" scenario drives most of the sync complexity — but is it a real MVP scenario?

**Stress test:** Operator A rejects a contract in the desktop app. Operator B (on another machine) simultaneously marks it as "watching" via... wait, the web portal is read-only. So who is Operator B? Where do cross-operator decisions come from?

### Tension 2: Schema Divergence Between SQLite and Postgres
**The conflict:** The same logical schema must exist in both SQLite and Postgres, but the type systems are different. `text[]` becomes JSON arrays, `gen_random_uuid()` becomes app-generated UUIDs, `timestamptz` becomes TEXT, `jsonb` becomes TEXT+json1. The SQL tables in the document are shown in Postgres syntax with a note about "appropriate type substitutions."

**Current resolution:** Implicit — the daemon team is expected to maintain parallel schemas.

**Why it might be wrong:** Schema drift between SQLite and Postgres will happen. A migration added to one but not the other. A column type that works in Postgres but behaves differently in SQLite (e.g., numeric precision, JSON path expressions). The sync engine depends on schema compatibility.

**Stress test:** A new column is added to `landed_cost_estimates` for a feature. It's added to the Postgres migration but the SQLite migration is forgotten. The sync engine tries to push data with the new column. What happens?

### Tension 3: Recovery Model Feasibility
**The conflict:** The plan claims that if local SQLite is lost, the daemon can "rebuild shared-state tables from Supabase and then rerun ingestion/enrichment from retained raw documents and manifests." But if Supabase only receives redacted summaries for agent logs and error context, recovery is lossy.

**Current resolution:** Stated but not fully specified.

**Why it might be wrong:** The recovery model depends on what's in Supabase being sufficient to bootstrap a new local SQLite. But the whole point of the architecture is that SQLite has MORE data (full terminal output, full error context, job leasing state, cursor/watermark state). Rebuilding from Supabase means losing all of this.

**Stress test:** Local SQLite file is corrupted (not lost — corrupted). How do you detect this? How do you know which records are trustworthy? Can you selectively rebuild?

### Tension 4: Sync Granularity and Ordering
**The conflict:** Push happens "after each material pipeline stage completion." But pipeline stages for different contracts can overlap. If Contract A's ingestion completes and pushes, then Contract B's enrichment completes and pushes, then Contract A's enrichment fails — the Supabase state for Contract A shows ingestion complete but no enrichment, which is confusing.

**Current resolution:** Not specified beyond "after completion."

**Why it might be wrong:** Without ordered, transactional sync, Supabase can show inconsistent snapshots of pipeline progress. Web viewers may see partial or confusing state.

**Stress test:** A daily pipeline run processes 50 contracts through multiple stages concurrently. Sync pushes happen interleaved. A web viewer refreshes the page. Do they see a coherent snapshot?

### Tension 5: Tombstone Accumulation and Convergence
**The conflict:** Deletes use tombstones (`deleted_at`) for sync convergence. But tombstones accumulate indefinitely unless garbage collected. The plan doesn't specify a garbage collection policy.

**Current resolution:** Tombstones exist, no GC specified.

**Why it might be wrong:** Over months of operation, tombstoned records will accumulate, slowing queries, bloating storage, and making sync slower. But garbage collecting tombstones too early risks the pull-sync missing deletes.

**Stress test:** After 6 months of operation, 30% of records in `supplier_quotes` are tombstoned. Performance degrades. How do you safely GC?

## What I Don't Want (Anti-Patterns)

- **Don't suggest using CRDTs or a complex sync framework.** The owner wants simple push-pull.
- **Don't suggest removing the dual-database architecture.** The corruption protection is a design requirement.
- **Don't suggest making the web portal read from local SQLite over a network.** The web portal reads from Supabase.
- **Don't suggest adding a message queue (Kafka, RabbitMQ) for sync.** Keep it simple.

## What I Do Want

1. **A concrete sync protocol specification** — what fields does each push/pull include? How is ordering maintained? What's the minimum viable sync that actually converges?

2. **Schema migration coordination strategy** — how do you keep SQLite and Postgres schemas in sync as the product evolves?

3. **Recovery model validation** — is the stated recovery model actually feasible? What's lost in each direction?

4. **Practical conflict resolution edge cases** — beyond "local wins for pipeline data" — what about the real edge cases?

5. **Ideas I haven't asked about** — sync failure modes, SQLite operational issues (VACUUM, WAL checkpointing, file locking), Postgres-side performance considerations, and patterns from production local-first systems.

6. **Recommendations in git-diff style** — reference specific sections of the research document (sections 4.7, 5.2, 5.4, 5.6, 5.8, 6.4) when proposing changes.

## Context Documents to Attach

When sending this to a reviewer, attach:
- The full research document: `docs/research/2026-03-08-public-procurement-intelligence-platform-research.md`
- Pay special attention to: Sections 4.7 (Database Architecture), 5.2 (Components — Sync Engine), 5.4 (Data Model), 5.6 (Ingestion Data Flow — sync step), 5.8 (Control Panel/Portal Data Flow), 6.4 (Performance)
