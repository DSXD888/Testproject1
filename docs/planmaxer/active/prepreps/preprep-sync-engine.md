# Pre-Prep: Sync Engine & Data Authority

## Step 1: Pre-Prep Research

Read the following documents in this exact order. Do not skip any.

1. **AGENTS.md** — project constraints, toolchain, rules
2. **The research document:** `docs/research/2026-03-08-public-procurement-intelligence-platform-research.md`
   - Read the ENTIRE document, but pay special attention to:
     - §4.7 (Database Architecture — authority matrix, push protocol with monotonic local_seq + row_version + content_hash, pull protocol with remote_change_seq + CAS, schema_epoch coordination, warm/cold recovery model, SQLite operational guidance, one daemon per org MVP)
     - §5.4 (Data Models — all SQL tables, sync tracking columns [local_seq, row_version, content_hash, remote_change_seq], outbox table, authority classification per table, junction tables, sync infrastructure tables)
     - §5.2 (Component Designs — Sync Engine component, SQLite Local Store component, Transactional Outbox)
     - §6.3 (Security — RLS policies per role [viewer, operator, admin, daemon], data protection, auth model, authority-based access control)
     - §6.4 (Performance — SQLite WAL mode, PRAGMA settings, index strategy, VACUUM policy, WAL checkpoint policy, Supabase connection pooling, push/pull batch sizing)
     - ADR-002 (Local SQLite as operational authority with Supabase sync)
     - §5.6 (Ingestion Data Flow — sync trigger points after pipeline stages)
     - §5.8 (Control Panel Data Flow — how desktop reads local SQLite, how web reads Supabase projections)
     - §9.4 (Migration Requirements — schema migration coordination, schema_epoch)
3. **The handoff prompt that produced the feedback:** `docs/planmaxer/active/handoffs/handoff-sync-engine.md`

After reading, you should be able to explain:
- The authority matrix: which tables are local-only, push-only (local authority → remote projection), pull-only (remote authority → local cache), and the prohibition on mixed-write tables
- The push protocol: transactional outbox (same txn as business write), batch envelope with monotonic local_seq, row_version for idempotency, content_hash for integrity, gap rejection on the remote side
- The pull protocol: remote_change_seq watermark, cursor-based fetch, CAS (compare-and-swap) for shared human decisions, config revision snapshots into jobs
- The recovery model: warm rebuild (Supabase healthy, local lost → pull shared + rerun pipeline) vs cold rebuild (Supabase corrupted → republish from local), and what's permanently lost in each scenario
- The schema coordination: schema_epoch in every push envelope, CI-enforced migration file pairing, Supabase migration runs first
- The SQLite operational model: WAL mode, PRAGMA settings, state.db + artifacts.db split, VACUUM policy, WAL checkpoint policy
- The RLS model: four roles (viewer, operator, admin, daemon), what each can see/modify, how authority classification maps to RLS policies
- The tombstone strategy: remote-owned records only, narrow scope, GC policy

**File Ownership:** This pre-prep owns the following sections for editing:
- §4.7 (Database Architecture) — FULL ownership
- §5.4 (Data Models) — Sync infrastructure tables (outbox, sync tracking columns, authority classification), SQL table definitions, junction tables ONLY. Do NOT modify TypeScript interface definitions (owned by pipeline-integration and other domains).
- §5.2 (Component Designs) — Sync Engine, SQLite Local Store, Transactional Outbox components ONLY
- §6.3 (Security) — RLS policies, data protection, authority-based access control ONLY
- §6.4 (Performance) — Sync-related performance: SQLite operational settings, push/pull batch sizing, Supabase connection pooling, index strategy ONLY
- ADR-002 — FULL ownership
- §9.4 (Migration Requirements) — FULL ownership

Do NOT edit: §5.3 (pipeline flow), §5.5 (state model), §5.6 (ingestion flow — except sync trigger points), §5.7 (intelligence pipeline), §5.8 (frontend data flows — except data source descriptions), §4.9 (daemon internals), §4.10 (skills), §5.11 (error types). The sync engine serves the pipeline and frontend, not the other way around. If the reviewer suggests changes to pipeline flow, skill execution, or frontend architecture, note them as deferred with `<!-- CROSS-DOMAIN: ... -->` comments.

---

## Step 2: Feedback Integration

