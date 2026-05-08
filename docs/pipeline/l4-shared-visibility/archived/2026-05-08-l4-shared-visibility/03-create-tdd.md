# L4 Shared Visibility — 03 · Create TDD + Bootstrap

## First Action

Invoke the `/spec-to-tdd` skill.

---

## Your Mission

Generate the **Test-Driven Development specification** (SOT #2) and the **Bootstrap document** (environment prerequisites) for L4 Shared Visibility. The TDD must define every test that verifies the spec; the Bootstrap must define every environment prerequisite (Supabase project, daemon HTTP client, `apps/web` workspace wiring, `packages/ui` invariants, calibration corpus seeding).

The TDD becomes SOT #2 — beads will reference test IDs (`L4-T-...`) directly and embed test bodies. Coverage must be 100% from spec L4-FR IDs to TDD test IDs.

---

## Source Documents (Read In This Order)

### Primary Input (CREATE FROM THIS)
```
docs/strategy/layers/layer-4-shared-visibility.md           — 797 lines, L4-FR-X.Y.Z requirement IDs assigned in step 01 — READ FULLY
```

### Source-of-Truth Context
```
docs/strategy/layer-strategy.md                              — cross-layer ordering doc — SKIM
docs/strategy/cross-reference-index.md                       — every L4 cross-ref — SKIM
docs/strategy/layers/layer-3-operational-hardening.md        — what L3 ships that L4 builds on — SKIM (validation infrastructure §1.3, validation_verdicts §4)
docs/strategy/layers/layer-2-single-operator-desktop.md      — desktop side of the sync coupling — SKIM (decision capture, FreshnessBadge contract)
docs/strategy/layers/layer-6-request-pipeline-automation.md  — L6 expectations of L4 — SKIM
```

### Codebase Context (STUDY FOR PATTERNS)
```
crates/daemon/src/db/sqlite.rs                              — 600 lines, single connection wrapper. Test patterns for db setup live here.
crates/daemon/src/db/checkpoints.rs                         — 231 lines, parallel module to where sync.rs will live. Mirror its test setup.
crates/daemon/src/validation/policy.rs                      — 959 lines, L3 dry-run policy. L4 tests must not break existing policy tests.
crates/daemon/src/validation/executor.rs                    — 1217 lines, validator skill spawner. L4 tests extend, do not replace.
crates/daemon/src/validation/manifest_writer.rs             — 54 lines, hybrid manifest writer
crates/daemon/src/errors/manifest.rs                        — 386 lines, error_manifests writer (AGENT_SUSPECT_OUTPUT will be added here)
crates/daemon/src/jobs/runner.rs                            — 1027 lines, job runner (L4 extends with provisional promotion)
crates/daemon/src/ipc/handlers/decisions.rs                 — pending_operator_intents writer (L4 adds CAS flush)
crates/daemon/Cargo.toml                                    — confirm: NO http client today; bootstrap must add one
crates/daemon/tests/                                        — list every existing integration test file; L4 adds 4 new ones
migrations/0001_sqlite_state_initial.sql                    — L0 schema (no local_outbox)
migrations/0004_l2_pending_operator_intents.sql             — pending_operator_intents queue (L4 reconciles against pulled state)
migrations/0005_l3_operational_hardening.sql                — L3 validation_verdicts schema (L4 projects to Supabase as redacted summary)
packages/ui/                                                — components apps/web reuses
packages/manifests/render-error-manifest.ts                 — TS render path apps/web reuses
apps/desktop/src/pages/ContractList.tsx                     — desktop's analogous page; web mirrors page surface read-only
apps/desktop/src/pages/ContractDetail.tsx                   — same
apps/desktop/src/pages/DaemonStatus.tsx                     — same
apps/desktop/e2e/                                           — Playwright config + tests; web e2e mirrors structure
skills/validation/base/                                     — template the 4 L4 overlays follow (SKILL.md, result.schema.json, checkpoint.schema.json, canary.inputs.jsonl, golden.assertions.yaml, validator-rubric.yaml)
package.json (root)                                         — workspaces, lint rule banning desktop/tauri imports from packages/ui, bun scripts
AGENTS.md                                                   — Stage Validation Test Gate (mandatory full-test invocation pattern)
```

### Quality Bar (MATCH OR EXCEED)
```
docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-tdd.md         — 557 lines, 11 sections — TARGET DENSITY
docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-bootstrap.md   — 729 lines, 16 sections — TARGET DENSITY
docs/pipeline/l3-operational-hardening/archived/2026-l3-operational-hardening/03-create-tdd.md  — L3's analogous handoff
```

### L3 TDD section structure (target — verify the L4 TDD has these 11 sections in this order)
1. Scope
2. Non-Negotiable Test Principles
3. Test Pyramid
4. CI Tiering And Cadence
5. Shared Test Infrastructure
6. Planned File Inventory          (lists the 4 L4 test files)
7. Static CI Gates                 (must include the full Stage Validation Test Gate)
8. Detailed Test Catalog           (largest section — every L4-T test ID with full assertion)
9. Coverage Matrix                 (L4-FR → L4-T mapping; 100% required)
10. Coverage Summary
11. Notes For Strategy / Beads

### L3 Bootstrap section structure (target — verify the L4 Bootstrap has these 16 sections in this order)
1. Scope
2. Success Criteria Coverage Summary
3. Environment Prerequisites       (Supabase project, env vars, daemon HTTP client, apps/web wiring, calibration corpus dir)
4. Validation Script
5. Dependency And Manifest Additions   (`reqwest` add to crates/daemon/Cargo.toml; apps/web added to root package.json workspaces + scripts)
6. Migration Scaffold              (SQLite migration `migrations/{NEXT_NNNN}_l4_sync_state.sql` + 8 Supabase migrations under `infra/supabase/migrations/`)
7. Skill Directory Scaffold        (4 validator overlays under `skills/validation/{procurement-ingestion,supplier-enrichment,document-acquisition,product-extraction}/` each with the 6-file template)
8. Test Workspace Layout
9. Shared Support Extensions
10. Fixture And Corpus Bootstrap   (calibration corpora seeding at `tests/fixtures/calibration/{slice_id}/{clean,weird,mutated}/`)
11. Cron Configuration Seeding     (any new L4 cron entries — calibration sweep, lease heartbeat — or "no new entries" note)
12. Command Matrix                 (rust + bun + apps/web test commands)
13. Artifact Checklist
14. Structured Logging Requirements
15. False Positive Prevention      (calibration harness pairwise + cross-model audit; auto-demotion thresholds)
16. Notes For Implementation

---

## Cardinal Rule: ZERO SCOPE DRIFT

**If it's in the spec, it has a test.** No spec L4-FR can exit this batch without a corresponding L4-T test ID. You may IMPROVE test rigor (more assertions, cleaner setups, clearer fixtures); you may NOT silently drop coverage.

**Phases are execution ORDER**, not test importance. Every test ships. Later test files must have the same depth as early test files.

---

## Key Context From Codebase Research

1. **The TDD covers 4 test files** (per L4 spec §4.2):
   - `crates/daemon/tests/sqlite_schema.rs` — full schema creation/migration: all SQLite tables, all constraints, all indices, sync tracking columns populated, integer pence/bps verified
   - `crates/daemon/tests/sqlite_supabase_sync.rs` — push-pull round-trip: outbox creation, batch assembly, transactional RPC apply, row_version ordering, writer fencing, commit_group_id atomicity, equal-version retry no-op, pull cursor with hi-water fence, CAS conflict + command-ledger idempotency, active-writer lease, schema_epoch guard, provisional promotion buffer
   - `crates/daemon/tests/redaction_boundary.rs` — summary projection redaction tests for all 6 summary projections
   - `apps/web/e2e/contracts.spec.ts` — Playwright e2e: navigate to contracts list, verify read-only (no decision controls), keyset pagination, sync freshness indicator, Broadcast live updates, viewer credentials cannot access remote-owned tables (RLS boundary verification)

2. **Test ID convention:** `L4-T-{file_stem}-{NNN}` (3-digit sequence). Examples:
   - `L4-T-sqlite_schema-001` … `L4-T-sqlite_schema-NNN`
   - `L4-T-sqlite_supabase_sync-001` … (likely 60-100+ tests in this file)
   - `L4-T-redaction_boundary-001` … (one per summary projection × scenario)
   - `L4-T-contracts_spec-001` … (Playwright tests)

3. **Cumulative test file count:** 38 (L3) + 4 (L4) = 52 (per L4 spec §4.2 row).

4. **Stage Validation Test Gate** (AGENTS.md): the minimum quality gate is `cargo test -p daemon --bin daemon` PLUS iterating every integration test file in `crates/daemon/tests/*.rs`. The TDD must specify this in §3 (Test Pyramid) or §7 (Static CI Gates) so beads include this gate in their ACs.

5. **Bootstrap doc owns ENVIRONMENT, not test bodies.** It defines: Supabase project provisioning, env vars, role/grant SQL, the daemon HTTP client dependency addition (`reqwest` + `rustls-tls` + `json`), the `apps/web` workspace wiring (root `package.json` workspaces + scripts), the `infra/supabase/config.toml` skeleton, the calibration corpus directory layout, fixture seeding scripts.

6. **Net-new test infrastructure L4 needs:**
   - **Postgres test container** — for `sqlite_supabase_sync.rs` and `redaction_boundary.rs` to apply Supabase migrations and exercise RLS. Likely `testcontainers-rs` with Postgres + `supabase/postgres` image; or `pg_tap`-style schema tests. Decide and document.
   - **Supabase RPC stubs** — `apply_push_batch`, `update_operator_decision` exist as PG functions; tests must call them through the HTTP client (after the dependency is added) or directly via `psql`-style.
   - **RLS role test fixtures** — viewer/operator/admin/daemon roles; tests must be parameterized by role.
   - **Calibration corpora** — `tests/fixtures/calibration/{slice_id}/{clean,weird,mutated}/` with at least the sample sizes from spec §3.6.
   - **Mutation library** — implements all 9 mutation families from spec §3.6 (URL chain swap, page-kind confusion, locale/currency swap, pack/unit distortion, accessory/primary confusion, selective omission, count mismatch, fabricated evidence, cross-segment duplicates).
   - **Playwright with Supabase auth** — `apps/web/e2e/contracts.spec.ts` needs viewer-scoped credentials.

7. **{ENRICHMENT: spec metadata}** — after step 02 finishes, the spec will have N requirement IDs in the `L4-FR-X.Y.Z` range. Replace this marker with the actual count + ID range when running this handoff. The TDD's coverage matrix must map every L4-FR to at least one L4-T.

---

## Step-by-Step Workflow

### Step 0: Read Source Documents (MANDATORY — before anything)

1. `docs/strategy/layers/layer-4-shared-visibility.md` — **READ FULLY**
2. `docs/strategy/layer-strategy.md` — **SKIM** §6, §12
3. `docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-tdd.md` — **STUDY THE STRUCTURE** (11 sections; 1 Scope, 2 Non-Negotiable Test Principles, 3 Test Pyramid, 4 CI Tiering And Cadence, 5 Shared Test Infrastructure, 6 Planned File Inventory, 7 Static CI Gates, 8 Detailed Test Catalog (largest), 9 Coverage Matrix, 10 Coverage Summary, 11 Notes For Strategy / Beads)
4. `docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-bootstrap.md` — **STUDY THE STRUCTURE** (16 sections)
5. `crates/daemon/tests/` directory listing — every existing integration test file (L3 added 18 files, total 38 cumulative) — confirm format
6. `migrations/0005_l3_operational_hardening.sql` — see how L3 migration was structured; mirror in L4 sync migration
7. `packages/ui/` source files — see what's reusable from `apps/web`
8. `apps/desktop/e2e/playwright.config.ts` — copy structure to `apps/web/e2e/`

### Step 1: Invoke `/spec-to-tdd`

**Inputs:**
- Source spec: `docs/strategy/layers/layer-4-shared-visibility.md`
- TDD output: `docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-tdd.md`
- Bootstrap output: `docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-bootstrap.md`

**Key arguments/context to provide the skill:**
- Test ID convention: `L4-T-{file_stem}-{NNN}`
- Test files (4): `sqlite_schema.rs`, `sqlite_supabase_sync.rs`, `redaction_boundary.rs`, `apps/web/e2e/contracts.spec.ts`
- Test framework: `cargo test` (Rust) + Playwright (TS e2e)
- Stage Validation Test Gate (from AGENTS.md): cargo test -p daemon --bin daemon + iterate `crates/daemon/tests/*.rs`
- Bead CLI: `br`
- Project key: `/Users/alinaaitova/Desktop/Testproject1`
- Quality bar: L3 TDD (557 lines, 11 sections) + L3 Bootstrap (729 lines, 16 sections)

**Coverage targets:**
- Every spec L4-FR-X.Y.Z gets ≥1 L4-T test ID
- Every §1.9 invariant gets a dedicated test
- Every §4.1 DoD row maps to ≥1 test
- Every §4.3 test scenario row maps to ≥1 test
- Every §3.6 mutation family gets a test in the calibration harness section
- Every of the 6 summary projections gets a redaction test
- Every of the 4 RLS roles gets RLS-policy tests across the 7 table categories
- Every of the 4 validator overlays gets canary tests + golden journeys

### Step 2: Feature-Specific Guidance

While the skill runs, pay attention to:

- **Sync engine tests must mock or containerize Supabase.** Not just unit-test the outbox writer in isolation — the integration tests must round-trip through `apply_push_batch`. Decide: testcontainers + supabase/postgres image, or local Supabase CLI start. Document the choice in Bootstrap §3 (Environment Prerequisites).
- **CAS tests must exercise the command ledger across two transactions.** Test scenario: command 1 lands, command 2 lands, command 1 retried — must return stored conflict result, not re-evaluate. This is a multi-step test; structure carefully.
- **Writer fencing tests must simulate two daemons.** One holds lease, increments writer_epoch; second daemon attempts push with stale epoch — rejected. Stale daemon recovers, attempts push — rejected with current epoch.
- **Equal-version retry has 4 cases** (§1.3) — each gets its own test ID with assertion semantics.
- **Hi-water fenced pull** must be tested with concurrent remote changes during pagination. Inject a write between `hiwater` snapshot and `cursor` advance — verify no skip/duplicate.
- **Provisional promotion tests must cover both branches:** approved-promotes-to-push-owned, rejected-stays-local-only-with-error-manifest.
- **Per-slice enforcement readiness tests** must exercise: meets-thresholds (enforcement-eligible), missing-canary-coverage (rejected), Wilson-bound below 70% (rejected), demotion-trigger-active (rejected → demoted).
- **RLS tests must use 4 different DB connections** with different roles (viewer/operator/admin/daemon). Verify viewer cannot SELECT from remote-owned config tables; operator cannot UPDATE `operator_decision_heads` directly (only via RPC); daemon can SELECT tombstones (`deleted_at IS NOT NULL`) for pull purposes.
- **Redaction tests** must verify: no operator identity in `operator_decision_summaries`; no full company profile in any summary; no full error context in `error_manifest_summaries`; no full validator reasoning in `validation_verdict_summaries`; no full job context in `workflow_job_summaries`.
- **Web e2e tests** must verify: viewer credentials work for SELECT on summaries but FAIL for SELECT on remote-owned config tables (RLS boundary verification at the network level, not just UI hiding).
- **Calibration harness tests** must implement the 9 mutation families AND the pairwise challenge sets AND the cross-model audit sampling AND the override feedback loop wiring.
- **Stage Validation Test Gate** must be in §7 Static CI Gates: `cargo fmt --check && cargo check --all-targets && cargo clippy --all-targets -- -D warnings && cargo test -p daemon --bin daemon && for t in crates/daemon/tests/*.rs; do cargo test --test "$(basename "$t" .rs)" || exit 1; done && bun run test && (cd apps/web && bun run test:e2e)`

### Step 3: Verify Output

After the skill completes, verify:

- [ ] `docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-tdd.md` exists
- [ ] `docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-bootstrap.md` exists
- [ ] TDD has 11 sections matching L3 quality bar
- [ ] TDD §6 Planned File Inventory lists exactly 4 test files (matching spec §4.2)
- [ ] TDD §8 Detailed Test Catalog has every test ID with full assertion description
- [ ] TDD §9 Coverage Matrix maps every L4-FR-X.Y.Z to ≥1 L4-T
- [ ] Bootstrap has 16 sections matching L3 quality bar
- [ ] Bootstrap §3 Environment Prerequisites lists Supabase project, env vars, role grants, daemon HTTP client (`reqwest` add), `apps/web` workspace wiring, calibration corpus directory
- [ ] Bootstrap §6 Migration Scaffold covers BOTH the SQLite migration `migrations/0012_l4_sync_state.sql` AND the 8 Supabase migrations under `infra/supabase/migrations/`
- [ ] Bootstrap §7 Skill Directory Scaffold covers the 4 validator overlays each as `skills/validation/{slug}/` with the 6-file template
- [ ] No "post-MVP", "Phase 2+", "stretch goal", "nice to have", "deferred", "future iteration" language anywhere
- [ ] Test count totals: ≥80 cumulative new test IDs across the 4 files (L3 added ~150+ tests in 18 files; L4 with 4 files but covering RLS × 4 roles × 7 categories + redaction × 6 projections + push-pull × dozens of scenarios + e2e × 10+ should still hit ≥80)

### Step 4: Follow-Up Pass (Sub-Optimization)

> Send this as a follow-up message in the same session after the main work is complete.

ULTRATHINK.

Re-read the TDD and Bootstrap with fresh eyes and answer:

1. **Open `crates/daemon/src/validation/executor.rs` (1217 lines).** The L3 executor produces verdicts but never quarantines results. L4 §3.4 says rejected results in enforcement mode are quarantined as `AGENT_SUSPECT_OUTPUT`. Walk the existing `ExecutorOutcome` / verdict-emission code — does the TDD have tests for the L4 extension that exercise BOTH paths (quarantine in enforcement, advance in dry-run)? If not, add tests `L4-T-sqlite_supabase_sync-{N}` covering: (a) enforcement-active slice + validator approves → outbox entry created, stage head advances; (b) enforcement-active slice + validator rejects → no outbox entry, error manifest created with type AGENT_SUSPECT_OUTPUT, stage head NOT advanced; (c) enforcement-inactive (dry-run) slice + validator rejects → outbox entry STILL created, verdict logged, stage head advances normally.

2. **Open `migrations/0004_l2_pending_operator_intents.sql`.** The table is the L2 source of operator decisions. L4 §1.4 says "after pull, reconcile pending_operator_intents queue: latest-per-process coalescing, pull-first-then-replay." The TDD must have tests that exercise the reconciliation. Does it? Verify there's a test `L4-T-sqlite_supabase_sync-{N}` covering: 3 unsuperseded intents for same `process_id` → after pull → only the most-recent-by-`created_at` intent flushes through CAS RPC; the older two get marked superseded.

3. **Open `crates/daemon/src/jobs/runner.rs` (1027 lines).** The runner is the single enqueue boundary. L4 adds the provisional promotion path: on enforcement-active slice, worker output goes to `provisional_worker_outputs` (or chosen staging mechanism) instead of directly to push-owned tables. Does the TDD have a test that simulates the runner emitting a worker job, confirming the staging happens, and confirming the validator job follows? If not, add `L4-T-sqlite_supabase_sync-{N}` covering the runner-validator-promotion flow end to end.

4. **Open `package.json` (root) workspaces array.** Currently includes `packages/*` and `apps/desktop`. L4 must add `apps/web`. Does the Bootstrap doc have a §5 Dependency And Manifest Additions step that specifies the exact root `package.json` edit? Does it cover the `bun run build` script extension to include the web build? Does it cover the lint rule (no `@tauri-apps/api` / `apps/desktop` / `crates/daemon` / `supabase` imports from `packages/ui`) is preserved when `apps/web` is added?

5. **Open `crates/daemon/Cargo.toml`.** Currently no HTTP client. The Bootstrap doc must specify the exact dependency lines to add: `reqwest = { version = "0.12", default-features = false, features = ["rustls-tls", "json"] }`. Does §5 of the Bootstrap have this exact line? If "tokio-postgres" is the alternative chosen, document the rationale and the dependency lines instead.

6. **Open `skills/validation/base/SKILL.md`.** This is the template for the 4 L4 overlays. Verify the Bootstrap §7 Skill Directory Scaffold lists each overlay with exactly the same 6 files: `SKILL.md`, `result.schema.json`, `checkpoint.schema.json`, `canary.inputs.jsonl`, `golden.assertions.yaml`, `validator-rubric.yaml`. Verify the SKILL.md frontmatter for each overlay declares `validator_overlay_compatible_versions` (per L3 §3.5 forward-compat).

7. **Walk the §9 Coverage Matrix.** Pick 5 L4-FR rows at random. For each, verify there's at least one L4-T test ID listed AND that test ID exists in the §8 Detailed Test Catalog with full assertion description. If any are missing or stub-only, fix.

8. **Re-scan TDD + Bootstrap for scope-drift language.** Search for: `post-MVP`, `Phase 2+`, `nice to have`, `deferred`, `stretch goal`, `simplified for now`, `can be added later`, `eventually`, `for now`. Any hit in load-bearing positions = BLOCKING failure.

9. **Bootstrap §11 Cron Configuration Seeding** — this is a copy from L3 in case L4 also seeds cron. Verify it's adapted: does L4 add new cron entries (e.g., calibration-harness sweep, lease-heartbeat sweep)? If yes, list them. If no, drop the section or reuse L3's verbatim with a "no new entries" note.

10. **Bootstrap §15 False Positive Prevention** — for L4, the principal false-positive risk is the calibration harness. Verify §15 has: explicit pairwise-challenge guard, explicit cross-model audit sampling cadence, explicit override-rate threshold for auto-demotion. If absent, add.

### Step 5: Scope-Preservation Check (MANDATORY — BLOCKING)

After the sub-optimization, perform this check:

1. **Enumerate** every L4-FR ID from the spec (post-step-02). For each, verify the TDD §9 Coverage Matrix lists ≥1 L4-T test ID.
2. **Enumerate** every §1.9 invariant (14). For each, verify the TDD has a dedicated invariant test.
3. **Enumerate** every §4.3 test scenario (17). For each, verify a corresponding TDD test exists.
4. **Enumerate** every of the 6 summary projections. For each, verify a redaction test in `redaction_boundary.rs`.
5. **Enumerate** every of the 4 validator overlays. For each, verify canary inputs + golden assertions referenced in the TDD.
6. **Scan** TDD + Bootstrap for red-flag language: `post-MVP`, `Phase 2+`, `stretch goal`, `nice to have`, `deferred`, `future iteration`, `out of scope`, `simplified for initial`, `can be added later`, `eventually`, `for now`.
7. **If any spec feature has zero test coverage** — BLOCKING failure. Add tests before proceeding.
8. **If any red-flag language is found** in load-bearing positions — BLOCKING failure.

---

## Key Reminders

- **Test ID format:** `L4-T-{file_stem}-{NNN}` consistently. The file_stem matches the test file name without extension (e.g., `sqlite_supabase_sync.rs` → `L4-T-sqlite_supabase_sync-001`).
- **Bootstrap covers HOW to set up the environment, not WHAT to build.** Resist the temptation to duplicate spec content. The Bootstrap should reference the spec, not re-explain it.
- **The TDD tests Stage Validation Test Gate** — every test file is exercised through `cargo test --test "$(basename ... .rs)"` iteration, not just `cargo test -p daemon --bin daemon`.
- **`apps/web` is net-new.** The Bootstrap must specify the entire workspace wiring: root `package.json` edits, new `tsconfig.json`, new `playwright.config.ts`, Supabase client + auth setup.
- **`infra/supabase/` is net-new.** The Bootstrap must specify CLI version, project init, migration scaffold, RLS policy testing approach.
- **Daemon HTTP client is net-new.** The Bootstrap must specify the exact `reqwest` (or alternative) dependency line and the rationale.
- **Calibration corpora live somewhere on disk.** The Bootstrap must specify the path: `tests/fixtures/calibration/{slice_id}/{clean,weird,mutated}/` (or chosen alternative). Bead authors will copy real run manifests into this tree.
- **The Stage Validation Test Gate from AGENTS.md is a single command set; reference it verbatim.**
- **AgentMail project key:** `/Users/alinaaitova/Desktop/Testproject1`. Reference this key in any operational guidance.
- **ZERO SCOPE DRIFT: every L4-FR must have at least one L4-T. No silent coverage drops.**
- **Bead CLI:** `br`. Sync command: `bin/br-sync` (or `br sync --flush-only`). Never raw `br sync` (which may run git commands).
