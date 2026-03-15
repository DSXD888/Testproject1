# Handoff: Sync Engine & Authority-Matrix Correctness

## Expert Framing

You are a **distributed systems engineer** who has built replication and sync engines between local-first databases and cloud backends. You have practical experience with CRDT-based systems, transactional outbox patterns, event sourcing, and the operational reality of keeping two databases in sync when one or both may be unavailable. You have debugged production sync failures at companies running offline-first architectures — systems like Replicache, PowerSync, Electric SQL, or custom SQLite-to-Postgres replication layers.

You understand that sync bugs are the hardest category of bugs to debug because they manifest as **silent data divergence**, not crashes. A user opens the web portal and sees stale data — but nothing errored. A daemon pushes a batch that the remote silently deduplicates — but the dedup logic had an edge case. An operator makes a decision that gets lost because the CAS retry window expired. These bugs don't page anyone; they erode trust in the system over weeks until someone notices the numbers don't add up.

Draw from: the literature on monotonic sequence-based replication, compare-and-swap coordination for shared decisions, transactional outbox patterns in single-writer databases, SQLite WAL mode concurrency characteristics, Postgres stored procedure atomicity guarantees, and the operational failure modes of systems where "eventually consistent" meets "commercially critical human decisions."

---

## Mission

ULTRATHINK and be meticulous.

The procurement intelligence platform uses a dual-database architecture: **local SQLite** (operational authority for all pipeline data) and **Supabase/Postgres** (shared authority for operator decisions and company config, plus push-only projections of pipeline results for viewing-portal visibility).

The sync layer connects them. Push: local to remote via transactional outbox with monotonic `local_seq`, `row_version`, `content_hash`, and `batch_id` deduplication. Pull: remote to local via cursor-based `remote_change_seq` with compare-and-swap for operator decisions.

The plan explicitly calls the sync layer **"the highest-risk component for data integrity"** (verbatim, in two separate locations: the authority-matrix introduction in section 4.7 and ADR-002). This is the correct assessment. A bug in normalization corrupts one pipeline stage; a bug in sync corrupts every downstream consumer of every table.

Your job: **find the sync bugs before they happen.** Where will these two databases silently diverge? What are the invariants that must hold, and what are the concrete scenarios where they break?

**Don't just answer my questions -- the questions I DON'T KNOW to ask are more important than the ones I do.**

---

## User's Strong Beliefs (Tensions to Engage With)

### 1. "Mixed-write tables are prohibited -- every table has one authority."

Clean rule. The authority matrix in section 4.7 maps every table to exactly one of: local-only, push-only, pull-only. No mixed-write tables exist. This is good.

But what about **derived data that depends on BOTH authorities?** The most obvious case: a desktop UI view that shows a procurement process with its pipeline assessment (push-owned, daemon-authored) AND its operator disposition (pull-owned, remote-authored). The UI joins these two tables. But the pipeline assessment was computed at time T1, and the operator disposition was set at time T2, and they were synced at different times via different protocols. Is the joined view ever inconsistent? Can the operator see a disposition that references a pipeline state that hasn't been pushed yet? Can the daemon see a pipeline assessment that was computed before an operator decision that should have changed the pipeline's behavior?

### 2. "Convergence via local_seq, row_version, remote_change_seq -- never clock-based."

The plan correctly avoids wall-clock conflict resolution. Monotonic sequences are better. But monotonic sequences have their own edge cases:

- **local_seq recovery:** `local_seq` is a monotonic counter per executor. What happens when the daemon process crashes and restarts? Is `local_seq` persisted in SQLite? If so, what happens if the outbox table is corrupted but the business tables are fine? The counter could be recovered from `MAX(local_seq)` in the outbox, but what if outbox entries were partially written?
- **row_version gaps:** `row_version` is per-row and monotonic. If a row is updated twice in rapid succession before sync, the remote sees version jump from 3 to 5. Is this a gap that triggers quarantine, or is it expected? The plan says "apply only if incoming row_version > stored row_version" -- does a jump from 3 to 5 satisfy this?
- **remote_change_seq ordering:** The pull protocol fetches changes where `remote_change_seq > cursor`. But `remote_change_seq` is assigned by Postgres. If two transactions commit concurrently, their sequence numbers may not be visible in commit order due to MVCC. Can the pull cursor advance past a sequence number that hasn't become visible yet?

