# Layer 4 — Shared Visibility

> **Authority:** This document is the execution-grade specification for Layer 4. An implementation agent should be able to build this layer from this document alone without reopening the research document.
>
> **Milestone:** M4
> **Dependencies:** Layers 0-3 (all pipeline data, all push-owned tables with sync tracking columns)
> **Exit criteria:** Two users view the same pipeline: desktop operator (read-write) + web viewer (read-only). Sync engine operational. Validation enforcement active.

---

## 1. Overview & Sync Engine

### 1.1 Layer Narrative

Collaboration infrastructure. The sync engine activates, projecting pipeline data to Supabase for the read-only web viewing portal. Operator decisions flow through Supabase CAS. Worker-validator enforcement mode is enabled after dry-run calibration. Two users can now view the same opportunity pipeline — one via desktop (read-write), one via web (read-only).

**Why this layer exists:** Layers 1-3 proved the pipeline works for a single operator on a single machine. Layer 4 makes the pipeline output visible to others and enables shared decision-making. The sync engine is the bridge between local operational authority and shared visibility.

**If we never build Layer 5:** Multi-user visibility works with screening-grade accuracy and single-marketplace pricing. The system is production-ready for the initial category whitelist.

### 1.2 Capability Delivered

Two users view the same pipeline: desktop operator (read-write) and web viewer (read-only). Sync engine pushes pipeline data to Supabase. Operator decisions use compare-and-swap via Supabase. Validator rejections quarantine results in enforcement mode. Domain-specific validator overlays for procurement, supplier enrichment, document acquisition, and product extraction.

### 1.3 Push Protocol (Local -> Supabase)

**Transactional outbox pattern:** From Layer 4 activation, every push-owned mutation creates a `local_outbox` entry in the same SQLite transaction as the data write. No data mutation without outbox entry.

**Push dispatcher contract:**
- Single-in-flight batch (one batch at a time per push stream)
- Identical-envelope retries on failure (same content, same order)
- Batch boundaries never cross `schema_epoch` or `stage_run_id`
- Batch boundaries never split a `commit_group_id` (preserves local SQLite transaction atomicity)
- `row_version` ordering within batch (monotonic per row)
- `content_hash` for idempotent deduplication
- Every batch carries `writer_epoch` (see §1.6 Active-Writer Lease)

**Push batch RPC:** `apply_push_batch(batch_json, writer_epoch)` — single transactional Supabase RPC. Batch either fully applies or fully rejects. No partial application. Remote validates `writer_epoch` matches current lease holder.

**Equal-version retry semantics:** The remote apply logic must define:
- Incoming `row_version` = remote `row_version` AND `content_hash` matches → **no-op success** (idempotent retry)
- Incoming `row_version` = remote `row_version` but `content_hash` differs → **hard conflict** (data corruption)
- Incoming `row_version` = remote `row_version` + 1 → **apply** (normal advance)
- Anything else (older, larger jump) → **reject** (stale or out-of-order)

Without defining the equal-version case, identical-envelope retries fail spuriously.

**Outbox entry structure:**

```sql
-- Each outbox entry (from L0 §2 DDL):
-- local_seq (autoincrement), executor_id, entity_kind, entity_id,
-- op (insert/update), row_version, stage_run_id, schema_epoch,
-- commit_group_id (groups rows from same SQLite transaction),
-- writer_epoch, payload_json, content_hash, created_at
```

**Provisional promotion buffer for enforcement slices:**

For slices where validation enforcement is active, worker outputs must NOT be written directly to push-owned tables before validation completes. Without this, rejected rows have already been committed and outboxed — making "quarantine" post-hoc theater.

For enforcement-active slices, the write flow becomes:
1. Worker completes → OQG runs
2. Worker outputs stored as **local-only provisional results** (artifact JSON + result contract blob in a staging table or artifact directory)
3. Validator runs against provisional results
4. If approved: promote to push-owned tables + create outbox entries + advance stage heads (normal path)
5. If rejected: keep artifacts and verdicts locally; do NOT promote to authoritative push-owned state; create error manifest

For dry-run slices and non-enforcement slices, the current direct-write path continues unchanged.

### 1.4 Pull Protocol (Supabase -> Local)

**Cursor-based pull with hi-water fence:** Per-domain pull cursors track `remote_change_seq`. Three pull domains:
- `shared_source_configs` — source enablement and configuration
- `shared_company_profile` — org-wide bidder profile
- `operator_decision_heads` — operator dispositions with CAS

**Hi-water fenced pull algorithm:**
1. Read current `hiwater = max(remote_change_seq)` for the domain (snapshot fence)
2. Fetch rows where `cursor < change_seq <= hiwater` (bounded range)
3. Apply to local SQLite in `change_seq` order (upsert based on primary key)
4. Advance cursor to `hiwater`

Without a hi-water fence, paginated pulls can skip or duplicate rows when remote changes occur during pagination. Per-domain monotonic pull fences are required; cross-domain serializability is not.

**Pull-before-push ordering:** On sync startup, pull remote-owned state FIRST, then classify/coalesce pending intents, then resume push. This ensures cross-authority consistency.

**Pending operator intent reconciliation:** After pull, reconcile `pending_operator_intents` queue:
1. Compare each queued intent against freshly pulled `operator_decision_heads`
2. If intent's `base_remote_revision` matches current remote revision → flush via CAS RPC
3. If revision mismatch → conflict, update local queue status, surface to operator
4. Coalesce multiple intents for same process (latest-per-process)

### 1.5 Compare-and-Swap (CAS) for Operator Decisions

**CAS RPC:** `update_operator_decision(process_id, disposition, base_revision, command_id)`
- `base_remote_revision` checked — mismatch means someone else changed it
- `command_id` for idempotent retry (lost-response handling) via **durable command ledger**
- Conflict returned (not silently overwritten)
- Desktop shows conflict resolution UI

**Durable command ledger:**

Storing `command_id` only on the `operator_decision_heads` row is insufficient — it only remembers the latest command. A retried command after a later command has already landed cannot be distinguished from a stale conflict. A separate command ledger provides true idempotency:

```sql
CREATE TABLE operator_decision_commands (
  command_id TEXT PRIMARY KEY,
  process_id TEXT NOT NULL,
  base_revision BIGINT NOT NULL,
  applied_revision BIGINT,          -- NULL if conflict
  disposition TEXT NOT NULL,
  result TEXT NOT NULL,              -- 'applied', 'conflict', 'superseded'
  response_payload_json JSONB,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
```

