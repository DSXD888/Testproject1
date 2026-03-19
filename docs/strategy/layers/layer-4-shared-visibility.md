# Layer 4 — Shared Visibility

> **Authority:** This document is the execution-grade specification for Layer 4. An implementation agent should be able to build this layer from this document alone without reopening the research document.
>
> **Milestone:** M4
> **Dependencies:** Layers 0-3 (all pipeline data, all push-owned tables with sync tracking columns)
> **Exit criteria:** Two users view the same pipeline: desktop operator (read-write) + web viewer (read-only). Sync engine operational. Validation enforcement active.

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
