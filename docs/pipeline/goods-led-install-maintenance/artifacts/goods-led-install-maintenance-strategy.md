# Goods-Led Install/Maintenance + UK Labour-Cost Engine - Implementation Strategy

## Source Documents

| Document | Path | Role |
|----------|------|------|
| Spec | `docs/pipeline/goods-led-install-maintenance/artifacts/goods-led-install-maintenance-spec.md` | SOT #1 - WHAT to build |
| TDD | `docs/pipeline/goods-led-install-maintenance/artifacts/goods-led-install-maintenance-tdd.md` | SOT #2 - HOW to verify |
| Research | `docs/research/2026-06-01-goods-led-install-maintenance-pipeline-research.md` | Dependency and sequence reference, especially sections 9 and 11 |
| Manifest | `docs/pipeline/goods-led-install-maintenance/manifest.yaml` | Recorded scope tiers, migration numbers, and handoff constants |
| Quality-bar reference | `docs/pipeline/l6-request-pipeline-automation/artifacts/l6-request-pipeline-automation-strategy.md` | Phase depth, gate specificity, and coverage-map benchmark |

## Strategy Guardrails

- Phases are execution order, not importance tiers. Every phase ships.
- The strategy routes work only. The spec owns feature content; the TDD owns test assertions.
- Tier A is committed scope: MVP plus recurring maintenance, access-equipment auto-selection, quarterly rate refresh, and `overlay-labour-cost`.
- Tier B is bounded in-scope research-spike scope: `FR-Z.1`, `FR-Z.2`, and `FR-Z.3` are tracked in Phase 8 with research/design gates.
- The qualification expansion and labour-cost engine are coupled by ADR-001. The Phase-2 dominance gate does not go live before labour and permit/TM rollup fields exist.
- The schema epoch bump is a foundation task and must update all four local constants plus Supabase metadata in one phase.
- Additive migrations must preserve legacy direct `INSERT` tests through nullable/default-0 behavior.
- Historical rows are not forcibly backfilled or recomputed for MVP. Additive labour/permit/maintenance columns default to `NULL` or `0`; only new Stage 11 assessments, or explicitly rerun assessments in a later approved scope, receive labour and goods-share values.
- Recurring maintenance is split by dependency: Phase 1 creates the first-class `MaintenancePeriodEstimate` schema/type surface, Phase 2 freezes maintenance-aware skill I/O and warning-code hooks, and Phase 5 owns persisted annualized maintenance behavior.
- New low-reg goods categories enter production only after their own gold-set validation slice is green.
- Bead-tracked execution uses `bin/br-claim <bead-id>` and `bin/br-complete <bead-id>`, never raw status updates.
- Bead content comes from the spec and TDD. This strategy supplies routing only.

## Pre-Flight

| Check | Required Outcome |
|-------|------------------|
| Branch | Work is on `main` |
| Agent Mail | Session identity, peer handshakes, and narrow file reservations exist before bead-tracked edits |
| Existing docs | Spec, TDD, architecture, bootstrap, manifest, and this strategy path are present |
| Scope tiers | Manifest Tier A/Tier B rules are accepted as recorded 2026-06-02 decisions |
| Migration numbering | SQLite migration is `0014_installation_labour_fields.sql`; Supabase migration is `0010_*.sql` |
| Schema epoch | Epoch moves 9 to 10 across daemon, three desktop constants, and Supabase metadata |
| Browser UI gate | `/agent-browser` is used before any rendered UI verification |

## Overview

| Phase | Name | Prerequisites | Gate Type | Target Beads |
|------|------|---------------|-----------|--------------|
| 1 | Foundation Schema, Types, And Epoch | Existing L5 schema and contracts surfaces | schema/type compile, epoch consistency, nullability | 10 |
| 2 | Labour Reference Data And Worker Skill | Phase 1 type surface | skill lint, worker contract, canaries, warning coverage | 12 |
| 3 | Landed-Cost Rollup And Live Loop | Phases 1-2 | domestic/import rollup, transient skill invocation, decision hash | 9 |
| 4 | Qualification And Goods-Share Gates | Phase 3 labour rollup | TypeScript qualification, service gate, coverage math, dominance synthesis | 11 |
| 5 | Recurring Maintenance Closure | Phases 1-4 | maintenance entity behavior, annualized opex, capital-event dominance | 8 |
| 6 | Operator Surfacing | Phases 3-5 | manifest rows, review-card metrics, rendered UI proof | 8 |
| 7 | Validation, Overlay, Gold Set, And Rate Refresh | Phases 1-6 | full corpus, overlay canary, rate refresh, L0 category gate, full stage gate | 13 |
| 8 | Tier-B Research Spikes | Phase 7 scope baseline | `SPIKE-Z-*` research/design outputs | 3 |

## Critical Path And Parallelism

- Root critical path: `1 -> 2 -> 3 -> 4 -> 5 -> 6 -> 7 -> 8`.
- Phase 1 is the root because migrations, projection fields, contracts-schema types, and schema epoch must exist before consumers can compile.
- Phase 2 depends on Phase 1 because the labour skill input/output contracts consume the new installation, landed-cost, and maintenance shapes.
- Phase 3 depends on Phase 2 because `run_landed_cost_live_loop` must invoke a real worker contract and roll up real labour/permit outputs.
- Phase 4 depends on Phase 3 because the Phase-2 dominance gate reads persisted labour, permit/TM, and goods-only subtotal fields. This is the ADR-001 "two halves ship together" edge.
- Phase 5 depends on Phase 4 because recurring maintenance must preserve the capital-event dominance route while adding annualized maintenance economics. It also consumes the Phase 1 `MaintenancePeriodEstimate` schema/type surface and the Phase 2 maintenance-aware worker hooks; if Phase 5 needs to invent new worker I/O, Phase 2 was incomplete.
- Phase 6 depends on Phases 3-5 because UI rows and review cards consume persisted labour/permit, goods-share, no-list, and maintenance signals.
- Phase 7 depends on all Tier-A behavior because the gold-set and overlay gates are the production promotion boundary.
- Phase 8 depends on Phase 7 because the research spikes design against the shipped Tier-A extension points.
- Phase 2 has two parallel lanes after schema/types: reference JSON files and worker metadata/schema files.
- Phase 4 has two parallel lanes after Phase 3: early TypeScript qualification/service-gate work and coverage-rollup/decision-synthesis work, but production activation waits for both.
- Phase 6 has parallel desktop and web/read-only surfacing lanes after shared row/type surfaces are stable.
- Merge-hot surfaces are serialized:
  - Schema epoch constants and migrations: Phase 1 only.
  - Labour skill worker, reference files, and maintenance-aware I/O hooks: Phase 2 only.
  - `crates/daemon/src/main.rs` rollup/live-loop changes: Phase 3 only.
  - `packages/intelligence` qualification and coverage changes: Phase 4 only.
  - `MaintenancePeriodEstimate` persistence behavior: Phase 5 only after the Phase 1 schema/type surface exists.
  - UI/shared package surfacing: Phase 6 only.
  - L0 whitelist and validation fixture promotion: Phase 7 only.

## Follow-Up Dependency Audit

