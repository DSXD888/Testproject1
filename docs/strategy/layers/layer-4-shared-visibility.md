# Layer 4 — Shared Visibility

> **Authority:** This document is the execution-grade specification for Layer 4. An implementation agent should be able to build this layer from this document alone without reopening the research document.
>
> **Milestone:** M4
> **Dependencies:** Layers 0-3 (all pipeline data, all push-owned tables with sync tracking columns)
> **Exit criteria:** Two users view the same pipeline: desktop operator (read-write) + web viewer (read-only). Sync engine operational. Validation enforcement active.

> **Implementation Status (as of 2026-05-02):** Layer 4 is **PLANNED — not yet built**. No production sync engine, read-only web portal, shared Supabase authority, or validation-enforcement code exists for this layer. This doc remains a planned execution-grade specification and intentionally has no inline implementation markers.

> **3-SOT Validation (2026-05-06):** Validated for downstream TDD, bead creation, and completion-audit coverage. Requirement IDs use `L4-FR-{section}.{subsection}.{item}` and acceptance criteria are listed under the requirements they verify.
>
> **Updated 2026-06-04 — goods-led labour-cost harmonization.** Forward notes added to §1.7 (Schema Epoch Guard), §3.5 (validator overlays), and the §5.2/§5.3 epoch stubs to reflect that the schema epoch has since advanced to **10** (goods-led labour migration `migrations/0014` + Supabase projection `0010`), and that the L5 `overlay-labour-cost` overlay joins the four L4 domain overlays. L4 itself did not add labour fields; these are forward pointers only.
>
> **Updated 2026-06-12 — autonomous daemon scheduling harmonization.** The schema epoch has since advanced to **11** (autonomous daemon scheduling migration `migrations/0015_procontract_pending_enrichment.sql`, a new **local-only** table with no Supabase projection; see L3 §1.11 for the invariant that this dedicated table leaves `interest_registration.state='completed'`); the §1.7, §5.2, and §5.3 epoch notes are updated accordingly. A clarification was added to §1.6 (Active-Writer Lease) that the lease **gates remote push to Supabase only, not local job execution** — the L3 autonomous worker runs jobs regardless of lease state. These are forward pointers only; L4 itself did not change.

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

**L4-FR-1.3.1 — Transactional outbox is the only push-owned write path after sync activation**
- AC1: Every push-owned SQLite mutation creates exactly one `local_outbox` entry in the same SQLite transaction as the data write.
- AC2: A transaction that mutates push-owned data and fails to create the matching outbox entry is rejected or rolled back.
- AC3: Enforcement-active rejected provisional results create no push-owned mutation and therefore create no outbox entry.
- AC4: `local_outbox` is created by the L4 SQLite migration `migrations/0012_l4_sync_state.sql`; L0 reserves the forward-compatible column shape but does not pre-create the table in `migrations/0001_sqlite_state_initial.sql`.

**Push dispatcher contract:**
- Single-in-flight batch (one batch at a time per push stream)
- Identical-envelope retries on failure (same content, same order)
- Batch boundaries never cross `schema_epoch` or `stage_run_id`
- Batch boundaries never split a `commit_group_id` (preserves local SQLite transaction atomicity)
- `row_version` ordering within batch (monotonic per row)
- `content_hash` for idempotent deduplication
- Every batch carries `writer_epoch` (see §1.6 Active-Writer Lease)

**L4-FR-1.3.2 — Push dispatcher sends one in-flight batch per push stream**
- AC1: The dispatcher never sends a second batch for the same stream until the prior batch is acknowledged, rejected, or moved into retry state.
- AC2: `sync_batches` records the in-flight range so daemon restart cannot create two active batches for the same local sequence range.

**L4-FR-1.3.3 — Failed pushes retry the identical envelope**
- AC1: Retry payloads preserve the same row set, ordering, `content_hash` values, `schema_epoch`, `stage_run_id`, `commit_group_id` boundaries, and `writer_epoch`.
- AC2: Network timeout or lost response does not rebuild the batch from newly-arrived outbox rows.
- AC3: If the daemon crashes after creating a `sync_batches` row but before receiving the RPC response, restart reloads the persisted batch envelope and retries it before constructing any later batch for that stream.
- AC4: If the remote already committed the crashed batch, equal-version/content-hash semantics return no-op success and the daemon marks the existing `sync_batches` row acknowledged.
- AC5: A terminal stale-writer rejection is not an indeterminate retry case: the daemon marks that `sync_batches` envelope rejected, re-acquires the lease, then may create a new batch envelope over the same unacknowledged outbox rows using the new `writer_epoch`.

**L4-FR-1.3.4 — Batch boundaries preserve epoch, run, and transaction atomicity**
- AC1: No batch contains rows from more than one `schema_epoch`.
- AC2: No batch contains rows from more than one `stage_run_id`.
- AC3: No batch splits rows that share a `commit_group_id`; all rows from the same SQLite write transaction are sent together or not sent.

**L4-FR-1.3.5 — Batch row ordering and content hashing make apply idempotent**
- AC1: Rows inside a batch are ordered by per-row `row_version` and deterministic entity key ordering for ties.
- AC2: Each row carries `content_hash` computed over the canonical payload that Supabase applies.
- AC3: Supabase uses `content_hash` with `row_version` to distinguish idempotent retry from equal-version corruption.

**Push batch RPC:** `apply_push_batch(batch_json, writer_epoch)` — a single Supabase REST RPC call made by the daemon HTTP client. The RPC is implemented as one Postgres function invoked through Supabase/PostgREST; Postgres executes the function body in one transaction, so the batch either fully applies or fully rejects. L4 does not use `tokio-postgres`, client-managed `BEGIN`/`COMMIT`, or direct daemon table upserts for projection writes. Remote validates `writer_epoch` matches current lease holder.

**L4-FR-1.3.6 — `apply_push_batch` is the only remote apply surface for push-owned data**
- AC1: The RPC applies the full batch in one Postgres transaction.
- AC2: Any validation failure rejects the full batch and leaves all target rows unchanged.
- AC3: The RPC validates `schema_epoch`, `writer_epoch`, `row_version`, `content_hash`, `stage_run_id`, and `commit_group_id` constraints before committing.
- AC4: Direct table upserts from the daemon are not used for push-owned projection writes.
- AC5: The daemon calls the RPC through the Supabase REST endpoint using the L4 HTTP client dependency from §5.4; implementation agents must not substitute a direct Postgres connection unless the spec is explicitly revised.

**Equal-version retry semantics:** The remote apply logic must define:
- Incoming `row_version` = remote `row_version` AND `content_hash` matches → **no-op success** (idempotent retry)
- Incoming `row_version` = remote `row_version` but `content_hash` differs → **hard conflict** (data corruption)
- Incoming `row_version` = remote `row_version` + 1 → **apply** (normal advance)
- Anything else (older, larger jump) → **reject** (stale or out-of-order)

Without defining the equal-version case, identical-envelope retries fail spuriously.

**L4-FR-1.3.7 — Equal-version retry semantics are explicit and testable**
- AC1: Equal `row_version` plus matching `content_hash` returns no-op success and does not duplicate rows.
- AC2: Equal `row_version` plus different `content_hash` returns a hard-conflict error classified as data corruption.
- AC3: Incoming `row_version = remote row_version + 1` applies as the normal advance case.
- AC4: Older versions and larger jumps reject as stale or out-of-order without mutating the remote row.

**Outbox entry structure:**

```sql
-- Each outbox entry (created by L4 in migrations/0012_l4_sync_state.sql;
-- L0 reserves the constitutional column shape but does not pre-create local_outbox):
-- local_seq (autoincrement), executor_id, entity_kind, entity_id,
-- op (insert/update), row_version, stage_run_id, schema_epoch,
-- commit_group_id (groups rows from same SQLite transaction),
-- writer_epoch (nullable until enveloped), payload_json, content_hash, created_at
```

**L4-FR-1.3.8 — `local_outbox` stores the full replay envelope**
- AC1: The `local_outbox` schema includes `commit_group_id` and nullable `writer_epoch` from first materialization in `migrations/0012_l4_sync_state.sql`.
- AC2: `payload_json` is sufficient to reconstruct the exact remote row image for retry or reproject.
- AC3: `local_seq` is monotonic and stable across daemon restart.
- AC4: `content_hash` is persisted with the outbox row and not recomputed differently during retry.
- AC5: `writer_epoch` on `local_outbox` is nullable until the row is assigned to a `sync_batches` envelope; `sync_batches` stores the immutable batch writer epoch used for identical retry.
- AC6: Re-enveloping unacknowledged outbox rows with a newer `writer_epoch` is allowed only after a terminal stale-writer rejection or explicit lease-rehome repair, never after an indeterminate network failure.

**Provisional promotion buffer for enforcement slices:**

For slices where validation enforcement is active, worker outputs must NOT be written directly to push-owned tables before validation completes. Without this, rejected rows have already been committed and outboxed — making "quarantine" post-hoc theater.

For enforcement-active slices, the write flow becomes:
1. Worker completes → OQG runs
2. Worker outputs stored as **local-only provisional results** in the SQLite `provisional_worker_outputs` table
3. Validator runs against provisional results
4. If approved: promote to push-owned tables + create outbox entries + advance stage heads (normal path)
5. If rejected: keep artifacts and verdicts locally; do NOT promote to authoritative push-owned state; create error manifest

For dry-run slices and non-enforcement slices, the current direct-write path continues unchanged.

**L4-FR-1.3.9 — Enforcement-active slices use a provisional promotion buffer**
- AC1: Worker outputs for enforcement-active slices are stored locally in `provisional_worker_outputs` before validator approval.
- AC2: Validator approval atomically promotes provisional results to push-owned tables, creates outbox entries, and advances stage heads.
- AC3: Validator rejection preserves provisional artifacts and verdicts locally, emits an error manifest, and creates no push-owned rows or outbox entries.
- AC4: Dry-run slices and non-enforcement slices keep the L3 direct-write behavior unchanged.
- AC5: `migrations/0012_l4_sync_state.sql` creates `provisional_worker_outputs` with `worker_job_id`, `payload_json`, `result_contract_blob_path`, `created_at`, and `validation_status` constrained to `pending`, `approved`, or `rejected`; `worker_job_id` is indexed and links the provisional row to the existing L3 validation verdict flow.

### 1.4 Pull Protocol (Supabase -> Local)

**Cursor-based pull with hi-water fence:** Per-domain pull cursors track `remote_change_seq`. Three pull domains:
- `shared_source_configs` — source enablement and configuration
- `shared_company_profile` — org-wide bidder profile
- `operator_decision_heads` — operator dispositions with CAS

**L4-FR-1.4.1 — Pull cursors are maintained per remote-owned domain**
- AC1: `pull_cursors` has independent cursor rows for `shared_source_configs`, `shared_company_profile`, and `operator_decision_heads`.
- AC2: Each cursor stores the last applied `remote_change_seq` for its domain.
- AC3: A failed pull for one domain does not advance that domain cursor and does not require cross-domain serializability.

**Hi-water fenced pull algorithm:**
1. Read current `hiwater = max(remote_change_seq)` for the domain (snapshot fence)
2. Fetch rows where `cursor < change_seq <= hiwater` (bounded range)
3. Apply to local SQLite in `change_seq` order (upsert based on primary key)
4. Advance cursor to `hiwater`

Without a hi-water fence, paginated pulls can skip or duplicate rows when remote changes occur during pagination. Per-domain monotonic pull fences are required; cross-domain serializability is not.

`remote_change_seq` is a domain-level monotonic sequence, not a per-row version. Each remote-owned domain (`shared_source_configs`, `shared_company_profile`, `operator_decision_heads`) has a single sequence source used by insert, update, and tombstone writes in that domain. Admin DML and CAS RPC paths must assign `remote_change_seq = nextval(domain_sequence)` through triggers or the RPC body before the change commits. `remote_revision` remains the per-row CAS/content revision.

**L4-FR-1.4.2 — Pulls use a hi-water fence to survive concurrent remote changes**
- AC1: Each pull reads `hiwater` before fetching pages for the domain.
- AC2: Every page query is bounded by `cursor < remote_change_seq <= hiwater`.
- AC3: Local application is ordered by `remote_change_seq`.
- AC4: The cursor advances to `hiwater` only after all rows up to `hiwater` are applied.
- AC5: Remote changes with `remote_change_seq > hiwater` wait for the next pull and cannot be skipped by pagination.
- AC6: Supabase migrations create or trigger a domain-level monotonic `remote_change_seq` source for each remote-owned domain; tests fail if two changes in one domain can reuse or move backward in `remote_change_seq`.