The shape is right. The protocol edges are where it will quietly lie to you.

The real blockers are these:

1. `remote_change_seq` is unsafe unless remote-owned writes are serialized.
2. CAS writes need idempotency, not just revision checks.
3. `executor_id` is doing too many jobs at once.
4. The daemon pull role must be able to see tombstones.
5. “Full rebuild” cannot use the same equal-version rules as incremental apply.

If those stay fuzzy, you won’t get crashes. You’ll get “desktop says one thing, portal says another” six weeks later.

## Where it will silently diverge

### 1) `remote_change_seq` can skip real remote changes forever

If `remote_change_seq` is assigned from a normal Postgres sequence/identity value inside concurrent transactions, the pull cursor is broken.

Concrete failure:

* Local cursor is `909`.
* Remote tx T1 updates `operator_decision_heads(process_id = A)`, gets `remote_change_seq = 910`, then stalls before commit.
* Remote tx T2 updates `operator_decision_heads(process_id = B)`, gets `remote_change_seq = 911`, commits.
* Pull query runs: `WHERE remote_change_seq > 909 ORDER BY remote_change_seq`.
* Under Read Committed, that query sees rows committed before the query began, so it sees `911` but not `910`.
* Local applies `911` and advances cursor to `911`.
* T1 commits later.
* `910` is now behind the cursor forever.

That is a permanent silent divergence, not a transient race. Postgres sequences are not transactional in the way you need here: sequence changes are visible immediately and are not rolled back, while a Read Committed query only sees rows committed before the query started. ([PostgreSQL][1])

**What to do instead**

Keep the scalar cursor, but serialize *all* remote-owned mutations per pull domain. For MVP that is cheap because config and operator decisions are low-volume. The cleanest rule is:

* every decision mutation RPC acquires a transaction-scoped domain lock,
* then reads current row,
* then allocates `remote_change_seq`,
* then writes the head row,
* then commits.

Postgres transaction-level advisory locks are made for exactly this sort of application-defined serialization and are automatically released at transaction end. ([PostgreSQL][2])

Do **not** “fix” this with commit timestamps. That drags you back into clock semantics, and PostgreSQL explicitly says commit timestamp data is only useful when enabled and is routinely removed during vacuum. ([PostgreSQL][3])

Related nuance:

* `row_version` gaps are fine. `3 -> 5` is normal.
* `remote_change_seq` gaps are also fine on pull-owned *head tables*; multiple updates can collapse to the latest row.
* What is **not** fine is advancing the cursor past a lower committed change that was invisible only because commit order and seq order diverged.

---

### 2) Push correctness above SQLite is underspecified

SQLite solves producer atomicity, not dispatcher correctness.

The business-row + outbox-row atomicity is fine. In WAL mode, readers see snapshots and do not see partial transactions, so a crash does not create “business row committed but outbox row half-written” if both were in one SQLite transaction. Corruption and stale backups are different problems. ([sqlite.org][4])

The real risk is above that layer:

* batch selection,
* in-flight retry,
* ack handling,
* cursor advancement.

Concrete failure:

* Outbox has `4810..4820`.
* Dispatcher D1 builds batch `B-1234` for `4810..4820` and sends it.
* Remote commits the batch, but the HTTP response dies.
* While D1 is timing out, a second dispatcher or retry path re-reads outbox and builds `B-1235` for `4810..4825`.
* Remote may treat part of that as duplicate and part as new.
* Local may mark only `4810..4820` acked even though `4821..4825` already landed.
* Later retries “fix” it, but your local ack state, lag metrics, and alerting are now fiction.

With one active daemon per org, there is no upside to pipelining push windows. The safe contract is:

* exactly **one** in-flight push batch per push stream,
* retry resends the **identical persisted envelope** with the **same** `batch_id`,
* same `batch_id` + different body hash = hard protocol bug,
* local ack cursor only advances from a matching remote response for the current in-flight batch.

Also: your 60-second push deadline is not theoretical. Supabase’s client/dashboard query path has a max-configurable 60-second timeout; longer transactions need Supavisor or direct connections. So full rebuilds or giant stage batches should not go over the normal client RPC path unchanged. ([Supabase][5])