CAS flow with command ledger:
1. Check command ledger for `command_id` — if present, return stored result (idempotent replay)
2. Compare `base_revision` against current `operator_decision_heads` row
3. If match: apply, increment revision, write command ledger row transactionally
4. If mismatch: write conflict entry to command ledger, return conflict

Command ledger entries are GC'd after a configurable retention period (default 30 days).

### 1.6 Active-Writer Lease (Fenced)

One daemon per org in MVP. Enforced at push protocol level with **writer fencing token**:
- Daemon registers with `executor_id` on first push
- Second daemon attempting to push → rejected with lease conflict
- Lease heartbeat via `viewer_sync_status` updates

**Fencing token:** Without a fencing token, a stale daemon (partitioned, then recovered) can race a newly-leased daemon. The remote lease row carries:

```sql
-- In schema_metadata or dedicated lease table:
-- org_id, holder_executor_id, lease_expires_at, writer_epoch (integer, incremented on acquisition)
```

On lease acquisition, increment `writer_epoch`. Every `apply_push_batch` must include the current `writer_epoch`. Remote rejects batches with stale epoch. This is the correct MVP single-writer fence.

**Operational rule (MVP):** One bound machine per org. Other installations are viewer-only unless explicitly rehomed. Active-writer lease protects remote pushing but does not prevent a second offline daemon from creating divergent local state. Document this as an operational constraint.

### 1.7 Schema Epoch Guard

`schema_epoch` (integer) on every push batch and in `PRAGMA user_version`:
- If batch epoch != remote epoch → sync pauses
- Local operation continues uninterrupted
- Operator/admin must resolve epoch mismatch before sync resumes
- Prevents schema-mismatched data from corrupting the remote projection

### 1.8 Degraded Mode Behavior

| Sync State | Desktop | Web |
|-----------|---------|-----|
| Healthy | Full read-write-trigger | Full read-only with live Broadcast updates |
| Degraded (push failing) | Full local operation; push queues in outbox | Stale data (amber indicator); Broadcast stopped |
| Offline (Supabase unreachable) | Full local operation; decision capture continues into local queues until CAS flush resumes | Red staleness indicator; last-known data |

Desktop continues operating on local SQLite regardless of sync state. Web reflects last-pushed data with staleness indicator.

**Remote config staleness classes:**

If pull breaks, the daemon continues with stale remote-owned config. Not all config is equally safe to use stale:

| Config | Class | Stale Behaviour |
|--------|-------|----------------|
| `shared_company_profile` | Informational | Stale OK — used only for bid form filling (L6) |
| `shared_source_configs` (display/schedule) | Operational | Warn after 1 hour; use cached values |
| `shared_source_configs` (enabled/disabled) | Safety-critical | If a source is remotely disabled (legal/compliance), an offline daemon must NOT scrape it indefinitely. Degrade that source after configurable TTL (default 4 hours) |
| `operator_decision_heads` | Operational | CAS protects correctness on push; stale pull means decisions may not reflect latest remote state |

**Backup and durability:** Local SQLite authority without backup is availability theater. Encrypted WAL-aware snapshots should be taken periodically. Single-device durability is a known MVP limitation — if the machine dies before push, authoritative data is lost.

### 1.9 Invariants Enforced

| Invariant | Type | Enforcement |
|-----------|------|-------------|
| Push batch atomicity | Constitutional | Single transactional RPC, not individual upserts |
| Outbox + data in same SQLite transaction | Constitutional | Every push-owned mutation creates outbox entry atomically |
| CAS for operator decisions | Constitutional | `base_remote_revision` checked; mismatch -> conflict |
| CAS command ledger for idempotency | Constitutional | Durable `operator_decision_commands` table; retry returns stored result |
| Active-writer lease with fencing token | Constitutional | `writer_epoch` on every batch; stale epoch -> rejected |
| `schema_epoch` on every batch | Constitutional | Incompatible epoch -> sync pauses |
| Batch boundaries never cross `stage_run_id` | Constitutional | Push dispatcher enforces |
| Batch boundaries never split `commit_group_id` | Constitutional | Preserves local SQLite transaction atomicity |
| Single-in-flight batch | Constitutional | One batch at a time per push stream |
| Equal-version retry = no-op on content_hash match | Constitutional | Identical-envelope retries must not fail spuriously |
| Provisional promotion for enforcement slices | Constitutional | Worker outputs not committed to push-owned tables until validator completes |
| Validation enforcement | Operational | Rejections quarantine results after dry-run calibration |
| RLS on all Supabase tables | Constitutional | Policy matrix at database boundary |
| Pull-before-push ordering | Constitutional | Pull remote state FIRST on startup |
| Hi-water fenced pull | Constitutional | Per-domain bounded range pull prevents skip/duplicate during pagination |

### 1.10 Components Touched

| Component | Action | Files |
|-----------|--------|-------|
| Database | Create | `0002_sqlite_state_sync.sql` (SQLite sync tables), 8 Supabase migrations, `infra/supabase/config.toml` |
| `crates/daemon` | Create | `src/db/sync.rs` (push-pull sync engine) |
| Skills | Create | 4 domain validator overlays |
| `apps/web` | Create | 7 files (full Next.js viewing portal) |

---

## 2. Supabase Schema, RLS & Remote-Owned Tables

> **Ownership:** This section owns the Supabase-side schema: projection tables, summary tables, remote-owned tables, junction tables, RLS policies, and migration listing. Push/pull protocol mechanics are owned by §1 (Sync Engine). SQLite schema is owned by L0 §2 and L1 §3.

### 2.1 Supabase Projection Tables (Push-Owned)

All 18 push-owned SQLite tables get Supabase counterparts. These are **push-only projections** — the daemon pushes via `apply_push_batch(batch_json)` RPC. The local SQLite copy is authoritative; Supabase is a read-only mirror for the web portal.

