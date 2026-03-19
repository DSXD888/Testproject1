# Layer 4 — Shared Visibility

> **Authority:** This document is the execution-grade specification for Layer 4. An implementation agent should be able to build this layer from this document alone without reopening the research document.
>
> **Milestone:** M4
> **Dependencies:** Layers 0-3 (all pipeline data, all push-owned tables with sync tracking columns)
> **Exit criteria:** Two users view the same pipeline: desktop operator (read-write) + web viewer (read-only). Sync engine operational. Validation enforcement active.

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

Redacted/summarized views of local-only tables, pushed to Supabase for web portal visibility.

| Table | Source | Purpose | Key Fields |
|-------|--------|---------|-----------|
| `workflow_job_summaries` | `workflow_jobs` (local-only) | Redacted job status for web | `job_id`, `job_kind`, `status`, `skill_path`, `started_at`, `completed_at`, `error_type` (no full context) |
| `error_manifest_summaries` | `error_manifests` (local-only) | Redacted error context for web | `manifest_id`, `job_id`, `error_type`, `error_code`, `incident_signature`, `created_at` (no full context_json or markdown) |
| `validation_verdict_summaries` | `validation_verdicts` (local-only) | Verdict status for web | `verdict_id`, `worker_job_id`, `verdict`, `basis_sufficiency`, `enforcement_eligible`, `created_at` (no full reasoning) |
| `viewer_sync_status` | Daemon sync state | Sync freshness indicator | `sync_health` (healthy/degraded/offline), `last_push_success_at`, `pending_outbox_count`, `schema_epoch` |
| `executors` | `executors` (push-owned) | Daemon heartbeat summary | `executor_id`, `heartbeat_at`, `daemon_version`, `started_at` |

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
  decision_stale boolean NOT NULL DEFAULT false,    -- set true when upstream changes basis
  operator_id text,                                 -- who made the decision
  command_id text,                                  -- idempotency key for CAS (handles lost-response retries)
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
| Remote-owned config | `shared_company_profile`, `shared_source_configs` | SELECT | SELECT | Full CRUD | SELECT (pull) |
| Remote-owned decisions | `operator_decision_heads` | SELECT | SELECT, UPDATE (via CAS RPC) | Full CRUD | SELECT (pull) |
| Sync infrastructure | `schema_metadata` | — | — | SELECT, UPDATE | SELECT, UPDATE |

**Implementation rules:**
- Push-owned tables: daemon writes through `apply_push_batch(batch_json)` RPC that validates `schema_epoch`, checks `row_version` ordering, applies atomically. RLS grants INSERT/UPDATE only to `daemon` role.
- Operator decisions: `update_operator_decision()` RPC enforces CAS. Direct UPDATE blocked.
- All SELECT policies include `WHERE deleted_at IS NULL` for tombstoned tables (excludes soft-deleted rows from portal views).
- **RLS tombstone visibility fix:** The daemon pull query must see tombstoned rows (to propagate deletions locally). Daemon role SELECT policy does NOT filter on `deleted_at IS NULL` for remote-owned tables.
- `viewer` and `operator` roles cannot see sync infrastructure tables.

### 2.6 Supabase Migrations

Six migration files create the full Supabase schema:

| # | Migration | Purpose |
|---|-----------|---------|
| 1 | `supabase/migrations/0001_projection_tables.sql` | CREATE TABLE for all 18 push-owned projection tables + 3 junction tables. Schema mirrors SQLite DDL with Postgres types. |
| 2 | `supabase/migrations/0002_remote_owned_tables.sql` | CREATE TABLE for `shared_company_profile`, `shared_source_configs`, `operator_decision_heads`. |
| 3 | `supabase/migrations/0003_summary_tables.sql` | CREATE TABLE for 5 summary tables: `workflow_job_summaries`, `error_manifest_summaries`, `validation_verdict_summaries`, `viewer_sync_status`, `executors`. |
| 4 | `supabase/migrations/0004_rls_policies.sql` | RLS policies for all 4 roles (viewer, operator, admin, daemon) across all table categories. |
| 5 | `supabase/migrations/0005_sync_rpcs.sql` | `apply_push_batch(batch_json)` transactional RPC + `update_operator_decision(process_id, disposition, base_revision, command_id)` CAS RPC. |
| 6 | `supabase/migrations/0006_schema_metadata.sql` | `schema_metadata` table with `schema_epoch` for compatibility checking. |

**Schema coordination rule:** Both SQLite and Postgres derive from the same canonical logical schema. CI must apply both migrations from scratch, introspect both, and fail if logical schema hash differs.

**SQLite sync migration:** `supabase/migrations/0007_sqlite_sync.sql` — adds `local_outbox`, `sync_batches`, `pull_cursors` tables to SQLite (see §1 for DDL).

### 2.7 Cross-Authority Consistency

Pipeline data crosses authority boundaries at specific points. Each crossing has a classification:

| Crossing Point | Authority Domains | Classification | Provenance Tracking |
|---|---|---|---|
| Pipeline output → operator review | Push-owned → display | **Display** — no authority conflict | `decisionBasisHash` links decision to pipeline snapshot |
| Operator decision → bid workflow | Remote-owned → push-owned automation | **Automation** — must track decision provenance | `reviewedAtStageRunId`, `reviewedBasisHash` carried through |
| Coverage metrics → qualification gate | Push-owned → push-owned control | **Control** — no authority crossing | Internal to pipeline |
| Sync health → UX mode | Push-owned infra → display | **Display** — no authority conflict | `viewer_sync_status` drives 3-state UX |

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

**Web-specific data-fetching:** The web uses Supabase client for all data access. It reuses `packages/ui` presentational components (ContractCardFrame, ManifestViewer, PipelineBadge, SyncAgeBadge, ErrorSummaryList) but has its own data-fetching hooks. No SQLite, no daemon bridge, no `invoke()`.

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

**Per-slice enforcement readiness:**

The unit of enforcement is **worker skill x source x worker version x validator overlay version** -- not the whole system. Enforcement is enabled independently per slice.

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

**Override feedback loop:** Every operator override becomes: (1) labeled calibration case, (2) incident-cluster member, (3) candidate mutation template for challenge bank.

### 3.7 `viewer_sync_status` Table

Push-owned projection table in Supabase. Updated after each successful remote apply.

```sql
CREATE TABLE viewer_sync_status (
  org_id TEXT PRIMARY KEY,
  sync_health TEXT NOT NULL DEFAULT 'offline'
    CHECK (sync_health IN ('healthy', 'degraded', 'offline')),
  last_push_success_at TEXT,           -- ISO-8601
  last_push_attempt_at TEXT,           -- ISO-8601
  last_acked_local_seq INTEGER,        -- Latest local_seq pushed
  pending_outbox_count INTEGER DEFAULT 0,
  schema_epoch INTEGER NOT NULL,
  daemon_version TEXT,
  updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);
```