One more doc-level correction here: if the daemon is calling Supabase/PostgREST `/rpc`, `apply_push_batch` should be a **function**, not a stored procedure. PostgREST exposes functions under `/rpc`; stored procedures are not supported, and POST requests execute in a read/write transaction. ([PostgREST 14][6])

---

### 3) `schema_epoch` + backlog can wedge the stream

The current migration story is incomplete around a non-empty outbox.

Concrete failure:

* Old daemon keeps running during expand phase and appends outbox rows `700..1200` at epoch 3.
* Remote later moves to contract phase and drops epoch-3 compatibility.
* New daemon starts at epoch 4 and finds unsent epoch-3 rows in outbox.
* If it sends them as epoch 3, remote rejects them.
* If it re-encodes them as epoch 4 from current business rows, it can corrupt ordering and `row_version` semantics because seq `700` no longer corresponds to the original row image.

That backlog is now a wedge.

**Required rule**

* batch builder must **never** cross a `schema_epoch` boundary,
* remote contract phase cannot begin until `old_epoch_outbox_count = 0` for every active push stream,
* new binaries must retain N-1 projection encoders until backlog drains,
* retries must use the stored batch body/hash from write time, not recompute under new serialization rules.

This same issue applies to `content_hash`. If the canonical hash algorithm or JSON canonicalization changes with the new epoch and you recompute old unsent rows under new code, you will manufacture false “equal version, different hash” quarantines.

---

### 4) CAS without idempotency turns a success into a fake conflict

This is the easiest sync bug to miss because the data often ends up correct, but the operator sees nonsense.

Concrete failure:

* Remote `operator_decision_heads(process = X)` is at revision 5.
* Desktop sends `approve(base_revision = 5)`.
* Remote commits revision 6.
* Response is lost.
* Client retries `approve(base_revision = 5)`.
* Remote now sees current revision 6 and returns conflict.

The user now sees a conflict for a command that already succeeded.

So compare-and-swap is **not enough**. The mutation itself must be idempotent. Every decision command needs a `command_id` / idempotency key. Same key retried after a lost response must return “already applied” with the applied revision, not conflict.

The offline queue has a second correctness hole:

* operator clicks `reject`,
* then changes mind offline and clicks `approve`,
* queue replays both later in order,
* first succeeds, second conflicts,
* final shared state is `reject`, even though the operator’s final offline intent was `approve`.

That is user-intent loss.

**Required rule**

* queued intents are a **latest command per process**, not click history,
* keep audit history separately if you want it,
* on recovery: **pull remote-owned state first**, then classify/coalesce local pending intents, then replay,
* queued intents have explicit states: `queued`, `superseded`, `applied`, `conflicted`, `canceled`,
* pending local intents are UI overlay only; they are **not** authoritative input for automation.

If the daemon is offline for two hours, the UI needs to say three different things, not one vague “degraded” badge:

* local pipeline still running,
* shared decisions queued locally (`N`, oldest age),
* portal stale by `X`.

Otherwise operators will assume the web view is current when it is not.

---

### 5) “Full rebuild” as currently written cannot repair some corruption

Your normal incremental apply rule is:

* incoming `row_version > stored.row_version` => apply
* equal version + matching hash => replay
* equal version + differing hash => quarantine

That is correct for incremental push.

It is **not** enough for remote rebuild.

Concrete failure:

* Remote `pipeline_assessments(process = P42)` was manually corrupted or mangled by a bad migration.
* Remote row still says `row_version = 8`, but `content_hash = H_bad`.
* Local authoritative row is `row_version = 8`, `content_hash = H_good`.
* “Full rebuild” republishes the local row through the normal RPC.
* Remote sees equal version, different hash, and quarantines.

So the very rule that protects incremental correctness prevents rebuild repair.

**Required rule**

A full remote rebuild must not be “incremental apply against existing rows.” It must be one of:

* org-scoped truncate-then-republish,
* shadow generation / generation-swap,
* explicit force-rebuild mode that writes into a fresh projection generation.

Without that, “remote corrupted/lost → republish from local” is only half true.

There is a second recovery problem: if remote is restored from an old snapshot and loses acknowledged seq ranges, the local daemon must detect that remote generation regression and switch to full rebuild. Continuing incremental push from the local last-acked seq will silently strand remote rows that existed before the restore.

---

### 6) Your RLS wording currently breaks tombstone replication

This one is a straight doc bug.

