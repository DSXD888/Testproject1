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

---

## Step 2: Feedback Integration

<INSERT REVIEWER RESPONSE HERE>

### Integration Rules

For each proposed change from the reviewer:

1. **Agree strongly** — Integrate directly.
2. **Partially agree** — Integrate core insight, adapt to conventions.
3. **Disagree** — Do NOT integrate. Note why.

### Integration Checklist

For each change you make:
- [ ] Preserve ADR-002 numbering
- [ ] If modifying SQL tables, provide BOTH Postgres syntax (for Supabase) AND note SQLite adaptations needed
- [ ] If adding new columns to existing tables, ensure they're compatible with BOTH databases
- [ ] If modifying the sync protocol, update ALL places it's referenced (4.7, 5.2 Sync Engine component, 5.4 Sync Authority section, 5.6 sync step, 5.8 data flow)
- [ ] If adding new sync-related tables, include RLS considerations
- [ ] If modifying the recovery model, validate it's actually feasible given what each database stores
- [ ] Maintain the redaction boundary (what goes to Supabase vs. stays local)
- [ ] Update the Change Log in Appendix E

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