**Pull-before-push ordering:** On sync startup and at the start of every sync cycle, pull remote-owned state FIRST, then classify/coalesce pending intents, then resume CAS flushing and push dispatch. This ensures cross-authority consistency. If a cycle cannot complete remote-owned pulls, the daemon applies the staleness classes in §1.8 before deciding which local work may continue.

**L4-FR-1.4.3 — Sync cycles pull remote-owned state before pushing**
- AC1: On daemon sync startup and every scheduled/manual sync cycle, remote-owned pulls complete before pending operator intents are flushed.
- AC2: CAS flushing and push dispatch resume only after pull completion and local intent classification/coalescing for that cycle.
- AC3: A pull failure leaves push dispatch paused for cross-authority operator decisions while local operation continues in degraded mode.

**Pending operator intent reconciliation:** After pull, reconcile `pending_operator_intents` queue:
1. Compare each queued intent against freshly pulled `operator_decision_heads`
2. If intent's `base_remote_revision` matches current remote revision → flush via CAS RPC
3. If revision mismatch → conflict, update local queue status, surface to operator
4. Coalesce multiple intents for same process (latest-per-process)

**L4-FR-1.4.4 — Pending operator intents reconcile against pulled CAS heads**
- AC1: Reconciliation compares each queued intent's `base_remote_revision` with the pulled `operator_decision_heads.remote_revision`.
- AC2: Matching revisions flush through the CAS RPC.
- AC3: Revision mismatches mark the local queue row conflicted and surface a desktop conflict state.
- AC4: Multiple unsent intents for the same process coalesce latest-per-process before CAS flush.
- AC5: The coalescing winner is the most recent unsuperseded intent by `queued_at DESC, id DESC`, matching `migrations/0004_l2_pending_operator_intents.sql` and `crates/daemon/src/ipc/handlers/decisions.rs`; older unsuperseded intents for the process are marked with `superseded_by_intent_id` pointing to the winner before flush.

### 1.5 Compare-and-Swap (CAS) for Operator Decisions

**CAS RPC:** `update_operator_decision(process_id, disposition, base_revision, command_id)`
- `base_remote_revision` checked — mismatch means someone else changed it
- `command_id` for idempotent retry (lost-response handling) via **durable command ledger**
- Conflict returned (not silently overwritten)
- Desktop shows conflict resolution UI

**L4-FR-1.5.1 — Operator decisions mutate only through CAS**
- AC1: The RPC rejects when `base_revision` differs from current `operator_decision_heads.remote_revision`.
- AC2: A conflict response returns structured conflict data and does not overwrite the current decision.
- AC3: Desktop conflict resolution UI receives enough state to refresh and retry intentionally.
- AC4: Direct table UPDATE is blocked for operator decision mutation paths outside the RPC.
- AC5: The desktop decision IPC flow remains `capture_decision` -> write `pending_operator_intents` -> CAS flusher attempts promotion -> desktop re-fetches fresh state after `MutationAck.visible_after_seq`.
- AC6: On CAS conflict, daemon emits a `decision.conflict` IPC event carrying current `remote_revision`, current `disposition`, and the rejected intent payload.
- AC7: The desktop conflict UI shows the remote disposition/revision, the local attempted disposition/note/reviewed hashes, and actions to refresh, discard the local intent, or retry by issuing a new command against the current remote revision.

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

**L4-FR-1.5.2 — `operator_decision_commands` is the durable idempotency ledger**
- AC1: `command_id` is the primary key and survives later commands on the same process.
- AC2: Applied commands store `applied_revision` and response payload.
- AC3: Conflicted commands store `applied_revision = NULL`, `result = 'conflict'`, and response payload.
- AC4: The ledger is written in the same transaction as the decision-head update or conflict result.

CAS flow with command ledger:
1. Check command ledger for `command_id` — if present, return stored result (idempotent replay)
2. Compare `base_revision` against current `operator_decision_heads` row
3. If match: apply, increment revision, write command ledger row transactionally
4. If mismatch: write conflict entry to command ledger, return conflict

**L4-FR-1.5.3 — CAS command replay returns the original result**
- AC1: A repeated `command_id` returns the stored ledger result without re-evaluating current `operator_decision_heads`.
- AC2: A new command with matching base revision increments `remote_revision` exactly once.
- AC3: A new command with mismatched base revision writes a conflict ledger row and returns conflict.
- AC4: Lost-response retry after a later command landed returns the original stored result for the retried command.

Command ledger entries are GC'd after a configurable retention period (default 30 days).

**L4-FR-1.5.4 — CAS command ledger retention is bounded but durable for retries**
- AC1: Default command ledger retention is 30 days.
- AC2: GC never removes entries newer than the configured retention period.
- AC3: GC removal does not alter current `operator_decision_heads` state.
- AC4: A replayed `command_id` after its ledger row has been GC'd is treated as a new command and evaluated against the current `operator_decision_heads.remote_revision`; tests cover the post-GC replay path and expect conflict if the supplied `base_revision` is stale.

### 1.6 Active-Writer Lease (Fenced)

One daemon per org in MVP. Enforced at push protocol level with **writer fencing token**:
- Daemon registers with `executor_id` on first push
- Second daemon attempting to push → rejected with lease conflict
- Lease heartbeat via `viewer_sync_status` updates

> **Forward note (2026-06-12):** The active-writer lease **gates remote push to Supabase only — it does not gate local job execution.** Local SQLite is operational authority; the lease only governs who may push shared-authority rows to Supabase. The L3 autonomous daemon worker (see Layer 3) executes scheduled and triggered jobs **regardless of lease state** — job runs continue to write local operational state normally. On lease loss, only push dispatch is suspended (the outbox accumulates and drains once the lease is reacquired); in-flight and subsequent local work is unaffected.

**L4-FR-1.6.1 — Active-writer lease rejects concurrent pushers**
- AC1: The first pushing daemon registers or renews lease ownership with its `executor_id`.
- AC2: A second daemon with a different `executor_id` is rejected from push apply while the lease is valid.
- AC3: Lease heartbeat and freshness are visible through `viewer_sync_status`.

**Fencing token:** Without a fencing token, a stale daemon (partitioned, then recovered) can race a newly-leased daemon. The remote lease row carries:

```sql
-- In dedicated active_writer_lease table:
-- org_id, holder_executor_id, lease_expires_at, writer_epoch (integer, incremented on acquisition)
```

On lease acquisition, increment `writer_epoch`. Every `apply_push_batch` must include the current `writer_epoch`. Remote rejects batches with stale epoch. This is the correct MVP single-writer fence.

**L4-FR-1.6.2 — Writer fencing token prevents stale-daemon races**
- AC1: Lease acquisition increments `writer_epoch`.
- AC2: Every `apply_push_batch` includes the daemon's current `writer_epoch`.
- AC3: Supabase rejects any batch whose `writer_epoch` does not match the active lease row.
- AC4: A partitioned daemon that recovers after lease loss cannot push stale batches.
- AC5: `infra/supabase/migrations/0006_schema_metadata.sql` creates a dedicated `active_writer_lease` table for `org_id`, `holder_executor_id`, `lease_expires_at`, and `writer_epoch`; `schema_metadata` stores schema epoch metadata only.
- AC6: Lease acquisition is compare-and-swap based: a daemon can acquire only when `holder_executor_id` is itself, NULL, or `lease_expires_at < now()`, and acquisition increments `writer_epoch` in the same transaction.
- AC7: Lease heartbeat runs at least every 30 seconds while sync is active and extends `lease_expires_at` to at least 120 seconds from the heartbeat time.
- AC8: If a daemon crashes without releasing the lease, another writer can acquire only after `lease_expires_at` passes; stale batches from the crashed daemon remain rejected by `writer_epoch`.

**Operational rule (MVP):** One bound machine per org. Other installations are viewer-only unless explicitly rehomed. Active-writer lease protects remote pushing but does not prevent a second offline daemon from creating divergent local state. Document this as an operational constraint.

**L4-FR-1.6.3 — MVP operational binding is explicit**
- AC1: Operator/admin documentation states that only one bound machine per org may run writer-mode daemon.
- AC2: Other installations are configured viewer-only unless an explicit rehome procedure changes the bound writer.
- AC3: The lease conflict path explains that remote push is protected but divergent offline local state is outside the MVP operating model.
- AC4: A lease-conflict rejection is surfaced in desktop sync status as a writer-mode conflict with the current holder executor ID, lease expiry, and next safe action ("switch this install to viewer-only" or "perform explicit rehome").

### 1.7 Schema Epoch Guard

`schema_epoch` (integer) on every push batch and in `PRAGMA user_version`:
- If batch epoch != remote epoch → sync pauses
- Local operation continues uninterrupted
- Operator/admin must resolve epoch mismatch before sync resumes
- Prevents schema-mismatched data from corrupting the remote projection

Current daemon grounding: `crates/daemon/src/db/sqlite.rs::open_state_db(path, schema_epoch)` threads the expected epoch into `StateDbConfig`, applies eligible migrations, and persists the resulting value in `PRAGMA user_version`. L4 extends that existing path rather than introducing a second local epoch source. The L4 SQLite migration `migrations/0012_l4_sync_state.sql` is registered in the `MIGRATIONS` list at the next schema epoch after the current repo epoch (`DEFAULT_SCHEMA_EPOCH = 8` before L4), and push code reads the active epoch from `StateDb::schema_epoch()`/`PRAGMA user_version` when constructing outbox and batch envelopes.

> **Forward note (2026-06-12):** The epoch has advanced since L4. The **current schema epoch is 11**, last bumped by the autonomous daemon scheduling migration `migrations/0015_procontract_pending_enrichment.sql`, which adds a new **local-only** `procontract_pending_enrichment` table. That table is dedicated awaiting-access state, not an `interest_registration.state` flip; `interest_registration.state` stays `'completed'` so the register-interest re-fire guard still excludes the tender. The prior bump to 10 was the goods-led installation-labour migration `migrations/0014_installation_labour_fields.sql`. Unlike `0014`, migration `0015` is **SQLite-only**: its new table is local operational state and has **no Supabase projection**, so it does not touch `infra/supabase/migrations/` or change Supabase `schema_metadata`'s projected row shape. The Supabase `schema_metadata` projected epoch is therefore **unchanged — still 10, as last set by `0010_installation_labour_projection.sql`** (the highest Supabase migration); only the local daemon guard and desktop expectation advance to 11. Because the bump is a local-only additive table with no change to any projected table, the existing Supabase projection stays compatible and no `infra/supabase/0011` is required. The "epoch 8 before L4" baseline above describes the pre-L4 state only — L4 itself did not add labour or enrichment fields. The desktop↔daemon epoch coupling is what moves to 11: `crates/daemon/src/db/sqlite.rs` `DEFAULT_SCHEMA_EPOCH`, `apps/desktop/src-tauri/src/main.rs` `EXPECTED_SCHEMA_EPOCH`, `daemon_bridge.rs`, and `query_service.rs` are all reconciled at 11.
>
> The earlier bump to 10 was last described as: schema epoch 10, bumped by the goods-led installation-labour migration `migrations/0014_installation_labour_fields.sql`, with labour/permit columns and the `maintenance_period_estimates` table projected to Supabase by `infra/supabase/migrations/0010_installation_labour_projection.sql` (the 0014 migration also closed the prior desktop/daemon epoch-coupling lag). That projection remains in place; `0015` builds on top of it without adding a projection of its own.

**L4-FR-1.7.1 — Schema epoch mismatch pauses sync without stopping local operation**
- AC1: Every push batch includes `schema_epoch`.
- AC2: Local SQLite `PRAGMA user_version` and remote `schema_metadata.schema_epoch` are compared before apply.
- AC3: Mismatch pauses push/pull sync and emits operator/admin-visible status.
- AC4: Local pipeline execution and desktop local operation continue while sync is paused.
- AC5: Remote projection writes do not resume until epoch repair and reproject minimum path in §5.2 completes.
- AC6: The L4 implementation updates `crates/daemon/src/db/sqlite.rs` migration registration and default epoch so `open_state_db` applies `migrations/0012_l4_sync_state.sql` through the existing PRAGMA-backed path.

### 1.8 Degraded Mode Behavior