| Dependency Edge | Created In | First Consumer | Blocking Gate |
|-----------------|------------|----------------|---------------|
| `LandedCostEstimateRow` labour, permit/TM, and goods-only subtotal fields | Phase 1 | Phase 3 rollup persistence, Phase 4 dominance synthesis, Phase 6 surfacing | `TS-SCHEMA-002`, `RS-SCHEMA-001`, `RS-SCHEMA-002` |
| `PipelineAssessmentRow` goods-share and labour/permit coverage fields | Phase 1 | Phase 4 coverage rollup, Phase 6 review cards | `TS-SCHEMA-003`, `RS-SCHEMA-003`, `TS-COV-007..009` |
| `ProductCandidateRow` installation-requirement shape | Phase 1 | Phase 2 worker input, Phase 3 skip/invoke decision | `TS-SCHEMA-001`, `E2E-IL-008` |
| `MaintenancePeriodEstimateRow` type surface | Phase 1 | Phase 5 recurring-maintenance behavior | `TS-SCHEMA-005`, `E2E-IL-004` |
| Labour skill plus four skill-local reference JSON files | Phase 2 | Phase 3 live-loop invocation | `SKL-LINT-001..005`, `CAN-LAB-001..007` |
| Maintenance-aware `InstallationRequirement`, `maintenance_schedule`, and `MAINTENANCE_TERM_ASSUMED` hooks | Phase 2 | Phase 5 recurring-maintenance behavior | `SKL-LINT-004`, `CAN-WARN-001`, `E2E-IL-004` |
| Labour/permit rollup fields | Phase 3 | Phase 4 Phase-2 goods-dominance gate | `RS-ROLLUP-001..005`, `SMOKE-LIVE-001`, `TS-COV-007..009` |
| `installation_labour_engine.rs` harness seed helper | Phase 1 | Phase 3 Scenario 1/8 rollup gates, Phase 4 qualification gates, Phase 5 maintenance gate, Phase 7 full corpus closure | `RS-SEED-001`, `E2E-IL-001..011`, `GOLD-L0-001` |
| Null/default historical-row semantics and no forced backfill | Phase 1 | Phase 3 hash/rollup reads, Phase 4 goods-share reads, Phase 6/7 surfacing and validation | `RS-SCHEMA-001`, `RS-SCHEMA-002`, `RS-HASH-003` |

No consumer above depends on a field or worker artifact created in a later phase.

## Suggested Bead Fan-Out

| Phase | Task Beads | Epic / Review Beads |
|------|------------|---------------------|
| 1 | 10 | 1 |
| 2 | 12 | 1 |
| 3 | 9 | 1 |
| 4 | 11 | 1 |
| 5 | 8 | 1 |
| 6 | 8 | 1 |
| 7 | 13 | 1 |
| 8 | 3 | 1 |

Expected total: `74` task beads plus `8` phase sub-epic/review beads.

Recommended phase sub-epics:

- `gim-p1-foundation-schema-types-epoch`
- `gim-p2-labour-reference-worker-skill`
- `gim-p3-rollup-live-loop`
- `gim-p4-qualification-goods-share`
- `gim-p5-recurring-maintenance`
- `gim-p6-operator-surfacing`
- `gim-p7-validation-overlay-goldset-refresh`
- `gim-p8-tier-b-research-spikes`

## Phase Bead Routing Batches

Rows are routing labels, not miniature specifications. Beads in the same parallel group may run concurrently after the listed predecessor or phase gate is green.

| Phase | Routing Batches | Must Follow | Primary Gate IDs |
|-------|-----------------|-------------|------------------|
| 1 | SQLite migration; Supabase projection; schema epoch constants; contracts-schema rows; installation requirement type; maintenance row/table type; legacy insert guard; `installation_labour_engine.rs` seed helper | Existing L5 schema | `TS-SCHEMA-001..005`, `RS-SCHEMA-001..003`, `RS-SEED-001`, `CI-GIM-001..007` |
| 2 | skill-local reference JSON; worker frontmatter; worker result schema; I/O contract; maintenance-aware input/warning hooks; profile resolver; permits/TM resolver; access-equipment resolver; fail-soft warnings; canary corpus | Phase 1 | `SKL-LINT-001..005`, `CAN-LAB-001..007`, `CAN-WARN-001`, `GOLD-L0-003`, `CI-GIM-001..007` |
| 3 | domestic rollup; import rollup; Scenario 1/8 E2E additions; labour job kind; live-loop transient load; run counters/session IDs; goods-only subtotal persistence; decision-basis hash | Phases 1-2 | `RS-ROLLUP-001..005`, `SMOKE-LIVE-001`, `RS-HASH-001..003`, `E2E-IL-001`, `E2E-IL-008`, `CI-GIM-001..007` |
| 4 | Phase-1 provisional admit; separable-mixed downgrade; service-gate split; no-list/bespoke flag; coverage rollup; Phase-2 dominance gate; basis caveat logging | Phase 3 | `TS-QUAL-003a`, `TS-QUAL-003b`, `TS-QUAL-006..008`, `TS-DQ-001..008`, `TS-COV-007..009`, `NEG-CTRL-001..003`, `E2E-IL-005..007`, `E2E-IL-010`, `E2E-IL-011`, `CI-GIM-001..007` |
| 5 | maintenance type closure; maintenance requirement extraction route; visit/term modelling; annualized opex; capital-event denominator guard; warning-code fixture; Scenario 4 E2E; surfacing handoff | Phase 4 | `E2E-IL-004`, `TS-SCHEMA-005`, `CAN-WARN-001`, `CI-GIM-001..007` |
| 6 | shared surfacing data shape; manifest labour row; manifest permit/TM row; warning/exclusion text; contract-card metric; review reason list; desktop queue proof; web read-only proof | Phases 3-5 | `UI-AGB-001..003`, `E2E-IL-001`, `E2E-IL-011`, `TS-COV-009`, `CI-GIM-008` |
| 7 | Appendix-A fixture completion; `installation_labour_engine.rs` full corpus closure; integrated Scenario 1 hard band rerun; negative/regression controls; labour canary fixture corpus; overlay skill; overlay outage/rejection proof; rate-refresh skill; rate-refresh canary; L0 whitelist; per-category gold slices; full mixed-stack gate; phase review | Phases 1-6 | `E2E-IL-001..011`, `GOLD-L0-001..003`, `CAN-OVERLAY-001`, `SKL-RATE-REFRESH-001..003`, `CAN-RATE-REFRESH-001`, `CI-GIM-001..008` |
| 8 | live enrichment research/design; installer-quote research/design; permit/zone research/design | Phase 7 | `SPIKE-Z-001`, `SPIKE-Z-002`, `SPIKE-Z-003` |

## Implementation Unit Allocation