| # | Table | Scope | Notes |
|---|-------|-------|-------|
| 1 | `procurement_processes` | Process | Canonical process entity |
| 2 | `contract_notices` | Notice | Source-specific notice records |
| 3 | `notice_lots` | Lot | Per-notice lot decomposition |
| 4 | `tender_document_sets` | Process | Acquired document sets |
| 5 | `qualification_runs` | Lot | Per-lot qualification results |
| 6 | `product_candidates` | Lot | Extracted product candidates |
| 7 | `search_profiles` | Candidate | Structured search profiles |
| 8 | `search_attempts` | Candidate | Per-wave search execution records |
| 9 | `supplier_quotes` | Candidate | Marketplace/RFQ quotes |
| 10 | `candidate_pricing_assessments` | Candidate | Gross margin gate results |
| 11 | `landed_cost_estimates` | Candidate-quote | Per-candidate-quote cost estimates |
| 12 | `manifests` | Process | Generated opportunity manifests |
| 13 | `pipeline_assessments` | Process | Process-level pipeline rollup |
| 14 | `bid_workflow_states` | Process | Bid automation state (dormant until L6) |
| 15 | `process_stage_status` | Scoped | Per-stage execution records |
| 16 | `process_stage_heads` | Scoped | Current stage head per scope |
| 17 | `process_relations` | Process | Inter-process relationship edges |
| 18 | `process_aliases` | Process | Merged-process alias redirects |

Schema for these tables mirrors the SQLite DDL from L0 §2 with Postgres type substitutions (TEXT → text, INTEGER → integer/bigint, etc.). All push-owned columns (`owner_id`, `row_version`, `last_local_seq`, `stage_run_id`, `created_at`, `updated_at`) are present.

### 2.2 Summary Tables (Push-Owned Projections)

Redacted/summarized views of local-only tables, plus a read-only decision summary projection of remote-owned dispositions, pushed to Supabase for web portal visibility.

| Table | Source | Purpose | Key Fields |
|-------|--------|---------|-----------|
| `workflow_job_summaries` | `workflow_jobs` (local-only) | Redacted job status for web | `job_id`, `job_kind`, `status`, `skill_path`, `started_at`, `completed_at`, `error_type` (no full context) |
| `error_manifest_summaries` | `error_manifests` (local-only) | Redacted error context for web | `manifest_id`, `job_id`, `error_type`, `error_code`, `incident_signature`, `created_at` (no full context_json or markdown) |
| `validation_verdict_summaries` | `validation_verdicts` (local-only) | Verdict status for web | `verdict_id`, `worker_job_id`, `verdict`, `basis_sufficiency`, `enforcement_eligible`, `created_at` (no full reasoning) |
| `viewer_sync_status` | Daemon sync state | Sync freshness indicator | `sync_health` (healthy/degraded/offline), `last_push_success_at`, `pending_outbox_count`, `schema_epoch` |
| `executors` | `executors` (push-owned) | Daemon heartbeat summary | `executor_id`, `heartbeat_at`, `daemon_version`, `started_at` |
| `operator_decision_summaries` | `operator_decision_heads` (remote-owned, pulled locally) | Decision display for web viewers | `process_id`, `disposition`, `decision_stale`, `reviewed_at_stage_run_id`, `updated_at` (no operator identity, no company profile, no command history) |

### 2.3 Remote-Owned Tables

These tables originate in Supabase and are pulled to local. The daemon does NOT write to these — it only reads (pulls).

#### shared_company_profile

Org-wide bidder profile used by the bid workflow (Layer 6).

```sql
CREATE TABLE shared_company_profile (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  org_id text NOT NULL,
  company_name text NOT NULL,
  registration_number text,
  vat_number text,
  address_json jsonb,
  contact_json jsonb,
  bid_profile_json jsonb,       -- portal-specific form field defaults
  remote_change_seq bigint NOT NULL DEFAULT 1,
  remote_revision bigint NOT NULL DEFAULT 1,
  deleted_at timestamptz,       -- tombstone for soft-delete
  created_at timestamptz NOT NULL DEFAULT now(),
  updated_at timestamptz NOT NULL DEFAULT now()
);
```

#### shared_source_configs

Source enablement and configuration. Controls which procurement/supplier sources are active and their scheduling parameters.

```sql
CREATE TABLE shared_source_configs (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  org_id text NOT NULL,
  source_id text NOT NULL UNIQUE,
  source_type text NOT NULL CHECK (source_type IN ('procurement', 'supplier')),
  display_name text NOT NULL,
  enabled boolean NOT NULL DEFAULT true,
  skill_path text NOT NULL,                       -- e.g., 'procurement-ingestion/fts'
  schedule_cron text,                              -- cron expression for scheduled runs
  config_json jsonb,                               -- source-specific configuration
  remote_change_seq bigint NOT NULL DEFAULT 1,
  remote_revision bigint NOT NULL DEFAULT 1,
  deleted_at timestamptz,
  created_at timestamptz NOT NULL DEFAULT now(),
  updated_at timestamptz NOT NULL DEFAULT now()
);
```

#### operator_decision_heads

Operator dispositions with compare-and-swap semantics. This is the **remote-authoritative** decision table — before Layer 4, operator decisions exist only as local intents in `pending_operator_intents`.

```sql
CREATE TABLE operator_decision_heads (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  org_id text NOT NULL,
  process_id text NOT NULL UNIQUE,
  disposition text NOT NULL CHECK (disposition IN ('pending', 'watching', 'approved', 'rejected')),
  reviewed_basis_hash text,                        -- hash of candidates/quotes/estimates reviewed
  reviewed_manifest_hash text,                     -- hash of manifest read
  reviewed_at_stage_run_id text,                   -- stage run decision was based on
  decision_stale boolean NOT NULL DEFAULT false,    -- set true when decisionBasisHash changes or L5 freshness vectors expire
  operator_id text,                                 -- who made the decision
  command_id text,                                  -- latest applied CAS command for audit/debug; durable idempotency lives in operator_decision_commands
  remote_change_seq bigint NOT NULL DEFAULT 1,
  remote_revision bigint NOT NULL DEFAULT 1,
  deleted_at timestamptz,
  created_at timestamptz NOT NULL DEFAULT now(),
  updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX idx_decisions_process ON operator_decision_heads(process_id);
CREATE INDEX idx_decisions_disposition ON operator_decision_heads(disposition);
```

**CAS semantics:** Updates via `update_operator_decision(process_id, disposition, base_remote_revision, command_id)` RPC. Mismatch on `base_remote_revision` → conflict returned (not silently overwritten). `command_id` provides idempotency for lost-response retries. Direct table UPDATE is blocked — only the RPC path is exposed.

### 2.4 Junction Tables (Supabase)

Mirrored from SQLite for web portal querying:

| Table | Purpose |
|-------|---------|
| `notice_cpv_codes` | Notice ↔ CPV code mapping |
| `product_commodity_codes` | Product ↔ commodity code mapping |
| `lot_cpv_codes` | Lot ↔ CPV code mapping |

