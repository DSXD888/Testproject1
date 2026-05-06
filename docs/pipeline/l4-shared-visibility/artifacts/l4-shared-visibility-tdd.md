# L4 Shared Visibility - TDD Specification

## 1. Scope

**Primary source of truth:** `docs/strategy/layers/layer-4-shared-visibility.md`

**Supporting context:**
- `docs/strategy/layer-strategy.md` sections 6 and 12
- `docs/strategy/cross-reference-index.md`
- `docs/strategy/layers/layer-3-operational-hardening.md`
- `docs/strategy/layers/layer-2-single-operator-desktop.md`
- `docs/strategy/layers/layer-6-request-pipeline-automation.md`
- `docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-tdd.md`
- `docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-bootstrap.md`
- `crates/daemon/src/db/sqlite.rs`
- `crates/daemon/src/db/checkpoints.rs`
- `crates/daemon/src/validation/policy.rs`
- `crates/daemon/src/validation/executor.rs`
- `crates/daemon/src/validation/manifest_writer.rs`
- `crates/daemon/src/errors/manifest.rs`
- `crates/daemon/src/jobs/runner.rs`
- `crates/daemon/src/ipc/handlers/decisions.rs`
- `migrations/0001_sqlite_state_initial.sql`
- `migrations/0004_l2_pending_operator_intents.sql`
- `migrations/0005_l3_operational_hardening.sql`
- `packages/ui/`
- `packages/manifests/render-error-manifest.ts`
- `apps/desktop/src/pages/ContractList.tsx`
- `apps/desktop/src/pages/ContractDetail.tsx`
- `apps/desktop/src/pages/DaemonStatus.tsx`
- `apps/desktop/e2e/`
- `skills/validation/base/`
- `package.json`
- `AGENTS.md` stage validation gate

**Purpose of this document:** define the complete test inventory that must exist before Layer 4 implementation can be marked complete. This is SOT #2 for L4 Shared Visibility. Beads must reference the stable test IDs in this document.

**Spec metadata:** the validated L4 spec contains `147` unique `L4-FR-*` requirement IDs, ranging from `L4-FR-1.3.1` through `L4-FR-5.4.3`.

**Coverage target:** every one of the `147` L4 requirement IDs maps to at least one `L4-T-*` test ID. Every L4 invariant in section 1.9, every Definition of Done row in section 4.1, every scenario in section 4.3, every mutation family in section 3.6, every summary projection, every RLS role/category boundary, and every validator overlay has direct test coverage.

**Planned inventory:** `158` total checks
- `8` static CI gates
- `150` runnable tests
- exactly `4` new test files from L4 spec section 4.2

## 2. Non-Negotiable Test Principles

- Use Cargo only for Rust tests and Bun only for TypeScript/web tests.
- Do not mock SQLite. Schema, outbox, transactional promotion, CAS reconciliation, and calibration tests use real temp-file SQLite through the existing `StateDb` path.
- Do not replace the L3 validation policy or executor in tests. L4 tests extend the existing policy/executor surfaces and prove both dry-run and enforcement behavior.
- Sync tests must exercise a real Supabase-compatible stack, including Postgres RLS, PostgREST RPC calls, Auth-scoped credentials where relevant, and Realtime Broadcast for web E2E.
- Push-owned projection mutation must go through `apply_push_batch(batch_json, writer_epoch)`. Direct daemon upsert tests are invalid except when asserting that direct upserts are not used.
- Operator decisions must mutate through `update_operator_decision(process_id, disposition, base_revision, command_id)`. Direct UPDATE paths are tested only as denied boundary checks.
- Equal-version retry semantics require four explicit cases: equal hash no-op, equal hash mismatch hard conflict, next version apply, older/larger jump reject.
- Pull tests must use a hi-water fence with concurrent writes injected between pagination pages.
- Enforcement-active worker output must be staged in `provisional_worker_outputs` before validator approval. Tests that write rejected results directly to push-owned tables are invalid.
- RLS tests must use role-specific credentials or role-scoped DB sessions for `viewer`, `operator`, `admin`, and `daemon`.
- Web read-only is a database security boundary, not only UI hiding. Playwright tests must verify network-level denial of remote-owned tables and mutation RPCs with viewer credentials.
- Summary projection tests must assert sensitive-field absence by name and by seeded sentinel values.
- Calibration tests must include clean, weird-valid, mutated, pairwise, cross-model audit, override feedback, Wilson lower-bound, rule-of-three, and demotion-threshold coverage.
- Every Rust and Playwright test name must include the stable `L4-T-*` ID verbatim.
- No `.skip`, `#[ignore]`, `test.todo`, empty test bodies, existence-only assertions, vague "works" assertions, or tests that pass against hardcoded no-op implementations.
- Run `ubs` on changed files before committing implementation that satisfies this TDD.

## 3. Test Pyramid

| Layer | Purpose | Allowed Medium | Disallowed Shortcuts | Primary Evidence |
|------|---------|----------------|----------------------|------------------|
| Static Gates | build, lint, format, package wiring, exact file inventory, forbidden imports, full stage gate | Cargo, Bun, `rg`, `ubs` | manual inspection | command exit status, inventory output |
| Unit | pure calibration math, freshness helper thresholds, mutation library, batch slicing decisions | Rust pure functions, Vitest pure helpers, in-memory inputs | fake policy replacement, approximate assertions | exact bps, exact enum, exact ordering |
| Integration | SQLite migrations, outbox transactionality, local sync state, Supabase migrations, RPC application, RLS, CAS, lease fencing | temp-file SQLite, local Supabase CLI Docker stack, PostgREST RPC through `reqwest`, role-scoped sessions | direct daemon table upserts, mock-only Supabase | DB rows, RPC payload/response, RLS denials |
| E2E | web read-only portal, list/detail/status flows, Broadcast invalidation, viewer credential boundary | Playwright against `apps/web`, local Supabase stack, seeded projection data | DOM-only security checks, service-role browser keys | screenshots/traces, network responses, Supabase logs |
| Manual Smoke | two-user milestone signoff | one desktop writer plus one web viewer against seeded Supabase | replacing required automated checks | smoke diary and linked automated test run |

**Stage Validation Test Gate:** any L4 stage validation manifest must run the complete gate below. The daemon binary unit tests alone are insufficient because they exclude integration test files.

```bash
cargo fmt --check
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test -p daemon --bin daemon
for t in crates/daemon/tests/*.rs; do
  cargo test --test "$(basename "$t" .rs)" || exit 1
done
bun run test
(cd apps/web && bun run test:e2e)
```

## 4. CI Tiering And Cadence

| Tier | Budget | Cadence | Includes |
|------|--------|---------|----------|
| `T1` | `< 3 min` | every commit | static gates, SQLite schema smoke, pure calibration math, mutation library, freshness helper tests, package import boundary |
| `T2` | `< 10 min` | every commit on runners with Docker | SQLite + local Supabase schema/RLS/RPC integration, outbox transactionality, CAS command ledger, writer lease smoke |
| `T3` | `< 25 min` | pre-merge and nightly | full sync replay, hi-water concurrent pull, epoch repair reseed, overlay canaries, calibration corpus sweeps |
| `T4` | `< 45 min` | milestone signoff | Playwright web E2E with Supabase Auth, Realtime Broadcast, two-user desktop-seeded/web-viewer smoke |

**Tier rule:** tests that start or reset the Supabase local stack are `T2` or higher. Tests that require Realtime Broadcast and browser credentials are `T4`.

## 5. Shared Test Infrastructure

| Harness / Fixture | Scope | Purpose | Setup Requirements |
|------------------|-------|---------|--------------------|
| `L4TestStateDb` | `T1` / `T2` / `T3` | Extends existing `TestStateDb` by opening temp-file SQLite at the L4 schema epoch and seeding push-owned rows, pending intents, validation verdicts, and stage heads | `StateDb::open`, migrations through `0012_l4_sync_state.sql`, WAL enabled |
| `SqliteSchemaAssert` | `T1` | Introspects SQLite tables, columns, checks, indexes, PRAGMA user_version, and integer pence/bps columns | migrated temp DB, `PRAGMA table_info`, `PRAGMA index_list` |
| `SupabaseLocalStack` | `T2` / `T3` / `T4` | Starts the local Supabase CLI Docker stack, applies `infra/supabase/migrations/*.sql`, seeds roles and JWTs, exposes PostgREST/Auth/Realtime URLs | Docker, Supabase CLI, project `infra/supabase/config.toml` |
| `SupabaseRpcClient` | `T2` / `T3` | Calls `apply_push_batch` and `update_operator_decision` through HTTP using the daemon `reqwest` client path | local API URL, role-scoped keys, service role only in daemon/test harness |
| `RoleMatrixFixture` | `T2` | Opens viewer/operator/admin/daemon role sessions and checks SELECT/INSERT/UPDATE/RPC permissions | seeded roles, RLS enabled, sentinel rows |
| `OutboxHarness` | `T2` / `T3` | Creates push-owned mutations through `StateDb::with_write_transaction`, assembles envelopes, persists `sync_batches`, and asserts retry identity | migrated SQLite, seeded process/candidate/quote rows |
| `PullCursorHarness` | `T2` / `T3` | Seeds remote-owned domains with monotonic `remote_change_seq`, injects concurrent writes during pagination, and verifies cursor advancement | Supabase local stack, per-domain sequences/triggers |
| `CasCommandHarness` | `T2` / `T3` | Exercises pending-intent coalescing, CAS RPC conflicts, command ledger replay, retention GC, and post-GC replay | `pending_operator_intents`, `operator_decision_heads`, `operator_decision_commands` |
| `WriterLeaseHarness` | `T2` / `T3` | Simulates two daemon executor IDs, lease heartbeat, expiry, writer_epoch increments, stale writer rejection, and rehome repair | `active_writer_lease`, `viewer_sync_status` |
| `ProjectionGenerationHarness` | `T3` | Exercises schema_epoch pause, epoch repair, generation ledger, current-row reseed, stale outbox discard after publish | `projection_generations`, projection tables, `viewer_sync_status` |
| `EnforcementHarness` | `T2` / `T3` | Drives worker -> provisional output -> validator verdict -> approve/reject/override promotion paths through existing runner/executor boundaries | `provisional_worker_outputs`, `validation_verdicts`, `error_manifests`, stage heads |
| `CalibrationHarness` | `T1` / `T3` | Seeds corpora, mutation families, pairwise cases, cross-model audit rows, override feedback, Wilson/rule-of-three inputs, demotion thresholds | `tests/fixtures/calibration`, `validation_calibration_cases`, `validation_incident_clusters`, `enforcement_readiness` |
| `OverlaySkillFixture` | `T3` | Materializes the four domain overlays with the same six-file contract as `skills/validation/base/` and runs canary/golden journeys | overlay skill dirs, result/checkpoint schemas, canary inputs, golden assertions |
| `RedactionSentinelFixture` | `T2` | Seeds sensitive operator identity, company profile, supplier raw data, tender metadata, validator reasoning, full job context, command history | summary projection source tables plus summaries |
| `WebPortalHarness` | `T4` | Runs `apps/web` with viewer credentials, seeded Supabase data, Broadcast channels, and Playwright traces/screenshots | Bun, Next.js app, local Supabase stack, viewer auth |