| Surface Group | Home Phase | Ordering Reason |
|---------------|------------|-----------------|
| SQLite additive migration, maintenance table/type, and direct-INSERT compatibility | 1 | Consumers must compile and legacy tests must keep passing before behavior lands |
| Supabase projection migration and metadata epoch | 1 | Desktop/web projection reads depend on the same schema epoch |
| Four local schema-epoch constants | 1 | Splitting the epoch bump is a known read-breaker |
| `packages/contracts-schema/src/index.ts` row extensions | 1 | TypeScript consumers depend on these shapes |
| `InstallationRequirement` and `MaintenancePeriodEstimate` type surfaces | 1 | Labour skill, rollup, and UI consume these structures later |
| `installation_labour_engine.rs` seed helper | 1 | Phase 3/4/5 E2E gates must not consume a test harness first created in Phase 7 |
| Labour skill reference JSON files | 2 | The worker cannot run without versioned local data |
| `skills/landed-cost/uk-installation-labour-estimate/` | 2 | Worker contract, maintenance-aware hooks, Scenario 1 canary band, and canaries close before daemon live-loop invocation |
| Daemon landed-cost builders and persistence | 3 | Labour/permit fields must be summed and persisted before decision synthesis |
| `run_landed_cost_live_loop` labour invocation | 3 | ADR-002 requires transient `load_skill` invocation after FX and before synthesis |
| Decision-basis hash extension | 3 | Hash must see labour/permit basis before downstream bid readiness consumes it |
| TypeScript qualification/service gate | 4 | Early admission and service rejection are package-local, but final activation waits for rollup |
| Coverage rollup and Phase-2 goods dominance | 4 | The dominance denominator consumes Phase 3 labour and permit/TM fields |
| Recurring maintenance annualized economics | 5 | Capital-event dominance must already be stable |
| Manifest viewer and contract card | 6 | Operator surfaces consume final row fields and review reasons |
| Overlay, rate refresh, gold set, and L0 doc | 7 | Production category go-live is gated by validated corpus and overlay proof |
| Tier-B extension-point design | 8 | Research spikes use the shipped Tier-A structures as their baseline |

## Phase 1: Foundation Schema, Types, And Epoch

**Target beads:** `10`

### Prerequisites

- Existing L5 landed-cost schema and projection files are present.
- No overlapping reservation exists on migration, schema-epoch, or contracts-schema surfaces.

### Requirement IDs Covered

- Primary: `FR-R.1`, `FR-R.5`, `FR-R.6`.
- Foundation support for `FR-L.1`, `FR-L.8`, `FR-R.2`, `FR-R.3`, `FR-R.4`, `FR-R.7`, `FR-U.1`, and `FR-U.2`.

### Test IDs Covered

- `TS-SCHEMA-001` through `TS-SCHEMA-005`.
- `RS-SCHEMA-001`, `RS-SCHEMA-002`, `RS-SCHEMA-003`, `RS-SEED-001`.
- Static gates `CI-GIM-001` through `CI-GIM-007`.

### Ordering

- Parallel: SQLite migration, Supabase projection migration, contracts-schema row extensions, install/maintenance type surface, and `installation_labour_engine.rs` seed-helper scaffold.
- Sequential: SQLite migration -> legacy direct-INSERT assertion -> schema epoch 9 to 10 constants -> Supabase metadata epoch.
- Sequential: SQLite migration -> `MaintenancePeriodEstimate` persisted structure -> contracts-schema `MaintenancePeriodEstimateRow` -> Phase 5 behavior readiness.
- Sequential: `installation_labour_engine.rs` seed-helper scaffold -> `RS-SEED-001` -> Phase 3/4/5 scenario assertions.
- Sequential: contracts-schema rows -> package build/type checks -> downstream TypeScript consumer readiness.
- Sequential: projection migration -> desktop/web projection-read readiness.

### Gate Criteria

- `migrations/0014_installation_labour_fields.sql` is present and recorded by `RS-SCHEMA-002`.
- New direct-INSERT-sensitive fields are nullable or `DEFAULT 0`; `RS-SCHEMA-001` and `RS-SCHEMA-002` stay green.
- The same additive migration creates the first-class `MaintenancePeriodEstimate` persisted structure or equivalent table; Phase 5 must not need a second schema-epoch bump to write recurring-maintenance rows.
- Supabase `infra/supabase/migrations/0010_*.sql` projects landed-cost and pipeline-assessment additions; `RS-SCHEMA-003` is green.
- Schema epoch is `10` in daemon, all three desktop constants, Supabase metadata, and the `customs_fx_freight_engine.rs` user-version assertion.
- `TS-SCHEMA-001..005` prove the contracts-schema row surface, including the Tier-B `installer_quote` extension point and recurring-maintenance row type.
- `RS-SEED-001` proves `installation_labour_engine.rs` has the explicit-column `landed_cost_insert_sql()` seed helper before any later phase consumes `E2E-IL-*` IDs.
- The MVP backfill decision is encoded: historical rows remain `NULL`/`0`, no forced recompute sweep is scheduled, and new labour/goods-share values appear only on new or explicitly rerun assessments.
- `CI-GIM-001..007` pass for the touched schema/type surfaces.

### Roll Back If

- Epoch 10 cannot be represented consistently across daemon, desktop, and Supabase.
- Legacy direct `landed_cost_estimates` inserts require test rewrites instead of nullable/default-safe migration behavior.
- Contracts-schema consumers need incompatible duplicate row definitions.
- Recurring-maintenance persistence requires a second migration phase or untyped JSON storage.
- Phase 3/4/5 scenario gates would need to create their own E2E harness instead of consuming the Phase 1 seed helper.

## Phase 2: Labour Reference Data And Worker Skill

**Target beads:** `12`

### Prerequisites

- Phase 1 gate passed.
- Contracts-schema and migration fields are available to describe labour inputs and outputs.

### Requirement IDs Covered

- Primary: `FR-L.1`, `FR-L.2`, `FR-L.3`, `FR-L.4`, `FR-L.5`, `FR-L.6`, `FR-L.7`, `FR-L.9`, `FR-L.12`.

### Test IDs Covered

- `SKL-LINT-001` through `SKL-LINT-005`.
- `CAN-LAB-001` through `CAN-LAB-007`.
- `CAN-WARN-001`, `GOLD-L0-003`.
- Supporting `E2E-IL-001`, `E2E-IL-008`, `E2E-IL-009`, `E2E-IL-010`, `E2E-IL-011`.

### Ordering

- Parallel: four skill-local reference JSON files and worker metadata/schema files.
- Sequential: reference JSON -> installation profile resolver -> one-off install/de-install -> access-equipment resolver -> permits/TM resolver -> maintenance-aware input/warning hooks -> warning-code coverage.
- Sequential: worker frontmatter -> result schema -> canary inputs -> golden assertions -> skill lint.
- The worker remains `http_public`; browser/CDP membership is not part of this route.

### Gate Criteria

- `SKL-LINT-001..005` prove frontmatter, required sections, warning-code inventory, `http_public`, and no CDP allowlist membership.
- `CAN-LAB-001..007` prove the pinned canary cases: `lights-nw-2day-01`, `lights-lon-2day-01`, `cctv-yh-install-01`, `furniture-lon-assembly-01`, `region-unknown-01`, `scope-unresolved-01`, and `rate-stale-01`.
- `CAN-LAB-001` is the Phase 2 hard engine-acceptance gate for Scenario 1: `lights-nw-2day-01` must assert `210000 <= installation_labour_cost_gbp_pence <= 265000` before Phase 3 may invoke the labour worker from the live loop.
- The worker input/result schema accepts the Phase 1 `InstallationRequirement` maintenance fields (`requires_maintenance`, `maintenance_schedule`) and enumerates `MAINTENANCE_TERM_ASSUMED`; Phase 5 consumes these hooks rather than inventing new worker I/O.
- `CAN-WARN-001` proves every warning code has a pinned canary or E2E assertion.
- `GOLD-L0-003` proves required `validator-rubric.yaml` files are stable-production blockers.
- `CI-GIM-001..007` pass for skill-lint and Rust/TypeScript surfaces touched by this phase.