| Sync State | Desktop | Web |
|-----------|---------|-----|
| Healthy | Full read-write-trigger | Full read-only with live Broadcast updates |
| Degraded (push failing) | Full local operation; push queues in outbox | Stale data (amber indicator); Broadcast stopped |
| Offline (Supabase unreachable) | Full local operation; decision capture continues into local queues until CAS flush resumes | Red staleness indicator; last-known data |

Desktop continues operating on local SQLite regardless of sync state. Web reflects last-pushed data with staleness indicator.

**L4-FR-1.8.1 — Sync health exposes three distinct operating states**
- AC1: Healthy mode gives desktop full read/write/trigger behavior and web live read-only Broadcast updates.
- AC2: Degraded mode keeps desktop fully local with queued outbox pushes and shows web amber stale state.
- AC3: Offline mode keeps desktop local operation and decision capture in queues and shows web red last-known-data state.
- AC4: `viewer_sync_status.sync_health` records the state used by web and desktop surfaces.

**Remote config staleness classes:**

If pull breaks, the daemon continues with stale remote-owned config. Not all config is equally safe to use stale:

| Config | Class | Stale Behaviour |
|--------|-------|----------------|
| `shared_company_profile` | Informational | Stale OK — used only for bid form filling (L6) |
| `shared_source_configs` (display/schedule) | Operational | Warn after 1 hour; use cached values |
| `shared_source_configs` (enabled/disabled) | Safety-critical | If a source is remotely disabled (legal/compliance), an offline daemon must NOT scrape it indefinitely. Degrade that source after configurable TTL (default 4 hours) |
| `operator_decision_heads` | Operational | CAS protects correctness on push; stale pull means decisions may not reflect latest remote state |

**L4-FR-1.8.2 — Remote-owned config staleness is classified by safety impact**
- AC1: Stale `shared_company_profile` remains usable as informational bid-form input.
- AC2: Stale `shared_source_configs` display/schedule fields warn after 1 hour and use cached values.
- AC3: Stale `shared_source_configs.enabled` degrades the source after the safety-critical TTL, default 4 hours, so a remotely disabled source is not scraped indefinitely.
- AC4: Stale `operator_decision_heads` does not bypass CAS; pending local decisions may conflict when pull resumes.

**Backup and durability:** Local SQLite authority without backup is availability theater. Encrypted WAL-aware snapshots should be taken periodically. Single-device durability is a known MVP limitation — if the machine dies before push, authoritative data is lost.

**L4-FR-1.8.3 — Local authority has explicit backup and durability behavior**
- AC1: The daemon documents and schedules encrypted WAL-aware local SQLite snapshots.
- AC2: Snapshot failures surface as operational warnings without blocking local pipeline work.
- AC3: Single-device pre-push loss remains an explicit MVP limitation in operator/admin documentation.

### 1.9 Invariants Enforced

| ID | Invariant | Type | Enforcement |
|----|-----------|------|-------------|
| L4-FR-1.9.1 | Push batch atomicity | Constitutional | Single transactional RPC, not individual upserts |
| L4-FR-1.9.2 | Outbox + data in same SQLite transaction | Constitutional | Every push-owned mutation creates outbox entry atomically |
| L4-FR-1.9.3 | CAS for operator decisions | Constitutional | `base_remote_revision` checked; mismatch -> conflict |
| L4-FR-1.9.4 | CAS command ledger for idempotency | Constitutional | Durable `operator_decision_commands` table; retry returns stored result |
| L4-FR-1.9.5 | Active-writer lease with fencing token | Constitutional | `writer_epoch` on every batch; stale epoch -> rejected |
| L4-FR-1.9.6 | `schema_epoch` on every batch | Constitutional | Incompatible epoch -> sync pauses |
| L4-FR-1.9.7 | Batch boundaries never cross `stage_run_id` | Constitutional | Push dispatcher enforces |
| L4-FR-1.9.8 | Batch boundaries never split `commit_group_id` | Constitutional | Preserves local SQLite transaction atomicity |
| L4-FR-1.9.9 | Single-in-flight batch | Constitutional | One batch at a time per push stream |
| L4-FR-1.9.10 | Equal-version retry = no-op on content_hash match | Constitutional | Identical-envelope retries must not fail spuriously |
| L4-FR-1.9.11 | Provisional promotion for enforcement slices | Constitutional | Worker outputs not committed to push-owned tables until validator completes |
| L4-FR-1.9.12 | Validation enforcement | Operational | Rejections quarantine results after dry-run calibration |
| L4-FR-1.9.13 | RLS on all Supabase tables | Constitutional | Policy matrix at database boundary |
| L4-FR-1.9.14 | Fenced pull-before-push ordering | Constitutional | Pull remote state FIRST on startup and at the start of every sync cycle; per-domain hi-water fenced pull prevents skip/duplicate during pagination |

**Acceptance criteria for L4-FR-1.9.1 through L4-FR-1.9.14**
- AC1: Each invariant has at least one corresponding test scenario in §4.3 or Definition of Done criterion in §4.1.
- AC2: Any implementation path that violates a constitutional invariant fails validation before Layer 4 can be marked complete.
- AC3: The operational validation-enforcement invariant has explicit calibration and demotion criteria in §§3.4-3.6.
- AC4: This table intentionally preserves 14 invariant rows; hi-water fenced pull remains a required subclause of the pull-before-push invariant and retains its detailed ACs in §1.4.

### 1.10 Components Touched

| Component | Action | Files |
|-----------|--------|-------|
| Database | Create | `migrations/0012_l4_sync_state.sql` (SQLite sync tables), 8 Supabase migrations, `infra/supabase/config.toml` |
| `crates/daemon` | Create | `src/db/sync.rs` (push-pull sync engine) |
| `crates/daemon` | Extend/Create | `src/validation/policy.rs`, `src/validation/executor.rs`, `src/validation/calibration.rs`, `src/jobs/runner.rs`, `src/ipc/handlers/decisions.rs` |
| `packages/ui` | Extend | `packages/ui/freshness-badge.tsx` (3-state sync health input while preserving the VM path) |
| Root Bun workspace | Extend | `package.json` build/test/lint scripts include `apps/web` and preserve `packages/ui` import boundary |
| Skills | Create | 4 domain validator overlays |
| `apps/web` | Create | 7 files (full Next.js viewing portal) |

**L4-FR-1.10.1 — Layer 4 creates the net-new sync and visibility surfaces**
- AC1: SQLite sync infrastructure is created in `migrations/0012_l4_sync_state.sql`.
- AC2: The daemon adds one new sync module at `crates/daemon/src/db/sync.rs`; current `crates/daemon/src/db/sqlite.rs` remains the single connection wrapper the module builds on.
- AC3: Four validator overlay skill directories are created under `skills/validation/`.
- AC4: `apps/web/` is created as the read-only Next.js portal.
- AC5: The chosen SQLite migration number `0012` was verified on 2026-05-06 by listing existing flat repo migrations; `0011_commercial_basis_coverage.sql` was the last existing migration.
- AC6: Validation enforcement extends existing L3 modules and adds `crates/daemon/src/validation/calibration.rs`; it does not fork or replace `validation/policy.rs` or `validation/executor.rs`.
- AC7: The web portal extends the existing `packages/ui/freshness-badge.tsx` component rather than creating a duplicate FreshnessBadge component.

---

## 2. Supabase Schema, RLS & Remote-Owned Tables

> **Ownership:** This section owns the Supabase-side schema: projection tables, summary tables, remote-owned tables, junction tables, RLS policies, and migration listing. Push/pull protocol mechanics are owned by §1 (Sync Engine). L4 owns the SQLite sync-infrastructure DDL in `migrations/0012_l4_sync_state.sql`; L0 reserves only the forward-compatible shape that L4 must materialize.

### 2.1 Supabase Projection Tables (Push-Owned)

All 20 push-owned SQLite tables get Supabase counterparts. These are **push-only projections** — the daemon pushes via `apply_push_batch(batch_json, writer_epoch)` RPC. The local SQLite copy is authoritative; Supabase is a read-only mirror for the web portal.

| ID | # | Table | Scope | Notes |
|----|---|-------|-------|-------|
| L4-FR-2.1.1 | 1 | `procurement_processes` | Process | Canonical process entity |
| L4-FR-2.1.2 | 2 | `contract_notices` | Notice | Source-specific notice records |
| L4-FR-2.1.3 | 3 | `notice_lots` | Lot | Per-notice lot decomposition |
| L4-FR-2.1.4 | 4 | `tender_document_sets` | Process | Acquired document sets |
| L4-FR-2.1.5 | 5 | `qualification_runs` | Lot | Per-lot qualification results |
| L4-FR-2.1.6 | 6 | `product_candidates` | Lot | Extracted product candidates |
| L4-FR-2.1.7 | 7 | `search_profiles` | Candidate | Structured search profiles |
| L4-FR-2.1.8 | 8 | `search_attempts` | Candidate | Per-wave search execution records |
| L4-FR-2.1.9 | 9 | `supplier_quotes` | Candidate | Marketplace/RFQ quotes |
| L4-FR-2.1.10 | 10 | `candidate_pricing_assessments` | Candidate | Gross margin gate results |
| L4-FR-2.1.11 | 11 | `landed_cost_estimates` | Candidate-quote | Per-candidate-quote cost estimates |
| L4-FR-2.1.12 | 12 | `manifests` | Process | Generated opportunity manifests |
| L4-FR-2.1.13 | 13 | `pipeline_assessments` | Process | Process-level pipeline rollup |
| L4-FR-2.1.14 | 14 | `bid_workflow_states` | Process | Bid automation state (dormant until L6) |
| L4-FR-2.1.15 | 15 | `process_stage_status` | Scoped | Per-stage execution records |
| L4-FR-2.1.16 | 16 | `process_stage_heads` | Scoped | Current stage head per scope |
| L4-FR-2.1.17 | 17 | `process_relations` | Process | Inter-process relationship edges |
| L4-FR-2.1.18 | 18 | `process_aliases` | Process | Merged-process alias redirects |
| L4-FR-2.1.19 | 19 | `document_extractions` | Notice | Structural extraction output (markdown, sections, tables) |
| L4-FR-2.1.20 | 20 | `document_semantic_extractions` | Notice | Semantic mapping output from pdf-extraction skills |

Schema for these tables mirrors the SQLite DDL from L0 §2 with Postgres type substitutions (TEXT → text, INTEGER → integer/bigint, etc.). All push-owned columns (`owner_id`, `row_version`, `last_local_seq`, `stage_run_id`, `created_at`, `updated_at`) are present.

**Acceptance criteria for L4-FR-2.1.1 through L4-FR-2.1.20**
- AC1: Each listed table has a Supabase projection table with columns matching the canonical SQLite logical schema after Postgres type substitution.
- AC2: Each table includes push-owned sync tracking fields needed by `apply_push_batch`: `owner_id`, `row_version`, `last_local_seq`, `stage_run_id`, `created_at`, and `updated_at`.
- AC3: Each projection is written only by the daemon through `apply_push_batch` and is read-only for web viewer credentials.
- AC4: Projection DDL avoids SQL foreign keys to local-only tables; cross-authority references remain opaque IDs.

### 2.2 Summary Tables (Push-Owned Projections)

Redacted/summarized views of local-only tables, plus a read-only decision summary projection of remote-owned dispositions, pushed to Supabase for web portal visibility.

| ID | Table | Source | Purpose | Key Fields |
|----|-------|--------|---------|-----------|
| L4-FR-2.2.1 | `workflow_job_summaries` | `workflow_jobs` (local-only) | Redacted job status for web | `job_id`, `job_kind`, `status`, `skill_path`, `started_at`, `completed_at`, `error_type` (no full context) |
| L4-FR-2.2.2 | `error_manifest_summaries` | `error_manifests` (local-only) | Redacted error context for web | `manifest_id`, `job_id`, `error_type`, `error_code`, `incident_signature`, `created_at` (no full context_json or markdown) |
| L4-FR-2.2.3 | `validation_verdict_summaries` | `validation_verdicts` (local-only) | Verdict status for web | `verdict_id`, `worker_job_id`, `verdict`, `basis_sufficiency`, `enforcement_eligible`, `created_at` (no full reasoning) |
| L4-FR-2.2.4 | `viewer_sync_status` | Daemon sync state | Sync freshness indicator | `sync_health` (healthy/degraded/offline), `last_push_success_at`, `pending_outbox_count`, `schema_epoch`, `projection_generation` |
| L4-FR-2.2.5 | `executors` | `executors` (push-owned) | Daemon heartbeat summary | `executor_id`, `heartbeat_at`, `daemon_version`, `started_at` |
| L4-FR-2.2.6 | `operator_decision_summaries` | `operator_decision_heads` (remote-owned, pulled locally) | Decision display for web viewers | `process_id`, `disposition`, `decision_stale`, `reviewed_at_stage_run_id`, `updated_at` (no operator identity, no company profile, no command history) |

