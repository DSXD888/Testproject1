# L4 Shared Visibility — 01 · Validate Existing Spec for 3-SOT Pipeline

## First Action

**No skill is invoked.** This is a manual validation pass. Read the L4 layer doc, then verify it has the structure needed for downstream coverage verification (TDD generation, bead creation, completion audit).

---

## Your Mission

The L4 layer doc already exists at `docs/strategy/layers/layer-4-shared-visibility.md` (797 lines). It is execution-grade — but before it enters the TDD pipeline, you must verify it has unique requirement IDs (so test IDs can map back), explicit acceptance criteria (so beads can be self-contained), and zero scope-drift language (per the cardinal rule).

**This is NOT research-to-spec.** The content is already deeply iterated on. You are checking STRUCTURE, FORMAT, and SCOPE PRESERVATION — not rewriting content.

You are also resolving three known repo-vs-spec discrepancies recorded in `manifest.yaml > spec_callouts`. These are not scope changes; they are corrections that improve HOW the spec is realized in this codebase.

---

## Source Documents (Read In This Order)

### Primary Input (VALIDATE THIS)
```
docs/strategy/layers/layer-4-shared-visibility.md   — 797 lines — READ FULLY
```

### Source-of-Truth Context (SKIM)
```
docs/strategy/layer-strategy.md                     — cross-layer ordering doc (skim §6, §12)
docs/strategy/cross-reference-index.md              — search for "L4" / "Layer 4" / "shared visibility"
docs/strategy/layers/layer-3-operational-hardening.md  — what L3 delivers that L4 builds on
docs/strategy/layers/layer-5-expanded-commercial-intelligence.md  — L5 expectations of L4
docs/strategy/layers/layer-6-request-pipeline-automation.md       — L6 expectations of L4
```

### Codebase Context (GROUND TRUTH — verify spec aligns with reality)
```
migrations/0001_sqlite_state_initial.sql            — 545 lines, L0 schema. Search: does `local_outbox` exist? (Spec says "from L0 §2 DDL" but it does not exist here — see §1.3 callout below.)
migrations/0003_sqlite_runtime.sql                  — L1 runtime tables (executors, error_manifests, source_cursors, commit_clock)
migrations/0004_l2_pending_operator_intents.sql     — pending_operator_intents (queue L4 reconciles)
migrations/0005_l3_operational_hardening.sql        — validation_verdicts, listing_evidence, process_relations, process_aliases
crates/daemon/src/db/sqlite.rs                      — 600 lines, single connection wrapper. Confirms: NO sync.rs today.
crates/daemon/src/validation/policy.rs              — 959 lines, L3 dry-run policy engine (L4 transitions slices to enforcement)
crates/daemon/src/validation/executor.rs            — 1217 lines, validator skill spawner (writes to validation_verdicts)
crates/daemon/Cargo.toml                            — confirm: NO reqwest, NO tokio-postgres dependency today
apps/                                               — confirm: only `desktop/` exists; `web/` is net-new
infra/                                              — confirm: directory does NOT exist; net-new
packages/ui/                                        — read root package.json's lint rule that bans imports from @tauri-apps/api / apps/desktop / crates/daemon / supabase / transport
skills/validation/base/                             — template for the 4 L4 overlays (SKILL.md, result.schema.json, checkpoint.schema.json, canary.inputs.jsonl, golden.assertions.yaml, validator-rubric.yaml, references/)
AGENTS.md                                           — project rules; confirm bead CLI is `br` (the duplicated `bd` block at ~L651-708 is outdated)
```

### Quality Bar (MATCH THIS STRUCTURE)
```
docs/pipeline/l3-operational-hardening/archived/2026-l3-operational-hardening/01-validate-spec.md  — L3's analogous validation pass
docs/strategy/layers/layer-3-operational-hardening.md  — L3's spec, already validated, target shape
```

---

## Cardinal Rule: ZERO SCOPE DRIFT