### 3. "MVP constraint: one active daemon per org."

This simplifies sync enormously -- no need for multi-executor coordination, source leases, or primary election. But what are the **exact failure modes when this constraint is accidentally violated?**

- User installs daemon on laptop AND desktop, forgets to stop one
- User reinstalls OS, gets a new `executor_id`, old daemon is still running on a backup disk image
- Docker container restart creates a second instance briefly overlapping with the first

The plan mentions `executor_id` but doesn't specify how uniqueness is enforced. If two executors push to the same remote with different `executor_id` values, does the remote accept both? The outbox deduplication is keyed on `(executor_id, batch_id)` -- two different executors can push overlapping data without triggering dedup. What happens?

---

## Essential Context

### What the System Is

A UK public procurement intelligence platform. A local Rust daemon scrapes procurement notices via browser automation (agent skills), enriches them through a multi-stage pipeline (qualification, product extraction, supplier pricing, landed cost), and surfaces high-margin opportunities to operators. The desktop app reads local SQLite for fast, offline-capable access. A web viewing portal reads Supabase/Postgres for multi-user visibility.

### The Authority Matrix

The dual-database architecture uses a **domain-owned authority model**:

- **Local operational authority** (SQLite): Pipeline execution state, job leasing, source cursors, full raw session-output artifacts, full error context, raw documents. The daemon that ran the job is authoritative for its outputs.
- **Remote shared authority** (Supabase/Postgres): Shared configuration (company profile, source configs) and shared human decisions (operator dispositions). These originate remotely and are pulled to local.
- **Remote projections** (Supabase/Postgres): Push-only copies of pipeline facts and status. Never authoritative -- the local copy can always republish.

**Per-table authority mapping** (from section 4.7):

| Authority | Example Tables | Sync Direction |
|-----------|---------------|----------------|
| Local-only | `raw_documents`, `source_cursors`, `listing_evidence`, `pending_operator_intents`, `workflow_jobs` (full), `agent_session_logs` (full), `error_manifests` (full), `validation_verdicts` (full) | None |
| Push-only | `procurement_processes`, `contract_notices`, `notice_lots`, `qualification_runs`, `product_candidates`, `search_profiles`, `supplier_quotes`, `landed_cost_estimates`, `manifests`, `pipeline_assessments`, `bid_workflow_states`, `process_stage_status`, `process_stage_heads`, `workflow_job_summaries`, `error_manifest_summaries`, `validation_verdict_summaries` | Local to remote |
| Pull-only | `shared_company_profile`, `shared_source_configs`, `operator_decision_heads` | Remote to local |

---

## Current Architecture

### Push Protocol (Local to Remote) -- Section 4.7

Every local mutation to a push-owned entity writes to both business tables and a **local outbox** in the same SQLite transaction. This is the fundamental atomicity guarantee -- crashes cannot create ghost writes or missing replications.

- **Push unit:** Full-row projections (not field patches), batched by `local_seq` range
- **Ordering:** Monotonic `local_seq` per executor. Remote rejects gaps -- if remote has seq 4810 and receives 4812 first, it fails closed until 4811 arrives
- **Idempotency:** Remote deduplicates by `(executor_id, batch_id)` and `(executor_id, local_seq)`. Replays are harmless
- **Row versioning:** For mutable push-owned rows, apply only if incoming `row_version > stored row_version` for the same owner. Equal version with matching `content_hash` = replay (ignore). Equal version with differing hash = quarantined as protocol bug
- **Atomicity:** Remote applies each push batch through a single transactional RPC / stored procedure
- **Stage coherence:** All outputs from a pipeline stage share a `stage_run_id`. The stage's `process_stage_status` is updated and `process_stage_heads` is advanced in the same transaction. Failed stages publish failure status but do NOT advance the head
- **Schema compatibility:** Every push batch carries a `schema_epoch`. Incompatible epochs pause sync but local operation continues
- **Delivery timeout:** 60-second ack/fail deadline. After 3 consecutive failures or 15 minutes pending, daemon marks sync `degraded` and surfaces operator alert

