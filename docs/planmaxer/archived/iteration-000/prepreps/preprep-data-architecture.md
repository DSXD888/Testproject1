# Pre-Prep: Data Architecture & Sync

## Domain Ownership

This pre-prep owns modifications to these sections of the research document:
- **Section 4.7** — Database Architecture — Local SQLite + Supabase/Postgres Sync
- **Section 5.2** — High-Level Component Design (SQLite Local Store, Sync Engine components)
- **Section 5.4** — Data Model Design (ALL tables, SQLite compatibility notes, sync authority)
- **Section 5.8** — Control Panel and Viewing Portal Data Flow (data source sections)
- **Section 6.3** — Security Considerations (data protection, auth, RLS)
- **Section 6.4** — Performance Considerations (SQLite/Supabase performance)
- **ADR-002** — Local SQLite as Source of Truth with Supabase/Postgres Sync

**Shared with other domains:** Other domains may note field additions to tables in 5.4 — incorporate those. Section 5.8 frontend data flows are shared with frontend concerns (not a separate domain in this iteration).

---

## Step 1: Pre-Prep Research

Before integrating the reviewer's feedback, read and deeply understand:

1. **The full research document** — `docs/research/2026-03-08-public-procurement-intelligence-platform-research.md`
   - Focus heavily on sections 4.7, 5.4, 5.8, 6.3, 6.4
   - Understand the sync authority model (local wins for pipeline, remote wins for config)
   - Understand the change tracking fields (updated_at, sync_version, updated_by_executor_id)
   - Understand the recovery model (both directions)
   - Understand the SQLite compatibility adaptations (arrays, UUIDs, timestamps, JSONB)
   - Understand the tombstone-based delete strategy

2. **The handoff prompt** — `docs/planmaxer/active/handoffs/handoff-data-architecture.md`
   - Understand the 5 tensions: sync complexity vs needs, schema divergence, recovery feasibility, sync granularity, tombstone accumulation
   - Note anti-patterns and user beliefs

3. **Pay attention to:**
   - How sync is triggered (after pipeline stages, on startup, periodic heartbeat)
   - What data is redacted when synced to Supabase (terminal output, error context)
   - The relationship between RLS roles (viewer, operator, admin, daemon) and data access
   - Missing sync specification details (ordering, batching, retry)

## Step 1 Results: Deep Analysis of Current State

### 1.1 Sync Authority Model — Current Understanding

The document establishes a two-rule conflict resolution model (Section 5.4, lines 1789–1794):

| Data Category | Authority | Sync Direction | Redaction |
|---|---|---|---|
| Pipeline execution state | Local SQLite | Push (local→remote) | None for data; redacted for terminal output and error context |
| Job leasing/execution | Local SQLite | Push (local→remote) | Lease fields "mirrored for visibility only" |
| Full PTY terminal output | Local SQLite only | NOT synced (or redacted excerpt) | Full redaction |
| Full error context | Local SQLite only | NOT synced (or redacted excerpt) | Full redaction |
| Shared config (company profile, source configs) | Supabase | Pull (remote→local) | None |
| Cross-operator decisions | Supabase | Pull (remote→local) | None |

**Gap identified:** The document never specifies WHICH tables are push-only, pull-only, or bidirectional. The sync authority section (5.4, line 1789) states principles but doesn't map them per-table. This is the single biggest specification gap in the data architecture.

**Gap identified:** `opportunity_decisions` is ambiguous. It contains pipeline-generated fields (`gross_margin_pct`, `net_margin_pct`, `status` progressing through automated stages) AND operator-editable fields (`decided_by`, `decided_at`, `status` set to `operator_rejected` or `watching`). The "local wins for pipeline / remote wins for config" rule doesn't cleanly apply here. This is exactly Tension 2 from the handoff.

### 1.2 Change Tracking Fields — Current State

Section 5.4 (line 1791) states: "Mutable synced rows should carry `updated_at`, `sync_version`, and `updated_by_executor_id`" but adds "even where those repetitive columns are omitted from examples below for brevity."

**Gap identified:** These columns appear in ZERO of the actual SQL table definitions. Every table uses `created_at` but none include `updated_at`, `sync_version`, or `updated_by_executor_id`. The sync engine has no actual columns to work with in the current schema.

**Gap identified:** `sync_version` is mentioned once and never defined. Is it a monotonic counter? A logical clock? Per-row or per-table? The entire sync protocol depends on this field and it has no specification.

### 1.3 Recovery Model — Feasibility Analysis

Section 5.4 (lines 1793–1794) specifies two recovery directions:

**Recovery: Supabase corrupted → republish from local SQLite**
- Feasible for: all pipeline tables (notices, qualifications, quotes, costs, manifests, jobs, decisions)
- Feasible for: error manifests (local has full context)
- NOT feasible for: shared config that lives only in Supabase (company profile, source configs if modified remotely)
- **Assessment:** Mostly feasible. The one gap is shared config that originated in Supabase.

**Recovery: Local SQLite lost → rebuild from Supabase + rerun**
- Recoverable from Supabase: normalized notices, qualification results, quotes, landed costs, manifests, decisions, job summaries
- LOST permanently: full PTY terminal output, full error context, cursor/watermark state, job leasing history
- "Rerun ingestion/enrichment from retained raw documents" — but `raw_documents` stores `payload_json` or `storage_path`. If `storage_path` references local filesystem files, those are also lost.
- **Assessment:** Partially feasible but LOSSY. The document should acknowledge what's permanently lost. Cursor/watermark loss means the daemon will re-ingest everything from scratch on next run (which is actually recoverable, just expensive).

### 1.4 SQLite Compatibility — Table-by-Table Audit

Current tables using Postgres-only features that need SQLite adaptation:

| Table | Postgres Feature | SQLite Adaptation Needed |
|---|---|---|
| `contract_notices` | `text[] not null default '{}'` for `cpv_codes` | TEXT with JSON array `'[]'` |
| `product_candidates` | `text[] not null default '{}'` for `candidate_commodity_codes` | TEXT with JSON array `'[]'` |
| `raw_documents` | `jsonb` for `payload_json`, `metadata` | TEXT with json1 |
| `sources` | `jsonb not null` for `config` | TEXT with json1 |
| `qualification_runs` | `jsonb not null` for `evidence` | TEXT with json1 |
| `supplier_quotes` | `jsonb not null` for `evidence` | TEXT with json1 |
| `landed_cost_estimates` | `jsonb not null` for `warnings` | TEXT with json1 |
| `error_manifests` | `jsonb not null` for `error_context` | TEXT with json1 |
| `agent_session_logs` | `jsonb` for `structured_trace` | TEXT with json1 |
| `workflow_jobs` | `jsonb not null` for `input_payload`, `output_payload` | TEXT with json1 |
| ALL tables | `uuid primary key default gen_random_uuid()` | TEXT primary key, app-generated |
| ALL tables | `timestamptz not null default now()` | TEXT not null default (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')) |
| `raw_documents` | CHECK constraint syntax | Same syntax works in SQLite |
| `contract_notices` | UNIQUE constraint | Same syntax works in SQLite |

**Gap identified:** The document says "appropriate type substitutions" but never provides the actual SQLite DDL. The reviewer should be asked whether the document should include BOTH DDLs side by side, or whether a single canonical DDL (SQLite) with Postgres migration notes is better.

### 1.5 Tombstone Strategy — Current State

Section 5.4 (line 1792): "Deletes use tombstones (`deleted_at`) for shared records rather than hard deletes so pull-sync can converge safely."

**Gap identified:** No table in the current schema has a `deleted_at` column. Like `sync_version`, this is specified in prose but absent from the DDL.

**Gap identified:** No garbage collection policy is specified. Which records get tombstoned? All synced records, or only "shared records"? What's a "shared record" — everything in Supabase, or only config tables?

**Gap identified:** No specification of how tombstones interact with foreign keys. If a `procurement_process` is tombstoned, what happens to its `contract_notices`, `qualification_runs`, etc.? Cascade tombstone? Leave orphaned?

### 1.6 Sync Protocol — What's Missing

The handoff correctly identifies that the sync protocol is underspecified. Here's a complete inventory of what's stated vs. missing:

**Stated:**
- Push after material pipeline stage completion
- Push on manual sync trigger
- Pull on daemon startup
- Pull on periodic heartbeat
- Local wins for pipeline data
- Remote wins for shared config
- Change tracking via `updated_at`, `sync_version`, `updated_by_executor_id`
- Tombstone-based deletes

**NOT specified (needed for implementation):**
- Push/pull batch size or transaction boundaries
- Ordering guarantees (per-table? per-process? global?)
- Retry policy on sync failure (network down, Supabase unavailable)
- What happens to data that fails to sync (buffered? retried? flagged?)
- How `sync_version` is compared (last-write-wins? version vector?)
- Whether sync is per-row or per-table
- How the daemon knows which rows have changed since last sync (dirty flag? WAL inspection? `updated_at` comparison?)
- Whether sync is idempotent (can a push be safely replayed?)
- How the daemon identifies "cross-operator decisions" during pull (what query? what watermark?)
- Sync status visibility (can the operator see "last synced at" and "pending sync items"?)
- Partial sync failure handling (5 of 10 rows pushed, then network drops)

### 1.7 RLS Model — Current State (Section 6.3)

Four roles defined (line 2510–2514): `viewer`, `operator`, `admin`, `daemon`

**Gap identified:** No RLS policy specifications. The document says "RLS for all user-facing tables" but doesn't define which tables are user-facing, or what each role can see/do. Key questions:
- Can `viewer` see all processes, or only those in certain statuses?
- Can `viewer` see `supplier_quotes` and `landed_cost_estimates` (commercially sensitive data)?
- Can `operator` modify `opportunity_decisions` in Supabase directly? (This contradicts the "web portal is read-only" design unless operators also use the web portal)
- Does `daemon` use a service role or a dedicated RLS role? (Line 2509 says "prefer a dedicated daemon role exposed through RLS-safe RPCs")

### 1.8 Performance — SQLite-Specific Gaps (Section 6.4)

**Stated considerations:**
- WAL mode for concurrent reads (line 2549)
- Appropriate indexes (line 2549)
- Separate files for large terminal output logs (line 2557)
- Precomputed summary tables/materialized views for Supabase web portal (line 2550)

**NOT specified:**
- Which indexes are needed (no index definitions anywhere in the schema)
- VACUUM policy (SQLite doesn't auto-vacuum by default; database file grows monotonically without it)
- WAL checkpoint policy (WAL files can grow very large without periodic checkpointing)
- Maximum expected database size and when to consider archival
- Connection pooling strategy for Supabase (daemon pushing many rows)

### 1.9 Cross-Section Consistency Issues

1. **TypeScript interfaces vs SQL tables mismatch:**
   - `QualificationResult` has `rationale: string[]` but SQL `qualification_runs` has `evidence jsonb` — different field name, different type
   - `SupplierQuote` has `evidence: string[]` and `validUntil?: string` but SQL `supplier_quotes` has `evidence jsonb` and `valid_until timestamptz`
   - `LandedCostEstimate` has `warnings: string[]` but SQL has `warnings jsonb`
   - `OpportunityDecision` has `decidedBy?: string` but SQL has `decided_by uuid` — type mismatch (string vs uuid)

2. **Section 5.2 Sync Engine component** (line 1712–1714) is a 3-line description. For the most complex component in the system, this needs significant expansion.

3. **Section 5.6 sync step** (line 2218) says "Results are synced to Supabase for multi-user visibility" — one line for the entire sync operation. Similar one-liners at lines 2260 and 2277 for supplier and landed-cost sync steps.

### 1.10 Tension Analysis — Reviewer Preparation

**Tension 1 (Sync Complexity vs. Needs):** The handoff correctly identifies that the web portal is read-only and asks where "Operator B" makes cross-operator decisions. Currently the ONLY place decisions could come from remotely is another desktop app (another daemon instance). For MVP with a single daemon, the pull direction carries only shared config — no cross-operator decisions exist. This dramatically simplifies the sync protocol for MVP.

**Tension 2 (Schema Divergence):** The current document provides zero tooling or process for keeping schemas in sync. This is a real operational risk. The reviewer should propose a migration coordination strategy.

**Tension 3 (Recovery Feasibility):** As analyzed in 1.3 above, recovery is lossy in both directions. The document should honestly state what's lost.

**Tension 4 (Sync Granularity/Ordering):** With no transaction boundaries or ordering specified, concurrent pipeline runs WILL produce interleaved, potentially confusing Supabase state. The reviewer should propose minimum viable ordering guarantees.

**Tension 5 (Tombstone Accumulation):** No GC policy, no tombstone columns in DDL. The reviewer should propose a practical tombstone lifecycle.

---

## Step 2: Feedback Integration

**Status: EXECUTED on 2026-03-12**

Reviewer response: distributed systems engineer with expertise in offline-first architectures and sync protocols.

### Triage Results

| # | Recommendation | Verdict |
|---|---|---|
| 1 | Authority matrix replacing singular "source of truth" | **Agree strongly** — Integrated |
| 2 | Split `opportunity_decisions` into 3 tables | **Agree strongly** — Integrated |
| 3 | Local outbox pattern (same txn as business write) | **Agree strongly** — Integrated |
| 4 | Push batch envelope with full-row projections | **Agree strongly** — Integrated |
| 5 | Remote apply rules (dedup, gap rejection, row_version) | **Agree strongly** — Integrated |
| 6 | Pull protocol with `remote_change_seq` | **Agree strongly** — Integrated |
| 7 | Stage ordering (`stage_run_id`, stage heads/status) | **Agree strongly** — Integrated |
| 8 | Narrow tombstone scope + GC policy | **Agree strongly** — Integrated |
| 9 | Schema migration coordination (`schema_epoch`, CI) | **Agree strongly** — Integrated |
| 10 | Recovery model honesty (warm vs cold rebuild) | **Agree strongly** — Integrated |
| 11 | One active daemon per org in MVP | **Agree strongly** — Integrated |
| 12 | Integer pence for money, basis points for percentages | **Partially agree** — Integrated for synced monetary fields in new split tables |
| 13 | Junction tables for cpv_codes, commodity_codes | **Partially agree** — Added junction tables; kept JSON for opaque evidence/payloads |
| 14 | SQLite operational guidance (WAL, PRAGMA, backups) | **Agree strongly** — Integrated |
| 15 | Supabase Realtime guidance (projection events) | **Agree strongly** — Integrated |
| 16 | Compare-and-swap for shared human decisions | **Agree strongly** — Integrated |
| 17 | Config revision snapshot into jobs | **Agree strongly** — Integrated |
| 18 | Split shared config vs local secrets | **Agree strongly** — Integrated |
| 19 | Ban mixed-write tables | **Agree strongly** — Integrated |
| 20 | STRICT tables for SQLite | **Partially agree** — Noted in SQLite Local Store component for tables not using json1 |

### Integration Rules

For each proposed change from the reviewer:

1. **Agree strongly** — Integrate directly.
2. **Partially agree** — Integrate core insight, adapt to conventions.
3. **Disagree** — Do NOT integrate. Note why.

### Integration Checklist

- [x] Preserve ADR-002 numbering — Updated ADR-002 with authority matrix model
- [x] If modifying SQL tables, provide BOTH Postgres syntax (for Supabase) AND note SQLite adaptations needed — SQLite compatibility notes in §5.4 header; new tables use compatible types
- [x] If adding new columns to existing tables, ensure they're compatible with BOTH databases — sync tracking columns use text/integer (compatible)
- [x] If modifying the sync protocol, update ALL places it's referenced — Updated §4.7 (full protocol), §5.2 (Sync Engine component), §5.4 (Sync Authority), §5.6 (ingestion sync step), §5.7 (supplier/landed cost sync steps), §5.8 (portal data flow)
- [x] If adding new sync-related tables, include RLS considerations — §6.3 expanded with authority-based RLS per role
- [x] If modifying the recovery model, validate it's actually feasible given what each database stores — Honest warm/cold rebuild tiers with explicit loss acknowledgment
- [x] Maintain the redaction boundary (what goes to Supabase vs. stays local) — Preserved in §6.3 and §5.4 table comments
- [x] Update the Change Log in Appendix E — Entry added

### What You Can Edit Directly
- Section 4.7 — full ownership
- Section 5.2 (SQLite Local Store, Sync Engine components) — full ownership
- Section 5.4 — full ownership of ALL tables, schema, and sync authority
- Section 5.8 — data source sections
- Section 6.3 — full ownership
- Section 6.4 — full ownership
- ADR-002 — can update
- Section 8.2 (Files to Create) — Database/Infra sections
- Section 9.4 (Migration Requirements) — full ownership

### What Requires Coordination
- Section 5.6 (Ingestion Data Flow) — sync step at end; note changes for agent-orchestration domain
- Other sections that reference specific table names or fields

---

## Step 3: Fresh-Eyes Optimization

After integrating:

1. **Schema completeness check:** Does every entity in the TypeScript interfaces have a corresponding SQL table? Do the columns match?

2. **SQLite/Postgres parity check:** Is every SQL table specified in a way that works for both databases? Are the type adaptations documented?

3. **Sync completeness check:** Does every table have clear sync direction (push, pull, or both) and authority (local or remote)?

4. **RLS completeness check:** Are all user-facing tables covered by RLS? Are the four roles (viewer, operator, admin, daemon) sufficient?

5. **Cross-domain check:** Did other domain pre-preps note field additions? Incorporate them into the schema.

---

## Step 4: Sync Pass

1. Update Change Log in Appendix E.
2. Ensure section 9.4 (Migration Requirements) reflects any new migration files needed.
3. If new tables were added, ensure they appear in section 8.2 (Files to Create) migration listings.
4. Verify the Table of Contents is still accurate.
