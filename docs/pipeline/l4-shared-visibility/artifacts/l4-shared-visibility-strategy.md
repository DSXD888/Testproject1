# L4 Shared Visibility - Implementation Strategy

## Source Documents

| Document | Path | Role |
|----------|------|------|
| Spec | `docs/strategy/layers/layer-4-shared-visibility.md` | SOT #1 - WHAT to build |
| TDD | `docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-tdd.md` | SOT #2 - HOW to verify |
| Bootstrap | `docs/pipeline/l4-shared-visibility/artifacts/l4-shared-visibility-bootstrap.md` | Harness/bootstrap reference |
| Cross-layer sequencing | `docs/strategy/layer-strategy.md` Sections 6 and 12 | L4 milestone ordering and stack sequencing |
| Cross-reference index | `docs/strategy/cross-reference-index.md` | Navigation aid for L4 cross-domain references |
| L3 predecessor spec | `docs/strategy/layers/layer-3-operational-hardening.md` | Dry-run validation and operational-hardening predecessor context |
| Quality-bar reference | `docs/pipeline/l3-operational-hardening/artifacts/l3-operational-hardening-strategy.md` | Structure, gate specificity, and fan-out benchmark |

## Strategy Guardrails

- Phases are EXECUTION ORDER, not importance tiers. Every phase ships.
- This document defines ordering, dependency edges, phase ownership, fan-out, and gate criteria only. The L4 spec and TDD remain the content authorities.
- The strategy provides ordering only. It does not restate requirements, acceptance criteria, test bodies, schema bodies, or implementation details.
- Later phases receive the same execution depth as earlier phases. Phase number only answers which bead is claimed first.
- Layer 4 stays at `6` execution phases. That is the smallest count that keeps schema, push, pull/CAS, validation enforcement, web, and final integration acyclic.
- `migrations/0012_l4_sync_state.sql`, `infra/supabase/config.toml`, and the eight `infra/supabase/migrations/*.sql` files are the root of the DAG. No sync engine code runs before the schema gate is green.
- `crates/daemon/Cargo.toml` HTTP client work is part of the foundation phase because daemon sync code cannot compile without it.
- Root `package.json` workspace/script work is part of the foundation phase because web beads must be exercised by the Bun gates from their first web commit.
- `crates/daemon/src/main.rs` is the final integration choke point for startup sync sequencing, lease acquisition, and schema-epoch checks. It should not absorb Phase 2-5 feature wiring before those phase-local contracts are gateable.
- `crates/daemon/src/db/sync.rs` is the sync-engine module. Phase 2 owns the push trunk; Phase 3 extends the same module for pull/CAS after the schema gate.
- `crates/daemon/src/jobs/runner.rs` remains the single enqueue/completion boundary. Provisional promotion work routes through that boundary instead of creating a second promotion path.
- `crates/daemon/src/ipc/handlers/decisions.rs` remains the pending-intent writer. CAS flushing and conflict surfacing extend this flow without replacing decision capture.
- `validation/policy.rs` and `validation/executor.rs` are extend-in-place L3 surfaces. Enforcement and overlays do not fork the dry-run validator system.
- Web read-only is a Supabase credential/RLS boundary. UI hiding alone never satisfies a web phase gate.
- T1/T2 gates remain cumulative. T3/T4 and manual two-user verification close milestone behavior; they do not replace earlier phase exits.
- Target bead envelope: `50` task beads plus `6` phase sub-epics/review beads.
- Existing epic `testproject1-4zq` is the parent Layer 4 epic. L4 uses six phase sub-epics beneath it; task beads parent to those phase epics, not directly to the root epic.
- Chokepoint serialization is binding:
  - `crates/daemon/src/main.rs` has one L4 edit home: the Phase 6 startup wiring bead.
  - `crates/daemon/src/jobs/runner.rs` has one L4 edit home: the Phase 4 provisional-promotion bead.
  - `crates/daemon/src/validation/policy.rs` lands before the executor extension inside Phase 4.
  - `crates/daemon/src/validation/executor.rs` lands after the policy/readiness contract, provisional staging table, and `AGENT_SUSPECT_OUTPUT` error classification are stable.
  - root `package.json` has one L4 edit home: the Phase 1 workspace/script bead.
  - `crates/daemon/Cargo.toml` has one L4 edit home: the Phase 1 HTTP-client dependency bead.

## Overview

| Phase | Name | Prerequisites | Gate Type | Target Beads |
|------|------|---------------|-----------|--------------|
| 1 | Schema Foundation | L0-L3 present | schema, dependencies, workspace, inventory | 7 |
| 2 | Push Engine | Phase 1 | SQLite outbox, push RPC, lease, status | 8 |
| 3 | Pull Engine And CAS | Phase 1 | pull cursors, CAS ledger, intent reconciliation | 8 |
| 4 | Validation Enforcement | Phase 1; Phase 2 push writer seam for promotion | enforcement, overlays, calibration | 12 |
| 5 | Web Portal | Phase 1; seeded or pushed projection rows available | read-only Next.js portal, Broadcast, viewer boundary | 8 |
| 6 | Integration And Hardening | Phases 2, 3, 4, and 5 | startup sync integration, RLS closure, DoD walkthrough | 7 |

## Critical Path And Parallelism

- Longest sequential chain: `1 -> 2 -> 4 -> 6`.
- Alternate critical chain: `1 -> 3 -> 6`.
- Web side lane: `1 -> 5 -> 6`, with Phase 5 allowed to start once projection schemas and seeded/readable projection rows exist.
- Critical-path length: `4` gated dependencies including the shared foundation and final convergence.
- Phase `1` is deliberately short and atomic:
  - SQLite migration registration, Supabase migration inventory, daemon HTTP dependency, root web script wiring, and schema-hash gate land before engines.
  - This phase freezes names and migration slots so later phases do not debate DDL order.
- Phase `2` and Phase `3` are parallel after Phase `1`:
  - Phase `2` owns local-to-remote push, outbox, RPC apply, writer lease, and `viewer_sync_status`.
  - Phase `3` owns remote-to-local pull, CAS ledger, pending-intent reconciliation, and conflict events.
- Phase `4` can start after Phase `1` for overlay filesystem and calibration math lanes, but its promotion path waits for the Phase `2` outbox writer seam.
- Phase `5` is a side lane:
  - it depends on Phase `1` schema/workspace truth,
  - it can use seeded projection fixtures before the push dispatcher is complete,
  - it rejoins the sync engines at Phase `6` for two-user milestone verification.
- Phase `6` is the convergence phase:
  - startup ordering in `main.rs`,
  - final RLS/redaction closure,
  - tombstone/provenance/reseed closure,
  - two-user verification,
  - full DoD and stage-gate run.
- Merge-hot surfaces are terminal per phase:
  - `crates/daemon/src/main.rs` only in Phase `6` for L4 startup sync wiring.
  - `crates/daemon/src/jobs/runner.rs` only in Phase `4` for provisional promotion plus Phase `6` capstone assertions.
  - `crates/daemon/src/ipc/handlers/decisions.rs` only in Phase `3` for CAS reconciliation/conflict flow plus Phase `4` override feedback call-in if needed.

## Suggested Bead Fan-Out

| Phase | Task Beads | Epic / Review Beads |
|------|------------|---------------------|
| 1 | 7 | 1 |
| 2 | 8 | 1 |
| 3 | 8 | 1 |
| 4 | 12 | 1 |
| 5 | 8 | 1 |
| 6 | 7 | 1 |