### 2.5 RLS Policy Matrix

Row-Level Security policies enforce the authority matrix at the Supabase database boundary. All tables scoped by `org_id` (implicit single-org in MVP).

| Table Category | Tables | `viewer` | `operator` | `admin` | `daemon` |
|---|---|---|---|---|---|
| Push-owned projections | All 18 projection tables | SELECT | SELECT | SELECT | INSERT, UPDATE (via `apply_push_batch` RPC) |
| Push-owned summaries | 5 summary tables + executors | SELECT | SELECT | SELECT | INSERT, UPDATE (via `apply_push_batch` RPC) |
| Remote-owned config | `shared_company_profile` | — | — | Full CRUD | SELECT (pull) |
| Remote-owned config | `shared_source_configs` | — | — | Full CRUD | SELECT (pull) |
| Remote-owned decisions | `operator_decision_heads` | — | SELECT, UPDATE (via CAS RPC) | Full CRUD | SELECT (pull) |
| Decision display projection | `operator_decision_summaries` (push-owned) | SELECT | SELECT | SELECT | INSERT, UPDATE (via `apply_push_batch` RPC) |
| Sync infrastructure | `schema_metadata`, lease table | — | — | SELECT, UPDATE | SELECT, UPDATE |
| CAS infrastructure | `operator_decision_commands` | — | — | SELECT | INSERT (via CAS RPC), SELECT |

**Web read-only is a security boundary, not a UI convention:**
- The web portal uses **viewer-scoped credentials only** — the browser must not possess operator or admin mutation rights
- `viewer` role has SELECT on push-owned projections and summaries only
- `viewer` and `operator` roles cannot access `shared_company_profile` or `shared_source_configs` directly — config display is surfaced through push-owned summary projections where appropriate
- Operator decisions are exposed to web viewers via a read-only `operator_decision_summaries` push-owned projection, not via the raw remote-owned `operator_decision_heads` table
- `operator` role accesses `operator_decision_heads` only from the desktop via daemon-mediated CAS

**Implementation rules:**
- Push-owned tables: daemon writes through `apply_push_batch(batch_json, writer_epoch)` RPC that validates `schema_epoch`, checks `writer_epoch`, checks `row_version` ordering, applies atomically. RLS grants INSERT/UPDATE only to `daemon` role.
- Operator decisions: `update_operator_decision()` RPC enforces CAS with durable command ledger. Direct UPDATE blocked.
- All SELECT policies include `WHERE deleted_at IS NULL` for tombstoned tables (excludes soft-deleted rows from portal views).
- **RLS tombstone visibility fix:** The daemon pull query must see tombstoned rows (to propagate deletions locally). Daemon role SELECT policy does NOT filter on `deleted_at IS NULL` for remote-owned tables.
- `viewer` role cannot see sync infrastructure, CAS infrastructure, or remote-owned tables.
- `operator` role cannot see sync infrastructure or CAS infrastructure.

**Redaction boundary testing:** Add explicit redaction tests for every summary projection to verify no leakage of operator identity, company profile, raw supplier data, tender document metadata, or full error context.

### 2.6 Supabase Migrations

Eight migration files create the full Supabase schema:

| # | Migration | Purpose |
|---|-----------|---------|
| 1 | `supabase/migrations/0001_projection_tables.sql` | CREATE TABLE for all 18 push-owned projection tables + 3 junction tables. Schema mirrors SQLite DDL with Postgres types. |
| 2 | `supabase/migrations/0002_remote_owned_tables.sql` | CREATE TABLE for `shared_company_profile`, `shared_source_configs`, `operator_decision_heads`. |
| 3 | `supabase/migrations/0003_summary_tables.sql` | CREATE TABLE for 5 summary tables: `workflow_job_summaries`, `error_manifest_summaries`, `validation_verdict_summaries`, `viewer_sync_status`, `executors`. |
| 4 | `supabase/migrations/0004_rls_policies.sql` | RLS policies for all 4 roles (viewer, operator, admin, daemon) across all table categories. |
| 5 | `supabase/migrations/0005_sync_rpcs.sql` | `apply_push_batch(batch_json)` transactional RPC + `update_operator_decision(process_id, disposition, base_revision, command_id)` CAS RPC. |
| 6 | `supabase/migrations/0006_schema_metadata.sql` | `schema_metadata` table with `schema_epoch` + active-writer lease row with `writer_epoch` for fencing. |
| 7 | `supabase/migrations/0007_command_ledger.sql` | `operator_decision_commands` table for durable CAS idempotency. |
| 8 | `supabase/migrations/0008_decision_summaries.sql` | `operator_decision_summaries` push-owned projection for web viewer access. |

**Schema coordination rule:** Both SQLite and Postgres derive from the same canonical logical schema. CI must apply both migrations from scratch, introspect both, and fail if logical schema hash differs.

**SQLite sync migration:** `sqlite/migrations/0002_sqlite_state_sync.sql` — adds `local_outbox`, `sync_batches`, `pull_cursors` tables to SQLite (see §1 for DDL). This is a SQLite-only migration, separate from the Supabase migration namespace above.

### 2.7 Cross-Authority Consistency

Pipeline data crosses authority boundaries at specific points. Each crossing has a classification:

| Crossing Point | Authority Domains | Classification | Provenance Tracking |
|---|---|---|---|
| Pipeline output → operator review | Push-owned → display | **Display** — no authority conflict | `decisionBasisHash` links decision to pipeline snapshot |
| Operator decision → bid workflow | Remote-owned → push-owned automation | **Automation** — must track decision provenance | `reviewedAtStageRunId`, `reviewedBasisHash` carried through |
| Coverage metrics → qualification gate | Push-owned → push-owned control | **Control** — no authority crossing | Internal to pipeline |
| Sync health → UX mode | Push-owned infra → display | **Display** — no authority conflict | `viewer_sync_status` drives 3-state UX |

**Provenance of remote-owned config used in a run:** Record the exact revisions consumed by each pipeline run: `source_config_revision` (from `shared_source_configs.remote_revision` at pull time), `company_profile_revision` (if consumed, e.g., L6 bid workflow), `operator_decision_revision` (when a workflow consumes a decision). This enables replay and blame assignment when something goes wrong.

---

## 3. Web Portal & Validation Enforcement

### 3.1 Web Viewing Portal (`apps/web/` -- 7 files)