## 6. Planned File Inventory

| File | Count | Coverage |
|------|-------|----------|
| `crates/daemon/tests/sqlite_schema.rs` | `24` tests | SQLite L4 migration, local sync tables, indices, constraints, sync tracking, integer money/bps, migration registry, Supabase migration inventory |
| `crates/daemon/tests/sqlite_supabase_sync.rs` | `80` tests | Outbox, batch assembly, transactional RPC apply, retry semantics, pull protocol, CAS ledger, writer fencing, schema epoch guard, degraded mode, provisional promotion, enforcement readiness, overlays, calibration |
| `crates/daemon/tests/redaction_boundary.rs` | `26` tests | RLS role matrix, summary projection redaction, raw-table denial, tombstone visibility, mutation RPC denial, browser credential boundary |
| `apps/web/e2e/contracts.spec.ts` | `20` tests | Read-only web portal list/detail/status, keyset pagination, freshness states, Broadcast invalidation, viewer credential network boundary, two-user visibility |

## 7. Static CI Gates

| Test ID | Tier | Fixtures | Exact Assertions | Requirements |
|--------|------|----------|------------------|--------------|
| `CI-L4-001` | `T1` | repo workspace | `cargo check --all-targets` exits `0` after L4 sync modules, reqwest dependency, migrations, and tests are added | `L4-FR-1.10.1`, `L4-FR-5.4.1` |
| `CI-L4-002` | `T1` | repo workspace | `cargo clippy --all-targets -- -D warnings` exits `0`; no L4 production or test code ships with warnings | all daemon requirements |
| `CI-L4-003` | `T1` | repo workspace | `cargo fmt --check` exits `0` | supporting invariant |
| `CI-L4-004` | `T1` | full stage gate | Runs `cargo test -p daemon --bin daemon`, then iterates every `crates/daemon/tests/*.rs` file with `cargo test --test "$(basename "$t" .rs)"`; any skipped integration file fails the gate | all Rust requirements |
| `CI-L4-005` | `T1` | Bun workspace | `bun run test` exits `0` and includes `packages/ui`, `packages/intelligence`, `packages/manifests`, `apps/desktop`, and `apps/web` test paths | `L4-FR-3.1.1`, `L4-FR-5.4.3` |
| `CI-L4-006` | `T1` | web workspace | `(cd apps/web && bun run test:e2e)` exits `0` against the local Supabase stack and captures Playwright traces/screenshots on failure | `L4-FR-3.1.1`-`L4-FR-3.3.4` |
| `CI-L4-007` | `T1` | repository inventory | `migrations/0012_l4_sync_state.sql`, all eight `infra/supabase/migrations/*.sql`, `infra/supabase/config.toml`, four overlay skill dirs, and `apps/web` seven-file surface exist exactly as declared | `L4-FR-1.10.1`, `L4-FR-2.6.1`-`L4-FR-2.6.11`, `L4-FR-3.5.1`-`L4-FR-3.5.4` |
| `CI-L4-008` | `T1` | policy scan | No new Rust unsafe code, no ignored/skipped/TODO tests, no `bd` terminology in L4 implementation docs, no forbidden `packages/ui` imports from Tauri, desktop, daemon, Supabase, or transport modules; `ubs` exits `0` on changed files | `L4-FR-5.4.2`, `L4-FR-5.4.3` |

## 8. Detailed Test Catalog

### 8.1 `sqlite_schema.rs`

| Test ID | Tier | Fixtures | Exact Assertions | Requirements |
|--------|------|----------|------------------|--------------|
| `L4-T-sqlite_schema-001` | `T1` | `L4TestStateDb` | opening a new state DB applies `migrations/0012_l4_sync_state.sql`, records it in `schema_migrations`, and persists `PRAGMA user_version = 9` through the existing `open_state_db` path | `L4-FR-1.7.1`, `L4-FR-2.6.11` |
| `L4-T-sqlite_schema-002` | `T1` | `SqliteSchemaAssert` | `local_outbox` exists with `local_seq`, `executor_id`, `entity_kind`, `entity_id`, `op`, `row_version`, `stage_run_id`, `schema_epoch`, `commit_group_id`, nullable `writer_epoch`, `payload_json`, `content_hash`, `created_at`; `local_seq` is monotonic | `L4-FR-1.3.8`, `L4-FR-2.6.11` |
| `L4-T-sqlite_schema-003` | `T1` | `SqliteSchemaAssert` | `sync_batches` exists with persisted batch envelope fields, in-flight status, local sequence range, immutable `writer_epoch`, `schema_epoch`, `stage_run_id`, `commit_group_id` coverage, and retry payload hash | `L4-FR-1.3.2`, `L4-FR-1.3.3` |
| `L4-T-sqlite_schema-004` | `T1` | `SqliteSchemaAssert` | `pull_cursors` has independent primary keys for `shared_source_configs`, `shared_company_profile`, and `operator_decision_heads`, each storing last applied `remote_change_seq` | `L4-FR-1.4.1` |
| `L4-T-sqlite_schema-005` | `T1` | `SqliteSchemaAssert` | `provisional_worker_outputs` has `worker_job_id`, `payload_json`, `result_contract_blob_path`, `created_at`, and `validation_status` constrained to `pending`, `approved`, `rejected`; `worker_job_id` is indexed | `L4-FR-1.3.9`, `L4-FR-3.4.2` |
| `L4-T-sqlite_schema-006` | `T1` | `SqliteSchemaAssert` | `enforcement_readiness` is keyed by deterministic `slice_id` plus the enforcement tuple and stores state, counts, Wilson/rule-of-three bps, demotion reason, and threshold config snapshot | `L4-FR-3.4.3`, `L4-FR-3.4.4`, `L4-FR-3.4.6` |
| `L4-T-sqlite_schema-007` | `T1` | `SqliteSchemaAssert` | `projection_generations` exists with `status in ('started','published','failed')`, schema epoch, timestamps, max local sequence before reseed, remote ack time, and error JSON | `L4-FR-5.2.1` |
| `L4-T-sqlite_schema-008` | `T1` | `SqliteSchemaAssert` | `validation_calibration_cases` exists with corpus kind, expected/actual verdict, ground truth, mutation family, pair ID, model family, source artifact path, created/adjudicated timestamps | `L4-FR-3.6.1`-`L4-FR-3.6.17` |
| `L4-T-sqlite_schema-009` | `T1` | `SqliteSchemaAssert` | `validation_incident_clusters` stores incident signature, dominant failure family, affected slice IDs, candidate mutation template JSON, and override linkage | `L4-FR-3.6.16` |
| `L4-T-sqlite_schema-010` | `T1` | `SqliteSchemaAssert` | all 20 push-owned SQLite logical tables retain `owner_id`, `row_version`, `last_local_seq`, `stage_run_id`, `created_at`, and `updated_at` where specified by the canonical L0-L3 schema | `L4-FR-2.1.1`-`L4-FR-2.1.20` |
| `L4-T-sqlite_schema-011` | `T1` | `SqliteSchemaAssert` | monetary columns ending in `_pence` and rate/confidence/margin columns ending in `_bps` are stored as INTEGER in SQLite and reject non-integer fixture values through insert assertions | `L4-FR-2.1.1`-`L4-FR-2.1.20` |
| `L4-T-sqlite_schema-012` | `T1` | L0-only temp migration | applying only `migrations/0001_sqlite_state_initial.sql` does not create `local_outbox`; applying L4 migration creates it, preserving the L4 ownership rule | `L4-FR-1.3.1`, `L4-FR-2.6.11` |
| `L4-T-sqlite_schema-013` | `T1` | `L4TestStateDb` | reopening an already migrated DB does not duplicate local sync tables, seed rows, indices, or migration log entries | `L4-FR-2.6.11` |
| `L4-T-sqlite_schema-014` | `T1` | `SqliteSchemaAssert` | required indices exist for `local_outbox` dispatch ordering, `sync_batches` in-flight lookup, `pull_cursors` domain lookup, `provisional_worker_outputs.worker_job_id`, and `enforcement_readiness.slice_id` | `L4-FR-1.3.2`, `L4-FR-1.3.9`, `L4-FR-3.4.6` |
| `L4-T-sqlite_schema-015` | `T1` | `SqliteSchemaAssert` | `local_outbox` constrains `op` to insert/update, stores non-empty `content_hash`, and rejects rows missing `commit_group_id`, `stage_run_id`, or `schema_epoch` | `L4-FR-1.3.4`, `L4-FR-1.3.5`, `L4-FR-1.9.7`, `L4-FR-1.9.8` |
| `L4-T-sqlite_schema-016` | `T1` | `SqliteSchemaAssert` | `pending_operator_intents` retains the L2 queue shape including `base_remote_revision`, `idempotency_key`, `status`, `superseded_by_intent_id`, `queued_at`, and latest-intent index needed by L4 reconciliation | `L4-FR-1.4.4`, `L4-FR-1.5.1` |
| `L4-T-sqlite_schema-017` | `T1` | `SqliteSchemaAssert` | `validation_verdicts` still has L3 verdict fields plus override fields and can be joined to provisional output by `worker_job_id` without breaking existing L3 policy tests | `L4-FR-3.4.2`, `L4-FR-3.6.16` |
| `L4-T-sqlite_schema-018` | `T1` | repo inventory | `infra/supabase/config.toml` and exactly eight migrations `0001` through `0008` exist under `infra/supabase/migrations/` | `L4-FR-2.6.1`-`L4-FR-2.6.9` |
| `L4-T-sqlite_schema-019` | `T2` | `SupabaseLocalStack` | applying `0001_projection_tables.sql` creates all 20 projection tables plus the three junction tables with required sync tracking fields and no foreign keys to local-only tables | `L4-FR-2.1.1`-`L4-FR-2.1.20`, `L4-FR-2.4.1`-`L4-FR-2.4.3`, `L4-FR-2.6.1` |
| `L4-T-sqlite_schema-020` | `T2` | `SupabaseLocalStack` | applying `0003_summary_tables.sql` and `0008_decision_summaries.sql` creates the six summary projections and omits raw context columns by schema, including company profile, command history, validator reasoning, and full error context | `L4-FR-2.2.1`-`L4-FR-2.2.6`, `L4-FR-2.6.3`, `L4-FR-2.6.8` |
| `L4-T-sqlite_schema-021` | `T2` | `SupabaseLocalStack` | applying `0002_remote_owned_tables.sql` creates remote-owned tables, per-domain `remote_change_seq` sequences/triggers, `remote_revision`, `deleted_at`, and role-compatible tombstone columns | `L4-FR-1.4.2`, `L4-FR-2.3.1`-`L4-FR-2.3.3`, `L4-FR-2.6.2` |
| `L4-T-sqlite_schema-022` | `T2` | `SupabaseLocalStack`, `RoleMatrixFixture` | applying `0004_rls_policies.sql` enables RLS on every table category and grants only the matrix permissions declared for viewer, operator, admin, and daemon | `L4-FR-1.9.13`, `L4-FR-2.5.1`-`L4-FR-2.5.10`, `L4-FR-2.6.4` |
| `L4-T-sqlite_schema-023` | `T2` | `SupabaseLocalStack` | applying `0006_schema_metadata.sql` creates `schema_metadata` for schema epoch only and a dedicated `active_writer_lease` table with `org_id`, holder, expiry, and `writer_epoch` | `L4-FR-1.6.2`, `L4-FR-1.7.1`, `L4-FR-2.6.6` |
| `L4-T-sqlite_schema-024` | `T1` | repository scripts | root workspace wiring includes `apps/web` through `apps/*`, root scripts actually execute web build/test/lint, and daemon `Cargo.toml` contains the `reqwest` dependency with `rustls-tls` and `json` | `L4-FR-3.1.1`, `L4-FR-5.4.1`, `L4-FR-5.4.3` |