**Expected total:** `50` task beads plus `6` phase sub-epic/review beads.

Phase 6 is intentionally smaller than the push and enforcement phases because it owns convergence, final wiring, and verification, not first-home implementation for a new subsystem. Its gate is not smaller: it runs the complete stage validation block, all L4 daemon integration targets, all redaction tests, web E2E, and the DoD walkthrough.

Recommended issue routing:

- Use bead prefix `l4`.
- Use project key `/Users/alinaaitova/Desktop/Testproject1`.
- Keep existing epic `testproject1-4zq` as the parent Layer 4 epic.
- Create six required phase sub-epics beneath it:
  - `l4-schema-foundation`
  - `l4-push-engine`
  - `l4-pull-cas`
  - `l4-validation-enforcement`
  - `l4-web-portal`
  - `l4-integration-hardening`
- Use `br` and `bin/br-sync` terminology in bead handoffs.
- Use AgentMail reservations per bead on the smallest path set that can satisfy that bead.

## Phase Epic And Task Bead Inventory

These bead names are execution-routing labels, not miniature specifications. Each bead links back to SOT #1 for required behavior and SOT #2 for exact assertions. Beads in the same `Parallel Group` may run concurrently after their listed predecessor bead or phase gate is green. Rows with `review` in `Parallel Group` are the six phase epic/review beads and are not counted in the `50` task-bead total.

| Phase Epic | Bead | Parallel Group | Must Follow | Primary Gate IDs |
|------------|-----------|----------------|-------------|------------------|
| `l4-schema-foundation` | `l4-p1-daemon-http-client` | P1-A | none | `CI-L4-001`, `L4-T-sqlite_schema-024` |
| `l4-schema-foundation` | `l4-p1-sqlite-sync-migration` | P1-B | none | `L4-T-sqlite_schema-001..017` |
| `l4-schema-foundation` | `l4-p1-sqlite-registry-epoch` | P1-B | `l4-p1-sqlite-sync-migration` | `L4-T-sqlite_schema-001`, `012`, `013` |
| `l4-schema-foundation` | `l4-p1-supabase-config-and-migration-bundle` | P1-C | none | `CI-L4-007`, `L4-T-sqlite_schema-018..023` |
| `l4-schema-foundation` | `l4-p1-root-web-workspace-scripts` | P1-D | none | `CI-L4-005`, `CI-L4-008`, `L4-T-sqlite_schema-024` |
| `l4-schema-foundation` | `l4-p1-schema-hash-ci-gate` | P1-E | SQLite + Supabase migration beads | `L4-T-redaction_boundary-026` |
| `l4-schema-foundation` | `l4-p1-test-inventory-and-fixtures` | P1-E | schema hash CI gate | `CI-L4-007`, `CI-L4-008`, `L4-T-sqlite_schema-018`, `024` |
| `l4-schema-foundation` | `l4-p1-review-gate` | review | all Phase 1 task beads | `CI-L4-001`, `003`, `005`, `007`, `008` |
| `l4-push-engine` | `l4-p2-outbox-transaction-writer` | P2-A | Phase 1 | `L4-T-sqlite_supabase_sync-001..003` |
| `l4-push-engine` | `l4-p2-batch-envelope-retry` | P2-A | outbox writer | `L4-T-sqlite_supabase_sync-004..013`, `020`, `021` |
| `l4-push-engine` | `l4-p2-apply-push-batch-rpc` | P2-B | Phase 1 RPC migration slot | `L4-T-sqlite_supabase_sync-014`, `015`, `L4-T-redaction_boundary-023` |
| `l4-push-engine` | `l4-p2-equal-version-semantics` | P2-B | apply RPC | `L4-T-sqlite_supabase_sync-016..019` |
| `l4-push-engine` | `l4-p2-viewer-sync-status-writer` | P2-C | outbox writer | `L4-T-sqlite_supabase_sync-046` |
| `l4-push-engine` | `l4-p2-active-writer-lease` | P2-D | Phase 1 metadata migration | `L4-T-sqlite_supabase_sync-039`, `040`, `042` |
| `l4-push-engine` | `l4-p2-stale-writer-reenvelope` | P2-D | active writer lease | `L4-T-sqlite_supabase_sync-008`, `041` |
| `l4-push-engine` | `l4-p2-schema-epoch-pause-and-backup` | P2-E | batch envelope | `L4-T-sqlite_supabase_sync-044`, `048` |
| `l4-push-engine` | `l4-p2-review-gate` | review | all Phase 2 task beads | `L4-T-sqlite_supabase_sync-001..024`, `039..046`, `048` |
| `l4-pull-cas` | `l4-p3-pull-cursors` | P3-A | Phase 1 | `L4-T-sqlite_supabase_sync-025` |
| `l4-pull-cas` | `l4-p3-hiwater-pagination` | P3-A | pull cursors | `L4-T-sqlite_supabase_sync-026`, `027` |
| `l4-pull-cas` | `l4-p3-pull-before-push-service-contract` | P3-B | pull cursors | `L4-T-sqlite_supabase_sync-028`, `029` |
| `l4-pull-cas` | `l4-p3-cas-rpc` | P3-C | Phase 1 RPC migration slot | `L4-T-sqlite_supabase_sync-030`, `031`, `033`, `L4-T-redaction_boundary-022` |
| `l4-pull-cas` | `l4-p3-command-ledger-retention` | P3-C | CAS RPC | `L4-T-sqlite_supabase_sync-034..038` |
| `l4-pull-cas` | `l4-p3-pending-intent-reconciliation` | P3-D | pull-before-push contract | `L4-T-sqlite_supabase_sync-030..032` |
| `l4-pull-cas` | `l4-p3-decision-conflict-ipc` | P3-D | pending-intent reconciliation | `L4-T-sqlite_supabase_sync-033` |
| `l4-pull-cas` | `l4-p3-remote-config-staleness-ttl` | P3-E | pull cursors | `L4-T-sqlite_supabase_sync-047` |
| `l4-pull-cas` | `l4-p3-review-gate` | review | all Phase 3 task beads | `L4-T-sqlite_supabase_sync-025..038`, `047`, `L4-T-redaction_boundary-021`, `022`, `024` |
| `l4-validation-enforcement` | `l4-p4-provisional-storage-access` | P4-A | Phase 1 | `L4-T-sqlite_schema-005`, `L4-T-sqlite_supabase_sync-003` |
| `l4-validation-enforcement` | `l4-p4-policy-readiness-contract` | P4-B | provisional storage | `L4-T-sqlite_schema-006`, `L4-T-sqlite_supabase_sync-050`, `057` |
| `l4-validation-enforcement` | `l4-p4-error-manifest-classification` | P4-C | policy readiness contract | `L4-T-sqlite_supabase_sync-023`, `080` |
| `l4-validation-enforcement` | `l4-p4-executor-overlay-quarantine` | P4-C | policy readiness contract + Phase 1 provisional staging + error manifest classification | `L4-T-sqlite_supabase_sync-022..024`, `080` |
| `l4-validation-enforcement` | `l4-p4-runner-provisional-promotion` | P4-C | provisional staging + executor overlay/quarantine | `L4-T-sqlite_supabase_sync-022`, `023`, `079` |
| `l4-validation-enforcement` | `l4-p4-calibration-math-readiness` | P4-D | policy readiness contract | `L4-T-sqlite_supabase_sync-050..057`, `075` |
| `l4-validation-enforcement` | `l4-p4-calibration-corpora-and-cases` | P4-D | calibration math | `L4-T-sqlite_supabase_sync-062..074` |
| `l4-validation-enforcement` | `l4-p4-override-feedback-loop` | P4-E | runner provisional promotion | `L4-T-sqlite_schema-009`, `L4-T-sqlite_supabase_sync-077` |
| `l4-validation-enforcement` | `l4-p4-overlay-procurement-ingestion` | P4-F | base validator template | `L4-T-sqlite_supabase_sync-058` |
| `l4-validation-enforcement` | `l4-p4-overlay-supplier-enrichment` | P4-F | base validator template | `L4-T-sqlite_supabase_sync-059` |
| `l4-validation-enforcement` | `l4-p4-overlay-document-acquisition` | P4-F | base validator template | `L4-T-sqlite_supabase_sync-060` |
| `l4-validation-enforcement` | `l4-p4-overlay-product-extraction` | P4-F | base validator template | `L4-T-sqlite_supabase_sync-061` |
| `l4-validation-enforcement` | `l4-p4-review-gate` | review | all Phase 4 task beads | `L4-T-sqlite_schema-005..009`, `017`, `L4-T-sqlite_supabase_sync-003`, `022..024`, `050..080` |
| `l4-web-portal` | `l4-p5-web-package-config` | P5-A | Phase 1 workspace scripts | `CI-L4-005`, `006`, `L4-T-contracts_spec-020` |
| `l4-web-portal` | `l4-p5-web-supabase-client-boundary` | P5-B | web package config | `L4-T-contracts_spec-013..015`, `020` |
| `l4-web-portal` | `l4-p5-web-broadcast-client` | P5-B | Supabase client boundary | `L4-T-contracts_spec-008..010`, `019` |
| `l4-web-portal` | `l4-p5-web-root-layout-and-redirect` | P5-C | web package config | `L4-T-contracts_spec-001`, `002` |
| `l4-web-portal` | `l4-p5-web-contract-list` | P5-C | Supabase client boundary | `L4-T-contracts_spec-001`, `004`, `008` |
| `l4-web-portal` | `l4-p5-web-contract-detail` | P5-C | Supabase client + Broadcast | `L4-T-contracts_spec-009..011` |
| `l4-web-portal` | `l4-p5-web-status-page` | P5-C | Supabase client boundary | `L4-T-contracts_spec-012`, `019` |
| `l4-web-portal` | `l4-p5-web-freshness-consumption-and-boundary` | P5-D | web package config | `L4-T-contracts_spec-005..007`, `016`, `CI-L4-008` |
| `l4-web-portal` | `l4-p5-review-gate` | review | all Phase 5 task beads | `L4-T-contracts_spec-001..020`, `L4-T-redaction_boundary-009`, `025` |
| `l4-integration-hardening` | `l4-p6-main-startup-sync-wiring` | P6-A | Phases 2 + 3 | `L4-T-sqlite_supabase_sync-028`, `039..046` |
| `l4-integration-hardening` | `l4-p6-epoch-repair-generation` | P6-B | Phase 2 epoch pause | `L4-T-sqlite_schema-007`, `L4-T-sqlite_supabase_sync-045`, `L4-T-contracts_spec-019` |
| `l4-integration-hardening` | `l4-p6-final-rls-redaction-closure` | P6-C | Phases 1 + 5 | `L4-T-redaction_boundary-001..026` |
| `l4-integration-hardening` | `l4-p6-tombstone-retention-proof` | P6-D | Phase 3 pull/RLS lanes | `L4-T-sqlite_supabase_sync-078`, `L4-T-redaction_boundary-021` |
| `l4-integration-hardening` | `l4-p6-cross-authority-provenance-and-ttl` | P6-D | Phases 3 + 5 | `L4-T-sqlite_supabase_sync-047`, `049`, `L4-T-contracts_spec-018` |
| `l4-integration-hardening` | `l4-p6-logical-consistency-auditor-stub` | P6-D | epoch repair + provenance | `L4-T-sqlite_supabase_sync-049`, `CI-L4-004` |
| `l4-integration-hardening` | `l4-p6-two-user-verification` | P6-E | Phases 2-5 | `L4-T-contracts_spec-017`, `018` |
| `l4-integration-hardening` | `l4-p6-stage-gate-and-dod-walkthrough` | review | all L4 task beads | `CI-L4-001..008`, DoD `1..18` |