### Pull Protocol (Remote to Local) -- Section 4.7

- Applies only to remote-owned domains (shared config, operator decisions)
- Remote-owned tables carry monotonic `remote_change_seq` and `remote_revision`
- Local stores per-domain pull cursors (`last_pulled_config_seq`, `last_pulled_decision_seq`)
- Pull fetches `remote_change_seq > cursor`, ordered by seq, in bounded batches
- Apply locally in order, transactionally, advance cursor only after successful commit
- **Operator decisions:** Compare-and-swap -- update includes `base_remote_revision`, succeeds only if current revision matches. On mismatch, return conflict for manual reconciliation. Last-write-wins is NOT used for human commercial decisions

### Outbox Design -- Sections 4.7, 5.6

- Business write + outbox entry in the same SQLite transaction
- Outbox entries carry: `local_seq`, `entity_type`, `entity_id`, `row_version`, `content_hash`, `stage_run_id`, `batch_id` (assigned at push time), `schema_epoch`
- Stage sync (step 17 of ingestion flow): In the same transaction that writes stage outputs, the daemon writes all rows with a shared `stage_run_id`, updates `process_stage_status`, advances `process_stage_heads`, and writes outbox entries

### Stage Coherence -- Section 5.6

- All outputs from a single pipeline stage share a `stage_run_id`
- `process_stage_status` tracks per-stage success/failure
- `process_stage_heads` records the latest successful stage per process
- Both are updated atomically with stage outputs in the same SQLite transaction
- On push, these go as a single batch -- viewers see atomic stage transitions

### Recovery Model -- Section 4.7

- **Remote corrupted/lost:** Daemon republishes all push-owned projections from local. Two modes: incremental (outbox replay) and full remote rebuild (republish all current state)
- **Warm rebuild (local damaged):** Disk still available. `PRAGMA quick_check` to assess, restore from hot backup, repull remote-owned state, republish projections
- **Cold rebuild (machine loss):** Full rebuild from Supabase projections + off-box artifact backups. Permanently lost: full raw session-output artifacts, full error context, source cursors (re-ingest from scratch), local job leases, raw documents

### Tombstone Policy -- Section 4.7

- Tombstones (`deleted_at`) only for entities with real shared delete semantics: shared config, operator decisions, viewer-facing projections where delete must propagate
- High-volume pipeline history is append-only and superseded by stage head pointers, not tombstoned
- GC safe only when: every active replica has pulled past the delete's `remote_change_seq` AND tombstone is older than `max_offline_window + safety_margin` (30 + 7 = 37 days default)
- Replicas offline past the window become stale and must full-bootstrap

### Schema Coordination -- Section 5.4

- Canonical logical schema generates both SQLite and Postgres DDL
- `schema_epoch` stored in SQLite `PRAGMA user_version`, Postgres `schema_metadata` table, and every sync batch envelope
- Incompatible epochs on startup pause sync, keep local operation
- Migrations use expand/migrate/contract rollouts only -- no one-step destructive hot migrations
- CI: apply both migrations from scratch, introspect both, run sync round-trip test, fail if logical schema hash differs

### RLS and Sync Security -- Section 6.3

