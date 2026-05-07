# L4 Shared Visibility — Bead Bulk Import

> **Authority:** This document is the bead-creation manifest for Layer 4. Each section below is a self-contained bead definition embedding verbatim spec text, TDD test IDs, and concrete file paths. The L4 spec, TDD, and bootstrap remain authoritative.
>
> **Source documents:**
> - Spec (SOT #1): `docs/strategy/layers/layer-4-shared-visibility.md` (147 L4-FR-* requirements)
> - TDD (SOT #2): `docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-tdd.md` (158 checks: 8 static gates + 150 runnable tests)
> - Strategy (SOT #3, ordering only): `docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-strategy.md`
> - Bootstrap: `docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-bootstrap.md`
>
> **Bead totals:** 6 phase sub-epics + 50 task beads + 6 review-gate task beads = 62 beads.
>
> **Bead CLI:** `br` (NOT `bd`). Sync: `bin/br-sync` then manual `git add .beads/ && git commit && git push`.
>
> **Parent epic:** `testproject1-4zq` (Epic: Layer 4 — Shared Visibility). All 6 phase sub-epics parent to this; all 50 task beads + 6 review-gate task beads parent to their phase sub-epic.
>
> **Coverage assertion:** every L4-FR-X.Y.Z appears in at least one bead's "Spec Requirements" section. Every L4-T-* test ID is owned by exactly one task bead's "Tests to Pass" list (review-gate beads aggregate phase coverage but do not double-own test IDs).

---

# Phase Sub-Epics

These six sub-epics parent under `testproject1-4zq` and serve as both phase containers and review-gate aggregators. Each sub-epic's description embeds the strategy's phase-gate criteria.

---

## l4-schema-foundation: Phase 1 Sub-Epic — Schema Foundation

- type: epic
- priority: P0
- labels: layer-4,l4-phase-1,sub-epic
- parent: testproject1-4zq

### Description

## SOURCE OF TRUTH WARNING

- **Spec**: `docs/strategy/layers/layer-4-shared-visibility.md` — §1.10, §2.6, §5.4 (PRIMARY)
- **TDD**: `docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-tdd.md` — §7 static gates, §8.1 sqlite_schema (PRIMARY)
- **Strategy**: `docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-strategy.md` — Phase 1 (ordering)
- **Bootstrap**: `docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-bootstrap.md` — §3, §5, §6

## Phase Goal

Freeze schema names, migration slots, dependency additions, and workspace wiring before any sync engine code runs. SQLite migration `0012_l4_sync_state.sql`, the eight Supabase migrations, daemon HTTP client, root workspace scripts, schema hash gate, and test inventory all land here.

## Phase 1 Task Beads (7)

1. `l4-p1-daemon-http-client` — add `reqwest` to `crates/daemon/Cargo.toml`.
2. `l4-p1-sqlite-sync-migration` — `migrations/0012_l4_sync_state.sql` with 8 tables.
3. `l4-p1-sqlite-registry-epoch` — `crates/daemon/src/db/sqlite.rs` migration registry + `DEFAULT_SCHEMA_EPOCH = 9`.
4. `l4-p1-supabase-config-and-migration-bundle` — `infra/supabase/config.toml` + 8 migrations.
5. `l4-p1-root-web-workspace-scripts` — root `package.json` build/test/lint cover `apps/web` while preserving `packages/ui` lint boundary.
6. `l4-p1-schema-hash-ci-gate` — CI gate that applies SQLite + Supabase migrations and hashes the logical schema.
7. `l4-p1-test-inventory-and-fixtures` — `crates/daemon/tests/sqlite_schema.rs` + `support/db.rs`/`support/supabase.rs` scaffolding.

## Phase 1 Review Bead

`l4-p1-review-gate` runs the cumulative gate after all 7 tasks above are complete.

## Gate Criteria (from strategy §Phase 1)

- Static gates `CI-L4-001`, `CI-L4-003`, `CI-L4-005`, `CI-L4-007`, `CI-L4-008` are green for foundation scope.
- `L4-T-sqlite_schema-001` through `L4-T-sqlite_schema-024` pass.
- `L4-T-redaction_boundary-026` passes (logical schema hash gate).
- `cargo test -p daemon --test sqlite_schema -- --nocapture` and `cargo test -p daemon --test redaction_boundary -- --nocapture` cover the listed IDs when those files exist.
- Root workspace executes web build/test/lint commands once web files exist; not just `apps/*` glob.
- SQLite and Supabase migrations apply from scratch before any Phase 2 or Phase 3 sync code runs.

## Roll Back If

- The L4 SQLite migration cannot be applied through the existing `open_state_db`/`PRAGMA user_version` path.
- Supabase migration ordering requires engine code to exist before DDL applies cleanly.
- Root scripts cannot include `apps/web` while preserving the `packages/ui` import boundary.

## Spec Coverage (this phase)

- L4-FR-1.10.1 (components touched / inventory)
- L4-FR-1.7.1 (schema epoch — registration, full behavior in P2)
- L4-FR-2.1.1 through L4-FR-2.1.20 (all 20 push-owned projection tables — DDL)
- L4-FR-2.2.1 through L4-FR-2.2.6 (6 summary projections — DDL)
- L4-FR-2.3.1 through L4-FR-2.3.3 (3 remote-owned tables — DDL)
- L4-FR-2.4.1 through L4-FR-2.4.3 (3 junction tables)
- L4-FR-2.5.1 through L4-FR-2.5.11 (RLS policies — file enters here, closes in P6)
- L4-FR-2.6.1 through L4-FR-2.6.11 (Supabase + SQLite migration inventory)
- L4-FR-1.3.1, 1.3.8, 1.3.9 (outbox/provisional storage — DDL only)
- L4-FR-1.4.1 (pull cursors — DDL only)
- L4-FR-3.4.6 (enforcement_readiness — DDL only)
- L4-FR-3.6.1 through L4-FR-3.6.17 (calibration corpora — DDL/skeleton only; behavior in P4)
- L4-FR-5.2.1 (projection_generations — DDL only; behavior in P6)
- L4-FR-5.4.1 (reqwest dep)
- L4-FR-5.4.2 (br terminology)
- L4-FR-5.4.3 (packages/ui import boundary)

---

## l4-push-engine: Phase 2 Sub-Epic — Push Engine

- type: epic
- priority: P0
- labels: layer-4,l4-phase-2,sub-epic
- parent: testproject1-4zq

### Description

## SOURCE OF TRUTH WARNING

- **Spec**: `docs/strategy/layers/layer-4-shared-visibility.md` — §1.3, §1.6, §1.7, §1.8 (PRIMARY)
- **TDD**: `docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-tdd.md` — §8.2 sqlite_supabase_sync 001..024, 039..048 (PRIMARY)
- **Strategy**: `docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-strategy.md` — Phase 2

## Phase Goal

Local-to-remote push: outbox writer, batch envelope assembly, identical-envelope retry, transactional `apply_push_batch` RPC, equal-version semantics, active-writer lease with `writer_epoch` fencing, schema-epoch pause, and `viewer_sync_status` write path. Phase 6 closes startup wiring; Phase 4 closes provisional promotion.

## Phase 2 Task Beads (8)

1. `l4-p2-outbox-transaction-writer` — push-owned mutations write outbox row in same SQLite transaction.
2. `l4-p2-batch-envelope-retry` — batch assembly + persistence; identical-envelope retry.
3. `l4-p2-apply-push-batch-rpc` — Supabase Postgres function + daemon HTTP client.
4. `l4-p2-equal-version-semantics` — 4-case row_version/content_hash semantics.
5. `l4-p2-viewer-sync-status-writer` — daemon writes `viewer_sync_status` after apply.
6. `l4-p2-active-writer-lease` — lease acquisition + `writer_epoch` increment + heartbeat.
7. `l4-p2-stale-writer-reenvelope` — terminal stale-writer rejection + lease re-acquisition + re-envelope.
8. `l4-p2-schema-epoch-pause-and-backup` — epoch mismatch pause + backup scheduling.

## Phase 2 Review Bead

`l4-p2-review-gate` runs the cumulative gate.

## Gate Criteria (from strategy §Phase 2)

- Rust gate: `cargo check --all-targets`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check` pass.
- `L4-T-sqlite_supabase_sync-001` through `L4-T-sqlite_supabase_sync-024` pass.
- `L4-T-sqlite_supabase_sync-039` through `L4-T-sqlite_supabase_sync-046` pass (epoch-repair reseed closure remains Phase 6).
- `L4-T-sqlite_supabase_sync-048` passes if backup path is touched here.
- `L4-T-redaction_boundary-023` passes (direct upsert denial).
- `cargo test -p daemon --test sqlite_supabase_sync -- --nocapture` and `cargo test -p daemon --test redaction_boundary -- --nocapture` cover the listed IDs.
- Push-owned projection mutation only through batch RPC.

## Roll Back If

- Outbox writes cannot be made atomic with push-owned data writes.
- Dispatcher retry requires rebuilding envelopes after indeterminate failures.
- Push apply cannot keep single-in-flight, batch-boundary, lease, epoch, and equal-version semantics in one coherent engine lane.

## Spec Coverage (this phase)

- L4-FR-1.3.1 through L4-FR-1.3.8 (push protocol)
- L4-FR-1.6.1 through L4-FR-1.6.3 (active-writer lease)
- L4-FR-1.7.1 (schema epoch — pause behavior; full repair in P6)
- L4-FR-1.8.1 (sync health states)
- L4-FR-1.8.3 (backup/durability)
- L4-FR-1.9.1, L4-FR-1.9.2, L4-FR-1.9.5, L4-FR-1.9.6, L4-FR-1.9.7, L4-FR-1.9.8, L4-FR-1.9.9, L4-FR-1.9.10 (push invariants)
- L4-FR-3.2.3 (Broadcast emission point — partial; web behavior in P5)
- L4-FR-3.7.1 (viewer_sync_status writer)

---

## l4-pull-cas: Phase 3 Sub-Epic — Pull Engine and CAS

- type: epic
- priority: P0
- labels: layer-4,l4-phase-3,sub-epic
- parent: testproject1-4zq

### Description

## SOURCE OF TRUTH WARNING

- **Spec**: `docs/strategy/layers/layer-4-shared-visibility.md` — §1.4, §1.5, §1.8 (PRIMARY)
- **TDD**: `docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-tdd.md` — §8.2 sqlite_supabase_sync 025..038, 047, §8.3 redaction_boundary 021, 022, 024 (PRIMARY)
- **Strategy**: `docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-strategy.md` — Phase 3

## Phase Goal

Remote-to-local pull: per-domain pull cursors with hi-water fenced pagination, pull-before-push startup contract, CAS RPC with durable command ledger, pending-intent reconciliation with latest-per-process coalescing, decision conflict IPC events, remote-config staleness TTL classes. Parallel with Phase 2; both depend on Phase 1 schema only.

## Phase 3 Task Beads (8)

1. `l4-p3-pull-cursors` — pull cursor domain rows + monotonic remote_change_seq.
2. `l4-p3-hiwater-pagination` — hi-water fenced pagination.
3. `l4-p3-pull-before-push-service-contract` — startup/cycle ordering.
4. `l4-p3-cas-rpc` — `update_operator_decision` Postgres function.
5. `l4-p3-command-ledger-retention` — `operator_decision_commands` ledger + 30-day GC + post-GC replay.
6. `l4-p3-pending-intent-reconciliation` — latest-per-process coalescing in daemon.
7. `l4-p3-decision-conflict-ipc` — `crates/daemon/src/ipc/handlers/decisions.rs` extension.
8. `l4-p3-remote-config-staleness-ttl` — staleness classes (informational/operational/safety-critical).

## Phase 3 Review Bead

`l4-p3-review-gate` runs the cumulative gate.

## Gate Criteria (from strategy §Phase 3)

- Rust gate: `cargo check --all-targets`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check` pass.
- `L4-T-sqlite_supabase_sync-025` through `L4-T-sqlite_supabase_sync-038` pass.
- `L4-T-sqlite_supabase_sync-047` passes for remote config staleness handling.
- `L4-T-redaction_boundary-021`, `022`, and `024` pass.
- CAS conflict behavior emits `decision.conflict` IPC event referenced by `L4-T-sqlite_supabase_sync-033`.

## Roll Back If

- Pull pagination cannot be fenced by a domain hi-water value.
- Pending intent reconciliation needs to bypass the existing `pending_operator_intents` capture flow.
- CAS replay needs to re-evaluate current decision state instead of returning ledgered results.

## Spec Coverage (this phase)

- L4-FR-1.4.1 through L4-FR-1.4.4 (pull protocol)
- L4-FR-1.5.1 through L4-FR-1.5.4 (CAS)
- L4-FR-1.8.2 (remote config staleness TTL)
- L4-FR-1.9.3, L4-FR-1.9.4, L4-FR-1.9.14 (pull/CAS invariants)
- L4-FR-2.5.10 (mutation rights via RPC; direct UPDATE blocked)

---

## l4-validation-enforcement: Phase 4 Sub-Epic — Validation Enforcement

- type: epic
- priority: P0
- labels: layer-4,l4-phase-4,sub-epic
- parent: testproject1-4zq

### Description

## SOURCE OF TRUTH WARNING

- **Spec**: `docs/strategy/layers/layer-4-shared-visibility.md` — §1.3.9, §3.4, §3.5, §3.6 (PRIMARY)
- **TDD**: `docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-tdd.md` — §8.2 sqlite_supabase_sync 003, 022..024, 050..080 (PRIMARY)
- **Strategy**: `docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-strategy.md` — Phase 4

## Phase Goal

Provisional promotion buffer for enforcement-active slices, durable per-slice readiness, executor overlay quarantine, runner promotion convergence, calibration math + corpora, override feedback loop, and 4 domain validator overlays. Extends L3 validation policy/executor in place — does NOT fork.

## Phase 4 Task Beads (12)

1. `l4-p4-provisional-storage-access` — `provisional_worker_outputs` access layer.
2. `l4-p4-policy-readiness-contract` — `validation/policy.rs` reads `enforcement_readiness`.
3. `l4-p4-error-manifest-classification` — `AGENT_SUSPECT_OUTPUT` classification.
4. `l4-p4-executor-overlay-quarantine` — `validation/executor.rs` overlay loading + quarantine.
5. `l4-p4-runner-provisional-promotion` — `jobs/runner.rs` provisional → push-owned.
6. `l4-p4-calibration-math-readiness` — Wilson lower-bound + rule-of-three.
7. `l4-p4-calibration-corpora-and-cases` — corpora + mutation library + cases storage.
8. `l4-p4-override-feedback-loop` — override → calibration case + incident cluster + mutation template.
9. `l4-p4-overlay-procurement-ingestion` — overlay skill (6-file template).
10. `l4-p4-overlay-supplier-enrichment` — overlay skill.
11. `l4-p4-overlay-document-acquisition` — overlay skill.
12. `l4-p4-overlay-product-extraction` — overlay skill.

## Phase 4 Review Bead

`l4-p4-review-gate` runs the cumulative gate.

## Gate Criteria (from strategy §Phase 4)

- Rust gate: `cargo check --all-targets`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check` pass.
- `CI-L4-007` green for overlay directories + calibration artifacts.
- `L4-T-sqlite_schema-005..009`, `017` pass.
- `L4-T-sqlite_supabase_sync-003`, `022`, `023`, `024`, `050..080` pass.
- Overlay canary gates `L4-T-sqlite_supabase_sync-058..061` pass.
- Mutation-family gates `L4-T-sqlite_supabase_sync-065..073`, pairwise `074`, math `075`, cross-model `076`, override feedback `077` pass.

## Roll Back If

- Enforcement requires replacing the L3 validator policy/executor instead of extending them.
- Rejected enforcement-active output has already reached push-owned state before validation verdict handling.
- Demotion/readiness state cannot be made durable and queryable through SQLite.

## Spec Coverage (this phase)

- L4-FR-1.3.9 (provisional promotion buffer)
- L4-FR-1.9.11, L4-FR-1.9.12 (provisional + enforcement invariants)
- L4-FR-3.4.1 through L4-FR-3.4.6 (enforcement)
- L4-FR-3.5.1 through L4-FR-3.5.4 (4 overlays)
- L4-FR-3.6.1 through L4-FR-3.6.16 (calibration harness)
- L4-FR-3.6.17 (tombstone retention — deferred to Phase 6 for full proof)

---

## l4-web-portal: Phase 5 Sub-Epic — Web Portal

- type: epic
- priority: P0
- labels: layer-4,l4-phase-5,sub-epic
- parent: testproject1-4zq

### Description

## SOURCE OF TRUTH WARNING

- **Spec**: `docs/strategy/layers/layer-4-shared-visibility.md` — §3.1, §3.2, §3.3 (PRIMARY)
- **TDD**: `docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-tdd.md` — §8.4 contracts_spec 001..020 (PRIMARY)
- **Strategy**: `docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-strategy.md` — Phase 5

## Phase Goal

Read-only Next.js web portal under `apps/web/` with 7 source files. Supabase client + Broadcast invalidation, page-scoped subscriptions, three-state freshness UX (green/amber/red), keyset pagination, viewer-credential security boundary at the database/API layer, and `packages/ui` import-boundary preservation. Side lane parallel with Phases 2-4.

## Phase 5 Task Beads (8)

1. `l4-p5-web-package-config` — `apps/web/package.json`, `tsconfig.json`, `next.config.ts`, `playwright.config.ts`.
2. `l4-p5-web-supabase-client-boundary` — `src/lib/supabase.ts`.
3. `l4-p5-web-broadcast-client` — `src/lib/broadcast.ts`.
4. `l4-p5-web-root-layout-and-redirect` — `src/app/layout.tsx` + `src/app/page.tsx`.
5. `l4-p5-web-contract-list` — `src/app/contracts/page.tsx`.
6. `l4-p5-web-contract-detail` — `src/app/contracts/[id]/page.tsx`.
7. `l4-p5-web-status-page` — `src/app/status/page.tsx`.
8. `l4-p5-web-freshness-consumption-and-boundary` — extend `packages/ui/freshness-badge.tsx` adapter + viewer credential boundary tests.

## Phase 5 Review Bead

`l4-p5-review-gate` runs the cumulative gate.

## Gate Criteria (from strategy §Phase 5)

- Bun gates: `bun run test`, `bun run lint`, `(cd apps/web && bun run test)`, `(cd apps/web && bun run test:e2e)` pass.
- Static gates `CI-L4-005`, `CI-L4-006`, `CI-L4-008` green.
- `L4-T-contracts_spec-001` through `L4-T-contracts_spec-020` pass.
- `L4-T-redaction_boundary-009` and `025` pass.

## Roll Back If

- Browser-held credentials need operator/admin/daemon/service-role privileges.
- Web data fetching leaks into `packages/ui`.
- Portal behavior depends on local daemon IPC, SQLite, or desktop transport.

## Spec Coverage (this phase)

- L4-FR-3.1.1 through L4-FR-3.1.8 (web portal)
- L4-FR-3.2.1, L4-FR-3.2.2, L4-FR-3.2.4 (Broadcast)
- L4-FR-3.3.1 through L4-FR-3.3.5 (degraded mode UX)
- L4-FR-3.7.1 (viewer_sync_status read)

---

## l4-integration-hardening: Phase 6 Sub-Epic — Integration and Hardening

- type: epic
- priority: P0
- labels: layer-4,l4-phase-6,sub-epic
- parent: testproject1-4zq

### Description

## SOURCE OF TRUTH WARNING

- **Spec**: `docs/strategy/layers/layer-4-shared-visibility.md` — §1.7 (repair), §2.5 (RLS final), §2.7 (provenance), §4.1 DoD, §5.2 (epoch repair) (PRIMARY)
- **TDD**: `docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-tdd.md` — §8.2 sqlite_supabase_sync 028, 045..049, 078, §8.3 redaction_boundary all, §8.4 contracts_spec 017..019 (PRIMARY)
- **Strategy**: `docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-strategy.md` — Phase 6

## Phase Goal

Convergence: `main.rs` startup wiring (pull-before-push, lease acquisition, schema-epoch checks), epoch-repair `projection_generation` reseed, final RLS/redaction closure across all 26 tests + 28 role/category assertions, tombstone retention proof, cross-authority provenance, logical consistency auditor stub, two-user verification, and DoD walkthrough.

## Phase 6 Task Beads (7)

1. `l4-p6-main-startup-sync-wiring` — `crates/daemon/src/main.rs` startup ordering.
2. `l4-p6-epoch-repair-generation` — `projection_generations` reseed path + Broadcast `projection_generation` increment.
3. `l4-p6-final-rls-redaction-closure` — final RLS policy content + 26 redaction tests.
4. `l4-p6-tombstone-retention-proof` — 14-day retention + daemon pull tombstone visibility.
5. `l4-p6-cross-authority-provenance-and-ttl` — record `source_config_revision`, `company_profile_revision`, `operator_decision_revision` per pipeline run.
6. `l4-p6-logical-consistency-auditor-stub` — testable minimum stub per §5.2.
7. `l4-p6-two-user-verification` — manual verification doc + smoke diary.

## Phase 6 Review Bead

`l4-p6-stage-gate-and-dod-walkthrough` runs the full Stage Validation Test Gate, all CI-L4-001..008, all 4 test files end-to-end, and DoD rows 1..18.

## Gate Criteria (from strategy §Phase 6)

- Mandatory Stage Validation Test Gate passes:
  - `cargo fmt --check`
  - `cargo check --all-targets`
  - `cargo clippy --all-targets -- -D warnings`
  - `cargo test -p daemon --bin daemon`
  - every `crates/daemon/tests/*.rs` target through the integration-test loop
  - `bun run test`
  - `(cd apps/web && bun run test:e2e)`
- Supplemental `bun run lint` passes.
- Static gates `CI-L4-001` through `CI-L4-008` green.
- `L4-T-sqlite_supabase_sync-025..049`, `078`, plus all Phase 2/4 sync IDs pass under the full suite.
- `L4-T-redaction_boundary-001` through `026` pass.
- `L4-T-contracts_spec-017`, `018`, `019` pass against integrated push/web behavior.
- `l4-p6-two-user-verification` records a manual smoke diary linked to `L4-T-contracts_spec-017` and `018`.
- `l4-p6-logical-consistency-auditor-stub` is testable; cannot be a TODO/empty/doc-only note.
- Definition of Done rows 1 through 18 verified in one walkthrough.

## Roll Back If

- Startup ordering requires pushing before remote-owned pulls complete.
- Final RLS closure cannot enforce viewer/operator/admin/daemon boundaries at the database/API layer.
- Two-user verification cannot show desktop writer and web viewer observing the same projection through the integrated sync path.

## Spec Coverage (this phase)

- L4-FR-1.7.1 (epoch repair convergence)
- L4-FR-1.9.13 (RLS final closure)
- L4-FR-2.5.11 (redaction final)
- L4-FR-2.6.10 (schema hash CI re-run integrated)
- L4-FR-2.7.1 through L4-FR-2.7.5 (cross-authority provenance)
- L4-FR-3.6.17 (tombstone retention proof)
- L4-FR-5.2.1 (epoch-repair reproject minimum path)

---

# Phase 1 Task Beads — Schema Foundation

---

## l4-p1-daemon-http-client: Add reqwest HTTP Client Dependency to Daemon

- type: task
- priority: P0
- labels: layer-4,l4-phase-1,rust,deps,chokepoint
- parent: l4-schema-foundation

### Description

## SOURCE OF TRUTH WARNING

- **Spec**: `docs/strategy/layers/layer-4-shared-visibility.md` — §5.4 Toolchain Prerequisites (PRIMARY)
- **TDD**: `docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-tdd.md` — `CI-L4-001`, `L4-T-sqlite_schema-024` (PRIMARY)
- **Strategy**: `docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-strategy.md` — Phase 1, P1-A
- **Bootstrap**: `docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-bootstrap.md` — §5.1

## Context

Daemon currently has no HTTP client; L4 sync RPCs must call Supabase REST. This bead adds the `reqwest` dependency in exactly the form mandated by the bootstrap so `crates/daemon/src/db/sync.rs` compiles when Phase 2 begins.

## Files to Create/Modify

- **Modify:** `crates/daemon/Cargo.toml` — add `reqwest` dependency line.

## Spec Requirements (verbatim)

**L4-FR-5.4.1 — Add daemon HTTP client dependency: `reqwest` with `rustls-tls` and `json` features.**
- The daemon currently has no HTTP client in `crates/daemon/Cargo.toml`; L4 sync must call Supabase RPCs.

**Acceptance criteria from spec §5.4 (excerpt):**
- AC1: `crates/daemon/Cargo.toml` includes `reqwest = { ..., features = ["rustls-tls", "json"] }` or an explicitly approved equivalent Supabase HTTP client dependency.
- AC4: `cargo check --all-targets` is the verification hook for the daemon HTTP-client dependency resolving under the pinned Rust toolchain.

## Implementation Details (from bootstrap §5.1)

Add this exact line to `crates/daemon/Cargo.toml` `[dependencies]`:

```toml
reqwest = { version = "0.12", default-features = false, features = ["rustls-tls", "json"] }
```

Do not add `tokio-postgres` as a production daemon dependency. Daemon sync uses Supabase REST RPCs via `reqwest`. Test SQL connections are harness-only.

## Tests to Pass

- **`CI-L4-001`** (T1): `cargo check --all-targets` exits `0` after L4 sync modules, reqwest dependency, migrations, and tests are added. Requirements: `L4-FR-1.10.1`, `L4-FR-5.4.1`.
- **`L4-T-sqlite_schema-024`** (T1, repository scripts fixture): root workspace wiring includes `apps/web` through `apps/*`, root scripts actually execute web build/test/lint, and daemon `Cargo.toml` contains the `reqwest` dependency with `rustls-tls` and `json`. Requirements: `L4-FR-3.1.1`, `L4-FR-5.4.1`, `L4-FR-5.4.3`.

## Acceptance Criteria

- [ ] `crates/daemon/Cargo.toml` includes the exact `reqwest` line above.
- [ ] `cargo check --all-targets` exits 0.
- [ ] `cargo clippy --all-targets -- -D warnings` exits 0.
- [ ] No `tokio-postgres`, `ureq`, or `hyper` direct dependency added.
- [ ] `L4-T-sqlite_schema-024` reqwest grep assertion passes.

## Dependencies

- **Blocked by:** none (Phase 1 root, parallel group P1-A).
- **Blocks:** `l4-p2-apply-push-batch-rpc`, `l4-p3-cas-rpc`, all daemon sync code.
- **Phase:** 1.

---

## l4-p1-sqlite-sync-migration: SQLite L4 Sync State Migration

- type: task
- priority: P0
- labels: layer-4,l4-phase-1,sqlite,migration,foundation
- parent: l4-schema-foundation

### Description

## SOURCE OF TRUTH WARNING

- **Spec**: `docs/strategy/layers/layer-4-shared-visibility.md` — §1.3, §1.4, §2.6.11, §3.4.6, §3.6, §5.2 (PRIMARY)
- **TDD**: `docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-tdd.md` — `L4-T-sqlite_schema-001..017` (PRIMARY)
- **Strategy**: `docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-strategy.md` — Phase 1, P1-B
- **Bootstrap**: `docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-bootstrap.md` — §6.1

## Context

The SQLite root migration of L4. Creates 8 tables: `local_outbox`, `sync_batches`, `pull_cursors`, `provisional_worker_outputs`, `enforcement_readiness`, `projection_generations`, `validation_calibration_cases`, `validation_incident_clusters`. Slot `0012` confirmed by spec §1.10/AC5: `0011_commercial_basis_coverage.sql` is the last existing flat migration.

## Files to Create/Modify

- **Create:** `migrations/0012_l4_sync_state.sql` — 8 tables + indices + checks.

## Spec Requirements (verbatim)

**L4-FR-1.3.8 — `local_outbox` stores the full replay envelope.**
- AC1: The `local_outbox` schema includes `commit_group_id` and nullable `writer_epoch` from first materialization in `migrations/0012_l4_sync_state.sql`.
- AC2: `payload_json` is sufficient to reconstruct the exact remote row image for retry or reproject.
- AC3: `local_seq` is monotonic and stable across daemon restart.
- AC4: `content_hash` is persisted with the outbox row and not recomputed differently during retry.
- AC5: `writer_epoch` on `local_outbox` is nullable until the row is assigned to a `sync_batches` envelope; `sync_batches` stores the immutable batch writer epoch used for identical retry.
- AC6: Re-enveloping unacknowledged outbox rows with a newer `writer_epoch` is allowed only after a terminal stale-writer rejection or explicit lease-rehome repair.

**L4-FR-1.3.9 — Enforcement-active slices use a provisional promotion buffer.**
- AC5: `migrations/0012_l4_sync_state.sql` creates `provisional_worker_outputs` with `worker_job_id`, `payload_json`, `result_contract_blob_path`, `created_at`, and `validation_status` constrained to `pending`, `approved`, or `rejected`; `worker_job_id` is indexed and links the provisional row to the existing L3 validation verdict flow.

**L4-FR-1.4.1 — Pull cursors are maintained per remote-owned domain.**
- AC1: `pull_cursors` has independent cursor rows for `shared_source_configs`, `shared_company_profile`, and `operator_decision_heads`.
- AC2: Each cursor stores the last applied `remote_change_seq` for its domain.

**L4-FR-3.4.6 — Enforcement readiness state is durable SQLite state.**
- AC1: `migrations/0012_l4_sync_state.sql` creates `enforcement_readiness` keyed by the enforcement slice tuple, with a deterministic `slice_id` plus `worker_skill_id`, `source_id`, `worker_behavior_version`, and `validator_overlay_version`.
- AC2: `enforcement_readiness.enforcement_state` is constrained to `eligible`, `dry_run`, or `demoted`.
- AC3: Stores `last_evaluated_at`, `wilson_lower_bound_bps`, `false_pass_upper_bound_bps`, `adjudicated_count`, `audited_approvals_count`, nullable `demotion_reason`.
- AC6: Stores active demotion threshold config snapshot.

**L4-FR-5.2.1 — Epoch-repair reproject minimum path is part of L4.**
- `projection_generations` is a local SQLite repair ledger created by this migration: `generation_id INTEGER PRIMARY KEY`, `schema_epoch INTEGER NOT NULL`, `status TEXT CHECK (status IN ('started','published','failed'))`, `started_at`, `completed_at`, `max_local_seq_before_reseed`, `remote_acked_at`, `error_json`.

**L4-FR-2.6.11 — SQLite sync migration owns local sync infrastructure DDL.**
- AC1: `migrations/0012_l4_sync_state.sql` creates `local_outbox`, `sync_batches`, `pull_cursors`, `provisional_worker_outputs`, `enforcement_readiness`, `projection_generations`, `validation_calibration_cases`, and `validation_incident_clusters`.
- AC2: `local_outbox` includes `commit_group_id` and `writer_epoch` when first materialized.

**L4-FR-3.6.1 through L4-FR-3.6.17 — Calibration corpora and harness:**
- AC: `validation_calibration_cases` stores corpus kind, expected/actual verdict, ground truth, mutation family, pair ID, model family, source artifact path, created/adjudicated timestamps.
- AC: `validation_incident_clusters` stores incident signature, dominant failure family, affected slice IDs, candidate mutation template JSON, override linkage.

## Implementation Details (from bootstrap §6.1)

Required tables and assertions:

| Table | Required Assertions |
|-------|---------------------|
| `local_outbox` | full replay envelope, monotonic `local_seq`, `commit_group_id`, nullable `writer_epoch`, persisted `content_hash`, op check (insert/update) |
| `sync_batches` | immutable envelope, in-flight range, status, retry count, response payload, writer epoch, schema epoch, stage run, commit group coverage |
| `pull_cursors` | one row per remote-owned domain (`shared_source_configs`, `shared_company_profile`, `operator_decision_heads`), last applied `remote_change_seq`, no cross-domain serializability requirement |
| `provisional_worker_outputs` | local-only worker output staging with status check (pending/approved/rejected) and `worker_job_id` index |
| `enforcement_readiness` | deterministic slice key, worker/source/behavior versions, state check, bps metrics, count metrics, demotion reason, threshold snapshot |
| `projection_generations` | epoch-repair ledger with status, max local seq, remote ack, error JSON |
| `validation_calibration_cases` | clean/weird/mutated/pairwise/live evidence, expected/actual verdicts, mutation family, pair id, model family |
| `validation_incident_clusters` | incident signature, dominant failure family, affected slices, candidate mutation template JSON |

Required indices:
- `local_outbox` dispatch ordering index on `(executor_id, schema_epoch, stage_run_id, commit_group_id, local_seq)` or equivalent.
- `sync_batches` in-flight lookup index on stream/status.
- `pull_cursors` domain lookup (primary key).
- `provisional_worker_outputs.worker_job_id` index.
- `enforcement_readiness.slice_id` unique index.

L0 cross-domain note: `commit_group_id` is `text NULL`, `writer_epoch` is `integer NULL`. Repo's L0 migration (`migrations/0001_sqlite_state_initial.sql`) does NOT pre-create `local_outbox`; L4 owns first materialization.

## Tests to Pass

- **`L4-T-sqlite_schema-001`** (T1): opening a new state DB applies `migrations/0012_l4_sync_state.sql`, records it in `schema_migrations`, and persists `PRAGMA user_version = 9`.
- **`L4-T-sqlite_schema-002`** (T1): `local_outbox` exists with `local_seq`, `executor_id`, `entity_kind`, `entity_id`, `op`, `row_version`, `stage_run_id`, `schema_epoch`, `commit_group_id`, nullable `writer_epoch`, `payload_json`, `content_hash`, `created_at`; `local_seq` monotonic.
- **`L4-T-sqlite_schema-003`** (T1): `sync_batches` exists with persisted batch envelope fields, in-flight status, local sequence range, immutable `writer_epoch`, `schema_epoch`, `stage_run_id`, `commit_group_id` coverage, retry payload hash.
- **`L4-T-sqlite_schema-004`** (T1): `pull_cursors` has independent primary keys for the 3 remote-owned domains.
- **`L4-T-sqlite_schema-005`** (T1): `provisional_worker_outputs` has the required columns and `validation_status` check; `worker_job_id` indexed.
- **`L4-T-sqlite_schema-006`** (T1): `enforcement_readiness` keyed by deterministic `slice_id` with the full tuple + state/counts/bps/threshold snapshot.
- **`L4-T-sqlite_schema-007`** (T1): `projection_generations` exists with status check, schema epoch, timestamps, max local sequence before reseed, remote ack, error JSON.
- **`L4-T-sqlite_schema-008`** (T1): `validation_calibration_cases` exists with corpus kind, expected/actual verdict, ground truth, mutation family, pair ID, model family, source artifact path, timestamps.
- **`L4-T-sqlite_schema-009`** (T1): `validation_incident_clusters` stores incident signature, dominant family, slice IDs, candidate mutation template JSON, override linkage.
- **`L4-T-sqlite_schema-010`** (T1): all 20 push-owned SQLite logical tables retain `owner_id`, `row_version`, `last_local_seq`, `stage_run_id`, `created_at`, `updated_at` (verified against canonical schema — this migration must not break L0-L3 columns).
- **`L4-T-sqlite_schema-011`** (T1): monetary `_pence` columns and `_bps` columns stay INTEGER and reject non-integer fixture values.
- **`L4-T-sqlite_schema-012`** (T1): applying only `migrations/0001_sqlite_state_initial.sql` does NOT create `local_outbox`; applying L4 migration creates it.
- **`L4-T-sqlite_schema-013`** (T1): reopening an already-migrated DB does not duplicate sync tables/seed rows/indices/migration log.
- **`L4-T-sqlite_schema-014`** (T1): required indices exist (dispatch order, in-flight, pull_cursors, provisional, enforcement_readiness).
- **`L4-T-sqlite_schema-015`** (T1): `local_outbox` constrains `op` to insert/update, stores non-empty `content_hash`, rejects rows missing `commit_group_id`/`stage_run_id`/`schema_epoch`.
- **`L4-T-sqlite_schema-016`** (T1): `pending_operator_intents` retains the L2 queue shape including `base_remote_revision`, `idempotency_key`, `status`, `superseded_by_intent_id`, `queued_at`, latest-intent index needed by L4 reconciliation.
- **`L4-T-sqlite_schema-017`** (T1): `validation_verdicts` retains L3 verdict fields plus override fields and joins to provisional output by `worker_job_id`.

## Acceptance Criteria

- [ ] `migrations/0012_l4_sync_state.sql` exists and creates all 8 tables.
- [ ] `local_outbox.commit_group_id` is `text NULL`; `writer_epoch` is `integer NULL`.
- [ ] `provisional_worker_outputs.validation_status` constrained to `pending`/`approved`/`rejected`.
- [ ] `enforcement_readiness` keyed on deterministic `slice_id` derived from worker_skill_id+source_id+worker_behavior_version+validator_overlay_version.
- [ ] `projection_generations` status check `('started','published','failed')`.
- [ ] All listed indices created.
- [ ] L4-T-sqlite_schema-001..017 pass under `cargo test -p daemon --test sqlite_schema`.

## Dependencies

- **Blocked by:** none (parallel group P1-B root).
- **Blocks:** `l4-p1-sqlite-registry-epoch`, `l4-p1-schema-hash-ci-gate`, `l4-p1-test-inventory-and-fixtures`, `l4-p2-outbox-transaction-writer`, `l4-p4-provisional-storage-access`, `l4-p4-policy-readiness-contract`, `l4-p4-calibration-math-readiness`, `l4-p4-calibration-corpora-and-cases`, `l4-p6-epoch-repair-generation`.
- **Phase:** 1.

---

## l4-p1-sqlite-registry-epoch: SQLite Migration Registry and Default Schema Epoch

- type: task
- priority: P0
- labels: layer-4,l4-phase-1,rust,sqlite
- parent: l4-schema-foundation

### Description

## SOURCE OF TRUTH WARNING

- **Spec**: `docs/strategy/layers/layer-4-shared-visibility.md` — §1.7, §1.10, §2.6.11 (PRIMARY)
- **TDD**: `L4-T-sqlite_schema-001`, `012`, `013` (PRIMARY)
- **Strategy**: Phase 1, P1-B (after sync migration)
- **Bootstrap**: §6.1

## Context

Wire `migrations/0012_l4_sync_state.sql` into `crates/daemon/src/db/sqlite.rs` migration registry, register it at schema epoch 9, and bump `DEFAULT_SCHEMA_EPOCH` from 8 to 9. The existing `open_state_db(path, schema_epoch)` PRAGMA-backed path applies the new migration cleanly.

## Files to Create/Modify

- **Modify:** `crates/daemon/src/db/sqlite.rs` — register migration, bump default epoch.

## Spec Requirements (verbatim)

**L4-FR-1.7.1 — Schema epoch mismatch pauses sync without stopping local operation.**
- AC6: The L4 implementation updates `crates/daemon/src/db/sqlite.rs` migration registration and default epoch so `open_state_db` applies `migrations/0012_l4_sync_state.sql` through the existing PRAGMA-backed path.

**L4-FR-1.10.1 — Layer 4 creates the net-new sync and visibility surfaces.**
- AC2: The daemon adds one new sync module at `crates/daemon/src/db/sync.rs`; current `crates/daemon/src/db/sqlite.rs` remains the single connection wrapper the module builds on.

**L4-FR-2.6.11 — SQLite sync migration owns local sync infrastructure DDL.**
- AC3: The L4 migration is added to the daemon migration registry in `crates/daemon/src/db/sqlite.rs` during implementation.

## Implementation Details (from bootstrap §6.1)

Add:

```rust
const L4_SYNC_STATE_MIGRATION_PATH: &str = "migrations/0012_l4_sync_state.sql";
```

Register in the `MIGRATIONS` list:

```rust
("0012_l4_sync_state", 9, L4_SYNC_STATE_MIGRATION_PATH)
```

`DEFAULT_SCHEMA_EPOCH` becomes `9`.

## Tests to Pass

- **`L4-T-sqlite_schema-001`** (T1): opening a new state DB applies the L4 migration, records it in `schema_migrations`, persists `PRAGMA user_version = 9` through existing `open_state_db` path.
- **`L4-T-sqlite_schema-012`** (T1): applying only L0 migration does not create `local_outbox`; applying L4 migration creates it.
- **`L4-T-sqlite_schema-013`** (T1): reopening an already-migrated DB does not duplicate tables/seed rows/indices/migration log entries.

## Acceptance Criteria

- [ ] `crates/daemon/src/db/sqlite.rs` MIGRATIONS list includes the L4 migration entry at epoch 9.
- [ ] `DEFAULT_SCHEMA_EPOCH = 9`.
- [ ] `cargo check --all-targets` succeeds.
- [ ] L4-T-sqlite_schema-001/012/013 pass.

## Dependencies

- **Blocked by:** `l4-p1-sqlite-sync-migration`.
- **Blocks:** `l4-p1-schema-hash-ci-gate`, `l4-p1-test-inventory-and-fixtures`, `l4-p2-outbox-transaction-writer`, `l4-p2-schema-epoch-pause-and-backup`.
- **Phase:** 1.

---

## l4-p1-supabase-config-and-migration-bundle: Supabase Config and 8 Migrations

- type: task
- priority: P0
- labels: layer-4,l4-phase-1,supabase,migration,foundation
- parent: l4-schema-foundation

### Description

## SOURCE OF TRUTH WARNING

- **Spec**: `docs/strategy/layers/layer-4-shared-visibility.md` — §2.1, §2.2, §2.3, §2.4, §2.5, §2.6 (PRIMARY)
- **TDD**: `CI-L4-007`, `L4-T-sqlite_schema-018..023` (PRIMARY)
- **Strategy**: Phase 1, P1-C
- **Bootstrap**: §6.2, §6.3

## Context

Net-new `infra/supabase/` tree. One bead owns all eight migrations because they form a coherent schema bundle that must apply atomically: projection tables → remote-owned tables → summary tables → RLS → RPCs → metadata + lease → command ledger → decision summaries. Phase 2 finishes RPC behavior; Phase 6 closes RLS final content.

## Files to Create/Modify

- **Create:** `infra/supabase/config.toml`
- **Create:** `infra/supabase/migrations/0001_projection_tables.sql` — 20 push-owned projection tables + 3 junction tables.
- **Create:** `infra/supabase/migrations/0002_remote_owned_tables.sql` — `shared_company_profile`, `shared_source_configs`, `operator_decision_heads`, per-domain `remote_change_seq` sequences/triggers.
- **Create:** `infra/supabase/migrations/0003_summary_tables.sql` — 5 summary tables (`workflow_job_summaries`, `error_manifest_summaries`, `validation_verdict_summaries`, `viewer_sync_status` including `projection_generation`, `executors`).
- **Create:** `infra/supabase/migrations/0004_rls_policies.sql` — RLS policies for viewer/operator/admin/daemon roles across all categories. Daemon SELECT does NOT filter `deleted_at IS NULL` for remote-owned tables.
- **Create:** `infra/supabase/migrations/0005_sync_rpcs.sql` — `apply_push_batch(batch_json, writer_epoch)` + `update_operator_decision(process_id, disposition, base_revision, command_id)` Postgres functions.
- **Create:** `infra/supabase/migrations/0006_schema_metadata.sql` — `schema_metadata` (epoch only) + dedicated `active_writer_lease` table with `writer_epoch`.
- **Create:** `infra/supabase/migrations/0007_command_ledger.sql` — `operator_decision_commands` table.
- **Create:** `infra/supabase/migrations/0008_decision_summaries.sql` — `operator_decision_summaries` push-owned projection.

## Spec Requirements (verbatim)

**L4-FR-2.1.1 through L4-FR-2.1.20 (20 push-owned projection tables):**
1 `procurement_processes`, 2 `contract_notices`, 3 `notice_lots`, 4 `tender_document_sets`, 5 `qualification_runs`, 6 `product_candidates`, 7 `search_profiles`, 8 `search_attempts`, 9 `supplier_quotes`, 10 `candidate_pricing_assessments`, 11 `landed_cost_estimates`, 12 `manifests`, 13 `pipeline_assessments`, 14 `bid_workflow_states`, 15 `process_stage_status`, 16 `process_stage_heads`, 17 `process_relations`, 18 `process_aliases`, 19 `document_extractions`, 20 `document_semantic_extractions`.
- AC1: Each table has a Supabase projection counterpart with columns matching the canonical SQLite logical schema after Postgres type substitution.
- AC2: Each table includes push-owned sync tracking fields needed by `apply_push_batch`: `owner_id`, `row_version`, `last_local_seq`, `stage_run_id`, `created_at`, `updated_at`.
- AC3: Each projection is written only by the daemon through `apply_push_batch` and is read-only for web viewer credentials.
- AC4: Projection DDL avoids SQL foreign keys to local-only tables; cross-authority references remain opaque IDs.

**L4-FR-2.2.1 through L4-FR-2.2.6 (6 summary tables):**
1 `workflow_job_summaries`, 2 `error_manifest_summaries`, 3 `validation_verdict_summaries`, 4 `viewer_sync_status`, 5 `executors`, 6 `operator_decision_summaries`.
- AC1: Each summary projection is written by the daemon through the push batch path and readable by viewer credentials.
- AC2: Each summary omits local-only raw context, full error markdown, validator reasoning, operator identity, company profile fields, raw supplier data, tender document metadata.
- AC3: `operator_decision_summaries` is derived from pulled `operator_decision_heads` and does not expose command ledger history.

**L4-FR-2.3.1 — `shared_company_profile` is remote-owned bidder profile authority.**
- AC1: Supabase creates `shared_company_profile` with `org_id`, bidder identity fields, profile JSON fields, `remote_change_seq`, `remote_revision`, `deleted_at`, timestamps.

**L4-FR-2.3.2 — `shared_source_configs` is remote-owned source enablement authority.**
- AC1: Columns `source_id`, `source_type` (procurement/supplier check), `display_name`, `enabled`, `skill_path`, schedule/config fields, `remote_change_seq`, `remote_revision`, `deleted_at`, timestamps.

**L4-FR-2.3.3 — `operator_decision_heads` is remote-authoritative decision state.**
- AC1: Unique row per `process_id`. `disposition` check `pending|watching|approved|rejected`. `reviewed_basis_hash`, `reviewed_manifest_hash`, `reviewed_at_stage_run_id`, `decision_stale`, `operator_id`, `command_id`, `remote_change_seq`, `remote_revision`, `deleted_at`, timestamps.

**L4-FR-2.4.1, L4-FR-2.4.2, L4-FR-2.4.3 (3 junction tables):**
- `notice_cpv_codes`, `product_commodity_codes`, `lot_cpv_codes`. Push-owned.

**L4-FR-2.5.1 through L4-FR-2.5.10 (RLS matrix):**
| Category | viewer | operator | admin | daemon |
|---|---|---|---|---|
| Push-owned projections (20) | SELECT | SELECT | SELECT | INSERT/UPDATE via RPC |
| Push-owned summaries (5+executors) | SELECT | SELECT | SELECT | INSERT/UPDATE via RPC |
| Remote-owned `shared_company_profile` | — | — | Full CRUD | SELECT (pull) |
| Remote-owned `shared_source_configs` | — | — | Full CRUD | SELECT (pull) |
| Remote-owned `operator_decision_heads` | — | SELECT, UPDATE via CAS RPC | Full CRUD | SELECT (pull) |
| Decision display `operator_decision_summaries` | SELECT | SELECT | SELECT | INSERT/UPDATE via RPC |
| Sync infra `schema_metadata`, `active_writer_lease` | — | — | SELECT/UPDATE | SELECT/UPDATE |
| CAS infra `operator_decision_commands` | — | — | SELECT | INSERT via CAS RPC, SELECT |

- All SELECT policies include `WHERE deleted_at IS NULL` for tombstoned tables EXCEPT daemon role on remote-owned tables (RLS tombstone visibility fix per L4-FR-2.5.10 AC4).

**L4-FR-2.6.1 through L4-FR-2.6.9 — Migration inventory (8 files + config.toml).**
- AC1: `infra/supabase/config.toml` is created with project-local Supabase configuration.
- AC2: All eight Supabase migrations under `infra/supabase/migrations/`.
- AC3: Repo currently has no `infra/` directory; L4 owns creating this tree.

**L4-FR-1.6.2 — Writer fencing token (relevant subset for migration 0006).**
- AC5: `infra/supabase/migrations/0006_schema_metadata.sql` creates a dedicated `active_writer_lease` table for `org_id`, `holder_executor_id`, `lease_expires_at`, `writer_epoch`; `schema_metadata` stores schema epoch metadata only.

**L4-FR-3.7.1 — `viewer_sync_status` projection.**
- AC1: Columns from §3.7 DDL: `org_id` PK, `sync_health` check, `last_push_success_at`, `last_push_attempt_at`, `last_acked_local_seq`, `pending_outbox_count`, `schema_epoch`, `projection_generation`, `daemon_version`, `updated_at`.

## Implementation Details (from bootstrap §6.2, §6.3)

Initialize the local project (Supabase CLI ≥ 2.98.1) from the repository root:

```bash
mkdir -p infra
(cd infra && supabase init)
(cd infra/supabase && supabase start)
(cd infra/supabase && supabase status --output env)
(cd infra/supabase && supabase migration up)
```

`infra/supabase/config.toml` minimum (project_id `procurement-intelligence-l4`, api port 54321, db port 54322, major_version 15, auth+realtime enabled).

Schema mirrors SQLite DDL with Postgres type substitutions (TEXT → text, INTEGER → integer/bigint).

## Tests to Pass

- **`CI-L4-007`** (T1, repo inventory): all 8 Supabase migrations + `infra/supabase/config.toml` + 4 overlay skill dirs + `apps/web` 7-file surface exist.
- **`L4-T-sqlite_schema-018`** (T1): `infra/supabase/config.toml` and exactly eight migrations `0001` through `0008` exist under `infra/supabase/migrations/`.
- **`L4-T-sqlite_schema-019`** (T2, SupabaseLocalStack): applying `0001_projection_tables.sql` creates all 20 projection tables plus the three junction tables with required sync tracking fields and no foreign keys to local-only tables.
- **`L4-T-sqlite_schema-020`** (T2): applying `0003_summary_tables.sql` and `0008_decision_summaries.sql` creates the six summary projections and omits raw context columns by schema (company profile, command history, validator reasoning, full error context).
- **`L4-T-sqlite_schema-021`** (T2): applying `0002_remote_owned_tables.sql` creates remote-owned tables, per-domain `remote_change_seq` sequences/triggers, `remote_revision`, `deleted_at`, and role-compatible tombstone columns.
- **`L4-T-sqlite_schema-022`** (T2): applying `0004_rls_policies.sql` enables RLS on every table category and grants only the matrix permissions for the 4 roles.
- **`L4-T-sqlite_schema-023`** (T2): applying `0006_schema_metadata.sql` creates `schema_metadata` for schema epoch only and a dedicated `active_writer_lease` table with `org_id`, holder, expiry, `writer_epoch`.

## Acceptance Criteria

- [ ] `infra/supabase/config.toml` exists with the bootstrap §6.3 minimum keys.
- [ ] All 8 migration files exist with the `0001`..`0008` exact filenames.
- [ ] All 20 push-owned projection tables created with required sync tracking columns.
- [ ] All 6 summary projections created (5 in 0003 + `operator_decision_summaries` in 0008).
- [ ] All 3 remote-owned tables with monotonic per-domain `remote_change_seq` sequences/triggers.
- [ ] All 3 junction tables created.
- [ ] `active_writer_lease` is a separate table (not embedded in `schema_metadata`).
- [ ] RLS enabled with the matrix in spec §2.5; daemon role can SELECT tombstones on remote-owned tables.
- [ ] `apply_push_batch` and `update_operator_decision` Postgres functions created in 0005 (function bodies may be skeleton — full behavior in P2/P3).
- [ ] CI-L4-007 + L4-T-sqlite_schema-018..023 pass.

## Dependencies

- **Blocked by:** none (parallel group P1-C root; can run in parallel with sqlite migration).
- **Blocks:** `l4-p1-schema-hash-ci-gate`, `l4-p2-apply-push-batch-rpc`, `l4-p2-active-writer-lease`, `l4-p3-cas-rpc`, `l4-p3-pull-cursors`, `l4-p3-command-ledger-retention`, `l4-p4-runner-provisional-promotion`.
- **Phase:** 1.

---

## l4-p1-root-web-workspace-scripts: Root package.json Web Workspace and Scripts

- type: task
- priority: P0
- labels: layer-4,l4-phase-1,workspace,frontend,chokepoint
- parent: l4-schema-foundation

### Description

## SOURCE OF TRUTH WARNING

- **Spec**: `docs/strategy/layers/layer-4-shared-visibility.md` — §3.1.1, §5.4 (PRIMARY)
- **TDD**: `CI-L4-005`, `CI-L4-008`, `L4-T-sqlite_schema-024` (PRIMARY)
- **Strategy**: Phase 1, P1-D
- **Bootstrap**: §5.2, §5.7

## Context

Single edit home for root `package.json`. Add `apps/web` coverage to build/test/lint scripts and preserve the `packages/ui` lint boundary that bans `@tauri-apps/api`, `apps/desktop`, `crates/daemon`, `supabase`, and `transport` imports. The existing `apps/*` workspace entry already includes web once the directory exists; this bead owns the script-level execution.

## Files to Create/Modify

- **Modify:** root `package.json` — `scripts.build`, `scripts.test`, `scripts.lint`.

## Spec Requirements (verbatim)

**L4-FR-3.1.1 — `apps/web/` is a net-new read-only Next.js portal.**
- AC4: Root Bun scripts include `apps/web` in build, test, and lint paths, with workspace coverage confirmed through the existing `apps/*` entry.
- AC7: Root `bun run build` runs an `apps/web` build/typecheck step, either `(cd apps/web && bun run build)` for Next.js or `bunx tsc -p apps/web/tsconfig.json --noEmit` if the web package keeps build and typecheck separate.
- AC8: Root `bun run test` runs `apps/web` tests/E2E or a documented `test:e2e` script, and root `bun run lint` runs the web lint command in addition to preserving the `packages/ui` import-boundary grep.

**L4-FR-5.4.3 — Keep `packages/ui` dependency-clean.**
- AC1: Root lint bans `@tauri-apps/api`, `apps/desktop`, `crates/daemon`, `supabase`, `transport` imports from `packages/ui`.
- AC3: `bun run lint` fails if `packages/ui` imports web/desktop/daemon/Supabase/transport-specific modules.

## Implementation Details (from bootstrap §5.2)

Required workspace shape:

```json
{
  "workspaces": ["apps/*", "packages/*"]
}
```

Required script shape:

```json
{
  "scripts": {
    "build": "(cd packages/ui && bun run build) && (cd packages/intelligence && bun run build) && (cd packages/manifests && bun run build) && bunx tsc -p apps/desktop/tsconfig.json --noEmit && (cd apps/web && bun run build)",
    "lint": "if rg -n '^import .*(@tauri-apps/api|.*apps/desktop|.*crates/daemon|.*supabase|.*transport)' packages/ui; then exit 1; fi && (cd packages/intelligence && bun run lint) && (cd packages/manifests && bun run lint) && (cd apps/web && bun run lint)",
    "test": "(cd packages/ui && bun run test) && (cd apps/desktop && bun run test) && (cd packages/intelligence && bun run test) && (cd packages/manifests && bun run test) && (cd apps/web && bun run test)"
  }
}
```

Web E2E (Playwright) is invoked via `(cd apps/web && bun run test:e2e)` — separate from `bun run test`.

## Tests to Pass

- **`CI-L4-005`** (T1, Bun workspace): `bun run test` exits 0 and includes `packages/ui`, `packages/intelligence`, `packages/manifests`, `apps/desktop`, `apps/web` test paths.
- **`CI-L4-008`** (T1, policy scan): no `bd` terminology in L4 docs; no forbidden `packages/ui` imports from Tauri/desktop/daemon/Supabase/transport modules; `ubs` exits 0 on changed files.
- **`L4-T-sqlite_schema-024`** (T1): root workspace wiring includes `apps/web` through `apps/*`, root scripts actually execute web build/test/lint, daemon `Cargo.toml` contains `reqwest` with `rustls-tls`+`json`.

## Acceptance Criteria

- [ ] Root `package.json` `workspaces` is `["apps/*", "packages/*"]`.
- [ ] `scripts.build` includes `(cd apps/web && bun run build)`.
- [ ] `scripts.test` includes `(cd apps/web && bun run test)`.
- [ ] `scripts.lint` retains the `packages/ui` import-boundary grep AND adds `(cd apps/web && bun run lint)`.
- [ ] CI-L4-005, CI-L4-008, L4-T-sqlite_schema-024 pass.

## Dependencies

- **Blocked by:** none (parallel group P1-D).
- **Blocks:** `l4-p5-web-package-config`, all Phase 5 web beads.
- **Phase:** 1.

---

## l4-p1-schema-hash-ci-gate: Schema Hash Coordination CI Gate

- type: task
- priority: P0
- labels: layer-4,l4-phase-1,ci,redaction
- parent: l4-schema-foundation

### Description

## SOURCE OF TRUTH WARNING

- **Spec**: `docs/strategy/layers/layer-4-shared-visibility.md` — §2.6 (Schema coordination rule), §2.6.10 (PRIMARY)
- **TDD**: `L4-T-redaction_boundary-026` (PRIMARY)
- **Strategy**: Phase 1, P1-E
- **Bootstrap**: §6.4

## Context

CI gate that applies SQLite migrations through `0012_l4_sync_state.sql` and Supabase migrations `0001`..`0008` from scratch, introspects logical column names + nullability + checks + sync tracking fields for projection tables, computes a hash of both schemas, and fails if they differ. Lives as `L4-T-redaction_boundary-026` in `crates/daemon/tests/redaction_boundary.rs`.

## Files to Create/Modify

- **Create or extend:** `crates/daemon/tests/redaction_boundary.rs` — add `L4-T-redaction_boundary-026` schema hash test.
- **Create or extend:** `crates/daemon/tests/support/db.rs` and/or `support/supabase.rs` — schema introspection helpers.

## Spec Requirements (verbatim)

**L4-FR-2.6.10 — SQLite and Supabase logical schemas stay coordinated.**
- AC1: CI applies `migrations/0012_l4_sync_state.sql` and all eight Supabase migrations from scratch in a clean database.
- AC2: CI introspects both logical schemas after migration.
- AC3: CI fails if the computed logical schema hash differs between the SQLite logical source and Supabase projection target.
- AC4: This gate is testable and cannot be replaced by manual inspection.

**L4-FR-2.6.10 and L4-FR-2.6.11 — Schema coordination rules (verbatim).**
- "Both SQLite and Postgres derive from the same canonical logical schema. CI must apply both migrations from scratch, introspect both, and fail if logical schema hash differs."

## Implementation Details (from bootstrap §6.4)

The schema hash gate must:

1. Apply SQLite migrations through `0012_l4_sync_state.sql` from scratch.
2. Apply Supabase migrations `0001` through `0008` from scratch.
3. Introspect logical projection table names, columns, nullability, checks where portable, and sync tracking fields.
4. Fail if the SQLite logical source and Supabase projection target hash differ.

The introspection MUST cover all 20 push-owned projection tables and the 5 summary tables that originate from local SQLite. It MUST NOT include remote-owned tables (those have no SQLite counterpart).

## Tests to Pass

- **`L4-T-redaction_boundary-026`** (T2): schema hash coordination test applies SQLite and Supabase schemas from scratch, introspects logical columns for projection tables, fails on logical mismatch. Requirements: `L4-FR-2.6.10`.

## Acceptance Criteria

- [ ] `L4-T-redaction_boundary-026` exists in `crates/daemon/tests/redaction_boundary.rs`.
- [ ] Test applies both SQLite and Supabase migrations from scratch (uses `SupabaseLocalStack` fixture).
- [ ] Test computes a deterministic logical-schema hash for projection tables on both sides.
- [ ] Test fails when a column is added/removed/typed-differently on one side only.
- [ ] Test passes when both schemas match.
- [ ] Phase 6 will re-run this gate in the integrated environment.

## Dependencies

- **Blocked by:** `l4-p1-sqlite-sync-migration`, `l4-p1-sqlite-registry-epoch`, `l4-p1-supabase-config-and-migration-bundle`.
- **Blocks:** `l4-p1-test-inventory-and-fixtures`, `l4-p1-review-gate`, `l4-p6-final-rls-redaction-closure`.
- **Phase:** 1.

---

## l4-p1-test-inventory-and-fixtures: sqlite_schema.rs File and Support Modules

- type: task
- priority: P0
- labels: layer-4,l4-phase-1,rust,tests
- parent: l4-schema-foundation

### Description

## SOURCE OF TRUTH WARNING

- **Spec**: `docs/strategy/layers/layer-4-shared-visibility.md` — §1.10 (component inventory), §4.2 (test files) (PRIMARY)
- **TDD**: `CI-L4-007`, `CI-L4-008`, `L4-T-sqlite_schema-018`, `024`; full `sqlite_schema.rs` file (PRIMARY)
- **Strategy**: Phase 1, P1-E (after schema hash gate)
- **Bootstrap**: §8, §9.1, §9.2, §9.3

## Context

Create the `crates/daemon/tests/sqlite_schema.rs` test file containing all 24 tests `L4-T-sqlite_schema-001..024`, plus the `support/` modules used by sync and RLS tests in later phases. This is the test-inventory landing for Phase 1; the body content is the union of the L4-T-sqlite_schema-* tests defined in TDD §8.1 and harness scaffolding from bootstrap §9.

## Files to Create/Modify

- **Create:** `crates/daemon/tests/sqlite_schema.rs` — 24 tests `L4-T-sqlite_schema-001..024`.
- **Create:** `crates/daemon/tests/support/db.rs` — extend existing `TestStateDb` with `L4TestStateDb` opening at L4 schema epoch and seeding push-owned rows.
- **Create:** `crates/daemon/tests/support/supabase.rs` — `SupabaseLocalStack` (start_or_reuse, apply_migrations, reset_schema, api_url, db_url, anon_key, service_role_key, viewer_client, operator_client, admin_session, daemon_client, assert_rls_denied) per bootstrap §9.1.
- **Create:** `crates/daemon/tests/support/redaction.rs` — `RedactionSentinelFixture` per bootstrap §9.3.

## Spec Requirements (verbatim)

**L4-FR-1.10.1 — Layer 4 creates the net-new sync and visibility surfaces.**
- AC5: The chosen SQLite migration number `0012` was verified on 2026-05-06 by listing existing flat repo migrations; `0011_commercial_basis_coverage.sql` was the last existing migration.

**§4.2 Test File Listing (4 New, Cumulative 52):**

| File | Coverage |
|------|---------|
| `sqlite_schema.rs` | Full schema creation and migration: all tables created, all constraints enforced, all indices present, sync tracking columns populated, integer pence/bps verified, `provisional_worker_outputs`, `enforcement_readiness`, `projection_generations`, `validation_calibration_cases`, `validation_incident_clusters` created |

## Tests to Pass (the file's job is to be the test surface)

- All 24 `L4-T-sqlite_schema-001..024` tests live in this file (test bodies are the assertions in TDD §8.1).
- **`CI-L4-007`** (T1): repository inventory test — `migrations/0012_l4_sync_state.sql`, all eight `infra/supabase/migrations/*.sql`, `infra/supabase/config.toml`, four overlay skill dirs, `apps/web` 7-file surface exist exactly as declared.
- **`CI-L4-008`** (T1): no skipped/ignored tests, no `bd` terminology in L4 docs, `packages/ui` import boundary clean.
- **`L4-T-sqlite_schema-018`** (T1): inventory of Supabase config + 8 migrations.
- **`L4-T-sqlite_schema-024`** (T1): root workspace + reqwest grep.

## Acceptance Criteria

- [ ] `crates/daemon/tests/sqlite_schema.rs` exists with all 24 test functions, each named with stable `L4-T-sqlite_schema-NNN` ID verbatim in the function name or test annotation.
- [ ] `support/db.rs`, `support/supabase.rs`, `support/redaction.rs` exist with the helpers required by bootstrap §9.
- [ ] Tests use `cargo test -p daemon --test sqlite_schema -- --nocapture`.
- [ ] No `.skip`, `#[ignore]`, `test.todo`, empty bodies, or "works" assertions.
- [ ] CI-L4-007 + CI-L4-008 + L4-T-sqlite_schema-018, 024 pass.

## Dependencies

- **Blocked by:** `l4-p1-schema-hash-ci-gate` (because the schema hash test scaffolding lives alongside).
- **Blocks:** `l4-p1-review-gate`, all Phase 2/3/4/6 sync tests.
- **Phase:** 1.

---

## l4-p1-review-gate: Phase 1 Review Gate — Schema Foundation Cumulative

- type: task
- priority: P0
- labels: layer-4,l4-phase-1,review-gate
- parent: l4-schema-foundation

### Description

## SOURCE OF TRUTH WARNING

- **Spec**: `docs/strategy/layers/layer-4-shared-visibility.md` — §1.10, §2.6 (PRIMARY)
- **TDD**: `CI-L4-001`, `003`, `005`, `007`, `008`; `L4-T-sqlite_schema-001..024`; `L4-T-redaction_boundary-026` (PRIMARY)
- **Strategy**: Phase 1 review row

## Context

Phase 1 cumulative gate. Verifies all 7 Phase 1 task beads are green and ready to unblock Phase 2 + Phase 3 + Phase 4 (DDL-dependent beads) + Phase 5 (workspace-dependent beads).

## Verification Commands

```bash
cargo fmt --check
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test -p daemon --test sqlite_schema -- --nocapture
cargo test -p daemon --test redaction_boundary -- --nocapture --filter L4-T-redaction_boundary-026
bun run test
bun run lint
```

## Tests to Pass (cumulative)

- `CI-L4-001` cargo check.
- `CI-L4-003` cargo fmt.
- `CI-L4-005` bun run test.
- `CI-L4-007` repo inventory.
- `CI-L4-008` policy scan.
- `L4-T-sqlite_schema-001..024` (all 24 schema tests).
- `L4-T-redaction_boundary-026` (schema hash gate).

## Acceptance Criteria

- [ ] All 7 Phase 1 task beads have status closed.
- [ ] All listed Phase 1 gate test IDs pass.
- [ ] Schema hash gate runs in CI.
- [ ] Phase 2/3/4/5 dependencies cleared to start.

## Dependencies

- **Blocked by:** all 7 Phase 1 task beads.
- **Blocks:** `l4-p2-outbox-transaction-writer`, `l4-p2-apply-push-batch-rpc`, `l4-p3-pull-cursors`, `l4-p3-cas-rpc`, `l4-p4-provisional-storage-access`, `l4-p5-web-package-config`.
- **Phase:** 1.

---

# Phase 2 Task Beads — Push Engine

---

## l4-p2-outbox-transaction-writer: Transactional Outbox Writer

- type: task
- priority: P0
- labels: layer-4,l4-phase-2,rust,sync,push
- parent: l4-push-engine

### Description

## SOURCE OF TRUTH WARNING

- **Spec**: `docs/strategy/layers/layer-4-shared-visibility.md` — §1.3.1, §1.3.8, §1.3.9, §1.9.2 (PRIMARY)
- **TDD**: `L4-T-sqlite_supabase_sync-001..003` (PRIMARY)
- **Strategy**: Phase 2, P2-A root
- **Bootstrap**: §9.2 (`OutboxHarness`)

## Context

First push-engine bead. Every push-owned SQLite mutation creates exactly one `local_outbox` entry in the same SQLite transaction as the data write. Sets the foundation for batch envelope assembly. Lives in the new `crates/daemon/src/db/sync.rs` module; the existing `crates/daemon/src/db/sqlite.rs` connection wrapper remains the single connection surface.

## Files to Create/Modify

- **Create:** `crates/daemon/src/db/sync.rs` — module with `OutboxWriter` skeleton + push-owned write path.
- **Modify:** push-owned-write call sites under `crates/daemon/src/` to route through the outbox writer.

## Spec Requirements (verbatim)

**L4-FR-1.3.1 — Transactional outbox is the only push-owned write path after sync activation.**
- AC1: Every push-owned SQLite mutation creates exactly one `local_outbox` entry in the same SQLite transaction as the data write.
- AC2: A transaction that mutates push-owned data and fails to create the matching outbox entry is rejected or rolled back.
- AC3: Enforcement-active rejected provisional results create no push-owned mutation and therefore create no outbox entry.

**L4-FR-1.3.8 — `local_outbox` stores the full replay envelope.**
- AC1-AC4 (paraphrased above in `l4-p1-sqlite-sync-migration`); writer must populate `local_seq`, `executor_id`, `entity_kind`, `entity_id`, `op`, `row_version`, `stage_run_id`, `schema_epoch`, `commit_group_id`, `payload_json`, `content_hash`, `created_at`. `writer_epoch` left NULL until envelope assignment.

**L4-FR-1.9.2 — Outbox + data in same SQLite transaction (Constitutional invariant).**
- Enforcement: every push-owned mutation creates outbox entry atomically.

## Implementation Details

`commit_group_id` is the per-SQLite-transaction grouping token. Generate one per `with_write_transaction` and stamp every outbox row produced inside that transaction.

Outbox writer signature:
```rust
fn write_push_owned_with_outbox(
    tx: &mut SqliteTransaction,
    entity_kind: &str,
    entity_id: &str,
    op: OutboxOp,
    payload_json: &str,
    content_hash: &str,
    schema_epoch: i32,
    stage_run_id: &str,
    commit_group_id: &str,
) -> Result<i64 /* local_seq */>;
```

Cross-domain note (L0): `commit_group_id` is `text NULL`, `writer_epoch` is `integer NULL`. Both are reserved by L0 and materialized by L4.

## Tests to Pass

- **`L4-T-sqlite_supabase_sync-001`** (T2, `L4TestStateDb` + `OutboxHarness`): a push-owned row mutation and its `local_outbox` row commit in the same SQLite transaction with one matching outbox entry and shared `commit_group_id`. Requirements: `L4-FR-1.3.1`, `L4-FR-1.9.2`.
- **`L4-T-sqlite_supabase_sync-002`** (T2, `L4TestStateDb` + `OutboxHarness`): if a push-owned mutation path omits the matching outbox insert, the transaction returns an error or rolls back so neither data row nor outbox row remains. Requirements: `L4-FR-1.3.1`, `L4-FR-1.9.2`.
- **`L4-T-sqlite_supabase_sync-003`** (T2, `EnforcementHarness`): an enforcement-active rejected provisional output creates no push-owned mutation and no outbox entry. Requirements: `L4-FR-1.3.1`, `L4-FR-1.3.9`, `L4-FR-3.4.1`.

## Acceptance Criteria

- [ ] `crates/daemon/src/db/sync.rs` exists with `OutboxWriter` and the function above.
- [ ] All push-owned write paths route through the writer; no path exists that writes a push-owned row without creating the matching outbox entry.
- [ ] `commit_group_id` shared between data row and outbox row.
- [ ] L4-T-sqlite_supabase_sync-001/002/003 pass.

## Dependencies

- **Blocked by:** `l4-p1-review-gate`.
- **Blocks:** `l4-p2-batch-envelope-retry`, `l4-p2-viewer-sync-status-writer`, `l4-p4-runner-provisional-promotion`.
- **Phase:** 2.

---

## l4-p2-batch-envelope-retry: Batch Envelope Assembly and Identical-Envelope Retry

- type: task
- priority: P0
- labels: layer-4,l4-phase-2,rust,sync,push
- parent: l4-push-engine

### Description

## SOURCE OF TRUTH WARNING

- **Spec**: `docs/strategy/layers/layer-4-shared-visibility.md` — §1.3.2, §1.3.3, §1.3.4, §1.3.5, §1.3.8 (PRIMARY)
- **TDD**: `L4-T-sqlite_supabase_sync-004..013, 020, 021` (PRIMARY)
- **Strategy**: Phase 2, P2-A (after outbox writer)
- **Bootstrap**: §9.2

## Context

Batch envelope assembly — single in-flight batch per push stream, persisted `sync_batches` envelope, identical-envelope retry on indeterminate failures. Batches never cross `schema_epoch`/`stage_run_id`/`commit_group_id` boundaries. Row ordering is by `row_version` with deterministic tiebreaker. `content_hash` is computed once and reused on retry.

## Files to Create/Modify

- **Modify:** `crates/daemon/src/db/sync.rs` — add `BatchEnvelopeAssembler`, `SyncBatchPersistence`, retry logic.

## Spec Requirements (verbatim)

**L4-FR-1.3.2 — Push dispatcher sends one in-flight batch per push stream.**
- AC1: The dispatcher never sends a second batch for the same stream until the prior batch is acknowledged, rejected, or moved into retry state.
- AC2: `sync_batches` records the in-flight range so daemon restart cannot create two active batches for the same local sequence range.

**L4-FR-1.3.3 — Failed pushes retry the identical envelope.**
- AC1: Retry payloads preserve the same row set, ordering, `content_hash` values, `schema_epoch`, `stage_run_id`, `commit_group_id` boundaries, and `writer_epoch`.
- AC2: Network timeout or lost response does not rebuild the batch from newly-arrived outbox rows.
- AC3: If the daemon crashes after creating a `sync_batches` row but before receiving the RPC response, restart reloads the persisted batch envelope and retries it before constructing any later batch for that stream.
- AC4: If the remote already committed the crashed batch, equal-version/content-hash semantics return no-op success and the daemon marks the existing `sync_batches` row acknowledged.
- AC5: A terminal stale-writer rejection is not an indeterminate retry case: the daemon marks that `sync_batches` envelope rejected, re-acquires the lease, then may create a new batch envelope over the same unacknowledged outbox rows using the new `writer_epoch`.

**L4-FR-1.3.4 — Batch boundaries preserve epoch, run, and transaction atomicity.**
- AC1: No batch contains rows from more than one `schema_epoch`.
- AC2: No batch contains rows from more than one `stage_run_id`.
- AC3: No batch splits rows that share a `commit_group_id`.

**L4-FR-1.3.5 — Batch row ordering and content hashing make apply idempotent.**
- AC1: Rows inside a batch are ordered by per-row `row_version` and deterministic entity key ordering for ties.
- AC2: Each row carries `content_hash` computed over the canonical payload that Supabase applies.
- AC3: Supabase uses `content_hash` with `row_version` to distinguish idempotent retry from equal-version corruption.

## Tests to Pass

- **`L4-T-sqlite_supabase_sync-004`** (T2): two dispatchable batch ranges queued for the same stream — dispatcher materializes only the first `sync_batches` row and refuses a second in-flight batch until first is acknowledged/rejected/retry. `L4-FR-1.3.2`, `L4-FR-1.9.9`.
- **`L4-T-sqlite_supabase_sync-005`** (T2): daemon restart reloads the persisted in-flight `sync_batches` envelope and does not create a duplicate batch. `L4-FR-1.3.2`, `L4-FR-1.3.3`.
- **`L4-T-sqlite_supabase_sync-006`** (T3): network timeout/lost response retries the exact row set, order, hashes, epoch, stage run, commit groups, writer epoch. `L4-FR-1.3.3`.
- **`L4-T-sqlite_supabase_sync-007`** (T3): crash after batch persistence + before response causes restart to retry the persisted envelope before any later outbox rows are enveloped. `L4-FR-1.3.3`.
- **`L4-T-sqlite_supabase_sync-008`** (T3, `WriterLeaseHarness`): terminal stale-writer rejection marks old batch rejected, re-acquires lease, re-envelopes same unacknowledged rows only with new `writer_epoch`. `L4-FR-1.3.3`, `L4-FR-1.3.8`, `L4-FR-1.6.2`.
- **`L4-T-sqlite_supabase_sync-009`** (T2): batch assembly rejects mixed `schema_epoch` rows. `L4-FR-1.3.4`, `L4-FR-1.9.6`.
- **`L4-T-sqlite_supabase_sync-010`** (T2): batch assembly rejects rows from more than one `stage_run_id`. `L4-FR-1.3.4`, `L4-FR-1.9.7`.
- **`L4-T-sqlite_supabase_sync-011`** (T2): rows sharing `commit_group_id` are sent together or not at all; configured batch size cannot split. `L4-FR-1.3.4`, `L4-FR-1.9.8`.
- **`L4-T-sqlite_supabase_sync-012`** (T2): rows in a batch ordered by `row_version` and deterministic entity key tie-break. `L4-FR-1.3.5`.
- **`L4-T-sqlite_supabase_sync-013`** (T2): `content_hash` computed over canonical payload JSON, persisted on outbox, reused without recomputation drift on retry. `L4-FR-1.3.5`, `L4-FR-1.3.8`.
- **`L4-T-sqlite_supabase_sync-020`** (T2): outbox `local_seq`, `payload_json`, `content_hash`, `commit_group_id`, null pre-envelope `writer_epoch` stable across DB close/reopen. `L4-FR-1.3.8`.
- **`L4-T-sqlite_supabase_sync-021`** (T2): `writer_epoch` remains NULL on `local_outbox` until assignment to `sync_batches`; retries use immutable batch epoch, not mutable row epoch. `L4-FR-1.3.8`, `L4-FR-1.6.2`.

## Acceptance Criteria

- [ ] `BatchEnvelopeAssembler` enforces single-in-flight-per-stream invariant via `sync_batches.status` lookup.
- [ ] Restart-safe: `sync_batches` row persists across daemon restart and is the source of truth for retry payload.
- [ ] Batch never crosses `schema_epoch`/`stage_run_id`/`commit_group_id`.
- [ ] Identical-envelope retry — same content bytes including hash, never recomputed.
- [ ] L4-T-sqlite_supabase_sync-004..013, 020, 021 all pass.

## Dependencies

- **Blocked by:** `l4-p2-outbox-transaction-writer`.
- **Blocks:** `l4-p2-apply-push-batch-rpc`, `l4-p2-stale-writer-reenvelope`, `l4-p2-schema-epoch-pause-and-backup`.
- **Phase:** 2.

---

## l4-p2-apply-push-batch-rpc: apply_push_batch Postgres Function and Daemon HTTP Client

- type: task
- priority: P0
- labels: layer-4,l4-phase-2,rust,supabase,rpc
- parent: l4-push-engine

### Description

## SOURCE OF TRUTH WARNING

- **Spec**: `docs/strategy/layers/layer-4-shared-visibility.md` — §1.3.6, §1.9.1, §2.5.10 (PRIMARY)
- **TDD**: `L4-T-sqlite_supabase_sync-014, 015`, `L4-T-redaction_boundary-023` (PRIMARY)
- **Strategy**: Phase 2, P2-B
- **Bootstrap**: §6.2 (migration 0005)

## Context

Single transactional REST RPC for all push-owned data. Postgres function executes the full batch in one DB transaction; any validation failure rejects the entire batch atomically. The daemon `reqwest` client calls the RPC through Supabase REST. Direct daemon table upserts are NOT a supported sync path. Validates `schema_epoch`, `writer_epoch`, `row_version`, `content_hash`, `stage_run_id`, `commit_group_id` constraints before commit.

## Files to Create/Modify

- **Modify:** `infra/supabase/migrations/0005_sync_rpcs.sql` — `apply_push_batch(batch_json, writer_epoch)` Postgres function body.
- **Modify:** `crates/daemon/src/db/sync.rs` — `SupabaseRpcClient::apply_push_batch` HTTP path.

## Spec Requirements (verbatim)

**L4-FR-1.3.6 — `apply_push_batch` is the only remote apply surface for push-owned data.**
- AC1: The RPC applies the full batch in one Postgres transaction.
- AC2: Any validation failure rejects the full batch and leaves all target rows unchanged.
- AC3: The RPC validates `schema_epoch`, `writer_epoch`, `row_version`, `content_hash`, `stage_run_id`, and `commit_group_id` constraints before committing.
- AC4: Direct table upserts from the daemon are not used for push-owned projection writes.
- AC5: The daemon calls the RPC through the Supabase REST endpoint using the L4 HTTP client dependency from §5.4; implementation agents must not substitute a direct Postgres connection unless the spec is explicitly revised.

**L4-FR-1.9.1 — Push batch atomicity (Constitutional).**
- Single transactional RPC, not individual upserts.

**L4-FR-2.5.10 — Operator and daemon mutation rights stay behind RPC boundaries.**
- AC2: Daemon projection mutation is available only through `apply_push_batch(batch_json, writer_epoch)`.

## Implementation Details

Function signature (Postgres):
```sql
create or replace function apply_push_batch(batch_json jsonb, writer_epoch integer)
returns jsonb
language plpgsql
security definer
as $$
declare
  result jsonb;
begin
  -- Validate writer_epoch matches current active_writer_lease
  -- Validate schema_epoch matches current schema_metadata.schema_epoch
  -- For each row in batch_json: validate row_version, content_hash, stage_run_id, commit_group_id
  -- Apply rows transactionally (atomic across all target tables)
  -- Return acknowledgement payload with applied/rejected sets
  ...
end;
$$;
```

Daemon `reqwest` invocation goes through Supabase REST `/rest/v1/rpc/apply_push_batch` with service-role key; never expose service key to browser.

## Tests to Pass

- **`L4-T-sqlite_supabase_sync-014`** (T2, `SupabaseRpcClient`): `apply_push_batch` applies the full batch in one Postgres transaction; injecting one invalid row leaves every target table unchanged. `L4-FR-1.3.6`, `L4-FR-1.9.1`.
- **`L4-T-sqlite_supabase_sync-015`** (T2, `SupabaseRpcClient` + `RoleMatrixFixture`): daemon sync code uses HTTP RPC and no direct projection table upsert path; direct table write with daemon HTTP credentials is denied or absent. `L4-FR-1.3.6`, `L4-FR-2.5.10`.
- **`L4-T-redaction_boundary-023`** (T2, `RoleMatrixFixture`): daemon direct projection table upsert is denied outside RPC; the same payload succeeds through `apply_push_batch`. `L4-FR-1.3.6`, `L4-FR-2.5.10`.

## Acceptance Criteria

- [ ] `apply_push_batch` Postgres function created in 0005, validates all constraints listed in AC3 before commit.
- [ ] Daemon `reqwest` client calls REST `/rpc/apply_push_batch`; no direct table upsert path.
- [ ] Function uses `security definer` and is owned by `daemon` role; viewer/operator cannot execute.
- [ ] Partial-write injection test: invalid row in batch → entire batch rejected, target tables unchanged.
- [ ] L4-T-sqlite_supabase_sync-014/015 + L4-T-redaction_boundary-023 pass.

## Dependencies

- **Blocked by:** `l4-p1-daemon-http-client`, `l4-p1-supabase-config-and-migration-bundle`, `l4-p1-review-gate`, `l4-p2-batch-envelope-retry`.
- **Blocks:** `l4-p2-equal-version-semantics`, `l4-p2-active-writer-lease`.
- **Phase:** 2.

---

## l4-p2-equal-version-semantics: Equal-Version Retry Four-Case Semantics

- type: task
- priority: P0
- labels: layer-4,l4-phase-2,supabase,rpc
- parent: l4-push-engine

### Description

## SOURCE OF TRUTH WARNING

- **Spec**: `docs/strategy/layers/layer-4-shared-visibility.md` — §1.3 Equal-version retry semantics, §1.3.7, §1.9.10 (PRIMARY)
- **TDD**: `L4-T-sqlite_supabase_sync-016..019` (PRIMARY)
- **Strategy**: Phase 2, P2-B (after RPC)
- **Bootstrap**: §15

## Context

Without explicit equal-version semantics, identical-envelope retries fail spuriously. Implements four cases inside `apply_push_batch`: (1) equal `row_version` + matching `content_hash` → no-op success; (2) equal `row_version` + different `content_hash` → hard conflict; (3) `row_version + 1` → apply; (4) older or larger jump → reject.

## Files to Create/Modify

- **Modify:** `infra/supabase/migrations/0005_sync_rpcs.sql` — extend `apply_push_batch` body with the 4-case branch.

## Spec Requirements (verbatim)

**Equal-version retry semantics (spec §1.3, 4-case table verbatim):**
- Incoming `row_version` = remote `row_version` AND `content_hash` matches → **no-op success** (idempotent retry).
- Incoming `row_version` = remote `row_version` but `content_hash` differs → **hard conflict** (data corruption).
- Incoming `row_version` = remote `row_version` + 1 → **apply** (normal advance).
- Anything else (older, larger jump) → **reject** (stale or out-of-order).

**L4-FR-1.3.7 — Equal-version retry semantics are explicit and testable.**
- AC1: Equal `row_version` plus matching `content_hash` returns no-op success and does not duplicate rows.
- AC2: Equal `row_version` plus different `content_hash` returns a hard-conflict error classified as data corruption.
- AC3: Incoming `row_version = remote row_version + 1` applies as the normal advance case.
- AC4: Older versions and larger jumps reject as stale or out-of-order without mutating the remote row.

**L4-FR-1.9.10 — Equal-version retry = no-op on content_hash match (Constitutional).**
- Identical-envelope retries must not fail spuriously.

## Tests to Pass

- **`L4-T-sqlite_supabase_sync-016`** (T2): equal `row_version` + matching `content_hash` → no-op success, no duplicate, acknowledges local outbox row. `L4-FR-1.3.7`, `L4-FR-1.9.10`.
- **`L4-T-sqlite_supabase_sync-017`** (T2): equal `row_version` + different `content_hash` → hard-conflict data-corruption error, remote row unchanged. `L4-FR-1.3.7`.
- **`L4-T-sqlite_supabase_sync-018`** (T2): incoming `row_version = remote + 1` applies exactly once, updates `last_local_seq`. `L4-FR-1.3.7`.
- **`L4-T-sqlite_supabase_sync-019`** (T2): older versions and larger jumps reject as stale/out-of-order; remote unchanged. `L4-FR-1.3.7`.

## Acceptance Criteria

- [ ] `apply_push_batch` body explicitly handles all four cases with distinct error classification.
- [ ] Hard conflict returns a structured payload identifying the offending row and signaling data corruption (not a transient retryable error).
- [ ] Larger-jump (>= +2) is rejected (not silently filled).
- [ ] L4-T-sqlite_supabase_sync-016..019 pass.

## Dependencies

- **Blocked by:** `l4-p2-apply-push-batch-rpc`.
- **Blocks:** `l4-p2-review-gate`.
- **Phase:** 2.

---

## l4-p2-viewer-sync-status-writer: viewer_sync_status Daemon Write Path

- type: task
- priority: P1
- labels: layer-4,l4-phase-2,rust,sync,status
- parent: l4-push-engine

### Description

## SOURCE OF TRUTH WARNING

- **Spec**: `docs/strategy/layers/layer-4-shared-visibility.md` — §1.8.1, §3.3.4, §3.7.1 (PRIMARY)
- **TDD**: `L4-T-sqlite_supabase_sync-046` (PRIMARY)
- **Strategy**: Phase 2, P2-C
- **Bootstrap**: §9.2

## Context

Daemon updates `viewer_sync_status` after each successful remote apply AND on lease-heartbeat/status-only updates. This row drives the three-state freshness UX (green/amber/red) shared by web and desktop. Heartbeat-only commits update health/timestamps even when no projection rows changed.

## Files to Create/Modify

- **Modify:** `crates/daemon/src/db/sync.rs` — `viewer_sync_status` update calls after `apply_push_batch` ack and on lease heartbeat.

## Spec Requirements (verbatim)

**L4-FR-1.8.1 — Sync health exposes three distinct operating states.**
- AC4: `viewer_sync_status.sync_health` records the state used by web and desktop surfaces.

**L4-FR-3.3.4 — `viewer_sync_status` drives freshness state across surfaces.**
- AC1: Stores `syncHealth`, `lastPushSuccessAt`, `lastPushAttemptAt`, `lastAckedLocalSeq`, `pendingOutboxCount`, `schemaEpoch`, `projectionGeneration`, `daemonVersion`.
- AC2: The row updates after each successful remote apply and during lease-heartbeat/status-only updates.

**L4-FR-3.7.1 — `viewer_sync_status` is the Supabase sync freshness projection.**
- AC1-AC5 (full DDL + lease-heartbeat update behavior + RLS + `projection_generation` increment after epoch-repair reseed).

## Tests to Pass

- **`L4-T-sqlite_supabase_sync-046`** (T2, `WriterLeaseHarness`): `viewer_sync_status` records healthy/degraded/offline health, timestamps, last acked sequence, pending outbox count, schema epoch, projection generation, daemon version, and updates on heartbeat/status-only commits. `L4-FR-1.8.1`, `L4-FR-3.3.4`, `L4-FR-3.7.1`.

## Acceptance Criteria

- [ ] After every successful `apply_push_batch` ack, daemon writes `viewer_sync_status` with refreshed timestamps + health + acked seq + pending count.
- [ ] On lease heartbeat (every ≤30s while sync active), daemon writes `viewer_sync_status` with refreshed `last_push_attempt_at`/health/expiry-derived state even if no rows changed.
- [ ] Health computed from time delta + connectivity per spec §3.3 thresholds (90s green, 5min amber, >5min red).
- [ ] L4-T-sqlite_supabase_sync-046 passes.

## Dependencies

- **Blocked by:** `l4-p2-outbox-transaction-writer`.
- **Blocks:** `l4-p5-web-status-page`, `l4-p5-web-freshness-consumption-and-boundary`.
- **Phase:** 2.

---

## l4-p2-active-writer-lease: Active-Writer Lease and Writer Epoch Fencing

- type: task
- priority: P0
- labels: layer-4,l4-phase-2,rust,supabase,sync,fencing
- parent: l4-push-engine

### Description

## SOURCE OF TRUTH WARNING

- **Spec**: `docs/strategy/layers/layer-4-shared-visibility.md` — §1.6.1, §1.6.2, §1.6.3, §1.9.5 (PRIMARY)
- **TDD**: `L4-T-sqlite_supabase_sync-039, 040, 042, 043` (PRIMARY)
- **Strategy**: Phase 2, P2-D
- **Bootstrap**: §6.2 (migration 0006), §11 (heartbeat)

## Context

One daemon per org in MVP. Active-writer lease enforces single-writer at the push protocol level via `writer_epoch` fencing token. Lease acquisition is CAS based: a daemon can acquire only when `holder_executor_id` is itself, NULL, or `lease_expires_at < now()`. Heartbeat every ≤30s extends `lease_expires_at` to ≥120s. Stale daemon rejected.

## Files to Create/Modify

- **Modify:** `infra/supabase/migrations/0006_schema_metadata.sql` — confirm `active_writer_lease` table CAS UPDATE function (or write helper).
- **Modify:** `crates/daemon/src/db/sync.rs` — `WriterLease::acquire`, `::heartbeat`, `::release`, `::current_epoch` paths.

## Spec Requirements (verbatim)

**L4-FR-1.6.1 — Active-writer lease rejects concurrent pushers.**
- AC1: First pushing daemon registers/renews lease ownership with its `executor_id`.
- AC2: Second daemon with different `executor_id` rejected from push apply while lease is valid.
- AC3: Lease heartbeat and freshness visible through `viewer_sync_status`.

**L4-FR-1.6.2 — Writer fencing token prevents stale-daemon races.**
- AC1: Lease acquisition increments `writer_epoch`.
- AC2: Every `apply_push_batch` includes daemon's current `writer_epoch`.
- AC3: Supabase rejects any batch whose `writer_epoch` does not match the active lease row.
- AC4: A partitioned daemon that recovers after lease loss cannot push stale batches.
- AC5: `infra/supabase/migrations/0006_schema_metadata.sql` creates a dedicated `active_writer_lease` table for `org_id`, `holder_executor_id`, `lease_expires_at`, `writer_epoch`; `schema_metadata` stores schema epoch metadata only.
- AC6: Lease acquisition is compare-and-swap based: a daemon can acquire only when `holder_executor_id` is itself, NULL, or `lease_expires_at < now()`, and acquisition increments `writer_epoch` in the same transaction.
- AC7: Lease heartbeat runs at least every 30 seconds while sync is active and extends `lease_expires_at` to at least 120 seconds from heartbeat time.
- AC8: If a daemon crashes without releasing the lease, another writer can acquire only after `lease_expires_at` passes; stale batches from crashed daemon remain rejected by `writer_epoch`.

**L4-FR-1.6.3 — MVP operational binding is explicit.**
- AC1: Operator/admin docs state only one bound machine per org may run writer-mode daemon.
- AC4: Lease conflict surfaces in desktop sync status with current holder, expiry, next safe action.

**L4-FR-1.9.5 — Active-writer lease with fencing token (Constitutional).**

## Tests to Pass

- **`L4-T-sqlite_supabase_sync-039`** (T2, `WriterLeaseHarness`): first daemon executor acquires/renews lease and increments/preserves `writer_epoch` per holder/expiry rules. `L4-FR-1.6.1`, `L4-FR-1.6.2`.
- **`L4-T-sqlite_supabase_sync-040`** (T2): second daemon with different executor ID rejected while lease valid; desktop sync status receives current holder + expiry. `L4-FR-1.6.1`, `L4-FR-1.6.3`.
- **`L4-T-sqlite_supabase_sync-042`** (T2): heartbeat at least every 30s, extends expiry ≥120s, another writer can acquire only after expiry. `L4-FR-1.6.2`.
- **`L4-T-sqlite_supabase_sync-043`** (T1, config/doc fixture): writer-mode operational binding rejects unapproved second writer config and surfaces viewer-only/explicit rehome next actions. `L4-FR-1.6.3`.

## Acceptance Criteria

- [ ] `active_writer_lease` table with `org_id`, `holder_executor_id`, `lease_expires_at`, `writer_epoch`.
- [ ] CAS-based acquire: only when self / NULL / expired; increments `writer_epoch` atomically.
- [ ] Heartbeat interval ≤30s; extends to ≥120s.
- [ ] Lease conflict response includes holder + expiry for desktop UX.
- [ ] L4-T-sqlite_supabase_sync-039/040/042/043 pass.

## Dependencies

- **Blocked by:** `l4-p1-supabase-config-and-migration-bundle`, `l4-p2-apply-push-batch-rpc`.
- **Blocks:** `l4-p2-stale-writer-reenvelope`, `l4-p6-main-startup-sync-wiring`.
- **Phase:** 2.

---

## l4-p2-stale-writer-reenvelope: Stale-Writer Rejection and Re-Envelope Path

- type: task
- priority: P0
- labels: layer-4,l4-phase-2,rust,sync,fencing
- parent: l4-push-engine

### Description

## SOURCE OF TRUTH WARNING

- **Spec**: `docs/strategy/layers/layer-4-shared-visibility.md` — §1.3.3 AC5, §1.3.8 AC6, §1.6.2 (PRIMARY)
- **TDD**: `L4-T-sqlite_supabase_sync-008, 041` (PRIMARY)
- **Strategy**: Phase 2, P2-D (after lease)

## Context

Terminal stale-writer rejection is NOT an indeterminate retry. The daemon marks the rejected `sync_batches` envelope rejected, re-acquires the lease (which increments `writer_epoch`), and re-envelopes the same unacknowledged outbox rows with the new `writer_epoch`. This is the only path that allows re-enveloping with a new epoch.

## Files to Create/Modify

- **Modify:** `crates/daemon/src/db/sync.rs` — terminal rejection branch + re-envelope helper.

## Spec Requirements (verbatim)

**L4-FR-1.3.3 — Failed pushes retry the identical envelope (relevant ACs).**
- AC5: A terminal stale-writer rejection is not an indeterminate retry case: the daemon marks that `sync_batches` envelope rejected, re-acquires the lease, then may create a new batch envelope over the same unacknowledged outbox rows using the new `writer_epoch`.

**L4-FR-1.3.8 — `local_outbox` stores the full replay envelope (relevant AC).**
- AC6: Re-enveloping unacknowledged outbox rows with a newer `writer_epoch` is allowed only after a terminal stale-writer rejection or explicit lease-rehome repair, never after an indeterminate network failure.

**L4-FR-1.6.2 — Writer fencing token prevents stale-daemon races (relevant AC).**
- AC4: A partitioned daemon that recovers after lease loss cannot push stale batches.

## Tests to Pass

- **`L4-T-sqlite_supabase_sync-008`** (T3, `WriterLeaseHarness` + `OutboxHarness`): terminal stale-writer rejection marks old batch rejected, re-acquires lease, re-envelopes same unacknowledged rows only with new `writer_epoch`. `L4-FR-1.3.3`, `L4-FR-1.3.8`, `L4-FR-1.6.2`.
- **`L4-T-sqlite_supabase_sync-041`** (T3, `WriterLeaseHarness` + `SupabaseRpcClient`): partitioned daemon recovering with old `writer_epoch` cannot push; remote rejects batch even if local rows otherwise valid. `L4-FR-1.6.2`, `L4-FR-1.9.5`.

## Acceptance Criteria

- [ ] Terminal stale-writer detection from RPC response.
- [ ] Re-envelope helper preserves outbox row content but recomputes envelope `writer_epoch`.
- [ ] Indeterminate (network/timeout) failures do NOT trigger re-envelope.
- [ ] L4-T-sqlite_supabase_sync-008/041 pass.

## Dependencies

- **Blocked by:** `l4-p2-active-writer-lease`.
- **Blocks:** `l4-p2-review-gate`.
- **Phase:** 2.

---

## l4-p2-schema-epoch-pause-and-backup: Schema Epoch Mismatch Pause and Encrypted Backup

- type: task
- priority: P1
- labels: layer-4,l4-phase-2,rust,sync,epoch
- parent: l4-push-engine

### Description

## SOURCE OF TRUTH WARNING

- **Spec**: `docs/strategy/layers/layer-4-shared-visibility.md` — §1.7.1, §1.8.3, §1.9.6 (PRIMARY)
- **TDD**: `L4-T-sqlite_supabase_sync-044, 048` (PRIMARY)
- **Strategy**: Phase 2, P2-E (after batch envelope)

## Context

Push batch carries `schema_epoch`. RPC validates against remote `schema_metadata.schema_epoch`. Mismatch pauses push/pull sync without stopping local pipeline operation; emits operator-visible status. Repair convergence is owned by Phase 6. Phase 2 also schedules encrypted WAL-aware SQLite snapshots; failures surface as operational warnings without blocking work.

## Files to Create/Modify

- **Modify:** `crates/daemon/src/db/sync.rs` — schema epoch validation + sync pause state machine + backup scheduler.

## Spec Requirements (verbatim)

**L4-FR-1.7.1 — Schema epoch mismatch pauses sync without stopping local operation.**
- AC1: Every push batch includes `schema_epoch`.
- AC2: Local SQLite `PRAGMA user_version` and remote `schema_metadata.schema_epoch` are compared before apply.
- AC3: Mismatch pauses push/pull sync and emits operator/admin-visible status.
- AC4: Local pipeline execution and desktop local operation continue while sync is paused.
- AC5: Remote projection writes do not resume until epoch repair and reproject minimum path in §5.2 completes.

**L4-FR-1.8.3 — Local authority has explicit backup and durability behavior.**
- AC1: The daemon documents and schedules encrypted WAL-aware local SQLite snapshots.
- AC2: Snapshot failures surface as operational warnings without blocking local pipeline work.
- AC3: Single-device pre-push loss remains an explicit MVP limitation in operator/admin documentation.

**L4-FR-1.9.6 — `schema_epoch` on every batch (Constitutional).**

## Tests to Pass

- **`L4-T-sqlite_supabase_sync-044`** (T2, `SupabaseRpcClient` + `L4TestStateDb`): schema epoch mismatch pauses push/pull sync, emits operator-visible status, leaves local pipeline writes available, prevents projection writes. `L4-FR-1.7.1`, `L4-FR-1.9.6`.
- **`L4-T-sqlite_supabase_sync-048`** (T1, backup fixture): encrypted WAL-aware SQLite snapshot scheduling records success/failure warnings and does not block local pipeline work. `L4-FR-1.8.3`.

## Acceptance Criteria

- [ ] Push pause state on schema epoch mismatch; resumes only after Phase 6 repair.
- [ ] Backup scheduler runs at configured cadence; failures logged as warnings.
- [ ] Backup is WAL-aware (uses SQLite online backup API or equivalent).
- [ ] Local pipeline writes continue during sync pause.
- [ ] L4-T-sqlite_supabase_sync-044/048 pass.

## Dependencies

- **Blocked by:** `l4-p1-sqlite-registry-epoch`, `l4-p2-batch-envelope-retry`.
- **Blocks:** `l4-p2-review-gate`, `l4-p6-epoch-repair-generation`.
- **Phase:** 2.

---

## l4-p2-review-gate: Phase 2 Review Gate — Push Engine Cumulative

- type: task
- priority: P0
- labels: layer-4,l4-phase-2,review-gate
- parent: l4-push-engine

### Description

## SOURCE OF TRUTH WARNING

- **Spec**: §1.3, §1.6, §1.7, §1.8, §1.9 (PRIMARY)
- **TDD**: `L4-T-sqlite_supabase_sync-001..024, 039..046, 048`; `L4-T-redaction_boundary-023` (PRIMARY)
- **Strategy**: Phase 2 review row

## Context

Phase 2 cumulative gate. Verifies push trunk operational: outbox writer atomic, batch envelope retry-safe, RPC apply transactional, equal-version semantics correct, lease/fencing in place, viewer_sync_status updates, schema-epoch pause works. Closes Phase 2 to unblock Phase 6 startup wiring (which also needs Phase 3).

## Verification Commands

```bash
cargo fmt --check
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test -p daemon --test sqlite_supabase_sync -- --nocapture
cargo test -p daemon --test redaction_boundary -- --nocapture --filter L4-T-redaction_boundary-023
```

## Tests to Pass (cumulative)

- `L4-T-sqlite_supabase_sync-001..024` (push trunk).
- `L4-T-sqlite_supabase_sync-039..046, 048` (lease, status, epoch, backup).
- `L4-T-redaction_boundary-023` (direct upsert denial).

## Acceptance Criteria

- [ ] All 8 Phase 2 task beads closed.
- [ ] All listed Phase 2 gate IDs pass.
- [ ] Push-owned mutation only through `apply_push_batch`.
- [ ] Phase 6 startup-wiring dependency cleared on push-side.

## Dependencies

- **Blocked by:** all 8 Phase 2 task beads.
- **Blocks:** `l4-p6-main-startup-sync-wiring`.
- **Phase:** 2.

---

# Phase 3 Task Beads — Pull Engine and CAS

---

## l4-p3-pull-cursors: Per-Domain Pull Cursors

- type: task
- priority: P0
- labels: layer-4,l4-phase-3,rust,sync,pull
- parent: l4-pull-cas

### Description

## SOURCE OF TRUTH WARNING

- **Spec**: `docs/strategy/layers/layer-4-shared-visibility.md` — §1.4.1, §2.3, §2.6.2 (PRIMARY)
- **TDD**: `L4-T-sqlite_supabase_sync-025` (PRIMARY)
- **Strategy**: Phase 3, P3-A root
- **Bootstrap**: §9.2 (`PullCursorHarness`)

## Context

Independent pull cursors for `shared_source_configs`, `shared_company_profile`, `operator_decision_heads`. Each cursor stores last applied `remote_change_seq` for its domain. A failed pull in one domain does not advance that cursor and does not require cross-domain serializability — domains are independent.

## Files to Create/Modify

- **Modify:** `crates/daemon/src/db/sync.rs` — `PullCursor` struct + per-domain pull entry points.

## Spec Requirements (verbatim)

**L4-FR-1.4.1 — Pull cursors are maintained per remote-owned domain.**
- AC1: `pull_cursors` has independent cursor rows for `shared_source_configs`, `shared_company_profile`, `operator_decision_heads`.
- AC2: Each cursor stores the last applied `remote_change_seq` for its domain.
- AC3: A failed pull for one domain does not advance that domain cursor and does not require cross-domain serializability.

## Tests to Pass

- **`L4-T-sqlite_supabase_sync-025`** (T2, `PullCursorHarness`): three domains maintain independent cursor rows; failed pull in one domain does not advance that cursor and does not alter the other two domain cursors. `L4-FR-1.4.1`.

## Acceptance Criteria

- [ ] `pull_cursors` rows created at first sync init for all 3 domains.
- [ ] Per-domain pull entry advances only its own cursor on success.
- [ ] Failed pull leaves cursor unchanged.
- [ ] L4-T-sqlite_supabase_sync-025 passes.

## Dependencies

- **Blocked by:** `l4-p1-review-gate`.
- **Blocks:** `l4-p3-hiwater-pagination`, `l4-p3-pull-before-push-service-contract`, `l4-p3-remote-config-staleness-ttl`.
- **Phase:** 3.

---

## l4-p3-hiwater-pagination: Hi-Water Fenced Pull Pagination

- type: task
- priority: P0
- labels: layer-4,l4-phase-3,rust,sync,pull,fencing
- parent: l4-pull-cas

### Description

## SOURCE OF TRUTH WARNING

- **Spec**: `docs/strategy/layers/layer-4-shared-visibility.md` — §1.4.2, §1.9.14, §2.3.1, §2.3.2, §2.3.3 (PRIMARY)
- **TDD**: `L4-T-sqlite_supabase_sync-026, 027` (PRIMARY)
- **Strategy**: Phase 3, P3-A (after pull cursors)

## Context

Without a hi-water fence, paginated pulls can skip or duplicate rows when remote changes occur during pagination. Algorithm: read `hiwater = max(remote_change_seq)` once, fetch `cursor < change_seq <= hiwater` in pages, apply in `change_seq` order, advance cursor to hiwater. Per-domain monotonic; cross-domain serializability NOT required.

## Files to Create/Modify

- **Modify:** `crates/daemon/src/db/sync.rs` — pagination algorithm + concurrent-write fence test hooks.

## Spec Requirements (verbatim)

**L4-FR-1.4.2 — Pulls use a hi-water fence to survive concurrent remote changes.**
- AC1: Each pull reads `hiwater` before fetching pages for the domain.
- AC2: Every page query is bounded by `cursor < remote_change_seq <= hiwater`.
- AC3: Local application is ordered by `remote_change_seq`.
- AC4: The cursor advances to `hiwater` only after all rows up to `hiwater` are applied.
- AC5: Remote changes with `remote_change_seq > hiwater` wait for the next pull.
- AC6: Supabase migrations create or trigger a domain-level monotonic `remote_change_seq` source for each remote-owned domain; tests fail if two changes in one domain can reuse or move backward in `remote_change_seq`.

**L4-FR-1.9.14 — Fenced pull-before-push ordering (Constitutional, hi-water subclause).**

## Tests to Pass

- **`L4-T-sqlite_supabase_sync-026`** (T3, `PullCursorHarness`): pull reads `hiwater`, paginates `cursor < remote_change_seq <= hiwater`, injects concurrent `remote_change_seq > hiwater` write, applies only fenced range, advances cursor to hiwater, reads injected write on next pull without skip/duplicate. `L4-FR-1.4.2`, `L4-FR-1.9.14`.
- **`L4-T-sqlite_supabase_sync-027`** (T2, `PullCursorHarness`): each remote-owned domain assigns monotonic, non-reused `remote_change_seq` values on insert, update, and tombstone writes. `L4-FR-1.4.2`, `L4-FR-2.3.1`, `L4-FR-2.3.2`, `L4-FR-2.3.3`.

## Acceptance Criteria

- [ ] Pull algorithm implements the 4-step fence protocol verbatim.
- [ ] Domain `remote_change_seq` triggers verified monotonic and non-reused (test #027).
- [ ] Concurrent-write injection test demonstrates the fence preserves correctness.
- [ ] L4-T-sqlite_supabase_sync-026/027 pass.

## Dependencies

- **Blocked by:** `l4-p3-pull-cursors`.
- **Blocks:** `l4-p3-pull-before-push-service-contract`.
- **Phase:** 3.

---

## l4-p3-pull-before-push-service-contract: Pull-Before-Push Startup and Cycle Contract

- type: task
- priority: P0
- labels: layer-4,l4-phase-3,rust,sync,ordering
- parent: l4-pull-cas

### Description

## SOURCE OF TRUTH WARNING

- **Spec**: `docs/strategy/layers/layer-4-shared-visibility.md` — §1.4.3, §1.8.1, §1.9.14 (PRIMARY)
- **TDD**: `L4-T-sqlite_supabase_sync-028, 029` (PRIMARY)
- **Strategy**: Phase 3, P3-B

## Context

On daemon sync startup AND every scheduled/manual sync cycle: pull remote-owned state FIRST, then classify/coalesce pending intents, then resume CAS flushing and push dispatch. Pull failure pauses cross-authority push dispatch but local SQLite operation continues in degraded mode. This contract is asserted at Phase 3 layer; final `main.rs` startup wiring lands in Phase 6.

## Files to Create/Modify

- **Modify:** `crates/daemon/src/db/sync.rs` — `SyncCycle::run()` orchestration, `start_sync_cycle()` ordering invariants.

## Spec Requirements (verbatim)

**L4-FR-1.4.3 — Sync cycles pull remote-owned state before pushing.**
- AC1: On daemon sync startup and every scheduled/manual sync cycle, remote-owned pulls complete before pending operator intents are flushed.
- AC2: CAS flushing and push dispatch resume only after pull completion and local intent classification/coalescing for that cycle.
- AC3: A pull failure leaves push dispatch paused for cross-authority operator decisions while local operation continues in degraded mode.

**L4-FR-1.8.1 — Sync health states (relevant: degraded mode behavior).**

**L4-FR-1.9.14 — Fenced pull-before-push ordering (Constitutional).**

## Tests to Pass

- **`L4-T-sqlite_supabase_sync-028`** (T2, `PullCursorHarness` + `CasCommandHarness`): sync startup and every sync cycle complete remote-owned pulls before pending-intent reconciliation, CAS flush, or push dispatch. `L4-FR-1.4.3`, `L4-FR-1.9.14`.
- **`L4-T-sqlite_supabase_sync-029`** (T2, `PullCursorHarness`): pull failure leaves CAS flushing and push dispatch for cross-authority decisions paused while local SQLite operation continues in degraded mode. `L4-FR-1.4.3`, `L4-FR-1.8.1`.

## Acceptance Criteria

- [ ] `SyncCycle::run()` enforces strict ordering: pull → reconcile intents → CAS flush + push dispatch.
- [ ] Pull failure → CAS-flush/push-dispatch paused but local operation continues.
- [ ] Phase 6 startup wiring will reuse this contract from `main.rs`.
- [ ] L4-T-sqlite_supabase_sync-028/029 pass.

## Dependencies

- **Blocked by:** `l4-p3-pull-cursors`, `l4-p3-hiwater-pagination`.
- **Blocks:** `l4-p3-pending-intent-reconciliation`, `l4-p6-main-startup-sync-wiring`.
- **Phase:** 3.

---

## l4-p3-cas-rpc: update_operator_decision CAS RPC

- type: task
- priority: P0
- labels: layer-4,l4-phase-3,supabase,rpc,cas
- parent: l4-pull-cas

### Description

## SOURCE OF TRUTH WARNING

- **Spec**: `docs/strategy/layers/layer-4-shared-visibility.md` — §1.5.1, §2.3.3, §2.5.10 (PRIMARY)
- **TDD**: `L4-T-sqlite_supabase_sync-030, 031, 033`, `L4-T-redaction_boundary-022` (PRIMARY)
- **Strategy**: Phase 3, P3-C
- **Bootstrap**: §6.2 (migration 0005)

## Context

Postgres function `update_operator_decision(process_id, disposition, base_revision, command_id)`. Mismatch on `base_remote_revision` returns structured conflict (NOT silently overwritten). `command_id` enables idempotent retry via durable command ledger. Direct UPDATE blocked by RLS — only RPC path exposed.

## Files to Create/Modify

- **Modify:** `infra/supabase/migrations/0005_sync_rpcs.sql` — `update_operator_decision` Postgres function body.
- **Modify:** `crates/daemon/src/db/sync.rs` — `SupabaseRpcClient::update_operator_decision`.

## Spec Requirements (verbatim)

**L4-FR-1.5.1 — Operator decisions mutate only through CAS.**
- AC1: The RPC rejects when `base_revision` differs from current `operator_decision_heads.remote_revision`.
- AC2: A conflict response returns structured conflict data and does not overwrite the current decision.
- AC3: Desktop conflict resolution UI receives enough state to refresh and retry intentionally.
- AC4: Direct table UPDATE is blocked for operator decision mutation paths outside the RPC.
- AC6: On CAS conflict, daemon emits a `decision.conflict` IPC event carrying current `remote_revision`, current `disposition`, and the rejected intent payload.

**L4-FR-2.3.3 — `operator_decision_heads` is remote-authoritative decision state.**
- AC2: `remote_revision` increments only through CAS RPC or admin-controlled maintenance path.
- AC4: Daemon pulls decision heads for local display/reconciliation and does not directly update outside CAS.

**L4-FR-2.5.10 — Operator and daemon mutation rights stay behind RPC boundaries.**
- AC1: Operator decision mutation only through `update_operator_decision()`.
- AC3: Direct UPDATE blocked for operator decision heads except admin maintenance paths.

## Implementation Details

```sql
create or replace function update_operator_decision(
  process_id text, disposition text, base_revision bigint, command_id text
) returns jsonb
language plpgsql security definer
as $$
declare
  ledger_existing record;
  current_head record;
begin
  -- Step 1: check command ledger for command_id (idempotent replay; full behavior in l4-p3-command-ledger-retention)
  -- Step 2: select current operator_decision_heads row for process_id
  -- Step 3: if base_revision = current.remote_revision: apply, increment revision, write ledger row, return success
  -- Step 4: else: write conflict ledger row, return structured conflict {current_revision, current_disposition, rejected_payload}
end;
$$;
```

## Tests to Pass

- **`L4-T-sqlite_supabase_sync-030`** (T2, `CasCommandHarness`): queued intent with matching `base_remote_revision` flushes through CAS RPC and marks queue row applied after remote head advances. `L4-FR-1.4.4`, `L4-FR-1.5.1`.
- **`L4-T-sqlite_supabase_sync-031`** (T2, `CasCommandHarness`): two concurrent operator decisions start from same `base_remote_revision`; first applies, second marked conflicted locally and surfaces structured conflict data without overwriting remote head. `L4-FR-1.4.4`, `L4-FR-1.5.1`.
- **`L4-T-sqlite_supabase_sync-033`** (T2, `CasCommandHarness`): CAS conflict response carries current remote revision, disposition, rejected payload, drives `decision.conflict` IPC event payload. `L4-FR-1.5.1`.
- **`L4-T-redaction_boundary-022`** (T2, `RoleMatrixFixture`): operator direct UPDATE to `operator_decision_heads` fails while `update_operator_decision` RPC succeeds or conflicts according to base revision. `L4-FR-1.5.1`, `L4-FR-2.5.10`.

## Acceptance Criteria

- [ ] `update_operator_decision` Postgres function created in 0005 with security definer.
- [ ] Conflict response is structured (not silent overwrite).
- [ ] Daemon `reqwest` client wraps the RPC.
- [ ] L4-T-sqlite_supabase_sync-030/031/033 + L4-T-redaction_boundary-022 pass.

## Dependencies

- **Blocked by:** `l4-p1-supabase-config-and-migration-bundle`, `l4-p1-daemon-http-client`, `l4-p1-review-gate`.
- **Blocks:** `l4-p3-command-ledger-retention`, `l4-p3-pending-intent-reconciliation`.
- **Phase:** 3.

---

## l4-p3-command-ledger-retention: operator_decision_commands Ledger and 30-Day GC

- type: task
- priority: P0
- labels: layer-4,l4-phase-3,supabase,cas,ledger
- parent: l4-pull-cas

### Description

## SOURCE OF TRUTH WARNING

- **Spec**: `docs/strategy/layers/layer-4-shared-visibility.md` — §1.5.2, §1.5.3, §1.5.4, §1.9.4 (PRIMARY)
- **TDD**: `L4-T-sqlite_supabase_sync-034..038` (PRIMARY)
- **Strategy**: Phase 3, P3-C (after CAS RPC)
- **Bootstrap**: §6.2 (migration 0007)

## Context

Storing `command_id` only on `operator_decision_heads` is insufficient — it remembers only the latest command. The `operator_decision_commands` ledger provides true idempotency: every CAS command writes a ledger row in the same transaction as the head update or conflict result. Retention default 30 days; replay after GC is treated as a new command (not idempotent any more).

## Files to Create/Modify

- **Modify:** `infra/supabase/migrations/0007_command_ledger.sql` — full table + check constraints.
- **Modify:** `infra/supabase/migrations/0005_sync_rpcs.sql` — extend `update_operator_decision` to consult/write ledger.
- **Add:** ledger GC job (daily cron entry seeded under bootstrap §11).

## Spec Requirements (verbatim)

**L4-FR-1.5.2 — `operator_decision_commands` is the durable idempotency ledger.**
- AC1: `command_id` is the primary key and survives later commands on the same process.
- AC2: Applied commands store `applied_revision` and response payload.
- AC3: Conflicted commands store `applied_revision = NULL`, `result = 'conflict'`, and response payload.
- AC4: The ledger is written in the same transaction as the decision-head update or conflict result.

**L4-FR-1.5.3 — CAS command replay returns the original result.**
- AC1: A repeated `command_id` returns the stored ledger result without re-evaluating current `operator_decision_heads`.
- AC2: A new command with matching base revision increments `remote_revision` exactly once.
- AC3: A new command with mismatched base revision writes a conflict ledger row and returns conflict.
- AC4: Lost-response retry after a later command landed returns the original stored result for the retried command.

**L4-FR-1.5.4 — CAS command ledger retention is bounded but durable for retries.**
- AC1: Default command ledger retention is 30 days.
- AC2: GC never removes entries newer than retention.
- AC3: GC removal does not alter current `operator_decision_heads` state.
- AC4: A replayed `command_id` after its ledger row has been GC'd is treated as a new command and evaluated against current `operator_decision_heads.remote_revision`.

**L4-FR-1.9.4 — CAS command ledger for idempotency (Constitutional).**

## Implementation Details

```sql
create table operator_decision_commands (
  command_id text primary key,
  process_id text not null,
  base_revision bigint not null,
  applied_revision bigint,
  disposition text not null,
  result text not null check (result in ('applied', 'conflict', 'superseded')),
  response_payload_json jsonb,
  created_at timestamptz not null default now()
);
```

GC: delete rows where `created_at < now() - interval '30 days'`.

## Tests to Pass

- **`L4-T-sqlite_supabase_sync-034`** (T2, `CasCommandHarness`): applied CAS command writes `operator_decision_commands` in same transaction as decision-head update with all fields. `L4-FR-1.5.2`.
- **`L4-T-sqlite_supabase_sync-035`** (T3, `CasCommandHarness`): command 1 applies, command 2 applies, command 1 retry returns command 1 stored ledger response without re-evaluating current decision head. `L4-FR-1.5.3`.
- **`L4-T-sqlite_supabase_sync-036`** (T2, `CasCommandHarness`): new command with mismatched base revision writes a conflict ledger row with null `applied_revision`, result `conflict`, response JSON in same transaction. `L4-FR-1.5.2`, `L4-FR-1.5.3`.
- **`L4-T-sqlite_supabase_sync-037`** (T2, `CasCommandHarness`): command ledger GC preserves entries newer than retention and never changes current `operator_decision_heads` state. `L4-FR-1.5.4`.
- **`L4-T-sqlite_supabase_sync-038`** (T2, `CasCommandHarness`): replaying command id after ledger row has been GC'd is evaluated as a new command and conflicts when its base revision is stale. `L4-FR-1.5.4`.

## Acceptance Criteria

- [ ] 0007 migration creates `operator_decision_commands` with PK, check constraints, all columns.
- [ ] `update_operator_decision` consults ledger first, returns stored result on hit.
- [ ] Ledger row written same-tx as head update or conflict result.
- [ ] GC job runs daily and respects 30-day retention.
- [ ] Post-GC replay treated as new command.
- [ ] L4-T-sqlite_supabase_sync-034..038 pass.

## Dependencies

- **Blocked by:** `l4-p3-cas-rpc`.
- **Blocks:** `l4-p3-pending-intent-reconciliation`, `l4-p3-review-gate`.
- **Phase:** 3.

---

## l4-p3-pending-intent-reconciliation: Pending Intent Coalescing and CAS Flush

- type: task
- priority: P0
- labels: layer-4,l4-phase-3,rust,cas,reconciliation
- parent: l4-pull-cas

### Description

## SOURCE OF TRUTH WARNING

- **Spec**: `docs/strategy/layers/layer-4-shared-visibility.md` — §1.4.4, §1.5.1 (PRIMARY)
- **TDD**: `L4-T-sqlite_supabase_sync-030, 031, 032` (PRIMARY)
- **Strategy**: Phase 3, P3-D (after pull-before-push contract)

## Context

After pull, reconcile `pending_operator_intents` queue. Compare each intent's `base_remote_revision` against pulled `operator_decision_heads.remote_revision`. Coalesce multiple unsuperseded intents for same process — winner = latest by `(queued_at DESC, id DESC)`; older intents marked `superseded_by_intent_id` pointing to winner before flush.

## Files to Create/Modify

- **Modify:** `crates/daemon/src/db/sync.rs` — `PendingIntentReconciler` extending the existing pending-intent capture/flush flow without editing the IPC handler.

## Spec Requirements (verbatim)

**L4-FR-1.4.4 — Pending operator intents reconcile against pulled CAS heads.**
- AC1: Reconciliation compares each queued intent's `base_remote_revision` with pulled `operator_decision_heads.remote_revision`.
- AC2: Matching revisions flush through CAS RPC.
- AC3: Revision mismatches mark the local queue row conflicted and surface a desktop conflict state.
- AC4: Multiple unsent intents for the same process coalesce latest-per-process before CAS flush.
- AC5: Coalescing winner is the most recent unsuperseded intent by `queued_at DESC, id DESC`, matching `migrations/0004_l2_pending_operator_intents.sql` and `crates/daemon/src/ipc/handlers/decisions.rs`; older unsuperseded intents marked with `superseded_by_intent_id` pointing to winner before flush.

**L4-FR-1.5.1 — Operator decisions mutate only through CAS.**
- AC5: Desktop decision IPC flow remains `capture_decision` -> write `pending_operator_intents` -> CAS flusher attempts promotion -> desktop re-fetches fresh state after `MutationAck.visible_after_seq`.

## Tests to Pass

- **`L4-T-sqlite_supabase_sync-030`** (T2): matching base-revision intent flushes through CAS RPC. (Co-owned with l4-p3-cas-rpc; lives here for reconciler responsibility.) `L4-FR-1.4.4`, `L4-FR-1.5.1`.
- **`L4-T-sqlite_supabase_sync-031`** (T2): two concurrent operator decisions — first applies, second conflicts locally. `L4-FR-1.4.4`, `L4-FR-1.5.1`.
- **`L4-T-sqlite_supabase_sync-032`** (T2): three unsuperseded intents for same process reconciled latest-by `queued_at DESC, id DESC`; older rows receive `superseded_by_intent_id`; only winner flushes through CAS. `L4-FR-1.4.4`.

## Acceptance Criteria

- [ ] `PendingIntentReconciler` runs after pull completes (per `l4-p3-pull-before-push-service-contract`).
- [ ] Coalescing rule matches L2 ordering (`queued_at DESC, id DESC`).
- [ ] Older intents are marked superseded BEFORE the CAS flush, not after.
- [ ] Conflicted intents update queue status (no silent skip).
- [ ] L4-T-sqlite_supabase_sync-030/031/032 pass.

## Dependencies

- **Blocked by:** `l4-p3-pull-before-push-service-contract`, `l4-p3-cas-rpc`, `l4-p3-command-ledger-retention`.
- **Blocks:** `l4-p3-decision-conflict-ipc`.
- **Phase:** 3.

---

## l4-p3-decision-conflict-ipc: decision.conflict IPC Event in decisions.rs

- type: task
- priority: P1
- labels: layer-4,l4-phase-3,rust,ipc,cas,chokepoint
- parent: l4-pull-cas

### Description

## SOURCE OF TRUTH WARNING

- **Spec**: `docs/strategy/layers/layer-4-shared-visibility.md` — §1.5.1 AC6+AC7 (PRIMARY)
- **TDD**: `L4-T-sqlite_supabase_sync-033` (PRIMARY)
- **Strategy**: Phase 3, P3-D (after pending-intent reconciliation; chokepoint for `decisions.rs`)

## Context

Single L4 edit home for `crates/daemon/src/ipc/handlers/decisions.rs`. Adds `decision.conflict` IPC event emission carrying `remote_revision`, `disposition`, and rejected intent payload — enough state for desktop conflict UI to refresh and retry intentionally. This bead also owns any neutral handler seam needed by later validation-feedback work; later Phase 4 beads must implement `validation/feedback.rs` without editing `decisions.rs`.

## Files to Create/Modify

- **Modify:** `crates/daemon/src/ipc/handlers/decisions.rs` — emit `decision.conflict` IPC event and preserve a single handler seam for downstream validation-feedback invocation without implementing Phase 4 feedback behavior here.

## Spec Requirements (verbatim)

**L4-FR-1.5.1 — Operator decisions mutate only through CAS (relevant ACs).**
- AC6: On CAS conflict, daemon emits a `decision.conflict` IPC event carrying current `remote_revision`, current `disposition`, and the rejected intent payload.
- AC7: The desktop conflict UI shows the remote disposition/revision, the local attempted disposition/note/reviewed hashes, and actions to refresh, discard local intent, or retry by issuing a new command against the current remote revision.

## Tests to Pass

- **`L4-T-sqlite_supabase_sync-033`** (T2, `CasCommandHarness`): CAS conflict response carries current remote revision, current disposition, rejected payload, drives `decision.conflict` IPC event payload. `L4-FR-1.5.1`.

## Acceptance Criteria

- [ ] `decisions.rs` emits `decision.conflict` event on conflict path.
- [ ] Event payload includes remote_revision, disposition, rejected intent payload.
- [ ] Existing pending-intent capture flow (L2) untouched.
- [ ] This remains the only L4 bead that edits `crates/daemon/src/ipc/handlers/decisions.rs`; Phase 4 feedback work consumes the seam without modifying the handler.
- [ ] L4-T-sqlite_supabase_sync-033 passes.

## Dependencies

- **Blocked by:** `l4-p3-pending-intent-reconciliation`.
- **Blocks:** `l4-p3-review-gate`, `l4-p4-override-feedback-loop`.
- **Phase:** 3.

---

## l4-p3-remote-config-staleness-ttl: Remote-Config Staleness Class TTLs

- type: task
- priority: P1
- labels: layer-4,l4-phase-3,rust,sync,degraded
- parent: l4-pull-cas

### Description

## SOURCE OF TRUTH WARNING

- **Spec**: `docs/strategy/layers/layer-4-shared-visibility.md` — §1.8.2 (PRIMARY)
- **TDD**: `L4-T-sqlite_supabase_sync-047` (PRIMARY)
- **Strategy**: Phase 3, P3-E

## Context

Not all remote-owned config is equally safe to use stale. Implements the staleness class table from spec §1.8.2: `shared_company_profile` (informational), `shared_source_configs` display/schedule (operational, 1h warn), `shared_source_configs.enabled` (safety-critical, 4h TTL), `operator_decision_heads` (operational; CAS protects correctness on push).

## Files to Create/Modify

- **Modify:** `crates/daemon/src/db/sync.rs` — `RemoteConfigStaleness::evaluate_class()` + degrade logic for safety-critical TTL.

## Spec Requirements (verbatim)

**L4-FR-1.8.2 — Remote-owned config staleness is classified by safety impact.**
- AC1: Stale `shared_company_profile` remains usable as informational bid-form input.
- AC2: Stale `shared_source_configs` display/schedule fields warn after 1 hour and use cached values.
- AC3: Stale `shared_source_configs.enabled` degrades the source after the safety-critical TTL, default 4 hours.
- AC4: Stale `operator_decision_heads` does not bypass CAS; pending local decisions may conflict when pull resumes.

## Tests to Pass

- **`L4-T-sqlite_supabase_sync-047`** (T2, `PullCursorHarness`): stale `shared_source_configs.enabled` degrades a source after the default 4-hour safety-critical TTL, while display/schedule warns after 1 hour and company profile remains informational. `L4-FR-1.8.2`, `L4-FR-2.3.2`.

## Acceptance Criteria

- [ ] Safety-critical TTL (default 4h) degrades source if pull cannot refresh.
- [ ] 1h operational warn applied to display/schedule fields.
- [ ] Company profile staleness does not block usage.
- [ ] L4-T-sqlite_supabase_sync-047 passes.

## Dependencies

- **Blocked by:** `l4-p3-pull-cursors`.
- **Blocks:** `l4-p3-review-gate`, `l4-p6-cross-authority-provenance-and-ttl`.
- **Phase:** 3.

---

## l4-p3-review-gate: Phase 3 Review Gate — Pull Engine and CAS Cumulative

- type: task
- priority: P0
- labels: layer-4,l4-phase-3,review-gate
- parent: l4-pull-cas

### Description

## SOURCE OF TRUTH WARNING

- **Spec**: §1.4, §1.5, §1.8.2 (PRIMARY)
- **TDD**: `L4-T-sqlite_supabase_sync-025..038, 047`; `L4-T-redaction_boundary-021, 022, 024` (PRIMARY)
- **Strategy**: Phase 3 review row

## Context

Phase 3 cumulative gate. Verifies pull trunk + CAS trunk + reconciliation operational. Closes Phase 3 to unblock Phase 6 startup wiring.

## Verification Commands

```bash
cargo fmt --check && cargo check --all-targets && cargo clippy --all-targets -- -D warnings
cargo test -p daemon --test sqlite_supabase_sync -- --nocapture
cargo test -p daemon --test redaction_boundary -- --nocapture --filter "L4-T-redaction_boundary-(021|022|024)"
```

## Tests to Pass (cumulative)

- `L4-T-sqlite_supabase_sync-025..038` (pull + CAS).
- `L4-T-sqlite_supabase_sync-047` (staleness TTL).
- `L4-T-redaction_boundary-021, 022, 024` (tombstone visibility, decision UPDATE blocked, admin maintenance).

## Acceptance Criteria

- [ ] All 8 Phase 3 task beads closed.
- [ ] All listed Phase 3 gate IDs pass.
- [ ] Phase 6 startup-wiring dependency cleared on pull-side.

## Dependencies

- **Blocked by:** all 8 Phase 3 task beads.
- **Blocks:** `l4-p6-main-startup-sync-wiring`.
- **Phase:** 3.

---

# Phase 4 Task Beads — Validation Enforcement

---

## l4-p4-provisional-storage-access: provisional_worker_outputs Access Layer

- type: task
- priority: P0
- labels: layer-4,l4-phase-4,rust,validation,enforcement
- parent: l4-validation-enforcement

### Description

## SOURCE OF TRUTH WARNING

- **Spec**: `docs/strategy/layers/layer-4-shared-visibility.md` — §1.3.9, §1.9.11, §3.4.2 (PRIMARY)
- **TDD**: `L4-T-sqlite_schema-005`, `L4-T-sqlite_supabase_sync-003` (PRIMARY)
- **Strategy**: Phase 4, P4-A root
- **Bootstrap**: §9.4 (`EnforcementHarness`)

## Context

Access layer over `provisional_worker_outputs` (created by P1 migration). Provides `insert_provisional`, `mark_approved`, `mark_rejected`, `lookup_by_worker_job_id`. The entry point used by runner staging in `l4-p4-runner-provisional-promotion`. Local-only — no outbox writes.

## Files to Create/Modify

- **Create:** `crates/daemon/src/validation/provisional.rs` — provisional access layer.

## Spec Requirements (verbatim)

**L4-FR-1.3.9 — Enforcement-active slices use a provisional promotion buffer.**
- AC1: Worker outputs for enforcement-active slices stored locally in `provisional_worker_outputs` before validator approval.
- AC2: Validator approval atomically promotes provisional results to push-owned tables, creates outbox entries, advances stage heads.
- AC3: Validator rejection preserves provisional artifacts and verdicts locally, emits an error manifest, and creates no push-owned rows or outbox entries.

**L4-FR-1.9.11 — Provisional promotion for enforcement slices (Constitutional).**

## Tests to Pass

- **`L4-T-sqlite_schema-005`** (T1): `provisional_worker_outputs` schema correctness (DDL test, owned by P1 migration but exercised by this access layer).
- **`L4-T-sqlite_supabase_sync-003`** (T2): an enforcement-active rejected provisional output creates no push-owned mutation and no outbox entry.

## Acceptance Criteria

- [ ] `validation/provisional.rs` exposes the four methods listed.
- [ ] All writes are local-only — no outbox writes from provisional path.
- [ ] L4-T-sqlite_supabase_sync-003 passes.

## Dependencies

- **Blocked by:** `l4-p1-review-gate`.
- **Blocks:** `l4-p4-policy-readiness-contract`, `l4-p4-executor-overlay-quarantine`, `l4-p4-runner-provisional-promotion`.
- **Phase:** 4.

---

## l4-p4-policy-readiness-contract: validation/policy.rs Reads enforcement_readiness

- type: task
- priority: P0
- labels: layer-4,l4-phase-4,rust,validation,policy,chokepoint
- parent: l4-validation-enforcement

### Description

## SOURCE OF TRUTH WARNING

- **Spec**: `docs/strategy/layers/layer-4-shared-visibility.md` — §3.4.3, §3.4.4, §3.4.6 (PRIMARY)
- **TDD**: `L4-T-sqlite_schema-006`, `L4-T-sqlite_supabase_sync-050, 057` (PRIMARY)
- **Strategy**: Phase 4, P4-B (chokepoint for `policy.rs`)

## Context

**Single edit home for `crates/daemon/src/validation/policy.rs`.** Extends existing L3 dry-run policy with per-slice readiness reads. Looks up `enforcement_readiness` row by deterministic `slice_id` (worker_skill × source × worker_behavior_version × validator_overlay_version) and decides enforcement vs dry-run mode. State persists across daemon restart.

## Files to Create/Modify

- **Modify:** `crates/daemon/src/validation/policy.rs` — extend with `read_enforcement_state(slice_id)`, `evaluate_slice_eligibility()`.

## Spec Requirements (verbatim)

**L4-FR-3.4.3 — Enforcement readiness is per slice, not global.**
- AC1: The enforcement slice key is worker skill × source × worker behavior version × validator behavior version.
- AC2: Enforcement can be enabled for one slice without enabling it for other slices.

**L4-FR-3.4.4 — A slice enters enforcement only after seven prerequisites pass.**
- AC1-AC7: overlay compatible-version + ≥30 adjudicated + ≥60 audited + Wilson lower-bound ≥70% + rule-of-three upper-bound ≤5% + dominant family coverage + no active demotion trigger.

**L4-FR-3.4.6 — Enforcement readiness state is durable SQLite state.**
- AC4: `validation/policy.rs` reads the persisted readiness row when deciding enforcement mode, and calibration/demotion jobs update the row so state survives daemon restart.

## Tests to Pass

- **`L4-T-sqlite_schema-006`** (T1): `enforcement_readiness` table shape (DDL).
- **`L4-T-sqlite_supabase_sync-050`** (T1, `CalibrationHarness`): slice with all 7 prerequisites becomes `eligible` per readiness recompute; `validation/policy.rs` reads `eligible` and selects enforcement mode. `L4-FR-3.4.3`, `L4-FR-3.4.4`, `L4-FR-3.6.14`.
- **`L4-T-sqlite_supabase_sync-057`** (T2): enforcement readiness decisions survive daemon restart and are read by `validation/policy.rs` instead of recomputed from volatile state. `L4-FR-3.4.6`.

## Acceptance Criteria

- [ ] `validation/policy.rs` no-op-extends L3 dry-run path with readiness lookup.
- [ ] `slice_id` derived deterministically from worker_skill + source + worker_behavior + validator_overlay version tuple.
- [ ] Daemon restart preserves enforcement decisions (durable SQLite state).
- [ ] L4-T-sqlite_supabase_sync-050/057 pass.

## Dependencies

- **Blocked by:** `l4-p4-provisional-storage-access`.
- **Blocks:** `l4-p4-error-manifest-classification`, `l4-p4-executor-overlay-quarantine`, `l4-p4-calibration-math-readiness`.
- **Phase:** 4.

---

## l4-p4-error-manifest-classification: AGENT_SUSPECT_OUTPUT Error Type

- type: task
- priority: P1
- labels: layer-4,l4-phase-4,rust,errors,chokepoint
- parent: l4-validation-enforcement

### Description

## SOURCE OF TRUTH WARNING

- **Spec**: `docs/strategy/layers/layer-4-shared-visibility.md` — §3.4.1, §3.4.2 (PRIMARY)
- **TDD**: `L4-T-sqlite_supabase_sync-023, 080` (PRIMARY)
- **Strategy**: Phase 4, P4-C (sequential before executor)

## Context

Add `AGENT_SUSPECT_OUTPUT` error type to `crates/daemon/src/errors/manifest.rs` for enforcement-mode rejections. Worker outputs that the validator rejects in enforcement mode get classified as `AGENT_SUSPECT_OUTPUT` and an operator-alert error manifest is created. Validator-crash path uses existing `VALIDATION_SKIPPED_VALIDATOR_ERROR` (fail-open).

## Files to Create/Modify

- **Modify:** `crates/daemon/src/errors/manifest.rs` — add `AGENT_SUSPECT_OUTPUT` enum variant + manifest renderer.

## Spec Requirements (verbatim)

**L4-FR-3.4.1 — Enforcement mode turns validator rejection into quarantine.**
- AC1: Rejected enforcement-active worker output is classified as `AGENT_SUSPECT_OUTPUT`.
- AC5: An error manifest with operator alert is created for each enforcement rejection.

**L4-FR-3.4.2 — Enforcement uses the L4 provisional promotion buffer.**
- AC4: Fail-open validator-crash behavior remains the L3 fail-open path.

## Tests to Pass

- **`L4-T-sqlite_supabase_sync-023`** (T3, `EnforcementHarness` + `RedactionSentinelFixture`): enforcement-active worker output with validator rejection emits `AGENT_SUSPECT_OUTPUT` error manifest with operator alert. `L4-FR-1.3.9`, `L4-FR-3.4.1`, `L4-FR-3.4.2`.
- **`L4-T-sqlite_supabase_sync-080`** (T3, `EnforcementHarness`): validator crash/timeout/invalid payload follows L3 fail-open semantics with `VALIDATION_SKIPPED_VALIDATOR_ERROR`. `L4-FR-3.4.2`.

## Acceptance Criteria

- [ ] `AGENT_SUSPECT_OUTPUT` exists in `errors/manifest.rs`.
- [ ] Manifest renderer produces operator-alert markdown.
- [ ] Validator-crash path NOT classified as `AGENT_SUSPECT_OUTPUT` (uses L3 fail-open).
- [ ] L4-T-sqlite_supabase_sync-023/080 pass.

## Dependencies

- **Blocked by:** `l4-p4-policy-readiness-contract`.
- **Blocks:** `l4-p4-executor-overlay-quarantine`.
- **Phase:** 4.

---

## l4-p4-executor-overlay-quarantine: validation/executor.rs Overlay Loading and Quarantine

- type: task
- priority: P0
- labels: layer-4,l4-phase-4,rust,validation,executor,chokepoint
- parent: l4-validation-enforcement

### Description

## SOURCE OF TRUTH WARNING

- **Spec**: `docs/strategy/layers/layer-4-shared-visibility.md` — §3.4.1, §3.4.2, §3.5 (PRIMARY)
- **TDD**: `L4-T-sqlite_supabase_sync-022..024, 080` (PRIMARY)
- **Strategy**: Phase 4, P4-C (chokepoint for `executor.rs`)

## Context

**Single edit home for `crates/daemon/src/validation/executor.rs`.** Extends existing L3 executor with overlay loading (loads base validator + selected overlay per skill metadata), provisional staging write path, dry-run versus enforcement-active branch logic, and quarantine path (rejected enforcement-active output stays provisional, emits `AGENT_SUSPECT_OUTPUT`, creates no push-owned rows, and creates no outbox). Approval atomically promotes through runner.

## Files to Create/Modify

- **Modify:** `crates/daemon/src/validation/executor.rs` — overlay loading + verdict handling for enforcement.

## Spec Requirements (verbatim)

**L4-FR-3.4.1 — Enforcement mode turns validator rejection into quarantine.**
- AC2: Rejected output does not advance stage heads.
- AC3: Rejected output does not create outbox entries or remote projection rows.
- AC4: Pipeline rollups exclude quarantined results.

**L4-FR-3.4.2 — Enforcement uses the L4 provisional promotion buffer.**
- AC1: Validation executor extends existing L3 executor with provisional-result staging instead of replacing.
- AC2: Approved provisional results promoted atomically with outbox creation and stage-head advancement.
- AC3: Rejected provisional results stay local and auditable.

**L4-FR-3.5.1 through L4-FR-3.5.4 — Domain-Specific Validator Overlays (4 skills).**
- AC3: Existing L3 Validation Executor loads the base validator plus the selected overlay; L4 extends executor and does not replace it.

## Tests to Pass

- **`L4-T-sqlite_supabase_sync-022`** (T3): enforcement-active worker output with validator approval atomically updates provisional status, promotes push-owned rows, creates outbox entries, advances stage heads. `L4-FR-1.3.9`, `L4-FR-3.4.2`, `L4-FR-1.9.11`.
- **`L4-T-sqlite_supabase_sync-023`** (T3): enforcement-active worker output with validator rejection leaves provisional row local and executor-side behavior emits `AGENT_SUSPECT_OUTPUT`; manifest classification mapping remains in `l4-p4-error-manifest-classification`. `L4-FR-1.3.9`, `L4-FR-3.4.1`, `L4-FR-3.4.2`.
- **`L4-T-sqlite_supabase_sync-024`** (T3): dry-run or non-enforcement slice with validator rejection follows L3 direct-write behavior; push-owned rows + outbox entries created, verdict logged, stage heads advance. `L4-FR-1.3.9`, `L4-FR-3.4.2`.
- **`L4-T-sqlite_supabase_sync-080`** (T3): validator crash/timeout/invalid payload — fail-open with `VALIDATION_SKIPPED_VALIDATOR_ERROR`. `L4-FR-3.4.2`.

## Acceptance Criteria

- [ ] `executor.rs` extends, does not replace, L3 logic.
- [ ] Overlay selection driven by skill metadata `validator_overlay_compatible_versions`.
- [ ] Provisional staging write path is used before validator verdict handling for enforcement-capable slices.
- [ ] Enforcement-active rejection: provisional unchanged → still local, no outbox, `AGENT_SUSPECT_OUTPUT` emitted.
- [ ] Dry-run or non-enforcement rejection follows L3 direct-write behavior and records the verdict without quarantine.
- [ ] Approval: atomic promote via runner promotion path.
- [ ] L4-T-sqlite_supabase_sync-022/023/024/080 pass.

## Dependencies

- **Blocked by:** `l4-p4-policy-readiness-contract`, `l4-p4-provisional-storage-access`, `l4-p4-error-manifest-classification`.
- **Blocks:** `l4-p4-runner-provisional-promotion`.
- **Phase:** 4.

---

## l4-p4-runner-provisional-promotion: jobs/runner.rs Provisional Promotion Convergence

- type: task
- priority: P0
- labels: layer-4,l4-phase-4,rust,runner,chokepoint
- parent: l4-validation-enforcement

### Description

## SOURCE OF TRUTH WARNING

- **Spec**: `docs/strategy/layers/layer-4-shared-visibility.md` — §1.3.9, §3.4.2 (PRIMARY)
- **TDD**: `L4-T-sqlite_supabase_sync-022, 023, 079` (PRIMARY)
- **Strategy**: Phase 4, P4-C (chokepoint for `runner.rs` — single L4 edit)

## Context

**Single L4 edit home for `crates/daemon/src/jobs/runner.rs`.** Provisional promotion converges through the runner boundary: enforcement-active worker job emits result into `provisional_worker_outputs`, enqueues/links validator job, on approval atomically promotes (data row + outbox entry + stage head advance) using same `worker_job_id`. Phase 6 capstone tests runner behavior but does not edit this file.

## Files to Create/Modify

- **Modify:** `crates/daemon/src/jobs/runner.rs` — provisional staging branch + promotion path.

## Spec Requirements (verbatim)

**L4-FR-1.3.9 — Enforcement-active slices use a provisional promotion buffer.**
- AC2: Validator approval atomically promotes provisional results to push-owned tables, creates outbox entries, and advances stage heads.

**L4-FR-3.4.2 — Enforcement uses the L4 provisional promotion buffer.**
- AC5: An operator override of a rejected provisional result records `validation_verdicts.override_*`, then promotes the same provisional payload through the normal approved path in one SQLite transaction.
- AC6: Override promotion changes `provisional_worker_outputs.validation_status` to `approved`, creates push-owned rows/outbox entries, advances stage heads exactly once using the original `worker_job_id`.
- AC7: Override promotion emits audit/provenance fields so downstream rollups can distinguish validator-approved from operator-overridden.
- AC8: Replaying the same override command is idempotent; cannot create duplicate outbox rows.

## Tests to Pass

- **`L4-T-sqlite_supabase_sync-022`** (T3): approved enforcement-active output atomically promoted through the runner boundary; executor verdict handling remains in `l4-p4-executor-overlay-quarantine`.
- **`L4-T-sqlite_supabase_sync-023`** (T3): rejected enforcement-active output stays local with no outbox when observed through the runner boundary; executor quarantine handling remains in `l4-p4-executor-overlay-quarantine`.
- **`L4-T-sqlite_supabase_sync-079`** (T3, `EnforcementHarness`): `jobs/runner.rs` emits enforcement-active worker job into provisional storage, enqueues/links validator job through existing runner boundary, promotes only after approved verdict. `L4-FR-1.3.9`, `L4-FR-3.4.2`.

## Acceptance Criteria

- [ ] Single L4 edit to `runner.rs` (chokepoint discipline preserved).
- [ ] Enforcement-active worker → provisional staging path.
- [ ] Validator job linked via `parent_job_id` to worker job.
- [ ] Promotion atomic (single SQLite tx: provisional update + push-owned insert + outbox insert + stage head advance).
- [ ] Override path is idempotent.
- [ ] L4-T-sqlite_supabase_sync-022/023/079 pass.

## Dependencies

- **Blocked by:** `l4-p4-provisional-storage-access`, `l4-p4-executor-overlay-quarantine`.
- **Blocks:** `l4-p4-override-feedback-loop`, `l4-p4-review-gate`.
- **Phase:** 4.

---

## l4-p4-calibration-math-readiness: Wilson Lower-Bound and Rule-of-Three Math

- type: task
- priority: P0
- labels: layer-4,l4-phase-4,rust,calibration,math
- parent: l4-validation-enforcement

### Description

## SOURCE OF TRUTH WARNING

- **Spec**: `docs/strategy/layers/layer-4-shared-visibility.md` — §3.4.4, §3.4.5, §3.4.6, §3.6.14 (PRIMARY)
- **TDD**: `L4-T-sqlite_supabase_sync-050..057, 075` (PRIMARY)
- **Strategy**: Phase 4, P4-D
- **Bootstrap**: §9.4

## Context

Pure functions in `crates/daemon/src/validation/calibration.rs`: Wilson 95% lower bound, rule-of-three false-pass upper bound, demotion-threshold checks, event-driven readiness recompute after every `validation_verdicts` insert/update (synchronous within same SQLite tx).

## Files to Create/Modify

- **Create:** `crates/daemon/src/validation/calibration.rs` — Wilson, rule-of-three, demotion eval, readiness recompute.

## Spec Requirements (verbatim)

**L4-FR-3.4.4 — A slice enters enforcement only after seven prerequisites pass.**
- AC4: Wilson-bound reject precision lower bound is at least 70%.
- AC5: Rule-of-three false-pass upper bound is at most 5%.

**L4-FR-3.4.5 — Demotion triggers immediately return a slice to dry-run.**
- AC1-AC4 (4 demotion triggers + thresholds): override spike (rolling 7-day, ≥10 reviewed rejections, ≥1500 bps or 2× baseline); challenge regression (any critical mutated-bad approved or pass rate <95%); anomaly-conditioned validator error (≥20 samples, >5% or 2× baseline); compatible-range exit.
- AC5: Re-promotion blocked until all calibration thresholds pass again.

**L4-FR-3.6.14 — Calibration targets are minimum gates before enforcement.**
- AC6: Wilson-bound and rule-of-three calculations live in pure functions in `crates/daemon/src/validation/calibration.rs` and are asserted by tests with boundary cases around 30/60 sample counts.

## Tests to Pass

- **`L4-T-sqlite_supabase_sync-050`** (T1): slice with all 7 prerequisites becomes eligible. `L4-FR-3.4.3`, `L4-FR-3.4.4`, `L4-FR-3.6.14`.
- **`L4-T-sqlite_supabase_sync-051`** (T1): missing dominant family coverage keeps slice in dry_run, records exact missing family set. `L4-FR-3.4.4`, `L4-FR-3.6.3`.
- **`L4-T-sqlite_supabase_sync-052`** (T1): Wilson lower bound below 7000 bps blocks enforcement. `L4-FR-3.4.4`, `L4-FR-3.6.14`.
- **`L4-T-sqlite_supabase_sync-053`** (T1): event-driven readiness recompute runs after every `validation_verdicts` insert/update; override-spike threshold flips slice to demoted synchronously in same SQLite tx. `L4-FR-3.4.5`, `L4-FR-3.4.6`.
- **`L4-T-sqlite_supabase_sync-054`** (T1): approving any critical mutated-bad assertion or pass rate <95% immediately demotes. `L4-FR-3.4.5`, `L4-FR-3.6.3`.
- **`L4-T-sqlite_supabase_sync-055`** (T1): anomaly-conditioned validator error rate over threshold demotes. `L4-FR-3.4.5`, `L4-FR-3.4.6`.
- **`L4-T-sqlite_supabase_sync-056`** (T1): worker behavior version outside overlay compatible range disables enforcement. `L4-FR-3.4.5`.
- **`L4-T-sqlite_supabase_sync-057`** (T2): enforcement readiness decisions survive daemon restart. `L4-FR-3.4.6`.
- **`L4-T-sqlite_supabase_sync-075`** (T1): Wilson lower-bound and rule-of-three pure functions assert known boundary cases: 26/30 → ~7030-7040 bps passes; 25/30 fails; 0/60 → 500 bps passes; 0/59 fails. `L4-FR-3.6.14`.

## Acceptance Criteria

- [ ] `validation/calibration.rs` exposes pure `wilson_lower_bound_bps(correct, total)` and `rule_of_three_false_pass_upper_bound_bps(misses, audited)`.
- [ ] Boundary tests pass at exact bps values listed.
- [ ] Event-driven readiness hook runs same-tx with verdict insert/update.
- [ ] All 4 demotion triggers implemented.
- [ ] L4-T-sqlite_supabase_sync-050..057 + 075 pass.

## Dependencies

- **Blocked by:** `l4-p4-policy-readiness-contract`.
- **Blocks:** `l4-p4-calibration-corpora-and-cases`, `l4-p4-override-feedback-loop`, `l4-p4-review-gate`.
- **Phase:** 4.

---

## l4-p4-calibration-corpora-and-cases: Calibration Corpora, Mutation Library, Pairwise

- type: task
- priority: P1
- labels: layer-4,l4-phase-4,calibration,fixtures,tests
- parent: l4-validation-enforcement

### Description

## SOURCE OF TRUTH WARNING

- **Spec**: `docs/strategy/layers/layer-4-shared-visibility.md` — §3.6.1..§3.6.13, §3.6.15 (PRIMARY)
- **TDD**: `L4-T-sqlite_supabase_sync-062..074, 076` (PRIMARY)
- **Strategy**: Phase 4, P4-D (after calibration math)
- **Bootstrap**: §10, §11 (cross-model audit cron)

## Context

Per skill family: 25-30 clean real, 15-20 weird-valid, 30-50 mutated bad, ≥1 pairwise per dominant failure family. Mutations: 9 families (`L4-FR-3.6.4..3.6.12`). Each case has `metadata.json` with `case_id`, `slice_id`, `corpus_kind`, `expected_verdict`, `ground_truth_label`, `mutation_family`, `pair_id`, `model_family`, `source_artifact_path`. Cross-model audit sampling daily cron via `crates/daemon/src/scheduler/cron.rs`.

## Files to Create/Modify

- **Create:** `tests/fixtures/calibration/{slice_id}/{clean,weird,mutated,pairwise}/` — corpora directory tree with seed cases.
- **Modify:** `crates/daemon/src/validation/calibration.rs` — mutation library, corpus seeding helpers.
- **Modify:** `crates/daemon/src/scheduler/cron.rs` — cross-model audit cron entry seeding.

## Spec Requirements (verbatim)

**L4-FR-3.6.1 — Clean real corpus establishes healthy baseline.** AC1: each skill family has 25-30 clean real runs.
**L4-FR-3.6.2 — Weird-but-valid corpus protects valid edge cases.** AC1: each skill family has 15-20 weird-valid runs.
**L4-FR-3.6.3 — Mutated bad corpus exercises realistic failure families.** AC1: each skill family has 30-50 mutated bad runs.
**L4-FR-3.6.4 through L4-FR-3.6.12 — 9 mutation families** (verbatim from spec table):
- 3.6.4 Wrong source/domain in URL chain
- 3.6.5 Wrong page kind (search vs category vs detail)
- 3.6.6 Locale/currency swap (claims GBP, evidence EUR)
- 3.6.7 Pack/unit distortion (per-unit vs per-box vs per-pallet)
- 3.6.8 Accessory-vs-primary confusion
- 3.6.9 Selective warning omission (says 2, daemon shows 5)
- 3.6.10 Manifest/result count mismatch (claims 12, contract has 8)
- 3.6.11 Fabricated evidence references
- 3.6.12 Cross-segment duplicates/gaps (continuation chains)

**L4-FR-3.6.13 — Pairwise challenge sets measure relative trust.**
- AC1-AC4: pair_id-shared clean+corrupted siblings; failures feed readiness/demotion.

**L4-FR-3.6.15 — Cross-model audit sampling detects correlated blindness.**
- AC1-AC4: periodic alternate model family sample; daily cadence ≥ once per 30 days for active slices with new traffic.

## Tests to Pass

- **`L4-T-sqlite_supabase_sync-062`** (T3): each skill family has 25-30 clean real cases. `L4-FR-3.6.1`.
- **`L4-T-sqlite_supabase_sync-063`** (T3): each family has 15-20 weird-valid cases. `L4-FR-3.6.2`.
- **`L4-T-sqlite_supabase_sync-064`** (T3): each family has 30-50 mutated cases. `L4-FR-3.6.3`.
- **`L4-T-sqlite_supabase_sync-065..073`** (T1, one per mutation family): wrong-domain, page-kind, currency/locale, unit-basis, product-identity, warning-omission, count-mismatch, fabricated-evidence, continuation-integrity. Validator must emit `rejected` with the matching issue category. `L4-FR-3.6.4..3.6.12`.
- **`L4-T-sqlite_supabase_sync-074`** (T1): pairwise siblings share `pair_id`; corrupted ranks less trustworthy. `L4-FR-3.6.13`.
- **`L4-T-sqlite_supabase_sync-076`** (T3, `CalibrationCronHarness`): daily `validation-cross-model-audit-sample` cron selects active slices with new traffic, dispatches alternate-model validator job, records model_family + disagreement. `L4-FR-3.6.15`.

## Acceptance Criteria

- [ ] Corpora directories exist with required min counts per skill family.
- [ ] All 9 mutation families have golden assertions.
- [ ] Pairwise pairs share `pair_id` in `validation_calibration_cases`.
- [ ] Cross-model audit cron seeded.
- [ ] L4-T-sqlite_supabase_sync-062..074, 076 pass.

## Dependencies

- **Blocked by:** `l4-p4-calibration-math-readiness`.
- **Blocks:** `l4-p4-overlay-procurement-ingestion`, `l4-p4-overlay-supplier-enrichment`, `l4-p4-overlay-document-acquisition`, `l4-p4-overlay-product-extraction`, `l4-p4-review-gate`.
- **Phase:** 4.

---

## l4-p4-override-feedback-loop: Override Feedback Loop in feedback.rs

- type: task
- priority: P1
- labels: layer-4,l4-phase-4,rust,calibration,feedback
- parent: l4-validation-enforcement

### Description

## SOURCE OF TRUTH WARNING

- **Spec**: `docs/strategy/layers/layer-4-shared-visibility.md` — §3.6.16 (PRIMARY)
- **TDD**: `L4-T-sqlite_schema-009`, `L4-T-sqlite_supabase_sync-077` (PRIMARY)
- **Strategy**: Phase 4, P4-E (after runner promotion)
- **Bootstrap**: §9.5

## Context

Every operator override of a rejected provisional result becomes: (1) labeled calibration case, (2) incident-cluster member, (3) candidate mutation template. Override is a biased metric — used as demotion signal, incident source, mutation template source — but never as sole precision estimator. `crates/daemon/src/ipc/handlers/decisions.rs` is owned exclusively by `l4-p3-decision-conflict-ipc`; this bead implements the feedback fan-out behind that existing handler seam and must not edit the handler.

## Files to Create/Modify

- **Create:** `crates/daemon/src/validation/feedback.rs` — override fan-out logic.

## Spec Requirements (verbatim)

**L4-FR-3.6.16 — Operator overrides feed calibration without becoming the sole estimator.**
- AC1: Every override creates a labeled calibration case.
- AC2: Every override joins an incident cluster.
- AC3: Every override is evaluated as a candidate mutation template.
- AC4: Override rate can trigger demotion but cannot be the only precision estimator.
- AC5: Override ingestion is automatic in the daemon path that records `validation_verdicts.override_*`; it inserts/updates `validation_calibration_cases` and `validation_incident_clusters` in the same SQLite transaction as the override audit update.
- AC6: Candidate mutation templates stored as JSON on the incident cluster.

## Tests to Pass

- **`L4-T-sqlite_schema-009`** (T1): `validation_incident_clusters` schema is available from the Phase 1 DDL owner before this feedback path writes incident-cluster rows.
- **`L4-T-sqlite_supabase_sync-077`** (T2, `OverrideFeedbackHarness`): operator override updates `validation_verdicts.override_*`, changes `provisional_worker_outputs.validation_status` to `approved`, promotes original payload through normal outbox path exactly once, writes/updates calibration case and incident cluster rows in same SQLite tx, stores candidate mutation template JSON. `L4-FR-3.4.2`, `L4-FR-3.6.16`.

## Acceptance Criteria

- [ ] `validation/feedback.rs` exists with `ingest_override(verdict_id)`.
- [ ] No edit is made to `crates/daemon/src/ipc/handlers/decisions.rs`; the handler edit remains solely owned by `l4-p3-decision-conflict-ipc`.
- [ ] Single SQLite tx covers: override audit + provisional → approved + push promotion + calibration case + incident cluster + mutation template.
- [ ] Idempotent on replay (no duplicate outbox).
- [ ] L4-T-sqlite_supabase_sync-077 passes.

## Dependencies

- **Blocked by:** `l4-p3-decision-conflict-ipc`, `l4-p4-runner-provisional-promotion`, `l4-p4-calibration-math-readiness`.
- **Blocks:** `l4-p4-review-gate`.
- **Phase:** 4.

---

## l4-p4-overlay-procurement-ingestion: Procurement Ingestion Validator Overlay

- type: task
- priority: P1
- labels: layer-4,l4-phase-4,skill,validation,overlay
- parent: l4-validation-enforcement

### Description

## SOURCE OF TRUTH WARNING

- **Spec**: `docs/strategy/layers/layer-4-shared-visibility.md` — §3.5.1, §3.5 (PRIMARY)
- **TDD**: `L4-T-sqlite_supabase_sync-058` (PRIMARY)
- **Strategy**: Phase 4, P4-F parallel
- **Bootstrap**: §7

## Context

Overlay skill at `skills/validation/overlay-procurement-ingestion/`. Six required files: `SKILL.md`, `result.schema.json`, `checkpoint.schema.json`, `canary.inputs.jsonl`, `golden.assertions.yaml`, `validator-rubric.yaml`, plus `references/`. Domain checks: notice completeness, OCDS field presence, source locale, notice-type consistency, expiry plausibility, CPV validity.

## Files to Create/Modify

- **Create:** `skills/validation/overlay-procurement-ingestion/SKILL.md`
- **Create:** `skills/validation/overlay-procurement-ingestion/result.schema.json`
- **Create:** `skills/validation/overlay-procurement-ingestion/checkpoint.schema.json`
- **Create:** `skills/validation/overlay-procurement-ingestion/canary.inputs.jsonl`
- **Create:** `skills/validation/overlay-procurement-ingestion/golden.assertions.yaml`
- **Create:** `skills/validation/overlay-procurement-ingestion/validator-rubric.yaml`
- **Create:** `skills/validation/overlay-procurement-ingestion/references/`

## Spec Requirements (verbatim)

**L4-FR-3.5.1 — Procurement ingestion overlay.** Domain checks: notice completeness, OCDS field presence, correct source locale, notice-type consistency, expiry date plausibility, CPV code validity.

**L4-FR-3.5.1 through L4-FR-3.5.4 — All 4 overlays.**
- AC1: Each overlay directory contains the 6 files + `references/`, following `skills/validation/base/`.
- AC2: Each overlay declares compatible worker behavior versions.
- AC4: Canary tests prove each overlay emits verdicts and applies the listed domain checks.

## Implementation Details (from bootstrap §7)

`SKILL.md` frontmatter:
```yaml
skill_id: validation/overlay-procurement-ingestion
skill_version: 2026.05.06-1
interaction_profile: headless_contract
result_schema_version: 1.0
checkpoint_schema_version: 1.0
validator_overlay_version: 2026.05.06-1
validator_overlay_compatible_versions:
  worker_behavior_versions:
    - procurement-ingestion/*@2026.05.06-1+1.0
model_profile: structured_extract_low_cost
agent_cli: claude
side_effect_class: read_only
promotion_stage: candidate
```

`canary.inputs.jsonl` MUST include: 1 clean-real approval, 1 weird-valid no-rejection, 1 mutated-bad rejection, 1 pairwise pair (if overlay owns any mutation family).

## Tests to Pass

- **`L4-T-sqlite_supabase_sync-058`** (T3, `OverlaySkillFixture`): procurement-ingestion overlay has the six-file skill contract, compatible versions, canary verdicts, and golden assertions for notice completeness, OCDS fields, locale, notice type, expiry, CPV validity. `L4-FR-3.5.1`.

## Acceptance Criteria

- [ ] All 6 required files + `references/` directory exist.
- [ ] `SKILL.md` frontmatter matches bootstrap template.
- [ ] `canary.inputs.jsonl` has the 4 required cases.
- [ ] `golden.assertions.yaml` covers all 6 domain checks.
- [ ] L4-T-sqlite_supabase_sync-058 passes.

## Dependencies

- **Blocked by:** `l4-p4-calibration-corpora-and-cases` (for mutation families) — NOTE: strategy lists this as P4-F parallel after base validator template; can run in parallel with other overlays.
- **Blocks:** `l4-p4-review-gate`.
- **Phase:** 4.

---

## l4-p4-overlay-supplier-enrichment: Supplier Enrichment Validator Overlay

- type: task
- priority: P1
- labels: layer-4,l4-phase-4,skill,validation,overlay
- parent: l4-validation-enforcement

### Description

## SOURCE OF TRUTH WARNING

- **Spec**: `docs/strategy/layers/layer-4-shared-visibility.md` — §3.5.2 (PRIMARY)
- **TDD**: `L4-T-sqlite_supabase_sync-059` (PRIMARY)
- **Strategy**: Phase 4, P4-F parallel

## Context

Overlay at `skills/validation/overlay-supplier-enrichment/`. Domain checks: price plausibility (too high/low for category), currency correctness, pack basis consistency, seller diversity, listing freshness, regulated-goods compliance evidence completeness.

## Files to Create/Modify

- **Create:** all 6 files + `references/` under `skills/validation/overlay-supplier-enrichment/`.

## Spec Requirements (verbatim)

**L4-FR-3.5.2 — Supplier enrichment overlay.** Domain checks listed above.
**L4-FR-3.5.2 — Per-overlay AC1-AC4** as in `l4-p4-overlay-procurement-ingestion`.

## Tests to Pass

- **`L4-T-sqlite_supabase_sync-059`** (T3, `OverlaySkillFixture`): supplier-enrichment overlay has the six-file contract, compatible versions, canary verdicts, golden assertions for price plausibility, currency, pack basis, seller diversity, freshness, regulated-goods compliance evidence. `L4-FR-3.5.2`.

## Acceptance Criteria

- [ ] All 6 files + `references/` exist.
- [ ] Frontmatter has `skill_id: validation/overlay-supplier-enrichment` and worker behavior version compatibility.
- [ ] Canary inputs cover the 6 domain checks.
- [ ] L4-T-sqlite_supabase_sync-059 passes.

## Dependencies

- **Blocked by:** `l4-p4-calibration-corpora-and-cases`.
- **Blocks:** `l4-p4-review-gate`.
- **Phase:** 4.

---

## l4-p4-overlay-document-acquisition: Document Acquisition Validator Overlay

- type: task
- priority: P1
- labels: layer-4,l4-phase-4,skill,validation,overlay
- parent: l4-validation-enforcement

### Description

## SOURCE OF TRUTH WARNING

- **Spec**: `docs/strategy/layers/layer-4-shared-visibility.md` — §3.5.3 (PRIMARY)
- **TDD**: `L4-T-sqlite_supabase_sync-060` (PRIMARY)
- **Strategy**: Phase 4, P4-F parallel

## Context

Overlay at `skills/validation/overlay-document-acquisition/`. Domain checks: document type identification correctness, download completeness, content-type verification, file size plausibility, document set hash consistency.

## Files to Create/Modify

- **Create:** all 6 files + `references/` under `skills/validation/overlay-document-acquisition/`.

## Spec Requirements (verbatim)

**L4-FR-3.5.3 — Document acquisition overlay.** Domain checks listed above.
**L4-FR-3.5.3 — Per-overlay AC1-AC4.**

## Tests to Pass

- **`L4-T-sqlite_supabase_sync-060`** (T3, `OverlaySkillFixture`): document-acquisition overlay has the six-file contract, compatible versions, canary verdicts, golden assertions for document type, download completeness, content type, file size, set hash consistency. `L4-FR-3.5.3`.

## Acceptance Criteria

- [ ] All 6 files + `references/` exist.
- [ ] Frontmatter has `skill_id: validation/overlay-document-acquisition`.
- [ ] Canary inputs cover the 5 domain checks.
- [ ] L4-T-sqlite_supabase_sync-060 passes.

## Dependencies

- **Blocked by:** `l4-p4-calibration-corpora-and-cases`.
- **Blocks:** `l4-p4-review-gate`.
- **Phase:** 4.

---

## l4-p4-overlay-product-extraction: Product Extraction Validator Overlay

- type: task
- priority: P1
- labels: layer-4,l4-phase-4,skill,validation,overlay
- parent: l4-validation-enforcement

### Description

## SOURCE OF TRUTH WARNING

- **Spec**: `docs/strategy/layers/layer-4-shared-visibility.md` — §3.5.4 (PRIMARY)
- **TDD**: `L4-T-sqlite_supabase_sync-061` (PRIMARY)
- **Strategy**: Phase 4, P4-F parallel

## Context

Overlay at `skills/validation/overlay-product-extraction/`. Domain checks: candidate type plausibility for CPV family, archetype assignment consistency, slot completeness for archetype, quantity/unit plausibility, bundle decomposition correctness.

## Files to Create/Modify

- **Create:** all 6 files + `references/` under `skills/validation/overlay-product-extraction/`.

## Spec Requirements (verbatim)

**L4-FR-3.5.4 — Product extraction overlay.** Domain checks listed above.
**L4-FR-3.5.4 — Per-overlay AC1-AC4.**

## Tests to Pass

- **`L4-T-sqlite_supabase_sync-061`** (T3, `OverlaySkillFixture`): product-extraction overlay has the six-file contract, compatible versions, canary verdicts, golden assertions for CPV family, archetype, slots, quantity/unit, bundle decomposition. `L4-FR-3.5.4`.

## Acceptance Criteria

- [ ] All 6 files + `references/` exist.
- [ ] Frontmatter has `skill_id: validation/overlay-product-extraction`.
- [ ] Canary inputs cover the 5 domain checks.
- [ ] L4-T-sqlite_supabase_sync-061 passes.

## Dependencies

- **Blocked by:** `l4-p4-calibration-corpora-and-cases`.
- **Blocks:** `l4-p4-review-gate`.
- **Phase:** 4.

---

## l4-p4-review-gate: Phase 4 Review Gate — Validation Enforcement Cumulative

- type: task
- priority: P0
- labels: layer-4,l4-phase-4,review-gate
- parent: l4-validation-enforcement

### Description

## SOURCE OF TRUTH WARNING

- **Spec**: §1.3.9, §3.4, §3.5, §3.6 (PRIMARY)
- **TDD**: `L4-T-sqlite_schema-005..009, 017`; `L4-T-sqlite_supabase_sync-003, 022..024, 050..080` (PRIMARY)
- **Strategy**: Phase 4 review row

## Context

Phase 4 cumulative gate. Provisional buffer + executor overlay + runner promotion + calibration math + corpora + override feedback + 4 overlays operational.

## Verification Commands

```bash
cargo fmt --check && cargo check --all-targets && cargo clippy --all-targets -- -D warnings
cargo test -p daemon --test sqlite_schema -- --nocapture
cargo test -p daemon --test sqlite_supabase_sync -- --nocapture
```

## Tests to Pass (cumulative)

- `L4-T-sqlite_schema-005..009, 017`.
- `L4-T-sqlite_supabase_sync-003, 022, 023, 024, 050..080`.
- Overlay canaries `058..061`.
- Mutation families `065..073`, pairwise `074`, math `075`, cross-model `076`, override `077`.

## Acceptance Criteria

- [ ] All 12 Phase 4 task beads closed.
- [ ] Listed Phase 4 gate IDs pass.
- [ ] No L3 dry-run path replaced (extension only).

## Dependencies

- **Blocked by:** all 12 Phase 4 task beads.
- **Blocks:** `l4-p6-stage-gate-and-dod-walkthrough`.
- **Phase:** 4.

---

# Phase 5 Task Beads — Web Portal

---

## l4-p5-web-package-config: apps/web Package Configuration

- type: task
- priority: P0
- labels: layer-4,l4-phase-5,web,frontend,config
- parent: l4-web-portal

### Description

## SOURCE OF TRUTH WARNING

- **Spec**: `docs/strategy/layers/layer-4-shared-visibility.md` — §3.1.1, §5.4 (PRIMARY)
- **TDD**: `CI-L4-005`, `CI-L4-006`, `L4-T-contracts_spec-020` (PRIMARY)
- **Strategy**: Phase 5, P5-A
- **Bootstrap**: §5.3, §5.4, §5.5, §5.6

## Context

Net-new `apps/web/` workspace package: `package.json` (next 16, react 19, supabase-js 2.103.2, playwright 1.58.2), `tsconfig.json` (strict, bundler resolution), `next.config.ts` (reactStrictMode), `playwright.config.ts` (mirrors apps/desktop conventions: testDir './e2e', fullyParallel false, list reporter, single chromium, traces on failure, webServer via `bun run dev`).

## Files to Create/Modify

- **Create:** `apps/web/package.json` per bootstrap §5.3 exactly.
- **Create:** `apps/web/tsconfig.json` per bootstrap §5.4 exactly.
- **Create:** `apps/web/next.config.ts` per bootstrap §5.6.
- **Create:** `apps/web/playwright.config.ts` per bootstrap §5.5.

## Spec Requirements (verbatim)

**L4-FR-3.1.1 — `apps/web/` is a net-new read-only Next.js portal.**
- AC1: L4 creates `apps/web/` with the seven files listed below.

## Implementation Details

`apps/web/package.json` (verbatim from bootstrap §5.3):
```json
{
  "name": "@project/web",
  "private": true,
  "type": "module",
  "scripts": {
    "build": "next build",
    "lint": "bunx tsc -p tsconfig.json --noEmit",
    "test": "bunx tsc -p tsconfig.json --noEmit",
    "test:e2e": "playwright test"
  },
  "dependencies": {
    "@project/contracts-schema": "workspace:*",
    "@project/ui": "workspace:*",
    "@supabase/supabase-js": "2.103.2",
    "next": "16.1.0",
    "react": "19.2.4",
    "react-dom": "19.2.4"
  },
  "devDependencies": {
    "@playwright/test": "1.58.2",
    "@types/node": "25.5.0",
    "@types/react": "19.2.14",
    "@types/react-dom": "19.2.3",
    "typescript": "6.0.2"
  }
}
```

If `packages/contracts-schema` is absent at L4 implementation start, create that boundary package before wiring `apps/web`.

## Tests to Pass

- **`CI-L4-005`** (T1): `bun run test` exits 0 including `apps/web` test path.
- **`CI-L4-006`** (T1): `(cd apps/web && bun run test:e2e)` exits 0 against local Supabase stack with traces/screenshots on failure.
- **`L4-T-contracts_spec-020`** (T1, package lint): web app imports `packages/ui` and `packages/contracts-schema` but does NOT import desktop modules, Tauri, daemon, SQLite, IPC transport.

## Acceptance Criteria

- [ ] All 4 config files exist with the bootstrap-exact contents.
- [ ] No dependency ranges (use exact versions per bootstrap).
- [ ] `playwright.config.ts` mirrors apps/desktop conventions.
- [ ] CI-L4-005, CI-L4-006, L4-T-contracts_spec-020 pass.

## Dependencies

- **Blocked by:** `l4-p1-root-web-workspace-scripts`, `l4-p1-review-gate`.
- **Blocks:** all other Phase 5 web beads.
- **Phase:** 5.

---

## l4-p5-web-supabase-client-boundary: src/lib/supabase.ts Single Supabase Boundary

- type: task
- priority: P0
- labels: layer-4,l4-phase-5,web,supabase,boundary
- parent: l4-web-portal

### Description

## SOURCE OF TRUTH WARNING

- **Spec**: `docs/strategy/layers/layer-4-shared-visibility.md` — §3.1.7, §3.1.1 AC6 (PRIMARY)
- **TDD**: `L4-T-contracts_spec-013, 014, 015, 020` (PRIMARY)
- **Strategy**: Phase 5, P5-B

## Context

`apps/web/src/lib/supabase.ts` is the SINGLE direct Supabase client/package boundary in the web app. Initializes Supabase client with viewer-scoped anon key only; provides typed query helpers using `packages/contracts-schema` types. No service-role keys in browser.

## Files to Create/Modify

- **Create:** `apps/web/src/lib/supabase.ts`.

## Spec Requirements (verbatim)

**L4-FR-3.1.7 — `src/lib/supabase.ts` Supabase client initialization, typed query helpers using `packages/contracts-schema` types.**

**L4-FR-3.1.1 — `apps/web/` is a net-new read-only Next.js portal.**
- AC6: `apps/web/src/lib/supabase.ts` is the sole direct Supabase client/package boundary in the web app, and `apps/web/src/lib/broadcast.ts` consumes that boundary rather than importing Supabase directly.

## Tests to Pass

- **`L4-T-contracts_spec-013`** (T4, `WebPortalHarness`): viewer credential network requests can SELECT summaries/projections; direct browser-client query to `shared_company_profile`, `shared_source_configs`, `operator_decision_heads` receives explicit 401/403 or RLS denial. `L4-FR-2.5.9`, `L4-FR-3.1.1`.
- **`L4-T-contracts_spec-014`** (T4, `WebPortalHarness`): viewer attempts to call `update_operator_decision`, `apply_push_batch`, INSERT/UPDATE/DELETE fail at network/API boundary. `L4-FR-2.5.9`, `L4-FR-2.5.10`.
- **`L4-T-contracts_spec-015`** (T4, `WebPortalHarness`): bundled browser code/runtime contains no service-role/admin/operator/daemon token or local daemon endpoint. `L4-FR-2.5.9`, `L4-FR-3.1.1`.
- **`L4-T-contracts_spec-020`** (T1, package lint): web imports `packages/ui` + `packages/contracts-schema` only.

## Acceptance Criteria

- [ ] `apps/web/src/lib/supabase.ts` is the only file that imports `@supabase/supabase-js` in `apps/web/`.
- [ ] Uses `NEXT_PUBLIC_SUPABASE_URL` + `NEXT_PUBLIC_SUPABASE_ANON_KEY` only.
- [ ] Typed query helpers use `@project/contracts-schema` types.
- [ ] L4-T-contracts_spec-013/014/015/020 pass.

## Dependencies

- **Blocked by:** `l4-p5-web-package-config`.
- **Blocks:** `l4-p5-web-broadcast-client`, `l4-p5-web-contract-list`, `l4-p5-web-contract-detail`, `l4-p5-web-status-page`, `l4-p5-web-root-layout-and-redirect`.
- **Phase:** 5.

---

## l4-p5-web-broadcast-client: src/lib/broadcast.ts Subscription Manager

- type: task
- priority: P1
- labels: layer-4,l4-phase-5,web,broadcast,realtime
- parent: l4-web-portal

### Description

## SOURCE OF TRUTH WARNING

- **Spec**: `docs/strategy/layers/layer-4-shared-visibility.md` — §3.1.8, §3.2 (PRIMARY)
- **TDD**: `L4-T-contracts_spec-008, 009, 010, 019` (PRIMARY)
- **Strategy**: Phase 5, P5-B (after Supabase client)

## Context

Page-scoped Broadcast subscription manager. Topic schema: `org:{orgId}:contracts`, `process:{processId}:updates`, `org:{orgId}:status`, `org:{orgId}:errors`, `org:{orgId}:sync`. Three typed payload shapes: `contracts.changed`, `process.updated`, `sync.updated`. Imports Supabase via `lib/supabase.ts` (NOT `@supabase/supabase-js` directly).

## Files to Create/Modify

- **Create:** `apps/web/src/lib/broadcast.ts`.

## Spec Requirements (verbatim)

**L4-FR-3.1.8 — `src/lib/broadcast.ts` Supabase Broadcast subscription manager: topic-based invalidation, per-page subscription lifecycle.**

**L4-FR-3.2.1 — Broadcast subscriptions are page-scoped invalidation channels.**
- AC1: `/contracts` subscribes to `org:{orgId}:contracts` and `org:{orgId}:sync`.
- AC2: `/contracts/[id]` subscribes to `process:{processId}:updates` and `org:{orgId}:sync`.
- AC3: `/status` subscribes to `org:{orgId}:status`, `org:{orgId}:errors`, `org:{orgId}:sync`.
- AC4: Broadcast topics carry invalidation hints only, not full projection rows.

**L4-FR-3.2.2 — Broadcast payloads use the three typed invalidation shapes.**
- AC1-AC4: `contracts.changed`/`process.updated`/`sync.updated` payload schemas as in spec §3.2.

**Type definitions (verbatim from spec):**
```typescript
type PortalUpdate =
  | { event: 'contracts.changed'; processIds: string[]; changed: Array<'assessment'|'decision'|'manifest'|'error'>; maxSeq: number }
  | { event: 'process.updated'; processId: string; changed: Array<'quotes'|'landed_cost'|'manifest'|'jobs'|'errors'>; major: boolean; maxSeq: number }
  | { event: 'sync.updated'; syncHealth: 'healthy'|'degraded'|'offline'; lastAckedSeq: number; lastSuccessAt: string; projectionGeneration: number }
```

## Tests to Pass

- **`L4-T-contracts_spec-008`** (T4): `/contracts` subscribes correctly; after remote projection write commits, Broadcast invalidation hint received within 3s, `contracts.changed` shows "N new/updated" pill, visible rows do NOT reorder. `L4-FR-3.2.1, 3.2.2, 3.2.4`.
- **`L4-T-contracts_spec-009`** (T4): detail page subscribes; minor `process.updated` refreshes only affected section. `L4-FR-3.2.1, 3.2.4`.
- **`L4-T-contracts_spec-010`** (T4): major `process.updated` shows refresh chip and does not replace visible content until user action. `L4-FR-3.2.2, 3.2.4`.
- **`L4-T-contracts_spec-019`** (T4): `sync.updated` with increased `projectionGeneration` invalidates stale page snapshots and refetches without mixing old generation rows. `L4-FR-3.2.2, 3.7.1, 5.2.1`.

## Acceptance Criteria

- [ ] `broadcast.ts` exports per-page subscribe helpers (list/detail/status).
- [ ] All 5 topic patterns implemented.
- [ ] All 3 payload shapes typed exactly per spec.
- [ ] Imports go through `lib/supabase.ts` (no direct `@supabase/supabase-js` import).
- [ ] L4-T-contracts_spec-008/009/010/019 pass.

## Dependencies

- **Blocked by:** `l4-p5-web-supabase-client-boundary`.
- **Blocks:** `l4-p5-web-contract-detail`, `l4-p5-review-gate`.
- **Phase:** 5.

---

## l4-p5-web-root-layout-and-redirect: layout.tsx and page.tsx

- type: task
- priority: P1
- labels: layer-4,l4-phase-5,web
- parent: l4-web-portal

### Description

## SOURCE OF TRUTH WARNING

- **Spec**: `docs/strategy/layers/layer-4-shared-visibility.md` — §3.1.2, §3.1.3 (PRIMARY)
- **TDD**: `L4-T-contracts_spec-001, 002` (PRIMARY)
- **Strategy**: Phase 5, P5-C

## Context

`apps/web/src/app/layout.tsx`: root layout with global sync status badge driven by `viewer_sync_status`. `apps/web/src/app/page.tsx`: redirect to `/contracts`. Status badge comes from shared `packages/ui/freshness-badge.tsx` extension.

## Files to Create/Modify

- **Create:** `apps/web/src/app/layout.tsx`.
- **Create:** `apps/web/src/app/page.tsx`.

## Spec Requirements (verbatim)

**L4-FR-3.1.2 — `src/app/layout.tsx` Root layout with global sync status badge (`viewer_sync_status`).**
**L4-FR-3.1.3 — `src/app/page.tsx` Redirect to contracts list.**

## Tests to Pass

- **`L4-T-contracts_spec-001`** (T4): `/` redirects to `/contracts` and renders read-only contracts list using viewer credentials and projection tables. `L4-FR-3.1.1, 3.1.3, 3.1.4`.
- **`L4-T-contracts_spec-002`** (T4): root layout renders global sync status from `viewer_sync_status` and never calls daemon IPC or SQLite transport. `L4-FR-3.1.2, 3.3.4`.

## Acceptance Criteria

- [ ] `layout.tsx` includes the FreshnessBadge bound to `viewer_sync_status` projection.
- [ ] `page.tsx` redirects to `/contracts`.
- [ ] No daemon IPC or SQLite imports.
- [ ] L4-T-contracts_spec-001/002 pass.

## Dependencies

- **Blocked by:** `l4-p5-web-supabase-client-boundary`.
- **Blocks:** `l4-p5-review-gate`.
- **Phase:** 5.

---

## l4-p5-web-contract-list: contracts/page.tsx Keyset-Paginated List

- type: task
- priority: P1
- labels: layer-4,l4-phase-5,web
- parent: l4-web-portal

### Description

## SOURCE OF TRUTH WARNING

- **Spec**: `docs/strategy/layers/layer-4-shared-visibility.md` — §3.1.4, §3.2.4 (PRIMARY)
- **TDD**: `L4-T-contracts_spec-001, 004, 008` (PRIMARY)
- **Strategy**: Phase 5, P5-C

## Context

Keyset-paginated opportunity list using Supabase client + `packages/ui` `ContractCardFrame`. Live updates show "N new/updated" pill — visible rows do NOT reorder while user is reading.

## Files to Create/Modify

- **Create:** `apps/web/src/app/contracts/page.tsx`.

## Spec Requirements (verbatim)

**L4-FR-3.1.4 — `src/app/contracts/page.tsx` Keyset-paginated opportunity list using Supabase client + `packages/ui` `ContractCardFrame`. Shows "N new/updated" pill on live updates (not live reorder).**

**L4-FR-3.2.4 — Web pages respond to invalidation without disruptive content swaps.**
- AC1: List page live updates never reorder the visible list while a user is reading it.

## Tests to Pass

- **`L4-T-contracts_spec-001`** (T4): list renders read-only with viewer credentials.
- **`L4-T-contracts_spec-004`** (T4): keyset pagination fetches page 1 + page 2 without duplicates, preserves visible ordering, uses cursor fields rather than offset pagination. `L4-FR-3.1.4`.
- **`L4-T-contracts_spec-008`** (T4): subscribes to `org:{orgId}:contracts`, displays "N new/updated" pill on Broadcast invalidation, does NOT reorder visible rows. `L4-FR-3.2.1, 3.2.2, 3.2.4`.

## Acceptance Criteria

- [ ] List query uses keyset pagination (cursor-based, NOT offset).
- [ ] Reuses `packages/ui` `ContractCardFrame` unchanged.
- [ ] Pill UI on Broadcast `contracts.changed`; no reorder.
- [ ] L4-T-contracts_spec-001/004/008 pass.

## Dependencies

- **Blocked by:** `l4-p5-web-supabase-client-boundary`.
- **Blocks:** `l4-p5-review-gate`.
- **Phase:** 5.

---

## l4-p5-web-contract-detail: contracts/[id]/page.tsx Detail Page

- type: task
- priority: P1
- labels: layer-4,l4-phase-5,web
- parent: l4-web-portal

### Description

## SOURCE OF TRUTH WARNING

- **Spec**: `docs/strategy/layers/layer-4-shared-visibility.md` — §3.1.5, §3.2.4 (PRIMARY)
- **TDD**: `L4-T-contracts_spec-009, 010, 011` (PRIMARY)
- **Strategy**: Phase 5, P5-C

## Context

Opportunity detail page: candidates, quotes, margins, manifest, errors, decision summary. Auto-refresh for minor changes; non-destructive refresh chip for major changes (manifest, landed cost, canonical process). Reuses `packages/ui` `ManifestViewer`, `PipelineBadge`, `ErrorSummaryList`.

## Files to Create/Modify

- **Create:** `apps/web/src/app/contracts/[id]/page.tsx`.

## Spec Requirements (verbatim)

**L4-FR-3.1.5 — `src/app/contracts/[id]/page.tsx` Opportunity detail: candidates, quotes, margins, manifest, errors. Auto-refresh for minor changes; non-destructive refresh chip for major changes.**

**L4-FR-3.2.4 — Web pages respond to invalidation without disruptive content swaps.**
- AC2: Minor updates auto-refresh only the affected section.
- AC3: Major updates show a refresh chip and wait for user action before replacing visible content.

## Tests to Pass

- **`L4-T-contracts_spec-009`** (T4): detail subscribes to `process:{processId}:updates` + `org:{orgId}:sync`; minor `process.updated` refreshes only affected section. `L4-FR-3.2.1, 3.2.4`.
- **`L4-T-contracts_spec-010`** (T4): major `process.updated` for manifest/landed cost/canonical process shows refresh chip and does not replace visible content until user action. `L4-FR-3.2.2, 3.2.4`.
- **`L4-T-contracts_spec-011`** (T4): renders candidates, quotes, margins, manifest, errors, decision summary from projection/summary tables using `packages/ui` components. `L4-FR-3.1.5, 2.2.1..2.2.6`.

## Acceptance Criteria

- [ ] Detail uses `packages/ui` components without modification.
- [ ] Minor/major Broadcast distinction implemented.
- [ ] No mutation controls.
- [ ] L4-T-contracts_spec-009/010/011 pass.

## Dependencies

- **Blocked by:** `l4-p5-web-supabase-client-boundary`, `l4-p5-web-broadcast-client`.
- **Blocks:** `l4-p5-review-gate`.
- **Phase:** 5.

---

## l4-p5-web-status-page: status/page.tsx Status Page

- type: task
- priority: P1
- labels: layer-4,l4-phase-5,web
- parent: l4-web-portal

### Description

## SOURCE OF TRUTH WARNING

- **Spec**: `docs/strategy/layers/layer-4-shared-visibility.md` — §3.1.6, §3.7.1 (PRIMARY)
- **TDD**: `L4-T-contracts_spec-012, 019` (PRIMARY)
- **Strategy**: Phase 5, P5-C

## Context

Pipeline status, job summaries, error summaries, sync health. Reads `viewer_sync_status`, `workflow_job_summaries`, `error_manifest_summaries`. Shows `projection_generation` so user can see epoch-repair reseeds.

## Files to Create/Modify

- **Create:** `apps/web/src/app/status/page.tsx`.

## Spec Requirements (verbatim)

**L4-FR-3.1.6 — `src/app/status/page.tsx` Pipeline status, job summaries, error summaries, sync health.**

**L4-FR-3.7.1 — `viewer_sync_status` is the Supabase sync freshness projection (relevant AC).**
- AC5: `projection_generation` increments after epoch-repair reseed and appears in next `sync.updated` Broadcast payload.

## Tests to Pass

- **`L4-T-contracts_spec-012`** (T4): status page renders pipeline status, job summaries, error summaries, sync health, projection generation from summary projections. `L4-FR-3.1.6, 3.7.1`.
- **`L4-T-contracts_spec-019`** (T4): `sync.updated` with increased `projectionGeneration` invalidates stale snapshots without mixing old-generation rows.

## Acceptance Criteria

- [ ] Status page reads only summary projections (no raw tables).
- [ ] Reuses existing `packages/ui` `PipelineBadge`, `FreshnessBadge`, and `ErrorSummaryList` components unchanged for status, freshness, and error-summary presentation.
- [ ] Displays `projection_generation`.
- [ ] L4-T-contracts_spec-012/019 pass.

## Dependencies

- **Blocked by:** `l4-p5-web-supabase-client-boundary`.
- **Blocks:** `l4-p5-review-gate`.
- **Phase:** 5.

---

## l4-p5-web-freshness-consumption-and-boundary: FreshnessBadge Three-State + Viewer Credential Boundary

- type: task
- priority: P0
- labels: layer-4,l4-phase-5,web,ui,security,chokepoint
- parent: l4-web-portal

### Description

## SOURCE OF TRUTH WARNING

- **Spec**: `docs/strategy/layers/layer-4-shared-visibility.md` — §3.3.1, §3.3.2, §3.3.3, §3.3.4, §3.3.5, §2.5.9 (PRIMARY)
- **TDD**: `L4-T-contracts_spec-005, 006, 007, 016`, `L4-T-redaction_boundary-009, 025`, `CI-L4-008` (PRIMARY)
- **Strategy**: Phase 5, P5-D

## Context

Three-state freshness UX (green <90s, amber 90s-5min, red >5min). Extends `packages/ui/freshness-badge.tsx` with `syncHealth: 'healthy'|'degraded'|'offline'` and `lastSuccessAt: ISO-8601` while preserving the VM display path. Viewer credential boundary: bundle scan asserts no service-role/admin/operator/daemon tokens. `packages/ui` import boundary preserved.

## Files to Create/Modify

- **Modify:** `packages/ui/freshness-badge.tsx` — add 3-state input.
- **Create or modify:** `apps/web/e2e/contracts.spec.ts` — freshness state tests + viewer boundary tests.
- **Create:** `apps/web/e2e/fixtures/supabase-auth.ts` and `projection-seed.ts` per bootstrap §9.6.

## Spec Requirements (verbatim)

**L4-FR-3.3.1 — Healthy state.** Green badge, "Data current". `< 90s`.
**L4-FR-3.3.2 — Degraded state.** Amber badge, "Updates delayed". `90s - 5min`.
**L4-FR-3.3.3 — Offline state.** Red badge, "Sync offline". `> 5min`.
**L4-FR-3.3.4 — `viewer_sync_status` drives freshness.**
- AC5: `packages/ui/freshness-badge.tsx` accepts `syncHealth: 'healthy'|'degraded'|'offline'` and `lastSuccessAt: ISO-8601 string` plus VM display VM path; the 90s and 300s thresholds are computed inside the component or co-located pure helper.

**L4-FR-3.3.5 — Desktop adds sync health without replacing L2 control-plane state.**

**L4-FR-2.5.9 — Web read-only access enforced by RLS, not UI hiding.**
- AC1-AC4: viewer-scoped credentials only, no admin/daemon privileges in browser.

## Tests to Pass

- **`L4-T-contracts_spec-005`** (T4): green appears when `lastPushSuccessAt` younger than 90s. `L4-FR-3.3.1, 3.3.4`.
- **`L4-T-contracts_spec-006`** (T4): amber for 90s-5min. `L4-FR-3.3.2, 3.3.4`.
- **`L4-T-contracts_spec-007`** (T4): red after 5min or explicit offline health. `L4-FR-3.3.3, 3.3.4`.
- **`L4-T-contracts_spec-016`** (T1, package lint): `packages/ui` remains presentation-only — no Supabase, Broadcast, Tauri, daemon, desktop, transport imports; `apps/web/src/lib/supabase.ts` is sole direct Supabase boundary.
- **`L4-T-redaction_boundary-009`** (T2): viewer credentials can SELECT projection/summary rows but cannot INSERT/UPDATE/DELETE/call mutation RPCs/SELECT raw remote-owned tables. `L4-FR-2.5.9`.
- **`L4-T-redaction_boundary-025`** (T2): browser-held viewer credentials contain no operator/admin/daemon/service-role privileges; cannot use service-key-only endpoints. `L4-FR-2.5.9, 3.1.1`.
- **`CI-L4-008`** (T1): `packages/ui` import boundary clean.

## Acceptance Criteria

- [ ] FreshnessBadge accepts new props per spec §3.3.4 AC5.
- [ ] Thresholds (90s, 300s) computed inside the component.
- [ ] `packages/ui` retains existing imports only (no Supabase/Broadcast/Tauri/daemon/desktop/transport).
- [ ] Bundle inspection finds zero privileged tokens.
- [ ] L4-T-contracts_spec-005/006/007/016, L4-T-redaction_boundary-009/025, CI-L4-008 pass.

## Dependencies

- **Blocked by:** `l4-p5-web-package-config`.
- **Blocks:** `l4-p5-review-gate`, `l4-p6-final-rls-redaction-closure`, `l4-p6-two-user-verification`.
- **Phase:** 5.

---

## l4-p5-review-gate: Phase 5 Review Gate — Web Portal Cumulative

- type: task
- priority: P0
- labels: layer-4,l4-phase-5,review-gate
- parent: l4-web-portal

### Description

## SOURCE OF TRUTH WARNING

- **Spec**: §3.1, §3.2, §3.3 (PRIMARY)
- **TDD**: `L4-T-contracts_spec-001..020`; `L4-T-redaction_boundary-009, 025` (PRIMARY)
- **Strategy**: Phase 5 review row

## Context

Phase 5 cumulative gate. Web portal complete and verified read-only. Closes Phase 5 to unblock Phase 6 final RLS closure + two-user verification.

## Verification Commands

```bash
bun run test
bun run lint
(cd apps/web && bun run test)
(cd apps/web && bun run test:e2e)
```

## Tests to Pass (cumulative)

- `L4-T-contracts_spec-001..020` (all 20 web E2E + lint tests).
- `L4-T-redaction_boundary-009, 025` (viewer boundary).
- `CI-L4-005, 006, 008`.

## Acceptance Criteria

- [ ] All 8 Phase 5 task beads closed.
- [ ] Listed Phase 5 gate IDs pass.
- [ ] No `packages/ui` imports of Supabase/Broadcast/Tauri/daemon/desktop/transport.
- [ ] No privileged tokens in browser bundle.

## Dependencies

- **Blocked by:** all 8 Phase 5 task beads.
- **Blocks:** `l4-p6-final-rls-redaction-closure`, `l4-p6-two-user-verification`.
- **Phase:** 5.

---

# Phase 6 Task Beads — Integration and Hardening

---

## l4-p6-main-startup-sync-wiring: main.rs Startup Pull-Before-Push, Lease, Epoch Wiring

- type: task
- priority: P0
- labels: layer-4,l4-phase-6,rust,main,chokepoint,integration
- parent: l4-integration-hardening

### Description

## SOURCE OF TRUTH WARNING

- **Spec**: `docs/strategy/layers/layer-4-shared-visibility.md` — §1.4.3, §1.6, §1.7 (PRIMARY)
- **TDD**: `L4-T-sqlite_supabase_sync-028, 039..046` (PRIMARY)
- **Strategy**: Phase 6, P6-A (single L4 edit to `main.rs` — chokepoint)

## Context

**Single L4 edit home for `crates/daemon/src/main.rs`.** Wires startup ordering: pull-before-push, active-writer lease acquisition, schema-epoch checks, pull cursor advancement, `viewer_sync_status` update wiring, and push dispatcher invocation. No Phase 2-5 implementation bead edits this file — they all defer wiring here.

## Files to Create/Modify

- **Modify:** `crates/daemon/src/main.rs` — startup sync sequence in the daemon initialization path.

## Spec Requirements (verbatim)

**L4-FR-1.4.3 — Sync cycles pull remote-owned state before pushing.**
- AC1: On daemon sync startup and every scheduled/manual sync cycle, remote-owned pulls complete before pending operator intents are flushed.

**L4-FR-1.6.1, 1.6.2 — Active-writer lease + writer epoch fencing.**
- Daemon acquires lease at startup before push dispatch; heartbeat at ≤30s while active.

**L4-FR-1.7.1 — Schema epoch mismatch pauses sync.**
- AC2: Local SQLite `PRAGMA user_version` and remote `schema_metadata.schema_epoch` compared before apply; mismatch pauses sync.

## Tests to Pass

- **`L4-T-sqlite_supabase_sync-028`** (T2): startup completes pulls before pending-intent reconciliation, CAS flush, push dispatch.
- **`L4-T-sqlite_supabase_sync-039`** (T2): first daemon executor acquires/renews lease at startup.
- **`L4-T-sqlite_supabase_sync-040`** (T2): second daemon rejected.
- **`L4-T-sqlite_supabase_sync-041`** (T3): partitioned daemon recovering with old `writer_epoch` cannot push.
- **`L4-T-sqlite_supabase_sync-042`** (T2): heartbeat every ≤30s.
- **`L4-T-sqlite_supabase_sync-044`** (T2): schema epoch mismatch pauses sync.
- **`L4-T-sqlite_supabase_sync-046`** (T2): `viewer_sync_status` updated after apply + heartbeat.

## Acceptance Criteria

- [ ] Single edit to `main.rs` (no other L4 bead modifies this file).
- [ ] Startup sequence: load db → check schema epoch → acquire lease → run pull-before-push remote-owned pull cycle → advance pull cursors after successful apply → start CAS flush → invoke push dispatcher.
- [ ] Heartbeat task spawned and tied to lease.
- [ ] `viewer_sync_status` update wiring records successful apply/heartbeat state for web freshness.
- [ ] L4-T-sqlite_supabase_sync-028, 039..046 pass under integration.

## Dependencies

- **Blocked by:** `l4-p2-review-gate`, `l4-p3-review-gate`.
- **Blocks:** `l4-p6-stage-gate-and-dod-walkthrough`.
- **Phase:** 6.

---

## l4-p6-epoch-repair-generation: projection_generations Reseed Path

- type: task
- priority: P0
- labels: layer-4,l4-phase-6,rust,sync,epoch,repair
- parent: l4-integration-hardening

### Description

## SOURCE OF TRUTH WARNING

- **Spec**: `docs/strategy/layers/layer-4-shared-visibility.md` — §5.2, §1.7.1 (PRIMARY)
- **TDD**: `L4-T-sqlite_schema-007`, `L4-T-sqlite_supabase_sync-045`, `L4-T-contracts_spec-019` (PRIMARY)
- **Strategy**: Phase 6, P6-B (after Phase 2 epoch pause)

## Context

After epoch repair, daemon: (1) inserts `projection_generations` row with status='started' + `max_local_seq_before_reseed`; (2) republishes current row images from local authoritative push-owned tables with active `projection_generation`; (3) discards stale-epoch outbox entries only after reseed succeeds (status='published'); (4) updates `viewer_sync_status.projection_generation`; (5) emits `sync.updated` Broadcast with new `projectionGeneration`.

## Files to Create/Modify

- **Modify:** `crates/daemon/src/db/sync.rs` — `EpochRepair::reseed()` flow.

## Spec Requirements (verbatim)

**L4-FR-5.2.1 — Epoch-repair reproject minimum path is part of L4.**
- AC1: After epoch repair, daemon marks a new `projection_generation`.
- AC2: Daemon inserts a `projection_generations` row with `status = 'started'` and `max_local_seq_before_reseed` before republishing.
- AC3: Daemon republishes current row images from local authoritative push-owned tables with the active `projection_generation`.
- AC4: Stale-epoch outbox entries are discarded only after the reseed succeeds and the repair row reaches `status = 'published'`.
- AC5: A test proves remote projection catches up after epoch mismatch without relying on manual full rebuild.

## Tests to Pass

- **`L4-T-sqlite_schema-007`** (T1): `projection_generations` schema (DDL).
- **`L4-T-sqlite_supabase_sync-045`** (T3, `ProjectionGenerationHarness`): after epoch repair, daemon creates `projection_generations` row, republishes current local row images, publishes `viewer_sync_status.projection_generation`, discards stale-epoch outbox rows only after success. `L4-FR-5.2.1`.
- **`L4-T-contracts_spec-019`** (T4): `sync.updated` with increased `projectionGeneration` invalidates stale page snapshots and refetches without mixing old generation rows. `L4-FR-3.2.2, 3.7.1, 5.2.1`.

## Acceptance Criteria

- [ ] Reseed flow inserts started row first, publishes after success.
- [ ] Stale-epoch outbox entries discarded ONLY after reseed published.
- [ ] `viewer_sync_status.projection_generation` updated post-reseed.
- [ ] Web invalidation works on `projectionGeneration` change.
- [ ] L4-T-sqlite_schema-007 + L4-T-sqlite_supabase_sync-045 + L4-T-contracts_spec-019 pass.

## Dependencies

- **Blocked by:** `l4-p2-schema-epoch-pause-and-backup`, `l4-p2-review-gate`.
- **Blocks:** `l4-p6-logical-consistency-auditor-stub`, `l4-p6-stage-gate-and-dod-walkthrough`.
- **Phase:** 6.

---

## l4-p6-final-rls-redaction-closure: Final RLS Policy Closure + 26 Redaction Tests

- type: task
- priority: P0
- labels: layer-4,l4-phase-6,supabase,rls,redaction
- parent: l4-integration-hardening

### Description

## SOURCE OF TRUTH WARNING

- **Spec**: `docs/strategy/layers/layer-4-shared-visibility.md` — §2.5, §2.5.11, §2.6.10 (PRIMARY)
- **TDD**: `L4-T-redaction_boundary-001..026` (full file, PRIMARY)
- **Strategy**: Phase 6, P6-C (after Phases 1+5)

## Context

Final RLS policy content in `infra/supabase/migrations/0004_rls_policies.sql` and full 26 redaction tests in `crates/daemon/tests/redaction_boundary.rs`. Verifies all 28 role/category cells, all 6 summary projections (no leakage of operator identity, company profile, raw supplier, tender metadata, full error context, validator reasoning, command history), tombstone visibility for daemon, mutation rights through RPC only.

## Files to Create/Modify

- **Modify:** `infra/supabase/migrations/0004_rls_policies.sql` — final policy content.
- **Modify:** `crates/daemon/tests/redaction_boundary.rs` — finalize all 26 tests + harness extensions.

## Spec Requirements (verbatim)

**L4-FR-2.5.1 through L4-FR-2.5.11 — Full RLS matrix + redaction boundary tests.** (Per spec §2.5 and acceptance criteria above; full file content from spec §2.5).

**L4-FR-1.9.13 — RLS on all Supabase tables (Constitutional).**

## Tests to Pass

All 26 `L4-T-redaction_boundary-001..026` tests pass:
- 001..009: RLS role matrix per category.
- 010..020: summary projection redaction (sentinel-based).
- 021: tombstone visibility for daemon (vs hidden from viewer/operator).
- 022: operator direct UPDATE blocked, RPC succeeds/conflicts.
- 023: daemon direct upsert denied; RPC succeeds.
- 024: admin maintenance mutation OK with tombstone/seq triggers respected.
- 025: browser-held viewer credentials carry no privileged tokens.
- 026: schema hash coordination test.

## Acceptance Criteria

- [ ] Final 0004 RLS policies cover all 7 table categories × 4 roles.
- [ ] Daemon SELECT does NOT filter `deleted_at IS NULL` for remote-owned tables.
- [ ] All 6 summary projections pass sentinel-based redaction.
- [ ] L4-T-redaction_boundary-001..026 all pass.

## Dependencies

- **Blocked by:** `l4-p1-supabase-config-and-migration-bundle`, `l4-p1-schema-hash-ci-gate`, `l4-p5-review-gate`.
- **Blocks:** `l4-p6-stage-gate-and-dod-walkthrough`.
- **Phase:** 6.

---

## l4-p6-tombstone-retention-proof: 14-Day Tombstone Retention + Daemon Pull Visibility

- type: task
- priority: P1
- labels: layer-4,l4-phase-6,supabase,tombstone,retention
- parent: l4-integration-hardening

### Description

## SOURCE OF TRUTH WARNING

- **Spec**: `docs/strategy/layers/layer-4-shared-visibility.md` — §3.6.17, §2.5.10 (PRIMARY)
- **TDD**: `L4-T-sqlite_supabase_sync-078`, `L4-T-redaction_boundary-021` (PRIMARY)
- **Strategy**: Phase 6, P6-D

## Context

Tombstone retention default ≥14 days; daemon pull policies can see tombstoned remote-owned rows until retention expires. If shorter retention is chosen, periodic anti-entropy/full-hash diff is required.

## Files to Create/Modify

- **Modify or create:** documentation + retention configuration in `infra/supabase/migrations/0004_rls_policies.sql` (daemon SELECT does not filter `deleted_at IS NULL`).
- **Add:** retention test in `crates/daemon/tests/sqlite_supabase_sync.rs`.

## Spec Requirements (verbatim)

**L4-FR-3.6.17 — Tombstones remain visible long enough for offline daemons.**
- AC1: Default tombstone retention is at least 14 days.
- AC2: Retention is at least max expected offline duration plus safety margin.
- AC3: Daemon pull policies can see tombstoned remote-owned rows until retention expires.
- AC4: If retention is shortened, periodic anti-entropy or full-hash diff is required.

## Tests to Pass

- **`L4-T-sqlite_supabase_sync-078`** (T2, `PullCursorHarness` + `RoleMatrixFixture`): default tombstone retention ≥14 days; daemon pull policies see tombstoned rows until retention expires. `L4-FR-3.6.17, 2.5.10`.
- **`L4-T-redaction_boundary-021`** (T2): tombstoned rows hidden from viewer/operator portal SELECT; visible to daemon pull until retention. `L4-FR-2.5.10, 3.6.17`.

## Acceptance Criteria

- [ ] Default tombstone retention configured at ≥14 days.
- [ ] Daemon RLS policy on remote-owned tables omits `deleted_at IS NULL` filter.
- [ ] L4-T-sqlite_supabase_sync-078 + L4-T-redaction_boundary-021 pass.

## Dependencies

- **Blocked by:** `l4-p3-review-gate`, `l4-p6-final-rls-redaction-closure`.
- **Blocks:** `l4-p6-stage-gate-and-dod-walkthrough`.
- **Phase:** 6.

---

## l4-p6-cross-authority-provenance-and-ttl: Cross-Authority Provenance Recording + Safety-Critical TTL

- type: task
- priority: P1
- labels: layer-4,l4-phase-6,rust,sync,provenance
- parent: l4-integration-hardening

### Description

## SOURCE OF TRUTH WARNING

- **Spec**: `docs/strategy/layers/layer-4-shared-visibility.md` — §2.7, §1.8.2 (PRIMARY)
- **TDD**: `L4-T-sqlite_supabase_sync-047, 049`, `L4-T-contracts_spec-018` (PRIMARY)
- **Strategy**: Phase 6, P6-D

## Context

Each pipeline run records `source_config_revision`, `company_profile_revision`, `operator_decision_revision` (when consumed). Enables replay + blame. Safety-critical TTL behavior verified end-to-end through web (degraded UX shown).

## Files to Create/Modify

- **Modify:** push-owned write paths under `crates/daemon/src/jobs/runner.rs` and pipeline modules to record provenance revisions.
- **Modify:** `crates/daemon/tests/sqlite_supabase_sync.rs` — provenance test (049).

## Spec Requirements (verbatim)

**L4-FR-2.7.1 — Pipeline output → operator review (Display, no authority conflict).** Provenance: `decisionBasisHash` links decision to pipeline snapshot.
**L4-FR-2.7.2 — Operator decision → bid workflow (Automation, must track decision provenance).**
**L4-FR-2.7.3 — Coverage metrics → qualification gate (Control, internal).**
**L4-FR-2.7.4 — Sync health → UX mode (Display).**
**L4-FR-2.7.5 — Pipeline runs record cross-authority provenance revisions.**
- AC1-AC4: Runs that consume `shared_source_configs`/`shared_company_profile`/`operator_decision_heads` record corresponding revision; provenance persisted with run.

**L4-FR-1.8.2 — Remote-owned config staleness classes** (already covered by P3, finalized here for E2E).

## Tests to Pass

- **`L4-T-sqlite_supabase_sync-047`** (T2): safety-critical TTL degrades disabled source after 4h.
- **`L4-T-sqlite_supabase_sync-049`** (T2): pipeline runs that consume source config / company profile / operator decision state persist corresponding remote revision provenance. `L4-FR-2.7.1..2.7.5`.
- **`L4-T-contracts_spec-018`** (T4): degraded/offline state changes shown in web; desktop remains locally read-write.

## Acceptance Criteria

- [ ] Provenance fields recorded per pipeline run.
- [ ] Web shows degraded/offline state correctly during sync interruption.
- [ ] L4-T-sqlite_supabase_sync-049 + L4-T-contracts_spec-018 pass.

## Dependencies

- **Blocked by:** `l4-p3-review-gate`, `l4-p3-remote-config-staleness-ttl`, `l4-p5-review-gate`.
- **Blocks:** `l4-p6-logical-consistency-auditor-stub`, `l4-p6-stage-gate-and-dod-walkthrough`.
- **Phase:** 6.

---

## l4-p6-logical-consistency-auditor-stub: Logical Consistency Auditor Minimum Stub

- type: task
- priority: P2
- labels: layer-4,l4-phase-6,rust,sync,audit
- parent: l4-integration-hardening

### Description

## SOURCE OF TRUTH WARNING

- **Spec**: `docs/strategy/layers/layer-4-shared-visibility.md` — §5.2 stubs (PRIMARY)
- **TDD**: `L4-T-sqlite_supabase_sync-049`, `CI-L4-004` (PRIMARY)
- **Strategy**: Phase 6, P6-D (after epoch + provenance)

## Context

Phase 6 minimum stub: testable domain-invariant check entrypoint with conditional auto-repair explicitly DISABLED unless a safe repair rule is registered. NOT documentation-only or empty. Strategy says: "must expose a testable domain-invariant check entrypoint with conditional auto-repair explicitly disabled unless a safe repair rule is registered; the Phase 6 bead links this to L4-T-sqlite_supabase_sync-049 provenance assertions or amends the TDD before implementation if a distinct test ID is required."

## Files to Create/Modify

- **Create:** `crates/daemon/src/audit/consistency.rs` — testable invariant check entry.

## Spec Requirements (verbatim)

**§5.2 Stubs:** "Logical consistency auditor — Not implemented. Future: domain invariant checking with conditional auto-repair." Phase 6 owns the minimum stub.

## Tests to Pass

- **`L4-T-sqlite_supabase_sync-049`** (T2): provenance assertions exercise the auditor entrypoint. `L4-FR-2.7.1..2.7.5`.
- **`CI-L4-004`** (T1): full stage gate.

## Acceptance Criteria

- [ ] `audit/consistency.rs` exposes a callable check function (NOT TODO/empty).
- [ ] Auto-repair OFF by default; safe repair rule registration is the API surface.
- [ ] Reuses provenance test for assertion (or extends TDD if new ID needed — strategy explicitly allows this).
- [ ] L4-T-sqlite_supabase_sync-049 + CI-L4-004 pass.

## Dependencies

- **Blocked by:** `l4-p6-epoch-repair-generation`, `l4-p6-cross-authority-provenance-and-ttl`.
- **Blocks:** `l4-p6-stage-gate-and-dod-walkthrough`.
- **Phase:** 6.

---

## l4-p6-two-user-verification: Two-User Desktop + Web Verification Protocol

- type: task
- priority: P0
- labels: layer-4,l4-phase-6,manual,verification,milestone
- parent: l4-integration-hardening

### Description

## SOURCE OF TRUTH WARNING

- **Spec**: `docs/strategy/layers/layer-4-shared-visibility.md` — §1.1 capability, §1.8.1, §3.1.1, §3.3.5 (PRIMARY)
- **TDD**: `L4-T-contracts_spec-017, 018` (PRIMARY)
- **Strategy**: Phase 6, P6-E (after Phases 2-5)

## Context

Manual verification + automated E2E. Desktop operator (read-write) seeds projection data through sync path; web viewer signs in with viewer-scoped Supabase credentials and observes the same process list/detail/status without mutation affordances. `viewer_sync_status` badges match. A smoke diary records: desktop action, expected web observation, observed sync latency, blocked web mutation attempt, matching sync-state badges.

## Files to Create/Modify

- **Create:** `docs/pipeline/l4-shared-visibility/artifacts/l4-two-user-smoke-diary.md` — verification protocol + diary template.
- **Verify:** `apps/web/e2e/contracts.spec.ts` test cases 017, 018 pass against integrated push.

## Spec Requirements (verbatim)

**§1.1 Layer Narrative:** "Two users can now view the same opportunity pipeline — one via desktop (read-write), one via web (read-only)."

**§4.1 DoD #12:** Two users can view the same pipeline: desktop (read-write) + web (read-only). Verification: `apps/web/e2e/contracts.spec.ts` + manual smoke.

**Strategy two-user pass criteria (verbatim):** "the viewer sees the same process list, detail, status, decision summary, and freshness state that the desktop operator just produced; the web session exposes no mutation controls or privileged credentials; `viewer_sync_status` badges match between desktop sync health and web freshness."

## Tests to Pass

- **`L4-T-contracts_spec-017`** (T4, `WebPortalHarness` + desktop seed fixture): desktop writer seeds projection data through sync path, web viewer sees same process list/detail/status without mutation affordances. `L4-FR-1.8.1, 3.1.1, 3.3.5`.
- **`L4-T-contracts_spec-018`** (T4): when sync degraded/offline or Broadcast killed mid-session, desktop remains locally read-write, web shows stale amber/red and red realtime badge, no Broadcast data-change updates applied, subsequent fetch degrades gracefully without exposing mutation controls or privileged errors.

## Acceptance Criteria

- [ ] Smoke diary template exists with: desktop action, expected web observation, observed latency, blocked mutation attempt log, badge match.
- [ ] L4-T-contracts_spec-017/018 pass under integrated push/web behavior.
- [ ] Diary linked to test runs.
- [ ] DoD row #12 verified.

## Dependencies

- **Blocked by:** `l4-p2-review-gate`, `l4-p3-review-gate`, `l4-p4-review-gate`, `l4-p5-review-gate`.
- **Blocks:** `l4-p6-stage-gate-and-dod-walkthrough`.
- **Phase:** 6.

---

## l4-p6-stage-gate-and-dod-walkthrough: Phase 6 Review — Full Stage Gate + DoD 1..18 Walkthrough

- type: task
- priority: P0
- labels: layer-4,l4-phase-6,review-gate,dod,milestone
- parent: l4-integration-hardening

### Description

## SOURCE OF TRUTH WARNING

- **Spec**: `docs/strategy/layers/layer-4-shared-visibility.md` — §4.1 DoD (PRIMARY)
- **TDD**: `CI-L4-001..008`; full Stage Validation Test Gate; all 4 test files (PRIMARY)
- **Strategy**: Phase 6 review row

## Context

Final L4 milestone gate. Runs the mandatory Stage Validation Test Gate end-to-end, confirms CI-L4-001..008, exercises every `crates/daemon/tests/*.rs` integration target, runs `bun run test`, runs `(cd apps/web && bun run test:e2e)`, runs `bun run lint`, and walks the 18-row DoD checklist from spec §4.1.

## Verification Commands

```bash
cargo fmt --check
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test -p daemon --bin daemon
for t in crates/daemon/tests/*.rs; do
  cargo test --test "$(basename "$t" .rs)" || exit 1
done
bun run test
bun run lint
(cd apps/web && bun run test:e2e)
```

## DoD Row Walkthrough (verbatim from spec §4.1)

1. Push protocol: transactional outbox entries (L4-FR-1.3.1, 1.9.2) — `sqlite_supabase_sync.rs`.
2. Push batches applied via single transactional RPC with schema_epoch + row_version ordering (L4-FR-1.3.5, 1.3.6, 1.7.1, 1.9.1) — `sqlite_supabase_sync.rs`.
3. Pull cursor for 3 domains (L4-FR-1.4.1, 1.4.2) — `sqlite_supabase_sync.rs`.
4. Operator decisions through CAS (L4-FR-1.4.4, 1.5.1, 2.3.3) — `sqlite_supabase_sync.rs`.
5. Active-writer lease + writer_epoch fencing (L4-FR-1.6.1, 1.6.2) — `sqlite_supabase_sync.rs`.
6. RLS for all 4 roles (L4-FR-2.5.1..2.5.10) — `redaction_boundary.rs`.
7. Web viewing portal (L4-FR-3.1.1..3.1.8) — `apps/web/e2e/contracts.spec.ts`.
8. Broadcast invalidation (L4-FR-3.2.1..3.2.4) — web E2E.
9. viewer_sync_status freshness (L4-FR-3.3.1..3.3.4, 3.7.1) — web E2E.
10. Validation enforcement + calibration (L4-FR-1.3.9, 3.4.1..3.4.6, 3.6.1..3.6.17) — `sqlite_supabase_sync.rs`.
11. 4 domain validator overlays (L4-FR-3.5.1..3.5.4) — `sqlite_supabase_sync.rs`.
12. Two users view the same pipeline (L4-FR-1.8.1, 3.1.1, 3.3.5) — web E2E + manual smoke.
13. Writer fencing: stale daemon rejected (L4-FR-1.6.2, 1.9.5) — `sqlite_supabase_sync.rs`.
14. CAS command ledger replay (L4-FR-1.5.2, 1.5.3, 1.5.4) — `sqlite_supabase_sync.rs`.
15. Provisional promotion buffer (L4-FR-1.3.9, 3.4.2) — `sqlite_supabase_sync.rs`.
16. Equal-version retry no-op (L4-FR-1.3.7, 1.9.10) — `sqlite_supabase_sync.rs`.
17. Web viewer-scoped credentials (L4-FR-2.5.9, 3.1.1) — `redaction_boundary.rs` + web E2E.
18. Redaction tests pass (L4-FR-2.2.1..2.2.6, 2.5.11) — `redaction_boundary.rs`.

## Tests to Pass (cumulative, all of L4)

- All `CI-L4-001..008`.
- All `L4-T-sqlite_schema-001..024`.
- All `L4-T-sqlite_supabase_sync-001..080`.
- All `L4-T-redaction_boundary-001..026`.
- All `L4-T-contracts_spec-001..020`.
- 158 total checks (8 static gates + 150 runnable tests).

## Acceptance Criteria

- [ ] All 6 phase review-gate beads closed.
- [ ] All 50 task beads closed.
- [ ] Stage Validation Test Gate passes end-to-end with no skips.
- [ ] DoD 1..18 walkthrough completed and logged.
- [ ] Two-user smoke diary produced.

## Dependencies

- **Blocked by:** `l4-p6-main-startup-sync-wiring`, `l4-p6-epoch-repair-generation`, `l4-p6-final-rls-redaction-closure`, `l4-p6-tombstone-retention-proof`, `l4-p6-cross-authority-provenance-and-ttl`, `l4-p6-logical-consistency-auditor-stub`, `l4-p6-two-user-verification`, `l4-p4-review-gate`.
- **Blocks:** none (final L4 gate; closes the layer).
- **Phase:** 6.

---

# Coverage Verification Summary

## Spec Requirement Coverage

All 147 unique L4-FR-* requirement IDs map to at least one bead:

| Range | Owning Phase(s) / Bead(s) |
|-------|---------------------------|
| L4-FR-1.3.1..1.3.8 | P1 (DDL only via sqlite-sync-migration) + P2 (push behavior) |
| L4-FR-1.3.9 | P4 (provisional buffer) |
| L4-FR-1.4.1..1.4.4 | P3 (pull engine + reconciliation) |
| L4-FR-1.5.1..1.5.4 | P3 (CAS + ledger) |
| L4-FR-1.6.1..1.6.3 | P2 (active-writer lease) + P6 (main.rs wiring) |
| L4-FR-1.7.1 | P1 (registry/epoch) + P2 (pause behavior) + P6 (repair convergence) |
| L4-FR-1.8.1..1.8.3 | P2 (status writer + backup) + P3 (staleness TTL) + P5 (web UX) + P6 (E2E) |
| L4-FR-1.9.1..1.9.14 | P2 (push), P3 (pull/CAS), P4 (provisional/enforcement), P6 (RLS final) |
| L4-FR-1.10.1 | P1 (foundation), P4 (validation surfaces), P5 (web) |
| L4-FR-2.1.1..2.1.20 | P1 (DDL via Supabase migration bundle) |
| L4-FR-2.2.1..2.2.6 | P1 (DDL) + P5 (web read) + P6 (redaction final) |
| L4-FR-2.3.1..2.3.3 | P1 (DDL) + P3 (pull behavior) + P6 (RLS final) |
| L4-FR-2.4.1..2.4.3 | P1 (DDL) |
| L4-FR-2.5.1..2.5.11 | P1 (DDL) + P5 (viewer boundary) + P6 (final RLS closure) |
| L4-FR-2.6.1..2.6.11 | P1 (Supabase migration bundle + SQLite migration + schema hash gate) |
| L4-FR-2.7.1..2.7.5 | P6 (cross-authority provenance) |
| L4-FR-3.1.1..3.1.8 | P5 (web portal beads) |
| L4-FR-3.2.1..3.2.4 | P5 (Broadcast client + pages) + P2 (emission point) |
| L4-FR-3.3.1..3.3.5 | P5 (freshness UX) |
| L4-FR-3.4.1..3.4.6 | P4 (enforcement + executor + runner + readiness) |
| L4-FR-3.5.1..3.5.4 | P4 (4 overlay beads) |
| L4-FR-3.6.1..3.6.17 | P4 (calibration math, corpora, override feedback) + P6 (tombstone retention 3.6.17) |
| L4-FR-3.7.1 | P2 (status writer) + P5 (web read) |
| L4-FR-5.2.1 | P6 (epoch repair generation) |
| L4-FR-5.4.1..5.4.3 | P1 (deps + workspace + boundary) |

**Coverage: 147 / 147 (100%).**

## TDD Test ID Ownership

All 158 checks (8 static gates + 150 runnable tests) are owned by exactly one task bead's "Tests to Pass" list. Review-gate beads aggregate phase coverage but do not double-own test IDs.

| File | Test Range | Owning Beads |
|------|-----------|--------------|
| Static gates `CI-L4-001..008` | 8 | P1 (l4-p1-daemon-http-client, l4-p1-root-web-workspace-scripts, l4-p1-supabase-config-and-migration-bundle, l4-p1-test-inventory-and-fixtures), P5 (l4-p5-web-package-config, l4-p5-web-freshness-consumption-and-boundary), P6 (l4-p6-logical-consistency-auditor-stub) |
| `sqlite_schema.rs` 001..024 | 24 | P1 beads (sqlite-sync-migration, sqlite-registry-epoch, supabase-config-and-migration-bundle, root-web-workspace-scripts, test-inventory-and-fixtures), P4 beads (provisional-storage-access, policy-readiness-contract, calibration-math-readiness, calibration-corpora-and-cases, override-feedback-loop), P6 (epoch-repair-generation) |
| `sqlite_supabase_sync.rs` 001..080 | 80 | P2 (8 beads), P3 (8 beads), P4 (12 beads), P6 (5 beads) |
| `redaction_boundary.rs` 001..026 | 26 | P1 (schema-hash-ci-gate for 026), P2 (apply-push-batch-rpc for 023), P3 (cas-rpc for 022 + remote-config-staleness for 024), P5 (freshness-consumption for 009/025), P6 (final-rls-redaction-closure for full sweep + tombstone-retention for 021) |
| `apps/web/e2e/contracts.spec.ts` 001..020 | 20 | P5 (8 beads), P6 (epoch-repair for 019, two-user for 017/018) |

**Coverage: 158 / 158 (100%).**

## Single-Edit Chokepoint Audit

| Chokepoint File | Single Edit Bead | Phase |
|-----------------|------------------|-------|
| `crates/daemon/src/main.rs` | `l4-p6-main-startup-sync-wiring` | 6 |
| `crates/daemon/src/jobs/runner.rs` | `l4-p4-runner-provisional-promotion` | 4 |
| `crates/daemon/src/validation/policy.rs` | `l4-p4-policy-readiness-contract` | 4 |
| `crates/daemon/src/validation/executor.rs` | `l4-p4-executor-overlay-quarantine` | 4 |
| `crates/daemon/src/ipc/handlers/decisions.rs` | `l4-p3-decision-conflict-ipc` | 3 |
| `crates/daemon/Cargo.toml` | `l4-p1-daemon-http-client` | 1 |
| root `package.json` | `l4-p1-root-web-workspace-scripts` | 1 |

All chokepoints have a single edit home; Phase 4 override feedback consumes the Phase 3 handler seam and does not edit `decisions.rs`.

## Bead Totals

- 6 phase sub-epics (l4-schema-foundation, l4-push-engine, l4-pull-cas, l4-validation-enforcement, l4-web-portal, l4-integration-hardening).
- 50 task beads (7+8+8+12+8+7).
- 6 review-gate task beads (l4-p1-review-gate through l4-p6-stage-gate-and-dod-walkthrough).
- **Total descendant rows: 62. Total `layer-4` rows including the parent epic: 63.**

The strategy's "50 task beads + 6 phase sub-epic/review beads" count refers to the 50 implementation task beads plus the 6 review-gate beads; sub-epics are containers for parenting. The live `layer-4` inventory is 63 rows: 1 parent epic, 6 sub-epics, 50 implementation tasks, and 6 review-gate tasks.

## br create / dep / label Sequence

See companion shell script `scripts/l4-create-beads.sh` (generated alongside this manifest). The sequence is:

1. Create 6 sub-epics (`br create --type=epic`).
2. For each sub-epic: `br dep add <sub-epic-id> testproject1-4zq`.
3. Create 50 task beads + 6 review-gate beads (`br create --type=task` with `--description` from this doc).
4. For each task bead: `br label add <id> layer-4` and `br label add <id> l4-phase-{N}`.
5. For each task bead: `br dep add <id> <each-blocking-bead-id>`.
6. `bin/br-sync && git add .beads/ && git commit && git pull --rebase && git push`.

---

# Audit Trail — Sub-Optimization Pass (post-creation)

After bead creation, a 14-step audit (chokepoint discipline / migration uniqueness / overlay completeness / web file ownership / Supabase migration ownership / §1.9 invariants / §4.1 DoD / §3.6 calibration / L4-T claim uniqueness / scope-drift / br lint / parent linkage) was run and the following patches applied to bead descriptions in the live DB. The bulk-import sections above describe initial intent; the bead DB is now authoritative.

## Patches Applied

1. **`l4-p6-cross-authority-provenance-and-ttl` (testproject1-4zq.6.5)** — removed literal `crates/daemon/src/jobs/runner.rs` reference from "Files to Create/Modify" to preserve runner.rs single-edit chokepoint discipline. Provenance recording now extends `crates/daemon/src/db/sync.rs` push-owned write helpers (`OutboxWriter`/promotion path) and stage-write modules; runner.rs remains exclusively edited by `l4-p4-runner-provisional-promotion`.

2. **`l4-p4-overlay-supplier-enrichment`, `l4-p4-overlay-document-acquisition`, `l4-p4-overlay-product-extraction`** — expanded "all 6 files" shorthand to explicit per-file enumeration (SKILL.md, result.schema.json, checkpoint.schema.json, canary.inputs.jsonl, golden.assertions.yaml, validator-rubric.yaml + references/) with `skills/validation/base/SKILL.md` named as the structural template.

3. **`l4-p5-web-contract-list` (testproject1-4zq.5.5)** — added explicit `L4-T-contracts_spec-003` bullet under "Tests to Pass" so the no-mutation-controls test has a non-cumulative claimant.

4. **`l4-p4-calibration-corpora-and-cases` (testproject1-4zq.4.7)** — replaced `L4-T-sqlite_supabase_sync-065..073` range notation in "Tests to Pass" with 9 explicit per-mutation-family bullets.

5. **`l4-p6-final-rls-redaction-closure` (testproject1-4zq.6.3)** — replaced `L4-T-redaction_boundary-001..026` range notation in "Tests to Pass" with 26 explicit per-test bullets covering each role/category and each summary-projection redaction case.

6. **`l4-validation-enforcement` sub-epic (testproject1-4zq.4)** — replaced "deferred to Phase 6 for full proof" with "closed in Phase 6 with full proof; calibration shape is owned here" (eliminates flagged drift word).

7. **All 6 sub-epics** — added "## Success Criteria" section to satisfy `br lint` template gate (alongside existing "## Phase Goal" / "## Gate Criteria" / "## Roll Back If" / "## Spec Coverage" sections).

## Final Audit Status

- **Coverage:** 147/147 L4-FR + 150/150 L4-T + 8/8 CI-L4 (158/158 checks, 100%, post-patch).
- **Chokepoints:** all single-edit (main.rs / runner.rs / policy.rs / executor.rs / Cargo.toml / root package.json / decisions.rs Phase 3 only).
- **§1.9 invariants:** 14/14 covered.
- **§4.1 DoD:** 18/18 enumerated in `l4-p6-stage-gate-and-dod-walkthrough`.
- **§3.6 calibration elements:** 7/7 covered (corpora, 9 mutation families, pairwise, Wilson, rule-of-three, cross-model audit, override feedback).
- **Test ownership:** 158/158 checks have an explicit primary owner in the coverage table; repeated cumulative gate and cross-reference mentions are verification dependencies, not ownership transfers.
- **`br lint`:** all L4 rows pass; global corpus warnings are handled in the step-08 lint cleanup pass.
- **Scope-drift scan:** 4 hits, all confirmed negative-context (forbidding TODO/skip in test code or stub avoidance), false positives.
- **Parent linkage:** all 6 sub-epics have parent-child edge to `testproject1-4zq`; all 56 task/review beads parent under their respective sub-epic.

---

# Audit Trail — Extended Sub-Optimization Pass (post-initial audit)

A second, deeper audit (TDD §11 cross-ref obligations, structural template anchors, silent scope-cut prose, AC/Files-section presence) found 5 categories of additional gaps that the initial audit's substring-based scans missed. Patches:

1. **Web bead structural template anchors** — added "## Structural Template (apps/desktop)" sections to:
   - `l4-p5-web-contract-list` → references `apps/desktop/src/pages/ContractList.tsx`.
   - `l4-p5-web-contract-detail` → references `apps/desktop/src/pages/ContractDetail.tsx`.
   - `l4-p5-web-status-page` → references `apps/desktop/src/pages/DaemonStatus.tsx`.

2. **TDD §11 web cross-ref block** added to 7 P5 web beads + `l4-p6-two-user-verification`. Each now explicitly cites `CI-L4-005`, `CI-L4-006`, and the `packages/ui` import-boundary check per "Beads that touch `apps/web` must include..." rule.

3. **`l4-p4-review-gate` test enumeration** — replaced range notation (`050..080`, `065..073`) with 38 explicit test-ID bullets so TDD §11's mandatory enforcement IDs (003, 022, 023, 024, 050..057, 058..061, 062..064, 065..073, 074..080) are individually claimed in the cumulative aggregator.

4. **`l4-p1-supabase-config-and-migration-bundle` Supabase RLS cross-ref block** — added "## Cross-Reference Tests" section citing `L4-T-redaction_boundary-001..009` (RLS role matrix exercised by the policy file this bead creates) and `L4-T-redaction_boundary-026` (schema hash gate).

## Confirmed False Positives (not patched)

- **Drift scan**: 4 hits in negative-context bans — `l4-p1-test-inventory-and-fixtures` ("No `.skip`, `#[ignore]`, `test.todo`"), both review/walkthrough beads ("cannot be a TODO/empty/doc-only note"), `l4-p6-logical-consistency-auditor-stub` ("NOT TODO/empty"). All forbid the practice rather than allow it.
- **`bd` terminology scan**: 2 hits in `l4-p1-test-inventory-and-fixtures` and `l4-p1-root-web-workspace-scripts` describing CI-L4-008's *purpose* of banning `bd`. Removing the literal `bd` would lose the gate's meaning.

## Final Status (Post-Extended-Audit)

- **Coverage:** 147/147 L4-FR + 150/150 L4-T + 8/8 CI gates (158/158 checks, 100%).
- **Initial 14-step audit:** step-08 fresh-eyes follow-up patched the `decisions.rs` chokepoint call-in and live inventory count.
- **Extended audit:** 0 actionable findings (only false-positive negative-context hits remain).
- **`br lint`:** clean after step-08 lint cleanup.
- **Chokepoint discipline:** `main.rs`, `runner.rs`, `policy.rs`, `executor.rs`, `Cargo.toml`, root `package.json`, and `decisions.rs` each have a single L4 edit home.
- **TDD §11 cross-ref obligations:** satisfied for web (CI-L4-005/006 + packages/ui boundary), validation enforcement (38 IDs), calibration (065-077 + 074/075/076), Supabase schema/RLS (sqlite_schema + redaction_boundary cross-ref).
- **Structural template anchors:** all 3 page-level web beads reference the corresponding `apps/desktop/src/pages/` template.

---

# Audit Trail — Pass-3 Deep-Quality Audit (post-extended-audit)

A third audit pass focused on quality dimensions the substring scans of passes 1-2 couldn't catch: AC text specificity, verbatim spec-phrase fidelity, cross-domain note coverage, calibration threshold numerics, required DDL excerpts, bead-body size distribution.

## Findings & Fixes

1. **S — Calibration sample-count phrases** — `l4-p4-calibration-math-readiness` lacked the literal "≥30 adjudicated" / "≥60 audited" phrases that gate enforcement readiness. Added explicit AC bullets with both narrative phrasing and SQL-friendly forms (`adjudicated_count >= 30`, `audited_approvals_count >= 60`, `wilson_lower_bound_bps >= 7000`, `false_pass_upper_bound_bps <= 500`).

2. **T1 — `operator_decision_commands` DDL** — `l4-p3-command-ledger-retention` contained a lowercase `create table` excerpt; replaced with the verbatim spec §1.5.2 DDL (uppercase `CREATE TABLE`, `command_id TEXT PRIMARY KEY`, comment `-- NULL if conflict`).

3. **T2 — `viewer_sync_status` DDL** — `l4-p2-viewer-sync-status-writer` had no full DDL block. Added the verbatim spec §3.7 DDL with column constraints and inline comments so the writer-path implementer knows the exact remote schema.

4. **W — `L4-FR-5.4.2` (br terminology)** — explicit FR ID was missing from the owning bead `l4-p1-test-inventory-and-fixtures`. Added a verbatim AC stanza calling out: "AGENTS.md contains an older duplicated `bd` block at ~L651-708 that is OUTDATED" and "use `br`/`bin/br-sync` terminology, not `bd`".

## Quality Verifications

- **M (SOT WARNING):** all 62 beads have `## SOURCE OF TRUTH WARNING`.
- **N (phase labels):** all 62 beads have `layer-4` + `l4-phase-N` labels.
- **P (AC quality):** every task bead has specific test-ID-bearing ACs; no bare "all tests pass" without specifics.
- **Q (verbatim spec phrases):** sampled 5 highly-distinctive AC strings; all present verbatim in their owning beads.
- **R (cross-domain notes from spec §5.5):** all 7 (L0 commit_group_id, L0 writer_epoch, L2 FreshnessBadge, L2 pending_operator_intents, L6 safety-critical TTL) covered in the relevant beads.
- **S (threshold numerics):** 12 specific values verified — 90s/300s freshness, 30s/120s lease, 30-day ledger, 14-day tombstone, 4h source TTL, 1500 bps override, 7000 bps Wilson, 500 bps rule-of-three, 30/60 sample counts.
- **T (DDL excerpts):** `operator_decision_commands`, `viewer_sync_status`, `active_writer_lease` shapes all present.
- **U (§5.2 stubs):** 2 implementation stubs (epoch-repair-generation, logical-consistency-auditor) owned; 2 explicit MVP boundaries (schema epoch transitions, multi-org) acknowledged.
- **W (§5.4 prerequisites):** all 3 toolchain prerequisites have explicit bead ownership with FR-ID citation.
- **X (stale terminology):** no raw `bd sync` or `br sync` (without `--flush-only`) anywhere outside negative-context bans.
- **Bead size distribution:** min 1107 / avg 3519 / max 12940 chars. 14 thin beads (<2500 chars) all spot-checked self-contained — they're intentionally focused single-file work (e.g., `l4-p3-pull-cursors` at 1850 chars: SOT + Context + 1-file Files + verbatim L4-FR-1.4.1 + test + 4 ACs + deps).

## Confirmed False Positives (3 audit passes total)

- **Drift scan**: 4 hits in negative-context bans (`No .skip / #[ignore] / test.todo`, `cannot be a TODO/empty/doc-only note`, `NOT TODO/empty`). All forbid the practice; none allow it.
- **`bd` terminology scan**: 5 hits in CI-L4-008's anti-`bd` educational text — explicitly warning agents that AGENTS.md has an outdated `bd` block and L4 must use `br`. Removing the literal `bd` would strip the warning's meaning.

## Final Status (Post-Pass-3)

- **Coverage:** 147/147 L4-FR + 150/150 L4-T + 8/8 CI gates (158/158 checks, 100%).
- **Pass 1 (14-step):** step-08 fresh-eyes follow-up patched the `decisions.rs` chokepoint call-in and live inventory count.
- **Pass 2 (extended TDD §11):** 0 actionable findings.
- **Pass 3 (deep quality):** 0 actionable findings; all 4 categorical fixes (S/T1/T2/W) applied.
- **`br lint`:** clean for all 6 L4 sub-epics.
- **62 beads ready** for `/bead-plan-audit` or `/beads-to-execution`.