### 8.2 `sqlite_supabase_sync.rs`

| Test ID | Tier | Fixtures | Exact Assertions | Requirements |
|--------|------|----------|------------------|--------------|
| `L4-T-sqlite_supabase_sync-001` | `T2` | `L4TestStateDb`, `OutboxHarness` | a push-owned row mutation and its `local_outbox` row commit in the same SQLite transaction with one matching outbox entry and shared `commit_group_id` | `L4-FR-1.3.1`, `L4-FR-1.9.2` |
| `L4-T-sqlite_supabase_sync-002` | `T2` | `L4TestStateDb`, `OutboxHarness` | if a push-owned mutation path omits the matching outbox insert, the transaction returns an error or rolls back so neither data row nor outbox row remains | `L4-FR-1.3.1`, `L4-FR-1.9.2` |
| `L4-T-sqlite_supabase_sync-003` | `T2` | `EnforcementHarness` | an enforcement-active rejected provisional output creates no push-owned mutation and no outbox entry | `L4-FR-1.3.1`, `L4-FR-1.3.9`, `L4-FR-3.4.1` |
| `L4-T-sqlite_supabase_sync-004` | `T2` | `OutboxHarness` | dispatcher refuses to materialize a second in-flight batch for the same stream while the prior `sync_batches` row is pending | `L4-FR-1.3.2`, `L4-FR-1.9.9` |
| `L4-T-sqlite_supabase_sync-005` | `T2` | `OutboxHarness` | daemon restart reloads the persisted in-flight `sync_batches` envelope and does not create a duplicate batch for the same local sequence range | `L4-FR-1.3.2`, `L4-FR-1.3.3` |
| `L4-T-sqlite_supabase_sync-006` | `T3` | `SupabaseRpcClient`, `OutboxHarness` | network timeout/lost response retries the exact row set, order, content hashes, schema epoch, stage run, commit groups, and writer epoch | `L4-FR-1.3.3` |
| `L4-T-sqlite_supabase_sync-007` | `T3` | `SupabaseRpcClient`, `OutboxHarness` | crash after batch persistence and before response causes restart to retry the persisted envelope before any later outbox rows are enveloped | `L4-FR-1.3.3` |
| `L4-T-sqlite_supabase_sync-008` | `T3` | `WriterLeaseHarness`, `OutboxHarness` | terminal stale-writer rejection marks the old batch rejected, re-acquires lease, and re-envelopes the same unacknowledged rows only with the new `writer_epoch` | `L4-FR-1.3.3`, `L4-FR-1.3.8`, `L4-FR-1.6.2` |
| `L4-T-sqlite_supabase_sync-009` | `T2` | `OutboxHarness` | batch assembly rejects mixed `schema_epoch` rows and leaves them unacknowledged for separate handling | `L4-FR-1.3.4`, `L4-FR-1.9.6` |
| `L4-T-sqlite_supabase_sync-010` | `T2` | `OutboxHarness` | batch assembly rejects rows from more than one `stage_run_id` | `L4-FR-1.3.4`, `L4-FR-1.9.7` |
| `L4-T-sqlite_supabase_sync-011` | `T2` | `OutboxHarness` | rows sharing a `commit_group_id` are sent together or not sent; configured batch size cannot split the group | `L4-FR-1.3.4`, `L4-FR-1.9.8` |
| `L4-T-sqlite_supabase_sync-012` | `T2` | `OutboxHarness` | rows inside a batch are ordered by `row_version` and deterministic entity key order for ties | `L4-FR-1.3.5` |
| `L4-T-sqlite_supabase_sync-013` | `T2` | `OutboxHarness` | `content_hash` is computed over canonical payload JSON, persisted on outbox rows, and reused without recomputation drift on retry | `L4-FR-1.3.5`, `L4-FR-1.3.8` |
| `L4-T-sqlite_supabase_sync-014` | `T2` | `SupabaseRpcClient` | `apply_push_batch` applies the full batch in one Postgres transaction; injecting one invalid row leaves every target table unchanged | `L4-FR-1.3.6`, `L4-FR-1.9.1` |
| `L4-T-sqlite_supabase_sync-015` | `T2` | `SupabaseRpcClient`, `RoleMatrixFixture` | daemon sync code uses HTTP RPC and no direct projection table upsert path; direct table write with daemon HTTP credentials is denied or absent from the client API | `L4-FR-1.3.6`, `L4-FR-2.5.10` |
| `L4-T-sqlite_supabase_sync-016` | `T2` | `SupabaseRpcClient` | incoming row with equal `row_version` and matching `content_hash` returns no-op success, creates no duplicate rows, and acknowledges the local outbox row | `L4-FR-1.3.7`, `L4-FR-1.9.10` |
| `L4-T-sqlite_supabase_sync-017` | `T2` | `SupabaseRpcClient` | incoming row with equal `row_version` and different `content_hash` returns a hard-conflict data-corruption error and leaves remote row unchanged | `L4-FR-1.3.7` |
| `L4-T-sqlite_supabase_sync-018` | `T2` | `SupabaseRpcClient` | incoming `row_version = remote row_version + 1` applies exactly once and updates `last_local_seq` | `L4-FR-1.3.7` |
| `L4-T-sqlite_supabase_sync-019` | `T2` | `SupabaseRpcClient` | older versions and larger jumps reject as stale or out-of-order without mutating remote state | `L4-FR-1.3.7` |
| `L4-T-sqlite_supabase_sync-020` | `T2` | `L4TestStateDb` | outbox `local_seq`, `payload_json`, `content_hash`, `commit_group_id`, and null pre-envelope `writer_epoch` remain stable across DB close/reopen | `L4-FR-1.3.8` |
| `L4-T-sqlite_supabase_sync-021` | `T2` | `OutboxHarness` | `writer_epoch` remains null on `local_outbox` until assignment to `sync_batches`; retries use immutable batch epoch, not mutable row epoch | `L4-FR-1.3.8`, `L4-FR-1.6.2` |
| `L4-T-sqlite_supabase_sync-022` | `T3` | `EnforcementHarness` | enforcement-active worker output with validator approval atomically updates provisional status, promotes push-owned rows, creates outbox entries, and advances stage heads | `L4-FR-1.3.9`, `L4-FR-3.4.2`, `L4-FR-1.9.11` |
| `L4-T-sqlite_supabase_sync-023` | `T3` | `EnforcementHarness`, `RedactionSentinelFixture` | enforcement-active worker output with validator rejection leaves provisional row local, emits `AGENT_SUSPECT_OUTPUT` error manifest with operator alert, creates no outbox entry, and does not advance stage heads | `L4-FR-1.3.9`, `L4-FR-3.4.1`, `L4-FR-3.4.2` |
| `L4-T-sqlite_supabase_sync-024` | `T3` | `EnforcementHarness` | dry-run or non-enforcement slice with validator rejection follows L3 direct-write behavior: push-owned rows and outbox entries are created, verdict is logged, and stage heads advance | `L4-FR-1.3.9`, `L4-FR-3.4.2` |
| `L4-T-sqlite_supabase_sync-025` | `T2` | `PullCursorHarness` | failed pull in one remote-owned domain does not advance that domain cursor and does not alter other domain cursors | `L4-FR-1.4.1` |
| `L4-T-sqlite_supabase_sync-026` | `T3` | `PullCursorHarness` | pull reads `hiwater`, paginates `cursor < remote_change_seq <= hiwater`, injects a concurrent `remote_change_seq > hiwater` write, applies only the fenced range, advances cursor to hiwater, and reads the injected write on the next pull without skip/duplicate | `L4-FR-1.4.2`, `L4-FR-1.9.14` |
| `L4-T-sqlite_supabase_sync-027` | `T2` | `PullCursorHarness` | each remote-owned domain assigns monotonic, non-reused `remote_change_seq` values on insert, update, and tombstone writes | `L4-FR-1.4.2`, `L4-FR-2.3.1`, `L4-FR-2.3.2`, `L4-FR-2.3.3` |
| `L4-T-sqlite_supabase_sync-028` | `T2` | `PullCursorHarness`, `CasCommandHarness` | sync startup and every sync cycle complete remote-owned pulls before pending-intent reconciliation, CAS flush, or push dispatch | `L4-FR-1.4.3`, `L4-FR-1.9.14` |
| `L4-T-sqlite_supabase_sync-029` | `T2` | `PullCursorHarness` | pull failure leaves CAS flushing and push dispatch for cross-authority decisions paused while local SQLite operation continues in degraded mode | `L4-FR-1.4.3`, `L4-FR-1.8.1` |
| `L4-T-sqlite_supabase_sync-030` | `T2` | `CasCommandHarness` | queued intent with matching `base_remote_revision` flushes through CAS RPC and marks the local queue row applied after the remote decision head advances | `L4-FR-1.4.4`, `L4-FR-1.5.1` |
| `L4-T-sqlite_supabase_sync-031` | `T2` | `CasCommandHarness` | queued intent with stale `base_remote_revision` is marked conflicted locally and surfaces structured conflict data | `L4-FR-1.4.4`, `L4-FR-1.5.1` |
| `L4-T-sqlite_supabase_sync-032` | `T2` | `CasCommandHarness` | three unsuperseded intents for the same process are reconciled latest-by creation time, using the L2 `queued_at DESC, id DESC` ordering; older rows receive `superseded_by_intent_id` and only the winner flushes through CAS | `L4-FR-1.4.4` |
| `L4-T-sqlite_supabase_sync-033` | `T2` | `CasCommandHarness` | CAS conflict response carries current remote revision, current disposition, rejected payload, and drives a `decision.conflict` IPC event payload | `L4-FR-1.5.1` |
| `L4-T-sqlite_supabase_sync-034` | `T2` | `CasCommandHarness` | applied CAS command writes `operator_decision_commands` in the same transaction as decision-head update with command id, process id, base revision, applied revision, disposition, result, and response JSON | `L4-FR-1.5.2` |
| `L4-T-sqlite_supabase_sync-035` | `T3` | `CasCommandHarness` | command 1 applies, command 2 applies, command 1 retry returns command 1 stored ledger response without re-evaluating the current decision head | `L4-FR-1.5.3` |
| `L4-T-sqlite_supabase_sync-036` | `T2` | `CasCommandHarness` | new command with mismatched base revision writes a conflict ledger row with null `applied_revision`, result `conflict`, and response JSON in the same transaction | `L4-FR-1.5.2`, `L4-FR-1.5.3` |
| `L4-T-sqlite_supabase_sync-037` | `T2` | `CasCommandHarness` | command ledger GC preserves entries newer than retention and never changes current `operator_decision_heads` state | `L4-FR-1.5.4` |
| `L4-T-sqlite_supabase_sync-038` | `T2` | `CasCommandHarness` | replaying a command id after its ledger row has been GC'd is evaluated as a new command and conflicts when its base revision is stale | `L4-FR-1.5.4` |
| `L4-T-sqlite_supabase_sync-039` | `T2` | `WriterLeaseHarness` | first daemon executor acquires or renews active writer lease and increments or preserves `writer_epoch` according to holder/expiry rules | `L4-FR-1.6.1`, `L4-FR-1.6.2` |
| `L4-T-sqlite_supabase_sync-040` | `T2` | `WriterLeaseHarness` | second daemon with different executor ID is rejected while the lease is valid and desktop sync status receives current holder and expiry | `L4-FR-1.6.1`, `L4-FR-1.6.3` |
| `L4-T-sqlite_supabase_sync-041` | `T3` | `WriterLeaseHarness`, `SupabaseRpcClient` | partitioned daemon recovering with old `writer_epoch` cannot push; remote rejects the batch even if local rows are otherwise valid | `L4-FR-1.6.2`, `L4-FR-1.9.5` |
| `L4-T-sqlite_supabase_sync-042` | `T2` | `WriterLeaseHarness` | heartbeat runs at least every 30 seconds while sync is active, extends expiry to at least 120 seconds, and another writer can acquire only after expiry | `L4-FR-1.6.2` |
| `L4-T-sqlite_supabase_sync-043` | `T1` | config/doc fixture | writer-mode operational binding rejects unapproved second writer configuration and surfaces viewer-only or explicit rehome next actions | `L4-FR-1.6.3` |
| `L4-T-sqlite_supabase_sync-044` | `T2` | `SupabaseRpcClient`, `L4TestStateDb` | schema epoch mismatch pauses push/pull sync, emits operator-visible status, leaves local pipeline writes available, and prevents projection writes | `L4-FR-1.7.1`, `L4-FR-1.9.6` |
| `L4-T-sqlite_supabase_sync-045` | `T3` | `ProjectionGenerationHarness` | after epoch repair, daemon creates a `projection_generations` row, republishes current local row images, publishes `viewer_sync_status.projection_generation`, and discards stale-epoch outbox rows only after success | `L4-FR-5.2.1` |
| `L4-T-sqlite_supabase_sync-046` | `T2` | `WriterLeaseHarness` | `viewer_sync_status` records healthy/degraded/offline health, timestamps, last acked sequence, pending outbox count, schema epoch, projection generation, daemon version, and updates on heartbeat/status-only commits | `L4-FR-1.8.1`, `L4-FR-3.3.4`, `L4-FR-3.7.1` |
| `L4-T-sqlite_supabase_sync-047` | `T2` | `PullCursorHarness` | stale `shared_source_configs.enabled` degrades a source after the default 4-hour safety-critical TTL, while display/schedule warns after 1 hour and company profile remains informational | `L4-FR-1.8.2`, `L4-FR-2.3.2` |
| `L4-T-sqlite_supabase_sync-048` | `T1` | backup fixture | encrypted WAL-aware SQLite snapshot scheduling records success/failure warnings and does not block local pipeline work | `L4-FR-1.8.3` |
| `L4-T-sqlite_supabase_sync-049` | `T2` | `L4TestStateDb` | pipeline runs that consume source config, company profile, or operator decision state persist the corresponding remote revision provenance | `L4-FR-2.7.1`-`L4-FR-2.7.5` |
| `L4-T-sqlite_supabase_sync-050` | `T1` | `CalibrationHarness` | slice with compatible overlay, >=30 adjudicated rejections, >=60 audited approvals, Wilson lower bound >=7000 bps, false-pass upper bound <=500 bps, challenge coverage, and no demotion trigger becomes `eligible` | `L4-FR-3.4.3`, `L4-FR-3.4.4`, `L4-FR-3.6.14` |
| `L4-T-sqlite_supabase_sync-051` | `T1` | `CalibrationHarness` | missing dominant failure family coverage keeps the slice in `dry_run` and records the exact missing family set | `L4-FR-3.4.4`, `L4-FR-3.6.3` |
| `L4-T-sqlite_supabase_sync-052` | `T1` | `CalibrationHarness` | Wilson lower bound below 7000 bps blocks enforcement even when sample counts are present | `L4-FR-3.4.4`, `L4-FR-3.6.14` |
| `L4-T-sqlite_supabase_sync-053` | `T1` | `CalibrationHarness` | rolling override spike threshold immediately writes `demoted` with demotion reason and blocks re-promotion until all gates pass | `L4-FR-3.4.5`, `L4-FR-3.4.6` |
| `L4-T-sqlite_supabase_sync-054` | `T1` | `CalibrationHarness` | approving any critical mutated-bad assertion or total challenge pass rate below 95% immediately demotes the affected slice | `L4-FR-3.4.5`, `L4-FR-3.6.3` |
| `L4-T-sqlite_supabase_sync-055` | `T1` | `CalibrationHarness` | anomaly-conditioned validator error rate over threshold demotes the affected slice and records the threshold snapshot | `L4-FR-3.4.5`, `L4-FR-3.4.6` |
| `L4-T-sqlite_supabase_sync-056` | `T1` | `CalibrationHarness` | worker behavior version outside overlay compatible range disables enforcement immediately | `L4-FR-3.4.5`, `L4-FR-3.5.1`-`L4-FR-3.5.4` |
| `L4-T-sqlite_supabase_sync-057` | `T2` | `CalibrationHarness`, `L4TestStateDb` | enforcement readiness decisions survive daemon restart and are read by `validation/policy.rs` instead of recomputed from volatile state | `L4-FR-3.4.6` |
| `L4-T-sqlite_supabase_sync-058` | `T3` | `OverlaySkillFixture` | procurement-ingestion overlay has the six-file skill contract, compatible versions, canary verdicts, and golden assertions for notice completeness, OCDS fields, locale, notice type, expiry, and CPV validity | `L4-FR-3.5.1` |
| `L4-T-sqlite_supabase_sync-059` | `T3` | `OverlaySkillFixture` | supplier-enrichment overlay has the six-file skill contract, compatible versions, canary verdicts, and golden assertions for price plausibility, currency, pack basis, seller diversity, freshness, and compliance evidence | `L4-FR-3.5.2` |
| `L4-T-sqlite_supabase_sync-060` | `T3` | `OverlaySkillFixture` | document-acquisition overlay has the six-file skill contract, compatible versions, canary verdicts, and golden assertions for document type, download completeness, content type, file size, and set hash consistency | `L4-FR-3.5.3` |
| `L4-T-sqlite_supabase_sync-061` | `T3` | `OverlaySkillFixture` | product-extraction overlay has the six-file skill contract, compatible versions, canary verdicts, and golden assertions for CPV family, archetype, slots, quantity/unit, and bundle decomposition | `L4-FR-3.5.4` |
| `L4-T-sqlite_supabase_sync-062` | `T3` | `CalibrationHarness` | each skill family has 25-30 clean real fixture/live cases, source artifacts, and `corpus_kind = 'clean'` rows | `L4-FR-3.6.1` |
| `L4-T-sqlite_supabase_sync-063` | `T3` | `CalibrationHarness` | each skill family has 15-20 weird-valid cases covering layout change, localization, partial honest data, and low-result-count valid examples without forced rejection | `L4-FR-3.6.2` |
| `L4-T-sqlite_supabase_sync-064` | `T3` | `CalibrationHarness` | each skill family has 30-50 mutated-bad cases derived from real good runs and stored with `corpus_kind = 'mutated'` plus mutation family | `L4-FR-3.6.3` |
| `L4-T-sqlite_supabase_sync-065` | `T1` | `CalibrationHarness` | URL chain swap mutation is present in challenge bank and a validator approval fails the golden assertion | `L4-FR-3.6.4` |
| `L4-T-sqlite_supabase_sync-066` | `T1` | `CalibrationHarness` | wrong page-kind mutation is present in challenge bank and a validator approval fails the golden assertion | `L4-FR-3.6.5` |
| `L4-T-sqlite_supabase_sync-067` | `T1` | `CalibrationHarness` | locale/currency swap mutation is present in challenge bank and a validator approval fails the golden assertion | `L4-FR-3.6.6` |
| `L4-T-sqlite_supabase_sync-068` | `T1` | `CalibrationHarness` | pack/unit distortion mutation is present in challenge bank and a validator approval fails the golden assertion | `L4-FR-3.6.7` |
| `L4-T-sqlite_supabase_sync-069` | `T1` | `CalibrationHarness` | accessory-vs-primary confusion mutation is present in challenge bank and a validator approval fails the golden assertion | `L4-FR-3.6.8` |
| `L4-T-sqlite_supabase_sync-070` | `T1` | `CalibrationHarness` | selective warning omission mutation is present in challenge bank and a validator approval fails the golden assertion | `L4-FR-3.6.9` |
| `L4-T-sqlite_supabase_sync-071` | `T1` | `CalibrationHarness` | manifest/result count mismatch mutation is present in challenge bank and a validator approval fails the golden assertion | `L4-FR-3.6.10` |
| `L4-T-sqlite_supabase_sync-072` | `T1` | `CalibrationHarness` | fabricated evidence references mutation is present in challenge bank and a validator approval fails the golden assertion | `L4-FR-3.6.11` |
| `L4-T-sqlite_supabase_sync-073` | `T1` | `CalibrationHarness` | cross-segment duplicate/gap mutation is present in challenge bank and a validator approval fails the golden assertion | `L4-FR-3.6.12` |
| `L4-T-sqlite_supabase_sync-074` | `T1` | `CalibrationHarness` | pairwise clean/corrupted siblings share a durable `pair_id`; corrupted sibling must rank less trustworthy and failures feed readiness/demotion inputs | `L4-FR-3.6.13` |
| `L4-T-sqlite_supabase_sync-075` | `T1` | `CalibrationHarness` | Wilson lower-bound and rule-of-three pure functions assert boundary cases around 30 rejected/flagged and 60 audited approvals | `L4-FR-3.6.14` |
| `L4-T-sqlite_supabase_sync-076` | `T3` | `CalibrationHarness` | cross-model audit samples enforcement-active slices at least once per 30 days with new traffic, records model family and disagreement metadata, and can block or demote | `L4-FR-3.6.15` |
| `L4-T-sqlite_supabase_sync-077` | `T2` | `CalibrationHarness`, `EnforcementHarness` | every operator override updates `validation_verdicts.override_*`, writes/updates calibration case and incident cluster rows in the same SQLite transaction, and stores candidate mutation template JSON | `L4-FR-3.4.2`, `L4-FR-3.6.16` |
| `L4-T-sqlite_supabase_sync-078` | `T2` | `PullCursorHarness`, `RoleMatrixFixture` | default tombstone retention is at least 14 days and daemon pull policies can see tombstoned remote-owned rows until retention expires | `L4-FR-3.6.17`, `L4-FR-2.5.10` |
| `L4-T-sqlite_supabase_sync-079` | `T3` | `EnforcementHarness` | `jobs/runner.rs` emits an enforcement-active worker job into provisional storage, enqueues/links validator job through the existing runner boundary, and promotes only after approved verdict | `L4-FR-1.3.9`, `L4-FR-3.4.2` |
| `L4-T-sqlite_supabase_sync-080` | `T3` | `EnforcementHarness` | validator crash/timeout/invalid payload follows L3 fail-open semantics with `VALIDATION_SKIPPED_VALIDATOR_ERROR`, advances worker result, and records validator failure manifest without treating it as an enforcement rejection | `L4-FR-3.4.2` |