§6.3 says all `SELECT` policies include `WHERE deleted_at IS NULL` for tombstoned tables. In Supabase/Postgres, RLS behaves like an implicit `WHERE` clause on every query. If the daemon’s pull role gets that same policy on `shared_source_configs` or `operator_decision_heads`, it will never see tombstones, which means deletes never propagate to local. ([Supabase][7])

Concrete failure:

* Remote tombstones a shared source config with `remote_change_seq = 501`.
* Viewer/operator selects should hide it. Good.
* Daemon pull query also hides it. Bad.
* Local never deletes its cached config.
* Tombstone later gets GC’d remotely.
* Local now carries a stale remote-owned row forever.

**Required rule**

* viewer/operator policies exclude tombstones,
* daemon pull policies **include** tombstones and `remote_change_seq`,
* or the daemon pulls from dedicated views/RPCs that expose deleted rows.

---

### 7) `executor_id` is overloaded and that will hurt you

Right now `executor_id` is trying to be:

* the stable owner of push-owned rows,
* the currently running daemon instance,
* the dedup/high-water stream identity.

Those are not the same thing.

That overload creates three failure modes:

1. **Split-brain**
   Laptop daemon and desktop daemon both push with different executor IDs. Remote dedup on `(executor_id, batch_id)` does nothing to stop both.

2. **Machine replacement**
   New install gets a new executor ID, but logically it is the same authority writer.

3. **Outbox reset / rebuild**
   If local outbox is lost but business tables are okay, you need a new stream identity, not necessarily a new row owner.

**What to do**

Split it:

* `owner_id`: stable logical authority for push-owned rows,
* `instance_id`: runtime process/device heartbeat identity,
* `push_stream_id` or `outbox_epoch`: dedup/high-water stream identity.

Then add a remote active-writer lease. Push RPC accepts writes only from the lease holder. A second daemon does not “mostly work”; it gets rejected and the losing desktop enters read-only / secondary mode.

Without that, “one active daemon per org” is a wish, not a protocol rule.

---

### 8) Cross-authority joins are safe for display only if you carry provenance

You are right that mixed-write tables are prohibited. But the *business object* is still mixed-authority.

A joined view of:

* `pipeline_assessments` (local push-owned),
* `operator_decision_heads` (remote pull-owned)

is not a single consistent fact unless you record *which remote revision the pipeline stage saw*.

Concrete failure:

* pricing stage runs using decision revision 5,
* remote operator changes decision to revision 6,
* local hasn’t pulled yet,
* pipeline pushes fresh assessment computed against rev 5,
* portal shows fresh assessment + rev 6 decision in one page.

That may be acceptable for display if clearly labeled. It is not acceptable as automation input.

**Required rule**

For any stage that consulted remote-owned state, store:

* `decision_revision_seen`,
* `config_seq_seen`,
* optionally `based_on_stage_run_id` on the decision side for operator provenance.

Then define three consistency classes:

* **display join**: okay with stale badges,
* **control join**: must show provenance mismatch,
* **automation input**: must pin revisions and requeue if they changed.

Also: on both desktop and portal, current-state detail views should join child rows by `current_stage_run_id`, not by `process_id` alone. Otherwise you will mix current and superseded history and call it a sync bug when it is really a read-model bug.

---

### 9) `quick_check` / `integrity_check` are nowhere near enough

You already know this, but the doc should say it harder.

SQLite’s `quick_check` explicitly skips UNIQUE and index/table consistency checks, and even `integrity_check` does not find foreign-key errors. Neither one tells you whether your stage heads, status rows, quotes, and assessments are semantically coherent. ([sqlite.org][8])

So add a **logical consistency auditor** with domain invariants like:

* no `process_stage_heads` row without matching `process_stage_status`,
* no successful stage head without matching expected output counts/hash,
* no `landed_cost_estimates` row pointing to a missing quote/candidate unless explicitly allowed,
* no remote projection row with owner mismatch,
* no queued intent whose `base_remote_revision` is older than current pulled revision without being marked `stale_base`,
* no assessment used for automation if `decision_revision_seen != current decision revision`.

And make auto-repair conditional:

* if local logical audit passes and remote parity fails => republish,
* if local logical audit fails => quarantine locally first; do **not** spread bad state faster.

---

## The push concurrency model you actually want

SQLite’s single-writer property gives you one committed `local_seq` order. That part is fine.

