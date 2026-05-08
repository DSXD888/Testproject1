# L4 Shared Visibility — 05 · Create Strategy (Phase Ordering Only)

## First Action

Invoke the `/spec-to-strategy` skill.

---

## Your Mission

Generate the **Strategy** document (SOT #3) for L4. The strategy defines **execution order only** — phase sequencing, dependency DAG, fan-out parallelism, gate criteria. It does NOT duplicate spec or TDD content.

**Phases are execution ORDER, not importance tiers.** Every phase ships. Later phases get the SAME depth as early phases. The strategy's role is to answer "in what order does an agent claim beads?" — not "which features are most important?".

---

## Source Documents (Read In This Order)

### Primary Inputs (CREATE FROM THESE)
```
docs/strategy/layers/layer-4-shared-visibility.md                          — Spec (SOT #1)        — READ FULLY
docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-tdd.md   — TDD (SOT #2)         — READ FULLY
docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-bootstrap.md  — Bootstrap        — READ FULLY
```

### Source-of-Truth Context
```
docs/strategy/layer-strategy.md                              — cross-layer ordering doc — SKIM (§6 L4, §12 milestones)
docs/strategy/cross-reference-index.md                       — every L4 cross-ref — REFERENCE
docs/strategy/layers/layer-3-operational-hardening.md        — what L3 delivers — SKIM
```

### Codebase Context (DEPENDENCY DISCOVERY — CRITICAL FOR ORDERING)
```
crates/daemon/src/db/sqlite.rs                              — confirm: schema migration must precede sync.rs
crates/daemon/src/db/checkpoints.rs                         — pattern: db modules wait for migration
crates/daemon/src/validation/policy.rs                      — 959 lines, L3 dry-run policy — extends in L4
crates/daemon/src/validation/executor.rs                    — 1217 lines, validator skill spawner — extends in L4
crates/daemon/src/validation/manifest_writer.rs             — 54 lines
crates/daemon/src/jobs/runner.rs                            — 1027 lines, single enqueue boundary — extends with provisional promotion
crates/daemon/src/ipc/handlers/decisions.rs                 — pending_operator_intents writer — extends with CAS flush
crates/daemon/src/main.rs                                   — 1.08 MB monolithic init — wires startup pull-before-push, lease acquisition, schema_epoch check
crates/daemon/Cargo.toml                                    — net-new HTTP client dependency (must be added before sync.rs work)
migrations/                                                 — flat sequence; L4 owns next NNNN_ (likely 0012)
infra/supabase/                                             — net-new directory; must be created before push/pull tests
apps/web/                                                   — net-new directory; must be created before web bead work
package.json (root)                                         — workspaces edit before apps/web work; lint rule must hold
skills/validation/base/                                     — template the 4 overlays follow
```

### Quality Bar (MATCH OR EXCEED)
```
docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-strategy.md  — 478 lines, 12 sections — TARGET
docs/pipeline/l3-operational-hardening/archived/2026-l3-operational-hardening/05-create-strategy.md  — L3's analogous handoff
```

### L3 Strategy section structure (target — verify the L4 strategy has these 12 sections in this order)
1. Source Documents
2. Strategy Guardrails       (must restate: phases = execution order, not importance; strategy provides ordering only; spec/TDD content not duplicated)
3. Overview                  (table summarizing all phases with bead counts + gate criteria)
4. Critical Path And Parallelism   (explicit DAG; mark which phases run in parallel)
5. Suggested Bead Fan-Out
6. Implementation Unit Allocation
7. Coverage Summary          (L4-FR ↔ phase mapping; verify 100%)
8. Phase 1 (Schema Foundation)
9. Phase 2 (Push Engine)
10. Phase 3 (Pull Engine + CAS)
11. Phase 4 (Validation Enforcement)
12. Phase 5 (Web Portal) — and Phase 6 (Integration & Hardening) as section 12 sub or separate phase blocks
   (L3 had 6 phase sections after §7; L4 should also have one section per phase. If you choose 6 phases, that's 6 sections — total 12+ section count is acceptable.)

Plus closing sections: Dependency Diagram, Completeness Mapping, Gates Summary (L3 had these as final blocks).

---

## Cardinal Rule: ZERO SCOPE DRIFT

**Phases are EXECUTION ORDER**, not importance tiers. You may resequence WHEN features are built; you may NOT decide some phases are "less important" than others. Later phases get the SAME depth as early phases. Every phase ships.

The most common failure mode: a "Phase 6: Polish & Hardening" or "Phase 6: Stretch Items" — both are scope drift in disguise. The right framing is "Phase N: integration tests + final wiring + DoD walkthrough", not "Phase 6: optional follow-ups".

---

## Key Context From Codebase Research

1. **Critical dependency ordering** (must be reflected in phase DAG):
   - **Schema migrations come first.** `migrations/0012_l4_sync_state.sql` and `infra/supabase/migrations/0001-0008` must apply cleanly before any sync code runs. The schema migration is the root of the dependency DAG.
   - **HTTP client dependency before sync.rs.** Adding `reqwest` to `crates/daemon/Cargo.toml` is a tiny PR but blocks every push/pull bead.
   - **`apps/web` workspace wiring before any web file work.** Adding `apps/web` to root `package.json` workspaces + scripts is a single bead but blocks every Next.js bead.
   - **Outbox writer before push dispatcher.** The dispatcher reads `local_outbox`; the writer creates entries. Within the push engine, writer is upstream.
   - **Pull engine independent from push.** They share the SQLite layer but can be built in parallel after schema lands.
   - **CAS RPC + command ledger before pending intent reconciliation.** The reconciliation flushes through the CAS path.
   - **Provisional promotion buffer before enforcement transition.** Enforcement assumes the buffer exists.
   - **4 validator overlays parallel.** Each is a `skills/validation/{slug}/` directory with the 6-file template; pure filesystem work.
   - **Web portal can start as soon as projection tables (push-owned) are populated** in Supabase. Doesn't strictly need pull or CAS first — the web is read-only.
   - **Calibration harness wires through validation/policy.rs (L3) and L4 enforcement** — needs both to be in place. Comes late.
   - **Schema-epoch + lease integration in `main.rs`** = phase-end integration bead (don't disturb early; finalize last).

2. **Suggested phase shape (heuristic — validate against critical-path analysis):**

   - **Phase 1 — Schema Foundation** (root of DAG; small but blocks everything)
     - SQLite migration `migrations/0012_l4_sync_state.sql` (creates `local_outbox`, `sync_batches`, `pull_cursors`)
     - Supabase migrations 0001-0008 in `infra/supabase/migrations/`
     - `infra/supabase/config.toml` skeleton
     - Add `reqwest` (or chosen alternative) to `crates/daemon/Cargo.toml`
     - Add `apps/web` to root `package.json` workspaces + scripts
     - Validate cross-DDL (CI rule: both migrations from scratch + introspect + hash)

   - **Phase 2 — Push Engine** (depends on Phase 1)
     - Outbox writer (transactional with data writes; commit_group_id grouping)
     - `apply_push_batch` RPC (Postgres function)
     - Push dispatcher (single-in-flight; identical-envelope retries; equal-version retry semantics; batch boundaries respected)
     - Writer fencing (`writer_epoch` increment on lease; lease row in `schema_metadata`)
     - Active-writer lease acquisition + heartbeat
     - `viewer_sync_status` update on success
     - Schema epoch guard (pause sync on mismatch; minimum epoch-repair reproject path)

   - **Phase 3 — Pull Engine + CAS** (depends on Phase 1; parallelizable with Phase 2)
     - Hi-water fenced cursor pull (3 domains: shared_source_configs, shared_company_profile, operator_decision_heads)
     - Tombstone-aware daemon pull SELECT
     - Pull-before-push ordering at startup
     - `update_operator_decision` CAS RPC (Postgres function)
     - `operator_decision_commands` durable command ledger
     - Pending operator intent reconciliation (latest-per-process coalescing)
     - IPC: daemon emits `decision.conflict` event on CAS conflict

   - **Phase 4 — Validation Enforcement** (depends on Phases 2 + 3 OR can start when Phase 1 done if testable in isolation)
     - Provisional promotion buffer (staging table or artifact directory — pick the spec's choice)
     - Per-slice enforcement readiness gate (Wilson-bound precision, rule-of-three false-pass, all 7 prerequisites)
     - Auto-demotion triggers (override rate, challenge-bank regression, anomaly-conditioned, version drift)
     - 4 validator overlays in `skills/validation/{procurement-ingestion,supplier-enrichment,document-acquisition,product-extraction}/`
     - `AGENT_SUSPECT_OUTPUT` error type wired in `crates/daemon/src/errors/manifest.rs`
     - Quarantine flow: rejected results stay local, error manifest created, stage head NOT advanced
     - Calibration harness: 3 corpora, 9 mutation families, pairwise challenge sets, Wilson-bound math, rule-of-three math, cross-model audit sampling
     - Override feedback loop: every override → labeled calibration case + incident-cluster member + candidate mutation template

   - **Phase 5 — Web Portal** (depends on Phase 1; can start in parallel with 2/3 once schema is up)
     - `apps/web/src/app/layout.tsx` (root layout + sync status badge)
     - `apps/web/src/app/page.tsx` (redirect to /contracts)
     - `apps/web/src/app/contracts/page.tsx` (keyset-paginated list)
     - `apps/web/src/app/contracts/[id]/page.tsx` (opportunity detail)
     - `apps/web/src/app/status/page.tsx` (pipeline status)
     - `apps/web/src/lib/supabase.ts` (client init, typed queries)
     - `apps/web/src/lib/broadcast.ts` (subscription manager, topic-based invalidation)
     - Three-state degraded-mode UX (healthy/amber/red with thresholds)
     - Reuse `packages/ui` components without modification (`packages/ui` lint invariant holds)
     - Viewer-credentials-only enforcement (security boundary, not UI convention)
     - `apps/web/e2e/contracts.spec.ts` Playwright tests

   - **Phase 6 — Integration & Hardening** (depends on Phases 2-5)
     - Wire startup sequence in `main.rs`: pull-before-push, lease acquisition, schema_epoch check
     - Final RLS policy file `infra/supabase/migrations/0004_rls_policies.sql` with 4 roles × 7 categories
     - All 6 summary projections + redaction tests in `crates/daemon/tests/redaction_boundary.rs`
     - Tombstone retention >=14 days (operational constraint + CI test)
     - Logical consistency auditor stub (per §5.2 — minimum stub, not full)
     - Cross-authority provenance recording per pipeline run (`source_config_revision`, `company_profile_revision`, `operator_decision_revision`)
     - Two-user manual verification protocol (desktop operator + web viewer same data)
     - Full E2E push-pull round-trip (`crates/daemon/tests/sqlite_supabase_sync.rs` invocation list)
     - Schema validation CI gate (logical schema hash check)
     - DoD walkthrough — verify all 18 criteria in §4.1

3. **Expected bead count:** ~30-50 task beads + 4-6 epic beads. (L3 had 36 task beads + 6 review-gate epics = 42 total. L4 may run higher because of the web app + 8 Supabase migrations + 4 overlays.)

4. **{ENRICHMENT: tdd metadata}** — after step 04 completes, the TDD will have N test IDs across the 4 files. The strategy's gate criteria can reference specific test IDs (e.g., "Phase 2 gate: `L4-T-sqlite_supabase_sync-001` through `L4-T-sqlite_supabase_sync-{push-engine-end}` pass"). Replace this marker with actuals.

5. **{ENRICHMENT: spec metadata}** — total L4-FR count + ID range. Replace with actuals.

6. **AgentMail project key:** `/Users/alinaaitova/Desktop/Testproject1`. Bead CLI: `br`. Sync: `bin/br-sync`. These are for the strategy doc to reference in operational guidance only — not for the strategy to execute itself.

---

## Step-by-Step Workflow

### Step 0: Read Source Documents (MANDATORY — before anything)

1. `docs/strategy/layers/layer-4-shared-visibility.md` — **READ FULLY**
2. `docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-tdd.md` — **READ FULLY**
3. `docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-bootstrap.md` — **READ FULLY**
4. `docs/strategy/layer-strategy.md` — **SKIM** §6, §12
5. `docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-strategy.md` — **STUDY** the 12-section structure (Source Documents, Strategy Guardrails, Overview, Critical Path And Parallelism, Suggested Bead Fan-Out, Implementation Unit Allocation, Coverage Summary, Phase 1-6, Dependency Diagram, Completeness Mapping, Gates Summary)
6. `crates/daemon/src/main.rs` (skim only — this is huge but the wiring concept matters)
7. `apps/desktop/` directory listing — infer the parallel structure for `apps/web/`
8. Spec §1.10 Components Touched table — every component must appear in some phase
9. Spec §5.4 Dependencies — every dependency must be in Phase 1 or earlier

### Step 1: Invoke `/spec-to-strategy`

**Inputs:**
- Spec: `docs/strategy/layers/layer-4-shared-visibility.md`
- TDD: `docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-tdd.md`
- Bootstrap: `docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-bootstrap.md`
- Output: `docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-strategy.md`

**Key arguments/context to provide the skill:**
- Bead prefix: `l4`
- Project key: `/Users/alinaaitova/Desktop/Testproject1`
- Existing epic: `testproject1-4zq` (Layer 4 — Shared Visibility) — strategy will guide whether to keep this single epic or sub-divide
- Quality bar: L3 strategy (478 lines, 12 sections, 6 phases)
- Suggested phase shape: 6 phases per "Key Context" §2 above (Schema Foundation, Push Engine, Pull Engine + CAS, Validation Enforcement, Web Portal, Integration & Hardening) — but VALIDATE against critical-path analysis; do not blindly accept
- Strategy provides ORDERING ONLY — must NOT duplicate spec or TDD content
- Every phase has a gate criterion referencing TDD test IDs

### Step 2: Feature-Specific Guidance

While the skill runs, pay attention to:

- **Phase 1 must be SHORT and ATOMIC.** Schema migrations + dependency adds + workspace wiring. Should be 5-8 beads. If it sprawls, it's not a foundation phase.
- **Phase 2 (Push Engine) is the largest** — 8-12 beads (outbox writer, dispatcher, RPC, fencing, lease, viewer_sync_status update, epoch guard, equal-version retry, batch boundary checks).
- **Phase 3 (Pull + CAS) is medium** — 6-8 beads (hi-water fenced pull × 3 domains, CAS RPC, command ledger, pending intent reconciliation, decision.conflict IPC event).
- **Phase 4 (Enforcement) is medium-large** — 8-12 beads (provisional promotion, per-slice readiness gate, demotion triggers, 4 overlays, AGENT_SUSPECT_OUTPUT, calibration harness with 3 corpora + 9 mutations + pairwise + Wilson + rule-of-three + cross-model + override loop).
- **Phase 5 (Web Portal) is medium** — 6-10 beads (7 files + workspace + auth + RLS verification + Playwright).
- **Phase 6 (Integration) is small-medium** — 5-8 beads (main.rs wiring, RLS policies file, redaction tests, tombstone retention, two-user verification protocol, DoD walkthrough).
- **Critical sequencing:** schema → outbox writer → push dispatcher (Phase 2 internal); schema → pull cursor → CAS → reconciliation (Phase 3 internal). Validate.
- **Parallelism:** Phase 2 and Phase 3 can run in parallel after Phase 1. Phase 5 (web) can start early once projection tables exist (don't block on full Phase 2). Phase 4 overlays can be developed in parallel with Phase 4 enforcement gating logic.
- **`main.rs` integration is phase-end**: do NOT touch `main.rs` from Phase 2-5 beads except via a designated integration bead at end of phase. This prevents merge conflicts during parallel execution.
- **Calibration harness is a phase 4 capstone** — depends on enforcement gating + 4 overlays + validator base policy.
- **Two-user verification** is a phase 6 deliverable, not a phase 5 web bead.
- **Every phase has a gate.** The gate must be a test invocation pattern (e.g., "Phase 2 gate: all `L4-T-sqlite_supabase_sync-{push-N}` pass under `cargo test --test sqlite_supabase_sync`").

### Step 3: Verify Output

After the skill completes, verify:

- [ ] `docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-strategy.md` exists
- [ ] Strategy has ~12 sections matching L3 quality bar
- [ ] §3 Overview lists ≥6 phases (5 is too few; 8 is too many for L4 scope)
- [ ] §4 Critical Path And Parallelism has an explicit DAG (textual or ASCII)
- [ ] Every phase has: (a) explicit dependency on prior phase(s); (b) suggested bead count; (c) gate criterion referencing TDD test IDs
- [ ] Spec components from §1.10 (Database, daemon `db/sync.rs`, Skills × 4 overlays, `apps/web` × 7 files) all appear in some phase
- [ ] Spec dependencies from §5.4 (Supabase project, 8 migrations, `db/sync.rs`, 4 overlays, web app) all addressed
- [ ] Strategy says explicitly: "Phases are EXECUTION ORDER, not importance tiers. Every phase ships."
- [ ] No "post-MVP" / "Phase 2+" / "stretch goal" / "nice to have" / "deferred" / "future iteration" language anywhere
- [ ] No spec content is duplicated (the strategy references the spec; it does not re-state requirements)
- [ ] No TDD content is duplicated (gate criteria reference TDD test IDs, not test bodies)

### Step 4: Follow-Up Pass (Sub-Optimization)

> Send this as a follow-up message in the same session after the main work is complete.

ULTRATHINK.

Re-read the strategy with fresh eyes and answer:

1. **Open `crates/daemon/src/main.rs`.** This is huge (1.08 MB). The strategy must NOT have multiple Phase 2-5 beads editing `main.rs` simultaneously. Verify your strategy has exactly ONE designated `main.rs` integration bead per phase (or a single phase-6 wiring bead). If multiple beads in different phases touch `main.rs` concurrently, this WILL cause merge conflicts in parallel execution.

2. **Open `migrations/`** and confirm the existing sequence (0001, 0003, 0004, 0005, ..., 0011 — no 0002 by accident). Verify your strategy's Phase 1 explicitly names `migrations/0012_l4_sync_state.sql` (or the correct next-NNNN). If the spec was edited in step 01 to use this name, the strategy must match.

3. **Open `crates/daemon/Cargo.toml`** and confirm no HTTP client. Verify your strategy's Phase 1 has a "Add reqwest dependency" bead before any sync.rs work. If the dep add is folded into a sync.rs bead, that's a serialization point that should be split.

4. **Open `package.json` (root)** and confirm workspaces include `packages/*` and `apps/desktop`. Verify your strategy's Phase 1 has an "Add apps/web to workspaces + scripts" bead. Verify it preserves the `packages/ui` lint rule banning desktop/tauri imports.

5. **Open `crates/daemon/src/jobs/runner.rs`** (1027 lines). The provisional promotion path extends the runner. Verify your strategy doesn't have multiple beads simultaneously editing the runner — this is the "single enqueue boundary" file and modifications must be serialized.

6. **Open `crates/daemon/src/validation/policy.rs`** (959 lines). L4 extends with per-slice enforcement readiness. Verify your strategy puts the policy-engine extension bead AFTER the schema bead but possibly BEFORE the executor extension bead (the executor consumes the policy decisions).

7. **Walk every of the 18 DoD criteria.** For each, identify which phase ships it. If any DoD criterion has no clear phase home, that's a coverage gap.

8. **Walk every of the 14 invariants.** For each, identify which phase establishes the invariant. Invariants that span multiple phases are fine — but each invariant must have a "home" where it's first established.

9. **Re-scan strategy for scope-drift language.** Search for: `post-MVP`, `Phase 2+`, `nice to have`, `stretch goal`, `deferred`, `simplified for now`, `can be added later`, `eventually`, `for now`, `optional`, `if time permits`. Hits in load-bearing positions = BLOCKING.

10. **Phase coverage equality check.** Count beads per phase. If Phase 6 has 2 beads while Phase 2 has 12, investigate — Phase 6 may be silently scope-cut. If Phase 6's role is genuinely smaller (e.g., final wiring), document why; if it should be larger, expand.

11. **Verify the gate criterion for every phase is testable.** "Phase 2 gate: all push-engine tests pass" is too vague. "Phase 2 gate: `L4-T-sqlite_supabase_sync-001` through `L4-T-sqlite_supabase_sync-NNN` (push-engine subset) pass under `cargo test --test sqlite_supabase_sync` AND `cargo clippy --all-targets -- -D warnings` is clean for `crates/daemon/src/db/sync.rs` push-side modules" is testable.

### Step 5: Scope-Preservation Check (MANDATORY — BLOCKING)

After the sub-optimization, perform this check:

1. **Enumerate every spec L4-FR.** Verify each is owned by some phase in the strategy.
2. **Enumerate every of the 4 phase-1 prerequisites** (SQLite migration, Supabase migrations, daemon HTTP client, apps/web workspace wiring). Verify all 4 are in Phase 1 (or earlier — but there is no earlier).
3. **Scan strategy for red-flag language.** Any `post-MVP`/`Phase 2+`/`stretch goal`/`nice to have`/`deferred`/`future iteration`/`simplified for now`/`can be added later`/`eventually`/`for now` in load-bearing position = BLOCKING.
4. **Check phase coverage equality.** No phase should be anomalously thin compared to others (unless its scope is genuinely smaller and that's documented).
5. **Verify every phase has a gate criterion that references TDD test IDs.** Generic gates ("when it works") are insufficient.
6. **If any spec feature has no phase home** — BLOCKING failure. Add it before proceeding.
7. **If any red-flag language found** — BLOCKING failure.

---

## Key Reminders

- **Strategy provides ORDERING ONLY.** Never duplicate spec or TDD content. References, not copies.
- **Phases are execution order**, NOT importance tiers. Every phase ships. Later phases get same depth as early phases.
- **Critical dependencies must reflect codebase reality:** schema first; HTTP client before sync.rs; workspace wiring before web; main.rs integration phase-end.
- **Parallelism explicit:** mark which phases can run concurrently after Phase 1 (push and pull are independent; web can start early; overlays parallel).
- **Single-edit chokepoints flagged:** `main.rs`, `jobs/runner.rs`, `validation/policy.rs`, `validation/executor.rs` — only ONE bead at a time edits each.
- **Every phase has a gate** referencing TDD test IDs.
- **Bead prefix:** `l4`. **Existing epic:** `testproject1-4zq` (you may sub-divide into 4-6 epics in step 07; the strategy can suggest the sub-division).
- **Bead CLI:** `br`. **Sync:** `bin/br-sync`. **NEVER** raw `br sync`.
- **AgentMail project key:** `/Users/alinaaitova/Desktop/Testproject1`.
- **ZERO SCOPE DRIFT: every spec L4-FR must have a phase home.**