### 8.3 `redaction_boundary.rs`

| Test ID | Tier | Fixtures | Exact Assertions | Requirements |
|--------|------|----------|------------------|--------------|
| `L4-T-redaction_boundary-001` | `T2` | `RoleMatrixFixture` | for all 20 push-owned projection tables, viewer/operator/admin can SELECT non-tombstoned rows, viewer/operator cannot mutate, and daemon mutation is only accepted through `apply_push_batch` | `L4-FR-2.5.1` |
| `L4-T-redaction_boundary-002` | `T2` | `RoleMatrixFixture` | for push-owned summaries, viewer/operator/admin can SELECT, viewer/operator cannot mutate, and daemon writes are accepted only through the batch RPC | `L4-FR-2.5.2`, `L4-FR-2.5.6` |
| `L4-T-redaction_boundary-003` | `T2` | `RoleMatrixFixture` | `shared_company_profile` denies viewer/operator raw SELECT and mutation, allows admin CRUD, and allows daemon SELECT including tombstones | `L4-FR-2.3.1`, `L4-FR-2.5.3` |
| `L4-T-redaction_boundary-004` | `T2` | `RoleMatrixFixture` | `shared_source_configs` denies viewer/operator raw SELECT and mutation, allows admin CRUD, and allows daemon SELECT including tombstones | `L4-FR-2.3.2`, `L4-FR-2.5.4` |
| `L4-T-redaction_boundary-005` | `T2` | `RoleMatrixFixture` | `operator_decision_heads` denies viewer raw SELECT/direct UPDATE, allows operator SELECT only where permitted plus CAS RPC mutation, allows admin CRUD, and allows daemon pull SELECT | `L4-FR-2.3.3`, `L4-FR-2.5.5` |
| `L4-T-redaction_boundary-006` | `T2` | `RoleMatrixFixture` | `operator_decision_summaries` is readable by viewer/operator/admin and mutable only by daemon through batch RPC | `L4-FR-2.2.6`, `L4-FR-2.5.6` |
| `L4-T-redaction_boundary-007` | `T2` | `RoleMatrixFixture` | `schema_metadata` and `active_writer_lease` deny viewer/operator access and allow admin/daemon SELECT/UPDATE according to the sync infrastructure policy | `L4-FR-2.5.7` |
| `L4-T-redaction_boundary-008` | `T2` | `RoleMatrixFixture` | `operator_decision_commands` denies viewer/operator access, allows admin SELECT, and allows daemon insert/select only through CAS RPC behavior | `L4-FR-2.5.8` |
| `L4-T-redaction_boundary-009` | `T2` | `RoleMatrixFixture` | viewer credentials can SELECT projection and summary rows but cannot INSERT, UPDATE, DELETE, call mutation RPCs, or SELECT raw remote-owned tables | `L4-FR-2.5.9` |
| `L4-T-redaction_boundary-010` | `T2` | `RedactionSentinelFixture` | `workflow_job_summaries` exposes job id/kind/status/skill/timestamps/error type but omits full job context, full payload, prompt, raw session context, and local-only artifacts | `L4-FR-2.2.1`, `L4-FR-2.5.11` |
| `L4-T-redaction_boundary-011` | `T2` | `RedactionSentinelFixture` | `error_manifest_summaries` exposes manifest id/job id/error type/code/incident signature/timestamp but omits markdown, `context_json`, runtime context, and linked artifact details | `L4-FR-2.2.2`, `L4-FR-2.5.11` |
| `L4-T-redaction_boundary-012` | `T2` | `RedactionSentinelFixture` | `validation_verdict_summaries` exposes verdict, basis sufficiency, enforcement eligibility, and timestamp but omits full validator reasoning, issues JSON, prompt evidence, and quality suggestions | `L4-FR-2.2.3`, `L4-FR-2.5.11` |
| `L4-T-redaction_boundary-013` | `T2` | `RedactionSentinelFixture` | `viewer_sync_status` exposes sync health/freshness/projection generation and no local DB path, service key, executor secret, or company profile data | `L4-FR-2.2.4`, `L4-FR-3.7.1`, `L4-FR-2.5.11` |
| `L4-T-redaction_boundary-014` | `T2` | `RedactionSentinelFixture` | `executors` exposes executor id/heartbeat/version/start time and omits socket paths, host usernames, local filesystem paths, and machine secrets | `L4-FR-2.2.5`, `L4-FR-2.5.11` |
| `L4-T-redaction_boundary-015` | `T2` | `RedactionSentinelFixture` | `operator_decision_summaries` exposes process id/disposition/staleness/reviewed stage/update time and omits operator identity, company profile, command history, and decision notes | `L4-FR-2.2.6`, `L4-FR-2.5.11` |
| `L4-T-redaction_boundary-016` | `T2` | `RedactionSentinelFixture`, `RoleMatrixFixture` | seeded company registration/VAT/address/contact sentinels are absent from every summary projection visible to viewer credentials | `L4-FR-2.2.1`-`L4-FR-2.2.6`, `L4-FR-2.3.1` |
| `L4-T-redaction_boundary-017` | `T2` | `RedactionSentinelFixture` | raw supplier URLs, source raw payloads, tender document metadata, and extraction internals are absent from all viewer summary rows | `L4-FR-2.2.1`-`L4-FR-2.2.6` |
| `L4-T-redaction_boundary-018` | `T2` | `RedactionSentinelFixture` | full validator reasoning sentinel cannot be found in `validation_verdict_summaries` or any web-visible summary table | `L4-FR-2.2.3`, `L4-FR-2.5.11` |
| `L4-T-redaction_boundary-019` | `T2` | `RedactionSentinelFixture` | full error markdown/context sentinel cannot be found in `error_manifest_summaries` or any web-visible summary table | `L4-FR-2.2.2`, `L4-FR-2.5.11` |
| `L4-T-redaction_boundary-020` | `T2` | `RedactionSentinelFixture` | command ledger history and command ids are absent from `operator_decision_summaries` and blocked from viewer/operator raw SELECT | `L4-FR-1.5.2`, `L4-FR-2.2.6`, `L4-FR-2.5.8` |
| `L4-T-redaction_boundary-021` | `T2` | `RoleMatrixFixture` | tombstoned rows are hidden from viewer/operator portal SELECT policies and visible to daemon pull policies until retention expiry | `L4-FR-2.5.10`, `L4-FR-3.6.17` |
| `L4-T-redaction_boundary-022` | `T2` | `RoleMatrixFixture` | operator direct UPDATE to `operator_decision_heads` fails while `update_operator_decision` RPC succeeds or conflicts according to base revision | `L4-FR-1.5.1`, `L4-FR-2.5.10` |
| `L4-T-redaction_boundary-023` | `T2` | `RoleMatrixFixture` | daemon direct projection table upsert is denied outside RPC; the same payload succeeds through `apply_push_batch` | `L4-FR-1.3.6`, `L4-FR-2.5.10` |
| `L4-T-redaction_boundary-024` | `T2` | `RoleMatrixFixture` | admin maintenance paths can mutate remote-owned config and decision state while still respecting tombstone/change-sequence triggers | `L4-FR-2.3.1`-`L4-FR-2.3.3`, `L4-FR-2.5.3`-`L4-FR-2.5.5` |
| `L4-T-redaction_boundary-025` | `T2` | `RoleMatrixFixture` | browser-held viewer credentials contain no operator/admin/daemon/service-role privileges and cannot use service-key-only endpoints | `L4-FR-2.5.9`, `L4-FR-3.1.1` |
| `L4-T-redaction_boundary-026` | `T2` | `RoleMatrixFixture` | schema hash coordination test applies SQLite and Supabase schemas from scratch, introspects logical columns for projection tables, and fails on logical mismatch | `L4-FR-2.6.10` |