Phase 1's Supabase migration bundle owns all eight files `0001_projection_tables.sql` through `0008_decision_summaries.sql`. Later phases may finish behavior inside RPC/RLS files, but they do not create new migration slots or reorder the migration sequence.

The Phase 5 web beads cover all seven spec-owned web source files: `src/app/layout.tsx`, `src/app/page.tsx`, `src/app/contracts/page.tsx`, `src/app/contracts/[id]/page.tsx`, `src/app/status/page.tsx`, `src/lib/supabase.ts`, and `src/lib/broadcast.ts`.

Phase 5 web beads reuse shared `packages/ui` components (`ContractCardFrame`, `ManifestViewer`, `PipelineBadge`, `FreshnessBadge`, `ErrorSummaryList`) and do not edit `packages/ui` or desktop pages. Phase 5 consumes the shared contract through web-local adapters and fixtures; desktop sync-health compatibility closes in Phase 6.

## Implementation Unit Allocation

### Foundation Surfaces

| Surface | Home Phase | Ordering reason |
|---------|------------|-----------------|
| `migrations/0012_l4_sync_state.sql` | 1 | SQLite sync/enforcement tables are the local root for push, pull, and enforcement |
| `crates/daemon/src/db/sqlite.rs` | 1 | Migration registry and `DEFAULT_SCHEMA_EPOCH` must precede DB module work |
| `infra/supabase/config.toml` | 1 | Supabase local stack configuration must exist before Supabase tests |
| `infra/supabase/migrations/0001_projection_tables.sql` | 1 | Projection table DDL precedes push and web |
| `infra/supabase/migrations/0002_remote_owned_tables.sql` | 1 | Remote-owned domains precede pull and CAS |
| `infra/supabase/migrations/0003_summary_tables.sql` | 1 | Summary projections precede web and redaction tests |
| `infra/supabase/migrations/0004_rls_policies.sql` | 1 + 6 | Policy file enters the schema gate and closes at the final role/category gate |
| `infra/supabase/migrations/0005_sync_rpcs.sql` | 1 + 2 + 3 | RPC migration slot precedes code; push and CAS behavior close in their engine phases |
| `infra/supabase/migrations/0006_schema_metadata.sql` | 1 + 2 | Epoch metadata and writer lease block push dispatch |
| `infra/supabase/migrations/0007_command_ledger.sql` | 1 + 3 | CAS command ledger blocks pending-intent flush |
| `infra/supabase/migrations/0008_decision_summaries.sql` | 1 + 5 | Decision summary projection blocks viewer decision display |
| `crates/daemon/Cargo.toml` | 1 | HTTP client dependency blocks sync RPC client code |
| root `package.json` | 1 | `apps/web` workspace coverage and web scripts block web gates |

### Sync Engine Surfaces

| Surface | Home Phase | Ordering reason |
|---------|------------|-----------------|
| `crates/daemon/src/db/sync.rs` push lane | 2 | Outbox writer and dispatcher are upstream of remote projection visibility |
| `crates/daemon/src/db/sync.rs` pull/CAS lane | 3 | Pull and CAS share schema but do not depend on push dispatcher completion |
| `viewer_sync_status` push/update path | 2 | Web freshness needs push-owned status rows |
| Active writer lease path | 2 | Dispatcher cannot safely push before lease/fencing behavior exists |
| Epoch guard/reseed path | 2 + 6 | Mismatch pause begins with push; full repair convergence closes in Phase 6 |