**If it's in the L4 spec, it gets implemented.** Phases are execution ORDER, not importance tiers. Every phase ships. Later sections get the same depth as early sections.

You may IMPROVE clarity, IDs, structure, completeness. You may NOT:
- Remove features
- Defer features to "post-MVP", "Phase 2+", "future iteration", "stretch goal", "nice to have", "deferred", "out of scope", "simplified for initial implementation", "can be added later"
- Reduce ACs below what is stated
- Treat later sections as less important

**Specific L4 features that are sometimes mistakenly seen as deferable — they are NOT:**
- Provisional promotion buffer (§1.3 + §3.4) — it is constitutional for enforcement slices
- Durable command ledger (`operator_decision_commands`, §1.5) — without it CAS retries break
- Writer fencing token (`writer_epoch`, §1.6) — without it stale-daemon races corrupt remote
- Equal-version retry semantics (§1.3) — without it identical-envelope retries fail spuriously
- Hi-water fenced pull (§1.4) — without it paginated pulls skip/duplicate during concurrent changes
- Tombstone retention >=14 days (§3.6) — without it offline daemons miss deletions
- Epoch-repair reproject minimum path (§5.1 / §5.2) — the spec EXPLICITLY says "cannot remain a future nice-to-have"
- Cross-authority provenance tracking (§2.7) — `source_config_revision`, `company_profile_revision`, `operator_decision_revision` recorded per pipeline run
- Redaction tests for ALL 6 summary projections (§2.5) — security boundary, not UI convention

---

## Key Context From Codebase Research

1. **L4 spec is 797 lines, 5 sections.** §1 Overview & Sync Engine (10 subsections), §2 Supabase Schema/RLS/Remote-Owned Tables (7 subsections), §3 Web Portal & Validation Enforcement (7 subsections), §4 Testing & Acceptance, §5 Open Questions/Forward-Compat/Dependencies.
2. **18 DoD criteria** in §4.1, **14 invariants** in §1.9, **17 test scenarios** in §4.3, **8 risks** in §4.4, **3 open questions** in §5.1, **4 stubs** in §5.2.
3. **Three repo-vs-spec discrepancies** (recorded in `manifest.yaml > spec_callouts`):
   - **Migration filename:** spec writes `sqlite/migrations/0002_sqlite_state_sync.sql` (§1.10, §2.6, §5.4 footnote). Actual repo convention is flat `migrations/NNNN_*.sql`. Existing files: `0001`, `0003`, `0004`, `0005`, ..., `0011`. Next free is `0012`. **Action:** rename to `migrations/0012_l4_sync_state.sql` (or next available NNNN_) throughout the spec. This is HOW, not WHAT — no scope change.
   - **`local_outbox` reservation:** spec §1.3 says "From L0 §2 DDL". But `migrations/0001_sqlite_state_initial.sql` does NOT contain `local_outbox`. The cross-domain note for L0 at the bottom of §5.5 says "the constitutional table shape is reserved in L0, so these fields must be present when the table is first materialized for sync activation" — but the table itself is not pre-created in L0. **Action:** clarify in §1.3: L4 owns the `local_outbox` DDL in its SQLite migration; L0 reserves only the fact that L4 will materialize it.
   - **No HTTP client in daemon:** spec assumes the daemon can call Supabase RPCs. `crates/daemon/Cargo.toml` has neither `reqwest` nor `tokio-postgres`. **Action:** §5.4 should explicitly list "Add HTTP client dependency to daemon (`reqwest` with `rustls-tls`, `json` features)" as a prerequisite. (The bootstrap doc in batch 2 will own the actual `Cargo.toml` change; the spec just needs to call it out as a dependency.)