### 8.4 `apps/web/e2e/contracts.spec.ts`

| Test ID | Tier | Fixtures | Exact Assertions | Requirements |
|--------|------|----------|------------------|--------------|
| `L4-T-contracts_spec-001` | `T4` | `WebPortalHarness` | `/` redirects to `/contracts` and renders the read-only contracts list using viewer credentials and projection tables | `L4-FR-3.1.1`, `L4-FR-3.1.3`, `L4-FR-3.1.4` |
| `L4-T-contracts_spec-002` | `T4` | `WebPortalHarness` | root layout renders global sync status from `viewer_sync_status` and never calls daemon IPC or SQLite transport | `L4-FR-3.1.2`, `L4-FR-3.3.4` |
| `L4-T-contracts_spec-003` | `T4` | `WebPortalHarness` | list and detail pages expose no approve/reject/watch controls, agent controls, daemon triggers, or config mutation controls | `L4-FR-3.1.1`, `L4-FR-2.5.9` |
| `L4-T-contracts_spec-004` | `T4` | `WebPortalHarness` | keyset pagination fetches page 1 and page 2 without duplicates, preserves visible ordering, and uses projection table cursor fields rather than offset pagination | `L4-FR-3.1.4` |
| `L4-T-contracts_spec-005` | `T4` | `WebPortalHarness` | green freshness state appears when `lastPushSuccessAt` is younger than 90 seconds with label/tone/tooltip from the shared FreshnessBadge path | `L4-FR-3.3.1`, `L4-FR-3.3.4` |
| `L4-T-contracts_spec-006` | `T4` | `WebPortalHarness` | amber freshness state appears for 90 seconds through 5 minutes and states that updates are delayed | `L4-FR-3.3.2`, `L4-FR-3.3.4` |
| `L4-T-contracts_spec-007` | `T4` | `WebPortalHarness` | red freshness state appears after 5 minutes or explicit offline health and states that last-known data is shown | `L4-FR-3.3.3`, `L4-FR-3.3.4` |
| `L4-T-contracts_spec-008` | `T4` | `WebPortalHarness` | `/contracts` subscribes to `org:{orgId}:contracts` and `org:{orgId}:sync`; `contracts.changed` shows an `N new/updated` pill and does not reorder visible rows | `L4-FR-3.2.1`, `L4-FR-3.2.2`, `L4-FR-3.2.4` |
| `L4-T-contracts_spec-009` | `T4` | `WebPortalHarness` | detail page subscribes to `process:{processId}:updates` and `org:{orgId}:sync`; minor `process.updated` refreshes only affected section | `L4-FR-3.2.1`, `L4-FR-3.2.4` |
| `L4-T-contracts_spec-010` | `T4` | `WebPortalHarness` | major `process.updated` for manifest/landed cost/canonical process shows a refresh chip and does not replace visible content until user action | `L4-FR-3.2.2`, `L4-FR-3.2.4` |
| `L4-T-contracts_spec-011` | `T4` | `WebPortalHarness` | contract detail renders candidates, quotes, margins, manifest, errors, and decision summary from projection/summary tables using `packages/ui` components | `L4-FR-3.1.5`, `L4-FR-2.2.1`-`L4-FR-2.2.6` |
| `L4-T-contracts_spec-012` | `T4` | `WebPortalHarness` | status page renders pipeline status, job summaries, error summaries, sync health, and projection generation from summary projections | `L4-FR-3.1.6`, `L4-FR-3.7.1` |
| `L4-T-contracts_spec-013` | `T4` | `WebPortalHarness` | viewer credential network requests can SELECT summaries/projections and receive explicit 401/403 or RLS denial for `shared_company_profile`, `shared_source_configs`, and `operator_decision_heads` raw tables | `L4-FR-2.5.9`, `L4-FR-3.1.1` |
| `L4-T-contracts_spec-014` | `T4` | `WebPortalHarness` | viewer credential attempts to call `update_operator_decision`, `apply_push_batch`, INSERT, UPDATE, or DELETE fail at the network/API boundary | `L4-FR-2.5.9`, `L4-FR-2.5.10` |
| `L4-T-contracts_spec-015` | `T4` | `WebPortalHarness` | bundled browser code and runtime environment contain no service-role key, admin key, operator token, daemon token, or local daemon endpoint | `L4-FR-2.5.9`, `L4-FR-3.1.1` |
| `L4-T-contracts_spec-016` | `T1` | package lint | `packages/ui` remains presentation-only: no Supabase, Broadcast, Tauri, daemon, desktop, or transport imports; `apps/web/src/lib/supabase.ts` is the sole direct Supabase boundary | `L4-FR-3.1.1`, `L4-FR-5.4.3` |
| `L4-T-contracts_spec-017` | `T4` | `WebPortalHarness`, desktop seed fixture | desktop writer seeds projection data through the sync path, web viewer sees the same process list/detail/status without mutation affordances | `L4-FR-1.8.1`, `L4-FR-3.1.1`, `L4-FR-3.3.5` |
| `L4-T-contracts_spec-018` | `T4` | `WebPortalHarness` | when sync is degraded/offline, desktop remains locally read-write while web shows stale amber/red state and no Broadcast data-change updates | `L4-FR-1.8.1`, `L4-FR-3.3.1`-`L4-FR-3.3.5` |
| `L4-T-contracts_spec-019` | `T4` | `WebPortalHarness` | `sync.updated` with increased `projectionGeneration` invalidates stale page snapshots and refetches without mixing old generation rows | `L4-FR-3.2.2`, `L4-FR-3.7.1`, `L4-FR-5.2.1` |
| `L4-T-contracts_spec-020` | `T1` | package lint | web app imports `packages/ui` and `packages/contracts-schema` but does not import desktop modules, Tauri APIs, daemon code, SQLite, or IPC transport | `L4-FR-3.1.1`-`L4-FR-3.1.8` |