Above SQLite, the sync engine should look like this:

* many producers append outbox rows,
* one dispatcher selects the next contiguous unsent window,
* one persisted in-flight batch exists per push stream,
* ack handler advances only the contiguous ack cursor for that exact batch,
* new writes can append above the in-flight window,
* no second dispatcher, no overlapping windows, no optimistic re-slicing.

Anything more aggressive than that is buying complexity you do not need in MVP.

---

## Monitoring that catches silent divergence

Use the fields you already have.

1. **Parity audits by shard**
   For each push-owned table and hash shard (say `hash(entity_id) % 256`), compare:

   * row count,
   * max row_version,
   * aggregate hash over `(entity_id, row_version, content_hash)`.
     When a shard mismatches, drill down by process and republish that slice.

2. **Push stream truth metrics**

   * `oldest_unsent_age`
   * `unsent_rows`
   * `unsent_bytes`
   * `inflight_batch_age`
   * `batch_apply_p95`
   * `old_epoch_outbox_count`
   * `duplicate_batch_responses`
   * `equal_version_different_hash_count`

3. **Pull truth metrics**

   * decision/config cursor lag
   * `stale_replica_bootstrap_required_count`
   * tombstone rows pulled per day
   * intents in `conflicted` / `superseded` / `queued`
   * oldest queued intent age

4. **Cross-authority freshness**

   * assessments where `decision_revision_seen != current decision revision`
   * decisions whose `based_on_stage_run_id != current stage head`
   * portal staleness age vs desktop heartbeat

5. **Split-brain alarms**

   * push rejected due to lease mismatch
   * multiple active instances claiming same owner_id
   * remote rows with unexpected owner_id

6. **Logical audits**

   * stage head without status
   * successful stage without expected output counts/hash
   * dangling FK/domain references
   * current-state read models returning rows from non-head stage runs

Also watch SQLite WAL and checkpoint behavior. WAL lets readers and writers run concurrently, but readers hold snapshots, and passive checkpoints can remain unfinished while readers/writers are active. If your desktop keeps long-lived read transactions open, outbox/WAL growth will be your first warning sign, not a hard failure. ([sqlite.org][9])

---

## Testing I would add immediately

1. **Stateful model-based sync tests**
   Generate random sequences of:

   * local push-owned writes,
   * remote decision/config writes,
   * push attempts with outcomes `{commit+ack, commit+timeout, rollback+timeout}`,
   * daemon crash/restart,
   * second-daemon lease contention,
   * schema upgrade with non-empty outbox,
   * tombstone GC,
   * full bootstrap interruption.

   Check invariants after each step.

2. **Operator decision command tests**

   * lost response after successful CAS write returns idempotent success on retry,
   * offline `reject -> approve` coalesces to one replay command,
   * pull-first-then-replay on recovery.

3. **Migration tests with backlog**

   * epoch N backlog present,
   * upgrade daemon to N+1,
   * remote still accepting N,
   * drain backlog,
   * only then contract.
     Also test the forbidden case and assert the system refuses to continue silently.

4. **Remote restore generation tests**

   * remote snapshot restored to older projection generation/high-water,
   * local detects generation regression,
   * system flips to full rebuild instead of continuing incremental push.

5. **Tombstone/RLS tests**

   * viewer cannot see tombstoned rows,
   * daemon pull can,
   * delete actually removes local cache.

6. **Read-order assumption tests**
   Run SQLite sync tests with `PRAGMA reverse_unordered_selects=ON`. SQLite explicitly warns that unordered `SELECT` result order is not guaranteed, and this pragma helps expose code that accidentally depends on it. That is perfect for catching missing `ORDER BY local_seq` on outbox reads. ([sqlite.org][8])

7. **Stage boundary tests**
   Assert batch builder never cuts through a `stage_run_id` or `schema_epoch` boundary.

---

## Git-diff style changes

```diff
@@ §4.7 Pull Protocol
- Remote-owned tables carry monotonic remote_change_seq and remote_revision.
+ Scalar pull cursors are only valid if remote-owned mutations are serialized per pull domain.
+ Every operator-decision and shared-config mutation RPC MUST serialize writers for that domain
+ before reading current state, assigning remote_change_seq, and writing the new head row.
+ The daemon pull path MUST be able to read tombstoned rows.
+ Remote MUST expose min_incremental_seq per pull domain; if a replica cursor is below it,
+ incremental pull is rejected and full bootstrap is required.
```