### Roll Back If

- The worker needs CDP membership or direct browser automation.
- Reference data cannot remain skill-local without adding daemon reference-loader infrastructure.
- Warning coverage cannot be made deterministic through canaries or E2E manifests.
- Scenario 1's labour band cannot be asserted at the worker canary boundary before daemon rollup.
- Phase 5 requires new worker contract fields that were not frozen in Phase 2.

## Phase 3: Landed-Cost Rollup And Live Loop

**Target beads:** `9`

### Prerequisites

- Phase 1 gate passed.
- Phase 2 worker contract and canary corpus are stable.

### Requirement IDs Covered

- Primary: `FR-R.2`, `FR-R.3`, `FR-R.7`.

### Test IDs Covered

- `RS-ROLLUP-001` through `RS-ROLLUP-005`.
- `SMOKE-LIVE-001`.
- `RS-HASH-001`, `RS-HASH-002`, `RS-HASH-003`.
- Supporting integrated checks `E2E-IL-001` and `E2E-IL-008`.

### Ordering

- Sequential: labour result parse path -> domestic rollup -> import rollup -> persistence -> decision-basis hash.
- Sequential: transient `load_skill` -> job kind `landed_cost_uk_installation_labour` -> live-loop sequencing -> run counters/session IDs.
- The import flow lands after FX and before import landed-cost synthesis.
- The domestic flow lands after supplier-cost basis and before domestic landed-cost synthesis.
- No `registry.rs` route exists for this skill.

### Gate Criteria

- `RS-ROLLUP-001..005` prove labour/permit additions at both rollup sites, low/high band behavior, pure-goods unchanged behavior, and saturating integer-pence arithmetic.
- `SMOKE-LIVE-001` proves transient worker load, job ordering, and no browser/CDP dependency.
- `RS-HASH-001..003` prove labour and permit/TM fields participate in decision-basis hashing and null/default historical rows hash deterministically.
- `E2E-IL-001` and `E2E-IL-008` prove non-zero install and pure-goods regression behavior on real SQLite state.
- `CI-GIM-001..007` pass.

### Roll Back If

- Labour invocation cannot be inserted without changing landed-cost skill registry semantics.
- Goods-only subtotal cannot be reconstructed or persisted for the Phase-2 denominator.
- Rollup changes require non-saturating arithmetic or floating-point money.

## Phase 4: Qualification And Goods-Share Gates

**Target beads:** `11`

### Prerequisites

- Phase 3 gate passed.
- Labour, permit/TM, and goods-only subtotal fields are persisted and readable.

### Requirement IDs Covered

- Primary: `FR-Q.1`, `FR-Q.2`, `FR-Q.3`, `FR-Q.4`, `FR-Q.5`, `FR-Q.6`, `FR-R.4`.

### Test IDs Covered

- `TS-QUAL-003a`, `TS-QUAL-003b`, `TS-QUAL-006`, `TS-QUAL-007`, `TS-QUAL-008`.
- `TS-DQ-001` through `TS-DQ-008`.
- `TS-COV-007`, `TS-COV-008`, `TS-COV-009`.
- `NEG-CTRL-001`, `NEG-CTRL-002`, `NEG-CTRL-003`.
- `E2E-IL-005`, `E2E-IL-006`, `E2E-IL-007`, `E2E-IL-010`, `E2E-IL-011`.

### Ordering

- Parallel after Phase 3: TypeScript qualification/service-gate lane and coverage-rollup/decision-synthesis lane.
- Sequential in qualification lane: `TS-QUAL-003` split -> Phase-1 provisional-admit branch -> no-list/bespoke flag -> basis caveat logging.
- Sequential in service-gate lane: truth table -> `goods_led_mixed` union variant -> pure-service negative controls.
- Sequential in coverage lane: goods-share bps -> coverage bps -> Phase-2 dominance synthesis.
- Production activation waits for both early admission and labour-backed dominance synthesis.

### Gate Criteria

- `TS-QUAL-003a`, `TS-QUAL-003b`, and `TS-QUAL-008` prove separable mixed cannot pass on density alone and prefers contract-value share when present.
- `TS-QUAL-006` proves Phase-1 provisional admission routes credible goods-bearing mixed/works notices to review.
- `TS-QUAL-007` and `E2E-IL-011` prove no-list/bespoke review behavior.
- `TS-DQ-001..008` and `NEG-CTRL-001..003` prove the service-gate split rejects pure services while preserving goods-led mixed descriptions.
- `TS-COV-007..009` prove goods-share denominator math, permit-heavy cases, and conservative labour coverage behavior.
- `E2E-IL-006` and `E2E-IL-010` prove labour-dominant rejection happens at the Phase-2 dominance gate.
- ADR-001 is closed only when Phase 3 and Phase 4 gates are both green; the Phase-2 filter must not ship ahead of the engine.
- `CI-GIM-001..007` pass.

### Roll Back If

- The service-gate split admits pure-service negative controls.
- Cost-proxy near-floor cases auto-reject instead of routing to review.
- The Phase-2 dominance gate cannot read persisted labour/permit fields from Phase 3.

## Phase 5: Recurring Maintenance Closure

**Target beads:** `8`

### Prerequisites

- Phase 4 gate passed.
- The Phase 1 maintenance row/table/type surface is available.
- The Phase 2 maintenance-aware worker I/O and `MAINTENANCE_TERM_ASSUMED` warning hook are frozen.

### Requirement IDs Covered

- Primary: `FR-L.8`.

### Test IDs Covered

- `E2E-IL-004`.
- `TS-SCHEMA-005`.
- `CAN-WARN-001`.

### Ordering

- Sequential: Phase 1 maintenance table/type consumption -> requirement extraction route -> visit/term modelling -> annualized opex -> capital-event denominator guard -> warning-code fixture -> Scenario 4 E2E -> surfacing handoff.
- Maintenance work consumes the Phase 4 dominance gate; it does not reopen Phase 4 qualification logic.
- This phase is a full modelling phase, not a validation tail. It owns recurring-maintenance behavior after Phase 1 creates the row/table/type surface and Phase 2 freezes the worker hooks.

### Gate Criteria

- `TS-SCHEMA-005` confirms recurring maintenance has a first-class row/type surface.
- `E2E-IL-004` proves recurring maintenance rows, annualized opex, visible-term opex, and capital-event-only dominance.
- `CAN-WARN-001` proves `MAINTENANCE_TERM_ASSUMED` is pinned in warning-code coverage.
- Phase 5 cannot close until recurring maintenance has a separate route for annualized economics and a separate route for capital-event dominance protection.
- Phase 5 must consume the Phase 1 persisted structure and Phase 2 worker hooks; it cannot close by hiding recurring-maintenance fields in generic JSON or by reopening the labour worker contract.
- `CI-GIM-001..007` pass.

### Roll Back If

- Multi-year maintenance has to be capitalized into the goods-dominance denominator.
- Recurring maintenance fields can only be hidden inside untyped JSON.
- Annualized margin cannot be surfaced separately from the capital-event landed-cost row.
- The annualized maintenance path requires a new migration or worker I/O that should have been created in Phase 1 or Phase 2.

## Phase 6: Operator Surfacing

**Target beads:** `8`

### Prerequisites

- Phase 3 rollup fields are persisted.
- Phase 4 review reasons and goods-share basis are stable.
- Phase 5 maintenance signals are readable for review context.