The web viewing portal is a **read-only** hosted client. It reads Supabase projection tables and receives live invalidation hints via Supabase Broadcast. It does NOT support mutations, decision capture, agent management, or configuration changes.

**Architecture:**

```text
Next.js / React
  -> Supabase snapshot queries (projection tables)
  -> Supabase Broadcast private channels (invalidation hints)
  -> refetch / patch read-only projections
```

**7 files:**

| File | Responsibility |
|------|---------------|
| `src/app/layout.tsx` | Root layout with global sync status badge (`viewer_sync_status`) |
| `src/app/page.tsx` | Redirect to contracts list |
| `src/app/contracts/page.tsx` | Keyset-paginated opportunity list using Supabase client + `packages/ui` `ContractCardFrame`. Shows "N new/updated" pill on live updates (not live reorder). |
| `src/app/contracts/[id]/page.tsx` | Opportunity detail: candidates, quotes, margins, manifest, errors. Auto-refresh for minor changes; non-destructive refresh chip for major changes. |
| `src/app/status/page.tsx` | Pipeline status, job summaries, error summaries, sync health |
| `src/lib/supabase.ts` | Supabase client initialization, typed query helpers using `packages/contracts-schema` types |
| `src/lib/broadcast.ts` | Supabase Broadcast subscription manager: topic-based invalidation, per-page subscription lifecycle |

**Web-specific data-fetching:** The web uses Supabase client for all data access. It reuses `packages/ui` presentational components (ContractCardFrame, ManifestViewer, PipelineBadge, FreshnessBadge, ErrorSummaryList) but has its own data-fetching hooks. No SQLite, no daemon bridge, no `invoke()`.

### 3.2 Supabase Broadcast Invalidation

The portal uses **Supabase Broadcast as an invalidation plane**, not as a database subscription. Broadcast payloads are small invalidation hints, not full rows.

**Why Broadcast over Postgres Changes:** Postgres Changes is simpler but scales worse (single-threaded for ordering). Broadcast supports private channels, custom payloads via `realtime.send`, and better security scoping.

**Topic scheme (page-level, not row-level):**

| Page | Topics |
|------|--------|
| `/contracts` | `org:{orgId}:contracts`, `org:{orgId}:sync` |
| `/contracts/[id]` | `process:{processId}:updates`, `org:{orgId}:sync` |
| `/status` | `org:{orgId}:status`, `org:{orgId}:errors`, `org:{orgId}:sync` |

**Invalidation payload types:**

```typescript
type PortalUpdate =
  | {
      event: 'contracts.changed'
      processIds: string[]
      changed: Array<'assessment' | 'decision' | 'manifest' | 'error'>
      maxSeq: number
    }
  | {
      event: 'process.updated'
      processId: string
      changed: Array<'quotes' | 'landed_cost' | 'manifest' | 'jobs' | 'errors'>
      major: boolean
      maxSeq: number
    }
  | {
      event: 'sync.updated'
      syncHealth: 'healthy' | 'degraded' | 'offline'
      lastAckedSeq: number
      lastSuccessAt: string
    }
```

**Emission point:** Daemon emits Broadcast notifications **after** the remote projection batch is successfully applied. Coalesces affected topics so 20 quote upserts produce one notification per topic.

**Per-page behavior:**
- **List page:** Patch badges/counts if in payload. Otherwise invalidate list query. Show "N new/updated opportunities" pill -- do NOT reorder the visible list while user is reading.
- **Detail page (minor):** Auto-refresh the affected section.
- **Detail page (major -- manifest, landed cost, canonical process):** Show non-destructive refresh chip. Do NOT silently swap content under the user.

### 3.3 Degraded-Mode UX -- Three Distinct States

The portal is honest about staleness. Three states, not one badge:

| State | Indicator | UX |
|-------|-----------|-----|
| **Healthy** | Green badge, "Data current" | Full real-time updates via Broadcast. All data fresh. |
| **Degraded** | Amber badge, "Updates delayed -- data may be behind desktop" | Broadcast connected but daemon sync is behind. Data is stale by 90s-5min. All features work but data is not current. |
| **Offline** | Red badge, "Sync offline -- showing last known data" | No sync for >5min or daemon offline. Browse cached projection data only. Broadcast disconnected. |

**Thresholds (from `viewer_sync_status` projection table):**
- Green: < 90 seconds since last successful sync
- Amber: 90s -- 5 min
- Red: > 5 min or daemon offline

The `viewer_sync_status` table (push-owned projection) is updated after each successful remote apply:

```typescript
interface ViewerSyncStatus {
  orgId: string
  syncHealth: 'healthy' | 'degraded' | 'offline'
  lastPushSuccessAt: string       // ISO-8601
  lastPushAttemptAt: string       // ISO-8601
  lastAckedLocalSeq: number       // Latest local_seq successfully pushed
  pendingOutboxCount: number      // Entries waiting to be pushed
  schemaEpoch: number             // Current schema epoch
  daemonVersion: string           // Daemon version string
}
```