```diff
@@ §4.7 Push Protocol
- Push unit: full-row projections batched by local_seq range.
+ Exactly one push batch may be in flight per push_stream_id.
+ Retries MUST resend the identical persisted batch envelope: same batch_id, same seq range,
+ same body_hash, same schema_epoch.
+ Batch builder MUST NOT cross a schema_epoch boundary or a successful stage_run_id boundary.
+ Duplicate batch_id + different body_hash is a hard protocol bug and MUST quarantine.
+ Local ack cursor advances only from a matching remote response for the current in-flight batch.
```

```diff
@@ §4.7 Recovery Model
- Full remote rebuild: republish current state of all local-owned tables + stage heads.
+ Full remote rebuild MUST use a new projection_generation (shadow generation / slice swap or
+ truncate-then-republish), not normal equal-row_version incremental apply semantics.
+ If remote projection_generation regresses or remote high-water is older than local's last
+ acknowledged stream position, local MUST enter full-rebuild mode instead of continuing
+ incremental push.
```

```diff
@@ §5.4 Data Model
- executor_id
+ owner_id           -- stable logical authority for push-owned rows
+ instance_id        -- runtime daemon/process identity for heartbeats
+ push_stream_id     -- dedup/high-water stream identity; changes on outbox reset

+ operator_decision_heads.last_command_id
+ pending_operator_intents.command_id
+ pending_operator_intents.superseded_by
+ pipeline_assessments.decision_revision_seen
+ pipeline_assessments.config_seq_seen
+ process_stage_status.expected_output_counts_json
+ process_stage_status.aggregate_hash
+ schema_metadata.projection_generation
+ schema_metadata.min_incremental_decision_seq
+ schema_metadata.min_incremental_config_seq
```

```diff
@@ §5.5 / §5.7 Cross-Authority State
+ Any pipeline stage whose behavior consulted remote-owned decisions/config MUST record the
+ revision(s) it saw.
+ Cross-authority joins are classified as display-only, control-surface, or automation-input.
+ Automation-input joins require pinned revision provenance and stale-input requeue rules.
```

```diff
@@ §5.6 Ingestion / Sync Recovery Order
+ On startup and after remote recovery:
+   1. pull remote-owned state,
+   2. mark queued local intents stale/superseded/conflicted as needed,
+   3. replay remaining intents,
+   4. resume schedulers.
+ Pending operator intents are coalesced to the latest unsent intent per process for replay;
+ full click history remains local audit only.
```

```diff
@@ §6.3 RLS and Sync Security
- All SELECT policies include WHERE deleted_at IS NULL for tombstoned tables.
+ Viewer/operator SELECT policies exclude tombstones.
+ Daemon pull policies (or pull RPC views) MUST include tombstones and remote_change_seq.
- admin: full shared config table writes
+ admin shared-config writes go through RPCs that enforce revision/sequence/idempotency rules;
+ direct table DML is maintenance-only and not part of normal ops.
- apply_push_batch via stored procedure
+ apply_push_batch and update_operator_decision are VOLATILE SQL/PLpgSQL functions exposed via /rpc.
```

```diff
@@ §6.4 Performance
+ Push batches on the normal Supabase client RPC path target p95 < 10s and hard-fail well below
+ the platform timeout cliff.
+ Full rebuilds use direct PG / Supavisor session connections or generation-sharded chunks.
+ Alert on oldest_unsent_age, inflight_batch_age, old_epoch_outbox_count, WAL_bytes,
+ checkpoint_lag, and pull-domain stale-bootstrap watermarks.
```

```diff
@@ §6.5 Testing
+ Add model-based tests for:
+   - commit-with-lost-ack push retries
+   - idempotent CAS retries
+   - split-brain lease rejection
+   - schema-upgrade with non-empty old-epoch outbox
+   - stale replica forced bootstrap
+   - remote projection generation regression
+   - tombstone visibility under RLS
+   - stage_run_id / schema_epoch batch-boundary preservation
+ Run SQLite sync tests with PRAGMA reverse_unordered_selects=ON.
```

If this were my handoff, I’d block implementation on blockers 1 through 5. Those are not polish items. Those are the places the system can become “quietly wrong.”