### Existing Rust Surfaces

| Surface | Home Phase | Ordering reason |
|---------|------------|-----------------|
| `crates/daemon/src/validation/policy.rs` | 4 | Enforcement readiness extends existing dry-run policy decisions |
| `crates/daemon/src/validation/executor.rs` | 4 | Overlay loading and verdict handling extend the validator spawner |
| `crates/daemon/src/validation/manifest_writer.rs` | 4 | Error manifest evidence stays on the existing validation surface |
| `crates/daemon/src/validation/calibration.rs` | 4 | Calibration math and corpus aggregation feed enforcement readiness |
| `crates/daemon/src/validation/feedback.rs` | 4 | Override feedback fan-out belongs with enforcement feedback |
| `crates/daemon/src/jobs/runner.rs` | 4 | Provisional promotion converges through the runner boundary |
| `crates/daemon/src/ipc/handlers/decisions.rs` | 3 + 4 | Pending intent capture extends to CAS reconciliation and override feedback linkage |
| `crates/daemon/src/errors/manifest.rs` | 4 | Enforcement rejection classification is verified on the error-manifest surface |
| `crates/daemon/src/main.rs` | 6 | Startup pull-before-push, lease, and epoch checks land only after engines exist |

### Chokepoint Serialization

| Chokepoint | Edit Home | Serialization rule |
|------------|-----------|--------------------|
| `crates/daemon/src/main.rs` | Phase 6 | Single startup wiring bead; no Phase 2-5 implementation bead edits this file |
| `crates/daemon/src/jobs/runner.rs` | Phase 4 | Single provisional-promotion bead after provisional staging and executor quarantine; Phase 6 tests runner behavior but does not schedule a runner edit bead |
| `crates/daemon/src/validation/policy.rs` | Phase 4 | Policy/readiness bead lands after schema and before executor |
| `crates/daemon/src/validation/executor.rs` | Phase 4 | Executor/overlay bead consumes the policy/readiness contract, Phase 1 provisional staging table, and `AGENT_SUSPECT_OUTPUT` manifest classification |
| `crates/daemon/src/ipc/handlers/decisions.rs` | Phase 3 + serialized Phase 4 call-in | CAS reconciliation lands first; override feedback call-in is serialized after the feedback contract exists |

### Web And Package Surfaces

| Surface | Home Phase | Ordering reason |
|---------|------------|-----------------|
| `apps/web/package.json`, `tsconfig.json`, `next.config.ts`, `playwright.config.ts` | 5 | Web package setup must precede app files and Playwright tests |
| `apps/web/src/app/layout.tsx` | 5 | Root status display is the first web frame |
| `apps/web/src/app/page.tsx` | 5 | Route entry belongs with list page setup |
| `apps/web/src/app/contracts/page.tsx` | 5 | List page owns keyset navigation gate |
| `apps/web/src/app/contracts/[id]/page.tsx` | 5 | Detail page owns projection-detail gate |
| `apps/web/src/app/status/page.tsx` | 5 | Status page owns summary projection display gate |
| `apps/web/src/lib/supabase.ts` | 5 | Supabase client boundary precedes web reads |
| `apps/web/src/lib/broadcast.ts` | 5 | Broadcast invalidation follows client boundary |
| shared `packages/ui` component imports | 5 reuse only + 6 verification | Web consumes shared UI through adapters; Phase 5 does not reserve or edit `packages/ui`, and desktop/web display compatibility closes in the two-user gate |
| `packages/contracts-schema/` | 5 | If absent at implementation start, typed web query boundary lands before web query helpers |

### Skill And Test Surfaces

| Surface | Home Phase | Ordering reason |
|---------|------------|-----------------|
| `skills/validation/overlay-procurement-ingestion/` | 4 | Overlay canaries parallelize after base skill contract is known |
| `skills/validation/overlay-supplier-enrichment/` | 4 | Same overlay lane |
| `skills/validation/overlay-document-acquisition/` | 4 | Same overlay lane |
| `skills/validation/overlay-product-extraction/` | 4 | Same overlay lane |
| `crates/daemon/tests/sqlite_schema.rs` | 1 | Schema and inventory proof starts at the root phase |
| `crates/daemon/tests/sqlite_supabase_sync.rs` | 2 + 3 + 4 + 6 | Sync, CAS, enforcement, and repair gates share the integration surface |
| `crates/daemon/tests/redaction_boundary.rs` | 1 + 5 + 6 | RLS/schema checks start early; viewer boundary closes with web and final RLS |
| `apps/web/e2e/contracts.spec.ts` | 5 + 6 | Web behavior starts in the web phase and closes with two-user integration |

## Coverage Summary

| Coverage Target | Phase Home |
|-----------------|------------|
| `147 / 147` L4 requirement IDs | phases 1-6 |
| `158 / 158` TDD checks | phase gates by test file |
| `14 / 14` L4 invariants | phases 2, 3, 4, and 6 |
| `18 / 18` Definition of Done rows | Phase 6 final DoD walkthrough, with earlier phase gates |
| `17 / 17` §4.3 scenarios | phase gates by domain |
| `4 / 4` new test files | phases 1, 2-4, 5, and 6 |
| `6 / 6` summary projections | phases 1, 5, and 6 |
| `7 / 7` RLS table categories | phases 1 and 6 |
| `4 / 4` validator overlays | Phase 4 |
| `9 / 9` mutation families | Phase 4 |

Requirement range routing:

The L4 spec enumerates `147` unique `L4-FR-*` IDs. The ranges below cover that full enumerated set with no orphan requirement IDs.

| Requirement Range | Primary Phase |
|-------------------|---------------|
| `L4-FR-1.3.1`-`1.3.8` | 2 |
| `L4-FR-1.3.9` | 4 |
| `L4-FR-1.4.1`-`1.4.4` | 3 |
| `L4-FR-1.5.1`-`1.5.4` | 3 |
| `L4-FR-1.6.1`-`1.6.3` | 2 |
| `L4-FR-1.7.1` | 1 + 2 + 6 |
| `L4-FR-1.8.1`-`1.8.3` | 2 + 3 + 5 + 6 |
| `L4-FR-1.9.1`-`1.9.14` | 2 + 3 + 4 + 6 |
| `L4-FR-1.10.1` | 1 + 4 + 5 |
| `L4-FR-2.1.1`-`2.6.11` | 1 + 2 + 3 + 6 |
| `L4-FR-2.7.1`-`2.7.5` | 6 |
| `L4-FR-3.1.1`-`3.3.5` | 5 |
| `L4-FR-3.4.1`-`3.6.16` | 4 |
| `L4-FR-3.6.17` | 6 |
| `L4-FR-3.7.1` | 2 + 5 |
| `L4-FR-5.2.1` | 6 |
| `L4-FR-5.4.1`-`5.4.3` | 1 |

### Definition Of Done Phase Homes