- Four roles: `viewer` (SELECT on projections), `operator` (SELECT + CAS write on decisions), `admin` (full shared config), `daemon` (push via batch RPC, pull remote-owned)
- Push batches applied through `apply_push_batch(batch_json)` RPC that validates `schema_epoch`, checks `row_version` ordering, applies atomically
- Operator decisions through `update_operator_decision(process_id, disposition, base_revision)` CAS RPC
- All `SELECT` policies include `WHERE deleted_at IS NULL` for tombstoned tables

### Pending Operator Intents -- Section 4.7 (Authority Split table)

- `pending_operator_intents` is a local-only table
- When Supabase is unavailable, operator decisions from the desktop app queue locally
- On sync recovery, these are applied to remote via the CAS RPC

---

## Tensions to Resolve

### Tension 1: Outbox Ordering Under Concurrent Pipeline Stages

**The conflict:** Multiple pipeline stages can run concurrently for different procurement processes. For example, Process A is in the pricing stage while Process B is in the qualification stage. Both write to the outbox. The remote requires monotonic `local_seq` with no gaps -- "if remote has seq 4810 and receives 4812 first, it fails closed until 4811 arrives."

**Current resolution:** Outbox entries are written in the same SQLite transaction as business writes. SQLite is single-writer, so concurrent stages serialize at the SQLite write lock. `local_seq` is assigned within the transaction, so ordering is guaranteed.

**Why this might still be wrong:** SQLite single-writer serialization guarantees no interleaving within a transaction, but what about the batching step? The daemon reads outbox entries (seq 100-110), groups them into a batch, and pushes. While that push is in flight, another stage commits seq 111-115. If the first push fails and is retried, does the daemon retry 100-110 or does it send 100-115? If it retries 100-110 and succeeds, then sends 111-115, that's fine. But if the retry logic re-reads the outbox and now sends 100-115 as a single batch with a new `batch_id`, the remote may have partially applied the original batch (despite the single-transaction RPC -- see Tension 3).

**Stress test:** Stage A commits outbox entries 4810-4815 (qualification results for Process X). Stage B commits 4816-4820 (pricing results for Process Y). The daemon batches 4810-4820 and pushes. Push times out at 60 seconds. Daemon retries. What `batch_id` does the retry use? What if only 4810-4815 were applied before the timeout? What is the exact retry contract?

### Tension 2: Schema Epoch Migration Under Accumulated Outbox

**The conflict:** When a schema change is deployed, both the daemon and Supabase need to coordinate. The plan says "sync pauses but local operation continues" when `schema_epoch` values are incompatible. This is correct for preventing corrupted pushes. But what happens to the outbox during the pause?

**Current resolution:** Local operation continues, writing to business tables and the outbox. When sync resumes after the schema migration completes on both sides, the accumulated outbox entries are pushed.

**Why this might be wrong:** The outbox entries written during the pause carry the OLD `schema_epoch` (the daemon was still running the old code). When sync resumes, the daemon is now running the NEW code and the remote expects the NEW epoch. Do the old outbox entries get rejected because their epoch doesn't match? Or does the daemon need to "re-project" old business rows into new outbox entries? The plan specifies expand/migrate/contract rollouts, which means the remote schema is expanded first (new columns nullable) before the daemon is updated. But the outbox entries from the old daemon won't include the new columns.

**Stress test:** The daemon has been running for 6 hours during a schema migration window. The outbox has accumulated 500 entries at epoch 3. The migration completes, remote is now epoch 4. The daemon restarts with new code (epoch 4). It reads 500 outbox entries at epoch 3. What happens? Does it push them with epoch 3 headers? Does the remote accept epoch 3 during the expand phase? What if the remote has already moved to the contract phase and dropped the old columns?

### Tension 3: Partial Push Batch Failure and Ack Loss

**The conflict:** A push batch contains N entity upserts applied through a single transactional RPC on the remote. If the Postgres transaction commits but the acknowledgment is lost (network partition, daemon process crash after sending), the daemon retries. The remote deduplicates by `(executor_id, batch_id)`.

**Current resolution:** Idempotent replay via `batch_id` deduplication.