### Requirement IDs Covered

- Primary: `FR-U.1`, `FR-U.2`.

### Test IDs Covered

- `UI-AGB-001`, `UI-AGB-002`, `UI-AGB-003`.
- Supporting `E2E-IL-001`, `E2E-IL-011`, `TS-COV-009`.
- UI/browser gate `CI-GIM-008`.

### Ordering

- Sequential shared lane: row/type data -> manifest cost table -> review reason payloads -> web read-only shape.
- Parallel after shared lane: manifest labour row, manifest permit/TM row, contract-card metrics, desktop queue proof, and web read-only projection.
- Browser verification runs through `/agent-browser`, not shell-only smoke.

### Gate Criteria

- `UI-AGB-001` proves the Scenario 1 manifest exposes the `Installation labour` row.
- `UI-AGB-002` proves a separate `Permits & traffic management` row plus warning, basis, floor, and route context.
- `UI-AGB-003` proves contract-card labour coverage and review reasons, with web read-only remaining non-mutating.
- `E2E-IL-001`, `E2E-IL-011`, and `TS-COV-009` supply seeded review cases for rendered UI proof.
- `CI-GIM-008` passes with retained `/agent-browser` evidence.

### Roll Back If

- UI proof depends on hidden fields rather than projected/readable row data.
- Review contracts cannot remain visible in the desktop queue.
- Web read-only views require mutation affordances to display the new context.

## Phase 7: Validation, Overlay, Gold Set, And Rate Refresh

**Target beads:** `13`

### Prerequisites

- Phase 1 through Phase 6 gates passed.
- All Tier-A behavior has a runnable test surface.

### Requirement IDs Covered

- Primary: `FR-L.10`, `FR-L.11`, `FR-V.1`, `FR-V.2`, `FR-V.3`, `FR-V.4`, `FR-V.5`.

### Test IDs Covered

- `E2E-IL-001` through `E2E-IL-011`.
- `GOLD-L0-001`, `GOLD-L0-002`, `GOLD-L0-003`.
- `CAN-LAB-001` through `CAN-LAB-007`, `CAN-WARN-001`, `CAN-OVERLAY-001`.
- `SKL-RATE-REFRESH-001`, `SKL-RATE-REFRESH-002`, `SKL-RATE-REFRESH-003`, `CAN-RATE-REFRESH-001`.
- Full mixed-stack gates `CI-GIM-001` through `CI-GIM-008`.

### Ordering

- Sequential: Appendix-A corpus completion -> `installation_labour_engine.rs` full-corpus closure -> integrated Scenario 1 hard-band rerun -> negative/regression controls -> gold-set gate -> L0 whitelist doc -> category go-live check.
- Parallel after corpus shape: overlay lane and rate-refresh lane.
- Overlay lane: overlay contract -> canary/golden assertions -> outage/rejection review behavior.
- Rate-refresh lane: source-health skill contract -> stale-rate canary -> candidate JSON/provenance output.
- Final full gate sweep runs after overlay and rate-refresh close.

### Gate Criteria

- `E2E-IL-001..011` prove all Appendix-A deterministic manifests on real SQLite state.
- Phase 7 re-runs the Scenario 1 `210000..=265000` labour band as integrated SQLite proof; Phase 2 remains the worker-canary owner of the same hard band.
- `GOLD-L0-001` proves the 11-scenario corpus gate.
- `GOLD-L0-002` proves the L0 whitelist expansion, metal split, no-list exclusions, and labelled category slices.
- `GOLD-L0-003` proves missing labour/overlay rubric files block stable production.
- A new goods category enters production only after its labelled gold-set slice is green under the L0 rule.
- `CAN-OVERLAY-001` proves `overlay-labour-cost` lints and routes rejection/outage to review.
- `SKL-RATE-REFRESH-001..003` and `CAN-RATE-REFRESH-001` prove the quarterly refresh skill contract and no in-place production mutation.
- `CI-GIM-001..008` pass, including every daemon integration target and Bun tests.

### Roll Back If

- Any Appendix-A scenario lacks a runnable `E2E-IL-*` manifest.
- Category promotion can happen without a labelled gold-set slice.
- Overlay or rate-refresh behavior requires live per-estimate scraping.

## Phase 8: Tier-B Research Spikes

**Target beads:** `3`

### Prerequisites

- Phase 7 gate passed.
- Tier-A extension points are shipped and documented.

### Requirement IDs Covered

- Primary: `FR-Z.1`, `FR-Z.2`, `FR-Z.3`.

### Test IDs Covered

- `SPIKE-Z-001`, `SPIKE-Z-002`, `SPIKE-Z-003`.

### Ordering

- Parallel: live per-estimate enrichment lane research/design, installer-quote ingestion research/design, detailed permit/zone sub-model research/design.
- Sequential within each spike: research question -> extension-point design -> decision record -> successor test IDs or documented non-delivery decision.
- Each `FR-Z.*` item owns one combined research-spike bead covering research investigation, extension-point design, and decision record.

### Gate Criteria

- `SPIKE-Z-001` produces research output and extension-point design for operator-triggered live per-estimate enrichment.
- `SPIKE-Z-002` produces research output and extension-point design for `quote_class = installer_quote`.
- `SPIKE-Z-003` produces research output and extension-point design for detailed per-authority/per-zone permit and lane-rental modelling.
- No Phase 8 bead claims full behavior without replacing `SPIKE-Z-*` coverage with real requirement IDs and named code tests.
- Phase 8 gates are research/design gates only; they are not code-test gates.

### Roll Back If

- A spike starts coding full behavior before the research/design output is accepted.
- The proposed design requires changing Tier-A row semantics without a new spec/TDD update.
- A spike silently drops its source `FR-Z.*` requirement.

## Invariant Enforcement Phase Homes

| Invariant | Enforcement Phase | Mechanism Home |
|-----------|-------------------|----------------|
| Schema epoch 9 to 10 is atomic | 1 | Daemon constant, three desktop constants, Supabase metadata, and schema assertion |
| Migration nullability/default safety | 1 | SQLite migration plus `RS-SCHEMA-001` and `RS-SCHEMA-002` |
| No forced historical backfill | 1 | Additive `NULL`/`0` columns, `RS-HASH-003`, and explicit new-assessments-only MVP note |
| `installation_labour_engine.rs` exists before scenario consumers | 1 + 3 | Phase 1 seed helper; Phase 3 first Scenario 1/8 E2E assertions |
| Skill-local reference data | 2 | Worker `reference/*.json` and `SKL-LINT-005` |
| Scenario 1 hard labour band | 2 + 7 | `CAN-LAB-001` worker canary first; `E2E-IL-001` integrated SQLite rerun at corpus closure |
| Labour skill is not registry-bound | 2 + 3 | Worker metadata in Phase 2; live-loop transient load in Phase 3 |
| Two halves ship together | 3 + 4 | Labour/permit rollup in Phase 3; dominance gate activation in Phase 4 |
| Cost proxy is not ground truth | 4 | Contract-value preference, near-floor review, decision basis logging |
| Maintenance is not capitalized into dominance | 1 + 2 + 5 | Phase 1 row/table/type surface, Phase 2 maintenance hooks, Phase 5 capital-event dominance plus separate annualized maintenance row |
| Review visibility | 6 | Manifest/review card rows and desktop queue proof |
| L0 category expansion rule | 7 | Labelled gold-set slices before production go-live |
| Tier-B bounded scope | 8 | `SPIKE-Z-*` research/design gates only |