### 8.5 Scope-Preservation Checklist

The tables below make the non-negotiable coverage targets greppable without replacing the full requirement coverage matrix in section 9.

#### Section 1.9 Invariants

| Invariant | Dedicated Test ID(s) |
|-----------|----------------------|
| Push batch atomicity | `L4-T-sqlite_supabase_sync-014` |
| Outbox and data in same SQLite transaction | `L4-T-sqlite_supabase_sync-001`, `002` |
| CAS for operator decisions | `L4-T-sqlite_supabase_sync-030`, `031`, `033` |
| CAS command ledger for idempotency | `L4-T-sqlite_supabase_sync-034`, `035`, `036` |
| Active-writer lease with fencing token | `L4-T-sqlite_supabase_sync-039`, `041`, `042` |
| `schema_epoch` on every batch | `L4-T-sqlite_supabase_sync-009`, `044` |
| Batch boundaries never cross `stage_run_id` | `L4-T-sqlite_supabase_sync-010` |
| Batch boundaries never split `commit_group_id` | `L4-T-sqlite_supabase_sync-011` |
| Single-in-flight batch | `L4-T-sqlite_supabase_sync-004` |
| Equal-version retry no-op on content hash match | `L4-T-sqlite_supabase_sync-016` |
| Provisional promotion for enforcement slices | `L4-T-sqlite_supabase_sync-022`, `023`, `079` |
| Validation enforcement | `L4-T-sqlite_supabase_sync-050`, `053`, `054`, `055`, `056`, `057` |
| RLS on all Supabase tables | `L4-T-redaction_boundary-001`-`009` |
| Fenced pull-before-push ordering | `L4-T-sqlite_supabase_sync-026`, `028` |

#### Section 4.3 Scenarios

| Scenario | Dedicated Test ID(s) |
|----------|----------------------|
| Push batch with stale row version | `L4-T-sqlite_supabase_sync-019` |
| Push batch with incompatible schema epoch | `L4-T-sqlite_supabase_sync-044` |
| Two desktop operators make concurrent decisions | `L4-T-sqlite_supabase_sync-031`, `033` |
| Validator rejects output in enforcement mode | `L4-T-sqlite_supabase_sync-023`, `077` |
| Validator crashes during validation | `L4-T-sqlite_supabase_sync-080` |
| Sync degraded or Supabase unreachable | `L4-T-sqlite_supabase_sync-029`, `046`, `L4-T-contracts_spec-018` |
| Full push replay after sync recovery | `L4-T-sqlite_supabase_sync-006`, `007`, `016` |
| Batch boundary crosses stage run | `L4-T-sqlite_supabase_sync-010` |
| Pull-before-push ordering violation | `L4-T-sqlite_supabase_sync-028` |
| Network interruption or crash during push batch | `L4-T-sqlite_supabase_sync-006`, `007`, `014` |
| Stale daemon recovers after lease loss | `L4-T-sqlite_supabase_sync-041` |
| CAS command retried after another command landed | `L4-T-sqlite_supabase_sync-035` |
| Equal-version retry with matching content hash | `L4-T-sqlite_supabase_sync-016` |
| Equal-version retry with different content hash | `L4-T-sqlite_supabase_sync-017` |
| Enforcement-active worker completes and validator rejects | `L4-T-sqlite_supabase_sync-023` |
| Paginated pull during concurrent remote changes | `L4-T-sqlite_supabase_sync-026` |
| Source remotely disabled while daemon offline for five hours | `L4-T-sqlite_supabase_sync-047` |

#### Summary Projection Redaction

| Projection | Redaction Test ID |
|------------|-------------------|
| `workflow_job_summaries` | `L4-T-redaction_boundary-010` |
| `error_manifest_summaries` | `L4-T-redaction_boundary-011` |
| `validation_verdict_summaries` | `L4-T-redaction_boundary-012` |
| `viewer_sync_status` | `L4-T-redaction_boundary-013` |
| `executors` | `L4-T-redaction_boundary-014` |
| `operator_decision_summaries` | `L4-T-redaction_boundary-015` |

#### Domain Validator Overlays

| Overlay | Canary / Golden Test ID |
|---------|-------------------------|
| Procurement ingestion | `L4-T-sqlite_supabase_sync-058` |
| Supplier enrichment | `L4-T-sqlite_supabase_sync-059` |
| Document acquisition | `L4-T-sqlite_supabase_sync-060` |
| Product extraction | `L4-T-sqlite_supabase_sync-061` |

## 9. Coverage Matrix

Coverage accounting below is requirement-bearing. Static gates also enforce release policy and inventory integrity.