**Acceptance criteria for L4-FR-2.2.1 through L4-FR-2.2.6**
- AC1: Each summary projection is written by the daemon through the push batch path and is readable by viewer credentials.
- AC2: Each summary omits local-only raw context, full error markdown, validator reasoning, operator identity, company profile fields, raw supplier data, and tender document metadata unless explicitly listed as a redacted field above.
- AC3: `operator_decision_summaries` is derived from pulled `operator_decision_heads` and does not expose command ledger history.
- AC4: Redaction tests in `redaction_boundary.rs` cover all six summary projections, including `viewer_sync_status` and `executors`.

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
  remote_change_seq bigint NOT NULL DEFAULT nextval('shared_company_profile_change_seq'),
  remote_revision bigint NOT NULL DEFAULT 1,
  deleted_at timestamptz,       -- tombstone for soft-delete
  created_at timestamptz NOT NULL DEFAULT now(),
  updated_at timestamptz NOT NULL DEFAULT now()
);
```

**L4-FR-2.3.1 — `shared_company_profile` is remote-owned bidder profile authority**
- AC1: Supabase creates `shared_company_profile` with `org_id`, bidder identity fields, profile JSON fields, `remote_change_seq`, `remote_revision`, `deleted_at`, and timestamps.
- AC2: The daemon pulls this table into local cache and does not write it directly.
- AC3: Admin role can mutate the table; viewer and operator roles cannot read raw company profile data.
- AC4: Tombstoned rows remain visible to daemon pull policies until local deletion propagation succeeds.

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
  remote_change_seq bigint NOT NULL DEFAULT nextval('shared_source_configs_change_seq'),
  remote_revision bigint NOT NULL DEFAULT 1,
  deleted_at timestamptz,
  created_at timestamptz NOT NULL DEFAULT now(),
  updated_at timestamptz NOT NULL DEFAULT now()
);
```

**L4-FR-2.3.2 — `shared_source_configs` is remote-owned source enablement authority**
- AC1: Supabase creates `shared_source_configs` with `source_id`, `source_type`, `display_name`, `enabled`, `skill_path`, schedule/config fields, `remote_change_seq`, `remote_revision`, `deleted_at`, and timestamps.
- AC2: The daemon pulls source configuration and applies the staleness classes in §1.8 when pull is stale.
- AC3: Source enable/disable is admin-owned remote state; daemon and viewer credentials cannot mutate it directly.
- AC4: A remotely disabled source is treated as safety-critical stale config and degrades after the default 4-hour TTL if pull cannot refresh.

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
  remote_change_seq bigint NOT NULL DEFAULT nextval('operator_decision_heads_change_seq'),
  remote_revision bigint NOT NULL DEFAULT 1,
  deleted_at timestamptz,
  created_at timestamptz NOT NULL DEFAULT now(),
  updated_at timestamptz NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX idx_decisions_process ON operator_decision_heads(process_id);