## TDD Phase Homes

| TDD Surface | Phase Home | Notes |
|-------------|------------|-------|
| `TS-SCHEMA-001..005` | 1 | Contracts-schema and row/type surface |
| `RS-SCHEMA-001..003`, `RS-SEED-001` | 1 | Migration, epoch, Supabase projection, and seeding helper |
| `SKL-LINT-001..005` | 2 | Labour worker metadata and file inventory |
| `CAN-LAB-001..007`, `CAN-WARN-001` | 2 + 7 | Worker canaries in Phase 2; integrated corpus closure in Phase 7 |
| `RS-ROLLUP-001..005` | 3 | Domestic/import rollup and integer-pence arithmetic |
| `SMOKE-LIVE-001` | 3 | Live loop transient skill load and job ordering |
| `RS-HASH-001..003` | 3 | Decision-basis hash participation |
| `E2E-IL-001`, `E2E-IL-008` | 3 | First real-SQLite rollup scenarios; full corpus still closes in Phase 7 |
| `TS-QUAL-003a`, `TS-QUAL-003b`, `TS-QUAL-006..008` | 4 | Qualification branch and basis behavior |
| `TS-DQ-001..008` | 4 | Description-quality truth table |
| `TS-COV-007..009` | 4 | Goods-share and coverage rollup |
| `NEG-CTRL-001..003` | 4 | Pure-service negative controls |
| `E2E-IL-005..007`, `E2E-IL-010`, `E2E-IL-011` | 4 | Service-gate, dominance-floor, and no-list integrated controls |
| `E2E-IL-004` | 5 | Recurring maintenance closure |
| `UI-AGB-001..003` | 6 | Rendered operator surfaces |
| `E2E-IL-001..011` | 7 | Full Appendix-A deterministic corpus |
| `GOLD-L0-001..003` | 7 | Gold-set and L0 whitelist/category gate |
| `CAN-OVERLAY-001` | 7 | Validation overlay |
| `SKL-RATE-REFRESH-001..003`, `CAN-RATE-REFRESH-001` | 7 | Quarterly rate refresh |
| `CI-GIM-001..008` | 1 through 7 | Full mixed-stack quality gate, with UI/browser proof in Phase 6/7 |
| `SPIKE-Z-001..003` | 8 | Tier-B research-spike deliverables |

## Test File Phase Homes

| Test File Or Evidence | First Home | Closure Phase |
|-----------------------|------------|---------------|
| `packages/contracts-schema/src/index.ts` type/build coverage | 1 | 7 |
| `crates/daemon/tests/customs_fx_freight_engine.rs` schema assertion preservation | 1 | 7 |
| `crates/daemon/tests/installation_labour_engine.rs` seed helper and explicit-column insert helper | 1 | 7 |
| `crates/daemon/tests/skill_lint.rs` labour additions | 2 | 7 |
| `crates/daemon/tests/skill_canary_harness.rs` labour canaries and warning coverage | 2 | 7 |
| daemon bin rollup tests in touched rollup modules | 3 | 7 |
| `crates/daemon/tests/decision_basis_hash.rs` labour/permit hash additions | 3 | 7 |
| `crates/daemon/tests/installation_labour_engine.rs` Scenario 1 and Scenario 8 assertions | 3 | 7 |
| `packages/intelligence/src/qualification/goods-qualification.test.ts` | 4 | 7 |
| `packages/intelligence/src/customs/description-quality.test.ts` | 4 | 7 |
| `packages/intelligence/src/margins/coverage-rollup.test.ts` | 4 | 7 |
| `crates/daemon/tests/installation_labour_engine.rs` Scenario 4 recurring-maintenance assertions | 5 | 7 |
| `/agent-browser` desktop/web evidence for manifest and contract-card rendering | 6 | 7 |
| `skills/validation/overlay-labour-cost/` canary/golden evidence | 7 | 7 |
| `skills/source-health/uk-installation-labour-rate-refresh/` lint/canary evidence | 7 | 7 |
| `SPIKE-Z-*` research-spike decision records | 8 | 8 |

## Literal TDD ID Routing

| Phase | Literal Test IDs |
|-------|------------------|
| 1 | `TS-SCHEMA-001`, `TS-SCHEMA-002`, `TS-SCHEMA-003`, `TS-SCHEMA-004`, `TS-SCHEMA-005`, `RS-SCHEMA-001`, `RS-SCHEMA-002`, `RS-SCHEMA-003`, `RS-SEED-001`, `CI-GIM-001`, `CI-GIM-002`, `CI-GIM-003`, `CI-GIM-004`, `CI-GIM-005`, `CI-GIM-006`, `CI-GIM-007` |
| 2 | `SKL-LINT-001`, `SKL-LINT-002`, `SKL-LINT-003`, `SKL-LINT-004`, `SKL-LINT-005`, `CAN-LAB-001`, `CAN-LAB-002`, `CAN-LAB-003`, `CAN-LAB-004`, `CAN-LAB-005`, `CAN-LAB-006`, `CAN-LAB-007`, `CAN-WARN-001`, `GOLD-L0-003` |
| 3 | `RS-ROLLUP-001`, `RS-ROLLUP-002`, `RS-ROLLUP-003`, `RS-ROLLUP-004`, `RS-ROLLUP-005`, `SMOKE-LIVE-001`, `RS-HASH-001`, `RS-HASH-002`, `RS-HASH-003`, `E2E-IL-001`, `E2E-IL-008` |
| 4 | `TS-QUAL-003a`, `TS-QUAL-003b`, `TS-QUAL-006`, `TS-QUAL-007`, `TS-QUAL-008`, `TS-DQ-001`, `TS-DQ-002`, `TS-DQ-003`, `TS-DQ-004`, `TS-DQ-005`, `TS-DQ-006`, `TS-DQ-007`, `TS-DQ-008`, `TS-COV-007`, `TS-COV-008`, `TS-COV-009`, `NEG-CTRL-001`, `NEG-CTRL-002`, `NEG-CTRL-003`, `E2E-IL-005`, `E2E-IL-006`, `E2E-IL-007`, `E2E-IL-010`, `E2E-IL-011` |
| 5 | `E2E-IL-004`, `TS-SCHEMA-005`, `CAN-WARN-001` |
| 6 | `UI-AGB-001`, `UI-AGB-002`, `UI-AGB-003`, `E2E-IL-001`, `E2E-IL-011`, `TS-COV-009`, `CI-GIM-008` |
| 7 | `E2E-IL-001`, `E2E-IL-002`, `E2E-IL-003`, `E2E-IL-004`, `E2E-IL-005`, `E2E-IL-006`, `E2E-IL-007`, `E2E-IL-008`, `E2E-IL-009`, `E2E-IL-010`, `E2E-IL-011`, `GOLD-L0-001`, `GOLD-L0-002`, `GOLD-L0-003`, `CAN-OVERLAY-001`, `SKL-RATE-REFRESH-001`, `SKL-RATE-REFRESH-002`, `SKL-RATE-REFRESH-003`, `CAN-RATE-REFRESH-001`, `CI-GIM-001`, `CI-GIM-002`, `CI-GIM-003`, `CI-GIM-004`, `CI-GIM-005`, `CI-GIM-006`, `CI-GIM-007`, `CI-GIM-008` |
| 8 | `SPIKE-Z-001`, `SPIKE-Z-002`, `SPIKE-Z-003` |