4. **Bead CLI is `br`** (not `bd`). AGENTS.md has a duplicated `bd` block at ~L651-708 that is outdated. The L4 spec does not currently mention bead CLI; if it adds operational guidance for L4 implementation, it should reference `br` and `bin/br-sync`.
5. **AgentMail registration** at session start uses `project_key="/Users/alinaaitova/Desktop/Testproject1"`. This is implicit but should be referenced indirectly via "the project key conventions used by L1/L2/L3".
6. **L3 deliverables L4 builds on** (verify these survive L4 spec wording):
   - `validation/policy.rs` (959 lines) — L4 transitions selected slices to enforcement; do not "rewrite", "replace", or "defer" the policy engine
   - `validation/executor.rs` (1217 lines) — L4 adds provisional-promotion-buffer write path; do not "rewrite", just extend
   - `validation/manifest_writer.rs` (54 lines) — already hybrid; L4 reuses
   - `validation_verdicts` table — already populated by L3; L4 adds `validation_verdict_summaries` Supabase projection (redacted)
   - `pending_operator_intents` queue (`migrations/0004_*.sql`) — L4 reconciles against pulled `operator_decision_heads` via "latest-per-process coalescing"
   - `error_manifests` table + `errors/manifest.rs` — L4 adds `error_manifest_summaries` push-owned projection
   - `process_stage_status`, `process_stage_heads` — L0 already has these; L4 projects push-only
7. **Net-new infrastructure L4 owns** (verify spec lists these as net-new, not pre-existing):
   - `crates/daemon/src/db/sync.rs` (single new module)
   - `apps/web/` directory (entire Next.js app — 7 files)
   - `infra/supabase/` directory (config + 8 migrations)
   - SQLite tables: `local_outbox`, `sync_batches`, `pull_cursors`
   - Supabase tables: 20 push-owned projections + 3 junction + 6 summary + `shared_company_profile` + `shared_source_configs` + `operator_decision_heads` + `operator_decision_commands` + `schema_metadata` + lease table + `viewer_sync_status` + `operator_decision_summaries`
   - 4 validator overlay skill directories under `skills/validation/`
   - HTTP client dependency in `crates/daemon/Cargo.toml`
8. **Two-stage migration coordination rule** (§2.6): "CI must apply both migrations from scratch, introspect both, and fail if logical schema hash differs." This is a CI gate — verify the spec calls it out as testable.

---

## Step-by-Step Workflow

### Step 0: Read Source Documents (MANDATORY)

Read in this order before touching anything:

1. `docs/strategy/layers/layer-4-shared-visibility.md` — **READ FULLY** (797 lines)
2. `docs/strategy/layers/layer-3-operational-hardening.md` — **SKIM** (the §1.3 list of L3 modules + §6.3 forward-compat to L4)
3. `docs/strategy/cross-reference-index.md` — **GREP** for "L4" or "Layer 4" or "shared visibility"
4. `docs/strategy/layer-strategy.md` — **SKIM** §6 (L4 overview), §12 (milestone mapping)
5. **HTML comments in the spec:** L4 spec lines ~791-797 are `<!-- CROSS-DOMAIN NOTE FOR LX: ... -->` blocks. These are EASILY missed when scanning for prose. Open the spec in a tool that renders HTML comments (any text editor) — do NOT trust a markdown renderer that hides them. Verify each HTML-comment cross-domain note matches the actual statement made in the target layer's spec (L0, L2, L6). Update wording for consistency if needed.

6. `migrations/0001_sqlite_state_initial.sql` — **OPEN** and search for `local_outbox` (it does NOT exist)
7. `crates/daemon/Cargo.toml` — **OPEN** and confirm no `reqwest` / `tokio-postgres`
8. `apps/` listing — **CONFIRM** only `desktop/` (no `web/`)
9. `infra/` listing — **CONFIRM** directory does not exist
10. `docs/pipeline/l3-operational-hardening/archived/2026-l3-operational-hardening/01-validate-spec.md` — **MATCH** this structure for your validation output

### Step 1: Validation Checklist

For each item, edit the spec in place to fix issues. Do NOT remove content; if something is missing, add it.