CREATE INDEX idx_decisions_disposition ON operator_decision_heads(disposition);
```

**CAS semantics:** Updates via `update_operator_decision(process_id, disposition, base_remote_revision, command_id)` RPC. Mismatch on `base_remote_revision` → conflict returned (not silently overwritten). `command_id` provides idempotency for lost-response retries. Direct table UPDATE is blocked — only the RPC path is exposed.

**L4-FR-2.3.3 — `operator_decision_heads` is remote-authoritative decision state**
- AC1: Supabase creates `operator_decision_heads` with one unique row per `process_id`.
- AC2: `remote_revision` increments only through the CAS RPC or admin-controlled maintenance path.
- AC3: `decision_stale` can be set when `reviewed_basis_hash`, `reviewed_manifest_hash`, `reviewed_at_stage_run_id`, or later L5 freshness vectors no longer match current decision basis.
- AC4: The daemon pulls decision heads for local display/reconciliation and does not directly update this table outside CAS.
- AC5: Direct UPDATE by `viewer` or browser-held credentials is impossible under RLS.

### 2.4 Junction Tables (Supabase)

Mirrored from SQLite for web portal querying:

| ID | Table | Purpose |
|----|-------|---------|
| L4-FR-2.4.1 | `notice_cpv_codes` | Notice ↔ CPV code mapping |
| L4-FR-2.4.2 | `product_commodity_codes` | Product ↔ commodity code mapping |
| L4-FR-2.4.3 | `lot_cpv_codes` | Lot ↔ CPV code mapping |

**Acceptance criteria for L4-FR-2.4.1 through L4-FR-2.4.3**
- AC1: Each junction table has a Supabase migration counterpart queryable by the web portal.
- AC2: Junction projection writes are push-owned and applied through the same batch RPC as parent projections.
- AC3: `product_commodity_codes` preserves its L0 full-entity shape and is not collapsed into a thin junction.

### 2.5 RLS Policy Matrix

Row-Level Security policies enforce the authority matrix at the Supabase database boundary. All tables scoped by `org_id` (implicit single-org in MVP).

| ID | Table Category | Tables | `viewer` | `operator` | `admin` | `daemon` |
|----|---|---|---|---|---|---|
| L4-FR-2.5.1 | Push-owned projections | All 20 projection tables | SELECT | SELECT | SELECT | INSERT, UPDATE (via `apply_push_batch` RPC) |
| L4-FR-2.5.2 | Push-owned summaries | 5 summary tables + executors | SELECT | SELECT | SELECT | INSERT, UPDATE (via `apply_push_batch` RPC) |
| L4-FR-2.5.3 | Remote-owned config | `shared_company_profile` | — | — | Full CRUD | SELECT (pull) |
| L4-FR-2.5.4 | Remote-owned config | `shared_source_configs` | — | — | Full CRUD | SELECT (pull) |
| L4-FR-2.5.5 | Remote-owned decisions | `operator_decision_heads` | — | SELECT, UPDATE (via CAS RPC) | Full CRUD | SELECT (pull) |
| L4-FR-2.5.6 | Decision display projection | `operator_decision_summaries` (push-owned) | SELECT | SELECT | SELECT | INSERT, UPDATE (via `apply_push_batch` RPC) |
| L4-FR-2.5.7 | Sync infrastructure | `schema_metadata`, `active_writer_lease` | — | — | SELECT, UPDATE | SELECT, UPDATE |
| L4-FR-2.5.8 | CAS infrastructure | `operator_decision_commands` | — | — | SELECT | INSERT (via CAS RPC), SELECT |

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

**L4-FR-2.5.9 — Web read-only access is enforced by RLS, not by UI hiding**
- AC1: The web portal uses viewer-scoped credentials only.
- AC2: Viewer credentials can SELECT push-owned projections, summary projections, and `operator_decision_summaries`.
- AC3: Viewer credentials cannot INSERT, UPDATE, DELETE, call mutation RPCs, or SELECT remote-owned raw config/decision tables.
- AC4: Browser-held credentials never include operator, admin, or daemon role privileges.

**L4-FR-2.5.10 — Operator and daemon mutation rights stay behind RPC boundaries**
- AC1: Operator decision mutation is available only through `update_operator_decision()`.
- AC2: Daemon projection mutation is available only through `apply_push_batch(batch_json, writer_epoch)`.
- AC3: Direct UPDATE is blocked for operator decision heads except admin maintenance paths.
- AC4: Daemon role can see tombstoned remote-owned rows needed for pull propagation.

**L4-FR-2.5.11 — Redaction boundary tests cover every summary projection**
- AC1: `redaction_boundary.rs` tests `workflow_job_summaries`, `error_manifest_summaries`, `validation_verdict_summaries`, `viewer_sync_status`, `executors`, and `operator_decision_summaries`.
- AC2: Tests fail if any summary exposes operator identity, raw company profile fields, raw supplier data, tender document metadata, full error context, validator reasoning, or command history.
- AC3: Tests use viewer credentials to verify both allowed reads and blocked raw-table access.

### 2.6 Supabase Migrations

L4 creates the net-new `infra/supabase/` directory tree. Eight migration files create the full Supabase schema:

| ID | # | Migration | Purpose |
|----|---|-----------|---------|
| L4-FR-2.6.1 | 1 | `infra/supabase/migrations/0001_projection_tables.sql` | CREATE TABLE for all 20 push-owned projection tables + 3 junction tables. Schema mirrors SQLite DDL with Postgres types. |
| L4-FR-2.6.2 | 2 | `infra/supabase/migrations/0002_remote_owned_tables.sql` | CREATE TABLE for `shared_company_profile`, `shared_source_configs`, `operator_decision_heads`, plus per-domain `remote_change_seq` sequences/triggers for those remote-owned domains. |
| L4-FR-2.6.3 | 3 | `infra/supabase/migrations/0003_summary_tables.sql` | CREATE TABLE for 5 summary tables: `workflow_job_summaries`, `error_manifest_summaries`, `validation_verdict_summaries`, `viewer_sync_status` including `projection_generation`, `executors`. |
| L4-FR-2.6.4 | 4 | `infra/supabase/migrations/0004_rls_policies.sql` | RLS policies for all 4 roles (viewer, operator, admin, daemon) across all table categories. |
| L4-FR-2.6.5 | 5 | `infra/supabase/migrations/0005_sync_rpcs.sql` | `apply_push_batch(batch_json, writer_epoch)` transactional RPC + `update_operator_decision(process_id, disposition, base_revision, command_id)` CAS RPC. |
| L4-FR-2.6.6 | 6 | `infra/supabase/migrations/0006_schema_metadata.sql` | `schema_metadata` table with `schema_epoch` plus dedicated `active_writer_lease` table with `writer_epoch` for fencing. |
| L4-FR-2.6.7 | 7 | `infra/supabase/migrations/0007_command_ledger.sql` | `operator_decision_commands` table for durable CAS idempotency. |
| L4-FR-2.6.8 | 8 | `infra/supabase/migrations/0008_decision_summaries.sql` | `operator_decision_summaries` push-owned projection for web viewer access. |

**Schema coordination rule:** Both SQLite and Postgres derive from the same canonical logical schema. CI must apply both migrations from scratch, introspect both, and fail if logical schema hash differs.

**SQLite sync migration:** `migrations/0012_l4_sync_state.sql` — adds `local_outbox`, `sync_batches`, `pull_cursors`, `provisional_worker_outputs`, `enforcement_readiness`, `projection_generations`, `validation_calibration_cases`, and `validation_incident_clusters` tables to SQLite (see §1, §3.4, §3.6, and §5.2 for DDL requirements). This is a SQLite-only migration, separate from the Supabase migration namespace above. The `0012` sequence was chosen because `migrations/0011_commercial_basis_coverage.sql` was the last existing flat migration when this spec was validated on 2026-05-06.

**L4-FR-2.6.9 — Supabase infrastructure tree is created from scratch**
- AC1: `infra/supabase/config.toml` is created with the project-local Supabase configuration.
- AC2: All eight Supabase migrations live under `infra/supabase/migrations/`.
- AC3: The repo currently has no `infra/` directory; L4 owns creating this tree.

**L4-FR-2.6.10 — SQLite and Supabase logical schemas stay coordinated**
- AC1: CI applies `migrations/0012_l4_sync_state.sql` and all eight Supabase migrations from scratch in a clean database.
- AC2: CI introspects both logical schemas after migration.
- AC3: CI fails if the computed logical schema hash differs between the SQLite logical source and Supabase projection target.
- AC4: This gate is testable and cannot be replaced by manual inspection.

**L4-FR-2.6.11 — SQLite sync migration owns local sync infrastructure DDL**
- AC1: `migrations/0012_l4_sync_state.sql` creates `local_outbox`, `sync_batches`, `pull_cursors`, `provisional_worker_outputs`, `enforcement_readiness`, `projection_generations`, `validation_calibration_cases`, and `validation_incident_clusters`.
- AC2: `local_outbox` includes `commit_group_id` and `writer_epoch` when first materialized.
- AC3: The L4 migration is added to the daemon migration registry in `crates/daemon/src/db/sqlite.rs` during implementation.

### 2.7 Cross-Authority Consistency

Pipeline data crosses authority boundaries at specific points. Each crossing has a classification:

| ID | Crossing Point | Authority Domains | Classification | Provenance Tracking |
|----|---|---|---|---|
| L4-FR-2.7.1 | Pipeline output → operator review | Push-owned → display | **Display** — no authority conflict | `decisionBasisHash` links decision to pipeline snapshot |
| L4-FR-2.7.2 | Operator decision → bid workflow | Remote-owned → push-owned automation | **Automation** — must track decision provenance | `reviewedAtStageRunId`, `reviewedBasisHash` carried through |
| L4-FR-2.7.3 | Coverage metrics → qualification gate | Push-owned → push-owned control | **Control** — no authority crossing | Internal to pipeline |
| L4-FR-2.7.4 | Sync health → UX mode | Push-owned infra → display | **Display** — no authority conflict | `viewer_sync_status` drives 3-state UX |

**Provenance of remote-owned config used in a run:** Record the exact revisions consumed by each pipeline run: `source_config_revision` (from `shared_source_configs.remote_revision` at pull time), `company_profile_revision` (if consumed, e.g., L6 bid workflow), `operator_decision_revision` (when a workflow consumes a decision). This enables replay and blame assignment when something goes wrong.

**L4-FR-2.7.5 — Pipeline runs record cross-authority provenance revisions**
- AC1: Runs that consume `shared_source_configs` record `source_config_revision`.
- AC2: Runs that consume `shared_company_profile` record `company_profile_revision`.
- AC3: Runs that consume `operator_decision_heads` record `operator_decision_revision`.
- AC4: Provenance is persisted with the run or stage state so replay can explain which remote-owned revision influenced output.

---

## 3. Web Portal & Validation Enforcement

### 3.1 Web Viewing Portal (`apps/web/` -- 7 files)

The web viewing portal is a **read-only** hosted client. It reads Supabase projection tables and receives live invalidation hints via Supabase Broadcast. It does NOT support mutations, decision capture, agent management, or configuration changes.

`apps/web/` does not exist before L4; this layer creates it. Root `package.json` workspaces already cover `apps/*` (so `apps/web` is included once the directory exists), but the root scripts are explicit and currently build/typecheck only `apps/desktop`. L4 must update `bun run build`, `bun run test`, and `bun run lint` so `apps/web` is actually verified, while preserving the existing `packages/ui` lint boundary that bans imports from `@tauri-apps/api`, `apps/desktop`, `crates/daemon`, `supabase`, and `transport`.

**L4-FR-3.1.1 — `apps/web/` is a net-new read-only Next.js portal**
- AC1: L4 creates `apps/web/` with the seven files listed below.
- AC2: The web portal reads Supabase projection/summary tables through web-local data-fetching code only.
- AC3: The web portal exposes no decision capture, agent controls, daemon bridge calls, or configuration mutation surfaces.
- AC4: Root Bun scripts include `apps/web` in build, test, and lint paths, with workspace coverage confirmed through the existing `apps/*` entry.
- AC5: `packages/ui` remains presentation-only; Supabase and Broadcast imports stay in `apps/web/src/lib/`.
- AC6: `packages/ui` remains free of Supabase, Tauri, and daemon imports; `apps/web/src/lib/supabase.ts` is the sole direct Supabase client/package boundary in the web app, and `apps/web/src/lib/broadcast.ts` consumes that boundary rather than importing Supabase directly.
- AC7: Root `bun run build` runs an `apps/web` build/typecheck step, either `(cd apps/web && bun run build)` for Next.js or `bunx tsc -p apps/web/tsconfig.json --noEmit` if the web package keeps build and typecheck separate.
- AC8: Root `bun run test` runs `apps/web` tests/E2E or a documented `test:e2e` script, and root `bun run lint` runs the web lint command in addition to preserving the `packages/ui` import-boundary grep.

**Architecture:**

```text
Next.js / React
  -> Supabase snapshot queries (projection tables)
  -> Supabase Broadcast private channels (invalidation hints)
  -> refetch / patch read-only projections
```

**7 files:**

| ID | File | Responsibility |
|----|------|---------------|
| L4-FR-3.1.2 | `src/app/layout.tsx` | Root layout with global sync status badge (`viewer_sync_status`) |
| L4-FR-3.1.3 | `src/app/page.tsx` | Redirect to contracts list |
| L4-FR-3.1.4 | `src/app/contracts/page.tsx` | Keyset-paginated opportunity list using Supabase client + `packages/ui` `ContractCardFrame`. Shows "N new/updated" pill on live updates (not live reorder). |
| L4-FR-3.1.5 | `src/app/contracts/[id]/page.tsx` | Opportunity detail: candidates, quotes, margins, manifest, errors. Auto-refresh for minor changes; non-destructive refresh chip for major changes. |
| L4-FR-3.1.6 | `src/app/status/page.tsx` | Pipeline status, job summaries, error summaries, sync health |
| L4-FR-3.1.7 | `src/lib/supabase.ts` | Supabase client initialization, typed query helpers using `packages/contracts-schema` types |
| L4-FR-3.1.8 | `src/lib/broadcast.ts` | Supabase Broadcast subscription manager: topic-based invalidation, per-page subscription lifecycle |

**Web-specific data-fetching:** The web uses Supabase client for all data access. It reuses `packages/ui` presentational components (ContractCardFrame, ManifestViewer, PipelineBadge, FreshnessBadge, ErrorSummaryList) but has its own data-fetching hooks. No SQLite, no daemon bridge, no `invoke()`.

**Acceptance criteria for L4-FR-3.1.2 through L4-FR-3.1.8**
- AC1: Each listed file exists under `apps/web/` and has the listed responsibility.
- AC2: E2E tests verify list, detail, status, read-only access, keyset pagination, freshness badge, and viewer credential boundaries.
- AC3: Web code does not import Tauri APIs, desktop modules, daemon code, or SQLite/IPC transport.

### 3.2 Supabase Broadcast Invalidation

The portal uses **Supabase Broadcast as an invalidation plane**, not as a database subscription. Broadcast payloads are small invalidation hints, not full rows.

**Why Broadcast over Postgres Changes:** Postgres Changes is simpler but scales worse (single-threaded for ordering). Broadcast supports private channels, custom payloads via `realtime.send`, and better security scoping.

**Topic scheme (page-level, not row-level):**

| Page | Topics |
|------|--------|
| `/contracts` | `org:{orgId}:contracts`, `org:{orgId}:sync` |
| `/contracts/[id]` | `process:{processId}:updates`, `org:{orgId}:sync` |
| `/status` | `org:{orgId}:status`, `org:{orgId}:errors`, `org:{orgId}:sync` |

**L4-FR-3.2.1 — Broadcast subscriptions are page-scoped invalidation channels**
- AC1: `/contracts` subscribes to `org:{orgId}:contracts` and `org:{orgId}:sync`.
- AC2: `/contracts/[id]` subscribes to `process:{processId}:updates` and `org:{orgId}:sync`.
- AC3: `/status` subscribes to `org:{orgId}:status`, `org:{orgId}:errors`, and `org:{orgId}:sync`.
- AC4: Broadcast topics carry invalidation hints only, not full projection rows.

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
      projectionGeneration: number
    }
```

**Emission point:** Daemon emits data-change Broadcast notifications **after** the remote projection batch is successfully applied. Status-only `sync.updated` notifications emit after the `viewer_sync_status` heartbeat/status update commits. Coalesces affected topics so 20 quote upserts produce one notification per topic.

**L4-FR-3.2.2 — Broadcast payloads use the three typed invalidation shapes**
- AC1: `contracts.changed` carries affected process IDs, changed sections, and `maxSeq`.
- AC2: `process.updated` carries process ID, changed sections, major/minor flag, and `maxSeq`.
- AC3: `sync.updated` carries sync health, last acknowledged sequence, last success timestamp, and `projectionGeneration` so web pages can detect an epoch-repair reseed and invalidate snapshots.
- AC4: Payload schemas are covered by TypeScript typing and E2E or unit tests.

**L4-FR-3.2.3 — Broadcast emits only after successful remote apply**
- AC1: The daemon emits after `apply_push_batch` commits.
- AC2: Failed or rejected batches emit no data-change Broadcast.
- AC3: Multiple updates in one batch are coalesced to one notification per affected topic.
- AC4: `sync.updated` emits after committed `viewer_sync_status` heartbeat/status-only updates even when no projection data rows changed.

**Per-page behavior:**
- **List page:** Patch badges/counts if in payload. Otherwise invalidate list query. Show "N new/updated opportunities" pill -- do NOT reorder the visible list while user is reading.
- **Detail page (minor):** Auto-refresh the affected section.
- **Detail page (major -- manifest, landed cost, canonical process):** Show non-destructive refresh chip. Do NOT silently swap content under the user.

**L4-FR-3.2.4 — Web pages respond to invalidation without disruptive content swaps**
- AC1: List page live updates never reorder the visible list while a user is reading it.
- AC2: Detail-page minor updates auto-refresh only the affected section.
- AC3: Detail-page major updates show a refresh chip and wait for user action before replacing visible content.

### 3.3 Degraded-Mode UX -- Three Distinct States

The portal is honest about staleness. Three states, not one badge:

| ID | State | Indicator | UX |
|----|-------|-----------|-----|
| L4-FR-3.3.1 | **Healthy** | Green badge, "Data current" | Full real-time updates via Broadcast. All data fresh. |
| L4-FR-3.3.2 | **Degraded** | Amber badge, "Updates delayed -- data may be behind desktop" | Broadcast connected but daemon sync is behind. Data is stale by 90s-5min. All features work but data is not current. |
| L4-FR-3.3.3 | **Offline** | Red badge, "Sync offline -- showing last known data" | No sync for >5min or daemon offline. Browse cached projection data only. Broadcast disconnected. |

**Thresholds (from `viewer_sync_status` projection table):**
- Green: < 90 seconds since last successful sync
- Amber: 90s -- 5 min
- Red: > 5 min or daemon offline

**Acceptance criteria for L4-FR-3.3.1 through L4-FR-3.3.3**
- AC1: Green appears when last successful sync is younger than 90 seconds.
- AC2: Amber appears when last successful sync is 90 seconds to 5 minutes old.
- AC3: Red appears when last successful sync is older than 5 minutes or daemon sync is offline.
- AC4: Web E2E verifies the three labels, tones, and tooltips from `viewer_sync_status`.

The `viewer_sync_status` table (push-owned projection) is updated after each successful remote apply and on lease-heartbeat/status-only updates while sync is active:

```typescript
interface ViewerSyncStatus {
  orgId: string
  syncHealth: 'healthy' | 'degraded' | 'offline'
  lastPushSuccessAt: string       // ISO-8601
  lastPushAttemptAt: string       // ISO-8601
  lastAckedLocalSeq: number       // Latest local_seq successfully pushed
  pendingOutboxCount: number      // Entries waiting to be pushed
  schemaEpoch: number             // Current schema epoch
  projectionGeneration: number    // Current remote projection generation
  daemonVersion: string           // Daemon version string
}
```

**L4-FR-3.3.4 — `viewer_sync_status` drives freshness state across surfaces**
- AC1: `viewer_sync_status` stores `syncHealth`, `lastPushSuccessAt`, `lastPushAttemptAt`, `lastAckedLocalSeq`, `pendingOutboxCount`, `schemaEpoch`, `projectionGeneration`, and `daemonVersion`.
- AC2: The row updates after each successful remote apply and during lease-heartbeat/status-only updates so freshness can change even when no projection rows changed.
- AC3: The web portal computes freshness from this projection, not from local daemon IPC.
- AC4: Desktop sync warning surfaces consume the same health semantics once sync is active.
- AC5: `packages/ui/freshness-badge.tsx` accepts `syncHealth: 'healthy' | 'degraded' | 'offline'` and `lastSuccessAt: ISO-8601 string` in addition to its display VM path; the 90s and 300s thresholds are computed inside the component or a co-located pure helper, not separately by each caller.

**Desktop also shows three states** (upgraded from L2's basic detection):
- **Healthy:** Daemon connected, sync healthy -- full desktop
- **Remote degraded:** Daemon connected, sync behind/offline -- full local ops, sync warning banner
- **Browse-only:** Daemon unavailable -- read-only local browsing, clear banner

**L4-FR-3.3.5 — Desktop adds sync health without replacing L2 control-plane state**
- AC1: Desktop keeps L2 daemon/storage/control/load health dimensions.
- AC2: Sync health adds healthy, remote-degraded, and browse-only behavior as an additional dimension.
- AC3: Remote-degraded mode does not block local write/trigger operations.

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

**L4-FR-3.4.1 — Enforcement mode turns validator rejection into quarantine**
- AC1: Rejected enforcement-active worker output is classified as `AGENT_SUSPECT_OUTPUT`.
- AC2: The rejected output does not advance stage heads.
- AC3: The rejected output does not create outbox entries or remote projection rows.
- AC4: Pipeline rollups exclude quarantined results.
- AC5: An error manifest with operator alert is created for each enforcement rejection.

**Provisional promotion buffer:** For enforcement-active slices, worker outputs are held in local-only staging until validation completes (see §1.3). This resolves the collision between the outbox invariant ("every push-owned mutation creates an outbox entry") and enforcement ("rejected results must not be outboxed"). Without the promotion buffer, rejected rows would already be committed and outboxed before validation completes.

**L4-FR-3.4.2 — Enforcement uses the L4 provisional promotion buffer**
- AC1: The validation executor extends the existing L3 executor path with provisional-result staging instead of replacing the L3 policy/executor modules.
- AC2: Approved provisional results are promoted atomically with outbox creation and stage-head advancement.
- AC3: Rejected provisional results stay local and auditable.
- AC4: Fail-open validator-crash behavior remains the L3 fail-open path unless a slice-specific enforcement rule explicitly quarantines worker output after a completed rejection verdict.
- AC5: An operator override of a rejected provisional result records `validation_verdicts.override_verdict`, `override_reason`, `override_by`, and `override_at`, then promotes the same provisional payload through the normal approved path in one SQLite transaction.
- AC6: Override promotion changes `provisional_worker_outputs.validation_status` to `approved`, creates push-owned rows/outbox entries, and advances stage heads exactly once using the original `worker_job_id`.
- AC7: Override promotion emits audit/provenance fields so downstream rollups can distinguish validator-approved results from operator-overridden results without treating the row as quarantined.
- AC8: Replaying the same override command is idempotent and cannot create duplicate outbox rows.

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

**L4-FR-3.4.3 — Enforcement readiness is per slice, not global**
- AC1: The enforcement slice key is worker skill x source x worker behavior version x validator behavior version.
- AC2: Enforcement can be enabled for one slice without enabling it for other slices.
- AC3: Behavior versions change only when output semantics or decision boundaries materially change.

**L4-FR-3.4.4 — A slice enters enforcement only after seven prerequisites pass**
- AC1: The validator overlay compatible-version range includes the current worker behavior version.
- AC2: The slice has at least 30 adjudicated rejected/flagged cases with known ground truth.
- AC3: The slice has at least 60 audited approved cases checked for silent misses.
- AC4: Wilson-bound reject precision lower bound is at least 70%.
- AC5: Rule-of-three false-pass upper bound is at most 5%.
- AC6: Challenge bank covers dominant failure families for the slice.
- AC7: No automatic demotion trigger is active at promotion time.

**Automatic demotion** from enforcement back to dry-run:
- Operator override rate spikes (operators regularly reversing rejections)
- Challenge-bank performance regresses (validator stops catching known-bad)
- Anomaly-conditioned validator error rate rises
- Worker behavior version moves outside overlay's compatible range

Demotion is automatic and immediate. Re-promotion requires meeting calibration thresholds again. Default measurable thresholds are:
- Override spike: rolling 7-day window has at least 10 operator-reviewed enforcement rejections and either `operator_override_rate_bps >= 1500` or at least 2x the slice's trailing 30-day baseline.
- Challenge-bank regression: any critical mutated-bad assertion is approved, or total challenge-bank pass rate drops below 95%.
- Anomaly-conditioned validator error rise: anomaly-triggered validations have at least 20 samples in the rolling window and validator execution/error rate exceeds 5% or 2x the trailing 30-day baseline.
- Compatible-range exit: current `worker_behavior_version` is outside the overlay's declared compatible range.

**L4-FR-3.4.5 — Demotion triggers immediately return a slice to dry-run**
- AC1: Operator override-rate spike uses the rolling-window threshold above and disables enforcement for the affected slice.
- AC2: Challenge-bank regression uses the critical-assertion/pass-rate threshold above and disables enforcement for the affected slice.
- AC3: Elevated anomaly-conditioned validator error rate uses the rolling-window threshold above and disables enforcement for the affected slice.
- AC4: Worker behavior version outside overlay compatible range disables enforcement for the affected slice immediately.
- AC5: Re-promotion is blocked until all calibration thresholds pass again.
- AC6: Threshold values are configurable per slice but default values above are the minimum implementation; configuration cannot weaken the requirement that demotion be automatic and measurable.

**L4-FR-3.4.6 — Enforcement readiness state is durable SQLite state**
- AC1: `migrations/0012_l4_sync_state.sql` creates `enforcement_readiness` keyed by the enforcement slice tuple, with a deterministic `slice_id` plus `worker_skill_id`, `source_id`, `worker_behavior_version`, and `validator_overlay_version`.
- AC2: `enforcement_readiness.enforcement_state` is constrained to `eligible`, `dry_run`, or `demoted`.
- AC3: The table stores `last_evaluated_at`, `wilson_lower_bound_bps`, `false_pass_upper_bound_bps`, `adjudicated_count`, `audited_approvals_count`, and nullable `demotion_reason`.
- AC4: `validation/policy.rs` reads the persisted readiness row when deciding enforcement mode, and calibration/demotion jobs update the row so state survives daemon restart.
- AC5: Metrics and tests can query `enforcement_readiness` directly to verify promotion, demotion, and re-promotion behavior.
- AC6: `enforcement_readiness` stores the active demotion threshold config snapshot used for the latest decision, so later audits can explain why enforcement was enabled or demoted.

### 3.5 Domain-Specific Validator Overlays (4 skills at L4; +1 added by L5)

Layer 3 deploys the base validator (generic semantic quality). Layer 4 adds the four domain-specific overlays below that check business rules unique to each skill family. (The goods-led labour-cost extension later added a fifth overlay, `overlay-labour-cost`, listed under "Later additions".)

| ID | Overlay | Directory | Domain Checks |
|----|---------|-----------|--------------|
| L4-FR-3.5.1 | **Procurement ingestion** | `skills/validation/overlay-procurement-ingestion/` | Notice completeness, OCDS field presence, correct source locale, notice-type consistency, expiry date plausibility, CPV code validity |
| L4-FR-3.5.2 | **Supplier enrichment** | `skills/validation/overlay-supplier-enrichment/` | Price plausibility (too high/low for category), currency correctness, pack basis consistency, seller diversity, listing freshness, compliance evidence completeness for regulated goods |
| L4-FR-3.5.3 | **Document acquisition** | `skills/validation/overlay-document-acquisition/` | Document type identification correctness, download completeness, content-type verification, file size plausibility, document set hash consistency |
| L4-FR-3.5.4 | **Product extraction** | `skills/validation/overlay-product-extraction/` | Candidate type plausibility for CPV family, archetype assignment consistency, slot completeness for archetype, quantity/unit plausibility, bundle decomposition correctness |

**Later additions (not L4):**

| Added by | Overlay | Directory | Domain Checks |
|----------|---------|-----------|--------------|
| L5 (goods-led extension, 2026-06-04) | **Installation-labour cost** | `skills/validation/overlay-labour-cost/` | Labour-estimate sanity: rate-band plausibility against the curated reference tables, scope/region consistency, conservative-default usage (e.g. unknown permit responsibility → high band), and goods value-share basis coherence. See L5 §1.17. |

Each overlay follows the skill directory standard: `SKILL.md` + `result.schema.json` + `checkpoint.schema.json` + `canary.inputs.jsonl` + `golden.assertions.yaml` + `validator-rubric.yaml`.

Overlays are loaded alongside the base validator. The base checks generic quality; the overlay adds domain-specific rules. A skill can declare which overlay to use in its metadata; the Validation Executor loads both.

**Acceptance criteria for L4-FR-3.5.1 through L4-FR-3.5.4**
- AC1: Each overlay directory contains `SKILL.md`, `result.schema.json`, `checkpoint.schema.json`, `canary.inputs.jsonl`, `golden.assertions.yaml`, `validator-rubric.yaml`, and `references/`, following `skills/validation/base/`.
- AC2: Each overlay declares compatible worker behavior versions.
- AC3: The existing L3 Validation Executor loads the base validator plus the selected overlay; L4 extends the executor and does not replace it.
- AC4: Canary tests prove each overlay emits verdicts and applies the listed domain checks.

### 3.6 Formal Calibration Harness

Before enforcement, each slice must pass calibration. The harness uses **three corpora**:

**Corpus storage and durable evidence:** Static fixture corpora live under `tests/fixtures/calibration/{slice_id}/{clean,weird,mutated,pairwise}/`. Runtime/live evidence from L3 dry-run and L4 overrides is stored in SQLite `validation_calibration_cases`, keyed by `case_id` with `slice_id`, `corpus_kind`, `source_artifact_path`, `expected_verdict`, `actual_verdict`, `ground_truth_label`, `mutation_family`, `pair_id`, `model_family`, `created_at`, and `adjudicated_at`. Incident grouping is stored in `validation_incident_clusters` with `incident_signature`, dominant failure family, affected slice IDs, and candidate mutation template JSON. `crates/daemon/src/validation/calibration.rs` owns Wilson/rule-of-three math, corpus aggregation, override ingestion, and readiness inputs consumed by `validation/policy.rs`.

**1. Clean real runs (25-30 per skill family):**
Real successful runs from gold lots and live dry-run traffic. Establish baseline: what does healthy variability look like?

**L4-FR-3.6.1 — Clean real corpus establishes healthy baseline**
- AC1: Each skill family has 25-30 clean real runs from gold lots or live dry-run traffic.
- AC2: Clean runs include successful outputs and their evidence artifacts.
- AC3: Calibration records how healthy variability appears for the slice.
- AC4: Clean fixture cases are discoverable under `tests/fixtures/calibration/{slice_id}/clean/` and live clean cases are represented in `validation_calibration_cases` with `corpus_kind = 'clean'`.

**2. Weird-but-valid runs (15-20 per skill family):**
Unusual but acceptable runs: source layout changed but extraction correct, localized content correctly normalized, partial but honestly narrated data, low-result-count but legitimate.

**L4-FR-3.6.2 — Weird-but-valid corpus protects valid edge cases**
- AC1: Each skill family has 15-20 weird-but-valid runs.
- AC2: The corpus includes layout change, localization, partial honest data, and low-result-count valid examples.
- AC3: Validators must not reject these cases solely because they differ from normal clean runs.
- AC4: Weird-but-valid fixture cases are discoverable under `tests/fixtures/calibration/{slice_id}/weird/` and live cases are represented in `validation_calibration_cases` with `corpus_kind = 'weird'`.

**3. Mutated bad runs (30-50 per skill family):**
Real good runs mutated into realistic failures:

**L4-FR-3.6.3 — Mutated bad corpus exercises realistic failure families**
- AC1: Each skill family has 30-50 mutated bad runs.
- AC2: Mutations are derived from real good runs so evidence context remains realistic.
- AC3: Every mutation family listed below appears in the challenge bank with expected verdict assertions.
- AC4: Mutated fixture cases are discoverable under `tests/fixtures/calibration/{slice_id}/mutated/` and live/generated cases are represented in `validation_calibration_cases` with `corpus_kind = 'mutated'` plus `mutation_family`.

| ID | Mutation Family | What It Tests |
|----|----------------|--------------|
| L4-FR-3.6.4 | Wrong source/domain in URL chain | Validator catches navigation to wrong site |
| L4-FR-3.6.5 | Wrong page kind (search vs category vs detail) | Validator catches page-type confusion |
| L4-FR-3.6.6 | Locale/currency swap (claims GBP, evidence EUR) | Validator catches cross-checking failure |
| L4-FR-3.6.7 | Pack/unit distortion (per-unit vs per-box vs per-pallet) | Validator catches unit normalization errors |
| L4-FR-3.6.8 | Accessory-vs-primary confusion | Validator catches product identity errors |
| L4-FR-3.6.9 | Selective warning omission (says 2, daemon shows 5) | Validator catches narrative honesty |
| L4-FR-3.6.10 | Manifest/result count mismatch (claims 12, contract has 8) | Validator catches count reconciliation |
| L4-FR-3.6.11 | Fabricated evidence references | Validator catches fake anchors |
| L4-FR-3.6.12 | Cross-segment duplicates/gaps (continuation chains) | Validator catches chain-level coherence |

**Acceptance criteria for L4-FR-3.6.4 through L4-FR-3.6.12**
- AC1: Each mutation family has at least one golden assertion in the relevant overlay's `golden.assertions.yaml`.
- AC2: Each mutation family states the exact business-rule failure it is intended to reveal.
- AC3: A validator that approves the mutated-bad sample fails the overlay challenge-bank test for that family.

**Pairwise challenge sets:** In addition to absolute verdicts, build pairs: run A = clean, run B = one corruption. Measure whether the validator ranks B less trustworthy.

**L4-FR-3.6.13 — Pairwise challenge sets measure relative trust**
- AC1: Pairwise sets include a clean run and a single-corruption sibling run.
- AC2: Calibration records whether the validator ranks the corrupted run less trustworthy.
- AC3: Pairwise failures feed the same enforcement-readiness and demotion logic as absolute verdict failures.
- AC4: Pairwise fixtures are discoverable under `tests/fixtures/calibration/{slice_id}/pairwise/`, and both members share a durable `pair_id` in `validation_calibration_cases`.

**Minimum calibration targets per slice before enforcement:**
- >=30 adjudicated rejected/flagged cases with known ground truth
- >=60 audited approved cases checked for silent misses
- Wilson-bound reject precision lower bound >= 70% (~26 correct out of 30)
- Rule-of-three false-pass upper bound <= 5% (0 misses in 60 audited)
- Coverage of dominant failure families in challenge bank

**L4-FR-3.6.14 — Calibration targets are minimum gates before enforcement**
- AC1: Slice enforcement is blocked until at least 30 adjudicated rejected/flagged cases with known ground truth exist.
- AC2: Slice enforcement is blocked until at least 60 audited approved cases have been checked for silent misses.
- AC3: Wilson-bound reject precision lower bound must be at least 70%.
- AC4: Rule-of-three false-pass upper bound must be at most 5%.
- AC5: Challenge bank coverage must include the dominant failure families for that slice.
- AC6: Wilson-bound and rule-of-three calculations live in pure functions in `crates/daemon/src/validation/calibration.rs` and are asserted by tests with boundary cases around 30/60 sample counts.

**Cross-model audit sampling:** Periodically run a sample through a different model family to detect correlated blindness.

**L4-FR-3.6.15 — Cross-model audit sampling detects correlated blindness**
- AC1: Calibration schedule periodically sends a sample to a different model family.
- AC2: Disagreements are logged as calibration evidence.
- AC3: Correlated-blindness findings can block enforcement or trigger demotion for affected slices.
- AC4: Cross-model samples write `model_family` and disagreement metadata to `validation_calibration_cases`; the scheduler cadence is configurable but must run at least once per 30 days for enforcement-active slices with new traffic.

**Override feedback loop:** Every operator override becomes: (1) labeled calibration case, (2) incident-cluster member, (3) candidate mutation template for challenge bank. Note: override rate is a biased metric — overrides are selection-biased toward visible/contested cases. Use overrides as demotion signal, incident source, and mutation template source, but not as sole precision estimator.

**L4-FR-3.6.16 — Operator overrides feed calibration without becoming the sole estimator**
- AC1: Every override creates a labeled calibration case.
- AC2: Every override joins an incident cluster.
- AC3: Every override is evaluated as a candidate mutation template.
- AC4: Override rate can trigger demotion but cannot be the only precision estimator for promotion.
- AC5: Override ingestion is automatic in the daemon path that records `validation_verdicts.override_*`; it inserts/updates `validation_calibration_cases` and `validation_incident_clusters` in the same SQLite transaction as the override audit update.
- AC6: Candidate mutation templates are stored as JSON on the incident cluster for later review and are not required to become static fixtures before the override counts as calibration evidence.

**Tombstone retention:** Cursor-based pull requires tombstones to remain visible longer than the maximum expected offline window. If tombstones are GC'd too early, an offline daemon permanently misses deletions. Tombstone retention must be ≥ max offline duration + safety margin (default: 14 days). Alternatively, periodic anti-entropy / full-hash diff can detect missed deletes.

**L4-FR-3.6.17 — Tombstones remain visible long enough for offline daemons**
- AC1: Default tombstone retention is at least 14 days.
- AC2: Retention is at least maximum expected offline duration plus safety margin.
- AC3: Daemon pull policies can see tombstoned remote-owned rows until retention expires.
- AC4: If retention is shortened, periodic anti-entropy or full-hash diff is required to detect missed deletes.

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
  projection_generation BIGINT NOT NULL DEFAULT 0,
  daemon_version TEXT,
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
```

**L4-FR-3.7.1 — `viewer_sync_status` is the Supabase sync freshness projection**
- AC1: Supabase creates `viewer_sync_status` with the columns and checks shown above.
- AC2: The daemon updates the row after each successful remote apply and heartbeat/status-only update.
- AC3: Web and desktop freshness indicators read the three-state health semantics from this table/projection.
- AC4: RLS allows viewer/operator/admin SELECT and daemon INSERT/UPDATE through the push path.
- AC5: `projection_generation` increments after an epoch-repair reseed and appears in the next `sync.updated` Broadcast payload.

---

## 4. Testing & Acceptance

### 4.1 Definition of Done

| # | Criterion | Requirement IDs | Verification |
|---|----------|-----------------|-------------|
| 1 | Push protocol: transactional outbox entries created for all push-owned mutations in same SQLite transaction | L4-FR-1.3.1, L4-FR-1.9.2 | `sqlite_supabase_sync.rs` |
| 2 | Push batches applied to Supabase via single transactional RPC with schema_epoch, row_version ordering | L4-FR-1.3.5, L4-FR-1.3.6, L4-FR-1.7.1, L4-FR-1.9.1 | `sqlite_supabase_sync.rs` |
| 3 | Pull protocol: cursor-based pull for shared_source_configs, shared_company_profile, operator_decision_heads | L4-FR-1.4.1, L4-FR-1.4.2 | `sqlite_supabase_sync.rs` |
| 4 | Operator decisions: queued local intents flush through Supabase CAS RPC with base_remote_revision; conflicts returned (not silently overwritten) | L4-FR-1.4.4, L4-FR-1.5.1, L4-FR-2.3.3 | `sqlite_supabase_sync.rs` |
| 5 | Active-writer lease: second daemon instance detected and rejected; heartbeat/expiry and stale `writer_epoch` fencing verified | L4-FR-1.6.1, L4-FR-1.6.2 | `sqlite_supabase_sync.rs` |
| 6 | RLS policies enforced for all 4 roles (viewer, operator, admin, daemon) | L4-FR-2.5.1-L4-FR-2.5.10 | `redaction_boundary.rs` |
| 7 | Web viewing portal displays opportunities, manifests, pipeline status (read-only) | L4-FR-3.1.1-L4-FR-3.1.8 | `apps/web/e2e/contracts.spec.ts` |
| 8 | Supabase Broadcast invalidation delivers live updates to web portal | L4-FR-3.2.1-L4-FR-3.2.4 | `apps/web/e2e/contracts.spec.ts` |
| 9 | viewer_sync_status shows sync freshness (green/amber/red) and projection generation | L4-FR-3.3.1-L4-FR-3.3.4, L4-FR-3.7.1 | `apps/web/e2e/contracts.spec.ts` |
| 10 | Validation enforcement mode and calibration harness: validator rejections quarantine results as AGENT_SUSPECT_OUTPUT, overrides promote through audited provisional path, calibration gates compute readiness, and enforcement readiness persists per slice | L4-FR-1.3.9, L4-FR-3.4.1-L4-FR-3.4.6, L4-FR-3.6.1-L4-FR-3.6.17 | `sqlite_supabase_sync.rs` |
| 11 | 4 domain-specific validator overlays deployed and producing verdicts | L4-FR-3.5.1-L4-FR-3.5.4 | `sqlite_supabase_sync.rs` |
| 12 | Two users can view the same pipeline: desktop (read-write) + web (read-only) | L4-FR-1.8.1, L4-FR-3.1.1, L4-FR-3.3.5 | `apps/web/e2e/contracts.spec.ts` + manual smoke |
| 13 | Writer fencing: stale daemon with old `writer_epoch` rejected on push | L4-FR-1.6.2, L4-FR-1.9.5 | `sqlite_supabase_sync.rs` |
| 14 | CAS command ledger: retried command returns stored result (idempotent replay); post-GC replay is evaluated as a new command | L4-FR-1.5.2, L4-FR-1.5.3, L4-FR-1.5.4 | `sqlite_supabase_sync.rs` |
| 15 | Provisional promotion buffer: enforcement-active slices hold results in `provisional_worker_outputs` until validation completes | L4-FR-1.3.9, L4-FR-3.4.2 | `sqlite_supabase_sync.rs` |
| 16 | Equal-version retry: identical-envelope retry with matching content_hash = no-op success | L4-FR-1.3.7, L4-FR-1.9.10 | `sqlite_supabase_sync.rs` |
| 17 | Web portal uses viewer-scoped credentials only (security boundary, not UI convention) | L4-FR-2.5.9, L4-FR-3.1.1 | `redaction_boundary.rs` + `apps/web/e2e/contracts.spec.ts` |
| 18 | Redaction tests pass: no leakage of operator identity/company profile/raw supplier data in summary projections | L4-FR-2.2.1-L4-FR-2.2.6, L4-FR-2.5.11 | `redaction_boundary.rs` |

### 4.2 Test File Listing (4 New, Cumulative 52)

| File | Coverage |
|------|---------|
| `sqlite_schema.rs` | Full schema creation and migration: all tables created, all constraints enforced, all indices present, sync tracking columns populated, integer pence/bps verified, `provisional_worker_outputs`, `enforcement_readiness`, `projection_generations`, `validation_calibration_cases`, and `validation_incident_clusters` created |
| `sqlite_supabase_sync.rs` | Push-pull round-trip: outbox entry creation, batch assembly, crash-mid-batch identical-envelope retry, terminal stale-writer re-enveloping, transactional REST RPC apply, row_version ordering, writer fencing (stale epoch rejected), lease heartbeat/expiry, commit_group_id atomicity, equal-version retry no-op and hard conflict, pull cursor advancement with hi-water fence and domain-level `remote_change_seq`, pending operator intent coalescing/CAS flush, CAS conflict detection with command ledger idempotency and post-GC replay semantics, active-writer lease, schema_epoch guard, epoch-repair `projection_generation` reseed, provisional promotion buffer for enforcement slices, durable enforcement readiness promotion/demotion, four overlay canary execution paths, calibration math boundary cases |
| `redaction_boundary.rs` | RLS role-matrix and summary projection redaction tests: all 4 roles across table categories, operator_decision_summaries, workflow_job_summaries, error_manifest_summaries, validation_verdict_summaries, viewer_sync_status, executors — verify no leakage of operator identity, company profile, raw supplier data, tender document metadata, full error context, validator reasoning, or command history |
| `apps/web/e2e/contracts.spec.ts` | Web portal E2E: navigate to contracts list, verify read-only (no decision controls), verify keyset pagination, verify sync freshness indicator, verify Broadcast live updates, verify viewer credentials cannot access remote-owned tables, verify two-user desktop-seeded/web-viewer visibility with no web mutation affordances |

### 4.3 Test Scenarios

| Scenario | Requirement IDs | Expected Behavior |
|----------|-----------------|------------------|
| Push batch with stale row_version | L4-FR-1.3.7 | Remote rejects batch (idempotent); daemon retries with current version |
| Push batch with incompatible schema_epoch | L4-FR-1.7.1 | Sync pauses; local operation continues; operator alerted |
| Two desktop operators make concurrent decisions | L4-FR-1.5.1, L4-FR-2.3.3 | CAS detects conflict on base_remote_revision; second operator retries with refreshed state |
| Validator rejects output in enforcement mode | L4-FR-3.4.1, L4-FR-3.4.2 | Results quarantined as AGENT_SUSPECT_OUTPUT; error manifest created; operator override records `validation_verdicts.override_*` and promotes the original provisional payload through the normal outbox path exactly once |
| Validator itself crashes during validation | L4-FR-3.4.2 | Fail-open: worker results advance with VALIDATION_SKIPPED_VALIDATOR_ERROR; operator can review manually |
| Sync degraded (Supabase unreachable) | L4-FR-1.8.1, L4-FR-3.3.1-L4-FR-3.3.5 | Desktop stays in remote-degraded mode with full local operation and sync warning banner; web shows staleness indicator (amber/red); local operations continue uninterrupted |
| Full push replay after sync recovery | L4-FR-1.3.3, L4-FR-1.3.5, L4-FR-1.3.7 | Idempotent: no duplicates; row_version ordering prevents stale overwrites |
| Batch boundary crosses stage_run_id | L4-FR-1.3.4 | Push dispatcher rejects: batch boundaries never cross stage_run_id (enforced) |
| Pull-before-push ordering violation | L4-FR-1.4.3 | Sync startup and each sync cycle enforce pull first, then classify/coalesce pending intents, then resume CAS flushing/push |
| Network interruption or daemon crash during push batch | L4-FR-1.3.3, L4-FR-1.3.6 | Persisted `sync_batches` envelope is retried; batch either fully applied or fully rejected (transactional RPC); already-committed retry returns equal-version no-op |
| Stale daemon recovers after lease loss | L4-FR-1.6.2 | Push rejected due to stale `writer_epoch`; daemon must wait for/acquire current lease and desktop shows writer-mode conflict if another holder is active |
| CAS command retried after later command landed | L4-FR-1.5.2, L4-FR-1.5.3 | Command ledger returns stored conflict result (not re-evaluated against current state) |
| Equal-version retry with matching content_hash | L4-FR-1.3.7 | No-op success; no duplicate rows created |
| Equal-version retry with different content_hash | L4-FR-1.3.7 | Hard conflict; data corruption detected |
| Enforcement-active slice: worker completes, validator rejects | L4-FR-1.3.9, L4-FR-3.4.1 | Provisional results NOT promoted to push-owned tables; no outbox entries created; error manifest created |
| Paginated pull during concurrent remote changes | L4-FR-1.4.2 | Hi-water fence prevents skip/duplicate; pull bounded to snapshot range |
| Source remotely disabled while daemon offline for 5 hours | L4-FR-1.8.2, L4-FR-2.3.2 | Source degrade after safety-critical TTL (4h); daemon does not scrape disabled source |

### 4.4 Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Sync engine bugs corrupt remote data | Medium | Medium | Local copy is authoritative; remote can be fully republished from local outbox replay |
| Validator enforcement blocks good data | Medium | Medium | Dry-run calibration in L3; operator override available; false positive rate monitored via operator_override_rate_bps |
| Push batch size exceeds Supabase RPC limits | Low | Low | Batch size bounded by stage_run_id scope; single-in-flight batch constraint |
| Network interruptions during push | Medium | Low | Identical-envelope retries; degraded mode continues local operation; no data loss |
| Epoch mismatch causes permanent remote divergence | Medium | High | Epoch-repair reproject minimum path ships in L4 (see §5.2 stubs) |
| Single-device durability loss (machine dies before push) | Low | Critical | Encrypted WAL-aware snapshots; operational rule: one bound machine per org |
| Outbox growth during long offline windows | Medium | Medium | Coalesce superseded unsent updates by entity key where safe; monitor outbox depth |
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
- The minimum reproject path is required in L4 because epoch mismatch behavior otherwise creates permanent remote divergence.
- **L4 impact:** Minimum fix: after epoch repair, mark a `projection_generation`, republish current row images from local authoritative tables, discard stale-epoch outbox entries only after successful reseed.

**§10 Assumption #5: Company profile data can be centrally stored with appropriate protections**
- shared_company_profile in Supabase stores org-wide bidder data.
- Impact if wrong: bid workflows need stronger local-only secret partitioning.
- **L4 impact:** RLS policies must protect company profile; confirm data residency comfort level.

### 5.2 Stubs

| Feature | L4 Stub | Full Implementation |
|---------|---------|-------------------|
| Schema epoch transitions | No expand/drain/upgrade/contract *protocol* yet. Additive epoch bumps have since occurred — **current epoch is 11** (last bumped by the autonomous daemon scheduling migration `migrations/0015`, a local-only table; prior bump to 10 was the goods-led labour migration `migrations/0014`; see §1.7 forward note) — handled by the existing PRAGMA-backed guard | Future: expand/drain/upgrade/contract protocol |
| Multi-org | Single org hardcoded | Future: org_id scoping on all tables |
| Epoch-repair reproject | Minimum path: after epoch repair, mark `projection_generation`, republish current row images from local tables, discard stale-epoch outbox entries after successful reseed | Future: full generation-based rebuild with anti-entropy |
| Logical consistency auditor | Not implemented | Future: domain invariant checking with conditional auto-repair |

`projection_generations` is a local SQLite repair ledger created by `migrations/0012_l4_sync_state.sql`: `generation_id INTEGER PRIMARY KEY`, `schema_epoch INTEGER NOT NULL`, `status TEXT CHECK (status IN ('started','published','failed'))`, `started_at`, `completed_at`, `max_local_seq_before_reseed`, `remote_acked_at`, and `error_json`. Reseed batches carry `projection_generation = generation_id`; `viewer_sync_status.projection_generation` is updated only after the reseed succeeds.

**L4-FR-5.2.1 — Epoch-repair reproject minimum path is part of L4**
- AC1: After epoch repair, daemon marks a new `projection_generation`.
- AC2: Daemon inserts a `projection_generations` row with `status = 'started'` and `max_local_seq_before_reseed` before republishing.
- AC3: Daemon republishes current row images from local authoritative push-owned tables with the active `projection_generation`.
- AC4: Stale-epoch outbox entries are discarded only after the reseed succeeds and the repair row reaches `status = 'published'`.
- AC5: A test proves remote projection catches up after epoch mismatch without relying on manual full rebuild.

### 5.3 Forward-Compatibility

| Provision | What Exists in L4 | Target Layer |
|-----------|-------------------|-------------|
| schema_epoch on every batch | Per-batch epoch present, sync pauses on mismatch; epoch has since advanced to 11 via additive migrations (last bumped by local-only `migrations/0015`; prior step `0014`; see §1.7 forward note) | Future schema evolution protocol |
| RLS roles designed for multi-org | viewer/operator/admin/daemon roles | Future: org_id column for multi-tenancy |
| Projection tables | Denormalized push-owned mirrors | Future: materialized views for complex queries |

### 5.4 Dependencies

**Layer deps:** L0-L3 (all pipeline data, all push-owned tables with sync tracking columns)

#### External Accounts (L4)

| Account | Purpose |
|---------|---------|
| Supabase project | Shared sync target: Postgres database, Auth, Realtime (Broadcast), Storage |

#### Toolchain Prerequisites (L4)

| ID | Prerequisite | Purpose |
|----|--------------|---------|
| L4-FR-5.4.1 | Add daemon HTTP client dependency: `reqwest` with `rustls-tls` and `json` features | The daemon currently has no HTTP client in `crates/daemon/Cargo.toml`; L4 sync must call Supabase RPCs. |
| L4-FR-5.4.2 | Use `br` plus `bin/br-sync` for bead workflow handoffs | `AGENTS.md` contains an older duplicated `bd` block, but L4 pipeline metadata and current repo convention use `br`. |
| L4-FR-5.4.3 | Keep `packages/ui` dependency-clean | Root lint bans `@tauri-apps/api`, `apps/desktop`, `crates/daemon`, `supabase`, and `transport` imports from `packages/ui`; web data fetching stays in `apps/web`. |

**Acceptance criteria for L4-FR-5.4.1 through L4-FR-5.4.3**
- AC1: `crates/daemon/Cargo.toml` includes `reqwest = { ..., features = ["rustls-tls", "json"] }` or an explicitly approved equivalent Supabase HTTP client dependency.
- AC2: Implementation beads and handoff docs use `br`/`bin/br-sync` terminology, not `bd`.
- AC3: `bun run lint` fails if `packages/ui` imports web, desktop, daemon, Supabase, or transport-specific modules.
- AC4: `cargo check --all-targets` is the verification hook for the daemon HTTP-client dependency resolving under the pinned Rust toolchain.

#### Supabase Migrations

| Migration | Purpose |
|-----------|---------|
| `infra/supabase/migrations/0001_projection_tables.sql` | All 20 push-owned projection tables + 3 junction tables |
| `infra/supabase/migrations/0002_remote_owned_tables.sql` | `shared_company_profile`, `shared_source_configs`, `operator_decision_heads`, per-domain `remote_change_seq` sequences/triggers |
| `infra/supabase/migrations/0003_summary_tables.sql` | `workflow_job_summaries`, `error_manifest_summaries`, `validation_verdict_summaries`, `viewer_sync_status` including `projection_generation`, `executors` |
| `infra/supabase/migrations/0004_rls_policies.sql` | RLS policies per role (viewer, operator, admin, daemon) |
| `infra/supabase/migrations/0005_sync_rpcs.sql` | Transactional sync apply RPC (with `writer_epoch` validation) + operator decision CAS RPC (with command ledger) |
| `infra/supabase/migrations/0006_schema_metadata.sql` | Schema metadata table + initial schema_epoch + dedicated `active_writer_lease` table with `writer_epoch` |
| `infra/supabase/migrations/0007_command_ledger.sql` | `operator_decision_commands` table for durable CAS idempotency |
| `infra/supabase/migrations/0008_decision_summaries.sql` | `operator_decision_summaries` push-owned projection for web viewer access |

#### New Daemon Module

| Module | Purpose |
|--------|---------|
| `crates/daemon/src/db/sync.rs` | SQLite-to-Supabase push-pull sync engine |
| `crates/daemon/src/validation/calibration.rs` | Calibration corpus aggregation, Wilson/rule-of-three math, override ingestion, and enforcement-readiness inputs |

#### New Skills (4 Domain Validator Overlays)

| Skill | Purpose |
|-------|---------|
| `skills/validation/overlay-procurement-ingestion/` | OCID structure, date ranges, source verification |
| `skills/validation/overlay-supplier-enrichment/` | Pricing plausibility, currency, locale checks |
| `skills/validation/overlay-document-acquisition/` | Artifact type, content, completeness |
| `skills/validation/overlay-product-extraction/` | Candidate plausibility, slot consistency |

#### Web App

| Path | Files | Purpose |
|------|-------|---------|
| `apps/web` | 7 files | Next.js viewing portal: contracts list, contract detail, status page, realtime subscriptions, staleness store, topic map |

Root `package.json` must include `apps/web` in Bun build/test/lint scripts when this path is created; workspace coverage is already provided by the existing `apps/*` entry.

### 5.5 ADRs Addressed

| ADR | Decision | L4 Action |
|-----|----------|----------|
| **ADR-002** | Authority-matrix database | Sync/remote-projection half fully operational |
| **ADR-008** | Desktop = control, web = view | Web viewing portal created (read-only, viewer-scoped security boundary) |

<!-- CROSS-DOMAIN NOTE FOR L0: L4 owns the SQLite `local_outbox` DDL in `migrations/0012_l4_sync_state.sql`; the constitutional column shape (`commit_group_id`, `writer_epoch`) on `local_outbox` is reserved in L0 as a forward-compat note for when L4 first materializes the table. `commit_group_id` is text nullable and `writer_epoch` is integer nullable. The repo's L0 migration (`migrations/0001_sqlite_state_initial.sql`) does not pre-create `local_outbox`. `operator_decision_commands` remains a Supabase-only CAS-infrastructure table and therefore has no SQLite impact. -->

<!-- CROSS-DOMAIN NOTE FOR L2: Desktop degraded-mode UX in L4 adds explicit sync state on top of L2's control-plane model (healthy/remote-degraded/browse-only). L2's `ControlPlaneState` health dimensions should accommodate the sync health state from `viewer_sync_status` once sync is active. The FreshnessBadge component (L2 §2.1) should support the three-state sync indicator. -->

<!-- CROSS-DOMAIN NOTE FOR L2: Desktop operator decisions transition from local-only `pending_operator_intents` (L2) to remote CAS-backed `operator_decision_heads` (L4). Desktop's decision capture flow remains the same (write to `pending_operator_intents`), but L4 adds daemon-side CAS flush + conflict resolution UI. L2's decision capture should already carry `reviewedBasisHash`, `reviewedManifestHash`, `reviewedAtStageRunId` — these become the CAS payload in L4. -->

<!-- CROSS-DOMAIN NOTE FOR L6: Remote config staleness classes (L4 §1.8) include safety-critical TTL for source enablement. If bid workflow depends on source configs (e.g., checking source health before bid preparation), the safety-critical TTL model applies. Additionally, the upstream trust gate for bid workflow should be evaluated using the L3 mapping: no selected-scope `suspect` inputs and process-level `pipeline_assessments.validation_state = validated`, unless the operator explicitly accepts flagged/unverified trust debt. -->