## Requirement Routing

Each functional requirement below has exactly one primary phase home. Supporting prerequisites may appear earlier, but feature ownership lives in this table.

| Requirement IDs | Primary Phase |
|-----------------|---------------|
| `FR-R.1`, `FR-R.5`, `FR-R.6` | 1 |
| `FR-L.1`, `FR-L.2`, `FR-L.3`, `FR-L.4`, `FR-L.5`, `FR-L.6`, `FR-L.7`, `FR-L.9`, `FR-L.12` | 2 |
| `FR-R.2`, `FR-R.3`, `FR-R.7` | 3 |
| `FR-Q.1`, `FR-Q.2`, `FR-Q.3`, `FR-Q.4`, `FR-Q.5`, `FR-Q.6`, `FR-R.4` | 4 |
| `FR-L.8` | 5 |
| `FR-U.1`, `FR-U.2` | 6 |
| `FR-L.10`, `FR-L.11`, `FR-V.1`, `FR-V.2`, `FR-V.3`, `FR-V.4`, `FR-V.5` | 7 |
| `FR-Z.1`, `FR-Z.2`, `FR-Z.3` | 8 |

## Literal Requirement ID Routing

| Requirement | Tier | Primary Phase | Gate IDs |
|-------------|------|---------------|----------|
| `FR-Q.1` | A | 4 | `TS-QUAL-006`, `E2E-IL-001`, `E2E-IL-003`, `E2E-IL-009`, `E2E-IL-010`, `E2E-IL-011` |
| `FR-Q.2` | A | 4 | `TS-QUAL-003a`, `TS-QUAL-003b`, `TS-QUAL-008` |
| `FR-Q.3` | A | 4 | `TS-COV-007`, `TS-COV-008`, `E2E-IL-001`, `E2E-IL-006`, `E2E-IL-010`, `E2E-IL-011` |
| `FR-Q.4` | A | 4 | `TS-DQ-001..008`, `E2E-IL-005`, `E2E-IL-007` |
| `FR-Q.5` | A | 4 | `TS-QUAL-007`, `E2E-IL-011`, `UI-AGB-003` |
| `FR-Q.6` | A | 4 | `TS-COV-007`, `TS-COV-008`, `RS-HASH-001`, `RS-HASH-002`, `E2E-IL-001`, `E2E-IL-010` |
| `FR-L.1` | A | 2 | `E2E-IL-001`, `E2E-IL-004`, `E2E-IL-008`, `CAN-LAB-005`, `TS-SCHEMA-001` |
| `FR-L.2` | A | 2 | `SKL-LINT-001..005`, `SMOKE-LIVE-001` |
| `FR-L.3` | A | 2 | `CAN-LAB-001..007`, `TS-SCHEMA-002` |
| `FR-L.4` | A | 2 | `CAN-LAB-007`, `SKL-LINT-005`, `SMOKE-LIVE-001`, `GOLD-L0-003` |
| `FR-L.5` | A | 2 | `CAN-LAB-001`, `CAN-LAB-003`, `E2E-IL-001`, `E2E-IL-009`, `E2E-IL-010`, `E2E-IL-011`, `CAN-OVERLAY-001` |
| `FR-L.6` | A | 2 | `E2E-IL-001`, `CAN-LAB-001`, `RS-ROLLUP-002` |
| `FR-L.7` | A | 2 | `E2E-IL-001`, `E2E-IL-010`, `CAN-LAB-001`, `CAN-LAB-002`, `UI-AGB-002` |
| `FR-L.8` | A | 5 | `E2E-IL-004`, `TS-SCHEMA-005`, `CAN-WARN-001` |
| `FR-L.9` | A | 2 | `CAN-LAB-001`, `CAN-LAB-003`, `CAN-LAB-004`, `E2E-IL-001`, `E2E-IL-009` |
| `FR-L.10` | A | 7 | `CAN-LAB-007`, `SKL-RATE-REFRESH-001..003`, `CAN-RATE-REFRESH-001` |
| `FR-L.11` | A | 7 | `CAN-OVERLAY-001`, `CAN-LAB-006`, `SKL-LINT-005` |
| `FR-L.12` | A | 2 | `CAN-WARN-001`, `CAN-LAB-005`, `CAN-LAB-006`, `CAN-LAB-007`, `E2E-IL-004` |
| `FR-R.1` | A | 1 | `RS-SCHEMA-001`, `RS-SCHEMA-002`, `RS-ROLLUP-004`, `TS-SCHEMA-002`, `E2E-IL-008` |
| `FR-R.2` | A | 3 | `RS-ROLLUP-001..005`, `E2E-IL-001`, `E2E-IL-008` |
| `FR-R.3` | A | 3 | `SMOKE-LIVE-001`, `SKL-LINT-002`, `E2E-IL-001`, `E2E-IL-008` |
| `FR-R.4` | A | 4 | `TS-COV-007`, `TS-COV-008`, `TS-COV-009`, `TS-SCHEMA-003`, `E2E-IL-001`, `E2E-IL-006`, `E2E-IL-010` |
| `FR-R.5` | A | 1 | `TS-SCHEMA-001..005` |
| `FR-R.6` | A | 1 | `RS-SCHEMA-001`, `RS-SCHEMA-002`, `RS-SCHEMA-003`, `RS-SEED-001`, `CI-GIM-001..005` |
| `FR-R.7` | A | 3 | `RS-HASH-001`, `RS-HASH-002`, `RS-HASH-003` |
| `FR-U.1` | A | 6 | `UI-AGB-001`, `UI-AGB-002`, `E2E-IL-001`, `CAN-WARN-001` |
| `FR-U.2` | A | 6 | `UI-AGB-003`, `TS-COV-009`, `E2E-IL-001`, `E2E-IL-011` |
| `FR-V.1` | A | 7 | `E2E-IL-001..011`, `GOLD-L0-001` |
| `FR-V.2` | A | 7 | `E2E-IL-001..011` |
| `FR-V.3` | A | 7 | `CAN-LAB-001..007`, `CAN-WARN-001`, `GOLD-L0-003` |
| `FR-V.4` | A | 7 | `GOLD-L0-001`, `GOLD-L0-002`, `E2E-IL-003`, `E2E-IL-008`, `E2E-IL-011` |
| `FR-V.5` | A | 7 | `CI-GIM-001..008` |
| `FR-Z.1` | B | 8 | `SPIKE-Z-001` |
| `FR-Z.2` | B | 8 | `SPIKE-Z-002` |
| `FR-Z.3` | B | 8 | `SPIKE-Z-003` |

## Completeness Mapping