| DoD | First Phase Home | Gate Reference |
|-----|------------------|----------------|
| 1 | 2 | `L4-T-sqlite_supabase_sync-001..003` |
| 2 | 2 | `L4-T-sqlite_supabase_sync-012..019`, `044` |
| 3 | 3 | `L4-T-sqlite_supabase_sync-025..029` |
| 4 | 3 | `L4-T-sqlite_supabase_sync-030..033` |
| 5 | 2 | `L4-T-sqlite_supabase_sync-039..043` |
| 6 | 6 | `L4-T-redaction_boundary-001..009`, `021..026` |
| 7 | 5 | `L4-T-contracts_spec-001..004`, `011`, `012`, `020` |
| 8 | 5 | `L4-T-contracts_spec-008..010`, `019` |
| 9 | 5 | `L4-T-contracts_spec-005..007`, `L4-T-sqlite_supabase_sync-046` |
| 10 | 4 | `L4-T-sqlite_supabase_sync-022..024`, `050..057`, `062..077` |
| 11 | 4 | `L4-T-sqlite_supabase_sync-058..061` |
| 12 | 6 | `L4-T-contracts_spec-017`, `018` plus manual smoke |
| 13 | 2 | `L4-T-sqlite_supabase_sync-041` |
| 14 | 3 | `L4-T-sqlite_supabase_sync-034..038` |
| 15 | 4 | `L4-T-sqlite_supabase_sync-022..024`, `079`, `080` |
| 16 | 2 | `L4-T-sqlite_supabase_sync-016..019` |
| 17 | 5 | `L4-T-redaction_boundary-009`, `025`, `L4-T-contracts_spec-013..015` |
| 18 | 6 | `L4-T-redaction_boundary-010..020` |

### Invariant Phase Homes

| Invariant | First Phase Home | Gate Reference |
|-----------|------------------|----------------|
| `L4-FR-1.9.1` | 2 | `L4-T-sqlite_supabase_sync-014` |
| `L4-FR-1.9.2` | 2 | `L4-T-sqlite_supabase_sync-001`, `002` |
| `L4-FR-1.9.3` | 3 | `L4-T-sqlite_supabase_sync-030`, `031`, `033` |
| `L4-FR-1.9.4` | 3 | `L4-T-sqlite_supabase_sync-034..036` |
| `L4-FR-1.9.5` | 2 | `L4-T-sqlite_supabase_sync-039`, `041`, `042` |
| `L4-FR-1.9.6` | 2 | `L4-T-sqlite_supabase_sync-009`, `044` |
| `L4-FR-1.9.7` | 2 | `L4-T-sqlite_supabase_sync-010` |
| `L4-FR-1.9.8` | 2 | `L4-T-sqlite_supabase_sync-011` |
| `L4-FR-1.9.9` | 2 | `L4-T-sqlite_supabase_sync-004` |
| `L4-FR-1.9.10` | 2 | `L4-T-sqlite_supabase_sync-016` |
| `L4-FR-1.9.11` | 4 | `L4-T-sqlite_supabase_sync-022`, `023`, `079` |
| `L4-FR-1.9.12` | 4 | `L4-T-sqlite_supabase_sync-050`, `053..057` |
| `L4-FR-1.9.13` | 6 | `L4-T-redaction_boundary-001..009` |
| `L4-FR-1.9.14` | 3 | `L4-T-sqlite_supabase_sync-026`, `028` |

### Cross-Layer Phase Homes

| Cross-Layer Note | Phase Home | Bead / Gate |
|------------------|------------|-------------|
| L0 `local_outbox` constitutional columns `commit_group_id` and `writer_epoch` | 1 + 2 | `l4-p1-sqlite-sync-migration`; `l4-p2-outbox-transaction-writer`; `L4-T-sqlite_schema-002`, `015`, `L4-T-sqlite_supabase_sync-001`, `011`, `021` |
| L2 three-state sync health and shared `FreshnessBadge` consumption | 5 + 6 | `l4-p5-web-freshness-consumption-and-boundary`; `l4-p6-two-user-verification`; `L4-T-contracts_spec-005..007`, `017`, `018` |
| L2 pending operator intent flow into remote CAS | 3 | `l4-p3-pending-intent-reconciliation`; `l4-p3-decision-conflict-ipc`; `L4-T-sqlite_supabase_sync-030..033` |
| L6 remote-config safety-critical TTL and upstream trust gate | 3 + 6 | `l4-p3-remote-config-staleness-ttl`; `l4-p6-cross-authority-provenance-and-ttl`; `L4-T-sqlite_supabase_sync-047`, `049` |

## Phase 1: Schema Foundation

**Target beads:** `7`

### Prerequisites

- Layers 0-3 schema/runtime surfaces are present.
- Current flat migration list ends at `migrations/0011_commercial_basis_coverage.sql`.
- `apps/web/` and `infra/supabase/` are absent at L4 start.
- `crates/daemon/src/db/sync.rs`, `validation/calibration.rs`, and `validation/feedback.rs` are absent at L4 start.

### Ordering

- Sequential: `crates/daemon/Cargo.toml` HTTP client dependency -> `migrations/0012_l4_sync_state.sql` -> `crates/daemon/src/db/sqlite.rs` migration registry/epoch.
- Dedicated dependency bead: add the daemon `reqwest` HTTP client before any `crates/daemon/src/db/sync.rs` bead imports or uses an RPC client.
- Dedicated workspace bead: own `apps/web` root workspace coverage, extend `bun run {build,test,lint}` script coverage to include web, and preserve the `packages/ui` lint rule banning desktop/Tauri/daemon/Supabase/transport imports. In the current root package shape, existing `apps/*` coverage can satisfy the workspace-add half, but this single bead still owns the verification and script edits.
- Parallel after SQLite migration name is fixed:
  - Supabase config and migration inventory lane.
  - Root `package.json` web script lane.
  - Test inventory lane for `sqlite_schema.rs`.
- Sequential convergence: schema coordination/hash gate -> inventory gate.
- No `db/sync.rs` engine behavior starts until the Phase 1 gate passes.

### Phase Assets

- `crates/daemon/Cargo.toml`
- `migrations/0012_l4_sync_state.sql`
- `crates/daemon/src/db/sqlite.rs`
- `infra/supabase/config.toml`
- `infra/supabase/migrations/0001_projection_tables.sql` through `0008_decision_summaries.sql`
- root `package.json`
- `crates/daemon/tests/sqlite_schema.rs`
- schema hash assertion surface in `crates/daemon/tests/redaction_boundary.rs`

### Gate Criteria

- Static gates `CI-L4-001`, `CI-L4-003`, `CI-L4-005`, `CI-L4-007`, and `CI-L4-008` are green for foundation scope.
- `L4-T-sqlite_schema-001` through `L4-T-sqlite_schema-024` pass.
- `L4-T-redaction_boundary-026` passes for the logical schema hash gate.
- Command lane: `cargo test -p daemon --test sqlite_schema -- --nocapture` and `cargo test -p daemon --test redaction_boundary -- --nocapture` cover the listed IDs when those files exist.
- The root workspace executes web build/test/lint commands once web files exist; it does not merely include `apps/*`.
- SQLite and Supabase migrations apply from scratch before any Phase 2 or Phase 3 sync code runs.

### Roll Back If

- The L4 SQLite migration cannot be applied through the existing `open_state_db`/`PRAGMA user_version` path.
- Supabase migration ordering requires engine code to exist before DDL can apply cleanly.
- Root scripts cannot include `apps/web` while preserving the `packages/ui` import boundary.

## Phase 2: Push Engine

**Target beads:** `8`

### Prerequisites

- Phase 1 gate passed.
- `reqwest` dependency resolves under `cargo check --all-targets`.
- Supabase projection, summary, metadata, lease, and RPC migration slots apply cleanly.

### Ordering