**Desktop also shows three states** (upgraded from L2's basic detection):
- **Healthy:** Daemon connected, sync healthy -- full desktop
- **Remote degraded:** Daemon connected, sync behind/offline -- full local ops, sync warning banner
- **Browse-only:** Daemon unavailable -- read-only local browsing, clear banner

### 3.4 Validation Enforcement Mode

Transition from Layer 3 dry-run to production enforcement. In enforcement mode, validator rejections **quarantine** results instead of just logging.

**What changes from dry-run (L3) to enforcement (L4):**

| Aspect | Dry-Run (L3) | Enforcement (L4) |
|--------|-------------|-----------------|
| Rejected results | Advance normally, verdict logged | **Quarantined** as `AGENT_SUSPECT_OUTPUT` |
| Error manifests | Created for logging | Created with operator alert |
| Stage head advancement | Normal (verdict doesn't block) | **Blocked** for rejected results |
| Outbox entries | Normal | **Not created** for quarantined results |
| Pipeline state | Unchanged by verdicts | Quarantined results excluded from rollups |

**Provisional promotion buffer:** For enforcement-active slices, worker outputs are held in local-only staging until validation completes (see §1.3). This resolves the collision between the outbox invariant ("every push-owned mutation creates an outbox entry") and enforcement ("rejected results must not be outboxed"). Without the promotion buffer, rejected rows would already be committed and outboxed before validation completes.

**Per-slice enforcement readiness:**

The unit of enforcement is **worker skill x source x worker behavior version x validator behavior version** -- not the whole system. Enforcement is enabled independently per slice. Only bump behavior versions when output semantics or decision boundaries materially change (see L3 §3.2 for behavior version derivation).

**Prerequisites for enabling enforcement on a slice:**
1. Validator overlay `validator_overlay_compatible_versions` matches current worker behavior version
2. Minimum adjudicated cases threshold met (>=30 rejected/flagged with known ground truth)
3. Minimum audited approvals threshold met (>=60 checked for silent misses)
4. Wilson-bound reject precision lower bound >= 70%
5. Rule-of-three false-pass upper bound <= 5%
6. Coverage of dominant failure families in the challenge bank
7. No active demotion trigger

**Automatic demotion** from enforcement back to dry-run:
- Operator override rate spikes (operators regularly reversing rejections)
- Challenge-bank performance regresses (validator stops catching known-bad)
- Anomaly-conditioned validator error rate rises
- Worker behavior version moves outside overlay's compatible range

Demotion is automatic and immediate. Re-promotion requires meeting calibration thresholds again.

### 3.5 Domain-Specific Validator Overlays (4 skills)

Layer 3 deploys the base validator (generic semantic quality). Layer 4 adds domain-specific overlays that check business rules unique to each skill family.

| Overlay | Directory | Domain Checks |
|---------|-----------|--------------|
| **Procurement ingestion** | `skills/validation/overlay-procurement-ingestion/` | Notice completeness, OCDS field presence, correct source locale, notice-type consistency, expiry date plausibility, CPV code validity |
| **Supplier enrichment** | `skills/validation/overlay-supplier-enrichment/` | Price plausibility (too high/low for category), currency correctness, pack basis consistency, seller diversity, listing freshness, compliance evidence completeness for regulated goods |
| **Document acquisition** | `skills/validation/overlay-document-acquisition/` | Document type identification correctness, download completeness, content-type verification, file size plausibility, document set hash consistency |
| **Product extraction** | `skills/validation/overlay-product-extraction/` | Candidate type plausibility for CPV family, archetype assignment consistency, slot completeness for archetype, quantity/unit plausibility, bundle decomposition correctness |

Each overlay follows the skill directory standard: `SKILL.md` + `result.schema.json` + `checkpoint.schema.json` + `canary.inputs.jsonl` + `golden.assertions.yaml` + `validator-rubric.yaml`.

Overlays are loaded alongside the base validator. The base checks generic quality; the overlay adds domain-specific rules. A skill can declare which overlay to use in its metadata; the Validation Executor loads both.

### 3.6 Formal Calibration Harness

Before enforcement, each slice must pass calibration. The harness uses **three corpora**:

**1. Clean real runs (25-30 per skill family):**
Real successful runs from gold lots and live dry-run traffic. Establish baseline: what does healthy variability look like?

**2. Weird-but-valid runs (15-20 per skill family):**
Unusual but acceptable runs: source layout changed but extraction correct, localized content correctly normalized, partial but honestly narrated data, low-result-count but legitimate.

**3. Mutated bad runs (30-50 per skill family):**
Real good runs mutated into realistic failures:

| Mutation Family | What It Tests |
|----------------|--------------|
| Wrong source/domain in URL chain | Validator catches navigation to wrong site |
| Wrong page kind (search vs category vs detail) | Validator catches page-type confusion |
| Locale/currency swap (claims GBP, evidence EUR) | Validator catches cross-checking failure |
| Pack/unit distortion (per-unit vs per-box vs per-pallet) | Validator catches unit normalization errors |
| Accessory-vs-primary confusion | Validator catches product identity errors |
| Selective warning omission (says 2, daemon shows 5) | Validator catches narrative honesty |
| Manifest/result count mismatch (claims 12, contract has 8) | Validator catches count reconciliation |
| Fabricated evidence references | Validator catches fake anchors |
| Cross-segment duplicates/gaps (continuation chains) | Validator catches chain-level coherence |

**Pairwise challenge sets:** In addition to absolute verdicts, build pairs: run A = clean, run B = one corruption. Measure whether the validator ranks B less trustworthy.

**Minimum calibration targets per slice before enforcement:**
- >=30 adjudicated rejected/flagged cases with known ground truth
- >=60 audited approved cases checked for silent misses
- Wilson-bound reject precision lower bound >= 70% (~26 correct out of 30)
- Rule-of-three false-pass upper bound <= 5% (0 misses in 60 audited)
- Coverage of dominant failure families in challenge bank

**Cross-model audit sampling:** Periodically run a sample through a different model family to detect correlated blindness.

**Override feedback loop:** Every operator override becomes: (1) labeled calibration case, (2) incident-cluster member, (3) candidate mutation template for challenge bank. Note: override rate is a biased metric — overrides are selection-biased toward visible/contested cases. Use overrides as demotion signal, incident source, and mutation template source, but not as sole precision estimator.

**Tombstone retention:** Cursor-based pull requires tombstones to remain visible longer than the maximum expected offline window. If tombstones are GC'd too early, an offline daemon permanently misses deletions. Tombstone retention must be ≥ max offline duration + safety margin (default: 14 days). Alternatively, periodic anti-entropy / full-hash diff can detect missed deletes.

### 3.7 `viewer_sync_status` Table

Push-owned projection table in Supabase. Updated after each successful remote apply.

```sql
CREATE TABLE viewer_sync_status (
  org_id TEXT PRIMARY KEY,
  sync_health TEXT NOT NULL DEFAULT 'offline'
    CHECK (sync_health IN ('healthy', 'degraded', 'offline')),
  last_push_success_at TIMESTAMPTZ,
  last_push_attempt_at TIMESTAMPTZ,
  last_acked_local_seq BIGINT,         -- Latest local_seq pushed
  pending_outbox_count INTEGER DEFAULT 0,
  schema_epoch INTEGER NOT NULL,
  daemon_version TEXT,
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
```

---

## 4. Testing & Acceptance

### 4.1 Definition of Done

| # | Criterion | Verification |
|---|----------|-------------|
| 1 | Push protocol: transactional outbox entries created for all push-owned mutations in same SQLite transaction | `sqlite_supabase_sync.rs` |
| 2 | Push batches applied to Supabase via single transactional RPC with schema_epoch, row_version ordering | `sqlite_supabase_sync.rs` |
| 3 | Pull protocol: cursor-based pull for shared_source_configs, shared_company_profile, operator_decision_heads | `sqlite_supabase_sync.rs` |
| 4 | Operator decisions: queued local intents flush through Supabase CAS RPC with base_remote_revision; conflicts returned (not silently overwritten) | E2E test |
| 5 | Active-writer lease: second daemon instance detected and rejected | `sqlite_supabase_sync.rs` |
| 6 | RLS policies enforced for all 4 roles (viewer, operator, admin, daemon) | Supabase migration test |
| 7 | Web viewing portal displays opportunities, manifests, pipeline status (read-only) | `contracts.spec.ts` |
| 8 | Supabase Broadcast invalidation delivers live updates to web portal | Manual + E2E |
| 9 | viewer_sync_status shows sync freshness (green/amber/red) | Manual verification |
| 10 | Validation enforcement mode: validator rejections quarantine results as AGENT_SUSPECT_OUTPUT | `sqlite_supabase_sync.rs` |
| 11 | 4 domain-specific validator overlays deployed and producing verdicts | Validator canary tests |
| 12 | Two users can view the same pipeline: desktop (read-write) + web (read-only) | Manual: two-browser test |
| 13 | Writer fencing: stale daemon with old `writer_epoch` rejected on push | `sqlite_supabase_sync.rs` |
| 14 | CAS command ledger: retried command returns stored result (idempotent replay) | `sqlite_supabase_sync.rs` |
| 15 | Provisional promotion buffer: enforcement-active slices hold results until validation completes | `sqlite_supabase_sync.rs` |
| 16 | Equal-version retry: identical-envelope retry with matching content_hash = no-op success | `sqlite_supabase_sync.rs` |
| 17 | Web portal uses viewer-scoped credentials only (security boundary, not UI convention) | Supabase migration test + RLS test |
| 18 | Redaction tests pass: no leakage of operator identity/company profile/raw supplier data in summary projections | `redaction_boundary.rs` |

### 4.2 Test File Listing (4 New, Cumulative 42)

| File | Coverage |
|------|---------|
| `sqlite_schema.rs` | Full schema creation and migration: all tables created, all constraints enforced, all indices present, sync tracking columns populated, integer pence/bps verified |
| `sqlite_supabase_sync.rs` | Push-pull round-trip: outbox entry creation, batch assembly, transactional RPC apply, row_version ordering, writer fencing (stale epoch rejected), commit_group_id atomicity, equal-version retry no-op, pull cursor advancement with hi-water fence, CAS conflict detection with command ledger idempotency, active-writer lease, schema_epoch guard, provisional promotion buffer for enforcement slices |
| `redaction_boundary.rs` | Summary projection redaction tests: operator_decision_summaries, workflow_job_summaries, error_manifest_summaries, validation_verdict_summaries — verify no leakage of operator identity, company profile, raw supplier data, tender document metadata, or full error context |
| `apps/web/e2e/contracts.spec.ts` | Web portal E2E: navigate to contracts list, verify read-only (no decision controls), verify keyset pagination, verify sync freshness indicator, verify Broadcast live updates, verify viewer credentials cannot access remote-owned tables |

### 4.3 Test Scenarios

| Scenario | Expected Behavior |
|----------|------------------|
| Push batch with stale row_version | Remote rejects batch (idempotent); daemon retries with current version |
| Push batch with incompatible schema_epoch | Sync pauses; local operation continues; operator alerted |
| Two desktop operators make concurrent decisions | CAS detects conflict on base_remote_revision; second operator retries with refreshed state |
| Validator rejects output in enforcement mode | Results quarantined as AGENT_SUSPECT_OUTPUT; error manifest created; operator can override |
| Validator itself crashes during validation | Fail-open: worker results advance with VALIDATION_SKIPPED_VALIDATOR_ERROR; operator can review manually |
| Sync degraded (Supabase unreachable) | Desktop stays in remote-degraded mode with full local operation and sync warning banner; web shows staleness indicator (amber/red); local operations continue uninterrupted |
| Full push replay after sync recovery | Idempotent: no duplicates; row_version ordering prevents stale overwrites |
| Batch boundary crosses stage_run_id | Push dispatcher rejects: batch boundaries never cross stage_run_id (enforced) |
| Pull-before-push ordering violation | Sync startup enforces pull first, then classify/coalesce pending intents, then resume push |
| Network interruption during push batch | Identical-envelope retry; batch either fully applied or fully rejected (transactional RPC) |
| Stale daemon recovers after lease loss | Push rejected due to stale `writer_epoch`; daemon must re-acquire lease |
| CAS command retried after later command landed | Command ledger returns stored conflict result (not re-evaluated against current state) |
| Equal-version retry with matching content_hash | No-op success; no duplicate rows created |
| Equal-version retry with different content_hash | Hard conflict; data corruption detected |
| Enforcement-active slice: worker completes, validator rejects | Provisional results NOT promoted to push-owned tables; no outbox entries created; error manifest created |
| Paginated pull during concurrent remote changes | Hi-water fence prevents skip/duplicate; pull bounded to snapshot range |
| Source remotely disabled while daemon offline for 5 hours | Source degrade after safety-critical TTL (4h); daemon does not scrape disabled source |

### 4.4 Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Sync engine bugs corrupt remote data | Medium | Medium | Local copy is authoritative; remote can be fully republished from local outbox replay |
| Validator enforcement blocks good data | Medium | Medium | Dry-run calibration in L3; operator override available; false positive rate monitored via operator_override_rate_bps |
| Push batch size exceeds Supabase RPC limits | Low | Low | Batch size bounded by stage_run_id scope; single-in-flight batch constraint |
| Network interruptions during push | Medium | Low | Identical-envelope retries; degraded mode continues local operation; no data loss |
| Epoch mismatch causes permanent remote divergence | Medium | High | Epoch-repair reproject path required (see §5.2 stubs); cannot remain "future nice-to-have" |
| Single-device durability loss (machine dies before push) | Low | Critical | Encrypted WAL-aware snapshots; operational rule: one bound machine per org |
| Outbox growth during long offline windows | Medium | Medium | Eventually coalesce superseded unsent updates by entity key; monitor outbox depth |
| Stale daemon races after partition recovery | Low | High | Writer fencing token (`writer_epoch`) prevents stale pushes |

---

## 5. Open Questions, Forward Compatibility & Dependencies

### 5.1 Open Questions

**§10 Further Research #7: Validator calibration corpus bootstrapping**
- Formal calibration harness requires three corpora per skill family: clean real (25-30), weird-but-valid (15-20), mutated bad (30-50).
- L3 dry-run accumulated initial corpus. L4 enforcement decision depends on calibration quality.
- **L4 impact:** Must have sufficient calibration before enabling enforcement; per-slice readiness prevents premature enforcement.

**Epoch mismatch recovery and reproject**
- Current design: epoch mismatch → sync pauses, stale-epoch outbox entries never sent, local operation continues.
- This means remote permanently misses everything created during the mismatch window unless a full or table-level reproject occurs after epoch repair.
- "Full rebuild not implemented" cannot remain a future nice-to-have — it is required by current mismatch behavior.
- **L4 impact:** Minimum fix: after epoch repair, mark a `projection_generation`, republish current row images from local authoritative tables, discard stale-epoch outbox entries only after successful reseed.

**§10 Assumption #5: Company profile data can be centrally stored with appropriate protections**
- shared_company_profile in Supabase stores org-wide bidder data.
- Impact if wrong: bid workflows need stronger local-only secret partitioning.
- **L4 impact:** RLS policies must protect company profile; confirm data residency comfort level.

### 5.2 Stubs

| Feature | L4 Stub | Full Implementation |
|---------|---------|-------------------|
| Schema epoch transitions | Initial epoch only (no transitions needed) | Future: expand/drain/upgrade/contract protocol |
| Multi-org | Single org hardcoded | Future: org_id scoping on all tables |
| Epoch-repair reproject | Minimum path: after epoch repair, mark `projection_generation`, republish current row images from local tables, discard stale-epoch outbox entries after successful reseed | Future: full generation-based rebuild with anti-entropy |
| Logical consistency auditor | Not implemented | Future: domain invariant checking with conditional auto-repair |

### 5.3 Forward-Compatibility

| Provision | What Exists in L4 | Target Layer |
|-----------|-------------------|-------------|
| schema_epoch on every batch | Initial epoch; sync pauses on mismatch | Future schema evolution protocol |
| RLS roles designed for multi-org | viewer/operator/admin/daemon roles | Future: org_id column for multi-tenancy |
| Projection tables | Denormalized push-owned mirrors | Future: materialized views for complex queries |

### 5.4 Dependencies

**Layer deps:** L0-L3 (all pipeline data, all push-owned tables with sync tracking columns)

#### External Accounts (L4)

| Account | Purpose |
|---------|---------|
| Supabase project | Shared sync target: Postgres database, Auth, Realtime (Broadcast), Storage |

#### Supabase Migrations

| Migration | Purpose |
|-----------|---------|
| `0001_projection_tables.sql` | All 18 push-owned projection tables + 3 junction tables |
| `0002_remote_owned_tables.sql` | `shared_company_profile`, `shared_source_configs`, `operator_decision_heads` |
| `0003_summary_tables.sql` | `workflow_job_summaries`, `error_manifest_summaries`, `validation_verdict_summaries`, `viewer_sync_status`, `executors` |
| `0004_rls_policies.sql` | RLS policies per role (viewer, operator, admin, daemon) |
| `0005_sync_rpcs.sql` | Transactional sync apply RPC (with `writer_epoch` validation) + operator decision CAS RPC (with command ledger) |
| `0006_schema_metadata.sql` | Schema metadata table + initial schema_epoch + active-writer lease row with `writer_epoch` |
| `0007_command_ledger.sql` | `operator_decision_commands` table for durable CAS idempotency |
| `0008_decision_summaries.sql` | `operator_decision_summaries` push-owned projection for web viewer access |

#### New Daemon Module

| Module | Purpose |
|--------|---------|
| `crates/daemon/src/db/sync.rs` | SQLite-to-Supabase push-pull sync engine |

#### New Skills (4 Domain Validator Overlays)

| Skill | Purpose |
|-------|---------|
| `skills/validation/procurement-ingestion/` | OCID structure, date ranges, source verification |
| `skills/validation/supplier-enrichment/` | Pricing plausibility, currency, locale checks |
| `skills/validation/document-acquisition/` | Artifact type, content, completeness |
| `skills/validation/product-extraction/` | Candidate plausibility, slot consistency |

#### Web App

| Path | Files | Purpose |
|------|-------|---------|
| `apps/web` | 7 files | Next.js viewing portal: contracts list, contract detail, status page, realtime subscriptions, staleness store, topic map |

### 5.5 ADRs Addressed

| ADR | Decision | L4 Action |
|-----|----------|----------|
| **ADR-002** | Authority-matrix database | Sync/remote-projection half fully operational |
| **ADR-008** | Desktop = control, web = view | Web viewing portal created (read-only, viewer-scoped security boundary) |

<!-- CROSS-DOMAIN NOTE FOR L0: `commit_group_id` (text, nullable) and `writer_epoch` (integer, nullable) are part of the L4 operational activation of `local_outbox`. The constitutional table shape is reserved in L0, so these fields must be present when the table is first materialized for sync activation. `operator_decision_commands` remains a Supabase-only CAS-infrastructure table and therefore has no SQLite impact. -->

<!-- CROSS-DOMAIN NOTE FOR L2: Desktop degraded-mode UX in L4 adds explicit sync state on top of L2's control-plane model (healthy/remote-degraded/browse-only). L2's `ControlPlaneState` health dimensions should accommodate the sync health state from `viewer_sync_status` once sync is active. The FreshnessBadge component (L2 §2.1) should support the three-state sync indicator. -->

<!-- CROSS-DOMAIN NOTE FOR L2: Desktop operator decisions transition from local-only `pending_operator_intents` (L2) to remote CAS-backed `operator_decision_heads` (L4). Desktop's decision capture flow remains the same (write to `pending_operator_intents`), but L4 adds daemon-side CAS flush + conflict resolution UI. L2's decision capture should already carry `reviewedBasisHash`, `reviewedManifestHash`, `reviewedAtStageRunId` — these become the CAS payload in L4. -->

<!-- CROSS-DOMAIN NOTE FOR L6: Remote config staleness classes (L4 §1.8) include safety-critical TTL for source enablement. If bid workflow depends on source configs (e.g., checking source health before bid preparation), the safety-critical TTL model applies. Additionally, the upstream trust gate for bid workflow should be evaluated using the L3 mapping: no selected-scope `suspect` inputs and process-level `pipeline_assessments.validation_state = validated`, unless the operator explicitly accepts flagged/unverified trust debt. -->