| Spec Section | Phase | Notes |
|-------------|-------|-------|
| §1.1 Mission | 3 + 4 + 7 | Coupled capability closes through rollup, dominance gate, and full validation |
| §1.2 Recorded Operator Decisions | 1 + 4 + 7 | Category, threshold, labour scope, and estimate-grade decisions route by owning surface |
| §1.3 Operator-Confirmable Defaults | 2 + 4 + 5 + 6 | Defaults route to worker, qualification, maintenance, and surfacing homes |
| §1.4 Tier Rules | 7 + 8 | Tier A closes by Phase 7; Tier B closes as Phase 8 research spikes |
| §1.5 Non-Goals | 2 + 4 + 7 | Enforced by worker contract, service/no-list gates, and validation |
| §2.1 Qualification And Service Gates | 4 | `FR-Q.1..FR-Q.6` |
| §2.2 Labour-Cost Engine | 2 + 5 + 7 | Worker in Phase 2, recurring maintenance in Phase 5, overlay/rate refresh in Phase 7 |
| §2.3 Rollup, Schema, Sync, And Decision Synthesis | 1 + 3 + 4 | Schema/types first, rollup/hash second, coverage/dominance third |
| §2.4 UI And Operator Surfacing | 6 | `FR-U.1..FR-U.2` |
| §2.5 Validation, Gold Set, And Quality Gates | 7 | `FR-V.1..FR-V.5` |
| §3 User Stories | 1 through 8 | Routed through owning functional requirements |
| §4 E2E Test Scenarios | 7 | Full corpus closure; individual scenarios support earlier phase gates |
| §5 Dependencies | 1 through 7 | Consumed by phase prerequisites |
| §6 Risks And Mitigations | 2 + 4 + 6 + 7 | Risk controls route to worker, gates, UI, and validation |
| §7 Success-Criteria Coverage Matrix | 1 through 7 | Covered by TDD phase homes |
| §8 Tier B Bounded Future Scope | 8 | `FR-Z.1..FR-Z.3` research-spike phase |

Coverage: complete. All `35` functional requirement IDs have exactly one primary phase home, all Tier-A requirements have runnable gates, and all `3` Tier-B requirements have tracked research-spike gates.

## Dependency Diagram

```text
+---------------------------------------------+
| Phase 1: Foundation Schema, Types, Epoch     |
+----------------------+----------------------+
                       |
                       v
+---------------------------------------------+
| Phase 2: Labour Reference Data And Skill     |
+----------------------+----------------------+
                       |
                       v
+---------------------------------------------+
| Phase 3: Rollup And Live Loop                |
| labour + permits persisted before gate       |
+----------------------+----------------------+
                       |
                       v
+---------------------------------------------+
| Phase 4: Qualification And Goods-Share Gates |
| ADR-001 two-halves-ship-together closure     |
+----------------------+----------------------+
                       |
                       v
+---------------------------------------------+
| Phase 5: Recurring Maintenance Closure       |
+----------------------+----------------------+
                       |
                       v
+---------------------------------------------+
| Phase 6: Operator Surfacing                  |
+----------------------+----------------------+
                       |
                       v
+---------------------------------------------+
| Phase 7: Validation, Overlay, Gold Set,      |
| Rate Refresh, Full Stage Gate                |
+----------------------+----------------------+
                       |
                       v
+---------------------------------------------+
| Phase 8: Tier-B Research Spikes              |
+---------------------------------------------+
```

Internal dependency highlights:

```text
Migration/type/epoch -> worker contract -> live-loop labour -> Phase-2 dominance gate
Reference JSON -> profile/permitting/equipment resolvers -> canaries -> overlay/rate refresh
Goods-share persisted fields -> review reasons -> manifest/card UI -> gold-set go-live
Tier-A extension points -> FR-Z research/design outputs
```

## Gates Summary

| Phase | Gate | Roll Back If |
|------|------|-------------|
| 1 | `TS-SCHEMA-001..005`; `RS-SCHEMA-001..003`; `RS-SEED-001`; epoch 10 and null/default-safe migration; `CI-GIM-001..007` | epoch cannot be synchronized, legacy inserts break, or contracts-schema forks |
| 2 | `SKL-LINT-001..005`; `CAN-LAB-001..007`; `CAN-WARN-001`; `GOLD-L0-003`; `CI-GIM-001..007` | worker needs CDP, reference data cannot stay skill-local, or warnings are unpinned |
| 3 | `RS-ROLLUP-001..005`; `SMOKE-LIVE-001`; `RS-HASH-001..003`; `E2E-IL-001`; `E2E-IL-008`; `CI-GIM-001..007` | live-loop route needs registry changes, goods-only subtotal is unavailable, or money math becomes non-integer |
| 4 | `TS-QUAL-003a/b`; `TS-QUAL-006..008`; `TS-DQ-001..008`; `TS-COV-007..009`; `NEG-CTRL-001..003`; `E2E-IL-005..007`; `E2E-IL-010`; `E2E-IL-011`; `CI-GIM-001..007` | pure services leak, cost-proxy false rejects near-floor cases, or the gate outruns Phase 3 |
| 5 | `E2E-IL-004`; `TS-SCHEMA-005`; `CAN-WARN-001`; `CI-GIM-001..007` | maintenance must be capitalized into dominance or hidden in untyped JSON |
| 6 | `UI-AGB-001..003`; `E2E-IL-001`; `E2E-IL-011`; `TS-COV-009`; `CI-GIM-008` | review rows disappear, UI depends on non-projected data, or web read-only becomes mutating |
| 7 | `E2E-IL-001..011`; `GOLD-L0-001..003`; `CAN-OVERLAY-001`; `SKL-RATE-REFRESH-001..003`; `CAN-RATE-REFRESH-001`; `CI-GIM-001..008` | any scenario lacks a real manifest, category promotion bypasses gold-set, or overlay/rate refresh become live scrape behavior |
| 8 | `SPIKE-Z-001`; `SPIKE-Z-002`; `SPIKE-Z-003` | a spike codes full behavior before accepted research/design output or drops its `FR-Z.*` scope |

## Scope-Preservation Check

- Enumerated source set: `35` functional requirement IDs in the validated spec.
- Requirement routing contains all `35 / 35`; the missing set is empty by construction.
- Tier-A depth check: task-bead distribution is `10 / 12 / 9 / 11 / 8 / 8 / 13`; later Tier-A phases include full gates and non-trivial bead fan-out.
- Recurring-maintenance check: Phase 5 owns `FR-L.8` as a full modelling phase with `8` task beads, `TS-SCHEMA-005`, `E2E-IL-004`, and `CAN-WARN-001`.
- Tier-B check: all three bounded future requirements are tracked in Phase 8 with `3` combined research-spike task beads and named `SPIKE-Z-*` research/design gates.
- Coupling check: Phase 4 production activation depends on Phase 3 labour/permit rollup; the filter cannot ship ahead of the engine.
- Schema-footgun check: migration nullability and epoch 10 across all constants are Phase 1 gates.
- Backfill check: MVP explicitly avoids forced historical recompute; old rows remain `NULL`/`0`, `RS-HASH-003` covers deterministic historical hashing, and new values are produced only by new or explicitly rerun assessments.
- E2E harness order check: `installation_labour_engine.rs` seed helper is Phase 1, so Phase 3/4/5 scenario gates do not consume a Phase 7-created file.
- Recurring-maintenance order check: Phase 1 owns row/table/type foundation, Phase 2 owns maintenance-aware worker hooks and warning inventory, and Phase 5 owns annualized persisted behavior.
- Late Tier-A equality check: Phase 5 has recurring-maintenance model gates, Phase 6 has rendered operator-surfacing gates, and Phase 7 has three implementation-grade lanes: overlay, rate refresh, and gold-set/L0 promotion.
- L0 expansion check: Phase 7 gates category go-live with labelled gold-set slices.