[1]: https://www.postgresql.org/docs/current/functions-sequence.html#:~:text=,in%20the%20CREATE%20SEQUENCE%20command "PostgreSQL: Documentation: 18: 9.17. Sequence Manipulation Functions"
[2]: https://www.postgresql.org/docs/current/explicit-locking.html "https://www.postgresql.org/docs/current/explicit-locking.html"
[3]: https://www.postgresql.org/docs/current/functions-info.html "https://www.postgresql.org/docs/current/functions-info.html"
[4]: https://sqlite.org/isolation.html "Isolation In SQLite"
[5]: https://supabase.com/docs/guides/database/postgres/timeouts#:~:text=Extend%20database%20timeouts%20to%20execute,can%20view%20it%20by%20executing "Timeouts | Supabase Docs"
[6]: https://docs.postgrest.org/en/latest/references/api/functions.html#:~:text=,Warning%0A%0A%0A%0AStored%20Procedures%20are%20not%20supported "Functions as RPC — PostgREST devel  documentation"
[7]: https://supabase.com/docs/guides/database/postgres/row-level-security "Row Level Security | Supabase Docs"
[8]: https://sqlite.org/pragma.html "https://sqlite.org/pragma.html"
[9]: https://sqlite.org/wal.html#:~:text=There%20are%20advantages%20and%20disadvantages,all%20databases%20as%20a%20set "Write-Ahead Logging"


For each change proposed by the reviewer, evaluate:

### Integration Rules

1. **For push protocol improvements:** Integrate into §4.7 (push protocol subsection) and §5.2 (Sync Engine component). If the reviewer proposes batching strategies, compression, retry policies, partial failure handling (5 of 10 rows pushed, then network drops), or ordering guarantees, integrate with concrete specifications. The push protocol is the most critical sync path — it must be robust and idempotent.

2. **For pull protocol improvements:** Integrate into §4.7 (pull protocol subsection). If the reviewer proposes change feed subscription (Supabase Realtime), conflict resolution refinements, CAS retry logic, or pull frequency tuning, integrate while preserving remote_change_seq as the watermark mechanism. Pull is simpler than push for MVP (shared config + operator decisions only).

3. **For authority matrix refinements:** Integrate into §4.7 and §5.4 (table authority classifications). If the reviewer proposes reclassifying tables, splitting tables by authority, or adding new authority categories, integrate while preserving the mixed-write prohibition. Every table must have exactly one write authority.

4. **For recovery model improvements:** Integrate into §4.7 (recovery subsection). If the reviewer proposes recovery runbooks, automated recovery detection, integrity verification on startup, or backup strategies, integrate while being honest about what's permanently lost. Do not claim recovery capabilities that don't exist.

5. **For RLS policy refinements:** Integrate into §6.3. If the reviewer proposes row-level filters (e.g., viewer sees only processes above a certain stage), column-level restrictions (e.g., viewer cannot see cost data), or service-role patterns for the daemon, integrate with concrete policy definitions. Every user-facing table in Supabase must have an RLS policy.

6. **For SQLite operational improvements:** Integrate into §6.4 and §4.7. If the reviewer proposes specific PRAGMA values, index definitions, VACUUM scheduling, WAL checkpoint thresholds, or database size monitoring, integrate as concrete operational guidance. SQLite configuration is critical for daemon reliability.

7. **For schema migration coordination improvements:** Integrate into §9.4. If the reviewer proposes migration tooling (e.g., specific Rust migration crate), versioning strategies, rollback procedures, or data migration patterns, integrate while preserving schema_epoch as the coordination mechanism.

8. **For outbox table improvements:** Integrate into §5.4 (outbox table definition) and §5.2 (Transactional Outbox component). If the reviewer proposes outbox compaction, TTL for processed entries, monitoring for outbox growth, or multi-table outbox patterns, integrate with concrete specifications.

9. **For changes touching pipeline flow, skill execution, frontend architecture, or error taxonomy:** Do NOT integrate directly. Add `<!-- CROSS-DOMAIN: Reviewer suggested [X] for [component]. Needs coordination with [domain]. -->` notes.

### Evaluation Framework