#### 1.1 Requirement IDs
- [ ] Every functional requirement has a unique, stable ID. Use the convention `L4-FR-{section}.{subsection}.{item}` (e.g., `L4-FR-1.3.1` = §1.3 push protocol item 1, `L4-FR-3.5.2` = §3.5 validator overlay item 2).
- [ ] IDs are systematic (by section/subsystem, not random)
- [ ] A downstream tool could grep for all `L4-FR-` IDs mechanically and find every requirement
- [ ] If missing: ADD IDs systematically without changing requirement content. Add IDs to:
  - §1.3 Push Protocol items (single-in-flight batch, identical-envelope retries, batch boundaries, equal-version retry semantics, provisional promotion buffer, etc.)
  - §1.4 Pull Protocol items (hi-water fence, pull-before-push, pending intent reconciliation)
  - §1.5 CAS items (RPC, command ledger, retry semantics, GC retention)
  - §1.6 Active-writer lease items (writer_epoch, fencing, lease heartbeat, operational rule)
  - §1.7 Schema epoch guard items
  - §1.8 Degraded-mode items (3 sync states, remote config staleness classes, backup/durability)
  - §1.9 Invariants — convert each row to its own `L4-FR` ID
  - §2.1-§2.5 Schema items (each table, RLS for each role, redaction policy)
  - §2.6 Migration coordination rule
  - §2.7 Cross-authority items (4 classifications, provenance tracking)
  - §3.1-§3.7 Web portal + enforcement items

#### 1.2 Acceptance Criteria
- [ ] Every requirement has explicit, bullet-listed acceptance criteria
- [ ] ACs are specific and testable (not "handle appropriately", "behaves correctly")
- [ ] ACs are not buried in narrative prose
- [ ] ACs reference observable outcomes (table state, RPC response codes, UI badge values)
- [ ] If buried: EXTRACT into bullet lists under each requirement. Particular hot spots:
  - §1.3 "Equal-version retry semantics" — extract the 4 cases (no-op success, hard conflict, normal advance, reject) as ACs
  - §1.5 "CAS flow with command ledger" — extract the 4 numbered steps as ACs
  - §1.8 "Remote config staleness classes" — extract the 4-row table into ACs per config type
  - §3.4 "Per-slice enforcement readiness" — extract the 7 prerequisites as testable ACs
  - §3.4 "Automatic demotion" — extract each trigger as an AC
  - §3.6 "Calibration corpora" — extract the 3 corpus sizes + the 9 mutation families as ACs

#### 1.3 Structure
- [ ] Consistent section numbering (§1.1 → §1.10, §2.1 → §2.7, §3.1 → §3.7, §4.1 → §4.4, §5.1 → §5.5)
- [ ] Clear subsystem boundaries (sync engine vs Supabase schema/RLS vs web+enforcement)
- [ ] §4.1 Definition of Done table has 18 rows (verify count)
- [ ] §4.2 Test File Listing table has 4 rows (sqlite_schema.rs, sqlite_supabase_sync.rs, redaction_boundary.rs, apps/web/e2e/contracts.spec.ts)
- [ ] §4.3 Test Scenarios table has 17 rows
- [ ] §4.4 Risks table has 8 rows
- [ ] §1.9 Invariants table has 14 rows (10 constitutional + provisional promotion + validation enforcement + RLS + pull-before-push + hi-water — check count)
- [ ] No TBD sections, no hand-waving
- [ ] Cross-domain notes at §5.5 / bottom are dated/scoped (one note per target layer)

#### 1.4 Completeness
- [ ] Error paths defined (sync degraded, lease conflict, schema mismatch, CAS conflict, validator crash, RPC timeout) — see §4.3 test scenarios for coverage
- [ ] Edge cases called out (paginated pull during concurrent changes, equal-version retry with mismatched content_hash, source remotely disabled while daemon offline)
- [ ] Integration points specified (job runner → validation executor → outbox writer → push dispatcher; pending_operator_intents queue → CAS RPC → operator_decision_heads)
- [ ] No "to be determined later" items

