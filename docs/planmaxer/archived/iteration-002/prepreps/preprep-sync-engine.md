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

<INSERT REVIEWER RESPONSE HERE>

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