**Why this might be wrong:** The deduplication assumes the remote either fully applied the batch or did not apply it at all (transactional atomicity). This is true for Postgres transactions -- if the transaction commits, all rows are applied; if it rolls back, none are. The risk is not partial application within Postgres, but rather: what if the Postgres transaction committed, the RPC function returned success, but the **Supabase edge function / PostgREST layer** between the daemon and Postgres lost the response? The daemon sees a timeout, not a success. It retries with the same `batch_id`. The remote sees the `batch_id` already applied and returns... what? Success? A dedup signal? An error? The plan doesn't specify the dedup response contract.

**Deeper concern:** The dedup table (`batch_id` log) itself needs maintenance. If it grows unbounded, it becomes a performance problem. If it's pruned too aggressively, a very late retry could fail to dedup and cause double-application. What's the retention policy for the dedup log?

**Stress test:** The daemon pushes batch B-1234 containing 50 entity upserts. The Postgres transaction commits. The HTTP response is lost. The daemon retries batch B-1234. The remote dedup table has B-1234 recorded. The remote returns "already applied." The daemon marks the outbox entries as synced. But what if between the original commit and the retry, another push batch B-1235 was sent (from a concurrent stage) and also committed? The daemon marks B-1234 as synced but hasn't yet confirmed B-1235. Is the outbox cursor now correct?

### Tension 4: Operator Intent Queue Conflicts Under Extended Outage

**The conflict:** When Supabase is unavailable, operator decisions from the desktop app queue in `pending_operator_intents` (local-only table). When sync recovers, these are applied via the compare-and-swap RPC. The plan says "on mismatch, return conflict for manual reconciliation."

**Current resolution:** CAS with `base_remote_revision`. Each pending intent carries the `remote_revision` it was based on. On recovery, the intent is applied only if the remote revision hasn't changed.