- Sequential trunk: outbox writer -> batch envelope assembly -> persisted in-flight batch retry -> transactional push RPC behavior -> equal-version semantics -> acknowledgement/status update.
- Sequential lease trunk: active-writer acquisition -> heartbeat -> stale writer rejection -> terminal stale-writer re-envelope path.
- Sequential epoch trunk: batch epoch validation -> pause-on-mismatch -> repair ledger handoff to Phase 6.
- Parallel test lanes:
  - outbox/batch tests,
  - RPC idempotency tests,
  - writer lease tests,
  - `viewer_sync_status` update tests.
- Broadcast emission points can be named here, but end-user Broadcast behavior gates in Phase 5.
- `main.rs` startup wiring waits for Phase 6.

### Phase Assets

- `crates/daemon/src/db/sync.rs` push lane
- push behavior in `infra/supabase/migrations/0005_sync_rpcs.sql`
- lease/epoch behavior in `infra/supabase/migrations/0006_schema_metadata.sql`
- `viewer_sync_status` write path from `infra/supabase/migrations/0003_summary_tables.sql`
- `crates/daemon/tests/sqlite_supabase_sync.rs`
- `crates/daemon/tests/redaction_boundary.rs` direct-upsert denial checks

### Gate Criteria

- Rust gate: `cargo check --all-targets`, `cargo clippy --all-targets -- -D warnings`, and `cargo fmt --check` pass.
- `L4-T-sqlite_supabase_sync-001` through `L4-T-sqlite_supabase_sync-024` pass.
- `L4-T-sqlite_supabase_sync-039` through `L4-T-sqlite_supabase_sync-046` pass, except full epoch-repair reseed closure remains Phase 6.
- `L4-T-sqlite_supabase_sync-048` passes if its backup path is touched in this phase.
- `L4-T-redaction_boundary-023` passes.
- Command lane: `cargo test -p daemon --test sqlite_supabase_sync -- --nocapture` and `cargo test -p daemon --test redaction_boundary -- --nocapture` cover the listed IDs.
- Push-owned projection mutation is only through the batch RPC; direct daemon table upsert paths do not become a supported sync surface.

### Roll Back If

- Outbox writes cannot be made atomic with push-owned data writes.
- Dispatcher retry requires rebuilding envelopes after indeterminate failures.
- Push apply cannot keep single-in-flight, batch-boundary, lease, epoch, and equal-version semantics in one coherent engine lane.

## Phase 3: Pull Engine And CAS

**Target beads:** `8`

### Prerequisites

- Phase 1 gate passed.
- Remote-owned table migration and command-ledger migration apply cleanly.
- `pending_operator_intents` L2 queue shape remains intact.

### Ordering

- Sequential pull trunk: pull cursor domain rows -> hi-water fenced pagination -> tombstone-aware daemon pull selection -> startup/cycle pull-before-push contract.
- Sequential CAS trunk: CAS RPC behavior -> durable command ledger -> command replay/GC behavior.
- Sequential reconciliation trunk: pending intent coalescing -> CAS flush -> conflict status -> `decision.conflict` IPC event.
- Parallel lanes:
  - pull cursor tests,
  - CAS/ledger tests,
  - pending-intent reconciliation tests,
  - RLS direct-update denial tests.
- Phase 2 can run concurrently because both engines depend on Phase 1 schema, not on each other.
- `main.rs` startup ordering waits for Phase 6 after this phase proves the pull/CAS services.

### Phase Assets

- `crates/daemon/src/db/sync.rs` pull/CAS lane
- CAS behavior in `infra/supabase/migrations/0005_sync_rpcs.sql`
- `infra/supabase/migrations/0007_command_ledger.sql`
- `crates/daemon/src/ipc/handlers/decisions.rs`
- `crates/daemon/tests/sqlite_supabase_sync.rs`
- `crates/daemon/tests/redaction_boundary.rs`

### Gate Criteria

- Rust gate: `cargo check --all-targets`, `cargo clippy --all-targets -- -D warnings`, and `cargo fmt --check` pass.
- `L4-T-sqlite_supabase_sync-025` through `L4-T-sqlite_supabase_sync-038` pass.
- `L4-T-sqlite_supabase_sync-047` passes for remote config staleness handling.
- `L4-T-redaction_boundary-021`, `022`, and `024` pass for tombstone visibility and decision mutation boundaries.
- Command lane: `cargo test -p daemon --test sqlite_supabase_sync -- --nocapture` and `cargo test -p daemon --test redaction_boundary -- --nocapture` cover the listed IDs.
- CAS conflict behavior emits `decision.conflict` evidence through the daemon IPC surface referenced by `L4-T-sqlite_supabase_sync-033`.

### Roll Back If

- Pull pagination cannot be fenced by a domain hi-water value.
- Pending intent reconciliation needs to bypass the existing `pending_operator_intents` capture flow.
- CAS replay needs to re-evaluate current decision state instead of returning ledgered results.

## Phase 4: Validation Enforcement

**Target beads:** `12`

### Prerequisites

- Phase 1 gate passed.
- Phase 2 outbox writer/promotable write seam exists for approved provisional outputs.
- L3 validation policy/executor/manifest writer surfaces remain extendable in place.

### Ordering

- Parallel filesystem lane: four validator overlay directories after the base skill template is frozen.
- Sequential enforcement trunk: provisional storage path -> policy readiness read -> `AGENT_SUSPECT_OUTPUT` error manifest classification -> executor overlay loading/quarantine -> approval/rejection handling -> runner promotion convergence.
- Chokepoint order inside the trunk: `validation/policy.rs` readiness contract -> `errors/manifest.rs` rejection classification -> `validation/executor.rs` overlay/quarantine consumption -> one `jobs/runner.rs` provisional-promotion bead.
- Sequential calibration trunk: calibration math -> corpus/case persistence -> readiness recomputation -> demotion triggers -> override feedback.
- Sequential override trunk: decision/override handler call-in -> provisional override promotion -> calibration/incident fan-out.
- Calibration harness is the phase capstone after policy, overlays, and enforcement promotion paths are real.
- Phase 4 can run beside Phase 5 after its Phase 2 promotion dependency is satisfied; it joins Phase 6 for full milestone behavior.

### Phase Assets

- `crates/daemon/src/validation/policy.rs`
- `crates/daemon/src/validation/executor.rs`
- `crates/daemon/src/validation/manifest_writer.rs`
- `crates/daemon/src/validation/calibration.rs`
- `crates/daemon/src/validation/feedback.rs`
- `crates/daemon/src/jobs/runner.rs`
- `crates/daemon/src/ipc/handlers/decisions.rs`
- `crates/daemon/src/errors/manifest.rs`
- four `skills/validation/overlay-*/` directories
- `tests/fixtures/calibration/`
- `crates/daemon/tests/sqlite_supabase_sync.rs`

### Gate Criteria

- Rust gate: `cargo check --all-targets`, `cargo clippy --all-targets -- -D warnings`, and `cargo fmt --check` pass.
- Inventory gate `CI-L4-007` is green for the four overlay directories and calibration artifacts touched by this phase.
- `L4-T-sqlite_schema-005` through `L4-T-sqlite_schema-009` and `L4-T-sqlite_schema-017` pass.
- `L4-T-sqlite_supabase_sync-003`, `022`, `023`, `024`, and `050` through `080` pass.
- Overlay canary/golden gates `L4-T-sqlite_supabase_sync-058` through `061` pass.
- Mutation-family gates `L4-T-sqlite_supabase_sync-065` through `073`, pairwise gate `074`, math gate `075`, cross-model gate `076`, and override feedback gate `077` pass.
- Command lane: `cargo test -p daemon --test sqlite_schema -- --nocapture` and `cargo test -p daemon --test sqlite_supabase_sync -- --nocapture` cover the listed IDs.