| Spec Requirement | Test ID(s) | Test Type | Status |
|------------------|------------|-----------|--------|
| `L4-FR-1.3.1` | `L4-T-sqlite_schema-012`, `L4-T-sqlite_supabase_sync-001`, `002`, `003` | IT | Covered |
| `L4-FR-1.3.2` | `L4-T-sqlite_schema-003`, `014`, `L4-T-sqlite_supabase_sync-004`, `005` | IT | Covered |
| `L4-FR-1.3.3` | `L4-T-sqlite_schema-003`, `L4-T-sqlite_supabase_sync-005`, `006`, `007`, `008` | IT + T3 | Covered |
| `L4-FR-1.3.4` | `L4-T-sqlite_schema-015`, `L4-T-sqlite_supabase_sync-009`, `010`, `011` | IT | Covered |
| `L4-FR-1.3.5` | `L4-T-sqlite_schema-015`, `L4-T-sqlite_supabase_sync-012`, `013` | IT | Covered |
| `L4-FR-1.3.6` | `L4-T-sqlite_supabase_sync-014`, `015`, `L4-T-redaction_boundary-023` | IT | Covered |
| `L4-FR-1.3.7` | `L4-T-sqlite_supabase_sync-016`, `017`, `018`, `019` | IT | Covered |
| `L4-FR-1.3.8` | `L4-T-sqlite_schema-002`, `015`, `L4-T-sqlite_supabase_sync-013`, `020`, `021` | IT | Covered |
| `L4-FR-1.3.9` | `L4-T-sqlite_schema-005`, `014`, `L4-T-sqlite_supabase_sync-003`, `022`, `023`, `024`, `079` | IT + T3 | Covered |
| `L4-FR-1.4.1` | `L4-T-sqlite_schema-004`, `L4-T-sqlite_supabase_sync-025` | IT | Covered |
| `L4-FR-1.4.2` | `L4-T-sqlite_schema-021`, `L4-T-sqlite_supabase_sync-026`, `027` | IT + T3 | Covered |
| `L4-FR-1.4.3` | `L4-T-sqlite_supabase_sync-028`, `029` | IT | Covered |
| `L4-FR-1.4.4` | `L4-T-sqlite_schema-016`, `L4-T-sqlite_supabase_sync-030`, `031`, `032` | IT | Covered |
| `L4-FR-1.5.1` | `L4-T-sqlite_schema-016`, `L4-T-sqlite_supabase_sync-030`, `031`, `033`, `L4-T-redaction_boundary-022` | IT | Covered |
| `L4-FR-1.5.2` | `L4-T-sqlite_supabase_sync-034`, `036`, `L4-T-redaction_boundary-020` | IT | Covered |
| `L4-FR-1.5.3` | `L4-T-sqlite_supabase_sync-035`, `036` | IT + T3 | Covered |
| `L4-FR-1.5.4` | `L4-T-sqlite_supabase_sync-037`, `038` | IT | Covered |
| `L4-FR-1.6.1` | `L4-T-sqlite_supabase_sync-039`, `040` | IT | Covered |
| `L4-FR-1.6.2` | `L4-T-sqlite_schema-023`, `L4-T-sqlite_supabase_sync-008`, `021`, `039`, `041`, `042` | IT + T3 | Covered |
| `L4-FR-1.6.3` | `L4-T-sqlite_supabase_sync-040`, `043` | UT + IT | Covered |
| `L4-FR-1.7.1` | `L4-T-sqlite_schema-001`, `023`, `L4-T-sqlite_supabase_sync-044` | IT | Covered |
| `L4-FR-1.8.1` | `L4-T-sqlite_supabase_sync-029`, `046`, `L4-T-contracts_spec-017`, `018` | IT + E2E | Covered |
| `L4-FR-1.8.2` | `L4-T-sqlite_supabase_sync-047` | IT | Covered |
| `L4-FR-1.8.3` | `L4-T-sqlite_supabase_sync-048` | UT | Covered |
| `L4-FR-1.9.1` | `L4-T-sqlite_supabase_sync-014` | IT | Covered |
| `L4-FR-1.9.2` | `L4-T-sqlite_supabase_sync-001`, `002` | IT | Covered |
| `L4-FR-1.9.3` | `L4-T-sqlite_supabase_sync-030`, `031`, `033`, `L4-T-redaction_boundary-022` | IT | Covered |
| `L4-FR-1.9.4` | `L4-T-sqlite_supabase_sync-034`, `035`, `036` | IT | Covered |
| `L4-FR-1.9.5` | `L4-T-sqlite_supabase_sync-041` | T3 | Covered |
| `L4-FR-1.9.6` | `L4-T-sqlite_supabase_sync-009`, `044` | IT | Covered |
| `L4-FR-1.9.7` | `L4-T-sqlite_schema-015`, `L4-T-sqlite_supabase_sync-010` | IT | Covered |
| `L4-FR-1.9.8` | `L4-T-sqlite_schema-015`, `L4-T-sqlite_supabase_sync-011` | IT | Covered |
| `L4-FR-1.9.9` | `L4-T-sqlite_supabase_sync-004` | IT | Covered |
| `L4-FR-1.9.10` | `L4-T-sqlite_supabase_sync-016` | IT | Covered |
| `L4-FR-1.9.11` | `L4-T-sqlite_supabase_sync-022`, `023`, `079` | T3 | Covered |
| `L4-FR-1.9.12` | `L4-T-sqlite_supabase_sync-050`, `053`, `054`, `055`, `056`, `057` | UT + IT | Covered |
| `L4-FR-1.9.13` | `L4-T-sqlite_schema-022`, `L4-T-redaction_boundary-001`-`009` | IT | Covered |
| `L4-FR-1.9.14` | `L4-T-sqlite_supabase_sync-026`, `028` | T3 | Covered |
| `L4-FR-1.10.1` | `CI-L4-001`, `CI-L4-007`, `L4-T-sqlite_schema-018`, `024`, `L4-T-contracts_spec-016`, `020` | Static + IT + E2E | Covered |
| `L4-FR-2.1.1` | `L4-T-sqlite_schema-010`, `011`, `019`, `L4-T-redaction_boundary-001` | IT | Covered |
| `L4-FR-2.1.2` | `L4-T-sqlite_schema-010`, `011`, `019`, `L4-T-redaction_boundary-001` | IT | Covered |
| `L4-FR-2.1.3` | `L4-T-sqlite_schema-010`, `011`, `019`, `L4-T-redaction_boundary-001` | IT | Covered |
| `L4-FR-2.1.4` | `L4-T-sqlite_schema-010`, `011`, `019`, `L4-T-redaction_boundary-001` | IT | Covered |
| `L4-FR-2.1.5` | `L4-T-sqlite_schema-010`, `011`, `019`, `L4-T-redaction_boundary-001` | IT | Covered |
| `L4-FR-2.1.6` | `L4-T-sqlite_schema-010`, `011`, `019`, `L4-T-redaction_boundary-001` | IT | Covered |
| `L4-FR-2.1.7` | `L4-T-sqlite_schema-010`, `011`, `019`, `L4-T-redaction_boundary-001` | IT | Covered |
| `L4-FR-2.1.8` | `L4-T-sqlite_schema-010`, `011`, `019`, `L4-T-redaction_boundary-001` | IT | Covered |
| `L4-FR-2.1.9` | `L4-T-sqlite_schema-010`, `011`, `019`, `L4-T-redaction_boundary-001` | IT | Covered |
| `L4-FR-2.1.10` | `L4-T-sqlite_schema-010`, `011`, `019`, `L4-T-redaction_boundary-001` | IT | Covered |
| `L4-FR-2.1.11` | `L4-T-sqlite_schema-010`, `011`, `019`, `L4-T-redaction_boundary-001` | IT | Covered |
| `L4-FR-2.1.12` | `L4-T-sqlite_schema-010`, `011`, `019`, `L4-T-redaction_boundary-001` | IT | Covered |
| `L4-FR-2.1.13` | `L4-T-sqlite_schema-010`, `011`, `019`, `L4-T-redaction_boundary-001` | IT | Covered |
| `L4-FR-2.1.14` | `L4-T-sqlite_schema-010`, `011`, `019`, `L4-T-redaction_boundary-001` | IT | Covered |
| `L4-FR-2.1.15` | `L4-T-sqlite_schema-010`, `011`, `019`, `L4-T-redaction_boundary-001` | IT | Covered |
| `L4-FR-2.1.16` | `L4-T-sqlite_schema-010`, `011`, `019`, `L4-T-redaction_boundary-001` | IT | Covered |
| `L4-FR-2.1.17` | `L4-T-sqlite_schema-010`, `011`, `019`, `L4-T-redaction_boundary-001` | IT | Covered |
| `L4-FR-2.1.18` | `L4-T-sqlite_schema-010`, `011`, `019`, `L4-T-redaction_boundary-001` | IT | Covered |
| `L4-FR-2.1.19` | `L4-T-sqlite_schema-010`, `011`, `019`, `L4-T-redaction_boundary-001` | IT | Covered |
| `L4-FR-2.1.20` | `L4-T-sqlite_schema-010`, `011`, `019`, `L4-T-redaction_boundary-001` | IT | Covered |
| `L4-FR-2.2.1` | `L4-T-sqlite_schema-020`, `L4-T-redaction_boundary-010`, `016`, `017`, `L4-T-contracts_spec-011` | IT + E2E | Covered |
| `L4-FR-2.2.2` | `L4-T-sqlite_schema-020`, `L4-T-redaction_boundary-011`, `016`, `017`, `019`, `L4-T-contracts_spec-011` | IT + E2E | Covered |
| `L4-FR-2.2.3` | `L4-T-sqlite_schema-020`, `L4-T-redaction_boundary-012`, `016`, `018`, `L4-T-contracts_spec-011` | IT + E2E | Covered |
| `L4-FR-2.2.4` | `L4-T-sqlite_schema-020`, `L4-T-redaction_boundary-013`, `L4-T-contracts_spec-012` | IT + E2E | Covered |
| `L4-FR-2.2.5` | `L4-T-sqlite_schema-020`, `L4-T-redaction_boundary-014`, `L4-T-contracts_spec-012` | IT + E2E | Covered |
| `L4-FR-2.2.6` | `L4-T-sqlite_schema-020`, `L4-T-redaction_boundary-006`, `015`, `016`, `017`, `020`, `L4-T-contracts_spec-011` | IT + E2E | Covered |
| `L4-FR-2.3.1` | `L4-T-sqlite_schema-021`, `L4-T-sqlite_supabase_sync-027`, `L4-T-redaction_boundary-003`, `016`, `024` | IT | Covered |
| `L4-FR-2.3.2` | `L4-T-sqlite_schema-021`, `L4-T-sqlite_supabase_sync-027`, `047`, `L4-T-redaction_boundary-004`, `024` | IT | Covered |
| `L4-FR-2.3.3` | `L4-T-sqlite_schema-021`, `L4-T-sqlite_supabase_sync-027`, `030`, `031`, `L4-T-redaction_boundary-005`, `024` | IT | Covered |
| `L4-FR-2.4.1` | `L4-T-sqlite_schema-019` | IT | Covered |
| `L4-FR-2.4.2` | `L4-T-sqlite_schema-019` | IT | Covered |
| `L4-FR-2.4.3` | `L4-T-sqlite_schema-019` | IT | Covered |
| `L4-FR-2.5.1` | `L4-T-sqlite_schema-022`, `L4-T-redaction_boundary-001` | IT | Covered |
| `L4-FR-2.5.2` | `L4-T-sqlite_schema-022`, `L4-T-redaction_boundary-002` | IT | Covered |
| `L4-FR-2.5.3` | `L4-T-sqlite_schema-022`, `L4-T-redaction_boundary-003`, `024` | IT | Covered |
| `L4-FR-2.5.4` | `L4-T-sqlite_schema-022`, `L4-T-redaction_boundary-004`, `024` | IT | Covered |
| `L4-FR-2.5.5` | `L4-T-sqlite_schema-022`, `L4-T-redaction_boundary-005`, `022`, `024` | IT | Covered |
| `L4-FR-2.5.6` | `L4-T-redaction_boundary-002`, `006` | IT | Covered |
| `L4-FR-2.5.7` | `L4-T-redaction_boundary-007` | IT | Covered |
| `L4-FR-2.5.8` | `L4-T-redaction_boundary-008`, `020` | IT | Covered |
| `L4-FR-2.5.9` | `L4-T-redaction_boundary-009`, `025`, `L4-T-contracts_spec-003`, `013`, `014`, `015` | IT + E2E | Covered |
| `L4-FR-2.5.10` | `L4-T-sqlite_supabase_sync-015`, `L4-T-redaction_boundary-021`, `022`, `023`, `L4-T-contracts_spec-014` | IT + E2E | Covered |
| `L4-FR-2.5.11` | `L4-T-redaction_boundary-010`-`020` | IT | Covered |
| `L4-FR-2.6.1` | `L4-T-sqlite_schema-018`, `019` | IT | Covered |
| `L4-FR-2.6.2` | `L4-T-sqlite_schema-018`, `021` | IT | Covered |
| `L4-FR-2.6.3` | `L4-T-sqlite_schema-018`, `020` | IT | Covered |
| `L4-FR-2.6.4` | `L4-T-sqlite_schema-018`, `022`, `L4-T-redaction_boundary-001`-`009` | IT | Covered |
| `L4-FR-2.6.5` | `L4-T-sqlite_supabase_sync-014`, `015`, `030`, `031`, `034`, `035` | IT | Covered |
| `L4-FR-2.6.6` | `L4-T-sqlite_schema-023`, `L4-T-sqlite_supabase_sync-039`, `041`, `044` | IT | Covered |
| `L4-FR-2.6.7` | `L4-T-sqlite_supabase_sync-034`, `035`, `036`, `037`, `038` | IT | Covered |
| `L4-FR-2.6.8` | `L4-T-sqlite_schema-020`, `L4-T-redaction_boundary-006`, `015` | IT | Covered |
| `L4-FR-2.6.9` | `CI-L4-007`, `L4-T-sqlite_schema-018` | Static + IT | Covered |
| `L4-FR-2.6.10` | `L4-T-redaction_boundary-026` | IT | Covered |
| `L4-FR-2.6.11` | `L4-T-sqlite_schema-001`-`009`, `012`, `013`, `014`, `015` | UT + IT | Covered |
| `L4-FR-2.7.1` | `L4-T-sqlite_supabase_sync-049` | IT | Covered |
| `L4-FR-2.7.2` | `L4-T-sqlite_supabase_sync-049` | IT | Covered |
| `L4-FR-2.7.3` | `L4-T-sqlite_supabase_sync-049` | IT | Covered |
| `L4-FR-2.7.4` | `L4-T-sqlite_supabase_sync-046`, `049`, `L4-T-contracts_spec-012` | IT + E2E | Covered |
| `L4-FR-2.7.5` | `L4-T-sqlite_supabase_sync-049` | IT | Covered |
| `L4-FR-3.1.1` | `CI-L4-005`, `L4-T-sqlite_schema-024`, `L4-T-redaction_boundary-025`, `L4-T-contracts_spec-001`, `003`, `013`, `014`, `015`, `016`, `017`, `020` | Static + E2E | Covered |
| `L4-FR-3.1.2` | `L4-T-contracts_spec-002`, `020` | E2E | Covered |
| `L4-FR-3.1.3` | `L4-T-contracts_spec-001`, `020` | E2E | Covered |
| `L4-FR-3.1.4` | `L4-T-contracts_spec-001`, `004`, `008`, `020` | E2E | Covered |
| `L4-FR-3.1.5` | `L4-T-contracts_spec-009`, `010`, `011`, `020` | E2E | Covered |
| `L4-FR-3.1.6` | `L4-T-contracts_spec-012`, `020` | E2E | Covered |
| `L4-FR-3.1.7` | `L4-T-contracts_spec-013`, `014`, `015`, `020` | E2E | Covered |
| `L4-FR-3.1.8` | `L4-T-contracts_spec-008`, `009`, `010`, `019`, `020` | E2E | Covered |
| `L4-FR-3.2.1` | `L4-T-contracts_spec-008`, `009` | E2E | Covered |
| `L4-FR-3.2.2` | `L4-T-contracts_spec-008`, `010`, `019` | E2E | Covered |
| `L4-FR-3.2.3` | `L4-T-sqlite_supabase_sync-014`, `046`, `L4-T-contracts_spec-008`, `018` | IT + E2E | Covered |
| `L4-FR-3.2.4` | `L4-T-contracts_spec-008`, `009`, `010` | E2E | Covered |
| `L4-FR-3.3.1` | `L4-T-contracts_spec-005`, `018` | E2E | Covered |
| `L4-FR-3.3.2` | `L4-T-contracts_spec-006`, `018` | E2E | Covered |
| `L4-FR-3.3.3` | `L4-T-contracts_spec-007`, `018` | E2E | Covered |
| `L4-FR-3.3.4` | `L4-T-sqlite_supabase_sync-046`, `L4-T-contracts_spec-002`, `005`, `006`, `007` | IT + E2E | Covered |
| `L4-FR-3.3.5` | `L4-T-contracts_spec-017`, `018` | E2E | Covered |
| `L4-FR-3.4.1` | `L4-T-sqlite_supabase_sync-003`, `023` | T3 | Covered |
| `L4-FR-3.4.2` | `L4-T-sqlite_schema-005`, `017`, `L4-T-sqlite_supabase_sync-022`, `023`, `024`, `077`, `079`, `080` | IT + T3 | Covered |
| `L4-FR-3.4.3` | `L4-T-sqlite_schema-006`, `L4-T-sqlite_supabase_sync-050` | UT | Covered |
| `L4-FR-3.4.4` | `L4-T-sqlite_schema-006`, `L4-T-sqlite_supabase_sync-050`, `051`, `052` | UT | Covered |
| `L4-FR-3.4.5` | `L4-T-sqlite_supabase_sync-053`, `054`, `055`, `056` | UT | Covered |
| `L4-FR-3.4.6` | `L4-T-sqlite_schema-006`, `L4-T-sqlite_supabase_sync-053`, `055`, `057` | UT + IT | Covered |
| `L4-FR-3.5.1` | `L4-T-sqlite_supabase_sync-056`, `058` | T3 | Covered |
| `L4-FR-3.5.2` | `L4-T-sqlite_supabase_sync-056`, `059` | T3 | Covered |
| `L4-FR-3.5.3` | `L4-T-sqlite_supabase_sync-056`, `060` | T3 | Covered |
| `L4-FR-3.5.4` | `L4-T-sqlite_supabase_sync-056`, `061` | T3 | Covered |
| `L4-FR-3.6.1` | `L4-T-sqlite_schema-008`, `L4-T-sqlite_supabase_sync-062` | T3 | Covered |
| `L4-FR-3.6.2` | `L4-T-sqlite_schema-008`, `L4-T-sqlite_supabase_sync-063` | T3 | Covered |
| `L4-FR-3.6.3` | `L4-T-sqlite_schema-008`, `L4-T-sqlite_supabase_sync-051`, `054`, `064` | T3 | Covered |
| `L4-FR-3.6.4` | `L4-T-sqlite_supabase_sync-065` | UT | Covered |
| `L4-FR-3.6.5` | `L4-T-sqlite_supabase_sync-066` | UT | Covered |
| `L4-FR-3.6.6` | `L4-T-sqlite_supabase_sync-067` | UT | Covered |
| `L4-FR-3.6.7` | `L4-T-sqlite_supabase_sync-068` | UT | Covered |
| `L4-FR-3.6.8` | `L4-T-sqlite_supabase_sync-069` | UT | Covered |
| `L4-FR-3.6.9` | `L4-T-sqlite_supabase_sync-070` | UT | Covered |
| `L4-FR-3.6.10` | `L4-T-sqlite_supabase_sync-071` | UT | Covered |
| `L4-FR-3.6.11` | `L4-T-sqlite_supabase_sync-072` | UT | Covered |
| `L4-FR-3.6.12` | `L4-T-sqlite_supabase_sync-073` | UT | Covered |
| `L4-FR-3.6.13` | `L4-T-sqlite_supabase_sync-074` | UT | Covered |
| `L4-FR-3.6.14` | `L4-T-sqlite_supabase_sync-050`, `052`, `075` | UT | Covered |
| `L4-FR-3.6.15` | `L4-T-sqlite_supabase_sync-076` | T3 | Covered |
| `L4-FR-3.6.16` | `L4-T-sqlite_schema-009`, `L4-T-sqlite_supabase_sync-077` | IT | Covered |
| `L4-FR-3.6.17` | `L4-T-sqlite_supabase_sync-078`, `L4-T-redaction_boundary-021` | IT | Covered |
| `L4-FR-3.7.1` | `L4-T-sqlite_supabase_sync-046`, `L4-T-redaction_boundary-013`, `L4-T-contracts_spec-012`, `019` | IT + E2E | Covered |
| `L4-FR-5.2.1` | `L4-T-sqlite_schema-007`, `L4-T-sqlite_supabase_sync-045`, `L4-T-contracts_spec-019` | IT + E2E | Covered |
| `L4-FR-5.4.1` | `CI-L4-001`, `L4-T-sqlite_schema-024` | Static | Covered |
| `L4-FR-5.4.2` | `CI-L4-008` | Static | Covered |
| `L4-FR-5.4.3` | `CI-L4-005`, `CI-L4-008`, `L4-T-sqlite_schema-024`, `L4-T-contracts_spec-016`, `020` | Static + E2E | Covered |