#### 1.5 Scope Preservation Scan (BLOCKING)
- [ ] Search the file for: `post-MVP`, `Phase 2+`, `stretch goal`, `nice to have`, `nice-to-have`, `deferred`, `out of scope`, `out-of-scope`, `future iteration`, `simplified for initial implementation`, `can be added later`, `for now`, `eventually`, `someday`
- [ ] If any are found in load-bearing positions (i.e., describing a current-layer requirement, not legitimately describing a future layer's responsibilities), this is a BLOCKING failure. Remove the language and replace with full-implementation content.
- [ ] **Specific verifications:**
  - [ ] §5.2 stub for "Epoch-repair reproject" — must list the **minimum** in-L4 path (mark `projection_generation`, republish current row images, discard stale-epoch outbox after reseed). The spec already says this "cannot remain a future nice-to-have" — verify the minimum path is concrete and testable.
  - [ ] §5.2 stub for "Logical consistency auditor" — verify this is a stub (deferred), not a §1 invariant. (It's listed as Future in §5.2; OK.)
  - [ ] §3.6 calibration harness — verify all 9 mutation families are listed with explicit "what it tests" semantics
  - [ ] §3.5 validator overlays — verify all 4 are listed with concrete domain checks (not "TBD" or "details deferred")

#### 1.6 Discrepancy Resolution (THIS SPEC ONLY — record decisions IN the spec)
- [ ] **Migration filename**: BEFORE editing the spec, verify the next-free SQLite migration sequence number:
  ```bash
  ls migrations/ | grep -E '^[0-9]{4}_' | sort | tail -1
  # Last existing NNNN at handoff-generation time was 0011. The next-free is whatever 0011+1 currently is.
  ```
  Then locate every reference in §1.10, §2.6, §5.4 to `sqlite/migrations/0002_sqlite_state_sync.sql` and replace with `migrations/{NEXT_NNNN}_l4_sync_state.sql` consistently. Keep the existing semantic content; only the path changes. Record the chosen NNNN in a single footnote so future readers know why this number was picked.
- [ ] **`local_outbox` reservation**: edit §1.3 and the cross-domain note at the bottom of §5.5 to clarify that L4 owns the `local_outbox` DDL in `migrations/0012_l4_sync_state.sql`; L0 reserves only the column shape (`commit_group_id`, `writer_epoch`) for forward-compat.
- [ ] **HTTP client dependency**: add a line to §5.4 under "Daemon dependencies" or a new "Toolchain prerequisites" subsection: "Daemon must add `reqwest` (with `rustls-tls`, `json` features) — currently `crates/daemon/Cargo.toml` has no HTTP client."
- [ ] **`apps/web/` is net-new**: §3.1 already says "Create" — verify the spec is clear that this directory does not exist today and L4 creates it. Add a sentence: "Root `package.json` workspaces and the `bun run {build,test,lint}` scripts must be updated to include `apps/web`." 
- [ ] **`infra/supabase/` is net-new**: §2.6 must clearly state this is a new directory tree.

#### 1.7 Cross-Strategy-Doc Consistency (BLOCKING)

The L4 spec is part of a 10-file strategy set in `docs/strategy/`. After your edits in §1.1–§1.6, verify the cross-doc references are still accurate:

- [ ] **Open `docs/strategy/layer-strategy.md`** and grep for "L4" / "Layer 4" / "shared visibility". The references should describe what L4 builds in a way consistent with your post-validation spec. If `layer-strategy.md` still references `0002_sqlite_state_sync.sql` or other pre-validation paths, update them to match.
- [ ] **Open `docs/strategy/cross-reference-index.md`** and grep for "L4". Same exercise — every L4 cross-ref must match the post-validation spec.
- [ ] **Open `docs/strategy/README.md`** and confirm L4 is listed (if it has a layer table).
- [ ] **Open `docs/strategy/layers/layer-3-operational-hardening.md`** §6.3 (forward-compat to L4). Confirm L3's forward-compat statements are still accurate after your spec edits.
- [ ] **Open `docs/strategy/layers/layer-5-expanded-commercial-intelligence.md`** and `layer-6-request-pipeline-automation.md`. They depend on L4 — confirm their dependency declarations match the post-validation L4 spec.
- [ ] **Open `docs/strategy/layers/layer-0-gold-set-evaluation.md`** (4338 lines — large) — grep for `commit_group_id`, `writer_epoch`, `local_outbox`. The L4 cross-domain note at the bottom of L4 §5.5 says "the constitutional table shape is reserved in L0". Verify L0's spec actually contains the **reciprocal forward-compat reservation note** for these column shapes. If L0 doesn't yet reference them, ADD a single forward-compat bullet to L0 (not a full DDL — just a reservation note, e.g., "Forward-compat: when L4 materializes `local_outbox`, the `commit_group_id` (text NULL) and `writer_epoch` (integer NULL) columns are constitutional and must be present at first creation. L0 does not pre-create the table."). This is a paired edit — L4 says "L0 reserves X"; L0 must actually reserve X for the cross-reference to be honest.

Any cross-doc inconsistency = BLOCKING. Edit all affected docs in the same commit so cross-references remain coherent.

### Step 2: Output

The output is the **same spec file**, validated and structured in place. Do NOT create a new spec file. If the version field exists, bump it (e.g., `v0.5 → v0.6` with a "Validated for 3-SOT pipeline" note in the changelog).

Verify before exiting:
- [ ] No content was deleted (other than scope-drift language)
- [ ] No content was rephrased to weaken meaning
- [ ] All 18 DoD criteria still present (count after edits)
- [ ] All 14 invariants still present (count after edits)
- [ ] All 4 validator overlays still present in §3.5
- [ ] All 8 Supabase migrations still present in §2.6 / §5.4
- [ ] Migration path discrepancy resolved consistently (every reference uses the same path)

### Step 3: Follow-Up Pass (Sub-Optimization)

> Send this as a follow-up message in the same session after Step 2 is complete.

ULTRATHINK.

Re-read the validated spec with fresh eyes and answer:

1. **Open `crates/daemon/src/validation/executor.rs` (1217 lines).** The L3 executor writes verdicts but never quarantines results. L4 §3.4 says rejected results in enforcement mode are quarantined as `AGENT_SUSPECT_OUTPUT`. The provisional promotion buffer (§1.3) holds worker outputs in local-only staging until validation completes. **Question:** does the spec name a specific staging table OR an artifact directory for provisional results? If "staging table or artifact directory" is left ambiguous, agents implementing this will diverge. Pick one and write it into §1.3 as a new AC. (Recommendation: a SQLite staging table `provisional_worker_outputs` indexed by `worker_job_id` with `payload_json`, `result_contract_blob_path`, `created_at`, `validation_status enum('pending','approved','rejected')`. This is consistent with the existing `validation_verdicts` storage model.)

2. **Open `migrations/0004_l2_pending_operator_intents.sql`.** The table has `desired_disposition`, `reviewed_basis_hash`, `reviewed_manifest_hash`, `reviewed_at_stage_run_id`, `base_remote_revision`, `idempotency_key UNIQUE`, `superseded_by_intent_id`. The L4 spec §1.4 says "after pull, reconcile pending_operator_intents queue: ... coalesce multiple intents for same process (latest-per-process)." **Question:** does §1.4 specify which intent wins on coalescing — the highest `idempotency_key` lexicographic? The latest `created_at`? The most recent unsuperseded intent? Pick one and write it as an AC. The L2 schema's `superseded_by_intent_id` strongly suggests "all unsuperseded intents are coalesced into the most-recent-by-`created_at`" — verify.

3. **Open `crates/daemon/src/ipc/handlers/decisions.rs`.** This is where pending operator intents are written. After L4, the daemon writes intents AND flushes them through CAS. **Question:** does the spec call out the IPC contract change required in §1.5? (Desktop sends decision → daemon writes pending_operator_intents → CAS flusher promotes → desktop must re-fetch fresh state. The spec mentions "Desktop shows conflict resolution UI" but doesn't enumerate the IPC events.) Add an AC: "On CAS conflict, daemon emits `decision.conflict` event over IPC carrying current `remote_revision`, current `disposition`, and the rejected intent payload."

4. **Open `packages/ui/freshness-badge.tsx`.** L4 §3.3 specifies three sync-health states (green/amber/red) with thresholds (90s/300s). **Question:** does the spec specify that the FreshnessBadge component must accept a `syncHealth` prop, or does it just describe the visual states? The cross-domain note for L2 at the bottom of §5.5 says "FreshnessBadge component (L2 §2.1) should support the three-state sync indicator." Verify that the component contract is concrete enough that a bead author can implement it without re-reading §3.3 (e.g., add an AC: "FreshnessBadge accepts `syncHealth: 'healthy' | 'degraded' | 'offline'` prop and `lastSuccessAt: ISO-8601 string`; thresholds are computed inside the component, not by the caller").

5. **Open `package.json` (root).** The lint rule bans `packages/ui` from importing `@tauri-apps/api`, `apps/desktop`, `crates/daemon`, `supabase`, `transport`. **Question:** does §3.1 (`apps/web` 7 files) make clear that `apps/web/src/lib/supabase.ts` is the ONLY place Supabase imports live in the web app, and that web pages reuse `packages/ui` components without modifying them? Add an AC: "`packages/ui` remains free of Supabase, Tauri, and daemon imports. `apps/web/src/lib/supabase.ts` is the sole Supabase boundary in the web app."

6. **Walk every L4-FR you assigned.** Pick three at random. For each, ask: "If I were writing a bead from this requirement alone — could I? Does it have a file path, a function/type name, an observable outcome, a test ID hook?" If any one fails, revise.

7. **Per-slice enforcement readiness STATE LOCATION (spec gap).** §3.4 lists 7 prerequisites + 4 demotion triggers for "worker skill × source × worker behavior version × validator overlay version". But the spec does NOT say where this readiness state LIVES. Options the spec leaves implicit:
   (a) A new SQLite table `enforcement_readiness` keyed by slice tuple, persisted across daemon restarts
   (b) An in-memory cache in `validation/policy.rs` rebuilt at daemon start by scanning recent verdicts
   (c) Computed on-demand from `validation_verdicts` per-call (no cache)
   Pick ONE and write it as an AC under §3.4 (or a new §3.4.X subsection). Recommendation: (a) — a new SQLite table is consistent with the durable-state-first pattern in this codebase (per `crates/daemon/src/db/`), survives daemon restarts, and is queryable by metrics. Add columns: `slice_id` (composite or hashed), `worker_skill_id`, `source_id`, `worker_behavior_version`, `validator_overlay_version`, `enforcement_state` (eligible / dry_run / demoted), `last_evaluated_at`, `wilson_lower_bound_bps`, `false_pass_upper_bound_bps`, `adjudicated_count`, `audited_approvals_count`, `demotion_reason` (nullable). Add this table to `migrations/{NEXT_NNNN}_l4_sync_state.sql` (or a sibling migration). This is a HOW improvement, not WHAT scope reduction.

8. **Lease table location (spec ambiguity, BLOCKING).** §1.6 says "In `schema_metadata` or dedicated lease table" — leaving the choice open. Migration §2.6 row 6 names `schema_metadata` table with "active-writer lease row with writer_epoch". A bead author cannot proceed without a single answer. Pick ONE:
   (a) Single row in `schema_metadata` with columns `org_id, holder_executor_id, lease_expires_at, writer_epoch` (concise, but mixes schema metadata with operational state)
   (b) Dedicated `active_writer_lease` table (separation of concerns, easier to evolve)
   Recommendation: (b) — separate concerns. Migration `0006_schema_metadata.sql` then creates BOTH `schema_metadata` (epoch only) and `active_writer_lease` (the fencing token row). Update spec §1.6 + §2.6 row 6 + §5.4 to name the chosen approach consistently.

9. **`apply_push_batch` RPC signature INCONSISTENCY (BLOCKING).** §1.3 specifies `apply_push_batch(batch_json, writer_epoch)` — 2 arguments. §2.6 row 5 specifies `apply_push_batch(batch_json)` — 1 argument. ONE of these is wrong. Per §1.6 ("Every push batch must include the current `writer_epoch`. Remote rejects batches with stale epoch."), the 2-argument form is correct: the writer_epoch must be passed and validated atomically with the batch. Update §2.6 row 5 to match §1.3: `apply_push_batch(batch_json, writer_epoch)`. Verify §5.4 dependencies row 5 matches.

10. **Re-scan for scope-drift language one final time.** Search again for: `post-MVP`, `Phase 2+`, `nice to have`, `nice-to-have`, `deferred`, `stretch goal`, `simplified for initial`, `can be added later`, `eventually`, `for now`. Some occurrences may legitimately describe future-LAYER scope (L5, L6) — those are fine. The blocking question is: does any L4 in-scope feature get this treatment? If yes, fix.

### Step 4: Scope-Preservation Check (MANDATORY — BLOCKING)

After Step 3, perform this final blocking check:

1. **Enumerate** every feature/subsystem/capability described in the spec. For L4, the easiest enumeration is the §4.1 DoD table (18 rows) plus the §1.9 invariants (14 rows). Make a checklist on paper or in a scratch file.
2. **Verify** each one is represented in your validated output — nothing was silently dropped.
3. **Scan** the file again for any of these red-flag terms in load-bearing positions: `post-MVP`, `Phase 2+`, `stretch goal`, `nice to have`, `nice-to-have`, `deferred`, `out of scope`, `out-of-scope`, `future iteration`, `simplified for initial implementation`, `can be added later`, `eventually`, `someday`, `for now`.
4. **If any feature is missing** — BLOCKING failure. Add it before proceeding.
5. **If any red-flag language is found** in a load-bearing context (describing an L4 requirement) — BLOCKING failure. Remove and replace with proper content.

---

## Key Reminders

- **Do NOT delete files.** If you think a section is wrong, edit in place. Never delete the spec.
- **Do NOT rewrite the spec.** Add IDs, extract ACs, fix the three discrepancies, structure-check. That is all.
- **Do NOT introduce new requirements.** If something is missing technically, flag it as an open question in §5.1, do not silently invent ACs the original spec didn't have.
- **Bead CLI is `br`.** AGENTS.md has a duplicated `bd` block at ~L651-708 — that block is outdated. Reference `br` and `bin/br-sync` if you add operational guidance.
- **AgentMail project key is `/Users/alinaaitova/Desktop/Testproject1`.** This is implicit in cross-references; if you add coordination guidance, use this exact path.
- **The spec uses GBP integer pence and basis-points (bps) for fractions** (per L0/L3 conventions). Do not introduce floats anywhere in DDL or AC math.
- **Discrepancy decisions you make MUST be recorded in the spec** — do not just fix in the agent's head. Future readers must see what was changed and why.
- **ZERO SCOPE DRIFT: every feature in the layer doc must be covered in the output.** That includes the cross-domain notes at the bottom of §5.5 — those are commitments to other layers, not asides.
- **The L0 `local_outbox` cross-domain note** at the bottom of §5.5 needs a small wording edit: change "the constitutional table shape is reserved in L0" to "the constitutional column shape (`commit_group_id`, `writer_epoch`) on `local_outbox` is reserved here as a forward-compat note for when L4 first materializes the table". Confirm this matches the actual L0 spec wording.