For each proposed change, assess:
- **Agree strongly:** Improves sync reliability, reduces data loss risk, or makes the authority model more precise. Apply directly.
- **Agree partially:** Good pattern but may over-engineer for MVP (one daemon per org, single-operator simplification). Apply scoped to MVP constraints, noting future expansion path.
- **Disagree:** Conflicts with settled decisions (SQLite as operational authority, authority matrix model, mixed-write prohibition, transactional outbox pattern, one daemon per org MVP). Note disagreement with rationale.

### Preservation Rules
- Preserve SQLite as operational authority and Supabase as shared authority + projections
- Preserve the mixed-write prohibition (every table has exactly one write authority)
- Preserve the transactional outbox pattern (outbox write in same txn as business write)
- Preserve monotonic local_seq for push ordering
- Preserve row_version for idempotent push application
- Preserve remote_change_seq for pull watermarking
- Preserve CAS for shared human decisions
- Preserve schema_epoch for migration coordination
- Preserve the one-daemon-per-org MVP simplification
- Preserve the state.db + artifacts.db split
- Preserve integer pence for synced monetary values
- Use "as-if-always" language

---

## Step 3: Fresh-Eyes Optimization

After completing Step 2, re-read the sections you edited with fresh eyes. Check for:

1. **§4.7 push protocol ↔ §5.4 outbox schema consistency:** Does the push protocol described in §4.7 (batch envelope, local_seq, row_version, content_hash) match the outbox table definition in §5.4? Are all fields referenced in the protocol present in the schema? Does the outbox entry format match what the Sync Engine component (§5.2) produces?

2. **§6.3 RLS policies ↔ §5.4 table authority consistency:** Does every push-only table (local authority, projected to Supabase) have an RLS policy in §6.3 that restricts web users to read-only? Does every pull-only table (remote authority) have an RLS policy that allows appropriate write access? Are there any tables without an RLS classification?

3. **§4.7 recovery model ↔ §6.4 operational guidance consistency:** Does the recovery model in §4.7 (warm/cold rebuild) reference the backup and checkpoint mechanisms described in §6.4? Are the SQLite operational settings (WAL mode, VACUUM) consistent with the recovery assumptions?

4. **§5.4 per-table authority ↔ ADR-002 consistency:** Does every table's authority classification in §5.4 align with the principles stated in ADR-002? Are there any tables that seem to violate the mixed-write prohibition?

5. **§9.4 migration requirements ↔ §4.7 schema_epoch consistency:** Does the migration strategy in §9.4 use schema_epoch as described in §4.7? Are the CI enforcement rules described in both places consistent?

6. **Sync trigger points consistency:** Do the sync trigger points in §5.6 (after pipeline stages) match what the push protocol in §4.7 expects? Are there pipeline stages that produce data requiring sync but don't have a trigger point?

7. **Terminology:** Ensure consistent use of "authority" (not "ownership" or "source of truth" when referring to the write authority model), "push" (not "sync" or "replicate" when referring to local→remote), "pull" (not "fetch" when referring to remote→local), "outbox" (not "queue" or "buffer" when referring to the transactional outbox), "projection" (not "replica" when referring to Supabase copies of local-authority data).

Fix any inconsistencies found. Make targeted edits, not wholesale rewrites.

---

## Step 4: Sync Pass

1. **Update §1 (Executive Summary):** If the sync architecture changed materially (new protocol mechanisms, changed authority classifications, altered recovery model), update the executive summary to reflect the current state of the data architecture.

2. **Update Appendix E (Changelog):** Add a concise entry describing what was integrated in this review cycle. Include: which sections were modified, what the reviewer's key contributions were, and any deferred cross-domain items.

3. **Verify ADR-002:** Ensure the ADR rationale still matches the implementation described in §4.7 and §5.4. If the review cycle added nuance (e.g., new authority categories, refined recovery guarantees), update the ADR.

4. **Check §10 (Open Questions):** If any open questions about sync, database architecture, or schema migration have been resolved by this review cycle, mark them as resolved. If new questions emerged, add them.

5. **Verify §8.2 file references:** If the reviewer proposed new sync-related source files (sync engine modules, migration files, outbox processors), ensure they appear in §8.2 with correct milestone assignments.

6. **Cross-reference verification:** Every section that references sync behavior (§5.6 sync triggers, §5.8 data sources, §6.3 RLS policies) should cross-reference §4.7 for the protocol specification. Verify these cross-references are present and accurate.