**Why this might be wrong:** Consider the multi-desktop scenario (even though it's edge-case in MVP). Desktop A and Desktop B both have pull cursor at revision 5 for Process X's operator decision. Supabase goes down. Desktop A queues "reject Process X" (base_revision=5). Desktop B queues "approve Process X" (base_revision=5). Supabase comes back. Desktop A's daemon recovers first, applies the reject (revision 5 -> 6). Desktop B's daemon recovers, tries to apply approve with base_revision=5, gets a CAS conflict because current revision is now 6.

The plan says "return conflict for manual reconciliation." But what does "manual reconciliation" look like? Does the operator see a dialog? Does the pending intent remain in the queue indefinitely? What if the operator doesn't notice? Is there a staleness timeout for pending intents?

**Even without multi-desktop:** A single operator could queue a decision, then change their mind and queue another decision for the same process while offline. Both are in `pending_operator_intents`. Are they applied in order? Does the second one supersede the first?

**Stress test:** Supabase is down for 2 hours. The operator makes 15 decisions across 10 processes, including changing their mind on 3 of them (reject then approve). Sync recovers. What is the replay order? What if some succeed and some CAS-conflict? Is the operator shown a batch reconciliation view, or do they get 15 individual alerts?

### Tension 5: Recovery Confidence -- Logical Corruption vs. Physical Corruption

**The conflict:** The plan describes warm and cold rebuild paths. The warm path runs `PRAGMA quick_check` to detect corruption. This detects physical corruption -- damaged pages, broken b-tree nodes, invalid free lists. But SQLite corruption can be **logical** -- a bug writes syntactically valid but semantically wrong data. `quick_check` passes. The data looks fine structurally. But `pipeline_assessment` for Process X says "net_qualified" while the actual `landed_cost_estimate` it depends on was never written (the transaction that wrote the assessment committed but the one that should have written the cost estimate was in a different stage that failed silently).

**Current resolution:** `PRAGMA quick_check` on startup, `PRAGMA integrity_check` on maintenance windows.

**Why this might be wrong:** Physical corruption detection is necessary but not sufficient. The more dangerous corruption vector in this system is **logical inconsistency between related tables** -- especially between stage outputs and stage heads. The `process_stage_heads` table says "Process X completed pricing" but the `supplier_quotes` table has no quotes for Process X. This can happen if the atomic transaction that was supposed to write both was split by an application bug, not a SQLite bug.

**Stress test:** A bug in the stage sync code (step 17 of ingestion flow) causes `process_stage_heads` to advance without writing the corresponding `process_stage_status` entry. This is pushed to Supabase. The web portal shows Process X as "pricing complete" but the detail view shows no quotes. `PRAGMA quick_check` passes on both databases. `PRAGMA integrity_check` passes. The data is structurally sound but logically wrong. How is this detected? How is it repaired?

---

## What I Don't Want (Anti-Patterns)

- **Don't propose CRDTs or a different replication paradigm.** The push/pull authority model with outbox and CAS is the decided architecture. Work within it.
- **Don't suggest moving to a single database.** The dual-database authority-matrix architecture is a core decision (ADR-002). The local-first corruption resilience property is non-negotiable.
- **Don't propose multi-leader replication for MVP.** One daemon per org is the constraint. The sync protocol is deliberately simplified around this assumption.
- **Don't suggest replacing SQLite with another embedded database.** SQLite is the local store. Period.
- **Don't suggest adding a message queue (Kafka, RabbitMQ, etc.) for sync.** The outbox pattern with direct RPC push is the design. Keep it simple.

---

## What I Do Want

1. **Specific sync bug scenarios with concrete data examples.** Not "sync might fail" -- rather "Entity X in state Y on local, state Z on remote, sequence numbers A and B, here's exactly what the daemon does and here's the divergence." Walk through the push/pull code paths step by step.

2. **Analysis of the outbox ordering guarantee under concurrent writers.** SQLite single-writer serializes, but the batching, push, retry, and cursor-advance logic above the SQLite layer has concurrency. Map out the exact concurrency model of the sync engine above the database.

3. **Proposals for sync health monitoring beyond simple success/failure counts.** The plan has delivery timeout (60s) and degraded mode (3 failures / 15 minutes). But what about detecting silent divergence? Consistency audits? Drift detection between local and remote?

4. **Specific edge cases in the tombstone GC policy.** The 37-day window is a starting point. What about replicas that come back after exactly 37 days? What about the interaction between tombstone GC and full-bootstrap? What if a full-bootstrap is interrupted partway through?

5. **Ideas for testing the sync engine.** Property-based testing (e.g., QuickCheck/proptest for Rust) to generate random sequences of push/pull/crash/recover and verify convergence. Chaos engineering: inject network partitions, process crashes, slow responses. Round-trip fuzz testing: generate random business writes, push, pull back, verify equality.

6. **Ideas I haven't asked about.** Especially around the interaction between sync degradation and operator UX (what does the operator see when sync is degraded for 2 hours?), the interaction between outbox growth and SQLite performance, the interaction between `schema_epoch` coordination and CI/CD deployment sequencing, and any invariants that should be formally specified and continuously verified.

7. **Recommendations in git-diff style** -- reference specific doc sections (4.7, 5.4, 5.5, 5.6, 6.3, 6.4) when proposing changes or additions.

---

## Context Documents to Attach

When sending this to a reviewer, attach:

- **Full research document:** `docs/research/2026-03-08-public-procurement-intelligence-platform-research.md`
- **Pay special attention to:** Section 4.7 (Database Architecture, Authority Matrix, Push/Pull Protocols, Recovery, Tombstones), Section 5.4 (Data Model, Change Tracking Columns, Schema Coordination), Section 5.5 (Contract State Model with Split Authority), Section 5.6 (Ingestion Data Flow -- especially step 17 stage sync), Section 6.3 (Security -- RLS, Sync Security), Section 6.4 (Performance -- SQLite/Supabase operational guidance, bottleneck mitigations)