## 10. Coverage Summary

| Metric | Value |
|--------|-------|
| Unique requirements in spec | `147` |
| Requirements with at least one requirement-bearing test | `147` |
| Coverage percentage | `100%` |
| Static gates | `8` |
| Runnable tests | `150` |
| Total checks | `158` |
| Requirement-bearing checks | `154` |
| New test files | `4 / 4` |
| Section 1.9 invariants covered | `14 / 14` |
| Section 4.1 Definition of Done rows covered | `18 / 18` |
| Section 4.3 scenarios covered | `17 / 17` |
| Mutation families covered | `9 / 9` |
| Summary projections covered | `6 / 6` |
| RLS roles covered | `4 / 4` |
| RLS table categories covered | `8 / 8` |
| Validator overlays covered | `4 / 4` |
| Orphan requirements | `0` |
| Duplicate test IDs | `0` |

## 11. Notes For Strategy / Beads

- `sqlite_supabase_sync.rs` is intentionally large because sync correctness, CAS, writer fencing, epoch repair, provisional promotion, and calibration state must be validated through one coherent integration surface.
- Beads that touch sync protocol must include the relevant `L4-T-sqlite_supabase_sync-*` IDs and the full Stage Validation Test Gate from section 7.
- Beads that touch Supabase schema or RLS must include both `sqlite_schema.rs` and `redaction_boundary.rs` IDs. RLS cannot be signed off with UI-only tests.
- Beads that touch `apps/web` must include `L4-T-contracts_spec-*`, `CI-L4-005`, `CI-L4-006`, and the `packages/ui` import-boundary checks.
- Beads that touch validation enforcement must include `L4-T-sqlite_supabase_sync-022`, `023`, `024`, `077`, `079`, and `080` so dry-run, approval, rejection, override, runner staging, and validator-crash paths remain distinct.
- Beads that touch calibration must include the mutation-family IDs `L4-T-sqlite_supabase_sync-065` through `073`, pairwise ID `074`, math ID `075`, cross-model ID `076`, and override feedback ID `077`.
- The Bootstrap companion document owns environment setup, Supabase provisioning, dependency additions, scaffold commands, fixtures, and command matrix. This TDD owns test intent and required assertions.