### Roll Back If

- Enforcement requires replacing the L3 validator policy/executor instead of extending them.
- Rejected enforcement-active output has already reached push-owned state before validation verdict handling.
- Demotion/readiness state cannot be made durable and queryable through SQLite.

## Phase 5: Web Portal

**Target beads:** `8`

### Prerequisites

- Phase 1 gate passed.
- Supabase projection and summary schemas are queryable by viewer-scoped credentials.
- Seeded or pushed projection rows are available for web tests.
- Root Bun scripts include web build/test/lint execution.

### Ordering

- Sequential package trunk: `apps/web/package.json` -> `tsconfig.json` -> `next.config.ts` -> `playwright.config.ts`.
- Sequential client trunk: `src/lib/supabase.ts` -> `src/lib/broadcast.ts`.
- Sequential route trunk: root layout/page -> contracts list -> contract detail -> status page.
- Parallel test/UX lane:
  - freshness badge web adapter using shared UI,
  - keyset pagination,
  - Broadcast invalidation,
  - viewer credential denial,
  - no privileged browser credentials.
- Phase 5 can start while Phase 2 and Phase 3 are still under construction by using seeded projection fixtures. It joins the real push/pull engines in Phase 6.

### Phase Assets

- `apps/web/package.json`
- `apps/web/tsconfig.json`
- `apps/web/next.config.ts`
- `apps/web/playwright.config.ts`
- seven `apps/web/src/` files from the spec
- `apps/web/e2e/contracts.spec.ts`
- `apps/web/e2e/fixtures/`
- `packages/contracts-schema/` if absent at implementation start

### Gate Criteria

- Bun gates: `bun run test`, `bun run lint`, `(cd apps/web && bun run test)`, and `(cd apps/web && bun run test:e2e)` pass for web scope.
- Static gates `CI-L4-005`, `CI-L4-006`, and `CI-L4-008` are green for web/package scope.
- `L4-T-contracts_spec-001` through `L4-T-contracts_spec-020` pass.
- `L4-T-redaction_boundary-009` and `025` pass for viewer credential boundaries used by the web app.

### Roll Back If

- Browser-held credentials need operator, admin, daemon, or service-role privileges.
- Web data fetching leaks into `packages/ui`.
- Portal behavior depends on local daemon IPC, SQLite, or desktop transport.

## Phase 6: Integration And Hardening

**Target beads:** `7`

### Prerequisites

- Phase 2 gate passed.
- Phase 3 gate passed.
- Phase 4 gate passed.
- Phase 5 gate passed.

### Ordering

- Sequential startup trunk: `main.rs` module/startup wiring -> pull-before-push startup sequence -> active-writer lease acquisition/heartbeat -> schema-epoch status/repair checks.
- Sequential hardening trunk: schema-hash CI re-run -> final RLS policy closure -> redaction boundary sweep -> tombstone retention proof -> cross-authority provenance recording -> safety-critical remote-config TTL confirmation -> logical consistency auditor stub.
- Sequential repair trunk: epoch repair `projection_generation` path -> stale outbox discard gate -> web generation invalidation.
- Sequential milestone trunk: full push-pull round trip -> two-user desktop/web verification protocol -> DoD walkthrough.
- Parallel verification lanes may run after startup wiring:
  - sync round-trip,
  - redaction/RLS,
  - web E2E,
  - manual two-user smoke diary.
- Two-user verification setup is explicit: one desktop operator uses the daemon-mediated read-write control plane against the local SQLite authority; one web viewer signs into `apps/web` with viewer-scoped Supabase credentials. The viewer must see the operator-seeded pipeline within the sync latency exercised by `L4-T-contracts_spec-017` and must show degraded/offline state changes from `L4-T-contracts_spec-018`.
- Two-user pass criteria are binary: the viewer sees the same process list, detail, status, decision summary, and freshness state that the desktop operator just produced; the web session exposes no mutation controls or privileged credentials; `viewer_sync_status` badges match between desktop sync health and web freshness.
- The logical consistency auditor stub is a Phase 6 minimum stub, not an omitted feature. It must expose a testable domain-invariant check entrypoint with conditional auto-repair explicitly disabled unless a safe repair rule is registered; the Phase 6 bead links this to `L4-T-sqlite_supabase_sync-049` provenance assertions or amends the TDD before implementation if a distinct test ID is required.

### Phase Assets

- `crates/daemon/src/main.rs`
- final `crates/daemon/src/db/sync.rs` startup integration
- final RLS policy content in `infra/supabase/migrations/0004_rls_policies.sql`
- `crates/daemon/tests/sqlite_supabase_sync.rs`
- `crates/daemon/tests/redaction_boundary.rs`
- `apps/web/e2e/contracts.spec.ts`
- L4 validation/stage manifest evidence if a manifest is produced

### Gate Criteria

- Mandatory Stage Validation Test Gate passes:
  - `cargo fmt --check`
  - `cargo check --all-targets`
  - `cargo clippy --all-targets -- -D warnings`
  - `cargo test -p daemon --bin daemon`
  - every `crates/daemon/tests/*.rs` target through the integration-test loop
  - `bun run test`
  - `(cd apps/web && bun run test:e2e)`
- Supplemental `bun run lint` passes.
- Static gates `CI-L4-001` through `CI-L4-008` are green.
- `L4-T-sqlite_supabase_sync-025` through `049`, `078`, and all Phase 2/4 sync IDs pass under the full suite.
- `L4-T-redaction_boundary-001` through `026` pass.
- The schema hash CI gate from Phase 1 is re-run in the integrated environment through `L4-T-redaction_boundary-026`.
- `L4-T-contracts_spec-017`, `018`, and `019` pass against integrated push/web behavior.
- `l4-p6-two-user-verification` records a manual smoke diary linked to the `L4-T-contracts_spec-017` and `018` run: desktop operator action, expected web viewer observation, observed sync latency, blocked web mutation attempt, and matching sync-state badges.
- `l4-p6-logical-consistency-auditor-stub` is testable under the full integration loop and cannot be represented by a TODO, empty module, or documentation-only note.
- Definition of Done rows `1` through `18` are verified in one walkthrough.

### Roll Back If

- Startup ordering requires pushing before remote-owned pulls complete.
- Final RLS closure cannot enforce viewer/operator/admin/daemon boundaries at the database/API layer.
- Two-user verification cannot show desktop writer and web viewer observing the same projection through the integrated sync path.

## Dependency Diagram

```text
                 +-------------------+
                 | Phase 1: Schema   |
                 | Foundation        |
                 +---------+---------+
                           |
          +----------------+----------------+
          |                |                |
          v                v                v
+----------------+ +----------------+ +----------------+
| Phase 2: Push  | | Phase 3: Pull  | | Phase 5: Web  |
| Engine         | | Engine + CAS   | | Portal        |
+-------+--------+ +-------+--------+ +-------+--------+
        |                  |                  |
        v                  |                  |
+----------------+         |                  |
| Phase 4:       |         |                  |
| Validation     |         |                  |
| Enforcement    |         |                  |
+-------+--------+         |                  |
        |                  |                  |
        +------------------+------------------+
                           |
                           v
                 +-------------------+
                 | Phase 6:          |
                 | Integration And   |
                 | Hardening         |
                 +-------------------+
```

Internal ordering highlights:

```text
Schema -> outbox writer -> batch envelope -> apply_push_batch -> lease/status -> integration
Schema -> pull cursor -> hi-water pull -> CAS RPC -> command ledger -> reconciliation -> integration
Schema -> provisional buffer -> policy/executor -> overlays -> calibration -> enforcement gate -> integration
Schema -> web package -> Supabase client -> routes -> Broadcast/read-only E2E -> integration
```

## Completeness Mapping

| Spec Section | Phase | Notes |
|-------------|-------|-------|
| §1.1-1.2 | 6 | Narrative/capability closes at milestone verification |
| §1.3.1-1.3.8 | 2 | Push protocol and replay envelope |
| §1.3.9 | 4 | Provisional promotion buffer and enforcement-active write path |
| §1.4 | 3 | Pull protocol and pending-intent reconciliation |
| §1.5 | 3 | CAS and command ledger |
| §1.6 | 2 + 6 | Lease/fencing engine first; startup ownership closes in integration |
| §1.7 | 1 + 2 + 6 | Epoch schema registration, push pause, and repair convergence |
| §1.8 | 2 + 3 + 5 + 6 | Sync health, config staleness, backup, and cross-surface degraded behavior |
| §1.9 | 2 + 3 + 4 + 6 | Invariants map to owning engine and final DoD gate |
| §1.10 | 1 + 4 + 5 | Component inventory homes |
| §2.1-2.4 | 1 + 2 + 5 | Projection/Junction DDL first, push writes and web reads later |
| §2.5 | 1 + 5 + 6 | RLS matrix starts with migration, web verifies viewer boundary, final pass closes all roles |
| §2.6 | 1 + 2 + 3 + 6 | Supabase migration inventory, RPC behavior, metadata, ledger, decision summaries |
| §2.7 | 6 | Cross-authority provenance is final integrated run behavior |
| §3.1 | 5 | Web viewing portal |
| §3.2 | 2 + 5 | Broadcast emission after apply and web invalidation behavior |
| §3.3 | 5 + 6 | Web freshness and desktop integrated sync state |
| §3.4 | 4 | Validation enforcement mode |
| §3.5 | 4 | Four domain overlays |
| §3.6.1-3.6.16 | 4 | Calibration harness and override feedback |
| §3.6.17 | 6 | Tombstone retention closes with pull/RLS integration |
| §3.7 | 2 + 5 | Status projection write and web read |
| §4.1 | 6 | DoD walkthrough, with earlier phase gates as prerequisites |
| §4.2 | 1 + 2 + 5 | Test file homes and inventory |
| §4.3 | 2 + 3 + 4 + 6 | Scenario tests route by owning engine |
| §4.4 | 6 | Release risk closure at final gate |
| §5.1 | 4 + 6 | Calibration quality and RLS/data-protection questions close through enforcement and final security gates |
| §5.2 | 6 | L4 stubs and epoch repair path |
| §5.3 | 6 | Compatibility provisions are verified by final integration boundaries |
| §5.4 | 1 | Dependencies and toolchain prerequisites |
| §5.5 | 6 | ADR closure belongs with the final milestone gate |

**Coverage:** complete. All `147` unique L4 requirement IDs have a phase home, and every TDD test ID range is referenced by at least one phase gate.

## Fresh-Eyes Audit Closure

| Audit Dimension | Closure |
|-----------------|---------|
| Phase coverage | Requirement routing covers all `147` L4-FR IDs; §1.10 components are owned by Phase 1, 4, or 5; §5.4 dependencies land in Phase 1 before dependent engines or web work. |
| Phase coverage equality | Explicit fan-out is `7 / 8 / 8 / 12 / 8 / 7` task beads, matching the L3 order of magnitude without a late-phase junk drawer. |
| Critical dependency ordering | Phase 1 schema/dependency/workspace gate precedes sync and web; Phase 2 outbox precedes push dispatcher; Phase 3 pull cursor precedes CAS reconciliation; Phase 4 orders provisional storage, policy, error classification, executor, and runner promotion before enforcement convergence; Phase 6 `main.rs` startup wiring is last. |
| Chokepoint discipline | `main.rs`, `jobs/runner.rs`, `validation/policy.rs`, `validation/executor.rs`, root `package.json`, and daemon `Cargo.toml` each have one edit home. |
| Parallelism discipline | Phases 2 and 3 parallelize after Phase 1; Phase 5 can start from seeded projection fixtures; Phase 4 overlays parallelize within Phase 4; Phase 6 serializes integration. |
| Gate quality | Every phase gate names TDD IDs and command lanes; Phase 6 uses the full integration-test loop rather than daemon binary tests alone. |
| Phase sub-epic mapping | `testproject1-4zq` remains the parent epic; six required phase sub-epics own the task beads. |
| Calibration placement | The calibration harness is a Phase 4 capstone covering corpora, nine mutation families, pairwise cases, Wilson/rule-of-three math, cross-model cadence, and override feedback. |
| Two-user verification | Phase 6 owns the desktop-operator plus web-viewer manual smoke and links it to `L4-T-contracts_spec-017` and `018`. |
| Schema hash CI gate | Phase 1 owns `l4-p1-schema-hash-ci-gate` and `L4-T-redaction_boundary-026` after both SQLite and Supabase migrations land; Phase 6 re-runs that gate in the integrated environment. |
| Logical consistency auditor stub | Phase 6 owns a testable minimum stub; it cannot be a documentation-only placeholder. |
| Cross-layer touch points | L0 `commit_group_id`/`writer_epoch`, L2 `FreshnessBadge`/desktop health, L2 pending intents, and L6 safety-critical source-config TTL have explicit phase homes. |

Scope drift scan is clean: phases are execution order, every phase ships, and no phase title or gate language weakens L4 requirements.

## Gates Summary

| Phase | Gate | Roll Back If |
|------|------|-------------|
| 1 | `CI-L4-001`, `003`, `005`, `007`, `008`; `L4-T-sqlite_schema-001..024`; `L4-T-redaction_boundary-026` | schema cannot apply through existing migration path, Supabase DDL cannot precede engines, or web scripts break package boundaries |
| 2 | Rust compile/lint/fmt; `L4-T-sqlite_supabase_sync-001..024`, `039..046`, `048`; `L4-T-redaction_boundary-023` | outbox/data atomicity, identical retry, lease fencing, or push RPC atomicity cannot hold |
| 3 | Rust compile/lint/fmt; `L4-T-sqlite_supabase_sync-025..038`, `047`; `L4-T-redaction_boundary-021`, `022`, `024` | pull fencing, CAS ledger replay, or pending-intent reconciliation bypasses the intended authority boundary |
| 4 | Rust compile/lint/fmt; `CI-L4-007`; `L4-T-sqlite_schema-005..009`, `017`; `L4-T-sqlite_supabase_sync-003`, `022..024`, `050..080` | enforcement needs a forked validator path, rejected output is already push-owned, or readiness/demotion is not durable |
| 5 | `bun run test`; `bun run lint`; `(cd apps/web && bun run test)`; `(cd apps/web && bun run test:e2e)`; `CI-L4-005`, `006`, `008`; `L4-T-contracts_spec-001..020`; `L4-T-redaction_boundary-009`, `025` | web requires privileged browser credentials, imports daemon/desktop transport, or moves Supabase access into `packages/ui` |
| 6 | Full Stage Validation Test Gate; `bun run lint`; `CI-L4-001..008`; all L4 daemon integration tests; all redaction tests; all web E2E tests; DoD `1..18` walkthrough | startup pushes before pulls, final RLS cannot enforce role boundaries, or two-user verification fails through the integrated sync path |
